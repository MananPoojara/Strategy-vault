//! NIKSAN's MetaStock formula pack ("[sm] Metastock Plugin - NIKSAN's MS Formula All", research drive USB HDD 1/MetaStock/
//! @Systems & Indicators/@Misc/- Studies - No Sorted/; read-only). Its MS72PRFT.DTA holds 97 MetaStock 7.2 System Tester
//! records; this module (book id `nkms`, lane K1 30 Oct 2026) carries records 16-53 except 40 (PS Derivative MA =
//! mstt_the_derivative_ma): the Equis "PS" sample systems PS 10-Day Breakout ... PS Fractal Trading System 2. The other
//! records are lane K2's (`nkms2`). Record layout = MS65 (module msadd): name @1024, formulas @1065 + 2500*k; the text is
//! stored exactly as read (CR LF -> one space; authors/niksan/admission/extract.py).
//! Fills: MetaStock defaults (trade price close, trade delay 0); stops: none (every record carries the all-zero stop block
//! that msadd reads as "no stops"). Formulas run on the shared `msws` engine. Five built-ins the engine lacks are rebuilt
//! here for the systems that use them (PREV, ValueWhen, AroonUp/AroonDown, DayOfWeek): the parts of those formulas without
//! them are still evaluated by the engine from the printed text (the pieces are checked against the stored formulas in
//! the test); CMO() and ForecastOsc() are spelled out as formula text (`expand`). DMI() (Dynamic Momentum Index) is not
//! defined in the manuals: its three systems are listed, not run. Independent reference: authors/niksan/admission/.
use crate::books::msws::{self, Eng, Exec, Sig, Sys, EXEC_DEFAULT};
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
#[allow(unused_imports)]
use crate::connors::Status::{Live, NeedsData, NotTestable};
use std::sync::OnceLock;

pub const BOOK: &str = "nkms";
/// Admitted ids (authors/niksan/admission/nkms_ref.py (header)).
pub const REVIEWED: &[&str] = &["nkms_10_day_breakout", "nkms_2_day_range_switch", "nkms_3_day_range_switch", "nkms_4_day_range_switch", "nkms_adaptive_moving_average", "nkms_annual_strength_weakness_1", "nkms_annual_strength_weakness_2", "nkms_aroon_reversal", "nkms_aroon", "nkms_bull_power_bear_power_1", "nkms_bull_power_bear_power_2", "nkms_bull_power_bear_power_3", "nkms_bull_power_bear_power_4", "nkms_cci_commodity_channel_index", "nkms_cci_fibonacci", "nkms_cci_moving_average_crossover", "nkms_chande_forecast_oscillator", "nkms_cmo_reinforcement", "nkms_cmo_reversal", "nkms_congestion_breakout", "nkms_cooper_1234_pattern", "nkms_cycle_progression", "nkms_day_pattern_system", "nkms_detrended_ema_system", "nkms_directional_movement_sar_1", "nkms_directional_movement_sar_2", "nkms_division_system", "nkms_dns_trading_system", "nkms_elliott_oscillator_system_1", "nkms_elliott_oscillator_system_2", "nkms_exponential_moving_average", "nkms_fractal_trading_system_1", "nkms_fractal_trading_system_2"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }
#[allow(dead_code)]
fn _exec_type_check(e: Exec) -> Exec { e }
const NAN: f64 = f64::NAN;

// ------------------------------------------------------------------ text expansion (built-ins written as formula text)
fn split_args(s: &str) -> Vec<String> {
    let (mut out, mut cur, mut d) = (vec![], String::new(), 0i32);
    for c in s.chars() {
        match c { '(' => d += 1, ')' => d -= 1, ',' if d == 0 => { out.push(std::mem::take(&mut cur)); continue; } _ => {} }
        cur.push(c);
    }
    out.push(cur);
    out
}
/// CMO(x,n) = 100 x (up changes - down changes) / (up + down changes) over n bars (Chande, The New Technical Trader; the
/// MetaStock manuals describe it without a formula); ForecastOsc(x,n) = 100 x (x - yesterday's n-bar TSF) / x, the
/// vault's existing definition (module dgmf, "TSF Optimised Trading System"). Then mslib::expand (BBandTop/Bot).
pub fn expand(src: &str) -> String {
    let b: Vec<char> = src.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < b.len() {
        if b[i].is_ascii_alphabetic() && (i == 0 || !(b[i - 1].is_ascii_alphanumeric() || b[i - 1] == '_')) {
            let j0 = i;
            while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == '_') { i += 1; }
            let w: String = b[j0..i].iter().collect();
            let lw = w.to_ascii_lowercase();
            if i < b.len() && b[i] == '(' && (lw == "cmo" || lw == "forecastosc") {
                let (mut k, mut d) = (i, 0i32);
                while k < b.len() { match b[k] { '(' => d += 1, ')' => { d -= 1; if d == 0 { break; } } _ => {} } k += 1; }
                let a = split_args(&b[i + 1..k].iter().collect::<String>());
                if a.len() == 2 {
                    let (x, n) = (expand(&a[0]), a[1].trim().to_string());
                    out.push_str(&if lw == "cmo" {
                        format!("(100*(Sum(If(({x})>Ref(({x}),-1),({x})-Ref(({x}),-1),0),{n})-Sum(If(({x})<Ref(({x}),-1),Ref(({x}),-1)-({x}),0),{n}))/(Sum(If(({x})>Ref(({x}),-1),({x})-Ref(({x}),-1),0),{n})+Sum(If(({x})<Ref(({x}),-1),Ref(({x}),-1)-({x}),0),{n})))")
                    } else {
                        format!("(100*(({x})-Ref(TSF(({x}),{n}),-1))/({x}))")
                    });
                    i = k + 1;
                    continue;
                }
            }
            out.push_str(&w);
            continue;
        }
        out.push(b[i]);
        i += 1;
    }
    crate::books::mslib::expand(&out)
}

// ------------------------------------------------------------------ built-ins the shared engine lacks
fn fin(v: f64) -> bool { v.is_finite() }
fn tr(v: f64) -> bool { fin(v) && v != 0.0 }
fn bf(b: bool) -> f64 { b as u8 as f64 }
fn near(x: f64, y: f64) -> bool { (x - y).abs() <= 1e-12 * x.abs().max(y.abs()) }
/// Engine comparison semantics: undefined if either side is, values equal up to 1e-12 (relative) count as equal.
fn cmp(a: &[f64], b: &[f64], op: char) -> Vec<f64> {
    a.iter().zip(b).map(|(&x, &y)| if !fin(x) || !fin(y) { NAN } else { bf(match op { '<' => x < y && !near(x, y), '>' => x > y && !near(x, y), _ => near(x, y) }) }).collect()
}
fn k(n: usize, v: f64) -> Vec<f64> { vec![v; n] }
fn and(v: &[&[f64]]) -> Vec<f64> {
    (0..v[0].len()).map(|i| if v.iter().all(|s| fin(s[i])) { bf(v.iter().all(|s| s[i] != 0.0)) } else { NAN }).collect()
}
fn cross(p: &[f64], q: &[f64]) -> Vec<f64> {
    (0..p.len()).map(|i| if i >= 1 && fin(p[i]) && fin(q[i]) && fin(p[i - 1]) && fin(q[i - 1]) { bf(p[i] > q[i] && p[i - 1] <= q[i - 1]) } else { NAN }).collect()
}
/// ValueWhen(n, c, x): x on the bar of the n-th most recent bar where c was true (Pro 8 manual: "the value of the DATA
/// ARRAY when the EXPRESSION was true on the Nth most recent occurrence"); undefined until c has been true n times.
fn value_when(n: usize, c: &[f64], x: &[f64]) -> Vec<f64> {
    let mut hits: Vec<usize> = vec![];
    (0..c.len()).map(|i| { if tr(c[i]) { hits.push(i); } if fin(c[i]) && hits.len() >= n { x[hits[hits.len() - n]] } else { NAN } }).collect()
}
/// AroonUp/Down(n) = 100 x (n - bars since the highest high / lowest low of the last n+1 bars) / n, the latest bar on
/// ties (Pro 8 manual p.446: a new 14-day high gives 100; "not made a new high for 14 days" gives 0).
fn aroon(v: &[f64], n: usize, up: bool) -> Vec<f64> {
    (0..v.len()).map(|i| {
        if i < n { return NAN; }
        let w = &v[i - n..=i];
        let mut kk = 0;
        for j in 0..=n { if (up && w[j] >= w[kk]) || (!up && w[j] <= w[kk]) { kk = j; } }
        100.0 * kk as f64 / n as f64
    }).collect()
}
/// DayOfWeek(): 1 = Monday ... 7 = Sunday (MetaStock 7.2 manual), from the bar's date.
fn dow(date: &str) -> f64 {
    let p: Vec<i64> = date.split('-').map(|s| s.parse().unwrap_or(0)).collect();
    let (y, m, d) = (if p[1] <= 2 { p[0] - 1 } else { p[0] }, p[1], p[2]);
    let era = y.div_euclid(400); let yoe = y - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let days = era * 146097 + yoe * 365 + yoe / 4 - yoe / 100 + doy - 719468; // days since 1970-01-01 (a Thursday)
    ((days + 3).rem_euclid(7) + 1) as f64
}
/// PREV recursion: v(i) = f(i, previous defined value); 0 before the first defined value; an undefined bar keeps the
/// last defined value for the next bar.
fn prev_rec(n: usize, f: impl Fn(usize, f64) -> f64) -> Vec<f64> {
    let mut pv = 0.0;
    (0..n).map(|i| { let v = f(i, pv); if fin(v) { pv = v; v } else { NAN } }).collect()
}

/// Formula pieces evaluated by the shared engine (each one is text of the stored formula; see the test).
const P10: &str = "EL:=H>Ref(HHV(H,10),-1); CL:=Ref(H>Ref(HHV(H,10),-1),-1); ES:=L<Ref(LLV(L,10),-1); CS:=Ref(L<Ref(LLV(L,10),-1),-1);";
const RS: [[&str; 4]; 3] = [
    ["C>Ref(Max(H,Ref(H,-1)),-1)", "Min(L,Ref(L,-1))", "C<Ref(Min(L,Ref(L,-1)),-1)", "Max(H,Ref(H,-1))"],
    ["C>Ref(Max(H,Max(Ref(H,-1),Ref(H,-2))),-1)", "Min(L,Min(Ref(L,-1),Ref(L,-2)))", "C<Ref(Min(L,Min(Ref(L,-1),Ref(L,-2))),-1)", "Max(H,Max(Ref(H,-1),Ref(H,-2)))"],
    ["C>Ref(Max(Max(H,Ref(H,-3)),Max(Ref(H,-1),Ref(H,-2))),-1)", "Min(Min(L,Ref(L,-3)),Min(Ref(L,-1),Ref(L,-2)))", "C<Ref(Min(Min(L,Ref(L,-3)),Min(Ref(L,-1),Ref(L,-2))),-1)", "Max(Max(H,Ref(H,-3)),Max(Ref(H,-1),Ref(H,-2)))"],
];
const AMA_SC: &str = "Pwr((Abs((C-Ref(C,-4))/Sum(Abs(ROC(C,1,$)),4)))*((2/3)-(2/31))+(2/31),2)";
const BP: &str = "bear1:=(L-Mov(C,13,E))>Ref((L-Mov(C,13,E)),-1) AND Ref((L-Mov(C,13,E)),-1)<Ref((L-Mov(C,13,E)),-2); bull1:=(H-Mov(C,13,E))<Ref((H-Mov(C,13,E)),-1) AND Ref((H-Mov(C,13,E)),-1)>Ref((H-Mov(C,13,E)),-2);";
const BP_X: [&str; 6] = ["Ref((L-Mov(C,13,E)),-1)", "Ref((H-Mov(C,13,E)),-1)", "BarsSince(bear1)<BarsSince(bull1)", "BarsSince(bear1)>BarsSince(bull1)", "(L-Mov(C,13,E))<0", "(H-Mov(C,13,E))>0"];
const EO: &str = "var1:=Mov((H+L)/2,5,S)-Mov((H+L)/2,34,S); var2:=var1>Ref(var1,-1) AND Ref(var1,-1)<Ref(var1,-2); var3:=Mov((H+L)/2,5,S)-Mov((H+L)/2,34,S); var4:=var3<Ref(var3,-1) AND Ref(var3,-1)>Ref(var3,-2);";
const EO_X: [&str; 7] = ["Ref(var1,-1)", "Ref(var3,-1)", "BarsSince(var2)<BarsSince(var4)", "BarsSince(var4)<BarsSince(var2)", "var1<0", "var3>0", "Ref(C,-1)"];
const FR: [&str; 4] = ["H<Ref(H,-2) AND Ref(H,-1)<Ref(H,-2) AND Ref(H,-3)<Ref(H,-2) AND Ref(H,-4)<Ref(H,-2)", "L>Ref(L,-2) AND Ref(L,-1)>Ref(L,-2) AND Ref(L,-3)>Ref(L,-2) AND Ref(L,-4)>Ref(L,-2)", "Ref(H,-2)", "Ref(L,-2)"];
const DP: &str = "Ref(C,-1)>Ref(O,-5) AND Ref(C,-1)>Ref(O,-2)";

fn ev(e: &mut Eng, src: &str) -> Option<Vec<f64>> { e.eval_src(src).ok().map(|v| (*v).clone()) }
fn sig(v: [Vec<f64>; 4]) -> Sig {
    let [a, b, c, d] = v.map(|s| s.iter().map(|x| tr(*x)).collect::<Vec<bool>>());
    Sig { el: a, xl: b, es: c, xs: d }
}

/// Signals of the systems that use a built-in the shared engine lacks.
fn native(id: &str, x: &Ctx) -> Option<Sig> {
    let fl = |_: &str| -> Option<String> { None };
    let mut e = Eng::new(x, &fl);
    let n = x.c.len();
    let c = &x.c;
    Some(match id {
        "nkms_10_day_breakout" => {
            let v: Vec<Vec<f64>> = ["EL", "CL", "ES", "CS"].iter().map(|k| ev(&mut e, &format!("{P10} {k}"))).collect::<Option<_>>()?;
            // State:=If(Cum(1)=1,0,If(EL,1,If(ES,-1,If((CL AND PREV=1) OR (CS AND PREV=-1),0,PREV)))) - undefined while any input is
            let st = prev_rec(n, |i, pv| {
                if !(0..4).all(|j| fin(v[j][i])) { return NAN; }
                if i == 0 { 0.0 } else if tr(v[0][i]) { 1.0 } else if tr(v[2][i]) { -1.0 } else if (tr(v[1][i]) && pv == 1.0) || (tr(v[3][i]) && pv == -1.0) { 0.0 } else { pv }
            });
            let is = |a: f64, b: Option<f64>| -> Vec<f64> { st.iter().map(|&s| if fin(s) { bf(near(s, a) || b.is_some_and(|b| near(s, b))) } else { NAN }).collect() };
            sig([is(1.0, None), is(0.0, Some(-1.0)), is(-1.0, None), is(0.0, Some(1.0))])
        }
        "nkms_2_day_range_switch" | "nkms_3_day_range_switch" | "nkms_4_day_range_switch" => {
            let r = &RS[(id.as_bytes()[5] - b'2') as usize];
            let v: Vec<Vec<f64>> = r.iter().map(|s| ev(&mut e, s)).collect::<Option<_>>()?;
            // d:=If(up, lowest low, If(down, highest high, PREV))
            let d = prev_rec(n, |i, pv| if !(0..4).all(|j| fin(v[j][i])) { NAN } else if tr(v[0][i]) { v[1][i] } else if tr(v[2][i]) { v[3][i] } else { pv });
            let (up, dn) = (cross(c, &d), cross(&d, c));
            sig([up.clone(), dn.clone(), dn, up])
        }
        "nkms_adaptive_moving_average" => {
            let sc = ev(&mut e, AMA_SC)?;
            let five = ev(&mut e, "Cum(1)=5")?;
            // Sig:=If(Cum(1)=5, Ref(C,-1)+sc*(C-Ref(C,-1)), PREV+sc*(C-PREV))
            let s = prev_rec(n, |i, pv| if i == 0 || !fin(sc[i]) { NAN } else if tr(five[i]) { c[i - 1] + sc[i] * (c[i] - c[i - 1]) } else { pv + sc[i] * (c[i] - pv) });
            let s1: Vec<f64> = (0..n).map(|i| if i >= 1 { s[i - 1] } else { NAN }).collect();
            let (up, dn) = (cmp(&s, &s1, '>'), cmp(&s, &s1, '<'));
            sig([up.clone(), dn.clone(), dn, up])
        }
        "nkms_aroon_reversal" | "nkms_aroon" => {
            let (u, d) = (aroon(&x.h, 14, true), aroon(&x.l, 14, false));
            let (u100, u0, d100, d0) = (cmp(&u, &k(n, 100.0), '='), cmp(&u, &k(n, 0.0), '='), cmp(&d, &k(n, 100.0), '='), cmp(&d, &k(n, 0.0), '='));
            let (bull, bear) = (and(&[&u100, &d0]), and(&[&u0, &d100]));
            if id == "nkms_aroon" { sig([bull, u0, bear, d0]) } else { sig([bull.clone(), bear.clone(), bear, bull]) }
        }
        "nkms_bull_power_bear_power_3" | "nkms_bull_power_bear_power_4" => {
            let b1 = ev(&mut e, &format!("{BP} bear1"))?;
            let u1 = ev(&mut e, &format!("{BP} bull1"))?;
            let p: Vec<Vec<f64>> = BP_X.iter().map(|s| ev(&mut e, &format!("{BP} {s}"))).collect::<Option<_>>()?;
            let lo = cmp(&value_when(1, &b1, &p[0]), &value_when(2, &b1, &p[0]), '>');
            let hi = cmp(&value_when(1, &u1, &p[1]), &value_when(2, &u1, &p[1]), '<');
            let (l, s) = if id.ends_with('3') { (and(&[&lo, &p[2], &p[4]]), and(&[&hi, &p[3], &p[5]])) } else { (and(&[&lo, &p[2]]), and(&[&hi, &p[3]])) };
            sig([l.clone(), s.clone(), s, l])
        }
        "nkms_elliott_oscillator_system_1" | "nkms_elliott_oscillator_system_2" => {
            let v2 = ev(&mut e, &format!("{EO} var2"))?;
            let v4 = ev(&mut e, &format!("{EO} var4"))?;
            let p: Vec<Vec<f64>> = EO_X.iter().map(|s| ev(&mut e, &format!("{EO} {s}"))).collect::<Option<_>>()?;
            let z = k(n, 0.0);
            let trough_up = cmp(&value_when(1, &v2, &p[0]), &value_when(2, &v2, &p[0]), '>');
            let peak_dn = cmp(&value_when(1, &v4, &p[1]), &value_when(2, &v4, &p[1]), '<');
            let peak_neg = cmp(&value_when(1, &v4, &p[1]), &z, '<');
            let trough_pos = cmp(&value_when(1, &v2, &p[0]), &z, '>');
            let (l, s) = if id.ends_with('1') {
                (and(&[&trough_up, &p[2], &p[4], &peak_neg]), and(&[&peak_dn, &p[3], &p[5], &trough_pos]))
            } else {
                let cl = cmp(&value_when(1, &v2, &p[6]), &value_when(2, &v2, &p[6]), '<');
                let ch = cmp(&value_when(1, &v4, &p[6]), &value_when(2, &v4, &p[6]), '>');
                (and(&[&trough_up, &p[2], &cl, &p[4], &peak_neg]), and(&[&peak_dn, &p[3], &ch, &p[5], &trough_pos]))
            };
            sig([l, v4, s, v2])
        }
        "nkms_fractal_trading_system_1" | "nkms_fractal_trading_system_2" => {
            let p: Vec<Vec<f64>> = FR.iter().map(|s| ev(&mut e, s)).collect::<Option<_>>()?;
            let f1 = value_when(1, &p[0], &p[2]);
            let up = cross(&x.h, &f1);
            if id.ends_with('1') {
                let dn = cross(&value_when(1, &p[1], &p[3]), &x.l);
                sig([up.clone(), dn.clone(), dn, up])
            } else {
                sig([up, cross(&value_when(2, &p[1], &p[3]), &x.l), k(n, 0.0), k(n, 0.0)])
            }
        }
        "nkms_day_pattern_system" => {
            let d: Vec<f64> = x.r.iter().map(|r| dow(&r.date)).collect();
            let d1: Vec<f64> = (0..n).map(|i| if i >= 1 { d[i - 1] } else { NAN }).collect();
            let pc = ev(&mut e, DP)?;
            let el = and(&[&cmp(&d, &k(n, 1.0), '='), &cmp(&d1, &k(n, 5.0), '='), &pc]);
            let xl: Vec<f64> = d.iter().map(|&v| bf(near(v, 2.0) || near(v, 3.0))).collect();
            sig([el, xl, k(n, 0.0), k(n, 0.0)])
        }
        _ => return None,
    })
}
fn is_native(id: &str) -> bool {
    matches!(id, "nkms_10_day_breakout" | "nkms_2_day_range_switch" | "nkms_3_day_range_switch" | "nkms_4_day_range_switch" | "nkms_adaptive_moving_average"
        | "nkms_aroon_reversal" | "nkms_aroon" | "nkms_bull_power_bear_power_3" | "nkms_bull_power_bear_power_4" | "nkms_elliott_oscillator_system_1"
        | "nkms_elliott_oscillator_system_2" | "nkms_fractal_trading_system_1" | "nkms_fractal_trading_system_2" | "nkms_day_pattern_system")
}

// ------------------------------------------------------------------ catalogue / params / run (as msadd)
fn leak(s: String) -> &'static str { Box::leak(s.into_boxed_str()) }
fn expanded(s: &Sys) -> [Option<String>; 4] { [0, 1, 2, 3].map(|k| s.f[k].map(expand)) }
fn consts_of(s: &Sys) -> Vec<f64> { if is_native(s.id) { vec![] } else { let srcs: Vec<&str> = s.f.iter().flatten().copied().collect(); msws::constants(&srcs) } }
const CONV: &str = "Fills: MetaStock System Tester defaults (trade price close, trade delay 0; the record has no fill field). Stops: none (the record's stop block is the all-zero block that module msadd reads as no stops). One position at a time; a position closes only by its own Close formula (checked from the bar after entry); an exit plus an opposite entry on one bar reverse; no same-direction re-entry on the exit bar; the last position stays open, marked to the last close. PREV = the formula's own previous value (0 before its first value). Indicators: MetaStock definitions of the shared engine (module msws). Source markets not stated (Equis sample systems): indices, ETFs and stocks are a declared application.";

static CAT: OnceLock<Vec<(&'static [&'static str], &'static str, bool)>> = OnceLock::new();
pub fn catalogue() -> Vec<StrategyDef> {
    let cache = CAT.get_or_init(|| SYS.iter().map(|s| {
        let lab = ["E|Enter Long: ", "X|Close Long: ", "E|Enter Short: ", "X|Close Short: "];
        let rules: Vec<&'static str> = s.f.iter().zip(lab).filter_map(|(f, l)| f.map(|f| leak(format!("{l}{f}")))).collect();
        let fl = |_: &str| -> Option<String> { None };
        let live = s.status == Status::Live;
        let vol = live && expanded(s).iter().flatten().any(|f| msws::scan(f, &fl).0);
        let notes = if live { leak(format!("{} Source: {}. {}", s.notes, s.page, CONV)) } else { leak(format!("{} Source: {}.", s.notes, s.page)) };
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
    let mut v = vec![];
    if s.f[0].is_some() && s.f[2].is_some() { v.push(ParamDef { key: "side", label: "Sides traded: 0 = both as stored, 1 = long side only, -1 = short side only", def: 0., min: -1., max: 1., step: 1. }); }
    for c in consts_of(s) {
        let int = c.fract() == 0.0;
        let (min, max, step) = if int && c >= 5.0 { ((c * 0.5).round().max(1.0), (c * 2.0).round(), 1.0) } else { (c * 0.5, c * 2.0, (c / 10.0 * 1000.0).round() / 1000.0) };
        v.push(ParamDef { key: leak(msws::const_key(c)), label: leak(format!("Stored constant {c} (every use of {c} in this system's own formulas; Ref offsets excluded)")), def: c, min, max, step: if step > 0.0 { step } else { 0.001 } });
    }
    Some(v)
}
pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let s = SYS.iter().find(|s| s.id == id)?;
    if s.status != Status::Live || from >= to || to >= x.r.len() { return None; }
    let side = p.get("side").copied().unwrap_or(0.0).round() as i32;
    let mut sg = if is_native(id) {
        native(id, x)?
    } else {
        let map: Vec<(f64, f64)> = consts_of(s).into_iter().map(|c| (c, p.get(&msws::const_key(c)).copied().unwrap_or(c))).collect();
        let e = expanded(s);
        let srcs: [Option<&str>; 4] = [0, 1, 2, 3].map(|k| e[k].as_deref());
        let fl = |_: &str| -> Option<String> { None };
        let mut eng = Eng::new(x, &fl);
        msws::signals(&mut eng, srcs, &map, 0).ok()?
    };
    let n = x.c.len();
    if side > 0 { sg.es = vec![false; n]; sg.xs = vec![false; n]; }
    if side < 0 { sg.el = vec![false; n]; sg.xl = vec![false; n]; }
    Some(msws::simulate(x, from, to, &sg, s.exec))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn norm(s: &str) -> String { s.chars().filter(|c| !c.is_whitespace()).collect::<String>().to_ascii_lowercase() }
    #[test]
    fn pieces_are_stored_text_and_every_live_system_runs() {
        let all: String = SYS.iter().flat_map(|s| s.f.iter().flatten()).map(|f| norm(f)).collect::<Vec<_>>().join("|");
        let mut pieces: Vec<String> = vec![P10.into(), AMA_SC.into(), BP.into(), EO.into(), DP.into()];
        pieces.extend(RS.iter().flatten().map(|s| s.to_string()));
        pieces.extend(BP_X.iter().chain(EO_X.iter()).chain(FR.iter()).map(|s| s.to_string()));
        for p in pieces { assert!(all.contains(&norm(&p)), "{p}"); }
        assert_eq!(dow("1970-01-01"), 4.0); assert_eq!(dow("2026-10-30"), 5.0); assert_eq!(dow("2000-02-28"), 1.0);
        assert_eq!(aroon(&[1., 3., 2., 3., 1.], 4, true)[4], 75.0); // latest of the two 3s: 1 bar ago
        let vw = value_when(2, &[1., 0., 1., 1.], &[5., 6., 7., 8.]);
        assert!(vw[0].is_nan() && vw[1].is_nan() && vw[2] == 5. && vw[3] == 7.);
        let r: Vec<Row> = (0..700).map(|i| { let v = 100. + 20. * ((i as f64) / 9.).sin() + (i % 7) as f64; let d = 719163 + i * 7 / 5;
            Row { ticker: "SPX".into(), date: format!("{:04}-{:02}-{:02}", 1990 + i / 250, 1 + (i / 21) % 12, 1 + i % 28), open: v - 0.5, high: v + 1. + (d % 3) as f64, low: v - 1., close: v, volume: Some(1e6 + (i % 13) as f64 * 1e5) } }).collect();
        let x = Ctx::new(&r, &r, &[]);
        assert!(expand("CMO(C,13)>ForecastOsc(C,14)").contains("TSF((C),14)"));
        for s in SYS.iter().filter(|s| s.status == Status::Live) {
            assert!(run(s.id, &x, 300, 699, &Params::new()).is_some(), "{}", s.id);
        }
        assert_eq!(SYS.len(), 37);
    }
}

const SYS: &[Sys] = &[
    Sys { id: "nkms_10_day_breakout", name: r##"PS 10-Day Breakout"##, group: r##"Equis MetaStock 7.2 PS sample systems (NIKSAN pack)"##, family: ("nkms_ps", r##"Equis PS systems (NIKSAN pack)"##), page: r##"MS72PRFT.DTA record 16"##, status: Live, f: [Some(r##"EL:=H>Ref(HHV(H,10),-1); CL:=Ref(H>Ref(HHV(H,10),-1),-1); ES:=L<Ref(LLV(L,10),-1); CS:=Ref(L<Ref(LLV(L,10),-1),-1);  State:=If(Cum(1)=1,0,If(EL,1,If(ES,-1,If((CL AND PREV=1) OR (CS AND PREV=-1),0,PREV))));  State=1"##), Some(r##"EL:=H>Ref(HHV(H,10),-1); CL:=Ref(H>Ref(HHV(H,10),-1),-1); ES:=L<Ref(LLV(L,10),-1); CS:=Ref(L<Ref(LLV(L,10),-1),-1);  State:=If(Cum(1)=1,0,If(EL,1,If(ES,-1,If((CL AND PREV=1) OR (CS AND PREV=-1),0,PREV))));  State=0 OR State=-1"##), Some(r##"EL:=H>Ref(HHV(H,10),-1); CL:=Ref(H>Ref(HHV(H,10),-1),-1); ES:=L<Ref(LLV(L,10),-1); CS:=Ref(L<Ref(LLV(L,10),-1),-1);  State:=If(Cum(1)=1,0,If(EL,1,If(ES,-1,If((CL AND PREV=1) OR (CS AND PREV=-1),0,PREV))));  State=-1"##), Some(r##"EL:=H>Ref(HHV(H,10),-1); CL:=Ref(H>Ref(HHV(H,10),-1),-1); ES:=L<Ref(LLV(L,10),-1); CS:=Ref(L<Ref(LLV(L,10),-1),-1);  State:=If(Cum(1)=1,0,If(EL,1,If(ES,-1,If((CL AND PREV=1) OR (CS AND PREV=-1),0,PREV))));  State=0 OR State=1"##)], exec: EXEC_DEFAULT, notes: r##"Long when the high breaks above the highest high of the previous 10 days, short when the low breaks below the lowest low of the previous 10 days; the state returns to flat the day after a breakout unless a new breakout comes (self-referencing PREV state). PREV is evaluated by this module's own code (the shared engine has no PREV); every other part by the shared formula engine."## },
    Sys { id: "nkms_2_day_range_switch", name: r##"PS 2-Day-Range Switch"##, group: r##"Equis MetaStock 7.2 PS sample systems (NIKSAN pack)"##, family: ("nkms_ps", r##"Equis PS systems (NIKSAN pack)"##), page: r##"MS72PRFT.DTA record 17"##, status: Live, f: [Some(r##"d:=If(C>Ref(Max(H,Ref(H,-1)),-1),Min(L,Ref(L,-1)),If(C<Ref(Min(L,Ref(L,-1)),-1),Max(H,Ref(H,-1)),PREV));  Cross(C,d)"##), Some(r##"d:=If(C>Ref(Max(H,Ref(H,-1)),-1),Min(L,Ref(L,-1)),If(C<Ref(Min(L,Ref(L,-1)),-1),Max(H,Ref(H,-1)),PREV));  Cross(d,C)"##), Some(r##"d:=If(C>Ref(Max(H,Ref(H,-1)),-1),Min(L,Ref(L,-1)),If(C<Ref(Min(L,Ref(L,-1)),-1),Max(H,Ref(H,-1)),PREV));  Cross(d,C)"##), Some(r##"d:=If(C>Ref(Max(H,Ref(H,-1)),-1),Min(L,Ref(L,-1)),If(C<Ref(Min(L,Ref(L,-1)),-1),Max(H,Ref(H,-1)),PREV));  Cross(C,d)"##)], exec: EXEC_DEFAULT, notes: r##"A switch level flips to the 2-day low when the close breaks above the previous 2-day high and to the 2-day high when it breaks below the previous 2-day low (PREV); long on a close crossing above the level, short on a close crossing below. PREV is evaluated by this module's own code (the shared engine has no PREV); every other part by the shared formula engine."## },
    Sys { id: "nkms_3_day_range_switch", name: r##"PS 3-Day-Range Switch"##, group: r##"Equis MetaStock 7.2 PS sample systems (NIKSAN pack)"##, family: ("nkms_ps", r##"Equis PS systems (NIKSAN pack)"##), page: r##"MS72PRFT.DTA record 18"##, status: Live, f: [Some(r##"d:=If(C>Ref(Max(H,Max(Ref(H,-1),Ref(H,-2))),-1),Min(L,Min(Ref(L,-1),Ref(L,-2))),If(C<Ref(Min(L,Min(Ref(L,-1),Ref(L,-2))),-1),Max(H,Max(Ref(H,-1),Ref(H,-2))),PREV));  Cross(C,d)"##), Some(r##"d:=If(C>Ref(Max(H,Max(Ref(H,-1),Ref(H,-2))),-1),Min(L,Min(Ref(L,-1),Ref(L,-2))),If(C<Ref(Min(L,Min(Ref(L,-1),Ref(L,-2))),-1),Max(H,Max(Ref(H,-1),Ref(H,-2))),PREV));  Cross(d,C)"##), Some(r##"d:=If(C>Ref(Max(H,Max(Ref(H,-1),Ref(H,-2))),-1),Min(L,Min(Ref(L,-1),Ref(L,-2))),If(C<Ref(Min(L,Min(Ref(L,-1),Ref(L,-2))),-1),Max(H,Max(Ref(H,-1),Ref(H,-2))),PREV));  Cross(d,C)"##), Some(r##"d:=If(C>Ref(Max(H,Max(Ref(H,-1),Ref(H,-2))),-1),Min(L,Min(Ref(L,-1),Ref(L,-2))),If(C<Ref(Min(L,Min(Ref(L,-1),Ref(L,-2))),-1),Max(H,Max(Ref(H,-1),Ref(H,-2))),PREV));  Cross(C,d)"##)], exec: EXEC_DEFAULT, notes: r##"The 2-day range switch with 3-day highs and lows. PREV is evaluated by this module's own code (the shared engine has no PREV); every other part by the shared formula engine."## },
    Sys { id: "nkms_4_day_range_switch", name: r##"PS 4-Day-Range Switch"##, group: r##"Equis MetaStock 7.2 PS sample systems (NIKSAN pack)"##, family: ("nkms_ps", r##"Equis PS systems (NIKSAN pack)"##), page: r##"MS72PRFT.DTA record 19"##, status: Live, f: [Some(r##"d:=If(C>Ref(Max(Max(H,Ref(H,-3)),Max(Ref(H,-1),Ref(H,-2))),-1),Min(Min(L,Ref(L,-3)),Min(Ref(L,-1),Ref(L,-2))),If(C<Ref(Min(Min(L,Ref(L,-3)),Min(Ref(L,-1),Ref(L,-2))),-1),Max(Max(H,Ref(H,-3)),Max(Ref(H,-1),Ref(H,-2))),PREV));  Cross(C,d)"##), Some(r##"d:=If(C>Ref(Max(Max(H,Ref(H,-3)),Max(Ref(H,-1),Ref(H,-2))),-1),Min(Min(L,Ref(L,-3)),Min(Ref(L,-1),Ref(L,-2))),If(C<Ref(Min(Min(L,Ref(L,-3)),Min(Ref(L,-1),Ref(L,-2))),-1),Max(Max(H,Ref(H,-3)),Max(Ref(H,-1),Ref(H,-2))),PREV));  Cross(d,C)"##), Some(r##"d:=If(C>Ref(Max(Max(H,Ref(H,-3)),Max(Ref(H,-1),Ref(H,-2))),-1),Min(Min(L,Ref(L,-3)),Min(Ref(L,-1),Ref(L,-2))),If(C<Ref(Min(Min(L,Ref(L,-3)),Min(Ref(L,-1),Ref(L,-2))),-1),Max(Max(H,Ref(H,-3)),Max(Ref(H,-1),Ref(H,-2))),PREV));  Cross(d,C)"##), Some(r##"d:=If(C>Ref(Max(Max(H,Ref(H,-3)),Max(Ref(H,-1),Ref(H,-2))),-1),Min(Min(L,Ref(L,-3)),Min(Ref(L,-1),Ref(L,-2))),If(C<Ref(Min(Min(L,Ref(L,-3)),Min(Ref(L,-1),Ref(L,-2))),-1),Max(Max(H,Ref(H,-3)),Max(Ref(H,-1),Ref(H,-2))),PREV));  Cross(C,d)"##)], exec: EXEC_DEFAULT, notes: r##"The 2-day range switch with 4-day highs and lows. PREV is evaluated by this module's own code (the shared engine has no PREV); every other part by the shared formula engine."## },
    Sys { id: "nkms_adaptive_moving_average", name: r##"PS Adaptive Moving Average"##, group: r##"Equis MetaStock 7.2 PS sample systems (NIKSAN pack)"##, family: ("nkms_ps", r##"Equis PS systems (NIKSAN pack)"##), page: r##"MS72PRFT.DTA record 20"##, status: Live, f: [Some(r##"Sig:=If(Cum(1)=5,Ref(C,-1)+(Pwr((Abs((C-Ref(C,-4))/Sum(Abs(ROC(C,1,$)),4)))*((2/3)-(2/31))+(2/31),2))*(C-Ref(C,-1)),PREV+(Pwr((Abs((C-Ref(C,-4))/Sum(Abs(ROC(C,1,$)),4)))*((2/3)-(2/31))+(2/31),2))*(C-PREV));  Sig>Ref(Sig,-1)"##), Some(r##"Sig:=If(Cum(1)=5,Ref(C,-1)+(Pwr((Abs((C-Ref(C,-4))/Sum(Abs(ROC(C,1,$)),4)))*((2/3)-(2/31))+(2/31),2))*(C-Ref(C,-1)),PREV+(Pwr((Abs((C-Ref(C,-4))/Sum(Abs(ROC(C,1,$)),4)))*((2/3)-(2/31))+(2/31),2))*(C-PREV));  Sig<Ref(Sig,-1)"##), Some(r##"Sig:=If(Cum(1)=5,Ref(C,-1)+(Pwr((Abs((C-Ref(C,-4))/Sum(Abs(ROC(C,1,$)),4)))*((2/3)-(2/31))+(2/31),2))*(C-Ref(C,-1)),PREV+(Pwr((Abs((C-Ref(C,-4))/Sum(Abs(ROC(C,1,$)),4)))*((2/3)-(2/31))+(2/31),2))*(C-PREV));  Sig<Ref(Sig,-1)"##), Some(r##"Sig:=If(Cum(1)=5,Ref(C,-1)+(Pwr((Abs((C-Ref(C,-4))/Sum(Abs(ROC(C,1,$)),4)))*((2/3)-(2/31))+(2/31),2))*(C-Ref(C,-1)),PREV+(Pwr((Abs((C-Ref(C,-4))/Sum(Abs(ROC(C,1,$)),4)))*((2/3)-(2/31))+(2/31),2))*(C-PREV));  Sig>Ref(Sig,-1)"##)], exec: EXEC_DEFAULT, notes: r##"An adaptive average (efficiency ratio of the last 4 closes, squared smoothing constant between 2/31 and 2/3, built with PREV from the 5th bar); long while it rises, short while it falls. PREV is evaluated by this module's own code (the shared engine has no PREV); every other part by the shared formula engine."## },
    Sys { id: "nkms_annual_strength_weakness_1", name: r##"PS Annual Strength/Weakness 1"##, group: r##"Equis MetaStock 7.2 PS sample systems (NIKSAN pack)"##, family: ("nkms_ps", r##"Equis PS systems (NIKSAN pack)"##), page: r##"MS72PRFT.DTA record 21"##, status: Live, f: [Some(r##"var1:=((ROC(C,63,%))+(ROC(Ref(C,-63),63,%))+(ROC(Ref(C,-126),63,%))+(ROC(Ref(C,-189),63,%)));  var1>0"##), Some(r##"var1:=((ROC(C,63,%))+(ROC(Ref(C,-63),63,%))+(ROC(Ref(C,-126),63,%))+(ROC(Ref(C,-189),63,%)));  var1<0"##), Some(r##"var1:=((ROC(C,63,%))+(ROC(Ref(C,-63),63,%))+(ROC(Ref(C,-126),63,%))+(ROC(Ref(C,-189),63,%)));  var1<0"##), Some(r##"var1:=((ROC(C,63,%))+(ROC(Ref(C,-63),63,%))+(ROC(Ref(C,-126),63,%))+(ROC(Ref(C,-189),63,%)));  var1>0"##)], exec: EXEC_DEFAULT, notes: r##"Sum of the four quarterly (63-day) % changes of the last year: long above zero, short below."## },
    Sys { id: "nkms_annual_strength_weakness_2", name: r##"PS Annual Strength/Weakness 2"##, group: r##"Equis MetaStock 7.2 PS sample systems (NIKSAN pack)"##, family: ("nkms_ps", r##"Equis PS systems (NIKSAN pack)"##), page: r##"MS72PRFT.DTA record 22"##, status: Live, f: [Some(r##"var1:=((ROC(C,63,%))+(ROC(Ref(C,-63),63,%))+(ROC(Ref(C,-126),63,%))+(ROC(Ref(C,-189),63,%))); var2:=Sum((var1-LLV(var1,252)),5)/Sum((HHV(var1,252)-LLV(var1,252)),5);  var2>.5"##), Some(r##"var1:=((ROC(C,63,%))+(ROC(Ref(C,-63),63,%))+(ROC(Ref(C,-126),63,%))+(ROC(Ref(C,-189),63,%))); var2:=Sum((var1-LLV(var1,252)),5)/Sum((HHV(var1,252)-LLV(var1,252)),5);  var2<.5"##), Some(r##"var1:=((ROC(C,63,%))+(ROC(Ref(C,-63),63,%))+(ROC(Ref(C,-126),63,%))+(ROC(Ref(C,-189),63,%))); var2:=Sum((var1-LLV(var1,252)),5)/Sum((HHV(var1,252)-LLV(var1,252)),5);  var2<.5"##), Some(r##"var1:=((ROC(C,63,%))+(ROC(Ref(C,-63),63,%))+(ROC(Ref(C,-126),63,%))+(ROC(Ref(C,-189),63,%))); var2:=Sum((var1-LLV(var1,252)),5)/Sum((HHV(var1,252)-LLV(var1,252)),5);  var2>.5"##)], exec: EXEC_DEFAULT, notes: r##"The annual strength sum scaled into its 252-day range (5-day sums): long above 0.5, short below."## },
    Sys { id: "nkms_aroon_reversal", name: r##"PS Aroon Reversal"##, group: r##"Equis MetaStock 7.2 PS sample systems (NIKSAN pack)"##, family: ("nkms_ps", r##"Equis PS systems (NIKSAN pack)"##), page: r##"MS72PRFT.DTA record 23"##, status: Live, f: [Some(r##"AroonUp(14)=100 AND AroonDown(14)=0"##), Some(r##"AroonUp(14)=0 AND AroonDown(14)=100"##), Some(r##"AroonUp(14)=0 AND AroonDown(14)=100"##), Some(r##"AroonUp(14)=100 AND AroonDown(14)=0"##)], exec: EXEC_DEFAULT, notes: r##"Long when Aroon Up(14) = 100 and Aroon Down(14) = 0, short on the mirror; each side closes on the opposite condition. AroonUp/AroonDown is evaluated by this module's own code (the shared engine has no AroonUp/AroonDown); every other part by the shared formula engine."## },
    Sys { id: "nkms_aroon", name: r##"PS Aroon"##, group: r##"Equis MetaStock 7.2 PS sample systems (NIKSAN pack)"##, family: ("nkms_ps", r##"Equis PS systems (NIKSAN pack)"##), page: r##"MS72PRFT.DTA record 24"##, status: Live, f: [Some(r##"AroonUp(14)=100 AND  AroonDown(14)=0"##), Some(r##"AroonUp(14)=0"##), Some(r##"AroonUp(14)=0 AND  AroonDown(14)=100"##), Some(r##"AroonDown(14)=0"##)], exec: EXEC_DEFAULT, notes: r##"Long when Aroon Up(14) = 100 and Aroon Down(14) = 0, out when Aroon Up = 0; short on the mirror, out when Aroon Down = 0. AroonUp/AroonDown is evaluated by this module's own code (the shared engine has no AroonUp/AroonDown); every other part by the shared formula engine."## },
    Sys { id: "nkms_bull_power_bear_power_1", name: r##"PS Bull Power Bear Power 1"##, group: r##"Equis MetaStock 7.2 PS sample systems (NIKSAN pack)"##, family: ("nkms_ps", r##"Equis PS systems (NIKSAN pack)"##), page: r##"MS72PRFT.DTA record 25"##, status: Live, f: [Some(r##"(H-Mov(C,13,E))>0-(L-Mov(C,13,E))"##), Some(r##"(L-Mov(C,13,E))<0-(H-Mov(C,13,E))"##), Some(r##"(L-Mov(C,13,E))<0-(H-Mov(C,13,E))"##), Some(r##"(H-Mov(C,13,E))>0-(L-Mov(C,13,E))"##)], exec: EXEC_DEFAULT, notes: r##"Elder bull power (high minus 13-day EMA) greater than minus bear power (low minus the EMA): long; the mirror: short."## },
    Sys { id: "nkms_bull_power_bear_power_2", name: r##"PS Bull Power Bear Power 2"##, group: r##"Equis MetaStock 7.2 PS sample systems (NIKSAN pack)"##, family: ("nkms_ps", r##"Equis PS systems (NIKSAN pack)"##), page: r##"MS72PRFT.DTA record 26"##, status: Live, f: [Some(r##"var1:=LinRegSlope((L-Mov(C,13,E)),5)>Ref(LinRegSlope((L-Mov(C,13,E)),5),-1); var2:=LinRegSlope((H-Mov(C,13,E)),5)<Ref(LinRegSlope((H-Mov(C,13,E)),5),-1);  If(BarsSince(var1)<BarsSince(var2),1,0)=1"##), Some(r##"var1:=LinRegSlope((L-Mov(C,13,E)),5)>Ref(LinRegSlope((L-Mov(C,13,E)),5),-1); var2:=LinRegSlope((H-Mov(C,13,E)),5)<Ref(LinRegSlope((H-Mov(C,13,E)),5),-1);  If(BarsSince(var1)<BarsSince(var2),1,0)=0"##), Some(r##"var1:=LinRegSlope((L-Mov(C,13,E)),5)>Ref(LinRegSlope((L-Mov(C,13,E)),5),-1); var2:=LinRegSlope((H-Mov(C,13,E)),5)<Ref(LinRegSlope((H-Mov(C,13,E)),5),-1);  If(BarsSince(var1)<BarsSince(var2),1,0)=0"##), Some(r##"var1:=LinRegSlope((L-Mov(C,13,E)),5)>Ref(LinRegSlope((L-Mov(C,13,E)),5),-1); var2:=LinRegSlope((H-Mov(C,13,E)),5)<Ref(LinRegSlope((H-Mov(C,13,E)),5),-1);  If(BarsSince(var1)<BarsSince(var2),1,0)=1"##)], exec: EXEC_DEFAULT, notes: r##"Long while the 5-day regression slope of bear power last turned up more recently than the slope of bull power last turned down; short otherwise."## },
    Sys { id: "nkms_bull_power_bear_power_3", name: r##"PS Bull Power Bear Power 3"##, group: r##"Equis MetaStock 7.2 PS sample systems (NIKSAN pack)"##, family: ("nkms_ps", r##"Equis PS systems (NIKSAN pack)"##), page: r##"MS72PRFT.DTA record 27"##, status: Live, f: [Some(r##"bear1:=(L-Mov(C,13,E))>Ref((L-Mov(C,13,E)),-1) AND Ref((L-Mov(C,13,E)),-1)<Ref((L-Mov(C,13,E)),-2); bull1:=(H-Mov(C,13,E))<Ref((H-Mov(C,13,E)),-1) AND Ref((H-Mov(C,13,E)),-1)>Ref((H-Mov(C,13,E)),-2);  ValueWhen(1,bear1,Ref((L-Mov(C,13,E)),-1))>ValueWhen(2,bear1,Ref((L-Mov(C,13,E)),-1)) AND BarsSince(bear1)<BarsSince(bull1) AND (L-Mov(C,13,E))<0"##), Some(r##"bear1:=(L-Mov(C,13,E))>Ref((L-Mov(C,13,E)),-1) AND Ref((L-Mov(C,13,E)),-1)<Ref((L-Mov(C,13,E)),-2); bull1:=(H-Mov(C,13,E))<Ref((H-Mov(C,13,E)),-1) AND Ref((H-Mov(C,13,E)),-1)>Ref((H-Mov(C,13,E)),-2);  ValueWhen(1,bull1,Ref((H-Mov(C,13,E)),-1))<ValueWhen(2,bull1,Ref((H-Mov(C,13,E)),-1)) AND BarsSince(bear1)>BarsSince(bull1) AND (H-Mov(C,13,E))>0"##), Some(r##"bear1:=(L-Mov(C,13,E))>Ref((L-Mov(C,13,E)),-1) AND Ref((L-Mov(C,13,E)),-1)<Ref((L-Mov(C,13,E)),-2); bull1:=(H-Mov(C,13,E))<Ref((H-Mov(C,13,E)),-1) AND Ref((H-Mov(C,13,E)),-1)>Ref((H-Mov(C,13,E)),-2);  ValueWhen(1,bull1,Ref((H-Mov(C,13,E)),-1))<ValueWhen(2,bull1,Ref((H-Mov(C,13,E)),-1)) AND BarsSince(bear1)>BarsSince(bull1) AND (H-Mov(C,13,E))>0"##), Some(r##"bear1:=(L-Mov(C,13,E))>Ref((L-Mov(C,13,E)),-1) AND Ref((L-Mov(C,13,E)),-1)<Ref((L-Mov(C,13,E)),-2); bull1:=(H-Mov(C,13,E))<Ref((H-Mov(C,13,E)),-1) AND Ref((H-Mov(C,13,E)),-1)>Ref((H-Mov(C,13,E)),-2);  ValueWhen(1,bear1,Ref((L-Mov(C,13,E)),-1))>ValueWhen(2,bear1,Ref((L-Mov(C,13,E)),-1)) AND BarsSince(bear1)<BarsSince(bull1) AND (L-Mov(C,13,E))<0"##)], exec: EXEC_DEFAULT, notes: r##"Bear power trough higher than the previous trough (ValueWhen), troughs more recent than peaks and bear power below zero: long; the bull power mirror: short. ValueWhen is evaluated by this module's own code (the shared engine has no ValueWhen); every other part by the shared formula engine."## },
    Sys { id: "nkms_bull_power_bear_power_4", name: r##"PS Bull Power Bear Power 4"##, group: r##"Equis MetaStock 7.2 PS sample systems (NIKSAN pack)"##, family: ("nkms_ps", r##"Equis PS systems (NIKSAN pack)"##), page: r##"MS72PRFT.DTA record 28"##, status: Live, f: [Some(r##"bear1:=(L-Mov(C,13,E))>Ref((L-Mov(C,13,E)),-1) AND Ref((L-Mov(C,13,E)),-1)<Ref((L-Mov(C,13,E)),-2); bull1:=(H-Mov(C,13,E))<Ref((H-Mov(C,13,E)),-1) AND Ref((H-Mov(C,13,E)),-1)>Ref((H-Mov(C,13,E)),-2);   ValueWhen(1,bear1,Ref((L-Mov(C,13,E)),-1))>ValueWhen(2,bear1,Ref((L-Mov(C,13,E)),-1)) AND BarsSince(bear1)<BarsSince(bull1)"##), Some(r##"bear1:=(L-Mov(C,13,E))>Ref((L-Mov(C,13,E)),-1) AND Ref((L-Mov(C,13,E)),-1)<Ref((L-Mov(C,13,E)),-2); bull1:=(H-Mov(C,13,E))<Ref((H-Mov(C,13,E)),-1) AND Ref((H-Mov(C,13,E)),-1)>Ref((H-Mov(C,13,E)),-2);   ValueWhen(1,bull1,Ref((H-Mov(C,13,E)),-1))<ValueWhen(2,bull1,Ref((H-Mov(C,13,E)),-1)) AND BarsSince(bear1)>BarsSince(bull1)"##), Some(r##"bear1:=(L-Mov(C,13,E))>Ref((L-Mov(C,13,E)),-1) AND Ref((L-Mov(C,13,E)),-1)<Ref((L-Mov(C,13,E)),-2); bull1:=(H-Mov(C,13,E))<Ref((H-Mov(C,13,E)),-1) AND Ref((H-Mov(C,13,E)),-1)>Ref((H-Mov(C,13,E)),-2);   ValueWhen(1,bull1,Ref((H-Mov(C,13,E)),-1))<ValueWhen(2,bull1,Ref((H-Mov(C,13,E)),-1)) AND BarsSince(bear1)>BarsSince(bull1)"##), Some(r##"bear1:=(L-Mov(C,13,E))>Ref((L-Mov(C,13,E)),-1) AND Ref((L-Mov(C,13,E)),-1)<Ref((L-Mov(C,13,E)),-2); bull1:=(H-Mov(C,13,E))<Ref((H-Mov(C,13,E)),-1) AND Ref((H-Mov(C,13,E)),-1)>Ref((H-Mov(C,13,E)),-2);   ValueWhen(1,bear1,Ref((L-Mov(C,13,E)),-1))>ValueWhen(2,bear1,Ref((L-Mov(C,13,E)),-1)) AND BarsSince(bear1)<BarsSince(bull1)"##)], exec: EXEC_DEFAULT, notes: r##"Bull/bear power system 3 without the zero-line condition. ValueWhen is evaluated by this module's own code (the shared engine has no ValueWhen); every other part by the shared formula engine."## },
    Sys { id: "nkms_cci_commodity_channel_index", name: r##"PS CCI (Commodity Channel Index)"##, group: r##"Equis MetaStock 7.2 PS sample systems (NIKSAN pack)"##, family: ("nkms_ps", r##"Equis PS systems (NIKSAN pack)"##), page: r##"MS72PRFT.DTA record 29"##, status: Live, f: [Some(r##"CCI(14)<-200"##), Some(r##"CCI(14)>200"##), Some(r##"CCI(14)>200"##), Some(r##"CCI(14)<-200"##)], exec: EXEC_DEFAULT, notes: r##"Long when CCI(14) is below -200, short above +200; each side closes on the opposite extreme."## },
    Sys { id: "nkms_cci_fibonacci", name: r##"PS CCI Fibonacci"##, group: r##"Equis MetaStock 7.2 PS sample systems (NIKSAN pack)"##, family: ("nkms_ps", r##"Equis PS systems (NIKSAN pack)"##), page: r##"MS72PRFT.DTA record 30"##, status: Live, f: [Some(r##"Var1:=(CCI(8)+CCI(13)+CCI(21))/3;  Var1<-200"##), Some(r##"Var1:=(CCI(8)+CCI(13)+CCI(21))/3;  Var1>0"##), Some(r##"Var1:=(CCI(8)+CCI(13)+CCI(21))/3;  Var1>200"##), Some(r##"Var1:=(CCI(8)+CCI(13)+CCI(21))/3;  Var1<0"##)], exec: EXEC_DEFAULT, notes: r##"Average of CCI(8), CCI(13) and CCI(21): long below -200, out above 0; short above +200, out below 0."## },
    Sys { id: "nkms_cci_moving_average_crossover", name: r##"PS CCI Moving Average Crossover"##, group: r##"Equis MetaStock 7.2 PS sample systems (NIKSAN pack)"##, family: ("nkms_ps", r##"Equis PS systems (NIKSAN pack)"##), page: r##"MS72PRFT.DTA record 31"##, status: Live, f: [Some(r##"var1:=Sum(CCI(34),13)/Sum(HHV(CCI(34),200)-LLV(CCI(34),200),13);  Cross(var1,Mov(var1,9,E))"##), Some(r##"var1:=Sum(CCI(34),13)/Sum(HHV(CCI(34),200)-LLV(CCI(34),200),13);  Cross(Mov(var1,9,E),var1)"##), Some(r##"var1:=Sum(CCI(34),13)/Sum(HHV(CCI(34),200)-LLV(CCI(34),200),13);  Cross(Mov(var1,9,E),var1)"##), Some(r##"var1:=Sum(CCI(34),13)/Sum(HHV(CCI(34),200)-LLV(CCI(34),200),13);  Cross(var1,Mov(var1,9,E))"##)], exec: EXEC_DEFAULT, notes: r##"13-day sum of CCI(34) over the 13-day sum of its 200-day range, crossing its 9-day EMA: long on a cross up, short on a cross down."## },
    Sys { id: "nkms_chande_forecast_oscillator", name: r##"PS Chande Forecast Oscillator"##, group: r##"Equis MetaStock 7.2 PS sample systems (NIKSAN pack)"##, family: ("nkms_ps", r##"Equis PS systems (NIKSAN pack)"##), page: r##"MS72PRFT.DTA record 32"##, status: Live, f: [Some(r##"ForecastOsc(C,14)<0 AND Cross(ForecastOsc(C,14),Mov(ForecastOsc(C,14),3,S))"##), Some(r##"ForecastOsc(C,14)>0 AND Cross(Mov(ForecastOsc(C,14),3,S),ForecastOsc(C,14))"##), Some(r##"ForecastOsc(C,14)>0 AND Cross(Mov(ForecastOsc(C,14),3,S),ForecastOsc(C,14))"##), Some(r##"ForecastOsc(C,14)<0 AND Cross(ForecastOsc(C,14),Mov(ForecastOsc(C,14),3,S))"##)], exec: EXEC_DEFAULT, notes: r##"Forecast Oscillator(14) below zero crossing above its 3-day average: long; above zero crossing below it: short."## },
    Sys { id: "nkms_cmo_reinforcement", name: r##"PS CMO Reinforcement"##, group: r##"Equis MetaStock 7.2 PS sample systems (NIKSAN pack)"##, family: ("nkms_ps", r##"Equis PS systems (NIKSAN pack)"##), page: r##"MS72PRFT.DTA record 33"##, status: Live, f: [Some(r##"CMO(C,13)>Ref(CMO(C,13),-1) AND Ref(CMO(C,13),-1)<Ref(CMO(C,13),-2) AND CMO(C,34)>Ref(CMO(C,34),-1) AND Ref(CMO(C,34),-1)>Ref(CMO(C,34),-2)"##), Some(r##"CMO(C,13)<Ref(CMO(C,13),-1)"##), Some(r##"CMO(C,13)<Ref(CMO(C,13),-1) AND Ref(CMO(C,13),-1)>Ref(CMO(C,13),-2) AND CMO(C,34)<Ref(CMO(C,34),-1) AND Ref(CMO(C,34),-1)<Ref(CMO(C,34),-2)"##), Some(r##"CMO(C,13)>Ref(CMO(C,13),-1)"##)], exec: EXEC_DEFAULT, notes: r##"CMO(13) turning up while CMO(34) rises two days: long, out when CMO(13) falls; the mirror short."## },
    Sys { id: "nkms_cmo_reversal", name: r##"PS CMO Reversal"##, group: r##"Equis MetaStock 7.2 PS sample systems (NIKSAN pack)"##, family: ("nkms_ps", r##"Equis PS systems (NIKSAN pack)"##), page: r##"MS72PRFT.DTA record 34"##, status: Live, f: [Some(r##"CMO(C,13)>Ref(CMO(C,13),-1) AND Ref(CMO(C,13),-1)<Ref(CMO(C,13),-2)"##), Some(r##"CMO(C,13)<Ref(CMO(C,13),-1) AND Ref(CMO(C,13),-1)>Ref(CMO(C,13),-2)"##), Some(r##"CMO(C,13)<Ref(CMO(C,13),-1) AND Ref(CMO(C,13),-1)>Ref(CMO(C,13),-2)"##), Some(r##"CMO(C,13)>Ref(CMO(C,13),-1) AND Ref(CMO(C,13),-1)<Ref(CMO(C,13),-2)"##)], exec: EXEC_DEFAULT, notes: r##"CMO(13) turning up from a one-day dip: long; turning down from a one-day peak: short."## },
    Sys { id: "nkms_congestion_breakout", name: r##"PS Congestion Breakout"##, group: r##"Equis MetaStock 7.2 PS sample systems (NIKSAN pack)"##, family: ("nkms_ps", r##"Equis PS systems (NIKSAN pack)"##), page: r##"MS72PRFT.DTA record 35"##, status: Live, f: [Some(r##"cg:=((HHV(C,63)-LLV(C,63))/LLV(C,63))*100;  Ref(cg,-5)<10 AND cg>=10 AND C>Ref(HHV(C,63),-5) AND Mov(V,5,S)>=1.5*(Ref(Mov(V,20,S),-5))"##), Some(r##"((HHV(C,63)-LLV(C,63))/LLV(C,63))*100<=10"##), Some(r##"cg:=((HHV(C,63)-LLV(C,63))/LLV(C,63))*100;  Ref(cg,-5)<10 AND cg>=10 AND C<Ref(LLV(C,63),-5) AND Mov(V,5,S)>=1.5*(Ref(Mov(V,20,S),-5))"##), Some(r##"((HHV(C,63)-LLV(C,63))/LLV(C,63))*100<=10"##)], exec: EXEC_DEFAULT, notes: r##"After a 63-day close range under 10% five days ago, the range widens to 10% or more with a close beyond the range of 5 days ago and the 5-day volume at least 1.5 times the earlier 20-day volume: long/short; out when the 63-day range is back to 10% or less."## },
    Sys { id: "nkms_consolidation_breakout", name: r##"PS Consolidation Breakout"##, group: r##"Equis MetaStock 7.2 PS sample systems (NIKSAN pack)"##, family: ("nkms_ps", r##"Equis PS systems (NIKSAN pack)"##), page: r##"MS72PRFT.DTA record 36"##, status: Live, f: [Some(r##"var1:=If(((BBandTop(C,20,S,2)-BBandBot(C,20,S,2))<(1.75*ATR(1))) AND Cross(H,BBandTop(C,20,S,2)),1,If(ATR(1) AND Cross(BBandBot(C,20,S,2),L),-1,0));  var1=1"##), Some(r##"Cross(Mov(C,20,S),L)"##), Some(r##"var1:=If(((BBandTop(C,20,S,2)-BBandBot(C,20,S,2))<(1.75*ATR(1))) AND Cross(H,BBandTop(C,20,S,2)),1,If(ATR(1) AND Cross(BBandBot(C,20,S,2),L),-1,0));  var1=-1"##), Some(r##"Cross(H,Mov(C,20,S))"##)], exec: EXEC_DEFAULT, notes: r##"Bollinger band width (20, 2) under 1.75 true ranges with the high crossing above the upper band: long, out when the low crosses below the 20-day average; short when the low crosses below the lower band, out when the high crosses above the average."## },
    Sys { id: "nkms_cooper_1234_pattern", name: r##"PS Cooper 1234 Pattern"##, group: r##"Equis MetaStock 7.2 PS sample systems (NIKSAN pack)"##, family: ("nkms_ps", r##"Equis PS systems (NIKSAN pack)"##), page: r##"MS72PRFT.DTA record 37"##, status: Live, f: [Some(r##"var1:=(ADX(14)>30) AND (PDI(14)>MDI(14)) AND ((L<Ref(L,-1) AND Ref(L,-1)<Ref(L,-2) AND Ref(L,-2)<Ref(L,-3)) OR ((L<Ref(L,-1) AND Ref(L,-1)<Ref(L,-2) AND Ref(H,-2)<Ref(H,-3) AND Ref(L,-2)>Ref(L,-3)) OR (L<Ref(L,-1) AND  Ref(L,-2)<Ref(L,-3) AND Ref(H,-1)<Ref(H,-2) AND  Ref(L,-1)>Ref(L,-2)) OR (Ref(L,-1)<Ref(L,-2) AND Ref(L,-2)<Ref(L,-3) AND H<Ref(H,-1) AND  L>Ref(L,-1)))); var2:=Cross(Ref(L,-1),L);  If(BarsSince(var1)<BarsSince(var2),1,0)=1"##), Some(r##"var1:=(ADX(14)>30) AND (PDI(14)>MDI(14)) AND ((L<Ref(L,-1) AND Ref(L,-1)<Ref(L,-2) AND Ref(L,-2)<Ref(L,-3)) OR ((L<Ref(L,-1) AND Ref(L,-1)<Ref(L,-2) AND Ref(H,-2)<Ref(H,-3) AND Ref(L,-2)>Ref(L,-3)) OR (L<Ref(L,-1) AND  Ref(L,-2)<Ref(L,-3) AND Ref(H,-1)<Ref(H,-2) AND  Ref(L,-1)>Ref(L,-2)) OR (Ref(L,-1)<Ref(L,-2) AND Ref(L,-2)<Ref(L,-3) AND H<Ref(H,-1) AND  L>Ref(L,-1)))); var2:=Cross(Ref(L,-1),L);  If(BarsSince(var1)<BarsSince(var2),1,0)=0"##), Some(r##"var3:=(ADX(14)>30) AND (PDI(14)<MDI(14)) AND ((H>Ref(H,-1) AND Ref(H,-1)>Ref(H,-2) AND Ref(H,-2)>Ref(H,-3)) OR ((H>Ref(H,-1) AND Ref(H,-1)>Ref(H,-2) AND Ref(H,-2)<Ref(H,-3) AND Ref(L,-2)>Ref(L,-3)) OR (H>Ref(H,-1) AND  Ref(H,-2)>Ref(H,-3) AND Ref(H,-1)<Ref(H,-2) AND  Ref(L,-1)>Ref(L,-2)) OR (Ref(H,-1)>Ref(H,-2) AND Ref(H,-2)>Ref(H,-3) AND H<Ref(H,-1) AND  L>Ref(L,-1)))); var4:=Cross(H,Ref(H,-1));  If(BarsSince(var3)<BarsSince(var4),1,0)=1"##), Some(r##"var3:=(ADX(14)>30) AND (PDI(14)<MDI(14)) AND ((H>Ref(H,-1) AND Ref(H,-1)>Ref(H,-2) AND Ref(H,-2)>Ref(H,-3)) OR ((H>Ref(H,-1) AND Ref(H,-1)>Ref(H,-2) AND Ref(H,-2)<Ref(H,-3) AND Ref(L,-2)>Ref(L,-3)) OR (H>Ref(H,-1) AND  Ref(H,-2)>Ref(H,-3) AND Ref(H,-1)<Ref(H,-2) AND  Ref(L,-1)>Ref(L,-2)) OR (Ref(H,-1)>Ref(H,-2) AND Ref(H,-2)>Ref(H,-3) AND H<Ref(H,-1) AND  L>Ref(L,-1)))); var4:=Cross(H,Ref(H,-1));  If(BarsSince(var3)<BarsSince(var4),1,0)=0"##)], exec: EXEC_DEFAULT, notes: r##"Jeff Cooper 1-2-3-4 pullback set-ups with ADX(14) > 30 in the trend direction: long while the latest set-up is more recent than the latest new low (short mirror with highs)."## },
    Sys { id: "nkms_cycle_progression", name: r##"PS Cycle Progression"##, group: r##"Equis MetaStock 7.2 PS sample systems (NIKSAN pack)"##, family: ("nkms_ps", r##"Equis PS systems (NIKSAN pack)"##), page: r##"MS72PRFT.DTA record 38"##, status: Live, f: [Some(r##"Sum(If(C>Ref(C,-1),1,0),5)<Sum(If(C<Ref(C,-1),1,0),5)"##), Some(r##"Sum(If(C>Ref(C,-1),1,0),5)>Sum(If(C<Ref(C,-1),1,0),5)"##), Some(r##"Sum(If(C>Ref(C,-1),1,0),5)>Sum(If(C<Ref(C,-1),1,0),5)"##), Some(r##"Sum(If(C>Ref(C,-1),1,0),5)<Sum(If(C<Ref(C,-1),1,0),5)"##)], exec: EXEC_DEFAULT, notes: r##"Long when the last 5 days have more down closes than up closes, short when more up closes."## },
    Sys { id: "nkms_day_pattern_system", name: r##"PS Day Pattern System"##, group: r##"Equis MetaStock 7.2 PS sample systems (NIKSAN pack)"##, family: ("nkms_ps", r##"Equis PS systems (NIKSAN pack)"##), page: r##"MS72PRFT.DTA record 39"##, status: Live, f: [Some(r##"DayOfWeek()=1 AND Ref(DayOfWeek(),-1)=5 AND Ref(C,-1)>Ref(O,-5) AND Ref(C,-1)>Ref(O,-2)"##), Some(r##"DayOfWeek()=2 OR DayOfWeek()=3"##), None, None], exec: EXEC_DEFAULT, notes: r##"Long at Monday's close after a Friday when Friday's close is above the open of 5 and of 2 bars earlier; out at the close of Tuesday or Wednesday. Long only. DayOfWeek is evaluated by this module's own code (the shared engine has no DayOfWeek); every other part by the shared formula engine."## },
    Sys { id: "nkms_detrended_ema_system", name: r##"PS Detrended EMA System"##, group: r##"Equis MetaStock 7.2 PS sample systems (NIKSAN pack)"##, family: ("nkms_ps", r##"Equis PS systems (NIKSAN pack)"##), page: r##"MS72PRFT.DTA record 41"##, status: Live, f: [Some(r##"Var1:= Mov(CLOSE,55,E); Var2:= Mov(var1,55,E); Var3:= (var1-var2)+var1; Var1:= Mov(var3,55,E); Var2:= Mov(var1,55,E); Var4:= (var1-var2)+var1; Var5:=var3/var4;  Var3>var4 AND var5>1.05"##), Some(r##"Var1:= Mov(CLOSE,55,E); Var2:= Mov(var1,55,E); Var3:= (var1-var2)+var1; Var1:= Mov(var3,55,E); Var2:= Mov(var1,55,E); Var4:= (var1-var2)+var1; Var5:=var3/var4;  Var3<var4 AND var5<.95"##), Some(r##"Var1:= Mov(CLOSE,55,E); Var2:= Mov(var1,55,E); Var3:= (var1-var2)+var1; Var1:= Mov(var3,55,E); Var2:= Mov(var1,55,E); Var4:= (var1-var2)+var1; Var5:=var3/var4;  Var3<var4 AND var5<.95"##), Some(r##"Var1:= Mov(CLOSE,55,E); Var2:= Mov(var1,55,E); Var3:= (var1-var2)+var1; Var1:= Mov(var3,55,E); Var2:= Mov(var1,55,E); Var4:= (var1-var2)+var1; Var5:=var3/var4;  Var3>var4 AND var5>1.05"##)], exec: EXEC_DEFAULT, notes: r##"Two 55-day double-smoothed EMAs (DEMA of the close and DEMA of that): long when the first is more than 5% above the second, short when more than 5% below."## },
    Sys { id: "nkms_directional_movement_sar_1", name: r##"PS Directional Movement/SAR 1"##, group: r##"Equis MetaStock 7.2 PS sample systems (NIKSAN pack)"##, family: ("nkms_ps", r##"Equis PS systems (NIKSAN pack)"##), page: r##"MS72PRFT.DTA record 42"##, status: Live, f: [Some(r##"Cross(ADX(18),15) AND PDI(18)>MDI(18)"##), Some(r##"C<SAR(.02,.2)"##), Some(r##"Cross(ADX(18),15) AND PDI(18)<MDI(18)"##), Some(r##"C>SAR(.02,.2)"##)], exec: EXEC_DEFAULT, notes: r##"ADX(18) crossing above 15 with +DI above -DI: long, out on a close below the parabolic SAR (0.02/0.2); the mirror short."## },
    Sys { id: "nkms_directional_movement_sar_2", name: r##"PS Directional Movement/SAR 2"##, group: r##"Equis MetaStock 7.2 PS sample systems (NIKSAN pack)"##, family: ("nkms_ps", r##"Equis PS systems (NIKSAN pack)"##), page: r##"MS72PRFT.DTA record 43"##, status: Live, f: [Some(r##"ADX(18)>Ref(ADX(18),-1) AND PDI(18)>MDI(18) AND C>SAR(.02,.2)"##), Some(r##"C<SAR(.02,.2)"##), Some(r##"ADX(18)>Ref(ADX(18),-1) AND PDI(18)<MDI(18) AND C<SAR(.02,.2)"##), Some(r##"C>SAR(.02,.2)"##)], exec: EXEC_DEFAULT, notes: r##"ADX(18) rising with +DI above -DI and the close above the SAR: long, out on a close below the SAR; the mirror short."## },
    Sys { id: "nkms_division_system", name: r##"PS Division System"##, group: r##"Equis MetaStock 7.2 PS sample systems (NIKSAN pack)"##, family: ("nkms_ps", r##"Equis PS systems (NIKSAN pack)"##), page: r##"MS72PRFT.DTA record 44"##, status: Live, f: [Some(r##"var1:=LinRegSlope(C,63)/C;  var1>Mov(var1,13,S)"##), Some(r##"var1:=LinRegSlope(C,63)/C;  var1<Mov(var1,13,S)"##), Some(r##"var1:=LinRegSlope(C,63)/C;  var1<Mov(var1,13,S)"##), Some(r##"var1:=LinRegSlope(C,63)/C;  var1>Mov(var1,13,S)"##)], exec: EXEC_DEFAULT, notes: r##"63-day regression slope divided by the close, against its 13-day average: long above, short below."## },
    Sys { id: "nkms_dns_trading_system", name: r##"PS DNS Trading System"##, group: r##"Equis MetaStock 7.2 PS sample systems (NIKSAN pack)"##, family: ("nkms_ps", r##"Equis PS systems (NIKSAN pack)"##), page: r##"MS72PRFT.DTA record 45"##, status: Live, f: [Some(r##"var1:=If(SAR(.02,.2)<C,1,0)+ If((Mov(C,5,E)>Mov(C,13,E)),1,0)+ If((Mov(C,13,E)>Mov(C,40,E)),1,0)+ If((Mov(C,8,E)-Mov(C,17,E))>(Mov((Mov(C,8,E)-Mov(C,17,E)),9,E)),1,0)+ If((Mov(C,50,S)-Ref(Mov(C,50,S),-15)>0),1,0)+ If((Mov(ROC(C,12,%),3,E)>=-6 OR ROC(C,12,%)>0),1,0)+ If(OBV()>Mov(OBV(),40,S),1,0)+ If((V>Mov(V,120,S)),1,0);  var1=0"##), Some(r##"var1:=If(SAR(.02,.2)<C,1,0)+ If((Mov(C,5,E)>Mov(C,13,E)),1,0)+ If((Mov(C,13,E)>Mov(C,40,E)),1,0)+ If((Mov(C,8,E)-Mov(C,17,E))>(Mov((Mov(C,8,E)-Mov(C,17,E)),9,E)),1,0)+ If((Mov(C,50,S)-Ref(Mov(C,50,S),-15)>0),1,0)+ If((Mov(ROC(C,12,%),3,E)>=-6 OR ROC(C,12,%)>0),1,0)+ If(OBV()>Mov(OBV(),40,S),1,0)+ If((V>Mov(V,120,S)),1,0);  var1=8"##), Some(r##"var1:=If(SAR(.02,.2)<C,1,0)+ If((Mov(C,5,E)>Mov(C,13,E)),1,0)+ If((Mov(C,13,E)>Mov(C,40,E)),1,0)+ If((Mov(C,8,E)-Mov(C,17,E))>(Mov((Mov(C,8,E)-Mov(C,17,E)),9,E)),1,0)+ If((Mov(C,50,S)-Ref(Mov(C,50,S),-15)>0),1,0)+ If((Mov(ROC(C,12,%),3,E)>=-6 OR ROC(C,12,%)>0),1,0)+ If(OBV()>Mov(OBV(),40,S),1,0)+ If((V>Mov(V,120,S)),1,0);  var1=8"##), Some(r##"var1:=If(SAR(.02,.2)<C,1,0)+ If((Mov(C,5,E)>Mov(C,13,E)),1,0)+ If((Mov(C,13,E)>Mov(C,40,E)),1,0)+ If((Mov(C,8,E)-Mov(C,17,E))>(Mov((Mov(C,8,E)-Mov(C,17,E)),9,E)),1,0)+ If((Mov(C,50,S)-Ref(Mov(C,50,S),-15)>0),1,0)+ If((Mov(ROC(C,12,%),3,E)>=-6 OR ROC(C,12,%)>0),1,0)+ If(OBV()>Mov(OBV(),40,S),1,0)+ If((V>Mov(V,120,S)),1,0);  var1=0"##)], exec: EXEC_DEFAULT, notes: r##"Eight trend/volume tests (SAR, EMA 5/13/40, MACD 8/17/9, 50-day SMA slope, 12-day ROC, OBV, volume) summed: long when none is positive, out when all eight are; short when all eight are, out when none is (as stored)."## },
    Sys { id: "nkms_dynamic_momentum_index_1", name: r##"PS Dynamic Momentum Index 1"##, group: r##"Equis MetaStock 7.2 PS sample systems (NIKSAN pack)"##, family: ("nkms_ps", r##"Equis PS systems (NIKSAN pack)"##), page: r##"MS72PRFT.DTA record 46"##, status: NotTestable(r##"uses DMI(), MetaStock's built-in Dynamic Momentum Index (Chande/Kroll): the manuals give only its bounds (3 to 30 periods) and that it is driven by a 5-period standard deviation against its 10-period average, not the formula, so it cannot be rebuilt exactly; not coded"##), f: [Some(r##"Cross(30,DMI(C))"##), Some(r##"Cross(Mov(C,5,S),C) OR (BarsSince(Cross(30,DMI(C)))=3 AND ROC(C,3,%)<0)"##), Some(r##"Cross(DMI(C),70)"##), Some(r##"Cross(C,Mov(C,5,S)) OR (BarsSince(Cross(DMI(C),70))=3 AND ROC(C,3,%)>0)"##)], exec: EXEC_DEFAULT, notes: r##"Dynamic Momentum Index crossing 30 / 70 with a 5-day average exit."## },
    Sys { id: "nkms_dynamic_momentum_index_2", name: r##"PS Dynamic Momentum Index 2"##, group: r##"Equis MetaStock 7.2 PS sample systems (NIKSAN pack)"##, family: ("nkms_ps", r##"Equis PS systems (NIKSAN pack)"##), page: r##"MS72PRFT.DTA record 47"##, status: NotTestable(r##"uses DMI(), MetaStock's built-in Dynamic Momentum Index (Chande/Kroll): the manuals give only its bounds (3 to 30 periods) and that it is driven by a 5-period standard deviation against its 10-period average, not the formula, so it cannot be rebuilt exactly; not coded"##), f: [Some(r##"Cross(DMI(C),20)"##), Some(r##"Cross(80,DMI(C))"##), Some(r##"Cross(80,DMI(C))"##), Some(r##"Cross(DMI(C),20)"##)], exec: EXEC_DEFAULT, notes: r##"Dynamic Momentum Index crossing 20 / 80."## },
    Sys { id: "nkms_dynamic_momentum_index_3", name: r##"PS Dynamic Momentum Index 3"##, group: r##"Equis MetaStock 7.2 PS sample systems (NIKSAN pack)"##, family: ("nkms_ps", r##"Equis PS systems (NIKSAN pack)"##), page: r##"MS72PRFT.DTA record 48"##, status: NotTestable(r##"uses DMI(), MetaStock's built-in Dynamic Momentum Index (Chande/Kroll): the manuals give only its bounds (3 to 30 periods) and that it is driven by a 5-period standard deviation against its 10-period average, not the formula, so it cannot be rebuilt exactly; not coded"##), f: [Some(r##"Cross(DMI(C),30)"##), Some(r##"Cross(80,DMI(C)) OR If(ValueWhen(1,Cross(DMI(C),30),C)>Mov(C,200,S),Cross(Mov(C,200,S),C),0)"##), Some(r##"Cross(80,DMI(C))"##), Some(r##"Cross(DMI(C),30) OR If(ValueWhen(1,Cross(80,DMI(C)),C)<Mov(C,200,S),Cross(C,Mov(C,200,S)),0)"##)], exec: EXEC_DEFAULT, notes: r##"Dynamic Momentum Index crossing 30 / 80 with a 200-day average exit."## },
    Sys { id: "nkms_elliott_oscillator_system_1", name: r##"PS Elliott Oscillator System 1"##, group: r##"Equis MetaStock 7.2 PS sample systems (NIKSAN pack)"##, family: ("nkms_ps", r##"Equis PS systems (NIKSAN pack)"##), page: r##"MS72PRFT.DTA record 49"##, status: Live, f: [Some(r##"var1:=Mov((H+L)/2,5,S)-Mov((H+L)/2,34,S); var2:=var1>Ref(var1,-1) AND Ref(var1,-1)<Ref(var1,-2); var3:=Mov((H+L)/2,5,S)-Mov((H+L)/2,34,S); var4:=var3<Ref(var3,-1) AND Ref(var3,-1)>Ref(var3,-2);  ValueWhen(1,var2,Ref(var1,-1))>ValueWhen(2,var2,Ref(var1,-1)) AND BarsSince(var2)<BarsSince(var4) AND var1<0 AND ValueWhen(1,var4,Ref(var3,-1))<0"##), Some(r##"var3:=Mov((H+L)/2,5,S)-Mov((H+L)/2,34,S); var4:=var3<Ref(var3,-1) AND Ref(var3,-1)>Ref(var3,-2);  var4"##), Some(r##"var1:=Mov((H+L)/2,5,S)-Mov((H+L)/2,34,S); var2:=var1>Ref(var1,-1) AND Ref(var1,-1)<Ref(var1,-2); var3:=Mov((H+L)/2,5,S)-Mov((H+L)/2,34,S); var4:=var3<Ref(var3,-1) AND Ref(var3,-1)>Ref(var3,-2);  ValueWhen(1,var4,Ref(var3,-1))<ValueWhen(2,var4,Ref(var3,-1)) AND BarsSince(var4)<BarsSince(var2) AND var3>0 AND ValueWhen(1,var2,Ref(var1,-1))>0"##), Some(r##"var1:=Mov((H+L)/2,5,S)-Mov((H+L)/2,34,S); var2:=var1>Ref(var1,-1) AND Ref(var1,-1)<Ref(var1,-2);  var2"##)], exec: EXEC_DEFAULT, notes: r##"Elliott oscillator (5 - 34 average of the mid-price): a higher trough below zero after a peak below zero: long, out at the next oscillator peak; the mirror short. ValueWhen is evaluated by this module's own code (the shared engine has no ValueWhen); every other part by the shared formula engine."## },
    Sys { id: "nkms_elliott_oscillator_system_2", name: r##"PS Elliott Oscillator System 2"##, group: r##"Equis MetaStock 7.2 PS sample systems (NIKSAN pack)"##, family: ("nkms_ps", r##"Equis PS systems (NIKSAN pack)"##), page: r##"MS72PRFT.DTA record 50"##, status: Live, f: [Some(r##"var1:=Mov((H+L)/2,5,S)-Mov((H+L)/2,34,S); var2:=var1>Ref(var1,-1) AND Ref(var1,-1)<Ref(var1,-2); var3:=Mov((H+L)/2,5,S)-Mov((H+L)/2,34,S); var4:=var3<Ref(var3,-1) AND Ref(var3,-1)>Ref(var3,-2);  ValueWhen(1,var2,Ref(var1,-1))>ValueWhen(2,var2,Ref(var1,-1)) AND BarsSince(var2)<BarsSince(var4)  AND ValueWhen(1,var2,Ref(C,-1))<ValueWhen(2,var2,Ref(C,-1)) AND var1<0 AND ValueWhen(1,var4,Ref(var3,-1))<0;"##), Some(r##"var3:=Mov((H+L)/2,5,S)-Mov((H+L)/2,34,S); var4:=var3<Ref(var3,-1) AND Ref(var3,-1)>Ref(var3,-2);  var4"##), Some(r##"var1:=Mov((H+L)/2,5,S)-Mov((H+L)/2,34,S); var2:=var1>Ref(var1,-1) AND Ref(var1,-1)<Ref(var1,-2); var3:=Mov((H+L)/2,5,S)-Mov((H+L)/2,34,S); var4:=var3<Ref(var3,-1) AND Ref(var3,-1)>Ref(var3,-2);  ValueWhen(1,var4,Ref(var3,-1))<ValueWhen(2,var4,Ref(var3,-1)) AND BarsSince(var4)<BarsSince(var2)  AND ValueWhen(1,var4,Ref(C,-1))>ValueWhen(2,var4,Ref(C,-1)) AND var3>0 AND ValueWhen(1,var2,Ref(var1,-1))>0;"##), Some(r##"var1:=Mov((H+L)/2,5,S)-Mov((H+L)/2,34,S); var2:=var1>Ref(var1,-1) AND Ref(var1,-1)<Ref(var1,-2);  var2"##)], exec: EXEC_DEFAULT, notes: r##"Elliott oscillator system 1 that also needs a price divergence (the close before the new trough lower than before the previous trough; mirror for shorts). ValueWhen is evaluated by this module's own code (the shared engine has no ValueWhen); every other part by the shared formula engine."## },
    Sys { id: "nkms_exponential_moving_average", name: r##"PS Exponential Moving Average"##, group: r##"Equis MetaStock 7.2 PS sample systems (NIKSAN pack)"##, family: ("nkms_ps", r##"Equis PS systems (NIKSAN pack)"##), page: r##"MS72PRFT.DTA record 51"##, status: Live, f: [Some(r##"Mov(C,2,E)>Ref(Mov(C,2,E),-1)"##), Some(r##"Mov(C,2,E)<Ref(Mov(C,2,E),-1)"##), Some(r##"Mov(C,2,E)<Ref(Mov(C,2,E),-1)"##), Some(r##"Mov(C,2,E)>Ref(Mov(C,2,E),-1)"##)], exec: EXEC_DEFAULT, notes: r##"2-day EMA rising: long; falling: short."## },
    Sys { id: "nkms_fractal_trading_system_1", name: r##"PS Fractal Trading System 1"##, group: r##"Equis MetaStock 7.2 PS sample systems (NIKSAN pack)"##, family: ("nkms_ps", r##"Equis PS systems (NIKSAN pack)"##), page: r##"MS72PRFT.DTA record 52"##, status: Live, f: [Some(r##"F1:=ValueWhen(1,H<Ref(H,-2) AND Ref(H,-1)<Ref(H,-2) AND Ref(H,-3)<Ref(H,-2) AND Ref(H,-4)<Ref(H,-2),Ref(H,-2));  Cross(H,F1)"##), Some(r##"F2:=ValueWhen(1,L>Ref(L,-2) AND Ref(L,-1)>Ref(L,-2) AND Ref(L,-3)>Ref(L,-2) AND Ref(L,-4)>Ref(L,-2),Ref(L,-2));  Cross(F2,L)"##), Some(r##"F2:=ValueWhen(1,L>Ref(L,-2) AND Ref(L,-1)>Ref(L,-2) AND Ref(L,-3)>Ref(L,-2) AND Ref(L,-4)>Ref(L,-2),Ref(L,-2));  Cross(F2,L)"##), Some(r##"F1:=ValueWhen(1,H<Ref(H,-2) AND Ref(H,-1)<Ref(H,-2) AND Ref(H,-3)<Ref(H,-2) AND Ref(H,-4)<Ref(H,-2),Ref(H,-2));  Cross(H,F1)"##)], exec: EXEC_DEFAULT, notes: r##"High crossing above the latest up fractal (5-bar high): long; low crossing below the latest down fractal: short; each side closes on the opposite cross. ValueWhen is evaluated by this module's own code (the shared engine has no ValueWhen); every other part by the shared formula engine."## },
    Sys { id: "nkms_fractal_trading_system_2", name: r##"PS Fractal Trading System 2"##, group: r##"Equis MetaStock 7.2 PS sample systems (NIKSAN pack)"##, family: ("nkms_ps", r##"Equis PS systems (NIKSAN pack)"##), page: r##"MS72PRFT.DTA record 53"##, status: Live, f: [Some(r##"F1:=ValueWhen(1,H<Ref(H,-2) AND Ref(H,-1)<Ref(H,-2) AND Ref(H,-3)<Ref(H,-2) AND Ref(H,-4)<Ref(H,-2),Ref(H,-2));  Cross(H,F1)"##), Some(r##"F2:=ValueWhen(2,L>Ref(L,-2) AND Ref(L,-1)>Ref(L,-2) AND Ref(L,-3)>Ref(L,-2) AND Ref(L,-4)>Ref(L,-2),Ref(L,-2));  Cross(F2,L)"##), None, None], exec: EXEC_DEFAULT, notes: r##"High crossing above the latest up fractal: long; out when the low crosses below the second-latest down fractal. Long only. ValueWhen is evaluated by this module's own code (the shared engine has no ValueWhen); every other part by the shared formula engine."## },
];
