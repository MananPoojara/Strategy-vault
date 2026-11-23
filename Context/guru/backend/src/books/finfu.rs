//! SP_U (Financial Facilitators Inc., finf.com), TradeStation: research share `USB HDD 1/TradeStation Systems & Indicators/SP_U - Financial Facilitators inc. (finf.com)/
//! SP_U.rar::SP_U.txt` (read-only copy read in full). Book id `finfu`. Lane K4 (23 Nov 2026, code pass, user ruling "code has the logic"):
//! the S&P system run on daily indices as a labelled R4 #340 adaptation. Independent check authors/code-1123/ref/finfu_ref.py.
use super::rrsim::{g, o, pd, run_ts, sdef, sma, A, P};
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::ajmisc::avg_true_range;

const ID: &str = "finfu_spu";
const NOTE: &str = "[Declared adaptation, R4 #340 and the user ruling of 23 Nov 2026] The code is 'SP_U' (an S&P futures system; the file names no bar size and prints no result). The vault runs it literally on daily index bars, UNVERIFIED. Rule as printed, each close: iMACD = MACD(Close,7,10) (XAverage difference), iAvg = XAverage(iMACD,4), iDiff = iMACD - iAvg. Not long, iDiff crosses over 0 and Close > Average(Close,5): Buy next bar at market; not short, iDiff crosses under 0 and Close < Average(Close,5): Sell Short next bar at market (UseLimitOrders false). Exit: SetStopLoss(StopAmount 1 x BigPointValue) = a 1-point stop from the entry price; a stop-and-reverse style trailing stop: on the first bar in the position iStopPrice = Low - ATR(5) x 2.25 (High + ATR x 2.25 for a short) with iAF = 0.0025 and iTradeHH = High; later bars: iTradeHH = max(iTradeHH, High); iStopPrice = iStopPrice + iAF x (iTradeHH - iStopPrice); iAF rises by min(0.0025, 0.6 - iAF) when iTradeHH made a new high; iStopPrice is never above Low (never below High for a short); Sell / Buy to Cover next bar at iStopPrice stop each bar. Once MaxPositionProfit >= GetFlatAmount (1 x BigPointValue = 1 point), the opposite MACD cross with the close not beyond its 5-bar average closes the trade at market (Buy to Cover when iDiff crosses over 0 and Close <= Average(Close,5); Sell when it crosses under 0 and Close >= Average(Close,5)). [Declared readings] (1) daily bars; (2) the code's one-point amounts (stop 1, get-flat 1) are S&P points around a level of 1,000, scaled by the entry price, amount x entry price / 1,000 (one labelled detail); (3) the built-in 1-point stop is placed from the entry fill as a stop order for the bars after the entry bar (the vault's bar model has no same-bar built-in stop); MaxPositionProfit = the best open profit on bar highs / lows from the entry bar to the signal bar. One position, fills as the TradeStation bar model.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![sdef("finfu", ID, "SP_U (Financial Facilitators): MACD(7,10) signal-line cross with a 5-bar average filter, ATR-start accelerating trailing stop, 1-point stop", "SP_U (SP_U.txt)", "Long/Short", Status::Live, &[
        "F|\"iMACD = MACD(CLOSE, FastLength, SlowLength); iAvg = XAverage(iMACD, MACDLength); iDiff = (iMACD - iAvg)\" (7, 10, 4)",
        "E|\"If iMP <> 1 and iDiff CROSS OVER 0 and CLOSE > average(CLOSE, BuyAvg) then Buy (\\\"B\\\") NEXT BAR MARKET\"; \"If iMP <> -1 and iDiff CROSS UNDER 0 and CLOSE < average(CLOSE, SellAvg) then Sell Short (\\\"S\\\") NEXT BAR MARKET\" (BuyAvg = SellAvg = 5)",
        "X|\"SETSTOPLOSS (StopAmount * BIGPOINTVALUE)\" (StopAmount 1 = one point); trailing: \"iStopPrice = LOW - AvgTrueRange(ATRLength) * NumATRs\" then \"iStopPrice = iStopPrice + iAF * (iTradeHH - iStopPrice)\", \"Sell (\\\"LX\\\") NEXT BAR iStopPrice STOP\" (afStep .0025, afLimit .6, ATR 5, 2.25)",
        "X|\"If MAXPOSITIONPROFIT >= (GetFlatAmount * BIGPOINTVALUE) then If iMP = 1 and iDiff CROSS UNDER 0 and CLOSE >= average(CLOSE, SellAvg) then Sell (\\\"LX Flat\\\") NEXT BAR MARKET\" (mirrored for a short; GetFlatAmount 1)",
        NOTE], NOTE)]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { (id == ID).then_some((ID, "SP_U (finf.com)", "MACD cross + ATR trail")) }
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    (id == ID).then(|| vec![pd("fast", "FastLength", 7., 2., 50., 1.), pd("slow", "SlowLength", 10., 3., 100., 1.), pd("sig", "MACDLength", 4., 2., 50., 1.), pd("avg", "BuyAvg = SellAvg", 5., 2., 50., 1.),
        pd("stop", "StopAmount (S&P points at 1,000, scaled by price)", 1., 0.25, 20., 0.25), pd("flat", "GetFlatAmount (S&P points at 1,000, scaled by price)", 1., 0.25, 20., 0.25),
        pd("afstep", "afStep", 0.0025, 0.0005, 0.05, 0.0005), pd("aflimit", "afLimit", 0.6, 0.1, 1., 0.05), pd("atr", "ATRLength", 5., 2., 30., 1.), pd("natr", "NumATRs", 2.25, 0.5, 6., 0.25),
        pd("ref", "reference level the code's points belong to (S&P about 1,000)", 1000., 100., 5000., 100.)])
}
fn ema(v: &[f64], n: usize) -> Vec<f64> {
    let a = 2.0 / (n as f64 + 1.0);
    let mut out = Vec::with_capacity(v.len());
    for (i, &x) in v.iter().enumerate() { out.push(if i == 0 { x } else { out[i - 1] + a * (x - out[i - 1]) }); }
    out
}
pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if id != ID || from >= to || to >= x.r.len() { return None; }
    let (nf, ns, nsg, na) = (g(p, "fast", 7.) as usize, g(p, "slow", 10.) as usize, g(p, "sig", 4.) as usize, g(p, "avg", 5.) as usize);
    let (stop, flat, step, lim, natr, rf) = (g(p, "stop", 1.), g(p, "flat", 1.), g(p, "afstep", 0.0025), g(p, "aflimit", 0.6), g(p, "natr", 2.25), g(p, "ref", 1000.));
    let (h, l, c, n) = (&x.h, &x.l, &x.c, x.c.len());
    let macd: Vec<f64> = ema(c, nf).iter().zip(ema(c, ns)).map(|(a, b)| a - b).collect();
    let sig = ema(&macd, nsg);
    let diff: Vec<f64> = macd.iter().zip(&sig).map(|(a, b)| a - b).collect();
    let ma = sma(c, na);
    let atr = avg_true_range(x, g(p, "atr", 5.) as usize);
    let (mut prev_pos, mut stop_px, mut af, mut hh, mut hh_prev) = (0i8, 0.0f64, 0.0f64, 0.0f64, 0.0f64);
    Some(run_ts(x, from, to, |i, st, now, next| {
        let imp = st.pos;
        let warm = i < na.max(2);
        let (cu, cd) = (!warm && diff[i] > 0. && diff[i - 1] <= 0., !warm && diff[i] < 0. && diff[i - 1] >= 0.);
        if imp == 1 && prev_pos != 1 { stop_px = l[i] - atr[i] * natr; af = step; hh = h[i]; hh_prev = f64::NAN; }
        else if imp == 1 { hh_prev = hh; if h[i] > hh { hh = h[i]; } stop_px += af * (hh - stop_px); if hh > hh_prev && af < lim { af += step.min(lim - af); } }
        if imp == 1 && stop_px > l[i] { stop_px = l[i]; }
        if imp == -1 && prev_pos != -1 { stop_px = h[i] + atr[i] * natr; af = step; hh = l[i]; hh_prev = f64::NAN; }
        else if imp == -1 { hh_prev = hh; if l[i] < hh { hh = l[i]; } stop_px -= af * (stop_px - hh); if hh < hh_prev && af < lim { af += step.min(lim - af); } }
        if imp == -1 && stop_px < h[i] { stop_px = h[i]; }
        prev_pos = imp;
        let _ = now;
        if i + 1 >= n || warm || !ma[i].is_finite() || !atr[i].is_finite() { return; }
        if imp != 1 && cu && c[i] > ma[i] { next.push(o(A::Buy, P::Mkt, "B")); }
        if imp != -1 && cd && c[i] < ma[i] { next.push(o(A::Sell, P::Mkt, "S")); }
        if imp == 1 {
            next.push(o(A::Xl, P::Stop(stop_px), "LX"));
            if st.bse(i) >= 1 { next.push(o(A::Xl, P::Stop(st.entry_px - stop * st.entry_px / rf), "LX stop")); }
            let mpp = h[st.entry_i..=i].iter().cloned().fold(f64::MIN, f64::max) - st.entry_px;
            if mpp >= flat * st.entry_px / rf && cd && c[i] >= ma[i] { next.push(o(A::Xl, P::Mkt, "LX Flat")); }
        }
        if imp == -1 {
            next.push(o(A::Xs, P::Stop(stop_px), "SX"));
            if st.bse(i) >= 1 { next.push(o(A::Xs, P::Stop(st.entry_px + stop * st.entry_px / rf), "SX stop")); }
            let mpp = st.entry_px - l[st.entry_i..=i].iter().cloned().fold(f64::MAX, f64::min);
            if mpp >= flat * st.entry_px / rf && cu && c[i] <= ma[i] { next.push(o(A::Xs, P::Mkt, "SX Flat")); }
        }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn runs() {
        let r = super::super::sctraub::tests::bars(1500);
        let x = Ctx::new(&r, &r, &[]);
        let t = run(ID, &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() > 5 && t.iter().any(|z| z.dir < 0.));
        assert!(run("finfu_x", &x, 0, 5, &Params::new()).is_none());
    }
}

// Instrument admission: SP_U 2026-11-23 (lane K4).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "finfu" && def.id == "finfu_spu",
    label: "An S&P futures system with no result shown and no stated bar size; runs on daily indices and ETFs (a declared application; amounts scaled by price, R4 #340), not the author's test.",
    basis: "SP_U.txt (Financial Facilitators); independent check authors/code-1123/ref/finfu_ref.py",
};
