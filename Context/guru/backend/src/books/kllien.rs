//! Kathy Lien: "High Probability Trading Setups for the Currency Market" (with Boris Schlossberg, Investopedia 2006; book id
//! `klhpt`), "Day Trading and Swing Trading the Currency Market" (Wiley 2008; `kldst`) and "Day Trading the Currency Market"
//! (Wiley 2005; `kldtc`). Reading: claude-continuation-20261020/authors/lien/agents/. Live: the "Do the Right Thing" CCI trade on
//! daily bars (klhpt Setup 2) and the moving-average perfect order (kldst Ch.9 = kldtc Ch.8). The other setups use pip-sized
//! entries, stops or targets, intraday charts, hand-picked swing points or interest-rate carry and are held. Book: currency pairs;
//! indices, ETFs and stocks are a declared application.
use crate::connors::{dmi, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "klhpt";
pub const REVIEWED: &[&str] = &["klhpt_cci_right", "kldst_perfect_order", "klhpt_rsi_coaster"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const CCI_NOTE: &str = "[Declared readings] CCI(20) = (typical price - its 20-day average) / (0.015 x mean absolute deviation), typical price = (high + low + close) / 3. An excursion is a run of closes with CCI above +100 (short: below -100); the prior peak is the highest (lowest) CCI of the last finished excursion. In a later excursion the first close whose CCI is beyond that peak enters at the close; one entry per excursion. Stop = the signal candle's low (high); risk R = entry - stop (no trade if R = 0). Half is sold at entry + R, the stop on the rest then moves to the entry price from the next bar, and the rest is sold at entry + 2R. Limits fill at the level or a better open; stops at the level or a worse open; when a bar touches both the stop and a target the stop is taken first (the order inside the bar is unknown). Daily bars (the book: daily or hourly).";
const PO_NOTE: &str = "[Declared readings] Perfect order up = 10-day > 20-day > 50-day > 100-day > 200-day simple averages of closes (down: the reverse). The formation day is the first close in perfect order. If the order holds on that close and the next four, and the 14-day ADX rose on the fourth (\"ADX pointing upward\"; the 'ideally greater than 20' wish is not applied), buy (sell) at the open of the fifth candle after the formation. Stop = the formation day's low (high), from the entry bar. Exit at the next open after the first close when the perfect order no longer holds (the book's examples exit 'when prices open'). One position at a time; a new trade needs a new formation.";

const RSIC_NOTE: &str = "[Declared readings] (1) RSI = Wilder's RSI of the close, length 14 (the text gives only the 70/30 levels). (2) The signal bar is the first close back through the level on a candle of the signal's colour: RSI of the previous close below 30 and RSI of this close above 30 on an up candle (close above open); short mirrored at 70. (3) The swing low (high) for the stop is the lowest low (highest high) from the first bar of the run of RSI readings beyond the level through the signal bar. (4) Entry at the next bar's open (Rule 3; the worked example's same-price re-entry is not used); a trade whose open is already at or beyond the stop has no risk and is not taken. The half target (50% of the entry-to-stop distance) and the stop are walked on the daily bar by the module's convention, the stop first when one bar holds both, resolved on real 1-minute bars where they exist (EP6); the breakeven stop works from the bar after the first target. The rest closes at the close of the bar on which RSI, having been beyond the opposite level since entry, comes back inside it, and a signal on that same bar enters at the next open. Wilder RSI; a signal while a position is open is not taken; a position open at the end is marked open. Book: currency pairs (daily and 4-hour charts); no result table is printed. ";

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    let d = |id, name, group, book, status, rules, notes| StrategyDef { id, name, group, session: 0, dir: "Long/Short", status, needs_volume: false,
        needs_vix: false, needs_series: &[], book, rules, notes, claim: None };
    let nt = |id, name, group, book, why, rules| d(id, name, group, book, NotTestable(why), rules, "Held after the 2026-10-24 reading (authors/lien/agents).");
    let pips = "entry, stop or target is a fixed number of pips (10, 15, 20, 30...), which has no equivalent for indices, ETFs or stocks";
    vec![
        d("klhpt_cci_right", "\"Do the Right Thing\" CCI trade (new CCI peak beyond +100 / -100)", "Part 2 Setup 2 (p.27-33)", "klhpt", Live,
          &["F|\"On the daily or the hourly charts place the CCI indicator with standard input of 20\" (Rules 1)",
            "E|\"Note the very last time the CCI registered a reading of greater than +100 before dropping back below the +100 zone. Take a measure of the peak CCI reading ... If CCI once again trades above the +100 and if its value exceeds the prior peak reading, go long at market at the close of the candle\" (Rules 2-4)",
            "X|\"Measure the low of the candle and use it as your stop. If the position moves in your favor by the amount of your original stop, sell half and move stop to breakeven. Take profit on the rest of the trade when position moves to two times your stop\" (Rules 5-7)"], CCI_NOTE),
        d("kldst_perfect_order", "Perfect order in the 10/20/50/100/200-day moving averages", "Ch.9 Technical Trading Strategies (p.138-140)", "kldst", Live,
          &["F|\"the 10-day simple moving average (SMA) is at a higher price level than the 20-day SMA, which is higher than the 50-day SMA. Meanwhile, the 100-day SMA would be below the 50-day SMA, while the 200-day SMA would be below the 100-day SMA\" (p.138)",
            "E|\"1. Look for a currency pair with moving averages in perfect order. 2. Look for ADX pointing upward, ideally greater than 20. 3. Buy five candles after the initial formation of the perfect order (if it still holds)\" (p.139)",
            "X|\"4. The initial stop is the low on the day of the initial crossover for longs and the high for shorts. 5. Exit the position when the perfect order no longer holds\" (p.139); same rule in Day Trading the Currency Market (2005) Ch.8"], PO_NOTE),
        nt("klhpt_momo", "Five-minute momo trade", "Part 2 Setup 1 (p.20-26)", "klhpt", pips, &["E|\"Go long 10 pips above the 20-period EMA\" on 5-minute charts with the MACD histogram turning positive"]),
        nt("klhpt_ma_macd", "Moving average MACD combo (50/100 SMA)", "Part 2 Setup 3 (p.34-40)", "klhpt", "pip buffers (10 pips) with no index equivalent; lane D1 pass 2 coded it on the Norgate AUD/GBP/CHF daily futures (0 mismatches against a reference, #240 0%) but the honest count of unstated choices is 5-6 (MACD definition and what 'crosses to positive' means, bar timing: signal at the close and fill at the next open, which price the 10 pips are measured from, whether the five-bar window includes the signal bar, one entry per MACD signal, how the SMA exit fills), over the R4 #339 limit of 4, so it is held (review 9029ba2f)", &["E|\"Once the price has broken above the closest SMA by 10 pips or more, enter long if MACD crosses to positive within the last five bars\""]),
        d("klhpt_rsi_coaster", "RSI rollercoaster (RSI 30/70 turn on a reversal candle, half off at 50% of the risk)", "Part 2 Setup 4 (p.41-44)", "klhpt", Live,
          &["F|\"RSI reading must be less than 30. Wait for an up candle to form and close with an RSI reading greater than 30.\" (Rules for a Long Trade 1-2, p.42); short: RSI greater than 70, then a down candle closing with RSI less than 70",
            "E|\"Go long at market on the open of the next candle.\" (Rule 3)",
            "X|\"Place your stop at the swing low. Exit half of the position at 50% of the risk and immediately move the stop on the rest to breakeven. Exit the rest ... stopped at breakeven, or trade first moves into overbought territory marked by RSI readings of greater than 70 and then eventually drops from that zone. As soon as RSI declines below 70, sell at market on the close of that candle.\" (Rules 4-6)"], RSIC_NOTE),
        nt("klhpt_pure_fade", "Pure fade (3/2/1 SD Bollinger bands)", "Part 2 Setup 5 (p.49-55)", "klhpt", "hourly charts only, and the stop is the swing low minus 10 pips", &["E|\"After one candle closes fully within the 2SD-1SD BB zone, buy at market\""]),
        nt("klhpt_memory", "The memory of price", "Part 2 Setup 6 (p.56-67)", "klhpt", "the up-move, its retrace and the swing points are picked on the chart; the position is added to at a loss", &["E|\"Enter short half the position when price rallies to the swing high\""]),
        nt("klhpt_seven_day", "Seven-day extension fade", "Part 2 Setup 7 (p.68-75)", "klhpt", "the stop is the seventh bar's low minus 10 pips; with the 1R/3R targets the risk depends on that pip buffer (user may choose a buffer for stocks and indices)", &["E|\"Look for seven consecutive bars of weakness (where each bar's close is below the prior). Buy at the open of the next bar. Place stop at the low of the seventh bar minus 10 pips\""]),
        nt("klhpt_turn_trend", "Turn to trend and turn to the carry", "Part 2 Setup 8 (p.76-85)", "klhpt", "hourly Bollinger entries with a 'swing low minus five points' stop; the carry version trades only in the direction of the positive interest-rate differential of a currency pair", &["E|\"Once the price breaks through and closes above the lower 3SD-2SD Bollinger band channel\" (hourly)"]),
        nt("kldst_pip_strategies", "Double zeros, waiting for the real deal, inside day breakout, the fader, 20-100 momentum, seven-day turn", "Ch.9 Technical Trading Strategies", "kldst", pips, &["E|e.g. inside day: \"Buy 10 pips above the high of the previous inside day\"; fader: \"Place an entry order to buy 15 pips above the previous day's high\""]),
        nt("kldst_false_breakouts", "Filtering false breakouts", "Ch.9 (p.134-136)", "kldst", "the stop is 'a few pips below' the two-day low and the exit is a choice between a trailing stop and a 2R target", &["E|\"Buy the pair if it takes out the 20-day high within three days of making the two-day low\""]),
        nt("kldst_channel", "Channel strategy", "Ch.9 (p.136-138)", "kldst", "the channel lines are drawn by hand", &["E|\"Enter long as the price breaks above the upper channel line\""]),
        nt("kldst_news_fundamental", "News trading, carry trades and fundamental strategies", "Ch.9-11", "kldst", "needs economic release times and consensus forecasts, interest rates and bond spreads", &[]),
        nt("kldtc_same_rules", "Technical strategies of the 2005 edition", "Day Trading the Currency Market Ch.8", "kldtc", "the same strategies as the 2008 edition (perfect order is run under kldst_perfect_order); the rest use pips", &[]),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "klhpt_cci_right" => ("klhpt_cci_right", "Lien & Schlossberg CCI trade", "daily"),
        "kldst_perfect_order" => ("kldst_perfect_order", "Lien perfect order", "10/20/50/100/200"),
        _ if id.starts_with("klhpt_") || id.starts_with("kldst_") || id.starts_with("kldtc_") => ("kllien_nt", "Lien (held)", "-"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let mut v = params_base(id)?;
    crate::intraday_ep6::add_param(id, &mut v); // EP6 (#243, lane K63)
    Some(v)
}

fn params_base(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(match id {
        "klhpt_cci_right" => vec![p("len", "CCI length", 20., 5., 50., 1.), p("level", "CCI zone level", 100., 50., 200., 10.)],
        "klhpt_rsi_coaster" => vec![p("len", "RSI length", 14., 2., 50., 1.), p("level", "Overbought level (oversold = 100 minus this)", 70., 55., 90., 1.), p("target", "First target as a fraction of the entry-to-stop risk", 0.5, 0.25, 2., 0.25)],
        "kldst_perfect_order" => vec![p("wait", "Candles after the formation before entry", 5., 1., 20., 1.)],
        _ if id.starts_with("klhpt_") || id.starts_with("kldst_") || id.starts_with("kldtc_") => vec![],
        _ => return None,
    })
}

fn sma_exact(v: &[f64], n: usize) -> Vec<f64> {
    (0..v.len()).map(|i| if n > 0 && i + 1 >= n { v[i + 1 - n..=i].iter().sum::<f64>() / n as f64 } else { f64::NAN }).collect()
}

fn cci(x: &Ctx, n: usize) -> Vec<f64> {
    let tp: Vec<f64> = (0..x.c.len()).map(|i| (x.h[i] + x.l[i] + x.c[i]) / 3.).collect();
    let m = sma_exact(&tp, n);
    (0..tp.len()).map(|i| {
        if !m[i].is_finite() { return f64::NAN; }
        let md = tp[i + 1 - n..=i].iter().map(|v| (v - m[i]).abs()).sum::<f64>() / n as f64;
        if md > 0. { (tp[i] - m[i]) / (0.015 * md) } else { f64::NAN }
    }).collect()
}

/// One bar of a held CCI trade (EP6 state, lane K63): first target taken (`done` at the bar start keeps its breakeven stop;
/// a first target taken on this bar leaves the signal-candle stop for the rest of the bar, the module's reading), the
/// scale-out pieces, and the close (price, reason) if the trade ended on this bar.
#[derive(Clone)]
struct Cci { done: bool, ex: Vec<(usize, f64, f64)>, closed: Option<(f64, &'static str)> }

/// EP6 walk of bar i for a held trade (dir d, entry e, signal-candle stop, risk r, `done0` = first target taken before the
/// bar): no segments = the open; on a segment the stop counts when the path goes strictly beyond it, a target when it reaches
/// it (at the level; on a gap segment at the gap's end).
fn cci_walk(i: usize, o: f64, d: f64, e: f64, stop: f64, r: f64, done0: bool, z: &mut Cci, segs: &[crate::intraday_ep6::Seg]) -> Vec<crate::intraday_ep6::Ev> {
    use crate::intraday_ep6::Ev;
    let s = if done0 { e } else { stop };
    let why_s = if done0 { "breakeven stop after the first target" } else { "stop: signal candle" };
    let (t1, t2) = (e + d * r, e + d * 2. * r);
    let mut ev = vec![];
    let open = [crate::intraday_ep6::Seg { a: o, b: o, gap: false }];
    for (n, g) in if segs.is_empty() { &open[..] } else { segs }.iter().enumerate() {
        if z.closed.is_some() { break; }
        let first = segs.is_empty() && n == 0;
        // stop: the open beyond it, or the path from at/inside it to strictly beyond it
        let beyond = |p: f64| if d > 0. { p < s } else { p > s };
        if first && beyond(o) { z.closed = Some((o, why_s)); ev.push(Ev { k: 0, px: o, up: d < 0. }); break; }
        if !first && !beyond(g.a) && beyond(g.b) { let px = if g.gap { g.b } else { s }; z.closed = Some((px, why_s)); ev.push(Ev { k: 0, px, up: d < 0. }); break; }
        let reach = |tg: f64| if first { if d > 0. { o >= tg } else { o <= tg } } else { (g.b - g.a) * d > 0. && (if d > 0. { g.b >= tg && g.a < tg } else { g.b <= tg && g.a > tg }) };
        if !z.done && reach(t1) { let px = if first || g.gap { g.b } else { t1 }; z.ex.push((i, px, 0.5)); z.done = true; ev.push(Ev { k: 1, px, up: d > 0. }); }
        if z.done && reach(t2) { let px = if first || g.gap { g.b } else { t2 }; z.closed = Some((px, "second target (2 x risk)")); ev.push(Ev { k: 2, px, up: d > 0. }); }
    }
    ev
}

fn run_cci(x: &Ctx, from: usize, to: usize, n: usize, lv: f64) -> Vec<Trade> { run_cci_ep6(x, from, to, n, lv, false) }

/// `run_cci` with EP6 (#243, lane K63): with `on`, a bar of a held trade whose exits differ between its two extreme orders is
/// walked on its real 1-minute path where minutes exist (intraday_ep6::resolve_conv); otherwise the module's daily rule
/// (the stop first, then the targets).
fn run_cci_ep6(x: &Ctx, from: usize, to: usize, n: usize, lv: f64, on: bool) -> Vec<Trade> {
    use crate::intraday_ep6::{bar_segs, mark, resolve_conv, Day, Seg};
    let k = cci(x, n);
    let mut od = Day::Plain;
    let (mut out, mut cur) = (vec![], None::<(Trade, f64, f64, bool, Vec<(usize, f64, f64)>)>); // trade, stop, R, first target done, exits
    // excursion state per side: (in excursion, running extreme, prior peak, entered this excursion)
    let mut st = [(false, f64::NAN, f64::NAN, false), (false, f64::NAN, f64::NAN, false)];
    for i in 1..=to {
        let mut today = Day::Plain; // how this bar's exit was decided (a trade entered at its close carries it)
        // manage the open trade on bar i (entered on an earlier close)
        if i >= from {
            if let Some((mut t, stop, r, done, ex)) = cur.take() {
                let (d, e) = (t.dir, t.legs[0].px);
                let (o, h, l, c) = (x.o[i], x.h[i], x.l[i], x.c[i]);
                let conv = |z: &mut Cci| {
                    let s = if done { e } else { stop };
                    let beyond_stop = |p: f64| if d > 0. { p < s } else { p > s };
                    let reach = |tg: f64| if d > 0. { h >= tg } else { l <= tg };
                    let (t1, t2) = (e + d * r, e + d * 2. * r);
                    if beyond_stop(o) { z.closed = Some((o, if done { "breakeven stop after the first target" } else { "stop: signal candle" })); }
                    else if beyond_stop(if d > 0. { l } else { h }) { z.closed = Some((s, if done { "breakeven stop after the first target" } else { "stop: signal candle" })); }
                    else {
                        if !done && reach(t1) { z.ex.push((i, if (d > 0. && o >= t1) || (d < 0. && o <= t1) { o } else { t1 }, 0.5)); z.done = true; }
                        if z.done && reach(t2) { z.closed = Some((if (d > 0. && o >= t2) || (d < 0. && o <= t2) { o } else { t2 }, "second target (2 x risk)")); }
                    }
                };
                let mut z = Cci { done, ex, closed: None };
                let day = if on {
                    resolve_conv(x, i, true, &mut z, conv,
                        |w: &mut Cci, hf: bool| { let mut v = cci_walk(i, o, d, e, stop, r, done, w, &[]); v.extend(cci_walk(i, o, d, e, stop, r, done, w, &bar_segs(o, h, l, c, hf))); v },
                        |w: &mut Cci, g: &[Seg]| cci_walk(i, o, d, e, stop, r, done, w, g))
                } else { conv(&mut z); Day::Plain };
                od = od.worst(day);
                today = day;
                let Cci { done: done2, mut ex, closed } = z;
                match closed {
                    Some((px, why)) => {
                        if ex.is_empty() { t.exit_px = px; } else { ex.push((i, px, 0.5)); t.exit_px = ex.iter().map(|e| e.1 * e.2).sum::<f64>(); t.exits = ex; }
                        t.exit_i = i; t.signal_i = i; t.reason = why.into(); mark(&mut t, od); out.push(t);
                    }
                    None => cur = Some((t, stop, r, done2, ex)),
                }
            }
        }
        // excursions and signals at the close of bar i
        if !k[i].is_finite() { continue; }
        for (s, d) in [(0usize, 1.), (1usize, -1.)] {
            let inside = if d > 0. { k[i] > lv } else { k[i] < -lv };
            let e = &mut st[s];
            if inside {
                if !e.0 { e.0 = true; e.1 = k[i]; e.3 = false; } else { e.1 = if d > 0. { e.1.max(k[i]) } else { e.1.min(k[i]) }; }
                let beyond = e.2.is_finite() && (if d > 0. { k[i] > e.2 } else { k[i] < e.2 });
                if beyond && !e.3 && i >= from && i < to && cur.is_none() {
                    e.3 = true;
                    let stop = if d > 0. { x.l[i] } else { x.h[i] };
                    let r = (x.c[i] - stop) * d;
                    if r > 0. {
                        od = today;
                        cur = Some((Trade { dir: d, legs: vec![Leg { i, px: x.c[i], w: 1. }], exit_i: i, exit_px: x.c[i], reason: String::new(), entry_kind: "close", signal_i: i, open: false, exits: vec![] }, stop, r, false, vec![]));
                    }
                } else if beyond { e.3 = true; }
            } else if e.0 { e.0 = false; e.2 = e.1; }
        }
    }
    if let Some((mut t, _, _, _, mut ex)) = cur {
        if ex.is_empty() { t.exit_px = x.c[to]; } else { ex.push((to, x.c[to], 0.5)); t.exit_px = ex.iter().map(|e| e.1 * e.2).sum::<f64>(); t.exits = ex; }
        t.exit_i = to; t.signal_i = to; t.reason = "open".into(); t.open = true; mark(&mut t, od); out.push(t);
    }
    out
}

/// Lien's RSI rollercoaster (see RSIC_NOTE). One position; `pend` is a signal of the previous close waiting for the open.
fn run_rsic(x: &Ctx, from: usize, to: usize, n: usize, hi: f64, tf: f64, on: bool) -> Vec<Trade> {
    use crate::intraday_ep6::{mark, stop_target, Day};
    struct P { t: Trade, stop: f64, e: f64, risk: f64, done: bool, ob: bool, ex: Vec<(usize, f64, f64)>, od: Day }
    let (lo, r) = (100. - hi, crate::connors::rsi(&x.c, n));
    let (mut out, mut cur, mut pend) = (vec![], None::<P>, None::<(f64, f64, usize)>);
    let close = |mut p: P, i: usize, px: f64, why: &str, open: bool, out: &mut Vec<Trade>| {
        let w = if p.done { 0.5 } else { 1. };
        if p.ex.is_empty() { p.t.exit_px = px; } else { p.ex.push((i, px, w)); p.t.exit_px = p.ex.iter().map(|e| e.1 * e.2).sum::<f64>(); p.t.exits = p.ex; }
        p.t.exit_i = i; p.t.signal_i = i; p.t.reason = why.into(); p.t.open = open; mark(&mut p.t, p.od); out.push(p.t);
    };
    for i in 1..=to {
        if let Some((d, stop, sig)) = pend.take() {
            let e = x.o[i];
            if cur.is_none() && i >= from && (e - stop) * d > 0. {
                let t = Trade { dir: d, legs: vec![Leg { i, px: e, w: 1. }], exit_i: i, exit_px: e, reason: String::new(), entry_kind: "open", signal_i: sig, open: false, exits: vec![] };
                cur = Some(P { t, stop, e, risk: (e - stop) * d, done: false, ob: false, ex: vec![], od: Day::Plain });
            }
        }
        if let Some(mut p) = cur.take() {
            let d = p.t.dir;
            let (s, tgt) = (if p.done { p.e } else { p.stop }, if p.done { f64::NAN } else { p.e + d * tf * p.risk });
            let (o, h, l) = (x.o[i], x.h[i], x.l[i]);
            let conv = || -> Option<(f64, &'static str)> {
                let why = if p.done { "breakeven stop after the first target" } else { "stop: swing" };
                if (d > 0. && o <= s) || (d < 0. && o >= s) { return Some((o, why)); }
                if (d > 0. && l <= s) || (d < 0. && h >= s) { return Some((s, why)); }
                if tgt.is_finite() && ((d > 0. && o >= tgt) || (d < 0. && o <= tgt)) { return Some((o, "target")); }
                if tgt.is_finite() && ((d > 0. && h >= tgt) || (d < 0. && l <= tgt)) { return Some((tgt, "target")); }
                None
            };
            let (res, day) = stop_target(x, i, d, s, tgt, on, (if p.done { "breakeven stop after the first target" } else { "stop: swing" }, "target"), conv);
            p.od = p.od.worst(day);
            let mut dead = None;
            match res {
                Some((px, "target")) => { p.ex.push((i, px, 0.5)); p.done = true; }
                Some((px, why)) => dead = Some((px, why)),
                None => {}
            }
            if let Some((px, why)) = dead { close(p, i, px, if why.starts_with("stop") { "stop: swing" } else { "breakeven stop after the first target" }, false, &mut out); }
            else {
                // close of bar i: RSI back inside the opposite level after having been beyond it
                let beyond = if d > 0. { r[i] > hi } else { r[i] < lo };
                let back = r[i].is_finite() && p.ob && if d > 0. { r[i] < hi } else { r[i] > lo };
                if back { close(p, i, x.c[i], "RSI back inside the opposite level: exit at the close", false, &mut out); }
                else { if beyond { p.ob = true; } if i == to { close(p, i, x.c[i], "open", true, &mut out); } else { cur = Some(p); } }
            }
        }
        if cur.is_none() && i >= from && i < to && r[i].is_finite() && r[i - 1].is_finite() {
            let (up, dn) = (r[i - 1] < lo && r[i] > lo && x.c[i] > x.o[i], r[i - 1] > hi && r[i] < hi && x.c[i] < x.o[i]);
            if up || dn {
                let mut j = i - 1;
                while j > 0 && r[j - 1].is_finite() && (if up { r[j - 1] < lo } else { r[j - 1] > hi }) { j -= 1; }
                let stop = if up { x.l[j..=i].iter().cloned().fold(f64::INFINITY, f64::min) } else { x.h[j..=i].iter().cloned().fold(f64::NEG_INFINITY, f64::max) };
                pend = Some((if up { 1. } else { -1. }, stop, i));
            }
        }
    }
    out
}

fn run_po(x: &Ctx, from: usize, to: usize, wait: usize) -> Vec<Trade> {
    let c = &x.c;
    let m: Vec<Vec<f64>> = [10, 20, 50, 100, 200].iter().map(|&n| sma_exact(c, n)).collect();
    let adx = dmi(x.r, 14).1;
    let order = |i: usize| -> f64 {
        if !m[4][i].is_finite() { return 0.; }
        if (0..4).all(|k| m[k][i] > m[k + 1][i]) { 1. } else if (0..4).all(|k| m[k][i] < m[k + 1][i]) { -1. } else { 0. }
    };
    let (mut out, mut cur) = (vec![], None::<(Trade, f64)>);
    let mut pending_exit = false;
    for i in from.max(wait + 1)..=to {
        if let Some((mut t, s)) = cur.take() {
            let d = t.dir;
            let hit = if pending_exit { Some((x.o[i], i - 1, "perfect order no longer holds: exit at the open")) }
                else if (d > 0. && x.o[i] < s) || (d < 0. && x.o[i] > s) { Some((x.o[i], i, "stop: formation day")) }
                else if (d > 0. && x.l[i] < s) || (d < 0. && x.h[i] > s) { Some((s, i, "stop: formation day")) } else { None };
            pending_exit = false;
            match hit { Some((px, sig, why)) => { t.exit_i = i; t.exit_px = px; t.signal_i = sig; t.reason = why.into(); out.push(t); } None => cur = Some((t, s)) }
        }
        if cur.is_none() {
            let f = i - wait; // formation bar
            let d = order(f);
            if d != 0. && f >= 1 && order(f - 1) != d && (f..i).all(|j| order(j) == d) && adx[i - 1].is_finite() && adx[i - 2].is_finite() && adx[i - 1] > adx[i - 2] {
                let s = if d > 0. { x.l[f] } else { x.h[f] };
                let t = Trade { dir: d, legs: vec![Leg { i, px: x.o[i], w: 1. }], exit_i: i, exit_px: x.o[i], reason: String::new(), entry_kind: "open", signal_i: i - 1, open: false, exits: vec![] };
                // the stop is live from the entry bar
                if (d > 0. && x.l[i] < s) || (d < 0. && x.h[i] > s) {
                    let px = if (d > 0. && x.o[i] < s) || (d < 0. && x.o[i] > s) { x.o[i] } else { s };
                    let mut t = t; t.exit_i = i; t.exit_px = px; t.signal_i = i; t.reason = "stop: formation day".into(); out.push(t);
                } else { cur = Some((t, s)); }
            }
        }
        if let Some((t, _)) = cur.as_ref() { if order(i) != t.dir && i < to { pending_exit = true; } }
    }
    if let Some((mut t, _)) = cur { t.exit_i = to; t.exit_px = c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true; out.push(t); }
    out
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    Some(match id {
        "klhpt_cci_right" => {
            let Some((from, to)) = crate::intraday_ep6::window(id, x, from, to, p) else { return Some(vec![]) }; // EP6 #243 (c)
            run_cci_ep6(x, from, to, g("len").round().max(2.) as usize, g("level"), crate::intraday_ep6::on(id, p)) // EP6 (#243, lane K63)
        }
        "klhpt_rsi_coaster" => {
            let Some((from, to)) = crate::intraday_ep6::window(id, x, from, to, p) else { return Some(vec![]) };
            run_rsic(x, from, to, g("len").round().max(2.) as usize, g("level"), g("target"), crate::intraday_ep6::on(id, p))
        }
        "kldst_perfect_order" => run_po(x, from, to, g("wait").round().max(1.) as usize),
        _ => return None,
    })
}

// Instrument admission: Lien 2026-10-24 block, moved from instrument_policy.rs on 31 Oct 2026 (lane ST).
// Admission only for the module REVIEWED lists (authors/lien/admission/SOURCE-CONTRACT.md); kept in one block to merge.
// Runs on actual indices, ETFs and stocks, US and India (index_etf_or_stock, nifty50 basket, declared application).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| matches!(def.book,"klhpt"|"kldst"|"kldtc") && (crate::books::kllien::reviewed(def.id)),
    label: "Kathy Lien and Boris Schlossberg wrote these setups for currency pairs; runs on indices, ETFs and stocks, US and India, are a declared application, not the authors' test. Cash-index shorts are hypothetical exposure.",
    basis: "Kathy Lien and Boris Schlossberg, High Probability Trading Setups for the Currency Market (Investopedia 2006), Setup 2 Do the Right Thing (p.27-33); Kathy Lien, Day Trading and Swing Trading the Currency Market (Wiley 2008) Ch.9 Perfect Order (p.138-140), same rule in Day Trading the Currency Market (Wiley 2005) Ch.8. Reviewed in authors/lien/admission/SOURCE-CONTRACT.md with independent chronological ledgers; no costs.",
};

#[cfg(test)]
mod rsic_tests {
    use super::*;
    use crate::Row;
    fn rows(b: &[(f64, f64, f64, f64)]) -> Vec<Row> {
        b.iter().enumerate().map(|(i, &(o, h, l, c))| Row { ticker: "T".into(), date: format!("2020-{:02}-{:02}", 1 + i / 28, 1 + i % 28), open: o, high: h, low: l, close: c, volume: None }).collect()
    }
    #[test]
    fn rsi_coaster_long_half_off_then_rsi_exit() {
        // 20 falling bars (RSI far below 30), an up candle that closes back above 30, then a rise to a half target and an overbought turn
        let mut b = vec![];
        let mut px = 120.0;
        for _ in 0..20 { b.push((px, px + 0.2, px - 1.2, px - 1.0)); px -= 1.0; }
        b.push((px, px + 9.0, px - 0.5, px + 8.0)); px += 8.0; // signal bar, close 108 -> wait: close above its open
        let sig = b.len() - 1;
        b.push((px + 0.5, px + 6.0, px + 0.2, px + 5.0)); px += 5.0; // entry bar: opens above the signal close, runs up through the 50% target
        for _ in 0..12 { b.push((px, px + 3.0, px - 0.2, px + 2.5)); px += 2.5; } // keeps rising: RSI beyond 70
        b.push((px, px + 0.2, px - 26.0, px - 25.0)); // RSI falls back below 70 on a long down bar (the 15-point stop is not reached)
        b.push((px - 25.0, px - 24.0, px - 26.0, px - 25.0));
        let r = rows(&b);
        let x = Ctx::new(&r, &[], &[]);
        let t = run_rsic(&x, 1, r.len() - 1, 14, 70.0, 0.5, false);
        assert!(!t.is_empty(), "a trade");
        let t0 = &t[0];
        assert_eq!(t0.dir, 1.0);
        assert_eq!(t0.legs[0].i, sig + 1);
        assert_eq!(t0.legs[0].px, b[sig + 1].0, "entry at the next open");
        assert_eq!(t0.exits.len(), 2, "half at the target, the rest at the RSI exit: {:?} {}", t0.exits, t0.reason);
        assert!((t0.exits[0].2 - 0.5).abs() < 1e-12 && (t0.exits[1].2 - 0.5).abs() < 1e-12);
        assert!(t0.reason.starts_with("RSI back inside"), "{}", t0.reason);
    }
}
