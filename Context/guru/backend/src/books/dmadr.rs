//! Dennis Meyers, "A Daily A-D New High-New Low Market System", Technical Analysis of Stocks & Commodities V15:1 (1997) p.11-19
//! (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V15/C01/ADAILYA.pdf).
//! Book id `dmadr`, under Dennis Meyers. Lane PF (13 Nov 2026, A1 row 436; R4 #297); independent check authors/pf/w/pf_ref.py.
//! The shared core `rs_system` also runs Meyers' A/D-volume twin (dmvrs, S&C V14:7): only the breadth ratio differs.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::rbtie::target_trades;

pub const REVIEWED: &[&str] = &["dmadr_daily"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }
pub const SERIES: &[&str] = &["NYSE_ADV_DEC.Advances", "NYSE_ADV_DEC.Declines", "NYSE_NEW_HIGHS", "NYSE_NEW_LOWS", "DJ_INDUSTRIAL"];
pub const US_BREADTH: &[&str] = &["SPX", "SPY", "DJI", "DIA", "NDX", "QQQ"];

const NOTE: &str = "[Adapted: 2 unstated details, R4 #297] Run bar by bar on the close; 'buy' and 'sell' reverse the position (always in after the first signal, Figure 7). ai/di = NYSE advancing/declining issues (vault NYSE_ADV_DEC, 1965-03-01..2020-02-10); nh/nl = NYSE 52-week new highs/lows (Norgate #NYSEHI/#NYSELO, SOURCES norgate-us-indices-20200908-pb); djc = the DJIA close (Norgate $DJI, SOURCES norgate-us-indices-20200908-pf), read on the traded instrument's dates. Unstated in the article: (1) the EMA starting value; vault reading: 0, as a TradeStation variable (the 10-day EMA forgets it within weeks); (2) the printed 'adrx = adr[1] + a (adr - adr[1])' is read as the exponential moving average the text names, adrx = adrx[1] + a (adr - adrx[1]). A day without a breadth value keeps the averages unchanged; a zero range makes Strength undefined (no signal that day). If the buy and sell rules both hold on a bar (possible only with Brs <= Srs), the position is kept (the reading the V15:3 trade list shows for this author's systems). The S&P 500 is the traded instrument: the article's tests are on the S&P 500 index; the runs here are on the US indices and ETFs only (NYSE breadth, R4 #306), ending with the breadth data.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "dmadr_daily", name: "Meyers daily ADRrs-NH-NL: advance/decline strength vs the Dow, with NYSE new-high/new-low strength, stop and reverse", group: "S&C: A Daily A-D New High-New Low Market System (Meyers, V15:1)",
        session: 0, dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: SERIES, book: "dmadr", rules: &[
            "F|adr = (ai-di)/(ai+di); adrx, nlx, nhx = exponential moving averages of adr (a = 2/(1+10)), nl and nh (a = 2/(1+3)) (p.12)",
            "F|Strength(x,d) = 100{(x - lowest(x,d))/range(x,d)} - 50; NLstr = Strength(nlx,Ldays); NHstr = Strength(nhx,Hdays); ADRstr = Strength(adrx,xdays); DJCstr = Strength(djc,xdays); ADRrs = ADRstr - DJCstr (p.12)",
            "E|\"If ADRrs is greater than or equal to the value Brs and NLstr is less than or equal to the value NLbx and NHstr is greater than or equal to the value NHbx, then Buy the S&P 500 on the close.\" (p.13)",
            "X|\"If ADRrs is less than or equal to the value Srs and NLstr is greater than or equal to the value NLsx and NHstr is less than or equal to the value NHsx, then Sell the S&P 500 on the close.\" (p.13); the sell reverses to short (Figure 7)",
            "F|Optimum values (Figures 3-5): Brs 4, Srs -40, xdays 10, NLbx -10, NLsx 30, Ldays 40, NHbx -10, NHsx 0, Hdays 10"],
        notes: NOTE, claim: crate::connors::claim("47", "81%", "1,026.57 S&P 500 points total net profit", "n/a", "S&P 500 daily 1/3/1978-8/16/1996 (Figure 5; trade list Figure 7)") }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    reviewed(id).then_some(("dmadr_daily", "Meyers daily ADRrs-NH-NL", "Optimum values"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("brs", "Brs: ADRrs buy level", 4., -40., 40., 2.), p("srs", "Srs: ADRrs sell level", -40., -80., 20., 5.),
        p("xdays", "xdays: ADRstr/DJCstr lookback", 10., 4., 30., 2.), p("nlbx", "NLbx", -10., -50., 50., 10.), p("nlsx", "NLsx", 30., -50., 50., 10.),
        p("ldays", "Ldays", 40., 5., 100., 5.), p("nhbx", "NHbx", -10., -50., 50., 10.), p("nhsx", "NHsx", 0., -50., 50., 10.), p("hdays", "Hdays", 10., 3., 50., 1.)])
}

/// Exponential moving average starting at 0 (a TradeStation variable); a missing value keeps the average unchanged.
pub fn ema0(v: &[f64], a: f64) -> Vec<f64> {
    let mut x = 0.0;
    v.iter().map(|&s| { if s.is_finite() { x += a * (s - x); } x }).collect()
}
/// Exponential moving average seeded with the first finite value; a missing value keeps the average unchanged.
pub fn ema_first(v: &[f64], a: f64) -> Vec<f64> {
    let mut x = f64::NAN;
    v.iter().map(|&s| { if s.is_finite() { x = if x.is_finite() { x + a * (s - x) } else { s }; } x }).collect()
}
/// Meyers' Strength(x,d) = 100 (x - lowest(x,d)) / range(x,d) - 50 over the last d bars including today; NaN without d bars or range.
pub fn strength(x: &[f64], i: usize, d: usize) -> f64 {
    if d == 0 || i + 1 < d { return f64::NAN; }
    let w = &x[i + 1 - d..=i];
    if w.iter().any(|v| !v.is_finite()) { return f64::NAN; }
    let (lo, hi) = w.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |a, &v| (a.0.min(v), a.1.max(v)));
    if !(hi - lo > 0.0) { return f64::NAN; }
    100.0 * (x[i] - lo) / (hi - lo) - 50.0
}

