//! Gregory L. Morris, "Breadth of Fresh Air" (Technical Analysis of Stocks & Commodities V.23:12, Dec 2005, p.36-41; research share Trading Knowledge I/
//! Stocks & Commodities DVD 11.26/VOLUMES/V23/C12/249MORR.pdf, text layer, read in full). Book id `gmsi`. "If you invested in the NASDAQ composite index when the
//! McClellan summation index rose above the selected level and then you got out when it dropped back below, you would experience the results shown on the row for
//! that level" (Figure 1, levels +800 to -800; the text singles out zero and +300). McClellan oscillator = 19-day EMA minus 39-day EMA of net advances; the ratio form
//! (A-D)/(A+D) is used and the summation is the Miekka-calibrated, zero-level-neutral version. Lane K1 (23 Nov 2026, queue A4024). Independent check
//! authors/conv-1123/K1/gmsi_ref.py.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "gmsi";
pub const REVIEWED: &[&str] = &["gmsi_us", "gmsi_in"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared adaptation, R4 #340] The study runs the NASDAQ composite on NASDAQ advance/decline data, 1979-2004 (Figure 1: at zero, 54.17% winning years, 44.01% of time invested, 22.38% a year while invested; at +300, 56% winning years, 21.04% of time invested, maximum drawdown -4.1% against -77.93% for buy and hold). The vault holds no NASDAQ breadth, so the rule runs on the breadth it has: `gmsi_in` = NSE advances/declines (1999-) on the NSE indices NIFTY 50 and NIFTY BANK, `gmsi_us` = NYSE advances/declines (US sources) on the US indices SPX, NDX and DJI; indices only, never single stocks or ETFs. Nothing is compared with Figure 1 (different market, different breadth, different period; the table's 'winning percentage' counts winning years, not trades). UNVERIFIED. Default level 0 (the 'commonly used' level the text names first; +300 is the other level it recommends, p.level 300). [Declared readings, each an unstated detail] Card-specific unstated choices (9), each independent: (1) the ratio is (A-D)/(A+D) x 1000, the scale on which levels such as +300 and +800 make sense; (2) the 19- and 39-day EMAs of the ratio are each seeded with the first ratio; (3) the calibrated index = 19 x EMA39 - 9 x EMA19 (the running sum of the oscillator with its start-dependent constant dropped; the calibration formula itself is not printed); (4) a rise above the level = the index above the level having been at or below it on the previous breadth day; (5) the first close at or below the level sells (the source says below); (6) breadth is published after the close, so both the buy and the sell fill at the NEXT bar's close (R4 #374, as kpdr); (7) the filters the text only suggests (65-day average, direction, a 21-day average of the index) are not applied; (8) breadth days without a bar are skipped and leave the position unchanged; (9) the default level is 0 (the commonly used level the text names first; +300 is the other it recommends). Honest total: 9 card-specific choices. Engine-wide defaults / declared adaptations (labelled, not counted, R4 #404): NYSE advances/declines on SPX, NDX and DJI instead of NASDAQ breadth on the NASDAQ composite (declared adaptation #340); indices only (#364); no costs.";

pub fn catalogue() -> Vec<StrategyDef> {
    let mk = |id: &'static str, name: &'static str, series: &'static [&'static str]| StrategyDef { id, name, group: "Morris McClellan summation index", session: 0, dir: "Long", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: series, book: "gmsi",
        rules: &[
            "F|\"the McClellan oscillator, which is calculated by subtracting the 39-day exponential moving average from the 19-day exponential moving average of net advances (A - D)\"; \"McClellan summation index: sum (McClellan oscillator)\"; ratio (A - D)/(A + D) and the zero-level-neutral calibration (p.36)",
            "E|\"If you invested in the NASDAQ composite index when the McClellan summation index rose above the selected level ...\" (Figure 1 text, levels +800 to -800; zero and +300 singled out)",
            "X|\"... and then you got out when it dropped back below\""],
        notes: NOTE, claim: None };
    vec![mk("gmsi_in", "McClellan summation index above its level (NSE breadth, NSE indices): in above the level, out when it drops back below", &["NSE_ADV", "NSE_DECL"]),
         mk("gmsi_us", "McClellan summation index above its level (NYSE breadth, US indices): in above the level, out when it drops back below", &["NYSE_ADV", "NYSE_DECL"])]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id { "gmsi_in" | "gmsi_us" => Some(("gmsi", "Morris McClellan summation index level", "ratio x1000, 19/39 EMA, level 0")), _ => None }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    match id {
        "gmsi_in" | "gmsi_us" => Some(vec![p("level", "Summation index level (article names 0 and +300; table +800 to -800)", 0., -800., 800., 100.),
            p("fast", "Fast EMA, days (article 19)", 19., 5., 50., 1.), p("slow", "Slow EMA, days (article 39)", 39., 10., 100., 1.)]),
        _ => None,
    }
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let s = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| s.iter().find(|d| d.key == k).unwrap().def);
    let (a, d) = if id == "gmsi_in" { ("NSE_ADV", "NSE_DECL") } else { ("NYSE_ADV", "NYSE_DECL") };
    let (Some(adv), Some(dec)) = (x.series(a), x.series(d)) else { return Some(vec![]) };
    let (lvl, nf, ns) = (g("level"), g("fast").round(), g("slow").round());
    let (af, as_) = (2.0 / (nf + 1.0), 2.0 / (ns + 1.0));
    let (k_slow, k_fast) = ((1.0 - as_) / as_, (1.0 - af) / af);
    let c = &x.c;
    let (mut ef, mut es, mut started) = (0.0, 0.0, false);
    let (mut prev, mut cur) = (f64::NAN, None::<(usize, f64)>);
    // Breadth is published after the close (R4 #374, as kpdr): a signal read on bar i is filled at the close of the NEXT bar (i+1).
    let mut pend: Option<bool> = None; // Some(true) = enter, Some(false) = exit
    let mut out = vec![];
    for i in 0..=to {
        if i >= from {
            match (pend.take(), cur) {
                (Some(true), None) => cur = Some((i, c[i])),
                (Some(false), Some((e, ep))) => {
                    out.push(Trade { dir: 1., legs: vec![Leg { i: e, px: ep, w: 1. }], exit_i: i, exit_px: c[i], reason: "summation index fell back to the level".into(), entry_kind: "close", signal_i: i, open: false, exits: vec![] });
                    cur = None;
                }
                _ => {}
            }
        }
        let (ai, di) = (adv[i], dec[i]);
        if !(ai.is_finite() && di.is_finite() && ai + di > 0.0) { continue; }
        let r = 1000.0 * (ai - di) / (ai + di);
        if !started { ef = r; es = r; started = true; } else { ef += af * (r - ef); es += as_ * (r - es); }
        let si = (k_slow * es) - (k_fast * ef); // 19 x EMA39 - 9 x EMA19 for 19/39 (a general running-sum identity)
        let was = prev; prev = si;
        if i < from { continue; }
        if cur.is_none() && was.is_finite() && was <= lvl && si > lvl { pend = Some(true); }
        else if cur.is_some() && si <= lvl { pend = Some(false); }
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
    fn runs_with_breadth_and_rejects_other_ids() {
        let r: Vec<Row> = (0..300).map(|i| Row { ticker: "NIFTY 50".into(), date: format!("2010-{:03}", i), open: 100., high: 100., low: 100., close: 100. + i as f64, volume: None }).collect();
        let mut x = Ctx::new(&r, &r, &[]);
        let adv: Vec<f64> = (0..300).map(|i| 500.0 + 300.0 * (i as f64 * 0.05).sin()).collect();
        x.extra.insert("NSE_ADV".into(), adv);
        x.extra.insert("NSE_DECL".into(), vec![500.0; 300]);
        assert!(run("gmsi_x", &x, 0, 299, &Params::default()).is_none());
        let t = run("gmsi_in", &x, 0, 299, &Params::default()).unwrap();
        assert!(!t.is_empty());
        for z in &t { assert!(z.exit_i >= z.legs[0].i); }
    }
}

// Instrument admission: indices only (never single stocks or ETFs), declared adaptation.
pub fn index_only() -> &'static str { "Morris McClellan summation index level rule: the study trades the NASDAQ composite, an index; run on actual indices only (NSE breadth on NIFTY 50 / NIFTY BANK, NYSE breadth on SPX / NDX / DJI) as a declared adaptation (R4 #340); UNVERIFIED" }
