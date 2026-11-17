//! Michael D. Sheimo, interview "Michael Sheimo On Dow Theory" (by Thom Hartle), Technical Analysis of Stocks & Commodities V.16:6
//! (1998) pp.291-298 (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V16/C06/046INT.pdf,
//! PDF p.4). Book id `scsheimo`. Lane C6 (second pass, 17 Nov 2026); independent check authors/convert-1117/C6/ref/c6_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::msws::sma;
use super::sctraub::next_open;

pub const REVIEWED: &[&str] = &["scsheimo_9_18"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings, 4 unstated details, R4 #339; unverified] The interview prints the rule in prose (taken from Sheimo's book 'Cashing in on the Dow'): 'a nine-day and an 18-day moving average for buy and sell signals, with the nine-day as the shorter, or faster, moving average. We get a buy signal when both moving averages are moving up, and the shorter, or faster, one crosses the slower one. Then, when both averages are dropping and the nine-day crosses the 18-day, a sell signal is triggered.' Readings: (1) both are simple moving averages of the close; (2) 'moving up' / 'dropping' = today's value above / below yesterday's for each average; (3) 'crosses' = yesterday's 9-day at or below (above) the 18-day and today's above (below); (4) a sell signal closes the long and does not open a short (the interview names no short side); an opposite signal only exits. The signal is read at the close and filled at the next open. The article shows no results table, and a buy at a cross without a matching sell (the 18-day still rising when the 9-day crosses down) stays long until a sell signal. Runs on indices, ETFs and stocks, US and India, as a declared application of a rule given for the Dow.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "scsheimo_9_18", name: "Sheimo 9/18-day moving-average cross, both rising to buy, both falling to sell (long or flat, next open)", group: "S&C: Michael Sheimo on Dow Theory (V.16:6)", session: 0, dir: "Long", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "scsheimo", rules: &[
            "F|\"One of the examples I mention in Cashing in on the Dow is using a nine-day and an 18-day moving average for buy and sell signals, with the nine-day as the shorter, or faster, moving average.\" (V.16:6 p.294)",
            "E|\"We get a buy signal when both moving averages are moving up, and the shorter, or faster, one crosses the slower one.\" (p.294)",
            "X|\"Then, when both averages are dropping and the nine-day crosses the 18-day, a sell signal is triggered.\" (p.294)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { reviewed(id).then_some(("scsheimo_9_18", "Sheimo 9/18 moving averages (S&C)", "9 and 18 days")) }
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("fast", "Fast average, days (printed 9)", 9., 2., 60., 1.), p("slow", "Slow average, days (printed 18)", 18., 3., 120., 1.)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let f = p.get("fast").copied().unwrap_or(9.).round().max(2.) as usize;
    let s = (p.get("slow").copied().unwrap_or(18.).round() as usize).max(f + 1);
    let (a, b) = (sma(&x.c, f), sma(&x.c, s));
    Some(next_open(x, from, to, "sell signal", |t, side| {
        if t < 1 || !(a[t - 1].is_finite() && b[t - 1].is_finite() && b[t].is_finite() && a[t].is_finite()) { return side; }
        if a[t] > a[t - 1] && b[t] > b[t - 1] && a[t - 1] <= b[t - 1] && a[t] > b[t] { 1.0 }
        else if a[t] < a[t - 1] && b[t] < b[t - 1] && a[t - 1] >= b[t - 1] && a[t] < b[t] { 0.0 }
        else { side }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sheimo_trades_are_long_only_round_trips() {
        let r = super::super::sctraub::tests::bars(1500);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("scsheimo_9_18", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() > 10);
        assert!(t.iter().all(|z| z.dir > 0.0));
    }
}

// Instrument admission: S&C Sheimo 9/18 2026-11-17 (lane C6 second pass).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "scsheimo" && reviewed(def.id),
    label: "Michael Sheimo gave the 9/18-day moving-average buy and sell signals for the Dow in a Dow Theory interview, with no results table. Runs on indices, ETFs and stocks, US and India, are a declared application.",
    basis: "Technical Analysis of Stocks & Commodities V.16:6 pp.291-298 (Michael Sheimo On Dow Theory); independent check authors/convert-1117/C6/ref/c6_ref.py",
};
