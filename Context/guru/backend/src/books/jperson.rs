//! John L. Person - pivot-point books. Book ids:
//!   `jpcppt` Candlestick and Pivot Point Trading Triggers (Wiley 2007; printed page = pdf page - 16): the three-period
//!            pivot-point moving average close rule (p.167-168) and the pivot / pivot-average crossover, COMAS (p.168-174).
//!   `jpfxc`  Forex Conquered (Wiley 2007): the same two rules (p.94-95); 15-minute forex pivot systems (Ch.8, held).
//!   `jpcgt`  A Complete Guide to Technical Trading Tactics (Wiley 2004): generic moving-average rules, P3T, tactics (held).
//! Reading: claude-continuation-20261020/authors/person/agents/. The Commodity Trader's Almanac (with Hirsch) is `hcta`.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const REVIEWED: &[&str] = &["jpcppt_ppma_close", "jpcppt_comas"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

macro_rules! note { ($s:expr) => { concat!($s, " [Declared conventions, not from the book] A run starts flat and the first cross opens the first trade; history before the window builds the averages; an open trade is marked at the window's last close; one unit, no commissions. Index, ETF and stock runs (US and India) on daily bars are a declared application: the book shows forex, futures and stocks on 5-minute to daily charts. Cash-index shorts are hypothetical.") }; }

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    let live = |id, name, group, book, rules, notes| StrategyDef { id, name, group, session: 0, dir: "Long/Short", status: Live, needs_volume: false,
        needs_vix: false, needs_series: &[], book, rules, notes, claim: None };
    let held = |id, name, group, book, st, rules| StrategyDef { id, name, group, session: 0, dir: "Long/Short", status: st, needs_volume: false,
        needs_vix: false, needs_series: &[], book, rules, notes: "Held after the 2026-10-24 reading (authors/person/agents).", claim: None };
    vec![
        live("jpcppt_ppma_close", "Close above / below the three-period pivot-point average: long or short, stop and reverse", "Ch.6 · Pivot point moving average (p.161-168)", "jpcppt",
            &["F|\"the pivot point calculation provides the mean (average) for the session's trading range, or high, low, and close: (H + L + C)/3\"; \"Market Direction = (Pivot + Pivot + Pivot)/3 ... a three-period pivot point moving average\" (p.161-162)",
              "E|\"Closing Price > M/A = Go long or exit shorts\"; \"Closing Price < M/A = Go short or exit longs\" (p.167)",
              "X|\"there is no need to wait for the value of the moving average to start rising or falling ... A close above the moving average will trigger a long position, and a close below the moving average will trigger a short position\" (p.167-168; same text in Forex Conquered p.94-95)"],
            note!("The average is the simple 3-day mean of the daily pivot (H+L+C)/3 including today's bar, as plotted on the book's charts; p.prior = 1 uses the three previous days instead. Signals and fills at the close. A close equal to the average keeps the position. The p.167 bullets also ask for a close above the prior bar's high, a 'flat or higher-sloping' average and a stop 'below the lowest low point'; those soft conditions are not applied (the book's own summary and p.168 say a close beyond the average triggers).")),
        live("jpcppt_comas", "COMAS: the daily pivot point crossing its three-period average, stop and reverse", "Ch.6 · Conditional Optimized Moving Average System (p.168-174)", "jpcppt",
            &["F|\"the one-period pivot point with the three-period moving average of the pivot point\" (p.169)",
              "E|\"as the pivot point crosses above the three-period moving averages, it alerts us that the internal market condition is changing to bullish\" (p.169); \"triggered a sell signal when the pivot point crossed beneath the three-period moving average\" (p.173)",
              "X|\"A crossover provides both the entry and the exit signals\" (p.168); \"When a crossover occurs, traders should liquidate the entire position\" (p.171)"],
            note!("Pivot = (H+L+C)/3 of each day; the average is the simple 3-day mean of the pivot including today. Signals and fills at the close; equal values are no cross. The book's further 'set of rules or conditions' (closes above both averages, higher highs, candle patterns, pivot support targets) is not stated as a rule and is not applied; the half-position exit when the averages separate 'too far' is not run.")),
        held("jpcppt_hcd", "High close doji / low close doji / jackhammer at pivot support or resistance", "Ch.7-8 (p.187-256)", "jpcppt",
            NotTestable("needs an 'extended trend' and 'oversold' context and an exit 'near a key pivot point resistance number'; the doji is defined as close = open (p.280), which rarely happens on daily index data"), &["E|Buy a close above the doji's high near pivot support; stop-close-only below the doji low (p.216)"]),
        held("jpcppt_ideas", "Back-testing ideas: three up candles after five down, 3/5-period pivot-average cross near S-1/R-1", "Ch.11 (p.279-290)", "jpcppt",
            NotTestable("offered as ideas to program ('you can program your own system'), with 'say' parameters and no fixed exit"), &["E|Ideas only"]),
        held("jpfxc_pivot_stoch", "15-minute forex pivot systems: stochastics or MACD signal within 10 pips of R1-R3 / S1-S3, 100-pip stop, 40-pip target", "Forex Conquered Ch.8 (p.199-216)", "jpfxc",
            NeedsData("15-minute spot-forex bars and pip-denominated stops and targets; the vault has no forex data and a pip-to-index scale would be our choice"), &["E|Sell when %K/%D cross back below 72 within 10 pips of R1-R3 (code p.206-208)"]),
        held("jpfxc_defcon3", "Defcon III multidimensional system", "Forex Conquered Ch.8 (p.216-220)", "jpfxc",
            NotTestable("proprietary Genesis library; rules not disclosed"), &["E|Proprietary"]),
        held("jpcgt_tactics", "Moving-average rules, P3T pivot technique, Friday 10:30 rule, scale trading", "Complete Guide Ch.6-8, 12", "jpcgt",
            NotTestable("generic rules without periods ('favorites are 3, 9 and 18'), judgement targets and a 'casual observation' intraday rule with 'trending up' undefined"), &["E|Judgement"]),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "jpcppt_ppma_close" => ("jpcppt_ppma_close", "Person pivot-average close", "3-day pivot average"),
        "jpcppt_comas" => ("jpcppt_comas", "Person COMAS crossover", "pivot / 3-day pivot average"),
        _ if id.starts_with("jpcppt_") || id.starts_with("jpfxc_") || id.starts_with("jpcgt_") => ("jp_nt", "Person (held)", "-"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(match id {
        "jpcppt_ppma_close" | "jpcppt_comas" => vec![p("n", "Pivot average days (book: 3)", 3., 2., 50., 1.), p("prior", "Average of the previous days only (0 = includes today, book chart)", 0., 0., 1., 1.)],
        _ => return None,
    })
}

fn trade(d: f64, i: usize, px: f64) -> Trade {
    Trade { dir: d, legs: vec![Leg { i, px, w: 1. }], exit_i: i, exit_px: px, reason: String::new(), entry_kind: "close", signal_i: i, open: false, exits: vec![] }
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from > to || to >= x.r.len() { return None; }
    let n = p.get("n").copied().unwrap_or(3.).round().max(1.) as usize;
    let prior = p.get("prior").copied().unwrap_or(0.) >= 0.5;
    let c = &x.c;
    let pp: Vec<f64> = (0..c.len()).map(|i| (x.h[i] + x.l[i] + c[i]) / 3.).collect();
    // average of n pivots ending today (or ending yesterday with prior)
    let ma: Vec<f64> = (0..c.len()).map(|i| { let e = if prior { i } else { i + 1 }; if e < n { f64::NAN } else { pp[e - n..e].iter().sum::<f64>() / n as f64 } }).collect();
    let line: &[f64] = if id == "jpcppt_comas" { &pp } else { c };
    // +1 / -1 by line above / below the average, kept on ties; None until the first difference
    let mut st: Vec<Option<i8>> = vec![None; c.len()];
    let mut s = None;
    for i in 0..c.len() { if ma[i].is_finite() && line[i] != ma[i] { s = Some(if line[i] > ma[i] { 1 } else { -1 }); } st[i] = s; }
    let why = if id == "jpcppt_comas" { "pivot crossed its average" } else { "close crossed the pivot average" };
    let mut out = vec![];
    let mut cur: Option<Trade> = None;
    for i in from.max(1)..=to {
        let (Some(a), Some(b)) = (st[i], st[i - 1]) else { continue };
        if a == b { continue; }
        if let Some(mut t) = cur.take() { t.exit_i = i; t.exit_px = c[i]; t.signal_i = i; t.reason = why.into(); out.push(t); }
        cur = Some(trade(a as f64, i, c[i]));
    }
    if let Some(mut t) = cur { t.exit_i = to; t.exit_px = c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true; out.push(t); }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn catalogue_admits_exactly_the_reviewed_ids() {
        let live: Vec<_> = catalogue().into_iter().filter(|d| d.status == Status::Live).map(|d| d.id).collect();
        assert_eq!(live, REVIEWED);
    }
}
