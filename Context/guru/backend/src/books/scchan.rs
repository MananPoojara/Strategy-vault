//! Jacinta Chan, "Trading Trends With The Bollinger Bands Z-Test", Technical Analysis of Stocks & Commodities V.24:3 (March 2006)
//! pp.46-52 (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V24/C03/
//! 049CHAN.pdf). Book id `scchan`: the BBZ system. Lane K13 (titles loop, 2 Nov 2026), to-code row 213; independent check
//! authors/titles-1030/K13w/k13_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::tselhs::Book;

pub const REVIEWED: &[&str] = &["scchan_bbz"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] The MetaStock rules: BbandTop/BbandBot(CLOSE,20,SIMPLE,1), i.e. the 20-day SMA +/- one standard deviation of the 20 closes (MetaStock's population deviation by default; the article's Excel STDEV, the sample deviation, is an option). Flat between the bands. Signals and fills at the close (the article's example buys at the 6 July 2005 close, 898.5, and sells at the 9 August close, 941.0). HELD: the five-point stop and the trailing stops of the trading plan (index points are not converted, decision 19). Article (KLSE CI futures, 2 January 1996 - 7 December 2005): +792 index points after costs, 87 winning and 133 losing trades. Runs on indices, ETFs and stocks, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "scchan_bbz", name: "Chan Bollinger z-test (BBZ): long above the 20-day +1 SD band, short below the -1 SD band, out when back inside", group: "S&C: Trading Trends With The Bollinger Bands Z-Test (Chan, V.24:3)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "scchan", rules: &[
            "E|\"Under Buy long, key in: CLOSE>BbandTop(CLOSE,20,SIMPLE,1) ... Under Sell short, key in: CLOSE<BbandBot(CLOSE,20,SIMPLE,1)\" (V.24:3 p.48)",
            "X|\"Under Sell exit long, key in: CLOSE<BbandTop(CLOSE,20,SIMPLE,1) ... Under Buy exit short, key in: CLOSE>BbandBot(CLOSE,20,SIMPLE,1)\" (p.48)",
            "F|\"For the purpose of this article, I use 20 days and one standard deviation.\" Z = (Close - Moving average) / Standard deviation (p.47)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let id = REVIEWED.iter().find(|r| **r == id)?;
    Some((*id, "Chan Bollinger z-test (S&C)", "20-day, 1 SD"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    reviewed(id).then(|| vec![ParamDef { key: "sample", label: "Standard deviation (0 = population, MetaStock; 1 = sample, the article's Excel STDEV)", def: 0., min: 0., max: 1., step: 1. }])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let div = if p.get("sample").copied().unwrap_or(0.0) >= 0.5 { 19.0 } else { 20.0 };
    let mut pos = Book::new();
    for i in from.max(19)..=to {
        let w = &x.c[i - 19..=i];
        let m = w.iter().sum::<f64>() / 20.0;
        let sd = (w.iter().map(|v| (v - m).powi(2)).sum::<f64>() / div).sqrt();
        let (up, lo, c) = (m + sd, m - sd, x.c[i]);
        if pos.side() > 0.0 && c < up { pos.exit(i, c, "back under the upper band"); } else if pos.side() < 0.0 && c > lo { pos.exit(i, c, "back over the lower band"); }
        if pos.side() == 0.0 { if c > up { pos.enter(1.0, i, c, "close"); } else if c < lo { pos.enter(-1.0, i, c, "close"); } }
    }
    Some(pos.done(to, x.c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bbz_both_sides() {
        let r = super::super::sctraub::tests::bars(3000);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("scchan_bbz", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.iter().any(|z| z.dir > 0.0) && t.iter().any(|z| z.dir < 0.0));
    }
}

// Instrument admission: S&C Chan BBZ 2026-11-02 (titles lane K13).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "scchan" && reviewed(def.id),
    label: "Jacinta Chan tested BBZ on KLSE CI futures (1996-2005) and three other futures. Runs on indices, ETFs and stocks, US and India, are a declared application.",
    basis: "Technical Analysis of Stocks & Commodities V.24:3 pp.46-52 (Trading Trends With The Bollinger Bands Z-Test); independent check authors/titles-1030/K13w/k13_ref.py",
};
