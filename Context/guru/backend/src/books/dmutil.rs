//! Dennis Meyers, "The Utility Average Stock Market Indicator", Technical Analysis of Stocks & Commodities V.13 (1995) p.321-328
//! (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V13/C08/UTILITY.pdf).
//! Book id `dmutil`, under Dennis Meyers. Lane PB (13 Nov 2026, pool B row 433 of authors/a1/A1.tsv; R4 #297); independent check
//! authors/pb/w/pb_ref2.py. Two methods on weekly closes: the UD% switch and the utility relative-strength system.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::rbtie::target_trades;

pub const REVIEWED: &[&str] = &["dmutil_ud", "dmutil_rs"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }
pub const SERIES: &[&str] = &["DJ_UTILITY"];

const NOTE_UD: &str = "[Adapted: 1 unstated detail, R4 #297; unverified] Weekly bars: the last trading day of each Monday-Friday week. Dow Jones Utility Average weekly closes from Norgate ($DJU, 1929-2020-09-04), read on the traded instrument's week-end dates. Buy the S&P 500 at the week's close when the utility average has risen u = 5% or more from its lowest weekly close recorded while short; sell (and go short: the article's 'while we were short') when it has fallen d = 14% or more from its highest weekly close recorded while long (p.322, the best stable values of the 1/4/1980-3/31/1995 optimisation). Unstated: (1) the start, before any position; vault reading: the lowest and highest weekly closes since the series start, the first move of 5% up or 14% down decides. Meyers judges this indicator curve-fitted (the 1953-1975 out-of-sample test is much worse); it is coded because the rule and values are printed. The article's performance tables are images: no comparable table. Runs on SPX and SPY (the S&P 500 is the traded series); ends 2020-09-04 with the Norgate copy.";
const NOTE_RS: &str = "[Adapted: 1 unstated detail, R4 #297; unverified] Weekly bars: the last trading day of each Monday-Friday week. RS = Dow Jones Utility Average weekly close (Norgate $DJU) / S&P 500 weekly close; UtRs = 100 x (RS - RS 19 weeks earlier); UtDr = utility close - utility close 19 weeks earlier (p.323). Buy the S&P on the week's close if UtRs >= Brs = 1.5 and UtDr > 0 for Bcons = 0 weeks (no condition); sell if UtRs <= -Srs = -7 and UtDr < 0 for each of the last Scons = 11 weeks (final values p.324). Unstated: (1) whether the sell goes short; vault reading: stop and reverse (TradeStation buy/sell orders reverse, as the article's UD% system 'while we were short'). The article reports no losing trade 1/4/1980-3/11/1995 and a buy on 7/10/1981 at 129.37; the vault's SPX starts in 1983, so the table cannot be compared. Runs on SPX only: the S&P 500 index close is part of the ratio, and an ETF's price scale would change the thresholds; ends 2020-09-04 with the Norgate copy.";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, notes, rules| StrategyDef { id, name, group: "S&C: The Utility Average Stock Market Indicator (Meyers, V.13)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: SERIES, book: "dmutil", rules, notes, claim: None };
    vec![
        d("dmutil_ud", "Meyers utility UD%: buy the S&P when the Dow utilities rise 5% off their low, short when they fall 14% off their high (weekly)", NOTE_UD, &[
            "E|\"If the utility average rises u % from the weekly lowest low closing price that was recorded while we were short, then we would buy the S&P 500 average on its weekly close.\" (p.322)",
            "X|\"If the utility average falls d % from the weekly highest high closing price that was recorded while long\" -> sell the S&P 500 on its weekly close (p.322); \"The best stable optimization parameters were u % =5 and d % = 14\""]),
        d("dmutil_rs", "Meyers utility relative strength: buy on a 19-week utility/S&P gain of 1.5, sell on -7 with 11 falling weeks (weekly)", NOTE_RS, &[
            "F|\"First, we construct a utility relative strength (RS) ratio by dividing the Dow Jones utilities (Util) by the S&P 500 average (SP). Next, we will calculate the difference between today's utility RS ratio and the utility RS ratio a number of weeks ago... UtRs... Then multiply UtRs by 100.\" UtDr = Util - Util rslag weeks ago (p.323)",
            "E|\"Buy the S&P on the close if: A) UtRs is greater than or equal to the number Brs, and B) UtDr has been greater than zero for Bcons weeks.\" (p.323)",
            "X|\"Sell the S&P on the close if: A) UtRs is less than or equal to the number Srs, and B) UtDr has been less than zero for Scons weeks.\" Final values Brs = 1.5, Bcons = 0, Srs = 7 (used as -7), Scons = 11, rslag = 19 (p.324)"]),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id { "dmutil_ud" => Some(("dmutil_ud", "Meyers utility UD%", "u 5% / d 14%")), "dmutil_rs" => Some(("dmutil_rs", "Meyers utility relative strength", "Final values")), _ => None }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    match id {
        "dmutil_ud" => Some(vec![p("u", "u%: rise from the low to buy (5)", 5., 1., 20., 0.5), p("d", "d%: fall from the high to sell (14)", 14., 1., 30., 0.5)]),
        "dmutil_rs" => Some(vec![p("rslag", "rslag weeks (19)", 19., 2., 52., 1.), p("brs", "Brs (1.5)", 1.5, 0., 10., 0.5), p("bcons", "Bcons weeks (0)", 0., 0., 20., 1.),
            p("srs", "Srs (7, sell at UtRs <= -Srs)", 7., 0., 20., 0.5), p("scons", "Scons weeks (11)", 11., 0., 30., 1.)]),
        _ => None,
    }
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from > to || to >= x.c.len() { return None; }
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap().def);
    let Some(ut) = x.series(SERIES[0]) else { return Some(vec![]) };
    let wk: Vec<usize> = super::apta::bars(x, 1).into_iter().filter(|&i| i <= to && ut[i].is_finite()).collect();
    let mut tgt = vec![0i8; to + 1];
    let mut pos = 0i8;
    let set = |k: usize, pos: i8, tgt: &mut Vec<i8>| { let end = wk.get(k + 1).map_or(to, |&j| j - 1); for t in &mut tgt[wk[k]..=end] { *t = pos; } };
    if id == "dmutil_ud" {
        let (u, d) = (g("u") / 100., g("d") / 100.);
        let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
        for k in 0..wk.len() {
            let v = ut[wk[k]];
            if pos != 1 { lo = lo.min(v); }
            if pos != -1 { hi = hi.max(v); }
            if pos != 1 && v >= lo * (1. + u) { pos = 1; hi = v; }
            else if pos != -1 && v <= hi * (1. - d) { pos = -1; lo = v; }
            set(k, pos, &mut tgt);
        }
    } else {
        let lag = g("rslag").round().max(1.) as usize;
        let (bcons, scons) = (g("bcons").round().max(0.) as usize, g("scons").round().max(0.) as usize);
        let (brs, srs) = (g("brs"), g("srs"));
        let rs: Vec<f64> = wk.iter().map(|&i| ut[i] / x.c[i]).collect();
        let dr = |k: usize| ut[wk[k]] - ut[wk[k - lag]];
        for k in 0..wk.len() {
            if k >= lag {
                let utrs = 100. * (rs[k] - rs[k - lag]);
                let run_of = |n: usize, up: bool| n == 0 || (k + 1 >= lag + n && (k + 1 - n..=k).all(|j| if up { dr(j) > 0. } else { dr(j) < 0. }));
                if pos != 1 && utrs >= brs && run_of(bcons, true) { pos = 1; }
                else if pos != -1 && utrs <= -srs && run_of(scons, false) { pos = -1; }
            }
            set(k, pos, &mut tgt);
        }
    }
    Some(target_trades(x, &tgt, from, to, false, "Utility average signal"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn ctx(ut: &[f64], sp: &[f64]) -> Ctx<'static> {
        // one bar per Friday (each bar is its own week)
        let r: Vec<Row> = sp.iter().enumerate().map(|(k, &c)| Row { ticker: "SPX".into(), date: crate::books::apta::tests_friday(k), open: c, high: c, low: c, close: c, volume: None }).collect();
        let r: &'static [Row] = Box::leak(r.into_boxed_slice());
        let mut x = Ctx::new(r, r, &[]);
        x.extra.insert("DJ_UTILITY".into(), ut.to_vec());
        x
    }
    #[test]
    fn ud_switches_on_5_percent_up_and_14_percent_down() {
        // low 100 -> 105 (buy at week 2), high 110 -> 94.6 (sell short at week 5: 94.5 <= 110 x 0.86)
        let x = ctx(&[100., 101., 105., 110., 100., 94.5, 95.], &[10.; 7]);
        let t = run("dmutil_ud", &x, 0, 6, &Params::new()).unwrap();
        assert_eq!(t.iter().map(|t| (t.dir, t.legs[0].i, t.exit_i, t.open)).collect::<Vec<_>>(), vec![(1., 2, 5, false), (-1., 5, 6, true)]);
    }
    #[test]
    fn rs_buys_on_brs_and_sells_after_scons_falling_weeks() {
        let p = Params::from([("rslag".to_string(), 1.), ("scons".to_string(), 2.)]);
        // S&P flat at 100; utilities 50 -> 52 (UtRs +2 >= 1.5: buy week 1), then 44, 36 (UtRs -8, -8; UtDr < 0 two weeks: sell week 3)
        let x = ctx(&[50., 52., 44., 36., 36.], &[100.; 5]);
        let t = run("dmutil_rs", &x, 0, 4, &p).unwrap();
        assert_eq!(t.iter().map(|t| (t.dir, t.legs[0].i, t.exit_i)).collect::<Vec<_>>(), vec![(1., 1, 3), (-1., 3, 4)]);
        assert!(run("dmutil_x", &x, 0, 4, &p).is_none());
    }
}

// Instrument admission: Dennis Meyers, The Utility Average Stock Market Indicator (S&C 1995), lane PB 2026-11-13.
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "dmutil" && reviewed(def.id),
    label: "Dennis Meyers switches the S&P 500 on the Dow Jones Utility Average's weekly closes; the UD% switch runs on SPX and SPY, the relative-strength system (S&P index close in the ratio) on SPX only, with the unstated detail read as labelled on the card (R4 #297).",
    basis: "Dennis Meyers, The Utility Average Stock Market Indicator, Stocks & Commodities V.13 p.321-328 (VOLUMES/V13/C08/UTILITY.pdf); Dow Jones Utility Average from Norgate (SOURCES norgate-us-indices-20200908-pb); independent check authors/pb/w/pb_ref2.py",
};
