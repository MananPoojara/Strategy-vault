//! Jose Silva, MACDH Divergence Kit for MetaStock (metastocktools.com, 2004-2006): the 'Divergence - MACDH Minor' signals
//! traded by his 'MACDH Profit Long %' and 'Profit Short %' system examples (research share USB HDD 1/MetaStock/@Systems &
//! Indicators/MACDH Divergence Kit (metastocktools.com)/Documentation/Indicators.pdf, PDF p.32-40). Book id `jsmdk`. Lane
//! K9 (titles loop, 1 Nov 2026), to-code row 143; rule text authors/batch-c/C3w/macdh.txt; independent check
//! authors/titles-1030/K9w/k9_ref.py (header).
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::mkits::{ema, hhv, llv, macd};
use super::msws::{simulate, Exec, Sig};

/// Admitted ids (authors/titles-1030/K9w/k9_ref.py (header)).
pub const REVIEWED: &[&str] = &["jsmdk_minor_div"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "The author labels both profit formulas 'System development example - do not trade!'; shown as his example, without a claim. [Declared readings] x = MACD() - Mov(MACD(),9,E), MetaStock MACD() = exponential averages of the close at 15% and 7.5% (12/26 bars), averages started from the simple average of their first values. A peak is flagged on the bar after it: yesterday's x is the highest x of the last 3 bars (brkPds 3, Master Control default) and above 0 (method 1 'Original'; method 2 uses the 10-day exponential average of x instead of 0). Troughs mirror. Pk1/Pk2 = x at the last two peaks, PkPr1/PkPr2 = yesterday's close at those flags (price field 1; 2 = highs for peaks and lows for troughs). Bullish minor divergence = a trough flag with Tr1 > Tr2 and TrPr1 < TrPr2; bearish = a peak flag with Pk1 < Pk2 and PkPr1 > PkPr2. Profit Long %: buy on bullish, sell on bearish; Profit Short %: short on bearish, cover on bullish; run together they are stop and reverse (side option: long or short only). Entry and exit at the signal day's close (his defaults: Entry/Exit Price = Close, delay 0). Not used: his position sizing and risk normalisation, the 0.2% cost input (vault cost convention), SmartStop (a compiled DLL), the Major/Triple variants. Daily bars; indices, ETFs and US/India stocks are a declared application; no results printed.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "jsmdk_minor_div", name: "MACDH minor divergence (Silva): long on a bullish trough divergence, short on a bearish peak divergence",
        group: "Silva MACDH Divergence Kit", session: 0, dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "jsmdk", rules: &[
            "F|\"x:=MACD()-Mov(MACD(),x,E); method:=If(method=1,0,Mov(x,10,E)); pk:=Ref(x,-1)=HHV(x,LastValue(brkPds)) AND Ref(x,-1)>method; tr:=Ref(x,-1)=LLV(x,LastValue(brkPds)) AND Ref(x,-1)<method;\" (Divergence - MACDH Minor)",
            "E|\"bullish:=Tr AND Tr1>Tr2 AND TrPr1<TrPr2;\" - \"{* Entry Long *} x:=Fml(\"Divergence - MACDH Minor\"); entry:=x=1;\" (Profit Long %)",
            "X|\"bearish:=Pk AND Pk1<Pk2 AND PkPr1>PkPr2;\" - \"{* Exit Long *} exit:=x=-1;\"; Profit Short %: \"entry:=x=-1; ... exit:=x=1;\""],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == "jsmdk_minor_div").then_some(("jsmdk_minor_div", "Silva MACDH divergence", "minor, brkPds 3"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![
        p("brk", "Peak/trough breakout lookback (Master Control 3)", 3., 2., 20., 1.),
        p("method", "Divergence method: 1 = Original (zero line), 2 = Alternative (10-day EMA of the histogram)", 1., 1., 2., 1.),
        p("price", "Price field: 1 = Close, 2 = High/Low", 1., 1., 2., 1.),
        p("side", "Sides: 0 = long and short (both profit formulas), 1 = long only, -1 = short only", 0., -1., 1., 1.),
    ])
}

/// (bullish, bearish) minor divergence flags.
pub fn minor(x: &Ctx, brk: usize, alt: bool, hl: bool) -> (Vec<bool>, Vec<bool>) {
    let m = macd(&x.c);
    let s = ema(&m, 9);
    let h: Vec<f64> = m.iter().zip(&s).map(|(a, b)| a - b).collect();
    let thr = if alt { ema(&h, 10) } else { vec![0.; h.len()] };
    let (hh, ll) = (hhv(&h, brk), llv(&h, brk));
    let n = h.len();
    let (mut bull, mut bear) = (vec![false; n], vec![false; n]);
    let (mut pk, mut tr): (Vec<(f64, f64)>, Vec<(f64, f64)>) = (vec![], vec![]); // (histogram, price) at the flags
    for i in 1..n {
        let v = h[i - 1];
        if v == hh[i] && v > thr[i] {
            pk.push((v, if hl { x.h[i - 1] } else { x.c[i - 1] }));
            if let [.., a, b] = pk.as_slice() { bear[i] = b.0 < a.0 && b.1 > a.1; }
        }
        if v == ll[i] && v < thr[i] {
            tr.push((v, if hl { x.l[i - 1] } else { x.c[i - 1] }));
            if let [.., a, b] = tr.as_slice() { bull[i] = b.0 > a.0 && b.1 < a.1; }
        }
    }
    (bull, bear)
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d);
    let (bull, bear) = minor(x, g("brk", 3.).round().max(2.) as usize, g("method", 1.) >= 1.5, g("price", 1.) >= 1.5);
    let side = g("side", 0.).round();
    let none = vec![false; bull.len()];
    let (lng, sht) = (side >= 0., side <= 0.);
    let sig = Sig { el: if lng { bull.clone() } else { none.clone() }, xl: if lng { bear.clone() } else { none.clone() },
                    es: if sht { bear } else { none.clone() }, xs: if sht { bull } else { none } };
    Some(simulate(x, from, to, &sig, Exec::default()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn divergences_alternate_the_position() {
        let mut seed = 12345u64;
        let mut c = 100.;
        let r: Vec<Row> = (0..3000).map(|i| { seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            c *= 1. + ((seed >> 33) as f64 / (1u64 << 31) as f64 - 0.5) * 0.04;
            Row { ticker: "T".into(), date: format!("{:05}", i), open: c, high: c * 1.01, low: c * 0.99, close: c, volume: None } }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let (bull, bear) = minor(&x, 3, false, false);
        assert!(bull.iter().any(|b| *b) && bear.iter().any(|b| *b));
        assert!(bull.iter().zip(&bear).all(|(a, b)| !(*a && *b)));
        let t = run("jsmdk_minor_div", &x, 0, r.len() - 1, &Params::new()).unwrap();
        for w in t.windows(2) { assert_eq!(w[0].exit_i, w[1].legs[0].i); assert!(w[0].dir != w[1].dir); }
        let lo = run("jsmdk_minor_div", &x, 0, r.len() - 1, &Params::from([("side".to_string(), 1.)])).unwrap();
        assert!(lo.iter().all(|z| z.dir > 0.));
        assert!(run("jsmdk_x", &x, 0, 5, &Params::new()).is_none());
    }
}

// Instrument admission: Jose Silva (MACDH Divergence Kit) 2026-11-01 (titles lane K9).
// Admission only for the module REVIEWED lists (authors/titles-1030/K9w/k9_ref.py (header)).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "jsmdk" && reviewed(def.id),
    label: "Jose Silva's MACDH divergence profit formulas are a 'do not trade' system development example with no printed test; runs on indices, ETFs and stocks, US and India, are a declared application.",
    basis: "Jose Silva, MACDH Divergence Kit (metastocktools.com), Documentation/Indicators.pdf PDF p.32-40: Divergence - MACDH Master Control, Minor, Profit Long %, Profit Short %",
};
