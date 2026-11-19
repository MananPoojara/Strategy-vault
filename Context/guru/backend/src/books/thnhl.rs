//! Tim Hayes (Ned Davis Research), "New Highs And New Lows", Technical Analysis of Stocks & Commodities V.14:3 (1996) p.97-102,
//! Figure 4 (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V14/C03/
//! NEWHIGH.pdf). Book id `thnhl`, under Timothy Hayes. Lane PB (13 Nov 2026, pool B row 398 of authors/a1/A1.tsv; R4 #297);
//! independent check authors/pb/w/pb_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::rbtie::target_trades;

pub const REVIEWED: &[&str] = &["thnhl_ema55"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }
pub const SERIES: &[&str] = &["NYSE_NEW_HIGHS", "NYSE_NEW_LOWS"];

const NOTE: &str = "[Adapted: 2 unstated details, R4 #297; unverified] Indicator = 55-day exponential moving average (alpha 2/56, seeded with the first daily value) of 100 x NYSE new 52-week highs / (new highs + new lows), Norgate #NYSEHI/#NYSELO (1932-2020-09-04), read on the traded instrument's dates; a day with no breadth value or with no new highs and no new lows leaves the average unchanged. Buy (long only, when flat) at the close of the first day the average is above 21 after it has been below 21 (Figure 4: 'buy signals by dropping to an oversold level below 21 and then rising back above that level'). Sell when the average has fallen at least 40.5 points from its peak ('rising to a peak and then dropping by at least 40 and a half points'). Unstated in the article: (1) where the peak is measured from; vault reading: the highest average since the buy; (2) the fill; vault reading: the signal day's close (option: next open). Hayes reports 20 profitable of 22 signals since November 1962 without naming the traded index or listing the trades, so there is no comparable table. NYSE breadth: runs only on the US indices and ETFs SPX, SPY, DJI, DIA, NDX, QQQ (as R4 #306); results end 2020-09-04, where the Norgate breadth copy ends.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "thnhl_ema55", name: "Hayes NH/(NH+NL) 55-day EMA: buy the rise back above 21, sell after a 40.5-point drop from the peak", group: "S&C: New Highs And New Lows (Hayes, V.14:3)",
        session: 0, dir: "Long", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: SERIES, book: "thnhl", rules: &[
            "F|\"The indicator determines the ratio of new highs to the total of new highs and new lows and then smoothes the result with a 55-day exponential moving average (EMA).\" (p.100)",
            "E|\"It produces buy signals by dropping to an oversold level below 21 and then rising back above that level\" (p.100; Figure 4)",
            "X|\"and it generates sell signals by rising to a peak and then dropping by at least 40 and a half points, indicating serious breadth damage.\" (p.100; Figure 4: 'flashes sell signals after 40.5-point declines')"],
        notes: NOTE, claim: crate::connors::claim("22 signals", "91% (20 of 22)", "n/a", "n/a", "since November 1962 (to Dec 1995), index not named") }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    reviewed(id).then_some(("thnhl_ema55", "Hayes new highs / new lows", "21 / 40.5 points"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("n", "EMA days (55)", 55., 5., 200., 1.), p("lo", "Buy: back above this oversold level (21)", 21., 5., 50., 1.),
        p("drop", "Sell: points below the peak (40.5)", 40.5, 5., 80., 0.5), p("open", "Fills: 0 = signal day's close (default), 1 = next open", 0., 0., 1., 1.)])
}

/// 55-day EMA of 100*NH/(NH+NL) on the instrument's bars; a bar without a usable ratio carries the previous value.
pub fn indicator(nh: &[f64], nl: &[f64], n: usize) -> Vec<f64> {
    let a = 2. / (n as f64 + 1.);
    let mut e = f64::NAN;
    nh.iter().zip(nl).map(|(&h, &l)| {
        let r = 100. * h / (h + l);
        if r.is_finite() { e = if e.is_finite() { e + a * (r - e) } else { r }; }
        e
    }).collect()
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from > to || to >= x.c.len() { return None; }
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap().def);
    let (Some(nh), Some(nl)) = (x.series(SERIES[0]), x.series(SERIES[1])) else { return Some(vec![]) };
    let e = indicator(&nh[..=to], &nl[..=to], g("n").round().max(1.) as usize);
    let (lo, drop) = (g("lo"), g("drop"));
    let mut tgt = vec![0i8; to + 1];
    let (mut armed, mut long, mut peak) = (false, false, f64::NAN);
    for i in 0..=to {
        let v = e[i];
        if v.is_finite() {
            if long {
                peak = peak.max(v);
                if v <= peak - drop { long = false; armed = v < lo; }
            } else if armed && v > lo { long = true; peak = v; armed = false; }
            else if v < lo { armed = true; }
        }
        tgt[i] = long as i8;
    }
    Some(target_trades(x, &tgt, from, to, g("open") > 0.5, "Hayes NH/NL signal"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn buys_back_above_21_and_sells_40_5_below_the_peak() {
        let n = 40;
        let r: Vec<Row> = (0..n).map(|i| Row { ticker: "SPX".into(), date: format!("2001-{:02}-{:02}", 1 + i / 28, 1 + i % 28), open: 100. + i as f64, high: 101. + i as f64, low: 99. + i as f64, close: 100. + i as f64, volume: None }).collect();
        let mut x = Ctx::new(&r, &r, &[]);
        // with n = 1 the indicator is the raw ratio: 10% (below 21) .. 30% (buy at bar 5) .. 90% peak .. 49% (drop 41 -> sell at bar 20)
        let ratio: Vec<f64> = (0..n).map(|i| match i { 0..=4 => 10., 5..=9 => 30., 10..=14 => 90., 15..=19 => 60., _ => 49. }).collect();
        x.extra.insert("NYSE_NEW_HIGHS".into(), ratio.clone());
        x.extra.insert("NYSE_NEW_LOWS".into(), ratio.iter().map(|r| 100. - r).collect());
        let p = Params::from([("n".to_string(), 1.)]);
        let t = run("thnhl_ema55", &x, 0, n - 1, &p).unwrap();
        assert_eq!(t.iter().map(|t| (t.legs[0].i, t.exit_i, t.open)).collect::<Vec<_>>(), vec![(5, 20, false)]);
        assert_eq!(t[0].legs[0].px, x.c[5]);
        // no NH+NL on a day: the average carries; the default 55-day EMA is seeded with the first ratio
        let e = indicator(&[1., 0., 3.], &[1., 0., 1.], 3);
        assert!((e[0] - 50.).abs() < 1e-12 && e[1] == e[0] && (e[2] - 62.5).abs() < 1e-12);
        assert!(run("thnhl_x", &x, 0, 5, &p).is_none());
    }
}

// Instrument admission: Tim Hayes, New Highs And New Lows (S&C 1996), lane PB 2026-11-13.
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "thnhl" && reviewed(def.id),
    label: "Tim Hayes's NYSE new-high/new-low indicator times 'the market' without naming the traded index; it runs on the US indices and ETFs SPX, SPY, DJI, DIA, NDX and QQQ only (NYSE breadth, R4 #306), with the two unstated details read as labelled on the card (R4 #297).",
    basis: "Tim Hayes, New Highs And New Lows, Stocks & Commodities V.14:3 p.97-102, Figure 4 (VOLUMES/V14/C03/NEWHIGH.pdf); NYSE new highs/lows from Norgate (SOURCES norgate-us-indices-20200908-pb); independent check authors/pb/w/pb_ref.py",
};
