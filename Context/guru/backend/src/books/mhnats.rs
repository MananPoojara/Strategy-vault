//! Matthew Hanson, *The Alchemy of Trading Systems* (2004/2005), "An Actual System" (printed p.85-91).
//! Source: research drive Trading Knowledge I/Matthew Hanson - The Alchemy of Trading Systems/The Alchemy of Trading
//! Systems (Matthew Hanson,2006).pdf (scanned, read from page images); copy in Books/Matthew Hanson/. Book id "mhnats".
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "mhnats";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        StrategyDef {
            id: "mhnats_oi7", name: "S&P open-interest system: 7-day average OI vs today's OI, always in, open to open", group: "An Actual System (p.85-87)",
            session: 0, dir: "Long/Short", status: Status::NeedsData("daily S&P 500 futures open interest (the rule uses only the S&P futures contract's open interest; NSE NIFTY futures OI history is not in the vault either)"),
            needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
            rules: &[
                "F|\"The system graphed in table 1.2 measures differences in the open interest of the S&P 500 futures contract on a daily basis.\" (p.85)",
                "E|\"=IF (AVERAGE (G3:G9)>G9, 1, -1) This means if the average open interest of the previous seven days of trading is greater than today's open interest, then go long (+1), if not, go short (-1).\" (p.86)",
                "X|\"If the signal stayed the same, you would keep your existing position.\" Data for the close of the 12th is traded \"on the opening of the 13th and holding the position overnight (i.e. one full 24 hour period trading open to open)\" (p.87-88)",
            ],
            notes: "G3:G9 is the 7-row window ending today (G9). The book's improved 'adjacent columns' version (p.88, Table 1.4) is not specified, so it is not catalogued as a strategy. Book: chart results only (Tables 1.2-1.7), no trade statistics.",
            claim: None,
        },
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == "mhnats_oi7").then_some(("mhnats_oi7", "Hanson S&P open-interest system", "p.86 spreadsheet rule"))
}

pub fn params(_id: &str) -> Option<Vec<ParamDef>> { None }

pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }
