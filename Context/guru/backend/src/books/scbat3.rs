//! Steve Burns, "Tweaking The T3 Trading System", Technical Analysis of Stocks & Commodities V.19:6 (June 2001) pp.30-37
//! (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V19/C06/064T3.pdf).
//! Book id `scbat3`. Lane PA1 (13 Nov 2026, pool A of authors/a1/A1.tsv; R4 #297). Independent check authors/pa1/pa1_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::msws::ema;
use super::sctraub::week_groups;
use super::tselhs::Book;

pub const REVIEWED: &[&str] = &["scbat3_at3"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Adapted: 1 unstated detail, R4 #297; unverified] The printed MetaStock 6.5 AT3 system: e1..e6 = six chained MetaStock exponential averages Mov(P,5,E) of the close (each seeded with its input's first value, alpha 2/6); a = RSquared(C,opt2), the r-squared of a least-squares line through the last opt2 closes; c1 = -a^3, c2 = 3a^2+3a^3, c3 = -6a^2-3a-3a^3, c4 = 1+3a+a^3+3a^2; AT3 = c1*e6+c2*e5+c3*e4+c4*e3. Enter Long: C < AT3; Close Long: C >= AT3; Enter Short: C > AT3; Close Short: C < AT3 (as printed: long below the line, short above it), at the bar's close (MetaStock System Tester default). Weekly bars (the article's AT3 strategy: 'Trade weekly so price trends can develop'; weekly bars = calendar weeks of the vault's daily bars, traded at the week's last close; the week holding the data's last bar may be unfinished and is not traded); option weekly = 0 runs it on daily bars. Unstated in the article: (1) the r-squared length opt2, optimised per stock (range 0.1-15); vault reading: 9, the only AT3 optimum the article prints (Child Comp Service, Figure 4) and inside its advice to keep t and r under 10. Options: t = the code's periods (5), r = opt2, shorts = 0 for long only (the Child Comp test). Article results (Aristotle, Child Comp Service: penny stocks, optimised, all equity reinvested) are not comparable; runs on indices, ETFs and stocks, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "scbat3_at3", name: "Burns AT3: Tillson T3 with r-squared as its volume factor; long below the line, short above it (weekly)", group: "S&C: Tweaking The T3 Trading System (Burns, V.19:6)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "scbat3", rules: &[
            "F|\"periods:=5; a:=RSquared(C,opt2); e1:=Mov(P,periods,E); ... e6:=Mov(e5,periods,E); c1:=-a*a*a; c2:=3*a*a+3*a*a*a; c3:=-6*a*a-3*a-3*a*a*a; c4:=1+3*a+a*a*a+3*a*a; res:=c1*e6+c2*e5+c3*e4+c4*e3;\" (V.19:6 p.36, AT3 MetaStock code)",
            "E|\"Enter Long ... C<res\"; \"Enter Short ... C> res\" (p.36)",
            "X|\"Close Long ... C>= res\"; \"Close Short ... C< res\" (p.36); \"2 Trade weekly so price trends can develop\" (p.36)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    reviewed(id).then_some(("scbat3_at3", "Burns adaptive T3 (S&C)", "t 5, r 9"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("t", "T3 period (code 5)", 5., 2., 12., 1.), p("r", "R-squared length opt2 (unstated; vault 9)", 9., 2., 15., 1.),
        p("weekly", "1 = weekly bars (article), 0 = daily", 1., 0., 1., 1.), p("shorts", "1 = long and short (code), 0 = long only", 1., 0., 1., 1.)])
}

/// MetaStock RSquared(v, n): squared correlation of the last n values with time; NaN until n values exist.
pub fn rsquared(v: &[f64], n: usize) -> Vec<f64> {
    (0..v.len()).map(|i| {
        if n < 2 || i + 1 < n { return f64::NAN; }
        let w = &v[i + 1 - n..=i];
        let (xm, ym) = ((n as f64 - 1.) / 2., w.iter().sum::<f64>() / n as f64);
        let (mut sxy, mut sxx, mut syy) = (0., 0., 0.);
        for (k, y) in w.iter().enumerate() { let (dx, dy) = (k as f64 - xm, y - ym); sxy += dx * dy; sxx += dx * dx; syy += dy * dy; }
        if syy == 0. { f64::NAN } else { sxy * sxy / (sxx * syy) }
    }).collect()
}

/// AT3 line over a close series.
pub fn at3(c: &[f64], t: usize, r: usize) -> Vec<f64> {
    let a = rsquared(c, r);
    let mut e = vec![ema(c, t)];
    for _ in 1..6 { let nx = ema(e.last().unwrap(), t); e.push(nx); }
    (0..c.len()).map(|i| {
        let a = a[i];
        let (c1, c2, c3, c4) = (-a * a * a, 3. * a * a + 3. * a * a * a, -6. * a * a - 3. * a - 3. * a * a * a, 1. + 3. * a + a * a * a + 3. * a * a);
        c1 * e[5][i] + c2 * e[4][i] + c3 * e[3][i] + c4 * e[2][i]
    }).collect()
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d);
    let (t, r) = (g("t", 5.).round().max(1.) as usize, g("r", 9.).round().max(2.) as usize);
    let shorts = g("shorts", 1.) >= 0.5;
    // bar k -> its last daily index
    let weekly = g("weekly", 1.) >= 0.5;
    // R4 #324 M3: the week holding the last bar of the data may still be in progress, so it is not traded
    let ends: Vec<usize> = if weekly { week_groups(x).into_iter().map(|(_, e)| e).filter(|&e| e + 1 < x.r.len()).collect() } else { (0..x.r.len()).collect() };
    let c: Vec<f64> = ends.iter().map(|&e| x.c[e]).collect();
    let line = at3(&c, t, r);
    let mut pos = Book::new();
    for (k, &i) in ends.iter().enumerate() {
        if i < from { continue; }
        if i > to { break; }
        if !line[k].is_finite() { continue; }
        let (cl, l) = (c[k], line[k]);
        if pos.side() > 0. && cl >= l { pos.exit(i, cl, "close >= AT3"); }
        if pos.side() < 0. && cl < l { pos.exit(i, cl, "close < AT3"); }
        if pos.side() == 0. {
            if cl < l { pos.enter(1., i, cl, "close"); } else if cl > l && shorts { pos.enter(-1., i, cl, "close"); }
        }
    }
    Some(pos.done(to, x.c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn at3_reverses_around_the_line() {
        let r = super::super::sctraub::tests::bars(3000);
        let x = Ctx::new(&r, &r, &[]);
        assert!((rsquared(&[1., 2., 3., 4.], 4)[3] - 1.).abs() < 1e-12);
        let mut p = Params::new(); p.insert("weekly".into(), 0.);
        let t = run("scbat3_at3", &x, 0, r.len() - 1, &p).unwrap();
        let line = at3(&x.c, 5, 9);
        assert!(t.len() > 20 && t.iter().any(|z| z.dir < 0.));
        for z in &t { let e = z.legs[0].i; assert!(z.dir * (line[e] - x.c[e]) > 0.); }
        assert!(run("scbat3_at3", &x, 0, r.len() - 1, &Params::new()).unwrap().len() < t.len());
        p.insert("shorts".into(), 0.);
        assert!(run("scbat3_at3", &x, 0, r.len() - 1, &p).unwrap().iter().all(|z| z.dir > 0.));
    }
}

// Instrument admission: S&C Burns AT3 2026-11-13 (lane PA1, R4 #297).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "scbat3" && reviewed(def.id),
    label: "Steve Burns showed AT3 on two penny stocks with optimised settings; indices, ETFs and stocks, US and India, on weekly bars built from the vault's daily bars are a declared application with one labelled reading (R4 #297).",
    basis: "Technical Analysis of Stocks & Commodities V.19:6 pp.30-37 (Tweaking The T3 Trading System); R4 #297; independent check authors/pa1/pa1_ref.py",
};
