//! Job table for backtest requests. Std only. Touches no calculation: it decides when one runs and who gets the bytes.
//! - A fixed number of workers per lane, so compute threads cannot grow with connections.
//! - Identical requests (same key) share one job; a finished result stays in a byte-bounded LRU cache.
//! - When the last waiting client leaves, the job is dropped from the table and its stop flag is set. The calculation
//!   sees the flag at its next strategy boundary. A later identical request starts a fresh job, so nobody can attach to
//!   a stopped one. The waiter count only changes under the table lock, which is what makes that safe.
use std::collections::{HashMap, VecDeque};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering::SeqCst};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

/// Cache budget. Engine base is about 200-290 MB under a 1 GiB container cap.
const MAX_CACHE_BYTES: usize = 150 << 20;
/// One optimizer or basket body above this is kept out, so it cannot empty the cache on its own.
const MAX_ENTRY_BYTES: usize = 32 << 20;
/// Results cheaper than this are recomputed: a 16 ms run (2.5-4 MB) must not evict a 13 s summary.
const MIN_COST: Duration = Duration::from_millis(100);
/// Waiting jobs per lane. Above this the client is told to come back (429 + Retry-After).
const MAX_QUEUED: usize = 16;
const POLL: Duration = Duration::from_millis(100);

pub struct Outcome {
    pub status: &'static str,
    pub ctype: &'static str,
    pub body: Vec<u8>,
    /// False for a body that depends on the moment, not on the request (the basket time budget).
    pub cacheable: bool,
}
pub type Compute = Box<dyn FnOnce(&AtomicBool) -> Outcome + Send>;
#[derive(Clone, Copy)]
pub enum Lane { Heavy = 0, Light = 1 }
pub enum Reply { Done(Arc<Outcome>), Busy, Gone }

struct Job {
    waiters: AtomicUsize,
    stop: AtomicBool,
    done: Mutex<Option<Option<Arc<Outcome>>>>, // outer None: running; inner None: the calculation panicked
    ready: Condvar,
}
struct Cached { out: Arc<Outcome>, last_used: u64, cost: usize }
#[derive(Default)]
struct Inner {
    running: HashMap<String, Arc<Job>>,
    queues: [VecDeque<(String, Arc<Job>, Compute)>; 2],
    cache: HashMap<String, Cached>,
    bytes: usize,
    tick: u64,
}
pub struct Table { inner: Mutex<Inner>, work: [Condvar; 2] }

impl Table {
    pub fn start(heavy: usize, light: usize) -> Arc<Table> {
        let table = Arc::new(Table { inner: Mutex::default(), work: [Condvar::new(), Condvar::new()] });
        for (lane, workers) in [(0, heavy), (1, light)] {
            for _ in 0..workers {
                let table = table.clone();
                std::thread::spawn(move || table.worker(lane));
            }
        }
        table
    }

    /// Blocks until the result is ready, the client is gone, or the lane is full.
    pub fn serve(&self, key: String, lane: Lane, compute: Compute, client_gone: &dyn Fn() -> bool) -> Reply {
        let job = {
            let mut guard = self.inner.lock().unwrap();
            let inner = &mut *guard;
            inner.tick += 1;
            if let Some(hit) = inner.cache.get_mut(&key) {
                hit.last_used = inner.tick;
                return Reply::Done(hit.out.clone());
            }
            if let Some(job) = inner.running.get(&key) {
                job.waiters.fetch_add(1, SeqCst);
                job.clone()
            } else {
                if inner.queues[lane as usize].len() >= MAX_QUEUED { return Reply::Busy; }
                let job = Arc::new(Job { waiters: AtomicUsize::new(1), stop: AtomicBool::new(false), done: Mutex::new(None), ready: Condvar::new() });
                inner.running.insert(key.clone(), job.clone());
                inner.queues[lane as usize].push_back((key.clone(), job.clone(), compute));
                self.work[lane as usize].notify_one();
                job
            }
        };
        let mut done = job.done.lock().unwrap();
        loop {
            if let Some(out) = done.as_ref() {
                return out.clone().map_or(Reply::Gone, Reply::Done);
            }
            if client_gone() {
                drop(done);
                self.leave(&key, &job);
                return Reply::Gone;
            }
            done = job.ready.wait_timeout(done, POLL).unwrap().0;
        }
    }

    fn leave(&self, key: &str, job: &Arc<Job>) {
        let mut inner = self.inner.lock().unwrap();
        if job.waiters.fetch_sub(1, SeqCst) > 1 { return; }
        job.stop.store(true, SeqCst);
        if inner.running.get(key).is_some_and(|j| Arc::ptr_eq(j, job)) { inner.running.remove(key); }
        for queue in &mut inner.queues { queue.retain(|(_, j, _)| !Arc::ptr_eq(j, job)); }
    }

