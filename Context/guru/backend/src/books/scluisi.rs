//! Joe Luisi, "On Range Trading", Technical Analysis of Stocks & Commodities V.13:12 (December 1995) pp.513-515 (research share
//! Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V13/C12/ONRANGE.pdf). Book id `scluisi`.
//! Lane K40 (4 Nov 2026, decision #203); independent check authors/titles-1030/K40w/k40_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::lc3::Pos;

pub const REVIEWED: &[&str] = &["scluisi_nr"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] Day trade. When yesterday's range (high - low) is 3.00 S&P points or less, today buy on a stop at yesterday's high or sell short on a stop at yesterday's low (a trade beyond the level; filled at the level, or at the open when the market opens beyond it); the protective stop is half of yesterday's range from the entry level ('If the high is 500.00 and the low is 497.00, the stop-loss order is placed at half the range, or 1.50 points from the entry (high or low)'); otherwise the position is liquidated at the close. One trade a day: if both levels trade, the one nearer the open is taken first; when the stop lies inside the entry day's range it is taken as hit (the order inside the day is unknown, so the worse case is used). Decision #203: the 3.00-point cap is the article's S&P 500 amount, read literally on SPX only (the allowed list); option mode = 1 uses 0.6% of yesterday's close (3.00 / 500, the article's 1995 index level) for other price levels, labelled, and is not published elsewhere. The article's 'second method' (several days below the average range, a three-day high/low trigger) is not specified enough and is left out. Book: S&P 500 futures May-June 1995 chart examples, no test statistics. SPX results start in 2014, where the index's opens are real.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "scluisi_nr", name: "Luisi narrow range: after a day of 3.00 S&P points or less, buy / sell the break of its high / low, stop at half the range, out at the close", group: "S&C: On Range Trading (Luisi, V.13:12)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "scluisi", rules: &[
            "F|\"The first method is a simple range, obtained by subtracting the high from the low of the day ... in the S&P 500, 3.00 points is a good average.\" (p.513-514)",
            "E|\"One method with which to look at the S&Ps is whenever the market's range is 3.00 points or lower, look to buy when the high of the previous day is broken or sell when the low is broken.\" (p.514)",
            "X|\"place the stop-loss order at half the high/low range. If the high is 500.00 and the low is 497.00, the stop-loss order is placed at half the range, or 1.50 points from the entry (high or low).\"; \"Then liquidate the position at the close of trading for the day\" (p.514)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { reviewed(id).then_some(("scluisi_nr", "Luisi narrow-range breakout (S&C)", "3.00 S&P points")) }

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("mode", "Range cap: 0 = points (article, SPX); 1 = % of yesterday's close", 0., 0., 1., 1.),
        p("cap", "Range cap, index points (article 3.00)", 3., 0.25, 50., 0.25),
        p("pct", "Range cap, % of yesterday's close (0.6 = 3.00 / 500)", 0.6, 0.05, 5., 0.05)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let (pct_mode, cap, pct) = (g("mode").round() == 1., g("cap"), g("pct") / 100.);
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let mut pos = Pos::new();
    for t in from.max(1)..=to {
        let (ph, pl) = (h[t - 1], l[t - 1]);
        let rng = ph - pl;
        let lim = if pct_mode { pct * c[t - 1] } else { cap };
        if !(rng > 0. && rng <= lim) { continue; }
        let (up, dn) = (h[t] > ph, l[t] < pl);
        let side = if up && dn { if ph - o[t] <= o[t] - pl { 1. } else { -1. } } else if up { 1. } else if dn { -1. } else { continue };
        if side > 0. {
            let stop = ph - rng / 2.;
            pos.enter(1., t, o[t].max(ph), "stop");
            if l[t] <= stop { pos.exit(t, stop, "stop: half the range"); } else { pos.exit(t, c[t], "close"); }
        } else {
            let stop = pl + rng / 2.;
            pos.enter(-1., t, o[t].min(pl), "stop");
            if h[t] >= stop { pos.exit(t, stop, "stop: half the range"); } else { pos.exit(t, c[t], "close"); }
        }
    }
    Some(pos.done(to, c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn narrow_range_day_trades() {
        let r = super::super::sctraub::tests::bars(3000);
        let x = Ctx::new(&r, &r, &[]);
        let mut pr = Params::new(); pr.insert("cap".into(), 2.6);
        let t = run("scluisi_nr", &x, 0, r.len() - 1, &pr).unwrap();
        assert!(t.len() > 5 && t.iter().all(|z| z.exit_i == z.legs[0].i && !z.open));
        for z in &t { let i = z.legs[0].i; assert!(x.h[i - 1] - x.l[i - 1] <= 2.6); }
    }
}

// Instrument admission: S&C Luisi narrow-range breakout 2026-11-04 (titles lane K40). The gate's allowed list keeps it on SPX.
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "scluisi" && reviewed(def.id),
    label: "Joe Luisi showed the narrow-range breakout on S&P 500 futures (1995). The 3.00-point cap is read literally on SPX only (decision #203); other instruments are not published.",
    basis: "Technical Analysis of Stocks & Commodities V.13:12 pp.513-515 (On Range Trading); independent check authors/titles-1030/K40w/k40_ref.py",
};
