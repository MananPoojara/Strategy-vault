//! Jay Kaeppel, "The Stock Market, The Calendar, And You", Technical Analysis of Stocks & Commodities V.20:12 (December 2002)
//! pp.52-61 (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V20/C12/
//! 164stoc.pdf). Book id `sckcal`: the article's combined "seasonal index" (+2 or higher). Its four tendencies are live cards
//! of Kaeppel's book module kpss (lane K13): T1 = kpss_hol_d2, T3 = kpss_nov_may, T4 = kpss_stoken15; their held days are
//! taken from those cards' own trades (no kpss code changed); T2 (Fosback) is counted here. Lane K15 (titles loop, 2 Nov
//! 2026), to-code row 210, lead decision 93; independent check authors/titles-1030/K15w/k15_ref.py.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const REVIEWED: &[&str] = &["sckcal_si2"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }
/// The holiday tendency is set by US exchange holidays: US instruments only (instrument_policy, as kpss::US_ONLY).
pub const US_ONLY: &[&str] = &["sckcal_si2"];

const NOTE: &str = "[Declared readings] The seasonal index counts, for each trading day, how many of the article's four tendencies hold the market that day (a day = the close-to-close change into that close): (1) the two trading days before each of eight US market holidays (New Year's, Presidents', Good Friday, Memorial, July 4, Labor, Thanksgiving, Christmas), as live card kpss_hol_d2; (2) the last trading day of each month and the first four of the next ('buy at the close on the next to last trading day ... sell at the close on the fourth trading day'); (3) November 1 to the third trading day of May, as kpss_nov_may; (4) October 1 of the year two years before each presidential election to December 31 of the following year, as kpss_stoken15. Long on every day the index is +2 or higher, cash otherwise; consecutive days form one trade (in at the close before the first day, out at the close of the last). Cash interest and the article's 2:1 leverage variant are not modelled. The article tested the Nasdaq Composite (OTC) 1972-2002, which the vault does not hold: NDX/QQQ, SPX, DJI and other US instruments are a declared application. US instruments only: the holiday tendency follows US exchange holidays and no official NSE holiday list is held (lead decisions 4, 75).";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "sckcal_si2", name: "Kaeppel seasonal index: long on days when at least two of four calendar tendencies are bullish", group: "S&C: The Stock Market, The Calendar, And You (Kaeppel, V.20:12)", session: 0, dir: "Long", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "sckcal", rules: &[
            "F|\"Using this approach, the seasonal index can register a daily reading anywhere from zero to +4. If all four of the tendencies discussed are currently bullish, then the seasonal index will register a reading of +4.\" (S&C V.20:12 p.58)",
            "E|\"holding the OTC only when the seasonal index was at +2 or higher would have outperformed a buy and hold approach by about 3 to 1\" (p.58)",
            "F|Tendencies: \"the two preholiday trading days\" (p.53); \"Buy at the close on the next to last trading day of each month, and sell at the close on the fourth trading day of the next month\"; \"Buy at the close on the last trading day of October and sell at the close on the third trading day the following May\"; \"buy at the close on the last trading day of September two years prior to each Presidential election and then sell at the close on the last trading day of December the following year\" (p.52-53)", "X|Out at the close of the last day with the index at +2 or higher (in cash on days below +2, p.58)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == "sckcal_si2").then_some(("sckcal_si2", "Kaeppel seasonal index", "two or more of four tendencies"))
}
pub fn params(id: &str) -> Option<Vec<ParamDef>> { reviewed(id).then(Vec::new) }

/// Days held by a kpss card over the whole series: each trade holds the bars after its entry close through its exit close.
fn held(id: &str, x: &Ctx, w: &mut [u8]) {
    let n = x.r.len();
    for t in crate::books::kpss::run(id, x, 0, n - 1, &Params::new()).unwrap_or_default() {
        for k in t.legs[0].i + 1..=t.exit_i { w[k] += 1; }
    }
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, _p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let n = x.r.len();
    let mut w = vec![0u8; n];
    held("kpss_hol_d2", x, &mut w);
    held("kpss_nov_may", x, &mut w);
    held("kpss_stoken15", x, &mut w);
    // Fosback: the last trading day of a month (a later bar in another month exists) and trading days 1-4
    let ym = |i: usize| &x.r[i].date[..7];
    let mut td = 0;
    for i in 0..n {
        td = if i > 0 && ym(i) == ym(i - 1) { td + 1 } else { 1 };
        if td <= 4 || (i + 1 < n && ym(i + 1) != ym(i)) { w[i] += 1; }
    }
    let on: Vec<bool> = w.iter().map(|&v| v >= 2).collect();
    let mut out = vec![];
    let mut t = from + 1;
    while t <= to {
        if !on[t] { t += 1; continue; }
        let s = t;
        while t < to && on[t + 1] { t += 1; }
        let open = t == to && (to + 1 >= n || on[to + 1]);
        out.push(Trade { dir: 1., legs: vec![Leg { i: s - 1, px: x.c[s - 1], w: 1. }], exit_i: t, exit_px: x.c[t],
            reason: if open { "open".into() } else { "seasonal index below +2 · close".into() }, entry_kind: "close", signal_i: t, open, exits: vec![] });
        t += 1;
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn seasonal_index_holds_month_turns_in_winter_and_is_us_only() {
        // weekdays of 2001-2003
        let mut r = vec![];
        let mut z = 11323i64; // 2001-01-01 (days since 1970-01-01)
        while r.len() < 700 {
            if (z + 3).rem_euclid(7) < 5 {
                let (y, m, d) = { let zz = z + 719_468; let era = zz.div_euclid(146_097); let doe = zz - era * 146_097; let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
                    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); let mp = (5 * doy + 2) / 153; let d = doy - (153 * mp + 2) / 5 + 1; let m = if mp < 10 { mp + 3 } else { mp - 9 };
                    (yoe + era * 400 + (m <= 2) as i64, m, d) };
                let c = 100. + (r.len() % 13) as f64;
                r.push(Row { ticker: "SPY".into(), date: format!("{y:04}-{m:02}-{d:02}"), open: c, high: c + 1., low: c - 1., close: c, volume: None });
            }
            z += 1;
        }
        let x = Ctx::new(&r, &r, &[]);
        let t = run("sckcal_si2", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(!t.is_empty());
        // 2002-12-02 (first trading day of December, inside Nov-May and Stoken's window) is held
        let k = r.iter().position(|b| b.date == "2002-12-02").unwrap();
        assert!(t.iter().any(|q| q.legs[0].i < k && q.exit_i >= k));
        let def = catalogue().remove(0);
        assert!(crate::instrument_policy::check(&def, "NIFTY 50", false).is_some());
        assert!(crate::instrument_policy::check(&def, "SPY", false).is_none());
    }
}

// Instrument admission: Kaeppel S&C V.20:12 seasonal index 2026-11-02 (titles lane K15). US only (instrument_policy).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "sckcal" && reviewed(def.id),
    label: "Kaeppel tested the Nasdaq Composite, which the vault does not hold; runs on US indices, ETFs and stocks are a declared application (US exchange holidays: US instruments only).",
    basis: "Technical Analysis of Stocks & Commodities V.20:12 (Dec 2002) pp.52-61; tendencies as live kpss cards; independent check authors/titles-1030/K15w/k15_ref.py",
};
