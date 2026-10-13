//! Dennis Meyers, "The Turbo A/D, NH, NL Market System", Technical Analysis of Stocks & Commodities V15:7 (1997) p.337-346.
//! Source (read-only): research `USB HDD 2/tRADING KNO-2/_Abstracts and Misc. Articles/Articulos Varios Español/Articulos
//! Varios- Club Directores/AAApublicaciones/Traducidos/theturboAD/THETURB.PDF` (the English S&C reprint, 10 pages).
//! Defaults = the article's "final optimum values" (p.341). Stop-and-reverse on the S&P 500 close.
use crate::connors::{claim, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

const BOOK: &str = "dmtad";
const SERIES: &[&str] = &["NYSE_ADV_DEC.Advances", "NYSE_ADV_DEC.Declines", "NYSE_ADV_DEC.AdvVolume", "NYSE_ADV_DEC.DecVolume", "NYSE_NEW_HIGHS", "NYSE_NEW_LOWS"];

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef {
        id: "dmtad_turbo", name: "Turbo A/D, NH, NL market system", group: "S&C V15:7 · Turbo A/D", session: 0, dir: "Long/Short",
        status: Status::Live, needs_volume: false, needs_vix: false, needs_series: SERIES, book: BOOK,
        rules: &[
            "F|adr = ai/di; advr = av/dv; dar = di/ai; davr = dv/av (NYSE advancing/declining issues and volume); nlx = nlx[1] + 0.5*(nl - nlx[1]), nhx likewise (3-day EMA of NYSE 52-week new lows / highs) (p.339)",
            "F|Strength(x,d) = 100*{(x - lowest(x,d))/range(x,d)} - 50; NLstr = Strength(nlx,Ldays); NHstr = Strength(nhx,Hdays) (p.339)",
            "E|Buy the S&P 500 on the close if NLstr <= NLbx and NHstr >= NHbx and spx is greater than the lowest spx price found while in last sell by at least pctud percent and [adr >= buyr for xbuy days out of nbuy days and adr >= 1 for all of the nbuy days, or advr >= buyvolr] (p.340)",
            "E|Sell the S&P 500 on the close if NLstr >= NLsx and NHstr <= NHsx and spx is less than the highest spx price found while in last buy by at least pctud percent and [NLstr - NLstr[1] < 30 and dar >= sellr for xsell days out of nsell days, or (davr >= sellvolr and davr[1] >= sellvolr2) or (davr[1] >= sellvolr and davr >= sellvolr2)] (p.340)",
            "X|Stop and reverse: a buy covers the short, a sell closes the long (the article's long and short trade summaries, Figs 1-4)",
            "F|Final optimum values (p.341): buyr 1.2, xbuy 2, nbuy 4; buyvolr 7.5; sellr 1.9, xsell 3, nsell 4; sellvolr 10, sellvolr2 2.2; NLbx 0, NLsx 30, Ldays 50; NHbx -10, NHsx 0, Hdays 10; pctud 1.25",
        ],
        notes: "Series: NYSE_ADV_DEC.* = daily NYSE advancing/declining issues and volume (in the vault, 1965-2020); NYSE_NEW_HIGHS / NYSE_NEW_LOWS = daily NYSE 52-week new highs / new lows counts (NOT in the vault yet: the API reports the data gap). Readings: 'lowest spx price found while in last sell' = lowest close from the sell bar to today; before the first opposite trade there is no such price and the confirmation is skipped (the article starts mid-history); a zero range makes Strength undefined, so no signal that day. The second sell rule's 'davr[1] >= sellvol' is read as sellvolr (typo in the article). India: no NSE 52-week new highs/lows series exists in the vault.",
        claim: claim("17", "76%", "$30.95 (S&P 500 points)", "165 bars (winners)", "S&P 500 daily 01/02/87-12/29/95 (test segment); 29 trades 66% 01/03/78-02/27/87"),
    }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == "dmtad_turbo").then_some(("dmtad_turbo", "Meyers Turbo A/D, NH, NL", "Final optimum values"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    if id != "dmtad_turbo" { return None; }
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(vec![
        p("buyr", "buyr: adr threshold", 1.2, 1.0, 3.0, 0.1), p("xbuy", "xbuy days", 2.0, 1.0, 4.0, 1.0), p("nbuy", "out of nbuy days", 4.0, 2.0, 5.0, 1.0),
        p("buyvolr", "buyvolr: advr threshold", 7.5, 4.0, 10.0, 0.5),
        p("sellr", "sellr: dar threshold", 1.9, 1.2, 3.0, 0.1), p("xsell", "xsell days", 3.0, 1.0, 5.0, 1.0), p("nsell", "out of nsell days", 4.0, 3.0, 5.0, 1.0),
        p("sellvolr", "sellvolr: davr spike", 10.0, 6.0, 14.0, 0.5), p("sellvolr2", "sellvolr2: davr follow-up", 2.2, 1.0, 4.0, 0.1),
        p("nlbx", "NLbx", 0.0, -50.0, 50.0, 5.0), p("nlsx", "NLsx", 30.0, -50.0, 50.0, 5.0), p("ldays", "Ldays", 50.0, 10.0, 100.0, 5.0),
        p("nhbx", "NHbx", -10.0, -50.0, 50.0, 5.0), p("nhsx", "NHsx", 0.0, -50.0, 50.0, 5.0), p("hdays", "Hdays", 10.0, 5.0, 50.0, 5.0),
        p("pctud", "pctud: % confirmation from the last extreme", 1.25, 0.0, 5.0, 0.25),
    ])
}

fn ema_half(v: &[f64]) -> Vec<f64> {
    let mut x = 0.0; // TradeStation var starts at 0
    v.iter().map(|&a| { if a.is_finite() { x += 0.5 * (a - x); } else { x = f64::NAN; } x }).collect()
}
fn strength(x: &[f64], i: usize, d: usize) -> f64 {
    if d == 0 || i + 1 < d { return f64::NAN; }
    let w = &x[i + 1 - d..=i];
    let (lo, hi) = w.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |a, &v| (a.0.min(v), a.1.max(v)));
    if !(hi - lo > 0.0) { return f64::NAN; }
    100.0 * (x[i] - lo) / (hi - lo) - 50.0
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if from > to || to >= x.c.len() { return Some(vec![]); }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let u = |k: &str| g(k).round().max(1.0) as usize;
    let s: Vec<&[f64]> = match SERIES.iter().map(|n| x.series(n)).collect::<Option<Vec<_>>>() { Some(v) => v, None => return Some(vec![]) };
    let (ai, di, av, dv) = (s[0], s[1], s[2], s[3]);
    let (nlx, nhx) = (ema_half(&s[5][..=to]), ema_half(&s[4][..=to]));
    let c = &x.c;
    let (nbuy, xbuy, nsell, xsell) = (u("nbuy"), u("xbuy"), u("nsell"), u("xsell"));
    let pct = g("pctud") / 100.0;
    let mut out = Vec::new();
    let mut pos: Option<(f64, usize)> = None; // (dir, entry bar)
    let mut ext = f64::NAN; // lowest close while short / highest close while long
    for i in 1..=to {
        if let Some((d, _)) = pos { ext = if d > 0.0 { ext.max(c[i]) } else { ext.min(c[i]) }; }
        let (nl, nl1, nh) = (strength(&nlx, i, u("ldays")), strength(&nlx, i - 1, u("ldays")), strength(&nhx, i, u("hdays")));
        let ratio = |a: &[f64], b: &[f64], k: usize| a[k] / b[k];
        let count = |n: usize, f: &dyn Fn(usize) -> bool| if i + 1 < n { None } else { Some((i + 1 - n..=i).filter(|&k| f(k)).count()) };
        let short = matches!(pos, Some((d, _)) if d < 0.0);
        let long = matches!(pos, Some((d, _)) if d > 0.0);
        let buy = !long && nl <= g("nlbx") && nh >= g("nhbx") && (!short || c[i] >= ext * (1.0 + pct)) && {
            let adr = |k: usize| ratio(ai, di, k);
            let a = count(nbuy, &|k| adr(k) >= g("buyr")).is_some_and(|n| n >= xbuy) && count(nbuy, &|k| adr(k) >= 1.0) == Some(nbuy);
            a || ratio(av, dv, i) >= g("buyvolr")
        };
        let sell = !buy && !short && nl >= g("nlsx") && nh <= g("nhsx") && (!long || c[i] <= ext * (1.0 - pct)) && {
            let dar = |k: usize| ratio(di, ai, k);
            let davr = |k: usize| ratio(dv, av, k);
            let a = nl - nl1 < 30.0 && count(nsell, &|k| dar(k) >= g("sellr")).is_some_and(|n| n >= xsell);
            let (v0, v1) = (davr(i), davr(i - 1));
            a || (v0 >= g("sellvolr") && v1 >= g("sellvolr2")) || (v1 >= g("sellvolr") && v0 >= g("sellvolr2"))
        };
        if !(buy || sell) { continue; }
        let d = if buy { 1.0 } else { -1.0 };
        if let Some((pd, e)) = pos.take() {
            out.push(Trade { dir: pd, legs: vec![Leg { i: e, px: c[e], w: 1.0 }], exit_i: i, exit_px: c[i], reason: "Reversed".into(),
                entry_kind: "close", signal_i: i, open: false, exits: Vec::new() });
        }
        pos = Some((d, i));
        ext = c[i];
    }
    if let Some((d, e)) = pos {
        out.push(Trade { dir: d, legs: vec![Leg { i: e, px: c[e], w: 1.0 }], exit_i: to, exit_px: c[to], reason: "open".into(),
            entry_kind: "close", signal_i: to, open: true, exits: Vec::new() });
    }
    Some(out.into_iter().filter(|t| t.legs[0].i >= from).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    /// Unit-test fixture only (no market data): breadth that turns strongly positive, then strongly negative.
    #[test]
    fn buys_on_breadth_then_reverses_on_volume_spike() {
        let n = 80;
        let r: Vec<Row> = (0..n).map(|i| { let c = 100.0 + if i < 70 { i as f64 * 0.1 } else { 7.0 - (i - 70) as f64 }; Row { ticker: "T".into(),
            date: format!("2000-{:02}-{:02}", 1 + i / 28, 1 + i % 28), open: c, high: c, low: c, close: c, volume: None } }).collect();
        let mut x = Ctx::new(&r, &r, &[]);
        // new lows fall to 0 by bar 60 (NLstr = -50 <= 0), new highs rise (NHstr >= -10); adr 2.0 from bar 60 -> buy at bar 63 (4 days of adr >= 1.2)
        let nl: Vec<f64> = (0..n).map(|i| if i < 60 { 50.0 - i as f64 * 0.8 } else if i < 72 { 2.0 } else { 200.0 }).collect();
        let nh: Vec<f64> = (0..n).map(|i| if i < 72 { 10.0 + i as f64 } else { 0.0 }).collect();
        let ai: Vec<f64> = (0..n).map(|i| if (60..72).contains(&i) { 1000.0 } else if i >= 72 { 200.0 } else { 600.0 }).collect();
        let di: Vec<f64> = (0..n).map(|i| if (60..72).contains(&i) { 500.0 } else if i >= 72 { 1000.0 } else { 900.0 }).collect();
        let av = vec![100.0; n];
        let dv: Vec<f64> = (0..n).map(|i| if i >= 74 { 1200.0 } else { 100.0 }).collect();
        for (k, v) in SERIES.iter().zip([ai, di, av, dv, nh, nl]) { x.extra.insert((*k).into(), v); }
        let t = run("dmtad_turbo", &x, 0, n - 1, &Params::new()).unwrap();
        let got: Vec<_> = t.iter().map(|t| (t.dir, t.legs[0].i, t.exit_i, t.open)).collect();
        // sell at 74: NLstr = +50 (jump from -50 on bar 72 blocks rule 1 there; < 30 later), NHstr falling, close 103 <= 107*(1-.0125),
        // dar = 5 on 3 of the last 4 days (bars 72-74) only from bar 74
        assert_eq!(got, vec![(1.0, 63, 74, false), (-1.0, 74, 79, true)]);
    }
}
