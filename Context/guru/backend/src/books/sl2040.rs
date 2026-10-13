//! "20/40 Trading" (Safelock 20-40 System; author not named), "20 40 book.pdf" (25 pp., Jan 2007) with the encrypted
//! TradeStation file safelockfinal.ELD. Book id `sl2040`. Source: research drive "USB HDD 1/TradeStation Systems &
//! Indicators/Safelock 20-40 System for TS/", copied to Books/Safelock/. Refs are the printed page numbers.
//! An intraday S&P e-mini (ES) day-trading grid: entries and exits are touches of price lines 4 points apart around the
//! prior close, with 2/4-point targets, a 2-point stop, a once-only re-entry and a 1.5-point "violation" flip. Which
//! line, target or stop is touched first inside the day decides every trade, so daily OHLC cannot run it: NeedsData.
//! `run` returns None.
use crate::connors::{claim, Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "sl2040";

const WHY: &str = "needs intraday S&P e-mini (ES) futures bars fine enough to order touches of levels 2 points apart (1-minute or tick): every entry, target, stop, re-entry and the 1.5-point violation rule depends on the intraday price path, which daily OHLC cannot give; the vault has no intraday SPX/ES (or NIFTY) bars";

const NOTE: &str = "Points are ES index points (86 points = $4,300, i.e. $50 per point, p.12). The rules give no end-of-day exit; all exits are price levels (targets, stop, breakeven). The example days also use unstated discretionary exits (\"if it goes that far against us, exit if we can\", p.16; \"took too long and would have exited with Breakeven\", p.19; \"we will call it a breakeven trade\", p.23) and a 'closed circuit' reset (\"If the 4 point range was closed, you can buy and sell twice at that level again\", p.14) that the rules page does not define; these would have to be resolved from the text before coding. The ELD is encrypted.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef {
        id: "sl2040_grid", name: "20/40 system: 4-point grid countertrend with 2/4-point targets",
        group: "p.3 · The Rules of 20/40 system", session: 0, dir: "Long/Short", status: Status::NeedsData(WHY),
        needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
        rules: &[
            "F|\"Draw a horizontal line on the close of the prior day. Draw horizontal lines in 4 point increments above and below the close of yesterday.\" (p.3)",
            "E|\"When price advances or declines to a 4 point line enter countertrend with two contracts. Each contract will be taken out at different profit targets.\" (p.3)",
            "X|\"First target is 2 points. Second target is 4 points. Stop loss is 2 points.\" (p.3)",
            "A|\"If we hit the 1st target, and price retraces back to the entry level, enter one contract again. Our targets are still 2 points and 4 points, but once the 1st target is hit, our stop on our second contract is Breakeven. We do not re-enter again ... we can trade from that level twice and twice only.\" (p.3)",
            "E|\"If price violates one of these levels by more then 1.5 points and it retraces to that level enter the market in the direction it violated the level.\" (p.3)",
            "F|Published: trade examples 11-1-06 through 11-17-06: \"86 Points = $4300.00\" (p.12); daily trade lists p.13-24",
        ],
        notes: NOTE,
        claim: claim("", "", "86 points ($4,300) in 13 days", "intraday", "11/1/2006-11/17/2006, ES"),
    }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "sl2040_grid" => ("sl2040_grid", "Safelock 20/40", "Book rules"),
        _ => return None,
    })
}
pub fn params(_id: &str) -> Option<Vec<ParamDef>> { None }
pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }

#[cfg(test)]
mod tests {
    #[test]
    fn catalogue_is_needsdata_and_owned() {
        for s in &super::catalogue() {
            assert!(s.id.starts_with("sl2040_") && s.book == super::BOOK);
            assert!(matches!(s.status, crate::connors::Status::NeedsData(_)));
            assert!(super::family_of(s.id).is_some());
        }
    }
}
