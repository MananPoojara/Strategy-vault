//! Omega Research, "System Trading and Development Club" (STAD), Vol. 1-13 (1997-2000). Book id `stad`.
//! Source copies: Books/Omega Research/STAD01..13.pdf (+ the CD's compiled .ELA/.ELS archives, not readable as text).
//! Transcription, testability table, verification: library/stad/README.md.
//!
//! Every Live strategy is a literal port of the volume's printed EasyLanguage code (the code wins over the prose
//! where they differ; each such case is listed in the rule text and README). Strategies are evaluated the way
//! TradeStation evaluates them on daily bars:
//! * the code runs at each bar's close (from MaxBarsBack = the longest lookback the code uses; BarNumber 1 there);
//!   "this bar on close" orders fill at that close; "next bar" orders are active for the next bar only;
//! * MarketPosition / EntryPrice / BarsSinceEntry / PositionProfit read inside the code are the values BEFORE this
//!   bar's own close orders fill (TradeStation fills orders after the evaluation);
//! * intrabar path (STAD Vol. 5, ch.1 "Bar Assumptions & Bouncing Ticks"): open nearer the low -> O-L-H-C, open nearer
//!   the high -> O-H-L-C (a tie, which the book does not cover, runs O-L-H-C); after each stop/limit fill the path
//!   "bounces back" 10% of the bar's range (the book's stated default) before continuing; a stop or limit that the
//!   open already satisfies fills at the open; market orders fill at the open;
//! * Buy/Sell reverse an opposite position; no pyramiding ("Do not allow multiple entries in same direction",
//!   Vol. 1 General System Format); exit orders only act on the position that existed when they were issued.
//! Book money-management stops whose amount the book leaves to the user ("an appropriate dollar amount") are
//! not modelled. Share counts in the code become leg weights relative to the strategy's initial size.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "stad";
/// Admitted ids (authors/stad/admission/SOURCE-CONTRACT.md).
pub const REVIEWED: &[&str] = &["stad1_maco", "stad1_macdbo", "stad1_tf", "stad1_sr", "stad1_pivot", "stad1_chbo", "stad2_adxdiff", "stad2_hs", "stad2_addiv", "stad2_ccibb", "stad2_pctchan", "stad2_stats3", "stad3_bdsr", "stad3_engulf", "stad3_ochist", "stad3_bb", "stad3_varzones", "stad3_sdv", "stad3_volclus", "stad4_dmiadx", "stad4_keltner", "stad4_linreg", "stad4_cc", "stad4_3xa", "stad4_dmacv", "stad4_pricedist", "stad4_momret", "stad7_vix", "stad3_mmmw", "stad10_mwt", "stad2_stats2" /* intraday wave 2b: daily bars decide every trade */];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

// ------------------------------------------------------------------ TradeStation functions

fn sma(v: &[f64], n: usize) -> Vec<f64> {
    let mut out = vec![f64::NAN; v.len()];
    if n == 0 { return out; }
    // window sum recomputed each bar (no running-sum drift: equality tests like Close <= Average(Close,2) must be exact)
    for i in n - 1..v.len() { out[i] = v[i + 1 - n..=i].iter().sum::<f64>() / n as f64; }
    out
}
/// XAverage: alpha 2/(n+1), seeded with the first value.
fn xavg(v: &[f64], n: usize) -> Vec<f64> {
    let a = 2.0 / (n as f64 + 1.0);
    let mut out = vec![f64::NAN; v.len()];
    for i in 0..v.len() { out[i] = if i == 0 { v[0] } else { out[i - 1] + a * (v[i] - out[i - 1]) }; }
    out
}
fn true_range(x: &Ctx) -> Vec<f64> {
    (0..x.c.len()).map(|i| if i == 0 { x.h[0] - x.l[0] } else { x.h[i].max(x.c[i - 1]) - x.l[i].min(x.c[i - 1]) }).collect()
}
/// TradeStation Volatility(n): Wilder-smoothed true range, seeded with the n-bar average true range.
fn volatility(x: &Ctx, n: usize) -> Vec<f64> {
    let tr = true_range(x);
    let a = sma(&tr, n);
    let mut out = vec![f64::NAN; tr.len()];
    for i in 0..tr.len() {
        out[i] = if i == 0 || !out[i - 1].is_finite() { a[i] } else { ((n as f64 - 1.0) * out[i - 1] + tr[i]) / n as f64 };
    }
    out
}
fn highest(v: &[f64], n: usize, i: usize) -> f64 { v[(i + 1).saturating_sub(n)..=i].iter().copied().fold(f64::NEG_INFINITY, f64::max) }
fn lowest(v: &[f64], n: usize, i: usize) -> f64 { v[(i + 1).saturating_sub(n)..=i].iter().copied().fold(f64::INFINITY, f64::min) }
fn xover(a: &[f64], b: &[f64], i: usize) -> bool { i > 0 && a[i] > b[i] && a[i - 1] <= b[i - 1] }
fn xunder(a: &[f64], b: &[f64], i: usize) -> bool { i > 0 && a[i] < b[i] && a[i - 1] >= b[i - 1] }
/// SwingHighBar(1, H, s, s+1) = s: bar i-s high >= the s bars before it and > the s bars after it.
fn swing_high(h: &[f64], s: usize, i: usize) -> bool {
    i >= 2 * s && (1..=s).all(|k| h[i - s] >= h[i - s - k] && h[i - s] > h[i - s + k])
}
fn swing_low(l: &[f64], s: usize, i: usize) -> bool {
    i >= 2 * s && (1..=s).all(|k| l[i - s] <= l[i - s - k] && l[i - s] < l[i - s + k])
}


/// WAverage: linearly weighted average (newest weight n).
fn wavg(v: &[f64], n: usize) -> Vec<f64> {
    let den = (n * (n + 1) / 2) as f64;
    (0..v.len()).map(|i| if i + 1 < n { f64::NAN } else { (0..n).map(|k| (n - k) as f64 * v[i - k]).sum::<f64>() / den }).collect()
}
/// StdDev (population, TradeStation) and StdDevS (sample, STAD Vol 4 p.89).
fn stdev(v: &[f64], n: usize, ddof: f64) -> Vec<f64> {
    let m = sma(v, n);
    (0..v.len()).map(|i| if i + 1 < n { f64::NAN } else {
        (v[i + 1 - n..=i].iter().map(|x| (x - m[i]).powi(2)).sum::<f64>() / (n as f64 - ddof)).sqrt() }).collect()
}
fn bband(v: &[f64], n: usize, k: f64) -> Vec<f64> {
    let (m, s) = (sma(v, n), stdev(v, n, 0.0));
    (0..v.len()).map(|i| m[i] + k * s[i]).collect()
}
/// CCI(n) on (H+L+C)/3 with mean deviation.
fn cci(x: &Ctx, n: usize) -> Vec<f64> {
    let tp: Vec<f64> = (0..x.c.len()).map(|i| (x.h[i] + x.l[i] + x.c[i]) / 3.0).collect();
    let m = sma(&tp, n);
    (0..tp.len()).map(|i| if i + 1 < n { f64::NAN } else {
        let md = tp[i + 1 - n..=i].iter().map(|v| (v - m[i]).abs()).sum::<f64>() / n as f64;
        if md == 0.0 { 0.0 } else { (tp[i] - m[i]) / (0.015 * md) } }).collect()
}
/// SlowK(n), legacy TradeStation smoothing (help.tradestation.com, Stochastic SmoothingType 2): FastK smoothed
/// as ((3-1)*prev + FastK)/3, seeded with FastK.
fn slowk(x: &Ctx, n: usize) -> Vec<f64> {
    let mut out = vec![f64::NAN; x.c.len()];
    for i in 0..x.c.len() {
        if i + 1 < n { continue; }
        let (hh, ll) = (highest(&x.h, n, i), lowest(&x.l, n, i));
        let fk = if hh > ll { (x.c[i] - ll) / (hh - ll) * 100.0 } else { 0.0 };
        out[i] = if i == 0 || !out[i - 1].is_finite() { fk } else { (2.0 * out[i - 1] + fk) / 3.0 };
    }
    out
}
/// Wilder DMI: (+DI, -DI, ADX), same smoothing as connors::dmi.
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
/// LinearRegValue(price, n, 0): least-squares line over the last n bars, value at the current bar.
fn linreg(v: &[f64], n: usize) -> Vec<f64> {
    let nf = n as f64;
    let sx = nf * (nf - 1.0) / 2.0;
    let sxx = (nf - 1.0) * nf * (2.0 * nf - 1.0) / 6.0;
    (0..v.len()).map(|i| if i + 1 < n { f64::NAN } else {
        // x = 0 for the oldest bar .. n-1 for the current bar
        let (mut sy, mut sxy) = (0.0, 0.0);
        for k in 0..n { let y = v[i + 1 - n + k]; sy += y; sxy += k as f64 * y; }
        let b = (nf * sxy - sx * sy) / (nf * sxx - sx * sx);
        let a = (sy - b * sx) / nf;
        a + b * (nf - 1.0) }).collect()
}
/// Volume, 0 where the file has none (TradeStation treats a missing volume as 0).
fn vol(x: &Ctx) -> Vec<f64> { x.r.iter().map(|r| r.volume.filter(|v| v.is_finite()).unwrap_or(0.0)).collect() }
fn obv(x: &Ctx) -> Vec<f64> {
    let v = vol(x);
    let mut out = vec![0.0; v.len()];
    for i in 1..v.len() { out[i] = out[i - 1] + if x.c[i] > x.c[i - 1] { v[i] } else if x.c[i] < x.c[i - 1] { -v[i] } else { 0.0 }; }
    out
}
/// AccumDist(Volume): running total of ((C-L)-(H-C))/(H-L)*V (0 when H = L).
fn accumdist(x: &Ctx) -> Vec<f64> {
    let v = vol(x);
    let mut out = vec![0.0; v.len()];
    for i in 0..v.len() {
        let r = x.h[i] - x.l[i];
        let add = if r > 0.0 { ((x.c[i] - x.l[i]) - (x.h[i] - x.c[i])) / r * v[i] } else { 0.0 };
        out[i] = if i == 0 { add } else { out[i - 1] + add };
    }
    out
}
/// STAD Vol 4 p.88-89 Kurtosis and Skew functions (sample standard deviation).
fn kurtosis(v: &[f64], n: usize) -> Vec<f64> {
    let (m, s) = (sma(v, n), stdev(v, n, 1.0));
    let nf = n as f64;
    let p1 = nf * (nf + 1.0) / ((nf - 1.0) * (nf - 2.0) * (nf - 3.0));
    let p3 = 3.0 * (nf - 1.0).powi(2) / ((nf - 2.0) * (nf - 3.0));
    (0..v.len()).map(|i| if i + 1 < n || s[i] == 0.0 { f64::NAN } else {
        p1 * (0..n).map(|k| ((v[i - k] - m[i]) / s[i]).powi(4)).sum::<f64>() - p3 }).collect()
}
fn skew(v: &[f64], n: usize) -> Vec<f64> {
    let (m, s) = (sma(v, n), stdev(v, n, 1.0));
    let nf = n as f64;
    let y = nf / ((nf - 1.0) * (nf - 2.0));
    (0..v.len()).map(|i| if i + 1 < n || s[i] == 0.0 { f64::NAN } else {
        y * (0..n).map(|k| ((v[i - k] - m[i]) / s[i]).powi(3)).sum::<f64>() }).collect()
}
/// MRO(cond, len, inst) over a precomputed condition series: bars ago of the inst-th most recent true within len bars, or -1.
fn mro(c: &[bool], i: usize, len: usize, inst: usize) -> i64 {
    let mut k = 0;
    for b in 0..len.min(i + 1) { if c[i - b] { k += 1; if k == inst { return b as i64; } } }
    -1
}
/// (year, month, day, weekday 0=Mon) from "YYYY-MM-DD".
fn ymdw(d: &str) -> (i64, i64, i64, i64) {
    let y: i64 = d[0..4].parse().unwrap_or(0); let m: i64 = d[5..7].parse().unwrap_or(1); let dd: i64 = d[8..10].parse().unwrap_or(1);
    let (yy, mm) = if m <= 2 { (y - 1, m + 9) } else { (y, m - 3) };
    let era = yy.div_euclid(400); let yoe = yy - era * 400;
    let doy = (153 * mm + 2) / 5 + dd - 1; let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146097 + doe - 719468; // days since 1970-01-01 (a Thursday)
    (y, m, dd, (days + 3).rem_euclid(7))
}
fn days(d: &str) -> i64 { let (y, m, dd, _) = ymdw(d); let (yy, mm) = if m <= 2 { (y - 1, m + 9) } else { (y, m - 3) };
    let era = yy.div_euclid(400); let yoe = yy - era * 400; let doy = (153 * mm + 2) / 5 + dd - 1;
    era * 146097 + yoe * 365 + yoe / 4 - yoe / 100 + doy - 719468 }
/// The StopPrice trailing exit used across Vols 2-4: on the first bar in a position
/// stop = Low - k*Average(Range,4) (High + ... short); each bar exit at the stop, then move it by frac of the gap.
#[derive(Default)]
struct Para { sp: f64, mpp: i8 }
impl Para {
    fn step(&mut self, x: &Ctx, i: usize, mp: i8, ar4: f64, k: f64, fl: f64, fs: f64, nx: &mut Vec<O>, tag: &'static str) {
        if mp == 1 && self.mpp != 1 { self.sp = x.l[i] - ar4 * k; }
        if mp == -1 && self.mpp != -1 { self.sp = x.h[i] + ar4 * k; }
        if mp == 1 { nx.push(o(A::Xl, P::Stop(self.sp), tag)); self.sp += (x.l[i] - self.sp) * fl; }
        if mp == -1 { nx.push(o(A::Xs, P::Stop(self.sp), tag)); self.sp -= (self.sp - x.h[i]) * fs; }
        self.mpp = mp;
    }
}

// ------------------------------------------------------------------ order engine

