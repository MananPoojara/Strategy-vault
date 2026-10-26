//! Active Trader Magazine, 2002 volume: the monthly "Trading System Lab" columns (Jan-Dec 2002) and the
//! "Futures & Options Trading System Lab" (Oct-Dec 2002). Book id `atm02`. Source copies: Books/Active Trader Magazine/2002/
//! (tradingsystem(s)lab*.djvu, futures-tsl*.djvu, tsl1202.djvu; Aug 2002 from "Binder - Trading System Labs V2
//! Countrend Trading.pdf"). The Oct 2002 equity column (p.56) is not on the research drive.
//! Transcription, testability table and checks: library/atm02/README.md.
//!
//! Engine and fill conventions: atm00 `sim` (see atm00.rs). Sizing (ATR-based share counts, % risk) is not modelled:
//! full equity per trade, one index or stock at a time. India: NIFTY 50 / basket:nifty50:<SYMBOL>.
//! Duplicates already in the vault (not repeated): Expert exits (Jun 2002) = strmm_expert (p.hh_lag=1 is the column's
//! prose reading); Counterpunch stock system (Nov 2002) = strmm_counterpunch.
use super::atm00::{avg_tr, levels, prev_week_closes, sim, Ord};
use crate::connors::{sma, Claim, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "atm02";
/// Admitted ids (authors/activetrader2/admission/SOURCE-CONTRACT.md).
pub const REVIEWED: &[&str] = &["atm02_bandwagon", "atm02_rev_bandwagon", "atm02_pristine", "atm02_ma_9_36", "atm02_idnr4", "atm02_counterpunch_fut"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

macro_rules! def {
    ($id:expr, $name:expr, $g:expr, $dir:expr, $st:expr, $rules:expr, $notes:expr, $claim:expr) => {
        StrategyDef { id: $id, name: $name, group: $g, session: 0, dir: $dir, status: $st, needs_volume: false, needs_vix: false,
            needs_series: &[], book: BOOK, rules: $rules, notes: $notes, claim: $claim }
    };
}
fn cl(t: &'static str, w: &'static str, a: &'static str, d: &'static str, p: &'static str) -> Option<Claim> {
    Some(Claim { trades: t, win: w, avg: a, hold: d, period: p })
}

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    const MULTI: &str = "multi-instrument position: the column holds a long and a short stock at the same time in one account (pair / cross-section of a stock group); the vault engine runs one ticker per request and a Trade has one instrument";
    vec![
        def!("atm02_pair_rank", "Pair-trade system: long weakest / short strongest of 10 semis (Jan 2002 TSL)", "Trading System Lab · Jan 2002", "Long/Short",
            NeedsData(MULTI), &["E|\"1. Go long tomorrow the stock with the lowest rate of change over the last five days. 2. Go short tomorrow the stock with the highest rate of change over the last five days.\" ROC = close today / close five days ago (TSL Jan 2002 p.64)",
              "X|\"4. Exit all trades after six days.\" (TSL Jan 2002 p.64)"],
            "Book: ADI, AMAT, INTC, KLAC, KYO, LLTC, MXIM, NVLS, TXN, XLNX, Mar 1992-Oct 2001, 4% risk: 783 trades, avg $113. India would need a 10-stock NSE sector basket traded as one portfolio.", None),
        def!("atm02_pair_ema", "Pair trade II: 9-day EMA of the A/B price ratio, stop-and-reverse (Feb 2002 TSL)", "Trading System Lab · Feb 2002", "Long/Short",
            NeedsData(MULTI), &["F|\"The relative strength of the stocks is measured by the nine-day exponential moving average of the ratio of the price of stocks A and B\" (TSL Feb 2002 p.58)",
              "E|\"1. Go long market A and short market B on the next trading day if the nine-day average relative strength of the current trading day is higher than the nine-day average relative strength of nine days ago. 2. Go short market A and long market B ... lower\" (TSL Feb 2002 p.58)",
              "X|\"4. No exit rules. System is stop-and-reverse.\" (TSL Feb 2002 p.58)"],
            "Book: HAL/SLB, NOK/ERICY, DELL/CPQ, BGEN/HGSI, Apr 1992-Nov 2001: 962 trades, 45.8% winners.", None),
        def!("atm02_bandwagon", "The bandwagon trade: 2 x ATR(5) breakout (Mar 2002 TSL)", "Trading System Lab · Mar 2002", "Long/Short", Live,
            &["E|\"1. Go long tomorrow if the market crosses above yesterday's close plus two times the five-day average true range.\" (TSL Mar 2002 p.76) - logic: \"trades above the closing price two days ago by an amount equal to two times the five-day ATR\": stop order at the close of the bar before the signal bar + 2 x ATR(5) of the signal bar (p.ref_lag=0: signal-bar close)",
              "X|\"3. Exit with a loss if the market trades below the entry price by an amount equal to the five-day average true range. 4. Exit with a profit if the market trades two times the five-day average true range above the entry price. Reverse rules for shorts.\" (TSL Mar 2002 p.76)",
              "F|ATR = TradeStation AvgTrueRange (simple 5-day average of true range), taken at the signal bar"],
            "Book: 30 Nasdaq-100 stocks, May 1992-Nov 2001, 1% risk, $1 million start: 4,114 trades, 32.8% winners, avg -$127, 5.3 days (TSL Mar 2002 p.77).",
            cl("4,114", "32.8%", "-$127", "5.3 days", "30 Nasdaq-100 stocks 5/1992-11/2001")),
        def!("atm02_rev_bandwagon", "Reverse Bandwagon: fade a 2 x ATR(5) move with limits (Aug 2002 TSL)", "Trading System Lab · Aug 2002", "Long/Short", Live,
            &["E|\"1. Go long tomorrow with a limit order if the market touches yesterday's close minus two times the five-day ATR.\" (TSL Aug 2002 p.1; binder p.3) - same reference close as atm02_bandwagon (\"closing price two days earlier\"); short mirror",
              "X|\"3. Exit with a loss if the market trades below the entry price by an amount equal to the five-day ATR. 4. Exit with a profit if the market trades above the entry price by an amount equal to two times the five-day ATR. Reverse rules for short trades.\""],
            "Book: 30 Nasdaq-100 stocks, Sep 1992-Apr 2002, 5% risk: 4,157 trades, 39.7% winners, avg $26, 5.6 days.",
            cl("4,157", "39.7%", "$26", "5.6 days", "30 Nasdaq-100 stocks 9/1992-4/2002")),
        def!("atm02_pair_trend", "Pair-trade system III: trend-dependent rank pair (Apr 2002 TSL)", "Trading System Lab · Apr 2002", "Long/Short",
            NeedsData(MULTI), &["F|\"The general trend is defined by an 80-day moving average of the Nasdaq 100 index. If the average is above its value 11 days ago, the trend is considered to be up\" (TSL Apr 2002 p.62)",
              "E|\"2a. If up: Go long the stock with the lowest ROC over the last five days. Go short the stock with the highest ROC ... 2b. If down: Go long the stock with the highest ROC ... Go short the stock with the lowest ROC\" (TSL Apr 2002 p.62)",
              "X|\"4. Exit all trades after six days.\" (TSL Apr 2002 p.62)"],
            "Book: same 10 semiconductor stocks, Jun 1992-Dec 2001, 3% risk: 1,552 trades, 51.5% winners.", None),
        def!("atm02_pristine", "Pristine variation: three down bars pullback (May 2002 TSL)", "Trading System Lab · May 2002", "Long", Live,
            &["F|\"1. Setup: Look for three consecutive down bars with closes lower than their opens and successively lower highs.\" (TSL May 2002 p.44 (issue PDF; djvu clipping p.70)) - each high below the previous bar's high",
              "E|\"2. Entry: Go long on a move above the last (and lowest) of the three down bars.\" (TSL May 2002 p.44 (issue PDF; djvu clipping p.70)) - buy stop at its high, the next day only [reading]",
              "X|\"3. Trailing stop: Exit if price breaks yesterday's low. 4. Profit taking: Take profits on the open if the market gaps higher than yesterday's high. 5. 'Reversal' stop: Exit if today's high is higher than yesterday's high, but today's close is lower than yesterday's close.\" (TSL May 2002 p.44 (issue PDF; djvu clipping p.70)) - the reversal stop is a close test and also applies on the entry day"],
            "Book: 30 Nasdaq-100 stocks, Jun 1992-Jan 2002, 5% risk / 4 x ATR(10) sizing, $20 costs: 873 trades, 55.9% winners, avg $587, 5.2 days (TSL May 2002 p.45).",
            cl("873", "55.9%", "$587", "5.2 days", "30 Nasdaq-100 stocks 6/1992-1/2002")),
        def!("atm02_ma_9_36", "Moving average crossover 9/36 with 4 x ATR stop (Jul 2002 TSL)", "Trading System Lab · Jul 2002", "Long", Live,
            &["E|\"1. Go long tomorrow on the open if the short-term MA crosses above the long-term MA.\" nine-day and 36-day averages, long only (TSL Jul 2002 p.70)",
              "X|\"2. Exit with a loss if the market goes against you by more than four times the 10-day ATR. 3. Exit with a profit or loss on the open tomorrow if the short-term MA crosses below the long-term MA.\" (TSL Jul 2002 p.70) - ATR(10) of the signal bar, stop fixed"],
            "Book: 30 Nasdaq-100 stocks, Aug 1992-Mar 2002, 2% risk: 1,065 trades, 45.0% winners, avg $860, 38 days (TSL Jul 2002 p.71).",
            cl("1,065", "45.0%", "$860", "38.0 days", "30 Nasdaq-100 stocks 8/1992-3/2002")),
        def!("atm02_idnr4", "Low-volatility breakout: inside day / NR4, 3:1 range exits (Sep 2002 TSL)", "Trading System Lab · Sep 2002", "Long/Short", Live,
            &["F|\"1. Prepare to go long tomorrow if today's range is the smallest of the last four days, and if today's high and low are below and above yesterday's high and low, respectively.\" (TSL Sep 2002 p.78) - strictly smallest",
              "E|\"2. Go long tomorrow if the market trades above today's high.\" (TSL Sep 2002 p.78); \"goes long or short ... (reverse the rules for short trades)\" (logic)",
              "X|\"3. Exit with a loss if the market trades below the entry price by an amount equal to the range of the day preceding the entry. 4. Exit with a profit if the market trades above the entry price by three times the range of the day preceding entry.\" (TSL Sep 2002 p.78)"],
            "Book: 30 Nasdaq-100 stocks, Sep 1992-Apr 2002, 4% risk: 945 trades, 41.3% winners, avg $89, 6.9 days (TSL Sep 2002 p.79).",
            cl("945", "41.3%", "$89", "6.9 days", "30 Nasdaq-100 stocks 9/1992-4/2002")),
        def!("atm02_vol_breakout_fut", "Futures volatility breakout: 60-day MA + Bollinger (Oct 2002 F&O TSL)", "F&O Trading System Lab · Oct 2002", "Long/Short",
            NotTestable("the Bollinger Band length is not given in this column - \"the logic and tools for this system are described in the Trading System Lab on p. 56\" (the October 2002 equity column, not on the research drive)"),
            &["E|\"1. Go long tomorrow if price moves above the 60-day moving average and breaks the upper Bollinger Band.\" (F&O TSL Oct 2002 p.70)",
              "X|\"2. Exit with a profit or loss if price moves below its 30-day moving average and penetrates a lower Bollinger Band set one standard deviation away (instead of the usual two). Reverse the rules for short trades.\""],
            "Book: 8 currency futures, Nov 1992-Jun 2002: 303 trades, 42.1% winners.", None),
        def!("atm02_counterpunch_fut", "Counterpunch stock index futures system (Nov 2002 F&O TSL)", "F&O Trading System Lab · Nov 2002", "Long/Short", Live,
            &["E|\"1. Go long tomorrow on the open if a) today's close is below both yesterday's close and the close of the previous week, b) yesterday's close is below the previous day's close and c) the close of the previous week is below the close of the week before that.\" (F&O TSL p.74) Week = calendar week, week close = its last daily close",
              "X|\"2. Exit one-third of the position with a loss if the trade goes against you by 1 percent. 3. Exit one-third of the position with a profit if the trade goes your way by 4 percent.\"",
              "X|\"4. Exit two-thirds of the position with a profit or loss if the trade moves 1.6 percent away from your maximum open profit (i.e., use a trailing stop 1.6 percent away from the high of the trade). 5. Exit two-thirds of the position with a profit if the trade goes your way by 4.5 percent.\"",
              "X|\"6. Exit the entire position after eight days in the trade. Reverse the rules for short trades.\" - close of the 8th day counting the entry day [reading]; the stock version (halves) is strmm_counterpunch"],
            "Book: S&P 500, Nasdaq 100 and Dow futures, Jan 1993-Jul 2002, 6% risk, $25 per contract: 1,258 trades, 36.3% winners, avg $849, 3.4 days, 21.9 trades/market/year (Dow from late 1996, Nasdaq from late 1997). Vault: SPX/NDX/DJI cash indexes; India: NIFTY 50.",
            cl("1,258 (3 markets)", "36.3%", "$849", "3.4 days", "S&P/Nasdaq/Dow futures 1/1993-7/2002")),
        def!("atm02_vol_bench_stock", "Volatility benchmark system, stocks (Dec 2002 TSL)", "Trading System Lab · Dec 2002", "Long/Short",
            NotTestable("entries are chosen by a random number generator (\"it's impossible to duplicate the results\") and the trend is fixed by hindsight (up to April 1, 2000, down after) - TSL Dec 2002 p.50"),
            &["E|\"1. Go long on the close if the long-term trend is up and the short-term volatility is higher than the long-term volatility\" (10-day vs 63-day ATR), taken only when the RNG returns 1",
              "X|\"3. Exit with a loss if the trade goes against you by the 10-day ATR. 4. Exit with a profit if the trade goes your way by three times the 10-day ATR. 5. Exit any remaining positions ... after 10 days in the trade.\""],
            "Book: 30 Nasdaq-100 stocks, Feb 1993-Aug 2002: 3,706 trades, 22.4% winners.", None),
        def!("atm02_vol_bench_fut", "Futures volatility benchmark system (Dec 2002 F&O TSL)", "F&O Trading System Lab · Dec 2002", "Long/Short",
            NotTestable("random entry: \"all trades are entered at random\" (F&O TSL Dec 2002 p.62); the 80-day MA / 20-day slope trend and 10/63-day ATR filter alone are not the system"),
            &["E|\"1. Go long on the close if the long-term trend is up and the short-term volatility (10-day ATR) is higher than the long-term volatility (63-day ATR).\" trend up = 80-day MA above its value 20 days ago; random entry",
              "X|ATR(10) stop, 3 x ATR(10) target, exit after 10 days"],
            "Book: 8 currency futures, Feb 1993-Aug 2002: 819 trades, 38.7% winners.", None),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let d = catalogue().into_iter().find(|d| d.id == id)?;
    Some(("atm02_tsl", "Active Trader · Trading System Lab 2002", d.name))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let bw = || vec![p("atr_n", "ATR days", 5., 2., 20., 1.), p("k_entry", "Entry distance x ATR", 2., 0.5, 5., 0.25),
        p("k_stop", "Stop x ATR", 1., 0.25, 5., 0.25), p("k_tgt", "Target x ATR", 2., 0.5, 10., 0.25),
        p("ref_lag", "Reference close: 1 bar before signal bar, 0 signal bar", 1., 0., 1., 1.)];
    Some(match id {
        "atm02_bandwagon" | "atm02_rev_bandwagon" => bw(),
        "atm02_pristine" => vec![p("bars", "Down bars", 3., 2., 6., 1.)],
        "atm02_ma_9_36" => vec![p("fast", "Short MA days", 9., 2., 50., 1.), p("slow", "Long MA days", 36., 10., 200., 1.),
            p("atr_n", "ATR days", 10., 2., 50., 1.), p("k", "Stop x ATR", 4., 1., 10., 0.5)],
        "atm02_idnr4" => vec![p("nr", "Narrowest range of days", 4., 2., 10., 1.), p("k_stop", "Stop x prior range", 1., 0.25, 5., 0.25),
            p("k_tgt", "Target x prior range", 3., 0.5, 10., 0.25)],
        "atm02_counterpunch_fut" => vec![p("stop_pct", "Part 1 stop %", 1., 0.25, 5., 0.25), p("tgt1_pct", "Part 1 target %", 4., 1., 10., 0.25),
            p("trail_pct", "Part 2 trailing stop %", 1.6, 0.25, 5., 0.1), p("tgt2_pct", "Part 2 target %", 4.5, 1., 10., 0.25),
            p("hold", "Exit on close of day (entry day = 1)", 8., 2., 20., 1.), p("part1", "Part 1 fraction", 1. / 3., 0.1, 0.9, 0.05)],
        _ => return None,
    })
}

fn counterpunch(x: &Ctx, from: usize, to: usize, g: &dyn Fn(&str) -> f64) -> Vec<Trade> {
    let (c, o, h, l) = (&x.c, &x.o, &x.h, &x.l);
    let wc = prev_week_closes(x);
    let (sp, t1, tr, t2, f1) = (g("stop_pct") / 100., g("tgt1_pct") / 100., g("trail_pct") / 100., g("tgt2_pct") / 100., g("part1"));
    let hold = g("hold").round().max(1.0) as usize;
    let sig = |s: usize| -> f64 {
        if s < 2 { return 0.0; }
        let (w1, w2) = wc[s];
        if !(w1.is_finite() && w2.is_finite()) { return 0.0; }
        if c[s] < c[s - 1] && c[s] < w1 && c[s - 1] < c[s - 2] && w1 < w2 { 1.0 }
        else if c[s] > c[s - 1] && c[s] > w1 && c[s - 1] > c[s - 2] && w1 > w2 { -1.0 } else { 0.0 }
    };
    struct P { d: f64, ei: usize, e: f64, hh: f64, ll: f64, a: bool, b: bool, ex: Vec<(usize, f64, f64)>, why: Vec<&'static str> }
    let close = |p: P, out: &mut Vec<Trade>, open: bool| {
        let i = p.ex.iter().map(|e| e.0).max().unwrap_or(p.ei);
        let px = p.ex.iter().map(|e| e.1 * e.2).sum::<f64>() / p.ex.iter().map(|e| e.2).sum::<f64>();
        out.push(Trade { dir: p.d, legs: vec![Leg { i: p.ei, px: p.e, w: 1.0 }], exit_i: i, exit_px: px, reason: p.why.join(" + "),
            entry_kind: "open", signal_i: i, open, exits: p.ex });
    };
    let mut out = vec![];
    let mut pos: Option<P> = None;
    for t in from.max(1)..=to {
        if let Some(mut p) = pos.take() {
            if t > p.ei {
                if p.a {
                    if let Some((px, w)) = levels(x, t, p.d, Some(p.e * (1.0 - p.d * sp)), Some(p.e * (1.0 + p.d * t1))) {
                        p.ex.push((t, px, f1)); p.why.push(if w.starts_with("stop") { "1/3 stop" } else { "1/3 target" }); p.a = false;
                    }
                }
                if p.b {
                    let trail = if p.d > 0.0 { p.hh * (1.0 - tr) } else { p.ll * (1.0 + tr) };
                    if let Some((px, w)) = levels(x, t, p.d, Some(trail), Some(p.e * (1.0 + p.d * t2))) {
                        p.ex.push((t, px, 1.0 - f1)); p.why.push(if w.starts_with("stop") { "2/3 trailing stop" } else { "2/3 target" }); p.b = false;
                    }
                }
            }
            p.hh = p.hh.max(h[t]);
            p.ll = p.ll.min(l[t]);
            if (p.a || p.b) && t + 1 - p.ei >= hold {
                let rem = if p.a { f1 } else { 0.0 } + if p.b { 1.0 - f1 } else { 0.0 };
                p.ex.push((t, c[t], rem)); p.why.push("time exit (8 days)"); p.a = false; p.b = false;
            }
            if p.a || p.b { pos = Some(p); } else { close(p, &mut out, false); continue; }
        }
        if pos.is_none() {
            let d = sig(t - 1);
            if d != 0.0 { pos = Some(P { d, ei: t, e: o[t], hh: h[t], ll: l[t], a: true, b: true, ex: vec![], why: vec![] }); }
        }
    }
    if let Some(mut p) = pos {
        let rem = if p.a { f1 } else { 0.0 } + if p.b { 1.0 - f1 } else { 0.0 };
        p.ex.push((to, c[to], rem)); p.why.push("open");
        close(p, &mut out, true);
    }
    out
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !id.starts_with("atm02_") { return None; }
    let schema = params(id)?;
    if from >= to || to >= x.r.len() { return Some(vec![]); }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let u = |k: &str| g(k).round().max(1.0) as usize;
    let (c, o, h, l) = (&x.c, &x.o, &x.h, &x.l);
    Some(match id {
        "atm02_bandwagon" | "atm02_rev_bandwagon" => {
            let a = avg_tr(x, u("atr_n"));
            let (ke, ks, kt) = (g("k_entry"), g("k_stop"), g("k_tgt"));
            let lag = g("ref_lag").round().max(0.0) as usize;
            let rev = id == "atm02_rev_bandwagon";
            sim(x, from, to, &|t| {
                let s = t - 1;
                if s < lag || !a[s].is_finite() { return vec![]; }
                let (up, dn) = (c[s - lag] + ke * a[s], c[s - lag] - ke * a[s]);
                if rev { vec![(1.0, Ord::Limit(dn)), (-1.0, Ord::Limit(up))] } else { vec![(1.0, Ord::Stop(up)), (-1.0, Ord::Stop(dn))] }
            }, &|t, q| {
                if t == q.ei { return None; }
                let r = a[q.sig];
                levels(x, t, q.dir, Some(q.epx - q.dir * ks * r), Some(q.epx + q.dir * kt * r))
            })
        }
        "atm02_pristine" => {
            let nb = u("bars");
            sim(x, from, to, &|t| {
                let s = t - 1;
                if s < nb + 1 { return vec![]; }
                let ok = (0..nb).all(|k| { let b = s - k; c[b] < o[b] && h[b] < h[b - 1] });
                if ok { vec![(1.0, Ord::Stop(h[s]))] } else { vec![] }
            }, &|t, q| {
                if t > q.ei {
                    if o[t] > h[t - 1] { return Some((o[t], "gap above yesterday's high (open)")); }
                    if let Some(e) = levels(x, t, 1.0, Some(l[t - 1]), None) { return Some(e); }
                }
                (h[t] > h[t - 1] && c[t] < c[t - 1]).then(|| (c[t], "reversal stop (close)"))
            })
        }
        "atm02_ma_9_36" => {
            let (f, s) = (sma(c, u("fast")), sma(c, u("slow")));
            let a = avg_tr(x, u("atr_n"));
            let k = g("k");
            sim(x, from, to, &|t| {
                let y = t - 1;
                if y >= 1 && f[y] > s[y] && f[y - 1] <= s[y - 1] { vec![(1.0, Ord::Open)] } else { vec![] }
            }, &|t, q| {
                if t == q.ei { return None; }
                let y = t - 1;
                if f[y] < s[y] && f[y - 1] >= s[y - 1] { return Some((o[t], "MA cross below: exit next open")); }
                levels(x, t, 1.0, Some(q.epx - k * a[q.sig]), None)
            })
        }
        "atm02_idnr4" => {
            let nr = u("nr");
            let (ks, kt) = (g("k_stop"), g("k_tgt"));
            let rg: Vec<f64> = (0..c.len()).map(|i| h[i] - l[i]).collect();
            sim(x, from, to, &|t| {
                let s = t - 1;
                if s < nr { return vec![]; }
                let narrow = (1..nr).all(|k| rg[s] < rg[s - k]);
                if narrow && h[s] < h[s - 1] && l[s] > l[s - 1] { vec![(1.0, Ord::Stop(h[s])), (-1.0, Ord::Stop(l[s]))] } else { vec![] }
            }, &|t, q| {
                if t == q.ei { return None; }
                let r = rg[q.sig];
                levels(x, t, q.dir, Some(q.epx - q.dir * ks * r), Some(q.epx + q.dir * kt * r))
            })
        }
        "atm02_counterpunch_fut" => counterpunch(x, from, to, &g),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn idnr4_long_breakout_hits_three_range_target() {
        let v = [(100., 110., 90., 100.), (100., 108., 92., 100.), (100., 106., 94., 100.), (100., 104., 96., 100.),
                 (100., 103., 98., 101.), // inside day, range 5 < 8,12,16 -> long stop 103 / short stop 98
                 (102., 104., 101., 103.5), // long fills at 103; stop 98, target 118
                 (104., 119., 103., 118.5)]; // target 118 hit
        let r: Vec<Row> = v.iter().enumerate().map(|(i, &(o, h, l, c))| Row { ticker: "T".into(), date: format!("2002-01-{:02}", i + 1),
            open: o, high: h, low: l, close: c, volume: None }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let t = run("atm02_idnr4", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert_eq!(t.len(), 1);
        assert_eq!((t[0].dir, t[0].legs[0].i, t[0].legs[0].px), (1.0, 5, 103.0));
        assert_eq!((t[0].exit_i, t[0].exit_px), (6, 118.0));
    }
}
