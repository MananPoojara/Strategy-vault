//! John Bollinger, Bollinger on Bollinger Bands (McGraw-Hill 2002; 265-page PDF) and the TradeStation
//! "CODE.DOC" of its indicators (DELTA 2003). Page refs are the book's printed pages (PDF page = printed + 30).
//! Only rules the book states exactly are Live; defaults are the book's values; readings where the book is
//! silent are labelled in `notes`. Sources, testability table and the independent oracle: library/bobb/.
use crate::connors::{pct_b, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "bobb";
/// Admitted ids (authors/bollinger/admission/SOURCE-CONTRACT.md).
pub const REVIEWED: &[&str] = &["bobb_m2_band", "bobb_m2_sar", "bobb_m3_long"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, dir, status, vol, rules, notes| StrategyDef {
        id, name, group, session: 0, dir, status, needs_volume: vol, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None,
    };
    use Status::*;
    const M2: &str = "Bands: SMA(20) ± 2 population standard deviations (\"the basic Bollinger Band settings of 20 periods and +2 standard deviations\", p.157; StdDev type 1 in CODE.DOC); %b = (Last - lower BB)/(upper BB - lower BB) (Table 8.1, p.60). MFI(10) = 100 - 100/(1 + positive typical-price x volume sum / negative sum) (Table 18.3, p.148), length \"approximately half the length of the calculation period for the bands\" (p.157). Signal on the close, entry at the next open (book silent: TradeStation 'next bar at market'). One position at a time; an opposite signal while in a trade is ignored (book silent). Stock method; run on the index (instrument swap); needs volume.";
    vec![
        d("bobb_m2_band", "Method II · trend following, exit on a tag of the opposite band", "Ch 19 · Method II", "Long/Short", Live, true, &[
            "E|\"if %b is greater than 0.8 and MFI(10) is greater than 80, then buy\" (p.155); \"Buy when %b is greater than 0.8 and MFI is greater than 80\" (p.159)",
            "E|\"Sell when %b is less than 0.2 and MFI is less than 20\" (p.159)",
            "X|\"We'll use the same exit techniques, a modified version of Parabolic or a tag of the Bollinger Band on the opposite side of the trade\" (p.155); \"in a buy use a tag of the lower band as an exit, and in a sell use a tag of the upper band as an exit\" (p.127)",
        ], M2),
        d("bobb_m2_sar", "Method II · trend following, Parabolic stop", "Ch 19 · Method II", "Long/Short", Live, true, &[
            "E|Same entries as bobb_m2_band (%b > 0.8 and MFI(10) > 80 buy; %b < 0.2 and MFI(10) < 20 sell) (p.155, p.159)",
            "X|\"Use a Parabolic stop.\" (p.159); Parabolic started \"under the entry day as is common\" (p.158); \"Welles Wilder's Parabolic ... the initial stop is set just below the range ... and then incremented upward each day the trade is open\" (p.127)",
            "F|Parabolic = Wilder's: SAR(tomorrow) = SAR + AF x (EP - SAR), AF 0.02 rising 0.02 per new extreme to 0.20, SAR not inside the last two bars (Wilder, New Concepts, Sec II)",
        ], "Stop only, not a reversal. Book: 'a modified version of Parabolic' (p.155) whose modification is not given, so Wilder's standard Parabolic is used, started at the entry day's low (long) / high (short) and first applied the next day; a gap through it fills at the open. Same entry readings as bobb_m2_band."),
        d("bobb_m3_long", "Method III · reversal buy: lower-band tag with positive II%", "Ch 20 · Method III", "Long", Live, true, &[
            "E|\"go long if %b is less than 0.05 and II% is greater than zero\" (p.165), II% = 21-day Intraday Intensity normalised: sum of (2C - H - L)/(H - L) x volume / sum of volume (Tables 18.3-18.4, p.148-151; \"21-day II%\", p.165)",
            "X|\"A tag of the upper band, the logical target after such a setup\" (p.165)",
        ], "Bands SMA(20) ± 2 SD (defaults, p.157). Entry at the next open (book silent). Exit when the day's high reaches the upper band, at that close (book silent on the fill; the band is only known at the close). No stop in the book. An H = L day adds 0 to II (book silent; CODE.DOC carries the previous value). Run on the index."),
        d("bobb_m3_short", "Method III · reversal sell: upper-band tag with negative AD%", "Ch 20 · Method III", "Short", NotTestable("The book gives the sell setup (\"go short if %b is greater than 0.95 and AD% is less than zero\", p.165) but no exit for it; the 'logical target' is stated only for the buy."), true, &[
            "E|\"go short if %b is greater than 0.95 and AD% is less than zero\" (p.165), AD% = 20-day Accumulation Distribution normalised (p.151, p.165)",
        ], "Not testable: no exit (user rule)."),
        d("bobb_m1_squeeze", "Method I · volatility breakout after The Squeeze", "Ch 15-16 · Method I", "Long/Short", NotTestable("Setup 'BandWidth at a six-month low' is exact, but the book never says how long after a Squeeze a breakout still counts (\"Just wait for a Squeeze and go with the first breakout\"), nor whether 'exceeded' means a close or an intraday trade beyond the band, and it calls the selection a visual skill (\"instructs the eye ... as no hard-and-fast rules ever can\", p.129)."), false, &[
            "F|\"BandWidth is at a six-month low\" (p.67, p.129); BandWidth = (upper BB - lower BB)/middle BB (Table 8.2, p.63)",
            "E|\"we'll actually buy when the upper band is exceeded and short when the lower band is broken to the downside\" (p.126)",
            "X|Parabolic started \"just below the range of the breakout formation\" or \"a tag of the opposite band\" (p.127)",
        ], "Not testable: setup-to-breakout window undefined; head-fake handling is discretionary (p.128-129)."),
        d("bobb_ch8_examples", "Ch 8 · example system statements (%b with II / %b of an indicator)", "Ch 8 · %b and BandWidth", "Long/Short", NotTestable("Illustrations of how to program a rule, with no exit: \"If %b is greater than one and 21-day II is less than zero, sell\" (p.61); \"If %b(price) is greater than 0.9 and %b(indicator) is less than 0.3, sell\" (p.63); the W-bottom statement needs a judged 'first low' (p.61)."), true, &[
            "E|\"If %b is greater than one and 21-day II is less than zero, sell\" (p.61)",
        ], "Not testable: no exit."),
        d("bobb_ch20_breadth", "Ch 20 · DJIA 4% bands / Bollinger Bands with A-D and up-down volume MACD(21,100)", "Ch 20 · Method III", "Long/Short", NotTestable("Needs NYSE advances-declines and up/down volume, and gives tags 'accompanied by' oscillator readings as signals with no exit (p.161-162)."), false, &[
            "E|\"Tags of the upper band accompanied by negative oscillator readings ... were taken as sell signals. Buy signals were generated by tags of the lower band accompanied by positive oscillator readings\" (p.161)",
        ], "Not testable: breadth data and no exit."),
        d("bobb_patterns", "W bottoms, M tops, walking the bands, head fakes", "Ch 10-14 · Pattern recognition", "Long/Short", NotTestable("Chart-pattern reading (W2/W4, M16, three pushes, 'buy the first strong up day') is judgement (p.77-118, p.162-164)."), false, &[
            "E|\"if we form a W2 bottom with %b higher on the retest ... check your volume oscillator ... If it does then buy the first strong up day\" (p.162)",
        ], "Not testable: discretionary."),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "bobb_m2_band" => ("bobb_m2", "Bollinger Method II", "Opposite-band exit"),
        "bobb_m2_sar" => ("bobb_m2", "Bollinger Method II", "Parabolic stop"),
        "bobb_m3_long" => ("bobb_m3", "Bollinger Method III", "Buy · upper-band target"),
        "bobb_m3_short" => ("bobb_m3", "Bollinger Method III", "Sell · no exit"),
        "bobb_m1_squeeze" => ("bobb_m1", "Bollinger Method I", "Not testable"),
        "bobb_ch8_examples" | "bobb_ch20_breadth" | "bobb_patterns" => ("bobb_other", "Bollinger examples", "Not testable"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let bands = || vec![p("n", "Band days", 20., 10., 50., 1.), p("k", "Band width (SD)", 2., 1.5, 2.5, 0.1)];
    Some(match id {
        "bobb_m2_band" | "bobb_m2_sar" => {
            let mut v = bands();
            v.extend([p("mfi_n", "MFI days", 10., 5., 25., 1.), p("pb_hi", "Buy: %b above", 0.8, 0.5, 1.2, 0.05), p("mfi_hi", "Buy: MFI above", 80., 50., 95., 5.),
                p("pb_lo", "Sell: %b below", 0.2, -0.2, 0.5, 0.05), p("mfi_lo", "Sell: MFI below", 20., 5., 50., 5.)]);
            if id == "bobb_m2_sar" { v.extend([p("af0", "Parabolic start AF", 0.02, 0.01, 0.1, 0.01), p("afstep", "AF step", 0.02, 0.01, 0.1, 0.01), p("afmax", "AF max", 0.2, 0.1, 0.5, 0.05)]); }
            v
        }
        "bobb_m3_long" => { let mut v = bands(); v.extend([p("pb", "%b below", 0.05, -0.2, 0.3, 0.05), p("ii_n", "II% days", 21., 5., 40., 1.)]); v }
        _ => return None,
    })
}

fn bands(c: &[f64], n: usize, k: f64) -> (Vec<f64>, Vec<f64>) {
    let (mut up, mut lo) = (vec![f64::NAN; c.len()], vec![f64::NAN; c.len()]);
    for i in n.saturating_sub(1)..c.len() {
        let w = &c[i + 1 - n..=i];
        let m = w.iter().sum::<f64>() / n as f64;
        let sd = (w.iter().map(|v| (v - m).powi(2)).sum::<f64>() / n as f64).sqrt();
        up[i] = m + k * sd;
        lo[i] = m - k * sd;
    }
    (up, lo)
}

/// MFI per Table 18.3: 100 - 100/(1 + pos/neg) over n days of typical price x volume (NaN without volume).
pub fn mfi(x: &Ctx, n: usize) -> Vec<f64> {
    let len = x.c.len();
    let tp: Vec<f64> = (0..len).map(|i| (x.h[i] + x.l[i] + x.c[i]) / 3.).collect();
    let v: Vec<f64> = x.r.iter().map(|r| r.volume.unwrap_or(f64::NAN)).collect();
    let mut out = vec![f64::NAN; len];
    for i in n..len {
        let (mut pos, mut neg) = (0., 0.);
        for j in i + 1 - n..=i {
            let mf = tp[j] * v[j];
            if tp[j] > tp[j - 1] { pos += mf } else if tp[j] < tp[j - 1] { neg += mf }
        }
        if pos.is_nan() || neg.is_nan() || (0..n).any(|d| v[i - d].is_nan()) { continue; }
        out[i] = if neg == 0. { 100. } else { 100. - 100. / (1. + pos / neg) };
    }
    out
}

/// II% = 100 x sum((2C - H - L)/(H - L) x V) / sum(V) over n days (Tables 18.3-18.4).
pub fn ii_pct(x: &Ctx, n: usize) -> Vec<f64> {
    let len = x.c.len();
    let v: Vec<f64> = x.r.iter().map(|r| r.volume.unwrap_or(f64::NAN)).collect();
    let ii: Vec<f64> = (0..len).map(|i| { let r = x.h[i] - x.l[i]; if r > 0. { (2. * x.c[i] - x.h[i] - x.l[i]) / r * v[i] } else { 0. * v[i] } }).collect();
    let mut out = vec![f64::NAN; len];
    for i in n.saturating_sub(1)..len {
        let (a, b): (f64, f64) = (ii[i + 1 - n..=i].iter().sum(), v[i + 1 - n..=i].iter().sum());
        if a.is_finite() && b > 0. { out[i] = 100. * a / b; }
    }
    out
}

fn open_t(dir: f64, i: usize, px: f64, sig: usize) -> Trade {
    Trade { dir, legs: vec![Leg { i, px, w: 1. }], exit_i: i, exit_px: px, reason: String::new(), entry_kind: "open", signal_i: sig, open: false, exits: vec![] }
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if from > to || to >= x.r.len() { return None; }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let u = |k: &str| g(k).round().max(2.) as usize;
    let (n, k) = (u("n"), g("k"));
    let pb = pct_b(&x.c, n, k);
    let (up, lo) = bands(&x.c, n, k);
    // signal at close i -> direction
    let sig: Box<dyn Fn(usize) -> f64> = match id {
        "bobb_m2_band" | "bobb_m2_sar" => {
            let m = mfi(x, u("mfi_n"));
            let (ph, mh, pl, ml) = (g("pb_hi"), g("mfi_hi"), g("pb_lo"), g("mfi_lo"));
            Box::new(move |i| if pb[i] > ph && m[i] > mh { 1. } else if pb[i] < pl && m[i] < ml { -1. } else { 0. })
        }
        "bobb_m3_long" => {
            let ii = ii_pct(x, u("ii_n"));
            let t = g("pb");
            Box::new(move |i| if pb[i] < t && ii[i] > 0. { 1. } else { 0. })
        }
        _ => return None,
    };
    let sar_mode = id == "bobb_m2_sar";
    let (a0, astep, amax) = (g("af0"), g("afstep"), g("afmax"));
    let (mut out, mut cur): (Vec<Trade>, Option<Trade>) = (vec![], None);
    let (mut sar, mut ep, mut af) = (0., 0., 0.);
    for i in from..=to {
        if let Some(t) = cur.as_mut() {
            let (e, d) = (t.legs[0].i, t.dir);
            let mut exit: Option<(f64, &str)> = None;
            if sar_mode {
                if i > e {
                    if (d > 0. && x.l[i] <= sar) || (d < 0. && x.h[i] >= sar) {
                        exit = Some((if d > 0. { x.o[i].min(sar) } else { x.o[i].max(sar) }, "Parabolic stop"));
                    }
                }
                if exit.is_none() {
                    // advance the Parabolic for tomorrow (Wilder): new extreme -> AF step; SAR not inside the last two bars
                    if i == e { sar = if d > 0. { x.l[i] } else { x.h[i] }; ep = if d > 0. { x.h[i] } else { x.l[i] }; af = a0; } else {
                        if (d > 0. && x.h[i] > ep) || (d < 0. && x.l[i] < ep) { ep = if d > 0. { x.h[i] } else { x.l[i] }; af = (af + astep).min(amax); }
                        sar += af * (ep - sar);
                        let (l2, h2) = (x.l[i].min(x.l[i - 1]), x.h[i].max(x.h[i - 1]));
                        sar = if d > 0. { sar.min(l2) } else { sar.max(h2) };
                    }
                }
            } else if id == "bobb_m3_long" {
                if x.h[i] >= up[i] { exit = Some((x.c[i], "Tag of the upper band")); }
            } else if (d > 0. && x.l[i] <= lo[i]) || (d < 0. && x.h[i] >= up[i]) {
                exit = Some((x.c[i], if d > 0. { "Tag of the lower band" } else { "Tag of the upper band" }));
            }
            if let Some((px, why)) = exit {
                let mut t = cur.take().unwrap();
                t.exit_i = i; t.exit_px = px; t.signal_i = i; t.reason = why.into();
                out.push(t);
            }
        }
        if cur.is_none() && i < to {
            let d = sig(i);
            if d != 0. {
                cur = Some(open_t(d, i + 1, x.o[i + 1], i));
            }
        }
    }
    if let Some(mut t) = cur { t.exit_i = to; t.exit_px = x.c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true; out.push(t); }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn mfi_and_ii_match_hand_values() {
        let b = [[10., 11., 9., 10.5, 100.], [10.5, 12., 10., 11.5, 200.], [11.5, 11.8, 10.2, 10.4, 150.], [10.4, 11., 10., 10.9, 120.]];
        let r: Vec<Row> = b.iter().enumerate().map(|(i, v)| Row { ticker: "T".into(), date: format!("2000-01-0{}", i + 1), open: v[0], high: v[1], low: v[2], close: v[3], volume: Some(v[4]) }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let m = mfi(&x, 2);
        // day 3: tp2 = 11.1667 > tp1 10.1667 (pos 11.1667*200), tp3 = 10.8 < tp2 (neg 10.8*150)
        let want = 100. - 100. / (1. + (11.0 + 1. / 6.) * 200. / (10.8 * 150.));
        assert!((m[2] - want).abs() < 1e-9);
        let ii = ii_pct(&x, 2);
        let a = (2. * 11.5 - 12. - 10.) / 2. * 200. + (2. * 10.4 - 11.8 - 10.2) / 1.6 * 150.;
        assert!((ii[2] - 100. * a / 350.).abs() < 1e-9);
    }
}
