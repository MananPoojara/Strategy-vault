//! Census group "candles" (2026-10-26, research: sv-census-work/research_candles.md). Formulas: see each definition string.
//! Reuses the TA-Lib candle helpers of x_candles.rs (K, TA_CANDLEAVERAGE, CandleSettings). Skipped as UNCONFIRMED (nothing invented):
//! Judas Candle (one uncited Wikipedia line; "smaller" and "lower tail equal to the black candle in length" have no source threshold,
//! and no platform, book or code defines it), Hogscraper (a c. 1780–1860 candle holder, not a chart pattern: census false positive).
use super::bars::Bars;
use super::catalogue::{int, n, p, real, IndicatorDef, Needs, Output, ParamSpec, Params, Placement};
use super::ta::NA;
use super::x_candles::{K, BODY_LONG, BODY_SHORT, SHADOW_VERY_SHORT};

const fn def(id: &'static str, name: &'static str, params: &'static [ParamSpec], outputs: &'static [&'static str], definition: &'static str,
             first: &'static [usize], f: fn(&Bars, &Params) -> Output) -> IndicatorDef {
    IndicatorDef { id, name, category: CDL, params, outputs, placement: Placement::Pane, needs: Needs::NONE, definition, first, sparse: false, compute: Some(f), note: "" }
}
const CDL: &str = "Candlestick patterns";
fn ev(hit: bool) -> f64 { if hit { 1. } else { 0. } }
/// 1/0 per bar from `from` (the first bar the rule is defined), NaN before it.
fn events(len: usize, from: usize, f: impl Fn(usize) -> bool) -> Vec<f64> { (0..len).map(|i| if i < from { NA } else { ev(f(i)) }).collect() }

const STAR: &[ParamSpec] = &[int("variant", "Colour rule (0 StockCharts any colour, 1 TA-Lib doji-star colour)", 1., 0., 1.)];
fn star(b: &Bars, prm: &Params) -> Output {
    let (k, any) = (K::new(b), n(prm, "variant") == 0);
    let base = |i: usize| k.rb(i - 1) > k.avg(BODY_LONG, i - 1) && k.rb(i) <= k.avg(BODY_SHORT, i);
    Ok(vec![events(b.len(), 11, |i| base(i) && k.gap_up(i, i - 1) && (any || k.col(i - 1) == 1)),
            events(b.len(), 11, |i| base(i) && k.gap_dn(i, i - 1) && (any || k.col(i - 1) == -1))])
}

const BODY: &[ParamSpec] = &[int("variant", "Doji close = open (0 Nison / TradingView neither colour, 1 TA-Lib white)", 0., 0., 1.)];
fn body_color(b: &Bars, prm: &Params) -> Output {
    let (o, c, ta) = (&b.open, &b.close, n(prm, "variant") == 1);
    Ok(vec![events(b.len(), 0, |i| c[i] > o[i] || (ta && c[i] == o[i])), events(b.len(), 0, |i| c[i] < o[i])])
}

const SHAVEN: &[ParamSpec] = &[int("variant", "Tolerance (0 Nison exact, 1 TA-Lib ShadowVeryShort)", 0., 0., 1.)];
fn shaven(b: &Bars, prm: &Params, head: bool) -> Output {
    let (k, ta) = (K::new(b), n(prm, "variant") == 1);
    let sh = |i: usize| if head { k.us(i) } else { k.ls(i) };
    Ok(vec![events(b.len(), if ta { 10 } else { 0 }, |i| if ta { sh(i) < k.avg(SHADOW_VERY_SHORT, i) } else { sh(i) <= 0. })])
}
fn shaven_head(b: &Bars, prm: &Params) -> Output { shaven(b, prm, true) }
fn shaven_bottom(b: &Bars, prm: &Params) -> Output { shaven(b, prm, false) }

const PIN: &[ParamSpec] = &[
    real("max_nose_body", "MaxNoseBodySize: max nose body / nose length", 0.33, 0., 1., 0.01),
    real("nose_body_position", "NoseBodyPosition: body within this fraction of the bar from its tail end", 0.4, 0., 1., 0.01),
    int("left_eye_opposite", "LeftEyeOppositeDirection (0/1)", 1., 0., 1.),
    int("nose_same", "NoseSameDirection (0/1)", 0., 0., 1.),
    int("nose_in_eye_body", "NoseBodyInsideLeftEyeBody (0/1)", 0., 0., 1.),
    real("left_eye_min_body", "LeftEyeMinBodySize: min left eye body / left eye length", 0.1, 0., 1., 0.01),
    real("nose_protruding", "NoseProtruding: nose beyond the left eye / nose length", 0.5, 0., 10., 0.01),
    real("nose_body_to_eye_body", "NoseBodyToLeftEyeBody: max nose body / left eye body", 1., 0., 100., 0.01),
    real("nose_to_eye_length", "NoseLengthToLeftEyeLength: min nose length / left eye length", 0., 0., 100., 0.01),
    real("left_eye_depth", "LeftEyeDepth: left eye beyond the nose's other end / left eye length", 0.1, 0., 10., 0.01),
    int("min_nose", "MinimumNoseLength (points)", 1., 0., 1e6),
    real("point", "Point size (MetaTrader _Point)", 0.05, 0.0001, 1000., 0.0001)];
fn pin_bar(b: &Bars, prm: &Params) -> Output {
    let (k, (o, h, l, c), pt) = (K::new(b), (&b.open, &b.high, &b.low, &b.close), p(prm, "point"));
    let (mb, pos, emb, pro, nbe, nle, dep) = (p(prm, "max_nose_body"), p(prm, "nose_body_position"), p(prm, "left_eye_min_body"), p(prm, "nose_protruding"),
        p(prm, "nose_body_to_eye_body"), p(prm, "nose_to_eye_length"), p(prm, "left_eye_depth"));
    let (opp, same, inb, min) = (n(prm, "left_eye_opposite") == 1, n(prm, "nose_same") == 1, n(prm, "nose_in_eye_body") == 1, p(prm, "min_nose") * pt);
    let q = |x: f64| if x == 0. { pt } else { x }; // EarnForex: a zero length or body counts as one point
    let mut out = vec![vec![NA; b.len()]; 2];
    for i in 1..b.len() {
        let raw = h[i] - l[i];
        let (nl, el, nb, eb) = (q(raw), q(h[i - 1] - l[i - 1]), q(k.rb(i)), q(k.rb(i - 1)));
        let both = !(raw < min) && nb / nl <= mb && eb / el >= emb && k.top(i) <= h[i - 1] && k.bot(i) >= l[i - 1] && nb / eb <= nbe && nl / el >= nle
            && (!inb || (k.top(i) <= k.top(i - 1) && k.bot(i) >= k.bot(i - 1)));
        out[0][i] = ev(both && l[i - 1] - l[i] >= nl * pro && 1. - (k.bot(i) - l[i]) / nl < pos && (!opp || c[i - 1] < o[i - 1]) && (!same || c[i] > o[i])
            && h[i - 1] - h[i] >= el * dep);
        out[1][i] = ev(both && h[i] - h[i - 1] >= nl * pro && 1. - (h[i] - k.top(i)) / nl < pos && (!opp || c[i - 1] > o[i - 1]) && (!same || c[i] < o[i])
            && l[i] - l[i - 1] >= el * dep);
    }
    Ok(out)
}

pub const DEFS: &[IndicatorDef] = &[
    def("cdl_star", "Star", STAR, &["up", "down"],
        "Star (Nison; StockCharts ChartSchool \"Star Position\": a small real body that gaps away from the large real body before it; shadows may overlap), coded with the TA-Lib star sub-rule of CDLDOJISTAR / CDLMORNINGSTAR (TA-Lib has no standalone star): bar t−1 long, |C−O|[t−1] > BodyLong = mean |C−O| of bars t−11..t−2; bar t short, |C−O|[t] ≤ BodyShort = mean |C−O| of bars t−10..t−1 (TA-Lib default CandleSettings, period 10, factor 1, averaged over the bars before the tested candle); real-body gap (TA_REALBODYGAPUP/DOWN, strict, shadows ignored): up = min(O,C)[t] > max(O,C)[t−1], down = max(O,C)[t] < min(O,C)[t−1]. Variant 1 (default, TA-Lib doji-star colour; white = C ≥ O): up needs a white t−1, down a black t−1; variant 0 (StockCharts \"any combination of white and black\"): either gap, any colour. A doji counts as a star. A star is a position, not a direction call: up = gapped up. Outputs up and down 1 on the star bar, else 0; NaN before bar 11 (TA-Lib CDLDOJISTAR lookback)",
        &[11, 11], star),
    def("cdl_body_color", "Black Body / White Body", BODY, &["white", "black"],
        "Candle body colour (Nison, The Candlestick Course ch. 1; StockCharts hollow/filled; TradingView All Candlestick Patterns C_WhiteBody = open < close, C_BlackBody = open > close): white 1 when close > open, black 1 when close < open, a doji (close = open) neither (variant 0, default). Variant 1 (TA-Lib TA_CANDLECOLOR): white = close ≥ open, so a doji is white; black = close < open. No size condition (a Big White/Black Candle is a separate pattern). Defined from bar 0",
        &[0, 0], body_color),
    def("cdl_shaven_head", "Shaven Head", SHAVEN, &["signal"],
        "Shaven Head (Nison: a candle with no upper shadow, either colour, any body; e.g. the bearish belt hold is a \"black opening shaven head\"; Wikipedia Candlestick pattern): upper shadow H − max(O,C). Variant 0 (default, exact): 1 when the upper shadow ≤ 0 (a flat bar O = H = L = C counts), defined from bar 0. Variant 1 (TA-Lib ShadowVeryShort, as in CDLCLOSINGMARUBOZU / CDLBELTHOLD): 1 when the upper shadow < 0.1 × mean (H − L) of the 10 bars before, strict; NaN before bar 10. Else 0. No body-length rule (unlike the marubozu, closing marubozu and belt hold)",
        &[0], shaven_head),
    def("cdl_shaven_bottom", "Shaven Bottom", SHAVEN, &["signal"],
        "Shaven Bottom (Nison: a candle with no lower shadow, either colour, any body; e.g. the bullish belt hold is a \"white opening shaven bottom\"; Wikipedia Candlestick pattern): lower shadow min(O,C) − L. Variant 0 (default, exact): 1 when the lower shadow ≤ 0 (a flat bar O = H = L = C counts), defined from bar 0. Variant 1 (TA-Lib ShadowVeryShort, as in CDLCLOSINGMARUBOZU / CDLBELTHOLD): 1 when the lower shadow < 0.1 × mean (H − L) of the 10 bars before, strict; NaN before bar 10. Else 0. No body-length rule",
        &[0], shaven_bottom),
    def("cdl_pin_bar", "Pin Bar", PIN, &["bull", "bear"],
        "Pin Bar (Martin Pring's Pinocchio bar; rules: EarnForex Pinbar Detector v1.03 for MetaTrader 4/5, all defaults from its source): nose = bar t, left eye = bar t−1; NL = H−L, EL = H−L of t−1, NB = |C−O|, EB = |C−O| of t−1, each 0 replaced by one point (default 0.05); no signal when H−L of t < MinimumNoseLength (1) × point. Both directions: NB/NL ≤ 0.33; EB/EL ≥ 0.1; nose body inside the left eye's range (max(O,C) ≤ H[t−1], min(O,C) ≥ L[t−1]); NB/EB ≤ 1; NL/EL ≥ 0; NoseBodyInsideLeftEyeBody (off): max(O,C) ≤ max(O,C)[t−1] and min(O,C) ≥ min(O,C)[t−1]. Bull: L[t−1] − L ≥ 0.5 × NL; 1 − (min(O,C) − L)/NL < 0.4; left eye black (C < O)[t−1] (LeftEyeOppositeDirection, on); nose white if NoseSameDirection (off); H[t−1] − H ≥ 0.1 × EL. Bear mirrors: H − H[t−1] ≥ 0.5 × NL; 1 − (H − max(O,C))/NL < 0.4; left eye white (C > O)[t−1]; nose black if NoseSameDirection; L − L[t−1] ≥ 0.1 × EL. 1 on the closed nose bar, else 0; NaN on bar 0 (no left eye). Common 2/3-wick teaching rules are not this indicator",
        &[1, 1], pin_bar),
];

pub const SOURCES: &[(&str, &str, &str)] = &[
    ("cdl_star", "StockCharts ChartSchool: Star Position; TA-Lib: star rule inside CDLDOJISTAR / CDLMORNINGSTAR (no standalone function); Nison: Star", ""),
    ("cdl_body_color", "TradingView All Candlestick Patterns: C_WhiteBody / C_BlackBody; TA-Lib: TA_CANDLECOLOR; StockCharts: hollow / filled candlestick; Nison: White Body / Black Body", ""),
    ("cdl_shaven_head", "Nison / Wikipedia: Shaven Head; TA-Lib: ShadowVeryShort setting (CDLCLOSINGMARUBOZU, CDLBELTHOLD)", ""),
    ("cdl_shaven_bottom", "Nison / Wikipedia: Shaven Bottom; TA-Lib: ShadowVeryShort setting (CDLCLOSINGMARUBOZU, CDLBELTHOLD)", ""),
    ("cdl_pin_bar", "EarnForex (MetaTrader 4/5): Pinbar Detector; Martin Pring: Pinocchio bar", ""),
];

#[cfg(test)]
mod tests {
    use crate::lab::testdata::{assert_series, compiles, from_ohlc, go};
    const N: f64 = f64::NAN;

    /// 10 bars with body 1 (O 100, C 101, range 100–101.5), then `tail`.
    fn hist(tail: &[(f64, f64, f64, f64)]) -> crate::lab::bars::Bars {
        let mut rows = vec![(100., 101.5, 100., 101.); 10];
        rows.extend_from_slice(tail);
        from_ohlc(&rows)
    }
    #[test] fn star_needs_long_then_small_gapped_body() {
        // bar 10: long white body 3 > mean 1; bar 11: body 0.5 ≤ mean (9·1 + 3)/10 = 1.2, min(O,C) 104.5 > 104: up star
        let b = hist(&[(101., 104.2, 100.9, 104.), (104.5, 105.5, 103.8, 105.)]);
        let o = go("cdl_star", &b, &[]);
        assert_series(&o[0], &[N, N, N, N, N, N, N, N, N, N, N, 1.]);
        assert_series(&o[1][11..], &[0.]);
        // touching bodies (104 = 104) are not a gap
        assert_series(&go("cdl_star", &hist(&[(101., 104.2, 100.9, 104.), (104., 105., 103.8, 104.5)]), &[])[0][11..], &[0.]);
        // a black long bar with a star above: TA-Lib colour rule says no, StockCharts any colour says up
        let b = hist(&[(104., 104.2, 100.9, 101.), (104.5, 105.5, 103.8, 105.)]);
        assert_series(&go("cdl_star", &b, &[])[0][11..], &[0.]);
        assert_series(&go("cdl_star", &b, &[("variant", 0.)])[0][11..], &[1.]);
        // down star after the black long bar: max(O,C) 100.5 < 101
        let b = hist(&[(104., 104.2, 100.9, 101.), (100.5, 101., 99., 100.)]);
        assert_series(&go("cdl_star", &b, &[])[1][11..], &[1.]);
    }
    #[test] fn body_color_doji_variants() {
        let b = from_ohlc(&[(1., 2., 0., 1.5), (1., 2., 0., 0.5), (1., 2., 0., 1.)]);
        let o = go("cdl_body_color", &b, &[]);
        assert_series(&o[0], &[1., 0., 0.]);
        assert_series(&o[1], &[0., 1., 0.]);
        assert_series(&go("cdl_body_color", &b, &[("variant", 1.)])[0], &[1., 0., 1.]);
    }
    #[test] fn shaven_exact_and_talib() {
        // head: high = max(O,C) on bars 0 (white) and 1 (black); bottom: low = min(O,C) on bars 1 and 2
        let b = from_ohlc(&[(1., 2., 0.5, 2.), (2., 2., 1., 1.), (1., 1.5, 1., 1.2)]);
        assert_series(&go("cdl_shaven_head", &b, &[])[0], &[1., 1., 0.]);
        assert_series(&go("cdl_shaven_bottom", &b, &[])[0], &[0., 1., 1.]);
        // TA-Lib: bar 10 threshold 0.1 × mean range 1.5 = 0.15, upper shadow 0.1 passes; bar 11 threshold 0.1 × 15.6/10 = 0.156, 0.2 fails
        let b = hist(&[(101., 102.1, 100., 102.), (101., 102.2, 100., 102.)]);
        let o = go("cdl_shaven_head", &b, &[("variant", 1.)]);
        assert!(o[0][..10].iter().all(|v| v.is_nan()));
        assert_series(&o[0][10..], &[1., 0.]);
        assert_series(&go("cdl_shaven_bottom", &hist(&[(101.1, 102., 101., 102.)]), &[("variant", 1.)])[0][10..], &[1.]);
    }
    #[test] fn pin_bar_earnforex_rules() {
        // eye: black 10→9 in 8.5–10.5 (EL 2, EB 1); nose 9.6→9.8 in 7–10 (NL 3, NB 0.2):
        // protrusion 8.5−7 = 1.5 ≥ 1.5; 0.2/3 ≤ 0.33; 1 − (9.6−7)/3 = 0.133 < 0.4; body inside eye range; 0.2/1 ≤ 1; eye depth 10.5−10 = 0.5 ≥ 0.2
        let b = from_ohlc(&[(10., 10.5, 8.5, 9.), (9.6, 10., 7., 9.8)]);
        let o = go("cdl_pin_bar", &b, &[]);
        assert_series(&o[0], &[N, 1.]);
        assert_series(&o[1], &[N, 0.]);
        // a white eye fails LeftEyeOppositeDirection unless it is switched off
        let b = from_ohlc(&[(9., 10.5, 8.5, 10.), (9.6, 10., 7., 9.8)]);
        assert_series(&go("cdl_pin_bar", &b, &[])[0], &[N, 0.]);
        assert_series(&go("cdl_pin_bar", &b, &[("left_eye_opposite", 0.)])[0], &[N, 1.]);
        // bear mirror: eye white 9→10 in 8.5–10.5; nose 9.4→9.2 in 9–12
        let b = from_ohlc(&[(9., 10.5, 8.5, 10.), (9.4, 12., 9., 9.2)]);
        assert_series(&go("cdl_pin_bar", &b, &[])[1], &[N, 1.]);
        // a nose shorter than MinimumNoseLength points never fires
        assert_series(&go("cdl_pin_bar", &from_ohlc(&[(10., 10.5, 8.5, 9.), (9.6, 10., 7., 9.8)]), &[("point", 4.)])[0], &[N, 0.]);
    }
    #[test] fn every_output_compiles_in_rules() {
        for d in super::DEFS { for o in d.outputs { compiles(d.id, o).unwrap_or_else(|e| panic!("{}.{o}: {e:?}", d.id)); } }
    }
}
