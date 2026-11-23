//! Jani, "Simulating Trading Portfolios", Technical Analysis of Stocks & Commodities V.12:3 (Mar 1994), pp. 91-97, rule on p. 92
//! (research share Trading Knowledge I/Stocks & Commodities DVD 11.26/VOLUMES/V12/C03/SIMULAT.PDF; the same rule is reprinted in V.12:4 pp. 165-166).
//! Book id `jani`. Lane K2 (23 Nov 2026, queue R4 #381). Daily bars. The article tests it on six commodity and currency futures (1981-91, six ticks
//! slippage and $50 commission); here it runs on the NSE index series as a labelled #340 adaptation, UNVERIFIED.
use crate::books::rgcts::sma;
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "jani";
const ID: &str = "jani_ma525";

const NOTE: &str = "A|Declared readings (lane K2), 4: (1) the moving averages are simple averages of the close; (2) the cross is seen at the close and the order is filled at the next open ('on the open'); (3) the 50-day stop is the 50-day average of the previous close, checked from the day after entry, filled at the stop or at the open when the day gaps through it, and after a stop-out the system stays flat until the next 5/25 cross; (4) the opposite 5/25 cross closes the open trade at the next open and reverses it (the article states no other exit). Market adaptation: forex/commodity futures original run on index bars, no slippage or commission.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: ID, name: "5/25-day MA cross with 50-day MA stop", group: "Simulating Trading Portfolios (Jani, 1994)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
        rules: &["E|\"Go long on the open when the five-day moving average crosses over the 25-day moving average. Go short on the open when the five-day moving average crosses under the 25-day moving average.\" (p. 92)",
            "X|\"Once a trade has been initiated in either direction, use the 50-day moving average as the stop-loss level.\" (p. 92)", NOTE],
        notes: "Jani, S&C V.12:3 p. 92. The article prints $197,204 total profit over six futures 1981-91 (120 signals); not comparable with an index run.", claim: None }]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { (id == ID).then_some(("jani", "5/25 MA cross, 50-day stop", "")) }
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    (id == ID).then(|| vec![ParamDef { key: "fast", label: "Fast MA (days)", def: 5., min: 2., max: 20., step: 1. }, ParamDef { key: "slow", label: "Slow MA (days)", def: 25., min: 10., max: 100., step: 1. },
        ParamDef { key: "stop", label: "Stop MA (days)", def: 50., min: 20., max: 200., step: 1. }])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if id != ID || to >= x.c.len() || from >= to { return None; }
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d).round().max(1.0) as usize;
    let (nf, ns, nt) = (g("fast", 5.0), g("slow", 25.0), g("stop", 50.0));
    let (f, s, t) = (sma(&x.c, nf), sma(&x.c, ns), sma(&x.c, nt));
    let warm = nf.max(ns).max(nt); // first index with every average defined is warm-1
    let mut out: Vec<Trade> = vec![];
    let mut pos: Option<(f64, usize, f64)> = None;
    let leg = |ei: usize, ep: f64| vec![Leg { i: ei, px: ep, w: 1.0 }];
    for d in from.max(warm + 1)..=to {
        let (a, b) = (d - 1, d - 2);
        let sig = if f[a] > s[a] && f[b] <= s[b] { 1.0 } else if f[a] < s[a] && f[b] >= s[b] { -1.0 } else { 0.0 };
        if let Some((dir, ei, ep)) = pos {
            if sig != 0.0 && sig != dir {
                out.push(Trade { dir, legs: leg(ei, ep), exit_i: d, exit_px: x.o[d], reason: "opposite 5/25 cross".into(), entry_kind: "open", signal_i: ei, open: false, exits: Vec::new() });
                pos = Some((sig, d, x.o[d]));
            } else if d > ei {
                let stop = t[d - 1];
                let hit = if dir > 0.0 { x.l[d] <= stop } else { x.h[d] >= stop };
                if hit {
                    let px = if dir > 0.0 { x.o[d].min(stop) } else { x.o[d].max(stop) };
                    out.push(Trade { dir, legs: leg(ei, ep), exit_i: d, exit_px: px, reason: "50-day MA stop".into(), entry_kind: "open", signal_i: ei, open: false, exits: Vec::new() });
                    pos = None;
                }
            }
        } else if sig != 0.0 { pos = Some((sig, d, x.o[d])); }
    }
    if let Some((dir, ei, ep)) = pos { out.push(Trade { dir, legs: leg(ei, ep), exit_i: to, exit_px: x.c[to], reason: "open".into(), entry_kind: "open", signal_i: ei, open: true, exits: Vec::new() }); }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sane_runs() { super::super::nijat::tests::sane(run, &[ID]); }
    /// hand-run dump for the pandas reference: JANI_CSV=<idx_data.csv> JANI_OUT=<file> cargo test --release jani_dump -- --ignored
    #[test]
    #[ignore]
    fn jani_dump() {
        use std::io::Write;
        let data = crate::load_csv(&std::env::var("JANI_CSV").unwrap()).unwrap();
        let mut f = std::io::BufWriter::new(std::fs::File::create(std::env::var("JANI_OUT").unwrap()).unwrap());
        writeln!(f, "id,ticker,dir,entry_date,entry_px,exit_date,exit_px,open").unwrap();
        for (t, rows) in data.iter() {
            let x = Ctx::new(rows, &[], &[]);
            for z in run(ID, &x, 0, rows.len() - 1, &Params::new()).unwrap() {
                writeln!(f, "{ID},{t},{},{},{},{},{},{}", z.dir, rows[z.legs[0].i].date, z.legs[0].px, rows[z.exit_i].date, z.exit_px, z.open).unwrap();
            }
        }
    }
}

pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "jani",
    label: "Daily moving-average cross with no currency amount; run and independently checked on the five NSE index series only (NIFTY 50, NIFTYBANK, NIFTYIT, NIFTYMIDCAP100, NIFTYMIDCAPSELECT).",
    basis: "Jani, Simulating Trading Portfolios (S&C V.12:3), p. 92; independent check authors/conv-1123/k2w/jani_ref.py",
};
