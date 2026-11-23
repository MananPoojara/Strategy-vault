//! Jay Kaeppel, "Stock timing to the discount rates" (Technical Analysis of Stocks & Commodities V.8:1, Jan 1990, p.31-32; research share Trading Knowledge I/
//! Stocks & Commodities DVD 11.26/VOLUMES/V08/C01/STOCTIM.PDF, text layer, read in full). Book id `kpdr`. Bullish signal: "11 previous cases when the Fed has cut
//! the discount rate, following one or more hikes"; bearish signal: "whenever the discount rate has been hiked three times in a row (known as the 'three steps
//! and a stumble' rule)". Lane K1 (23 Nov 2026, queue A807). Independent check authors/conv-1123/K1/kpdr_ref.py.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "kpdr";
pub const SERIES: &str = "RBI_REPO_RATE.ChangeBp";
pub const REVIEWED: &[&str] = &["kpdr_discount"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared adaptation, R4 #340] The article uses the US Federal Reserve discount rate and the S&P 500 / DJIA 1954-1989 and prints no trade list (Figure 2: the S&P 500 was higher 12 months after each of 11 cuts that followed hikes; Figure 3: 12 'three steps and a stumble' signals), so no figure is compared. The vault has no US discount-rate series; the card runs only on the NSE indices NIFTY 50 and NIFTY BANK with the verified RBI repo-rate change list (2003-2026, extra_data RBI_REPO_RATE.ChangeBp, the series the vault's as51_repo_rate card also uses), UNVERIFIED. The repo rate is the RBI's policy rate, not a discount rate: the closest available substitute. [Declared readings, each an unstated detail] (1) The printed buy is read as: go long at the close of the bar AFTER the bar of a policy change that cuts the rate when the previous rate change was a hike (AsOf event convention, R4 #372/#374: the change is known from its effective date, the announcement time is not in the data, so it is acted on at the next bar's close); the printed sell signal (the third hike in a row) ends the long at the next bar's close the same way; the article gives no other exit (the 12 months are an evaluation window, not a rule; p.hold > 0 adds an exit after that many trading bars as an option). (2) A change counts on its effective date when that date is a trading bar; a change dated on a non-trading day gives no signal and is not counted in a run of hikes. (3) Long or flat: the bearish signal is not traded short. Only a handful of signals exist in the 23 years of RBI history.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "kpdr_discount", name: "Buy a rate cut that follows a hike, sell the third hike in a row (RBI repo rate on NSE indices, long/flat)",
        group: "Kaeppel discount-rate timing", session: 0, dir: "Long", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[SERIES], book: "kpdr",
        rules: &[
            "F|\"One of the most useful interest rate measures is the Federal Reserve Board's (the Fed) discount rate\" (p.31); here the RBI repo-rate changes",
            "E|\"there have been 11 previous cases when the Fed has cut the discount rate, following one or more hikes. In every case, the S&P 500 was higher 12 months later\" (Figure 2)",
            "X|\"whenever the discount rate has been hiked three times in a row (known as the 'three steps and a stumble' rule), it was thought to be flashing a bearish signal for stock prices\" (Figure 3)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id { "kpdr_discount" => Some(("kpdr_discount", "Kaeppel discount-rate timing", "cut after hike buys, three hikes sell")), _ => None }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    match id {
        "kpdr_discount" => Some(vec![p("hikes", "Hikes in a row that end the long (article 3)", 3., 2., 6., 1.),
            p("hold", "Option: also exit after N trading bars (0 = only the printed sell signal; article's evaluation window is 12 months = 252)", 0., 0., 504., 21.)]),
        _ => None,
    }
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let s = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| s.iter().find(|d| d.key == k).unwrap().def);
    let (nh, hold) = (g("hikes").round() as usize, g("hold").round() as usize);
    let Some(bp) = x.extra.get(SERIES) else { return Some(vec![]) };
    let c = &x.c;
    let (mut out, mut cur) = (vec![], None::<(usize, f64)>);
    let (mut last_hike, mut run_len) = (false, 0usize);
    // AsOf event convention (R4 #372/#374, as51_repo_rate precedent): the change is known from the first bar dated on or after its effective date; the announcement
    // time is not in the data, so the signal read on bar t is acted on at the close of the NEXT bar (t+1).
    let (mut buy, mut sell) = (false, false);
    for t in 0..=to {
        let (act_buy, act_sell) = (buy, sell);
        let v = bp.get(t).copied().unwrap_or(f64::NAN);
        buy = false; sell = false;
        if v.is_finite() && v != 0.0 {
            if v > 0.0 { run_len = if last_hike { run_len + 1 } else { 1 }; last_hike = true; sell = run_len >= nh; }
            else { buy = last_hike; last_hike = false; run_len = 0; }
        }
        let (buy_now, sell_now) = (act_buy, act_sell);
        if t < from { continue; }
        if let Some((e, ep)) = cur {
            let why = if sell_now { Some("third hike in a row") } else if hold > 0 && t >= e + hold { Some("holding period over") } else { None };
            if let Some(w) = why {
                out.push(Trade { dir: 1., legs: vec![Leg { i: e, px: ep, w: 1. }], exit_i: t, exit_px: c[t], reason: w.into(), entry_kind: "close", signal_i: t, open: false, exits: vec![] });
                cur = None;
            }
        } else if buy_now { cur = Some((t, c[t])); }
    }
    if let Some((e, ep)) = cur {
        out.push(Trade { dir: 1., legs: vec![Leg { i: e, px: ep, w: 1. }], exit_i: to, exit_px: c[to], reason: "open".into(), entry_kind: "close", signal_i: to, open: true, exits: vec![] });
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn hike_then_cut_buys_and_third_hike_sells() {
        let r: Vec<Row> = (0..30).map(|i| Row { ticker: "NIFTY 50".into(), date: format!("2010-01-{:02}", i + 1), open: 100. + i as f64, high: 100. + i as f64, low: 100. + i as f64, close: 100. + i as f64, volume: None }).collect();
        let mut x = Ctx::new(&r, &r, &[]);
        let mut bp = vec![f64::NAN; 30];
        bp[2] = 25.; bp[5] = -25.; bp[10] = 25.; bp[12] = 25.; bp[14] = 25.;
        x.extra.insert(SERIES.into(), bp);
        let t = run("kpdr_discount", &x, 0, 29, &Params::default()).unwrap();
        assert_eq!(t.len(), 1);
        assert_eq!((t[0].legs[0].i, t[0].exit_i), (6, 15)); // acted on the bar after the event
        assert!(run("kpdr_x", &x, 0, 29, &Params::default()).is_none());
    }
}
