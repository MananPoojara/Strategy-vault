//! Chuck LeBeau's System Traders Club (traderclub.com) stock-index systems, 1999-2002:
//! "Prudent" S&P v1.2 (LeBeau & Bicer), "Remarkable" S&P v1.3 (LeBeau & Bicer), "Piranha" S&P v1.0
//! (LeBeau & Elden) and its 2002 "Modified Piranha" (LeBeau, Elden & Arnheim), and "Mack" the NYFE v1.0
//! (LeBeau & Elden). Each manual prints its TradeStation code; the code is followed literally (it wins
//! over the prose where they differ). Sources, oracle and verification: library/lebsp/.
//!
//! Engine: TradeStation 4 daily backtest. The code runs at each bar's close and places stop / limit /
//! next-open orders for the next bar; a stop or limit fills at its price or at the open if the open is
//! already through it; several orders on one bar fill in bar-path order (open, the nearer of high/low,
//! the other, close); an opposite entry reverses; the money-management stop (a TradeStation system
//! property, in index points) is live from the fill on, including the entry bar.
//! On each manual's own data file this reproduces the printed trade-by-trade reports (see tests).
use super::lebtc::{atr_sma, dmi3};
use crate::connors::{sma, Claim, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

const BOOK: &str = "lebsp";

pub fn catalogue() -> Vec<StrategyDef> {
    let make = |id, name, group, dir, status, needs_series, rules, notes, claim| StrategyDef {
        id, name, group, session: 0, dir, status, needs_volume: false, needs_vix: false,
        needs_series, book: BOOK, rules, notes, claim,
    };
    vec![
        make("lebsp_prudent", "Prudent S&P System v1.2", "Prudent S&P v1.2 manual (1999)", "Long", Status::Live, &[],
            &["F|\"The range yesterday must be less than 1.25 times the average true range of the last 40 days.\" (code: AvgTrueRange(1) < 1.25 * AvgTrueRange(40))",
              "F|\"The 14 day ADX must be greater than 12.\"",
              "E|+DI(14) > -DI(14): \"enter only after the 5-day RSI drops below 40\"; buy stop at \"the highest high of the last three days\" or \"the previous close plus 0.4 of the Average True Range of the last 3 days\", whichever is closer (code: RoundUpSP(Close + Round(AvgTrueRange(3),2)*0.4); if Highest(High,3) is lower it is used)",
              "E|+DI(14) <= -DI(14): RSI(5) below 50; buy stop at the 3-day high or close + 0.6 ATR(3), whichever is closer",
              "X|Profit targets (limit, from the entry price, re-computed daily with that day's ATR(3) and ADX): DMI up and ADX <= 16: +1.50 ATR; ADX 16-30: +4 ATR; ADX > 30: +1 ATR; DMI down: +1.50 ATR (code: RoundUpSP(..., TRUE) = up to the next .50/1.00)",
              "X|\"If the trade closes with a profit of one Average True Range or more we enter a protective stop 200 points above our entry\" (code: highest close of the trade >= entry + 1 ATR -> sell stop RoundUpSP(EntryPrice + 2.0))",
              "X|\"If the trade closes with a profit of three Average True Ranges or more we will enter a protective stop two Average True Ranges above our entry price.\"",
              "X|\"we will exit on a stop when our loss reaches $1500\" = 6.00 S&P points at the $250 point value the manual uses throughout"],
            "Manual 'Prudent S&P Trading System v1.2' (Prudent12.doc) + its TradeStation code; code wins where prose differs (ADX <= 16 for target 1). Long only, S&P futures (day session, back-adjusted), MaxBarsBack 50, $150 per trade. Run here on SPX / NIFTY 50 (instrument swap). Reproduces 146/147 printed trades on the manual's own Sp.lng (the 1 difference is the first trade, Feb 1989, 85 bars into the data: ADX seeding). RoundUpSP rounds order prices up to 0.1 (p.rnd=0 turns it off). Dollar amounts are converted to the manual's own index points (dollars / point value) and used unchanged on whichever index is selected (instrument swap only, no rescaling), so on NIFTY 50 or a 2020s SPX these fixed-point stops are far tighter in percent than in the 1990s.",
            Some(Claim { trades: "147", win: "67%", avg: "$905.02 (net of $150)", hold: "4 bars winners / 3 bars losers", period: "10/17/88-12/31/98, S&P futures, $250/pt" })),
        make("lebsp_remarkable", "Remarkable S&P Trading System v1.3", "Remarkable S&P v1.3 manual (1999)", "Long/Short", Status::Live, &[],
            &["F|Long setup: \"The bottom band is placed 1.25 Average True Ranges (we use 10 days to calculate the ATR) below the simple 5 day moving average of closing prices. When the low of a daily bar penetrates below the bottom band we are set to enter sometime within the next five bars\" (code: BrkFlags/BrkDays counter, 5 bars)",
              "F|Long filter: \"the 10 bar RSI of the 25 day moving average is greater than 5\" (code: RSI(Average(Close,25),10) >= 5)",
              "E|Long: \"go long tomorrow on a stop placed 1.75 ATRs above the lowest low of the last five days\" (code: RoundUpSP(Lowest(Low,5) + ATR*1.75))",
              "F|Short setup: high above the top band = 5-day SMA + 1.50 ATR(10); same 5-bar window. No RSI filter on shorts.",
              "E|Short: sell stop at the higher of RoundDownSP(setup-bar low - today's range*0.25) and RoundDownSP(Highest(High,5) - ATR*1.25) (code; prose: \"today's low\")",
              "X|Long targets (limit): ADX(14) <= 20: entry + 2.5 ATR; 20-30: + 3.5 ATR; > 30: + 0.5 ATR",
              "X|Long: once a close is above entry + 1.5 ATR, sell stop at entry + $1,000 (= 4.00 points at $250)",
              "X|Short targets (limit): ADX <= 20: entry - 3.0 ATR; > 20: entry - 1.0 ATR. Once a close is below entry - 2.0 ATR, buy stop at entry - $500 (= 2.00 points)",
              "X|\"exit on a stop when our loss reaches $4500\" (code comment: \"an 18 point stop\"). An opposite entry reverses the position."],
            "Manual 'Remarkable S&P Trading System v1.3' (remark13.doc) + TradeStation code; code wins (e.g. the short trigger uses the setup bar's low minus 0.25 x range; the 5-bar window counts from the first penetration). ATR = Round(AvgTrueRange(10),2), ADX rounded to 2 decimals as in the code. The MaxTradeClose/MinTradeClose variables are reset only while flat, as written. RoundDownSP's body is not printed: read as down to 0.1. Reproduces 111/114 printed trades on the manual's SP.LNG (3 differ by 0.1 point in one order price). Dollar amounts are converted to the manual's own index points (dollars / point value) and used unchanged on whichever index is selected (instrument swap only, no rescaling), so on NIFTY 50 or a 2020s SPX these fixed-point stops are far tighter in percent than in the 1990s.",
            Some(Claim { trades: "114 (70 long, 44 short)", win: "84%", avg: "$1,818.97 (net of $150)", hold: "9 bars winners / 11 bars losers", period: "10/17/88-12/31/98, S&P futures, $250/pt" })),
        make("lebsp_piranha", "Piranha S&P Trading System v1.0", "Piranha S&P v1.0 manual (1999)", "Long/Short",
            Status::NeedsData("T-bond futures closes (Data2: 30-year Treasury bond futures, continuous back-adjusted, day session) for the 120-day bond trend filter; the vault has no bond futures. Code reads series TBOND_FUT_CLOSE."),
            &["TBOND_FUT_CLOSE"],
            &["F|Long: bond futures close above their 120-day MA; ADX(14) > 15; RSI(5) < 65",
              "E|Long: \"buy tomorrow on a stop order at today's close plus 0.25 Average True Ranges\" (ATR 30 days)",
              "X|Long: \"If the highest close of the trade is 6 points ($1500) or more above our entry we will exit on a stop placed at the highest high of the trade minus 6 points\" (code: MaxTradeClose > EntryPrice + 6 -> MaxTradeHigh[1] - 6 stop)",
              "X|Long: \"If the close today is less than the entry price then we will place a sell stop at today's close minus 12 points\"",
              "F|Short: bond close below its 120-day MA; ADX(14) > 15; RSI(5) > 45",
              "E|Short: \"sell short tomorrow on a stop order at the highest high of the last two days minus 1.8 Average True Ranges\"",
              "X|Short: lowest close 10 points below entry -> buy stop at the lowest low of the trade (through yesterday) + 8; close above entry -> buy stop at close + 12",
              "X|\"We will exit on a stop when our loss reaches $10,000\" (= 40 points). An opposite entry reverses."],
            "Manual 'Piranha S&P Trading System v1.0' (piranha.doc). Needs the T-bond futures series (none in the vault). With the manual's own SP.LNG + UA.LNG the code reproduces all 156 printed trades (06/15/88-10/29/99). The bond 120-day average is taken over the bond's own bars.",
            Some(Claim { trades: "156 (127 long, 29 short)", win: "76%", avg: "$1,955.37 (net of $150)", hold: "12 bars winners / 9 bars losers", period: "6/15/88-10/29/99, S&P futures, $250/pt" })),
        make("lebsp_piranha2", "Modified Piranha S&P (2002)", "Piranha 'New' version (LeBeau, Elden, Arnheim, 15 Jan 2002)", "Long", Status::Live, &[],
            &["F|\"On an uptrend (C > C[12])\"; \"Where ADX confirms trend\" (code: Adx(14) > 15); \"RSI must be within 35 and 65\" (code: RSI(C,4) < 65 and > 35)",
              "E|\"Enter on close * .25 of volatility range (ATR(30)\" (code: Buy Tomorrow Close + .25 * AvgTrueRange(30) Stop)",
              "X|\"When profitable, Exit on 6 point retracement\" (code: if MaxTradeClose > EntryPrice + 6 then ExitLong tomorrow MaxTradeClose[1] - 6 Stop)",
              "X|\"When in a loss, exit on 1 day 17 point retracement\" (code: if Close < EntryPrice then ExitLong Close - 17 Stop)"],
            "Chuck LeBeau's note and code sent with the Piranha files (Piranha New.rtf, PIRANHA020115.ela): bonds dropped, long only, \"shortened some of the parameters\". No results are printed. The new code has no money-management stop and none is added. Points are S&P points. Dollar amounts are converted to the manual's own index points (dollars / point value) and used unchanged on whichever index is selected (instrument swap only, no rescaling), so on NIFTY 50 or a 2020s SPX these fixed-point stops are far tighter in percent than in the 1990s.",
            None),
        make("lebsp_mack", "Mack the NYFE Trading System v1.0", "Mack NYFE v1.0 manual (1999)", "Long/Short", Status::Live, &[],
            &["F|Long: average(L,3) < average(H,20) - 2 * ATR(12) and lowest(L,20) > highest(H,20) - 5 * ATR(12) (code; prose: \"twenty day average of the highs must be within five average true ranges of the twenty day average of the lows\")",
              "E|Long: buy stop at lowest(L,3) + 1.75 * ATR(7) (code; prose says 12-day ATR)",
              "X|Long: sell stop at the lowest low of 16 days; after a close > entry + 1.5 ATR(12): lowest low of 12 days; when the trade's highest high reaches entry + 2 / 5 / 10 / 12 ATR: stop at entry + 1 / 4 / 9 / 12 ATR",
              "F|Short: average(H,6) > average(L,20) + 2 * ATR(12)",
              "E|Short: sell stop at highest(H,3) - 3.5 * ATR(7)",
              "X|Short: buy stop at the highest high of 16 days; after a close < entry - 1.5 ATR: 12 days; trade's lowest low <= entry - 2.5 ATR: buy stop at entry - 1 ATR; close < entry - 3 ATR: exit at the next open",
              "X|\"We will exit on a stop when our loss reaches $2500\" = 5.00 NYFE points at $500/point. An opposite entry reverses."],
            "Manual 'Mack the NYFE Trading System v1.0' (Mack.doc) + TradeStation code; code wins where the prose differs (the two noted above). NYSE Composite futures, run here on SPX / NIFTY 50 (instrument swap). Reproduces all 45 printed trades on the manual's own YX.LNG (10/17/88-12/31/98, MaxBarsBack 50). Dollar amounts are converted to the manual's own index points (dollars / point value) and used unchanged on whichever index is selected (instrument swap only, no rescaling), so on NIFTY 50 or a 2020s SPX these fixed-point stops are far tighter in percent than in the 1990s.",
            Some(Claim { trades: "45 (36 long, 9 short)", win: "69%", avg: "$3,855 (net of $150)", hold: "16 bars winners / 4 bars losers", period: "10/17/88-12/31/98, NYFE futures, $500/pt" })),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "lebsp_prudent" => ("lebsp_index", "LeBeau traderclub index systems", "Prudent S&P"),
        "lebsp_remarkable" => ("lebsp_index", "LeBeau traderclub index systems", "Remarkable S&P"),
        "lebsp_piranha" => ("lebsp_piranha", "Piranha S&P", "v1.0 (bond filter)"),
        "lebsp_piranha2" => ("lebsp_piranha", "Piranha S&P", "Modified 2002 (no bonds)"),
        "lebsp_mack" => ("lebsp_index", "LeBeau traderclub index systems", "Mack NYFE"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let tick = p("tick", "Round stop/limit prices to this tick (0 = exact prices; the manuals' futures data used 0.10 S&P / 0.05 NYFE)", 0., 0., 1., 0.01);
    let mut v = match id {
        "lebsp_prudent" => vec![
            p("vol_mult", "Volatility filter: yesterday's range below x ATR(40)", 1.25, 0.5, 3., 0.05), p("vol_n", "Volatility filter ATR length", 40., 10., 100., 1.),
            p("adx_min", "ADX(14) above", 12., 0., 30., 1.), p("rsi_n", "RSI length", 5., 2., 14., 1.),
            p("rsi_up", "RSI below (DMI up)", 40., 10., 70., 1.), p("rsi_dn", "RSI below (DMI down)", 50., 10., 80., 1.),
            p("atr_n", "ATR length for entries and targets", 3., 2., 20., 1.), p("hh_n", "Entry: highest high of N days", 3., 1., 10., 1.),
            p("k_up", "Entry: close + x ATR (DMI up)", 0.4, 0., 2., 0.05), p("k_dn", "Entry: close + x ATR (DMI down)", 0.6, 0., 2., 0.05),
            p("t1", "Target 1 (DMI up, ADX <= 16) in ATRs", 1.5, 0.25, 6., 0.25), p("t2", "Target 2 (DMI up, ADX 16-30)", 4., 0.25, 8., 0.25),
            p("t3", "Target 3 (DMI up, ADX > 30)", 1., 0.25, 6., 0.25), p("t4", "Target 4 (DMI down)", 1.5, 0.25, 6., 0.25),
            p("adx_lo", "Target ADX band low", 16., 5., 40., 1.), p("adx_hi", "Target ADX band high", 30., 10., 60., 1.),
            p("minprof2", "Small lock after a close this many ATRs in profit", 1., 0.25, 5., 0.25), p("lock", "Small lock: stop this many points above entry (\"200 points\")", 2., 0., 20., 0.5),
            p("minprof1", "Big lock after a close this many ATRs in profit", 3., 1., 8., 0.25), p("trail1", "Big lock: stop this many ATRs above entry", 2., 0., 6., 0.25),
            p("mm", "Money stop in points ($1,500 at $250/pt)", 6., 1., 100., 1.), p("rnd", "1 = RoundUpSP order rounding (code); 0 = exact", 1., 0., 1., 1.),
        ],
        "lebsp_remarkable" => vec![
            p("ma_n", "Band centre: SMA of closes", 5., 2., 20., 1.), p("atr_n", "ATR length", 10., 3., 30., 1.),
            p("bot_k", "Lower band: ATRs below the SMA", 1.25, 0.25, 4., 0.05), p("top_k", "Upper band: ATRs above the SMA", 1.5, 0.25, 4., 0.05),
            p("win", "Entry window after a band penetration (bars)", 5., 1., 15., 1.),
            p("rsi_ma", "Long filter: SMA length for the RSI", 25., 5., 60., 1.), p("rsi_n", "Long filter: RSI length", 10., 2., 30., 1.), p("rsi_min", "Long filter: RSI of the SMA at least", 5., 0., 50., 1.),
            p("long_k", "Long entry: ATRs above the 5-day low", 1.75, 0.25, 4., 0.05), p("ll_n", "Long entry: lowest low of N days", 5., 1., 15., 1.),
            p("short_k", "Short entry: ATRs below the 5-day high", 1.25, 0.25, 4., 0.05), p("range_k", "Short entry: fraction of today's range below the setup low", 0.25, 0., 1., 0.05),
            p("lt1", "Long target, ADX <= 20 (ATRs)", 2.5, 0.25, 8., 0.25), p("lt2", "Long target, ADX 20-30", 3.5, 0.25, 8., 0.25), p("lt3", "Long target, ADX > 30", 0.5, 0.25, 8., 0.25),
            p("st1", "Short target, ADX <= 20", 3., 0.25, 8., 0.25), p("st2", "Short target, ADX > 20", 1., 0.25, 8., 0.25),
            p("adx_a", "Target ADX threshold 1", 20., 5., 40., 1.), p("adx_b", "Target ADX threshold 2 (longs)", 30., 10., 60., 1.),
            p("pl_atr", "Long lock after a close this many ATRs up", 1.5, 0.25, 5., 0.25), p("pl_pts", "Long lock: points above entry ($1,000)", 4., 0., 20., 0.5),
            p("ps_atr", "Short lock after a close this many ATRs down", 2., 0.25, 5., 0.25), p("ps_pts", "Short lock: points below entry ($500)", 2., 0., 20., 0.5),
            p("mm", "Money stop in points ($4,500 = 18 points)", 18., 1., 200., 1.), p("rnd", "1 = RoundUpSP / RoundDownSP order rounding (code); 0 = exact", 1., 0., 1., 1.),
        ],
        "lebsp_piranha" => vec![
            p("bond_n", "Bond futures MA length", 120., 20., 250., 5.), p("adx_min", "ADX(14) above", 15., 0., 40., 1.), p("rsi_n", "RSI length", 5., 2., 14., 1.),
            p("rsi_l", "Long: RSI below", 65., 30., 90., 1.), p("rsi_s", "Short: RSI above", 45., 10., 70., 1.), p("atr_n", "ATR length", 30., 5., 60., 1.),
            p("k_l", "Long entry: close + x ATR", 0.25, 0., 2., 0.05), p("k_s", "Short entry: 2-day high - x ATR", 1.8, 0., 4., 0.05),
            p("lock_at", "Long lock after highest close this many points up", 6., 0., 50., 1.), p("lock", "Long lock: points below the trade's high", 6., 0., 50., 1.), p("loss", "Long loss stop: points below the close", 12., 1., 60., 1.),
            p("s_lock_at", "Short lock after lowest close this many points down", 10., 0., 50., 1.), p("s_lock", "Short lock: points above the trade's low", 8., 0., 50., 1.), p("s_loss", "Short loss stop: points above the close", 12., 1., 60., 1.),
            p("mm", "Money stop in points ($10,000 = 40 points)", 40., 1., 200., 1.),
        ],
        "lebsp_piranha2" => vec![
            p("lkbk", "Uptrend: close above the close N bars ago", 12., 2., 50., 1.), p("rsi_n", "RSI length", 4., 2., 14., 1.),
            p("rsi_hi", "RSI below", 65., 30., 90., 1.), p("rsi_lo", "RSI above", 35., 5., 60., 1.), p("adx_min", "ADX(14) above", 15., 0., 40., 1.),
            p("atr_n", "ATR length", 30., 5., 60., 1.), p("k", "Entry: close + x ATR", 0.25, 0., 2., 0.05),
            p("lock_at", "Lock after highest close this many points up", 6., 0., 50., 1.), p("lock", "Lock: points below the highest close (LXProfit)", 6., 0., 50., 1.),
            p("loss", "Loss stop: points below the close (LXMM)", 17., 1., 60., 1.),
        ],
        "lebsp_mack" => vec![
            p("atr_n", "ATR length (rules)", 12., 3., 40., 1.), p("atr_e", "ATR length (entry triggers)", 7., 2., 30., 1.),
            p("dip_k", "Long: 3-day avg low at least x ATR below the 20-day avg high", 2., 0.5, 5., 0.25), p("crash_k", "Long: 20-day low within x ATR of the 20-day high", 5., 1., 10., 0.25),
            p("entry_k", "Long entry: ATRs above the 3-day low", 1.75, 0.25, 5., 0.25), p("ch", "Long/short channel stop (days)", 16., 3., 40., 1.), p("ch2", "Tighter channel (days)", 12., 2., 40., 1.),
            p("prof_k", "Tighter channel after a close this many ATRs in profit", 1.5, 0.25, 5., 0.25),
            p("rally_k", "Short: 6-day avg high at least x ATR above the 20-day avg low", 2., 0.5, 5., 0.25), p("s_entry_k", "Short entry: ATRs below the 3-day high", 3.5, 0.5, 8., 0.25),
            p("s_exit_k", "Short: exit next open after a close this many ATRs in profit", 3., 0.5, 8., 0.25), p("s_be_at", "Short lock when the trade's low is x ATRs below entry", 2.5, 0.5, 8., 0.25),
            p("s_be", "Short lock: ATRs below entry", 1., 0., 5., 0.25), p("mm", "Money stop in points ($2,500 = 5 NYFE points)", 5., 1., 100., 1.),
        ],
        _ => return None,
    };
    v.push(tick);
    Some(v)
}

// ---------------------------------------------------------------- helpers

/// Wilder RSI that skips leading NaNs in the input (RSI of a moving average).
fn rsi_nan(v: &[f64], n: usize) -> Vec<f64> {
    let len = v.len();
    let (mut up, mut dn) = (vec![f64::NAN; len], vec![f64::NAN; len]);
    for i in 1..len {
        let d = v[i] - v[i - 1];
        if d.is_finite() {
            up[i] = d.max(0.);
            dn[i] = (-d).max(0.);
        }
    }
    let w = crate::connors::wilder;
    let (au, ad) = (w(&up, n), w(&dn, n));
    (0..len).map(|i| {
        if au[i].is_nan() || ad[i].is_nan() { f64::NAN } else if ad[i] == 0. { if au[i] == 0. { 50. } else { 100. } } else { 100. - 100. / (1. + au[i] / ad[i]) }
    }).collect()
}

fn lowest(v: &[f64], k: usize, n: usize) -> f64 { v[k + 1 - n.min(k + 1)..=k].iter().copied().fold(f64::INFINITY, f64::min) }
fn highest(v: &[f64], k: usize, n: usize) -> f64 { v[k + 1 - n.min(k + 1)..=k].iter().copied().fold(f64::NEG_INFINITY, f64::max) }
fn gt(a: f64, b: f64) -> bool { a > b } // NaN compares false
fn lt(a: f64, b: f64) -> bool { a < b }

const EPS: f64 = 1e-7;
/// Jack Bicer's RoundUpSP: up to 0.1; with `pad`, on to the next .50 / 1.00 (a whole number pads to .50, as coded).
fn round_up_sp(price: f64, pad: bool) -> f64 {
    let f = (price * 10. + EPS).floor();
    let mut x = f / 10.;
    if price * 10. - f > EPS { x += 0.1; }
    if pad {
        let i = (x + EPS).floor();
        x = i + if x - i <= 0.5 + EPS { 0.5 } else { 1.0 };
    }
    (x * 1e6).round() / 1e6
}
/// RoundDownSP (name in the Remarkable code; body not printed): down to 0.1.
fn round_down_sp(price: f64) -> f64 { ((price * 10. + EPS).floor() / 10. * 1e6).round() / 1e6 }

#[derive(Clone, Copy, PartialEq)]
enum K { Buy, Sell, Xl, Xs }
#[derive(Clone, Copy, PartialEq)]
enum Ty { Stop, Limit, Open }
#[derive(Clone, Copy)]
struct Ord { k: K, ty: Ty, px: f64, name: &'static str }
fn o(k: K, ty: Ty, px: f64, name: &'static str) -> Ord { Ord { k, ty, px, name } }

#[derive(Clone, Copy, Default)]
struct Pos { mp: i8, ep: f64 }

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

/// TradeStation-4 style daily order engine (see module docs). `code(k, pos)` runs at the close of bar k
/// and returns the orders for bar k + 1. `mm` = money stop in points (None = none).
fn sim(x: &Ctx, from: usize, to: usize, mm: Option<f64>, tick: f64, mut code: impl FnMut(usize, Pos) -> Vec<Ord>) -> Vec<Trade> {
    let (op, hi, lo, cl) = (&x.o, &x.h, &x.l, &x.c);
    let mut out = vec![];
    let mut pos = Pos::default();
    let mut cur: Option<Trade> = None;
    let mut orders: Vec<Ord> = vec![];
    for i in from..=to {
        let (oo, hh, ll, cc) = (op[i], hi[i], lo[i], cl[i]);
        // events: (t, seq, px, kind, name, generation the exit belongs to)
        let mut ev: Vec<(f64, usize, f64, K, &'static str, u32)> = vec![];
        for (s, od) in orders.iter().enumerate() {
            if od.ty == Ty::Open {
                ev.push((0., s, oo, od.k, od.name, 0));
                continue;
            }
            let rising = matches!((od.k, od.ty), (K::Buy, Ty::Stop) | (K::Xs, Ty::Stop) | (K::Xl, Ty::Limit) | (K::Sell, Ty::Limit));
            let mut px = od.px;
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
                    pos = Pos { mp: want, ep: px };
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

/// MaxTradeClose / MaxTradeHigh / MinTradeClose / MinTradeLow series functions as printed in the manuals.
#[derive(Clone, Copy)]
struct Ext { mtc: f64, mth: f64, mnc: f64, mnl: f64 }
impl Ext {
    fn new() -> Self { Ext { mtc: -999999., mth: -999999., mnc: 999999., mnl: 999999. } }
    fn step(&self, mp: i8, h: f64, l: f64, c: f64) -> Self {
        let mut e = Ext::new();
        if mp == 1 { e.mtc = self.mtc.max(c); e.mth = self.mth.max(h); }
        if mp == -1 { e.mnc = self.mnc.min(c); e.mnl = self.mnl.min(l); }
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
    let tick = g("tick");
    let (h, l, c) = (&x.h, &x.l, &x.c);
    Some(match id {
        "lebsp_prudent" => {
            let a = atr_sma(x, u("atr_n"));
            let (avol, a1) = (atr_sma(x, u("vol_n")), atr_sma(x, 1));
            let (pdi, mdi, adx) = dmi3(x, 14);
            let r = crate::connors::rsi(c, u("rsi_n"));
            let rnd = g("rnd") >= 0.5;
            let up = |v: f64, pad: bool| if rnd { round_up_sp(v, pad) } else { v };
            let (vm, amin, ru, rd, ku, kd, hn) = (g("vol_mult"), g("adx_min"), g("rsi_up"), g("rsi_dn"), g("k_up"), g("k_dn"), u("hh_n"));
            let (t1, t2, t3, t4, alo, ahi) = (g("t1"), g("t2"), g("t3"), g("t4"), g("adx_lo"), g("adx_hi"));
            let (mp1, tr1, mp2, lock) = (g("minprof1"), g("trail1"), g("minprof2"), g("lock"));
            let mut mtc = 0.0;
            sim(x, from.max(50), to, Some(g("mm")), tick, |k, pos| {
                let mut v = vec![];
                let at = (a[k] * 100.).round() / 100.;
                let (av, dmiup) = (adx[k], gt(pdi[k], mdi[k]));
                if pos.mp == 0 && lt(a1[k], vm * avol[k]) && gt(av, amin) {
                    mtc = 0.0;
                    if dmiup && lt(r[k], ru) {
                        v.push(o(K::Buy, Ty::Stop, up(c[k] + at * ku, false).min(highest(h, k, hn)), "DMI Up"));
                    }
                    if !dmiup && lt(r[k], rd) {
                        v.push(o(K::Buy, Ty::Stop, up(c[k] + at * kd, false).min(highest(h, k, hn)), "DMI Down"));
                    }
                }
                if pos.mp == 1 {
                    let ep = pos.ep;
                    if dmiup {
                        if av <= alo { v.push(o(K::Xl, Ty::Limit, up(ep + at * t1, true), "ATR target 1")); }
                        if av > alo && av <= ahi { v.push(o(K::Xl, Ty::Limit, up(ep + at * t2, true), "ATR target 2")); }
                        if av > ahi { v.push(o(K::Xl, Ty::Limit, up(ep + at * t3, true), "ATR target 3")); }
                    } else {
                        v.push(o(K::Xl, Ty::Limit, up(ep + at * t4, true), "ATR target 4"));
                    }
                    if c[k] > mtc { mtc = c[k]; }
                    let xp = if mtc >= ep + at * mp1 { up(ep + at * tr1, true) } else if mtc >= ep + at * mp2 { up(ep + lock, false) } else { 0. };
                    if xp != 0. { v.push(o(K::Xl, Ty::Stop, xp, "Lock profit")); }
                }
                v
            })
        }
        "lebsp_remarkable" => {
            let a = atr_sma(x, u("atr_n"));
            let adx = dmi3(x, 14).2;
            let mid = sma(c, u("ma_n"));
            let rma = rsi_nan(&sma(c, u("rsi_ma")), u("rsi_n"));
            let rnd = g("rnd") >= 0.5;
            let up = |v: f64| if rnd { round_up_sp(v, false) } else { v };
            let dn = |v: f64| if rnd { round_down_sp(v) } else { v };
            let (bk, tk, win, rmin, lk, lln, sk, rk) = (g("bot_k"), g("top_k"), g("win") as i32, g("rsi_min"), g("long_k"), u("ll_n"), g("short_k"), g("range_k"));
            let (lt1, lt2, lt3, st1, st2, aa, ab) = (g("lt1"), g("lt2"), g("lt3"), g("st1"), g("st2"), g("adx_a"), g("adx_b"));
            let (pla, plp, psa, psp) = (g("pl_atr"), g("pl_pts"), g("ps_atr"), g("ps_pts"));
            let (mut flags, mut days, mut bp, mut maxc, mut minc) = (0i32, 0i32, 0.0f64, 0.0f64, 99999.0f64);
            sim(x, from.max(50), to, Some(g("mm")), tick, |k, pos| {
                let mut v = vec![];
                if a[k].is_nan() || mid[k].is_nan() { return v; }
                let at = (a[k] * 100.).round() / 100.;
                let av = (adx[k] * 100.).round() / 100.;
                let (top, bot) = (mid[k] + a[k] * tk, mid[k] - a[k] * bk);
                if pos.mp == 0 { maxc = 0.; minc = 99999.; }
                if flags == 0 { days = 0; }
                if days >= win { flags = 0; days = 0; }
                if l[k] < bot { flags = 1; bp = h[k]; }
                if h[k] > top { flags = -1; bp = l[k]; }
                if pos.mp >= 0 && flags == -1 && days < win {
                    let v1 = dn(bp - (h[k] - l[k]) * rk);
                    let e = dn(highest(h, k, 5) - at * sk);
                    v.push(if v1 > e { o(K::Sell, Ty::Stop, v1, "Short (setup low)") } else { o(K::Sell, Ty::Stop, e, "Short (5-day high)") });
                }
                if pos.mp <= 0 && flags == 1 && days < win && rma[k] >= rmin {
                    v.push(o(K::Buy, Ty::Stop, up(lowest(l, k, lln) + at * lk), "Long (5-day low)"));
                }
                days += 1;
                if pos.mp == 1 {
                    flags = 0; days = 0;
                    if av <= aa { v.push(o(K::Xl, Ty::Limit, up(pos.ep + at * lt1), "ATR target (ADX <= 20)")); }
                    if av > aa && av <= ab { v.push(o(K::Xl, Ty::Limit, up(pos.ep + at * lt2), "ATR target (ADX 20-30)")); }
                    if av > ab { v.push(o(K::Xl, Ty::Limit, up(pos.ep + at * lt3), "ATR target (ADX > 30)")); }
                }
                if pos.mp == -1 {
                    flags = 0; days = 0;
                    if av <= aa { v.push(o(K::Xs, Ty::Limit, dn(pos.ep - at * st1), "ATR target (ADX <= 20)")); }
                    if av > aa { v.push(o(K::Xs, Ty::Limit, dn(pos.ep - at * st2), "ATR target (ADX > 20)")); }
                }
                if c[k] < minc { minc = c[k]; }
                if c[k] > maxc { maxc = c[k]; }
                if pos.mp == 1 && maxc > pos.ep + pla * at { v.push(o(K::Xl, Ty::Stop, pos.ep + plp, "Long profit protection")); }
                if pos.mp == -1 && minc < pos.ep - psa * at { v.push(o(K::Xs, Ty::Stop, pos.ep - psp, "Short profit protection")); }
                v
            })
        }
        "lebsp_piranha" => {
            let bond = x.series("TBOND_FUT_CLOSE")?;
            let bma = bond_ma(bond, u("bond_n"));
            let r = crate::connors::rsi(c, u("rsi_n"));
            let a = atr_sma(x, u("atr_n"));
            let adx = dmi3(x, 14).2;
            let (amin, rl, rs, kl, ks) = (g("adx_min"), g("rsi_l"), g("rsi_s"), g("k_l"), g("k_s"));
            let (la, lk, ls, sa, sk, ss) = (g("lock_at"), g("lock"), g("loss"), g("s_lock_at"), g("s_lock"), g("s_loss"));
            let mut e = Ext::new();
            sim(x, from.max(120), to, Some(g("mm")), tick, |k, pos| {
                let prev = e;
                e = e.step(pos.mp, h[k], l[k], c[k]);
                let mut v = vec![];
                let bc = last_valid(bond, k);
                let c5 = gt(adx[k], amin);
                if pos.mp != 1 && lt(r[k], rl) && lt(bma[k], bc) && c5 { v.push(o(K::Buy, Ty::Stop, c[k] + kl * a[k], "L Entry")); }
                if pos.mp == 1 {
                    if e.mtc > pos.ep + la { v.push(o(K::Xl, Ty::Stop, prev.mth - lk, "L Lock")); }
                    if c[k] < pos.ep { v.push(o(K::Xl, Ty::Stop, c[k] - ls, "L Loss")); }
                }
                if pos.mp != -1 && gt(r[k], rs) && gt(bma[k], bc) && c5 { v.push(o(K::Sell, Ty::Stop, highest(h, k, 2) - ks * a[k], "S Entry")); }
                if pos.mp == -1 {
                    if e.mnc < pos.ep - sa { v.push(o(K::Xs, Ty::Stop, prev.mnl + sk, "S Lock")); }
                    if c[k] > pos.ep { v.push(o(K::Xs, Ty::Stop, c[k] + ss, "S Loss")); }
                }
                v
            })
        }
        "lebsp_piranha2" => {
            let r = crate::connors::rsi(c, u("rsi_n"));
            let a = atr_sma(x, u("atr_n"));
            let adx = dmi3(x, 14).2;
            let (n, rhi, rlo, amin, kk, la, lk, ls) = (u("lkbk"), g("rsi_hi"), g("rsi_lo"), g("adx_min"), g("k"), g("lock_at"), g("lock"), g("loss"));
            let mut e = Ext::new();
            sim(x, from.max(50), to, None, tick, |k, pos| {
                let prev = e;
                e = e.step(pos.mp, h[k], l[k], c[k]);
                let mut v = vec![];
                if pos.mp != 1 && k >= n && lt(r[k], rhi) && c[k] > c[k - n] && gt(adx[k], amin) && gt(r[k], rlo) {
                    v.push(o(K::Buy, Ty::Stop, c[k] + kk * a[k], "L Entry"));
                }
                if pos.mp == 1 {
                    if e.mtc > pos.ep + la { v.push(o(K::Xl, Ty::Stop, prev.mtc - lk, "L Lock")); }
                    if c[k] < pos.ep { v.push(o(K::Xl, Ty::Stop, c[k] - ls, "L Loss")); }
                }
                v
            })
        }
        "lebsp_mack" => {
            let (a, a7) = (atr_sma(x, u("atr_n")), atr_sma(x, u("atr_e")));
            let (l3, h20, l20, h6) = (sma(l, 3), sma(h, 20), sma(l, 20), sma(h, 6));
            let (dk, ck, ek, ch, ch2, pk, rk, sek, sxk, sba, sb) =
                (g("dip_k"), g("crash_k"), g("entry_k"), u("ch"), u("ch2"), g("prof_k"), g("rally_k"), g("s_entry_k"), g("s_exit_k"), g("s_be_at"), g("s_be"));
            let mut e = Ext::new();
            sim(x, from.max(50), to, Some(g("mm")), tick, |k, pos| {
                e = e.step(pos.mp, h[k], l[k], c[k]);
                let mut v = vec![];
                let at = a[k];
                if at.is_nan() || h20[k].is_nan() || a7[k].is_nan() { return v; }
                let c1 = l3[k] < h20[k] - dk * at;
                let c2 = lowest(l, k, 20) > highest(h, k, 20) - ck * at;
                let c3 = h6[k] > l20[k] + rk * at;
                if pos.mp != 1 && c1 && c2 { v.push(o(K::Buy, Ty::Stop, lowest(l, k, 3) + ek * a7[k], "Long Entry")); }
                if pos.mp == 1 {
                    let ep = pos.ep;
                    v.push(o(K::Xl, Ty::Stop, lowest(l, k, ch), "Long Channel"));
                    if c[k] > ep + pk * at { v.push(o(K::Xl, Ty::Stop, lowest(l, k, ch2), "Long Profit 1.5")); }
                    for (lvl, stop, nm) in [(2., 1., "Long Breakeven"), (5., 4., "Long Profit 5"), (10., 9., "Long Profit 10"), (12., 12., "Long Profit 12")] {
                        if e.mth >= ep + lvl * at { v.push(o(K::Xl, Ty::Stop, ep + stop * at, nm)); }
                    }
                }
                if pos.mp != -1 && c3 { v.push(o(K::Sell, Ty::Stop, highest(h, k, 3) - sek * a7[k], "Short Entry")); }
                if pos.mp == -1 {
                    let ep = pos.ep;
                    v.push(o(K::Xs, Ty::Stop, highest(h, k, ch), "Short Channel"));
                    if c[k] < ep - pk * at { v.push(o(K::Xs, Ty::Stop, highest(h, k, ch2), "Short Profit 1.5")); }
                    if c[k] < ep - sxk * at { v.push(o(K::Xs, Ty::Open, 0., "Short Profit 3 (next open)")); }
                    if e.mnl <= ep - sba * at { v.push(o(K::Xs, Ty::Stop, ep - sb * at, "Short Breakeven")); }
                }
                v
            })
        }
        _ => return None,
    })
}

/// Bond-futures moving average over the bond's own bars (NaN days = no bond bar), aligned back.
fn bond_ma(bond: &[f64], n: usize) -> Vec<f64> {
    let idx: Vec<usize> = (0..bond.len()).filter(|&i| bond[i].is_finite()).collect();
    let own: Vec<f64> = idx.iter().map(|&i| bond[i]).collect();
    let m = sma(&own, n);
    let mut out = vec![f64::NAN; bond.len()];
    let mut j = 0;
    let mut last = f64::NAN;
    for (i, v) in out.iter_mut().enumerate() {
        while j < idx.len() && idx[j] <= i { last = m[j]; j += 1; }
        *v = last;
    }
    out
}
fn last_valid(v: &[f64], k: usize) -> f64 { (0..=k).rev().map(|i| v[i]).find(|x| x.is_finite()).unwrap_or(f64::NAN) }

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;

    fn load(name: &str) -> Option<Vec<Row>> {
        let dir = std::env::var("LEBSP_DATA").unwrap_or("/home/server/.local/share/strategy-vault/work/lebsp/data".into());
        let txt = std::fs::read_to_string(format!("{dir}/{name}")).ok()?;
        Some(txt.lines().skip(1).map(|l| {
            let f: Vec<&str> = l.split(',').collect();
            let n = |k: usize| f[k].parse::<f64>().unwrap();
            Row { ticker: "F".into(), date: f[0].into(), open: n(1), high: n(2), low: n(3), close: n(4), volume: None }
        }).collect())
    }
    fn window(r: &[Row], a: &str, b: &str) -> Vec<Row> { r.iter().filter(|x| x.date.as_str() >= a && x.date.as_str() <= b).cloned().collect() }
    /// (closed trades, net USD after $150 per trade, winners)
    fn usd(ts: &[Trade], pv: f64) -> (usize, f64, usize) {
        let v: Vec<f64> = ts.iter().filter(|t| !t.open).map(|t| (t.exit_px - t.legs[0].px) * t.dir * pv - 150.).collect();
        (v.len(), v.iter().sum(), v.iter().filter(|x| **x > 0.).count())
    }
    fn go(id: &str, file: &str, a: &str, b: &str, tick: f64, x_extra: Option<(&str, Vec<f64>)>) -> Option<(Vec<Row>, Vec<Trade>)> {
        let r = window(&load(file)?, a, b);
        let mut x = Ctx::new(&r, &r, &[]);
        if let Some((k, v)) = x_extra { x.extra.insert(k.into(), v); }
        let ts = run(id, &x, 0, r.len() - 1, &Params::from([("tick".into(), tick)]))?;
        Some((r, ts))
    }

    #[test]
    fn mack_reproduces_manual() {
        let Some((r, ts)) = go("lebsp_mack", "yx_mack.csv", "1988-10-17", "1998-12-31", 0.05, None) else { return };
        let (n, net, w) = usd(&ts, 500.);
        assert_eq!((n, w), (45, 31)); // Table 1: 45 trades, 31 winners, $173,475
        assert!((net - 173475.).abs() < 1e-6, "{net}");
        assert_eq!((r[ts[0].legs[0].i].date.as_str(), ts[0].legs[0].px), ("1989-03-06", 261.65));
    }

    #[test]
    fn piranha_reproduces_manual() {
        let Some(sp) = load("sp_piranha.csv") else { return };
        let Some(ua) = load("ua_piranha.csv") else { return };
        let r = window(&sp, "1988-06-15", "1999-10-29");
        let m: std::collections::HashMap<&str, f64> = ua.iter().filter(|x| x.date.as_str() >= "1988-06-15").map(|x| (x.date.as_str(), x.close)).collect();
        let bond: Vec<f64> = r.iter().map(|x| m.get(x.date.as_str()).copied().unwrap_or(f64::NAN)).collect();
        let (_, ts) = go("lebsp_piranha", "sp_piranha.csv", "1988-06-15", "1999-10-29", 0.1, Some(("TBOND_FUT_CLOSE", bond))).unwrap();
        let (n, net, w) = usd(&ts, 250.);
        assert_eq!((n, w), (156, 119)); // Table 1: 156 trades, 119 winners, $305,037.50
        assert!((net - 305037.5).abs() < 1e-6, "{net}");
    }

    #[test]
    fn remarkable_close_to_manual() {
        let Some((_, ts)) = go("lebsp_remarkable", "sp_remark.csv", "1988-10-17", "1998-12-31", 0.1, None) else { return };
        let (n, net, w) = usd(&ts, 250.);
        assert_eq!((n, w), (114, 96)); // book: 114 trades, 96 winners, $207,362.50 (3 trades differ by 0.1 pt)
        assert!((net - 207387.5).abs() < 1e-6, "{net}");
    }

    #[test]
    fn prudent_close_to_manual() {
        let Some((_, ts)) = go("lebsp_prudent", "sp_prudent.csv", "1988-10-17", "1998-12-31", 0.1, None) else { return };
        let (n, net, _) = usd(&ts, 250.);
        assert_eq!(n, 147); // book: 147 trades, $133,037.50 (first trade exits differently: ADX seeding)
        assert!((net - 133337.5).abs() < 1e-6, "{net}");
    }

    #[test]
    fn money_stop_fires_on_entry_bar() {
        // bar 1: buy stop 101 fills, then the bar falls 6+ points: the $1,500 (6 pt) money stop exits the same bar
        let rows: Vec<Row> = [(100., 100.5, 99.5, 100.), (100.5, 101.5, 94., 95.)].iter().enumerate()
            .map(|(i, &(o, h, l, c))| Row { ticker: "T".into(), date: format!("{i:05}"), open: o, high: h, low: l, close: c, volume: None }).collect();
        let x = Ctx::new(&rows, &rows, &[]);
        let ts = sim(&x, 0, 1, Some(6.), 0., |_, _| vec![o(K::Buy, Ty::Stop, 101., "t")]);
        assert_eq!((ts[0].legs[0].px, ts[0].exit_i, ts[0].exit_px), (101., 1, 95.));
    }
}
