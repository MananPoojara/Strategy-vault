//! 'Gann modify' MetaStock Expert with the Gann-HiLo2, Gann-Swing2 and Gann-Trend2 formulas (Italian MetaStock pack
//! 'Gann Systems + Info'; research share USB HDD 1/MetaStock/@Systems & Indicators/@Misc/- Metastock Systems & Indicators/
//! Gann Systems + Info/: info.doc, MS70FORM.DTA formula strings, EXPERTS/EXPT0001.DTA). Book id `gnmod`, no named author.
//! One rule: long while C > HiLo2 and Trend2 = 1, short while C < HiLo2 and Trend2 = -1. Lane K6 (titles loop,
//! 31 Oct 2026); independent check authors/titles-1030/K6w/k6_ref.py (header).
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::mkits::ema;
use super::rbtie::target_trades;

/// Admitted ids (authors/titles-1030/K6w/k6_ref.py (header)).
pub const REVIEWED: &[&str] = &["gnmod_hilo_trend"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] The formulas are coded as printed in MS70FORM.DTA. Mov(x,3,E) = MetaStock exponential average, alpha 2/4, seeded with the simple average of its first 3 values. A MetaStock value that is not yet defined (BarsSince, HighestSince, LowestSince or ValueWhen before their first event) makes the comparison that uses it false (no signal). The Expert colours a bar long when C > Gann-HiLo2 and Gann-Trend2 = 1 and short when C < Gann-HiLo2 and Gann-Trend2 = -1 (info.doc: 'short positions coloured red, long positions green'); the Expert's ribbon file is partly unreadable, so on bars where neither holds the position is flat (default; option: hold until the opposite state). Entries and exits at the close of the bar on which the state changes. Related but different: krgann_basic (Krausz). Daily bars; indices, ETFs and US/India stocks are a declared application; the pack prints no results.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "gnmod_hilo_trend", name: "Gann modify: Gann HiLo activator with the Gann swing trend, long or short while both agree", group: "Gann Systems + Info (MetaStock pack)",
        session: 0, dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "gnmod", rules: &[
            "F|GANN-HiLo2: \"HLd:=If(CLOSE>Ref(Mov(H,3,E),-1),1,If(CLOSE<Ref(Mov(L,3,E),-1),-1,0)); HLv:=ValueWhen(1,HLd<>0,HLd); HiLo:=If(HLv=-1,Mov(H,3,E),Mov(L,3,E))\"",
            "F|Gann-Swing2: \"Us:=BarsSince((H > Ref(H,-1)) AND (Ref(H,-1) > Ref(H,-2))); Ds:=BarsSince((L < Ref(L,-1)) AND (Ref(L,-1) < Ref(L,-2))); Sd1:=If(Us=0,If(Ref(L,-1)<>LowestSince(1,Ds=0,L),1,0),If(Ds=0,If(Ref(H,-1)<>HighestSince(1,Us=0,H),-1,0),0)); Sd2:=If(Sd1=1,If(Ref(BarsSince(Sd1=1),-1) > Ref(BarsSince(Sd1=-1),-1),1,0),If(Sd1=-1,If(Ref(BarsSince(Sd1=1),-1) < Ref(BarsSince(Sd1=-1),-1),-1,0),0)); TD1:=ValueWhen(1,Sd2<>0,Sd2)\"",
            "F|Gann-Trend2: \"Sch:=If(Sd=1 AND Ref(sd,-1)=-1,1,0); Scl:=If(Sd=-1 AND Ref(Sd,-1)=1,1,0); Pv:=If(Scl=1,HighestSince(1,Sch=1,H),0); Tv:=If(Sch=1,LowestSince(1,Scl=1,L),0); Td:=If(H>ValueWhen(1,Pv>0,Pv),1,If(L<ValueWhen(1,Tv>0,Tv),-1,0)); Tdv:=ValueWhen(1,Td<>0,Td)\"",
            "E|Expert 'Gann modify': \"C> Fml( \"GANN-HiLo2\") AND Fml( \"Gann-Trend2\")=1\" (long, green); the mirror C < HiLo2 AND Trend2 = -1 (short, red)",
            "X|The position ends at the close of the first bar on which its state no longer holds"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == "gnmod_hilo_trend").then_some(("gnmod_hilo_trend", "Gann modify expert", "HiLo2 + Trend2"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    (id == "gnmod_hilo_trend").then(|| vec![ParamDef { key: "hold", label: "Neutral bars: 0 = flat (default), 1 = hold until the opposite state", def: 0., min: 0., max: 1., step: 1. }])
}

/// HighestSince(1, cond, v) (hi) or LowestSince(1, cond, v): extreme of v since the latest bar where cond holds, inclusive.
fn since(cond: &[bool], v: &[f64], hi: bool) -> Vec<f64> {
    let mut run = f64::NAN;
    (0..v.len()).map(|i| {
        if cond[i] { run = v[i]; } else if run.is_finite() { run = if hi { run.max(v[i]) } else { run.min(v[i]) }; }
        run
    }).collect()
}
/// BarsSince(cond) (NaN before the first event).
fn bars_since(cond: &[bool]) -> Vec<f64> {
    let mut last = None;
    (0..cond.len()).map(|i| { if cond[i] { last = Some(i); } last.map_or(f64::NAN, |k| (i - k) as f64) }).collect()
}
/// ValueWhen(1, v <> 0, v) for a signal series (NaN before the first non-zero value).
fn carry(v: &[f64]) -> Vec<f64> {
    let mut last = f64::NAN;
    v.iter().map(|&x| { if x != 0. && x.is_finite() { last = x; } last }).collect()
}
fn when_pos(v: &[f64]) -> Vec<f64> {
    let mut last = f64::NAN;
    v.iter().map(|&x| { if x > 0. { last = x; } last }).collect()
}

/// (HiLo2, Trend2) per bar.
pub fn gann(x: &Ctx) -> (Vec<f64>, Vec<f64>) {
    let (h, l, c, n) = (&x.h, &x.l, &x.c, x.c.len());
    let (eh, el) = (ema(h, 3), ema(l, 3));
    let hld: Vec<f64> = (0..n).map(|i| if i > 0 && c[i] > eh[i - 1] { 1. } else if i > 0 && c[i] < el[i - 1] { -1. } else { 0. }).collect();
    let hlv = carry(&hld);
    let hilo: Vec<f64> = (0..n).map(|i| if hlv[i] == -1. { eh[i] } else { el[i] }).collect();
    let up: Vec<bool> = (0..n).map(|i| i >= 2 && h[i] > h[i - 1] && h[i - 1] > h[i - 2]).collect();
    let dn: Vec<bool> = (0..n).map(|i| i >= 2 && l[i] < l[i - 1] && l[i - 1] < l[i - 2]).collect();
    let (ls, hs) = (since(&dn, l, false), since(&up, h, true));
    let sd1: Vec<f64> = (0..n).map(|i| {
        if up[i] { if i > 0 && ls[i].is_finite() && l[i - 1] != ls[i] { 1. } else { 0. } }
        else if dn[i] { if i > 0 && hs[i].is_finite() && h[i - 1] != hs[i] { -1. } else { 0. } }
        else { 0. }
    }).collect();
    let (b1, bm1) = (bars_since(&sd1.iter().map(|&v| v == 1.).collect::<Vec<_>>()), bars_since(&sd1.iter().map(|&v| v == -1.).collect::<Vec<_>>()));
    let sd2: Vec<f64> = (0..n).map(|i| {
        if i == 0 { 0. } else if sd1[i] == 1. && b1[i - 1] > bm1[i - 1] { 1. } else if sd1[i] == -1. && b1[i - 1] < bm1[i - 1] { -1. } else { 0. }
    }).collect();
    let sd = carry(&sd2);
    let sch: Vec<bool> = (0..n).map(|i| i > 0 && sd[i] == 1. && sd[i - 1] == -1.).collect();
    let scl: Vec<bool> = (0..n).map(|i| i > 0 && sd[i] == -1. && sd[i - 1] == 1.).collect();
    let (hsch, lscl) = (since(&sch, h, true), since(&scl, l, false));
    let pv: Vec<f64> = (0..n).map(|i| if scl[i] && hsch[i].is_finite() { hsch[i] } else { 0. }).collect();
    let tv: Vec<f64> = (0..n).map(|i| if sch[i] && lscl[i].is_finite() { lscl[i] } else { 0. }).collect();
    let (pw, tw) = (when_pos(&pv), when_pos(&tv));
    let td: Vec<f64> = (0..n).map(|i| if h[i] > pw[i] { 1. } else if l[i] < tw[i] { -1. } else { 0. }).collect();
    (hilo, carry(&td))
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let hold = p.get("hold").copied().unwrap_or(0.).round() >= 1.;
    let (hilo, tr) = gann(x);
    let mut tgt = vec![0i8; x.c.len()];
    for i in 0..x.c.len() {
        let s = if x.c[i] > hilo[i] && tr[i] == 1. { 1 } else if x.c[i] < hilo[i] && tr[i] == -1. { -1 } else { 0 };
        tgt[i] = if s == 0 && hold && i > 0 { tgt[i - 1] } else { s };
    }
    Some(target_trades(x, &tgt, from, to, false, "state ended"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn swing_trend_and_hilo_states() {
        // a down leg, an up swing that clears the last peak, then a fall through the last trough
        let hs = [20., 19., 18., 17., 16., 17., 18., 19., 20., 21., 22., 21., 20., 19., 18., 17., 16., 15., 14., 13.];
        let r: Vec<Row> = hs.iter().enumerate().map(|(i, &h)| Row { ticker: "T".into(), date: format!("2001-01-{:02}", i + 1), open: h - 0.5, high: h, low: h - 1., close: h - 0.5, volume: None }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let (hilo, tr) = gann(&x);
        // HiLo2 on bar 3: close 16.5 < EMA3(L) of bar 2 (18) -> HLv = -1 -> HiLo = EMA3(H) of bar 3 = 19 + (17 - 19)/2 = 18
        assert!((hilo[3] - 18.).abs() < 1e-12);
        let t = run("gnmod_hilo_trend", &x, 0, r.len() - 1, &Params::new()).unwrap();
        for z in &t {
            let e = z.legs[0].i;
            assert!(if z.dir > 0. { x.c[e] > hilo[e] && tr[e] == 1. } else { x.c[e] < hilo[e] && tr[e] == -1. });
        }
        assert!(run("gnmod_x", &x, 0, 5, &Params::new()).is_none());
    }
}

// Instrument admission: 'Gann modify' MetaStock pack 2026-10-31 (titles lane K6).
// Admission only for the module REVIEWED lists (authors/titles-1030/K6w/k6_ref.py (header)).
// Runs on actual indices, ETFs and stocks, US and India (index_etf_or_stock, nifty50 basket, declared application).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "gnmod" && crate::books::gnmod::reviewed(def.id),
    label: "A MetaStock Expert for any security, with no named author and no printed test; indices, ETFs and US/India stocks are a declared application.",
    basis: "'Gann Systems + Info' MetaStock pack: MS70FORM.DTA (GANN-HiLo2, Gann-Swing2, Gann-Trend2), EXPERTS 'Gann modify', info.doc",
};
