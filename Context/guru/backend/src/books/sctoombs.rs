//! John Toombs, "A Short-Term Buy Signal For The DJIA", Technical Analysis of Stocks & Commodities V.8:7 (July 1990) pp.245-247
//! (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V08/C07/SHORT.pdf).
//! Book id `sctoombs`. Lane K13 (titles loop, 2 Nov 2026), to-code row 197, lead decision 71; independent check
//! authors/titles-1030/K13w/k13_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::tselhs::Book;

pub const REVIEWED: &[&str] = &["sctoombs_stoch20"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] Raw value over the last 20 trading days' highs and lows (50 when the range is zero); %K and %D start at the first raw value (bar 20). Step 1 arms the signal when %D reaches 84 while flat; the buy is taken at that day's close (the article gives no time; it counts the trade in DJIA closing points); after a sell, %D must reach 84 again. The '60 DJIA points, on a closing basis' stop runs only on the DJIA (DJI), the article's own index (decision 71); on every other instrument the point stop is held, not converted (decision 19). Article (DJIA, September 1984 to 20 January 1990): 20 signals, 19 winners, +785 points in 165 trading days. Our DJI run over the same span: 20 trades, 18 winners, +740.8 points in 166 days. Lead decision 195: the 60-point stop is part of the rule, so the card is published on DJI only; results without the stop on other instruments are withdrawn.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "sctoombs_stoch20", name: "Toombs slow 20-day stochastic: after %D reaches 84, buy the first dip (%D under 70, %K 4 under %D); sell when %K is 4 over %D", group: "S&C: A Short-Term Buy Signal For The DJIA (Toombs, V.8:7)", session: 0, dir: "Long", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "sctoombs", rules: &[
            "F|\"Raw value = 100 x (Last close - Lowest low) / (Highest high - Lowest low)\" over 20 trading days; \"%K is ... the preceding day's %K by 2, adding the new raw value, then dividing by 3. The %D ... the preceding day's %D by 2, adding the new %K, then dividing by 3.\" (V.8:7 p.245)",
            "E|\"The slow 20-day %D must reach a reading of 84. Buy the next time the %D falls below 70 and the %K is 4 points or more below %D.\" (p.245)",
            "X|\"Sell when the %K is 4 points or more above %D and %D is 30 or higher. Use 60 DJIA points, on a closing basis, as a stop.\" (p.245; the point stop runs on DJI only)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let id = REVIEWED.iter().find(|r| **r == id)?;
    Some((*id, "Toombs DJIA stochastic buy signal (S&C)", "Slow 20-day stochastic"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    reviewed(id).then(|| vec![ParamDef { key: "stop", label: "Closing stop in index points, DJI only (60 printed; 0 off)", def: 60., min: 0., max: 500., step: 10. }])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let n = x.c.len();
    let (mut k, mut d) = (vec![f64::NAN; n], vec![f64::NAN; n]);
    for i in 19..n {
        let hh = x.h[i - 19..=i].iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let ll = x.l[i - 19..=i].iter().copied().fold(f64::INFINITY, f64::min);
        let raw = if hh > ll { 100.0 * (x.c[i] - ll) / (hh - ll) } else { 50.0 };
        if i == 19 { k[i] = raw; d[i] = raw; } else { k[i] = (2.0 * k[i - 1] + raw) / 3.0; d[i] = (2.0 * d[i - 1] + k[i]) / 3.0; }
    }
    let dji = x.r.first().is_some_and(|r| r.ticker == "DJI");
    let stop = if dji { p.get("stop").copied().unwrap_or(60.0) } else { 0.0 };
    let mut pos = Book::new();
    let mut armed = false;
    for i in from.max(19)..=to {
        if pos.side() > 0.0 {
            if k[i] >= d[i] + 4.0 && d[i] >= 30.0 { pos.exit(i, x.c[i], "%K 4 over %D"); }
            else if stop > 0.0 && x.c[i] <= pos.entry_px() - stop { pos.exit(i, x.c[i], "60-point closing stop"); }
        }
        if pos.side() == 0.0 {
            if d[i] >= 84.0 { armed = true; }
            else if armed && d[i] < 70.0 && k[i] <= d[i] - 4.0 { pos.enter(1.0, i, x.c[i], "close"); armed = false; }
        }
    }
    Some(pos.done(to, x.c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn toombs_buys_dips_after_strength() {
        let r = super::super::sctraub::tests::bars(3000);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("sctoombs_stoch20", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() > 5 && t.iter().all(|z| z.dir > 0.0 && z.legs[0].px == x.c[z.legs[0].i]));
        assert!(family_of("sctoombs_stoch20").is_some() && params("sctoombs_stoch20").is_some());
    }
}

// Instrument admission: S&C Toombs 2026-11-02 (titles lane K13).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "sctoombs" && reviewed(def.id),
    label: "John Toombs tested the signal on the DJIA (1984-1990). Published on DJI only, where the 60-point stop means what the article meant (lead decision 195).",
    basis: "Technical Analysis of Stocks & Commodities V.8:7 pp.245-247 (A Short-Term Buy Signal For The DJIA); independent check authors/titles-1030/K13w/k13_ref.py",
};
