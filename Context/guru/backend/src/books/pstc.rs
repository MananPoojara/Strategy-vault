//! Steve Primo, "Professional Swing Trading College" (TradingMarkets / Connors Research Group, Feb-May 2005):
//! Week 1-10 class notes + the "Other version" session decks, the course's TradeStation performance reports
//! (SPX/NDX/SOX R3, RSI 20, RSI 1%; R4 on 8 stocks) and the TC2000 scan code. Book id "pstc". Library notes,
//! source map and the independent oracle: library/pstc/.
//! Course conventions (Week 1 "Order Entry"): "All simulated results and back testing were done with Close to Close
//! testing"; every rule enters and exits on the close. Index methods were tested on $SPX.X / $NDX.X / $SOX.X
//! (TradeStation, 1 unit, no costs); the vault runs them on the indices (instrument swap only).
//! Not duplicates: stc9 `r3` (2010 course) exits on RSI(2) > 75 OR close > SMA5 and has no short side; this 2005
//! R3 exits on RSI(2) only. hpetf `hpetf_r3_*` (2009 book) adds on a lower close and exits at 70/30.
#![allow(dead_code)]
use crate::connors::*;

pub const BOOK: &str = "pstc";
/// Admitted ids (authors/primo/admission/SOURCE-CONTRACT.md).
pub const REVIEWED: &[&str] = &["pstc_rs2", "pstc_r3_long", "pstc_r3_short", "pstc_r4_long", "pstc_r4_short", "pstc_rsi20_long", "pstc_rsi20_short", "pstc_4_10_60"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

// ---------------------------------------------------------------- rules text

const R3_L: &[&str] = &[
    "F|\"1. Index is above its 200-day MA\" (Week 2 notes, R3 METHOD Rules (for buys))",
    "E|\"2. 2-day RSI falls 3 days in a row (inclusive)\" = RSI(2) below the prior day's RSI(2) on the last 2 closes (3 days counted inclusive); proved by the course's TradeStation list SPX_current_R3.xls (\"Drops3\"), see notes",
    "E|\"3. Today's 2-day RSI is below 10\"",
    "E|\"4. The 2-day RSI, 3 days ago, is below 60\" = RSI(2) on the first day of that 3-day window (\"The first falling day (day #1) ... must be below the 60 benchmark\", Other version Session 1)",
    "E|\"5. Buy at the close\"",
    "A|\"6. Buy an additional unit when the 2-day RSI falls 5 days in a row\" (5 consecutive lower RSI(2) closes, on a bar after the entry; one add, as the TradeStation setting \"up to 2 entry orders\")",
    "X|\"7. Exit at the close when the 2-day RSI closes above 75\"",
];
const R3_S: &[&str] = &[
    "F|\"1. Index is below its 200-day MA\" (Week 2 notes, R3 METHOD Rules (for sells))",
    "E|\"2. 2-day RSI rises 3 days in a row (inclusive)\" = RSI(2) above the prior day's RSI(2) on the last 2 closes",
    "E|\"3. Today's 2-day RSI is above 90\"",
    "E|\"4. The 2-day RSI, 3 days ago, is above 40\" (\"The first rising day (day #1) ... must be above the 40 benchmark\")",
    "E|\"5. Sell short at the close\" (Other version Session 1; the Week 2 notes print \"Buy at the close\", a typo on the sell page)",
    "A|\"6. Sell an additional unit when the 2-day RSI rises 5 days in a row\"",
    "X|\"7. Exit at the close when the 2-day RSI closes below 25\"",
];
const R4_L: &[&str] = &[
    "F|\"1. Stock is above its 200-day MA\" (Week 4/5 notes, R4 METHOD Rules (for buys))",
    "E|\"2. 2 period RSI falls 4 days in a row (inclusive)\" = RSI(2) lower than the prior day on the last 3 closes (the R3 'inclusive' count proved by the course's R3 trade list; p.inclusive=0 = 4 drops, the TC2000 scan)",
    "E|\"3. Today's 2 period RSI is below 10\"",
    "E|\"4. The 2 period RSI, 4 days ago, is below 60\" (first day of the 4-day window)",
    "E|\"5. Buy at the close\"",
    "A|\"6. Buy an additional unit when the 2 period RSI falls 5 days in a row\"",
    "X|\"7. Exit at the close when the 2 period RSI closes above 75\"",
];
const R4_S: &[&str] = &[
    "F|\"1. Stock is below its 200-day MA\" (Week 4/5 notes, R4 METHOD Rules (for sells))",
    "E|\"2. 2 period RSI rises 4 days in a row (inclusive)\"",
    "E|\"3. Today's 2 period RSI is above 90\"",
    "E|\"4. The 2 period RSI, 4 days ago, is above 40\"",
    "E|\"5. Sell at the close\"",
    "A|\"6. Sell an additional unit when the 2 period RSI rises 5 days in a row\"",
    "X|\"7. Exit at the close when the 2 period RSI closes below 25\"",
];
const RSI20_L: &[&str] = &[
    "F|\"1. Index is above its 200-day MA\" (Week 3 notes, RSI 20 METHOD Rules (for buys))",
    "E|\"2. 3 period RSI is less than 20\"",
    "E|\"3. Buy this bar on the close\"",
    "A|\"4. If one day has passed and the 3 period RSI is still below 20 - buy another unit\" = a close with RSI(3) < 20 at least 2 bars after the entry (one full day in between); proved by the course's TradeStation list SPX3periodRSI.xls and the Week 2 homework answer (buy 4/14/05, add 4/18/05)",
    "X|\"5. Sell once 3 period RSI closes above 55\"",
];
const RSI20_S: &[&str] = &[
    "F|\"1. Index is below its 200-day MA\" (Week 3 notes, RSI 20 METHOD Rules (for sells))",
    "E|\"2. 3 period RSI is greater than 80\"",
    "E|\"3. Sell this bar on the close\"",
    "A|\"4. If one day has passed and the 3 period RSI is still above 80 - sell another unit\"",
    "X|\"5. Buy to cover once 3 period RSI closes below 45\"",
];
const RSI1: &[&str] = &[
    "F|\"1. Index is above its 200-day MA\" (Week 3 notes, RSI 1% METHOD Rules (for buys only))",
    "E|\"2. 2 period RSI is less than 10\"",
    "E|\"3. Index closes down at least 1% on the day\" (close <= 0.99 x the prior close)",
    "E|\"4. Buy on close of bar\"",
    "X|\"5. Exit when the 2 period RSI closes above 80\"",
];
const RS2: &[&str] = &[
    "E|\"1. Buy the SPX (can use SPY or Futures) when the 2-period RSI is under 2\" (Other version, Session 1, RS2 METHOD); on the close",
    "X|\"2. Exit when the 2-period RSI closes above 75\"",
];
const M41060: &[&str] = &[
    "F|\"Stock is above its 200-day MA\" (Week 7 notes, 4/10/60 METHOD Rules (for buys))",
    "F|\"100-day Historical Volatility is greater than 20\" (TC2000 code: population st.dev of the last 100 ln(C/C1) x 100 x 16.1245 (= sqrt 260))",
    "E|\"Today's 4 period RSI is below 10\"",
    "E|\"Buy at the close\"",
    "X|\"Exit at the close when the 4 period RSI closes above 60\"",
];
const M204075: &[&str] = &[
    "E|\"Today's 20 period RSI is below 40\" (Week 8 notes, 20/40/75 METHOD Rules (for buys only))",
    "E|\"Buy Equity call LEAP at the close\" - the vault buys the underlying on the close (the course's results table is per underlying stock)",
    "X|\"Exit at the close when the 20 period RSI closes above 75\"",
];
const RS2DW: &[&str] = &[
    "E|\"1. 2-period RSI of SPY closes below 20\" (Week 6 notes, RS2/DW METHOD Rules (buys only))",
    "E|\"2. 2-period RSI on 8 or more S&P 100 stocks must close below 2\"",
    "F|\"3. Stocks must be above $5 price\"",
    "E|\"4. Buy at the close\"",
    "X|\"5. Exit at the close when the 2 period RSI closes above 75\"",
];

macro_rules! idx_note { () => { "Course test: TradeStation daily $SPX.X / $NDX.X / $SOX.X, 3/1994-2/2005 (200 bars warm-up), 1 unit per entry, every entry/add listed as its own trade, no costs. Wilder RSI as in TradeStation. The vault runs the rules unchanged on any index (NIFTY 50, NIFTY BANK, ...): instrument swap only. Each unit = half of the position allocation when the strategy can add a second unit." } }
macro_rules! stock_note { () => { "Meant for stocks (S&P 100 / Nasdaq 100). The vault runs one ticker per run: an index, or an NSE stock via the basket catalogue (basket:<list>:<SYMBOL>); instrument swap only. The course's per-stock results (ERICY, SEBL, SYMC, MXIM, NXTL, AMGN, ALTR, SUNW, S&P 100 stocks 1989-2005) cannot be checked: no US stock data in the vault." } }

// ---------------------------------------------------------------- catalogue

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    let d = |id, name, group, dir, status, rules, notes, claim| StrategyDef {
        id, name, group, session: 0, dir, status, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim,
    };
    vec![
        d("pstc_rs2", "RS2 Method (RSI(2) < 2, exit > 75)", "Session 1 · RS2", "Long", Live, RS2,
          concat!("Other version Session 1 (Week_1.pdf). No trend filter in the rule. Book instrument SPX (\"can use SPY or Futures\"); the printed points (487.25, quarter-point steps) look like S&P futures, so SPX reproduces it only roughly. ",
                  "Course test: TradeStation daily, close to close, 1 unit, no costs. The vault runs the rule unchanged on any index (instrument swap only)."),
          claim("25 (487.25 / 19.49)", "88%", "19.49 points", "6.82% of the time in the market", "1995-2005, SPX (Other version Session 1): total 487.25 points, PF 4.27")),
        d("pstc_r3_long", "R3 Method · long", "Week 2 · R3", "Long", Live, R3_L,
          concat!("Readings proved by the course's own TradeStation trade list (SPX_current_R3.xls, strategy _RSIdrops3row, 74 long entries, total +1164.23 pts): '3 days in a row (inclusive)' = 2 consecutive lower RSI(2) closes and '3 days ago' = the first day of that 3-day window (SPX 70/74 entries+exits identical; the 3-drop reading matches 22/74). The add needs 5 consecutive lower closes (5 drops: 70/74; 4 drops: 66/74). ",
                  "p.stop_pct = the Week 3 / Week 10 '% STOP RESULTS ON SPYs USING R3 METHOD' table (10/5/3/2/1%), default 0 = no stop (the rules have none). Stop reading [Not from the book]: a stop order at the first unit's price x (1 - %), any bar after the entry, gap fills at the open, whole position. ",
                  "Course test: TradeStation daily $SPX.X 3/14/1994-2/14/2005 (200 bars warm-up), 1 unit per entry, each add listed as its own trade, no costs. India: instrument swap only."),
          claim("74 (SPX long)", "87.84%", "15.73 points", "6.01 bars", "1/1/95-2/17/05 SPX long: net +1164.23 pts, PF 9.18; NDX long 69 trades, 86.96%, +2921.41; SOX long 62, 77.42%")),
        d("pstc_r3_short", "R3 Method · short", "Week 2 · R3", "Short", Live, R3_S,
          concat!("Same readings as pstc_r3_long, mirrored (SPX 27/27 short entries+exits identical with the course's trade list). ",
                  "Course test: TradeStation daily $SPX.X, 1 unit per entry, no costs. India: instrument swap only."),
          claim("27 (SPX short)", "59.26%", "12.70 points", "6.96 bars", "1/1/95-2/17/05 SPX short: net +342.77 pts, PF 5.14; NDX short 28 trades, 67.86%, +482.48; SOX short 42, 66.67%")),
        d("pstc_rsi20_long", "RSI 20 Method · long (RSI(3) < 20)", "Week 3 · RSI 20", "Long", Live, RSI20_L,
          concat!("Add-on reading proved by the course's TradeStation list SPX3periodRSI.xls (strategy 3dayRSItest): adds happen on any close with RSI(3) < 20 from the 2nd bar after the entry on, at most one add (SPX 94/96 long entries+exits identical; 'next day only' matches 71/96). ", idx_note!()),
          claim("96 (SPX long+adds of 137)", "75.91% (all trades)", "-", "5.49 bars (all trades)", "1995-2005 SPX all trades: net 1355.34 pts, PF 4.63, max DD -212.10 (Week 3)")),
        d("pstc_rsi20_short", "RSI 20 Method · short (RSI(3) > 80)", "Week 3 · RSI 20", "Short", Live, RSI20_S,
          concat!("Mirror of pstc_rsi20_long (SPX 41/41 short entries+exits identical with SPX3periodRSI.xls). ", idx_note!()),
          claim("41 (SPX short of 137)", "75.91% (all trades)", "-", "5.49 bars (all trades)", "1995-2005 SPX all trades: net 1355.34 pts, PF 4.63 (Week 3)")),
        d("pstc_rsi1pct", "RSI 1% Method (RSI(2) < 10 and down 1%)", "Week 3 · RSI 1%", "Long", Live, RSI1,
          concat!("[Book-vs-results gap] The printed results (SPX 1995-2005: net 1127.79, PF 5.76, 83.33%) are the course's TradeStation list SPX_RSI_1_ls.xls (strategy _2dayRSI<10test, 78 trades): it has NO 1% down-day condition (SPX 73/78 identical without it; with it 39/78). The default is the printed rule (down at least 1%); p.down_pct = 0 reproduces the trade list behind the printed numbers. ",
                  "'Closes down at least 1% on the day' = close <= prior close x (1 - 1%). ", idx_note!()),
          claim("78 (list without the 1% rule)", "83.33%", "-", "6.82 bars", "1995-2005 SPX: net 1127.79 pts, PF 5.76, max DD -138.12 (Week 3) - computed without the 1% condition, see notes")),
        d("pstc_r4_long", "R4 Method · long", "Week 4-5 · R4", "Long", Live, R4_L,
          concat!("Same wording as R3 with 4 days; read with the convention the R3 trade list proves (3 drops; RSI(2) of the first window day < 60). The TC2000 scan in the course folder (an Aug-2005 student file) codes 4 drops + 'RSI2.4 < 60': p.inclusive = 0 [labelled option]. ", stock_note!()),
          claim("58-68 per stock", "73-90%", "-", "5.8-7.2 bars", "1994-2005 (Week 4): ERICY 58 trades 89.66% PF 15.61; SEBL 54 77.78%; SYMC 67 77.61%; MXIM 64 79.69%; NXTL 68 82.35%; AMGN 67 82.09%; ALTR 67 82.09%; SUNW 60 73.33% (long+short)")),
        d("pstc_r4_short", "R4 Method · short", "Week 4-5 · R4", "Short", Live, R4_S,
          concat!("Mirror of pstc_r4_long. ", stock_note!()), None),
        d("pstc_4_10_60", "4/10/60 Method (RSI(4) < 10, HV > 20, exit > 60)", "Week 7 · 4/10/60", "Long", Live, M41060,
          concat!("HV per the course's TC2000 code: natural-log returns (TC2000 LOG), population st.dev over 100 returns, x sqrt(260) x 100. p.hv_gt 30 / 40 = the course's other two result tables. ", stock_note!()),
          claim("851", "75%", "2.21%", "13 bars", "S&P 100 stocks 1989-2005, HV > 20: PF 3.02, max loss -28.90%; HV > 30: 365 trades 74% avg 2.73%; HV > 40: 128 trades 79% avg 4.00% (Week 7)")),
        d("pstc_20_40_75", "20/40/75 Method (RSI(20) < 40, exit > 75) on the underlying", "Week 8 · 20/40/75 LEAPS", "Long", Live, M204075,
          concat!("The course trades equity call LEAPS on the signal; its results table is measured on the underlying stock ('Avg. Gain / Underlying Stock'), so the vault tests the signal on the underlying. The LEAPS P&L itself is not tested (needs option chains). ", stock_note!()),
          claim("474", "80%", "36.29% (underlying)", "599 days (median)", "S&P 100 stocks 1989-2005: PF 6.88, max gain 335.83%, max loss -99.46% (Week 8)")),
        StrategyDef {
            needs_series: &["SP100_RSI2_LT2"],
            ..d("pstc_rs2dw", "RS2/DW Method (SPY RSI(2) < 20 + 8 S&P 100 stocks RSI(2) < 2)", "Week 6 · RS2/DW", "Long",
                NeedsData("the daily count of S&P 100 member stocks (point-in-time membership) priced above $5 with RSI(2) < 2 (series SP100_RSI2_LT2); the vault has no US stock data. India would need the NSE equivalent count over NIFTY 50/100 members, which is a universe change, not an instrument swap"),
                RS2DW,
                "Coded against the series SP100_RSI2_LT2 (count of S&P 100 members with close > $5 and Wilder RSI(2) < 2 on that day); SPY's own RSI(2) and exit as printed. The exit reads SPY's RSI(2) (\"the 2 period RSI\").",
                claim("88", "83%", "1.42 points", "6.5 bars", "SPY 1994-2005: 125.21 points, PF 6.12, 18.9% of the time in the market, max DD -6.45 (Week 6); rs2dwresults.xls row 'All SP100, 8, 20, 75': 88 trades, 82.95%, 125.21 pts"))
        },
        d("pstc_index_scaling", "Scaling into index positions (25/50/75/100%)", "Week 9 · Portfolio", "Long/Short",
          NotTestable("which 'signals' are combined is never defined (\"The more signals the better\"), and the exit for a scaled position is left to discretion (\"if you want to use discretion, this is up to you\")"),
          &["E|\"One signal = 25% position, Two signals = 50% position, Three signals = 75% position, Four signals = 100% Full position\" (Week 9)",
            "E|\"1 or more signals on the same day = 25% position; Add another signal (and/or plus one's) at least one day later = 50% position; Add 1-2 more signals ... = 100% position\" (Week 9, Week 10 live portfolio)",
            "X|\"You may want to scale out - if you want to use discretion, this is up to you. Otherwise exit according to the rules.\""],
          "Not testable: the signal set is not specified and the exits are discretionary.", None),
        d("pstc_stock_portfolio", "Stock portfolio rules (10 positions, RSI / HV tiebreak)", "Week 9 · Portfolio", "Long/Short",
          NeedsData("a point-in-time S&P 100 / Nasdaq 100 stock universe on one equity curve (the engine backtests one ticker per run; no US stocks in the vault)"),
          &["F|\"Max Open Position = 10 (Initial and Second Entry are counted as 1 position)\" (Week 9)",
            "F|\"Max % of Equity for any Position = 10% for Initial Entry, additional 10% for Second Entry\"; \"Margin barrowing not allowed\"",
            "F|\"stocks are chosen (tiebreakers) by either highest/lowest value of RSI 2 indicator ... or Historical Volatility (i.e. highest HV100 gets priority)\""],
          "Portfolio layer for R4 on stocks; the tiebreak is an either/or the course leaves open.", None),
        d("pstc_live_r5", "Live trading portfolio · R5 equities with VIX filter", "Week 10 · Live portfolio", "Long/Short",
          NotTestable("the R5 rules are never stated (only \"R5's are better than R4's\"); extending R3/R4 to 5 days would invent a rule"),
          &["F|\"Equities - Rules for trading R5's: Only buying when VIX is 5% or more above it's 10-day sma; Only selling short when VIX is 5% or more below it's 10-day sma\" (Week 10)",
            "F|\"Each trade = 10% of the 50% / position\"; \"Max position size is 5% of the net asset value\"; \"RSI is the tiebreaker\""],
          "Not testable: R5 entry/exit rules are not in the course material.", None),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "pstc_rs2" => ("pstc_rs2", "PSTC · RS2", "RSI(2) < 2"),
        "pstc_r3_long" => ("pstc_r3", "PSTC · R3 Method", "Long"),
        "pstc_r3_short" => ("pstc_r3", "PSTC · R3 Method", "Short"),
        "pstc_r4_long" => ("pstc_r4", "PSTC · R4 Method", "Long"),
        "pstc_r4_short" => ("pstc_r4", "PSTC · R4 Method", "Short"),
        "pstc_rsi20_long" => ("pstc_rsi20", "PSTC · RSI 20 Method", "Long"),
        "pstc_rsi20_short" => ("pstc_rsi20", "PSTC · RSI 20 Method", "Short"),
        "pstc_rsi1pct" => ("pstc_rsi1pct", "PSTC · RSI 1% Method", "Long"),
        "pstc_4_10_60" => ("pstc_4_10_60", "PSTC · 4/10/60 Method", "Long"),
        "pstc_20_40_75" => ("pstc_20_40_75", "PSTC · 20/40/75 Method", "Underlying"),
        "pstc_rs2dw" => ("pstc_rs2dw", "PSTC · RS2/DW Method", "Needs S&P 100 breadth"),
        "pstc_index_scaling" | "pstc_stock_portfolio" | "pstc_live_r5" => ("pstc_portfolio", "PSTC · Portfolio rules", "Not mechanical / needs universe"),
        _ => return None,
    })
}

