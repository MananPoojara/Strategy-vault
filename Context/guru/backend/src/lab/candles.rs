//! Candlestick patterns (TradingView built-ins). Formulas: TradingView "All Candlestick Patterns" (Pine v4), Decision 20.
use super::bars::Bars;
#[allow(unused_imports)]
use super::catalogue::{get, int, n, p, run, IndicatorDef, Needs, Output, ParamSpec, Params, Placement};
use super::ta;

const CAT: &str = "Candlestick patterns";
const TREND: &[ParamSpec] = &[int("trend", "Trend rule (0 SMA50, 1 SMA50+SMA200, 2 none)", 0., 0., 2.)];

/// TradingView C_* candle features per bar; booleans are false where an input is missing, as Pine bools are.
pub(super) struct F {
    up: Vec<bool>, dn: Vec<bool>, bh: Vec<f64>, bl: Vec<f64>, body: Vec<f64>, mid: Vec<f64>, avg: Vec<f64>, us: Vec<f64>, ds: Vec<f64>, rng: Vec<f64>,
    small: Vec<bool>, long: Vec<bool>, hus: Vec<bool>, hds: Vec<bool>, white: Vec<bool>, black: Vec<bool>, dojib: Vec<bool>, doji: Vec<bool>, hl2: Vec<f64>,
}
pub(super) fn feats(b: &Bars, trend: usize) -> F {
    let (o, h, l, c, len) = (&b.open, &b.high, &b.low, &b.close, b.len());
    let (s50, s200) = (ta::sma(c, 50), ta::sma(c, 200));
    let (up, dn): (Vec<bool>, Vec<bool>) = (0..len).map(|i| match trend {
        0 => (c[i] > s50[i], c[i] < s50[i]),
        1 => (c[i] > s50[i] && s50[i] > s200[i], c[i] < s50[i] && s50[i] < s200[i]),
        _ => (true, true),
    }).unzip();
    let bh: Vec<f64> = (0..len).map(|i| o[i].max(c[i])).collect();
    let bl: Vec<f64> = (0..len).map(|i| o[i].min(c[i])).collect();
    let body: Vec<f64> = (0..len).map(|i| bh[i] - bl[i]).collect();
    let avg = ta::ema(&body, 14);
    let us: Vec<f64> = (0..len).map(|i| h[i] - bh[i]).collect();
    let ds: Vec<f64> = (0..len).map(|i| bl[i] - l[i]).collect();
    let rng: Vec<f64> = (0..len).map(|i| h[i] - l[i]).collect();
    let dojib: Vec<bool> = (0..len).map(|i| rng[i] > 0. && body[i] <= rng[i] * 0.05).collect();
    // x/0 is inf or NaN in Rust and na in Pine: both make `< 100` false
    let doji: Vec<bool> = (0..len).map(|i| dojib[i] && (us[i] == ds[i] || ((us[i] - ds[i]).abs() / ds[i] * 100. < 100. && (ds[i] - us[i]).abs() / us[i] * 100. < 100.))).collect();
    F { small: (0..len).map(|i| body[i] < avg[i]).collect(), long: (0..len).map(|i| body[i] > avg[i]).collect(),
        hus: (0..len).map(|i| us[i] > 0.05 * body[i]).collect(), hds: (0..len).map(|i| ds[i] > 0.05 * body[i]).collect(),
        white: (0..len).map(|i| o[i] < c[i]).collect(), black: (0..len).map(|i| o[i] > c[i]).collect(),
        mid: (0..len).map(|i| body[i] / 2. + bl[i]).collect(), hl2: b.hl2(), up, dn, bh, bl, body, avg, us, ds, rng, dojib, doji }
}
fn ham(f: &F, i: usize) -> bool { f.small[i] && f.body[i] > 0. && f.bl[i] > f.hl2[i] && f.ds[i] >= 2. * f.body[i] && !f.hus[i] }
fn inv(f: &F, i: usize) -> bool { f.small[i] && f.body[i] > 0. && f.bh[i] < f.hl2[i] && f.us[i] >= 2. * f.body[i] && !f.hds[i] }
fn maru(f: &F, i: usize) -> bool { f.long[i] && f.us[i] <= 0.05 * f.body[i] && f.ds[i] <= 0.05 * f.body[i] }
fn spin(f: &F, i: usize) -> bool { f.ds[i] >= f.rng[i] * 0.34 && f.us[i] >= f.rng[i] * 0.34 && !f.dojib[i] }
fn drag(f: &F, i: usize) -> bool { f.dojib[i] && f.us[i] <= f.body[i] }
fn grav(f: &F, i: usize) -> bool { f.dojib[i] && f.ds[i] <= f.body[i] }

