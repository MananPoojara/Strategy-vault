//! Markos Katsanos, "Detecting Breakouts" (Finite Volume Elements), Technical Analysis of Stocks & Commodities,
//! April 2003, pp.27-38. Source + checks: library/mkfve/.
use crate::connors::{sma, Claim, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "mkfve";

const RULES: &[&str] = &[
    "F|MF:=C-(H+L)/2+Typical()-Ref(Typical(),-1); FVE:=Sum(If(MF>0.3*C/100,+V,If(MF<-0.3*C/100,-V,0)),PERIOD)/Mov(V,PERIOD,S)/PERIOD*100; PERIOD 22 (MetaStock formula, p.29)",
    "E|Buy: FVE crosses the -5 line upward, and the 35-day linear regression slopes of FVE and price are positive and negative respectively (p.36)",
    "X|Sell: the 25-day FVE linear regression slopes down, or at the latest 50 days from the buy signal (p.36; Figure 8 'Inactivity stop')",
    "F|[B] Fills at the signal bar's close (MetaStock System Tester default; the article gives no order type)",
];

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef {
        id: "mkfve_div", name: "FVE divergence with LRS (Finite Volume Elements)", group: "S&C Apr 2003 Detecting Breakouts",
        session: 0, dir: "Long", status: Status::Live, needs_volume: true, needs_vix: false, needs_series: &[], book: BOOK,
        rules: RULES,
        notes: "Book test: XM Satellite Radio (XMSR) stock, 12/28/99-2/15/02, $1,000, 4 trades all winners (Figures 7-8, pp.36-38); a single stock we do not have, so no reproduction. Run on the vault's indices/ETFs with volume (instrument swap). Cross(FVE,-5): FVE > -5 today and <= -5 yesterday. Linear regression slope = least-squares slope over the last 35 / 25 bars. [B] The time exit is the close 50 bars after the entry bar (Figure 8: entry 10/16/01 -> inactivity stop 12/27/01 = 50 trading days later); the slope exit is checked from the bar after entry. The article's 'Traders' Tips' code (p.80) is not on the drive.",
        claim: Some(Claim { trades: "4", win: "100%", avg: "$1,098.79 / trade ($1,000 start)", hold: "35.5 bars", period: "12/28/99-2/15/02, XMSR" }),
    }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == "mkfve_div").then_some(("mkfve", "Katsanos Finite Volume Elements", "Divergence with LRS"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    (id == "mkfve_div").then(|| vec![
        p("period", "FVE period", 22., 10., 80., 1.),
        p("cutoff", "Money-flow cutoff % of close", 0.3, 0.1, 1.0, 0.05),
        p("level", "FVE cross level", -5., -30., 10., 5.),
        p("lrs_in", "Entry regression bars", 35., 15., 60., 5.),
        p("lrs_out", "Exit regression bars", 25., 10., 50., 5.),
        p("hold", "Time exit bars (0 = off)", 50., 0., 100., 5.),
    ])
}

/// FVE per the article's MetaStock formula; NaN where volume or history is missing.
pub fn fve(x: &Ctx, period: usize, cutoff: f64) -> Vec<f64> {
    let n = x.c.len();
    let v: Vec<f64> = x.r.iter().map(|r| r.volume.filter(|v| *v > 0.0).unwrap_or(f64::NAN)).collect();
    let typ: Vec<f64> = (0..n).map(|i| (x.h[i] + x.l[i] + x.c[i]) / 3.0).collect();
    let term: Vec<f64> = (0..n).map(|i| {
        if i == 0 { return f64::NAN; }
        let mf = x.c[i] - (x.h[i] + x.l[i]) / 2.0 + typ[i] - typ[i - 1];
        let cut = cutoff * x.c[i] / 100.0;
        if mf > cut { v[i] } else if mf < -cut { -v[i] } else if v[i].is_finite() { 0.0 } else { f64::NAN }
    }).collect();
    let s = sma(&term, period); // Sum/period
    let mv = sma(&v, period);
    (0..n).map(|i| s[i] / mv[i] * 100.0).collect()
}

/// LinRegSlope(x,n): least-squares slope of the last n values.
pub fn lrs(v: &[f64], n: usize) -> Vec<f64> {
    let xm = (n as f64 - 1.0) / 2.0;
    let sxx: f64 = (0..n).map(|k| (k as f64 - xm).powi(2)).sum();
    (0..v.len()).map(|i| {
        if i + 1 < n { return f64::NAN; }
        let w = &v[i + 1 - n..=i];
        let ym = w.iter().sum::<f64>() / n as f64;
        w.iter().enumerate().map(|(k, y)| (k as f64 - xm) * (y - ym)).sum::<f64>() / sxx
    }).collect()
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if from > to || to >= x.c.len() { return None; }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let u = |k: &str| g(k).max(1.0) as usize;
    let f = fve(x, u("period"), g("cutoff"));
    let (fin, cin, fout) = (lrs(&f, u("lrs_in").max(2)), lrs(&x.c, u("lrs_in").max(2)), lrs(&f, u("lrs_out").max(2)));
    let (lv, hold) = (g("level"), g("hold").max(0.0) as usize);
    let (mut out, mut cur): (Vec<Trade>, Option<Trade>) = (vec![], None);
    for t in from..=to {
        if let Some(tr) = cur.as_mut() {
            let e = tr.legs[0].i;
            let why = if hold > 0 && t >= e + hold { Some("50 bars from the buy (inactivity stop)") }
                      else if t > e && fout[t] < 0.0 { Some("25-day FVE regression slope down") } else { None };
            if let Some(w) = why {
                tr.exit_i = t; tr.exit_px = x.c[t]; tr.signal_i = t; tr.reason = w.into();
                out.push(cur.take().unwrap());
            }
            continue;
        }
        if t > 0 && t < to && f[t] > lv && f[t - 1] <= lv && fin[t] > 0.0 && cin[t] < 0.0 {
            cur = Some(Trade { dir: 1.0, legs: vec![Leg { i: t, px: x.c[t], w: 1.0 }], exit_i: t, exit_px: x.c[t], reason: String::new(),
                               entry_kind: "close", signal_i: t, open: false, exits: vec![] });
        }
    }
    if let Some(mut t) = cur { t.exit_i = to; t.exit_px = x.c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true; out.push(t); }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn fve_matches_article_spreadsheet_row() {
        // Sidebar Figure 1 (p.28), MANU 11/3/00: H 62.56, L 59.06, C 60.25, prev typical 60.107 -> MF -0.04333, |MF| < 0.3%*C -> 0
        let r = vec![
            Row { ticker: "T".into(), date: "2000-11-02".into(), open: 57.56, high: 63.13, low: 55.38, close: 61.81, volume: Some(23376.) },
            Row { ticker: "T".into(), date: "2000-11-03".into(), open: 61.53, high: 62.56, low: 59.06, close: 60.25, volume: Some(17538.) },
        ];
        let x = Ctx::new(&r, &r, &[]);
        let typ0: f64 = (63.13 + 55.38 + 61.81) / 3.0;
        let mf = 60.25 - (62.56 + 59.06) / 2.0 + (62.56 + 59.06 + 60.25) / 3.0 - typ0;
        assert!((mf - (-0.04333)).abs() < 1e-4);
        assert!((fve(&x, 1, 0.3)[1]).abs() < 1e-12); // inside the cutoff -> 0 volume
    }
    #[test]
    fn slope_by_hand() { assert!((lrs(&[2., 4., 6., 8.], 4)[3] - 2.0).abs() < 1e-12); }
}
