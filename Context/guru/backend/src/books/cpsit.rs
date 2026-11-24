//! Capstone "Situational System" (S&P daily; research share Trading Courses/Capstone ... _Older Systems/Situational System for TS/System Volitality.html,
//! printed EasyLanguage). Book id `cpsit`. Lane K2 (24 Nov 2026, tier 5 row I-T125). Independent check authors/conv-1123/k2w/qi/cpsit_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::tselhs::Book;

pub const REVIEWED: &[&str] = &["cpsit_situational", "cpsit_trend2"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "A|[Declared readings, lane K2, 4] (1) The printed code buys 5 S&P points above the signal close; on Indian prices this is the same fraction of price: 0.4% of the signal close (5 points on an S&P near 1,200), a parameter. (2) The printed SetStopLoss(7000) is $7,000 per S&P contract = 28 points at $250 a point, about 2.5% of an S&P near 1,100: read as a 2.5% stop from the entry price (a parameter), checked from the bar after entry and filled at the stop or at a gap open beyond it. (3) The page prints the short side only as 'mirror'; the mirror of the long conditions is used (two up bars, close in the top 20% of its range, open in the bottom 30%). (4) A stop order that the next open gaps through fills at that open. The page text says 'close in the lower 80%' but the code (C < H - .8 x Range, the lowest 20%) controls.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "cpsit_situational", name: "Capstone Situational System: two down closes in the low 20% of range with opens in the top 30%, buy stop; exit at the first profitable close", group: "Capstone situational system (EasyLanguage on the page)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "cpsit", rules: &[
            "E|Long: C < O and C[1] < O[1] and C < H - .8 x Range and O > L + .7 x Range and the same on the previous bar (C[1] < H[1] - .8 x Range[1], O[1] > L[1] + .7 x Range[1]): Buy next bar at C + 5 Stop. Short: the mirror, Sell Short next bar at C - 5 Stop",
            "X|If MarketPosition = 1 and PositionProfit > 0 then Sell this bar on close (short: cover on the first profitable close, including the entry bar); the page also sets a $7,000 stop (read as 2.5%, reading 2)",
            NOTE], notes: NOTE, claim: None },
    StrategyDef { id: "cpsit_trend2", name: "Capstone Trend Swing 2: strong-close bar, buy next open, 10 x 12-bar average range stop, exit at a later close whose open is beyond the entry", group: "Capstone situational system (EasyLanguage on the page)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "cpsit", rules: &[
            "E|Long signal bar: (C - O) >= 0.3 x (H - L) and (H - C) <= 0.3 x (H - L) and (O - L) <= 0.3 x (H - L); buy at the next bar. Short: (O - C) >= 0.3R, (H - O) <= 0.3R, (C - L) <= 0.3R (Trend2.txt in Code.rar)",
            "X|Initial stop 10 x Average(Range, 12) points from the entry; exit on a later bar at its close if that bar's open is beyond the entry price (above for longs, below for shorts) and the bar is not again a signal bar of the same side",
            NOTE2], notes: NOTE2, claim: None }]
}
const NOTE2: &str = "A|[Declared readings, lane K2, 4] (1) The next-bar market order is filled at the next open. (2) The close exit is evaluated from the bar after the entry bar. (3) The short side is the exact mirror of the long conditions as printed in the code. (4) The stop fills at the stop price, or at the open when a bar gaps through it. Signals while a position is open are ignored.";
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { reviewed(id).then_some(("cpsit", "Capstone situational system", "Buy stop after two weak closes")) }
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    (id == "cpsit_situational").then(|| vec![ParamDef { key: "stop_pct", label: "Protective stop, % of entry ($7,000 / $250 per point = 28 points, about 2.5% of an S&P near 1,100; 0 = none)", def: 2.5, min: 0., max: 10., step: 0.25 }, ParamDef { key: "off_pct", label: "Stop offset above/below the signal close, % (5 S&P points on about 1,200 = 0.4)", def: 0.4, min: 0., max: 2., step: 0.05 }])
}

/// +1 long signal, -1 short signal, 0 none, on bar i (needs bar i-1).
pub fn signal(x: &Ctx, i: usize) -> i8 {
    if i == 0 { return 0; }
    let f = |k: usize| { let r = x.h[k] - x.l[k]; (x.o[k], x.c[k], x.h[k], x.l[k], r) };
    let (o1, c1, h1, l1, r1) = f(i - 1);
    let (o0, c0, h0, l0, r0) = f(i);
    let lo = |o: f64, c: f64, h: f64, l: f64, r: f64| c < o && c < h - 0.8 * r && o > l + 0.7 * r;
    let hi = |o: f64, c: f64, h: f64, l: f64, r: f64| c > o && c > l + 0.8 * r && o < h - 0.7 * r;
    if lo(o0, c0, h0, l0, r0) && lo(o1, c1, h1, l1, r1) { 1 } else if hi(o0, c0, h0, l0, r0) && hi(o1, c1, h1, l1, r1) { -1 } else { 0 }
}

/// Trend Swing 2 signal on bar i: +1 / -1 / 0.
pub fn t2_signal(x: &Ctx, i: usize) -> i8 {
    let r = x.h[i] - x.l[i];
    if (x.c[i] - x.o[i]) >= 0.3 * r && (x.h[i] - x.c[i]) <= 0.3 * r && (x.o[i] - x.l[i]) <= 0.3 * r && r > 0. { 1 }
    else if (x.o[i] - x.c[i]) >= 0.3 * r && (x.h[i] - x.o[i]) <= 0.3 * r && (x.c[i] - x.l[i]) <= 0.3 * r && r > 0. { -1 } else { 0 }
}

