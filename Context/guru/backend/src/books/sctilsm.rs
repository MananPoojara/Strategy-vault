//! Dennis L. Tilley, "Moving Averages With Resistance And Support", Technical Analysis of Stocks & Commodities V.16:9 (September
//! 1998) pp.428-433 (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V16/
//! C09/070AVGS.pdf). Book id `sctilsm`: SMARS. Lane K16 (titles loop, 2 Nov 2026), to-code row 248, decisions 64, 73;
//! independent check authors/titles-1030/K16w/k16_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::lc3::{hi, lo};
use super::msws::sma;
use super::sctraub::week_groups;
use super::tselhs::Book;

pub const REVIEWED: &[&str] = &["sctilsm_smars"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] Figure 4 rules on weekly bars (calendar weeks of the daily bars; P = the week's last close, weekly highs/lows = the week's extremes), x = 10 weeks and F = 1.08 (the article's figures; the optimum lookback range is 10-14 weeks; F = 1 is the plain SMA system). A crossover = P on the other side of the SMA than the week before. While the system is a buy and P crosses below the SMA, Rx = the highest weekly high from the week of the current support level's low to now; sell when P < Sx, or at the crossover when SMA/Sx >= F. While it is a sell and P crosses above the SMA, Sx = the lowest weekly low from the week of the current resistance level's high to now; buy when P > Rx, or at the crossover when Rx/SMA >= F. 'The time corresponding to Sx/Rx' is read as the week the level's low/high was made. Start: cash, no resistance known, so the first upward crossover buys. Long or cash (the article switches between a fund and a money market fund, decision 64); fills at the close of the trading day after the signal week ('buy and sell accordingly at the following Monday's closing net asset value'). Article: Montgomery Emerging Markets Fund (MNEMX) 1993-1997, results as charts. Mutual funds are not held; runs on indices, ETFs and stocks, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "sctilsm_smars", name: "Tilley SMARS: 10-week SMA crossovers confirmed by resistance / support within 8%, weekly, long or cash", group: "S&C: Moving Averages With Resistance And Support (Tilley, V.16:9)", session: 0, dir: "Long", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "sctilsm", rules: &[
            "F|\"Resistance (Rx) If system is currently a buy and P < SMAx (at time of crossing), then: Rx = Maximum (weekly highs between the current time back to the time corresponding to Sx) Support (Sx) If system is currently a sell and P > SMAx (at time of crossing), then: Sx = Minimum (weekly lows between the current time back to the time corresponding to Rx)\" (V.16:9, PDF p.3, Figure 4)",
            "E|\"Buy signal P > Rx or At time of crossover: P > SMAx and Rx/SMAx >= F\" (PDF p.3)",
            "X|\"Sell signal P < Sx or At time of crossover: P < SMAx and SMAx/Sx >= F\" (PDF p.3); \"buy and sell accordingly at the following Monday's closing net asset value\" (PDF p.3)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    reviewed(id).then_some(("sctilsm_smars", "Tilley SMARS (S&C)", "10 weeks, F 1.08"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("x", "SMA lookback, weeks (10; optimum 10-14)", 10., 2., 40., 1.), p("f", "Confirmation factor F (1.08; 1 = plain SMA)", 1.08, 1., 1.3, 0.01)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let n = p.get("x").copied().unwrap_or(10.).round().max(1.) as usize;
    let f = p.get("f").copied().unwrap_or(1.08);
    let g = week_groups(x);
    let (wh, wl): (Vec<f64>, Vec<f64>) = g.iter().map(|&(s, e)| (hi(&x.h, s, e), lo(&x.l, s, e))).unzip();
    let wc: Vec<f64> = g.iter().map(|&(_, e)| x.c[e]).collect();
    let m = sma(&wc, n);
    let (mut buy, mut rx, mut sx) = (false, f64::NAN, f64::NAN);
    let (mut rk, mut sk) = (0usize, 0usize); // weeks where the current Rx high / Sx low was made
    let start = g.iter().position(|&(_, e)| e >= from).unwrap_or(g.len());
    let mut sk_set = false;
    let mut pos = Book::new();
    for k in start.max(1)..g.len() {
        let e = g[k].1;
        if e >= to { break; }
        if !(m[k].is_finite() && m[k - 1].is_finite()) { continue; }
        let (pc, up, dn) = (wc[k], wc[k] > m[k] && wc[k - 1] <= m[k - 1], wc[k] < m[k] && wc[k - 1] >= m[k - 1]);
        let mut act = 0;
        if buy {
            if dn {
                let a = if sk_set { sk } else { start };
                let (j, v) = (a..=k).fold((a, f64::NEG_INFINITY), |b, j| if wh[j] > b.1 { (j, wh[j]) } else { b });
                rx = v; rk = j;
            }
            if pc < sx || (dn && m[k] / sx >= f) { act = -1; }
        } else {
            if up {
                let a = if rx.is_finite() { rk } else { start };
                let (j, v) = (a..=k).fold((a, f64::INFINITY), |b, j| if wl[j] < b.1 { (j, wl[j]) } else { b });
                sx = v; sk = j; sk_set = true;
            }
            if pc > rx || (up && (!rx.is_finite() || rx / m[k] >= f)) { act = 1; }
        }
        if act != 0 {
            buy = act > 0;
            let i = e + 1;
            if buy { pos.enter(1.0, i, x.c[i], "next close"); } else { pos.exit(i, x.c[i], "SMARS sell"); }
        }
    }
    Some(pos.done(to, x.c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn smars_weekly() {
        let r = super::super::sctraub::tests::bars(3000);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("sctilsm_smars", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() > 3 && t.iter().all(|z| z.dir > 0.0 && z.legs[0].px == x.c[z.legs[0].i]));
        let mut p = Params::new();
        p.insert("f".into(), 1.0);
        assert!(run("sctilsm_smars", &x, 0, r.len() - 1, &p).unwrap().len() >= t.len());
    }
}

// Instrument admission: S&C Tilley SMARS 2026-11-02 (titles lane K16).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "sctilsm" && reviewed(def.id),
    label: "Dennis Tilley tested SMARS on an emerging-markets mutual fund (MNEMX, 1993-1997). Runs on indices, ETFs and stocks, US and India, are a declared application.",
    basis: "Technical Analysis of Stocks & Commodities V.16:9 pp.428-433 (Moving Averages With Resistance And Support), Figure 4 rules; independent check authors/titles-1030/K16w/k16_ref.py",
};
