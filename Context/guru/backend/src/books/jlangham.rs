//! James Mars Langham: "Planetary Effects on Stock Market Prices" (book id `jlpe`; the share copy is dated 1936) and "Cyclical Market Forecasting: Stocks and Grain"
//! (c.1938; `jlmf`). Reading: claude-continuation-20261020/authors/langham/agents/. Both are astrological forecasting books: every rule reads
//! planetary aspects, ingresses, eclipses, lunations or sun-spots from an ephemeris and is interpreted by the author; held cards only.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub fn catalogue() -> Vec<StrategyDef> {
    let nt = |id, name, group, book, why, rules| StrategyDef { id, name, group, session: 0, dir: "Long/Short", status: Status::NotTestable(why),
        needs_volume: false, needs_vix: false, needs_series: &[], book, rules, notes: "Held after the 2026-10-25 reading (authors/langham/agents).", claim: None };
    vec![
        nt("jlpe_aspects", "Major-planet aspects and the long trend", "Planetary Effects on Stock Market Prices", "jlpe",
           "needs planetary longitudes (an ephemeris; not in the vault) and the bullish/bearish reading of each aspect is the author's; no entry or exit dates are rule-based",
           &["F|Long market trends correlated with major planetary formations tabulated 1860-1935 (preface, ch.1)"]),
        nt("jlmf_long_trend", "Long-trend planet combinations and chain effects", "Cyclical Market Forecasting · Ch.III-V", "jlmf",
           "needs an ephemeris (the book prints monthly geocentric longitudes 1900-1950 only, ch.XIII) and the interpretation of combinations, chains and transits is judgement",
           &["F|\"Major market trends and which planet combinations cause them\" (ch.III); chain or sequence of planetary formations (ch.IV)"]),
        nt("jlmf_short_swings", "Minor planets, eclipses, ingresses and lunations for short swings", "Cyclical Market Forecasting · Ch.VII-IX", "jlmf",
           "needs daily planetary positions and charts erected for each lunation or ingress; the market reading of each chart is the author's",
           &["F|\"What minor planets indicate the short swings ... Breeders and exciters\" (ch.VII); eclipses, ingresses and lunations (ch.IX)"]),
        nt("jlmf_sunspots_grain", "Sun-spots and grain forecasting", "Cyclical Market Forecasting · Ch.VI, X", "jlmf",
           "needs sun-spot counts and grain futures; weather and crop readings from the zodiac are interpretation",
           &["F|Correlation of sun-spots to economic trends (ch.VI); grain key points of the zodiac, rainfall and drought (ch.X)"]),
    ]
}

fn mine(id: &str) -> bool { id.starts_with("jlmf_") || id.starts_with("jlpe_") }

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    if mine(id) { Some(("jlangham_nt", "Langham planetary forecasting (held)", "-")) } else { None }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> { if mine(id) { Some(vec![]) } else { None } }

pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }
