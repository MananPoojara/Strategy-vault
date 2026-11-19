//! Lane K80 (13 Nov 2026; authors/titles-1030/K80.tsv): Joe Ross, "Trading by the Minute" (revised 1999, 2001 printing; the
//! 1991 revision read as well). Book id "jr_tbtm". Research share 'Trading Books/Joe Ross - Trading by the Minute/' (both
//! editions are page scans: OCR authors/ross/text/ocr/minute1999.txt and minute1991.txt, read in full by the 22 Oct Ross reading,
//! authors/ross/agents/by_the_minute; K80 re-read the 1991 rule chapters, K80w/ocr). Pages: 1999 PDF page (printed = PDF - 3).
//! Two rules are complete in the 1999 edition and are coded on the vault's 1-minute files (intraday_ep::K80, the four minute
//! tickers), as day trades on 5-minute session bars with the book's "violation method" exit:
//!  * jr_tbtm_shrl  Sagging Highs / Rising Lows (PDF 171-174, 353);
//!  * jr_tbtm_3day  the intermediate entry signal, a breakout of the last three days' extreme (PDF 54, 112).
//! The other methods are held (duplicates, judgement, or the 1991 dollar money management), with the reason on each card.
//! Independent reference: authors/titles-1030/K80w/k80_ref.py (written from the pages, not from this file).
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use crate::intraday::day_trade;
use crate::intraday_ep6::{self as ep6, Ev, Seg};

pub const BOOK: &str = "jr_tbtm";
/// Coded minute rules (admitted through intraday_ep::K80 on the four minute tickers).
pub const INTRA: &[&str] = &["jr_tbtm_shrl", "jr_tbtm_3day"];

const SRC: &str = "Joe Ross, Trading by the Minute (revised 1999, 2001 printing; 1991 revision also read). Research-share page scans, OCR authors/ross/text/ocr (PDF pages cited; printed = PDF - 3).";
const VIOLATION: &str = "X|Violation method, the whole position at the first violation (\"you have a choice as to whether or not you would sell the entire position at the time of the violation\", PDF 74): \"If I see two reversal bars in a series of price bars that are moving in the desired direction away from my entry point\" (a reversal bar closes against the trade: \"in an up move ... two bars which close lower than they open\", PDF 73) -> out at the close of the second; \"If I see a bar that makes a lower low in an up move, or a price bar that makes a higher high in a down move\" (PDF 72-73) -> out one tick beyond the previous bar's low (high), on the minute it is reached";
const DAYTRADE: &str = "X|\"WE WILL NEVER CARRY A DAYTRADE OVERNIGHT! We will always be out by the close of trading\" (PDF 34): square-off at the close of the session's last bar (conversion to a position trade is the trader's option, PDF 188-190, not used)";
const READ: &str = "A|Read: 5-minute session-aligned bars from the 1-minute file (the book's S&P day-trading chart, \"daytrading a five minute chart\", PDF 12-14), counted inside today's session only (\"interested only in what is happening today\", 1991 p.23); 1 tick = the instrument's tick (US 0.01, NSE 0.05; #57); the violation stop works from the entry bar (after the fill, on the minute path) with the bar before as its reference; reversal bars are counted from the entry bar inclusive; one position at a time; orders, fills and the order inside a minute on the real 1-minute path (intraday_ep6::walk_minutes)";

macro_rules! def {
    ($id:expr, $name:expr, $group:expr, $status:expr, $rules:expr) => {
        StrategyDef { id: $id, name: $name, group: $group, session: 0, dir: "Long/Short", status: $status, needs_volume: false,
            needs_vix: false, needs_series: &[], book: BOOK, rules: $rules, notes: SRC, claim: None }
    };
}

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        def!("jr_tbtm_shrl", "Sagging Highs / Rising Lows day trade (5-minute): three lower highs -> sell the break of that bar's low; violation exit", "Trading by the Minute · counting methods", Status::Live,
            &["E|\"Sagging highs require 3 consecutive bars of Lower Highs. Three consecutive bars means exactly that, three consecutive bars; no intervening bars are allowed ... Three successive lower highs yield an entry signal at the point where the low of the bar that made the third lower high is taken out ... Sell a breakout of the low of the bar that made the third successive lower high\" (PDF 353; chapter PDF 171-174)",
              "E|\"Rising Lows require 3 consecutive bars of Higher Lows ... Buy a breakout of the high of the bar that made the third successive higher low\" (PDF 353)",
              "A|Read: three lower highs = H[k-3] > H[k-2] > H[k-1] > H[k] inside the session; the stop order (one tick beyond bar k) works on the next bar only, the book's 'no shifting of the count' (PDF 353); a later bar that again completes three lower highs places a new order",
              VIOLATION, DAYTRADE, READ]),
        def!("jr_tbtm_3day", "Three-day extreme breakout day trade (intermediate entry signal): buy the break of the last three days' highest high; violation exit", "Trading by the Minute · intermediate entry signals", Status::Live,
            &["E|\"A breakout of the highest high of the last three bars taken as a group. A breakout of the lowest low of the last three bars taken as a group. A breakout of any individual low of the last three bars. This includes a breakout of the previous bar's low ... The breakout must be by virtue of prices trading through the entry points. A gap opening past the entry point nullifies the trade initially\" (PDF 54; daily bars: \"They work best taken from daily or greater charts\")",
              "E|p.level 0 (default) = the group extreme, \"The stronger trade may come as a result of a breakout of the highest high or lowest low of the last three bars\" (PDF 54); p.level 1 = yesterday's high / low, \"I may enter based upon a non-gap breakout of yesterday's high or low\" (PDF 112) [labelled option]",
              "A|Read: buy stop one tick above the level, sell stop one tick below, both working from the session's first minute; an opening minute at or beyond a level nullifies that side for the day; each side fills at most once a day; the violation stop has no reference bar on the session's first 5-minute bar, so it starts on the second",
              VIOLATION, DAYTRADE, READ]),
        def!("jr_tbtm_snem", "Successive New Extremes (simple count)", "Trading by the Minute · counting methods",
            Status::NotTestable("Duplicate of jr_sne_l / jr_sne_s (Electronic Trading TNT I ch.27): the same count; this book's verbal rule without a correction bar (PDF 352) is already that card's labelled option p.correction 0."), &[]),
        def!("jr_tbtm_major", "Major entry signals: trading range, 1-2-3, ledge and Ross hook breakouts", "Trading by the Minute · major entry signals",
            Status::NotTestable("Duplicate of the Law of Charts formations already in the vault (jr_123_std_l/s, jr_hook_std_l/s, jr_ledge_tbtb; jr_range_l/s and jr_ledge_l/s held there): this book uses them as daily signal levels for intraday entries and adds context judgement ('major or intermediate turning points', PDF 36-41)."), &[]),
        def!("jr_tbtm_minor", "Minor entry signal and the trading filter: breakouts of intraday congestion", "Trading by the Minute · minor signals, filter",
            Status::NotTestable("Held: the congestion boundaries and the choice of first or second break are judged ('eye of the beholder'; PDF 59-61, 92-102, 147-156)."), &[]),
        def!("jr_tbtm_segments", "Segment Counting, True Correction and True Trend methods", "Trading by the Minute · trend methods",
            Status::NotTestable("Held: the segment points and the trend line are chosen on the chart; the True Trend Method's exits are the bean-oil dollar thresholds ($50 to breakeven, 50% of profits once $100) and 'be on guard / tighten stops' (1991 PDF 158-162; 1999 PDF 174-187, 334-349)."), &[]),
        def!("jr_tbtm_mm1991", "The 1991 day-trade plan: 10-tick stop, $70-$100 per contract out, breakeven, 50% protection", "Trading by the Minute (1991) · stops, trading plan",
            Status::NotTestable("Held: the exits are dollars per futures contract and contract sets ('take profits at or about $100', a mental stop 10 ticks away, 1991 PDF 25-26, 31). On the S&P contract of the book ($500 a point) that is a 0.14-0.20 point target and a 0.50 point stop; from 2018 on, 57-98% of the vault's SPX minutes span 0.70 points (K80w), so most trades would depend on the order inside one minute (decision #175)."), &[]),
    ]
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let pd = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(match id {
        "jr_tbtm_shrl" => vec![pd("bar", "Bar length in minutes (5, the book's day-trading chart)", 5., 1., 60., 1.)],
        "jr_tbtm_3day" => vec![pd("bar", "Bar length in minutes for the violation exit (5)", 5., 1., 60., 1.),
            pd("level", "Entry level: 0 = last three days' highest high / lowest low as a group (PDF 54); 1 = yesterday's high / low (PDF 112)", 0., 0., 1., 1.)],
        _ => return None,
    })
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let d = catalogue().into_iter().find(|d| d.id == id)?;
    Some((d.id, d.name, "Trading by the Minute (Ross)"))
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if params(id).is_none() || to >= x.c.len() || from > to { return None; }
    if x.minutes.is_none() { return Some(vec![]); } // no minute bars: no trades (as every other minute module), not "no runner"
    let g = |k: &str| super::jb624::getp(id, p, k);
    let n = g("bar").round().clamp(1., 60.) as u16;
    Some(day_trades(x, from, to, n, if id == "jr_tbtm_3day" { Some(g("level") >= 0.5) } else { None }))
}

#[derive(Clone, Copy, Debug)]
struct Pos { d: f64, ei: usize, ep: f64, stop: f64, rev: u8, amb: bool }
#[derive(Clone, Default)]
struct W { lv: [f64; 2], stops: [f64; 2], pos: Option<Pos>, done: Vec<(Pos, f64)>, oco: bool }

/// A level reached on segment g moving `up` (already beyond at the start counts): fill at the level, or at the end of a gap.
fn reach(g: &Seg, lv: f64, up: bool) -> Option<f64> {
    if !lv.is_finite() { return None; }
    if if up { g.a >= lv } else { g.a <= lv } { return Some(if g.gap { g.b } else { g.a }); }
    (if up { g.b >= lv } else { g.b <= lv }).then(|| if g.gap { g.b } else { lv })
}

fn walk(s: &mut W, segs: &[Seg], i: usize) -> Vec<Ev> {
    let mut ev = vec![];
    for g in segs {
        let mut g = *g;
        loop {
            if let Some(q) = s.pos {
                let Some(px) = reach(&g, q.stop, q.d < 0.0) else { break };
                s.done.push((q, px));
                s.pos = None;
                ev.push(Ev { k: 1, px, up: q.d < 0.0 });
                g = Seg { a: px, b: g.b, gap: false };
                continue;
            }
            let hit = (0..2).find_map(|k| reach(&g, s.lv[k], k == 0).map(|px| (k, px)));
            let Some((k, px)) = hit else { break };
            let d = if k == 0 { 1.0 } else { -1.0 };
            s.pos = Some(Pos { d, ei: i, ep: px, stop: s.stops[k], rev: 0, amb: false });
            if s.oco { s.lv = [f64::NAN; 2]; } else { s.lv[k] = f64::NAN; }
            ev.push(Ev { k: 0, px, up: d > 0.0 });
            g = Seg { a: px, b: g.b, gap: false };
        }
    }
    ev
}

/// Both rules, one session at a time: `three_day` None = Sagging Highs / Rising Lows; Some(yesterday) = the three-day breakout
/// (false: the group extreme, true: yesterday's high / low).
fn day_trades(x: &Ctx, from: usize, to: usize, n: u16, three_day: Option<bool>) -> Vec<Trade> {
    let tick = x.market_tick();
    let mut out = vec![];
    let push = |out: &mut Vec<Trade>, q: Pos, xi: usize, px: f64, why: &str, amb: bool| {
        let mut z = day_trade(q.d, q.ei, q.ep, xi, px, why, "stop", false);
        if amb || q.amb { z.reason.push_str(super::as51ts::AMB); }
        out.push(z);
    };
    for i in from.max(3)..=to {
        let Some((_, _, m, v)) = crate::intraday_ep::day_bars(x, i, n) else { continue };
        let mut s = W { lv: [f64::NAN; 2], stops: [f64::NAN; 2], oco: three_day.is_none(), ..W::default() };
        if let Some(yday) = three_day {
            let (hh, ll) = if yday { (x.h[i - 1], x.l[i - 1]) } else { (x.h[i - 3].max(x.h[i - 2]).max(x.h[i - 1]), x.l[i - 3].min(x.l[i - 2]).min(x.l[i - 1])) };
            let op = m[0].o;
            s.lv = [if op >= hh + tick { f64::NAN } else { hh + tick }, if op <= ll - tick { f64::NAN } else { ll - tick }];
        }
        let mut prev = m[0].o;
        for (k, b) in v.iter().enumerate() {
            s.stops = if k == 0 { [f64::NAN; 2] } else { [v[k - 1].l - tick, v[k - 1].h + tick] };
            if let Some(q) = s.pos.as_mut() { q.stop = s.stops[if q.d > 0.0 { 0 } else { 1 }]; }
            let (amb, w) = ep6::walk_minutes(&m[b.a..b.b], prev, std::mem::take(&mut s), &|st: &mut W, g: &[Seg]| walk(st, g, i));
            s = w;
            for (q, px) in std::mem::take(&mut s.done) { push(&mut out, q, i, px, if q.d > 0.0 { "lower low" } else { "higher high" }, amb); }
            if amb { if let Some(q) = s.pos.as_mut() { q.amb = true; } }
            prev = b.c;
            let last = k + 1 == v.len();
            if let Some(mut q) = s.pos {
                if (q.d > 0.0 && b.c < b.o) || (q.d < 0.0 && b.c > b.o) { q.rev += 1; }
                s.pos = Some(q);
                let why = if q.rev >= 2 { Some("second reversal bar") } else if last { Some("square-off") } else { None };
                if let Some(why) = why { push(&mut out, q, i, b.c, why, false); s.pos = None; }
            }
            if three_day.is_none() {
                s.lv = [f64::NAN; 2];
                if s.pos.is_none() && !last && k >= 3 {
                    let (h, l) = (|j: usize| v[j].h, |j: usize| v[j].l);
                    if l(k - 3) < l(k - 2) && l(k - 2) < l(k - 1) && l(k - 1) < l(k) { s.lv[0] = h(k) + tick; }
                    if h(k - 3) > h(k - 2) && h(k - 2) > h(k - 1) && h(k - 1) > h(k) { s.lv[1] = l(k) - tick; }
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;

    /// One SPY day of 5-minute bars, each bar given as one minute (09:30 + 5k). Four bars of rising lows, then the break of bar
    /// 3's high fills the long; bar 5 closes lower than it opens (reversal 1), bar 6 again (reversal 2): out at bar 6's close.
    #[test]
    fn rising_lows_entry_and_second_reversal_bar_exit() {
        let bars = [(100.0, 101.0, 99.0, 100.5), (100.5, 101.2, 99.5, 101.0), (101.0, 101.5, 100.0, 101.4), (101.4, 101.8, 100.5, 101.6),
            (101.6, 102.5, 101.5, 102.0), (102.0, 102.6, 101.7, 101.9), (101.9, 102.4, 101.8, 101.85), (101.85, 102.0, 101.6, 101.9)];
        let (x, _rows) = ctx(&bars);
        let t = day_trades(&x, 0, 3, 5, None);
        assert_eq!(t.len(), 1, "{t:?}");
        is(&t[0], 1.0, 101.81, 101.85, "second reversal bar");
    }

    /// The violation stop: long entered on bar 4 (break of 101.80); bar 5 trades below bar 4's low - 1 tick (101.49): out there.
    #[test]
    fn rising_lows_lower_low_violation() {
        let bars = [(100.0, 101.0, 99.0, 100.5), (100.5, 101.2, 99.5, 101.0), (101.0, 101.5, 100.0, 101.4), (101.4, 101.8, 100.5, 101.6),
            (101.6, 102.5, 101.5, 102.4), (102.4, 102.6, 101.2, 101.3)];
        let (x, _rows) = ctx(&bars);
        let t = day_trades(&x, 0, 3, 5, None);
        is(&t[0], 1.0, 101.81, 101.49, "lower low");
    }

    /// Three-day breakout: the last three days' highest high is 105; the day opens at 104 and trades to 106: long at 105.01,
    /// out at the session's last bar (square-off). An opening at 105.5 nullifies the long.
    #[test]
    fn three_day_breakout_fill_and_gap_nullifies() {
        let bars = [(104.0, 104.5, 103.9, 104.4), (104.4, 105.5, 104.3, 105.4), (105.4, 106.0, 105.3, 105.9)];
        let (x, _rows) = ctx(&bars);
        let t = day_trades(&x, 3, 3, 5, Some(false));
        is(&t[0], 1.0, 105.01, 105.9, "square-off");
        let bars = [(105.5, 106.0, 105.4, 105.9), (105.9, 106.5, 105.8, 106.4)];
        let (x, _rows) = ctx(&bars);
        assert!(day_trades(&x, 3, 3, 5, Some(false)).is_empty());
    }

    /// K80_OUT=<csv> VAULT_US=.. VAULT_IDX=.. VAULT_M1_DIR=.. cargo test --release k80_dump -- --ignored; then
    /// authors/titles-1030/K80w/cmp_dump.py. Every trade of both rules (defaults and the labelled option) on the minute tickers.
    #[test]
    #[ignore]
    fn k80_dump() {
        use std::io::Write;
        let mut f = std::io::BufWriter::new(std::fs::File::create(std::env::var("K80_OUT").unwrap()).unwrap());
        for (file, t) in [("VAULT_US", "SPY"), ("VAULT_US", "SPX"), ("VAULT_IDX", "NIFTY 50"), ("VAULT_IDX", "NIFTYBANK")] {
            let rows = crate::load_csv(&std::env::var(file).unwrap()).unwrap().remove(t).unwrap();
            let mut x = Ctx::new(&rows, &rows, &[]);
            x.minutes = Some(Box::leak(Box::new(crate::intraday::Minutes::load(&std::env::var("VAULT_M1_DIR").unwrap(), t).unwrap())));
            let from = rows.partition_point(|r| r.date.as_str() < x.minutes.unwrap().first());
            for (id, kv) in [("jr_tbtm_shrl", ""), ("jr_tbtm_3day", ""), ("jr_tbtm_3day", "level=1")] {
                let mut p = Params::new();
                if let Some((k, v)) = kv.split_once('=') { p.insert(k.into(), v.parse().unwrap()); }
                for z in run(id, &x, from, rows.len() - 1, &p).unwrap() {
                    writeln!(f, "{id}{}{kv},{t},{},{},{},{},{},\"{}\",{}", if kv.is_empty() { "" } else { "|" }, z.dir, rows[z.legs[0].i].date, z.legs[0].px, rows[z.exit_i].date, z.exit_px, z.reason, z.open).unwrap();
                }
            }
        }
    }

    fn is(z: &Trade, d: f64, ep: f64, xp: f64, why: &str) {
        assert!(z.dir == d && (z.legs[0].px - ep).abs() < 1e-9 && (z.exit_px - xp).abs() < 1e-9 && z.reason == why, "{z:?}");
    }

    /// Three daily rows before the test day (highs 103 / 105 / 104, lows 100 / 101 / 102) and the test day 2020-01-08 whose
    /// minutes are the given bars, one minute every 5 minutes from 09:30.
    fn ctx(bars: &[(f64, f64, f64, f64)]) -> (Ctx<'static>, &'static [Row]) {
        let mk = |d: &str, h: f64, l: f64| Row { ticker: "SPY".into(), date: d.into(), open: l, high: h, low: l, close: l, volume: None };
        let day = "2020-01-08";
        let rows: &'static [Row] = Box::leak(vec![mk("2020-01-03", 103.0, 100.0), mk("2020-01-06", 105.0, 101.0), mk("2020-01-07", 104.0, 102.0),
            mk(day, bars.iter().map(|b| b.1).fold(f64::MIN, f64::max), bars.iter().map(|b| b.2).fold(f64::MAX, f64::min))].into_boxed_slice());
        let mut x = Ctx::new(rows, rows, &[]);
        static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let dir = std::env::temp_dir().join(format!("jrtbtm{}-{}", std::process::id(), N.fetch_add(1, std::sync::atomic::Ordering::SeqCst)));
        std::fs::create_dir_all(&dir).unwrap();
        let mut body = String::from("Date,Time,Open,High,Low,Close\n");
        for (k, b) in bars.iter().enumerate() { let m = 570 + 5 * k; body.push_str(&format!("{day},{:02}:{:02},{},{},{},{}\n", m / 60, m % 60, b.0, b.1, b.2, b.3)); }
        let start = "Date,Time,Open,High,Low,Close\n".len();
        std::fs::write(dir.join("T.m1idx.csv"), format!("Date,offset,bytes\n{day},{start},{}\n", body.len() - start)).unwrap();
        std::fs::write(dir.join("T.m1.csv"), &body).unwrap();
        x.minutes = Some(Box::leak(Box::new(crate::intraday::Minutes::load(dir.to_str().unwrap(), "T").unwrap())));
        (x, rows)
    }
}
