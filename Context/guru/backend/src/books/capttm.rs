//! Curtis M. Arnold, *Timing the Market: How to Profit in Bull and Bear Markets with Technical Analysis*, revised ed.
//! (Probus 1993; ch.1-25 (c) Weiss Research). Source copy: Books/Curtis Arnold/Curtis Arnold - Timing the Market.pdf
//! (140 PDF pages, two printed pages per PDF page). Page refs are printed pages. Notes: library/capttm/README.md.
//! A survey of indicators. Live: the two moving-average systems of ch.13 (p.82-83), the only rules the book gives
//! with both an entry and an exit and fixed numbers that run on index prices. Breadth / sentiment / monetary rules with
//! numbers need NYSE data the vault lacks (NeedsData); the rest is interpretation (NotTestable).
use crate::connors::{sma, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

const BOOK: &str = "capttm";

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    let d = |id, name, group, dir, status, rules, notes| StrategyDef {
        id, name, group, session: 0, dir, status, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None,
    };
    let nd = |id, name, group, why, rules, notes| d(id, name, group, "Long/Short", NeedsData(why), rules, notes);
    let nt = |id, name, group, why, rules| d(id, name, group, "Long/Short", NotTestable(why), rules, "");
    vec![
        d("capttm_ma14_50", "Two-crossover model: 14-day / 50-day moving averages", "Ch 13 · Moving averages", "Long", Live, &[
            "E|\"Buy when the 14-day moving average crosses above the 50-day moving average\" (p.82)",
            "X|\"sell when it crosses below the 50-day moving average.\" (p.82)",
        ], concat!("Simple averages of closes (\"two simple moving averages\", p.82). Signals on the close, filled at that close [book silent on the fill]. ",
            "Default reads 'sell' as closing the long: the next paragraph says 'Sell short' when it means a short (p.82). p.short=1 [non-default] also sells short on the cross below and covers on the cross above (stop-and-reverse). ",
            "Book example: gold, from a Merrill Lynch study (p.82); no results printed. India: NIFTY indices (instrument swap).")),
        d("capttm_ma_price_both", "Modified two-average system: price above / below both 14- and 50-day averages", "Ch 13 · Moving averages", "Long/Short", Live, &[
            "E|\"(1) Buy when the actual price crosses above both moving averages and exit the market when the price crosses below either moving average.\" (p.82)",
            "E|\"(2) Sell short when the actual price crosses below both moving averages and exit the market when the price crosses above either moving average.\" (p.82)",
            "X|Exits as quoted above.",
        ], concat!("'This system' = the 14/50-day pair of the preceding paragraph (p.82). Price = the daily close (\"you may wish to use closing prices only\", p.81). ",
            "Crossing above both: close > both averages today and not yesterday; exit long: close below either average (close < the higher one). Short mirror. All fills at the signal close [book silent]. A long exit and a short entry can happen on the same close. India: NIFTY indices.")),
        nt("capttm_price_ma", "Price crossing a single moving average", "Ch 13 · Moving averages",
            "no single length: 'a 200-day moving average ... (Shorter-term signals would be generated using a 10- or 20-day moving average.)' (p.81)", &[
            "E|\"Buy when prices cross above the moving average. Sell when prices cross below the moving average.\" (p.81)"]),
        nt("capttm_dow_ma200", "Dow vs its 200-day moving average", "Ch 13 · Moving averages",
            "no exit: a break below the average 'does not necessarily signal an end to the bull market' (p.83); the confirmation (average turning up) has no trade rule", &[
            "E|\"If ... the Dow should move up and cross the Moving Average, it indicates that a new bull market has begun.\" (p.83)"]),
        nt("capttm_momentum_osc", "Momentum oscillator overbought/oversold examples", "Ch 12 · Momentum",
            "illustrative 'examples of different trading rules' with sell rules only and limits the reader must set from a year of history (p.76)", &[
            "E|\"1. Sell when the five-day oscillator goes above 0.9 or 90 percent. 2. ... stays above 90 percent for two days. 3. ... both the five-day and ten-day oscillators are above 90 percent.\" (p.76)"]),
        nt("capttm_haurlan", "Haurlan Index (duplicate)", "Ch 16 · Breadth",
            "duplicate of the vault's Haurlan breadth rules (apwms); the '3-day weighted moving average' weights are not given here (p.103)", &[
            "E|\"When the index moves above +100, a short-term buy signal is generated, which remains in effect until the index drops below -150\" (p.103)"]),
        nt("capttm_arms5", "Arms Index 5-day average above/below 1.00 (duplicate)", "Ch 16 · Breadth",
            "duplicate of apwms_trin5 / apwms_trin5_nse (5-day TRIN, sell above 1.00, buy below 1.00)", &[
            "E|\"a sell signal is generated when the index rises above 1.00; a buy signal produced when it falls below 1.00\" (5-day moving average of the Arms index, p.107-108)"]),
        nt("capttm_heiby", "Heiby 'Advance-Decline Quartile Divergence Syndrome'", "Ch 16 · Breadth",
            "after the quartile criteria are met 'he looks at other indicators such as short sales, odd-lot sales...' (judgement); no exit (p.114)", &[
            "E|Buy: S&P in its bottom quartile of the last 50 days, advancing issues in the top quartile, advances > declines, unchanged not in the top quartile (p.114)"]),
        nd("capttm_most_active_net", "Indicator Digest: 15 most active stocks, 30-day net total +9 / -9", "Ch 17 · Leadership",
            "NYSE 15 Most Active Issues daily advancers and decliners (NYSE_MA15_NET = advancers - decliners among the 15); not in the vault", &[
            "E|\"Maintain a 30-day cumulative total ... a buy signal is generated when the 30-day total rises to +9; a sell signal is generated when the 30-day total falls to below -9; readings between +9 and -9 are neutral, meaning that the previous signal remains in effect.\" (p.119)"],
            "Not coded until the series exists; 'sell signal' = exit or short is not stated."),
        nt("capttm_most_active_pct", "15 most actives: 10-day average of the percent advancing", "Ch 17 · Leadership",
            "'reaches the 60-70 percent region and starts to turn down' / '30-35 percent ... begins to turn up' are ranges with no turn definition and no exit (p.119)", &[]),
        nt("capttm_short_interest", "Short interest ratio above 1.75", "Ch 18 · Sentiment", "buy signal only, no exit (p.124)", &[
            "E|\"If the short interest ratio is above 1.75, it is considered a bullish signal.\" (p.124)"]),
        nd("capttm_specialist_short", "Specialist short-sale ratio 33/35 and 58/55 percent", "Ch 18 · Sentiment",
            "weekly NYSE specialist short sales / total short sales (Barron's, two-week lag); not in the vault", &[
            "E|\"A buy signal can be generated by a single weekly reading below 33 percent, or a series of four weekly readings averaging below 35 percent.\" (p.125)",
            "X|\"A sell signal is indicated when a single weekly reading exceeds 58 percent or the average of the latest four weekly readings exceeds 55 percent.\" (p.125)"],
            "Not coded until the series exists."),
        nt("capttm_member_short", "Member short-sale ratio below 65 percent", "Ch 18 · Sentiment",
            "the exit is 'a sale is indicated when the ratio approaches 88 percent' (p.126), with no level for 'approaches'; also needs weekly NYSE member short sales", &[
            "E|\"When the 10-week moving average of Member Short Sales was less than 65 percent\" (p.126)"]),
        nd("capttm_margin_debt", "Customer margin debt vs its 12-month moving average (Fosback)", "Ch 18 · Sentiment",
            "monthly NYSE customer margin debt (released 2-3 weeks after month end); not in the vault", &[
            "E|\"A buy signal is given when the current figure moves above the 12-month moving average. A sell signal is rendered when the current figure drops below\" (p.128)"],
            "Not coded until the series exists; release lag must be applied."),
        nd("capttm_free_credit", "Free credit balances vs their 12-month moving average", "Ch 18 · Sentiment",
            "monthly NYSE free credit balances; not in the vault", &[
            "E|\"A buy signal is generated when the current series crosses above the 12-month moving average; a sell signal occurs when the current series crosses below\" (p.128)"],
            "Not coded until the series exists."),
        nd("capttm_djua_15w", "Dow Jones Utility Average vs its 15-week moving average", "Ch 18 · Sentiment",
            "weekly Dow Jones Utility Average closes; not in the vault", &[
            "F|\"When the current readings are above the 15-week moving average, the utility stocks can be classified as being in an uptrend and the stock market also should continue higher.\" (p.129)",
            "X|\"When current readings drop below the 15-week moving average, a sell signal is in order.\" (p.129)"],
            "No explicit buy wording ('should continue higher'); to settle when the data exists."),
        nt("capttm_trin10", "10-day TRIN overbought/oversold", "Ch 20 · Short-term indicators",
            "describes conditions ('an immediate correction can be expected'), no trade or exit rule (p.136)", &[]),
        nt("capttm_monetary", "Net free reserves, money supply, discount rate, fed funds", "Ch 19 · Monetary",
            "descriptive; no entry/exit levels (p.131-133)", &[]),
        nt("capttm_charting", "Trendlines, reversal and consolidation patterns, gaps, retracements, Dow Theory, cycles", "Ch 4-11, 15, 23-25",
            "chart reading by eye (judgement)", &[]),
        nt("capttm_trend_filter", "28-day moving-average trend filter", "Ch 31 · Developing a trading system",
            "a filter only: 'Only take your long signals if prices are above the 28-day moving average' with no entry or exit of its own (p.244)", &[]),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "capttm_ma14_50" => ("capttm_ma", "Timing the Market moving-average systems", "14/50 crossover"),
        "capttm_ma_price_both" => ("capttm_ma", "Timing the Market moving-average systems", "price vs both averages"),
        s if s.starts_with("capttm_") => ("capttm_other", "Timing the Market indicators", ""),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let fast = p("fast", "Fast moving average (days)", 14., 2., 100., 1.);
    let slow = p("slow", "Slow moving average (days)", 50., 5., 300., 1.);
    if !id.starts_with("capttm_") { return None; }
    Some(match id {
        "capttm_ma14_50" => vec![fast, slow, p("short", "Also sell short on the cross below (stop-and-reverse; not the default reading)", 0., 0., 1., 1.)],
        "capttm_ma_price_both" => vec![fast, slow],
        _ => vec![],
    })
}

fn trade(dir: f64, i: usize, px: f64) -> Trade {
    Trade { dir, legs: vec![Leg { i, px, w: 1. }], exit_i: i, exit_px: px, reason: String::new(), entry_kind: "close", signal_i: i, open: false, exits: vec![] }
}
fn close(out: &mut Vec<Trade>, cur: &mut Option<Trade>, i: usize, px: f64, why: &str) {
    if let Some(mut t) = cur.take() { t.exit_i = i; t.exit_px = px; t.signal_i = i; t.reason = why.into(); out.push(t); }
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if id != "capttm_ma14_50" && id != "capttm_ma_price_both" { return None; }
    let schema = params(id)?;
    if from > to || to >= x.r.len() { return None; }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(0., |d| d.def));
    let (f, s) = (sma(&x.c, g("fast").round().max(1.) as usize), sma(&x.c, g("slow").round().max(1.) as usize));
    let ok = |i: usize| f[i].is_finite() && s[i].is_finite();
    let mut out = vec![];
    let mut cur: Option<Trade> = None;
    for i in from.max(1)..=to {
        if !ok(i) || !ok(i - 1) { continue; }
        let c = x.c[i];
        if id == "capttm_ma14_50" {
            let up = f[i] > s[i] && f[i - 1] <= s[i - 1];
            let dn = f[i] < s[i] && f[i - 1] >= s[i - 1];
            let short = g("short") >= 0.5;
            if dn && cur.as_ref().map_or(false, |t| t.dir > 0.) { close(&mut out, &mut cur, i, c, "14-day average crossed below the 50-day"); }
            if up && cur.as_ref().map_or(false, |t| t.dir < 0.) { close(&mut out, &mut cur, i, c, "14-day average crossed above the 50-day"); }
            if cur.is_none() && i < to {
                if up { cur = Some(trade(1., i, c)); } else if dn && short { cur = Some(trade(-1., i, c)); }
            }
        } else {
            let (hi, lo) = (f[i].max(s[i]), f[i].min(s[i]));
            let (hi1, lo1) = (f[i - 1].max(s[i - 1]), f[i - 1].min(s[i - 1]));
            let above = c > hi && !(x.c[i - 1] > hi1);
            let below = c < lo && !(x.c[i - 1] < lo1);
            match cur.as_ref().map(|t| t.dir) {
                Some(d) if d > 0. && c < hi => close(&mut out, &mut cur, i, c, "close crossed below one of the averages"),
                Some(d) if d < 0. && c > lo => close(&mut out, &mut cur, i, c, "close crossed above one of the averages"),
                _ => {}
            }
            if cur.is_none() && i < to {
                if above { cur = Some(trade(1., i, c)); } else if below { cur = Some(trade(-1., i, c)); }
            }
        }
    }
    if let Some(mut t) = cur { t.exit_i = to; t.exit_px = x.c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true; out.push(t); }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn crossover_buys_and_sells_on_the_close() {
        // fast=2, slow=3: closes 10,10,10,13,16,10,4
        let cl = [10., 10., 10., 13., 16., 10., 4.];
        let r: Vec<Row> = cl.iter().enumerate().map(|(k, &c)| Row { ticker: "T".into(), date: format!("2001-01-{:02}", k + 1), open: c, high: c, low: c, close: c, volume: None }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let mut pr = Params::new();
        pr.insert("fast".into(), 2.); pr.insert("slow".into(), 3.);
        let ts = run("capttm_ma14_50", &x, 0, r.len() - 1, &pr).unwrap();
        // sma2: -,10,10,11.5,14.5,13,7 ; sma3: -,-,10,11,13,13,10 -> up at 3, down at 6 (13>=13 at 5)
        assert_eq!(ts.len(), 1);
        assert_eq!((ts[0].legs[0].i, ts[0].exit_i, ts[0].exit_px), (3, 6, 4.));
        assert!(catalogue().iter().all(|d| d.id.starts_with("capttm_") && family_of(d.id).is_some() && params(d.id).is_some()));
    }
}
