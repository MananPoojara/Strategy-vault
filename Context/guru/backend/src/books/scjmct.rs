//! John L. Momsen, "Trade Crude Now: Options, Not Outright", Technical Analysis of Stocks & Commodities V.19:8 (2001) pp.53-56
//! (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V19/C08/089CRDE.pdf),
//! sidebar "Crude oil seasonal trade #2" (the TradeStation code). Book id `scjmct`. Lane PA2 (13 Nov 2026); independent check
//! authors/pa2/w/pa2_fut_ref.py. Runs on the Norgate crude-oil continuous contract only (futures::CARDS_PA2, R4 #293 (1)).
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::tselhs::{stop_fill, Book};

pub const REVIEWED: &[&str] = &["scjmct_crude2"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Printed TradeStation code, no unstated detail; unverified] 'If Month(d) = 8 then buy (\"Long\") next bar at Highest (H,5) + 1 point stop; ExitLong from entry (\"Long\") next bar at Lowest (L,3) - 1 point stop; (This is not active on the day of entry ...) If EntryPrice<=Lowest (L,8)-1 point then ExitLong at Lowest (L,8)-1 point stop; (Profit Stop) If (Month(d)=9 and DayOfMonth(d)>26) or Month(d)=10 then ExitLong on close;'. Read literally on daily bars: an August close places a buy stop one tick above the last five highs for the next bar (so the entry window runs to the first September session, as the article says: 'active from August 3 through September 1'); from the day after entry a sell stop sits one tick under the last three lows, raised to one tick under the last eight lows once that level is at or above the entry price; a stop fills at its level, or at the open when the day opens through it; the position is sold at the close of any session after September 26 or in October. One point = one tick (0.01, R4 #57). No re-entry on the day of an exit; one position at a time. The entry day has only the entry order and later days only sell orders, so no trade depends on the order inside a day (#240). The article trades the November contract (and an option spread on it); the vault runs the Norgate crude continuous contract (signals on the back-adjusted series, trades priced in the contract held at entry, R4 #293, #301). Norgate continuous futures, ends 2020-09-11; roll method not stated by the vendor. Figure 2 plots the yearly profits 1983-1999 as a chart only, so it stays UNVERIFIED.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "scjmct_crude2", name: "Momsen crude oil seasonal trade #2: August 5-day-high buy stop, 3-day / 8-day low stops, out after September 26 (crude futures)", group: "S&C: trade crude now (Momsen, V.19:8)", session: 0, dir: "Long", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "scjmct", rules: &[
            "E|\"If Month(d) = 8 then buy (\"Long\") next bar at Highest (H,5) + 1 point stop;\" (V.19:8 sidebar, PDF p.4)",
            "X|\"ExitLong from entry (\"Long\") next bar at$ Lowest (L,3) - 1 point stop; (This is not active on the day of entry so the protective stop for that day will need to be computed by hand.)\" (sidebar)",
            "X|\"If EntryPrice<=Lowest (L,8)-1 point then ExitLong at Lowest (L,8)-1 point stop; (Profit Stop)\" (sidebar)",
            "X|\"If (Month(d)=9 and DayOfMonth(d)>26 ) or Month(d)=10 then ExitLong on close;\" (sidebar)",
            "F|\"For my crude oil #2 trade, the entry channel is five trading days (active from August 3 through September 1), the protective stop is three trading days, and the trailing profit stop is eight trading days (the exit day is September 27 on close).\" (PDF p.2)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { reviewed(id).then_some(("scjmct_crude2", "Momsen crude oil seasonal (S&C)", "#2")) }
pub fn params(id: &str) -> Option<Vec<ParamDef>> { reviewed(id).then(Vec::new) }

fn md(x: &Ctx, i: usize) -> (u32, u32) { let d = &x.r[i].date; (d[5..7].parse().unwrap_or(0), d[8..10].parse().unwrap_or(0)) }
fn hi(v: &[f64], t: usize, n: usize) -> f64 { v[t + 1 - n..=t].iter().copied().fold(f64::MIN, f64::max) }
fn lo(v: &[f64], t: usize, n: usize) -> f64 { v[t + 1 - n..=t].iter().copied().fold(f64::MAX, f64::min) }

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from > to || to >= x.r.len() { let _ = p; return None; }
    let tick = x.market_tick();
    let mut pos = Book::new();
    for i in from.max(8)..=to {
        let t = i - 1; // orders placed at the close of bar t for bar i
        let (m, _) = md(x, t);
        if pos.side() > 0. {
            let e = pos.entry_bar().unwrap_or(i);
            if e < i {
                // stops placed at t's close (not on the entry day itself: e == i is excluded above)
                let prot = lo(&x.l, t, 3) - tick;
                let prof = lo(&x.l, t, 8) - tick;
                let stop = if pos.entry_px() <= prof { prot.max(prof) } else { prot };
                if let Some(px) = stop_fill(x, i, -1., stop) { pos.exit(i, px, "channel stop"); continue; }
            }
            let (mi, di) = md(x, i);
            if (mi == 9 && di > 26) || mi == 10 { pos.exit(i, x.c[i], "after September 26: on close"); }
            continue;
        }
        if m == 8 && i < to {
            if let Some(px) = stop_fill(x, i, 1., hi(&x.h, t, 5) + tick) { pos.enter(1., i, px, "stop"); }
        }
    }
    Some(pos.done(to, x.c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn august_entries_and_september_exits() {
        let r = super::super::sctraub::tests::bars(2600);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("scjmct_crude2", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() >= 5);
        for z in &t {
            let e = z.legs[0].i;
            assert!(&r[e - 1].date[5..7] == "08" && z.dir > 0.);
            assert!(z.exit_i > e && (z.open || z.reason.contains("stop") || &r[z.exit_i].date[5..7] >= "09"));
        }
    }
}
