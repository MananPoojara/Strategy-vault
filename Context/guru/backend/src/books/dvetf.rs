//! David Vomund, "ETF Trading Strategies Revealed" (Marketplace Books, 2007).
//! The mechanical models (Part III) all rotate between several ETFs/funds; the vault lacks most of those series
//! (S&P 600 value/growth, sector ETFs, Fidelity sector funds, VUSTX, EEM/EFA history) and the engine runs one ticker,
//! so they are NeedsData. The chart-reading chapters (Raschke, Palmquist) are not testable. No index-only rule exists.
//! Source: library/dvetf/.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "dvetf";

const STYLE: &str = "price history of DIA/DJIA, QQQQ/Nasdaq-100, SPY/S&P 500, MDY/S&P 400, IJS/S&P 600 BARRA Value, IJT/S&P 600 BARRA Growth, IWM/Russell 2000 from 1997, ranked together (IJS/IJT and their indexes are not in the vault; the engine runs one ticker at a time)";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, status, rules, notes| StrategyDef {
        id, name, group, session: 0, dir: "Long", status, needs_volume: false, needs_vix: false, needs_series: &[],
        book: BOOK, rules, notes, claim: None,
    };
    use Status::*;
    let g4 = "Ch. 4 Market Rotation Strategy";
    vec![
        d("dvetf_basic_rotation", "Basic Rotation (6-month price change, top 2 of 7)", g4, NeedsData(STYLE),
          &["E|\"the two best-performing ETFs were purchased with equal dollar amounts ... held through the remainder of the month\" (p. 22)",
            "F|\"buys the best performing ETFs as measured over the previous six-month period\" - \"six-month returns (approximately 120 trading days)\" (p. 22)",
            "X|\"As long as the current holdings were among the top three performers, then there were no changes. If a holding was not one of the three best six-month performers, then it was sold and the best performer was purchased. We never doubled into a position.\" (p. 22)",
            "F|\"rebalanced at the end of every year\"; \"positions were bought and sold on the day a switch is indicated and a $17 commission was applied. Dividends were excluded\" (p. 22)"],
          "Book: 1998-Mar 2006 AROI 16.91%, 4 1/4 round trips/yr, avg hold 179 days (Table 2, p. 23)."),
        d("dvetf_style_index", "Style Index rotation (weighted 120-day RS, every other week)", g4, NeedsData(STYLE),
          &["F|\"break the six-month time period into four equal parts (i.e., about thirty trading days). Calculate a percentage change for each period and then average them, placing twice the weight on the most recent period\" (p. 24)",
            "E|top two of seven bought with equal dollars; evaluated \"every other week\" (p. 24)",
            "X|\"If a holding ... was no longer in the top three, then it was sold and the highest-rated ETF was purchased\" (p. 24)"],
          "Book: 1998-May 2006 AROI 15.20%, 10 round trips/yr, avg hold 74 days (Table 3, p. 25). Weighting ambiguous (2x on newest quarter: weights 1,1,1,2 / 5)."),
        d("dvetf_style_weekly", "Style Index · Variation #1 weekly rotation", "Ch. 5 Types of Variations", NeedsData(STYLE),
          &["E|\"run the Style Index relative strength report every week\" (p. 26)"], "Book: AROI 16.35%, 12 round trips/yr (Table 4, p. 26)."),
        d("dvetf_style_bond", "Style Index · Variation #2 with bond fund", "Ch. 5 Types of Variations",
          NeedsData("the seven style series above plus Vanguard Long-Term U.S. Bond Fund (VUSTX) from 1997"),
          &["E|eight choices, hold while in the top four (p. 27)"], "Book: AROI 15.14%, 7 1/2 round trips/yr (Table 5, p. 27)."),
        d("dvetf_style_intl", "Style Index · Variation #3 with EEM and EFA", "Ch. 5 Types of Variations",
          NeedsData("the seven style series above plus EEM and EFA (or MSCI EM / EAFE indexes) from 1997"),
          &["E|nine choices, hold while in the top four (p. 27)"], "Book: AROI 19.10%, 6 round trips/yr (Table 6, p. 28)."),
        d("dvetf_fidelity_sectors", "Style Index model on 41 Fidelity sector funds", "Ch. 6 Sector ETF Rotation Using Style Index Model",
          NeedsData("daily NAVs of the 41 Fidelity Select sector funds from 1998"),
          &["E|top two by the weighted 120-day RS, held while in the top twenty, every two weeks; fills at the report day's close (p. 28)"],
          "Book: 1998-Mar 2006 AROI 26.30%, 3 3/4 round trips/yr (Table 7, p. 29)."),
        d("dvetf_ishares_sectors_rs", "Style Index model on iShares sector/international ETFs", "Ch. 6 Sector ETF Rotation Using Style Index Model",
          NeedsData("the 24 iShares sector and regional ETFs of Table 8 from 2002"),
          &["E|top two, held while in the top half (9 of 19 in 2002, 12 after); fills at the report day's close (p. 30)"],
          "Book: 2002-Mar 2006 AROI 20.04%, 5 round trips/yr (Table 9, p. 30)."),
        d("dvetf_lookback_research", "Look-back study: rotate to best 2 every 22 days", "Ch. 7 Sector ETF Rotation Research",
          NeedsData("Fidelity sector funds from 1995 and the Table 8 iShares ETFs from 2002"),
          &["E|\"Every twenty-two business days ... the two best performing sector funds were bought and held for twenty-two days\" (p. 31)"],
          "Book: Fidelity 1m 9.86%, 3m 15.90%, 6m 16.81%, 1y 19.79%, 1.5y 15.66% (Table 10, p. 31); iShares 7.47/8.39/13.97/23.21/15.10% (Table 11, p. 32)."),
        d("dvetf_sector_1y", "Sector ETF Trading Model (1-year return, top 2, monthly)", "Ch. 8 Sector ETF Trading Model",
          NeedsData("the 24 iShares sector and regional ETFs of Table 8 from 2001"),
          &["E|\"At the start of every month, we calculate a one-year return ... The two best performing ETFs are purchased with equal dollar amounts\" (p. 32)",
            "X|\"If a holding is no longer one of the two best performers, then it is sold and replaced by the best performing ETF\" (p. 32)"],
          "Book: 2002-Mar 2006 AROI 17.20%, 8 1/4 round trips/yr, avg hold 78 days (Table 12, p. 34)."),
        d("dvetf_raschke_swings", "Raschke ETF swing/pullback trading", "Ch. 2 Valuable Advice from Linda Bradford Raschke",
          NotTestable("chart patterns, swings, divergences and trend side chosen by eye; the 8-bar time stop is paired with an unspecified fixed stop"),
          &["E|\"I trade the basic chart patterns like the triangle. I trade breakouts, and I trade pullbacks after breakouts\" (p. 7)",
            "X|\"I often employ an eight bar time stop in conjunction with a fixed stop\" (p. 10)"], "Interview; not coded."),
        d("dvetf_palmquist_pullback", "Palmquist pullback / base breakout setups", "Ch. 3 Steve Palmquist & ETF Trading Techniques",
          NotTestable("'rapid price rise of at least two weeks', market mode, targets at 'recent high / upper Bollinger Band / trendline' and topping-pattern exits are judgement"),
          &["E|setup: \"a rapid price rise of at least two weeks with several above average, volume up days, followed by a pullback ... on generally below average volume\"; trigger: higher high than the previous day on above-average volume (p. 13)",
            "X|stop just under the lowest low of the setup; limit target judged per market mode (p. 13-14)"], "Not coded."),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "dvetf_basic_rotation" | "dvetf_style_index" | "dvetf_style_weekly" | "dvetf_style_bond" | "dvetf_style_intl" => ("dvetf_style", "Vomund style rotation", "needs data"),
        "dvetf_fidelity_sectors" | "dvetf_ishares_sectors_rs" | "dvetf_lookback_research" | "dvetf_sector_1y" => ("dvetf_sector", "Vomund sector rotation", "needs data"),
        "dvetf_raschke_swings" | "dvetf_palmquist_pullback" => ("dvetf_chart", "ETF chart trading (interviews)", "not testable"),
        _ => return None,
    })
}

pub fn params(_id: &str) -> Option<Vec<ParamDef>> { None }

pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }
