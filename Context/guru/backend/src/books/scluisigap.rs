//! Joe Luisi and Jayanthi Gopalakrishnan, "What You Need To Know About Gaps", Technical Analysis of Stocks & Commodities V.18:9
//! (September 2000) pp.32-39 (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V18/C09/081GAP.pdf).
//! Book id `scluisigap`: the Figure 8 gap-fade day trade (sell up-gaps at the open, buy down-gaps at the open, exit on the close, five-point stop).
//! Lane I2 (Q385 row 66, 24 Nov 2026, R4 #382); independent check authors/conv-1123/i2w/scluisigap_ref.py.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

const ID: &str = "scluisigap_fade";
pub const REVIEWED: &[&str] = &[ID];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "A|Declared readings (lane I2, 3; R4 #340/#348/#380/#382): (1) a gap up = the open is above the previous bar's high (a gap down: below the previous low); the article's trade rule does not print a threshold; (2) the five-point stop is carried as ONE price-relative rule: stop distance = 5 x previous close / 1,400 (the S&P 500 traded about 1,400 in the Figure 8 trades); (3) the stop is a daily-bar test: a short is stopped when the high reaches open + distance (a long when the low reaches open - distance), filled at the stop; otherwise exit at the close. Entry is at the open, so the stop never lies behind the entry and the daily path has no same-bar order ambiguity (#240 n/a). The article's Figure 8 is the S&P 500 contract December 1999 to March 14, 2000 (19 trades, 137.02 points); the vault applies the printed rule to the NSE index series, declared adaptation, no costs.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: ID, name: "Gap fade (Luisi): sell up-gaps and buy down-gaps at the open, exit on the close, fixed stop scaled to price", group: "S&C: What You Need To Know About Gaps (Luisi, V.18:9)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "scluisigap",
        rules: &[
            "E|\"1 Sell the upgaps at the open. 2 Buy the downgaps at the open.\" (V.18:9, PDF p.5, trade strategy for Figure 8)",
            "X|\"3 Exit on the close. 4 Use a five-point stop-loss.\" (PDF p.5)",
            "F|\"Some traders prefer to trade against the gap; if the market has a gap up at the open, you would sell and then exit on the close, or vice versa.\" (PDF p.5); Figure 8: 19 trades, 11 profitable, 137.02 points (PDF p.5)",
            NOTE],
        notes: "Luisi and Gopalakrishnan, S&C V.18:9: gap-fade day trade. Three labelled readings; declared adaptation to the NSE index series.",
        claim: None }]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { reviewed(id).then_some((ID, "Gap fade (Luisi, S&C)", "5-point stop")) }
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    reviewed(id).then(|| vec![ParamDef { key: "pts", label: "Stop distance in S&P points at a level of 1,400 (article 5)", def: 5., min: 1., max: 30., step: 0.5 }])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.c.len() { return None; }
    let pts = p.get("pts").copied().unwrap_or(5.);
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let mut out = vec![];
    for d in from.max(1)..=to {
        let k = pts * c[d - 1] / 1400.;
        let dir = if o[d] > h[d - 1] { -1. } else if o[d] < l[d - 1] { 1. } else { continue };
        let st = o[d] - dir * k; // short: open + k, long: open - k
        let (hit, why) = if dir < 0. { (h[d] >= st, "stop") } else { (l[d] <= st, "stop") };
        let (xp, why) = if hit { (st, why) } else { (c[d], "close") };
        out.push(Trade { dir, legs: vec![Leg { i: d, px: o[d], w: 1.0 }], exit_i: d, exit_px: xp, reason: why.into(), entry_kind: "open", signal_i: d, open: false, exits: Vec::new() });
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn gap_fade_day_trades() {
        let r = super::super::sctraub::tests::bars(1500);
        let x = Ctx::new(&r, &r, &[]);
        let t = run(ID, &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.iter().all(|z| z.exit_i == z.legs[0].i));
        for z in &t { assert!(z.exit_px >= r[z.exit_i].low - 1e-9 && z.exit_px <= r[z.exit_i].high + 1e-9); }
    }
    /// hand-run dump for the pandas reference: Q385_CSV=<idx_data.csv> Q385_OUT4=<file> cargo test --release scluisigap_dump -- --ignored
    #[test]
    #[ignore]
    fn scluisigap_dump() {
        use std::io::Write;
        let data = crate::load_csv(&std::env::var("Q385_CSV").unwrap()).unwrap();
        let mut f = std::io::BufWriter::new(std::fs::File::create(std::env::var("Q385_OUT4").unwrap()).unwrap());
        writeln!(f, "id,ticker,dir,entry_date,entry_px,exit_date,exit_px,open").unwrap();
        for (tk, rows) in data.iter() {
            let x = Ctx::new(rows, &[], &[]);
            for z in run(ID, &x, 0, rows.len() - 1, &Params::new()).unwrap() {
                writeln!(f, "{ID},{tk},{},{},{},{},{},{}", z.dir, rows[z.legs[0].i].date, z.legs[0].px, rows[z.exit_i].date, z.exit_px, z.open).unwrap();
            }
        }
    }
}

pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "scluisigap",
    label: "Luisi and Gopalakrishnan traded one S&P 500 contract per gap (December 1999 to March 2000); the vault runs the printed rule on the five NSE index series (NIFTY 50, NIFTYBANK, NIFTYIT, NIFTYMIDCAP100, NIFTYMIDCAPSELECT), a declared adaptation, indices only.",
    basis: "Technical Analysis of Stocks & Commodities V.18:9 pp.32-39 (What You Need To Know About Gaps); independent check authors/conv-1123/i2w/scluisigap_ref.py",
};
