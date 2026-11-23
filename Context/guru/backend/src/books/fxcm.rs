//! Forex Capital Markets (FXCM), GBP/USD Trading Strategy Workshop, "SF 1" (research share Trading Knowledge 3/Forex Capital Markets LLC -
//! GBRUSD Trading Strategy Workshop (fxcm.com)/Lesson 5.pdf p. 1; the same text is in the FXCM Trading Course copy). Book id `fxcm`.
//! Lane K2 (23 Nov 2026, queue R4 #381). The workshop trades GBP/USD on 2-hour bars; here it runs on the NSE index daily bars as a
//! labelled #340 adaptation, UNVERIFIED. The rule has no currency amount.
use crate::books::rgcts::sma;
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "fxcm";
const ID: &str = "fxcm_sf1";

const NOTE: &str = "A|Declared readings (lane K2), 4: (1) the '15, 5, 5' slow stochastic is %K = 5-bar average of 100 x (close - lowest low of 15)/(highest high of 15 - lowest low of 15) and %D = 5-bar average of %K (the text gives only the three numbers); (2) the signal is read on the close of the cross bar (both lines below 20 on that bar, both above 80 for shorts) and the trade is entered at that close; (3) the exit is the opposite %K/%D cross, taken at that bar's close, and no stop is printed; (4) the original runs on GBP/USD 2-hour bars, here daily bars of the NSE indices (no slippage or commission).";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: ID, name: "SF 1: 15,5,5 slow stochastic cross with 200-day SMA filter", group: "FXCM GBP/USD Trading Strategy Workshop", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
        rules: &["F|\"2 hour chart; 15, 5, 5 slow stochastic; 200 period SMA\" (Lesson 5, p. 1)",
            "E|\"For buy trades, the price must be above the 200 period SMA. Once the price is above the 200 period SMA, look for the slow stochastic %K line to cross above the %D line while both lines are below the 20 level. The crossover is a buy signal and we should enter a long trade at this point.\" Sell: price below the 200 SMA, %K crosses below %D with both lines above the 80 level (mirror)",
            "X|\"The trade is exited once the slow stochastic %K line crosses back below the %D line.\" (shorts: back above)", NOTE],
        notes: "FXCM workshop Lesson 5 p. 1; no result is printed.", claim: None }]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { (id == ID).then_some(("fxcm", "SF 1 stochastic cross", "")) }
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    (id == ID).then(|| vec![ParamDef { key: "k", label: "Stochastic length", def: 15., min: 5., max: 30., step: 1. }, ParamDef { key: "slow", label: "%K smoothing", def: 5., min: 1., max: 10., step: 1. },
        ParamDef { key: "d", label: "%D smoothing", def: 5., min: 1., max: 10., step: 1. }, ParamDef { key: "ma", label: "Trend SMA", def: 200., min: 50., max: 300., step: 10. }])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if id != ID || to >= x.c.len() || from >= to { return None; }
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d).round().max(1.0) as usize;
    let (nk, ns, nd, nm) = (g("k", 15.0), g("slow", 5.0), g("d", 5.0), g("ma", 200.0));
    let len = x.c.len();
    let mut raw = vec![f64::NAN; len];
    for i in nk.saturating_sub(1)..len {
        let (mut hh, mut ll) = (f64::MIN, f64::MAX);
        for j in i + 1 - nk..=i { hh = hh.max(x.h[j]); ll = ll.min(x.l[j]); }
        if hh > ll { raw[i] = 100.0 * (x.c[i] - ll) / (hh - ll); }
    }
    let k = sma(&raw, ns); let d = sma(&k, nd); let m = sma(&x.c, nm);
    let mut out: Vec<Trade> = vec![];
    let mut pos: Option<(f64, usize, f64)> = None;
    let leg = |ei: usize, ep: f64| vec![Leg { i: ei, px: ep, w: 1.0 }];
    for i in from.max(1)..=to {
        if !(k[i].is_finite() && d[i].is_finite() && k[i - 1].is_finite() && d[i - 1].is_finite() && m[i].is_finite()) { continue; }
        let up = k[i] > d[i] && k[i - 1] <= d[i - 1];
        let dn = k[i] < d[i] && k[i - 1] >= d[i - 1];
        if let Some((dir, ei, ep)) = pos {
            if (dir > 0.0 && dn) || (dir < 0.0 && up) {
                out.push(Trade { dir, legs: leg(ei, ep), exit_i: i, exit_px: x.c[i], reason: "%K crossed back through %D".into(), entry_kind: "close", signal_i: ei, open: false, exits: Vec::new() });
                pos = None;
            }
        }
        if pos.is_none() {
            if up && k[i] < 20.0 && d[i] < 20.0 && x.c[i] > m[i] { pos = Some((1.0, i, x.c[i])); }
            else if dn && k[i] > 80.0 && d[i] > 80.0 && x.c[i] < m[i] { pos = Some((-1.0, i, x.c[i])); }
        }
    }
    if let Some((dir, ei, ep)) = pos { out.push(Trade { dir, legs: leg(ei, ep), exit_i: to, exit_px: x.c[to], reason: "open".into(), entry_kind: "close", signal_i: ei, open: true, exits: Vec::new() }); }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sane_runs() { super::super::nijat::tests::sane(run, &[ID]); }
    /// hand-run dump for the pandas reference: FXCM_CSV=<idx_data.csv> FXCM_OUT=<file> cargo test --release fxcm_dump -- --ignored
    #[test]
    #[ignore]
    fn fxcm_dump() {
        use std::io::Write;
        let data = crate::load_csv(&std::env::var("FXCM_CSV").unwrap()).unwrap();
        let mut f = std::io::BufWriter::new(std::fs::File::create(std::env::var("FXCM_OUT").unwrap()).unwrap());
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
    reviewed: |def: &crate::connors::StrategyDef| def.book == "fxcm",
    label: "Daily stochastic-cross rule with no currency amount; run and independently checked on the five NSE index series only (NIFTY 50, NIFTYBANK, NIFTYIT, NIFTYMIDCAP100, NIFTYMIDCAPSELECT).",
    basis: "FXCM GBP/USD Trading Strategy Workshop, SF 1 (Lesson 5 p. 1); independent check authors/conv-1123/k2w/fxcm_ref.py",
};
