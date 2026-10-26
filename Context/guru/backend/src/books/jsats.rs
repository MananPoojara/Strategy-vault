//! Jack Schwager's Guide to Winning with Automated Trading Systems (course, traderslibrary.com) with "The Jack Schwager
//! Trading Course: Library of Charts and Systems" lesson manuals 2-22 (the course's EasyLanguage code; code wins over
//! the slide captions). Sources: Books/Jack D. Schwager/Jack Schwager's Guide to Winning with Automated Trading
//! Systems.pdf and lesson_N.pdf (image scans; OCR and page-image reading in claude-continuation-20261020/authors/schwager/
//! agents/ats_course/). Cites read "L18 p.15" = lesson 18 manual, PDF page 15.
//!
//! Execution as the course sets TradeStation up (L13 p.3 "Generate orders for next bar"; L19 p.18 "5-Day up gap triggers
//! buy at next open"): the code runs at each bar's close and "at market" orders fill at the next bar's open. Buy/Sell
//! reverse, a Buy while long (Sell while short) is ignored, ExitLong/ExitShort only close the position held at the
//! signal bar. All code variables start at their initial values on the selected start date; price functions (averages,
//! highest/lowest, true range) use earlier bars. An order on the last bar of the window is not filled.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "jsats";

/// Rules admitted after the 2026-10-23 full reading (authors/schwager/admission/SOURCE-CONTRACT.md) and independent
/// chronological Python ledgers (0 mismatches, US and India). Indices, ETFs and stocks are a declared application.
pub const REVIEWED: &[&str] = &["jsats_bo", "jsats_ma_x", "jsats_ma_x_ts", "jsats_mom", "jsats_gap", "jsats_consec", "jsats_consec_rev", "jsats_wrd1", "jsats_wrd2", "jsats_wrd3", "jsats_band", "jsats_bo_atr", "jsats_bo_time", "jsats_bo_thrust", "jsats_bo_tx", "jsats_bo_txr", "jsats_stock1"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "Run as the course's EasyLanguage code on daily bars: the code runs at each close and orders fill at the next open (\"Generate orders for next bar\", L13 p.3; \"triggers buy at next open\", L19 p.18). The course tests on futures (D-mark, T-bonds, corn, sugar, gold, Canadian dollar, soybeans) and US stocks (IBM, Apple, Allied Signal, Caterpillar, Alcoa); indices and ETFs are a declared application, stocks only where the course used futures. Code variables start at their initial values on the selected start date; earlier bars only warm averages, channels and true ranges. One unit, prices in points; no commissions or dollar results are claimed.";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, dir, rules| StrategyDef {
        id, name, group, session: 0, dir, status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[],
        book: BOOK, rules, notes: NOTE, claim: None,
    };
    const TF: &str = "Trend-following systems";
    const CT: &str = "Counter-trend and pattern systems";
    const CF: &str = "Confirmation, exits and filters";
    vec![
        d("jsats_bo", "N-day breakout (80 days, stop-and-reverse)", TF, "Long/Short", &[
            "E|\"IF Close > Highest(High, N)[1] Then Buy at Market; IF Close < Lowest(Low, N)[1] Then Sell at Market;\" Inputs: N(80) (L18 p.15-16)",
            "X|Exit = the opposite breakout (Sell reverses a long)",
        ]),
        d("jsats_ma_x", "Moving-average crossover (10/100, stop-and-reverse)", TF, "Long/Short", &[
            "E|Method 1: \"IF Average(Close, Length1) > Average(Close, Length2) Then Buy at the Market; IF Average(Close, Length1) < Average(Close, Length2) Then Sell at the Market;\" Input: Length1(10), Length2(100) (L18 p.5)",
            "E|Option p.method = 2: \"IF Average(Close, Length1) crosses over Average(Close, Length2) Then Buy ... crosses under ... Then Sell\" (L18 p.6; \"No signal until moving averages cross\", p.8)",
            "X|Exit = the opposite signal (stop-and-reverse); the course also runs 10/50 and 12/120",
        ]),
        d("jsats_ma_x_ts", "Crossover 12/120 with 40-day time stop (optional 50-day re-entry)", CF, "Long/Short", &[
            "E|Crossover signals (Method 2) with 12/120 averages (L18 p.9; L12 p.52-53, corn)",
            "X|\"IF BarsSinceEntry > N AND MarketPosition = 1 AND Close < EntryPrice Then ExitLong at Market Else IF BarsSinceEntry > N AND MarketPosition = -1 AND Close > EntryPrice Then ExitShort at Market;\" N = 40, in the Else branch of the crossover signals (L18 p.10; \"Close below buy after 40 days. Stop cuts loss\", p.9)",
            "E|Option p.reentry = 1: \"IF MarketPosition = 0 AND MarketPosition(1) = 1 AND Close > Highest(High, N)[1] Then Buy at Market; IF MarketPosition = 0 AND MarketPosition(1) = -1 AND Close < Lowest(Low, N)[1] Then Sell at Market;\" N = 50 (L18 p.12; L12 p.54-61)",
        ]),
        d("jsats_mom", "Close versus close N days ago (50 days, stop-and-reverse)", TF, "Long/Short", &[
            "E|\"{This system will buy when today's close is greater than the close of N days ago.} Input: N(50); IF Close > Close[N] Then Buy at market; IF Close < Close[N] Then Sell at market;\" (L13 p.4; L6 p.22-25 uses 10 and 180 on soybeans)",
            "X|Exit = the opposite signal",
        ]),
        d("jsats_gap", "N-day gap system (5 days, optional 10-day stop)", CT, "Long/Short", &[
            "E|\"If low > Highest(high,N)[1] Then Buy at market; If high < Lowest(low,N)[1] Then Sell at market;\" Inputs: N(5) (L19 p.19; \"5-Day up gap triggers buy at next open\", p.18)",
            "X|Exit = the opposite gap signal",
            "X|Option p.stop = 1 (Gap2, L19 p.21-22): \"LongStop = Lowest(low,N2)[1]\" set on every buy signal (ShortStop = Highest(high,N2)[1] on every sell signal); \"if close > ShortStop then Exitshort at market; if close < LongStop then Exitlong at market;\" N2(10)",
        ]),
        d("jsats_consec", "Consecutive closes (3 up closes buy, 3 down closes sell)", CT, "Long/Short", &[
            "E|\"IF MRO(Close <= Close[1], N, 1) = -1 Then Buy at market; IF MRO(Close >= Close[1], N, 1) = -1 Then Sell at market;\" Input: N(3) (ConsecClose2, L19 p.10)",
            "X|Exit = the opposite signal",
        ]),
        d("jsats_consec_rev", "Consecutive closes after a 20-day closing low/high (optional stop)", CT, "Long/Short", &[
            "E|\"If close > close[1] Then ConsecUp = ConsecUp + 1 Else ConsecUp = 0; If ConsecUp = N AND close[N] < Lowest(close, N2)[N+1] Then Buy at market;\" N(3), N2(20) (L19 p.13); sell case mirrored (ConsecDown = N AND close[N] > Highest(close, N2)[N+1]) [mirror of the printed buy case]",
            "X|Exit = the opposite signal",
            "X|Option p.stop = 1 (ConsecCloseReversLiq, L19 p.15-17): \"If MarketPosition <= 0 Then LongStop = Lowest(low, N)[1]; Buy at market\" ... \"If close < LongStop Then ExitLong at market;\" (short side mirrored); the stop is not moved by later signals in the same direction",
        ]),
        d("jsats_wrd1", "Wide-ranging day #1 (close beyond the latest WRD's true high/low)", CT, "Long/Short", &[
            "E|\"If close > BuyPoint Then Buy at market; If close < SellPoint Then Sell at market;\" then \"If AvgTrueRange(1) > K * AvgTrueRange(N)[1] Then Begin BuyPoint = TrueHigh; SellPoint = TrueLow; End;\" inputs: N(30), K(2.0) (L19 p.24-25; \"Check for signals first\")",
            "X|Exit = the opposite signal",
        ]),
        d("jsats_wrd2", "Wide-ranging day #2 (close beyond the N2-day high/low ending at the WRD)", CT, "Long/Short", &[
            "E|As #1 but \"BuyPoint = Highest(high, N2); SellPoint = Lowest(low, N2);\" inputs: N(25), N2(5), K(2.0) (L19 p.27-28)",
            "X|Exit = the opposite signal",
        ]),
        d("jsats_wrd3", "Wide-ranging day #3 (close beyond the extreme of the last Num WRDs)", CT, "Long/Short", &[
            "E|As #1 but each WRD's TrueHigh/TrueLow is stored in arrays and \"BuyPoint\" = highest of the last Num, \"SellPoint\" = lowest of the last Num; inputs: N(30), K(2.0), Num(2) (L19 p.30-32; the chart uses K 1.5, Num 4)",
            "F|The arrays start at 0, so no sell signal can occur until Num WRDs have been stored (code as printed)",
            "X|Exit = the opposite signal",
        ]),
        d("jsats_band", "Moving-average ATR band counter-trend (60 days, 3 ATR)", CT, "Long/Short", &[
            "F|\"UB = Average(close, N) + K * AvgTrueRange(N); LB = Average(close, N) - K * AvgTrueRange(N);\" inputs: K(3), N(60) (MovAvgBandCT1, L19 p.33)",
            "E|\"If low crosses below LB Then Buy at market; If high crosses above UB Then Sell at market;\" (L19 p.34)",
            "X|Exit = the opposite band signal",
        ]),
        d("jsats_bo_atr", "Breakout with volatility confirmation (30 days, 3 ATR)", CF, "Long/Short", &[
            "E|\"If Close > Highest(High, N)[1] and Buymonitor=0 Then Begin Buypoint = close + k*AvgTrueRange(N); Buymonitor=1; Sellmonitor=0; End; If close > Buypoint Then Begin buy at market; ...\" N(30), k(3) (L18 p.18); sell case mirrored [mirror of the printed buy case]",
            "X|Exit = the opposite confirmed signal",
        ]),
        d("jsats_bo_time", "Breakout with time confirmation (30 days, 5 days)", CF, "Long/Short", &[
            "E|\"If Close > Highest(High, N)[1] AND Buypoint=10000 Then Begin Buypoint = close; Start = CurrentBar; End; If Close < Lowest(Low, N)[1] Then Buypoint = 10000; If CurrentBar >= Start + N2 AND close > Buypoint Then Begin Buy at market; Buypoint = 10000; End;\" N(30), N2(5) (L18 p.20-22; \"Close above signal price 5 days after signal provides confirmation\"); sell mirrored",
            "X|Exit = the opposite confirmed signal",
        ]),
        d("jsats_bo_thrust", "Breakout with thrust-day confirmation (30 days, 4 thrust days)", CF, "Long/Short", &[
            "E|\"If Buymonitor =1 AND close > Buypoint AND close>high[1] Then Buycount = Buycount + 1;\" with the time-confirmation monitor (Buypoint = signal close, cancelled by an opposite breakout) and \"If buycount >= N2 AND close > Buypoint Then Buy\" N(30), N2(4) (L18 p.25; the charts use 5); sell mirrored",
            "X|Exit = the opposite confirmed signal",
        ]),
        d("jsats_bo_tx", "Breakout with time exit after 200 days, no re-entry (80 days)", CF, "Long/Short", &[
            "E|\"IF Close > Highest(High, N1)[1] AND BuyLock = 0 Then Begin Buy at market; Shortcount = 0; SellLock = 0; End Else Begin IF MarketPosition = -1 Then Shortcount = Shortcount + 1; IF Shortcount = N2 Then Begin Exitshort at market; Shortcount = 0; SellLock = 1; End; End;\" N1(80), N2(200) (L18 p.27-29); long side mirrored",
            "X|\"Automatic exit 200 days after entry\" or the opposite breakout; no re-entry in the same direction until an opposite signal",
        ]),
        d("jsats_bo_txr", "Breakout with time exit after 30 days and re-entry (40 days, optional filter)", CF, "Long/Short", &[
            "E|\"IF Close > Highest(High, N1)[1] AND MarketPosition < 1 Then Begin Buy at market; Shortcount = 0; End Else Begin IF MarketPosition = -1 Then Shortcount = Shortcount + 1; IF Shortcount = N2 Then Begin Exitshort at market; Shortcount = 0; End; End;\" N1(40), N2(30) (L18 p.30); long side mirrored",
            "F|Option p.lock > 0 (BreakoutTimeExitRFil, L18 p.31): \"If BuyFilter > 0 Then BuyFilter = BuyFilter + 1;\" and entries only while \"BuyFilter < TrdeLock\" (BuyFilter starts at the first signal and is reset by an opposite signal); code TrdeLock 180, chart 100 (L11 p.2)",
            "X|Time exit after N2 bars in the trade or the opposite breakout",
        ]),
        d("jsats_stock1", "Stock breakout, long only (100 days, exit on the 100-day low)", TF, "Long", &[
            "E|\"Input: N(100); If Close > Highest(High, N)[1] Then Buy at market; If Close < Lowest(Low, N)[1] Then Exitlong at market;\" (BreakoutStock1, L19 p.2-3; BreakoutStock3 adds a constant $1,000 position, which does not change the trades)",
            "X|Exit on the N-day low close",
        ]),
        StrategyDef { id: "jsats_held", name: "Course systems not run (multiple units, pyramids, dollar stops)", group: CF, session: 0, dir: "Long/Short",
            status: Status::NotTestable(HELD), needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
            rules: &["E|Multiple-position breakout (L16 p.5-7), pyramid on retracement with pyramid stop (L18 p.34-40), 12/120 crossover with TradeStation dollar stops (L12), BreakoutStock4 with % trailing stops (L12 p.70-99)"], notes: HELD, claim: None },
    ]
}

