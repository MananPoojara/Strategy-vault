//! Dennis Meyers, "Short Term Systems & Indicators" Version 2.0 User's Manual (STMANUAL, Meyers Analytics
//! L.L.C., 1997-98). Book id `dmst2`. Source: research drive ".../Dennis Meyers Short Term Systems & Indicators 2
//! (meyersanalytics.com)/STMANUAL.PDF" (102 pp.), copy in Books/Dennis Meyers/.
//! Transcription, page refs, oracle and verification: library/dmst2/.
//!
//! Every system is printed as complete TradeStation 4 EasyLanguage; the code is the rule and wins over the prose
//! (defaults = the code's `input:` values; where the prose lists other numbers they are noted). All systems are
//! stop-and-reverse. TS4 conventions reproduced exactly: `at market` = next open; `on close` = this close; `Buy X
//! stop` placed on bar t acts on bar t+1, fills when the high reaches X, at the open if the open is already beyond
//! X; orders in the direction already held are ignored; series history before the first computed bar returns the
//! first bar's value. Prices and point thresholds (xo, xprc, $up, tik, minimum ranges) are used as printed, in the
//! selected instrument's points (instrument swap only, no rescaling). The study runs from the first bar of the data
//! after MaxBarsBack (30 where the code states it, else the TS4 default 50); positions taken before the selected
//! start are not reported. The book's MARK input is the engine's open-trade mark.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "dmst2";
/// Admitted ids (authors/meyers/admission/SOURCE-CONTRACT.md).
pub const REVIEWED: &[&str] = &["dmst2_xavtr", "dmst2_lsqr_t1", "dmst2_t2", "dmst2_polychrom", "dmst2_maxlik", "dmst2_pctupdn"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

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

fn stop(d: i8, px: f64, tag: &'static str) -> O { O { d, k: K::Stop, px, tag } }

// ---------------------------------------------------------------------------------------------------------
type PDef = (&'static str, &'static str, f64, f64, f64, f64);
struct S { id: &'static str, name: &'static str, group: &'static str, rules: &'static [&'static str], notes: &'static str, defs: &'static [PDef], fam: (&'static str, &'static str, &'static str) }

const PARAB: &[PDef] = &[
    ("start", "start, initial acceleration factor", 0.01, 0.005, 0.1, 0.005),
    ("inc", "inc, AF increment on a new extreme", 0.02, 0.005, 0.1, 0.005),
    ("max", "max, maximum AF", 0.04, 0.01, 0.5, 0.01),
    ("xo", "xo, points price must cross SAR[1] by", 0.45, 0., 10., 0.05),
    ("xprc", "xprc, points added to the initial stop", 2.5, 0., 10., 0.5),
];
const NCH: &[PDef] = &[
    ("nhi", "nhi, bars in the highest high", 24., 2., 30., 1.),
    ("nlo", "nlo, bars in the lowest low", 12., 2., 30., 1.),
    ("xo", "xo, noise filter in points", 0.7, 0., 10., 0.1),
];
const XATR: &[PDef] = &[
    ("atrma", "atrma, bars in the exponential average true range", 3., 1., 10., 1.),
    ("bx", "bx, buy multiplier of xAvTR", 2.5, 0.5, 3., 0.1),
    ("sx", "sx, sell multiplier of xAvTR (0 = bx)", 2.3, 0., 3., 0.1),
    ("xma", "xma, bars smoothing the channels (1 = none)", 1., 1., 9., 1.),
    ("xatrmin", "xatrmin, minimum channel width in points", 0., 0., 10., 0.5),
];
const MBAR: &[PDef] = &[
    ("blag", "Blag, bars in the buy range (incl. today)", 6., 2., 14., 1.),
    ("bx", "bx, buy range multiplier", 0.85, 0.1, 2., 0.025),
    ("bminrge", "bminrge, minimum buy range in points", 0.563, 0., 10., 0.1),
    ("slag", "Slag, bars in the sell range (incl. today)", 2., 2., 14., 1.),
    ("sx", "sx, sell range multiplier", 1.55, 0.1, 2., 0.025),
    ("sminrge", "sminrge, minimum sell range in points", 0.563, 0., 10., 0.1),
];
const LSQ: &[PDef] = &[
    ("lslen", "lslen (T), closes in the least-squares line", 30., 10., 50., 1.),
    ("pctup", "pctup, % rise of pf from its low while short", 1.425, 0.1, 3., 0.025),
    ("pctdn", "pctdn, % fall of pf from its high while long", 1.425, 0.1, 3., 0.025),
    ("pctjup", "pctjup, % pf must exceed pf 2 days ago", 0.35, 0.05, 2., 0.01),
    ("pctjdn", "pctjdn, % pf must fall below pf 2 days ago", 0.5, 0.05, 2., 0.01),
];
const T2: &[PDef] = &[
    ("nt2", "Nt2 (T), closes in the least-squares quadratic", 43., 10., 50., 1.),
    ("pctup", "pctup, % rise of pf from its low while short", 2.4, 0.1, 4., 0.05),
    ("pctdn", "pctdn, % fall of pf from its high while long", 2.9, 0.1, 4., 0.05),
    ("pctjup", "pctjup, % pf must exceed pf 2 days ago", 1.14, 0.1, 3., 0.02),
    ("pctjdn", "pctjdn, % pf must fall below pf 2 days ago", 1.2, 0.1, 3., 0.02),
];
const POLY: &[PDef] = &[
    ("ns", "ns, first momentum lookback", 1., 1., 10., 1.),
    ("ne", "ne, last momentum lookback (and mtmstr window)", 14., 2., 49., 1.),
    ("alf", "alf, weight exponent (momentum / n^alf)", 0.5, 0., 2., 0.05),
    ("bxo", "bxo, mave level for a buy", 0., -3., 3., 0.1),
    ("sxo", "sxo, mave level for a sell", -1.6, -3., 3., 0.1),
    ("blev", "blev, mtmstr level for a buy", 0., 0., 100., 5.),
    ("slev", "slev, mtmstr level for a sell", 80., 0., 100., 5.),
    ("mtmtype", "mtmtype: 0 points, 1 percent momentum", 1., 0., 1., 1.),
];
const MXL: &[PDef] = &[
    ("slen", "slen, first lookback n", 8., 2., 14., 1.),
    ("llen", "llen, last lookback n", 21., 3., 49., 1.),
    ("xoh", "xoh, rgehmax level for a buy", 0.65, 0.3, 2., 0.05),
    ("xol", "xol, rgelmax level for a sell", 0.85, 0.3, 2., 0.05),
    ("alf", "alf, exponent α in the n^α weight", 0.75, 0.3, 1., 0.05),
    ("xbot", "xbot, the opposite ratio must be below this", 0.9, 0.3, 3., 0.05),
];
const PUD: &[PDef] = &[
    ("switch", "switch: 0 closes (on close), 1 highs/lows, 2 (H+L)/2, 3 lows/highs (stops)", 0., 0., 3., 1.),
    ("pctup", "%up, percent rise from the low while short", 3., 0.5, 10., 0.1),
    ("pctdn", "%dn, percent fall from the high while long", 3., 0.5, 10., 0.1),
];
const DUD: &[PDef] = &[
    ("switch", "switch: 0 closes (on close), 1 highs/lows, 2 (H+L)/2, 3 lows/highs (stops)", 0., 0., 3., 1.),
    ("dup", "dup, points rise from the low while short", 3., 0.5, 500., 0.5),
    ("ddn", "ddn, points fall from the high while long", 3., 0.5, 500., 0.5),
];
const NCH2: &[PDef] = &[
    ("nhi", "nhi, bars in the highest high", 21., 5., 30., 1.),
    ("xou", "xou, buy noise filter (points)", 0., 0., 10., 0.125),
    ("nlo", "nlo, bars in the lowest low", 14., 5., 30., 1.),
    ("xod", "xod, sell noise filter (points)", 0.625, 0., 10., 0.125),
    ("tik", "tik, minimum tick added beyond the channel (points)", 0.03125, 0., 1., 0.03125),
];

fn table() -> Vec<S> {
    vec![
        S { id: "dmst2_parabxox", name: "Five Parameter Parabolic (parabxox)", group: "III · Five Parameter Parabolic", defs: PARAB,
            rules: &[
                "F|SAR = Wilder's parabolic with a free starting AF (start), increment (inc) and maximum (max); on a reversal the SAR restarts at the extreme of the previous signal ± xprc. (pp. III-1–III-2, code III-11)",
                "E|Buy if price crosses above SAR[1] by more than xo (code: Buy SAR+xo stop, placed for the next bar).",
                "E|Sell (reverse to short) if price crosses below SAR[1] by more than xo (code: Sell SAR-xo stop).",
                "X|Stop and reverse; no other exit.",
            ],
            notes: "Code defaults start=0.010, inc=0.020, max=0.04, xo=0.45, xprc=2.5 (S&P points). Book market: S&P 500 futures 1988-1998 walk-forward (103 trades, 44.7% win, avg 0.57%). xo/xprc are S&P points used as printed; on NIFTY they are a far smaller fraction of price. TS4 default MaxBarsBack 50 assumed.",
            fam: ("dmst2_breakout", "Meyers channel & parabolic reversal systems", "Five-parameter parabolic") },
        S { id: "dmst2_noisechannel", name: "Noise Channel Breakout", group: "IV · Noise Channel Breakout", defs: NCH,
            rules: &[
                "F|ahhp = highest high of the last nhi bars, but while short it can only stay the same or go lower; allp = lowest low of the last nlo bars, while long it can only stay the same or go higher. (p. IV-1, code IV-7)",
                "E|Buy if price crosses above ahhp by xo or more (code: Buy ahhp+xo stop for the next bar).",
                "E|Sell (reverse to short) if price crosses below allp by xo or more (code: Sell allp-xo stop).",
                "X|Stop and reverse; no other exit.",
            ],
            notes: "Code defaults nhi=24, nlo=12, xo=0.7 (prose: xo = 0.0070 = 70 ticks of the Deutsche Mark; the code's 0.7 is the vendor-scaled DM price). Book market: Deutsche Mark futures 1988-1998 (30 trades, 63.3% win). MaxBarsBack 30 as the code note states. The code's internal `position` variable governs the channel ratchet, exactly as printed.",
            fam: ("dmst2_breakout", "Meyers channel & parabolic reversal systems", "Noise channel") },
        S { id: "dmst2_xavtr", name: "xAvTR Channel Breakout", group: "V · xAvTR Channel Breakout", defs: XATR,
            rules: &[
                "F|xAvTR = exponential average (α = 2/(1+atrma)) of true range; buyx = close + bx·xAvTR, sellx = close - sx·xAvTR, each smoothed by an xma-bar EMA into xb, xs. (pp. V-1–V-2, code V-10)",
                "E|Buy if price crosses above xb[1] (code: Buy xb stop for the next bar).",
                "E|Sell (reverse to short) if price crosses below xs[1] (code: Sell xs stop).",
                "X|Stop and reverse; no other exit.",
            ],
            notes: "Code defaults atrma=3, bx=2.5, sx=2.3, xma=1 (no smoothing), xatrmin=0. Book market: Deutsche Mark futures 1988-1998 (107 trades, 54.2% win). TS4 default MaxBarsBack 50 assumed.",
            fam: ("dmst2_breakout", "Meyers channel & parabolic reversal systems", "Average-true-range channel") },
        S { id: "dmst2_multibar", name: "MultiBar Range Channel Breakout", group: "VI · MultiBar Range Channel Breakout", defs: MBAR,
            rules: &[
                "F|Range(n) = highest high - lowest low of the last n bars including today; brge = max(Range(Blag), bminrge), srge = max(Range(Slag), sminrge). (pp. VI-1–VI-2, code VI-10)",
                "E|Buy if price crosses above close + bx·brge of the previous bar (code: Buy c + Value1·bx stop for the next bar).",
                "E|Sell (reverse to short) if price crosses below close - sx·srge of the previous bar (code: Sell c - Value2·sx stop).",
                "X|Stop and reverse; no other exit.",
            ],
            notes: "Code defaults Blag=6, bx=0.85, bminrge=0.563, Slag=2, sx=1.55, sminrge=0.563; the prose list (VI-2) prints Blag 4, bx 0.25, bminrge 0, Slag 4, sx 1.4, sminrge 0 — the code is used. Book market: S&P 500 futures 1988-1998 walk-forward (97 trades, 52.6% win, avg 1.40%). TS4 default MaxBarsBack 50 assumed.",
            fam: ("dmst2_breakout", "Meyers channel & parabolic reversal systems", "Multi-bar range channel") },
        S { id: "dmst2_lsqr_t1", name: "Least Squares t+1 system", group: "VII · Least Squares t+1", defs: LSQ,
            rules: &[
                "F|pf = least-squares straight line through the last T closes, forecast for tomorrow: pf = a0 + b0(T+1). (pp. VII-1–VII-2, code VII-9)",
                "E|Buy at tomorrow's open if pf has moved up by more than pctup % from the lowest pf recorded while short (code also requires close > pf[1]); or if pf > pf two days ago × (1+pctjup/100).",
                "E|Sell (reverse to short) at tomorrow's open if pf has moved down by more than pctdn % from the highest pf recorded while long (code also requires close < pf[1]); or if pf < pf two days ago × (1-pctjdn/100).",
                "X|Stop and reverse; no other exit.",
            ],
            notes: "Code defaults lslen=30, pctup=1.425, pctdn=1.425, pctjup=0.35, pctjdn=0.50 (= prose). Book market: US Bond futures 1988-1998 (58 trades, 58.6% win, avg 1.88%). TS4 default MaxBarsBack 50 assumed.",
            fam: ("dmst2_poly", "Meyers least-squares trend systems", "Straight line (t+1)") },
        S { id: "dmst2_t2", name: "T**2 (least-squares quadratic) system", group: "VIII · T**2", defs: T2,
            rules: &[
                "F|pf = least-squares quadratic through the last T closes, forecast for tomorrow (code weights 3/T·(a0 + a1·t + a2·t²), a0=(T+3)(T+2)/((T-1)(T-2)), a1=-2(4T+7)/((T-1)(T-2)), a2=10/((T-1)(T-2))). (pp. VIII-1–VIII-3, code VIII-9)",
                "E|Buy at tomorrow's open if pf has moved up by more than pctup % from the lowest pf recorded while short (and close > pf[1]); or if pf > pf two days ago × (1+pctjup/100).",
                "E|Sell (reverse to short) at tomorrow's open if pf has moved down by more than pctdn % from the highest pf recorded while long (and close < pf[1]); or if pf < pf two days ago × (1-pctjdn/100).",
                "X|Stop and reverse; no other exit.",
            ],
            notes: "Code defaults Nt2=43, pctup=2.4, pctdn=2.9, pctjup=1.14, pctjdn=1.2 (prose VIII-3: 40, 2.05, 2.8, 0.74, 1.12 — the code is used). The printed closed-form formula on p. VIII-2 has a typo (2(4T+3)); the code's coefficients are the exact least-squares forecast (checked). Book market: US Bond futures. TS4 default MaxBarsBack 50 assumed.",
            fam: ("dmst2_poly", "Meyers least-squares trend systems", "Quadratic (T**2)") },
        S { id: "dmst2_polychrom", name: "Polychromatic Momentum system", group: "IX · Polychromatic Momentum", defs: POLY,
            rules: &[
                "F|mtm[i] = 100·(price - price[i])/price[i]; mave = Σ mtm[i]/i^alf / Σ 1/i^alf for i = ns..ne; rgemtm = max - min of the weighted mtm[i]; mtmstr = 100·(rgemtm - lowest(rgemtm, ne))/(highest - lowest). (pp. IX-1–IX-2, code IX-9)",
                "E|Buy on the close if mave > bxo and mtmstr > blev (code: `buy on close`; the prose says tomorrow's open — the code is used).",
                "E|Sell (reverse to short) on the close if mave < sxo and mtmstr > slev and mave < mave[1].",
                "X|Stop and reverse; no other exit.",
            ],
            notes: "Code defaults ns=1, ne=14, alf=0.5, bxo=0, sxo=-1.6, slev=80, blev=0, mtmtype=1, price=close. Book market: S&P 500 futures 1988-1998 walk-forward (39 trades, 53.8% win, avg 2.48%). TS4 default MaxBarsBack 50 assumed.",
            fam: ("dmst2_momentum", "Meyers momentum & range systems", "Polychromatic momentum") },
        S { id: "dmst2_maxlik", name: "Maximum Likelihood Range system", group: "X · Maximum Likelihood Range", defs: MXL,
            rules: &[
                "F|For n = slen..llen: avTR(n) = mean true range of the n bars before today; rgeh(n) = (high - low[n])/(avTR(n)·n^alf); rgel(n) = (high[n] - low)/(avTR(n)·n^alf); rgehmax, rgelmax = their maxima. (pp. X-1–X-2, code X-9)",
                "E|Buy at tomorrow's open if rgehmax >= xoh and rgelmax < xbot.",
                "E|Sell (reverse to short) at tomorrow's open if rgelmax >= xol and rgehmax < xbot.",
                "X|Stop and reverse; no other exit.",
            ],
            notes: "Code defaults slen=8, llen=21, xoh=0.65, xol=0.85, alf=0.75, xbot=0.9 (= prose). Book market: US Bond futures 1988-1998 (46 trades, 56.5% win, avg 2.10%). If both conditions hold on one bar only the order opposite to the current position can fill. TS4 default MaxBarsBack 50 assumed.",
            fam: ("dmst2_momentum", "Meyers momentum & range systems", "Maximum likelihood range") },
        S { id: "dmst2_pctupdn", name: "%Up-%Down system", group: "XI · %Up %Down", defs: PUD,
            rules: &[
                "E|switch=0: buy on the close if the close has moved up by %up from the lowest close recorded while short; sell on the close if the close has moved down by %dn from the highest close recorded while long. (pp. XI-1–XI-2, code XI-5)",
                "E|switch=1: buy (stop) when price moves up %up from the lowest low while short; sell (stop) when price moves down %dn from the highest high while long. switch=2: same from the price function (H+L)/2; switch=3: from the lowest high / highest low.",
                "X|Stop and reverse; no other exit.",
            ],
            notes: "Code defaults switch=0, %up=3, %dn=3, prcfunc=(H+L)/2 (prose: %up 1.4, %dn 3.7, prcfunc close — the code is used). No market results are printed for this chapter. TS4 default MaxBarsBack 50 assumed.",
            fam: ("dmst2_updn", "Meyers up/down reversal systems", "Percent") },
        S { id: "dmst2_dolupdn", name: "$Up-$Down system", group: "XII · $Up $Down", defs: DUD,
            rules: &[
                "E|switch=0: buy on the close if the close has moved up by $up points from the lowest close recorded while short; sell on the close if it has moved down by $dn points from the highest close recorded while long. (pp. XII-1–XII-2, code XII-5)",
                "E|switch=1..3: as the %Up-%Down system but in points, with stop orders.",
                "X|Stop and reverse; no other exit.",
            ],
            notes: "Code defaults switch=0, dup=3, ddn=3 points, prcfunc=(H+L)/2 (prose: 1.4 / 3.7). Points are used as printed (no rescaling to the index). No market results are printed. TS4 default MaxBarsBack 50 assumed.",
            fam: ("dmst2_updn", "Meyers up/down reversal systems", "Points") },
        S { id: "dmst2_noisechannel2", name: "Noise Channel2 Breakout", group: "XIII · Noise Channel2 Breakout", defs: NCH2,
            rules: &[
                "F|ahhp = highest high of the last nhi bars (cannot rise while short or flat); allp = lowest low of the last nlo bars (cannot fall while long or flat). (p. XIII-1, code XIII-9)",
                "E|Buy if price crosses above ahhp by more than xou (code: Buy ahhp + xou + tik stop).",
                "E|Sell (reverse to short) if price crosses below allp by more than xod (code: Sell allp - xod - tik stop).",
                "X|Stop and reverse; no other exit.",
            ],
            notes: "Code defaults nhi=21, xou=0, nlo=14, tik=0.03125; the code's xod input is misprinted `0.0.625`, so xod = 0.625 from the prose list (which prints nhi = 24; the code's 21 is used). Book market: US Bond futures 1989-1998 (41 trades, 48.8% win, avg 2.05%). MaxBarsBack 30 as the code note states.",
            fam: ("dmst2_breakout", "Meyers channel & parabolic reversal systems", "Noise channel 2") },
    ]
}

pub fn catalogue() -> Vec<StrategyDef> {
    table().into_iter().map(|s| StrategyDef { id: s.id, name: s.name, group: s.group, session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules: s.rules, notes: s.notes, claim: None }).collect()
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { table().into_iter().find(|s| s.id == id).map(|s| s.fam) }
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let s = table().into_iter().find(|s| s.id == id)?;
    Some(s.defs.iter().map(|&(key, label, def, min, max, step)| ParamDef { key, label, def, min, max, step }).collect())
}
const INTS: &[&str] = &["nhi", "nlo", "atrma", "xma", "blag", "slag", "lslen", "nt2", "ns", "ne", "mtmtype", "slen", "llen", "switch"];
fn get(id: &str, p: &Params) -> Option<impl Fn(&str) -> f64> {
    let defs = params(id)?;
    for (k, v) in p {
        let d = defs.iter().find(|d| d.key == k)?;
        if !v.is_finite() || *v < d.min || *v > d.max || (INTS.contains(&k.as_str()) && v.fract() != 0.) { return None; }
    }
    let p = p.clone();
    Some(move |k: &str| p.get(k).copied().unwrap_or_else(|| defs.iter().find(|d| d.key == k).unwrap().def))
}
fn hh(v: &[f64], i: usize, n: usize) -> f64 { v[i + 1 - n..=i].iter().copied().fold(f64::MIN, f64::max) }
fn ll(v: &[f64], i: usize, n: usize) -> f64 { v[i + 1 - n..=i].iter().copied().fold(f64::MAX, f64::min) }
/// TradeStation TrueRange of bar i.
fn tr(x: &Ctx, i: usize) -> f64 { x.h[i].max(x.c[i - 1]) - x.l[i].min(x.c[i - 1]) }

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !id.starts_with("dmst2_") { return None; }
    let g = get(id, p)?;
    if to >= x.c.len() || from > to { return None; }
    let (h, l, c) = (&x.h, &x.l, &x.c);
    Some(match id {
        "dmst2_parabxox" => {
            let (start, inc, max, xo, xprc) = (g("start"), g("inc"), g("max"), g("xo"), g("xprc"));
            let (mut position, mut sar, mut af, mut hi, mut lo) = (-1i8, 0., 0.02, 0., 0.);
            engine(x, from, to, 50, |i, cb, mp| {
                let (pos1, sar1, hi1, lo1) = if cb == 1 { (-1, c[i], h[i], l[i]) } else { (position, sar, hi, lo) };
                if cb == 1 { position = 1; sar = c[i]; hi = h[i]; lo = l[i]; } else {
                    if h[i] > hi { hi = h[i]; }
                    if l[i] < lo { lo = l[i]; }
                    if position == 1 { if l[i] <= sar1 - xo { position = -1; } } else if h[i] >= sar1 + xo { position = 1; }
                }
                if position == 1 {
                    if pos1 != 1 { sar = lo - xprc; af = start; lo = l[i]; hi = h[i]; } else {
                        sar = sar1 + af * (hi - sar1);
                        if hi > hi1 && af < max { af += inc; }
                        if af > max { af = max; }
                    }
                    let mut tlo = l[i];
                    if l[i - 1] < l[i] { tlo = l[i - 1]; }
                    if tlo < sar1 { tlo = sar1; }
                    if sar > tlo { sar = tlo; }
                } else {
                    if pos1 != -1 { sar = hi + xprc; af = start; lo = l[i]; hi = h[i]; } else {
                        sar = sar1 + af * (lo - sar1);
                        if lo < lo1 && af < max { af += inc; }
                        if af > max { af = max; }
                    }
                    let mut thi = h[i];
                    if h[i - 1] < h[i] { thi = h[i - 1]; }
                    if thi > sar1 { thi = sar1; }
                    if sar < thi { sar = thi; }
                }
                let mut v = vec![];
                if mp <= 0 { v.push(stop(1, sar + xo, "parabxox buy")); }
                if mp >= 0 { v.push(stop(-1, sar - xo, "parabxox sell")); }
                v
            })
        }
        "dmst2_noisechannel" => {
            let (nhi, nlo, xo) = (g("nhi") as usize, g("nlo") as usize, g("xo"));
            let (mut position, mut hp, mut lp) = (0i8, 0., 0.);
            engine(x, from, to, 30.max(nhi.max(nlo) - 1), |i, cb, mp| {
                let (mut hhv, mut llv) = (hh(h, i, nhi), ll(l, i, nlo));
                let mut v = vec![];
                if cb > 1 {
                    if position >= 0 && llv < lp { llv = lp; }
                    if position <= 0 && hhv > hp { hhv = hp; }
                    if position == 1 { if l[i] < lp - xo { position = -1; } } else if h[i] > hp + xo { position = 1; }
                    let (up, dn) = (hhv + xo, llv - xo);
                    if mp == 0 { v.push(stop(1, up, "bpos=0")); position = 1; }
                    if mp == 0 { v.push(stop(-1, dn, "spos=0")); position = -1; }
                    if mp < 0 { v.push(stop(1, up, "noise channel buy")); }
                    if mp > 0 { v.push(stop(-1, dn, "noise channel sell")); }
                }
                hp = hhv; lp = llv;
                v
            })
        }
        "dmst2_xavtr" => {
            let (atrma, bx, sx, xma, xmin) = (g("atrma"), g("bx"), g("sx"), g("xma"), g("xatrmin"));
            let alf = if atrma > 1. { 2. / (atrma + 1.) } else { 1. };
            let alf2 = if xma > 1. { 2. / (1. + xma) } else { 1. };
            let sxx = if sx == 0. { bx } else { sx };
            let (mut atr, mut xb, mut xs) = (0., 0., 0.);
            engine(x, from, to, 50, |i, cb, mp| {
                if cb == 1 { atr = h[i] - l[i]; xb = c[i] + atr; xs = c[i] - atr; return vec![]; }
                atr += alf * (tr(x, i) - atr);
                let (up, dn) = ((bx * atr).max(xmin), (sxx * atr).max(xmin));
                xb += alf2 * (c[i] + up - xb);
                xs += alf2 * (c[i] - dn - xs);
                let mut v = vec![];
                if mp <= 0 { v.push(stop(1, xb, "xAvTR buy")); }
                if mp >= 0 { v.push(stop(-1, xs, "xAvTR sell")); }
                v
            })
        }
        "dmst2_multibar" => {
            let (blag, bx, bmin, slag, sx, smin) = (g("blag") as usize, g("bx"), g("bminrge"), g("slag") as usize, g("sx"), g("sminrge"));
            engine(x, from, to, 50.max(blag.max(slag) - 1), |i, cb, _mp| {
                if cb == 1 { return vec![]; }
                let v1 = (hh(h, i, blag) - ll(l, i, blag)).max(bmin);
                let v2 = (hh(h, i, slag) - ll(l, i, slag)).max(smin);
                vec![stop(1, c[i] + v1 * bx, "MultiBar buy"), stop(-1, c[i] - v2 * sx, "MultiBar sell")]
            })
        }
        "dmst2_lsqr_t1" | "dmst2_t2" => {
            let lsq = id == "dmst2_lsqr_t1";
            let n = if lsq { (g("lslen") as usize).max(2) } else { (g("nt2") as usize).max(3) };
            let jf = n as f64;
            // forecast weights on prc[ii-1], ii = 1..n (ii=1 today)
            let w: Vec<f64> = (1..=n).map(|ii| {
                let t = (n - ii + 1) as f64; // time index, today = n
                if lsq {
                    let (b1, b2) = (12. / (jf * (jf - 1.) * (jf + 1.)), 6. / (jf * (jf - 1.)));
                    let b0w = b1 * t - b2; // slope weight
                    1. / jf - b0w * (jf + 1.) / 2. + b0w * (jf + 1.)
                } else {
                    let bott = (jf - 1.) * (jf - 2.);
                    3. / jf * ((jf + 3.) * (jf + 2.) / bott - 2. * (4. * jf + 7.) / bott * t + 10. / bott * t * t)
                }
            }).collect();
            let pf = |i: usize| -> f64 { (1..=n).map(|ii| w[ii - 1] * c[i + 1 - ii]).sum() };
            let (up, dn) = (1. + g("pctup") / 100., 1. - g("pctdn") / 100.);
            let (up2, dn2) = (1. + g("pctjup") / 100., 1. - g("pctjdn") / 100.);
            let mut xp = vec![f64::NAN; c.len()];
            let mut sw = Swing::new(true, up, dn, 0.);
            engine(x, from, to, 50.max(n - 1), |i, cb, mp| {
                if lsq && cb < 3 {
                    let f = pf(i);
                    if cb == 1 { for v in &mut xp[..i] { *v = c[i]; } }
                    xp[i] = f; sw = Swing::new(true, up, dn, f); return vec![];
                }
                if !lsq && cb == 1 { for v in &mut xp[..=i] { *v = c[i]; } sw = Swing::new(true, up, dn, c[i]); return vec![]; }
                let f = pf(i);
                xp[i] = f;
                sw.update(f, Some((c[i], xp[i - 1])));
                let mut v = vec![];
                if mp <= 0 {
                    if sw.turned(1) { v.push(mkt(1, "B%LSup")); }
                    if f > xp[i - 2] * up2 { sw.force(1, f); v.push(mkt(1, "B2dy%jmp")); }
                }
                if mp >= 0 {
                    if sw.turned(-1) { v.push(mkt(-1, "S%LSdn")); }
                    if f < xp[i - 2] * dn2 { sw.force(-1, f); v.push(mkt(-1, "S2dy%jmp")); }
                }
                v
            })
        }
        "dmst2_polychrom" => {
            let (ns, ne, alf) = (g("ns") as usize, g("ne") as usize, g("alf"));
            let (bxo, sxo, blev, slev, typ) = (g("bxo"), g("sxo"), g("blev"), g("slev"), g("mtmtype"));
            let ms = ns.max(1);
            let me = if ne <= ms { ms + 1 } else { ne };
            let me = me.min(50);
            let wght = |k: usize| if k == 1 { 1. } else { 1. / (k as f64).powf(alf) };
            let mtot: f64 = (ms..=me).map(wght).sum();
            let mut mtm = vec![0f64; c.len()];
            let (mut mave1, mut mtmstr) = (0., 0.);
            engine(x, from, to, 50, |i, cb, mp| {
                if cb == 1 { return vec![]; }
                let m = |k: usize| -> f64 {
                    if typ == 0. { c[i] - c[i - k] } else if c[i - k] > 0. { 100. * (c[i] - c[i - k]) / c[i - k] } else { 0. }
                };
                let first = m(ms) * wght(ms);
                let (mut mx, mut mn, mut mave) = (first, first, first);
                for k in ms + 1..=me {
                    let t = m(k) * wght(k);
                    mave += t;
                    if t > mx { mx = t; }
                    if t < mn { mn = t; }
                }
                mave /= mtot;
                mtm[i] = mx - mn;
                let (hv, lv) = (hh(&mtm, i, me), ll(&mtm, i, me));
                if hv - lv > 0. { mtmstr = 100. * (mtm[i] - lv) / (hv - lv); }
                let mut v = vec![];
                if mp <= 0 && mave > bxo && mtmstr > blev { v.push(on_close(1, "bx")); }
                if mp >= 0 && mave < sxo && mtmstr > slev && mave < mave1 { v.push(on_close(-1, "sx")); }
                mave1 = mave;
                v
            })
        }
        "dmst2_maxlik" => {
            let (slen, llen, xoh, xol, alf, xbot) = (g("slen") as usize, g("llen") as usize, g("xoh"), g("xol"), g("alf"), g("xbot"));
            if llen < slen { return None; }
            let nalf = |k: usize| if k == 1 { 1. } else { (k as f64).powf(alf) };
            engine(x, from, to, 50.max(llen + 1), |i, cb, _mp| {
                if cb == 1 { return vec![]; }
                let (mut hmax, mut lmax, mut sum) = (0f64, 0f64, 0.);
                for v in 1..slen { sum += tr(x, i - v); }
                for v in slen..=llen {
                    sum += tr(x, i - v);
                    if sum > 0. {
                        let mult = 1. / ((sum / v as f64) * nalf(v));
                        let (rh, rl) = ((h[i] - l[i - v]) * mult, (h[i - v] - l[i]) * mult);
                        if rh > hmax { hmax = rh; }
                        if rl > lmax { lmax = rl; }
                    }
                }
                let mut v = vec![];
                if hmax >= xoh && lmax < xbot { v.push(mkt(1, "MaxLkhd buy")); }
                if lmax >= xol && hmax < xbot { v.push(mkt(-1, "MaxLkhd sell")); }
                v
            })
        }
        "dmst2_pctupdn" | "dmst2_dolupdn" => {
            let pct = id == "dmst2_pctupdn";
            let sw = g("switch") as u8;
            let (a, b) = if pct { (g("pctup"), g("pctdn")) } else { (g("dup"), g("ddn")) };
            let bs = move |v: f64| if pct { v * (1. + a / 100.) } else { v + a };
            let ss = move |v: f64| if pct { v * (1. - b / 100.) } else { v - b };
            let pf = |i: usize| (h[i] + l[i]) / 2.;
            let (mut position, mut h_ep, mut l_ep, mut sellstop, mut buystop) = (0i8, 0., 0., 0., 0.);
            engine(x, from, to, 50, |i, cb, mp| {
                let (hi, lo, cl) = (h[i], l[i], c[i]);
                if cb == 1 {
                    position = 0;
                    let (he, le) = match sw { 1 => (hi, lo), 2 => (pf(i), pf(i)), 3 => (lo, hi), _ => (cl, cl) };
                    h_ep = he; l_ep = le; sellstop = ss(he); buystop = bs(le);
                    return vec![];
                }
                let (p1, ss1, bs1) = (position, sellstop, buystop);
                let mut v = vec![];
                if p1 >= 0 {
                    let cand = match sw { 1 => hi, 2 => pf(i), 3 => lo, _ => cl };
                    if cand > h_ep { h_ep = cand; }
                    sellstop = ss(h_ep);
                    let hit = if sw == 0 { cl <= ss1 } else { lo < ss1 };
                    if hit {
                        if sw == 0 { v.push(on_close(-1, "UpDn sell on close")); }
                        position = -1;
                        l_ep = match sw { 1 => lo, 2 => pf(i), 3 => hi, _ => cl };
                        buystop = bs(l_ep);
                    }
                }
                if p1 <= 0 {
                    let cand = match sw { 1 => lo, 2 => pf(i), 3 => hi, _ => cl };
                    if cand < l_ep { l_ep = cand; }
                    buystop = bs(l_ep);
                    let hit = if sw == 0 { cl >= bs1 } else { hi > bs1 };
                    if hit {
                        if sw == 0 { v.push(on_close(1, "UpDn buy on close")); }
                        position = 1;
                        h_ep = match sw { 1 => hi, 2 => pf(i), 3 => lo, _ => cl };
                        sellstop = ss(h_ep);
                    }
                }
                if sw > 0 {
                    if mp <= 0 { v.push(stop(1, buystop, "UpDn buy stop")); }
                    if mp >= 0 { v.push(stop(-1, sellstop, "UpDn sell stop")); }
                }
                v
            })
        }
        "dmst2_noisechannel2" => {
            let (nhi, xou, nlo, xod, tik) = (g("nhi") as usize, g("xou"), g("nlo") as usize, g("xod"), g("tik"));
            let nsdn = if xod < 0. { xou } else { xod };
            let (mut hp, mut lp) = (0., 0.);
            engine(x, from, to, 30.max(nhi.max(nlo) - 1), |i, cb, mp| {
                let (mut hhv, mut llv) = (hh(h, i, nhi), ll(l, i, nlo));
                let mut v = vec![];
                if cb > 1 {
                    if mp >= 0 && llv < lp { llv = lp; }
                    if mp <= 0 && hhv > hp { hhv = hp; }
                    if mp <= 0 { v.push(stop(1, hhv + xou + tik, "NoiseCh2 buy")); }
                    if mp >= 0 { v.push(stop(-1, llv - nsdn - tik, "NoiseCh2 sell")); }
                }
                hp = hhv; lp = llv;
                v
            })
        }
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    /// deterministic noisy walk (LCG) so breakout stops actually trigger
    fn walk(n: usize) -> Vec<Row> {
        let mut s: u64 = 7;
        let mut v = 100.;
        (0..n).map(|i| {
            s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            let u = ((s >> 33) as f64 / (1u64 << 31) as f64) - 0.5;
            let o = v;
            v = (v * (1. + 0.08 * u + 0.004 * ((i as f64) / 40.).sin())).max(1.);
            let (hi, lo) = (o.max(v) * 1.002, o.min(v) * 0.998);
            Row { ticker: "T".into(), date: format!("{:04}-01-01", 1000 + i), open: o, high: hi, low: lo, close: v, volume: None }
        }).collect()
    }
    #[test]
    fn lsq_and_quadratic_weights_forecast_exactly() {
        // weights must extrapolate a line (t+1) and a parabola (T**2) one bar ahead
        let n = 30usize; let jf = n as f64;
        let (b1, b2) = (12. / (jf * (jf - 1.) * (jf + 1.)), 6. / (jf * (jf - 1.)));
        let f = |t: f64| 5. + 0.3 * t;
        let est: f64 = (1..=n).map(|ii| { let t = (n - ii + 1) as f64; let b0w = b1 * t - b2; (1. / jf + b0w * (jf + 1.) / 2.) * f(t) }).sum();
        assert!((est - f(jf + 1.)).abs() < 1e-9);
        let n = 43usize; let jf = n as f64; let bott = (jf - 1.) * (jf - 2.);
        let q = |t: f64| 1. + 0.2 * t - 0.01 * t * t;
        let est: f64 = (1..=n).map(|t| { let t = t as f64; 3. / jf * ((jf + 3.) * (jf + 2.) / bott - 2. * (4. * jf + 7.) / bott * t + 10. / bott * t * t) * q(t) }).sum();
        assert!((est - q(jf + 1.)).abs() < 1e-9);
    }
    #[test]
    fn stop_orders_fill_at_level_or_gap_open_and_trades_chain() {
        let mut r = walk(300);
        r[200].open = r[199].close * 1.1; r[200].high = r[200].open * 1.01; r[200].close = r[200].open; // a gap bar
        let x = Ctx::new(&r, &r, &[]);
        for id in ["dmst2_noisechannel", "dmst2_noisechannel2", "dmst2_multibar", "dmst2_xavtr", "dmst2_parabxox"] {
            let t = run(id, &x, 0, 299, &Params::new()).unwrap();
            assert!(t.len() > 2, "{id}");
            for k in &t {
                let i = k.legs[0].i;
                assert!(k.legs[0].px >= x.l[i] - 1e-9 && k.legs[0].px <= x.h[i] + 1e-9, "{id} fill outside range");
            }
            for w in t.windows(2) { assert_eq!(w[0].exit_i, w[1].legs[0].i, "{id}"); }
        }
    }
    #[test]
    fn every_system_runs_truncation_keeps_earlier_trades() {
        let r = walk(500);
        let x = Ctx::new(&r, &r, &[]);
        for s in catalogue() {
            let full = run(s.id, &x, 0, 499, &Params::new()).unwrap();
            assert!(!full.is_empty(), "{}", s.id);
            let cut = run(s.id, &x, 0, 300, &Params::new()).unwrap();
            for t in cut.iter().filter(|t| !t.open) {
                assert!(full.iter().any(|f| f.legs[0].i == t.legs[0].i && f.exit_i == t.exit_i && f.exit_px == t.exit_px), "{}", s.id);
            }
        }
        assert!(run("dmst2_pctupdn", &x, 0, 499, &Params::from([("switch".into(), 1.5)])).is_none());
    }
}
