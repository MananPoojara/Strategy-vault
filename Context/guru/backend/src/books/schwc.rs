//! Ichu Cheng, "Holt-Winter Channel: taking the formula one step beyond", Technical Analysis of Stocks & Commodities V.6:8 (1988)
//! pp.285-290, with the sidebar "Holt-Winter Channel Calculation" and the Figure 5 work sheet (research share Trading Knowledge I/
//! Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V06/C08/HOLTWIN.pdf). Book id `schwc`: the three trading rules.
//! Lane K41 (batch T7a, 4 Nov 2026); independent check authors/titles-1030/K41w/k41_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::sctraub::next_open;

pub const REVIEWED: &[&str] = &["schwc_aggressive", "schwc_moderate", "schwc_conservative"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] Recursion on daily closes from the first bar of the data (t = 0: F = C, V = A = 0): FC(t) = F(t-1) + V(t-1) + 0.5 A(t-1); F(t) = (1-a) FC(t) + a C(t); V(t) = (1-b)(V(t-1) + A(t-1)) + b (F(t) - F(t-1)); A(t) = (1-c) A(t-1) + c (V(t) - V(t-1)); Var(t) = (1-d) Var(t-1) + d (C(t) - FC(t))^2, with Var(1) = (C(1) - FC(1))^2; UB/LB(t) = FC(t) +/- T sqrt(Var(t)); a = b = c = d = 0.5, T = 1. The indexing and the first variance follow the Figure 5 work sheet (DJIA, January 1988: e.g. 6-Jan-88 F 2033.13, V 7.92, A 2.95, Var 175.71, FC 2028.45, UB 2041.71, LB 2015.20), which the Rust test reproduces. Regions from the close: above UB improving, below LB deteriorating, else neutral (from t = 2). Rule 1 (aggressive): buy when the region rises (deteriorating to neutral or neutral to improving), sell when it falls; Rule 2 (moderate): buy on a move into improving from below, sell on a move into deteriorating from above; Rule 3 (conservative): Rule 2 with buys only while the trend is up [reading: V > 0, the rate of change of the trend]. A move across the whole channel in one day counts as passing the neutral region. Long only ('No short selling is allowed'). The article does not give the fill: orders fill at the next open. Article (19 stocks, 2 January - 31 December 1987): Rule 1 139 completed trades, 58 profitable, average gain 30% (Figure 2 lists each stock). Runs on indices, ETFs and stocks, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    const F: &str = "F|\"Ft+1 = (1 - a) (Ft + Vt + 0.5 At) + aCt+1; Vt+1 = (1 - b) (Vt + At) + b(Ft+1 - Ft); At+1 = (1 - c) At + c (Vt+1 - Vt) ... FCt+1 = Ft+Vt+0.5At; Var t+1 = (1 - d) Var t + d (Ct - FCt)2; UBt+1 = FCt+1 + T Stdt; LBt+1 = FCt+1 - T Stdt ... C0 = 2015.25, F0 = 2015.25, A0 = 0.0, V0 = 0.0, Var0 = 0.0, T = 1.0, a = 0.5, b = 0.5, c = 0.5, d = 0.5\" (V.6:8, PDF p.10 sidebar; Figure 5 work sheet PDF p.8)";
    const G: &str = "F|\"The region below the lower boundary is the 'deteriorating' region ... The region within the channel is the 'neutral' region ... The region above the upper boundary is the 'improving' region.\" (PDF p.4)";
    const X: &str = "X|Long only: the sell move closes the long; fills at the next open [book silent on fill]";
    let d = |id, name, rules: &'static [&'static str]| StrategyDef { id, name, group: "S&C: Holt-Winter channel (Cheng, V.6:8)", session: 0, dir: "Long", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "schwc", rules, notes: NOTE, claim: None };
    vec![
        d("schwc_aggressive", "Holt-Winter channel, rule 1: buy on each move up a region, sell on each move down, long only",
          &[F, G, X, "E|\"Rule 1 (Aggressive Trading Rule): Buy when the price line moves from the deteriorating region to the neutral region or from neutral to improving; sell when the price line moves in the opposite direction. No short selling is allowed.\" (PDF p.5)"]),
        d("schwc_moderate", "Holt-Winter channel, rule 2: buy on a move into the improving region, sell on a move into the deteriorating region",
          &[F, G, X, "E|\"Rule 2 (Moderate Trading Rule): Buy when the price line moves from the neutral region to the improving region; sell when the price line moves from the neutral to the deteriorating region.\" (PDF p.6)"]),
        d("schwc_conservative", "Holt-Winter channel, rule 3: rule 2 with buys only in an uptrend",
          &[F, G, X, "E|\"Rule 3 (Conservative Trading Rule): Buy when the price line moves from the neutral region to the improving region and the price line is on the uptrend; sell when the price line moves from the neutral to the deteriorating region.\" (PDF p.6)"]),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let v = match id { "schwc_aggressive" => "Rule 1 aggressive", "schwc_moderate" => "Rule 2 moderate", "schwc_conservative" => "Rule 3 conservative", _ => return None };
    Some(("schwc_channel", "Holt-Winter channel (Cheng, S&C)", v))
}
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    reviewed(id).then(|| vec![
        ParamDef { key: "a", label: "Smoothing a = b = c = d (0.5 as printed)", def: 0.5, min: 0.05, max: 0.95, step: 0.05 },
        ParamDef { key: "t", label: "Comfort factor T (1 as printed)", def: 1., min: 0.25, max: 3., step: 0.25 }])
}

