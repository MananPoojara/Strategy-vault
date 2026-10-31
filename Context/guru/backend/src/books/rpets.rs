//! Robert P. Rotella, "The Elements of Successful Trading" (New York Institute of Finance, 1992; book id `rpets`;
//! printed pages, PDF = printed + 43). Reading: claude-continuation-20261020/authors/rrotella (admission/ets_ref.py;
//! OCR authors/day-1030/R1w/rotella.ch13.txt). Live: two of Chapter 13's 'patterns to consider' - the 1% opening gap
//! faded from the open to that day's close (p.270) and three consecutive closes bought or sold on the close and closed
//! out at the close of the fifth day (p.274). Not coded: the inside day, key reversal and island rules (entry only, no
//! exit), the variations named without a full rule (p.270-274). Book: futures; indices, ETFs and US/India stocks here
//! are a declared application.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};
/// Admitted ids (authors/rrotella/admission/ets_ref.py (header)).
pub const REVIEWED: &[&str] = &["rpets_gap_fade", "rpets_three_closes"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE_G: &str = "[Declared readings] A gap of '1% above the previous close' is read as an open at least 1% above yesterday's close (open >= close x 1.01), and '1% below' as an open at most 99% of it. The fade sells short (gap up) or buys (gap down) at that open and closes the position at the same day's close; no stop is given, so daily bars fully decide every trade (the open always comes before the close). Mode 1 = the trend version the book names (buy the gap up, sell the gap down, p.270). Gaps are measured from the bar before the test window when the window starts.";
const NOTE_C: &str = "[Declared readings] 'Closes higher three days in a row' = three successive closes each above the close before (three lower = each below). Entry market-on-close on the third such close; 'the close of the fifth day' is read as the close of the fifth trading day after the entry day. One position at a time: signals while a position is open are ignored, and a new signal on the exit day may be taken at that same close. The variations the book only names (trade on the fourth day, wait for a down day) are not coded. A position still open on the last bar is marked at its close.";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id: &'static str, name: &'static str, rules: &'static [&'static str], notes: &'static str| StrategyDef { id, name, group: "Rotella Elements of Successful Trading", session: 0,
        dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "rpets", rules, notes, claim: None };
    vec![
        d("rpets_gap_fade", "1% opening gap faded from the open to the same day's close", &[
            "E|\"1. Sell if the market gaps open 1% above the previous close, and close out the position at the end of the day. 2. Buy if the market gaps open 1% below the previous close, and close out the position at the end of the day.\" (p.270)",
            "X|Out at the same day's close (p.270); no stop",
            "F|\"For those who prefer trending methods, the rules can always be changed to buy if the market gaps higher and sell if the market gaps lower.\" (p.270): mode 1"], NOTE_G),
        d("rpets_three_closes", "Three consecutive higher (lower) closes, on the close, out at the close of the fifth day", &[
            "E|\"Buy market on close if the market closes higher three days in a row, and close out the position at the close of the fifth day. Sell market on close if the market closes lower three days in a row, and close out the position at the close of the fifth day.\" (p.274)",
            "X|Out at the close of the fifth trading day after the entry (p.274); no stop"], NOTE_C),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id {
        "rpets_gap_fade" => Some(("rpets_gap_fade", "Rotella 1% gap fade", "open to close")),
        "rpets_three_closes" => Some(("rpets_three_closes", "Rotella consecutive closes", "3 closes, out day 5")),
        _ => None,
    }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    match id {
        "rpets_gap_fade" => Some(vec![p("pct", "Gap, % of the previous close (book 1)", 1., 0.1, 5., 0.1),
            p("mode", "0 = fade the gap (book rules 1-2), 1 = trend version: buy the gap up, sell the gap down (p.270)", 0., 0., 1., 1.)]),
        "rpets_three_closes" => Some(vec![p("n", "Consecutive closes in a row (book 3)", 3., 2., 10., 1.),
            p("hold", "Out at the close of this trading day after the entry (book 5)", 5., 1., 20., 1.)]),
        _ => None,
    }
}

fn mk(d: f64, ei: usize, ep: f64, kind: &'static str, xi: usize, xp: f64, why: &str, open: bool) -> Trade {
    Trade { dir: d, legs: vec![Leg { i: ei, px: ep, w: 1. }], exit_i: xi, exit_px: xp, reason: why.into(), entry_kind: kind, signal_i: xi, open, exits: vec![] }
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let (o, c) = (&x.o, &x.c);
    let mut out = vec![];
    if id == "rpets_gap_fade" {
        let (pct, trend) = (g("pct").max(0.1), g("mode").round() >= 1.);
        for t in from.max(1)..=to {
            let d = if o[t] >= c[t - 1] * (1. + pct / 100.) { -1. } else if o[t] <= c[t - 1] * (1. - pct / 100.) { 1. } else { continue };
            out.push(mk(if trend { -d } else { d }, t, o[t], "open", t, c[t], "day's close", false));
        }
    } else {
        let (n, hold) = (g("n").round().max(1.) as usize, g("hold").round().max(1.) as usize);
        let mut free = 0; // first bar at whose close a new position may open
        for t in from.max(n)..to {
            if t < free { continue; }
            let d = if (0..n).all(|j| c[t - j] > c[t - j - 1]) { 1. } else if (0..n).all(|j| c[t - j] < c[t - j - 1]) { -1. } else { continue };
            let xi = t + hold;
            out.push(if xi <= to { mk(d, t, c[t], "close", xi, c[xi], "time exit", false) } else { mk(d, t, c[t], "close", to, c[to], "open", true) });
            free = xi;
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn bars(v: &[(f64, f64)]) -> Vec<Row> {
        v.iter().enumerate().map(|(i, &(o, c))| Row { ticker: "T".into(), date: format!("2001-01-{:02}", i + 1), open: o, high: o.max(c), low: o.min(c), close: c, volume: None }).collect()
    }
    #[test]
    fn gap_fade_at_least_one_percent_open_to_close() {
        let r = bars(&[(100., 100.), (101.5, 102.), (101.5, 99.), (98., 98.5), (98., 98.)]);
        // bar 1: 101.5 >= 100 x 1.01 -> short 101.5 -> 102; bar 2: 101.5 vs 102 -> no gap; bar 3: 98 <= 99 x 0.99 -> long
        let x = Ctx::new(&r, &r, &[]);
        let t = run("rpets_gap_fade", &x, 0, r.len() - 1, &Params::new()).unwrap();
        let got: Vec<_> = t.iter().map(|z| (z.dir, z.legs[0].i, z.legs[0].px, z.exit_i, z.exit_px)).collect();
        assert_eq!(got, vec![(-1., 1, 101.5, 1, 102.), (1., 3, 98., 3, 98.5)]);
        let p1: Params = [("mode".to_string(), 1.)].into_iter().collect();
        assert_eq!(run("rpets_gap_fade", &x, 0, r.len() - 1, &p1).unwrap().iter().map(|z| z.dir).collect::<Vec<_>>(), vec![1., -1.]);
    }
    #[test]
    fn three_closes_out_on_the_fifth_day_one_position_at_a_time() {
        let cl = [10., 11., 12., 13., 14., 15., 16., 17., 18., 19., 18., 17.];
        let r = bars(&cl.iter().map(|&c| (c, c)).collect::<Vec<_>>());
        let x = Ctx::new(&r, &r, &[]);
        let t = run("rpets_three_closes", &x, 0, r.len() - 1, &Params::new()).unwrap();
        let got: Vec<_> = t.iter().map(|z| (z.dir, z.legs[0].i, z.exit_i, z.open)).collect();
        // buy at bar 3 (3 higher closes), out at bar 8; bar 8 is again the 3rd higher close -> buy, open at the last bar (11)
        assert_eq!(got, vec![(1., 3, 8, false), (1., 8, 11, true)]);
        assert!(run("rpets_x", &x, 0, r.len() - 1, &Params::new()).is_none());
    }
}

// Instrument admission: Robert P. Rotella 2026-10-30 (day lane A3) block, moved from instrument_policy.rs on 31 Oct 2026 (lane ST).
// Admission only for the module REVIEWED lists (authors/rrotella/admission/ets_ref.py (header)); kept in one block to merge.
// Runs on actual indices, ETFs and stocks, US and India (index_etf_or_stock, nifty50 basket, declared application).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book=="rpets" && crate::books::rpets::reviewed(def.id),
    label: "Robert Rotella states these pattern rules for futures; runs on indices, ETFs and US/India stocks are a declared application, not the author's test.",
    basis: "Robert P. Rotella, The Elements of Successful Trading (1992), p.270 and p.274",
};
