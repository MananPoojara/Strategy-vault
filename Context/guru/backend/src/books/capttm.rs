//! Curtis M. Arnold, *Timing the Market: How to Profit in Bull and Bear Markets with Technical Analysis*, revised ed.
//! (Probus 1993; ch.1-25 (c) Weiss Research). Source copy: Books/Curtis Arnold/Curtis Arnold - Timing the Market.pdf
//! (140 PDF pages, two printed pages per PDF page). Page refs are printed pages. Notes: library/capttm/README.md.
//! A survey of indicators. Live: the two moving-average systems of ch.13 (p.82-83), the only rules the book gives
//! with both an entry and an exit and fixed numbers that run on index prices. Breadth / sentiment / monetary rules with
//! numbers need NYSE data the vault lacks (NeedsData); the rest is interpretation (NotTestable).
use crate::connors::{sma, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

const BOOK: &str = "capttm";
/// Admitted ids (authors/carnold/admission/SOURCE-CONTRACT.md).
pub const REVIEWED: &[&str] = &["capttm_ma14_50", "capttm_ma_price_both", "capttm_price_ma", "capttm_djua_15w"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

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
        d("capttm_price_ma", "Price crossing a single moving average (200-day)", "Ch 13 · Moving averages", "Long", Live, &[
            "E|\"Buy when prices cross above the moving average.\" (p.81); the example average is the 200-day (\"when a stock price is above its 200-day moving average, it is considered a bullish sign\", p.81)",
            "X|\"Sell when prices cross below the moving average.\" (p.81)",
            "A|[Declared reading 1, lane H1c 19 Nov 2026, R4 #339/#343] length: the default is the 200-day average, the text's own example for stocks (p.81); the shorter-term 10- and 20-day averages the text names (p.81) are the length option, not the default",
            "A|[Declared reading 2] average type: the simple average of closes with today's close included (built in Table 13-1, p.79-80); the weighted and exponential averages the text also names (p.80) are not run; price = the close (\"typically, you may wish to use closing prices only\", p.81)",
            "A|[Declared reading 3] 'cross above' = the close is above the average today and was not above it on the previous bar; 'cross below' the mirror; a close exactly on the average is neither (tie rule)",
            "A|[Declared reading 4] 'Sell' closes the long and does not open a short: the next paragraph separates 'sell' from 'Sell short' (p.82); the stop-and-reverse reading is the option short = 1, not the default",
        ], "Book p.81 prints no example results; the Dow 200-day chart (Figure 13-3, p.83) is a chart, not a trade list. Signals on the close, filled at that close [engine convention, as capttm_ma14_50]. Runs on the held indices, ETFs and stocks, US and India (declared application). adapted: 4 labelled readings (length, average type, cross/tie rule, sell = exit), UNVERIFIED."),
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
        StrategyDef { id: "capttm_djua_15w", name: "Dow Jones Utility Average vs its 15-week moving average (trade the S&P 500)", group: "Ch 18 · Sentiment", session: 0, dir: "Long", status: Live,
            needs_volume: false, needs_vix: false, needs_series: &["DJ_UTILITY"], book: BOOK, claim: None, rules: &[
            "E|\"When the current readings are above the 15-week moving average, the utility stocks can be classified as being in an uptrend and the stock market also should continue higher.\" (p.129)",
            "X|\"When current readings drop below the 15-week moving average, a sell signal is in order.\" (p.129)"],
            notes: "[Adapted: 3 labelled readings, lane K3 24 Nov 2026; unverified] (1) Weekly bars: the last session of each Monday-Friday week on the traded index's calendar; the weekly close of the Dow Jones Utility Average (Norgate $DJU, 1929-2020-09-04) is read on that session, and a week whose last session has no utility close is dropped (not moved to the day before). (2) The 15-week simple average includes the current week. (3) The book gives no buy order ('the stock market also should continue higher') and says 'a sell signal is in order': above the average = long the traded index at that week's close, below = close the long (p.short=1 [non-default] sells short instead of going flat). Convention, not a reading: a close exactly equal to the average holds the current position. Window: the S&P 500 run starts on 1991-05-28 (the first date the engine finds the utility series fully covered; earlier weeks only warm the average) and ends 2020-09-04, where the utility series ends. No results printed." },
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
        "capttm_price_ma" => ("capttm_ma", "Timing the Market moving-average systems", "price vs one average"),
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
        "capttm_price_ma" => vec![p("len", "Moving-average length (days; the book's example 200, shorter-term 10 or 20)", 200., 2., 400., 1.), p("short", "Also sell short on the cross below (stop-and-reverse; not the default reading)", 0., 0., 1., 1.)],
        "capttm_djua_15w" => vec![p("len", "Moving-average length (weeks; the book says 15)", 15., 2., 60., 1.), p("short", "Sell short below the average instead of going flat (not the default reading)", 0., 0., 1., 1.)],
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
    if !reviewed(id) { return None; }
    let schema = params(id)?;
    if from > to || to >= x.r.len() { return None; }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(0., |d| d.def));
    if id == "capttm_djua_15w" { // lane K3 (24 Nov 2026): weekly Dow Jones Utility Average vs its 15-week average, traded on the index
        let Some(ut) = x.series("DJ_UTILITY") else { return Some(vec![]) };
        let to = (0..=to).rev().find(|&i| ut[i].is_finite()).unwrap_or(0); // the utility series ends 2020-09-04: no signal beyond it
        let wk: Vec<usize> = super::apta::bars(x, 1).into_iter().filter(|&i| i <= to && ut[i].is_finite()).collect();
        let (len, short) = (g("len").round().max(1.) as usize, g("short") >= 0.5);
        let mut tgt = vec![0i8; to + 1];
        let mut pos = 0i8;
        for k in 0..wk.len() {
            if k + 1 >= len {
                let m = wk[k + 1 - len..=k].iter().map(|&i| ut[i]).sum::<f64>() / len as f64;
                if ut[wk[k]] > m { pos = 1; } else if ut[wk[k]] < m { pos = if short { -1 } else { 0 }; }
            }
            let end = wk.get(k + 1).map_or(to, |&j| j - 1);
            for t in &mut tgt[wk[k]..=end] { *t = pos; }
        }
        return Some(super::rbtie::target_trades(x, &tgt, from, to, false, "Utility average vs its 15-week average"));
    }
    if id == "capttm_price_ma" { // lane H1c (19 Nov 2026): close crossing its simple average, long (optionally stop-and-reverse)
        let m = sma(&x.c, g("len").round().max(1.) as usize);
        let (mut out, mut cur) = (vec![], None::<Trade>);
        for i in from.max(1)..=to {
            if !m[i].is_finite() || !m[i - 1].is_finite() { continue; }
            let c = x.c[i];
            let up = c > m[i] && !(x.c[i - 1] > m[i - 1]);
            let dn = c < m[i] && !(x.c[i - 1] < m[i - 1]);
            if dn && cur.as_ref().map_or(false, |t| t.dir > 0.) { close(&mut out, &mut cur, i, c, "close crossed below the average"); }
            if up && cur.as_ref().map_or(false, |t| t.dir < 0.) { close(&mut out, &mut cur, i, c, "close crossed above the average"); }
            if cur.is_none() && i < to {
                if up { cur = Some(trade(1., i, c)); } else if dn && g("short") >= 0.5 { cur = Some(trade(-1., i, c)); }
            }
        }
        if let Some(mut t) = cur { t.exit_i = to; t.exit_px = x.c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true; out.push(t); }
        return Some(out);
    }
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
    #[test]
    fn price_crosses_its_average() {
        // len 3: closes 10,10,10,13,16,10,4 -> sma3 -,-,10,11,13,13,10; close 13 > 11 and 10 <= 10 at bar 3: buy; close 10 < 13 at bar 5: sell
        let cl = [10., 10., 10., 13., 16., 10., 4.];
        let r: Vec<Row> = cl.iter().enumerate().map(|(k, &c)| Row { ticker: "T".into(), date: format!("2001-01-{:02}", k + 1), open: c, high: c, low: c, close: c, volume: None }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let mut pr = Params::new(); pr.insert("len".into(), 3.);
        let ts = run("capttm_price_ma", &x, 0, r.len() - 1, &pr).unwrap();
        assert_eq!(ts.len(), 1);
        assert_eq!((ts[0].legs[0].i, ts[0].exit_i, ts[0].exit_px), (3, 5, 10.));
    }
}

// Instrument admission: CArnold 2026-10-24 block, moved from instrument_policy.rs on 31 Oct 2026 (lane ST).
// Admission only for the module REVIEWED lists (authors/carnold/admission/SOURCE-CONTRACT.md); kept in one block to merge.
// Runs on actual indices, ETFs and stocks, US and India (index_etf_or_stock, nifty50 basket, declared application).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| matches!(def.book,"capttm") && (crate::books::capttm::reviewed(def.id)),
    label: "Curtis Arnold's moving-average examples come from gold futures (Merrill Lynch study) and general futures/stock charting; runs on indices, ETFs and stocks, US and India, are a declared application, not the author's test. Cash-index shorts are hypothetical exposure.",
    basis: "Curtis M. Arnold, Timing the Market (1993), ch.13 Moving averages (p.82): the single price-vs-moving-average rule (p.81, 200-day default), the 14/50-day two-crossover model and its modified version (close above/below both averages, exit when the close crosses either). Signals and fills on the close. Reviewed in authors/carnold/admission/SOURCE-CONTRACT.md with independent chronological ledgers; no costs.",
};

#[cfg(test)]
mod h1c_dump {
    use super::*;
    /// Lane H1c reference feed: `CP_DUMP=out CP_CSV=us_data.csv CP_TICKER=SPY [CP_LEN=200] [CP_SHORT=1] cargo test --release cp_dump -- --ignored` (CP_FMT=idx for idx_data.csv dates)
    #[test]
    #[ignore]
    fn cp_dump() {
        let (Ok(out), Ok(csv), Ok(tk)) = (std::env::var("CP_DUMP"), std::env::var("CP_CSV"), std::env::var("CP_TICKER")) else { return };
        let iso = |d: &str| format!("{}-{}-{}", &d[6..10], &d[3..5], &d[0..2]);
        let rows: Vec<crate::Row> = std::fs::read_to_string(csv).unwrap().lines().skip(1).filter_map(|l| { let f: Vec<&str> = l.split(',').collect();
            if f[0] != tk { return None; }
            Some(crate::Row { ticker: tk.clone(), date: iso(f[1]), open: f[2].parse().ok()?, high: f[3].parse().ok()?, low: f[4].parse().ok()?, close: f[5].parse().ok()?, volume: None }) }).collect();
        let x = Ctx::new(&rows, &rows, &[]);
        let mut p = Params::new();
        if let Ok(v) = std::env::var("CP_LEN") { p.insert("len".into(), v.parse().unwrap()); }
        if let Ok(v) = std::env::var("CP_SHORT") { p.insert("short".into(), v.parse().unwrap()); }
        let t = run("capttm_price_ma", &x, 0, rows.len() - 1, &p).unwrap();
        let mut o = String::new();
        for z in &t { o += &format!("{}\t{}\t{}\t{}\t{}\t{}\n", rows[z.legs[0].i].date, z.legs[0].px, rows[z.exit_i].date, z.exit_px, z.dir, z.open); }
        std::fs::write(out, o).unwrap();
    }
}