/// (F, V, A, Var, FC, UB, LB) per bar; FC/UB/LB from bar 1, NaN before.
pub fn channel(c: &[f64], s: f64, tt: f64) -> Vec<[f64; 7]> {
    let n = c.len();
    let mut out = vec![[f64::NAN; 7]; n];
    if n == 0 { return out; }
    out[0] = [c[0], 0., 0., 0., f64::NAN, f64::NAN, f64::NAN];
    for t in 1..n {
        let [f, v, a, var, ..] = out[t - 1];
        let fc = f + v + 0.5 * a;
        let f1 = (1. - s) * fc + s * c[t];
        let v1 = (1. - s) * (v + a) + s * (f1 - f);
        let a1 = (1. - s) * a + s * (v1 - v);
        let e2 = (c[t] - fc).powi(2);
        let var1 = if t == 1 { e2 } else { (1. - s) * var + s * e2 };
        let sd = var1.sqrt();
        out[t] = [f1, v1, a1, var1, fc, fc + tt * sd, fc - tt * sd];
    }
    out
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let (s, tt) = (p.get("a").copied().unwrap_or(0.5), p.get("t").copied().unwrap_or(1.));
    let ch = channel(&x.c, s, tt);
    let reg = |t: usize| if x.c[t] > ch[t][5] { 2 } else if x.c[t] < ch[t][6] { 0 } else { 1 };
    Some(next_open(x, from, to, "Region change down", |t, side| {
        if t < 3 { return side; }
        let (r0, r1) = (reg(t - 1), reg(t));
        let (buy, sell) = match id {
            "schwc_aggressive" => (r1 > r0, r1 < r0),
            _ => (r1 == 2 && r0 < 2 && (id != "schwc_conservative" || ch[t][1] > 0.0), r1 == 0 && r0 > 0),
        };
        if buy { 1.0 } else if sell { 0.0 } else { side }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn figure_5_work_sheet() {
        // DJIA closes 4-29 January 1988 (Figure 5, PDF p.8)
        let c = [2015.25, 2031.50, 2037.80, 2051.89, 1911.31, 1945.13, 1928.55, 1924.73, 1916.11, 1956.07, 1963.86, 1936.34, 1879.14, 1879.31, 1903.51, 1946.45, 1920.59, 1911.14, 1930.04, 1958.22];
        let ch = channel(&c, 0.5, 1.0);
        let near = |a: f64, b: f64| (a - b).abs() < 0.011;
        assert!(near(ch[1][0], 2023.38) && near(ch[1][3], 264.06) && near(ch[1][4], 2015.25));
        assert!(near(ch[2][0], 2033.13) && near(ch[2][1], 7.92) && near(ch[2][2], 2.95) && near(ch[2][3], 175.71) && near(ch[2][5], 2041.71) && near(ch[2][6], 2015.20));
        assert!(near(ch[4][3], 11352.53) && near(ch[4][5], 2168.10) && near(ch[4][6], 1955.01));
        assert!(near(ch[19][0], 1942.89) && near(ch[19][1], 15.20) && near(ch[19][4], 1927.55) && near(ch[19][5], 1955.76) && near(ch[19][6], 1899.34));
        let r = super::super::sctraub::tests::bars(1500);
        let x = Ctx::new(&r, &r, &[]);
        for id in REVIEWED { assert!(run(id, &x, 0, r.len() - 1, &Params::new()).unwrap().iter().all(|z| z.dir > 0.0), "{id}"); }
    }
}
