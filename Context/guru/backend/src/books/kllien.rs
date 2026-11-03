//! Kathy Lien: "High Probability Trading Setups for the Currency Market" (with Boris Schlossberg, Investopedia 2006; book id
//! `klhpt`), "Day Trading and Swing Trading the Currency Market" (Wiley 2008; `kldst`) and "Day Trading the Currency Market"
//! (Wiley 2005; `kldtc`). Reading: claude-continuation-20261020/authors/lien/agents/. Live: the "Do the Right Thing" CCI trade on
//! daily bars (klhpt Setup 2) and the moving-average perfect order (kldst Ch.9 = kldtc Ch.8). The other setups use pip-sized
//! entries, stops or targets, intraday charts, hand-picked swing points or interest-rate carry and are held. Book: currency pairs;
//! indices, ETFs and stocks are a declared application.
use crate::connors::{dmi, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "klhpt";
pub const REVIEWED: &[&str] = &["klhpt_cci_right", "kldst_perfect_order"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const CCI_NOTE: &str = "[Declared readings] CCI(20) = (typical price - its 20-day average) / (0.015 x mean absolute deviation), typical price = (high + low + close) / 3. An excursion is a run of closes with CCI above +100 (short: below -100); the prior peak is the highest (lowest) CCI of the last finished excursion. In a later excursion the first close whose CCI is beyond that peak enters at the close; one entry per excursion. Stop = the signal candle's low (high); risk R = entry - stop (no trade if R = 0). Half is sold at entry + R, the stop on the rest then moves to the entry price from the next bar, and the rest is sold at entry + 2R. Limits fill at the level or a better open; stops at the level or a worse open; when a bar touches both the stop and a target the stop is taken first (the order inside the bar is unknown). Daily bars (the book: daily or hourly).";
const PO_NOTE: &str = "[Declared readings] Perfect order up = 10-day > 20-day > 50-day > 100-day > 200-day simple averages of closes (down: the reverse). The formation day is the first close in perfect order. If the order holds on that close and the next four, and the 14-day ADX rose on the fourth (\"ADX pointing upward\"; the 'ideally greater than 20' wish is not applied), buy (sell) at the open of the fifth candle after the formation. Stop = the formation day's low (high), from the entry bar. Exit at the next open after the first close when the perfect order no longer holds (the book's examples exit 'when prices open'). One position at a time; a new trade needs a new formation.";

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
        nt("klhpt_ma_macd", "Moving average MACD combo (50/100 SMA)", "Part 2 Setup 3 (p.34-40)", "klhpt", pips, &["E|\"Once the price has broken above the closest SMA by 10 pips or more, enter long if MACD crosses to positive within the last five bars\""]),
        nt("klhpt_rsi_coaster", "RSI rollercoaster", "Part 2 Setup 4 (p.41-48)", "klhpt", "the stop is 'the swing low' with no definition of the swing, and the first target is half of that risk", &["E|\"RSI reading must be less than 30. Wait for an up candle to form and close with an RSI reading greater than 30. Go long at market on the open of the next candle\""]),
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
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(match id {
        "klhpt_cci_right" => vec![p("len", "CCI length", 20., 5., 50., 1.), p("level", "CCI zone level", 100., 50., 200., 10.)],
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

fn run_cci(x: &Ctx, from: usize, to: usize, n: usize, lv: f64) -> Vec<Trade> {
    let k = cci(x, n);
    let (mut out, mut cur) = (vec![], None::<(Trade, f64, f64, bool, Vec<(usize, f64, f64)>)>); // trade, stop, R, first target done, exits
    // excursion state per side: (in excursion, running extreme, prior peak, entered this excursion)
    let mut st = [(false, f64::NAN, f64::NAN, false), (false, f64::NAN, f64::NAN, false)];
    for i in 1..=to {
        // manage the open trade on bar i (entered on an earlier close)
        if i >= from {
            if let Some((mut t, stop, r, done, mut ex)) = cur.take() {
                let (d, e) = (t.dir, t.legs[0].px);
                let (o, h, l) = (x.o[i], x.h[i], x.l[i]);
                let s = if done { e } else { stop };
                let beyond_stop = |p: f64| if d > 0. { p < s } else { p > s };
                let reach = |tg: f64| if d > 0. { h >= tg } else { l <= tg };
                let (t1, t2) = (e + d * r, e + d * 2. * r);
                let mut closed = None::<(f64, &str)>;
                let mut done2 = done;
                if beyond_stop(o) { closed = Some((o, if done { "breakeven stop after the first target" } else { "stop: signal candle" })); }
                else if beyond_stop(if d > 0. { l } else { h }) { closed = Some((s, if done { "breakeven stop after the first target" } else { "stop: signal candle" })); }
                else {
                    if !done && reach(t1) { ex.push((i, if (d > 0. && o >= t1) || (d < 0. && o <= t1) { o } else { t1 }, 0.5)); done2 = true; }
                    if done2 && reach(t2) { closed = Some((if (d > 0. && o >= t2) || (d < 0. && o <= t2) { o } else { t2 }, "second target (2 x risk)")); }
                }
                match closed {
                    Some((px, why)) => {
                        if ex.is_empty() { t.exit_px = px; } else { ex.push((i, px, 0.5)); t.exit_px = ex.iter().map(|e| e.1 * e.2).sum::<f64>(); t.exits = ex; }
                        t.exit_i = i; t.signal_i = i; t.reason = why.into(); out.push(t);
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
                        cur = Some((Trade { dir: d, legs: vec![Leg { i, px: x.c[i], w: 1. }], exit_i: i, exit_px: x.c[i], reason: String::new(), entry_kind: "close", signal_i: i, open: false, exits: vec![] }, stop, r, false, vec![]));
                    }
                } else if beyond { e.3 = true; }
            } else if e.0 { e.0 = false; e.2 = e.1; }
        }
    }
    if let Some((mut t, _, _, _, mut ex)) = cur {
        if ex.is_empty() { t.exit_px = x.c[to]; } else { ex.push((to, x.c[to], 0.5)); t.exit_px = ex.iter().map(|e| e.1 * e.2).sum::<f64>(); t.exits = ex; }
        t.exit_i = to; t.signal_i = to; t.reason = "open".into(); t.open = true; out.push(t);
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
        "klhpt_cci_right" => run_cci(x, from, to, g("len").round().max(2.) as usize, g("level")),
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
