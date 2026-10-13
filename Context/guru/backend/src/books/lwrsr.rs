//! Larry Williams, "The Right Stock at the Right Time: Prospering in the Coming Good Years" (Wiley 2003). Book id `lwrsr`.
//! Source (read in place; COPY PENDING): research drive "Trading Books/Larry Williams - The Right Stock at the Right Time.
//! Prospering in the Coming Good Years/Larry Williams - The Right Stock At The Right Time. Prospering In The Coming Good
//! Years.pdf" (239 pp, text). Page refs = printed pages. Transcription and checks: library/lwrsr/.
//!
//! Only the October/April Dow trade (Table 3.2) has exact dates; everything else is a long-term road map, stock selection on
//! fundamentals, or needs data the vault lacks.
use crate::connors::{claim, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "lwrsr";

macro_rules! sd {
    ($id:expr, $name:expr, $group:expr, $dir:expr, $status:expr, $rules:expr, $notes:expr, $claim:expr) => {
        StrategyDef { id: $id, name: $name, group: $group, session: 0, dir: $dir, status: $status, needs_volume: false,
            needs_vix: false, needs_series: &[], book: BOOK, rules: $rules, notes: $notes, claim: $claim }
    };
}

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::{Live, NeedsData, NotTestable};
    vec![
        sd!("lwrsr_oct_apr", "Buy the Dow the last day of October, sell the last trading day of April", "Ch 3 · The October effect", "Long", Live,
            &["E|\"I assumed buying the Dow Jones Industrial Average on the last day in October\" (p.32): Read: on the close of the last trading day of October",
              "X|\"and exiting on the last trading day of April\" (p.32): on that day's close"],
            "Table 3.2 (p.33): one trade a year 1976-2000, average gain 9.05%. Book instrument: the DJIA (DJI in the vault; NIFTY 50 = instrument swap). Similar in spirit to apta_six_best (exit May 4) but a different exit day.",
            claim("25", "", "9.05% a year", "6 months", "1976-2000")),
        sd!("lwrsr_apr_oct", "Buy the Dow the last day of April, sell the last trading day of October", "Ch 3 · The October effect", "Long", Live,
            &["E|\"buy the Dow Jones Industrial Average on the last day of April\" (p.33): on the close", "X|\"and exit on the last trading day of October\" (p.33-34): on that day's close"],
            "Table 3.2 (p.33): average gain 1.1% a year, 1976-2000.", claim("25", "", "1.1% a year", "6 months", "1976-2000")),
        sd!("lwrsr_decennial", "Decennial pattern, four-year phenomenon, 'sure thing seven' and 'straight eight' years, March-to-December / -September tables", "Ch 1-2 · Cycles", "Long",
            NotTestable("Road maps ('years ending in twos and threes', 'the fall of any year ending in seven', 'about the eighth week of the eight year') without exact entry/exit days; Tables 2.1-2.2 use Shiller's monthly-average S&P prices, which are not tradable fills."),
            &[], "", None),
        sd!("lwrsr_value_fed_sentiment", "Dow yield above 6%, Fed discount rate, bond volatility stop, gold, odd-lot / adviser sentiment, mutual-fund cash", "Ch 4 · Bottoms", "Long",
            NeedsData("Needs Dow dividend yield history, Federal Reserve rates, weekly T-Bond futures, gold and sentiment series; none are in the vault."), &[], "", None),
        sd!("lwrsr_stock_selection", "Dow stock and utility portfolios (price-to-sales, price-to-book, working capital, forward P/E, profit margin, high yield), rolling Dow/Utilities programme", "Ch 8-11 · Selection", "Long",
            NeedsData("Needs the Dow 30 and Dow Utilities constituents with fundamentals (sales, book value, earnings estimates, yields); the entry day ('the last week of October') is also not exact."), &[], "", None),
        sd!("lwrsr_fund_timing", "Fabian formula and Freeburg's variant (39-week averages; 2% / 4% / 8% Dow, Transports, Utilities rule)", "Ch 11 · Fund timing", "Long",
            NeedsData("Needs mutual-fund prices and the Dow Transports and Utilities averages; the vault has only the Industrials (DJI)."),
            &["E|\"Buy when the Dow 30 index closes 2 percent above the lowest point of the market decline, and the Dow Transports are 4 percent above ... and the Dow Utilities are 8 percent above\" (p.191)"], "", None),
        sd!("lwrsr_sentiment_options", "Fade the crowd with individual stock sentiment; put writing", "Ch 9 · Sentiment", "Long/Short",
            NeedsData("Needs per-stock sentiment polls and option prices."), &[], "", None),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let f = |a, b, c| Some((a, b, c));
    match id {
        "lwrsr_oct_apr" => f("lwrsr_october", "October effect", "October to April"),
        "lwrsr_apr_oct" => f("lwrsr_october", "October effect", "April to October"),
        "lwrsr_decennial" | "lwrsr_value_fed_sentiment" | "lwrsr_stock_selection" | "lwrsr_fund_timing" | "lwrsr_sentiment_options" =>
            f("lwrsr_other", "Long-term timing and selection", "Not testable / needs data"),
        _ => None,
    }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    match id { "lwrsr_oct_apr" | "lwrsr_apr_oct" => Some(vec![]), _ => None }
}

/// Last bar of each calendar month (needs the next bar to know the month ended).
fn month_end(x: &Ctx, i: usize, m: &str) -> bool { i + 1 < x.r.len() && &x.r[i].date[5..7] == m && x.r[i + 1].date[5..7] != x.r[i].date[5..7] }

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, _p: &Params) -> Option<Vec<Trade>> {
    params(id)?;
    if from > to || to >= x.r.len() { return None; }
    let (buy_m, sell_m) = if id == "lwrsr_oct_apr" { ("10", "04") } else { ("04", "10") };
    let mut out = vec![];
    let mut ent: Option<usize> = None;
    for i in from..=to {
        if let Some(e) = ent {
            if month_end(x, i, sell_m) {
                out.push(Trade { dir: 1.0, legs: vec![Leg { i: e, px: x.c[e], w: 1.0 }], exit_i: i, exit_px: x.c[i], reason: "last trading day of the month, close".into(),
                    entry_kind: "close", signal_i: i, open: false, exits: vec![] });
                ent = None;
            }
        } else if month_end(x, i, buy_m) && i < to { ent = Some(i); }
    }
    if let Some(e) = ent {
        out.push(Trade { dir: 1.0, legs: vec![Leg { i: e, px: x.c[e], w: 1.0 }], exit_i: to, exit_px: x.c[to], reason: "open".into(),
            entry_kind: "close", signal_i: to, open: true, exits: vec![] });
    }
    Some(out)
}
