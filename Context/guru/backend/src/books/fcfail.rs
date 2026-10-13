//! Fabrizio Corrias, "TL-Failure Swing" (MetaStock translation of Corrias' .ela by IGB Trading System, Ing. Giuseppe Belfiori;
//! distributed by tradinglibrary.com). Source: research drive MetaStock/@Systems & Indicators/Failure Swing for MS/MS70PRFT.DTA
//! (System Tester formulas) and failure.zip!Failure Swing.txt (Expert); copies in Books/Fabrizio Corrias/TL-Failure Swing/.
//! Book id "fcfail". Notes: library/fcfail/README.md.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "fcfail";
const NOTE: &str = "System Tester formulas (MS70PRFT.DTA) are the default. Mov(x,n,E) = EMA with smoothing 2/(n+1) seeded with the first value; BarsSince(Event) counts from the most recent Event (a new Event while in a trade restarts the count). The DTA does not store a readable trade price/delay, so fills are at the signal close (MetaStock System Tester default: price Close, delay 0) [assumption]. Long and short are separate strategies (the tester's 'positions' setting is not readable). No result is printed. India: instrument swap; index shorts are hypothetical.";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, dir, rules, notes| StrategyDef { id, name, group: "TL-Failure Swing (MetaStock system)", session: 0, dir, status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None };
    vec![
        d("fcfail_long", "TL-Failure Swing long: 20-bar low retest with MACD < 0 above its 20-bar low", "Long",
          &["F|\"LENGHT:=20; LL:=Ref(LLV(L, LENGHT),-1); OSC:= Mov(C, 12, E)-Mov(C,26,E);\"",
            "E|Enter long: \"Ref(L,-1) <> LL AND Ref(L,-2) <> LL AND Ref(L,-3) <> LL AND L <= LL AND C >= LL AND OSC < 0 AND OSC > Ref(LLV(OSC,LENGHT),-1)\"",
            "X|Close long: \"Sellsignal1:=If(BarsSince(Event)>1,C < Ref(LLV(L,5),-1),0); Sellsignal2:=If(BarsSince(Event)>10,C < Mov(L,8,E),0); Sellsignal1 OR Sellsignal2\" (Event = the entry condition)"],
          NOTE),
        d("fcfail_short", "TL-Failure Swing short: 20-bar high retest with MACD > 0 below its 20-bar high", "Short",
          &["F|\"LENGHT:=20; HH:=Ref(HHV(H, LENGHT),-1); OSC:= Mov(C, 12, E)-Mov(C,26,E);\"",
            "E|Enter short: \"Ref(H,-1) <> HH AND Ref(H,-2) <> HH AND Ref(H,-3) <> HH AND H >= HH AND C <= HH AND OSC > 0 AND OSC < Ref(HHV(OSC,LENGHT),-1)\"",
            "X|Close short: \"Buysignal1:= If(BarsSince(event)>1,C > Ref(HHV(L,5),-1),0); Buysignal2:=If(BarsSince(event)>10, C> Mov(H,8,E),0); Buysignal1 OR Buysignal2\"",
            "X|p.cover_on_high=1 [non-default]: Buysignal1 with Ref(HHV(H,5),-1), the mirror of the long exit. The default keeps HHV(L,5) as printed in both the System and the Expert ('Exitshort C > Ref(HHV(L,5),-1)')."],
          NOTE),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id {
        "fcfail_long" => Some(("fcfail_fs", "TL-Failure Swing (Corrias)", "Long")),
        "fcfail_short" => Some(("fcfail_fs", "TL-Failure Swing (Corrias)", "Short")),
        _ => None,
    }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let mut v = vec![p("len", "LENGHT (retest and MACD window)", 20., 10., 40., 1.), p("fast", "MACD fast EMA", 12., 5., 20., 1.), p("slow", "MACD slow EMA", 26., 15., 50., 1.),
        p("exit_n", "Exit channel bars", 5., 2., 10., 1.), p("bars1", "Channel exit after BarsSince >", 1., 0., 5., 1.),
        p("bars2", "EMA exit after BarsSince >", 10., 2., 20., 1.), p("exit_ema", "Exit EMA of lows/highs", 8., 3., 20., 1.)];
    match id {
        "fcfail_long" => Some(v),
        "fcfail_short" => { v.push(p("cover_on_high", "Cover channel on HHV(H) instead of the printed HHV(L)", 0., 0., 1., 1.)); Some(v) }
        _ => None,
    }
}

/// Mov(x,n,E): smoothing 2/(n+1), seeded with the first value.
fn ema(v: &[f64], n: usize) -> Vec<f64> {
    let k = 2.0 / (n as f64 + 1.0);
    let mut e = f64::NAN;
    v.iter().map(|&x| { e = if e.is_nan() { x } else { e + k * (x - e) }; e }).collect()
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let dir = match id { "fcfail_long" => 1., "fcfail_short" => -1., _ => return None };
    if from >= to || to >= x.r.len() { return None; }
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d).max(0.) as usize;
    let (len, en, b1, b2) = (g("len", 20.).max(1), g("exit_n", 5.).max(1), g("bars1", 1.), g("bars2", 10.));
    let (ef, es) = (ema(&x.c, g("fast", 12.).max(1)), ema(&x.c, g("slow", 26.).max(1)));
    let osc: Vec<f64> = ef.iter().zip(&es).map(|(a, b)| a - b).collect();
    let long = dir > 0.;
    let ex = if long { ema(&x.l, g("exit_ema", 8.).max(1)) } else { ema(&x.h, g("exit_ema", 8.).max(1)) };
    let chan = if long || g("cover_on_high", 0.) == 0 { &x.l } else { &x.h };
    let mn = |v: &[f64], a: usize, b: usize| v[a..b].iter().copied().fold(f64::MAX, f64::min);
    let mx = |v: &[f64], a: usize, b: usize| v[a..b].iter().copied().fold(f64::MIN, f64::max);
    let event = |i: usize| -> bool {
        if i < len.max(3) { return false; }
        if long {
            let ll = mn(&x.l, i - len, i);
            x.l[i - 1] != ll && x.l[i - 2] != ll && x.l[i - 3] != ll && x.l[i] <= ll && x.c[i] >= ll && osc[i] < 0. && osc[i] > mn(&osc, i - len, i)
        } else {
            let hh = mx(&x.h, i - len, i);
            x.h[i - 1] != hh && x.h[i - 2] != hh && x.h[i - 3] != hh && x.h[i] >= hh && x.c[i] <= hh && osc[i] > 0. && osc[i] < mx(&osc, i - len, i)
        }
    };
    let mut out = vec![];
    let mut cur: Option<Trade> = None;
    let mut last_ev: Option<usize> = None;
    // events before the window still set BarsSince (MetaStock computes it over all loaded bars)
    for i in 0..from { if event(i) { last_ev = Some(i); } }
    for i in from..=to {
        let ev = event(i);
        if ev { last_ev = Some(i); }
        if let Some(t) = cur.as_mut() {
            let bs = last_ev.map_or(usize::MAX, |e| i - e);
            let s1 = bs != usize::MAX && bs > b1 && i >= en && if long { x.c[i] < mn(chan, i - en, i) } else { x.c[i] > mx(chan, i - en, i) };
            let s2 = bs != usize::MAX && bs > b2 && if long { x.c[i] < ex[i] } else { x.c[i] > ex[i] };
            if s1 || s2 {
                t.exit_i = i; t.exit_px = x.c[i]; t.signal_i = i;
                t.reason = if s1 { format!("Close beyond the prior {en}-bar channel · close") } else { "Close beyond the exit EMA · close".into() };
                out.push(cur.take().unwrap());
            }
            continue;
        }
        if ev && i < to {
            cur = Some(Trade { dir, legs: vec![Leg { i, px: x.c[i], w: 1. }], exit_i: i, exit_px: x.c[i], reason: String::new(), entry_kind: "close", signal_i: i, open: false, exits: vec![] });
        }
    }
    if let Some(mut t) = cur { t.exit_i = to; t.exit_px = x.c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true; out.push(t); }
    Some(out)
}
