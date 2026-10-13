//! Jason Perl, *DeMark Indicators* (Bloomberg Press, 2008). Book id `tdperl`.
//! Source copy: Book-to-rust/Books/Jason Perl/. Transcription, verification and README: library/tdperl/.
//! Printed page p = PDF page p+33 (chapter 1) / p+33..35 (later chapters).
//!
//! Every sell rule is the buy rule on mirrored prices (o,h,l,c -> -o,-l,-h,-c), so one code path
//! serves both sides. Perl, p.48: "it's up to you to manage the risk and the take-profit level (since
//! DeMark doesn't explain how to manage the trade once it starts to move in your favor). Remember,
//! TD Sequential is an indicator, not a system." Countdown trades therefore have the book's entry and
//! the book's TD risk-level stop, plus one labelled vault exit (the opposite signal of the same study).
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "tdperl";

const NO_EXIT: &str = "[Not from the book] Perl gives no profit exit (p.48: \"it's up to you to manage the risk and the take-profit level ... TD Sequential is an indicator, not a system\"). Vault exit: the opposite signal of the same study, at its close (p.49: \"there isn't always a signal in the opposite direction to tell you where to close out a profitable trade\"). Optional time exit max_bars (p.28: \"a meaningful response within twelve price bars\") is off by default.";
const STOP: &str = "X|TD risk level (p.26-27, 39): \"identify the lowest true low throughout the TD Sequential Buy Countdown process, which includes bars one through thirteen, whether or not it is a numbered price bar; calculate the difference between the true high and true low for that bar; and subtract that true range from its true low.\" Stop = intrabar sell stop at that level (Perl: \"My own preference is to stop out of a long position as soon as the downside risk level is violated on an intrabar basis\", p.28); a gap through it fills at the open. Sell side mirrored.";
const ENTRY_OPTS: &str = "E|Entry options (entry=1..6, not default) are the book's conservative and alternative entries (p.24-26, 35-38): 1 first close beyond the close four bars earlier after the 13 (TD Price Flip), 2 TD Camouflage, 3 TD Clop, 4 TD Clopwin, 5 TD Open, 6 TD Trap. [Book silent] a pending alternative entry is void once price reaches the TD risk level or an opposite 13 prints.";
const SETUP: &str = "F|TD Buy Setup (p.2-3): \"A Bearish TD Price Flip occurs when the market records a close greater than the close four bars earlier, immediately followed by a close less than the close four bars earlier\"; \"after a bearish TD Price Flip, there must be nine consecutive closes, each one less than the corresponding close four bars earlier\" (flip bar = bar one). Sell Setup mirrored (bullish flip, closes greater than the close four bars earlier).";
const CANCEL: &str = "F|Cancellation (p.21, 34): \"either of the following conditions erases an incomplete TD Buy Countdown: 1. if the price action rallies and generates a TD Sell Setup, or 2. if the market ... posts a true low above the true high of the prior TD Buy Setup—that is, TDST resistance\" (true high of setup bars 1-9; Bloomberg 'Include Prior Four Bars' off, p.57).";
const RECYCLE: &str = "F|Recycle (p.21-22, settings p.57-58): a new same-direction setup completing while a countdown is incomplete: if its true range is >= 1.000 and < 1.618 x the prior setup's (maximum-extent) true range, the larger setup becomes active and countdown restarts; if its closing range and extreme lie within the prior setup's true range, the prior setup stays active. [Book silent] any other case keeps the prior countdown (recycle_other=1 restarts instead). R (p.22): a later same-direction setup that extends to 18 bars while the countdown is incomplete negates it; counting restarts from that setup's bar 9.";

fn d(id: &'static str, name: &'static str, group: &'static str, dir: &'static str, status: Status,
     rules: &'static [&'static str], notes: &'static str) -> StrategyDef {
    StrategyDef { id, name, group, session: 0, dir, status, needs_volume: false, needs_vix: false,
        needs_series: &[], book: BOOK, rules, notes, claim: None }
}

