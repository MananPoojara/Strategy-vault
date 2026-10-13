//! D. Aronson, Evidence-Based Technical Analysis (Wiley 2007), Part II case study (ch.8-9, p.389-450).
//! Only rules whose input is the traded index's own close are Live (input series 1 = S&P 500 close).
//! Source: library/arebta/.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

const NOTE: &str = "Case study: 6,402 binary long/short reversal rules on the S&P 500, 1 Nov 1980 - 1 Jul 2005, evaluated on DETRENDED daily returns (mean daily change subtracted, p.391-392); entries and exits \"on the open price of the day following a position-reversal signal\" (p.449). Input series 1 = S&P 500 close; here the traded index's own close (instrument swap for NIFTY). Always in the market once the rule output is defined (the position is taken at the first open after the selected start).";
const OTHER: &str = "the other 38 inputs (DJIA H/L/C, transports, utilities, NYSE financials, DJ 20 bonds, NASDAQ Composite, Value Line, NYSE advances/declines/unchanged, new highs/lows, total/up/down volume, T-bill, T-bond, AAA, BAA yields and 29 indicators built from them, Table 8.1-8.2) are not in the vault as extra_data series";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, dir, status, rules, notes, claim| StrategyDef { id, name, group: "Ch.8 S&P 500 rule data-mining case study", session: 0, dir, status, rules, notes,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "arebta", claim };
    vec![
        d("arebta_tt", "Traditional trend rule TT-1-n (channel breakout of the close)", "Long/Short", Status::Live,
          &["F|CBO(n): upper = max of the series over the past n periods, lower = min, \"not including the current period\" (p.397)",
            "E|\"signals long positions when the analyzed series exceeds its maximum value established over the past n-periods ... short positions are signaled when the time series falls below its n-period minimum\" (p.397); otherwise the output is unchanged",
            "E|\"A traditional version of a trend rule produced an output value of +1 (long position in the S&P 500) when the input time series was determined to be in an upward trend ... -1 (short)\" (p.420)",
            "X|Reversal only; trades at the next open (p.449). n tested: 3, 5, 8, 12, 18, 27, 41, 61, 91, 137, 205 (p.420)"],
          NOTE, None),
        d("arebta_ti", "Inverse trend rule TI-1-n", "Long/Short", Status::Live,
          &["E|\"Inverse trend rules simply produced the opposite signals (e.g., short S&P 500 when the analyzed series was determined to be in an uptrend)\" (p.420)",
            "F|Same CBO(n) as TT (p.397), n in {3, 5, 8, 12, 18, 27, 41, 61, 91, 137, 205}", "X|Reversal only; next open (p.449)"],
          NOTE, None),
        d("arebta_e", "Extreme-value & transition rule E-type-1-disp-span", "Long/Short", Status::Live,
          &["F|Indicator = CN(LMA(series, 4), N): 4-day linearly weighted MA (weights 4,3,2,1 / 10, p.401) then channel normalisation CN = (S - min_N)/(max_N - min_N) x 100 over the last N days (p.402, 421)",
            "F|Thresholds 50 +/- displacement (10 or 20); N in {15, 30, 60} (p.429)",
            "E|Events: 1 lower threshold crossed downward, 2 lower crossed upward, 3 upper crossed upward, 4 upper crossed downward (p.422)",
            "E|Type table (long entry/short exit ; short entry/long exit): 1:1;2 2:1;3 3:1;4 4:2;3 5:2;4 6:3;4 7:2;1 8:3;1 9:4;1 10:3;2 11:4;2 12:4;3 (Table 8.3, p.423)",
            "X|Reversal only; next open (p.449). Default E-2-1-20-30 (the series-1 rule printed in Table 9.1)"],
          "Crossing = previous value on/at one side, today strictly beyond (equality convention not stated by the book). CN undefined (flat window) = no event. Same timing/detrending notes as the trend rules.",
          Some(crate::connors::Claim { trades: "-", win: "-", avg: "7.09%/yr mean return on detrended S&P 500 (rule E-2-1-20-30)", hold: "-", period: "11/1/1980 - 7/1/2005, S&P 500 (Table 9.1 p.445; not significant after data-mining correction)" })),
        d("arebta_other_inputs", "TT/TI, E and divergence rules on the other 38 input series", "Long/Short", Status::NeedsData(OTHER),
          &["E|Same TT/TI (p.420), E (p.421-430) rules on inputs 3-40", "E|Divergence D-type-series-disp-span: CN{CN(series, n) - CN(S&P 500, n), 10n} with the 12 threshold types of Table 8.4 (p.433-440)"],
          "Best rule overall E-12-28-10-30 (cumulative money flow): 10.25%/yr detrended, p = 0.8164 (WRC), not significant (p.441).", None),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "arebta_tt" => ("arebta_trend", "Aronson trend rules (series 1)", "TT · traditional"),
        "arebta_ti" => ("arebta_trend", "Aronson trend rules (series 1)", "TI · inverse"),
        _ => return catalogue().into_iter().find(|d| d.id == id).map(|d| (d.id, d.name, "")),
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(match id {
        "arebta_tt" | "arebta_ti" => vec![p("n", "CBO look-back (book grid 3,5,8,12,18,27,41,61,91,137,205)", 3., 3., 205., 1.)],
        "arebta_e" => vec![p("type", "E-rule type 1-12 (Table 8.3)", 2., 1., 12., 1.), p("disp", "Threshold displacement from 50 (book: 10 or 20)", 20., 10., 20., 10.), p("span", "CN look-back N (book: 15, 30, 60)", 30., 15., 60., 15.)],
        _ => return None,
    })
}

/// Rule output +1/-1 (0 = not yet defined) at each close.
fn output(id: &str, x: &Ctx, g: &dyn Fn(&str) -> f64) -> Vec<f64> {
    let c = &x.c;
    let mut out = vec![0.; c.len()];
    let mut s = 0.;
    if id == "arebta_e" {
        let (ty, disp, span) = (g("type") as usize, g("disp"), g("span") as usize);
        const T: [(u8, u8); 12] = [(1, 2), (1, 3), (1, 4), (2, 3), (2, 4), (3, 4), (2, 1), (3, 1), (4, 1), (3, 2), (4, 2), (4, 3)];
        let (le, se) = T[ty.clamp(1, 12) - 1];
        let (lo, up) = (50. - disp, 50. + disp);
        let lma: Vec<f64> = (0..c.len()).map(|i| if i < 3 { f64::NAN } else { (4. * c[i] + 3. * c[i - 1] + 2. * c[i - 2] + c[i - 3]) / 10. }).collect();
        let cn: Vec<f64> = (0..c.len()).map(|i| {
            if i + 1 < span + 3 { return f64::NAN; }
            let w = &lma[i + 1 - span..=i];
            let (mn, mx) = (w.iter().copied().fold(f64::MAX, f64::min), w.iter().copied().fold(f64::MIN, f64::max));
            if mx > mn { (lma[i] - mn) / (mx - mn) * 100. } else { f64::NAN }
        }).collect();
        let ev = |e: u8, a: f64, b: f64| match e { 1 => a >= lo && b < lo, 2 => a <= lo && b > lo, 3 => a <= up && b > up, _ => a >= up && b < up };
        for i in 1..c.len() {
            let (a, b) = (cn[i - 1], cn[i]);
            if a.is_finite() && b.is_finite() {
                if ev(le, a, b) { s = 1.; } else if ev(se, a, b) { s = -1.; }
            }
            out[i] = s;
        }
    } else {
        let n = g("n").max(1.) as usize;
        let inv = if id == "arebta_ti" { -1. } else { 1. };
        for i in n..c.len() {
            let w = &c[i - n..i];
            if c[i] > w.iter().copied().fold(f64::MIN, f64::max) { s = inv; } else if c[i] < w.iter().copied().fold(f64::MAX, f64::min) { s = -inv; }
            out[i] = s;
        }
    }
    out
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if from > to || to >= x.c.len() { return None; }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap().def);
    let sig = output(id, x, &g);
    let (mut out, mut cur): (Vec<Trade>, Option<Trade>) = (vec![], None);
    for i in from..to {
        let want = sig[i];
        if want == 0. || cur.as_ref().is_some_and(|t| t.dir == want) { continue; }
        let px = x.o[i + 1];
        if let Some(mut t) = cur.take() { t.exit_i = i + 1; t.exit_px = px; t.signal_i = i; t.reason = "Rule output reversed · next open".into(); out.push(t); }
        cur = Some(Trade { dir: want, legs: vec![Leg { i: i + 1, px, w: 1. }], exit_i: i + 1, exit_px: px, reason: String::new(), entry_kind: "open", signal_i: i, open: false, exits: vec![] });
    }
    if let Some(mut t) = cur { t.exit_i = to; t.exit_px = x.c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true; out.push(t); }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn tt3_reverses_on_next_open() {
        let cl = [10., 11., 12., 13., 12.5, 12., 9., 9.5];
        let r: Vec<Row> = cl.iter().enumerate().map(|(i, &c)| Row { ticker: "T".into(), date: format!("{:04}-01-01", 2000 + i), open: c + 0.1, high: c + 1., low: c - 1., close: c, volume: None }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let t = run("arebta_tt", &x, 0, 7, &Params::new()).unwrap();
        // bar 3: 13 > max(10,11,12) -> long at open of bar 4; bar 6: 9 < min(13,12.5,12) -> short at open of bar 7
        assert_eq!((t[0].dir, t[0].legs[0].i, t[0].exit_i), (1., 4, 7));
        assert_eq!((t[1].dir, t[1].open), (-1., true));
        let ti = run("arebta_ti", &x, 0, 7, &Params::new()).unwrap();
        assert_eq!(ti[0].dir, -1.);
    }
}
