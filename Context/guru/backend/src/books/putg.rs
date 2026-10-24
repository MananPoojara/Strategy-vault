//! John Hill, George Pruitt & Lundy Hill, "The Ultimate Trading Guide" (Wiley 2000, Futures Truth). Book id `putg`.
//! Source (read in place, copy pending): research drive "USB HDD 2/tRADING KNO-2/George Pruitt, John R.Hill - The
//! Ultimate Trading Guide & Code/The Ultimate Trading Guide by John R. Hill, George Pruitt, Lundy Hill.pdf".
//! Transcription, verification and README: library/putg/. Page refs are printed pages (PDF page = printed + 10).
//!
//! Chapter 9's five "turnkey" systems are coded from the book's Appendix EasyLanguage listings (p.285-291; code wins
//! over prose). Chapter 3's computer studies are coded from their rule lists plus the table headers. Order handling,
//! TradeStation functions and the bar path are the shared simulator in `pbwts` (George Pruitt's companion book):
//! next-bar stop/limit orders fill at their price or at a gapping open; several orders on one bar fill in the order
//! the bar reaches them (open nearer the low = O->L->H->C). EasyLanguage MarketPosition(1) is the direction of the
//! previous position (positions ago), EntriesToday(date) the entries filled on the current bar.
//! The book tested 21-29 futures (one contract, $) and 12 stocks; here the rules run on an index or an NSE stock
//! (instrument swap only, % returns, one unit per trade).
use super::pbwts::{entry, exit, highest, lowest, sim, ts_avg_tr, Kind};
use crate::connors::{claim, sma, true_range, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "putg";
/// Admitted ids (authors/pruitt/admission/SOURCE-CONTRACT.md).
pub const REVIEWED: &[&str] = &["putg_donchian_mod", "putg_ma_cross", "putg_stvorbo", "putg_pattern_vbo", "putg_four_close", "putg_daily_range", "putg_reversal_day", "putg_hook", "putg_narrow_range", "putg_gap"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

macro_rules! live {
    ($id:expr, $name:expr, $group:expr, $dir:expr, $rules:expr, $notes:expr, $claim:expr) => {
        StrategyDef { id: $id, name: $name, group: $group, session: 0, dir: $dir, status: Status::Live, needs_volume: false,
            needs_vix: false, needs_series: &[], book: BOOK, rules: $rules, notes: $notes, claim: $claim }
    };
}
macro_rules! off {
    ($id:expr, $name:expr, $group:expr, $dir:expr, $status:expr, $rules:expr, $notes:expr) => {
        StrategyDef { id: $id, name: $name, group: $group, session: 0, dir: $dir, status: $status, needs_volume: false,
            needs_vix: false, needs_series: &[], book: BOOK, rules: $rules, notes: $notes, claim: None }
    };
}

const CH9: &str = "Ch 9 · Turnkey systems (Appendix code)";
const CH3: &str = "Ch 3 · Bar chart computer studies";

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::{NeedsData, NotTestable};
    vec![
        live!("putg_donchian_mod", "System #1 · Modified Donchian breakout", CH9, "Long/Short",
            &["E|buy(\"donchian buy\") tomorrow at highest(high,30) stop; sell(\"donchian sell\") tomorrow at lowest(low,30) stop (p.285)",
              "F|if marketPosition = 1: midLevel = entryPrice(0) - (highest(high,30) - lowest(low,30))/2.0; if -1: entryPrice(0) + (...)/2.0 (p.285)",
              "F|if marketPosition <> 0 and barsSinceEntry(0) > 0 and Mod(barsSinceEntry(0),5) = 0: lookBack = maxlist(lookBack - 2, 6); if marketPosition = 0 then lookBack = 30 (p.285)",
              "X|longLiqPt = maxlist(lowest(low,lookBack), midLevel); shortLiqPt = minlist(highest(high,lookBack), midLevel); on the entry bar both = midLevel (p.285)",
              "X|if marketPosition = 1 then exitlong(\"long exit\") at longLiqPt stop; if -1 then exitshort(\"short exit\") at shortLiqPt stop (p.285)"],
            "Prose rules p.188 (\"Always use the stop that is currently closer to the market\"). The lookBack only resets when flat, as coded (a direct reversal keeps the shortened look-back). Table 9.12 (p.189): 22 futures, net $483,604, max DD $67,200, 9-13 trades/yr per market. No stock-index row.",
            claim("9-13/yr per market (22 futures)", "33.2%-49.4%", "$483,604 net", "", "1/1/83-8/31/99")),
        live!("putg_ma_cross", "System #2 · Moving average crossover", CH9, "Long/Short",
            &["E|if average(close,13) crosses above average(close,39) and close > close[39] then buy(\"macross buy\") tomorrow at open (p.286)",
              "E|if average(close,13) crosses below average(close,39) and close < close[39] then sell(\"macross sell\") tomorrow at open (reverses) (p.286-287)",
              "X|on the entry bar: protLongStop = entryPrice(0) - 5*atr[1] (atr = avgtruerange(20)); exitlong(\"lprot exit\") at protLongStop stop; shorts mirrored (p.286)",
              "X|if marketposition <> marketposition(1): ptargMult = 1, longLiqPt = 0, shortLiqPt = 999999 (p.286)",
              "X|if marketposition = 1 and high > entryPrice(0) + 5*atr*ptargMult: ptargMult = ptargMult + 1, longLiqPt = high - 5*atr; exitlong(\"long exit\") at longLiqPt stop; shorts mirrored (p.286)",
              "F|Option reset = 1 (labelled, not the code): read marketposition(1) as the previous bar's position, i.e. reset only when the position changes (the prose 'every time we move five ATRs in the profit', p.191-192)"],
            "Default = the code with EasyLanguage semantics: marketposition(1) is the previous POSITION, so while the current position differs from the previous one the trailing state is reset on every bar. Table 9.13 (p.191): 22 futures, net $355,638, max DD $61,425, 5-8 trades/yr per market. No stock-index row.",
            claim("5-8/yr per market (22 futures)", "29.6%-53.9%", "$355,638 net", "", "1/1/83-8/31/99")),
        live!("putg_stvorbo", "System #3 · Short-term volatility open range breakout", CH9, "Long/Short",
            &["F|bed (buy easier) if close < close[1]; sed (sell easier) if close >= close[1]; vol = highest(high,3) - lowest(low,3) (p.287)",
              "E|bed: stb = open of tomorrow + .5*vol, sts = open of tomorrow - vol; sed: stb = open + vol, sts = open - .5*vol; buy(\"stvorbo buy\") tomorrow at stb stop; sell(\"stvorbo sell\") tomorrow at sts stop (p.287-288)",
              "X|on the entry bar: protLongStop = entryPrice(0) - 3*atr[1] (atr = avgtruerange(10)); exitlong(\"lprot exit\") at protLongStop stop; shorts mirrored (p.287-288)",
              "X|if marketposition <> marketposition(1): longLiqPt = 0, shortLiqPt = 999999; if marketposition = 1 and high[1] > entryPrice(0) + 3*atr[1] then longLiqPt = entryPrice(0) (break-even); exitlong(\"long exit\") at longLiqPt stop; shorts mirrored (p.287-288)",
              "F|Option easier = 1: Table 9.15 Test A, 50% off the open both sides; easier = 2: Table 9.16 Test B, 100% both sides (p.195-196)"],
            "\"Popularized by Larry Williams\" (p.192). Table 9.14 (p.194): 22 futures, net $588,204, max DD $84,129, 26-38 trades/yr per market; Test A (9.15) $541,105; Test B (9.16) $397,126. No stock-index row.",
            claim("26-38/yr per market (22 futures)", "35.9%-43.9%", "$588,204 net", "", "1/1/83-8/31/99")),
        off!("putg_sp_daytrade", "System #4 · S&P day trade", CH9, "Long/Short",
            NeedsData("5-minute intraday S&P bars (data1) plus daily bars (data2): entries between 9:00 and 14:40 CT, failed-breakout entries, exit on the close"),
            &["F|atr = avgTrueRange(10) of data2; canTrade = 0 if average |open-close| (10 days) / atr < 0.5 (p.288)",
              "F|keyPrice = (high + low + close of data2)/3; buyDay if close of data2 > keyPrice, sellDay if < (p.288-289)",
              "E|buyDay: stb = open + .30*atr, sts = open - .60*atr; sellDay: .60 / .30; buy(\"buyBreakout\") at stb stop, sell(\"sellBreakout\") at sts stop, one long and one short per day (p.289-290)",
              "E|bco (low <= keyPrice - .75*range of data2) -> buy(\"bco buy\") at keyPrice - .25*range stop; sco mirrored (p.289-290)",
              "X|exitlong at entryprice - 3.00 stop; exitshort at entryprice + 3.00 stop; out on the close (p.290, p.201)"],
            "Table 9.17 (p.200): S&P 1/1/90-2/29/00, 1,269 trades, 40.80%, avg trade $71, $100 comm/slip."),
        live!("putg_pattern_vbo", "System #5 · Volatility breakout with pattern recognition", CH9, "Long/Short",
            &["F|atr = avgtruerange(30); canTrade = 1 if highest(close,4) - lowest(close,4) < atr (p.290)",
              "E|stb = open of tomorrow + .62*truerange; sts = open of tomorrow - .62*truerange (today's true range) (p.290)",
              "E|if canTrade = 1 and entriestoday(date) = 0 then buy(\"pattbuy\") tomorrow at stb stop and sell(\"pattsell\") tomorrow at sts stop (p.290)",
              "X|if canTrade = 0 then exitlong(\"longliq\") at sts stop and exitshort(\"shrtliq\") at stb stop (p.291)",
              "F|Option pattern = 0: Table 9.19, the same system without the four-close filter (canTrade always 1) (p.205)"],
            "Created by John Hill (p.203). The prose says 'average actual range for the past 30 days'; the code uses avgtruerange(30) (code wins). Table 9.18 (p.204): 22 futures, net $707,044, max DD $75,293, 40-57 trades/yr; without the pattern (9.19) $774,142, DD $168,930. No stock-index row.",
            claim("40-57/yr per market (22 futures)", "33.4%-47.2%", "$707,044 net", "", "1/1/83-8/31/99")),
        // ---------------- Chapter 3
        live!("putg_four_close", "Four closes in tight range breakout", CH3, "Long/Short",
            &["F|\"The range of four closes is less than the 30-day average bar range\" (p.44): highest - lowest of the 4 prior closes < Average(High - Low, 30) of the prior day",
              "F|\"The closing price of the prior day is greater than the closing price 50 days ago\" (p.44): Close[1] > Close[50] counted from the entry day (sells: <)",
              "E|\"The market must move 62% of the prior days range from the opening. This point must also be above the highest close of the prior four days\" (p.44): buy stop at max(Open + 0.62*(High[1]-Low[1]), highest of 4 prior closes); sells mirrored",
              "X|\"Stop = maximum of open - 62% of prior day's range on any day after entry and four prior day's closes\" (p.44; Table 3.2 header: 'max of opening range breakout of 62% of prior days range and lowest low of 4 prior closes'); shorts mirrored",
              "F|Option longs_only = 1: the stock version (Table 3.1)"],
            "Read (book-supported): 'N days ago' counts from the entry day, as in the authors' System #2 code where 'yesterday's close > close of 40 days back' is close > close[39]. The opposite signal reverses an open position (TradeStation default). Table 3.2 (p.46): 25 futures 1/1/84-6/30/99, no costs; S&P 500 row: 21 trades/yr, 39.1% wins, $49,000. Table 3.1: 12 stocks 1/1/96-10/30/99.",
            claim("21/yr (S&P 500 futures)", "39.1%", "$49,000 total", "", "1/1/84-6/30/99")),
        live!("putg_daily_range", "Daily range study (fade the close)", CH3, "Long/Short",
            &["E|\"Buy the opening if the market on the prior day closes below the opening and mid-range of the day\" (p.49)",
              "E|\"Sell the opening (on futures only), if the market on the prior day closes above the opening and mid-range\" (p.49)",
              "X|\"Liquidate on the close (day trade). No stops are used.\" (p.49)",
              "F|Option longs_only = 1: the stock version (Table 3.3)"],
            "Table 3.4 (p.48): 22 futures 1/1/84-6/30/99, no costs, ~199-213 trades/yr per market (no S&P row); Table 3.3: 12 stocks long only.", None),
        live!("putg_inside_day", "Inside day breakout (day trade)", CH3, "Long/Short",
            &["F|Inside day: \"the entire day's price action is within the range of the previous day\" (p.49): High < High[1] and Low > Low[1]",
              "E|\"have a buy order three ticks above the high of the bar with a sell order three ticks below the low of the bar\" (p.49-50)",
              "X|Day trade: liquidate on the close (Table 3.6 header 'Buy/sell breakout from inside bar - day trade')",
              "F|\"Trading in the direction of the close of the bar immediately preceding the inside bar\" (item 3, p.50): buy only if that bar closed above its open, sell only if it closed below; \"Tables 3.5 and 3.6 show that it is statistically significant ... if the preceding bar showed evidence of demand by closing above the opening (item 3 above)\" (p.50). Option dir_filter = 0: both stops every inside day"],
            "Fitted reading (C): the text ties the tables to item 3, and SPX 1984-1999 gives 15 trades/yr with it vs 33 without (book 17). Tick: 0.05 index points (S&P 500 futures minimum tick 1984-1997, also the NSE tick); 3 ticks = 0.15. Table 3.6 (p.51): 22 futures 1/1/84-6/30/99, no costs; S&P 500 row: 17 trades/yr, 49.6%, $5,245.",
            claim("17/yr (S&P 500 futures)", "49.6%", "$5,245 total", "day trade", "1/1/84-6/30/99")),
        live!("putg_reversal_day", "Reversal day (trend filtered)", CH3, "Long/Short",
            &["F|\"A reversal day is one where the price trades below (above) the previous day's range and then closes above (below) the previous day's low/high and opening\" (p.52): Low < Low[1], Close > Low[1] and Close > Open (sells mirrored)",
              "F|\"A buy entry is made only if the close of the prior day was greater than the close 50 days ago\" (p.55)",
              "E|\"Buy/sell on close of day of basic reversal day pattern setup\" (p.55)",
              "X|\"exit is made on any day that the market had a 60% opening range breakout of the previous day's range\" (p.55); Read: an adverse breakout = sell stop at Open - 0.60*(High[1]-Low[1]) on each day after entry (the book's stop method 3, p.70). Table 3.8 header says 0.5: param orb"],
            "The exit direction is not stated; the book's own list of stop methods calls the '50% opening range breakout after the date of entry' a stop (p.70), so the adverse reading is used. An opposite reversal day while in a trade reverses at that close. Table 3.8 (p.54): 21 futures 1/1/83-6/30/99, no costs, 23-31 trades/yr (no S&P row).", None),
        live!("putg_hook", "Hook closing", CH3, "Long/Short",
            &["F|\"The market has two consecutive up closes. A down close follows that is below the close of the prior day but above the close of the second day back\" (p.63)",
              "E|\"A purchase is made on the opening the following day\"; \"The reverse is true on sell orders (futures only)\" (p.63)",
              "X|\"and liquidated on the close\" (p.63)",
              "F|Option longs_only = 1: the stock version (Table 3.11)"],
            "Table 3.12 (p.62): 25 futures 1/1/94-6/30/99, no costs; S&P 500 row: 35 trades/yr, 49.4%, $13,525.",
            claim("35/yr (S&P 500 futures)", "49.4%", "$13,525 total", "day trade", "1/1/94-6/30/99")),
        live!("putg_narrow_range", "Narrow range breakout", CH3, "Long/Short",
            &["F|\"The range of the previous day is less than the range of any of the five prior days\" (p.66)",
              "F|\"The close of the prior day is greater than the close of 50 days ago\" (p.66)",
              "E|\"Buy at the maximum of an opening range breakout of 0.5 range of prior day and the high of the prior day\" (p.66): buy stop at max(Open + 0.5*Range[1], High[1]); sells mirrored",
              "X|\"Exit the trade on any day at the market that has a 0.5 opening range breakout\" (p.66), 'on any day after entry' (Table 3.14); Read: adverse 0.5 ORB stop from the day after entry (stop method 3, p.70)"],
            "Table 3.14 (p.65): 23 futures 1/1/93-6/30/99, no costs; S&P 500 row: 13 trades/yr, 45.9%, $47,343, 12% time in market. The header also says 'day trade', which contradicts 'any day after entry'; the rule list is followed.",
            claim("13/yr (S&P 500 futures)", "45.9%", "$47,343 total", "", "1/1/93-6/30/99")),
        live!("putg_gap", "Gap opening pullback (day trade)", CH3, "Long/Short",
            &["F|Gap: the market opens above the previous day's high (or below its low) (p.73)",
              "E|\"Sell a gap up opening but only if it pulls back to the prior day's high\" (p.75): sell stop at High[1] on a day that opened above it; buys mirrored at Low[1] (Table 3.15)",
              "X|\"no stops and as a day trade\" (p.75): exit on the close"],
            "Similar to Larry Williams' Oops (p.75; vault lwlts has Oops with a different exit). Table 3.15 (p.74): 29 futures 1/1/77-6/30/99, no costs; S&P 500 row: 44 trades/yr, 56.2%, $30,113.",
            claim("44/yr (S&P 500 futures)", "56.2%", "$30,113 total", "day trade", "1/1/77-6/30/99")),
        off!("putg_3de", "Three-day equilibrium reverse", CH3, "Long/Short",
            NotTestable("the pattern's first rule, 'first two days' closing prices in close proximity', has no number"),
            &["F|first two days' closes in close proximity; first two days smaller range than the third; third close above the 62% point of its range and above the two previous closes; third low above the second low (p.55-56)",
              "E|buy on the close of the day following the pattern; exit on a 0.5 range breakout off the open on any day after entry; trend Close[2] > Close[50]; Close[2] < Close[4] (p.57)"],
            "Table 3.10 (p.58): 21 futures, no S&P row."),
        off!("putg_stc_vbased", "STC-Vbased S&P day trade (Stafford Trading)", "Ch 13 · Top ten systems", "Long/Short",
            NeedsData("5-minute intraday S&P bars plus daily bars; the full rules are only in the compressed STCSP.ELA archive (the manual and the book give the outline only)"),
            &["E|\"a volatility based breakout with a breakout failure overlay (similar to R-Breaker)\" plus a filter that skips extremely volatile periods (p.280)",
              "F|Futures Truth filter (Stafford manual): the buy stop must be greater than the low of two days ago; the sell stop less than the high of two days ago; start 45 minutes after the open, stop 45 minutes before the close"],
            "Table 13.11 (p.280): S&P 1/1/90-8/31/99, $100 comm/slip. Programming by George Pruitt (manual)."),
        off!("putg_vendor_systems", "Top ten vendor systems (Aberration, Benchmark, Big Blue, DCS II, Dollar Trader, Dynamic Breakout, Golden SX, Mystery, R-Breaker)", "Ch 13 · Top ten systems", "Long/Short",
            NotTestable("commercial systems: the book prints performance only, no rules (the original Dynamic Breakout is said to be in the appendix but is not printed there)"),
            &["F|Descriptions only (p.258-279)"], "Tables 13.2-13.10."),
        off!("putg_chart_reading", "Chart patterns, swing and channel trading (Ch 1-2, 4-7)", "Ch 1-7 · Chart reading", "Long/Short",
            NotTestable("judgment: buy/sell zones, pivots, trendlines, 'close proximity', thrusts and targets are not defined numerically ('requires judgment', p.80; 'some subjectivity is required', p.118)"),
            &["F|Stages of market action, Elliott corrections, trendline/four-close (TL4C), swing charts, springs/upthrusts, yum-yum, clear outs, Keltner channel trading modified, Drummond PLdot (p.8-152)"],
            "Keltner's 10-day moving average rule (p.117) is already in the vault (kftsm_keltner)."),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "putg_donchian_mod" => ("putg_turnkey", "UTG turnkey systems", "#1 Donchian"),
        "putg_ma_cross" => ("putg_turnkey", "UTG turnkey systems", "#2 MA cross"),
        "putg_stvorbo" => ("putg_turnkey", "UTG turnkey systems", "#3 Volatility ORB"),
        "putg_sp_daytrade" => ("putg_turnkey", "UTG turnkey systems", "#4 S&P day trade"),
        "putg_pattern_vbo" => ("putg_turnkey", "UTG turnkey systems", "#5 Pattern breakout"),
        "putg_four_close" => ("putg_studies", "UTG bar chart studies", "Four closes"),
        "putg_daily_range" => ("putg_studies", "UTG bar chart studies", "Daily range"),
        "putg_inside_day" => ("putg_studies", "UTG bar chart studies", "Inside day"),
        "putg_reversal_day" => ("putg_studies", "UTG bar chart studies", "Reversal day"),
        "putg_hook" => ("putg_studies", "UTG bar chart studies", "Hook closing"),
        "putg_narrow_range" => ("putg_studies", "UTG bar chart studies", "Narrow range"),
        "putg_gap" => ("putg_studies", "UTG bar chart studies", "Gap pullback"),
        "putg_3de" => ("putg_studies", "UTG bar chart studies", "Three-day equilibrium"),
        "putg_stc_vbased" => ("putg_vendor", "UTG top ten systems", "STC-Vbased"),
        "putg_vendor_systems" => ("putg_vendor", "UTG top ten systems", "Others"),
        "putg_chart_reading" => ("putg_chart", "UTG chart reading", ""),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let longs = || p("longs_only", "Longs only (stock version)", 0., 0., 1., 1.);
    Some(match id {
        "putg_donchian_mod" => vec![p("entry_len", "Breakout days", 30., 10., 60., 5.), p("step_bars", "Bars per look-back step", 5., 2., 10., 1.),
            p("step", "Look-back decrement", 2., 1., 5., 1.), p("min_lb", "Minimum look-back", 6., 2., 20., 1.)],
        "putg_ma_cross" => vec![p("fast", "Fast average", 13., 5., 30., 1.), p("slow", "Slow average", 39., 20., 80., 1.), p("mom", "Close vs close[n]", 39., 10., 80., 1.),
            p("atr_len", "ATR length", 20., 10., 40., 5.), p("mult", "ATR multiple", 5., 2., 8., 0.5), p("reset", "Reset trailing on position change only (labelled reading)", 0., 0., 1., 1.)],
        "putg_stvorbo" => vec![p("vol_len", "High-low range days", 3., 2., 10., 1.), p("near", "Easier side fraction", 0.5, 0.25, 1., 0.05), p("far", "Harder side fraction", 1., 0.5, 2., 0.05),
            p("atr_len", "ATR length", 10., 5., 30., 1.), p("mult", "ATR multiple", 3., 1., 5., 0.5), p("easier", "0 = buy/sell easier, 1 = Test A 50/50, 2 = Test B 100/100", 0., 0., 2., 1.)],
        "putg_pattern_vbo" => vec![p("atr_len", "ATR length", 30., 10., 60., 5.), p("closes", "Closes in pattern", 4., 2., 10., 1.), p("pct", "True-range fraction", 0.62, 0.3, 1.5, 0.02),
            p("pattern", "Four-close filter (0 = Table 9.19)", 1., 0., 1., 1.)],
        "putg_four_close" => vec![p("closes", "Closes in pattern", 4., 2., 10., 1.), p("rng_len", "Average range days", 30., 10., 60., 5.),
            p("trend", "Trend: close N days ago", 50., 10., 100., 5.), p("pct", "Opening range fraction", 0.62, 0.3, 1.5, 0.02), longs()],
        "putg_daily_range" | "putg_hook" => vec![longs()],
        "putg_inside_day" => vec![p("ticks", "Ticks beyond the inside bar", 3., 0., 10., 1.), p("tick", "Tick size (index points)", 0.05, 0.01, 1., 0.01),
            p("dir_filter", "Trade in the direction of the preceding bar (item 3)", 1., 0., 1., 1.)],
        "putg_reversal_day" => vec![p("trend", "Trend: close N days ago", 50., 10., 100., 5.), p("orb", "Exit opening range breakout (0.6 text, 0.5 table)", 0.6, 0.3, 1., 0.05)],
        "putg_narrow_range" => vec![p("lookback", "Narrower than N prior days", 5., 2., 10., 1.), p("trend", "Trend: close N days ago", 50., 10., 100., 5.),
            p("pct", "Opening range fraction", 0.5, 0.2, 1., 0.05)],
        "putg_gap" => vec![],
        _ => return None,
    })
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if from > to || to >= x.r.len() { return None; }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let u = |k: &str| { let v = g(k).round(); if v.is_finite() && v >= 1.0 { v as usize } else { 1 } };
    let n = x.r.len();
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let rng: Vec<f64> = (0..n).map(|i| h[i] - l[i]).collect();
    const B: f64 = 1.0;
    const S: f64 = -1.0;
    use Kind::*;
    let longs_only = g("longs_only") >= 0.5;
    Some(match id {
        "putg_donchian_mod" => {
            let en = u("entry_len");
            let (hh, ll) = (highest(h, en), lowest(l, en));
            let (step_bars, step, min_lb) = (u("step_bars"), u("step"), u("min_lb"));
            let (mut look, mut mid) = (en, 0.0);
            sim(x, from, to, move |i, st| {
                let mp = st.mp();
                let bse = st.bars_since_entry(i);
                if mp == B { mid = st.entry_px() - (hh[i] - ll[i]) / 2.0; }
                if mp == S { mid = st.entry_px() + (hh[i] - ll[i]) / 2.0; }
                if mp != 0.0 && bse > 0 && bse % step_bars == 0 { look = look.saturating_sub(step).max(min_lb); }
                let lw = if i + 1 >= look { l[i + 1 - look..=i].iter().copied().fold(f64::INFINITY, f64::min) } else { f64::NAN };
                let hg = if i + 1 >= look { h[i + 1 - look..=i].iter().copied().fold(f64::NEG_INFINITY, f64::max) } else { f64::NAN };
                let (mut lliq, mut sliq) = (lw.max(mid), hg.min(mid));
                if bse == 0 && mp != 0.0 { lliq = mid; sliq = mid; }
                let mut v = vec![];
                if mp == B { v.push(exit(S, Stop, lliq, "long exit")); }
                if mp == S { v.push(exit(B, Stop, sliq, "short exit")); }
                v.push(entry(B, Stop, hh[i], "donchian buy"));
                v.push(entry(S, Stop, ll[i], "donchian sell"));
                if mp == 0.0 { look = en; }
                v
            })
        }
        "putg_ma_cross" | "putg_stvorbo" => {
            let ma = id == "putg_ma_cross";
            let atr = ts_avg_tr(x, u("atr_len"));
            let k = g("mult");
            let by_bar = ma && g("reset") >= 0.5;
            let (fast, slow, mom) = if ma { (sma(c, u("fast")), sma(c, u("slow")), u("mom")) } else { (vec![], vec![], 0) };
            let vl = if ma { 1 } else { u("vol_len") };
            let (hv, lv) = (highest(h, vl), lowest(l, vl));
            let (near, far, mode) = (g("near"), g("far"), g("easier").round() as i32);
            let (mut pm, mut lliq, mut sliq, mut pls, mut pss) = (1.0, 0.0, 999999.0, 0.0, 0.0);
            let mut prev_bar = 0.0;
            sim(x, from, to, move |i, st| {
                let mp = st.mp();
                let bse = st.bars_since_entry(i);
                let ep = st.entry_px();
                let a1 = if i >= 1 { atr[i - 1] } else { f64::NAN };
                if mp == B && bse == 0 { pls = ep - k * a1; }
                if mp == S && bse == 0 { pss = ep + k * a1; }
                let changed = if by_bar { mp != prev_bar } else { mp != st.last_dir };
                prev_bar = mp;
                if changed { pm = 1.0; lliq = 0.0; sliq = 999999.0; }
                if ma {
                    if mp == B && h[i] > ep + k * atr[i] * pm { pm += 1.0; lliq = h[i] - k * atr[i]; }
                    if mp == S && l[i] < ep - k * atr[i] * pm { pm += 1.0; sliq = l[i] + k * atr[i]; }
                } else {
                    if mp == B && i >= 1 && h[i - 1] > ep + k * a1 { lliq = ep; }
                    if mp == S && i >= 1 && l[i - 1] < ep - k * a1 { sliq = ep; }
                }
                let mut v = vec![];
                if mp == B { v.push(exit(S, Stop, lliq, if ma { "long exit (trail)" } else { "long exit (break-even)" })); v.push(exit(S, Stop, pls, "lprot exit")); }
                if mp == S { v.push(exit(B, Stop, sliq, if ma { "short exit (trail)" } else { "short exit (break-even)" })); v.push(exit(B, Stop, pss, "sprot exit")); }
                if ma {
                    let cu = i >= 1 && fast[i - 1] < slow[i - 1] && fast[i] > slow[i];
                    let cd = i >= 1 && fast[i - 1] > slow[i - 1] && fast[i] < slow[i];
                    if cu && i >= mom && c[i] > c[i - mom] { v.push(entry(B, Open, f64::NAN, "macross buy")); }
                    if cd && i >= mom && c[i] < c[i - mom] { v.push(entry(S, Open, f64::NAN, "macross sell")); }
                } else if i >= 1 && i + 1 < n {
                    let vol = hv[i] - lv[i];
                    let bed = c[i] < c[i - 1];
                    let (fb, fs) = match mode { 1 => (0.5, 0.5), 2 => (1.0, 1.0), _ => if bed { (near, far) } else { (far, near) } };
                    v.push(entry(B, Stop, o[i + 1] + fb * vol, "stvorbo buy"));
                    v.push(entry(S, Stop, o[i + 1] - fs * vol, "stvorbo sell"));
                }
                v
            })
        }
        "putg_pattern_vbo" => {
            let atr = ts_avg_tr(x, u("atr_len"));
            let (hc, lc) = (highest(c, u("closes")), lowest(c, u("closes")));
            let tr = true_range(x.r);
            let (pct, pat) = (g("pct"), g("pattern") >= 0.5);
            sim(x, from, to, |i, st| {
                if i + 1 >= n { return vec![]; }
                let can = !pat || hc[i] - lc[i] < atr[i];
                let (stb, sts) = (o[i + 1] + pct * tr[i], o[i + 1] - pct * tr[i]);
                let mut v = vec![];
                if can && st.entries_today == 0 { v.push(entry(B, Stop, stb, "pattbuy")); v.push(entry(S, Stop, sts, "pattsell")); }
                if !can { v.push(exit(S, Stop, sts, "longliq")); v.push(exit(B, Stop, stb, "shrtliq")); }
                v
            })
        }
        "putg_four_close" => {
            let k = u("closes");
            let (hc, lc) = (highest(c, k), lowest(c, k));
            let avg_rng = sma(&rng, u("rng_len"));
            let (tb, pct) = (u("trend"), g("pct"));
            sim(x, from, to, |i, st| {
                // orders for bar j = i + 1; i is "the prior day"
                let j = i + 1;
                if j >= n || j < tb { return vec![]; }
                let mut v = vec![];
                let mp = st.mp();
                // the stop applies on every day after the entry day (orders built at a close are for the next bar)
                if mp == B { v.push(exit(S, Stop, (o[j] - pct * rng[i]).max(lc[i]), "62% ORB / 4-close stop")); }
                if mp == S { v.push(exit(B, Stop, (o[j] + pct * rng[i]).min(hc[i]), "62% ORB / 4-close stop")); }
                if hc[i] - lc[i] < avg_rng[i] {
                    if c[i] > c[j - tb] { v.push(entry(B, Stop, (o[j] + pct * rng[i]).max(hc[i]), "four-close buy")); }
                    if c[i] < c[j - tb] && !longs_only { v.push(entry(S, Stop, (o[j] - pct * rng[i]).min(lc[i]), "four-close sell")); }
                }
                v
            })
        }
        "putg_daily_range" => sim(x, from, to, |i, _| {
            let mid = (h[i] + l[i]) / 2.0;
            let mut v = vec![];
            if c[i] < o[i] && c[i] < mid { v.push(entry(B, Open, f64::NAN, "buy down close")); v.push(exit(S, Close, f64::NAN, "close")); }
            if c[i] > o[i] && c[i] > mid && !longs_only { v.push(entry(S, Open, f64::NAN, "sell up close")); v.push(exit(B, Close, f64::NAN, "close")); }
            v
        }),
        "putg_inside_day" => {
            let off = g("ticks") * g("tick");
            let prior_up = g("dir_filter") >= 0.5;
            sim(x, from, to, |i, _| {
                if i < 2 || !(h[i] < h[i - 1] && l[i] > l[i - 1]) { return vec![]; }
                let mut v = vec![];
                if !prior_up || c[i - 1] > o[i - 1] { v.push(entry(B, Stop, h[i] + off, "inside buy")); }
                if !prior_up || c[i - 1] < o[i - 1] { v.push(entry(S, Stop, l[i] - off, "inside sell")); }
                v.push(exit(S, Close, f64::NAN, "close"));
                v.push(exit(B, Close, f64::NAN, "close"));
                v
            })
        }
        "putg_narrow_range" => {
            let (tb, orb, lb) = (u("trend"), g("pct"), u("lookback"));
            sim(x, from, to, |i, st| {
                let j = i + 1;
                if j >= n { return vec![]; }
                let mut v = vec![];
                let mp = st.mp();
                if mp == B { v.push(exit(S, Stop, o[j] - orb * rng[i], "0.5 ORB stop")); }
                if mp == S { v.push(exit(B, Stop, o[j] + orb * rng[i], "0.5 ORB stop")); }
                if i >= lb && j >= tb && (1..=lb).all(|k| rng[i] < rng[i - k]) {
                    if c[i] > c[j - tb] { v.push(entry(B, Stop, (o[j] + orb * rng[i]).max(h[i]), "narrow range buy")); }
                    if c[i] < c[j - tb] { v.push(entry(S, Stop, (o[j] - orb * rng[i]).min(l[i]), "narrow range sell")); }
                }
                v
            })
        }
        "putg_reversal_day" => reversal_day(x, from, to, u("trend"), g("orb"), &rng),
        "putg_hook" => sim(x, from, to, |i, _| {
            if i < 3 { return vec![]; }
            let up2 = c[i - 2] > c[i - 3] && c[i - 1] > c[i - 2];
            let dn2 = c[i - 2] < c[i - 3] && c[i - 1] < c[i - 2];
            let mut v = vec![];
            if up2 && c[i] < c[i - 1] && c[i] > c[i - 2] { v.push(entry(B, Open, f64::NAN, "hook buy")); v.push(exit(S, Close, f64::NAN, "close")); }
            if dn2 && c[i] > c[i - 1] && c[i] < c[i - 2] && !longs_only { v.push(entry(S, Open, f64::NAN, "hook sell")); v.push(exit(B, Close, f64::NAN, "close")); }
            v
        }),
        "putg_gap" => sim(x, from, to, |i, _| {
            let j = i + 1;
            if j >= n { return vec![]; }
            let mut v = vec![];
            if o[j] > h[i] { v.push(entry(S, Stop, h[i], "gap-up pullback sell")); v.push(exit(B, Close, f64::NAN, "close")); }
            if o[j] < l[i] { v.push(entry(B, Stop, l[i], "gap-down pullback buy")); v.push(exit(S, Close, f64::NAN, "close")); }
            v
        }),
        _ => return None,
    })
}

/// Reversal day: entry on the close of the pattern day, then the adverse opening-range-breakout stop on each later
/// day (fills at the stop, or at the open if the day opens through it). An opposite reversal day reverses at its close.
fn reversal_day(x: &Ctx, from: usize, to: usize, tb: usize, orb: f64, rng: &[f64]) -> Vec<Trade> {
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let mut out = Vec::new();
    let mut cur: Option<(f64, usize, f64)> = None; // dir, entry bar, entry price
    let push = |out: &mut Vec<Trade>, (d, e, px): (f64, usize, f64), k: usize, xp: f64, why: &str, sig: usize, open: bool| out.push(Trade {
        dir: d, legs: vec![Leg { i: e, px, w: 1.0 }], exit_i: k, exit_px: xp, reason: why.into(), entry_kind: "close", signal_i: sig, open, exits: vec![] });
    for i in from..=to {
        if let Some((d, e, _)) = cur {
            if i > e {
                let stop = o[i] - d * orb * rng[i - 1];
                if (d > 0.0 && l[i] <= stop) || (d < 0.0 && h[i] >= stop) {
                    let px = if d > 0.0 { o[i].min(stop) } else { o[i].max(stop) };
                    push(&mut out, cur.take().unwrap(), i, px, "opening range breakout stop", i - 1, false);
                }
            }
        }
        if i < tb { continue; }
        let up = l[i] < l[i - 1] && c[i] > l[i - 1] && c[i] > o[i] && c[i - 1] > c[i - tb];
        let dn = h[i] > h[i - 1] && c[i] < h[i - 1] && c[i] < o[i] && c[i - 1] < c[i - tb];
        let d = if up { 1.0 } else if dn { -1.0 } else { 0.0 };
        if d == 0.0 || cur.is_some_and(|t| t.0 == d) { continue; }
        if let Some(t) = cur.take() { push(&mut out, t, i, c[i], "reversed by the opposite reversal day", i, false); }
        cur = Some((d, i, c[i]));
    }
    if let Some(t) = cur { push(&mut out, t, to, c[to], "open", to, true); }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn hook_buys_next_open_and_exits_that_close() {
        let px = [(10., 10.5, 9.5, 10.), (10., 11.5, 9.9, 11.), (11., 12.5, 10.9, 12.), (12., 12.2, 11.2, 11.5), (11.6, 12.4, 11.4, 12.3)];
        let r: Vec<Row> = px.iter().enumerate().map(|(i, &(o, h, l, c))| Row { ticker: "T".into(), date: format!("2001-01-{:02}", i + 1), open: o, high: h, low: l, close: c, volume: None }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let t = run("putg_hook", &x, 1, 4, &Params::new()).unwrap();
        assert_eq!(t.len(), 1);
        assert_eq!((t[0].dir, t[0].legs[0].i, t[0].legs[0].px, t[0].exit_i, t[0].exit_px), (1., 4, 11.6, 4, 12.3));
    }
    #[test]
    fn every_live_strategy_runs_with_fills_inside_the_bar() {
        let r: Vec<Row> = (0..500).map(|i| { let b = 100.0 + (i as f64 / 7.0).sin() * 8.0 + (i as f64 / 31.0).cos() * 5.0;
            let c = b + 0.3 * ((i % 5) as f64 - 2.0);
            Row { ticker: "T".into(), date: format!("{:04}-{:02}-{:02}", 2000 + i / 250, 1 + (i / 21) % 12, 1 + i % 21), open: b, high: b.max(c) + 1.0 + (i % 3) as f64, low: b.min(c) - 1.2, close: c, volume: None } }).collect();
        let x = Ctx::new(&r, &r, &[]);
        for d in catalogue() {
            let live = d.status == Status::Live;
            assert_eq!(live, params(d.id).is_some(), "{}", d.id);
            assert!(family_of(d.id).is_some());
            if !live { continue; }
            let t = run(d.id, &x, 60, 499, &Params::new()).unwrap();
            for tr in &t {
                let (e, k) = (tr.legs[0].i, tr.exit_i);
                assert!(x.l[e] <= tr.legs[0].px && tr.legs[0].px <= x.h[e], "{} entry", d.id);
                assert!(x.l[k] <= tr.exit_px && tr.exit_px <= x.h[k], "{} exit", d.id);
            }
        }
    }
}
