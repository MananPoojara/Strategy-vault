//! Sigma Trend System (sigmatrend.com), TradeStation code "Open Code.txt" (research share USB HDD 1/TradeStation Systems &
//! Indicators/Sigma Trend System for TS (sigmatrend.com)/Open Code.rar -> Open Code.txt). Book id `tssigma`. Lane K18 (titles loop,
//! 2 Nov 2026), to-code row 345; independent check authors/titles-1030/K18w/k18_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::rrsim::{o, run_ts, A, P};

pub const REVIEWED: &[&str] = &["tssigma_ama"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] TradeStation code as printed (period 70, fastend 0.666, slowend 0.0645), Kaufman's adaptive moving average: for the first 70 bars ama = the close; then efratio = |Close - Close[70]| / the sum of |Close - Close[1]| over 70 bars, smooth = (efratio x (0.666 - 0.0645) + 0.0645)^2, ama = ama[1] + smooth x (Close - ama[1]). extlow = ama[1] when the ama turns up, exthigh = ama[1] when it turns down (both start at 0, as the code's variables do). Buy this bar on the close when the ama rises and ama - extlow exceeds the filter; sell short this bar on the close when it falls and exthigh - ama exceeds the filter (stop and reverse, no other exit). The filter (0.2) is a fixed price amount, so it is held (decision 19): the default filter is 0 (option filter = the code's 0.2 price units, literal). With extlow still 0 the first rising bar buys. Fills on the vault's TradeStation daily-bar engine (rrsim). No market is named and no results are printed. Runs on daily indices, ETFs and stocks, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "tssigma_ama", name: "Sigma Trend: Kaufman adaptive average (70) turns up from a low -> buy the close, turns down from a high -> short (stop and reverse)", group: "TradeStation code: Sigma Trend (sigmatrend.com)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "tssigma", rules: &[
            "F|efratio = @AbsValue(close - close[period]) / @Summation(dff, period); smooth = @Power(efratio*(fastend - slowend) + slowend, 2); ama = ama[1] + smooth*(close - ama[1]) (period 70, fastend .666, slowend .0645)",
            "E|if (ama > ama[1] and ama - extlow > filter) then Buy This Bar on close; if (ama < ama[1] and exthigh - ama > filter) then Sell Short This Bar on close",
            "X|Stop and reverse only. The 0.2 filter is a price amount: held (default 0)."],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { reviewed(id).then_some(("tssigma_ama", "Sigma Trend (TradeStation)", "AMA 70")) }

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("period", "period (70)", 70., 5., 200., 1.), p("filter", "filter in price units (code 0.2; held, 0)", 0., 0., 50., 0.1)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let per = p.get("period").copied().unwrap_or(70.).round().max(2.) as usize;
    let filter = p.get("filter").copied().unwrap_or(0.);
    let c = &x.c;
    let n = c.len();
    let mut ama = vec![f64::NAN; n];
    let (mut extlow, mut exthigh) = (0., 0.);
    let (fe, se) = (0.666, 0.0645);
    Some(run_ts(x, from, to, |i, _st, now, _next| {
        if i < per { ama[i] = c[i]; return; }
        let signal = (c[i] - c[i - per]).abs();
        let mut noise = 0.;
        for k in (i + 1 - per..=i).rev() { noise += (c[k] - c[k - 1]).abs(); }
        let er = signal / noise;
        let t = er * (fe - se) + se;
        let smooth = t * t;
        ama[i] = ama[i - 1] + smooth * (c[i] - ama[i - 1]);
        if ama[i] > ama[i - 1] && ama[i - 1] < ama[i - 2] { extlow = ama[i - 1]; }
        if ama[i] < ama[i - 1] && ama[i - 1] > ama[i - 2] { exthigh = ama[i - 1]; }
        if ama[i] > ama[i - 1] && ama[i] - extlow > filter { now.push(o(A::Buy, P::Mkt, "LE")); }
        if ama[i] < ama[i - 1] && exthigh - ama[i] > filter { now.push(o(A::Sell, P::Mkt, "SE")); }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sigma_reverses() {
        let r = super::super::sctraub::tests::bars(2500);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("tssigma_ama", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() > 3 && t.iter().any(|z| z.dir > 0.) && t.iter().any(|z| z.dir < 0.));
        assert!(run("tssigma_x", &x, 0, 5, &Params::new()).is_none());
    }
}

// Instrument admission: Sigma Trend 2026-11-02 (titles lane K18).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "tssigma" && reviewed(def.id),
    label: "TradeStation adaptive-average trend system with no named market and no published results. Runs on daily indices, ETFs and stocks, US and India, are a declared application.",
    basis: "Sigma Trend System Open Code.txt (sigmatrend.com, printed code); independent check authors/titles-1030/K18w/k18_ref.py",
};
