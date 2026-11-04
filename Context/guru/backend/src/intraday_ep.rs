//! Intraday engine pieces EP1/EP2 (lane K39, 4 Nov 2026, decision #202): N-minute bars built from the 1-minute file.
//! One helper, `nbars`, aggregates one session's minutes. EP1 (bars inside one day, reset daily, the first bar starting at the
//! session open) and EP2 (one continuous N-minute series across days, TradeStation style: indicators and positions carry
//! overnight, no bar spans two sessions) are the same bars; a rule keeps (EP2) or resets (EP1) its own state between days.
//! Reuses intraday.rs (Bar1, session_open, day_trade) and intraday_w2b.rs (day: the NIFTY_BAD_FROM rule) unchanged.
use crate::connors::Trade;
use crate::intraday::Bar1;

/// One N-minute bar: `s` = the bucket's first clock minute, `e` = its TradeStation time stamp (bucket end, or the session
/// close for a shorter last bar), `a..b` = its minutes in the day's vector. A bucket with no minutes makes no bar.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BarN { pub s: u16, pub e: u16, pub o: f64, pub h: f64, pub l: f64, pub c: f64, pub a: usize, pub b: usize }

/// Regular-session close on the exchange clock (the minute after the last start-stamped minute): US 16:00 ET, NSE 15:30 IST.
/// Checked against the files (4 Nov 2026): no minute at or after it on any day of SPY, SPX, NIFTY 50 or NIFTYBANK.
pub fn session_close(ticker: &str) -> u16 { if ticker.starts_with("NIFTY") { 15 * 60 + 30 } else { 16 * 60 } }

/// The day's 1-minute bars (time order) as N-minute bars whose buckets start at `anchor` + k*n: anchor = the session open
/// (session-aligned bars) or 0 (clock-aligned "natural" bars). Open = first minute's open, close = last minute's close,
/// high/low = the extremes; stamp = min(bucket end, `close`). Minutes before `anchor` (none in the files) go to bucket 0.
pub fn nbars(day: &[Bar1], anchor: u16, n: u16, close: u16) -> Vec<BarN> {
    let mut out: Vec<BarN> = vec![];
    for (k, z) in day.iter().enumerate() {
        let s = anchor + z.m.saturating_sub(anchor) / n * n;
        match out.last_mut() {
            Some(b) if b.s == s => { b.h = b.h.max(z.h); b.l = b.l.min(z.l); b.c = z.c; b.b = k + 1; }
            _ => out.push(BarN { s, e: (s + n).min(close), o: z.o, h: z.h, l: z.l, c: z.c, a: k, b: k + 1 }),
        }
    }
    out
}

/// Session-aligned N-minute bars of daily row i (None: no minutes, day excluded by the cleaner, or a rejected NIFTY 50 day).
/// Returns (session open minute, session close minute, bars).
pub fn day_bars(x: &crate::connors::Ctx, i: usize, n: u16) -> Option<(u16, u16, Vec<Bar1>, Vec<BarN>)> {
    let m = crate::intraday_w2b::day(x, i)?;
    let r = &x.r[i];
    let (open, close) = (crate::intraday::session_open(&r.ticker, &r.date), session_close(&r.ticker));
    let b = nbars(&m, open, n, close);
    (!b.is_empty()).then_some((open, close, m, b))
}

/// One position at a time, the TradeStation strategy model of the EP rules: an entry in the held direction is ignored, an
/// opposite entry reverses (exit and entry at the same price), an exit while flat is ignored.
#[derive(Default)]
pub struct Book { pub pos: Option<(f64, usize, f64, &'static str)>, pub out: Vec<Trade> }
impl Book {
    pub fn dir(&self) -> f64 { self.pos.map_or(0.0, |p| p.0) }
    pub fn exit(&mut self, i: usize, px: f64, why: &str) {
        if let Some((d, ei, ep, k)) = self.pos.take() { self.out.push(crate::intraday::day_trade(d, ei, ep, i, px, why, k, false)); }
    }
    pub fn enter(&mut self, dir: f64, i: usize, px: f64, kind: &'static str, why: &str) {
        if self.dir() == dir { return; }
        self.exit(i, px, why);
        self.pos = Some((dir, i, px, kind));
    }
    /// A position still open on the run's last day is closed there at `px`, marked open.
    pub fn finish(mut self, i: usize, px: f64) -> Vec<Trade> {
        if let Some((d, ei, ep, k)) = self.pos.take() { self.out.push(crate::intraday::day_trade(d, ei, ep, i, px, "open", k, true)); }
        self.out
    }
}

/// TradeStation RSI (Wilder smoothing; first value after `n` changes: NetChgAvg = (P - P[n]) / n, TotChgAvg = average |change|
/// over n; then X += (change - X) / n; RSI = 50 x (NetChgAvg / TotChgAvg + 1), 50 when TotChgAvg = 0).
pub struct Rsi { n: usize, first: Vec<f64>, net: f64, tot: f64, prev: f64 }
impl Rsi {
    pub fn new(n: usize) -> Rsi { Rsi { n, first: vec![], net: 0.0, tot: 0.0, prev: f64::NAN } }
    pub fn push(&mut self, p: f64) -> Option<f64> {
        if self.first.len() <= self.n {
            self.first.push(p);
            self.prev = p;
            if self.first.len() <= self.n { return None; }
            let f = &self.first;
            self.net = (p - f[0]) / self.n as f64;
            self.tot = f.windows(2).map(|w| (w[1] - w[0]).abs()).sum::<f64>() / self.n as f64;
        } else {
            let ch = p - self.prev;
            self.prev = p;
            self.net += (ch - self.net) / self.n as f64;
            self.tot += (ch.abs() - self.tot) / self.n as f64;
        }
        Some(if self.tot != 0.0 { 50.0 * (self.net / self.tot + 1.0) } else { 50.0 })
    }
}

/// Rules on these bars that run on all four minute tickers (SPY, SPX, NIFTY 50, NIFTYBANK): no fixed amount; clock times,
/// where a rule has them, are minutes after the session open (the book's own way of moving its times to other markets).
pub const IDS: &[&str] = &["acbff_open_rel", "acbff_2bar_open", "acbff_final_third", "acbff_1230", "acbff_9oc", "acbff_midpoint",
    "acbff_rsi_5min", "atm04_60min", "jmls_bar_breakout", "jmls_ma_cross_30m"];
/// Rules whose time filter is a US Eastern clock time the book gives no NSE equivalent for: SPY and SPX only.
pub const US: &[&str] = &["jmls_ls_model"];
pub fn id(id: &str) -> bool { IDS.contains(&id) || US.contains(&id) || K42.contains(&id) || K42_SPX.contains(&id) || K44_SPX.contains(&id) || K45_SPX.contains(&id) }
pub fn us_id(id: &str) -> bool { US.contains(&id) }
pub fn book(b: &str) -> bool { matches!(b, "acbff" | "atm04" | "jmls" | "gfstsb" | "stad" | "kftsm" | "pbwts" | "jbeft") }
// ---- lane K42 block (4 Nov 2026, decision #211; authors/titles-1030/K42.tsv)
/// No fixed amount: all four minute tickers. Ford's three NinjaTrader systems on 60-minute bars (EP2) and the STAD Vol 3
/// inside-day breakout failure on 30-minute bars with a daily Data2 (EP1).
pub const K42: &[&str] = &["gfstsb_ema_cross", "gfstsb_channel", "gfstsb_perfect_order", "stad3_idbf"];
/// Book amounts in S&P points ('20 point' = 0.20 S&P points, McNutt): SPX only (instrument_policy::intraday_w1_spx_id).
pub const K42_SPX: &[&str] = &["kftsm_first_hour"];
pub fn spx_id(id: &str) -> bool { K42_SPX.contains(&id) || K44_SPX.contains(&id) || K45_SPX.contains(&id) }
// jbcdt2_or30 (30-Minute Breakout, S&P points) is coded in books/jbcdt2.rs but held: 29.3% of its SPX trades depend on the
// order inside one minute (decision #175 limit 5%); the #177b scaled-SPY check passed (K42w/check177b.out).
// ---- end lane K42 block
// ---- lane K44 block (4 Nov 2026, decision #213; authors/titles-1030/K44.tsv)
/// Pruitt's Super Combo on 5-minute bars with a daily Data2 (EP1); its 3.00 protective-stop floor is S&P points: SPX only.
pub const K44_SPX: &[&str] = &["pbwts_super_combo"];
// ---- end lane K44 block
// ---- lane K45 block (4 Nov 2026, decision #214; authors/titles-1030/K45.tsv)
/// Bernstein's open/close oscillator on 15-minute bars (EP2); its stops and floor are S&P points at the $500 contract: SPX only.
pub const K45_SPX: &[&str] = &["jbeft_oc_osc"];
// jbeft_channel_break (60-minute bars, books/jbeft.rs) is coded but held: 15.2% of its SPX trades depend on the order inside
// one minute (decision #175 limit 5%), and #177b flags it (SPX +1,148.9% vs scaled SPY +71.0%, 2014-2025; K45w/check177b.out).
// ---- end lane K45 block
// Decision #177a for these rules (flagged on the SPY/SPX overlap: SPY only) goes into intraday_w3::SPY, the one #177a list.

#[cfg(test)]
mod tests {
    use super::*;
    fn b(m: u16, o: f64, h: f64, l: f64, c: f64) -> Bar1 { Bar1 { m, o, h, l, c } }
    #[test]
    fn nbars_session_and_clock_aligned_by_hand() {
        // 09:30-09:44 session open 570; one minute missing (09:33) and the 09:40-09:44 bucket starting late at 09:41
        let v = [b(570, 10., 11., 9.5, 10.5), b(571, 10.5, 12., 10., 11.), b(572, 11., 11.2, 8., 9.), b(574, 9., 9.5, 8.5, 9.2),
            b(575, 9.2, 9.3, 9.1, 9.25), b(581, 9.3, 9.9, 9.0, 9.8), b(584, 9.8, 10.1, 9.7, 10.0)];
        let x = nbars(&v, 570, 5, 960);
        assert_eq!(x, vec![BarN { s: 570, e: 575, o: 10., h: 12., l: 8., c: 9.2, a: 0, b: 4 },
            BarN { s: 575, e: 580, o: 9.2, h: 9.3, l: 9.1, c: 9.25, a: 4, b: 5 },
            BarN { s: 580, e: 585, o: 9.3, h: 10.1, l: 9.0, c: 10.0, a: 5, b: 7 }]);
        // clock-aligned 60-minute bars from a 09:30 open: 09:30-09:59 is the first (short) bar, stamped 10:00
        let w = [b(570, 1., 2., 0.5, 1.5), b(599, 1.5, 3., 1., 2.), b(600, 2., 2.5, 1.8, 2.2), b(959, 2.2, 2.4, 2.1, 2.3)];
        assert_eq!(nbars(&w, 0, 60, 960), vec![BarN { s: 540, e: 600, o: 1., h: 3., l: 0.5, c: 2., a: 0, b: 2 },
            BarN { s: 600, e: 660, o: 2., h: 2.5, l: 1.8, c: 2.2, a: 2, b: 3 }, BarN { s: 900, e: 960, o: 2.2, h: 2.4, l: 2.1, c: 2.3, a: 3, b: 4 }]);
        // 135-minute bars on a 390-minute US session: stamps 135, 270 after the open and the close (a 120-minute last bar)
        let u: Vec<Bar1> = (570..960).map(|m| b(m, m as f64, m as f64 + 0.5, m as f64 - 0.5, m as f64 + 0.25)).collect();
        let y = nbars(&u, 570, 135, 960);
        assert_eq!(y.iter().map(|z| (z.s, z.e, z.b - z.a)).collect::<Vec<_>>(), vec![(570, 705, 135), (705, 840, 135), (840, 960, 120)]);
        assert_eq!((y[2].o, y[2].h, y[2].l, y[2].c), (840., 959.5, 839.5, 959.25));
        // NSE: 30-minute bars from 09:15 to 15:30 (375 minutes): the last bar is 15:15-15:29, stamped 15:30
        let z: Vec<Bar1> = (555..930).map(|m| b(m, 1., 1., 1., 1.)).collect();
        let q = nbars(&z, 555, 30, 930);
        assert_eq!((q.len(), q[12].s, q[12].e, q[12].b - q[12].a), (13, 915, 930, 15));
        assert!(nbars(&[], 570, 5, 960).is_empty());
    }
    #[test]
    fn rsi_is_tradestation_wilder() {
        let mut r = Rsi::new(3);
        let v: Vec<Option<f64>> = [10., 11., 10.5, 12., 11., 11.].iter().map(|&p| r.push(p)).collect();
        assert_eq!(&v[..3], &[None, None, None]);
        // net = (12-10)/3, tot = (1+.5+1.5)/3 = 1 -> 50 x (2/3 + 1)
        assert!((v[3].unwrap() - 50. * (2. / 3. + 1.)).abs() < 1e-12);
        let (net, tot) = (2. / 3. + (-1. - 2. / 3.) / 3., 1. + (1. - 1.) / 3.);
        assert!((v[4].unwrap() - 50. * (net / tot + 1.)).abs() < 1e-12);
        let (net, tot) = (net - net / 3., tot - tot / 3.);
        assert!((v[5].unwrap() - 50. * (net / tot + 1.)).abs() < 1e-12);
        let mut flat = Rsi::new(2);
        assert_eq!([5., 5., 5.].map(|p| flat.push(p)), [None, None, Some(50.)]);
    }
    #[test]
    fn book_reverses_and_ignores_same_side() {
        let mut k = Book::default();
        k.enter(1., 3, 100., "open", "x");
        k.enter(1., 3, 101., "open", "x");
        k.exit(4, 102., "end of day");
        k.exit(4, 103., "end of day");
        k.enter(-1., 5, 104., "open", "x");
        k.enter(1., 5, 103., "open", "reverse");
        let t = k.finish(6, 105.);
        assert_eq!(t.iter().map(|z| (z.dir, z.legs[0].i, z.legs[0].px, z.exit_i, z.exit_px, z.open)).collect::<Vec<_>>(),
            vec![(1., 3, 100., 4, 102., false), (-1., 5, 104., 5, 103., false), (1., 5, 103., 6, 105., true)]);
    }
    fn minutes(name: &str, days: &[(&str, Vec<String>)]) -> &'static crate::intraday::Minutes {
        let dir = std::env::temp_dir().join(format!("ep{name}{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let (mut body, mut idx) = (String::from("Date,Time,Open,High,Low,Close\n"), String::from("Date,offset,bytes\n"));
        for (d, lines) in days {
            let start = body.len();
            for l in lines { body.push_str(&format!("{d},{l}\n")); }
            idx.push_str(&format!("{d},{start},{}\n", body.len() - start));
        }
        std::fs::write(dir.join("T.m1.csv"), &body).unwrap();
        std::fs::write(dir.join("T.m1idx.csv"), &idx).unwrap();
        Box::leak(Box::new(crate::intraday::Minutes::load(dir.to_str().unwrap(), "T").unwrap()))
    }
    fn hm(m: u16) -> String { format!("{:02}:{:02}", m / 60, m % 60) }
    #[test]
    fn open_rel_and_bar_breakout_on_hand_built_minutes() {
        use crate::connors::{Ctx, Params};
        let row = |d: &str| crate::Row { ticker: "T".into(), date: d.into(), open: 1., high: 1., low: 1., close: 1., volume: None };
        let r = vec![row("2020-01-02"), row("2020-01-03")];
        let mut x = Ctx::new(&r, &r, &[]);
        // day 1 rises 0.01 a minute from 100: the 10:00-10:29 bar (stamped 9:30 CT) closes above the open -> long at the
        // 10:30 minute's open, out at the 15:59 close. Day 2 falls: short.
        let up: Vec<String> = (570..960u16).map(|m| { let p = 100. + (m - 570) as f64 * 0.01; format!("{},{p},{},{},{p}", hm(m), p + 0.005, p - 0.005) }).collect();
        let dn: Vec<String> = (570..960u16).map(|m| { let p = 100. - (m - 570) as f64 * 0.01; format!("{},{p},{},{},{p}", hm(m), p + 0.005, p - 0.005) }).collect();
        x.minutes = Some(minutes("orel", &[("2020-01-02", up.clone()), ("2020-01-03", dn)]));
        let t = crate::books::acbff::run("acbff_open_rel", &x, 0, 1, &Params::new()).unwrap();
        let v: Vec<(f64, usize, f64, usize, f64)> = t.iter().map(|z| (z.dir, z.legs[0].i, (z.legs[0].px * 100.).round() / 100., z.exit_i, (z.exit_px * 100.).round() / 100.)).collect();
        assert_eq!(v, vec![(1., 0, 100.6, 0, 103.89), (-1., 1, 99.4, 1, 96.11)]);
        // bar breakout: day 1 rises (26 bars); the 25-bar window is full after bar 24 (stamped 15:45, not the last bar), so a buy
        // stop at its high (103.745) lives in the 15:45 bar, whose first minute opens through it at 103.75 (flat: both stops
        // live). Day 2 falls from 100: after its first bar the 25-bar low is that bar's low 99.855 and the 09:45 minute opens
        // through it at 99.85: long out, short in (reverse), held to the end of the run (open)
        let dn2: Vec<String> = (570..960u16).map(|m| { let p = 100. - (m - 570) as f64 * 0.01; format!("{},{p},{},{},{p}", hm(m), p + 0.005, p - 0.005) }).collect();
        x.minutes = Some(minutes("bb", &[("2020-01-02", up), ("2020-01-03", dn2)]));
        let t = crate::books::jmls::run("jmls_bar_breakout", &x, 0, 1, &Params::new()).unwrap();
        let v: Vec<(f64, usize, f64, usize, f64, bool)> = t.iter().map(|z| (z.dir, z.legs[0].i, (z.legs[0].px * 1e6).round() / 1e6, z.exit_i, (z.exit_px * 1e6).round() / 1e6, z.open)).collect();
        assert_eq!(v, vec![(1., 0, 103.75, 1, 99.85, false), (-1., 1, 99.85, 1, 96.11, true)]);
        assert!(crate::books::jmls::run("acbff_open_rel", &x, 0, 1, &Params::new()).is_none()); // module prefix guard
    }
    #[test]
    fn ep_admission_and_tickers() {
        use crate::instrument_policy as ip;
        let cat = crate::connors::catalogue();
        for id in IDS.iter().chain(US).chain(K42).chain(K42_SPX).chain(K44_SPX).chain(K45_SPX) {
            let d = cat.iter().find(|d| d.id == *id).unwrap_or_else(|| panic!("{id}"));
            assert!(d.status == crate::connors::Status::Live && ip::intraday_p1(d) && ip::reviewed_cash_rule(d) && ip::held_reason(id).is_none(), "{id}");
            assert_eq!(ip::policy(d).requirement, "declared_index_timing", "{id}");
            let want: &[&str] = if crate::intraday_w3::spx_held(id) { &[] } else if crate::intraday_w3::spy_id(id) { &["SPY"] } else if spx_id(id) { &["SPX"] } else if us_id(id) { &["SPY", "SPX"] } else { ip::INTRADAY_P1_TICKERS };
            for t in ip::INTRADAY_P1_TICKERS { assert_eq!(ip::check(d, t, false).is_none(), want.contains(t), "{id} {t}"); }
            assert!(ip::check(d, "QQQ", false).is_some() && ip::check(d, "AAPL", true).is_some(), "{id}");
            let g = crate::gate::get(id).unwrap_or_else(|| panic!("gate {id}"));
            assert!((matches!(g.verdict, "unverified" | "verified") || (crate::intraday_w3::spx_held(id) && g.verdict == "withheld")) && (g.reason.contains("K39") || g.reason.contains("K42") || g.reason.contains("K44") || g.reason.contains("K45")) && g.reason.contains("p."), "{id}");
        }
        assert!(ip::check(cat.iter().find(|d| d.id == "jmls_ls_model").unwrap(), "NIFTY 50", false).unwrap().note.contains("US Eastern clock time"));
    }
    /// EP check against the real files (run by hand, K39w/nbars_check.py compares with a Python aggregation):
    /// VAULT_M1_DIR=<data/intraday> NBARS_OUT=<dir> cargo test --release nbars_dump -- --ignored
    #[test]
    #[ignore]
    fn nbars_dump() {
        use std::io::Write;
        let (dir, out) = (std::env::var("VAULT_M1_DIR").unwrap(), std::env::var("NBARS_OUT").unwrap());
        for t in ["SPY", "SPX", "NIFTY 50", "NIFTYBANK"] {
            let m = crate::intraday::Minutes::load(&dir, t).unwrap();
            let text = std::fs::read_to_string(format!("{dir}/{}.m1idx.csv", t.replace(' ', "_"))).unwrap();
            let mut f = std::io::BufWriter::new(std::fs::File::create(format!("{out}/{}.rust.csv", t.replace(' ', "_"))).unwrap());
            for d in text.lines().skip(1).filter_map(|l| l.split(',').next()) {
                let day = m.day(d).unwrap();
                for (anchor, n) in [(crate::intraday::session_open(t, d), 5u16), (crate::intraday::session_open(t, d), 10), (crate::intraday::session_open(t, d), 15),
                    (crate::intraday::session_open(t, d), 30), (crate::intraday::session_open(t, d), 60), (crate::intraday::session_open(t, d), 135), (0, 60)] {
                    for z in nbars(&day, anchor, n, session_close(t)) {
                        writeln!(f, "{d},{anchor},{n},{},{},{},{},{},{},{}", z.s, z.e, z.o, z.h, z.l, z.c, z.b - z.a).unwrap();
                    }
                }
            }
        }
    }
}
