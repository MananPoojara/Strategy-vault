//! Dennis Meyers, "Advanced Short Term Systems & Indicators" v1.01 User's Manual (ASTMAN10, Meyers Analytics
//! L.L.C., 1998-99). Book id `dmast`. Source: research drive ".../Dennis Meyers Advanced Short-Term Systems &
//! Indicators 1.01 (meyersanalytics.com)/ASTMAN10.PDF" (91 pp.), copy in Books/Dennis Meyers/.
//! Transcription, page refs, oracle and verification: library/dmast/.
//!
//! Every system is printed as complete TradeStation 4 EasyLanguage; that code is the rule and wins over the
//! prose (defaults = the code's `input:` values; where the "System Input Parameters" prose prints other numbers
//! they are noted). All systems are stop-and-reverse (always in the market after the first signal).
//! TS4 conventions reproduced exactly: `at market` = next bar's open, `on close` = this bar's close; an order in
//! the direction already held is ignored; series references before the first computed bar return the variable's
//! value on the first bar. The book traded one futures contract (BP, JY, US bonds, gold, S&P 500); here one unit
//! of the selected index (instrument swap only). The study runs from the first bar of the data after the book's
//! stated "days of past data" (MaxBarsBack); positions taken before the selected start date are not reported, so
//! the first reported trade is the first reversal inside the window. The book's MARK input (exit on the last bar)
//! is the engine's open-trade mark. Walk-forward ("WF") parameter splicing is not a separate rule set; every
//! table row's parameters can be entered as parameters.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "dmast";

const FMP2_GAP: &str = "IBM 5-minute intraday bars (June 1999 onward) — the vault has daily bars only. The rule is the Fixed Memory Polynomial code applied to 5-minute bars with a 4-week walk-forward; it cannot be run on daily data without changing the bar size the book specifies.";

// ---------------------------------------------------------------------------------------------------------
// TradeStation-4 order engine (one contract, stop-and-reverse)
#[derive(Clone, Copy, PartialEq, Debug)]
enum K { Mkt, Stop, Close }
#[derive(Clone, Copy, Debug)]
struct O { d: i8, k: K, px: f64, tag: &'static str }
fn mkt(d: i8, tag: &'static str) -> O { O { d, k: K::Mkt, px: 0., tag } }
fn on_close(d: i8, tag: &'static str) -> O { O { d, k: K::Close, px: 0., tag } }

/// `step(i, cb, mp)` runs the system code on bar `i` (`cb` = TS CurrentBar, 1 on the first bar after
/// MaxBarsBack; `mp` = MarketPosition after today's open/stop fills) and returns its orders: market/stop orders
/// act on bar i+1, close orders at close[i]. Market and stop orders in the direction already held are ignored;
/// when several opposite orders could fill on one bar (only possible while flat), the last market order wins,
/// else the stop whose fill is nearest the open.
fn engine(x: &Ctx, from: usize, to: usize, mbb: usize, mut step: impl FnMut(usize, usize, i8) -> Vec<O>) -> Vec<Trade> {
    let mut out = vec![];
    let mut cur: Option<Trade> = None;
    let mut pos: i8 = 0;
    let mut pending: Vec<O> = vec![];
    let flip = |d: i8, i: usize, px: f64, k: K, tag: &str, pos: &mut i8, cur: &mut Option<Trade>, out: &mut Vec<Trade>| {
        let sig = if k == K::Mkt { i - 1 } else { i };
        if let Some(mut t) = cur.take() {
            t.exit_i = i; t.exit_px = px; t.signal_i = sig;
            t.reason = format!("Reverse: {tag} ({})", match k { K::Mkt => "next open", K::Stop => "stop", K::Close => "on close" });
            out.push(t);
        }
        *pos = d;
        if i >= from {
            *cur = Some(Trade { dir: d as f64, legs: vec![Leg { i, px, w: 1. }], exit_i: i, exit_px: px, reason: String::new(),
                entry_kind: match k { K::Mkt => "open", K::Stop => "stop", K::Close => "close" }, signal_i: sig, open: false, exits: vec![] });
        }
    };
    if mbb >= x.c.len() { return out; }
    for i in mbb..=to {
        if i > mbb && !pending.is_empty() {
            let opp: Vec<O> = pending.iter().copied().filter(|o| o.d != pos).collect();
            if let Some(o) = opp.iter().rev().find(|o| o.k == K::Mkt) {
                flip(o.d, i, x.o[i], K::Mkt, o.tag, &mut pos, &mut cur, &mut out);
            } else {
                let mut best: Option<(f64, O, f64)> = None;
                for o in opp.iter().filter(|o| o.k == K::Stop) {
                    let hit = if o.d > 0 { x.h[i] >= o.px } else { x.l[i] <= o.px };
                    if !hit { continue; }
                    let fill = if o.d > 0 { x.o[i].max(o.px) } else { x.o[i].min(o.px) };
                    let dist = (fill - x.o[i]).abs();
                    if best.map_or(true, |b| dist < b.0) { best = Some((dist, *o, fill)); }
                }
                if let Some((_, o, fill)) = best { flip(o.d, i, fill, K::Stop, o.tag, &mut pos, &mut cur, &mut out); }
            }
        }
        let ords = step(i, i - mbb + 1, pos);
        if let Some(o) = ords.iter().rev().find(|o| o.k == K::Close && o.d != pos) {
            flip(o.d, i, x.c[i], K::Close, o.tag, &mut pos, &mut cur, &mut out);
        }
        pending = ords.into_iter().filter(|o| o.k != K::Close).collect();
    }
    if let Some(mut t) = cur { t.exit_i = to; t.exit_px = x.c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true; out.push(t); }
    out
}

/// Meyers' "UP DWN %CHG from previous local high or low" block (bopos), shared by every system here.
/// `mul`: thresholds are percentages of the curve (h_ep*oneminus); otherwise points (h_ep-oneminus).
#[derive(Clone, Copy)]
struct Swing { mul: bool, up: f64, dn: f64, h_ep: f64, l_ep: f64, buystop: f64, sellstop: f64, bopos: i8, prev: i8 }
impl Swing {
    fn new(mul: bool, up: f64, dn: f64, x0: f64) -> Self {
        let mut s = Swing { mul, up, dn, h_ep: x0, l_ep: x0, buystop: 0., sellstop: 0., bopos: 0, prev: 0 };
        s.buystop = s.bs(x0); s.sellstop = s.ss(x0); s
    }
    fn bs(&self, v: f64) -> f64 { if self.mul { v * self.up } else { v + self.up } }
    fn ss(&self, v: f64) -> f64 { if self.mul { v * self.dn } else { v - self.dn } }
    /// `cond` = extra price confirmation (Some((prc, xplus[1])) for the polynomial systems).
    fn update(&mut self, xp: f64, cond: Option<(f64, f64)>) {
        self.prev = self.bopos;
        let p1 = self.prev;
        if p1 >= 0 {
            if xp > self.h_ep { self.h_ep = xp; self.sellstop = self.ss(xp); }
            if xp <= self.sellstop && cond.map_or(true, |(p, x1)| p < x1) { self.bopos = -1; self.l_ep = xp; self.buystop = self.bs(xp); }
        }
        if p1 <= 0 {
            if xp < self.l_ep { self.l_ep = xp; self.buystop = self.bs(xp); }
            if xp >= self.buystop && cond.map_or(true, |(p, x1)| p > x1) { self.bopos = 1; self.h_ep = xp; self.sellstop = self.ss(xp); }
        }
    }
    fn turned(&self, d: i8) -> bool { self.bopos == d && self.prev != d }
    fn force(&mut self, d: i8, xp: f64) {
        self.bopos = d;
        if d > 0 { self.h_ep = xp; self.sellstop = self.ss(xp); } else { self.l_ep = xp; self.buystop = self.bs(xp); }
    }
}

/// Least-squares polynomial (degree `pw`) through the last `n` closes, evaluated one bar ahead (t = n):
/// the book's normalized discrete Legendre (Morrison) solution. Returns weights w[k], k = 0 oldest .. n-1 today.
fn ls_forecast_weights(n: usize, pw: usize) -> Vec<f64> {
    let s = (n as f64 - 1.) / 2.;
    let t: Vec<f64> = (0..=n).map(|k| (k as f64 - s) / s.max(1.)).collect();
    // modified Gram-Schmidt on monomials over the sample points 0..n-1, carrying the value at point n
    let mut basis: Vec<(Vec<f64>, f64)> = vec![];
    for d in 0..=pw.min(n - 1) {
        let mut v: Vec<f64> = t[..n].iter().map(|z| z.powi(d as i32)).collect();
        let mut vn = t[n].powi(d as i32);
        for (q, qn) in &basis {
            let dot: f64 = v.iter().zip(q).map(|(a, b)| a * b).sum();
            for (a, b) in v.iter_mut().zip(q) { *a -= dot * b; }
            vn -= dot * qn;
        }
        let norm = v.iter().map(|a| a * a).sum::<f64>().sqrt();
        basis.push((v.iter().map(|a| a / norm).collect(), vn / norm));
    }
    (0..n).map(|k| basis.iter().map(|(q, qn)| q[k] * qn).sum()).collect()
}

// ---------------------------------------------------------------------------------------------------------
type PDef = (&'static str, &'static str, f64, f64, f64, f64);
struct S { id: &'static str, name: &'static str, group: &'static str, status: Status, rules: &'static [&'static str], notes: &'static str, defs: &'static [PDef], fam: (&'static str, &'static str, &'static str) }

const FMP: &[PDef] = &[
    ("pw", "Polynomial order pw (2 quadratic, 3 cubic … max 6)", 3., 1., 6., 1.),
    ("n", "N, closes in the least-squares fit", 40., 10., 60., 1.),
    ("pctup", "pctup, % rise of the curve from its low while short", 0.2, 0.05, 3., 0.05),
    ("pctdn", "pctdn, % fall of the curve from its high while long", 1.7, 0.05, 3., 0.05),
    ("pctjup", "pctjup, % the curve must exceed its value 2 days ago (jump buy)", 0.15, 0.05, 2., 0.05),
    ("pctjdn", "pctjdn, % the curve must fall below its value 2 days ago (jump sell)", 0.7, 0.05, 2., 0.05),
];
const FAD1: &[PDef] = &[
    ("pctsw", "pctsw: 0 = Tosc in points, 1 = Tosc in percent of the EMA", 0., 0., 1., 1.),
    ("degree", "Polynomial degree (1 line … 6)", 1., 1., 6., 1.),
    ("ndays", "NDays, alpha = 2/(1+NDays)", 25., 5., 60., 1.),
    ("dup", "dup, Tosc level for a buy", 0.325, 0.025, 3., 0.025),
    ("ddn", "ddn, Tosc level (negative) for a sell", 0.525, 0.025, 3., 0.025),
];
const FAD2: &[PDef] = &[
    ("pctsw", "pctsw: 0 = Tosc in points, 1 = Tosc in percent of the EMA", 1., 0., 1., 1.),
    ("degree", "Polynomial degree (1 line … 6)", 2., 1., 6., 1.),
    ("ndays", "NDays, alpha = 2/(1+NDays)", 15., 5., 60., 1.),
    ("delup", "delup, rise of Tosc from its low while short", 1.75, 0.05, 6., 0.05),
    ("deldn", "deldn, fall of Tosc from its high while long", 3.0, 0.05, 6., 0.05),
];
const KGH: &[PDef] = &[
    ("theta", "theta, Kalman decay constant (g = 1-θ², h = (1-θ)²)", 0.89, 0.5, 0.99, 0.01),
    ("delup", "delup, rise of dxnorm from its low while short", 0.135, 0.005, 0.5, 0.005),
    ("deldn", "deldn, fall of dxnorm from its high while long", 0.055, 0.005, 0.5, 0.005),
];
const KGHK: &[PDef] = &[
    ("gn", "gn, Kalman g-h-k position weight", 0.135, 0.01, 0.5, 0.005),
    ("delup", "delup, rise of dxnorm from its low while short", 0.02, 0.005, 0.5, 0.005),
    ("deldn", "deldn, fall of dxnorm from its high while long", 0.095, 0.005, 0.5, 0.005),
];
const KFM: &[PDef] = &[
    ("lam", "Lam (λ), discount factor on the Kalman gain", 0.75, 0.5, 0.99, 0.01),
    ("ralf", "ralf (α), exponential weight for the noise estimate r", 0.2, 0.05, 1., 0.05),
    ("pctup", "pctup, % rise of the filter from its low while short", 1.4, 0.05, 3., 0.05),
    ("pctdn", "pctdn, % fall of the filter from its high while long", 1.0, 0.05, 3., 0.05),
];

fn table() -> Vec<S> {
    use Status::*;
    vec![
        S { id: "dmast_fixmempoly", name: "Fixed Memory Polynomial (least-squares cubic) system", group: "III · Fixed Memory Polynomial", status: Live,
            rules: &[
                "F|PF = least-squares polynomial of order pw fitted to the last N closes, forecast for tomorrow (normalized discrete Legendre solution). (pp. III-1–III-2, code III-11)",
                "E|Buy at tomorrow's open if PF has moved up by more than pctup % from the lowest PF recorded while short (code: PF >= low*(1+pctup/100) and close > PF[1]).",
                "E|Or buy at tomorrow's open if PF > PF two days ago × (1+pctjup/100).",
                "E|Sell (reverse to short) at tomorrow's open if PF has moved down by more than pctdn % from the highest PF recorded while long (code: PF <= high*(1-pctdn/100) and close < PF[1]); or if PF < PF two days ago × (1-pctjdn/100).",
                "X|Stop and reverse: always in the market; each opposite signal exits and reverses. No other stop or target.",
            ],
            notes: "Code defaults (III-11) pw=3, N=40, pctup=0.2, pctdn=1.7, pctjup=0.15, pctjdn=0.7 = the last walk-forward row (1/1/93-12/31/97). The prose list (III-3) prints pctup 0.4, pctdn 1.1, pctjup 0.20, pctjdn 1.10; the code is used. Book market: British Pound futures 1989-1998 (129 trades, 54% profitable, PF 2.12 incl. walk-forward splice). Run here on the selected index (instrument swap). The book's 50-bar MaxBarsBack is kept.",
            defs: FMP, fam: ("dmast_poly", "Meyers polynomial trend systems", "Fixed memory (least squares)") },
        S { id: "dmast_fadmem1", name: "Fading Memory Polynomial I (Tosc level) system", group: "IV · Fading Memory Polynomial I", status: Live,
            rules: &[
                "F|b0(t)=(1-α)b0(t-1)+x(t); bk(t)=(1-α)bk(t-1)+bk-1(t)-bk-1(t-1); Xnest(t+1)=α·Σbk(t), k=0..degree; X0est = EMA with α=2/(1+NDays). (pp. IV-1–IV-2)",
                "F|Tosc = Xnest(t+1) - X0est (pctsw=0) or 100·(Xnest(t+1)-X0est)/X0est (pctsw=1).",
                "E|Buy on the close if Tosc is greater than dup.",
                "E|Sell (reverse to short) on the close if Tosc is less than -ddn.",
                "X|Stop and reverse; no other exit.",
            ],
            notes: "Code IV-10 defaults: pctsw=0, degree=1, NDays=25, dup=0.325, ddn=0.525 (delup/deldn=99 = disabled). Book market: Japanese Yen futures 1988-1998 (24 trades, 66.7% win, PF 8.47). pctsw=0 measures Tosc in price points, so on an index the default levels are tiny relative to price; this is the book's literal setting (the book recommends pctsw=1 for wide-ranging markets such as the S&P 500 — available as a parameter). MaxBarsBack 40 as the book states.",
            defs: FAD1, fam: ("dmast_poly", "Meyers polynomial trend systems", "Fading memory I (level)") },
        S { id: "dmast_fadmem2", name: "Fading Memory Polynomial II (Tosc swing) system", group: "V · Fading Memory Polynomial II", status: Live,
            rules: &[
                "F|Same recursive polynomial oscillator Tosc as system I (pctsw=1: percent of the EMA). (p. V-1)",
                "E|Buy on the close if Tosc has moved up by more than delup from the lowest Tosc recorded while short.",
                "E|Sell (reverse to short) on the close if Tosc has moved down by more than deldn from the highest Tosc recorded while long.",
                "X|Stop and reverse; no other exit.",
            ],
            notes: "Code V-8 defaults: pctsw=1, degree=2, NDays=15, delup=1.75, deldn=3.00 (dup/ddn=99 = disabled). Book market: US Bond futures 1988-1998 (51 trades, 60.8% win, PF 3.48). MaxBarsBack 40.",
            defs: FAD2, fam: ("dmast_poly", "Meyers polynomial trend systems", "Fading memory II (swing)") },
        S { id: "dmast_kalman_gh", name: "Kalman g-h Filter system", group: "VI · Kalman g-h Filter", status: Live,
            rules: &[
                "F|Velocity dx = dx[1] + h·err, estimate xs = xs[1] + dx + g·err, err = close - xs[1]; g = 1-θ², h = (1-θ)². dxnorm = 100·dx/close. (pp. VI-1–VI-2, code VI-9)",
                "E|Buy at tomorrow's open if dxnorm has moved up by more than delup from the lowest dxnorm recorded while short.",
                "E|Sell (reverse to short) at tomorrow's open if dxnorm has moved down by more than deldn from the highest dxnorm recorded while long.",
                "X|Stop and reverse; no other exit.",
            ],
            notes: "Code defaults θ=0.89, delup=0.135, deldn=0.055 (= prose VI-2 and the last walk-forward row 1/1/95-12/31/97). Book market: Gold futures 1989-1998. TS4 default MaxBarsBack (50) assumed; the code needs only 2 bars.",
            defs: KGH, fam: ("dmast_kalman", "Meyers Kalman filter systems", "g-h (velocity)") },
        S { id: "dmast_kalman_ghk", name: "Kalman g-h-k Filter system", group: "VII · Kalman g-h-k Filter", status: Live,
            rules: &[
                "F|Acceleration ddxs = ddxs[1] + 2k·err; velocity dxs = dxs[1] + ddxs + h·err; xs = xs[1] + dxs + ddxs/2 + g·err; h = 2(2-g) - 4√(1-g), k = h²/(4g); dxnorm = 100·dxs/close. (pp. VII-1–VII-2, code VII-9)",
                "E|Buy at tomorrow's open if dxnorm has moved up by more than delup from the lowest dxnorm recorded while short.",
                "E|Sell (reverse to short) at tomorrow's open if dxnorm has moved down by more than deldn from the highest dxnorm recorded while long.",
                "X|Stop and reverse; no other exit. (dxup/dxdn level entries = 99, disabled, as printed.)",
            ],
            notes: "Code defaults gn=0.135, delup=0.020, deldn=0.095; the prose list (VII-3) prints gn 0.12, delup 0.025, deldn 0.150 — the code is used. Book market: S&P 500 futures 1989-1998 (results in the extraction; compared on SPX cash in the README). TS4 default MaxBarsBack (50) assumed.",
            defs: KGHK, fam: ("dmast_kalman", "Meyers Kalman filter systems", "g-h-k (acceleration)") },
        S { id: "dmast_kalman_fmem", name: "Kalman Fading Memory Filter system", group: "VIII · Kalman Fading Memory Filter", status: Live,
            rules: &[
                "F|Fading-memory Kalman filter: k = p⁻/(p⁻ + λ·r̄), x̂ = x̂⁻ + k(y - x̂⁻), p = (1-k)p⁻/λ, r̄ = r̄[1] + α((y-x̂⁻)² - r̄[1]), x̂⁻ = x̂ (φ = h = 1). (pp. VIII-1–VIII-2, code VIII-10)",
                "E|Buy at tomorrow's open if the filter has moved up by more than pctup % from its lowest value recorded while short.",
                "E|Sell (reverse to short) at tomorrow's open if the filter has moved down by more than pctdn % from its highest value recorded while long.",
                "X|Stop and reverse; no other exit. (2-day jump entries pctjup/pctjdn = 99, disabled, as printed.)",
            ],
            notes: "Code defaults Lam=0.75, ralf=0.20, pctup=1.4, pctdn=1.00 (= prose VIII-3). Book market: US Bond futures 1988-1998. TS4 default MaxBarsBack (50) assumed.",
            defs: KFM, fam: ("dmast_kalman", "Meyers Kalman filter systems", "Fading-memory Kalman") },
        S { id: "dmast_fixmempoly2_ibm5m", name: "Fixed Memory Polynomial II (IBM 5-minute bars)", group: "IX · Fixed Memory Polynomial II", status: NeedsData(FMP2_GAP),
            rules: &[
                "F|Pf = 4th-order least-squares polynomial through the last N=80 five-minute bars, forecast one bar ahead. (pp. IX-1–IX-2)",
                "E|Buy IBM at the market if Pf has moved up by more than pctup=0.05 % from the lowest Pf recorded while short.",
                "E|Sell IBM at the market if Pf has moved down by more than pctdn=0.35 % from the highest Pf recorded while long.",
                "X|Stop and reverse on 5-minute bars; 4-week optimisation / 1-week walk-forward.",
            ],
            notes: "Same code as the Fixed Memory Polynomial system, specified for IBM 5-minute bars (parameters pw=4, N=80, pctup=0.05, pctdn=0.35). Needs intraday bars; the daily version is dmast_fixmempoly.",
            defs: &[], fam: ("dmast_poly", "Meyers polynomial trend systems", "Fixed memory II · IBM 5-min (data missing)") },
    ]
}

pub fn catalogue() -> Vec<StrategyDef> {
    table().into_iter().map(|s| StrategyDef { id: s.id, name: s.name, group: s.group, session: 0, dir: "Long/Short", status: s.status,
        needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules: s.rules, notes: s.notes, claim: None }).collect()
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { table().into_iter().find(|s| s.id == id).map(|s| s.fam) }
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let s = table().into_iter().find(|s| s.id == id && matches!(s.status, Status::Live))?;
    Some(s.defs.iter().map(|&(key, label, def, min, max, step)| ParamDef { key, label, def, min, max, step }).collect())
}
const INTS: &[&str] = &["pw", "n", "degree", "pctsw"];
fn get(id: &str, p: &Params) -> Option<impl Fn(&str) -> f64> {
    let defs = params(id)?;
    for (k, v) in p {
        let d = defs.iter().find(|d| d.key == k)?;
        if !v.is_finite() || *v < d.min || *v > d.max || (INTS.contains(&k.as_str()) && v.fract() != 0.) { return None; }
    }
    let p = p.clone();
    Some(move |k: &str| p.get(k).copied().unwrap_or_else(|| defs.iter().find(|d| d.key == k).unwrap().def))
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !id.starts_with("dmast_") { return None; }
    let g = get(id, p)?;
    if to >= x.c.len() || from > to { return None; }
    let c = &x.c;
    Some(match id {
        "dmast_fixmempoly" => {
            let (pw, n) = (g("pw") as usize, g("n") as usize);
            let (up, dn) = (1. + g("pctup") / 100., 1. - g("pctdn") / 100.);
            let (up2, dn2) = (1. + g("pctjup") / 100., 1. - g("pctjdn") / 100.);
            let w = ls_forecast_weights(n, pw);
            let mbb = 50.max(n - 1);
            let mut xp = vec![f64::NAN; c.len()];
            let mut sw = Swing::new(true, up, dn, 0.);
            engine(x, from, to, mbb, |i, cb, mp| {
                if cb == 1 { for v in &mut xp[..=i] { *v = c[i]; } sw = Swing::new(true, up, dn, c[i]); return vec![]; }
                let f: f64 = (0..n).map(|k| w[k] * c[i + 1 - n + k]).sum();
                xp[i] = f;
                sw.update(f, Some((c[i], xp[i - 1])));
                let mut o = vec![];
                if mp <= 0 {
                    if sw.turned(1) { o.push(mkt(1, "B%LSup")); }
                    if f > xp[i - 2] * up2 { sw.force(1, f); o.push(mkt(1, "B2dy%jmp")); }
                }
                if mp >= 0 {
                    if sw.turned(-1) { o.push(mkt(-1, "S%LSdn")); }
                    if f < xp[i - 2] * dn2 { sw.force(-1, f); o.push(mkt(-1, "S2dy%jmp")); }
                }
                o
            })
        }
        "dmast_fadmem1" | "dmast_fadmem2" => {
            let one = id == "dmast_fadmem1";
            let (pctsw, deg) = (g("pctsw"), g("degree") as usize);
            let alf = 2. / (1. + g("ndays"));
            let cc = 1. - alf;
            let (dup, ddn, delup, deldn) = if one { (g("dup"), g("ddn"), 99., 99.) } else { (99., 99., g("delup"), g("deldn")) };
            let (mut b, mut xma) = ([0f64; 7], 0.);
            let mut sw = Swing::new(false, delup, deldn, 0.);
            engine(x, from, to, 40, |i, cb, mp| {
                let prc = c[i];
                if cb == 1 { b = [0.; 7]; b[0] = prc / alf; xma = prc; sw = Swing::new(false, delup, deldn, 0.); return vec![]; }
                let bp = b;
                b[0] = cc * bp[0] + prc;
                for k in 1..7 { b[k] = cc * bp[k] + b[k - 1] - bp[k - 1]; }
                let xp1 = alf * b[..=deg].iter().sum::<f64>();
                xma += alf * (prc - xma);
                let ratio = if pctsw == 1. { 100. * (xp1 - xma) / xma } else { xp1 - xma };
                if delup < 99. { sw.update(ratio, None); }
                let mut o = vec![];
                if dup != 99. {
                    if mp <= 0 && ratio > dup { o.push(on_close(1, "Bdup")); }
                    if mp >= 0 && ratio < -ddn { o.push(on_close(-1, "Sddn")); }
                }
                if delup != 99. {
                    if mp <= 0 && sw.turned(1) { o.push(on_close(1, "B%Dxup")); }
                    if mp >= 0 && sw.turned(-1) { o.push(on_close(-1, "S%DXdn")); }
                }
                o
            })
        }
        "dmast_kalman_gh" => {
            let th = g("theta");
            let (gn, hn) = (1. - th * th, (1. - th) * (1. - th));
            let (delup, deldn) = (g("delup"), g("deldn"));
            let (mut dx, mut xs) = (0., 0.);
            let mut sw = Swing::new(false, delup, deldn, 0.);
            engine(x, from, to, 50, |i, cb, mp| {
                let y = c[i];
                if cb < 3 { xs = y; sw = Swing::new(false, delup, deldn, 0.); return vec![]; }
                let err = y - xs;
                dx += hn * err;
                xs = xs + dx + gn * err;
                sw.update(100. * dx / y, None);
                let mut o = vec![];
                if mp <= 0 && sw.turned(1) { o.push(mkt(1, "B%Dxup")); }
                if mp >= 0 && sw.turned(-1) { o.push(mkt(-1, "S%DXdn")); }
                o
            })
        }
        "dmast_kalman_ghk" => {
            let gn = g("gn");
            let hn = 2. * (2. - gn) - 4. * (1. - gn).sqrt();
            let kn = hn * hn / (4. * gn);
            let (delup, deldn) = (g("delup"), g("deldn"));
            let (mut ddxs, mut dxs, mut xs) = (0., 0., 0.);
            let mut sw = Swing::new(false, delup, deldn, 0.);
            engine(x, from, to, 50, |i, cb, mp| {
                let y = c[i];
                if cb < 4 { xs = y; dxs = 0.; ddxs = 0.; sw = Swing::new(false, delup, deldn, 0.); return vec![]; }
                let err = y - xs;
                ddxs += 2. * kn * err;
                dxs = dxs + ddxs + hn * err;
                xs = xs + dxs + ddxs / 2. + gn * err;
                sw.update(100. * dxs / y, None);
                let mut o = vec![];
                if mp <= 0 && sw.turned(1) { o.push(mkt(1, "B%Dxup")); }
                if mp >= 0 && sw.turned(-1) { o.push(mkt(-1, "S%DXdn")); }
                o
            })
        }
        "dmast_kalman_fmem" => {
            let (lam, ralf) = (g("lam"), g("ralf"));
            let (up, dn) = (1. + g("pctup") / 100., 1. - g("pctdn") / 100.);
            let (mut xhatm, mut pm, mut rrave) = (0., 1., 0.);
            let mut sw = Swing::new(true, up, dn, 0.);
            engine(x, from, to, 50, |i, cb, mp| {
                let y = c[i];
                if cb == 1 {
                    xhatm = 0.99 * y; pm = 1.; rrave = (y - xhatm) * (y - xhatm);
                    sw = Swing::new(true, up, dn, y); return vec![];
                }
                let rr = (y - xhatm) * (y - xhatm);
                rrave += ralf * (rr - rrave);
                let temp = pm + lam * rrave;
                let k = if temp == 0. { 1. } else { pm / temp };
                let xhat = xhatm + k * (y - xhatm);
                let p = (1. - k) * pm / lam;
                xhatm = xhat; pm = p;
                sw.update(xhat, None);
                let mut o = vec![];
                if mp <= 0 && sw.turned(1) { o.push(mkt(1, "B%Kup")); }
                if mp >= 0 && sw.turned(-1) { o.push(mkt(-1, "S%Kdn")); }
                o
            })
        }
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn rows(c: &[f64]) -> Vec<Row> {
        c.iter().enumerate().map(|(i, &v)| Row { ticker: "T".into(), date: format!("{:04}-01-01", 1900 + i), open: v, high: v + 1., low: v - 1., close: v, volume: None }).collect()
    }
    #[test]
    fn ls_weights_extrapolate_polynomials_exactly() {
        for (n, pw) in [(40usize, 3usize), (10, 2), (80, 4)] {
            let w = ls_forecast_weights(n, pw);
            let f = |t: f64| 2. + 0.5 * t - 0.01 * t * t + if pw >= 3 { 0.0003 * t * t * t } else { 0. };
            let est: f64 = (0..n).map(|k| w[k] * f(k as f64)).sum();
            assert!((est - f(n as f64)).abs() < 1e-7, "{n} {pw} {est} {}", f(n as f64));
        }
    }
    #[test]
    fn swing_reverses_on_percent_move_and_engine_fills_next_open() {
        // rising then falling closes: FixedMemPoly must go long then reverse short, fills at the next open
        let mut c: Vec<f64> = (0..120).map(|i| 100. + i as f64).collect();
        c.extend((0..60).map(|i| 219. - 3. * i as f64));
        let r = rows(&c);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("dmast_fixmempoly", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() >= 2);
        assert_eq!(t[0].dir, 1.);
        let s = t.iter().find(|t| t.dir < 0.).unwrap();
        assert_eq!(s.legs[0].px, x.o[s.legs[0].i]);
        assert_eq!(t[0].signal_i + 1, t[0].exit_i);
        assert_eq!(t[0].exit_i, s.legs[0].i);
        for w in t.windows(2) { assert_eq!(w[0].exit_i, w[1].legs[0].i); assert_eq!(w[0].exit_px, w[1].legs[0].px); }
    }
    #[test]
    fn every_live_system_runs_and_rejects_bad_params() {
        let c: Vec<f64> = (0..400).map(|i| 100. + 10. * ((i as f64) / 9.).sin() + i as f64 * 0.05).collect();
        let r = rows(&c);
        let x = Ctx::new(&r, &r, &[]);
        for s in catalogue() {
            let res = run(s.id, &x, 0, r.len() - 1, &Params::new());
            if matches!(s.status, Status::Live) { assert!(!res.unwrap().is_empty(), "{}", s.id); } else { assert!(res.is_none()); }
        }
        assert!(run("dmast_fixmempoly", &x, 0, 399, &Params::from([("pw".into(), 2.5)])).is_none());
        assert!(run("dmast_fixmempoly", &x, 0, 399, &Params::from([("zz".into(), 1.)])).is_none());
    }
}
