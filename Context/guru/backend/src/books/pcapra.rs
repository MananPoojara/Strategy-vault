//! Greg Capra: "Intra-Day Trading Techniques" (Pristine seminar manual, 2001; catalogue book `p_intraday_capra`).
//! Reading: claude-continuation-20261020/authors/capra/. 66 slides on 5/15/60-minute charts: candle concepts, 20/200-period averages,
//! retracements, TICK/TRIN; the Pristine Buy/Sell Setup has no stop, exit or numeric definition; held card only.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub fn catalogue() -> Vec<StrategyDef> {
    let nt = |id, name, group, book, why, rules| StrategyDef { id, name, group, session: 0, dir: "Long/Short", status: Status::NotTestable(why),
        needs_volume: false, needs_vix: false, needs_series: &[], book, rules, notes: "Held after the 2026-10-25 reading (authors/capra).", claim: None };
    vec![
        nt("pcit_buy_sell_setup", "Pristine Buy & Sell Setup on 5/15/60-minute charts", "Intra-Day Trading Techniques · Pristine Buy & Sell Setup", "p_intraday_capra",
           "intraday (5-, 15- and 60-minute bars) and discretionary: the set-up bars only 'open in the area of the prior bar's close', the reversal candle, trend side, retracement depth and TICK/TRIN readings are judged, and no stop, target or exit is given",
           &["E|\"Each new bar opens in the area of the prior bar's close and also closes above/below its opening price until a reversal of that occurs. The setup is complete after a reversal candle has formed.\"", "F|\"Long positions are favored when prices are Above their 20-MA on a 60-Min. chart. Entry points are found on 5- or 15-Min. charts.\""]),
    ]
}

fn mine(id: &str) -> bool { id.starts_with("pcit_") }

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    if mine(id) { Some(("pcapra_nt", "Capra, Intra-Day Trading Techniques (held)", "-")) } else { None }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> { if mine(id) { Some(vec![]) } else { None } }

pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }
