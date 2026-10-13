//! Tim Tillson, "Better Moving Averages and Oscillators" (CedarCreek, MetaStock 6.5 article; the T3 / ILRS paper).
//! Book id "tilma". Source copy: Books/Tim Tillson/Better Moving Averages and Oscillators.doc (Word text; the
//! "Trading Results" section and Appendix I MetaStock code). Notes and the oracle: library/tilma/.
//! One system, five moving averages: take a moving average of the close, its 1-bar point change d1 = ROC(MA,1,points);
//! "A long position was entered at bottoms, and closed at tops, of the derivative. No shorts were taken."
#![allow(dead_code)]
use crate::connors::*;

pub const BOOK: &str = "tilma";

const ENTRY: &str = "E|\"res:=Mov(C,opt1,E); d1:=ROC(res,1,points); d1 > Ref(d1,-1) AND Ref(d1,-1) < Ref(d1,-2)\" (enter long; the EMA code printed in 'Trading Results'; the other averages replace res) = d1 turns up from a bottom";
const EXIT: &str = "X|\"A long position was entered at bottoms, and closed at tops, of the derivative.\" = close long when d1 < Ref(d1,-1) AND Ref(d1,-1) > Ref(d1,-2) (the mirror of the printed entry code)";
const FILL: &str = "Fills: the article tests in the MetaStock 6.5 System Tester and does not print the trade price/delay; the vault fills at the close of the signal bar (System Tester default: trade price close, delay 0) [book silent]. p.fill_next=1 = next open (labelled option). Article test: NDX 7/19/93-6/30/97, 4% interest on idle cash, 0.1% cost per entry and exit, opt1 optimised 2-10; the defaults are the printed optimal lengths. India: instrument swap only.";

const SMA_R: &[&str] = &["F|Moving average: Mov(C,opt1,S), simple; optimal opt1 = 8 (table)", ENTRY, EXIT];
const EMA_R: &[&str] = &["F|Moving average: Mov(C,opt1,E), exponential; optimal opt1 = 8 (table)", ENTRY, EXIT];
const DEMA_R: &[&str] = &["F|Moving average: DEMA = 2 x EMA(n) - EMA(EMA(n)) (MetaStock Dema); optimal opt1 = 7 (table)", ENTRY, EXIT];
const ILRS_R: &[&str] = &[
    "F|Moving average: ILRS = \"Cum(LinRegSlope(P,periods))+start\" (Appendix I); its 1-bar change is LinRegSlope(C, n) (the constant of integration cancels, so no look-ahead); optimal opt1 = 7 (table)",
    ENTRY, EXIT];
const T3_R: &[&str] = &[
    "F|Moving average: T3 (Appendix I): e1..e6 = EMA of EMA (periods); c1 = -a^3, c2 = 3a^2 + 3a^3, c3 = -6a^2 - 3a - 3a^3, c4 = 1 + 3a + a^3 + 3a^2; T3 = c1 e6 + c2 e5 + c3 e4 + c4 e3; optimal \"5 (volume = .7)\"",
    ENTRY, EXIT];

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    let d = |id, name, rules, claim| StrategyDef {
        id, name, group: "Trading Results (NDX 1993-97)", session: 0, dir: "Long", status: Live, needs_volume: false, needs_vix: false,
        needs_series: &[], book: BOOK, rules, notes: FILL, claim,
    };
    vec![
        d("tilma_t3", "T3(5, 0.7) slope bottoms/tops", T3_R, claim("-", "-", "-", "-", "Best APR 33.59% (NDX 7/19/93-6/30/97, MetaStock, 4% interest, 0.1% costs)")),
        d("tilma_dema", "DEMA(7) slope bottoms/tops", DEMA_R, claim("-", "-", "-", "-", "Best APR 28.81% (NDX 7/19/93-6/30/97)")),
        d("tilma_ema", "EMA(8) slope bottoms/tops", EMA_R, claim("-", "-", "-", "-", "Best APR 28.17% (NDX 7/19/93-6/30/97)")),
        d("tilma_sma", "SMA(8) slope bottoms/tops", SMA_R, claim("-", "-", "-", "-", "Best APR 20.20% (NDX 7/19/93-6/30/97)")),
        d("tilma_ilrs", "ILRS(7) slope bottoms/tops", ILRS_R, claim("-", "-", "-", "-", "Best APR 19.02% (NDX 7/19/93-6/30/97)")),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let f = ("tilma_slope", "Tillson MA slope turns (T3 article)");
    Some(match id {
        "tilma_t3" => (f.0, f.1, "T3"), "tilma_dema" => (f.0, f.1, "DEMA"), "tilma_ema" => (f.0, f.1, "EMA"),
        "tilma_sma" => (f.0, f.1, "SMA"), "tilma_ilrs" => (f.0, f.1, "ILRS"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let n = |v: f64| p("n", "Moving average length opt1 (article tested 2-10)", v, 2., 10., 1.);
    let fill = p("fill_next", "0 = fill at the signal close [book silent]; 1 = next open [labelled option]", 0., 0., 1., 1.);
    Some(match id {
        "tilma_t3" => vec![n(5.), p("a", "T3 volume factor a ('Hot')", 0.7, 0., 2., 0.1), fill],
        "tilma_dema" | "tilma_ilrs" => vec![n(7.), fill],
        "tilma_ema" | "tilma_sma" => vec![n(8.), fill],
        _ => return None,
    })
}

/// MetaStock-style EMA: alpha = 2/(n+1), seeded with the first valid value.
pub fn ema(v: &[f64], n: usize) -> Vec<f64> {
    let a = 2. / (n as f64 + 1.);
    let mut out = vec![f64::NAN; v.len()];
    let mut prev = f64::NAN;
    for (i, &x) in v.iter().enumerate() {
        if x.is_nan() { continue; }
        prev = if prev.is_nan() { x } else { prev + a * (x - prev) };
        out[i] = prev;
    }
    out
}

/// Least-squares slope of the last n values (MetaStock LinRegSlope).
pub fn linreg_slope(v: &[f64], n: usize) -> Vec<f64> {
    let mut out = vec![f64::NAN; v.len()];
    let nf = n as f64;
    let sx = nf * (nf - 1.) / 2.;
    let sxx = (nf - 1.) * nf * (2. * nf - 1.) / 6.;
    for i in n.saturating_sub(1)..v.len() {
        if i + 1 < n { continue; }
        let w = &v[i + 1 - n..=i];
        if w.iter().any(|x| x.is_nan()) { continue; }
        let sy: f64 = w.iter().sum();
        let sxy: f64 = w.iter().enumerate().map(|(k, y)| k as f64 * y).sum();
        out[i] = (nf * sxy - sx * sy) / (nf * sxx - sx * sx);
    }
    out
}

pub fn t3(c: &[f64], n: usize, a: f64) -> Vec<f64> {
    let e1 = ema(c, n); let e2 = ema(&e1, n); let e3 = ema(&e2, n); let e4 = ema(&e3, n); let e5 = ema(&e4, n); let e6 = ema(&e5, n);
    let (c1, c2, c3, c4) = (-a * a * a, 3. * a * a + 3. * a * a * a, -6. * a * a - 3. * a - 3. * a * a * a, 1. + 3. * a + a * a * a + 3. * a * a);
    (0..c.len()).map(|i| c1 * e6[i] + c2 * e5[i] + c3 * e4[i] + c4 * e3[i]).collect()
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let c = &x.c;
    let n = g("n") as usize;
    let d1: Vec<f64> = if id == "tilma_ilrs" {
        linreg_slope(c, n)
    } else {
        let m = match id {
            "tilma_sma" => sma(c, n),
            "tilma_ema" => ema(c, n),
            "tilma_dema" => { let e = ema(c, n); let ee = ema(&e, n); e.iter().zip(&ee).map(|(a, b)| 2. * a - b).collect() }
            _ => t3(c, n, g("a")),
        };
        (0..m.len()).map(|i| if i >= 1 { m[i] - m[i - 1] } else { f64::NAN }).collect()
    };
    let ok = |i: usize| i >= 2 && d1[i].is_finite() && d1[i - 1].is_finite() && d1[i - 2].is_finite();
    // Comparisons ignore floating-point noise (1e-9 x price): mathematically equal slopes are not a turn.
    let gt = |a: f64, b: f64, i: usize| a > b + 1e-9 * c[i].abs();
    let bottom = |i: usize| ok(i) && gt(d1[i], d1[i - 1], i) && gt(d1[i - 2], d1[i - 1], i);
    let top = |i: usize| ok(i) && gt(d1[i - 1], d1[i], i) && gt(d1[i - 1], d1[i - 2], i);
    let next = g("fill_next") >= 0.5;
    let to = to.min(c.len() - 1);
    let (mut out, mut cur): (Vec<Trade>, Option<Trade>) = (vec![], None);
    for i in from..=to {
        if let Some(t) = cur.as_ref() {
            if top(i) && i > t.signal_i {
                let (k, px) = if next { (i + 1, if i < to { x.o[i + 1] } else { f64::NAN }) } else { (i, c[i]) };
                if px.is_nan() { continue; }
                let mut t = cur.take().unwrap();
                (t.exit_i, t.exit_px, t.signal_i, t.reason) = (k, px, i, "d1 top".into());
                out.push(t);
            }
            continue;
        }
        if out.last().is_some_and(|t| t.exit_i > i) { continue; }
        if bottom(i) {
            let (k, px, kind) = if next { if i >= to { continue; } (i + 1, x.o[i + 1], "open") } else { (i, c[i], "close") };
            cur = Some(Trade { dir: 1., legs: vec![Leg { i: k, px, w: 1. }], exit_i: k, exit_px: f64::NAN, reason: String::new(),
                entry_kind: kind, signal_i: i, open: false, exits: vec![] });
        }
    }
    if let Some(mut t) = cur {
        (t.exit_i, t.exit_px, t.reason, t.signal_i, t.open) = (to, c[to], "open".into(), to, true);
        out.push(t);
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn linreg_slope_of_a_line_is_its_slope() {
        let v: Vec<f64> = (0..20).map(|i| 3. + 2. * i as f64).collect();
        let s = linreg_slope(&v, 7);
        assert!(s[6..].iter().all(|x| (x - 2.).abs() < 1e-9) && s[5].is_nan());
        // T3 of a constant is the constant (c1+c2+c3+c4 = 1)
        let t = t3(&[5.; 50], 5, 0.7);
        assert!((t[49] - 5.).abs() < 1e-9);
    }
}
