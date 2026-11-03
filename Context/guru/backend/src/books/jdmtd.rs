//! Joe Duarte, "Market Timing for Dummies" (Wiley; edition with 2007-08 examples; research share Trading Knowledge I/
//! Joe Duarte - Marking Timing for Dummies/Market_Timing_For_Dummies.pdf). Book id `jdmtd`. Live: two calendar rules from
//! chapter 6, printed p.90 - the broad seasonal calendar for an S&P 500 or Dow index fund, and the small-cap January
//! effect window (31 December to 28 February). Lane K6 (titles loop, 31 Oct 2026); independent check
//! authors/titles-1030/K6w/k6_ref.py (header).
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::rbtie::target_trades;

/// Admitted ids (authors/titles-1030/K6w/k6_ref.py (header)).
pub const REVIEWED: &[&str] = &["jdmtd_seasonal", "jdmtd_january_effect"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE_S: &str = "[Declared readings] 'In' months: November, December, January, March, April, July and August. A month's position is taken at the close of the last trading day before it and closed at the close of the last 'in' trading day (sell at the end of January, buy back at the end of February, out at the end of April, buy back at the end of June, out at the end of August). August is held by default ('take it or leave it in August'); option: out at the end of July. Cash earns nothing. The book's rule is for an S&P 500 or Dow index fund; other indices, ETFs and US/India stocks are a declared application. The book prints no test of this calendar.";
const NOTE_J: &str = "[Declared readings] Bought at the close of the last trading day of December and sold at the close of the last trading day of February (the book: 'starts on December 31, and extends until February 28'); option: sold at the close of the last trading day on or before 15 February ('If you sell by February 15, you don't lose much'). The book measures the window on the Russell 2000 (average +6.6%); the vault's closest held small-cap series are IWM (US) and NIFTY MIDCAP 100 (India, a mid-cap index); every other index, ETF or stock is a declared application. Cash earns nothing.";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, rules, notes| StrategyDef { id, name, group: "Duarte Market Timing for Dummies", session: 0, dir: "Long", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "jdmtd", rules, notes, claim: None };
    vec![
        d("jdmtd_seasonal", "Seasonal calendar: in November-January, March-April and July-August, out otherwise", &[
            "E|\"own an S&P 500 or Dow index fund in November, December, and January\"; \"buy back in March and April\"; \"buy back in July\" (p.90)",
            "X|\"sell at the end of January and stay out in February\"; out May and June; \"take it or leave it in August\"; \"Get out by September\" (p.90)"], NOTE_S),
        d("jdmtd_january_effect", "January effect window: 31 December to 28 February", &[
            "E|\"The most profitable period ... starts on December 31\" (p.90)",
            "X|\"and extends until February 28, with the average return, as measured by the Russell 2000 index of small stocks being 6.6 percent\"; \"If you sell by February 15, you don't lose much (6.5%)\" (p.90)"], NOTE_J),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id {
        "jdmtd_seasonal" => Some(("jdmtd_seasonal", "Duarte seasonal calendar", "Nov-Jan, Mar-Apr, Jul-Aug")),
        "jdmtd_january_effect" => Some(("jdmtd_january_effect", "Duarte January effect", "31 Dec - 28 Feb")),
        _ => None,
    }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    match id {
        "jdmtd_seasonal" => Some(vec![p("aug", "Hold August (1, default) or sell at the end of July (0)", 1., 0., 1., 1.)]),
        "jdmtd_january_effect" => Some(vec![p("feb15", "Sell at the end of February (0, default) or by 15 February (1)", 0., 0., 1., 1.)]),
        _ => None,
    }
}

fn md(d: &str) -> (u32, u32) { (d[5..7].parse().unwrap_or(0), d[8..10].parse().unwrap_or(0)) }

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let (aug, feb15) = (g("aug").round() >= 1., g("feb15").round() >= 1.);
    let inside = |(m, day): (u32, u32)| if id == "jdmtd_seasonal" { matches!(m, 11 | 12 | 1 | 3 | 4 | 7) || (m == 8 && aug) } else { m == 1 || (m == 2 && (!feb15 || day <= 15)) };
    let n = x.r.len();
    // held over the next bar? decided at this bar's close (the last bar of the data looks at its own date)
    let tgt: Vec<i8> = (0..n).map(|i| inside(md(&x.r[(i + 1).min(n - 1)].date)) as i8).collect();
    Some(target_trades(x, &tgt, from, to, false, "calendar exit"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn calendar_windows_switch_at_month_end_closes() {
        let days = ["2000-10-30", "2000-10-31", "2000-11-01", "2001-01-31", "2001-02-01", "2001-02-15", "2001-02-16", "2001-02-28", "2001-03-01", "2001-04-30", "2001-05-01", "2001-06-29", "2001-07-02", "2001-07-31", "2001-08-01", "2001-08-31", "2001-09-04"];
        let r: Vec<Row> = days.iter().enumerate().map(|(i, d)| Row { ticker: "T".into(), date: d.to_string(), open: 10. + i as f64, high: 10. + i as f64, low: 10. + i as f64, close: 10. + i as f64, volume: None }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let s = |id, p: &Params| run(id, &x, 0, r.len() - 1, p).unwrap().iter().map(|z| (z.legs[0].i, z.exit_i)).collect::<Vec<_>>();
        assert_eq!(s("jdmtd_seasonal", &Params::new()), vec![(1, 3), (7, 9), (11, 15)]);
        let no_aug: Params = [("aug".to_string(), 0.)].into_iter().collect();
        assert_eq!(s("jdmtd_seasonal", &no_aug), vec![(1, 3), (7, 9), (11, 13)]);
        // bar 2 is the last bar before a January bar (the test data skip December); out at the last February close
        assert_eq!(s("jdmtd_january_effect", &Params::new()), vec![(2, 7)]);
        let f15: Params = [("feb15".to_string(), 1.)].into_iter().collect();
        assert_eq!(s("jdmtd_january_effect", &f15), vec![(2, 5)]);
        assert!(run("jdmtd_x", &x, 0, 3, &Params::new()).is_none());
    }
}

// Instrument admission: Joe Duarte (Market Timing for Dummies) 2026-10-31 (titles lane K6).
// Admission only for the module REVIEWED lists (authors/titles-1030/K6w/k6_ref.py (header)).
// Runs on actual indices, ETFs and stocks, US and India (index_etf_or_stock, nifty50 basket, declared application).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "jdmtd" && crate::books::jdmtd::reviewed(def.id),
    label: "Joe Duarte states the calendar for an S&P 500 or Dow index fund and the January window on the Russell 2000; other indices, ETFs and US/India stocks are a declared application.",
    basis: "Joe Duarte, Market Timing for Dummies, chapter 6, printed p.90",
};
