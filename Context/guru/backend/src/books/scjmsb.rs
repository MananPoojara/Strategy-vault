//! John L. Momsen, "Seasonal Trades And The Channel Breakout System", Technical Analysis of Stocks & Commodities V.18:2 (2000)
//! pp.36-43 (research share Trading Courses,Seminars,Videos/Stocks & Commodities Magazine CD 6.21 (traders.com)/volumes/V18/C02/
//! 012SEA.pdf), sidebar "Soybean seasonal trade" (printed TradeStation code). Book id `scjmsb`. Lane K3 (23 Nov 2026).
//! Runs on the Norgate soybean continuous contract only (futures.rs card); the article trades the November contract.
//! Independent check authors/conv-1123/k3w/scjmsb_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::tselhs::{stop_fill, Book};

pub const REVIEWED: &[&str] = &["scjmsb_soy"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Printed TradeStation code; unverified] 'Input: EntStop(16), ProtStop(5), ProfSt(11); Condition1 = Month(d) = 5 and DayOfMonth(d) > 14; Condition2 = Month(d) = 6; Condition3 = Month(d) = 7; If (Condition1 or Condition2 or Condition3) then sell (\"Short\") tomorrow at Lowest(L,EntStop) - 6 points stop; ExitShort from entry (\"Short\") next bar at Highest(H,ProtStop) + 1 point stop; (not active on the day of entry) If EntryPrice > Highest(H,ProfSt) + 1 point then ExitShort at Highest(H,ProfSt) + 1 point stop; If Month(d) = 8 and DayOfMonth(d) > 12 then ExitShort on close'. Read literally on daily bars: from a close after 14 May through July a sell stop is placed 6 points under the lowest low of the last 16 bars for the next bar (the article: a 1.5 cent penetration); from the day after entry a buy stop sits one point over the last five highs, replaced by one point over the last eleven highs once that level is under the entry price (the lower stop is the one that fills); a stop fills at its level, or at the open when the day opens through it; the position is covered at the close of any session after 12 August. One point = one tick = 0.25 cent (the article: 'Place your protective stop at 1 tick (0.25 cent) above the high' and '11/2 cents' = 6 points). Re-entry on a later bar is allowed (the article: 're-enter the trade at the new 16-day low less 11/2 cents'); no re-entry on the day of an exit; one position at a time. The entry day has only the entry order and later days only buy orders. The article trades the November contract 1977-1999 and prints a 23-year table (net $44,268); this card runs on the Norgate continuous contract (back-adjusted), a labelled adaptation of the November-contract file.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "scjmsb_soy", name: "Momsen soybean seasonal short: 16-day channel-low sell stop (mid-May to July), 5-day / 11-day high stops, out after 12 August (soybean futures)", group: "S&C: seasonal trades and the channel breakout (Momsen, V.18:2)", session: 0, dir: "Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "scjmsb", rules: &[
            "E|\"Condition1 = Month(d) = 5 and DayOfMonth(d) > 14; Condition2 = Month(d) = 6; Condition3 = Month(d) = 7; If (Condition1 or Condition2 or Condition3) then sell (\\\"Short\\\") tomorrow at Lowest(L,EntStop) - 6 points stop;\" (V.18:2 sidebar, PDF p.4)",
            "X|\"ExitShort from entry (\\\"Short\\\") next bar at Highest(H,ProtStop) + 1 point stop; (this stop is not active on the day of entry)\" (sidebar)",
            "X|\"If EntryPrice > Highest(H,ProfSt) + 1 point then ExitShort at Highest(H,ProfSt) + 1 point stop;\" (sidebar)",
            "X|\"If Month(d) = 8 and DayOfMonth(d) > 12 then ExitShort on close;\" (sidebar)",
            "F|\"a 16-day channel low for entering the trade, a five-day channel high for the initial protective stop, and an 11-day channel high for the trailing profit stop\" (V.18:2 p.39)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { reviewed(id).then_some(("scjmsb_soy", "Momsen soybean seasonal (S&C)", "soybeans")) }
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    reviewed(id).then(|| vec![
        ParamDef { key: "ent", label: "Entry channel: lowest low of the last N bars (EntStop)", def: 16., min: 2., max: 40., step: 1. },
        ParamDef { key: "prot", label: "Protective stop: highest high of the last N bars (ProtStop)", def: 5., min: 2., max: 20., step: 1. },
        ParamDef { key: "prof", label: "Trailing profit stop: highest high of the last N bars (ProfSt)", def: 11., min: 2., max: 40., step: 1. }])
}

fn md(x: &Ctx, i: usize) -> (u32, u32) { let d = &x.r[i].date; (d[5..7].parse().unwrap_or(0), d[8..10].parse().unwrap_or(0)) }
fn hi(v: &[f64], t: usize, n: usize) -> f64 { v[t + 1 - n..=t].iter().copied().fold(f64::MIN, f64::max) }
fn lo(v: &[f64], t: usize, n: usize) -> f64 { v[t + 1 - n..=t].iter().copied().fold(f64::MAX, f64::min) }

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from > to || to >= x.r.len() { return None; }
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d).round().max(2.) as usize;
    let (ent, prot_n, prof_n) = (g("ent", 16.), g("prot", 5.), g("prof", 11.));
    let tick = x.market_tick();
    let warm = ent.max(prot_n).max(prof_n);
    let mut pos = Book::new();
    for i in from.max(warm)..=to {
        let t = i - 1; // orders placed at the close of bar t for bar i
        if pos.side() < 0. {
            let e = pos.entry_bar().unwrap_or(i);
            if e < i {
                let prot = hi(&x.h, t, prot_n) + tick;
                let prof = hi(&x.h, t, prof_n) + tick;
                let stop = if pos.entry_px() > prof { prot.min(prof) } else { prot };
                if let Some(px) = stop_fill(x, i, 1., stop) { pos.exit(i, px, "channel stop"); continue; }
            }
            let (mi, di) = md(x, i);
            if mi == 8 && di > 12 { pos.exit(i, x.c[i], "after 12 August: on close"); }
            continue;
        }
        let (m, d) = md(x, t);
        if (m == 5 && d > 14 || m == 6 || m == 7) && i < to {
            if let Some(px) = stop_fill(x, i, -1., lo(&x.l, t, ent) - 6. * tick) { pos.enter(-1., i, px, "stop"); }
        }
    }
    Some(pos.done(to, x.c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn summer_entries_and_august_exits() {
        let r = super::super::sctraub::tests::bars(2600);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("scjmsb_soy", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(!t.is_empty());
        for z in &t {
            let e = z.legs[0].i;
            assert!(z.dir < 0. && matches!(&r[e].date[5..7], "05" | "06" | "07" | "08"));
            assert!(z.exit_i > e);
        }
    }
    /// hand-run dump for the pandas reference: SBM_DIR=<norgate dir> SBM_OUT=<file> cargo test --release scjmsb_dump -- --ignored
    #[test]
    #[ignore]
    fn scjmsb_dump() {
        use std::io::Write;
        let mut data = std::collections::BTreeMap::new();
        crate::futures::load(std::path::Path::new(&std::env::var("SBM_DIR").unwrap()), &mut data);
        let rows = &data["Soybeans (Norgate &ZS)"];
        let x = Ctx::new(rows, &[], &[]);
        let mut f = std::io::BufWriter::new(std::fs::File::create(std::env::var("SBM_OUT").unwrap()).unwrap());
        writeln!(f, "dir,entry_date,entry_px,exit_date,exit_px,open").unwrap();
        for z in run("scjmsb_soy", &x, 0, rows.len() - 1, &Params::new()).unwrap() {
            writeln!(f, "{},{},{},{},{},{}", z.dir, rows[z.legs[0].i].date, z.legs[0].px, rows[z.exit_i].date, z.exit_px, z.open).unwrap();
        }
    }
}
