//! TradeStation, "EasyLanguage Home Study Course" (2007; research share USB HDD 1/TradeStation/_Knowleage/EasyLanguage Home Study
//! Course, $149 (tradestation.com)/EasyLanguage Home Study Course.pdf, printed page = PDF page - 10; ocr2 text authors/batch-e/E7w/
//! elhsc_p*.txt). Book id `tselhs`. Exercises #24 Breakout, #25 Mov Avg Cross, #26 Real Body Avg and #27 Momentum Cross, coded as
//! labelled teaching examples (R4 decisions 49, 50, 57, 58). Lane K11 (titles loop, 1 Nov 2026), to-code rows 183-186; independent
//! check authors/titles-1030/K11w/k11_ref.py. Also holds the TradeStation order helpers used by `ttelru` and `omelgs`.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use crate::connors::Leg;
use super::lc3::{hi, high_first, lo};
use super::msws::sma;

/// Admitted ids (authors/titles-1030/K11w/k11_ref.py (header)).
pub const REVIEWED: &[&str] = &["tselhs_breakout8", "tselhs_ma_cross", "tselhs_rb_avg", "tselhs_mom_cross"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

macro_rules! common { () => { " TradeStation semantics: the strategy runs at each bar's close; 'next bar' orders are for the following bar only (a market order fills at its open; a stop fills at the stop price, or at the open if the open is already through it; a limit fills at the limit, or at the open if the open is already better); one position, no pyramiding, a same-direction entry is ignored and an opposite entry reverses (TradeStation SellShort / Buy). Daily bars are a declared application: the course's chart examples are intraday CSCO, INTC and AAPL. A teaching example: the course teaches EasyLanguage and does not recommend these strategies (lead decision 49). No printed results; the vault's standard costs (decision 50)." } }

const BRK: &str = concat!("[Declared readings] Exercise #24 Breakout (printed p.86-92): at every close, Buy next bar at the highest high of the last 8 bars (current bar included, 'we do not refer to 1 bar ago') + .02 stop and Sell Short next bar at the lowest low of the last 8 bars - .02 stop; 'there are no exits in this strategy since it continually closes and then reverses the open position'. The .02 / '2 points' offset = 2 minimum price ticks of the instrument (0.01 US, 0.05 NSE; lead decision 57); p.tick = 0 removes it. While long only the sell-short stop can act (and the mirror); when flat and one bar reaches both stops, the one nearer the open fills first and the other is not taken that bar. Option p.n = 4: the Russian Omega course 'BreakOut' (same code with 4 bars and 2-point stops). Not coded: #29 Breakout2's built-in $0.20 stop, $0.30 target and $0.15 breakeven per share, tuned to 10-minute bars and active inside the entry bar, which daily bars cannot order.", common!());
const MAC: &str = concat!("[Declared readings] Challenge #25 Mov Avg Cross (task p.92, answer code p.210-211): ShortMA = Average(Close,9), LongMA = Average(Close,18); ShortMA crosses over LongMA -> Buy next bar at market, crosses under -> Sell Short next bar at market (each reverses). Stale-trade exit: if BarsSinceEntry > 8 (the entry bar is 0) and OpenPositionProfit < $150 at the close -> Sell / Buy To Cover next bar at market, then flat until the next cross; a cross on the same close wins (it reverses). $150 on the course's 100-share trade size (p.91) = $1.50 a share. Dollar amount: US instruments only (lead decision 58); on Indian instruments the crossover runs without it ('dollar exit held'). p.stale = 0 removes it everywhere. The Russian Omega course version 'MACross' (this bar on close, PositionProfit < 1000) is not offered: its dollar amount has no stated share count and dollar amounts are not converted (lead decisions 19, 32, 63).", common!());
const RB: &str = concat!("[Declared readings] Exercise #26 Real Body Avg (printed p.93-95): RB = Close - Open, AvgRB = Average(RB,10) (simple). AvgRB crosses over 0 (above 0 now, at or below 0 on the prior bar) -> Buy next bar at market; crosses under 0 (below now, at or above before) -> Sell Short next bar at market. Always in after the first signal; no stop, no other exit.", common!());
const MOM: &str = concat!("[Declared readings] Exercise #27 Momentum Cross (printed p.96-98, final listing): Mom = Close - Close[10]. BullCx = Mom crosses over 0, BearCx = Mom crosses under 0. BullCx and MRO(BearCx,4,1) = -1 (no BearCx in the last 4 bars, current bar included) -> Buy next bar at a limit = this bar's close; BearCx with no BullCx in the last 4 bars -> Sell Short next bar at a limit = this bar's close (a short fill reverses a long, and the mirror). Exits, checked at every close while in a position, the entry bar included: Mom < Mom[1] and Mom[1] < Mom[2] -> Sell this bar on close; Mom > Mom[1] > Mom[2] -> Buy To Cover this bar on close. Option p.exit = 1: the first draft on p.96, exits next bar at market.", common!());

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, rules, notes| StrategyDef { id, name, group: "TradeStation EasyLanguage Home Study Course exercises", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "tselhs", rules, notes, claim: None };
    vec![
        d("tselhs_breakout8", "#24 Breakout: stop and reverse 2 ticks beyond the 8-bar high / low", &[
            "E|\"BuyPx = Highest(High, 8); SellPx = Lowest(Low, 8); Buy (\\\"BrkBuy\\\") next bar at BuyPx + .02 Stop; Sell Short (\\\"BrkSs\\\") next bar at SellPx - .02 Stop;\" (p.86); input version \"+ 2 points Stop\" (p.87)",
            "X|\"there are no exits in this strategy since it continually closes and then reverses the open position\" (p.91)"], BRK),
        d("tselhs_ma_cross", "#25 Mov Avg Cross: 9/18 average cross, stop and reverse, out after 8 bars if the profit is under $150 (US)", &[
            "E|\"If ShortMA crosses over LongMA then Buy next bar at market; If ShortMA crosses under LongMA then Sell Short next bar at market;\" Input: ShortLen(9), LongLen(18) (p.210)",
            "X|\"If BarsSinceEntry > MinHold and OpenPositionProfit < MinProf then begin Sell next bar at market; Buy to Cover next bar at market; end;\" Input: MinHold(8), MinProf(150) (p.211)"], MAC),
        d("tselhs_rb_avg", "#26 Real Body Avg: 10-bar average of close minus open crossing zero, stop and reverse", &[
            "F|\"RB = Close - Open; AvgRB = Average(RB, Length);\" Inputs: Length(10) (p.94)",
            "E|\"Condition1 = AvgRB crosses over 0; Condition2 = AvgRB crosses under 0; If Condition1 then Buy next bar at market; If Condition2 then Sell Short next bar at market;\" (p.94-95)",
            "X|No exit is written: each entry reverses the open position (Buy covers a short, Sell Short closes a long)."], RB),
        d("tselhs_mom_cross", "#27 Momentum Cross: 10-bar momentum crossing zero, limit entries at the close, out after 2 bars of falling (rising) momentum", &[
            "E|\"Mom = Momentum(Close,BarsAgo); BullCx = Mom crosses over 0; BearCx = Mom crosses under 0; If BullCx and MRO(BearCx,4,1) = -1 then Buy next bar at Close of this bar limit; If BearCx and MRO(BullCx,4,1) = -1 then SellShort next bar at Close of this bar limit;\" Input: BarsAgo(10) (p.97-98)",
            "X|\"If Mom < Mom[1] and Mom[1] < Mom[2] then Sell this bar on Close; If Mom > Mom[1] and Mom[1] > Mom[2] then BuytoCover this bar on Close;\" (p.98)"], MOM),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let f = match id {
        "tselhs_breakout8" => ("EL Home Study #24 Breakout", "8-bar stops, 2 ticks"),
        "tselhs_ma_cross" => ("EL Home Study #25 Mov Avg Cross", "9/18, stale exit"),
        "tselhs_rb_avg" => ("EL Home Study #26 Real Body Avg", "10-bar body average"),
        "tselhs_mom_cross" => ("EL Home Study #27 Momentum Cross", "momentum 10, limits"),
        _ => return None,
    };
    let id = REVIEWED.iter().find(|r| **r == id)?;
    Some((*id, f.0, f.1))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(match id {
        "tselhs_breakout8" => vec![p("n", "Bars in the high / low (8; Russian course BreakOut: 4)", 8., 2., 50., 1.), p("tick", "Stop offset in ticks (2; 0 = none)", 2., 0., 10., 1.)],
        "tselhs_ma_cross" => vec![p("stale", "Stale-trade exit, US only: 1 on (course), 0 off", 1., 0., 1., 1.)],
        "tselhs_rb_avg" => vec![],
        "tselhs_mom_cross" => vec![p("exit", "Exits: 0 = this bar on close (final listing), 1 = next bar at market (first draft)", 0., 0., 1., 1.)],
        _ => return None,
    })
}

/// Indian instrument (NSE index or a native NSE stock): dollar amounts are not applied there (lead decisions 32, 58).
pub fn india(x: &Ctx) -> bool { x.native_market == Some(crate::baskets::Market::India) || x.r.first().is_some_and(|r| r.ticker.starts_with("NIFTY")) }
/// TradeStation 'crosses over' / 'crosses under' a level or series: beyond it now, at or on the other side on the prior bar.
pub fn over(a: &[f64], b: &[f64], i: usize) -> bool { i > 0 && a[i] > b[i] && a[i - 1] <= b[i - 1] }
pub fn under(a: &[f64], b: &[f64], i: usize) -> bool { i > 0 && a[i] < b[i] && a[i - 1] >= b[i - 1] }
/// A one-bar limit entry on bar i: fill price, or None. Buy limit fills at or below px (the open if it is already lower).
pub fn limit_fill(x: &Ctx, i: usize, d: f64, px: f64) -> Option<f64> {
    if d > 0. { (x.l[i] <= px).then(|| x.o[i].min(px)) } else { (x.h[i] >= px).then(|| x.o[i].max(px)) }
}
/// A one-bar stop on bar i (d = direction of the order): fill price, or None. A buy stop fills at or above px.
pub fn stop_fill(x: &Ctx, i: usize, d: f64, px: f64) -> Option<f64> {
    if d > 0. { (x.h[i] >= px).then(|| x.o[i].max(px)) } else { (x.l[i] <= px).then(|| x.o[i].min(px)) }
}

fn g(p: &Params, k: &str, id: &str) -> f64 {
    p.get(k).copied().unwrap_or_else(|| params(id).and_then(|s| s.iter().find(|d| d.key == k).map(|d| d.def)).unwrap_or(f64::NAN))
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let (o, h, l, c, len) = (&x.o, &x.h, &x.l, &x.c, x.c.len());
    let mut pos = Book::new();
    match id {
        "tselhs_breakout8" => {
            let n = g(p, "n", id).round().max(1.) as usize;
            let off = g(p, "tick", id).round().max(0.) * x.market_tick();
            for i in from + 1..=to {
                let t = i - 1;
                if t + 1 < n { continue; }
                let (bs, ss) = (hi(h, t + 1 - n, t) + off, lo(l, t + 1 - n, t) - off);
                let (up, dn) = (stop_fill(x, i, 1., bs), stop_fill(x, i, -1., ss));
                let side = pos.side();
                let pick = match (up, dn) {
                    (Some(u), Some(d)) if side == 0. => if high_first(o[i], h[i], l[i]) { Some((1., u)) } else { Some((-1., d)) },
                    (Some(u), _) if side <= 0. => Some((1., u)),
                    (_, Some(d)) if side >= 0. => Some((-1., d)),
                    _ => None,
                };
                if let Some((d, px)) = pick { pos.enter(d, i, px, "stop"); }
            }
        }
        "tselhs_ma_cross" => {
            let (s, lg) = (sma(c, 9), sma(c, 18));
            let us = g(p, "stale", id) > 0.5 && !india(x);
            let mut pend: Option<f64> = None; // next-open order: +1 / -1 entry (reversing), 0 exit
            for i in from..=to {
                if let Some(a) = pend.take() {
                    if a == 0. { pos.exit(i, o[i], "stale trade"); } else { pos.enter(a, i, o[i], "open"); }
                }
                let sig = if over(&s, &lg, i) { 1. } else if under(&s, &lg, i) { -1. } else { 0. };
                // $150 on the course's 100-share trade size (p.91) = $1.50 a share
                let stale = us && pos.entry_bar().is_some_and(|e| i - e > 8) && pos.side() * (c[i] - pos.entry_px()) < 1.5;
                if i < to {
                    if sig != 0. && sig != pos.side() { pend = Some(sig); } else if stale { pend = Some(0.); }
                }
            }
        }
        "tselhs_rb_avg" => {
            let rb: Vec<f64> = (0..len).map(|i| c[i] - o[i]).collect();
            let a = sma(&rb, 10);
            let z = vec![0.; len];
            let mut pend: Option<f64> = None;
            for i in from..=to {
                if let Some(d) = pend.take() { pos.enter(d, i, o[i], "open"); }
                if i < to { if over(&a, &z, i) { pend = Some(1.); } else if under(&a, &z, i) { pend = Some(-1.); } }
            }
        }
        "tselhs_mom_cross" => {
            let mom: Vec<f64> = (0..len).map(|i| if i >= 10 { c[i] - c[i - 10] } else { f64::NAN }).collect();
            let z = vec![0.; len];
            let (bull, bear): (Vec<bool>, Vec<bool>) = ((0..len).map(|i| over(&mom, &z, i)).collect(), (0..len).map(|i| under(&mom, &z, i)).collect());
            let none_in = |v: &[bool], i: usize| (i.saturating_sub(3)..=i).all(|k| !v[k]);
            let next_open = g(p, "exit", id) > 0.5;
            let (mut lim, mut pexit): (Option<(f64, f64)>, bool) = (None, false);
            for i in from..=to {
                if std::mem::take(&mut pexit) { pos.exit(i, o[i], "momentum turned"); }
                if let Some((d, px)) = lim.take() {
                    if pos.side() != d { if let Some(f) = limit_fill(x, i, d, px) { pos.enter(d, i, f, "limit"); } }
                }
                if i >= 12 && pos.side() != 0. {
                    let d = pos.side();
                    if d * (mom[i] - mom[i - 1]) < 0. && d * (mom[i - 1] - mom[i - 2]) < 0. {
                        if !next_open { pos.exit(i, c[i], "momentum turned"); } else if i < to { pexit = true; }
                    }
                }
                if i < to {
                    if bull[i] && none_in(&bear, i) { lim = Some((1., c[i])); } else if bear[i] && none_in(&bull, i) { lim = Some((-1., c[i])); }
                }
            }
        }
        _ => return None,
    }
    Some(pos.done(to, c[to]))
}

/// One position at a time (TradeStation without pyramiding): a same-direction entry is ignored, an opposite one reverses.
pub struct Book { pub out: Vec<Trade>, cur: Option<(f64, usize, f64, &'static str)> }
impl Book {
    pub fn new() -> Self { Book { out: vec![], cur: None } }
    pub fn side(&self) -> f64 { self.cur.map_or(0., |c| c.0) }
    pub fn entry_bar(&self) -> Option<usize> { self.cur.map(|c| c.1) }
    pub fn entry_px(&self) -> f64 { self.cur.map_or(f64::NAN, |c| c.2) }
    pub fn exit(&mut self, i: usize, px: f64, why: &str) {
        if let Some((d, ei, ep, k)) = self.cur.take() {
            self.out.push(Trade { dir: d, legs: vec![Leg { i: ei, px: ep, w: 1. }], exit_i: i, exit_px: px, reason: why.into(), entry_kind: k, signal_i: i, open: false, exits: vec![] });
        }
    }
    pub fn enter(&mut self, d: f64, i: usize, px: f64, kind: &'static str) {
        if self.side() == d { return; }
        self.exit(i, px, "reversed");
        self.cur = Some((d, i, px, kind));
    }
    pub fn done(mut self, to: usize, c_to: f64) -> Vec<Trade> {
        if let Some((d, ei, ep, k)) = self.cur.take() {
            self.out.push(Trade { dir: d, legs: vec![Leg { i: ei, px: ep, w: 1. }], exit_i: to, exit_px: c_to, reason: "open".into(), entry_kind: k, signal_i: to, open: true, exits: vec![] });
        }
        self.out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    pub fn bars() -> Vec<Row> {
        (0..900).map(|i| { let f = i as f64; let c = 100. + 9. * (f / 17.).sin() + 4. * (f / 5.3).cos() + 0.02 * f;
            Row { ticker: "T".into(), date: format!("{:05}", i), open: c - 0.6 * (f / 2.1).sin(), high: c + 1.3, low: c - 1.1, close: c, volume: None } }).collect()
    }
    #[test]
    fn home_study_exercises_trade_both_sides() {
        let r = bars();
        let x = Ctx::new(&r, &r, &[]);
        for id in REVIEWED {
            let t = run(id, &x, 0, r.len() - 1, &Params::new()).unwrap();
            assert!(t.len() > 5 && t.iter().any(|z| z.dir > 0.) && t.iter().any(|z| z.dir < 0.), "{id}");
            for w in t.windows(2) { assert!(w[1].legs[0].i >= w[0].exit_i, "{id}"); }
            assert!(family_of(id).is_some() && params(id).is_some());
        }
        // #24: every fill is at or beyond the stop (8-bar extreme +/- 2 ticks of 0.01)
        for z in run("tselhs_breakout8", &x, 0, r.len() - 1, &Params::new()).unwrap() {
            let (i, px) = (z.legs[0].i, z.legs[0].px);
            if z.dir > 0. { assert!(px >= hi(&x.h, i - 8, i - 1) + 0.02 - 1e-9); } else { assert!(px <= lo(&x.l, i - 8, i - 1) - 0.02 + 1e-9); }
        }
        assert!(run("tselhs_x", &x, 0, 5, &Params::new()).is_none());
    }
}

// Instrument admission: TradeStation EasyLanguage Home Study Course 2026-11-01 (titles lane K11).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "tselhs" && reviewed(def.id),
    label: "Teaching exercises from TradeStation's EasyLanguage Home Study Course, shown on intraday CSCO, INTC and AAPL charts with no results. Runs on daily indices, ETFs and stocks, US and India, are a declared application.",
    basis: "EasyLanguage Home Study Course (2007), exercises #24-#27, printed p.86-98 and answers p.210-211; independent check authors/titles-1030/K11w/k11_ref.py",
};
