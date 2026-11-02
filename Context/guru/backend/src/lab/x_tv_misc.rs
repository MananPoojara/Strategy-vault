//! TradingView built-in indicators the lab did not have (2026-10-30 gap check against TradingView's built-in list).
//! Computed here: a causal VWAP Auto Anchored (price/volume anchors) and TradingView's Seasonality built-in (causal as
//! published). Not added, already in the catalogue: Net Volume (listed net_volume = the current lower-timeframe built-in;
//! the older daily form is bar_up_down_volume.net, the footprint form up_down_volume.delta), Relative Volume at Time
//! (listed rvol_at_time), Visible Average Price (listed visible_avg_price) and Bollinger Bars (listed bollinger_bars: it
//! only redraws open/high/low/close). Formulas: see each definition string.
use super::bars::Bars;
use super::catalogue::{int, n, p, real, IndicatorDef, Needs, Output, ParamSpec, Params, Placement};
use super::ta::NA;

#[allow(clippy::too_many_arguments)]
const fn def(id: &'static str, name: &'static str, category: &'static str, params: &'static [ParamSpec], outputs: &'static [&'static str], placement: Placement,
             needs: Needs, definition: &'static str, first: &'static [usize], sparse: bool, f: fn(&Bars, &Params) -> Output) -> IndicatorDef {
    IndicatorDef { id, name, category, params, outputs, placement, needs, definition, first, sparse, compute: Some(f), note: "" }
}
const VOL: Needs = Needs { volume: true, ..Needs::NONE };

const VA: &[ParamSpec] = &[int("anchor", "Anchor (0 highest high, 1 lowest low, 2 highest volume)", 0., 0., 2.), int("length", "Length (bars searched for the anchor)", 100., 1., 2000.),
    real("mult", "Bands multiplier", 1., 0.1, 10., 0.1)];
/// At each bar t: anchor = the bar of the highest high / lowest low / highest volume among bars t − length + 1 ..= t (ties → the
/// most recent), then the volume-weighted hlc3 and its standard deviation from that anchor to t.
fn vwap_auto_anchored(b: &Bars, prm: &Params) -> Output {
    let (k, which, m, len) = (n(prm, "length"), n(prm, "anchor"), p(prm, "mult"), b.len());
    let (v, src) = (b.volume.as_deref().unwrap_or(&[]), b.hlc3());
    let key: &[f64] = match which { 0 => &b.high, 1 => &b.low, _ => v };
    let mut o = vec![vec![NA; len]; 3];
    for t in (k.max(1) - 1)..len {
        let mut a = t; // strict comparison scanning back from t keeps the most recent bar on ties
        for j in (t + 1 - k..t).rev() { if (which == 1 && key[j] < key[a]) || (which != 1 && key[j] > key[a]) { a = j; } }
        let (mut sv, mut spv, mut spv2) = (0., 0., 0.);
        for i in a..=t { sv += v[i]; spv += src[i] * v[i]; spv2 += src[i] * src[i] * v[i]; }
        if sv == 0. { continue; }
        let x = spv / sv;
        let sd = (spv2 / sv - x * x).max(0.).sqrt();
        (o[0][t], o[1][t], o[2][t]) = (x, x + m * sd, x - m * sd);
    }
    Ok(o)
}

const SE: &[ParamSpec] = &[int("start_year", "Starting year", 2015., 1800., 2100.)];
fn ym(d: &str) -> (i64, usize) { (d.get(0..4).and_then(|s| s.parse().ok()).unwrap_or(0), d.get(5..7).and_then(|s| s.parse().ok()).unwrap_or(0)) }
/// TradingView Seasonality: on the first bar of each month, the mean / sample stdev / count of the same calendar month's
/// close-to-close % change in earlier completed months (years ≥ start_year), held through the month.
fn seasonality(b: &Bars, prm: &Params) -> Output {
    let (start, len) = (p(prm, "start_year") as i64, b.len());
    let mut o = vec![vec![NA; len]; 4]; // expected_price, average, stdev, count
    let mut hist: [Vec<f64>; 13] = Default::default(); // completed monthly % changes per calendar month (1..=12)
    let mut prev_month_close = NA; // close of the month before the one that just ended
    for t in 1..len {
        let (cur, last) = (ym(&b.date[t]), ym(&b.date[t - 1]));
        if cur != last {
            // the month of bar t − 1 is complete: its change is its last close over the previous month's last close
            let ch = 100. * (b.close[t - 1] - prev_month_close) / prev_month_close;
            if last.0 >= start && ch.is_finite() { hist[last.1].push(ch); }
            prev_month_close = b.close[t - 1];
            let h = &hist[cur.1.min(12)];
            let c = h.len() as f64;
            let avg = if h.is_empty() { NA } else { h.iter().sum::<f64>() / c };
            let sd = if h.len() < 2 { NA } else { (h.iter().map(|x| (x - avg).powi(2)).sum::<f64>() / (c - 1.)).sqrt() };
            (o[0][t], o[1][t], o[2][t], o[3][t]) = (b.close[t - 1] * (1. + avg / 100.), avg, sd, c);
        } else {
            for s in o.iter_mut() { s[t] = s[t - 1]; }
        }
    }
    Ok(o)
}

pub const DEFS: &[IndicatorDef] = &[
    def("vwap_auto_anchored", "VWAP Auto Anchored (causal)", "Volume", VA, &["vwap", "upper", "lower"], Placement::Price, VOL,
        "TradingView VWAP Auto Anchored, Highest High / Lowest Low / Highest Volume anchor periods, computed again on every bar. On bar t the anchor is the bar with the highest high (anchor 0), lowest low (anchor 1) or highest volume (anchor 2) among the last length bars t − length + 1 ..= t. TradingView compares high (or low) whatever the Source is. Ties go to the most recent bar. Source = hlc3 (TradingView's default). vwap = Σ(hlc3·v)/Σv from the anchor to t. sd = √max(Σ(v·hlc3²)/Σv − vwap², 0), and upper/lower = vwap ± mult·sd (the vwap_period band formula, as in TradingView's ta.vwap). If Σv = 0 the outputs are NaN. Blank before bar length − 1, when the window is not yet full. Adaptation: TradingView draws one VWAP from the anchor found on the last chart bar and redraws it when a newer anchor appears, so its history looks ahead. Here each bar uses the anchor visible on that bar, so the value is causal and the line jumps when the anchor moves. TradingView does not publish a default Length (the indicator is not written in Pine). The lab default is 100, from the help-page example. Only price/volume anchors are offered. Earnings, Dividends and Splits anchors need corporate-event dates the lab does not hold, and the calendar anchors (week, month, quarter, year) are vwap_period. Percentage bands (vwap·(1 ± mult/100)) and band sets #2/#3 are not separate outputs: run it again with another mult. Sources: TradingView Help Center, VWAP Auto Anchored (https://www.tradingview.com/support/solutions/43000652199-vwap-auto-anchored/); TradingView blog, VWAP improvements across the board (https://tradingview.com/blog/en/vwap-improvements-across-the-board-28974).",
        &[99, 99, 99], false, vwap_auto_anchored),
    def("seasonality", "Seasonality (TradingView, monthly)", "Periods & calendar", SE, &["expected_price", "average", "stdev", "count"], Placement::Pane, Needs::NONE,
        "TradingView built-in Seasonality (Pine v6). Months follow the bar dates, and a month's close is the close of its last bar. A month's change is 100·(its close − the previous month's close)/the previous month's close, so the data's first month has none. A change is stored under its calendar month when its year ≥ start_year. On the first bar of each new month (month or year of the date differs from the previous bar): average = the mean of the stored changes for this calendar month, from earlier years only; stdev = their sample standard deviation (n − 1, Pine array.stdev(biased = false); NaN for fewer than 2); count = how many there are; expected_price = previous bar's close·(1 + average/100). If none are stored, average and expected_price are NaN and count is 0. All four values are held until the next month starts, and they are NaN through the data's first month. This is causal as published: TradingView reads closed months through request.security(\"1M\", lookahead_on) with a one-month offset. Adaptation: TradingView's Ignored months input and the heatmap table / projection boxes (display only) are not reproduced. This is not TradingView's Seasonals chart panel, which overlays each past year's whole price path, including bars after today's date, and so cannot be used in rules. Sources: TradingView built-in Seasonality Pine v6 source (indicator(\"Seasonality\"), calculateMontlyChanges and the currMonthAverage / currMonthStDev / currMonthExpectedPrice plots; built-in copy mirrored at github.com/0xpoot-poot/ml-trading indicators/tradingview_pinescript/visual_drawing_tools/seasonality.pine); TradingView Help Center, Seasonality (https://www.tradingview.com/support/solutions/43000723025-seasonality/: starting year, expected price = previous month's close moved by the month's average change, values as of the month's opening bar); TradingView Help Center, Seasonals (https://www.tradingview.com/support/solutions/43000745201-seasonals/).",
        &[X, X, X, 31], true, seasonality),
];
const X: usize = usize::MAX;
/// (id, source, licence/notes), one per entry.
pub const SOURCES: &[(&str, &str, &str)] = &[
    ("vwap_auto_anchored", "VWAP Auto Anchored (Highest High / Lowest Low / Highest Volume, causal)", ""),
    ("seasonality", "Seasonality", ""),
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lab::catalogue::run;
    use crate::lab::testdata::{assert_series, compiles, go, walk};
    const N: f64 = f64::NAN;
    fn bars(rows: &[(&str, f64, f64, f64, f64)]) -> Bars {
        let mut b = Bars::default();
        for (i, &(d, h, l, c, v)) in rows.iter().enumerate() {
            b.date.push(d.into()); b.open.push(c); b.high.push(h); b.low.push(l); b.close.push(c); b.end.push(i);
            b.volume.get_or_insert_with(Vec::new).push(v);
        }
        b
    }

    #[test] fn vwap_restarts_at_the_window_extreme() {
        // hlc3 = 2, 3, 5, 4, 3; highest high in 3 bars: bar 2 (h 6) on bars 2–4
        let b = bars(&[("2024-01-01", 3., 1., 2., 1.), ("2024-01-02", 4., 2., 3., 1.), ("2024-01-03", 6., 4., 5., 2.), ("2024-01-04", 5., 3., 4., 2.), ("2024-01-05", 4., 2., 3., 1.)]);
        let o = go("vwap_auto_anchored", &b, &[("length", 3.)]);
        assert_series(&o[0], &[N, N, 5., (10. + 8.) / 4., (10. + 8. + 3.) / 5.]);
        // bar 3: values 5, 4 weights 2, 2 → mean 4.5, sd 0.5
        assert_series(&o[1], &[N, N, 5., 5., (21. / 5.) + ((50. + 32. + 9.) / 5. - (21. / 5f64).powi(2)).sqrt()]);
        // lowest low: bar 0 (low 1) on bar 2, bar 1 (low 2) on bar 3, bar 4 (low 2) on bar 4
        assert_series(&go("vwap_auto_anchored", &b, &[("length", 3.), ("anchor", 1.)])[0], &[N, N, (2. + 3. + 10.) / 4., (3. + 10. + 8.) / 5., 3.]);
        // highest volume: bars 2 and 3 tie at 2 → bar 3 from bar 3 on
        assert_series(&go("vwap_auto_anchored", &b, &[("length", 3.), ("anchor", 2.)])[0], &[N, N, 5., 4., (8. + 3.) / 3.]);
    }
    #[test] fn seasonality_averages_the_same_month_of_earlier_years() {
        let b = bars(&[("2022-12-30", 1., 1., 100., 1.), ("2023-01-31", 1., 1., 110., 1.), ("2023-02-01", 1., 1., 111., 1.), ("2023-12-29", 1., 1., 120., 1.),
            ("2024-01-02", 1., 1., 121., 1.), ("2024-01-31", 1., 1., 132., 1.), ("2024-02-01", 1., 1., 133., 1.), ("2025-01-02", 1., 1., 140., 1.)]);
        let o = go("seasonality", &b, &[]);
        // Jan 2023 = +10 % (stored on 2023-02-01), Feb 2023 = 111/110 (stored on 2023-12-29), Jan 2024 = +10 %
        let feb = 100. / 110.;
        assert_series(&o[3], &[N, 0., 0., 0., 1., 1., 1., 2.]);
        assert_series(&o[1], &[N, N, N, N, 10., 10., feb, 10.]);
        assert_series(&o[0], &[N, N, N, N, 132., 132., 132. * (1. + feb / 100.), 133. * 1.1]);
        assert_series(&o[2], &[N, N, N, N, N, N, N, 0.]);
        // a start year after the data stores nothing
        assert_series(&go("seasonality", &b, &[("start_year", 2025.)])[3], &[N, 0., 0., 0., 0., 0., 0., 0.]);
    }
    #[test] fn rules_reach_tv_misc() { for (id, out) in [("vwap_auto_anchored", "lower"), ("seasonality", "count")] { compiles(id, out).unwrap(); } }
    #[test] fn defaults_on_a_walk() {
        let o = run(&crate::lab::catalogue::get("seasonality").unwrap(), &walk(1000, 7, true), &Params::new()).unwrap();
        // walk starts 2020-01-01 (one bar per calendar day): Feb 2021 is the first month with an earlier Feb change
        assert_eq!(o[1].iter().position(|v| !v.is_nan()), Some(397));
    }

    /// TV_BARS=<csv date,open,high,low,close,volume> TV_OUT=<json> cargo test --bin guru lab::x_tv_misc::tests::tv_misc_dump -- --ignored --exact
    #[test] #[ignore] fn tv_misc_dump() {
        let text = std::fs::read_to_string(std::env::var("TV_BARS").expect("TV_BARS")).expect("read TV_BARS");
        let (mut b, mut vol) = (Bars::default(), Vec::new());
        for (i, line) in text.lines().skip(1).filter(|s| !s.trim().is_empty()).enumerate() {
            let f: Vec<&str> = line.split(',').map(str::trim).collect();
            let x = |k: usize| f[k].parse::<f64>().unwrap();
            b.date.push(f[0].to_string());
            b.open.push(x(1)); b.high.push(x(2)); b.low.push(x(3)); b.close.push(x(4)); vol.push(x(5)); b.end.push(i);
        }
        b.volume = Some(vol);
        let num = |v: f64| if v.is_finite() { format!("{v:?}") } else { "null".into() };
        let cases = [("vwap_auto_anchored", 0.), ("vwap_auto_anchored", 1.), ("vwap_auto_anchored", 2.), ("seasonality", 0.)];
        let body: Vec<String> = cases.iter().map(|&(id, a)| {
            let d = crate::lab::catalogue::get(id).unwrap();
            let prm: Params = if id.starts_with("vwap") { [("anchor".to_string(), a), ("length".to_string(), 20.)].into() } else { [("start_year".to_string(), 2000.)].into() };
            let out = run(&d, &b, &prm).unwrap();
            let cols: Vec<String> = d.outputs.iter().zip(&out).map(|(k, s)| format!("\"{k}\":[{}]", s.iter().map(|&v| num(v)).collect::<Vec<_>>().join(","))).collect();
            format!("\"{id}#{a}\":{{{}}}", cols.join(","))
        }).collect();
        std::fs::write(std::env::var("TV_OUT").expect("TV_OUT"), format!("{{{}}}", body.join(","))).expect("write TV_OUT");
    }
}
