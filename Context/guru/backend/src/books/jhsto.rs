//! Jack Hershey's Stochastics Strategy (EasyLanguage text shipped in the NinjaTrader 6.5 "Jack Hershey Indicators 1.021"
//! folder). Source (research drive, read in place, read-only): `USB HDD 1/NINJA TARDER 6.5/NinjaTrader 6.5/Jack Hershey
//! Indicators 1.021/Jack Hershey's Stochastics Strategy.rar` -> `Jack Hershey_s Stochastics Strategy.txt` (data only).
//! Book id `jhsto`. Notes: library/jhsto/README.md. Engine: books::rrsim.
use crate::books::rrsim::{g, lowest, highest, o, pd, run_ts, sdef, sma, A, P};
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "jhsto";
const ID: &str = "jhsto_stoch_basics";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![sdef(BOOK, ID, "Jack Hershey stochastics basics (FastD and SlowD both beyond 75 / 25)", "Hershey Stochastics Strategy (.txt)", "Long/Short", Status::Live, &[
        "F|\"Stochastic(High, Low, Close, StochLength 14, SmoothingLength1 1, SmoothingLength2 3, SmoothingType 1 {Original}, oFastK, oFastD, oSlowK, oSlowD)\"; OverSold 25, OverBought 75",
        "E|\"If MarketPosition = 0 Then ... If oFastD > OverBought and oSlowD > Overbought then buy next bar at open\"",
        "E|\"If MarketPosition = 0 Then ... If oFastD < OverSold and oSlowD < OverSold then sellshort next bar at open\"",
        "X|\"If MarketPosition = 1 Then If oFastD < OverBought and oSlowD < Overbought then sell next bar at open\"",
        "X|\"If MarketPosition = -1 Then If oFastD > OverSold and oSlowD > OverSold then buytocover next bar at open\"",
        "X|\"ImmediateProfitExit(0.00)\": the profit-target limit exit is off by default",
    ], "Code wins over its header comment (the comment says 'crosses above/below'; the code tests levels). Timeframe not stated; run on daily bars. \
[Assumption, TradeStation function] Stochastic, SmoothingType 1 (Original): FastK = 100 (C - Lowest(L,14)) / (Highest(H,14) - Lowest(L,14)); FastD with SmoothingLength1 = 1 equals FastK; SlowD = 3-bar simple average of SlowK (= FastD). \
p.legacy = 1 uses TradeStation's legacy smoothing (SlowD = (2 x SlowD[1] + SlowK)/3) instead. A zero 14-bar range leaves the bar undefined (no signal). \
Fills: next open. India: instrument swap; index shorts hypothetical.")]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == ID).then_some(("jhsto", "Jack Hershey stochastics", "code literal"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    (id == ID).then(|| vec![pd("len", "StochLength", 14., 5., 40., 1.), pd("ob", "OverBought", 75., 50., 95., 1.), pd("os", "OverSold", 25., 5., 50., 1.),
        pd("legacy", "SlowD smoothing: 0 = Original (3-bar average), 1 = legacy (2 prev + new)/3", 0., 0., 1., 1.)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if id != ID || from >= to || to >= x.r.len() { return None; }
    let n = g(p, "len", 14.).max(1.) as usize;
    let (ob, os) = (g(p, "ob", 75.), g(p, "os", 25.));
    let fk: Vec<f64> = (0..x.c.len()).map(|i| {
        let (hh, ll) = (highest(&x.h, n, i), lowest(&x.l, n, i));
        if hh > ll { 100. * (x.c[i] - ll) / (hh - ll) } else { f64::NAN }
    }).collect();
    let sd: Vec<f64> = if g(p, "legacy", 0.) >= 0.5 {
        let mut v = vec![f64::NAN; fk.len()];
        for i in 0..fk.len() { v[i] = if i > 0 && v[i - 1].is_finite() { (2. * v[i - 1] + fk[i]) / 3. } else { fk[i] }; }
        v
    } else { sma(&fk, 3) };
    Some(run_ts(x, from, to, |i, st, _now, next| {
        let (f, s) = (fk[i], sd[i]);
        if !f.is_finite() || !s.is_finite() { return; }
        match st.pos {
            0 => {
                if f > ob && s > ob { next.push(o(A::Buy, P::Mkt, "LE")); }
                if f < os && s < os { next.push(o(A::Sell, P::Mkt, "SE")); }
            }
            1 => if f < ob && s < ob { next.push(o(A::Xl, P::Mkt, "LX")); },
            _ => if f > os && s > os { next.push(o(A::Xs, P::Mkt, "SX")); },
        }
    }))
}
