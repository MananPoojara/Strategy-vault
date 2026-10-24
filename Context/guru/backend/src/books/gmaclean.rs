//! George A. MacLean: "Fibonacci and Gann Applications in Financial Markets" (Wiley 2005; book id `gmfg`). Reading:
//! claude-continuation-20261020/authors/maclean/agents/. Retracements, fanlines, timelines and Gann angles are drawn from swing points the
//! analyst chooses ("there are no hard and fast rules as to the application", p.27); held cards only.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub fn catalogue() -> Vec<StrategyDef> {
    let nt = |id, name, group, book, why, rules| StrategyDef { id, name, group, session: 0, dir: "Long/Short", status: Status::NotTestable(why),
        needs_volume: false, needs_vix: false, needs_series: &[], book, rules, notes: "Held after the 2026-10-25 reading (authors/maclean/agents).", claim: None };
    vec![
        nt("gmfg_fib_retrace", "Fibonacci retracements and extensions", "Ch.2-5 Retracements and extensions", "gmfg",
           "the origin and the significant high or low are chosen by the analyst ('There are no hard and fast rules as to the application', p.27) and entries, stops and targets are judged per chart",
           &["F|23.6/38.2/50/61.8/100/161.8% levels between the origin and the significant extreme of a move (ch.2-3)"]),
        nt("gmfg_fans_timelines", "Fibonacci fanlines and timelines", "Ch.6-7 Fanlines and timelines", "gmfg",
           "fanlines and time projections start from analyst-chosen extremes; 'no hard and fast rules as to what the optimal time after the formation' (p.107)",
           &[]),
        nt("gmfg_gann_swing", "Gann swing chart breakouts", "Ch.9 Gann (p.147)", "gmfg",
           "'once a swing chart base has been formed' is a judgement, and outside days need to know whether the high or the low came first (not in daily bars)",
           &["E|\"A strong buy signal is given once a swing chart base has been formed and the price action breaks out of the range\" (p.147)", "X|\"stop loss levels ... are set at the last change of direction in TIL level\" (p.147)"]),
        nt("gmfg_gann_angles", "Gann angles, price-time squares and synthetic ratios", "Ch.9-10 Gann and synthetic ratios", "gmfg",
           "angles are drawn from chosen pivots with a price-per-day scale set by the analyst (32nds, 360-day year); synthetic ratios are projections without entries or exits",
           &[]),
    ]
}

fn mine(id: &str) -> bool { id.starts_with("gmfg_") }

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    if mine(id) { Some(("gmaclean_nt", "MacLean Fibonacci and Gann (held)", "-")) } else { None }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> { if mine(id) { Some(vec![]) } else { None } }

pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }
