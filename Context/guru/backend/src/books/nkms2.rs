//! NIKSAN MetaStock formula pack, part 2 (lane K2, 30 Oct 2026; part 1 = module `nkms`, lane K1). Research drive
//! USB HDD 1/MetaStock/@Systems & Indicators/@Misc/- Studies - No Sorted/[sm] Metastock Plugin - NIKSAN's MS Formula
//! All/NIKSAN's MSFormula3/MS72PRFT.DTA (read in place; read-only): 97 MetaStock 7.2 System Tester records, among them
//! Equis's "PS ..." sample systems. This module holds the PS records from "Impulse Meter" to "Volume Trading System"
//! (record indexes 54-96, 0-based) that the title loop sent to coding. Book id `nkms2`.
//! Record layout (same as MS65, books::msadd): header 4184 bytes, records 28371 bytes, name @1024, four formulas
//! (Enter Long, Close Long, Enter Short, Close Short) @1065 + 2500*k; the record's line breaks are shown as spaces.
//! Every record here has no notes and the same all-zero binary stop block (every entry tag 0x4E24, value 0.0) that
//! books::msadd reads as "no stops"; fills = MetaStock defaults (trade price close, trade delay 0), as in msadd.
//! Formulas run as stored through the `msws` formula engine; Mo() through books::mslib::expand (MetaStock 8 manual
//! p.482: Mo(n) = C / Ref(C,-n) * 100). Records using ValueWhen() or PREV are run by hand-written code below that
//! evaluates every other part of the stored text with the same engine. Records needing a MetaStock built-in whose
//! calculation neither the record nor the MetaStock 8 manual prints are NotTestable with the reason.
//! Duplicate check: no record here matches a system in msadd, mslib, mssta, mstt, msws, dgmf or djcs (formula text
//! compared after removing spaces and case; similar names checked by hand). Independent check:
//! authors/niksan-k2/admission/nkms2_ref.py (header).
use crate::books::msws::{self, Book, Eng, Sys, EXEC_DEFAULT};
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
#[allow(unused_imports)]
use crate::connors::Status::{Live, NeedsData, NotTestable};
use std::sync::OnceLock;

pub const BOOK: &str = "nkms2";
/// Admitted ids (authors/niksan-k2/admission/nkms2_ref.py (header)). Not admitted although Live: nkms2_rsi_trailing_exit
/// (its trades depend on whether an opposite Enter signal closes the open position, a System Tester setting the record
/// does not store; authors/niksan-k2/admission/alt-check.txt).
pub const REVIEWED: &[&str] = &["nkms2_landis_reversal", "nkms2_linear_regression_crossover", "nkms2_long_sell_short_sale_5_day", "nkms2_macd_histogram_1", "nkms2_macd_histogram_2", "nkms2_meisels_overbought_oversold", "nkms2_modified_momentum", "nkms2_momentum", "nkms2_new_relative_volatility_index", "nkms2_oscillator_composite", "nkms2_pattern_trading_system_1", "nkms2_pattern_trading_system_2", "nkms2_pattern_trading_system_3", "nkms2_pattern_trading_system_4", "nkms2_pattern_trading_system_5", "nkms2_pattern_trading_system_6", "nkms2_percentage_crossover_3pct", "nkms2_short_term_range_rejection", "nkms2_small_triangle_breakout", "nkms2_starcband_system", "nkms2_stochastic_oscillator_system", "nkms2_stochrsi", "nkms2_ultimate_oscillator", "nkms2_volatility_breakout_standard", "nkms2_volatility_stop", "nkms2_volume_trading_system"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const GROUP: &str = "NIKSAN MetaStock pack · Equis PS system tests (MS72PRFT.DTA, part 2)";
const FAMILY: (&str, &str) = ("nkms2_ps", "Equis PS System Tester samples (NIKSAN pack, part 2)");
const FML: &[(&str, &str)] = &[("MO_100", "100")];
const CONV: &str = "One position at a time; a position closes only by its own Close formula (checked from the bar after entry); an exit plus an opposite entry on one bar reverse; no same-direction re-entry on the exit bar; parameter side = 1/-1 runs one side alone. The record has no stops (all-zero stop block) and no fill field: MetaStock defaults, trade at the close, no delay. Indicators: MetaStock definitions as in module msws. Runs on indices, ETFs and single stocks (US and India) are a declared application: the record names no market and prints no results.";
static NK: Book = Book { id: BOOK, sys: SYS, fml: FML, conv: CONV, cat: OnceLock::new(), par: OnceLock::new() };
/// Records run by hand-written code (ValueWhen / PREV); only the `side` parameter.
const HAND: &[&str] = &["nkms2_macd_histogram_1", "nkms2_macd_histogram_2", "nkms2_percentage_crossover_3pct", "nkms2_volatility_stop"];

pub fn catalogue() -> Vec<StrategyDef> { NK.catalogue() }
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { NK.family_of(id) }
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    if HAND.contains(&id) {
        SYS.iter().find(|s| s.id == id && s.status == Status::Live)?;
        return Some(vec![ParamDef { key: "side", label: "Sides traded: 0 = both as stored, 1 = long side only, -1 = short side only", def: 0., min: -1., max: 1., step: 1. }]);
    }
    NK.params(id)
}

fn lookup(n: &str) -> Option<String> { NK.fml_lookup(n) }
pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let s = SYS.iter().find(|s| s.id == id)?;
    if s.status != Status::Live || from >= to || to >= x.r.len() { return None; }
    let side = p.get("side").copied().unwrap_or(0.0).round() as i32;
    let fl: &dyn Fn(&str) -> Option<String> = &lookup;
    let mut eng = Eng::new(x, fl);
    let sig = if HAND.contains(&id) { hand(id, &mut eng, x, side).ok()? } else {
        let raw: Vec<&str> = s.f.iter().flatten().copied().collect();
        let map: Vec<(f64, f64)> = msws::constants(&raw).into_iter().map(|c| (c, p.get(&msws::const_key(c)).copied().unwrap_or(c))).collect();
        let e: [Option<String>; 4] = s.f.map(|f| f.map(crate::books::mslib::expand));
        msws::signals(&mut eng, [0, 1, 2, 3].map(|k| e[k].as_deref()), &map, side).ok()?
    };
    Some(msws::simulate(x, from, to, &sig, s.exec))
}

// ------------------------------------------------------------------ hand-written ValueWhen / PREV records
const NAN: f64 = f64::NAN;
fn near(a: f64, b: f64) -> bool { (a - b).abs() <= 1e-12 * a.abs().max(b.abs()) }
/// Engine comparison conventions: undefined -> undefined (NaN); equal within 1e-12 relative.
fn gt(a: f64, b: f64) -> f64 { if a.is_finite() && b.is_finite() { (a > b && !near(a, b)) as u8 as f64 } else { NAN } }
fn lt(a: f64, b: f64) -> f64 { gt(b, a) }
fn and(a: f64, b: f64) -> f64 { if a.is_finite() && b.is_finite() { (a != 0. && b != 0.) as u8 as f64 } else { NAN } }
fn back(v: &[f64], k: usize) -> Vec<f64> { (0..v.len()).map(|i| if i >= k { v[i - k] } else { NAN }).collect() }
/// ValueWhen(nth, cond, data) (MetaStock 8 manual p.283): data on the nth most recent bar where cond was true; undefined
/// before the nth occurrence or where cond is undefined.
fn value_when(nth: usize, cond: &[f64], data: &[f64]) -> Vec<f64> {
    let mut hits: Vec<usize> = vec![];
    (0..cond.len()).map(|i| {
        if cond[i].is_finite() && cond[i] != 0. { hits.push(i); }
        if !cond[i].is_finite() || hits.len() < nth { NAN } else { data[hits[hits.len() - nth]] }
    }).collect()
}
/// BarsSince(cond): bars since cond was last true; undefined before the first occurrence or where cond is undefined.
fn bars_since(cond: &[f64]) -> Vec<f64> {
    let mut last: Option<usize> = None;
    (0..cond.len()).map(|i| { if cond[i].is_finite() && cond[i] != 0. { last = Some(i); } match last { Some(j) if cond[i].is_finite() => (i - j) as f64, _ => NAN } }).collect()
}
fn cross(p: &[f64], q: &[f64]) -> Vec<f64> {
    (0..p.len()).map(|i| if i >= 1 && [p[i], q[i], p[i - 1], q[i - 1]].iter().all(|v| v.is_finite()) { (p[i] > q[i] && p[i - 1] <= q[i - 1]) as u8 as f64 } else { NAN }).collect()
}
fn truth(v: &[f64]) -> Vec<bool> { v.iter().map(|x| x.is_finite() && *x != 0.).collect() }
const HIST: &str = "MACD()-Mov(MACD(),9,E)";
fn hand(id: &str, eng: &mut Eng, x: &Ctx, side: i32) -> Result<msws::Sig, String> {
    let n = x.c.len();
    let (el, xl, es, xs): (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) = match id {
        "nkms2_macd_histogram_1" | "nkms2_macd_histogram_2" => {
            // a1 / a2 = histogram turning up / down; b1 / b2 = the last two turns diverge from the closes at those turns
            let h = eng.eval_src(HIST)?.to_vec();
            let a1 = eng.eval_src(&format!("{HIST}>Ref({HIST},-1) AND Ref({HIST},-1)<Ref({HIST},-2)"))?.to_vec();
            let a2 = eng.eval_src(&format!("{HIST}<Ref({HIST},-1) AND Ref({HIST},-1)>Ref({HIST},-2)"))?.to_vec();
            let h1 = back(&h, 1);
            let b = |a: &[f64], up: bool| -> Vec<f64> {
                let (v1, v2, c1, c2) = (value_when(1, a, &h1), value_when(2, a, &h1), value_when(1, a, &x.c), value_when(2, a, &x.c));
                (0..n).map(|i| if up { and(gt(v1[i], v2[i]), lt(c1[i], c2[i])) } else { and(lt(v1[i], v2[i]), gt(c1[i], c2[i])) }).collect()
            };
            let (bs1, bs2) = (bars_since(&b(&a1, true)), bars_since(&b(&a2, false)));
            let state: Vec<f64> = (0..n).map(|i| lt(bs1[i], bs2[i])).collect();
            let st1 = back(&state, 1);
            let up: Vec<f64> = (0..n).map(|i| gt(state[i], st1[i])).collect();
            let dn: Vec<f64> = (0..n).map(|i| lt(state[i], st1[i])).collect();
            if id.ends_with('1') { (up.clone(), dn.clone(), dn, up) } else {
                let hd: Vec<f64> = (0..n).map(|i| lt(h[i], h1[i])).collect();
                let hu: Vec<f64> = (0..n).map(|i| gt(h[i], h1[i])).collect();
                (up, hd, dn, hu)
            }
        }
        "nkms2_percentage_crossover_3pct" => {
            // var1 := If(C*.97 > PREV, C*.97, If(C*1.03 < PREV, C*1.03, PREV)); PREV = 0 on the first bar of the data
            let mut v = vec![NAN; n]; let mut prev = 0.0;
            for i in 0..n { let c = x.c[i]; prev = if gt(c * 0.97, prev) == 1. { c * 0.97 } else if lt(c * 1.03, prev) == 1. { c * 1.03 } else { prev }; v[i] = prev; }
            let (u, d) = (cross(&x.c, &v), cross(&v, &x.c));
            (u.clone(), d.clone(), d, u)
        }
        "nkms2_volatility_stop" => {
            // hstop := Min(Ref(C,-1)+ATR(4)*2, If(Ref(H,-1)>PREV, C*2, PREV)); lstop := Max(Ref(C,-1)-ATR(4)*2, If(Ref(L,-1)<PREV, 0, PREV))
            // PREV = 0 on the first bar where ATR(4) is defined; undefined before.
            let atr = eng.eval_src("ATR(4)")?.to_vec();
            let (mut hs, mut ls) = (vec![NAN; n], vec![NAN; n]);
            let (mut hp, mut lp) = (0.0, 0.0);
            for i in 1..n {
                if !atr[i].is_finite() { continue; }
                let (pc, ph, pl, c) = (x.c[i - 1], x.h[i - 1], x.l[i - 1], x.c[i]);
                hp = (pc + atr[i] * 2.).min(if gt(ph, hp) == 1. { c * 2. } else { hp });
                lp = (pc - atr[i] * 2.).max(if lt(pl, lp) == 1. { 0. } else { lp });
                hs[i] = hp; ls[i] = lp;
            }
            let bu = bars_since(&(0..n).map(|i| gt(x.c[i], hs[i])).collect::<Vec<_>>());
            let bd = bars_since(&(0..n).map(|i| lt(x.c[i], ls[i])).collect::<Vec<_>>());
            let l: Vec<f64> = (0..n).map(|i| gt(bu[i], bd[i])).collect();
            let s: Vec<f64> = (0..n).map(|i| lt(bu[i], bd[i])).collect();
            (l.clone(), s.clone(), s, l)
        }
        _ => return Err("not hand-coded".into()),
    };
    let off = vec![false; n];
    let (el, xl) = if side >= 0 { (truth(&el), truth(&xl)) } else { (off.clone(), off.clone()) };
    let (es, xs) = if side <= 0 { (truth(&es), truth(&xs)) } else { (off.clone(), off) };
    Ok(msws::Sig { el, xl, es, xs })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn rows() -> Vec<Row> {
        (0..400).map(|i| { let t = i as f64; let c = 100. + 10. * (t / 9.).sin() + 6. * (t / 23.).cos() + t * 0.02;
            Row { ticker: "T".into(), date: format!("D{i:04}"), open: c - 0.4 * (t / 5.).sin(), high: c + 1.2 + 0.3 * (t / 3.).cos(), low: c - 1.1 - 0.2 * (t / 4.).sin(), close: c, volume: Some(1000. + 300. * (t / 7.).sin() + if i % 6 == 0 { 2500. } else { 0. }) } }).collect()
    }
    #[test]
    fn every_live_record_parses_runs_and_trades() {
        let r = rows(); let x = Ctx::new(&r, &r, &[]);
        let live: Vec<&Sys> = SYS.iter().filter(|s| s.status == Status::Live).collect();
        assert_eq!(live.len(), 27);
        let mut trading = 0;
        for s in &live {
            let t = run(s.id, &x, 50, r.len() - 1, &Params::new()).unwrap_or_else(|| panic!("{} did not run", s.id));
            if t.len() >= 2 { trading += 1; }
            assert!(params(s.id).is_some(), "{}", s.id);
        }
        assert!(trading >= 24, "only {trading} records traded on the synthetic series");
        for id in REVIEWED { assert!(live.iter().any(|s| s.id == *id), "{id}"); }
        assert_eq!(SYS.len(), 38);
    }
    #[test]
    fn value_when_and_bars_since() {
        let c = [0., 1., 0., 1., 1., 0.]; let d = [10., 11., 12., 13., 14., 15.];
        assert!(value_when(1, &c, &d)[0].is_nan());
        assert_eq!(value_when(1, &c, &d)[2], 11.); assert_eq!(value_when(2, &c, &d)[4], 13.); assert!(value_when(2, &c, &d)[2].is_nan());
        assert_eq!(bars_since(&c)[5], 1.); assert!(bars_since(&c)[0].is_nan());
    }
}

const SYS: &[Sys] = &[
    Sys { id: "nkms2_impulse_meter", name: r##"PS Impulse Meter"##, group: GROUP, family: FAMILY, page: "MS72PRFT.DTA record index 54 (0-based)", status: NotTestable(r##"uses Wilders() smoothing and PREV (self-reference) on Mo()-based terms; the vault formula engine runs neither, and neither the record nor the MetaStock 8 manual (p.246 shows PREV only by example) gives PREV's start value or the Wilders() seed. Not coded."##), f: [Some(r##"var1:=If(Max(Mo(2),Max(Mo(3),Max(Mo(4),Max(Mo(5),Max(Mo(6),Max(Mo(7),Max(Mo(8),Max(Mo(9),Max(Mo(10),Max(Mo(11),Mo(12)))))))))))-Min(Mo(2),Min(Mo(3),Min(Mo(4),Min(Mo(5),Min(Mo(6),Min(Mo(7),Min(Mo(8),Min(Mo(9),Min(Mo(10),Min(Mo(11),Mo(12)))))))))))<>0,Stdev(C,10)/(Max(Mo(2),Max(Mo(3),Max(Mo(4),Max(Mo(5),Max(Mo(6),Max(Mo(7),Max(Mo(8),Max(Mo(9),Max(Mo(10),Max(Mo(11),Mo(12)))))))))))-Min(Mo(2),Min(Mo(3),Min(Mo(4),Min(Mo(5),Min(Mo(6),Min(Mo(7),Min(Mo(8),Min(Mo(9),Min(Mo(10),Min(Mo(11),Mo(12)))))))))))),PREV); Z1:=Wilders(If(ROC(var1,1,$)>0,ROC(var1,1,$),0),7);  Y1:=Wilders(If(ROC(var1,1,$)<0,Abs(ROC(var1,1,$)),0),7);  RS:=Z1/Y1;  b1:=100-(100/(1+RS));  Cross(b1,30)"##), Some(r##"var1:=If(Max(Mo(2),Max(Mo(3),Max(Mo(4),Max(Mo(5),Max(Mo(6),Max(Mo(7),Max(Mo(8),Max(Mo(9),Max(Mo(10),Max(Mo(11),Mo(12)))))))))))-Min(Mo(2),Min(Mo(3),Min(Mo(4),Min(Mo(5),Min(Mo(6),Min(Mo(7),Min(Mo(8),Min(Mo(9),Min(Mo(10),Min(Mo(11),Mo(12)))))))))))<>0,Stdev(C,10)/(Max(Mo(2),Max(Mo(3),Max(Mo(4),Max(Mo(5),Max(Mo(6),Max(Mo(7),Max(Mo(8),Max(Mo(9),Max(Mo(10),Max(Mo(11),Mo(12)))))))))))-Min(Mo(2),Min(Mo(3),Min(Mo(4),Min(Mo(5),Min(Mo(6),Min(Mo(7),Min(Mo(8),Min(Mo(9),Min(Mo(10),Min(Mo(11),Mo(12)))))))))))),PREV); Z1:=Wilders(If(ROC(var1,1,$)>0,ROC(var1,1,$),0),7);  Y1:=Wilders(If(ROC(var1,1,$)<0,Abs(ROC(var1,1,$)),0),7);  RS:=Z1/Y1;  b1:=100-(100/(1+RS));  Cross(70,b1)"##), Some(r##"var1:=If(Max(Mo(2),Max(Mo(3),Max(Mo(4),Max(Mo(5),Max(Mo(6),Max(Mo(7),Max(Mo(8),Max(Mo(9),Max(Mo(10),Max(Mo(11),Mo(12)))))))))))-Min(Mo(2),Min(Mo(3),Min(Mo(4),Min(Mo(5),Min(Mo(6),Min(Mo(7),Min(Mo(8),Min(Mo(9),Min(Mo(10),Min(Mo(11),Mo(12)))))))))))<>0,Stdev(C,10)/(Max(Mo(2),Max(Mo(3),Max(Mo(4),Max(Mo(5),Max(Mo(6),Max(Mo(7),Max(Mo(8),Max(Mo(9),Max(Mo(10),Max(Mo(11),Mo(12)))))))))))-Min(Mo(2),Min(Mo(3),Min(Mo(4),Min(Mo(5),Min(Mo(6),Min(Mo(7),Min(Mo(8),Min(Mo(9),Min(Mo(10),Min(Mo(11),Mo(12)))))))))))),PREV); Z1:=Wilders(If(ROC(var1,1,$)>0,ROC(var1,1,$),0),7);  Y1:=Wilders(If(ROC(var1,1,$)<0,Abs(ROC(var1,1,$)),0),7);  RS:=Z1/Y1;  b1:=100-(100/(1+RS));  Cross(70,b1)"##), Some(r##"var1:=If(Max(Mo(2),Max(Mo(3),Max(Mo(4),Max(Mo(5),Max(Mo(6),Max(Mo(7),Max(Mo(8),Max(Mo(9),Max(Mo(10),Max(Mo(11),Mo(12)))))))))))-Min(Mo(2),Min(Mo(3),Min(Mo(4),Min(Mo(5),Min(Mo(6),Min(Mo(7),Min(Mo(8),Min(Mo(9),Min(Mo(10),Min(Mo(11),Mo(12)))))))))))<>0,Stdev(C,10)/(Max(Mo(2),Max(Mo(3),Max(Mo(4),Max(Mo(5),Max(Mo(6),Max(Mo(7),Max(Mo(8),Max(Mo(9),Max(Mo(10),Max(Mo(11),Mo(12)))))))))))-Min(Mo(2),Min(Mo(3),Min(Mo(4),Min(Mo(5),Min(Mo(6),Min(Mo(7),Min(Mo(8),Min(Mo(9),Min(Mo(10),Min(Mo(11),Mo(12)))))))))))),PREV); Z1:=Wilders(If(ROC(var1,1,$)>0,ROC(var1,1,$),0),7);  Y1:=Wilders(If(ROC(var1,1,$)<0,Abs(ROC(var1,1,$)),0),7);  RS:=Z1/Y1;  b1:=100-(100/(1+RS));  Cross(b1,30)"##)], exec: EXEC_DEFAULT, notes: r##"Mo(n) = 100 x C / Ref(C,-n) (MetaStock 8 manual p.482; run as 100 + ROC(C,n,%) through books::mslib::expand, as in mslib)."## },
    Sys { id: "nkms2_intraday_momentum_index", name: r##"PS Intraday Momentum Index"##, group: GROUP, family: FAMILY, page: "MS72PRFT.DTA record index 55 (0-based)", status: NotTestable(r##"uses IMI(14), MetaStock's built-in Intraday Momentum Index; the record does not define it and the MetaStock 8 manual describes it only in words (an RSI-like count on candle bodies, Interpretation section), not its exact sums or smoothing. Not coded."##), f: [Some(r##"Cross(IMI(14),30)"##), Some(r##"Cross(70,IMI(14))"##), Some(r##"Cross(70,IMI(14))"##), Some(r##"Cross(IMI(14),30)"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "nkms2_landis_reversal", name: r##"PS Landis Reversal"##, group: GROUP, family: FAMILY, page: "MS72PRFT.DTA record index 56 (0-based)", status: Live, f: [Some(r##"var1:=((Stoch(8,3)*.05)+(Stoch(89,21)*.43)+(Stoch(55,13)*.26)+(Stoch(34,8)*.16)+(Stoch(21,5)*.1));  var1>Ref(var1,-1)"##), Some(r##"var1:=((Stoch(8,3)*.05)+(Stoch(89,21)*.43)+(Stoch(55,13)*.26)+(Stoch(34,8)*.16)+(Stoch(21,5)*.1));  var1<Ref(var1,-1)"##), Some(r##"var1:=((Stoch(8,3)*.05)+(Stoch(89,21)*.43)+(Stoch(55,13)*.26)+(Stoch(34,8)*.16)+(Stoch(21,5)*.1));  var1<Ref(var1,-1)"##), Some(r##"var1:=((Stoch(8,3)*.05)+(Stoch(89,21)*.43)+(Stoch(55,13)*.26)+(Stoch(34,8)*.16)+(Stoch(21,5)*.1));  var1>Ref(var1,-1)"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "nkms2_linear_regression_crossover", name: r##"PS Linear Regression Crossover"##, group: GROUP, family: FAMILY, page: "MS72PRFT.DTA record index 57 (0-based)", status: Live, f: [Some(r##"LinearReg(C,21)>Mov(C,21,S)"##), Some(r##"LinearReg(C,21)<Mov(C,21,S)"##), Some(r##"LinearReg(C,21)<Mov(C,21,S)"##), Some(r##"LinearReg(C,21)>Mov(C,21,S)"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "nkms2_long_sell_short_sale_5_day", name: r##"PS Long Sell Short Sale-5 Day"##, group: GROUP, family: FAMILY, page: "MS72PRFT.DTA record index 58 (0-based)", status: Live, f: [Some(r##"(((HHV(H,5)-Ref(O,-4))+(C-LLV(L,5)))/(2*(HHV(H,5)-LLV(L,5))))<.3"##), Some(r##"(((HHV(H,5)-Ref(O,-4))+(C-LLV(L,5)))/(2*(HHV(H,5)-LLV(L,5))))>.7"##), Some(r##"(((HHV(H,5)-Ref(O,-4))+(C-LLV(L,5)))/(2*(HHV(H,5)-LLV(L,5))))>.7"##), Some(r##"(((HHV(H,5)-Ref(O,-4))+(C-LLV(L,5)))/(2*(HHV(H,5)-LLV(L,5))))<.3"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "nkms2_macd_histogram_1", name: r##"PS MACD Histogram 1"##, group: GROUP, family: FAMILY, page: "MS72PRFT.DTA record index 59 (0-based)", status: Live, f: [Some(r##"a1:=MACD()-Mov(MACD(),9,E)>Ref(MACD()-Mov(MACD(),9,E),-1) AND Ref(MACD()-Mov(MACD(),9,E),-1)<Ref(MACD()-Mov(MACD(),9,E),-2); a2:=MACD()-Mov(MACD(),9,E)<Ref(MACD()-Mov(MACD(),9,E),-1) AND Ref(MACD()-Mov(MACD(),9,E),-1)>Ref(MACD()-Mov(MACD(),9,E),-2); b1:=ValueWhen(1,a1,Ref(MACD()-Mov(MACD(),9,E),-1))>ValueWhen(2,a1,Ref(MACD()-Mov(MACD(),9,E),-1)) AND ValueWhen(1,a1,C)<ValueWhen(2,a1,C); b2:=ValueWhen(1,a2,Ref(MACD()-Mov(MACD(),9,E),-1))<ValueWhen(2,a2,Ref(MACD()-Mov(MACD(),9,E),-1)) AND ValueWhen(1,a2,C)>ValueWhen(2,a2,C); state1:=If(BarsSince(b1)<BarsSince(b2),1,0);  state1>Ref(state1,-1)"##), Some(r##"a1:=MACD()-Mov(MACD(),9,E)>Ref(MACD()-Mov(MACD(),9,E),-1) AND Ref(MACD()-Mov(MACD(),9,E),-1)<Ref(MACD()-Mov(MACD(),9,E),-2); a2:=MACD()-Mov(MACD(),9,E)<Ref(MACD()-Mov(MACD(),9,E),-1) AND Ref(MACD()-Mov(MACD(),9,E),-1)>Ref(MACD()-Mov(MACD(),9,E),-2); b1:=ValueWhen(1,a1,Ref(MACD()-Mov(MACD(),9,E),-1))>ValueWhen(2,a1,Ref(MACD()-Mov(MACD(),9,E),-1)) AND ValueWhen(1,a1,C)<ValueWhen(2,a1,C); b2:=ValueWhen(1,a2,Ref(MACD()-Mov(MACD(),9,E),-1))<ValueWhen(2,a2,Ref(MACD()-Mov(MACD(),9,E),-1)) AND ValueWhen(1,a2,C)>ValueWhen(2,a2,C); state1:=If(BarsSince(b1)<BarsSince(b2),1,0);  state1<Ref(state1,-1)"##), Some(r##"a1:=MACD()-Mov(MACD(),9,E)>Ref(MACD()-Mov(MACD(),9,E),-1) AND Ref(MACD()-Mov(MACD(),9,E),-1)<Ref(MACD()-Mov(MACD(),9,E),-2); a2:=MACD()-Mov(MACD(),9,E)<Ref(MACD()-Mov(MACD(),9,E),-1) AND Ref(MACD()-Mov(MACD(),9,E),-1)>Ref(MACD()-Mov(MACD(),9,E),-2); b1:=ValueWhen(1,a1,Ref(MACD()-Mov(MACD(),9,E),-1))>ValueWhen(2,a1,Ref(MACD()-Mov(MACD(),9,E),-1)) AND ValueWhen(1,a1,C)<ValueWhen(2,a1,C); b2:=ValueWhen(1,a2,Ref(MACD()-Mov(MACD(),9,E),-1))<ValueWhen(2,a2,Ref(MACD()-Mov(MACD(),9,E),-1)) AND ValueWhen(1,a2,C)>ValueWhen(2,a2,C); state1:=If(BarsSince(b1)<BarsSince(b2),1,0);  state1<Ref(state1,-1)"##), Some(r##"a1:=MACD()-Mov(MACD(),9,E)>Ref(MACD()-Mov(MACD(),9,E),-1) AND Ref(MACD()-Mov(MACD(),9,E),-1)<Ref(MACD()-Mov(MACD(),9,E),-2); a2:=MACD()-Mov(MACD(),9,E)<Ref(MACD()-Mov(MACD(),9,E),-1) AND Ref(MACD()-Mov(MACD(),9,E),-1)>Ref(MACD()-Mov(MACD(),9,E),-2); b1:=ValueWhen(1,a1,Ref(MACD()-Mov(MACD(),9,E),-1))>ValueWhen(2,a1,Ref(MACD()-Mov(MACD(),9,E),-1)) AND ValueWhen(1,a1,C)<ValueWhen(2,a1,C); b2:=ValueWhen(1,a2,Ref(MACD()-Mov(MACD(),9,E),-1))<ValueWhen(2,a2,Ref(MACD()-Mov(MACD(),9,E),-1)) AND ValueWhen(1,a2,C)>ValueWhen(2,a2,C); state1:=If(BarsSince(b1)<BarsSince(b2),1,0);  state1>Ref(state1,-1)"##)], exec: EXEC_DEFAULT, notes: r##"Run by hand-written code: ValueWhen(n, cond, data) = data on the nth most recent bar where cond was true (MetaStock 8 manual, Formula Language, Value When); BarsSince undefined before the first occurrence; every other part is the stored text evaluated by the engine."## },
    Sys { id: "nkms2_macd_histogram_2", name: r##"PS MACD Histogram 2"##, group: GROUP, family: FAMILY, page: "MS72PRFT.DTA record index 60 (0-based)", status: Live, f: [Some(r##"a1:=MACD()-Mov(MACD(),9,E)>Ref(MACD()-Mov(MACD(),9,E),-1) AND Ref(MACD()-Mov(MACD(),9,E),-1)<Ref(MACD()-Mov(MACD(),9,E),-2); a2:=MACD()-Mov(MACD(),9,E)<Ref(MACD()-Mov(MACD(),9,E),-1) AND Ref(MACD()-Mov(MACD(),9,E),-1)>Ref(MACD()-Mov(MACD(),9,E),-2); b1:=ValueWhen(1,a1,Ref(MACD()-Mov(MACD(),9,E),-1))>ValueWhen(2,a1,Ref(MACD()-Mov(MACD(),9,E),-1)) AND ValueWhen(1,a1,C)<ValueWhen(2,a1,C); b2:=ValueWhen(1,a2,Ref(MACD()-Mov(MACD(),9,E),-1))<ValueWhen(2,a2,Ref(MACD()-Mov(MACD(),9,E),-1)) AND ValueWhen(1,a2,C)>ValueWhen(2,a2,C); state1:=If(BarsSince(b1)<BarsSince(b2),1,0);  state1>Ref(state1,-1)"##), Some(r##"(MACD()-Mov(MACD(),9,E))<Ref((MACD()-Mov(MACD(),9,E)),-1)"##), Some(r##"a1:=MACD()-Mov(MACD(),9,E)>Ref(MACD()-Mov(MACD(),9,E),-1) AND Ref(MACD()-Mov(MACD(),9,E),-1)<Ref(MACD()-Mov(MACD(),9,E),-2); a2:=MACD()-Mov(MACD(),9,E)<Ref(MACD()-Mov(MACD(),9,E),-1) AND Ref(MACD()-Mov(MACD(),9,E),-1)>Ref(MACD()-Mov(MACD(),9,E),-2); b1:=ValueWhen(1,a1,Ref(MACD()-Mov(MACD(),9,E),-1))>ValueWhen(2,a1,Ref(MACD()-Mov(MACD(),9,E),-1)) AND ValueWhen(1,a1,C)<ValueWhen(2,a1,C); b2:=ValueWhen(1,a2,Ref(MACD()-Mov(MACD(),9,E),-1))<ValueWhen(2,a2,Ref(MACD()-Mov(MACD(),9,E),-1)) AND ValueWhen(1,a2,C)>ValueWhen(2,a2,C); state1:=If(BarsSince(b1)<BarsSince(b2),1,0);  state1<Ref(state1,-1)"##), Some(r##"(MACD()-Mov(MACD(),9,E))>Ref((MACD()-Mov(MACD(),9,E)),-1)"##)], exec: EXEC_DEFAULT, notes: r##"Run by hand-written code: ValueWhen(n, cond, data) = data on the nth most recent bar where cond was true (MetaStock 8 manual, Formula Language, Value When); BarsSince undefined before the first occurrence; every other part is the stored text evaluated by the engine."## },
    Sys { id: "nkms2_meisels_overbought_oversold", name: r##"PS Meisels Overbought/Oversold"##, group: GROUP, family: FAMILY, page: "MS72PRFT.DTA record index 61 (0-based)", status: Live, f: [Some(r##"M1:=Sum(If(C>Ref(C,-1),+1,If(C<Ref(C,-1),-1,0)),10);  Cross(-6,M1) OR M1=-6"##), Some(r##"M1:=Sum(If(C>Ref(C,-1),+1,If(C<Ref(C,-1),-1,0)),10);  Cross(M1,0) OR M1=0"##), Some(r##"M1:=Sum(If(C>Ref(C,-1),+1,If(C<Ref(C,-1),-1,0)),10);  Cross(M1,6) OR M1=6"##), Some(r##"M1:=Sum(If(C>Ref(C,-1),+1,If(C<Ref(C,-1),-1,0)),10);  Cross(0,M1) OR M1=0"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "nkms2_modified_momentum", name: r##"PS Modified Momentum"##, group: GROUP, family: FAMILY, page: "MS72PRFT.DTA record index 63 (0-based)", status: Live, f: [Some(r##"var1:=(Mov(ROC(Mov(C,3,S),25,%),15,E));  var1>Mov(var1,9,E)"##), Some(r##"var1:=(Mov(ROC(Mov(C,3,S),25,%),15,E));  var1<Mov(var1,9,E)"##), Some(r##"var1:=(Mov(ROC(Mov(C,3,S),25,%),15,E));  var1<Mov(var1,9,E)"##), Some(r##"var1:=(Mov(ROC(Mov(C,3,S),25,%),15,E));  var1>Mov(var1,9,E)"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "nkms2_momentum", name: r##"PS Momentum"##, group: GROUP, family: FAMILY, page: "MS72PRFT.DTA record index 64 (0-based)", status: Live, f: [Some(r##"LinearReg(Mo(12),8)>100"##), Some(r##"LinearReg(Mo(12),8)<100"##), Some(r##"LinearReg(Mo(12),8)<100"##), Some(r##"LinearReg(Mo(12),8)>100"##)], exec: EXEC_DEFAULT, notes: r##"Mo(n) = 100 x C / Ref(C,-n) (MetaStock 8 manual p.482; run as 100 + ROC(C,n,%) through books::mslib::expand, as in mslib)."## },
    Sys { id: "nkms2_new_relative_volatility_index", name: r##"PS New Relative Volatility Index"##, group: GROUP, family: FAMILY, page: "MS72PRFT.DTA record index 65 (0-based)", status: Live, f: [Some(r##"var1:=(100*Mov(If(H>Ref(H,-1),Std(H,10),0),14,E)/(Mov(If(H>Ref(H,-1),Std(H,10),0),14,E)+Mov(If(H<Ref(H,-1),Std(H,10),0),14,E))+100*Mov(If(L>Ref(L,-1),Std(L,10),0),14,E)/(Mov(If(L>Ref(L,-1),Std(L,10),0),14,E)+Mov(If(L<Ref(L,-1),Std(L,10),0),14,E)))/2;  var1>Ref(var1,-1)"##), Some(r##"var1:=(100*Mov(If(H>Ref(H,-1),Std(H,10),0),14,E)/(Mov(If(H>Ref(H,-1),Std(H,10),0),14,E)+Mov(If(H<Ref(H,-1),Std(H,10),0),14,E))+100*Mov(If(L>Ref(L,-1),Std(L,10),0),14,E)/(Mov(If(L>Ref(L,-1),Std(L,10),0),14,E)+Mov(If(L<Ref(L,-1),Std(L,10),0),14,E)))/2;  var1<Ref(var1,-1)"##), Some(r##"var1:=(100*Mov(If(H>Ref(H,-1),Std(H,10),0),14,E)/(Mov(If(H>Ref(H,-1),Std(H,10),0),14,E)+Mov(If(H<Ref(H,-1),Std(H,10),0),14,E))+100*Mov(If(L>Ref(L,-1),Std(L,10),0),14,E)/(Mov(If(L>Ref(L,-1),Std(L,10),0),14,E)+Mov(If(L<Ref(L,-1),Std(L,10),0),14,E)))/2;  var1<Ref(var1,-1)"##), Some(r##"var1:=(100*Mov(If(H>Ref(H,-1),Std(H,10),0),14,E)/(Mov(If(H>Ref(H,-1),Std(H,10),0),14,E)+Mov(If(H<Ref(H,-1),Std(H,10),0),14,E))+100*Mov(If(L>Ref(L,-1),Std(L,10),0),14,E)/(Mov(If(L>Ref(L,-1),Std(L,10),0),14,E)+Mov(If(L<Ref(L,-1),Std(L,10),0),14,E)))/2;  var1>Ref(var1,-1)"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "nkms2_oscillator_composite", name: r##"PS Oscillator Composite"##, group: GROUP, family: FAMILY, page: "MS72PRFT.DTA record index 66 (0-based)", status: Live, f: [Some(r##"Var1:=RSI(34)+Stoch(34,3)+Mo(34)+CCI(34);  Var1<-50"##), Some(r##"Var1:=RSI(34)+Stoch(34,3)+Mo(34)+CCI(34);  Var1>450"##), Some(r##"Var1:=RSI(34)+Stoch(34,3)+Mo(34)+CCI(34);  Var1>450"##), Some(r##"Var1:=RSI(34)+Stoch(34,3)+Mo(34)+CCI(34);  Var1<-50"##)], exec: EXEC_DEFAULT, notes: r##"Mo(n) = 100 x C / Ref(C,-n) (MetaStock 8 manual p.482; run as 100 + ROC(C,n,%) through books::mslib::expand, as in mslib)."## },
    Sys { id: "nkms2_pattern_trading_system_1", name: r##"PS Pattern Trading System 1"##, group: GROUP, family: FAMILY, page: "MS72PRFT.DTA record index 67 (0-based)", status: Live, f: [Some(r##"C>Ref(C,-1) AND Ref(C,-1)<Ref(C,-2)"##), Some(r##"C<Ref(C,-1) AND Ref(C,-1)>Ref(C,-2) AND Ref(C,-2)>Ref(C,-3)"##), None, None], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "nkms2_pattern_trading_system_2", name: r##"PS Pattern Trading System 2"##, group: GROUP, family: FAMILY, page: "MS72PRFT.DTA record index 68 (0-based)", status: Live, f: [Some(r##"L>Ref(L,-1) AND C>Ref(H,-1) AND O>Ref(C,-1) AND C>((H+L)/2) AND C>O"##), Some(r##"H<Ref(H,-1) AND C<Ref(L,-1) AND O<Ref(C,-1) AND C<((H+L)/2) AND C<O"##), Some(r##"H<Ref(H,-1) AND C<Ref(L,-1) AND O<Ref(C,-1) AND C<((H+L)/2) AND C<O"##), Some(r##"L>Ref(L,-1) AND C>Ref(H,-1) AND O>Ref(C,-1) AND C>((H+L)/2) AND C>O"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "nkms2_pattern_trading_system_3", name: r##"PS Pattern Trading System 3"##, group: GROUP, family: FAMILY, page: "MS72PRFT.DTA record index 69 (0-based)", status: Live, f: [Some(r##"C>O AND O>Ref(C,-1) AND C>=((H+L)/2)"##), Some(r##"C<O AND O<Ref(C,-1) AND C<=((H+L)/2)"##), Some(r##"C<O AND O<Ref(C,-1) AND C<=((H+L)/2)"##), Some(r##"C>O AND O>Ref(C,-1) AND C>=((H+L)/2)"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "nkms2_pattern_trading_system_4", name: r##"PS Pattern Trading System 4"##, group: GROUP, family: FAMILY, page: "MS72PRFT.DTA record index 70 (0-based)", status: Live, f: [Some(r##"(C>Ref(H,-2) AND (V>1.75*Mov(V,20,S)))"##), Some(r##"(C<Ref(L,-2) AND (V>1.75*Mov(V,20,S)))"##), Some(r##"(C<Ref(L,-2) AND (V>1.75*Mov(V,20,S)))"##), Some(r##"(C>Ref(H,-2) AND (V>1.75*Mov(V,20,S)))"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "nkms2_pattern_trading_system_5", name: r##"PS Pattern Trading System 5"##, group: GROUP, family: FAMILY, page: "MS72PRFT.DTA record index 71 (0-based)", status: Live, f: [Some(r##"var1:=(H>Ref(H,-1) AND L>Ref(L,-1) AND C>Ref(C,-1)) AND (Ref(H,-1)<Ref(H,-2) AND Ref(L,-1)<Ref(L,-2) AND Ref(C,-1)<Ref(C,-2)) AND (Ref(H,-2)<Ref(H,-3) AND Ref(L,-2)<Ref(L,-3) AND Ref(C,-2)<Ref(C,-3)) AND Ref(L,-1)>Mov(C,20,E);  var1 AND ADX(14)>15"##), Some(r##"var1:=(H>Ref(H,-1) AND L>Ref(L,-1) AND C>Ref(C,-1)) AND (Ref(H,-1)<Ref(H,-2) AND Ref(L,-1)<Ref(L,-2) AND Ref(C,-1)<Ref(C,-2)) AND (Ref(H,-2)<Ref(H,-3) AND Ref(L,-2)<Ref(L,-3) AND Ref(C,-2)<Ref(C,-3)) AND Ref(L,-1)>Mov(C,20,E);  Ref(var1,-2) OR L<Ref(L,-1)"##), Some(r##"var2:=(H<Ref(H,-1) AND L<Ref(L,-1) AND C<Ref(C,-1)) AND (Ref(H,-1)>Ref(H,-2) AND Ref(L,-1)>Ref(L,-2) AND Ref(C,-1)>Ref(C,-2)) AND (Ref(H,-2)>Ref(H,-3) AND Ref(L,-2)>Ref(L,-3) AND Ref(C,-2)>Ref(C,-3)) AND Ref(L,-1)<Mov(C,20,E);  var2 AND ADX(14)>15"##), Some(r##"var2:=(H<Ref(H,-1) AND L<Ref(L,-1) AND C<Ref(C,-1)) AND (Ref(H,-1)>Ref(H,-2) AND Ref(L,-1)>Ref(L,-2) AND Ref(C,-1)>Ref(C,-2)) AND (Ref(H,-2)>Ref(H,-3) AND Ref(L,-2)>Ref(L,-3) AND Ref(C,-2)>Ref(C,-3)) AND Ref(L,-1)<Mov(C,20,E);  Ref(var2,-2) OR H>Ref(H,-1)"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "nkms2_pattern_trading_system_6", name: r##"PS Pattern Trading System 6"##, group: GROUP, family: FAMILY, page: "MS72PRFT.DTA record index 72 (0-based)", status: Live, f: [Some(r##"H>Ref(H,-1) AND L>Ref(L,-1)"##), Some(r##"H<Ref(H,-1) AND L<Ref(L,-1)"##), Some(r##"H<Ref(H,-1) AND L<Ref(L,-1)"##), Some(r##"H>Ref(H,-1) AND L>Ref(L,-1)"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "nkms2_percentage_crossover_3pct", name: r##"PS Percentage Crossover 3%"##, group: GROUP, family: FAMILY, page: "MS72PRFT.DTA record index 73 (0-based)", status: Live, f: [Some(r##"var1:=If((C*.97)>PREV,C*.97,If((C*1.03)<PREV,C*1.03,PREV));  Cross(C,var1)"##), Some(r##"var1:=If((C*.97)>PREV,C*.97,If((C*1.03)<PREV,C*1.03,PREV));  Cross(var1,C)"##), Some(r##"var1:=If((C*.97)>PREV,C*.97,If((C*1.03)<PREV,C*1.03,PREV));  Cross(var1,C)"##), Some(r##"var1:=If((C*.97)>PREV,C*.97,If((C*1.03)<PREV,C*1.03,PREV));  Cross(C,var1)"##)], exec: EXEC_DEFAULT, notes: r##"Run by hand-written code: PREV = the formula's value on the previous bar (MetaStock 8 manual p.246); [reading] PREV = 0 on the first bar of the data (the manual gives no start value), so var1 starts at 0.97 x the first close."## },
    Sys { id: "nkms2_polarized_fractal_efficiency", name: r##"PS Polarized Fractal Efficiency"##, group: GROUP, family: FAMILY, page: "MS72PRFT.DTA record index 75 (0-based)", status: NotTestable(r##"uses PFE(C,34,3), the built-in Polarized Fractal Efficiency; the MetaStock 8 manual only says it 'calculates the predefined Polarized Fractal Efficiency' (Formula Language) and the record does not define it. Not coded."##), f: [Some(r##"var1:=(PFE(C,34,3)-Lowest(PFE(C,34,3)))/(Highest(PFE(C,34,3))-Lowest(PFE(C,34,3)));  Cross(var1,.5)"##), Some(r##"var1:=(PFE(C,34,3)-Lowest(PFE(C,34,3)))/(Highest(PFE(C,34,3))-Lowest(PFE(C,34,3)));  Cross(.5,var1) OR Cross(var1,.8)"##), Some(r##"var1:=(PFE(C,34,3)-Lowest(PFE(C,34,3)))/(Highest(PFE(C,34,3))-Lowest(PFE(C,34,3)));  Cross(.5,var1)"##), Some(r##"var1:=(PFE(C,34,3)-Lowest(PFE(C,34,3)))/(Highest(PFE(C,34,3))-Lowest(PFE(C,34,3)));  Cross(var1,.5) OR Cross(.2,var1)"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "nkms2_projection_oscillator_1", name: r##"PS Projection Oscillator 1"##, group: GROUP, family: FAMILY, page: "MS72PRFT.DTA record index 76 (0-based)", status: NotTestable(r##"uses ProjOsc(8,3), the built-in Projection Oscillator; its calculation is printed neither in the record nor in the MetaStock 8 manual ('predefined'). Not coded."##), f: [Some(r##"Cross(ProjOsc(8,3),Mov(ProjOsc(8,3),3,S))"##), Some(r##"Cross(Mov(ProjOsc(8,3),3,S),ProjOsc(8,3))"##), Some(r##"Cross(Mov(ProjOsc(8,3),3,S),ProjOsc(8,3))"##), Some(r##"Cross(ProjOsc(8,3),Mov(ProjOsc(8,3),3,S))"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "nkms2_projection_oscillator_2", name: r##"PS Projection Oscillator 2"##, group: GROUP, family: FAMILY, page: "MS72PRFT.DTA record index 77 (0-based)", status: NotTestable(r##"uses ProjOsc(14,3), the built-in Projection Oscillator; its calculation is printed neither in the record nor in the MetaStock 8 manual ('predefined'). Not coded."##), f: [Some(r##"Cross(ProjOsc(14,3),30)"##), Some(r##"Cross(70,ProjOsc(14,3))"##), Some(r##"Cross(70,ProjOsc(14,3))"##), Some(r##"Cross(ProjOsc(14,3),30)"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "nkms2_relative_momentum_index", name: r##"PS Relative Momentum Index"##, group: GROUP, family: FAMILY, page: "MS72PRFT.DTA record index 78 (0-based)", status: NotTestable(r##"uses RMI(C,20,5), the built-in Relative Momentum Index; the MetaStock 8 manual describes it only in words (an RSI on the close against the close x days ago), not its smoothing. Not coded."##), f: [Some(r##"Cross(RMI(C,20,5),30)"##), Some(r##"Cross(70,RMI(C,20,5))"##), Some(r##"Cross(70,RMI(C,20,5))"##), Some(r##"Cross(RMI(C,20,5),30)"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "nkms2_relative_volatility_index_1", name: r##"PS Relative Volatility Index 1"##, group: GROUP, family: FAMILY, page: "MS72PRFT.DTA record index 79 (0-based)", status: NotTestable(r##"uses RVI(14), the built-in Relative Volatility Index; the MetaStock 8 manual says only 'identical to the RSI except ... standard deviation', without the standard-deviation length or which of Dorsey's two versions (1993 / 1995) is built in. Not coded."##), f: [Some(r##"Cross(RVI(14),30)"##), Some(r##"Cross(70,RVI(14))"##), Some(r##"Cross(70,RVI(14))"##), Some(r##"Cross(RVI(14),30)"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "nkms2_relative_volatility_index_2", name: r##"PS Relative Volatility Index 2"##, group: GROUP, family: FAMILY, page: "MS72PRFT.DTA record index 80 (0-based)", status: NotTestable(r##"uses RVI(14), the built-in Relative Volatility Index; the MetaStock 8 manual says only 'identical to the RSI except ... standard deviation', without the standard-deviation length or which of Dorsey's two versions (1993 / 1995) is built in. Not coded."##), f: [Some(r##"Cross(RVI(14),40)"##), Some(r##"Cross(60,RVI(14))"##), Some(r##"Cross(60,RVI(14))"##), Some(r##"Cross(RVI(14),40)"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "nkms2_rsi_trailing_exit", name: r##"PS RSI Trailing Exit"##, group: GROUP, family: FAMILY, page: "MS72PRFT.DTA record index 81 (0-based)", status: Live, f: [Some(r##"RSI(14)>Mov(RSI(14),3,S) AND Cross(RSI(14),30)"##), Some(r##"Cross(30,RSI(14)) OR (RSI(14)<Mov(RSI(14),3,S) AND RSI(14)>70)"##), Some(r##"RSI(14)<Mov(RSI(14),3,S) AND Cross(70,RSI(14))"##), Some(r##"Cross(RSI(14),70) OR (RSI(14)>Mov(RSI(14),3,S) AND RSI(14)<30)"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "nkms2_short_term_range_rejection", name: r##"PS Short Term Range Rejection"##, group: GROUP, family: FAMILY, page: "MS72PRFT.DTA record index 82 (0-based)", status: Live, f: [Some(r##"var1:=HHV(C,21)<1.1*Mov(C,21,S) AND LLV(C,21)>0.9*Mov(C,21,S);  var1=0 AND LinRegSlope(C,5)>0 AND LinRegSlope(C,21)>0"##), Some(r##"var1:=HHV(C,21)<1.1*Mov(C,21,S) AND LLV(C,21)>0.9*Mov(C,21,S);  var1=1 OR LinRegSlope(C,21)<0"##), Some(r##"var1:=HHV(C,21)<1.1*Mov(C,21,S) AND LLV(C,21)>0.9*Mov(C,21,S);  var1=0 AND LinRegSlope(C,5)<0 AND LinRegSlope(C,21)<0"##), Some(r##"var1:=HHV(C,21)<1.1*Mov(C,21,S) AND LLV(C,21)>0.9*Mov(C,21,S);  var1=1 OR LinRegSlope(C,21)>0"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "nkms2_small_triangle_breakout", name: r##"PS Small Triangle Breakout"##, group: GROUP, family: FAMILY, page: "MS72PRFT.DTA record index 83 (0-based)", status: Live, f: [Some(r##"var1:=(H<=Ref(H,-1) AND Ref(H,-1)<=Ref(H,-2) AND L>=Ref(L,-1) AND Ref(L,-1)>=Ref(L,-2));  Ref(var1,-1) AND H>Ref(H,-1)"##), Some(r##"L<Ref(L,-1)"##), Some(r##"var1:=(H<=Ref(H,-1) AND Ref(H,-1)<=Ref(H,-2) AND L>=Ref(L,-1) AND Ref(L,-1)>=Ref(L,-2));  Ref(var1,-1) AND L<Ref(L,-1)"##), Some(r##"H>Ref(H,-1)"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "nkms2_starcband_system", name: r##"PS StarcBand System"##, group: GROUP, family: FAMILY, page: "MS72PRFT.DTA record index 84 (0-based)", status: Live, f: [Some(r##"Cross(Mov(C,6,S)-(ATR(15)*2),Mov(C,20,E))"##), Some(r##"Cross(Mov(C,20,E),Mov(C,6,S)+(ATR(15)*2))"##), Some(r##"Cross(Mov(C,20,E),Mov(C,6,S)+(ATR(15)*2))"##), Some(r##"Cross(Mov(C,6,S)-(ATR(15)*2),Mov(C,20,E))"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "nkms2_stochastic_oscillator_system", name: r##"PS Stochastic Oscillator System"##, group: GROUP, family: FAMILY, page: "MS72PRFT.DTA record index 86 (0-based)", status: Live, f: [Some(r##"Cross(Stoch(14,1),20)"##), Some(r##"Cross(80,Stoch(14,1)) OR Stoch(14,1)<20"##), Some(r##"Cross(80,Stoch(14,1))"##), Some(r##"Cross(Stoch(14,1),20) OR Stoch(14,1)>80"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "nkms2_stochrsi", name: r##"PS StochRSI"##, group: GROUP, family: FAMILY, page: "MS72PRFT.DTA record index 87 (0-based)", status: Live, f: [Some(r##"Cross(20,(Sum(RSI(10)-LLV(RSI(10),10),3)/Sum(HHV(RSI(10),10)-LLV(RSI(10),10),3))*100)"##), Some(r##"Cross((Sum(RSI(10)-LLV(RSI(10),10),3)/Sum(HHV(RSI(10),10)-LLV(RSI(10),10),3))*100,80)"##), Some(r##"Cross((Sum(RSI(10)-LLV(RSI(10),10),3)/Sum(HHV(RSI(10),10)-LLV(RSI(10),10),3))*100,80)"##), Some(r##"Cross(20,(Sum(RSI(10)-LLV(RSI(10),10),3)/Sum(HHV(RSI(10),10)-LLV(RSI(10),10),3))*100)"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "nkms2_swing_index", name: r##"PS Swing Index"##, group: GROUP, family: FAMILY, page: "MS72PRFT.DTA record index 88 (0-based)", status: NotTestable(r##"uses Swing(3), Wilder's Swing Index with a limit move of 3 - a futures exchange's daily price-limit amount with no index, ETF or stock equivalent; the calculation is not printed in the record. Not coded."##), f: [Some(r##"Swing(3)>0"##), Some(r##"Swing(3)<0"##), Some(r##"Swing(3)<0"##), Some(r##"Swing(3)>0"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "nkms2_ultimate_oscillator", name: r##"PS Ultimate Oscillator"##, group: GROUP, family: FAMILY, page: "MS72PRFT.DTA record index 91 (0-based)", status: Live, f: [Some(r##"Ult(7,14,28)>Ref(Ult(7,14,28),-1)"##), Some(r##"Ult(7,14,28)<Ref(Ult(7,14,28),-1)"##), Some(r##"Ult(7,14,28)<Ref(Ult(7,14,28),-1)"##), Some(r##"Ult(7,14,28)>Ref(Ult(7,14,28),-1)"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "nkms2_vertical_horizontal_filter", name: r##"PS Vertical Horizontal Filter"##, group: GROUP, family: FAMILY, page: "MS72PRFT.DTA record index 92 (0-based)", status: NotTestable(r##"uses VHF(C,21), the built-in Vertical Horizontal Filter; its calculation is printed neither in the record nor in the MetaStock 8 manual ('predefined'). Not coded."##), f: [Some(r##"Cross(VHF(C,21),.3) AND LinRegSlope(C,21)>0"##), Some(r##"Cross(.75,VHF(C,21)) OR Cross(0,LinRegSlope(C,21))"##), Some(r##"Cross(VHF(C,21),.3) AND LinRegSlope(C,21)<0"##), Some(r##"Cross(.75,VHF(C,21)) OR Cross(LinRegSlope(C,21),0);"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "nkms2_volatility_breakout_chaikin", name: r##"PS Volatility Breakout (Chaikin)"##, group: GROUP, family: FAMILY, page: "MS72PRFT.DTA record index 93 (0-based)", status: NotTestable(r##"uses Vol(10,10), MetaStock's built-in Chaikin's Volatility; its exact definition is not printed (the vault holds mstt_chaikin_s_volatility for the same reason). Not coded."##), f: [Some(r##"Vol(10,10)>Ref(Vol(10,10),-1) AND C>Ref(C,-1)"##), Some(r##"Vol(10,10)<Ref(Vol(10,10),-1) OR C<Ref(C,-1)"##), Some(r##"Vol(10,10)>Ref(Vol(10,10),-1) AND C<Ref(C,-1)"##), Some(r##"Vol(10,10)<Ref(Vol(10,10),-1) OR C>Ref(C,-1)"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "nkms2_volatility_breakout_standard", name: r##"PS Volatility Breakout (Standard)"##, group: GROUP, family: FAMILY, page: "MS72PRFT.DTA record index 94 (0-based)", status: Live, f: [Some(r##"v1:=Std(Log(C/Ref(C,-1)),10)*Sqrt(365)*100;  v1>Ref(v1,-1) AND C>Ref(C,-1)"##), Some(r##"v1:=Std(Log(C/Ref(C,-1)),10)*Sqrt(365)*100;  v1<Ref(v1,-1) OR C<Ref(C,-1)"##), Some(r##"v1:=Std(Log(C/Ref(C,-1)),10)*Sqrt(365)*100;  v1>Ref(v1,-1) AND C<Ref(C,-1)"##), Some(r##"v1:=Std(Log(C/Ref(C,-1)),10)*Sqrt(365)*100;  v1<Ref(v1,-1) OR C>Ref(C,-1)"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "nkms2_volatility_stop", name: r##"PS Volatility Stop"##, group: GROUP, family: FAMILY, page: "MS72PRFT.DTA record index 95 (0-based)", status: Live, f: [Some(r##"hstop:=Min(Ref(C,-1)+(ATR(4)*2),If(Ref(H,-1)>PREV,C*2,PREV)); lstop:=Max(Ref(C,-1)-(ATR(4)*2),If(Ref(L,-1)<PREV,0,PREV));  BarsSince(C>hstop)>BarsSince(C<lstop)"##), Some(r##"hstop:=Min(Ref(C,-1)+(ATR(4)*2),If(Ref(H,-1)>PREV,C*2,PREV)); lstop:=Max(Ref(C,-1)-(ATR(4)*2),If(Ref(L,-1)<PREV,0,PREV));  BarsSince(C>hstop)<BarsSince(C<lstop)"##), Some(r##"hstop:=Min(Ref(C,-1)+(ATR(4)*2),If(Ref(H,-1)>PREV,C*2,PREV)); lstop:=Max(Ref(C,-1)-(ATR(4)*2),If(Ref(L,-1)<PREV,0,PREV));  BarsSince(C>hstop)<BarsSince(C<lstop)"##), Some(r##"hstop:=Min(Ref(C,-1)+(ATR(4)*2),If(Ref(H,-1)>PREV,C*2,PREV)); lstop:=Max(Ref(C,-1)-(ATR(4)*2),If(Ref(L,-1)<PREV,0,PREV));  BarsSince(C>hstop)>BarsSince(C<lstop)"##)], exec: EXEC_DEFAULT, notes: r##"Run by hand-written code: PREV = the variable's own value on the previous bar (MetaStock 8 manual p.246); [reading] each stop is undefined while ATR(4) is undefined and PREV = 0 on its first defined bar (the manual gives no start value). As stored, Enter Long fires when the close last broke BELOW the lower stop more recently than ABOVE the upper stop."## },
    Sys { id: "nkms2_volume_trading_system", name: r##"PS Volume Trading System"##, group: GROUP, family: FAMILY, page: "MS72PRFT.DTA record index 96 (0-based)", status: Live, f: [Some(r##"C>Ref(C,-1) AND V>Ref(V,-1)"##), Some(r##"C<Ref(C,-1) AND V>Ref(V,-1)"##), Some(r##"C<Ref(C,-1) AND V>Ref(V,-1)"##), Some(r##"C>Ref(C,-1) AND V>Ref(V,-1)"##)], exec: EXEC_DEFAULT, notes: r##""## },
];

// Instrument admission: NIKSAN MetaStock pack part 2 (Equis PS systems) 2026-10-30 (titles lane K2) block, moved from instrument_policy.rs on 31 Oct 2026 (lane ST).
// Admission only for the module REVIEWED lists (authors/niksan-k2/admission/nkms2_ref.py (header)); kept in one block to merge.
// Runs on actual indices, ETFs and stocks, US and India (index_etf_or_stock, nifty50 basket, declared application).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book=="nkms2" && crate::books::nkms2::reviewed(def.id),
    label: "Equis MetaStock System Tester samples stored in the NIKSAN formula pack, with no market, notes or results; runs on indices, ETFs and single stocks (US and India) are a declared application, one unit per trade, trade at the close.",
    basis: "NIKSAN MetaStock formula pack, MS72PRFT.DTA (Equis PS System Tester records 54-96)",
};