// ---------------------------------------------------------------- params

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let rdrop = |days: f64, long: bool| {
        let mut v = vec![
            p("trend", "200-day MA filter length", 200., 50., 300., 50.),
            p("days", "RSI(2) falls/rises N days in a row (inclusive count)", days, 2., 6., 1.),
            p("inclusive", "1 = 'inclusive' count (N days = N-1 consecutive RSI changes, as the course's R3 trade list); 0 = N changes (TC2000 scan) [labelled option]", 1., 0., 1., 1.),
            if long { p("first_lt", "RSI(2) on the first day of the window below", 60., 30., 90., 5.) } else { p("first_gt", "RSI(2) on the first day of the window above", 40., 10., 70., 5.) },
            if long { p("rsi_lt", "Today's RSI(2) below", 10., 2., 30., 1.) } else { p("rsi_gt", "Today's RSI(2) above", 90., 70., 98., 1.) },
            p("add_days", "Add a unit after N consecutive RSI(2) drops/rises (0 = no add)", 5., 0., 8., 1.),
            if long { p("exit_gt", "Exit when RSI(2) closes above", 75., 50., 95., 5.) } else { p("exit_lt", "Exit when RSI(2) closes below", 25., 5., 50., 5.) },
        ];
        if days == 3. {
            v.push(p("stop_pct", "% stop (Week 3/10 table: 10/5/3/2/1; 0 = none, the rules' default)", 0., 0., 10., 1.));
        }
        v
    };
    Some(match id {
        "pstc_rs2" => vec![p("rsi_lt", "Buy when RSI(2) below", 2., 1., 10., 1.), p("exit_gt", "Exit when RSI(2) closes above", 75., 50., 95., 5.)],
        "pstc_r3_long" => rdrop(3., true),
        "pstc_r3_short" => rdrop(3., false),
        "pstc_r4_long" => rdrop(4., true),
        "pstc_r4_short" => rdrop(4., false),
        "pstc_rsi20_long" => vec![
            p("trend", "200-day MA filter length", 200., 50., 300., 50.),
            p("rsi_lt", "Buy when RSI(3) below", 20., 5., 40., 5.),
            p("add_wait", "Add when RSI(3) still below after N full days have passed (0 = no add)", 1., 0., 5., 1.),
            p("exit_gt", "Sell when RSI(3) closes above", 55., 40., 80., 5.),
        ],
        "pstc_rsi20_short" => vec![
            p("trend", "200-day MA filter length", 200., 50., 300., 50.),
            p("rsi_gt", "Short when RSI(3) above", 80., 60., 95., 5.),
            p("add_wait", "Add when RSI(3) still above after N full days have passed (0 = no add)", 1., 0., 5., 1.),
            p("exit_lt", "Cover when RSI(3) closes below", 45., 20., 60., 5.),
        ],
        "pstc_rsi1pct" => vec![
            p("trend", "200-day MA filter length", 200., 50., 300., 50.),
            p("rsi_lt", "RSI(2) below", 10., 2., 30., 1.),
            p("down_pct", "Close down at least % on the day (0 = off: the trade list behind the printed results)", 1., 0., 3., 0.5),
            p("exit_gt", "Exit when RSI(2) closes above", 80., 50., 95., 5.),
        ],
        "pstc_4_10_60" => vec![
            p("trend", "200-day MA filter length", 200., 50., 300., 50.),
            p("rsi_lt", "RSI(4) below", 10., 2., 30., 1.),
            p("hv_gt", "HV(100) above (course tables: 20, 30, 40)", 20., 0., 60., 5.),
            p("exit_gt", "Exit when RSI(4) closes above", 60., 40., 90., 5.),
        ],
        "pstc_20_40_75" => vec![p("rsi_lt", "Buy when RSI(20) below", 40., 25., 50., 5.), p("exit_gt", "Exit when RSI(20) closes above", 75., 55., 90., 5.)],
        "pstc_rs2dw" => vec![
            p("spy_rsi_lt", "SPY RSI(2) below", 20., 2., 40., 1.),
            p("count_min", "At least N S&P 100 stocks with RSI(2) < 2", 8., 1., 30., 1.),
            p("exit_gt", "Exit when RSI(2) closes above", 75., 50., 95., 5.),
        ],
        _ => return None,
    })
}

