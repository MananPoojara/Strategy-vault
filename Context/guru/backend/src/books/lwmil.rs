//! Larry Williams, "How I Made One Million Dollars ... Last Year ... Trading Commodities" (Windsor Books, 1973/74). Book id
//! `lwmil`. Source (read in place; COPY PENDING): research drive "USB HDD 2/tRADING KNO-2/Larry Williams - How I Made One Million
//! Dollars/How I made one million dollars  Larry R Williams.pdf" (image scan, 146 PDF pages = printed pp.1-130; every page
//! image was read; printed p.108 is missing from the scan). Catalogue only; see library/lwmil/README.md.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "lwmil";

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::{NeedsData, NotTestable};
    let d = |id, name, group, status, rules, notes| StrategyDef { id, name, group, session: 0, dir: "Long/Short", status, needs_volume: false,
        needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None };
    vec![
        d("lwmil_fundamental_tools", "Large traders, premiums, open interest in a trading range", "Ch 4-5 · Fundamental tools",
            NeedsData("Needs commodity futures with open interest, nearby/deferred contract spreads and the large-trader report; \"a 25% increase or decrease is enough\" (p.55) but the trading range is drawn by eye; set-ups, not timing tools."),
            &["F|\"a price consolidation ... during which Open Interest declines ... We should be buyers\"; OI rising in a range = sell (p.32, 52-53)", "F|nearby contracts at a premium to the distant = bull market (p.30, 41-42)"], ""),
        d("lwmil_pctr_ten_week", "10-day %R signals with the ten-week average trend", "Ch 6 · Technical tools",
            NotTestable("Entries only: %R below 95% (inverted scale) buys in bull markets chosen by the fundamental tools and the ten-week average, and the exit is a break of a momentum trend line or a blow-off (p.77, 129), drawn by eye; \"flat\" ten-week average (go long and short) is not defined."),
            &["F|%R = (10-day high - close) / (10-day high - 10-day low); \"readings below 95% give a buy indication - during bull markets. A reading above 10% gives a sell signal during bear markets\" (p.69)",
              "F|After a blow-off: wait for %R 100%, five trading days, then below 95% again (p.71)", "F|Trading range: buy at %R 90% or lower, sell and short at 10% (p.76)",
              "F|Ten week average of Friday closes: up = only buy, down = only sell short, flat = both (p.78-80)"], ""),
        d("lwmil_momentum", "25-day momentum, 10-day average crossing, 20-day MA difference", "Ch 6 · Technical tools",
            NotTestable("Momentum trend-line breaks are drawn by eye; the 10-day average crossing of momentum is given as \"another approach\" with the cycle-dependent length chosen per market (p.67) and no exit."),
            &["F|\"a 10-day moving average of the momentum index itself, taking it as a 'buy' signal when the momentum index crosses above its own 10-day moving average, as a 'sell' signal when it falls below it\" (p.67)"], ""),
        d("lwmil_moon", "Full moon buys / new moon sells (silver, wheat, corn, soybean oil), sugar declination", "Ch 8 · The great silver secret",
            NeedsData("Needs silver, wheat, corn, soybean oil and sugar futures; signals are \"on or about\" the moon phase with the exact purchase refined by the trader (p.93)."),
            &["E|\"A buy signal is forecast for Silver, Wheat, Corn and Soy Bean Oil whenever a full moon appears\"; sells at the new moon (p.93-94)"], "Results quoted: 15 of 18 silver signals, 22 corn signals 'no bad signals' (p.95)."),
        d("lwmil_chart_hints", "Reversal days, gaps, five-day signal, secret of three, pivot price, stops", "Ch 7, 10-11",
            NotTestable("Observations and order tactics, not complete rules: no entry order or exit for reversal days, gaps or the five-day signal (\"look for a spot in which to take a position\", p.110); the pivot formula forecasts levels; stops are sized by the trader."),
            &["F|\"Any time prices have moved up, three days in a row, the odds are 82% for a down move on the fourth and fifth day\" (p.110)", "F|Pivot = (H + L + C) / 3; next high = pivot + (pivot - low) (p.119)",
              "X|\"never expose any trade to a loss greater than 5% of your commodity funds\" (p.14, 126); four-day time stop (p.115)"], "Money management (30% stop rule, six positions, p.11-16) is sizing."),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    id.starts_with("lwmil_").then_some(("lwmil", "How I Made One Million Dollars", if id.contains("fundamental") || id.contains("moon") { "Needs data" } else { "Not testable" }))
}
pub fn params(_id: &str) -> Option<Vec<ParamDef>> { None }
pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }
