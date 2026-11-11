//! D. Aronson, Evidence-Based Technical Analysis (Wiley 2007), Part II case study (ch.8-9, p.389-450).
//! Only rules whose input is the traded index's own close are Live (input series 1 = S&P 500 close).
//! Source: library/arebta/.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};
/// Admitted ids (authors/aronson/admission/SOURCE-CONTRACT.md).
pub const REVIEWED: &[&str] = &["arebta_tt", "arebta_ti", "arebta_e", "arebta_tt_br", "arebta_ti_br", "arebta_e_br", "arebta_d_br" /* lane K78, R4 #276 (3), #287 */];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "Case study: 6,402 binary long/short reversal rules on the S&P 500, 1 Nov 1980 - 1 Jul 2005, evaluated on DETRENDED daily returns (mean daily change subtracted, p.391-392); entries and exits \"on the open price of the day following a position-reversal signal\" (p.449). Input series 1 = S&P 500 close; here the traded index's own close (instrument swap for NIFTY). Always in the market once the rule output is defined (the position is taken at the first open after the selected start).";
const OTHER: &str = "the other 33 rule inputs of Table 8.2 (p.418) are not in the vault: S&P Industrials, Dow Jones transports and utilities, NYSE financials, NASDAQ Composite, Value Line, NYSE new highs/lows, the DJ 20 bonds, T-bill, T-bond, AAA and BAA yields, and the volume indicators. The DJIA high/low/close (Table 8.1 raw series 3-5) feed only the accumulation-distribution and money-flow indicators (inputs 12-17, p.410-411), which multiply by NYSE volume; NYSE volume changes units on 2002-06-21 (thousands to shares), so every volume-level input (9-23, 33) is excluded (R4 #287). The six breadth inputs 24-29 are coded as arebta_tt_br, arebta_ti_br, arebta_e_br and arebta_d_br";
/// Lane K78 (R4 #276 (3), #287): Table 8.2 breadth inputs 24-29 from NYSE advances/declines/unchanged and up/down/unchanged volume.
pub const BREADTH: &[&str] = &["NYSE_ADV_DEC.Advances", "NYSE_ADV_DEC.Declines", "NYSE_ADV_DEC.Unchanged", "NYSE_ADV_DEC.AdvVolume", "NYSE_ADV_DEC.DecVolume", "NYSE_ADV_DEC.UnchVolume"];
pub fn breadth(id: &str) -> bool { matches!(id, "arebta_tt_br" | "arebta_ti_br" | "arebta_e_br" | "arebta_d_br") }
const BR_NOTE: &str = "Lane K78 (R4 #276 (3), #287, #288). Input = NYSE breadth (extra_data NYSE_ADV_DEC, all NYSE issues, 1965-03-01..2020-02-10, as the book's 'all issues traded on the NYSE', p.413); traded = the instrument's own next open (the book traded the S&P 500; other US indices, ETFs and stocks are a declared application). US only (no NYSE breadth for Indian instruments). Runs go to the end of the breadth data (2020-02-10); the book's test period 1 Nov 1980 - 1 Jul 2005 (p.393) is used only for the book check (R4 #288 (1)). Up/down/unchanged volume switch together from thousands to shares on 2002-06-21 (the source lacks 2002-06-20), so the volume RATIO (inputs 27-29) has no unit break; volume levels are not used. [Declared] A day without a breadth print is skipped by the indicator (sums, averages and channels count breadth days only) and the position is unchanged; crossing = previous value on/at one side, today strictly beyond; a flat channel = no value. No detrending (the book scores detrended returns, p.391-392); one unit per trade.";
const INPUTS: &str = "Input (Table 8.2, p.418): 24 CADR, 25 ADR10, 26 ADR30, 27 CUDR (cum. up-down volume ratio), 28 UDR10, 29 UDR30";

fn br(d: StrategyDef) -> StrategyDef { StrategyDef { needs_series: BREADTH, ..d } }

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
          "Best rule overall E-12-28-10-30 (input 28 = up-down volume ratio 10-day MA, Table 8.2 p.418; coded as arebta_e_br): 10.25%/yr detrended, p = 0.8164 (WRC), not significant (p.441).", None),
        br(d("arebta_tt_br", "Traditional trend rule TT on NYSE breadth (inputs 24-29)", "Long/Short", Status::Live,
          &["F|Breadth inputs (p.413-415): ADR = (advancing - declining) / (advancing + declining + unchanged NYSE issues); NVR = (up volume - down volume) / (up + down + unchanged volume); CADR / CUDR = cumulative sums; ADR10/ADR30, UDR10/UDR30 = 10- and 30-day moving averages (Table 8.2 inputs 24-29, p.418)",
            "E|CBO(input, n): +1 (long the S&P 500) when the input exceeds its maximum of the past n days, not including today; -1 below the n-day minimum (p.397, 420)",
            "X|Reversal only; next open (p.449). Default TT-28-12 = the mirror of TI-28-12, the best printed trend rule on these inputs (Table 9.1 p.445)"],
          BR_NOTE, None)),
        br(d("arebta_ti_br", "Inverse trend rule TI on NYSE breadth (inputs 24-29)", "Long/Short", Status::Live,
          &["F|Same breadth inputs and CBO(n) as arebta_tt_br (p.397, 413-415, 418)",
            "E|\"Inverse trend rules simply produced the opposite signals\" (p.420): short when the input breaks out up, long when it breaks down",
            "X|Reversal only; next open (p.449). Default TI-28-12 (Table 9.1 p.445: 8.49%/yr detrended)"],
          BR_NOTE, Some(crate::connors::Claim { trades: "-", win: "-", avg: "8.49%/yr mean return on detrended S&P 500 (TI-28-12)", hold: "-", period: "11/1/1980 - 7/1/2005, S&P 500 (Table 9.1 p.445; not significant after data-mining correction)" }))),
        br(d("arebta_e_br", "Extreme-value & transition rule E on NYSE breadth (inputs 24-29)", "Long/Short", Status::Live,
          &["F|Indicator = CN(LMA(input, 4), N), the arebta_e operators on a breadth input (p.401-402, 421)",
            "E|Thresholds 50 +/- 10 or 20; N 15/30/60; 12 types of Table 8.3 (p.422-423, 429)",
            "X|Reversal only; next open (p.449). Default E-12-28-10-30, the best of all 6,402 rules (p.441; Table 9.1 p.445: 10.25%/yr detrended)"],
          BR_NOTE, Some(crate::connors::Claim { trades: "-", win: "-", avg: "10.25%/yr mean return on detrended S&P 500 (E-12-28-10-30)", hold: "-", period: "11/1/1980 - 7/1/2005, S&P 500 (Table 9.1 p.445; p = 0.8164, not significant, p.441)" }))),
        br(d("arebta_d_br", "Divergence rule D: NYSE breadth vs the traded index", "Long/Short", Status::Live,
          &["F|Divergence indicator = CN{CN(companion, n) - CN(S&P 500, n), 10n} (double channel normalisation, p.437-438); companion = a breadth input 24-29; the S&P 500 = the SPX close on the same date (default, every US instrument; R4 #288 (2)); p.ref = 1 (labelled option): the traded instrument's own close",
            "E|Upper/lower thresholds 50 +/- 10 or 20; n 15/30/60; the 12 types of Table 8.4 (p.440) = the event pairs of Table 8.3 (p.439)",
            "X|Reversal only; next open (p.449). Default D-10-28-10-30, the best printed divergence rule on these inputs (Table 9.1 p.445: 8.69%/yr detrended)"],
          BR_NOTE, Some(crate::connors::Claim { trades: "-", win: "-", avg: "8.69%/yr mean return on detrended S&P 500 (D-10-28-10-30)", hold: "-", period: "11/1/1980 - 7/1/2005, S&P 500 (Table 9.1 p.445; not significant after data-mining correction)" }))),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "arebta_tt" => ("arebta_trend", "Aronson trend rules (series 1)", "TT · traditional"),
        "arebta_ti" => ("arebta_trend", "Aronson trend rules (series 1)", "TI · inverse"),
        "arebta_tt_br" => ("arebta_trend_br", "Aronson trend rules on NYSE breadth", "TT · traditional"),
        "arebta_ti_br" => ("arebta_trend_br", "Aronson trend rules on NYSE breadth", "TI · inverse"),
        _ => return catalogue().into_iter().find(|d| d.id == id).map(|d| (d.id, d.name, "")),
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(match id {
        "arebta_tt" | "arebta_ti" => vec![p("n", "CBO look-back (book grid 3,5,8,12,18,27,41,61,91,137,205)", 3., 3., 205., 1.)],
        "arebta_e" => vec![p("type", "E-rule type 1-12 (Table 8.3)", 2., 1., 12., 1.), p("disp", "Threshold displacement from 50 (book: 10 or 20)", 20., 10., 20., 10.), p("span", "CN look-back N (book: 15, 30, 60)", 30., 15., 60., 15.)],
        "arebta_tt_br" | "arebta_ti_br" => vec![p("input", INPUTS, 28., 24., 29., 1.), p("n", "CBO look-back (book grid 3,5,8,12,18,27,41,61,91,137,205)", 12., 3., 205., 1.)],
        "arebta_e_br" => vec![p("input", INPUTS, 28., 24., 29., 1.), p("type", "E-rule type 1-12 (Table 8.3)", 12., 1., 12., 1.), p("disp", "Threshold displacement from 50 (book: 10 or 20)", 10., 10., 20., 10.), p("span", "CN look-back N (book: 15, 30, 60)", 30., 15., 60., 15.)],
        "arebta_d_br" => vec![p("input", INPUTS, 28., 24., 29., 1.), p("type", "D-rule type 1-12 (Table 8.4)", 10., 1., 12., 1.), p("disp", "Threshold displacement from 50 (book: 10 or 20)", 10., 10., 20., 10.), p("span", "First CN look-back n (book: 15, 30, 60); the second is 10n", 30., 15., 60., 15.), p("ref", "S&P 500 series: 0 = the SPX close (the book's, default); 1 = the traded instrument's own close (labelled option)", 0., 0., 1., 1.)],
        _ => return None,
    })
}

