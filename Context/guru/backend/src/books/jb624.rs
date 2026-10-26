//! Jake Bernstein, "6-2-4 Winning Strategies and Systems" report (Network Press, September 2006, 164 PDF pages).
//! Book id `jb624`. Source copy: Books/Jake Bernstein/6-2-4 Winning Strategies & Systems.pdf
//! (printed page = PDF page - 1). Transcription and checks: library/jb624/.
//!
//! Conventions (bar-by-bar, no look-ahead):
//!   * a rule is evaluated on the CLOSE of bar i with data <= i; "on the close" fills at close[i]
//!   * stop and limit orders are placed after the close of bar i for bar i+1 (levels known at i)
//!   * buy stop at L: fills at open if open >= L, else at L if high >= L; sell stop mirrored
//!   * buy limit at L: fills at min(open, L) if low <= L; sell limit mirrored
//!   * indicators warm up on all bars before `from`; entries only on bars >= from
//! The helpers here (MAC, Williams A/D, stops) are shared with `jbudt`.
use crate::connors::{sma, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "jb624";

/// Rules admitted after the 2026-10-22 full reading (authors/bernstein/g1-admission/SOURCE-CONTRACT.md) and an
/// independent chronological Python ledger check. Futures methods (and stocks for the MAC / 3HL / POP charts):
/// actual stocks, indices and ETFs are a declared application. The Friday-Monday system uses S&P-futures points
/// and runs on SPX only. Held: FlatBase, 30-minute breakout, seasonal key dates, stochastic 50/45/55, PMF, COT, MOM/MA.
pub const REVIEWED: &[&str] = &[
    "jb624_mac2_sar", "jb624_mac2_stop", "jb624_mac5", "jb624_mac_adma", "jb624_mac_sr", "jb624_3hl", "jb624_pop", "jb624_gap2", "jb624_fri_mon",
];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }
/// Rules whose fills read the open (gapped stops, limit fills at the open, next-open entries, first profitable opening).
pub fn reads_open(id: &str) -> bool { matches!(id, "jb624_mac2_stop" | "jb624_mac5" | "jb624_mac_sr" | "jb624_3hl" | "jb624_gap2" | "jb624_fri_mon") }

macro_rules! def {
    ($id:expr, $name:expr, $group:expr, $dir:expr, $status:expr, $rules:expr, $notes:expr) => {
        StrategyDef { id: $id, name: $name, group: $group, session: 0, dir: $dir, status: $status, needs_volume: false,
            needs_vix: false, needs_series: &[], book: BOOK, rules: $rules, notes: $notes, claim: None }
    };
}

const MAC_DEF: &str = "F|\"Calculate an 8 period simple MA of the low price ... a 10 period simple MA of the high price for each bar (i.e. daily or weekly or intraday)\" (p.32)";
const MAC_BAR: &str = "F|\"THE PRICE BARS MUST BE COMPLETELY ABOVE OR BELOW THE MA LINE TO BE VALID ... THE ENTIRE BAR MUST BE OUTSIDE THE CHANNEL\" (p.33): above = low > MA of highs; below = high < MA of lows";
const MAC_TRIG: &str = "E|\"Two consecutive price bars above the MAH constitutes a new uptrend and a change from an existing downtrend. Two consecutive price bars below the MAL constitutes a new downtrend\" (p.39); trigger = the 2nd consecutive complete bar (p.32)";
const MAC_NOTE: &str = "Index run (instrument swap only). Entry on the close of the trigger bar [book silent on the order: the trigger exists only once the 2nd bar is complete]. A trigger is a change of MAC trend: further bars outside the channel in the same direction are not new signals.";

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::{Live, NeedsData, NotTestable};
    vec![
        def!("jb624_mac2_sar", "MAC trend change · reverse on the opposite signal", "Method #1 · Moving Average Channel", "Long/Short", Live,
            &[MAC_DEF, MAC_BAR, MAC_TRIG,
              "E|\"You can buy or sell or reverse positions accordingly\" (p.39): long on a buy trigger, short on a sell trigger",
              "X|\"The suggested stop loss trade is ... OR a reversing MAC signal\" (p.39): this variant uses the reversing signal (always in the market)"],
            MAC_NOTE),
        def!("jb624_mac2_stop", "MAC trend change · stop at twice the channel width", "Method #1 · Moving Average Channel", "Long/Short", Live,
            &[MAC_DEF, MAC_BAR, MAC_TRIG,
              "X|\"The suggested stop loss trade is twice the width of the channel as measured on the day that the trade is entered\" (p.39): stop = entry -/+ 2 x (MAH - MAL) of the entry bar, a resting stop from the next bar",
              "X|\"... OR a reversing MAC signal\" (p.39): an opposite trigger closes the trade and opens the opposite one"],
            "Variant of jb624_mac2_sar with the printed channel-width stop. After a stop-out the system waits for the next trend change. Stop fills at the stop, or at the open when the open is already beyond it."),
        def!("jb624_mac5", "MAC 5 consecutive bar pattern", "Method #2 · 5 Bar MAC Pattern", "Long/Short", Live,
            &[MAC_DEF, MAC_BAR,
              "E|\"When a new MAC buy signal occurs with 5 consecutive bars above the MAH then buy on the open of trading the 6th bar\" (p.40); sell mirror (p.40)",
              "X|\"The profit target is the range of the 5 bars. Take profit at the profit target\" (p.40): target = entry +/- (highest high - lowest low of the 5 bars), a resting limit",
              "X|\"Initial stop loss is in the range of the 5 bars or a reversing signal\" (p.41): default = the reversing MAC signal (exit on its close); p.stop_range=1 adds a stop at entry -/+ the 5-bar range [reading of \"in the range\", non-default]"],
            "\"new MAC buy signal\": the 5-bar run must contain the 2-bar trigger that changed the MAC trend (p.new_only=1); p.new_only=0 accepts any fresh 5-bar run. Partial-profit/trailing alternatives (p.40) are not coded."),
        def!("jb624_mac_adma", "MAC confirmed by Williams A/D and its 28-period MA", "Method #1 · MAC with AD/MA", "Long/Short", Live,
            &[MAC_DEF, MAC_BAR, "E|\"Use the basic 2 bar MAC buy and sell signals with the AD/MA\" (p.43)",
              "F|\"Use the Williams Accumulation-Distribution Indicator (note that you will NOT be using the volume based Accumulation-Distribution indicator) and a 28 period moving average of the AD\" (p.43)",
              "F|\"When AD moves above its MA for 2 consecutive postings a buy is triggered. When AD moves below its MA for 2 consecutive postings a sell is triggered\" (p.43-44)",
              "E|\"Confirm the MAC signals with the AD/MA signals and vice versa\" (p.44): long on the close of the bar on which both are on a buy, short when both are on a sell",
              "X|\"The stop loss is the reverse of the current signal\" (p.44): exit on the close when either signal reverses; reverse when both now agree the other way"],
            "Williams A/D (MetaStock definition): close > prior close adds close - min(low, prior close); close < prior close adds close - max(high, prior close); unchanged adds 0; cumulative. Simple 28-bar MA. How the two signals combine is read from \"confirm ... and vice versa\" (in the market only while both agree)."),
        def!("jb624_mac_sr", "MAC support/resistance swing trade", "Method #1 · MAC swing trading", "Long/Short", Live,
            &[MAC_DEF, MAC_BAR, MAC_TRIG,
              "E|\"If the MAC trend is up, then buy at the MAL ... If the MAC trend is down, then sell at the MAH\" (p.37): a resting limit at yesterday's MA of lows (highs)",
              "X|\"... and take profit at the MAH\" / \"take profit at the MAL\" (p.37): a resting limit at yesterday's MA of highs (lows), from the bar after entry",
              "X|No stop is printed for this application; p.exit_on_flip=1 (non-default) also exits on the close of an opposite MAC trigger"],
            "MA levels are yesterday's values (today's MA includes today's high/low, unknown when the order is placed). The trailing-stop alternative after the target (p.37) is not coded."),
        def!("jb624_3hl", "Three High Low Moving Average Channel", "Method #3 · Three High Low MAC", "Long/Short", Live,
            &["F|\"Use a three-day simple moving average of highs and a three-day simple moving average of lows\" (p.49)",
              "F|\"The trend is determined by running a nine and 18 period MACD of closing prices and an eight bar moving average\" of it; \"When the MACD crosses above its moving average the trend is up ... below ... down\" (p.49)",
              "E|\"When the trend is up buy the three period moving average of the low\" (p.49): resting limit at yesterday's 3-bar MA of lows",
              "X|\"... and exit at the three period moving average of the high\" (p.49): resting limit at yesterday's 3-bar MA of highs, from the bar after entry",
              "E|\"When the trend is down sell the three period moving average of the high and exit at the three period moving average of the low\" (p.50)",
              "X|No stop is printed; p.exit_on_flip=1 (non-default) also exits on the close when the MACD trend flips"],
            "MACD = EMA(9) - EMA(18) of closes; its 8-bar average is exponential, as the author defines the MACD signal line (\"An exponential moving average of this difference is then computed\", The Ultimate Day Trader ch.9). EMAs seeded with the first close. Author: \"not totally mechanical ... how much risk you want to take\" (p.47); risk level is not printed, so no stop by default."),
        def!("jb624_pop", "Stochastic POP (14, 5-bar smoothing, 75/25)", "Method #5 · Stochastic POP", "Long/Short", Live,
            &["F|\"Use a 14 period slow stochastic 5 bar smoothing %K. Do not use %D\" (p.76): slow %K = 100 x sum over 5 bars of (C - lowest low 14) / sum over 5 bars of (highest high 14 - lowest low 14) (Lane's original slowing, TradeStation's default; p.k_method=2 averages fast %K instead)",
              "E|\"When stochastic K goes above 75, BUY\" (p.76): on the close of the bar on which %K crosses above 75",
              "X|\"Exit on tailing stop or when stochastic K goes below 75\" (p.76): sell on the close when %K < 75",
              "E|\"When stochastic K goes below 25, sell short\" (p.76)",
              "X|\"Exit with a trailing stop when K goes back above 25\" (p.76): cover on the close when %K > 25"],
            "Signals at the end of the bar (\"If at the end of the time bar you are using ...\", p.73). The trailing-stop alternative has no printed level and is not coded. The 50/45/55 variant (p.76) is listed separately as not testable."),
        def!("jb624_gap2", "2-Day Breakaway Gap", "Method #7 · 2 Day Breakaway Gap", "Long/Short", Live,
            &["F|\"A GAP UP occurs when a market opens above the high of the previous day and does not trade into the range of the previous day\" (p.79): low > prior high",
              "E|\"Setup for a BUY is a one day gap up. Trigger: if the low of day two and the low of day three are higher than the high of day one, then a two day gap up has triggered on the close of day three\" (p.80)",
              "E|\"Entry for a long position on the 2 day gap up is on the close of trading on the 2nd day\" (p.81): the 2nd day of the two-day gap = day three, the trigger close",
              "X|\"The profit target is the RANGE of the 2 days that constitute the gap at which point you take profit on part of your position. Then you trail a stop at 75% on the balance of your position (i.e. locking in 75% of the profit)\" (p.81)",
              "X|\"The initial stop loss is a CLOSE ABOVE the highest high of the two days of a sell gap or a CLOSE BELOW the lowest low of the two days of the buy gap\" (p.81)",
              "X|\"Do not add to your position in the event of another signal in the same direction\" (p.81)"],
            "Book silent: the size of the \"part\" taken at the target (p.part, default 1/2) and how the 75% is measured (here 75% of the best open profit, highest high/lowest low since entry, a resting stop from the bar after the target fill). Target = entry +/- (highest high - lowest low of days 2-3)."),
        StrategyDef { claim: crate::connors::claim("198", "77.8%", "$716 (1 S&P contract)", "1.51 bars", "S&P futures 1999-2006, PF 1.66"),
          ..def!("jb624_fri_mon", "Friday-Monday S&P system", "Trading System I · Friday-Monday", "Long/Short", Live,
            &["F|\"If on Friday or the last trading day of the week the closing price is higher than the opening price then there is a bullish setup ... lower than the opening price, then there is a bearish setup\" (p.126)",
              "E|\"the high exceeds the Friday high by X number of points then a buy is triggered ... drops below the low of Friday X number of points then a sell is triggered\" (p.126); inputs (p.130): DOW Buy HowManyTicksAbove 54, DOW Sell HowManyTicksBelow 1 = 5.40 and 0.10 S&P points (0.10-point tick)",
              "X|\"Follow-through is either a specific dollar stop loss or an exit on the first profitable opening\" (p.127); inputs (p.130): Stop Loss Amount long $5,000, short $2,550 = 20.0 and 10.2 points at $250 per point",
              "X|\"The first profitable opening is defined as any opening price after a trigger that is profitable relative to the price at which the trade triggered\" (p.127)"],
            "Book: full S&P 500 futures day session, Genesis (p.125-131). Points are kept as printed on every index (instrument swap only). Week boundary: the next bar falls in a later calendar week. Entry bar: the stop counts only if the bar path (open, nearer extreme, other extreme, close: TradeStation's default assumption) reaches it after the entry; p.entry_day=1 counts any touch. One position at a time.") },
        def!("jb624_flatbase", "FlatBase Breakout (Granville)", "Market structure", "Long", NotTestable("discretionary: the \"sideways pattern\" (at least 40 days) is not defined"),
            &["F|\"The FBO is characterized by a lengthy period of sideways movement ... On a daily price chart I like to see at least 40 days in the sideways pattern\" (p.26-27)",
              "E|\"The breakout or buy signal would occur if there were two consecutive closing prices above the highest high or the lowest low during the FlatBase development period\" (p.27)"],
            "Author: \"given the somewhat loose definition that I have provided above\" (p.27). What counts as sideways is chart judgement."),
        def!("jb624_or30", "30-Minute Breakout Method (S&P)", "Method #4 · 30-Minute Breakout", "Long/Short", NeedsData("intraday 30-minute bars (first 30-minute high/low and every later 30-minute close)"),
            &["F|\"Use the 30 minute S&P futures chart day session only\"; note \"the high and the low of the first 30 minutes\" (p.51)",
              "E|\"The high of the first 30 minutes, if exceeded at the end of any 30-Minute bar, thereafter triggers a buy signal\"; low mirrored (p.51-52)",
              "X|\"The stop loss is the opposite signal ... the trade must be reversed\"; \"The initial profit target is the range of the first half-hour bar\"; break-even and trailing stops; \"Exit all positions by the end of the day\" (p.52)",
              "F|\"If a buy or sell trade is not triggered by the last hour of trading then there will be no trade for the rest of the day\" (p.55)"],
            "Triggers are 30-minute closes in sequence through the day; daily OHLC cannot order them."),
        def!("jb624_seasonal", "Seasonal Key Date trades with stochastic trigger", "Method #5 · Seasonal Key Dates", "Long/Short", NeedsData("individual futures contract histories (e.g. July lean hogs, September heating oil, September soybeans, September Canadian dollar) and the HOST key-date database"),
            &["F|Key Date Seasonals give \"The commodity and contract month to trade ... The stop loss in % of enter price (close only stop)\" (p.58)",
              "F|Filters: accuracy >= 75%, P/L ratio >= 4, <= 2 consecutive losses, average profit >= $400, positive growth; keep setups passing 4 of 5 (p.64-65)",
              "E|\"Use a 14 period slow stochastic %K with 5 bar smoothing ... A BUY TRIGGER occurs when SI %K has been below 25% and has gone back above it. A SELL Trigger occurs when SI %K has been above 75% and has gone back below it\" (p.66-67)",
              "E|Enter MOC on the entry date if triggered (or next open); allow 15% more time, rounded up, in trading days (p.68)",
              "X|Exit on the key exit date; close-only % stop from the seasonal listing (p.58, 61)"],
            "Examples printed: Long Jul Lean Hogs 7/2-7/6 stop 1%, 35 trades 77.14% (p.60); Long Sep Heating Oil 7/25-8/13 stop 4%, 25 trades 80% (p.61); Short Sep Soybeans 7/14-7/30 stop 5%, 36 trades 77.78% (p.62); Short Sep Canada $ 7/17-7/24 stop 1%, 28 trades 82.14% (p.63-64). No stock-index key date is printed."),
        def!("jb624_stoch50", "Stochastic 50/45/55 variant", "Method #5 · Stochastic POP", "Long/Short", NotTestable("stochastic length and the %D line it reverses on are not given; author: \"not thoroughly tested as yet\""),
            &["E|\"When the stochastic reads above 50%, and then falls below 45%, sell short. Your stop loss is a crossing of the stochastic lines or a move back over the 50% line, which would cause you to reverse to a long position ... When SI has been below 50% and the raise above 55%, you buy\" (p.76)"],
            "The reverse level (back over 50) contradicts the buy level (above 55)."),
        def!("jb624_pmf", "Power Momentum Formula (momentum divergence)", "Power Momentum Formula", "Long/Short", NotTestable("the setup search leaves the MOM-high window, the search range for point C and how long a setup stays valid undefined, and the single-unit exit refers to a trailing-stop procedure printed only for 2- and 3-unit positions"),
            &["F|\"Look at the last 60 price bars; Find the highest intraday price high; Look at the 28 MOM; Was the highest intraday high accompanied by a new high in MOM +/- 5 days on either side of the date of the price high?\" (p.106)",
              "F|\"A= the new intraday price high, B= the MOM for the same date as A, C= the highest MOM previous to B, D= the intraday price high for the same date as point C\" (p.106-107); at least six bars from A to D (p.105)",
              "E|\"find the LOWEST MOM between points B and C inclusive ... E. If MOM penetrates E at the end of any bar then you have a SELL trigger\" (p.107); buy mirrored (p.108-109)",
              "X|PT1 = 50% of the A-D high-low range from the trigger price, PT2 = the full range (p.111-112); stop: \"If MOM closes below point C then you exit the trade\" (p.121)"],
            "Written up in full in library/jb624/README.md. The same method is in the Power Momentum Formula manuals (2004) and The Ultimate Day Trader ch.9-11, where the author says divergence \"is not a trading system ... it does require some degree of judgment\"."),
        def!("jb624_cot", "COT Commercials with weekly MAC / ADMA / dual MA triggers", "COT Report", "Long/Short", NeedsData("CFTC Commitments of Traders commercial net positions and weekly futures continuation charts; the author also excludes stock indices (\"works best on all markets other than the financials\", p.91)"),
            &["F|\"When the COT Commercials net positions rises above the zero level begin monitoring the weekly MAC\"; \"When the MAC gives a buy signal on the weekly chart you can be a buyer\" (p.92)",
              "E|Dual MA: \"Plot a 19 period simple moving average of the Commercials COT. Plot a 9 period simple moving average of the 19 period moving average ... When the shorter MA drops below the longer MA a buy is triggered\" (p.95-96)"],
            "Also judgement: \"you can also attempt to buy if and when prices decline to MAC support\" (p.92)."),
        def!("jb624_mom_ma", "MOM/MA system (Australian dollar futures)", "Trading System II · Momentum/Moving Average", "Long/Short", NeedsData("Australian dollar futures; the Genesis exit inputs (Profit Floor, Profit Risk Percent) are not defined in the text"),
            &["E|\"In its simplest form we use a 28 period momentum and 28 period moving average of the momentum\" (p.132)",
              "F|Australian dollar inputs (p.137): long MomLength 39, MAvgLength 39; short MomLength 65, MAvgLength 77; stop loss $2,350 / $1,650; profit floor $1,000 / $1,750 with profit risk percent 72 / 78",
              "X|\"a dollar risk management stop, an initial profit target, and a trailing stop\" (p.135)"],
            "Printed A$ result (p.136): 157 trades, 67.5%, $92,635, PF 2.63. The generic 28/28 version has no printed exit."),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "jb624_mac2_sar" => ("jb624_mac", "Bernstein Moving Average Channel (6-2-4)", "2-bar trend change · reverse"),
        "jb624_mac2_stop" => ("jb624_mac", "Bernstein Moving Average Channel (6-2-4)", "2-bar trend change · 2x channel stop"),
        "jb624_mac5" => ("jb624_mac", "Bernstein Moving Average Channel (6-2-4)", "5 consecutive bars"),
        "jb624_mac_adma" => ("jb624_mac", "Bernstein Moving Average Channel (6-2-4)", "with Williams AD/MA 28"),
        "jb624_mac_sr" => ("jb624_mac", "Bernstein Moving Average Channel (6-2-4)", "support/resistance swing"),
        "jb624_3hl" => ("jb624_3hl", "Three High Low Moving Average Channel", "MACD 9/18/8 trend"),
        "jb624_pop" => ("jb624_stoch", "Bernstein stochastic methods (6-2-4)", "POP 14/5, 75/25"),
        "jb624_stoch50" => ("jb624_stoch", "Bernstein stochastic methods (6-2-4)", "50/45/55 · not testable"),
        "jb624_gap2" => ("jb624_gap2", "2-Day Breakaway Gap", "Target + 75% trail"),
        "jb624_fri_mon" => ("jb624_fri_mon", "Friday-Monday S&P system", "Genesis inputs (p.130)"),
        "jb624_flatbase" => ("jb624_flatbase", "FlatBase Breakout", "Not testable"),
        "jb624_or30" => ("jb624_or30", "30-Minute Breakout", "Needs intraday bars"),
        "jb624_seasonal" => ("jb624_seasonal", "Seasonal Key Dates", "Needs futures contracts"),
        "jb624_pmf" => ("jb624_pmf", "Power Momentum Formula", "Not testable"),
        "jb624_cot" => ("jb624_cot", "COT Commercials triggers", "Needs COT data"),
        "jb624_mom_ma" => ("jb624_mom_ma", "MOM/MA system", "Needs A$ futures"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let mac = || vec![p("mah", "MA of highs (bars)", 10., 5., 20., 1.), p("mal", "MA of lows (bars)", 8., 4., 20., 1.), p("bars", "Consecutive complete bars for a trigger", 2., 1., 4., 1.)];
    Some(match id {
        "jb624_mac2_sar" => mac(),
        "jb624_mac2_stop" => { let mut v = mac(); v.push(p("stop_w", "Stop = this many channel widths", 2., 0.5, 4., 0.5)); v }
        "jb624_mac5" => vec![p("mah", "MA of highs (bars)", 10., 5., 20., 1.), p("mal", "MA of lows (bars)", 8., 4., 20., 1.), p("bars", "Consecutive complete bars for a trigger", 2., 1., 4., 1.),
            p("run", "Consecutive bars outside the channel", 5., 3., 8., 1.), p("new_only", "Run must start a new MAC signal (1 = book)", 1., 0., 1., 1.),
            p("stop_range", "Add a stop at entry -/+ the run's range (0 = book default)", 0., 0., 1., 1.)],
        "jb624_mac_adma" => { let mut v = mac(); v.push(p("ad_ma", "MA of Williams A/D (bars)", 28., 10., 60., 1.)); v.push(p("ad_post", "Consecutive postings above/below the MA", 2., 1., 4., 1.)); v }
        "jb624_mac_sr" => { let mut v = mac(); v.push(p("exit_on_flip", "Also exit on an opposite MAC trigger (0 = book)", 0., 0., 1., 1.)); v }
        "jb624_3hl" => vec![p("hl", "MA of highs/lows (bars)", 3., 2., 10., 1.), p("fast", "MACD fast EMA", 9., 5., 15., 1.), p("slow", "MACD slow EMA", 18., 12., 30., 1.),
            p("sig", "MACD average (bars)", 8., 3., 15., 1.), p("exit_on_flip", "Also exit when the trend flips (0 = book)", 0., 0., 1., 1.)],
        "jb624_pop" => vec![p("k", "Stochastic length", 14., 5., 30., 1.), p("smooth", "%K smoothing (bars)", 5., 1., 10., 1.), p("upper", "Buy above", 75., 55., 90., 5.), p("lower", "Sell below", 25., 10., 45., 5.),
            p("k_method", "Slowing: 1 = Lane original (TradeStation default), 2 = average of fast %K", 1., 1., 2., 1.)],
        "jb624_gap2" => vec![p("part", "Fraction taken at the target (book: \"part\")", 0.5, 0.25, 1., 0.25), p("trail", "Trailing stop locks this % of the open profit", 75., 50., 90., 5.)],
        "jb624_fri_mon" => vec![p("buy_off", "Buy stop above Friday's high (points)", 5.4, 0., 10., 0.1), p("sell_off", "Sell stop below Friday's low (points)", 0.1, 0., 10., 0.1),
            p("stop_long", "Long stop loss (points; $5,000 / $250)", 20., 5., 40., 0.5), p("stop_short", "Short stop loss (points; $2,550 / $250)", 10.2, 5., 40., 0.2),
            p("entry_day", "Entry-day stop: 0 = bar path O-near-far-C, 1 = any touch", 0., 0., 1., 1.)],
        _ => return None,
    })
}

// ---------------------------------------------------------------- shared helpers (also used by jbudt)

pub fn getp(id: &str, p: &Params, key: &str) -> f64 {
    p.get(key).copied().unwrap_or_else(|| {
        crate::books::params(id).and_then(|s| s.iter().find(|d| d.key == key).map(|d| d.def)).unwrap_or_else(|| panic!("{id} has no param {key}"))
    })
}

pub fn new_trade(dir: f64, i: usize, px: f64, kind: &'static str, sig: usize) -> Trade {
    Trade { dir, legs: vec![Leg { i, px, w: 1.0 }], exit_i: i, exit_px: px, reason: String::new(), entry_kind: kind, signal_i: sig, open: false, exits: vec![] }
}

pub fn finish(mut t: Trade, i: usize, px: f64, sig: usize, why: &str) -> Trade {
    t.exit_i = i;
    t.exit_px = px;
    t.signal_i = sig;
    t.reason = why.into();
    t
}

pub fn mark_open(out: &mut Vec<Trade>, cur: Option<Trade>, x: &Ctx, to: usize) {
    if let Some(mut t) = cur {
        t.exit_i = to;
        if !t.exits.is_empty() {
            let done: f64 = t.exits.iter().map(|e| e.2).sum();
            t.exits.push((to, x.c[to], 1.0 - done));
        }
        t.exit_px = if t.exits.is_empty() { x.c[to] } else { t.exits.iter().map(|e| e.1 * e.2).sum() };
        t.signal_i = to;
        t.reason = "open".into();
        t.open = true;
        out.push(t);
    }
}

/// Stop entry/exit in direction `dir` (+1 = buy stop, -1 = sell stop) at level `lvl` on bar i.
pub fn stop_fill(x: &Ctx, i: usize, dir: f64, lvl: f64) -> Option<f64> {
    if !lvl.is_finite() {
        return None;
    }
    let eps = 1e-9 * lvl.abs(); // a touch within float noise of a computed level counts
    if dir > 0.0 {
        if x.o[i] >= lvl { Some(x.o[i]) } else if x.h[i] >= lvl - eps { Some(lvl.min(x.h[i])) } else { None }
    } else if x.o[i] <= lvl { Some(x.o[i]) } else if x.l[i] <= lvl + eps { Some(lvl.max(x.l[i])) } else { None }
}

/// Limit order (+1 = buy limit, -1 = sell limit) at `lvl` on bar i.
pub fn limit_fill(x: &Ctx, i: usize, dir: f64, lvl: f64) -> Option<f64> {
    if !lvl.is_finite() {
        return None;
    }
    let eps = 1e-9 * lvl.abs();
    if dir > 0.0 {
        (x.l[i] <= lvl + eps).then(|| x.o[i].min(lvl).max(x.l[i]))
    } else {
        (x.h[i] >= lvl - eps).then(|| x.o[i].max(lvl).min(x.h[i]))
    }
}

pub fn ema(v: &[f64], n: usize) -> Vec<f64> {
    let a = 2.0 / (n as f64 + 1.0);
    let mut out = vec![f64::NAN; v.len()];
    let mut prev = f64::NAN;
    for i in 0..v.len() {
        if v[i].is_nan() {
            continue;
        }
        prev = if prev.is_nan() { v[i] } else { prev + a * (v[i] - prev) };
        out[i] = prev;
    }
    out
}

/// Larry Williams' Accumulation/Distribution (price only, cumulative).
pub fn williams_ad(x: &Ctx) -> Vec<f64> {
    let mut out = vec![0.0; x.c.len()];
    for i in 1..x.c.len() {
        let pc = x.c[i - 1];
        let d = if x.c[i] > pc { x.c[i] - x.l[i].min(pc) } else if x.c[i] < pc { x.c[i] - x.h[i].max(pc) } else { 0.0 };
        out[i] = out[i - 1] + d;
    }
    out
}

pub struct Mac {
    pub mah: Vec<f64>,
    pub mal: Vec<f64>,
    /// +1 / -1 on the bar a trend-change trigger completes, else 0
    pub trig: Vec<i8>,
    /// MAC trend after bar i's close (0 before the first trigger)
    pub trend: Vec<i8>,
    pub up_run: Vec<usize>,
    pub dn_run: Vec<usize>,
}

pub fn mac(x: &Ctx, mah_n: usize, mal_n: usize, bars: usize) -> Mac {
    let n = x.c.len();
    let mah = sma(&x.h, mah_n);
    let mal = sma(&x.l, mal_n);
    let (mut up_run, mut dn_run) = (vec![0usize; n], vec![0usize; n]);
    let (mut trig, mut trend) = (vec![0i8; n], vec![0i8; n]);
    let mut tr = 0i8;
    for i in 0..n {
        // strictly outside the channel; an exact tie (within float noise of the running-sum MA) touches the line
        let above = x.l[i] - mah[i] > 1e-9 * mah[i].abs();
        let below = mal[i] - x.h[i] > 1e-9 * mal[i].abs();
        up_run[i] = if above { if i > 0 { up_run[i - 1] + 1 } else { 1 } } else { 0 };
        dn_run[i] = if below { if i > 0 { dn_run[i - 1] + 1 } else { 1 } } else { 0 };
        if up_run[i] >= bars && tr != 1 {
            tr = 1;
            trig[i] = 1;
        } else if dn_run[i] >= bars && tr != -1 {
            tr = -1;
            trig[i] = -1;
        }
        trend[i] = tr;
    }
    Mac { mah, mal, trig, trend, up_run, dn_run }
}

/// Slow %K of length n with `smooth`-bar slowing. method 1 = George Lane's original (TradeStation's default
/// SmoothingType 1): 100 x sum(C - LL) / sum(HH - LL) over the slowing bars; method 2 = TradeStation "legacy":
/// simple average of fast %K.
pub fn slow_k(x: &Ctx, n: usize, smooth: usize, method: u8) -> Vec<f64> {
    let len = x.c.len();
    let (mut num, mut den, mut fast) = (vec![f64::NAN; len], vec![f64::NAN; len], vec![f64::NAN; len]);
    for i in 0..len {
        if i + 1 < n {
            continue;
        }
        let hh = x.h[i + 1 - n..=i].iter().cloned().fold(f64::MIN, f64::max);
        let ll = x.l[i + 1 - n..=i].iter().cloned().fold(f64::MAX, f64::min);
        num[i] = x.c[i] - ll;
        den[i] = hh - ll;
        fast[i] = if hh > ll { 100.0 * (x.c[i] - ll) / (hh - ll) } else { 50.0 };
    }
    if method == 2 {
        return sma(&fast, smooth);
    }
    let (sn, sd) = (sma(&num, smooth), sma(&den, smooth));
    sn.iter().zip(&sd).map(|(a, b)| if b.is_nan() { f64::NAN } else if *b > 0.0 { 100.0 * a / b } else { 50.0 }).collect()
}

// ---------------------------------------------------------------- run

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !id.starts_with("jb624_") || params(id).is_none() || to >= x.c.len() || from > to {
        return None;
    }
    let g = |k: &str| getp(id, p, k);
    let u = |k: &str| g(k).round().max(1.0) as usize;
    let mut out = Vec::new();
    let mut cur: Option<Trade> = None;
    match id {
        "jb624_mac2_sar" | "jb624_mac2_stop" | "jb624_mac_adma" => {
            let m = mac(x, u("mah"), u("mal"), u("bars"));
            let with_stop = id == "jb624_mac2_stop";
            let stop_w = if with_stop { g("stop_w") } else { 0.0 };
            // desired side per bar
            let want: Vec<i8> = if id == "jb624_mac_adma" {
                let ad = williams_ad(x);
                let adma = sma(&ad, u("ad_ma"));
                let post = u("ad_post");
                let (mut st, mut ab, mut be) = (0i8, 0usize, 0usize);
                (0..x.c.len())
                    .map(|i| {
                        if adma[i].is_finite() {
                            ab = if ad[i] > adma[i] { ab + 1 } else { 0 };
                            be = if ad[i] < adma[i] { be + 1 } else { 0 };
                            if ab >= post { st = 1 } else if be >= post { st = -1 }
                        }
                        if st != 0 && st == m.trend[i] { st } else { 0 }
                    })
                    .collect()
            } else {
                m.trig.clone()
            };
            let mut stop = f64::NAN;
            for i in from..=to {
                // resting stop (mac2_stop)
                if let Some(t) = cur.as_ref() {
                    if with_stop && t.legs[0].i < i {
                        if let Some(px) = stop_fill(x, i, -t.dir, stop) {
                            out.push(finish(cur.take().unwrap(), i, px, i, "Stop: 2 x channel width"));
                        }
                    }
                }
                let w = want[i];
                if id == "jb624_mac_adma" {
                    let side = cur.as_ref().map_or(0, |t| t.dir as i8);
                    if side != 0 && w != side {
                        out.push(finish(cur.take().unwrap(), i, x.c[i], i, "MAC and AD/MA no longer agree"));
                    }
                    if cur.is_none() && w != 0 && i < to && (i == from || want[i - 1] != w) {
                        cur = Some(new_trade(w as f64, i, x.c[i], "close", i));
                    }
                    continue;
                }
                if w == 0 {
                    continue;
                }
                if let Some(t) = cur.as_ref() {
                    if t.dir as i8 == w {
                        continue;
                    }
                    out.push(finish(cur.take().unwrap(), i, x.c[i], i, "Reversing MAC signal"));
                }
                if i < to {
                    cur = Some(new_trade(w as f64, i, x.c[i], "close", i));
                    stop = x.c[i] - w as f64 * stop_w * (m.mah[i] - m.mal[i]);
                }
            }
        }
        "jb624_mac5" => {
            let m = mac(x, u("mah"), u("mal"), u("bars"));
            let (run_n, new_only, stop_range) = (u("run"), g("new_only") >= 0.5, g("stop_range") >= 0.5);
            let bars = u("bars");
            let (mut tgt, mut stp) = (f64::NAN, f64::NAN);
            let mut i = from;
            while i <= to {
                if let Some(t) = cur.as_ref() {
                    let d = t.dir;
                    // stop first (conservative), then target, then reversing signal on the close
                    if stop_range {
                        if let Some(px) = stop_fill(x, i, -d, stp) {
                            out.push(finish(cur.take().unwrap(), i, px, i, "Stop: range of the 5 bars"));
                            i += 1;
                            continue;
                        }
                    }
                    if let Some(px) = limit_fill(x, i, -d, tgt) {
                        out.push(finish(cur.take().unwrap(), i, px, i, "Profit target: range of the 5 bars"));
                        i += 1;
                        continue;
                    }
                    if m.trig[i] as f64 == -d {
                        out.push(finish(cur.take().unwrap(), i, x.c[i], i, "Reversing MAC signal"));
                    }
                }
                if cur.is_none() && i < to {
                    for d in [1i8, -1] {
                        let runv = if d > 0 { &m.up_run } else { &m.dn_run };
                        if runv[i] != run_n {
                            continue;
                        }
                        let start = i + 1 - run_n;
                        // the bar on which the run produced its 2-bar trigger
                        let ok_new = !new_only || (start + bars - 1 <= i && (start..=i).any(|k| m.trig[k] == d));
                        if !ok_new {
                            continue;
                        }
                        let hh = x.h[start..=i].iter().cloned().fold(f64::MIN, f64::max);
                        let ll = x.l[start..=i].iter().cloned().fold(f64::MAX, f64::min);
                        let e = x.o[i + 1];
                        let mut t = new_trade(d as f64, i + 1, e, "open", i);
                        t.signal_i = i;
                        tgt = e + d as f64 * (hh - ll);
                        stp = e - d as f64 * (hh - ll);
                        cur = Some(t);
                        break;
                    }
                }
                i += 1;
            }
        }
        "jb624_mac_sr" | "jb624_3hl" => {
            let (hi, lo, trend): (Vec<f64>, Vec<f64>, Vec<i8>) = if id == "jb624_mac_sr" {
                let m = mac(x, u("mah"), u("mal"), u("bars"));
                (m.mah, m.mal, m.trend)
            } else {
                let n = u("hl");
                let macd: Vec<f64> = ema(&x.c, u("fast")).iter().zip(ema(&x.c, u("slow"))).map(|(a, b)| a - b).collect();
                let sig = ema(&macd, u("sig"));
                let mut st = 0i8;
                let tr = (0..x.c.len())
                    .map(|i| {
                        if i >= u("slow") {
                            if macd[i] > sig[i] { st = 1 } else if macd[i] < sig[i] { st = -1 }
                        }
                        st
                    })
                    .collect();
                (sma(&x.h, n), sma(&x.l, n), tr)
            };
            let flip = g("exit_on_flip") >= 0.5;
            for i in from.max(1)..=to {
                if let Some(t) = cur.as_ref() {
                    let d = t.dir;
                    if t.legs[0].i < i {
                        let lvl = if d > 0.0 { hi[i - 1] } else { lo[i - 1] };
                        if let Some(px) = limit_fill(x, i, -d, lvl) {
                            out.push(finish(cur.take().unwrap(), i, px, i, if d > 0.0 { "Take profit at the MA of highs" } else { "Take profit at the MA of lows" }));
                            continue;
                        }
                    }
                    if flip && trend[i] as f64 == -d {
                        out.push(finish(cur.take().unwrap(), i, x.c[i], i, "Trend flipped (non-default)"));
                    }
                    continue;
                }
                let d = trend[i - 1];
                if d == 0 {
                    continue;
                }
                let lvl = if d > 0 { lo[i - 1] } else { hi[i - 1] };
                if let Some(px) = limit_fill(x, i, d as f64, lvl) {
                    let mut t = new_trade(d as f64, i, px, "limit", i - 1);
                    t.signal_i = i - 1;
                    cur = Some(t);
                    if flip && trend[i] == -d {
                        out.push(finish(cur.take().unwrap(), i, x.c[i], i, "Trend flipped (non-default)"));
                    }
                }
            }
        }
        "jb624_pop" => {
            let k = slow_k(x, u("k"), u("smooth"), if g("k_method") >= 1.5 { 2 } else { 1 });
            let (up, lo) = (g("upper"), g("lower"));
            // a reading within float noise of a level is on the level (neither above nor below it)
            let gt = |a: f64, b: f64| a - b > 1e-9 * b.abs();
            for i in from.max(1)..=to {
                if let Some(t) = cur.as_ref() {
                    let long = t.dir > 0.0;
                    if (long && gt(up, k[i])) || (!long && gt(k[i], lo)) {
                        out.push(finish(cur.take().unwrap(), i, x.c[i], i, if long { "%K back below the upper level" } else { "%K back above the lower level" }));
                    }
                    continue;
                }
                if i == to {
                    break;
                }
                if gt(k[i], up) && !gt(k[i - 1], up) && k[i - 1].is_finite() {
                    cur = Some(new_trade(1.0, i, x.c[i], "close", i));
                } else if gt(lo, k[i]) && !gt(lo, k[i - 1]) && k[i - 1].is_finite() {
                    cur = Some(new_trade(-1.0, i, x.c[i], "close", i));
                }
            }
        }
        "jb624_gap2" => {
            let (part, trail) = (g("part").clamp(0.0, 1.0), g("trail") / 100.0);
            let (mut tgt, mut cstop, mut peak, mut hit_i) = (f64::NAN, f64::NAN, f64::NAN, usize::MAX);
            for i in from.max(2)..=to {
                if let Some(t) = cur.as_mut() {
                    let d = t.dir;
                    let e = t.legs[0].px;
                    if hit_i == usize::MAX {
                        if let Some(px) = limit_fill(x, i, -d, tgt) {
                            hit_i = i;
                            peak = if d > 0.0 { x.h[i] } else { x.l[i] };
                            if part >= 1.0 {
                                let t = cur.take().unwrap();
                                out.push(finish(t, i, px, i, "Profit target: range of the 2 gap days"));
                                continue;
                            }
                            t.exits.push((i, px, part));
                            if (d > 0.0 && x.c[i] < cstop) || (d < 0.0 && x.c[i] > cstop) {
                                let mut t = cur.take().unwrap();
                                t.exits.push((i, x.c[i], 1.0 - part));
                                let avg = t.exits.iter().map(|e| e.1 * e.2).sum::<f64>();
                                out.push(finish(t, i, avg, i, "Close beyond the 2 gap days (initial stop) on the balance"));
                                continue;
                            }
                        } else if (d > 0.0 && x.c[i] < cstop) || (d < 0.0 && x.c[i] > cstop) {
                            out.push(finish(cur.take().unwrap(), i, x.c[i], i, "Close beyond the 2 gap days (initial stop)"));
                            continue;
                        }
                    } else if hit_i < i {
                        let lvl = e + trail * (peak - e);
                        let fill = stop_fill(x, i, -d, lvl);
                        if let Some(px) = fill {
                            let mut t = cur.take().unwrap();
                            t.exits.push((i, px, 1.0 - part));
                            let avg = t.exits.iter().map(|e| e.1 * e.2).sum::<f64>();
                            out.push(finish(t, i, avg, i, "Trailing stop (75% of open profit) on the balance"));
                            continue;
                        }
                        if (d > 0.0 && x.c[i] < cstop) || (d < 0.0 && x.c[i] > cstop) {
                            let mut t = cur.take().unwrap();
                            t.exits.push((i, x.c[i], 1.0 - part));
                            let avg = t.exits.iter().map(|e| e.1 * e.2).sum::<f64>();
                            out.push(finish(t, i, avg, i, "Close beyond the 2 gap days (initial stop) on the balance"));
                            continue;
                        }
                        peak = if d > 0.0 { peak.max(x.h[i]) } else { peak.min(x.l[i]) };
                    }
                    continue;
                }
                if i == to {
                    break;
                }
                let (d1, d2, d3) = (i - 2, i - 1, i);
                let d = if x.l[d2] > x.h[d1] && x.l[d3] > x.h[d1] { 1.0 } else if x.h[d2] < x.l[d1] && x.h[d3] < x.l[d1] { -1.0 } else { 0.0 };
                if d != 0.0 {
                    let hh = x.h[d2].max(x.h[d3]);
                    let ll = x.l[d2].min(x.l[d3]);
                    tgt = x.c[i] + d * (hh - ll);
                    cstop = if d > 0.0 { ll } else { hh };
                    hit_i = usize::MAX;
                    cur = Some(new_trade(d, i, x.c[i], "close", i));
                }
            }
        }
        "jb624_fri_mon" => {
            let (bo, so, sl, ss) = (g("buy_off"), g("sell_off"), g("stop_long"), g("stop_short"));
            let mut stop = f64::NAN;
            for i in from..=to {
                if let Some(t) = cur.as_ref().filter(|t| t.legs[0].i < i) {
                    let (d, e) = (t.dir, t.legs[0].px);
                    if (x.o[i] - e) * d > 0.0 {
                        out.push(finish(cur.take().unwrap(), i, x.o[i], i, "First profitable opening"));
                    } else if let Some(px) = stop_fill(x, i, -d, stop) {
                        out.push(finish(cur.take().unwrap(), i, px, i, "Stop loss"));
                    }
                }
                if cur.is_none() && i < to && i + 1 <= to && week_changes(x, i) {
                    let j = i + 1;
                    let d = if x.c[i] > x.o[i] { 1.0 } else if x.c[i] < x.o[i] { -1.0 } else { 0.0 };
                    if d == 0.0 {
                        continue;
                    }
                    let lvl = if d > 0.0 { x.h[i] + bo } else { x.l[i] - so };
                    if let Some(px) = stop_fill(x, j, d, lvl) {
                        stop = px - d * if d > 0.0 { sl } else { ss };
                        let t = new_trade(d, j, px, "stop", i);
                        // stop on the entry bar: TradeStation bar path O -> nearer extreme -> other extreme -> C
                        // (p.entry_day=1: counted whenever the bar reaches it)
                        let hit = if g("entry_day") >= 0.5 { if d > 0.0 { x.l[j] <= stop } else { x.h[j] >= stop } } else { hit_after_entry(x, j, d, lvl, stop) };
                        if hit {
                            out.push(finish(t, j, stop, j, "Stop loss (entry day)"));
                        } else {
                            cur = Some(t);
                        }
                    }
                }
            }
            // a trade entered on bar `to` (possible only when i+1 == to) stays open
        }
        _ => return Some(vec![]),
    }
    mark_open(&mut out, cur, x, to);
    Some(out)
}

