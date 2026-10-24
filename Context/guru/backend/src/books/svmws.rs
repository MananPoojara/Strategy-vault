//! Victor Sperandeo, "Trader Vic - Methods of a Wall Street Master" (Wiley, 1991).
//! Only the 10-week/30-week moving-average rule (Ch. 8, pp. 90-93) states both a buy and a sell signal exactly.
//! 1-2-3 / 2B (Ch. 7), Gordon's 200-day rule ("flattens out"), the oscillators and stock selection are judgement.
//! Source and checks: library/svmws/.
use crate::books::swsp::week_ends;
use crate::books::svtvc::sma_exact;
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "svmws";
/// Admitted ids (authors/sperandeo/admission/SOURCE-CONTRACT.md).
pub const REVIEWED: &[&str] = &["svmws_10w30w_long", "svmws_10w30w_short"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const LONG: &[&str] = &["E|\"When the 10-week moving average crosses the 30-week moving average and the slope of both averages is up, this comprises a buy signal, provided that prices are above both moving average lines\" (p. 90)", "X|\"When the 10-week moving average crosses the 30-week moving average and the slope of both averages is down, this comprises a sell signal, provided prices are below both moving average lines\" (p. 93)"];
const SHORT: &[&str] = &["E|\"When the 10-week moving average crosses the 30-week moving average and the slope of both averages is down, this comprises a sell signal, provided prices are below both moving average lines\" (p. 93)", "X|\"When the 10-week moving average crosses the 30-week moving average and the slope of both averages is up, this comprises a buy signal, provided that prices are above both moving average lines\" (p. 90)"];
const NOTE: &str = "Weekly close = close of the last trading day of each Monday-Sunday week; simple MAs of weekly closes (the book's MA definition, p. 88). \
Cross = this week's 10-wk MA above (below) the 30-wk MA and last week's not; slope = this week's MA vs last week's. All three conditions must hold in the week of the cross \
[literal reading; a cross without them gives no signal]. Fill at that weekly close [book silent]. The book names a buy and a sell signal and no other exit, \
so the long is closed only by the sell signal (short: only by the buy signal). Meant for stocks, stock indexes and commodities (p. 90); India: same rule on NIFTY 50.";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, dir, status, rules, notes| StrategyDef {
        id, name, group: "Ch. 8 What the Analysts Don't Know Can Kill You", session: 0, dir, status, needs_volume: false, needs_vix: false,
        needs_series: &[], book: BOOK, rules, notes, claim: None,
    };
    use Status::*;
    vec![
        d("svmws_10w30w_long", "10-week / 30-week MA cross · buys", "Long", Live, LONG, NOTE),
        d("svmws_10w30w_short", "10-week / 30-week MA cross · sells", "Short", Live, SHORT, NOTE),
        d("svmws_200d_gordon", "Gordon 200-day moving average rule", "Long/Short",
          NotTestable("'flattens out' is not quantified: \"If the 200-day moving average line flattens out following a previous decline or is advancing, and prices penetrate the moving average line on the upside\" (p. 90)"),
          &["E|\"If the 200-day moving average line flattens out following a previous decline or is advancing, and prices penetrate the moving average line on the upside, this comprises a major buying signal\" (p. 90)",
            "X|\"If the 200-day moving average line flattens out following a previous rise or is declining, and prices penetrate the moving average line on the downside, this comprises a major selling signal\" (p. 90)"],
          "Book cites W. Gordon: Dow stocks 1917-1967, 18.5% average simple yearly return (p. 90). Dropping 'flattens out' would change the rule, so not coded."),
        d("svmws_123", "1-2-3 change of trend", "Long/Short",
          NotTestable("trendline drawn by eye from the lowest low to the 'highest minor low'; 'test' and 'minor' highs/lows are chart judgement; no exit rule"),
          &["F|\"(1) a break in the trend line; (2) a test of the preceding high or low; (3) the breaking of a preceding minor rally high or minor sell-off low\" (p. 76)"],
          "Not coded."),
        d("svmws_2b", "2B rule", "Long/Short",
          NotTestable("the 'previous high' is a short/intermediate/long-term swing chosen on the chart; only a stop is given ('close your position when prices break past the new 2B high', p. 80) and winners are to 'run' with no exit"),
          &["E|\"In an uptrend, if a higher high is made but fails to carry through, and then prices drop below the previous high, then the trend is apt to reverse\" (p. 78)",
            "X|\"you should immediately close your position when prices break past the new 2B high\" (p. 80)"],
          "Not coded."),
        d("svmws_oscillators", "Breadth and price oscillators", "Long/Short",
          NotTestable("indicators only: 'approach or break through significant previous highs or lows' (pp. 100-103) with no entry or exit; breadth one also needs NYSE A/D"),
          &["F|Price oscillator: 10-day running sum of (close - close 5 days earlier) of the NYSE Composite (p. 103)",
            "F|Breadth oscillator: 10-day sum of NYSE advances minus declines; long-term = 30-day sum / 3 (pp. 101-103)"],
          "Supporting tools; no trade rule."),
        d("svmws_stock_selection", "Stock selection (earnings growth, PE, relative strength)", "Long",
          NotTestable("fundamental and chart judgement (earnings-growth slope vs price-trend slope, PE, leverage) on a stock universe"),
          &["F|\"if the rate of change of earnings growth ... is less than the rate of change of prices ... then the stock is a buy\" (p. 106)"], "Not coded."),
        d("svmws_dow_theory", "Dow Theory market calls", "Long/Short",
          NotTestable("primary/secondary trend and confirmation are judged on the chart (Ch. 4-5)"),
          &["F|Bull/bear confirmation by Industrials and Transports (Ch. 4)"], "Not coded."),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "svmws_10w30w_long" => ("svmws_10w30w", "Sperandeo 10/30-week MA rule", "Buys"),
        "svmws_10w30w_short" => ("svmws_10w30w", "Sperandeo 10/30-week MA rule", "Sells"),
        "svmws_200d_gordon" | "svmws_oscillators" => ("svmws_ma", "Sperandeo MA & oscillators", "not testable"),
        "svmws_123" | "svmws_2b" | "svmws_dow_theory" => ("svmws_trend_change", "Sperandeo change of trend", "not testable"),
        "svmws_stock_selection" => ("svmws_stocks", "Sperandeo stock selection", "not testable"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    match id {
        "svmws_10w30w_long" | "svmws_10w30w_short" => Some(vec![
            ParamDef { key: "fast", label: "Fast MA (weeks)", def: 10., min: 4., max: 20., step: 1. },
            ParamDef { key: "slow", label: "Slow MA (weeks)", def: 30., min: 11., max: 50., step: 1. },
        ]),
        _ => None,
    }
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let dir = match id { "svmws_10w30w_long" => 1., "svmws_10w30w_short" => -1., _ => return None };
    if from >= to || to >= x.r.len() { return None; }
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d).round().max(2.) as usize;
    let (nf, ns) = (g("fast", 10.), g("slow", 30.));
    let we: Vec<usize> = week_ends(x).into_iter().filter(|&i| i <= to).collect();
    let wc: Vec<f64> = we.iter().map(|&i| x.c[i]).collect();
    let (f, s) = (sma_exact(&wc, nf), sma_exact(&wc, ns));
    let (mut out, mut cur) = (vec![], None::<Trade>);
    for k in 1..we.len() {
        let i = we[k];
        if i < from || !(f[k].is_finite() && s[k].is_finite() && f[k - 1].is_finite() && s[k - 1].is_finite()) { continue; }
        // slope: this week's MA vs last week's = the week entering vs the week leaving the window (exact; flat = neither)
        let (fu, fd, su, sd) = (wc[k] > wc[k - nf], wc[k] < wc[k - nf], wc[k] > wc[k - ns], wc[k] < wc[k - ns]);
        let buy = f[k - 1] <= s[k - 1] && f[k] > s[k] && fu && su && wc[k] > f[k] && wc[k] > s[k];
        let sell = f[k - 1] >= s[k - 1] && f[k] < s[k] && fd && sd && wc[k] < f[k] && wc[k] < s[k];
        let (enter, exit) = if dir > 0. { (buy, sell) } else { (sell, buy) };
        if let Some(t) = cur.as_mut() {
            if exit {
                t.exit_i = i; t.exit_px = x.c[i]; t.signal_i = i;
                t.reason = if dir > 0. { "10/30-week sell signal".into() } else { "10/30-week buy signal".into() };
                out.push(cur.take().unwrap());
            }
        } else if enter {
            cur = Some(Trade { dir, legs: vec![Leg { i, px: x.c[i], w: 1. }], exit_i: i, exit_px: x.c[i], reason: String::new(),
                entry_kind: "close", signal_i: i, open: false, exits: vec![] });
        }
    }
    if let Some(mut t) = cur { t.exit_i = to; t.exit_px = x.c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true; out.push(t); }
    Some(out)
}
