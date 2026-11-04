//! Matt Blackman, "Working Money: Mining For Maximum Shine", Technical Analysis of Stocks & Commodities V.25:2 (February 2007)
//! pp.40-42 (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V25/C02/
//! 027BLAC.pdf). Book id `scblack`. Lane K40 follow-up (4 Nov 2026, decision #207(2)): the article's gold rule on GLD; silver,
//! palladium, platinum and copper stay held (no SLV or other metal series in the vault). Independent check
//! authors/titles-1030/K40w/k40_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::lc3::Pos;

pub const REVIEWED: &[&str] = &["scblack_gold"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }
/// The vault's only gold series.
pub const HOME: &str = "GLD";

const NOTE: &str = "[Declared readings] The article's worked formula: 'Buy when month is equal to 7 and day of the month is equal to or greater than 23. Sell when month is equal to 12 and day of the month is equal to or greater than 23.' On daily bars: buy at the close of the first bar in July dated the 23rd or later, sell at the close of the first bar in December dated the 23rd or later; long or flat, once a year (MetaStock's system tester, the article's tool, fills at the close by default). The article tested weekly Engelhard gold prices (1/4/1995-12/30/2005); the vault holds no spot gold, so the rule runs on GLD (SPDR Gold Shares, from 2005), the vault's only gold series, as a labelled adaptation (decision #207); a gold season on equities would change the asset class, so other instruments are not published. Silver (Aug-Mar), palladium (Nov-Mar), platinum (Sep-Feb) and copper (Apr-Feb) need series the vault does not hold. Book (Figure 4, Engelhard gold, 11 years): 7 of 11 trades profitable, average trade 5.76%, maximum drawdown -8.44%, 44% of the time in the market; GLD starts in 2005, so the book window cannot be reproduced.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "scblack_gold", name: "Blackman gold seasonal: buy gold in the last week of July (day >= 23), sell in the last week of December, on GLD", group: "S&C: Mining For Maximum Shine (Blackman, V.25:2)", session: 0, dir: "Long", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "scblack", rules: &[
            "F|\"Say we want to determine how profitable a system is that buys gold at the July low and sells at the December high, buying and selling in the last week of the month\" (p.41)",
            "E|\"Buy when month is equal to 7 and day of the month is equal to or greater than 23.\" (p.41)",
            "X|\"Sell when month is equal to 12 and day of the month is equal to or greater than 23.\" (p.41); Figure 4: \"Engelhard Gold (.GLLD) ... July-Dec\" (p.42)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { reviewed(id).then_some(("scblack_gold", "Blackman metals seasonal (S&C)", "gold July-December")) }

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("buy", "Buy month (article 7)", 7., 1., 12., 1.), p("sell", "Sell month (article 12)", 12., 1., 12., 1.),
        p("day", "From day of the month (article 23)", 23., 1., 31., 1.)])
}

/// The card is published on GLD only.
pub fn home(id: &str, ticker: &str) -> Option<String> {
    (reviewed(id) && ticker != HOME).then(|| format!("Not published on {ticker}: the article's season is for gold; the vault's only gold series is GLD (open the card on GLD). A gold season on another instrument would change the asset class."))
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d).round() as i64;
    let (buy, sell, day) = (g("buy", 7.), g("sell", 12.), g("day", 23.));
    let mut pos = Pos::new();
    for t in from..=to {
        let d = &x.r[t].date;
        let (m, dd): (i64, i64) = (d[5..7].parse().unwrap_or(0), d[8..10].parse().unwrap_or(0));
        if dd < day { continue; }
        if pos.side() > 0. && m == sell { pos.exit(t, x.c[t], "sell month, day >= 23"); }
        else if pos.side() == 0. && m == buy && t < to { pos.enter(1., t, x.c[t], "close"); }
    }
    Some(pos.done(to, x.c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn gold_season_july_23_to_december_23() {
        let r = super::super::sctraub::tests::bars(1000);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("scblack_gold", &x, 0, r.len() - 1, &Params::new()).unwrap();
        let got: Vec<_> = t.iter().filter(|z| !z.open).map(|z| (r[z.legs[0].i].date.as_str(), r[z.exit_i].date.as_str())).collect();
        assert_eq!(got, vec![("2001-07-23", "2001-12-24"), ("2002-07-23", "2002-12-23"), ("2003-07-23", "2003-12-23")]);
        assert!(home("scblack_gold", "SPY").is_some() && home("scblack_gold", "GLD").is_none());
    }
}

// Instrument admission: S&C Blackman gold seasonal 2026-11-04 (titles lane K40, decision #207). Published on GLD only (home()).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "scblack" && reviewed(def.id),
    label: "Matt Blackman tested the gold season on weekly Engelhard gold prices (1995-2005). It runs on GLD, the vault's only gold series, a declared adaptation; other instruments are not published.",
    basis: "Technical Analysis of Stocks & Commodities V.25:2 pp.40-42 (Mining For Maximum Shine), formula text and Figure 4; independent check authors/titles-1030/K40w/k40_ref.py",
};
