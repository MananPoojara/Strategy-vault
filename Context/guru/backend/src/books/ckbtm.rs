//! Charles D. Kirkpatrick II, *Beat the Market: Invest by Knowing What Stocks to Buy and What Stocks to Sell*
//! (FT Press, 2008). Book id `ckbtm`. Source: research drive "USB HDD 2/NEW BOOKS/Charles D. Kirkpatrick - Beat the
//! Market_ ... (2008, FT Press).pdf" (read in place; copy refused - COPY PENDING, see library/ckbtm/README.md).
//! Page refs are printed pages. The method ranks every stock weekly by relative price strength, relative reported
//! earnings growth and relative price-to-sales (percentiles 0-99) and keeps equal-weighted model lists; it needs a
//! cross-sectional stock universe with sales and earnings, so nothing is Live.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "ckbtm";

const XS: &str = "needs a weekly cross-sectional stock universe (all US stocks, ~1980s-2007) with prices, trailing sales, reported earnings and market capitalization to compute percentile ranks, plus a portfolio engine for the equal-weighted model list; the vault runs single instruments and has no fundamentals";
const RS: &str = "Relative price strength: \"Each week, I calculate the ratio of the current weekly closing price to the 26-week moving average ... of closing prices ... I rank the entire list in percentiles with 99 as the highest and zero as the lowest\" (p.96-97).";

macro_rules! nd {
    ($id:expr, $name:expr, $group:expr, $status:expr, $rules:expr) => {
        StrategyDef { id: $id, name: $name, group: $group, session: 0, dir: "Long", status: $status,
            needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules: $rules, notes: RS, claim: None }
    };
}

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        nd!("ckbtm_growth", "Growth List (STRACT rules)", "Ch 10 · Putting it together (p.116-117)", Status::NeedsData(XS),
            &["E|\"To add a stock, it must meet the following conditions: Relative price strength greater than or equal to the 90th percentile ... Relative reported earnings growth greater than or equal to the 90th percentile; Chart pattern showing an upward trend (two higher highs in a three-point reversal point-and-figure chart); Market capitalization of $1 billion or better; Market price greater than or equal to $10\" (p.116)",
              "X|\"To delete a stock, it must meet only one of the following conditions: Relative price strength less than or equal to the 30th percentile; Relative reported earnings growth less than the 70th percentile; Chart break of two previous important lows\" (p.117)"]),
        nd!("ckbtm_value", "Value List (STRACT rules)", "Ch 10 · Putting it together (p.117)", Status::NeedsData(XS),
            &["E|\"To add a stock ... Relative price strength greater than or equal to the 90th percentile; Relative reported earnings growth greater than or equal to the 90th percentile; Relative price-to-sales ratio less than or equal to the 30th percentile; Market capitalization of $500 million; Market price greater than or equal to $10\" (p.117)",
              "X|\"To delete a stock ... Relative price strength less than or equal to the 30th percentile; Relative reported earnings growth less than the 50th percentile; Stocks are not deleted for extraordinarily high relative price-to-sales ratios\" (p.117)"]),
        nd!("ckbtm_rs_triggers", "Relative price strength buy/sell zones", "Ch 9 · Relative price strength selection (p.95-108)", Status::NeedsData(XS),
            &["E|\"Best Buy Zone: Highest possible rank\"; \"Best Sell Zone: Less than 52\" (best trigger levels table, Ch 9); in a declining market \"the best selling rank is anything below the 37th percentile\""]),
        nd!("ckbtm_ma_exposure", "Model-value moving-average exposure rule (12/26/52 weeks)", "Ch 12 · Creating a portfolio (p.137-)", Status::NeedsData("needs the weekly value of the model stock list above (itself NeedsData); the rule is applied to the model value, not to an index"),
            &["E|\"Every week, I compare the current model value to the 12-, 26-, and 52-week simple moving averages. If the current model value declines below its 12-week moving average, I sell 25 percent of my portfolio\" (Ch 12); re-buy 25% steps as it rises back above each average"]),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "ckbtm_growth" => ("ckbtm_lists", "Kirkpatrick relative-strength lists", "Growth"),
        "ckbtm_value" => ("ckbtm_lists", "Kirkpatrick relative-strength lists", "Value"),
        "ckbtm_rs_triggers" => ("ckbtm_lists", "Kirkpatrick relative-strength lists", "RS zones"),
        "ckbtm_ma_exposure" => ("ckbtm_lists", "Kirkpatrick relative-strength lists", "Exposure"),
        _ => return None,
    })
}
pub fn params(_id: &str) -> Option<Vec<ParamDef>> { None }
pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }
