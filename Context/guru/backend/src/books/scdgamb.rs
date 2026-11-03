//! Walter T. Downs, "The Forex Gambit", Technical Analysis of Stocks & Commodities, June 2010, pp.22-27 (research share Trading
//! Knowledge I/Stocks & Commodities 2010 -2011 (traders.com)/SC2010-06.pdf, OCR of PDF p.7-9). Book id `scdgamb`. Lane K19
//! (titles loop, 2 Nov 2026), to-code row 299; independent check authors/titles-1030/K19w/k19_ref.py.
use crate::connors::{sma, Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::tselhs::Book;

pub const REVIEWED: &[&str] = &["scdgamb_sma30"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] 'Two lower lows' = the bar's low below the previous low and that low below the one before (mirrored: two higher highs); the bar midpoint = (high + low) / 2; SMA(30) of closes. Entry at the signal bar's close; the stop is exactly the entry bar's low (high for shorts) from the next bar, filled at the stop or at the open beyond it. 'Has been in the market two days' = from the third bar after the entry bar the stop is the entry price. A bar after the entry bar whose range exceeds each of the previous three ranges and whose close is in its top 10% (close >= high - 0.1 x range) arms the long exit: out at the first later close below the previous close (mirrored for shorts with the bottom 10%). Out at the close of the 16th bar after the entry bar. One trade at a time; a new pattern counts from the bar after an exit. Not applied: the lot-size and $200-reserve betting algorithm and the 'variations to consider' (discretionary). Article: EUR/USD and GBP/USD examples; three walk-through (not computer) runs of 90 trades, 75.9%, 62% and 63% winners. Different rules from the e-book of the same name (wtdfg_bb_pullback). Runs on indices, ETFs and stocks, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "scdgamb_sma30", name: "Downs Forex Gambit (S&C): two lower lows closing above the bar midpoint and the 30-day SMA (mirror for shorts), stop at the entry bar, 16-day limit", group: "S&C: The Forex Gambit (Downs, Jun 2010)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "scdgamb", rules: &[
            "E|\"Buy two lower lows and a closing price that is greater than the midpoint of the bar and is above the SMA. Place a stop-loss order just below the entry bar. Sell two higher highs and a closing price that is less than the midpoint of the bar and is below the SMA. Place a stop-loss order just above the entry bar.\" (SMA period 30, PDF p.7)",
            "X|\"When a bar occurs whose range is greater than the previous three ranges and whose close is in the top 10% of the bar, exit a long position on the first closing price that is lower than the previous closing price\" (mirrored for shorts); \"Close any position that has been in the market 16 days. Move stop-loss orders to break even on any trade that has been in the market two days.\" (PDF p.7)",
            "F|\"Take only one trade at a time. Another trade is not opened until this trade is closed.\" (PDF p.7)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    reviewed(id).then_some(("scdgamb_sma30", "Downs Forex Gambit, S&C article", "SMA 30, 16 days"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("n", "SMA period (30)", 30., 5., 100., 1.), p("days", "Time exit, days in the market (16)", 16., 2., 60., 1.), p("be", "Break-even after days in the market (2)", 2., 0., 20., 1.)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d).round().max(0.) as usize;
    let (n, days, be) = (g("n", 30.).max(1), g("days", 16.).max(1), g("be", 2.));
    let m = sma(&x.c, n);
    let rg = |i: usize| x.h[i] - x.l[i];
    let mut pos = Book::new();
    let (mut init, mut armed) = (0.0, false);
    for i in from.max(3)..=to {
        let d = pos.side();
        if d != 0.0 {
            let (ei, ep) = (pos.entry_bar().unwrap(), pos.entry_px());
            let lv = if i > ei + be { ep } else { init };
            if d > 0.0 && x.l[i] <= lv { pos.exit(i, x.o[i].min(lv), "stop"); continue; }
            if d < 0.0 && x.h[i] >= lv { pos.exit(i, x.o[i].max(lv), "stop"); continue; }
            if i - ei >= days { pos.exit(i, x.c[i], "16 days"); continue; }
            if armed && d * (x.c[i] - x.c[i - 1]) < 0.0 { pos.exit(i, x.c[i], "close against the trade after a wide bar"); continue; }
            let wide = rg(i) > rg(i - 1) && rg(i) > rg(i - 2) && rg(i) > rg(i - 3);
            if wide && (if d > 0.0 { x.c[i] >= x.h[i] - 0.1 * rg(i) } else { x.c[i] <= x.l[i] + 0.1 * rg(i) }) { armed = true; }
            continue;
        }
        if i >= to || !m[i].is_finite() { continue; }
        let mid = (x.h[i] + x.l[i]) / 2.0;
        if x.l[i] < x.l[i - 1] && x.l[i - 1] < x.l[i - 2] && x.c[i] > mid && x.c[i] > m[i] { pos.enter(1.0, i, x.c[i], "close"); init = x.l[i]; armed = false; }
        else if x.h[i] > x.h[i - 1] && x.h[i - 1] > x.h[i - 2] && x.c[i] < mid && x.c[i] < m[i] { pos.enter(-1.0, i, x.c[i], "close"); init = x.h[i]; armed = false; }
    }
    Some(pos.done(to, x.c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn one_trade_at_a_time_and_time_limit() {
        let r = super::super::sctraub::tests::bars(2500);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("scdgamb_sma30", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() > 3);
        for z in &t { assert!(z.exit_i - z.legs[0].i <= 16); }
        for w in t.windows(2) { assert!(w[1].legs[0].i > w[0].exit_i); }
    }
}

// Instrument admission: S&C Downs Forex Gambit 2026-11-02 (titles lane K19).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "scdgamb" && reviewed(def.id),
    label: "Downs walked the system through currency pairs by hand (EUR/USD, GBP/USD; no computer test). Daily runs on indices, ETFs and stocks, US and India, are a declared application.",
    basis: "Technical Analysis of Stocks & Commodities, June 2010, pp.22-27 (The Forex Gambit; OCR of PDF p.7-9); independent check authors/titles-1030/K19w/k19_ref.py",
};
