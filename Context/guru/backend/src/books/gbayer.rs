//! George Bayer - astrological market timing (1937-1950s). Book ids `gbywol` (George Wollsten, Expert Stock and Grain Trader),
//! `gbygn` (Gold Nuggets), `gbytd` (Hand-Book of Trend Determination), `gbytf` (Time Factors in the Stock Market).
//! Reading: claude-continuation-20261020/authors/bayer/agents/. Catalogue only: every rule is a planetary configuration that marks a
//! change of trend; there is no entry price, stop or exit, and we hold no ephemeris.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, book, why, rules| StrategyDef { id, name, group, session: 0, dir: "Long/Short", status: Status::NotTestable(why), needs_volume: false,
        needs_vix: false, needs_series: &[], book, rules, notes: "Read 2026-10-24 (authors/bayer/agents).", claim: None };
    vec![
        d("gbytd_rules", "48 wheat and 11 hides/stocks rules (planetary trend changes)", "Hand-Book of Trend Determination · Parts II-IV", "gbytd",
          "needs planetary positions (an ephemeris) and gives no entry price, stop or exit; the trader waits 'a few days to see whether or not the market changes as expected'",
          &["F|A change of trend is due when a listed planetary configuration occurs (e.g. Mars-Mercury speed differential, Mercury at 14° Aquarius)"]),
        d("gbywol_timing", "Half-way points, ellipses and 17 1/2-point swing units", "George Wollsten · p.215-231", "gbywol",
          "a novel: trades are story events timed by planets and hand-laid ellipses; no defined entry or exit",
          &["F|Turns expected at Mercury crossings of Saturn and Jupiter and their half-way points"]),
        d("gbygn_essay", "Planetary price-change laws (essay)", "Gold Nuggets", "gbygn",
          "an essay without a trading rule",
          &["F|Prices follow the planets' motion around the Zodiac"]),
        d("gbytf_theory", "Time factors from planetary motion", "Time Factors in the Stock Market", "gbytf",
          "astrological theory without a trading rule",
          &["F|Swing direction and distance from planetary cycles"]),
    ]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id.starts_with("gbywol_") || id.starts_with("gbygn_") || id.starts_with("gbytd_") || id.starts_with("gbytf_")).then_some(("gby_nt", "Bayer astrological timing (held)", "-"))
}
pub fn params(_id: &str) -> Option<Vec<ParamDef>> { None }
pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }
