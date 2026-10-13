//! Chuck LeBeau's System Traders Club (traderclub.com) futures systems, 1998-2000: "Strategic" Swiss Franc v1.0,
//! "Roughneck" and "Wildcat" Crude Oil, "Phoenix", "Serendipity" v1.1, "Sidewinder" and "Big Dipper" T-Bond,
//! "First Sword" Yen v2.0 (each manual prints its TradeStation code and a trade-by-trade report), plus
//! "Millennium" and "Empire" Eurodollar (TradeStation 2000i code only, no manual). Sources, oracle and
//! verification: library/lebfut/.
//!
//! All of them are futures systems whose stops and offsets are in the contract's own dollars / ticks, so they
//! stay NeedsData here (the vault has no futures). The code is complete and, on the data files shipped with
//! the manuals, reproduces the printed trade lists (tests below).
//!
//! Engine: TradeStation 4 daily backtest, as in lebsp (the code runs at each bar's close and places orders
//! for the next bar; a stop fills at its price or at the open if the open is through it; several orders on
//! one bar fill in bar-path order: open, the nearer of high/low, the other, close; the money-management stop
//! is live from the fill on, including the entry bar). ADX/DMI follow TradeStation 4's own functions, which
//! start at CurrentBar 1 (the first bar after MaxBarsBack); this is what the printed lists need.
use crate::connors::{rsi, sma, true_range, Claim, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

const BOOK: &str = "lebfut";
const SF: &str = "Swiss franc futures (continuous back-adjusted, day session; $1,250 per point, tick 0.01). The $3,500 money stop (= 2.80 points) and the ATR exits are in franc points; no index equivalent";
const CL: &str = "Crude oil futures (continuous back-adjusted, day session; $1,000 per point, tick 0.01). The dollar stop and the \"100 points\" profit lock are crude-oil cents; no index equivalent";
const US: &str = "US 30-year T-bond futures (continuous back-adjusted, day session; $1,000 per point, tick 1/32). Entry offsets and stops are in 32nds and bond dollars; no index equivalent";
const JY: &str = "Japanese yen futures (continuous back-adjusted, day session; $1,250 per point, tick 0.01). The $2,000 money stop (= 1.60 points) has no index equivalent";
const ED: &str = "Eurodollar futures. The code's breakeven levels (0.16 / 0.08) and 'points' are Eurodollar price units; no index equivalent, and no manual or results exist to check the code against";

pub fn catalogue() -> Vec<StrategyDef> {
    let make = |id, name, group, dir, status, rules, notes, claim| StrategyDef {
        id, name, group, session: 0, dir, status, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim,
    };
    vec![
        make("lebfut_strategic", "Strategic Swiss Franc System v1.0", "Strategic Swiss manual (LeBeau, Tan & Elden, 1999)", "Long/Short", Status::NeedsData(SF),
            &["E|Long: \"If the CCI crosses above 170 today then buy tomorrow on the open.\" (code: if MarketPosition=0 and CCI(20) crosses above 170 then Buy Open)",
              "E|Short: \"If the CCI crosses below 170 today then sell short tomorrow on the open.\" (code: CCI(20) crosses below -170; flat only)",
              "X|Long: \"Exit on a stop at the lowest low of the last twenty-four days\"; close > entry + 3 ATR(25): stop at the 10-day low; trade high >= entry + 4 ATR: stop at entry + 2 ATR; close > entry + 6 ATR: stop at the 4-day low",
              "X|Long ADX divergence (code): MaxTradeClose > entry + 1 ATR and MaxTradeClose > MaxTradeClose[3] and ADX(14) < ADX(14)[3]: stop at today's low (prose: \"the highest close occurred within the last 3 days\")",
              "X|Short mirror: 24-day high; 10-day high after a close 3 ATR in profit; entry - 2 ATR once the trade low reaches entry - 4 ATR; 4-day high after 6 ATR; divergence (MinTradeClose < entry - 1 ATR, < MinTradeClose[5], ADX < ADX[5]): stop at the 5-day high",
              "X|\"Use a $3,500 money management stop for both long and short trades.\" (= 2.80 points). The code enters only when flat, so an opposite signal does not reverse."],
            "Manual 'Strategic Swiss Franc Trading System v1.0' (strategic.txt/.doc) + its TradeStation code; code wins where it differs from the prose (short CCI level -170; MaxTradeClose[3] test). Test 10/17/88-9/30/99, MaxBarsBack 50, $100 per trade. Reproduces all 54 printed trades on the shipped SF.LNG.",
            Some(Claim { trades: "54 (28 long, 26 short)", win: "78%", avg: "$2,320.37 (net of $100)", hold: "34 bars winners / 16 bars losers", period: "10/17/88-9/30/99, Swiss franc futures, $1,250/pt" })),
        make("lebfut_roughneck", "Roughneck Crude Oil System", "Roughneck Crude Oil manual (LeBeau & Elden, 1999)", "Long/Short", Status::NeedsData(CL),
            &["F|Long: \"The 10 day average of the lows must be higher than the 20 day average of the lows\"; 20-day average of the lows at least .25 above its value 10 days ago; close >= lowest low of 8 days + 1.5 ATR(45)",
              "E|Long: \"buy tomorrow on a stop at the highest high of the last four days\" (code: MarketPosition <> 1, so a short reverses)",
              "X|Long (code): BarsSinceEntry < 4 and close < entry: stop at the 5-day low; MaxTradeLow > entry + 2 / 4 / 5 ATR: stop at the 4-day / 2-day / 1-day low (prose says 3 days for the second; code used); MaxTradeClose >= entry + 100 points: stop at entry + 40 points",
              "F|Short: 10-day average of the highs below the 20-day; 20-day average of the highs at least .25 below its value 10 days ago; close <= highest high of 3 days - 2 ATR(45)",
              "E|Short: \"sell short tomorrow on a stop at the lowest low of the last four days\"",
              "X|Short (code): BarsSinceEntry < 5 and close > entry: stop at the 6-day high; MinTradeHigh < entry - 2 / 4 / 6 ATR: stop at the 8 / 2 / 1-day high; MinTradeClose <= entry - 120 points: stop at entry - 80 points (code only; the prose says no such short exit)",
              "X|$1,000 money-management stop (= 1.00 point)."],
            "Manual 'Roughneck Crude Oil Trading System' (rough.txt/.doc) + code; code wins. Points = 0.01. Test 10/17/88-12/31/98, MaxBarsBack 50, $100 per trade. Reproduces 58/59 printed trades on the shipped CL.LNG; the first book trade (1/12/89) starts 8 bars later than MaxBarsBack 50 implies (with p.mbb=58: 59/59).",
            Some(Claim { trades: "59 (43 long, 16 short)", win: "78%", avg: "$730.51 (net of $100)", hold: "16 bars winners / 11 bars losers", period: "10/17/88-12/31/98, crude oil futures, $1,000/pt" })),
        make("lebfut_wildcat", "Wildcat Crude Oil System v1.1", "Wildcat Crude Oil manual (LeBeau, Lucas & Elden, 1998)", "Long/Short", Status::NeedsData(CL),
            &["F|\"The 12 day Average Directional Index (ADX) must be higher today than it was yesterday\", \"higher today than it was 12 days ago\", \"above 15\"",
              "F|Long: close above the 30-day SMA; RSI(4) below 60. Short: close below the 30-day SMA; RSI(4) above 40",
              "E|Long: \"buy tomorrow on a stop if and when prices rise 0.40 Average True Ranges above today's close\" (ATR 45). Short: 0.40 ATR below the close. Flat only.",
              "X|\"A stop order at $900.00 below the entry price\" (= 0.90 point) or \"at the lowest low of the last 20 days\" (short: highest high of 20 days)",
              "X|\"if today's close is profitable by more than 8 Average True Ranges, then exit tomorrow on the open\" (short: 6 ATR). p.pfill_close=1 fills it at that next day's close instead, as the printed trade list does."],
            "Manual 'Wildcat Crude Oil Trading System' (WReadme.txt) + code. Default = the text: the profit exit fills at the next open (code \"tomorrow open\", prose \"exit tomorrow on the open\"). The printed list (made on the shipped Cl.txt, 1/3/84-4/22/98) fills these exits at the next day's close; with p.pfill_close=1, 26/38 printed trades are identical and all 12 others are 1984-88, where the printed prices sit 0.03 above the shipped file (a different data version).",
            Some(Claim { trades: "37 (18 long, 19 short)", win: "62%", avg: "$1,735.14 (net of $100)", hold: "30 bars winners / 15 bars losers", period: "1/3/84-4/22/98, crude oil futures, $1,000/pt" })),
        make("lebfut_phoenix", "Phoenix Bond System", "Phoenix T-Bond manual (Tan & LeBeau, 1998)", "Long", Status::NeedsData(US),
            &["F|\"The 11 day ADX must be greater than 20.\" \"The 11 day Plus DI must cross above the 11 day Minus DI.\" \"The 2-day Average True Range must be greater than the 6-day Average True Range.\"",
              "E|\"buy tomorrow at market on open\" (flat only)",
              "X|\"Exit at the lowest low of the last 22 days on a stop.\" \"Exit if loss reaches $1500 on a stop.\" (= 1.5 points)",
              "X|\"If the maximum close attained in the trade exceeds $1500 of profit, place a break-even stop at 6 ticks above the entry price.\" \"If the maximum close attained in the trade exceeds $7500 of profit, exit at the most recent low on a stop.\""],
            "Manual 'Phoenix Bond Trading System' (phxread.txt) + code. Test 1/4/88-4/22/98, MaxBarsBack 50, $100 per trade. Reproduces all 31 printed trades on the shipped Us.txt.",
            Some(Claim { trades: "31", win: "68%", avg: "$2,715.52 (net of $100)", hold: "46 bars winners / 8 bars losers", period: "1/4/88-4/22/98, T-bond futures, $1,000/pt" })),
        make("lebfut_serendipity", "Serendipity Bond System v1.1", "Serendipity T-Bond manual (LeBeau & Elden, 1999)", "Long/Short", Status::NeedsData(US),
            &["F|Long: close above the 32-day SMA and below the 8-day SMA. Short: close below the 32-day SMA and above the 8-day SMA",
              "E|Long: \"buy tomorrow only if and when prices rise 22 ticks (22/32) above today's opening price\". Short: 16 ticks (16/32) below today's open. An opposite entry reverses.",
              "X|Long: stop at the lowest low of 20 days; \"Take a profit at the open following any close that shows an open profit of one Average True Range or more\" (code: C > entry + 1 x ATR(45))",
              "X|Short: stop at the highest high of 20 days; BarsSinceEntry < 2 and close < entry: stop at today's high; next-open exit after a close 1 ATR in profit",
              "X|$1,250 money-management stop (= 1.25 points)."],
            "Manual 'Serendipity 1.1 Bond Trading System' (serendipity.txt) + code. Not the same rules as stad10_serendipity (Stridsman's RSI-based adaptation). Test 1/4/88-4/30/99 on the shipped UA.LNG, MaxBarsBack 50, $100 per trade. Reproduces all 140 printed trades.",
            Some(Claim { trades: "140 (68 long, 72 short)", win: "68%", avg: "$478.79 (net of $100)", hold: "4 bars winners / 3 bars losers", period: "1/4/88-4/30/99, T-bond futures, $1,000/pt" })),
        make("lebfut_sidewinder", "Sidewinder Bond System", "Sidewinder T-Bond manual (LeBeau & Elden, 1998)", "Long/Short", Status::NeedsData(US),
            &["F|\"The 14 day ADX must be one point or more lower than yesterday.\" (code: ADX(14) < ADX(14)[1] - 1)",
              "E|Long: RSI(3) below 45: \"buy tomorrow at todays high on a stop\". Short: RSI(3) above 55: \"sell tomorrow at todays low on a stop\". An opposite entry reverses.",
              "X|Long: close > entry + 5 ATR(4): stop at the 8-day low; close > entry + 2 ATR(4): stop at entry + 1.5 ATR",
              "X|Short: close < entry - 1 ATR(4): \"exit tomorrow at todays high on a stop\"",
              "X|$2,500 money-management stop (= 2.5 points)."],
            "Manual 'Sidewinder Bond Trading System' (SWread.txt) + code. Test 1/4/88-4/22/98, MaxBarsBack 50, $100 per trade. Reproduces all 96 printed trades on the shipped Us.txt (needs TradeStation's own ADX start at CurrentBar 1).",
            Some(Claim { trades: "96 (47 long, 49 short)", win: "78%", avg: "$996.68 (net of $100)", hold: "15 bars winners / 7 bars losers", period: "1/4/88-4/22/98, T-bond futures, $1,000/pt" })),
        make("lebfut_bigdipper", "Big Dipper Bond System", "Big Dipper T-Bond manual (LeBeau & Tan, 1998)", "Long", Status::NeedsData(US),
            &["F|\"The close must be above the 200-day simple moving average.\" \"above the 35-day simple moving average.\" \"The 9-day RSI must have declined more than 5 points from 4 days ago.\"",
              "E|\"buy tomorrow only if and when prices rise 8 ticks (8/32) above the opening price tomorrow\" (code: buy Open Tomorrow + 8 points stop; flat only)",
              "X|\"A stop order at $2,000 below the entry price.\" (= 2.0 points)",
              "X|\"If the most recent close is profitable by more than 10 ticks, place a stop order tomorrow to exit at the lowest low of the last 10 days.\" \"profitable by more than 30 ticks, place a stop order to exit the trade at 5 ticks above the entry price.\""],
            "Manual 'Big Dipper Bond Trading System' (BReadme.txt inside BReadme.rar) + code. Test 1/2/87-1/27/98, MaxBarsBack 200, $100 per trade. Reproduces all 43 printed trades on the shipped Us.txt.",
            Some(Claim { trades: "43", win: "88%", avg: "$1,712.50 (net of $100)", hold: "28 bars winners / 14 bars losers", period: "1/1/87-1/27/98, T-bond futures, $1,000/pt" })),
        make("lebfut_firstsword", "First Sword Yen System v2.0", "First Sword Yen manual (LeBeau & Elden, 1998)", "Long/Short", Status::NeedsData(JY),
            &["F|Long: \"the close is greater than the 40 day simple moving average\", \"below the 5 day simple moving average\", \"at least half of an average true range below the highest close of the last two days\" (ATR 4)",
              "E|Long: \"buy tomorrow on a stop at today's high\". Short mirror: sell stop at today's low. Code: MarketPosition <> 1 / <> -1, so an opposite entry reverses.",
              "X|Long: stop at the 12-day low; close > entry + 3 ATR: stop at entry + 2.5 ATR; close > entry + 5 ATR: stop at entry + 4.5 ATR; \"highest high of the trade minus 3.5 Average True Ranges\" (Chandelier)",
              "X|Short mirror: 12-day high; entry - 2.5 / - 4.5 ATR; lowest low of the trade + 3.5 ATR",
              "X|\"Use a $2,000 money management stop\" (= 1.60 points)."],
            "Manual 'First Sword Yen Trading System v2.0' (FSY2.TXT) + code. Test 1/4/88-12/17/98, MaxBarsBack 50, $100 per trade. Reproduces 83/85 printed trades on the shipped JY.LNG; the 2 others (March-April 1996) follow from an exact tie on 3/11/96 (close 107.90 = its 5-day SMA) that TradeStation's single-precision running sum resolved as 'below'.",
            Some(Claim { trades: "85 (36 long, 49 short)", win: "55%", avg: "$1,371.62 (net of $100)", hold: "19 bars winners / 6 bars losers", period: "1/4/88-12/17/98, yen futures, $1,250/pt" })),
        make("lebfut_millennium", "Millennium Eurodollar System", "Millennium Eurodollar (TradeStation 2000i code only)", "Long/Short", Status::NeedsData(ED),
            &["E|Long (code): MarketPosition <> 1 and close > SMA(35) and close < Highest(High,3) - 1 ATR(25) and close > Highest(High,3) - 3 ATR(25): buy next bar at this bar's high stop",
              "X|Long (code): close > entry + 8 ATR(25): exit next open; MaxTradeClose > entry + 4 ATR: stop at the 14-day low; always: stop at close - 1.5 ATR(5) (\"YoYo\"); MaxTradeClose > entry + 0.16: stop at entry + 0.08; BarsSinceEntry >= 24 and close < entry: exit next open",
              "E|Short (code): MarketPosition <> -1 and close < SMA(35) and close > Lowest(Low,3) + 1.5 ATR(25) and close < Lowest(Low,3) + 3 ATR(25): sell next bar at this bar's low stop",
              "X|Short (code): close < entry - 8 ATR: exit next open; MinTradeClose < entry - 4 ATR: stop at the 14-day high; always: stop at close + 3 ATR(5); MinTradeClose < entry - 0.16: stop at entry - 0.08; BarsSinceEntry >= 24 and close > entry: exit next open"],
            "Source: decompiled TradeStation 2000i code (Code.rar beside MILLENNIUM_EURODOLLAR.ELD); there is no manual, no money-management setting and no printed result, so nothing was added and nothing could be reproduced.",
            None),
        make("lebfut_empire", "Empire Eurodollar System", "Empire Eurodollar (TradeStation 2000i code only)", "Long/Short", Status::NeedsData(ED),
            &["E|Long (code): flat and XAverage(C,6) - XAverage(C,22) > 0: buy next bar at Highest(High,4) stop",
              "E|Short (code): flat and ADX(14) > 20 and XAverage(C,7) - XAverage(C,24) < 0: sell short next bar at Lowest(Low,3) stop",
              "X|(code) BarsSinceEntry > 0 and Lowest(Low,15) + 8 ATR(10) < Highest(High,2): long stop at close - 4 points; BarsSinceEntry > 0 and Highest(High,15) - 7 ATR(10) > Lowest(Low,2): short stop at close + 4 points",
              "X|(code) long: stop at Lowest(Low,21) + 7 points; short: stop at Highest(High,21) - 7 points"],
            "Source: decompiled TradeStation 2000i code ('From other source/Code.rar' beside EMPIRE EURODOLLAR.ELD); no manual, money-management setting or printed results. 'Point' is TradeStation's price value of one point for the contract; the code does not state it, so it is a parameter (p.point, set per contract).",
            None),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let fam = ("lebfut_futures", "LeBeau traderclub futures systems");
    Some(match id {
        "lebfut_strategic" => (fam.0, fam.1, "Strategic Swiss Franc"),
        "lebfut_roughneck" => (fam.0, fam.1, "Roughneck Crude"),
        "lebfut_wildcat" => (fam.0, fam.1, "Wildcat Crude"),
        "lebfut_phoenix" => (fam.0, fam.1, "Phoenix T-Bond"),
        "lebfut_serendipity" => (fam.0, fam.1, "Serendipity T-Bond"),
        "lebfut_sidewinder" => (fam.0, fam.1, "Sidewinder T-Bond"),
        "lebfut_bigdipper" => (fam.0, fam.1, "Big Dipper T-Bond"),
        "lebfut_firstsword" => (fam.0, fam.1, "First Sword Yen"),
        "lebfut_millennium" => (fam.0, fam.1, "Millennium Eurodollar"),
        "lebfut_empire" => (fam.0, fam.1, "Empire Eurodollar"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let tick = |t: f64| p("tick", "Order prices round to this tick (the contract's minimum move; 0 = exact)", t, 0., 1., 0.0025);
    Some(match id {
        "lebfut_strategic" => vec![
            p("cci_n", "CCI length", 20., 5., 60., 1.), p("cci_lvl", "CCI cross level (long +, short -)", 170., 50., 300., 5.), p("atr_n", "ATR length", 25., 5., 60., 1.),
            p("ch", "Initial channel stop (days)", 24., 5., 60., 1.), p("k3", "Tighter channel after a close this many ATRs in profit", 3., 0.5, 10., 0.25), p("ch3", "Tighter channel (days)", 10., 2., 40., 1.),
            p("k4", "Lock once the trade's extreme is this many ATRs in profit", 4., 0.5, 10., 0.25), p("lock", "Lock: ATRs from entry", 2., 0., 8., 0.25),
            p("k6", "Tightest channel after a close this many ATRs in profit", 6., 1., 15., 0.25), p("ch6", "Tightest channel (days)", 4., 1., 20., 1.),
            p("adx_n", "ADX length (divergence exit)", 14., 5., 40., 1.), p("dl", "Long divergence lookback (bars)", 3., 1., 10., 1.), p("ds", "Short divergence lookback (bars)", 5., 1., 10., 1.),
            p("dsh", "Short divergence stop: highest high of N days", 5., 1., 20., 1.),
            p("mm", "Money stop in points ($3,500 at $1,250/pt)", 2.8, 0.5, 10., 0.1), tick(0.01), p("mbb", "MaxBarsBack (bars before trading starts)", 50., 0., 300., 1.),
        ],
        "lebfut_roughneck" => vec![
            p("atr_n", "ATR length", 45., 10., 90., 1.), p("rise", "20-day average must have risen / fallen this much over 10 days", 0.25, 0., 2., 0.05),
            p("lk", "Long: close at least x ATR above the 8-day low", 1.5, 0.25, 5., 0.25), p("sk", "Short: close at least x ATR below the 3-day high", 2., 0.25, 5., 0.25),
            p("lock_at", "Long lock after the trade's highest close is this far up (100 points)", 1., 0., 5., 0.05), p("lock", "Long lock: above entry (40 points)", 0.4, 0., 5., 0.05),
            p("s_lock_at", "Short lock after the trade's lowest close is this far down (120 points)", 1.2, 0., 5., 0.05), p("s_lock", "Short lock: below entry (80 points)", 0.8, 0., 5., 0.05),
            p("mm", "Money stop in points ($1,000 at $1,000/pt)", 1., 0.1, 5., 0.05), tick(0.01), p("mbb", "MaxBarsBack (bars before trading starts)", 50., 0., 300., 1.),
        ],
        "lebfut_wildcat" => vec![
            p("adx_n", "ADX length", 12., 5., 40., 1.), p("adx_min", "ADX above", 15., 0., 40., 1.), p("ma_n", "Trend SMA length", 30., 5., 100., 1.),
            p("rsi_n", "RSI length", 4., 2., 14., 1.), p("rsi_l", "Long: RSI below", 60., 10., 90., 1.), p("rsi_s", "Short: RSI above", 40., 10., 90., 1.),
            p("atr_n", "ATR length", 45., 10., 90., 1.), p("k", "Entry: x ATR beyond the close", 0.4, 0., 2., 0.05), p("ch", "Channel stop (days)", 20., 5., 60., 1.),
            p("lt", "Long profit exit after a close x ATR in profit", 8., 1., 20., 0.5), p("st", "Short profit exit after a close x ATR in profit", 6., 1., 20., 0.5),
            p("pfill_close", "1 = profit exit fills at the next day's close (as the printed list); 0 = next open (code and prose)", 0., 0., 1., 1.),
            p("mm", "Money stop in points ($900 at $1,000/pt)", 0.9, 0.1, 5., 0.05), tick(0.01), p("mbb", "MaxBarsBack (bars before trading starts)", 50., 0., 300., 1.),
        ],
        "lebfut_phoenix" => vec![
            p("dmi_n", "ADX / DI length", 11., 5., 30., 1.), p("adx_min", "ADX above", 20., 0., 50., 1.), p("a_fast", "Range expansion: fast ATR", 2., 1., 10., 1.), p("a_slow", "Range expansion: slow ATR", 6., 2., 30., 1.),
            p("ch", "Channel stop (days)", 22., 5., 60., 1.), p("be_at", "Breakeven after the highest close is this many points up ($1,500)", 1.5, 0., 10., 0.03125),
            p("be", "Breakeven stop above entry (6/32)", 0.1875, 0., 2., 0.03125), p("big_at", "Tight stop (today's low) after the highest close is this many points up ($7,500)", 7.5, 0., 20., 0.25),
            p("mm", "Money stop in points ($1,500 at $1,000/pt)", 1.5, 0.25, 10., 0.25), tick(0.03125), p("mbb", "MaxBarsBack (bars before trading starts)", 50., 0., 300., 1.),
        ],
        "lebfut_serendipity" => vec![
            p("ma_l", "Trend SMA length", 32., 10., 80., 1.), p("ma_s", "Dip / rally SMA length", 8., 3., 20., 1.),
            p("l_off", "Long: points above today's open (22/32)", 0.6875, 0., 3., 0.03125), p("s_off", "Short: points below today's open (16/32)", 0.5, 0., 3., 0.03125),
            p("ch", "Channel stop (days)", 20., 5., 60., 1.), p("atr_n", "ATR length (profit exit)", 45., 10., 90., 1.), p("pk", "Profit exit after a close x ATR in profit", 1., 0.25, 5., 0.25),
            p("tight", "Short tight stop while BarsSinceEntry below", 2., 0., 10., 1.),
            p("mm", "Money stop in points ($1,250 at $1,000/pt)", 1.25, 0.25, 10., 0.25), tick(0.03125), p("mbb", "MaxBarsBack (bars before trading starts)", 50., 0., 300., 1.),
        ],
        "lebfut_sidewinder" => vec![
            p("adx_n", "ADX length", 14., 5., 40., 1.), p("adx_drop", "ADX at least this much below yesterday", 1., 0., 5., 0.25), p("rsi_n", "RSI length", 3., 2., 14., 1.),
            p("rsi_l", "Long: RSI below", 45., 5., 70., 1.), p("rsi_s", "Short: RSI above", 55., 30., 95., 1.), p("atr_n", "ATR length", 4., 2., 20., 1.),
            p("lk1", "Long: tight channel after a close x ATR in profit", 5., 1., 10., 0.5), p("ch", "Long tight channel (days)", 8., 2., 30., 1.),
            p("lk2", "Long: lock after a close x ATR in profit", 2., 0.5, 10., 0.25), p("lock", "Long lock: entry + x ATR", 1.5, 0., 5., 0.25),
            p("sk", "Short: stop at today's high after a close x ATR in profit", 1., 0.25, 5., 0.25),
            p("mm", "Money stop in points ($2,500 at $1,000/pt)", 2.5, 0.25, 10., 0.25), tick(0.03125), p("mbb", "MaxBarsBack (bars before trading starts)", 50., 0., 300., 1.),
        ],
        "lebfut_bigdipper" => vec![
            p("ma_l", "Long-term SMA", 200., 50., 300., 5.), p("ma_m", "Intermediate SMA", 35., 10., 100., 1.), p("rsi_n", "RSI length", 9., 2., 30., 1.),
            p("rsi_lb", "RSI decline lookback (days)", 4., 1., 10., 1.), p("rsi_drop", "RSI decline more than", 5., 0., 30., 1.),
            p("off", "Entry: points above tomorrow's open (8/32)", 0.25, 0., 2., 0.03125), p("p1", "Channel stop after a close more than x points up (10/32)", 0.3125, 0., 3., 0.03125),
            p("ch", "Channel stop (days)", 10., 2., 40., 1.), p("p2", "Breakeven after a close more than x points up (30/32)", 0.9375, 0., 5., 0.03125),
            p("be", "Breakeven stop above entry (5/32)", 0.15625, 0., 2., 0.03125),
            p("mm", "Money stop in points ($2,000 at $1,000/pt)", 2., 0.25, 10., 0.25), tick(0.03125), p("mbb", "MaxBarsBack (bars before trading starts)", 200., 0., 300., 1.),
        ],
        "lebfut_firstsword" => vec![
            p("ma_l", "Trend SMA", 40., 10., 100., 1.), p("ma_s", "Pullback SMA", 5., 2., 20., 1.), p("atr_n", "ATR length", 4., 2., 30., 1.), p("dip", "Dip: x ATR from the 2-day extreme close", 0.5, 0., 3., 0.05),
            p("ch", "Channel stop (days)", 12., 3., 40., 1.), p("k1", "Lock 1 after a close x ATR in profit", 3., 0.5, 10., 0.25), p("s1", "Lock 1 stop: x ATR from entry", 2.5, 0., 10., 0.25),
            p("k2", "Lock 2 after a close x ATR in profit", 5., 0.5, 15., 0.25), p("s2", "Lock 2 stop: x ATR from entry", 4.5, 0., 15., 0.25), p("chand", "Chandelier: x ATR from the trade's extreme", 3.5, 0.5, 10., 0.25),
            p("mm", "Money stop in points ($2,000 at $1,250/pt)", 1.6, 0.25, 10., 0.05), tick(0.01), p("mbb", "MaxBarsBack (bars before trading starts)", 50., 0., 300., 1.),
        ],
        "lebfut_millennium" => vec![
            p("ma_n", "Trend SMA", 35., 5., 100., 1.), p("atr_n", "ATR length", 25., 5., 60., 1.), p("hh_n", "Setup: highest high / lowest low of N days", 3., 1., 10., 1.),
            p("l_lo", "Long setup: close at least x ATR below the N-day high", 1., 0., 5., 0.25), p("l_hi", "Long setup: close at most x ATR below the N-day high", 3., 0.5, 10., 0.25),
            p("s_lo", "Short setup: close at least x ATR above the N-day low", 1.5, 0., 5., 0.25), p("s_hi", "Short setup: close at most x ATR above the N-day low", 3., 0.5, 10., 0.25),
            p("pt", "Profit exit (next open) after a close x ATR in profit", 8., 1., 20., 0.5), p("ch_at", "Channel after the trade's best close is x ATR in profit", 4., 0.5, 10., 0.25), p("ch", "Channel (days)", 14., 3., 40., 1.),
            p("yy_n", "YoYo ATR length", 5., 2., 20., 1.), p("yy_l", "Long YoYo: x ATR below the close", 1.5, 0.25, 5., 0.25), p("yy_s", "Short YoYo: x ATR above the close", 3., 0.25, 5., 0.25),
            p("be_at", "Breakeven after the best close is this far in profit (price units)", 0.16, 0., 1., 0.01), p("be", "Breakeven stop: this far beyond entry", 0.08, 0., 1., 0.01),
            p("time", "Time exit (next open) when losing after N bars", 24., 2., 100., 1.), tick(0.0), p("mbb", "MaxBarsBack (bars before trading starts)", 50., 0., 300., 1.),
        ],
        "lebfut_empire" => vec![
            p("atr_n", "ATR length", 10., 3., 40., 1.), p("lf", "Long XAverage fast", 6., 2., 30., 1.), p("ls", "Long XAverage slow", 22., 5., 80., 1.),
            p("sf", "Short XAverage fast", 7., 2., 30., 1.), p("ss", "Short XAverage slow", 24., 5., 80., 1.), p("adx_min", "Short: ADX(14) above", 20., 0., 50., 1.),
            p("point", "Price value of one TradeStation point for the contract (not stated in the code; set per contract)", 0.01, 0.0001, 1., 0.0001),
            tick(0.0), p("mbb", "MaxBarsBack (bars before trading starts)", 50., 0., 300., 1.),
        ],
        _ => return None,
    })
}

// ---------------------------------------------------------------- helpers

fn lowest(v: &[f64], k: usize, n: usize) -> f64 { v[k + 1 - n.min(k + 1)..=k].iter().copied().fold(f64::INFINITY, f64::min) }
fn highest(v: &[f64], k: usize, n: usize) -> f64 { v[k + 1 - n.min(k + 1)..=k].iter().copied().fold(f64::NEG_INFINITY, f64::max) }
fn gt(a: f64, b: f64) -> bool { a > b } // NaN compares false
fn lt(a: f64, b: f64) -> bool { a < b }
fn atr(x: &Ctx, n: usize) -> Vec<f64> { sma(&true_range(x.r), n) }

/// TradeStation CCI(n): (TP - SMA(TP,n)) / (0.015 * mean |TP - SMA|), TP = H+L+C (the /3 cancels).
fn cci(x: &Ctx, n: usize) -> Vec<f64> {
    let tp: Vec<f64> = (0..x.c.len()).map(|i| x.h[i] + x.l[i] + x.c[i]).collect();
    (0..tp.len()).map(|i| {
        if i + 1 < n { return f64::NAN; }
        let w = &tp[i + 1 - n..=i];
        let m = w.iter().sum::<f64>() / n as f64;
        let md = w.iter().map(|v| (v - m).abs()).sum::<f64>() / n as f64;
        if md > 0. { (tp[i] - m) / (0.015 * md) } else { 0. }
    }).collect()
}

fn xaverage(v: &[f64], n: usize) -> Vec<f64> {
    let a = 2. / (n as f64 + 1.);
    let mut out: Vec<f64> = Vec::with_capacity(v.len());
    for (i, &x) in v.iter().enumerate() {
        let y = if i == 0 { x } else { out[i - 1] + a * (x - out[i - 1]) };
        out.push(y);
    }
    out
}

/// TradeStation 4 DMIPlus / DMIMinus / ADX with their CurrentBar logic; `cb1` = bar index of CurrentBar 1
/// (the first bar after MaxBarsBack). At CurrentBar 1 the DMs and true ranges of the last n bars are summed,
/// then Wilder-smoothed; ADX = running mean of DX while CurrentBar < n, then (ADX[1]*(n-1) + DX)/n.
/// Ties: "If MinusDM >= PlusDM then PlusDM = 0" and the mirror. Values before `cb1` are NaN.
fn dmi_ts(x: &Ctx, n: usize, cb1: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let (h, l) = (&x.h, &x.l);
    let len = x.c.len();
    let tr = true_range(x.r);
    let dm = |i: usize| {
        let p = (h[i] - h[i - 1]).max(0.);
        let m = (l[i - 1] - l[i]).max(0.);
        (if m >= p { 0. } else { p }, if p >= m { 0. } else { m })
    };
    let (mut pdi, mut mdi, mut dx, mut adx) = (vec![f64::NAN; len], vec![f64::NAN; len], vec![f64::NAN; len], vec![f64::NAN; len]);
    let (mut sp, mut sm, mut st) = (0., 0., 0.);
    let nf = n as f64;
    if cb1 < n { return (pdi, mdi, adx); }
    for i in cb1..len {
        let cb = i - cb1 + 1;
        if cb == 1 {
            (sp, sm, st) = (0., 0., 0.);
            for j in i + 1 - n..=i {
                let (a, b) = dm(j);
                sp += a; sm += b; st += tr[j];
            }
        } else {
            let (a, b) = dm(i);
            st = st - st / nf + tr[i];
            sp = sp - sp / nf + a;
            sm = sm - sm / nf + b;
        }
        pdi[i] = if st != 0. { 100. * sp / st } else { 0. };
        mdi[i] = if st != 0. { 100. * sm / st } else { 0. };
        let s = pdi[i] + mdi[i];
        dx[i] = if s != 0. { 100. * (pdi[i] - mdi[i]).abs() / s } else { 0. };
        adx[i] = if cb < n { dx[cb1..=i].iter().sum::<f64>() / cb as f64 } else { (adx[i - 1] * (nf - 1.) + dx[i]) / nf };
    }
    (pdi, mdi, adx)
}

// ---------------------------------------------------------------- TradeStation 4 daily order engine
// (copied from lebsp.rs, whose engine is private; adds a next-close market order and a stop priced off the
// next bar's open)

const EPS: f64 = 1e-7;

#[derive(Clone, Copy, PartialEq)]
enum K { Buy, Sell, Xl, Xs }
#[derive(Clone, Copy, PartialEq)]
enum Ty { Stop, Open, Close, StopOpen }
#[derive(Clone, Copy)]
struct Ord { k: K, ty: Ty, px: f64, name: &'static str }
fn o(k: K, ty: Ty, px: f64, name: &'static str) -> Ord { Ord { k, ty, px, name } }

#[derive(Clone, Copy, Default)]
struct Pos { mp: i8, ep: f64, ei: usize }

/// First (distance along the bar path, fill price) at which `px` is reached at or after `t0`.
/// Path: open -> nearer of high/low -> the other -> close. `rising`: fills when price trades up to px.
fn touch(o: f64, h: f64, l: f64, c: f64, px: f64, rising: bool, t0: f64) -> Option<(f64, f64)> {
    if t0 <= 0. && ((rising && o >= px) || (!rising && o <= px)) {
        return Some((0., o));
    }
    let pts = if h - o <= o - l { [o, h, l, c] } else { [o, l, h, c] };
    let mut t = 0.;
    for w in pts.windows(2) {
        let (a, b) = (w[0], w[1]);
        if a.min(b) - EPS <= px && px <= a.max(b) + EPS {
            let at = t + (px - a).abs();
            if at >= t0 - EPS { return Some((at, px)); }
        }
        t += (b - a).abs();
    }
    None
}

/// `code(k, pos)` runs at the close of bar k and returns the orders for bar k + 1. `mm` = money stop in points.
fn sim(x: &Ctx, from: usize, to: usize, mm: Option<f64>, tick: f64, mut code: impl FnMut(usize, Pos) -> Vec<Ord>) -> Vec<Trade> {
    let (op, hi, lo, cl) = (&x.o, &x.h, &x.l, &x.c);
    let mut out = vec![];
    let mut pos = Pos::default();
    let mut cur: Option<Trade> = None;
    let mut orders: Vec<Ord> = vec![];
    for i in from..=to {
        let (oo, hh, ll, cc) = (op[i], hi[i], lo[i], cl[i]);
        let mut ev: Vec<(f64, usize, f64, K, &'static str, u32)> = vec![];
        for (s, od) in orders.iter().enumerate() {
            match od.ty {
                Ty::Open => { ev.push((0., s, oo, od.k, od.name, 0)); continue; }
                Ty::Close => { ev.push((f64::MAX, s, cc, od.k, od.name, 0)); continue; }
                _ => {}
            }
            let mut px = if od.ty == Ty::StopOpen { oo + od.px } else { od.px };
            let rising = matches!(od.k, K::Buy | K::Xs);
            if tick > 0. {
                px = if rising { (px / tick - 1e-6).ceil() * tick } else { (px / tick + 1e-6).floor() * tick };
                px = (px * 1e6).round() / 1e6;
            }
            if let Some((t, f)) = touch(oo, hh, ll, cc, px, rising, 0.) { ev.push((t, s, f, od.k, od.name, 0)); }
        }
        let mut seq = orders.len();
        if let (Some(m), true) = (mm, pos.mp != 0) {
            if let Some((t, f)) = touch(oo, hh, ll, cc, pos.ep - pos.mp as f64 * m, pos.mp < 0, 0.) {
                ev.push((t, seq, f, if pos.mp > 0 { K::Xl } else { K::Xs }, "Money management stop", 0));
                seq += 1;
            }
        }
        let mut gen = 0u32;
        let mut done = vec![false; ev.len()];
        loop {
            let Some(j) = (0..ev.len()).filter(|&j| !done[j]).min_by(|&a, &b| ev[a].0.total_cmp(&ev[b].0).then(ev[a].1.cmp(&ev[b].1))) else { break };
            done[j] = true;
            let (t, _, px, k, name, g) = ev[j];
            match k {
                K::Xl | K::Xs => {
                    let want = if k == K::Xl { 1 } else { -1 };
                    if pos.mp == want && g == gen {
                        let mut tr = cur.take().unwrap();
                        tr.exit_i = i; tr.exit_px = px; tr.signal_i = i; tr.reason = name.into();
                        out.push(tr);
                        pos.mp = 0;
                    }
                }
                K::Buy | K::Sell => {
                    let want: i8 = if k == K::Buy { 1 } else { -1 };
                    if pos.mp == want { continue; }
                    if let Some(mut tr) = cur.take() {
                        tr.exit_i = i; tr.exit_px = px; tr.signal_i = i; tr.reason = format!("Reversed: {name}");
                        out.push(tr);
                    }
                    gen += 1;
                    cur = Some(Trade { dir: want as f64, legs: vec![Leg { i, px, w: 1. }], exit_i: i, exit_px: px, reason: String::new(),
                        entry_kind: if t == 0. { "open" } else { "stop" }, signal_i: i.saturating_sub(1), open: false, exits: vec![] });
                    pos = Pos { mp: want, ep: px, ei: i };
                    if let Some(m) = mm {
                        if let Some((t2, f)) = touch(oo, hh, ll, cc, px - want as f64 * m, want < 0, t) {
                            ev.push((t2, seq, f, if want > 0 { K::Xl } else { K::Xs }, "Money management stop", gen));
                            done.push(false);
                            seq += 1;
                        }
                    }
                }
            }
        }
        orders = if i < to { code(i, pos) } else { vec![] };
    }
    if let Some(mut t) = cur {
        t.exit_i = to; t.exit_px = x.c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true;
        out.push(t);
    }
    out
}

/// MaxTradeClose/High/Low and MinTradeClose/Low/High series functions as printed (kept while in the
/// position, reset to -/+999999 otherwise; the entry bar's own values count).
#[derive(Clone, Copy)]
struct Ext { mtc: f64, mth: f64, mtl: f64, mnc: f64, mnl: f64, mnh: f64 }
impl Ext {
    fn new() -> Self { Ext { mtc: -999999., mth: -999999., mtl: -999999., mnc: 999999., mnl: 999999., mnh: 999999. } }
    fn step(&self, mp: i8, h: f64, l: f64, c: f64) -> Self {
        let mut e = Ext::new();
        if mp == 1 { e.mtc = self.mtc.max(c); e.mth = self.mth.max(h); e.mtl = self.mtl.max(l); }
        if mp == -1 { e.mnc = self.mnc.min(c); e.mnl = self.mnl.min(l); e.mnh = self.mnh.min(h); }
        e
    }
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if from > to || to >= x.r.len() {
        return None;
    }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap().def);
    let u = |k: &str| g(k).round().max(1.) as usize;
    let (tick, start) = (g("tick"), from.max(g("mbb").round().max(0.) as usize));
    let mm = schema.iter().any(|d| d.key == "mm").then(|| g("mm"));
    let (h, l, c, op) = (&x.h, &x.l, &x.c, &x.o);
    if start > to { return Some(vec![]); }
    Some(match id {
        "lebfut_strategic" => {
            let (cc, a, adx) = (cci(x, u("cci_n")), atr(x, u("atr_n")), dmi_ts(x, u("adx_n"), start).2);
            let (lv, k3, k4, k6, lock) = (g("cci_lvl"), g("k3"), g("k4"), g("k6"), g("lock"));
            let (ch, ch3, ch6, dl, ds, dsh) = (u("ch"), u("ch3"), u("ch6"), u("dl"), u("ds"), u("dsh"));
            let mut e = Ext::new();
            let (mut mtc_h, mut mnc_h) = (vec![-999999.; x.c.len()], vec![999999.; x.c.len()]);
            sim(x, start, to, mm, tick, |k, pos| {
                e = e.step(pos.mp, h[k], l[k], c[k]);
                (mtc_h[k], mnc_h[k]) = (e.mtc, e.mnc);
                let mut v = vec![];
                if k < 1 { return v; }
                if pos.mp == 0 && gt(cc[k], lv) && !gt(cc[k - 1], lv) { v.push(o(K::Buy, Ty::Open, 0., "L Entry")); }
                if pos.mp == 1 {
                    let (ep, at) = (pos.ep, a[k]);
                    v.push(o(K::Xl, Ty::Stop, lowest(l, k, ch), "L 24 ch"));
                    if c[k] > k3 * at + ep { v.push(o(K::Xl, Ty::Stop, lowest(l, k, ch3), "L3/10Ch")); }
                    if e.mth >= ep + k4 * at { v.push(o(K::Xl, Ty::Stop, ep + lock * at, "L2Atr")); }
                    if c[k] > k6 * at + ep { v.push(o(K::Xl, Ty::Stop, lowest(l, k, ch6), "L6/4Ch")); }
                    if k >= dl && e.mtc > ep + at && e.mtc > mtc_h[k - dl] && lt(adx[k], adx[k - dl]) { v.push(o(K::Xl, Ty::Stop, l[k], "LDev")); }
                }
                if pos.mp == 0 && lt(cc[k], -lv) && !lt(cc[k - 1], -lv) { v.push(o(K::Sell, Ty::Open, 0., "S Entry")); }
                if pos.mp == -1 {
                    let (ep, at) = (pos.ep, a[k]);
                    v.push(o(K::Xs, Ty::Stop, highest(h, k, ch), "S 24 Ch"));
                    if c[k] < ep - k3 * at { v.push(o(K::Xs, Ty::Stop, highest(h, k, ch3), "S3/10Ch")); }
                    if e.mnl <= ep - k4 * at { v.push(o(K::Xs, Ty::Stop, ep - lock * at, "S2Atr")); }
                    if c[k] < ep - k6 * at { v.push(o(K::Xs, Ty::Stop, highest(h, k, ch6), "S6/4Ch")); }
                    if k >= ds && e.mnc < ep - at && e.mnc < mnc_h[k - ds] && lt(adx[k], adx[k - ds]) { v.push(o(K::Xs, Ty::Stop, highest(h, k, dsh), "SDev")); }
                }
                v
            })
        }
        "lebfut_roughneck" => {
            let a = atr(x, u("atr_n"));
            let (l10, l20, h10, h20) = (sma(l, 10), sma(l, 20), sma(h, 10), sma(h, 20));
            let (rise, lk, sk, la, lp, sa, sp) = (g("rise"), g("lk"), g("sk"), g("lock_at"), g("lock"), g("s_lock_at"), g("s_lock"));
            let mut e = Ext::new();
            sim(x, start, to, mm, tick, |k, pos| {
                e = e.step(pos.mp, h[k], l[k], c[k]);
                let mut v = vec![];
                if k < 10 || a[k].is_nan() || l20[k - 10].is_nan() { return v; }
                let at = a[k];
                let (c1, c3, c5) = (l10[k] > l20[k], l20[k] > l20[k - 10] + rise, c[k] >= lowest(l, k, 8) + lk * at);
                let (c2, c4, c6) = (h10[k] < h20[k], h20[k] < h20[k - 10] - rise, c[k] <= highest(h, k, 3) - sk * at);
                if pos.mp != 1 && c1 && c3 && c5 { v.push(o(K::Buy, Ty::Stop, highest(h, k, 4), "Breakout")); }
                if pos.mp == 1 {
                    let (ep, bse) = (pos.ep, k - pos.ei);
                    if bse < 4 && c[k] < ep { v.push(o(K::Xl, Ty::Stop, lowest(l, k, 5), "L Tight")); }
                    if e.mtl > ep + 2. * at { v.push(o(K::Xl, Ty::Stop, lowest(l, k, 4), "L Ch2")); }
                    if e.mtl > ep + 4. * at { v.push(o(K::Xl, Ty::Stop, lowest(l, k, 2), "L Ch3")); }
                    if e.mtl > ep + 5. * at { v.push(o(K::Xl, Ty::Stop, lowest(l, k, 1), "L Ch6")); }
                    if e.mtc >= ep + la { v.push(o(K::Xl, Ty::Stop, ep + lp, "L Points")); }
                }
                if pos.mp != -1 && c2 && c4 && c6 { v.push(o(K::Sell, Ty::Stop, lowest(l, k, 4), "Breakdown")); }
                if pos.mp == -1 {
                    let (ep, bse) = (pos.ep, k - pos.ei);
                    if bse < 5 && c[k] > ep { v.push(o(K::Xs, Ty::Stop, highest(h, k, 6), "S Tight")); }
                    if e.mnh < ep - 2. * at { v.push(o(K::Xs, Ty::Stop, highest(h, k, 8), "S Ch1")); }
                    if e.mnh < ep - 4. * at { v.push(o(K::Xs, Ty::Stop, highest(h, k, 2), "S Ch4")); }
                    if e.mnh < ep - 6. * at { v.push(o(K::Xs, Ty::Stop, highest(h, k, 1), "S Ch6")); }
                    if e.mnc <= ep - sa { v.push(o(K::Xs, Ty::Stop, ep - sp, "S Points")); }
                }
                v
            })
        }
        "lebfut_wildcat" => {
            let n = u("adx_n");
            let adx = dmi_ts(x, n, start).2;
            let (ma, r, a) = (sma(c, u("ma_n")), rsi(c, u("rsi_n")), atr(x, u("atr_n")));
            let (amin, rl, rs, kk, lt_, st_, ch) = (g("adx_min"), g("rsi_l"), g("rsi_s"), g("k"), g("lt"), g("st"), u("ch"));
            let pty = if g("pfill_close") >= 0.5 { Ty::Close } else { Ty::Open };
            sim(x, start, to, mm, tick, |k, pos| {
                let mut v = vec![];
                if k < n { return v; }
                let ok = gt(adx[k], adx[k - n]) && gt(adx[k], adx[k - 1]) && gt(adx[k], amin);
                if pos.mp == 0 && ok && gt(c[k], ma[k]) && lt(r[k], rl) { v.push(o(K::Buy, Ty::Stop, c[k] + kk * a[k], "Long Entry")); }
                if pos.mp == 1 {
                    v.push(o(K::Xl, Ty::Stop, lowest(l, k, ch), "Long Exit"));
                    if c[k] > pos.ep + lt_ * a[k] { v.push(o(K::Xl, pty, 0., "Long Profit")); }
                }
                if pos.mp == 0 && ok && lt(c[k], ma[k]) && gt(r[k], rs) { v.push(o(K::Sell, Ty::Stop, c[k] - kk * a[k], "Short Entry")); }
                if pos.mp == -1 {
                    v.push(o(K::Xs, Ty::Stop, highest(h, k, ch), "Short Exit"));
                    if c[k] < pos.ep - st_ * a[k] { v.push(o(K::Xs, pty, 0., "Short Profit")); }
                }
                v
            })
        }
        "lebfut_phoenix" => {
            let (pdi, mdi, adx) = dmi_ts(x, u("dmi_n"), start);
            let (af, asl) = (atr(x, u("a_fast")), atr(x, u("a_slow")));
            let (amin, ch, bea, be, biga) = (g("adx_min"), u("ch"), g("be_at"), g("be"), g("big_at"));
            let mut mc = 0.;
            sim(x, start, to, mm, tick, |k, pos| {
                let mut v = vec![];
                if k < 1 { return v; }
                if pos.mp == 0 && gt(adx[k], amin) && gt(pdi[k], mdi[k]) && !gt(pdi[k - 1], mdi[k - 1]) && gt(af[k], asl[k]) {
                    v.push(o(K::Buy, Ty::Open, 0., "Long Entry"));
                    mc = 0.;
                }
                if pos.mp == 1 {
                    if c[k] > mc { mc = c[k]; }
                    v.push(o(K::Xl, Ty::Stop, lowest(l, k, ch), "Long Channel Exit"));
                    if mc > pos.ep + bea { v.push(o(K::Xl, Ty::Stop, pos.ep + be, "Long Profit")); }
                    if mc > pos.ep + biga { v.push(o(K::Xl, Ty::Stop, l[k], "Long Large Profit")); }
                }
                v
            })
        }
        "lebfut_serendipity" => {
            let (ml, ms, a) = (sma(c, u("ma_l")), sma(c, u("ma_s")), atr(x, u("atr_n")));
            let (lo_, so_, ch, pk, tight) = (g("l_off"), g("s_off"), u("ch"), g("pk"), g("tight") as usize);
            sim(x, start, to, mm, tick, |k, pos| {
                let mut v = vec![];
                if pos.mp != 1 && gt(c[k], ml[k]) && lt(c[k], ms[k]) { v.push(o(K::Buy, Ty::Stop, op[k] + lo_, "Long Entry")); }
                if pos.mp == 1 {
                    v.push(o(K::Xl, Ty::Stop, lowest(l, k, ch), "Long Exit"));
                    if c[k] > pos.ep + pk * a[k] { v.push(o(K::Xl, Ty::Open, 0., "Long Profit")); }
                }
                if pos.mp != -1 && lt(c[k], ml[k]) && gt(c[k], ms[k]) { v.push(o(K::Sell, Ty::Stop, op[k] - so_, "Sell Entry")); }
                if pos.mp == -1 {
                    v.push(o(K::Xs, Ty::Stop, highest(h, k, ch), "Short Exit"));
                    if k - pos.ei < tight && c[k] < pos.ep { v.push(o(K::Xs, Ty::Stop, h[k], "Tight Channel")); }
                    if c[k] < pos.ep - pk * a[k] { v.push(o(K::Xs, Ty::Open, 0., "Short Profit")); }
                }
                v
            })
        }
        "lebfut_sidewinder" => {
            let adx = dmi_ts(x, u("adx_n"), start).2;
            let (r, a) = (rsi(c, u("rsi_n")), atr(x, u("atr_n")));
            let (drop, rl, rs, lk1, ch, lk2, lock, sk) = (g("adx_drop"), g("rsi_l"), g("rsi_s"), g("lk1"), u("ch"), g("lk2"), g("lock"), g("sk"));
            sim(x, start, to, mm, tick, |k, pos| {
                let mut v = vec![];
                if k < 1 { return v; }
                let c1 = lt(adx[k], adx[k - 1] - drop);
                if pos.mp != 1 && c1 && lt(r[k], rl) { v.push(o(K::Buy, Ty::Stop, h[k], "Buy Entry")); }
                if pos.mp == 1 {
                    if c[k] > pos.ep + lk1 * a[k] { v.push(o(K::Xl, Ty::Stop, lowest(l, k, ch), "Long Profit")); }
                    if c[k] > pos.ep + lk2 * a[k] { v.push(o(K::Xl, Ty::Stop, pos.ep + lock * a[k], "Small Long Profit")); }
                }
                if pos.mp != -1 && c1 && gt(r[k], rs) { v.push(o(K::Sell, Ty::Stop, l[k], "Sell Entry")); }
                if pos.mp == -1 && c[k] < pos.ep - sk * a[k] { v.push(o(K::Xs, Ty::Stop, h[k], "Short Profit")); }
                v
            })
        }
        "lebfut_bigdipper" => {
            let (ml, mm_, r) = (sma(c, u("ma_l")), sma(c, u("ma_m")), rsi(c, u("rsi_n")));
            let (lb, drop, off, p1, ch, p2, be) = (u("rsi_lb"), g("rsi_drop"), g("off"), g("p1"), u("ch"), g("p2"), g("be"));
            sim(x, start, to, mm, tick, |k, pos| {
                let mut v = vec![];
                if k < lb { return v; }
                if pos.mp == 0 && gt(c[k], ml[k]) && gt(c[k], mm_[k]) && lt(r[k], r[k - lb] - drop) { v.push(o(K::Buy, Ty::StopOpen, off, "Entry")); }
                if pos.mp == 1 {
                    if c[k] > pos.ep + p1 { v.push(o(K::Xl, Ty::Stop, lowest(l, k, ch), "10-day low")); }
                    if c[k] > pos.ep + p2 { v.push(o(K::Xl, Ty::Stop, pos.ep + be, "Breakeven")); }
                }
                v
            })
        }
        "lebfut_firstsword" => {
            let (ml, ms, a) = (sma(c, u("ma_l")), sma(c, u("ma_s")), atr(x, u("atr_n")));
            let (dip, ch, k1, s1, k2, s2, chand) = (g("dip"), u("ch"), g("k1"), g("s1"), g("k2"), g("s2"), g("chand"));
            let mut e = Ext::new();
            sim(x, start, to, mm, tick, |k, pos| {
                e = e.step(pos.mp, h[k], l[k], c[k]);
                let mut v = vec![];
                let at = a[k];
                if pos.mp != 1 && lt(c[k], ms[k]) && gt(c[k], ml[k]) && lt(c[k], highest(c, k, 2) - dip * at) { v.push(o(K::Buy, Ty::Stop, h[k], "Long Entry")); }
                if pos.mp == 1 {
                    let ep = pos.ep;
                    v.push(o(K::Xl, Ty::Stop, lowest(l, k, ch), "Channel long"));
                    if c[k] > ep + k1 * at { v.push(o(K::Xl, Ty::Stop, ep + s1 * at, "Med Long Profit")); }
                    if c[k] > ep + k2 * at { v.push(o(K::Xl, Ty::Stop, ep + s2 * at, "Big Long Profit")); }
                    v.push(o(K::Xl, Ty::Stop, e.mth - chand * at, "Chandelier long"));
                }
                if pos.mp != -1 && gt(c[k], ms[k]) && lt(c[k], ml[k]) && gt(c[k], lowest(c, k, 2) + dip * at) { v.push(o(K::Sell, Ty::Stop, l[k], "Short entry")); }
                if pos.mp == -1 {
                    let ep = pos.ep;
                    v.push(o(K::Xs, Ty::Stop, highest(h, k, ch), "channel short"));
                    if c[k] < ep - k1 * at { v.push(o(K::Xs, Ty::Stop, ep - s1 * at, "Med Short Profit")); }
                    if c[k] < ep - k2 * at { v.push(o(K::Xs, Ty::Stop, ep - s2 * at, "Big Short Profit")); }
                    v.push(o(K::Xs, Ty::Stop, e.mnl + chand * at, "Chandelier sh"));
                }
                v
            })
        }
        "lebfut_millennium" => {
            let (ma, a, a5) = (sma(c, u("ma_n")), atr(x, u("atr_n")), atr(x, u("yy_n")));
            let (n3, llo, lhi, slo, shi, pt, cha, ch) = (u("hh_n"), g("l_lo"), g("l_hi"), g("s_lo"), g("s_hi"), g("pt"), g("ch_at"), u("ch"));
            let (yl, ys, bea, be, tm) = (g("yy_l"), g("yy_s"), g("be_at"), g("be"), u("time"));
            let mut e = Ext::new();
            sim(x, start, to, None, tick, |k, pos| {
                e = e.step(pos.mp, h[k], l[k], c[k]);
                let mut v = vec![];
                let at = a[k];
                let (hh, ll) = (highest(h, k, n3), lowest(l, k, n3));
                if pos.mp != 1 && gt(c[k], ma[k]) && lt(c[k], hh - llo * at) && gt(c[k], hh - lhi * at) { v.push(o(K::Buy, Ty::Stop, h[k], "Millennium_Long")); }
                if pos.mp == 1 {
                    let ep = pos.ep;
                    if c[k] > ep + pt * at { v.push(o(K::Xl, Ty::Open, 0., "Millennium_Profit")); }
                    if e.mtc > ep + cha * at { v.push(o(K::Xl, Ty::Stop, lowest(l, k, ch), "Millennium Long")); }
                    v.push(o(K::Xl, Ty::Stop, c[k] - yl * a5[k], "Mill_YoYoLong"));
                    if e.mtc > ep + bea { v.push(o(K::Xl, Ty::Stop, ep + be, "Mill_Breakeven")); }
                    if k - pos.ei >= tm && c[k] < ep { v.push(o(K::Xl, Ty::Open, 0., "Mill_TimeLong")); }
                }
                if pos.mp != -1 && lt(c[k], ma[k]) && gt(c[k], ll + slo * at) && lt(c[k], ll + shi * at) { v.push(o(K::Sell, Ty::Stop, l[k], "Mill_ShortEntryL")); }
                if pos.mp == -1 {
                    let ep = pos.ep;
                    if c[k] < ep - pt * at { v.push(o(K::Xs, Ty::Open, 0., "Mill_ProfitTarget_S")); }
                    if e.mnc < ep - cha * at { v.push(o(K::Xs, Ty::Stop, highest(h, k, ch), "Mill_ChannelShort")); }
                    v.push(o(K::Xs, Ty::Stop, c[k] + ys * a5[k], "Mill_YoYoShort"));
                    if e.mnc < ep - bea { v.push(o(K::Xs, Ty::Stop, ep - be, "Mill_BreakevenShort")); }
                    if k - pos.ei >= tm && c[k] > ep { v.push(o(K::Xs, Ty::Open, 0., "Mill_TimeShort")); }
                }
                v
            })
        }
        "lebfut_empire" => {
            let a = atr(x, u("atr_n"));
            let (d1, d2): (Vec<f64>, Vec<f64>) = {
                let (f1, s1, f2, s2) = (xaverage(c, u("lf")), xaverage(c, u("ls")), xaverage(c, u("sf")), xaverage(c, u("ss")));
                ((0..c.len()).map(|i| f1[i] - s1[i]).collect(), (0..c.len()).map(|i| f2[i] - s2[i]).collect())
            };
            let adx = dmi_ts(x, 14, start).2;
            let (amin, pt) = (g("adx_min"), g("point"));
            sim(x, start, to, None, tick, |k, pos| {
                let mut v = vec![];
                if pos.mp == 0 && d1[k] > 0. { v.push(o(K::Buy, Ty::Stop, highest(h, k, 4), "Empire_Long")); }
                if pos.mp == 0 && gt(adx[k], amin) && d2[k] < 0. { v.push(o(K::Sell, Ty::Stop, lowest(l, k, 3), "Empire_Short")); }
                let bse = if pos.mp != 0 { k - pos.ei } else { 0 };
                if pos.mp == 1 && bse > 0 && lowest(l, k, 15) + 8. * a[k] < highest(h, k, 2) { v.push(o(K::Xl, Ty::Stop, c[k] - 4. * pt, "Empire_L Exit")); }
                if pos.mp == -1 && bse > 0 && highest(h, k, 15) - 7. * a[k] > lowest(l, k, 2) { v.push(o(K::Xs, Ty::Stop, c[k] + 4. * pt, "Empire_S Exit")); }
                if pos.mp == 1 { v.push(o(K::Xl, Ty::Stop, lowest(l, k, 21) + 7. * pt, "Empire_L Trail")); }
                if pos.mp == -1 { v.push(o(K::Xs, Ty::Stop, highest(h, k, 21) - 7. * pt, "Empire_S Trail")); }
                v
            })
        }
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;

    fn load(name: &str, a: &str, b: &str) -> Option<Vec<Row>> {
        let dir = std::env::var("LEBFUT_DATA").unwrap_or("/home/server/.local/share/strategy-vault/work/lebfut/data".into());
        let txt = std::fs::read_to_string(format!("{dir}/{name}")).ok()?;
        Some(txt.lines().skip(1).filter_map(|l| {
            let f: Vec<&str> = l.split(',').collect();
            let n = |k: usize| f[k].parse::<f64>().unwrap();
            (f[0] >= a && f[0] <= b).then(|| Row { ticker: "F".into(), date: f[0].into(), open: n(1), high: n(2), low: n(3), close: n(4), volume: None })
        }).collect())
    }
    /// (closed trades, net USD after $100 per trade, winners) on a manual's own data, default parameters.
    fn book(id: &str, file: &str, a: &str, b: &str, pv: f64, extra: &[(&str, f64)]) -> Option<(usize, f64, usize)> {
        let r = load(file, a, b)?;
        let x = Ctx::new(&r, &r, &[]);
        let p: Params = extra.iter().map(|(k, v)| (k.to_string(), *v)).collect();
        let ts = run(id, &x, 0, r.len() - 1, &p)?;
        let v: Vec<f64> = ts.iter().filter(|t| !t.open).map(|t| (t.exit_px - t.legs[0].px) * t.dir * pv - 100.).collect();
        Some((v.len(), (v.iter().sum::<f64>() * 100.).round() / 100., v.iter().filter(|x| **x > 0.).count()))
    }

    #[test]
    fn exact_reproductions() {
        // book tables / printed lists: (trades, net $, winners)
        for (id, f, a, b, pv, want) in [
            ("lebfut_strategic", "sf_strategic.csv", "1988-10-17", "1999-09-30", 1250., (54, 125300., 42)),
            ("lebfut_phoenix", "us.csv", "1988-01-04", "1998-04-22", 1000., (31, 84181.25, 21)),
            ("lebfut_serendipity", "ua_seren.csv", "1988-01-04", "1999-04-30", 1000., (140, 67031.25, 95)),
            ("lebfut_sidewinder", "us.csv", "1988-01-04", "1998-04-22", 1000., (96, 95681.25, 75)),
            ("lebfut_bigdipper", "us.csv", "1987-01-02", "1998-01-27", 1000., (43, 73637.5, 38)),
        ] {
            let Some(got) = book(id, f, a, b, pv, &[]) else { return };
            assert_eq!(got, want, "{id}");
        }
    }

    #[test]
    fn near_reproductions() {
        // Roughneck: 58/59 identical (first book trade starts later; mbb 58 gives all 59 = $43,100)
        let Some(got) = book("lebfut_roughneck", "cl_rough.csv", "1988-10-17", "1998-12-31", 1000., &[("mbb", 58.)]) else { return };
        assert_eq!(got, (59, 43100., 46));
        // First Sword: all 85 printed trades plus one extra (12/19/94), from an exact close = SMA(5) tie on 12/16/94
        let got = book("lebfut_firstsword", "jy_fsy.csv", "1988-01-04", "1998-12-17", 1250., &[]).unwrap();
        assert_eq!(got, (86, 115800., 47));
        // Wildcat with the printed list's next-close profit fills: 38 trades (26 identical; 1984-88 data differs)
        let got = book("lebfut_wildcat", "cl_wildcat.csv", "1984-01-03", "1998-04-22", 1000., &[("pfill_close", 1.)]).unwrap();
        assert_eq!(got.0, 40);
    }

    /// Writes every system's trades on its manual's data to $LEBFUT_DUMP/<id>.csv (for verify/rust_vs_oracle.py).
    #[test]
    #[ignore]
    fn dump_trades() {
        let Ok(dir) = std::env::var("LEBFUT_DUMP") else { return };
        for (id, f, a, b) in [
            ("lebfut_strategic", "sf_strategic.csv", "1988-10-17", "1999-09-30"), ("lebfut_roughneck", "cl_rough.csv", "1988-10-17", "1998-12-31"),
            ("lebfut_wildcat", "cl_wildcat.csv", "1984-01-03", "1998-04-22"), ("lebfut_phoenix", "us.csv", "1988-01-04", "1998-04-22"),
            ("lebfut_serendipity", "ua_seren.csv", "1988-01-04", "1999-04-30"), ("lebfut_sidewinder", "us.csv", "1988-01-04", "1998-04-22"),
            ("lebfut_bigdipper", "us.csv", "1987-01-02", "1998-01-27"), ("lebfut_firstsword", "jy_fsy.csv", "1988-01-04", "1998-12-17"),
            ("lebfut_millennium", "us.csv", "1988-01-04", "1998-04-22"), ("lebfut_empire", "us.csv", "1988-01-04", "1998-04-22"),
        ] {
            let r = load(f, a, b).unwrap();
            let x = Ctx::new(&r, &r, &[]);
            let ts = run(id, &x, 0, r.len() - 1, &Params::new()).unwrap();
            let s: String = ts.iter().map(|t| format!("{},{},{},{},{},{},{}\n", t.dir, r[t.legs[0].i].date, t.legs[0].px, r[t.exit_i].date, t.exit_px, t.reason, t.open)).collect();
            std::fs::write(format!("{dir}/{id}.csv"), s).unwrap();
        }
    }

    #[test]
    fn stop_from_next_open() {
        // Big Dipper style order: buy stop at the next bar's open + 0.25
        let rows: Vec<Row> = [(100., 100.5, 99.5, 100.), (101., 101.5, 100.8, 101.2), (101., 101.1, 100.9, 101.)].iter().enumerate()
            .map(|(i, &(o, h, l, c))| Row { ticker: "T".into(), date: format!("{i:05}"), open: o, high: h, low: l, close: c, volume: None }).collect();
        let x = Ctx::new(&rows, &rows, &[]);
        let ts = sim(&x, 0, 2, None, 0., |k, _| if k == 0 { vec![o(K::Buy, Ty::StopOpen, 0.25, "t")] } else { vec![o(K::Xl, Ty::Close, 0., "c")] });
        assert_eq!((ts[0].legs[0].i, ts[0].legs[0].px, ts[0].exit_i, ts[0].exit_px), (1, 101.25, 2, 101.));
    }
}
