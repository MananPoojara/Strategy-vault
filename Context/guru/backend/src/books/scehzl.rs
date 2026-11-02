//! John Ehlers and Ric Way, "Zero Lag (Well, Almost)", Technical Analysis of Stocks & Commodities, November 2010, pp.30-35,
//! sidebar "EasyLanguage For The Zero-Lag Trading Strategy" (research share Trading Knowledge I/Stocks & Commodities 2010 -2011
//! (traders.com)/SC2010-11.pdf, PDF p.30-35). Book id `scehzl`. Lane K19 (titles loop, 2 Nov 2026), to-code row 296;
//! independent check authors/titles-1030/K19w/k19_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::sctraub::next_open;

pub const REVIEWED: &[&str] = &["scehzl_ec"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] The strategy sidebar's inputs Length 20, GainLimit 50, Thresh 1 are the default (the code is the rule, decision 82); the article's MSFT test set, length 32, gain limit 22, thresh 0.75, is option preset len = 32, gain = 22, thresh = 0.75. EMA = alpha x Close + (1 - alpha) x EMA[1] with alpha = 2 / (Length + 1); for each bar the gain Value1 / 10, Value1 = -GainLimit ... GainLimit, with the least |Close - EC| is kept (the first one on a tie, as the strict '<' test does), then EC = alpha x (EMA + BestGain x (Close - EC[1])) + (1 - alpha) x EC[1]. EMA and EC start at 0 like TradeStation variables, so an instrument's first months are warm-up. 'Crosses over' = EC above EMA after being at or below it on the previous bar. Both orders are next-bar market orders at the open; always in once the first fills. Article: MSFT, 5/11/2005-3/1/2010, 100 shares, 32/22/0.75: 20 trades, 60% profitable, net $3,107, profit factor 4.1. Runs on indices, ETFs and stocks, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "scehzl_ec", name: "Ehlers & Way zero-lag error-correcting EMA: reverse when EC crosses the 20-bar EMA with the least error above 1% of the close, next open", group: "S&C: Zero Lag (Well, Almost) (Ehlers & Way, Nov 2010)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "scehzl", rules: &[
            "F|\"EMA = alpha*Close + (1 - alpha)*EMA[1]; LeastError = 1000000; For Value1 = -GainLimit to GainLimit Begin Gain = Value1/10; EC = alpha*(EMA + Gain*(Close - EC[1])) + (1 - alpha)*EC[1]; Error = Close - EC; If AbsValue(Error) < LeastError Then Begin LeastError = AbsValue(Error); BestGain = Gain; End; End; EC = alpha*(EMA + BestGain*(Close - EC[1])) + (1 - alpha)*EC[1]\" (sidebar, PDF p.32)",
            "E|\"If EC Crosses Over EMA and 100*LeastError / Close > Thresh Then Buy Next Bar on Open\" (Length 20, GainLimit 50, Thresh 1)",
            "X|\"If EC Crosses Under EMA and 100*LeastError / Close > Thresh Then Sell Short Next Bar on Open\" (sidebar)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    reviewed(id).then_some(("scehzl_ec", "Ehlers & Way zero-lag EC (S&C)", "20 / 50 / 1"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("len", "Length (code 20; MSFT test 32)", 20., 2., 100., 1.), p("gain", "GainLimit (code 50; MSFT test 22)", 50., 1., 100., 1.),
        p("thresh", "Thresh, % of the close (code 1; MSFT test 0.75)", 1., 0., 5., 0.05)])
}

/// (EMA, EC, LeastError) per bar.
pub fn zero_lag(c: &[f64], len: f64, gl: i64) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let a = 2.0 / (len + 1.0);
    let (mut ema, mut ec, mut le) = (Vec::with_capacity(c.len()), Vec::with_capacity(c.len()), Vec::with_capacity(c.len()));
    let (mut e1, mut ec1) = (0.0, 0.0);
    for &cl in c {
        let e = a * cl + (1.0 - a) * e1;
        let (mut least, mut best) = (1000000.0_f64, 0.0);
        for v in -gl..=gl {
            let g = v as f64 / 10.0;
            let err = (cl - (a * (e + g * (cl - ec1)) + (1.0 - a) * ec1)).abs();
            if err < least { least = err; best = g; }
        }
        let z = a * (e + best * (cl - ec1)) + (1.0 - a) * ec1;
        ema.push(e); ec.push(z); le.push(least);
        e1 = e; ec1 = z;
    }
    (ema, ec, le)
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d);
    let (ema, ec, le) = zero_lag(&x.c, g("len", 20.).round().max(1.), g("gain", 50.).round().max(0.) as i64);
    let th = g("thresh", 1.);
    Some(next_open(x, from, to, "reversed", |t, side| {
        if t == 0 || 100.0 * le[t] / x.c[t] <= th { return side; }
        if ec[t] > ema[t] && ec[t - 1] <= ema[t - 1] { 1.0 } else if ec[t] < ema[t] && ec[t - 1] >= ema[t - 1] { -1.0 } else { side }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn gain_search_and_reversals() {
        let (ema, ec, _) = zero_lag(&[0., 0., 10., 10.], 3., 50);
        assert_eq!(ema[2], 5.0);
        assert!(ec[2] > ema[2]);
        let r = super::super::sctraub::tests::bars(2000);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("scehzl_ec", &x, 0, r.len() - 1, &[("thresh".to_string(), 0.)].into_iter().collect()).unwrap();
        assert!(t.len() > 5);
        for w in t.windows(2) { assert_eq!(w[0].exit_i, w[1].legs[0].i); }
    }
}

// Instrument admission: S&C Ehlers & Way zero lag 2026-11-02 (titles lane K19).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "scehzl" && reviewed(def.id),
    label: "Ehlers and Way tested the zero-lag strategy on MSFT stock (2005-2010). Runs on indices, ETFs and other stocks, US and India, are a declared application.",
    basis: "Technical Analysis of Stocks & Commodities, November 2010, pp.30-35 (Zero Lag (Well, Almost), EasyLanguage strategy sidebar); independent check authors/titles-1030/K19w/k19_ref.py",
};
