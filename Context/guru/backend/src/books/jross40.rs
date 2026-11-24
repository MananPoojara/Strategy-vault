//! Joe Ross, "Andy's E-mini Bar: Day Trading on the Mini S&P 40 Minutes" (Trading Educators, 2005; research share Trading Courses,
//! Seminars,Videos/Joe Ross - Andy's EMini Bar - 40 Min System/jross40min.pdf, 'Trading Rules' and the worked examples). Book id `jross40`.
//! Two 40-minute 'trigger bars' at the start of the day; limit orders at the bar's low (buy) and high (sell) for the next 40 minutes;
//! target 8 ticks inside the opposite extreme, stop 20 ticks from the entry, time stop. Real 1-minute bars. Lane K4 (24 Nov 2026, R4 #397).
//! Independent pandas reference: authors/code-1123/ref/jross40_ref.py.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};
use crate::intraday::{fill_px, touched, Bar1, Level};

pub const BOOK: &str = "jross40";
const REF: f64 = 1460.0; // 1 S&P point = 1/1460 of the trigger bar's close (the one scaling rule, as utz)
const BAR: u16 = 40;
const TIME_STOP: u16 = 220; // minutes after the session's first minute (12:10 pm CT after the 8:30 am CT open)

const NOTE: &str = "[Declared readings] Joe Ross, Andy's E-mini Bar 40 Minutes (2005), Trading Rules and Examples. Trigger Bar #1 = the first 40 minutes of the session, Trigger Bar #2 = the next 40 minutes. After a bar closes, if its high - low is at least 10 ticks (2.5 S&P points), a buy limit at its low and a sell limit at its high are live for the next 40 minutes (bar 1: until the end of bar 2; bar 2: until 80 minutes later); the first filled cancels the other. Exit: long = sell limit 8 ticks below the trigger bar's high, stop 20 ticks below the entry (short = buy limit 8 ticks above its low, stop 20 ticks above); the target and stop cancel each other; no position is carried past the time stop. No pyramiding: a same-direction fill from bar 2 while in the trade is ignored; an opposite fill from bar 2 reverses (checked on the printed example: bar 1 low 1217.25 / high 1220.00 -> short 1220.00, target 1217.25 + 8 ticks = 1219.25, stop 1225.00). Four labelled readings: (1) the bars are measured from the first minute of the session (the book's 8:10 bar start is 20 minutes before the 8:30 CT cash open; the NSE and US files have no pre-open minutes); (2) the time stop is 220 minutes after the first minute (12:10 pm CT) and exits at that minute's open; (3) one scaling rule, 1 S&P point = 1/1460 of the trigger bar's close (10 ticks, 8 ticks and 20 ticks are 2.5, 2.0 and 5.0 points); (4) a minute that touches two live levels uses the level nearer its open, and exits are tested from the minute after the entry minute.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "jross40_bars", name: "Andy's E-mini Bar: two 40-minute trigger bars, limit entries at the bar extreme, 8-tick target, 20-tick stop", group: "Joe Ross · Andy's E-mini Bar 40 Minutes", session: 0,
        dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
        rules: &[
            "F|\"We use a 40-minute chart ... Trigger Bar #1 is the bar from 8:10 am till 8:50 am ... Trigger Bar #2 is the bar from 8:50 am till 9:30 am\"; the bar's high - low \"has to be at least 10 ticks\" (Trading Rules)",
            "E|\"Long Signal: Buy at the low of the Trigger Bar using a Limit Order - wait until the Trigger Bar is finished ... Short Signal: Sell at the high ... If your buy order gets filled cancel your sell order\"; no entries after the next bar (9:30 am for bar 1, 10:10 am for bar 2)",
            "X|Long: \"Place a sell limit order 8 ticks below the high of the Trigger Bar ... a sell stop market order 20 ticks below your entry\" (short mirrored); \"Time Stop ... If not filled by 12:10 pm U.S. Central Time, exit the trade via a market order\"",
            "A|\"NO PYRAMIDING! As long as you are long on Trigger Bar #1 you don't go long on Trigger Bar #2 ... Allow entries on Trigger Bar #2 to reverse your open position!\""],
        notes: NOTE, claim: None }]
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    if id != "jross40_bars" { return None; }
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(vec![p("path", "1 = real 1-minute bars (the only path; days without minutes make no trade)", 1., 1., 1., 1.),
              p("minrange", "Minimum trigger bar range in S&P points at the 1460 reference (printed: 10 ticks = 2.5)", 2.5, 0.5, 6., 0.25),
              p("target", "Target distance inside the opposite extreme, S&P points (printed: 8 ticks = 2)", 2., 0.5, 6., 0.25),
              p("stop", "Stop distance from the entry, S&P points (printed: 20 ticks = 5)", 5., 1., 12., 0.25)])
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == "jross40_bars").then_some(("jross40_bars", "Andy's E-mini Bar 40 Minutes", "two 40-minute trigger bars, 8-tick target, 20-tick stop"))
}

