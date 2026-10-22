//! Joe Ross - the complete daily rules admitted after the full corpus reading (authors/ross, 2026-10-23).
//! `joe_ross.rs` keeps the 20 page-cited source references (all held); this module holds only runnable rules.
//! Readings: authors/ross/agents/<work>/methods.json; contract: authors/ross/admission/SOURCE-CONTRACT.md.
//!
//! Daily-bar conventions [Not from the book], shared by every rule here:
//!   * orders are worked during bar k from what was known at the close of bar k-1; one position at a time;
//!     a new trade needs a flat book at the close of the bar before (Ambush reversals excepted);
//!   * 1 tick = tick_pct % of the previous close (Street Smarts convention; -1 = auto: 0.01% on an index/ETF,
//!     the stock's real tick, 0.05 INR NSE / 0.01 USD US, on an actual stock); the book's tick counts are the defaults;
//!   * Ross enters only when prices trade through the entry ("we will not enter a trade if prices gap over our entry
//!     point", Law of Charts p.12): a stop entry with the open already beyond it is not filled and the setup is dropped;
//!   * exit stops fill at the stop, or at the open when the bar opens beyond it; targets likewise (limit);
//!     a bar reaching both the stop and the target is taken as a stop;
//!   * on the entry bar an exit counts only if the bar proves it came after the fill: after a stop entry the target by
//!     the bar's extreme and the stop by the close; after a limit entry the stop by the extreme and the target by the close.
use crate::connors::{atr, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const ZULU: &str = "jr_zulu";
pub const AMBUSH: &str = "jr_ambush";
pub const TBTB: &str = "jr_tbtb";
pub const TNT1: &str = "jr_tnt1";
pub const TNT3: &str = "jr_tnt3";
pub const LOC: &str = "jr_loc";

/// Every id here is reviewed and admitted (index, ETF and stock, US and India, declared application).
pub const REVIEWED: &[&str] = &[
    "jr_zulu", "jr_ambush", "jr_ledge_tbtb", "jr_sne_l", "jr_sne_s", "jr_bb2_l", "jr_bb2_s",
    "jr_123_std_l", "jr_123_std_s", "jr_hook_std_l", "jr_hook_std_s",
];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }
/// All of them read the open (gap tests, stops filled at a gapped open, day-4 open exit).
pub fn reads_open(id: &str) -> bool { reviewed(id) }

macro_rules! def {
    ($id:expr, $name:expr, $group:expr, $dir:expr, $book:expr, $rules:expr, $notes:expr) => {
        StrategyDef { id: $id, name: $name, group: $group, session: 0, dir: $dir, status: Status::Live, needs_volume: false,
            needs_vix: false, needs_series: &[], book: $book, rules: $rules, notes: $notes, claim: None }
    };
}

