//! TradingView Technical Ratings from lab catalogue components (rules: TradingView help 43000614331).
use super::bars::Bars;
use super::catalogue::{get, run, IndicatorDef, Needs, Output, Params, Placement};
use super::ta::NA;

fn vote(buy: bool, sell: bool, inputs: &[f64]) -> f64 { if inputs.iter().any(|v| v.is_nan()) { NA } else if buy { 1. } else if sell { -1. } else { 0. } }
fn mean(v: &[f64]) -> f64 { if v.iter().any(|x| x.is_nan()) { NA } else { v.iter().sum::<f64>() / v.len() as f64 } }
fn tech_ratings(b: &Bars, _: &Params) -> Output {
    let r = |id: &str, kv: &[(&str, f64)]| run(&get(id).expect("component in catalogue"), b, &kv.iter().map(|(k, v)| (k.to_string(), *v)).collect());
    let c = &b.close;
    let mut mas: Vec<Vec<f64>> = Vec::new();
    for k in [10., 20., 30., 50., 100., 200.] { mas.push(r("sma", &[("length", k)])?.remove(0)); mas.push(r("ema", &[("length", k)])?.remove(0)); }
    mas.push(r("hma", &[("length", 9.)])?.remove(0));
    if b.volume.is_some() { mas.push(r("vwma", &[("length", 20.)])?.remove(0)); }
    let ich = r("ichimoku", &[])?;
    let trend = r("sma", &[("length", 50.)])?.remove(0);
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
        let mut m: Vec<f64> = mas.iter().map(|s| vote(s[i] < c[i], s[i] > c[i], &[s[i]])).collect();
        let (t, k, sa, sb) = (ich[0][i], ich[1][i], ich[2][i], ich[3][i]);
        m.push(vote(sa > sb && k > sa && t > k && c[i] > t, sa < sb && k < sa && t < k && c[i] < t, &[t, k, sa, sb]));
        let (up, dn) = (c[i] > trend[i], c[i] < trend[i]);
        let (r1, c1, a1, m1, w1) = (back(&rsi, i, 1), back(&cci, i, 1), back(&dmi[2], i, 1), back(&mom, i, 1), back(&wr, i, 1));
        let (ao1, ao2, bull1, bear1) = (back(&ao, i, 1), back(&ao, i, 2), back(&bbp[1], i, 1), back(&bbp[2], i, 1));
        let o = [
            vote(rsi[i] < 30. && rsi[i] > r1, rsi[i] > 70. && rsi[i] < r1, &[rsi[i], r1]),
            vote(st[0][i] < 20. && st[1][i] < 20. && st[0][i] > st[1][i], st[0][i] > 80. && st[1][i] > 80. && st[0][i] < st[1][i], &[st[0][i], st[1][i]]),
            vote(cci[i] < -100. && cci[i] > c1, cci[i] > 100. && cci[i] < c1, &[cci[i], c1]),
            vote(dmi[0][i] > dmi[1][i] && dmi[2][i] > 20. && dmi[2][i] > a1, dmi[0][i] < dmi[1][i] && dmi[2][i] > 20. && dmi[2][i] < a1, &[dmi[0][i], dmi[1][i], dmi[2][i], a1]),
            vote((ao[i] > 0. && ao1 <= 0.) || (ao[i] > 0. && ao1 > 0. && ao[i] > ao1 && ao1 < ao2),
                 (ao[i] < 0. && ao1 >= 0.) || (ao[i] < 0. && ao1 < 0. && ao[i] < ao1 && ao1 > ao2), &[ao[i], ao1, ao2]),
            vote(mom[i] > m1, mom[i] < m1, &[mom[i], m1]),
            vote(macd[0][i] > macd[1][i], macd[0][i] < macd[1][i], &[macd[0][i], macd[1][i]]),
            vote(dn && srsi[0][i] < 20. && srsi[1][i] < 20. && srsi[0][i] > srsi[1][i], up && srsi[0][i] > 80. && srsi[1][i] > 80. && srsi[0][i] < srsi[1][i], &[srsi[0][i], srsi[1][i], trend[i]]),
            vote(wr[i] < -80. && wr[i] > w1, wr[i] > -20. && wr[i] < w1, &[wr[i], w1]),
            vote(up && bbp[2][i] < 0. && bbp[2][i] > bear1, dn && bbp[1][i] > 0. && bbp[1][i] < bull1, &[bbp[1][i], bbp[2][i], bull1, bear1, trend[i]]),
            vote(uo[i] > 70., uo[i] < 30., &[uo[i]]),
        ];
        ma[i] = mean(&m); osc[i] = mean(&o); all[i] = (ma[i] + osc[i]) / 2.;
    }
    Ok(vec![ma, osc, all])
}
pub const DEFS: &[IndicatorDef] = &[IndicatorDef { id: "tech_ratings", name: "Technical Ratings", category: "Ratings", params: &[], outputs: &["ma", "osc", "all"],
    placement: Placement::Pane, needs: Needs::NONE, definition: "TradingView Technical Ratings: each component votes +1/−1/0; ma = mean of SMA & EMA 10/20/30/50/100/200, HMA 9, VWMA 20 (dropped without volume) and Ichimoku (as plotted); osc = mean of RSI 14, Stoch 14/3/3, CCI 20, ADX 14/14, AO, Momentum 10, MACD 12/26/9, Stoch RSI 3/3/14/14, Williams %R 14, Bull Bear Power 13, UO 7/14/28 (TradingView help rules); trend = close vs sma(close, 50) (declared); a group is NaN until all its components exist; all = (ma + osc)/2", first: &[199, 49, 199], sparse: false, compute: Some(tech_ratings), note: "" }];
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
            assert!(up[0][198].is_nan() && !up[0][199].is_nan());
            assert_eq!(up[0][299], 1.0, "every MA below price in a steady rise (volume {volume})");
        }
        let down = go("tech_ratings", &series(0.99, true), &[]);
        assert!((down[0][299] + 13. / 15.).abs() < 1e-12, "HMA overshoots in a steady fall: 13 sells, 1 buy of 15 → {}", down[0][299]);
    }
    #[test] fn overall_is_the_mean_of_the_groups() {
        let o = go("tech_ratings", &crate::lab::testdata::walk(400, 3, true), &[]);
        for i in 199..400 { assert!(close_eq(o[2][i], (o[0][i] + o[1][i]) / 2.) && !(o[1][i].abs() > 1.), "bar {i}"); }
    }
    #[test] fn rules_reach_ratings() { compiles("tech_ratings", "all").unwrap(); }
}
