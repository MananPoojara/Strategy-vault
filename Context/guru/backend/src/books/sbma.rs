//! Steve & Holly Burns, "Moving Averages 101: Incredible Signals That Will Make You Money in the Stock Market"
//! (Stolly Media, 2015). End-of-day moving-average rules on the daily chart. Only rules with both an entry and an
//! exit stated in the book are Live; the many "can be used as" ideas without an exit, or exits keyed to an RSI whose
//! period the book never gives, are listed as not testable. Sources and checks: library/sbma/.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

const BOOK: &str = "sbma";
const EOD: &str = "E|Signals are taken on the close and filled at that close: \"making trading decisions primarily at the end of the day\" (p.63), \"End of day momentum buy signals as price breaks and closes above key moving averages\", \"An end of day stop, if price closes on the wrong side of your key moving average\" (p.16). p.fill=1 = the book's alternative \"Next day ... from the previous close\" / \"close a position on the next open\" (p.16)";
const EMA_DEF: &str = "F|EMA = exponential moving average of the close, weight 2/(n+1), seeded with the simple average of the first n closes (StockCharts.com convention; the book's charts are StockCharts, p.4)";
const CROSS: &str = "E|\"Moving average crossover systems are a way to capture trends by entering a trade when a shorter term moving average crosses over a longer term one, then exiting as it crosses back under\" (p.58); listed pair from \"Here are some popular moving average crossovers that are used\" (p.60)";
const CROSS_X: &str = "X|Exit when the shorter average crosses back under the longer one (p.58, p.67). p.always_in=1 = the book's \"always in approach\" that reverses short on the cross under (p.59)";
const REVIEWED_STOCK_NOTE: &str = "Source-supported default selected-stock research application: signal/fill and closed-trade ledgers independently checked on actual AAPL and RELIANCE; final platform accounting checked separately. Flat test start, real full-history SMA warmup, one sequential long position, no basket allocation or original Monster-stock population/performance reproduction. Quote adjustments and membership remain qualified. See library/sbma/INDEPENDENT-REVIEW-2026-10-15.md.";
const REVIEWED_STOCK_FILL: &str = "E|Completed daily close signals. fill=0 is an idealized same-close research convention; fill=1 executes at the next actual observed open, as a source-described alternative (physical PDF16/63). A terminal next-open order remains unfilled; open final positions retain the platform's disclosed estimated liquidation costs, separately from actual filled exit orders.";
const NOTE: &str = "Book prints no test results. Written for stocks, stock indexes and ETFs (charts of SPY, QQQ, INDU, TSLA, AAPL); runs on any ticker (India = instrument swap, NIFTY 50 / NSE basket stocks).";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, dir, status, rules, notes| StrategyDef {
        id, name, group, session: 0, dir, status, needs_volume: false, needs_vix: false, needs_series: &[],
        book: BOOK, rules, notes, claim: None,
    };
    let x = |id, name, rules| d(id, name, "Ch. 9 · Moving average crossover systems", "Long", Status::Live, rules, NOTE);
    let nt = |id, name, group, dir, why, rules, notes| d(id, name, group, dir, Status::NotTestable(why), rules, notes);
    vec![
        d("sbma_sma50_long", "50-day SMA green light / red light", "Ch. 6 · 50 day SMA", "Long", Status::Live,
            &["E|\"In the strongest of Monster stocks, the 50 day can act as a standalone Green light/Red Light indicator. Staying long as price is above the 50 day line\" (p.42): buy on a close above the 50-day SMA while flat",
              "X|\"and exiting if price drops below\" (p.42): sell on a close below the 50-day SMA", REVIEWED_STOCK_FILL],
            REVIEWED_STOCK_NOTE),
        d("sbma_sma200_short", "Short the first loss of the 200-day SMA", "Ch. 8 · 200 day SMA", "Short", Status::Live,
            &["E|\"Taking short positions as the 200 day is first lost gives traders exposure to profit from market downtrends\" (p.56): sell short on the first close below the 200-day SMA after a close at or above it",
              "X|\"A stop loss can be set at a close back over the 200 day SMA.\" (p.56)", EOD,
              "F|Not coded (judgement): p.54 optional 5- or 10-day EMA trailing stop 'depending on the historical volatility of the stock'"],
            "The long side of the same rule (long above the 200-day, cash below, p.50-51) duplicates apta_ma200_dji and is not repeated. Book prints no results."),
        d("sbma_x_5s20s", "5-day SMA / 20-day SMA crossover", "Ch. 9 · Moving average crossover systems", "Long", Status::Live,
            &[CROSS, "F|Fast = 5-day SMA, slow = 20-day SMA (physical PDF60); current fast strictly above slow after previous fast at or below slow enters long; equality alone holds", "X|Exit long when current fast is strictly below slow after previous fast at or above slow. This reviewed subset stays in cash; short reversal is not admitted.", REVIEWED_STOCK_FILL], REVIEWED_STOCK_NOTE),
        x("sbma_x_5e50s", "5-day EMA / 50-day SMA crossover", &[CROSS, "F|Fast = 5-day EMA, slow = 50-day SMA (p.60)", EMA_DEF, CROSS_X, EOD]),
        x("sbma_x_10e20e", "10-day EMA / 20-day EMA crossover", &[CROSS, "F|Fast = 10-day EMA, slow = 20-day EMA (p.60)", EMA_DEF, CROSS_X, EOD]),
        x("sbma_x_10e50e", "10-day EMA / 50-day EMA crossover", &[CROSS, "F|Fast = 10-day EMA, slow = 50-day EMA (p.60, also the $INDU example p.70)", EMA_DEF, CROSS_X, EOD]),
        x("sbma_x_15e30e", "15-day EMA / 30-day EMA crossover", &[CROSS, "F|Fast = 15-day EMA, slow = 30-day EMA (p.60)", EMA_DEF, CROSS_X, EOD]),
        x("sbma_x_15e150e", "15-day EMA / 150-day EMA crossover", &[CROSS, "F|Fast = 15-day EMA, slow = 150-day EMA (p.60)", EMA_DEF, CROSS_X, EOD]),
        x("sbma_x_10s200s", "10-day SMA / 200-day SMA crossover", &[CROSS, "F|Fast = 10-day SMA, slow = 200-day SMA (p.60)", CROSS_X, EOD]),
        x("sbma_x_50s200s", "50-day / 200-day SMA crossover (Golden Cross)", &[CROSS, "F|Fast = 50-day SMA, slow = 200-day SMA (p.60); \"the 'Golden Cross' which is when the 50 day SMA crosses up through the 200 day SMA\" (p.64)", CROSS_X, EOD]),
        d("sbma_x_10e30e_sar", "10-day / 30-day EMA crossover in both directions", "Ch. 9 · Moving average crossover systems", "Long/Short", Status::Live,
            &["E|\"The 10 day EMA / 30 day EMA crossover signal in both directions. You go long when the 10 day EMA crosses over the 30 day\" (p.60)",
              "X|\"and exit the long and go short when the 10 day goes back under the 30 day\" (p.60); the next cross over exits the short and goes long (always in after the first cross)", EMA_DEF, EOD],
            "QQQ example p.62-63 (chart only, no numbers). Book prints no results."),
        nt("sbma_ema5", "5-day EMA momentum / trailing stop", "Ch. 3 · 5 day EMA", "Long/Short",
            "the book says the 5-day EMA 'can't be used as a standalone indicator' (p.26); its uses need a separate entry signal or judgement ('strong bullish candle', 'after being below it for weeks')",
            &["E|Break above the 5-day EMA as short-term momentum (p.20)", "X|End-of-day trailing stop after another entry (p.20-21)"], "Not testable."),
        nt("sbma_ema10_trend", "10-day EMA with the 200-day SMA filter", "Ch. 4 · 10 day EMA", "Long/Short",
            "entry and exit spread over hedged bullets ('can be used', 'can keep you'); profit exit is 'as the RSI becomes overbought near the 70 RSI' with no RSI period anywhere in the book",
            &["F|\"The 10 day EMA can be used as a long signal only if the long term trend is up as measured by the 200 day SMA\" (p.28)", "X|\"exiting long positions as the RSI becomes overbought near the 70 RSI\" (p.29)"], "Not testable."),
        nt("sbma_ema10_sma50", "Long over the 10-day EMA while over the 50-day SMA", "Ch. 6 · 50 day SMA", "Long",
            "no exit stated (p.41 gives only the entry and the filter)",
            &["E|\"go long on a stock when the price broke over the 10 day EMA, and was also trading over the 50 day SMA\" (p.41)"], "A rule with no exit is not testable; no exit is added."),
        nt("sbma_ema21", "21-day EMA trend / end-of-day stop", "Ch. 5 · 21 day EMA", "Long/Short",
            "entries are other signals ('a break out over the 50 day', 'a loss of the 21 day EMA') with exits at the RSI 30/70 'or their price target'; RSI period never given",
            &["E|Buying the break over the 50 day with an end of day stop of a close under the 21 day EMA (p.33)", "X|Cover shorts near the 30 RSI, sell longs near the 70 RSI (p.36)"], "Not testable."),
        nt("sbma_sma100_rsi", "100-day SMA dip buy with RSI 30-35", "Ch. 7 · 100 day SMA", "Long",
            "entry needs 'the 100 day SMA, Oversold RSI 30-35, and the MACD bullish crossover all lined up' (judgement of 'near', RSI period not given) and no exit for this setup",
            &["E|100-day SMA coinciding with an oversold RSI of 30-35 (p.45-48)"],
            "The mechanical part of ch. 7 ('stop out ... with a close under the 100 day SMA and re-enter ... on a close back above', p.45) duplicates apta_ma100_dji and is not repeated."),
        nt("sbma_ladder", "200-day entry with a stop ladder 50 -> 10 -> 5", "Ch. 3 · 5 day EMA", "Long",
            "the moves of the stop from the 50-day SMA to the 10-day EMA and then the 5-day EMA have no stated trigger (p.21)",
            &["E|Enter after a break above the 200 day SMA, end-of-day stop on a close below the 200 day (p.20-21)", "X|If price breaks above the 50 day SMA move the stop to a close below the 50 day, then to the 10 day EMA, finally to a close below the 5 day EMA (p.21)"], "Not testable."),
        nt("sbma_x_10e50e_rsi", "10/50-day EMA crossover with RSI exits ($PCLN)", "Ch. 9 · Moving average crossover systems", "Long/Short",
            "exits 'as price became oversold at 30 RSI' / 'as the RSI went over 70' use an RSI whose period the book never states; the book calls it discretionary (p.65)",
            &["E|Short as the 10 day EMA crossed under the 50 day EMA, long as it crossed back over (p.65)", "X|Cover at 30 RSI, sell above 70 RSI (p.65)"], "The crossover-only version is sbma_x_10e50e."),
        nt("sbma_p73_200_rsi65", "Break of the 200-day SMA with RSI 65 target", "Ch. 10 · Combining moving averages with other indicators", "Long",
            "target is 'the 65 RSI' with no RSI period; the trailing stop applies only 'If it trends' (judgement)",
            &["E|\"Enter on a break and close above the 200 day SMA, stop loss is a close back under the 200 day. If it trends the trailing stop could be a close back under the 10 day EMA with a target of the 65 RSI\" (p.73)"], "Not testable."),
        nt("sbma_p73_50_bounce", "Bounce off the 50-day SMA", "Ch. 10 · Combining moving averages with other indicators", "Long",
            "'bounces off the 50 day SMA' is chart judgement; target 70 RSI has no period",
            &["E|\"Enter a stock as it bounces off the 50 day SMA. Stop loss is a close back under the 50 day SMA. Trailing stop in a trend is a close under the 10 day EMA. Target is the 70 RSI.\" (p.73)"], "Not testable."),
        nt("sbma_p73_rsi34", "Index at RSI 34 above the 200-day SMA", "Ch. 10 · Combining moving averages with other indicators", "Long",
            "every threshold is an RSI level (34 / 30 / 50) and the book never states the RSI period",
            &["E|\"Enter when a stock index is at a 34 RSI but still over the 200 day SMA. Your initial stop loss is a close below the 30 RSI. Your trailing stop is a close below the previous day's low of day. Your target is the 50 RSI.\" (p.73)"], "Not testable."),
        nt("sbma_p73_rsi51_short", "Index rally to RSI 51 below the 200-day SMA (short)", "Ch. 10 · Combining moving averages with other indicators", "Short",
            "every threshold is an RSI level (51 / 54 / 32) and the book never states the RSI period",
            &["E|\"You take a short position in a stock index as it rallies back to the 51 RSI, but the market is under the 200 day SMA. Your stop loss is a close over the 54 RSI. Your trailing stop is the previous day's high of day. Your target is to cover your short at the 32 RSI.\" (p.73)"], "Not testable."),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "sbma_sma50_long" => ("sbma_price_ma", "Burns price vs moving average", "50-day SMA long"),
        "sbma_sma200_short" => ("sbma_price_ma", "Burns price vs moving average", "200-day SMA short"),
        "sbma_x_5s20s" => ("sbma_cross", "Burns MA crossovers", "5 / 20 SMA"),
        "sbma_x_5e50s" => ("sbma_cross", "Burns MA crossovers", "5 EMA / 50 SMA"),
        "sbma_x_10e20e" => ("sbma_cross", "Burns MA crossovers", "10 / 20 EMA"),
        "sbma_x_10e50e" => ("sbma_cross", "Burns MA crossovers", "10 / 50 EMA"),
        "sbma_x_15e30e" => ("sbma_cross", "Burns MA crossovers", "15 / 30 EMA"),
        "sbma_x_15e150e" => ("sbma_cross", "Burns MA crossovers", "15 / 150 EMA"),
        "sbma_x_10s200s" => ("sbma_cross", "Burns MA crossovers", "10 / 200 SMA"),
        "sbma_x_50s200s" => ("sbma_cross", "Burns MA crossovers", "50 / 200 SMA (Golden Cross)"),
        "sbma_x_10e30e_sar" => ("sbma_cross", "Burns MA crossovers", "10 / 30 EMA both directions"),
        "sbma_x_10e50e_rsi" => ("sbma_cross", "Burns MA crossovers", "10 / 50 EMA + RSI exits (not testable)"),
        "sbma_ema5" | "sbma_ema10_trend" | "sbma_ema10_sma50" | "sbma_ema21" | "sbma_sma100_rsi" | "sbma_ladder" =>
            ("sbma_single", "Burns single moving averages (not testable)", ""),
        "sbma_p73_200_rsi65" | "sbma_p73_50_bounce" | "sbma_p73_rsi34" | "sbma_p73_rsi51_short" =>
            ("sbma_p73", "Burns entry/exit strategies p.73 (not testable)", ""),
        _ => return None,
    })
}

/// (fast length, fast is EMA, slow length, slow is EMA, always in by default)
fn pair(id: &str) -> Option<(f64, bool, f64, bool, f64)> {
    Some(match id {
        "sbma_x_5s20s" => (5., false, 20., false, 0.),
        "sbma_x_5e50s" => (5., true, 50., false, 0.),
        "sbma_x_10e20e" => (10., true, 20., true, 0.),
        "sbma_x_10e50e" => (10., true, 50., true, 0.),
        "sbma_x_15e30e" => (15., true, 30., true, 0.),
        "sbma_x_15e150e" => (15., true, 150., true, 0.),
        "sbma_x_10s200s" => (10., false, 200., false, 0.),
        "sbma_x_50s200s" => (50., false, 200., false, 0.),
        "sbma_x_10e30e_sar" => (10., true, 30., true, 1.),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let fill_label = if matches!(id,"sbma_sma50_long"|"sbma_x_5s20s") { "Research fill: 0 = idealized signal close, 1 = next actual open" } else { "Fill: 0 = signal close (book default), 1 = next open" };
    let fill = p("fill", fill_label, 0., 0., 1., 1.);
    if let Some((f, _, s, _, ai)) = pair(id) {
        let mut v = vec![p("fast", "Fast average length", f, 2., 100., 1.), p("slow", "Slow average length", s, 10., 300., 5.)];
        if !id.ends_with("_sar") { v.push(p("always_in", "1 = always in, reverse short on the cross under (p.59)", ai, 0., 1., 1.)); }
        v.push(fill);
        return Some(v);
    }
    match id {
        "sbma_sma50_long" => Some(vec![p("ma", "SMA length", 50., 10., 250., 5.), fill]),
        "sbma_sma200_short" => Some(vec![p("ma", "SMA length", 200., 50., 300., 10.), fill]),
        _ => None,
    }
}

/// Simple average from a fresh window sum on every bar (no running-sum drift, so an exact tie between the close
/// and its average stays a tie: the book's "above" / "below" are strict).
fn sma(v: &[f64], n: usize) -> Vec<f64> {
    (0..v.len()).map(|i| if n > 0 && i + 1 >= n { v[i + 1 - n..=i].iter().sum::<f64>() / n as f64 } else { f64::NAN }).collect()
}

/// EMA with weight 2/(n+1) seeded by the SMA of the first n values (StockCharts).
fn ema(v: &[f64], n: usize) -> Vec<f64> {
    let mut out = vec![f64::NAN; v.len()];
    if n == 0 || v.len() < n { return out; }
    let k = 2. / (n as f64 + 1.);
    let mut e = v[..n].iter().sum::<f64>() / n as f64;
    out[n - 1] = e;
    for i in n..v.len() { e += k * (v[i] - e); out[i] = e; }
    out
}

/// Position state machine. `want(i)` returns the target position after bar i's close (+1, 0, -1), or None for
/// "keep the current position". Fills at the close of i (fill = 0) or at the open of i+1 (fill = 1).
fn positions(x: &Ctx, from: usize, to: usize, next_open: bool, want: impl Fn(usize, f64) -> Option<f64>) -> Vec<Trade> {
    let mut out = vec![];
    let mut cur: Option<Trade> = None;
    let mk = |dir: f64, i: usize, px: f64| Trade { dir, legs: vec![Leg { i, px, w: 1. }], exit_i: i, exit_px: px, reason: String::new(),
        entry_kind: if next_open { "open" } else { "close" }, signal_i: i, open: false, exits: vec![] };
    for i in from..=to {
        let pos = cur.as_ref().map_or(0., |t| t.dir);
        let Some(target) = want(i, pos) else { continue };
        if target == pos { continue; }
        let (fi, px) = if next_open { if i + 1 > to { break; } (i + 1, x.o[i + 1]) } else { (i, x.c[i]) };
        if let Some(mut t) = cur.take() {
            t.exit_i = fi; t.exit_px = px; t.signal_i = i; t.reason = "moving-average exit".into();
            out.push(t);
        }
        if target != 0. { cur = Some(mk(target, fi, px)); }
    }
    if let Some(mut t) = cur {
        t.exit_i = to; t.exit_px = x.c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true;
        out.push(t);
    }
    out
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if from > to || to >= x.r.len() { return None; }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(0., |d| d.def));
    let next_open = g("fill") >= 0.5;
    let len = |k: &str| g(k).round().max(1.) as usize;
    let c = &x.c;
    Some(if let Some((_, fe, _, se, _)) = pair(id) {
        let avg = |n: usize, e: bool| if e { ema(c, n) } else { sma(c, n) };
        let (f, s) = (avg(len("fast"), fe), avg(len("slow"), se));
        let both = id.ends_with("_sar") || g("always_in") >= 0.5;
        let ok = |i: usize| i > 0 && [f[i], s[i], f[i - 1], s[i - 1]].iter().all(|v| v.is_finite());
        positions(x, from, to, next_open, |i, _| {
            if !ok(i) { return None; }
            if f[i] > s[i] && f[i - 1] <= s[i - 1] { Some(1.) }
            else if f[i] < s[i] && f[i - 1] >= s[i - 1] { Some(if both { -1. } else { 0. }) }
            else { None }
        })
    } else if id == "sbma_sma50_long" {
        let m = sma(c, len("ma"));
        positions(x, from, to, next_open, |i, pos| {
            if !m[i].is_finite() { return None; }
            if pos == 0. && c[i] > m[i] { Some(1.) } else if pos > 0. && c[i] < m[i] { Some(0.) } else { None }
        })
    } else {
        let m = sma(c, len("ma"));
        positions(x, from, to, next_open, |i, pos| {
            if i == 0 || !m[i].is_finite() || !m[i - 1].is_finite() { return None; }
            if pos == 0. && c[i] < m[i] && c[i - 1] >= m[i - 1] { Some(-1.) } else if pos < 0. && c[i] > m[i] { Some(0.) } else { None }
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn rows(cl: &[f64]) -> Vec<Row> {
        cl.iter().enumerate().map(|(i, &c)| Row { ticker: "T".into(), date: format!("{:05}", i), open: c, high: c + 1., low: c - 1., close: c, volume: None }).collect()
    }
    #[test]
    fn reviewed_crossover_is_registered_as_five_twenty_sma_long_cash() {
        let d=catalogue().into_iter().find(|d|d.id=="sbma_x_5s20s").expect("source-listed SMA crossover");
        assert_eq!(d.dir,"Long");
        assert_eq!(pair(d.id),Some((5.,false,20.,false,0.)));
        assert_eq!(family_of(d.id).unwrap().0,"sbma_cross");
    }
    #[test]
    fn reviewed_crossover_tie_then_strict_cross_is_long_cash() {
        let mut cl=vec![100.;20];cl.extend([101.,90.,90.]);
        let r=rows(&cl);let x=Ctx::new(&r,&r,&[]);
        let ts=run("sbma_x_5s20s",&x,19,22,&Params::new()).unwrap();
        assert_eq!(ts.len(),1);let t=&ts[0];
        assert_eq!((t.dir,t.legs[0].i,t.exit_i,t.open),(1.,20,21,false));
        assert_eq!((t.legs[0].px,t.exit_px),(101.,90.));
        // Later values cannot change an earlier signal or its close fill.
        let mut future=r.clone();future[22].close=500.;future[22].high=501.;
        let xx=Ctx::new(&future,&future,&[]);
        let prefix=run("sbma_x_5s20s",&xx,19,21,&Params::new()).unwrap();
        assert_eq!((prefix[0].legs[0].i,prefix[0].exit_i),(20,21));
    }
    #[test]
    fn reviewed_next_open_uses_actual_prices_and_keeps_terminal_order_unfilled() {
        let mut cl=vec![100.;20];cl.extend([101.,90.,90.]);
        let mut r=rows(&cl);r[21].open=92.;r[21].high=93.;r[22].open=89.;
        let x=Ctx::new(&r,&r,&[]);let p=Params::from([("fill".into(),1.)]);
        assert!(run("sbma_x_5s20s",&x,19,20,&p).unwrap().is_empty());
        let pending=run("sbma_x_5s20s",&x,19,21,&p).unwrap();
        assert_eq!(pending.len(),1);let t=&pending[0];
        assert_eq!((t.legs[0].i,t.legs[0].px,t.exit_i,t.exit_px,t.open),(21,92.,21,90.,true));
        assert_eq!(t.entry_kind,"open");
        let closed=run("sbma_x_5s20s",&x,19,22,&p).unwrap();let t=&closed[0];
        assert_eq!((t.legs[0].px,t.exit_i,t.exit_px,t.signal_i,t.open),(92.,22,89.,21,false));
    }
    #[test]
    fn reviewed_fifty_sma_cold_start_above_does_not_require_a_fresh_cross() {
        let mut cl=vec![100.;50];cl.extend([110.,110.,90.]);
        let r=rows(&cl);let x=Ctx::new(&r,&r,&[]);
        let ts=run("sbma_sma50_long",&x,51,52,&Params::new()).unwrap();
        assert_eq!(ts.len(),1);assert_eq!((ts[0].legs[0].i,ts[0].exit_i),(51,52));
    }
    #[test]
    fn ema_is_sma_seeded() {
        let e = ema(&[1., 2., 3., 4.], 3);
        assert!(e[1].is_nan() && e[2] == 2. && (e[3] - 3.).abs() < 1e-12);
    }
    #[test]
    fn sma50_enters_above_and_exits_below_on_the_close() {
        let mut cl = vec![100.; 60];
        cl[55] = 110.; cl[56] = 111.; cl[57] = 95.;
        let r = rows(&cl);
        let x = Ctx::new(&r, &r, &[]);
        let ts = run("sbma_sma50_long", &x, 50, 59, &Params::new()).unwrap();
        assert_eq!(ts.len(), 1);
        assert_eq!((ts[0].legs[0].i, ts[0].legs[0].px, ts[0].exit_i, ts[0].exit_px), (55, 110., 57, 95.));
    }
    #[test]
    fn sar_reverses_on_each_cross() {
        let cl: Vec<f64> = (0..200).map(|i| 100. + 20. * ((i as f64) / 15.).sin()).collect();
        let r = rows(&cl);
        let x = Ctx::new(&r, &r, &[]);
        let ts = run("sbma_x_10e30e_sar", &x, 0, 199, &Params::new()).unwrap();
        assert!(ts.len() > 3);
        for w in ts.windows(2) { assert_eq!(w[0].exit_i, w[1].legs[0].i); assert_eq!(w[0].dir, -w[1].dir); }
    }
}
