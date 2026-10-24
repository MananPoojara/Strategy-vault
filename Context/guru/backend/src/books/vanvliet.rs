//! Benjamin Van Vliet - programming and process books. Book ids `vvbats` (Building Automated Trading Systems), `vvmfm` (Modeling
//! Financial Markets, with Hendry), `vvqmm` (Quality Money Management, with Kumiega). Reading: claude-continuation-20261020/authors/vanvliet/agents/.
//! Catalogue only: the sample systems run on futures ticks or quotes, two-legged spreads or fundamental rankings; the author says of
//! his examples that 'in the real world this trading system will not make money' (BATS p.267, p.291).
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, book, why, rules| StrategyDef { id, name, group, session: 0, dir: "Long/Short", status: Status::NotTestable(why), needs_volume: false,
        needs_vix: false, needs_series: &[], book, rules, notes: "Read 2026-10-24 (authors/vanvliet/agents).", claim: None };
    vec![
        d("vvbats_tickma", "Tick moving-average crossover with target and stop (NQ e-mini)", "Building Automated Trading Systems · Ch.44", "vvbats",
          "needs tick-by-tick Nasdaq-100 e-mini trades; the target and stop tick counts are user inputs with no values; the author says it will not make money",
          &["E|Short MA (say 5 ticks) crosses above the long MA (say 20 ticks): buy and bracket with a target and a stop a set number of ticks away (p.267)", "X|Cross down: close the long and go short (code p.272-274)"]),
        d("vvbats_spread", "ES/NQ ratio-spread z-score arbitrage", "Building Automated Trading Systems · Ch.46", "vvbats",
          "a two-legged futures spread traded on bid/ask quote changes; we hold no futures or quote data",
          &["E|Spread = -1 x ES bid + 2 x NQ ask; z = (spread - MA30) / sd30; z > 2 short the spread, z < -2 long (p.290-291)", "X|Target z = 0; stop 4 ticks"]),
        d("vvmfm_optquote", "Option quoting around theoretical value (programming example)", "Modeling Financial Markets · order-entry example", "vvmfm",
          "a programming example on option bid/ask quotes (options engine pending; no quote history)",
          &["E|Buy when the ask is 10 cents below the model value; sell when the bid is 10 cents above"]),
        d("vvqmm_pairs", "Pairs statistical arbitrage (z-score of a two-stock spread)", "Quality Money Management · Ch.5 §5.4", "vvqmm",
          "the pair is chosen on 'similar fundamentals' and not named; the holding period is printed as 20 and as 5 trading days; a two-legged long/short trade",
          &["E|z = (spread - 30-day MA) / 30-day sd; z > 2 short the spread, z < -2 long, at the close (p.63-64)", "X|Close after a fixed number of days"]),
        d("vvqmm_factor", "Buy-write filter and multifactor decile long/short", "Quality Money Management · Ch.5 §5.5-5.6", "vvqmm",
          "fundamental ranking portfolios (P/B, P/E, P/S, growth) with optimised weights not given, plus index-call overlay; needs fundamentals and option prices",
          &["E|Top 50 S&P 500 stocks on P/B, size and momentum held a month, index calls sold; or buy the top and sell the bottom decile of a multifactor score (p.64-65)"]),
    ]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id.starts_with("vvbats_") || id.starts_with("vvmfm_") || id.starts_with("vvqmm_")).then_some(("vv_nt", "Van Vliet example systems (held)", "-"))
}
pub fn params(_id: &str) -> Option<Vec<ParamDef>> { None }
pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }
