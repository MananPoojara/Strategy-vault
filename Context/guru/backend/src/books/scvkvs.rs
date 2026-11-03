//! Mark Vakkur, "The Volatility Stop System", Technical Analysis of Stocks & Commodities V.17:10 (October 1999) pp.427-432
//! (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V17/C10/075VOL.pdf).
//! Book id `scvkvs`. Lane K13 (titles loop, 2 Nov 2026), to-code row 204, lead decision 76; independent check
//! authors/titles-1030/K13w/k13_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::msws::sma;
use super::sctraub::{ema0, ticks, week_groups};
use super::tselhs::Book;

pub const REVIEWED: &[&str] = &["scvkvs_volstop"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] Weekly bars built from the daily bars of each calendar week. The sell stop is the printed indicator Volatility_Stop = MaxList(c[1] - AvgTrueRange(4) x ATR_fac, iff(l[1] < plot1[1], 0, plot1[1])): the last week's close minus 2 x the 4-week average true range, never lowered unless the last week's low went under it. The buy stop for next week ('buy at this week's high plus one tick on a stop') is placed only when this week's close is above the 12-week EMA of weekly closes (seeded with the first week). Stops are worked on the daily bars of the following week: a stop fills at its price, or at the open if the open is beyond it; the exit stop is not worked on the entry day. One tick = the instrument's minimum tick (0 is an option). ATR factor 2 is the code default; the article optimised 0.5-2.5 (options). Article, DJIA February 1988 - December 1998: factor 2, 25 trades, 44% profitable, +3,555 points (2.5: 16 trades, 63%, +4,603). Our DJI run over the same span: factor 2, 28 trades, 13 winners, +3,362 points; 2.5: 15 trades, 10 winners, +5,149 points (DJI opens before 2016 are the prior close, so gap fills are approximate). Runs on other indices, ETFs and stocks, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "scvkvs_volstop", name: "Vakkur Volatility Stop System (weekly): above the 12-week EMA buy a stop over the week's high; out at close minus 2 x 4-week ATR, ratcheting", group: "S&C: The Volatility Stop System (Vakkur, V.17:10)", session: 0, dir: "Long", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "scvkvs", rules: &[
            "E|\"if this week's close is greater than the 12-week exponential moving average of the closes, then buy at this week's high plus one tick on a stop.\" (V.17:10 p.428)",
            "X|\"Exit the long position at the volatility stop on a stop.\" Indicator: \"Inputs: ATR_len(4), ATR_fac(2) Volatility_Stop = MaxList ( c[1] - AvgTrueRange(ATR_len)*ATR_fac, iff(l[1] < plot1[1], 0, plot1[1]))\" (sidebar, p.430)",
            "F|\"the volatility stop cannot go down, only up or sideways, unless penetrated by the bar's low\" (p.428); weekly bars, four-week lookback"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let id = REVIEWED.iter().find(|r| **r == id)?;
    Some((*id, "Vakkur Volatility Stop System (S&C)", "Weekly, ATR factor 2"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("fac", "ATR factor (2 = code default; tested 0.5-2.5)", 2., 0.5, 3., 0.5), p("tick", "Buy stop offset in ticks (1 printed; 0 = none)", 1., 0., 10., 1.)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let (fac, k0) = (p.get("fac").copied().unwrap_or(2.0), ticks(x, p));
    let g = week_groups(x);
    let wh: Vec<f64> = g.iter().map(|&(s, e)| super::lc3::hi(&x.h, s, e)).collect();
    let wl: Vec<f64> = g.iter().map(|&(s, e)| super::lc3::lo(&x.l, s, e)).collect();
    let wc: Vec<f64> = g.iter().map(|&(_, e)| x.c[e]).collect();
    let tr: Vec<f64> = (0..g.len()).map(|k| if k == 0 { wh[0] - wl[0] } else { wh[k].max(wc[k - 1]) - wl[k].min(wc[k - 1]) }).collect();
    let atr = sma(&tr, 4);
    let e12 = ema0(&wc, 12);
    let mut vs = vec![f64::NAN; g.len()];
    for k in 4..g.len() {
        let prev = if vs[k - 1].is_finite() { vs[k - 1] } else { 0.0 };
        vs[k] = (wc[k - 1] - fac * atr[k - 1]).max(if wl[k - 1] < prev { 0.0 } else { prev });
    }
    let mut wk = vec![0usize; x.r.len()];
    for (k, &(s, e)) in g.iter().enumerate() { for i in s..=e { wk[i] = k; } }
    let mut pos = Book::new();
    let mut level: Option<(usize, f64)> = None;
    let mut ent = usize::MAX;
    for i in from..=to {
        let k = wk[i];
        match level {
            Some((w, lv)) if pos.side() == 0.0 && w == k && x.h[i] >= lv => { pos.enter(1.0, i, x.o[i].max(lv), "stop"); ent = i; }
            _ => if pos.side() > 0.0 && i > ent && vs[k].is_finite() && x.l[i] <= vs[k] { pos.exit(i, x.o[i].min(vs[k]), "volatility stop"); }
        }
        if i == g[k].1 && i < to && k >= 11 && pos.side() == 0.0 && wc[k] > e12[k] { level = Some((k + 1, wh[k] + k0)); }
    }
    Some(pos.done(to, x.c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn volatility_stop_weekly() {
        let r = super::super::sctraub::tests::bars(3000);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("scvkvs_volstop", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() > 3 && t.iter().all(|z| z.dir > 0.0 && z.exit_i > z.legs[0].i));
    }
}

// Instrument admission: S&C Vakkur volatility stop 2026-11-02 (titles lane K13).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "scvkvs" && reviewed(def.id),
    label: "Mark Vakkur tested the weekly system on the DJIA (1988-1998), Best Buy, Micron and two Fidelity funds; DJI is the article's own index. Runs on other indices, ETFs and stocks, US and India, are a declared application.",
    basis: "Technical Analysis of Stocks & Commodities V.17:10 pp.427-432 (The Volatility Stop System); independent check authors/titles-1030/K13w/k13_ref.py",
};
