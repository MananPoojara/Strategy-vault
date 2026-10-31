//! Compuvision Australia, TradeSim Application Note AN-2 "Implementing Volatility Trailing Stops the Correct Way" (20 Sep 2004;
//! research share USB HDD 1/MetaStock/@Systems & Indicators/TradeSim 4.2.2 Enterprise Edition + Manuals (compuvision.com.au)/
//! AN2 - Trailing Stops.pdf, p.6-8; text authors/batch-e/E4w/tradesim_an2.txt). Book id `cvan2`. Lane K10 (titles loop,
//! 1 Nov 2026), to-code row 174; independent check authors/titles-1030/K10w/k10_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::lc3::Pos;
use super::msws::{ema, wild};

/// Admitted ids (authors/titles-1030/K10w/k10_ref.py (header)).
pub const REVIEWED: &[&str] = &["cvan2_macd_vts"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] The note's RecordTrades example, MetaStock formulas as printed: enter long at the close when MACD() crosses above its 9-period exponential average; exit at the close of a day whose low is at or below yesterday's trailing band. Band = If(L <= PREV, Stop, If(Stop > PREV, Stop, PREV)) with Stop = close - 3 x ATR(10), worked bar by bar over the whole history (it restarts whenever the low touches it, and is not reset at entry); it starts at the first defined Stop. MACD() = MetaStock's built-in MACD (exponential averages with the fixed constants 0.15 and 0.075, started at the first close), Mov(..,9,E) = MetaStock exponential average (started at the first value), ATR = Wilder's average true range (the note says so). One position; the exit is checked from the day after entry; no new long on the day a long closes, so a new entry needs a new cross. InitialStop = the band at entry, which the trailing exit already covers, so it is not modelled separately. Options: the note's printed short example (p.side = -1: MACD crossing below its signal, Stop = close + 3 x ATR(10), band If(H >= PREV, Stop, If(Stop < PREV, Stop, PREV)), exit when the high reaches yesterday's band), the ARC (3.0; the note suggests 1.8-3.0) and the ATR period. A vendor sample with no instrument and no results; indices, ETFs and US/India stocks are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "cvan2_macd_vts", name: "TradeSim AN-2: MACD crosses its signal line, out on a 3 x ATR(10) volatility trailing stop", group: "TradeSim application notes",
        session: 0, dir: "Long", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "cvan2", rules: &[
            "F|\"Period:=10; ARC:=3.0; Stop:=C-ARC*ATR(Period); BandLong:=If(L<=PREV,Stop,If(Stop>PREV,Stop,PREV));\" (p.6-7)",
            "E|\"EntryTrigger := Cross(MACD(),Mov(MACD(),9,E)); EntryPrice := CLOSE;\" (p.8)",
            "X|\"ExitTrigger := (L <= ref(BandLong,-1)); ExitPrice := CLOSE; InitialStop := BandLong;\" (p.8); short side: \"EntryTrigger := Cross(Mov(MACD(),9,E),MACD()) ... ExitTrigger := (H >= ref(BandShort,-1))\""],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == "cvan2_macd_vts").then_some(("cvan2_macd_vts", "TradeSim MACD + volatility stop", "3 x ATR(10)"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("side", "Side: 1 = long (default), -1 = the note's short example", 1., -1., 1., 2.),
        p("arc", "Average range constant (3.0)", 3., 1., 5., 0.1), p("n", "ATR period (10)", 10., 2., 50., 1.)])
}

/// MetaStock exponential average with a fixed constant, started at the first close.
fn ema_k(v: &[f64], k: f64) -> Vec<f64> {
    let mut out = vec![f64::NAN; v.len()];
    for i in 0..v.len() { out[i] = if i == 0 { v[0] } else { out[i - 1] + k * (v[i] - out[i - 1]) }; }
    out
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap().def);
    let (h, l, c, n) = (&x.h, &x.l, &x.c, x.c.len());
    let d = if g("side") < 0. { -1. } else { 1. };
    let (e1, e2) = (ema_k(c, 0.15), ema_k(c, 0.075));
    let macd: Vec<f64> = (0..n).map(|i| e1[i] - e2[i]).collect();
    let sig = ema(&macd, 9);
    let tr: Vec<f64> = (0..n).map(|i| if i == 0 { f64::NAN } else { (h[i] - l[i]).max((h[i] - c[i - 1]).abs()).max((l[i] - c[i - 1]).abs()) }).collect();
    let atr = wild(&tr, g("n").round().max(1.) as usize);
    let arc = g("arc");
    let mut band = vec![f64::NAN; n];
    for i in 0..n {
        let st = c[i] - d * arc * atr[i];
        if !st.is_finite() { continue; }
        let pv = if i > 0 { band[i - 1] } else { f64::NAN };
        band[i] = if !pv.is_finite() { st } else if d > 0. { if l[i] <= pv || st > pv { st } else { pv } } else if h[i] >= pv || st < pv { st } else { pv };
    }
    let cross = |a: &[f64], b: &[f64], i: usize| i > 0 && a[i] > b[i] && a[i - 1] <= b[i - 1];
    let mut pos = Pos::new();
    for i in from..=to {
        if let Some(e) = pos.entry_bar() {
            if i > e && band[i - 1].is_finite() && (if d > 0. { l[i] <= band[i - 1] } else { h[i] >= band[i - 1] }) {
                pos.exit(i, c[i], "volatility trailing stop"); continue;
            }
        }
        if pos.side() == 0. && (if d > 0. { cross(&macd, &sig, i) } else { cross(&sig, &macd, i) }) { pos.enter(d, i, c[i], "close"); }
    }
    Some(pos.done(to, c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn macd_entries_and_band_exits_at_the_close() {
        let r: Vec<Row> = (0..800).map(|i| { let f = i as f64; let c = 100. + 12. * (f / 23.).sin() + 3. * (f / 4.1).cos() + 0.02 * f;
            Row { ticker: "T".into(), date: format!("{:05}", i), open: c, high: c + 1.5, low: c - 1.5, close: c, volume: None } }).collect();
        let x = Ctx::new(&r, &r, &[]);
        for s in [1., -1.] {
            let t = run("cvan2_macd_vts", &x, 0, r.len() - 1, &Params::from([("side".to_string(), s)])).unwrap();
            assert!(t.len() > 4 && t.iter().all(|z| z.dir == s));
            for z in &t { assert_eq!(z.legs[0].px, x.c[z.legs[0].i]); assert!(z.exit_i > z.legs[0].i); }
            for w in t.windows(2) { assert!(w[1].legs[0].i > w[0].exit_i); }
        }
        assert!(run("cvan2_x", &x, 0, 5, &Params::new()).is_none());
    }
}

// Instrument admission: Compuvision TradeSim AN-2 2026-11-01 (titles lane K10).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "cvan2" && reviewed(def.id),
    label: "A TradeSim application-note sample with no instrument and no results. Runs on indices, ETFs and stocks, US and India, are a declared application.",
    basis: "Compuvision Australia, TradeSim AN-2 Implementing Volatility Trailing Stops (2004) p.6-8; independent check authors/titles-1030/K10w/k10_ref.py",
};
