//! Donald Dorsey, "The Relative Volatility Index", Technical Analysis of Stocks & Commodities V.11:6 (June 1993) pp.253-256,
//! with the sidebar "Relative volatility index" (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008)
//! (traders.com)/VOLUMES/V11/C06/THERELA.pdf and Siderel.pdf). Book id `scdrvi`: the 10/20 SMA crossover confirmed by the RVI.
//! Lane K13 (titles loop, 2 Nov 2026), to-code row 198, lead decision 72; independent check authors/titles-1030/K13w/k13_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::msws::sma;
use super::sctraub::wilder_from;
use super::tselhs::Book;

pub const REVIEWED: &[&str] = &["scdrvi_ma1020"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] RVI as the sidebar: u = the population standard deviation (Excel STDEVP) of the last 10 closes on an up close, else 0; d likewise on a down close; U and D start as the average of the first 14 values, then (13 x previous + new) / 14; RVI = 100 U / (U + D). Crossovers: SMA10 above SMA20 now and at or below it on the last bar (mirror for a sell). A crossover with the RVI on the wrong side is 'ignored' and stays pending until the opposite crossover; rules 3-4 then enter at RVI >= 60 / <= 40. Every order fills at the signal bar's close; a confirmed opposite crossover reverses. The article tested 650 one-hour bars of six futures markets (S&P 500 9-10-1 with the RVI, $5,200); daily bars on indices, ETFs and stocks, US and India, are a declared application (decision 72). The live scart ddrvi card is Dorsey's later 'Refining the RVI' (inertia system), a different rule.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "scdrvi_ma1020", name: "Dorsey RVI-confirmed 10/20 SMA crossover: take a cross only with the RVI on its side, a missed cross at RVI 60/40, out at RVI 40/60", group: "S&C: The Relative Volatility Index (Dorsey, V.11:6)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "scdrvi", rules: &[
            "F|\"If today's close > yesterday's close, then u = STD(10) otherwise u=0 ... U = (13 Uprevious + u) / 14 ... RVI = 100 U / (U+D)\"; \"=IF(D11-D10>0,STDEVP(D2:D11),0)\"; \"The first step is to average the first 14 values\" (sidebar, V.11:6)",
            "E|\"1 Only take buy signals from moving average crossover when RVI > 50. 2 Only take sell signals when RVI < 50. 3 Enter the market long if after an ignored buy signal the RVI rises to 60 or above. 4 Enter the market short if after an ignored sell signal the RVI falls to 40 or below.\" (p.254; trading indicator: 'a 10-bar simple moving average crossing a 20-bar simple moving average')",
            "X|\"5 Exit a long position if the RVI falls to 40 or below. 6 Exit a short position if the RVI rises to 60 or above.\" (p.254)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let id = REVIEWED.iter().find(|r| **r == id)?;
    Some((*id, "Dorsey Relative Volatility Index (S&C 1993)", "RVI-confirmed 10/20 SMA crossover"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> { reviewed(id).then(Vec::new) }

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, _p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let (c, n) = (&x.c, x.c.len());
    let (mut u, mut d) = (vec![0.0; n], vec![0.0; n]);
    for i in 9..n {
        let w = &c[i - 9..=i];
        let m = w.iter().sum::<f64>() / 10.0;
        let sd = (w.iter().map(|v| (v - m).powi(2)).sum::<f64>() / 10.0).sqrt();
        if c[i] > c[i - 1] { u[i] = sd; }
        if c[i] < c[i - 1] { d[i] = sd; }
    }
    let (uu, dd) = (wilder_from(&u, 14, 9), wilder_from(&d, 14, 9));
    let rvi: Vec<f64> = (0..n).map(|i| if uu[i].is_finite() && uu[i] + dd[i] > 0.0 { 100.0 * uu[i] / (uu[i] + dd[i]) } else { f64::NAN }).collect();
    let (s10, s20) = (sma(c, 10), sma(c, 20));
    let mut pos = Book::new();
    let (mut pb, mut ps) = (false, false);
    for i in from.max(20)..=to {
        let r = rvi[i];
        if !r.is_finite() { continue; }
        let up = s10[i] > s20[i] && s10[i - 1] <= s20[i - 1];
        let dn = s10[i] < s20[i] && s10[i - 1] >= s20[i - 1];
        if up {
            ps = false;
            if r > 50.0 { pos.enter(1.0, i, c[i], "close"); pb = false; } else { pb = true; }
        } else if dn {
            pb = false;
            if r < 50.0 { pos.enter(-1.0, i, c[i], "close"); ps = false; } else { ps = true; }
        } else if pb && r >= 60.0 { pos.enter(1.0, i, c[i], "close"); pb = false; }
        else if ps && r <= 40.0 { pos.enter(-1.0, i, c[i], "close"); ps = false; }
        if pos.side() > 0.0 && r <= 40.0 { pos.exit(i, c[i], "RVI <= 40"); }
        else if pos.side() < 0.0 && r >= 60.0 { pos.exit(i, c[i], "RVI >= 60"); }
    }
    Some(pos.done(to, c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rvi_confirmed_crossover() {
        let r = super::super::sctraub::tests::bars(3000);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("scdrvi_ma1020", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() > 5 && t.iter().any(|z| z.dir > 0.0) && t.iter().any(|z| z.dir < 0.0));
        assert!(family_of("scdrvi_ma1020").is_some() && params("scdrvi_ma1020").is_some());
    }
}

// Instrument admission: S&C Dorsey RVI 2026-11-02 (titles lane K13).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "scdrvi" && reviewed(def.id),
    label: "Donald Dorsey tested the rule on one-hour bars of six futures markets. Daily runs on indices, ETFs and stocks, US and India, are a declared application.",
    basis: "Technical Analysis of Stocks & Commodities V.11:6 pp.253-256 and sidebar (The Relative Volatility Index); independent check authors/titles-1030/K13w/k13_ref.py",
};
