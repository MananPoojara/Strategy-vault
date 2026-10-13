//! Steven Primo / TradingMarkets (The Connors Group), "PowerRatings" course manuals (2006):
//! PRP_CONSERV, PRP_LONG-SHORT, PRP_MOMENTUM, PRP_SHORT-ONLY, PRP_BONUS-RESEARCH. Book id `prpr`.
//! Source: research drive `Trading Courses,Seminars,Videos/Steven Primo - PowerRatings Course (Videos & Manuals 1.28 GB), $4995 (tradingmarkets.com)/`.
//! Every system needs the proprietary daily PowerRatings score per stock. Notes: library/prpr/README.md.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

const PR: &str = "daily PowerRatings (TradingMarkets' proprietary 1-10 stock score) history for every S&P 500 and NASDAQ stock 1995-2005; the score's formula is not published, and the vault's NSE baskets have no such score. The systems also rank candidates across the whole stock universe by 100-day HV";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, dir, rules, notes| StrategyDef { id, name, group, session: 0, dir, status: Status::NeedsData(PR),
        needs_volume: true, needs_vix: false, needs_series: &[], book: "prpr", rules, notes, claim: None };
    vec![
        d("prpr_double", "Conservative Portfolio - Double System", "Conservative Portfolio", "Long",
          &["F|PowerRatings 9 or 10 today and yesterday; Close > SMA(200); Close < SMA(5); ADX(14) > 25; HV(100) > 30; 20-day avg of Close*Volume > 65-day avg of (S&P 500 * NYSE volume) * 0.000003",
            "E|Buy limit at Close - (10-day avg of High-Low)/1; max 15 positions, highest HV(100) first",
            "X|Daily sell limit at the previous day's 5-day MA + Range/5"],
          "Claims (1995-2005, portfolio): 100% winning years, 73% winning trades."),
        d("prpr_jump", "Conservative Portfolio - Jump System", "Conservative Portfolio", "Long",
          &["F|PR = 8 and yesterday <= 5, or PR = 9 and yesterday <= 7, or PR = 10 and yesterday <= 8; Close > SMA(200); Close < SMA(5); ADX(14) > 25; HV(100) > 30; long volume filter",
            "E|Buy limit at Close - Range/4", "X|Daily sell limit at the previous day's 5-day MA + Range/10"], ""),
        d("prpr_short", "Short System (Conservative, Long/Short and Short-Only portfolios)", "Short", "Short",
          &["F|PowerRatings = 1; Close > SMA(10); ADX(14) > 30; HV(100) > 30 (Short-Only manual prints \"> 3\"); 20-day avg Close*Volume > 65-day avg (S&P 500 * NYSE volume) * 0.00002",
            "E|Short limit at Close + Range/10; max 12 positions",
            "X|Daily cover limit at the previous day's 10-day MA - Range/5; cover stop at a 50% loss from entry"],
          "Same rules in three manuals."),
        d("prpr_pullback", "Long/Short and Momentum Portfolios - long side", "Long/Short / Momentum", "Long",
          &["F|PowerRatings 9 or 10; Close > SMA(200); Close < SMA(10); ADX(14) > 25; HV(100) > 30; long volume filter",
            "E|Buy limit at Close - Range/3; max 15 (Long/Short) or 10 (Momentum) positions",
            "X|Daily sell limit at the previous day's 10-day MA + Range/4"],
          "Momentum claim: nearly 3% average gain per trade, 1995-2005."),
        d("prpr_market_timing", "PowerRatings Market Timing Indicator", "Bonus Research", "Long/Short",
          &["F|\"Applies PowerRatings in timing both up and down markets\" on the S&P 500 index, 1995-2005 (bonus p.2)"],
          "No rule is printed, only highlights (\"Up-market timing correct over 68%\", 1-2 trades a month)."),
    ]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    id.starts_with("prpr_").then_some(("prpr", "PowerRatings portfolios", "Needs PowerRatings data"))
}
pub fn params(_: &str) -> Option<Vec<ParamDef>> { None }
pub fn run(_: &str, _: &Ctx, _: usize, _: usize, _: &Params) -> Option<Vec<Trade>> { None }
