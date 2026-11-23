//! Emini-Watch.com, "Better TRIN Indicators: Installation & EasyLanguage Code" (version 18 September 2007; research share USB HDD 2/tRADING KNO-2/... Better TRIN Indicators.pdf,
//! text layer, read in full: the "Better TRIN System" EasyLanguage listing). Book id `ewtrin`. The listing: Value3 = -(ln TRIN_NYSE + ln TRIN_NASDAQ) x 100, Value4 = running sum,
//! Value6 = Value4 - Average(Value4, 2 x Length), Value7/Value8 = Highest/Lowest(Value6, Length), Value9 = (Value6-Value8)/(Value7-Value8) - 0.5 + 0.5 x Value9[1] (clamped +-0.9999),
//! Value10 = 0.25 ln((1+Value9)/(1-Value9)) + 0.5 x Value10[1], Value11 = 100 x Value10; five entry types A-E and one exit, all at the bar close. Lane K1 (23 Nov 2026, queue T2-249).
//! Independent check authors/conv-1123/K1/ewtrin_ref.py.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "ewtrin";
pub const REVIEWED: &[&str] = &["ewtrin_us", "ewtrin_in"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared adaptation, R4 #340] The listing is written for the S&P e-mini daily with two TRIN feeds, NYSE ($TRIN) and NASDAQ ($TRINQ); the vault holds no NASDAQ TRIN, so the rule runs on the one TRIN it has per market: `ewtrin_us` = NYSE TRIN on the US indices SPX, NDX and DJI, `ewtrin_in` = NSE TRIN on NIFTY 50 and NIFTY BANK; indices only, never single stocks or ETFs. The listing sums the logs of the two TRINs and the oscillator is scale-free (it normalises the running sum between its 5-bar highest and lowest), so one TRIN counted twice and one TRIN alone give the same signals; the second feed's information is the only thing lost. The document prints no results; nothing is compared. UNVERIFIED. All five entry types A-E are on (the listing's defaults). [Declared readings, each an unstated detail] (1) The listing's DoubleUp input (two contracts after a losing trade) is a position size, not a signal, and one unit is traded; (2) bars with no TRIN reading are skipped, so every look-back counts TRIN days; (3) the first 20 TRIN days only warm up the running sum and its smoothing, no orders; (4) orders fill in the listing's order (A to E, buy then sell, then the exit) at the bar close and a new entry reverses an open position; an entry in the direction already held is ignored.";

pub fn catalogue() -> Vec<StrategyDef> {
    let mk = |id: &'static str, name: &'static str, series: &'static [&'static str]| StrategyDef { id, name, group: "Emini-Watch Better TRIN system", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: series, book: "ewtrin",
        rules: &[
            "E|\"Better TRIN System\" (EasyLanguage listing): Value3 = -(Log(TRIN)+Log(TRINQ))*100; Value4 = Value4[1]+Value3; Value5 = Average(Value4, Length*2); Value6 = Value4-Value5; Value7 = Highest(Value6, Length); Value8 = Lowest(Value6, Length); Value9 = ((Value6-Value8)/(Value7-Value8)-0.5) + 0.5*Value9[1], clamped to +-0.9999; Value10 = 0.25*Log((1+Value9)/(1-Value9)) + 0.5*Value10[1]; Value11 = Value10*100; Length = 5",
            "E|Type A: Buy when Value11 < Value11[1] and Value11[1] >= Value11[2] and Value11[1] <= 0 (sell short mirrored); Type B: Value11 < [1] < [2], [2] >= [3], [2] <= 0; Type C: Value11 < [1] < [2] < [3], [3] >= [4], [3] <= 0; Type D: Value11 > [1], [1] < [2], [2] < 0; Type E: Value11 < [1], [1] > [2], [2] < [3], Value11 < 0; each \"this bar close\"; sell short = the mirror image",
            "X|\"If MarketPosition = 1 and BarsSinceEntry > 1 and Value11 < Value11[1] and Value11[1] >= Value11[2] and Value11[1] >= 0 then Sell this bar close\" (and the mirror for shorts)"],
        notes: NOTE, claim: None };
    vec![mk("ewtrin_in", "Better TRIN oscillator system (NSE TRIN, NSE indices): five turn entries A-E at the close, exit on the turn after a zero-side peak", &["NSE_TRIN"]),
         mk("ewtrin_us", "Better TRIN oscillator system (NYSE TRIN, US indices): five turn entries A-E at the close, exit on the turn after a zero-side peak", &["NYSE_TRIN"])]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id { "ewtrin_in" | "ewtrin_us" => Some(("ewtrin", "Emini-Watch Better TRIN system", "length 5, entry types A-E")), _ => None }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    match id {
        "ewtrin_in" | "ewtrin_us" => Some(vec![p("length", "Length (listing 5; smoothing window is 2 x Length)", 5., 2., 20., 1.),
            p("typeA", "Entry type A on (1) / off (0)", 1., 0., 1., 1.), p("typeB", "Entry type B on (1) / off (0)", 1., 0., 1., 1.), p("typeC", "Entry type C on (1) / off (0)", 1., 0., 1., 1.),
            p("typeD", "Entry type D on (1) / off (0)", 1., 0., 1., 1.), p("typeE", "Entry type E on (1) / off (0)", 1., 0., 1., 1.)]),
        _ => None,
    }
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let s = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| s.iter().find(|d| d.key == k).unwrap().def);
    let tr = x.series(if id == "ewtrin_in" { "NSE_TRIN" } else { "NYSE_TRIN" });
    let Some(tr) = tr else { return Some(vec![]) };
    let n = g("length").round().max(2.0) as usize;
    let on = [g("typeA") > 0.5, g("typeB") > 0.5, g("typeC") > 0.5, g("typeD") > 0.5, g("typeE") > 0.5];
    let c = &x.c;
    // state over TRIN days (k counts processed days)
    let (mut v4, mut v9, mut v10) = (0.0f64, 0.0f64, 0.0f64);
    let mut v4h: Vec<f64> = vec![]; let mut v6h: Vec<f64> = vec![]; let mut v11: Vec<f64> = vec![];
    let mut pos = 0i8; let mut entry: Option<(usize, f64, usize)> = None; // (bar, price, k at entry)
    let mut out: Vec<Trade> = vec![];
    let close = |out: &mut Vec<Trade>, e: (usize, f64, usize), dir: f64, i: usize, why: &str| {
        out.push(Trade { dir, legs: vec![Leg { i: e.0, px: e.1, w: 1. }], exit_i: i, exit_px: c[i], reason: why.into(), entry_kind: "close", signal_i: i, open: false, exits: vec![] });
    };
    let mut k = 0usize;
    for i in 0..=to {
        let t = tr[i];
        if !(t.is_finite() && t > 0.0) { continue; }
        let v3 = -(2.0 * t.ln()) * 100.0; // the listing's two feeds are the same TRIN here
        v4 += v3; v4h.push(v4);
        let m = 2 * n;
        let v6 = if v4h.len() >= m { v4 - v4h[v4h.len() - m..].iter().sum::<f64>() / m as f64 } else { f64::NAN };
        v6h.push(v6);
        if v6h.len() >= n && v6h[v6h.len() - n..].iter().all(|z| z.is_finite()) {
            let w = &v6h[v6h.len() - n..];
            let (hi, lo) = (w.iter().cloned().fold(f64::MIN, f64::max), w.iter().cloned().fold(f64::MAX, f64::min));
            if hi != lo { v9 = ((v6 - lo) / (hi - lo) - 0.5) + 0.5 * v9; }
            v9 = v9.clamp(-0.9999, 0.9999);
            v10 = 0.25 * ((1.0 + v9) / (1.0 - v9)).ln() + 0.5 * v10;
        }
        v11.push(v10 * 100.0);
        let kk = k; k += 1;
        if i < from || kk < 20 { continue; }
        let a = |j: usize| v11[kk - j];
        let (v0, v1, v2, v3_, v4_) = (a(0), a(1), a(2), a(3), a(4));
        let pos_open = pos; let ent_k = entry.map(|e| e.2).unwrap_or(0);
        // orders in listing order: (buy, sell) for each of A..E, then the exit
        let buys = [v0 < v1 && v1 >= v2 && v1 <= 0.0,
                    v0 < v1 && v1 < v2 && v2 >= v3_ && v2 <= 0.0,
                    v0 < v1 && v1 < v2 && v2 < v3_ && v3_ >= v4_ && v3_ <= 0.0,
                    v0 > v1 && v1 < v2 && v2 < 0.0,
                    v0 < v1 && v1 > v2 && v2 < v3_ && v0 < 0.0];
        let sells = [v0 > v1 && v1 <= v2 && v1 >= 0.0,
                     v0 > v1 && v1 > v2 && v2 <= v3_ && v2 >= 0.0,
                     v0 > v1 && v1 > v2 && v2 > v3_ && v3_ <= v4_ && v3_ >= 0.0,
                     v0 < v1 && v1 > v2 && v2 > 0.0,
                     v0 > v1 && v1 < v2 && v2 > v3_ && v0 > 0.0];
        let mut orders: Vec<i8> = vec![];
        for q in 0..5 { if on[q] { if buys[q] { orders.push(1); } if sells[q] { orders.push(-1); } } }
        if pos_open == 1 && kk > ent_k + 1 && v0 < v1 && v1 >= v2 && v1 >= 0.0 { orders.push(2); }
        if pos_open == -1 && kk > ent_k + 1 && v0 > v1 && v1 <= v2 && v1 <= 0.0 { orders.push(3); }
        for o in orders {
            match (o, pos) {
                (1, 1) | (-1, -1) => {}
                (1, -1) | (-1, 1) => { let e = entry.take().unwrap(); close(&mut out, e, pos as f64, i, "reversal"); pos = o; entry = Some((i, c[i], kk)); }
                (1, 0) | (-1, 0) => { pos = o; entry = Some((i, c[i], kk)); }
                (2, 1) => { let e = entry.take().unwrap(); close(&mut out, e, 1.0, i, "oscillator turned down from a peak at or above zero"); pos = 0; }
                (3, -1) => { let e = entry.take().unwrap(); close(&mut out, e, -1.0, i, "oscillator turned up from a trough at or below zero"); pos = 0; }
                _ => {}
            }
        }
    }
    if let Some(e) = entry {
        out.push(Trade { dir: pos as f64, legs: vec![Leg { i: e.0, px: e.1, w: 1. }], exit_i: to, exit_px: c[to], reason: "open".into(), entry_kind: "close", signal_i: to, open: true, exits: vec![] });
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn runs_with_trin_and_rejects_other_ids() {
        let r: Vec<Row> = (0..400).map(|i| Row { ticker: "NIFTY 50".into(), date: format!("2010-{:03}", i), open: 100., high: 100., low: 100., close: 100. + (i as f64 * 0.1).sin() * 5.0, volume: None }).collect();
        let mut x = Ctx::new(&r, &r, &[]);
        x.extra.insert("NSE_TRIN".into(), (0..400).map(|i| 1.0 + 0.5 * (i as f64 * 0.17).sin()).collect());
        assert!(run("ewtrin_x", &x, 0, 399, &Params::default()).is_none());
        let t = run("ewtrin_in", &x, 0, 399, &Params::default()).unwrap();
        assert!(!t.is_empty());
        for z in &t { assert!(z.exit_i >= z.legs[0].i); }
    }
}

// Instrument admission: indices only (never single stocks or ETFs), declared adaptation.
pub fn index_only() -> &'static str { "Emini-Watch Better TRIN system: written for the S&P e-mini daily; run on actual indices only (NSE TRIN on NIFTY 50 / NIFTY BANK, NYSE TRIN on SPX / NDX / DJI) as a declared adaptation (R4 #340); UNVERIFIED" }
