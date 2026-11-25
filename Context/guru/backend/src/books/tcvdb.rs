//! Tushar S. Chande, "Identifying Powerful Breakouts Early" (Technical Analysis of Stocks & Commodities V.13:10, Oct 1995, p.418-424; research share
//! Trading Knowledge I/Stocks & Commodities DVD 11.26/VOLUMES/V13/C10/IDENTIF.pdf, text layer, read in full). Book id `tcvdb`. A close beyond the 1% bands
//! around VIDYA (equivalent to a 12-day EMA driven by a 9-day CMO) is a breakout when the 14-period dynamic momentum index (length 5 to 30) is above 70
//! (long) or below 30 (short); "a reversal will be defined when prices close beyond the 1% band in the opposite direction"; "a breakout may gather
//! momentum several days after actually closing below or above the 1% band. In this case, the trade will be entered after the DMI criterion is satisfied."
//! Lane K1 (23 Nov 2026, queue A1458). Independent check authors/conv-1123/K1/tcvdb_ref.py.
use super::b1ind::dmi;
use super::chntt::vidya;
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "tcvdb";
pub const REVIEWED: &[&str] = &["tcvdb_breakout"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared adaptation, R4 #340] The article shows futures (yen, coffee, T-notes, heating oil, sugar, pound, bonds) with chart examples and a few stock charts (KO, IBM), no trade list or table, so no figure is compared; the author says the approach 'can be used with all financial markets, including futures, stocks and mutual funds'. The vault runs it on indices, ETFs and stocks, UNVERIFIED. VIDYA and the dynamic momentum index are the vault chntt::vidya and b1ind::dmi built-ins; the choices inside them are listed below. [Declared readings, each an unstated choice] (1) entry is a state test: whenever flat, a close beyond a 1% band with the DMI qualified (above 70 long, below 30 short) is a breakout, including several days after the first band close; (2) the entry is filled at that signal day's close (the article gives no fill rule); (3) a reversal exit is filled at the close that is beyond the opposite 1% band; (4) on that same close the opposite trade is entered only if its DMI test holds, otherwise the card stays flat until the next breakout; (5) band and DMI tests are strict: close strictly above VIDYA x 1.01 or below VIDYA x 0.99, DMI strictly above 70 or below 30; (6) VIDYA (constant 2/13, the 12-day EMA equivalent, scaled by the absolute 9-day CMO) is seeded with the 9th close; (7) DMI volatility index = population standard deviation of the last 5 closes divided by its 10-day simple average (Chande's definition, not printed in this article); (8) DMI length = 14 divided by that index, truncated to a whole number and clamped to 5-30; (9) DMI = 100 x sum of up close-to-close changes / (up + down changes) over that length (RSI-style on closes); (10) no trade before both VIDYA and the DMI exist (warm-up); (11) the 'suspect breakout' tightening and the multiple-close or low-beyond-band filters the author names are not applied. Honest total: 11 card-specific choices. Engine-wide defaults / declared adaptations (labelled, not counted, R4 #404): one position at a time (a reversal closes one trade as the next opens), an open trade marked at the window's last close, the data window; indices, ETFs and stocks instead of the article's futures and two stock charts (declared adaptation #340).";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "tcvdb_breakout", name: "VIDYA 1% band breakout confirmed by the dynamic momentum index (above 70 / below 30), reversed on a close beyond the opposite band",
        group: "Chande powerful breakouts", session: 0, dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "tcvdb",
        rules: &[
            "F|\"VIDYA is set up to be equivalent to a 12-day exponential moving average (EMA) driven by a nine-day CMO\"; \"1% bands around VIDYA\"; \"14-period DMI ... allow the length to vary from as few as five days to as many as 30\" (p.418)",
            "E|\"Whenever the market closes above the upper band or below the lower band, we can assume that prices have broken out in that direction\"; \"DMI rise above 70 or fall below 30 to confirm\"; \"the trade will be entered after the DMI criterion is satisfied\" (p.418-419)",
            "X|\"a reversal will be defined when prices close beyond the 1% band in the opposite direction. As a result, an upside breakout will be reversed when prices close below the 1% band\" (p.419)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id { "tcvdb_breakout" => Some(("tcvdb_breakout", "Chande VIDYA band + DMI breakout", "VIDYA 12/CMO 9, 1% bands, DMI 70/30")), _ => None }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    match id {
        "tcvdb_breakout" => Some(vec![p("ema", "VIDYA equivalent to an EMA of N days (article 12)", 12., 3., 60., 1.), p("cmo", "CMO length driving VIDYA (article 9)", 9., 3., 30., 1.),
            p("band", "Band width around VIDYA, % (article 1)", 1., 0.25, 5., 0.25), p("hi", "DMI above (article 70)", 70., 50., 95., 5.), p("lo", "DMI below (article 30)", 30., 5., 50., 5.),
            p("shorts", "Short side: 1 = yes (article), 0 = long only", 1., 0., 1., 1.)]),
        _ => None,
    }
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let s = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| s.iter().find(|d| d.key == k).unwrap().def);
    let (c, bw, hi, lo, shorts) = (&x.c, g("band") / 100.0, g("hi"), g("lo"), g("shorts") > 0.5);
    let v = vidya(c, g("cmo").round() as usize, 2.0 / (g("ema") + 1.0));
    let d = dmi(c);
    let (mut out, mut cur) = (vec![], None::<(f64, usize, f64)>);
    for i in from..=to {
        if !v[i].is_finite() || !d[i].is_finite() { continue; }
        let (up, dn) = (c[i] > v[i] * (1.0 + bw), c[i] < v[i] * (1.0 - bw));
        if let Some((dir, e, ep)) = cur {
            if (dir > 0. && dn) || (dir < 0. && up) {
                out.push(Trade { dir, legs: vec![Leg { i: e, px: ep, w: 1. }], exit_i: i, exit_px: c[i], reason: "close beyond the opposite 1% band".into(), entry_kind: "close", signal_i: i, open: false, exits: vec![] });
                cur = None;
            } else { continue; }
        }
        if up && d[i] > hi { cur = Some((1., i, c[i])); } else if shorts && dn && d[i] < lo { cur = Some((-1., i, c[i])); }
    }
    if let Some((dir, e, ep)) = cur {
        out.push(Trade { dir, legs: vec![Leg { i: e, px: ep, w: 1. }], exit_i: to, exit_px: c[to], reason: "open".into(), entry_kind: "close", signal_i: to, open: true, exits: vec![] });
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn rejects_other_ids_and_runs() {
        let r: Vec<Row> = (0..400).map(|i| { let v = 100.0 + (i as f64 * 0.07).sin() * 20.0 + (i as f64 * 0.31).sin() * 3.0; Row { ticker: "T".into(), date: format!("2001-{:03}", i), open: v, high: v + 1., low: v - 1., close: v, volume: None } }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let p = Params::default();
        assert!(run("tcvdb_x", &x, 0, 399, &p).is_none());
        for z in run("tcvdb_breakout", &x, 0, 399, &p).unwrap() { assert!(z.exit_i >= z.legs[0].i); }
    }
}

// Instrument admission: Chande powerful breakouts 2026-11-23 (lane K1). Admission only for the module REVIEWED lists.
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "tcvdb" && crate::books::tcvdb::reviewed(def.id),
    label: "Chande shows the rule on futures (yen, coffee, T-notes, pound) with chart examples and says it applies to stocks and funds; runs on indices, ETFs and US/India stocks are a declared application (R4 #340), not the author's test.",
    basis: "Tushar S. Chande, Identifying Powerful Breakouts Early, Technical Analysis of Stocks & Commodities V.13:10 (1995), p.418-424",
};
