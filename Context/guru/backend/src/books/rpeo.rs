//! Robert Pardo, The Evaluation and Optimization of Trading Strategies (2nd ed., Wiley, 2008).
//! Mostly method; only the fully specified example systems are coded. Simulators shared with rpdt.rs (1992 edition).
//! Source + verification: ~/.local/share/strategy-vault/library/rpeo/.
use super::rpdt::{check, ma_cross, range_sar};
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "rpeo";
/// Admitted ids (authors/pardo/admission/SOURCE-CONTRACT.md).
pub const REVIEWED: &[&str] = &["rpeo_eots_ma", "rpeo_range_bo"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

fn def(id: &'static str, name: &'static str, group: &'static str, status: Status, rules: &'static [&'static str], notes: &'static str) -> StrategyDef {
    StrategyDef { id, name, group, session: 0, dir: "Long/Short", status, needs_volume: false, needs_vix: false,
        needs_series: &[], book: BOOK, rules, notes, claim: None }
}

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    vec![
        def("rpeo_eots_ma", "EOTS-MA two moving-average crossover (10/30, always in)", "Ch 7 · Make a vague idea precise", Live, &[
            "F|\"MA1 = [C(t) + C(t+1) + ... + C(t+X-1)]/X\"; \"MA2 = [C(t) + ... + C(t+Y-1)]/Y\"; \"Y is never less than 2 times X\" (p.151); EasyLanguage \"Input: MA1 Period(10), MA2 Period(30)\", average(close, ...) (p.153)",
            "E|\"if MA1 Today > MA2 Today then begin if MA1 Yesterday < MA2 Yesterday then begin buy(\\\"LOen\\\") next bar on open\" (p.153)",
            "E|\"if MA1 Today < MA2 Today then begin if MA1 Yesterday > MA2 Yesterday then begin sell(\\\"SHen\\\") next bar on open\" (p.153)",
            "X|\"an order is set to buy and reverse the current short position with the open of the next bar ... sell and reverse the current long position\" (p.155); rules 8 and 10 (p.151-152) - stop-and-reverse",
        ], "Defaults are the EasyLanguage inputs 10/30 (p.153). The multimarket test (Table 8.6, p.174, 'MA 5x20 R1Test') used 5/20: run with p.fast=5&p.slow=20 to compare the S&P column (1997-2006 two-year segments, see README). Same rule as rpdt_ma_xover (1992, default 5/20). Book traded S&P futures; run on the index with % returns, hypothetical index shorts."),
        def("rpeo_range_bo", "125% / 145% of yesterday's range stop-and-reverse", "Ch 7 · Formulate the trading strategy", Live, &[
            "E|\"Go long on a stop when the market rallies to a price level that is equal to or more than yesterday's close plus 125 percent of yesterday's daily range.\" (p.146)",
            "E|\"Go short on a stop when the market falls to a price level that is equal to or less than yesterday's close minus 145 percent of yesterday's daily range.\" (p.146)",
            "X|\"Exit and reverse positions on opposite entry signals.\" (p.146)",
        ], "Stops on daily bars: touching the level fills at the level ('equal to or more than'); a gap beyond it fills at the open. Both levels crossed on one bar: TradeStation intrabar path (open nearer the high: O-H-L-C, else O-L-H-C) - book silent (class B). 1992 edition version (125% both sides, 'more than') is rpdt_range_bo."),
        def("rpeo_tts", "Turtle Trading Strategy (simplified, x-day / y-day)", "Ch 2 · The systematic trading edge", NotTestable("not testable: the book gives the lengths only as variables x and y ('goes long when an x-day high has been penetrated ... exits ... from a shorter y-day high or low'); no values. Turtle systems with their own numbers are already in the vault (covel / Faith modules)."), &[
            "E|\"TTS goes long when an x-day high has been penetrated and goes short when an x-day low has been broken.\" (p.20)",
            "X|\"It exits positions on opposite signals from a shorter y-day high or low.\" (p.20)",
        ], "Illustration of a simple systematic strategy; no parameter values."),
        def("rpeo_proprietary", "XT06ct / XTct / RSI CT example systems", "Ch 8, 11 · Testing and walk-forward examples", NotTestable("not testable: Pardo's own systems used only to show reports (trade list Table 8.2 p.162, Tables 8.7-8.8 p.175-176, walk-forward of RSI CT p.250-262); their rules are never given (only 'XTct (38, 38, 3)' parameter values)"), &[
            "E|rules not printed in the book",
        ], "Shown for reports/walk-forward illustration only."),
        def("rpeo_entry_examples", "Example entry rules (3-day high/low, 5-day high / 5x20 MA, RSI < 20, ...)", "Ch 5 · Elements of strategy design", NotTestable("not testable: illustrations of entry rules; exits, reversal and order types not stated (same examples as the 1992 edition, see rpdt_3day_hl, rpdt_5dh_ma_asym, rpdt_entry_examples). The 5/20 crossover example with its exit is the EOTS-MA rule (rpeo_eots_ma with 5/20)."), &[
            "E|\"a buy signal occurs when price rises through a three-day high and a sell signal occurs when price breaks through a three-day low\"; \"a buy signal occurs when a five-day high is broken and a sell signal occurs when a five-day moving average falls below a 20-day moving average\" (p.75)",
            "E|\"The Relative Strength Index closes below a reading of 20\"; \"The daily close rises by 1 percent and the weekly close rises by 1 percent\"; \"Today's close is higher than yesterday's close plus 50 percent of the daily range\"; \"Today's close is higher than the previous three closes\" (p.77)",
        ], "Illustrations only."),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "rpeo_eots_ma" => ("rp_ma", "Pardo two moving-average crossover", "2008 · EOTS-MA 10/30 next open"),
        "rpeo_range_bo" => ("rp_range", "Pardo yesterday's-range breakout", "2008 · 125% up / 145% down"),
        "rpeo_tts" | "rpeo_proprietary" | "rpeo_entry_examples" => ("rp_examples", "Pardo illustrative rules", "not testable"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    match id {
        "rpeo_eots_ma" => Some(vec![p("fast", "MA1 Period (closes)", 10., 1., 20., 1.), p("slow", "MA2 Period (closes, >= 2 x MA1; Table 8.6 used 5x20)", 30., 2., 120., 5.)]),
        "rpeo_range_bo" => Some(vec![p("up", "Buy stop: multiple of yesterday's range above the close", 1.25, 0.25, 3., 0.05), p("dn", "Sell stop: multiple of yesterday's range below the close", 1.45, 0.25, 3., 0.05)]),
        _ => None,
    }
}

pub fn validate(id: &str, p: &Params) -> Option<String> {
    let s = params(id)?;
    if id == "rpeo_eots_ma" { check(&s, p, &["fast", "slow"], true) } else { check(&s, p, &[], false) }
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let s = params(id)?;
    if validate(id, p).is_some() || from >= to || to >= x.r.len() { return None }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| s.iter().find(|d| d.key == k).unwrap().def);
    Some(match id {
        "rpeo_eots_ma" => ma_cross(x, from, to, g("fast") as usize, g("slow") as usize),
        "rpeo_range_bo" => range_sar(x, from, to, g("up"), g("dn"), false),
        _ => return None,
    })
}
