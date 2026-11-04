//! Lawrence Chan & Louis Lin, "Boosting Profitability: Advance Issues Momentum", Technical Analysis of Stocks & Commodities V.22:8
//! (August 2004) pp.28-32, with the sidebar NeoTicker code (research share Trading Knowledge I/Stocks & Commodities DVD 11.26
//! (1982 - 2008) (traders.com)/VOLUMES/V22/C08/166CHAN.pdf and 166SB.pdf). Book id `scaim`. Lane K40 (4 Nov 2026, decision
//! #204(2)); independent check authors/titles-1030/K40w/k40_ref.py.
//! The AIM series is built once at start-up from the US basket catalogue (us_dow30, Norgate closes) and served as the extra
//! series `DOW30_AIM20` (main.rs), so readiness refuses it where the basket is absent (India) and ends it where the basket ends.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::lc3::Pos;
use std::collections::BTreeMap;

pub const REVIEWED: &[&str] = &["scaim_dow30"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }
pub const SERIES: &str = "DOW30_AIM20";
pub const BASKET: &str = "us_dow30";
pub const N: usize = 20;

const NOTE: &str = "[Declared readings] AIM = the number of Dow Jones Industrial Average component stocks whose close is above their own close 20 trading days earlier ('n-days momentum = today's close - close of n trading days ago ... AIM is defined as the total number of advancing issues on any single day'; Figure 1 uses 20 days). Basket = the vault's us_dow30 list (the 30 Dow members of September 2020, Norgate daily closes to 2020-09-04), used as a fixed basket the way the article fixes its basket ('a fixed-symbol basket'); it is NOT dated index membership: before a member joined the index it still counts, and a member without prices yet (DOW from 2019, V from 2008, CRM from 2004, GS from 1999) is not counted, so early AIM values run over fewer than 30 names. Signals (sidebar code, data2(k) = AIM k bars ago): long when AIM[3] > AIM[2] > AIM[1] < AIM[0], AIM[0] - AIM[1] < 5 and AIM[0] < 20; short when AIM[3] < AIM[2] < AIM[1] > AIM[0], AIM[1] - AIM[0] < 5 and AIM[0] > 10. 'longatmarket / shortatmarket' = the next bar's open; always in the market after the first signal, one unit, reversing ('the system will be in the market all the time with a single contract'); no stops. Book: e-mini S&P futures 1/1/1998-5/30/2003, total profit $69,237.50 (Figure 4). US instruments only (the basket is US); results end with the basket data. Runs on US indices, ETFs and stocks other than the e-mini are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "scaim_dow30", name: "Chan & Lin Advance Issues Momentum: Dow 30 stocks above their 20-day-ago close, two falls then a small turn up = long, the mirror = short, always in", group: "S&C: Advance Issues Momentum (Chan & Lin, V.22:8)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[SERIES], book: "scaim", rules: &[
            "F|\"n-days momentum = today's close - close of n trading days ago. If the n-days momentum of a stock is positive, it is identified as an advancing issue. AIM is defined as the total number of advancing issues on any single day.\"; \"I will use the components of the Dow Jones Industrial Average (DJIA) as the basket.\" (pp.28-30); \"FIGURE 1: AIM WITH 20-DAY MOMENTUM\"",
            "E|\"buysignal := data2 (3) > data2 (2) and data2 (2) > data2 (1) and data2 (1) < data2 and data2 - data2 (1) < 5 and data2 < 20; sellsignal := data2 (3) < data2 (2) and data2 (2) < data2 (1) and data2 (1) > data2 and data2 (1) - data2 < 5 and data2 > 10; longatmarket (buysignal, 1); shortatmarket (sellsignal, 1);\" (sidebar)",
            "X|\"I did not use stops ... Since there are only long and short entries, the system will be in the market all the time with a single contract only.\" (p.31)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { reviewed(id).then_some(("scaim_dow30", "Chan & Lin AIM (S&C)", "Dow 30, 20-day momentum")) }

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("up", "Long only if AIM below (sidebar 20)", 20., 1., 30., 1.),
        p("dn", "Short only if AIM above (sidebar 10)", 10., 0., 29., 1.),
        p("step", "Turn smaller than (sidebar 5)", 5., 1., 30., 1.)])
}

/// AIM by date from the basket's member closes (each member's own trading days); None when the basket is not loaded.
pub fn aim_series(cat: &crate::baskets::Catalogue) -> Option<BTreeMap<String, f64>> {
    let members = cat.members(BASKET);
    if members.is_empty() { return None; }
    let mut out: BTreeMap<String, f64> = BTreeMap::new();
    for m in members {
        let rows = match cat.rows(&m.key) { Ok(r) => r, Err(e) => { eprintln!("warning: {SERIES} not built: {} {e}", m.key); return None; } };
        for i in N..rows.len() { *out.entry(rows[i].date.clone()).or_insert(0.) += (rows[i].close > rows[i - N].close) as u8 as f64; }
    }
    Some(out)
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let Some(a) = x.series(SERIES) else { return Some(vec![]) };
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d);
    let (up, dn, step) = (g("up", 20.), g("dn", 10.), g("step", 5.));
    let mut pos = Pos::new();
    let mut pend = 0.;
    for t in from..=to {
        if pend != 0. { pos.enter(pend, t, x.o[t], "open"); pend = 0.; }
        if t < 3 || t >= to { continue; }
        let (a0, a1, a2, a3) = (a[t], a[t - 1], a[t - 2], a[t - 3]);
        if !(a0.is_finite() && a1.is_finite() && a2.is_finite() && a3.is_finite()) { continue; }
        let s = if a3 > a2 && a2 > a1 && a1 < a0 && a0 - a1 < step && a0 < up { 1. }
            else if a3 < a2 && a2 < a1 && a1 > a0 && a1 - a0 < step && a0 > dn { -1. } else { 0. };
        if s != 0. && s != pos.side() { pend = s; }
    }
    Some(pos.done(to, x.c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn aim_turns_reverse_at_the_next_open() {
        let r = super::super::sctraub::tests::bars(60);
        let mut x = Ctx::new(&r, &r, &[]);
        let mut v = vec![15.; r.len()];
        v[10] = 18.; v[11] = 16.; v[12] = 14.; v[13] = 17.; // long at bar 14's open
        v[30] = 12.; v[31] = 14.; v[32] = 16.; v[33] = 13.; // short at bar 34's open
        x.extra.insert(SERIES.into(), v);
        let t = run("scaim_dow30", &x, 0, r.len() - 1, &Params::new()).unwrap();
        let got: Vec<_> = t.iter().map(|z| (z.dir, z.legs[0].i, z.exit_i)).collect();
        assert_eq!(got, vec![(1., 14, 34), (-1., 34, 59)]);
        assert!(t.iter().all(|z| z.legs[0].px == x.o[z.legs[0].i]));
    }
}

// Instrument admission: S&C Chan & Lin AIM 2026-11-04 (titles lane K40). India is refused by readiness (no DOW30_AIM20 there).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "scaim" && reviewed(def.id),
    label: "Lawrence Chan and Louis Lin traded the e-mini S&P on the momentum breadth of the 30 Dow stocks (1998-2003). The signal is built from the vault's us_dow30 closes (the 2020 member list, not dated membership); runs on US indices, ETFs and stocks are a declared application; Indian instruments have no Dow basket.",
    basis: "Technical Analysis of Stocks & Commodities V.22:8 pp.28-32 (Boosting Profitability, Advance Issues Momentum) and the NeoTicker sidebar; independent check authors/titles-1030/K40w/k40_ref.py",
};
