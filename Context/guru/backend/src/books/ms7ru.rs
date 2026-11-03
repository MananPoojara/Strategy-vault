//! "Учебник по MetaStock 7.0" (an anonymous Russian MetaStock 7 course; research share USB HDD 1/MetaStock/@Knowledge/_Russian/
//! Учебник по MetaStock 7.0/, chapters 7-10 .doc; text authors/batch-e/E3w/ms7ru_ch07..ch10). Book id `ms7ru`. Six System
//! Tester exercises, coded as labelled teaching examples (R4 decisions 49-50). Lane K10 (titles loop, 1 Nov 2026), to-code rows
//! 168-173; independent check authors/titles-1030/K10w/k10_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::msws::{ema, rsi, simulate, sma, Exec, Sig, EXEC_NEXT_OPEN};

/// Admitted ids (authors/titles-1030/K10w/k10_ref.py (header)).
pub const REVIEWED: &[&str] = &["ms7ru_ma21", "ms7ru_3ma", "ms7ru_ma39", "ms7ru_mo13_wave", "ms7ru_rsi_30_70", "ms7ru_willr_wave"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

macro_rules! common { () => { " [Course settings] The course tells the student to set before every test: trade at the open with a 1-day delay (signal at the close, fill at the next day's open), both long and short positions, 2% commission per side, 20,000 starting capital. Fills follow it; the 2% commission is a tester setting, not part of the rule, so the vault's standard costs apply (lead decision 50). MetaStock System Tester as in the msws engine: one position; a close rule is checked from the entry day's close; a close and the opposite entry on the same day reverse at the next open. Mov(..,S) = simple average, Mov(..,E) = MetaStock exponential average (alpha 2/(n+1), started at the first close), RSI = Wilder's, Cross(a,b) = a above b today and at or below it yesterday. A teaching example on Russian stocks (EESR, LKOH) with no printed result; indices, ETFs and US/India stocks are a declared application." } }

const MA21: &str = concat!("[Declared readings] Exercise 'MA Test' (ch.7): long when the close crosses above its 21-day simple average, short when it crosses below; each cross closes the other side (stop and reverse). The course's later stop tasks (12% maximum loss, then an inactivity stop of 10% within 14 days) are not offered here.", common!());
const MA3: &str = concat!("[Declared readings] Exercise 1 of ch.7, '3MA' without stops: long while EMA5 > EMA13 > EMA21, closed (and reversed short) when EMA21 > EMA13 > EMA5; the printed short rules are the same two formulas swapped. Task 2's 28-day long average is the option p.slow = 28. Task 3's 46% profit target and task 4's 'stop on profit 10% within 41 days' are not applied (task 4 names no MetaStock stop that fits); the course itself calls the optimised result curve-fitted.", common!());
const MA39: &str = concat!("[Declared readings] Ch.8's example of a simple moving-average crossover: long when the close is above its 39-day simple average, out when it is below. Long only, no stops. The chapter's optimisation ranges (EMA 8-30 with a points stop, RSI 20-40/70-80, EMA cross 1-100) are not rules and are not coded.", common!());
const MO13: &str = concat!("[Declared readings] Ch.9 control task: binary wave = (+1 if Mo(13) > 100, else -1) + (-1 if the 21-day simple average is above the close, else +1); enter long at wave = 2, close long at wave = -2; enter short at wave = -2, close short at wave = 2. So long when 13-day momentum is above 100 and the close is at or above the average, out and short when momentum is 100 or less and the close is below it. Mo(n) = 100 x close / close n days ago (MetaStock Momentum). The course asks to add and optimise stops afterwards but gives no values: none applied.", common!());
const RSI: &str = concat!("[Declared readings] Ch.10 'RSI w/opt' system: enter long when RSI(14) crosses above 30, close the long when it crosses below 70; enter short when it crosses below 70, close the short when it crosses above 30. The name says 'w/opt' but the printed formulas hold fixed values; the loss/profit stops the course asks for afterwards have no values and are not applied.", common!());
const WR: &str = concat!("[Declared readings] Ch.10 control task: wave = (+1 if Williams %R(11) > -10, -1 if it is < -90, else 0) + (-1 if the 7-day exponential average is above the close, else +1); long at wave = 2, out at wave = -2; short at wave = -2, out at wave = 2. So long when %R is above -10 (the overbought zone is the buy side, as printed) and the close is at or above the average; short when %R is below -90 and the close is below the average. %R(11) = -100 x (highest high of 11 days - close) / (highest high - lowest low); undefined when the 11-day range is zero.", common!());

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, dir, rules, notes| StrategyDef { id, name, group: "Russian MetaStock 7 course exercises", session: 0, dir, status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "ms7ru", rules, notes, claim: None };
    vec![
        d("ms7ru_ma21", "MA Test: close crosses its 21-day average, long and short at the next open", "Long/Short", &[
            "E|\"Enter Long: cross(close, mov(close,21,simple))\"; \"Enter Short: cross(mov(close,21,simple),close)\" (ch.7)",
            "X|\"Close Long: cross(mov(close,21,simple),close)\"; \"Close Short: cross(close,mov(close,21,simple))\" (ch.7)"], MA21),
        d("ms7ru_3ma", "3MA: EMA 5 > 13 > 21 long, EMA 21 > 13 > 5 short, at the next open", "Long/Short", &[
            "E|\"EL: mov(c,5,e)>mov(c,13,e) and mov(c,13,e)>mov(c,21,e)\"; \"ES: mov(c,21,e)>mov(c,13,e) and mov(c,13,e)>mov(c,5,e)\" (ch.7, task 1)",
            "X|\"CL: mov(c,21,e)>mov(c,13,e) and mov(c,13,e)>mov(c,5,e)\"; \"CS: mov(c,5,e)>mov(c,13,e) and mov(c,13,e)>mov(c,21,e)\" (ch.7, task 1)"], MA3),
        d("ms7ru_ma39", "Close above its 39-day simple average: long, out below it, at the next open", "Long", &[
            "E|\"Enter Long: c > mov(c,39,s)\" (ch.8)", "X|\"Close Long: c < mov(c,39,s)\" (ch.8)"], MA39),
        d("ms7ru_mo13_wave", "Momentum(13) + 21-day average binary wave: long at +2, short at -2, at the next open", "Long/Short", &[
            "F|\"If(Mo(13)>100,1,-1) + If(Mov(C,21,S)>C,-1,1)\" (ch.9, control task)",
            "E|Enter Long: wave = 2; Enter Short: wave = -2", "X|Close Long: wave = -2; Close Short: wave = 2"], MO13),
        d("ms7ru_rsi_30_70", "RSI(14) crosses up through 30 long, down through 70 short, at the next open", "Long/Short", &[
            "E|\"Enter Long: cross(rsi(14),30)\"; \"Enter Short: cross(70,rsi(14))\" (ch.10)",
            "X|\"Close Long: cross(70,rsi(14))\"; \"Close Short: cross(rsi(14),30)\" (ch.10)"], RSI),
        d("ms7ru_willr_wave", "Williams %R(11) + 7-day EMA binary wave: long at +2, short at -2, at the next open", "Long/Short", &[
            "F|\"If(WillR(11)>-10,1,If(WillR(11)<-90,-1,0)) + If(Mov(C,7,E)>C,-1,1)\" (ch.10, control task 1)",
            "E|Enter Long: wave = 2; Enter Short: wave = -2", "X|Close Long: wave = -2; Close Short: wave = 2"], WR),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let f = match id {
        "ms7ru_ma21" => ("Russian MS7 course: MA Test", "21-day SMA cross"),
        "ms7ru_3ma" => ("Russian MS7 course: 3MA", "EMA 5/13/21"),
        "ms7ru_ma39" => ("Russian MS7 course: 39-day average", "close vs SMA 39"),
        "ms7ru_mo13_wave" => ("Russian MS7 course: momentum wave", "Mo 13 + SMA 21"),
        "ms7ru_rsi_30_70" => ("Russian MS7 course: RSI 30/70", "RSI 14"),
        "ms7ru_willr_wave" => ("Russian MS7 course: %R wave", "%R 11 + EMA 7"),
        _ => return None,
    };
    let id = REVIEWED.iter().find(|r| **r == id)?;
    Some((*id, f.0, f.1))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    if !reviewed(id) { return None; }
    Some(if id == "ms7ru_3ma" { vec![ParamDef { key: "slow", label: "Long EMA (21; task 2: 28)", def: 21., min: 14., max: 60., step: 1. }] } else { vec![] })
}

fn cross(a: &[f64], b: &[f64], i: usize) -> bool { i > 0 && a[i] > b[i] && a[i - 1] <= b[i - 1] }

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let (h, l, c, n) = (&x.h, &x.l, &x.c, x.c.len());
    let all = |f: &dyn Fn(usize) -> bool| -> Vec<bool> { (0..n).map(f).collect() };
    let none = vec![false; n];
    let wave = |w: &[f64]| Sig { el: all(&|i| w[i] == 2.), xl: all(&|i| w[i] == -2.), es: all(&|i| w[i] == -2.), xs: all(&|i| w[i] == 2.) };
    let sig = match id {
        "ms7ru_ma21" => { let m = sma(c, 21); let (u, d) = (all(&|i| cross(c, &m, i)), all(&|i| cross(&m, c, i))); Sig { el: u.clone(), xl: d.clone(), es: d, xs: u } }
        "ms7ru_3ma" => {
            let slow = p.get("slow").copied().unwrap_or(21.).round().max(2.) as usize;
            let (a, b, s) = (ema(c, 5), ema(c, 13), ema(c, slow));
            let (u, d) = (all(&|i| a[i] > b[i] && b[i] > s[i]), all(&|i| s[i] > b[i] && b[i] > a[i]));
            Sig { el: u.clone(), xl: d.clone(), es: d, xs: u }
        }
        "ms7ru_ma39" => { let m = sma(c, 39); Sig { el: all(&|i| c[i] > m[i]), xl: all(&|i| c[i] < m[i]), es: none.clone(), xs: none } }
        "ms7ru_mo13_wave" => {
            let m = sma(c, 21);
            wave(&(0..n).map(|i| if i < 13 || !m[i].is_finite() { f64::NAN } else {
                (if 100. * c[i] / c[i - 13] > 100. { 1. } else { -1. }) + (if m[i] > c[i] { -1. } else { 1. }) }).collect::<Vec<_>>())
        }
        "ms7ru_rsi_30_70" => {
            let r = rsi(c, 14); let (k30, k70) = (vec![30.; n], vec![70.; n]);
            let (u30, d70) = (all(&|i| cross(&r, &k30, i)), all(&|i| cross(&k70, &r, i)));
            Sig { el: u30.clone(), xl: d70.clone(), es: d70, xs: u30 }
        }
        "ms7ru_willr_wave" => {
            let e = ema(c, 7);
            wave(&(0..n).map(|i| {
                if i < 10 || !e[i].is_finite() { return f64::NAN; }
                let (hh, ll) = (h[i - 10..=i].iter().copied().fold(f64::MIN, f64::max), l[i - 10..=i].iter().copied().fold(f64::MAX, f64::min));
                if hh == ll { return f64::NAN; }
                let wr = -100. * (hh - c[i]) / (hh - ll);
                (if wr > -10. { 1. } else if wr < -90. { -1. } else { 0. }) + (if e[i] > c[i] { -1. } else { 1. })
            }).collect::<Vec<_>>())
        }
        _ => return None,
    };
    let e: Exec = EXEC_NEXT_OPEN;
    Some(simulate(x, from, to, &sig, e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn every_exercise_trades_at_the_next_open() {
        let r: Vec<Row> = (0..900).map(|i| { let f = i as f64; let c = 100. + 9. * (f / 17.).sin() + 4. * (f / 5.3).cos() + 0.02 * f;
            Row { ticker: "T".into(), date: format!("{:05}", i), open: c - 0.5 * (f / 2.1).sin(), high: c + 1.3, low: c - 1.1, close: c, volume: None } }).collect();
        let x = Ctx::new(&r, &r, &[]);
        for id in REVIEWED {
            let t = run(id, &x, 0, r.len() - 1, &Params::new()).unwrap();
            assert!(t.len() > 3, "{id}");
            for z in &t { assert_eq!(z.legs[0].px, x.o[z.legs[0].i], "{id}"); }
            for w in t.windows(2) { assert!(w[1].legs[0].i >= w[0].exit_i, "{id}"); }
            if *id == "ms7ru_ma39" { assert!(t.iter().all(|z| z.dir > 0.)); }
            assert!(family_of(id).is_some() && params(id).is_some());
        }
        assert!(run("ms7ru_x", &x, 0, 5, &Params::new()).is_none());
    }
}

// Instrument admission: Russian MetaStock 7.0 course (Учебник по MetaStock 7.0) 2026-11-01 (titles lane K10).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "ms7ru" && reviewed(def.id),
    label: "Teaching examples from a Russian MetaStock 7 course, practised on Russian stocks with no printed results. Runs on indices, ETFs and stocks, US and India, are a declared application.",
    basis: "Учебник по MetaStock 7.0, ch.7 (System Tester: MA Test, 3MA), ch.8 (39-day average), ch.9 (momentum wave), ch.10 (RSI, %R wave) .doc files; independent check authors/titles-1030/K10w/k10_ref.py",
};
