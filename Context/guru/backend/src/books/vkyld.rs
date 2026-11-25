//! Mark Vakkur, "Treasury Bond Yields And The S&P 500", Technical Analysis of Stocks & Commodities V14:8 (1996) pp.364-369
//! (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V14/C08/TREASUR.pdf; text k3w/text/Q414/treasur.txt).
//! Book id `vkyld`. Lane K3 (25 Nov 2026, R4 #414 ledger A2121: 'Treasury yield series not held' was stale, FRED DGS30 is held). Reference authors/conv-1123/k3w/vkyld_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "vkyld";
pub const REVIEWED: &[&str] = &["vkyld_cash"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Adapted: 4 unstated details + 1 declared adaptation, R4 #400/#414] Vakkur's 'Rising Yield Cash System based on last month's yield' (Figure 4, p.366): be 100% in cash when last month's closing 30-year Treasury yield was 0.1% (a tenth of a percentage point) or more above its six-month arithmetic moving average, otherwise 100% in the S&P 500; the moving average uses the yield on the last day of each of the prior six months (p.364-365). The article tests monthly closes 1950-1995, dividends ignored, cash earning the 90-day bill. Unstated, one per item: (1) a month's closing yield = the last published daily FRED DGS30 value of that calendar month (bond-market holidays are not filled); (2) the six-month average is the mean of the six month-end yields BEFORE last month's (the 'past six months' compared with the 'current yield'), so the test is y[m-1] - mean(y[m-7..m-2]) >= 0.1; (3) the switch is made at the open of the first trading day of the new month (the month-end yield is known at that close; no look-ahead), and a run that starts in stocks enters at the open of the window's second bar; (4) a month with no usable yield (the FRED 30-year series has a hole from Feb 2002 to Feb 2006 and starts in Feb 1977) gives no cash signal: the card stays in stocks. Declared adaptation: cash = flat, the Treasury-bill interest is not credited, price only, no costs. Figures 2-5 (e.g. $10,000 -> $1,323,359 for Figure 4, 1950-1995) use monthly S&P 500 closes from 1950 and cannot be reproduced on the vault's S&P data (1983-), so nothing is compared.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "vkyld_cash", name: "Vakkur rising-yield cash system: out of stocks when last month's 30-year yield is 0.1 above its six-month average", group: "S&C: Treasury Bond Yields and the S&P 500 (Vakkur, V14:8)", session: 0, dir: "Long",
        status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &["DGS30"], book: BOOK,
        rules: &[
            "F|\"I used the moving average of the yield on the last day of each of the prior six months\" (p.365); yield minus its six-month arithmetic moving average (p.364-365)",
            "E|\"One in 100% stocks ... moving back into 100% stocks\" when the yield is not 0.1% or more above its moving average (Figure 2, p.365)",
            "X|\"100% in cash when last month's Treasury yield was 0.1% or more above its six-month moving average\" (Figure 4, p.366)",
        ], notes: NOTE, claim: None }]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { reviewed(id).then_some(("vkyld", "Vakkur rising-yield cash system", "Figure 4")) }
pub fn params(_id: &str) -> Option<Vec<ParamDef>> { None }

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, _p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from > to || to >= x.c.len() { return None; }
    let y = x.series("DGS30")?;
    let dates: Vec<&str> = (0..=to).map(|i| x.r[i].date.as_str()).collect();
    // month keys and the last finite yield of each month
    let (mut keys, mut me): (Vec<&str>, Vec<f64>) = (vec![], vec![]);
    for i in 0..=to {
        let k = &dates[i][..7];
        if keys.last() != Some(&k) { keys.push(k); me.push(f64::NAN); }
        if y[i].is_finite() { *me.last_mut().unwrap() = y[i]; }
    }
    let cash: Vec<bool> = (0..keys.len()).map(|m| {
        if m < 7 || !me[m].is_finite() { return false; }
        let w = &me[m - 6..m]; // the six month-ends before month m
        if w.iter().any(|v| !v.is_finite()) { return false; }
        me[m] - w.iter().sum::<f64>() / 6.0 >= 0.1 - 1e-12
    }).collect();
    // position held during month m: out of stocks when the previous month's verdict is cash; fills at the open of the month's first bar
    let mut pos = vec![0i8; to + 1];
    let mut m = 0usize;
    for i in 0..=to {
        while keys[m] != &dates[i][..7] { m += 1; }
        pos[i] = if m >= 1 && cash[m - 1] { 0 } else { 1 };
    }
    // the first bars before any verdict are stocks too; target_trades(next_open = true) executes tgt[i] at the open of bar i+1: shift left by one
    // bars before the run window are flat, so a run that starts in stocks enters at the open of the window's second bar (the first bar's close is the first order time)
    let tgt: Vec<i8> = (0..=to).map(|i| if i < from { 0 } else if i < to { pos[i + 1] } else { pos[i] }).collect();
    Some(super::rbtie::target_trades(x, &tgt, from, to, true, "monthly yield signal"))
}

pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == BOOK && reviewed(def.id),
    label: "Mark Vakkur's yield-trend switch is an S&P 500 rule; it runs on the S&P 500 and SPY (and, as a declared application, the Dow DJI and DIA) with the FRED 30-year Treasury yield, US only (there is no Indian yield series, R4 #414).",
    basis: "Mark Vakkur, Treasury Bond Yields and the S&P 500, Stocks & Commodities V14:8 pp.364-369 (VOLUMES/V14/C08/TREASUR.pdf); FRED DGS30 (extra_data/us)",
};