const HELD: &str = "Not run: the multiple-position breakout and the pyramid-on-retracement system add units to one position (this runner publishes one unit per trade); the 12/120 crossover dollar stops ($1,000 money-management, $2,000 breakeven, $2,000-$3,000 trailing, $3,000-$4,000 objectives) are per-contract futures amounts with no index or stock equivalent; BreakoutStock4 uses TradeStation's built-in intraday % trailing and money-management stops, whose fill path inside a daily bar the course does not give. Moving Average Band #3, the 6x ATR dynamic stop and the SRD example are incomplete in the course.";

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let f = match id {
        "jsats_bo" => "N-day breakout", "jsats_ma_x" => "MA crossover", "jsats_ma_x_ts" => "MA crossover time stop",
        "jsats_mom" => "Close vs close N days ago", "jsats_gap" => "N-day gap", "jsats_consec" => "Consecutive closes",
        "jsats_consec_rev" => "Consecutive closes after a closing low", "jsats_wrd1" => "Wide-ranging day #1",
        "jsats_wrd2" => "Wide-ranging day #2", "jsats_wrd3" => "Wide-ranging day #3", "jsats_band" => "ATR band counter-trend",
        "jsats_bo_atr" => "Breakout ATR confirmation", "jsats_bo_time" => "Breakout time confirmation",
        "jsats_bo_thrust" => "Breakout thrust confirmation", "jsats_bo_tx" => "Breakout time exit, no re-entry",
        "jsats_bo_txr" => "Breakout time exit with re-entry", "jsats_stock1" => "Stock breakout long only",
        "jsats_held" => "Multi-unit and dollar-stop systems · not run",
        _ => return None,
    };
    Some(("jsats", "Schwager automated trading systems course", f))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(match id {
        "jsats_bo" => vec![p("n", "Breakout days (N)", 80.0, 2.0, 200.0, 1.0)],
        "jsats_ma_x" => vec![p("short", "Length1", 10.0, 2.0, 60.0, 1.0), p("long", "Length2", 100.0, 10.0, 250.0, 1.0), p("method", "Method: 1 above/below (L18 p.5), 2 crossover (p.6)", 1.0, 1.0, 2.0, 1.0)],
        "jsats_ma_x_ts" => vec![p("short", "Length1", 12.0, 2.0, 60.0, 1.0), p("long", "Length2", 120.0, 10.0, 250.0, 1.0), p("n", "Time stop days", 40.0, 5.0, 200.0, 1.0),
            p("reentry", "Re-entry on the N-day high/low: 0 off, 1 on (L18 p.12)", 0.0, 0.0, 1.0, 1.0), p("rn", "Re-entry days", 50.0, 5.0, 200.0, 1.0)],
        "jsats_mom" => vec![p("n", "Days back (N)", 50.0, 2.0, 250.0, 1.0)],
        "jsats_gap" => vec![p("n", "Gap days (N)", 5.0, 1.0, 50.0, 1.0), p("stop", "Gap2 stop: 0 off, 1 on (L19 p.21)", 0.0, 0.0, 1.0, 1.0), p("n2", "Stop days (N2)", 10.0, 2.0, 50.0, 1.0)],
        "jsats_consec" => vec![p("n", "Consecutive closes (N)", 3.0, 2.0, 10.0, 1.0)],
        "jsats_consec_rev" => vec![p("n", "Consecutive closes (N)", 3.0, 2.0, 10.0, 1.0), p("n2", "Closing low/high days (N2)", 20.0, 5.0, 100.0, 1.0),
            p("stop", "Liquidation stop: 0 off, 1 on (L19 p.15)", 0.0, 0.0, 1.0, 1.0)],
        "jsats_wrd1" => vec![p("n", "ATR days (N)", 30.0, 5.0, 100.0, 1.0), p("k", "WRD multiple (K)", 2.0, 1.0, 4.0, 0.1)],
        "jsats_wrd2" => vec![p("n", "ATR days (N)", 25.0, 5.0, 100.0, 1.0), p("k", "WRD multiple (K)", 2.0, 1.0, 4.0, 0.1), p("n2", "High/low days (N2)", 5.0, 1.0, 20.0, 1.0)],
        "jsats_wrd3" => vec![p("n", "ATR days (N)", 30.0, 5.0, 100.0, 1.0), p("k", "WRD multiple (K)", 2.0, 1.0, 4.0, 0.1), p("num", "WRDs used (Num)", 2.0, 1.0, 10.0, 1.0)],
        "jsats_band" => vec![p("n", "Average and ATR days (N)", 60.0, 5.0, 200.0, 1.0), p("k", "ATR multiple (K)", 3.0, 0.5, 6.0, 0.5)],
        "jsats_bo_atr" => vec![p("n", "Breakout days (N)", 30.0, 5.0, 200.0, 1.0), p("k", "ATR multiple (k)", 3.0, 0.5, 6.0, 0.5)],
        "jsats_bo_time" => vec![p("n", "Breakout days (N)", 30.0, 5.0, 200.0, 1.0), p("n2", "Delay days (N2)", 5.0, 1.0, 30.0, 1.0)],
        "jsats_bo_thrust" => vec![p("n", "Breakout days (N)", 30.0, 5.0, 200.0, 1.0), p("n2", "Thrust days (N2)", 4.0, 1.0, 10.0, 1.0)],
        "jsats_bo_tx" => vec![p("n1", "Breakout days (N1)", 80.0, 5.0, 200.0, 1.0), p("n2", "Exit after days (N2)", 200.0, 5.0, 500.0, 1.0)],
        "jsats_bo_txr" => vec![p("n1", "Breakout days (N1)", 40.0, 5.0, 200.0, 1.0), p("n2", "Exit after days (N2)", 30.0, 5.0, 500.0, 1.0),
            p("lock", "Re-entry filter days (TrdeLock; 0 = no filter; code 180, chart 100)", 0.0, 0.0, 500.0, 1.0)],
        "jsats_stock1" => vec![p("n", "Breakout days (N)", 100.0, 5.0, 250.0, 1.0)],
        _ => return None,
    })
}

#[derive(Clone, Copy, PartialEq)]
enum O { Buy, Sell, XLong, XShort }

fn sma(v: &[f64], n: usize, i: usize) -> f64 { if i + 1 < n { f64::NAN } else { v[i + 1 - n..=i].iter().sum::<f64>() / n as f64 } }
fn hh(v: &[f64], n: usize, i: usize) -> f64 { if i < n { f64::NAN } else { v[i - n..i].iter().copied().fold(f64::MIN, f64::max) } }
fn ll(v: &[f64], n: usize, i: usize) -> f64 { if i < n { f64::NAN } else { v[i - n..i].iter().copied().fold(f64::MAX, f64::min) } }

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if from > to || to >= x.r.len() { return None; }
    for (k, &v) in p {
        let d = schema.iter().find(|d| d.key == k)?;
        let int = d.def.fract() == 0.0 && d.step.fract() == 0.0;
        if !v.is_finite() || v < d.min || v > d.max || (int && v.fract() != 0.0) { return None; }
    }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap().def);
    let u = |k: &str| g(k) as usize;
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let n_all = c.len();
    let tr: Vec<f64> = (0..n_all).map(|i| if i == 0 { h[0] - l[0] } else { h[i].max(c[i - 1]) - l[i].min(c[i - 1]) }).collect();
    let atr = |n: usize, i: usize| sma(&tr, n, i);
    let th = |i: usize| if i == 0 { h[0] } else { h[i].max(c[i - 1]) };
    let tl = |i: usize| if i == 0 { l[0] } else { l[i].min(c[i - 1]) };

    let mut out: Vec<Trade> = vec![];
    let mut cur: Option<Trade> = None;
    let mut last_dir = 0.0f64; // MarketPosition(1): direction of the most recent closed trade
    // code state (initial values at the start date)
    let (mut s1, mut s2) = (0.0f64, 0.0f64);
    let (mut k1, mut k2, mut k3, mut k4) = (0i64, 0i64, 0i64, 0i64);
    // The code's 10000 / close + 10000 sentinels (futures prices) become +/- infinity, so index levels above 10,000
    // cannot reach them.
    let inf = f64::INFINITY;
    match id { "jsats_wrd1" | "jsats_wrd2" | "jsats_wrd3" | "jsats_bo_time" | "jsats_bo_atr" | "jsats_bo_thrust" => { s1 = inf; s2 = -inf; } _ => {} }
    if id == "jsats_gap" || id == "jsats_consec_rev" { s1 = -inf; s2 = inf; } // LongStop, ShortStop
    let mut arr_h: Vec<f64> = vec![0.0; 10];
    let mut arr_l: Vec<f64> = vec![0.0; 10];
    let mut start = (0usize, 0usize); // time-confirmation start bars (buy, sell)

    for i in from..=to {
        let mp = cur.as_ref().map_or(0.0, |t| t.dir);
        let mut ords: Vec<O> = vec![];
        match id {
            "jsats_bo" | "jsats_stock1" => {
                let n = u("n");
                if c[i] > hh(h, n, i) { ords.push(O::Buy); }
                if c[i] < ll(l, n, i) { ords.push(if id == "jsats_bo" { O::Sell } else { O::XLong }); }
            }
            "jsats_ma_x" => {
                let (a, b) = (sma(c, u("short"), i), sma(c, u("long"), i));
                if g("method") == 1.0 {
                    if a > b { ords.push(O::Buy); }
                    if a < b { ords.push(O::Sell); }
                } else if i > 0 {
                    let (a1, b1) = (sma(c, u("short"), i - 1), sma(c, u("long"), i - 1));
                    if a > b && a1 <= b1 { ords.push(O::Buy); }
                    if a < b && a1 >= b1 { ords.push(O::Sell); }
                }
            }
            "jsats_ma_x_ts" => {
                let (a, b) = (sma(c, u("short"), i), sma(c, u("long"), i));
                let (a1, b1) = if i > 0 { (sma(c, u("short"), i - 1), sma(c, u("long"), i - 1)) } else { (f64::NAN, f64::NAN) };
                let n = u("n") as i64;
                let since = cur.as_ref().map_or(0, |t| (i - t.legs[0].i) as i64);
                let ep = cur.as_ref().map_or(0.0, |t| t.legs[0].px);
                if a > b && a1 <= b1 { ords.push(O::Buy); }
                else if a < b && a1 >= b1 { ords.push(O::Sell); }
                else if since > n && mp == 1.0 && c[i] < ep { ords.push(O::XLong); }
                else if since > n && mp == -1.0 && c[i] > ep { ords.push(O::XShort); }
                if g("reentry") == 1.0 {
                    let rn = u("rn");
                    if mp == 0.0 && last_dir == 1.0 && c[i] > hh(h, rn, i) { ords.push(O::Buy); }
                    if mp == 0.0 && last_dir == -1.0 && c[i] < ll(l, rn, i) { ords.push(O::Sell); }
                }
            }
            "jsats_mom" => {
                let n = u("n");
                if i >= n && c[i] > c[i - n] { ords.push(O::Buy); }
                if i >= n && c[i] < c[i - n] { ords.push(O::Sell); }
            }
            "jsats_gap" => {
                let (n, n2) = (u("n"), u("n2"));
                if l[i] > hh(h, n, i) { ords.push(O::Buy); s1 = ll(l, n2, i); }
                if h[i] < ll(l, n, i) { ords.push(O::Sell); s2 = hh(h, n2, i); }
                if g("stop") == 1.0 {
                    if c[i] > s2 { ords.push(O::XShort); }
                    if c[i] < s1 { ords.push(O::XLong); }
                }
            }
            "jsats_consec" => {
                let n = u("n");
                if i >= n {
                    if (i + 1 - n..=i).all(|j| c[j] > c[j - 1]) { ords.push(O::Buy); }
                    if (i + 1 - n..=i).all(|j| c[j] < c[j - 1]) { ords.push(O::Sell); }
                }
            }
            "jsats_consec_rev" => {
                let (n, n2, stop) = (u("n"), u("n2"), g("stop") == 1.0);
                if i > 0 && c[i] > c[i - 1] { k1 += 1 } else { k1 = 0 }
                if i > 0 && c[i] < c[i - 1] { k2 += 1 } else { k2 = 0 }
                // close[N] against Lowest(close, N2)[N+1]: the close before the run against the N2 closes before it
                if k1 == n as i64 && i >= n + n2 && c[i - n] < c[i - n - n2..i - n].iter().copied().fold(f64::MAX, f64::min) {
                    if stop && mp <= 0.0 { s1 = ll(l, n, i); }
                    ords.push(O::Buy);
                }
                if k2 == n as i64 && i >= n + n2 && c[i - n] > c[i - n - n2..i - n].iter().copied().fold(f64::MIN, f64::max) {
                    if stop && mp >= 0.0 { s2 = hh(h, n, i); }
                    ords.push(O::Sell);
                }
                if stop {
                    if c[i] < s1 { ords.push(O::XLong); }
                    if c[i] > s2 { ords.push(O::XShort); }
                }
            }
            "jsats_wrd1" | "jsats_wrd2" | "jsats_wrd3" => {
                let (n, k) = (u("n"), g("k"));
                if c[i] > s1 { ords.push(O::Buy); }
                if c[i] < s2 { ords.push(O::Sell); }
                if i >= n && tr[i] > k * atr(n, i - 1) {
                    match id {
                        "jsats_wrd1" => { s1 = th(i); s2 = tl(i); }
                        "jsats_wrd2" => { let n2 = u("n2"); s1 = hh(h, n2, i + 1); s2 = ll(l, n2, i + 1); }
                        _ => {
                            arr_h.rotate_right(1); arr_l.rotate_right(1);
                            arr_h[0] = th(i); arr_l[0] = tl(i);
                            let num = u("num");
                            s1 = arr_h[..num].iter().copied().fold(f64::MIN, f64::max);
                            s2 = arr_l[..num].iter().copied().fold(f64::MAX, f64::min);
                        }
                    }
                }
            }
            "jsats_band" => {
                let (n, k) = (u("n"), g("k"));
                if i > 0 {
                    let band = |j: usize| (sma(c, n, j) + k * atr(n, j), sma(c, n, j) - k * atr(n, j));
                    let ((ub, lb), (ub1, lb1)) = (band(i), band(i - 1));
                    if l[i] < lb && l[i - 1] >= lb1 { ords.push(O::Buy); }
                    if h[i] > ub && h[i - 1] <= ub1 { ords.push(O::Sell); }
                }
            }
            "jsats_bo_atr" => {
                // s1/s2 = Buypoint/Sellpoint, k1/k2 = Buymonitor/Sellmonitor; as printed, a set Buypoint stays live
                // until it fires (only the monitor is switched off by a downside breakout)
                let (n, k) = (u("n"), g("k"));
                if c[i] > hh(h, n, i) && k1 == 0 { s1 = c[i] + k * atr(n, i); k1 = 1; k2 = 0; }
                if c[i] > s1 { ords.push(O::Buy); s1 = inf; k1 = 0; k2 = 0; }
                if c[i] < ll(l, n, i) && k2 == 0 { s2 = c[i] - k * atr(n, i); k2 = 1; k1 = 0; }
                if c[i] < s2 { ords.push(O::Sell); s2 = -inf; k1 = 0; k2 = 0; }
            }
            "jsats_bo_time" => {
                let (n, n2) = (u("n"), u("n2"));
                if c[i] > hh(h, n, i) && s1 == inf { s1 = c[i]; start.0 = i; }
                if c[i] < ll(l, n, i) { s1 = inf; }
                if i >= start.0 + n2 && c[i] > s1 { ords.push(O::Buy); s1 = inf; }
                if c[i] < ll(l, n, i) && s2 == -inf { s2 = c[i]; start.1 = i; }
                if c[i] > hh(h, n, i) { s2 = -inf; }
                if i >= start.1 + n2 && c[i] < s2 { ords.push(O::Sell); s2 = -inf; }
            }
            "jsats_bo_thrust" => {
                // k1/k2 monitors, k3/k4 counts, s1/s2 signal closes
                let (n, n2) = (u("n"), u("n2") as i64);
                if k1 == 1 && c[i] > s1 && c[i] > h[i - 1] { k3 += 1; }
                if k2 == 1 && c[i] < s2 && c[i] < l[i - 1] { k4 += 1; }
                if c[i] > hh(h, n, i) && k1 == 0 { s1 = c[i]; k1 = 1; k2 = 0; }
                if c[i] < ll(l, n, i) { k1 = 0; k3 = 0; }
                if k3 >= n2 && c[i] > s1 { ords.push(O::Buy); s1 = inf; k1 = 0; k2 = 0; k3 = 0; }
                if c[i] < ll(l, n, i) && k2 == 0 { s2 = c[i]; k2 = 1; k1 = 0; }
                if c[i] > hh(h, n, i) { k2 = 0; k4 = 0; }
                if k4 >= n2 && c[i] < s2 { ords.push(O::Sell); s2 = -inf; k1 = 0; k2 = 0; k4 = 0; }
            }
            "jsats_bo_tx" => {
                // k1/k2 = Shortcount/Longcount, k3/k4 = BuyLock/SellLock
                let (n1, n2) = (u("n1"), u("n2") as i64);
                if c[i] > hh(h, n1, i) && k3 == 0 { ords.push(O::Buy); k1 = 0; k4 = 0; }
                else {
                    if mp == -1.0 { k1 += 1; }
                    if k1 == n2 { ords.push(O::XShort); k1 = 0; k4 = 1; }
                }
                if c[i] < ll(l, n1, i) && k4 == 0 { ords.push(O::Sell); k2 = 0; k3 = 0; }
                else {
                    if mp == 1.0 { k2 += 1; }
                    if k2 == n2 { ords.push(O::XLong); k2 = 0; k3 = 1; }
                }
            }
            "jsats_bo_txr" => {
                // k1/k2 = Shortcount/Longcount, k3/k4 = BuyFilter/SellFilter
                let (n1, n2, lock) = (u("n1"), u("n2") as i64, g("lock") as i64);
                if lock > 0 && k3 > 0 { k3 += 1; }
                if lock > 0 && k4 > 0 { k4 += 1; }
                if c[i] > hh(h, n1, i) && (lock == 0 || k3 < lock) && mp < 1.0 { ords.push(O::Buy); if k3 == 0 { k3 = 1; } k1 = 0; k4 = 0; }
                else {
                    if mp == -1.0 { k1 += 1; }
                    if k1 == n2 { ords.push(O::XShort); k1 = 0; }
                }
                if c[i] < ll(l, n1, i) && (lock == 0 || k4 < lock) && mp > -1.0 { ords.push(O::Sell); if k4 == 0 { k4 = 1; } k2 = 0; k3 = 0; }
                else {
                    if mp == 1.0 { k2 += 1; }
                    if k2 == n2 { ords.push(O::XLong); k2 = 0; }
                }
            }
            _ => return None,
        }
        // orders fill at the next open: Buy/Sell reverse, same-side entries ignored, exits close the held side only
        let mut tgt = mp;
        for &od in &ords {
            match od {
                O::Buy => if tgt <= 0.0 { tgt = 1.0 },
                O::Sell => if tgt >= 0.0 { tgt = -1.0 },
                O::XLong => if mp == 1.0 && tgt == 1.0 { tgt = 0.0 },
                O::XShort => if mp == -1.0 && tgt == -1.0 { tgt = 0.0 },
            }
        }
        if tgt == mp || i == to { continue; }
        let (j, px) = (i + 1, o[i + 1]);
        if let Some(mut t) = cur.take() {
            t.exit_i = j; t.exit_px = px; t.signal_i = i;
            t.reason = if tgt == 0.0 { "exit".into() } else { "reverse".into() };
            last_dir = t.dir; out.push(t);
        }
        if tgt != 0.0 {
            cur = Some(Trade { dir: tgt, legs: vec![Leg { i: j, px, w: 1.0 }], exit_i: j, exit_px: px, reason: String::new(),
                entry_kind: "open", signal_i: i, open: false, exits: vec![] });
        }
    }
    if let Some(mut t) = cur { t.exit_i = to; t.exit_px = c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true; out.push(t); }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn breakout_fills_next_open_and_reverses() {
        let px = [10., 10., 10., 12., 12., 8., 8.];
        let r: Vec<Row> = px.iter().enumerate().map(|(i, &c)| Row { ticker: "T".into(), date: format!("2001-01-{:02}", i + 1), open: c + 0.5, high: c + 1.0, low: c - 1.0, close: c, volume: None }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let mut p = Params::new(); p.insert("n".into(), 2.0);
        let t = run("jsats_bo", &x, 0, r.len() - 1, &p).unwrap();
        // bar 3 close 12 > max high(1,2)=11 -> buy at open 4; bar 5 close 8 < min low(3,4)=11 -> sell at open 6
        assert_eq!((t[0].dir, t[0].legs[0].i, t[0].legs[0].px, t[0].exit_i), (1.0, 4, 12.5, 6));
        assert_eq!((t[1].dir, t[1].open), (-1.0, true));
        for d in catalogue().iter().filter(|d| d.status == Status::Live) { assert!(run(d.id, &x, 0, r.len() - 1, &Params::new()).is_some(), "{}", d.id); }
    }
}
