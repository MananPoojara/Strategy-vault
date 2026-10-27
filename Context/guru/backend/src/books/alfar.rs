//! alfaranda.com TradeStation systems recovered from RARs (read with the read-only libarchive reader):
//! - Alfanet Basis II: `T:/Alfanet Basis II Trading System for TS (alfaranda.com)/AlfanetBasis2(openCode).rar`
//!   -> `AlfanetBasis2_openCode_.txt` (TradeStation 2000i code);
//! - Gear: `T:/Gear System for TS (alfaranda.com)/Open Code.rar` (catalogued: needs the XAU index as data2).
//! Avanti (`T:/Avanti System for TS (alfaranda.com)/Open Code.rar`) is the Dennis Meyers Polychromatic Momentum system
//! already in the vault as dmst2_polychrom (same code and defaults) and is not repeated.
//! (`T:` = research drive `USB HDD 1/TradeStation Systems & Indicators`.) Book id "alfar". Notes: library/alfar/README.md.
use crate::books::ajmisc::{ord, true_high, true_low, Sim, Ty, K};
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "alfar";
/// Admitted ids (authors/lanea/vendor1/decisions.json).
pub const REVIEWED_LANEA: &[&str] = &["alfar_basis2"];
pub fn reviewed_lanea(id: &str) -> bool { REVIEWED_LANEA.contains(&id) }
const AB2: &str = "alfar_basis2";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, status, rules, notes| StrategyDef { id, name, group, session: 0, dir: "Long/Short", status, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None };
    vec![
        d(AB2, "Alfanet Basis II (pivot trendline breakout, stop and reverse)", "Alfanet Basis II (openCode)", Status::Live, &[
            "F|Pivot low (STRENGTH 2): \"IF L[R] < LOWEST(TrueLow,R)\" and, after skipping equal lows to the left, \"IF L[R] < LOWEST(TrueLow,R)[R+E]\" -> pushed into a 20-deep list LP with its bars-ago LPBAR; pivot highs mirror with TrueHigh",
            "F|Support line through the latest pivot low LP[0] and the most recent earlier pivot low below it (LPOINT1): USLOPE = (LPOINT2-LPOINT1)/(LPOINT1BAR-LPOINT2BAR); ULINE = LPOINT1 advanced LPOINT1BAR+1 bars (tomorrow's value)",
            "E|\"QSELL = 2*C-TrueHigh > ULINE; IF QSELL AND MARKETPOSITION > -1 AND (C > ULINE OR L > ULINE-SLOPE) THEN SELL(\"Q1 LINE Sell\") ULINE STOP\"",
            "F|Resistance line through the latest pivot high and the most recent earlier higher pivot high: SLOPE, LINE (tomorrow's value)",
            "E|\"QBUY = 2*C-TrueLow < LINE; IF QBUY AND MARKETPOSITION < 1 AND (C < LINE OR H < LINE-SLOPE) THEN BUY(\"Q1 LINE Buy\") LINE STOP\"",
            "X|Stop and reverse (TS2000i: SELL opens a short and closes a long); no other exit in the code."],
            "Code only (no manual in the folder; no bar size or market named): run on daily bars of the cash indices (instrument swap). Literal details kept: the sell test uses 'L > ULINE-SLOPE' with SLOPE, the resistance-line slope left over from the previous bar (the buy test uses the current SLOPE); \
the line is advanced by repeated addition exactly as the FOR loop does; equal-low skipping looks left without limit. TrueLow/TrueHigh include the prior close. Fills: vault TradeStation daily-bar convention (ajmisc.rs header). India: instrument swap; index shorts hypothetical."),
        d("alfar_gear", "Gear (XAU momentum + close trailing reversal, walk-forward inputs)", "Gear (Open Code)", Status::NeedsData("data2 = 'xau(C OF DATA2)': daily closes of the PHLX Gold & Silver index (XAU) aligned to the traded market; not in the vault"), &[
            "F|\"if xau > xau[jlagx]*oneplus then position=2\"; \"if xau < xau[jlagx]*oneminus then position=-2\" (bjmp/sjmp/jlag switched by date: 1990-1996 walk-forward inputs)",
            "E|\"if marketposition <= 0 and position=-2 and bopos=2 then Buy This Bar on close\"; \"if marketposition >= 0 and position=2 and bopos=-2 then Sell Short This Bar on close\" (bopos = close trailing fup/fdn % reversal)"], "Contrarian to the gold-index move. Coded nowhere until an XAU series exists."),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id { AB2 => ("alfar_basis2", "Alfanet Basis II (alfaranda)", "code"), "alfar_gear" => ("alfar_gear", "Gear (alfaranda)", "code"), _ => return None })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    (id == AB2).then(|| vec![ParamDef { key: "strength", label: "STRENGTH (pivot bars)", def: 2., min: 1., max: 6., step: 1. }])
}

/// One pivot side of the code. `low` = support (pivot lows); returns the list update for bar i.
struct Piv { p: [f64; 20], bar: [i64; 20] }

impl Piv {
    fn step(&mut self, x: &Ctx, tv: &[f64], i: usize, r: usize, low: bool) {
        for z in 0..20 { self.bar[z] += 1; }
        let px = if low { &x.l } else { &x.h };
        let beyond = |a: f64, b: f64| if low { a < b } else { a > b };
        // LOWEST(TrueLow,R)[k] = extreme of tv over bars k..k+R-1 ago
        let ext = |k: usize| -> Option<f64> {
            if i < k + r - 1 { return None; }
            let w = &tv[i + 1 - k - r..=i - k];
            Some(if low { w.iter().copied().fold(f64::MAX, f64::min) } else { w.iter().copied().fold(f64::MIN, f64::max) })
        };
        if i < r { return; }
        let v = px[i - r];
        if let Some(e0) = ext(0) {
            if beyond(v, e0) {
                let mut e = 1;
                while i >= r + e && px[i - r - e] == v { e += 1; }
                if let Some(e1) = ext(r + e) {
                    if beyond(v, e1) {
                        for z in (1..20).rev() { self.p[z] = self.p[z - 1]; self.bar[z] = self.bar[z - 1]; }
                        self.p[0] = v; self.bar[0] = r as i64;
                    }
                }
            }
        }
    }
    /// (point1, bar1, point2, bar2) when a line exists.
    fn line(&self, low: bool, last_bar1: &mut i64) -> Option<(f64, i64, f64, i64)> {
        let (p2, b2) = (self.p[0], self.bar[0]);
        let mut p1 = 0.;
        let mut y = 1;
        while p1 == 0. && y < 20 {
            if if low { self.p[y] < self.p[0] } else { self.p[y] > self.p[0] } { p1 = self.p[y]; *last_bar1 = self.bar[y]; }
            y += 1;
        }
        (p1 > 0. && *last_bar1 > b2).then_some((p1, *last_bar1, p2, b2))
    }
}

fn basis2(x: &Ctx, from: usize, to: usize, p: &Params) -> Vec<Trade> {
    let r = p.get("strength").copied().unwrap_or(2.).max(1.) as usize;
    let (th, tl) = (true_high(x), true_low(x));
    let mut lo = Piv { p: [0.; 20], bar: [0; 20] };
    let mut hi = Piv { p: [0.; 20], bar: [0; 20] };
    let (mut slope, mut b1l, mut b1h) = (0f64, 0i64, 0i64);
    let mut s = Sim::new();
    for i in from.max(1)..to {
        let c = x.c[i];
        let mut q = vec![];
        lo.step(x, &tl, i, r, true);
        if let Some((p1, b1, p2, b2)) = lo.line(true, &mut b1l) {
            let us = (p2 - p1) / (b1 - b2) as f64;
            let mut ul = p1;
            for _ in 0..=b1 { ul += us; }
            let qsell = 2. * c - th[i] > ul;
            if qsell && s.pos > -1 && (c > ul || x.l[i] > ul - slope) { q.push(ord(K::Short, Ty::Stop(ul), "Q1 LINE Sell")); }
        }
        hi.step(x, &th, i, r, false);
        if let Some((p1, b1, p2, b2)) = hi.line(false, &mut b1h) {
            slope = (p2 - p1) / (b1 - b2) as f64;
            let mut line = p1;
            for _ in 0..=b1 { line += slope; }
            let qbuy = 2. * c - tl[i] < line;
            if qbuy && s.pos < 1 && (c < line || x.h[i] < line - slope) { q.push(ord(K::Buy, Ty::Stop(line), "Q1 LINE Buy")); }
        }
        s.bar(x, i + 1, &q);
    }
    s.done(x, to)
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if id != AB2 || from >= to || to >= x.r.len() { return None; }
    Some(basis2(x, from, to, p))
}
