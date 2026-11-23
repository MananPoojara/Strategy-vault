//! Thomas Stridsman, "Short-term strategy, long-term perspective", Active Trader, April 2000, pp. 36-38 (rules box p. 50 of the
//! issue; research share Trading Knowledge II/@NewsLetters-Mags/Active Trader Magazine/Misc Numbers/Binder - Thomas Stridsman -
//! System Desgin And Testing Collection.pdf, binder pp. 36-38). Book id `stda`. Lane K2 (23 Nov 2026, R4 #340). Daily bars.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "stda";
const ID: &str = "stda_obv_ma6";

const NOTE: &str = "A|Adapted (R4 #340, lane K2 23 Nov 2026), UNVERIFIED: the article tests S&P 500 futures 1985-1999 (177 trades, profit factor 1.56, 58.2% winners, Table 2; no costs); this runs on NIFTY 50 / NIFTYBANK with the lane-IN2 FUTURES daily volume (connors::FUT_VOL: minute volumes summed per day, days with a blank volume absent), so OBV starts at the first futures day (2018-12) and the 200-day average of OBV needs 200 days with a volume. Readings, 5, all labelled: (1) the 6-day and 200-day averages are simple, the 6-day on closes; 'crosses above' = above today and not above yesterday (high and close both), the short side the mirror (low and close cross below; OBV below its 200-day average); (2) OBV = running sum of volume signed by the close change, an unchanged close adds nothing; (3) 'always lock in a profit of 0.6 percent' = once a trade's high (short: low) has reached +0.6% on an earlier day, a stop at entry +0.6% (short: -0.6%) joins the 1.1% stop; the 2.8% target is a limit; a gap through a level fills at the open; when one bar reaches both the stop and the target the stop is taken (no minute path); (4) the day count of 'exit after eight days' is the close of the 8th bar after the entry bar; entry is at the close of the signal day; (5) 'never enter on the same day as an exit' means no entry on a day a trade exits. The printed S&P results are for another market and period and are not compared. No costs.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: ID, name: "OBV-filtered 6-day MA breakout (Stridsman, Active Trader Apr 2000)", group: "Short-term strategy, long-term perspective (Stridsman, 2000)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: true, needs_vix: false, needs_series: &[], book: BOOK,
        rules: &["E|Long at the close: OBV above its 200-day moving average; today's high crosses above its six-day moving average; today's close crosses above its six-day moving average; never enter on the same day as an exit (p. 50)",
            "X|Stop out if the loss reaches 1.1 percent; always lock in a profit of 0.6 percent; exit immediately if the profit reaches 2.8 percent; if still in the trade exit after eight days; the short side reverses the rules (p. 50)", NOTE],
        notes: "Stridsman, Active Trader April 2000, pp. 36-38. The article's Table 2: S&P 500 futures 1985-1999, 177 trades, profit factor 1.56, 58.19% winners.", claim: None }]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { (id == ID).then_some(("stda", "OBV-filtered MA breakout", "")) }
pub fn params(id: &str) -> Option<Vec<ParamDef>> { (id == ID).then(Vec::new) }

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, _p: &Params) -> Option<Vec<Trade>> {
    if id != ID || to >= x.c.len() || from >= to { return None; }
    let v = x.fut_vol();
    let n = x.c.len();
    let (mut obv, mut nv) = (vec![f64::NAN; n], vec![0usize; n]); // nv = days with a volume so far
    let mut acc = 0.0;
    let mut cnt = 0;
    for i in 0..n {
        if i > 0 && v[i].is_finite() { acc += (x.c[i] - x.c[i - 1]).signum() * if x.c[i] == x.c[i - 1] { 0.0 } else { v[i] }; cnt += 1; }
        obv[i] = acc; nv[i] = cnt;
    }
    // 200-day simple average of the OBV over the 200 most recent days that had a volume
    let mut vol_days: Vec<usize> = vec![];
    let mut ma_obv = vec![f64::NAN; n];
    for i in 0..n {
        if i > 0 && v[i].is_finite() { vol_days.push(i); }
        if vol_days.len() >= 200 && v[i].is_finite() { ma_obv[i] = vol_days[vol_days.len() - 200..].iter().map(|&k| obv[k]).sum::<f64>() / 200.0; }
    }
    let ma6: Vec<f64> = (0..n).map(|i| if i >= 5 { x.c[i - 5..=i].iter().sum::<f64>() / 6.0 } else { f64::NAN }).collect();
    let mut out: Vec<Trade> = vec![];
    let mut pos: Option<(f64, usize, f64, bool)> = None; // dir, entry bar, entry px, lock armed
    let mut last_exit = usize::MAX;
    let mk = |dir: f64, ei: usize, ep: f64, xi: usize, xp: f64, why: &str, open: bool| Trade { dir, legs: vec![Leg { i: ei, px: ep, w: 1.0 }], exit_i: xi, exit_px: xp, reason: why.into(), entry_kind: "close", signal_i: ei, open, exits: Vec::new() };
    for t in from.max(1)..=to {
        if let Some((dir, ei, ep, armed)) = pos {
            let stop = if armed { ep * (1.0 + dir * 0.006) } else { ep * (1.0 - dir * 0.011) };
            let stop = if armed { if dir > 0.0 { stop.max(ep * 0.989) } else { stop.min(ep * 1.011) } } else { stop };
            let tgt = ep * (1.0 + dir * 0.028);
            let hit = crate::books::atm00::levels(x, t, dir, Some(stop), Some(tgt));
            if let Some((px, why)) = hit { out.push(mk(dir, ei, ep, t, px, why, false)); pos = None; last_exit = t; }
            else if t >= ei + 8 { out.push(mk(dir, ei, ep, t, x.c[t], "exit after eight days", false)); pos = None; last_exit = t; }
            else {
                let reached = if dir > 0.0 { x.h[t] >= ep * 1.006 } else { x.l[t] <= ep * 0.994 };
                pos = Some((dir, ei, ep, armed || reached));
            }
        }
        if pos.is_none() && last_exit != t && ma_obv[t].is_finite() && t >= 6 && ma6[t - 1].is_finite() {
            let hi_up = x.h[t] > ma6[t] && x.h[t - 1] <= ma6[t - 1];
            let cl_up = x.c[t] > ma6[t] && x.c[t - 1] <= ma6[t - 1];
            let lo_dn = x.l[t] < ma6[t] && x.l[t - 1] >= ma6[t - 1];
            let cl_dn = x.c[t] < ma6[t] && x.c[t - 1] >= ma6[t - 1];
            if obv[t] > ma_obv[t] && hi_up && cl_up { pos = Some((1.0, t, x.c[t], false)); }
            else if obv[t] < ma_obv[t] && lo_dn && cl_dn { pos = Some((-1.0, t, x.c[t], false)); }
        }
    }
    if let Some((dir, ei, ep, _)) = pos { out.push(mk(dir, ei, ep, to, x.c[to], "open", true)); }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sane_runs() { assert!(params(ID).unwrap().is_empty()); }
    /// hand-run dump for the pandas reference: STDA_CSV=<idx_data.csv> STDA_INTRA=<data>/intraday STDA_OUT=<file> cargo test --release stda_dump -- --ignored
    #[test]
    #[ignore]
    fn stda_dump() {
        use std::io::Write;
        let data = crate::load_csv(&std::env::var("STDA_CSV").unwrap()).unwrap();
        let dir = std::env::var("STDA_INTRA").unwrap();
        let mut f = std::io::BufWriter::new(std::fs::File::create(std::env::var("STDA_OUT").unwrap()).unwrap());
        writeln!(f, "id,ticker,dir,entry_date,entry_px,exit_date,exit_px,open").unwrap();
        for t in ["NIFTY 50", "NIFTYBANK"] {
            let rows = &data[t];
            let mut x = Ctx::new(rows, &[], &[]);
            let v = crate::intraday::fut_daily_volume(&format!("{dir}/{}.FUT1.m1.csv", t.replace(' ', "_")));
            x.extra.insert(crate::connors::FUT_VOL.to_string(), rows.iter().map(|r| v.get(r.date.as_str()).copied().unwrap_or(f64::NAN)).collect());
            for z in run(ID, &x, 0, rows.len() - 1, &Params::new()).unwrap() {
                writeln!(f, "{ID},{t},{},{},{},{},{},{}", z.dir, rows[z.legs[0].i].date, z.legs[0].px, rows[z.exit_i].date, z.exit_px, z.open).unwrap();
            }
        }
    }
}

pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "stda",
    label: "Daily OBV-filtered moving-average breakout with percent exits; its volume on the Indian indices is the NIFTY / BANKNIFTY futures volume (lane IN2): run and independently checked on NIFTY 50 and NIFTYBANK only.",
    basis: "Stridsman, Active Trader April 2000, pp. 36-38 (Binder - Thomas Stridsman - System Desgin And Testing Collection.pdf); independent check authors/conv-1123/k2w/stda_ref.py",
};
