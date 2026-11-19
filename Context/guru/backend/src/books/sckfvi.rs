//! Markos Katsanos, "Detecting Breakouts In Intraday Charts" (the volatility-modified FVE strategy), Technical Analysis of
//! Stocks & Commodities V.21:9 (September 2003) pp.44-55 (research share Trading Knowledge I/Stocks & Commodities DVD 11.26
//! (1982 - 2008) (traders.com)/VOLUMES/V21/C09/190KATS.pdf). Book id `sckfvi`. Lane PA1 (13 Nov 2026, pool A of
//! authors/a1/A1.tsv). Not the April 2003 FVE divergence system (mkfve_div): different FVE cutoff, entry and exit.
//! Independent check authors/pa1/pa1_ref.py.
use crate::connors::{sma, Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::sctraub::ema0;
use super::tselhs::Book;

pub const REVIEWED: &[&str] = &["sckfvi_daily"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Complete rule (printed TradeStation 7 code, Sidebar 1 part 3, with the daily column of Figure 7); unverified] TP = (H+L+C)/3; INTRA = ln H - ln L; INTER = ln TP - ln TP[1]; VINTRA, VINTER = population standard deviations (TradeStation StandardDev, data type 1 as in Sidebar 1) of the last Samples values; CUTOFF = 0.1 VINTRA + 0.1 VINTER; MF = C - (H+L)/2 + TP - TP[1]; factor = +1 if MF > CUTOFF x C, -1 if MF < -CUTOFF x C, else 0; FVE = sum of volume x factor over Samples bars / (Samples-bar average volume x Samples) x 100. Daily inputs (Figure 7): Samples 24, entry -20 < FVE < 10, linear regression angle of FVE over 20 bars > 30 degrees, FVE above its 40-bar exponential average, the 30-bar regression slope of the close between -0.2% and +0.6% of the line's value 29 bars ago (a basing price); exit when the 20-bar FVE regression angle is below -30 degrees, or 50 bars after the entry (at the first close at least 50 bars after it when a volume-less day leaves FVE undefined, R4 #324 M2). Buy and sell at the bar's close ('this bar on close'), long only. Angle = arctangent of the regression slope in degrees (TradeStation LinearRegAngle). [Declared reading] FVE starts after 2 x Samples bars, as the code's 'BarNumber > 2*Samples'; its exponential average is seeded with the first FVE value. The article's daily test (Atmel, 9/4/2001-5/20/2003, $10,000 a trade): 4 trades, 3 winners, +$16,943.70. Atmel is not held, so no book check. The 5-, 15- and 30-minute versions need stock minute data the vault does not hold. Runs on indices, ETFs and stocks with volume, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "sckfvi_daily", name: "Katsanos volatility FVE breakout: FVE turning up sharply near zero while price bases; out on a steep FVE fall or after 50 bars", group: "S&C: Detecting Breakouts In Intraday Charts (Katsanos, V.21:9)", session: 0, dir: "Long", status: Status::Live,
        needs_volume: true, needs_vix: false, needs_series: &[], book: "sckfvi", rules: &[
            "F|\"INTRA=LOG(High)-LOG(Low); VINTRA=StandardDev(INTRA,SAMPLES); INTER=LOG(TP)-LOG(TP1); VINTER=StandardDev(INTER,SAMPLES); CUTOFF=CINTRA*VINTRA+CINTER*VINTER; MF=(Close - (High + Low)/2)+ TP-TP1; ... FVE=(FVEsum / (Average(Volume,Samples)*Samples))*100\" (V.21:9 Sidebar 1, p.54)",
            "E|\"Condition1=FVE>FVEENTERL AND FVE<FVEENTERU; Condition2=LinearRegANGLEFC(FVE,LRPERIOD) >BANGLE; Condition3=FVE> XAVERAGE(FVE,MA); Condition4 =LinearRegSlopeFC(C,LRC)< UB*LINEARREGVALUE(C,LRC,LRC-1)/100 AND LinearRegSlopeFC(C,LRC ) >LB*LINEARREGVALUE(C,LRC,LRC-1)/100 ... Buy (\"BUY\") THIS BAR ON CLOSE\" (p.54); daily inputs FVE period 24, bounds -20/10, EMA 40, regression 20 bars, angles 30/-30, price regression 30 bars, bounds 0.6/-0.2%, time exit 50 bars (Figure 7)",
            "X|\"Condition5 =LinearRegANGLE(FVE,LRPERIOD )<SANGLE; If condition5 then Sell (\"FVE EXIT\") this bar AT CLOSE; If BarsSinceEntry = BarToExitOn then Sell (\"TimeBarsLX\") this bar AT CLOSE\" (p.54)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    reviewed(id).then_some(("sckfvi_daily", "Katsanos volatility FVE breakout (S&C)", "daily"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("samples", "FVE period (daily 24)", 24., 5., 120., 1.), p("hold", "Time exit, bars (daily 50)", 50., 5., 150., 1.),
        p("ub", "Price regression upper bound, % a bar (daily 0.6)", 0.6, 0., 2., 0.01), p("lb", "Price regression lower bound, % a bar (daily -0.2)", -0.2, -2., 0., 0.01),
        p("sangle", "Exit angle, degrees (daily -30)", -30., -80., 0., 1.)])
}

