//! Murray A. Ruggiero, Jr., "Cybernetic Trading Strategies" (Wiley, 1997), Ch.2 Counterseasonal, corn seasonal short and counterseasonal long (p.57).
//! Book id `rgcsea` (the card was NeedsData "needs cash/futures corn prices" in books/rgcts.rs; Norgate &ZC corn continuous futures are held, R4 #414).
//! Lane K3 (25 Nov 2026). Independent check authors/conv-1123/k3w/rgcsea_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::rbtie::target_trades;

pub const BOOK: &str = "rgcsea";
pub const REVIEWED: &[&str] = &["rgc_corn_counterseasonal"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Adapted: 4 unstated details, R4 #400/#414] The book's corn seasonal: sell corn on the first trading day after June 20 and exit November 1; if the short is not profitable after July 21, go long on the first trading day after July 21 and hold until the first trading day of the next year (p.57; the book prints $28,487.50 with 65% winners for the short and $38,537.50 with 58% for the short plus the counterseasonal long, window not stated on the card). Runs on the Norgate corn continuous contract (&ZC, back-adjusted daily bars). Unstated, one per item: (1) every order is a calendar order filled at the open of its dated bar (no look-ahead: the date alone decides); (2) 'not profitable after July 21' = the close of the last trading day on or before July 21 is at or above the short's entry price, both on the back-adjusted bars (a roll gap is not profit; a close equal to the entry counts as not profitable); (3) the reversal covers the short and buys at the open of the first trading day after July 21 (no flat bar between); (4) 'exit November 1' = the open of the first trading day on or after November 1; the long is closed at the open of the first trading day of the next calendar year. One unit, no costs, no stops.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "rgc_corn_counterseasonal", name: "Corn seasonal short and counterseasonal long", group: "Ch. 2 · Counterseasonal", session: 0, dir: "Long/Short",
        status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
        rules: &["E|Sell corn on the first trading day after June 20 (p.57)", "X|Exit November 1 (p.57)", "E|If the short is not profitable after July 21, go long the first trading day after July 21 (p.57)", "X|Hold the long until the first trading day of the next year (p.57)"],
        notes: NOTE, claim: None }]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { reviewed(id).then_some(("rgcsea", "Ruggiero corn seasonal", "Counterseasonal")) }
pub fn params(_id: &str) -> Option<Vec<ParamDef>> { None }

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, _p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from > to || to >= x.c.len() { return None; }
    let d: Vec<&str> = (0..=to).map(|i| x.r[i].date.as_str()).collect();
    let year = |i: usize| d[i][..4].parse::<i32>().unwrap_or(0);
    let first_gt = |lo: usize, key: &str| (lo..=to).find(|&i| &d[i][..10] > key);
    let first_ge = |lo: usize, key: &str| (lo..=to).find(|&i| &d[i][..10] >= key);
    let mut pos = vec![0i8; to + 1]; // position held from the OPEN of bar i
    let mut y0 = 0usize;
    while y0 <= to {
        let y = year(y0);
        let mut y1 = y0; while y1 < to && year(y1 + 1) == y { y1 += 1; }
        let (k20, k721, k11) = (format!("{y}-06-20"), format!("{y}-07-21"), format!("{y}-11-01"));
        if let Some(s) = first_gt(y0, &k20).filter(|&s| s <= y1) {
            let entry = x.o[s];
            let jul = (y0..=y1).rev().find(|&i| &d[i][..10] <= k721.as_str());
            let losing = jul.is_some_and(|j| j >= s && x.c[j] >= entry);
            let rev = first_gt(s, &k721).filter(|&r| r <= y1);
            if losing && rev.is_some() {
                let r = rev.unwrap();
                for i in s..r { pos[i] = -1; }
                let end = if y1 < to { y1 + 1 } else { to + 1 }; // first bar of the next year (open) or the series end
                for i in r..end { pos[i] = 1; }
            } else {
                let e = first_ge(s, &k11).filter(|&e| e <= y1).unwrap_or(y1 + 1);
                for i in s..e.min(to + 1) { pos[i] = -1; }
            }
        }
        y0 = y1 + 1;
    }
    // target_trades(next_open = true) executes tgt[i] at the open of bar i+1: shift the by-bar positions left by one
    let tgt: Vec<i8> = (0..=to).map(|i| if i < to { pos[i + 1] } else { pos[i] }).collect();
    Some(target_trades(x, &tgt, from, to, true, "calendar order (Ch.2 corn seasonal)"))
}
