//! James Alphier & Bill Kuhn, "A helping hand from the Arms Index", Technical Analysis of Stocks & Commodities V.5:4 (April 1987)
//! pp.142-143 (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V05/C04/
//! AHELPIN.pdf; Figures 1-2 are image pages, read with authors/ocr2). Book id `scarms`. Lane K40 (4 Nov 2026, decision #204(1));
//! independent check authors/titles-1030/K40w/k40_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::lc3::Pos;

pub const REVIEWED: &[&str] = &["scarms_265"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] Arms Index = NYSE TRIN (decision #204: the vault's NYSE_TRIN, extra_data/us, 1965-03-01 to 2020-02-10; it reproduces the article's readings, e.g. 2.81, 3.31 and 4.09 on 7 July, 11 September and 18 November 1986). Buy at the close of any day with the Arms Index at 2.65 or higher; each new reading of 2.65 or higher while long restarts the count; sell at the close of the last trading day on or before the date eight calendar months after the last reading (Figure 2: 6/24/65 -> 2/24/66, and Saturday anniversaries sold the Friday before, e.g. 12/2/74 -> 8/1/75, 10/25/82 -> 6/24/83; a day 29-31 that the eighth month lacks becomes its last day). 'On or before' reads the next bar's date (the exchange calendar). Long only. Book (Figure 2, S&P 500 6/24/1965-9/2/1985): 11 trades, 10 up, +441.1% compounded against +192.1% buy and hold; the vault's SPX starts in 1983, where the two printed trades are 2/8/84 155.85 -> 10/8/84 162.13 (our closes are the same) and 1/2/85 165.37 -> 9/2/85 187.91 (the vault's SPX file has a 2 September 1985 Labor Day row, volume 0, close 182.80, where this rule sells; the printed 187.91 is the 3 September close). US instruments only: India has no NYSE breadth (needs data). Runs on US indices, ETFs and stocks other than the S&P 500 are a declared application; results end with the TRIN data.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "scarms_265", name: "Alphier-Kuhn Arms Index panic: buy the close of a day with NYSE TRIN at 2.65 or higher, sell 8 calendar months after the last such day", group: "S&C: A helping hand from the Arms Index (Alphier & Kuhn, V.5:4)", session: 0, dir: "Long", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &["NYSE_TRIN"], book: "scarms", rules: &[
            "F|\"The Arms formula: (Advancing Issue / Declining Issues) (AdvancingVolume / DecliningVolume)\" [the article's typesetting; the Arms Index divides the two ratios] \"At 2.65, our arbitrary threshold, there have been but 18 trading days in the past 21 years\" (p.142)",
            "E|\"we simply assumed that an investor 'bought' the S&P 500 on the day of the high Arms Index reading\"; Figure 2 'Simple Strategy': \"Buy: On day of Arms Index 2.65 or higher\" (pp.142-143)",
            "X|\"and then held for eight calendar months and sold. If new high index readings appeared before the eight months were out, we re-started the eight-month count from that new high reading, selling only when the high readings had not occurred for eight full calendar months.\"; Figure 2: \"Sell: 8 calendar months later or after last 'overlapping' 2.65 reading\" (pp.142-143)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { reviewed(id).then_some(("scarms_265", "Alphier-Kuhn Arms Index 2.65 (S&C)", "8 calendar months, restart")) }

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("level", "Arms Index at or above (article 2.65)", 2.65, 1.5, 5., 0.05),
        p("months", "Calendar months held after the last reading (article 8)", 8., 1., 24., 1.)])
}

/// "YYYY-MM-DD" + k calendar months; a day the target month lacks becomes its last day.
pub fn add_months(d: &str, k: i64) -> String {
    let (y, m, dd): (i64, i64, i64) = (d[0..4].parse().unwrap_or(0), d[5..7].parse().unwrap_or(1), d[8..10].parse().unwrap_or(1));
    let z = y * 12 + (m - 1) + k;
    let (y2, m2) = (z.div_euclid(12), z.rem_euclid(12) + 1);
    let last = match m2 { 4 | 6 | 9 | 11 => 30, 2 => if (y2 % 4 == 0 && y2 % 100 != 0) || y2 % 400 == 0 { 29 } else { 28 }, _ => 31 };
    format!("{y2:04}-{m2:02}-{:02}", dd.min(last))
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let Some(trin) = x.series("NYSE_TRIN") else { return Some(vec![]) };
    let (level, months) = (p.get("level").copied().unwrap_or(2.65), p.get("months").copied().unwrap_or(8.).round() as i64);
    let n = x.r.len();
    let mut pos = Pos::new();
    let mut due = String::new();
    for t in from..=to {
        let sig = trin[t] >= level;
        if pos.side() > 0. {
            if sig { due = add_months(&x.r[t].date, months); }
            else if x.r[t].date >= due || (t + 1 < n && x.r[t + 1].date > due) { pos.exit(t, x.c[t], "eight calendar months after the last 2.65 reading"); }
        } else if sig && t < to { pos.enter(1., t, x.c[t], "close"); due = add_months(&x.r[t].date, months); }
    }
    Some(pos.done(to, x.c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn eight_months_after_the_last_reading() {
        assert_eq!(add_months("1974-12-02", 8), "1975-08-02");
        assert_eq!(add_months("2001-01-31", 1), "2001-02-28");
        let r = super::super::sctraub::tests::bars(800);
        let mut x = Ctx::new(&r, &r, &[]);
        let mut v = vec![1.0; r.len()];
        let i = r.iter().position(|b| b.date == "2001-03-05").unwrap();
        let j = r.iter().position(|b| b.date == "2001-06-04").unwrap();
        v[i] = 2.7; v[j] = 3.0; // a second reading restarts the count: due 2002-02-04
        x.extra.insert("NYSE_TRIN".into(), v);
        let t = run("scarms_265", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert_eq!(t.len(), 1);
        assert_eq!((r[t[0].legs[0].i].date.as_str(), r[t[0].exit_i].date.as_str()), ("2001-03-05", "2002-02-04"));
    }
}

// Instrument admission: S&C Alphier-Kuhn Arms Index 2026-11-04 (titles lane K40). India is refused by readiness (no NYSE_TRIN there).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "scarms" && reviewed(def.id),
    label: "James Alphier and Bill Kuhn bought the S&P 500 on NYSE Arms Index readings (1965-1986). The signal is the vault's real NYSE TRIN; runs on US indices, ETFs and stocks are a declared application; Indian instruments have no NYSE breadth.",
    basis: "Technical Analysis of Stocks & Commodities V.5:4 pp.142-143 (A helping hand from the Arms Index), Figures 1-2; independent check authors/titles-1030/K40w/k40_ref.py",
};
