//! Richard D. Wyckoff: "The Day Trader's Bible" (1919/2003; book id `rwdtb`), "How I Trade and Invest in Stocks and Bonds" (1922; `rwhit`),
//! "Stock Market Technique No. 1" (1933; `rwsmt`) and the Wyckoff course units (`rwcrs`). Reading: claude-continuation-20261020/authors/wyckoff/agents/.
//! Catalogue only: tape reading, accumulation and distribution and the trader's judgement of supply and demand.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub fn catalogue() -> Vec<StrategyDef> {
    let nt = |id, name, group, book, why, rules| StrategyDef { id, name, group, session: 0, dir: "Long/Short", status: Status::NotTestable(why),
        needs_volume: false, needs_vix: false, needs_series: &[], book, rules, notes: "Held after the 2026-10-24 reading (authors/wyckoff/agents).", claim: None };
    vec![
        nt("rwdtb_tape", "Tape reading (day trader's bible)", "The Day Trader's Bible", "rwdtb",
           "reading the ticker's price and volume flow; 'Each indication is to be judged not so much by rule as according to' the conditions (Ch.XII); needs tick data",
           &[]),
        nt("rwhit_rules", "Rules I follow in trading and investing", "How I Trade and Invest in Stocks and Bonds · Ch.VIII", "rwhit",
           "principles (cut losses, follow the leaders, judge the market's position) without objective entry or exit",
           &[]),
        nt("rwcrs_method", "Wyckoff method: accumulation/distribution, springs and upthrusts, point-and-figure counts", "Stock Market Technique No.1; course units", "rwsmt",
           "phases and tests are identified on the chart by the trader; the course teaches judgement, not a mechanical rule",
           &[]),
    ]
}

fn mine(id: &str) -> bool { id.starts_with("rwcrs_") || id.starts_with("rwdtb_") || id.starts_with("rwhit_") }

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    if mine(id) { Some(("rdwyck_nt", "Wyckoff method (held)", "-")) } else { None }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> { if mine(id) { Some(vec![]) } else { None } }

pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }
