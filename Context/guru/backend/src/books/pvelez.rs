//! Oliver Velez & Greg Capra: "Tools & Tactics for the Master Day Trader" (McGraw-Hill 2000; catalogue book `p_tools_tactics`).
//! Reading: claude-continuation-20261020/authors/velez/agents/. Pristine day- and swing-trading tools: every set-up is a menu of
//! characteristics ('the more, the better'), entries are mostly intraday and stops/exits are personal choices; held cards only.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub fn catalogue() -> Vec<StrategyDef> {
    let nt = |id, name, group, book, why, rules| StrategyDef { id, name, group, session: 0, dir: "Long/Short", status: Status::NotTestable(why),
        needs_volume: false, needs_vix: false, needs_series: &[], book, rules, notes: "Held after the 2026-10-25 reading (authors/velez/agents).", claim: None };
    vec![
        nt("pvtt_3to5_drop", "3 to 5 bar drop buy (Pristine Buy Set-up)", "Tools & Tactics · Charting tool #10 (p.237-247)", "p_tools_tactics",
           "a down bar needs its open 'at or near the high' and its close 'at or near the low' (no tolerance); 'stocks in strong up trends' is judged; the initial stop is one of two 'personal choice' methods and the exit is a set of 'sellable events' and targets",
           &["E|\"After a 3 to 5 bar drop, the master trader buys the very next time the stock trades above a prior bar's high\" (p.240)", "X|Prior-bar or current-bar stop 1/16 to 1/8 below the low, 'totally a matter of personal choice' (p.306-307)"]),
        nt("pvtt_intraday_entries", "Key Buy, 30-minute buy and late-day breakout entries", "Tools & Tactics · Ch.14 (p.293-303)", "p_tools_tactics",
           "intraday entries on 5- and 15-minute charts with Level II and reversal-time judgement; the exits are the Ch.15-16 menus",
           &[]),
        nt("pvtt_market_tools", "Market timing with S&P futures, TICK, TRIN, new lows and the Mighty 5", "Tools & Tactics · Ch.10 (p.169-190)", "p_tools_tactics",
           "intraday TICK and futures readings interpreted by the trader; no entry or exit numbers",
           &[]),
        nt("pvtt_chart_tools", "Narrow-range and reversal bars, tails, gaps, retracements, climactic volume, moving averages", "Tools & Tactics · Ch.11 (p.193-236)", "p_tools_tactics",
           "charting events described without complete entry and exit rules ('at or near', 'strong trend')",
           &[]),
        nt("pvtt_trade_management", "Initial, break-even, trailing and time stops; incremental sales", "Tools & Tactics · Ch.15-16 (p.305-325)", "p_tools_tactics",
           "stop placement is a choice between methods and the break-even move waits until 'the stock advances enough'; exits follow 'sellable events'",
           &[]),
    ]
}

fn mine(id: &str) -> bool { id.starts_with("pvtt_") }

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    if mine(id) { Some(("pvelez_nt", "Velez & Capra, Tools & Tactics (held)", "-")) } else { None }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> { if mine(id) { Some(vec![]) } else { None } }

pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }
