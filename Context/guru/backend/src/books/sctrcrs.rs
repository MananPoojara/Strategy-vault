//! Anthony Trongone, "Following The Crowd After A One-Day Correction", Technical Analysis of Stocks & Commodities V.26:13 (Bonus
//! Issue 2008) pp.16-21 (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/
//! V26/C13/239TRON.pdf). Book id `sctrcrs`: the corrective recovery system (CRS). Lane K13 (titles loop, 2 Nov 2026), to-code row
//! 215, lead decision 79; independent check authors/titles-1030/K13w/k13_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::tselhs::Book;

pub const REVIEWED: &[&str] = &["sctrcrs_fxi"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] Regular trading session = today's close minus today's open; a correction day is a session loss of 1.0% or more of the open ((close - open) / open <= -1.0%). The next day, if the overnight session advances (open above the correction day's close), buy at the open and sell at that day's close. The article's ETF is FXI (iShares FTSE/Xinhua China 25), which is held (decision 79); the rule needs no volume, so other indices, ETFs and stocks, US and India, run as declared applications. Article (FXI, 12 October 2004 - 31 October 2007): 63 trades, 45 up, 16 down, 2 unchanged, $56.03 a share; a 3% correction is shown as an option. Our FXI run over the same span: 57 trades, 41 up, 14 down.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "sctrcrs_fxi", name: "Trongone corrective recovery (FXI): after a 1% session loss, buy the next open if it gaps up; sell at that close", group: "S&C: Following The Crowd After A One-Day Correction (Trongone, V.26:13)", session: 0, dir: "Long", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "sctrcrs", rules: &[
            "E|\"1 The regular trading session declines at least 1% (RTS < -1.00%) 2 This decline is followed by an advancing overnight trading session (OVS > $0.00) When these two conditions occur, take a long position at the opening of trading (9:30 ET).\" (V.26:13 p.18)",
            "F|\"Overnight trading session (OVS) - Today's opening price minus yesterday's closing price ... Regular trading session (RTS) - Today's closing price minus today's opening price.\" (p.19)",
            "X|The regular session of that day: out at its close (Figure 6, '1 Day')."],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let id = REVIEWED.iter().find(|r| **r == id)?;
    Some((*id, "Trongone corrective recovery system (S&C)", "FXI, 1% correction"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    reviewed(id).then(|| vec![ParamDef { key: "thr", label: "Session loss, % of the open (1 printed; 3 = the stronger case)", def: 1., min: 0.5, max: 5., step: 0.5 }])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let thr = p.get("thr").copied().unwrap_or(1.0);
    let mut pos = Book::new();
    for t in from + 1..=to {
        if 100.0 * (x.c[t - 1] - x.o[t - 1]) / x.o[t - 1] <= -thr && x.o[t] > x.c[t - 1] { pos.enter(1.0, t, x.o[t], "open"); pos.exit(t, x.c[t], "close"); }
    }
    Some(pos.done(to, x.c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn crs_day_trades() {
        let r = super::super::sctraub::tests::bars(3000);
        let x = Ctx::new(&r, &r, &[]);
        let p: Params = [("thr".to_string(), 0.5)].into_iter().collect();
        let t = run("sctrcrs_fxi", &x, 0, r.len() - 1, &p).unwrap();
        assert!(t.iter().all(|z| z.exit_i == z.legs[0].i && x.o[z.exit_i] > x.c[z.exit_i - 1]));
    }
}

// Instrument admission: S&C Trongone CRS 2026-11-02 (titles lane K13).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "sctrcrs" && reviewed(def.id),
    label: "Anthony Trongone tested the rule on FXI (2004-2007), the default. Other indices, ETFs and stocks, US and India, are a declared application.",
    basis: "Technical Analysis of Stocks & Commodities V.26:13 pp.16-21 (Following The Crowd After A One-Day Correction); independent check authors/titles-1030/K13w/k13_ref.py",
};