struct Pos { dir: f64, entry: f64, target: f64, stop: f64, from: usize }

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if id != "jross40_bars" { return None; }
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d);
    let (mr, tg, sp) = (g("minrange", 2.5), g("target", 2.), g("stop", 5.));
    let mut out = Vec::new();
    for j in from..=to.min(x.r.len().saturating_sub(1)) {
        let Some(b) = crate::intraday_w2b::day(x, j) else { continue };
        let Some(first) = b.first() else { continue };
        let m0 = first.m;
        let bar = |k: u16| -> Vec<Bar1> { b.iter().copied().filter(|z| z.m >= m0 + k * BAR && z.m < m0 + (k + 1) * BAR).collect() };
        // per trigger bar: (live from, live until, long level, short level, target for long, target for short, scale)
        let mut orders: Vec<(u16, u16, f64, f64, f64, f64, f64)> = vec![];
        for k in 0..2u16 {
            let z = bar(k);
            if z.is_empty() || b.last().map_or(0, |l| l.m) + 1 < m0 + (k + 1) * BAR { continue; }
            let hi = z.iter().map(|q| q.h).fold(f64::MIN, f64::max);
            let lo = z.iter().map(|q| q.l).fold(f64::MAX, f64::min);
            let sc = z[z.len() - 1].c / REF;
            if hi - lo < mr * sc { continue; }
            orders.push((m0 + (k + 1) * BAR, m0 + (k + 2) * BAR, lo, hi, hi - tg * sc, lo + tg * sc, sc));
        }
        let mut pos: Option<Pos> = None;
        let mut done = vec![false; orders.len()]; // the filled order cancels its twin
        let push = |out: &mut Vec<Trade>, q: &Pos, px: f64, why: &str| out.push(Trade { dir: q.dir, legs: vec![Leg { i: j, px: q.entry, w: 1.0 }], exit_i: j, exit_px: px, reason: why.into(), entry_kind: "limit", signal_i: j, open: false, exits: vec![] });
        for (ix, m) in b.iter().enumerate() {
            if m.m < m0 + BAR { continue; }
            if m.m >= m0 + TIME_STOP { if let Some(q) = pos.take() { push(&mut out, &q, m.o, "time stop"); } break; }
            // candidate levels this minute: (kind 0 entry long, 1 entry short, 2 target, 3 stop, level, up, order index)
            let mut cand: Vec<(u8, f64, bool, usize)> = vec![];
            for (oi, o) in orders.iter().enumerate() {
                if m.m >= o.0 && m.m < o.1 && !done[oi] {
                    cand.push((0, o.2, false, oi)); cand.push((1, o.3, true, oi));
                }
            }
            if let Some(q) = &pos {
                if q.from < ix { cand.push((2, q.target, q.dir > 0., 0)); cand.push((3, q.stop, q.dir < 0., 0)); }
            }
            let mut hit: Vec<&(u8, f64, bool, usize)> = cand.iter().filter(|c| touched(m, Level { up: c.2, px: c.1 })).collect();
            if hit.is_empty() { continue; }
            hit.sort_by(|a, c| (a.1 - m.o).abs().total_cmp(&(c.1 - m.o).abs()));
            let c = *hit[0];
            let fp = fill_px(m, Level { up: c.2, px: c.1 });
            match c.0 {
                2 | 3 => { if let Some(q) = pos.take() { push(&mut out, &q, fp, if c.0 == 2 { "target" } else { "stop" }); } }
                _ => {
                    let dir = if c.0 == 0 { 1.0 } else { -1.0 };
                    if pos.as_ref().is_some_and(|q| q.dir == dir) { done[c.3] = true; continue; } // no pyramiding (the order is used up)
                    if let Some(q) = pos.take() { push(&mut out, &q, fp, "reversed"); }
                    done[c.3] = true;
                    let o = orders[c.3];
                    let (target, stop) = if dir > 0. { (o.4, fp - sp * o.6) } else { (o.5, fp + sp * o.6) };
                    pos = Some(Pos { dir, entry: fp, target, stop, from: ix });
                }
            }
        }
        if let Some(q) = pos.take() { let lc = b[b.len() - 1].c; push(&mut out, &q, lc, "day end"); }
    }
    Some(out)
}
