//! Nicolas Darvas: "How I Made $2,000,000 in the Stock Market" (1960; book id `darvas2m`) and "You Can Still Make It in the Market"
//! (1977; `dvycs`). Reading: claude-continuation-20261020/authors/darvas/agents/ (both scans OCR'd in full). Catalogue only. The modern
//! Darvas-box runs in the vault are Daryl Guppy's (dgdwb).
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub fn catalogue() -> Vec<StrategyDef> {
    let nt = |id, name, group, book, why, rules| StrategyDef { id, name, group, session: 0, dir: "Long/Short", status: Status::NotTestable(why),
        needs_volume: false, needs_vix: false, needs_series: &[], book, rules, notes: "Held after the 2026-10-24 reading (authors/darvas/agents).", claim: None };
    vec![
        nt("darvas2m_box", "Box theory (How I Made $2,000,000)", "Ch.4 Developing the Box Theory (p.47-59)", "darvas2m",
           "box limits are decided by the trader ('What I had to decide was the range of the box'; 'I did not find any fixed rule as to how this takes place. It just has to be observed', p.52); stocks were chosen by volume behaviour and later by earnings (techno-fundamental); the trailing stop distance is not fixed",
           &["E|buy on an automatic on-stop order when the stock rises into a new higher box (p.54)", "X|\"I placed my stop-losses one fraction below the ceiling through which the stock broke through\" (letters, p.188)"]),
        nt("dvycs_darcard", "DAR-CARD (three-day box top and bottom, 10% first stop, 5% danger zone)", "You Can Still Make It in the Market · How I draw / use a DAR-CARD (p.9-11)", "dvycs",
           "the box rules are exact (top = a high not surpassed for three or more days, bottom = a low not penetrated for three or more days, 5% danger zone) but a BUY needs the stock to be in its 'topmost box' and the method is for a 'rapidly rising stock' Darvas selected; neither is defined (user option: topmost = above the prior three years' high shown on the card)",
           &["F|\"When the price of a rapidly rising stock reaches a resistance point which it does not surpass for three or more consecutive days, that point represents the top of the box\"", "E|\"If the price of the stock moves above the top of this topmost box the stock becomes a BUY. A 10 percent stop-loss should be set on the first breakout\"", "X|\"Having formed a new higher box, if the price falls below the bottom into the shaded area of this box the stock is a SELL\" (shaded = 5% below the bottom)"]),
        nt("darvas2m_selection", "Techno-fundamental stock selection", "How I Made $2,000,000 · Ch.6-10", "darvas2m",
           "growth industries, earnings improvement and unusual volume are judged by the author; no screen is given",
           &[]),
    ]
}

fn mine(id: &str) -> bool { id.starts_with("darvas2m_") || id.starts_with("dvycs_") }

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    if mine(id) { Some(("darvas_nt", "Darvas box (held)", "-")) } else { None }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> { if mine(id) { Some(vec![]) } else { None } }

pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }
