//! Jerry Favors, "The Gann Quarterly Chart", Technical Analysis of Stocks & Commodities V.10:1 (January 1992) pp.19-21 (research
//! share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V10/C01/THEGANN.pdf). Book id
//! `scfavq`. Lane K40 (4 Nov 2026, decision #203); independent check authors/titles-1030/K40w/k40_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::lc3::{high_first, Pos};
use super::scfavw::{ge, le};

pub const REVIEWED: &[&str] = &["scfavq_quarterly"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] Calendar quarters Jan-Mar, Apr-Jun, Jul-Sep, Oct-Dec; the levels are the previous calendar quarter's highest high and lowest low (the data's first quarter, possibly partial, gives no levels). The chart turns up the first day the high trades above the previous quarter's high and stays up until a day trades below the previous quarter's low (and vice versa); a day that could do both turns the chart against its current direction only, or, before the first turn, by the extreme nearer the open. Orders: a buy stop one tick above the previous quarter's high (the article's July 7, 1980 turn 'at 896.34 intraday' over the 896.33 high), a sell stop one tick below its low, filled at that level or at the open beyond it (tick = 0.01 US, 0.05 NSE; option tick = 0 uses the level itself). Default long / flat: a turn down sells (the article times bull and bear markets and never shorts); option side = 0 also sells short at the turn down. Book: DJIA turns 7/7/80 up, 7/2/81 down, 8/23/82 up, 2/1/84 down, 8/3/84 up, 10/15/87 down, 4/12/88 up, 8/17/90 down, 1/18/91 up (pp.19-21); the vault's DJI daily highs and lows give exactly these nine dates (K40 check). DJI and SPX results start where their opens are real (gap fills read the open). Runs on other indices, ETFs and stocks, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "scfavq_quarterly", name: "Favors Gann quarterly chart: buy when price trades above the previous quarter's high, sell below the previous quarter's low", group: "S&C: The Gann Quarterly Chart (Favors, V.10:1)", session: 0, dir: "Long", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "scfavq", rules: &[
            "F|\"Those quarters run the traditional schedule: January to March, April to June, July to September and October to December. As each new quarter begins ... it is necessary to keep track of the intraday high and low of the previous quarter.\" (p.19)",
            "E|\"The Gann quarterly chart ... will signal a turn up from any bear market low when the Dow Jones Industrial Average (DJIA) rallies above the intraday high of the prior quarter.\" (p.19); \"That occurred on July 7, 1980, at 896.34 intraday\" over \"the 896.33 intraday high of June 26, 1980\" (p.20)",
            "X|\"Until the DJIA falls below the intraday low of the previous quarter, the trend remains up. However, the trend turns down when the DJIA falls below the low of the previous quarter.\" (p.19)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { reviewed(id).then_some(("scfavq_quarterly", "Favors Gann quarterly chart (S&C)", "previous quarter's high / low")) }

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("side", "Sides: 1 = long / flat (default); 0 = also short at the turn down", 1., 0., 1., 1.),
        p("tick", "Stop offset beyond the quarter's high / low, in instrument ticks (article 1)", 1., 0., 10., 1.)])
}

fn quarter(d: &str) -> i64 { d[0..4].parse::<i64>().unwrap_or(0) * 4 + (d[5..7].parse::<i64>().unwrap_or(1) - 1) / 3 }

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d);
    let (long_only, tk) = (g("side", 1.).round() != 0., g("tick", 1.) * x.market_tick());
    let mut pos = Pos::new();
    let q0 = quarter(&x.r[0].date); // the data's first quarter may be partial: its high / low are not used
    let (mut dir, mut q, mut prev, mut cur) = (0i8, i64::MIN, None::<(f64, f64)>, (f64::NEG_INFINITY, f64::INFINITY));
    for t in 0..=to {
        let qt = quarter(&x.r[t].date);
        if qt != q { prev = (q == qt - 1 && q != q0).then_some(cur); q = qt; cur = (f64::NEG_INFINITY, f64::INFINITY); }
        if let Some((ph, pl)) = prev {
            let (up, dn) = (ph + tk, pl - tk);
            let (hit_up, hit_dn) = (ge(x.h[t], up) && dir != 1, le(x.l[t], dn) && dir != -1);
            let turn = if hit_up && hit_dn { if high_first(x.o[t], x.h[t], x.l[t]) { 1 } else { -1 } } else if hit_up { 1 } else if hit_dn { -1 } else { 0 };
            if turn != 0 {
                dir = turn;
                if t >= from {
                    if turn == 1 { pos.enter(1., t, x.o[t].max(up), "stop"); }
                    else if long_only { if pos.side() > 0. { pos.exit(t, x.o[t].min(dn), "below the previous quarter's low"); } }
                    else { pos.enter(-1., t, x.o[t].min(dn), "stop"); }
                }
            }
        }
        cur = (cur.0.max(x.h[t]), cur.1.min(x.l[t]));
    }
    Some(pos.done(to, x.c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn quarterly_turns_fill_at_the_previous_quarter_levels() {
        let r = super::super::sctraub::tests::bars(3000);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("scfavq_quarterly", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(!t.is_empty() && t.iter().all(|z| z.dir > 0.));
        for z in &t { let i = z.legs[0].i; assert!(z.legs[0].px >= x.o[i] && z.legs[0].px <= x.h[i] + 1e-6); }
        assert_eq!(quarter("2001-03-31") + 1, quarter("2001-04-02"));
        let mut both = Params::new(); both.insert("side".into(), 0.);
        assert!(run("scfavq_quarterly", &x, 0, r.len() - 1, &both).unwrap().iter().any(|z| z.dir < 0.));
    }
}

// Instrument admission: S&C Favors Gann quarterly chart 2026-11-04 (titles lane K40).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "scfavq" && reviewed(def.id),
    label: "Jerry Favors applied the Gann quarterly chart to the Dow Jones Industrial Average (1980-1991 signals). Runs on other indices, ETFs and stocks, US and India, are a declared application.",
    basis: "Technical Analysis of Stocks & Commodities V.10:1 pp.19-21 (The Gann Quarterly Chart); independent check authors/titles-1030/K40w/k40_ref.py",
};
