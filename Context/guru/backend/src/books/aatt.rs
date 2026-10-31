//! Andrew Abraham, "Trading the Trend", Technical Analysis of Stocks & Commodities V16:9 (1998) p.425-427, inside the
//! 55-page S&C compilation R:/Trading Knowledge III/!PDFs/Combine ADX and MACD. Detecting Trend Direction and Strenght.pdf
//! (pdf p.11-13); copy in Books/Andrew Abraham/. Book id "aatt". Notes and verification: library/aatt/README.md.
//! Live: the volatility indicator used "as a stop-and-reverse method". The wave-filtered trading plan and the
//! multiple-time-frame method are recorded as not testable.
use crate::connors::{true_range, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};
/// Admitted ids (authors/lanea/singles/decisions.json).
pub const REVIEWED_LANEA: &[&str] = &["aatt_vstop_sar"];
pub fn reviewed_lanea(id: &str) -> bool { REVIEWED_LANEA.contains(&id) }

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, dir, status, rules, notes| StrategyDef { id, name, group: "Trading the Trend (S&C V16:9, 1998)", session: 0, dir, status,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "aatt", rules, notes, claim: None };
    vec![
        d("aatt_vstop_sar", "Volatility indicator stop-and-reverse (3 x 21-bar weighted average true range)", "Long/Short", Status::Live,
          &["F|\"The calculation uses a 21-period weighted average of the true range, giving higher weight to the true range of the most recent bar. The final value is then multiplied by 3.\" (p.425). Weighted average = TradeStation WAverage (linear weights 21..1; the article's charts are TradeStation). True range = Wilder's (p.425 lists the three ranges).",
            "E|\"The volatility indicator is used as a stop-and-reverse method. Let's say the market has been rising, then the volatility indicator is calculated each day and subtracted from the highest close during the rising market. The highest close is always used, even if there has been a series of lower closes since the highest close.\" (p.425)",
            "E|\"If the market closes below the volatility indicator, then for the next day, the current reading of the volatility indicator is added to the lowest close. This step is followed each day until the market closes above the trailing volatility indicator.\" (p.425)",
            "X|Reverse on every flip: long when the close crosses above the down-trend line (lowest close + 3 x WATR), short when it closes below the up-trend line (highest close - 3 x WATR). Line for day t uses that day's WATR and the extreme close including day t; strictly below / above. [Book silent on the same-day vs prior-day value]",
            "E|Fill at the next open after the reversing close (TradeStation default, p.fill=0); p.fill=1 fills at the reversing close. [Book silent on the fill]. Start: the first bar with a 21-bar average opens an up-trend with highest close = that close; no position until the first reversal. [Book silent on the start]"],
          "The article adds a wave filter to this (see aatt_wave_plan, not testable); this entry tests only the stop-and-reverse method the article states for the indicator. No published results. Run on the index instead of futures/stocks."),
        d("aatt_wave_plan", "Trend-filtered wave entries (prior wave broken)", "Long/Short",
          Status::NotTestable("the entry rule is not complete or consistent: the corn example requires 'the prior low was broken' (Nov) and 'the lows of February were broken' (Mar), but the January short and the Swiss franc June short are taken on the indicator flip alone; the order type and level for the break are not given; how the trend definition changes after an 'alert' is left to the next wave without a stated switch rule; the starting trend is not defined"),
          &["F|\"uptrends are made up of waves of higher highs, with prior lows not being surpassed. Conversely, downtrends are composed of waves of lower lows and prior highs not being surpassed.\" (p.425)",
            "E|\"During November, the volatility indicator reversed trend, and the prior low was broken. This was our signal to go short. Our exit signal will be the volatility indicator turning positive.\" (p.426)",
            "E|\"since the rally's high beginning in January did not surpass the highs of October, our second definition of an uptrend was not met. As a result, we went short again when the volatility indicator went negative.\" (p.426)",
            "F|\"We will use the lowest low while the volatility indicator signals an uptrend for our low point. This is just an alert ... If the next upwave surpasses the prior upwave, our trend is intact and our alert turned off.\" (p.426)"],
          "Recorded so the page shows it; no code."),
        d("aatt_multi_tf", "Multiple time frames (5-min trend, 1-min trades)", "Long/Short",
          Status::NotTestable("intraday bars, and the entry ('prepare a buy-stop to pull him in the market if an upwave becomes present') has no stated level"),
          &["F|\"We look to a higher time frame to identify the trend and only want to trade in that direction.\" (p.427)",
            "E|\"if on the 15-minute chart he is in an uptrend and identifies blue upwaves, he would go down to his five-minute chart, identify a red downwave and prepare a buy-stop to pull him in the market if an upwave becomes present.\" (p.427)"],
          "Needs intraday data and an unstated buy-stop level."),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "aatt_vstop_sar" => ("aatt_vstop", "Abraham volatility indicator", "Stop-and-reverse"),
        "aatt_wave_plan" => ("aatt_vstop", "Abraham volatility indicator", "Wave filter (not testable)"),
        "aatt_multi_tf" => ("aatt_vstop", "Abraham volatility indicator", "Multiple time frames (not testable)"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    (id == "aatt_vstop_sar").then(|| vec![
        p("len", "Weighted average length of true range", 21., 5., 50., 1.),
        p("mult", "Multiplier", 3., 1., 5., 0.25),
        p("fill", "0 = next open (TradeStation default), 1 = reversing close", 0., 0., 1., 1.),
    ])
}

/// TradeStation WAverage: linear weights len (newest) .. 1 (oldest).
fn wma(a: &[f64], n: usize) -> Vec<f64> {
    let den = (n * (n + 1) / 2) as f64;
    (0..a.len()).map(|i| if i + 1 < n { f64::NAN } else { (0..n).map(|k| a[i - k] * (n - k) as f64).sum::<f64>() / den }).collect()
}

/// +1 / -1 on the bar whose close flips the trend, 0 otherwise.
pub fn flips(x: &Ctx, len: usize, mult: f64) -> Vec<i8> {
    let v = wma(&true_range(x.r), len);
    let mut out = vec![0i8; x.c.len()];
    let (mut trend, mut ext) = (0i8, 0.);
    for i in 0..x.c.len() {
        if !v[i].is_finite() { continue; }
        let (c, s) = (x.c[i], mult * v[i]);
        match trend {
            0 => { trend = 1; ext = c; }
            1 => { ext = f64::max(ext, c); if c < ext - s { trend = -1; ext = c; out[i] = -1; } }
            _ => { ext = f64::min(ext, c); if c > ext + s { trend = 1; ext = c; out[i] = 1; } }
        }
    }
    out
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if from > to || to >= x.r.len() { return None; }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap().def);
    let (len, mult, at_close) = (g("len").round().max(1.) as usize, g("mult"), g("fill") >= 0.5);
    let f = flips(x, len, mult);
    let mut out = vec![];
    let mut cur: Option<Trade> = None;
    for i in from..=to {
        let want = f[i] as f64;
        if want == 0. || cur.as_ref().is_some_and(|t| t.dir == want) { continue; }
        let (j, px) = if at_close { (i, x.c[i]) } else if i < to { (i + 1, x.o[i + 1]) } else { break };
        if let Some(mut t) = cur.take() {
            t.exit_i = j; t.exit_px = px; t.signal_i = i;
            t.reason = format!("close {} volatility stop: reverse", if want > 0. { "above" } else { "below" });
            out.push(t);
        }
        cur = Some(Trade { dir: want, legs: vec![Leg { i: j, px, w: 1. }], exit_i: j, exit_px: px, reason: String::new(),
            entry_kind: if at_close { "close" } else { "open" }, signal_i: i, open: false, exits: vec![] });
    }
    if let Some(mut t) = cur {
        t.exit_i = to; t.exit_px = x.c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true;
        out.push(t);
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn wma_is_linear_weighted() {
        let w = wma(&[1., 2., 3.], 3);
        assert!((w[2] - (1. + 4. + 9.) / 6.).abs() < 1e-12);
    }
    #[test]
    fn flips_down_when_close_breaks_highest_close_minus_3_watr() {
        // bars 0-39: true range 2; bar 40 (116): TR 4.5, line 119.5-6.68 = 112.82 < 116, no flip
        let mut cl: Vec<f64> = (0..40).map(|i| 100. + i as f64 * 0.5).collect(); // up to 119.5
        cl.extend([116., 110.]); // bar 41: TR 7 -> WATR (2*190+4.5*20+7*21)/231 = 2.671, line 119.5-8.01 = 111.49 > 110 -> flip
        let rows: Vec<Row> = cl.iter().enumerate().map(|(i, &c)| Row { ticker: "T".into(), date: format!("{i:05}"), open: c, high: c + 1., low: c - 1., close: c, volume: None }).collect();
        let x = Ctx::new(&rows, &rows, &[]);
        let f = flips(&x, 21, 3.);
        assert_eq!(f[40], 0);
        assert_eq!(f[41], -1);
    }
}

// Instrument admission: Andrew Abraham (aatt, lane A) 2026-10-26 (lane B) block, moved from instrument_policy.rs on 31 Oct 2026 (lane ST).
// Admission only for the module REVIEWED lists (authors/lanea/singles/decisions.json); kept in one block to merge.
// Runs on actual indices, ETFs and stocks, US and India (index_etf_or_stock, nifty50 basket, declared application).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book=="aatt" && crate::books::aatt::reviewed_lanea(def.id),
    label: "Abraham's volatility stop-and-reverse was written for futures and stocks (corn, Swiss franc, Chase Manhattan); Runs on indices, ETFs and single stocks (US and India) are a declared application, one unit per trade.",
    basis: "Andrew Abraham / Trading the Trend, Technical Analysis of Stocks & Commodities V16:9 (1998)",
};