const STD_X: &str = "X|[Not from the book] the book gives no complete exit (cost covering, partial exits and trailing are the trader's choice), so the user's standard exits are used: p.exit 0 = close after p.hold bars (3 / 5 / 10), 1 = trailing stop 2 x Wilder ATR(5) of the previous bar behind the best price since entry, 2 = first up close (shorts: first down close); always together with the book's initial stop";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        def!("jr_zulu", "ZULU gap reversal", "Course package · Zulu Trading Method (soybeans)", "Long/Short", ZULU,
            &["E|\"Buy one tick above the low of the last trading day using a Stop Order, but only when the market opens with a gap down (this is when the market opens below the low of the last trading day)\" (Zulu Method p.11)",
              "E|\"Sell one tick below the high of the last trading day using a Stop Order, but only when the market opens with a gap up\" (p.11)",
              "E|\"If you don't get a gap, do not place any order! Cancel your order if not filled by the close of the same day! Do NOT reverse your position! Enter a new trade only when you are flat!\" (p.12)",
              "X|Long: \"Place a sell limit order 32 ticks above your entry. Place a sell stop market order 16 ticks below your entry\"; short mirror (p.12-13). \"We do hold position overnight!\" - no time stop",
              "X|[Not from the book] the optional move of the stop to breakeven when the target is touched but not filled (p.13: \"experiment\") is not used"],
            "Book: CBOT soybeans (1 tick = 1/4 cent; 32 ticks = 8 cents). Here 32 / 16 ticks of tick_pct % of price (default 0.01% on an index/ETF = 0.32% / 0.16%; one soybean tick was about 0.04% of price in 2005: tick_pct 0.04 is closer to the book's distances). Index, ETF and stock runs are a declared application."),
        def!("jr_ambush", "AMBUSH gap fade", "Course package · Ambush Trading Method (wheat, corn)", "Long/Short", AMBUSH,
            &["E|\"Buy at the high of the last trading day using a Limit Order, but only when the market opens with a gap up (this is when the market opens above the high of the last trading day)\" (Ambush p.9)",
              "E|\"Sell at the low of the last trading day using a Limit Order, but only when the market opens with a gap down\" (p.9); \"Cancel your order if not filled by the close of the same day!\" (p.10)",
              "E|\"Do reverse your position! If you are long and you get an entry signal to go short, reverse your position\" (p.10); no pyramiding",
              "X|Long: \"Place a sell limit order 30 ticks above your entry. Place a sell stop market order 40 ticks below your entry\"; short mirror (p.10)",
              "X|\"Time Stop: In case we do NOT get filled at the target or at the stop, we exit the trade after eight days using a MOC (Market On Close) order. We count the entry day as the first day.\" (p.11)",
              "X|[Not from the book] on a reversal day the old position closes at the new entry price (the reversal takes priority over its intraday stop/target unless the open already gapped through them)"],
            "Book: CBOT wheat and corn (1 tick = 1/4 cent; 30 ticks = 7.5 cents, 40 ticks = 10 cents). Here 30 / 40 ticks of tick_pct % of price (soybean-like grain ticks were about 0.06-0.12% of price; default 0.01% on an index/ETF). Index, ETF and stock runs are a declared application."),
        def!("jr_ledge_tbtb", "Ledge straddle (Trading by the Book)", "Trading by the Book · Part V", "Long/Short", TBTB,
            &["F|Rule 1: \"a correction or congestion that is at least three bars in length, but no more than 10 bars in length\" (TBTB p.238); \"Once there are more than ten bars in the ledge, I stop trying\" (p.240)",
              "F|Matching: two highs and two lows, each pair separated by at least one bar, \"off by 1-2 ticks, but no more\" (p.241); p.match_ticks = 2. [Declared reading] the ledge is the longest window of 3-10 bars ending on the latest bar whose highest high and lowest low are each matched (within p.match_ticks) by another bar at least two bars away",
              "F|Rule 10: \"I can go back only as far as the first leg of the previous market swing to find a matching high or low\" (p.239). [Declared swing convention] a bar is an up-leg bar if it makes a higher high without a lower low, a down-leg bar if it makes a lower low without a higher high, an outside bar takes the colour of its close (up if close > open, down if close < open), any other bar continues the current leg; a leg is a run of same-direction bars; the window may start no earlier than the first bar of the leg two legs before the current one",
              "E|Rules 3, 5, 6: \"I trade the potential breakout in either direction\"; buy stop above the ledge high, sell stop below the ledge low, \"1 tick above or below the breakout point\" in the direction of the current leg, \"at least 3 ticks\" against it (p.238); orders called in each day",
              "E|[Declared] stop-limit orders as in the S&P example (p.246): no fill when the open is already beyond the order; if both sides trade on one bar, the side nearer the open fills first and the other side then stops the trade",
              "X|\"Whichever way the breakout occurs, the other open order stop becomes my initial protective stop\" (Rule 5, p.238)",
              "X|Rule 7: \"As soon as two bars on the chart are out of the congestion, I move my stop to one tick ... below the second bar if the breakout is to the upside\" (p.238); [declared] out = closed beyond the ledge boundary, entry bar = bar 1",
              "X|\"Our profit objective is an amount equal to the height of the ledge\" measured from the ledge boundary (34800 - 455 = 34345, p.246)",
              "X|\"If our target objective is not hit on the third day, we place it as a regular market stop order the fourth day ... filled on or about the open\" (p.246): exit at the open of day 4 (entry day = day 1)",
              "X|Not used [account-dependent or not numeric]: the doubled reversing stop (Rules 3 and 9), \"move our stop as quickly as possible to breakeven\" (p.246)"],
            "Trading by the Book Part V, OCR pages 238-246; also German Futures-Trading PDF 249-257. Book: US futures, daily. A different, book-literal ledge from the Law of Charts ledge (jr_ledge, held). Index, ETF and stock runs are a declared application."),
        def!("jr_sne_l", "Simple count: three higher highs · Long", "Electronic Trading TNT I · ch.27 Trend finding", "Long", TNT1,
            &["F|\"We must see three bars in a row of higher highs\" (TNT I p.231): three bars each with a higher high than the bar before",
              "E|\"WE WILL ATTEMPT TO BUY A BREAKOUT OF THE THIRD HIGH PROVIDED THE MARKET CORRECTS ONE OR MORE OF THE BARS BY MAKING A LOWER LOW ONCE THREE HIGHS ARE IN PLACE\" (p.231): after a bar with a lower low than its predecessor (the third-high bar itself may be that bar, p.231 chart), buy one tick above the third high",
              "E|p.correction 0 = the 1999 By the Minute recap (p.349: \"The method does not require that you wait for a correction bar\") [labelled option]",
              "E|[Declared] a fourth consecutive higher high becomes the new third high; three consecutive lower lows cancel the long setup; each setup trades once",
              "X|\"we must exit the trade as soon as we see that we have violated a low when prices are supposed to be moving up\" (p.232): sell stop one tick below the previous bar's low, from the bar after entry"],
            "TNT I examples are intraday stock charts; the book says the rules apply \"just as ... to the daily or weekly chart\" (p.230). Daily index/ETF/stock runs are a declared application; the day-trade close-out does not apply to daily bars."),
        def!("jr_sne_s", "Simple count: three lower lows · Short", "Electronic Trading TNT I · ch.27 Trend finding", "Short", TNT1,
            &["F|\"For down moves, the same rules will apply in reverse\" (TNT I p.231): three bars in a row of lower lows",
              "E|After a bar with a higher high than its predecessor (correction), sell one tick below the third low; p.correction 0 = By the Minute 1999 recap without the correction [labelled option]",
              "E|[Declared] a fourth consecutive lower low becomes the new third low; three consecutive higher highs cancel the short setup; each setup trades once",
              "X|Exit as soon as a bar violates the previous bar's high: buy stop one tick above it, from the bar after entry (p.232 in reverse)"],
            "Mirror of jr_sne_l. Daily index/ETF/stock runs are a declared application; cash-index shorts are hypothetical exposure."),
        def!("jr_bb2_l", "Bollinger Bands Method #2 · Long", "Electronic Trading TNT III · Bollinger Bands", "Long", TNT3,
            &["F|\"In Method #2, we set the moving average for the Bollinger Bands to a value of 9 bars ... The standard deviation set at two remains\" (TNT III p.218); p.len 20 = German Band III (p.171) [labelled option]",
              "F|\"Once prices are clearly in the upper channel, they remain there until prices drop cleanly below the moving average ... It requires only one price bar to move entirely into the opposite channel\" (p.218): upper channel from a bar entirely above the average until a bar entirely below it",
              "E|\"Prices must touch or exceed the moving average, but they cannot move cleanly into the opposite channel. Once prices have touched the moving average, we enter on a violation of the extreme of the bar that touched the moving average\" (p.219-220): buy one tick above the high of the latest touching bar while still in the upper channel",
              "E|[Declared simplification] the \"ahead of a #2 point violation or a Rh\" context is not required (the worked trades enter on each touch-bar break)",
              "X|\"We remain in the trade until prices violate the extreme of the previous bar by moving against the direction of the trend\" (p.220): sell stop one tick below the previous bar's low, from the bar after entry",
              "X|\"... or we take profits on reversal bars at the bands\" (p.220): a bar reaching the upper band that closes below its open exits on the close (full exit [declared]; p.band_exit 0 switches it off)"],
            "TNT III example: 5-minute Microsoft. Daily index/ETF/stock runs are a declared application. Population standard deviation of the closes."),
        def!("jr_bb2_s", "Bollinger Bands Method #2 · Short", "Electronic Trading TNT III · Bollinger Bands", "Short", TNT3,
            &["F|9-bar average, 2 standard deviations (TNT III p.218); p.len 20 = German Band III [labelled option]; lower channel from a bar entirely below the average until a bar entirely above it",
              "E|A bar in the lower channel touching the average without moving entirely above it; sell one tick below that bar's low (p.219-220, mirror)",
              "X|Buy stop one tick above the previous bar's high, from the bar after entry; a bar reaching the lower band that closes above its open exits on the close (p.220)"],
            "Mirror of jr_bb2_l. Cash-index shorts are hypothetical exposure."),
        def!("jr_123_std_l", "1-2-3 low breakout with standard exits · Long", "Law of Charts · 1-2-3 (standard exits)", "Long", LOC,
            &["F|Point 1 = \"the last bar to have made a new low in the most recent down-leg\" (Law of Charts p.4): [declared] the lowest low since the last breakout; any bar at or below it nullifies the formation and becomes the new point 1 (\"nullified when any price bar moves prices equal to or beyond the number 1 point\", p.5)",
              "F|Point 2 = highest high after point 1, confirmed by a \"full correction\": within up to three bars a bar with a lower high and a bar with a lower low than the bar before (any order); more than three bars = congestion (p.4)",
              "F|Point 3 = lowest low after point 2 (above point 1), confirmed within one or two bars by a bar with a higher high and a bar with a higher low than the bar before (p.5)",
              "F|\"On the daily chart, I prefer to see at least four bars making up the formation from 1-3\" (Trading by the Book p.97): point 1 to point 3 at least 4 bars; \"The number two point does not have to be a significant high or low\" (p.371)",
              "E|\"I will enter this market if prices reach one tick higher than the number 2 point\" (TBTB p.80), from the bar after point 3 is confirmed; no fill on a gap over the entry (LoC p.12); a bar reaching point 1 cancels the order",
              "X|\"My stop will be placed one tick lower than the number 3 point\" (TBTB p.80)",
              STD_X],
            "Formation definitions: The Law of Charts (also TNT I-III, By the Minute). Stop and 4-bar minimum: Trading by the Book. The \"major or intermediate turning point / 50% retracement\" context (LoC p.12) is a judgement and is not applied. Daily index/ETF/stock runs are a declared application."),
        def!("jr_123_std_s", "1-2-3 high breakout with standard exits · Short", "Law of Charts · 1-2-3 (standard exits)", "Short", LOC,
            &["F|1-2-3 high (Law of Charts p.2-3): point 1 = highest high since the last breakout (a bar at or above it nullifies); point 2 = lowest low after it, confirmed within three bars by a higher high and a higher low bar; point 3 = highest high after point 2 (below point 1), confirmed within two bars by a lower high and a lower low bar; at least 4 bars from 1 to 3",
              "E|Sell one tick below point 2 from the bar after point 3 is confirmed; no fill on a gap through the entry; a bar reaching point 1 cancels the order",
              "X|Stop one tick above point 3 (TBTB p.80, mirror)",
              STD_X],
            "Mirror of jr_123_std_l. Cash-index shorts are hypothetical exposure."),
        def!("jr_hook_std_l", "Ross Hook after a 1-2-3 breakout with standard exits · Long", "Law of Charts · Ross Hook (standard exits)", "Long", LOC,
            &["F|\"In an up-trending market, after the breakout of a 1-2-3 low, the first instance of the failure of a price bar to make a new high creates a Ross Hook. (A double high/double top also creates a Ross Hook)\" (Law of Charts p.7): the first bar after the breakout whose high is not above the previous high; hook point = that previous high",
              "F|[Declared] only the 1-2-3 origin is used (ledge and trading-range origins are not coded); the 1-2-3 is the jr_123_std_l formation",
              "E|\"Buy on a breakout of the point of the Ross Hook\" (p.13), one tick above it; no fill on a gap over it (p.17-18); \"if we were to get more than three correcting bars, we would have to assume that the trend is at least temporarily over\" (p.23): the order works for three bars after the hook bar",
              "X|\"Place a protective stop one tick below the lower of the last two days, including the day you enter the trade\" (Trading by the Book p.375), working from the bar after entry",
              STD_X],
            "Hook definition: The Law of Charts; stop: Trading by the Book newspaper version. Congestion-precedence and second-time-through rules of Trading the Ross Hook are not applied (that book's version stays held as jr_hook). Daily index/ETF/stock runs are a declared application."),
        def!("jr_hook_std_s", "Ross Hook after a 1-2-3 breakout with standard exits · Short", "Law of Charts · Ross Hook (standard exits)", "Short", LOC,
            &["F|\"after the breakout of a 1-2-3 high, the first instance of the failure of a price bar to make a new low creates a Ross Hook\" (Law of Charts p.7); hook point = the previous low",
              "E|Sell one tick below the hook point for three bars after the hook bar; no fill on a gap through it",
              "X|Stop one tick above the higher of the highs of the entry day and the day before (TBTB p.375, mirror), from the bar after entry",
              STD_X],
            "Mirror of jr_hook_std_l. Cash-index shorts are hypothetical exposure."),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "jr_zulu" => ("jr_zulu", "ZULU gap reversal", "Long/Short"),
        "jr_ambush" => ("jr_ambush", "AMBUSH gap fade", "Long/Short"),
        "jr_ledge_tbtb" => ("jr_ledge_tbtb", "Ledge straddle (Trading by the Book)", "Long/Short"),
        "jr_sne_l" => ("jr_sne", "Simple count (successive new extremes)", "Long"),
        "jr_sne_s" => ("jr_sne", "Simple count (successive new extremes)", "Short"),
        "jr_bb2_l" => ("jr_bb2", "Bollinger Bands Method #2", "Long"),
        "jr_bb2_s" => ("jr_bb2", "Bollinger Bands Method #2", "Short"),
        // the same methods as the held source references jr_123 / jr_hook: counted once
        "jr_123_std_l" => ("jr_123", "1-2-3 reversal", "Long · standard exits"),
        "jr_123_std_s" => ("jr_123", "1-2-3 reversal", "Short · standard exits"),
        "jr_hook_std_l" => ("jr_hook", "Ross Hook breakout", "Long · standard exits"),
        "jr_hook_std_s" => ("jr_hook", "Ross Hook breakout", "Short · standard exits"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let tick = || p("tick_pct", "1 tick = % of the previous close (-1 = auto: 0.01% on an index/ETF, the real tick on a stock) [declared convention]", -1.0, -1.0, 0.2, 0.01);
    let std = || vec![p("exit", "[Not from the book] exit: 0 = close after hold bars, 1 = 2 x ATR(5) trailing stop, 2 = first up close (shorts: first down close)", 0.0, 0.0, 2.0, 1.0),
        p("hold", "[Not from the book] bars held for exit 0 (3 / 5 / 10)", 5.0, 3.0, 10.0, 1.0), tick()];
    Some(match id {
        "jr_zulu" => vec![p("target_ticks", "Profit target, ticks (book 32)", 32.0, 8.0, 64.0, 4.0), p("stop_ticks", "Protective stop, ticks (book 16)", 16.0, 4.0, 40.0, 2.0), tick()],
        "jr_ambush" => vec![p("target_ticks", "Profit target, ticks (book 30)", 30.0, 8.0, 64.0, 2.0), p("stop_ticks", "Protective stop, ticks (book 40)", 40.0, 8.0, 80.0, 4.0),
            p("days", "Time stop: exit on the close of this day, entry day = 1 (book 8)", 8.0, 2.0, 15.0, 1.0), tick()],
        "jr_ledge_tbtb" => vec![p("match_ticks", "Matching highs/lows within ticks (book 1-2)", 2.0, 0.0, 3.0, 1.0), p("with_ticks", "Entry ticks beyond the ledge with the current leg (book 1)", 1.0, 1.0, 3.0, 1.0),
            p("against_ticks", "Entry ticks beyond the ledge against the current leg (book at least 3)", 3.0, 1.0, 5.0, 1.0), tick()],
        "jr_sne_l" | "jr_sne_s" => vec![p("correction", "Wait for a correction bar (1 = TNT I p.231; 0 = By the Minute 1999 recap) ", 1.0, 0.0, 1.0, 1.0), tick()],
        "jr_bb2_l" | "jr_bb2_s" => vec![p("len", "Average and band length (9 = TNT III; 20 = German Band III)", 9.0, 5.0, 30.0, 1.0), p("sd", "Standard deviations", 2.0, 1.0, 3.0, 0.5),
            p("band_exit", "Exit on a reversal bar at the band (1 = book)", 1.0, 0.0, 1.0, 1.0), tick()],
        "jr_123_std_l" | "jr_123_std_s" | "jr_hook_std_l" | "jr_hook_std_s" => std(),
        _ => return None,
    })
}

