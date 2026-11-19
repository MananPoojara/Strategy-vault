//! Stephane Reverre, "Are Two Channels Better Than One?", Technical Analysis of Stocks & Commodities V.18:1 (2000) pp.46-53
//! (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V18/C01/004TWO.pdf).
//! Book id `scsrcc`: the moving-average deviation channel (the first of the article's three systems). Lane PA2 (13 Nov 2026);
//! independent check authors/pa2/w/pa2_ref.py. Admission: sclang.rs (PA2_BOOKS).
//! Held: the CAPM-delta channel and the combination (the 10-day beta's estimator is not printed).
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::tselhs::Book;

pub const REVIEWED: &[&str] = &["scsrcc_ma_z"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Adapted: 1 unstated detail, R4 #297; unverified] z = (close - its 10-day simple moving average) / the 10-day standard deviation of the close (population formula, as the article's Bollinger sidebar: STDEVP), 'the algebraic difference between GE and its moving average, expressed as a multiple of its standard deviation'. Trend reading (the article's tested one): 'take a long position every time that difference goes above the trigger ... go short whenever it goes below'; trigger 1.5 standard deviations ('typically between one and 1.5'); 'keep the position one single day and trade on a single share from close to close'. Unstated: (1) 'goes above' - vault reading: the close on which z crosses the trigger (today beyond it, yesterday not; the article's GE year has 31 positions, far fewer than the days spent beyond the band). Option fade = 1 takes the opposite positions (the correction reading the article also describes); option trigger 1.0-1.5. Runs on indices, ETFs and stocks, US and India, are a declared application (the article: the Dow 30 stocks, 08/18/1998-08/03/1999).";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "scsrcc_ma_z", name: "Reverre moving-average channel: close crosses 1.5 standard deviations beyond its 10-day average -> one-day trade in the move's direction, close to close", group: "S&C: are two channels better than one? (Reverre, V.18:1)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "scsrcc", rules: &[
            "F|\"Here's the open-high-low-close chart of General Electric from August 18, 1998, to August 3, 1999, with a 10-day moving average and one-standard deviation Bollinger Bands ... We can visualize the same information by taking the algebraic difference between GE and its moving average, expressed as a multiple of its standard deviation (that is, volatility)\" (V.18:1, PDF p.1-2)",
            "E|\"If you decide that a crossover between the price of GE and the Bollinger channels indicates a new trend forming, you take a long position every time that difference goes above the trigger, typically between one and 1.5 standard deviations. Alternatively, you go short whenever it goes below between -1.5 and -1 standard deviations.\" (PDF p.2)",
            "X|\"If you keep the position one single day and trade on a single share from close to close, the total profit for this system on GE is $0.63 (17 longs for a profit of $2.69, 14 shorts for a loss of -$2.06).\" (PDF p.2)",
            "F|Figure 4 (Dow 30, trigger 1.5): total profit $94.21 for 910 positions (458 long, -$23.58; 452 short, +$117.79); per stock e.g. JPM 30 positions +$22.19, MRK 32 +$19.72, KO 24 +$12.06, GE 31 +$0.63, IBM 24 -$7.66, AA 34 -$10.09"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { reviewed(id).then_some(("scsrcc_ma_z", "Reverre moving-average channel (S&C)", "1.5")) }
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    reviewed(id).then(|| vec![
        ParamDef { key: "trigger", label: "Trigger in standard deviations (article 1.5; 'typically between one and 1.5')", def: 1.5, min: 0.5, max: 3., step: 0.1 },
        ParamDef { key: "fade", label: "0 = trade the move's direction (article's test); 1 = fade it", def: 0., min: 0., max: 1., step: 1. }])
}

pub fn zscore(c: &[f64], n: usize) -> Vec<f64> {
    (0..c.len()).map(|i| {
        if i + 1 < n { return f64::NAN; }
        let w = &c[i + 1 - n..=i];
        let m = w.iter().sum::<f64>() / n as f64;
        let sd = (w.iter().map(|v| (v - m) * (v - m)).sum::<f64>() / n as f64).sqrt();
        if sd > 0. { (c[i] - m) / sd } else { f64::NAN }
    }).collect()
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from > to || to >= x.r.len() { return None; }
    let k = p.get("trigger").copied().unwrap_or(1.5);
    let flip = if p.get("fade").copied().unwrap_or(0.) > 0.5 { -1. } else { 1. };
    let z = zscore(&x.c, 10);
    let mut pos = Book::new();
    for t in from.max(1)..to {
        if !z[t].is_finite() || !z[t - 1].is_finite() { continue; }
        let d = if z[t] > k && z[t - 1] <= k { 1. } else if z[t] < -k && z[t - 1] >= -k { -1. } else { 0. };
        if d != 0. { pos.exit(t, x.c[t], "next close"); pos.enter(d * flip, t, x.c[t], "close"); pos.exit(t + 1, x.c[t + 1], "next close"); }
    }
    Some(pos.done(to, x.c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn one_day_trades_on_z_crossings() {
        let r = super::super::sctraub::tests::bars(1500);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("scsrcc_ma_z", &x, 0, r.len() - 1, &Params::new()).unwrap();
        let z = zscore(&x.c, 10);
        assert!(t.len() > 10 && t.iter().all(|q| q.exit_i == q.legs[0].i + 1));
        for q in &t { let i = q.legs[0].i; assert!(if q.dir > 0. { z[i] > 1.5 && z[i - 1] <= 1.5 } else { z[i] < -1.5 && z[i - 1] >= -1.5 }); }
        let f = run("scsrcc_ma_z", &x, 0, r.len() - 1, &Params::from([("fade".into(), 1.)])).unwrap();
        assert_eq!(f.len(), t.len()); assert!(f.iter().zip(&t).all(|(a, b)| a.dir == -b.dir));
    }
}