/// One compute fn per pattern: 1 where `$e` holds on bar `$i`, else 0. `trend` patterns read the `trend` parameter.
macro_rules! pattern {
    ($name:ident, trend, |$f:ident, $b:ident, $i:ident| $e:expr) => {
        #[allow(unused_variables)]
        fn $name(b: &Bars, prm: &Params) -> Output { let $f = feats(b, n(prm, "trend")); let $b = b; Ok(vec![(0..b.len()).map(|$i| if $e { 1. } else { 0. }).collect()]) }
    };
    ($name:ident, |$f:ident, $b:ident, $i:ident| $e:expr) => {
        #[allow(unused_variables)]
        fn $name(b: &Bars, _: &Params) -> Output { let $f = feats(b, 2); let $b = b; Ok(vec![(0..b.len()).map(|$i| if $e { 1. } else { 0. }).collect()]) }
    };
}
pattern!(dark_cloud_cover, trend, |f, b, i| i >= 1 && f.up[i - 1] && f.white[i - 1] && f.long[i - 1] && f.black[i] && b.open[i] >= b.high[i - 1] && b.close[i] < f.mid[i - 1] && b.close[i] > b.open[i - 1]);
pattern!(doji, |f, b, i| f.doji[i] && !drag(&f, i) && !grav(&f, i));
pattern!(doji_star_bear, trend, |f, b, i| i >= 1 && f.up[i] && f.white[i - 1] && f.long[i - 1] && f.dojib[i] && f.bl[i] > f.bh[i - 1]);
pattern!(doji_star_bull, trend, |f, b, i| i >= 1 && f.dn[i] && f.black[i - 1] && f.long[i - 1] && f.dojib[i] && f.bh[i] < f.bl[i - 1]);
pattern!(dragonfly_doji, |f, b, i| drag(&f, i));
pattern!(engulfing_bear, trend, |f, b, i| i >= 1 && f.up[i] && f.black[i] && f.long[i] && f.white[i - 1] && f.small[i - 1] && b.close[i] <= b.open[i - 1] && b.open[i] >= b.close[i - 1] && (b.close[i] < b.open[i - 1] || b.open[i] > b.close[i - 1]));
pattern!(engulfing_bull, trend, |f, b, i| i >= 1 && f.dn[i] && f.white[i] && f.long[i] && f.black[i - 1] && f.small[i - 1] && b.close[i] >= b.open[i - 1] && b.open[i] <= b.close[i - 1] && (b.close[i] > b.open[i - 1] || b.open[i] < b.close[i - 1]));
pattern!(falling_window, trend, |f, b, i| i >= 1 && f.dn[i - 1] && f.rng[i] != 0. && f.rng[i - 1] != 0. && b.high[i] < b.low[i - 1]);
pattern!(gravestone_doji, |f, b, i| grav(&f, i));
pattern!(hammer, trend, |f, b, i| ham(&f, i) && f.dn[i]);
pattern!(hanging_man, trend, |f, b, i| ham(&f, i) && f.up[i]);
pattern!(harami_bear, trend, |f, b, i| i >= 1 && f.long[i - 1] && f.white[i - 1] && f.up[i - 1] && f.black[i] && f.small[i] && b.high[i] <= f.bh[i - 1] && b.low[i] >= f.bl[i - 1]);
pattern!(harami_bull, trend, |f, b, i| i >= 1 && f.long[i - 1] && f.black[i - 1] && f.dn[i - 1] && f.white[i] && f.small[i] && b.high[i] <= f.bh[i - 1] && b.low[i] >= f.bl[i - 1]);
pattern!(harami_cross_bear, trend, |f, b, i| i >= 1 && f.long[i - 1] && f.white[i - 1] && f.up[i - 1] && f.dojib[i] && b.high[i] <= f.bh[i - 1] && b.low[i] >= f.bl[i - 1]);
pattern!(harami_cross_bull, trend, |f, b, i| i >= 1 && f.long[i - 1] && f.black[i - 1] && f.dn[i - 1] && f.dojib[i] && b.high[i] <= f.bh[i - 1] && b.low[i] >= f.bl[i - 1]);
pattern!(inverted_hammer, trend, |f, b, i| inv(&f, i) && f.dn[i]);
pattern!(kicking_bear, |f, b, i| i >= 1 && maru(&f, i - 1) && f.white[i - 1] && maru(&f, i) && f.black[i] && b.low[i - 1] > b.high[i]);
pattern!(kicking_bull, |f, b, i| i >= 1 && maru(&f, i - 1) && f.black[i - 1] && maru(&f, i) && f.white[i] && b.high[i - 1] < b.low[i]);
pattern!(long_lower_shadow, |f, b, i| f.ds[i] > f.rng[i] * 0.75);
pattern!(long_upper_shadow, |f, b, i| f.us[i] > f.rng[i] * 0.75);
pattern!(marubozu_black, |f, b, i| maru(&f, i) && f.black[i]);
pattern!(marubozu_white, |f, b, i| maru(&f, i) && f.white[i]);
pattern!(on_neck, trend, |f, b, i| i >= 1 && f.dn[i] && f.black[i - 1] && f.long[i - 1] && f.white[i] && b.open[i] < b.close[i - 1] && f.small[i] && f.rng[i] != 0. && (b.close[i] - b.low[i - 1]).abs() <= f.avg[i] * 0.05);
pattern!(piercing, trend, |f, b, i| i >= 1 && f.dn[i - 1] && f.black[i - 1] && f.long[i - 1] && f.white[i] && b.open[i] <= b.low[i - 1] && b.close[i] > f.mid[i - 1] && b.close[i] < b.open[i - 1]);
pattern!(rising_window, trend, |f, b, i| i >= 1 && f.up[i - 1] && f.rng[i] != 0. && f.rng[i - 1] != 0. && b.low[i] > b.high[i - 1]);
pattern!(shooting_star, trend, |f, b, i| inv(&f, i) && f.up[i]);
pattern!(spinning_top_black, |f, b, i| spin(&f, i) && f.black[i]);
pattern!(spinning_top_white, |f, b, i| spin(&f, i) && f.white[i]);
// TradingView's source tests C_UpTrend[1] for Tweezer Bottom too; kept as published (Decision 20).
pattern!(tweezer_bottom, trend, |f, b, i| i >= 1 && f.up[i - 1] && (!f.dojib[i] || (f.hus[i] && f.hds[i])) && (b.low[i] - b.low[i - 1]).abs() <= f.avg[i] * 0.05 && f.black[i - 1] && f.white[i] && f.long[i - 1]);
pattern!(tweezer_top, trend, |f, b, i| i >= 1 && f.up[i - 1] && (!f.dojib[i] || (f.hus[i] && f.hds[i])) && (b.high[i] - b.high[i - 1]).abs() <= f.avg[i] * 0.05 && f.white[i - 1] && f.black[i] && f.long[i - 1]);

macro_rules! cdl {
    ($id:expr, $name:expr, $params:expr, $f:expr) => {
        IndicatorDef { id: $id, name: $name, category: CAT, params: $params, outputs: &["signal"], placement: Placement::Pane, needs: Needs::NONE,
            definition: concat!($name, ": TradingView All Candlestick Patterns rule (Pine v4 source, see candles.rs); 1 on the bar the pattern completes, else 0"),
            first: &[0], sparse: false, compute: Some($f), note: "" }
    };
}
pub const DEFS: &[IndicatorDef] = &[
    cdl!("cdl_dark_cloud_cover", "Dark Cloud Cover - Bearish", TREND, dark_cloud_cover),
    cdl!("cdl_doji", "Doji", &[], doji),
    cdl!("cdl_doji_star_bear", "Doji Star - Bearish", TREND, doji_star_bear),
    cdl!("cdl_doji_star_bull", "Doji Star - Bullish", TREND, doji_star_bull),
    cdl!("cdl_dragonfly_doji", "Dragonfly Doji - Bullish", &[], dragonfly_doji),
    cdl!("cdl_engulfing_bear", "Engulfing - Bearish", TREND, engulfing_bear),
    cdl!("cdl_engulfing_bull", "Engulfing - Bullish", TREND, engulfing_bull),
    cdl!("cdl_falling_window", "Falling Window - Bearish", TREND, falling_window),
    cdl!("cdl_gravestone_doji", "Gravestone Doji - Bearish", &[], gravestone_doji),
    cdl!("cdl_hammer", "Hammer - Bullish", TREND, hammer),
    cdl!("cdl_hanging_man", "Hanging Man - Bearish", TREND, hanging_man),
    cdl!("cdl_harami_bear", "Harami - Bearish", TREND, harami_bear),
    cdl!("cdl_harami_bull", "Harami - Bullish", TREND, harami_bull),
    cdl!("cdl_harami_cross_bear", "Harami Cross - Bearish", TREND, harami_cross_bear),
    cdl!("cdl_harami_cross_bull", "Harami Cross - Bullish", TREND, harami_cross_bull),
    cdl!("cdl_inverted_hammer", "Inverted Hammer - Bullish", TREND, inverted_hammer),
    cdl!("cdl_kicking_bear", "Kicking - Bearish", &[], kicking_bear),
    cdl!("cdl_kicking_bull", "Kicking - Bullish", &[], kicking_bull),
    cdl!("cdl_long_lower_shadow", "Long Lower Shadow - Bullish", &[], long_lower_shadow),
    cdl!("cdl_long_upper_shadow", "Long Upper Shadow - Bearish", &[], long_upper_shadow),
    cdl!("cdl_marubozu_black", "Marubozu Black - Bearish", &[], marubozu_black),
    cdl!("cdl_marubozu_white", "Marubozu White - Bullish", &[], marubozu_white),
    cdl!("cdl_on_neck", "On Neck - Bearish", TREND, on_neck),
    cdl!("cdl_piercing", "Piercing - Bullish", TREND, piercing),
    cdl!("cdl_rising_window", "Rising Window - Bullish", TREND, rising_window),
    cdl!("cdl_shooting_star", "Shooting Star - Bearish", TREND, shooting_star),
    cdl!("cdl_spinning_top_black", "Spinning Top Black", &[], spinning_top_black),
    cdl!("cdl_spinning_top_white", "Spinning Top White", &[], spinning_top_white),
    cdl!("cdl_tweezer_bottom", "Tweezer Bottom - Bullish", TREND, tweezer_bottom),
    cdl!("cdl_tweezer_top", "Tweezer Top - Bearish", TREND, tweezer_top),
];

#[cfg(test)]
mod tests {
    use super::DEFS;
    use crate::lab::catalogue::get;
    use crate::lab::testdata::{compiles, from_ohlc, go};
    const PRE: (f64, f64, f64, f64) = (10., 11.2, 9.8, 11.); // 14 identical bars: body 1, so the body EMA is 1
    pub(super) fn case(rows: &[(f64, f64, f64, f64)]) -> crate::lab::bars::Bars { let mut all = vec![PRE; 14]; all.extend_from_slice(rows); from_ohlc(&all) }
    /// The pattern's value on the last bar, with trend detection off (trend = 2) where the pattern has a trend rule.
    pub(super) fn last(id: &str, rows: &[(f64, f64, f64, f64)]) -> f64 {
        let kv: &[(&str, f64)] = if get(id).unwrap().params.is_empty() { &[] } else { &[("trend", 2.)] };
        *go(id, &case(rows), kv)[0].last().unwrap()
    }
    const CASES: &[(&str, &[(f64, f64, f64, f64)])] = &[
        ("cdl_dark_cloud_cover", &[(10.0, 13.1, 9.9, 13.0), (13.2, 13.3, 11.0, 11.2)]),
        ("cdl_doji", &[(11.0, 11.6, 10.4, 11.02)]),
        ("cdl_doji_star_bear", &[(10.0, 13.1, 9.9, 13.0), (13.5, 13.8, 13.2, 13.51)]),
        ("cdl_doji_star_bull", &[(13.0, 13.1, 9.9, 10.0), (9.5, 9.8, 9.2, 9.51)]),
        ("cdl_dragonfly_doji", &[(11.0, 11.01, 10.0, 11.01)]),
        ("cdl_engulfing_bear", &[(10.5, 11.1, 10.4, 11.0), (11.2, 11.3, 9.9, 10.0)]),
        ("cdl_engulfing_bull", &[(11.0, 11.1, 10.4, 10.5), (10.3, 12.1, 10.2, 12.0)]),
        ("cdl_falling_window", &[(11.0, 11.2, 10.8, 11.0), (10.4, 10.6, 10.2, 10.5)]),
        ("cdl_gravestone_doji", &[(11.0, 12.0, 11.0, 11.01)]),
        ("cdl_hammer", &[(11.0, 11.1, 9.6, 11.1)]),
        ("cdl_hanging_man", &[(11.0, 11.1, 9.6, 11.1)]),
        ("cdl_harami_bear", &[(10.0, 13.1, 9.9, 13.0), (12.5, 12.6, 11.6, 12.0)]),
        ("cdl_harami_bull", &[(13.0, 13.1, 9.9, 10.0), (10.5, 11.4, 10.4, 11.0)]),
        ("cdl_harami_cross_bear", &[(10.0, 13.1, 9.9, 13.0), (11.5, 12.0, 11.0, 11.51)]),
        ("cdl_harami_cross_bull", &[(13.0, 13.1, 9.9, 10.0), (11.5, 12.0, 11.0, 11.51)]),
        ("cdl_inverted_hammer", &[(11.0, 12.6, 11.0, 11.1)]),
        ("cdl_kicking_bear", &[(10.0, 13.0, 10.0, 13.0), (9.5, 9.5, 6.5, 6.5)]),
        ("cdl_kicking_bull", &[(13.0, 13.0, 10.0, 10.0), (13.5, 16.5, 13.5, 16.5)]),
        ("cdl_long_lower_shadow", &[(11.0, 11.1, 9.0, 11.05)]),
        ("cdl_long_upper_shadow", &[(11.0, 13.0, 10.95, 10.98)]),
        ("cdl_marubozu_black", &[(13.0, 13.0, 10.0, 10.0)]),
        ("cdl_marubozu_white", &[(10.0, 13.0, 10.0, 13.0)]),
        ("cdl_on_neck", &[(13.0, 13.1, 10.0, 10.1), (9.5, 10.2, 9.4, 10.02)]),
        ("cdl_piercing", &[(13.0, 13.1, 10.0, 10.1), (9.9, 12.0, 9.8, 11.9)]),
        ("cdl_rising_window", &[(11.0, 11.2, 10.8, 11.0), (11.4, 11.6, 11.3, 11.5)]),
        ("cdl_shooting_star", &[(11.0, 12.6, 11.0, 11.1)]),
        ("cdl_spinning_top_black", &[(11.1, 11.8, 10.2, 11.0)]),
        ("cdl_spinning_top_white", &[(11.0, 11.8, 10.2, 11.1)]),
        ("cdl_tweezer_bottom", &[(13.0, 13.1, 9.9, 10.0), (10.5, 12.0, 9.9, 11.8)]),
        ("cdl_tweezer_top", &[(10.0, 13.1, 9.9, 13.0), (12.5, 13.1, 11.0, 11.2)]),
    ];
    #[test] fn each_pattern_fires_on_its_case() { for (id, rows) in CASES { assert_eq!(last(id, rows), 1., "{id}"); } }
    #[test] fn plain_bars_fire_nothing() {
        for d in DEFS.iter().filter(|d| d.id.starts_with("cdl_") && d.id != "cdl_all") {
            let kv: &[(&str, f64)] = if d.params.is_empty() { &[] } else { &[("trend", 2.)] };
            assert!(go(d.id, &case(&[PRE]), kv)[0].iter().all(|v| *v == 0.), "{}", d.id);
        }
    }
    #[test] fn trend_rule_gates_reversal_patterns() {
        // with the default SMA50 trend rule, 15 bars have no trend yet, so a hammer is not reported
        assert_eq!(*go("cdl_hammer", &case(&[(11., 11.1, 9.6, 11.1)]), &[])[0].last().unwrap(), 0.);
    }
    #[test] fn rules_reach_candles() { compiles("cdl_engulfing_bull", "signal").unwrap(); compiles("cdl_doji", "signal").unwrap(); }
}
