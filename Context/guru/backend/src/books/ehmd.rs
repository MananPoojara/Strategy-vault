//! John Ehlers - MESA 98/2000 article set ("Mesa Misc. Articles") + "MESA Adaptive Moving
//! Averages", Stocks & Commodities V.19:9 (2001). Source + checks: library/ehlers/.
//! Explicit research MaxBarsBack, degrees, next-bar market fills, TS crossings
//! and the Hilbert homodyne core are shared with ehrs (Rocket Science for Traders).
use super::ehrs::{atan_d, cross_over, cross_under, homodyne_with_mbb, next_open_sim, MBB};
use crate::connors::{claim, Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "ehmd";

/// MAMA / FAMA exactly as the S&C sidebar (alpha also capped at FastLimit, a no-op since DeltaPhase >= 1).
pub fn mama(x: &Ctx, fast: f64, slow: f64) -> (Vec<f64>, Vec<f64>) {
    mama_with_mbb(x, fast, slow, MBB)
}

pub fn mama_with_mbb(x: &Ctx, fast: f64, slow: f64, mbb: usize) -> (Vec<f64>, Vec<f64>) {
    let hd = homodyne_with_mbb(x, mbb);
    let n = x.c.len();
    let (mut phase, mut m, mut f) = (vec![0f64; n], vec![0f64; n], vec![0f64; n]);
    for i in mbb..n {
        phase[i] = phase[i - 1]; m[i] = m[i - 1]; f[i] = f[i - 1];
        if i - mbb + 1 <= 5 { continue; }
        if hd.i1[i] != 0. { phase[i] = atan_d(hd.q1[i] / hd.i1[i]); }
        let mut dp = phase[i - 1] - phase[i];
        if dp < 1. { dp = 1.; }
        let mut a = fast / dp;
        if a < slow { a = slow; }
        if a > fast { a = fast; }
        m[i] = a * hd.price[i] + (1. - a) * m[i - 1];
        f[i] = 0.5 * a * m[i] + (1. - 0.5 * a) * f[i - 1];
    }
    (m, f)
}

/// 4-element Laguerre RSI (Time Warp, Fig. 9). No CurrentBar gate in the code.
pub fn laguerre_rsi(x: &Ctx, g: f64) -> Vec<f64> {
    let n = x.c.len();
    let (mut l0, mut l1, mut l2, mut l3, mut rsi) = (vec![0f64; n], vec![0f64; n], vec![0f64; n], vec![0f64; n], vec![0f64; n]);
    for i in MBB..n {
        l0[i] = (1. - g) * x.c[i] + g * l0[i - 1];
        l1[i] = -g * l0[i] + l0[i - 1] + g * l1[i - 1];
        l2[i] = -g * l1[i] + l1[i - 1] + g * l2[i - 1];
        l3[i] = -g * l2[i] + l2[i - 1] + g * l3[i - 1];
        let (mut cu, mut cd) = (0., 0.);
        if l0[i] >= l1[i] { cu = l0[i] - l1[i]; } else { cd = l1[i] - l0[i]; }
        if l1[i] >= l2[i] { cu += l1[i] - l2[i]; } else { cd += l2[i] - l1[i]; }
        if l2[i] >= l3[i] { cu += l2[i] - l3[i]; } else { cd += l3[i] - l2[i]; }
        rsi[i] = if cu + cd != 0. { cu / (cu + cd) } else { rsi[i - 1] };
    }
    rsi
}

/// Griffiths LMS predictor of the zero-lag average (Linear Prediction, Sidebar 1, counter-trend
/// form). Returns (Value1 = zero-lag average, Value2 = 2-bar prediction).
pub fn lms_zerolag(x: &Ctx, len: usize) -> (Vec<f64>, Vec<f64>) {
    let n = x.c.len();
    let price: Vec<f64> = (0..n).map(|i| (x.h[i] + x.l[i]) / 2.).collect();
    let (mut v1, mut v2) = (vec![0f64; n], vec![0f64; n]);
    let (mut g, mut sp) = ([0f64; 31], [0f64; 31]);
    let mut mu = 0.;
    for i in MBB..n {
        v2[i] = v2[i - 1];
        v1[i] = 0.25 * (price[i] + 0.5 * (price[i] - price[i - 4])) + 0.75 * v1[i - 1];
        let pow = (0..len).map(|k| v1[i - k] * v1[i - k]).sum::<f64>() / len as f64;
        if pow > 0. { mu = 0.25 / (pow * len as f64); }
        if i - MBB + 1 > len {
            let xbar: f64 = (1..=len).map(|k| v1[i - k] * g[k]).sum();
            for k in 1..=len { g[k] += mu * (v1[i] - xbar) * v1[i - k]; }
            for k in 0..=len { sp[k] = v1[i - (len - k)]; }
            for k in len + 1..=len + 5 {
                sp[k] = 0.;
                for k1 in 1..=len { sp[k] += sp[k - k1] * g[k1]; }
            }
            v2[i] = sp[len + 2];
        }
    }
    (v1, v2)
}

const NT: &str = "Mesa articles · indicators without trading rules";
pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, dir, status, rules, notes, claim| StrategyDef { id, name, group, session: 0, dir, status,
        needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim };
    let nt = |id, name, why, rules, notes| d(id, name, NT, "Long/Short", Status::NotTestable(why), rules, notes, None);
    vec![
        d("ehmd_mama", "MAMA / FAMA crossover (long only)", "MAMA - S&C V.19:9 (2001)", "Long", Status::Live,
          &["F|MAMA/FAMA per the article's EasyLanguage: Price = (H+L)/2, FastLimit = .5, SlowLimit = .05; alpha = FastLimit/DeltaPhase (DeltaPhase = Phase[1] - Phase, floored at 1), bounded by SlowLimit; MAMA = alpha*Price + (1-alpha)*MAMA[1]; FAMA = .5*alpha*MAMA + (1-.5*alpha)*FAMA[1]. (S&C sidebar; RSFT Fig. 17.1)",
            "E|\"I tested the MAMA crossover system ... taking long side trades only\": buy when MAMA crosses over FAMA (fill next open).",
            "X|Exit the long when MAMA crosses under FAMA (fill next open)."],
          "The article explicitly tests a long-only MAMA/FAMA crossover on 100 stocks. This research run uses one actual stock at a time, with MAMA above FAMA as the long crossing and the opposite crossing as exit. Next-open market fills are a declared execution convention: the supplied code plots indicators and does not specify orders. MaxBarsBack=50 is an explicit configurable research setting, not printed author metadata. Source Vars(0) and CurrentBar>5 are preserved. Flat at selected start, one position, terminal positions stay open. No stock universe, portfolio sizing or source-study performance is reproduced. Review: library/ehlers/REVIEW-2026-10-13.md.",
          claim("1317 (100 stocks)", "37.5%", "$4.86/share gross", "~4.4 trades/yr/stock", "2 Jan 1998-2 Jan 2001, 100 stocks")),
        d("ehmd_laguerre_rsi", "Laguerre RSI 20/80", "Time Warp - Without Space Travel", "Long/Short", Status::Live,
          &["F|Laguerre RSI, gamma = .5: L0 = (1-gamma)*Close + gamma*L0[1]; L1 = -gamma*L0 + L0[1] + gamma*L1[1]; L2, L3 likewise; RSI = CU/(CU+CD) over the four Laguerre elements. (Fig. 9)",
            "E|\"buy after the line crosses back over the 20% level\" (RSI crosses over .2; fill next open).",
            "E|\"sell after the price [line] crosses back down over the 80% level\" (RSI crosses under .8; fill next open).",
            "X|No separate exit is stated: the opposite signal closes the position and (default reading) reverses it."],
          "Default reading: Ehlers' TS2000i 'sell' = sell short, so buy/sell reverse (stop-and-reverse). Option long_only = 1 treats 'sell' as exit-long only (labelled non-default). Article cites wellinvested.com: top system on DIA in 2002 and 118.3% annualized on AKSYS - no trade statistics printed.",
          None),
        d("ehmd_lms_zerolag", "LMS predictor of zero-lag average (counter-trend)", "Linear Prediction", "Long/Short", Status::Live,
          &["F|Value1 = .25*(Price + .5*(Price - Price[4])) + .75*Value1[1], Price = (H+L)/2; Griffiths LMS predictor, Length = 10, Mu = .25/(SigPower*Length); Value2 = 2-bar prediction (Sidebar 1).",
            "E|\"the signal to buy is when the zero lag moving average crosses over the 2 day prediction of the moving average\" (fill next open).",
            "E|\"the signal to sell is when the zero lag moving average crosses under the 2 day prediction\" (fill next open).",
            "X|No separate exit is stated: the opposite signal reverses the position."],
          "Signals stated as counter-trend line crossings; positions read as long/short reversals (the text walks from a long to a short signal). TS next-bar market fills. No published statistics.",
          None),
        nt("ehmd_lms_stoch", "LMS predictor of the Slow Stochastic",
           "SlowK(10) is a TradeStation 2000i built-in whose smoothing (legacy vs Lane original) the article does not define",
           &["E|Buy when the 2-day prediction crosses over the Stochastic; sell when it crosses under (Linear Prediction article)."], ""),
        nt("ehmd_market_mode", "Instantaneous Trendline trend rule (Market Mode Strategies)",
           "prose guideline; the author says 'In future articles we will develop filters to engage these rules with greater precision' - the precise version is RSFT SineTrend (ehrs)",
           &["E|Don't trade the trend until price has crossed the Instantaneous Trendline more than a half dominant cycle ago; stop trading the trend when price crosses again."], "Duplicate in spirit of ehrs_sinetrend_trend."),
        nt("ehmd_opt_predictor", "Optimum predictive filter on RSI",
           "step 1 is judgement: 'gradually decreasing the observation period so that the peak values almost reach the limits'",
           &["E|Buy/sell when the optimum predictor (RSI-0.5 minus its EMA, alpha .25) crosses the RSI."], ""),
        nt("ehmd_squelch", "Squelch indicator", "mode filter (paintbar) only, no entry or exit rules", &["F|Trend Mode if the measured dominant cycle > 20 bars."], ""),
        nt("ehmd_bandpass", "Bandpass indicator + leading signal", "crossings flag 'long and short trade entry points' but the article never states which crossing is long, and Pcenter is a user choice",
           &["E|Entries at crossings of the Bandpass Indicator and the Leading Signal."], ""),
        nt("ehmd_osc_family", "Fisher Transform / CG / RVI / RSI smoothing / Joy of Detrending",
           "indicators only; e.g. RVI 'the rules for the use of the RVI are flexible'", &["F|Oscillators with trigger lines."], ""),
        nt("ehmd_phase_tutorial", "Stay in Phase / Cycles Tutorial / STARE-E-O / Skinning the Cat",
           "discretionary chart examples (MESA2000 screens, stereograms)", &["E|Sinewave Indicator signals while the market is judged to be in Cycle Mode."], ""),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "ehmd_mama" => ("ehmd_mama", "Ehlers MAMA/FAMA", "Long only"),
        "ehmd_laguerre_rsi" => ("ehmd_laguerre", "Ehlers Laguerre RSI", "20/80 crossings"),
        "ehmd_lms_zerolag" | "ehmd_lms_stoch" => ("ehmd_lms", "Ehlers LMS predictor", if id == "ehmd_lms_zerolag" { "Zero-lag average" } else { "Slow Stochastic" }),
        "ehmd_market_mode" | "ehmd_opt_predictor" | "ehmd_squelch" | "ehmd_bandpass" | "ehmd_osc_family" | "ehmd_phase_tutorial" => ("ehmd_indicators", "Ehlers MESA articles", "No system rules"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(match id {
        "ehmd_mama" => vec![p("fast_limit", "FastLimit", 0.5, 0.1, 0.9, 0.05), p("slow_limit", "SlowLimit", 0.05, 0.01, 0.2, 0.01),
            p("max_bars_back", "MaxBarsBack (declared research setting)", 50., 50., 250., 1.)],
        "ehmd_laguerre_rsi" => vec![p("gamma", "Laguerre damping gamma", 0.5, 0.1, 0.9, 0.05), p("lower", "Buy level", 0.2, 0.05, 0.4, 0.05),
            p("upper", "Sell level", 0.8, 0.6, 0.95, 0.05), p("long_only", "Long only (1 = 'sell' only exits; not the default reading)", 0., 0., 1., 1.)],
        "ehmd_lms_zerolag" => vec![p("length", "LMS Length", 10., 4., 25., 1.)],
        _ => return None,
    })
}

pub fn validate(id: &str, p: &Params) -> Option<String> {
    if id != "ehmd_mama" { return None; }
    let schema = params(id).unwrap();
    for (key, value) in p {
        let Some(d) = schema.iter().find(|d| d.key == key) else { return Some(format!("Unknown MAMA parameter: {key}")); };
        if !value.is_finite() || !(d.min..=d.max).contains(value) { return Some(format!("Invalid MAMA parameter: {key}")); }
    }
    let get = |k| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap().def);
    if get("slow_limit") > get("fast_limit") { return Some("SlowLimit must not exceed FastLimit.".into()); }
    if get("max_bars_back").fract() != 0. { return Some("MaxBarsBack must be an integer.".into()); }
    None
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if validate(id, p).is_some() || from >= to || to >= x.c.len() { return None; }
    for (k, v) in p { let d = schema.iter().find(|d| d.key == k)?; if !(d.min..=d.max).contains(v) { return None; } }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap().def);
    let mbb = if id == "ehmd_mama" { g("max_bars_back") as usize } else { MBB };
    Some(match id {
        "ehmd_mama" => {
            let (m, f) = mama_with_mbb(x, g("fast_limit"), g("slow_limit"), mbb);
            next_open_sim(x, from, to, |i, _| mama_order(&m, &f, i, mbb))
        }
        "ehmd_laguerre_rsi" => {
            let r = laguerre_rsi(x, g("gamma"));
            let n = r.len();
            let (lo, hi) = (vec![g("lower"); n], vec![g("upper"); n]);
            let short = if g("long_only") == 1. { 0 } else { -1 };
            next_open_sim(x, from, to, |i, _| {
                if i <= MBB { None } else if cross_over(&r, &lo, i) { Some((1, "Laguerre RSI crosses over 20% · buy")) }
                else if cross_under(&r, &hi, i) { Some((short, "Laguerre RSI crosses under 80% · sell")) } else { None }
            })
        }
        "ehmd_lms_zerolag" => {
            let len = g("length");
            if len.fract() != 0. { return None; }
            let (v1, v2) = lms_zerolag(x, len as usize);
            next_open_sim(x, from, to, |i, _| {
                if i < MBB + len as usize + 1 { None } else if cross_over(&v1, &v2, i) { Some((1, "Zero-lag avg crosses over 2-bar prediction · buy")) }
                else if cross_under(&v1, &v2, i) { Some((-1, "Zero-lag avg crosses under 2-bar prediction · sell")) } else { None }
            })
        }
        _ => return None,
    })
}

pub fn mama_order(m: &[f64], f: &[f64], i: usize, mbb: usize) -> Option<(i8, &'static str)> {
    if i < mbb + 5 { None } else if cross_over(m, f, i) { Some((1, "MAMA crosses over FAMA · buy")) }
    else if cross_under(m, f, i) { Some((0, "MAMA crosses under FAMA · exit")) } else { None }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn mama_zero_seed_waits_for_source_current_bar_gate() {
        let r:Vec<Row>=(0..100).map(|i|Row {ticker:"T".into(),date:i.to_string(),open:100.,high:101.,low:99.,close:100.,volume:None}).collect();
        let x=Ctx::new(&r,&r,&[]);let (m,f)=mama_with_mbb(&x,0.5,0.05,60);
        assert!(m[..65].iter().chain(&f[..65]).all(|v|*v==0.));
        assert_eq!(m[65],50.);assert_eq!(f[65],12.5);
        assert!(mama_order(&m,&f,65,60).is_none()); // no earlier opposite inequality
    }
    #[test]
    fn mama_rejects_fractional_mbb_nonfinite_and_inverted_alpha_limits() {
        for p in [Params::from([("max_bars_back".into(),50.5)]),Params::from([("fast_limit".into(),f64::NAN)]),
            Params::from([("fast_limit".into(),0.1),("slow_limit".into(),0.2)]),Params::from([("unknown".into(),1.)])] {
            assert!(validate("ehmd_mama",&p).is_some());
        }
    }
    #[test]
    fn mama_long_only_and_laguerre_bounds() {
        let r: Vec<Row> = (0..500).map(|i| { let c = 100. + 8. * (i as f64 / 9.).sin() + 0.02 * i as f64;
            Row { ticker: "T".into(), date: format!("{:05}", i), open: c, high: c + 1., low: c - 1., close: c, volume: None } }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let ts = run("ehmd_mama", &x, 0, 499, &Params::new()).unwrap();
        assert!(!ts.is_empty() && ts.iter().all(|t| t.dir == 1.));
        assert!(laguerre_rsi(&x, 0.5).iter().all(|v| (0. ..=1.).contains(v)));
        let lr = run("ehmd_laguerre_rsi", &x, 0, 499, &Params::new()).unwrap();
        assert!(lr.iter().any(|t| t.dir == -1.));
        let lo = run("ehmd_laguerre_rsi", &x, 0, 499, &Params::from([("long_only".to_string(), 1.)])).unwrap();
        assert!(lo.iter().all(|t| t.dir == 1.));
    }
}
