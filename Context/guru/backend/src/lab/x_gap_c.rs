//! Platform-survey gap group "c" (2026-11-02): indicators the external platform survey found missing from the lab.
//! Computed here: Augen Price Spike, Bill Williams Divergent Bars, MOBO Bands, Kirshenbaum Bands, Uni-Channel, Volatility
//! Ratio (Schwager), Volatility Cone (Burghardt & Lane, causal rolling form), Probability Cone (causal), The Strat bar types
//! and Pesavento Patterns (swing ratios on confirmed swings). Listed only: ATR Extension, Boring and Explosive Candles, Gap
//! and Reverse, Historical Volatility Bands and Probability Box (no single published rule), Elliott Wave (user-placed labels).
//! Not added: Breakout (already the donchian id: close > donchian.upper[1]); Hogscraper (a c. 1780–1860 candle holder, not a
//! chart pattern: the census false positive already skipped in x_census_candles.rs). Formulas: see each definition string.
use super::bars::Bars;
use super::catalogue::{int, len, n, p, real, IndicatorDef, Needs, Output, ParamSpec, Params, Placement};
use super::ta::{self, NA};

#[allow(clippy::too_many_arguments)]
const fn def(id: &'static str, name: &'static str, category: &'static str, params: &'static [ParamSpec], outputs: &'static [&'static str], placement: Placement,
             needs: Needs, definition: &'static str, first: &'static [usize], sparse: bool, f: fn(&Bars, &Params) -> Output) -> IndicatorDef {
    IndicatorDef { id, name, category, params, outputs, placement, needs, definition, first, sparse, compute: Some(f), note: "" }
}
/// Listed in the picker, not computed (the app shows the note).
const fn listed(id: &'static str, name: &'static str, category: &'static str, definition: &'static str, note: &'static str) -> IndicatorDef {
    IndicatorDef { id, name, category, params: &[], outputs: &["value"], placement: Placement::Pane, needs: Needs::NONE, definition,
                   first: &[usize::MAX], sparse: false, compute: None, note }
}
const NO: Needs = Needs::NONE;
const PR: Placement = Placement::Price;
const PN: Placement = Placement::Pane;
fn flag(b: bool) -> f64 { if b { 1. } else { 0. } }
/// ln(close[i]/close[i − 1]); NaN on bar 0 or a non-positive close.
fn log_returns(c: &[f64]) -> Vec<f64> {
    (0..c.len()).map(|i| if i > 0 && c[i] > 0. && c[i - 1] > 0. { (c[i] / c[i - 1]).ln() } else { NA }).collect()
}

const VR: &[ParamSpec] = &[len("length", "Length (EMA of true range)", 14.)];
/// today's true range / ema(true range, n); bar 0 has no true range.
fn volatility_ratio(b: &Bars, prm: &Params) -> Output {
    let tr = ta::tr(b, false);
    let e = ta::ema(&tr, n(prm, "length"));
    Ok(vec![(0..b.len()).map(|i| if e[i] == 0. { NA } else { tr[i] / e[i] }).collect()])
}

const KB: &[ParamSpec] = &[len("length", "EMA length", 30.), int("se_length", "Standard-error length", 20., 2., 2000.), real("mult", "Multiplier", 1., 0.1, 10., 0.05)];
/// √(Σ residual²/n) of the least-squares line through the window (x = 1..n).
fn se_window(w: &[f64]) -> f64 {
    let nf = w.len() as f64;
    let (mut sk, mut sy, mut skk, mut sky) = (0., 0., 0., 0.);
    for (j, &y) in w.iter().enumerate() { let k = (j + 1) as f64; sk += k; sy += y; skk += k * k; sky += k * y; }
    let den = nf * skk - sk * sk;
    let slope = if den == 0. { 0. } else { (nf * sky - sk * sy) / den };
    let a = (sy - slope * sk) / nf;
    (w.iter().enumerate().map(|(j, &y)| (y - a - slope * (j + 1) as f64).powi(2)).sum::<f64>() / nf).sqrt()
}
fn kirshenbaum_bands(b: &Bars, prm: &Params) -> Output {
    let (k, m, len) = (n(prm, "se_length"), p(prm, "mult"), b.len());
    let mid = ta::ema(&b.close, n(prm, "length"));
    let mut se = vec![NA; len];
    for t in k.saturating_sub(1)..len { se[t] = se_window(&b.close[t + 1 - k..=t]); }
    Ok(vec![(0..len).map(|i| mid[i] + m * se[i]).collect(), mid.clone(), (0..len).map(|i| mid[i] - m * se[i]).collect()])
}

const UC: &[ParamSpec] = &[len("length", "SMA length", 10.), real("upper_factor", "Upper factor", 0.02, 0., 1e6, 0.01), real("lower_factor", "Lower factor", 0.02, 0., 1e6, 0.01),
    int("type", "Factor type (0 fraction of the average, 1 fixed price amount)", 0., 0., 1.)];
fn uni_channel(b: &Bars, prm: &Params) -> Output {
    let (u, l, pts) = (p(prm, "upper_factor"), p(prm, "lower_factor"), n(prm, "type") == 1);
    let ma = ta::sma(&b.close, n(prm, "length"));
    let up: Vec<f64> = ma.iter().map(|&m| if pts { m + u } else { m + m * u }).collect();
    let dn: Vec<f64> = ma.iter().map(|&m| if pts { m - l } else { m - m * l }).collect();
    Ok(vec![up, ma, dn])
}

const MB: &[ParamSpec] = &[len("length", "Length", 10.), real("dev_up", "Deviations above", 0.8, 0., 10., 0.05), real("dev_dn", "Deviations below", 0.8, 0., 10., 0.05)];
fn mobo_bands(b: &Bars, prm: &Params) -> Output {
    let (k, du, dd, len) = (n(prm, "length"), p(prm, "dev_up"), p(prm, "dev_dn"), b.len());
    let (ma, sd) = (ta::sma(&b.close, k), ta::stdev(&b.close, k));
    let up: Vec<f64> = (0..len).map(|i| ma[i] + du * sd[i]).collect();
    let dn: Vec<f64> = (0..len).map(|i| ma[i] - dd * sd[i]).collect();
    let mut state = vec![NA; len];
    let mut s = 0.;
    for i in 0..len {
        if ma[i].is_nan() { continue; }
        if b.close[i] > up[i] { s = 1. } else if b.close[i] < dn[i] { s = -1. }
        state[i] = s;
    }
    Ok(vec![up, ma, dn, state])
}

const AP: &[ParamSpec] = &[int("length", "Length (returns)", 20., 2., 2000.)];
fn augen_price_spike(b: &Bars, prm: &Params) -> Output {
    let (k, c) = (n(prm, "length"), &b.close);
    let r = log_returns(c);
    // ln(close[1]/close[2]): the returns before today's bar
    let lagged: Vec<f64> = (0..c.len()).map(|i| if i >= 1 { r[i - 1] } else { NA }).collect();
    let sd = ta::stdev(&lagged, k);
    let adj = (k as f64 / (k as f64 - 1.)).sqrt();
    Ok(vec![(0..c.len()).map(|t| {
        if sd[t].is_nan() { return NA; }
        let m = sd[t] * adj * c[t - 1];
        if m == 0. { 0. } else { (c[t] - c[t - 1]) / m }
    }).collect()])
}

fn divergent_bars(b: &Bars, _: &Params) -> Output {
    let mut o = vec![vec![NA; b.len()]; 2];
    for t in 1..b.len() {
        let mid = (b.high[t] + b.low[t]) / 2.;
        o[0][t] = flag(b.low[t] < b.low[t - 1] && b.close[t] > mid);
        o[1][t] = flag(b.high[t] > b.high[t - 1] && b.close[t] < mid);
    }
    Ok(o)
}

fn the_strat(b: &Bars, _: &Params) -> Output {
    let mut o = vec![NA; b.len()];
    for t in 1..b.len() {
        let (up, dn) = (b.high[t] > b.high[t - 1], b.low[t] < b.low[t - 1]);
        o[t] = match (up, dn) { (true, true) => 3., (true, false) => 2., (false, true) => -2., (false, false) => 1. };
    }
    Ok(vec![o])
}

const VC: &[ParamSpec] = &[len("length", "Volatility horizon (returns)", 20.), int("lookback", "History window (bars)", 252., 2., 5000.),
    int("days", "Annualisation days", 252., 1., 366.)];
fn volatility_cone(b: &Bars, prm: &Params) -> Output {
    let (lb, days, len) = (n(prm, "lookback"), p(prm, "days"), b.len());
    let hv: Vec<f64> = ta::stdev(&log_returns(&b.close), n(prm, "length")).iter().map(|s| 100. * s * days.sqrt()).collect();
    let mut o = vec![hv.clone(), vec![NA; len], vec![NA; len], vec![NA; len]];
    for t in lb.saturating_sub(1)..len {
        let w = &hv[t + 1 - lb..=t];
        if w.iter().any(|v| v.is_nan()) { continue; }
        let mut s = w.to_vec();
        s.sort_by(|a, b| a.total_cmp(b));
        let med = if lb % 2 == 1 { s[lb / 2] } else { (s[lb / 2 - 1] + s[lb / 2]) / 2. };
        (o[1][t], o[2][t], o[3][t]) = (s[lb - 1], med, s[0]);
    }
    Ok(o)
}

const PC: &[ParamSpec] = &[len("length", "Volatility length (returns)", 20.), len("horizon", "Horizon (bars)", 20.), real("mult", "Standard deviations", 1., 0.1, 10., 0.1)];
fn probability_cone(b: &Bars, prm: &Params) -> Output {
    let (h, z, len) = (n(prm, "horizon"), p(prm, "mult"), b.len());
    let sd = ta::stdev(&log_returns(&b.close), n(prm, "length"));
    let w = z * (h as f64).sqrt();
    let fu: Vec<f64> = (0..len).map(|t| b.close[t] * (w * sd[t]).exp()).collect();
    let fl: Vec<f64> = (0..len).map(|t| b.close[t] * (-w * sd[t]).exp()).collect();
    let shift = |x: &[f64]| (0..len).map(|t| if t >= h { x[t - h] } else { NA }).collect::<Vec<f64>>();
    Ok(vec![shift(&fu), shift(&fl), fu, fl])
}

/// Larry Pesavento's ratio set as printed in the ZUP documentation (MQL5 article 1470).
const PESAVENTO: [f64; 15] = [0.25, 0.382, 0.5, 0.618, 0.707, 0.786, 0.841, 1.0, 1.128, 1.272, 1.414, 1.618, 2.0, 2.618, 4.0];
const PP: &[ParamSpec] = &[real("dev", "Swing reversal (%)", 5., 0.1, 100., 0.1), real("tolerance", "Tolerance (fraction of the level)", 0.04, 0., 1., 0.01)];
fn pesavento_patterns(b: &Bars, prm: &Params) -> Output {
    let (d, tol, len) = (p(prm, "dev") / 100., p(prm, "tolerance"), b.len());
    let mut o = vec![vec![NA; len]; 2];
    if len == 0 { return Ok(o); }
    let mut piv: Vec<f64> = Vec::new();
    let (mut dir, mut ext) = (0i8, NA);
    let (mut hi, mut hi_i, mut lo, mut lo_i) = (b.high[0], 0usize, b.low[0], 0usize);
    let (mut ratio, mut level) = (NA, NA);
    for t in 1..len {
        let mut new = false;
        match dir {
            0 => {
                if b.high[t] > hi { (hi, hi_i) = (b.high[t], t); }
                if b.low[t] < lo { (lo, lo_i) = (b.low[t], t); }
                if hi_i > lo_i && hi >= lo * (1. + d) { piv.push(lo); (dir, ext, new) = (1, hi, true); }
                else if lo_i > hi_i && lo <= hi * (1. - d) { piv.push(hi); (dir, ext, new) = (-1, lo, true); }
            }
            1 => {
                if b.high[t] > ext { ext = b.high[t]; }
                else if b.low[t] <= ext * (1. - d) { piv.push(ext); (dir, ext, new) = (-1, b.low[t], true); }
            }
            _ => {
                if b.low[t] < ext { ext = b.low[t]; }
                else if b.high[t] >= ext * (1. + d) { piv.push(ext); (dir, ext, new) = (1, b.high[t], true); }
            }
        }
        if new && piv.len() >= 3 {
            let k = piv.len();
            let prev = (piv[k - 2] - piv[k - 3]).abs();
            ratio = if prev == 0. { NA } else { (piv[k - 1] - piv[k - 2]).abs() / prev };
            level = NA;
            let mut best = f64::INFINITY;
            for &lv in &PESAVENTO {
                let e = (ratio / lv - 1.).abs();
                if e <= tol && e < best { (best, level) = (e, lv); }
            }
        }
        (o[0][t], o[1][t]) = (ratio, level);
    }
    Ok(o)
}

const UNP: &str = "Unpublished formulas";
pub const DEFS: &[IndicatorDef] = &[
    def("volatility_ratio", "Volatility Ratio (Schwager)", "Volatility", VR, &["vr"], PN, NO,
        "Jack Schwager's Volatility Ratio: vr = today's true range / ema(true range, length), length 14. True range = max(high − low, |high − close[1]|, |low − close[1]|); bar 0 has none. The EMA includes today's true range (MotiveWave and Incredible Charts word it as today's TR over the EMA of TR over x days); EMA seeded with the SMA of the first length true ranges (the lab's ta::ema). A zero EMA gives NaN. Values above 2.0 mark wide-ranging days (Incredible Charts). Blank before bar length. Not the Historical Volatility Ratio (hv_ratio), and not variants that divide by the n-day true-range span or exclude today's bar from the average. Sources: Schwager, Schwager on Futures: Technical Analysis (Wiley 1996); MotiveWave Volatility Ratio (https://docs.motivewave.com/studies/u-v); Incredible Charts, Volatility Ratio (Schwager) (https://incrediblecharts.com/indicators/volatility_ratio_schwager.php).",
        &[14], false, volatility_ratio),
    def("kirshenbaum_bands", "Kirshenbaum Bands", "Bands & channels", KB, &["upper", "middle", "lower"], PR, NO,
        "Paul Kirshenbaum's bands: middle = ema(close, length 30); se = standard error of the least-squares line fitted to the last se_length (20) closes, √(Σ(close − fitted)²/se_length) (GeniusTrader StandardError: the average of the squared distances to the regression line, then the square root); upper/lower = middle ± mult·se, mult 1. Defaults 30 / 20 / 1 are MotiveWave's (Kirshenbaum is reported to have recommended 1.75; unverified forum post, so not the default). Note the divisor: the lab's stderr id (AmiBroker StdErr) divides by n − 2, this one by n. Blank before bar max(length, se_length) − 1 (29). Sources: MotiveWave Kirshenbaum Bands (https://docs.motivewave.com/studies/k-l, ma = EMA(period1), sdErr = stdErr(period2), top/bottom = ma ± sdErr·noSdDev); GeniusTrader GT::Indicators::KirshenbaumBands and StandardError (https://geniustrader.sourceforge.net/doc/GT/Indicators/StandardError.pm.html).",
        &[29, 29, 29], false, kirshenbaum_bands),
    def("uni_channel", "Uni-Channel", "Bands & channels", UC, &["upper", "middle", "lower"], PR, NO,
        "Omega Research (1997) Uni-Channel, as MotiveWave publishes it: middle = sma(close, length 10). type 0 (default): upper = middle + middle·upper_factor, lower = middle − middle·lower_factor (factor 0.02 = 2 %); type 1: upper = middle + upper_factor, lower = middle − lower_factor (fixed price amounts). MotiveWave's signals (sell when high > upper, buy when low < lower) are left to rules. With equal percentage factors it equals the envelope id on an SMA. Blank before bar length − 1. Source: MotiveWave Uni Channel (https://docs.motivewave.com/studies/u-v#uni-channel).",
        &[9, 9, 9], false, uni_channel),
    def("mobo_bands", "MOBO Bands (Momentum Breakout Bands)", "Bands & channels", MB, &["upper", "middle", "lower", "state"], PR, NO,
        "David Elliot's MOBO (Momentum Breakout) Bands: middle = sma(close, length 10); upper = middle + dev_up·stdev(close, length), lower = middle − dev_dn·stdev(close, length), dev 0.8 both (population standard deviation, as Pine ta.stdev and thinkorswim StDev); i.e. narrow Bollinger Bands. state = +1 after a close above upper (breakout), −1 after a close below lower (breakdown), held otherwise; 0 until the first break. Blank before bar length − 1. thinkorswim's displace input is not reproduced (0). Sources: TradingView community MoBo scripts (https://www.tradingview.com/script/kRkcnXyP-MoBo/: Length 10, Num Dev Up 0.8, Num Dev Down −0.8); thinkorswim FW MOBO Basic / Advanced (https://toslc.thinkorswim.com/center/reference/Tech-Indicators/studies-library/E-F/FW-MOBO-Basic); Wealth-Lab forum, David Elliot's Mobo (https://wl6.wealth-lab.com/Forum/Posts/David-Elliot-39-s-Mobo-Momentum-Breakout-Bands-39767).",
        &[9, 9, 9, 9], false, mobo_bands),
    def("augen_price_spike", "Augen Price Spike", "Volatility", AP, &["spike"], PN, NO,
        "Jeff Augen's price spike (The Volatility Edge in Options Trading, 2008): today's close-to-close change in standard deviations of the preceding returns. r = ln(close[1]/close[2]); sd = stdev(r, length)·√(length/(length − 1)) (sample standard deviation, as the thinkorswim script); spike = (close − close[1])/(sd·close[1]). length 20. If sd·close[1] = 0 the value is 0 (QuantConnect Lean AugenPriceSpike). Today's return is not in its own standard deviation. Blank before bar length + 1. Sources: usethinkscript, Price Spikes Indicators for ThinkOrSwim (https://usethinkscript.com/threads/price-spikes-indicators-for-thinkorswim.3077/); QuantConnect Lean AugenPriceSpike.cs, Apache-2.0 (https://github.com/QuantConnect/Lean/blob/master/Indicators/AugenPriceSpike.cs; Lean's default period 3 and population deviation differ).",
        &[21], false, augen_price_spike),
    def("divergent_bars", "Divergent Bars (Bill Williams)", "Candlestick patterns", &[], &["bull", "bear"], PN, NO,
        "Bill Williams' divergent bars (Trading Chaos, 2nd ed., 2004): bull = 1 when low < low[1] and close > (high + low)/2 (a lower low closing in the upper half), bear = 1 when high > high[1] and close < (high + low)/2 (a higher high closing in the lower half), else 0; a close exactly at the midpoint is neither. Williams also wants the bar well away from the Alligator (angulation) with the Awesome Oscillator agreeing; that location filter is not built in: combine with the alligator and awesome ids in a rule. Blank on bar 0. Sources: TradingView community Bill Williams Divergent Bars (https://www.tradingview.com/script/lLgCdjag-Bill-Williams-Divergent-Bars/); LuxAlgo, Bill Williams Profitunity (https://www.luxalgo.com/library/concept/bill-williams-profitunity/).",
        &[1, 1], false, divergent_bars),
    def("the_strat", "The Strat bar type (Rob Smith)", "Candlestick patterns", &[], &["type"], PN, NO,
        "Rob Smith's The Strat bar numbering against the previous bar: 1 = inside (high ≤ high[1] and low ≥ low[1]; an equal high or low does not take it out), 2 = directional up (2U: high > high[1], low ≥ low[1]), −2 = directional down (2D: low < low[1], high ≤ high[1]), 3 = outside (high > high[1] and low < low[1]). Combos (e.g. 2-1-2, 3-1-2) and timeframe continuity are rules on type and type[1], type[2]. Blank on bar 0. Sources: TrendSpider, A comprehensive guide to TheStrat (https://trendspider.com/learning-center/a-comprehensive-guide-to-thestrat-trading-strategy-for-the-modern-trader/); TradingView community The Strat (https://www.tradingview.com/script/ZCXMMKx5-The-Strat/).",
        &[1], false, the_strat),
    def("volatility_cone", "Volatility Cone (rolling, causal)", "Volatility", VC, &["hv", "max", "median", "min"], PN, NO,
        "Burghardt & Lane volatility cone (How to Tell if Options Are Cheap, Journal of Portfolio Management, Winter 1990) for one horizon: hv = 100·stdev(ln(close/close[1]), length)·√days (population stdev, as the hv id), length 20, days 252; max / median / min = the largest, median and smallest hv over the last lookback (252) bars including today. Run it again with another length for each horizon of the cone (e.g. 20, 40, 60, 120). Adaptation: the published cone uses the whole history once; here the distribution is a trailing window ending at each bar, so it is causal. Overlapping windows are not bias-corrected, and only min / median / max are given (no quartiles). Compare with implied volatility for the matching horizon outside the lab (implied volatility is not held). hv blank before bar length; the others before bar length + lookback − 1 (271). Sources: Burghardt & Lane (1990); LuxAlgo, Volatility Cones (https://www.luxalgo.com/library/concept/volatility-cones/); Montréal Exchange, Volatility cones (https://www.m-x.ca/f_publications_en/cone_vol_en.pdf).",
        &[20, 271, 271, 271], false, volatility_cone),
    def("probability_cone", "Probability Cone (historical volatility, causal)", "Volatility", PC, &["upper", "lower", "fwd_upper", "fwd_lower"], PR, NO,
        "Lognormal expected-move cone: sd = stdev(ln(close/close[1]), length) per bar (population), length 20; fwd_upper / fwd_lower = close·exp(±mult·sd·√horizon), the mult-sigma range expected horizon bars after this bar (mult 1 ≈ 68 %, 2 ≈ 95 %), horizon 20. upper / lower = fwd_upper / fwd_lower of horizon bars ago, i.e. the cone edge for this bar drawn from the bar the cone started on (compare today's close with it). Adaptation: platforms draw the cone into future bars from the last bar; here each bar's cone is a series value, so it is causal. thinkorswim's Probability of Expiring Cone uses option implied volatility, which the lab does not hold; historical volatility substitutes, with no drift. fwd_* blank before bar length, upper/lower before bar length + horizon. Sources: TradingView community Probability Cone (https://www.tradingview.com/script/uPhPuNle-Probability-Cone/: expected move from a selectable volatility estimator, SD multiple); Hahn-Tech, offset Probability Of Expiring Cone (https://www.hahn-tech.com/?p=4773); lognormal price model (Hull, Options, Futures and Other Derivatives, ch. 15).",
        &[40, 40, 20, 20], false, probability_cone),
    def("pesavento_patterns", "Pesavento Patterns (swing ratios, confirmed swings)", "Chart patterns", PP, &["ratio", "level"], PN, NO,
        "Larry Pesavento's pattern ratios as the MetaTrader ZUP Pesavento Patterns label them: the size of each new zigzag leg divided by the size of the leg before it, and the Pesavento level it matches. Zigzag (high/low, dev 5 %): in an up-leg a higher high extends the leg, otherwise a low ≤ extreme·(1 − dev/100) confirms the swing high and starts a down-leg at that low (mirror for down-legs); the first swing is the earlier of the running high/low since bar 0 once the range reaches dev. On the bar a swing is confirmed (three swings known): ratio = |P3 − P2|/|P2 − P1| (the completed leg over the leg before it); level = the nearest of 0.25, 0.382, 0.5, 0.618, 0.707, 0.786, 0.841, 1, 1.128, 1.272, 1.414, 1.618, 2, 2.618, 4 with |ratio/level − 1| ≤ tolerance (0.04, Pesavento's 4 %), else NaN. Both are held until the next confirmed swing. Adaptation: ZUP labels the current (still moving) leg on a repainting zigzag; here only confirmed swings count, so values are causal and appear when the swing is confirmed, not at the pivot bar. Time ratios and area ratios are not given. Sparse: NaN until three swings are confirmed, level NaN when no level matches. Sources: MQL5 article 1470, ZUP indicator, Pesavento Patterns (https://www.mql5.com/en/articles/1470); Pesavento, Fibonacci Ratios with Pattern Recognition (Traders Press, 1997).",
        &[X, X], true, pesavento_patterns),
    listed("atr_extension", "ATR Extension", UNP,
        "ATR Extension: distance of price from a moving average in ATR multiples. The listing platform was not identified and the public scripts disagree: (close − MA)/ATR, (high or low − MA)/close over ATR/close with a daily ATR(20), ((price − MA50)/MA50)/(ATR14/price), with SMA or EMA and 20/50/200 lengths (TradingView community scripts qExl3i5v by shftd, oFDWGReF by BB_9791).",
        "not available yet — formula not published (listing platform not identified; the public TradingView scripts use different prices, averages and ATR normalisations)"),
    listed("boring_explosive_candles", "Boring and Explosive Candles", UNP,
        "Supply/demand teaching terms: a boring (base) candle has a small body relative to its range, an explosive (leg) candle a large body and a large range. Community scripts set different thresholds (boring body ≤ 50 % of range; explosive body > 50, 60 or 70 % of range, sometimes with range ≥ an ATR multiple or ≥ 1.5–1.9 × the base candle).",
        "not available yet — formula not published (listing platform not identified; community scripts use different body and ATR thresholds)"),
    listed("gap_and_reverse", "Gap and Reverse", UNP,
        "Gap and Reverse: a bar that gaps away from the previous bar and closes back the other way. No published rule found for the gap size or how far the close must reverse; it is not David Pieper's Gap Reversal (a ≥ 1 % gap-down whose bar closes above its open and its midpoint, buy only).",
        "not available yet — formula not published (listing platform not identified; no rule found for the gap size or the reversal)"),
    listed("hv_bands", "Historical Volatility Bands", UNP,
        "Historical Volatility Bands: a moving-average middle line with bands set from historical volatility (Mladen Rakic's MetaTrader 5 version, LuxAlgo's). Neither page gives the band formula (whether annualised HV is added to or multiplied with the average, which estimator, which average).",
        "not available yet — formula not published (no platform lists it; the MT5 and LuxAlgo pages do not give the band formula)"),
    listed("probability_box", "Probability Box", UNP,
        "Probability Box: no indicator of this name was found; similar tools (BigBeluga Low Volatility Range Breaks, LuxAlgo Probabilistic Breakout Forecaster, IV Probability Ranges) each define their own boxes.",
        "not available yet — formula not published (no platform lists it; no primary definition found)"),
    listed("elliott_wave", "Elliott Wave", "Drawings & display",
        "Elliott Wave (R. N. Elliott, The Wave Principle, 1938): impulse (1-2-3-4-5) and corrective (A-B-C, triangle, combinations) wave counts. TradingView and Sierra Chart offer them as drawing tools whose wave points the user places; the count is a subjective judgement, not a calculation. For causal swing measurements use zigzag or pesavento_patterns.",
        "chart drawing/display — wave labels placed by the user; not a bar-by-bar series; not usable in rules"),
];
const X: usize = usize::MAX;
/// (id, source, licence/notes), one per entry.
pub const SOURCES: &[(&str, &str, &str)] = &[
    ("volatility_ratio", "MotiveWave: Volatility Ratio; Incredible Charts: Volatility Ratio (Schwager)", ""),
    ("kirshenbaum_bands", "MotiveWave: Kirshenbaum Bands; GeniusTrader: KirshenbaumBands", ""),
    ("uni_channel", "MotiveWave: Uni Channel (Omega Research)", ""),
    ("mobo_bands", "TradingView (community): MoBo Bands; thinkorswim: FW MOBO Basic / Advanced", ""),
    ("augen_price_spike", "thinkorswim (usethinkscript): Augen Price Spikes; QuantConnect Lean: AugenPriceSpike", "Lean: Apache-2.0"),
    ("divergent_bars", "TradingView (community): Bill Williams Divergent Bars", ""),
    ("the_strat", "TradingView (community): The Strat; TrendSpider: TheStrat", ""),
    ("volatility_cone", "Volatility Cone (Burghardt & Lane 1990)", ""),
    ("probability_cone", "TradingView (community): Probability Cone; thinkorswim: Probability of Expiring Cone", ""),
    ("pesavento_patterns", "MetaTrader (ZUP): Pesavento Patterns", ""),
    ("atr_extension", "ATR Extension", ""),
    ("boring_explosive_candles", "Boring and Explosive Candles", ""),
    ("gap_and_reverse", "Gap and Reverse", ""),
    ("hv_bands", "Historical Volatility Bands", ""),
    ("probability_box", "Probability Box", ""),
    ("elliott_wave", "TradingView: Elliott Wave drawing tools; Sierra Chart: Elliott Wave", ""),
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lab::catalogue::{get, run};
    use crate::lab::testdata::{assert_series, closes, compiles, from_ohlc, go, walk};
    const N: f64 = f64::NAN;
    const E: f64 = std::f64::consts::E;

    #[test] fn volatility_ratio_is_tr_over_its_ema() {
        let b = from_ohlc(&[(10., 11., 9., 10.), (10., 12., 10., 11.), (11., 12., 11., 11.5), (11.5, 15., 11., 14.)]);
        // tr = –, 2, 1, 4; ema(2): seed 1.5 on bar 2, then 1.5 + 2/3·2.5 = 19/6
        assert_series(&go("volatility_ratio", &b, &[("length", 2.)])[0], &[N, N, 1. / 1.5, 4. / (19. / 6.)]);
    }
    #[test] fn kirshenbaum_uses_the_regression_standard_error() {
        let o = go("kirshenbaum_bands", &closes(&[1., 2., 4., 4.]), &[("length", 2.), ("se_length", 3.)]);
        // ema 1.5, 19/6, 67/18; se [1,2,4] = √(1/18), [2,4,4] = √(2/9)
        assert_series(&o[1], &[N, 1.5, 19. / 6., 67. / 18.]);
        assert_series(&o[0], &[N, N, 19. / 6. + (1f64 / 18.).sqrt(), 67. / 18. + (2f64 / 9.).sqrt()]);
        assert_series(&o[2], &[N, N, 19. / 6. - (1f64 / 18.).sqrt(), 67. / 18. - (2f64 / 9.).sqrt()]);
    }
    #[test] fn uni_channel_percent_and_points() {
        let b = closes(&[1., 2., 3.]);
        let o = go("uni_channel", &b, &[("length", 2.), ("upper_factor", 0.1), ("lower_factor", 0.1)]);
        assert_series(&o[0], &[N, 1.65, 2.75]);
        assert_series(&o[2], &[N, 1.35, 2.25]);
        let o = go("uni_channel", &b, &[("length", 2.), ("upper_factor", 0.5), ("lower_factor", 0.25), ("type", 1.)]);
        assert_series(&o[0], &[N, 2., 3.]);
        assert_series(&o[1], &[N, 1.5, 2.5]);
        assert_series(&o[2], &[N, 1.25, 2.25]);
    }
    #[test] fn mobo_state_holds_until_the_other_band_breaks() {
        let o = go("mobo_bands", &closes(&[1., 2., 3., 10., 1., 1.]), &[("length", 2.), ("dev_up", 0.5), ("dev_dn", 0.5)]);
        // sma 1.5, 2.5, 6.5, 5.5, 1; stdev .5, .5, 3.5, 4.5, 0
        assert_series(&o[0], &[N, 1.75, 2.75, 8.25, 7.75, 1.]);
        assert_series(&o[2], &[N, 1.25, 2.25, 4.75, 3.25, 1.]);
        assert_series(&o[3], &[N, 1., 1., 1., -1., -1.]);
    }
    #[test] fn augen_spike_in_prior_standard_deviations() {
        // lagged returns: bar 2: 1, bar 3: 1, bar 4: 0 → sd(2) on bar 3 = 0 (spike 0), on bar 4 = .5·√2
        let o = go("augen_price_spike", &closes(&[1., E, E * E, E * E, E * E * E]), &[("length", 2.)]);
        assert_series(&o[0], &[N, N, N, 0., (E - 1.) / (0.5 * 2f64.sqrt())]);
    }
    #[test] fn divergent_bars_and_strat_types() {
        let b = from_ohlc(&[(10., 11., 9., 10.), (10., 10.5, 8., 10.), (10., 12., 9., 9.5), (10., 11., 9., 10.), (10., 13., 8., 10.5)]);
        let o = go("divergent_bars", &b, &[]);
        assert_series(&o[0], &[N, 1., 0., 0., 0.]);
        assert_series(&o[1], &[N, 0., 1., 0., 0.]);
        assert_series(&go("the_strat", &b, &[])[0], &[N, -2., 2., 1., 3.]);
    }
    #[test] fn volatility_cone_window_extremes() {
        // returns 1, 2, 0, 1 → sd(2) .5, 1, .5 → hv 50, 100, 50 (days 1)
        let o = go("volatility_cone", &closes(&[1., E, E.powi(3), E.powi(3), E.powi(4)]), &[("length", 2.), ("lookback", 2.), ("days", 1.)]);
        assert_series(&o[0], &[N, N, 50., 100., 50.]);
        assert_series(&o[1], &[N, N, N, 100., 100.]);
        assert_series(&o[2], &[N, N, N, 75., 75.]);
        assert_series(&o[3], &[N, N, N, 50., 50.]);
    }
    #[test] fn probability_cone_projects_and_aligns() {
        let o = go("probability_cone", &closes(&[1., E, E.powi(3), E.powi(3)]), &[("length", 2.), ("horizon", 1.)]);
        assert_series(&o[2], &[N, N, E.powf(3.5), E.powi(4)]);
        assert_series(&o[3], &[N, N, E.powf(2.5), E.powi(2)]);
        assert_series(&o[0], &[N, N, N, E.powf(3.5)]);
        assert_series(&o[1], &[N, N, N, E.powf(2.5)]);
    }
    #[test] fn pesavento_ratio_on_confirmed_swings() {
        let b = closes(&[100., 110., 120., 100., 95., 110., 104., 98.]);
        // swings confirmed: low 100 on bar 2 (100·1.1 rounds above 110), high 120 on bar 3, low 95 on bar 5 (25/20), high 110 on bar 7 (15/25)
        let o = go("pesavento_patterns", &b, &[("dev", 10.)]);
        assert_series(&o[0], &[N, N, N, N, N, 1.25, 1.25, 0.6]);
        assert_series(&o[1], &[N, N, N, N, N, 1.272, 1.272, 0.618]);
        assert_series(&go("pesavento_patterns", &b, &[("dev", 10.), ("tolerance", 0.01)])[1], &[N; 8]);
        let w = run(&get("pesavento_patterns").unwrap(), &walk(1000, 7, true), &Params::new()).unwrap();
        assert!(w[0].iter().any(|v| v.is_finite()));
    }
    #[test] fn listed_entries_carry_honest_notes() {
        for d in DEFS.iter().filter(|d| d.compute.is_none()) {
            assert!(d.note.starts_with("not available yet — formula not published (") || d.note.starts_with("chart drawing/display"), "{}", d.id);
            assert!(!d.needs.any_data(), "{}", d.id);
        }
        for d in DEFS { assert_eq!(SOURCES.iter().filter(|s| s.0 == d.id).count(), 1, "{}", d.id); }
        assert_eq!(SOURCES.len(), DEFS.len());
    }
    #[test] fn rules_reach_gap_c() {
        for (id, out) in [("volatility_ratio", "vr"), ("kirshenbaum_bands", "upper"), ("uni_channel", "lower"), ("mobo_bands", "state"), ("augen_price_spike", "spike"),
            ("divergent_bars", "bull"), ("the_strat", "type"), ("volatility_cone", "median"), ("probability_cone", "upper"), ("pesavento_patterns", "ratio")] {
            compiles(id, out).unwrap();
        }
    }
}