// ---------------------------------------------------------------- engine

/// HV per the course's TC2000 code: population st.dev of the last n ln(C/C[-1]) x sqrt(260) x 100.
fn hv_tc2000(c: &[f64], n: usize) -> Vec<f64> {
    hv_ddof(c, n, 0.).iter().map(|v| v / 252f64.sqrt() * 260f64.sqrt()).collect()
}

/// Close-to-close engine: entry on the signal close, at most one added unit on a later close, exit on a close;
/// optional % stop (stop order at the first unit's price, gap through fills at the open). One position at a time.
fn sim(x: &Ctx, from: usize, to: usize, dir: f64, entry: &dyn Fn(usize) -> bool, add: Option<&dyn Fn(usize, usize) -> bool>,
       exit: &dyn Fn(usize) -> bool, stop: f64, why: &str) -> Vec<Trade> {
    let n = x.c.len();
    let to = to.min(n - 1);
    let w = if add.is_some() { 0.5 } else { 1. };
    let (mut out, mut cur): (Vec<Trade>, Option<Trade>) = (vec![], None);
    for i in from..=to {
        if let Some(t) = cur.as_mut() {
            let e = t.legs[0].i;
            if stop > 0. {
                let lvl = t.legs[0].px * (1. - dir * stop / 100.);
                let hit = if dir > 0. { x.l[i] <= lvl } else { x.h[i] >= lvl };
                if hit {
                    let px = if dir > 0. { x.o[i].min(lvl) } else { x.o[i].max(lvl) };
                    let mut t = cur.take().unwrap();
                    (t.exit_i, t.exit_px, t.signal_i, t.reason) = (i, px, i, format!("{}% stop", stop));
                    out.push(t);
                    continue;
                }
            }
            if exit(i) {
                let mut t = cur.take().unwrap();
                (t.exit_i, t.exit_px, t.signal_i, t.reason) = (i, x.c[i], i, why.to_string());
                out.push(t);
                continue;
            }
            if let Some(a) = add {
                if t.legs.len() == 1 && a(i, e) {
                    t.legs.push(Leg { i, px: x.c[i], w });
                }
            }
            continue;
        }
        if entry(i) {
            cur = Some(Trade { dir, legs: vec![Leg { i, px: x.c[i], w }], exit_i: i, exit_px: f64::NAN, reason: String::new(),
                entry_kind: "close", signal_i: i, open: false, exits: vec![] });
        }
    }
    if let Some(mut t) = cur {
        (t.exit_i, t.exit_px, t.reason, t.signal_i, t.open) = (to, x.c[to], "open".into(), to, true);
        out.push(t);
    }
    out
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let c = &x.c;
    let ok = |v: f64| v.is_finite();
    Some(match id {
        "pstc_rs2" => {
            let (lt, ex) = (g("rsi_lt"), g("exit_gt"));
            let r = &x.rsi2;
            sim(x, from, to, 1., &|i| ok(r[i]) && r[i] < lt, None, &|i| r[i] > ex, 0., "RSI(2) > exit level")
        }
        "pstc_r3_long" | "pstc_r3_short" | "pstc_r4_long" | "pstc_r4_short" => {
            let long = id.ends_with("_long");
            let s = if long { 1. } else { -1. };
            let ma = x.sma_n(g("trend") as usize);
            let r = &x.rsi2;
            let days = g("days") as usize;
            let k = if g("inclusive") >= 0.5 { days - 1 } else { days }; // consecutive RSI changes
            let addk = g("add_days") as usize;
            // k consecutive RSI(2) drops (long) / rises (short) ending on bar i
            let run_of = |i: usize, k: usize| i >= k && (i + 1 - k..=i).all(|j| ok(r[j]) && ok(r[j - 1]) && s * (r[j] - r[j - 1]) < 0.);
            let (first, ext, ex) = if long { (g("first_lt"), g("rsi_lt"), g("exit_gt")) } else { (g("first_gt"), g("rsi_gt"), g("exit_lt")) };
            let entry = |i: usize| {
                ok(ma[i]) && i >= k && run_of(i, k)
                    && if long { c[i] > ma[i] && r[i] < ext && r[i - k] < first } else { c[i] < ma[i] && r[i] > ext && r[i - k] > first }
            };
            let add = |i: usize, _e: usize| run_of(i, addk);
            let exit = |i: usize| if long { r[i] > ex } else { r[i] < ex };
            let stop = if days == 3 && schema.iter().any(|d| d.key == "stop_pct") { g("stop_pct") } else { 0. };
            sim(x, from, to, s, &entry, if addk > 0 { Some(&add) } else { None }, &exit, stop, "RSI(2) exit level")
        }
        "pstc_rsi20_long" | "pstc_rsi20_short" => {
            let long = id.ends_with("_long");
            let ma = x.sma_n(g("trend") as usize);
            let r = &x.rsi3;
            let (lvl, ex, wait) = if long { (g("rsi_lt"), g("exit_gt"), g("add_wait")) } else { (g("rsi_gt"), g("exit_lt"), g("add_wait")) };
            let ext = |i: usize| ok(r[i]) && if long { r[i] < lvl } else { r[i] > lvl };
            let entry = |i: usize| ok(ma[i]) && ext(i) && if long { c[i] > ma[i] } else { c[i] < ma[i] };
            let add = |i: usize, e: usize| i >= e + 1 + wait as usize && ext(i);
            let exit = |i: usize| if long { r[i] > ex } else { r[i] < ex };
            sim(x, from, to, if long { 1. } else { -1. }, &entry, if wait > 0. { Some(&add) } else { None }, &exit, 0., "RSI(3) exit level")
        }
        "pstc_rsi1pct" => {
            let ma = x.sma_n(g("trend") as usize);
            let r = &x.rsi2;
            let (lt, dn, ex) = (g("rsi_lt"), g("down_pct") / 100., g("exit_gt"));
            let entry = |i: usize| i > 0 && ok(ma[i]) && ok(r[i]) && c[i] > ma[i] && r[i] < lt && (dn <= 0. || c[i] <= c[i - 1] * (1. - dn));
            sim(x, from, to, 1., &entry, None, &|i| r[i] > ex, 0., "RSI(2) > exit level")
        }
        "pstc_4_10_60" => {
            let ma = x.sma_n(g("trend") as usize);
            let r = &x.rsi4;
            let hv = hv_tc2000(c, 100);
            let (lt, hvg, ex) = (g("rsi_lt"), g("hv_gt"), g("exit_gt"));
            let entry = |i: usize| ok(ma[i]) && ok(r[i]) && ok(hv[i]) && c[i] > ma[i] && r[i] < lt && hv[i] > hvg;
            sim(x, from, to, 1., &entry, None, &|i| r[i] > ex, 0., "RSI(4) > exit level")
        }
        "pstc_20_40_75" => {
            let r = x.rsi_n(20);
            let (lt, ex) = (g("rsi_lt"), g("exit_gt"));
            sim(x, from, to, 1., &|i| ok(r[i]) && r[i] < lt, None, &|i| r[i] > ex, 0., "RSI(20) > exit level")
        }
        "pstc_rs2dw" => {
            let cnt = x.series("SP100_RSI2_LT2")?;
            let r = &x.rsi2;
            let (lt, m, ex) = (g("spy_rsi_lt"), g("count_min"), g("exit_gt"));
            sim(x, from, to, 1., &|i| ok(r[i]) && r[i] < lt && cnt[i] >= m, None, &|i| r[i] > ex, 0., "RSI(2) > exit level")
        }
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn rows(cl: &[f64]) -> Vec<Row> {
        cl.iter().enumerate().map(|(i, &c)| Row { ticker: "T".into(), date: format!("d{:04}", i), open: c, high: c * 1.01, low: c * 0.99, close: c, volume: None }).collect()
    }
    #[test]
    fn r3_inclusive_count_add_and_exit() {
        // 229 rising bars (above the 200 MA), a small dip (bar 229, RSI(2) < 60), 2 sharp drops (bar 231 = entry:
        // 3 days inclusive), 2 more drops (bar 233 = 5 drops in a row: add), then a rally (exit).
        let mut cl: Vec<f64> = (0..229).map(|i| 100. + i as f64 * 0.5).collect();
        let mut v = *cl.last().unwrap() - 1.5;
        cl.push(v);
        for _ in 0..2 { v -= 3.; cl.push(v); }
        for _ in 0..2 { v -= 0.5; cl.push(v); }
        for _ in 0..3 { v += 4.; cl.push(v); }
        let r = rows(&cl);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("pstc_r3_long", &x, 0, r.len() - 1, &Params::new()).unwrap();
        let t = t.iter().find(|t| t.legs[0].i == 231).expect("entry on the 2nd drop (3 days inclusive)");
        assert!(x.rsi2[229] < 60. && x.rsi2[231] < 10.);
        assert_eq!(t.legs.len(), 2);
        assert_eq!(t.legs[1].i, 233, "add after 5 consecutive drops");
        assert!(x.rsi2[t.exit_i] > 75. && t.exit_px == cl[t.exit_i]);
        // 3-change reading (TC2000): no entry on bar 231
        let p = Params::from([("inclusive".to_string(), 0.)]);
        assert!(run("pstc_r3_long", &x, 0, r.len() - 1, &p).unwrap().iter().all(|t| t.legs[0].i != 231));
    }
}
