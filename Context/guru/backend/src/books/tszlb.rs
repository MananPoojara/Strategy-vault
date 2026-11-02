//! Smoothed %b system from ZeroLag.txt, a TradeStation code collection kept in the Pruitt & Hill "Building Winning Trading Systems
//! with TradeStation" code folder (research share USB HDD 2/tRADING KNO-2/George Pruitt, John R.Hill - Building Winning Trading
//! Systems with TradeStation & Code/Code/ZeroLag.txt); no author is named for the system. Book id `tszlb`. Lane K18 (titles loop,
//! 2 Nov 2026), to-code row 329; independent check authors/titles-1030/K18w/k18_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::rrsim::{o, run_ts, sma, A, P};

pub const REVIEWED: &[&str] = &["tszlb_pctb"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] TradeStation code as printed (Price = Close, Length 5, StdDevUp 1, StdDevDn -1, MALength 200, LongExit 0.9, ShortExit 0.1, Smooth 3). BollingerBand = Average +/- deviations x StdDev (TradeStation's population standard deviation); percentb = (Close - BBBot) / (BBTop - BBBot); a zero-width band (five equal closes) keeps the previous %b. Two-pole coefficients for a length N as printed (a1 = exp(-1.414 x 3.14159 / N), b1 = 2 a1 cos(1.414 x 180 / N degrees), coef2 = b1, coef3 = -a1^2, coef1 = 1 - coef2 - coef3). SmoothMA (N = 200) = coef1 x Price + coef2 x Price[1] + coef3 x Price[2], a three-bar filter of price exactly as printed (not recursive); Smoothb (N = 3) = coef1 x %b + coef2 x Smoothb[1] + coef3 x Smoothb[2], its first two values equal to %b (CurrentBar < 3, counted from the first bar with a %b). Buy next bar at the open when Price > SmoothMA and Smoothb < 0.1 on this and the two previous bars; exit long on the close when Smoothb > 0.9; sell short next bar at the open when Price < SmoothMA and Smoothb > 0.9 on three bars; exit short on the close when Smoothb < 0.1. The code's comment says the long signals are more accurate than the shorts; the short rules are in the code, so both sides run (option side 1 = long only). Fills on the vault's TradeStation daily-bar engine (rrsim). The header says it trades the cash indices (SPX, NDX); no results are printed. Runs on ETFs and stocks, and on Indian indices, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "tszlb_pctb", name: "Smoothed %b system: three closes with smoothed %b(5) below 0.1 above the 200 smoother -> buy, exit above 0.9 (shorts mirrored)", group: "TradeStation code: smoothed %b (ZeroLag.txt)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "tszlb", rules: &[
            "F|percentb = (Close - BBBot) / (BBTop - BBBot), BollingerBand(Price, 5, +/-1); Smoothb = coef1*percentb + coef2*Smoothb[1] + coef3*Smoothb[2] (Smooth 3); SmoothMA = MAcoef1*Price + MAcoef2*Price[1] + MAcoef3*Price[2] (MALength 200)",
            "E|If Price > SmoothMA and Smoothb < ShortExit and Smoothb[1] < ShortExit and Smoothb[2] < ShortExit then Buy next bar at Open (short side mirrored with LongExit)",
            "X|If Smoothb > LongExit then ExitLong on Close; If Smoothb < ShortExit then ExitShort on Close"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { reviewed(id).then_some(("tszlb_pctb", "Smoothed %b system (TradeStation)", "5 / 200 / 3")) }

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("len", "Band length (5)", 5., 2., 50., 1.), p("ma", "MALength (200)", 200., 10., 400., 1.), p("smooth", "Smooth (3)", 3., 2., 20., 1.),
        p("side", "1 = long only (shorts off)", 0., 0., 1., 1.)])
}

fn coefs(n: f64) -> (f64, f64, f64) {
    let a1 = (-1.414 * 3.14159 / n).exp();
    let b1 = 2. * a1 * (1.414 * 180. / n).to_radians().cos();
    let (c2, c3) = (b1, -a1 * a1);
    (1. - c2 - c3, c2, c3)
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d);
    let len = g("len", 5.).round().max(2.) as usize;
    let long_only = g("side", 0.) > 0.5;
    let c = &x.c;
    let n = c.len();
    let m = sma(c, len);
    let mut pb = vec![f64::NAN; n];
    for i in len - 1..n {
        let mut q = 0.;
        for k in 0..len { q += (c[i - k] - m[i]).powi(2); }
        let sd = (q / len as f64).sqrt();
        let (top, bot) = (m[i] + sd, m[i] - sd);
        pb[i] = if top - bot != 0. { (c[i] - bot) / (top - bot) } else if i > 0 { pb[i - 1] } else { f64::NAN };
    }
    let (m1, m2, m3) = coefs(g("ma", 200.));
    let sma_ = |i: usize| if i < 2 { c[i] } else { m1 * c[i] + m2 * c[i - 1] + m3 * c[i - 2] };
    let (s1, s2, s3) = coefs(g("smooth", 3.));
    let mut sb = vec![f64::NAN; n];
    let first = len - 1;
    for i in first..n { sb[i] = if i < first + 2 { pb[i] } else { s1 * pb[i] + s2 * sb[i - 1] + s3 * sb[i - 2] }; }
    let (lx, sx) = (0.9, 0.1);
    Some(run_ts(x, from, to, |i, _st, now, next| {
        if i < first + 2 { return; }
        if c[i] > sma_(i) && sb[i] < sx && sb[i - 1] < sx && sb[i - 2] < sx { next.push(o(A::Buy, P::Mkt, "LE")); }
        if sb[i] > lx { now.push(o(A::Xl, P::Mkt, "LX")); }
        if !long_only && c[i] < sma_(i) && sb[i] > lx && sb[i - 1] > lx && sb[i - 2] > lx { next.push(o(A::Sell, P::Mkt, "SE")); }
        if sb[i] < sx { now.push(o(A::Xs, P::Mkt, "SX")); }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn smoothed_pctb_trades() {
        let r = super::super::sctraub::tests::bars(2500);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("tszlb_pctb", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() > 5 && t.iter().any(|z| z.dir > 0.));
        let mut q = Params::new(); q.insert("side".into(), 1.);
        assert!(run("tszlb_pctb", &x, 0, r.len() - 1, &q).unwrap().iter().all(|z| z.dir > 0.));
        assert!(run("tszlb_x", &x, 0, 5, &Params::new()).is_none());
    }
}

// Instrument admission: smoothed %b system 2026-11-02 (titles lane K18).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "tszlb" && reviewed(def.id),
    label: "TradeStation code written for the US cash indices (SPX, NDX), with no printed results. Runs on ETFs, stocks and Indian indices are a declared application.",
    basis: "ZeroLag.txt in the Pruitt & Hill code folder (printed code, no author named); independent check authors/titles-1030/K18w/k18_ref.py",
};
