//! Alex Krzhechevsky, "WWW Trading System" (webtradingsystem.com; research share USB HDD 2/Trading Knowledge 3/FOREX WWW Trading
//! System $99 (WebTradingSystem.com)/system.pdf, opened with the password the vendor ships in the folder's key.txt (R4 decision 66);
//! rules PDF p.8-13, text authors/batch-g/G1w/www_system.txt). Book id `wwwts`. EMA 5/13 cross with an exit on the cross back,
//! coded as a daily declared application (decision 67). Lane K12 (titles loop, 2 Nov 2026), to-code row 193; independent check
//! authors/titles-1030/K12w/k12_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::msws::ema;
use super::tselhs::Book;

/// Admitted ids (authors/titles-1030/K12w/k12_ref.py (header)).
pub const REVIEWED: &[&str] = &["wwwts_ema5_13"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const WWW: &str = "[Declared readings] 'Buy when EMA5 has crossed EMA13 from below. Sell when EMA5 crosses EMA13 from above. Wait until the time interval (selected by you) closes. If by this time EMA5 and EMA13 have crossed with each other, enter the market at the price of opening the following time interval.' 'Close the position in case: a) the closing price showed crossing or equality of EMA5 and EMA13'. Read: EMA of the close (alpha 2/(n+1), started at the first close); a cross up = EMA5 above EMA13 at this close and at or below it at the last one (cross down mirrored); long from the next open after a cross up, short from the next open after a cross down; a long is closed at the next open after a close with EMA5 at or below EMA13 (short: at or above), and the opposite cross on that close enters the other way at the same open. 'Sell' is read as a short entry, as the author trades forex both ways. The author: 'These rules can be applied to all time periods' (best on one-hour USD/CHF); daily bars on indices, ETFs and US/India stocks are a declared application (lead decision 67), as forex hourly data is not held. HELD, in pips, not converted (decisions 19, 67): the stop 50 pips from EMA13, the 'more than 100 pips' skip rule, the 1-pip EMA gap filter and the position-size formula; no stop is applied. The same 5/13 cross on other cards is a different source (decision 48). One unit; the vault's standard costs.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "wwwts_ema5_13", name: "WWW Trading System: EMA 5 crosses EMA 13, long or short at the next open, out on the cross back", group: "WWW Trading System (Krzhechevsky)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "wwwts", rules: &[
            "F|\"I take from Fibonacci sequence numbers 5 and 13 as the parameters for moving average ... use Exponential Moving Average (EMA) 5 and 13\" (p.8)",
            "E|\"Buy when EMA5 has crossed EMA13 from below. Sell when EMA5 crosses EMA13 from above. Wait until the time interval (selected by you) closes. If by this time EMA5 and EMA13 have crossed with each other, enter the market at the price of opening the following time interval.\" (p.8)",
            "X|\"Close the position in case: a) the closing price showed crossing or equality of EMA5 and EMA13; b) the prices reached the level of your stop order.\" (p.9; the pip stop is held)"],
        notes: WWW, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let id = REVIEWED.iter().find(|r| **r == id)?;
    Some((*id, "WWW Trading System", "EMA 5/13 cross"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> { reviewed(id).then(Vec::new) }

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, _p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let (o, c) = (&x.o, &x.c);
    let (f, s) = (ema(c, 5), ema(c, 13));
    let mut pos = Book::new();
    let (mut exit, mut entry) = (false, 0.);
    for t in from..=to {
        if std::mem::take(&mut exit) { pos.exit(t, o[t], "EMA cross back"); }
        if entry != 0. { pos.enter(entry, t, o[t], "open"); entry = 0.; }
        if t == to || t == 0 { continue; }
        let d = pos.side();
        exit = (d > 0. && f[t] <= s[t]) || (d < 0. && f[t] >= s[t]);
        if f[t] > s[t] && f[t - 1] <= s[t - 1] { entry = 1.; } else if f[t] < s[t] && f[t - 1] >= s[t - 1] { entry = -1.; }
    }
    Some(pos.done(to, c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn ema_5_13_cross() {
        let r: Vec<Row> = (0..900).map(|i| { let f = i as f64; let c = 100. + 9. * (f / 17.).sin() + 4. * (f / 5.3).cos() + 0.02 * f;
            Row { ticker: "T".into(), date: format!("{:05}", i), open: c - 0.6 * (f / 2.1).sin(), high: c + 1.3, low: c - 1.1, close: c, volume: None } }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let t = run("wwwts_ema5_13", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() > 10 && t.iter().any(|z| z.dir > 0.) && t.iter().any(|z| z.dir < 0.));
        let (f, s) = (ema(&x.c, 5), ema(&x.c, 13));
        for z in &t {
            let i = z.legs[0].i;
            assert_eq!(z.legs[0].px, x.o[i]);
            assert!(z.dir * (f[i - 1] - s[i - 1]) > 0. && z.dir * (f[i - 2] - s[i - 2]) <= 0.); // crossed on the close before
        }
        assert!(family_of("wwwts_ema5_13").is_some() && params("wwwts_ema5_13").is_some() && run("wwwts_x", &x, 0, 5, &Params::new()).is_none());
    }
}

// Instrument admission: WWW Trading System 2026-11-02 (titles lane K12).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "wwwts" && reviewed(def.id),
    label: "Alex Krzhechevsky's forex system (best on one-hour USD/CHF, any time frame by his text), with no printed test. Daily runs on indices, ETFs and stocks, US and India, are a declared application.",
    basis: "WWW Trading System (webtradingsystem.com), system.pdf p.8-13 (When to Trade Part Two, Crucial Rules); independent check authors/titles-1030/K12w/k12_ref.py",
};
