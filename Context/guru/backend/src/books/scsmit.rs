//! Damir Smitlener, "Where Is The Weakest Link?", Technical Analysis of Stocks & Commodities V.22:12 (Dec 2004) pp.42-45
//! (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V22/C12/265SMIT.pdf).
//! Book id `scsmit`. Lane C7 (second pass, 17 Nov 2026); independent check authors/convert-1117/C7/ref_sm/c7sm_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::rbtie::target_trades;

pub const REVIEWED: &[&str] = &["scsmit_r1g1", "scsmit_r2g1", "scsmit_r2g2"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }
fn defaults(id: &str) -> (f64, f64) { match id { "scsmit_r1g1" => (1., 1.), "scsmit_r2g1" => (2., 1.), _ => (2., 2.) } }

const NOTE: &str = "[Adapted: 3 unstated details, R4 #297 / #339] Smitlener's streak system trades only the closes, with no indicator: 'it does not depend on any indicators whatsoever, relying instead on nothing more than the previous two closes'. Long after N consecutive red closes, out after M consecutive green closes (Figure 3: every red close, out at the next green close; Figure 4A: two red closes, out at the next green close; Figure 4B: two red closes, out at the second consecutive green close). Long only, no stop ('let's eliminate stop-losses'), one unit. The article's martingale-style additions on each further red close (Figures 5-7, bet sizes not stated) are not run. [Unstated details] (1) a red close is a close below the previous close and a green close a close above it (the article does not define the colours; a close equal to the previous one is neither and breaks a streak); (2) fills are at the close of the signal day (the article gives no fill rule); (3) the M green closes counted for the exit are the closes after the entry day. [Book results] The figures print wins, losses, average percentages and Dow points gained for the DJIA since 1930 (Dow points, one unit); the pdf text layer scrambles which numbers belong to which figure and the vault's DJI does not reach back to 1930, so no like-for-like figure is quoted or matched. UNVERIFIED.";

fn mk(id: &'static str, name: &'static str, rules: &'static [&'static str]) -> StrategyDef {
    StrategyDef { id, name, group: "S&C: Where Is The Weakest Link? (Smitlener, V.22:12)", session: 0, dir: "Long", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "scsmit", rules, notes: NOTE, claim: None }
}

pub fn catalogue() -> Vec<StrategyDef> {
    vec![mk("scsmit_r1g1", "Smitlener streak: buy a red close, sell the next green close (Figure 3)", &[
            "E|\"a long-only system that attempts to play the market bias by doing nothing but buying a red close and waiting for a green close\" (V.22:12 p.44)",
            "X|\"waiting for a green close\"; Figure 3: \"going long every red close, exiting next green close\" (p.44)"]),
         mk("scsmit_r2g1", "Smitlener streak: buy after two consecutive red closes, sell the next green close (Figure 4A)", &[
            "E|\"let's adjust the system to wait for a second consecutive red close and only then go long\" (p.44)",
            "X|Figure 4A: \"going long after two red closes, exiting next green close\" (p.44)"]),
         mk("scsmit_r2g2", "Smitlener streak: buy after two consecutive red closes, sell after two consecutive green closes (Figure 4B)", &[
            "E|\"wait for a second consecutive red close and only then go long\" (p.44)",
            "X|\"waiting for consecutive green closes before exiting\"; \"you wait for the second consecutive red close to go long and the second consecutive green close to exit\"; Figure 4B (p.44-45)"])]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    reviewed(id).then_some(("scsmit_streak", "Smitlener streak system (S&C)", "red closes in, green closes out"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let (r, g) = defaults(id);
    reviewed(id).then(|| vec![p("reds", "Consecutive red closes to buy (article 1 or 2)", r, 1., 5., 1.), p("greens", "Consecutive green closes to sell (article 1 or 2)", g, 1., 5., 1.)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from > to || to >= x.c.len() { return None; }
    let (dr, dg) = defaults(id);
    let n = |k: &str, d: f64| p.get(k).copied().unwrap_or(d).round().max(1.) as usize;
    let (r, g) = (n("reds", dr), n("greens", dg));
    let c = &x.c;
    let (mut tgt, mut long, mut reds, mut greens) = (vec![0i8; to + 1], false, 0usize, 0usize);
    for i in 1..=to {
        if !(c[i].is_finite() && c[i - 1].is_finite()) { reds = 0; greens = 0; tgt[i] = long as i8; continue; }
        let (red, green) = (c[i] < c[i - 1], c[i] > c[i - 1]);
        if long {
            greens = if green { greens + 1 } else { 0 };
            if greens >= g { long = false; greens = 0; reds = 0; }
        } else {
            reds = if red { reds + 1 } else { 0 };
            if reds >= r { long = true; greens = 0; reds = 0; }
        }
        tgt[i] = long as i8;
    }
    Some(target_trades(x, &tgt, from, to, false, "streak"))
}

pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "scsmit" && reviewed(def.id),
    label: "Damir Smitlener ran the streak system on the DJIA (Dow points, since 1930); runs on actual indices, ETFs and stocks, US and India, are a declared application, not the author's test, with the unstated details read as labelled on the card.",
    basis: "Technical Analysis of Stocks & Commodities V.22:12 pp.42-45 (Where Is The Weakest Link?); independent check authors/convert-1117/C7/ref_sm/c7sm_ref.py",
};

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn streak_has_catalogue_and_params() {
        assert!(REVIEWED.iter().all(|i| family_of(i).is_some() && params(i).is_some()));
        let r = super::super::sctraub::tests::bars(300);
        let x = Ctx::new(&r, &r, &[]);
        for i in REVIEWED { assert!(run(i, &x, 0, r.len() - 1, &Params::new()).is_some()); }
    }
}
