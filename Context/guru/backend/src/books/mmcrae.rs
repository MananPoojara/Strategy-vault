//! Mark McRae: "Sure-Fire Forex Trading" (surefire-forex-trading.com e-book; book id `msfft`). Reading:
//! claude-continuation-20261020/authors/mcrae/agents/. 4-hour / 30-minute / 5-minute forex method: Fibonacci entries from an eyeballed
//! point A, oscillator extremes judged on the chart; held cards only. McRae's Sure Fire Trading Plans are carded in books/msftp.rs.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub fn catalogue() -> Vec<StrategyDef> {
    let nt = |id, name, group, book, why, rules| StrategyDef { id, name, group, session: 0, dir: "Long/Short", status: Status::NotTestable(why),
        needs_volume: false, needs_vix: false, needs_series: &[], book, rules, notes: "Held after the 2026-10-25 reading (authors/mcrae/agents).", claim: None };
    vec![
        nt("msfft_ema_fib", "89/144 EMA bands, swing-point trend indicator and 38.2% Fibonacci entry", "Sure-Fire Forex Trading · Trading rules (p.44-95)", "msfft",
           "intraday forex (4-hour, 30-minute and 5-minute charts); point A of the Fibonacci measurement is eyeballed ('I prefer just to eyeball a low or high point', p.78) and trades are selected 'as determined by market conditions' (p.75)",
           &["F|\"when both of the shorter 89's cross above both 144's then the trend is up\" (EMA of highs and lows, p.50)", "E|\"We only enter on a 38.2% retracement. You only place your stop loss under the 61.8% retracement\" (p.75)"]),
        nt("msfft_oscillators", "MACD 12-26-9, Williams %R 89 and RSI 13 extremes with the EMA bands", "Sure-Fire Forex Trading · Advanced techniques (p.96-110)", "msfft",
           "the extremes are judged by eye ('forget these levels and just concentrate on what you see', p.97) on intraday charts",
           &[]),
    ]
}

fn mine(id: &str) -> bool { id.starts_with("msfft_") }

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    if mine(id) { Some(("mmcrae_nt", "McRae, Sure-Fire Forex Trading (held)", "-")) } else { None }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> { if mine(id) { Some(vec![]) } else { None } }

pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }
