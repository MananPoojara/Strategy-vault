//! Platform-survey gap group "e" (2026-11-02): support/resistance and pivot items the lab did not have.
//! Computed: Sierra Chart Auto Retracement/Projection, Fibonacci Fans and Speed Resistance Lines (anchored causally on the last
//! confirmed swing pivots), Fibonacci Time Zones and the Fibonacci/Lucas Time Series (anchored on a start date), Support and
//! Resistance (last confirmed swing high / low), eSignal Pivot Points Moving Average, TraderEvolution PPMA and Round Numbers.
//! Listed only: Fibonacci Arcs and Gann Angles (drawings whose geometry depends on the chart's price/time scale), Ermanometry
//! (no primary rule text found) and Contract High/Low (needs each futures contract's own history). Formulas: see each definition.
use super::bars::Bars;
use super::catalogue::{int, len, n, p, real, IndicatorDef, Needs, Output, ParamSpec, Params, Placement};
use super::price::pivots;
use super::resample::period_key;
use super::ta::{self, NA};

#[allow(clippy::too_many_arguments)]
const fn def(id: &'static str, name: &'static str, category: &'static str, params: &'static [ParamSpec], outputs: &'static [&'static str], placement: Placement,
             needs: Needs, definition: &'static str, first: &'static [usize], sparse: bool, f: fn(&Bars, &Params) -> Output) -> IndicatorDef {
    IndicatorDef { id, name, category, params, outputs, placement, needs, definition, first, sparse, compute: Some(f), note: "" }
}
/// Listed in the picker, not computed (the app shows the note).
const fn listed(id: &'static str, name: &'static str, category: &'static str, needs: Needs, definition: &'static str, note: &'static str) -> IndicatorDef {
    IndicatorDef { id, name, category, params: &[], outputs: &["value"], placement: Placement::Pane, needs, definition,
                   first: &[usize::MAX], sparse: false, compute: None, note }
}
const NO: Needs = Needs::NONE;
const PR: Placement = Placement::Price;
const PN: Placement = Placement::Pane;
fn flag(b: bool) -> f64 { if b { 1. } else { 0. } }

// ---- swing-pivot anchored lines -------------------------------------------------------------------------------------
const LR: &[ParamSpec] = &[len("left", "Left bars", 5.), len("right", "Right bars", 5.)];
/// On each bar: the last confirmed pivot high and pivot low as (pivot bar, price); a pivot at p is known on bar p + right.
fn swings(b: &Bars, prm: &Params) -> Vec<(Option<(usize, f64)>, Option<(usize, f64)>)> {
    let (l, r) = (n(prm, "left"), n(prm, "right"));
    let (ph, pl) = (pivots(&b.high, l, r, true), pivots(&b.low, l, r, false));
    let (mut hi, mut lo) = (None, None);
    (0..b.len()).map(|t| {
        if !ph[t].is_nan() { hi = Some((t - r, ph[t])); }
        if !pl[t].is_nan() { lo = Some((t - r, pl[t])); }
        (hi, lo)
    }).collect()
}
fn support_resistance(b: &Bars, prm: &Params) -> Output {
    let s = swings(b, prm);
    Ok(vec![s.iter().map(|x| x.0.map_or(NA, |q| q.1)).collect(), s.iter().map(|x| x.1.map_or(NA, |q| q.1)).collect()])
}
/// Lines from the older swing point A through A + f·(E − A) at the newer swing point E's bar, extended to bar t.
fn swing_lines(b: &Bars, prm: &Params, fr: &[f64]) -> Output {
    let s = swings(b, prm);
    let mut o = vec![vec![NA; b.len()]; fr.len()];
    for (t, &(hi, lo)) in s.iter().enumerate() {
        let (Some(hi), Some(lo)) = (hi, lo) else { continue };
        if hi.0 == lo.0 { continue; } // one bar is both pivots: no direction
        let (a, e) = if hi.0 < lo.0 { (hi, lo) } else { (lo, hi) };
        let x = (t - a.0) as f64 / (e.0 - a.0) as f64;
        for (k, f) in fr.iter().enumerate() { o[k][t] = a.1 + f * (e.1 - a.1) * x; }
    }
    Ok(o)
}
fn speed_resistance_lines(b: &Bars, prm: &Params) -> Output { swing_lines(b, prm, &[1., 2. / 3., 1. / 3.]) }
fn fib_fans(b: &Bars, prm: &Params) -> Output { swing_lines(b, prm, &[1. - 0.382, 0.5, 1. - 0.618]) }

// ---- date-anchored time counts --------------------------------------------------------------------------------------
const ANCHOR: &[ParamSpec] = &[int("year", "Start year", 1900., 1800., 2100.), int("month", "Start month", 1., 1., 12.), int("day", "Start day", 1., 1., 31.)];
const FL: &[ParamSpec] = &[int("year", "Start year", 1900., 1800., 2100.), int("month", "Start month", 1., 1., 12.), int("day", "Start day", 1., 1., 31.),
    len("period", "EMA period", 21.)];
/// Bars since the first bar dated on or after the start date; NaN before it.
fn bars_since_date(b: &Bars, prm: &Params) -> Vec<f64> {
    let key = format!("{:04}-{:02}-{:02}", n(prm, "year"), n(prm, "month"), n(prm, "day"));
    let mut start = None;
    (0..b.len()).map(|t| {
        if start.is_none() && b.date[t].get(0..10).unwrap_or(b.date[t].as_str()) >= key.as_str() { start = Some(t); }
        start.map_or(NA, |s| (t - s) as f64)
    }).collect()
}
/// x is a member of a, c, a + c, … (Fibonacci from 1, 2; Lucas from 2, 1).
fn in_seq(x: usize, mut a: usize, mut c: usize) -> bool {
    loop {
        if a == x { return true; }
        if a > x && c > x { return false; }
        (a, c) = (c, a + c);
    }
}
fn fib_time_zones(b: &Bars, prm: &Params) -> Output {
    let k = bars_since_date(b, prm);
    let zone: Vec<f64> = k.iter().map(|&c| if c.is_nan() { NA } else { flag(c == 0. || in_seq(c as usize, 1, 2)) }).collect();
    let next: Vec<f64> = k.iter().map(|&c| {
        if c.is_nan() { return NA; }
        let (mut a, mut d) = (1usize, 2usize);
        while (a as f64) <= c { (a, d) = (d, a + d); }
        a as f64 - c
    }).collect();
    Ok(vec![zone, k, next])
}
fn fib_lucas_time_series(b: &Bars, prm: &Params) -> Output {
    let k = bars_since_date(b, prm);
    let f = |a: usize, c: usize| -> Vec<f64> { k.iter().map(|&x| if x.is_nan() { NA } else { flag(x > 0. && in_seq(x as usize, a, c)) }).collect() };
    Ok(vec![f(1, 2), f(2, 1), k.clone(), ta::ema(&b.close, n(prm, "period"))])
}

// ---- Sierra Chart Auto Retracement/Projection ------------------------------------------------------------------------
const ARP_LEVELS: [f64; 5] = [0.236, 0.382, 0.5, 0.618, 0.786];
const ARP: &[ParamSpec] = &[int("unit", "Time period (0 day, 1 week, 2 month, 3 quarter, 4 year)", 2., 0., 4.), int("method", "Method (0 retracement, 1 projection)", 0., 0., 1.),
    int("reference", "Reference (0 automatic, 1 high, 2 low)", 0., 0., 2.)];
fn auto_retracement_projection(b: &Bars, prm: &Params) -> Output {
    let (unit, proj, rf, m) = (n(prm, "unit"), n(prm, "method") == 1, n(prm, "reference"), b.len());
    let day = |t: usize| b.date[t].get(0..10).unwrap_or(b.date[t].as_str());
    let mut o = vec![vec![NA; m]; ARP_LEVELS.len() + 2];
    let (mut po, mut ph, mut pl) = (NA, NA, NA);
    for t in 0..m {
        let new = t == 0 || (if unit == 0 { day(t) != day(t - 1) } else { period_key(&b.date[t], unit) != period_key(&b.date[t - 1], unit) });
        if new { (po, ph, pl) = (b.open[t], b.high[t], b.low[t]); } else { ph = ph.max(b.high[t]); pl = pl.min(b.low[t]); }
        let r = ph - pl;
        let from_high = match rf { 1 => true, 2 => false, _ => b.close[t] >= po };
        for (k, q) in ARP_LEVELS.iter().enumerate() {
            o[k][t] = match (proj, from_high) { (false, true) => pl + q * r, (false, false) => ph - q * r, (true, true) => ph + q * r, (true, false) => pl - q * r };
        }
        let k = ARP_LEVELS.len();
        (o[k][t], o[k + 1][t]) = (ph, pl);
    }
    Ok(o)
}

// ---- pivot-point averages and round numbers --------------------------------------------------------------------------
const PPM: &[ParamSpec] = &[len("period", "Moving average period", 3.)];
fn pivot_points_ma(b: &Bars, prm: &Params) -> Output {
    let (k, m, pp) = (n(prm, "period"), b.len(), b.hlc3());
    let rng = |t: usize| b.high[t] - b.low[t];
    let raw: [Vec<f64>; 5] = [pp.clone(), (0..m).map(|t| 2. * pp[t] - b.low[t]).collect(), (0..m).map(|t| pp[t] + rng(t)).collect(),
        (0..m).map(|t| 2. * pp[t] - b.high[t]).collect(), (0..m).map(|t| pp[t] - rng(t)).collect()];
    // displacement 1: bar t shows the averages of the levels computed through bar t − 1
    Ok(raw.iter().map(|x| { let s = ta::sma(x, k); (0..m).map(|t| if t == 0 { NA } else { s[t - 1] }).collect::<Vec<f64>>() }).collect())
}
const PPMA: &[ParamSpec] = &[len("period", "Period", 9.)];
fn ppma(b: &Bars, prm: &Params) -> Output { Ok(vec![ta::sma(&b.hlc3(), n(prm, "period"))]) }
const RN: &[ParamSpec] = &[real("step", "Round-number step (price units)", 100., 0.0001, 1_000_000., 0.01)];
fn round_numbers(b: &Bars, prm: &Params) -> Output {
    let s = p(prm, "step");
    // shortcut: 1e-9 absorbs binary rounding of close/step (0.3/0.1 = 2.9999…); a close within 1e-9 steps below a level counts as on it
    let below: Vec<f64> = b.close.iter().map(|c| (c / s + 1e-9).floor() * s).collect();
    let above: Vec<f64> = below.iter().map(|x| x + s).collect();
    Ok(vec![below, above])
}

const SR: &str = "Bands & channels";
const DD: &str = "Drawings & display";
pub const DEFS: &[IndicatorDef] = &[
    def("auto_retracement_projection", "Auto Retracement/Projection (Sierra Chart)", SR, ARP, &["level_236", "level_382", "level_500", "level_618", "level_786", "high", "low"], PR, NO,
        "Sierra Chart Auto Retracement/Projection. The period is the calendar day, week (Monday–Sunday), month, quarter or year of the bar's date (unit 0–4, default month). On every bar the period's high H and low L so far (from its first bar up to this bar, never beyond) and R = H − L give one line per level q: Retracement with the High as reference = L + q·R; Retracement with the Low as reference = H − q·R (both as the Sierra page words them: percentages multiplied by the range are added to the Low, or subtracted from the High); Projection with the High as reference = H + q·R (the page: lines going upward from the period High); Projection with the Low as reference = L − q·R (adaptation: the page describes only the upward case; this mirrors it). Reference Automatic (default) compares this bar's close with the period's first open: close ≥ open → High, close < open → Low, so the lines can flip inside a period as the page says. high/low = H and L. Defined from the first bar. Adaptations: Sierra's 14 user-set Level percentages become five fixed outputs 0.236, 0.382, 0.5, 0.618, 0.786 (q = 0 and 1 are the low/high outputs); Time Period Unit Length is fixed at 1; Round to Nearest Tick Size, the evening-session reset and Number of Days to Calculate are not reproduced; Sierra's own default unit and percentages are not published on the page, so the lab defaults are month and the common Fibonacci set. Source: Sierra Chart Technical Studies Reference, Auto Retracement/Projection (https://www.sierrachart.com/index.php?page=doc/StudiesReference.php&ID=169).",
        &[0, 0, 0, 0, 0, 0, 0], false, auto_retracement_projection),
    def("fib_fans", "Fibonacci Fans (from confirmed swings)", SR, LR, &["fan_382", "fan_500", "fan_618"], PR, NO,
        "Fibonacci Fan lines (StockCharts ChartSchool): rising fan lines extend up from a trough and pass through the 38.2 %, 50 % and 61.8 % retracements of the advance (trough to peak) at the peak's bar; falling fan lines extend down from a peak through the retracements of the decline. Anchors (adaptation, so the lines are causal): the last confirmed pivot high and the last confirmed pivot low on bar t, TradingView ta.pivothigh/ta.pivotlow(left, right) (ties on the left allowed, ties on the right cancel; a pivot at bar p is known on bar p + right), defaults left = right = 5. A = the older of the two, E = the newer. fan_r on bar t = A.price + (1 − r)·(E.price − A.price)·(t − A.bar)/(E.bar − A.bar) for r = 0.382, 0.5, 0.618 (the line from A through E − r·(E − A) at E's bar). NaN until both pivots exist, and NaN while both are the same bar (one outside bar is both pivots, so there is no trend). The lines jump when a newer pivot is confirmed; StockCharts' user-placed trough/peak and its optional extra fan lines are not reproduced. Sources: StockCharts ChartSchool, Fibonacci Fans (https://chartschool.stockcharts.com/table-of-contents/chart-analysis/chart-annotation-tools/fibonacci-fans); StockCharts Help, Line Studies (https://help.stockcharts.com/charts-and-tools/sharpcharts/chartnotes/line-studies); pivots as the lab's pivot_high / pivot_low.",
        &[28, 28, 28], true, fib_fans),
    def("speed_resistance_lines", "Speed Resistance Lines (Gould, from confirmed swings)", SR, LR, &["line_1", "line_2_3", "line_1_3"], PR, NO,
        "Edson Gould's Speed Resistance Lines (StockCharts ChartSchool): the first line runs from the low to the high in an uptrend (high to low in a downtrend); the other two run from the same start to the 2/3 and 1/3 points of the move at the end point's bar (downtrend: 2/3 point = High − (High − Low)·2/3, 1/3 point = High − (High − Low)·1/3; uptrend mirrored above the low), and all extend to the right. Anchors (adaptation, so the lines are causal): the last confirmed pivot high and pivot low on bar t, ta.pivothigh/ta.pivotlow(left, right), defaults 5/5; A = the older, E = the newer. Line with fraction f on bar t = A.price + f·(E.price − A.price)·(t − A.bar)/(E.bar − A.bar): line_1 f = 1, line_2_3 f = 2/3, line_1_3 f = 1/3 (StockCharts writes 0.667 and 0.333; exact thirds here). NaN until both pivots exist and while both are the same bar. Sources: StockCharts ChartSchool, Speed Resistance Lines (https://chartschool.stockcharts.com/table-of-contents/chart-analysis/chart-annotation-tools/speed-resistance-lines); Gopalakrishnan, Speed Resistance Lines, TASC V.18:5 (https://store.traders.com/v16065speedr.html).",
        &[28, 28, 28], true, speed_resistance_lines),
    def("fib_time_zones", "Fibonacci Time Zones", "Cycles", ANCHOR, &["zone", "bars_since", "next_in"], PN, NO,
        "Fibonacci Time Zones: vertical lines at Fibonacci bar counts from a starting point (StockCharts: click to set the start; the spacing of the later lines is fixed; sequence 0, 1, 1, 2, 3, 5, 8, 13, 21, …). Start = the first bar dated on or after year-month-day (default 1900-01-01, i.e. the first bar of the data; set it to the swing high or low you would click). bars_since = bars from the start bar (0 on it). zone = 1 when bars_since is 0 or a Fibonacci number (1, 2, 3, 5, 8, 13, …), else 0. next_in = bars from this bar to the next zone line after it. NaN before the start bar. Causal: the start date is a parameter and each value uses only the bar count. Adaptation: the chart tool's start is a click; here it is a date. Sources: StockCharts ChartSchool, Fibonacci Time Zones (https://chartschool.stockcharts.com/table-of-contents/chart-analysis/chart-annotation-tools/fibonacci-time-zones); StockCharts Help, Line Studies (https://help.stockcharts.com/charts-and-tools/sharpcharts/chartnotes/line-studies).",
        &[0, 0, 0], false, fib_time_zones),
    def("fib_lucas_time_series", "Fibonacci Lucas Time Series (Coles)", "Cycles", FL, &["fib", "lucas", "bars_since", "ma"], PN, NO,
        "Fibonacci/Lucas Time Series (Andrew Coles, TASC August 2011; MotiveWave Fibonacci Lucas Time Series Indicator). Start = the first bar dated on or after year-month-day (MotiveWave: the bar matching the entered date and time; default here 1900-01-01, i.e. the first bar). exitNo = bars_since = bars from the start bar. fib = 1 when exitNo is a Fibonacci number (1, 2, 3, 5, 8, 13, 21, …), else 0; lucas = 1 when exitNo is a Lucas number (2, 1, 3, 4, 7, 11, 18, 29, …), else 0; the start bar itself (exitNo 0) is the entry and is 0 in both. ma = EMA(close, period), MotiveWave's plot (method EMA, period 21, input close), seeded with the SMA of the first period closes (lab ema). NaN before the start bar (ma from bar period − 1). Adaptations: the start is matched on the date only (no hour/minute) and on or after it rather than exactly; MotiveWave's bar colouring and signal messages become the 0/1 outputs. Sources: MotiveWave studies E–F, Fibonacci Lucas Time Series Indicator (https://docs.motivewave.com/studies/e-f#fibonacci-lucas-time-series-indicator); Traders' Tips, TASC August 2011, Coles 'Fibonacci And Lucas Automated Techniques For Intraday Traders' (https://traders.com/Documentation/FEEDbk_docs/2011/08/TradersTips.html).",
        &[0, 0, 0, 20], false, fib_lucas_time_series),
    def("support_resistance", "Support and Resistance (last confirmed swing)", SR, LR, &["resistance", "support"], PR, NO,
        "Support and resistance as StockCharts ChartSchool defines them: support is the level where demand is thought strong enough to stop a decline (established at prior lows), resistance the level where supply stops an advance (prior highs). Here: resistance = the price of the last confirmed pivot high, support = the price of the last confirmed pivot low, ta.pivothigh/ta.pivotlow(left, right) (ties on the left allowed, ties on the right cancel), defaults left = right = 5, each held until the next pivot of its kind is confirmed on bar p + right. NaN until the first pivot of each kind. Adaptation: the textbook levels are drawn by judgement (zones, several prior lows, round numbers, broken support turning into resistance); this keeps only the latest confirmed swing of each side. Sources: StockCharts ChartSchool, Support and Resistance (https://chartschool.stockcharts.com/table-of-contents/chart-analysis/support-and-resistance); pivots as the lab's pivot_high / pivot_low (TradingView ta.pivothigh/ta.pivotlow).",
        &[28, 25], false, support_resistance),
    def("pivot_points_ma", "Pivot Points Moving Average (eSignal)", "Periods & calendar", PPM, &["pivot", "r1", "r2", "s1", "s2"], PR, NO,
        "eSignal Pivot Points Moving Average: the floor pivot levels of each bar, each smoothed by a simple moving average and displaced one bar forward (the levels for today come from the previous bars' High, Low and Close). Per bar P = (H + L + C)/3; pivot = SMA(P, period); r1 = SMA(2·P − L); s1 = SMA(2·P − H); r2 = SMA(P + (H − L)); s2 = SMA(P − (H − L)); then each value is shown one bar later (PivDispl = 1). Defaults: period 3, displacement 1 (eSignal's defaults). First value on bar period (3). eSignal writes S1 = MA of ((PIVOT × 2) − High); PIVOT is read as the bar's P, matching R1. Designed for daily bars. Source: eSignal Help, Pivot Points Moving Average (https://download.esignal.com/products/workstation/help/charts/studies/Pivot_Points_Moving_Average.htm).",
        &[3, 3, 3, 3, 3], false, pivot_points_ma),
    def("ppma", "Pivot Point Moving Average (PPMA)", "Moving averages", PPMA, &["ppma"], PR, NO,
        "TraderEvolution PPMA (Pivot Point Moving Average): Pivot Point = (High + Low + Close)/3 (the typical price) and PPMA = SMA(Pivot Point, period). TraderEvolution publishes no default period; the lab default is 9 (the lab's sma default). First value on bar period − 1. Source: TraderEvolution Guide, PPMA (Pivot point moving average) (https://guide.traderevolution.com/project/desktop-application-for-windows/windows/technical-indicators/moving-averages/ppma-pivot-point-moving-average).",
        &[8], false, ppma),
    def("round_numbers", "Round Numbers", SR, RN, &["below", "above"], PR, NO,
        "Round-number (psychological) levels: prices on a fixed grid of step price units (big figures, $100 marks, thousand-point index levels). below = floor(close/step)·step, the grid level at or below the close; above = below + step, the next level above. Default step 100; set it for the instrument (e.g. 0.01 for FX big figures quoted to 4 decimals, 1 or 0.5 for low-priced stocks). close/step gets 1e-9 added before the floor so binary rounding (0.3/0.1 = 2.999…) does not drop a close that sits on a level. Defined from the first bar. Platforms draw the grid as horizontal lines; the two nearest levels are the rule-usable part. Sources: LuxAlgo Library, Round Numbers (https://www.luxalgo.com/library/concept/round-numbers/); Quantower Help, Round Numbers (https://help.quantower.com/analytics-panels/chart/technical-indicators/channels/round-numbers, no formula published).",
        &[0, 0], false, round_numbers),
    listed("fib_arcs", "Fibonacci Arcs", DD, NO,
        "Fibonacci Arcs (StockCharts ChartSchool): half circles centred on the end of a user-drawn Base Line (trough to peak after an advance, peak to trough after a decline) with radii 0.382, 0.5 and 0.618 of the Base Line's length. The radius is measured on the chart's screen, so where an arc crosses a bar depends on the chart's price/time scaling. Source: https://chartschool.stockcharts.com/table-of-contents/chart-analysis/chart-annotation-tools/fibonacci-arcs",
        "chart drawing/display — needs user-placed anchor points, and the arcs' radii depend on the chart's price/time scaling; not a bar-by-bar series; not usable in rules"),
    listed("gann_angles", "Gann Angles", DD, NO,
        "W. D. Gann's angles: lines drawn from a chosen high or low at fixed price-per-time ratios (1x1 = one price unit per time unit, plus 1x2, 2x1, 1x4, 4x1, 1x8, 8x1 …). The price unit per bar is a scale the analyst picks for each market (Gann's square chart); the sources give no default. Sources: https://en.wikipedia.org/wiki/William_Delbert_Gann; StockCharts Help, Line Studies (https://help.stockcharts.com/charts-and-tools/sharpcharts/chartnotes/line-studies)",
        "chart drawing/display — needs a user-placed anchor point and a user-chosen price-per-bar scale (the 1x1 unit); not a bar-by-bar series; not usable in rules"),
    listed("ermanometry", "Ermanometry", DD, NO,
        "Ermanometry: a ratio-projection drawing tool listed by one platform in the survey; no primary rule text or vendor help page was found (Sierra Chart's studies reference and drawing-tool pages do not describe it).",
        "not available yet — formula not published (no primary rule text found; a drawing tool on user-placed swing points)"),
    listed("contract_high_low", "Contract High/Low", "Derivatives market data", Needs { derivatives: true, ..Needs::NONE },
        "Contract Hi Low (Omega Research 1995; MotiveWave): the highest high and lowest low of the futures contract since it started trading. A continuous (rolled) series joins several contracts, so its all-data high/low is not any one contract's high/low. Source: MotiveWave studies C–D, Contract Hi Low (https://docs.motivewave.com/studies/c-d)",
        "not available yet — needs each futures contract's own price history since listing (a rolled continuous series mixes contracts)"),
];
/// (id, source, licence/notes), one per entry.
pub const SOURCES: &[(&str, &str, &str)] = &[
    ("auto_retracement_projection", "Sierra Chart: Auto Retracement/Projection", ""),
    ("fib_fans", "StockCharts: Fibonacci Fans (causal, from confirmed swings)", ""),
    ("speed_resistance_lines", "StockCharts: Speed Resistance Lines (causal, from confirmed swings)", ""),
    ("fib_time_zones", "StockCharts: Fibonacci Time Zones (date-anchored)", ""),
    ("fib_lucas_time_series", "MotiveWave: Fibonacci Lucas Time Series Indicator (Coles, TASC 2011-08)", ""),
    ("support_resistance", "StockCharts: Support and Resistance (last confirmed swing high/low)", ""),
    ("pivot_points_ma", "eSignal: Pivot Points Moving Average", ""),
    ("ppma", "TraderEvolution: PPMA (Pivot point moving average)", ""),
    ("round_numbers", "Quantower / LuxAlgo: Round Numbers", ""),
    ("fib_arcs", "StockCharts: Fibonacci Arcs (drawing)", ""),
    ("gann_angles", "Gann Angles (drawing)", ""),
    ("ermanometry", "Ermanometry (drawing; no primary source found)", ""),
    ("contract_high_low", "MotiveWave: Contract Hi Low", ""),
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lab::catalogue::{get, run};
    use crate::lab::testdata::{assert_series, compiles, from_ohlc, go, walk};
    const N: f64 = f64::NAN;
    fn hl(h: &[f64], l: &[f64]) -> Bars { from_ohlc(&h.iter().zip(l).map(|(&h, &l)| ((h + l) / 2., h, l, (h + l) / 2.)).collect::<Vec<_>>()) }
    const LR1: &[(&str, f64)] = &[("left", 1.), ("right", 1.)];

    #[test] fn swing_levels_and_lines() {
        // left = right = 1: pivot highs (1, 5) on bar 2 and (3, 4) on bar 4; pivot lows (2, 2) on bar 3 and (4, 1) on bar 5
        let b = hl(&[2., 5., 3., 4., 2., 3., 6.], &[1., 4., 2., 3., 1., 2., 5.]);
        let sr = go("support_resistance", &b, LR1);
        assert_series(&sr[0], &[N, N, 5., 5., 4., 4., 4.]);
        assert_series(&sr[1], &[N, N, N, 2., 2., 1., 1.]);
        // bar 3: A (1, 5) → E (2, 2), x = 2; bar 4: (2, 2) → (3, 4), x = 2; bars 5, 6: (3, 4) → (4, 1), x = 2, 3
        let line = |f: f64| vec![N, N, N, 5. - 6. * f, 2. + 4. * f, 4. - 6. * f, 4. - 9. * f];
        let s = go("speed_resistance_lines", &b, LR1);
        assert_series(&s[0], &[N, N, N, -1., 6., -2., -5.]);
        assert_series(&s[1], &line(2. / 3.));
        assert_series(&s[2], &line(1. / 3.));
        let f = go("fib_fans", &b, LR1);
        assert_series(&f[0], &line(0.618));
        assert_series(&f[1], &[N, N, N, 2., 4., 1., -0.5]);
        assert_series(&f[2], &line(0.382));
        // one outside bar that is both pivots: no direction
        let o = go("fib_fans", &hl(&[2., 5., 3.], &[1., 0., 1.]), LR1);
        assert!(o[0][2].is_nan());
    }
    #[test] fn time_counts_from_the_start_date() {
        let b = from_ohlc(&(1..=12).map(|i| { let x = i as f64; (x, x, x, x) }).collect::<Vec<_>>()); // dates 2024-01-01 ..= 2024-01-12
        let start = [("year", 2024.), ("month", 1.), ("day", 3.)];
        let z = go("fib_time_zones", &b, &start);
        assert_series(&z[0], &[N, N, 1., 1., 1., 1., 0., 1., 0., 0., 1., 0.]);
        assert_series(&z[1], &[N, N, 0., 1., 2., 3., 4., 5., 6., 7., 8., 9.]);
        assert_series(&z[2], &[N, N, 1., 1., 1., 2., 1., 3., 2., 1., 5., 4.]);
        let l = go("fib_lucas_time_series", &b, &[("year", 2024.), ("month", 1.), ("day", 3.), ("period", 3.)]);
        assert_series(&l[0], &[N, N, 0., 1., 1., 1., 0., 1., 0., 0., 1., 0.]);
        assert_series(&l[1], &[N, N, 0., 1., 1., 1., 1., 0., 0., 1., 0., 0.]);
        // EMA(3) of 1..12 seeded with SMA: close − 1
        assert_series(&l[3], &[N, N, 2., 3., 4., 5., 6., 7., 8., 9., 10., 11.]);
        // a start date before the data anchors on bar 0
        assert_eq!(go("fib_time_zones", &b, &[])[1][0], 0.);
    }
    #[test] fn auto_retracement_follows_the_period_range() {
        let mut b = from_ohlc(&[(10., 12., 9., 11.), (11., 13., 10., 10.5), (10., 11., 8., 9.)]);
        b.date = vec!["2024-01-30".into(), "2024-01-31".into(), "2024-02-01".into()];
        let o = go("auto_retracement_projection", &b, &[]);
        // Jan: open 10, close ≥ open → from High: L + q·R; Feb: open 10, close 9 → from Low: H − q·R
        assert_series(&o[2], &[10.5, 11., 9.5]);
        assert_series(&o[0], &[9. + 0.236 * 3., 9. + 0.236 * 4., 11. - 0.236 * 3.]);
        assert_series(&o[1], &[9. + 0.382 * 3., 9. + 0.382 * 4., 11. - 0.382 * 3.]);
        assert_series(&o[5], &[12., 13., 11.]);
        assert_series(&o[6], &[9., 9., 8.]);
        assert_series(&go("auto_retracement_projection", &b, &[("method", 1.), ("reference", 1.)])[2], &[13.5, 15., 12.5]);
        assert_series(&go("auto_retracement_projection", &b, &[("method", 1.), ("reference", 2.)])[2], &[7.5, 7., 6.5]);
        // day unit: bar 1 is its own period, open 11 > close 10.5 → H − 0.5·R
        assert_eq!(go("auto_retracement_projection", &b, &[("unit", 0.)])[2][1], 13. - 1.5);
    }
    #[test] fn pivot_averages() {
        // P = 10.5, 35/3, 12.5, 41/3; ranges all 3
        let b = from_ohlc(&[(10., 12., 9., 10.5), (11., 13., 10., 12.), (12., 14., 11., 12.5), (13., 15., 12., 14.)]);
        let o = go("pivot_points_ma", &b, &[("period", 2.)]);
        let (p1, p2) = ((10.5 + 35. / 3.) / 2., (35. / 3. + 12.5) / 2.);
        assert_series(&o[0], &[N, N, p1, p2]);
        assert_series(&o[1], &[N, N, (12. + 40. / 3.) / 2., (40. / 3. + 14.) / 2.]);
        assert_series(&o[2], &[N, N, p1 + 3., p2 + 3.]);
        assert_series(&o[3], &[N, N, (9. + 31. / 3.) / 2., (31. / 3. + 11.) / 2.]);
        assert_series(&o[4], &[N, N, p1 - 3., p2 - 3.]);
        assert_series(&go("ppma", &b, &[("period", 2.)])[0], &[N, p1, p2, (12.5 + 41. / 3.) / 2.]);
    }
    #[test] fn round_number_grid() {
        let b = from_ohlc(&[(1., 1., 1., 1017.3), (1., 1., 1., 1100.), (1., 1., 1., 99.99), (1., 1., 1., 0.3)]);
        let o = go("round_numbers", &b, &[]);
        assert_series(&o[0], &[1000., 1100., 0., 0.]);
        assert_series(&o[1], &[1100., 1200., 100., 100.]);
        assert_series(&go("round_numbers", &b, &[("step", 0.1)])[0], &[1017.3, 1100., 99.9, 0.3]);
    }
    #[test] fn sparse_lines_start_where_declared() {
        let bars = walk(1000, 7, true);
        for id in ["fib_fans", "speed_resistance_lines"] {
            let d = get(id).unwrap();
            for (k, s) in run(&d, &bars, &Params::new()).unwrap().iter().enumerate() { assert_eq!(s.iter().position(|v| !v.is_nan()), Some(d.first[k]), "{id}"); }
        }
    }
    #[test] fn rules_reach_gap_e() {
        for (id, out) in [("auto_retracement_projection", "level_618"), ("fib_fans", "fan_500"), ("speed_resistance_lines", "line_1_3"), ("fib_time_zones", "zone"),
            ("fib_lucas_time_series", "lucas"), ("support_resistance", "support"), ("pivot_points_ma", "s1"), ("ppma", "ppma"), ("round_numbers", "above")] {
            compiles(id, out).unwrap();
        }
    }
    #[test] fn listed_entries_carry_notes_and_every_entry_one_source() {
        for d in DEFS {
            assert_eq!(SOURCES.iter().filter(|s| s.0 == d.id).count(), 1, "{}", d.id);
            if d.compute.is_some() { continue; }
            assert!(!d.note.is_empty(), "{}", d.id);
            if d.needs.any_data() { assert!(d.note.starts_with("not available yet — needs "), "{}", d.id); }
        }
        assert_eq!(SOURCES.len(), DEFS.len());
        assert_eq!(DEFS.iter().filter(|d| d.compute.is_none()).count(), 4);
    }
}
