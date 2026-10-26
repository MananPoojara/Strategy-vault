//! Michael Covel's books and the Turtle rules they document.
//! Sources: The Original Turtle Trading Rules (originalturtles.org, Curtis Faith 2003; reprinted as the appendix of
//! Way of the Turtle pp.249-270), Way of the Turtle ch.10 (six Trading Blox test systems), Trend Following (Updated Ed.)
//! App. F (Mechanica 89/13 sample system), Trend Commandments p.60-61 (two reversal examples), The Complete
//! TurtleTrader ch.5 (Covel's paraphrase of S1/S2). Page refs, testability table and the oracle: library/covel/.
//! Trend Following Manifesto course (TF_MORE archive): Alternative Trend Following Entries guide, Weekly Bar and Donchian 5&20
//! system sheets, Money Management Guide, Trade Walk Through. Page refs and oracle: library/covel2/.
//! Single-index model: one position per market, unit weights = fraction of equity (1 unit = risk% of equity per N).
use crate::connors::{atr, sma, true_range, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "covel";
/// Admitted ids (authors/covel/laneb/SOURCE-CONTRACT.md).
pub const REVIEWED_COVEL: &[&str] = &["cov_tc_100day_reversal", "cov_tc_4week_reversal", "cov_tf_mechanica_89_13", "cov_tfm_bollinger_75", "cov_tfm_sma_75", "cov_tfm_ma_cross_50_75", "cov_tfm_hh75_ll25"];
pub fn reviewed_covel(id: &str) -> bool { REVIEWED_COVEL.contains(&id) }
/// Admitted ids (authors/faith/admission/SOURCE-CONTRACT.md).
pub const REVIEWED: &[&str] = &["cov_wott_atr_cbo", "cov_wott_bollinger_cbo", "cov_wott_donchian_trend", "cov_wott_donchian_time", "cov_wott_dual_ma", "cov_wott_triple_ma"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const TURTLE_NOTE: &str = "Futures rules run on the index (instrument swap). N = Turtle N: (19*PDN+TR)/20 seeded with the 20-day simple average of TR (OTR p.14). Unit = 1% of equity per N, so each unit's weight = 1% x fill / N (N of the day before the breakout, frozen for the trade's adds and stops, as in the book's worked examples). Stop/limit fills: at the level, or at the open when the market opens through it. Daily-bar order (book silent): on the entry bar entry -> adds -> stops/exit vs the bar's adverse extreme; on later bars open gaps -> stops/exit -> adds; no re-entry on an exit bar. 1 tick = 0.05 points (NSE). Not modelled (need several futures): correlated/direction unit limits, buy-strongest, rolls; contract truncation; the notional-account 20% cut after 10% loss (yearly reset is Rich's subjective call).";
const WOTT_NOTE: &str = "Way of the Turtle ch.10 test system (27 US futures, Jan 1996-Jun 2006, Trading Blox). Run on one index (instrument swap). Size = 0.5% of equity per ATR (p.136), weight = 0.5% x fill / ATR of the signal day. ATR = the book's N (\"N is simply the 20-day exponential moving average of the True Range, which is now more commonly known as the ATR\", p.252). Table 10-1 is a portfolio result: not comparable to a single index.";
const PORTFOLIO_GAP: &str = "Needs several correlated futures markets traded together: unit limits per closely correlated group (6), loosely correlated (10) and per direction (12), and buying the strongest / shorting the weakest market of a group. Only single indices exist in the vault.";
const WHIPSAW_GAP: &str = "Book states the stop (1/2 N per unit) and that a stopped unit is re-entered at its original entry price, but not the re-entered unit's stop, whether re-entries count toward the 4-unit limit, or whether re-entry continues after the 10/20-day exit price was hit. Not coded rather than invent these.";
const STOCKS_GAP: &str = "Needs the 24,000-stock US universe with delisted names and the price/liquidity filters (values not printed). Also the rule is incomplete in the book: the ATR lookback and what the 10 ATR trailing stop trails (highest close? highest high?) are not stated.";

const TFM_NOTE: &str = "Trend Following Manifesto course, Alternative Trend Following Entries guide (stocks/futures/forex chart examples, no published results). Run on one index (instrument swap). Signal from the day's bar, executed at the next open (the course's Trade Walk Through p.2: \"The trading signal was given on 1/2/2013 ... The shares were bought on the open of the next trading day\"). Full position, one unit: the course's 2-ATR initial stop and pyramiding overlay is not coded (see cov_tfm_atr_pyramid). Moving averages are simple (the guide: \"simple moving average\").";
const WEEKLY_BAR_GAP: &str = "The sheet gives no exact numbers for the rules it describes: channel = \"6-month\" (no day count), initial stop at \"various multiples (but no less than 4)\" of the daily range, ATR lookback not given, and the opposite-direction channel used as trailing stop has no length. Its \"More variations\" (100-day and four-week reversals) are already in the vault as cov_tc_100day_reversal / cov_tc_4week_reversal.";
const DONCHIAN_GAP: &str = "The source text is incomplete: Rule F's exception test ends mid-sentence (\"any 5 DMA penetration (in either direction) of the last\" - the lookback is missing, p.3), and the unit table's top row (\"400.00 and above 0.002\") breaks the table's own progression (0.20 for 120-400), so one full unit on an index (> 400) is unreliable. Rules A/B/C/F/G form one decision tree; coding part of it would invent the rest.";
const PYRAMID_GAP: &str = "The course states a 2% risk per trade, a 2-ATR stop moved with each pyramid, and a switch to the entry's exit once the stop reaches breakeven (Alt. Entries p.18, p.21), but its own examples disagree on the numbers a program needs: ATR = 15-day average of TR blended 1/15 with the prior day (MM Guide p.3-5) vs a 20-day ATR (Walk Through p.2); add every 1 ATR (MM p.15, Walk Through) vs every 0.50 ATR (MM p.22); 4 pyramids (Alt. Entries p.21) vs 6 units (MM p.15-29) vs 3 (Walk Through p.3); and the last add is placed (MM p.20) or skipped (Walk Through p.3) when the 2-ATR stop passes breakeven.";
const MM_PORTFOLIO_GAP: &str = "Total unit risk = largest side's units - smallest side's units / 2, excluding positions whose stop is at breakeven (MM Guide p.95-97): needs several markets held together; the vault runs one index at a time.";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, dir, status, rules, notes| StrategyDef {
        id, name, group, session: 0, dir, status, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None,
    };
    use Status::*;
    vec![
        d("cov_turtle_s1", "Turtle System 1 (20/10, last-breakout filter, 55 failsafe)", "Original Turtle Rules · Ch 4-6", "Long/Short", Live, &[
            "E|\"Turtles entered positions when the price exceeded by a single tick the high or low of the preceding 20 days\" (buy stop 1 tick over the 20-day high / sell stop 1 tick under the 20-day low; \"the Turtles would enter positions on the open if a market opened through the price of the breakout\") OTR p.19-20",
            "F|\"System 1 breakout entry signals would be ignored if the last breakout would have resulted in a winning trade ... irrespective of whether or not that particular breakout was actually taken, or was skipped ... a losing breakout if the price subsequent to the date of the breakout moved 2N against the position before a profitable 10-day exit occurred. The direction of the last breakout was irrelevant\" p.20",
            "E|\"in the event that a System 1 entry breakout was skipped because the previous trade had been a winner, an entry would be made at the 55-day breakout\" (Failsafe) p.20",
            "A|\"added to those positions at 1/2 N intervals following their initial entry ... based on the actual fill price of the previous order\" up to 4 Units (single-market limit, p.17) p.20-21",
            "X|\"Turtle stops were set at 2 N below the entry for long positions, and 2 N above the entry for short positions ... if additional units were added, the stops for earlier units were raised by 1/2 N\" p.23",
            "X|\"The System 1 exit was a 10 day low for long positions and a 10 day high for short positions. All the Units in the position would be exited\" p.27",
            "F|N = (19 x PDN + TR)/20, started from a 20-day simple average of TR; Unit = 1% of Account / (N x Dollars per Point) p.14-15",
        ], TURTLE_NOTE),
        d("cov_turtle_s2", "Turtle System 2 (55/20)", "Original Turtle Rules · Ch 4-6", "Long/Short", Live, &[
            "E|\"Entered when the price exceeded by a single tick the high or low of the preceding 55 days\" OTR p.20",
            "F|\"All breakouts for System 2 would be taken whether the previous breakout had been a winner or not.\" p.20",
            "A|Add 1 Unit every 1/2 N from the previous fill, maximum 4 Units p.17, 20-21",
            "X|2N stop from each unit's entry; earlier units' stops raised by 1/2 N per added unit p.23",
            "X|\"The System 2 exit was a 20 day low for long positions and a 20 day high for short positions. All the Units in the position would be exited\" p.27",
        ], TURTLE_NOTE),
        d("cov_turtle_whipsaw", "Turtle Whipsaw stop (alternate)", "Original Turtle Rules · Ch 5", "Long/Short", NotTestable(WHIPSAW_GAP), &[
            "X|\"stops were placed at 1/2 N for 1/2% account risk. If a given Unit was stopped out, the Unit would be re-entered if the market reached the original entry price.\" OTR p.24",
        ], WHIPSAW_GAP),
        d("cov_turtle_portfolio", "Turtle portfolio unit limits & buy-strength", "Original Turtle Rules · Ch 3, 7", "Long/Short", NeedsData(PORTFOLIO_GAP), &[
            "F|Levels: single market 4 Units, closely correlated markets 6, loosely correlated 10, single direction 12 (OTR p.17)",
            "F|\"If the signals came all at once, we always bought the strongest markets and sold short the weakest markets in a group\" p.30",
        ], PORTFOLIO_GAP),
        d("cov_wott_atr_cbo", "ATR Channel Breakout (350 MA +7/-3 ATR)", "Way of the Turtle · Ch 10", "Long/Short", Live, &[
            "E|\"The channel is formed by adding 7 ATR to a 350-day moving average of the closing prices for the channel top and subtracting 3 ATR from the moving average for the channel bottom. A long trade is entered on the open if the previous day's close exceeded the top of the channel; a short trade is entered if the previous day's close fell below the bottom\" WotT p.136",
            "X|\"Trades are exited when the close crosses back through the moving average price.\" (at the next open, like the entries; \"systems shown here that buy on the open based on closing price data\" p.213) p.136",
        ], WOTT_NOTE),
        d("cov_wott_bollinger_cbo", "Bollinger Breakout (350 MA +/-2.5 SD)", "Way of the Turtle · Ch 10", "Long/Short", Live, &[
            "E|\"formed by adding and subtracting 2.5 standard deviations of the close to a 350-day moving average. A long trade is entered on the open if the previous day's close exceeded the top of the channel; a short trade ... below the bottom\" WotT p.138 (SD over the same 350 days: a Bollinger band uses one lookback; population SD)",
            "X|\"Trades are exited when the close crosses back through the moving average price.\" (next open) p.138",
        ], WOTT_NOTE),
        d("cov_wott_donchian_trend", "Donchian Trend (20/10 + 25/350 EMA filter, 2-ATR stop)", "Way of the Turtle · Ch 10", "Long/Short", Live, &[
            "E|\"uses a 20-day breakout for entry\" (buy when the price exceeds the highest high of the previous 20 days, short below the lowest low, p.68) WotT p.139",
            "F|\"350-day/25-day exponential moving average trend filter ... If the 25-day moving average is above the 350-day average, only longs may be taken; if below, only shorts\" p.139 (prior close)",
            "X|\"a 10-day breakout for exits\" p.139",
            "X|\"The system also uses a 2-ATR stop just as the original Turtle system did.\" p.139",
        ], WOTT_NOTE),
        d("cov_wott_donchian_time", "Donchian Trend with 80-day Time Exit", "Way of the Turtle · Ch 10", "Long/Short", Live, &[
            "E|Donchian Trend entry: 20-day breakout with the 25/350-day EMA trend filter WotT p.139-140",
            "X|\"uses a time-based exit instead of a breakout exit. It exits the trade after 80 days and does not use any stops whatsoever.\" p.140 (close of the 80th trading day after entry)",
        ], WOTT_NOTE),
        d("cov_wott_dual_ma", "Dual Moving Average (100/350, always in)", "Way of the Turtle · Ch 10", "Long/Short", Live, &[
            "E|\"buys and sells when a 100-day moving average crosses over a slower 350-day moving average ... always in the market, either long or short\" WotT p.141 (simple MAs: the book's unqualified 'moving average' is the simple one, p.126; acts at the next open, p.213)",
            "X|\"The only time a trade is exited is when the fast-moving average crosses over, at which time the trade is exited and a new trade is initiated in the opposite direction.\" p.141",
        ], WOTT_NOTE),
        d("cov_wott_triple_ma", "Triple Moving Average (150/250 with 350 filter)", "Way of the Turtle · Ch 10", "Long/Short", Live, &[
            "E|\"The buys and sells occur when the 150-day moving average crosses over a slower 250-day moving average ... Trades happen only when both moving averages are on the same side as the longer 350-day average\" WotT p.142",
            "X|\"Trades are exited when the 150-day average crosses the 250-day average.\" p.142 (next open)",
        ], WOTT_NOTE),
        d("cov_tf_mechanica_89_13", "Mechanica sample system (89-day breakout / 13-day exit)", "Trend Following · App. F", "Long/Short", Live, &[
            "E|\"COL2 = MAX[H,89,1] + TICK[1] '89-day breakout for long entry ... COL4 = MIN[L,89,1] - TICK[1]\" BUYSTOP/SELLSTOP TF p.387",
            "X|\"COL3 = MIN[L,13,1] - TICK[1] '13-day breakdown for long exit ... COL5 = MAX[H,13,1] + TICK[1]\" p.387",
            "F|Size: lesser of 2% equity / |entry - stop| and 2% equity / (2 x 15-day average true range) p.387 (the 100-contract cap has no index equivalent)",
        ], "Trading Recipes code, 15 futures 1991-2000 (portfolio result, not comparable). Run on one index. ATR[15] = 15-day simple average of true range, yesterday's value. Weight = 2% x fill / max(|fill - stop|, 2 x ATR15). Stop entries fill at the level or the open if gapped. 1 tick = 0.05."),
        d("cov_tc_100day_reversal", "100-day breakout reversal (always in)", "Trend Commandments · Trading Systems", "Long/Short", Live, &[
            "E|\"You enter a long position (and exit a short position) if the current price exceeds the highest price in the previous 100 days. You enter a short position (and exit a long position) if the current price drops below the lowest price in the previous 100 days.\" TC p.60",
            "X|\"It is always in the market. It closes one position by opening a new one in the opposite direction.\" (exit = the opposite 100-day breakout) TC p.60",
        ], "Stop-and-reverse at the breakout price (1 tick = 0.05 beyond the 100-day extreme; the open if gapped). Full position (the volatility sizing paragraph gives no number). At most one reversal per bar."),
        d("cov_tc_4week_reversal", "Four-week rule reversal", "Trend Commandments · Trading Systems", "Long/Short", Live, &[
            "E|\"You buy when a market makes a new four-week high or sell when it makes a new four-week low. If you are long, you go short if it makes a new four-week low. If you are short, you go long if it makes a new four-week high.\" TC p.60-61",
            "X|Exit = the opposite four-week breakout (stop-and-reverse, \"closes one position by opening a new one in the opposite direction\") TC p.60",
        ], "Four weeks = 20 trading days (Covel, Complete TurtleTrader p.72: \"A two-week low was a ten-day breakout - counting trading days only\"). Stop-and-reverse at 1 tick beyond the 20-day extreme (open if gapped), full position."),
        d("cov_tf_stocks_ath", "All-time-high entry with 10 ATR trailing stop (Blackstar)", "Trend Following · App. A", "Long", NotTestable(STOCKS_GAP), &[
            "E|\"if today's close is greater than or equal to the highest close during the stock's entire history, then buy tomorrow on the open\" TF p.309",
            "X|10 ATR trailing stop; \"exit a stock on the open the day after the exit level was breached\" p.312",
        ], STOCKS_GAP),
        d("cov_tfm_bollinger_75", "Bollinger Band entry & midpoint exit (75 days, 2 SD)", "TF Manifesto · Alternative Entries", "Long/Short", Live, &[
            "E|\"If the market closes above the upper Bollinger Band then you enter a long position. If the market closes below the lower Bollinger Band then you enter a short position.\" Alt. Entries p.18",
            "X|\"Typically traders use a move back to the Midpoint as an exit.\" p.18 (close back through the middle average)",
            "F|\"Bollinger Band - 75 day in length with a +2 upper standard deviation and a -2 standard deviation\" p.22, 27, 32, 37, 42, 47; \"Avoid using one standard deviation length for the upper band and another for the lower band\" p.5",
        ], TFM_NOTE),
        d("cov_tfm_sma_75", "Single moving average entry & exit (75 days, always in)", "TF Manifesto · Alternative Entries", "Long/Short", Live, &[
            "E|\"As Soybeans closed above the simple moving average in August 2003 you entered the market.\" Alt. Entries p.16; short: \"a close below the single moving average\" p.10",
            "X|\"When the market broke below the average you exited the trade.\" p.16 (close below; \"Stick to one approach\" p.16) - the close below is also the short entry",
            "F|\"Single Moving Average Crossover - 75 days in length\" p.22",
        ], TFM_NOTE),
        d("cov_tfm_ma_cross_50_75", "Moving average crossover (50/75, always in)", "TF Manifesto · Alternative Entries", "Long/Short", Live, &[
            "E|\"Entering on a crossover requires the trader wait until the shorter term moving average crosses above the longer term moving average ... If the short term average is below the long term average then the direction is down, signaling a short entry.\" Alt. Entries p.17",
            "X|Exit on the counter crossover: \"switch to a counter crossover (the short term average crossing below the long term average)\" p.21",
            "F|\"Moving Average Crossover - 50 days for the fast moving average and 75 days for the slow moving average\" p.22",
        ], TFM_NOTE),
        d("cov_tfm_hh75_ll25", "75-day high entry / 25-day low exit", "TF Manifesto · Alternative Entries", "Long/Short", Live, &[
            "E|\"if today's high is greater than the highest high of the last x period then enter the market\" Alt. Entries p.13; short: \"a simple break below the lowest low of x period\" p.14",
            "E|\"The blue dots represent a new 75 day high. The first blue dot signals an entry into the market.\" p.15",
            "X|\"As the trade progressed a 25 day low, the red dot, signaled a trade exit.\" p.15 (short: a 25-day high)",
            "F|Variant (param close = 1): \"wait for a close above the highest high of the last x time frame\" p.13",
        ], TFM_NOTE),
        d("cov_tfm_weekly_bar", "Weekly Bar system (6-month channel, ATR money stop)", "TF Manifesto · Weekly Bar system", "Long/Short", NotTestable(WEEKLY_BAR_GAP), &[
            "E|\"Use a 6-month high-low price channel breakout ... When a market price closes above or below the channel initiate a unit.\" Weekly Bar p.2",
            "X|\"The initial stop is set at various multiples (but no less than 4) of the daily price range from the current price.\" p.2",
            "A|\"Add to winning positions when the price moves in your favor by at least one daily price range ... in 5-steps\" p.2",
            "X|\"Once you have a full unit, you start to use channel breakout in opposite direction stops\" p.2",
        ], WEEKLY_BAR_GAP),
        d("cov_tfm_donchian_5_20", "Donchian 5 & 20 system", "TF Manifesto · Donchian Trading System", "Long/Short", NotTestable(DONCHIAN_GAP), &[
            "E|Rule A: \"Has TC crossed the 20 DMA and penetrated by one full unit or more the maximum of the previous 20 DMA crossing in the same direction no matter how long ago?\" Donchian p.2",
            "E|Rule B: crossing began within the previous twenty trading days and \"TC exceed by one full unit the previous fifteen closes\" p.2; Rule C: \"TC exceed by one full unit the previous twenty-five closes\" p.3",
            "X|Rule F: TC crossed the 5 DMA against the CBS ... exception ends \"of the last\" (text missing) p.3; Rule G re-entry p.3; signal timing and unit table p.3-4",
        ], DONCHIAN_GAP),
        d("cov_tfm_atr_pyramid", "ATR stop & pyramiding overlay (2% risk, 2 ATR)", "TF Manifesto · Money Management", "Long/Short", NotTestable(PYRAMID_GAP), &[
            "X|\"As a trend follower you are using a 2 ATR stop, moved with each pyramid entry. Once the stop comes to break even then you wait for a move against the position.\" Alt. Entries p.18",
            "A|\"You are going to pyramid 4 times into this position ... Once the ATR stop is equal to your average trade entry you can switch to a counter crossover\" p.21",
        ], PYRAMID_GAP),
        d("cov_tfm_portfolio_risk", "Total unit risk across a portfolio", "TF Manifesto · Money Management", "Long/Short", NeedsData(MM_PORTFOLIO_GAP), &[
            "F|\"Sum the total units of long risk ... short risk ... Divide the smallest number of risk units by 2 ... Subtract the result from the largest number of risk units.\" MM Guide p.95",
        ], MM_PORTFOLIO_GAP),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "cov_turtle_s1" => ("cov_turtle", "Turtle Trading Rules", "System 1"),
        "cov_turtle_s2" => ("cov_turtle", "Turtle Trading Rules", "System 2"),
        "cov_turtle_whipsaw" => ("cov_turtle", "Turtle Trading Rules", "Whipsaw stop · not testable"),
        "cov_turtle_portfolio" => ("cov_turtle", "Turtle Trading Rules", "Portfolio limits · data missing"),
        "cov_wott_atr_cbo" => ("cov_wott", "Way of the Turtle systems", "ATR Channel Breakout"),
        "cov_wott_bollinger_cbo" => ("cov_wott", "Way of the Turtle systems", "Bollinger Breakout"),
        "cov_wott_donchian_trend" => ("cov_wott", "Way of the Turtle systems", "Donchian Trend"),
        "cov_wott_donchian_time" => ("cov_wott", "Way of the Turtle systems", "Donchian Time Exit"),
        "cov_wott_dual_ma" => ("cov_wott", "Way of the Turtle systems", "Dual MA"),
        "cov_wott_triple_ma" => ("cov_wott", "Way of the Turtle systems", "Triple MA"),
        "cov_tf_mechanica_89_13" => ("cov_tf", "Trend Following examples", "Mechanica 89/13"),
        "cov_tf_stocks_ath" => ("cov_tf", "Trend Following examples", "Stocks ATH · not testable"),
        "cov_tc_100day_reversal" => ("cov_tc", "Trend Commandments reversals", "100-day"),
        "cov_tc_4week_reversal" => ("cov_tc", "Trend Commandments reversals", "Four-week rule"),
        "cov_tfm_bollinger_75" => ("cov_tfm", "Trend Following Manifesto entries", "Bollinger 75/2"),
        "cov_tfm_sma_75" => ("cov_tfm", "Trend Following Manifesto entries", "75-day SMA"),
        "cov_tfm_ma_cross_50_75" => ("cov_tfm", "Trend Following Manifesto entries", "50/75 MA cross"),
        "cov_tfm_hh75_ll25" => ("cov_tfm", "Trend Following Manifesto entries", "75-day high / 25-day low"),
        "cov_tfm_weekly_bar" => ("cov_tfm", "Trend Following Manifesto entries", "Weekly Bar · not testable"),
        "cov_tfm_donchian_5_20" => ("cov_tfm", "Trend Following Manifesto entries", "Donchian 5&20 · not testable"),
        "cov_tfm_atr_pyramid" => ("cov_tfm", "Trend Following Manifesto entries", "ATR stop & pyramids · not testable"),
        "cov_tfm_portfolio_risk" => ("cov_tfm", "Trend Following Manifesto entries", "Portfolio unit risk · data missing"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let tick = || p("tick", "One tick in index points (book: 'a single tick'; NSE 0.05)", 0.05, 0.0, 1.0, 0.05);
    let nlen = || p("n_len", "N / ATR days", 20.0, 10.0, 40.0, 5.0);
    let turtle = |e: f64, x: f64| vec![p("entry", "Entry breakout days", e, 10.0, 100.0, 5.0), p("exit", "Exit breakout days", x, 5.0, 50.0, 5.0),
        p("stop_n", "Stop distance (N)", 2.0, 0.5, 4.0, 0.5), p("add_n", "Add a unit every (N)", 0.5, 0.25, 2.0, 0.25),
        p("units", "Maximum units", 4.0, 1.0, 5.0, 1.0), p("risk", "Unit size: % of equity per N", 1.0, 0.25, 2.0, 0.25), nlen(), tick()];
    let wott = || vec![p("risk", "% of equity per ATR", 0.5, 0.25, 2.0, 0.25), nlen()];
    let v = match id {
        "cov_turtle_s1" => { let mut v = turtle(20.0, 10.0); v.push(p("failsafe", "Failsafe breakout days", 55.0, 30.0, 100.0, 5.0)); v.push(p("filter", "Skip after a winning last breakout (1 = book)", 1.0, 0.0, 1.0, 1.0)); v }
        "cov_turtle_s2" => turtle(55.0, 20.0),
        "cov_wott_donchian_trend" | "cov_wott_donchian_time" => {
            let mut v = vec![p("entry", "Entry breakout days", 20.0, 10.0, 60.0, 5.0), p("fast", "Fast EMA days", 25.0, 10.0, 100.0, 5.0), p("slow", "Slow EMA days", 350.0, 150.0, 500.0, 50.0), tick()];
            v.extend(if id.ends_with("trend") { vec![p("exit", "Exit breakout days", 10.0, 5.0, 40.0, 5.0), p("stop_n", "Stop (ATR)", 2.0, 0.5, 6.0, 0.5)] } else { vec![p("hold", "Exit after (trading days)", 80.0, 20.0, 200.0, 10.0)] });
            v.extend(wott()); v
        }
        "cov_wott_atr_cbo" => { let mut v = vec![p("ma", "Close MA days", 350.0, 100.0, 500.0, 50.0), p("top", "Channel top (ATR)", 7.0, 1.0, 10.0, 0.5), p("bottom", "Channel bottom (ATR)", 3.0, 1.0, 10.0, 0.5)]; v.extend(wott()); v }
        "cov_wott_bollinger_cbo" => { let mut v = vec![p("ma", "MA / SD days", 350.0, 100.0, 500.0, 50.0), p("k", "Band width (SD)", 2.5, 1.0, 4.0, 0.25)]; v.extend(wott()); v }
        "cov_wott_dual_ma" => { let mut v = vec![p("fast", "Fast MA days", 100.0, 20.0, 200.0, 10.0), p("slow", "Slow MA days", 350.0, 150.0, 500.0, 50.0)]; v.extend(wott()); v }
        "cov_wott_triple_ma" => { let mut v = vec![p("fast", "Fast MA days", 150.0, 50.0, 250.0, 10.0), p("mid", "Middle MA days", 250.0, 100.0, 350.0, 10.0), p("slow", "Trend-filter MA days", 350.0, 200.0, 500.0, 50.0)]; v.extend(wott()); v }
        "cov_tf_mechanica_89_13" => vec![p("entry", "Entry breakout days", 89.0, 20.0, 200.0, 1.0), p("exit", "Exit breakdown days", 13.0, 5.0, 50.0, 1.0),
            p("atr", "ATR days", 15.0, 5.0, 40.0, 1.0), p("risk", "% of equity risked", 2.0, 0.5, 4.0, 0.5), tick()],
        "cov_tc_100day_reversal" => vec![p("n", "Breakout days", 100.0, 20.0, 250.0, 10.0), tick()],
        "cov_tc_4week_reversal" => vec![p("n", "Breakout days (4 weeks)", 20.0, 10.0, 60.0, 5.0), tick()],
        "cov_tfm_bollinger_75" => vec![p("ma", "Band length (days)", 75.0, 20.0, 200.0, 5.0), p("k", "Band width (SD, both bands)", 2.0, 1.0, 4.0, 0.25)],
        "cov_tfm_sma_75" => vec![p("ma", "Moving average days", 75.0, 20.0, 200.0, 5.0)],
        "cov_tfm_ma_cross_50_75" => vec![p("fast", "Short-term MA days", 50.0, 10.0, 150.0, 5.0), p("slow", "Long-term MA days", 75.0, 20.0, 250.0, 5.0)],
        "cov_tfm_hh75_ll25" => vec![p("entry", "Entry: highest high / lowest low days", 75.0, 20.0, 200.0, 5.0), p("exit", "Exit: opposite extreme days", 25.0, 5.0, 100.0, 5.0),
            p("close", "Signal on the close beyond the level (1) instead of the day's high/low (0 = book example)", 0.0, 0.0, 1.0, 1.0)],
        _ => return None,
    };
    Some(v)
}

/// None if a parameter is unknown, out of range, or a day count is not an integer.
pub fn validate(id: &str, p: &Params) -> Option<String> {
    let schema = params(id)?;
    for (k, &v) in p {
        let Some(d) = schema.iter().find(|d| d.key == k) else { return Some(format!("unknown parameter {k}")) };
        let int = d.def.fract() == 0.0 && d.step.fract() == 0.0;
        if !v.is_finite() || v < d.min || v > d.max || (int && v.fract() != 0.0) { return Some(format!("{k} must be in {}..{}", d.min, d.max)); }
    }
    None
}

/// EMA (alpha 2/(n+1)) seeded with the simple average of the first n values.
fn ema(v: &[f64], n: usize) -> Vec<f64> {
    let mut out = vec![f64::NAN; v.len()];
    if v.len() < n { return out; }
    let (a, mut e) = (2.0 / (n as f64 + 1.0), v[..n].iter().sum::<f64>() / n as f64);
    out[n - 1] = e;
    for i in n..v.len() { e += a * (v[i] - e); out[i] = e; }
    out
}
fn hh(x: &Ctx, i: usize, n: usize) -> f64 { x.h[i - n..i].iter().copied().fold(f64::MIN, f64::max) }
fn ll(x: &Ctx, i: usize, n: usize) -> f64 { x.l[i - n..i].iter().copied().fold(f64::MAX, f64::min) }

/// Stop entry on bar i at long/short trigger levels (NaN = side not allowed). Gap through = open fill.
/// Both touched on one bar (book silent): the side gapped through, else the trigger nearer the open.
fn stop_entry(x: &Ctx, i: usize, up: f64, dn: f64) -> Option<(f64, f64)> {
    let o = x.o[i];
    let (u, d) = (x.h[i] >= up, x.l[i] <= dn);
    match (u, d) {
        (true, false) => Some((1.0, o.max(up))),
        (false, true) => Some((-1.0, o.min(dn))),
        (true, true) if o >= up => Some((1.0, o)),
        (true, true) if o <= dn => Some((-1.0, o)),
        (true, true) => Some(if up - o <= o - dn { (1.0, up) } else { (-1.0, dn) }),
        _ => None,
    }
}

struct Bo { entry: usize, exit: usize, hold: usize, stop: f64, add: f64, units: usize, tick: f64 }

struct Pos { dir: f64, n: f64, legs: Vec<Leg>, stops: Vec<f64>, out: Vec<Option<(usize, f64, &'static str)>>, next_add: f64, entry_i: usize }

impl Pos {
    fn live(&self) -> usize { self.out.iter().filter(|o| o.is_none()).count() }
    /// Adds at next_add while the bar reaches it (gap = open fill); earlier stops move up by add x N.
    fn adds(&mut self, x: &Ctx, b: &Bo, i: usize, size: &dyn Fn(f64, f64, f64) -> f64, ex: f64) {
        while self.live() == self.legs.len() && self.legs.len() < b.units && (if self.dir > 0.0 { x.h[i] >= self.next_add } else { x.l[i] <= self.next_add }) {
            let fill = if self.dir > 0.0 { x.o[i].max(self.next_add) } else { x.o[i].min(self.next_add) };
            for s in &mut self.stops { *s += self.dir * b.add * self.n; }
            self.legs.push(Leg { i, px: fill, w: size(self.n, fill, ex) });
            self.stops.push(if b.stop > 0.0 { fill - self.dir * b.stop * self.n } else { -self.dir * f64::INFINITY });
            self.out.push(None);
            self.next_add = fill + self.dir * b.add * self.n;
        }
    }
    /// Stops / exit breakout vs the bar; `gap` = the position was held at the open.
    fn exits(&mut self, x: &Ctx, i: usize, ex: f64, gap: bool) {
        for k in 0..self.legs.len() {
            if self.out[k].is_some() { continue; }
            let (stop, t) = if self.dir > 0.0 { (self.stops[k] >= ex || ex.is_nan(), self.stops[k].max(ex)) } else { (self.stops[k] <= ex || ex.is_nan(), self.stops[k].min(ex)) };
            let why = if stop { "N stop" } else { "exit breakout" };
            let (o, hit) = (x.o[i], if self.dir > 0.0 { x.l[i] <= t } else { x.h[i] >= t });
            if gap && (o - t) * self.dir <= 0.0 { self.out[k] = Some((i, o, why)); } else if hit { self.out[k] = Some((i, t, why)); }
        }
    }
    fn close_all(&mut self, i: usize, px: f64, why: &'static str) {
        for o in &mut self.out { if o.is_none() { *o = Some((i, px, why)); } }
    }
    fn trade(self, open: bool) -> Trade {
        let outs: Vec<(usize, f64, &str)> = self.out.into_iter().map(Option::unwrap).collect();
        let q: Vec<f64> = self.legs.iter().map(|l| l.w / l.px).collect(); // contracts per leg, in equity units
        let qs: f64 = q.iter().sum();
        let exit_i = outs.iter().map(|o| o.0).max().unwrap();
        let pieces: Vec<(usize, f64, f64)> = outs.iter().zip(&q).map(|(o, q)| (o.0, o.1, q / qs)).collect();
        let exit_px = pieces.iter().map(|e| e.1 * e.2).sum();
        // one exit bar: a single (contract-weighted) exit price is exact; else keep the pieces for the daily marks
        let exits = if outs.iter().all(|o| o.0 == exit_i) { vec![] } else { pieces };
        let mut why: Vec<&str> = outs.iter().map(|o| o.2).collect();
        why.dedup();
        Trade { dir: self.dir, legs: self.legs, exit_i, exit_px, reason: if open { "open".into() } else { why.join(" + ") },
            entry_kind: "stop", signal_i: exit_i, open, exits }
    }
}

/// Breakout engine (Turtle, Donchian, Mechanica). `gate(i, dir_allowed)` filters entries; returns trades for entries in from..=to.
fn breakout(x: &Ctx, n: &[f64], b: &Bo, from: usize, to: usize, levels: &dyn Fn(usize) -> Option<(f64, f64)>,
            size: &dyn Fn(f64, f64, f64) -> f64) -> Vec<Trade> {
    let warm = b.entry.max(b.exit) + 1;
    let mut out = vec![];
    let mut cur: Option<Pos> = None;
    for i in from.max(warm)..=to {
        let exl = |dir: f64| if b.exit == 0 { f64::NAN } else if dir > 0.0 { ll(x, i, b.exit) - b.tick } else { hh(x, i, b.exit) + b.tick };
        if let Some(mut p) = cur.take() {
            let ex = exl(p.dir);
            p.exits(x, i, ex, true);
            if p.live() > 0 && b.hold > 0 && i >= p.entry_i + b.hold { p.close_all(i, x.c[i], "time exit"); }
            if p.live() == 0 { out.push(p.trade(false)); continue; } // no re-entry on an exit bar
            p.adds(x, b, i, size, ex);
            cur = Some(p);
            continue;
        }
        let nn = n[i - 1];
        if !nn.is_finite() { continue; }
        let Some((up, dn)) = levels(i) else { continue };
        let Some((dir, fill)) = stop_entry(x, i, up, dn) else { continue };
        let ex = exl(dir);
        let mut p = Pos { dir, n: nn, legs: vec![Leg { i, px: fill, w: size(nn, fill, ex) }],
            stops: vec![if b.stop > 0.0 { fill - dir * b.stop * nn } else { -dir * f64::INFINITY }], out: vec![None], next_add: fill + dir * b.add * nn, entry_i: i };
        p.adds(x, b, i, size, ex);
        p.exits(x, i, ex, false);
        if p.live() == 0 { out.push(p.trade(false)); } else { cur = Some(p); }
    }
    if let Some(mut p) = cur { p.close_all(to, x.c[to], "open"); out.push(p.trade(true)); }
    out
}

/// Close-signal engine: `want(i, cur)` = desired position after bar i's close, acted on at the next open.
fn on_close(x: &Ctx, from: usize, to: usize, size: &dyn Fn(usize, f64) -> f64, want: &dyn Fn(usize, f64) -> f64, why: &str) -> Vec<Trade> {
    let mut out = vec![];
    let mut cur: Option<Trade> = None;
    for i in from..to {
        let d = cur.as_ref().map_or(0.0, |t| t.dir);
        let w = want(i, d);
        if w == d { continue; }
        if let Some(mut t) = cur.take() { t.exit_i = i + 1; t.exit_px = x.o[i + 1]; t.signal_i = i; t.reason = why.into(); out.push(t); }
        if w != 0.0 {
            cur = Some(Trade { dir: w, legs: vec![Leg { i: i + 1, px: x.o[i + 1], w: size(i, x.o[i + 1]) }], exit_i: i + 1, exit_px: x.o[i + 1],
                reason: String::new(), entry_kind: "open", signal_i: i, open: false, exits: vec![] });
        }
    }
    if let Some(mut t) = cur { t.exit_i = to; t.exit_px = x.c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true; out.push(t); }
    out
}

/// Always-in stop-and-reverse at the n-day extreme +/- tick.
fn reversal(x: &Ctx, from: usize, to: usize, n: usize, tick: f64) -> Vec<Trade> {
    let mut out = vec![];
    let mut cur: Option<Trade> = None;
    let mk = |dir, i, px| Trade { dir, legs: vec![Leg { i, px, w: 1.0 }], exit_i: i, exit_px: px, reason: String::new(), entry_kind: "stop", signal_i: i, open: false, exits: vec![] };
    for i in from.max(n)..=to {
        let (up, dn) = (hh(x, i, n) + tick, ll(x, i, n) - tick);
        let hit = match cur.as_ref().map(|t| t.dir) {
            None => stop_entry(x, i, up, dn),
            Some(d) if d > 0.0 => stop_entry(x, i, f64::INFINITY, dn),
            Some(_) => stop_entry(x, i, up, f64::NEG_INFINITY),
        };
        let Some((dir, px)) = hit else { continue };
        if let Some(mut t) = cur.take() { t.exit_i = i; t.exit_px = px; t.signal_i = i; t.reason = format!("{n}-day breakout · reverse"); out.push(t); }
        cur = Some(mk(dir, i, px));
    }
    if let Some(mut t) = cur { t.exit_i = to; t.exit_px = x.c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true; out.push(t); }
    out
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if validate(id, p).is_some() || from > to || to >= x.r.len() { return None; }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap().def);
    let u = |k: &str| g(k) as usize;
    let n = || atr(x.r, u("n_len"));
    let per_n = |n: Vec<f64>, risk: f64| move |i: usize, fill: f64| risk / 100.0 * fill / n[i];
    Some(match id {
        "cov_turtle_s1" | "cov_turtle_s2" => {
            let (nv, tick, e) = (n(), g("tick"), u("entry"));
            let b = Bo { entry: e, exit: u("exit"), hold: 0, stop: g("stop_n"), add: g("add_n"), units: u("units"), tick };
            let size = |nn: f64, fill: f64, _| g("risk") / 100.0 * fill / nn;
            if id == "cov_turtle_s2" || g("filter") == 0.0 {
                return Some(breakout(x, &nv, &b, from, to, &|i| Some((hh(x, i, e) + tick, ll(x, i, e) - tick)), &size));
            }
            // Shadow System 1: every 20-day breakout as a single unit, 2N stop, 10-day exit, all signals taken, from the first bar.
            let fs = u("failsafe");
            let sb = Bo { units: 1, ..b };
            let shadow = breakout(x, &nv, &sb, 0, to, &|i| Some((hh(x, i, e) + tick, ll(x, i, e) - tick)), &|_, _, _| 1.0);
            let (mut won, mut fresh) = (vec![None; x.r.len()], vec![false; x.r.len()]);
            for t in shadow.iter() {
                fresh[t.legs[0].i] = true;
                if !t.open { for w in won.iter_mut().skip(t.exit_i + 1) { *w = Some(t.ret(0.0) > 0.0); } }
            }
            let warm = fs.max(e);
            let levels = |i: usize| -> Option<(f64, f64)> {
                if won[i] == Some(true) { (i >= warm).then(|| (hh(x, i, fs) + tick, ll(x, i, fs) - tick)) }
                else if fresh[i] { Some((hh(x, i, e) + tick, ll(x, i, e) - tick)) } else { None }
            };
            breakout(x, &nv, &Bo { entry: warm, ..b }, from, to, &levels, &size)
        }
        "cov_wott_donchian_trend" | "cov_wott_donchian_time" => {
            let (nv, tick, e) = (n(), g("tick"), u("entry"));
            let (f, s) = (ema(&x.c, u("fast")), ema(&x.c, u("slow")));
            let trend = id.ends_with("trend");
            let b = Bo { entry: e, exit: if trend { u("exit") } else { 0 }, hold: if trend { 0 } else { u("hold") }, stop: if trend { g("stop_n") } else { 0.0 }, add: 0.0, units: 1, tick };
            let levels = |i: usize| {
                let (a, z) = (f[i - 1], s[i - 1]);
                (a.is_finite() && z.is_finite() && a != z).then(|| if a > z { (hh(x, i, e) + tick, f64::NEG_INFINITY) } else { (f64::INFINITY, ll(x, i, e) - tick) })
            };
            let size = |nn: f64, fill: f64, _| g("risk") / 100.0 * fill / nn;
            breakout(x, &nv, &b, from, to, &levels, &size)
        }
        "cov_tf_mechanica_89_13" => {
            let tick = g("tick");
            let a = sma(&true_range(x.r), u("atr"));
            let e = u("entry");
            let b = Bo { entry: e, exit: u("exit"), hold: 0, stop: 0.0, add: 0.0, units: 1, tick };
            let risk = g("risk") / 100.0;
            let size = |atr: f64, fill: f64, stop: f64| risk * fill / (fill - stop).abs().max(2.0 * atr);
            breakout(x, &a, &b, from, to, &|i| Some((hh(x, i, e) + tick, ll(x, i, e) - tick)), &size)
        }
        "cov_wott_atr_cbo" | "cov_wott_bollinger_cbo" | "cov_tfm_bollinger_75" => {
            let (m, ma) = (u("ma"), x.sma_n(u("ma")).into_owned());
            let alt = id == "cov_tfm_bollinger_75";
            let nv = if alt { vec![] } else { n() };
            let (top, bot): (Vec<f64>, Vec<f64>) = if id == "cov_wott_atr_cbo" {
                (0..x.c.len()).map(|i| (ma[i] + g("top") * nv[i], ma[i] - g("bottom") * nv[i])).unzip()
            } else {
                let k = g("k");
                (0..x.c.len()).map(|i| {
                    if i + 1 < m || !ma[i].is_finite() { return (f64::NAN, f64::NAN); }
                    let sd = (x.c[i + 1 - m..=i].iter().map(|c| (c - ma[i]).powi(2)).sum::<f64>() / m as f64).sqrt();
                    (ma[i] + k * sd, ma[i] - k * sd)
                }).unzip()
            };
            let c = &x.c;
            let want = |i: usize, cur: f64| {
                if !(top[i].is_finite() && bot[i].is_finite()) { return cur; }
                if c[i] > top[i] { 1.0 } else if c[i] < bot[i] { -1.0 } else if (cur > 0.0 && c[i] < ma[i]) || (cur < 0.0 && c[i] > ma[i]) { 0.0 } else { cur }
            };
            if alt { return Some(on_close(x, from, to, &|_, _| 1.0, &want, "close back through the midpoint / opposite band · next open")); }
            on_close(x, from, to, &per_n(nv, g("risk")), &want, "close crossed the MA / opposite channel · next open")
        }
        "cov_tfm_sma_75" | "cov_tfm_ma_cross_50_75" => {
            let single = id == "cov_tfm_sma_75";
            let (f, m) = if single { (x.c.to_vec(), x.sma_n(u("ma")).into_owned()) } else { (x.sma_n(u("fast")).into_owned(), x.sma_n(u("slow")).into_owned()) };
            let want = |i: usize, cur: f64| {
                if i == 0 || !(f[i - 1].is_finite() && m[i - 1].is_finite()) { return cur; }
                if f[i] > m[i] && f[i - 1] <= m[i - 1] { 1.0 } else if f[i] < m[i] && f[i - 1] >= m[i - 1] { -1.0 } else { cur }
            };
            on_close(x, from, to, &|_, _| 1.0, &want, if single { "close crossed the MA · next open" } else { "MA crossover · next open" })
        }
        "cov_tfm_hh75_ll25" => {
            let (e, xd, close) = (u("entry"), u("exit"), g("close") == 1.0);
            // signal price vs the previous e / xd days' extremes; book example: the day's high/low (p.13, 15), variant: the close
            let hi_lo = |i: usize| if close { (x.c[i], x.c[i]) } else { (x.h[i], x.l[i]) };
            let want = |i: usize, cur: f64| {
                if i < e.max(xd) { return cur; }
                let (h, l) = hi_lo(i);
                let (up, dn) = (h > hh(x, i, e), l < ll(x, i, e));
                if cur > 0.0 { if l < ll(x, i, xd) { if dn { -1.0 } else { 0.0 } } else { 1.0 } }
                else if cur < 0.0 { if h > hh(x, i, xd) { if up { 1.0 } else { 0.0 } } else { -1.0 } }
                else if up && !dn { 1.0 } else if dn && !up { -1.0 } else { 0.0 }
            };
            on_close(x, from, to, &|_, _| 1.0, &want, "entry / exit breakout · next open")
        }
        "cov_wott_dual_ma" | "cov_wott_triple_ma" => {
            let triple = id == "cov_wott_triple_ma";
            let (f, m) = (x.sma_n(u("fast")).into_owned(), x.sma_n(u(if triple { "mid" } else { "slow" })).into_owned());
            let s = if triple { x.sma_n(u("slow")).into_owned() } else { vec![f64::NAN; x.c.len()] };
            let want = |i: usize, cur: f64| {
                if i == 0 || !(f[i - 1].is_finite() && m[i - 1].is_finite()) { return cur; }
                let (upx, dnx) = (f[i] > m[i] && f[i - 1] <= m[i - 1], f[i] < m[i] && f[i - 1] >= m[i - 1]);
                if !triple { return if upx { 1.0 } else if dnx { -1.0 } else { cur }; }
                if upx { if f[i] > s[i] && m[i] > s[i] { 1.0 } else if cur < 0.0 { 0.0 } else { cur } }
                else if dnx { if f[i] < s[i] && m[i] < s[i] { -1.0 } else if cur > 0.0 { 0.0 } else { cur } } else { cur }
            };
            on_close(x, from, to, &per_n(n(), g("risk")), &want, "MA crossover · next open")
        }
        "cov_tc_100day_reversal" | "cov_tc_4week_reversal" => reversal(x, from, to, u("n"), g("tick")),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn row(o: f64, h: f64, l: f64, c: f64) -> Row { Row { ticker: "T".into(), date: "2000-01-01".into(), open: o, high: h, low: l, close: c, volume: None } }

    #[test]
    fn n_matches_heating_oil_table() {
        // OTR p.15-16: from N=0.0134 on 11/1/2002, (19*PDN+TR)/20 gives the printed N column (4 dp).
        let tr = [0.0097, 0.0176, 0.0130, 0.0224, 0.0114, 0.0114, 0.0085, 0.0210, 0.0065, 0.0081, 0.0264, 0.0121, 0.0171, 0.0081, 0.0145, 0.0167, 0.0086, 0.0096, 0.0148, 0.0137, 0.0280, 0.0178];
        let book = [0.0132, 0.0134, 0.0134, 0.0139, 0.0137, 0.0136, 0.0134, 0.0138, 0.0134, 0.0131, 0.0138, 0.0137, 0.0139, 0.0136, 0.0136, 0.0138, 0.0135, 0.0133, 0.0134, 0.0134, 0.0141, 0.0143];
        let mut v = vec![0.0134; 20];
        v.extend(tr);
        let n = crate::connors::wilder(&v, 20);
        for (k, b) in book.iter().enumerate() { assert!((n[20 + k] - b).abs() < 0.00006, "day {k}: {} vs {b}", n[20 + k]); }
    }

    /// Crude oil example, OTR p.21 and p.23-24: N = 1.20, 55-day breakout 28.30, units 28.30/28.90/29.50/30.10, stops 27.70;
    /// with the 4th unit gapping to 30.80 its stop is 28.40 while the others stay 27.70.
    fn crude(gap: bool) -> Vec<Row> {
        let mut r: Vec<Row> = (0..60).map(|_| row(27.0, 27.6, 26.4, 27.0)).collect();
        r[49] = row(27.6, 28.2, 27.0, 27.6);
        r[50] = row(27.7, 28.29, 27.09, 27.7); // 55-day high 28.29 + 1 tick (0.01) = the book's 28.30 breakout
        r[51] = row(27.2, 27.8, 26.6, 27.2);
        if gap {
            r.push(row(28.0, 29.6, 27.9, 29.5));
            r.push(row(30.8, 31.0, 30.5, 30.9));
            r.push(row(30.0, 30.1, 28.0, 28.5));
            r.push(row(28.5, 28.9, 28.1, 28.6));
        } else {
            r.push(row(28.0, 30.2, 27.9, 30.0));
            r.push(row(29.0, 29.5, 27.6, 28.0));
        }
        r
    }
    #[test]
    fn crude_oil_units_and_stops() {
        let p: Params = [("tick".to_string(), 0.01)].into();
        let r = crude(false);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("cov_turtle_s2", &x, 55, r.len() - 1, &p).unwrap();
        assert_eq!(t.len(), 1);
        let px: Vec<f64> = t[0].legs.iter().map(|l| (l.px * 100.0).round() / 100.0).collect();
        assert_eq!(px, vec![28.30, 28.90, 29.50, 30.10]);
        assert!(t[0].legs.iter().all(|l| l.i == 60) && t[0].exit_i == 61 && (t[0].exit_px - 27.70).abs() < 1e-9 && t[0].exits.is_empty());
        assert!((t[0].legs[0].w - 0.01 * 28.30 / 1.2).abs() < 1e-9); // unit = 1% of equity per N

        let r = crude(true);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("cov_turtle_s2", &x, 55, r.len() - 1, &p).unwrap();
        assert_eq!(t[0].legs.len(), 4);
        assert!((t[0].legs[3].px - 30.80).abs() < 1e-9 && t[0].legs[3].i == 61);
        assert!(t[0].open);
        let e = &t[0].exits[3];
        assert_eq!(e.0, 62);
        assert!((e.1 - 28.40).abs() < 1e-9 && (e.2 - 0.25).abs() < 1e-9); // only the gapped unit is stopped
        assert!(t[0].exits[..3].iter().all(|e| e.0 == 63)); // others still open, marked at the last close
    }

    /// Alt. Entries p.13-15: a high above the previous 75 days' highest high = long signal, a low under the previous
    /// 25 days' lowest low = exit; both executed at the next open (Walk Through p.2). Close variant waits for the close.
    #[test]
    fn hh75_ll25_by_hand() {
        let mut r: Vec<Row> = (0..100).map(|_| row(100.0, 101.0, 99.0, 100.0)).collect();
        r[40] = row(100.0, 101.0, 97.0, 100.0); // older low: keeps the 75-day low (97) under the 25-day low (99) at bar 90
        r[80] = row(100.0, 101.5, 99.5, 100.5); // high 101.5 > 101: signal (close 100.5 < 101: no close signal)
        r[81] = row(100.7, 101.0, 99.5, 100.0);
        r[90] = row(100.0, 100.5, 98.5, 99.0); // low 98.5 < 99: exit signal (not under 97: no short)
        r[91] = row(99.2, 100.0, 98.0, 99.0);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("cov_tfm_hh75_ll25", &x, 45, r.len() - 1, &Params::new()).unwrap();
        assert_eq!(t.len(), 1);
        assert!(t[0].dir == 1.0 && t[0].legs[0].i == 81 && t[0].legs[0].px == 100.7 && t[0].legs[0].w == 1.0);
        assert!(!t[0].open && t[0].exit_i == 91 && t[0].exit_px == 99.2);
        let p: Params = [("close".to_string(), 1.0)].into();
        let t = run("cov_tfm_hh75_ll25", &x, 45, r.len() - 1, &p).unwrap();
        assert!(t.is_empty());
    }

    #[test]
    fn statuses() {
        let c = catalogue();
        assert_eq!(c.iter().filter(|d| d.status == Status::Live).count(), 15);
        for d in &c { assert!(family_of(d.id).is_some()); assert_eq!(d.status == Status::Live, params(d.id).is_some()); }
    }
}
