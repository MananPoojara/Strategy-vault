//! George Panagakis, "Trading T-Bonds And Currencies", Technical Analysis of Stocks & Commodities V.13:11 (November 1995)
//! pp.478-485 (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V13/C11/
//! TBONDS.pdf). Book id `scpanag`. Lane K19 (titles loop, 2 Nov 2026), to-code row 292; independent check
//! authors/titles-1030/K19w/k19_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::lc3::{hi, lo};
use super::scwhite::complete_weeks;
use super::tselhs::Book;

pub const REVIEWED: &[&str] = &["scpanag_2close"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] Weekly bars = calendar weeks of the daily bars (the data's last week counts only when it ends on a Friday). Buy signal: this week's close above last week's high and last week's close above the high of the week before; sell signal mirrored with lows. Trades at the signal week's close (the article buys 'from the price of 104-20', week 3's close). The position is closed at the close of the eighth week after the starting week, except: a weekly close beyond the extreme of the two signal weeks (long: below their lowest low) gets flat at that close; an opposite signal reverses; a same-side signal in the eighth week keeps the position, with that week as the new starting week and signal weeks. The article states the exceptions for longs; its yen example applies the eight-week and repeat-signal rules to shorts, and the stop is mirrored for shorts. A same-side signal before the eighth week is ignored. Futures roll-overs do not apply. Article: T-bond and yen futures, fiscal years 1980-1994 (not held). Runs on indices, ETFs and stocks, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "scpanag_2close", name: "Panagakis weekly two-close system: follow two weekly closes beyond the previous week's range, out after eight weeks", group: "S&C: Trading T-Bonds And Currencies (Panagakis, V.13:11)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "scpanag", rules: &[
            "E|\"When two weekly closes occur in a row that are higher than the highest price of the previous week, we get a buy signal ... Whenever two weekly closes occur in a row that are lower than the lowest price of the previous week, we get a sell signal\" (p.480)",
            "X|\"this position will normally be closed after eight weeks ... first, if we are long and there is a weekly close below the lowest price of the two higher weekly closes that gave us our entry signal, we would get flat; second, if we are long and during the eight weeks we get a sell signal ... we would reverse ... third, if we are long and weeks 7 and 8 both close at higher prices from the highest price from the previous week, (a buy signal), we would remain long, with the eighth week as our new starting point\" (p.481)",
            "F|\"we now have a buy signal, so we initiate a long position in the bond market from the price of 104-20\" (week 3's close, p.481)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    reviewed(id).then_some(("scpanag_2close", "Panagakis weekly two-close system (S&C)", "8-week hold"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("hold", "Normal holding, weeks (8; the article: 5 or 10 also work)", 8., 1., 20., 1.)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let hold = p.get("hold").copied().unwrap_or(8.).round().max(1.) as usize;
    let w = complete_weeks(x);
    let (wh, wl): (Vec<f64>, Vec<f64>) = w.iter().map(|&(s, e)| (hi(&x.h, s, e), lo(&x.l, s, e))).unzip();
    let wc: Vec<f64> = w.iter().map(|&(_, e)| x.c[e]).collect();
    let sig = |k: usize| -> f64 {
        if k < 2 { 0.0 } else if wc[k] > wh[k - 1] && wc[k - 1] > wh[k - 2] { 1.0 } else if wc[k] < wl[k - 1] && wc[k - 1] < wl[k - 2] { -1.0 } else { 0.0 }
    };
    let stop_of = |d: f64, k: usize| if d > 0.0 { wl[k - 1].min(wl[k]) } else { wh[k - 1].max(wh[k]) };
    let mut pos = Book::new();
    let (mut start, mut stop) = (0usize, f64::NAN);
    for k in 0..w.len() {
        let e = w[k].1;
        if e < from { continue; }
        if e > to { break; }
        let (d, s) = (pos.side(), sig(k));
        if d != 0.0 {
            if s == -d {
                if e < to { pos.enter(s, e, wc[k], "close"); start = k; stop = stop_of(s, k); } else { pos.exit(e, wc[k], "reversed"); }
                continue;
            }
            if d * (wc[k] - stop) < 0.0 { pos.exit(e, wc[k], "close beyond the signal weeks"); continue; }
            if k == start + hold {
                if s == d { start = k; stop = stop_of(d, k); } else { pos.exit(e, wc[k], "eight weeks"); }
            }
            continue;
        }
        if s != 0.0 && e < to { pos.enter(s, e, wc[k], "close"); start = k; stop = stop_of(s, k); }
    }
    Some(pos.done(to, x.c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn eight_week_exits_at_week_closes() {
        let r = super::super::sctraub::tests::bars(3000);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("scpanag_2close", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() > 3);
        for z in t.iter().filter(|z| !z.open) { assert_eq!(z.exit_px, x.c[z.exit_i]); assert_eq!(super::super::sctraub::weekday(&x.r[z.exit_i].date), 4); }
    }
}

// Instrument admission: S&C Panagakis weekly two-close system 2026-11-02 (titles lane K19).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "scpanag" && reviewed(def.id),
    label: "Panagakis tested the weekly system on T-bond and yen futures (1980-1994). Runs on indices, ETFs and stocks, US and India, are a declared application.",
    basis: "Technical Analysis of Stocks & Commodities V.13:11 pp.478-485 (Trading T-Bonds And Currencies); independent check authors/titles-1030/K19w/k19_ref.py",
};
