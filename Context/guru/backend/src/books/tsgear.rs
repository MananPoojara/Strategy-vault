//! SqueezePlay package (impressivetrading.com), TradeStation 2000i MACD-histogram system "Open Code.txt" (research share USB HDD 1/
//! TradeStation Systems & Indicators/SqueezePlay for TS2000i, TS 7 (impressivetrading.com)/Open Code.rar -> Open Code.txt).
//! Book id `tsgear`. Lane K18 (titles loop, 2 Nov 2026), to-code row 339; independent check authors/titles-1030/K18w/k18_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::rrsim::{o, run_ts, sma, xavg, A, P};
use super::sctraub::ticks;

pub const REVIEWED: &[&str] = &["tsgear_macd"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] TradeStation 2000i code as printed (FastLength 8, SlowLength 66, MACDLength 3, BuyAvg 4, SellAvg 17). iMACD = MACD(Close, 8, 66) = XAverage(Close, 8) - XAverage(Close, 66), iAvg = XAverage(iMACD, 3), iDiff = iMACD - iAvg (TradeStation exponential averages seeded with the first value). When not long, iDiff crosses over 0 and the close is above its 4-bar average: buy next bar with a limit at the close - LimitFade points; when not short, iDiff crosses under 0 and the close is below its 17-bar average: sell short next bar with a limit at the close + LimitFade points. 'LimitFade POINT' is 0.25 of TradeStation's point (one minimum tick, decision 57), so the default limit sits a quarter tick from the close (option fade 0 = at the close). The limit lives for the next bar only. An opposite entry reverses the position. HELD (decision 19): the 15-point SetStopLoss (StopAmount x BigPointValue), the 5.75-point half-position profit target, the $550 break-even switch, the 15-point stop under it and the $2,100 profit floor that turns on the swing trailing stop (Strength 7); one unit is traded (NoOfContracts 2 sets the half-position target, which is held). Fills on the vault's TradeStation daily-bar engine (rrsim). No market is named and no results are printed. Runs on daily indices, ETFs and stocks, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "tsgear_macd", name: "SqueezePlay MACD(8, 66) histogram cross with a 4 / 17-bar average filter, limit entry at the close, stop and reverse", group: "TradeStation code: SqueezePlay MACD system (impressivetrading.com)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "tsgear", rules: &[
            "E|If iMP <> 1 and iDiff CROSS OVER 0 and CLOSE > average(CLOSE, BuyAvg) then BUY NEXT BAR CLOSE - LimitFade POINT LIMIT (iDiff = MACD(C, 8, 66) - XAverage(MACD, 3), BuyAvg 4)",
            "E|If iMP <> -1 and iDiff CROSS UNDER 0 and CLOSE < average(CLOSE, SellAvg) then SELL SHORT NEXT BAR CLOSE + LimitFade POINT LIMIT (SellAvg 17)",
            "X|The opposite entry reverses. Point and dollar exits (15-point stop, 5.75-point target, $550 break-even, $2,100 trailing floor) held."],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { reviewed(id).then_some(("tsgear_macd", "SqueezePlay MACD system (TradeStation)", "8 / 66 / 3")) }

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("fast", "FastLength (8)", 8., 2., 50., 1.), p("slow", "SlowLength (66)", 66., 5., 200., 1.), p("tick", "LimitFade in ticks (0.25 point)", 0.25, 0., 5., 0.25)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d).round().max(1.) as usize;
    let mut q = p.clone();
    q.entry("tick".into()).or_insert(0.25);
    let fade = ticks(x, &q);
    let c = &x.c;
    let (f, s) = (xavg(c, g("fast", 8.)), xavg(c, g("slow", 66.)));
    let m: Vec<f64> = (0..c.len()).map(|i| f[i] - s[i]).collect();
    let av = xavg(&m, 3);
    let d: Vec<f64> = (0..c.len()).map(|i| m[i] - av[i]).collect();
    let (b4, s17) = (sma(c, 4), sma(c, 17));
    Some(run_ts(x, from, to, |i, st, _now, next| {
        if i == 0 { return; }
        if st.pos != 1 && d[i] > 0. && d[i - 1] <= 0. && c[i] > b4[i] { next.push(o(A::Buy, P::Lim(c[i] - fade), "B")); }
        if st.pos != -1 && d[i] < 0. && d[i - 1] >= 0. && c[i] < s17[i] { next.push(o(A::Sell, P::Lim(c[i] + fade), "S")); }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn squeezeplay_reverses() {
        let r = super::super::sctraub::tests::bars(2500);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("tsgear_macd", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() > 3 && t.iter().any(|z| z.dir > 0.) && t.iter().any(|z| z.dir < 0.));
        for w in t.windows(2) { assert_eq!(w[0].exit_i, w[1].legs[0].i); }
        assert!(run("tsgear_x", &x, 0, 5, &Params::new()).is_none());
    }
}

// Instrument admission: SqueezePlay MACD system 2026-11-02 (titles lane K18).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "tsgear" && reviewed(def.id),
    label: "TradeStation futures system with no named market and no published results. Runs on daily indices, ETFs and stocks, US and India, are a declared application.",
    basis: "SqueezePlay Open Code.txt (impressivetrading.com, printed code); independent check authors/titles-1030/K18w/k18_ref.py",
};
