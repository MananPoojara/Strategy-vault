//! Jim Berg, The Stock Trading Handbook (2005 ebook, jimberg.com.au) + his article
//! "Trading With Volatility Indicators". Only the dual moving-average crossover is complete
//! and mechanical; Berg's own volatility systems need chart reading or unstated lengths.
//! Source notes: library/bergsth/README.md.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "bergsth";
/// Admitted ids (authors/lanea/singles/decisions.json).
pub const REVIEWED_LANEA: &[&str] = &["bergsth_ma_cross", "bergsth_stoch_cross" /* lane R2A, R4 #297 */];
pub fn reviewed_lanea(id: &str) -> bool { REVIEWED_LANEA.contains(&id) }

const NT_SHORT: &str = "not testable: weekly 'higher highs & higher lows' are hand-labelled swing points (p.174, Chart 8.22), the initial stop is 'below the recent low' with no lookback (p.177), the RSI(7)-to-entry window is not given, and the JB Profit Taker's moving-average length is never stated (p.178)";
const NT_ARTICLE: &str = "not testable: 'higher highs and higher lows on a weekly chart' is chart reading, the initial stop is 'just under the recent low' (no lookback), and the article gives no fill rule or RSI-to-entry window";
const NT_LONG: &str = "not testable: sector selection, fundamentals (P/E, ROE, EPS growth scans), 'breakout' and support/resistance stops are judgement (pp.156-162)";
const NT_ENV: &str = "not testable: entry is 'when the price receives support ... near the lower band' in 'a rising market' (judgement, p.106)";
const NT_BB: &str = "not testable: no Bollinger length/width given; 'receive support at the lower band' is judgement (p.115)";

pub fn catalogue() -> Vec<StrategyDef> {
    let make = |id, name, group, dir, status, rules, notes| StrategyDef {
        id, name, group, session: 0, dir, status, rules, notes,
        needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, claim: None,
    };
    vec![
        make("bergsth_ma_cross", "Dual moving average crossover (7 / 14)", "Ch 6 · Indicators", "Long", Status::Live,
            &["F|\"Moving averages ... are calculated by adding the prices over a selected period and dividing by the unit for that period\" (p.103) -> simple MAs of the close",
              "E|\"A buy signal occurs when a shorter-term (faster) average of, say, seven days crosses above a longer-term (slower) average of fourteen days.\" (p.104)",
              "X|\"A sell signal occurs when a shorter-term moving average crosses below a longer-term average.\" (p.104)",
              "F|Orders: signal on the close, fill at the next day's open, as Berg does for traders 'who do not sit in front of [the] screen': \"Their entry is on the following day's open\" (p.177); \"exited on the following day's opening price\" (p.134)"],
            "Long only (a share trader's buy / sell, Ch 6-8). Defaults 7/14 are the book's example; \"Other commonly used periods are 10 and 30 day\" (p.104) = labelled variation via p.fast=10, p.slow=30. Book prints no results (Chart 6.4 is a picture). Instrument swap: NIFTY 50 / SPX instead of ASX shares; NSE basket stocks via basket:<list>:<SYMBOL>. Duplicates skipped: 26-day close-vs-SMA (long side of rwmts_sma26_sar), MACD 12/26/9 crossover long + next open (hbiab_macd_example)."),
        make("bergsth_short_term_vol", "Short-term volatility system (34-wk MA, RSI(7), ATR entry, trailing stop, JB Profit Taker)", "Ch 8 · Trading systems", "Long", Status::NotTestable(NT_SHORT),
            &["F|Weekly: \"Higher Highs & Higher Lows\", \"Close > moving average\" (34 week), \"Moving average rising\"; Daily: \"RSI(7) Oversold\" = \"An RSI at '30' or below\" (pp.174-175)",
              "E|\"The trigger entry level is two times the ATR from the lowest price over the previous 20 days\" (10-day ATR); \"entry is on the following day's open\" (pp.176-177)",
              "X|Initial stop: \"Recent low / Support/resistance level\"; trailing: \"Second consecutive close below trailing stop\", stop = close - 2xATR held at its highest for 15 days (pp.133, 177)",
              "X|\"JB Profit Taker ... is a moving average of the highs plus 2 times the ATR\"; \"Ignore first thrust through Profit Taker\" unless +25% (p.178)"],
            NT_SHORT),
        make("bergsth_article_vol", "Volatility entry + volatility trailing stop (article)", "Article · Trading With Volatility Indicators", "Long", Status::NotTestable(NT_ARTICLE),
            &["F|\"making higher highs and higher lows on a weekly chart; Closing prices are above the 34-week moving average; The 34-week moving average is rising\"; RSI(7) drops to 30",
              "E|\"C>(LLV(L,20) + 2*ATR(10))\"",
              "X|\"My initial stop is just under the recent low\"; \"exit the position after two consecutive closes below the trailing stop ... HHV(C-2*ATR(10),15)\""],
            NT_ARTICLE),
        make("bergsth_long_term_weekly", "Long-term weekly sector + fundamentals system", "Ch 8 · Trading systems", "Long", Status::NotTestable(NT_LONG),
            &["F|Sector & share: trend of the 34-week MA, close > MA, MA rising, \"Breakout\"; Mkt Cap > 100M, P/E < 25, ROE > 5%, EPS growth > 5% (pp.156, 161)",
              "E|\"Monday opening\" after the weekly signal (p.162)",
              "X|Initial: \"Recent low (pivot point) / Support/resistance level\", no more than 10% away; trailing: weekly volatility stop, second close below (p.162)"],
            NT_LONG),
        make("bergsth_stoch_cross", "Stochastic %K / %D crossover (MetaStock 5 / 3 / 3)", "Ch 6 · Indicators", "Long", Status::Live,
            &["F|\"The Stochastic compares the latest closing price with the total range of price action for a specified period ... The indicator is composed of two lines, the %K and the %D.\" (p.110)",
              "E|\"A buy signal is generated when %K crosses above %D\" (p.110)",
              "X|\"... and a sell signal when crossing below.\" (p.110)",
              "F|Orders: signal on the close, fill at the next day's open, as for the book's moving-average crossover: \"Their entry is on the following day's open\" (p.177); \"exited on the following day's opening price\" (p.134)"],
            "Adapted: 2 unstated details (R4 #297). Unstated in the book: the %K length (and its slowing); vault reading: MetaStock's default Stochastic Oscillator, %K periods 5 with %K slowing 3, because the book's charts are MetaStock layouts (Chart 6.9 'MetaStock Diversified Financials Layout', p.110): %K = 100 x Sum(C - LLV(L,5), 3) / Sum(HHV(H,5) - LLV(L,5), 3) (p.k, p.slow; p.slow = 1 is the unsmoothed fast %K). Unstated in the book: the %D length; vault reading: MetaStock's default %D, a 3-period simple average of %K (p.d). Long only, like the book's other share-trader signals (Ch 6-8): buy on a cross up, sell on the next cross down. 'Crosses above' = %K > %D today and %K <= %D yesterday. Chart 6.9 is a picture: no results printed. Instrument swap: indices, ETFs and stocks instead of ASX shares."),
        make("bergsth_ma_envelope", "21-day EMA 8% envelope", "Ch 6 · Indicators", "Long", Status::NotTestable(NT_ENV),
            &["E|\"In a rising market, short term trading opportunities occur when the price receives support ... near the lower band\" (21-day EMA, 8% shift)",
              "X|\"Price objective is the band at the upper level of the envelope.\" (p.106)"], NT_ENV),
        make("bergsth_bollinger", "Bollinger lower-band buy / upper-band sell", "Ch 6 · Indicators", "Long", Status::NotTestable(NT_BB),
            &["E|\"some traders will buy at the lower band and sell at the upper\" when the swing is \"greater than 10%\" (p.115)"], NT_BB),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id {
        "bergsth_ma_cross" => Some(("bergsth_ma", "Berg moving-average signals", "7/14 SMA crossover · next open")),
        "bergsth_stoch_cross" => Some(("bergsth_stoch", "Berg stochastic crossover", "MetaStock 5 / 3 / 3 · next open")),
        "bergsth_ma_envelope" | "bergsth_bollinger" => Some(("bergsth_ind", "Berg indicator signals", "Not testable")),
        "bergsth_short_term_vol" | "bergsth_article_vol" | "bergsth_long_term_weekly" => Some(("bergsth_sys", "Berg volatility systems", "Not testable")),
        _ => None,
    }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    match id {
        "bergsth_ma_cross" => Some(vec![p("fast", "Fast SMA days (book 7; variant 10)", 7., 2., 50., 1.),
                                        p("slow", "Slow SMA days (book 14; variant 30)", 14., 3., 200., 1.)]),
        "bergsth_stoch_cross" => Some(vec![p("k", "%K periods (not in the book; MetaStock default 5)", 5., 2., 50., 1.),
                                           p("slow", "%K slowing (not in the book; MetaStock default 3; 1 = fast %K)", 3., 1., 10., 1.),
                                           p("d", "%D periods, simple average of %K (not in the book; MetaStock default 3)", 3., 1., 20., 1.)]),
        _ => None,
    }
}

/// Window-sum SMA (no running-sum drift); NaN until n bars exist.
fn sma(v: &[f64], n: usize) -> Vec<f64> {
    (0..v.len()).map(|i| if n == 0 || i + 1 < n { f64::NAN } else { v[i + 1 - n..=i].iter().sum::<f64>() / n as f64 }).collect()
}

/// MetaStock Stochastic Oscillator: %K = 100 x Sum(C - LLV(L,k), slow) / Sum(HHV(H,k) - LLV(L,k), slow); %D = simple MA of %K.
fn stoch(x: &Ctx, k: usize, slow: usize, d: usize) -> (Vec<f64>, Vec<f64>) {
    let n = x.c.len();
    let (mut num, mut den) = (vec![f64::NAN; n], vec![f64::NAN; n]);
    for i in k - 1..n {
        let hh = x.h[i + 1 - k..=i].iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let ll = x.l[i + 1 - k..=i].iter().copied().fold(f64::INFINITY, f64::min);
        num[i] = x.c[i] - ll; den[i] = hh - ll;
    }
    let (sn, sd) = (sma(&num, slow), sma(&den, slow));
    let kk: Vec<f64> = (0..n).map(|i| if sd[i] > 0. { 100. * sn[i] / sd[i] } else { f64::NAN }).collect();
    let dd = sma(&kk, d);
    (kk, dd)
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap().def);
    if from >= to || to >= x.c.len() { return None; }
    let (f, s, what) = match id {
        "bergsth_ma_cross" => {
            let (fast, slow) = (g("fast"), g("slow"));
            if fast.fract() != 0. || slow.fract() != 0. || fast < 1. || fast >= slow { return None; }
            (sma(&x.c, fast as usize), sma(&x.c, slow as usize), "Fast SMA crossed below slow · next open")
        }
        "bergsth_stoch_cross" => {
            let (k, sl, d) = (g("k"), g("slow"), g("d"));
            if [k, sl, d].iter().any(|v| v.fract() != 0. || *v < 1.) || k < 2. { return None; }
            let (kk, dd) = stoch(x, k as usize, sl as usize, d as usize);
            (kk, dd, "%K crossed below %D · next open")
        }
        _ => return None,
    };
    let above = |i: usize| f[i] > s[i];
    let crossed = |i: usize, up: bool| i > 0 && [f[i], s[i], f[i - 1], s[i - 1]].iter().all(|v| v.is_finite())
        && if up { above(i) && f[i - 1] <= s[i - 1] } else { f[i] < s[i] && f[i - 1] >= s[i - 1] };
    let (mut out, mut cur): (Vec<Trade>, Option<Trade>) = (vec![], None);
    for i in from..to {
        if let Some(t) = cur.as_mut() {
            if crossed(i, false) {
                t.exit_i = i + 1; t.exit_px = x.o[i + 1]; t.signal_i = i; t.reason = what.into();
                out.push(cur.take().unwrap());
            }
        } else if crossed(i, true) {
            cur = Some(Trade { dir: 1., legs: vec![Leg { i: i + 1, px: x.o[i + 1], w: 1. }], exit_i: i + 1, exit_px: x.o[i + 1],
                reason: String::new(), entry_kind: "open", signal_i: i, open: false, exits: vec![] });
        }
    }
    if let Some(mut t) = cur { t.exit_i = to; t.exit_px = x.c[to]; t.reason = "open".into(); t.open = true; out.push(t); }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn cross_up_buys_next_open_and_cross_down_sells_next_open() {
        // closes: flat 100, rise to 120 (fast crosses up), then fall to 80 (fast crosses down)
        let closes: Vec<f64> = (0..60).map(|i| if i < 20 { 100. } else if i < 35 { 120. } else { 80. }).collect();
        let r: Vec<Row> = closes.iter().enumerate().map(|(i, &c)| Row { ticker: "T".into(), date: format!("{:04}-01-01", 1900 + i),
            open: c + 1., high: c + 5., low: c - 5., close: c, volume: None }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let ts = run("bergsth_ma_cross", &x, 0, 59, &Params::new()).unwrap();
        assert_eq!(ts.len(), 1);
        // day 20: fast(7) = (6*100+120)/7 > slow(14) = (13*100+120)/14, both previously equal -> signal 20, buy open 21
        assert_eq!((ts[0].signal_i, ts[0].legs[0].i, ts[0].legs[0].px), (35, 21, 121.));
        // day 35: first close 80; fast = (6*120+80)/7=114.3 < slow=(13*120+80)/14=117.1 -> sell open 36 = 81
        assert_eq!((ts[0].exit_i, ts[0].exit_px), (36, 81.));
        assert!(run("bergsth_short_term_vol", &x, 0, 59, &Params::new()).is_none());
    }
    #[test]
    fn stoch_cross_metastock_5_3_3_hand_checked() {
        // lane R2A (R4 #297): %K(5, slowing 3), %D = SMA3; flat bars H=c+1, L=c-1.
        let closes = [10., 10., 10., 10., 10., 10., 10., 9., 8., 7., 6., 7., 8., 9., 10., 9., 8., 7., 6., 5.];
        let r: Vec<Row> = closes.iter().enumerate().map(|(i, &c)| Row { ticker: "T".into(), date: format!("{:04}-01-01", 1900 + i),
            open: c + 0.5, high: c + 1., low: c - 1., close: c, volume: None }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let (k, d) = stoch(&x, 5, 3, 3);
        // bar 6: every window flat -> num 1, den 2 per bar -> %K 50
        assert!((k[6] - 50.).abs() < 1e-9 && k[5].is_nan() && d[8].is_finite() && d[7].is_nan());
        let ts = run("bergsth_stoch_cross", &x, 0, 19, &Params::new()).unwrap();
        // first cross up after the low at bar 10 (close 6) -> buy at the next open; then cross down when prices fall again
        assert!(!ts.is_empty());
        let t = &ts[0];
        let i = t.legs[0].i - 1;
        assert!(k[i] > d[i] && k[i - 1] <= d[i - 1]);
        if !t.open { let j = t.exit_i - 1; assert!(t.signal_i == j && k[j] < d[j] && k[j - 1] >= d[j - 1] && t.exit_px == x.o[t.exit_i]); }
        assert_eq!(t.legs[0].px, x.o[t.legs[0].i]);
    }
}

// Instrument admission: Jim Berg (bergsth, lane A) 2026-10-26 (lane B) block, moved from instrument_policy.rs on 31 Oct 2026 (lane ST).
// Admission only for the module REVIEWED lists (authors/lanea/singles/decisions.json); kept in one block to merge.
// Runs on actual indices, ETFs and stocks, US and India (index_etf_or_stock, nifty50 basket, declared application).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book=="bergsth" && crate::books::bergsth::reviewed_lanea(def.id),
    label: "Berg's moving-average and stochastic crossovers were written for ASX shares; Runs on indices, ETFs and single stocks (US and India) are a declared application, one unit per trade.",
    basis: "Jim Berg / The Stock Trading Handbook (2005)",
};
