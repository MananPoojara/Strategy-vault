//! Omega Research, "System/Strategy Trading and Development Club" (STAD), Vol. 5-13 (1998-2000), incl. Vol. 13
//! "Strategies Revisited". Book id `stad2` (Vol. 1-4 and two systems of Vol. 7/10 are in `stad`).
//! Source copies: Books/Omega Research/STAD05..13.pdf. Transcription, testability table, verification:
//! library/stad2/README.md.
//!
//! Every Live strategy is a literal port of the printed EasyLanguage (code wins over prose; a volume's
//! "Volume in Review" errata are applied; for TradeStation 2000i strategies the Strategy Inputs table wins over the
//! signal's own input defaults, because the strategy passes its inputs to the signal). Strategies are evaluated
//! the way the `stad` module evaluates TradeStation code on daily bars (same order engine, copied here because
//! `stad.rs` must not be modified):
//! * code runs at each bar's close from MaxBarsBack; "this bar on close" fills at that close; "next bar" orders
//!   live for the next bar only; MarketPosition/EntryPrice read inside the code are the values before this bar's
//!   close orders fill;
//! * intrabar path (STAD Vol 5 ch.1): open nearer the low -> O-L-H-C, else O-H-L-C (tie -> O-L-H-C); 10% bounce after
//!   each stop/limit fill; a stop or limit already satisfied at the open fills at the open;
//! * Buy/Sell reverse; same-direction entries are ignored unless the strategy pyramids (then each entry adds one leg,
//!   up to the strategy's maximum; EntryPrice/BarsSinceEntry refer to the first entry - assumption);
//!   "N contracts total" exits close part of the position (Trade.exits);
//! * built-in stops (TS2000i Set* reserved words and the TS4 Format-dialog stops they replaced): stop loss and profit
//!   target sit at entry -/+ amount from the entry bar on; breakeven, $ trailing and % trailing are evaluated at
//!   each close from the position's maximum favourable excursion (bar highs/lows, entry bar included) and act on the
//!   next bar (the BreakEven signal code printed in Vol 8 p.110). Dollar amounts become points through the book's
//!   own test instrument: points = $ / (BigPointValue x quantity), both exposed as parameters.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "stad2";
/// Admitted ids (authors/stad/admission/SOURCE-CONTRACT.md).
pub const REVIEWED: &[&str] = &["stad5_stochob", "stad5_dsdb", "stad6_adxmac", "stad6_mapairs", "stad6_mrtr", "stad6_fps", "stad6_parvol", "stad6_vwm", "stad7_dma", "stad7_stsr", "stad7_masr", "stad7_trbo", "stad8_escalator", "stad8_jib", "stad8_ryanshope", "stad9_dyf", "stad9_nohurry", "stad9_obvrev", "stad9_redrover", "stad9_skinny", "stad9_swinger", "stad9_trafficjam", "stad10_serendipity", "stad10_eightball", "stad10_goinstyle", "stad10_wmts", "stad10_refdev", "stad11_highfive", "stad11_bullseye", "stad11_nighttrain", "stad11_trendscore", "stad11_gapper", "stad12_firstprize", "stad12_inthezone", "stad12_jailbreak", "stad12_lrcross", "stad13_ochist", "stad13_lrm", "stad12_mist2", "stad12_musketeers", "stad12_springs", "stad12_ntw", "stad13_mrts", "stad11_symtri"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

// ------------------------------------------------------------------ TradeStation functions

fn sma(v: &[f64], n: usize) -> Vec<f64> {
    let mut out = vec![f64::NAN; v.len()];
    if n == 0 { return out; }
    for i in n - 1..v.len() { out[i] = v[i + 1 - n..=i].iter().sum::<f64>() / n as f64; }
    out
}
/// XAverage: alpha 2/(n+1), seeded with the first value (NaN inputs are carried as the previous value).
fn xavg(v: &[f64], n: usize) -> Vec<f64> {
    let a = 2.0 / (n as f64 + 1.0);
    let mut out = vec![f64::NAN; v.len()];
    for i in 0..v.len() {
        out[i] = if i == 0 || !out[i - 1].is_finite() { v[i] } else if v[i].is_finite() { out[i - 1] + a * (v[i] - out[i - 1]) } else { out[i - 1] };
    }
    out
}
fn true_range(x: &Ctx) -> Vec<f64> {
    (0..x.c.len()).map(|i| if i == 0 { x.h[0] - x.l[0] } else { x.h[i].max(x.c[i - 1]) - x.l[i].min(x.c[i - 1]) }).collect()
}
fn rng(x: &Ctx) -> Vec<f64> { (0..x.c.len()).map(|i| x.h[i] - x.l[i]).collect() }
/// AvgTrueRange(n) = Average(TrueRange, n)
fn atr(x: &Ctx, n: usize) -> Vec<f64> { sma(&true_range(x), n) }
fn highest(v: &[f64], n: usize, i: usize) -> f64 { v[(i + 1).saturating_sub(n.max(1))..=i].iter().copied().fold(f64::NEG_INFINITY, f64::max) }
fn lowest(v: &[f64], n: usize, i: usize) -> f64 { v[(i + 1).saturating_sub(n.max(1))..=i].iter().copied().fold(f64::INFINITY, f64::min) }
/// bars ago of the highest/lowest value in the last n bars (most recent on ties, TradeStation HighestBar)
fn highest_bar(v: &[f64], n: usize, i: usize) -> usize { let mut b = 0; for k in 0..n.min(i + 1) { if v[i - k] > v[i - b] { b = k; } } b }
fn lowest_bar(v: &[f64], n: usize, i: usize) -> usize { let mut b = 0; for k in 0..n.min(i + 1) { if v[i - k] < v[i - b] { b = k; } } b }
fn xover(a: &[f64], b: &[f64], i: usize) -> bool { i > 0 && a[i] > b[i] && a[i - 1] <= b[i - 1] }
fn xunder(a: &[f64], b: &[f64], i: usize) -> bool { i > 0 && a[i] < b[i] && a[i - 1] >= b[i - 1] }
fn xover_k(a: &[f64], k: f64, i: usize) -> bool { i > 0 && a[i] > k && a[i - 1] <= k }
fn xunder_k(a: &[f64], k: f64, i: usize) -> bool { i > 0 && a[i] < k && a[i - 1] >= k }
/// SwingHigh(1, v, s, s+1) on bar i: bar i-s is >= the s bars before it and > the s bars after it.
fn swing_high(h: &[f64], s: usize, i: usize) -> bool { i >= 2 * s && (1..=s).all(|k| h[i - s] >= h[i - s - k] && h[i - s] > h[i - s + k]) }
fn swing_low(l: &[f64], s: usize, i: usize) -> bool { i >= 2 * s && (1..=s).all(|k| l[i - s] <= l[i - s - k] && l[i - s] < l[i - s + k]) }
fn stdev(v: &[f64], n: usize) -> Vec<f64> {
    let m = sma(v, n);
    (0..v.len()).map(|i| if i + 1 < n { f64::NAN } else { (v[i + 1 - n..=i].iter().map(|x| (x - m[i]).powi(2)).sum::<f64>() / n as f64).sqrt() }).collect()
}
fn bband(v: &[f64], n: usize, k: f64) -> Vec<f64> { let (m, s) = (sma(v, n), stdev(v, n)); (0..v.len()).map(|i| m[i] + k * s[i]).collect() }
fn cci(x: &Ctx, n: usize) -> Vec<f64> {
    let tp: Vec<f64> = (0..x.c.len()).map(|i| (x.h[i] + x.l[i] + x.c[i]) / 3.0).collect();
    let m = sma(&tp, n);
    (0..tp.len()).map(|i| if i + 1 < n { f64::NAN } else {
        let md = tp[i + 1 - n..=i].iter().map(|v| (v - m[i]).abs()).sum::<f64>() / n as f64;
        if md == 0.0 { 0.0 } else { (tp[i] - m[i]) / (0.015 * md) } }).collect()
}
/// SlowK / SlowD, legacy TradeStation smoothing ((2*prev + new)/3), as in module `stad`.
fn slowkd(x: &Ctx, n: usize) -> (Vec<f64>, Vec<f64>) {
    let len = x.c.len();
    let (mut k, mut d) = (vec![f64::NAN; len], vec![f64::NAN; len]);
    for i in 0..len {
        if i + 1 < n { continue; }
        let (hh, ll) = (highest(&x.h, n, i), lowest(&x.l, n, i));
        let fk = if hh > ll { (x.c[i] - ll) / (hh - ll) * 100.0 } else { 0.0 };
        k[i] = if i == 0 || !k[i - 1].is_finite() { fk } else { (2.0 * k[i - 1] + fk) / 3.0 };
        d[i] = if i == 0 || !d[i - 1].is_finite() { k[i] } else { (2.0 * d[i - 1] + k[i]) / 3.0 };
    }
    (k, d)
}
/// Wilder DMI: (+DI, -DI, ADX)
fn dmi3(x: &Ctx, n: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let len = x.c.len();
    let (mut pdm, mut mdm, mut tr) = (vec![f64::NAN; len], vec![f64::NAN; len], vec![f64::NAN; len]);
    let t = true_range(x);
    for i in 1..len {
        let (u, d) = (x.h[i] - x.h[i - 1], x.l[i - 1] - x.l[i]);
        pdm[i] = if u > d && u > 0.0 { u } else { 0.0 };
        mdm[i] = if d > u && d > 0.0 { d } else { 0.0 };
        tr[i] = t[i];
    }
    let w = |v: &[f64]| crate::connors::wilder(v, n);
    let (sp, sm, st) = (w(&pdm), w(&mdm), w(&tr));
    let (mut p, mut m, mut dx) = (vec![f64::NAN; len], vec![f64::NAN; len], vec![f64::NAN; len]);
    for i in 0..len {
        if st[i] > 0.0 {
            p[i] = 100.0 * sp[i] / st[i]; m[i] = 100.0 * sm[i] / st[i];
            dx[i] = if p[i] + m[i] > 0.0 { 100.0 * (p[i] - m[i]).abs() / (p[i] + m[i]) } else { 0.0 };
        }
    }
    let adx = w(&dx);
    (p, m, adx)
}
fn linreg(v: &[f64], n: usize) -> Vec<f64> {
    let nf = n as f64;
    let sx = nf * (nf - 1.0) / 2.0;
    let sxx = (nf - 1.0) * nf * (2.0 * nf - 1.0) / 6.0;
    (0..v.len()).map(|i| if i + 1 < n || n < 2 { f64::NAN } else {
        let (mut sy, mut sxy) = (0.0, 0.0);
        for k in 0..n { let y = v[i + 1 - n + k]; sy += y; sxy += k as f64 * y; }
        let b = (nf * sxy - sx * sy) / (nf * sxx - sx * sx);
        let a = (sy - b * sx) / nf;
        a + b * (nf - 1.0) }).collect()
}
fn macd(c: &[f64], f: usize, s: usize) -> Vec<f64> { let (a, b) = (xavg(c, f), xavg(c, s)); (0..c.len()).map(|i| a[i] - b[i]).collect() }
fn mom(v: &[f64], n: usize) -> Vec<f64> { (0..v.len()).map(|i| if i >= n { v[i] - v[i - n] } else { f64::NAN }).collect() }
/// TriAverage: SMA of an SMA, both of length int(Length/2)+1 (STAD Vol 9 p.29 / Vol 13 p.75: "one more than half").
fn triavg(v: &[f64], n: usize) -> Vec<f64> { let k = n / 2 + 1; sma(&sma(v, k), k) }
fn vol(x: &Ctx) -> Vec<f64> { x.r.iter().map(|r| r.volume.filter(|v| v.is_finite()).unwrap_or(0.0)).collect() }
fn shift(v: &[f64], k: usize) -> Vec<f64> { (0..v.len()).map(|i| if i >= k { v[i - k] } else { f64::NAN }).collect() }
/// MRO(cond, len, inst): bars ago of the inst-th most recent true within len bars, else -1.
fn mro(c: &[bool], i: usize, len: usize, inst: usize) -> i64 {
    let mut k = 0;
    for b in 0..len.min(i + 1) { if c[i - b] { k += 1; if k == inst { return b as i64; } } }
    -1
}
/// Wilder Parabolic SAR with acceleration step `af` (max .20). Starts long on the first bar with the SAR at its low.
/// Returns the SAR in force on each bar after that bar's close (the next bar's stop/reversal level).
fn parabolic(x: &Ctx, af0: f64) -> Vec<f64> {
    let n = x.c.len();
    let mut out = vec![f64::NAN; n];
    if n == 0 { return out; }
    let (mut long, mut sar, mut ep, mut af) = (true, x.l[0], x.h[0], af0);
    out[0] = sar;
    for i in 1..n {
        let mut nsar = sar + af * (ep - sar);
        if long {
            nsar = nsar.min(x.l[i - 1]); if i >= 2 { nsar = nsar.min(x.l[i - 2]); }
            if x.l[i] <= nsar { long = false; nsar = ep; ep = x.l[i]; af = af0; }
            else if x.h[i] > ep { ep = x.h[i]; af = (af + af0).min(0.2); }
        } else {
            nsar = nsar.max(x.h[i - 1]); if i >= 2 { nsar = nsar.max(x.h[i - 2]); }
            if x.h[i] >= nsar { long = true; nsar = ep; ep = x.h[i]; af = af0; }
            else if x.l[i] < ep { ep = x.l[i]; af = (af + af0).min(0.2); }
        }
        sar = nsar; out[i] = sar;
    }
    out
}
/// (year, month, day, weekday 0=Mon) and days since 1970 from "YYYY-MM-DD".
fn days(d: &str) -> i64 {
    let y: i64 = d[0..4].parse().unwrap_or(0); let m: i64 = d[5..7].parse().unwrap_or(1); let dd: i64 = d[8..10].parse().unwrap_or(1);
    let (yy, mm) = if m <= 2 { (y - 1, m + 9) } else { (y, m - 3) };
    let era = yy.div_euclid(400); let yoe = yy - era * 400;
    let doy = (153 * mm + 2) / 5 + dd - 1; let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}
fn weekday(d: &str) -> i64 { (days(d) + 3).rem_euclid(7) }
/// Weekly bars (Monday-Friday weeks) of the daily series and, for each daily bar, the index of the last weekly bar
/// that is complete on that day (the week's last trading day closes it) - TradeStation Data2 = weekly.
struct Weekly { h: Vec<f64>, l: Vec<f64>, c: Vec<f64>, map: Vec<i64> }
fn weekly(x: &Ctx) -> Weekly {
    let n = x.c.len();
    let wk: Vec<i64> = (0..n).map(|i| (days(&x.r[i].date) - weekday(&x.r[i].date)).div_euclid(7)).collect();
    let (mut h, mut l, mut c, mut map) = (vec![], vec![], vec![], vec![-1i64; n]);
    let mut i = 0;
    while i < n {
        let mut j = i;
        while j + 1 < n && wk[j + 1] == wk[i] { j += 1; }
        h.push(highest(&x.h, j - i + 1, j)); l.push(lowest(&x.l, j - i + 1, j)); c.push(x.c[j]);
        let w = c.len() as i64 - 1;
        for k in i..=j { map[k] = if k == j { w } else { w - 1 }; }
        i = j + 1;
    }
    Weekly { h, l, c, map }
}

// ------------------------------------------------------------------ order engine

#[derive(Clone, Copy, PartialEq, Debug)]
enum A { Buy, Sell, Xl, Xs }
#[derive(Clone, Copy, Debug)]
enum P { Mkt, Stop(f64), Lim(f64) }
#[derive(Clone, Copy, Debug)]
struct O { a: A, p: P, w: f64, tag: &'static str, from: &'static str, prot: f64, qty: f64 }
fn o(a: A, p: P, tag: &'static str) -> O { O { a, p, w: 1.0, tag, from: "", prot: f64::NAN, qty: 0.0 } }
fn ow(a: A, p: P, w: f64, tag: &'static str) -> O { O { w, ..o(a, p, tag) } }
/// Exit that only applies to the position opened by entry `from` ("ExitLong from entry (...)").
fn of(a: A, p: P, tag: &'static str, from: &'static str) -> O { O { from, ..o(a, p, tag) } }
/// Partial exit of `qty` units ("ExitLong ... N Contracts Total").
fn oq(a: A, p: P, qty: f64, tag: &'static str) -> O { O { qty, ..o(a, p, tag) } }

/// Built-in stops in points (0 = off).
#[derive(Clone, Copy, Default)]
struct SetStops { sl: f64, pt: f64, be: f64, dtr: f64, pfloor: f64, ppct: f64, sl_entry_bar_only: bool, sl_pos_basis: bool }
#[derive(Clone, Copy)]
struct Cfg { max_ent: usize, ss: SetStops }
fn cfg() -> Cfg { Cfg { max_ent: 1, ss: SetStops::default() } }

/// What the code can read at the start of its evaluation of a bar.
#[derive(Clone, Copy, Default)]
struct St { pos: i8, entry_px: f64, entry_i: usize, bn: i64, units: f64, ntr: usize }
impl St {
    fn bse(&self, i: usize) -> usize { if self.pos == 0 { 0 } else { i - self.entry_i } }
}

struct Eng<'a> { x: &'a Ctx<'a>, pos: i8, units: f64, tot: f64, nent: usize, cur: Option<Trade>, prev_pos: i8, out: Vec<Trade>,
    tag: &'static str, prot: f64, cfg: Cfg, mfe: f64 }

impl<'a> Eng<'a> {
    fn close_pos(&mut self, i: usize, px: f64, why: &str, sig: usize) {
        if let Some(mut t) = self.cur.take() {
            if !t.exits.is_empty() { let f = self.units / self.tot; t.exits.push((i, px, f));
                t.exit_px = t.exits.iter().map(|e| e.1 * e.2).sum::<f64>() / t.exits.iter().map(|e| e.2).sum::<f64>(); } else { t.exit_px = px; }
            t.exit_i = i; t.reason = why.into(); t.signal_i = sig; self.out.push(t);
        }
        self.prev_pos = self.pos; self.pos = 0; self.units = 0.0; self.prot = f64::NAN;
    }
    fn exec(&mut self, od: &O, i: usize, px: f64, kind: &'static str, sig: usize) {
        match od.a {
            A::Buy | A::Sell => {
                let d: i8 = if od.a == A::Buy { 1 } else { -1 };
                if self.pos == d {
                    // pyramid add
                    if let Some(t) = self.cur.as_mut() { t.legs.push(Leg { i, px, w: od.w }); }
                    self.units += od.w; self.tot += od.w; self.nent += 1;
                    return;
                }
                if self.pos == -d { self.close_pos(i, px, od.tag, sig); }
                self.pos = d; self.tag = od.tag; self.prot = od.prot; self.units = od.w; self.tot = od.w; self.nent = 1; self.mfe = 0.0;
                self.cur = Some(Trade { dir: d as f64, legs: vec![Leg { i, px, w: od.w }], exit_i: i, exit_px: px,
                    reason: String::new(), entry_kind: kind, signal_i: sig, open: false, exits: vec![] });
            }
            A::Xl | A::Xs => {
                if od.qty > 0.0 && od.qty < self.units - 1e-9 {
                    if let Some(t) = self.cur.as_mut() { t.exits.push((i, px, od.qty / self.tot)); }
                    self.units -= od.qty;
                } else { self.close_pos(i, px, od.tag, sig); }
            }
        }
    }
    fn valid(&self, od: &O) -> bool {
        let tag_ok = od.from.is_empty() || od.from == self.tag;
        match od.a {
            A::Buy => self.pos != 1 || (self.nent < self.cfg.max_ent),
            A::Sell => self.pos != -1 || (self.nent < self.cfg.max_ent),
            A::Xl => self.pos == 1 && tag_ok,
            A::Xs => self.pos == -1 && tag_ok,
        }
    }
    fn entry_px(&self) -> f64 { self.cur.as_ref().map_or(0.0, |t| t.legs[0].px) }
    fn entry_i(&self) -> usize { self.cur.as_ref().map_or(0, |t| t.legs[0].i) }
    /// stop-loss / profit-target orders of the open position (active from its entry bar)
    fn set_orders(&self, i: usize) -> Vec<O> {
        let mut v = vec![];
        let ss = self.cfg.ss;
        if self.pos == 0 { return v; }
        let (e, d) = (self.entry_px(), self.pos as f64);
        let xa = if self.pos == 1 { A::Xl } else { A::Xs };
        // position basis with several units: the dollar loss is shared by the units (STAD13 ADD, SetStopPosition)
        let sl = if ss.sl_pos_basis && self.units > 0.0 { ss.sl / self.units } else { ss.sl };
        if sl > 0.0 && (!ss.sl_entry_bar_only || self.entry_i() == i) { v.push(o(xa, P::Stop(e - d * sl), "StopLoss")); }
        if ss.pt > 0.0 { v.push(o(xa, P::Lim(e + d * ss.pt), "ProfitTarget")); }
        v
    }
    /// Fill the orders issued at bar i-1 on bar i.
    fn fill_bar(&mut self, i: usize, ords0: &[O]) {
        let mut ords: Vec<O> = ords0.to_vec();
        if self.pos != 0 && self.prot.is_finite() {
            ords.push(O { a: if self.pos == 1 { A::Xl } else { A::Xs }, p: P::Stop(self.prot), from: self.tag, ..o(A::Xl, P::Mkt, "Entry-bar stop (at$)") });
        }
        ords.extend(self.set_orders(i));
        if ords.is_empty() { return; }
        let x = self.x;
        let (op, hi, lo, cl) = (x.o[i], x.h[i], x.l[i], x.c[i]);
        let mut done = vec![false; ords.len()];
        macro_rules! fire { ($k:expr, $px:expr) => {{
            done[$k] = true; let od = ords[$k]; let was = self.pos;
            self.exec(&od, i, $px, kind_of(&od), i - 1);
            if matches!(od.a, A::Buy | A::Sell) && self.pos != was {
                if od.prot.is_finite() {
                    ords.push(O { a: if od.a == A::Buy { A::Xl } else { A::Xs }, p: P::Stop(od.prot), from: od.tag, ..o(A::Xl, P::Mkt, "Entry-bar stop (at$)") });
                    done.push(false);
                }
                for so in self.set_orders(i) { ords.push(so); done.push(false); }
            }
        }}; }
        for pass in 0..2 {
            for k in 0..ords.len() {
                let od = ords[k];
                let is_exit = matches!(od.a, A::Xl | A::Xs);
                if matches!(od.p, P::Mkt) && (pass == 0) == is_exit && !done[k] && self.valid(&od) { fire!(k, op); }
            }
        }
        let rising = |a: A| matches!(a, A::Buy | A::Xs);
        loop {
            let hit = ords.iter().enumerate().find(|(k, od)| !done[*k] && self.valid(od) && match od.p {
                P::Stop(s) => if rising(od.a) { op >= s } else { op <= s },
                P::Lim(s) => if rising(od.a) { op <= s } else { op >= s },
                P::Mkt => false,
            }).map(|(k, _)| k);
            match hit { Some(k) => { fire!(k, op); } None => break }
        }
        let bounce = 0.1 * (hi - lo);
        let mut path: std::collections::VecDeque<f64> = if hi - op < op - lo { [hi, lo, cl].into() } else { [lo, hi, cl].into() };
        let mut cur = op;
        while let Some(&t) = path.front() {
            let up = t > cur;
            let mut best: Option<(usize, f64)> = None;
            for (k, od) in ords.iter().enumerate() {
                if done[k] || !self.valid(od) { continue; }
                let lv = match od.p { P::Stop(s) if rising(od.a) == up => s, P::Lim(s) if rising(od.a) != up => s, _ => continue };
                let inside = if up { lv > cur && lv <= t } else { lv < cur && lv >= t };
                if inside && best.map_or(true, |(_, b)| (lv - cur).abs() < (b - cur).abs()) { best = Some((k, lv)); }
            }
            match best {
                Some((k, lv)) => {
                    fire!(k, lv);
                    cur = lv;
                    let back = if up { (lv - bounce).max(lo) } else { (lv + bounce).min(hi) };
                    if back != lv { path.push_front(back); }
                }
                None => { cur = t; path.pop_front(); }
            }
        }
    }
    /// breakeven / $ trailing / % trailing for the next bar, from the maximum favourable excursion through bar i
    fn close_eval(&mut self, i: usize, nx: &mut Vec<O>) {
        if self.pos == 0 { return; }
        let ss = self.cfg.ss;
        let (e, d) = (self.entry_px(), self.pos as f64);
        let fav = if self.pos == 1 { self.x.h[i] - e } else { e - self.x.l[i] };
        if fav > self.mfe { self.mfe = fav; }
        let xa = if self.pos == 1 { A::Xl } else { A::Xs };
        if ss.be > 0.0 && self.mfe >= ss.be { nx.push(o(xa, P::Stop(e), "Breakeven")); }
        if ss.dtr > 0.0 { nx.push(o(xa, P::Stop(e + d * (self.mfe - ss.dtr)), "$ Risk Trailing")); }
        if ss.ppct > 0.0 && self.mfe >= ss.pfloor && self.mfe > 0.0 { nx.push(o(xa, P::Stop(e + d * self.mfe * (1.0 - ss.ppct / 100.0)), "% Risk Trailing")); }
    }
}
fn kind_of(od: &O) -> &'static str { match od.p { P::Mkt => "open", P::Stop(_) => "stop", P::Lim(_) => "limit" } }

/// Run TradeStation-style strategy code `f(i, state, close_orders, next_bar_orders)` over bars 0..=to.
fn run_ts(x: &Ctx, from: usize, to: usize, mbb: usize, cf: Cfg, mut f: impl FnMut(usize, &St, &mut Vec<O>, &mut Vec<O>)) -> Vec<Trade> {
    let mut e = Eng { x, pos: 0, units: 0.0, tot: 0.0, nent: 0, cur: None, prev_pos: 0, out: vec![], tag: "", prot: f64::NAN, cfg: cf, mfe: 0.0 };
    let mut pending: Vec<O> = vec![];
    for i in 0..=to {
        if i > from { e.fill_bar(i, &pending); }
        pending.clear();
        if i < mbb { if i >= from && i < to { e.close_eval(i, &mut pending); } continue; }
        let st = St { pos: e.pos, entry_px: e.entry_px(), entry_i: e.entry_i(), bn: (i - mbb + 1) as i64, units: e.units, ntr: e.out.len() };
        let (mut now, mut next) = (vec![], vec![]);
        f(i, &st, &mut now, &mut next);
        if i >= from {
            for od in now.iter() {
                if e.valid(od) { e.exec(od, i, x.c[i], "close", i); }
            }
            if i < to { pending = next; e.close_eval(i, &mut pending); }
        }
    }
    if let Some(mut t) = e.cur.take() {
        if !t.exits.is_empty() { let f = e.units / e.tot; t.exits.push((to, x.c[to], f)); t.exit_px = t.exits.iter().map(|q| q.1 * q.2).sum::<f64>(); } else { t.exit_px = x.c[to]; }
        t.exit_i = to; t.signal_i = to; t.reason = "open".into(); t.open = true; e.out.push(t);
    }
    e.out
}

// ---------------------------------------------- StrategyBuilder common exits (Vol 8 ch.9, Vol 9-13 appendices)

/// The ATR signal family: Protective (EntryPrice -/+ k*ATR), Breakeven (highest/lowest close since entry beyond
/// EntryPrice +/- k*ATR -> stop at EntryPrice), Trailing (position high/low -/+ k*ATR), Volatility (printed code:
/// EntryPrice -/+ k*ATR), Big Profit (highest/lowest close beyond EntryPrice +/- k*ATR -> n-bar low/high stop).
/// Each takes its own ATR length; 0 multiple = signal not in the strategy.
#[derive(Clone, Copy, Default)]
struct AtrX { prot: f64, be: f64, trail: f64, volat: f64, bp: f64, bpn: usize, pt: f64 }
#[derive(Default)]
struct AtrState { pos_hl: f64, pos_high: f64, pos_low: f64, e_atr: f64 }
fn atr_exits(x: &Ctx, i: usize, st: &St, a: &AtrX, atrv: f64, s: &mut AtrState, nx: &mut Vec<O>) {
    let (h, l, c) = (&x.h, &x.l, &x.c);
    // "If BarsSinceEntry = 0 Then PosHL = Close" sits outside the position test (BarsSinceEntry is 0 while flat);
    // the trailing stop's PosHigh/PosLow reset sits inside "If MarketPosition = 1/-1".
    if st.bse(i) == 0 { s.pos_hl = c[i]; s.e_atr = a.pt * atrv; if st.pos != 0 { s.pos_high = h[i]; s.pos_low = l[i]; } }
    let e = st.entry_px;
    if st.pos == 1 {
        if c[i] > s.pos_hl { s.pos_hl = c[i]; }
        if h[i] > s.pos_high { s.pos_high = h[i]; }
        if a.prot > 0.0 { nx.push(o(A::Xl, P::Stop(e - a.prot * atrv), "ATR Protective")); }
        if a.be > 0.0 && s.pos_hl > e + a.be * atrv { nx.push(o(A::Xl, P::Stop(e), "ATR Breakeven")); }
        if a.trail > 0.0 { nx.push(o(A::Xl, P::Stop(s.pos_high - a.trail * atrv), "ATR Trailing")); }
        if a.volat > 0.0 { nx.push(o(A::Xl, P::Stop(e - a.volat * atrv), "ATR Volatility")); }
        if a.bp > 0.0 && s.pos_hl > e + a.bp * atrv { nx.push(o(A::Xl, P::Stop(lowest(l, a.bpn, i)), "ATR Big Profit")); }
        if a.pt > 0.0 { nx.push(o(A::Xl, P::Lim(e + s.e_atr), "ATR Profit Target")); }
    }
    if st.pos == -1 {
        if c[i] < s.pos_hl { s.pos_hl = c[i]; }
        if l[i] < s.pos_low { s.pos_low = l[i]; }
        if a.prot > 0.0 { nx.push(o(A::Xs, P::Stop(e + a.prot * atrv), "ATR Protective")); }
        if a.be > 0.0 && s.pos_hl < e - a.be * atrv { nx.push(o(A::Xs, P::Stop(e), "ATR Breakeven")); }
        if a.trail > 0.0 { nx.push(o(A::Xs, P::Stop(s.pos_low + a.trail * atrv), "ATR Trailing")); }
        if a.volat > 0.0 { nx.push(o(A::Xs, P::Stop(e + a.volat * atrv), "ATR Volatility")); }
        if a.bp > 0.0 && s.pos_hl < e - a.bp * atrv { nx.push(o(A::Xs, P::Stop(highest(h, a.bpn, i)), "ATR Big Profit")); }
        if a.pt > 0.0 { nx.push(o(A::Xs, P::Lim(e - s.e_atr), "ATR Profit Target")); }
    }
}
/// Parabolic Trailing / ModStop LX+SX (Vol 8 p.50-53, Vol 10 p.58-62, Vol 12 p.152-156), printed code.
struct ParaX { af_l: f64, mult_l: f64, af_s: f64, mult_s: f64, sp_l: f64, a_l: f64, hv: f64, hv1: f64, sp_s: f64, a_s: f64, lv: f64, lv1: f64, mpp: i8 }
impl ParaX {
    fn new(af_l: f64, mult_l: f64, af_s: f64, mult_s: f64) -> Self {
        ParaX { af_l, mult_l, af_s, mult_s, sp_l: 0.0, a_l: 0.0, hv: 0.0, hv1: 0.0, sp_s: 0.0, a_s: 0.0, lv: 0.0, lv1: 0.0, mpp: 0 }
    }
    fn step(&mut self, x: &Ctx, i: usize, mp: i8, atr3: f64, nx: &mut Vec<O>, long: bool, short: bool) {
        let (h, l) = (x.h[i], x.l[i]);
        if long {
            self.hv1 = self.hv;
            if h > self.hv { self.hv = h; }
            if mp == 1 {
                if self.mpp != 1 { self.sp_l = l - atr3 * self.mult_l; self.a_l = self.af_l; self.hv = h; }
                else {
                    self.sp_l += self.a_l * (self.hv - self.sp_l);
                    if self.hv > self.hv1 && self.a_l < 0.2 { self.a_l += self.af_l.min(0.2 - self.a_l); }
                }
                if self.sp_l > l { self.sp_l = l; }
                nx.push(o(A::Xl, P::Stop(self.sp_l), "Parabolic LX"));
            }
        }
        if short {
            self.lv1 = self.lv;
            if l < self.lv { self.lv = l; }
            if mp == -1 {
                if self.mpp != -1 { self.sp_s = h + atr3 * self.mult_s; self.a_s = self.af_s; self.lv = l; }
                else {
                    self.sp_s -= self.a_s * (self.sp_s - self.lv);
                    if self.lv < self.lv1 && self.a_s < 0.2 { self.a_s += self.af_s.min(0.2 - self.a_s); }
                }
                if self.sp_s < h { self.sp_s = h; }
                nx.push(o(A::Xs, P::Stop(self.sp_s), "Parabolic SX"));
            }
        }
        self.mpp = mp;
    }
}

// ------------------------------------------------------------------ catalogue

struct Def { id: &'static str, name: &'static str, group: &'static str, dir: &'static str, status: Status,
    needs_volume: bool, rules: &'static [&'static str], notes: &'static str, family: (&'static str, &'static str, &'static str) }


fn defs_a() -> Vec<Def> {
    use Status::*;
    let d = |id, name, group, dir, status, rules, notes, family| Def { id, name, group, dir, status, needs_volume: false, rules, notes, family };
    vec![
    // ================= Volume 5 (1998)
    d("stad5_curbond", "Currency/Bond System", "Vol 5 · Trending", "Long/Short",
      NeedsData("Needs a second aligned data stream: Data1 = a currency future, Data2 = US Treasury Bond futures (XAverage(Close,50) of Data2). No bond or currency futures in the vault."),
      &["F|FastXAvg = XAverage(C,4), MedXAvg = XAverage(C,12), SlowXAvg = XAverage(C,23); LongXAvg = XAverage(Close,50) of Data2 (bonds) (Vol 5 p.13-14 code)",
        "E|FastXAvg > FastXAvg[1] and Med > Slow and LongXAvg > LongXAvg[1]: BuyLevel = High + .50*Average(TrueRange,10); buy stop for 5 bars (sell mirror)",
        "X|MedXAvg < SlowXAvg: ExitLong next bar at market (short mirror); ExitLong next bar at Close - StopAmt($1000)/BigPointValue stop"],
      "Vol 5 ch.2 p.12-20. Book: Swiss Franc + bonds 6/93-5/98: $28,425 on 56 trades, 62% winners, win/loss 1.47, PF 2.45.", ("stad5_curbond", "Currency/Bond", "Vol 5")),
    d("stad5_im1", "Intermarket I", "Vol 5 · Trending", "Long",
      NeedsData("Needs Data2 = the US Treasury bond yield (daily); no rate series in the vault."),
      &["E|If Close of Data2 < Close[1] of Data2 + LYldTrg(.25): Buy this bar on close (Vol 5 p.21 code; prose '25% of the yield' - code adds .25)",
        "X|BarsSinceEntry > TNBars(50): ExitLong this bar on close"],
      "Vol 5 ch.2 p.20-25. Book: S&P index 1/88-9/98: 132 trades, 88% profitable, PF 5.58.", ("stad5_im1", "Intermarket I", "Vol 5")),
    d("stad5_im2", "Intermarket II", "Vol 5 · Trending", "Long",
      NeedsData("Needs Data3 = the US prime rate (daily); no rate series in the vault."),
      &["F|If Close of Data3 < Close[1] of Data3 then prSteps += 1 else if Close of Data3 > Close[1] then prSteps = 0 (Vol 5 p.26-27 code)",
        "E|prSteps >= PStepTrg(1) and prSteps > prSteps[1]: Buy this bar on close", "X|BarsSinceEntry > PNBars(100): ExitLong this bar on close; Format: $4,500 money-management stop"],
      "Vol 5 ch.2 p.26-32. Book: S&P futures 1/78-9/98: 15 trades, 53% winners, win/loss 3.17, PF 3.63, avg trade $5,005.", ("stad5_im2", "Intermarket II", "Vol 5")),
    d("stad5_fib382", "Fibonacci .382 Retracement", "Vol 5 · Support/Resistance", "Long/Short", Live,
      &["F|XAvg = XAverage(Close,15); HiHi/LoLo = Highest(High,50)/Lowest(Low,50); HiHiBar/LoLoBar = HighestBar/LowestBar; Retracement = (HiHi-LoLo)*.382 (Vol 5 p.35-36 code)",
        "F|Close <= HiHi - Retracement and HiHiBar < LoLoBar: BuySetup = 0; Close >= LoLo + Retracement and HiHiBar > LoLoBar: SellSetup = 0; both counters +1 every bar",
        "E|BuySetup <= SetUpLen(24) and (SwingLow(1,XAvg,1,2) <> -1 or Close > XAvg): Buy next bar at market; BuySetup = SetUpLen",
        "E|SellSetup <= 24 and (SwingHigh(1,XAvg,1,2) <> -1 or Close < XAvg): Sell next bar at market; SellSetup = SetUpLen",
        "X|Format dialog (p.38): $200 money-management stop and $400 $ Risk Trailing stop (book's Gold test: $100/point -> 2 and 4 points; params mm_usd, tr_usd, bpv)"],
      "Vol 5 ch.3 p.34-41. Setup counters start at 0 (as declared), so the first 24 bars after MaxBarsBack can enter without a setup (literal). Book: Gold futures 6/93-7/98: 85 trades, 35% profitable, win/loss 2.76, PF 1.51.",
      ("stad5_fib382", "Fibonacci .382 Retracement", "Vol 5")),
    d("stad5_hlosc", "High/Low Oscillator", "Vol 5 · Support/Resistance", "Long/Short", Live,
      &["F|HighOsc = High - XAverage(High,10); LowOsc = Low - XAverage(Low,10); CloseAvg = XAverage(Close,60) (Vol 5 p.43-44 code)",
        "E|CloseAvg > CloseAvg[1] and Close > CloseAvg and LowOsc < 0 and LowOsc > LowOsc[1]: Buy next bar at market",
        "E|CloseAvg < CloseAvg[1] and Close < CloseAvg and HighOsc > 0 and HighOsc < HighOsc[1]: Sell next bar at market",
        "X|Long: ExitLong next bar at Close - TrailStp(700)/BigPointValue stop; HighOsc crosses under 0: ExitLong next bar at market (short mirror)",
        "X|Format dialog (p.46): $300 money-management stop, breakeven floor $200 (book's Wal-Mart test, per 100 shares: 3 and 2 points)"],
      "Vol 5 ch.3 p.42-48. BigPointValue of the book's stock = 1, so the coded trail is 700 points (literal). Book: Wal-Mart daily 1988-1998: 129 trades, 60% profitable, win/loss 1.46, PF 2.23.",
      ("stad5_hlosc", "High/Low Oscillator", "Vol 5")),
    d("stad5_stochob", "Stochastic Overbought/Oversold", "Vol 5 · Support/Resistance", "Long/Short", Live,
      &["F|KVal = SlowKCustom(H,L,C,12); DVal = SlowDCustom(H,L,C,12) (Vol 5 p.50-51 code)",
        "F|KVal < OSold(30) and KVal crosses above DVal: BuySetup = Highest(High,2), BuyStop = Low, BuyPeriod = 0 (sell mirror above OBought 70)",
        "F|BuyPeriod = SetUpLen+1 while long, else +1 each bar (sell mirror)",
        "E|BuyPeriod <= SetUpLen(6): Buy next bar at BuySetup + 1 point stop; SellPeriod <= 6: Sell next bar at SellSetup - 1 point stop",
        "X|KVal crosses below DVal and KVal > OSold: BuyPeriod = SetUpLen, ExitLong next bar at market (short mirror)",
        "X|ExitLong next bar at BuyStop stop; ExitShort next bar at SellStop stop (every bar)"],
      "Vol 5 ch.3 p.48-57. Book test was weekly Caterpillar 2/89-9/98: 28 trades, 67% profitable, win/loss 1.39; runs on daily index bars here (timeframe differs). No Format stops.",
      ("stad5_stochob", "Stochastic OB/OS", "Vol 5")),
    d("stad5_pivotsr", "Pivot Point Support & Resistance", "Vol 5 · Support/Resistance", "Long/Short", Live,
      &["F|Condition1 = SwingLow(1,Low,1,2) <> -1; Condition3 = Low[1] = Lowest(Low,8)[1]; LPivot = MRO(Cond1 and Cond3, 4, 1); Condition5 = TrueRange > AvgTrueRange(10) (Vol 5 p.59-61 code; highs mirror)",
        "F|LPivot > 0 and Lowest(Low,LPivot) < Low[LPivot+1] and Condition5 and Close > MaxList(Close[1], Open, Low + 2*TrueRange/3, Low[LPivot+1]): BuySetupHigh/Low = High/Low, count = 0",
        "F|HPivot > 0 and Highest(High,HPivot) > High[HPivot+1] and Condition5 and Close < MinList(Close[1], Open, Low + TrueRange/3, High[HPivot+1]): SellSetupLow/High, count = 0",
        "X|MP = 1 or TotalTrades changed: BuySetupCount = 24; if Close > EntryPrice + 2*(EntryPrice - LExitLow) or LExitFlag = 0: ExitLong next bar at EntryPrice stop, LExitFlag = 0 (short mirror)",
        "E|BuySetupCount <= SetUpLen(24) and BuySetupHigh + 1 point > EMA(9): Buy next bar at BuySetupHigh + 1 point stop, LExitLow = BuySetupLow, LExitFlag = 1",
        "E|SellSetupCount <= 24 and SellSetupLow + 1 point < EMA (code reads '+'): Sell next bar at SellSetupLow - 1 point stop, SExitHigh = SellSetupHigh, SExitFlag = 1",
        "X|LExitFlag = 1: ExitLong next bar at LExitLow - 1 point stop (short mirror); ExitLong next bar at Highest(High,44) limit; ExitShort at Lowest(Low,44) limit",
        "X|Format dialog (p.65): breakeven floor $200 (book's T-bond test, $1,000/point: 0.2 points)"],
      "Vol 5 ch.3 p.57-69. PMult is the code's own variable (2); the RiskMult input is declared but unused (param 'riskmult' sets PMult). Book: T-bonds 6/93-9/98: 17 trades, 70% profitable, win/loss 1.93, PF 4.64.",
      ("stad5_pivotsr", "Pivot Point S&R", "Vol 5")),
    d("stad5_cur4day", "Currency Four-Day", "Vol 5 · Volatility breakout", "Long/Short", Live,
      &["E|If DayOfWeek(Date) = 5 (Friday): RSI(Close,50) > 50: Buy next bar at Open of next bar + .2*Average(TrueRange,10) stop; RSI < 50: Sell next bar at Open of next bar - .2*ATR stop (Vol 5 p.73 code)",
        "X|On Friday: if Open of next bar > High: ExitLong next bar at High stop; ExitLong next bar at Low stop (short mirror)",
        "X|MarketPosition <> 0: ExitLong and ExitShort next bar at market",
        "X|Format dialog (p.75): $200 money-management stop for the day of entry (book's D-Mark test, $125,000/point: 0.0016 points)"],
      "Vol 5 ch.4 p.72-77. Code buys on the Monday after the Friday setup (prose says Friday; code wins). Book: D-Mark 6/93-7/98: 172 trades, 49% winners, $9,825, win/loss 1.75, PF 1.67.",
      ("stad5_cur4day", "Currency Four-Day", "Vol 5")),
    d("stad5_chbopb", "Channel Breakout with Pullback", "Vol 5 · Volatility breakout", "Long/Short", Live,
      &["F|HiChannel = Highest(Close,10)[1]; LoChannel = Lowest(Close,10)[1] (Vol 5 p.79 code)",
        "E|MRO(Close > HiChannel,10,1) > -1 and Low < Lowest(Low,4)[1]: Buy next bar at market",
        "E|MRO(Close < LoChannel,10,1) > -1 and High > Highest(High,4)[1]: Sell next bar at market",
        "X|High > Highest(High,8)[1]: ExitLong this bar on close; Low < Lowest(Low,8)[1]: ExitShort this bar on close",
        "X|Format dialog (p.80): $1,000 money-management stop, breakeven floor $200 (book's T-bond test: 1.0 and 0.2 points)"],
      "Vol 5 ch.4 p.77-84. Book: T-bonds 6/93-9/98: 220 trades, 69% profitable, win/loss .73, PF 1.66.", ("stad5_chbopb", "Channel Breakout with Pullback", "Vol 5")),
    d("stad5_dsdb", "Displaced Standard Deviation Bands", "Vol 5 · Volatility breakout", "Long/Short", Live,
      &["F|AvgVal = Average(Close,3); SDmult = StdDev(Close,SDLen=12)*2; DispTop = AvgVal[16] + SDmult; DispBottom = AvgVal[16] - SDmult (Vol 5 p.86 code; prose says 15 bars - code wins)",
        "E|CurrentBar > 16: Buy next bar at DispTop stop; Sell next bar at DispBottom stop",
        "X|Stop and reverse: a long ends only when the DispBottom sell stop reverses it (short mirror); no other exits"],
      "Vol 5 ch.4 p.85-91. Only the average is displaced; the deviation is current (code). Book: J&J daily 1/88-9/98: 63 trades, 60% profitable, win/loss 1.38, PF 2.10.",
      ("stad5_dsdb", "Displaced Std Dev Bands", "Vol 5")),
    // ================= Volume 6 (1998)
    d("stad6_tenbar", "Ten-Bar Channel Breakout (ch.1 example)", "Vol 6 · Concepts", "Long/Short",
      NotTestable("The system's EasyLanguage is printed only as a screen image (Chart 2, p.12); the text gives no rules beyond 'simple stop-and-reverse ten-bar channel breakout' plus optimised Format stops."),
      &["E|Stop-and-reverse ten-bar channel breakout (Vol 6 p.12-16, Chart 2 image)", "X|Chart 14: $500 money-management stop, $900 $ risk trailing, $500 breakeven floor (D-Mark)"],
      "Vol 6 ch.1 p.12-16 (D-Mark: SAR $72,762 on 148 trades; with all stops $85,150 on 312 trades, 57% wins).", ("stad6_tenbar", "Ten-Bar Channel Breakout", "Vol 6")),
    d("stad6_asi", "Accumulation Swing Index", "Vol 6 · Trending", "Long/Short",
      NotTestable("AccumSwingIndex is weighted by the symbol's DailyLimit (the futures limit move; TradeStation help, AccumSwingIndex/SwingIndex). Indices and stocks have no daily limit, so the indicator's value is undefined here, and the book does not give the formula."),
      &["F|ASI = AccumSwingIndex; SH = SwingHighBar(1,High,2,3); SHAgo = SwingHighBar(2,High,2,40) (Vol 6 p.19-20 code)",
        "E|SH <> -1 and ASI[SH] > ASI[SHAgo] and MP <> 1: BuyPrice = Highest(High,2); buy stop BuyPrice + 1 point for 5 bars (sell mirror)",
        "X|ExitLong at Lowest(Low,2) - 1 point; trailing at the latest strength-1 swing low - 1 point"],
      "Vol 6 ch.2 p.18-26. Book: weekly Sears 1/78-10/98: $2,173 on 23 trades, 57% profitable (no stops).", ("stad6_asi", "Accumulation Swing Index", "Vol 6")),
    d("stad6_adxmac", "ADX & Moving-Average Channel", "Vol 6 · Trending", "Long/Short", Live,
      &["F|UpperMA = XAverage(High,30)[1]; LowerMA = XAverage(Low,30)[1]; ChanSpread = (UpperMA-LowerMA)/2; ADX(12) (Vol 6 p.27-28 code)",
        "F|BuySetup = Close > UpperMA and ADX > ADX[1] -> BuyTarget = Close + ChanSpread; SellSetup = Close < LowerMA and ADX > ADX[1] -> SellTarget = Close - ChanSpread",
        "E|MROBS = MRO(BuySetup,EntryBar=2,1); MROBS <> -1 and MRO(MarketPosition=1, MROBS, 1) = -1: Buy next bar at BuyTarget stop (sell mirror)",
        "X|ExitLong next bar at UpperMA - 1 point stop; ExitShort next bar at LowerMA + 1 point stop (code; prose uses the opposite band - code wins)"],
      "Vol 6 ch.2 p.26-34. Book: Intel daily 1/88-10/98 long only: $5,735 on 51 trades, 51% profitable, PF 3.26; Yen 1/95-10/98 no stops: 28 trades, 39%. The book also enabled Format-dialog stops whose dollar amounts it does not print; they are not modelled.", ("stad6_adxmac", "ADX & MA Channel", "Vol 6")),
    d("stad6_dmachan", "Displaced MA Channel & Range Leaders", "Vol 6 · Trending", "Long/Short", Live,
      &["F|UpperAvg = Average(High,20)[5]; LowerAvg = Average(Low,20)[5]; ExitAvg = Average(MedianPrice,20)[5] (Vol 6 p.35 code)",
        "F|RangeLeadB = MedianPrice > High[1] and Range > Range[1]; RangeLeadS = MedianPrice < Low[1] and Range > Range[1]",
        "E|RangeLeadB and Close > UpperAvg: Buy this bar on close; RangeLeadS and Close < LowerAvg: Sell this bar on close",
        "X|BarsSinceEntry < ExitBar(5): ExitLong and ExitShort next bar at ExitAvg stop; else ExitLong at UpperAvg - 1 point stop, ExitShort at LowerAvg + 1 point stop",
        "X|Format dialog (p.38): money-management $200, breakeven $100, $ risk trailing $600 (Wal-Mart test, per 100 shares: 2, 1 and 6 points)"],
      "Vol 6 ch.2 p.34-41. Book: weekly Wal-Mart 7/79-10/98: $2,713 on 54 trades, 57% profitable, PF 2.16; British Pound 10/96-10/98 ($900 MM): 59 trades, 86%.",
      ("stad6_dmachan", "DMA Channel & Range Leaders", "Vol 6")),
    d("stad6_mapairs", "Four Sets of Moving Averages Crossover", "Vol 6 · Trending", "Long/Short", Live,
      &["E|Average(Close,LEFast=5) > Average(Close,LESlow=20): Buy next bar at High + 1 point stop (Vol 6 p.43 code: 'greater than', prose 'crosses' - code wins)",
        "X|Average(Close,3) crosses below Average(Close,10): ExitLong next bar at market",
        "E|Average(Close,5) < Average(Close,20): Sell next bar at Low - 1 point stop",
        "X|Average(Close,3) crosses above Average(Close,10): ExitShort next bar at market",
        "F|Labelled option dm=1: the D-Mark pairs of the text (7/23, 4/8, 6/18, 2/9)"],
      "Vol 6 ch.2 p.41-48. Code defaults are the Merck values (5/20, 3/10). Book: D-Mark 6/93-10/98 (text pairs): 177 trades, 41%, PF 1.27; weekly Merck long-only (code values): 50 trades, 60%, PF 3.93. The book also enabled Format-dialog stops whose dollar amounts it does not print; they are not modelled.",
      ("stad6_mapairs", "Four Sets of MA Crossover", "Vol 6")),
    d("stad6_cciavg", "Commodity Channel Index Average", "Vol 6 · Support/Resistance", "Long/Short", Live,
      &["F|AvgCCI = Average(CCI(30),5); BSetup = AvgCCI crosses above -100; SSetup = AvgCCI crosses below 100 (Vol 6 p.51-52 code)",
        "E|AvgCCI > -100 and MRO(BSetup,EntryBar=4,1) <> -1: Buy next bar at Highest(High,9)[1] + 1 point stop (sell mirror at Lowest(Low,9)[1] - 1 point)",
        "X|In a position, each new 9-bar high (low when short) cuts TrailStop by 1 (not below 1); BarsSinceEntry = 0 resets TrailStop = 9",
        "X|ExitLong next bar at Lowest(Low,TrailStop) - 1 point stop; ExitShort next bar at Highest(High,TrailStop) + 1 point stop",
        "X|Format dialog: breakeven floor $400 (UTX weekly test, per 100 shares: 4 points); the money-management and $ risk trailing amounts are not printed"],
      "Vol 6 ch.3 p.50-57. Book: weekly UTX 1/70-10/98: $3,567 on 36 trades, 58% profitable, PF 3.79; CRB daily 1/95-10/98: 29 trades, 86%.",
      ("stad6_cciavg", "CCI Average", "Vol 6")),
    d("stad6_mrtr", "Minor Reaction / Trend Resumption", "Vol 6 · Support/Resistance", "Long/Short", Live,
      &["F|HiChan = Highest(High,40)[1]; LoChan = Lowest(Low,40)[1] (Vol 6 p.59-61 code)",
        "F|Not long and High > HiChan: LReaction = 1, LReacHigh/Low = High/Low, LExit = LReacLow, LThrust = 0, SReaction = 0; when long: LReaction = 0, LThrust = 0 (short block mirrors, but its long-position branch resets SReaction and LThrust as printed)",
        "F|Short breakout sets SReacLow = High (Vol 7 p.87 erratum of the printed 'Low'), SExit = SReacLow",
        "F|LReaction >= 1: Low < LExit -> LExit = Low, LThrust = 0; High < LReacHigh and Low < LReacLow -> LReaction +1, new reaction bar, NewLThrust = True (short mirror)",
        "F|LReaction >= Reaction(4) and Close > High[1]: LThrust +1; LTR = LThrust = TCount(2) and NewLThrust -> LEntry = High + 1 point",
        "E|MRO(LTR,Setup=4,1) <> -1 and LReaction >= 4: Buy next bar at LEntry stop (sell mirror at Low - 1 point)",
        "X|LReaction = 0: ExitLong next bar at LExit stop; SReaction = 0: ExitShort next bar at SExit stop",
        "F|Labelled option erratum=0: the originally printed SReacLow = Low"],
      "Vol 6 ch.3 p.58-67 (Jack Schwager's 'Reversal of Minor Reaction'). The $ risk trailing amount is not printed (not modelled). Book: weekly HP 1/78-10/98: $3,175 on 11 trades, 64%; GSCI daily 6/93-10/98: 23 trades, 48%.",
      ("stad6_mrtr", "Minor Reaction/Trend Resumption", "Vol 6")),
    d("stad6_fps", "First Pullback", "Vol 6 · Volatility breakout", "Long/Short", Live,
      &["F|MACDLine = MACD(Close,4,10); SignalLine = XAverage(MACD,16); ATR = AvgTrueRange(10) (Vol 6 p.71-73 code)",
        "F|Signal crosses above 0: UpTrend, cancel sell setup, SignalFlag = False; below 0 mirror",
        "F|UpTrend: MACD crosses below Signal and SignalFlag = False -> BuySetup, CTrendLow = Low; MACD < Signal and Low < CTrendLow -> CTrendLow = Low (short mirror)",
        "F|MP = 1: BuySetup = False, SignalFlag = True (short mirror)",
        "E|BuySetup: Buy next bar at Close + 1.25*ATR stop; SellSetup: Sell next bar at Close - 1.25*ATR stop",
        "X|ExitLong next bar at CTrendLow - 1 point stop and at Close - 2.0*ATR stop (short mirror); DnTrend: ExitLong this bar on close; UpTrend: ExitShort this bar on close"],
      "Vol 6 ch.4 p.70-79 (after Linda Raschke). Book: British Pound 6/93-11/98: $6,925 on 22 trades, 36%, win/loss 3.96. The book also enabled Format-dialog stops whose dollar amounts it does not print; they are not modelled.", ("stad6_fps", "First Pullback", "Vol 6")),
    Def { needs_volume: true, ..d("stad6_parvol", "Parabolic & Volume", "Vol 6 · Volatility breakout", "Long/Short", Live,
      &["F|P = Parabolic(AccFact=.03)[1]; VAvg = Average(Volume,5) (Vol 6 p.81 code)",
        "F|BuySetup = High crosses above P and Volume > VAvg: BSFlag, BSPrice = Close, BSP = P[1] (sell mirror with Low crosses below P)",
        "E|MRO(BuySetup,SetupLen=4,1) <> -1 and BSFlag: Buy next bar at BSPrice + .30*(P - BSP) stop (sell: SSPrice - .30*(SSP - P))",
        "X|Long: ExitLong next bar at P stop, BSFlag = False; short: ExitShort next bar at P stop"],
      "Vol 6 ch.4 p.79-87. Parabolic = Wilder SAR with step .03, max .20, started long at the first bar (TradeStation help gives no initialisation; assumption). Needs volume. Book: weekly P&G long-only 1/78-10/98: $4,411 on 36 trades, 58%, PF 4.75. The book also enabled Format-dialog stops whose dollar amounts it does not print; they are not modelled.",
      ("stad6_parvol", "Parabolic & Volume", "Vol 6")) },
    Def { needs_volume: true, ..d("stad6_vwm", "Volume-Weighted Momentum", "Vol 6 · Volatility breakout", "Long/Short", Live,
      &["F|VWM = XAverage(Volume * Momentum(Close,5), 20); ATR = AvgTrueRange(5) (Vol 6 p.90 code)",
        "F|VWM crosses above 0: LSetup = 0, LEPrice = Close (sell mirror); counters +1 every bar",
        "E|LSetup <= SetupLen(5): Buy next bar at LEPrice + .25*ATR stop and ExitLong next bar at Low - .25*ATR stop (Vol 7 p.87 erratum: ATR, not ATRLen) (short mirror)",
        "X|MP = 1: LSetup = SetupLen; VWM crosses below 0: ExitLong next bar at market (short mirror)"],
      "Vol 6 ch.4 p.88-96. LSetup/LEPrice start at 0, so the first bars place a buy stop near zero (literal). Needs volume. Book: Intel long-only: 62 trades, 50%, win/loss 5.88. The book also enabled Format-dialog stops whose dollar amounts it does not print; they are not modelled.",
      ("stad6_vwm", "Volume-Weighted Momentum", "Vol 6")) },
    ]
}

fn defs_b() -> Vec<Def> {
    use Status::*;
    let d = |id, name, group, dir, status, rules, notes, family| Def { id, name, group, dir, status, needs_volume: false, rules, notes, family };
    vec![
    // ================= Volume 7 (1999)
    d("stad7_dow", "Dow Theory (Chris Evans)", "Vol 7 · Trending", "Long",
      NeedsData("Needs four aligned streams: a Dow 30 stock (Data1), the Dow Industrials (Data2), Dow Transports (Data3) and Dow Utilities (Data4). The vault has the Industrials (DJI) only - no Transports or Utilities and no single stocks with the indices aligned."),
      &["E|High Data1 > Highest(High Data1,50)[1] starts a 10-bar confirmation window; all three Dow averages make new 50-bar highs in it -> Buy next bar at market (Vol 7 p.15 code)",
        "X|ExitLong next bar at highest High since entry - 5*AvgTrueRange(10) stop"],
      "Vol 7 ch.2 p.14-20. Book (parameters 3/6/5/10, 1/2/90-4/1/98): CAT 31 trades 58%; T 31 trades 42%; KO 24 trades 63% ...", ("stad7_dow", "Dow Theory", "Vol 7")),
    d("stad7_dma", "Derivative Moving Average (Hans Stimming)", "Vol 7 · Trending", "Long/Short", Live,
      &["F|AvgVal = Average(Close,9); DerivMA1 = (AvgVal - AvgVal[9])/9; DerivMA2 = DerivMA1 - DerivMA1[1] (Vol 7 p.25 code)",
        "F|DerivMA2 crosses above 0: BuyFlag = True; DerivMA2 crosses below 0: SellFlag = True",
        "E|DerivMA1 crosses above 0 and BuyFlag: Buy next bar at Close + 1 point stop, BuyFlag = False",
        "E|DerivMA1 crosses below 0 and SellFlag: Sell next bar at Close - 1 point stop, SellFlag = False",
        "X|Stop and reverse only (no stops; p.27)"],
      "Vol 7 ch.2 p.21-30. Book: S&P 500 10/1/90-10/1/98: 42% winners, profit factor 1.61, return on account 315%.", ("stad7_dma", "Derivative Moving Average", "Vol 7")),
    d("stad7_stsr", "Short Term Support/Resistance (SROsc)", "Vol 7 · Support/Resistance", "Long/Short", Live,
      &["F|If TrueRange <> 0: SROsc = ((TrueHigh - Open) + (Close - TrueLow)) / (2*TrueRange) (Vol 7 p.34 code)",
        "F|Not long: SROsc crosses below BuyLev(.30) -> LoOsc = SROsc; SROsc < .30 and < LoOsc -> LoOsc = SROsc; not short: SROsc crosses above SellLev(.70) -> HiOsc, rising highs update",
        "E|SROsc crosses above .30: Buy next bar at market; SROsc crosses below .70: Sell next bar at market",
        "X|SROsc crosses below LoOsc: ExitLong this bar on close; SROsc crosses above HiOsc: ExitShort this bar on close",
        "X|SROsc > .70: ExitLong next bar at Lowest(Low,2) stop; SROsc < .30: ExitShort next bar at Highest(High,2) stop"],
      "Vol 7 ch.3 p.32-39 (oscillator by Art Putt). Book: Boeing 1/95-1/99: 160 trades, 51% profitable, win/loss 1.45, PF 1.52.", ("stad7_stsr", "Short Term S/R", "Vol 7")),
    d("stad7_masr", "Moving Average Support/Resistance", "Vol 7 · Support/Resistance", "Long/Short", Live,
      &["F|ATRVal = AvgTrueRange(10); AvgVal = Average(Close,10) (Vol 7 p.41-42 code)",
        "F|Res <> 0 and Close crosses below AvgVal: if ResFlag then LOrder = Res, ResFlag = False; SuppFlag = True, Supp = Low",
        "F|Supp <> 0 and Close crosses above AvgVal: if SuppFlag then SOrder = Supp, SuppFlag = False; ResFlag = True, Res = High",
        "F|Close > AvgVal and High >= Res: Res = High; Close < AvgVal and Low <= Supp: Supp = Low",
        "E|Close crosses above LOrder: Buy next bar at High + 1 point stop; Close crosses below SOrder: Sell next bar at Low - 1 point stop",
        "X|Long: ExitLong next bar at EntryPrice - 1*ATRVal stop and at PosHigh - 2.5*ATRVal stop (PosHigh = highest High since entry) (short mirror)"],
      "Vol 7 ch.3 p.39-47. Book: Microsoft (4 years): return 52%, PF 1.5, wins 2x losses (no counts printed).", ("stad7_masr", "MA Support/Resistance", "Vol 7")),
    d("stad7_idbo", "Intraday Breakout", "Vol 7 · Support/Resistance", "Long/Short",
      NeedsData("Intraday system (initial 90-minute range, entries on intraday closes, close all trades at the end of the day session); needs intraday bars (series OPEN90_HIGH/OPEN90_LOW and intraday closes)."),
      &["F|Range of the first InitMin(90) minutes of the session (Vol 7 p.49 code)", "E|Close crosses above the range high: Buy this bar on close (once per day); below the low: Sell", "X|ExitLong next bar at SetLow stop; ExitShort at SetHigh stop; close at end of day"],
      "Vol 7 ch.3 p.47-53. Book: 10-minute IBM/AMZN/LU/BA, 1/99.", ("stad7_idbo", "Intraday Breakout", "Vol 7")),
    d("stad7_imsbb", "Intermarket S&P/Bonds/Bills", "Vol 7 · Support/Resistance", "Long",
      NeedsData("Needs weekly Treasury bond (Data2) and Treasury bill (Data3) futures aligned to the traded index; no bond/bill futures in the vault."),
      &["E|Close Data2 > XAverage(Close,30) Data2 and Close Data3 > XAverage(Close,30) Data3 and SlowK(9) crosses above 30: buy stop at Close + .30*ATR for 5 bars (Vol 7 p.55 code)",
        "X|ExitLong at EntryPrice - 1.5*ATR stop; after SlowK > 70: ExitLong at Lowest(Low,2)[1] stop"],
      "Vol 7 ch.3 p.54-60. Book: 57% profitable, win/loss 1.43, PF 1.88.", ("stad7_imsbb", "Intermarket S&P/Bonds/Bills", "Vol 7")),
    d("stad7_fibret", "Fibonacci Retracement (intraday)", "Vol 7 · Support/Resistance", "Long/Short",
      NeedsData("Intraday system (range until StartTme 12:00, limit entries at .50/.33 retracements, one trade per day); needs intraday bars."),
      &["E|After 12:00, high of the range more recent: buy limits at the .50 and .33 retracements (Vol 7 p.61-63 code)", "X|Stop at EntryPrice - 2*Average(Range,4), then StopPrice + (Low - StopPrice)/2 each bar"],
      "Vol 7 ch.3 p.60-68. Book: 5-minute S&P, 10 days: 71% profitable.", ("stad7_fibret", "Fibonacci Retracement", "Vol 7")),
    d("stad7_trbo", "Trading Range Breakout", "Vol 7 · Volatility breakout", "Long/Short", Live,
      &["F|RangeH = Highest(High,7)[1]; RangeL = Lowest(Low,7)[1]; TRange = RangeH - RangeL; ATR = AvgTrueRange(2) (Vol 7 p.75-76 code)",
        "F|NoTrades = sum over the previous 7 bars of (RangeH - High) where High < RangeH plus (Low - RangeL) where Low > RangeL",
        "F|Condition1 = NoTrades >= TRange*RngPcnt(200)/100; Condition2 = TrueRange > AvgTrueRange(7)[1]",
        "E|Cond1 and Cond2 and Close > RangeH and MedianPrice > High[1] and not long: Buy this bar on close, LongRisk = RangeL, LongHigh = High",
        "E|Cond1 and Cond2 and Close < RangeL and MedianPrice < Low[1] and not short: Sell this bar on close, ShortRisk = RangeH, ShortLow = Low",
        "X|Long: LongHigh = max(High); ExitLong next bar at LongRisk - 1 point stop and at LongHigh - 8*ATR stop (short mirror)"],
      "Vol 7 ch.4 p.74-82 (after Alex Saitta). Book: Microsoft 4 years: 50% winners, PF 2.32, win/loss 2.32, return 1795%.", ("stad7_trbo", "Trading Range Breakout", "Vol 7")),
    // ================= Volume 8 (1999)
    d("stad8_escalator", "Escalator (2-Bar Reversal)", "Vol 8 · Strategies", "Long/Short", Live,
      &["F|Long: Close[1] <= Low[1] + .25*Range[1] and Close >= High - .25*Range and Close above Average(Close,8) and Average(Close,40) (Vol 8 p.21 code)",
        "E|Long setup and MP <> 1: Buy next bar at Highest(High,2) + 1 point stop; LongRisk = Lowest(Low,RiskLength=2) - 1 point",
        "X|ExitLong next bar at LongRisk stop (every bar); MP = 1: ExitLong next bar at EntryPrice + 2*LongRisk limit (as printed - a price, so it never fills; labelled option target_mode=1 uses 2x the risk distance as the prose says)",
        "E|Short mirror: Close[1] in the top 25%, Close in the bottom 25%, below both averages: Sell next bar at Lowest(Low,2) - 1 point stop; ShortRisk = Highest(High,2) + 1 point",
        "X|ExitShort next bar at ShortRisk stop; MP = -1: ExitShort next bar at EntryPrice - 2*ShortRisk limit"],
      "Vol 8 ch.2 p.19-31. Book: T-bonds 1/84-3/99 (optimised 12/20/4 long, 4/20/5 short): $61,472 on 98 trades, 45%, win/loss 2.08; GM long-only: 47 trades, 45%.",
      ("stad8_escalator", "Escalator", "Vol 8")),
    d("stad8_jib", "Jack-in-the-Box", "Vol 8 · Strategies", "Long/Short", Live,
      &["F|TrendHigh/TrendLow start at the first bar's High/Low; Low > TrendHigh: CurrentTrend = 1, TrendHigh/Low = High/Low, BuyFlag, Counter = 0; High < TrendLow: mirror (Vol 8 p.47-48 code)",
        "F|CurrentTrend = 1 and High > TrendHigh: TrendHigh/Low = High/Low (downtrend mirror); MP = 1 or Counter >= QBars(5): BuyFlag = False (mirror); Counter +1",
        "E|BuyFlag: Buy next bar at Open of next bar + .50*AvgTrueRange(10) stop; SellFlag: Sell next bar at Open of next bar - .50*ATR stop",
        "X|Parabolic Trailing LX (code inputs Acceleration .2, FirstBarMultp 1.5; the table prints .02 - code wins): first bar StopPrice = Low - 1.5*Average(TrueRange,3), then StopPrice += AF*(HighValue - StopPrice), AF += step on new highs up to .2, capped at Low",
        "X|Parabolic Trailing SX (table: Acceleration .02, FirstBarMultp 3; code not listed separately)"],
      "Vol 8 ch.4 p.45-59. Book: British Pound 1/84-3/99 (optimised): $73,220 on 150 trades, 41%; IBM long-only: 44 trades, 68%.", ("stad8_jib", "Jack-in-the-Box", "Vol 8")),
    d("stad8_abpd", "ActivityBar Price Distribution", "Vol 8 · Strategies", "Long/Short",
      NeedsData("Uses ActivityBar data (1-minute activity inside 60-minute bars: AB_ModePrice, AB_GetZoneLow); needs intraday tick/minute distributions."),
      &["E|Open of next bar > ModePrice and Average(Close,3) rising: Buy next bar at market (Vol 8 p.63 code)", "X|Mode/average reversals exit next open or this close"],
      "Vol 8 ch.5 p.61-68. Book: 60-minute Microsoft and Dell.", ("stad8_abpd", "ActivityBar Price Distribution", "Vol 8")),
    d("stad8_ryanshope", "Ryan's Hope (Ryan Jones)", "Vol 8 · Strategies", "Long/Short", Live,
      &["E|AvgVal = Average(Close,Length1=20); AvgVal > AvgVal[Length2=5] and Close < Close[5] and Close > Close[25]: Buy next bar at market (Vol 8 p.79-80 code)",
        "E|AvgVal < AvgVal[5] and Close > Close[5] and Close < Close[25]: Sell next bar at market",
        "X|ATR Protective 3, ATR Trailing 4, ATR Volatility 2 (printed code: EntryPrice -/+ 2*ATR), ATR Big Profit 7 with 3-bar exit, ATR length 10 (strategy inputs, Vol 8 p.79; common exits ch.9 p.101-107)"],
      "Vol 8 ch.7 p.77-88. Book: crude oil 1/84-3/99 (optimised 20/3, IPS 3, TS 5, VolSt 2, BP 6/8): $75,198 on 173 trades, 49%, win/loss 2.13, PF 2.06.", ("stad8_ryanshope", "Ryan's Hope", "Vol 8")),
    d("stad8_stampede", "Stampede (pyramiding channel breakout)", "Vol 8 · Strategies", "Long/Short", Live,
      &["F|CC = CurrentContracts; CC > CC[1] and MP = MP[1] and Counter < MaximumEntries(10): Counter +1; MP <> MP[1]: Counter = 1 (Vol 8 p.91-92 code)",
        "E|Counter < MaximumEntries: Buy next bar at Highest(High,ChanLength=20) + 1 point stop and Sell next bar at Lowest(Low,20) - 1 point stop; each same-direction fill adds one unit (pyramiding)",
        "X|ATR Protective 3, ATR Trailing 4, ATR Volatility 2, ATR Big Profit 7 with 3-bar exit, ATR length 10 (strategy inputs p.91; prose defaults 30/9/3/4/3/8/5 differ - strategy inputs win)"],
      "Vol 8 ch.8 p.89-99. EntryPrice/BarsSinceEntry of a pyramided position = the first entry (assumption). Book (optimised per market, 1/84-3/99): BP 1,056 trades, 42.7%; crude 772 trades, 52.7%; ...", ("stad8_stampede", "Stampede", "Vol 8")),
    // ================= Volume 9 (1999)
    d("stad9_dyf", "Double Your Fun", "Vol 9 · Strategies", "Long/Short", Live,
      &["F|DMA = Average(Close,5)[5] (Vol 9 p.13-14 code)",
        "F|Close crosses above DMA and MRO(Close crosses below DMA, SetupQ2=5, 1) <> -1 and MRO(Close crosses above DMA,50,2) - MRO(Close crosses below DMA,5,1) <= SetupQ1(3): BuySetup, BuyPrice = High, BuyCount = 0 (sell mirror, SellPrice = Low)",
        "E|BuySetup and BuyCount <= EntrySetup(4): if long then BuySetup = False else BuyCount +1 and Buy next bar at BuyPrice + 1 point stop; otherwise BuySetup = False",
        "E|SellSetup mirror: Sell next bar at SellPrice + 1 point stop (as printed in the code listing; the explanation says - 1 point - code wins)",
        "X|DMA Stop: ExitLong next bar at DMA - 1 point stop; ExitShort at DMA + 1 point stop",
        "X|Trailing Stop LX/SX: ExitLong next bar at LowestFC(Low,5) stop; ExitShort at HighestFC(High,5) stop (every bar)"],
      "Vol 9 ch.1 p.11-27 (idea of Joe DiNapoli). Book (optimised): Sears long-only 33 trades 45%; British Pound 97 trades 43%.", ("stad9_dyf", "Double Your Fun", "Vol 9")),
    d("stad9_luxor", "LUXOR (Vol 9)", "Vol 9 · Strategies", "Long/Short", Live,
      &["F|Fast = TriAverage(Close,5); Slow = TriAverage(Close,20); ADX(14); ADXSetup = ADX > ADX[FastLength] (Vol 9 p.31-32 code)",
        "F|BuySetup = Fast > Slow and ADXSetup and High < Fast; SellSetup = Fast < Slow and ADXSetup and Low > Fast",
        "E|(Fast > Slow and not long) or (BuySetup and CurrentContracts < 3): if not long Buy next bar at High + 1 point stop, else Buy next bar at Fast + 1 point stop (pyramid add) (short mirror)",
        "X|Long: ExitLong next bar at Slow - 1 point stop; short: ExitShort next bar at Slow + 1 point stop"],
      "Vol 9 ch.2 p.29-38. TriAverage = Average of Average, each int(Length/2)+1 bars (the book's definition). Book (optimised 5/30/12, D-Mark): 382 trades, 32%, PF 1.52.", ("stad_luxor", "LUXOR", "Vol 9")),
    d("stad9_nohurry", "No Hurry (delayed channel breakout)", "Vol 9 · Strategies", "Long/Short", Live,
      &["F|UpperChan = Highest(High,20)[15]; LowerChan = Lowest(Low,20)[15]; PositionFlag = direction of the latest position (Vol 9 p.41 code)",
        "E|PositionFlag <> 1 and MP <> 1: Buy next bar at UpperChan + 1 point stop; PositionFlag <> -1 and MP <> -1: Sell next bar at LowerChan - 1 point stop",
        "X|ATR Protective 3, ATR Breakeven 2, ATR Trailing 5, ATR length 10 (strategy inputs p.40)"],
      "Vol 9 ch.3 p.39-48. Book: IBM long-only (40/15, 4/2/4): 37 trades 51%, PF 3.74; crude (10/15): 93 trades 41%, PF 2.44.", ("stad9_nohurry", "No Hurry", "Vol 9")),
    Def { needs_volume: true, ..d("stad9_obvrev", "OBV Revisited (Weighted OBV)", "Vol 9 · Strategies", "Long/Short", Live,
      &["F|If Range <> 0: WOBV = WOBV + ((Close - Open)/Range)*Volume; SMA = Average(WOBV,25) (Vol 9 p.51-52 code)",
        "E|WOBV crosses above SMA: BuySetup, LEPrice = High, SellSetup = False, ExitShort this bar on close; crosses below: mirror (ExitLong this bar on close)",
        "E|MP = 1 cancels BuySetup (mirror); BuySetup: Buy next bar at LEPrice + 1 point stop; SellSetup: Sell next bar at SEPrice - 1 point stop",
        "X|ATR Protective 4, ATR Breakeven 4, ATR Trailing 4, ATR length 20 (strategy inputs p.51)"],
      "Vol 9 ch.4 p.49-60. Needs volume. Book: weekly J&J long-only: 21 trades 57%; British Pound: 227 trades 37%.", ("stad9_obvrev", "OBV Revisited", "Vol 9")) },
    d("stad9_redrover", "Red Rover, Red Rover", "Vol 9 · Strategies", "Long/Short", Live,
      &["F|WAvgPrice = (High + Low + 2*Close)/4; Resistance = 2*WAvgPrice - Low; Support = 2*WAvgPrice - High (Vol 9 p.63 code)",
        "E|MP <> 1: Buy next bar at Resistance + 1 point stop; MP <> -1: Sell next bar at Support - 1 point stop (stop and reverse)",
        "X|ATR Profit Target: EntryATR = 2*AvgTrueRange(10) on the entry bar; ExitLong next bar at EntryPrice + EntryATR limit (short mirror) (strategy inputs p.62: target 2 ATRs, length 10 - printed as 'ADX_Length')",
        "F|Labelled option side=1: long entries only; the Sell stop then only closes the long (TradeStation long-only testing; the book's S&P tests)"],
      "Vol 9 ch.5 p.61-70. Book: S&P futures daily ~1984-1999, long only, ATR 7, 3-ATR target, $50 costs: $182,797 on 888 trades, 45%, win/loss 1.76, avg $205.", ("stad9_redrover", "Red Rover", "Vol 9")),
    d("stad9_skinny", "Skinny Dipper (long side)", "Vol 9 · Strategies", "Long", Live,
      &["E|Close > XAverage(Close,50) and Close > XAverage(Close,15) and Close < Low[1]: Buy next bar at High[1] + 1 point stop (Vol 9 p.73 code; prose: the naked-close bar's high - code wins)",
        "X|ATR Breakeven 2, ATR Profit Target 4, ATR Protective 2, ATR length 10 (strategy inputs p.73)"],
      "Vol 9 ch.6 p.71-80. Book: Wal-Mart long-only (70/20, 1/1/10): 27 trades, 33%, PF 11.49; D-Mark: 112 trades, 46%.", ("stad9_skinny", "Skinny Dipper", "Vol 9")),
    d("stad9_skinny_short", "Skinny Dipper (short side)", "Vol 9 · Strategies", "Short",
      NotTestable("Short-side code is not printed ('analogous ... included on the STAD 9 CD', p.74). The printed long code uses the previous bar's high while the prose uses the naked-close bar's extreme, so the mirror is not unambiguous."),
      &["E|Close below both EMAs and above the previous bar's high: sell 1 point below the low of the naked-close bar (prose p.71)"],
      "Vol 9 ch.6 p.71-80.", ("stad9_skinny", "Skinny Dipper", "Vol 9 short")),
    d("stad9_djnikkei", "DJ vs. Nikkei (intermarket example)", "Vol 9 · Strategies", "Long/Short",
      NeedsData("Needs the Dow Jones Industrials as an aligned second data stream for the traded index (Data2). For NIFTY that means DJI aligned to NSE dates; no such cross-market alignment in the engine."),
      &["E|Close[1] of Data2 < Close of Data2: Buy next bar at Open; else Sell next bar at Open (Vol 9 p.82)"],
      "Vol 9 ch.7 p.82-84 (Hans Stimming). Book: Nikkei since 1/1/98: 61% profitable.", ("stad9_iic", "International Index Composite", "DJ vs Nikkei")),
    d("stad9_iic", "International Index Composite", "Vol 9 · Strategies", "Long/Short",
      NeedsData("Needs six aligned foreign index streams (TSE 300, CAC 40, DAX, MIBTel, Nikkei, FTSE 100) as Data2-Data7."),
      &["F|Composite = weighted average of the closes of Data2-Data7 (weights .49/.07/.09/.02/.22/.11) (Vol 9 p.86-87)",
        "E|AverageFC(Composite,9) crosses above AverageFC(Composite,18): Buy next bar at Close + 1 point stop; crosses below: Sell at Close - 1 point stop"],
      "Vol 9 ch.7 p.81-89. Book (S&P 500, optimised 10/9): 81% winners.", ("stad9_iic", "International Index Composite", "Composite")),
    d("stad9_swinger", "Swinger", "Vol 9 · Strategies", "Long/Short", Live,
      &["F|PriceOsc = PriceOscillator(Close,5,20) (Average(5) - Average(20)); AvgFilter = Average(Close,50) (Vol 9 p.93-94 code)",
        "E|PriceOsc > PriceOsc[1] and PriceOsc < 0 and Close > AvgFilter: Buy next bar at market",
        "E|PriceOsc < PriceOsc[1] and PriceOsc > 0 and Close < AvgFilter: Sell next bar at market",
        "X|Long and PriceOsc < PriceOsc[1]: ExitLong next bar at Lowest(Low,3) stop (short mirror)",
        "X|ATR Protective 4, ATR length 10 (strategy inputs p.93)"],
      "Vol 9 ch.8 p.91-101. Book: IBM (optimised 3/10/9/30/40/8): 104 trades, 39%, win/loss 2.82; coffee: 348 trades 34%.", ("stad9_swinger", "Swinger", "Vol 9")),
    d("stad9_trafficjam", "Traffic Jam", "Vol 9 · Strategies", "Long/Short", Live,
      &["F|Condition1 = ADX(14) < ADXLevel(25) and ADX < ADX[MaxList(3,3)]; Condition2 = 3 consecutive down closes; Condition3 = 3 consecutive up closes (Vol 9 p.105 code)",
        "E|Condition1 and Condition2: Buy this bar on close; Condition1 and Condition3: Sell this bar on close",
        "X|Fixed Bar Exit: BarsSinceEntry = 3: exit this bar on close (strategy input 3; the signal's own default is 5)",
        "X|ATR Protective 3, ATR length 5 (strategy inputs p.105)"],
      "Vol 9 ch.9 p.103-112. Book: GM long-only (2/7/14/20): 59 trades, 47%, PF 1.73; crude: 19 trades, 63%.", ("stad9_trafficjam", "Traffic Jam", "Vol 9")),
    ]
}

fn defs_c() -> Vec<Def> {
    use Status::*;
    let d = |id, name, group, dir, status, rules, notes, family| Def { id, name, group, dir, status, needs_volume: false, rules, notes, family };
    vec![
    // ================= Volume 10 (1999)
    d("stad10_serendipity", "Serendipity (Chuck LeBeau's trigger)", "Vol 10 · Strategies", "Long/Short", Live,
      &["E|RSI(Close,9) > OverBought(70): Buy next bar at Open (of the setup bar) + NPtsLE(20) points stop (Vol 10 p.23 code; Serendipity trigger p.19-20)",
        "E|RSI(Close,9) < OverSold(30): Sell next bar at Open - NPtsSE(20) points stop",
        "X|ATR Breakeven 4, ATR Profit Target 4, ATR Protective 3, ATR length 10 (strategy inputs p.22)"],
      "Vol 10 ch.2-3 p.19-30. 'Points' = minimum price moves (param point, 0.01). Book: T-bonds ~15 years (optimised 2/10/8/5, 20/5, RSI 11, 60/35): $64,431 on 152 trades, 30%, win/loss 3.35.", ("stad10_serendipity", "Serendipity", "Vol 10")),
    d("stad10_eightball", "Eight Ball", "Vol 10 · Strategies", "Long/Short", Live,
      &["F|ATR = AvgTrueRange(10); SlowAvg = XAverage(Close,40); FastAvg = XAverage(Close,8) (Vol 10 p.33-34 code)",
        "E|SlowAvg > SlowAvg[1] and FastAvg < FastAvg[1]: Buy next bar at Open of next bar + Trigger(1)*ATR stop (the code's countertrend setup; Vol 11 p.159 confirms the CD code; prose 'both rising' = labelled option prose=1)",
        "E|SlowAvg < SlowAvg[1] and FastAvg > FastAvg[1]: Sell next bar at Open of next bar - 1*ATR stop",
        "X|ATR Big Profit 7 (3-bar exit), ATR Protective 3, ATR Trailing 4, ATR Volatility 2 (EntryPrice -/+ 2*ATR), ATR length 10 (strategy inputs p.33)"],
      "Vol 10 ch.4 p.31-47. Book (optimised): IBM long-only 24 trades 58%, PF 8.13; T-bonds 54 trades 48%; Swiss franc 218 trades 33%.", ("stad10_eightball", "Eight Ball", "Vol 10")),
    d("stad10_goinstyle", "Going In Style", "Vol 10 · Strategies", "Long/Short", Live,
      &["F|ATR = AvgTrueRange(Length=20); Condition1 = High > Highest(High,20)[1]; Condition2 = Low < Lowest(Low,20)[1] (Vol 10 p.57 code)",
        "E|Condition1: Buy next bar at Close + ATR*Trigger(.66) stop; Condition2: Sell next bar at Close - ATR*Trigger stop (the listing writes 'Constant'/'ATRt', undefined names; the signal's input table defines Trigger as the multiple used for the entry prices)",
        "X|Parabolic Trailing LX (Acceleration .02, FirstBarMult 1.5) and SX (.02, 3) (strategy inputs p.56)"],
      "Vol 10 ch.6 p.55-67. Book (optimised): coffee 30/.50: 88 trades 42%, PF 2.17; weekly IBM long-only: 27 trades 56%.", ("stad10_goinstyle", "Going In Style", "Vol 10")),
    d("stad10_wmts", "Who's Minding the Store", "Vol 10 · Strategies", "Long/Short", Live,
      &["F|StochK = SlowK(14); StochD = SlowD(14); BFlag = SetupLength(2) when long else +1 (SFlag mirror) (Vol 10 p.72 code)",
        "F|Condition1 = K < D and D < OverSold(40); Condition2 = K > D and D > OverBought(60); KeyRevUp = Low < Lowest(Low,8)[1] and Close > Close[1]",
        "E|BFlag < 2: Buy next bar at Highest(High,4)[BFlag] + 1 point stop and ExitLong next bar at Lowest(Low,8) - 1 point stop; else if Condition1 and KeyRevUp: same orders, BFlag = 0",
        "E|SFlag < 2: Sell next bar at Lowest(Low,4)[SFlag] - 1 point stop and ExitShort at Highest(High,8) + 1 point stop; else if Condition2 and KeyRevUp (printed: KeyRevUp, not KeyRevDn - code wins): same, SFlag = 0",
        "X|EntryBar ATR Stop: MP <> 1: ExitLong next bar at Close - 1*AvgTrueRange(5) stop (short mirror)",
        "X|ATR Trailing 4, ATR Big Profit 7 (3-bar exit), ATR length 10 (strategy inputs p.71)"],
      "Vol 10 ch.7 p.69-80. Book: IBM long-only (optimised NewLow 1, setup 5, trail 7, BP 10): 25 trades, 48%, PF 4.19.", ("stad10_wmts", "Who's Minding the Store", "Vol 10")),
    d("stad10_gemini", "Gemini (weekly setup, daily entry)", "Vol 10 · Strategies", "Long/Short", Live,
      &["F|Data2 = weekly bars of the same symbol: MACDLine = MACD(Close,3,10) Data2; SignalLine = XAverage(MACD,15) Data2 (Vol 10 p.84 code)",
        "F|Signal > 0: DnTrend = False; MACD > Signal and UpFlag -> UpTrend = True, UpFlag = False, DnFlag = True; Signal < 0: mirror",
        "E|UpTrend and MACD < Signal: Buy next bar at Highest(High,EntryLength=3) + 1 point stop; DnTrend and MACD > Signal: Sell next bar at Lowest(Low,3) - 1 point stop; MP = 1 clears UpTrend (mirror)",
        "X|ExitLong next bar at Lowest(Low,4) Data2 - 1 point stop; ExitShort at Highest(High,4) Data2 + 1 point stop (weekly)",
        "X|Trailing Stop LX/SX length 3 (LowestFC/HighestFC); Dllr Risk Trailing $1,000 and Breakeven floor $1,000 (IBM test, per 100 shares: 10 points each)"],
      "Vol 10 ch.8 p.81-94. Weekly bars are built from the daily file (Monday-Friday weeks); a daily bar sees the last completed week. Book: IBM long-only: 20 trades, 40%, PF 2.84; Yen: 25 trades, 56%.", ("stad10_gemini", "Gemini", "Vol 10")),
    d("stad10_refdev", "Reference Deviation (William Brower)", "Vol 10 · Strategies", "Long/Short", Live,
      &["F|RMA = Average(Close,15); DRD = Close - RMA; RDV = 100*Summation(DRD,15)/Summation(|DRD|,15) (Vol 10 p.96-97 code)",
        "E|RDV > ETLong(5): Buy next bar at market; RDV < ETShort(-5): Sell next bar at market",
        "X|RDV < 0: ExitLong next bar at market; RDV > 0: ExitShort next bar at market"],
      "Vol 10 ch.9 p.95-102. Book: futures portfolio 1980-98 (per-market MA lengths), e.g. Yen MA 15 in-sample 73 trades 48%.", ("stad10_refdev", "Reference Deviation", "Vol 10")),
    d("stad10_superman", "Superman", "Vol 10 · Strategies", "Long/Short", Live,
      &["F|MarketStrength(5): net of the last 5 close-to-close changes / (sum of up changes if the net >= 0, else sum of down changes) * 100 (Vol 10 p.103 text); DollarsPerBar1 = (C - C[4])/4 >= 0 (Cond1); DollarsPerBar2 = (C[4] - C[8])/5 >= 0 (Cond2) (p.105 code)",
        "E|MS >= 95 and Cond1 and not Cond2 and High = Highest(High,5) and MP <> 1: Buy next bar at Highest(High,5) stop; ExitShort this bar on close; StopLoss = Lowest(Low,StopLen=6); ExitLong next bar at StopLoss stop",
        "E|MS <= -95 and not Cond1 and Cond2 and Low = Lowest(Low,5) and MP <> -1: Sell next bar at Lowest(Low,5) stop; ExitLong this bar on close; StopLoss = Highest(High,6); ExitShort at StopLoss stop",
        "X|MP = 1: ExitLong next bar at StopLoss stop and at EntryPrice + (EntryPrice - StopLoss)*ProfitFactor(1) limit (short mirror)",
        "X|Stop Loss signal: SetStopLoss($100) (Alcoa test, per 100 shares: 1 point)"],
      "Vol 10 ch.10 p.103-114. Book (optimised): weekly Alcoa 5 trades 80%; weekly Merck 17 trades; unleaded gas 78 trades 38%; coffee 122 trades 41%.", ("stad10_superman", "Superman", "Vol 10")),
    // ================= Volume 11 (2000)
    d("stad11_highfive", "High Five", "Vol 11 · Strategies", "Long/Short", Live,
      &["F|Five Average(Close, 5*2^(k-1)) = 5/10/20/40/80; LongCounter = number rising, ShortCounter = number falling; MP <> MP[1] clears both setups (Vol 11 p.14 code)",
        "E|LongCounter = 5: BuySetup, LEntryValue = Close + Range*Multiplier(.2); MP <> 1 and BuySetup: Buy next bar at LEntryValue stop (sell mirror)",
        "X|LongCounter <= 2: ExitLong next bar at market; ShortCounter <= 2: ExitShort next bar at market",
        "X|ATR Protective 3, ATR Breakeven 4, ATR length 10 (strategy inputs p.13)"],
      "Vol 11 ch.1 p.11-26. Multiplier .2 is the input default (prose says 50%). Book (optimised): AXP 10 trades 80%; CSCO 13 trades 61%; Yen 46 trades 47%.", ("stad11_highfive", "High Five", "Vol 11")),
    d("stad11_bullseye", "Hit the Bull's Eye", "Vol 11 · Strategies", "Long/Short", Live,
      &["F|Long: High = Highest(High,RangeLength=5) and Close > High[2] and XAverage(Close,50) rising (Vol 11 p.29-30 code)",
        "E|Long setup and MP <> 1: BuyEntry = MedianPrice[2], BuyCounter = 0, LongExitTarget = High + Factor(3)*(High - BuyEntry); while BuyCounter <= BarstoEnter(5): Buy next bar at BuyEntry limit",
        "X|MP = 1: ExitLong next bar at LongExitTarget limit",
        "E|Short: Low = Lowest(Low,5) and Close < Low[2] and XAverage falling: Sell next bar at SellEntry = MedianPrice[2] limit; ShortExitTarget = Low - 3*(SellEntry - Low)",
        "X|ATR Protective 3, ATR length 10 (strategy inputs p.29)"],
      "Vol 11 ch.2 p.27-44. Book (optimised): IBM 27 trades 48%, PF 5.15; MSFT 19 trades 73%; crude 52 trades 55%.", ("stad11_bullseye", "Hit the Bull's Eye", "Vol 11")),
    d("stad11_miser", "MISER (Murray's Intermarket Strategy Revisited)", "Vol 11 · Strategies", "Long",
      NeedsData("Needs US Treasury Bond futures as Data2 aligned to the traded index (Average(Close of Data2,26)); no bond series in the vault."),
      &["E|Close < Average(Close,16) and Close of Data2 > Average(Close of Data2,26): Buy next bar at market (Vol 11 p.48 code)",
        "X|Close > SPAvg and bonds below their average: ExitLong next bar at market; ATR Breakeven 4, Big Profit 7/3, Protective 3, Trailing LX 3 (+ entry-bar version), Volatility 2"],
      "Vol 11 ch.3 p.45-56. Book: S&P long-only 2/92-11/99 (optimised): $178,727 on 42 trades, 78%.", ("stad11_miser", "MISER", "Vol 11")),
    d("stad11_exits", "Chandelier and Yo-Yo exits (LeBeau & Tan)", "Vol 11 · Articles", "Long/Short",
      NotTestable("Exit techniques only: the printed examples use placeholder entries ('Buy_Condition'), so there is no complete system to test."),
      &["X|Chandelier: highest high (or close) since entry - 3*AvgTrueRange(20) stop", "X|Yo-Yo: Close -/+ 2*AvgTrueRange(20) stop"],
      "Vol 11 ch.4 p.57-64.", ("stad11_exits", "Chandelier / Yo-Yo exits", "Vol 11")),
    d("stad11_nighttrain", "JK Night Train (Joe Krutsinger)", "Vol 11 · Strategies", "Long/Short", Live,
      &["E|Close < Close[2]: Buy next bar at High stop; Close >= Close[2]: Sell next bar at Low stop (Vol 11 p.67 code)",
        "X|MP = 1 and Open of tomorrow > EntryPrice: ExitLong next bar at market; MP = -1 and Open of tomorrow < EntryPrice: ExitShort next bar at market (first profitable open)",
        "X|ATR Protective 3, ATR length 10 (strategy inputs p.66)",
        "X|Labelled options: prot (book's S&P test 2.5, MSFT 6.5); sl_usd/bpv: Joe's NASDAQ 100 test used a $1,000 dollar risk stop instead; side=1 long entries only, the Sell stop only closes longs (book's S&P test)"],
      "Vol 11 ch.5 p.65-74. Book: NASDAQ 100 4/96-10/99 long+short, $1,000 stop, $55 costs: 333 trades, 49%, win/loss 1.6, PF 1.54; S&P long-only 1/98-10/99 (2.5 ATR): 59 trades, 88%, win/loss .26, PF 1.93.", ("stad11_nighttrain", "JK Night Train", "Vol 11")),
    d("stad11_symtri", "Symmetrical Triangle", "Vol 11 · Strategies", "Long/Short", Live,
      &["F|DMISpread = DMIPlus(14) - DMIMinus(14); swing points of strength 1; far/near pivot highs and lows kept as printed in DMILongTriangle / DMIShortTriangle (Vol 11 p.78-80, 86-88 code)",
        "F|Long triangle: PivotHigh[2] > PivotHigh[1], PivotLow[2] < PivotLow[1], points alternate with Spacing(1), within EvaluationRange(40), DMISpread at the far high >= 15, formed on a swing-low bar, upper line above lower line",
        "E|Buy next bar at Resistance + 1 point stop while the formation holds (cancelled when Low crosses below Support or the lines meet) (short: Sell at Support - 1 point, DMISpread <= -15)",
        "X|First bar of the position: profit target EntryPrice + (PivotHigh[2] - PivotLow[2]); protective stop PivotLow[1] - .5*AvgTrueRange(10); Close >= EntryPrice + 2*(EntryPrice - stop) moves the stop to EntryPrice (short code as printed)"],
      "Vol 11 ch.6 p.75-98. Trendline values = straight lines through the two pivots by bar count. Book (optimised): IBM 6 trades 66%; MSFT 7 trades; crude 10 trades 50%.", ("stad11_symtri", "Symmetrical Triangle", "Vol 11")),
    d("stad11_trendscore", "TrendScore (Tushar Chande)", "Vol 11 · Strategies", "Long/Short", Live,
      &["F|TrendScore = sum over v = 1..10 of (+1 if Momentum(Close, LookBack(10)+v) >= 0 else -1); TSA = Average(TrendScore,18); SMA = Average(Close,18) (Vol 11 p.105-106 code)",
        "E|TrendScore > TSA and Close > SMA: Buy next bar at market, else if long: ExitLong next bar at market",
        "E|TrendScore < TSA and Close < SMA: Sell next bar at market, else if short: ExitShort next bar at market",
        "X|ATR Breakeven 4, ATR Protective 3, ATR Trailing 4, ATR length 10 (strategy inputs p.105)"],
      "Vol 11 ch.8 p.103-113. Book (optimised): ADBE 61 trades 32.8%; AXP 33%; Yen 110 trades 31.8%.", ("stad11_trendscore", "TrendScore", "Vol 11")),
    d("stad11_gapper", "Win One for the Gapper", "Vol 11 · Strategies", "Long/Short", Live,
      &["E|ADX(14) > 25 and ADX > ADX[1]: DMIPlus > DMIMinus and Open of next bar < Low: Buy next bar at Low stop; DMIPlus < DMIMinus and Open of next bar > High: Sell next bar at High stop (Vol 11 p.140 code)",
        "X|ATR Big Profit 4 (3-bar exit), ATR Breakeven 4, ATR Protective 3, ATR Trailing 4, ATR length 10 (strategy inputs p.139)"],
      "Vol 11 ch.10 p.137-147 (Larry Williams' Oops). Book (optimised): IBM 12 trades 66%; BP 38 trades 47%; coffee.", ("stad11_gapper", "Win One for the Gapper", "Vol 11")),
    // ================= Volume 12 (2000)
    d("stad12_firstprize", "First Prize", "Vol 12 · Strategies", "Long/Short", Live,
      &["F|MACDLine = MACD(Close,3,10); SignalLine = XAverage(MACD,16); XAvgReq = XAverage(Close,60); ATR = AvgTrueRange(10); UpTrend = Signal > 0, DownTrend = Signal < 0 (Vol 12 p.17-19 code)",
        "F|New uptrend: NewBuySignal; first MACD cross below Signal: BuySetup, CTrendLow = Low; otherwise MRO(MACD > Signal, BarsReq=2, 2) <> -1 cancels; CTrendLow tracks lower lows while MACD < Signal (short mirror)",
        "E|BuySetup and High > XAvgReq: Buy next bar at High + 1 point stop; SellSetup and Low < XAvgReq: Sell next bar at Low - 1 point stop",
        "X|Long: EntryPrice - 3*ATR stop; EntryPrice stop once MaxTradeClose >= EntryPrice + 3*ATR; MaxTradeClose - 6*ATR stop; Close - 3*ATR stop; Lowest(Low,5) stop once MaxTradeClose >= EntryPrice + 9*ATR; CTrendLow - .6*ATR stop; DownTrend and BarsSinceEntry >= 2: Low stop (short mirror)"],
      "Vol 12 ch.1 p.11-30. MaxTradeClose/MinTradeClose = highest/lowest close of the trade (functions not printed; defined by the text and Vol 11 p.61-63). Book: six stocks long-only 1990-99 with defaults earned $34,326 (AXP $7,353 ...).", ("stad12_firstprize", "First Prize", "Vol 12")),
    d("stad12_inthezone", "In The Zone (Joe Stowell)", "Vol 12 · Strategies", "Long/Short", Live,
      &["F|If Close[1] > High[3]: UpperBuyZone = High[3], LowerBuyZone = Lowest(Low,3)[1], BuySetup = Close inside the zone and Close < Close[1]; then LEnterPrice = High + 1 point, LPTExit = High[1] + 5*ATR, LIPSValue = Low - 3*ATR (Vol 12 p.35-36 code)",
        "F|Close < LowerBuyZone cancels BuySetup; short mirror with Close[1] < Low[3] (Close > UpperSellZone cancels)",
        "E|BuySetup and MP <> 1: Buy next bar at LEnterPrice stop; SellSetup and MP <> -1: Sell next bar at SEnterPrice stop",
        "X|Long: Close crosses above EntryPrice + 3*(EntryPrice - LIPSValue) -> LIPSValue = EntryPrice; ExitLong next bar at LIPSValue stop and LPTExit limit (short: printed 'crosses above' EntryPrice - 3*(SIPS - EntryPrice))"],
      "Vol 12 ch.2 p.31-46. ATR = AvgTrueRange(10); commission 0 (book). Book (optimised): QCOM 75% profitable; SUNW 84%; coffee 48%; bonds 44%.", ("stad12_inthezone", "In The Zone", "Vol 12")),
    d("stad12_jailbreak", "Jailbreak", "Vol 12 · Strategies", "Long/Short", Live,
      &["E|Buy next bar at Highest(High,Length1=50) + 1 point stop; Sell next bar at Lowest(Low,50) - 1 point stop (Vol 12 p.50 code)",
        "X|Long: ExitLong next bar at EntryPrice - IPS(4)*AvgTrueRange(10) stop and at Lowest(Low,L2) - 1 point stop, L2 = MinList(Length1, Length2=30) (short mirror)"],
      "Vol 12 ch.3 p.47-56. Book: IBM long-only (60/20/2): 25 trades 52%, PF 6.71; bonds (80/40/4): 18 trades 72%.", ("stad12_jailbreak", "Jailbreak", "Vol 12")),
    d("stad12_lrcross", "LR Crossing (Julio Zayas)", "Vol 12 · Strategies", "Long/Short", Live,
      &["E|LinearRegValue(Close,9,0) crosses above LinearRegValue(Close,18,0): Buy next bar at market; crosses below: Sell next bar at market (Vol 12 p.59 code)",
        "X|Profit Target, Pcnt Risk Trailing and Stop Loss signals with input defaults 0 (off) (strategy inputs p.58); the book's coffee/S&P dollar settings are labelled options"],
      "Vol 12 ch.4 p.57-68. Book: 1000 days of coffee and S&P (with dollar stops): coffee 109 trades ~50%; S&P 122 trades ~$70,000.", ("stad12_lrcross", "LR Crossing", "Vol 12")),
    d("stad12_ntw", "Not That Way, The Other Way", "Vol 12 · Strategies", "Long/Short", Live,
      &["F|Indicator = MACD(Close,3,10), SignalLine = XAverage(Indicator,16); price and MACD swing points of Strength(2) aligned within Alignment(2) bars (Vol 12 p.75-77, 84-86 code)",
        "F|Bullish reverse divergence: price swing low X < Z (higher low) while MACD swing X > Z; then MACD crosses above Signal: EntryStop = Close + 50% of (Z - X), setup for 50% of the bars between X and Z",
        "E|EntrySetup and MP <> 1: Buy next bar at EntryStop stop (bearish mirror: Sell at Close - 50% of (X - Z))",
        "X|First bar: ExitStop = EntryPrice -/+ 50% of the pivot difference, ProfitTarget = EntryPrice +/- the printed IntervalPeak/IntervalTrough; Close crossing EntryPrice +/- that amount moves the stop to EntryPrice"],
      "Vol 12 ch.5 p.69-96. The bullish listing (IntervalPeak) is used where the explanation repeats the bearish text. Book: IBM long-only 15 trades 66%; crude 76 trades 51%.", ("stad12_ntw", "Not That Way, The Other Way", "Vol 12")),
    d("stad12_shifty", "Seems Kinda Shifty", "Vol 12 · Strategies", "Long/Short", Live,
      &["F|FastAvg = Average(Close,Fast=30); SlowAvg = Average(Close,Slow=60) (strategy inputs p.99; the signal's own defaults 10/20 are overridden)",
        "E|Close > SlowAvg: Buy next bar at FastAvg + 1 point stop; Close < SlowAvg: Sell next bar at FastAvg - 1 point stop (Vol 12 p.100 code)",
        "X|Close crosses under FastAvg records ThisCrossUnder (previous -> LastCrossUnder); ThisCrossUnder < LastCrossUnder and long: ExitLong next bar at FastAvg - 1 point stop, else Close crosses below SlowAvg: ExitLong this bar on close (short mirror)",
        "X|Dllr Risk Trailing $1,000 (CSCO test, per 100 shares: 10 points)"],
      "Vol 12 ch.6 p.97-108. Book (optimised 40/80): CSCO long-only 64 trades 45%, PF 5.15; crude 200 trades 29%.", ("stad12_shifty", "Seems Kinda Shifty", "Vol 12")),
    d("stad12_springs", "Springs & Upthrusts", "Vol 12 · Strategies", "Long/Short", Live,
      &["F|Spring: SwingLow(1,Low,5,6) sets PivotLow = Low[5], Length = 5; Length +1 up to 20 (at 20 the setup is cancelled) (Vol 12 p.113 code)",
        "F|Low < PivotLow and Close > PivotLow and Close >= High - 25% of Range and Range >= 150% of AvgTrueRange(MaxList(Length,3)): Spring, EnterPrice = Open of tomorrow + 50% of Range, PriceToExit = PivotLow - 1 point",
        "E|Spring: Buy next bar at EnterPrice stop; MP = 1: ExitLong next bar at PriceToExit stop (Upthrust mirror on swing highs)",
        "X|Parabolic ModStop LX/SX (strategy inputs: Acceleration .02, FirstBarMult 1.5)"],
      "Vol 12 ch.7 p.109-124 (Wyckoff). Book: NXTL (optimised) 69% profitable, PF 6.05; Swiss franc 67 trades 43%.", ("stad12_springs", "Springs & Upthrusts", "Vol 12")),
    d("stad12_musketeers", "Three Musketeers", "Vol 12 · Strategies", "Long/Short", Live,
      &["F|ATR = AvgTrueRange(10); DLine = SlowD(12); UpTrend = DLine > 50; DownTrend = DLine < 50 (Vol 12 p.128-129 code)",
        "E|MP <> 1 and UpTrend and Low < Low[2]: Buy next bar at High stop, Risk = Low - 1 point; MP <> -1 and DownTrend and High > High[2]: Sell next bar at Low stop, Risk = High + 1 point (one shared Risk variable, as printed)",
        "X|Long: ExitLong next bar at Risk stop and at EntryPrice + 6*(EntryPrice - Risk) limit; MaxTradeClose >= EntryPrice + 3*ATR: ExitLong next bar at EntryPrice stop (short mirror)"],
      "Vol 12 ch.8 p.125-137. Book (optimised): MSFT long-only ~40% profitable, PF 2.79; crude 231 trades 33%.", ("stad12_musketeers", "Three Musketeers", "Vol 12")),
    d("stad12_mist2", "Tigers in the Mist #2 (Walter Downs)", "Vol 12 · Strategies", "Long/Short", Live,
      &["F|Condition1 = High > Highest(High,3)[1] and Close > Close[1]; Condition2 = Low > Lowest(Low,3)[1] and Close < Close[1] (printed '>' - code wins) (Vol 12 p.142 code)",
        "E|Condition1[1] and Open > High[1] and Close < High[1]: Buy('L2') next bar at High stop; Condition2[1] and Open < Low[1] and Close > Low[1]: Sell('S2') next bar at Low stop",
        "X|ExitLong from entry L2 at AT$ Low stop; ExitShort from entry S2 at AT$ High stop (the signal bar's low/high)",
        "X|Parabolic ModStop LX/SX (strategy inputs: Acceleration .03, FirstBarMultiplier 1)"],
      "Vol 12 ch.9 p.139-146. Book: IBM long-only 6 trades 67%; bonds 6 trades 67%.", ("stad12_mist2", "Tigers in the Mist #2", "Vol 12")),
    // ================= Volume 13 "Strategies Revisited" (2000)
    d("stad13_ochist", "Open-Close Histogram (revisited)", "Vol 13 · Revisited", "Long/Short", Live,
      &["F|Histogram = XAverage(Close,30) - XAverage(Open,30); ATR = AvgTrueRange(30) (strategy input Length 30; Vol 13 p.35-36 code)",
        "F|Histogram crosses over 0: BuyPrice = High + .5*ATR, LongExitPrice = Low - .5*ATR; crosses under 0: SellPrice = Low - .5*ATR, ShortExitPrice = High + .5*ATR",
        "E|MRO(Histogram crosses over 0,10,1) > -1 and Histogram > 0: Buy('LE') next bar at BuyPrice stop (short mirror)",
        "X|Histogram crosses under 0: ExitLong this bar at close; ExitLong from entry LE next bar at LongExitPrice stop (short mirror)"],
      "Vol 13 ch.4 p.33-46. Book (optimised): IBM long-only 21 trades 67%; crude 36 trades 42%.", ("stad13_ochist", "Open-Close Histogram", "Vol 13")),
    d("stad13_dmarl", "DMA and Range Leaders (revisited)", "Vol 13 · Revisited", "Long/Short", Live,
      &["F|UpperBand = Average(High,20)[5]; LowerBand = Average(Low,20)[5]; BuyRangeLeader = MedianPrice > High[1] and Range > Range[1] (Vol 13 p.49-50 code)",
        "E|BuyRangeLeader and Close > UpperBand: Buy next bar at High + 1 point stop; SellRangeLeader and Close < LowerBand: Sell next bar at Low - 1 point stop",
        "X|Stop Loss $500, Breakeven floor $500, Dllr Risk Trailing $500 (INTC test, per 100 shares: 5 points each)"],
      "Vol 13 ch.5 p.47-58. Book (optimised): INTC 16 trades 63%; coffee 81 trades 52%.", ("stad13_dmarl", "DMA and Range Leaders", "Vol 13")),
    d("stad13_lrm", "Linear Regression and Momentum (revisited)", "Vol 13 · Revisited", "Long/Short", Live,
      &["F|XLRAverage = XAverage(LinearRegValue(Close,60,0),30); Moment = Momentum(Close,20); ATR = AvgTrueRange(10) (strategy inputs p.62; Vol 13 p.63 code)",
        "F|BuySetup = XLR rising and Close > XLR and Moment < 0 and Moment rising -> BuyPrice = High + .5*ATR, LongCounter = 1 (sell mirror); counters +1 every bar",
        "E|LongCounter <= EntryCounter(3): Buy next bar at BuyPrice stop; ShortCounter <= 3: Sell next bar at SellPrice stop",
        "X|ATR Big Profit 8 (4-bar exit), Trailing 5, Volatility 3, Breakeven 3, Protective 3, ATR length 10"],
      "Vol 13 ch.6 p.59-74. Counters and prices start at 0, so the first bars after MaxBarsBack place a buy stop at 0 (literal). Book (optimised): CSCO 13 trades 77%; Yen 26 trades 65%.", ("stad13_lrm", "Linear Regression and Momentum", "Vol 13")),
    d("stad13_luxor", "LUXOR (revisited)", "Vol 13 · Revisited", "Long/Short", Live,
      &["F|Fast = TriAverage(Close,5); Slow = TriAverage(Close,30); Trending = ADX(15) > ADX(15)[FastLength] (strategy inputs p.78; Vol 13 p.78-79 code)",
        "F|Fast crosses above Slow: BuyStop = High + 1 point; crosses below: SellStop = Low - 1 point",
        "E|Fast > Slow and flat: Buy('Long') next bar at BuyStop stop; long: High < Fast and Trending -> Buy('Longer') next bar at Fast + 1 point stop (pyramid) (short mirror)",
        "X|Long: ExitLong next bar at Slow - 1 point stop; short: ExitShort next bar at Slow + 1 point stop",
        "F|Maximum 3 entries per position (prose p.76; Vol 9 code CurrentContracts < 3) - param maxent"],
      "Vol 13 ch.7 p.75-86. Book (optimised): NXTL 47 trades 51%, PF 2.87; bonds 229 trades 46%.", ("stad_luxor", "LUXOR", "Vol 13")),
    d("stad13_cbi", "Currency/Bonds/Dollar Index (revisited)", "Vol 13 · Revisited", "Long/Short",
      NeedsData("Needs bond futures (Data2) and the US Dollar Index (Data3) aligned to a currency future; none in the vault."),
      &["E|Fast rising, Medium > Slow, bonds' XAverage(50) rising and the Dollar Index's XAverage(50) falling: buy stop High + ATR/2 for 5 bars (Vol 13 p.90 code)", "X|Medium crosses below Slow: ExitLong next bar at market"],
      "Vol 13 ch.8 p.87-98.", ("stad5_curbond", "Currency/Bond", "Vol 13")),
    d("stad13_mrts", "Momentum Retracement (revisited)", "Vol 13 · Revisited", "Long/Short", Live,
      &["F|HighBand/LowBand = XAverage(High/Low,15); CloseLine = XAverage(Close,5); MACD(3,10) with 15-bar signal; RSI(Close,5); DMISpread = DMIPlus(15) - DMIMinus(15) (Vol 13 p.104-106 code)",
        "F|CloseLine crosses above HighBand: ChannelUptrend (below LowBand: ChannelDowntrend); Uptrend = ChannelUptrend and Signal > 0; BullishMomentum = RSI > 70 and DMISpread > 15",
        "F|Uptrend: track UptrendHigh, HighCounter (bars since the high, up to TimetoRetrace 10), HighRSIValue; BullishMomentum starts CheckRetracement",
        "E|HighCounter in 1..9 and within HighCounter bars: Low < HighBand, MACD crossed below Signal, RSI crossed below 50 or below HighRSIValue - 30 -> CounterTrendDecline, BuyStop = Highest(High,BestEntry=2); Buy next bar at BuyStop stop (short mirror)",
        "X|Signal < 0 and DMISpread < 0: ExitLong next bar at market; Signal > 0 and DMISpread > 0: ExitShort next bar at market",
        "X|ATR Big Profit 9 (4-bar exit), Breakeven 4, Protective 3, Trailing 6, Volatility 3, ATR length 10 (strategy inputs p.103; the test list prints Breakeven 3 - inputs win)"],
      "Vol 13 ch.9 p.99-121. Book (optimised): QCOM 12 trades 75%; coffee 13 trades 69%.", ("stad13_mrts", "Momentum Retracement", "Vol 13")),
    Def { needs_volume: true, ..d("stad13_add", "Advance-Decline Divergence (revisited)", "Vol 13 · Revisited", "Long/Short", Live,
      &["F|If High - Low <> 0: ADLine = (Close - Open)/(High - Low)*Volume, ADVolume = XAverage(ADLine,9); LowADVolume/HighADVolume = Lowest/Highest(ADVolume,30) (Vol 13 p.126-127 code)",
        "E|Low < Lowest(Low,30)[1]: ADVolume > HighADVolume[1] -> Buy('Strong Buy') 2 units next bar at market; else MRO(ADVolume < LowADVolume[1],5,1) = -1 -> Buy('Weak Buy') 1 unit (short mirror on new 30-bar highs)",
        "X|First bar long: StopPrice = Low - Average(Range,4); ExitLong next bar at StopPrice stop; StopPrice += (Low - StopPrice)/3 (short mirror)",
        "X|Stop Loss $500 position basis (GE test: 100 shares per unit -> 5 points per unit)"],
      "Vol 13 ch.10 p.123-134. Needs volume. Book: GE long-only 4/96-4/00: 29 trades, 55%, PF 3.43.", ("stad13_add", "Advance-Decline Divergence", "Vol 13")) },
    d("stad13_tripleplay", "Triple Play (Vol 11 & 13)", "Vol 13 · Revisited", "Long/Short", Live,
      &["F|Bollinger Bands(11, 2); strong swing high (strength 7) above the upper band -> RecentLow = MRO(swing low, StrongLowBar - StrongHighBar, 1) (Vol 13 p.138-140 code; Vol 11 p.118-120 identical with fixed divisors)",
        "F|WeakLow = SwingLow(1,Low,7,8)[Value1] - Value1 is never assigned in the printed code, so it reads the current bar (literal; option weak_src=1 uses RecentLow as the explanation says)",
        "E|Long formation within PatternLength(25)+7 bars: LongEntryPrice = (StrongHigh - StrongLow)/4 + WeakLow; while flat for BarsToEnter(10) bars: Buy 2 units next bar at LongEntryPrice stop (short mirror)",
        "X|2 units: ExitLong at WeakLow - 1 point stop; exit 1 unit at LongTarget1 = (WeakLow + LongTarget2)/2 limit, LongTarget2 = WeakLow + (StrongHigh - StrongLow)",
        "X|1 unit left: ExitLong at EntryPrice stop; after a close above StrongHigh trail at highest close - (StrongHigh - WeakLow)/4; ExitLong at LongTarget2 limit"],
      "Vol 13 ch.11 p.135-151 (after Charles Lindsay's Trident). Book (optimised): IBM long-only 6 trades 50%; BP 12 trades 75%.", ("stad13_tripleplay", "Triple Play", "Vol 13")),
    d("stad13_volex", "VolEx (revisited)", "Vol 13 · Revisited", "Long/Short", Live,
      &["E|Buy next bar at Open of next bar + 1.2*Average(Range,4) stop; Sell next bar at Open of next bar - 1.8*Average(Range,4) stop (Vol 13 p.156 code)",
        "X|Price Trailing Exit: ExitLong next bar at Lowest(Low,4) stop; ExitShort at Highest(High,4) stop",
        "X|Stale Position Exit (0 bars, $0): BarsSinceEntry > 0 and OpenPositionProfit < 0: exit next bar at market",
        "X|First Good Open ($200, delay 0): BarsSinceEntry > 0 and next open beyond EntryPrice +/- 200/BigPointValue: exit next bar at market",
        "X|Stop Loss $1,000 (S&P futures 1999-2000, $250/point: 4 points; GoodOpen 0.8 points)"],
      "Vol 13 ch.12 p.153-162. Book: S&P futures 1/99-4/00 (optimised .75/.75, $2,000, 2-bar trail): $90,490 on 105 trades, 41%.", ("stad13_volex", "VolEx", "Vol 13")),
    d("stad13_volclus", "Volatility Clusters (revisited)", "Vol 13 · Revisited", "Long/Short", Live,
      &["F|CloseToClose = |Close - Close[1]|; CTCDeviations = Average(CTC,15) + 2*StdDev(CTC,15); RangeDeviations = Average(Range,15) + 2*StdDev(Range,15) (strategy input Length 15; Vol 13 p.166-167 code)",
        "E|CTC > CTCDeviations[1] or Range > RangeDeviations[1]: Buy next bar at Close + Average(Range,3)[1] stop and Sell next bar at Close - Average(Range,3)[1] stop",
        "X|First Good Open ($0, delay 0); Stop Loss $300 (AXP test, per 100 shares: 3 points)"],
      "Vol 13 ch.13 p.163-172. Book (optimised): AXP 62 trades 56%; Yen 189 trades 55%.", ("stad13_volclus", "Volatility Clusters", "Vol 13")),
    ]
}

fn defs() -> Vec<Def> { let mut v = defs_a(); v.extend(defs_b()); v.extend(defs_c()); v }

pub fn catalogue() -> Vec<StrategyDef> {
    defs().into_iter().map(|d| StrategyDef { id: d.id, name: d.name, group: d.group, session: 0, dir: d.dir, status: d.status,
        needs_volume: d.needs_volume, needs_vix: false, needs_series: &[], book: BOOK, rules: d.rules, notes: d.notes, claim: None }).collect()
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    defs().into_iter().find(|d| d.id == id).map(|d| d.family)
}

// ------------------------------------------------------------------ parameters (book defaults)

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def: f64, min, max, step| ParamDef { key, label, def, min, max, step };
    let usd = |key, label, def: f64| ParamDef { key, label, def, min: 0.0, max: def.max(1.0) * 5.0, step: (def / 4.0).max(1.0) };
    let bpv = |def: f64, qty: f64| vec![ParamDef { key: "bpv", label: "BigPointValue of the book's test instrument ($ per point)", def, min: 1.0, max: 250000.0, step: 1.0 },
        ParamDef { key: "qty", label: "Shares/contracts in the book's test (dollar stops -> points)", def: qty, min: 1.0, max: 1000.0, step: 1.0 }];
    let atrx = |prot: f64, be: f64, trail: f64, volat: f64, bp: f64, bpn: f64, pt: f64, len: f64| {
        let mut v = vec![];
        if prot > 0.0 { v.push(p("prot", "ATR Protective stop (ATRs)", prot, 0., 10., 0.5)); }
        if be > 0.0 { v.push(p("be", "ATR Breakeven floor (ATRs)", be, 0., 10., 0.5)); }
        if trail > 0.0 { v.push(p("trail", "ATR Trailing stop (ATRs)", trail, 0., 12., 0.5)); }
        if volat > 0.0 { v.push(p("volat", "ATR Volatility stop (ATRs)", volat, 0., 10., 0.5)); }
        if bp > 0.0 { v.push(p("bp", "ATR Big Profit level (ATRs)", bp, 0., 15., 1.)); v.push(p("bpn", "Big Profit exit bars", bpn, 1., 10., 1.)); }
        if pt > 0.0 { v.push(p("atr_pt", "ATR Profit Target (ATRs)", pt, 0., 10., 0.5)); }
        v.push(p("atrlen", "ATR length", len, 2., 40., 1.));
        v
    };
    let mut v: Vec<ParamDef> = match id {
        "stad5_fib382" => [vec![p("xavglen", "XAverage length", 15., 5., 40., 1.), p("hilolen", "High/low range bars", 50., 20., 100., 5.), p("retrace", "Retracement", 0.382, 0.2, 0.618, 0.01),
            p("setuplen", "Setup length bars", 24., 5., 50., 1.), usd("mm_usd", "Money-management stop ($)", 200.), usd("tr_usd", "$ Risk trailing ($)", 400.)], bpv(100., 1.)].into_iter().flatten().collect::<Vec<_>>(),
        "stad5_hlosc" => [vec![p("len1", "High/Low XAverage length", 10., 3., 30., 1.), p("len2", "Close XAverage length", 60., 20., 120., 5.), p("trailstp", "TrailStp ($ / BigPointValue)", 700., 0., 2000., 50.),
            usd("mm_usd", "Money-management stop ($)", 300.), usd("be_usd", "Breakeven floor ($)", 200.)], bpv(1., 100.)].into_iter().flatten().collect::<Vec<_>>(),
        "stad5_stochob" => vec![p("len", "Stochastic length", 12., 5., 30., 1.), p("ob", "Overbought", 70., 50., 90., 5.), p("os", "Oversold", 30., 10., 50., 5.), p("setuplen", "Setup length bars", 6., 1., 20., 1.)],
        "stad5_pivotsr" => [vec![p("hilolen", "Pivot lowest/highest bars", 8., 3., 20., 1.), p("atrlen", "ATR length", 10., 3., 30., 1.), p("setupint", "Setup initialisation bars", 4., 1., 10., 1.),
            p("xavglen", "XAverage length", 9., 3., 30., 1.), p("setuplen", "Setup length bars", 24., 5., 50., 1.), p("riskmult", "PMult (risk multiple)", 2., 1., 5., 0.5), p("hilostop", "Exit limit bars", 44., 10., 100., 1.),
            usd("be_usd", "Breakeven floor ($)", 200.)], bpv(1000., 1.)].into_iter().flatten().collect::<Vec<_>>(),
        "stad5_cur4day" => [vec![p("rsilen", "RSI length", 50., 5., 100., 1.), p("pcnt", "ATR fraction", 0.2, 0., 1., 0.05), usd("mm_usd", "Entry-day money-management stop ($)", 200.)], bpv(125000., 1.)].into_iter().flatten().collect::<Vec<_>>(),
        "stad5_chbopb" => [vec![p("channel", "Channel bars", 10., 3., 30., 1.), p("retrbars", "Retracement window bars", 10., 3., 30., 1.), p("newlohi", "New low/high bars", 4., 2., 10., 1.), p("trailstp", "Exit bars", 8., 3., 20., 1.),
            usd("mm_usd", "Money-management stop ($)", 1000.), usd("be_usd", "Breakeven floor ($)", 200.)], bpv(1000., 1.)].into_iter().flatten().collect::<Vec<_>>(),
        "stad5_dsdb" => vec![p("avglen", "Average length", 3., 1., 10., 1.), p("disp", "Displacement", 16., 0., 30., 1.), p("sdlen", "StdDev length", 12., 5., 30., 1.), p("sdev", "StdDevs", 2., 0.5, 4., 0.25)],
        "stad6_adxmac" => vec![p("avglen", "Channel XAverage length", 30., 5., 60., 1.), p("adxlen", "ADX length", 12., 5., 30., 1.), p("entrybar", "Entry window bars", 2., 1., 10., 1.)],
        "stad6_dmachan" => [vec![p("avglen", "Average length", 20., 5., 50., 1.), p("disp", "Displacement", 5., 0., 15., 1.), p("exitbar", "ExitAvg stop bars", 5., 1., 20., 1.),
            usd("mm_usd", "Money-management stop ($)", 200.), usd("be_usd", "Breakeven floor ($)", 100.), usd("dtr_usd", "$ Risk trailing ($)", 600.)], bpv(1., 100.)].into_iter().flatten().collect::<Vec<_>>(),
        "stad6_mapairs" => vec![p("lef", "Long entry fast", 5., 2., 30., 1.), p("les", "Long entry slow", 20., 5., 60., 1.), p("lxf", "Long exit fast", 3., 2., 30., 1.), p("lxs", "Long exit slow", 10., 3., 60., 1.),
            p("sef", "Short entry fast", 5., 2., 30., 1.), p("ses", "Short entry slow", 20., 5., 60., 1.), p("sxf", "Short exit fast", 3., 2., 30., 1.), p("sxs", "Short exit slow", 10., 3., 60., 1.),
            p("dm", "Option: 1 = the text's D-Mark pairs 7/23, 4/8, 6/18, 2/9", 0., 0., 1., 1.)],
        "stad6_cciavg" => [vec![p("ccilen", "CCI length", 30., 5., 60., 1.), p("avglen", "CCI average length", 5., 1., 20., 1.), p("hilolen", "Entry/exit channel bars", 9., 3., 30., 1.), p("entrybar", "Entry window bars", 4., 1., 10., 1.),
            usd("be_usd", "Breakeven floor ($)", 400.)], bpv(1., 100.)].into_iter().flatten().collect::<Vec<_>>(),
        "stad6_mrtr" => vec![p("chanlen", "Channel bars", 40., 10., 100., 5.), p("reaction", "Reaction bars", 4., 1., 10., 1.), p("tcount", "Thrust bars", 2., 1., 5., 1.), p("setup", "Entry window bars", 4., 1., 10., 1.),
            p("erratum", "1 = Vol 7 erratum SReacLow = High; 0 = as printed (Low)", 1., 0., 1., 1.)],
        "stad6_fps" => vec![p("fast", "MACD fast", 4., 2., 20., 1.), p("slow", "MACD slow", 10., 5., 40., 1.), p("avg", "Signal XAverage", 16., 3., 30., 1.), p("atrlen", "ATR length", 10., 3., 30., 1.),
            p("eatr", "Entry ATRs", 1.25, 0., 3., 0.25), p("xatr", "Exit ATRs", 2., 0.5, 5., 0.25)],
        "stad6_parvol" => vec![p("acc", "Parabolic acceleration", 0.03, 0.01, 0.1, 0.01), p("volavg", "Volume average length", 5., 2., 20., 1.), p("pcntdiff", "Entry fraction of SAR move", 0.3, 0., 1., 0.05), p("setuplen", "Setup window bars", 4., 1., 10., 1.)],
        "stad6_vwm" => vec![p("momlen", "Momentum length", 5., 2., 20., 1.), p("avglen", "XAverage length", 20., 5., 50., 1.), p("atrlen", "ATR length", 5., 2., 20., 1.), p("atrpcnt", "ATR fraction", 0.25, 0., 1., 0.05), p("setuplen", "Setup length bars", 5., 1., 15., 1.)],
        "stad7_dma" => vec![p("len", "Average length", 9., 3., 30., 1.)],
        "stad7_stsr" => vec![p("buylev", "Buy level", 0.3, 0.1, 0.5, 0.05), p("selllev", "Sell level", 0.7, 0.5, 0.9, 0.05), p("exitlen", "Trailing stop bars", 2., 1., 10., 1.)],
        "stad7_masr" => vec![p("len", "Average length", 10., 3., 40., 1.), p("patrs", "Protective ATRs", 1., 0.5, 5., 0.5), p("tatrs", "Trailing ATRs", 2.5, 0.5, 6., 0.5), p("atrlen", "ATR length", 10., 3., 30., 1.)],
        "stad7_trbo" => vec![p("rangelen", "Range bars", 7., 3., 20., 1.), p("rngpcnt", "No-trade % of range", 200., 50., 400., 10.), p("atrs", "Trailing ATRs", 8., 1., 15., 0.5), p("atrlen", "ATR length", 2., 1., 20., 1.)],
        "stad8_escalator" => vec![p("fast", "Fast average", 8., 3., 30., 1.), p("slow", "Slow average", 40., 10., 100., 5.), p("risk", "Risk bars", 2., 1., 10., 1.),
            p("target_mode", "Target: 0 = printed EntryPrice + 2*LongRisk (never fills), 1 = 2x the risk distance (prose)", 0., 0., 1., 1.)],
        "stad8_jib" => vec![p("qbars", "Qualifying bars", 5., 1., 15., 1.), p("atrpcnt", "Entry ATR fraction", 0.5, 0., 2., 0.05), p("acc_l", "Parabolic LX acceleration (code .2)", 0.2, 0.01, 0.2, 0.01),
            p("mult_l", "Parabolic LX first-bar ATRs", 1.5, 0.5, 5., 0.5), p("acc_s", "Parabolic SX acceleration", 0.02, 0.01, 0.2, 0.01), p("mult_s", "Parabolic SX first-bar ATRs", 3., 0.5, 5., 0.5)],
        "stad8_ryanshope" => [vec![p("len1", "Average length", 20., 5., 50., 1.), p("len2", "Retracement bars", 5., 2., 15., 1.)], atrx(3., 0., 4., 2., 7., 3., 0., 10.)].into_iter().flatten().collect::<Vec<_>>(),
        "stad8_stampede" => [vec![p("chan", "Channel bars", 20., 5., 60., 1.), p("maxent", "Maximum entries", 10., 1., 20., 1.)], atrx(3., 0., 4., 2., 7., 3., 0., 10.)].into_iter().flatten().collect::<Vec<_>>(),
        "stad9_dyf" => vec![p("avglen", "Average length", 5., 2., 20., 1.), p("disp", "Displacement", 5., 0., 15., 1.), p("entrysetup", "Entry window bars", 4., 1., 10., 1.), p("q1", "First cross window", 3., 1., 10., 1.),
            p("q2", "Second cross window", 5., 1., 15., 1.), p("trail", "Trailing stop bars (LowestFC/HighestFC)", 5., 1., 15., 1.)],
        "stad9_luxor" => vec![p("fast", "Fast TriAverage", 5., 3., 20., 1.), p("slow", "Slow TriAverage", 20., 10., 60., 1.), p("adxlen", "ADX length", 14., 5., 30., 1.), p("maxcc", "CurrentContracts limit", 3., 1., 10., 1.)],
        "stad9_nohurry" => [vec![p("chan", "Channel bars", 20., 5., 60., 1.), p("delay", "Channel delay bars", 15., 0., 40., 1.)], atrx(3., 2., 5., 0., 0., 0., 0., 10.)].into_iter().flatten().collect::<Vec<_>>(),
        "stad9_obvrev" => [vec![p("avglen", "WOBV average length", 25., 5., 60., 1.)], atrx(4., 4., 4., 0., 0., 0., 0., 20.)].into_iter().flatten().collect::<Vec<_>>(),
        "stad9_redrover" => [vec![p("side", "Option: 1 = long entries only, the Sell stop only closes longs (book's S&P test)", 0., 0., 1., 1.)], atrx(0., 0., 0., 0., 0., 0., 2., 10.)].into_iter().flatten().collect::<Vec<_>>(),
        "stad9_skinny" => [vec![p("lt", "Long-term XAverage", 50., 20., 100., 5.), p("st", "Short-term XAverage", 15., 5., 40., 1.)], atrx(2., 2., 0., 0., 0., 0., 4., 10.)].into_iter().flatten().collect::<Vec<_>>(),
        "stad9_swinger" => [vec![p("fast", "Oscillator fast", 5., 2., 20., 1.), p("slow", "Oscillator slow", 20., 5., 60., 1.), p("avg", "Filter average", 50., 10., 100., 5.), p("nbars", "Exit bars", 3., 1., 10., 1.)],
            atrx(4., 0., 0., 0., 0., 0., 0., 10.)].into_iter().flatten().collect::<Vec<_>>(),
        "stad9_trafficjam" => [vec![p("adxlen", "ADX length", 14., 5., 30., 1.), p("adxlevel", "ADX level", 25., 10., 40., 1.), p("dn", "Consecutive down closes", 3., 1., 6., 1.), p("up", "Consecutive up closes", 3., 1., 6., 1.),
            p("fixbar", "Fixed bar exit", 3., 1., 10., 1.)], atrx(3., 0., 0., 0., 0., 0., 0., 5.)].into_iter().flatten().collect::<Vec<_>>(),
        "stad10_serendipity" => [vec![p("nptsle", "Long entry points above the open", 20., 0., 100., 1.), p("nptsse", "Short entry points below the open", 20., 0., 100., 1.), p("rsilen", "RSI length", 9., 3., 30., 1.),
            p("ob", "Overbought", 70., 50., 90., 5.), p("os", "Oversold", 30., 10., 50., 5.)], atrx(3., 4., 0., 0., 0., 0., 4., 10.)].into_iter().flatten().collect::<Vec<_>>(),
        "stad10_eightball" => [vec![p("slow", "Slow XAverage", 40., 10., 100., 5.), p("fast", "Fast XAverage", 8., 3., 30., 1.), p("trigger", "Entry ATRs", 1., 0., 3., 0.25),
            p("prose", "Option: 1 = the prose setup (fast average rising for longs)", 0., 0., 1., 1.)], atrx(3., 0., 4., 2., 7., 3., 0., 10.)].into_iter().flatten().collect::<Vec<_>>(),
        "stad10_goinstyle" => vec![p("len", "ATR / channel length", 20., 5., 60., 1.), p("trigger", "Entry ATRs", 0.66, 0., 2., 0.02), p("acc_l", "Parabolic LX acceleration", 0.02, 0.01, 0.2, 0.01),
            p("mult_l", "Parabolic LX first-bar ATRs", 1.5, 0.5, 5., 0.5), p("acc_s", "Parabolic SX acceleration", 0.02, 0.01, 0.2, 0.01), p("mult_s", "Parabolic SX first-bar ATRs", 3., 0.5, 5., 0.5)],
        "stad10_wmts" => [vec![p("os", "Oversold", 40., 10., 50., 5.), p("ob", "Overbought", 60., 50., 90., 5.), p("newlow", "New low bars", 8., 2., 20., 1.), p("newhigh", "New high bars", 8., 2., 20., 1.),
            p("setup", "Setup length", 2., 1., 10., 1.), p("ebatr", "Entry-bar ATRs (5-bar ATR)", 1., 0., 5., 0.25)], atrx(0., 0., 4., 0., 7., 3., 0., 10.)].into_iter().flatten().collect::<Vec<_>>(),
        "stad10_gemini" => [vec![p("fast", "Weekly MACD fast", 3., 2., 20., 1.), p("slow", "Weekly MACD slow", 10., 5., 40., 1.), p("avg", "Weekly signal XAverage", 15., 3., 30., 1.), p("entrylen", "Entry bars", 3., 1., 10., 1.),
            p("wkstop", "Weekly stop bars", 4., 1., 10., 1.), p("trail", "Daily trailing stop bars", 3., 1., 10., 1.), usd("dtr_usd", "$ Risk trailing ($)", 1000.), usd("be_usd", "Breakeven floor ($)", 1000.)], bpv(1., 100.)].into_iter().flatten().collect::<Vec<_>>(),
        "stad10_refdev" => vec![p("etlong", "Long entry threshold", 5., 0., 50., 1.), p("etshort", "Short entry threshold", -5., -50., 0., 1.), p("rmalen", "Reference average length", 15., 5., 60., 1.)],
        "stad10_superman" => [vec![p("len", "MarketStrength / channel length", 5., 3., 20., 1.), p("stoplen", "Stop length", 6., 2., 20., 1.), p("pf", "Profit factor", 1., 0.5, 5., 0.25), usd("sl_usd", "Stop loss ($)", 100.)], bpv(1., 100.)].into_iter().flatten().collect::<Vec<_>>(),
        "stad11_highfive" => [vec![p("base", "Base length", 5., 2., 20., 1.), p("mult", "Range multiplier", 0.2, 0., 1., 0.05)], atrx(3., 4., 0., 0., 0., 0., 0., 10.)].into_iter().flatten().collect::<Vec<_>>(),
        "stad11_bullseye" => [vec![p("rangelen", "New high/low bars", 5., 2., 20., 1.), p("xavglen", "XAverage length", 50., 10., 100., 5.), p("bte", "Bars to enter", 5., 1., 15., 1.), p("factor", "Target factor", 3., 1., 6., 0.5)],
            atrx(3., 0., 0., 0., 0., 0., 0., 10.)].into_iter().flatten().collect::<Vec<_>>(),
        "stad11_nighttrain" => [atrx(3., 0., 0., 0., 0., 0., 0., 10.), vec![usd("sl_usd", "Option: $ risk stop (Joe's NASDAQ test $1,000; 0 = off)", 0.),
            p("side", "Option: 1 = long entries only, the Sell stop only closes longs (book's S&P test)", 0., 0., 1., 1.)], bpv(100., 1.)].into_iter().flatten().collect::<Vec<_>>(),
        "stad11_symtri" => vec![p("er", "Evaluation range", 40., 10., 100., 5.), p("dmilen", "DMI length", 14., 5., 30., 1.), p("strength", "Swing strength", 1., 1., 5., 1.), p("exitatrs", "ATR length for the stop", 10., 3., 30., 1.),
            p("exitfactor", "Stop ATR factor", 0.5, 0., 3., 0.25), p("mult", "Risk multiple for the breakeven move", 2., 0.5, 5., 0.5), p("spacing", "Bars between swing points", 1., 0., 5., 1.)],
        "stad11_trendscore" => [vec![p("lookback", "Look-back", 10., 1., 30., 1.), p("len", "Average length", 18., 5., 50., 1.)], atrx(3., 4., 4., 0., 0., 0., 0., 10.)].into_iter().flatten().collect::<Vec<_>>(),
        "stad11_gapper" => [vec![p("len", "ADX/DMI length", 14., 5., 30., 1.)], atrx(3., 4., 4., 0., 4., 3., 0., 10.)].into_iter().flatten().collect::<Vec<_>>(),
        "stad12_firstprize" => vec![p("xavglen", "XAverage length", 60., 10., 120., 5.), p("fast", "MACD fast", 3., 2., 20., 1.), p("slow", "MACD slow", 10., 5., 40., 1.), p("signal", "Signal XAverage", 16., 3., 30., 1.),
            p("ips", "Protective ATRs", 3., 0.5, 8., 0.5), p("bes", "Breakeven ATRs", 3., 0.5, 8., 0.5), p("ts", "Trailing ATRs", 6., 1., 12., 0.5), p("vs", "Volatility ATRs", 3., 0.5, 8., 0.5),
            p("bps", "Big profit ATRs", 9., 2., 15., 1.), p("nb", "Big profit exit bars", 5., 1., 15., 1.), p("atrpcnt", "Counter-trend stop ATRs", 0.6, 0., 2., 0.1), p("barsin", "Bars in before trend exit", 2., 0., 10., 1.),
            p("barsreq", "Bars required to cancel a setup", 2., 1., 10., 1.)],
        "stad12_inthezone" => vec![p("ips", "Protective ATRs", 3., 0.5, 8., 0.5), p("be", "Breakeven risk multiple", 3., 0.5, 8., 0.5), p("ptf", "Target ATRs", 5., 1., 12., 0.5)],
        "stad12_jailbreak" => vec![p("len1", "Entry channel bars", 50., 10., 120., 5.), p("len2", "Exit channel bars", 30., 5., 100., 5.), p("ips", "Protective ATRs", 4., 0.5, 10., 0.5)],
        "stad12_lrcross" => [vec![p("fast", "Fast regression length", 9., 3., 30., 1.), p("slow", "Slow regression length", 18., 5., 60., 1.), usd("sl_usd", "Stop loss $ (0 = off; book S&P 1250, coffee 750)", 0.),
            usd("pfloor_usd", "% trailing floor $ (0 = off; book 1250 / 750)", 0.), p("ppct", "% trailing give-back (0 = off; book 25)", 0., 0., 100., 5.), usd("pt_usd", "Profit target $ (0 = off; book 5000 / 1875)", 0.)], bpv(250., 1.)].into_iter().flatten().collect::<Vec<_>>(),
        "stad12_ntw" => vec![p("strength", "Swing strength", 2., 1., 6., 1.), p("fast", "MACD fast", 3., 2., 20., 1.), p("slow", "MACD slow", 10., 5., 40., 1.), p("signal", "Signal XAverage", 16., 3., 30., 1.),
            p("align", "Alignment bars", 2., 1., 6., 1.), p("len", "Maximum pivot distance", 20., 5., 60., 1.), p("pente", "Percent to enter", 50., 0., 100., 5.), p("psetup", "Percent setup bars", 50., 0., 100., 5.), p("pexit", "Percent to exit", 50., 0., 100., 5.)],
        "stad12_shifty" => [vec![p("fast", "Fast average (strategy input)", 30., 5., 100., 1.), p("slow", "Slow average (strategy input)", 60., 10., 200., 5.), usd("dtr_usd", "$ Risk trailing ($)", 1000.)], bpv(1., 100.)].into_iter().flatten().collect::<Vec<_>>(),
        "stad12_springs" => vec![p("strength", "Swing strength", 5., 1., 10., 1.), p("closepct", "Close within % of range", 25., 5., 50., 5.), p("rangepct", "Range % of ATR", 150., 50., 300., 10.),
            p("entrypct", "Entry % of range", 50., 0., 100., 5.), p("acc", "Parabolic acceleration", 0.02, 0.01, 0.2, 0.01), p("mult", "Parabolic first-bar ATRs", 1.5, 0.5, 5., 0.5)],
        "stad12_musketeers" => vec![p("len", "%D length", 12., 5., 30., 1.), p("tlx", "Long target risk multiple", 6., 1., 10., 0.5), p("tsx", "Short target risk multiple", 6., 1., 10., 0.5),
            p("belx", "Long breakeven ATRs", 3., 0.5, 8., 0.5), p("besx", "Short breakeven ATRs", 3., 0.5, 8., 0.5), p("up", "Up-trend level", 50., 30., 70., 5.), p("dn", "Down-trend level", 50., 30., 70., 5.)],
        "stad12_mist2" => vec![p("len", "Mist length", 3., 2., 10., 1.), p("acc", "Parabolic acceleration", 0.03, 0.01, 0.2, 0.01), p("mult", "Parabolic first-bar ATRs", 1., 0.5, 5., 0.5)],
        "stad13_ochist" => vec![p("len", "XAverage / ATR length (strategy input)", 30., 5., 60., 1.), p("ef", "Entry ATRs", 0.5, 0., 2., 0.1), p("xf", "Exit ATRs", 0.5, 0., 2., 0.1)],
        "stad13_dmarl" => [vec![p("len", "Average length", 20., 5., 50., 1.), p("disp", "Displacement", 5., 0., 15., 1.), usd("sl_usd", "Stop loss ($)", 500.), usd("be_usd", "Breakeven floor ($)", 500.),
            usd("dtr_usd", "$ Risk trailing ($)", 500.)], bpv(1., 100.)].into_iter().flatten().collect::<Vec<_>>(),
        "stad13_lrm" => [vec![p("reg", "Regression length", 60., 10., 120., 5.), p("xma", "XAverage of the regression", 30., 5., 60., 1.), p("mom", "Momentum length", 20., 5., 40., 1.),
            p("entrypct", "Entry ATRs", 0.5, 0., 2., 0.05), p("entrycounter", "Entry window bars", 3., 1., 10., 1.)], atrx(3., 3., 5., 3., 8., 4., 0., 10.)].into_iter().flatten().collect::<Vec<_>>(),
        "stad13_luxor" => vec![p("fast", "Fast TriAverage", 5., 3., 20., 1.), p("slow", "Slow TriAverage", 30., 10., 60., 1.), p("adxlen", "ADX length", 15., 5., 30., 1.), p("maxent", "Maximum entries per position", 3., 1., 10., 1.)],
        "stad13_mrts" => [vec![p("chan", "Channel XAverage", 15., 5., 40., 1.), p("closelen", "Close XAverage", 5., 2., 20., 1.), p("fast", "MACD fast", 3., 2., 20., 1.), p("slow", "MACD slow", 10., 5., 40., 1.),
            p("signal", "Signal XAverage", 15., 3., 30., 1.), p("rsilen", "RSI length", 5., 2., 30., 1.), p("ob", "RSI overbought", 70., 50., 90., 5.), p("os", "RSI oversold", 30., 10., 50., 5.),
            p("dmilen", "DMI length", 15., 5., 30., 1.), p("ttr", "Time to retrace", 10., 2., 30., 1.), p("rsichange", "RSI change", 30., 5., 50., 5.), p("bestentry", "Best entry bars", 2., 1., 10., 1.)],
            atrx(3., 4., 6., 3., 9., 4., 0., 10.)].into_iter().flatten().collect::<Vec<_>>(),
        "stad13_add" => [vec![p("longterm", "New high/low bars", 30., 10., 60., 1.), p("shortterm", "Recent bars", 5., 1., 15., 1.), p("xavglen", "XAverage length", 9., 3., 30., 1.), usd("sl_usd", "Stop loss, position basis ($)", 500.)],
            bpv(1., 100.)].into_iter().flatten().collect::<Vec<_>>(),
        "stad13_tripleplay" => vec![p("patlen", "Pattern length", 25., 10., 60., 1.), p("strength", "Swing strength", 7., 2., 12., 1.), p("bte", "Bars to enter", 10., 1., 30., 1.), p("len", "Bollinger length", 11., 5., 30., 1.),
            p("dev", "Bollinger deviations", 2., 1., 3., 0.25), p("epd", "Entry price divisor", 4., 1., 10., 1.), p("t1d", "Target 1 divisor", 2., 1., 4., 1.), p("tsd", "Trailing stop divisor", 4., 1., 10., 1.),
            p("weak_src", "WeakLow source: 0 = printed (Value1 never set -> current bar), 1 = RecentLow (explanation)", 0., 0., 1., 1.)],
        "stad13_volex" => [vec![p("lf", "Long factor", 1.2, 0.25, 3., 0.05), p("sf", "Short factor", 1.8, 0.25, 3., 0.05), usd("sl_usd", "Stop loss ($)", 1000.), p("stale_t", "Stale position bars", 0., 0., 20., 1.),
            usd("stale_usd", "Stale minimum profit ($)", 0.), p("trail", "Trailing exit bars", 4., 1., 10., 1.), usd("go_usd", "First good open base profit ($)", 200.), p("delay", "Good open delay bars", 0., 0., 10., 1.)], bpv(250., 1.)].into_iter().flatten().collect::<Vec<_>>(),
        "stad13_volclus" => [vec![p("dev", "Deviations", 2., 0.5, 4., 0.25), p("len", "Length (strategy input)", 15., 5., 40., 1.), usd("sl_usd", "Stop loss ($)", 300.), usd("go_usd", "First good open base profit ($)", 0.),
            p("delay", "Good open delay bars", 0., 0., 10., 1.)], bpv(1., 100.)].into_iter().flatten().collect::<Vec<_>>(),
        _ => return None,
    };
    v.push(p("point", "One point (minimum price move)", 0.01, 0.0001, 1., 0.0001));
    Some(v)
}

// ------------------------------------------------------------------ strategies

fn avg_at(v: &[f64], n: usize, i: usize) -> f64 { let n = n.max(1); if i + 1 < n { f64::NAN } else { v[i + 1 - n..=i].iter().sum::<f64>() / n as f64 } }
fn bv(n: usize, f: impl Fn(usize) -> bool) -> Vec<bool> { (0..n).map(f).collect() }
/// value of the straight line through (a, pa) and (b, pb) at bar k (TLValue / TL_GetValue)
fn tl_at(a: f64, pa: f64, b: f64, pb: f64, k: f64) -> f64 { if a == b { pb } else { pa + (pb - pa) * (k - a) / (b - a) } }
/// MarketStrength(len): net close change of the last len bars over the up (net >= 0) or down (net < 0) changes, x100
fn market_strength(c: &[f64], len: usize, i: usize) -> f64 {
    if i < len { return f64::NAN; }
    let ch: Vec<f64> = (0..len).map(|k| c[i - k] - c[i - k - 1]).collect();
    let net: f64 = ch.iter().sum();
    let den = if net >= 0.0 { ch.iter().filter(|v| **v > 0.0).sum::<f64>() } else { ch.iter().filter(|v| **v < 0.0).map(|v| -v).sum::<f64>() };
    if den == 0.0 { 0.0 } else { 100.0 * net / den }
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if from > to || to >= x.c.len() { return None; }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(0.0, |d| d.def));
    let u = |k: &str| g(k).max(1.0).round() as usize;
    let usd = |k: &str| g(k) / (g("bpv") * g("qty"));
    let (o_, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let n = c.len();
    let pt = g("point");
    let tr = true_range(x);
    let rg = rng(x);
    let nxo = |i: usize| if i + 1 < n { o_[i + 1] } else { c[i] };
    let ax = AtrX { prot: g("prot"), be: g("be"), trail: g("trail"), volat: g("volat"), bp: g("bp"), bpn: u("bpn"), pt: g("atr_pt") };
    let atrv = atr(x, u("atrlen"));
    let mut ast = AtrState::default();
    Some(match id {
        // ============================================================ Volume 5
        "stad5_fib382" => {
            let (xa, hl, rt, sul) = (xavg(c, u("xavglen")), u("hilolen"), g("retrace"), g("setuplen"));
            let mut cf = cfg(); cf.ss.sl = usd("mm_usd"); cf.ss.dtr = usd("tr_usd");
            let (mut bs, mut ss) = (0.0, 0.0);
            run_ts(x, from, to, hl.max(u("xavglen")), cf, |i, _st, _now, nx| {
                let (hh, ll, hb, lb) = (highest(h, hl, i), lowest(l, hl, i), highest_bar(h, hl, i), lowest_bar(l, hl, i));
                let r = (hh - ll) * rt;
                if c[i] <= hh - r && hb < lb { bs = 0.0; }
                if c[i] >= ll + r && hb > lb { ss = 0.0; }
                bs += 1.0; ss += 1.0;
                if bs <= sul && (swing_low(&xa, 1, i) || c[i] > xa[i]) { nx.push(o(A::Buy, P::Mkt, "Fib Buy")); bs = sul; }
                if ss <= sul && (swing_high(&xa, 1, i) || c[i] < xa[i]) { nx.push(o(A::Sell, P::Mkt, "Fib Sell")); ss = sul; }
            })
        }
        "stad5_hlosc" => {
            let (xh, xl, ca) = (xavg(h, u("len1")), xavg(l, u("len1")), xavg(c, u("len2")));
            let ho: Vec<f64> = (0..n).map(|i| h[i] - xh[i]).collect();
            let lo: Vec<f64> = (0..n).map(|i| l[i] - xl[i]).collect();
            let trail = g("trailstp") / g("bpv");
            let mut cf = cfg(); cf.ss.sl = usd("mm_usd"); cf.ss.be = usd("be_usd");
            run_ts(x, from, to, u("len2"), cf, |i, st, _now, nx| {
                let bs = ca[i] > ca[i - 1] && c[i] > ca[i] && lo[i] < 0.0;
                let ss = ca[i] < ca[i - 1] && c[i] < ca[i] && ho[i] > 0.0;
                if bs && lo[i] > lo[i - 1] { nx.push(o(A::Buy, P::Mkt, "HL Buy")); }
                if st.pos == 1 { nx.push(o(A::Xl, P::Stop(c[i] - trail), "LTrailStop")); if xunder_k(&ho, 0.0, i) { nx.push(o(A::Xl, P::Mkt, "LTiming")); } }
                if ss && ho[i] < ho[i - 1] { nx.push(o(A::Sell, P::Mkt, "HL Sell")); }
                if st.pos == -1 { nx.push(o(A::Xs, P::Stop(c[i] + trail), "STrailStop")); if xover_k(&lo, 0.0, i) { nx.push(o(A::Xs, P::Mkt, "STiming")); } }
            })
        }
        "stad5_stochob" => {
            let (k, d) = slowkd(x, u("len"));
            let (ob, os, sul) = (g("ob"), g("os"), g("setuplen"));
            let (mut bsu, mut ssu, mut bp, mut sp, mut bstop, mut sstop) = (0.0, 0.0, 99.0, 99.0, 0.0, 0.0);
            run_ts(x, from, to, u("len") + 1, cfg(), |i, st, _now, nx| {
                if k[i] < os && xover(&k, &d, i) { bsu = highest(h, 2, i); bstop = l[i]; bp = 0.0; }
                if k[i] > ob && xunder(&k, &d, i) { ssu = lowest(l, 2, i); sstop = h[i]; sp = 0.0; }
                if st.pos == 1 { bp = sul + 1.0; } else { bp += 1.0; }
                if st.pos == -1 { sp = sul + 1.0; } else { sp += 1.0; }
                if bp <= sul { nx.push(o(A::Buy, P::Stop(bsu + pt), "Stoch Buy")); }
                if sp <= sul { nx.push(o(A::Sell, P::Stop(ssu - pt), "Stoch Sell")); }
                if xunder(&k, &d, i) && k[i] > os { bp = sul; nx.push(o(A::Xl, P::Mkt, "K below D")); }
                if xover(&k, &d, i) && k[i] < ob { sp = sul; nx.push(o(A::Xs, P::Mkt, "K above D")); }
                nx.push(o(A::Xl, P::Stop(bstop), "BuyStop")); nx.push(o(A::Xs, P::Stop(sstop), "SellStop"));
            })
        }
        "stad5_pivotsr" => {
            let (hll, si, sul, pm, hls) = (u("hilolen"), u("setupint"), g("setuplen"), g("riskmult"), u("hilostop"));
            let atrp = atr(x, u("atrlen"));
            let ema = xavg(c, u("xavglen"));
            let c13 = bv(n, |k| k >= 2 && swing_low(l, 1, k) && l[k - 1] == lowest(l, hll, k - 1));
            let c24 = bv(n, |k| k >= 2 && swing_high(h, 1, k) && h[k - 1] == highest(h, hll, k - 1));
            let mut cf = cfg(); cf.ss.be = usd("be_usd");
            let (mut bsh, mut bsl, mut bsc, mut ssl, mut ssh, mut ssc) = (0.0, 0.0, 100.0, 0.0, 0.0, 100.0);
            let (mut lxf, mut sxf, mut lxl, mut sxh, mut ttp) = (0, 0, 0.0, 0.0, 0usize);
            let mbb = hls.max(hll + 2).max(u("atrlen")).max(u("xavglen")).max(si + 2);
            run_ts(x, from, to, mbb, cf, |i, st, _now, nx| {
                let lp = mro(&c13, i, si, 1); let hp = mro(&c24, i, si, 1);
                let c5 = tr[i] > atrp[i];
                let tt = st.ntr; let e = st.entry_px;
                if lp > 0 { let lp = lp as usize; if i > lp && lowest(l, lp, i) < l[i - lp - 1] && c5 && c[i] > c[i - 1].max(o_[i]).max(l[i] + 2.0 * (tr[i] / 3.0)).max(l[i - lp - 1]) { bsh = h[i]; bsl = l[i]; bsc = 0.0; } }
                if hp > 0 { let hp = hp as usize; if i > hp && highest(h, hp, i) > h[i - hp - 1] && c5 && c[i] < c[i - 1].min(o_[i]).min(l[i] + tr[i] / 3.0).min(h[i - hp - 1]) { ssl = l[i]; ssh = h[i]; ssc = 0.0; } }
                if st.pos == 1 || tt != ttp { bsc = sul; if c[i] > e + pm * (e - lxl) || lxf == 0 { nx.push(o(A::Xl, P::Stop(e), "Breakeven above floor")); lxf = 0; } }
                if st.pos == -1 || tt != ttp { ssc = sul; if c[i] < e - pm * (sxh - e) || sxf == 0 { nx.push(o(A::Xs, P::Stop(e), "Breakeven above floor")); sxf = 0; } }
                bsc += 1.0; ssc += 1.0;
                if bsc <= sul && bsh + pt > ema[i] { nx.push(o(A::Buy, P::Stop(bsh + pt), "Pivot Buy")); lxl = bsl; lxf = 1; }
                if ssc <= sul && ssl + pt < ema[i] { nx.push(o(A::Sell, P::Stop(ssl - pt), "Pivot Sell")); sxh = ssh; sxf = 1; }
                if lxf == 1 { nx.push(o(A::Xl, P::Stop(lxl - pt), "Initial stop")); }
                if sxf == 1 { nx.push(o(A::Xs, P::Stop(sxh + pt), "Initial stop")); }
                nx.push(o(A::Xl, P::Lim(highest(h, hls, i)), "HiLo limit")); nx.push(o(A::Xs, P::Lim(lowest(l, hls, i)), "HiLo limit"));
                ttp = tt;
            })
        }
        "stad5_cur4day" => {
            let r = crate::connors::rsi(c, u("rsilen"));
            let a10 = sma(&tr, 10);
            let k = g("pcnt");
            let mut cf = cfg(); cf.ss.sl = usd("mm_usd"); cf.ss.sl_entry_bar_only = true;
            run_ts(x, from, to, u("rsilen").max(10), cf, |i, st, _now, nx| {
                if weekday(&x.r[i].date) == 4 {
                    let on = nxo(i);
                    if r[i] > 50.0 { nx.push(o(A::Buy, P::Stop(on + k * a10[i]), "Friday Buy")); }
                    if on > h[i] { nx.push(o(A::Xl, P::Stop(h[i]), "Gap exit")); }
                    nx.push(o(A::Xl, P::Stop(l[i]), "Low stop"));
                    if r[i] < 50.0 { nx.push(o(A::Sell, P::Stop(on - k * a10[i]), "Friday Sell")); }
                    if on < l[i] { nx.push(o(A::Xs, P::Stop(l[i]), "Gap exit")); }
                    nx.push(o(A::Xs, P::Stop(h[i]), "High stop"));
                }
                if st.pos != 0 { nx.push(o(A::Xl, P::Mkt, "Next open")); nx.push(o(A::Xs, P::Mkt, "Next open")); }
            })
        }
        "stad5_chbopb" => {
            let (ch, rb, nl, ts) = (u("channel"), u("retrbars"), u("newlohi"), u("trailstp"));
            let cu = bv(n, |k| k >= ch && c[k] > highest(c, ch, k - 1));
            let cd = bv(n, |k| k >= ch && c[k] < lowest(c, ch, k - 1));
            let mut cf = cfg(); cf.ss.sl = usd("mm_usd"); cf.ss.be = usd("be_usd");
            run_ts(x, from, to, (ch + 1).max(nl + 1).max(ts + 1), cf, |i, _st, now, nx| {
                if mro(&cu, i, rb, 1) > -1 && l[i] < lowest(l, nl, i - 1) { nx.push(o(A::Buy, P::Mkt, "Pullback Buy")); }
                if mro(&cd, i, rb, 1) > -1 && h[i] > highest(h, nl, i - 1) { nx.push(o(A::Sell, P::Mkt, "Pullback Sell")); }
                if h[i] > highest(h, ts, i - 1) { now.push(o(A::Xl, P::Mkt, "New high exit")); }
                if l[i] < lowest(l, ts, i - 1) { now.push(o(A::Xs, P::Mkt, "New low exit")); }
            })
        }
        "stad5_dsdb" => {
            let (al, dp, sdl) = (u("avglen"), g("disp").max(0.0).round() as usize, u("sdlen"));
            let av = sma(c, al); let sd = stdev(c, sdl); let k = g("sdev");
            run_ts(x, from, to, sdl.max(al + dp), cfg(), |i, st, _now, nx| {
                if st.bn > dp as i64 {
                    nx.push(o(A::Buy, P::Stop(av[i - dp] + sd[i] * k), "DispTop")); nx.push(o(A::Sell, P::Stop(av[i - dp] - sd[i] * k), "DispBottom"));
                }
            })
        }
        // ============================================================ Volume 6
        "stad6_adxmac" => {
            let (al, eb) = (u("avglen"), u("entrybar"));
            let (xh, xl) = (xavg(h, al), xavg(l, al));
            let adx = dmi3(x, u("adxlen")).2;
            let um = shift(&xh, 1); let lm = shift(&xl, 1);
            let bs = bv(n, |k| k >= 2 && c[k] > um[k] && adx[k] > adx[k - 1]);
            let ss = bv(n, |k| k >= 2 && c[k] < lm[k] && adx[k] > adx[k - 1]);
            let mut mph = vec![0i8; n];
            let (mut bt, mut stt) = (0.0, 0.0);
            run_ts(x, from, to, (al + 1).max(2 * u("adxlen") + 2), cfg(), |i, st, _now, nx| {
                mph[i] = st.pos;
                let spread = (um[i] - lm[i]) / 2.0;
                if bs[i] { bt = c[i] + spread; }
                if ss[i] { stt = c[i] - spread; }
                let (mb, ms) = (mro(&bs, i, eb, 1), mro(&ss, i, eb, 1));
                let mp_mro = |v: i8, len: i64| (0..len.max(0) as usize).any(|b| b <= i && mph[i - b] == v);
                if mb != -1 && !mp_mro(1, mb) { nx.push(o(A::Buy, P::Stop(bt), "ADX Buy")); }
                if ms != -1 && !mp_mro(-1, ms) { nx.push(o(A::Sell, P::Stop(stt), "ADX Sell")); }
                nx.push(o(A::Xl, P::Stop(um[i] - pt), "UpperMA stop")); nx.push(o(A::Xs, P::Stop(lm[i] + pt), "LowerMA stop"));
            })
        }
        "stad6_dmachan" => {
            let (al, dp, eb) = (u("avglen"), g("disp").abs().round() as usize, g("exitbar") as usize);
            let med: Vec<f64> = (0..n).map(|i| (h[i] + l[i]) / 2.0).collect();
            let (ua, la, ea) = (shift(&sma(h, al), dp), shift(&sma(l, al), dp), shift(&sma(&med, al), dp));
            let mut cf = cfg(); cf.ss.sl = usd("mm_usd"); cf.ss.be = usd("be_usd"); cf.ss.dtr = usd("dtr_usd");
            run_ts(x, from, to, al + dp + 1, cf, |i, st, now, nx| {
                let rlb = med[i] > h[i - 1] && rg[i] > rg[i - 1];
                let rls = med[i] < l[i - 1] && rg[i] > rg[i - 1];
                if rlb && c[i] > ua[i] { now.push(o(A::Buy, P::Mkt, "Range Leader Buy")); }
                if rls && c[i] < la[i] { now.push(o(A::Sell, P::Mkt, "Range Leader Sell")); }
                if st.bse(i) < eb { nx.push(o(A::Xl, P::Stop(ea[i]), "ExitAvg")); nx.push(o(A::Xs, P::Stop(ea[i]), "ExitAvg")); }
                else { nx.push(o(A::Xl, P::Stop(ua[i] - pt), "UpperAvg")); nx.push(o(A::Xs, P::Stop(la[i] + pt), "LowerAvg")); }
            })
        }
        "stad6_mapairs" => {
            let dm = g("dm") >= 0.5;
            let pick = |k: &str, d: usize| if dm { d } else { u(k) };
            let (lef, les, lxf, lxs) = (pick("lef", 7), pick("les", 23), pick("lxf", 4), pick("lxs", 8));
            let (sef, ses, sxf, sxs) = (pick("sef", 6), pick("ses", 18), pick("sxf", 2), pick("sxs", 9));
            let (a1, a2, a3, a4, a5, a6, a7, a8) = (sma(c, lef), sma(c, les), sma(c, lxf), sma(c, lxs), sma(c, sef), sma(c, ses), sma(c, sxf), sma(c, sxs));
            let mbb = [lef, les, lxf, lxs, sef, ses, sxf, sxs].into_iter().max().unwrap_or(1) + 1;
            run_ts(x, from, to, mbb, cfg(), |i, _st, _now, nx| {
                if a1[i] > a2[i] { nx.push(o(A::Buy, P::Stop(h[i] + pt), "MA Buy")); }
                if xunder(&a3, &a4, i) { nx.push(o(A::Xl, P::Mkt, "MA LX")); }
                if a5[i] < a6[i] { nx.push(o(A::Sell, P::Stop(l[i] - pt), "MA Sell")); }
                if xover(&a7, &a8, i) { nx.push(o(A::Xs, P::Mkt, "MA SX")); }
            })
        }
        "stad6_cciavg" => {
            let (cl, al, hl, eb) = (u("ccilen"), u("avglen"), u("hilolen"), u("entrybar"));
            let ac = sma(&cci(x, cl), al);
            let bset = bv(n, |k| xover_k(&ac, -100.0, k));
            let sset = bv(n, |k| xunder_k(&ac, 100.0, k));
            let mut cf = cfg(); cf.ss.be = usd("be_usd");
            let mut ts = 0usize;
            run_ts(x, from, to, cl + al + hl, cf, |i, st, _now, nx| {
                if ac[i] > -100.0 && mro(&bset, i, eb, 1) != -1 { nx.push(o(A::Buy, P::Stop(highest(h, hl, i - 1) + pt), "CCI Buy")); }
                if ac[i] < 100.0 && mro(&sset, i, eb, 1) != -1 { nx.push(o(A::Sell, P::Stop(lowest(l, hl, i - 1) - pt), "CCI Sell")); }
                if st.pos == 1 && h[i] > highest(h, hl, i - 1) && ts > 1 { ts -= 1; }
                if st.pos == -1 && l[i] < lowest(l, hl, i - 1) && ts > 1 { ts -= 1; }
                if st.bse(i) == 0 { ts = hl; }
                nx.push(o(A::Xl, P::Stop(lowest(l, ts, i) - pt), "Trail LX")); nx.push(o(A::Xs, P::Stop(highest(h, ts, i) + pt), "Trail SX"));
            })
        }
        "stad6_mrtr" => {
            let (cl, re, tc, su) = (u("chanlen"), g("reaction"), g("tcount"), u("setup"));
            let errat = g("erratum") >= 0.5;
            let (mut lr, mut lrh, mut lrl, mut sr, mut srh, mut srl, mut lt, mut sth, mut le, mut lx, mut se, mut sx) = (0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
            let (mut nlt, mut nst) = (false, false);
            let (mut ltrh, mut strh) = (vec![false; n], vec![false; n]);
            run_ts(x, from, to, cl + 1, cfg(), |i, st, _now, nx| {
                let (hic, loc) = (highest(h, cl, i - 1), lowest(l, cl, i - 1));
                if st.pos != 1 { if h[i] > hic { lr = 1.0; lrh = h[i]; lrl = l[i]; lx = lrl; lt = 0.0; sr = 0.0; } } else { lr = 0.0; lt = 0.0; }
                if st.pos != -1 { if l[i] < loc { sr = 1.0; srh = h[i]; srl = if errat { h[i] } else { l[i] }; sx = srl; sth = 0.0; lr = 0.0; } } else { sr = 0.0; lt = 0.0; }
                if lr >= 1.0 { if l[i] < lx { lx = l[i]; lt = 0.0; } if h[i] < lrh && l[i] < lrl { lr += 1.0; lrh = h[i]; lrl = l[i]; nlt = true; } }
                if sr >= 1.0 { if h[i] > sx { sx = h[i]; sth = 0.0; } if l[i] > srl && h[i] > srh { sr += 1.0; srl = l[i]; srh = h[i]; nst = true; } }
                if lr >= re && c[i] > h[i - 1] { lt += 1.0; }
                if sr >= re && c[i] < l[i - 1] { sth += 1.0; }
                ltrh[i] = lt == tc && nlt; strh[i] = sth == tc && nst;
                if ltrh[i] { le = h[i] + pt; nlt = false; }
                if strh[i] { se = l[i] - pt; nst = false; }
                if mro(&ltrh, i, su, 1) != -1 && lr >= re { nx.push(o(A::Buy, P::Stop(le), "Trend resumption Buy")); }
                if mro(&strh, i, su, 1) != -1 && sr >= re { nx.push(o(A::Sell, P::Stop(se), "Trend resumption Sell")); }
                if lr == 0.0 { nx.push(o(A::Xl, P::Stop(lx), "LExit")); }
                if sr == 0.0 { nx.push(o(A::Xs, P::Stop(sx), "SExit")); }
            })
        }
        "stad6_fps" => {
            let m = macd(c, u("fast"), u("slow")); let sg = xavg(&m, u("avg"));
            let a = atr(x, u("atrlen")); let (ea, xa) = (g("eatr"), g("xatr"));
            let (mut up, mut dn, mut bs, mut ss, mut sf, mut ctl, mut cth) = (false, false, false, false, false, 0.0, 0.0);
            run_ts(x, from, to, u("slow") + u("avg"), cfg(), |i, st, now, nx| {
                if xover_k(&sg, 0.0, i) { up = true; dn = false; ss = false; sf = false; }
                if xunder_k(&sg, 0.0, i) { dn = true; up = false; bs = false; sf = false; }
                if up { if xunder(&m, &sg, i) && !sf { bs = true; ctl = l[i]; } if m[i] < sg[i] && l[i] < ctl { ctl = l[i]; } }
                if dn { if xover(&m, &sg, i) && !sf { ss = true; cth = h[i]; } if m[i] > sg[i] && h[i] > cth { cth = h[i]; } }
                if st.pos == 1 { bs = false; sf = true; }
                if st.pos == -1 { ss = false; sf = true; }
                if bs { nx.push(o(A::Buy, P::Stop(c[i] + ea * a[i]), "Pullback Buy")); }
                if ss { nx.push(o(A::Sell, P::Stop(c[i] - ea * a[i]), "Pullback Sell")); }
                nx.push(o(A::Xl, P::Stop(ctl - pt), "CTrendLow")); nx.push(o(A::Xl, P::Stop(c[i] - xa * a[i]), "ATR LX"));
                nx.push(o(A::Xs, P::Stop(cth + pt), "CTrendHigh")); nx.push(o(A::Xs, P::Stop(c[i] + xa * a[i]), "ATR SX"));
                if dn { now.push(o(A::Xl, P::Mkt, "DnTrend")); }
                if up { now.push(o(A::Xs, P::Mkt, "UpTrend")); }
            })
        }
        "stad6_parvol" => {
            let par = parabolic(x, g("acc")); let pv = shift(&par, 1);
            let v = vol(x); let va = sma(&v, u("volavg")); let (pd, sl) = (g("pcntdiff"), u("setuplen"));
            let bsu = bv(n, |k| xover(h, &pv, k) && v[k] > va[k]);
            let ssu = bv(n, |k| xunder(l, &pv, k) && v[k] > va[k]);
            let (mut bsf, mut bsp, mut bsp_p, mut ssf, mut ssp, mut ssp_p) = (false, 0.0, 0.0, false, 0.0, 0.0);
            run_ts(x, from, to, u("volavg") + 2, cfg(), |i, st, _now, nx| {
                if bsu[i] { bsf = true; bsp = c[i]; bsp_p = pv[i - 1]; }
                if ssu[i] { ssf = true; ssp = c[i]; ssp_p = pv[i - 1]; }
                if mro(&bsu, i, sl, 1) != -1 && bsf { nx.push(o(A::Buy, P::Stop(bsp + pd * (pv[i] - bsp_p)), "ParVol Buy")); }
                if mro(&ssu, i, sl, 1) != -1 && ssf { nx.push(o(A::Sell, P::Stop(ssp - pd * (ssp_p - pv[i])), "ParVol Sell")); }
                if st.pos == 1 { nx.push(o(A::Xl, P::Stop(pv[i]), "Parabolic")); bsf = false; }
                if st.pos == -1 { nx.push(o(A::Xs, P::Stop(pv[i]), "Parabolic")); ssf = false; }
            })
        }
        "stad6_vwm" => {
            let v = vol(x); let mo = mom(c, u("momlen"));
            let vm: Vec<f64> = (0..n).map(|i| v[i] * mo[i]).collect();
            let w = xavg(&vm, u("avglen")); let a = atr(x, u("atrlen")); let (k, sl) = (g("atrpcnt"), g("setuplen"));
            let (mut ls, mut ss, mut lep, mut sep) = (0.0, 0.0, 0.0, 0.0);
            run_ts(x, from, to, u("momlen") + u("avglen"), cfg(), |i, st, _now, nx| {
                if xover_k(&w, 0.0, i) { ls = 0.0; lep = c[i]; }
                if xunder_k(&w, 0.0, i) { ss = 0.0; sep = c[i]; }
                ls += 1.0; ss += 1.0;
                if ls <= sl { nx.push(o(A::Buy, P::Stop(lep + k * a[i]), "VWM Buy")); nx.push(o(A::Xl, P::Stop(l[i] - k * a[i]), "VWM LX stop")); }
                if ss <= sl { nx.push(o(A::Sell, P::Stop(sep - k * a[i]), "VWM Sell")); nx.push(o(A::Xs, P::Stop(h[i] + k * a[i]), "VWM SX stop")); }
                if st.pos == 1 { ls = sl; if xunder_k(&w, 0.0, i) { nx.push(o(A::Xl, P::Mkt, "VWM below 0")); } }
                if st.pos == -1 { ss = sl; if xover_k(&w, 0.0, i) { nx.push(o(A::Xs, P::Mkt, "VWM above 0")); } }
            })
        }
        // ============================================================ Volume 7
        "stad7_dma" => {
            let ln = u("len"); let av = sma(c, ln);
            let d1: Vec<f64> = (0..n).map(|i| if i >= ln { (av[i] - av[i - ln]) / ln as f64 } else { f64::NAN }).collect();
            let d2: Vec<f64> = (0..n).map(|i| if i >= 1 { d1[i] - d1[i - 1] } else { f64::NAN }).collect();
            let (mut bf, mut sf) = (false, false);
            run_ts(x, from, to, 2 * ln + 1, cfg(), |i, _st, _now, nx| {
                if xover_k(&d2, 0.0, i) { bf = true; }
                if xunder_k(&d2, 0.0, i) { sf = true; }
                if xover_k(&d1, 0.0, i) && bf { nx.push(o(A::Buy, P::Stop(c[i] + pt), "DMA Buy")); bf = false; }
                if xunder_k(&d1, 0.0, i) && sf { nx.push(o(A::Sell, P::Stop(c[i] - pt), "DMA Sell")); sf = false; }
            })
        }
        "stad7_stsr" => {
            let (bl, sl, el) = (g("buylev"), g("selllev"), u("exitlen"));
            let (mut sr, mut sr_p, mut lo, mut lo_p, mut hi, mut hi_p) = (0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
            run_ts(x, from, to, 2, cfg(), |i, st, now, nx| {
                if tr[i] != 0.0 { let (th, tl) = (h[i].max(c[i - 1]), l[i].min(c[i - 1])); sr = ((th - o_[i]) + (c[i] - tl)) / (2.0 * tr[i]); }
                if st.pos != 1 { if sr < bl && sr_p >= bl { lo = sr; } if sr < bl && sr < lo { lo = sr; } }
                if st.pos != -1 { if sr > sl && sr_p <= sl { hi = sr; } if sr > sl && sr > hi { hi = sr; } }
                if sr > bl && sr_p <= bl { nx.push(o(A::Buy, P::Mkt, "SROsc Buy")); }
                if sr < sl && sr_p >= sl { nx.push(o(A::Sell, P::Mkt, "SROsc Sell")); }
                if sr < lo && sr_p >= lo_p { now.push(o(A::Xl, P::Mkt, "Initial stop")); }
                if sr > hi && sr_p <= hi_p { now.push(o(A::Xs, P::Mkt, "Initial stop")); }
                if sr > sl { nx.push(o(A::Xl, P::Stop(lowest(l, el, i)), "Trailing")); }
                if sr < bl { nx.push(o(A::Xs, P::Stop(highest(h, el, i)), "Trailing")); }
                sr_p = sr; lo_p = lo; hi_p = hi;
            })
        }
        "stad7_masr" => {
            let ln = u("len"); let av = sma(c, ln); let a = atr(x, u("atrlen")); let (pa, ta) = (g("patrs"), g("tatrs"));
            let (mut supp, mut res, mut ph, mut pl, mut rf, mut sf, mut lord, mut sord, mut lord_p, mut sord_p) = (0.0, 0.0, 0.0, 0.0, false, false, 0.0, 0.0, 0.0, 0.0);
            run_ts(x, from, to, ln.max(u("atrlen")) + 1, cfg(), |i, st, _now, nx| {
                let (c1, c2, c3, c4) = (res != 0.0, supp != 0.0, xunder(c, &av, i), xover(c, &av, i));
                if c1 && c3 { if rf { lord = res; rf = false; } sf = true; supp = l[i]; }
                if c2 && c4 { if sf { sord = supp; sf = false; } rf = true; res = h[i]; }
                if c[i] > av[i] && h[i] >= res { res = h[i]; }
                if c[i] < av[i] && l[i] <= supp { supp = l[i]; }
                if c[i] > lord && c[i - 1] <= lord_p { nx.push(o(A::Buy, P::Stop(h[i] + pt), "MA S/R Buy")); }
                if c[i] < sord && c[i - 1] >= sord_p { nx.push(o(A::Sell, P::Stop(l[i] - pt), "MA S/R Sell")); }
                let e = st.entry_px;
                if st.pos == 1 { if st.bse(i) == 0 { ph = h[i]; } if h[i] > ph { ph = h[i]; }
                    nx.push(o(A::Xl, P::Stop(e - a[i] * pa), "Protective")); nx.push(o(A::Xl, P::Stop(ph - a[i] * ta), "Trailing")); }
                if st.pos == -1 { if st.bse(i) == 0 { pl = l[i]; } if l[i] < pl { pl = l[i]; }
                    nx.push(o(A::Xs, P::Stop(e + a[i] * pa), "Protective")); nx.push(o(A::Xs, P::Stop(pl + a[i] * ta), "Trailing")); }
                lord_p = lord; sord_p = sord;
            })
        }
        "stad7_trbo" => {
            let (rl, rp, at) = (u("rangelen"), g("rngpcnt"), g("atrs"));
            let a = atr(x, u("atrlen")); let ar = atr(x, rl);
            let (mut lrk, mut lhi, mut srk, mut slo) = (0.0, 0.0, 0.0, 0.0);
            run_ts(x, from, to, 2 * rl + 1, cfg(), |i, st, now, nx| {
                let (rh, rlw) = (highest(h, rl, i - 1), lowest(l, rl, i - 1));
                let trange = rh - rlw;
                let mut nt = 0.0;
                for k in 1..=rl { if h[i - k] < rh { nt += rh - h[i - k]; } if l[i - k] > rlw { nt += l[i - k] - rlw; } }
                let c1 = nt >= trange * (rp / 100.0); let c2 = tr[i] > ar[i - 1];
                let med = (h[i] + l[i]) / 2.0;
                if c1 && c2 {
                    if c[i] > rh && med > h[i - 1] && st.pos != 1 { now.push(o(A::Buy, P::Mkt, "Range Breakout Buy")); lrk = rlw; lhi = h[i]; }
                    if c[i] < rlw && med < l[i - 1] && st.pos != -1 { now.push(o(A::Sell, P::Mkt, "Range Breakout Sell")); srk = rh; slo = l[i]; }
                }
                if st.pos == 1 { if h[i] > lhi { lhi = h[i]; } nx.push(o(A::Xl, P::Stop(lrk - pt), "LongRisk")); nx.push(o(A::Xl, P::Stop(lhi - at * a[i]), "ATR trail")); }
                if st.pos == -1 { if l[i] < slo { slo = l[i]; } nx.push(o(A::Xs, P::Stop(srk + pt), "ShortRisk")); nx.push(o(A::Xs, P::Stop(slo + at * a[i]), "ATR trail")); }
            })
        }
        // ============================================================ Volume 8
        "stad8_escalator" => {
            let (fa, sa, rl) = (sma(c, u("fast")), sma(c, u("slow")), u("risk"));
            let tmode = g("target_mode") >= 0.5;
            let (mut lr, mut sr) = (0.0, 0.0);
            run_ts(x, from, to, u("slow").max(u("fast")).max(rl) + 1, cfg(), |i, st, _now, nx| {
                let e = st.entry_px;
                let lc1 = c[i - 1] <= l[i - 1] + 0.25 * rg[i - 1]; let lc2 = c[i] >= h[i] - 0.25 * rg[i]; let lc3 = c[i] > fa[i] && c[i] > sa[i];
                if lc1 && lc2 && lc3 && st.pos != 1 { nx.push(o(A::Buy, P::Stop(highest(h, 2, i) + pt), "2-Bar Reversal Long")); lr = lowest(l, rl, i) - pt; }
                nx.push(o(A::Xl, P::Stop(lr), "LX1"));
                if st.pos == 1 { nx.push(o(A::Xl, P::Lim(if tmode { e + 2.0 * (e - lr) } else { e + 2.0 * lr }), "LX2")); }
                let sc1 = c[i - 1] >= h[i - 1] - 0.25 * rg[i - 1]; let sc2 = c[i] <= l[i] + 0.25 * rg[i]; let sc3 = c[i] < fa[i] && c[i] < sa[i];
                if sc1 && sc2 && sc3 && st.pos != -1 { nx.push(o(A::Sell, P::Stop(lowest(l, 2, i) - pt), "2-Bar Reversal Short")); sr = highest(h, rl, i) + pt; }
                nx.push(o(A::Xs, P::Stop(sr), "SX1"));
                if st.pos == -1 { nx.push(o(A::Xs, P::Lim(if tmode { e - 2.0 * (sr - e) } else { e - 2.0 * sr }), "SX2")); }
            })
        }
        "stad8_jib" => {
            let a = atr(x, 10); let a3 = sma(&tr, 3); let (q, k) = (g("qbars"), g("atrpcnt"));
            let mut px = ParaX::new(g("acc_l"), g("mult_l"), g("acc_s"), g("mult_s"));
            let (mut ct, mut th, mut tl, mut bf, mut sf, mut cnt, mut init) = (0, 0.0, 0.0, false, false, 0.0, false);
            run_ts(x, from, to, 10, cfg(), |i, st, _now, nx| {
                if !init { th = h[i]; tl = l[i]; init = true; }
                if l[i] > th { ct = 1; th = h[i]; tl = l[i]; bf = true; cnt = 0.0; }
                if h[i] < tl { ct = -1; th = h[i]; tl = l[i]; sf = true; cnt = 0.0; }
                if ct == 1 && h[i] > th { th = h[i]; tl = l[i]; }
                if ct == -1 && l[i] < tl { th = h[i]; tl = l[i]; }
                if st.pos == 1 || cnt >= q { bf = false; }
                if st.pos == -1 || cnt >= q { sf = false; }
                cnt += 1.0;
                if bf { nx.push(o(A::Buy, P::Stop(nxo(i) + k * a[i]), "B1")); }
                if sf { nx.push(o(A::Sell, P::Stop(nxo(i) - k * a[i]), "S1")); }
                px.step(x, i, st.pos, a3[i], nx, true, true);
            })
        }
        "stad8_ryanshope" => {
            let (l1, l2) = (u("len1"), u("len2")); let av = sma(c, l1);
            run_ts(x, from, to, l1 + l2 + 1, cfg(), |i, st, _now, nx| {
                if av[i] > av[i - l2] && c[i] < c[i - l2] && c[i] > c[i - l1 - l2] { nx.push(o(A::Buy, P::Mkt, "Ryan's Hope Buy")); }
                if av[i] < av[i - l2] && c[i] > c[i - l2] && c[i] < c[i - l1 - l2] { nx.push(o(A::Sell, P::Mkt, "Ryan's Hope Sell")); }
                atr_exits(x, i, st, &ax, atrv[i], &mut ast, nx);
            })
        }
        "stad8_stampede" => {
            let (ch, me) = (u("chan"), g("maxent"));
            let mut cf = cfg(); cf.max_ent = usize::MAX;
            let (mut counter, mut ccp, mut mpp) = (1.0, 0.0, 0i8);
            run_ts(x, from, to, ch.max(u("atrlen")), cf, |i, st, _now, nx| {
                let (cc, mp) = (st.units, st.pos);
                if cc > ccp && mp == mpp && counter < me { counter += 1.0; }
                if mp != mpp { counter = 1.0; }
                if counter < me { nx.push(o(A::Buy, P::Stop(highest(h, ch, i) + pt), "Chan Break Pyramid")); nx.push(o(A::Sell, P::Stop(lowest(l, ch, i) - pt), "Chan Break Pyramid")); }
                ccp = cc; mpp = mp;
                atr_exits(x, i, st, &ax, atrv[i], &mut ast, nx);
            })
        }
        // ============================================================ Volume 9
        "stad9_dyf" => {
            let (al, dp, es, q1, q2, trl) = (u("avglen"), g("disp").round() as usize, g("entrysetup"), g("q1") as i64, u("q2"), u("trail"));
            let dma = shift(&sma(c, al), dp);
            let xa = bv(n, |k| xover(c, &dma, k)); let xb = bv(n, |k| xunder(c, &dma, k));
            let (mut bs, mut ss, mut bp, mut sp, mut bc, mut sc) = (false, false, 0.0, 0.0, 0.0, 0.0);
            run_ts(x, from, to, 50 + al + dp, cfg(), |i, st, _now, nx| {
                if xa[i] && mro(&xb, i, q2, 1) != -1 && mro(&xa, i, 50, 2) - mro(&xb, i, q2, 1) <= q1 { bs = true; bp = h[i]; bc = 0.0; }
                if xb[i] && mro(&xa, i, q2, 1) != -1 && mro(&xb, i, 50, 2) - mro(&xa, i, q2, 1) <= q1 { ss = true; sp = l[i]; sc = 0.0; }
                if bs && bc <= es { if st.pos == 1 { bs = false; } else { bc += 1.0; nx.push(o(A::Buy, P::Stop(bp + pt), "DYF Buy")); } } else { bs = false; }
                if ss && sc <= es { if st.pos == -1 { ss = false; } else { sc += 1.0; nx.push(o(A::Sell, P::Stop(sp + pt), "DYF Sell")); } } else { ss = false; }
                if st.pos == 1 { nx.push(o(A::Xl, P::Stop(dma[i] - pt), "DMA Stop")); }
                if st.pos == -1 { nx.push(o(A::Xs, P::Stop(dma[i] + pt), "DMA Stop")); }
                nx.push(o(A::Xl, P::Stop(lowest(l, trl, i)), "Trl")); nx.push(o(A::Xs, P::Stop(highest(h, trl, i)), "Trl"));
            })
        }
        "stad9_luxor" => {
            let (fl, sl) = (u("fast"), u("slow")); let (fa, sa) = (triavg(c, fl), triavg(c, sl));
            let adx = dmi3(x, u("adxlen")).2; let mc = g("maxcc");
            let mut cf = cfg(); cf.max_ent = usize::MAX;
            run_ts(x, from, to, sl.max(2 * u("adxlen") + fl + 1), cf, |i, st, _now, nx| {
                let (ls, ssu, adxs) = (fa[i] > sa[i], fa[i] < sa[i], adx[i] > adx[i - fl]);
                let bs = ls && adxs && h[i] < fa[i]; let ss = ssu && adxs && l[i] > fa[i];
                if (ls && st.pos != 1) || (bs && st.units < mc) { if st.pos != 1 { nx.push(o(A::Buy, P::Stop(h[i] + pt), "LUXOR Buy")); } else { nx.push(o(A::Buy, P::Stop(fa[i] + pt), "LUXOR Add")); } }
                if (ssu && st.pos != -1) || (ss && st.units < mc) { if st.pos != -1 { nx.push(o(A::Sell, P::Stop(l[i] - pt), "LUXOR Sell")); } else { nx.push(o(A::Sell, P::Stop(fa[i] - pt), "LUXOR Add")); } }
                if st.pos == 1 { nx.push(o(A::Xl, P::Stop(sa[i] - pt), "Slow stop")); }
                if st.pos == -1 { nx.push(o(A::Xs, P::Stop(sa[i] + pt), "Slow stop")); }
            })
        }
        "stad9_nohurry" => {
            let (ch, dl) = (u("chan"), g("delay").round() as usize);
            let mut pf = 0;
            run_ts(x, from, to, (ch + dl).max(u("atrlen")), cfg(), |i, st, _now, nx| {
                let (uc, lc) = (highest(h, ch, i - dl), lowest(l, ch, i - dl));
                if st.pos == 1 { pf = 1; }
                if st.pos == -1 { pf = -1; }
                if pf != 1 && st.pos != 1 { nx.push(o(A::Buy, P::Stop(uc + pt), "Delayed Chan Buy")); }
                if pf != -1 && st.pos != -1 { nx.push(o(A::Sell, P::Stop(lc - pt), "Delayed Chan Sell")); }
                atr_exits(x, i, st, &ax, atrv[i], &mut ast, nx);
            })
        }
        "stad9_obvrev" => {
            let v = vol(x); let al = u("avglen");
            let (mut wobv, mut wh, mut smh) = (0.0, vec![0.0; n], vec![0.0; n]);
            let (mut bs, mut ss, mut lep, mut sep) = (false, false, 0.0, 0.0);
            run_ts(x, from, to, al.max(u("atrlen")), cfg(), |i, st, now, nx| {
                if rg[i] != 0.0 { wobv += (c[i] - o_[i]) / rg[i] * v[i]; }
                wh[i] = wobv; smh[i] = avg_at(&wh, al, i);
                if wh[i] > smh[i] && wh[i - 1] <= smh[i - 1] { bs = true; lep = h[i]; ss = false; now.push(o(A::Xs, P::Mkt, "WOBV cross")); }
                if wh[i] < smh[i] && wh[i - 1] >= smh[i - 1] { ss = true; sep = l[i]; bs = false; now.push(o(A::Xl, P::Mkt, "WOBV cross")); }
                if st.pos == 1 { bs = false; }
                if st.pos == -1 { ss = false; }
                if bs { nx.push(o(A::Buy, P::Stop(lep + pt), "OBV Buy")); }
                if ss { nx.push(o(A::Sell, P::Stop(sep - pt), "OBV Sell")); }
                atr_exits(x, i, st, &ax, atrv[i], &mut ast, nx);
            })
        }
        "stad9_redrover" => {
            let long_only = g("side") >= 0.5;
            run_ts(x, from, to, u("atrlen"), cfg(), |i, st, _now, nx| {
                let wap = (h[i] + l[i] + 2.0 * c[i]) / 4.0;
                if st.pos != 1 { nx.push(o(A::Buy, P::Stop(2.0 * wap - l[i] + pt), "Red Rover Buy")); }
                // long only (TradeStation "long entries only"): the Sell order still closes the long
                if st.pos != -1 { nx.push(if long_only { o(A::Xl, P::Stop(2.0 * wap - h[i] - pt), "Red Rover Sell (exit)") } else { o(A::Sell, P::Stop(2.0 * wap - h[i] - pt), "Red Rover Sell") }); }
                atr_exits(x, i, st, &ax, atrv[i], &mut ast, nx);
            })
        }
        "stad9_skinny" => {
            let (xl, xs) = (xavg(c, u("lt")), xavg(c, u("st")));
            run_ts(x, from, to, u("lt").max(u("atrlen")), cfg(), |i, st, _now, nx| {
                if c[i] > xl[i] && c[i] > xs[i] && c[i] < l[i - 1] { nx.push(o(A::Buy, P::Stop(h[i - 1] + pt), "Skinny Dipper")); }
                atr_exits(x, i, st, &ax, atrv[i], &mut ast, nx);
            })
        }
        "stad9_swinger" => {
            let (fa, sa, af, nb) = (sma(c, u("fast")), sma(c, u("slow")), sma(c, u("avg")), u("nbars"));
            let po: Vec<f64> = (0..n).map(|i| fa[i] - sa[i]).collect();
            run_ts(x, from, to, u("avg").max(u("slow")) + 1, cfg(), |i, st, _now, nx| {
                if po[i] > po[i - 1] && po[i] < 0.0 && c[i] > af[i] { nx.push(o(A::Buy, P::Mkt, "Swing Buy")); }
                if po[i] < po[i - 1] && po[i] > 0.0 && c[i] < af[i] { nx.push(o(A::Sell, P::Mkt, "Swing Sell")); }
                if st.pos == 1 && po[i] < po[i - 1] { nx.push(o(A::Xl, P::Stop(lowest(l, nb, i)), "Swing LX")); }
                if st.pos == -1 && po[i] > po[i - 1] { nx.push(o(A::Xs, P::Stop(highest(h, nb, i)), "Swing SX")); }
                atr_exits(x, i, st, &ax, atrv[i], &mut ast, nx);
            })
        }
        "stad9_trafficjam" => {
            let adx = dmi3(x, u("adxlen")).2; let (lev, dn, upn, fb) = (g("adxlevel"), u("dn"), u("up"), u("fixbar"));
            let m = dn.max(upn);
            run_ts(x, from, to, (2 * u("adxlen") + m + 1).max(u("atrlen")), cfg(), |i, st, now, nx| {
                let c1 = adx[i] < lev && adx[i] < adx[i - m];
                let c2 = (0..dn).all(|k| c[i - k] < c[i - k - 1]);
                let c3 = (0..upn).all(|k| c[i - k] > c[i - k - 1]);
                if c1 { if c2 { now.push(o(A::Buy, P::Mkt, "Traffic Jam Buy")); } if c3 { now.push(o(A::Sell, P::Mkt, "Traffic Jam Sell")); } }
                if st.pos != 0 && st.bse(i) == fb { now.push(o(A::Xl, P::Mkt, "FxBr")); now.push(o(A::Xs, P::Mkt, "FxBr")); }
                atr_exits(x, i, st, &ax, atrv[i], &mut ast, nx);
            })
        }
        // ============================================================ Volume 10
        "stad10_serendipity" => {
            let r = crate::connors::rsi(c, u("rsilen")); let (ob, os, nle, nse) = (g("ob"), g("os"), g("nptsle"), g("nptsse"));
            run_ts(x, from, to, u("rsilen").max(u("atrlen")) + 1, cfg(), |i, st, _now, nx| {
                if r[i] > ob { nx.push(o(A::Buy, P::Stop(o_[i] + nle * pt), "LongEntry")); }
                if r[i] < os { nx.push(o(A::Sell, P::Stop(o_[i] - nse * pt), "ShortEntry")); }
                atr_exits(x, i, st, &ax, atrv[i], &mut ast, nx);
            })
        }
        "stad10_eightball" => {
            let a = atr(x, 10); let (sa, fa, trg) = (xavg(c, u("slow")), xavg(c, u("fast")), g("trigger"));
            let prose = g("prose") >= 0.5;
            run_ts(x, from, to, u("slow").max(u("atrlen")).max(10), cfg(), |i, st, _now, nx| {
                let (fup, fdn) = if prose { (fa[i] > fa[i - 1], fa[i] < fa[i - 1]) } else { (fa[i] < fa[i - 1], fa[i] > fa[i - 1]) };
                if sa[i] > sa[i - 1] && fup { nx.push(o(A::Buy, P::Stop(nxo(i) + trg * a[i]), "Long Entry")); }
                if sa[i] < sa[i - 1] && fdn { nx.push(o(A::Sell, P::Stop(nxo(i) - trg * a[i]), "Short Entry")); }
                atr_exits(x, i, st, &ax, atrv[i], &mut ast, nx);
            })
        }
        "stad10_goinstyle" => {
            let ln = u("len"); let a = atr(x, ln); let a3 = sma(&tr, 3); let trg = g("trigger");
            let mut px = ParaX::new(g("acc_l"), g("mult_l"), g("acc_s"), g("mult_s"));
            run_ts(x, from, to, ln + 1, cfg(), |i, st, _now, nx| {
                if h[i] > highest(h, ln, i - 1) { nx.push(o(A::Buy, P::Stop(c[i] + a[i] * trg), "Go Long")); }
                if l[i] < lowest(l, ln, i - 1) { nx.push(o(A::Sell, P::Stop(c[i] - a[i] * trg), "Go Short")); }
                px.step(x, i, st.pos, a3[i], nx, true, true);
            })
        }
        "stad10_wmts" => {
            let (k, d) = slowkd(x, 14); let a5 = atr(x, 5);
            let (os, ob, nl, nh, su, eb) = (g("os"), g("ob"), u("newlow"), u("newhigh"), g("setup") as i64, g("ebatr"));
            let (mut bf, mut sf) = (su, su);
            run_ts(x, from, to, 14usize.max(nl + 1).max(nh + 1).max(u("atrlen")) + su.max(0) as usize + 4, cfg(), |i, st, _now, nx| {
                if st.pos == 1 { bf = su; } else { bf += 1; }
                if st.pos == -1 { sf = su; } else { sf += 1; }
                let c1 = k[i] < d[i] && d[i] < os; let c2 = k[i] > d[i] && d[i] > ob;
                let kru = l[i] < lowest(l, nl, i - 1) && c[i] > c[i - 1];
                if bf < su { nx.push(o(A::Buy, P::Stop(highest(h, 4, i - bf as usize) + pt), "WMTS Buy")); nx.push(o(A::Xl, P::Stop(lowest(l, nl, i) - pt), "WMTS LX")); }
                else if c1 && kru { nx.push(o(A::Buy, P::Stop(highest(h, 4, i) + pt), "WMTS Buy")); nx.push(o(A::Xl, P::Stop(lowest(l, nl, i) - pt), "WMTS LX")); bf = 0; }
                if sf < su { nx.push(o(A::Sell, P::Stop(lowest(l, 4, i - sf as usize) - pt), "WMTS Sell")); nx.push(o(A::Xs, P::Stop(highest(h, nh, i) + pt), "WMTS SX")); }
                else if c2 && kru { nx.push(o(A::Sell, P::Stop(lowest(l, 4, i) - pt), "WMTS Sell")); nx.push(o(A::Xs, P::Stop(highest(h, nh, i) + pt), "WMTS SX")); sf = 0; }
                if st.pos != 1 { nx.push(o(A::Xl, P::Stop(c[i] - eb * a5[i]), "EBTR")); }
                if st.pos != -1 { nx.push(o(A::Xs, P::Stop(c[i] + eb * a5[i]), "EBTR")); }
                atr_exits(x, i, st, &ax, atrv[i], &mut ast, nx);
            })
        }
        "stad10_gemini" => {
            let wk = weekly(x); let wm = macd(&wk.c, u("fast"), u("slow")); let ws = xavg(&wm, u("avg"));
            let (el, wl, trl) = (u("entrylen"), u("wkstop"), u("trail"));
            let mut cf = cfg(); cf.ss.dtr = usd("dtr_usd"); cf.ss.be = usd("be_usd");
            let (mut up, mut dn, mut uf, mut df) = (false, false, true, true);
            run_ts(x, from, to, 60, cf, |i, st, _now, nx| {
                let w = wk.map[i];
                if w < 0 { return; }
                let w = w as usize;
                let (ml, sl) = (wm[w], ws[w]);
                if sl > 0.0 { dn = false; if ml > sl && uf { up = true; uf = false; df = true; } }
                if sl < 0.0 { up = false; if ml < sl && df { dn = true; df = false; uf = true; } }
                if up && ml < sl { nx.push(o(A::Buy, P::Stop(highest(h, el, i) + pt), "Gemini Buy")); }
                if dn && ml > sl { nx.push(o(A::Sell, P::Stop(lowest(l, el, i) - pt), "Gemini Sell")); }
                if st.pos == 1 { up = false; }
                if st.pos == -1 { dn = false; }
                nx.push(o(A::Xl, P::Stop(lowest(&wk.l, wl, w) - pt), "Weekly stop")); nx.push(o(A::Xs, P::Stop(highest(&wk.h, wl, w) + pt), "Weekly stop"));
                nx.push(o(A::Xl, P::Stop(lowest(l, trl, i)), "Trl")); nx.push(o(A::Xs, P::Stop(highest(h, trl, i)), "Trl"));
            })
        }
        "stad10_refdev" => {
            let ln = u("rmalen"); let rma = sma(c, ln); let (etl, ets) = (g("etlong"), g("etshort"));
            let drd: Vec<f64> = (0..n).map(|i| c[i] - rma[i]).collect();
            let mut rdv = 0.0;
            run_ts(x, from, to, 2 * ln, cfg(), |i, _st, _now, nx| {
                let ndv: f64 = drd[i + 1 - ln..=i].iter().sum(); let tdv: f64 = drd[i + 1 - ln..=i].iter().map(|v| v.abs()).sum();
                if tdv > 0.0 { rdv = 100.0 * ndv / tdv; }
                if rdv > etl { nx.push(o(A::Buy, P::Mkt, "RDV Buy")); }
                if rdv < ets { nx.push(o(A::Sell, P::Mkt, "RDV Sell")); }
                if rdv < 0.0 { nx.push(o(A::Xl, P::Mkt, "RDV < 0")); }
                if rdv > 0.0 { nx.push(o(A::Xs, P::Mkt, "RDV > 0")); }
            })
        }
        "stad10_superman" => {
            let (ln, sl, pf) = (u("len"), u("stoplen"), g("pf"));
            let mut cf = cfg(); cf.ss.sl = usd("sl_usd");
            let mut slv = 0.0;
            run_ts(x, from, to, 9usize.max(ln + 1).max(sl), cf, |i, st, now, nx| {
                let ms = market_strength(c, ln, i);
                let c1 = (c[i] - c[i - 4]) / 4.0 >= 0.0; let c2 = (c[i - 4] - c[i - 8]) / 5.0 >= 0.0;
                if ms >= 95.0 && c1 && !c2 && h[i] == highest(h, ln, i) && st.pos != 1 {
                    nx.push(o(A::Buy, P::Stop(highest(h, ln, i)), "Superman Buy")); now.push(o(A::Xs, P::Mkt, "LngRev")); slv = lowest(l, sl, i); nx.push(o(A::Xl, P::Stop(slv), "InitLng"));
                }
                if ms <= -95.0 && !c1 && c2 && l[i] == lowest(l, ln, i) && st.pos != -1 {
                    nx.push(o(A::Sell, P::Stop(lowest(l, ln, i)), "Superman Sell")); now.push(o(A::Xl, P::Mkt, "ShrtRev")); slv = highest(h, sl, i); nx.push(o(A::Xs, P::Stop(slv), "InitShrt"));
                }
                let e = st.entry_px;
                if st.pos == 1 { nx.push(o(A::Xl, P::Stop(slv), "LngStp")); nx.push(o(A::Xl, P::Lim(e + (e - slv) * pf), "LngPrft")); }
                if st.pos == -1 { nx.push(o(A::Xs, P::Stop(slv), "ShrtStp")); nx.push(o(A::Xs, P::Lim(e - (slv - e) * pf), "ShrtPrft")); }
            })
        }
        // ============================================================ Volume 11
        "stad11_highfive" => {
            let b = u("base"); let mult = g("mult");
            let avs: Vec<Vec<f64>> = (0..5).map(|k| sma(c, b << k)).collect();
            let (mut bs, mut ss, mut lev, mut sev, mut mpp) = (false, false, 0.0, 0.0, 0i8);
            run_ts(x, from, to, (b << 4) + 1, cfg(), |i, st, _now, nx| {
                if st.pos != mpp { bs = false; ss = false; }
                let lc = avs.iter().filter(|a| a[i] > a[i - 1]).count(); let sc = avs.iter().filter(|a| a[i] < a[i - 1]).count();
                if lc == 5 { bs = true; lev = c[i] + rg[i] * mult; }
                if sc == 5 { ss = true; sev = c[i] - rg[i] * mult; }
                if st.pos != 1 && bs { nx.push(o(A::Buy, P::Stop(lev), "High Five Buy")); }
                if st.pos != -1 && ss { nx.push(o(A::Sell, P::Stop(sev), "High Five Sell")); }
                if lc <= 2 { nx.push(o(A::Xl, P::Mkt, "LongCounter <= 2")); }
                if sc <= 2 { nx.push(o(A::Xs, P::Mkt, "ShortCounter <= 2")); }
                mpp = st.pos;
                atr_exits(x, i, st, &ax, atrv[i], &mut ast, nx);
            })
        }
        "stad11_bullseye" => {
            let (rl, bte, f) = (u("rangelen"), g("bte"), g("factor")); let xa = xavg(c, u("xavglen"));
            let (mut be, mut bs, mut bc, mut lt, mut se, mut ss, mut sc, mut stg) = (0.0, false, 0.0, 0.0, 0.0, false, 0.0, 0.0);
            run_ts(x, from, to, u("xavglen").max(rl).max(u("atrlen")) + 2, cfg(), |i, st, _now, nx| {
                if st.pos != 1 && h[i] == highest(h, rl, i) && c[i] > h[i - 2] && xa[i] > xa[i - 1] { be = (h[i - 2] + l[i - 2]) / 2.0; bc = 0.0; bs = true; lt = h[i] + f * (h[i] - be); }
                if bc > bte { bs = false; } else { bc += 1.0; }
                if st.pos == 1 { bs = false; nx.push(o(A::Xl, P::Lim(lt), "Bull's Eye target")); }
                if bs { nx.push(o(A::Buy, P::Lim(be), "Bull's Eye Buy")); }
                if st.pos != -1 && l[i] == lowest(l, rl, i) && c[i] < l[i - 2] && xa[i] < xa[i - 1] { se = (h[i - 2] + l[i - 2]) / 2.0; sc = 0.0; ss = true; stg = l[i] - f * (se - l[i]); }
                if sc > bte { ss = false; } else { sc += 1.0; }
                if st.pos == -1 { ss = false; nx.push(o(A::Xs, P::Lim(stg), "Bull's Eye target")); }
                if ss { nx.push(o(A::Sell, P::Lim(se), "Bull's Eye Sell")); }
                atr_exits(x, i, st, &ax, atrv[i], &mut ast, nx);
            })
        }
        "stad11_nighttrain" => {
            let mut cf = cfg(); cf.ss.sl = usd("sl_usd");
            let long_only = g("side") >= 0.5;
            run_ts(x, from, to, u("atrlen").max(2), cf, |i, st, _now, nx| {
                let e = st.entry_px;
                if c[i] < c[i - 2] { nx.push(o(A::Buy, P::Stop(h[i]), "Night Train Buy")); }
                if st.pos == 1 && nxo(i) > e { nx.push(o(A::Xl, P::Mkt, "First profitable open")); }
                // long only (TradeStation "long entries only"): the Sell order still closes the long
                if c[i] >= c[i - 2] { nx.push(if long_only { o(A::Xl, P::Stop(l[i]), "Night Train Sell (exit)") } else { o(A::Sell, P::Stop(l[i]), "Night Train Sell") }); }
                if st.pos == -1 && nxo(i) < e { nx.push(o(A::Xs, P::Mkt, "First profitable open")); }
                atr_exits(x, i, st, &ax, atrv[i], &mut ast, nx);
            })
        }
        "stad11_symtri" => {
            let (er, s, mult, sp) = (g("er") as i64, u("strength"), g("mult"), g("spacing") as i64);
            let (pdi, mdi, _) = dmi3(x, u("dmilen"));
            let dms: Vec<f64> = (0..n).map(|i| pdi[i] - mdi[i]).collect();
            let ax10 = atr(x, u("exitatrs")); let ef = g("exitfactor");
            // one state per signal (DMILongTriangle / DMIShortTriangle keep separate variables)
            #[derive(Clone, Copy)]
            struct Tri { ph: [f64; 3], pl: [f64; 3], phb: [i64; 3], plb: [i64; 3], form: bool, drawn: bool, setup: bool,
                l1: (f64, f64, f64, f64), l2: (f64, f64, f64, f64), sup: f64, res: f64, prot: f64, prof: f64, mpp: i8 }
            let t0 = Tri { ph: [0.0; 3], pl: [0.0; 3], phb: [0; 3], plb: [0; 3], form: false, drawn: false, setup: false,
                l1: (0.0, 0.0, 0.0, 0.0), l2: (0.0, 0.0, 0.0, 0.0), sup: 0.0, res: 0.0, prot: 0.0, prof: 0.0, mpp: 0 };
            let (mut tl_, mut ts_) = (t0, t0);
            let tlv = |p1: f64, b1: i64, p2: f64, b2: i64| if b1 == b2 { p2 } else { p2 + (p2 - p1) * b2 as f64 / (b1 - b2) as f64 };
            run_ts(x, from, to, (2 * u("dmilen") + 2).max(u("exitatrs")).max(2 * s + 1), cfg(), |i, st, _now, nx| {
                let hs = swing_high(h, s, i); let ls = swing_low(l, s, i);
                let (hsp, hsb) = if hs { (h[i - s], s as i64) } else { (-1.0, -1) };
                let (lsp, lsb) = if ls { (l[i - s], s as i64) } else { (-1.0, -1) };
                let e = st.entry_px; let fi = i as f64;
                for (long, t) in [(true, &mut tl_), (false, &mut ts_)] {
                    for k in 1..=2 { t.phb[k] += 1; t.plb[k] += 1; }
                    if long {
                        if t.phb[2] > er { t.ph[2] = t.ph[1]; t.ph[1] = 0.0; t.phb[2] = t.phb[1]; } else if xunder_k(&dms, -15.0, i) { t.ph[2] = 0.0; }
                        if hsp != -1.0 { if hsp > t.ph[2] { t.ph[2] = hsp; t.ph[1] = 0.0; t.phb[2] = hsb; } else if hsp > t.ph[1] { t.ph[1] = hsp; t.phb[1] = hsb; } else if t.plb[2] < t.phb[1] { t.ph[1] = hsp; t.phb[1] = hsb; } }
                        if lsp != -1.0 { if lsp < t.pl[2] || t.phb[2] < t.plb[2] { t.pl[2] = lsp; t.pl[1] = 99999.0; t.plb[2] = lsb; } else if lsp < t.pl[1] { t.pl[1] = lsp; t.plb[1] = lsb; } else if t.phb[1] < t.plb[1] { t.pl[1] = lsp; t.plb[1] = lsb; } }
                    } else {
                        if t.plb[2] > er { t.pl[2] = t.pl[1]; t.pl[1] = 99999.0; t.plb[2] = t.plb[1]; } else if xover_k(&dms, 15.0, i) { t.pl[2] = 99999.0; }
                        if hsp != -1.0 { if hsp > t.ph[2] || t.plb[2] < t.phb[2] { t.ph[2] = hsp; t.ph[1] = 0.0; t.phb[2] = hsb; } else if hsp > t.ph[1] { t.ph[1] = hsp; t.phb[1] = hsb; } else if t.plb[1] < t.phb[1] { t.ph[1] = hsp; t.phb[1] = hsb; } }
                        if lsp != -1.0 { if lsp < t.pl[2] { t.pl[2] = lsp; t.pl[1] = 99999.0; t.plb[2] = lsb; } else if lsp < t.pl[1] { t.pl[1] = lsp; t.plb[1] = lsb; } else if t.phb[2] < t.plb[1] { t.pl[1] = lsp; t.plb[1] = lsb; } }
                    }
                    let c1 = t.ph[2] > t.ph[1] && t.pl[2] < t.pl[1];
                    let (c2, c3, c4, c5) = if long {
                        (t.phb[2] > t.plb[2] + sp && t.plb[2] > t.phb[1] + sp && t.phb[1] > t.plb[1] + sp, t.phb[2] - t.plb[1] <= er,
                         t.phb[2] <= er + s as i64 && t.phb[2] >= 0 && t.phb[2] as usize <= i && dms[i - t.phb[2] as usize] >= 15.0, lsb != -1)
                    } else {
                        (t.plb[2] > t.phb[2] + sp && t.phb[2] > t.plb[1] + sp && t.plb[1] > t.phb[1] + sp, t.plb[2] - t.phb[1] <= er,
                         t.plb[2] <= er + s as i64 && t.plb[2] >= 0 && t.plb[2] as usize <= i && dms[i - t.plb[2] as usize] <= -15.0, hsb != -1)
                    };
                    let c6 = tlv(t.ph[2], t.phb[2], t.ph[1], t.phb[1]) > tlv(t.pl[2], t.plb[2], t.pl[1], t.plb[1]);
                    if c1 && c2 && c3 && c4 && c5 && c6 {
                        t.form = true;
                        t.l1 = (fi - t.phb[2] as f64, t.ph[2], fi - t.phb[1] as f64, t.ph[1]);
                        t.l2 = (fi - t.plb[2] as f64, t.pl[2], fi - t.plb[1] as f64, t.pl[1]);
                        t.drawn = true; t.setup = true;
                    }
                    if t.drawn {
                        let (sup_p, res_p) = (t.sup, t.res);
                        t.res = tl_at(t.l1.0, t.l1.1, t.l1.2, t.l1.3, fi); t.sup = tl_at(t.l2.0, t.l2.1, t.l2.2, t.l2.3, fi);
                        if t.res <= t.sup { t.setup = false; }
                        if t.form {
                            if long {
                                if l[i] < t.sup && l[i - 1] >= sup_p { t.setup = false; }
                                if t.setup && st.pos != 1 { nx.push(o(A::Buy, P::Stop(t.res + pt), "DMILongTriangle")); }
                            } else {
                                if h[i] > t.res && h[i - 1] <= res_p { t.setup = false; }
                                if t.setup && st.pos != -1 { nx.push(o(A::Sell, P::Stop(t.sup - pt), "DMIShortTriangle")); }
                            }
                        }
                        if t.res <= t.sup { t.form = false; }
                        if long && st.pos == 1 {
                            t.setup = false;
                            if st.pos != t.mpp { t.prof = e + (t.ph[2] - t.pl[2]); t.prot = t.pl[1] - ax10[i] * ef; }
                            if c[i] >= e + mult * (e - t.prot) { t.prot = e; }
                            nx.push(o(A::Xl, P::Stop(t.prot), "Triangle stop")); nx.push(o(A::Xl, P::Lim(t.prof), "Triangle target"));
                        }
                        if !long && st.pos == -1 {
                            t.setup = false;
                            if st.pos != t.mpp { t.prof = e - (t.ph[2] - t.pl[2]); t.prot = t.ph[1] + ax10[i] * ef; }
                            if c[i] >= e - mult * (e - t.prot) { t.prot = e; }
                            nx.push(o(A::Xs, P::Stop(t.prot), "Triangle stop")); nx.push(o(A::Xs, P::Lim(t.prof), "Triangle target"));
                        }
                    }
                    t.mpp = st.pos;
                }
            })
        }
        "stad11_trendscore" => {
            let (lb, ln) = (u("lookback"), u("len"));
            let tsv: Vec<f64> = (0..n).map(|i| if i < lb + 10 { f64::NAN } else { (1..=10).map(|v| if c[i] - c[i - lb - v] >= 0.0 { 1.0 } else { -1.0 }).sum() }).collect();
            let (tsa, sm) = (sma(&tsv, ln), sma(c, ln));
            run_ts(x, from, to, (lb + 10 + ln).max(u("atrlen")), cfg(), |i, st, _now, nx| {
                if tsv[i] > tsa[i] && c[i] > sm[i] { nx.push(o(A::Buy, P::Mkt, "TrendScore Buy")); } else if st.pos == 1 { nx.push(o(A::Xl, P::Mkt, "TrendScore LX")); }
                if tsv[i] < tsa[i] && c[i] < sm[i] { nx.push(o(A::Sell, P::Mkt, "TrendScore Sell")); } else if st.pos == -1 { nx.push(o(A::Xs, P::Mkt, "TrendScore SX")); }
                atr_exits(x, i, st, &ax, atrv[i], &mut ast, nx);
            })
        }
        "stad11_gapper" => {
            let (pdi, mdi, adx) = dmi3(x, u("len"));
            run_ts(x, from, to, (2 * u("len") + 2).max(u("atrlen")), cfg(), |i, st, _now, nx| {
                if adx[i] > 25.0 && adx[i] > adx[i - 1] {
                    if pdi[i] > mdi[i] && nxo(i) < l[i] { nx.push(o(A::Buy, P::Stop(l[i]), "Gapper Buy")); }
                    if pdi[i] < mdi[i] && nxo(i) > h[i] { nx.push(o(A::Sell, P::Stop(h[i]), "Gapper Sell")); }
                }
                atr_exits(x, i, st, &ax, atrv[i], &mut ast, nx);
            })
        }
        // ============================================================ Volume 12
        "stad12_firstprize" => {
            let m = macd(c, u("fast"), u("slow")); let sg = xavg(&m, u("signal")); let xr = xavg(c, u("xavglen")); let a = atr(x, 10);
            let (ips, bes, tsf, vs, bps, nb, ap, bi, br) = (g("ips"), g("bes"), g("ts"), g("vs"), g("bps"), u("nb"), g("atrpcnt"), g("barsin") as usize, u("barsreq"));
            let mabove = bv(n, |k| m[k] > sg[k]); let mbelow = bv(n, |k| m[k] < sg[k]);
            let (mut bs, mut ss, mut nbs, mut nss, mut ctl, mut cth) = (false, false, false, false, 0.0, 0.0);
            run_ts(x, from, to, u("xavglen").max(u("slow") + u("signal")).max(nb), cfg(), |i, st, _now, nx| {
                let (up, dn, up1, dn1) = (sg[i] > 0.0, sg[i] < 0.0, sg[i - 1] > 0.0, sg[i - 1] < 0.0);
                if up {
                    if dn1 { nbs = true; ss = false; nss = false; }
                    if nbs { if xunder(&m, &sg, i) { bs = true; ctl = l[i]; nbs = false; } } else if mro(&mabove, i, br, br) != -1 { bs = false; }
                    if m[i] < sg[i] && l[i] < ctl { ctl = l[i]; }
                }
                if dn {
                    if up1 { nss = true; bs = false; nbs = false; }
                    if nss { if xover(&m, &sg, i) { ss = true; cth = h[i]; nss = false; } } else if mro(&mbelow, i, br, br) != -1 { ss = false; }
                    if m[i] > sg[i] && h[i] > cth { cth = h[i]; }
                }
                if st.pos == 1 { bs = false; }
                if st.pos == -1 { ss = false; }
                if bs && h[i] > xr[i] { nx.push(o(A::Buy, P::Stop(h[i] + pt), "First Prize Buy")); }
                if ss && l[i] < xr[i] { nx.push(o(A::Sell, P::Stop(l[i] - pt), "First Prize Sell")); }
                let e = st.entry_px;
                if st.pos == 1 {
                    let mtc = highest(c, i - st.entry_i + 1, i);
                    nx.push(o(A::Xl, P::Stop(e - ips * a[i]), "LX1"));
                    if mtc >= e + bes * a[i] { nx.push(o(A::Xl, P::Stop(e), "LX2")); }
                    nx.push(o(A::Xl, P::Stop(mtc - tsf * a[i]), "LX3")); nx.push(o(A::Xl, P::Stop(c[i] - vs * a[i]), "LX4"));
                    if mtc >= e + bps * a[i] { nx.push(o(A::Xl, P::Stop(lowest(l, nb, i)), "LX5")); }
                    nx.push(o(A::Xl, P::Stop(ctl - ap * a[i]), "LX6"));
                    if dn && st.bse(i) >= bi { nx.push(o(A::Xl, P::Stop(l[i]), "LX7")); }
                }
                if st.pos == -1 {
                    let mtc = lowest(c, i - st.entry_i + 1, i);
                    nx.push(o(A::Xs, P::Stop(e + ips * a[i]), "SX1"));
                    if mtc <= e - bes * a[i] { nx.push(o(A::Xs, P::Stop(e), "SX2")); }
                    nx.push(o(A::Xs, P::Stop(mtc + tsf * a[i]), "SX3")); nx.push(o(A::Xs, P::Stop(c[i] + vs * a[i]), "SX4"));
                    if mtc <= e - bps * a[i] { nx.push(o(A::Xs, P::Stop(highest(h, nb, i)), "SX5")); }
                    nx.push(o(A::Xs, P::Stop(cth + ap * a[i]), "SX6"));
                    if up && st.bse(i) >= bi { nx.push(o(A::Xs, P::Stop(h[i]), "SX7")); }
                }
            })
        }
        "stad12_inthezone" => {
            let a = atr(x, 10); let (ipsf, bef, ptf) = (g("ips"), g("be"), g("ptf"));
            let (mut ubz, mut lbz, mut bs, mut lep, mut lpt, mut lips) = (0.0, 0.0, false, 0.0, 0.0, 0.0);
            let (mut usz, mut lsz, mut ss, mut sep, mut spt, mut sips) = (0.0, 0.0, false, 0.0, 0.0, 0.0);
            let (mut thl_p, mut ths_p) = (0.0, 0.0);
            run_ts(x, from, to, 10, cfg(), |i, st, _now, nx| {
                if c[i - 1] > h[i - 3] {
                    ubz = h[i - 3]; lbz = lowest(l, 3, i - 1); bs = c[i] < ubz && c[i] > lbz && c[i] < c[i - 1];
                    if bs { lep = h[i] + pt; lpt = h[i - 1] + ptf * a[i]; lips = l[i] - ipsf * a[i]; }
                }
                if c[i] < lbz { bs = false; }
                if c[i - 1] < l[i - 3] {
                    usz = highest(h, 3, i - 1); lsz = l[i - 3]; ss = c[i] < usz && c[i] > lsz && c[i] > c[i - 1];
                    if ss { sep = l[i] - pt; spt = l[i - 1] - ptf * a[i]; sips = h[i] + ipsf * a[i]; }
                }
                if c[i] > usz { ss = false; }
                if bs && st.pos != 1 { nx.push(o(A::Buy, P::Stop(lep), "Zone Buy")); }
                if ss && st.pos != -1 { nx.push(o(A::Sell, P::Stop(sep), "Zone Sell")); }
                let e = st.entry_px;
                // "Close crosses above EntryPrice + k * (EntryPrice - LIPSValue)": the expression's previous value is
                // the previous bar's EntryPrice/LIPSValue (EntryPrice = 0 while flat)
                if st.pos == 1 {
                    let th = e + bef * (e - lips);
                    if c[i] > th && c[i - 1] <= thl_p { lips = e; }
                    nx.push(o(A::Xl, P::Stop(lips), "LIPS")); nx.push(o(A::Xl, P::Lim(lpt), "LPT"));
                }
                if st.pos == -1 {
                    let th = e - bef * (sips - e);
                    if c[i] > th && c[i - 1] <= ths_p { sips = e; }
                    nx.push(o(A::Xs, P::Stop(sips), "SIPS")); nx.push(o(A::Xs, P::Lim(spt), "SPT"));
                }
                thl_p = e + bef * (e - lips); ths_p = e - bef * (sips - e);
            })
        }
        "stad12_jailbreak" => {
            let (l1, ips) = (u("len1"), g("ips")); let l2 = l1.min(u("len2")); let a = atr(x, 10);
            run_ts(x, from, to, l1.max(10), cfg(), |i, st, _now, nx| {
                let e = st.entry_px;
                nx.push(o(A::Buy, P::Stop(highest(h, l1, i) + pt), "Jailbreak Buy"));
                if st.pos == 1 { nx.push(o(A::Xl, P::Stop(e - ips * a[i]), "Protective")); nx.push(o(A::Xl, P::Stop(lowest(l, l2, i) - pt), "Exit channel")); }
                nx.push(o(A::Sell, P::Stop(lowest(l, l1, i) - pt), "Jailbreak Sell"));
                if st.pos == -1 { nx.push(o(A::Xs, P::Stop(e + ips * a[i]), "Protective")); nx.push(o(A::Xs, P::Stop(highest(h, l2, i) + pt), "Exit channel")); }
            })
        }
        "stad12_lrcross" => {
            let (f, s) = (linreg(c, u("fast")), linreg(c, u("slow")));
            let mut cf = cfg(); cf.ss.sl = usd("sl_usd"); cf.ss.pt = usd("pt_usd"); cf.ss.pfloor = usd("pfloor_usd"); cf.ss.ppct = g("ppct");
            run_ts(x, from, to, u("slow").max(u("fast")) + 1, cf, |i, _st, _now, nx| {
                if xover(&f, &s, i) { nx.push(o(A::Buy, P::Mkt, "SLR Buy")); }
                if xunder(&f, &s, i) { nx.push(o(A::Sell, P::Mkt, "SLR Sell")); }
            })
        }
        "stad12_ntw" => {
            let (s, al, ln) = (u("strength"), u("align"), g("len"));
            let (pe, ps, px_) = (g("pente"), g("psetup"), g("pexit"));
            let ind = macd(c, u("fast"), u("slow")); let sgl = xavg(&ind, u("signal"));
            #[derive(Clone, Copy, Default)]
            struct Ntw { pxv: f64, pxb: f64, pzv: f64, pzb: f64, ixv: f64, ixb: f64, izv: f64, izb: f64, div: bool, cc: bool, eh: f64, bte: f64, xh: f64,
                ivl: f64, es: f64, esu: bool, sc: f64, xp: f64, xs: f64, ptg: f64, mpp: i8, th_p: f64 }
            // bullish: swing lows; bearish: swing highs
            let ppl = bv(n, |k| swing_low(l, s, k)); let ipl = bv(n, |k| swing_low(&ind, s, k));
            let pph = bv(n, |k| swing_high(h, s, k)); let iph = bv(n, |k| swing_high(&ind, s, k));
            let (mut bu, mut be) = (Ntw::default(), Ntw::default());
            run_ts(x, from, to, u("slow") + u("signal") + 2 * s + 2, cfg(), |i, st, _now, nx| {
                let e = st.entry_px;
                for (bull, t) in [(true, &mut bu), (false, &mut be)] {
                    let (pp, ip) = if bull { (&ppl, &ipl) } else { (&pph, &iph) };
                    let price = if bull { l } else { h };
                    let mut abp = false; let mut abi = false;
                    if pp[i] && mro(ip, i, al, 1) != -1 { abp = true; }
                    if abp {
                        t.pxv = t.pzv; t.pxb = t.pzb; t.ixv = t.izv; t.ixb = t.izb;
                        t.pzv = price[i - s]; t.pzb = s as f64; t.izb = (mro(ip, i, al, 1) + s as i64) as f64; t.izv = ind[i - t.izb as usize];
                        t.div = if bull { t.pxv < t.pzv && t.ixv > t.izv } else { t.pxv > t.pzv && t.ixv < t.izv } && t.pxv != 0.0;
                    }
                    if !pp[i] && ip[i] && mro(pp, i, al, 1) != -1 { abi = true; }
                    if abi {
                        t.ixv = t.izv; t.ixb = t.izb; t.pxv = t.pzv; t.pxb = t.pzb;
                        t.izv = ind[i - s]; t.izb = s as f64; t.pzb = (mro(pp, i, al, 1) + s as i64) as f64; t.pzv = price[i - t.izb as usize];
                        t.div = if bull { t.pxv < t.pzv && t.ixv > t.izv } else { t.pxv > t.pzv && t.ixv < t.izv } && t.pxv != 0.0;
                    }
                    if t.div {
                        t.div = false; t.cc = true;
                        let d = if bull { t.pzv - t.pxv } else { t.pxv - t.pzv };
                        t.eh = d * pe / 100.0; t.bte = (t.pxb - t.pzb) * ps / 100.0; t.xh = d * px_ / 100.0;
                        let span = (t.pxb - t.pzb).max(1.0) as usize; let off = t.pzb.max(0.0) as usize;
                        t.ivl = if bull { highest(h, span, i - off) } else { lowest(l, span, i - off) } - (t.pzv + t.pxv) / 2.0;
                    }
                    t.pxb = if t.pxb < ln { t.pxb + 1.0 } else { 0.0 };
                    t.pzb = if t.pzb < t.pxb { t.pzb + 1.0 } else { 0.0 };
                    if t.pzb == 0.0 { t.pzv = 0.0; }
                    t.ixb = if t.ixb < ln { t.ixb + 1.0 } else { 0.0 };
                    t.izb = if t.izb < ln { t.izb + 1.0 } else { 0.0 };
                    if t.cc && if bull { xover(&ind, &sgl, i) } else { xunder(&ind, &sgl, i) } {
                        t.cc = false; t.es = if bull { c[i] + t.eh } else { c[i] - t.eh }; t.esu = true; t.sc = t.bte; t.xp = t.xh;
                    }
                    if t.esu {
                        t.sc -= 1.0;
                        if t.sc == 0.0 { t.esu = false; }
                        if bull && st.pos != 1 { nx.push(o(A::Buy, P::Stop(t.es), "Not THAT Way Bullish")); }
                        if !bull && st.pos != -1 { nx.push(o(A::Sell, P::Stop(t.es), "Not THAT Way Bearish")); }
                    }
                    let th = if bull { e + t.xp } else { e - t.xp };
                    if bull && st.pos == 1 {
                        t.esu = false;
                        if st.pos != t.mpp { t.xs = e - t.xp; t.ptg = e + t.ivl; } else if c[i] > th && c[i - 1] <= t.th_p { t.xs = e; }
                        nx.push(o(A::Xl, P::Stop(t.xs), "Stop")); nx.push(o(A::Xl, P::Lim(t.ptg), "Profit"));
                    }
                    if !bull && st.pos == -1 {
                        t.esu = false;
                        if st.pos != t.mpp { t.xs = e + t.xp; t.ptg = e - t.ivl; } else if c[i] < th && c[i - 1] >= t.th_p { t.xs = e; }
                        nx.push(o(A::Xs, P::Stop(t.xs), "Stop")); nx.push(o(A::Xs, P::Lim(t.ptg), "Profit"));
                    }
                    t.th_p = th; t.mpp = st.pos;
                }
            })
        }
        "stad12_shifty" => {
            let (fa, sa) = (sma(c, u("fast")), sma(c, u("slow")));
            let mut cf = cfg(); cf.ss.dtr = usd("dtr_usd");
            let (mut tcu, mut lcu, mut tco, mut lco) = (0.0, 0.0, 0.0, 0.0);
            run_ts(x, from, to, u("slow").max(u("fast")) + 1, cf, |i, st, now, nx| {
                if c[i] > sa[i] { nx.push(o(A::Buy, P::Stop(fa[i] + pt), "Shifty Buy")); }
                if xunder(c, &fa, i) { lcu = tcu; tcu = c[i]; }
                if tcu < lcu && st.pos == 1 { nx.push(o(A::Xl, P::Stop(fa[i] - pt), "Lower cross")); } else if xunder(c, &sa, i) { now.push(o(A::Xl, P::Mkt, "Below slow")); }
                if c[i] < sa[i] { nx.push(o(A::Sell, P::Stop(fa[i] - pt), "Shifty Sell")); }
                if xover(c, &fa, i) { lco = tco; tco = c[i]; }
                if tco > lco && st.pos == -1 { nx.push(o(A::Xs, P::Stop(fa[i] + pt), "Higher cross")); } else if xover(c, &sa, i) { now.push(o(A::Xs, P::Mkt, "Above slow")); }
            })
        }
        "stad12_springs" => {
            let (s, cp, rp, ep) = (u("strength"), g("closepct") / 100.0, g("rangepct") / 100.0, g("entrypct") / 100.0);
            let a3 = sma(&tr, 3);
            let mut px = ParaX::new(g("acc"), g("mult"), g("acc"), g("mult"));
            let (mut pvl, mut lenl, mut spr, mut epl, mut xl) = (0.0, 0usize, false, 0.0, 0.0);
            let (mut pvh, mut lenh, mut upt, mut eph, mut xh) = (0.0, 0usize, false, 0.0, 0.0);
            run_ts(x, from, to, 21usize.max(2 * s + 1), cfg(), |i, st, _now, nx| {
                if swing_low(l, s, i) { pvl = l[i - s]; lenl = s; }
                if lenl < 20 { lenl += 1; } else { spr = false; }
                if l[i] < pvl && c[i] > pvl && c[i] >= h[i] - rg[i] * cp && rg[i] >= avg_at(&tr, lenl.max(3), i) * rp { spr = true; epl = nxo(i) + rg[i] * ep; xl = pvl - pt; }
                if spr { nx.push(o(A::Buy, P::Stop(epl), "Spring")); }
                if st.pos == 1 { spr = false; nx.push(o(A::Xl, P::Stop(xl), "Risk")); }
                if swing_high(h, s, i) { pvh = h[i - s]; lenh = s; }
                if lenh < 20 { lenh += 1; } else { upt = false; }
                if h[i] > pvh && c[i] < pvh && c[i] <= l[i] + rg[i] * cp && rg[i] >= avg_at(&tr, lenh.max(3), i) * rp { upt = true; eph = nxo(i) - rg[i] * ep; xh = pvh + pt; }
                if upt { nx.push(o(A::Sell, P::Stop(eph), "UpThrust")); }
                if st.pos == -1 { upt = false; nx.push(o(A::Xs, P::Stop(xh), "SRisk")); }
                px.step(x, i, st.pos, a3[i], nx, true, true);
            })
        }
        "stad12_musketeers" => {
            let a = atr(x, 10); let d = slowkd(x, u("len")).1;
            let (tlx, tsx, blx, bsx, upl, dnl) = (g("tlx"), g("tsx"), g("belx"), g("besx"), g("up"), g("dn"));
            let mut risk = 0.0;
            run_ts(x, from, to, u("len").max(10) + 2, cfg(), |i, st, _now, nx| {
                let (up, dn) = (d[i] > upl, d[i] < dnl);
                if st.pos != 1 && up && l[i] < l[i - 2] { nx.push(o(A::Buy, P::Stop(h[i]), "Musketeers Buy")); risk = l[i] - pt; }
                if st.pos != -1 && dn && h[i] > h[i - 2] { nx.push(o(A::Sell, P::Stop(l[i]), "Musketeers Sell")); risk = h[i] + pt; }
                let e = st.entry_px;
                if st.pos == 1 {
                    nx.push(o(A::Xl, P::Stop(risk), "Risk")); nx.push(o(A::Xl, P::Lim(e + tlx * (e - risk)), "Target"));
                    if highest(c, i - st.entry_i + 1, i) >= e + blx * a[i] { nx.push(o(A::Xl, P::Stop(e), "Breakeven")); }
                }
                if st.pos == -1 {
                    nx.push(o(A::Xs, P::Stop(risk), "Risk")); nx.push(o(A::Xs, P::Lim(e - tsx * (risk - e)), "Target"));
                    if lowest(c, i - st.entry_i + 1, i) <= e - bsx * a[i] { nx.push(o(A::Xs, P::Stop(e), "Breakeven")); }
                }
            })
        }
        "stad12_mist2" => {
            let ml = u("len"); let a3 = sma(&tr, 3);
            let c1 = bv(n, |k| k > ml && h[k] > highest(h, ml, k - 1) && c[k] > c[k - 1]);
            let c2 = bv(n, |k| k > ml && l[k] > lowest(l, ml, k - 1) && c[k] < c[k - 1]);
            let mut px = ParaX::new(g("acc"), g("mult"), g("acc"), g("mult"));
            run_ts(x, from, to, ml + 3, cfg(), |i, st, _now, nx| {
                if c1[i - 1] && o_[i] > h[i - 1] && c[i] < h[i - 1] { nx.push(o(A::Buy, P::Stop(h[i]), "L2")); }
                if c2[i - 1] && o_[i] < l[i - 1] && c[i] > l[i - 1] { nx.push(o(A::Sell, P::Stop(l[i]), "S2")); }
                // AT$ Low/High = the low/high of the bar on which the entry order was generated (entry bar - 1)
                if st.pos != 0 && st.entry_i >= 1 {
                    nx.push(of(A::Xl, P::Stop(l[st.entry_i - 1]), "AT$ stop", "L2")); nx.push(of(A::Xs, P::Stop(h[st.entry_i - 1]), "AT$ stop", "S2"));
                }
                px.step(x, i, st.pos, a3[i], nx, true, true);
            })
        }
        // ============================================================ Volume 13
        "stad13_ochist" => {
            let ln = u("len"); let (ef, xf) = (g("ef"), g("xf"));
            let (xc, xo) = (xavg(c, ln), xavg(o_, ln)); let a = atr(x, ln);
            let hist: Vec<f64> = (0..n).map(|i| xc[i] - xo[i]).collect();
            let xov = bv(n, |k| xover_k(&hist, 0.0, k)); let xun = bv(n, |k| xunder_k(&hist, 0.0, k));
            let (mut bp, mut sp, mut lxp, mut sxp) = (0.0, 0.0, 0.0, 0.0);
            run_ts(x, from, to, ln + 1, cfg(), |i, _st, now, nx| {
                if xov[i] { bp = h[i] + a[i] * ef; lxp = l[i] - a[i] * xf; }
                if xun[i] { sp = l[i] - a[i] * ef; sxp = h[i] + a[i] * xf; }
                if mro(&xov, i, 10, 1) > -1 && hist[i] > 0.0 { nx.push(o(A::Buy, P::Stop(bp), "LE")); }
                if mro(&xun, i, 10, 1) > -1 && hist[i] < 0.0 { nx.push(o(A::Sell, P::Stop(sp), "SE")); }
                if xun[i] { now.push(o(A::Xl, P::Mkt, "Histogram below 0")); }
                nx.push(of(A::Xl, P::Stop(lxp), "LongExitPrice", "LE"));
                if xov[i] { now.push(o(A::Xs, P::Mkt, "Histogram above 0")); }
                nx.push(of(A::Xs, P::Stop(sxp), "ShortExitPrice", "SE"));
            })
        }
        "stad13_dmarl" => {
            let (ln, dp) = (u("len"), g("disp").round() as usize);
            let (ub, lb) = (shift(&sma(h, ln), dp), shift(&sma(l, ln), dp));
            let mut cf = cfg(); cf.ss.sl = usd("sl_usd"); cf.ss.be = usd("be_usd"); cf.ss.dtr = usd("dtr_usd");
            run_ts(x, from, to, ln + dp + 1, cf, |i, _st, _now, nx| {
                let med = (h[i] + l[i]) / 2.0;
                if med > h[i - 1] && rg[i] > rg[i - 1] && c[i] > ub[i] { nx.push(o(A::Buy, P::Stop(h[i] + pt), "Range Leader Buy")); }
                if med < l[i - 1] && rg[i] > rg[i - 1] && c[i] < lb[i] { nx.push(o(A::Sell, P::Stop(l[i] - pt), "Range Leader Sell")); }
            })
        }
        "stad13_lrm" => {
            let xlr = xavg(&linreg(c, u("reg")), u("xma")); let mo = mom(c, u("mom")); let a = atr(x, 10);
            let (ep, ec) = (g("entrypct"), g("entrycounter"));
            let (mut lc, mut sc, mut bp, mut sp) = (0.0, 0.0, 0.0, 0.0);
            run_ts(x, from, to, (u("reg") + 1).max(u("mom") + 1).max(u("atrlen")).max(u("bpn")), cfg(), |i, st, _now, nx| {
                lc += 1.0; sc += 1.0;
                let bs = xlr[i] > xlr[i - 1] && c[i] > xlr[i] && mo[i] < 0.0 && mo[i] > mo[i - 1];
                let ss = xlr[i] < xlr[i - 1] && c[i] < xlr[i] && mo[i] > 0.0 && mo[i] < mo[i - 1];
                if bs { bp = h[i] + ep * a[i]; lc = 1.0; }
                if ss { sp = l[i] - ep * a[i]; sc = 1.0; }
                if lc <= ec { nx.push(o(A::Buy, P::Stop(bp), "LRM Buy")); }
                if sc <= ec { nx.push(o(A::Sell, P::Stop(sp), "LRM Sell")); }
                atr_exits(x, i, st, &ax, atrv[i], &mut ast, nx);
            })
        }
        "stad13_luxor" => {
            let (fl, sl) = (u("fast"), u("slow")); let (fa, sa) = (triavg(c, fl), triavg(c, sl));
            let adx = dmi3(x, u("adxlen")).2;
            let mut cf = cfg(); cf.max_ent = u("maxent");
            let (mut bst, mut sst) = (0.0, 0.0);
            run_ts(x, from, to, sl.max(2 * u("adxlen") + fl + 1), cf, |i, st, _now, nx| {
                let trending = adx[i] > adx[i - fl];
                if xover(&fa, &sa, i) { bst = h[i] + pt; }
                if xunder(&fa, &sa, i) { sst = l[i] - pt; }
                if fa[i] > sa[i] && st.pos == 0 { nx.push(o(A::Buy, P::Stop(bst), "Long")); }
                if fa[i] < sa[i] && st.pos == 0 { nx.push(o(A::Sell, P::Stop(sst), "Short")); }
                if st.pos == 1 { nx.push(o(A::Xl, P::Stop(sa[i] - pt), "Slow stop")); if h[i] < fa[i] && trending { nx.push(o(A::Buy, P::Stop(fa[i] + pt), "Longer")); } }
                if st.pos == -1 { nx.push(o(A::Xs, P::Stop(sa[i] + pt), "Slow stop")); if l[i] > fa[i] && trending { nx.push(o(A::Sell, P::Stop(fa[i] - pt), "Shorter")); } }
            })
        }
        "stad13_mrts" => {
            let (hb, lb, cl) = (xavg(h, u("chan")), xavg(l, u("chan")), xavg(c, u("closelen")));
            let ml = macd(c, u("fast"), u("slow")); let sg = xavg(&ml, u("signal")); let rsi = crate::connors::rsi(c, u("rsilen"));
            let (pdi, mdi, _) = dmi3(x, u("dmilen")); let dms: Vec<f64> = (0..n).map(|i| pdi[i] - mdi[i]).collect();
            let (ob, os, ttr, rch, bes) = (g("ob"), g("os"), g("ttr"), g("rsichange"), u("bestentry"));
            let lbelow = bv(n, |k| l[k] < hb[k]); let habove = bv(n, |k| h[k] > lb[k]);
            let mxd = bv(n, |k| xunder(&ml, &sg, k)); let mxu = bv(n, |k| xover(&ml, &sg, k));
            let (mut c3d, mut c3u) = (vec![false; n], vec![false; n]);
            let (mut chu, mut chd, mut chk, mut ctd, mut ctr, mut upt_p, mut dnt_p) = (false, false, false, false, false, false, false);
            let (mut uh, mut dl, mut hc, mut lc, mut hrsi, mut lrsi, mut bst, mut sst) = (0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
            let (mut hrsi_p, mut lrsi_p) = (0.0, 0.0);
            run_ts(x, from, to, (u("slow") + u("signal")).max(2 * u("dmilen") + 2).max(u("chan")).max(u("atrlen")).max(u("bpn")), cfg(), |i, st, _now, nx| {
                if st.pos != 0 { chk = false; if st.pos == 1 { ctd = false; } else { ctr = false; } }
                if xover(&cl, &hb, i) { chu = true; chd = false; }
                if xunder(&cl, &lb, i) { chd = true; chu = false; }
                let (mup, mdn) = (sg[i] > 0.0, sg[i] < 0.0);
                let (upt, dnt) = (chu && mup, chd && mdn);
                let (bull, bear) = (rsi[i] > ob && dms[i] > 15.0, rsi[i] < os && dms[i] < -15.0);
                if upt {
                    if !upt_p { uh = h[i]; hc = 0.0; hrsi = rsi[i]; chk = false; ctd = false; }
                    else { if h[i] > uh { uh = h[i]; hc = 0.0; } else if hc < ttr { hc += 1.0; } if rsi[i] > hrsi { hrsi = rsi[i]; } }
                }
                c3d[i] = (rsi[i] < 50.0 && rsi[i - 1] >= 50.0) || (rsi[i] < hrsi - rch && rsi[i - 1] >= hrsi_p - rch);
                if upt {
                    if bull { chk = true; }
                    if chk && hc != 0.0 && hc < ttr {
                        let k = hc as usize;
                        if mro(&lbelow, i, k, 1) != -1 && mro(&mxd, i, k, 1) != -1 && mro(&c3d, i, k, 1) != -1 { ctd = true; bst = highest(h, bes, i); chk = false; }
                    }
                    if ctd { nx.push(o(A::Buy, P::Stop(bst), "Counter-trend decline Buy")); }
                }
                if dnt {
                    if !dnt_p { dl = l[i]; lc = 0.0; lrsi = rsi[i]; chk = false; ctr = false; }
                    else { if l[i] < dl { dl = l[i]; lc = 0.0; } else if lc < ttr { lc += 1.0; } if rsi[i] < lrsi { lrsi = rsi[i]; } }
                }
                c3u[i] = (rsi[i] > 50.0 && rsi[i - 1] <= 50.0) || (rsi[i] > lrsi + rch && rsi[i - 1] <= lrsi_p + rch);
                if dnt {
                    if bear { chk = true; }
                    if chk && lc != 0.0 && lc < ttr {
                        let k = lc as usize;
                        if mro(&habove, i, k, 1) != -1 && mro(&mxu, i, k, 1) != -1 && mro(&c3u, i, k, 1) != -1 { ctr = true; sst = lowest(l, bes, i); chk = false; }
                    }
                    if ctr { nx.push(o(A::Sell, P::Stop(sst), "Counter-trend rally Sell")); }
                }
                if mdn && dms[i] < 0.0 { nx.push(o(A::Xl, P::Mkt, "Below 0")); }
                if mup && dms[i] > 0.0 { nx.push(o(A::Xs, P::Mkt, "Above 0")); }
                upt_p = upt; dnt_p = dnt; hrsi_p = hrsi; lrsi_p = lrsi;
                atr_exits(x, i, st, &ax, atrv[i], &mut ast, nx);
            })
        }
        "stad13_add" => {
            let (lt, stt, xl) = (u("longterm"), u("shortterm"), u("xavglen"));
            let v = vol(x); let al = 2.0 / (xl as f64 + 1.0);
            let mut cf = cfg(); cf.ss.sl = usd("sl_usd"); cf.ss.sl_pos_basis = true;
            let (mut adl, mut advx, mut adv, mut first) = (0.0, 0.0, 0.0, true);
            let (mut advh, mut ladh, mut hadh, mut cwh, mut csh) = (vec![0.0; n], vec![0.0; n], vec![0.0; n], vec![false; n], vec![false; n]);
            let (mut stp, mut mpp) = (0.0, 0i8);
            run_ts(x, from, to, lt + 1, cf, |i, st, _now, nx| {
                let mp = st.pos;
                if h[i] - l[i] != 0.0 { adl = (c[i] - o_[i]) / (h[i] - l[i]) * v[i]; }
                // XAverage(ADLine) is a series function: it advances every bar on the (held) ADLine value
                advx = if first { adl } else { advx + al * (adl - advx) }; first = false;
                if h[i] - l[i] != 0.0 { adv = advx; }
                advh[i] = adv; ladh[i] = lowest(&advh, lt, i); hadh[i] = highest(&advh, lt, i);
                cwh[i] = adv < ladh[i - 1]; csh[i] = adv > hadh[i - 1];
                if st.bn > 1 {
                    if l[i] < lowest(l, lt, i - 1) {
                        if adv > hadh[i - 1] { nx.push(ow(A::Buy, P::Mkt, 2.0, "Strong Buy")); }
                        else if mro(&cwh, i, stt, 1) == -1 { nx.push(ow(A::Buy, P::Mkt, 1.0, "Weak Buy")); }
                    }
                    if h[i] > highest(h, lt, i - 1) {
                        if adv < ladh[i - 1] { nx.push(ow(A::Sell, P::Mkt, 2.0, "Strong Sell")); }
                        else if mro(&csh, i, stt, 1) == -1 { nx.push(ow(A::Sell, P::Mkt, 1.0, "Weak Sell")); }
                    }
                }
                if mp == 1 { if mpp != 1 { stp = l[i] - avg_at(&rg, 4, i); } nx.push(o(A::Xl, P::Stop(stp), "AD LX")); stp += (l[i] - stp) / 3.0; }
                if mp == -1 { if mpp != -1 { stp = h[i] + avg_at(&rg, 4, i); } nx.push(o(A::Xs, P::Stop(stp), "AD SX")); stp -= (stp - h[i]) / 3.0; }
                mpp = mp;
            })
        }
        "stad13_tripleplay" => {
            let (pl, s, bte, epd, t1d, tsd) = (g("patlen"), u("strength"), g("bte"), g("epd"), g("t1d"), g("tsd"));
            let weak_rl = g("weak_src") >= 0.5;
            let (ub, lbd) = (bband(c, u("len"), g("dev")), bband(c, u("len"), -g("dev")));
            let swh = bv(n, |k| swing_high(h, s, k)); let swl = bv(n, |k| swing_low(l, s, k));
            let lim = pl + s as f64;
            let (mut sl_, mut sh, mut wl, mut wh, mut shb, mut slb) = (0.0, 0.0, 0.0, 0.0, 0.0f64, 0.0f64);
            let (mut lep, mut lxp, mut lt1, mut lt2, mut sep, mut sxp, mut st1, mut st2) = (99999.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
            let (mut fwl, mut fwh, mut ltt, mut stt, mut tdrop, mut trise, mut hcl, mut lcl) = (false, false, 0.0, 0.0, 0.0, 0.0, 0.0, 99999.0);
            let (mut bc, mut sc, mut mlt, mut mst, mut lts, mut sts) = (0.0, 0.0, false, false, false, false);
            run_ts(x, from, to, (2 * s + 1).max(u("len") + s), cfg(), |i, st, _now, nx| {
                shb = if shb <= lim { shb + 1.0 } else { shb };
                slb = if slb <= lim { slb + 1.0 } else { slb };
                if swh[i] && h[i - s] > ub[i - s] {
                    sh = h[i - s]; shb = s as f64;
                    let rl = mro(&swl, i, (slb - shb).max(0.0) as usize, 1);
                    if rl != -1 { wl = if weak_rl { l[i - rl as usize - s] } else if swl[i] { l[i - s] } else { -1.0 }; fwl = true; }
                }
                if swl[i] && l[i - s] < lbd[i - s] {
                    sl_ = l[i - s]; slb = s as f64;
                    let rh = mro(&swh, i, (shb - slb).max(0.0) as usize, 1);
                    if rh != -1 { wh = if weak_rl { h[i - rh as usize - s] } else if swh[i] { h[i - s] } else { -1.0 }; fwh = true; }
                }
                if fwl && slb < lim {
                    lep = (sh - sl_) / epd + wl; lxp = wl - pt; lt2 = wl + (sh - sl_); lt1 = (wl + lt2) / t1d; ltt = sh; tdrop = (sh - wl) / tsd;
                    bc = 0.0; mlt = true; mst = false;
                }
                if fwh && shb < lim {
                    sep = wh - (sh - sl_) / epd; sxp = wh + pt; st2 = wh - (sh - sl_); st1 = (wh + st2) / t1d; stt = sl_; trise = (wh - sl_) / tsd;
                    sc = 0.0; mst = true; mlt = false;
                }
                fwl = false; fwh = false;
                if st.pos == 0 {
                    if bc < bte { bc += 1.0; } else { mlt = false; }
                    if sc < bte { sc += 1.0; } else { mst = false; }
                    if mlt { nx.push(o(A::Buy, P::Stop(lep), "Buy 2")); }
                    if mst { nx.push(o(A::Sell, P::Stop(sep), "Sell 2")); }
                }
                let full = (st.units - 1.0).abs() < 1e-9;
                if st.pos == 1 {
                    mlt = false;
                    if full { lts = false; hcl = 0.0; nx.push(o(A::Xl, P::Stop(lxp), "LDump 1")); nx.push(oq(A::Xl, P::Lim(lt1), 0.5, "LTarget 1")); }
                    else { if c[i] > hcl { hcl = c[i]; } nx.push(o(A::Xl, P::Stop(st.entry_px), "LProtect 2")); if hcl > ltt { lts = true; }
                        if lts { nx.push(o(A::Xl, P::Stop(hcl - tdrop), "LTrail")); } nx.push(o(A::Xl, P::Lim(lt2), "LTarget 2")); }
                }
                if st.pos == -1 {
                    mst = false;
                    if full { sts = false; lcl = 99999.0; nx.push(o(A::Xs, P::Stop(sxp), "SDump 1")); nx.push(oq(A::Xs, P::Lim(st1), 0.5, "STarget 1")); }
                    else { if c[i] < lcl { lcl = c[i]; } nx.push(o(A::Xs, P::Stop(st.entry_px), "SProtect 2")); if lcl < stt { sts = true; }
                        if sts { nx.push(o(A::Xs, P::Stop(lcl + trise), "STrail")); } nx.push(o(A::Xs, P::Lim(st2), "STarget 2")); }
                }
            })
        }
        "stad13_volex" => {
            let (lf, sf, trl, delay) = (g("lf"), g("sf"), u("trail"), g("delay") as usize);
            let (stale_t, stale_pts, go) = (g("stale_t") as usize, usd("stale_usd"), g("go_usd") / g("bpv"));
            let mut cf = cfg(); cf.ss.sl = usd("sl_usd");
            run_ts(x, from, to, 4usize.max(trl), cf, |i, st, _now, nx| {
                let on = nxo(i); let ar = avg_at(&rg, 4, i);
                nx.push(o(A::Buy, P::Stop(on + ar * lf), "VolEx Buy")); nx.push(o(A::Sell, P::Stop(on - ar * sf), "VolEx Sell"));
                if st.pos == 1 { nx.push(o(A::Xl, P::Stop(lowest(l, trl, i)), "Price Trailing")); }
                if st.pos == -1 { nx.push(o(A::Xs, P::Stop(highest(h, trl, i)), "Price Trailing")); }
                let e = st.entry_px;
                let opp = if st.pos == 0 { 0.0 } else { (c[i] - e) * st.pos as f64 };
                if st.bse(i) > stale_t && opp < stale_pts { nx.push(o(A::Xl, P::Mkt, "Long Too Long")); nx.push(o(A::Xs, P::Mkt, "Short Too Long")); }
                if st.bse(i) > delay {
                    if st.pos == 1 && on > e + go { nx.push(o(A::Xl, P::Mkt, "Long Profit")); }
                    if st.pos == -1 && on < e - go { nx.push(o(A::Xs, P::Mkt, "Short Profit")); }
                }
            })
        }
        "stad13_volclus" => {
            let (dv, ln, delay) = (g("dev"), u("len"), g("delay") as usize); let go = g("go_usd") / g("bpv");
            let ctc: Vec<f64> = (0..n).map(|i| if i == 0 { 0.0 } else { (c[i] - c[i - 1]).abs() }).collect();
            let (cm, cs, rm, rs) = (sma(&ctc, ln), stdev(&ctc, ln), sma(&rg, ln), stdev(&rg, ln));
            let a3 = sma(&rg, 3);
            let mut cf = cfg(); cf.ss.sl = usd("sl_usd");
            run_ts(x, from, to, ln + 2, cf, |i, st, _now, nx| {
                let big = ctc[i] > cm[i - 1] + cs[i - 1] * dv || rg[i] > rm[i - 1] + rs[i - 1] * dv;
                if big { nx.push(o(A::Buy, P::Stop(c[i] + a3[i - 1]), "VolClus Buy")); nx.push(o(A::Sell, P::Stop(c[i] - a3[i - 1]), "VolClus Sell")); }
                let (e, on) = (st.entry_px, nxo(i));
                if st.bse(i) > delay {
                    if st.pos == 1 && on > e + go { nx.push(o(A::Xl, P::Mkt, "Long Profit")); }
                    if st.pos == -1 && on < e - go { nx.push(o(A::Xs, P::Mkt, "Short Profit")); }
                }
            })
        }
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn rows(v: &[(f64, f64, f64, f64)]) -> Vec<Row> {
        v.iter().enumerate().map(|(i, &(o, h, l, c))| Row { ticker: "T".into(), date: format!("2000-01-{:02}", i + 1), open: o, high: h, low: l, close: c, volume: Some(1000.0) }).collect()
    }
    #[test]
    fn pyramid_adds_legs_and_partial_exit_averages() {
        let r = rows(&[(100., 101., 99., 100.), (100., 102., 99., 101.), (101., 104., 100., 103.), (103., 106., 102., 105.), (105., 106., 104., 105.)]);
        let x = Ctx::new(&r, &r, &[]);
        let mut cf = cfg(); cf.max_ent = 2;
        let ts = run_ts(&x, 0, 4, 0, cf, |i, _st, _now, nx| {
            if i == 0 || i == 1 { nx.push(o(A::Buy, P::Mkt, "B")); }
            if i == 2 { nx.push(oq(A::Xl, P::Lim(105.0), 1.0, "half")); }
            if i == 3 { nx.push(o(A::Xl, P::Mkt, "rest")); }
        });
        assert_eq!(ts.len(), 1);
        assert_eq!(ts[0].legs.len(), 2);
        assert_eq!((ts[0].legs[0].px, ts[0].legs[1].px), (100.0, 101.0));
        assert_eq!(ts[0].exits.len(), 2);
        assert!((ts[0].exit_px - 105.0).abs() < 1e-9);
    }
    #[test]
    fn breakeven_set_stop_acts_next_bar_from_mfe() {
        let r = rows(&[(100., 101., 99., 100.), (100., 103., 99.5, 102.), (101., 101.5, 99., 99.5)]);
        let x = Ctx::new(&r, &r, &[]);
        let mut cf = cfg(); cf.ss.be = 2.0;
        let ts = run_ts(&x, 0, 2, 0, cf, |i, _st, _now, nx| if i == 0 { nx.push(o(A::Buy, P::Mkt, "B")); });
        assert_eq!(ts.len(), 1);
        assert_eq!((ts[0].legs[0].px, ts[0].exit_px, ts[0].exit_i), (100.0, 100.0, 2));
    }
    #[test]
    fn weekly_bar_maps_to_last_completed_week() {
        // 2024-01-01 is a Monday
        let r: Vec<Row> = (0..10).map(|k| Row { ticker: "T".into(), date: format!("2024-01-{:02}", [1, 2, 3, 4, 5, 8, 9, 10, 11, 12][k]), open: 1.0, high: 2.0 + k as f64, low: 0.5, close: 1.5, volume: None }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let w = weekly(&x);
        assert_eq!(w.c.len(), 2);
        assert_eq!(w.map, vec![-1, -1, -1, -1, 0, 0, 0, 0, 0, 1]);
        assert_eq!(w.h[0], 6.0);
    }
    #[test]
    fn ids_unique_prefixed_and_every_live_strategy_runs() {
        let cat = catalogue();
        let mut ids: Vec<_> = cat.iter().map(|d| d.id).collect(); ids.sort(); ids.dedup();
        assert_eq!(ids.len(), cat.len());
        let r: Vec<Row> = (0..700).map(|i| { let b = 100.0 + 10.0 * ((i as f64) / 9.0).sin() + 6.0 * ((i as f64) / 41.0).cos() + (i % 7) as f64;
            Row { ticker: "T".into(), date: format!("{:04}-{:02}-{:02}", 2000 + i / 250, 1 + (i % 250) / 21, 1 + (i % 250) % 21), open: b, high: b + 2.0 + (i % 3) as f64, low: b - 2.0, close: b + 0.5 - (i % 5) as f64 * 0.4, volume: Some(1000.0 + (i * 37 % 101) as f64) } }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let mut live = 0;
        for d in cat.iter().filter(|d| d.status == Status::Live) {
            live += 1;
            assert!(d.id.starts_with("stad") && !crate::books::stad::catalogue().iter().any(|s| s.id == d.id), "{}", d.id);
            let ts = run(d.id, &x, 0, 699, &Params::new()).unwrap_or_else(|| panic!("{} did not run", d.id));
            for t in &ts {
                let mut fills: Vec<(usize, f64)> = t.legs.iter().map(|g| (g.i, g.px)).collect();
                fills.extend(t.exits.iter().map(|e| (e.0, e.1)));
                if t.exits.is_empty() { fills.push((t.exit_i, t.exit_px)); }
                for (k, px) in fills { assert!(px >= x.l[k] - 1e-9 && px <= x.h[k] + 1e-9, "{} fill {} outside bar {}", d.id, px, k); }
            }
            assert!(family_of(d.id).is_some());
            assert!(params(d.id).is_some());
        }
        assert_eq!(live, 62);
    }
}
