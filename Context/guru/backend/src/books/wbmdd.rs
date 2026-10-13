//! William Blau, *Momentum, Direction, and Divergence* (Wiley, 1995).
//! Only the Chapter 2 "Trading Ergodics" systems state complete entry AND exit rules with every
//! parameter printed; everything else is a filter, an incomplete demo ("CAUTION: This is not intended
//! to be a complete trading system"), intraday tick data, or has an unprinted SMI(q,r,s).
//! Source, transcription and independent checks: library/wbmdd/.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "wbmdd";

const R_ERG_EMA: &[&str] = &[
    "F|Ergodic(Close,r) = TSI(Close,r,5); SignalLine(Close,r) = EMA(TSI(Close,r,5),5) (p.18; App. B Fig B-3: Value1 = TSI(Close,r,5,1), Value2 = XAverage(TSI(Close,r,5,1),5))",
    "F|TSI(close,r,s,u) = 100·TXAverage(Price−Price[1],r,s,u) / TXAverage(|Price−Price[1]|,r,s,u); TXAverage = XAverage(XAverage(XAverage(Price,r),s),u) (App. B Figs B-1, B-2)",
    "F|\"The trend is defined by a moving average on price.\" Fig 2-15: \"the EMA of the close is for 32 days; the Ergodic oscillator has a 32-day EMA\" -> trend = EMA(close,32), Ergodic = TSI(close,32,5) (p.20-21)",
];
const R_ERG_TSI: &[&str] = &[
    "F|Ergodic(Close,r) = TSI(Close,r,5); SignalLine = EMA(TSI(Close,r,5),5) (p.18; App. B Fig B-3)",
    "F|\"Refer to Figure 2-16, which shows a slow TSI(close,64,64)\" (p.22). \"Three systems may be considered as before with the moving average trend. The rules presented there remain the same except the 'trend' is now produced by the slow TSI trend.\" (p.23). Fig 2-17 labels the Ergodic TSI(close,64,5) (p.24)",
];
const S1: &str = "E|System 1: \"1. Enter or hold a position only when the slope of the Ergodic Signal Line has the same direction as the trend. 2. Stand aside when the slope of the Ergodic Signal Line is in the opposite direction of the trend. 3. Enter or exit a position when the Ergodic and its Signal Line cross.\" (p.20-21)";
const S2: &str = "E|System 2: \"1. Same as System 1. 2. Same as System 1. 3. Enter a position when the Ergodic and its Signal Line cross in the direction of the trend. See also (1). 4. Exit a position when the Ergodic reverses its direction.\" (p.21)";
const S3: &str = "E|System 3: \"1. Enter or hold a position only when the slope of the Ergodic Signal Line is in the same direction as the trend. 2. Exit a position or stand aside when the slope of the Ergodic Signal Line is in the opposite direction as the trend.\" (p.21)";
const X1: &str = "X|Long held while Signal-Line slope up, trend slope up and Ergodic above its Signal Line; any one failing exits (an opposite alignment reverses). Short mirror. Orders: next open after the signal close [book silent; TradeStation market order]";
const X2: &str = "X|Entry only on the bar the Ergodic crosses its Signal Line in the trend direction while both slopes agree; exit when the Ergodic turns against the position (Ergodic today vs yesterday) or the slopes stop agreeing. Orders: next open [book silent]";
const X3: &str = "X|Long while Signal-Line slope and trend slope are both up, short while both down, flat otherwise. Orders: next open [book silent]";

const EMA_1: &[&str] = &[R_ERG_EMA[0], R_ERG_EMA[1], R_ERG_EMA[2], S1, X1];
const TSI_1: &[&str] = &[R_ERG_TSI[0], R_ERG_TSI[1], S1, X1];
const EMA_2: &[&str] = &[R_ERG_EMA[0], R_ERG_EMA[1], R_ERG_EMA[2], S2, X2];
const TSI_2: &[&str] = &[R_ERG_TSI[0], R_ERG_TSI[1], S2, X2];
const EMA_3: &[&str] = &[R_ERG_EMA[0], R_ERG_EMA[1], R_ERG_EMA[2], S3, X3];
const TSI_3: &[&str] = &[R_ERG_TSI[0], R_ERG_TSI[1], S3, X3];

const NOTE_EMA: &str = "Book example: daily Deutsche mark (Fig 2-15, Aug 1989-Mar 1990) with lettered trades only; no numeric results are printed, so there is nothing to reproduce. Run on the cash index (vault convention; long and short). Slope = today's value minus yesterday's (> 0 up, < 0 down, 0 = no direction). EMAs are TradeStation XAverage (alpha 2/(n+1), seeded with the first value); trading starts after a warm-up of the longest smoothing chain (Ergodic r+5+5 bars, trend length). p.fill=1 (non-default) fills at the signal close instead of the next open. The book's aside that a short 'may not be exercised since it occurs in oversold territory' (p.21) is judgement and not coded.";
const NOTE_TSI: &str = "Book example: daily Deutsche mark Nov 1990-Apr 1991 (Fig 2-17) with lettered trades only; no numeric results. Trend slope = slope of TSI(close,64,64). The Ergodic length 64 is read from the Fig 2-17 label TSI(close,64,5); p.r=32 gives the Chapter 2-15 Ergodic. The OB/OS +15/-15 levels of the slow TSI are a 'caution' (p.23), not a rule, and are not coded. Same slope/EMA/warm-up/order conventions as the moving-average-trend systems.";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, status, rules, notes| StrategyDef {
        id, name, group, session: 0, dir: "Long/Short", status, needs_volume: false, needs_vix: false,
        needs_series: &[], book: BOOK, rules, notes, claim: None,
    };
    use Status::*;
    vec![
        d("wbmdd_erg_ema_s1", "Ergodic with EMA trend · System 1", "Ch 2 · Trading Ergodics with the Trend", Live,
          EMA_1, NOTE_EMA),
        d("wbmdd_erg_ema_s2", "Ergodic with EMA trend · System 2", "Ch 2 · Trading Ergodics with the Trend", Live,
          EMA_2, NOTE_EMA),
        d("wbmdd_erg_ema_s3", "Ergodic with EMA trend · System 3", "Ch 2 · Trading Ergodics with the Trend", Live,
          EMA_3, NOTE_EMA),
        d("wbmdd_erg_tsi_s1", "Ergodic with slow TSI trend · System 1", "Ch 2 · Trading Ergodics with a Slow TSI Trend", Live,
          TSI_1, NOTE_TSI),
        d("wbmdd_erg_tsi_s2", "Ergodic with slow TSI trend · System 2", "Ch 2 · Trading Ergodics with a Slow TSI Trend", Live,
          TSI_2, NOTE_TSI),
        d("wbmdd_erg_tsi_s3", "Ergodic with slow TSI trend · System 3", "Ch 2 · Trading Ergodics with a Slow TSI Trend", Live,
          TSI_3, NOTE_TSI),
        // ---------------------------------------------------------------- not testable
        d("wbmdd_dti_smi", "DTI_Trade with SMI (rudimentary trading vehicle)", "Ch 7 · Directional Trending",
          NotTestable("the SMI(q,r,s) oscillator that times entries and exits is never given numbers (p.68-69, p.71: 'filtered by a smoothed SMI(q,r,s), where the filtering is identical in all cases'); the tests ran on DM and T-bond futures"),
          &["E|\"Long entry occurs when DTI_Trade goes positive. The position is held as long as the SMI oscillator has the same slope going in the same direction as the trend\" (p.69)",
            "X|\"Long exit occurs when the slope of the SMI oscillator reverses ... or when DTI_Trade returns to zero—whichever comes first.\" Re-entry when the oscillator turns again in the DTI_Trade direction (p.69)",
            "F|Daily tests: DTI double smoothing 28 and 28 with an additional 5-day smoothing; $20 commission, $60 slippage, one contract (p.71)"],
          "Published (p.72-73, TradeStation): DM 6/82-6/92 110 trades, 49% win, avg $111.25, PF 1.24; DM 2/75-6/92 195 trades, 46%, avg $194.68, PF 1.54; T-bond 6/82-6/92 113 trades, 41%, avg $32.56, PF 1.05; T-bond 9/77-6/92 169 trades, 44%, avg $121.92, PF 1.21. Not reproducible: SMI(q,r,s) unprinted (no number is invented) and the markets are futures."),
        d("wbmdd_tsi_trade_smi", "TSI_Trade with SMI basic system", "Ch 8 · True Strength Directional Trending",
          NotTestable("the book says 'CAUTION: This is not intended to be a complete trading system' (p.82) and the SMI parameters are not printed"),
          &["E|\"Enter long when TSI_Trade rises above zero and the slopes of TSI_Trade and the SMI are both positive.\" (p.82)",
            "X|\"Exit long when SMI slope turns negative or when TSI_Trade reverts back to zero, whichever comes first.\" Short mirror (p.82)"],
          "Fig 8-1: TSI_Trade(close,32,13,3) on daily DM 1991; the SMI panel is labelled SMI(q,r,s) only."),
        d("wbmdd_sd_tsi", "Basic slope-divergence TSI system", "Ch 12 · Slope Divergence",
          NotTestable("the 'fast SMI(q,r,s)' that triggers trades has no printed parameters (p.100-101)"),
          &["F|SD_TSI(Price,r,s,u,x,y) = TSI when the slopes of TSI(Price,r,s,u) and DXAverage(Price,x,y) agree, else 0 (App. B Fig B-25); examples SD_TSI(close,32,32,1,32,1) and (close,32,32,7,32,7) (p.99-100)",
            "E|\"We take a position when the slopes are in the same direction. We exit a position when slopes are in opposition.\" (p.101)"],
          "AMGEN daily 1991 chart only; no results."),
        d("wbmdd_tvi_trade", "TVI_Trade intraday system", "Ch 4 / Ch 10 · Tick Volume Indicator",
          NotTestable("intraday up-tick/down-tick counts (no such data) and the book says 'This is not intended to be a complete trading system' (p.89)"),
          &["E|\"Enter long when TVI_Trade rises above zero and the slopes of TVI_Trade and the TVI oscillator are both positive.\" (p.89)",
            "X|\"Exit long when the TVI oscillator slope turns negative or when TVI_Trade reverts back to zero, whichever comes first.\" (p.89)"],
          "S&P 500 60-minute bars, TVI(32,32,5). Ergodic_TVI crossovers (p.45) are intraday illustrations without an exit rule."),
        d("wbmdd_filters_only", "Ergodic MDI/MACD/CSI, 2-day stochastic, SMI_Trade, DTI and ATF filters", "Ch 3, 5, 6, 7, 9, 11",
          NotTestable("filters and oscillators without a complete entry/exit rule set; the book only says trading 'could be' a signal-line crossover 'subject to other trading system constraints' (p.41)"),
          &["F|2-day stochastic: \"The simplest policy, for example, could be to buy when the oscillator crosses above its signal line and to sell when the oscillator crosses below its signal line, subject to other trading system constraints.\" (p.41)",
            "F|ATF: \"Trends are deemed to exist only for positive slopes\" of EMA(|TSI(close,32,1)|,32) or EMA(|SMI(32,1,1)|,32); direction is not indicated (p.92-95)",
            "F|DTI_Trade / SMI_Trade / Ergodic_MDI / Ergodic_CSI: regions and crossovers are described, no exit rule (p.48-58, 67, 83-86)"],
          "Seen and transcribed in library/wbmdd/extract-wbmdd.md; no exit rule is added."),
        d("wbmdd_wilder_di", "Wilder +DI/-DI crossover (Appendix A review)", "App. A · Wilder's ADX",
          NotTestable("a summary of Wilder (1978), not Blau's system; 'You only trade those markets that are high on a (positive only) ADX scale' gives no threshold (p.107)"),
          &["E|\"You go long when +DI14 crosses over -DI14 and you go short when -DI14 crosses over +DI14.\" (p.107)"],
          "Wilder's own rules belong to the Wilder book in the vault."),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let f = ("wbmdd_ergodic", "Blau Ergodic with the trend");
    Some(match id {
        "wbmdd_erg_ema_s1" => (f.0, f.1, "EMA trend · System 1"),
        "wbmdd_erg_ema_s2" => (f.0, f.1, "EMA trend · System 2"),
        "wbmdd_erg_ema_s3" => (f.0, f.1, "EMA trend · System 3"),
        "wbmdd_erg_tsi_s1" => (f.0, f.1, "Slow TSI trend · System 1"),
        "wbmdd_erg_tsi_s2" => (f.0, f.1, "Slow TSI trend · System 2"),
        "wbmdd_erg_tsi_s3" => (f.0, f.1, "Slow TSI trend · System 3"),
        "wbmdd_dti_smi" | "wbmdd_tsi_trade_smi" | "wbmdd_sd_tsi" | "wbmdd_tvi_trade" | "wbmdd_filters_only" | "wbmdd_wilder_di" =>
            ("wbmdd_other", "Blau filters and demo systems", "Not testable"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let fill = p("fill", "Fill: 0 = next open (default), 1 = signal close", 0., 0., 1., 1.);
    let sig = p("sig", "Signal Line EMA (and Ergodic 2nd smoothing)", 5., 3., 12., 1.);
    if id.starts_with("wbmdd_erg_ema_") {
        Some(vec![p("r", "Ergodic first smoothing r", 32., 10., 64., 1.), sig, p("trend", "Trend EMA of close", 32., 10., 100., 1.), fill])
    } else if id.starts_with("wbmdd_erg_tsi_") {
        Some(vec![p("r", "Ergodic first smoothing r", 64., 10., 100., 1.), sig, p("tr", "Slow TSI first smoothing", 64., 20., 100., 1.),
                  p("ts", "Slow TSI second smoothing", 64., 20., 100., 1.), fill])
    } else { None }
}

/// TradeStation XAverage: alpha = 2/(n+1), seeded with the first value.
pub fn xavg(v: &[f64], n: usize) -> Vec<f64> {
    let a = 2.0 / (n as f64 + 1.0);
    let mut out = Vec::with_capacity(v.len());
    for (i, &x) in v.iter().enumerate() {
        out.push(if i == 0 { x } else { out[i - 1] + a * (x - out[i - 1]) });
    }
    out
}

/// Blau TSI(Price,r,s,u) exactly as App. B Fig B-2 (0 when the denominator is 0).
pub fn tsi(c: &[f64], r: usize, s: usize, u: usize) -> Vec<f64> {
    let m: Vec<f64> = (0..c.len()).map(|i| if i == 0 { 0.0 } else { c[i] - c[i - 1] }).collect();
    let a: Vec<f64> = m.iter().map(|x| x.abs()).collect();
    let num = xavg(&xavg(&xavg(&m, r), s), u);
    let den = xavg(&xavg(&xavg(&a, r), s), u);
    num.iter().zip(&den).map(|(n, d)| if *d != 0.0 { 100.0 * n / d } else { 0.0 }).collect()
}

fn sgn(x: f64) -> i8 { if x > 0.0 { 1 } else if x < 0.0 { -1 } else { 0 } }

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if from >= to || to >= x.r.len() { return None; }
    for (k, v) in p {
        let d = schema.iter().find(|d| d.key == k)?;
        if !v.is_finite() || v.fract() != 0.0 || *v < d.min || *v > d.max { return None; }
    }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap().def) as usize;
    let (r, sig, at_close) = (g("r"), g("sig"), g("fill") == 1);
    let erg = tsi(&x.c, r, sig, 1);
    let sl = xavg(&erg, sig);
    let (trend, warm) = if id.starts_with("wbmdd_erg_ema_") {
        (xavg(&x.c, g("trend")), (r + 2 * sig).max(g("trend")))
    } else {
        (tsi(&x.c, g("tr"), g("ts"), 1), (r + 2 * sig).max(g("tr") + g("ts")))
    };
    let sys = id.as_bytes()[id.len() - 1];
    let start = from.max(warm).max(1);
    let mut out = vec![];
    let mut cur: Option<Trade> = None;
    let fill = |i: usize| if at_close { (i, x.c[i]) } else { (i + 1, x.o[i + 1]) };
    // signals on the close of bar i; the last bar can only fill at its own close (at_close mode)
    let last = if at_close { to } else { to - 1 };
    for i in start..=last {
        let (ss, ts) = (sgn(sl[i] - sl[i - 1]), sgn(trend[i] - trend[i - 1]));
        let agree = if ss != 0 && ss == ts { ss } else { 0 };
        let side = sgn(erg[i] - sl[i]);
        let pos = cur.as_ref().map(|t| t.dir as i8).unwrap_or(0);
        let target: i8 = match sys {
            b'1' => if agree != 0 && side == agree { agree } else { 0 },
            b'3' => agree,
            _ => {
                // System 2
                let cross = sgn(erg[i] - sl[i]) != 0 && sgn(erg[i - 1] - sl[i - 1]) != sgn(erg[i] - sl[i]);
                let hold = pos != 0 && agree == pos && sgn(erg[i] - erg[i - 1]) != -pos;
                if hold { pos } else if agree != 0 && cross && side == agree { agree } else { 0 }
            }
        };
        if target == pos { continue; }
        let (fi, px) = fill(i);
        if let Some(mut t) = cur.take() {
            t.exit_i = fi; t.exit_px = px; t.signal_i = i;
            t.reason = if target != 0 { "Opposite alignment · reverse".into() } else { "Exit rule".into() };
            out.push(t);
        }
        if target != 0 {
            cur = Some(Trade { dir: target as f64, legs: vec![Leg { i: fi, px, w: 1.0 }], exit_i: fi, exit_px: px,
                reason: String::new(), entry_kind: if at_close { "close" } else { "open" }, signal_i: i, open: false, exits: vec![] });
        }
    }
    if let Some(mut t) = cur {
        t.exit_i = to; t.exit_px = x.c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true; out.push(t);
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn xavg_and_tsi_follow_tradestation() {
        let e = xavg(&[1.0, 2.0, 3.0], 3);
        assert_eq!(e, vec![1.0, 1.5, 2.25]);
        // monotone rising closes -> TSI = +100 after the first bar
        let c: Vec<f64> = (0..50).map(|i| 100.0 + i as f64).collect();
        let t = tsi(&c, 32, 5, 1);
        assert_eq!(t[0], 0.0);
        assert!((t[10] - 100.0).abs() < 1e-9);
    }
    #[test]
    fn system3_goes_long_in_an_uptrend_next_open_and_never_before_warmup() {
        let r: Vec<Row> = (0..120).map(|i| { let v = 100.0 + (i as f64) * 0.5 + if i % 2 == 0 { 0.3 } else { 0.0 };
            Row { ticker: "T".into(), date: format!("{:04}-01-01", 1900 + i), open: v, high: v + 1.0, low: v - 1.0, close: v, volume: None } }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let ts = run("wbmdd_erg_ema_s3", &x, 0, 119, &Params::new()).unwrap();
        assert!(!ts.is_empty());
        assert!(ts.iter().all(|t| t.dir == 1.0 && t.legs[0].i >= 43 && t.legs[0].px == x.o[t.legs[0].i]));
        assert!(run("wbmdd_dti_smi", &x, 0, 119, &Params::new()).is_none());
        assert!(run("wbmdd_erg_ema_s1", &x, 0, 119, &Params::from([("r".into(), 1000.0)])).is_none());
    }
}
