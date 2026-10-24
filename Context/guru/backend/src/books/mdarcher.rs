//! Michael Duane Archer - forex books. Book ids `mdagsc` (Getting Started in Currency Trading, 3rd ed. and the earlier edition with Bickford),
//! `mdafts` (Getting Started in Forex Trading Strategies), `mdafcc` (The Forex Chartist Companion). Reading: claude-continuation-20261020/
//! authors/archer/agents/. Catalogue only: the Goodman Swing Count set-ups are mapped by eye ('eyeball the chart'), and the toolbox
//! rules lack settings or exits.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, book, why, rules| StrategyDef { id, name, group, session: 0, dir: "Long/Short", status: Status::NotTestable(why), needs_volume: false,
        needs_vix: false, needs_series: &[], book, rules, notes: "Read 2026-10-24 (authors/archer/agents).", claim: None };
    vec![
        d("mdafts_gscs", "Goodman Swing Count return and double-intersection trades (codex method)", "Getting Started in Forex Trading Strategies · Ch.9-13", "mdafts",
          "waves are visualised and candidates chosen by 'the four eyeball characteristics' (p.132); stops and objectives are set per trade",
          &["E|Enter on a Goodman 'return' (50% of a prior swing) or a double intersection on the trade chart, confirmed on the larger chart"]),
        d("mdagsc_nofri", "Nofri congestion phase and the MA/oscillator battery", "Getting Started in Currency Trading · Ch.12", "mdagsc",
          "the 'trading unit' is not defined and no stop or exit is given; the battery has no MA or oscillator settings",
          &["E|Two units one way, two inside them: the fifth unit is expected to follow the first two (p.149-150)"]),
        d("mdafcc_3lb", "Three-line break turnaround after three lines (Nison, as reported)", "The Forex Chartist Companion · Ch.33", "mdafcc",
          "a report of Nison's method with a judgement filter ('avoid trading in trendless markets', p.226); the line-break reversal itself is already tested as Pring's mpcse_3lb",
          &["E|Buy when a white line emerges after three adjacent black lines; sell when a black line appears after three white lines (p.226)"]),
        d("mdafcc_pnf", "Point-and-figure breakouts, kagi and arbitrage oscillator", "The Forex Chartist Companion · Ch.5, 12-18, 33", "mdafcc",
          "box sizes and kagi reversal amounts are forex pips (no index rule) and no exits are given; the arbitrage oscillator needs tick forex quotes",
          &["E|Buy a double or triple top breakout; buy when the kagi line turns thick"]),
    ]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id.starts_with("mdagsc_") || id.starts_with("mdafts_") || id.starts_with("mdafcc_")).then_some(("mda_nt", "Archer forex methods (held)", "-"))
}
pub fn params(_id: &str) -> Option<Vec<ParamDef>> { None }
pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }
