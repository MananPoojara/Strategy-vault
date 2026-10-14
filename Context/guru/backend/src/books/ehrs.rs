//! John F. Ehlers, *Rocket Science for Traders* (Wiley, 2001). Source + checks: library/ehlers/.
//! This module implements the SineTrend Automatic System (ch.12, Fig.12.1/12.4).
//! The intraday ZeroLag system (ch.16) remains held for actual intraday bars.
//! EasyLanguage is reproduced bar-for-bar with TradeStation conventions:
//! MaxBarsBack is explicit (research default 50; not printed in the reviewed source).
//! Code first runs at index MaxBarsBack, CurrentBar = 1 there; series variables are 0
//! before that), ArcTangent/Sine/Cosine in degrees, `Buy`/`Sell` = TS2000i stop-and-reverse
//! orders filled "next bar at market" (the next open), `Crosses Over/Under` per the TS
//! definition (skips bars where the two values are equal). Also used by ehmd (MAMA).
use crate::connors::{claim, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "ehrs";
/// Declared research MaxBarsBack, not a recovered author platform setting.
pub const MBB: usize = 50;

pub fn atan_d(x: f64) -> f64 { x.atan().to_degrees() }
fn sin_d(x: f64) -> f64 { x.to_radians().sin() }
fn cos_d(x: f64) -> f64 { x.to_radians().cos() }

/// TradeStation `a Crosses Over b` on bar i: a > b now and, at the most recent earlier bar
/// where they differ, a < b.
pub fn cross_over(a: &[f64], b: &[f64], i: usize) -> bool {
    if a[i] <= b[i] { return false; }
    let mut k = i;
    while k > 0 { k -= 1; if a[k] != b[k] { return a[k] < b[k]; } }
    false
}
pub fn cross_under(a: &[f64], b: &[f64], i: usize) -> bool {
    if a[i] >= b[i] { return false; }
    let mut k = i;
    while k > 0 { k -= 1; if a[k] != b[k] { return a[k] > b[k]; } }
    false
}

/// Hilbert transform homodyne discriminator, identical in Figs 9.1, 10.1, 11.1, 12.1, 17.1.
pub struct Homodyne { pub price: Vec<f64>, pub smooth: Vec<f64>, pub i1: Vec<f64>, pub q1: Vec<f64>, pub smooth_period: Vec<f64> }

pub fn homodyne(x: &Ctx) -> Homodyne {
    homodyne_with_mbb(x, MBB)
}

pub fn homodyne_with_mbb(x: &Ctx, mbb: usize) -> Homodyne {
    assert!(mbb >= 6);
    let n = x.c.len();
    let price: Vec<f64> = (0..n).map(|i| (x.h[i] + x.l[i]) / 2.).collect();
    let z = || vec![0f64; n];
    let (mut sm, mut de, mut i1, mut q1, mut ji, mut jq) = (z(), z(), z(), z(), z(), z());
    let (mut i2, mut q2, mut re, mut im, mut per, mut sp) = (z(), z(), z(), z(), z(), z());
    let fir = |a: &[f64], i: usize| 0.0962 * a[i] + 0.5769 * a[i - 2] - 0.5769 * a[i - 4] - 0.0962 * a[i - 6];
    for i in mbb..n {
        per[i] = per[i - 1]; // series variables keep their value until assigned
        if i - mbb + 1 <= 5 { continue; } // If CurrentBar > 5 then begin
        let p = &price;
        sm[i] = (4. * p[i] + 3. * p[i - 1] + 2. * p[i - 2] + p[i - 3]) / 10.;
        let g = 0.075 * per[i - 1] + 0.54;
        de[i] = fir(&sm, i) * g;
        q1[i] = fir(&de, i) * g;
        i1[i] = de[i - 3];
        ji[i] = fir(&i1, i) * g;
        jq[i] = fir(&q1, i) * g;
        i2[i] = i1[i] - jq[i];
        q2[i] = q1[i] + ji[i];
        i2[i] = 0.2 * i2[i] + 0.8 * i2[i - 1];
        q2[i] = 0.2 * q2[i] + 0.8 * q2[i - 1];
        re[i] = i2[i] * i2[i - 1] + q2[i] * q2[i - 1];
        im[i] = i2[i] * q2[i - 1] - q2[i] * i2[i - 1];
        re[i] = 0.2 * re[i] + 0.8 * re[i - 1];
        im[i] = 0.2 * im[i] + 0.8 * im[i - 1];
        let p1 = per[i - 1];
        let mut pd = per[i];
        if im[i] != 0. && re[i] != 0. { pd = 360. / atan_d(im[i] / re[i]); }
        if pd > 1.5 * p1 { pd = 1.5 * p1; }
        if pd < 0.67 * p1 { pd = 0.67 * p1; }
        if pd < 6. { pd = 6.; }
        if pd > 50. { pd = 50.; }
        per[i] = 0.2 * pd + 0.8 * p1;
        sp[i] = 0.33 * per[i] + 0.67 * sp[i - 1];
    }
    Homodyne { price, smooth: sm, i1, q1, smooth_period: sp }
}

/// Position target set on bar i's close, filled at the next open: 1 long, -1 short, 0 flat.
/// `sig(i, pos)` returns the target (None = no order). Flat at `from`; last position marked open.
pub fn next_open_sim(x: &Ctx, from: usize, to: usize, mut sig: impl FnMut(usize, i8) -> Option<(i8, &'static str)>) -> Vec<Trade> {
    let (mut out, mut cur, mut pos): (Vec<Trade>, Option<Trade>, i8) = (vec![], None, 0);
    for i in from..to {
        let Some((target, why)) = sig(i, pos) else { continue };
        if target == pos { continue; }
        if let Some(mut t) = cur.take() {
            t.exit_i = i + 1; t.exit_px = x.o[i + 1]; t.signal_i = i; t.reason = format!("{why} · next open");
            out.push(t);
        }
        if target != 0 {
            cur = Some(Trade { dir: target as f64, legs: vec![Leg { i: i + 1, px: x.o[i + 1], w: 1. }], exit_i: i + 1, exit_px: x.o[i + 1],
                reason: String::new(), entry_kind: "open", signal_i: i, open: false, exits: vec![] });
        }
        pos = target;
    }
    if let Some(mut t) = cur { t.exit_i = to; t.exit_px = x.c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true; out.push(t); }
    out
}

/// SineTrend indicator state per bar (Fig. 12.1 with the Fig. 12.4 CycPart trendline).
pub struct SineTrend { pub smooth: Vec<f64>, pub trendline: Vec<f64>, pub sine: Vec<f64>, pub lead: Vec<f64>, pub trend: Vec<i8> }

pub fn sinetrend(x: &Ctx, cyc_part: f64) -> SineTrend {
    // Preserve the authoritative implementation for unapproved legacy IDs.
    sinetrend_state(x, cyc_part, MBB, false)
}

pub fn sinetrend_with_mbb(x: &Ctx, cyc_part: f64, mbb: usize) -> SineTrend {
    sinetrend_state(x, cyc_part, mbb, true)
}

fn sinetrend_state(x: &Ctx, cyc_part: f64, mbb: usize, reviewed: bool) -> SineTrend {
    let hd = homodyne_with_mbb(x, mbb);
    let n = x.c.len();
    let (p, sm, sp) = (&hd.price, &hd.smooth, &hd.smooth_period);
    let mut dcphase = vec![0f64; n];
    let mut itrend = vec![0f64; n];
    let mut tl = vec![0f64; n];
    let mut trend = vec![0i8; n];
    let mut days = 0f64;
    // DCSine and LeadSine are Vars(0), assigned only inside CurrentBar > 5.
    // The DaysInTrend expression instead crosses Sine(DCPhase) and Sine(DCPhase+45)
    // directly; its historical values before the gate are 0 and sin(45).
    let (mut sine, mut lead) = (vec![0f64; n], vec![if reviewed { 0. } else { sin_d(45.) }; n]);
    let (mut phase_sine, mut phase_lead) = (vec![0f64; n], vec![sin_d(45.); n]);
    for i in mbb..n {
        let cb = i - mbb + 1;
        dcphase[i] = dcphase[i - 1];
        itrend[i] = itrend[i - 1];
        tl[i] = tl[i - 1];
        if cb > 5 {
            let dcp = (sp[i] + 0.5).trunc() as usize;
            let (mut real, mut imag) = (0., 0.);
            for k in 0..dcp {
                let a = 360. * k as f64 / dcp as f64;
                real += cos_d(a) * sm[i - k];
                imag += sin_d(a) * sm[i - k];
            }
            let mut ph = dcphase[i];
            if real.abs() > 0. { ph = atan_d(imag / real); }
            if real.abs() <= 0.001 { ph = 90. * if imag > 0. { 1. } else if imag < 0. { -1. } else { 0. }; }
            ph += 90.;
            ph += 360. / sp[i];
            if imag < 0. { ph += 180.; }
            if ph > 315. { ph -= 360.; }
            dcphase[i] = ph;
            let ip = (cyc_part * sp[i] + 0.5).trunc() as usize;
            let mut it = 0.;
            for k in 0..ip { it += if k <= i { p[i - k] } else { 0. }; }
            if dcp > 0 { it /= ip as f64; }
            itrend[i] = it;
            tl[i] = (4. * itrend[i] + 3. * itrend[i - 1] + 2. * itrend[i - 2] + itrend[i - 3]) / 10.;
            if cb < 12 { tl[i] = p[i]; }
        }
        if reviewed && cb <= 5 { continue; }
        sine[i] = sin_d(dcphase[i]);
        lead[i] = sin_d(dcphase[i] + 45.);
        phase_sine[i] = sine[i];
        phase_lead[i] = lead[i];
        if cb <= 5 { continue; }
        let mut t: i8 = 1;
        if cross_over(&phase_sine, &phase_lead, i) || cross_under(&phase_sine, &phase_lead, i) { days = 0.; t = 0; }
        days += 1.;
        if days < 0.5 * sp[i] { t = 0; }
        let d = dcphase[i] - dcphase[i - 1];
        if sp[i] != 0. && d > 0.67 * 360. / sp[i] && d < 1.5 * 360. / sp[i] { t = 0; }
        if ((sm[i] - tl[i]) / tl[i]).abs() >= 0.015 { t = 1; }
        trend[i] = t;
    }
    SineTrend { smooth: hd.smooth, trendline: tl, sine, lead, trend }
}

const GROUP: &str = "Ch. 12 · SineTrend Automatic System";
const RULES: &[&str] = &[
    "F|Price = (H+L)/2. Dominant cycle SmoothPeriod from the Hilbert homodyne discriminator; DCPhase from a DFT of SmoothPrice over DCPeriod = IntPortion(SmoothPeriod + .5) bars (+90, +360/SmoothPeriod lag compensation, +180 if ImagPart < 0, -360 if > 315). Sine = Sine(DCPhase), LeadSine = Sine(DCPhase + 45). (Fig. 12.1, pp.120-123)",
    "F|Trendline = 4-bar WMA of the simple average of Price over IntPortion(CycPart*SmoothPeriod + .5) bars; Trendline = Price while CurrentBar < 12. (Fig. 12.1; CycPart from Fig. 12.4, p.126)",
    "F|Trend = 1 (assume Trend Mode); Trend = 0 on a Sine/LeadSine crossing (DaysInTrend reset to 0); DaysInTrend + 1; Trend = 0 if DaysInTrend < .5*SmoothPeriod; Trend = 0 if DCPhase - DCPhase[1] is between .67*360/SmoothPeriod and 1.5*360/SmoothPeriod; Trend = 1 if |SmoothPrice - Trendline|/Trendline >= .015. (p.122-123)",
    "E|Trend = 1 and Trend[1] = 0: if MarketPosition = -1 and SmoothPrice >= Trendline then buy; if MarketPosition = 1 and SmoothPrice < Trendline then sell. (p.123)",
    "E|Trend = 1: if SmoothPrice Crosses Over Trendline then buy; if SmoothPrice Crosses Under Trendline then sell. (p.123)",
    "E|Trend = 0 (Cycle Mode): if LeadSine Crosses Over DCSine then buy; if LeadSine Crosses Under DCSine then sell. (p.123)",
    "X|Always in the market after the first signal: buy reverses a short, sell reverses a long (TS2000i Buy/Sell). Orders fill next bar at market (open).",
];
const RULES_TREND: &[&str] = &[
    "F|Identical indicator code to the full SineTrend (Fig. 12.1).",
    "E|\"I therefore simply deleted the four lines of code that made the Cycle Mode trades\" (p.124): only the two Trend Mode blocks trade (trend-start reversal and SmoothPrice/Trendline crossings).",
    "X|Stop-and-reverse; positions are held through Cycle Mode periods. Orders fill next bar at market (open).",
];

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, status, rules, notes, claim| StrategyDef { id, name, group: GROUP, session: 0, dir: "Long/Short", status,
        needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim };
    vec![
        d("ehrs_sinetrend", "SineTrend Automatic System (Fig. 12.1)", Status::Live, RULES,
          "Source p.119 calls Fig.12.1 a complete theoretical automatic system before adding a $1,000 stop for the Treasury-bond study. This run uses that signal code on selected actual daily OHLC as a declared research application, without the separate dollar stop or futures accounting. CycPart=1. MaxBarsBack=50 is an explicit configurable research setting, not printed author metadata. Zero state persists before CurrentBar>5; next-open fills; flat at selected start; terminal positions stay open. Book figures are separate and are not reproduced. Review: library/ehlers/REVIEW-2026-10-13.md.",
          claim("233", "40.77%", "$398.61", "20 bars win / 5 bars loss", "T-Bonds 9 Jul 1984-16 Jun 2000, $1,000 stop")),
        d("ehrs_sinetrend_trend", "SineTrend · Trend Mode only (Fig. 12.3)", Status::Live, RULES_TREND,
          "Fig. 12.3 test (T-Bonds, $1,000 stop). Author calls it \"simply awful\" - kept to show the Cycle Mode contribution.",
          claim("81", "18.52%", "$246.53", "38 bars win / 8 bars loss", "T-Bonds 9 Jul 1984-16 Jun 2000, $1,000 stop")),
        d("ehrs_sinetrend_cp115", "SineTrend · CycPart 1.15 (Fig. 12.5)", Status::Live, RULES,
          "Fig. 12.4 trendline with the book's optimized CycPart = 1.15; book also raised the stop to $1,100 (T-Bonds).",
          claim("191", "44.5%", "$594.37", "24 bars win / 6 bars loss", "T-Bonds 9 Jul 1984-16 Jun 2000, $1,100 stop")),
        d("ehrs_sinetrend_cp115_trend", "SineTrend · Trend only, CycPart 1.15 (Fig. 12.6)", Status::Live, RULES_TREND,
          "Trend Mode-only code with CycPart = 1.15; book stop $1,500 (T-Bonds).",
          claim("67", "35.82%", "$642.72", "52 bars win / 13 bars loss", "T-Bonds 9 Jul 1984-16 Jun 2000, $1,500 stop")),
        d("ehrs_sinetrend_cp110", "SineTrend · CycPart 1.10 (Fig. 12.8)", Status::Live, RULES,
          "Book: CycPart optimized to 1.10, $2,200 stop, back-adjusted Swiss Franc futures 13 Feb 1975 - 1 Jun 2000 (the Fig. 12.8 caption says Deutschemark; text says Swiss Franc).",
          claim("460", "50.87%", "$302.64", "16 bars win / 7 bars loss", "CHF/DM futures 13 Feb 1975-1 Jun 2000, $2,200 stop")),
        d("ehrs_zerolag", "ZeroLag Intraday Trading System (Fig. 16.3)", Status::NeedsData("intraday S&P futures bars (day session, bar size not legible in our scan) - the system needs yesterday's high/low, the first and second bar of each day and a close at the end of the day"),
          &["F|ZeroLag = alpha*(Price + .5*(Price - Price[3])) + (1 - alpha)*ZeroLag[1], alpha = .33; Instantaneous Trendline as Fig. 10.1. (pp.169-175)",
            "F|Demand an outside day: Ht >= Yh or Lt <= Yl (today's running high/low vs yesterday's).",
            "E|At the end of the second bar of the day, flat: buy if ZeroLag > Trendline, Open > Open[1], High > High[1] and Close[1] > Low[1] + (High[1]-Low[1])/3; sell if ZeroLag < Trendline, Open < Open[1], Low < Low[1] and Close[1] < High[1] - (High[1]-Low[1])/3.",
            "E|Crossover rule: if ZeroLag Crosses Over Trendline and High < High[1] and Close > Low + (High-Low)/2 then buy; if ZeroLag Crosses Under Trendline and Low > Low[1] and Close > High - (High-Low)/2 then sell.",
            "X|Intraday trade closed at the end of the day (text p.170); money-management stop in dollars."],
          "Book: S&P futures intraday 1 Jan 1997 - 7 Jun 2000. Not coded: needs intraday bars (only daily index data exist).",
          claim("283", "50.18%", "$517.4x (last digit cut in scan)", "7 bars win / 7 bars loss", "S&P futures intraday 1 Jan 1997-7 Jun 2000")),
        d("ehrs_sinewave_cycle", "Sinewave Indicator signals (Ch. 9)", Status::NotTestable("chart reading: signals are taken only while the reader judges the market to be in a Cycle Mode (p.103); the mechanical version is SineTrend"),
          &["E|LeadSine crossing the Sine line gives advance cycle-turn signals (pp.95-105)."], "Indicator chapter; automated in ehrs_sinetrend.", None),
        d("ehrs_optimum_predictor", "Optimum Predictor / Predictive MAs (Ch. 20)", Status::NotTestable("indicator only: 'Buy and sell signals occur when the Predict and Smooth2 lines cross' but the book says it should be used with other rules to remove false signals and gives no position or exit rules (p.211)"),
          &["E|Predict crossing Smooth2 (Fig. 20.2); Predict crossing Trigger (Fig. 20.4)."], "No complete system stated.", None),
        d("ehrs_adaptive_osc", "Adaptive RSI / Stochastic / CCI (Ch. 22)", Status::NotTestable("no rule: 'wait for signals to cross the 30 percent and 70 percent marks ... I will leave it to you to decide' (p.229)"),
          &["F|Indicators computed over CycPart*dominant cycle (Figs 22.1-22.3)."], "Indicators only.", None),
        d("ehrs_snr_filter", "Signal-to-Noise guideline (Ch. 8)", Status::NotTestable("guideline, not a system: 'Cycle Mode trading should be avoided when the SNR is below 6 dB' (p.93); it is not part of the SineTrend code"),
          &["F|SNR = 10*Log((I1^2+Q1^2)/Range^2)/Log(10) + 6 dB (Fig. 8.2)."], "", None),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let f = ("ehrs_sinetrend", "Ehlers SineTrend");
    Some(match id {
        "ehrs_sinetrend" => (f.0, f.1, "Fig. 12.1 · trend + cycle"),
        "ehrs_sinetrend_trend" => (f.0, f.1, "Fig. 12.3 · trend only"),
        "ehrs_sinetrend_cp115" => (f.0, f.1, "Fig. 12.5 · CycPart 1.15"),
        "ehrs_sinetrend_cp115_trend" => (f.0, f.1, "Fig. 12.6 · trend only, CycPart 1.15"),
        "ehrs_sinetrend_cp110" => (f.0, f.1, "Fig. 12.8 · CycPart 1.10"),
        "ehrs_zerolag" => ("ehrs_zerolag", "Ehlers ZeroLag intraday", "Fig. 16.3"),
        "ehrs_sinewave_cycle" | "ehrs_optimum_predictor" | "ehrs_adaptive_osc" | "ehrs_snr_filter" => ("ehrs_indicators", "Ehlers RSFT indicators", "No system rules"),
        _ => return None,
    })
}

fn variant(id: &str) -> Option<(f64, bool)> {
    Some(match id {
        "ehrs_sinetrend" => (1.0, false),
        "ehrs_sinetrend_trend" => (1.0, true),
        "ehrs_sinetrend_cp115" => (1.15, false),
        "ehrs_sinetrend_cp115_trend" => (1.15, true),
        "ehrs_sinetrend_cp110" => (1.10, false),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let (cp, _) = variant(id)?;
    if id != "ehrs_sinetrend" {
        return Some(vec![ParamDef { key: "cyc_part", label: "CycPart (trendline length / dominant cycle)", def: cp, min: 0.5, max: 1.5, step: 0.05 }]);
    }
    Some(vec![ParamDef { key: "cyc_part", label: "CycPart (trendline length / dominant cycle)", def: cp, min: 1., max: 1.5, step: 0.05 },
        ParamDef { key: "max_bars_back", label: "MaxBarsBack (declared research setting)", def: MBB.max((cp * 50. + 0.5).trunc() as usize - 1) as f64, min: 50., max: 250., step: 1. }])
}

pub fn validate(id: &str, p: &Params) -> Option<String> {
    if id != "ehrs_sinetrend" { return None; }
    let schema = params(id)?;
    for (key, value) in p {
        let Some(d) = schema.iter().find(|d| d.key == key) else { return Some(format!("Unknown SineTrend parameter: {key}")); };
        if !value.is_finite() || !(d.min..=d.max).contains(value) { return Some(format!("Invalid SineTrend parameter: {key}")); }
    }
    let get = |k| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap().def);
    let mbb = get("max_bars_back");
    if mbb.fract() != 0. || mbb < (get("cyc_part") * 50. + 0.5).trunc() - 1. {
        return Some("MaxBarsBack must be an integer covering the maximum source Price[count] reference.".into());
    }
    None
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let (cp_def, trend_only) = variant(id)?;
    let cp = p.get("cyc_part").copied().unwrap_or(cp_def);
    if id != "ehrs_sinetrend" {
        if !(0.5..=1.5).contains(&cp) || p.keys().any(|k| k != "cyc_part") || from >= to || to >= x.c.len() { return None; }
        let st = sinetrend(x, cp);
        return Some(next_open_sim(x, from, to, |i, pos| sinetrend_order(&st, i, pos, trend_only, MBB)));
    }
    if validate(id, p).is_some() || from >= to || to >= x.c.len() { return None; }
    let mbb = p.get("max_bars_back").copied().unwrap_or(MBB.max((cp_def * 50. + 0.5).trunc() as usize - 1) as f64) as usize;
    let st = sinetrend_with_mbb(x, cp, mbb);
    Some(next_open_sim(x, from, to, |i, pos| sinetrend_order(&st, i, pos, trend_only, mbb)))
}

/// The same order predicate is exposed for auditing signals that lack a next bar.
pub fn sinetrend_order(st: &SineTrend, i: usize, pos: i8, trend_only: bool, mbb: usize) -> Option<(i8, &'static str)> {
        if i < mbb + 5 { return None; }
        let (sm, tl) = (&st.smooth, &st.trendline);
        let mut order: Option<(i8, &'static str)> = None;
        if st.trend[i] == 1 {
            if st.trend[i - 1] == 0 {
                if pos == -1 && sm[i] >= tl[i] { order = Some((1, "Trend start, SmoothPrice >= Trendline · buy")); }
                if pos == 1 && sm[i] < tl[i] { order = Some((-1, "Trend start, SmoothPrice < Trendline · sell")); }
            }
            if cross_over(sm, tl, i) { order = Some((1, "SmoothPrice crosses over Trendline · buy")); }
            if cross_under(sm, tl, i) { order = Some((-1, "SmoothPrice crosses under Trendline · sell")); }
        } else if !trend_only {
            if cross_over(&st.lead, &st.sine, i) { order = Some((1, "Cycle mode: LeadSine crosses over Sine · buy")); }
            if cross_under(&st.lead, &st.sine, i) { order = Some((-1, "Cycle mode: LeadSine crosses under Sine · sell")); }
        }
        order
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn rows(n: usize, f: impl Fn(usize) -> f64) -> Vec<Row> {
        (0..n).map(|i| { let c = f(i); Row { ticker: "T".into(), date: format!("{:05}", i), open: c, high: c + 1., low: c - 1., close: c, volume: None } }).collect()
    }
    #[test]
    fn ts_crossing_skips_equal_bars() {
        let a = [1., 2., 2., 3.]; let b = [2., 2., 2., 2.];
        assert!(cross_over(&a, &b, 3)); assert!(!cross_over(&a, &b, 1));
        assert!(cross_under(&b, &a, 3));
        assert!(!cross_over(&[0.,0.,1.], &[0.,0.,0.], 2));
        assert!(!cross_over(&[1.,2.,3.], &[0.,2.,2.], 2));
    }
    #[test]
    fn source_zero_state_and_current_bar_gate_are_preserved() {
        let r=rows(100, |_|100.);let x=Ctx::new(&r,&r,&[]);
        let st=sinetrend_with_mbb(&x,1.,60);
        assert!(st.sine[..65].iter().chain(&st.lead[..65]).all(|v|*v==0.));
        assert!(st.smooth[..65].iter().all(|v|*v==0.));
        assert_eq!(st.smooth[65],100.);
        assert_eq!(st.trendline[65],100.);
        assert_eq!(atan_d(1.),45.);assert!((sin_d(90.)-1.).abs()<1e-15);
    }
    #[test]
    fn unapproved_variants_retain_authoritative_startup_and_parameters() {
        let r=rows(100, |_|100.);let x=Ctx::new(&r,&r,&[]);
        let legacy=sinetrend(&x,1.15);
        assert!(legacy.lead[..55].iter().all(|v|*v==sin_d(45.)));
        for id in ["ehrs_sinetrend_trend","ehrs_sinetrend_cp115","ehrs_sinetrend_cp115_trend","ehrs_sinetrend_cp110"] {
            let schema=params(id).unwrap();assert_eq!(schema.len(),1);
            assert_eq!(schema[0].key,"cyc_part");assert_eq!(schema[0].min,0.5);
            assert!(run(id,&x,60,99,&Params::from([("max_bars_back".into(),60.)])).is_none());
        }
    }
    #[test]
    fn next_open_orders_reverse_once_and_terminal_signal_has_no_fill() {
        let r=rows(5, |i|100.+i as f64);let x=Ctx::new(&r,&r,&[]);
        let t=next_open_sim(&x,0,4,|i,_|Some((if i<2 {1} else {-1},"test")));
        assert_eq!(t.len(),2);assert_eq!(t[0].legs[0].i,1);
        assert_eq!(t[0].exit_i,3);assert_eq!(t[0].exit_px,x.o[3]);
        assert_eq!(t[1].legs[0].i,3);assert!(t[1].open);assert_eq!(t[1].exit_px,x.c[4]);
        let empty=next_open_sim(&x,0,4,|i,_|if i==4 {Some((1,"unfilled"))} else {None});
        assert!(empty.is_empty());
    }
    #[test]
    fn sinetrend_rejects_invalid_platform_settings() {
        for p in [Params::from([("max_bars_back".into(),50.5)]),Params::from([("cyc_part".into(),f64::NAN)]),
            Params::from([("cyc_part".into(),1.5),("max_bars_back".into(),50.)]),Params::from([("unknown".into(),1.)])] {
            assert!(validate("ehrs_sinetrend",&p).is_some());
        }
    }
    #[test]
    fn homodyne_measures_a_pure_20_bar_cycle() {
        let r = rows(400, |i| 100. + 10. * (2. * std::f64::consts::PI * i as f64 / 20.).sin());
        let x = Ctx::new(&r, &r, &[]);
        let sp = homodyne(&x).smooth_period[399];
        assert!((sp - 20.).abs() < 1.5, "measured {sp}");
    }
    #[test]
    fn sinetrend_reverses_at_next_open_and_is_causal() {
        let r = rows(600, |i| 100. + 0.05 * i as f64 + 6. * (2. * std::f64::consts::PI * i as f64 / 24.).sin());
        let x = Ctx::new(&r, &r, &[]);
        let full = run("ehrs_sinetrend", &x, 0, 599, &Params::new()).unwrap();
        assert!(full.len() > 5);
        for w in full.windows(2) { assert_eq!(w[0].exit_i, w[1].legs[0].i); assert_eq!(w[0].dir, -w[1].dir); }
        let cut = run("ehrs_sinetrend", &x, 0, 400, &Params::new()).unwrap();
        for t in cut.iter().filter(|t| !t.open) {
            assert!(full.iter().any(|u| u.legs[0].i == t.legs[0].i && u.exit_i == t.exit_i));
        }
    }
}
