//! Four % Model on daily closes, from two printed codes of the same rule:
//!  - Fred G. Schutzman, "JJMBook.Four%Model" TradeStation code (logic by Ned Davis, after Zweig), research share USB HDD 1/
//!    TradeStation Systems & Indicators/Fred G.Schutzman - JJMBook  Four % Model System for TS/Code.rar -> Code.txt (copy
//!    authors/batch-j/J1w/jjm4pct.txt): the default (always in after the first reversal, long and short).
//!  - "Zweig system for ms" MetaStock expert, research share USB HDD 1/MetaStock/@Systems & Indicators/@Misc Indicators & Systems/
//!    Zweig system for ms.rar -> Zweig system for ms.txt (copy authors/batch-j/J3w/zweig_4pct.txt): option p.src = 1 (long only).
//! Book id `tsfour`. Lane PD (A1 pool D, 13 Nov 2026), batch-j J1/J3 to-code rows; independent check authors/pd/w/pd_ref.py.
//! R4 #326 (lane X5, review authors/rev/PD.md B3): both codes are the Zweig / Ned Davis 4% model of the live mbsot_zweig4 (Schutzman's
//! header: 'Model was designed to be applied to a weekly chart of the Value Line Composite Index'); daily bars are a variant, not a
//! new method (as masmt_vl4). The card is marked "Duplicate of mbsot_zweig4" (NotTestable, no ADMISSION) and counted once; the two
//! code files are cited on mbsot_zweig4's gate row. run() is kept for the record and its test.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::tselhs::Book;

