//! Lane R2C (13 Nov 2026; authors/titles-1030/R2C.tsv; R4 #297 "up to two unstated details"; reader row authors/o2/O2.tsv):
//! William Gallacher, "Winner Take All" (revised edition; research share 'Trading Books/William Gallacher - Winner Take All
//! (Revised Ed.)/William Gallacher - Winner Take All (Revised Ed.).pdf', 275 PDF pages, page images: read with authors/ocr2;
//! printed page = PDF page - 19). Book id `gwta`.
//! Coded: PLODDER, the ten-day high/low stop-and-reverse system of ch.4 'Technical Trading Systems' (printed p.78-87).
//! The rest of the book is commentary on the industry, fundamentals, money management and option writing (contents p.v-vii).
//! Independent reference: authors/titles-1030/R2Cw/r2c_ref.py (written from the pages, not from this file).
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::lc3::{hi, high_first, lo};
use super::tselhs::{stop_fill, Book};

pub const REVIEWED: &[&str] = &["gwta_plodder"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Adapted: 2 unstated details, R4 #297; unverified] Always in the market. While short, a buy stop sits one tick above the highest high of the previous ten days; while long, a sell stop one tick below the lowest low of the previous ten days (p.81, 84); when it is hit the position reverses at the stop, or at the open when the day opens beyond it ('Had the opening price been higher than the reversal price, the execution price would have to be taken as the opening price', p.84). One tick = the instrument's minimum price step (0.01 US, 0.05 NSE; R4 #57). Unstated in the book: (1) the first position - the book 'assumes an initial short position (we have to begin somewhere)' (p.81); vault reading: start flat and take the first stop that is hit, so a run does not depend on its start date's arbitrary side. (2) a day that reaches both stops - the book's daily table never shows one; vault reading: only the stop of the side being reversed is live at the open, the new opposite stop is not taken on the same day (it is checked from the next day); when flat and one day reaches both, the extreme nearer the open fills first (TradeStation's daily path). The book tests ten commodity futures over two and a half years around 1980-1982 in percent of margin (Table 4-3, p.88: pork bellies +277% a year ... average 385%); none of those series is run here, so there is no like-for-like comparison. Applied to indices, ETFs and stocks, US and India, as a declared application. Fills at the open or on stops need real opens, so index runs start where real opens start.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "gwta_plodder", name: "Gallacher PLODDER: stop and reverse one tick beyond the previous ten days' high / low", group: "PLODDER (ch.4, p.78-88)",
        session: 0, dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "gwta",
        rules: &["E|\"Assuming an initial short position (we have to begin somewhere), that position will be maintained provided on any subsequent trading day the highest price of the previous ten days is not exceeded. When this condition does occur, the short position will be covered, and a long position will be taken.\" (p.81)",
            "X|\"The long position will then be held as long as the low price on any day is not below the lowest price of the previous ten days.\" (p.81); \"Each day a stop-sell order is placed one tick (the minimum price change between trades) below the low price of the previous ten days.\" (p.84)",
            "F|\"it is fair to take 39.57 as the execution price as well as the reversal price, since the opening price is below the reversal price. Had the opening price been higher than the reversal price, the execution price would have to be taken as the opening price.\" (p.84)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == "gwta_plodder").then_some(("gwta_plodder", "Gallacher PLODDER ten-day reversal", "10"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("n", "Days in the high / low (book 10, 'purely arbitrary')", 10., 2., 100., 1.), p("tick", "Stop offset in ticks (book 1; 0 = none)", 1., 0., 10., 1.)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let n = p.get("n").copied().unwrap_or(10.).round().max(1.) as usize;
    let off = p.get("tick").copied().unwrap_or(1.).round().max(0.) * x.market_tick();
    let mut pos = Book::new();
    for i in from + 1..=to {
        let t = i - 1;
        if t + 1 < n { continue; }
        let (bs, ss) = (hi(&x.h, t + 1 - n, t) + off, lo(&x.l, t + 1 - n, t) - off);
        let (up, dn) = (stop_fill(x, i, 1., bs), stop_fill(x, i, -1., ss));
        let side = pos.side();
        let pick = match (up, dn) {
            (Some(u), Some(d)) if side == 0. => if high_first(x.o[i], x.h[i], x.l[i]) { Some((1., u)) } else { Some((-1., d)) },
            (Some(u), _) if side <= 0. => Some((1., u)),
            (_, Some(d)) if side >= 0. => Some((-1., d)),
            _ => None,
        };
        if let Some((d, px)) = pick { pos.enter(d, i, px, "stop"); }
    }
    Some(pos.done(to, x.c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn plodder_reverses_at_the_ten_day_stops() {
        let r = super::super::sctraub::tests::bars(2500);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("gwta_plodder", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() > 5 && t.iter().any(|z| z.dir > 0.) && t.iter().any(|z| z.dir < 0.));
        for z in &t {
            let (i, px) = (z.legs[0].i, z.legs[0].px);
            if z.dir > 0. { assert!(px >= hi(&x.h, i - 10, i - 1) + 0.01 - 1e-9); } else { assert!(px <= lo(&x.l, i - 10, i - 1) - 0.01 + 1e-9); }
        }
        for w in t.windows(2) { assert_eq!(w[1].legs[0].i, w[0].exit_i); assert_eq!(w[1].dir, -w[0].dir); assert_eq!(w[1].legs[0].px, w[0].exit_px); }
    }
}

// Instrument admission: Gallacher PLODDER daily application 2026-11-13 (lane R2C, R4 #297).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "gwta" && reviewed(def.id),
    label: "William Gallacher tests PLODDER on ten commodity futures (pork bellies, hogs, cattle, gold, copper, sugar, cocoa, coffee, soybeans, wheat); daily bars of indices, ETFs and stocks, US and India, are a declared application, with two unstated details read as labelled on the card (R4 #297).",
    basis: "Winner Take All (revised ed.) ch.4 printed p.78-88 (PDF 97-107, ocr2); R4 #297, #57; independent check authors/titles-1030/R2Cw/r2c_ref.py",
};