/// Least-squares line over v[i+1-n..=i]: (slope per bar, value at the window's first bar).
fn linreg(v: &[f64], i: usize, n: usize) -> (f64, f64) {
    if i + 1 < n { return (f64::NAN, f64::NAN); }
    let w = &v[i + 1 - n..=i];
    let (xm, ym) = ((n as f64 - 1.) / 2., w.iter().sum::<f64>() / n as f64);
    let (mut sxy, mut sxx) = (0., 0.);
    for (k, y) in w.iter().enumerate() { sxy += (k as f64 - xm) * (y - ym); sxx += (k as f64 - xm).powi(2); }
    let b = sxy / sxx;
    (b, ym - b * xm)
}
fn angle(v: &[f64], i: usize, n: usize) -> f64 { linreg(v, i, n).0.atan().to_degrees() }

fn pstd(v: &[f64], i: usize, n: usize) -> f64 {
    if i + 1 < n { return f64::NAN; }
    let w = &v[i + 1 - n..=i];
    let m = w.iter().sum::<f64>() / n as f64;
    (w.iter().map(|z| (z - m).powi(2)).sum::<f64>() / n as f64).sqrt()
}

/// Volatility-modified FVE (Sidebar 1); NaN for the first 2 x samples bars and where volume is missing.
pub fn fve(x: &Ctx, s: usize) -> Vec<f64> {
    let n = x.c.len();
    let v: Vec<f64> = x.r.iter().map(|r| r.volume.filter(|v| *v > 0.).unwrap_or(f64::NAN)).collect();
    let tp: Vec<f64> = (0..n).map(|i| (x.h[i] + x.l[i] + x.c[i]) / 3.).collect();
    let intra: Vec<f64> = (0..n).map(|i| x.h[i].ln() - x.l[i].ln()).collect();
    let inter: Vec<f64> = (0..n).map(|i| if i == 0 { f64::NAN } else { tp[i].ln() - tp[i - 1].ln() }).collect();
    let term: Vec<f64> = (0..n).map(|i| {
        if i == 0 { return f64::NAN; }
        let cut = 0.1 * pstd(&intra, i, s) + 0.1 * pstd(&inter, i, s);
        let mf = x.c[i] - (x.h[i] + x.l[i]) / 2. + tp[i] - tp[i - 1];
        let f = if mf > cut * x.c[i] { 1. } else if mf < -cut * x.c[i] { -1. } else if cut.is_finite() { 0. } else { f64::NAN };
        v[i] * f
    }).collect();
    let (sm, mv) = (sma(&term, s), sma(&v, s));
    (0..n).map(|i| if i < 2 * s { f64::NAN } else { sm[i] / mv[i] * 100. }).collect()
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d);
    let s = g("samples", 24.).round().max(2.) as usize;
    let hold = g("hold", 50.).round().max(1.) as usize;
    let (ub, lb, sang) = (g("ub", 0.6), g("lb", -0.2), g("sangle", -30.));
    let f = fve(x, s);
    let fe = ema0(&f, 40);
    let mut pos = Book::new();
    for t in from..=to {
        if pos.side() > 0. {
            // R4 #324 M2: the time exit does not depend on FVE (a volume-less day makes FVE NaN for Samples bars)
            let e = pos.entry_bar().unwrap_or(t);
            if f[t].is_finite() && angle(&f, t, 20) < sang { pos.exit(t, x.c[t], "FVE exit"); } else if t - e >= hold { pos.exit(t, x.c[t], "time exit"); }
            continue;
        }
        if !f[t].is_finite() { continue; }
        let ang = angle(&f, t, 20);
        let (slope, v0) = linreg(&x.c, t, 30);
        if f[t] > -20. && f[t] < 10. && ang > 30. && f[t] > fe[t] && slope < ub * v0 / 100. && slope > lb * v0 / 100. {
            pos.enter(1., t, x.c[t], "close");
        }
    }
    Some(pos.done(to, x.c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fve_entries_meet_every_condition() {
        let r = super::super::sctraub::tests::bars(4000);
        let x = Ctx::new(&r, &r, &[]);
        let f = fve(&x, 24);
        assert!(f[47].is_nan() && f[48].is_finite() && f.iter().skip(48).all(|v| (-100.0..=100.0).contains(v)));
        let mut p = Params::new(); p.insert("ub".into(), 2.); p.insert("lb".into(), -2.);
        let t = run("sckfvi_daily", &x, 0, r.len() - 1, &p).unwrap();
        let fe = ema0(&f, 40);
        assert!(!t.is_empty());
        for z in &t {
            let e = z.legs[0].i;
            assert!(f[e] > -20. && f[e] < 10. && angle(&f, e, 20) > 30. && f[e] > fe[e]);
            if !z.open { assert!(z.exit_i - e <= 50 && (z.exit_i - e == 50 || angle(&f, z.exit_i, 20) < -30.)); }
        }
    }
}

// Instrument admission: S&C Katsanos volatility FVE 2026-11-13 (lane PA1).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "sckfvi" && reviewed(def.id),
    label: "Markos Katsanos tested the volatility-modified FVE breakout on one Nasdaq stock (Atmel, daily and 5-30 minute bars); daily bars of indices, ETFs and stocks with volume, US and India, are a declared application.",
    basis: "Technical Analysis of Stocks & Commodities V.21:9 pp.44-55 (Detecting Breakouts In Intraday Charts), Sidebar 1 part 3 and Figure 7; independent check authors/pa1/pa1_ref.py",
};
