//! Nelson Freeburg, "Mechanical Timing Systems: The Key to Consistent Profits & Sharper Trading"
//! (TAG 21, Nov 1999, INO workshop, 29 pp. scan). Source and checks: library/nfmt/.
//! Same systems as the 1996/2000 workshops (Channel 25, Pathfinder, R-Squared, intermarket T-Bond,
//! CQIII, Value Line swing) - catalogued there. New here: the 1999 Quadram rules with a T-Bond filter.
use crate::connors::{ParamDef, Params, Status, StrategyDef, Trade, Ctx};

pub const BOOK: &str = "nfmt";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef {
        id: "nfmt_quadram_tbond", name: "Quadram S&P system with T-Bond filter (1999)", group: "Quadram S&P Futures Trading System", session: 0,
        dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: super::nfhp::NYSE, book: BOOK,
        rules: &["E|Long countertrend: \"T-Bond > T-Bond[5] AND CHAD Crosses Above 4-DMA Minus 0.8 Standard Deviation\"; trend-following: \"T-Bond > T-Bond[5] AND CHAD Crosses Above 25-DMA Plus 0.8 Standard Deviation\" (Fig. 43, p.24)",
                 "E|Short countertrend: \"T-Bond <= T-Bond[5] AND CHAD Crosses Below 4-DMA Plus 0.8 Standard Deviation\"; trend-following: \"... Crosses Below 25-DMA Minus 0.8 Standard Deviation\" (Fig. 43)",
                 "X|\"$500 Initial Stop; $500 Trailing Stop\" (Fig. 43)"],
        notes: "[Adapted: 2 readings, R4 #293 (4), #297; lane PF, 14 Nov 2026] Book (Fig. 41, p.23): S&P 500 futures 1982-1999, 2 contracts @ $250/pt: 784 trades, 43%, avg $395.41, PF 1.94. Runs only on Norgate continuous S&P 500 futures (&SP, back-adjusted) with Norgate T-bonds (&ZB) as the filter market; published 1982-04-21..1995-07-31 (R4 #330: every entry is at the next open), so the book window is not reproduced (unverified). Stops: $500 at the book's own $250 per point = 2.0 points (as nfhp_quadram_*). Readings: (1) the two printed entry modes are one system (a countertrend or a trend-following cross on the same bar, filtered by the bond; opposite signals on one bar = none); (2) 'T-Bond > T-Bond[5]' = the bond's close on the signal date (exact date; none = no signal) against its close 5 bond bars earlier. CHAD, bands, entries at the next open, stops and reversals exactly as nfhp_quadram_* (2000 rules).",
        claim: crate::connors::claim("784", "43%", "$395.41 / trade (2 lots)", "1 bar winners, 0 losers", "S&P 500 futures 1982-1999"),
    }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == "nfmt_quadram_tbond").then_some(("nf_quadram", "Freeburg Quadram S&P system", "T-Bond filtered (1999)"))
}
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    (id == "nfmt_quadram_tbond").then(|| vec![p("chad_n", "CHAD days", 5., 2., 20., 1.), p("k", "Band width (standard deviations)", 0.8, 0.2, 3., 0.1),
        p("stop_usd", "Initial stop ($)", 500., 0., 5000., 100.), p("trail_usd", "Trailing stop ($)", 500., 0., 5000., 100.),
        p("pt_usd", "Dollars per index point (book: $250)", 250., 50., 500., 50.), p("eds", "Initial stop armed on the entry day", 1., 0., 1., 1.),
        p("blag", "T-Bond lag (bars)", 5., 1., 20., 1.)])
}

/// Lane PF (R4 #293 (4)): the 1999 Quadram = both nfhp entry modes, each only with the T-bond filter (Fig. 43).
pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if from > to || to >= x.r.len() { return None; }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap().def);
    use super::nfhp::{quadram_run, quadram_signals, Quad, NYSE};
    let (Some(a), Some(d)) = (x.series(NYSE[0]), x.series(NYSE[1])) else { return Some(vec![]) };
    let Some(rows) = crate::futures::rows("&ZB") else { return Some(vec![]) };
    let pt = g("pt_usd").max(1e-9);
    let q = |len: usize, tf: bool| Quad { chad_n: g("chad_n").round().max(1.) as usize, len, k: g("k"), stop: g("stop_usd") / pt, trail: g("trail_usd") / pt, eds: g("eds") > 0.5, tf };
    let (ct, tf) = (quadram_signals(a, d, &q(4, false)), quadram_signals(a, d, &q(25, true)));
    let lag = g("blag").round().max(1.) as usize;
    let s = crate::second::align("&ZB", x.r, rows);
    let (bc, bl) = (s.exact(crate::second::Field::C), s.own(|r| (0..r.len()).map(|k| if k >= lag { r[k - lag].close } else { f64::NAN }).collect()));
    let sig: Vec<i8> = (0..x.r.len()).map(|i| {
        let (l, sh) = (ct[i] == 1 || tf[i] == 1, ct[i] == -1 || tf[i] == -1);
        if !(bc[i].is_finite() && bl[i].is_finite()) || l == sh { 0 } else if l && bc[i] > bl[i] { 1 } else if sh && bc[i] <= bl[i] { -1 } else { 0 }
    }).collect();
    Some(quadram_run(x, from, to, &sig, &q(4, false)))
}
