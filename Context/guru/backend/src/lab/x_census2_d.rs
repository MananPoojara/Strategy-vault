//! Census 2 group "d" (2026-10-27): indicators from trading_indicator_census.html that the lab did not have.
//! Formulas: see each definition string. Listed-only entries carry "not available yet — needs …" (data the lab does not hold)
//! or "not available yet — formula not published (…)". Nothing here is computed: every item needs data the lab does not hold
//! or has no primary formula. Skipped (not indicators): Elliott Wave (a wave-counting method), Hogscraper (a c. 1780–1860
//! candle holder, census false positive; sv-census-work/research_candles.md §6), Pesavento Patterns (ratio labels drawn on
//! zigzag legs, a drawing on a look-ahead zigzag). Judas Candle and Rounding Bottom keep the earlier UNCONFIRMED decision
//! (research_candles.md §5, research_chart_patterns.md §21): listed, not built.
#![allow(dead_code)]
use super::bars::Bars;
use super::catalogue::{IndicatorDef, Needs, Output, ParamSpec, Params, Placement};

#[allow(clippy::too_many_arguments)]
const fn def(id: &'static str, name: &'static str, category: &'static str, params: &'static [ParamSpec], outputs: &'static [&'static str], placement: Placement,
             needs: Needs, definition: &'static str, first: &'static [usize], sparse: bool, f: fn(&Bars, &Params) -> Output) -> IndicatorDef {
    IndicatorDef { id, name, category, params, outputs, placement, needs, definition, first, sparse, compute: Some(f), note: "" }
}
/// Listed in the picker, not computed: the data it needs is not admitted (the app shows the note as "data not available").
const fn listed(id: &'static str, name: &'static str, category: &'static str, needs: Needs, definition: &'static str, note: &'static str) -> IndicatorDef {
    IndicatorDef { id, name, category, params: &[], outputs: &["value"], placement: Placement::Pane, needs, definition,
                   first: &[usize::MAX], sparse: false, compute: None, note }
}

const BRD: &str = "Breadth";
const OFC: &str = "Order flow";
const IOF: &str = "Intraday & order flow";
const DER: &str = "Derivatives market data";
const PF: &str = "Price functions";
const CHP: &str = "Chart patterns";
const CDL: &str = "Candlestick patterns";
const NO: Needs = Needs::NONE;
const BR: Needs = Needs { breadth: true, ..Needs::NONE };
const DV: Needs = Needs { derivatives: true, ..Needs::NONE };
const ID: Needs = Needs { intraday: true, ..Needs::NONE };
const OF: Needs = Needs { order_flow: true, ..Needs::NONE };
const OF_TIME: Needs = Needs { intraday: true, order_flow: true, ..Needs::NONE };
const BOOK: Needs = Needs { order_book: true, ..Needs::NONE };
const SEC: Needs = Needs { second_symbol: true, ..Needs::NONE };
const FUN: Needs = Needs { fundamentals: true, ..Needs::NONE };

const NHNL: &str = "not available yet — needs daily new 52-week high and new 52-week low counts (the lab's breadth holds only advances, declines and unchanged)";
const MEMBERS: &str = "not available yet — needs price data for every member stock of the index (the lab's breadth holds only advance/decline counts)";
const COT: &str = "not available yet — needs CFTC Commitments of Traders positions";
const TRADES: &str = "not available yet — needs the number of trades per bar";
const TAPE: &str = "not available yet — needs time-ordered intraday (tick) trade data";
const BOOK_NOTE: &str = "not available yet — needs order-book (bid/ask quote and depth-of-market) data";
const XO: &str = "not available yet — formula not published (Prashant Shah / Definedge, Trading the Markets the Point & Figure Way; the book is not on the research share and the public TradingView versions disagree)";

pub const DEFS: &[IndicatorDef] = &[
    // Breadth & sentiment: new highs / new lows
    listed("high_low_index", "High-Low Index", BRD, BR, "StockCharts ChartSchool High-Low Index: 10-day simple moving average of Record High Percent, RHP = 100·new highs/(new highs + new lows), from daily new 52-week high and low counts", NHNL),
    listed("record_high_percent", "Record High Percent", BRD, BR, "StockCharts ChartSchool Record High Percent: 100·new 52-week highs/(new 52-week highs + new 52-week lows)", NHNL),
    listed("high_low_percent", "High-Low Percent", BRD, BR, "StockCharts ChartSchool High-Low Percent: (new 52-week highs − new 52-week lows) as a percent of total issues", NHNL),
    listed("net_new_highs", "Net New 52-Week Highs", BRD, BR, "Net new highs: daily count of new 52-week highs minus new 52-week lows (StockCharts net new highs breadth)", NHNL),
    listed("pring_net_new_highs", "Pring's Net New High Indicators", BRD, BR, "Martin Pring's net new high indicators (smoothed new highs minus new lows)", NHNL),
    listed("new_highs_lows_volume", "New Highs-New Lows Volume", BRD, BR, "Volume of the issues making new 52-week highs against the volume of those making new lows", "not available yet — needs the volume of new 52-week high and new 52-week low issues"),
    listed("hindenburg_omen", "Hindenburg Omen", BRD, BR, "Hindenburg Omen (Jim Miekka): a breadth warning built from daily new 52-week high and new low counts as a share of issues traded, a rising 10-week trend and a negative McClellan Oscillator", NHNL),
    // Breadth & sentiment: member-stock universe
    listed("percent_above_ma", "Percent Above Moving Average", BRD, BR, "StockCharts Percent Above Moving Average (e.g. $SPXA50R): percentage of an index's member stocks closing above their n-day moving average", MEMBERS),
    listed("bullish_percent_index", "Bullish Percent Index", BRD, BR, "StockCharts Bullish Percent Index: percentage of an index's member stocks on a Point & Figure buy signal", MEMBERS),
    listed("pring_diffusion", "Pring's Diffusion Indicators", BRD, BR, "Martin Pring's diffusion indicators: percentage of the components of a group in a positive state (e.g. above a moving average or with a positive rate of change)", MEMBERS),
    listed("pring_bottom_fisher", "Pring's Bottom Fisher", BRD, BR, "Martin Pring's Bottom Fisher: a diffusion-based oversold indicator built from the components of a group", MEMBERS),
    listed("pring_inflation_deflation", "Pring's Inflation and Deflation Indexes", BRD, SEC, "Martin Pring's Inflation and Deflation Indexes: composites of inflation-sensitive (commodity) and deflation-sensitive (bond, utility) series", "not available yet — needs the commodity and bond component price series"),
    listed("odd_lot", "Odd-Lot Theory (Odd Lotter)", BRD, BR, "Odd-lot theory (Garfield Drew): odd-lot sales against odd-lot purchases as a contrary sentiment gauge", "not available yet — needs exchange odd-lot purchase and sale statistics"),
    listed("dp_rydex_ratio", "DecisionPoint Rydex Ratio", BRD, FUN, "DecisionPoint Rydex Ratio (StockCharts): Rydex bear and money-market fund assets against bull fund assets as a sentiment gauge", "not available yet — needs Rydex mutual fund asset totals"),
    // Derivatives sentiment
    listed("cot", "Commitment of Traders", DER, DV, "CFTC Commitments of Traders report: weekly futures positions of commercials, large speculators and small traders", COT),
    listed("cot_index", "COT Index", DER, DV, "COT Index (Larry Williams): 100·(net position − its lowest over a lookback)/(highest − lowest over the lookback) from CFTC Commitments of Traders positions", COT),
    listed("cboe_skew", "CBOE SKEW Index", DER, DV, "Cboe SKEW Index: 100 − 10·S, S the risk-neutral skewness of 30-day S&P 500 returns from out-of-the-money option prices", "not available yet — needs Cboe SKEW index (S&P 500 option) data"),
    // Intraday
    listed("smart_money_index", "Smart Money Index", IOF, ID, "Smart Money Index (Don Hays): yesterday's value − the index change over the first 30 minutes + the change over the last hour", "not available yet — needs intraday (first 30 minutes / last hour) bars"),
    listed("rotation_factor", "Rotation Factor", IOF, ID, "Rotation Factor (Dalton, Mind Over Markets): sum over the session's half-hour periods of +1/−1 for a higher/lower high and +1/−1 for a higher/lower low than the previous period", "not available yet — needs intraday (half-hour) bars"),
    listed("range_bar_predictor", "Range Bar Predictor", IOF, ID, "Sierra Chart Range Bar Predictor: the prices at which the current range bar would close up or down; range bars are built from intraday data (see range_bars)", "not available yet — needs intraday (lower-timeframe/tick) bars"),
    // Order flow
    listed("number_of_trades", "Number of Trades", OFC, OF, "Number of trades executed in the bar (exchange trade count)", TRADES),
    listed("average_trade_size", "Average Trade Size", OFC, OF, "Average trade size: bar volume / number of trades in the bar", TRADES),
    listed("speed_of_tape", "Speed of Tape", IOF, OF_TIME, "ATAS Speed of Tape: trades (or volume, delta, buys, sells) per second from the time-and-sales tape", TAPE),
    listed("tape_patterns", "Tape Patterns", IOF, OF_TIME, "ATAS Tape Patterns: chains of trades on the tape that match set criteria", TAPE),
    listed("sweeps", "Sweeps Indicator", IOF, OF_TIME, "Order-flow sweep: one aggressive order filled through several price levels in a burst of trades (not the swing-level liquidity_sweep)", TAPE),
    listed("momentum_tick_difference", "Momentum of Tick Difference", IOF, OF_TIME, "Momentum of the difference between uptick and downtick trades", "not available yet — needs uptick and downtick trade counts (tick data)"),
    listed("order_flow_rhythm", "Order Flow Rhythm", IOF, OF_TIME, "Order flow rhythm study (no primary text found)", "not available yet — needs time-ordered intraday (tick) trade data; no primary formula was found either"),
    listed("bid_ask_spread", "Bid-Ask Spread", OFC, BOOK, "Bid-ask spread: best ask − best bid (not the lab's spread id, which is price minus a second instrument)", BOOK_NOTE),
    listed("dom_power", "DOM Power", OFC, BOOK, "ATAS DOM Power: activity in the depth of market", BOOK_NOTE),
    listed("dom_strength", "DOM Strength", OFC, BOOK, "Depth-of-market strength: resting bid against resting ask size", BOOK_NOTE),
    listed("absorption", "Absorption", OFC, NO, "Order-flow absorption: aggressive volume met by passive orders without price progress; described only qualitatively (x_orderflow.rs: no exact public rule)", "not available yet — formula not published (no exact public rule from ATAS, Bookmap or Sierra Chart)"),
    listed("spread_volumes", "Spread Volumes", OFC, NO, "Spread Volumes (named in ATAS material; no published definition)", "not available yet — formula not published (ATAS, closed source)"),
    listed("strength_level", "Strength Level Indicator", OFC, NO, "Strength Level Indicator (no primary text found)", "not available yet — formula not published (no primary text found)"),
    // Timeless: Point & Figure (XO) indicators
    listed("xo_ama", "XO AMA", PF, NO, "Prashant Shah's XO adaptive moving average on Point & Figure columns", XO),
    listed("xo_count", "XO Count", PF, NO, "Prashant Shah's count of Xs and Os per Point & Figure column", XO),
    listed("xo_indicator", "XO Indicator", PF, NO, "Prashant Shah's XO indicator on Point & Figure columns", XO),
    listed("xo_line", "XO Line", PF, NO, "Prashant Shah's XO line on Point & Figure columns", XO),
    listed("xo_strength", "XO Strength", PF, NO, "Prashant Shah's XO strength on Point & Figure columns", XO),
    listed("xo_volatility", "XO Volatility", PF, NO, "Prashant Shah's XO volatility on Point & Figure columns", XO),
    listed("xo_zone", "XO Zone", PF, NO, "Prashant Shah's Column XO Zone: Xs and Os counted per column over a lookback of columns, scaled by the column count", XO),
    // Patterns
    listed("chart_rounding_bottom", "Rounding Bottom", CHP, NO, "Rounding bottom / saucer (StockCharts, Bulkowski roundb): a rounded bowl confirmed by a close above the left peak; no source publishes a numeric shape rule (sv-census-work research_chart_patterns.md §21)", "not available yet — formula not published (StockCharts and Bulkowski give no numeric shape rule)"),
    listed("chart_semi_cup", "Semi-Cup Formation", CHP, NO, "Semi-cup (Giorgos Siligardos, Identifying Cup Formations Early, TASC 29:4, April 2011): the left half of a cup found early; thinkorswim SemiCupFormation summarises it as a run of bars whose first bar is the highest, first/last ≥ factor, split into five equal parts, last two parts in the lower 40% and the middle part in the lower 60% of the log-price range, a simple-smoothed directional index of log prices above 25 in the first two parts and below 25 in the last three; defaults, DI length and the per-part test are not given", "not available yet — formula not published (no readable primary text: the TASC article and its Traders' Tips code are paywalled; the thinkorswim summary gives no defaults, DI length or per-part rule)"),
    listed("cdl_judas", "Judas Candle", CDL, NO, "Judas candle: a large black candle followed by a smaller white candle whose lower tail equals the black candle's length (one uncited Wikipedia line, no thresholds; research_candles.md §5)", "not available yet — formula not published (only an uncited Wikipedia line, no thresholds)"),
    listed("cdl_falter_reverse", "Falter and Reverse", CDL, NO, "TradeStation Falter & Reverse Dn/Up (RadarScreen indicator, designed for intraday intervals): after a NumDays (3) price extension (MinConsolIndex 2.25), yesterday starts strong but retreats by the close and today pulls out of yesterday's range (FinalRangeFactor 0.75); the rules sit in undocumented functions RS_PriceExtension, RS_ReversalPatterns and RS_TrueExtremes", "not available yet — formula not published (TradeStation documents only the inputs and a qualitative setup; its RS_PriceExtension and RS_ReversalPatterns functions are not documented)"),
];
pub const SOURCES: &[(&str, &str, &str)] = &[
    ("high_low_index", "StockCharts: High-Low Index", ""),
    ("record_high_percent", "StockCharts: Record High Percent", ""),
    ("high_low_percent", "StockCharts: High-Low Percent", ""),
    ("net_new_highs", "StockCharts: Net New 52-Week Highs", ""),
    ("pring_net_new_highs", "Martin Pring: Net New High indicators", ""),
    ("new_highs_lows_volume", "New Highs-New Lows Volume", ""),
    ("hindenburg_omen", "Jim Miekka: Hindenburg Omen", ""),
    ("percent_above_ma", "StockCharts: Percent Above Moving Average", ""),
    ("bullish_percent_index", "StockCharts: Bullish Percent Index", ""),
    ("pring_diffusion", "Martin Pring: Diffusion Indicators", ""),
    ("pring_bottom_fisher", "Martin Pring: Bottom Fisher", ""),
    ("pring_inflation_deflation", "Martin Pring: Inflation and Deflation Indexes", ""),
    ("odd_lot", "Garfield Drew: Odd-Lot Theory", ""),
    ("dp_rydex_ratio", "StockCharts DecisionPoint: Rydex Ratio", ""),
    ("cot", "CFTC: Commitments of Traders", ""),
    ("cot_index", "Larry Williams: COT Index", ""),
    ("cboe_skew", "Cboe: SKEW Index", ""),
    ("smart_money_index", "Don Hays: Smart Money Index", ""),
    ("rotation_factor", "James Dalton (Mind Over Markets): Rotation Factor", ""),
    ("range_bar_predictor", "Sierra Chart: Range Bar Predictor", ""),
    ("number_of_trades", "Number of Trades", ""),
    ("average_trade_size", "Average Trade Size", ""),
    ("speed_of_tape", "ATAS: Speed of Tape", ""),
    ("tape_patterns", "ATAS: Tape Patterns", ""),
    ("sweeps", "Sweeps Indicator", ""),
    ("momentum_tick_difference", "Momentum of Tick Difference", ""),
    ("order_flow_rhythm", "Order Flow Rhythm", ""),
    ("bid_ask_spread", "Bid-Ask Spread", ""),
    ("dom_power", "ATAS: DOM Power", ""),
    ("dom_strength", "DOM Strength", ""),
    ("absorption", "Order flow: Absorption", ""),
    ("spread_volumes", "ATAS: Spread Volumes", ""),
    ("strength_level", "Strength Level Indicator", ""),
    ("xo_ama", "Prashant Shah: XO AMA", ""),
    ("xo_count", "Prashant Shah: XO Count", ""),
    ("xo_indicator", "Prashant Shah: XO Indicator", ""),
    ("xo_line", "Prashant Shah: XO Line", ""),
    ("xo_strength", "Prashant Shah: XO Strength", ""),
    ("xo_volatility", "Prashant Shah: XO Volatility", ""),
    ("xo_zone", "Prashant Shah: Column XO Zone", ""),
    ("chart_rounding_bottom", "StockCharts: Rounding Bottom; Bulkowski: Rounding Bottom", ""),
    ("chart_semi_cup", "Giorgos Siligardos: Identifying Cup Formations Early (TASC 29:4, April 2011); thinkorswim: SemiCupFormation", ""),
    ("cdl_judas", "Judas Candle", ""),
    ("cdl_falter_reverse", "TradeStation: Falter & Reverse Dn / Up indicator", ""),
];

#[cfg(test)]
mod tests {
    use super::{DEFS, SOURCES};
    use crate::lab::catalogue::{get, run, Params};
    use crate::lab::testdata::walk;

    #[test] fn every_entry_is_listed_with_a_note_and_a_source() {
        assert_eq!(DEFS.len(), 44);
        for d in DEFS {
            assert!(d.compute.is_none(), "{}", d.id);
            assert!(d.note.starts_with("not available yet — needs ") || d.note.starts_with("not available yet — formula not published ("), "{}: {}", d.id, d.note);
            // a data note must come with a data flag; an unpublished formula with none
            assert_eq!(d.needs.any_data(), d.note.starts_with("not available yet — needs "), "{}", d.id);
            assert_eq!(SOURCES.iter().filter(|s| s.0 == d.id).count(), 1, "{}", d.id);
            assert_eq!(get(d.id).map(|g| g.id), Some(d.id));
        }
        assert_eq!(SOURCES.len(), DEFS.len());
    }
    #[test] fn running_a_listed_entry_returns_its_note() {
        let d = get("cdl_judas").unwrap();
        assert_eq!(run(&d, &walk(30, 1, true), &Params::new()).unwrap_err().to_string(), d.note);
    }
}