macro_rules! seq_rules { ($($r:expr),* $(,)?) => { &[SETUP, $($r,)* CANCEL, RECYCLE, ENTRY_OPTS, STOP, "X|Exit at the close of the opposite 13 of the same study [Not from the book; see notes]"] } }

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    let cd_note = NO_EXIT;
    let opt_note = "Book: TD Sequential/Combo are indicators; Perl prints no statistics. India: instrument swap only.";
    vec![
        d("tdperl_seq13_long", "TD Sequential Buy Countdown 13", "Ch 1 · TD Sequential", "Long", NotTestable("no exit in the book: Perl p.48 \"it's up to you to manage the risk and the take-profit level ... TD Sequential is an indicator, not a system\" — entry and TD risk-level stop are exact, but a trade needs an exit, and the vault does not invent one"),
          seq_rules!["F|TD Buy Countdown (p.17-19): starts on setup bar nine (inclusive); \"a total of thirteen closes, each one less than, or equal to, the low two bars earlier\" (need not be consecutive).",
            "F|Bar 13 (p.19): \"1. The low of TD Buy Countdown bar thirteen must be less than, or equal to, the close of TD Buy Countdown bar eight, and 2. The close of TD Buy Countdown bar thirteen must be less than, or equal to, the low two bars earlier.\" Otherwise deferred (+). Options: q85=1 bar 8 low <= bar 5 close (p.20 'elective'); term_open=1 DeMark's open termination count (p.26).",
            "E|Aggressive Approach (p.24, Perl's preference): \"Buy on the close of a TD Buy Countdown bar thirteen\"."],
          cd_note),
        d("tdperl_seq13_short", "TD Sequential Sell Countdown 13", "Ch 1 · TD Sequential", "Short", NotTestable("no exit in the book: Perl p.48 \"it's up to you to manage the risk and the take-profit level ... TD Sequential is an indicator, not a system\" — entry and TD risk-level stop are exact, but a trade needs an exit, and the vault does not invent one"),
          seq_rules!["F|TD Sell Countdown (p.31): thirteen closes, each \"greater than, or equal to, the high two bars earlier\"; bar 13 high >= close of bar 8 (p.31).",
            "E|Aggressive Approach (p.35): \"Sell on the close of TD Sell Countdown bar thirteen\"."],
          cd_note),
        d("tdperl_agg13_long", "TD Aggressive Sequential Buy 13", "Ch 1 · TD Aggressive Sequential", "Long", NotTestable("no exit in the book: Perl p.48 \"it's up to you to manage the risk and the take-profit level ... TD Sequential is an indicator, not a system\" — entry and TD risk-level stop are exact, but a trade needs an exit, and the vault does not invent one"),
          seq_rules!["F|TD Aggressive Sequential (p.58): \"compares the low with the low two price bars earlier during the TD Buy Countdown phase\" (low <= low two bars earlier). [Book silent] bar-13 qualifier kept as TD Sequential (low of 13 <= close of 8): \"it's just the TD Countdown phase that differs\".",
            "E|Buy on the close of countdown bar thirteen."],
          cd_note),
        d("tdperl_agg13_short", "TD Aggressive Sequential Sell 13", "Ch 1 · TD Aggressive Sequential", "Short", NotTestable("no exit in the book: Perl p.48 \"it's up to you to manage the risk and the take-profit level ... TD Sequential is an indicator, not a system\" — entry and TD risk-level stop are exact, but a trade needs an exit, and the vault does not invent one"),
          seq_rules!["F|TD Aggressive Sequential (p.58): \"compares the high with the high two price bars earlier\" (high >= high two bars earlier).",
            "E|Sell on the close of countdown bar thirteen."],
          cd_note),
        d("tdperl_9139_long", "TD Sequential 9-13-9 Buy", "Ch 1 · TD Sequential", "Long", NotTestable("no exit in the book: Perl p.48 \"it's up to you to manage the risk and the take-profit level ... TD Sequential is an indicator, not a system\" — entry and TD risk-level stop are exact, but a trade needs an exit, and the vault does not invent one"),
          seq_rules!["F|After a TD Buy Countdown 13 (rules as tdperl_seq13_long).",
            "F|Validation (p.29): \"1. The TD Buy Setup must not begin before or on the same price bar as the completed TD Buy Countdown, 2. The ensuing bullish TD Buy Setup must be preceded by a TD Price Flip [bullish flip after the 13], and 3. No completed TD Sell Setup should occur prior to the appearance of the TD Buy Setup.\"",
            "E|\"a fresh long position can be established on the close of the completed TD Sequential 9-13-9\" (setup bar nine).",
            "X|Risk (p.31, Perl's own method): \"Subtract the true range of the price bar with the lowest true low in the TD Buy Countdown and ensuing TD Buy Setup from the true low of that bar\" (all bars from countdown bar 1 to setup bar 9)."],
          cd_note),
        d("tdperl_9139_short", "TD Sequential 9-13-9 Sell", "Ch 1 · TD Sequential", "Short", NotTestable("no exit in the book: Perl p.48 \"it's up to you to manage the risk and the take-profit level ... TD Sequential is an indicator, not a system\" — entry and TD risk-level stop are exact, but a trade needs an exit, and the vault does not invent one"),
          seq_rules!["F|Mirror of the buy 9-13-9 (p.41): sell setup not beginning before/on the sell 13, preceded by a bearish TD Price Flip, no completed TD Buy Setup in between.",
            "E|Sell on the close of the completed 9-13-9 sell setup bar nine.",
            "X|Risk (p.41): add the true range of the bar with the highest true high in the sell countdown and ensuing sell setup to that true high."],
          cd_note),
        d("tdperl_combo1_long", "TD Combo Buy 13 (Version I, strict)", "Ch 2 · TD Combo", "Long", NotTestable("no exit in the book: Perl p.48 \"it's up to you to manage the risk and the take-profit level ... TD Sequential is an indicator, not a system\" — entry and TD risk-level stop are exact, but a trade needs an exit, and the vault does not invent one"),
          seq_rules!["F|TD Combo Buy Countdown begins retrospectively at bar one of the completed TD Buy Setup (p.60). Each counted bar, Version I (p.60): \"1. The close must be less than, or equal to, the low two price bars earlier; 2. Each TD Combo Buy Countdown low must be less than, or equal to, the low of the prior price bar; 3. Each TD Combo Buy Countdown close must be less than the previous TD Combo Buy Countdown close; and 4. Each TD Combo Buy Countdown close must be less than the close of the prior price bar.\" Option combo_eq=1 uses <= for 3-4 (settings dialog p.67 shows '<=').",
            "E|\"Establish a long position on the close of bar thirteen\" (p.62).",
            "X|Risk (p.62): lowest true low of combo bars 1-13 (all bars) minus that bar's true range."],
          cd_note),
        d("tdperl_combo1_short", "TD Combo Sell 13 (Version I, strict)", "Ch 2 · TD Combo", "Short", NotTestable("no exit in the book: Perl p.48 \"it's up to you to manage the risk and the take-profit level ... TD Sequential is an indicator, not a system\" — entry and TD risk-level stop are exact, but a trade needs an exit, and the vault does not invent one"),
          seq_rules!["F|Mirror (p.64): close >= high two bars earlier; high >= prior high; close > previous countdown close; close > prior close; counted from sell setup bar one.",
            "E|Sell on the close of combo bar thirteen (p.65)."],
          cd_note),
        d("tdperl_combo2_long", "TD Combo Buy 13 (Version II, less strict)", "Ch 2 · TD Combo", "Long", NotTestable("no exit in the book: Perl p.48 \"it's up to you to manage the risk and the take-profit level ... TD Sequential is an indicator, not a system\" — entry and TD risk-level stop are exact, but a trade needs an exit, and the vault does not invent one"),
          seq_rules!["F|Version II (p.61): Version I conditions for bars 1-10; \"for TD Combo Buy Countdown bars eleven, twelve, and thirteen only, 5. The closes need only be successively lower\".",
            "E|Buy on the close of combo bar thirteen."],
          cd_note),
        d("tdperl_combo2_short", "TD Combo Sell 13 (Version II, less strict)", "Ch 2 · TD Combo", "Short", NotTestable("no exit in the book: Perl p.48 \"it's up to you to manage the risk and the take-profit level ... TD Sequential is an indicator, not a system\" — entry and TD risk-level stop are exact, but a trade needs an exit, and the vault does not invent one"),
          seq_rules!["F|Version II (p.65): bars 11-13 \"only need to be successively higher\".",
            "E|Sell on the close of combo bar thirteen."],
          cd_note),
        d("tdperl_setup_long", "Perfected TD Buy Setup (Perl's rules)", "Ch 1 · TD Setup", "Long", Live,
          &[SETUP,
            "F|Perl's rules (p.13): \"1. When the TD Buy Setup has been perfected, that is, the low of TD Buy Setup bar eight or nine is less than the lows of TD Buy Setup bars six and seven\" (definition box p.11: \"less than, or equal to\"; perfection must be on bar 8 or 9 since entry is the bar-9 close); \"2. When none of the bars within the TD Buy Setup has closed below TDST support\" (true low of the most recently completed TD Sell Setup, p.5-6).",
            "F|Rule 3 \"When the close of TD Buy Setup bar nine is in close proximity to TDST support\" has no number: not applied [judgement]. Optional (c9_lt_c8=1): close of bar 9 below close of bar 8 (\"optional\").",
            "F|p.14: \"I would take the trade only if the difference between the entry price (close of TD Buy Setup bar nine), and TDST Resistance is more than 1.5 times the difference between the close of TD Buy Setup bar nine and the TD risk level\" (rr).",
            "E|Buy on the close of TD Buy Setup bar nine (p.12 Fig 1.11).",
            "X|Stop (p.14): \"identify the TD Buy Setup bar with the lowest true low, and subtract the true range of that bar from its true low\"; intrabar stop.",
            "X|Target (p.14): \"The expectation would be for a return toward the upper-range extreme, as defined by TDST Resistance, in other words, the true high of the most recently completed TD Buy Setup\": sell limit at that level. Stop and target on one bar: stop first [conservative, book silent]."],
          "Book prints no statistics. India: instrument swap only."),
        d("tdperl_setup_short", "Perfected TD Sell Setup (Perl's rules)", "Ch 1 · TD Setup", "Short", Live,
          &[SETUP,
            "F|Perl's rules (p.16): sell setup perfected (high of bar 8 or 9 >= highs of bars 6 and 7); no setup bar closed above TDST resistance (true high of the most recent completed TD Buy Setup); rule 3 'close proximity' not applied [judgement]; reward to TDST support > 1.5 x risk (p.17).",
            "E|Sell on the close of TD Sell Setup bar nine.",
            "X|Stop: highest true high of the setup plus that bar's true range (p.17). Target: true low of the setup = TDST support (p.17)."],
          "Book prints no statistics. India: instrument swap only."),
        d("tdperl_tdline_long", "TD Supply Line qualified upside breakout", "Ch 4 · TD Lines", "Long", Live,
          &["F|TD Supply Point (p.91): a high with `level` lower highs on either side (strict); Perl's preference Level One (p.92). Confirmed only after the `level` bars to its right close.",
            "F|TD Supply Line (p.92-93): \"connect the two most recent TD Supply Points of the same magnitude (from right to left) to draw a downward-sloping TD Supply Line\"; Fig 4.2: most-recent point and \"the next-most-recent, higher\" point. Extended bar by bar.",
            "F|Breakout bar X = first bar whose high exceeds the line. Qualified if any of (p.98-100, settings p.101 all on): Q1 close of X-1 < close of X-2; Q2 open of X above the line; Q3 close(X-1) + (close(X-1) - min(low(X-1), close(X-2))) < line at X.",
            "E|\"Buy on the break of the TD Supply Line\" (intrabar buy stop at the line; a gap above fills at the open). Disqualified breaks are not traded (the book's intrabar fade has no exit).",
            "X|Failure exits on the bar after the breakout (p.100-101): \"1. if the open ... is below the TD Supply Line breakout level, exit at the open. 2. if the open ... is below the TD Supply Line, and the bar closes below the TD Supply Line breakout level, exit at the close. 3. if the high of the bar following the TD Supply Line break fails to post a higher high than the high of the breakout bar, exit at the close.\"",
            "X|Objective (p.100): \"identify the lowest true low below the TD Supply Line, and draw a perpendicular line up to the corresponding TD Supply Line above it. Add the value of the perpendicular line to the point where the qualified breakout occurred.\" [Book silent on the window] lowest true low from the older line point to the bar before X. Sell limit at the objective.",
            "X|\"The objective will remain intact unless the market has a qualified downside breakout\" (p.100): exit at a qualified TD Demand Line downside break (its stop price). Same bar as the objective: the adverse exit first [book silent]."],
          "Book prints no statistics. Level is a parameter (Perl prefers Level One). India: instrument swap only."),
        d("tdperl_tdline_short", "TD Demand Line qualified downside breakout", "Ch 4 · TD Lines", "Short", Live,
          &["F|TD Demand Point: a low with `level` higher lows on either side; TD Demand Line from the most recent demand point and the next-most-recent, lower one (p.92-93).",
            "F|Breakout bar X = first bar whose low is below the line. Qualified (p.94-96): Q1 close of X-1 > close of X-2; Q2 open below the line; Q3 close(X-1) - (max(high(X-1), close(X-2)) - close(X-1)) > line at X.",
            "E|\"sell on the break of the TD Demand Line\" (intrabar sell stop; gap fills at the open).",
            "X|Failure exits (p.97): open of next bar above the breakout level -> exit at the open; open above the line and close above the breakout level -> exit at the close; low of the next bar not below the breakout bar's low -> exit at the close.",
            "X|Objective (p.96): breakout level minus the distance from the highest true high above the line to the line. Exit at a qualified TD Supply Line upside break."],
          "Book prints no statistics. India: instrument swap only."),
        d("tdperl_rebo", "TD Range Expansion Breakout (REBO), daily, open base", "Ch 9 · TD REBO", "Long/Short", Live,
          &["F|p.158: \"My preference is to apply TD REBO to daily price bars and use the current price bar's open as the base. First, multiply the previous day's true range separately by 0.382 and by 0.618. Then to calculate upside levels, add those values to the current day's opening price. To determine downside levels, subtract them from the current day's opening price.\"",
            "E|p.157: TD REBO \"attempts to identify that initial price break, to enable traders to participate in the move, until a signal occurs in the opposite direction\": buy stop at open + 0.382 x prior true range, sell stop at open - 0.382 x prior true range; stop-and-reverse.",
            "X|Reverse on the opposite break. [Book silent] a bar touching both levels: flat -> no entry; in a position -> reverse at the adverse level only.",
            "F|Not coded: the qualified/fade intraday versions (0.618 objective, \"on an intraday basis\") have no exit."],
          "Book prints no statistics. Always in the market after the first break. India: instrument swap only."),
        d("tdperl_diff_up", "TD Differential up arrow · next-bar test", "Ch 10 · TD Differential", "Long", Live,
          &["F|Buying pressure = close - true low; selling pressure = true high - close (p.165).",
            "E|Up arrow (p.165): \"1. There must be two consecutive closes, each one less than the one prior, 2. Buying pressure for the current price bar must exceed buying pressure from the previous price bar, and 3. Selling pressure for the current price bar must be less than selling pressure from the previous price bar.\" Buy at the close.",
            "X|The book's forecast horizon (p.165): \"it suggests that the high of the next price bar will be greater than the high of the current price bar, or at the very least, that the next price bar's close will be above the current price bar's close\": exit at the next close (a test of the stated forecast, not a book trading rule)."],
          "Forecast test. Book prints no statistics."),
        d("tdperl_diff_down", "TD Differential down arrow · next-bar test", "Ch 10 · TD Differential", "Short", Live,
          &["E|Down arrow (p.166): two consecutive higher closes, selling pressure above the prior bar's, buying pressure below the prior bar's. Sell at the close.",
            "X|Forecast (p.166): \"the next price bar's close will be beneath the current price bar's close\": exit at the next close."],
          "Forecast test. Book prints no statistics."),
        d("tdperl_clop_long", "TD Clop buy · same-day test", "Ch 1 · alternative entries", "Long", Live,
          &["E|p.25: \"1. The open of the current price bar must be below the close and open of the previous price bar, and 2. The market must subsequently trade above both the open and close of the previous bar.\" Buy stop at max(previous open, previous close).",
            "X|\"there is a greater chance that upside momentum will continue into the close\" (p.25): exit at the same day's close (forecast test)."],
          "Forecast test of a stand-alone pattern (\"You can use all these indicators in isolation\", p.25)."),
        d("tdperl_clop_short", "TD Clop sell · same-day test", "Ch 1 · alternative entries", "Short", Live,
          &["E|p.38: open above the previous close and open, then trades below both: sell stop at min(previous open, previous close).",
            "X|\"increases the probability that downward momentum will continue into the close\": exit at the close."],
          "Forecast test. The printed condition 1 (p.38) is garbled; the sell rule is the mirror of the buy rule."),
        d("tdperl_clopwin_long", "TD Clopwin buy · next-bar test", "Ch 1 · alternative entries", "Long", Live,
          &["E|p.26: \"1. The open and close of the current price bar must be contained within the open and close range of the previous price bar, and 2. The close of the current price bar must be above the close of the prior price bar.\" Buy at the close. [Book silent] 'contained' inclusive.",
            "X|\"increases the probability that upside momentum will be sustained into the next price bar\": exit at the next close (forecast test)."],
          "Forecast test."),
        d("tdperl_clopwin_short", "TD Clopwin sell · next-bar test", "Ch 1 · alternative entries", "Short", Live,
          &["E|p.38: open and close inside the previous bar's open-close range; close below the prior close. Sell at the close.",
            "X|Exit at the next close (forecast test)."],
          "Forecast test."),
        // ---- seen, not testable
        d("tdperl_tdma1", "TD Moving Average I", "Ch 8 · TD Moving Averages", "Long/Short", NotTestable("entry is \"following dips to the moving average of the lows\" (p.149) with no trigger price or bar; only the exit (close below the 5-bar MA of lows, next open below it, p.146) is exact"),
          &["F|5-bar MA of lows plotted for 4 bars after a low above all twelve prior lows (p.146).", "E|\"initiate either long positions, following dips to the moving average of the lows\" (p.149) [no exact trigger]", "X|\"remain [long] unless the market closes beneath the five-bar moving average of the lows and then the next bar opens below it\" (p.146)"], opt_note),
        d("tdperl_tdma2", "TD Moving Average II", "Ch 8 · TD Moving Averages", "Long/Short", NotTestable("3- and 34-bar SMAs with rates of change: \"general tone\" only, no entry or exit rule (p.149-151)"), &["F|3-bar and 34-bar SMA of closes; ROC of the 3-bar vs 2 bars earlier, 34-bar vs 1 bar earlier (p.149-151)."], opt_note),
        d("tdperl_rei", "TD REI with TD POQ", "Ch 7 · TD Oscillators", "Long/Short", NotTestable("no exit or stop in the book, and the printed formula is internally inconsistent (bar value 'X = (H-H2)+(L-L2)' vs 'the difference between the high and low'; denominator 'H5-L5' vs a stated -100..+100 range; Condition II repeats 'high of two bars ago', p.128-132)"),
          &["F|TD REI(5), overbought +40 / oversold -40, duration 6 (p.129-132).", "E|POQ buy (p.130): REI below -40 for six or fewer periods; lower close; next open <= the two prior highs; market trades above the open and above either prior high."], opt_note),
        d("tdperl_demarker1", "TD DeMarker I", "Ch 7 · TD Oscillators", "Long/Short", NotTestable("entries only (p.134-135), no exit or stop; duration threshold differs between buy (13 bars) and sell (6 bars) as printed"), &["E|Buy: reading below 40 for thirteen bars or fewer; close below the low one or two bars earlier; close below the prior close and the open; next open <= either prior close; trades above one of the prior closes (p.134)."], opt_note),
        d("tdperl_demarker2", "TD DeMarker II", "Ch 7 · TD Oscillators", "Long/Short", NotTestable("formula only, no signal or exit rule (p.136-137)"), &["F|8-bar buying vs selling pressure ratio (p.136-137)."], opt_note),
        d("tdperl_pressure", "TD Pressure", "Ch 7 · TD Oscillators", "Long/Short", NotTestable("volume oscillator with arrows but no trade rule (p.137-139); index volume is also missing for most NSE indices"), &["F|(close-open)/range x volume, 5-bar ratio (p.139)."], opt_note),
        d("tdperl_roc", "TD ROC / TD Alignment", "Ch 7 · TD Oscillators", "Long/Short", NotTestable("overbought/oversold readings (97.5/102.5) and a composite score with no trade rule (p.140-142)"), &["F|close / close 12 bars earlier; Alignment = +1/-1 votes of five oscillators (p.140-142)."], opt_note),
        d("tdperl_dwave", "TD D-Wave", "Ch 3 · TD D-Wave", "Long/Short", NotTestable("wave labels shift retroactively and \"two alternate TD D-Wave counts are evident\" (p.83), resolved with RSI judgement; trading advice has no exact trigger/exit (p.86-87)"), &["F|21-bar low close starts wave 1, 13-bar high close confirms, 8-bar low close starts wave 2, ... (p.74-76)."], opt_note),
        d("tdperl_retrace", "TD Relative / Absolute Retracement and Arc", "Ch 5 · TD Retracements", "Long/Short", NotTestable("the reference high/low is chosen by the trader (\"your chosen reference low\", \"if you believe the market is basing\", p.103, 111)"), &["F|0.382/0.618/Magnet levels from a reference high/low; qualified breaks use the TD Line qualifiers (p.103-108)."], opt_note),
        d("tdperl_trendfactor", "TD Trend Factor / TD Propulsion", "Ch 6", "Long/Short", NotTestable("projection levels without trade rules; the Propulsion points X, Y, Z are not defined exactly (p.115-123)"), &["F|5.556% steps from a qualified high/low (p.116-118)."], opt_note),
        d("tdperl_rangeproj", "TD Range Projection / TD Channels", "Ch 9", "Long/Short", NotTestable("forecast levels and 'often' observations without entry/exit rules (p.153-163)"), &["F|Projected high/low from O/H/L/C; Channel I 3-bar MA of lows x 1.03 / highs x 0.97; Channel II highs x 0.995 (p.154-162)."], opt_note),
        d("tdperl_waldo", "TD Waldo patterns 2-8, TD Reverse/Anti-Differential, TD Camouflage, TD Open, TD Trap (stand-alone)", "Ch 10-11", "Long/Short", NotTestable("pattern descriptions with no stated horizon, entry or exit (p.167-174, 25-26); Open/Trap/Camouflage are coded as entry options of the countdown strategies"), &["F|See p.167-174."], opt_note),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let side = if id.ends_with("_long") || id.ends_with("_up") { "Buy" } else if id.ends_with("_short") || id.ends_with("_down") { "Sell" } else { "Both" };
    Some(match id.rsplit_once('_').map_or(id, |x| x.0) {
        "tdperl_seq13" => ("tdperl_seq", "TD Sequential (Perl)", if side == "Buy" { "Buy 13" } else { "Sell 13" }),
        "tdperl_agg13" => ("tdperl_seq", "TD Sequential (Perl)", if side == "Buy" { "Aggressive buy 13" } else { "Aggressive sell 13" }),
        "tdperl_9139" => ("tdperl_seq", "TD Sequential (Perl)", if side == "Buy" { "9-13-9 buy" } else { "9-13-9 sell" }),
        "tdperl_combo1" => ("tdperl_combo", "TD Combo (Perl)", if side == "Buy" { "Version I buy" } else { "Version I sell" }),
        "tdperl_combo2" => ("tdperl_combo", "TD Combo (Perl)", if side == "Buy" { "Version II buy" } else { "Version II sell" }),
        "tdperl_setup" => ("tdperl_setup", "TD Setup (Perl's rules)", side),
        "tdperl_tdline" => ("tdperl_tdline", "TD Lines", side),
        "tdperl_diff" => ("tdperl_short_term", "TD short-term patterns", if side == "Buy" { "Differential up" } else { "Differential down" }),
        "tdperl_clop" => ("tdperl_short_term", "TD short-term patterns", if side == "Buy" { "Clop buy" } else { "Clop sell" }),
        "tdperl_clopwin" => ("tdperl_short_term", "TD short-term patterns", if side == "Buy" { "Clopwin buy" } else { "Clopwin sell" }),
        _ if id == "tdperl_rebo" => ("tdperl_rebo", "TD REBO", "Daily, open base"),
        _ if id.starts_with("tdperl_") => ("tdperl_other", "DeMark indicators (not testable)", "Seen"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let setup = || vec![p("setup_n", "TD Setup bars", 9., 7., 11., 1.), p("look", "Setup: close vs close N bars earlier", 4., 3., 5., 1.)];
    let cd = |entry: bool| {
        let mut v = setup();
        v.extend([p("cd_n", "Countdown bars", 13., 11., 15., 1.), p("cd_look", "Countdown: vs N bars earlier", 2., 1., 3., 1.),
            p("mult", "Recycle multiplier upper bound", 1.618, 1.382, 2.618, 0.236), p("recycle_n", "Recycle: setup length (R)", 18., 14., 22., 1.),
            p("recycle_other", "Recycle when neither qualifier applies (0 keep [book silent], 1 restart)", 0., 0., 1., 1.),
            p("max_bars", "Time exit after N bars (0 off) [not from the book]", 0., 0., 24., 6.)]);
        if entry { v.push(p("entry", "Entry: 0 close of 13, 1 flip, 2 Camouflage, 3 Clop, 4 Clopwin, 5 Open, 6 Trap", 0., 0., 6., 1.)); }
        v
    };
    Some(match id {
        "tdperl_seq13_long" | "tdperl_seq13_short" | "tdperl_agg13_long" | "tdperl_agg13_short" => {
            let mut v = cd(true);
            v.extend([p("term_qual", "Bar 13 vs close of bar 8 qualifier", 1., 0., 1., 1.), p("q85", "Bar 8 vs close of bar 5 qualifier", 0., 0., 1., 1.),
                p("term_open", "Termination count on the open (DeMark)", 0., 0., 1., 1.)]);
            v
        }
        "tdperl_9139_long" | "tdperl_9139_short" => {
            let mut v = cd(false);
            v.extend([p("term_qual", "Bar 13 vs close of bar 8 qualifier", 1., 0., 1., 1.), p("q85", "Bar 8 vs close of bar 5 qualifier", 0., 0., 1., 1.),
                p("term_open", "Termination count on the open (DeMark)", 0., 0., 1., 1.)]);
            v
        }
        "tdperl_combo1_long" | "tdperl_combo1_short" | "tdperl_combo2_long" | "tdperl_combo2_short" => {
            let mut v = cd(true);
            v.push(p("combo_eq", "Use <= for the close comparisons (settings dialog p.67)", 0., 0., 1., 1.));
            v
        }
        "tdperl_setup_long" | "tdperl_setup_short" => {
            let mut v = setup();
            v.extend([p("rr", "Reward/risk to TDST must exceed (0 off)", 1.5, 0., 3., 0.5), p("c9_lt_c8", "Optional: bar 9 close beyond bar 8 close", 0., 0., 1., 1.)]);
            v
        }
        "tdperl_tdline_long" | "tdperl_tdline_short" => vec![p("level", "TD Point level", 1., 1., 5., 1.)],
        "tdperl_rebo" => vec![p("k", "Breakout fraction of prior true range", 0.382, 0.236, 0.618, 0.118)],
        _ if id.starts_with("tdperl_") && catalogue().iter().any(|s| s.id == id) => vec![],
        _ => return None,
    })
}

// ---------------------------------------------------------------- prices

/// Bars up to `to`; `short` mirrors them so that every sell rule is the buy rule.
struct Bars { o: Vec<f64>, h: Vec<f64>, l: Vec<f64>, c: Vec<f64>, th: Vec<f64>, tl: Vec<f64>, s: f64 }

impl Bars {
    fn new(x: &Ctx, to: usize, short: bool) -> Bars {
        let n = to + 1;
        let s = if short { -1. } else { 1. };
        let (o, c): (Vec<f64>, Vec<f64>) = (0..n).map(|i| (s * x.o[i], s * x.c[i])).unzip();
        let (h, l): (Vec<f64>, Vec<f64>) = (0..n).map(|i| if short { (-x.l[i], -x.h[i]) } else { (x.h[i], x.l[i]) }).unzip();
        let th = (0..n).map(|i| if i == 0 { h[0] } else { h[i].max(c[i - 1]) }).collect();
        let tl = (0..n).map(|i| if i == 0 { l[0] } else { l[i].min(c[i - 1]) }).collect();
        Bars { o, h, l, c, th, tl, s }
    }
    fn n(&self) -> usize { self.c.len() }
    /// Real price of a (possibly mirrored) level.
    fn px(&self, v: f64) -> f64 { self.s * v }
    /// TD risk level over bars a..=b: lowest true low minus that bar's true range (latest bar on ties).
    fn risk(&self, a: usize, b: usize) -> f64 {
        let j = (a..=b).rev().min_by(|&p, &q| self.tl[p].partial_cmp(&self.tl[q]).unwrap()).unwrap();
        self.tl[j] - (self.th[j] - self.tl[j])
    }
}

/// Buy-setup count on bars `b` (c < c[look] streak started by a bearish price flip); `up` = sell setup.
fn setup_count(b: &Bars, look: usize, up: bool) -> Vec<u32> {
    let mut v = vec![0u32; b.n()];
    let beyond = |i: usize| if up { b.c[i] > b.c[i - look] } else { b.c[i] < b.c[i - look] };
    let before = |i: usize| if up { b.c[i] < b.c[i - look] } else { b.c[i] > b.c[i - look] };
    for i in look + 1..b.n() {
        if beyond(i) {
            v[i] = if v[i - 1] > 0 { v[i - 1] + 1 } else if before(i - 1) { 1 } else { 0 };
        }
    }
    v
}

// ---------------------------------------------------------------- countdowns

#[derive(Clone, Copy, PartialEq)]
enum Kind { Seq, Agg, Combo1, Combo2 }

struct Opts { kind: Kind, setup_n: u32, look: usize, cd_n: usize, cd_look: usize, term_qual: bool, q85: bool, term_open: bool,
    mult: f64, recycle_n: u32, recycle_other: bool, combo_eq: bool }

/// A completed countdown 13 (in mirrored-bar coordinates for sells).
#[derive(Clone, Copy, Debug, PartialEq)]
struct Sig { i: usize, first: usize, risk: f64 }

struct Cd { s1: usize, end: usize, tdst: f64, lo: f64, hi: f64, counted: Vec<usize> }

fn countdowns(b: &Bars, o: &Opts) -> Vec<Sig> {
    let (bs, ss) = (setup_count(b, o.look, false), setup_count(b, o.look, true));
    let sn = o.setup_n as usize;
    let lt = |a: f64, z: f64| if o.combo_eq { a <= z } else { a < z };
    // Does bar j count as the next countdown bar?
    let qualifies = |cd: &Cd, j: usize| -> bool {
        if j < o.cd_look.max(2) { return false; }
        let k = cd.counted.len();
        let last = cd.counted.last().map(|&m| b.c[m]);
        let mut ok = match o.kind {
            Kind::Seq => if k + 1 == o.cd_n && o.term_open { b.o[j] <= b.l[j - o.cd_look] } else { b.c[j] <= b.l[j - o.cd_look] },
            Kind::Agg => b.l[j] <= b.l[j - o.cd_look],
            Kind::Combo1 | Kind::Combo2 => {
                let succ = last.map_or(true, |v| lt(b.c[j], v));
                if o.kind == Kind::Combo2 && k >= 10 { succ } else { b.c[j] <= b.l[j - o.cd_look] && b.l[j] <= b.l[j - 1] && lt(b.c[j], b.c[j - 1]) && succ }
            }
        };
        if matches!(o.kind, Kind::Seq | Kind::Agg) {
            if o.term_qual && k + 1 == o.cd_n && o.cd_n >= 8 { ok &= b.l[j] <= b.c[cd.counted[o.cd_n - 6]]; }
            if o.q85 && k + 1 == 8 && o.cd_n > 8 { ok &= b.l[j] <= b.c[cd.counted[4]]; }
        }
        ok
    };
    let start = |s1: usize, s9: usize| Cd {
        s1, end: s9,
        tdst: (s1..=s9).map(|j| b.th[j]).fold(f64::MIN, f64::max),
        lo: (s1..=s9).map(|j| b.tl[j]).fold(f64::MAX, f64::min),
        hi: (s1..=s9).map(|j| b.th[j]).fold(f64::MIN, f64::max),
        counted: vec![],
    };
    let combo = matches!(o.kind, Kind::Combo1 | Kind::Combo2);
    let mut out = vec![];
    let mut st: Option<Cd> = None;
    let mut cur: Option<usize> = None; // bar one of the buy setup in progress
    for i in 0..b.n() {
        if bs[i] == 1 { cur = Some(i) } else if bs[i] == 0 { cur = None }
        // "Use Maximum Setup": the active setup's range grows while it extends beyond nine.
        if let Some(cd) = st.as_mut() {
            if cur == Some(cd.s1) && i > cd.end { cd.end = i; cd.lo = cd.lo.min(b.tl[i]); cd.hi = cd.hi.max(b.th[i]); }
        }
        if st.as_ref().is_some_and(|cd| ss[i] == o.setup_n || b.tl[i] > cd.tdst) { st = None; }
        let mut retro: Option<usize> = None; // recount from this bar up to i-1
        if let Some(s) = cur.filter(|&s| bs[i] == o.recycle_n && st.as_ref().is_some_and(|cd| cd.s1 != s)) {
            st = Some(start(s, s + sn - 1));
            retro = Some(if combo { s } else { s + sn - 1 });
        }
        if bs[i] == o.setup_n && sn > 0 {
            let s1 = i + 1 - sn;
            let restart = match st.as_ref() {
                None => true,
                Some(cd) => {
                    let n = start(s1, i);
                    let (cmin, cmax) = (s1..=i).fold((f64::MAX, f64::MIN), |(a, z), j| (a.min(b.c[j]), z.max(b.c[j])));
                    let r = (n.hi - n.lo) / (cd.hi - cd.lo);
                    let within = cmin >= cd.lo && cmax <= cd.hi && n.lo >= cd.lo;
                    if r >= 1.0 && r < o.mult { true } else if within { false } else { o.recycle_other }
                }
            };
            if restart {
                st = Some(start(s1, i));
                retro = if combo { Some(s1) } else { None };
            }
        }
        if let (Some(cd), Some(r)) = (st.as_mut(), retro) {
            for j in r..i {
                if qualifies(cd, j) { cd.counted.push(j); }
            }
        }
        if let Some(cd) = st.as_mut() {
            if qualifies(cd, i) {
                cd.counted.push(i);
                if cd.counted.len() == o.cd_n {
                    out.push(Sig { i, first: cd.counted[0], risk: b.risk(cd.counted[0], i) });
                    st = None;
                }
            }
        }
    }
    out
}

/// 9-13-9: the first buy setup after each buy 13 that begins after it, follows a bullish flip, with no sell setup between.
fn nine_thirteen_nine(b: &Bars, sigs: &[Sig], o: &Opts) -> Vec<Sig> {
    let (bs, ss) = (setup_count(b, o.look, false), setup_count(b, o.look, true));
    let sn = o.setup_n as usize;
    let mut out = vec![];
    for (k, s) in sigs.iter().enumerate() {
        let next13 = sigs.get(k + 1).map_or(b.n(), |z| z.i);
        let mut flip: Option<usize> = None;
        for j in s.i + 1..b.n().min(next13) {
            if ss[j] == o.setup_n { break; }
            if j > o.look && b.c[j] > b.c[j - o.look] && b.c[j - 1] < b.c[j - 1 - o.look] && flip.is_none() { flip = Some(j); }
            if bs[j] == o.setup_n {
                let s1 = j + 1 - sn;
                if s1 > s.i && flip.is_some_and(|f| f < s1) {
                    out.push(Sig { i: j, first: s.first, risk: b.risk(s.first, j) });
                    break;
                }
            }
        }
    }
    out
}

// ---------------------------------------------------------------- trades

fn trade(b: &Bars, i: usize, px: f64, kind: &'static str) -> Trade {
    Trade { dir: b.s, legs: vec![Leg { i, px: b.px(px), w: 1. }], exit_i: i, exit_px: b.px(px), reason: String::new(),
        entry_kind: kind, signal_i: i, open: false, exits: vec![] }
}
fn close(mut t: Trade, b: &Bars, i: usize, px: f64, why: &str) -> Trade {
    t.exit_i = i; t.exit_px = b.px(px); t.signal_i = i; t.reason = why.into(); t
}
fn finish(cur: Option<Trade>, x: &Ctx, to: usize, out: &mut Vec<Trade>) {
    if let Some(mut t) = cur { t.exit_i = to; t.exit_px = x.c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true; out.push(t); }
}

/// Long side (on mirrored bars for shorts) of a countdown signal with the book stop and the opposite signal as exit.
fn run_countdown(x: &Ctx, b: &Bars, sigs: &[Sig], opp: &[usize], entry: u32, max_bars: usize, from: usize, to: usize) -> Vec<Trade> {
    let n = b.n();
    let mut out = vec![];
    let mut cur: Option<(Trade, f64)> = None;
    let mut waiting: Option<Sig> = None; // conservative / alternative entry pending
    let (mut si, mut oi) = (0, 0);
    for i in 0..n {
        let opp_now = { while oi < opp.len() && opp[oi] < i { oi += 1 } oi < opp.len() && opp[oi] == i };
        let sig_now = { while si < sigs.len() && sigs[si].i < i { si += 1 } sigs.get(si).filter(|s| s.i == i).copied() };
        if let Some((t, risk)) = cur.take() {
            let e = t.legs[0].i;
            if b.o[i] <= risk { out.push(close(t, b, i, b.o[i], "TD risk level (gap)")); }
            else if b.l[i] <= risk { out.push(close(t, b, i, risk, "TD risk level")); }
            else if opp_now { out.push(close(t, b, i, b.c[i], "Opposite 13 [not from the book]")); }
            else if max_bars > 0 && i >= e + max_bars { out.push(close(t, b, i, b.c[i], "Time exit [not from the book]")); }
            else { cur = Some((t, risk)); }
            continue; // no re-entry on an exit bar
        }
        if let Some(s) = sig_now { waiting = Some(s); }
        let Some(s) = waiting else { continue };
        if entry == 0 {
            waiting = None;
            if i >= from && i < to { cur = Some((trade(b, i, b.c[i], "close"), s.risk)); }
            continue;
        }
        if i == s.i || i < 5 { continue; } // alternative entries start on the bar after the 13
        if opp_now || b.l[i] <= s.risk { waiting = None; continue; } // [book silent] signal void
        let (o, h, l, c) = (b.o[i], b.h[i], b.l[i], b.c[i]);
        let (po, pc, ph, pl) = (b.o[i - 1], b.c[i - 1], b.h[i - 1], b.l[i - 1]);
        let (fill, kind) = match entry {
            1 => ((c > b.c[i - 4]).then_some(c), "close"),
            2 => ((c < pc && c > o && l < b.tl[i - 2]).then_some(c), "close"),
            3 => ((o < pc && o < po && h > pc.max(po)).then_some(pc.max(po)), "stop"),
            4 => ((o.max(c) <= po.max(pc) && o.min(c) >= po.min(pc) && c > pc).then_some(c), "close"),
            5 => ((o < pl && h > pl).then_some(pl), "stop"),
            _ => ((o >= pl && o <= ph && h > ph).then_some(ph), "stop"),
        };
        if let Some(px) = fill {
            waiting = None;
            if i >= from && i < to { cur = Some((trade(b, i, px, kind), s.risk)); }
        }
    }
    finish(cur.map(|c| c.0), x, to, &mut out);
    out
}

fn run_setup(x: &Ctx, b: &Bars, look: usize, sn: u32, rr: f64, c9: bool, from: usize, to: usize) -> Vec<Trade> {
    let (bs, ss) = (setup_count(b, look, false), setup_count(b, look, true));
    let n9 = sn as usize;
    let mut support: Option<f64> = None; // TDST support: true low of the latest completed opposite setup
    let mut out = vec![];
    let mut cur: Option<(Trade, f64, f64)> = None;
    for i in 0..b.n() {
        if let Some((t, risk, tgt)) = cur.take() {
            if b.o[i] <= risk { out.push(close(t, b, i, b.o[i], "TD risk level (gap)")); }
            else if b.l[i] <= risk { out.push(close(t, b, i, risk, "TD risk level")); }
            else if b.o[i] >= tgt { out.push(close(t, b, i, b.o[i], "TDST target (gap)")); }
            else if b.h[i] >= tgt { out.push(close(t, b, i, tgt, "TDST target")); }
            else { cur = Some((t, risk, tgt)); }
        } else if bs[i] == sn && n9 >= 4 && i >= from && i < to {
            let s1 = i + 1 - n9;
            let perfected = b.l[i - 1].min(b.l[i]) <= b.l[i - 3].min(b.l[i - 2]);
            let held = support.is_some_and(|sup| (s1..=i).all(|j| b.c[j] >= sup));
            let risk = b.risk(s1, i);
            let tgt = (s1..=i).map(|j| b.th[j]).fold(f64::MIN, f64::max);
            let ok_rr = rr <= 0. || tgt - b.c[i] > rr * (b.c[i] - risk);
            if perfected && held && ok_rr && (!c9 || b.c[i] < b.c[i - 1]) {
                cur = Some((trade(b, i, b.c[i], "close"), risk, tgt));
            }
        }
        if ss[i] == sn && n9 > 0 { support = Some((i + 1 - n9..=i).map(|j| b.tl[j]).fold(f64::MAX, f64::min)); }
    }
    finish(cur.map(|c| c.0), x, to, &mut out);
    out
}

// ---------------------------------------------------------------- TD Lines

#[derive(Clone, Copy, Debug)]
struct Brk { i: usize, level: f64, fill: f64, target: f64, p1: usize, p2: usize }

/// Qualified upside breaks of TD Supply Lines on bars `b` (demand-line downside breaks on mirrored bars).
fn line_breaks(b: &Bars, lvl: usize) -> Vec<Brk> {
    let n = b.n();
    let point = |p: usize| p >= lvl && p + lvl < n && (1..=lvl).all(|k| b.h[p] > b.h[p - k] && b.h[p] > b.h[p + k]);
    let mut pts: Vec<usize> = vec![];
    let mut broken: Option<(usize, usize)> = None;
    let mut out = vec![];
    for i in 0..n {
        // a point at p is confirmed by the close of p+lvl and usable from bar p+lvl+1
        if i > lvl && point(i - 1 - lvl) { pts.push(i - 1 - lvl); }
        let Some(&p1) = pts.last() else { continue };
        let Some(&p2) = pts.iter().rev().skip(1).find(|&&p| b.h[p] > b.h[p1]) else { continue };
        if broken == Some((p2, p1)) || i < 2 { continue; }
        let line = |j: usize| b.h[p1] + (b.h[p1] - b.h[p2]) / (p1 - p2) as f64 * (j as f64 - p1 as f64);
        let lv = line(i);
        if b.h[i] <= lv { continue; }
        broken = Some((p2, p1));
        let q1 = b.c[i - 1] < b.c[i - 2];
        let q2 = b.o[i] > lv;
        let q3 = b.c[i - 1] + (b.c[i - 1] - b.l[i - 1].min(b.c[i - 2])) < lv;
        if q1 || q2 || q3 {
            let j = (p2..i).rev().min_by(|&p, &q| b.tl[p].partial_cmp(&b.tl[q]).unwrap()).unwrap();
            out.push(Brk { i, level: lv, fill: b.o[i].max(lv), target: lv + (line(j) - b.tl[j]), p1, p2 });
        }
    }
    out
}

fn run_lines(x: &Ctx, b: &Bars, m: &Bars, lvl: usize, from: usize, to: usize) -> Vec<Trade> {
    let ups = line_breaks(b, lvl);
    let downs: Vec<Brk> = line_breaks(m, lvl); // mirrored: qualified opposite (adverse) breaks
    let mut out = vec![];
    let mut cur: Option<(Trade, Brk)> = None;
    let (mut ui, mut di) = (0, 0);
    for i in 0..b.n() {
        let adverse = { while di < downs.len() && downs[di].i < i { di += 1 } downs.get(di).filter(|d| d.i == i).map(|d| -d.fill) };
        let up = { while ui < ups.len() && ups[ui].i < i { ui += 1 } ups.get(ui).filter(|u| u.i == i).copied() };
        if let Some((t, k)) = cur.take() {
            let tgt = k.target;
            let after = i == k.i + 1;
            let line_here = { let (p1, p2) = (k.p1, k.p2); b.h[p1] + (b.h[p1] - b.h[p2]) / (p1 - p2) as f64 * (i as f64 - p1 as f64) };
            if after && b.o[i] < k.level { out.push(close(t, b, i, b.o[i], "Open failure (p.100)")); }
            else if let Some(px) = adverse { out.push(close(t, b, i, px, "Qualified opposite TD Line break (p.100)")); }
            else if b.o[i] >= tgt { out.push(close(t, b, i, b.o[i], "TD Line objective (gap)")); }
            else if b.h[i] >= tgt { out.push(close(t, b, i, tgt, "TD Line objective")); }
            else if after && b.o[i] < line_here && b.c[i] < k.level { out.push(close(t, b, i, b.c[i], "Close failure (p.101)")); }
            else if after && b.h[i] <= b.h[k.i] { out.push(close(t, b, i, b.c[i], "High failure (p.101)")); }
            else { cur = Some((t, k)); }
            continue;
        }
        if let Some(k) = up {
            if i >= from && i < to {
                let t = trade(b, i, k.fill, if k.fill > b.o[i] { "stop" } else { "open" });
                if b.h[i] >= k.target { out.push(close(t, b, i, k.target.max(k.fill), "TD Line objective (entry bar)")); }
                else { cur = Some((t, k)); }
            }
        }
    }
    finish(cur.map(|c| c.0), x, to, &mut out);
    out
}

// ---------------------------------------------------------------- REBO and short-term patterns

fn run_rebo(x: &Ctx, b: &Bars, k: f64, from: usize, to: usize) -> Vec<Trade> {
    let mut out = vec![];
    let mut cur: Option<Trade> = None;
    for i in from.max(2)..=to {
        let r = b.th[i - 1] - b.tl[i - 1];
        let (up, dn) = (b.o[i] + k * r, b.o[i] - k * r);
        let (hu, hd) = (b.h[i] >= up, b.l[i] <= dn);
        let side = cur.as_ref().map_or(0., |t| t.dir);
        let new = match (side > 0., side < 0., hu, hd) {
            (true, _, _, true) => Some((-1., dn)),
            (_, true, true, _) => Some((1., up)),
            (false, false, true, false) => Some((1., up)),
            (false, false, false, true) => Some((-1., dn)),
            _ => None,
        };
        if let Some((dir, px)) = new {
            if let Some(mut t) = cur.take() { t.exit_i = i; t.exit_px = px; t.signal_i = i; t.reason = "Opposite REBO break".into(); out.push(t); }
            if i == to { break; } // no new position on the last bar
            cur = Some(Trade { dir, legs: vec![Leg { i, px, w: 1. }], exit_i: i, exit_px: px, reason: String::new(), entry_kind: "stop", signal_i: i, open: false, exits: vec![] });
        }
    }
    finish(cur, x, to, &mut out);
    out
}

/// One-bar forecast tests: entry by `sig(i)` -> (price, kind), exit at close of bar i + hold.
fn run_pattern(x: &Ctx, b: &Bars, hold: usize, from: usize, to: usize, sig: &dyn Fn(usize) -> Option<(f64, &'static str)>) -> Vec<Trade> {
    let mut out = vec![];
    let mut busy_until = 0;
    for i in from.max(5)..=to {
        if i + hold > to || (i < busy_until) { continue; }
        if let Some((px, kind)) = sig(i) {
            out.push(close(trade(b, i, px, kind), b, i + hold, b.c[i + hold], if hold == 0 { "Same-day close" } else { "Next close" }));
            busy_until = i + hold;
        }
    }
    let _ = x;
    out
}

// ---------------------------------------------------------------- dispatch

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if from > to || to >= x.c.len() || !catalogue().iter().any(|d| d.id == id && d.status == Status::Live) { return None; }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(0., |d| d.def));
    let u = |k: &str| g(k).round().max(0.) as usize;
    let short = id.ends_with("_short") || id.ends_with("_down");
    let b = Bars::new(x, to, short);
    let m = Bars::new(x, to, !short);
    let opts = |kind| Opts { kind, setup_n: u("setup_n").max(2) as u32, look: u("look").max(1), cd_n: u("cd_n").max(8), cd_look: u("cd_look").max(1),
        term_qual: g("term_qual") >= 0.5, q85: g("q85") >= 0.5, term_open: g("term_open") >= 0.5, mult: g("mult"),
        recycle_n: u("recycle_n") as u32, recycle_other: g("recycle_other") >= 0.5, combo_eq: g("combo_eq") >= 0.5 };
    let base = id.rsplit_once('_').map_or(id, |s| s.0);
    let kind = match base { "tdperl_seq13" | "tdperl_9139" => Some(Kind::Seq), "tdperl_agg13" => Some(Kind::Agg),
        "tdperl_combo1" => Some(Kind::Combo1), "tdperl_combo2" => Some(Kind::Combo2), _ => None };
    Some(if let Some(k) = kind {
        let o = opts(k);
        let mine = countdowns(&b, &o);
        let opp: Vec<usize> = countdowns(&m, &o).iter().map(|s| s.i).collect();
        if base == "tdperl_9139" {
            run_countdown(x, &b, &nine_thirteen_nine(&b, &mine, &o), &opp, 0, u("max_bars"), from, to)
        } else {
            run_countdown(x, &b, &mine, &opp, u("entry") as u32, u("max_bars"), from, to)
        }
    } else {
        match base {
            "tdperl_setup" => run_setup(x, &b, u("look").max(1), u("setup_n") as u32, g("rr"), g("c9_lt_c8") >= 0.5, from, to),
            "tdperl_tdline" => run_lines(x, &b, &m, u("level").max(1), from, to),
            _ if id == "tdperl_rebo" => run_rebo(x, &b, g("k"), from, to),
            "tdperl_diff" => run_pattern(x, &b, 1, from, to, &|i| {
                let (bp, sp) = (|j: usize| b.c[j] - b.tl[j], |j: usize| b.th[j] - b.c[j]);
                (b.c[i] < b.c[i - 1] && b.c[i - 1] < b.c[i - 2] && bp(i) > bp(i - 1) && sp(i) < sp(i - 1)).then_some((b.c[i], "close"))
            }),
            "tdperl_clop" => run_pattern(x, &b, 0, from, to, &|i| {
                let lv = b.o[i - 1].max(b.c[i - 1]);
                (b.o[i] < b.c[i - 1] && b.o[i] < b.o[i - 1] && b.h[i] > lv).then_some((lv, "stop"))
            }),
            "tdperl_clopwin" => run_pattern(x, &b, 1, from, to, &|i| {
                let (po, pc) = (b.o[i - 1], b.c[i - 1]);
                (b.o[i].max(b.c[i]) <= po.max(pc) && b.o[i].min(b.c[i]) >= po.min(pc) && b.c[i] > pc).then_some((b.c[i], "close"))
            }),
            _ => return None,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn rows(c: &[f64]) -> Vec<Row> {
        c.iter().enumerate().map(|(i, &v)| Row { ticker: "T".into(), date: format!("{:04}-01-01", 1900 + i), open: v, high: v + 1., low: v - 1., close: v, volume: None }).collect()
    }
    #[test]
    fn setup_needs_a_price_flip_and_counts_nine() {
        // rising 6 bars, then 10 falling closes: flip on the first fall, setup 9 on the ninth
        let mut c: Vec<f64> = (0..6).map(|i| 100. + i as f64).collect();
        c.extend((1..=12).map(|i| 105. - 2. * i as f64));
        let r = rows(&c);
        let x = Ctx::new(&r, &r, &[]);
        let b = Bars::new(&x, r.len() - 1, false);
        let v = setup_count(&b, 4, false);
        assert_eq!(v[6], 0); // 103 is not below 102 (close four bars earlier)
        assert_eq!(v[7], 1);
        assert_eq!(v[15], 9);
        assert_eq!(v[16], 10);
        // equal close 4 bars earlier is not a flip prerequisite
        let mut c2 = vec![100., 100., 100., 100., 100., 100.];
        c2.extend((1..=10).map(|i| 100. - i as f64));
        let r2 = rows(&c2);
        let x2 = Ctx::new(&r2, &r2, &[]);
        assert!(setup_count(&Bars::new(&x2, r2.len() - 1, false), 4, false).iter().all(|&k| k == 0));
    }
    #[test]
    fn countdown_thirteen_and_risk_level_by_hand() {
        // 6 up bars, then a steady decline of 2/bar: flip at bar 7, setup 9 at bar 15; every bar close <= low two bars earlier
        // (c - (c_2 - 1) = -4 + 1 < 0), bar 13 = 12 bars after bar 15 (countdown starts on setup bar nine).
        let mut c: Vec<f64> = (0..6).map(|i| 100. + i as f64).collect();
        c.extend((1..=40).map(|i| 105. - 2. * i as f64));
        let r = rows(&c);
        let x = Ctx::new(&r, &r, &[]);
        let b = Bars::new(&x, r.len() - 1, false);
        let o = Opts { kind: Kind::Seq, setup_n: 9, look: 4, cd_n: 13, cd_look: 2, term_qual: true, q85: false, term_open: false, mult: 1.618, recycle_n: 18, recycle_other: false, combo_eq: false };
        let s = countdowns(&b, &o);
        // the setup reaches 18 at bar 23 but it is the countdown's own setup, so no recycle
        assert_eq!(s[0].first, 15);
        assert_eq!(s[0].i, 27);
        // lowest true low = low of bar 27 = c27-1; true range = c26 - (c27 - 1) = 3
        assert!((s[0].risk - (c[27] - 1. - 3.)).abs() < 1e-9);
    }
}
