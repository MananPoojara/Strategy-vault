//! Lane K80 (13 Nov 2026; authors/titles-1030/K80.tsv): Arthur H. Ullrich, "Day Trading S&P 500 Index Futures Contract"
//! (course text, revised 20 Oct 1996). Book id "uldsp". Catalogue only: every entry is taught for discretionary trading on
//! 5-minute S&P futures bars, and none has both an exact entry and an exit stated in the text.
//! Source copy: research share 'USB HDD 2/tRADING KNO-2/Arthur H.Ullrich - DayTrading S&P Futures/Ullrich.zip' (one Windows
//! Write file, tootertext.wri; text read in full, the embedded PBrush charts carry no text). The file has no page numbers,
//! so the cards cite the course's section and heading. S&P "points" in the text are 0.01 index points (5 points per tick).
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

const SRC: &str = "Ullrich, Day Trading S&P 500 Index Futures (course text, 1996); file tootertext.wri in Ullrich.zip, read in full. No page numbers: section and heading cited.";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, dir, why, rules| StrategyDef { id, name, group, session: 0, dir, status: Status::NotTestable(why), rules,
        notes: SRC, needs_volume: false, needs_vix: false, needs_series: &[], book: "uldsp", claim: None };
    vec![
        d("uldsp_trend_turn", "Trend Turn: reference bar, setup bar, confirmation close beyond both", "Section 3 · Entries", "Long/Short",
          "Held: the entry is mechanical, but the exit is not. Stops 'somewhere' beyond the reference bar (1, 3 or 5 ticks by the trade's quality, Section 4 Stop Loss Placement), two of three contracts off at '40 or more' points, the third trailed by judgement, and a 15-30 minute bail-out 'You develop your own rules' (Section 4 Targets, Bail Out). The trade filters (trend, RSI 30/70, CCI, divergence, ADX) are listed as conditions that raise probability, not as required rules.",
          &["F|\"Reference bar - the bar that makes the High of the move. Setup bar - the bar following the Reference bar. The Confirmation bar is the first bar thereafter that CLOSES lower than the low of the Reference bar AND the Setup bar\" (Section 3, Trend Turn); buy signal the mirror",
            "F|\"You can only have a TREND TURN Sell after a TREND TURN Buy and vise-versa\"; a new extreme before confirmation starts a new signal (Section 3)",
            "X|\"I enter with three contracts, take off two at a 40 or more point target, and then work the last contract for the long term\" (Section 4, Money Management); \"I'm now using 5 ticks unless I tell you otherwise\" (Stop Loss Placement)"]),
        d("uldsp_dragonfly", "Dragonfly, doji, stars (candlestick reversals)", "Section 3 · Entries", "Long/Short",
          "Held: judged by eye ('closes at or near its low', 'How far down the previous bar it closes defines the quality'); no exit (Section 3, Dragonfly).", &[]),
        d("uldsp_bek", "BEK: outside bar near the moving average in a trend", "Section 3 · Entries", "Long/Short",
          "Held: 'at or near the moving average' and 'after a pause' are not defined; no entry price or exit (Section 3, BEK Trades).", &[]),
        d("uldsp_123", "123 entry: with-trend pullback, entry 1-3 ticks beyond the pullback low", "Section 3 · Entries", "Long/Short",
          "Held: the pullback ('one or more bars that pull back') and the trend are not defined, and no exit is given (Section 3, 123 Entries).", &[]),
        d("uldsp_second_signal", "Second Signal: retest of a Trend Turn with RSI divergence", "Section 3 · Entries", "Long/Short",
          "Held: 'a bar similar to your entry signal' with 'RSI divergent' is shown on charts only ('Let me show it to you'); no exit (Section 3, Second Signal).", &[]),
        d("uldsp_20_20", "20/20 trade (Street Smarts 80-20 entry, 10-tick overshoot)", "Section 3 · Entries", "Long/Short",
          "Duplicate of ss_80_20 (Connors & Raschke, Street Smarts, which the course names as its source): the same daily setup and buy-back entry, with Ullrich's 10-tick overshoot inside the book's 5-15 ticks; the course gives no stop or exit (Section 3, 20/20 Trade).",
          &["E|\"Yesterday's daily bar opened within 20% of the top and closed within 20% of the bottom. On the next day, if the price first goes lower (exceeds the low) than the reference day by 5-15 ticks, then BUY when the price returns to the Reference day's low\"; \"I suggest that the price exceed the high/low of the reference day by 10 ticks\" (Section 3)"]),
        d("uldsp_gap_trade", "Gap Trade: fade a 35-200 point opening gap inside yesterday's range", "Section 3 · Entries", "Long/Short",
          "Held: no stop is given; entry waits for 'a bar that closes in that direction' with 'the short-term trend' (undefined), and the target is yesterday's close 'or near it', judged from 'the tops of the last few bars' (Section 3, Gap Trade).",
          &["F|\"1. The market gaps on the open 2. The gap is within yesterday's range 3. The gap is larger than 35-50 points 4. The gap is not larger than 200 points\" (Section 3)"]),
        d("uldsp_english", "English buy/sell: reversal at a one-hour 15-minute swing point", "Section 3 · Entries", "Long/Short",
          "Held: the reference point is mechanical (lowest low / highest high for one hour either side on 15-minute bars, failed beyond 9 ticks) and the stop is 1 tick beyond the fail point, but the entry is 'the first bar that goes in the other direction and closes beyond its own midpoint but always look for some confirmation', and there is no target or exit (Section 3, English Buy/Sell).", &[]),
        d("uldsp_outside_bar", "Outside bar reversal", "Section 3 · Entries", "Long/Short",
          "Held: 'it must be in the right place' (a bottom or a top, judged); no entry price or exit (Section 3, Outside Bar).", &[]),
        d("uldsp_engulfing", "Bullish / bearish engulfing bar", "Section 3 · Entries", "Long/Short",
          "Held: 'often an entry signal, especially when it is near the beginning of a move' (judged); no exit (Section 3, Engulfing Bar).", &[]),
        d("uldsp_holy_grail", "Holy Grail variant: ADX(14) > 30, close back across the 20 EMA", "Section 3 · Entries", "Long/Short",
          "Held: the course's close-based version has a stop ('on top of the high point over the MA') but no target or exit; the Street Smarts version it quotes is coded as ss_holy_grail (Section 3, Holy Grail).", &[]),
        d("uldsp_4th_bar", "4th Bar: first close above or below the previous 4 closes", "Section 3 · Entries", "Long/Short",
          "Held: entries are skipped by judgement (an earlier entry, indicators not confirming, counter to a strong trend) and there is no stop or exit (Section 3, The 4th Bar).", &[]),
        d("uldsp_hop_lop", "High / Low Opening Position (160 points from the open)", "Section 2 · General Comments", "Long/Short",
          "Held: market statistics ('over a 90% probability that the LOP will not be reached'), not a trade rule (Section 2).", &[]),
        d("uldsp_delta_turns", "Delta Turns (Wilder / BEK turn times)", "Section 4 · Delta Turns", "Long/Short",
          "Held: 'This is not a trading system, but an alert' and the turns may come 45 minutes early or late (Section 4, Delta Turns).", &[]),
        d("uldsp_projections", "Pivot points and projected highs / lows (DeMark, BEK)", "Section 4 · Pivot Points, Projected High/Low", "Long/Short",
          "Held: levels only: 'These calculations are not intended to be traded' (Section 4, Projected High/Low); no entry or exit.", &[]),
    ]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { catalogue().into_iter().find(|d| d.id == id).map(|d| (d.id, d.name, "")) }
pub fn params(_id: &str) -> Option<Vec<ParamDef>> { None }
pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }
