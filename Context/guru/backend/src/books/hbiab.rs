//! Howard B. Bandy, *Introduction to AmiBroker* (Blue Owl Press, 2008) - bounded reviewed examples within a legacy catalogue.
//! Source, page references and checks: library/hbiab/README.md.
//!
//! The reviewed examples use their own fixed-fraction normalized cash account
//! and exact instrument policy. Other catalogue entries remain legacy fragments,
//! outside this bounded source review. No index substitution or whole-book coverage
//! is implied. See docs/hbiab-three-review.md and docs/hbiab-simple-system-review.md.
use crate::connors::{true_range, wilder, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "hbiab";
/// Admitted ids (authors/bandy/admission/SOURCE-CONTRACT.md).
pub const REVIEWED: &[&str] = &["hbiab_trend_following", "hbiab_mean_reversion"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

fn ma(v: &[f64], n: usize) -> Vec<f64> {
    (0..v.len()).map(|i| if i + 1 < n { f64::NAN } else { v[i + 1 - n..=i].iter().sum::<f64>() / n as f64 }).collect()
}
fn stdev(v: &[f64], n: usize) -> Vec<f64> {
    (0..v.len()).map(|i| {
        if i + 1 < n { return f64::NAN; }
        let w = &v[i + 1 - n..=i];
        let m = w.iter().sum::<f64>() / n as f64;
        (w.iter().map(|x| (x - m).powi(2)).sum::<f64>() / n as f64).sqrt()
    }).collect()
}
fn ema(v: &[f64], n: usize) -> Vec<f64> {
    let f = 2.0 / (n as f64 + 1.0);
    let mut prev = f64::NAN;
    v.iter().map(|&x| { if !x.is_nan() { prev = if prev.is_nan() { x } else { f * x + (1.0 - f) * prev }; } prev }).collect()
}
fn cross(a: &[f64], b: &[f64]) -> Vec<bool> { (0..a.len()).map(|i| i > 0 && a[i] > b[i] && a[i - 1] <= b[i - 1]).collect() }
fn bars_since(b: &[bool]) -> Vec<Option<usize>> {
    let mut last = None;
    (0..b.len()).map(|i| { if b[i] { last = Some(i); } last.map(|l| i - l) }).collect()
}

const STK: &str = "The book shows this on single stocks / ETFs (CAT, IP, XLU, SPY) with dollar equity of a $10,000 account; no index result is printed. Actual stock/ETF applications require source-specific review; indices are not substitutes.";
macro_rules! def {
    ($id:expr, $name:expr, $g:expr, $st:expr, $rules:expr, $notes:expr) => {
        StrategyDef { id: $id, name: $name, group: $g, session: 0, dir: "Long", status: $st, needs_volume: false, needs_vix: false,
            needs_series: &[], book: BOOK, rules: $rules, notes: $notes, claim: None }
    };
}

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    const G3: &str = "Ch.3 Thirty minutes to useful results";
    const G5: &str = "Ch.5 Systems directory examples";
    const G10: &str = "Ch.9-10 Analysis and write it yourself";
    vec![
        def!("hbiab_ma5_20", "Example 6A/6B: 5/20 moving average crossover", G3, Live,
            &["E|FastMA = MA(C,5); SlowMA = MA(C,20); Buy = Cross(FastMA, SlowMA) (Example6A.afl, p.78; Example6X p.503)", "X|Sell = Cross(SlowMA, FastMA) (p.78)",
              "F|Settings: positions Long, \"Buy and Sell on the Close with Delay 0\" (p.503)"],
            "Book-reported CAT equity $14,013 on $10,000 (p.85), not reproduced. Printed503 also illustrates actual XLU ETF. Reviewed single-symbol application uses declared 100% fractional cash allocation, reserves entry costs, starts flat and retains terminal open marks. Separate Example7A portfolio is not reproduced."),
        def!("hbiab_ma7_4", "Example 8B: optimised 7/4 moving average crossover", G3, Live,
            &["E|FastMALength = 7, SlowMALength = 4 (the best in-sample values entered as the Optimize defaults, p.95-96); Buy = Cross(MA(C,7), MA(C,4)) (p.93)",
              "X|Sell = Cross(MA(C,4), MA(C,7)) (p.93)"],
            "\"we are buying when the fast moving average crosses down through the slow moving average\" - a mean-reversion reading (p.97). Book: CAT in-sample 7/1/2004-1/1/2007 final equity $21,154, to 8/31/2007 $22,447 (p.96-97)."),
        def!("hbiab_walk_forward", "Example 9: walk-forward re-optimisation of Example 8B", G3, NotTestable("a re-optimisation procedure: each six-month out-of-sample period uses the lengths the optimiser picked on CAT (p.102); the printed lengths are CAT optimiser output, not a rule"),
            &["E|Walk forward: in-sample 6 months, out-of-sample 6 months, RAR/MDD objective, 7/2004-8/2007 (p.98-103)"], ""),
        def!("hbiab_macd_example", "Example.afl: MACD / signal crossover, next-day open", G5, Live,
            &["E|Buy = Cross(MACD(), Signal()) (p.224)", "X|Sell = Cross(Signal(), MACD()) (p.224)", "F|SetTradeDelays(1,1,1,1); BuyPrice = SellPrice = Open (p.224-225)",
              "F|MACD12/26 and Signal9 defaults (printed308,317); full SMA seeds are a declared computational convention", "F|SetPositionSize(25, spsPercentOfEquity): source 25% of current equity (printed224)"],
            "Source sizes 25% of current equity per position (SetPositionSize(25, spsPercentOfEquity)); cash and paid costs retained in the reviewed normalized fractional account. EMA12/26 and signal9 use explicitly declared full SMA seeds; native AmiBroker binary parity is not claimed. No book result printed."),
        def!("hbiab_rotational", "ExampleRotational.afl: buy the 4 worst 252-day performers", G5, NeedsData("a stock universe ranked daily by -ROC(C,252) with 4 positions held (rotational backtest)"),
            &["E|PositionScore = 10000 - ROC(C,252); MaxPositions 4, WorstRankHeld 6, next-day open (p.225-226)"], ""),
        def!("hbiab_sector_portfolio", "Example 7A / BacktestExample: MA crossover on a watch-list portfolio", G10, NeedsData("the book's watch lists (Basic Materials stocks; the nine Select Sector SPDR ETFs) traded as a 2-3 position portfolio"),
            &["E|MA 5/20 crossover with MaxOpenPositions 2 or 3 and PositionSize = -100/PosQty (p.87-90, 497-501)"], ""),
        def!("hbiab_simple_ma25", "SimpleSystem.afl: above MA25, exit five bars since latest raw Buy", G10, Live,
            &["E|MA25 = MA(C,25); Buy = Close > MA25 (p.528)", "X|Sell = BarsSince(Buy) >= 5 (p.528)"],
            "Printed527-528 teaches interpretation/commentary on actual SPY; no performance study is printed. Every raw Buy resets the five-observed-bar clock, including while long; equality is false. Daily finalized same-close fills and 100% fractional current-equity cash sizing are declared research conventions. Full prior signal history, flat selected account start and terminal open positions; no native AmiBroker parity or book-performance claim. See docs/hbiab-simple-system-review.md."),
        def!("hbiab_trend_following", "TrendFollowing.afl: 50/30 MA crossover with 3.75 x ATR(14) chandelier stop", G10, Live,
            &["F|SetTradeDelays(1,1,1,1); BuyPrice = SellPrice = Open (p.542)", "E|MA1 = MA(C,50), MA2 = MA(C,30) (Optimize defaults); Buy = Cross(MA1, MA2) (p.542)",
              "X|Sell = Cross(MA2, MA1) (p.542)", "X|ApplyStop(stopTypeTrailing, stopModePoint, 3.75*ATR(14), True, True) - trailing stop 3.75 x ATR(14) below the highest high, exit at the stop (p.542)"],
            "\"This is just an example. Do Not trade this system\" (code comment). Trailing amount varies daily (volatile = True). Stops active from the entry bar (p.397 scenario 1: next-open trading). Book holds up to 2 positions of a sector-ETF watch list; one position here."),
        def!("hbiab_mean_reversion", "MeanReversion.afl: z-score(34) below -2, exit at 0, 9-bar and 3% profit stops", G10, Live,
            &["F|zs = (C - MA(C,34)) / StDev(C,34) (p.543)", "E|Buy = Cross(-2.0, zs) at the close (SetTradeDelays 0, prices Close) (p.543)", "X|Sell = Cross(zs, 0) (p.543)",
              "X|ApplyStop(stopTypeNBar, stopModeBars, 9) (p.543)", "X|ApplyStop(stopTypeProfit, stopModePercent, 3) - exit at entry + 3% (p.543)"],
            "\"This is just an example. Do Not trade this system.\" N-bar exit at the close 9 bars after entry; profit target intraday at the level (gap = open); stops from the bar after entry (close trading)."),
        def!("hbiab_zscore_weakness", "ZScoreBuyWeaknessTradingSystem.afl (walk-forward example)", G10, NotTestable("only the file name and its equity curves are shown (p.522-523); the code is not printed"),
            &["E|Code not printed (p.522-523)"], ""),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    if !id.starts_with("hbiab_") { return None; }
    let d = catalogue().into_iter().find(|d| d.id == id)?;
    let f = if id.contains("ma") && !id.contains("macd") || id == "hbiab_walk_forward" || id == "hbiab_sector_portfolio" || id == "hbiab_trend_following" {
        ("hbiab_ma", "Bandy AmiBroker · moving average systems")
    } else { ("hbiab_misc", "Bandy AmiBroker · other example systems") };
    Some((f.0, f.1, d.name))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(match id {
        "hbiab_ma5_20" => vec![p("fast", "FastMA length", 5., 5., 5., 1.), p("slow", "SlowMA length", 20., 20., 20., 1.)],
        "hbiab_ma7_4" => vec![p("fast", "FastMA length", 7., 7., 7., 1.), p("slow", "SlowMA length", 4., 4., 4., 1.)],
        "hbiab_macd_example" => vec![p("fast", "MACD fast EMA", 12., 12., 12., 1.), p("slow", "MACD slow EMA", 26., 26., 26., 1.), p("sig", "Signal EMA", 9., 9., 9., 1.)],
        "hbiab_simple_ma25" => vec![p("ma", "MA length", 25., 25., 25., 1.), p("hold", "Bars since latest raw Buy", 5., 5., 5., 1.)],
        "hbiab_trend_following" => vec![p("mal1", "MAL1", 50., 1., 51., 2.), p("mal2", "MAL2", 30., 2., 52., 2.), p("atr_mult", "ATR multiple", 3.75, 1., 5., 0.25), p("atr", "ATR length", 14., 2., 50., 1.)],
        "hbiab_mean_reversion" => vec![p("zs", "Z-score length", 34., 4., 50., 1.), p("level", "Buy level", -2.0, -4.0, 0.0, 0.25), p("hold", "N-bar stop", 9., 1., 10., 1.), p("pt", "Profit target %", 3., 0.5, 5., 0.25)],
        _ => return None,
    })
}

fn trade(ei: usize, ep: f64, xi: usize, xp: f64, why: &str) -> Trade {
    Trade { dir: 1.0, legs: vec![Leg { i: ei, px: ep, w: 1.0 }], exit_i: xi, exit_px: xp, reason: why.into(), entry_kind: "close", signal_i: xi, open: false, exits: vec![] }
}
fn finish(x: &Ctx, to: usize, mut out: Vec<Trade>, pos: Option<(usize, f64)>) -> Vec<Trade> {
    if let Some((ei, ep)) = pos { let mut t = trade(ei, ep, to, x.c[to], "open"); t.open = true; out.push(t); }
    out
}
/// Long-only, trades at the close of the signal bar (delay 0).
fn at_close(x: &Ctx, from: usize, to: usize, buy: &[bool], sell: &[bool]) -> Vec<Trade> {
    let (mut out, mut pos) = (vec![], None);
    for i in from..=to {
        if let Some((ei, ep)) = pos { if sell[i] { out.push(trade(ei, ep, i, x.c[i], "sell signal")); pos = None; } }
        if pos.is_none() && buy[i] && i < to { pos = Some((i, x.c[i])); }
    }
    finish(x, to, out, pos)
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if super::hbiab_reviewed::reviewed(id){return super::hbiab_reviewed::run(id,x,from,to,0.,p).ok().map(|a|a.trades);}
    if !id.starts_with("hbiab_") { return None; }
    let schema = params(id)?;
    if from >= to || to >= x.r.len() { return Some(vec![]); }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let u = |k: &str| g(k).round().max(1.0) as usize;
    let n = to + 1;
    let c = &x.c[..n];
    Some(match id {
        "hbiab_ma5_20" | "hbiab_ma7_4" => {
            let (f, s) = (ma(c, u("fast")), ma(c, u("slow")));
            at_close(x, from, to, &cross(&f, &s), &cross(&s, &f))
        }
        "hbiab_simple_ma25" => {
            let m = ma(c, u("ma"));
            let buy: Vec<bool> = (0..n).map(|i| c[i] > m[i]).collect();
            let sell: Vec<bool> = bars_since(&buy).iter().map(|b| b.is_some_and(|k| k >= u("hold"))).collect();
            at_close(x, from, to, &buy, &sell)
        }
        "hbiab_macd_example" => {
            let line: Vec<f64> = ema(c, u("fast")).iter().zip(ema(c, u("slow"))).map(|(a, b)| a - b).collect();
            let sig = ema(&line, u("sig"));
            let (buy, sell) = (cross(&line, &sig), cross(&sig, &line));
            let (mut out, mut pos) = (vec![], None);
            for t in from + 1..=to { // signals on bar t-1, trades at the open of t
                if let Some((ei, ep)) = pos { if sell[t - 1] { out.push(trade(ei, ep, t, x.o[t], "sell signal (next open)")); pos = None; } }
                if pos.is_none() && buy[t - 1] { pos = Some((t, x.o[t])); }
            }
            finish(x, to, out, pos)
        }
        "hbiab_trend_following" => {
            let (m1, m2) = (ma(c, u("mal1")), ma(c, u("mal2")));
            let (buy, sell) = (cross(&m1, &m2), cross(&m2, &m1));
            let a = wilder(&true_range(&x.r[..n]), u("atr"));
            let k = g("atr_mult");
            let (mut out, mut pos): (Vec<Trade>, Option<(usize, f64, f64)>) = (vec![], None); // entry i, px, highest high before today
            for t in from + 1..=to {
                if let Some((ei, ep, _)) = pos { if sell[t - 1] { out.push(trade(ei, ep, t, x.o[t], "sell signal (next open)")); pos = None; } }
                if pos.is_none() && buy[t - 1] { pos = Some((t, x.o[t], x.o[t])); }
                if let Some((ei, ep, hw)) = pos {
                    let st = hw - k * a[t];
                    if st.is_finite() && x.l[t] <= st {
                        let px = if x.o[t] <= st { x.o[t] } else { st };
                        out.push(trade(ei, ep, t, px, "chandelier trailing stop")); pos = None;
                    } else { pos = Some((ei, ep, hw.max(x.h[t]))); }
                }
            }
            finish(x, to, out, pos.map(|(i, p, _)| (i, p)))
        }
        "hbiab_mean_reversion" => {
            let (l, lev) = (u("zs"), g("level"));
            let (m, s) = (ma(c, l), stdev(c, l));
            let zs: Vec<f64> = (0..n).map(|i| (c[i] - m[i]) / s[i]).collect();
            let lvl = vec![lev; n];
            let zero = vec![0.0; n];
            let (buy, sell) = (cross(&lvl, &zs), cross(&zs, &zero));
            let (hold, pt) = (u("hold"), g("pt") / 100.0);
            let (mut out, mut pos): (Vec<Trade>, Option<(usize, f64)>) = (vec![], None);
            for i in from..=to {
                if let Some((ei, ep)) = pos {
                    if i > ei {
                        let tgt = ep * (1.0 + pt);
                        if i - ei >= hold { out.push(trade(ei, ep, i, x.c[i], "n-bar stop")); pos = None; }
                        else if x.o[i] >= tgt { out.push(trade(ei, ep, i, x.o[i], "profit target (gap)")); pos = None; }
                        else if x.h[i] >= tgt { out.push(trade(ei, ep, i, tgt, "profit target")); pos = None; }
                    }
                }
                if let Some((ei, ep)) = pos { if sell[i] { out.push(trade(ei, ep, i, x.c[i], "z-score back to 0")); pos = None; } }
                if pos.is_none() && buy[i] && i < to { pos = Some((i, x.c[i])); }
            }
            finish(x, to, out, pos)
        }
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn mean_reversion_n_bar_exit_on_the_ninth_bar() {
        let mut v: Vec<f64> = (0..60).map(|i| 100.0 + (i % 2) as f64).collect();
        for k in 40..60 { v[k] -= 20.0; } // a crash below -2 sigma, then no recovery
        let r: Vec<Row> = v.iter().enumerate().map(|(i, &c)| Row { ticker: "T".into(), date: format!("2001-{:02}-{:02}", 1 + i / 28, 1 + i % 28), open: c, high: c, low: c, close: c, volume: None }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let t = run("hbiab_mean_reversion", &x, 0, 59, &Params::new()).unwrap();
        assert_eq!(t[0].legs[0].i, 40); assert_eq!(t[0].exit_i, 49); assert_eq!(t[0].reason, "n-bar stop");
    }
    #[test]
    fn catalogue_ids_unique_and_live_have_params() {
        let c = catalogue(); let mut ids: Vec<_> = c.iter().map(|d| d.id).collect(); ids.sort(); ids.dedup();
        assert_eq!(ids.len(), c.len());
        for d in c.iter().filter(|d| d.status == Status::Live) { assert!(params(d.id).is_some(), "{}", d.id); }
    }
}
