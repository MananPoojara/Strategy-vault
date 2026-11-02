//! Equis MetaStock 11 "Expert System" tester records and "MS11 - Turtle Trader Hybrid" in MS11PRFT.DTA (research share USB HDD 1/
//! MetaStock/MetaStock 11 Indicators, Explorers, System Testers & Experts for Metastock 10/MS11PRFT.DTA; formulas read from the
//! record slots as text: authors/batch-j/J4w/expert_bollinger.txt, expert_cmo.txt, expert_dirmov.txt, expert_momentum.txt,
//! expert_stochastic.txt, turtle_trader_hybrid.txt). Book id `msexp` (the Equis sample System Tester records of the same file are
//! `msequis`, lane K15; none of these formulas is one of those). Lane K17 (titles loop, 2 Nov 2026), to-code rows 358-363.
//! The formulas use ValueWhen, HighestSince, PREV, CMO and ADXR, which the shared `msws` interpreter lacks, so they are written
//! out here by hand (msws is shared and not edited). Independent check authors/titles-1030/K17w/k17_ref.py.
use crate::connors::{atr, true_range, wilder, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};
use super::nijat::sdev;
use super::rrsim::{highest, lowest, sdef, sma, xavg};

pub const REVIEWED: &[&str] = &["msexp_bollinger", "msexp_cmo", "msexp_dirmov", "msexp_momentum", "msexp_stoch", "msexp_turtle"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

macro_rules! conv { () => { " MetaStock System Tester defaults (the records store no entry price and an all-zero stop block, so no stops): signals at the close, filled at that close; a Close formula, and an opposite Enter, act from the bar after the entry; an opposite Enter reverses the position (MetaStock 7.2 manual, The System Editor Dialog), and after an exit no same-direction entry on that bar. Plain comparisons; a function of an undefined value is undefined and never true; Cross(a,b) = a > b now and a <= b one bar earlier; BarsSince / ValueWhen / HighestSince are undefined before their condition first holds; PREV starts at 0. The records name no market or test; daily indices, ETFs and US/India stocks are a declared application; one unit; the vault's standard costs." } }
const BB: &str = concat!("[Declared readings] 'Equis - Bollinger Bands - Expert System' (after Bollinger, S&C Feb 1992), formulas as stored. BBandBot/Top(C,20,S,d) = SMA20 -/+ d x the population SD of 20 closes; RSI(14) = Wilder's. Setup bar S (long) = yesterday's close below the 2-SD lower band, today's low >= yesterday's, yesterday's low <= the low before, yesterday's RSI < 35. Enter Long when Abs(13 - BarsSince(S)) <= 7 (the setup 6 to 20 bars ago, as printed), yesterday's close between the 2-SD and 1.25-SD lower bands, the same low turn, yesterday's RSI above its value at the last setup (ValueWhen), BarsSince(C > 1.25-SD band) < BarsSince(S) and BarsSince(C < 2-SD band) > BarsSince(C > 1.25-SD band). Enter Short is the mirror on the upper bands with highs and RSI > 65. Close Long = the Enter Short formula and Close Short = the Enter Long formula (stop and reverse).", conv!());
const CM: &str = concat!("[Declared readings] 'Equis - Chande Momentum Oscillator - Expert System', formulas as stored. CMO(C,20) = 100 (sum of up closes - sum of down closes)/(their total) over 20 close changes (Chande); bcon = CMO crosses above its 9-day simple average, scon = crosses below; bset = Cross(-50, CMO) (CMO falls through -50), sset = Cross(CMO, 50) (rises through 50); setup = If(bset, 1, If(sset, -1, PREV)); signal = If(setup = 1 AND bcon, 1, If(setup = -1 AND scon, -1, PREV)). Enter Long = signal crosses above 0.5 (turns to +1), Enter Short = crosses below -0.5; the Close formulas are the opposite Enters (stop and reverse).", conv!());
const DM: &str = concat!("[Declared readings] 'Equis - Directional Movement - Expert System', formulas as stored (the Close Short slot read by lane J4; K15 had held this record as unreadable). PDI/MDI/ADX(14) = Wilder's directional system (Wilder averages seeded with the mean of the first 14 values); ADXR(14) = (ADX + ADX 14 bars ago)/2 (Wilder). E = Cross(PDI,MDI) AND ADXR > 25 AND (ADX > MDI OR ADX > PDI): Enter Long; F = the mirror Cross(MDI,PDI): Enter Short. Close Long = BarsSince(E) < BarsSince(F) AND HighestSince(1, E, K) turns from 0 to 1, K = ADX below yesterday's ADX while above both DIs and PDI > MDI (the first ADX turn down at a peak after the buy); Close Short mirrored with F and PDI < MDI. An opposite Enter also reverses.", conv!());
const MO: &str = concat!("[Declared readings] 'Equis - Momentum Indicators - Expert System', formulas as stored. Enter Long (and Close Short) = Cross(MACD(), Mov(MACD(),9,S)) OR Cross(CCI(14),-100) OR Cross(RSI(14),30) OR Cross(Stoch(5,3),20) OR (Stoch(5,3) > 80 AND RSI(14) > 70 AND CCI(14) > 100); Enter Short (and Close Long) is the mirror. MACD() = EMA(alpha 0.15) - EMA(alpha 0.075) of the close seeded with the first close; CCI(14) = (TP - SMA14 TP)/(0.015 x mean deviation), TP = (H+L+C)/3 (the record says CCIE could be used instead); RSI = Wilder's; Stoch(5,3) = 100 x Sum(C - LLV(L,5), 3)/Sum(HHV(H,5) - LLV(L,5), 3).", conv!());
const ST: &str = concat!("[Declared readings] 'Equis - Stochastic Oscillator - Expert System', formulas as stored: Enter Long = Cross(Stoch(5,3),20) AND C > Mov(C,200,E); Enter Short = Cross(80,Stoch(5,3)) AND C < Mov(C,200,E); Close Long = the Enter Short formula, Close Short = the Enter Long formula, so a long closes only on a fall through 80 below the 200-day EMA. Stoch(5,3) as in the Momentum expert; Mov(C,200,E) = EMA seeded with the first close, shown from bar 200.", conv!());
const TT: &str = concat!("[Declared readings] 'MS11 - Turtle Trader Hybrid' (system tester record, empty description), formulas as stored. Long state trade = If(PREV <= 0, If(LE, Ref(HHV(H,20),-1) - ATR(20), 0), If(L <= PREV, -1, If(LX, -2, PREV))) with LE = H crosses above the prior 20-day high, LX = L crosses below the prior 10-day low; Enter Long = Cross(trade > 0, 0.5), Close Long = Cross(0, trade) at the stored order price If(trade = -1, Ref(trade,-1), Ref(LLV(L,10),-1)) (the stop level if stopped, else the prior 10-day low), clipped to the bar's range. Short mirrored (SE = L crosses below the prior 20-day low, stop = prior 20-day low + ATR(20), stopped when H >= it, SX = H crosses above the prior 10-day high, exit price If(trade = -1, Ref(trade,-1), Ref(HHV(H,10),-1))). The long and short states run separately, as stored. ATR(20) = Wilder's. Entries at the signal bar's close (no entry price stored).", conv!());

pub fn catalogue() -> Vec<StrategyDef> {
    let g = "MetaStock 11 pack (MS11PRFT.DTA): Equis Expert System records and Turtle Trader Hybrid";
    vec![
        sdef("msexp", "msexp_bollinger", "Equis Bollinger Bands expert: after an RSI < 35 low under the 2-SD band, buy the close back inside the 1.25-SD band with higher RSI; mirror sells", g, "Long/Short", Status::Live, &[
            "E|Enter Long: \"Abs(13-BarsSince(Ref(C<BBandBot(C,20,S,2),-1) AND L>=Ref(L,-1) AND Ref(L,-1)<=Ref(L,-2) AND Ref(RSI(14)<35,-1)))<=7 AND Ref(C<BBandBot(C,20,S,1.25),-1) AND Ref(C>BBandBot(C,20,S,2),-1) AND L>=Ref(L,-1) AND Ref(L,-1)<=Ref(L,-2) AND Ref(RSI(14),-1)>ValueWhen(1,...,Ref(RSI(14),-1)) AND BarsSince(C>BBandBot(C,20,S,1.25))<BarsSince(...) AND BarsSince(C<BBandBot(C,20,S,2))>BarsSince(C>BBandBot(C,20,S,1.25))\"",
            "E|Enter Short: the mirror with BBandTop, H and RSI(14)>65", "X|Close Long = the Enter Short formula; Close Short = the Enter Long formula"], BB),
        sdef("msexp", "msexp_cmo", "Equis Chande Momentum expert: after CMO(20) falls through -50, buy when it crosses its 9-day average; after it rises through +50, sell when it crosses under", g, "Long/Short", Status::Live, &[
            "F|\"bcon:=Cross(CMO(C,20),Mov(CMO(C,20 ),9,S)); scon:=Cross(Mov(CMO(C,20 ),9,S),CMO(C,20)); bset:=Cross(-50,CMO(C,20)); sset:=Cross(CMO(C,20),50); setup:=If(bset,1, If(sset,-1,PREV));\"",
            "E|Enter Long: \"cross(If(setup=1 AND bcon,1,If(setup=-1 AND scon,-1,PREV)),0.5)\"; Enter Short: \"cross(-0.5,If(setup=1 AND bcon,1,If(setup=-1 AND scon,-1,PREV)))\"",
            "X|Close Long = the Enter Short formula; Close Short = the Enter Long formula"], CM),
        sdef("msexp", "msexp_dirmov", "Equis Directional Movement expert: +DI/-DI cross with ADXR(14) > 25 and ADX above a DI; exit at the first ADX turn down from above both DIs", g, "Long/Short", Status::Live, &[
            "E|Enter Long: \"Cross(PDI(14),MDI(14)) AND ADXR(14)>25 AND (ADX(14)>MDI(14) OR ADX(14)> PDI(14))\"; Enter Short: \"Cross(MDI(14),PDI(14)) AND ADXR(14)>25 AND (ADX(14)>MDI(14) OR ADX(14)> PDI(14))\"",
            "X|Close Long: \"BarsSince(<buy>)< BarsSince(<sell>) AND Ref(HighestSince(1,<buy>, ADX(14)<Ref(ADX(14),-1) AND ADX(14)>MDI(14) AND ADX(14)>PDI(14) AND PDI(14)>MDI(14)),-1)=0 AND HighestSince(1,<buy>, ...)=1\" (Close Short mirrored with PDI(14)<MDI(14))"], DM),
        sdef("msexp", "msexp_momentum", "Equis Momentum Indicators expert: buy on any of MACD/CCI/RSI/Stochastic turning up from oversold (or all overbought), sell on the mirror; always in", g, "Long/Short", Status::Live, &[
            "E|Enter Long: \"( Cross(MACD(), Mov(MACD(),9,S)) OR Cross(CCI(14),-100) OR Cross(RSI(14),30) OR Cross(Stoch(5,3),20 ) ) OR ( (Stoch(5,3) > 80) AND (RSI(14) > 70) AND (CCI(14) > 100) )\"",
            "E|Enter Short: \"( Cross(Mov(MACD(),9,S),MACD()) OR Cross(100,CCI(14)) OR Cross(70,RSI(14)) OR Cross(80,Stoch(5,3) ) ) OR ( (Stoch(5,3) < 20) AND (RSI(14) < 30) AND (CCI(14) < -100) )\"",
            "X|Close Long = the Enter Short formula; Close Short = the Enter Long formula"], MO),
        sdef("msexp", "msexp_stoch", "Equis Stochastic Oscillator expert: Stoch(5,3) up through 20 above the 200-day EMA buys, down through 80 below it sells; stop and reverse", g, "Long/Short", Status::Live, &[
            "E|Enter Long: \"Cross(Stoch(5,3),20) AND C>Mov(C,200,E)\"; Enter Short: \"Cross(80,Stoch(5,3)) AND C<Mov(C,200,E)\"",
            "X|Close Long: \"Cross(80,Stoch(5,3)) AND C<Mov(C,200,E)\"; Close Short: \"Cross(Stoch(5,3),20) AND C>Mov(C,200,E)\""], ST),
        sdef("msexp", "msexp_turtle", "MS11 Turtle Trader Hybrid: 20-day high (low) breakout at the close, ATR(20) stop, 10-day low (high) exit", g, "Long/Short", Status::Live, &[
            "F|\"LE:=Cross(H,Ref(HHV(H,20),-1)); LX:=Cross(Ref(LLV(L,10),-1),L); trade:=if(prev<=0, if(LE, Ref(HHV(H,20),-1)-atr(20),0), if(l<=prev, -1, if(LX, -2, prev)));\"",
            "E|Enter Long: \"cross(trade>0,0.5)\"; Enter Short: the same with \"SE:=Cross(Ref(LLV(L,20),-1),L); SX:=Cross(H,Ref(HHV(H,10),-1)); trade:=if(prev<=0, if(SE, Ref(LLV(L,20),-1)+atr(20),0), if(H>=prev, -1, if(SX, -2, prev)))\"",
            "X|Close Long: \"cross(0,trade)\" at the order price \"if(trade=-1, ref(trade,-1),ref(llv(l,10),-1))\"; Close Short at \"if(trade=-1,ref(trade,-1),ref(hhv(H,10),-1))\""], TT),
    ]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let f = match id {
        "msexp_bollinger" => ("Equis Bollinger Bands expert", "BB 20, 2 / 1.25 SD, RSI 14"),
        "msexp_cmo" => ("Equis Chande Momentum expert", "CMO 20, 9-day average, +/-50"),
        "msexp_dirmov" => ("Equis Directional Movement expert", "DMI 14, ADXR > 25"),
        "msexp_momentum" => ("Equis Momentum Indicators expert", "MACD, CCI 14, RSI 14, Stoch 5,3"),
        "msexp_stoch" => ("Equis Stochastic expert", "Stoch 5,3, 20/80, EMA 200"),
        "msexp_turtle" => ("MS11 Turtle Trader Hybrid", "20/10-day breakout, ATR 20 stop"),
        _ => return None,
    };
    let id = REVIEWED.iter().find(|r| **r == id)?;
    Some((*id, f.0, f.1))
}
pub fn params(id: &str) -> Option<Vec<ParamDef>> { reviewed(id).then(Vec::new) }

const NAN: f64 = f64::NAN;
fn fin(v: f64) -> bool { v.is_finite() }
/// Cross(a, b): a > b now and a <= b one bar earlier (all four defined).
fn cross(a: &[f64], b: &[f64], i: usize) -> bool { i > 0 && fin(a[i]) && fin(b[i]) && fin(a[i - 1]) && fin(b[i - 1]) && a[i] > b[i] && a[i - 1] <= b[i - 1] }
fn konst(v: f64, n: usize) -> Vec<f64> { vec![v; n] }
fn bars_since(c: &[bool]) -> Vec<f64> {
    let mut last: Option<usize> = None;
    c.iter().enumerate().map(|(i, &b)| { if b { last = Some(i); } last.map_or(NAN, |k| (i - k) as f64) }).collect()
}
fn value_when(c: &[bool], x: &[f64]) -> Vec<f64> {
    let mut v = NAN;
    c.iter().enumerate().map(|(i, &b)| { if b { v = x[i]; } v }).collect()
}
fn highest_since(c: &[bool], x: &[f64]) -> Vec<f64> {
    let mut v = NAN;
    c.iter().enumerate().map(|(i, &b)| { if b { v = x[i]; } else if fin(v) { v = v.max(x[i]); } v }).collect()
}
fn rsi14(c: &[f64]) -> Vec<f64> { crate::connors::rsi(c, 14) }
fn cci14(x: &Ctx) -> Vec<f64> {
    let tp: Vec<f64> = (0..x.c.len()).map(|i| (x.h[i] + x.l[i] + x.c[i]) / 3.).collect();
    (0..tp.len()).map(|i| if i < 13 { NAN } else {
        let w = &tp[i - 13..=i]; let mu = w.iter().sum::<f64>() / 14.; let md = w.iter().map(|v| (v - mu).abs()).sum::<f64>() / 14.;
        if md > 0. { (tp[i] - mu) / (0.015 * md) } else { NAN }
    }).collect()
}
fn stoch53(x: &Ctx) -> Vec<f64> {
    (0..x.c.len()).map(|i| if i < 6 { NAN } else {
        let (mut num, mut den) = (0.0f64, 0.0f64);
        for j in i - 2..=i { let ll = lowest(&x.l, 5, j); num += x.c[j] - ll; den += highest(&x.h, 5, j) - ll; }
        if den > 0. { 100. * num / den } else { NAN }
    }).collect()
}
fn ema_a(v: &[f64], a: f64) -> Vec<f64> {
    let mut out = vec![NAN; v.len()];
    for i in 0..v.len() { out[i] = if i == 0 { v[0] } else { out[i - 1] + a * (v[i] - out[i - 1]) }; }
    out
}
/// Wilder's directional system: (PDI, MDI, ADX), as connors::dmi but also returning -DI.
fn dmi14(x: &Ctx) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let n = x.c.len();
    let (mut pdm, mut mdm, mut tr) = (vec![NAN; n], vec![NAN; n], vec![NAN; n]);
    let trs = true_range(x.r);
    for i in 1..n {
        let up = x.h[i] - x.h[i - 1]; let dn = x.l[i - 1] - x.l[i];
        pdm[i] = if up > dn && up > 0. { up } else { 0. };
        mdm[i] = if dn > up && dn > 0. { dn } else { 0. };
        tr[i] = trs[i];
    }
    let (sp, sm, st) = (wilder(&pdm, 14), wilder(&mdm, 14), wilder(&tr, 14));
    let (mut pdi, mut mdi, mut dx) = (vec![NAN; n], vec![NAN; n], vec![NAN; n]);
    for i in 0..n {
        if st[i] > 0. {
            let (p, m) = (100. * sp[i] / st[i], 100. * sm[i] / st[i]);
            pdi[i] = p; mdi[i] = m;
            dx[i] = if p + m > 0. { 100. * (p - m).abs() / (p + m) } else { 0. };
        }
    }
    (pdi, mdi, wilder(&dx, 14))
}