/// The (A/D or volume) ratio strength minus Dow strength, with NH/NL strengths: buy/sell on the close, stop and reverse.
/// `ratio` = the day's (up-down)/(up+down) breadth ratio (NaN when missing); thresholds in the order
/// [buy rs, sell rs, rs days, NL buy, NL sell, NL days, NH buy, NH sell, NH days].
pub fn rs_system(x: &Ctx, ratio: &[f64], nh: &[f64], nl: &[f64], dj: &[f64], t: [f64; 9], from: usize, to: usize, why: &str) -> Vec<Trade> {
    let u = |v: f64| v.round().max(1.0) as usize;
    let (rd, ld, hd) = (u(t[2]), u(t[5]), u(t[8]));
    let rx = ema0(&ratio[..=to], 2.0 / 11.0);
    let (nlx, nhx) = (ema0(&nl[..=to], 0.5), ema0(&nh[..=to], 0.5));
    let mut tgt = vec![0i8; to + 1];
    let mut pos = 0i8;
    for i in 0..=to {
        let rs = strength(&rx, i, rd) - strength(dj, i, rd);
        let (nls, nhs) = (strength(&nlx, i, ld), strength(&nhx, i, hd));
        let buy = rs >= t[0] && nls <= t[3] && nhs >= t[6];
        let sell = rs <= t[1] && nls >= t[4] && nhs <= t[7];
        if buy != sell { pos = if buy { 1 } else { -1 }; } // both at once: the orders conflict, the position is kept
        tgt[i] = pos;
    }
    target_trades(x, &tgt, from, to, false, why)
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from > to || to >= x.c.len() { return None; }
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap().def);
    let s: Vec<&[f64]> = match SERIES.iter().map(|n| x.series(n)).collect::<Option<Vec<_>>>() { Some(v) => v, None => return Some(vec![]) };
    let adr: Vec<f64> = (0..x.c.len()).map(|i| (s[0][i] - s[1][i]) / (s[0][i] + s[1][i])).collect();
    let t = [g("brs"), g("srs"), g("xdays"), g("nlbx"), g("nlsx"), g("ldays"), g("nhbx"), g("nhsx"), g("hdays")];
    Some(rs_system(x, &adr, s[2], s[3], s[4], t, from, to, "ADRrs-NH-NL signal"))
}

pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "dmadr" && reviewed(def.id),
    label: "Dennis Meyers tested the daily ADRrs-NH-NL system on the S&P 500's closes (1978-1996); it reads NYSE breadth and the DJIA close and runs on the US indices and ETFs SPX, SPY, DJI, DIA, NDX and QQQ only (R4 #306), with two unstated details read as labelled on the card.",
    basis: "Dennis Meyers, A Daily A-D New High-New Low Market System, Stocks & Commodities V15:1 p.11-19 (VOLUMES/V15/C01/ADAILYA.pdf); NYSE new highs/lows and the DJIA from Norgate (SOURCES norgate-us-indices-20200908-pb/-pf); independent check authors/pf/w/pf_ref.py",
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn strength_and_reversal() {
        assert_eq!(strength(&[1., 3., 2.], 2, 3), 0.0);
        assert!(strength(&[2., 2.], 1, 2).is_nan());
        // Unit-test fixture only (no market data). Bars 0-29: breadth ratio and new highs rise, new lows and the Dow fall -> every
        // strength is at its extreme from bar 10 (10-bar windows; the 0-started new-low average is 50 on bar 0, its window low, until bar 0
        // leaves the window): ADRrs = +50 - (-50) = 100, NLstr -50, NHstr +50 -> buy at bar 10.
        // Bars 30-59: the reverse; once the lagging ratio average and the Dow reach the other extremes, ADRrs = -100 -> sell (reverse).
        let n = 60;
        let r: Vec<Row> = (0..n).map(|i| Row { ticker: "T".into(), date: format!("2000-{:02}-{:02}", 1 + i / 28, 1 + i % 28), open: 100., high: 100., low: 100., close: 100., volume: None }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let up = |i: usize| i < 30;
        let ratio: Vec<f64> = (0..n).map(|i| if up(i) { i as f64 } else { 28. - 2. * (i - 30) as f64 }).collect();
        let dj: Vec<f64> = (0..n).map(|i| if up(i) { 1000. - i as f64 } else { 970. + (i - 30) as f64 }).collect();
        let nl: Vec<f64> = (0..n).map(|i| if up(i) { 100. - 3. * i as f64 } else { 13. + 9. * (i - 30) as f64 }).collect();
        let nh: Vec<f64> = (0..n).map(|i| if up(i) { 5. + 3. * i as f64 } else { 95. - 3. * (i - 30) as f64 }).collect();
        let t = rs_system(&x, &ratio, &nh, &nl, &dj, [4., -40., 10., -10., 30., 10., -10., 0., 10.], 0, n - 1, "t");
        assert_eq!(t.iter().map(|t| (t.dir, t.open)).collect::<Vec<_>>(), vec![(1.0, false), (-1.0, true)]);
        assert_eq!(t[0].legs[0].i, 10);
        assert!((33..=50).contains(&t[1].legs[0].i), "{}", t[1].legs[0].i);
    }
}
