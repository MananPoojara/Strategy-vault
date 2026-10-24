//! John J. Murphy - textbooks. Book ids `jjmtafm` (Technical Analysis of the Financial Markets, 1999; printed = pdf - 37),
//! `jjmita` (Intermarket Technical Analysis, 1991), `jjmvi` (The Visual Investor, 2009). Reading: claude-continuation-20261020/
//! authors/murphy/agents/. Catalogue only: the complete rules the books state are other traders' systems already run in the
//! vault (Donchian's 4-week rule = kftsm_donchian_4wk, the 4-9-18 method = kftsm_4_9_18, double crossovers = kftsm_hh_cross);
//! oscillator signals are described as common practice without exits.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, book, why, rules| StrategyDef { id, name, group, session: 0, dir: "Long/Short", status: Status::NotTestable(why), needs_volume: false,
        needs_vix: false, needs_series: &[], book, rules, notes: "Read 2026-10-24 (authors/murphy/agents).", claim: None };
    vec![
        d("jjmtafm_weekly_rule", "Donchian 4 week rule (continuous)", "Ch 9 · The Weekly Rule (p.215-217)", "jjmtafm",
          "the same rule runs in the vault as kftsm_donchian_4wk (Kaufman p.177); Murphy reports it, he did not devise it",
          &["E|\"Cover short positions and buy long whenever the price exceeds the highs of the four preceding full calendar weeks\" (p.216)", "X|\"Liquidate long positions and sell short whenever the price falls below the lows of the four preceding full calendar weeks\" (p.216)"]),
        d("jjmtafm_4_9_18", "4-9-18-day moving average method", "Ch 9 (p.204-206)", "jjmtafm",
          "the same rule runs in the vault as kftsm_4_9_18", &["E|4-day crosses above the 9 and 18 = buy alert; the 9 then crossing above the 18 confirms (p.205)"]),
        d("jjmtafm_crossovers", "Double moving-average crossovers (5/20, 10/40, 4/9, 9/18)", "Ch 9 (p.203-204, 221)", "jjmtafm",
          "the generic double crossover runs in the vault as kftsm_hh_cross with these periods", &["E|Buy when the shorter average crosses above the longer; sell when it crosses below (p.203)"]),
        d("jjmtafm_oscillators", "RSI 70/30 and stochastic crossings, MACD", "Ch 10 (p.239-256)", "jjmtafm",
          "described as what 'many traders' do ('a crossing back above the 30 line ... is taken by many traders as a confirmation', p.245), with no exit or position rule",
          &["E|RSI crossing back above 30 = buy, back under 70 = sell (p.245)"]),
        d("jjmita_intermarket", "Intermarket ratio and stochastics signals (CRB, bonds, dollar, stocks)", "Intermarket Technical Analysis", "jjmita",
          "needs CRB, bond and dollar series and reads the relationships on charts", &["F|Buy signals in the CRB Index are bearish for bonds; read ratio charts"]),
        d("jjmvi_signals", "Indicator signals on ETF charts", "The Visual Investor", "jjmvi",
          "the same textbook signals as Technical Analysis of the Financial Markets, explained on charts", &["E|Moving-average, RSI, stochastic and MACD crossings"]),
    ]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id.starts_with("jjmtafm_") || id.starts_with("jjmita_") || id.starts_with("jjmvi_")).then_some(("jjm_nt", "Murphy textbook rules (held)", "-"))
}
pub fn params(_id: &str) -> Option<Vec<ParamDef>> { None }
pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }
