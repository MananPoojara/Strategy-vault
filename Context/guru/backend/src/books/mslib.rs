//! "Estrategias de inversion con Metastock" - the Libanes web collection of MetaStock System Tester
//! strategies (Spanish, 1998-99; José Luis / members.nbci.com/libanes), research drive MetaStock/
//! @Systems & Indicators/Libanes Formulas y Sistemas/DBZ_Estrategias de inversion metastock.doc (128 pp.
//! when printed by LibreOffice; page refs = that print). Book id `mslib`.
//! Every system is stored as the printed MetaStock formula text and run by the `msws` formula engine.
//! The source's own test convention is used for fills: "la misma demora de 1 día para operar a precio de
//! cierre" (p.66; also printed as "Trade price Close, Trade delay 1" in its reports, pp.12, 19-25):
//! a signal on bar t fills at the close of bar t+1 (parameter `delay`, 0 = MetaStock same-bar close).
//! Built-ins the msws engine lacks are spelled out here as formula text before parsing (the same device
//! msws uses for OBV/CO/PVT), with the MetaStock Pro 8 manual definitions:
//!   MP() = (H+L)/2 (manual p.481); GapUp() = L > Ref(H,-1), GapDown() = H < Ref(L,-1) (manual p.262; the
//!   source itself defines its gap the same way, p.43); Mo(n) = C/Ref(C,-n)*100 (manual p.482), written as
//!   100+ROC(C,n,%); BBandTop/Bot(x,n,m,d) = Mov(x,n,m) +/- d*Std(x,n) (manual pp.254-255, 308);
//!   When(a, op, b) = (a op b) and When(x) = (x): legacy MetaStock syntax; the Civila page glosses
//!   When(C-Mov(C,25,S),>,0) as "today's (close - moving average) > 0" (Civila p.85).
//! Transcription and checks: library/mslib/.
use crate::books::msws::{self, Eng, Exec, Sys, EXEC_DEFAULT};
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
#[allow(unused_imports)]
use crate::connors::Status::{Live, NeedsData, NotTestable};
use std::sync::OnceLock;

pub const BOOK: &str = "mslib";
/// Admitted ids (authors/vendor/laneb/SOURCE-CONTRACT.md).
pub const REVIEWED: &[&str] = &["mslib_alligator_charlis", "mslib_alligator_eiv", "mslib_manda_carayo", "mslib_1234", "mslib_180", "mslib_boomer", "mslib_expansion", "mslib_gilligan", "mslib_lizard", "mslib_pivot"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }
#[allow(dead_code)]
fn _exec_type_check(e: Exec) -> Exec { e }

/// Zero-argument built-ins, as formula text (Fml() bodies are not touched by parameter substitution).
const FML: &[(&str, &str)] = &[("MP()", "(H+L)/2"), ("GapUp()", "L>Ref(H,-1)"), ("GapDown()", "H<Ref(L,-1)"), ("MO_100", "100")];
fn fml(name: &str) -> Option<String> { FML.iter().find(|(k, _)| k.eq_ignore_ascii_case(name.trim())).map(|(_, v)| v.to_string()) }

/// Split a call's argument text at top-level commas.
fn split_args(s: &str) -> Vec<String> {
    let (mut out, mut cur, mut d, mut brace) = (vec![], String::new(), 0i32, false);
    for c in s.chars() {
        match c {
            '{' => brace = true,
            '}' => brace = false,
            '(' if !brace => d += 1,
            ')' if !brace => d -= 1,
            ',' if !brace && d == 0 => { out.push(std::mem::take(&mut cur)); continue; }
            _ => {}
        }
        cur.push(c);
    }
    out.push(cur);
    out
}

/// Rewrite the MetaStock built-ins the msws engine lacks into formula text it runs (see module doc).
pub fn expand(src: &str) -> String {
    let b: Vec<char> = src.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        if c == '{' { while i < b.len() && b[i] != '}' { out.push(b[i]); i += 1; } continue; }
        if c == '"' { out.push(c); i += 1; while i < b.len() && b[i] != '"' { out.push(b[i]); i += 1; } continue; }
        if c.is_ascii_alphabetic() && (i == 0 || !(b[i - 1].is_ascii_alphanumeric() || b[i - 1] == '_')) {
            let j0 = i;
            while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == '_') { i += 1; }
            let w: String = b[j0..i].iter().collect();
            let lw = w.to_ascii_lowercase();
            let mut j = i;
            while j < b.len() && b[j].is_whitespace() { j += 1; }
            if j < b.len() && b[j] == '(' && matches!(lw.as_str(), "mp" | "gapup" | "gapdown" | "when" | "bbandtop" | "bbandbot" | "mo") {
                let (mut k, mut d, mut brace) = (j, 0i32, false);
                while k < b.len() {
                    match b[k] { '{' => brace = true, '}' => brace = false, '(' if !brace => d += 1, ')' if !brace => { d -= 1; if d == 0 { break; } } _ => {} }
                    k += 1;
                }
                if k >= b.len() { out.push_str(&w); continue; }
                let inner: String = b[j + 1..k].iter().collect();
                let a = split_args(&inner);
                let t = |s: &str| s.trim().to_string();
                let rep = match (lw.as_str(), a.len()) {
                    ("mp", 1) if t(&a[0]).is_empty() => Some("Fml(\"MP()\")".to_string()),
                    ("gapup", 1) if t(&a[0]).is_empty() => Some("Fml(\"GapUp()\")".to_string()),
                    ("gapdown", 1) if t(&a[0]).is_empty() => Some("Fml(\"GapDown()\")".to_string()),
                    ("when", 1) => Some(format!("({})", expand(&a[0]))),
                    ("when", 3) if matches!(t(&a[1]).as_str(), ">" | "<" | "=" | ">=" | "<=" | "<>") => Some(format!("(({}){}({}))", expand(&a[0]), t(&a[1]), expand(&a[2]))),
                    ("bbandtop", 4) | ("bbandbot", 4) => {
                        let (x, n, m, dv) = (expand(&a[0]), expand(&a[1]), t(&a[2]), expand(&a[3]));
                        Some(format!("(Mov({x},{n},{m}){}({dv})*Std({x},{n}))", if lw == "bbandtop" { "+" } else { "-" }))
                    }
                    ("mo", 1) => Some(format!("(Fml(\"MO_100\")+ROC(C,{},%))", expand(&a[0]))),
                    _ => None,
                };
                match rep { Some(r) => { out.push_str(&r); i = k + 1; } None => out.push_str(&w) }
                continue;
            }
            out.push_str(&w);
            continue;
        }
        out.push(c);
        i += 1;
    }
    out
}

fn leak(s: String) -> &'static str { Box::leak(s.into_boxed_str()) }
fn expanded(s: &Sys) -> [Option<String>; 4] { [0, 1, 2, 3].map(|k| s.f[k].map(expand)) }
fn consts_of(s: &Sys) -> Vec<f64> {
    let e = expanded(s);
    let srcs: Vec<&str> = e.iter().flatten().map(|x| x.as_str()).collect();
    msws::constants(&srcs)
}
const CONV: &str = "Fills: trade price close, trade delay 1 (the source's own test setting, p.66: a signal on bar t fills at bar t+1's close; parameter delay = 0 gives MetaStock's same-bar close). One position at a time; a position closes only by its own Close formula; an exit plus an opposite entry reverse; no same-direction re-entry on the exit signal bar; parameter side = 1/-1 runs one side alone. MP(), GapUp(), GapDown(), Mo(), BBandTop/Bot() and the legacy When() are the MetaStock manual definitions (module doc). India: runs on NIFTY 50 / indices instead of the Spanish stocks it was written for (instrument swap only); price-point constants (.125, 0.25) are applied literally.";

static CAT: OnceLock<Vec<(&'static [&'static str], &'static str, bool)>> = OnceLock::new();
pub fn catalogue() -> Vec<StrategyDef> {
    let cache = CAT.get_or_init(|| SYS.iter().map(|s| {
        let lab = ["E|Enter Long: ", "X|Close Long: ", "E|Enter Short: ", "X|Close Short: "];
        let rules: Vec<&'static str> = s.f.iter().zip(lab).filter_map(|(f, l)| f.map(|f| leak(format!("{l}{f}")))).collect();
        let live = s.status == Status::Live;
        let vol = live && expanded(s).iter().flatten().any(|f| msws::scan(f, &fml).0);
        let notes = if live { leak(format!("{} Source p. {}. {}", s.notes, s.page, CONV)) } else { leak(format!("{} Source p. {}.", s.notes, s.page)) };
        (&*Box::leak(rules.into_boxed_slice()), notes, vol)
    }).collect());
    SYS.iter().zip(cache).map(|(s, (rules, notes, vol))| StrategyDef {
        id: s.id, name: s.name, group: s.group, session: 0,
        dir: match (s.f[0].is_some(), s.f[2].is_some()) { (true, true) => "Long/Short", (true, false) => "Long", (false, true) => "Short", _ => "Long/Short" },
        status: s.status, needs_volume: *vol, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None,
    }).collect()
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let s = SYS.iter().find(|s| s.id == id)?;
    let v = match s.status { Status::Live => s.name, Status::NeedsData(_) => "data missing", Status::NotTestable(_) => "not testable" };
    Some((s.family.0, s.family.1, v))
}
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let s = SYS.iter().find(|s| s.id == id && s.status == Status::Live)?;
    let mut v = vec![ParamDef { key: "delay", label: "Trade delay in bars before the close fill (1 = the source's test setting, p.66; 0 = MetaStock same-bar close)", def: 1., min: 0., max: 1., step: 1. }];
    if s.f[0].is_some() && s.f[2].is_some() { v.push(ParamDef { key: "side", label: "Sides traded: 0 = both as printed, 1 = long side only, -1 = short side only", def: 0., min: -1., max: 1., step: 1. }); }
    for c in consts_of(s) {
        let int = c.fract() == 0.0;
        let (min, max, step) = if int && c >= 5.0 { ((c * 0.5).round().max(1.0), (c * 2.0).round(), 1.0) } else { (c * 0.5, c * 2.0, (c / 10.0 * 1000.0).round() / 1000.0) };
        v.push(ParamDef { key: leak(msws::const_key(c)), label: leak(format!("Book constant {c} (every use of {c} in this system's printed formulas; Ref offsets excluded)")), def: c, min, max, step: if step > 0.0 { step } else { 0.001 } });
    }
    Some(v)
}
/// Delay the four signal series by `d` bars; a signal before `from` never fills.
fn delay_sig(v: &mut Vec<bool>, d: usize, from: usize) {
    let old = v.clone();
    for i in 0..v.len() { v[i] = i >= from + d && old[i - d]; }
}
pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let s = SYS.iter().find(|s| s.id == id)?;
    if s.status != Status::Live || from >= to || to >= x.r.len() { return None; }
    let side = p.get("side").copied().unwrap_or(0.0).round() as i32;
    let d = p.get("delay").copied().unwrap_or(1.0).round().clamp(0.0, 1.0) as usize;
    let map: Vec<(f64, f64)> = consts_of(s).into_iter().map(|c| (c, p.get(&msws::const_key(c)).copied().unwrap_or(c))).collect();
    let e = expanded(s);
    let srcs: [Option<&str>; 4] = [0, 1, 2, 3].map(|k| e[k].as_deref());
    let fl: &dyn Fn(&str) -> Option<String> = &fml;
    let mut eng = Eng::new(x, fl);
    let mut sig = msws::signals(&mut eng, srcs, &map, side).ok()?;
    if d > 0 { for v in [&mut sig.el, &mut sig.xl, &mut sig.es, &mut sig.xs] { delay_sig(v, d, from); } }
    let mut t = msws::simulate(x, from, to, &sig, EXEC_DEFAULT);
    if d > 0 { for tr in t.iter_mut().filter(|t| !t.open) { tr.signal_i -= d; tr.reason = "Close formula · next bar's close (trade delay 1)".into(); } }
    Some(t)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn rows(n: usize) -> Vec<crate::Row> {
        (0..n).map(|i| { let t = i as f64; let c = 1000. + 80. * (t / 23.).sin() + 40. * (t / 7.).cos() + t * 0.3;
            crate::Row { ticker: "T".into(), date: format!("{:04}-{:02}-{:02}", 2000 + i / 300, 1 + (i / 25) % 12, 1 + i % 25), open: c - 3. * (t / 3.).sin(), high: c + 6. + 2. * (t / 5.).sin().abs(), low: c - 6. - 2. * (t / 4.).cos().abs(), close: c, volume: Some(1e6 + 3e5 * (t / 9.).sin()) } }).collect()
    }
    #[test]
    fn expand_builtins() {
        assert_eq!(expand("When(Mov(C,6,E),>,Mov(C,60,E))"), "((Mov(C,6,E))>(Mov(C,60,E)))");
        assert_eq!(expand("When( ADX(14)>30 AND RSI(14)<30)"), "( ADX(14)>30 AND RSI(14)<30)");
        assert_eq!(expand("Mov(MP(),9,S)"), "Mov(Fml(\"MP()\"),9,S)");
        assert_eq!(expand("BBandTop(CLOSE,10,S,2)"), "(Mov(CLOSE,10,S)+(2)*Std(CLOSE,10))");
        assert_eq!(expand("Mo(14)>100"), "(Fml(\"MO_100\")+ROC(C,14,%))>100");
        assert_eq!(expand("C>Mov(C,30,E)"), "C>Mov(C,30,E)");
    }
    #[test]
    fn every_live_system_evaluates_and_runs() {
        let r = rows(1500);
        let x = Ctx::new(&r, &r, &[]);
        for s in SYS.iter() {
            let res = run(s.id, &x, 300, 1499, &Params::new());
            assert_eq!(res.is_some(), s.status == Status::Live, "{}", s.id);
            for t in res.unwrap_or_default() { assert!(t.legs[0].i >= 301 && t.exit_i <= 1499 && t.exit_i > t.legs[0].i || t.open, "{}", s.id); }
        }
    }
    #[test]
    fn cruce_dorado_fills_next_close() {
        // Cross(EMA6, EMA60) up with both rising on bar t -> long at bar t+1's close (trade delay 1)
        let r = rows(1500);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("mslib_cruce_dorado", &x, 300, 1499, &Params::new()).unwrap();
        let (e6, e60) = (msws::ema(&x.c, 6), msws::ema(&x.c, 60));
        assert!(!t.is_empty());
        for tr in t.iter().filter(|t| t.dir > 0.) {
            let s = tr.legs[0].i - 1;
            assert!(e6[s] > e60[s] && e6[s - 1] <= e60[s - 1] && e6[s - 1] < e6[s] && e60[s - 1] < e60[s]);
            assert_eq!(tr.legs[0].px, x.c[s + 1]);
        }
    }
}

const SYS: &[Sys] = &[
    Sys { id: "mslib_ema_opt", name: r##"Estrategia con dos medias móviles exponenciales"##, group: r##"Estrategias con Medias Móviles"##, family: ("mslib_ma", r##"Moving-average strategies"##), page: "1", status: NotTestable(r##"periods are given only as optimisation ranges (Opt1 17-20, Opt2 72-80, Opt3 17-20, Opt4 72-80); the source states no single value"##), f: [Some(r##"When(Mov(C,opt1,E),>,Mov(C,opt2,E))"##), Some(r##"When(Mov(C,opt3,E),<,Mov(C,opt4,E))"##), None, None], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "mslib_ponte_mp", name: r##"Median Price con filtro (Melchor Ponte)"##, group: r##"Estrategias con Medias Móviles"##, family: ("mslib_ma", r##"Moving-average strategies"##), page: "2-5", status: NotTestable(r##"the printed formula writes Ref(Cross(...) AND (Ref(C,+1) > C),-1): the net offsets are causal (cross 1-3 days ago and today's close above that day's close), but the books::msws engine rejects any positive Ref offset, so it cannot run "exactly as printed" without an engine change"##), f: [Some(r##"Ref(Cross(Mov(MP(),9,S),Mov(MP(),18,S)) AND (Ref(C,+1) > C),-1) OR (Ref(Cross(Mov(MP(),9,S),Mov(MP(),18,S)) AND Ref(C,+2) > C,-2)) OR (Ref(Cross(Mov(MP(),9,S),Mov(MP(),18,S)) AND Ref(C,+3) > C,-3))"##), Some(r##"Ref(Cross(Mov(MP(),18,S),Mov(MP(),9,S)) AND (Ref(C,+1) < C),-1) OR (Ref(Cross(Mov(MP(),18,S),Mov(MP(),9,S)) AND Ref(C,+2) < C,-2)) OR (Ref(Cross(Mov(MP(),18,S),Mov(MP(),9,S)) AND Ref(C,+3) < C,-3))"##), None, None], exec: EXEC_DEFAULT, notes: r##"Published (stock not named, to 14/09/98): 30 closed long trades, 20 winners, net profit 6257.86 on 1000, commissions 0.5%."## },
    Sys { id: "mslib_cml", name: r##"Estrategia CML con tres medias móviles exponenciales"##, group: r##"Estrategias con Medias Móviles"##, family: ("mslib_ma", r##"Moving-average strategies"##), page: "5", status: Live, f: [Some(r##"When(Mov(C,5,E)>Mov(C,13,E) AND Mov(C,13,E)>Mov(C,34,E))"##), Some(r##"When(Mov(C,13,E)>Mov(C,34,E) AND Mov(C,34,E)>Mov(C,5,E))"##), Some(r##"When(Mov(C,34,E)>Mov(C,13,E) AND Mov(C,13,E)>Mov(C,5,E))"##), Some(r##"When(Mov(C,5,E)>Mov(C,34,E) AND Mov(C,34,E)>Mov(C,13,E))"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "mslib_cruce_dorado", name: r##"Cruce dorado 6/60"##, group: r##"Estrategias con Medias Móviles"##, family: ("mslib_ma", r##"Moving-average strategies"##), page: "5-6", status: Live, f: [Some(r##"Cross(Mov(C,6,E), Mov(C,60,E)) AND Ref(Mov(C,6,E),-1)<Mov(C,6,E) AND Ref(Mov(C,60,E),-1)<Mov(C,60,E)"##), Some(r##"Cross(Mov(C,60,E), Mov(C,6,E))"##), Some(r##"Cross(Mov(C,60,E), Mov(C,6,E))"##), Some(r##"Cross(Mov(C,6,E), Mov(C,60,E)) AND Ref(Mov(C,6,E),-1)<Mov(C,6,E) AND Ref(Mov(C,60,E),-1)<Mov(C,60,E)"##)], exec: EXEC_DEFAULT, notes: r##""OPERA a largo y a corto"."## },
    Sys { id: "mslib_alligator_charlis", name: r##"Alligator, Versión Charlis"##, group: r##"Estrategias con Medias Móviles"##, family: ("mslib_alligator", r##"Alligator (Bill Williams) versions"##), page: "6-7", status: Live, f: [Some(r##"(Mov(Ref(MP(),-3),5,S))> (Mov(Ref(MP(),-5),8,S)) AND (Mov(Ref(MP(),-5),8,S))>(Mov(Ref(MP(),-8),13,S))"##), Some(r##"(Mov(Ref(MP(),-3),5,S))< (Mov(Ref(MP(),-5),8,S)) AND (Mov(Ref(MP(),-5),8,S))<(Mov(Ref(MP(),-8),13,S))"##), None, None], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "mslib_alligator_eiv", name: r##"Alligator, Versión eiv"##, group: r##"Estrategias con Medias Móviles"##, family: ("mslib_alligator", r##"Alligator (Bill Williams) versions"##), page: "7-8", status: Live, f: [Some(r##"(Mov(Ref(MP(),-3),5,S)>Mov(Ref(MP(),-5),8,S) AND Mov(Ref(MP(),-5),8,S)>Mov(Ref(MP(),-8),13,S)) OR (When(C, >,Mov(Ref(MP(),-5),8,S) AND When( Ref(C,-1), <=,Mov(Ref(MP(),-5),8,S))))"##), Some(r##"(Mov(Ref(MP(),-3),5,S)<Mov(Ref(MP(),-5),8,S) AND Mov(Ref(MP(),-5),8,S)<Mov(Ref(MP(),-8),13,S)) OR (When( C, <=,Mov(Ref(MP(),-5),8,S) AND When( Ref(C,-1), >,Mov(Ref(MP(),-5),8,S))))"##), None, None], exec: EXEC_DEFAULT, notes: r##"As printed the When( ... ) brackets nest: When(C, >, X AND When(Ref(C,-1), <=, X)) = C > (X AND (Ref(C,-1) <= X)); MetaStock precedence then compares C with a 0/1 value. Run literally."## },
    Sys { id: "mslib_gumbert", name: r##"Estrategia Gumbert con Movimiento Direccional"##, group: r##"Estrategias con Movimiento Direccional"##, family: ("mslib_dmi", r##"Directional-movement strategies"##), page: "9-10", status: NotTestable(r##"opt1..opt4 are optimisation ranges (8-12, 2-6, 15-20, 21-27); the author reports two found sets (10,4,19,23 and 10,4,16,26) without choosing one"##), f: [Some(r##"Cross(Mov(PDI(opt1),opt2,E),Mov(MDI(opt3),opt4,E))"##), Some(r##"Cross(Mov(MDI(opt1),opt2,E),Mov(PDI(opt3),opt4,E))"##), Some(r##"Cross(Mov(MDI(opt1),opt2,E),Mov(PDI(opt3),opt4,E))"##), Some(r##"Cross(Mov(PDI(opt1),opt2,E),Mov(MDI(opt3),opt4,E))"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "mslib_macd_adx", name: r##"Estrategia MACD+ADX"##, group: r##"Estrategias Combinadas"##, family: ("mslib_combined", r##"Combined MACD / ADX / RSI strategies"##), page: "10-11", status: NotTestable(r##"periods are given only as optimisation ranges (opt1 4-20 step 2); the source states no single value"##), f: [Some(r##"ADX(14)>30 and (Cross( MACD(), Mov( MACD(), opt1, EXPONENTIAL) )"##), Some(r##"ADX(14)>30 AND Cross(Mov(MACD(),9,EXPONENTIAL),MACD())"##), Some(r##"ADX(14)>30 AND Cross(Mov(MACD(),9,EXPONENTIAL),MACD())"##), Some(r##"ADX(14)>30 and (Cross( MACD(), Mov( MACD(), opt1, EXPONENTIAL) )"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "mslib_fierro", name: r##"Estrategia Fierro"##, group: r##"Estrategias Combinadas"##, family: ("mslib_combined", r##"Combined MACD / ADX / RSI strategies"##), page: "11-13", status: Live, f: [Some(r##"When(Mov(C,6,E),>,Mov(C,60,E)) OR When(MACD(),>,Mov(MACD(),9,EXPONENTIAL))"##), Some(r##"When(Mov(C,6,E),<,Mov(C,60,E)) AND When(MACD(),<,Mov(MACD(),9,EXPONENTIAL))"##), Some(r##"When(Mov(C,6,E),<,Mov(C,60,E)) AND When(MACD(),<,Mov(MACD(),9,EXPONENTIAL))"##), Some(r##"When(Mov(C,6,E),>,Mov(C,60,E)) OR When(MACD(),>,Mov(MACD(),9,EXPONENTIAL))"##)], exec: EXEC_DEFAULT, notes: r##"Published: Ibex-35 14/01/87-18/12/98, positions long and short, trade price close, trade delay 1, commission 0.5% each way: 101 closed trades (50 long / 51 short), 35 winners, net profit 3013.39 on 1000 (+301.34%), buy & hold 293.46%. No Ibex-35 data in the vault, so not reproducible."## },
    Sys { id: "mslib_fierramon", name: r##"Estrategia Fierramón"##, group: r##"Estrategias Combinadas"##, family: ("mslib_combined", r##"Combined MACD / ADX / RSI strategies"##), page: "13-14", status: NotTestable(r##"periods are given only as optimisation ranges (OPT1 1.05-1.1 by 0.025; the text says the stop "debe de optimizarse"); the source states no single value"##), f: [Some(r##"MACD()>Mov( MACD(),9,E)"##), Some(r##"Ref(C,-2)/C > opt1 AND MACD()<Mov( MACD(),9,E)"##), None, None], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "mslib_fierrando", name: r##"Estrategia Fierrando (Fernando Medina)"##, group: r##"Estrategias Combinadas"##, family: ("mslib_combined", r##"Combined MACD / ADX / RSI strategies"##), page: "14", status: Live, f: [Some(r##"Mov(C,8,E)>Mov(C,55,E)*1.02 OR MACD()>Mov(MACD(),21,E)*1.02"##), Some(r##"(Mov(C,8,E)<Mov(C,55,E)*0.98) AND (MACD()<Mov(MACD(),21,E)*0.98)"##), None, None], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "mslib_bb_rsi", name: r##"Estrategia combinando Bandas de Bollinguer con RSI"##, group: r##"Estrategias Combinadas"##, family: ("mslib_combined", r##"Combined MACD / ADX / RSI strategies"##), page: "14-15", status: Live, f: [Some(r##"CLOSE<=BBandBot(CLOSE,10,S,2) AND RSI(10)<25 AND BBandTop(CLOSE,10,S,2)<>BBandBot(CLOSE,10,S,2)"##), Some(r##"CLOSE>=BBandTop(CLOSE,10,S,2) AND RSI(10)>75 AND BBandTop(CLOSE,10,S,2)<>BBandBot(CLOSE,10,S,2)"##), Some(r##"CLOSE>=BBandTop(CLOSE,10,S,2) AND RSI(10)>75 AND BBandTop(CLOSE,10,S,2)<>BBandBot(CLOSE,10,S,2)"##), Some(r##"CLOSE<=BBandBot(CLOSE,10,S,2) AND RSI(10)<25 AND BBandTop(CLOSE,10,S,2)<>BBandBot(CLOSE,10,S,2)"##)], exec: EXEC_DEFAULT, notes: r##"Printed as two formulas: "Abrir largos y cerrar cortos" (= Enter Long and Close Short) and "Cerrar largos y abrir cortos" (= Close Long and Enter Short)."## },
    Sys { id: "mslib_combi", name: r##"Estrategia COMBI"##, group: r##"Estrategias Combinadas"##, family: ("mslib_combined", r##"Combined MACD / ADX / RSI strategies"##), page: "15-16", status: Live, f: [Some(r##"(When( ADX(14)>30 AND Cross(MACD(),Mov(MACD(),9,EXPONENTIAL)))) OR (When( ADX(14)<=30 AND RSI(14)<30))"##), Some(r##"(When( ADX(14)>30 AND Cross(Mov(MACD(),9,EXPONENTIAL),MACD()))) OR (When( ADX(14)<=30 AND RSI(14)>70))"##), Some(r##"(When( ADX(14)>30 AND Cross(Mov(MACD(),9,EXPONENTIAL),MACD()))) OR (When( ADX(14)<=30 AND RSI(14)>70))"##), Some(r##"(When( ADX(14)>30 AND Cross(MACD(),Mov(MACD(),9,EXPONENTIAL)))) OR (When( ADX(14)<=30 AND RSI(14)<30))"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "mslib_combi_opt", name: r##"Estrategia COMBI, versión optimizada"##, group: r##"Estrategias Combinadas"##, family: ("mslib_combined", r##"Combined MACD / ADX / RSI strategies"##), page: "16-17", status: NotTestable(r##"periods are given only as optimisation ranges (opt1 22-32, opt2 12-18, opt3 26-36, opt4 7-11, opt5 12-18, opt6 70-76); the source states no single value"##), f: [Some(r##"(When( ADX(opt2)>opt1 AND Cross(MACD(),Mov(MACD(),opt4,EXPONENTIAL)))) OR (When( ADX(opt2)<=opt1 AND RSI(opt5)<opt3))"##), Some(r##"(When( ADX(opt2)>opt1 AND Cross(Mov(MACD(),opt4,EXPONENTIAL),MACD()))) OR (When( ADX(opt2)<=opt1 AND RSI(opt5)>opt6))"##), Some(r##"(When( ADX(opt2)>opt1 AND Cross(Mov(MACD(),opt4,EXPONENTIAL),MACD()))) OR (When( ADX(opt2)<=opt1 AND RSI(opt5)>opt6))"##), Some(r##"(When( ADX(opt2)>opt1 AND Cross(MACD(),Mov(MACD(),opt4,EXPONENTIAL)))) OR (When( ADX(opt2)<=opt1 AND RSI(opt5)<opt3))"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "mslib_presario", name: r##"Presario (Joramón)"##, group: r##"Estrategias Combinadas"##, family: ("mslib_combined", r##"Combined MACD / ADX / RSI strategies"##), page: "17-18", status: Live, f: [Some(r##"((Mov(MP(),200,S)<C)+(C>Ref(C,-1))+(C>Mov(C,10,E))+(C>Mov(C,30,E))+(C>Mov(C,70,E))+(MACD()>Mov(MACD(),9,E))+(H>Ref(HHV(H,14),-1))+(L>Ref(LLV(L,14),-1))+0.5*(RSI(14)<30)+0.5*(RSI(14)<=70)+(Stoch(5,3)>Mov(Stoch(5,3),3,E))+(Mo(14)>100)+(OBV()>Ref(OBV(),-1))+(V>Mov(V,200,S)))>=11"##), Some(r##"((Mov(MP(),200,S)<C)+(C>Ref(C,-1))+(C>Mov(C,10,E))+(C>Mov(C,30,E))+(C>Mov(C,70,E))+(MACD()>Mov(MACD(),9,E))+(H>Ref(HHV(H,14),-1))+(L>Ref(LLV(L,14),-1))+0.5*(RSI(14)<30)+0.5*(RSI(14)<=70)+(Stoch(5,3)>Mov(Stoch(5,3),3,E))+(Mo(14)>100)+(OBV()>Ref(OBV(),-1))+(V>Mov(V,200,S)))<=4"##), None, None], exec: EXEC_DEFAULT, notes: r##"The page wraps "Mo-v(C,30,E)" and "0-.5" across lines; joined without spaces. 14 factors ("se compra si se superan 11 puntos y se vende si se alcanzan solo 4 puntos o menos")."## },
    Sys { id: "mslib_kowalsky", name: r##"Rompe-Tendencias de Juan Kowalsky"##, group: r##"Estrategias con Rompetendencias"##, family: ("mslib_rt", r##"Rompetendencias (trend-break) strategies"##), page: "18-20", status: NotTestable(r##"periods are given only as optimisation ranges (opt1 8-15, opt2 16-25); the source states no single value"##), f: [Some(r##"When( LLV(LOW,opt1) > LLV(LOW,opt2) )"##), Some(r##"When( HHV(HIGH,opt1) < HHV(HIGH,opt2))"##), None, None], exec: EXEC_DEFAULT, notes: r##"Published: Banco Santander 1998 (273 days), 7 trades, 3 winners - Spanish stock, no data."## },
    Sys { id: "mslib_rt2", name: r##"Rompetendencias-2 (A. Gonzalez)"##, group: r##"Estrategias con Rompetendencias"##, family: ("mslib_rt", r##"Rompetendencias (trend-break) strategies"##), page: "20-22", status: NotTestable(r##"periods are given only as optimisation ranges (OPT1 8-15, OPT2 16-25); the source states no single value"##), f: [Some(r##"When( HHV(H,opt1) = HHV(H,opt2) )"##), Some(r##"When( LLV(L,opt1) = LLV(L,opt2))"##), None, None], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "mslib_rt3", name: r##"Rompetendencias-3 (A. Gonzalez)"##, group: r##"Estrategias con Rompetendencias"##, family: ("mslib_rt", r##"Rompetendencias (trend-break) strategies"##), page: "22-24", status: NotTestable(r##"periods are given only as optimisation ranges (OPT1 8-15, OPT2 16-25); the source states no single value"##), f: [Some(r##"When( HHV(H,opt1) = HHV(H,opt2) )"##), Some(r##"When( HHV(H,opt1) < HHV(H,opt2))"##), None, None], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "mslib_rt4", name: r##"Rompetendencias-4 (A. Gonzalez)"##, group: r##"Estrategias con Rompetendencias"##, family: ("mslib_rt", r##"Rompetendencias (trend-break) strategies"##), page: "24-26", status: NotTestable(r##"periods are given only as optimisation ranges (OPT1 8-15, OPT2 16-25); the source states no single value"##), f: [Some(r##"When( LLV(L,opt1) > LLV(L,opt2) )"##), Some(r##"When( LLV(L,opt1) = LLV(L,opt2))"##), None, None], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "mslib_widner_sr", name: r##"Estrategia de Mel Widner con Soportes y Resistencias Automaticos"##, group: r##"Estrategias con Techos y Suelos"##, family: ("mslib_misc", r##"Other strategies"##), page: "26-30", status: NotTestable(r##"duplicate of vault entry dgmf_widner_sr (Not testable there): exits are MetaStock stop settings only (breakeven floor 2%, profit risk 10% ignoring 10 periods, maximum loss 7%); S1/R1 here also print Ref(LOW,4) (future bar) and use PREVIOUS"##), f: [Some(r##"Fml("WSO") > Mov( Fml("WSO") , 4 , S ) OR Mov( Fml("WRO") , 30 ,S ) > 95"##), None, None, None], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "mslib_mo_ros", name: r##"Estrategia de Ramón Ros de compraventa por Momento"##, group: r##"Estrategias con indicadores líderes"##, family: ("mslib_mo", r##"Momentum strategies"##), page: "30", status: NotTestable(r##"periods are given only as optimisation ranges (Opt1 5-20); the source states no single value"##), f: [Some(r##"Mo( opt1 ) > 100"##), Some(r##"Mo(opt1)<100"##), None, None], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "mslib_mome5", name: r##"Estrategia Mome-5"##, group: r##"Estrategias con indicadores líderes"##, family: ("mslib_mo", r##"Momentum strategies"##), page: "30-32", status: NotTestable(r##"periods are given only as optimisation ranges (OPT1 50-60, OPT2 130-160, OPT3 18-30); the source states no single value"##), f: [Some(r##"Mov(Mo(opt1),opt3,S)=LLV(Mov(Mo(opt1),opt3,S),opt2)"##), Some(r##"Mov(Mo(opt1),opt3,S)=HHV(Mov(Mo(opt1),opt3,S),opt2)"##), None, None], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "mslib_bns", name: r##"Estrategia BNS de Wolfgang y Chandler (Bill's New System)"##, group: r##"Estrategias complejas"##, family: ("mslib_bns", r##"Bill's New System variants"##), page: "32-34", status: NotTestable(r##"BNS2 uses OscP(13,55,E,%), a built-in price oscillator whose exact %-definition the source does not print (same treatment as mstt)"##), f: [Some(r##"ref(fml("BNS"),-1)<0 and fml("BNS")>0"##), Some(r##"ref(fml("BNS"),-1)>0 and fml("BNS")<0"##), None, None], exec: EXEC_DEFAULT, notes: r##"Published: Banco Santander 1995-1998, 20 trades, 10 winners (Spanish stock, no data)."## },
    Sys { id: "mslib_bns_robert", name: r##"Estrategia BNS de Frank Robert"##, group: r##"Estrategias complejas"##, family: ("mslib_bns", r##"Bill's New System variants"##), page: "34-35", status: NotTestable(r##"uses oscv(10,24,s,%), a volume oscillator whose definition the source does not print"##), f: [Some(r##"( (((H+L+C)/3)-Mov(C,28,S)) /(.015*Std(C,28))<0 ) and ( Ref((((H+L+C)/3)-Mov(C,28,S)) /(.015*Std(C,28)),-1)<(((H+L+C)/3)-Mov(C,28,S)) /(.015*Std(C,28)) ) and (Mov(Std(((h+l+c)/3),28),5,s) <mov(Std(((h+l+c)/3),34), 20, s)) and (ref(Fml("StocRSI(8,5)"),-1)<30) and(oscv(10,24,s,%)<-15)"##), Some(r##"((((H+L+C)/3)-Mov(C,28,S))/(.015*Std(C,28)) <0 )"##), None, None], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "mslib_bns2", name: r##"Estrategia BNS-2"##, group: r##"Estrategias complejas"##, family: ("mslib_bns", r##"Bill's New System variants"##), page: "35-36", status: NotTestable(r##"uses OscP(13,55,E,%), a built-in price oscillator whose exact %-definition the source does not print"##), f: [Some(r##"mov(fml("BradCCI Line 1"), 5, s)>mov(fml("BradCCI Line 1"), 34, s) and mov(stoch(8,5),5,s)>mov(stoch(8,5),34,s) and mov(fml("BradCCI Line 2"), 5, s) >mov(fml("BradCCI Line 2"), 34, s) and mov(OscP(13,55,E,% ), 5, s)>mov(OscP(13,55,E,% ), 34, s)"##), Some(r##"mov(fml("BradCCI Line 1"), 5, s)<mov(fml("BradCCI Line 1"), 34, s) and mov(stoch(8,5),5,s)<mov(stoch(8,5),34,s) and mov(fml("BradCCI Line 2"), 5, s)<mov(fml("BradCCI Line 2"), 34, s) and mov(OscP(13,55,E,% ), 5, s) <mov(OscP(13,55,E,% ), 34, s)"##), None, None], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "mslib_1234", name: r##"Estrategia 1234 - Hit and Run"##, group: r##"Estrategias de Corto Plazo"##, family: ("mslib_hr", r##"Short-term (Hit and Run) MetaStock versions"##), page: "36-39", status: Live, f: [Some(r##"(ADX(14)>30 AND PDI(14)>MDI(14) AND {3 lower low in a row} (L<Ref(L,-1) AND Ref(L,-1)<(Ref(L,-2) AND Ref(L,-2)<Ref(L,-3)) OR {2 lower lows in a row and an inside day} Ref(L,-2)<Ref(L,-3) AND Ref(L,-1)<Ref(L,-2) AND H<=Ref(H,-1) AND L>=Ref(L,-1)) OR {an inside day and 2 lower lows} (Ref((H<=Ref(H,-1) AND (L>=Ref(L,-1))),-2) AND Ref(L,-1)<Ref(L,-2) AND L<Ref(L,-1)))"##), Some(r##"(ADX(14)>30 AND PDI(14)<MDI(14) AND {3 higher highs in a row} (H>Ref(H,-1) AND Ref(H,-1)>(Ref(H,-2) AND Ref(H,-2)>Ref(H,-3)) OR {2 higher highs in a row and an inside day} Ref(H,-2)>Ref(H,-3) AND Ref(H,-1)>Ref(H,-2) AND H<=Ref(H,-1) AND L>=Ref(L,-1)) OR {an inside day and 2 higher highs} (Ref((H<=Ref(H,-1) AND (L>=Ref(L,-1))),-2) AND Ref(H,-1)>Ref(H,-2) AND H>Ref(H,-1)))"##), None, None], exec: EXEC_DEFAULT, notes: r##"Run literally as printed, including the bracket placement Ref(L,-1)<(Ref(L,-2) AND Ref(L,-2)<Ref(L,-3)) (a price compared with a 0/1 value) and the grouping that puts the second and third patterns outside the ADX/DI condition. This is the Spanish author's MetaStock version (with its own exit); Connors/Cooper's own 1-2-3-4 is in the vault separately (jchr2/ats)."## },
    Sys { id: "mslib_180", name: r##"Estrategia 180 - Hit and Run"##, group: r##"Estrategias de Corto Plazo"##, family: ("mslib_hr", r##"Short-term (Hit and Run) MetaStock versions"##), page: "39-40", status: Live, f: [Some(r##"C>Mov(C,10,S) AND C>Mov(C,50,S) AND Ref(C,-1)<=(Ref(L,-1)+(Ref(H-L,-1)*.25)) AND C>=L+((H-L)*.75)"##), Some(r##"C<Mov(C,10,S) AND C<Mov(C,50,S) AND Ref(C,-1)>=(Ref(L,-1)+(Ref(H-L,-1)*.75)) AND C<=L+((H-L)*.25)"##), None, None], exec: EXEC_DEFAULT, notes: r##"Cooper's own 180's are in the vault as jchr1_180 (Not testable: discretionary exits); this is the source's MetaStock version with its printed exit."## },
    Sys { id: "mslib_boomer", name: r##"Estrategia Boomer"##, group: r##"Estrategias de Corto Plazo"##, family: ("mslib_hr", r##"Short-term (Hit and Run) MetaStock versions"##), page: "40-42", status: Live, f: [Some(r##"ADX(14)>30 AND PDI(14)>MDI(14) AND H<=Ref(H,-1) AND Ref(H,-1)<=Ref(H,-2) AND L>=Ref(L,-1) AND Ref(L,-1)>=Ref(L,-2)"##), Some(r##"ADX(14)>30 AND PDI(14)<MDI(14) AND H<=Ref(H,-1) AND Ref(H,-1)<=Ref(H,-2) AND L>=Ref(L,-1) AND Ref(L,-1)>=Ref(L,-2)"##), None, None], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "mslib_expansion", name: r##"Estrategia Expansión"##, group: r##"Estrategias de Corto Plazo"##, family: ("mslib_hr", r##"Short-term (Hit and Run) MetaStock versions"##), page: "42-43", status: Live, f: [Some(r##"C>Ref(HHV(C,42),-1) AND (H-L)>=Ref(HHV((H-L),9),-1)"##), Some(r##"C<Ref(LLV(C,42),-1) AND (H-L)>=Ref(HHV((H-L),9),-1)"##), None, None], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "mslib_gilligan", name: r##"Estrategia Gilligan"##, group: r##"Estrategias de Corto Plazo"##, family: ("mslib_hr", r##"Short-term (Hit and Run) MetaStock versions"##), page: "43-44", status: Live, f: [Some(r##"GapDown() AND O<Ref(LLV(L,41),-1) AND C>=((H+L)/2) AND C>=O"##), Some(r##"GapUp() AND O>Ref(HHV(H,41),-1) AND C<=((H+L)/2) AND C<=O"##), None, None], exec: EXEC_DEFAULT, notes: r##"The text defines the gap itself: "si el mínimo del día anterior es mayor que el máximo del día" (GapDown)."## },
    Sys { id: "mslib_lizard", name: r##"Estrategia Lizard (lagarto)"##, group: r##"Estrategias de Corto Plazo"##, family: ("mslib_hr", r##"Short-term (Hit and Run) MetaStock versions"##), page: "44-46", status: Live, f: [Some(r##"O>=L+((H-L)*.75) AND C>=L+((H-L)*.75) AND L<Ref(LLV(L,9),-1)"##), Some(r##"O<=L+((H-L)*.25) AND C<=L+((H-L)*.25) AND H>Ref(HHV(H,9),-1)"##), None, None], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "mslib_pivot", name: r##"Estrategia Pivot"##, group: r##"Estrategias de Corto Plazo"##, family: ("mslib_hr", r##"Short-term (Hit and Run) MetaStock versions"##), page: "46-47", status: Live, f: [Some(r##"H-L>Ref(HHV((H-L),9),-1) AND ((L<= Mov(C,50,S)) OR Ref(L,-1)<=Ref(Mov(C,50,S),-1)) AND C>Mov(C,50,S)"##), Some(r##"H-L>Ref(HHV((H-L),9),-1) AND ((H>= Mov(C,50,S)) OR Ref(H,-1)>=Ref(Mov(C,50,S),-1)) AND C<Mov(C,50,S)"##), None, None], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "mslib_slingshot", name: r##"Estrategia Slingshot (tirador de honda)"##, group: r##"Estrategias de Corto Plazo"##, family: ("mslib_hr", r##"Short-term (Hit and Run) MetaStock versions"##), page: "47-49", status: Live, f: [Some(r##"(Ref(H>Ref(HHV(H,42),-1),-1) AND L<=(Ref(L,-1)-.125)) OR (Ref(H>Ref(HHV(H,42),-1),-2) AND Ref(L<=(Ref(L,-2)-.125),-1))"##), Some(r##"(Ref(L<Ref(LLV(L,42),-1),-1) AND H>=(Ref(H,-1)+.125)) OR (Ref(L<Ref(LLV(L,42),-1),-2) AND Ref(H>=(Ref(H,-2)+.125),-1))"##), None, None], exec: EXEC_DEFAULT, notes: r##".125 is a price-point (1/8) constant, applied literally on the index."## },
    Sys { id: "mslib_whoops", name: r##"Estrategia Whoops (alaridos)"##, group: r##"Estrategias de Corto Plazo"##, family: ("mslib_hr", r##"Short-term (Hit and Run) MetaStock versions"##), page: "49-50", status: Live, f: [Some(r##"C<Mov(C,10,S) AND C<Mov(C,50,S) AND O<Ref(C,-1)-0.25"##), Some(r##"C>Mov(C,10,S) AND C>Mov(C,50,S) AND O>Ref(C,-1)+0.25"##), None, None], exec: EXEC_DEFAULT, notes: r##"0.25 is a price-point ("un cuarto de punto") constant, applied literally on the index."## },
    Sys { id: "mslib_forman", name: r##"Estrategia Forman (John Forman)"##, group: r##"Estrategias de Corto Plazo"##, family: ("mslib_misc", r##"Other strategies"##), page: "50", status: Live, f: [Some(r##"CLOSE< Mov(C,20,S)-(2*Stdev(C,20))*1.002"##), Some(r##"CLOSE<Ref(CLOSE,-1)"##), Some(r##"CLOSE>Mov(C,20,S)+(2*Stdev(C,20))*0.998"##), Some(r##"CLOSE>Ref(CLOSE,-1)"##)], exec: EXEC_DEFAULT, notes: r##"As printed, *1.002 / *0.998 multiply only the 2*Stdev term (MetaStock precedence)."## },
    Sys { id: "mslib_demark_seq", name: r##"Demark Sequential"##, group: r##"Estrategias de Corto Plazo"##, family: ("mslib_misc", r##"Other strategies"##), page: "51-52", status: Live, f: [Some(r##"(C<Ref(C,-4) AND Ref(C,-1)<Ref(C,-5) AND Ref(C,-2)<Ref(C,-6) AND Ref(C,-3)<Ref(C,-7) AND Ref(C,-4)<Ref(C,-8) AND Ref(C,-5)<Ref(C,-9) AND Ref(C,-6)<Ref(C,-10) AND Ref(C,-7)<Ref(C,-11) AND Ref(C,-8)<Ref(C,-12))"##), Some(r##"(C>Ref(C,-4) AND Ref(C,-1)>Ref(C,-5) AND Ref(C,-2)>Ref(C,-6) AND Ref(C,-3)>Ref(C,-7) AND Ref(C,-4)>Ref(C,-8) AND Ref(C,-5)>Ref(C,-9) AND Ref(C,-6)>Ref(C,-10) AND Ref(C,-7)>Ref(C,-11) AND Ref(C,-8)>Ref(C,-12))"##), None, None], exec: EXEC_DEFAULT, notes: r##"Prose says "ocho dias seguidos"; the formula prints nine comparisons (C .. Ref(C,-8)) - the formula is run. Differs from mstt_demark_setup_long (13 bars + intersection, other exit)."## },
    Sys { id: "mslib_psar", name: r##"Estrategia usando Parabolic Sar"##, group: r##"Otras"##, family: ("mslib_misc", r##"Other strategies"##), page: "52-53", status: NotTestable(r##"opt1 0.01-0.06 and opt2 0.05-0.5 are optimisation ranges; the two parameter sets quoted (0.01/0.06 long only, 0.025/0.06 long and short) are optimisation results on the Ibex"##), f: [Some(r##"HIGH > SAR(opt1,opt2)"##), Some(r##"LOW < SAR(opt1,opt2)"##), Some(r##"LOW < SAR(opt1,opt2)"##), Some(r##"HIGH > SAR(opt1,opt2)"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "mslib_fractal", name: r##"Estrategia con fractales"##, group: r##"Otras"##, family: ("mslib_misc", r##"Other strategies"##), page: "53", status: NotTestable(r##"uses Ref(L,+1), Ref(L,+2), Ref(H,+1), Ref(H,+2): reads future bars (look-ahead)"##), f: [Some(r##"If(L<Ref(L,-1),1,0) AND If(L<Ref(L,-2),1,0) AND If(L<Ref(L,+1),1,0) AND If(L<Ref(L,+2),1,0)"##), Some(r##"If(H>Ref(H,-1),1,0) AND If(H>Ref(H,-2),1,0) AND If(H>Ref(H,+1),1,0) AND If(H>Ref(H,+2),1,0)"##), None, None], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "mslib_maruja", name: r##"Estrategia maruja (buy and hold)"##, group: r##"Otras"##, family: ("mslib_misc", r##"Other strategies"##), page: "53-54", status: NotTestable(r##"buy on the first day and never sell: no exit rule"##), f: [None, None, None, None], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "mslib_mgr", name: r##"Máxima ganancia razonable - MGR (Angel Ibarra)"##, group: r##"Otras"##, family: ("mslib_misc", r##"Other strategies"##), page: "54-61", status: NotTestable(r##"uses Zig(C,10,%), a zig-zag revised by later bars (look-ahead); the author says it must never be used for trading (benchmark only)"##), f: [Some(r##"Zig(C,10,%)>Ref(Zig(C,10,%),-1) AND C>1.03*LLV(C,10)"##), Some(r##"Zig(C,10,%)<Ref(Zig(C,10,%),-1) AND C<1.03*HHV(C,10)"##), Some(r##"Zig(C,10,%)<Ref(Zig(C,10,%),-1) AND C<1.03*HHV(C,10)"##), Some(r##"Zig(C,10,%)>Ref(Zig(C,10,%),-1) AND C>1.03*LLV(C,10)"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "mslib_vila", name: r##"Método de Rafael Vila"##, group: r##"Otras"##, family: ("mslib_misc", r##"Other strategies"##), page: "61-62", status: NotTestable(r##"periods are given only as optimisation ranges (OPT1 2-6, OPT5 1-1.1, OPT10 1-1.08); the source states no single value"##), f: [Some(r##"C/Ref(C,-opt1)>opt10"##), Some(r##"Ref(C,-opt1)/C>opt5"##), Some(r##"Ref(C,-opt1)/C>opt5"##), Some(r##"C/Ref(C,-opt1)>opt10"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "mslib_manda_carayo", name: r##"Estrategia manda carayo (Miguel Pérez)"##, group: r##"Otras"##, family: ("mslib_carayo", r##"Manda carayo family"##), page: "62", status: Live, f: [Some(r##"13 + BarsSince(LLV(L,13)=L) - BarsSince(HHV(L,13)=H) > Ref(13 + BarsSince(LLV(L,13)=L) - BarsSince(HHV(L,13)=H), -1)"##), Some(r##"13 + BarsSince(LLV(L,13)=L) - BarsSince(HHV(L,13)=H) < Ref(13 + BarsSince(LLV(L,13)=L) - BarsSince(HHV(L,13)=H), -1)"##), None, None], exec: EXEC_DEFAULT, notes: r##"Printed HHV(L,13)=H (the 13-bar highest LOW equal to today's high) is run literally; it is rarely true, so BarsSince() stays undefined for long stretches. The optimised versions on p.63 print HHV(H,opt1)=H, but they carry no fixed value and are not substituted here."## },
    Sys { id: "mslib_carayooo", name: r##"Estrategia El carayooooo"##, group: r##"Otras"##, family: ("mslib_carayo", r##"Manda carayo family"##), page: "63", status: NotTestable(r##"periods are given only as optimisation ranges (OPT1 10-30); the source states no single value"##), f: [Some(r##"( opt1 + BarsSince(LLV(L,opt1)=L) - BarsSince(HHV(H,opt1)=H) ) > Ref(opt1 + BarsSince(LLV(L,opt1)=L) - BarsSince(HHV(H,opt1)=H), -1)"##), Some(r##"opt1 + BarsSince(LLV(L,opt1)=L) - BarsSince(HHV(H,opt1)=H) < Ref(opt1 + BarsSince(LLV(L,opt1)=L) - BarsSince(HHV(H,opt1)=H), -1)"##), None, None], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "mslib_carayo2", name: r##"Estrategia carayo-2 (Carlos Fierro)"##, group: r##"Otras"##, family: ("mslib_carayo", r##"Manda carayo family"##), page: "63-64", status: NotTestable(r##"periods are given only as optimisation ranges (OPT1 10-30); the source states no single value"##), f: [Some(r##"(BarsSince(LLV(L,opt1)=L)>BarsSince(HHV(H,opt1)=H))"##), Some(r##"(BarsSince(LLV(L,opt1)=L)<BarsSince(HHV(H,opt1)=H))"##), None, None], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "mslib_carayo3", name: r##"Estrategia carayo-3 (Carlos Fierro)"##, group: r##"Otras"##, family: ("mslib_carayo", r##"Manda carayo family"##), page: "64", status: NotTestable(r##"periods are given only as optimisation ranges (OPT1 10-30); the source states no single value"##), f: [Some(r##"BarsSince(LLV(C,opt1)=C)>BarsSince(HHV(C,opt1)=C)"##), Some(r##"BarsSince(LLV(C,opt1)=C)<BarsSince(HHV(C,opt1)=C)"##), None, None], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "mslib_carayo4", name: r##"Estrategia carayo-4 (Carlos Fierro)"##, group: r##"Otras"##, family: ("mslib_carayo", r##"Manda carayo family"##), page: "64-65", status: NotTestable(r##"periods are given only as optimisation ranges (OPT1 10-30); the source states no single value"##), f: [Some(r##"HHV(C,opt1)=C"##), Some(r##"LLV(C,opt1)=C"##), None, None], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "mslib_cci_ma", name: r##"Estrategia con medias móviles del CCI"##, group: r##"Otras"##, family: ("mslib_misc", r##"Other strategies"##), page: "65-66", status: NotTestable(r##"periods are given only as optimisation ranges (OPT1 5-40, OPT2 5-40); the source states no single value"##), f: [Some(r##"When(CCI(opt1),<,Mov(CCI(opt1),opt2,E)) AND When(Ref(CCI(opt1),-1),>=,Ref(Mov(CCI(opt1),opt2,E),-1))"##), Some(r##"When(CCI(opt1),>,Mov(CCI(opt1),opt2,E)) AND When(Ref(CCI(opt1),-1),<=,Ref(Mov(CCI(opt1),opt2,E),-1))"##), None, None], exec: EXEC_DEFAULT, notes: r##""## },
];

// Instrument admission: Libanes MetaStock strategies 2026-10-26 (lane B) block, moved from instrument_policy.rs on 31 Oct 2026 (lane ST).
// Admission only for the module REVIEWED lists (authors/vendor/laneb/SOURCE-CONTRACT.md); kept in one block to merge.
// Runs on actual indices, ETFs and stocks, US and India (index_etf_or_stock, nifty50 basket, declared application).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book=="mslib" && crate::books::mslib::reviewed(def.id),
    label: "MetaStock System Tester code with no per-market test the vault can compare; runs on indices, ETFs and single stocks (US and India) are a declared application, one unit per trade, fills as stated on the card.",
    basis: "Estrategias de inversion con Metastock (Libanes collection, 1998-99)",
};
