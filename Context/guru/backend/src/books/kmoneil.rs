//! Chris Kacher and Gil Morales: "Trade Like an O'Neil Disciple" (Wiley 2010; book id `kmtld`), "In the Trading Cockpit with the O'Neil
//! Disciples" (Wiley 2013; `kmitc`) and "Short-Selling with the O'Neil Disciples" (Wiley 2015; `kmssd`). Reading:
//! claude-continuation-20261020/authors/kacher/agents/. Catalogue only: the buy points are taken in fundamentally strong leading stocks
//! with 'constructive' bases judged on the chart, the sell guide (10- or 50-day line) depends on the stock's 'character', and the
//! market timing model is not disclosed.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub fn catalogue() -> Vec<StrategyDef> {
    let nt = |id, name, group, book, why, rules| StrategyDef { id, name, group, session: 0, dir: "Long/Short", status: Status::NotTestable(why),
        needs_volume: false, needs_vix: false, needs_series: &[], book, rules, notes: "Held after the 2026-10-24 reading (authors/kacher/agents).", claim: None };
    vec![
        nt("kmtld_pocket_pivot", "Pocket pivot buy point", "Trade Like an O'Neil Disciple · Ch.5 (p.127-163)", "kmtld",
           "the volume signature is exact (up-volume above the largest down-volume of the prior 10 days, price above the 50-day line) but the trade is only for 'fundamentally strong stocks with constructive basing action', the base quality is judged on the chart and improper pocket pivots are excluded by judgement; the exit (10-day or 50-day line) depends on the stock's 'character'",
           &["E|\"On the pocket pivot you want to see up-volume equal to or greater than the largest down-volume day over the prior 10 days\" (p.133)", "F|\"pocket pivots should only be bought when they occur above the 50-day moving average\" (p.132)"]),
        nt("kmitc_buyable_gap_up", "Buyable gap-up", "In the Trading Cockpit · Introduction, Ch.2-4", "kmitc",
           "the size test is 'eyeballed' in practice (the 0.75 x 40-day ATR test is called a former calculation), the gap must come in a leading stock out of a base, and the stop has 'porosity' of 2-3% chosen by the trader",
           &["E|\"at least 0.75 times the 40-day average true range ... in practice it is enough to be able to 'eyeball' a gap-up move\"; volume \"at least 1.5 times, or 150 percent of the 50-day moving average of daily trading volume\" (p.xxii)", "X|stop at the intraday low of the gap-up day, allowing 2-3 percent porosity (p.xxiv)"]),
        nt("kmitc_seven_week", "Seven-week rule and moving-average violations (sell guides)", "In the Trading Cockpit · Introduction, Ch.4", "kmitc",
           "an exit rule only; the entries it serves are the judgement buy points above, and a violation's 'next few days' window is not fixed",
           &["X|\"whether the stock is able to hold above its 10-day moving average for at least seven weeks following a buy point without ever violating the moving average\" (p.xxvi)", "X|violation = a close below the line, then \"on next day or over the next few days\" a trade below that day's intraday low (p.101)"]),
        nt("kmtld_mdm", "Dr. K's Market Direction Model", "Trade Like an O'Neil Disciple · Ch.7 (p.226-259)", "kmtld",
           "the model's rules are not disclosed and depend on the 'quality' of NASDAQ/S&P base formations judged by experience ('a programmer would have a difficult if not impossible time programming the model', p.244)",
           &[]),
        nt("kmssd_setups", "Short-sale set-ups (head and shoulders, late-stage failed base, punchbowl of death)", "Short-Selling with the O'Neil Disciples · Ch.3-4", "kmssd",
           "chart patterns in former leading stocks identified by eye; entries 'into rallies' and covers are judgement",
           &["E|short the late-stage failed base or the right shoulder of a head-and-shoulders top in a former leader (Ch.3)"]),
        nt("kmtld_rules", "Trading commandments, case studies, psychology", "Trade Like an O'Neil Disciple · Ch.1-4, 8-10", "kmtld",
           "trading history, principles and psychology; no separate rule",
           &[]),
    ]
}

fn mine(id: &str) -> bool { id.starts_with("kmitc_") || id.starts_with("kmssd_") || id.starts_with("kmtld_") }

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    if mine(id) { Some(("kmoneil_nt", "Kacher & Morales (held)", "-")) } else { None }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> { if mine(id) { Some(vec![]) } else { None } }

pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }
