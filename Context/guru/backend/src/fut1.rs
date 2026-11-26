//! Q416 (R4 #416, K1): India day-trade cards re-opened on the NIFTY / NIFTYBANK FUT1 1-minute futures series.
//! The cash-index minute result of these cards is withdrawn (decisions #177a, #231, #234, #255: stale component prints, no tradable India minute series).
//! The vault now holds the first-month futures minutes (lane DI: NIFTY 50 2018-12-28.., NIFTYBANK 2023-09-06.., nearest-expiry continuous, NOT back-adjusted,
//! the card runs on a difference back-adjusted series built at each expiry switch of FUT1.days.csv, see back_adjust). The card's own rule then runs on futures daily bars built from those minutes (the
//! sprfs / acbff_fut_ path) with the futures minutes as the intraday source; trades are mapped back to the index rows. Declared adaptation (R4 #340), UNVERIFIED.
//! Only (id, India context) pairs the engine refuses today are opened: no cash-minute fill is ever used and no served result changes.
use crate::connors::{Ctx, Params, Trade};
use std::cell::Cell;

/// Pilot (Q416, 25 Nov 2026): ids with an independent, roll-aware reference at 0 mismatches on the FUT1 bars (K1/fut1/fut1_ref2.py, ref_as51.py); dropped after the lead's 18:05 decision: tdperl_rebo (no minute-order reference), stad2_adxdiff (registered reference cannot be imported, rule too long to rebuild), rwmts_15m_5m_rsi (the registered K53 reference does not reproduce the engine even on SPY cash minutes), jmls_bar_breakout (both pairs held by the India-better test, overnight carry across futures rolls).
pub const REOPEN: &[&str] = &["as51_ema_scalp", "bfmm_esl", "sdtcp_donchian", "jkbk_safe", "tcdt_idp1", "msadd_n_can_ha",
    // batch chunk 1 (Q416 batch, 25 Nov 2026): daily-bar rules, roll-aware references at 0 mismatches
    "bfmm_nql", "dgmf_gap3", "fgosc_crossover", "hbqts_parabolic_stop", "jkbk_starting_point", "krftj_pro", "lkcpt_rsimild", "msadd_n_can_pl", "msws_hanging_man_hammer_mfi", "nkms2_small_triangle_breakout", "rgc_runaway_gap_f5", "rinadz_rsi9"];
/// (id, ticker) pairs withheld by the #231/#234 India-better test against the tradable SPY result (average trade > 3x and > 0.003 pp, AND compounded > 10x): none after the re-run of 25 Nov with the real definition (K1/fut1/india-better-real.json) .
pub const FLAGGED: &[(&str, &str)] = &[];
thread_local! { static ACTIVE: Cell<bool> = const { Cell::new(false) }; static REFUSAL: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) }; }
/// The runtime refusal set by reopen_with (R4 #419): main.rs turns it into the run's error instead of publishing a result.
pub fn take_refusal() -> Option<String> { REFUSAL.with(|r| r.borrow_mut().take()) }
pub fn applies(id: &str, ticker: &str) -> bool { REOPEN.contains(&id) && matches!(ticker, "NIFTY 50" | "NIFTYBANK") && !FLAGGED.contains(&(id, ticker)) }
pub const NOTE: &str = "FUT1 ADAPTATION (R4 #416, #340, UNVERIFIED): on NIFTY 50 / NIFTYBANK this rule runs on the first-month futures 1-minute series (lane DI) and futures daily bars built from it (open = first minute, close = last minute before 15:30), never on cash-index minutes; the window is the days the futures file holds (NIFTY 50 from 2018-12-28, NIFTYBANK from 2023-09-06). The series is nearest-expiry continuous; for the multi-day daily-bar rules the signals and indicators run on a DIFFERENCE back-adjusted series (at each expiry switch of FUT1.days.csv the gap = the new contract's first open minus the old contract's last close is removed from all earlier bars, so the roll-day overnight move is dropped with it), and prices are shown on the contract held at entry (adjusted price + the entry bar's offset, as futures.rs does): a trade across a roll earns the adjusted points, never the roll gap. The same-day minute rules run on the raw minutes and take no entry on a roll day (their EMA carries the old contract's closes). The book's market is not NSE index futures: no printed result is comparable.";
/// The policy source_basis with the FUT1 label appended (cached: policy() returns &'static str).
pub fn basis(id: &str, old: &'static str) -> &'static str {
    static C: std::sync::Mutex<std::collections::BTreeMap<String, &'static str>> = std::sync::Mutex::new(std::collections::BTreeMap::new());
    let mut m = C.lock().unwrap_or_else(|e| e.into_inner());
    m.entry(id.to_string()).or_insert_with(|| Box::leak(format!("{old} {NOTE}").into_boxed_str()))
}
/// Cards that decide and fill inside one day on the raw FUT1 minutes (no roll can lie between their entry and exit): raw bars.
pub const SAME_DAY: &[&str] = &["as51_ema_scalp"];
/// date -> contract expiry from `<T>.FUT1.days.csv` (next to the minute file); empty when unreadable.
fn expiries(f: &crate::intraday::Minutes) -> std::collections::BTreeMap<String, String> {
    let p = f.file().to_string_lossy().replace(".m1.csv", ".days.csv");
    std::fs::read_to_string(p).map(|t| t.lines().skip(1).filter_map(|l| { let c: Vec<&str> = l.split(',').collect(); (c.len() > 1).then(|| (c[0].to_string(), c[1].to_string())) }).collect()).unwrap_or_default()
}
/// Difference back-adjustment: a roll sits at row i when the day's expiry differs from the row before; its gap is open[i] - close[i-1]
/// (the new contract's first open against the old contract's last close). Every earlier row is shifted by the sum of the later gaps, so the
/// series has no roll gap; offset[j] = that sum (adjusted = raw + offset[j]: the older contract is lifted to the new one's level; the latest contract is raw). Returns (adjusted rows, offsets, rolls).
pub fn back_adjust(rows: &[crate::Row], exp: &[String]) -> (Vec<crate::Row>, Vec<f64>, usize) {
    let n = rows.len();
    let mut off = vec![0.0; n];
    let (mut acc, mut rolls) = (0.0, 0);
    for j in (0..n).rev() {
        off[j] = acc;
        if j > 0 && exp[j] != exp[j - 1] { acc += rows[j].open - rows[j - 1].close; rolls += 1; }
    }
    let adj = rows.iter().zip(&off).map(|(r, o)| crate::Row { open: r.open + o, high: r.high + o, low: r.low + o, close: r.close + o, ..r.clone() }).collect();
    (adj, off, rolls)
}
/// Runs the card on the futures daily bars (+ futures minutes for the same-day rules) of an India context; None when the pair is not re-opened.
pub fn reopen(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if ACTIVE.with(|a| a.get()) || !applies(id, &x.r.first()?.ticker) { return None; }
    reopen_with(id, x, from, to, &|y| crate::connors::run(id, y, 0, y.r.len() - 1, p))
}
/// The roll-aware wrapper; `run` is the card's own run on the prepared futures context (a closure so the wrapper can be tested on its own).
pub fn reopen_with(id: &str, x: &Ctx, from: usize, to: usize, run: &dyn Fn(&Ctx) -> Option<Vec<Trade>>) -> Option<Vec<Trade>> {
    let (rows, map) = crate::books::sprfs::fut_daily_rows(x, from, to);
    if rows.len() < 2 { return Some(vec![]); }
    let same_day = SAME_DAY.contains(&id);
    let ex = x.fut.map(expiries).unwrap_or_default();
    let e: Vec<String> = rows.iter().map(|r| ex.get(&r.date).cloned().unwrap_or_default()).collect();
    if e.iter().any(|s| s.is_empty()) { return Some(vec![]); } // no expiry table for a day: never guess a roll
    let roll_day: Vec<bool> = (0..rows.len()).map(|j| j > 0 && e[j] != e[j - 1]).collect();
    let (use_rows, off) = if same_day { (rows.clone(), vec![0.0; rows.len()]) } else { let (a, o, _) = back_adjust(&rows, &e); (a, o) };
    let mut y = Ctx::new(&use_rows, &use_rows, &[]);
    if same_day { y.minutes = x.fut.map(|f| f.cut_at(15 * 60 + 30)); }
    y.fut = x.fut;
    ACTIVE.with(|a| a.set(true));
    let r = run(&y);
    ACTIVE.with(|a| a.set(false));
    // same-day rules keep EMA state across days: no entry on a roll day (the first day of a new contract), where the state still holds the old contract's closes
    let r = r.map(|t| if same_day { t.into_iter().filter(|z| !roll_day[z.legs[0].i]).collect() } else { t });
    // runtime guard (also in release builds, R4 #419): a multi-leg or scale-out trade would need its own entry-bar offset per leg; the cards in REOPEN open one
    // position per trade, so anything else is refused, never priced on one offset.
    if r.as_ref().is_some_and(|t| t.iter().any(|z| z.legs.len() != 1 || !z.exits.is_empty())) {
        REFUSAL.with(|f| *f.borrow_mut() = Some(format!("calculation_unavailable: FUT1 re-opening of {id}: a trade with more than one leg or a scale-out cannot be priced on the contract held at entry. No performance is published.")));
        return Some(vec![]);
    }
    // prices on the contract held at entry: adjusted price - the entry bar's offset (single-leg trades only, guarded above)
    r.map(|mut t| { for z in &mut t { let o = off[z.legs[0].i]; for l in &mut z.legs { l.px -= o; } z.exit_px -= o; } crate::books::sprfs::remap(t, &map) })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connors::Leg;
    fn row(d: &str, o: f64, h: f64, l: f64, c: f64) -> crate::Row { crate::Row { ticker: "NIFTY 50".into(), date: d.into(), open: o, high: h, low: l, close: c, volume: None } }
    #[test]
    fn back_adjust_removes_the_gap() {
        let rows = vec![row("2020-01-01", 100., 101., 99., 100.), row("2020-01-02", 100., 101., 99., 100.), row("2020-01-03", 130., 131., 129., 130.), row("2020-01-06", 130., 131., 129., 130.)];
        let exp: Vec<String> = ["A", "A", "B", "B"].iter().map(|s| s.to_string()).collect();
        let (adj, off, rolls) = back_adjust(&rows, &exp);
        assert_eq!((rolls, off), (1, vec![30., 30., 0., 0.]));
        assert!(adj.windows(2).all(|w| (w[1].open - w[0].close).abs() < 1e-9), "no gap left in the adjusted series");
    }
    /// Drives reopen_with on a real FUT1 minute file + FUT1.days.csv (written to a temp dir): expiry parsing, the daily bars built from the minutes,
    /// the roll day, the roll filter of the same-day rule and the price shift of a trade that spans the roll.
    #[test]
    fn reopen_on_a_real_fut1_roll_pays_no_gap_and_filters_roll_day_entries() {
        let dir = std::env::temp_dir().join(format!("fut1-roll-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        // four days, two minutes each (09:15, 09:16), volume > 0; contract A on days 1-2, contract B from day 3 opening 30 points higher (the roll gap)
        let days = [("2020-01-01", 100.), ("2020-01-02", 100.), ("2020-01-03", 130.), ("2020-01-06", 130.)];
        let mut m = String::from("Date,Time,Open,High,Low,Close,Volume,OI\n");
        let mut d = String::from("Date,expiry,source,bars,first,last,volume_verified,vol_over_bhav_contracts,flag\n");
        for (i, (date, p)) in days.iter().enumerate() {
            m += &format!("{date},09:15,{p},{},{},{p},10,1\n{date},09:16,{p},{},{},{p},10,1\n", p + 1., p - 1., p + 1., p - 1.);
            d += &format!("{date},{},store,2,09:15,09:16,1,75.0,\n", if i < 2 { "2020-01-30" } else { "2020-02-27" });
        }
        std::fs::write(dir.join("NIFTY_50.FUT1.m1.csv"), m).unwrap();
        std::fs::write(dir.join("NIFTY_50.FUT1.days.csv"), d).unwrap();
        let cash: Vec<crate::Row> = days.iter().map(|(date, p)| row(date, *p, p + 1., p - 1., *p)).collect();
        let mut x = Ctx::new(&cash, &cash, &[]);
        x.fut = crate::intraday::Minutes::scan_fut(dir.join("NIFTY_50.FUT1.m1.csv").to_str().unwrap()).map(|m| &*Box::leak(Box::new(m)));
        assert!(x.fut.is_some());
        let trade = |dir: f64, e: usize, xi: usize, epx: f64, xpx: f64| Trade { dir, legs: vec![Leg { i: e, px: epx, w: 1. }], exit_i: xi, exit_px: xpx, reason: "stub".into(), entry_kind: "close", signal_i: xi, open: false, exits: vec![] };
        // multi-day rule: a long from day 2 (index 1) to day 4 (index 3) across the roll; on the ADJUSTED rows both closes are 130 (no gap), so the trade earns 0
        let t = reopen_with("sdtcp_donchian", &x, 0, 3, &|y| { assert!((y.r[1].close - 130.).abs() < 1e-9 && (y.r[3].close - 130.).abs() < 1e-9, "the card sees the adjusted series: {:?}", y.r.iter().map(|r| r.close).collect::<Vec<_>>()); Some(vec![trade(1., 1, 3, y.r[1].close, y.r[3].close)]) }).unwrap();
        assert_eq!(t.len(), 1);
        assert_eq!((t[0].legs[0].px, t[0].exit_px), (100., 100.), "prices are shown on the contract held at entry (the old one: 100)");
        assert_eq!(t[0].ret(0.), 0.0, "no roll gap as profit");
        // the raw rows would have paid +30 on the same trade
        assert_eq!((cash[3].close - cash[1].close) / cash[1].close, 0.3);
        // same-day rule: raw rows; an entry on the roll day (index 2) is dropped, the others stay
        let s = reopen_with("as51_ema_scalp", &x, 0, 3, &|y| { assert!((y.r[2].open - 130.).abs() < 1e-9 && (y.r[1].close - 100.).abs() < 1e-9, "raw rows for the same-day rule"); Some(vec![trade(1., 1, 1, 100., 101.), trade(1., 2, 2, 130., 131.), trade(1., 3, 3, 130., 131.)]) }).unwrap();
        assert_eq!(s.iter().map(|z| z.legs[0].i).collect::<Vec<_>>(), vec![1, 3], "the roll-day entry is dropped");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_multi_leg_trade_is_refused_at_runtime() {
        let dir = std::env::temp_dir().join(format!("fut1-leg-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let days = [("2020-01-01", 100.), ("2020-01-02", 100.), ("2020-01-03", 130.)];
        let mut m = String::from("Date,Time,Open,High,Low,Close,Volume,OI\n");
        let mut d = String::from("Date,expiry,source,bars,first,last,volume_verified,vol_over_bhav_contracts,flag\n");
        for (i, (date, p)) in days.iter().enumerate() {
            m += &format!("{date},09:15,{p},{},{},{p},10,1\n", p + 1., p - 1.);
            d += &format!("{date},{},store,1,09:15,09:15,1,75.0,\n", if i < 2 { "A" } else { "B" });
        }
        std::fs::write(dir.join("NIFTY_50.FUT1.m1.csv"), m).unwrap();
        std::fs::write(dir.join("NIFTY_50.FUT1.days.csv"), d).unwrap();
        let cash: Vec<crate::Row> = days.iter().map(|(date, p)| row(date, *p, p + 1., p - 1., *p)).collect();
        let mut x = Ctx::new(&cash, &cash, &[]);
        x.fut = crate::intraday::Minutes::scan_fut(dir.join("NIFTY_50.FUT1.m1.csv").to_str().unwrap()).map(|m| &*Box::leak(Box::new(m)));
        let two = Trade { dir: 1., legs: vec![Leg { i: 0, px: 130., w: 0.5 }, Leg { i: 1, px: 130., w: 0.5 }], exit_i: 2, exit_px: 130., reason: "stub".into(), entry_kind: "close", signal_i: 2, open: false, exits: vec![] };
        let r = reopen_with("sdtcp_donchian", &x, 0, 2, &|_| Some(vec![two.clone()])).unwrap();
        assert!(r.is_empty(), "no trades are published");
        let msg = take_refusal().expect("a refusal is set");
        assert!(msg.starts_with("calculation_unavailable:") && msg.contains("more than one leg"), "{msg}");
        assert!(take_refusal().is_none(), "taken once");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
