//! Bollinger Master (TamFx), MetaTrader 4 expert advisor from the "33 Experts from 2008 MQL Championship" folder (research share
//! USB HDD 1/META TRADER/MetaTrader/33 Experts from 2008 MQL Championship/Bollinger Master.mq4). Book id `mtbbm`. Lane K18 (titles
//! loop, 2 Nov 2026), to-code row 327; independent check authors/titles-1030/K18w/k18_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::rrsim::{o, run_ts, sma, A, P};

pub const REVIEWED: &[&str] = &["mtbbm_bands"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] MetaTrader 4 expert advisor with no timeframe of its own, run on daily bars as a declared application (decision 114). Bands as MetaTrader's iBands: 20-bar simple average of the close +/- deviation x the population standard deviation of those 20 closes. Signal at a bar's close t (the EA's next tick): buy when Close[2] (bar t - 1) is below the narrow (2) lower band of bar t (shift 1); else sell when it is above the narrow upper band; fill at the next open. Exits fixed at entry from the wide (3) bands of bar t - 1 (shift 2): target 0.5 x (upper - lower) and stop 1.0 x (upper - lower) away from the entry price, tested from the entry bar on with the vault's TradeStation-style bar path (open, nearer extreme, other extreme). One position at a time: the EA adds orders while the signal holds (up to maxOrderCount 3, lots from free margin) and closes the worst order when margin runs low; neither is modelled. After a stop or target the next entry waits for the next open. HELD: the 15-point minimum and 200-point maximum on the stop and target distances (point amounts, decision 19). No results are printed. Runs on indices, ETFs and stocks, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "mtbbm_bands", name: "Bollinger Master EA: close two bars back beyond the 2-SD band -> fade it, target 0.5 and stop 1.0 x the 3-SD band width", group: "MT4 EA: Bollinger Master (2008 MQL Championship)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "mtbbm", rules: &[
            "E|if (Close[2] < lowBandNarrow) OrderSend(OP_BUY ...) else if (Close[2] > highBandNarrow) OrderSend(OP_SELL ...); narrow = iBands(20, 2, shift 1)",
            "X|stoploss = entry -/+ (highBandWide - lowBandWide) x 1, takeprofit = entry +/- (highBandWide - lowBandWide) x 0.5; wide = iBands(20, 3, shift 2)",
            "X|The 15 / 200-point clamps on those distances are held (point amounts)."],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { reviewed(id).then_some(("mtbbm_bands", "Bollinger Master EA (MT4)", "20, 2 / 3")) }

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("n", "Band period (20)", 20., 5., 100., 1.), p("narrow", "Narrow deviation (2)", 2., 0.5, 4., 0.1), p("wide", "Wide deviation (3)", 3., 0.5, 5., 0.1),
        p("tp", "Target x wide band width (0.5)", 0.5, 0.1, 3., 0.1), p("sl", "Stop x wide band width (1)", 1., 0.1, 3., 0.1)])
}

/// MetaTrader iBands: (upper, lower) with a population standard deviation.
pub fn bands(c: &[f64], n: usize, dev: f64) -> (Vec<f64>, Vec<f64>) {
    let m = sma(c, n);
    let (mut u, mut l) = (vec![f64::NAN; c.len()], vec![f64::NAN; c.len()]);
    for i in n.saturating_sub(1)..c.len() {
        let mut q = 0.;
        for k in 0..n { q += (c[i - k] - m[i]).powi(2); }
        let sd = (q / n as f64).sqrt();
        u[i] = m[i] + dev * sd; l[i] = m[i] - dev * sd;
    }
    (u, l)
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d);
    let n = g("n", 20.).round().max(2.) as usize;
    let (nu, nl) = bands(&x.c, n, g("narrow", 2.));
    let (wu, wl) = bands(&x.c, n, g("wide", 3.));
    let (tpk, slk) = (g("tp", 0.5), g("sl", 1.));
    let len = x.c.len();
    let (mut stop, mut tgt) = (0., 0.);
    Some(run_ts(x, from, to, |i, st, _now, next| {
        if st.pos == 0 {
            if i < 2 || i + 1 >= len { return; }
            let w = wu[i - 1] - wl[i - 1];
            let (tpp, slp) = (w * tpk, w * slk);
            let ep = x.o[i + 1];
            if x.c[i - 1] < nl[i] {
                stop = ep - slp; tgt = ep + tpp;
                next.push(o(A::Buy, P::Mkt, "BB buy"));
                next.push(o(A::Xl, P::Stop(stop), "stop")); next.push(o(A::Xl, P::Lim(tgt), "target"));
            } else if x.c[i - 1] > nu[i] {
                stop = ep + slp; tgt = ep - tpp;
                next.push(o(A::Sell, P::Mkt, "BB sell"));
                next.push(o(A::Xs, P::Stop(stop), "stop")); next.push(o(A::Xs, P::Lim(tgt), "target"));
            }
        } else if st.pos > 0 {
            next.push(o(A::Xl, P::Stop(stop), "stop")); next.push(o(A::Xl, P::Lim(tgt), "target"));
        } else {
            next.push(o(A::Xs, P::Stop(stop), "stop")); next.push(o(A::Xs, P::Lim(tgt), "target"));
        }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bollinger_master_exits_at_levels() {
        let r = super::super::sctraub::tests::bars(2000);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("mtbbm_bands", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() > 5 && t.iter().any(|z| z.dir > 0.) && t.iter().any(|z| z.dir < 0.));
        for z in &t { if !z.open { assert!(z.exit_i >= z.legs[0].i); } }
        assert!(run("mtbbm_x", &x, 0, 5, &Params::new()).is_none());
    }
}

// Instrument admission: Bollinger Master EA 2026-11-02 (titles lane K18).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "mtbbm" && reviewed(def.id),
    label: "MetaTrader 4 forex expert advisor from the 2008 MQL Championship folder, with no published results. Runs on daily indices, ETFs and stocks, US and India, are a declared application.",
    basis: "Bollinger Master.mq4 (TamFx, printed code); independent check authors/titles-1030/K18w/k18_ref.py",
};