#[derive(Clone, Copy, PartialEq, Debug)]
enum A { Buy, Sell, Xl, Xs }
#[derive(Clone, Copy, Debug)]
enum P { Mkt, Stop(f64), Lim(f64) }
#[derive(Clone, Copy, Debug)]
struct O { a: A, p: P, w: f64, tag: &'static str, from: &'static str, prot: f64 }
fn o(a: A, p: P, tag: &'static str) -> O { O { a, p, w: 1.0, tag, from: "", prot: f64::NAN } }
fn ow(a: A, p: P, w: f64, tag: &'static str) -> O { O { a, p, w, tag, from: "", prot: f64::NAN } }
/// Exit that only applies to the position opened by entry `from` ("ExitLong from entry (...)").
fn of(a: A, p: P, tag: &'static str, from: &'static str) -> O { O { a, p, w: 1.0, tag, from, prot: f64::NAN } }

/// What the strategy code can read (TradeStation reserved words), as of the start of its evaluation.
#[derive(Clone, Copy, Default)]
struct St { pos: i8, entry_px: f64, entry_i: usize, prev_pos: i8, bn: i64 }
impl St {
    fn profit_pts(&self, c: f64) -> f64 { if self.pos == 0 { 0.0 } else { self.pos as f64 * (c - self.entry_px) } }
}

struct Eng<'a> { x: &'a Ctx<'a>, pos: i8, pid: u64, cur: Option<Trade>, prev_pos: i8, out: Vec<Trade>, tag: &'static str, prot: f64 }

impl<'a> Eng<'a> {
    fn close_pos(&mut self, i: usize, px: f64, why: &str, sig: usize) {
        if let Some(mut t) = self.cur.take() {
            t.exit_i = i; t.exit_px = px; t.reason = why.into(); t.signal_i = sig; self.out.push(t);
        }
        self.prev_pos = self.pos; self.pos = 0; self.pid += 1; self.prot = f64::NAN;
    }
    fn exec(&mut self, od: &O, i: usize, px: f64, kind: &'static str, sig: usize) {
        match od.a {
            A::Buy | A::Sell => {
                let d: i8 = if od.a == A::Buy { 1 } else { -1 };
                if self.pos == -d { self.close_pos(i, px, od.tag, sig); }
                self.pos = d; self.pid += 1; self.tag = od.tag; self.prot = od.prot;
                self.cur = Some(Trade { dir: d as f64, legs: vec![Leg { i, px, w: od.w }], exit_i: i, exit_px: px,
                    reason: String::new(), entry_kind: kind, signal_i: sig, open: false, exits: vec![] });
            }
            A::Xl | A::Xs => self.close_pos(i, px, od.tag, sig),
        }
    }
    /// TradeStation: entries never add to a same-side position; exit orders act on whatever position of their
    /// side exists when the price is reached, including one entered earlier on the same bar (the volumes rely on
    /// this for entry-bar stops, e.g. Vol 2 ADX Differential "If MP <> 1 then ExitLong ... stop").
    fn valid(&self, od: &O, _pid0: u64, _pos0: i8) -> bool {
        let tag_ok = od.from.is_empty() || od.from == self.tag;
        match od.a {
            A::Buy => self.pos != 1,
            A::Sell => self.pos != -1,
            A::Xl => self.pos == 1 && tag_ok,
            A::Xs => self.pos == -1 && tag_ok,
        }
    }
    /// Fill the orders issued at bar i-1 on bar i.
    fn fill_bar(&mut self, i: usize, ords0: &[O]) {
        let mut ords: Vec<O> = ords0.to_vec();
        // an "at$" protective stop of the open position is re-issued every bar
        if self.pos != 0 && self.prot.is_finite() {
            ords.push(O { a: if self.pos == 1 { A::Xl } else { A::Xs }, p: P::Stop(self.prot), w: 1.0, tag: "Entry-bar stop (at$)", from: self.tag, prot: f64::NAN });
        }
        if ords.is_empty() { return; }
        let x = self.x;
        let (op, hi, lo, cl) = (x.o[i], x.h[i], x.l[i], x.c[i]);
        let (pid0, pos0) = (self.pid, self.pos);
        let mut done = vec![false; ords.len()];
        macro_rules! fire { ($k:expr, $px:expr) => {{
            done[$k] = true; let od = ords[$k]; self.exec(&od, i, $px, kind_of(&od), i - 1);
            if matches!(od.a, A::Buy | A::Sell) && od.prot.is_finite() {
                ords.push(O { a: if od.a == A::Buy { A::Xl } else { A::Xs }, p: P::Stop(od.prot), w: 1.0, tag: "Entry-bar stop (at$)", from: od.tag, prot: f64::NAN });
                done.push(false);
            }
        }}; }
        // market orders at the open: exits, then entries
        for pass in 0..2 {
            for k in 0..ords.len() {
                let od = &ords[k].clone();
                let is_exit = matches!(od.a, A::Xl | A::Xs);
                if matches!(od.p, P::Mkt) && (pass == 0) == is_exit && !done[k] && self.valid(od, pid0, pos0) {
                    fire!(k, op);
                }
            }
        }
        let rising = |a: A| matches!(a, A::Buy | A::Xs); // stop side that triggers on the way up
        // stops / limits the open already satisfies
        loop {
            let hit = ords.iter().enumerate().find(|(k, od)| !done[*k] && self.valid(od, pid0, pos0) && match od.p {
                P::Stop(s) => if rising(od.a) { op >= s } else { op <= s },
                P::Lim(s) => if rising(od.a) { op <= s } else { op >= s },
                P::Mkt => false,
            }).map(|(k, _)| k);
            match hit { Some(k) => { fire!(k, op); } None => break }
        }
        // intrabar path with 10% bounce after each fill
        let bounce = 0.1 * (hi - lo);
        let mut path: std::collections::VecDeque<f64> =
            if hi - op < op - lo { [hi, lo, cl].into() } else { [lo, hi, cl].into() };
        let mut cur = op;
        while let Some(&t) = path.front() {
            let up = t > cur;
            let mut best: Option<(usize, f64)> = None;
            for (k, od) in ords.iter().enumerate() {
                if done[k] || !self.valid(od, pid0, pos0) { continue; }
                let lv = match od.p {
                    P::Stop(s) if rising(od.a) == up => s,
                    P::Lim(s) if rising(od.a) != up => s,
                    _ => continue,
                };
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
}
fn kind_of(od: &O) -> &'static str { match od.p { P::Mkt => "open", P::Stop(_) => "stop", P::Lim(_) => "limit" } }

/// Run TradeStation-style strategy code `f(i, state, close_orders, next_bar_orders)` over bars 0..=to.
/// Indicators and the code's own variables run from MaxBarsBack (`mbb`); orders are only taken from `from` on.
fn run_ts(x: &Ctx, from: usize, to: usize, mbb: usize, mut f: impl FnMut(usize, &St, &mut Vec<O>, &mut Vec<O>)) -> Vec<Trade> {
    let mut e = Eng { x, pos: 0, pid: 0, cur: None, prev_pos: 0, out: vec![], tag: "", prot: f64::NAN };
    let mut pending: Vec<O> = vec![];
    for i in 0..=to {
        if i > from { e.fill_bar(i, &pending); }
        pending.clear();
        if i < mbb { continue; }
        let st = St { pos: e.pos, entry_px: e.cur.as_ref().map_or(0.0, |t| t.legs[0].px),
            entry_i: e.cur.as_ref().map_or(0, |t| t.legs[0].i), prev_pos: e.prev_pos, bn: (i - mbb + 1) as i64 };
        let (mut now, mut next) = (vec![], vec![]);
        f(i, &st, &mut now, &mut next);
        if i >= from {
            let pid0 = e.pid;
            for od in now.iter() {
                if e.valid(od, pid0, st.pos) { e.exec(od, i, x.c[i], "close", i); }
            }
            if i < to { pending = next; }
        }
    }
    if let Some(mut t) = e.cur.take() { t.exit_i = to; t.exit_px = x.c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true; e.out.push(t); }
    e.out
}

// ------------------------------------------------------------------ catalogue

struct Def { id: &'static str, name: &'static str, group: &'static str, dir: &'static str, status: Status,
    needs_volume: bool, needs_vix: bool, rules: &'static [&'static str], notes: &'static str, family: (&'static str, &'static str, &'static str) }

const NO_MM: &str = "Book's money-management stop (\"an appropriate dollar amount\", amount never given) is not modelled.";

fn defs() -> Vec<Def> {
    use Status::*;
    let d = |id, name, group, dir, status, rules, notes, family| Def { id, name, group, dir, status, needs_volume: false, needs_vix: false, rules, notes, family };
    vec![
    // ================= Volume 1 (1997)
    d("stad1_maco", "Moving Average Crossover (MACO)", "Vol 1 · Trending", "Long/Short", Live,
      &["F|FastMA = Average(Close,9); SlowMA = Average(Close,18) (Vol 1 p.15 code)",
        "E|When FastMA crosses over SlowMA: LEntryPrice = Highest(High,ChLen=12) * 1.03; buy stop at LEntryPrice next bar, re-issued each bar while BarNumber < cross bar + 12 and not long (code; prose rule a says 8 bars - code wins)",
        "E|When FastMA crosses under SlowMA: SEntryPrice = Lowest(Low,12) * .97; sell stop, active 12 bars (reverses a long)",
        "X|Long: ExitLong next bar at Lowest(Low,TrailBar=8) stop; short: ExitShort at Highest(High,8) stop",
        "A|Re-entry: after a long is exited (flat, previous position long) buy 100 shares (vs 300 initial) at Highest(High,10) stop, for up to 15 bars; mirror for shorts at Lowest(Low,10)"],
      "Vol 1 ch.2 p.12-21. Initial 300 shares, re-entry 100 shares -> re-entry legs weigh 1/3. The p.20 'Suggestions' [1]-offset variant is not the default. Book printed only a report figure (46% profitable). ",
      ("stad_maco", "Moving Average Crossover", "Vol 1")),
    d("stad1_macdbo", "MACD Breakout", "Vol 1 · Trending", "Long/Short", Live,
      &["F|MDif = MACD(Close,12,26) - XAverage(MACD,9); store bar/high/low of every bar where MDif crosses over or under 0 (p.23 code)",
        "E|If the 4th most recent cross is fewer than NBars=50 bars ago: buy stop at the highest high of the last 4 cross bars + Average(TrueRange,4)*.5, sell stop at their lowest low - Average(TrueRange,4)*.5 (code; inputs table prints MacdMA 12 / NBars 15 - code wins)",
        "X|ExitLong next bar at Lowest(Low,6) stop; ExitShort at Highest(High,6) stop"],
      "Vol 1 ch.2 p.21-28. Designed for stocks. Report is a figure only (about 48% profitable).", ("stad_macdbo", "MACD Breakout", "Vol 1")),
    d("stad1_tla", "Trend Line Automatic", "Vol 1 · Trending", "Long/Short", Live,
      &["F|Swing high: high >= the 4 prior highs and > the 4 following highs (RHStren 4); swing low mirror (RLStren 4 in the code; inputs table prints 15 - code wins). Keep the last 10 of each (p.30-31 code)",
        "F|On a new swing high, if an earlier stored swing high is higher, draw a down-trend line from the most recent such higher swing to the new one (up-trend line from swing lows likewise)",
        "E|Buy next bar at market if Close crosses over the down-trend line value and AbsValue(line value - prior-bar line value) > 2 points (Vol 2 Appendix A p.81 correction of the Vol 1 code)",
        "E|Sell next bar at market if Close crosses under the up-trend line and AbsValue(line change per bar) > 2",
        "F|Labelled option slope_mode=1: the original Vol 1 p.31 test Arctangent(prior - current) > 10, which Vol 2 p.80-81 says does not work",
        "X|ExitLong next bar at Lowest(Low,6) stop; ExitShort at Highest(High,6) stop"],
      "Vol 1 ch.2 p.28-35 + Vol 2 p.80-81 errata. Line value = linear extension by bar count (TL_GetValue). The 2-point slope threshold is in price points (scale dependent, as printed). Book report: net $80,905, 40% profitable, win/loss ratio 3.63 (symbol not printed).",
      ("stad_tla", "Trend Line Automatic", "Vol 1")),
    d("stad1_tf", "Trend Follower", "Vol 1 · Trending", "Long/Short", Live,
      &["F|BarNumber 1: TopBand = Open + Volatility(8)*2, BotBand = Open - Volatility(8)*2 (p.38 code)",
        "F|Bull = Average(C,4) > Average(C,9) > Average(C,18); Bear = reverse",
        "E|If XAverage(Close,4) > TopBand and Bull: buy this bar on close; BotBand = TopBand; TopBand += Volatility(8)*2",
        "E|If XAverage(Close,4) < BotBand and Bear: sell this bar on close; TopBand = BotBand; BotBand -= Volatility(8)*2",
        "X|Long: ExitLong next bar at BotBand stop; short: ExitShort next bar at TopBand stop"],
      "Vol 1 ch.2 p.35-42. Volatility = TradeStation Volatility function (Wilder-smoothed true range). Bands start on BarNumber 1 = first bar after MaxBarsBack (18). Book report is a figure (54% losing trades).",
      ("stad_tf", "Trend Follower", "Vol 1")),
    d("stad1_spread", "Spread (stock vs index % change)", "Vol 1 · Support/Resistance", "Long/Short",
      NeedsData("Needs a second aligned data stream (Data2 = the related index, e.g. DJIA, for each traded stock) and the share count behind the $100 'LowProf' stale-trade threshold; the engine runs one symbol per strategy."),
      &["F|Spread = (PctChange(Data1,10) / PctChange(Data2,10)) * 100 (p.46 code)", "E|Spread < 92: buy this bar on close; Spread > 108: sell this bar on close",
        "X|If PositionProfit > 0: trail Lowest(Low,6) / Highest(High,6) stop", "X|BarsSinceEntry > 6 and PositionProfit < $100: exit this bar on close"],
      "Vol 1 ch.3 p.44-50.", ("stad_spread", "Spread", "Vol 1")),
    d("stad1_sr", "Support and Resistance Levels", "Vol 1 · Support/Resistance", "Long/Short", Live,
      &["F|Swing high/low strength 5 (>= 5 bars before, > 5 bars after); keep the last 3; SwingHAvg / SwingLAvg = their averages; HighestHi / LowestLo of the stored swings (p.52-53 code; arrays start at 0)",
        "F|Condition1: SwingHAvg - SwingLAvg > Average(TrueRange,10)*2; Condition2: (SwingHAvg-LowestLo)/(HighestHi-LowestLo) > .66 and (SwingLAvg-LowestLo)/(..) < .33; Condition3: SwingLAvg < Close < SwingHAvg",
        "E|All true and XAverage(High,4) crosses under SwingHAvg: sell next bar at market (code; prose rule h differs - code wins)",
        "E|All true and XAverage(Low,4) crosses over SwingLAvg: buy next bar at market",
        "X|If PositionProfit > 0: ExitLong at Lowest(Low,TrailBar=5) stop, ExitShort at Highest(High,5) stop (code input 5; prose says 8 - code wins)"],
      "Vol 1 ch.3 p.49-58. 100 shares each side. Report is a figure (over 50% profitable, few trades).",
      ("stad_sr", "Support and Resistance Levels", "Vol 1")),
    d("stad1_pivot", "Pivot Points", "Vol 1 · Volatility breakout", "Long/Short", Live,
      &["F|Pivot = (L+H+C)/3; R2 = Pivot + Range; R3 = 2*Pivot - L + Range; S2 = Pivot - Range; S3 = 2*Pivot - H - Range (p.62 code)",
        "E|Bull (Avg4 > Avg9 > Avg18): buy stops at R2 and R3 next bar; Bear: sell stops at S2 and S3",
        "X|Exit at the first profitable open: next open > EntryPrice + commission (0 here) -> exit at that open; short mirror",
        "X|ExitLong next bar at Lowest(Low,4) stop; ExitShort at Highest(High,4) stop (issued every bar)"],
      "Vol 1 ch.4 p.60-65. Pivot uses (L+H+C)/3 in the rules and code (prose p.60 says Open+High+Low - code wins). The book's sample report is on a 30-minute S&P chart; the code has no time-of-day logic, so it runs unchanged on daily bars (timeframe differs from the book's example).", ("stad_pivot", "Pivot Points", "Vol 1")),
    d("stad1_pivot_id", "Pivot Points Intra-day", "Vol 1 · Volatility breakout", "Long/Short",
      NeedsData("Intraday system: pivots from the first intraday bar of the session; book test on 60-minute S&P (p.69). No intraday bars in the vault."),
      &["F|Pivots computed on the first bar of each day", "E|Within the session: buy stops at R2 and R3, sell stops at S2 and S3", "X|Lowest(Low,4) / Highest(High,4) trailing stops"],
      "Vol 1 ch.4 p.65-70.", ("stad_pivot", "Pivot Points", "Vol 1 intraday")),
    d("stad1_chbo", "Channel Breakout (2 breakouts)", "Vol 1 · Volatility breakout", "Long/Short", Live,
      &["F|HighChannel = Highest(High,10)[1]; LowChannel = Lowest(Low,10)[1] (p.72 code)",
        "E|High > HighChannel: breakout count +1, break-under count = 0; Low < LowChannel: break-under +1, breakout = 0 (in that order)",
        "E|Breakout count >= 2: buy this bar on close (count reset to 1); break-under count >= 2: sell this bar on close",
        "X|ExitLong next bar at Lowest(Low,6) stop; ExitShort at Highest(High,6) stop"],
      "Vol 1 ch.4 p.70-76. Prose mentions 14 bars, rules and code use 10 - code wins.", ("stad_chbo", "Channel Breakout", "Vol 1")),
    d("stad1_volex", "Volatility Expansion (VolEx)", "Vol 1 · Volatility breakout", "Long/Short", Live,
      &["E|Buy next bar at Open(next bar) + Average(Range,4)*1.2 stop; sell next bar at Open(next bar) - Average(Range,4)*1.8 stop (p.78 code)",
        "X|First profitable open: if long and next open > EntryPrice + ProfitPt(0) (+ commission, 0 here) exit at that open; short mirror",
        "X|ExitLong next bar at Lowest(Low,4) stop; ExitShort at Highest(High,4) stop",
        "X|BarsSinceEntry > 4 and PositionProfit < $500: exit next bar at market. Book: one S&P futures contract ($500 per point, 1997) -> 1.0 index point (param stale_pts)"],
      "Vol 1 ch.4 p.76-82. Book: one year of daily S&P, 30 trades, ~77% profitable, net $15,100, win/loss ratio .44, drawdown $27,000 (with commission/slippage, year not printed). The $1,800 stop of Fig 4-11 is a later variation, not default.",
      ("stad_volex", "Volatility Expansion", "Vol 1")),
    // ================= Volume 2 (1998)
    d("stad2_adxdiff", "ADX Differential", "Vol 2 · Trending", "Long/Short", Live,
      &["F|DeltaADX = ADX(14) - ADX(14)[1]; DeltaAvg = WAverage(DeltaADX,4) (code p.15; prose says exponential - code wins)",
        "E|If (DeltaAvg < 0 or DeltaAvg crosses over 0) and BarNumber > 1: UpCh = Close + Average(Range,4)/2, DnCh = Close - Average(Range,4)/2; orders active while BarNumber < that bar + CH=8 (prose says 10 - code wins) and flat",
        "E|Buy next bar at UpCh stop; Sell next bar at DnCh stop",
        "X|While those orders are active: if not long ExitLong next bar at Lowest(Low,6) stop; if not short ExitShort at Highest(High,6) stop (entry-bar protection; prose says 8 bars - code wins)",
        "X|First bar long: StopPrice = Low - Average(Range,4) (prose divides by 4 - code does not); each bar ExitLong at StopPrice stop, then StopPrice += (Low - StopPrice)/3; short mirror"],
      "Vol 2 ch.2 p.12-19. Book example: daily Qualcomm; report is a figure (under 50% winners, win/loss ~2).", ("stad_adxdiff", "ADX Differential", "Vol 2")),
    d("stad2_hs", "Head and Shoulders", "Vol 2 · Trending", "Long/Short", Live,
      &["F|Top: swing highs and lows of strength 5; last 3 of each kept with bar numbers (code p.22-24)",
        "F|Condition1: alternating order SH0 > SL0 > SH1 > SL1 > SH2 (bar numbers); Condition2: head SH1 above both shoulders SH2, SH0; Condition3: SL0 < SH2 and SL1 < SH0; Condition4: |SL0 - SL1| / (SH1 - min(SL0,SL1)) < .1 (true if the denominator is 0); each swing high used once",
        "E|Neckline = trend line through SL1 and SL0; if Close crosses under the neckline: sell next bar at open (once per neckline)",
        "E|Bottom: the book says the H&S Bottom code 'exactly mirrors' the Top code and does not print it; mirrored here (buy when Close crosses over the neckline of the swing highs)",
        "X|First bar short: StopPrice = High + Average(Range,4); each bar ExitShort at StopPrice stop, then StopPrice -= .3*(StopPrice - High); long mirror"],
      "Vol 2 ch.2 p.19-29. Uses the corrected main-system inputs (p.22 note). Book report: 12 trades, 5 winners, win/loss ratio 3.2 (symbol not printed).", ("stad_hs", "Head and Shoulders", "Vol 2")),
    Def { needs_volume: true, ..d("stad2_obv", "On Balance Volume New Highs/New Lows", "Vol 2 · Trending", "Long", Live,
      &["F|OBVNewHigh = OBV >= Highest(OBV,15); OBVNewLow = OBV <= Lowest(OBV,15) (code p.31)",
        "E|5 OBV new highs within the last 6 bars: buy 100 shares next bar at market; 3 new lows within 6 bars: buy 200 shares next bar at market",
        "X|First bar long: StopPrice = Low - Average(Range,4); each bar ExitLong at StopPrice stop, then StopPrice += (Low - StopPrice)/3"],
      "Vol 2 ch.2 p.29-35. Needs volume (SPX/DJI yes; NSE index files have none). The 200-share entry weighs 2x.", ("stad_obv", "OBV New Highs/Lows", "Vol 2")) },
    Def { needs_volume: true, ..d("stad2_addiv", "Advance Decline Divergence (volume move)", "Vol 2 · Support/Resistance", "Long/Short", Live,
      &["F|VolumeMove = XAverage((Close - Open)/(High - Low) * Volume, 9) (code p.39; 0 when High = Low)",
        "E|Low < Lowest(Low,30)[1] and VolumeMove made no new 30-bar low in the last 5 bars: buy 100 next bar at market; Low < Lowest(Low,30)[1] and VolumeMove > Highest(VolumeMove,30)[1]: buy 200",
        "E|High > Highest(High,30)[1] and no new 30-bar VolumeMove high in 5 bars: sell 100; High > Highest(High,30)[1] and VolumeMove < Lowest(VolumeMove,30)[1]: sell 200",
        "X|StopPrice trail: first bar Low - Average(Range,4) / High + Average(Range,4), then 1/3 of the gap each bar"],
      "Vol 2 ch.3 p.36-42. Despite the name it uses the symbol's own volume, not exchange breadth. Book example: 3 months of 10-minute bars (136 trades). Needs volume.", ("stad_addiv", "A/D (volume) Divergence", "Vol 2")) },
    d("stad2_stoch", "Stochastics", "Vol 2 · Support/Resistance", "Long/Short", Live,
      &["E|SlowK(14) >= 75 on each of the last 3 bars and not short: buy 200 next bar at market (Mode = 1) (code p.45)",
        "E|SlowK(14) <= 25 on each of the last 3 bars and not long: sell 200 next bar at market (Mode = -1)",
        "E|Long with Mode 1: first bar ReversePrice = Low - Average(Range,4); Sell ('ReverseLong') 400 at ReversePrice stop, then ReversePrice += (Low - ReversePrice)/3; short mirror ('ReverseShort' buy 400)",
        "X|ExitShort from entry 'ReverseLong' at Highest(High,5) stop; ExitLong from entry 'ReverseShort' at Lowest(Low,5) stop"],
      "Vol 2 ch.3 p.42-48. SlowK = TradeStation legacy SlowK (FastK smoothed (2*prev+new)/3). 400-share reversals weigh 2x.", ("stad_stoch", "Stochastics", "Vol 2")),
    d("stad2_ccibb", "CCI Bollinger Bands OB/OS", "Vol 2 · Support/Resistance", "Long/Short", Live,
      &["F|Bands = BollingerBand(Close,9,±2); CCIAvg = XAverage(CCI(14),2); Position = (Close - Bot)/(Top - Bot) (code p.51)",
        "F|BBStatus: Position crosses .5 -> 0; Low crosses under Bot -> -1; High crosses over Top -> 1",
        "E|BBStatus -1 and CCIAvg swing low (strength 1) one bar ago below -100: LEP = Highest(High,3); buy stop at LEP for the next 5 bars while not long",
        "E|BBStatus 1 and CCIAvg swing high (strength 2) two bars ago above 100: SEP = Lowest(Low,3); sell stop for 5 bars while not short (the code resets only LongCount when in a position - kept)",
        "X|StopPrice trail: first bar Low - Average(Range,4)*2/3 (High + .. short), then .3 of the gap each bar"],
      "Vol 2 ch.3 p.48-56. Book: about 160 trades in 12 years of one stock, 46% winners, win/loss 2.5.", ("stad_ccibb", "CCI Bollinger", "Vol 2")),
    d("stad2_pctchan", "Percent Change Channel", "Vol 2 · Volatility", "Long/Short", Live,
      &["F|PcntChange = Close / Close[5] * 100 (code p.58)",
        "E|PcntChange < 98: OS = true, HighBand = Highest(High,3); buy next bar at HighBand stop while OS and not long",
        "E|PcntChange > 102: OB = true, LowBand = Lowest(Low,3); sell next bar at LowBand stop while OB and not short; OS/OB reset after being true 2 bars",
        "X|First profitable open: long and next open > EntryPrice -> exit at that open; short mirror",
        "X|ExitLong at Lowest(Low,5) stop; ExitShort at Highest(High,5) stop"],
      "Vol 2 ch.4 p.57-62. Book report: 68% profitable, win/loss .77.", ("stad_pctchan", "Percent Change Channel", "Vol 2")),
    d("stad2_bullbeater", "BullBeater", "Vol 2 · Volatility", "Long/Short",
      NeedsData("Intraday system: trades only between 9:30 and 16:30 (Time inputs); needs intraday bars."),
      &["F|Bollinger(9,2) width < 1.5*Average(Range,4) arms orders for 5 bars", "E|RSI(4) < 45: buy stop Highest(High,5); RSI(4) > 55: sell stop Lowest(Low,5)", "X|StopPrice trail (1/3)"],
      "Vol 2 ch.4 p.62-67.", ("stad_bullbeater", "BullBeater", "Vol 2")),
    d("stad2_opengap", "Opening Gap", "Vol 2 · Volatility", "Long/Short",
      NeedsData("Intraday Data1 with daily Data2 of the same symbol; first-bar-of-day logic needs intraday bars."),
      &["F|Trend = XAverage(C,3) vs XAverage(C,6) of daily Data2", "E|Bull trend and first intraday close below yesterday's low: buy; bear and above yesterday's high: sell; then stop entries at the prior bar's high/low",
        "X|StopPrice trail (1/3)"], "Vol 2 ch.4 p.67-75.", ("stad_opengap", "Opening Gap", "Vol 2")),
    d("stad2_stats2", "Stats #2: buy the first day of the week", "Vol 2 · Market outlook", "Long", Live,
      &["E|Buy at the open of the first trading day of the week (p.75)", "X|Exit at the close of the same day"],
      "Vol 2 p.75-76. Code not printed; the rule is stated in full. Book (daily DJIA, last 10 years to 1998): over 65% of Mondays up, average winner better than 27 points, average loser a little over 24 points.",
      ("stad_stats", "Day-of-week statistics", "Stats #2 first day of week")),
    d("stad2_stats3", "Stats #3: short Friday the 13th", "Vol 2 · Market outlook", "Short", Live,
      &["E|Sell short at the open of every Friday the 13th (p.76)", "X|Exit at the close of the same day"],
      "Vol 2 p.76-77. Book (DJIA): a little under 50% profitable, average win nearly twice the average loss.",
      ("stad_stats", "Day-of-week statistics", "Stats #3 Friday 13th")),
    d("stad2_stats1", "Stats #1: Friday breakout held to Tuesday's open", "Vol 2 · Market outlook", "Long",
      NotTestable("Code not printed; 'buying on Friday at the highest High of the week' does not fix the order level (week-to-date high incl./excl. Friday, stop vs limit)."),
      &["E|Buy on Friday at the highest high of the week", "X|Exit Tuesday at the open"], "Vol 2 p.77.", ("stad_stats", "Day-of-week statistics", "Stats #1")),
    // ================= Volume 3 (1998)
    d("stad3_bdsr", "Buy Dips / Sell Rallies", "Vol 3 · Trending", "Long/Short", Live,
      &["F|UpTrend = RSI(Close,20) >= 50; DnTrend = RSI < 50; DMISpread = DMIPlus(5) - DMIMinus(5) (code p.16)",
        "E|UpTrend and DMISpread < 0 -> BuySetup (SellSetup off); on the first setup bar LongEntryPrice = High; buy next bar at it (stop) while BuySetup",
        "E|DnTrend and DMISpread > 0 -> SellSetup; ShortEntryPrice = Low of its first bar; sell stop",
        "X|StopPrice trail: first bar Low - Average(Range,4) / High + .., then 1/3 of the gap each bar"],
      "Vol 3 ch.2 p.14-22. Book example: weekly Intel (daily bars here).", ("stad_bdsr", "Buy Dips / Sell Rallies", "Vol 3")),
    d("stad3_engulf", "Bullish & Bearish Engulfing", "Vol 3 · Trending", "Long/Short", Live,
      &["F|UpTrend: XAverage(C,10) rising; DownTrend: falling; AvgBody = Average(|C-O|,10) (code p.24)",
        "F|BullEngulfing: up bar after down bar, Close > Open[1], Open < Close[1], body > 1.25*AvgBody, prior body < .75*AvgBody; bearish mirror",
        "E|DownTrend and BullEngulfing within the last 3 bars and not long: buy stop at that bar's high; short mirror at the low",
        "X|Long: ExitLong at highest close since entry - Average(Range,4)/2 stop; short: lowest close + Average(Range,4)/2"],
      "Vol 3 ch.2 p.22-29. Book: 18-year daily Intel, 12 trades, better than 55% winners.", ("stad_engulf", "Engulfing patterns", "Vol 3")),
    d("stad3_ochist", "Open-Close Histogram (Vol 3)", "Vol 3 · Trending", "Long/Short", Live,
      &["F|Histogram = XAverage(Close,10) - XAverage(Open,10) (code p.30)",
        "E|Crosses over 0: BuyPrice = High + Average(TrueRange,10)*.5, LongExitPrice = Low[1] - Average(TrueRange,10)[1]/2; buy stop ('LE') while the cross is within 10 bars and Histogram > 0",
        "E|Crosses under 0: SellPrice = Low - ATR*.5, ShortExitPrice = High[1] + ATR[1]/2; sell stop ('SE') while within 10 bars and Histogram < 0",
        "X|Histogram crosses under 0: ExitLong this bar on close; ExitLong from entry 'LE' next bar at LongExitPrice stop (short mirror)"],
      "Vol 3 ch.2 p.29-34. Book: weekly IBM ~18 years, ~40% winners, ~70 points. Revisited in Vol 13 (separate entry).", ("stad_ochist", "Open-Close Histogram", "Vol 3")),
    d("stad3_bb", "Bollinger Bands S/R", "Vol 3 · Support/Resistance", "Long/Short", Live,
      &["F|HiBand/LoBand = BollingerBand(Close,10,±2) (code p.38)",
        "E|Close > HiBand on each of the last 2 bars and not short: sell next bar at open; Close < LoBand 2 bars and not long: buy next bar at open",
        "X|StopPrice trail: first bar Low - Average(Range,4) / High + .., then 1/3 of the gap each bar",
        "F|The code's same-side limit adds ('BLim'/'SLim' while in a position) never fill without pyramiding - omitted"],
      "Vol 3 ch.3 p.36-43. Book example: 5-minute S&P (daily bars here); the $300 money-management stop of Fig 5 is a later variation.", ("stad_bb", "Bollinger Bands S/R", "Vol 3")),
    d("stad3_obtr", "On Balance True Range", "Vol 3 · Support/Resistance", "Long",
      NotTestable("The OBTR function code is not printed and the book's two descriptions disagree on its sign (sidebar p.44: add true range on down closes; prose: 'just like OBV', declines on down closes)."),
      &["F|OBTR < XAverage(OBTR,9) gates the Bollinger long side", "E|2 closes under the lower band: buy next open", "X|StopPrice trail"], "Vol 3 ch.3 p.43-47.", ("stad_bb", "Bollinger Bands S/R", "OBTR filter")),
    Def { needs_volume: true, ..d("stad3_mmmw", "Micro-M & Micro-W Formations", "Vol 3 · Support/Resistance", "Long/Short", Live,
      &["F|Price swing lows/highs (strength 4) and oscillator swings (AccumDist(Volume), strength 4; the combined system passes AccumDist to both halves) keep 10 each (code p.49-52)",
        "F|Found: on a new swing, if the newest price and oscillator swings are within 5 bars, search older pairs within 5 bars whose price and oscillator moved in opposite directions (either way); newest price swing used once",
        "E|W: Found within the last 5 bars and an up-close/down-close pair 1-2 bars ago: buy next bar at Highest(High,4) stop (code; prose says buy on a fall below the pattern low - code wins)",
        "E|M: Found within 5 bars and a down/up pair 1-2 bars ago: sell next bar at Lowest(Low,4) stop",
        "X|Long: the printed code resets StopPrice = Low - Average(Range,4) every bar ('MP[1] <> -1') - kept literally; short: StopPrice trail High + Average(Range,4), then 1/3 of the gap"],
      "Vol 3 ch.3 p.47-58. Needs volume.", ("stad_mmmw", "Micro-M & Micro-W", "Vol 3")) },
    d("stad3_varzones", "Variable Zones", "Vol 3 · Support/Resistance", "Long/Short", Live,
      &["F|Oscillator RSI(Close,20); swing highs/lows of the oscillator, strength 5; last 4 of each (p.61)",
        "F|TopBand = avg(swing highs) + .75*(highest swing high - avg); BotBand = avg(swing lows) - .75*(avg - lowest swing low) (setup a/b; function code not printed); bands start once 4 swings exist",
        "E|RSI crosses over BotBand: buy this bar on close; crosses under TopBand: sell this bar on close",
        "X|StopPrice trail (1/3)"],
      "Vol 3 ch.3 p.58-65. Book examples: 5-minute S&P and daily Intel.", ("stad_varzones", "Variable Zones", "Vol 3")),
    d("stad3_idbf", "Inside Day Breakout Failure", "Vol 3 · Volatility", "Long/Short",
      NeedsData("30-minute S&P bars with a daily Data2 (inside day on the daily chart, entries intraday)."),
      &["F|Inside day on the daily chart", "E|Intraday: fade the first penetration of the inside day's high/low", "X|Trailing stop at the opposite extreme +/- half the 3-day average range; exit at end of day"],
      "Vol 3 ch.4 p.66-72.", ("stad_idbf", "Inside Day Breakout Failure", "Vol 3")),
    d("stad3_sdv", "Standard Deviation - Volatility", "Vol 3 · Volatility", "Long/Short", Live,
      &["F|BandRange = 2*2*StdDev(Close,9) (top - bottom band of Average(Close,9) ± 2 StdDev) (code p.74)",
        "E|Buy next bar at Close*(1 + BandRange/Average(Close,4)) stop; sell next bar at Close*(1 - BandRange/Average(Close,4)) stop",
        "X|First profitable open (ProfitPt 0)"],
      "Vol 3 ch.4 p.72-77. StdDev = population (TradeStation StdDev, Vol 4 p.89).", ("stad_sdv", "Std Dev - Volatility", "Vol 3")),
    d("stad3_volclus", "Volatility Clusters (Vol 3)", "Vol 3 · Volatility", "Long/Short", Live,
      &["F||C - C[1]| > (Average + 2 StdDev of |C - C[1]| over 18)[1] or Range > (Average + 2 StdDev of Range over 18)[1] (code p.78)",
        "E|Then buy next bar at Close + Average(Range,3)[1] stop and sell next bar at Close - Average(Range,3)[1] stop",
        "X|First profitable open (ProfitPt 0)"],
      "Vol 3 ch.4 p.77-81. Revisited in Vol 13.", ("stad_volclus", "Volatility Clusters", "Vol 3")),
    // ================= Volume 4 (1998)
    d("stad4_dmiadx", "Directional Movement Index (DMI-ADX)", "Vol 4 · Trending", "Long/Short", Live,
      &["F|DMI+(7), DMI-(7), ADX(7) (code p.14 with the Vol 5 p.94 correction SellSetup = False when short)",
        "E|DMI+ > DMI- and ADX crosses over DMI- and not long: BuyPrice = High + 1 point, ExitLPrc = Low - 1 point; setup lasts until filled or ADX < DMI-; buy stop at BuyPrice",
        "E|DMI+ < DMI- and ADX crosses over DMI+ and not short: SellPrice = Low - 1 point, ExitSPrc = High + 1 point; sell stop",
        "X|In position: ExitLong at ExitLPrc stop / ExitShort at ExitSPrc stop",
        "X|ADX[1] >= 30 and ADX < ADX[1] and BarsSinceEntry > 1: exit this bar on close; DMI+ < DMI- and BarsSinceEntry > 1: ExitLong on close (DMI+ > DMI-: ExitShort)"],
      "Vol 4 ch.2 p.12-19. 1 point = minimum move (param point, 0.01). NumUnits($10,000 / lot 100) sizing becomes equal weight. Book example: daily Alcoa.", ("stad_dmiadx", "DMI-ADX", "Vol 4")),
    d("stad4_keltner", "Keltner Channel", "Vol 4 · Trending", "Long/Short", Live,
      &["F|KCU/KCL = Average(Close,10) ± 1.2*Average(TrueRange,10); ChanRng = (KCU - KCL)/2 (code p.21)",
        "E|Close crosses above KCU: SetBar = High, count 1; while Close > KCU and count <= 5: buy next bar at SetBar + .5*ChanRng stop",
        "E|Close crosses below KCL: SetBar = Low; while Close < KCL and count <= 5: sell at SetBar - .5*ChanRng stop (SetBar is shared by both sides, as printed)",
        "X|Close crosses below the average: ExitLong this bar on close (short mirror); ExitLong next bar at Lowest(Low,4) stop, ExitShort at Highest(High,4) stop"],
      "Vol 4 ch.2 p.19-26. Book example: weekly American Express, 36 trades.", ("stad_keltner", "Keltner Channel", "Vol 4")),
    d("stad4_linreg", "Linear Regression & Momentum (Vol 4)", "Vol 4 · Trending", "Long/Short", Live,
      &["F|XLinReg = XAverage(LinearRegValue(Close,20,0),15) (inputs table prints 1 - code wins); Mom = Close - Close[10] (code p.28)",
        "E|XLinReg rising and Mom < 0 and rising: SetBarL = High when count is 1 or > 4; for up to 4 bars buy ('Long') next bar at SetBarL + .5*(SetBarL - XLinReg) stop",
        "E|XLinReg falling and Mom > 0 and falling: SetBarS = Low; sell ('Short') at SetBarS - .5*(SetBarS - XLinReg) stop",
        "X|ExitLong from entry 'Long' at$ Low - .5*AvgTrueRange(10) stop (at$ = the order bar's values, active from the entry bar); short mirror",
        "X|XLinReg falling and Low < Lowest(Low,4)[1]: ExitLong next bar at market; rising and High > Highest(High,4)[1]: ExitShort"],
      "Vol 4 ch.2 p.26-31. Book example: daily AT&T. Revisited in Vol 13.", ("stad_linreg", "Linear Regression & Momentum", "Vol 4")),
    d("stad4_hpi", "Herrick Payoff Index & Channel Breakout on Close", "Vol 4 · Trending", "Long/Short",
      NeedsData("The Herrick Payoff Index needs futures open interest; no open-interest series in the vault."),
      &["F|Close > 10-bar high and HPI > 10-bar HPI high: buy setup (mirror for sell)", "E|Buy at setup high + 1 point within 5 days", "X|4-day low/high trailing stop; HPI crossing zero exits on the close"],
      "Vol 4 ch.2 p.31-38. Book: daily Coffee.", ("stad_hpi", "HPI breakout", "Vol 4")),
    d("stad4_cc", "Consecutive Close", "Vol 4 · Trending", "Long/Short", Live,
      &["E|No down close among the 3 bars before today and today closes down: buy next bar at Open(next) + .25*Average(TrueRange,8) stop; HH = Highest(High,4) (code p.42)",
        "E|No up close among the 3 bars before today and today closes up: sell next bar at Open(next) - .25*Average(TrueRange,8) stop; LL = Lowest(Low,4)",
        "X|Long: count next opens above HH; on the 2nd exit at that next open. Short: count closes below LL; on the 2nd exit next bar at open",
        "X|ExitLong next bar at Lowest(Low,10) stop; ExitShort at Highest(High,10) stop"],
      "Vol 4 ch.2 p.38-46. Book: daily Coca-Cola 1970-1998 (65% winners) and CRB futures.", ("stad_cc", "Consecutive Close", "Vol 4")),
    d("stad4_momret", "Momentum-Retracement (long side)", "Vol 4 · Trending", "Long", Live,
      &["F|PriceXMA = XAverage(C,5); HighXMA/LowXMA = XAverage(H/L,15); MACDSigLine = XAverage(MACD(C,3,10),15); MACDDiff = MACD - signal; DMISpread = DMI+(15) - DMI-(15) (code p.51)",
        "F|TrendUp and Momentum (PriceXMA > HighXMA, signal > 0, RSI(5) >= 70, DMISpread > 15): phase 1, track HighestHi and HighestRSI",
        "F|Retracement: phase 1 and PriceXMA < HighXMA and MACDDiff < 0 and (RSI(5) <= 50 or RSI(5) < HighestRSI - 30): phase 2, track LowestLo",
        "E|In phase 2 and not long: buy next bar at LowestLo + (HighestHi - LowestLo)/3 stop",
        "X|Vol 5 p.95 correction: Condition1 = PriceXMA < LowXMA or signal < 0; if long or Condition1 reset the phases; if Condition1 ExitLong next bar at market",
        "X|Not long: ExitLong next bar at Low - Average(TrueRange,10) stop (entry-bar stop); long: at EntryPrice - Average(TrueRange,10) stop",
        "X|The printed swing stop 'If SwingLow(1,Low,2,4) = 3 ... ExitPrc = MinList(LowXMA, ExitPrc)' compares a price to 3 and keeps ExitPrc at 0 - kept literally (never fills)"],
      "Vol 4 ch.2 p.46-56 + Vol 5 p.95 errata. Book example: weekly General Electric. Short side: see stad4_momret_short. Revisited in Vol 13.", ("stad_momret", "Momentum-Retracement", "Vol 4 long")),
    d("stad4_momret_short", "Momentum-Retracement (short side)", "Vol 4 · Trending", "Short",
      NotTestable("Short-side code is not printed ('exactly mirrors' the long side), and the long side's literal swing-stop line (SwingLow(...) = 3, MinList with ExitPrc 0) has no unambiguous mirror."),
      &["E|Mirror of the long side", "X|Mirror exits"], "Vol 4 p.49-56.", ("stad_momret", "Momentum-Retracement", "Vol 4 short")),
    d("stad4_3xa", "Three Exponential Moving Averages", "Vol 4 · Trending", "Long/Short", Live,
      &["F|Avg1/2/3 = XAverage(Close,6/12/28) (code p.58)",
        "E|Avg1 crosses over Avg2 and Avg2 > Avg3: buy next bar at open; Avg1 crosses under Avg2 and Avg2 < Avg3: sell next bar at open",
        "X|Avg1 < Avg2: ExitLong next bar at open; Avg1 > Avg2: ExitShort next bar at open",
        "X|Long StopPrice: first bar Low - Average(Range,4), then += (Low - StopPrice)/4; short: High + Average(Range,4), then -= (StopPrice - High)/3 (as printed, incl. the bold short block)"],
      "Vol 4 ch.2 p.57-62.", ("stad_3xa", "Three XAverages", "Vol 4")),
    d("stad4_spread_up", "Spread System Beta Up", "Vol 4 · Support/Resistance", "Long",
      NeedsData("Needs a second aligned data stream (Data2) for each traded symbol; the engine runs one symbol per strategy."),
      &["E|Close/Close[10] of Data1 < same of Data2 and XAverage(C,8) > XAverage(C,15): buy next open", "X|StopPrice trail (1/3)"], "Vol 4 ch.3 p.63-68.", ("stad_spread4", "Spread Beta", "Vol 4 up")),
    d("stad4_spread_dn", "Spread System Beta Down", "Vol 4 · Support/Resistance", "Short",
      NeedsData("Needs a second aligned data stream (Data2) for each traded symbol."),
      &["E|Mirror of Beta Up for the short side", "X|StopPrice trail"], "Vol 4 ch.3 p.68-72.", ("stad_spread4", "Spread Beta", "Vol 4 down")),
    Def { needs_volume: true, ..d("stad4_dmacv", "Displaced Moving Average & Volume", "Vol 4 · Volatility", "Long/Short", Live,
      &["F|DMA = Average(Close,7)[5]; DMAV = Average(Volume,7)[5] (code p.77)",
        "E|Close crosses above DMA and Volume > DMAV: buy next bar at Open(next) + .2*AvgTrueRange(10) stop; crosses below and Volume < DMAV: sell at Open(next) - .2*ATR stop",
        "X|ExitLong next bar at Lowest(Low,5) stop; ExitShort at Highest(High,5) stop; Close crosses below DMA: ExitLong next bar at market (above: ExitShort)"],
      "Vol 4 ch.4 p.75-82. Needs volume.", ("stad_dmacv", "DMA & Volume", "Vol 4")) },
    d("stad4_pricedist", "Price Distribution (Kurtosis & Skew)", "Vol 4 · Volatility", "Long/Short", Live,
      &["F|Kurt = Kurtosis(Close,40); Skew = Skew(Close,15) (STAD functions p.88-89, sample std dev)",
        "E|Kurt crosses under 0: Skew > 0 buy this bar on close; Skew < 0 sell this bar on close (code p.84)",
        "X|StopPrice trail (1/3)"],
      "Vol 4 ch.4 p.82-90.", ("stad_pricedist", "Price Distribution", "Vol 4")),
    // ================= Volume 7 (1999)
    Def { needs_vix: true, ..d("stad7_vix", "S&P 500 Implied Volatility (VIX key reversals)", "Vol 7 · Volatility", "Long/Short", Live,
      &["F|Data2 = the CBOE volatility index (daily) (Vol 7 p.71 code)",
        "E|Bear key reversal in VIX: VIX High >= Highest(VIX High,1)[1] and VIX Close < VIX Close[1] -> buy next bar on open",
        "E|Bull key reversal in VIX: VIX Low <= Lowest(VIX Low,12)[1] and VIX Close > VIX Close[1] -> sell next bar on open",
        "X|Stop-and-reverse only: no stops, no other exits (p.72)"],
      "Vol 7 ch.4 p.70-74. Book: daily S&P 500 futures, 1 contract, 2/8/1994-1/8/1999, no costs: avg win $6,225, avg loss -$2,521 (ratio 2.5), largest win $47,362, largest loss -$18,050. The book's VIX was the original S&P 100-based index (now VXO); the vault has today's CBOE VIX for US tickers and India VIX (from 2015) for NSE indices (instrument swap). VIX bars are aligned by date; a day without a VIX bar gives no signal.",
      ("stad_vix", "S&P 500 Implied Volatility", "Vol 7")) },
    // ================= Volume 10 (1999)
    d("stad10_mwt", "JK Monday We Trade (Joe Krutsinger)", "Vol 10 · Stock indices", "Long/Short", Live,
      &["E|If DayOfWeek(Date) = Friday and Close > Average(Close,2): buy next bar at market (Monday's open, or Tuesday's if Monday is closed) (Vol 10 p.13 code; strategy input Length 2)",
        "E|If Friday and Close <= Average(Close,2): sell short next bar at market",
        "X|SetExitOnClose: exit at the close of the entry day",
        "X|SetStopLoss StopLoss_Amount $1,500 per position (strategy input): with one S&P 500 contract at $500/point = 3 index points from the entry, active intraday (params stop_usd, pt_usd; stop_usd = 0 turns it off, as in Joe Krutsinger's NASDAQ test)"],
      "Vol 10 ch.1 p.11-17. Book: NASDAQ 100 index 4/96-8/99 (no costs, no stop mentioned): $107,035 on 167 trades, 59% winners, avg win $1,827 / avg loss -$1,087, PF 2.45. S&P 500 index 9/95-8/99 with $1,500 stop and $50/contract costs: $68,282 on 194 trades, 44.85% winners, win/loss 1.88, avg trade $351.",
      ("stad_mwt", "Monday We Trade", "Vol 10")),
    ]
}

pub fn catalogue() -> Vec<StrategyDef> {
    defs().into_iter().map(|d| StrategyDef { id: d.id, name: d.name, group: d.group, session: 0, dir: d.dir, status: d.status,
        needs_volume: d.needs_volume, needs_vix: d.needs_vix, needs_series: &[], book: BOOK, rules: d.rules,
        notes: if d.notes.is_empty() { NO_MM } else { d.notes }, claim: None }).collect()
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    defs().into_iter().find(|d| d.id == id).map(|d| d.family)
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def: f64, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(match id {
        "stad1_maco" => vec![p("fast", "Fast MA", 9., 3., 30., 1.), p("slow", "Slow MA", 18., 10., 60., 1.), p("chlen", "Channel / order life bars", 12., 4., 30., 1.),
            p("pct", "Entry offset %", 3., 0., 6., 0.5), p("trail", "Trailing stop bars", 8., 3., 20., 1.), p("rebars", "Re-entry window bars", 15., 5., 30., 1.),
            p("relen", "Re-entry channel bars", 10., 5., 20., 1.), p("rew", "Re-entry size / initial size", 100. / 300., 0., 1., 1. / 3.)],
        "stad1_macdbo" => vec![p("fast", "MACD fast", 12., 5., 20., 1.), p("slow", "MACD slow", 26., 15., 40., 1.), p("sig", "MACD average", 9., 5., 15., 1.),
            p("ncos", "Crosses", 4., 2., 8., 1.), p("nbars", "Within bars", 50., 15., 100., 5.), p("trail", "Trailing stop bars", 6., 3., 15., 1.)],
        "stad1_tla" => vec![p("hstren", "Swing high strength", 4., 2., 8., 1.), p("lstren", "Swing low strength", 4., 2., 15., 1.),
            p("slope_mode", "Slope test: 0 = |change| > pts (Vol 2 errata), 1 = arctangent > deg (original)", 0., 0., 1., 1.),
            p("pts", "Minimum line change per bar (points)", 2., 0., 10., 0.5), p("deg", "Minimum slope (degrees, option 1)", 10., 0., 45., 5.), p("trail", "Trailing stop bars", 6., 3., 15., 1.)],
        "stad1_tf" => vec![p("xma", "XAverage length", 4., 2., 10., 1.), p("vol", "Volatility length", 8., 4., 20., 1.), p("mult", "Band factor", 2., 1., 4., 0.5)],
        "stad1_sr" => vec![p("strength", "Swing strength", 5., 2., 10., 1.), p("nswing", "Swings averaged", 3., 2., 6., 1.), p("trail", "Trailing stop bars", 5., 3., 15., 1.)],
        "stad1_chbo" => vec![p("len", "Channel bars", 10., 5., 30., 1.), p("brk", "Breakouts required", 2., 1., 4., 1.), p("trail", "Trailing stop bars", 6., 3., 15., 1.)],
        "stad1_volex" => vec![p("lmult", "Long range factor", 1.2, 0.5, 2.5, 0.1), p("smult", "Short range factor", 1.8, 0.5, 3., 0.1),
            p("profitpt", "First-profitable-open extra points", 0., 0., 2., 0.25), p("trail", "Trailing stop bars", 4., 2., 10., 1.),
            p("stalebars", "Stale exit after bars", 4., 2., 10., 1.), p("stale_pts", "Stale profit threshold (index points; $500 / $500 per S&P point)", 1., 0., 5., 0.5)],
        "stad1_pivot" => vec![p("trail", "Trailing stop bars", 4., 2., 10., 1.)],
        "stad2_adxdiff" => vec![p("adx", "ADX length", 14., 5., 30., 1.), p("avg", "WAverage length", 4., 2., 10., 1.), p("ch", "Order life bars", 8., 3., 15., 1.), p("prot", "Entry-bar stop bars", 6., 3., 12., 1.)],
        "stad2_hs" => vec![p("top", "Top strength", 5., 2., 10., 1.), p("bot", "Bottom strength", 5., 2., 10., 1.)],
        "stad2_obv" => vec![p("len", "New high/low lookback", 15., 5., 30., 1.), p("back", "Window bars", 6., 3., 12., 1.), p("nh", "New highs needed", 5., 1., 6., 1.), p("nl", "New lows needed", 3., 1., 6., 1.)],
        "stad2_addiv" => vec![p("len", "XAverage length", 9., 3., 20., 1.), p("look", "New high/low lookback", 30., 10., 60., 5.)],
        "stad2_stoch" => vec![p("len", "Stochastic length", 14., 5., 30., 1.), p("ob", "Overbought", 75., 60., 90., 5.), p("os", "Oversold", 25., 10., 40., 5.)],
        "stad2_ccibb" => vec![p("bblen", "Bollinger length", 9., 5., 20., 1.), p("bbsd", "Std devs", 2., 1., 3., 0.5), p("ccilen", "CCI length", 14., 5., 30., 1.), p("cciavg", "CCI XAverage", 2., 1., 5., 1.)],
        "stad2_pctchan" => vec![p("pchg", "Percent-change bars", 5., 2., 10., 1.), p("ob", "Overbought %", 102., 101., 106., 0.5), p("os", "Oversold %", 98., 94., 99., 0.5), p("bands", "Entry channel bars", 3., 1., 6., 1.)],
        "stad2_stats2" | "stad2_stats3" => vec![],
        "stad3_bdsr" => vec![p("rsilen", "RSI length", 20., 5., 40., 1.), p("rsilevel", "RSI level", 50., 40., 60., 5.), p("dmilen", "DMI length", 5., 3., 20., 1.)],
        "stad3_engulf" => vec![p("len", "XAverage length", 10., 5., 30., 1.), p("body", "Body average length", 10., 5., 30., 1.)],
        "stad3_ochist" => vec![p("olen", "Open XAverage", 10., 5., 30., 1.), p("clen", "Close XAverage", 10., 5., 30., 1.)],
        "stad3_bb" => vec![p("nov", "Closes above upper band", 2., 1., 4., 1.), p("nun", "Closes below lower band", 2., 1., 4., 1.)],
        "stad3_mmmw" => vec![p("str", "Price swing strength", 4., 2., 8., 1.), p("str2", "Oscillator swing strength", 4., 2., 8., 1.)],
        "stad3_varzones" => vec![p("rsilen", "RSI length", 20., 5., 40., 1.), p("swings", "Swings averaged", 4., 2., 8., 1.), p("strength", "Swing strength", 5., 2., 10., 1.), p("factor", "Factor", 0.75, 0.25, 1.5, 0.25)],
        "stad3_sdv" => vec![p("sd", "Std devs", 2., 1., 3., 0.5), p("len", "Length", 9., 5., 30., 1.), p("profitpt", "First-profitable-open extra points", 0., 0., 2., 0.25)],
        "stad3_volclus" => vec![p("sd", "Std devs", 2., 1., 3., 0.5), p("len", "Length", 18., 5., 40., 1.), p("profitpt", "First-profitable-open extra points", 0., 0., 2., 0.25)],
        "stad4_dmiadx" => vec![p("dmilen", "DMI length", 7., 5., 20., 1.), p("adxlen", "ADX length", 7., 5., 20., 1.), p("adxexit", "ADX exit level", 30., 20., 50., 5.), p("point", "1 point (minimum move)", 0.01, 0.01, 1., 0.01)],
        "stad4_keltner" => vec![p("len", "Length", 10., 5., 30., 1.), p("k", "ATR multiple", 1.2, 0.5, 3., 0.1), p("pct", "Channel % added", 0.5, 0.1, 1., 0.1), p("life", "Order life bars", 5., 2., 10., 1.)],
        "stad4_linreg" => vec![p("lrlen", "Regression length", 20., 10., 40., 1.), p("avglen", "XAverage of regression", 15., 1., 30., 1.), p("momlen", "Momentum length", 10., 5., 20., 1.), p("pct", "Percent", 0.5, 0.1, 1., 0.1)],
        "stad4_cc" => vec![p("nup", "Up closes", 3., 2., 6., 1.), p("ndn", "Down closes", 3., 2., 6., 1.), p("atrlen", "ATR length", 8., 4., 20., 1.), p("pct", "ATR fraction", 0.25, 0.1, 1., 0.05)],
        "stad4_momret" => vec![p("xma", "Price XAverage", 5., 3., 10., 1.), p("len2", "High/Low XAverage", 15., 5., 30., 1.)],
        "stad4_3xa" => vec![p("a1", "Fast", 6., 3., 15., 1.), p("a2", "Medium", 12., 6., 30., 1.), p("a3", "Slow", 28., 15., 60., 1.)],
        "stad4_dmacv" => vec![p("plen", "Price MA", 7., 3., 20., 1.), p("pdisp", "Price displacement", 5., 0., 10., 1.), p("vlen", "Volume MA", 7., 3., 20., 1.), p("vdisp", "Volume displacement", 5., 0., 10., 1.), p("atrpct", "ATR fraction", 0.2, 0.05, 1., 0.05)],
        "stad7_vix" => vec![p("lowl", "Bull key reversal lookback (VIX lows)", 12., 1., 30., 1.), p("highh", "Bear key reversal lookback (VIX highs)", 1., 1., 30., 1.)],
        "stad10_mwt" => vec![p("len", "Average length", 2., 1., 10., 1.), p("stop_usd", "Stop loss $ per position (0 = none)", 1500., 0., 5000., 250.), p("pt_usd", "$ per index point (S&P 500 futures, 1995-99)", 500., 1., 1000., 50.)],
        "stad4_pricedist" => vec![p("klen", "Kurtosis length", 40., 10., 80., 5.), p("slen", "Skew length", 15., 5., 40., 1.)],
        _ => return None,
    })
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if from > to || to >= x.c.len() { return None; }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let u = |k: &str| g(k).max(1.0).round() as usize;
    let (h, l, c) = (&x.h, &x.l, &x.c);
    let n = c.len();
    Some(match id {
        "stad1_maco" => {
            let (fa, sl, ch, tr, rb, rl) = (sma(c, u("fast")), sma(c, u("slow")), u("chlen"), u("trail"), u("rebars") as i64, u("relen"));
            let (up, dn, rew) = (1.0 + g("pct") / 100.0, 1.0 - g("pct") / 100.0, g("rew"));
            let mbb = u("slow").max(ch).max(tr).max(rl);
            let (mut lep, mut sep, mut lc, mut sc, mut rc, mut curp_prev) = (0.0, 0.0, -999i64, -999i64, 0i64, 0i8);
            run_ts(x, from, to, mbb, |i, st, _now, nx| {
                if xover(&fa, &sl, i) && st.bn > 1 { lep = highest(h, ch, i) * up; lc = st.bn; }
                if st.pos != 1 && st.bn < lc + ch as i64 { nx.push(o(A::Buy, P::Stop(lep), "Cross Over Buy")); }
                if xunder(&fa, &sl, i) && st.bn > 1 { sep = lowest(l, ch, i) * dn; sc = st.bn; }
                if st.pos != -1 && st.bn < sc + ch as i64 { nx.push(o(A::Sell, P::Stop(sep), "Cross Under Sell")); }
                if st.pos == 1 { lc = -999; nx.push(o(A::Xl, P::Stop(lowest(l, tr, i)), "LongTStop")); }
                if st.pos == -1 { sc = -999; nx.push(o(A::Xs, P::Stop(highest(h, tr, i)), "ShortTStop")); }
                if st.pos == 0 && curp_prev != 0 { rc = 1; }
                curp_prev = st.pos;
                if st.pos == 0 && st.prev_pos == 1 && rc < rb { rc += 1; nx.push(ow(A::Buy, P::Stop(highest(h, rl, i)), rew, "Long ReEntry")); }
                if st.pos == 0 && st.prev_pos == -1 && rc < rb { rc += 1; nx.push(ow(A::Sell, P::Stop(lowest(l, rl, i)), rew, "Short ReEntry")); }
            })
        }
        "stad1_macdbo" => {
            let (fe, se) = (xavg(c, u("fast")), xavg(c, u("slow")));
            let m: Vec<f64> = (0..n).map(|i| fe[i] - se[i]).collect();
            let sg = xavg(&m, u("sig"));
            let md: Vec<f64> = (0..n).map(|i| m[i] - sg[i]).collect();
            let zero = vec![0.0; n];
            let tr = true_range(x);
            let atr4 = sma(&tr, 4);
            let (nc, nb, tb) = (u("ncos").min(51), u("nbars") as i64, u("trail"));
            let mbb = u("slow").max(tb).max(4);
            let mut co: Vec<(i64, f64, f64)> = vec![(0, 0.0, 0.0); 51];
            run_ts(x, from, to, mbb, |i, st, _now, nx| {
                if xover(&md, &zero, i) || xunder(&md, &zero, i) { co.rotate_right(1); co[0] = (st.bn, h[i], l[i]); }
                let hh = co[..nc].iter().map(|r| r.1).fold(-1.0, f64::max);
                let ll = co[..nc].iter().map(|r| r.2).fold(9999.0, f64::min);
                if st.bn - co[nc - 1].0 < nb {
                    nx.push(o(A::Buy, P::Stop(hh + atr4[i] * 0.5), "MACD BO Buy"));
                    nx.push(o(A::Sell, P::Stop(ll - atr4[i] * 0.5), "MACD BO Sell"));
                }
                nx.push(o(A::Xl, P::Stop(lowest(l, tb, i)), "Trail LX"));
                nx.push(o(A::Xs, P::Stop(highest(h, tb, i)), "Trail SX"));
            })
        }
        "stad1_tla" => {
            let (hs, ls, deg, tb, pts, sm) = (u("hstren"), u("lstren"), g("deg"), u("trail"), g("pts"), g("slope_mode") >= 0.5);
            let steep = |a: f64, b: f64| if sm { (a - b).atan().to_degrees() > deg } else { (a - b).abs() > pts };
            let mbb = (2 * hs.max(ls) + 1).max(tb);
            // swing arrays (bar index, value), row 0 = most recent; 11 rows as in the code
            let mut rh: Vec<(f64, f64)> = vec![(f64::NAN, 0.0); 11];
            let mut rlw: Vec<(f64, f64)> = vec![(f64::NAN, 99999.0); 11];
            let (mut hline, mut lline): (Option<(f64, f64, f64, f64)>, Option<(f64, f64, f64, f64)>) = (None, None);
            let (mut bp, mut bp1, mut sp, mut sp1) = (0.0f64, 0.0f64, 0.0f64, 0.0f64);
            let tl = |ln: (f64, f64, f64, f64), b: f64| ln.1 + (ln.3 - ln.1) / (ln.2 - ln.0) * (b - ln.0);
            run_ts(x, from, to, mbb, |i, _st, _now, nx| {
                if swing_high(h, hs, i) {
                    rh.rotate_right(1); rh[0] = ((i - hs) as f64, h[i - hs]);
                    if let Some(k) = (1..=10).find(|&k| rh[k].1 > rh[0].1) { hline = Some((rh[k].0, rh[k].1, rh[0].0, rh[0].1)); }
                }
                if swing_low(l, ls, i) {
                    rlw.rotate_right(1); rlw[0] = ((i - ls) as f64, l[i - ls]);
                    if let Some(k) = (1..=10).find(|&k| rlw[k].1 < rlw[0].1) { lline = Some((rlw[k].0, rlw[k].1, rlw[0].0, rlw[0].1)); }
                }
                let (pb, ps) = (bp, sp); // variable values on the previous bar
                if let Some(ln) = hline {
                    bp = tl(ln, i as f64); bp1 = tl(ln, i as f64 - 1.0);
                    if c[i] > bp && c[i - 1] <= pb && steep(bp1, bp) { nx.push(o(A::Buy, P::Mkt, "TL Buy")); }
                }
                if let Some(ln) = lline {
                    sp = tl(ln, i as f64); sp1 = tl(ln, i as f64 - 1.0);
                    if c[i] < sp && c[i - 1] >= ps && steep(sp, sp1) { nx.push(o(A::Sell, P::Mkt, "TL Sell")); }
                }
                nx.push(o(A::Xl, P::Stop(lowest(l, tb, i)), "Trail LX"));
                nx.push(o(A::Xs, P::Stop(highest(h, tb, i)), "Trail SX"));
            })
        }
        "stad1_tf" => {
            let (xa, vol, mf) = (xavg(c, u("xma")), volatility(x, u("vol")), g("mult"));
            let (f4, m9, s18) = (sma(c, 4), sma(c, 9), sma(c, 18));
            let mbb = 18usize.max(u("vol")).max(u("xma"));
            let (mut top, mut bot) = (0.0, 0.0);
            run_ts(x, from, to, mbb, |i, st, now, nx| {
                if st.bn == 1 { top = x.o[i] + vol[i] * mf; bot = x.o[i] - vol[i] * mf; }
                let bull = f4[i] > m9[i] && m9[i] > s18[i];
                let bear = f4[i] < m9[i] && m9[i] < s18[i];
                if xa[i] > top && bull { now.push(o(A::Buy, P::Mkt, "TF Buy")); bot = top; top += vol[i] * mf; }
                if xa[i] < bot && bear { now.push(o(A::Sell, P::Mkt, "TF Sell")); top = bot; bot -= vol[i] * mf; }
                if st.pos == 1 { nx.push(o(A::Xl, P::Stop(bot), "BotBand LX")); }
                if st.pos == -1 { nx.push(o(A::Xs, P::Stop(top), "TopBand SX")); }
            })
        }
        "stad1_sr" => {
            let (s, ns, tb) = (u("strength"), u("nswing").min(21), u("trail"));
            let tr = true_range(x);
            let atr10 = sma(&tr, 10);
            let (xh, xl) = (xavg(h, 4), xavg(l, 4));
            let mbb = (2 * s + 1).max(10).max(tb);
            let (mut shi, mut slo) = (vec![0.0f64; 21], vec![0.0f64; 21]);
            let (mut v17, mut v18) = (0.0, 0.0);
            let (mut ha, mut la) = (vec![0.0f64; n], vec![0.0f64; n]);
            run_ts(x, from, to, mbb, |i, st, _now, nx| {
                if swing_high(h, s, i) { shi.rotate_right(1); shi[0] = h[i - s]; }
                if swing_low(l, s, i) { slo.rotate_right(1); slo[0] = l[i - s]; }
                let (mut lo_, mut hi_) = (9999.0f64, 0.0f64);
                for j in 0..ns { if shi[j] > hi_ { hi_ = shi[j]; } if slo[j] < lo_ { lo_ = slo[j]; } }
                ha[i] = shi[..ns].iter().sum::<f64>() / ns as f64;
                la[i] = slo[..ns].iter().sum::<f64>() / ns as f64;
                if hi_ - lo_ != 0.0 { v17 = (ha[i] - lo_) / (hi_ - lo_); v18 = (la[i] - lo_) / (hi_ - lo_); }
                let c1 = ha[i] - la[i] > atr10[i] * 2.0;
                let c2 = v17 > 0.66 && v18 < 0.33;
                let c3 = c[i] < ha[i] && c[i] > la[i];
                if c1 && c2 && c3 {
                    if xunder(&xh, &ha, i) { nx.push(o(A::Sell, P::Mkt, "SR Sell")); }
                    if xover(&xl, &la, i) { nx.push(o(A::Buy, P::Mkt, "SR Buy")); }
                }
                if st.profit_pts(c[i]) > 0.0 {
                    nx.push(o(A::Xl, P::Stop(lowest(l, tb, i)), "Trail LX"));
                    nx.push(o(A::Xs, P::Stop(highest(h, tb, i)), "Trail SX"));
                }
            })
        }
        "stad1_chbo" => {
            let (len, brk, tb) = (u("len"), u("brk") as i64, u("trail"));
            let (mut bo, mut bu) = (0i64, 0i64);
            run_ts(x, from, to, (len + 1).max(tb), |i, _st, now, nx| {
                let (hc, lc) = (highest(h, len, i - 1), lowest(l, len, i - 1));
                if h[i] > hc { bo += 1; bu = 0; }
                if l[i] < lc { bu += 1; bo = 0; }
                if bo >= brk { now.push(o(A::Buy, P::Mkt, "Breakout Buy")); bo = 1; }
                if bu >= brk { now.push(o(A::Sell, P::Mkt, "Breakout Sell")); bu = 1; }
                nx.push(o(A::Xl, P::Stop(lowest(l, tb, i)), "Trail LX"));
                nx.push(o(A::Xs, P::Stop(highest(h, tb, i)), "Trail SX"));
            })
        }
        "stad1_volex" => {
            let rng: Vec<f64> = (0..n).map(|i| h[i] - l[i]).collect();
            let ar = sma(&rng, 4);
            let (lm, sm, pp, tb, sb, spt) = (g("lmult"), g("smult"), g("profitpt"), u("trail"), u("stalebars"), g("stale_pts"));
            run_ts(x, from, to, 4usize.max(tb), |i, st, _now, nx| {
                if i + 1 >= n { return; }
                let on = x.o[i + 1];
                nx.push(o(A::Buy, P::Stop(on + ar[i] * lm), "VolEx Buy"));
                nx.push(o(A::Sell, P::Stop(on - ar[i] * sm), "VolEx Sell"));
                if st.pos == 1 && on > st.entry_px + pp { nx.push(o(A::Xl, P::Mkt, "Long Profit")); }
                if st.pos == -1 && on < st.entry_px - pp { nx.push(o(A::Xs, P::Mkt, "Short Profit")); }
                if st.pos == 1 { nx.push(o(A::Xl, P::Stop(lowest(l, tb, i)), "Trail LX")); }
                if st.pos == -1 { nx.push(o(A::Xs, P::Stop(highest(h, tb, i)), "Trail SX")); }
                if st.pos != 0 && i - st.entry_i > sb && st.profit_pts(c[i]) < spt {
                    nx.push(o(A::Xl, P::Mkt, "Long Too Long")); nx.push(o(A::Xs, P::Mkt, "Short Too Long"));
                }
            })
        }
        "stad1_pivot" => {
            let (f4, m9, s18, tb) = (sma(c, 4), sma(c, 9), sma(c, 18), u("trail"));
            run_ts(x, from, to, 18usize.max(tb), |i, st, _now, nx| {
                let piv = (l[i] + h[i] + c[i]) / 3.0; let rg = h[i] - l[i];
                let (r1, s1) = (piv * 2.0 - l[i], piv * 2.0 - h[i]);
                if f4[i] > m9[i] && m9[i] > s18[i] { nx.push(o(A::Buy, P::Stop(piv + rg), "Bull R2")); nx.push(o(A::Buy, P::Stop(r1 + rg), "Bull R3")); }
                if f4[i] < m9[i] && m9[i] < s18[i] { nx.push(o(A::Sell, P::Stop(piv - rg), "Bear S2")); nx.push(o(A::Sell, P::Stop(s1 - rg), "Bear S3")); }
                if i + 1 < n {
                    if st.pos == 1 && x.o[i + 1] > st.entry_px { nx.push(o(A::Xl, P::Mkt, "Long Profit")); }
                    if st.pos == -1 && x.o[i + 1] < st.entry_px { nx.push(o(A::Xs, P::Mkt, "Short Profit")); }
                }
                nx.push(o(A::Xl, P::Stop(lowest(l, tb, i)), "Trail LX"));
                nx.push(o(A::Xs, P::Stop(highest(h, tb, i)), "Trail SX"));
            })
        }
        "stad2_adxdiff" => {
            let (_, _, adx) = dmi3(x, u("adx"));
            let dadx: Vec<f64> = (0..n).map(|i| if i == 0 { f64::NAN } else { adx[i] - adx[i - 1] }).collect();
            let davg = wavg(&dadx, u("avg"));
            let zero = vec![0.0; n];
            let ar4 = sma(&(0..n).map(|i| h[i] - l[i]).collect::<Vec<_>>(), 4);
            let (ch, pb) = (u("ch") as i64, u("prot"));
            let (mut up, mut dn, mut lob, mut sp, mut mpp) = (0.0, 0.0, 0i64, 0.0, 0i8);
            run_ts(x, from, to, 3 * u("adx") + u("avg"), |i, st, _now, nx| {
                let mp = st.pos;
                if (davg[i] < 0.0 || xover(&davg, &zero, i)) && st.bn > 1 { up = c[i] + ar4[i] / 2.0; dn = c[i] - ar4[i] / 2.0; lob = st.bn; }
                if mp != 0 { lob = -999; }
                if st.bn < lob + ch {
                    nx.push(o(A::Buy, P::Stop(up), "ADX Buy")); nx.push(o(A::Sell, P::Stop(dn), "ADX Sell"));
                    if mp != 1 { nx.push(o(A::Xl, P::Stop(lowest(l, pb, i)), "LEntryStop")); }
                    if mp != -1 { nx.push(o(A::Xs, P::Stop(highest(h, pb, i)), "SEntryStop")); }
                }
                if mp == 1 && mpp != 1 { lob = -999; sp = l[i] - ar4[i]; }
                if mp == -1 && mpp != -1 { sp = h[i] + ar4[i]; lob = -999; }
                if mp == 1 { nx.push(o(A::Xl, P::Stop(sp), "LongTrailStop")); sp += (l[i] - sp) / 3.0; }
                if mp == -1 { nx.push(o(A::Xs, P::Stop(sp), "ShortTrailStop")); sp -= (sp - h[i]) / 3.0; }
                mpp = mp;
            })
        }
        "stad2_hs" => {
            let (ts, bs) = (u("top"), u("bot"));
            let ar4 = sma(&(0..n).map(|i| h[i] - l[i]).collect::<Vec<_>>(), 4);
            // per half: pattern swings (bar, value, used) and neckline swings (bar, value)
            let mut th: Vec<(f64, f64, bool)> = vec![(0.0, 0.0, false); 4]; let mut tl_: Vec<(f64, f64)> = vec![(0.0, 0.0); 4];
            let mut bl: Vec<(f64, f64, bool)> = vec![(0.0, 0.0, false); 4]; let mut bh: Vec<(f64, f64)> = vec![(0.0, 0.0); 4];
            let (mut tline, mut bline): (Option<(f64, f64, f64, f64)>, Option<(f64, f64, f64, f64)>) = (None, None);
            let (mut tid, mut bid, mut tlast, mut blast) = (0u64, 0u64, 0u64, 0u64);
            let (mut psp, mut pmpp, mut qsp, mut qmpp) = (0.0, 0i8, 0.0, 0i8);
            let tlv = |ln: (f64, f64, f64, f64), b: f64| if ln.2 == ln.0 { ln.3 } else { ln.1 + (ln.3 - ln.1) / (ln.2 - ln.0) * (b - ln.0) };
            run_ts(x, from, to, 2 * ts.max(bs) + 2, |i, st, _now, nx| {
                let bn = st.bn as f64;
                // ---- top half
                if swing_high(h, ts, i) { th.rotate_right(1); th[0] = (bn - ts as f64, h[i - ts], false); }
                if swing_low(l, bs, i) { tl_.rotate_right(1); tl_[0] = (bn - bs as f64, l[i - bs]); }
                let c1 = th[0].0 > tl_[0].0 && tl_[0].0 > th[1].0 && th[1].0 > tl_[1].0 && tl_[1].0 > th[2].0;
                let c2 = th[1].1 > th[2].1 && th[1].1 > th[0].1;
                let c3 = tl_[0].1 < th[2].1 && tl_[1].1 < th[0].1;
                let den = th[1].1 - tl_[0].1.min(tl_[1].1);
                let c4 = if den != 0.0 { (tl_[0].1 - tl_[1].1).abs() / den < 0.1 } else { true };
                if c1 && c2 && c3 && c4 && !th[0].2 { th[0].2 = true; tline = Some((tl_[1].0, tl_[1].1, tl_[0].0, tl_[0].1)); tid += 1; }
                if let Some(ln) = tline { if tlast != tid && c[i] < tlv(ln, bn) && c[i - 1] >= tlv(ln, bn - 1.0) { nx.push(o(A::Sell, P::Mkt, "H&S Top")); tlast = tid; } }
                let mp = st.pos;
                if mp == -1 && pmpp != -1 { psp = h[i] + ar4[i]; }
                if mp == -1 { nx.push(o(A::Xs, P::Stop(psp), "parashort")); psp -= 0.3 * (psp - h[i]); }
                pmpp = mp;
                // ---- bottom half (mirror)
                if swing_low(l, bs, i) { bl.rotate_right(1); bl[0] = (bn - bs as f64, l[i - bs], false); }
                if swing_high(h, ts, i) { bh.rotate_right(1); bh[0] = (bn - ts as f64, h[i - ts]); }
                let d1 = bl[0].0 > bh[0].0 && bh[0].0 > bl[1].0 && bl[1].0 > bh[1].0 && bh[1].0 > bl[2].0;
                let d2 = bl[1].1 < bl[2].1 && bl[1].1 < bl[0].1;
                let d3 = bh[0].1 > bl[2].1 && bh[1].1 > bl[0].1;
                let den = bh[0].1.max(bh[1].1) - bl[1].1;
                let d4 = if den != 0.0 { (bh[0].1 - bh[1].1).abs() / den < 0.1 } else { true };
                if d1 && d2 && d3 && d4 && !bl[0].2 { bl[0].2 = true; bline = Some((bh[1].0, bh[1].1, bh[0].0, bh[0].1)); bid += 1; }
                if let Some(ln) = bline { if blast != bid && c[i] > tlv(ln, bn) && c[i - 1] <= tlv(ln, bn - 1.0) { nx.push(o(A::Buy, P::Mkt, "H&S Bottom")); blast = bid; } }
                if mp == 1 && qmpp != 1 { qsp = l[i] - ar4[i]; }
                if mp == 1 { nx.push(o(A::Xl, P::Stop(qsp), "paralong")); qsp += 0.3 * (l[i] - qsp); }
                qmpp = mp;
            })
        }
        "stad2_obv" => {
            let ob = obv(x);
            let (len, back, nh, nl) = (u("len"), u("back"), u("nh"), u("nl"));
            let newh: Vec<bool> = (0..n).map(|i| ob[i] >= highest(&ob, len, i)).collect();
            let newl: Vec<bool> = (0..n).map(|i| ob[i] <= lowest(&ob, len, i)).collect();
            let ar4 = sma(&(0..n).map(|i| h[i] - l[i]).collect::<Vec<_>>(), 4);
            let mut pa = Para::default();
            run_ts(x, from, to, len.max(back).max(4), |i, st, _now, nx| {
                if mro(&newh, i, back, nh) > -1 { nx.push(o(A::Buy, P::Mkt, "New High")); }
                if mro(&newl, i, back, nl) > -1 { nx.push(ow(A::Buy, P::Mkt, 2.0, "New Low")); }
                pa.step(x, i, st.pos, ar4[i], 1.0, 1.0 / 3.0, 1.0 / 3.0, nx, "ExitTrade");
            })
        }
        "stad2_addiv" => {
            let v = vol(x);
            let raw: Vec<f64> = (0..n).map(|i| { let r = h[i] - l[i]; if r > 0.0 { (c[i] - x.o[i]) / r * v[i] } else { 0.0 } }).collect();
            let vm = xavg(&raw, u("len"));
            let lb = u("look");
            let ar4 = sma(&(0..n).map(|i| h[i] - l[i]).collect::<Vec<_>>(), 4);
            let lo_v: Vec<bool> = (0..n).map(|i| i > lb && vm[i] < lowest(&vm, lb, i - 1)).collect();
            let hi_v: Vec<bool> = (0..n).map(|i| i > lb && vm[i] > highest(&vm, lb, i - 1)).collect();
            let mut pa = Para::default();
            run_ts(x, from, to, lb + 1, |i, st, _now, nx| {
                if st.bn > 1 {
                    let nlow = l[i] < lowest(l, lb, i - 1); let nhigh = h[i] > highest(h, lb, i - 1);
                    if nlow && mro(&lo_v, i, 5, 1) == -1 { nx.push(o(A::Buy, P::Mkt, "AD Buy 100")); }
                    if nlow && hi_v[i] { nx.push(ow(A::Buy, P::Mkt, 2.0, "AD Buy 200")); }
                    if nhigh && mro(&hi_v, i, 5, 1) == -1 { nx.push(o(A::Sell, P::Mkt, "AD Sell 100")); }
                    if nhigh && lo_v[i] { nx.push(ow(A::Sell, P::Mkt, 2.0, "AD Sell 200")); }
                }
                pa.step(x, i, st.pos, ar4[i], 1.0, 1.0 / 3.0, 1.0 / 3.0, nx, "Exit");
            })
        }
        "stad2_stoch" => {
            let sk = slowk(x, u("len"));
            let (obl, osl) = (g("ob"), g("os"));
            let below: Vec<bool> = (0..n).map(|i| sk[i] < obl).collect();
            let above: Vec<bool> = (0..n).map(|i| sk[i] > osl).collect();
            let ar4 = sma(&(0..n).map(|i| h[i] - l[i]).collect::<Vec<_>>(), 4);
            let (mut mode, mut rp, mut mpp) = (0i8, 0.0, 0i8);
            run_ts(x, from, to, u("len") + 3, |i, st, _now, nx| {
                let mp = st.pos;
                if mro(&below, i, 3, 1) == -1 && mp != -1 { nx.push(o(A::Buy, P::Mkt, "Stoch Buy")); mode = 1; }
                if mp == 1 && mpp != 1 { rp = l[i] - ar4[i]; }
                if mp == 1 && mode == 1 { nx.push(ow(A::Sell, P::Stop(rp), 2.0, "ReverseLong")); rp += (l[i] - rp) / 3.0; }
                nx.push(of(A::Xs, P::Stop(highest(h, 5, i)), "RL Exit", "ReverseLong"));
                if mro(&above, i, 3, 1) == -1 && mp != 1 { nx.push(o(A::Sell, P::Mkt, "Stoch Sell")); mode = -1; }
                if mp == -1 && mpp != -1 { rp = h[i] + ar4[i]; }
                if mp == -1 && mode == -1 { nx.push(ow(A::Buy, P::Stop(rp), 2.0, "ReverseShort")); rp -= (rp - h[i]) / 3.0; }
                nx.push(of(A::Xl, P::Stop(lowest(l, 5, i)), "RS Exit", "ReverseShort"));
                mpp = mp;
            })
        }
        "stad2_ccibb" => {
            let (bl_, sd) = (u("bblen"), g("bbsd"));
            let (top, bot) = (bband(c, bl_, sd), bband(c, bl_, -sd));
            let ca = xavg(&cci(x, u("ccilen")).iter().map(|v| if v.is_finite() { *v } else { 0.0 }).collect::<Vec<_>>(), u("cciavg"));
            let ar4 = sma(&(0..n).map(|i| h[i] - l[i]).collect::<Vec<_>>(), 4);
            let half = vec![0.5; n];
            let mut posn = vec![0.0; n];
            let (mut st_, mut lc, mut sc, mut lep, mut sep, mut sp, mut mpp) = (0i8, 0i64, 0i64, 0.0, 0.0, 0.0, 0i8);
            run_ts(x, from, to, bl_.max(u("ccilen")) + 5, |i, st, _now, nx| {
                posn[i] = if top[i] != bot[i] { (c[i] - bot[i]) / (top[i] - bot[i]) } else { posn[i - 1] };
                if st.bn > 1 {
                    if xover(&posn, &half, i) || xunder(&posn, &half, i) { st_ = 0; }
                    if xunder(l, &bot, i) { st_ = -1; }
                    if xover(h, &top, i) { st_ = 1; }
                }
                let up = ca[i - 1] <= ca[i - 2] && ca[i - 1] < ca[i] && ca[i - 1] < -100.0;
                let dn = ca[i - 2] >= ca[i - 3] && ca[i - 2] >= ca[i - 4] && ca[i - 2] > ca[i - 1] && ca[i - 2] > ca[i] && ca[i - 2] > 100.0;
                if st_ == -1 && up { lc = st.bn; lep = highest(h, 3, i); }
                if st.bn < lc + 5 && st.pos != 1 { nx.push(o(A::Buy, P::Stop(lep), "CCI Buy")); }
                if st_ == 1 && dn { sc = st.bn; sep = lowest(l, 3, i); }
                if st.bn < sc + 5 && st.pos != -1 { nx.push(o(A::Sell, P::Stop(sep), "CCI Sell")); }
                if st.pos != 0 { lc = -999; }
                let mp = st.pos;
                if mp == 1 && mpp != 1 { sp = l[i] - ar4[i] * 2.0 / 3.0; }
                if mp == -1 && mpp != -1 { sp = h[i] + ar4[i] * 2.0 / 3.0; }
                if mp == 1 { nx.push(o(A::Xl, P::Stop(sp), "ExitLong")); sp += 0.3 * (l[i] - sp); }
                if mp == -1 { nx.push(o(A::Xs, P::Stop(sp), "ExitShort")); sp -= 0.3 * (sp - h[i]); }
                mpp = mp;
            })
        }
        "stad2_pctchan" => {
            let (pc, obl, osl, bands) = (u("pchg"), g("ob"), g("os"), u("bands"));
            let (mut os, mut ob, mut hb, mut lb_) = (vec![false; n], vec![false; n], 0.0, 0.0);
            run_ts(x, from, to, pc.max(bands).max(5), |i, st, _now, nx| {
                os[i] = os[i - 1]; ob[i] = ob[i - 1];
                let pchg = c[i] / c[i - pc] * 100.0;
                if pchg < osl { os[i] = true; hb = highest(h, bands, i); }
                if pchg > obl { ob[i] = true; lb_ = lowest(l, bands, i); }
                if os[i] && st.pos != 1 { nx.push(o(A::Buy, P::Stop(hb), "PC Buy")); }
                if ob[i] && st.pos != -1 { nx.push(o(A::Sell, P::Stop(lb_), "PC Sell")); }
                if i + 1 < n {
                    if st.pos == 1 && x.o[i + 1] > st.entry_px { nx.push(o(A::Xl, P::Mkt, "Long Profit")); }
                    if st.pos == -1 && x.o[i + 1] < st.entry_px { nx.push(o(A::Xs, P::Mkt, "Short Profit")); }
                }
                if os[i] && os[i - 1] { os[i] = false; }
                if ob[i] && ob[i - 1] { ob[i] = false; }
                nx.push(o(A::Xl, P::Stop(lowest(l, 5, i)), "Trail LX"));
                nx.push(o(A::Xs, P::Stop(highest(h, 5, i)), "Trail SX"));
            })
        }
        "stad2_stats2" | "stad2_stats3" => {
            let short = id == "stad2_stats3";
            let wk = |i: usize| days(&x.r[i].date).div_euclid(7) - if ymdw(&x.r[i].date).3 >= 4 { 0 } else { 0 };
            // first trading day of a Monday-Friday week: previous bar falls in an earlier Monday-based week
            let mon_week = |i: usize| (days(&x.r[i].date) - ymdw(&x.r[i].date).3).div_euclid(7);
            let _ = wk;
            run_ts(x, from, to, 1, |i, st, now, nx| {
                if st.pos != 0 { now.push(o(if short { A::Xs } else { A::Xl }, P::Mkt, "Close of day")); }
                if i + 1 >= n { return; }
                let fire = if short { let (_, _, d, w) = ymdw(&x.r[i + 1].date); d == 13 && w == 4 } else { mon_week(i + 1) != mon_week(i) };
                if fire { nx.push(o(if short { A::Sell } else { A::Buy }, P::Mkt, if short { "Friday 13th" } else { "First day of week" })); }
            })
        }
        "stad3_bdsr" => {
            let r = crate::connors::rsi(c, u("rsilen"));
            let (p_, m_, _) = dmi3(x, u("dmilen"));
            let lvl = g("rsilevel");
            let ar4 = sma(&(0..n).map(|i| h[i] - l[i]).collect::<Vec<_>>(), 4);
            let (mut bs, mut ss, mut bs1, mut ss1, mut lep, mut sep) = (false, false, false, false, 0.0, 0.0);
            let mut pa = Para::default();
            run_ts(x, from, to, u("rsilen").max(u("dmilen")) * 2 + 1, |i, st, _now, nx| {
                let spread = p_[i] - m_[i];
                if r[i] >= lvl && spread < 0.0 { bs = true; ss = false; }
                if r[i] < lvl && spread > 0.0 { ss = true; bs = false; }
                if bs { if !bs1 { lep = h[i]; } nx.push(o(A::Buy, P::Stop(lep), "BD Buy")); }
                if ss { if !ss1 { sep = l[i]; } nx.push(o(A::Sell, P::Stop(sep), "SR Sell")); }
                bs1 = bs; ss1 = ss;
                pa.step(x, i, st.pos, ar4[i], 1.0, 1.0 / 3.0, 1.0 / 3.0, nx, "Trail");
            })
        }
        "stad3_engulf" => {
            let xa = xavg(c, u("len"));
            let body = sma(&(0..n).map(|i| (c[i] - x.o[i]).abs()).collect::<Vec<_>>(), u("body"));
            let ar4 = sma(&(0..n).map(|i| h[i] - l[i]).collect::<Vec<_>>(), 4);
            let o_ = &x.o;
            let bull: Vec<bool> = (0..n).map(|i| i > 0 && xa[i] < xa[i - 1] && c[i] > o_[i] && c[i - 1] < o_[i - 1] && c[i] > o_[i - 1] && o_[i] < c[i - 1]
                && c[i] - o_[i] > body[i] * 1.25 && o_[i - 1] - c[i - 1] < body[i] * 0.75).collect();
            let bear: Vec<bool> = (0..n).map(|i| i > 0 && xa[i] > xa[i - 1] && o_[i] > c[i] && o_[i - 1] < c[i - 1] && o_[i] > c[i - 1] && c[i] < o_[i - 1]
                && o_[i] - c[i] > body[i] * 1.25 && c[i - 1] - o_[i - 1] < body[i] * 0.75).collect();
            let (mut lep, mut sep, mut hc, mut lc) = (0.0, 0.0, 0.0, 999999.0);
            run_ts(x, from, to, u("len").max(u("body")) + 1, |i, st, _now, nx| {
                if bull[i] { lep = h[i]; }
                if bear[i] { sep = l[i]; }
                if mro(&bull, i, 3, 1) > -1 && st.pos != 1 { nx.push(o(A::Buy, P::Stop(lep), "Bull Engulf")); }
                if mro(&bear, i, 3, 1) > -1 && st.pos != -1 { nx.push(o(A::Sell, P::Stop(sep), "Bear Engulf")); }
                if st.pos == 1 { if c[i] > hc { hc = c[i]; } nx.push(o(A::Xl, P::Stop(hc - ar4[i] / 2.0), "Trail LX")); } else { hc = 0.0; }
                if st.pos == -1 { if c[i] < lc { lc = c[i]; } nx.push(o(A::Xs, P::Stop(lc + ar4[i] / 2.0), "Trail SX")); } else { lc = 999999.0; }
            })
        }
        "stad3_ochist" => {
            let (xc, xo) = (xavg(c, u("clen")), xavg(&x.o, u("olen")));
            let hist: Vec<f64> = (0..n).map(|i| xc[i] - xo[i]).collect();
            let zero = vec![0.0; n];
            let atr10 = sma(&true_range(x), 10);
            let co: Vec<bool> = (0..n).map(|i| xover(&hist, &zero, i)).collect();
            let cu: Vec<bool> = (0..n).map(|i| xunder(&hist, &zero, i)).collect();
            let (mut bp, mut sp, mut lxp, mut sxp) = (0.0, 0.0, 0.0, 0.0);
            run_ts(x, from, to, 11, |i, _st, now, nx| {
                if co[i] { bp = h[i] + atr10[i] * 0.5; lxp = l[i - 1] - atr10[i - 1] / 2.0; }
                if mro(&co, i, 10, 1) > -1 && hist[i] > 0.0 { nx.push(o(A::Buy, P::Stop(bp), "LE")); }
                if cu[i] { sp = l[i] - atr10[i] * 0.5; sxp = h[i - 1] + atr10[i - 1] / 2.0; }
                if mro(&cu, i, 10, 1) > -1 && hist[i] < 0.0 { nx.push(o(A::Sell, P::Stop(sp), "SE")); }
                if cu[i] { now.push(o(A::Xl, P::Mkt, "Hist under 0")); }
                nx.push(of(A::Xl, P::Stop(lxp), "LE stop", "LE"));
                if co[i] { now.push(o(A::Xs, P::Mkt, "Hist over 0")); }
                nx.push(of(A::Xs, P::Stop(sxp), "SE stop", "SE"));
            })
        }
        "stad3_bb" => {
            let (hb, lb_) = (bband(c, 10, 2.0), bband(c, 10, -2.0));
            let le_h: Vec<bool> = (0..n).map(|i| c[i] <= hb[i]).collect();
            let ge_l: Vec<bool> = (0..n).map(|i| c[i] >= lb_[i]).collect();
            let ar4 = sma(&(0..n).map(|i| h[i] - l[i]).collect::<Vec<_>>(), 4);
            let (nov, nun) = (u("nov"), u("nun"));
            let mut pa = Para::default();
            run_ts(x, from, to, 10 + nov.max(nun), |i, st, _now, nx| {
                if mro(&le_h, i, nov, 1) == -1 && st.pos != -1 { nx.push(o(A::Sell, P::Mkt, "BB Sell")); }
                if mro(&ge_l, i, nun, 1) == -1 && st.pos != 1 { nx.push(o(A::Buy, P::Mkt, "BB Buy")); }
                pa.step(x, i, st.pos, ar4[i], 1.0, 1.0 / 3.0, 1.0 / 3.0, nx, "Trail");
            })
        }
        "stad3_mmmw" => {
            let osc = accumdist(x);
            let (s1, s2) = (u("str"), u("str2"));
            let ar4 = sma(&(0..n).map(|i| h[i] - l[i]).collect::<Vec<_>>(), 4);
            // rows: (value, bar, flag) for price; (value, bar) for oscillator; 11 rows, -999 initial
            let mut wp: Vec<(f64, f64, f64)> = vec![(-999.0, -999.0, -999.0); 11]; let mut wo: Vec<(f64, f64)> = vec![(-999.0, -999.0); 11];
            let mut mp_: Vec<(f64, f64, f64)> = vec![(-999.0, -999.0, -999.0); 11]; let mut mo: Vec<(f64, f64)> = vec![(-999.0, -999.0); 11];
            let (mut wf, mut mf) = (vec![false; n], vec![false; n]);
            let up_dn: Vec<bool> = (0..n).map(|i| i >= 2 && c[i - 1] > c[i - 2] && c[i] < c[i - 1]).collect();
            let dn_up: Vec<bool> = (0..n).map(|i| i >= 2 && c[i - 1] < c[i - 2] && c[i] > c[i - 1]).collect();
            fn found3(p: &mut Vec<(f64, f64, f64)>, q: &[(f64, f64)]) -> bool {
                if (p[0].1 - q[0].1).abs() > 5.0 { return false; }
                for a in 1..=10 { for b in 1..=10 {
                    if (p[a].1 - q[b].1).abs() <= 5.0 && p[0].2 == 0.0 && ((p[0].0 > p[a].0 && q[0].0 < q[b].0) || (p[0].0 < p[a].0 && q[0].0 > q[b].0)) {
                        p[0].2 = -1.0; return true;
                    } } }
                false
            }
            let (mut wsp, mut msp, mut mmpp) = (0.0, 0.0, 0i8);
            run_ts(x, from, to, 2 * s1.max(s2) + 3, |i, st, _now, nx| {
                let bn = st.bn as f64;
                // W bottoms (long)
                let mut new = false;
                if swing_low(l, s1, i) { wp.rotate_right(1); wp[0] = (l[i - s1], bn - s1 as f64, 0.0); new = true; }
                if swing_low(&osc, s2, i) { wo.rotate_right(1); wo[0] = (osc[i - s2], bn - s2 as f64); new = true; }
                wf[i] = new && found3(&mut wp, &wo);
                let bsp = mro(&up_dn, i, 3, 1);
                if mro(&wf, i, 5, 1) > -1 && bsp > 0 { nx.push(o(A::Buy, P::Stop(highest(h, 4, i)), "Micro W")); }
                if st.pos == 1 { wsp = l[i] - ar4[i]; nx.push(o(A::Xl, P::Stop(wsp), "W Exit")); }
                // M tops (short)
                let mut new = false;
                if swing_high(h, s1, i) { mp_.rotate_right(1); mp_[0] = (h[i - s1], bn - s1 as f64, 0.0); new = true; }
                if swing_high(&osc, s2, i) { mo.rotate_right(1); mo[0] = (osc[i - s2], bn - s2 as f64); new = true; }
                mf[i] = new && found3(&mut mp_, &mo);
                let ssp = mro(&dn_up, i, 3, 1);
                if mro(&mf, i, 5, 1) > -1 && ssp > 0 { nx.push(o(A::Sell, P::Stop(lowest(l, 4, i)), "Micro M")); }
                if st.pos == -1 && mmpp != -1 { msp = h[i] + ar4[i]; }
                if st.pos == -1 { nx.push(o(A::Xs, P::Stop(msp), "M Exit")); msp -= (msp - h[i]) / 3.0; }
                mmpp = st.pos;
            })
        }
        "stad3_varzones" => {
            let r = crate::connors::rsi(c, u("rsilen"));
            let (sw, s, f) = (u("swings"), u("strength"), g("factor"));
            let (mut hs, mut ls): (Vec<f64>, Vec<f64>) = (vec![], vec![]);
            let (mut top, mut bot) = (vec![f64::NAN; n], vec![f64::NAN; n]);
            let rr: Vec<f64> = r.iter().map(|v| if v.is_finite() { *v } else { f64::NAN }).collect();
            let ar4 = sma(&(0..n).map(|i| h[i] - l[i]).collect::<Vec<_>>(), 4);
            let mut pa = Para::default();
            run_ts(x, from, to, u("rsilen") + 2 * s + 1, |i, st, now, nx| {
                if rr[i - 2 * s].is_finite() {
                    if swing_high(&rr, s, i) { hs.insert(0, rr[i - s]); hs.truncate(sw); }
                    if swing_low(&rr, s, i) { ls.insert(0, rr[i - s]); ls.truncate(sw); }
                }
                if hs.len() == sw { let a = hs.iter().sum::<f64>() / sw as f64; let mx = hs.iter().copied().fold(f64::MIN, f64::max); top[i] = a + f * (mx - a); }
                if ls.len() == sw { let a = ls.iter().sum::<f64>() / sw as f64; let mn = ls.iter().copied().fold(f64::MAX, f64::min); bot[i] = a - f * (a - mn); }
                if xover(&rr, &bot, i) { now.push(o(A::Buy, P::Mkt, "VZ Buy")); }
                if xunder(&rr, &top, i) { now.push(o(A::Sell, P::Mkt, "VZ Sell")); }
                pa.step(x, i, st.pos, ar4[i], 1.0, 1.0 / 3.0, 1.0 / 3.0, nx, "Trail");
            })
        }
        "stad3_sdv" => {
            let (len, sd, pp) = (u("len"), g("sd"), g("profitpt"));
            let s = stdev(c, len, 0.0);
            let a4 = sma(c, 4);
            run_ts(x, from, to, len.max(4), |i, st, _now, nx| {
                let br = 2.0 * sd * s[i];
                nx.push(o(A::Buy, P::Stop(c[i] * (1.0 + br / a4[i])), "SDV Buy"));
                nx.push(o(A::Sell, P::Stop(c[i] * (1.0 - br / a4[i])), "SDV Sell"));
                if i + 1 < n {
                    if st.pos == 1 && x.o[i + 1] > st.entry_px + pp { nx.push(o(A::Xl, P::Mkt, "Long Profit")); }
                    if st.pos == -1 && x.o[i + 1] < st.entry_px - pp { nx.push(o(A::Xs, P::Mkt, "Short Profit")); }
                }
            })
        }
        "stad3_volclus" => {
            let (len, sd, pp) = (u("len"), g("sd"), g("profitpt"));
            let ctc: Vec<f64> = (0..n).map(|i| if i == 0 { 0.0 } else { (c[i] - c[i - 1]).abs() }).collect();
            let rng: Vec<f64> = (0..n).map(|i| h[i] - l[i]).collect();
            let (m1, s1) = (sma(&ctc, len), stdev(&ctc, len, 0.0));
            let (m2, s2) = (sma(&rng, len), stdev(&rng, len, 0.0));
            let ar3 = sma(&rng, 3);
            run_ts(x, from, to, len + 2, |i, st, _now, nx| {
                let big = ctc[i] > m1[i - 1] + sd * s1[i - 1] || rng[i] > m2[i - 1] + sd * s2[i - 1];
                if big {
                    nx.push(o(A::Buy, P::Stop(c[i] + ar3[i - 1]), "VC Buy"));
                    nx.push(o(A::Sell, P::Stop(c[i] - ar3[i - 1]), "VC Sell"));
                }
                if i + 1 < n {
                    if st.pos == 1 && x.o[i + 1] > st.entry_px + pp { nx.push(o(A::Xl, P::Mkt, "Long Profit")); }
                    if st.pos == -1 && x.o[i + 1] < st.entry_px - pp { nx.push(o(A::Xs, P::Mkt, "Short Profit")); }
                }
            })
        }
        "stad4_dmiadx" => {
            let (dp, dm, _) = dmi3(x, u("dmilen"));
            let (_, _, adx) = dmi3(x, u("adxlen"));
            let (ax, pt) = (g("adxexit"), g("point"));
            let (mut bs, mut ss, mut bp, mut sp, mut xl, mut xs) = (false, false, 0.0, 0.0, 0.0, 0.0);
            run_ts(x, from, to, 3 * u("dmilen").max(u("adxlen")) + 1, |i, st, now, nx| {
                let mp = st.pos;
                if dp[i] > dm[i] && xover(&adx, &dm, i) && mp != 1 { bp = h[i] + pt; bs = true; xl = l[i] - pt; }
                if adx[i] < dm[i] || mp == 1 { bs = false; }
                if dp[i] < dm[i] && xover(&adx, &dp, i) && mp != -1 { sp = l[i] - pt; ss = true; xs = h[i] + pt; }
                if adx[i] < dp[i] || mp == -1 { ss = false; }
                if bs { nx.push(o(A::Buy, P::Stop(bp), "DMI Buy")); }
                if ss { nx.push(o(A::Sell, P::Stop(sp), "DMI Sell")); }
                if mp == 1 { bs = false; nx.push(o(A::Xl, P::Stop(xl), "Initial LX")); }
                if mp == -1 { ss = false; nx.push(o(A::Xs, P::Stop(xs), "Initial SX")); }
                let bse = if mp != 0 { i - st.entry_i } else { 0 };
                if adx[i - 1] >= ax && adx[i] < adx[i - 1] && bse > 1 {
                    if mp == 1 { now.push(o(A::Xl, P::Mkt, "ADX hook")); }
                    if mp == -1 { now.push(o(A::Xs, P::Mkt, "ADX hook")); }
                }
                if dp[i] < dm[i] && bse > 1 { now.push(o(A::Xl, P::Mkt, "DMI cross")); }
                if dp[i] > dm[i] && bse > 1 { now.push(o(A::Xs, P::Mkt, "DMI cross")); }
            })
        }
        "stad4_keltner" => {
            let (len, k, pct, life) = (u("len"), g("k"), g("pct"), u("life") as i64);
            let av = sma(c, len);
            let ar = sma(&true_range(x), len);
            let ku: Vec<f64> = (0..n).map(|i| av[i] + ar[i] * k).collect();
            let kl: Vec<f64> = (0..n).map(|i| av[i] - ar[i] * k).collect();
            let (mut setb, mut cl, mut cs) = (0.0, 0i64, 0i64);
            run_ts(x, from, to, len + 1, |i, _st, now, nx| {
                let rng = (ku[i] - kl[i]) / 2.0;
                cl += 1; cs += 1;
                if xover(c, &ku, i) { setb = h[i]; cl = 1; }
                if c[i] > ku[i] && cl <= life { nx.push(o(A::Buy, P::Stop(setb + rng * pct), "KC Buy")); }
                if xunder(c, &kl, i) { setb = l[i]; cs = 1; }
                if c[i] < kl[i] && cs <= life { nx.push(o(A::Sell, P::Stop(setb - rng * pct), "KC Sell")); }
                if xunder(c, &av, i) { now.push(o(A::Xl, P::Mkt, "Below average")); }
                if xover(c, &av, i) { now.push(o(A::Xs, P::Mkt, "Above average")); }
                nx.push(o(A::Xl, P::Stop(lowest(l, 4, i)), "Trail LX"));
                nx.push(o(A::Xs, P::Stop(highest(h, 4, i)), "Trail SX"));
            })
        }
        "stad4_linreg" => {
            let xl = xavg(&linreg(c, u("lrlen")).iter().map(|v| if v.is_finite() { *v } else { 0.0 }).collect::<Vec<_>>(), u("avglen"));
            let ml = u("momlen");
            let pct = g("pct");
            let atr10 = sma(&true_range(x), 10);
            let (mut sl_, mut ss_, mut cl, mut cs) = (0.0, 0.0, 0i64, 0i64);
            run_ts(x, from, to, u("lrlen") + u("avglen") + ml, |i, _st, _now, nx| {
                let mom = c[i] - c[i - ml]; let mom1 = c[i - 1] - c[i - 1 - ml];
                cl += 1; cs += 1;
                let (c1, c3) = (xl[i] > xl[i - 1], xl[i] < xl[i - 1]);
                let c2 = mom < 0.0 && mom > mom1; let c4 = mom > 0.0 && mom < mom1;
                if c1 && c2 {
                    if cl == 1 || cl > 4 { sl_ = h[i]; cl = 1; }
                    if cl <= 4 { let v = pct * (sl_ - xl[i]); nx.push(O { a: A::Buy, p: P::Stop(sl_ + v), w: 1.0, tag: "Long", from: "", prot: l[i] - pct * atr10[i] }); }
                }
                if c3 && c4 {
                    if cs == 1 || cs > 4 { ss_ = l[i]; cs = 1; }
                    if cs <= 4 { let v = pct * (ss_ - xl[i]); nx.push(O { a: A::Sell, p: P::Stop(ss_ - v), w: 1.0, tag: "Short", from: "", prot: h[i] + pct * atr10[i] }); }
                }
                if c3 && l[i] < lowest(l, 4, i - 1) { nx.push(o(A::Xl, P::Mkt, "LR turn")); }
                if c1 && h[i] > highest(h, 4, i - 1) { nx.push(o(A::Xs, P::Mkt, "LR turn")); }
            })
        }
        "stad4_cc" => {
            let (nu, nd, al, pct) = (u("nup"), u("ndn"), u("atrlen"), g("pct"));
            let atr = sma(&true_range(x), al);
            let dnc: Vec<bool> = (0..n).map(|i| i > 0 && c[i] < c[i - 1]).collect();
            let upc: Vec<bool> = (0..n).map(|i| i > 0 && c[i] > c[i - 1]).collect();
            let (mut hh, mut ll, mut lc, mut sc) = (0.0, 0.0, 0i64, 0i64);
            run_ts(x, from, to, al.max(nu.max(nd) + 2).max(10), |i, st, _now, nx| {
                if i + 1 >= n { return; }
                let on = x.o[i + 1];
                if mro(&dnc, i - 1, nu, 1) == -1 && c[i] < c[i - 1] { nx.push(o(A::Buy, P::Stop(on + atr[i] * pct), "CC Buy")); hh = highest(h, nu + 1, i); }
                if mro(&upc, i - 1, nd, 1) == -1 && c[i] > c[i - 1] { nx.push(o(A::Sell, P::Stop(on - atr[i] * pct), "CC Sell")); ll = lowest(l, nd + 1, i); }
                if st.pos != 1 { lc = 0; }
                if st.pos != -1 { sc = 0; }
                if st.pos == 1 && on > hh { lc += 1; if lc == 2 { nx.push(o(A::Xl, P::Mkt, "2nd open above")); } }
                if st.pos == -1 && c[i] < ll { sc += 1; if sc == 2 { nx.push(o(A::Xs, P::Mkt, "2nd close below")); } }
                nx.push(o(A::Xs, P::Stop(highest(h, 10, i)), "Protect SX"));
                nx.push(o(A::Xl, P::Stop(lowest(l, 10, i)), "Protect LX"));
            })
        }
        "stad4_momret" => {
            let (px, hx, lx) = (xavg(c, u("xma")), xavg(h, u("len2")), xavg(l, u("len2")));
            let (e3, e10) = (xavg(c, 3), xavg(c, 10));
            let macd: Vec<f64> = (0..n).map(|i| e3[i] - e10[i]).collect();
            let sig = xavg(&macd, 15);
            let (dp, dm, _) = dmi3(x, 15);
            let r5 = crate::connors::rsi(c, 5);
            let atr10 = sma(&true_range(x), 10);
            let (mut f1, mut f2, mut hhi, mut hrsi, mut llo, mut bsw, mut exitp) = (false, false, 0.0, 0.0, 999999.0, false, 0.0f64);
            run_ts(x, from, to, 50, |i, st, _now, nx| {
                let mdiff = macd[i] - sig[i]; let ds = dp[i] - dm[i];
                let trend = px[i] > hx[i] && sig[i] > 0.0;
                let momo = r5[i] >= 70.0 && ds > 15.0;
                if trend && momo {
                    if !f1 { hhi = 0.0; hrsi = 0.0; }
                    f1 = true; f2 = false;
                    if h[i] > hhi { hhi = h[i]; }
                    if r5[i] > hrsi { hrsi = r5[i]; }
                }
                let retr = f1 && px[i] < hx[i] && mdiff < 0.0 && (r5[i] <= 50.0 || r5[i] < hrsi - 30.0);
                if retr { if !f2 { llo = 999999.0; } f2 = true; f1 = false; bsw = false; }
                if f2 { if l[i] < llo { llo = l[i]; } bsw = true; }
                if bsw && st.pos != 1 { nx.push(o(A::Buy, P::Stop(llo + (hhi - llo) / 3.0), "MR Buy")); }
                let cond1 = px[i] < lx[i] || sig[i] < 0.0;
                if st.pos == 1 || cond1 { bsw = false; f1 = false; f2 = false; if cond1 { nx.push(o(A::Xl, P::Mkt, "Trend over")); } }
                if st.pos != 1 { nx.push(o(A::Xl, P::Stop(l[i] - atr10[i]), "Initial stop")); }
                else { nx.push(o(A::Xl, P::Stop(st.entry_px - atr10[i]), "Protective stop")); }
                // literal: SwingLow(1,Low,2,4) returns a price; "= 3" is never true for index prices
                exitp = lx[i].min(exitp);
                if st.pos == 1 { nx.push(o(A::Xl, P::Stop(exitp), "Swing/XMA stop")); }
            })
        }
        "stad4_3xa" => {
            let (a1, a2, a3) = (xavg(c, u("a1")), xavg(c, u("a2")), xavg(c, u("a3")));
            let ar4 = sma(&(0..n).map(|i| h[i] - l[i]).collect::<Vec<_>>(), 4);
            let mut pa = Para::default();
            run_ts(x, from, to, u("a3"), |i, st, _now, nx| {
                if xover(&a1, &a2, i) && a2[i] > a3[i] { nx.push(o(A::Buy, P::Mkt, "3XA Buy")); }
                if xunder(&a1, &a2, i) && a2[i] < a3[i] { nx.push(o(A::Sell, P::Mkt, "3XA Sell")); }
                if a1[i] < a2[i] { nx.push(o(A::Xl, P::Mkt, "Avg1 < Avg2")); }
                if a1[i] > a2[i] { nx.push(o(A::Xs, P::Mkt, "Avg1 > Avg2")); }
                pa.step(x, i, st.pos, ar4[i], 1.0, 0.25, 1.0 / 3.0, nx, "ExitTrade");
            })
        }
        "stad4_dmacv" => {
            let v = vol(x);
            let (pl, pd, vl, vd, ap) = (u("plen"), g("pdisp") as usize, u("vlen"), g("vdisp") as usize, g("atrpct"));
            let (ma, mv) = (sma(c, pl), sma(&v, vl));
            let dma: Vec<f64> = (0..n).map(|i| if i >= pd { ma[i - pd] } else { f64::NAN }).collect();
            let dmav: Vec<f64> = (0..n).map(|i| if i >= vd { mv[i - vd] } else { f64::NAN }).collect();
            let atr10 = sma(&true_range(x), 10);
            run_ts(x, from, to, (pl + pd).max(vl + vd).max(10) + 1, |i, _st, _now, nx| {
                if i + 1 >= n { return; }
                let on = x.o[i + 1];
                if xover(c, &dma, i) && v[i] > dmav[i] { nx.push(o(A::Buy, P::Stop(on + ap * atr10[i]), "DMA Buy")); }
                if xunder(c, &dma, i) && v[i] < dmav[i] { nx.push(o(A::Sell, P::Stop(on - ap * atr10[i]), "DMA Sell")); }
                nx.push(o(A::Xl, P::Stop(lowest(l, 5, i)), "Trail LX"));
                nx.push(o(A::Xs, P::Stop(highest(h, 5, i)), "Trail SX"));
                if xunder(c, &dma, i) { nx.push(o(A::Xl, P::Mkt, "Below DMA")); }
                if xover(c, &dma, i) { nx.push(o(A::Xs, P::Mkt, "Above DMA")); }
            })
        }
        "stad7_vix" => {
            let (Some(vh), Some(vl), Some(vc)) = (x.series("VIX.High"), x.series("VIX.Low"), x.series("VIX.Close")) else { return Some(vec![]) };
            let (ll, hh) = (u("lowl"), u("highh"));
            run_ts(x, from, to, ll.max(hh) + 1, |i, _st, _now, nx| {
                let buy = vh[i] >= highest(vh, hh, i - 1) && vc[i] < vc[i - 1];
                let sell = vl[i] <= lowest(vl, ll, i - 1) && vc[i] > vc[i - 1];
                if buy { nx.push(o(A::Buy, P::Mkt, "Bear key reversal VIX")); }
                if sell { nx.push(o(A::Sell, P::Mkt, "Bull key reversal VIX")); }
            })
        }
        "stad10_mwt" => {
            let ma = sma(c, u("len"));
            let stop = g("stop_usd") / g("pt_usd");
            run_ts(x, from, to, u("len"), |i, st, now, nx| {
                if st.pos != 0 { now.push(o(if st.pos == 1 { A::Xl } else { A::Xs }, P::Mkt, "Exit on close")); }
                if i + 1 < n && ymdw(&x.r[i].date).3 == 4 {
                    // SetStopLoss: entry (next open) -/+ stop points, armed from the entry bar
                    let on = x.o[i + 1];
                    let (d, tag) = if c[i] > ma[i] { (1.0, "MWT Buy") } else { (-1.0, "MWT Sell") };
                    let prot = if stop > 0.0 { on - d * stop } else { f64::NAN };
                    nx.push(O { a: if d > 0.0 { A::Buy } else { A::Sell }, p: P::Mkt, w: 1.0, tag, from: "", prot });
                }
            })
        }
        "stad4_pricedist" => {
            let (ku, sk) = (kurtosis(c, u("klen")), skew(c, u("slen")));
            let zero = vec![0.0; n];
            let ar4 = sma(&(0..n).map(|i| h[i] - l[i]).collect::<Vec<_>>(), 4);
            let mut pa = Para::default();
            run_ts(x, from, to, u("klen").max(u("slen")) + 1, |i, st, now, nx| {
                if xunder(&ku, &zero, i) {
                    if sk[i] > 0.0 { now.push(o(A::Buy, P::Mkt, "Kurt Buy")); }
                    if sk[i] < 0.0 { now.push(o(A::Sell, P::Mkt, "Kurt Sell")); }
                }
                pa.step(x, i, st.pos, ar4[i], 1.0, 1.0 / 3.0, 1.0 / 3.0, nx, "Trail");
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
    fn path_low_first_then_bounce_fills_buy_stop_then_sell_stop() {
        // Vol 5 Figure 6: open nearer the high (O-H-L-C). The buy stop 104 fills on the way up; the 10% bounce
        // (1.0 point) reaches the 103.5 sell stop before the ExitLong limit at 104.8, which is then cancelled.
        let r = rows(&[(100., 101., 99., 100.), (103.8, 105., 95., 104.)]);
        let x = Ctx::new(&r, &r, &[]);
        let ts = run_ts(&x, 0, 1, 0, |i, _st, _now, nx| if i == 0 {
            nx.push(o(A::Buy, P::Stop(104.0), "B")); nx.push(o(A::Sell, P::Stop(103.5), "S")); nx.push(o(A::Xl, P::Lim(104.8), "XL")); });
        assert_eq!(ts.len(), 2);
        assert_eq!((ts[0].dir, ts[0].legs[0].px, ts[0].exit_px), (1.0, 104.0, 103.5));
        assert!(ts[1].open && ts[1].dir == -1.0 && ts[1].legs[0].px == 103.5);
    }
    #[test]
    fn gap_through_stop_fills_at_open_and_flat_issued_exit_protects_entry_bar() {
        let r = rows(&[(100., 101., 99., 100.), (102., 103., 97., 98.)]);
        let x = Ctx::new(&r, &r, &[]);
        let ts = run_ts(&x, 0, 1, 0, |i, _st, _now, nx| if i == 0 {
            nx.push(o(A::Buy, P::Stop(101.0), "B")); nx.push(o(A::Xl, P::Stop(99.0), "X")); });
        assert_eq!(ts.len(), 1);
        assert_eq!((ts[0].legs[0].px, ts[0].exit_px, ts[0].exit_i), (102.0, 99.0, 1));
    }
    #[test]
    fn ids_unique_prefixed_and_every_live_strategy_runs() {
        let cat = catalogue();
        let mut ids: Vec<_> = cat.iter().map(|d| d.id).collect(); ids.sort(); ids.dedup();
        assert_eq!(ids.len(), cat.len());
        let r: Vec<Row> = (0..400).map(|i| { let b = 100.0 + 10.0 * ((i as f64) / 9.0).sin() + (i % 7) as f64;
            Row { ticker: "T".into(), date: format!("{:04}-{:02}-{:02}", 2000 + i / 300, 1 + (i / 25) % 12, 1 + i % 25), open: b, high: b + 2.0, low: b - 2.0, close: b + 0.5, volume: Some(1000.0 + i as f64) } }).collect();
        let x = Ctx::new(&r, &r, &[]);
        for d in cat.iter().filter(|d| d.status == Status::Live) {
            assert!(d.id.starts_with("stad"), "{}", d.id);
            let ts = run(d.id, &x, 60, 399, &Params::new()).unwrap_or_else(|| panic!("{} did not run", d.id));
            for t in &ts { for (k, px) in [(t.legs[0].i, t.legs[0].px), (t.exit_i, t.exit_px)] {
                assert!(px >= x.l[k] - 1e-9 && px <= x.h[k] + 1e-9, "{} fill outside the bar", d.id); } }
            assert!(family_of(d.id).is_some());
        }
    }
}