const T83: [(u8, u8); 12] = [(1, 2), (1, 3), (1, 4), (2, 3), (2, 4), (3, 4), (2, 1), (3, 1), (4, 1), (3, 2), (4, 2), (4, 3)];

/// CN(s, n) = (s - min) / (max - min) x 100 over the last n values including today (p.402); NaN if any is missing or flat.
fn cn(s: &[f64], n: usize) -> Vec<f64> {
    (0..s.len()).map(|k| {
        if k + 1 < n { return f64::NAN; }
        let w = &s[k + 1 - n..=k];
        if w.iter().any(|v| !v.is_finite()) { return f64::NAN; }
        let (mn, mx) = (w.iter().copied().fold(f64::MAX, f64::min), w.iter().copied().fold(f64::MIN, f64::max));
        if mx > mn { (s[k] - mn) / (mx - mn) * 100. } else { f64::NAN }
    }).collect()
}

/// Lane K78: Table 8.2 input 24-29 on the breadth days (index list into the context) of this context.
fn breadth_input(x: &Ctx, input: usize) -> Option<(Vec<usize>, Vec<f64>)> {
    let vol = input >= 27;
    let col = |k: usize| x.series(BREADTH[k + if vol { 3 } else { 0 }]);
    let (a, d, u) = (col(0)?, col(1)?, col(2)?);
    let days: Vec<usize> = (0..x.c.len()).filter(|&i| a[i].is_finite() && d[i].is_finite() && u[i].is_finite() && a[i] + d[i] + u[i] > 0. && x.c[i].is_finite()).collect();
    let ratio: Vec<f64> = days.iter().map(|&i| (a[i] - d[i]) / (a[i] + d[i] + u[i])).collect();
    let s = match (input - 24) % 3 {
        0 => ratio.iter().scan(0., |acc, r| { *acc += r; Some(*acc) }).collect(),
        k => { let n = if k == 1 { 10 } else { 30 }; (0..ratio.len()).map(|j| if j + 1 < n { f64::NAN } else { ratio[j + 1 - n..=j].iter().sum::<f64>() / n as f64 }).collect() }
    };
    Some((days, s))
}

/// Lane K78: TT/TI, E and D rules on a breadth input, computed on the breadth days (D: those with an SPX close too); the
/// state is carried over the other days.
fn output_br(id: &str, x: &Ctx, g: &dyn Fn(&str) -> f64) -> Vec<f64> {
    let mut out = vec![0.; x.c.len()];
    let Some((mut days, mut s)) = breadth_input(x, (g("input") as usize).clamp(24, 29)) else { return out };
    // R4 #288 (2): the divergence's S&P 500 = the SPX close on the same date (FOLLOWER_CLOSE, exact join); p.ref = 1: own close
    let spx = if id == "arebta_d_br" && g("ref") < 0.5 { match x.series("FOLLOWER_CLOSE") { Some(v) => Some(v), None => return out } } else { None };
    if let Some(v) = spx {
        let keep: Vec<usize> = (0..days.len()).filter(|&k| v[days[k]].is_finite()).collect();
        s = keep.iter().map(|&k| s[k]).collect();
        days = keep.iter().map(|&k| days[k]).collect();
    }
    let m = days.len();
    let mut sig = vec![0.; m];
    let mut st = 0.;
    if id == "arebta_tt_br" || id == "arebta_ti_br" {
        let n = g("n").max(1.) as usize;
        let inv = if id == "arebta_ti_br" { -1. } else { 1. };
        for k in 0..m {
            if k >= n && s[k - n..=k].iter().all(|v| v.is_finite()) {
                let w = &s[k - n..k];
                if s[k] > w.iter().copied().fold(f64::MIN, f64::max) { st = inv; } else if s[k] < w.iter().copied().fold(f64::MAX, f64::min) { st = -inv; }
            }
            sig[k] = st;
        }
    } else {
        let (ty, disp, span) = (g("type") as usize, g("disp"), g("span") as usize);
        let ind = if id == "arebta_e_br" {
            let lma: Vec<f64> = (0..m).map(|k| if k < 3 { f64::NAN } else { (4. * s[k] + 3. * s[k - 1] + 2. * s[k - 2] + s[k - 3]) / 10. }).collect();
            cn(&lma, span)
        } else {
            let px: Vec<f64> = days.iter().map(|&i| spx.map_or(x.c[i], |v| v[i])).collect();
            let (a, b) = (cn(&s, span), cn(&px, span));
            cn(&a.iter().zip(&b).map(|(p, q)| p - q).collect::<Vec<_>>(), 10 * span)
        };
        let (le, se) = T83[ty.clamp(1, 12) - 1];
        let (lo, up) = (50. - disp, 50. + disp);
        let ev = |e: u8, a: f64, b: f64| match e { 1 => a >= lo && b < lo, 2 => a <= lo && b > lo, 3 => a <= up && b > up, _ => a >= up && b < up };
        for k in 1..m {
            let (a, b) = (ind[k - 1], ind[k]);
            if a.is_finite() && b.is_finite() {
                if ev(le, a, b) { st = 1.; } else if ev(se, a, b) { st = -1.; }
            }
            sig[k] = st;
        }
    }
    let mut cur = 0.;
    let mut k = 0;
    for i in 0..out.len() {
        if k < m && days[k] == i { cur = sig[k]; k += 1; }
        out[i] = cur;
    }
    out
}

/// Rule output +1/-1 (0 = not yet defined) at each close.
fn output(id: &str, x: &Ctx, g: &dyn Fn(&str) -> f64) -> Vec<f64> {
    if breadth(id) { return output_br(id, x, g); }
    let c = &x.c;
    let mut out = vec![0.; c.len()];
    let mut s = 0.;
    if id == "arebta_e" {
        let (ty, disp, span) = (g("type") as usize, g("disp"), g("span") as usize);
        let (le, se) = T83[ty.clamp(1, 12) - 1];
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

    /// Lane K78: a context with NYSE breadth columns; `ratio` sets advances/declines so ADR = ratio (unchanged = 0).
    fn br_ctx(start: &str, ratio: &[f64], close: &[f64]) -> (Vec<Row>, Vec<Vec<f64>>) {
        let y: i32 = start[..4].parse().unwrap();
        let r: Vec<Row> = close.iter().enumerate().map(|(i, &c)| Row { ticker: "SPY".into(), date: format!("{:04}-{:02}-{:02}", y + (i / 336) as i32, 1 + (i % 336) / 28, 1 + i % 28), open: c + 0.5, high: c + 1., low: c - 1., close: c, volume: None }).collect();
        let cols = vec![ratio.iter().map(|q| if q.is_finite() { 1000. * (1. + q) } else { f64::NAN }).collect(), ratio.iter().map(|q| 1000. * (1. - q)).collect(), vec![0.; ratio.len()],
                        ratio.iter().map(|q| 1000. * (1. + q / 2.)).collect(), ratio.iter().map(|q| 1000. * (1. - q / 2.)).collect(), vec![0.; ratio.len()]];
        (r, cols)
    }
    fn with_br<'a>(r: &'a [Row], cols: &[Vec<f64>]) -> Ctx<'a> {
        let mut x = Ctx::new(r, r, &[]);
        for (k, c) in BREADTH.iter().zip(cols) { x.extra.insert(k.to_string(), c.clone()); }
        x
    }
    fn prm(v: &[(&str, f64)]) -> Params { v.iter().map(|(k, x)| (k.to_string(), *x)).collect() }

    #[test]
    fn tt_ti_br_cadr_breakout_skips_missing_breadth_days() {
        // CADR (input 24) rises 3 days, falls 3 days (day 5 has no breadth print: skipped, the state is unchanged), rises again
        let q = [0.1, 0.1, 0.1, 0.1, -0.1, f64::NAN, -0.1, -0.1, 0.1, 0.1, 0.1, 0.1];
        let (r, cols) = br_ctx("1995", &q, &[100.; 12]);
        let x = with_br(&r, &cols);
        let t = run("arebta_tt_br", &x, 0, 11, &prm(&[("input", 24.), ("n", 3.)])).unwrap();
        // breadth days 0..4,6..: CADR .1 .2 .3 .4 .3 | .2 .1 .2 .3 .4 .5; day 3 > max(0..2) long (open 4); day 6: .2 < min(.4,.3,... ) -> the 3 breadth days before are 2,3,4 (.3,.4,.3): short (open 7); day 9 .3 > max(.2,.1,.2): long (open 10)
        assert_eq!(t.iter().map(|z| (z.dir, z.legs[0].i)).collect::<Vec<_>>(), vec![(1., 4), (-1., 7), (1., 10)]);
        assert_eq!(run("arebta_ti_br", &x, 0, 11, &prm(&[("input", 24.), ("n", 3.)])).unwrap()[0].dir, -1.);
        // the volume ratio (input 27) carries the same signs here
        assert_eq!(run("arebta_tt_br", &x, 0, 11, &prm(&[("input", 27.), ("n", 3.)])).unwrap().len(), 3);
        // no 2005 cut (R4 #288 (1)): the same breadth in 2006 trades the same way
        let (r2, c2) = br_ctx("2006", &q, &[100.; 12]);
        assert_eq!(run("arebta_tt_br", &with_br(&r2, &c2), 0, 11, &prm(&[("input", 24.), ("n", 3.)])).unwrap().len(), 3);
        // no breadth series (India) -> no trades
        assert!(run("arebta_tt_br", &Ctx::new(&r, &r, &[]), 0, 11, &Params::new()).unwrap().is_empty());
    }

    #[test]
    fn e_br_type2_long_on_lower_down_cross_short_on_upper_up_cross() {
        // CADR up 25 days, down 25, up 25: CN(LMA(CADR,4),15) falls through 30 (event 1: long) then rises through 70 (event 3: short)
        let q: Vec<f64> = (0..75).map(|i| if (25..50).contains(&i) { -0.1 } else { 0.1 }).collect();
        let (r, cols) = br_ctx("1990", &q, &[100.; 75]);
        let x = with_br(&r, &cols);
        let t = run("arebta_e_br", &x, 0, 74, &prm(&[("input", 24.), ("type", 2.), ("disp", 20.), ("span", 15.)])).unwrap();
        assert_eq!(t.iter().map(|z| z.dir).collect::<Vec<_>>(), vec![1., -1.]);
        assert!((26..35).contains(&t[0].legs[0].i) && (51..60).contains(&t[1].legs[0].i), "{:?}", t.iter().map(|z| z.legs[0].i).collect::<Vec<_>>());
    }

    #[test]
    fn d_br_double_channel_normalisation() {
        // a companion (CADR) that lags the price by 3 days diverges from it; nothing can fire before n + 10n = 165 breadth days
        let px: Vec<f64> = (0..400).map(|i| 100. + 10. * ((i as f64) / 9.).sin()).collect();
        let lag: Vec<f64> = (0..400).map(|i| if i < 4 { 0.01 } else { (px[i - 3] - px[i - 4]) / 100. }).collect();
        let (r, cols) = br_ctx("1990", &lag, &px);
        let mut x = with_br(&r, &cols);
        // default reads the SPX close (R4 #288 (2)): none loaded -> no trades; p.ref = 1 uses the own close
        assert!(run("arebta_d_br", &x, 0, 399, &prm(&[("input", 24.), ("span", 15.), ("type", 1.)])).unwrap().is_empty());
        let own = run("arebta_d_br", &x, 0, 399, &prm(&[("input", 24.), ("span", 15.), ("type", 1.), ("disp", 10.), ("ref", 1.)])).unwrap();
        x.extra.insert("FOLLOWER_CLOSE".into(), px.clone());
        let t = run("arebta_d_br", &x, 0, 399, &prm(&[("input", 24.), ("span", 15.), ("type", 1.), ("disp", 10.)])).unwrap();
        assert_eq!(own.iter().map(|z| (z.dir, z.legs[0].i)).collect::<Vec<_>>(), t.iter().map(|z| (z.dir, z.legs[0].i)).collect::<Vec<_>>());
        assert!(t.len() >= 2 && t.windows(2).all(|w| w[0].dir == -w[1].dir) && t[0].legs[0].i > 164);
        // Table 8.3/8.4: type 7 swaps type 1's events, so it reverses at the same bars in the opposite direction
        let u = run("arebta_d_br", &x, 0, 399, &prm(&[("input", 24.), ("span", 15.), ("type", 7.), ("disp", 10.)])).unwrap();
        assert_eq!(t.iter().map(|z| (-z.dir, z.legs[0].i)).collect::<Vec<_>>(), u.iter().map(|z| (z.dir, z.legs[0].i)).collect::<Vec<_>>());
    }
}

// Instrument admission: David Aronson 2026-10-25 block, moved from instrument_policy.rs on 31 Oct 2026 (lane ST).
// Admission only for the module REVIEWED lists (authors/aronson/admission/SOURCE-CONTRACT.md); kept in one block to merge.
// Runs on actual indices, ETFs and stocks, US and India (index_etf_or_stock, nifty50 basket, declared application).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book=="arebta" && crate::books::arebta::reviewed(def.id),
    label: "Aronson data-mined these rules on the S&P 500 (1980-2005, detrended returns); runs on other indices, ETFs and single stocks (US and India) on their own closes are a declared application, one unit per trade, next-open fills, no detrending.",
    basis: "David Aronson, Evidence-Based Technical Analysis (Wiley 2007), ch.8, p.397-449",
};