/// Bar path as TradeStation assumes it without intrabar data: open, the extreme nearer the open, the other extreme, close.
pub fn bar_path(x: &Ctx, i: usize) -> [f64; 4] {
    if x.h[i] - x.o[i] <= x.o[i] - x.l[i] { [x.o[i], x.h[i], x.l[i], x.c[i]] } else { [x.o[i], x.l[i], x.h[i], x.c[i]] }
}

/// After a stop entry in direction d at level `lvl` on bar i, does the path later reach the protective `stop`?
pub fn hit_after_entry(x: &Ctx, i: usize, d: f64, lvl: f64, stop: f64) -> bool {
    let p = bar_path(x, i);
    // first point at/after which price has reached the entry level
    let k = if (p[0] - lvl) * d >= 0.0 { 0 } else { match (1..4).find(|&k| (p[k] - lvl) * d >= 0.0) { Some(k) => k, None => return false } };
    // from the entry (on the segment ending at point k) the price continues through points k..3
    (k..4).any(|m| (p[m] - stop) * d <= 0.0)
}

/// True when the bar after i falls in a later calendar (ISO) week, i.e. bar i is the last trading day of its week.
pub fn week_changes(x: &Ctx, i: usize) -> bool {
    i + 1 < x.r.len() && week_key(&x.r[i].date) != week_key(&x.r[i + 1].date)
}

fn week_key(d: &str) -> i64 {
    // days since 1970-01-01 (proleptic Gregorian), shifted so weeks start on Monday
    let y: i64 = d[0..4].parse().unwrap_or(1970);
    let m: i64 = d[5..7].parse().unwrap_or(1);
    let dd: i64 = d[8..10].parse().unwrap_or(1);
    let (y2, m2) = if m <= 2 { (y - 1, m + 12) } else { (y, m) };
    let days = 365 * y2 + y2 / 4 - y2 / 100 + y2 / 400 + (153 * (m2 - 3) + 2) / 5 + dd - 719469;
    (days + 3).div_euclid(7) // 1970-01-01 was a Thursday
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn row(d: &str, o: f64, h: f64, l: f64, c: f64) -> Row {
        Row { ticker: "T".into(), date: d.into(), open: o, high: h, low: l, close: c, volume: None }
    }
    #[test]
    fn week_key_monday_start() {
        assert_eq!(week_key("2006-09-01"), week_key("2006-08-28")); // Fri, Mon same week
        assert_ne!(week_key("2006-09-01"), week_key("2006-09-04")); // Fri vs next Mon
    }
    #[test]
    fn fri_mon_hand_checked() {
        // Fri up close, Mon trades through Fri high + 5.4, Tue opens above entry -> FPO exit
        let r = vec![
            row("2006-08-31", 100., 101., 99., 100.5),
            row("2006-09-01", 100., 102., 99.5, 101.5), // Friday: close > open
            row("2006-09-04", 102., 108., 101., 107.),  // Monday: buy stop 107.4? high 108 >= 107.4
            row("2006-09-05", 107.9, 109., 107., 108.), // opens above entry -> exit 107.9
        ];
        let x = Ctx::new(&r, &r, &[]);
        let ts = run("jb624_fri_mon", &x, 0, 3, &Params::new()).unwrap();
        assert_eq!(ts.len(), 1);
        assert_eq!(ts[0].legs[0].i, 2);
        assert!((ts[0].legs[0].px - 107.4).abs() < 1e-9);
        assert_eq!(ts[0].exit_i, 3);
        assert!((ts[0].exit_px - 107.9).abs() < 1e-9);
    }
    #[test]
    fn williams_ad_matches_definition() {
        let r = vec![row("2006-01-02", 10., 11., 9., 10.), row("2006-01-03", 10., 12., 10.5, 11.5), row("2006-01-04", 11., 11.2, 10., 10.5)];
        let x = Ctx::new(&r, &r, &[]);
        let ad = williams_ad(&x);
        assert!((ad[1] - 1.5).abs() < 1e-9); // 11.5 - min(10.5, 10)
        assert!((ad[2] - (1.5 - 1.0)).abs() < 1e-9); // 10.5 - max(11.2, 11.5)
    }
}
