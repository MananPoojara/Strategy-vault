//! Quadriga Superfund trendline system for TS7 (superfund.com), recovered from
//! `T:/Quadriga Superfund System for TS7 (superfund.com)/Quadriga_Superfund.rar` -> `Quadriga_Superfund.txt` (research drive, read in place).
//! Book id "qsfund". Helpers from jcfeed. Notes: library/qsfund/README.md.
use super::jcfeed::{gp, stops_into, Ord, Sim};
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "qsfund";
const QS: &str = "qsfund_trendline";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: QS, name: "Quadriga Superfund trendline breakout (stop and reverse)", group: "Quadriga Superfund (Quadriga_Superfund.rar)", session: 0, dir: "Long/Short",
        status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules: &[
            "F|Pivot low: \"If LOW[R] < LOWEST(TrueLow, R)\" and, past equal lows (\"WHILE LOW[R] = LOW[R+E]\"), \"LOW[R] < LOWEST(TrueLow, R)[R+E]\" -> push LOW[R] (STRENGTH R 3; 20 pivots kept); pivot highs mirrored with TrueHigh",
            "F|Support line: from the newest pivot low back to the most recent older pivot low below it; ULINE = that line projected to the next bar; resistance LINE from the newest pivot high and the most recent older higher pivot high",
            "E|\"QBUY = 2*CLOSE - TrueLow < LINE; If QBUY and MARKETPOSITION < 1 and (CLOSE < LINE OR HIGH < LINE - SLOPE) then BUY NEXT BAR LINE STOP\"",
            "E|\"QSELL = 2*CLOSE - TrueHigh > ULINE; If QSELL and MARKETPOSITION > -1 and (CLOSE > ULINE OR LOW > ULINE - SLOPE) then SELL SHORT NEXT BAR ULINE STOP\" (SLOPE = the resistance slope, as written)",
            "X|No exit order: an opposite entry stop reverses",
        ],
        notes: "Code input STRENGTH 3 is the default. The short condition uses SLOPE (the resistance-line slope from the previous bar's calculation), exactly as written; the support slope USLOPE is not used there. \
TrueLow = min(L, C[1]), TrueHigh = max(H, C[1]). Timeframe and market are not stated (only the .ELD and the code); Superfund trades a futures portfolio on daily bars and the code has no intraday construct, so it is run on daily bars [Assumption]. \
Fills [Assumption, code silent - TradeStation daily-bar path, as rarsys]: a stop fills at its level or at the open if the open is through it; when flat both stops can fill on one bar. \
India: instrument swap; index shorts hypothetical.",
        claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == QS).then_some((QS, "Quadriga Superfund trendline", "code literal"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    (id == QS).then(|| vec![ParamDef { key: "strength", label: "STRENGTH (pivot bars)", def: 3., min: 1., max: 10., step: 1. }])
}

struct Piv { p: [f64; 20], bar: [f64; 20] }

impl Piv {
    fn push(&mut self, v: f64, r: f64) {
        for z in (1..20).rev() { self.p[z] = self.p[z - 1]; self.bar[z] = self.bar[z - 1]; }
        self.p[0] = v; self.bar[0] = r;
    }
    /// (point1, point1bar) = first older pivot that is below (low side) / above (high side) the newest.
    fn older(&self, low: bool) -> Option<(f64, f64)> {
        (1..20).find(|&y| if low { self.p[y] < self.p[0] } else { self.p[y] > self.p[0] }).map(|y| (self.p[y], self.bar[y]))
    }
}

fn run_qs(x: &Ctx, from: usize, to: usize, p: &Params) -> Vec<Trade> {
    let r = (gp(p, "strength", 3.) as usize).max(1);
    let n = x.c.len();
    let tl: Vec<f64> = (0..n).map(|k| if k == 0 { f64::NAN } else { x.l[k].min(x.c[k - 1]) }).collect();
    let th: Vec<f64> = (0..n).map(|k| if k == 0 { f64::NAN } else { x.h[k].max(x.c[k - 1]) }).collect();
    // lowest/highest of v over n bars ending at k (all must be defined)
    let ex = |v: &[f64], k: isize, hi: bool| -> Option<f64> {
        if k - (r as isize) + 1 < 1 { return None; }
        let w = &v[(k as usize + 1 - r)..=k as usize];
        Some(if hi { w.iter().copied().fold(f64::MIN, f64::max) } else { w.iter().copied().fold(f64::MAX, f64::min) })
    };
    let (mut lo, mut hi) = (Piv { p: [0.; 20], bar: [0.; 20] }, Piv { p: [0.; 20], bar: [0.; 20] });
    let mut slope = 0f64;
    let mut sim = Sim::new();
    for i in 1..to.min(n - 1) {
        for z in 0..20 { lo.bar[z] += 1.; hi.bar[z] += 1.; }
        let mut ords = vec![];
        let ri = i as isize - r as isize;
        // ---- pivot lows / support line / sell short
        if ri >= 0 {
            if let Some(m) = ex(&tl, i as isize, false) {
                let v = x.l[ri as usize];
                if v < m {
                    let mut e = 1isize;
                    while ri - e >= 0 && v == x.l[(ri - e) as usize] { e += 1; }
                    if let Some(m2) = ex(&tl, ri - e, false) { if v < m2 { lo.push(v, r as f64); } }
                }
            }
        }
        let (p2, p2bar) = (lo.p[0], lo.bar[0]);
        if let Some((p1, p1bar)) = lo.older(true) {
            if p1 > 0. && p1bar > p2bar {
                let us = (p2 - p1) / (p1bar - p2bar);
                let mut uline = p1;
                for _ in 0..=(p1bar as usize) { uline += us; } // "FOR W = LPOINT1BAR DOWNTO 0: ULINE = ULINE + USLOPE"
                let qsell = 2. * x.c[i] - th[i] > uline;
                if qsell && sim.pos() > -1 && (x.c[i] > uline || x.l[i] > uline - slope) { ords.push((uline, Ord::SellE, "Sell stop")); }
            }
        }
        // ---- pivot highs / resistance line / buy
        if ri >= 0 {
            if let Some(m) = ex(&th, i as isize, true) {
                let v = x.h[ri as usize];
                if v > m {
                    let mut e = 1isize;
                    while ri - e >= 0 && v == x.h[(ri - e) as usize] { e += 1; }
                    if let Some(m2) = ex(&th, ri - e, true) { if v > m2 { hi.push(v, r as f64); } }
                }
            }
        }
        let (q2, q2bar) = (hi.p[0], hi.bar[0]);
        if let Some((q1, q1bar)) = hi.older(false) {
            if q1 > 0. && q1bar > q2bar {
                slope = (q2 - q1) / (q1bar - q2bar);
                let mut line = q1;
                for _ in 0..=(q1bar as usize) { line += slope; }
                let qbuy = 2. * x.c[i] - tl[i] < line;
                if qbuy && sim.pos() < 1 && (x.c[i] < line || x.h[i] < line - slope) { ords.push((line, Ord::BuyE, "Buy stop")); }
            }
        }
        if i >= from && !ords.is_empty() { stops_into(&mut sim, x, i + 1, &ords); }
    }
    sim.finish(x, to)
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if id != QS || from >= to || to >= x.r.len() { return None; }
    Some(run_qs(x, from, to, p))
}