pub const REVIEWED: &[&str] = &["tsfour_close"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const DUP: &str = "Duplicate of mbsot_zweig4 (R4 #326): the Schutzman JJMBook.Four%Model TradeStation code and the Zweig system for MetaStock are the Zweig / Ned Davis Four Percent Model (a 4% move off the lowest / highest close since the last signal), which the code's own header says was designed for weekly Value Line closes; the same model is the live card mbsot_zweig4 (p.pct = 4, p.short = 1 for the long-and-short form). Daily bars are a variant, not a new method; counted once, not run as a separate strategy";
const NOTE: &str = "Duplicate of mbsot_zweig4 (R4 #326); not run twice. Readings kept for the record: [Declared readings] Default p.src = 0, Schutzman's TradeStation code as printed: LC = lowest close and HC = highest close, both starting at the first bar's close. Trend 0 (no trades yet) becomes up when (close - LC)/LC >= 4% and down when (HC - close)/HC >= 4% (the second test wins if both hold); this first change places no order. Then: up and (HC - close)/HC >= 4% -> 'Sell Short This Bar on close', trend down, LC = close; down and (close - LC)/LC >= 4% -> 'Buy This Bar on close', trend up, HC = close; after that LC and HC take today's close if it is a new low / high. The comment's weekly Value Line chart is the model's origin (Zweig); the code reads the chart's bars, so daily bars here are a declared application. Option p.src = 1, the 'Zweig system for ms' MetaStock expert as printed: state a = +1 / -1 / 0 (start 0); from +1 it turns -1 when close < 0.96 x the highest close since it turned +1; from -1 it turns +1 when close > 1.04 x the lowest close since it turned -1; from 0 it turns +1 when close > 1.04 x the lowest close so far, else -1 when close < 0.96 x the highest close so far. Enter long when a = 1 (also the first time), close long when a = -1; no short side. Both fill at the signal close (TradeStation 'this bar on close'; MetaStock's default trade price, delay 0). The code files print no test results. Runs on indices, ETFs and stocks, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "tsfour_close", name: "Four % Model on daily closes (Schutzman code): reverse at the close after a 4% move off the highest / lowest close", group: "TradeStation / MetaStock code: Four % Model (Schutzman; Zweig system for MS)", session: 0, dir: "Long/Short", status: Status::NotTestable(DUP),
        needs_volume: false, needs_vix: false, needs_series: &[], book: "tsfour", rules: &[
            "F|\"Inputs: perOffLo(4.00), { percent off lowest close } perOffHi(4.00); { percent off highest close }\" (Code.txt)",
            "E|\"else if trend = -1 and ((close-LC) / LC) >= (perOffLo / 100) then begin Buy This Bar on close; trend = +1; HC = close;\" (Code.txt)",
            "E|\"else if trend = +1 and ((HC-close) / HC) >= (perOffHi / 100) then begin Sell Short This Bar on close; trend = -1; LC = close;\" (Code.txt)",
            "X|Stop and reverse: each order closes the opposite position (no other exit in the code)",
            "F|Option p.src = 1, Zweig system for ms: \"a:=If(PREV=1,If(CLOSE<HighestSince(1,PREV<>1,CLOSE)*.96,-1,PREV),If(PREV=-1,If(CLOSE>LowestSince(1,PREV<>-1,CLOSE)*1.04,+1,PREV),If(PREV=0,If(CLOSE>Lowest(CLOSE)*1.04,+1,If(CLOSE<Highest(CLOSE)*.96,-1,PREV)),PREV))); enter long: a=1; Close Long: a=-1\""],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    reviewed(id).then_some(("tsfour_close", "Four % Model, daily closes (code)", "Duplicate of mbsot_zweig4"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    reviewed(id).then(|| vec![
        ParamDef { key: "pct", label: "Reversal percent off the lowest / highest close (perOffLo = perOffHi; 4 printed)", def: 4., min: 1., max: 10., step: 0.5 },
        ParamDef { key: "src", label: "Code: 0 = Schutzman TradeStation (long and short, from the first reversal), 1 = Zweig system for MS expert (long only, enters at the first +1)", def: 0., min: 0., max: 1., step: 1. }])
}

/// Desired side after each close: Schutzman (Some(+1/-1) at a reversal, None otherwise) or Zweig MS (the state a).
fn states(c: &[f64], pct: f64, zweig: bool) -> Vec<Option<f64>> {
    let mut out = vec![None; c.len()];
    if c.is_empty() { return out; }
    let k = pct / 100.;
    if zweig {
        let (mut a, mut lo_all, mut hi_all, mut ext) = (0i32, f64::MAX, f64::MIN, 0.0f64);
        for (t, &x) in c.iter().enumerate() {
            lo_all = lo_all.min(x); hi_all = hi_all.max(x);
            let prev = a;
            if prev == 1 { ext = ext.max(x); if x < ext * (1. - k) { a = -1; } }
            else if prev == -1 { ext = ext.min(x); if x > ext * (1. + k) { a = 1; } }
            else if x > lo_all * (1. + k) { a = 1; } else if x < hi_all * (1. - k) { a = -1; }
            if a != prev { ext = x; } // HighestSince / LowestSince start at the bar where the state changed
            out[t] = Some(if a == 1 { 1. } else if a == -1 { 0. } else { f64::NAN });
        }
    } else {
        let (mut lc, mut hc, mut trend) = (c[0], c[0], 0i32);
        for (t, &x) in c.iter().enumerate() {
            if trend == 0 {
                if (x - lc) / lc >= k { trend = 1; }
                if (hc - x) / hc >= k { trend = -1; }
            } else if trend == 1 && (hc - x) / hc >= k { out[t] = Some(-1.); trend = -1; lc = x; }
            else if trend == -1 && (x - lc) / lc >= k { out[t] = Some(1.); trend = 1; hc = x; }
            if x < lc { lc = x; }
            if x > hc { hc = x; }
        }
    }
    out
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let zweig = p.get("src").copied().unwrap_or(0.) >= 0.5;
    let pct = p.get("pct").copied().unwrap_or(4.);
    let st = states(&x.c, pct, zweig);
    let mut pos = Book::new();
    for i in from..=to {
        match st[i] {
            Some(d) if zweig && d == 0. => pos.exit(i, x.c[i], "a = -1"),
            Some(d) if d == 1. || d == -1. => pos.enter(d, i, x.c[i], "close"),
            _ => {}
        }
    }
    Some(pos.done(to, x.c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn four_percent_reversals() {
        // 100 -> 105 (trend up, no order) -> 100.7 (4.1% off 105: short) -> 105 (4.3% off 100.7: long)
        let c = [100., 105., 100.7, 102., 105.];
        let s = states(&c, 4., false);
        assert_eq!(s, vec![None, None, Some(-1.), None, Some(1.)]);
        let z = states(&c, 4., true);
        assert_eq!(z[1], Some(1.)); assert_eq!(z[2], Some(0.)); assert_eq!(z[4], Some(1.));
        let r = super::super::ctm::tests::walk(3000);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("tsfour_close", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() > 5 && t.iter().any(|z| z.dir > 0.) && t.iter().any(|z| z.dir < 0.));
        let mut p = Params::new(); p.insert("src".into(), 1.);
        let t = run("tsfour_close", &x, 0, r.len() - 1, &p).unwrap();
        assert!(t.len() > 5 && t.iter().all(|z| z.dir > 0.));
        assert!(family_of("tsfour_close").is_some());
    }
}

// No ADMISSION: duplicate of mbsot_zweig4 (R4 #326).