fn getp(id: &str, p: &Params, key: &str) -> f64 {
    p.get(key).copied().unwrap_or_else(|| params(id).and_then(|s| s.iter().find(|d| d.key == key).map(|d| d.def)).unwrap_or_else(|| panic!("{id} has no param {key}")))
}

struct Pos { dir: f64, e: usize, px: f64, stop: f64, tgt: f64, kind: &'static str }

fn trade(p: &Pos, k: usize, px: f64, sig: usize, why: &str) -> Trade {
    Trade { dir: p.dir, legs: vec![Leg { i: p.e, px: p.px, w: 1.0 }], exit_i: k, exit_px: px, reason: why.into(), entry_kind: p.kind, signal_i: sig, open: false, exits: vec![] }
}
fn open_trade(p: &Pos, x: &Ctx, to: usize) -> Trade {
    Trade { open: true, ..trade(p, to, x.c[to], to, "open") }
}

/// Resting stop/target on a bar after the entry bar: gapped open first, then the stop, then the target.
fn bar_exit(x: &Ctx, k: usize, d: f64, stop: f64, tgt: f64) -> Option<(f64, &'static str)> {
    let (o, h, l) = (x.o[k], x.h[k], x.l[k]);
    let beyond = |a: f64, b: f64| if d > 0.0 { a <= b } else { a >= b }; // a at or past the stop side of b
    if stop.is_finite() && beyond(o, stop) { return Some((o, "stop")); }
    if tgt.is_finite() && beyond(tgt, o) { return Some((o, "target")); }
    if stop.is_finite() && beyond(if d > 0.0 { l } else { h }, stop) { return Some((stop, "stop")); }
    if tgt.is_finite() && beyond(tgt, if d > 0.0 { h } else { l }) { return Some((tgt, "target")); }
    None
}
/// Exits proven on the entry bar (see module conventions).
fn entry_bar_exit(x: &Ctx, k: usize, p: &Pos) -> Option<(f64, &'static str)> {
    let d = p.dir;
    let (ext_lo, ext_hi) = if d > 0.0 { (x.l[k], x.h[k]) } else { (x.h[k], x.l[k]) };
    let past = |a: f64, b: f64| if d > 0.0 { a <= b } else { a >= b };
    let (stop_by, tgt_by) = if p.kind == "stop" { (x.c[k], ext_hi) } else { (ext_lo, x.c[k]) };
    if p.stop.is_finite() && past(stop_by, p.stop) { return Some((p.stop, "stop")); }
    if p.tgt.is_finite() && past(p.tgt, tgt_by) { return Some((p.tgt, "target")); }
    None
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || to >= x.c.len() || from > to {
        return None;
    }
    let g = |k: &str| getp(id, p, k);
    let tp = g("tick_pct");
    let c = &x.c;
    // one tick for orders worked during bar k (k >= 1)
    let tick = |k: usize| match (tp < 0.0, x.stock_tick()) { (true, Some(t)) => t, (true, None) => c[k - 1] * 0.01 / 100.0, (false, _) => c[k - 1] * tp / 100.0 };
    let from = from.max(1);
    Some(match id {
        "jr_zulu" => zulu(x, from, to, g("target_ticks"), g("stop_ticks"), &tick),
        "jr_ambush" => ambush(x, from, to, g("target_ticks"), g("stop_ticks"), g("days").round().max(1.0) as usize, &tick),
        "jr_ledge_tbtb" => ledge(x, from, to, g("match_ticks"), g("with_ticks"), g("against_ticks"), &tick),
        "jr_sne_l" | "jr_sne_s" => {
            let d = if id.ends_with("_l") { 1.0 } else { -1.0 };
            let lv = sne_levels(x, to, d, g("correction") >= 0.5, &tick);
            violation_trades(x, from, to, d, &lv, &tick, None)
        }
        "jr_bb2_l" | "jr_bb2_s" => {
            let d = if id.ends_with("_l") { 1.0 } else { -1.0 };
            let (lv, band) = bb2_levels(x, to, d, g("len").round().max(2.0) as usize, g("sd"), &tick);
            violation_trades(x, from, to, d, &lv, &tick, (g("band_exit") >= 0.5).then_some(&band[..]))
        }
        _ => {
            let d = if id.ends_with("_l") { 1.0 } else { -1.0 };
            let (f123, hooks) = formations(x, to, d, &tick);
            let atr5 = atr(x.r, 5);
            let lv = if id.starts_with("jr_123") { f123 } else { hooks };
            std_trades(x, from, to, d, &lv, id.starts_with("jr_hook"), g("exit").round() as i64, g("hold").round().max(1.0) as usize, &atr5, &tick)
        }
    })
}

// ---------------------------------------------------------------- course packages

fn zulu(x: &Ctx, from: usize, to: usize, tt: f64, st: f64, tick: &dyn Fn(usize) -> f64) -> Vec<Trade> {
    let (o, h, l) = (&x.o, &x.h, &x.l);
    let (mut out, mut pos): (Vec<Trade>, Option<Pos>) = (Vec::new(), None);
    for k in from..=to {
        if let Some(p) = &pos {
            if let Some((px, why)) = bar_exit(x, k, p.dir, p.stop, p.tgt) {
                out.push(trade(p, k, px, k, why));
                pos = None;
            }
            continue; // a new trade needs a flat book at the previous close
        }
        let t = tick(k);
        let new = if o[k] < l[k - 1] && h[k] >= l[k - 1] + t {
            let px = l[k - 1] + t;
            Some(Pos { dir: 1.0, e: k, px, stop: px - st * t, tgt: px + tt * t, kind: "stop" })
        } else if o[k] > h[k - 1] && l[k] <= h[k - 1] - t {
            let px = h[k - 1] - t;
            Some(Pos { dir: -1.0, e: k, px, stop: px + st * t, tgt: px - tt * t, kind: "stop" })
        } else { None };
        if let Some(p) = new {
            if let Some((px, why)) = entry_bar_exit(x, k, &p) { out.push(trade(&p, k, px, k, why)); } else { pos = Some(p); }
        }
    }
    if let Some(p) = pos { out.push(open_trade(&p, x, to)); }
    out
}

fn ambush(x: &Ctx, from: usize, to: usize, tt: f64, st: f64, days: usize, tick: &dyn Fn(usize) -> f64) -> Vec<Trade> {
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let (mut out, mut pos): (Vec<Trade>, Option<Pos>) = (Vec::new(), None);
    for k in from..=to {
        let t = tick(k);
        // today's signal: gap up -> buy limit at yesterday's high, gap down -> sell limit at yesterday's low, filled
        // when prices trade back to it during the day
        let sig = if o[k] > h[k - 1] && l[k] <= h[k - 1] { Some((1.0, h[k - 1])) } else if o[k] < l[k - 1] && h[k] >= l[k - 1] { Some((-1.0, l[k - 1])) } else { None };
        let enter = |d: f64, px: f64, out: &mut Vec<Trade>| -> Option<Pos> {
            let p = Pos { dir: d, e: k, px, stop: px - d * st * t, tgt: px + d * tt * t, kind: "limit" };
            if let Some((xp, why)) = entry_bar_exit(x, k, &p) { out.push(trade(&p, k, xp, k, why)); None } else { Some(p) }
        };
        if let Some(p) = pos.take() {
            let d = p.dir;
            let opp = sig.filter(|s| s.0 == -d);
            let gap = if (d > 0.0 && o[k] <= p.stop) || (d < 0.0 && o[k] >= p.stop) { Some("stop") }
                else if (d > 0.0 && o[k] >= p.tgt) || (d < 0.0 && o[k] <= p.tgt) { Some("target") } else { None };
            if let Some(why) = gap {
                out.push(trade(&p, k, o[k], k, why));
                // only the reversing (opposite) order was working today
                if let Some((nd, lv)) = opp { pos = enter(nd, lv, &mut out); }
            } else if let Some((nd, lv)) = opp {
                out.push(trade(&p, k, lv, k, "reversed"));
                pos = enter(nd, lv, &mut out);
            } else if let Some((px, why)) = bar_exit(x, k, d, p.stop, p.tgt) {
                out.push(trade(&p, k, px, k, why));
            } else if k + 1 - p.e >= days {
                out.push(trade(&p, k, c[k], k, "time stop (market on close)"));
            } else {
                pos = Some(p);
            }
            continue;
        }
        if let Some((d, lv)) = sig { pos = enter(d, lv, &mut out); }
    }
    if let Some(p) = pos { out.push(open_trade(&p, x, to)); }
    out
}

// ---------------------------------------------------------------- Trading by the Book ledge

/// Leg direction of every bar (declared swing convention): +1 up, -1 down, carried when undecided.
fn legs(x: &Ctx, to: usize) -> Vec<i8> {
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let mut v = vec![0i8; to + 1];
    for t in 1..=to {
        let (up, dn) = (h[t] > h[t - 1], l[t] < l[t - 1]);
        v[t] = match (up, dn) {
            (true, false) => 1,
            (false, true) => -1,
            (true, true) if c[t] > o[t] => 1,
            (true, true) if c[t] < o[t] => -1,
            _ => v[t - 1],
        };
    }
    v
}

/// The ledge ending at bar t: (high, low) of the longest qualifying 3-10 bar window, or None.
fn ledge_at(x: &Ctx, t: usize, earliest: usize, tol: f64) -> Option<(f64, f64)> {
    let (h, l) = (&x.h, &x.l);
    for n in (3..=10usize).rev() {
        if n > t + 1 || t + 1 - n < earliest { continue; }
        let s = t + 1 - n;
        let hi = (s..=t).map(|i| h[i]).fold(f64::NEG_INFINITY, f64::max);
        let lo = (s..=t).map(|i| l[i]).fold(f64::INFINITY, f64::min);
        let pair = |near: &dyn Fn(usize) -> bool| (s..=t).any(|i| near(i) && (i + 2..=t).any(|j| near(j)));
        if pair(&|i| h[i] >= hi - tol) && pair(&|i| l[i] <= lo + tol) {
            return Some((hi, lo));
        }
    }
    None
}

fn ledge(x: &Ctx, from: usize, to: usize, mt: f64, wt: f64, at: f64, tick: &dyn Fn(usize) -> f64) -> Vec<Trade> {
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let lg = legs(x, to);
    // first bar of the leg two legs before the current one, for every bar (Rule 10)
    let mut start2 = vec![0usize; to + 1];
    let mut starts: Vec<usize> = vec![0];
    for t in 1..=to {
        if lg[t] != lg[t - 1] { starts.push(t); }
        start2[t] = if starts.len() >= 3 { starts[starts.len() - 3] } else { 0 };
    }
    let (mut out, mut pos): (Vec<Trade>, Option<(Pos, f64, f64)>) = (Vec::new(), None); // (pos, ledge hi, ledge lo)
    for k in from..=to {
        if let Some((p, hi, lo)) = &mut pos {
            let day = k - p.e + 1;
            if day >= 4 {
                out.push(trade(p, k, o[k], k - 1, "day 4 open"));
                pos = None;
                continue;
            }
            if let Some((px, why)) = bar_exit(x, k, p.dir, p.stop, p.tgt) {
                out.push(trade(p, k, px, k, why));
                pos = None;
                continue;
            }
            // Rule 7: after two bars out of the ledge (closed beyond it), stop one tick beyond the second bar
            if day == 2 {
                let out2 = |i: usize| if p.dir > 0.0 { c[i] > *hi } else { c[i] < *lo };
                if out2(p.e) && out2(k) {
                    let s = if p.dir > 0.0 { l[k] - tick(k + 1) } else { h[k] + tick(k + 1) };
                    p.stop = if p.dir > 0.0 { p.stop.max(s) } else { p.stop.min(s) };
                }
            }
            continue;
        }
        let t = k - 1;
        let tk = tick(k);
        let Some((hi, lo)) = ledge_at(x, t, start2[t], mt * tk) else { continue };
        let (bo, so) = if lg[t] > 0 { (wt, at) } else if lg[t] < 0 { (at, wt) } else { (at, at) };
        let (buy, sell) = (hi + bo * tk, lo - so * tk);
        if o[k] >= buy || o[k] <= sell { continue; } // stop-limit: no fill through a gap, the ledge is gone
        let (bh, sh) = (h[k] >= buy, l[k] <= sell);
        let height = hi - lo;
        let mk = |d: f64| if d > 0.0 { Pos { dir: 1.0, e: k, px: buy, stop: sell, tgt: hi + height, kind: "stop" } } else { Pos { dir: -1.0, e: k, px: sell, stop: buy, tgt: lo - height, kind: "stop" } };
        let p = match (bh, sh) {
            (true, true) => {
                let d = if buy - o[k] <= o[k] - sell { 1.0 } else { -1.0 };
                let p = mk(d);
                out.push(trade(&p, k, p.stop, k, "stop (both sides traded on the entry bar)"));
                continue;
            }
            (true, false) => mk(1.0),
            (false, true) => mk(-1.0),
            _ => continue,
        };
        if let Some((px, why)) = entry_bar_exit(x, k, &p) { out.push(trade(&p, k, px, k, why)); } else { pos = Some((p, hi, lo)); }
    }
    if let Some((p, _, _)) = pos { out.push(open_trade(&p, x, to)); }
    out
}

// ---------------------------------------------------------------- signal levels + violation exit

/// Entry levels per bar for the simple count: lv[k] = raw level (third high / third low) working during bar k.
/// The setup process does not depend on trades: a touch of level +/- 1 tick (or a gap through it) uses the setup up.
fn sne_levels(x: &Ctx, to: usize, d: f64, need_corr: bool, tick: &dyn Fn(usize) -> f64) -> Vec<f64> {
    let (o, h, l) = (&x.o, &x.h, &x.l);
    let mut lv = vec![f64::NAN; to + 1];
    let (mut level, mut corrected) = (f64::NAN, false);
    let ext = |i: usize| if d > 0.0 { h[i] > h[i - 1] } else { l[i] < l[i - 1] }; // new extreme in the trade direction
    let opp = |i: usize| if d > 0.0 { l[i] < l[i - 1] } else { h[i] > h[i - 1] }; // correction / opposite extreme
    for k in 1..=to {
        if level.is_finite() && (corrected || !need_corr) {
            lv[k] = level;
            let e = level + d * tick(k);
            let gap = if d > 0.0 { o[k] >= e } else { o[k] <= e };
            let hit = if d > 0.0 { h[k] >= e } else { l[k] <= e };
            if gap || hit { level = f64::NAN; }
        }
        if k >= 3 && ext(k) && ext(k - 1) && ext(k - 2) {
            level = if d > 0.0 { h[k] } else { l[k] };
            corrected = opp(k);
        } else if level.is_finite() {
            if opp(k) { corrected = true; }
            if k >= 3 && opp(k) && opp(k - 1) && opp(k - 2) { level = f64::NAN; } // three opposite extremes cancel
        }
    }
    lv
}

/// Bollinger Method #2 levels: touch-bar extreme working during bar k, and the band for the reversal-bar exit.
fn bb2_levels(x: &Ctx, to: usize, d: f64, n: usize, k_sd: f64, tick: &dyn Fn(usize) -> f64) -> (Vec<f64>, Vec<f64>) {
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let mut lv = vec![f64::NAN; to + 1];
    let mut band = vec![f64::NAN; to + 1];
    let mut ma = vec![f64::NAN; to + 1];
    for i in n - 1..=to {
        let w = &c[i + 1 - n..=i];
        let m = w.iter().sum::<f64>() / n as f64;
        let sd = (w.iter().map(|v| (v - m) * (v - m)).sum::<f64>() / n as f64).sqrt();
        ma[i] = m;
        band[i] = m + d * k_sd * sd;
    }
    let (mut state, mut level) = (0i8, f64::NAN);
    for k in 0..=to {
        if k >= 1 && level.is_finite() {
            lv[k] = level;
            let e = level + d * tick(k);
            let gap = if d > 0.0 { o[k] >= e } else { o[k] <= e };
            let hit = if d > 0.0 { h[k] >= e } else { l[k] <= e };
            if gap || hit { level = f64::NAN; }
        }
        if !ma[k].is_finite() { continue; }
        let side = if l[k] > ma[k] { 1 } else if h[k] < ma[k] { -1 } else { 0 };
        if side != 0 {
            if side as f64 != d { level = f64::NAN; } // cleanly into the opposite channel
            state = side;
        } else if state as f64 == d {
            level = if d > 0.0 { h[k] } else { l[k] }; // the latest bar touching the average
        }
    }
    (lv, band)
}

/// Entry at level +/- 1 tick from a flat book (no fill on a gap), exit on a violation of the previous bar's
/// extreme (one tick beyond it) from the bar after entry, or on a reversal bar at the band.
fn violation_trades(x: &Ctx, from: usize, to: usize, d: f64, lv: &[f64], tick: &dyn Fn(usize) -> f64, band: Option<&[f64]>) -> Vec<Trade> {
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let (mut out, mut pos): (Vec<Trade>, Option<Pos>) = (Vec::new(), None);
    for k in from..=to {
        if let Some(p) = &pos {
            let s = if d > 0.0 { l[k - 1] - tick(k) } else { h[k - 1] + tick(k) };
            if let Some((px, _)) = bar_exit(x, k, d, s, f64::NAN) {
                out.push(trade(p, k, px, k, if d > 0.0 { "lower low (violation)" } else { "higher high (violation)" }));
                pos = None;
            } else if let Some(b) = band {
                let rev = if d > 0.0 { h[k] >= b[k] && c[k] < o[k] } else { l[k] <= b[k] && c[k] > o[k] };
                if b[k].is_finite() && rev {
                    out.push(trade(p, k, c[k], k, "reversal bar at the band"));
                    pos = None;
                }
            }
            continue;
        }
        if !lv[k].is_finite() { continue; }
        let e = lv[k] + d * tick(k);
        let gap = if d > 0.0 { o[k] >= e } else { o[k] <= e };
        let hit = if d > 0.0 { h[k] >= e } else { l[k] <= e };
        if !gap && hit {
            pos = Some(Pos { dir: d, e: k, px: e, stop: f64::NAN, tgt: f64::NAN, kind: "stop" });
        }
    }
    if let Some(p) = pos { out.push(open_trade(&p, x, to)); }
    out
}

// ---------------------------------------------------------------- Law of Charts 1-2-3 and Ross Hook

/// An order working during one bar: raw level (entry = level +/- 1 tick), the stop reference (point 3; NaN for a
/// hook, whose stop is set on the entry bar) and the cancel level (point 1; NaN for a hook).
#[derive(Clone, Copy)]
pub struct Armed { pub level: f64, pub p3: f64, pub p1: f64 }

/// Scan the 1-2-3 formation and the first hook after each breakout in direction d (+1 = 1-2-3 low / buy).
/// Price-only state machine (independent of trades). Returns per-bar working orders (1-2-3, hook).
/// Mirrored scale: `lo` is the point-1/3 side, `hi` the point-2 (entry) side, so one code path serves both directions.
pub fn formations(x: &Ctx, to: usize, d: f64, tick: &dyn Fn(usize) -> f64) -> (Vec<Option<Armed>>, Vec<Option<Armed>>) {
    let (o, h, l) = (&x.o, &x.h, &x.l);
    let lo = |i: usize| if d > 0.0 { l[i] } else { -h[i] };
    let hi = |i: usize| if d > 0.0 { h[i] } else { -l[i] };
    let op = |i: usize| if d > 0.0 { o[i] } else { -o[i] };
    let mut f = vec![None; to + 1];
    let mut hk = vec![None; to + 1];
    // p1, p2 (candidate), p2 confirmed, p3 (lowest low after p2), armed
    let (mut p1, mut p2, mut p3): (Option<usize>, usize, usize) = (None, 0, 0);
    let (mut p2ok, mut armed, mut lh, mut ll) = (false, false, false, false);
    // hook watch: (bar of the latest new extreme since the breakout, hook bar once formed)
    let mut watch: Option<(usize, Option<usize>)> = None;
    for k in 0..=to {
        // 1. orders working during bar k (state from the close of bar k-1)
        let mut breakout = false;
        if armed && k >= 1 {
            let a = p1.unwrap();
            f[k] = Some(Armed { level: hi(p2), p3: lo(p3), p1: lo(a) });
            let e = hi(p2) + tick(k);
            if op(k) >= e || hi(k) >= e { breakout = true; }
        }
        if let Some((_, Some(j))) = watch {
            if k > j + 3 {
                watch = None;
            } else if k > j {
                hk[k] = Some(Armed { level: hi(j - 1), p3: f64::NAN, p1: f64::NAN });
                let e = hi(j - 1) + tick(k);
                if op(k) >= e || hi(k) >= e { watch = None; }
            }
        }
        // 2. bar k updates the hook watch and the formation
        if let Some((m, None)) = watch {
            if k > m {
                watch = Some(if hi(k) <= hi(k - 1) { (m, Some(k)) } else { (k, None) });
            }
        }
        // point 2 exceeded before the order was working: the formation broke out without an entry
        if !breakout && p1.is_some() && p2ok && !armed && hi(k) > hi(p2) { breakout = true; }
        if breakout {
            watch = Some((k, None));
            p1 = None; p2ok = false; armed = false;
            continue;
        }
        let a = match p1 {
            Some(a) if lo(k) > lo(a) => a,
            _ => {
                // first bar, or a bar at/below point 1 (nullified): the new point 1, also the first point-2 candidate
                p1 = Some(k); p2 = k; p2ok = false; armed = false; lh = false; ll = false;
                continue;
            }
        };
        if !p2ok {
            if hi(k) > hi(p2) {
                p2 = k; lh = false; ll = false;
            } else if k <= p2 + 3 {
                if hi(k) < hi(k - 1) { lh = true; }
                if lo(k) < lo(k - 1) { ll = true; }
                if lh && ll {
                    p2ok = true;
                    p3 = p2 + 1;
                    for i in p2 + 1..=k { if lo(i) <= lo(p3) { p3 = i; } }
                }
            }
            if !p2ok { continue; }
        } else if lo(k) <= lo(p3) {
            p3 = k; armed = false;
        }
        // point 3: within two bars after it, a higher-high bar and a higher-low bar (bar to bar)
        if !armed && p3 >= a + 3 {
            let end = k.min(p3 + 2);
            let up = (p3 + 1..=end).any(|i| hi(i) > hi(i - 1));
            let hl = (p3 + 1..=end).any(|i| lo(i) > lo(i - 1));
            if up && hl { armed = true; }
        }
    }
    (f, hk)
}

/// Entries from a flat book at the working level + 1 tick (no fill through a gap; a 1-2-3 order is cancelled by a
/// bar reaching point 1), with the book's initial stop and the user's standard exits.
#[allow(clippy::too_many_arguments)]
fn std_trades(x: &Ctx, from: usize, to: usize, d: f64, lv: &[Option<Armed>], hook: bool, mode: i64, hold: usize, atr5: &[f64], tick: &dyn Fn(usize) -> f64) -> Vec<Trade> {
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let s = |v: f64| if d > 0.0 { v } else { -v }; // back from the mirrored scale
    let past = |a: f64, b: f64| if d > 0.0 { a <= b } else { a >= b }; // a at or beyond the stop b
    let (mut out, mut pos): (Vec<Trade>, Option<Pos>) = (Vec::new(), None);
    let (mut best, mut trail) = (f64::NAN, f64::NAN);
    for k in from..=to {
        if let Some(p) = &mut pos {
            if hook && k == p.e + 1 {
                p.stop = if d > 0.0 { l[p.e - 1].min(l[p.e]) - tick(k) } else { h[p.e - 1].max(h[p.e]) + tick(k) };
            }
            let mut stop = p.stop;
            let mut why = "initial stop";
            if mode == 1 {
                if k > p.e + 1 { best = if d > 0.0 { best.max(h[k - 1]) } else { best.min(l[k - 1]) }; }
                let lv = best - d * 2.0 * atr5[k - 1];
                if lv.is_finite() { trail = if trail.is_finite() { if d > 0.0 { trail.max(lv) } else { trail.min(lv) } } else { lv }; }
                if trail.is_finite() && (!stop.is_finite() || (if d > 0.0 { trail > stop } else { trail < stop })) { stop = trail; why = "2 x ATR(5) trailing stop"; }
            }
            if stop.is_finite() && past(o[k], stop) { out.push(trade(p, k, o[k], k, why)); pos = None; continue; }
            if stop.is_finite() && past(if d > 0.0 { l[k] } else { h[k] }, stop) { out.push(trade(p, k, stop, k, why)); pos = None; continue; }
            if mode == 0 && k >= p.e + hold { out.push(trade(p, k, c[k], k, &format!("{hold} bars held"))); pos = None; continue; }
            if mode == 2 && (if d > 0.0 { c[k] > c[k - 1] } else { c[k] < c[k - 1] }) {
                out.push(trade(p, k, c[k], k, if d > 0.0 { "first up close" } else { "first down close" }));
                pos = None;
            }
            continue;
        }
        let Some(a) = lv[k] else { continue };
        let e = s(a.level) + d * tick(k);
        if past(e, o[k]) { continue; } // the open is already at or beyond the entry: no fill (gap)
        if !(if d > 0.0 { h[k] >= e } else { l[k] <= e }) { continue; }
        if !hook && (if d > 0.0 { l[k] <= s(a.p1) } else { h[k] >= s(a.p1) }) { continue; } // point 1 reached: cancelled
        let stop = if hook { f64::NAN } else { s(a.p3) - d * tick(k) };
        let p = Pos { dir: d, e: k, px: e, stop, tgt: f64::NAN, kind: "stop" };
        best = e;
        trail = f64::NAN;
        if stop.is_finite() && past(c[k], stop) { out.push(trade(&p, k, stop, k, "initial stop")); continue; }
        pos = Some(p);
    }
    if let Some(p) = pos { out.push(open_trade(&p, x, to)); }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn rows(v: &[(f64, f64, f64, f64)]) -> Vec<Row> {
        v.iter().enumerate().map(|(i, &(o, h, l, c))| Row { ticker: "SPY".into(), date: format!("2020-01-{:02}", i + 1), open: o, high: h, low: l, close: c, volume: None }).collect()
    }
    fn tp(t: f64) -> Params { Params::from([("tick_pct".to_string(), t)]) }

    #[test]
    fn catalogue_ids_are_reviewed_live_and_have_params() {
        let cat = catalogue();
        assert_eq!(cat.len(), REVIEWED.len());
        for d in &cat {
            assert!(reviewed(d.id) && d.status == Status::Live && d.book.starts_with("jr_") && d.claim.is_none(), "{}", d.id);
            assert!(params(d.id).is_some() && family_of(d.id).is_some(), "{}", d.id);
        }
    }
    #[test]
    fn zulu_gap_down_buys_one_tick_above_the_low_and_hits_the_target() {
        // tick_pct 1 -> 1 tick = 1% of the previous close (100 -> 1.0)
        let r = rows(&[(100., 101., 99., 100.), (97., 100.5, 96., 100.), (100., 135., 99.5, 130.)]);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("jr_zulu", &x, 0, 2, &tp(1.0)).unwrap();
        // entry 99 + 1 = 100; stop 100 - 16 = 84; target 100 + 32 = 132 (entry bar high 100.5 < 132)
        assert_eq!(t.len(), 1);
        assert_eq!((t[0].legs[0].i, t[0].legs[0].px, t[0].exit_i, t[0].exit_px, t[0].reason.as_str()), (1, 100.0, 2, 132.0, "target"));
    }
    #[test]
    fn ambush_reverses_on_the_opposite_gap() {
        // bar1 gaps up (open 103 > high 101) and trades back to 101: long at 101; bar2 gaps down below 100 and trades up
        // to 100: reverse short at 100
        let r = rows(&[(100., 101., 99., 100.), (103., 104., 100.5, 102.), (98., 100.5, 97., 99.), (99., 99.5, 98.5, 99.)]);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("jr_ambush", &x, 0, 3, &tp(1.0)).unwrap();
        assert_eq!(t[0].reason, "reversed");
        assert_eq!((t[0].dir, t[0].legs[0].px, t[0].exit_i, t[0].exit_px), (1.0, 101.0, 2, 100.5));
        assert_eq!((t[1].dir, t[1].legs[0].i, t[1].legs[0].px, t[1].open), (-1.0, 2, 100.5, true));
    }
    #[test]
    fn simple_count_needs_a_correction_by_default() {
        // three higher highs (bars 1-3), bar 4 lower low (correction), bar 5 breaks bar 3's high
        let r = rows(&[(10., 10., 9., 9.5), (9.6, 10.5, 9.4, 10.), (10., 11., 9.8, 10.8), (10.8, 12., 10.5, 11.5), (11.5, 11.8, 10.4, 11.), (11.2, 12.5, 11., 12.2), (12.2, 12.6, 11.9, 12.)]);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("jr_sne_l", &x, 0, 6, &tp(0.01)).unwrap();
        assert_eq!(t.len(), 1);
        assert_eq!(t[0].legs[0].i, 5);
        assert!((t[0].legs[0].px - (12.0 + 11.0 * 0.0001)).abs() < 1e-12);
        // bar 6 low 11.9 < bar 5 low 11 - tick? no: 11.9 > 10.9989 -> still open
        assert!(t[0].open);
        let no_corr = run("jr_sne_l", &x, 0, 6, &Params::from([("correction".to_string(), 0.0), ("tick_pct".to_string(), 0.01)])).unwrap();
        assert_eq!(no_corr[0].legs[0].i, 5); // working from bar 4 already, but bar 4 (high 11.8) does not reach 12
    }
}
