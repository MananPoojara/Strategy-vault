//! Ryan Watts, "The Slingshot Trading Method" (research share Trading Knowledge I/Ryan Watts - Scalping the Nasdaq Emini Futures
//! Method/TheSlingshotTradingMethod.pdf, PDF pp. 9-11, 15-16). Book id `rwsl`. Lane K2 (23 Nov 2026). Daily bars (the author: intraday
//! use "proves ineffective", p. 11). Momentum_n = close - close n days ago (the "Slingshot" histogram); LowMom_n = low - low n days ago
//! (the signal line). Buy when Momentum drops below LowMom; sell short when it spikes above; "exit when an opposite signal is given".
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "rwsl";
const IDS: [&str; 2] = ["rwsl_n5", "rwsl_n7"];

const NOTE: &str = "A|Declared readings (lane K2): the author uses the 3-, 5-, 7- and 13-day momentum and picks per stock by eye (p. 9), so the 5-day and 7-day lines are two separate cards, neither optimised; a signal is the day Momentum_n first crosses below (above) LowMom_n, and the position is entered at that day's close (the first listed entry timing, p. 16) and held until the opposite signal (the only exit the text states for the mechanical version, p. 15), always reversing; no stop, no filter (the %R / band / divergence confirmations are optional in the text and not applied). The book prints no result. No costs.";

pub fn catalogue() -> Vec<StrategyDef> {
    let mk = |id: &'static str, name: &'static str, n: u32| StrategyDef { id, name, group: "The Slingshot Trading Method", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
        rules: Box::leak(vec![Box::leak(format!("F|Momentum = close - close {n} days ago; LowMom = low - low {n} days ago (pp. 9-10)").into_boxed_str()) as &'static str,
            "E|Buy when Momentum drops below LowMom; sell short when it spikes above LowMom (p. 10); enter at that day's close (p. 16)",
            "X|Hold until the opposite signal, then reverse (p. 15)", NOTE].into_boxed_slice()),
        notes: "Ryan Watts, The Slingshot Trading Method, daily bars. Nothing is printed as a result.", claim: None };
    vec![mk(IDS[0], "Slingshot 5-day (momentum vs low momentum)", 5), mk(IDS[1], "Slingshot 7-day (momentum vs low momentum)", 7)]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { IDS.contains(&id).then_some(("rwsl", "Slingshot momentum cross", "")) }
pub fn params(id: &str) -> Option<Vec<ParamDef>> { IDS.contains(&id).then(Vec::new) }

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, _p: &Params) -> Option<Vec<Trade>> {
    if !IDS.contains(&id) || to >= x.c.len() || from >= to { return None; }
    let n: usize = if id == IDS[0] { 5 } else { 7 };
    let mut out: Vec<Trade> = vec![];
    let mut pos: Option<(f64, usize, f64)> = None;
    let diff = |i: usize| (x.c[i] - x.c[i - n]) - (x.l[i] - x.l[i - n]); // Momentum - LowMom
    for i in from.max(n + 1)..=to {
        let (d, dp) = (diff(i), diff(i - 1));
        let sig = if d < 0.0 && dp >= 0.0 { 1.0 } else if d > 0.0 && dp <= 0.0 { -1.0 } else { 0.0 };
        if sig == 0.0 || pos.map_or(false, |p| p.0 == sig) { continue; }
        if let Some((dir, ei, ep)) = pos.take() {
            out.push(Trade { dir, legs: vec![Leg { i: ei, px: ep, w: 1.0 }], exit_i: i, exit_px: x.c[i], reason: "opposite Slingshot signal".into(), entry_kind: "close", signal_i: ei, open: false, exits: Vec::new() });
        }
        pos = Some((sig, i, x.c[i]));
    }
    if let Some((dir, ei, ep)) = pos { out.push(Trade { dir, legs: vec![Leg { i: ei, px: ep, w: 1.0 }], exit_i: to, exit_px: x.c[to], reason: "open".into(), entry_kind: "close", signal_i: ei, open: true, exits: Vec::new() }); }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn slingshot_signals_alternate() {
        super::super::nijat::tests::sane(run, &IDS);
    }
    /// hand-run dump for the pandas reference: RWSL_CSV=<idx_data.csv> RWSL_OUT=<file> cargo test --release rwsl_dump -- --ignored
    #[test]
    #[ignore]
    fn rwsl_dump() {
        use std::io::Write;
        let data = crate::load_csv(&std::env::var("RWSL_CSV").unwrap()).unwrap();
        let mut f = std::io::BufWriter::new(std::fs::File::create(std::env::var("RWSL_OUT").unwrap()).unwrap());
        writeln!(f, "id,ticker,dir,entry_date,entry_px,exit_date,exit_px,open").unwrap();
        for (t, rows) in data.iter() {
            let x = Ctx::new(rows, &[], &[]);
            for id in IDS { for z in run(id, &x, 0, rows.len() - 1, &Params::new()).unwrap() {
                writeln!(f, "{id},{t},{},{},{},{},{},{}", z.dir, rows[z.legs[0].i].date, z.legs[0].px, rows[z.exit_i].date, z.exit_px, z.open).unwrap();
            } }
        }
    }
}

pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "rwsl",
    label: "Daily close-to-close momentum cross with no currency amounts, written for stocks and stock indexes (p. 3): runs on daily indices, ETFs and stocks, US and India, as a declared application.",
    basis: "The Slingshot Trading Method (Ryan Watts), pp. 9-11, 15-16; independent check authors/conv-1123/k2w/rwsl_ref.py",
};