fn run_t2(x: &Ctx, from: usize, to: usize) -> Vec<Trade> {
    let mut pos = Book::new();
    let mut stop_pts = 0.0;
    for t in from..=to {
        if pos.side() == 0. && t > from + 12 {
            let s = t2_signal(x, t - 1);
            if s != 0 {
                pos.enter(s as f64, t, x.o[t], "open");
                stop_pts = 10. * (t - 12..t).map(|k| x.h[k] - x.l[k]).sum::<f64>() / 12.;
            }
        }
        if pos.side() != 0. && pos.entry_bar().is_some_and(|e| e < t) {
            let d = pos.side(); let lvl = pos.entry_px() - d * stop_pts;
            if (d > 0. && x.l[t] <= lvl) || (d < 0. && x.h[t] >= lvl) { pos.exit(t, if d > 0. { x.o[t].min(lvl) } else { x.o[t].max(lvl) }, "stop"); }
            else if (x.o[t] - pos.entry_px()) * d > 0. && t2_signal(x, t) as f64 != d { pos.exit(t, x.c[t], "close beyond entry"); }
        }
    }
    pos.done(to, x.c[to])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    if id == "cpsit_trend2" { return Some(run_t2(x, from, to)); }
    let off = p.get("off_pct").copied().unwrap_or(0.4) / 100.;
    let stop = p.get("stop_pct").copied().unwrap_or(2.5) / 100.;
    let mut pos = Book::new();
    let mut pend: Option<(f64, f64)> = None; // (side, stop level) for this bar
    for t in from..=to {
        if let Some((d, lvl)) = pend.take() {
            if pos.side() == 0. {
                let hit = if d > 0. { x.h[t] >= lvl } else { x.l[t] <= lvl };
                if hit { let px = if d > 0. { x.o[t].max(lvl) } else { x.o[t].min(lvl) }; pos.enter(d, t, px, "stop"); }
            }
        }
        if pos.side() != 0. && stop > 0. && pos.entry_bar().is_some_and(|e| e < t) {
            let d = pos.side(); let lvl = pos.entry_px() * (1. - d * stop);
            if (d > 0. && x.l[t] <= lvl) || (d < 0. && x.h[t] >= lvl) { let px = if d > 0. { x.o[t].min(lvl) } else { x.o[t].max(lvl) }; pos.exit(t, px, "stop"); }
        }
        if pos.side() != 0. && (x.c[t] - pos.entry_px()) * pos.side() > 0. { pos.exit(t, x.c[t], "first profitable close"); }
        if t < to && pos.side() == 0. && t > from {
            match signal(x, t) { 1 => pend = Some((1., x.c[t] * (1. + off))), -1 => pend = Some((-1., x.c[t] * (1. - off))), _ => {} }
        }
    }
    Some(pos.done(to, x.c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    /// hand-run dump for the pandas reference: CPSIT_CSV=<idx_data.csv> CPSIT_OUT=<file> cargo test --release cpsit_dump -- --ignored
    #[test]
    #[ignore]
    fn cpsit_dump() {
        use std::io::Write;
        let data = crate::load_csv(&std::env::var("CPSIT_CSV").unwrap()).unwrap();
        let mut f = std::io::BufWriter::new(std::fs::File::create(std::env::var("CPSIT_OUT").unwrap()).unwrap());
        writeln!(f, "id,ticker,dir,entry_date,entry_px,exit_date,exit_px,open").unwrap();
        for (t, rows) in data.iter() {
            let x = Ctx::new(rows, rows, &[]);
            for id in REVIEWED { for z in run(id, &x, 0, rows.len() - 1, &Params::new()).unwrap() {
                writeln!(f, "{id},{t},{},{},{},{},{},{}", z.dir, rows[z.legs[0].i].date, z.legs[0].px, rows[z.exit_i].date, z.exit_px, z.open).unwrap();
            } }
        }
    }
    #[test]
    fn long_after_two_weak_bars_exits_on_profitable_close() {
        use crate::Row;
        let b = |d: &str, o: f64, h: f64, l: f64, c: f64| Row { ticker: "T".into(), date: d.into(), open: o, high: h, low: l, close: c, volume: None };
        // two bars: open in the top 30%, close in the lowest 20% (down bars), then a bar that trades above close x 1.004 and closes higher
        let r = vec![b("1", 100., 101., 90., 99.), b("2", 109., 110., 100., 101.), b("3", 109., 110., 100., 101.), b("4", 101., 106., 100.5, 104.), b("5", 104., 105., 103., 104.)];
        let x = Ctx::new(&r, &r, &[]);
        assert_eq!(signal(&x, 2), 1);
        let t = run("cpsit_situational", &x, 0, 4, &Params::new()).unwrap();
        assert_eq!(t.len(), 1);
        assert_eq!(t[0].legs[0].i, 3);
        assert!(t[0].legs[0].px >= 101. * 1.004 - 1e-9);
        assert_eq!(t[0].exit_i, 3);
    }
}

// Instrument admission: Capstone situational system 2026-11-24 (lane K2, tier 5).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "cpsit" && reviewed(def.id),
    label: "The page gives the daily rule on the S&P 500 with no printed results. Run on the five NSE index series the independent reference covers (the gate row allows only those, R4 #378); the 5-point offset is read as 0.4% of price (declared).",
    basis: "Capstone Situational System (System Volitality.html, EasyLanguage on the page); independent check authors/conv-1123/k2w/qi/cpsit_ref.py",
};
