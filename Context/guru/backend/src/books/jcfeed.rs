//! John Clayburg, Feeder Trader (c) 1999, clayburg.com. Source (research drive, read in place):
//! `T:/John Clayburg - Feeder Trader System for TS (clayburg.com)/Code.rar` -> `Code.txt`, plus `Feeder_Trader_setup.txt`
//! and `System Report.doc` (Feeder Cattle 1/2000-10/29/2004). Book id "jcfeed". Notes: library/jcfeed/README.md.
//! Also hosts the small order/position helpers shared by the RAR batch-77 modules (alfrd, ltsfut, qsfund, rvecl, sktbw).
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "jcfeed";
/// Admitted ids (authors/clayburg/admission/SOURCE-CONTRACT.md).
pub const REVIEWED: &[&str] = &["jcfeed_feeder_trader"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }
const FT: &str = "jcfeed_feeder_trader";

// ---------------------------------------------------------------- shared helpers (TradeStation order semantics)

/// Position book. Orders are applied one by one in TradeStation code order: an entry in the direction already held
/// is ignored (no pyramiding), an opposite entry reverses, an exit applies only to the side it names.
pub struct Sim { pub out: Vec<Trade>, pub cur: Option<Trade> }

impl Sim {
    pub fn new() -> Self { Sim { out: vec![], cur: None } }
    pub fn pos(&self) -> i8 { self.cur.as_ref().map_or(0, |t| t.dir as i8) }
    pub fn entry_i(&self) -> usize { self.cur.as_ref().map_or(0, |t| t.legs[0].i) }
    pub fn enter(&mut self, dir: i8, i: usize, px: f64, sig: usize, kind: &'static str, name: &str) {
        if self.pos() == dir { return; }
        self.close(i, px, sig, &format!("reversed by {name}"));
        self.cur = Some(Trade { dir: dir as f64, legs: vec![Leg { i, px, w: 1. }], exit_i: i, exit_px: px, reason: String::new(),
            entry_kind: kind, signal_i: sig, open: false, exits: vec![] });
    }
    pub fn exit(&mut self, side: i8, i: usize, px: f64, sig: usize, name: &str) {
        if self.pos() == side && side != 0 { self.close(i, px, sig, name); }
    }
    fn close(&mut self, i: usize, px: f64, sig: usize, reason: &str) {
        if let Some(mut t) = self.cur.take() { t.exit_i = i; t.exit_px = px; t.signal_i = sig; t.reason = reason.into(); self.out.push(t); }
    }
    pub fn finish(mut self, x: &Ctx, to: usize) -> Vec<Trade> {
        if let Some(mut t) = self.cur.take() { t.exit_i = to; t.exit_px = x.c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true; self.out.push(t); }
        self.out
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Ord { BuyE, SellE, XL, XS }

/// One daily bar of stop orders, TradeStation daily-bar path O -> nearer extreme -> far extreme -> C (same rule as rarsys).
/// Gap through at the open fills at the open. Returns fills (price, order, name) in path order and updates `pos`.
pub fn stop_bar(o: f64, h: f64, l: f64, c: f64, orders: &[(f64, Ord, &'static str)], pos: &mut i8) -> Vec<(f64, Ord, &'static str)> {
    let up = |k: Ord| matches!(k, Ord::BuyE | Ord::XS);
    let applies = |k: Ord, pos: i8| match k { Ord::BuyE => pos <= 0, Ord::SellE => pos >= 0, Ord::XL => pos == 1, Ord::XS => pos == -1 };
    let mut fills = vec![];
    let apply = |px: f64, k: Ord, name: &'static str, pos: &mut i8, fills: &mut Vec<_>| {
        if applies(k, *pos) { *pos = match k { Ord::BuyE => 1, Ord::SellE => -1, _ => 0 }; fills.push((px, k, name)); }
    };
    for pass in [true, false] {
        for &(lv, k, nm) in orders {
            let is_exit = matches!(k, Ord::XL | Ord::XS);
            if is_exit == pass && if up(k) { lv <= o } else { lv >= o } { apply(o, k, nm, pos, &mut fills); }
        }
    }
    let path = if h - o <= o - l { [o, h, l, c] } else { [o, l, h, c] };
    for s in path.windows(2) {
        let (a, b) = (s[0], s[1]);
        let mut cand: Vec<_> = orders.iter().copied().filter(|&(lv, k, _)| if b > a { up(k) && lv > a && lv <= b } else if b < a { !up(k) && lv < a && lv >= b } else { false }).collect();
        if b > a { cand.sort_by(|p, q| p.0.total_cmp(&q.0)) } else { cand.sort_by(|p, q| q.0.total_cmp(&p.0)) }
        for (lv, k, nm) in cand { apply(lv, k, nm, pos, &mut fills); }
    }
    fills
}

/// Apply a bar's stop orders to the book (entries reverse, exits flatten).
pub fn stops_into(sim: &mut Sim, x: &Ctx, j: usize, orders: &[(f64, Ord, &'static str)]) {
    let mut pos = sim.pos();
    for (px, k, nm) in stop_bar(x.o[j], x.h[j], x.l[j], x.c[j], orders, &mut pos) {
        match k {
            Ord::BuyE => sim.enter(1, j, px, j, "stop", nm),
            Ord::SellE => sim.enter(-1, j, px, j, "stop", nm),
            Ord::XL => sim.exit(1, j, px, j, nm),
            Ord::XS => sim.exit(-1, j, px, j, nm),
        }
    }
}

/// Highest / lowest of v[i+1-n ..= i] (TradeStation Highest/Lowest include the current bar).
pub fn ext(v: &[f64], i: usize, n: usize, hi: bool) -> f64 {
    let w = &v[i + 1 - n..=i];
    if hi { w.iter().copied().fold(f64::MIN, f64::max) } else { w.iter().copied().fold(f64::MAX, f64::min) }
}

pub fn gp(p: &Params, k: &str, d: f64) -> f64 { p.get(k).copied().unwrap_or(d) }

// ---------------------------------------------------------------- Feeder Trader

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: FT, name: "Feeder Trader (open +/- range-fraction stop and reverse)", group: "Feeder Trader (Code.rar)", session: 0, dir: "Long/Short",
        status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules: &[
            "F|Code.txt: \"HHB = Highest(H,B); LCB = Lowest((h+l+ClFctr*c)/(ClFctr+2),B); HCB = Highest((h+l+ClFctr*c)/(ClFctr+2),B); LLB = Lowest(L,B)\" (B 3, ClFctr 1)",
            "F|\"XB = MaxList((HHB-LCB),(HCB-LLB))*(B1*0.100)\" (B1 6); \"XS = MaxList((HHS-LCS),(HCS-LLS))*(S1*0.100)\" (S 1, S1 7)",
            "E|\"If MarketPosition < 1 then Buy Next Bar Round((Open of Tomorrow + XB), Prec) stop\" (Prec 2)",
            "E|\"If MarketPosition > -1 then Sell Short Next Bar Round((Open of Tomorrow - XS), Prec) stop\"",
            "X|No exit order: 'Reversal System based solely on Entry Points' - each entry stop reverses the position",
        ],
        notes: "Setup file: 'The defaults are shown in parenthesis and are set for the Feeder Cattle Market ... designed to End-of-Day trading and takes trades at the Open of Tomorrow signaled today' (daily bars). \
Defaults are the code inputs B 3, S 1, B1 6, S1 7. The stops are placed relative to the next bar's open (Open of Tomorrow), so they cannot gap through except when the range term is 0. \
Fills [Assumption, code silent - TradeStation daily-bar path, as rarsys]: stop fills at its level; bar runs open -> nearer extreme -> far extreme -> close; when flat (first trade only) both stops are live and both can fill on one bar (entry then reversal). \
Published results are Feeder Cattle futures in dollars (System Report.doc, 1/2000-10/29/2004: net $57,755, 53.65% profitable, PF 1.97) - no comparable data in the vault. \
Built for a futures contract; run on the indices as an instrument swap (no dollar or tick constants in the rules). India: instrument swap; index shorts hypothetical.",
        claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == FT).then_some((FT, "Feeder Trader (Clayburg)", "TradeStation code literal"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    (id == FT).then(|| vec![p("b", "B (buy lookback bars)", 3., 1., 10., 1.), p("s", "S (sell lookback bars)", 1., 1., 10., 1.),
        p("b1", "B1 (buy range fraction x 0.1)", 6., 1., 15., 1.), p("s1", "S1 (sell range fraction x 0.1)", 7., 1., 15., 1.)])
}

fn round2(v: f64) -> f64 { (v * 100.).round() / 100. }

fn run_ft(x: &Ctx, from: usize, to: usize, p: &Params) -> Vec<Trade> {
    let (b, s) = (gp(p, "b", 3.).max(1.) as usize, gp(p, "s", 1.).max(1.) as usize);
    let (b1, s1) = (gp(p, "b1", 6.), gp(p, "s1", 7.));
    let typ: Vec<f64> = (0..x.c.len()).map(|i| (x.h[i] + x.l[i] + x.c[i]) / 3.).collect();
    let rng = |i: usize, n: usize| (ext(&x.h, i, n, true) - ext(&typ, i, n, false)).max(ext(&typ, i, n, true) - ext(&x.l, i, n, false));
    let mut sim = Sim::new();
    for i in from.max(b.max(s) - 1)..to {
        let j = i + 1;
        let mut ords = vec![];
        if sim.pos() < 1 { ords.push((round2(x.o[j] + rng(i, b) * (b1 * 0.1)), Ord::BuyE, "Buy stop")); }
        if sim.pos() > -1 { ords.push((round2(x.o[j] - rng(i, s) * (s1 * 0.1)), Ord::SellE, "Sell Short stop")); }
        stops_into(&mut sim, x, j, &ords);
    }
    sim.finish(x, to)
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if id != FT || from >= to || to >= x.r.len() { return None; }
    Some(run_ft(x, from, to, p))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stop_bar_flat_both_stops() {
        // flat, open 100, low nearer: 97 first (sell 98 fills), then high 105 (buy 103 reverses)
        let mut pos = 0i8;
        let f = stop_bar(100., 105., 97., 101., &[(103., Ord::BuyE, "B"), (98., Ord::SellE, "S")], &mut pos);
        assert_eq!(f.iter().map(|z| (z.0, z.2)).collect::<Vec<_>>(), vec![(98., "S"), (103., "B")]);
        assert_eq!(pos, 1);
    }
}

// Instrument admission: Clayburg 2026-10-25 block, moved from instrument_policy.rs on 31 Oct 2026 (lane ST).
// Admission only for the module REVIEWED lists (authors/clayburg/admission/SOURCE-CONTRACT.md); kept in one block to merge.
// Runs on actual indices, ETFs and stocks, US and India (index_etf_or_stock, nifty50 basket, declared application).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book=="jcfeed" && crate::books::jcfeed::reviewed(def.id),
    label: "John Clayburg built and reported Feeder Trader on Feeder Cattle futures; runs on indices, ETFs and single stocks (US and India) are a declared application (range-fraction stops, no dollar or tick constants).",
    basis: "Feeder Trader (c) 1999, clayburg.com: Code.txt (Code.rar), Feeder_Trader_setup.txt (defaults B 3, S 1, B1 6, S1 7; end-of-day, orders at tomorrow's open). Index/ETF/stock runs are a declared application.",
};
