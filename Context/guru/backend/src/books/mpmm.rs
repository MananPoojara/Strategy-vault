//! Martin J. Pring, "Martin Pring on Market Momentum" (Probus / International Institute for Economic
//! Research, 1993; scan, 389 pdf pages; printed page = pdf page - 13 in the body). Book id `mpmm`.
//! Source and checks: library/mpmm/. Mostly interpretation (overbought/oversold, divergences, trendlines).
//! Pass 2 (2026-10-11) read Ch.4-7 and 11-12 in full and found the mechanical rules the book reports,
//! mostly Colby & Meyers' tests (Encyclopedia of Technical Market Indicators, 1988), Gartley's advanced
//! average, two trend-deviation systems, the long-term KST crossover and the McClellan Summation Index
//! crossover. These are Live. Fills at the signal bar's close (book silent; MetaStock convention as mpts).
//! Weekly / monthly rules read the last daily close of each Monday-based week / calendar month.
use crate::connors::{rsi, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};
use crate::books::mpts::sma;

pub const BOOK: &str = "mpmm";

/// Admitted after the 2026-10-23 full reading and independent ledgers (authors/pring/admission/SOURCE-CONTRACT.md).
pub const REVIEWED: &[&str] = &["mpmm_cm_rsi50_21w", "mpmm_cm_wma69w", "mpmm_cm_ema42w", "mpmm_cm_sma45w", "mpmm_gartley_25a3", "mpmm_cm_15_36w",
    "mpmm_cm_stoch39w", "mpmm_kst_lt", "mpmm_kst_wk", "mpmm_roc12_dir", "mpmm_dev26_40w", "mpmm_dev26_40w_adv10", "mpmm_summation35", "mpmm_summation35_nse"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

macro_rules! nt {
    ($id:expr, $name:expr, $group:expr, $why:expr, $rules:expr) => {
        StrategyDef { id: $id, name: $name, group: $group, session: 0, dir: "Long/Short", status: Status::NotTestable($why),
            needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules: $rules,
            notes: "Read in the scan; see library/mpmm/README.md.", claim: None }
    };
}

const LONG: &str = "Long when the rule says buy, flat when it says sell ('buy ... sell' with no mention of short selling); p.short=1 (not the default) reverses into shorts. Fill at the signal bar's close; no costs. Weekly bar = last trading day of each Monday-based week, monthly = last trading day of the month. Index swap: the book's S&P Composite / NYSE / stock tests run on NIFTY 50, SPX, DJI, ...";

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    let d = |id, name, group, dir, needs_series, rules, notes, claim| StrategyDef {
        id, name, group, session: 0, dir, status: Live, needs_volume: false, needs_vix: false, needs_series, book: BOOK, rules, notes, claim,
    };
    let cl = crate::connors::claim;
    vec![
        // ---------------- Live (pass 2)
        d("mpmm_cm_rsi50_21w", "Weekly 21-week RSI 50 crossover (Colby & Meyers)", "Ch.4 · Equilibrium Crossovers (p.100-101)", "Long", &[],
          &["E|\"Colby and Meyers found that the best results came from a simple equilibrium (50) crossover: Buy when above 50, sell when below. The best time spans were clustered on either side of the 20 weeks, with the 21-week span providing the highest profit of all.\" (p.100-101); \"based their research on weekly data using several decades of stock market data\" (p.100)",
            "F|RSI as the book computes it: average of the up closes over the span / average of the down closes, RSI = 100 - 100/(1+RS), window rolled one bar at a time (p.81, steps 1-7). [Read] p.81 says 'add the closing values for the up days'; read as the up closing changes (Wilder's definition, which the page cites). p.wilder=1 = Wilder's smoothing (the p.81 'short cut', not the default)",
            "X|Sell when the weekly RSI closes below 50 (equality = no signal)"],
          LONG, cl("75", "21 (28%)", "highest profit of all spans", "-", "weekly stock market data, several decades (p.101)")),
        d("mpmm_cm_wma69w", "Weekly close vs 69-week weighted MA (Colby & Meyers best)", "Ch.5 · Moving Averages (p.110)", "Long", &[],
          &["E|\"During that period the best weighted average crossover (69 weeks) turned in the best points profit, 118 points in the S&P Composite. This result compared with 112 and 111 for a 42-week EMA and a 45-week simple moving average, respectively\" (p.110); \"tested for a range of time spans (1-75 weeks) using weekly data\", \"the 19 years leading up to 1980\" (p.110)",
            "F|Weighted MA = linearly weighted, newest weight N ... oldest 1, divided by the sum of the weights (Table 5.3, p.108)",
            "X|Sell when the weekly close crosses below the average"],
          LONG, cl("-", "-", "+118 S&P points", "-", "S&P Composite weekly, 19 years to 1980 (p.110)")),
        d("mpmm_cm_ema42w", "Weekly close vs 42-week EMA (Colby & Meyers)", "Ch.5 · Moving Averages (p.110)", "Long", &[],
          &["E|\"112 ... for a 42-week EMA\" (p.110): buy when the weekly close crosses above the EMA",
            "F|EMA as the book computes it: start from the simple average, then EMA = EMA(yesterday) + exponent x (close - EMA(yesterday)), exponent = 2 / span (Tables 5.4-5.5, p.109: 10 weeks = 0.2)",
            "X|Sell when the weekly close crosses below the EMA"],
          LONG, cl("-", "-", "+112 S&P points", "-", "S&P Composite weekly, 19 years to 1980 (p.110)")),
        d("mpmm_cm_sma45w", "Weekly close vs 45-week SMA (Colby & Meyers)", "Ch.5 · Moving Averages (p.110)", "Long", &[],
          &["E|\"111 for ... a 45-week simple moving average\" (p.110): buy when the weekly close crosses above the SMA", "X|Sell when it crosses below"],
          LONG, cl("-", "-", "+111 S&P points", "-", "S&P Composite weekly, 19 years to 1980 (p.110)")),
        d("mpmm_gartley_25a3", "Close vs 25-day SMA advanced 3 days (Gartley)", "Ch.5 · Trend Deviation (p.114-116)", "Long", &[],
          &["E|\"He believed that the best combination of moving average and lead came from a 25-day simple moving average advanced by 3 days. He came to this conclusion from research covering the stock market from the late 1920s to the early 1930s\" (p.115); \"By a slight adaptation in plotting, [moving-average crossovers] can be made into a working tool\" (p.115)",
            "F|Advanced = the average of N days ago compared with today's close: \"instead of dividing today's close by today's average, we divide the closing price by the average five days ago\" (p.114)",
            "X|Sell when the close crosses below the advanced average"],
          LONG, None),
        d("mpmm_cm_15_36w", "Weekly 15/36-week SMA ratio zero crossover (Colby & Meyers)", "Ch.5 · Trend Deviation (p.121)", "Long", &[],
          &["E|\"When Colby and Meyers tested for the best combination of moving-average crossovers using weekly data for the S & P Composite, they found that between 1968 and 1986 the combination of a 15/36 week simple moving average worked best. They used the division instead of the subtraction calculation, basing buy and sell signals on a zero crossover.\" (p.121)",
            "X|Sell when the 15-week SMA / 36-week SMA ratio crosses back below its zero line (15-week below 36-week)"],
          LONG, cl("-", "-", "best combination", "-", "S&P Composite weekly 1968-1986 (p.121)")),
        d("mpmm_cm_stoch39w", "Weekly 39-week raw %K 50 crossover with close confirmation (Colby & Meyers)", "Ch.6 · How Does It Test? (p.138)", "Long/Short", &[],
          &["E|\"The rules they established called for a buy signal when the 39-week unsmoothed %K line (n = 39 with no moving average) crosses above 50%, and the %K and closing price are both above their previous closing levels.\" (p.138)",
            "X|\"They decided to both sell and sell short when the reverse conditions are met (i.e., when %K moves below 50% and the %K and closing prices are both below their previous week's levels).\" (p.138)",
            "F|%K = 100 x (close - lowest low of n weeks) / (highest high - lowest low) (Lane's %K, p.128); weekly high/low = the week's daily extremes. \"%D was not used\" (p.138). Signals only on the week of the 50 crossing."],
          "Always in after the first signal (sell and sell short / buy and cover). Book test: NYSE weekly 1977-86; 'fairly consistent profits were achieved for periods (n value) ranging from 38 to 66 weeks ... the 39-week span proved to be the most profitable' (p.138). Fill at the signal week's close; index swap.",
          cl("-", "-", "most profitable span; profits for n = 38-66", "-", "NYSE weekly 1977-1986 (p.138)")),
        d("mpmm_kst_lt", "Long-term (monthly) KST vs its 9-month MA", "Ch.7 · The KST System (p.152-159)", "Long", &[],
          &["F|Long-term KST: ROC 9 smoothed 6, weight 1; ROC 12 / 6, weight 2; ROC 18 / 6, weight 3; ROC 24 / 9, weight 4, monthly data (Table 7.1, p.159; p.153 'the first 3 periods are smoothed with a 6-month moving average and the 24-month time span with a 9-month moving average')",
            "E|\"Periods of accumulation and distribution occur between the time that the summed ROC and its moving average change direction. The first level is triggered when the indicator changes direction, the second when it crosses its moving average, and the third when the average also reverses direction. In most cases the moving-average crossover offers the best combination of timely signals with a minimum of whipsaws.\" (p.153); Chart 7.4B: long-term KST and its 9-month MA",
            "X|Sell when the KST crosses below its 9-month MA",
            "F|[Read, B] ROC = close / close n months ago - 1; smoothing and signal line = simple averages: the monthly row of Table 7.1 has no EMA mark, and the weekly KSTs 'differ from those presented earlier because they are smoothed by exponential rather than simple moving averages' (p.161)",
            "F|[Open] 'It is also mandatory to ensure that KST signals are confirmed by some kind of trend reversal in the price series' (p.157, in the short-term KST section); the confirmation is not put into numbers, so it is not applied"],
          LONG, None),
        d("mpmm_summation35", "McClellan Summation Index vs its 35-day SMA · NYSE breadth", "Ch.12 · McClellan Summation Index (p.299-305)", "Long",
          &["NYSE_ADV_DEC.Advances", "NYSE_ADV_DEC.Declines"],
          &["F|\"McClellan Oscillator = [19 EMA of (A - D)] - [39 EMA of (A - D)]\", the two averages \"represent a 10% and 5% trend value respectively\" (p.299): smoothing constants 0.10 and 0.05 as printed (Pring's 2/n rule would give 0.105 and 0.051)",
            "F|\"The Summation Index ... is calculated by accumulating the daily readings of the McClellan Oscillator\" (p.301)",
            "E|\"Trial and error has shown that a 35-day simple moving-average crossover offered very good results in the 1980s. However, as with all purely mechanical approaches there were some years when it did not operate well at all.\" (p.305, Chart 12.6 S&P 500 1981-84)",
            "X|Sell when the Summation Index crosses below its 35-day SMA",
            "F|[Read, B] EMAs start from the first A-D value; the Summation start value cancels out of the crossover. Days without breadth data are skipped."],
          "Daily NYSE advances/declines (extra_data NYSE_ADV_DEC, 1965-2020) applied to the US index; mpmm_summation35_nse uses NSE breadth for Indian indices. Long/flat default; p.short=1 adds shorts. Fill at the signal day's close.",
          cl("-", "-", "'very good results in the 1980s'", "-", "S&P 500 1981-1984 (Chart 12.6)")),
        d("mpmm_summation35_nse", "McClellan Summation Index vs its 35-day SMA · NSE breadth", "Ch.12 · McClellan Summation Index (p.299-305)", "Long",
          &["NSE_ADV_DEC.Advances", "NSE_ADV_DEC.Declines"],
          &["F|Same rules as mpmm_summation35 with NSE advances/declines (breadth swap only)",
            "E|\"a 35-day simple moving-average crossover\" (p.305): buy when the Summation Index crosses above its 35-day SMA",
            "X|Sell when the Summation Index crosses below its 35-day SMA"],
          "India mapping (E): NSE_ADV_DEC (from 1999) instead of NYSE breadth. Same conventions as mpmm_summation35.", None),
        d("mpmm_dev26_40w", "Weekly 26/40-week deviation crossing its 10-week MA", "Ch.5 · Trend Deviation (p.119)", "Long", &[],
          &["F|\"Chart 5.9 shows a 26-week moving average of the closing price divided by a 40-week average\" (p.119)",
            "E|\"Buy and sell signals are generated when the oscillator crosses above and below its 10-week moving average.\" (p.119)",
            "X|Sell when the oscillator crosses below its 10-week MA"],
          LONG, None),
        d("mpmm_dev26_40w_adv10", "Weekly 26/40-week deviation (40-week advanced 10 weeks) crossing its 10-week MA", "Ch.5 · Trend Deviation (p.119-120)", "Long", &[],
          &["F|\"Chart 5.10 shows essentially the same combination, except that the 40-week moving average was advanced by 10 weeks\" (p.119): oscillator = 26-week SMA / 40-week SMA of 10 weeks ago",
            "E|Buy when the oscillator crosses above its 10-week moving average (p.119)",
            "X|Sell when the oscillator crosses below its 10-week moving average (p.119)"],
          LONG, None),
        d("mpmm_kst_wk", "Weekly KST market-cycle model: KST crossing its EMA (long-term 26 weeks by default)", "Ch.7 · The KST Market Cycle Model (p.157-164)", "Long", &[],
          &["F|Table 7.1 (p.159) weekly KSTs, weights 1-4: short-term ROC 3/4/6/10 smoothed by 3/4/6/8-week EMAs; intermediate ROC 10/13/15/20 smoothed by 10/13/15/20-week EMAs; long-term ROC 39/52/78/104 smoothed by 26/26/26/39-week EMAs; \"These indicators differ from those presented earlier because they are smoothed by exponential rather than simple moving averages\" (p.161)",
            "E|\"The best signals are given when the KST crosses above or below its exponential moving average (EMA)\" (p.161); \"The short-term KST uses an 8-week exponential moving average for the crossover, the intermediate-term uses a 10-week moving average and the long-term a 26-week moving average\" (Chart 7.10, p.162): buy when the weekly KST crosses above its EMA",
            "X|Sell when the KST crosses below its EMA. p.kst = 1 short-term (8-week EMA), 2 intermediate (10-week EMA), 3 long-term (26-week EMA, default)",
            "F|EMA as the book computes it (Tables 5.4-5.5): exponent 2 / span, started from the simple average"],
          "Weekly bar = last trading day of each Monday-based week. Combining the three KSTs into market-cycle stages is judgement (p.163) and is not run; each KST runs on its own. The p.157 sentence that KST signals must be 'confirmed by some kind of trend reversal in the price series' is not put into numbers and is not applied [open]. Long/flat by default (p.short = 1 adds shorts). Fill at the signal week's close.", None),
        d("mpmm_roc12_dir", "12-month MA of the 12-month ROC: trade its changes of direction", "Ch.3 · Rate of Change (p.74)", "Long", &[],
          &["E|\"In chart 3.17, changes in direction of the 12-month moving average of a 12-month ROC are used as the signaling points. These are indicated by the arrows.\" (p.74): buy when the average turns up",
            "X|Sell when the average turns down (an unchanged month keeps the position)",
            "F|ROC = month-end close / close 12 months earlier; simple average of the last 12 month-end values [simple: the book's default average]"],
          LONG, None),
        // ---------------- Not testable (pass 1, refined)
        nt!("mpmm_momentum_ma_crossovers", "Double MA crossover, MA directional change, overbought/oversold and equilibrium crossovers on a momentum series", "Ch.2 · Principles of momentum interpretation II (p.41-50)",
            "no indicator, time span or smoothing is fixed: 'unless the selection of a moving average is made very carefully' (p.49); overbought/oversold lines are set by judgement (p.56-58); OB/OS crossovers are 'not appropriate for a mechanical trading system unless subsidiary rules are developed' (p.48)",
            &["E|\"Buy alerts are given when the indicator moves above zero and sell signals when it moves below zero\" (equilibrium crossover, p.49) - span unspecified"]),
        nt!("mpmm_divergence_trendlines", "Divergences, complex divergences, momentum trendlines, price patterns, peak-and-trough analysis", "Ch.1-7",
            "chart reading: 'Actual buy and sell signals can come only from a reversal in trend of the actual price' (p.4)",
            &["E|Interpretive principles only"]),
        nt!("mpmm_rsi_obos", "RSI 70/30 tops and bottoms, failure swings, smoothed RSI 65/35", "Ch.4 · Interpreting the RSI (p.86-100)",
            "levels 'vary with the time span' and are set by eye (p.86-87); failure swings and the 10-day MA of RSI (65/35, p.98) are judged on charts ('At first glance this approach looks to be almost perfect', p.98)",
            &["E|\"Signals are given when the average reverses direction from an extreme zone and crosses back on its way towards the 50 level\" (p.98)"]),
        nt!("mpmm_macd_trend_dev", "MACD signal-line crossovers, price envelopes, 1x25 advanced oscillators", "Ch.5 (p.112-124)",
            "MACD spans vary (9/15, Appel 8-17-9 and 12-25-9) and Pring calls the crossovers 'overrated' (p.122); 2% envelopes and the Chart 5.6-5.8 oscillators are illustrations without a stated signal rule; MACD systems are in apmacd",
            &["E|\"MACD crossovers generate buy and sell signals\" (p.122)"]),
        nt!("mpmm_stoch_lane", "Lane stochastic divergences, hinges, warnings, set-ups, failures", "Ch.6 (p.126-145)",
            "Lane's patterns are judged on charts; the one tested rule (Colby & Meyers 39-week) is mpmm_cm_stoch39w",
            &["E|Interpretive principles only"]),
        nt!("mpmm_kst_short_market_cycle", "Daily short-term KST and the combined market-cycle stages", "Ch.7-8 (p.157-178)",
            "the daily short-term KST has no stated signal-line span and its zero-crossover rule says 'take no action if the price has already moved a great deal' (p.170); combining the three weekly KSTs into stages is judgement (p.163). The weekly KST crossovers themselves run as mpmm_kst_wk",
            &["E|Short-term KST zero / MA crossovers with price confirmation (p.157-170)"]),
        nt!("mpmm_wilder_dmi", "Wilder directional movement rules and Elder's DMI version", "Ch.8 · Volatility and the Directional Movement System",
            "restatement of Wilder (New Concepts, 1978) and Elder (Trading for a Living, 1993) with unquantified filters; convert from the primary sources",
            &["E|\"Use crossovers of the +DI14 and -DI14 as timing points for entering and exiting the markets\""]),
        nt!("mpmm_wilder_parabolic_dps_asi", "Parabolic SAR, Directional Parabolic System, Accumulation Swing Index system, Commodity Selection Index", "Ch.9-10",
            "secondary restatement of Wilder's systems; ASI needs 'significant' swing points; CSI needs margin and commission data",
            &["E|Parabolic: AF 0.02 step, maximum 0.20, stop-and-reverse (Wilder)"]),
        nt!("mpmm_misc_volume_breadth", "Volume ROC/oscillators, Demand Index, McClellan OB/OS, Merrill weekly levels, diffusion, RS, net new highs, TRIN", "Ch.10-12 (p.273-324)",
            "interpretation only: McClellan +/-70-100 zones and Merrill's +/-280 weekly levels are forecasts of a 5-week move, not trades (p.300, 305); diffusion 20%/80% needs % of NYSE stocks above their MAs; net new highs and TRIN are read for divergences and 'zero-crossovers' without a fixed span; the tested Summation crossover is mpmm_summation35",
            &["E|Interpretive principles only"]),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "mpmm_cm_rsi50_21w" => ("mpmm_cm", "Colby & Meyers tests quoted by Pring", "RSI 21w / 50"),
        "mpmm_cm_wma69w" => ("mpmm_cm", "Colby & Meyers tests quoted by Pring", "69-week WMA"),
        "mpmm_cm_ema42w" => ("mpmm_cm", "Colby & Meyers tests quoted by Pring", "42-week EMA"),
        "mpmm_cm_sma45w" => ("mpmm_cm", "Colby & Meyers tests quoted by Pring", "45-week SMA"),
        "mpmm_cm_15_36w" => ("mpmm_cm", "Colby & Meyers tests quoted by Pring", "15/36-week ratio"),
        "mpmm_cm_stoch39w" => ("mpmm_cm", "Colby & Meyers tests quoted by Pring", "39-week %K / 50"),
        "mpmm_gartley_25a3" => ("mpmm_dev", "Pring trend deviation", "Gartley 25-day advanced 3"),
        "mpmm_dev26_40w" => ("mpmm_dev", "Pring trend deviation", "26/40 weeks vs 10-week MA"),
        "mpmm_dev26_40w_adv10" => ("mpmm_dev", "Pring trend deviation", "26/40 (advanced 10) vs 10-week MA"),
        "mpmm_kst_lt" => ("mpmm_kst", "Pring KST", "Long-term KST / 9-month MA"),
        "mpmm_kst_wk" => ("mpmm_kst", "Pring KST", "Weekly KST / EMA"),
        "mpmm_roc12_dir" => ("mpmm_kst", "Pring KST", "12-month MA of 12-month ROC turns"),
        "mpmm_summation35" => ("mpmm_breadth", "McClellan Summation crossover", "NYSE"),
        "mpmm_summation35_nse" => ("mpmm_breadth", "McClellan Summation crossover", "NSE"),
        "mpmm_wilder_dmi" | "mpmm_wilder_parabolic_dps_asi" => ("mpmm_wilder", "Wilder systems as restated by Pring", "-"),
        _ if id.starts_with("mpmm_") => ("mpmm_interp", "Pring momentum interpretation (not testable)", "-"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let short = p("short", "Also go short on sell signals (1)", 0., 0., 1., 1.);
    Some(match id {
        "mpmm_cm_rsi50_21w" => vec![p("n", "RSI span (weeks)", 21., 5., 52., 1.), p("lvl", "Equilibrium level", 50., 30., 70., 5.), p("wilder", "Wilder smoothing (1) / book's simple average (0)", 0., 0., 1., 1.), short],
        "mpmm_cm_wma69w" => vec![p("n", "Weighted MA (weeks)", 69., 1., 75., 1.), short],
        "mpmm_cm_ema42w" => vec![p("n", "EMA span (weeks)", 42., 1., 75., 1.), short],
        "mpmm_cm_sma45w" => vec![p("n", "SMA (weeks)", 45., 1., 75., 1.), short],
        "mpmm_gartley_25a3" => vec![p("n", "SMA (days)", 25., 5., 60., 1.), p("adv", "Advanced by (days)", 3., 0., 10., 1.), short],
        "mpmm_cm_15_36w" => vec![p("fast", "Fast SMA (weeks)", 15., 2., 40., 1.), p("slow", "Slow SMA (weeks)", 36., 10., 80., 1.), short],
        "mpmm_cm_stoch39w" => vec![p("n", "%K span (weeks)", 39., 5., 75., 1.), p("lvl", "Cross level", 50., 30., 70., 5.)],
        "mpmm_kst_lt" => vec![p("sig", "Signal MA (months)", 9., 3., 15., 1.), short],
        "mpmm_kst_wk" => vec![p("kst", "KST: 1 short-term, 2 intermediate, 3 long-term", 3., 1., 3., 1.), short],
        "mpmm_roc12_dir" => vec![p("roc", "ROC span (months)", 12., 3., 24., 1.), p("ma", "MA of the ROC (months)", 12., 2., 24., 1.), short],
        "mpmm_summation35" | "mpmm_summation35_nse" => vec![p("ma", "Summation MA (days)", 35., 5., 100., 1.), p("a1", "Fast EMA constant", 0.1, 0.02, 0.3, 0.01), p("a2", "Slow EMA constant", 0.05, 0.01, 0.2, 0.01), short],
        "mpmm_dev26_40w" => vec![p("fast", "Fast SMA (weeks)", 26., 2., 52., 1.), p("slow", "Slow SMA (weeks)", 40., 10., 80., 1.), p("sig", "Signal MA (weeks)", 10., 2., 26., 1.), short],
        "mpmm_dev26_40w_adv10" => vec![p("fast", "Fast SMA (weeks)", 26., 2., 52., 1.), p("slow", "Slow SMA (weeks)", 40., 10., 80., 1.), p("adv", "Slow MA advanced (weeks)", 10., 0., 26., 1.), p("sig", "Signal MA (weeks)", 10., 2., 26., 1.), short],
        _ => return None,
    })
}

// ---------------------------------------------------------------- helpers

fn day_number(date: &str) -> i64 {
    let (y, m, d): (i64, i64, i64) = (date[0..4].parse().unwrap(), date[5..7].parse().unwrap(), date[8..10].parse().unwrap());
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (m + if m > 2 { -3 } else { 9 }) + 2) / 5 + d - 1;
    era * 146_097 + (yoe * 365 + yoe / 4 - yoe / 100 + doy) - 719_468
}

fn week(date: &str) -> i64 { (day_number(date) + 3).div_euclid(7) }

/// Period-end bars (same convention as mpts): the next bar is in another Monday-based week / month; the
/// last data bar is a week end only on a Friday and never a month end.
fn period_ends(x: &Ctx, monthly: bool) -> Vec<bool> {
    (0..x.r.len()).map(|i| match x.r.get(i + 1) {
        Some(nx) if monthly => nx.date[..7] != x.r[i].date[..7],
        Some(nx) => week(&nx.date) != week(&x.r[i].date),
        None => !monthly && (day_number(&x.r[i].date) + 3).rem_euclid(7) == 4,
    }).collect()
}

/// f applied to the values on the `mask` bars, written back on those bars (NaN elsewhere).
fn on_mask(mask: &[bool], v: &[f64], f: impl Fn(&[f64]) -> Vec<f64>) -> Vec<f64> {
    let idx: Vec<usize> = (0..mask.len()).filter(|&i| mask[i]).collect();
    let r = f(&idx.iter().map(|&i| v[i]).collect::<Vec<_>>());
    let mut out = vec![f64::NAN; mask.len()];
    for (k, &i) in idx.iter().enumerate() { out[i] = r[k]; }
    out
}

fn sign(a: f64, b: f64) -> Option<i8> {
    if !(a.is_finite() && b.is_finite()) { None } else if a > b { Some(1) } else if a < b { Some(-1) } else { None }
}

fn shift(v: &[f64], k: usize) -> Vec<f64> { (0..v.len()).map(|i| if i >= k { v[i - k] } else { f64::NAN }).collect() }

/// Linearly weighted MA (Table 5.3).
pub fn wma(v: &[f64], n: usize) -> Vec<f64> {
    let den = (n * (n + 1)) as f64 / 2.;
    (0..v.len()).map(|i| if i + 1 < n { f64::NAN } else { (0..n).map(|k| (n - k) as f64 * v[i - k]).sum::<f64>() / den }).collect()
}

/// Pring's EMA (Tables 5.4-5.5): seeded with the n-period simple average, exponent 2/n.
pub fn ema_pring(v: &[f64], n: usize) -> Vec<f64> {
    let a = 2. / n as f64;
    let s = sma(v, n);
    let mut e = f64::NAN;
    (0..v.len()).map(|i| { e = if e.is_finite() { e + a * (v[i] - e) } else { s[i] }; e }).collect()
}

/// RSI as computed on p.81: simple averages of up and down changes over the last n changes.
pub fn rsi_simple(c: &[f64], n: usize) -> Vec<f64> {
    (0..c.len()).map(|i| {
        if i < n { return f64::NAN; }
        let (mut up, mut dn) = (0., 0.);
        for k in i + 1 - n..=i { let d = c[k] - c[k - 1]; if d > 0. { up += d } else { dn -= d } }
        if up + dn == 0. { f64::NAN } else if dn == 0. { 100. } else { 100. - 100. / (1. + up / dn) }
    }).collect()
}

fn ema_alpha(v: &[f64], a: f64) -> Vec<f64> {
    let mut e = f64::NAN;
    v.iter().map(|&x| { e = if e.is_finite() { e + a * (x - e) } else { x }; e }).collect()
}

fn open_trade(dir: f64, i: usize, px: f64) -> Trade {
    Trade { dir, legs: vec![Leg { i, px, w: 1. }], exit_i: i, exit_px: px, reason: String::new(), entry_kind: "close", signal_i: i, open: false, exits: vec![] }
}

/// `state[i]` = the rule's desired side on bar i (None = no information). A signal is a change of the
/// desired side versus the last known one (history before `from` counts; trades start inside the window).
fn run_states(x: &Ctx, from: usize, to: usize, state: &[Option<i8>], long_only: bool, why: [&str; 2]) -> Vec<Trade> {
    let (mut out, mut cur, mut last) = (vec![], None::<Trade>, None::<i8>);
    for i in 0..=to {
        let s = state[i];
        let signal = match (s, last) { (Some(a), Some(b)) if a != b => Some(a), _ => None };
        if s.is_some() { last = s; }
        if i < from { continue; }
        if let Some(sd) = signal {
            if let Some(mut t) = cur.take() {
                t.exit_i = i; t.exit_px = x.c[i]; t.signal_i = i; t.reason = (if sd > 0 { why[0] } else { why[1] }).into();
                out.push(t);
            }
            if (sd > 0 || !long_only) && i < to { cur = Some(open_trade(sd as f64, i, x.c[i])); }
        }
    }
    if let Some(mut t) = cur { t.exit_i = to; t.exit_px = x.c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true; out.push(t); }
    out
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    let n = x.c.len();
    if n == 0 || from > to || to >= n { return Some(vec![]); }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap().def);
    let u = |k: &str| g(k).round().max(1.) as usize;
    let long_only = schema.iter().any(|d| d.key == "short") && g("short") < 0.5;
    let c = &x.c;
    let wk = period_ends(x, false);
    let st_cross = |a: &[f64], b: &[f64]| -> Vec<Option<i8>> { (0..n).map(|i| sign(a[i], b[i])).collect() };
    Some(match id {
        "mpmm_cm_rsi50_21w" => {
            let k = u("n");
            let r = on_mask(&wk, c, |v| if g("wilder") >= 0.5 { rsi(v, k) } else { rsi_simple(v, k) });
            let lvl = vec![g("lvl"); n];
            run_states(x, from, to, &st_cross(&r, &lvl), long_only, ["weekly RSI closed above 50", "weekly RSI closed below 50"])
        }
        "mpmm_cm_wma69w" | "mpmm_cm_ema42w" | "mpmm_cm_sma45w" => {
            let k = u("n");
            let ma = on_mask(&wk, c, |v| match id { "mpmm_cm_wma69w" => wma(v, k), "mpmm_cm_ema42w" => ema_pring(v, k), _ => sma(v, k) });
            run_states(x, from, to, &st_cross(c, &ma), long_only, ["weekly close crossed above the average", "weekly close crossed below the average"])
        }
        "mpmm_gartley_25a3" => {
            let ma = shift(&sma(c, u("n")), g("adv").round().max(0.) as usize);
            run_states(x, from, to, &st_cross(c, &ma), long_only, ["close crossed above the advanced average", "close crossed below the advanced average"])
        }
        "mpmm_cm_15_36w" => {
            let (f, s) = (on_mask(&wk, c, |v| sma(v, u("fast"))), on_mask(&wk, c, |v| sma(v, u("slow"))));
            run_states(x, from, to, &st_cross(&f, &s), long_only, ["15/36 ratio crossed above zero", "15/36 ratio crossed below zero"])
        }
        "mpmm_dev26_40w" | "mpmm_dev26_40w_adv10" => {
            let adv = if id.ends_with("adv10") { g("adv").round().max(0.) as usize } else { 0 };
            let (f, s, m) = (u("fast"), u("slow"), u("sig"));
            let osc = on_mask(&wk, c, |v| {
                let (a, b) = (sma(v, f), shift(&sma(v, s), adv));
                (0..v.len()).map(|i| a[i] / b[i]).collect()
            });
            let sig = on_mask(&wk, &osc, |v| sma(v, m));
            run_states(x, from, to, &st_cross(&osc, &sig), long_only, ["oscillator crossed above its 10-week MA", "oscillator crossed below its 10-week MA"])
        }
        "mpmm_kst_lt" => {
            let me = period_ends(x, true);
            let m = u("sig");
            let kst = on_mask(&me, c, |v| {
                let roc = |k: usize| -> Vec<f64> { (0..v.len()).map(|i| if i >= k { 100. * (v[i] / v[i - k] - 1.) } else { f64::NAN }).collect() };
                let parts = [(9, 6, 1.), (12, 6, 2.), (18, 6, 3.), (24, 9, 4.)];
                let sm: Vec<Vec<f64>> = parts.iter().map(|&(r, s, _)| sma(&roc(r), s)).collect();
                (0..v.len()).map(|i| parts.iter().zip(&sm).map(|(&(_, _, w), s)| w * s[i]).sum()).collect()
            });
            let sig = on_mask(&me, &kst, |v| sma(v, m));
            run_states(x, from, to, &st_cross(&kst, &sig), long_only, ["KST crossed above its 9-month MA", "KST crossed below its 9-month MA"])
        }
        "mpmm_summation35" | "mpmm_summation35_nse" => {
            let pre = if id.ends_with("_nse") { "NSE_ADV_DEC" } else { "NYSE_ADV_DEC" };
            let (Some(a), Some(d)) = (x.series(&format!("{pre}.Advances")), x.series(&format!("{pre}.Declines"))) else { return Some(vec![]) };
            let mask: Vec<bool> = (0..n).map(|i| a[i].is_finite() && d[i].is_finite()).collect();
            let net: Vec<f64> = (0..n).map(|i| a[i] - d[i]).collect();
            let (a1, a2, m) = (g("a1"), g("a2"), u("ma"));
            let diff = on_mask(&mask, &net, |v| {
                let (e1, e2) = (ema_alpha(v, a1), ema_alpha(v, a2));
                let mut s = 0.;
                let sum: Vec<f64> = (0..v.len()).map(|i| { s += e1[i] - e2[i]; s }).collect();
                let ma = sma(&sum, m);
                (0..v.len()).map(|i| sum[i] - ma[i]).collect()
            });
            let zero = vec![0.; n];
            run_states(x, from, to, &st_cross(&diff, &zero), long_only, ["Summation Index crossed above its 35-day MA", "Summation Index crossed below its 35-day MA"])
        }
        "mpmm_kst_wk" => {
            let (parts, sig): ([(usize, usize, f64); 4], usize) = match g("kst").round() as i64 {
                1 => ([(3, 3, 1.), (4, 4, 2.), (6, 6, 3.), (10, 8, 4.)], 8),
                2 => ([(10, 10, 1.), (13, 13, 2.), (15, 15, 3.), (20, 20, 4.)], 10),
                _ => ([(39, 26, 1.), (52, 26, 2.), (78, 26, 3.), (104, 39, 4.)], 26),
            };
            let kst = on_mask(&wk, c, |v| {
                let roc = |k: usize| -> Vec<f64> { (0..v.len()).map(|i| if i >= k { 100. * (v[i] / v[i - k] - 1.) } else { f64::NAN }).collect() };
                let sm: Vec<Vec<f64>> = parts.iter().map(|&(r, s, _)| ema_pring(&roc(r), s)).collect();
                (0..v.len()).map(|i| parts.iter().zip(&sm).map(|(&(_, _, w), s)| w * s[i]).sum()).collect()
            });
            let sg = on_mask(&wk, &kst, |v| ema_pring(v, sig));
            run_states(x, from, to, &st_cross(&kst, &sg), long_only, ["KST crossed above its EMA", "KST crossed below its EMA"])
        }
        "mpmm_roc12_dir" => {
            let me = period_ends(x, true);
            let (rn, mn) = (u("roc"), u("ma"));
            let ma = on_mask(&me, c, |v| sma(&(0..v.len()).map(|i| if i >= rn { v[i] / v[i - rn] - 1. } else { f64::NAN }).collect::<Vec<_>>(), mn));
            let mi: Vec<usize> = (0..n).filter(|&i| me[i]).collect();
            let mut st = vec![None; n];
            for k in 1..mi.len() { st[mi[k]] = sign(ma[mi[k]], ma[mi[k - 1]]); }
            run_states(x, from, to, &st, long_only, ["the average turned up", "the average turned down"])
        }
        "mpmm_cm_stoch39w" => {
            let k = u("n");
            // weekly high / low on week-end bars
            let (mut wh, mut wl) = (vec![f64::NAN; n], vec![f64::NAN; n]);
            let (mut hh, mut ll) = (f64::MIN, f64::MAX);
            for i in 0..n {
                hh = hh.max(x.h[i]); ll = ll.min(x.l[i]);
                if wk[i] { wh[i] = hh; wl[i] = ll; hh = f64::MIN; ll = f64::MAX; }
            }
            let idx: Vec<usize> = (0..n).filter(|&i| wk[i]).collect();
            let lvl = g("lvl");
            let mut st = vec![None; n];
            let mut pk = f64::NAN;
            for (j, &i) in idx.iter().enumerate() {
                if j + 1 < k { continue; }
                let w = &idx[j + 1 - k..=j];
                let h = w.iter().map(|&q| wh[q]).fold(f64::MIN, f64::max);
                let l = w.iter().map(|&q| wl[q]).fold(f64::MAX, f64::min);
                let kk = if h > l { 100. * (c[i] - l) / (h - l) } else { f64::NAN };
                if pk.is_finite() && kk.is_finite() {
                    let pc = c[idx[j - 1]];
                    if pk <= lvl && kk > lvl && c[i] > pc { st[i] = Some(1) }
                    if pk >= lvl && kk < lvl && c[i] < pc { st[i] = Some(-1) }
                }
                pk = kk;
            }
            run_states(x, from, to, &st, false, ["39-week %K crossed above 50 with %K and close up", "39-week %K crossed below 50 with %K and close down"])
        }
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn book_tables() {
        // Table 5.5: a 10-period EMA uses exponent 0.2; seeded with the simple average
        let e = ema_pring(&[1., 2., 3., 4., 5., 6., 7., 8., 9., 10., 21.], 10);
        assert!((e[9] - 5.5).abs() < 1e-12 && (e[10] - (5.5 + 0.2 * (21. - 5.5))).abs() < 1e-12);
        // linear WMA of 3: (3*3 + 2*2 + 1*1)/6
        assert!((wma(&[1., 2., 3.], 3)[2] - 14. / 6.).abs() < 1e-12);
        // simple RSI: changes +2, -1 over 2 -> up 1, dn 0.5 -> RS 2 -> 66.67
        assert!((rsi_simple(&[10., 12., 11.], 2)[2] - 200. / 3.).abs() < 1e-9);
    }
}
