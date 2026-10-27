//! Angus Jackson, "FWIW some indicators & systems from Angus Jackson" (Omega list post, TradeStation 2000i code).
//! Source: research drive `USB HDD 1/TradeStation Systems & Indicators/Angus Jackson Misc. Indicators/
//! Angus Jackson, various ELA indicators (for TS).rar` -> `Angus Jackson_ various ELA indicators _for TS_.txt`
//! (read with the read-only libarchive reader; COPY PENDING into Books/). Book id "ajmisc". Notes: library/ajmisc/README.md.
//!
//! This module also holds the small TradeStation daily-bar order simulator (`Sim`) and indicator helpers used by the
//! other modules converted from the same RAR batch (tscom, alfar, andro, axsym, barra, ltcoat, pwadcs, gunsl, ginyd, jcurt).
//! It is NOT books::jkbwts::sim. Conventions (TradeStation, documented in each README):
//! - orders are generated at the close of bar i and work on bar i+1 only ("next bar"); "this bar on close" fills at C[i];
//! - market orders fill at the open; a stop or limit already through the open fills at the open (exits before entries);
//! - then the bar runs open -> nearer extreme -> far extreme -> close (tie: high first); a stop fills at its level when the
//!   path crosses it, a limit when the path reaches it; each order fills at most once per bar;
//! - an entry reverses an opposite position (TradeStation Buy / Sell Short); an entry in the direction already held is
//!   ignored (no pyramiding); an exit applies only to a position that was held at the start of the bar.
use crate::connors::{sma, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "ajmisc";
/// Admitted ids (authors/vendor/laneb/SOURCE-CONTRACT.md).
pub const REVIEWED: &[&str] = &["ajmisc_delayed_cbo", "ajmisc_r_osc", "ajmisc_tr_breakout", "ajmisc_turtle", "ajmisc_vol_rev"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

// ------------------------------------------------------------------------------------------------ simulator

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum K { Buy, Short, XL, XS }
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Ty { Mkt, Stop(f64), Lim(f64) }
#[derive(Clone, Copy, Debug)]
pub struct Ord { pub k: K, pub ty: Ty, pub nm: &'static str }
pub fn ord(k: K, ty: Ty, nm: &'static str) -> Ord { Ord { k, ty, nm } }

fn buy_side(k: K) -> bool { matches!(k, K::Buy | K::XS) }

pub struct Sim { pub out: Vec<Trade>, pub cur: Option<Trade>, pub pos: i8 }

impl Default for Sim { fn default() -> Self { Self::new() } }

impl Sim {
    pub fn new() -> Self { Sim { out: vec![], cur: None, pos: 0 } }
    /// TradeStation EntryPrice (0 when flat).
    pub fn ep(&self) -> f64 { self.cur.as_ref().map_or(0., |t| t.legs[0].px) }
    /// Bar index of the current entry.
    pub fn ei(&self) -> Option<usize> { self.cur.as_ref().map(|t| t.legs[0].i) }
    /// TradeStation BarsSinceEntry at the close of bar i (0 when flat).
    pub fn bse(&self, i: usize) -> usize { self.ei().map_or(0, |e| i - e) }

    fn apply(&mut self, i: usize, sig: usize, px: f64, k: K, nm: &str, pos_open: i8, kind: &'static str) -> bool {
        let ok = match k { K::Buy => self.pos <= 0, K::Short => self.pos >= 0, K::XL => self.pos == 1 && pos_open == 1, K::XS => self.pos == -1 && pos_open == -1 };
        if !ok { return false; }
        if let Some(mut t) = self.cur.take() {
            t.exit_i = i; t.exit_px = px; t.signal_i = sig;
            t.reason = if matches!(k, K::XL | K::XS) { nm.to_string() } else { format!("reversed by {nm}") };
            self.out.push(t);
        }
        self.pos = 0;
        if matches!(k, K::Buy | K::Short) {
            let dir = if k == K::Buy { 1. } else { -1. };
            self.pos = dir as i8;
            self.cur = Some(Trade { dir, legs: vec![Leg { i, px, w: 1. }], exit_i: i, exit_px: px, reason: String::new(), entry_kind: kind, signal_i: sig, open: false, exits: vec![] });
        }
        true
    }

    /// "this bar on close" order at bar i.
    pub fn on_close(&mut self, x: &Ctx, i: usize, k: K, nm: &str) -> bool { let p = self.pos; self.apply(i, i, x.c[i], k, nm, p, "close") }

    /// Bar j with the orders generated at the close of bar j-1.
    pub fn bar(&mut self, x: &Ctx, j: usize, ords: &[Ord]) {
        let (o, h, l, c) = (x.o[j], x.h[j], x.l[j], x.c[j]);
        let pos_open = self.pos;
        let sig = j.saturating_sub(1);
        let mut done = vec![false; ords.len()];
        let kind = |t: Ty| match t { Ty::Mkt => "open", Ty::Stop(_) => "stop", Ty::Lim(_) => "limit" };
        // at the open: market orders, then stops/limits already through the open (exits first)
        for pass_exit in [true, false] {
            for (n, q) in ords.iter().enumerate() {
                if done[n] || matches!(q.k, K::XL | K::XS) != pass_exit { continue; }
                let at_open = match q.ty {
                    Ty::Mkt => true,
                    Ty::Stop(lv) => if buy_side(q.k) { lv <= o } else { lv >= o },
                    Ty::Lim(lv) => if buy_side(q.k) { lv >= o } else { lv <= o },
                };
                if at_open && self.apply(j, sig, o, q.k, q.nm, pos_open, kind(q.ty)) { done[n] = true; }
            }
        }
        let path = if h - o <= o - l { [o, h, l, c] } else { [o, l, h, c] };
        for s in path.windows(2) {
            let (a, b) = (s[0], s[1]);
            if a == b { continue; }
            let up = b > a;
            let mut cand: Vec<(f64, usize)> = ords.iter().enumerate().filter(|(n, _)| !done[*n]).filter_map(|(n, q)| {
                let lv = match q.ty { Ty::Mkt => return None, Ty::Stop(v) | Ty::Lim(v) => v };
                let trig = match q.ty { Ty::Stop(_) => buy_side(q.k) == up, _ => buy_side(q.k) != up };
                let inside = if up { lv > a && lv <= b } else { lv < a && lv >= b };
                (trig && inside).then_some((lv, n))
            }).collect();
            cand.sort_by(|p, q| {
                let c = if up { p.0.total_cmp(&q.0) } else { q.0.total_cmp(&p.0) };
                c.then_with(|| (!matches!(ords[p.1].k, K::XL | K::XS)).cmp(&!matches!(ords[q.1].k, K::XL | K::XS)))
            });
            for (lv, n) in cand {
                if !done[n] && self.apply(j, sig, lv, ords[n].k, ords[n].nm, pos_open, kind(ords[n].ty)) { done[n] = true; }
            }
        }
    }

    pub fn done(mut self, x: &Ctx, to: usize) -> Vec<Trade> {
        if let Some(mut t) = self.cur.take() { t.exit_i = to; t.exit_px = x.c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true; self.out.push(t); }
        self.out
    }
}

// ------------------------------------------------------------------------------------------------ indicators

/// Highest of v over n bars ending at i (TradeStation Highest(v, n)); NaN while the window is short.
pub fn highest(v: &[f64], n: usize) -> Vec<f64> { roll(v, n, true) }
pub fn lowest(v: &[f64], n: usize) -> Vec<f64> { roll(v, n, false) }
fn roll(v: &[f64], n: usize, hi: bool) -> Vec<f64> {
    let n = n.max(1);
    (0..v.len()).map(|i| if i + 1 < n { f64::NAN } else {
        let w = &v[i + 1 - n..=i];
        if w.iter().any(|x| x.is_nan()) { f64::NAN } else if hi { w.iter().copied().fold(f64::MIN, f64::max) } else { w.iter().copied().fold(f64::MAX, f64::min) }
    }).collect()
}
/// v[i-k] (TradeStation v[k]); NaN before the start.
pub fn back(v: &[f64], k: usize) -> Vec<f64> { (0..v.len()).map(|i| if i >= k { v[i - k] } else { f64::NAN }).collect() }
/// TradeStation StdDev: population standard deviation over n bars.
pub fn stdev_pop(v: &[f64], n: usize) -> Vec<f64> {
    let n = n.max(1);
    (0..v.len()).map(|i| if i + 1 < n { f64::NAN } else {
        let w = &v[i + 1 - n..=i];
        let m = w.iter().sum::<f64>() / n as f64;
        (w.iter().map(|x| (x - m) * (x - m)).sum::<f64>() / n as f64).sqrt()
    }).collect()
}
/// TradeStation TrueHigh / TrueLow / TrueRange (first bar: high, low, range).
pub fn true_high(x: &Ctx) -> Vec<f64> { (0..x.c.len()).map(|i| if i == 0 { x.h[0] } else { x.h[i].max(x.c[i - 1]) }).collect() }
pub fn true_low(x: &Ctx) -> Vec<f64> { (0..x.c.len()).map(|i| if i == 0 { x.l[0] } else { x.l[i].min(x.c[i - 1]) }).collect() }
pub fn true_range(x: &Ctx) -> Vec<f64> { let (th, tl) = (true_high(x), true_low(x)); th.iter().zip(&tl).map(|(a, b)| a - b).collect() }
/// TradeStation AvgTrueRange(n) = simple average of TrueRange.
pub fn avg_true_range(x: &Ctx, n: usize) -> Vec<f64> { sma(&true_range(x), n.max(1)) }
/// TradeStation "1 point" = 1 / PriceScale = 0.01 for index and stock quotes (help.tradestation.com, Point).
pub const POINT: f64 = 0.01;

/// First bar from which all the series are finite (the strategy's MaxBarsBack).
pub fn first_ok(n: usize, from: usize, series: &[&[f64]]) -> usize {
    (from..n).find(|&i| series.iter().all(|s| s[i].is_finite())).unwrap_or(n)
}

// ------------------------------------------------------------------------------------------------ strategies

const DCBO: &str = "ajmisc_delayed_cbo";
const TRBO: &str = "ajmisc_tr_breakout";
const ROSC: &str = "ajmisc_r_osc";
const TURT: &str = "ajmisc_turtle";
const VREV: &str = "ajmisc_vol_rev";

const NOTE: &str = "Angus Jackson's Omega-list post of TradeStation 2000i code (no market, bar size or results given). TS2000i syntax: \"Sell\" opens a short and reverses a long; \"ExitLong/ExitShort\" close. \
Run on daily bars of the cash indices (instrument swap; the post names no market). Fills: vault TradeStation daily-bar convention (see ajmisc.rs header). India: instrument swap; index shorts hypothetical.";

fn def(id: &'static str, name: &'static str, status: Status, rules: &'static [&'static str], notes: &'static str) -> StrategyDef {
    StrategyDef { id, name, group: "SYSTEMS (Angus Jackson ELA post)", session: 0, dir: "Long/Short", status, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None }
}

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        def(DCBO, "Delayed Channel Breakout (13 bars, 6-bar delay)", Status::Live, &[
            "F|\"HighBOPrice = Highest(high,Len)[1]; LowBOPrice = Lowest(low,Len)[1]; HighBO = high > HighBOPrice; LowBO = low < LowBOPrice\" (Len 13)",
            "E|\"if HighBO[X_Bar] then Buy at HighBOPrice[X_Bar] + 1 point stop\" (X_Bar 6)",
            "E|\"if LowBO[X_Bar] then Sell at LowBOPrice[X_Bar] - 1 point stop\"",
            "X|Stop and reverse: no other exit in the code."],
            "The comment says the system enters only if 'we are still above/below the BO price' after X_Bars; the code expresses that only through the stop at the old breakout price, so the code is used. 1 point = 1/PriceScale = 0.01. "),
        def(TRBO, "True Range Breakout (range below its 20-bar average)", Status::Live, &[
            "F|\"Condition1 = range < Average(range,Period)\" (Period 20)",
            "E|\"If Condition1 then Buy at Open of tomorrow + range Stop\"",
            "E|\"If Condition1 then Sell at Open of tomorrow - range Stop\"",
            "X|Stop and reverse: no other exit in the code."], NOTE),
        def(ROSC, "R Oscillator (30-bar MA slope + PercentR(10) 20/80)", Status::Live, &[
            "E|\"IF CurrentBar > 1 and Average(Close,AvgLen) > Average(Close,AvgLen)[1] and PercentR(PrctRLen) < BuyLvl Then Buy on Close\" (AvgLen 30, PrctRLen 10, BuyLvl 20)",
            "E|\"IF CurrentBar > 1 and Average(Close,AvgLen) < Average(Close,AvgLen)[1] and PercentR(PrctRLen) > SellLvl Then Sell on Close\" (SellLvl 80)",
            "X|Stop and reverse at the close: no other exit in the code."],
            "PercentR = TradeStation's function: 100 x (C - lowest low) / (highest high - lowest low) over 10 bars (help.tradestation.com, PercentR: 100 minus Williams' %R)."),
        def(TURT, "Turtle (20-close breakout on the close, 10-close short exit)", Status::Live, &[
            "E|\"If Close > Highest(Close,EntryLen)[1] then buy(\"LEntry\") on Close\" (EntryLen 20)",
            "E|\"If Close < Lowest(Close,EntryLen)[1] then Sell(\"SEntry\") on Close\"",
            "X|\"If Close > Highest(Close,ExitLen)[1] then ExitShort from entry(\"SEntry\") on Close\" (ExitLen 10)",
            "X|\"If Close < Lowest(Close,EntryLen)[1] then ExitLong from entry(\"LEntry\") on Close\" (the code uses EntryLen here, not ExitLen: kept literal)"],
            "Literal code: the long exit uses EntryLen (20), so a long ends only when the short entry fires; p.long_exit_len can set another length (non-default)."),
        def(VREV, "Volatility Reversals (open +/- 0.65 x 5-bar ATR)", Status::Live, &[
            "F|\"Value1 = Ent * AvgTrueRange(lkbk)\" (lkbk 5, Ent .65)",
            "E|\"Buy next bar at Open of next bar + Value1 stop\"",
            "E|\"Sell next bar at Open of next bar - Value1 stop\"",
            "X|Stop and reverse: no other exit in the code."],
            "AvgTrueRange = simple average of TradeStation TrueRange (help.tradestation.com). "),
        def("ajmisc_dual_tf_ma", "Dual time frame moving average (13-bar XAverage on data1 and data2)", Status::NotTestable("data2 is a second time frame the post never names, so the confirming series is unknown"), &[
            "E|\"IF XAverage(Price,Length) > XAverage(Price,Length)[1] and XAverage(Price,Length) of data2 > XAverage(Price,Length)[1] of data2 then Buy this bar on Close\"",
            "X|\"IF XAverage(Price,Length) < XAverage(Price,Length)[1] then ExitLong this bar on Close\"; short mirror"], NOTE),
        def("ajmisc_divergence", "Divergence (RSI(14) bullish/bearish divergence, 5-bar breakout stop)", Status::NotTestable("BullishDivergence / BearishDivergence are TradeStation library functions whose code is not in the source"), &[
            "E|\"IF BullishDivergence(Close,Osc,2,30) = 1 Then Buy Highest(High,5) + 1 point Stop\"; \"IF BearishDivergence(Close,Osc,2,30) = 1 Then Sell Lowest(Low,5) - 1 point Stop\""], NOTE),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        DCBO => ("ajmisc_dcbo", "Delayed Channel Breakout (Angus Jackson)", "code"),
        TRBO => ("ajmisc_trbo", "True Range Breakout (Angus Jackson)", "code"),
        ROSC => ("ajmisc_rosc", "R Oscillator (Angus Jackson)", "code"),
        TURT => ("ajmisc_turtle", "Turtle (Angus Jackson)", "code"),
        VREV => ("ajmisc_vrev", "Volatility Reversals (Angus Jackson)", "code"),
        "ajmisc_dual_tf_ma" => ("ajmisc_dual_tf_ma", "Dual time frame MA (Angus Jackson)", "code"),
        "ajmisc_divergence" => ("ajmisc_divergence", "Divergence (Angus Jackson)", "code"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(match id {
        DCBO => vec![p("len", "Len (channel bars)", 13., 5., 60., 1.), p("x_bar", "X_Bar (delay bars)", 6., 0., 20., 1.)],
        TRBO => vec![p("period", "Period (average range)", 20., 5., 60., 1.)],
        ROSC => vec![p("avg_len", "AvgLen", 30., 5., 100., 1.), p("pr_len", "PrctRLen", 10., 3., 40., 1.), p("buy_lvl", "BuyLvl", 20., 5., 50., 5.), p("sell_lvl", "SellLvl", 80., 50., 95., 5.)],
        TURT => vec![p("entry_len", "EntryLen", 20., 5., 100., 1.), p("exit_len", "ExitLen (short exit)", 10., 2., 50., 1.), p("long_exit_len", "Long exit length (code: EntryLen)", 20., 2., 100., 1.)],
        VREV => vec![p("lkbk", "lkbk (ATR bars)", 5., 1., 30., 1.), p("ent", "Ent (ATR multiple)", 0.65, 0.1, 2., 0.05)],
        _ => return None,
    })
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if from >= to || to >= x.r.len() { return None; }
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d);
    let n = x.c.len();
    let mut s = Sim::new();
    match id {
        DCBO => {
            let (len, xb) = (g("len", 13.).max(1.) as usize, g("x_bar", 6.).max(0.) as usize);
            let hb = back(&highest(&x.h, len), 1);
            let lb = back(&lowest(&x.l, len), 1);
            let st = first_ok(n, from, &[&back(&hb, xb)]);
            for i in st..to {
                let k = i - xb;
                let mut q = vec![];
                if x.h[k] > hb[k] { q.push(ord(K::Buy, Ty::Stop(hb[k] + POINT), "HighBO stop")); }
                if x.l[k] < lb[k] { q.push(ord(K::Short, Ty::Stop(lb[k] - POINT), "LowBO stop")); }
                s.bar(x, i + 1, &q);
            }
        }
        TRBO => {
            let rg: Vec<f64> = (0..n).map(|i| x.h[i] - x.l[i]).collect();
            let ar = sma(&rg, g("period", 20.).max(1.) as usize);
            for i in first_ok(n, from, &[&ar])..to {
                let mut q = vec![];
                if rg[i] < ar[i] {
                    let o1 = x.o[i + 1];
                    q.push(ord(K::Buy, Ty::Stop(o1 + rg[i]), "open + range stop"));
                    q.push(ord(K::Short, Ty::Stop(o1 - rg[i]), "open - range stop"));
                }
                s.bar(x, i + 1, &q);
            }
        }
        ROSC => {
            let ma = sma(&x.c, g("avg_len", 30.).max(1.) as usize);
            let m = g("pr_len", 10.).max(1.) as usize;
            let (hh, ll) = (highest(&x.h, m), lowest(&x.l, m));
            let pr: Vec<f64> = (0..n).map(|i| if hh[i] - ll[i] != 0. { 100. * (x.c[i] - ll[i]) / (hh[i] - ll[i]) } else { 0. }).collect();
            let (bl, sl) = (g("buy_lvl", 20.), g("sell_lvl", 80.));
            for i in first_ok(n, from, &[&back(&ma, 1), &hh])..=to {
                if ma[i] > ma[i - 1] && pr[i] < bl { s.on_close(x, i, K::Buy, "Buy on close"); }
                if ma[i] < ma[i - 1] && pr[i] > sl { s.on_close(x, i, K::Short, "Sell on close"); }
            }
        }
        TURT => {
            let (en, ex, lx) = (g("entry_len", 20.).max(1.) as usize, g("exit_len", 10.).max(1.) as usize, g("long_exit_len", 20.).max(1.) as usize);
            let (he, le, hx, llx) = (back(&highest(&x.c, en), 1), back(&lowest(&x.c, en), 1), back(&highest(&x.c, ex), 1), back(&lowest(&x.c, lx), 1));
            let st = first_ok(n, from, &[&he, &hx, &llx]);
            for i in st..=to {
                let c = x.c[i];
                if c > he[i] { s.on_close(x, i, K::Buy, "LEntry on close"); }
                if c < le[i] { s.on_close(x, i, K::Short, "SEntry on close"); }
                if c > hx[i] { s.on_close(x, i, K::XS, "ExitShort on close (10-close high)"); }
                if c < llx[i] { s.on_close(x, i, K::XL, "ExitLong on close (20-close low)"); }
            }
        }
        VREV => {
            let atr = avg_true_range(x, g("lkbk", 5.).max(1.) as usize);
            let e = g("ent", 0.65);
            for i in first_ok(n, from, &[&atr])..to {
                let (o1, v) = (x.o[i + 1], e * atr[i]);
                s.bar(x, i + 1, &[ord(K::Buy, Ty::Stop(o1 + v), "open + .65 ATR stop"), ord(K::Short, Ty::Stop(o1 - v), "open - .65 ATR stop")]);
            }
        }
        _ => return None,
    }
    Some(s.done(x, to))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn ctx(bars: &[(f64, f64, f64, f64)]) -> Vec<Row> {
        bars.iter().enumerate().map(|(i, &(o, h, l, c))| Row { ticker: "T".into(), date: format!("2000-01-{:02}", i + 1), open: o, high: h, low: l, close: c, volume: None }).collect()
    }
    #[test]
    fn sim_exit_then_reverse_and_gap() {
        let rows = ctx(&[(10., 10., 10., 10.), (10., 14., 9.9, 13.5), (8., 8.5, 7., 7.5)]);
        let x = Ctx::new(&rows, &rows, &[]);
        let mut s = Sim::new();
        s.on_close(&x, 0, K::Short, "SE");
        // short; bar opens 10, high first to 14: cover stop at 12, then buy stop at 13 reverses to long
        s.bar(&x, 1, &[ord(K::Buy, Ty::Stop(13.), "LE"), ord(K::XS, Ty::Stop(12.), "SX"), ord(K::Short, Ty::Stop(5.), "SE2")]);
        assert_eq!((s.pos, s.out.len(), s.out[0].exit_px, s.ep()), (1, 1, 12., 13.));
        // long; gap below a sell stop fills at the open; an exit is not applied to a position opened this bar
        s.bar(&x, 2, &[ord(K::XL, Ty::Stop(9.), "LX"), ord(K::Short, Ty::Stop(7.2), "SE"), ord(K::XS, Ty::Stop(7.6), "SX")]);
        assert_eq!((s.out[1].exit_px, s.pos, s.ep()), (8., -1, 7.2));
    }
}
