//! Thomas P. Drinka and Steven L. Kille, "A modification of Wilder's directional movement system", Technical Analysis of
//! Stocks & Commodities V.3:6 (1985) pp.218-221 (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008)
//! (traders.com)/VOLUMES/V03/C06/MODW.pdf). Book id `scdkdmi`. Lane K41 (batch T7a, 4 Nov 2026); independent check
//! authors/titles-1030/K41w/k41_ref.py.
//! This module also holds the small helpers the other lane-K41 modules share (close fills, least-squares end value, equity).
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::sctraub::{dmi, next_open};
use super::tselhs::Book;

const NAN: f64 = f64::NAN;

// ------------------------------------------------------------------ shared helpers (lane K41)
/// Position changes decided and filled at the close of bars from..=to: `want(t, side)` = the side wanted (0 = flat).
pub fn at_close(x: &Ctx, from: usize, to: usize, why: &str, mut want: impl FnMut(usize, f64) -> f64) -> Vec<Trade> {
    let mut pos = Book::new();
    for t in from..=to {
        let d = want(t, pos.side());
        if d != pos.side() { if d == 0.0 { pos.exit(t, x.c[t], why) } else { pos.enter(d, t, x.c[t], "close") } }
    }
    pos.done(to, x.c[to])
}
/// Least-squares line over the last n values (x = 1..n), its value at the last point (Excel TREND(y,,n), MetaStock LinearReg).
pub fn ols_end(v: &[f64], n: usize) -> Vec<f64> {
    let nf = n as f64;
    let (sx, sxx) = (nf * (nf + 1.) / 2., nf * (nf + 1.) * (2. * nf + 1.) / 6.);
    (0..v.len()).map(|i| {
        if n < 2 || i + 1 < n { return NAN; }
        let w = &v[i + 1 - n..=i];
        let sy: f64 = w.iter().sum();
        let sxy: f64 = w.iter().enumerate().map(|(k, y)| (k + 1) as f64 * y).sum();
        let m = (nf * sxy - sx * sy) / (nf * sxx - sx * sx);
        (sy - m * sx) / nf + m * nf
    }).collect()
}
/// a > b beyond floating-point noise (|a - b| > 1e-9 x max(|a|, |b|)): exact ties computed two ways never decide a signal.
pub fn gt(a: f64, b: f64) -> bool { a - b > 1e-9 * a.abs().max(b.abs()) }
/// Equity in price points of a position path (+1/-1/0 per bar, changes at closes), marked to each close.
pub fn close_equity(c: &[f64], pos: &[f64]) -> Vec<f64> {
    let mut e = vec![0.0; c.len()];
    for t in 1..c.len() { e[t] = e[t - 1] + pos[t - 1] * (c[t] - c[t - 1]); }
    e
}

// ------------------------------------------------------------------ Drinka & Kille modified directional movement
pub const REVIEWED: &[&str] = &["scdkdmi_adx"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] '+DM14 = a 14-day sum of +DM', TR14 and DX14 are Wilder's own accumulations (the article restates Wilder's definitions and changes only the period and the ADX test): the default uses Wilder smoothing (the vault's DMI); option sum = 1 uses plain rolling N-day sums of +DM, -DM and TR and ADX = the N-day average of DX as printed ('ADX where DX14/14'). +DM/-DM follow Wilder (only the larger outside move counts). The signal is the state, not a cross: on each close with ADX >= the threshold the position goes long if +DI > -DI and short if -DI > +DI; while ADX is below the threshold no trade is made and the position is held. 'Trades were executed at the opening price on the business day following a trading signal.' Default N = 12, ADX = 30, the best average net profit of the article's grid (N 4-50 step 2, ADX 5-80 step 5; six S&P 500 futures contracts 1983-1984; -$13,416 at N = 6/ADX 15 up to +$3,774 at N = 12/ADX 30). Runs on indices, ETFs and stocks, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "scdkdmi_adx", name: "Modified Wilder DMI: long when +DI > -DI, short when -DI > +DI, only while ADX(12) >= 30, next open", group: "S&C: modified directional movement (Drinka & Kille, V.3:6)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "scdkdmi", rules: &[
            "F|\"+DI14 = +DM14/TR14 ... -DI14 = -DM14/TR14 ... ADX where DX14/14\" (V.3:6, PDF p.1); \"we use the relationship among +DI14, -DI14, and ADX ... allow the system to be analyzed for an ADX of any value\" (PDF p.2)",
            "E|\"to take a long position when +DI exceeds -DI, and to take a short position when -DI exceeds +DI; and 2) trades are made only if ADX is greater than or equal to a specified scalar value\" (PDF p.2)",
            "X|\"Trades were executed at the opening price on the business day following a trading signal.\" (PDF p.2); always in after the first trade (stop and reverse)",
            "F|\"average net profit was maximized under the 12-day directional movement with ADX at 30\" (PDF p.3)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { reviewed(id).then_some(("scdkdmi_adx", "Modified Wilder DMI (S&C)", "ADX threshold")) }
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    reviewed(id).then(|| vec![
        ParamDef { key: "n", label: "Directional movement period (article grid 4-50)", def: 12., min: 4., max: 50., step: 2. },
        ParamDef { key: "adx", label: "Trade only when ADX >= (article grid 5-80)", def: 30., min: 5., max: 80., step: 5. },
        ParamDef { key: "sum", label: "1 = plain rolling N-day sums and average DX as printed; 0 = Wilder smoothing", def: 0., min: 0., max: 1., step: 1. }])
}

/// +DI, -DI, ADX from plain rolling n-day sums (option sum = 1).
fn dmi_sum(x: &Ctx, n: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let len = x.c.len();
    let (mut tr, mut pm, mut mm) = (vec![NAN; len], vec![NAN; len], vec![NAN; len]);
    for i in 1..len {
        tr[i] = (x.h[i] - x.l[i]).max((x.h[i] - x.c[i - 1]).abs()).max((x.l[i] - x.c[i - 1]).abs());
        let (u, d) = (x.h[i] - x.h[i - 1], x.l[i - 1] - x.l[i]);
        pm[i] = if u > d && u > 0.0 { u } else { 0.0 };
        mm[i] = if d > u && d > 0.0 { d } else { 0.0 };
    }
    let (st, sp, sm) = (super::msws::sma(&tr, n), super::msws::sma(&pm, n), super::msws::sma(&mm, n));
    let (mut p, mut m, mut dx) = (vec![NAN; len], vec![NAN; len], vec![NAN; len]);
    for i in 0..len {
        if st[i].is_finite() && st[i] > 0.0 {
            p[i] = 100.0 * sp[i] / st[i];
            m[i] = 100.0 * sm[i] / st[i];
            dx[i] = if p[i] + m[i] > 0.0 { 100.0 * (p[i] - m[i]).abs() / (p[i] + m[i]) } else { 0.0 };
        }
    }
    let adx = super::msws::sma(&dx, n);
    (p, m, adx)
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d);
    let (n, thr) = (g("n", 12.).round().max(2.) as usize, g("adx", 30.));
    let (pd, md, adx) = if g("sum", 0.) > 0.5 { dmi_sum(x, n) } else { dmi(x, n) };
    Some(next_open(x, from, to, "DI reversal", |t, side| {
        if !(adx[t] >= thr) || !pd[t].is_finite() { return side; }
        if pd[t] > md[t] { 1.0 } else if md[t] > pd[t] { -1.0 } else { side }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dmi_state_with_adx_filter() {
        let r = super::super::sctraub::tests::bars(1500);
        let x = Ctx::new(&r, &r, &[]);
        for s in [0., 1.] {
            let mut p = Params::new(); p.insert("sum".into(), s); p.insert("adx".into(), 15.);
            let t = run("scdkdmi_adx", &x, 0, r.len() - 1, &p).unwrap();
            assert!(t.len() > 3 && t.iter().any(|z| z.dir < 0.0) && t.windows(2).all(|w| w[0].dir != w[1].dir && w[0].exit_i == w[1].legs[0].i));
        }
        let mut p = Params::new(); p.insert("adx".into(), 101.);
        assert!(run("scdkdmi_adx", &x, 0, r.len() - 1, &p).unwrap().is_empty());
        assert_eq!(ols_end(&[1., 3., 5., 7.], 3)[3], 7.0);
    }
}

// Instrument admission: lane K41 S&C articles, 4 Nov 2026 (one entry for the plain declared-application books of the lane).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| K41_BOOKS.contains(&def.book) && k41_reviewed(def.book, def.id),
    label: "Stocks & Commodities and Active Trader articles (1985-2008), tested by their authors on futures, single stocks, ETFs or indices; runs on indices, ETFs and stocks, US and India, are a declared application, not the authors' tests.",
    basis: "Lane K41 (batch T7a, decision 205): each card cites its article pages; independent check authors/titles-1030/K41w/k41_ref.py",
};
/// Books admitted by the entry above.
pub const K41_BOOKS: &[&str] = &["scdkdmi", "schwc", "sckcmab", "scmabz", "scgtdy", "scbds", "scjleq", "scedosc", "scjlme", "scnsema", "scehift", "scrkb", "scmkpsb", "scrcst", "scggetf", "atbig"];
fn k41_reviewed(book: &str, id: &str) -> bool {
    use super::{atbig, scbds, scedosc, scehift, scggetf, scgtdy, schwc, scjleq, scjlme, sckcmab, scmabz, scmkpsb, scnsema, scrcst, scrkb};
    match book {
        "scdkdmi" => reviewed(id), "schwc" => schwc::reviewed(id), "sckcmab" => sckcmab::reviewed(id), "scmabz" => scmabz::reviewed(id),
        "scgtdy" => scgtdy::reviewed(id), "scbds" => scbds::reviewed(id), "scjleq" => scjleq::reviewed(id), "scedosc" => scedosc::reviewed(id),
        "scjlme" => scjlme::reviewed(id), "scnsema" => scnsema::reviewed(id), "scehift" => scehift::reviewed(id), "scrkb" => scrkb::reviewed(id),
        "scmkpsb" => scmkpsb::reviewed(id), "scrcst" => scrcst::reviewed(id), "scggetf" => scggetf::reviewed(id), "atbig" => atbig::reviewed(id),
        _ => false,
    }
}
