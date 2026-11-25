//! Adam White, "A New Exit Strategy", Technical Analysis of Stocks & Commodities V.11:11 (November 1993) pp.462-466
//! (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V11/C11/ANEWEXI.pdf).
//! Book id `scwhitechl`: 10-bar close breakout entry with the CHL/LLF exit (long only, as tested). Lane I2 (Q385, 24 Nov 2026, R4 #382);
//! independent check authors/conv-1123/i2w/scwhitechl_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::sctraub::next_open;

const ID: &str = "scwhitechl_long";
pub const REVIEWED: &[&str] = &[ID];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "A|Declared readings (lane I2; R4 #340/#348/#382). Card-specific unstated choices, one per number: (1) breakout = the close exceeds the highest close of the previous 10 bars (strictly); (2) long side only, as tested (the article illustrates and tests only long positions); (3) the exit is tested at every close from the entry bar on, both conditions on the same bar; (4) ties count: HHV(L,5) = HHV(L,19) and L - Ref(L,-1) equal to its lowest value of the last 17 bars both test equality, as in the printed MetaStock formula; (5) no stop. Honest total: 5 card-specific choices. Engine-wide defaults / declared adaptations (labelled, not counted, R4 #404): one position at a time; entry and exit are filled at the next open, as the article does; the NSE index series substituted for the Treasury bond futures test; no costs. The article tests 1,000 daily bars of Treasury bond futures (14 wins, 5 losses, optimised 5/19/17); the vault applies the printed rule to the NSE index series, declared adaptation, no costs.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: ID, name: "CHL/LLF exit (White): 10-bar close breakout entry, exit when the 5/19-bar concurrent highest lows filter and the 17-bar low-to-low fall trigger agree", group: "S&C: A New Exit Strategy (White, V.11:11)", session: 0, dir: "Long", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "scwhitechl",
        rules: &[
            "E|\"I arbitrarily decided that the entry would be a standard 10-bar breakout based on closes. Both the entry and exits were taken on the open after the previous day's action generated a signal.\" (V.11:11, PDF p.4)",
            "X|\"The filter is positive whenever the highest low of the last five bars is also the highest low of the last 19 bars.\" (PDF p.2); \"we look for the largest low to low fall of the last x bars of the oscillator ... I used 17 bars\" (PDF p.2); \"Both the filter and trigger must concur\" (PDF p.2)",
            "F|\"when(hhv(L,opt1),=,(hhv(L,opt2))) and when(fm1(#1),=,llv(fm1(#1),opt3)) fm1(#1) is L-ref(L,-1) I used opt1 as 5, opt2 as 19, and opt3 as 17.\" (PDF p.3)",
            NOTE],
        notes: "White, S&C V.11:11 pp.462-466: CHL/LLF exit. Declared adaptation (R4 #340): the Treasury-bond test is run on the NSE index series; six card-specific choices listed in the card note, long only.",
        claim: None }]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { reviewed(id).then_some((ID, "CHL/LLF exit (White, S&C)", "10 / 5-19 / 17")) }
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    reviewed(id).then(|| vec![
        ParamDef { key: "brk", label: "Entry: close above the highest close of N bars (article 10)", def: 10., min: 2., max: 60., step: 1. },
        ParamDef { key: "chs", label: "CHL: short lows window (article 5)", def: 5., min: 2., max: 20., step: 1. },
        ParamDef { key: "chl", label: "CHL: long lows window (article 19)", def: 19., min: 6., max: 60., step: 1. },
        ParamDef { key: "llf", label: "LLF: largest low-to-low fall of N bars (article 17)", def: 17., min: 3., max: 60., step: 1. }])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.c.len() { return None; }
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d).round().max(2.) as usize;
    let (nb, ns, nl, nf) = (g("brk", 10.), g("chs", 5.), g("chl", 19.), g("llf", 17.));
    let (l, c) = (&x.l, &x.c);
    let hhv = |t: usize, n: usize| l[t + 1 - n..=t].iter().copied().fold(f64::NEG_INFINITY, f64::max);
    Some(next_open(x, from, to, "CHL/LLF exit", |t, side| {
        if side > 0. {
            if t < nl.max(nf + 1) { return side; }
            let dl = |i: usize| l[i] - l[i - 1];
            let low_fall = (t + 1 - nf..=t).map(dl).fold(f64::INFINITY, f64::min);
            if hhv(t, ns) == hhv(t, nl) && dl(t) == low_fall { 0.0 } else { side }
        } else if t >= nb && c[t] > c[t - nb..t].iter().copied().fold(f64::NEG_INFINITY, f64::max) { 1.0 } else { side }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn breakout_in_chl_llf_out() {
        let r = super::super::sctraub::tests::bars(2500);
        let x = Ctx::new(&r, &r, &[]);
        let t = run(ID, &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() > 10 && t.iter().all(|z| z.dir > 0.));
        for w in t.windows(2) { assert!(w[1].legs[0].i >= w[0].exit_i); }
    }
    /// hand-run dump for the pandas reference: Q385_CSV=<idx_data.csv> Q385_OUT=<file> cargo test --release scwhitechl_dump -- --ignored
    #[test]
    #[ignore]
    fn scwhitechl_dump() {
        use std::io::Write;
        let data = crate::load_csv(&std::env::var("Q385_CSV").unwrap()).unwrap();
        let mut f = std::io::BufWriter::new(std::fs::File::create(std::env::var("Q385_OUT2").unwrap()).unwrap());
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
    reviewed: |def: &crate::connors::StrategyDef| def.book == "scwhitechl",
    label: "White tested the CHL/LLF exit on 1,000 daily bars of Treasury bond futures; the vault runs the printed rule on the five NSE index series (NIFTY 50, NIFTYBANK, NIFTYIT, NIFTYMIDCAP100, NIFTYMIDCAPSELECT), a declared adaptation, indices only.",
    basis: "Technical Analysis of Stocks & Commodities V.11:11 pp.462-466 (A New Exit Strategy); independent check authors/conv-1123/i2w/scwhitechl_ref.py",
};