    fn worker(&self, lane: usize) {
        loop {
            let (key, job, compute) = {
                let mut inner = self.inner.lock().unwrap();
                loop {
                    if let Some(next) = inner.queues[lane].pop_front() { break next; }
                    inner = self.work[lane].wait(inner).unwrap();
                }
            };
            let started = Instant::now();
            // A panic closes the connection without a response, exactly as when it killed a per-connection thread.
            let out = catch_unwind(AssertUnwindSafe(|| compute(&job.stop))).ok().map(|mut out| {
                out.body.shrink_to_fit(); // format! may over-allocate up to 2x; the budget counts real bytes
                Arc::new(out)
            });
            {
                let mut inner = self.inner.lock().unwrap();
                if inner.running.get(&key).is_some_and(|j| Arc::ptr_eq(j, &job)) { inner.running.remove(&key); }
                if let Some(out) = out.as_ref().filter(|o| o.cacheable && o.status.starts_with("200") && o.body.len() <= MAX_ENTRY_BYTES && started.elapsed() >= MIN_COST) {
                    inner.store(key, out.clone());
                }
            }
            *job.done.lock().unwrap() = Some(out);
            job.ready.notify_all();
        }
    }
}

impl Inner {
    fn store(&mut self, key: String, out: Arc<Outcome>) {
        self.tick += 1;
        let cost = out.body.len() + key.len();
        let entry = Cached { out, last_used: self.tick, cost };
        if let Some(old) = self.cache.insert(key, entry) { self.bytes -= old.cost; }
        self.bytes += cost;
        while self.bytes > MAX_CACHE_BYTES {
            // ponytail: linear scan on eviction; a few hundred entries at most, add a linked LRU only if this ever shows up in a profile.
            let Some(oldest) = self.cache.iter().min_by_key(|(_, c)| c.last_used).map(|(k, _)| k.clone()) else { break };
            if let Some(old) = self.cache.remove(&oldest) { self.bytes -= old.cost; }
        }
    }
}

