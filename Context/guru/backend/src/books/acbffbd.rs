//! Art Collins, "Beating the Financial Futures Market" (Wiley 2006), Ch.4 p.11-12 (Figure 4.2): "Fading Closes - Dual Entries", daily soybeans.
//! Book id `acbff` (the card was NeedsData "needs soybean futures daily data" in books/acbff.rs; Norgate &ZS soybeans are held, R4 #414).
//! Lane K3 (25 Nov 2026). Independent check authors/conv-1123/k3w/acbffbd_ref.py.
//!
//! Rule (p.12): always fade the direction of the previous day's close. After a down close, if the market opens below the previous low the
//! previous CLOSE is the buy stop; if it opens at or above the low but below the close, the buy stop is the previous HIGH (up close: mirror).
//! "Once in the trade, I used the same rules of the above bond system" (p.11): hold one night; on Day 2 a long's profit objective is the Day-1
//! high and its stop the Day-1 low minus one tick (short: Day-1 low objective, Day-1 high plus one tick); if neither is filled, exit on the Day-2 close.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "acbff";
pub const ID: &str = "acbff_bean_dual";
const TICK: f64 = 0.25; // soybean futures tick in cents per bushel (futures::MARKETS &ZS)

const NOTE: &str = "[Adapted: 5 unstated details, R4 #400/#414] Collins p.11-12 (Figure 4.2): fade the previous close with a dual stop entry, hold one night, Day-2 objective = the Day-1 high (long) or low (short), stop = the opposite Day-1 extreme -/+ one tick, else exit at the Day-2 close. Unstated, labelled: (1) 'down/up close' = the previous close below/above the close before it (an unchanged close gives no order); (2) after a down close an open at or above the previous close gives no order (mirror for an up close): the book names only the two cases below the close; (3) the entry stop fills at its price on the entry day, no protective stop or objective on the entry day; (4) 'Day 1' = the whole entry day (its full high and low); one tick = 0.25 cent per bushel; (5) a Day-2 bar that reaches both the objective and the stop is ordered by the TradeStation default path (open, nearer extreme, other extreme, close) and the share of trades that depends on it is in the gate text (#240). A gap through the objective or the stop at the Day-2 open fills at the open. One unit, no costs, no new orders while a trade is open. The book prints no figures for this system ('very satisfying results').";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: ID, name: "Fading closes · dual entry soybeans (Ch 4)", group: "Ch 4, 6, 23, 32, 45 · Other", session: 0, dir: "Long/Short",
        status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
        rules: &["E|\"Following a down close, if the market opened below the previous low, the close would be the stop entry\" (p.12): buy stop at the previous close",
                 "E|\"If it opened at or higher than the low, but lower than the close, the order would be placed at the high\" (p.12): buy stop at the previous high (an up close: mirror, sell stops)",
                 "X|\"Once in the trade, I used the same rules of the above bond system\" (p.12): hold one night; Day 2: objective the Day-1 high (long) / low (short), stop the opposite Day-1 extreme -/+ one tick",
                 "X|\"If neither side was filled, I'd exit on that close\" (p.11): the Day-2 close"],
        notes: NOTE, claim: None }]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { (id == ID).then_some(("acbff_other", "Collins other", "Fading closes · dual entry soybeans (Ch 4)")) }
pub fn params(_id: &str) -> Option<Vec<ParamDef>> { None }

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, _p: &Params) -> Option<Vec<Trade>> {
    if id != ID || from > to || to >= x.c.len() { return None; }
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let mut out = vec![];
    let mut i = from.max(2);
    while i <= to {
        let s = i - 1;
        let (dir, lv) = if c[s] < c[s - 1] {
            (1.0, if o[i] < l[s] { c[s] } else if o[i] < c[s] { h[s] } else { f64::NAN })
        } else if c[s] > c[s - 1] {
            (-1.0, if o[i] > h[s] { c[s] } else if o[i] > c[s] { l[s] } else { f64::NAN })
        } else { (0.0, f64::NAN) };
        // entry stop reached after the open: a buy stop above the open (long), a sell stop below it (short)
        if !lv.is_finite() || !(if dir > 0.0 { h[i] >= lv } else { l[i] <= lv }) { i += 1; continue; }
        let (d1h, d1l) = (h[i], l[i]);
        let mut t = Trade { dir, legs: vec![Leg { i, px: lv, w: 1.0 }], exit_i: i, exit_px: c[i], reason: "open".into(), entry_kind: "stop", signal_i: i, open: true, exits: vec![] };
        if i == to { out.push(t); break; }
        let j = i + 1;
        let (obj, stp) = if dir > 0.0 { (d1h, d1l - TICK) } else { (d1l, d1h + TICK) };
        let (oj, hj, lj, cj) = (o[j], h[j], l[j], c[j]);
        let (hit_o, hit_s) = (if dir > 0.0 { hj >= obj } else { lj <= obj }, if dir > 0.0 { lj <= stp } else { hj >= stp });
        let (px, why) = if dir * (oj - stp) <= 0.0 { (oj, "stop (gap at the open)") } else if dir * (oj - obj) >= 0.0 { (oj, "objective (gap at the open)") }
            else if hit_o && hit_s {
                // both inside the bar: the TradeStation default path decides (high first when the high is nearer the open, a tie: the high)
                let up_first = hj - oj <= oj - lj;
                if (dir > 0.0) == up_first { (obj, "objective") } else { (stp, "stop") }
            } else if hit_o { (obj, "objective") } else if hit_s { (stp, "stop") } else { (cj, "exit on the Day-2 close") };
        t.exit_i = j; t.exit_px = px; t.reason = why.into(); t.open = false; t.signal_i = j;
        out.push(t);
        i = j + 1; // the Day-2 close is the earliest the next order can be placed: orders for bar j+1 use bar j
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn bar(d: &str, o: f64, h: f64, l: f64, c: f64) -> Row { Row { ticker: "T".into(), date: d.into(), open: o, high: h, low: l, close: c, volume: None } }
    #[test]
    fn down_close_buys_the_high_then_objective() {
        // bar1 close 99 < bar0 close 100 (down); bar2 opens 98.5: below the close 99, above the low 98 -> buy stop at the high of bar1 (100.5)
        let r = vec![bar("2020-01-01", 100., 101., 99., 100.), bar("2020-01-02", 100., 100.5, 98., 99.), bar("2020-01-03", 98.5, 101., 98.2, 100.),
            bar("2020-01-06", 100., 101.5, 99., 100.)];
        let x = Ctx::new(&r, &r, &[]);
        let t = run(ID, &x, 0, 3, &Params::new()).unwrap();
        assert_eq!(t.len(), 1);
        assert_eq!((t[0].dir, t[0].legs[0].px, t[0].exit_i, t[0].exit_px), (1.0, 100.5, 3, 101.0)); // Day-1 high 101 is the objective
    }
}
