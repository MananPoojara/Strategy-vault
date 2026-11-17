//! Damir Smitlener, "Where Is The Weakest Link?", Technical Analysis of Stocks & Commodities V.22:12 (December 2004) pp.42-45
//! (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V22/C12/265SMIT.pdf).
//! Book id `scsmit`: long after two consecutive red closes, out after two consecutive green closes, no stop-loss. Lane C3
//! (second pass of title-only 'No rule' S&C articles, 17 Nov 2026); independent check authors/convert-1117/C3/w/smit_ref.py.
//! Admitted by this module's own ADMISSION (C3_BOOKS).
use crate::connors::{Ctx, ParamDef, Params, StrategyDef, Trade, Status};
use super::tselhs::Book;

pub const REVIEWED: &[&str] = &["scsmit_streak"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Adapted: 2 unstated details, R4 #297/#339; unverified] The article's closing system (PDF p.3): 'wait for the second consecutive red close to go long and the second consecutive green close to exit', no stop-loss. Readings: (1) a red close is a close below the previous close and a green close one above it (the article's streak tables are close-to-close Dow data; a close equal to the previous one is neither and breaks both streaks); (2) a streak is counted on consecutive bars ending at the signal bar (from the series' first bar, so a window start sees the earlier streak), whether or not a position is open, so the second green close can also be the bar that follows the entry bar; after an exit a new entry needs two new red closes. Fills at the signal bar's close (the article trades 'a red close' and 'the next green close' on closes). One position at a time: the article's Figures 5-7 add entries on each further red close (up to four, three sizing rules) and are NOT applied (labelled: the card is the single-position Figure 4B system). Option 'greens = 1' is Figure 4A (exit on the next green close). Book figures are the DJIA from 1930 (Figure 4B 1,206 wins, 712 losses); the vault's DJIA starts 1970, so the result cannot be compared.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "scsmit_streak", name: "Smitlener red/green streak system: buy the close of the 2nd consecutive red close, sell the close of the 2nd consecutive green close, no stop", group: "S&C: where is the weakest link (Smitlener, V.22:12)", session: 0, dir: "Long", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "scsmit", rules: &[
            "F|\"Let's start with an ultra-simple approach: a long-only system that attempts to play the market bias by doing nothing but buying a red close and waiting for a green close.\" (V.22:12, PDF p.2)",
            "E|\"If the best predictor of a red tomorrow is a red today, let's adjust the system to wait for a second consecutive red close and only then go long (Figure 4A).\" (PDF p.2)",
            "X|\"As before, you wait for the second consecutive red close to go long and the second consecutive green close to exit.\" (PDF p.3; Figure 4B: 'going long after two red closes, exiting after consecutive green closes')",
            "F|\"let's eliminate stop-losses and limit our potential losses instead by using much smaller position sizes\" - no stop in the system (PDF p.2); the additional-entry variants of Figures 5-7 are not applied"],
        notes: NOTE, claim: crate::connors::claim("1,918", "63%", "", "", "DJIA 1930-2004, Figure 4B (1,206 wins, 712 losses)") }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { reviewed(id).then_some(("scsmit_streak", "Smitlener streak system (S&C)", "2 red / 2 green")) }
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    reviewed(id).then(|| vec![
        ParamDef { key: "reds", label: "Consecutive red closes to buy (article 2)", def: 2., min: 1., max: 10., step: 1. },
        ParamDef { key: "greens", label: "Consecutive green closes to sell (article 2; 1 = Figure 4A)", def: 2., min: 1., max: 10., step: 1. }])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from > to || to >= x.c.len() { return None; }
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d).round().max(1.) as usize;
    let (nr, ng) = (g("reds", 2.), g("greens", 2.));
    let (mut red, mut green) = (0usize, 0usize);
    let mut pos = Book::new();
    for t in 1..=to {
        let (c, pc) = (x.c[t], x.c[t - 1]);
        if !(c.is_finite() && pc.is_finite()) { red = 0; green = 0; continue; }
        if c < pc { red += 1; green = 0; } else if c > pc { green += 1; red = 0; } else { red = 0; green = 0; }
        if t < from { continue; } // streaks are counted on the full history; trades start at the window's first bar
        if pos.side() > 0. { if green >= ng { pos.exit(t, c, "consecutive green closes"); } }
        else if red >= nr && t < to { pos.enter(1., t, c, "close"); }
    }
    Some(pos.done(to, x.c[to]))
}

pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| C3_BOOKS.contains(&def.book) && c3_reviewed(def.book, def.id),
    label: "Stocks & Commodities articles read in lane C3 (second pass of title-only 'No rule' decisions, 17 Nov 2026); the authors tested them on Dow Jones Industrial Average or futures data; runs on daily bars of indices, ETFs and stocks, US and India, are a declared application, not the authors' tests (any unstated detail is a labelled reading on the card, R4 #297/#339).",
    basis: "Lane C3 (R4 #339): the card cites its article pages; independent check authors/convert-1117/C3/w/smit_ref.py",
};
/// Books admitted by the entry above.
pub const C3_BOOKS: &[&str] = &["scsmit"];
fn c3_reviewed(book: &str, id: &str) -> bool {
    match book { "scsmit" => reviewed(id), _ => false }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn streak_entries_and_exits() {
        let r = super::super::sctraub::tests::bars(3000);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("scsmit_streak", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() > 50 && t.iter().all(|z| z.dir > 0.));
        for z in &t {
            let i = z.legs[0].i;
            assert!(x.c[i] < x.c[i - 1] && x.c[i - 1] < x.c[i - 2], "entry on the 2nd consecutive red close");
            if !z.open { assert!(x.c[z.exit_i] > x.c[z.exit_i - 1] && x.c[z.exit_i - 1] > x.c[z.exit_i - 2], "exit on the 2nd consecutive green close"); }
        }
        for w in t.windows(2) { assert!(w[1].legs[0].i > w[0].exit_i - 1); }
    }
}
