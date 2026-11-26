//! TradingView Technical Ratings, as TradingView's TechnicalRating library v3 (2024-11-29) computes them: the ratings of the
//! "Technicals" gauge, the Screener's "Rating" and the built-in indicator. Rules ported from that library's published source
//! (Mozilla Public License 2.0, © TradingView, tradingview.com/script/jDWyb5PG-TechnicalRating); the older help-centre rules
//! (43000614331) differ in the ADX sell, the trend average, the Ichimoku offset and how missing values count.
use super::bars::Bars;
use super::catalogue::{get, run, IndicatorDef, Needs, Output, Params, Placement};
use super::ta::NA;

/// Pine `calcRating(src, bull, bear)`: NaN while `src` is NaN, else +1 / −1 / 0 (a comparison with a NaN is false).
fn vote(src: f64, buy: bool, sell: bool) -> f64 { if src.is_nan() { NA } else if buy { 1. } else if sell { -1. } else { 0. } }
/// VWMA(n) = sum(close·volume, n) / sum(volume, n) on bars whose volume starts at bar `k` (`v` holds the volume from `k`); NaN before.
fn late_vwma(c: &[f64], k: usize, v: &[f64], n: usize) -> Vec<f64> {
    let mut out = vec![NA; c.len()];
    for i in (k + n - 1)..c.len().min(k + v.len()) {
        let (mut pv, mut vol) = (0., 0.);
        for j in (i + 1 - n)..=i { pv += c[j] * v[j - k]; vol += v[j - k]; }
        if vol > 0. { out[i] = pv / vol; }
    }
    out
}
/// Pine `array.avg`: the mean of the values that exist; NaN when none does.
fn avg(v: &[f64]) -> f64 { let ok: Vec<f64> = v.iter().copied().filter(|x| !x.is_nan()).collect(); if ok.is_empty() { NA } else { ok.iter().sum::<f64>() / ok.len() as f64 } }
fn tech_ratings(b: &Bars, _: &Params) -> Output {
    let r = |id: &str, kv: &[(&str, f64)]| run(&get(id).expect("component in catalogue"), b, &kv.iter().map(|(k, v)| (k.to_string(), *v)).collect());
    let c = &b.close;
    let mut mas: Vec<Vec<f64>> = Vec::new();
    for k in [10., 20., 30., 50., 100., 200.] { mas.push(r("sma", &[("length", k)])?.remove(0)); }
    for k in [10., 20., 30., 50., 100., 200.] { mas.push(r("ema", &[("length", k)])?.remove(0)); }
    mas.push(r("hma", &[("length", 9.)])?.remove(0));
    if b.volume.is_some() { mas.push(r("vwma", &[("length", 20.)])?.remove(0)); } // no volume: Pine's vwma is NaN and drops out of the average
    else if let Some((k, v)) = &b.late_volume { mas.push(late_vwma(c, *k, v, 20)); } // volume from a later bar (NIFTY 50 from 2000): VWMA joins once it has 20 bars
    // Ichimoku 9/26/52: the library reads lead1[26] / lead2[26], the spans computed 26 bars earlier (the lab's displacement 26),
    // and checks the current, unshifted lead2 (displacement 0) for NaN
    let ich = r("ichimoku", &[("conversion", 9.), ("base", 26.), ("span_b", 52.), ("displacement", 26.)])?;
    let lead2_now = r("ichimoku", &[("conversion", 9.), ("base", 26.), ("span_b", 52.), ("displacement", 0.)])?.remove(3);
    let trend = r("ema", &[("length", 50.)])?.remove(0); // priceAvg = ema(close, 50)
    let rsi = r("rsi", &[("length", 14.)])?.remove(0);
    let st = r("stoch", &[("k", 14.), ("smooth_k", 3.), ("d", 3.)])?;
    let cci = r("cci", &[("length", 20.)])?.remove(0);
    let dmi = r("dmi", &[])?;
    let ao = r("ao", &[])?.remove(0);
    let mom = r("mom", &[("length", 10.)])?.remove(0);
    let macd = r("macd", &[])?;
    let srsi = r("stochrsi", &[])?;
    let wr = r("willr", &[("length", 14.)])?.remove(0);
    let bbp = r("bbp", &[("length", 13.)])?;
    let uo = r("uo", &[])?.remove(0);
    let back = |v: &[f64], i: usize, k: usize| if i >= k { v[i - k] } else { NA };
    let len = b.len();
    let (mut ma, mut osc, mut all) = (vec![NA; len], vec![NA; len], vec![NA; len]);
    for i in 0..len {
        // calcRatingMA: sign(close − ma)
        let mut m: Vec<f64> = mas.iter().map(|s| if s[i].is_nan() { NA } else { (c[i] - s[i]).signum() * f64::from(c[i] != s[i]) }).collect();
        let (con, base, l1, l2) = (ich[0][i], ich[1][i], ich[2][i], ich[3][i]);
        m.push(vote(lead2_now[i], l1 > l2 && base > l1 && con > base && c[i] > con, l1 < l2 && base < l1 && con < base && c[i] < con));
        let (up, dn) = (c[i] > trend[i], c[i] < trend[i]);
        let (rsi1, d1, cci1, adx1, ao1, ao2, mom1, wr1) = (back(&rsi, i, 1), back(&st[1], i, 1), back(&cci, i, 1), back(&dmi[2], i, 1), back(&ao, i, 1), back(&ao, i, 2), back(&mom, i, 1), back(&wr, i, 1));
        let (bull1, bear1) = (back(&bbp[1], i, 1), back(&bbp[2], i, 1));
        let (adx, dip, din) = (dmi[2][i], dmi[0][i], dmi[1][i]);
        let o = [
            vote(rsi1, rsi[i] < 30. && rsi1 < rsi[i], rsi[i] > 70. && rsi1 > rsi[i]),
            vote(d1, st[0][i] < 20. && st[1][i] < 20. && st[0][i] > st[1][i], st[0][i] > 80. && st[1][i] > 80. && st[0][i] < st[1][i]),
            vote(cci1, cci[i] < -100. && cci[i] > cci1, cci[i] > 100. && cci[i] < cci1),
            vote(adx, adx > 20. && adx > adx1 && dip > din, adx > 20. && adx > adx1 && dip < din), // both sides need a rising ADX
            vote(ao1, (ao[i] > 0. && ao1 <= 0.) || (ao[i] > 0. && ao1 > 0. && ao[i] > ao1 && ao2 > ao1),
                 (ao[i] < 0. && ao1 >= 0.) || (ao[i] < 0. && ao1 < 0. && ao[i] < ao1 && ao2 < ao1)),
            vote(mom1, mom[i] > mom1, mom[i] < mom1),
            vote(macd[1][i], macd[0][i] > macd[1][i], macd[0][i] < macd[1][i]),
            vote(trend[i], dn && srsi[0][i] < 20. && srsi[1][i] < 20. && srsi[0][i] > srsi[1][i], up && srsi[0][i] > 80. && srsi[1][i] > 80. && srsi[0][i] < srsi[1][i]),
            vote(wr1, wr[i] < -80. && wr[i] > wr1, wr[i] > -20. && wr[i] < wr1),
            vote(trend[i], up && bbp[2][i] < 0. && bbp[2][i] > bear1, dn && bbp[1][i] > 0. && bbp[1][i] < bull1),
            vote(uo[i], uo[i] > 70., uo[i] < 30.),
        ];
        ma[i] = avg(&m); osc[i] = avg(&o);
        // the total is the mean of the two groups, or the one group that exists
        all[i] = match (ma[i].is_nan(), osc[i].is_nan()) { (false, false) => (ma[i] + osc[i]) / 2., (false, true) => ma[i], (true, false) => osc[i], _ => NA };
    }
    Ok(vec![ma, osc, all])
}
pub const DEFS: &[IndicatorDef] = &[IndicatorDef { id: "tech_ratings", name: "Technical Ratings", category: "Ratings", params: &[], outputs: &["ma", "osc", "all"],
    placement: Placement::Pane, needs: Needs::NONE, definition: "TradingView Technical Ratings (TechnicalRating library v3): each component votes +1/−1/0 and a group is the mean of the components that already have a value; ma = SMA & EMA 10/20/30/50/100/200, HMA 9, VWMA 20 (dropped without volume) and Ichimoku 9/26/52 (cloud 26 bars back); osc = RSI 14, Stoch 14/3/3, CCI 20, ADX 14/14 (rising ADX for both sides), AO, Momentum 10, MACD 12/26/9, Stoch RSI 3/3/14/14, Williams %R 14, Bull Bear Power 13, UO 7/14/28; trend = close vs ema(close, 50); all = (ma + osc)/2, or the one group that exists", first: &[9, 11, 9], sparse: false, compute: Some(tech_ratings), note: "" }];
#[cfg(test)]
mod tests {
    use crate::lab::testdata::{close_eq, compiles, from_ohlc, go};
    fn series(growth: f64, volume: bool) -> crate::lab::bars::Bars {
        let mut b = from_ohlc(&(0..300).map(|i| { let c = 100. * growth.powi(i); (c, c * 1.005, c * 0.995, c) }).collect::<Vec<_>>());
        if volume { b.volume = Some(vec![1000.; 300]); }
        b
    }
    #[test] fn moving_average_rating_on_steady_trends() {
        for volume in [true, false] {
            let up = go("tech_ratings", &series(1.01, volume), &[]);
            assert_eq!(up[0][299], 1.0, "every MA below price in a steady rise (volume {volume})");
        }
        let down = go("tech_ratings", &series(0.99, true), &[]);
        assert!((down[0][299] + 13. / 15.).abs() < 1e-12, "HMA overshoots in a steady fall: 14 sells, 1 buy (HMA) of 15 votes → {}", down[0][299]);
    }
    /// Pine's array.avg: a group counts the components that already exist, so ratings start long before the 200-bar averages
    /// (TradingView's ratings exist on a young listing); the total is the one group that exists until both do.
    #[test] fn components_join_as_they_become_available() {
        let b = crate::lab::testdata::walk(400, 7, true);
        let o = go("tech_ratings", &b, &[]);
        let sma10 = go("sma", &b, &[("length", 10.)]).remove(0);
        let first = sma10.iter().position(|v| !v.is_nan()).unwrap();
        assert!(o[0][first - 1].is_nan() && !o[0][first].is_nan(), "the MA group starts with its first moving average (bar {first})");
        assert!(o[0][first].abs() <= 1. && o[0][150].abs() <= 1.);
        for i in 0..400 { if o[1][i].is_nan() && !o[0][i].is_nan() { assert!(close_eq(o[2][i], o[0][i]), "bar {i}: total = the MA group alone"); } }
    }
    /// Volume that starts late (NIFTY 50 before 2000 has none) still brings VWMA 20 into the MA group from its 20th volume bar.
    #[test] fn late_volume_adds_the_vwma_vote() {
        let mut b = series(1.01, true);
        let full = go("tech_ratings", &b, &[]);
        let v = b.volume.take().unwrap();
        b.late_volume = Some((250, v[250..].to_vec()));
        let late = go("tech_ratings", &b, &[]);
        assert_eq!(late[0][299], full[0][299], "with 50 volume bars the VWMA vote is back (all 15 MAs below price)");
        let none = { let mut c = b.clone(); c.late_volume = None; go("tech_ratings", &c, &[]) };
        assert_eq!(none[0][260], late[0][260], "before its 20th volume bar the VWMA is not counted");
        let vw = super::late_vwma(&b.close, 250, &v[250..], 20);
        assert!(vw[268].is_nan() && !vw[269].is_nan());
    }
    #[test] fn overall_is_the_mean_of_the_groups() {
        let o = go("tech_ratings", &crate::lab::testdata::walk(400, 3, true), &[]);
        for i in 199..400 { assert!(close_eq(o[2][i], (o[0][i] + o[1][i]) / 2.) && !(o[1][i].abs() > 1.), "bar {i}"); }
    }
    #[test] fn rules_reach_ratings() { compiles("tech_ratings", "all").unwrap(); }
    /// Recomputes the 11 oscillator votes from the component series with the TechnicalRating library v3 rules and compares the group mean.
    #[test] fn oscillator_votes_match_an_independent_recomputation() {
        use crate::lab::testdata::walk;
        let b = walk(1000, 5, true);
        let g = |id: &str, kv: &[(&str, f64)]| go(id, &b, kv);
        let (rsi, st, cci, dmi, ao) = (g("rsi", &[]).remove(0), g("stoch", &[("k", 14.), ("smooth_k", 3.), ("d", 3.)]), g("cci", &[]).remove(0), g("dmi", &[]), g("ao", &[]).remove(0));
        let (mom, macd, sr, wr, bbp, uo) = (g("mom", &[]).remove(0), g("macd", &[]), g("stochrsi", &[]), g("willr", &[]).remove(0), g("bbp", &[]), g("uo", &[]).remove(0));
        let trend = g("ema", &[("length", 50.)]).remove(0);
        let o = go("tech_ratings", &b, &[]);
        let v = |buy: bool, sell: bool| if buy { 1. } else if sell { -1. } else { 0. };
        let (mut mixed, mut nonzero) = (0, 0);
        for i in 200..1000 {
            let c = b.close[i];
            let votes = [
                v(rsi[i] < 30. && rsi[i] > rsi[i - 1], rsi[i] > 70. && rsi[i] < rsi[i - 1]),
                v(st[0][i] < 20. && st[1][i] < 20. && st[0][i] > st[1][i], st[0][i] > 80. && st[1][i] > 80. && st[0][i] < st[1][i]),
                v(cci[i] < -100. && cci[i] > cci[i - 1], cci[i] > 100. && cci[i] < cci[i - 1]),
                v(dmi[0][i] > dmi[1][i] && dmi[2][i] > 20. && dmi[2][i] > dmi[2][i - 1], dmi[0][i] < dmi[1][i] && dmi[2][i] > 20. && dmi[2][i] > dmi[2][i - 1]),
                v((ao[i] > 0. && ao[i - 1] <= 0.) || (ao[i] > 0. && ao[i - 1] > 0. && ao[i] > ao[i - 1] && ao[i - 1] < ao[i - 2]),
                  (ao[i] < 0. && ao[i - 1] >= 0.) || (ao[i] < 0. && ao[i - 1] < 0. && ao[i] < ao[i - 1] && ao[i - 1] > ao[i - 2])),
                v(mom[i] > mom[i - 1], mom[i] < mom[i - 1]),
                v(macd[0][i] > macd[1][i], macd[0][i] < macd[1][i]),
                v(c < trend[i] && sr[0][i] < 20. && sr[1][i] < 20. && sr[0][i] > sr[1][i], c > trend[i] && sr[0][i] > 80. && sr[1][i] > 80. && sr[0][i] < sr[1][i]),
                v(wr[i] < -80. && wr[i] > wr[i - 1], wr[i] > -20. && wr[i] < wr[i - 1]),
                v(c > trend[i] && bbp[2][i] < 0. && bbp[2][i] > bbp[2][i - 1], c < trend[i] && bbp[1][i] > 0. && bbp[1][i] < bbp[1][i - 1]),
                v(uo[i] > 70., uo[i] < 30.),
            ];
            let want = votes.iter().sum::<f64>() / 11.;
            assert!(close_eq(o[1][i], want), "bar {i}: osc {} vs {want}, votes {votes:?}", o[1][i]);
            if votes.contains(&1.) && votes.contains(&-1.) { mixed += 1; }
            if want.abs() > 0.2 { nonzero += 1; }
        }
        assert!(mixed > 5 && nonzero > 5, "series must exercise buys and sells: mixed {mixed}, nonzero {nonzero}");
    }
}