/// (enter long, close long, enter short, close short, close-long price, close-short price); NaN price = this bar's close.
type Sig = (Vec<bool>, Vec<bool>, Vec<bool>, Vec<bool>, Vec<f64>, Vec<f64>);
fn signals(id: &str, x: &Ctx) -> Sig {
    let n = x.c.len();
    let (h, l, c) = (&x.h, &x.l, &x.c);
    let none = vec![NAN; n];
    let sar = |el: Vec<bool>, es: Vec<bool>| (el.clone(), es.clone(), es, el, none.clone(), none.clone());
    match id {
        "msexp_bollinger" => {
            let (m, s) = (sma(c, 20), sdev(c, 20, 0.));
            let band = |d: f64| -> Vec<f64> { (0..n).map(|i| m[i] + d * s[i]).collect() };
            let (b2, b125, t2, t125) = (band(-2.), band(-1.25), band(2.), band(1.25));
            let r = rsi14(c);
            let lt = |a: f64, b: f64| fin(a) && fin(b) && a < b;
            let gt = |a: f64, b: f64| fin(a) && fin(b) && a > b;
            let sl: Vec<bool> = (0..n).map(|i| i >= 2 && lt(c[i - 1], b2[i - 1]) && l[i] >= l[i - 1] && l[i - 1] <= l[i - 2] && lt(r[i - 1], 35.)).collect();
            let ss: Vec<bool> = (0..n).map(|i| i >= 2 && gt(c[i - 1], t2[i - 1]) && h[i] <= h[i - 1] && h[i - 1] >= h[i - 2] && gt(r[i - 1], 65.)).collect();
            let rp: Vec<f64> = (0..n).map(|i| if i >= 1 { r[i - 1] } else { NAN }).collect();
            let (bsl, bss) = (bars_since(&sl), bars_since(&ss));
            let (vl, vs) = (value_when(&sl, &rp), value_when(&ss, &rp));
            let bs_above125: Vec<f64> = bars_since(&(0..n).map(|i| gt(c[i], b125[i])).collect::<Vec<_>>());
            let bs_below2: Vec<f64> = bars_since(&(0..n).map(|i| lt(c[i], b2[i])).collect::<Vec<_>>());
            let bs_below125t: Vec<f64> = bars_since(&(0..n).map(|i| lt(c[i], t125[i])).collect::<Vec<_>>());
            let bs_above2t: Vec<f64> = bars_since(&(0..n).map(|i| gt(c[i], t2[i])).collect::<Vec<_>>());
            let el: Vec<bool> = (0..n).map(|i| i >= 2 && fin(bsl[i]) && (13. - bsl[i]).abs() <= 7. && lt(c[i - 1], b125[i - 1]) && gt(c[i - 1], b2[i - 1])
                && l[i] >= l[i - 1] && l[i - 1] <= l[i - 2] && gt(rp[i], vl[i]) && lt(bs_above125[i], bsl[i]) && gt(bs_below2[i], bs_above125[i])).collect();
            let es: Vec<bool> = (0..n).map(|i| i >= 2 && fin(bss[i]) && (13. - bss[i]).abs() <= 7. && gt(c[i - 1], t125[i - 1]) && lt(c[i - 1], t2[i - 1])
                && h[i] <= h[i - 1] && h[i - 1] >= h[i - 2] && lt(rp[i], vs[i]) && lt(bs_below125t[i], bss[i]) && gt(bs_above2t[i], bs_below125t[i])).collect();
            sar(el, es)
        }
        "msexp_cmo" => {
            let mut cmo = vec![NAN; n];
            for i in 20..n {
                let (mut u, mut d) = (0.0f64, 0.0f64);
                for j in i - 19..=i { let ch = c[j] - c[j - 1]; if ch > 0. { u += ch } else { d -= ch } }
                if u + d > 0. { cmo[i] = 100. * (u - d) / (u + d); }
            }
            let ma = sma(&cmo, 9);
            let (k50, km50) = (konst(50., n), konst(-50., n));
            let (mut setup, mut sig) = (0.0f64, 0.0f64);
            let mut v = vec![0.0f64; n];
            for i in 0..n {
                if cross(&km50, &cmo, i) { setup = 1. } else if cross(&cmo, &k50, i) { setup = -1. }
                if setup == 1. && cross(&cmo, &ma, i) { sig = 1. } else if setup == -1. && cross(&ma, &cmo, i) { sig = -1. }
                v[i] = sig;
            }
            sar((0..n).map(|i| cross(&v, &konst(0.5, n), i)).collect(), (0..n).map(|i| cross(&konst(-0.5, n), &v, i)).collect())
        }
        "msexp_dirmov" => {
            let (p, m, a) = dmi14(x);
            let adxr: Vec<f64> = (0..n).map(|i| if i >= 14 { (a[i] + a[i - 14]) / 2. } else { NAN }).collect();
            let filt = |i: usize| fin(adxr[i]) && adxr[i] > 25. && fin(a[i]) && (a[i] > m[i] || a[i] > p[i]);
            let e: Vec<bool> = (0..n).map(|i| cross(&p, &m, i) && filt(i)).collect();
            let f: Vec<bool> = (0..n).map(|i| cross(&m, &p, i) && filt(i)).collect();
            let kk = |up: bool| -> Vec<f64> { (0..n).map(|i| if i >= 1 && fin(a[i]) && fin(a[i - 1]) && fin(p[i]) {
                (a[i] < a[i - 1] && a[i] > m[i] && a[i] > p[i] && if up { p[i] > m[i] } else { p[i] < m[i] }) as u8 as f64 } else { NAN }).collect() };
            let (be, bf) = (bars_since(&e), bars_since(&f));
            let (hl, hs) = (highest_since(&e, &kk(true)), highest_since(&f, &kk(false)));
            let xl: Vec<bool> = (0..n).map(|i| i >= 1 && fin(be[i]) && fin(bf[i]) && be[i] < bf[i] && hl[i - 1] == 0. && hl[i] == 1.).collect();
            let xs: Vec<bool> = (0..n).map(|i| i >= 1 && fin(be[i]) && fin(bf[i]) && be[i] > bf[i] && hs[i - 1] == 0. && hs[i] == 1.).collect();
            (e, xl, f, xs, none.clone(), none)
        }
        "msexp_momentum" => {
            let (e1, e2) = (ema_a(c, 0.15), ema_a(c, 0.075));
            let mc: Vec<f64> = (0..n).map(|i| e1[i] - e2[i]).collect();
            let sg = sma(&mc, 9);
            let (cc, r, st) = (cci14(x), rsi14(c), stoch53(x));
            let k = |v: f64| konst(v, n);
            let el: Vec<bool> = (0..n).map(|i| cross(&mc, &sg, i) || cross(&cc, &k(-100.), i) || cross(&r, &k(30.), i) || cross(&st, &k(20.), i)
                || (fin(st[i]) && fin(r[i]) && fin(cc[i]) && st[i] > 80. && r[i] > 70. && cc[i] > 100.)).collect();
            let es: Vec<bool> = (0..n).map(|i| cross(&sg, &mc, i) || cross(&k(100.), &cc, i) || cross(&k(70.), &r, i) || cross(&k(80.), &st, i)
                || (fin(st[i]) && fin(r[i]) && fin(cc[i]) && st[i] < 20. && r[i] < 30. && cc[i] < -100.)).collect();
            sar(el, es)
        }
        "msexp_stoch" => {
            let st = stoch53(x);
            let e: Vec<f64> = xavg(c, 200).into_iter().enumerate().map(|(i, v)| if i >= 199 { v } else { NAN }).collect();
            let (k20, k80) = (konst(20., n), konst(80., n));
            let el: Vec<bool> = (0..n).map(|i| cross(&st, &k20, i) && fin(e[i]) && c[i] > e[i]).collect();
            let es: Vec<bool> = (0..n).map(|i| cross(&k80, &st, i) && fin(e[i]) && c[i] < e[i]).collect();
            sar(el, es)
        }
        _ => {
            let at = atr(x.r, 20);
            let prior = |f: fn(&[f64], usize, usize) -> f64, v: &[f64], k: usize, i: usize| if i >= 1 { f(v, k, i - 1) } else { NAN };
            let (h20, l10, l20, h10): (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) = ((0..n).map(|i| prior(highest, h, 20, i)).collect(),
                (0..n).map(|i| prior(lowest, l, 10, i)).collect(), (0..n).map(|i| prior(lowest, l, 20, i)).collect(), (0..n).map(|i| prior(highest, h, 10, i)).collect());
            let (mut tl, mut ts) = (vec![0.0f64; n], vec![0.0f64; n]);
            let (mut pl, mut ps) = (0.0f64, 0.0f64);
            for i in 0..n {
                let le = cross(h, &h20, i); let lx = cross(&l10, l, i);
                let se = cross(&l20, l, i); let sx = cross(h, &h10, i);
                tl[i] = if pl <= 0. { if le { h20[i] - at[i] } else { 0. } } else if l[i] <= pl { -1. } else if lx { -2. } else { pl };
                ts[i] = if ps <= 0. { if se { l20[i] + at[i] } else { 0. } } else if h[i] >= ps { -1. } else if sx { -2. } else { ps };
                pl = tl[i]; ps = ts[i];
            }
            let pos = |t: &[f64], i: usize| (fin(t[i]) && t[i] > 0.) as u8 as f64;
            let el: Vec<bool> = (0..n).map(|i| i >= 1 && pos(&tl, i) > 0.5 && pos(&tl, i - 1) <= 0.5).collect();
            let es: Vec<bool> = (0..n).map(|i| i >= 1 && pos(&ts, i) > 0.5 && pos(&ts, i - 1) <= 0.5).collect();
            let xl: Vec<bool> = (0..n).map(|i| cross(&konst(0., n), &tl, i)).collect();
            let xs: Vec<bool> = (0..n).map(|i| cross(&konst(0., n), &ts, i)).collect();
            let xlp: Vec<f64> = (0..n).map(|i| if i >= 1 { if tl[i] == -1. { tl[i - 1] } else { l10[i] } } else { NAN }).collect();
            let xsp: Vec<f64> = (0..n).map(|i| if i >= 1 { if ts[i] == -1. { ts[i - 1] } else { h10[i] } } else { NAN }).collect();
            (el, xl, es, xs, xlp, xsp)
        }
    }
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, _p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let (el, xl, es, xs, xlp, xsp) = signals(id, x);
    let mut out: Vec<Trade> = vec![];
    let mut cur: Option<Trade> = None;
    for i in from..=to {
        let mut exited = 0.;
        if let Some(t) = cur.as_ref() {
            if i > t.legs[0].i {
                let d = t.dir;
                let (own, opp, px) = if d > 0. { (xl[i], es[i], xlp[i]) } else { (xs[i], el[i], xsp[i]) };
                if own || opp {
                    let fill = if own && fin(px) { px.max(x.l[i]).min(x.h[i]) } else { x.c[i] };
                    let mut t = cur.take().unwrap();
                    (t.exit_i, t.exit_px, t.signal_i) = (i, fill, i);
                    t.reason = if own { "Close formula".into() } else { "reversed by the opposite Enter · close".into() };
                    out.push(t);
                    exited = d;
                }
            }
        }
        if cur.is_none() {
            let d = if el[i] && exited <= 0. { 1. } else if es[i] && exited >= 0. { -1. } else { 0. };
            if d != 0. { cur = Some(Trade { dir: d, legs: vec![Leg { i, px: x.c[i], w: 1. }], exit_i: i, exit_px: x.c[i], reason: String::new(), entry_kind: "close", signal_i: i, open: false, exits: vec![] }); }
        }
    }
    if let Some(mut t) = cur { (t.exit_i, t.exit_px, t.signal_i, t.open) = (to, x.c[to], to, true); t.reason = "open".into(); out.push(t); }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ms11_experts() {
        let r = super::super::nijat::tests::bars(3000);
        let x = Ctx::new(&r, &r, &[]);
        for id in REVIEWED {
            let t = run(id, &x, 0, r.len() - 1, &Params::new()).unwrap();
            assert!(t.len() >= 2, "{id}: {}", t.len());
            for w in t.windows(2) { assert!(w[1].legs[0].i >= w[0].exit_i, "{id}"); }
            for z in &t { assert!(z.exit_px >= r[z.exit_i].low - 1e-9 && z.exit_px <= r[z.exit_i].high + 1e-9, "{id}"); }
            assert!(family_of(id).is_some() && run(id, &x, 5, 5, &Params::new()).is_none());
        }
        assert_eq!(bars_since(&[false, true, false, false])[3], 2.);
        assert_eq!(highest_since(&[false, true, false], &[5., 0., 1.])[2], 1.);
    }
}

// Instrument admission: MetaStock 11 Equis experts and Turtle Trader Hybrid 2026-11-02 (titles lane K17).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "msexp" && reviewed(def.id),
    label: "Equis's Expert System tester records and the MS11 Turtle Trader Hybrid name no market or test; runs on indices, ETFs and single stocks (US and India) are a declared application.",
    basis: "MS11PRFT.DTA (MetaStock 11 pack, research share USB HDD 1/MetaStock), Equis Bollinger Bands / Chande Momentum Oscillator / Directional Movement / Momentum Indicators / Stochastic Oscillator Expert System records and MS11 - Turtle Trader Hybrid (formula slots read by lane J4); independent check authors/titles-1030/K17w/k17_ref.py",
};
