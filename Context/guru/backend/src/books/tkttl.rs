//! Thomas K. Carr ("Dr. Stoxx"), "Trend Trading for a Living" (McGraw-Hill, 2008). Ten stock setups (ch. 9-10),
//! one entry method and a menu of exits (ch. 11). Every setup mixes chart judgement (trendlines, bases, "pivot high
//! ... sell-off or sideways consolidation", candle reading, a final eyeball screening) with mechanical parts, and the
//! exits are four stop methods and four profit methods "to determine which strategy best fits your trading style",
//! so no setup is fully stated. All are listed as not testable. Sources: library/tkttl/.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

const BOOK: &str = "tkttl";
const EXIT: &str = "X|Exit menu (ch. 11, p.229-237): percentage stop 'no greater than -8 percent', price-pattern stop, 2 x ATR(14) stop, parabolic SAR stop; percentage target 1.5-2.0 x the stop, support/resistance target, 3 x ATR target, trailing stop 'on a closing basis'; 'Try each one for yourself to determine which strategy best fits your trading style' (p.229) - no exit is assigned to any setup";
const ENTRY_L: &str = "E|\"enter the trade on a stop order set at either $0.02 (for slower-moving stocks) or $0.05 (for faster-moving stocks) above the intraday high of the previous trading day\"; gap above it: limit order at that price (p.228)";
const ENTRY_S: &str = "E|\"stop order to sell at either $0.02 (for slower-moving stocks) or $0.05 (for faster-moving stocks) below the intraday low of the previous trading day\"; gap below: limit at that price, or pay up (p.228)";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, dir, why, rules, notes| StrategyDef {
        id, name, group, session: 0, dir, status: Status::NotTestable(why), needs_volume: true, needs_vix: false,
        needs_series: &[], book: BOOK, rules, notes, claim: None,
    };
    let bull = "Ch. 9 · Bullish setups";
    let bear = "Ch. 10 · Bearish setups";
    vec![
        d("tkttl_pullback", "The Pullback", bull, "Long",
            "uptrend defined as 'making higher lows ... over at least the past three months' and '20 MA mostly rising' (judgement); bullish reversal candle 'of some kind'; no assigned exit",
            &["F|Uptrending stock pulled back into an upsloping 20 MA, between the 20 and 50 MA or to the 50 MA (p.159)", "F|Stochastics (5,3) dips to or below 20 (p.159)", "E|Bullish reversal candle (hammer, engulfing, piercing, doji) (p.159-160)", ENTRY_L, EXIT],
            "Screens p.160-162 are candidate lists for eyeballing, not the setup."),
        d("tkttl_coiled_spring", "The Coiled Spring", bull, "Long",
            "trendlines over the top and bottom of a narrowing 7-20 day consolidation; buy on 'a break of the trendline drawn over the tops' (chart drawing)",
            &["F|Weakly/strongly uptrending stock with a new 3-month high within 20 days (p.165)", "F|Coil 7-20 days, tightening, not angled up, not touching the 50 MA (p.165-166)", "E|Break of the upper trendline (p.166)", EXIT], "Not testable."),
        d("tkttl_bull_divergence", "The Bullish Divergence", bull, "Long",
            "'clear price lows', divergence 'in two or more' indicators at unstated settings, bullish candle 'of some kind'",
            &["F|50 MA above a rising 200 MA (p.171)", "F|Price below the 50 MA with two lows at least five days apart; higher low in two of MACD, histogram, stochastics, RSI, OBV, CCI (p.171-172)", "E|Bullish candle (p.172)", EXIT], "Not testable."),
        d("tkttl_blue_sky", "The Blue Sky Breakout", bull, "Long",
            "rule 1 needs a 'previous new pivot high' = 'a new price high followed by a sell-off or sideways consolidation of at least five bars' (lookback of that new high not given; the figure note says 'at least three down days'); exits are a menu",
            &["F|\"First, the closing price of the stock ... must register a new high following a previous new pivot high set within the past 20 trading days\" (p.176)", "F|No higher closing high in at least the past three months (p.177)", "F|New closing high / 52-week low <= 3.0 (p.177)", "F|Highest OBV in at least the past three months (p.177)", "F|Green candle (close > open) (p.178)", "E|\"When all five of these parameters have been met on the same day, we have a buy signal.\" (p.178)", ENTRY_L, EXIT],
            "AAPL, CREE examples (+22% in 12 days), no statistics."),
        d("tkttl_bull_base", "The Bullish Base Breakout", bull, "Long",
            "triangle / rectangle base identification and an OBV trendline break are chart drawing",
            &["F|Downtrending stock in a 30-day+ consolidation base (p.182-183)", "F|MACD higher lows; OBV breaks a trendline over its tops (p.184)", "E|First green candle after the OBV break (p.184)", EXIT], "Not testable."),
        d("tkttl_relief_rally", "The Relief Rally", bear, "Short",
            "mirror of the pullback: judgement trend definition and bearish candle 'of some kind'", &["F|Downtrending stock rallying into a downsloping 20/50 MA, stochastics at or above 80, bearish candle (ch. 10)", ENTRY_S, EXIT], "Not testable."),
        d("tkttl_bear_divergence", "The Bearish Divergence", bear, "Short",
            "mirror of the bullish divergence: 'clear price highs', indicator divergences at unstated settings", &["F|50 MA below a falling 200 MA, two price highs, lower highs in two indicators, bearish candle (ch. 10)", ENTRY_S, EXIT], "Not testable."),
        d("tkttl_gap_down", "The Gap Down", bear, "Short",
            "'The 50 MA is rising steadily without any major dips' is judgement; no exit assigned",
            &["F|Steady uptrend: 50 MA rising steadily, price above the 50 MA for at least 40 trading days (p.204)", "F|Unfilled gap down: today's high below yesterday's low (p.204)", "E|Short when price trades below the gap-down candle's low; negated if price trades into the gap first (p.204)", EXIT], "Not testable."),
        d("tkttl_blue_sea", "The Blue Sea Breakdown", bear, "Short",
            "rule 1 needs 'a previous new low set within the past 20 trading days' with no lookback for that low; exits are a menu",
            &["F|New closing low following a previous new low within 20 days (p.208)", "F|No lower closing low in three months (p.209)", "F|52-week high / new closing low <= 2.0 (p.209)", "F|Lowest OBV in three months (p.209)", "F|Red candle (p.209)", ENTRY_S, EXIT], "Not testable."),
        d("tkttl_rising_wedge", "The Rising Wedge Breakdown", bear, "Short",
            "wedge trendlines and an OBV trendline break are chart drawing", &["F|Rising wedge in an uptrend; OBV breaks a trendline under its lows (ch. 10)", "E|Sell short signal on the break (p.218)", EXIT], "Not testable."),
        d("tkttl_market_type", "Market-type filter (ch. 7)", "Ch. 7 · Determining general market direction", "Long/Short",
            "'20 MA mostly above', 'distance growing', 'rallies reach only to the 20 MA' are visual judgements; the chapter assigns setups to market types, it is not a timing system",
            &["F|Bullish strongly trending: 20 MA above 50 MA, both rising, distance growing (ch. 7)", "F|Bearish strongly trending: 20 MA below 50 MA, both falling (ch. 7)"], "Not testable."),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    if !id.starts_with("tkttl_") { return None; }
    Some(("tkttl_setups", "Carr trend-trading setups (not testable)", ""))
}

pub fn params(_id: &str) -> Option<Vec<ParamDef>> { None }

pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }

#[cfg(test)]
mod tests {
    #[test]
    fn every_setup_is_not_testable() {
        assert!(super::catalogue().iter().all(|d| matches!(d.status, crate::connors::Status::NotTestable(_))));
    }
}
