//! Jeffrey A. Hirsch, Stock Trader's Almanac 2025 (Wiley; research drive Seasonality Top Reads/10-Stock-Trader-s-Almanac-
//! 2025/Reading.pdf). Book id `hsta25`. Live: the Thanksgiving-Santa Claus rally trade (text-layer page 165, 'Traders
//! feast on small stocks Thanksgiving through Santa Claus rally'). Lane K2 (titles loop, 30 Oct 2026); independent check
//! authors/niksan-k2/admission/k2b_ref.py (header).
use crate::connors::{claim, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

/// Admitted ids (authors/niksan-k2/admission/k2b_ref.py (header)).
pub const REVIEWED: &[&str] = &["hsta25_thanks_santa"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] Close to close, as the book's table prints (Tuesday-before-Thanksgiving close to the close of the second trading day of the New Year). US Thanksgiving = fourth Thursday of November. If the Tuesday is not a trading day of the series (e.g. an Indian holiday), the entry is the last close before it in the same week. Book instrument: the Russell 2000 (S&P 500 and NASDAQ quoted); other indices, ETFs, single stocks and India (on the US Thanksgiving date) are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "hsta25_thanks_santa", name: "Thanksgiving-Santa Claus rally: buy the Tuesday before Thanksgiving, sell on the 2nd trading day of the New Year",
        group: "Stock Trader's Almanac 2025", session: 0, dir: "Long", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "hsta25",
        rules: &["E|\"Buy the Tuesday before Thanksgiving\" (p.165)", "X|\"and hold until the second trading day of the New Year\" (p.165; table 'Thanksgiving-Santa Claus Rally Trade': Tuesday B4 Thanksgiving close and 2nd Trading Day New Year close)"],
        notes: NOTE, claim: claim("45 (Russell 2000), 74 (S&P 500)", "77.8% Russell 2000 (35/45); 79.7% S&P 500 (59/74)", "+3.3% Russell 2000; +2.6% S&P 500", "", "Russell 2000 1979-2023; S&P 500 74 years; NASDAQ 40/53 up, +2.9% (p.165)") }]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == "hsta25_thanks_santa").then_some(("hsta25_thanks_santa", "Thanksgiving-Santa Claus rally trade", "Tuesday before Thanksgiving to New Year day 2"))
}
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    (id == "hsta25_thanks_santa").then(|| vec![ParamDef { key: "day", label: "Exit on this trading day of the New Year (book 2)", def: 2., min: 1., max: 20., step: 1. }])
}
fn days(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400); let yoe = y - era * 400; let mp = (m + 9) % 12;
    era * 146097 + yoe * 365 + yoe / 4 - yoe / 100 + (153 * mp + 2) / 5 + d - 1 - 719468
}
/// Day of November of the Tuesday before US Thanksgiving (fourth Thursday).
pub fn tuesday_before(y: i64) -> i64 { let wd = (days(y, 11, 1) + 3).rem_euclid(7); 1 + (3 - wd).rem_euclid(7) + 21 - 2 }
pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let k = p.get("day").copied().unwrap_or(2.).round().max(1.) as usize;
    let dt = |i: usize| x.r[i].date.as_str();
    let (y0, y1): (i64, i64) = (dt(0)[..4].parse().ok()?, dt(to)[..4].parse().ok()?);
    let mut out = vec![];
    for y in y0..=y1 {
        let td = tuesday_before(y);
        let (tue, mon) = (format!("{y}-11-{td:02}"), format!("{y}-11-{:02}", td - 1));
        // last bar on or before the Tuesday, within that week (from Monday)
        let Some(e) = (0..=to).rev().find(|&i| dt(i) <= tue.as_str()) else { continue };
        if dt(e) < mon.as_str() || e < from || e >= to || out.last().is_some_and(|t: &Trade| e <= t.exit_i) { continue; }
        let ny = format!("{}-01-01", y + 1);
        let xi = (e + 1..x.r.len()).find(|&i| dt(i) >= ny.as_str()).map(|f| f + k - 1).filter(|&i| i < x.r.len() && dt(i)[..4] == ny[..4]);
        let (xi, open) = match xi { Some(i) if i <= to => (i, false), _ => (to, true) };
        out.push(Trade { dir: 1., legs: vec![Leg { i: e, px: x.c[e], w: 1. }], exit_i: xi, exit_px: x.c[xi], reason: if open { "open".into() } else { "New Year trading day · close".into() },
            entry_kind: "close", signal_i: xi, open, exits: vec![] });
        if open { break; }
    }
    Some(out)
}
#[cfg(test)]
mod tests {
    #[test]
    fn thanksgiving_tuesdays() {
        // 2023-11-21, 2024-11-26, 1979-11-20 (book table years)
        assert_eq!(super::tuesday_before(2023), 21); assert_eq!(super::tuesday_before(2024), 26); assert_eq!(super::tuesday_before(1979), 20);
    }
}

// Instrument admission: Jeffrey A. Hirsch (Stock Trader's Almanac 2025) 2026-10-30 (titles lane K2) block, moved from instrument_policy.rs on 31 Oct 2026 (lane ST).
// Admission only for the module REVIEWED lists (authors/niksan-k2/admission/k2b_ref.py (header)); kept in one block to merge.
// Runs on actual indices, ETFs and stocks, US and India (index_etf_or_stock, nifty50 basket, declared application).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book=="hsta25" && crate::books::hsta25::reviewed(def.id),
    label: "The Almanac tests this trade on the Russell 2000 (S&P 500 and NASDAQ quoted); other indices, ETFs, single stocks and India (on the US Thanksgiving date) are a declared application.",
    basis: "Jeffrey A. Hirsch, Stock Trader's Almanac 2025, p.165",
};