/// Only when the client has closed. Ok(0) from peek is end of stream, WouldBlock means still connected.
/// A client that half-closes its sending side while still reading counts as gone; the gateway, curl and urllib do not.
pub fn peer_gone(stream: &std::net::TcpStream) -> bool {
    if stream.set_nonblocking(true).is_err() { return true; }
    let mut byte = [0u8; 1];
    let peeked = stream.peek(&mut byte);
    let _ = stream.set_nonblocking(false);
    match peeked { Ok(0) => true, Ok(_) => false, Err(e) => e.kind() != std::io::ErrorKind::WouldBlock }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;
    use std::thread::sleep;

    const STEP: Duration = Duration::from_millis(10);
    fn body(text: &str) -> Vec<u8> { text.as_bytes().to_vec() }
    fn ok(text: &str) -> Outcome { Outcome { status: "200 OK", ctype: "text/plain", body: body(text), cacheable: true } }
    /// A fake calculation of `steps` strategies that honours the stop flag.
    fn fake(steps: usize, ran: Arc<AtomicUsize>) -> Compute {
        Box::new(move |stop| {
            for _ in 0..steps {
                if stop.load(SeqCst) { return Outcome { status: "400 Bad Request", ctype: "text/plain", body: body("cancelled"), cacheable: true }; }
                sleep(STEP);
                ran.fetch_add(1, SeqCst);
            }
            ok("result")
        })
    }
    fn wait(table: &Table, key: &str, compute: Compute) -> Reply { table.serve(key.into(), Lane::Heavy, compute, &|| false) }
    fn text(reply: Reply) -> String {
        match reply { Reply::Done(o) => String::from_utf8(o.body.clone()).unwrap(), Reply::Busy => "BUSY".into(), Reply::Gone => "GONE".into() }
    }

    #[test]
    fn abandoned_job_stops_and_is_not_served_to_a_later_client() {
        let (table, ran) = (Table::start(2, 0), Arc::new(AtomicUsize::new(0)));
        let (t, r) = (table.clone(), ran.clone());
        let leaver = std::thread::spawn(move || {
            let begun = Instant::now();
            let gone = move || begun.elapsed() > Duration::from_millis(100);
            text(t.serve("a".into(), Lane::Heavy, fake(400, r), &gone))
        });
        assert_eq!(leaver.join().unwrap(), "GONE");
        sleep(Duration::from_millis(100));
        let after = ran.load(SeqCst);
        sleep(Duration::from_millis(100));
        assert_eq!(after, ran.load(SeqCst), "work continued after the client left");
        assert!(after < 400);
        // The stopped job's "cancelled" body must never reach anyone: same key computes afresh.
        assert_eq!(text(wait(&table, "a", fake(12, Arc::default()))), "result");
    }

    #[test]
    fn identical_requests_share_one_job_and_a_repeat_is_a_cache_hit() {
        let (table, ran) = (Table::start(2, 0), Arc::new(AtomicUsize::new(0)));
        let clients: Vec<_> = (0..8).map(|_| {
            let (t, r) = (table.clone(), ran.clone());
            std::thread::spawn(move || text(wait(&t, "b", fake(15, r))))
        }).collect();
        for c in clients { assert_eq!(c.join().unwrap(), "result"); }
        assert_eq!(ran.load(SeqCst), 15, "identical requests computed more than once");
        assert_eq!(text(wait(&table, "b", fake(15, ran.clone()))), "result");
        assert_eq!(ran.load(SeqCst), 15, "a repeat was not served from the cache");
    }

    #[test]
    fn one_of_two_clients_leaving_does_not_stop_the_shared_job() {
        let (table, ran) = (Table::start(1, 0), Arc::new(AtomicUsize::new(0)));
        let (t, r) = (table.clone(), ran.clone());
        let stayer = std::thread::spawn(move || text(wait(&t, "c", fake(30, r))));
        sleep(STEP);
        let begun = Instant::now();
        let gone = move || begun.elapsed() > Duration::from_millis(50);
        assert_eq!(text(table.serve("c".into(), Lane::Heavy, fake(30, ran.clone()), &gone)), "GONE");
        assert_eq!(stayer.join().unwrap(), "result");
        assert_eq!(ran.load(SeqCst), 30);
    }

    #[test]
    fn workers_are_bounded_and_a_full_lane_answers_busy() {
        let (table, peak, now) = (Table::start(2, 0), Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0)));
        let clients: Vec<_> = (0..6).map(|i| {
            let (t, p, n) = (table.clone(), peak.clone(), now.clone());
            std::thread::spawn(move || text(wait(&t, &format!("d{i}"), Box::new(move |_| {
                p.fetch_max(n.fetch_add(1, SeqCst) + 1, SeqCst);
                sleep(Duration::from_millis(30));
                n.fetch_sub(1, SeqCst);
                ok("result")
            }))))
        }).collect();
        for c in clients { assert_eq!(c.join().unwrap(), "result"); }
        assert_eq!(peak.load(SeqCst), 2);
        // No workers: everything queues until the lane is full, and a client leaving frees its place.
        let idle = Table::start(0, 0);
        let queued: Vec<_> = (0..MAX_QUEUED).map(|i| {
            let t = idle.clone();
            std::thread::spawn(move || {
                let begun = Instant::now();
                text(t.serve(format!("q{i}"), Lane::Heavy, fake(1, Arc::default()), &move || begun.elapsed() > Duration::from_millis(300)))
            })
        }).collect();
        sleep(Duration::from_millis(100));
        assert_eq!(text(idle.serve("over".into(), Lane::Heavy, fake(1, Arc::default()), &|| true)), "BUSY");
        for q in queued { assert_eq!(q.join().unwrap(), "GONE"); }
        assert!(idle.inner.lock().unwrap().queues[0].is_empty());
    }

    #[test]
    fn cache_keeps_to_its_byte_budget_and_evicts_the_oldest() {
        let table = Table::start(0, 0);
        let mut inner = table.inner.lock().unwrap();
        let big = |n: usize| Arc::new(Outcome { status: "200 OK", ctype: "text/plain", body: vec![0; n], cacheable: true });
        for i in 0..6 { inner.store(format!("k{i}"), big(30 << 20)); }
        assert!(inner.bytes <= MAX_CACHE_BYTES);
        assert!(!inner.cache.contains_key("k0") && inner.cache.contains_key("k5"));
        inner.store("k5".into(), big(1 << 20));
        assert_eq!(inner.bytes, inner.cache.values().map(|c| c.cost).sum::<usize>(), "replacing a key must not leak budget");
    }

    #[test]
    fn errors_cheap_results_and_uncacheable_bodies_are_not_kept_and_a_panic_serves_nobody() {
        let table = Table::start(1, 0);
        let slow = |out: Outcome| -> Compute { Box::new(move |_| { sleep(MIN_COST + STEP); out }) };
        wait(&table, "e", slow(Outcome { status: "400 Bad Request", ctype: "text/plain", body: body("bad"), cacheable: true }));
        wait(&table, "f", slow(Outcome { cacheable: false, ..ok("held") }));
        wait(&table, "g", Box::new(|_| ok("instant")));
        assert!(table.inner.lock().unwrap().cache.is_empty());
        assert_eq!(text(wait(&table, "h", Box::new(|_| panic!("expected test panic")))), "GONE");
        assert_eq!(text(wait(&table, "h", slow(ok("recovered")))), "recovered", "worker must survive a panic");
    }

    #[test]
    fn peer_gone_tells_a_closed_socket_from_a_quiet_one() {
        use std::net::{TcpListener, TcpStream};
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let client = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        let (server, _) = listener.accept().unwrap();
        assert!(!peer_gone(&server));
        drop(client);
        sleep(Duration::from_millis(50));
        assert!(peer_gone(&server));
    }
}
