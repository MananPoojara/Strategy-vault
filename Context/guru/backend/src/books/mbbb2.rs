//! Mike Barna, "The Big Blue-2 Trading System - A Short Term Multi-Pattern Futures Trading System" (April 25, 2004,
//! 48 pp.) with Info.txt and the encrypted "THE BIG BLUE.ELD". Book id `mbbb2`. Source: research drive "USB HDD 1/
//! TradeStation Systems & Indicators/Mike Barna - The Big Blue 2 Trading System (bigblue-2.com)/", copied to
//! Books/Mike Barna/. Refs are the printed page numbers. BIG_BLUE.rar is blocked (no unrar) and was not read.
//! A 15-minute S&P futures day-trading system of four patterns. The manual gives the pivot formulas, the 5-point stop
//! and the market-on-close exit, but not the entry offsets ("a certain threshold", "a certain amount"), the cluster
//! length N, or the time-of-day / volatility / overbought-oversold filters, and the code is encrypted. The entries
//! cannot be written without inventing those numbers, so every pattern is NotTestable. `run` returns None.
use crate::connors::{claim, Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "mbbb2";

const WHY: &str = "entry parameters not disclosed: the manual says only \"a certain threshold\" / \"a certain amount\" for the stop and limit offsets, gives no bar count N for the cluster, and says \"it is beyond the scope of this manual to go into complete detail regarding every filter\" (p.30); the TradeStation code is encrypted. It would also need 15-minute S&P futures bars (intraday), which the vault does not have";

const NOTE: &str = "Tests: 1 large S&P contract, point value fixed at $250, 9/30/1985-3/10/2004 (15-minute data built from back- and forward-adjusted continuous contracts), no slippage or commission, 5-point protective stop, exit at the stop or market on close (p.32). The system is said to trade SP/ES/ND/NQ/DJ (Info.txt).";

const PIVOTS: &str = "F|\"Pivot = (High + Low + Close)/3; Support = 2*Pivot - Yesterday's High; Resistance = 2*Pivot - Yesterday's Low; Lowest Low = Pivot - Resistance + Support; Highest High = Pivot - Support + Resistance; LL2 = Pivot - Highest High + Lowest Low; HH2 = Pivot - Lowest Low + Highest High\" (p.31)";
const EXIT: &str = "X|\"This signal exits on a 5 point protective stop or at MOC.\" (p.32); \"the fixed stop loss is set at 5 points\" (p.32-40)";

fn d(id: &'static str, name: &'static str, group: &'static str, rules: &'static [&'static str], c: Option<crate::connors::Claim>) -> StrategyDef {
    StrategyDef {
        id, name, group, session: 0, dir: "Long/Short", status: Status::NotTestable(WHY),
        needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes: NOTE, claim: c,
    }
}

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        d("mbbb2_pivot_ct", "Pattern 1: Pivot counter-trend", "Ch. IX · Pivot counter-trend pattern", &[
            PIVOTS,
            "E|\"The Long Pivot Counter-Trend pattern occurs when prices fall down below the pivot by a certain threshold, and then begin to rally higher. A stop is set a certain amount above the Support Pivot Price and will either be filled during certain times of the day or canceled at the end of the trading day.\" Short mirrored above Resistance (p.31)",
            "F|\"avoids some mid-day time periods, avoids all trading in November, and does not go short on Fridays. This pattern must signal the first trade during the day\" (p.32)",
            EXIT,
        ], claim("989", "58.54%", "$395.80", "intraday", "9/30/1985-3/10/2004, 1 S&P contract, $250/pt, no costs")),
        d("mbbb2_cluster_ct", "Pattern 2: Cluster counter-trend", "Ch. X · Cluster counter-trend pattern", &[
            "F|\"LL = The Lowest Low of the past N bars; HH = The Highest High of the past N bars.\" (p.35) [N not given]",
            "E|\"moving a limit entry to go long a certain amount below the LL point and by setting a limit to go short a certain amount above HH\" (p.35)",
            "F|\"avoiding certain times of the day, avoiding recent strong intermediate moves, when intraday volatility is less than daily volatility and will not trade at all on Fridays\" (p.35)",
            EXIT,
        ], claim("716", "56.70%", "$303.44", "intraday", "1985-2004, 1 S&P contract, $250/pt, no costs")),
        d("mbbb2_breakout", "Pattern 3: Breakout beyond the second pivot levels", "Ch. XI · Breakout pattern", &[
            PIVOTS,
            "E|\"set stops a predetermined amount above the Highest High Pivot Point and below the Lowest Low Pivot Point ... if it is not filled by the end of the day, the order will be canceled\" (p.38)",
            "F|\"Time of Day, Day of Month, Month, and Volatility filters\" (p.38) [not specified]",
            EXIT,
        ], claim("867", "53.17%", "$298.43", "intraday", "1985-2004, 1 S&P contract, $250/pt, no costs")),
        d("mbbb2_extreme_ct", "Pattern 4: Extreme counter-trend", "Ch. XII · Extreme counter-trend pattern", &[
            PIVOTS,
            "E|\"look at those days that have moved below or above the second level Pivot Price levels then set stops to capture the retracements\" (p.40) [offset not given]",
            "F|\"the available window for this pattern is early in the morning\"; \"avoid the months of July and April and avoid Fridays and Mondays to go short\" (p.40)",
            EXIT,
        ], claim("252", "51.59%", "$359.72", "intraday", "1985-2004, 1 S&P contract, $250/pt, no costs")),
        d("mbbb2_system", "Big Blue-2 complete system (patterns 1-4)", "Ch. XIV · Summary", &[
            "E|Patterns 1-4 combined; \"The system may trade twice per day\" (p.44)",
            EXIT,
            "F|Published full system 9/30/1985-3/10/2004: 1,826 trades, 56.68%, average $363.55, PF 2.13, max DD $12,900 (p.44-46); minimum-filter test: 2,562 trades, 52.07%, average $216.71, PF 1.60 (Figure 14, p.43)",
        ], claim("1,826", "56.68%", "$363.55", "intraday", "9/30/1985-3/10/2004, 1 S&P contract, $250/pt, no costs")),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let v = match id {
        "mbbb2_pivot_ct" => "Pivot counter-trend",
        "mbbb2_cluster_ct" => "Cluster counter-trend",
        "mbbb2_breakout" => "Breakout",
        "mbbb2_extreme_ct" => "Extreme counter-trend",
        "mbbb2_system" => "Complete system",
        _ => return None,
    };
    Some(("mbbb2_bigblue2", "Barna Big Blue-2", v))
}
pub fn params(_id: &str) -> Option<Vec<ParamDef>> { None }
pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }

#[cfg(test)]
mod tests {
    #[test]
    fn catalogue_is_not_testable_and_owned() {
        for s in &super::catalogue() {
            assert!(s.id.starts_with("mbbb2_") && s.book == super::BOOK);
            assert!(matches!(s.status, crate::connors::Status::NotTestable(_)));
            assert!(super::family_of(s.id).is_some());
        }
    }
}
