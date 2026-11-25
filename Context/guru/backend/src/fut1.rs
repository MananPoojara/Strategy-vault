//! Q416 (R4 #416, K1): India day-trade cards re-opened on the NIFTY / NIFTYBANK FUT1 1-minute futures series.
//! The cash-index minute result of these cards is withdrawn (decisions #177a, #231, #234, #255: stale component prints, no tradable India minute series).
//! The vault now holds the first-month futures minutes (lane DI: NIFTY 50 2018-12-28.., NIFTYBANK 2023-09-06.., nearest-expiry continuous, NOT back-adjusted,
//! a roll shows as a gap between days; a day trade never spans one). The card's own rule then runs on futures daily bars built from those minutes (the
//! sprfs / acbff_fut_ path) with the futures minutes as the intraday source; trades are mapped back to the index rows. Declared adaptation (R4 #340), UNVERIFIED.
//! Only (id, India context) pairs the engine refuses today are opened: no cash-minute fill is ever used and no served result changes.
use crate::connors::{Ctx, Params, Trade};
use std::cell::Cell;

/// Pilot (Q416, 25 Nov 2026): one id per family; the batch follows the lead's review.
pub const REOPEN: &[&str] = &["as51_ema_scalp", "tdperl_rebo", "bfmm_esl", "stad2_adxdiff", "rwmts_15m_5m_rsi", "sdtcp_donchian", "jkbk_safe", "tcdt_idp1", "jmls_bar_breakout", "msadd_n_can_ha"];
/// (id, ticker) pairs flagged by the #231/#234 India-better test against the tradable SPY result: not published (filled in by the pilot report).
pub const FLAGGED: &[(&str, &str)] = &[("tdperl_rebo", "NIFTY 50"), ("bfmm_esl", "NIFTYBANK"), ("stad2_adxdiff", "NIFTYBANK"), ("rwmts_15m_5m_rsi", "NIFTYBANK"), ("sdtcp_donchian", "NIFTYBANK"), ("jkbk_safe", "NIFTY 50"), ("tcdt_idp1", "NIFTY 50"), ("jmls_bar_breakout", "NIFTY 50"), ("jmls_bar_breakout", "NIFTYBANK")]; // pilot test 25 Nov: India sum return opposite in sign to, or > 10x, the SPY sum over the same dates (K1/fut1/india-better.json)
thread_local! { static ACTIVE: Cell<bool> = const { Cell::new(false) }; }
pub fn applies(id: &str, ticker: &str) -> bool { REOPEN.contains(&id) && matches!(ticker, "NIFTY 50" | "NIFTYBANK") && !FLAGGED.contains(&(id, ticker)) }
pub const NOTE: &str = "FUT1 ADAPTATION (R4 #416, #340, UNVERIFIED): on NIFTY 50 / NIFTYBANK this rule runs on the first-month futures 1-minute series (lane DI) and futures daily bars built from it (open = first minute, close = last minute before 15:30), never on cash-index minutes; the window is the days the futures file holds (NIFTY 50 from 2018-12-28, NIFTYBANK from 2023-09-06); the series is nearest-expiry continuous and not back-adjusted, so a roll appears as a gap between two days (trades never span one). The book's market is not NSE index futures: no printed result is comparable.";
/// The policy source_basis with the FUT1 label appended (cached: policy() returns &'static str).
pub fn basis(id: &str, old: &'static str) -> &'static str {
    static C: std::sync::Mutex<std::collections::BTreeMap<String, &'static str>> = std::sync::Mutex::new(std::collections::BTreeMap::new());
    let mut m = C.lock().unwrap_or_else(|e| e.into_inner());
    m.entry(id.to_string()).or_insert_with(|| Box::leak(format!("{old} {NOTE}").into_boxed_str()))
}
/// Runs the card on the futures daily bars + futures minutes of an India context; None when the pair is not re-opened.
pub fn reopen(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if ACTIVE.with(|a| a.get()) || !applies(id, &x.r.first()?.ticker) { return None; }
    let (rows, map) = crate::books::sprfs::fut_daily_rows(x, from, to);
    if rows.len() < 2 { return Some(vec![]); }
    let mut y = Ctx::new(&rows, &rows, &[]);
    y.minutes = x.fut.map(|f| f.cut_at(15 * 60 + 30));
    y.fut = x.fut;
    ACTIVE.with(|a| a.set(true));
    let r = crate::connors::run(id, &y, 0, rows.len() - 1, p);
    ACTIVE.with(|a| a.set(false));
    r.map(|t| crate::books::sprfs::remap(t, &map))
}
