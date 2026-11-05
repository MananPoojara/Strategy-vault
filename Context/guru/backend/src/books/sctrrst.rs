//! Anthony Trongone, "Restrictive Trading Systems: Trading When Less Is More", Technical Analysis of Stocks & Commodities V.22:2
//! (Feb 2004) pp.36-38, 40 (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/
//! V22/C02/027TRON.pdf). Book id `sctrrst`: the SPY/QQQ restrictive pair rule, short QQQ for one day. Lane K52 (5 Nov 2026), to-code
//! row 216, decisions #94, #223 (5), #226; independent check authors/titles-1030/K52w/k52_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::tselhs::Book;

pub const REVIEWED: &[&str] = &["sctrrst_qqq"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] After the close, when (1) SPY closes above its previous close, (2) SPY's percentage close change is greater than QQQ's and (3) SPY's percentage volume change is greater than QQQ's, short QQQ at that close and cover at the next day's close (the article's better play, Figure 2: shorting the cubes). Each signal day is one trade; a signal on the cover day covers and shorts again at the same close (Figure 1 lists 05/16/02 and 05/17/02 as two plays). Volume change = today's volume against the previous session's: the Figure 1 field definitions say 'relative to the previous trading session's trading volume', and their printed values reproduce on the vault's data (04/22/03: SPY 0.865, QQQ 0.341); the prose's 'eight-day simple moving average of volume' is the labelled option p.vol = 1 (the 8 sessions before the signal day). On the vault's SPY/QQQ the default reproduces 28 of the article's 29 signal days from 1 Jan 2002 to 30 Apr 2003 (04/22/03 is a tie at two decimals here, SPY +1.89% vs QQQ +1.89%), the option 20 of 29. SPY is the second series on QQQ's dates (decision #223); only the article's pair runs (decision #223 (5)): other instruments show no result. Shorts are hypothetical (no borrow cost). No commissions.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "sctrrst_qqq", name: "Trongone restrictive pair (QQQ): when SPY outgains QQQ in price and volume on an up day, short QQQ at the close for one day", group: "S&C: Restrictive Trading Systems (Trongone, V.22:2)", session: 0, dir: "Short", status: Status::Live,
        needs_volume: true, needs_vix: false, needs_series: &["SPY.C", "SPY.V"], book: "sctrrst", rules: &[
            "E|\"The SPDRs have a positive day (that is, for the trading day, there is a gain in price). The percentage change of the SPDRs' closing price is greater than the percentage change of the cubes' closing price. The percentage change of the SPDRs' closing volume is greater than the percentage change of the cubes' closing volume.\" (V.22:2 p.36)",
            "F|\"pc_spy volume: The percentage change in trading volume relative to the previous trading session's trading volume.\" (Figure 1 fields, p.37); prose: \"a comparison between an eight-day simple moving average of volume and the volume for the trading day\" (p.36) = option p.vol = 1",
            "E|\"At the close of trading, take a short position on the cubes.\" (p.38)", "X|\"On the next trading day, cover the position when the market closes.\" (p.38)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    reviewed(id).then_some(("sctrrst_qqq", "Trongone restrictive SPY/QQQ pair (S&C)", "short QQQ one day"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    reviewed(id).then(|| vec![ParamDef { key: "vol", label: "Volume change: 0 = vs the previous session (Figure 1), 1 = vs the 8-day average before the day (prose)", def: 0., min: 0., max: 1., step: 1. }])
}

/// Percentage volume change on bar i: vs the previous bar (avg8 = false) or vs the simple average of the 8 bars before i.
pub fn vchg(v: &[f64], i: usize, avg8: bool) -> f64 {
    if avg8 { if i < 8 { return f64::NAN; } v[i] / (v[i - 8..i].iter().sum::<f64>() / 8.) - 1. } else if i < 1 { f64::NAN } else { v[i] / v[i - 1] - 1. }
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from > to || to >= x.r.len() { return None; }
    let (Some(sc), Some(sv)) = (x.series("SPY.C"), x.series("SPY.V")) else { return Some(vec![]) };
    let avg8 = p.get("vol").copied().unwrap_or(0.) >= 0.5;
    let qv: Vec<f64> = x.r.iter().map(|r| r.volume.unwrap_or(f64::NAN)).collect();
    let c = &x.c;
    let mut pos = Book::new();
    for i in from.max(1)..=to {
        if pos.side() != 0. { pos.exit(i, c[i], "covered at the next day's close"); }
        let (ps, pq) = (sc[i] / sc[i - 1] - 1., c[i] / c[i - 1] - 1.);
        if sc[i] > sc[i - 1] && ps > pq && vchg(sv, i, avg8) > vchg(&qv, i, avg8) { pos.enter(-1., i, c[i], "close"); }
    }
    Some(pos.done(to, c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn short_qqq_for_one_day_when_spy_leads_in_price_and_volume() {
        // QQQ closes / volumes; SPY closes / volumes
        let q = [(10., 100.), (10.1, 120.), (10.0, 100.), (10.2, 100.), (10.3, 90.)];
        let s = [(20., 50.), (20.4, 70.), (20.6, 60.), (20.9, 80.), (21.0, 100.)];
        let r: Vec<Row> = q.iter().enumerate().map(|(i, &(c, v))| Row { ticker: "QQQ".into(), date: format!("2002-01-{:02}", i + 2), open: c, high: c, low: c, close: c, volume: Some(v) }).collect();
        let mut x = Ctx::new(&r, &r, &[]);
        x.extra.insert("SPY.C".into(), s.iter().map(|z| z.0).collect());
        x.extra.insert("SPY.V".into(), s.iter().map(|z| z.1).collect());
        let t = run("sctrrst_qqq", &x, 0, 4, &Params::new()).unwrap();
        // bar 1: SPY +2% > QQQ +1%, vol +40% > +20% -> short 10.1, cover 10.0 at bar 2; bar 2: SPY +0.98% > QQQ -0.99%, vol -14% > -17% -> short 10.0;
        // bar 3: SPY +1.46% > QQQ +2%? no -> cover 10.2; bar 4: SPY +0.48% > QQQ +0.98%? no
        assert_eq!(t.iter().map(|z| (z.dir, z.legs[0].i, z.legs[0].px, z.exit_i, z.exit_px, z.open)).collect::<Vec<_>>(),
            vec![(-1., 1, 10.1, 2, 10.0, false), (-1., 2, 10.0, 3, 10.2, false)]);
        assert!((vchg(&[1., 1., 1., 1., 1., 1., 1., 1., 2.], 8, true) - 1.).abs() < 1e-12);
    }
}

// Instrument admission: S&C Trongone restrictive pair 2026-11-05 (titles lane K52). QQQ only (pair_issue in second.rs).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "sctrrst" && reviewed(def.id),
    label: "Anthony Trongone tested the pair rule on QQQ with SPY (1 Jan 2002 - 30 Apr 2003), the only pair the vault runs (decision #223 (5)).",
    basis: "Technical Analysis of Stocks & Commodities V.22:2 pp.36-38, 40 (Restrictive Trading Systems), Figures 1-2; independent check authors/titles-1030/K52w/k52_ref.py",
};
