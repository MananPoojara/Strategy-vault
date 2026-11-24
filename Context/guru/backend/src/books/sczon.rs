//! Massimiliano Scorpio, "Developing Hot Zones", Technical Analysis of Stocks & Commodities V.20:12 (Dec 2002) pp.20-33 (research share
//! Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V20/C12/157zone.pdf), sample system p.30-31. Book id `sczon`.
//! Lane K2 (24 Nov 2026, tier 5 row I624). Independent check authors/conv-1123/k2w/qi/sczon_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade, Leg};

pub const REVIEWED: &[&str] = &["sczon_hotzone"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "A|[Declared readings, lane K2, 4] (1) ATR(10) is the simple mean of the true range over the 10 bars ending at the previous bar (TradeStation AvgTrueRange). (2) Both stops can be touched in one day and the article does not give the order inside a daily bar: the stop nearer the open is taken first and the day's other order is cancelled (one trade per day). (3) The sell stop is read as a short entry (the printed code sells at the previous high). (4) No tick rounding of the stop levels. The open lies strictly between the two levels, so both fill at their levels; the exit is the day's close.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "sczon_hotzone", name: "Scorpio Hot Zones: after a key reversal up day, an open between the previous high and +30% of ATR(10): buy stop above, sell stop at the previous high, exit at the close", group: "S&C: Developing Hot Zones (Scorpio, V.20:12)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "sczon", rules: &[
            "E|Setup: after a key reversal up day (low below the prior low and close above the prior close) the market must open above the previous high but lower than the previous high + 30% of the 10-period ATR. Buy order: buy at the previous high + 30% of ATR(10) at stop. Sell order: sell at the previous high at stop (pp.30-31 sample system)",
            "X|Exit order: close the position at the close of the day (SetExitOnClose)",
            NOTE], notes: NOTE, claim: None }]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { reviewed(id).then_some(("sczon", "Scorpio Hot Zones (S&C)", "Key reversal then zone stops")) }
pub fn params(_id: &str) -> Option<Vec<ParamDef>> { None }

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, _p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let mut out = vec![];
    let tr = |k: usize| (x.h[k] - x.l[k]).max((x.h[k] - x.c[k - 1]).abs()).max((x.l[k] - x.c[k - 1]).abs());
    for t in from.max(12)..=to {
        let k = t - 1; // the setup bar
        if !(x.l[k] < x.l[k - 1] && x.c[k] > x.c[k - 1]) { continue; }
        let atr = (k - 9..=k).map(tr).sum::<f64>() / 10.;
        let (buy, sell) = (x.h[k] + 0.3 * atr, x.h[k]);
        if !(x.o[t] > x.h[k] && x.o[t] < buy) { continue; }
        let (hb, hs) = (x.h[t] >= buy, x.l[t] <= sell);
        let side = match (hb, hs) { (false, false) => continue, (true, false) => 1., (false, true) => -1., (true, true) => if buy - x.o[t] <= x.o[t] - sell { 1. } else { -1. } };
        let px = if side > 0. { buy } else { sell };
        out.push(Trade { dir: side, legs: vec![Leg { i: t, px, w: 1. }], exit_i: t, exit_px: x.c[t], reason: "close".into(), entry_kind: "stop", signal_i: k, open: false, exits: vec![] });
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    /// hand-run dump for the pandas reference: SCZON_CSV=<idx_data.csv> SCZON_OUT=<file> cargo test --release sczon_dump -- --ignored
    #[test]
    #[ignore]
    fn sczon_dump() {
        use std::io::Write;
        let data = crate::load_csv(&std::env::var("SCZON_CSV").unwrap()).unwrap();
        let mut f = std::io::BufWriter::new(std::fs::File::create(std::env::var("SCZON_OUT").unwrap()).unwrap());
        writeln!(f, "id,ticker,dir,entry_date,entry_px,exit_date,exit_px,open").unwrap();
        for (t, rows) in data.iter() {
            let x = Ctx::new(rows, rows, &[]);
            for z in run("sczon_hotzone", &x, 0, rows.len() - 1, &Params::new()).unwrap() {
                writeln!(f, "sczon_hotzone,{t},{},{},{},{},{},{}", z.dir, rows[z.legs[0].i].date, z.legs[0].px, rows[z.exit_i].date, z.exit_px, z.open).unwrap();
            }
        }
    }
    #[test]
    fn buy_stop_filled_in_the_zone() {
        use crate::Row;
        let b = |d: &str, o: f64, h: f64, l: f64, c: f64| Row { ticker: "T".into(), date: d.into(), open: o, high: h, low: l, close: c, volume: None };
        let mut r: Vec<Row> = (0..13).map(|i| b(&format!("{i:02}"), 100., 102., 98., 100.)).collect();
        r.push(b("13", 99., 103., 97., 102.)); // key reversal up: low 97 < 98, close 102 > 100
        r.push(b("14", 103.5, 106., 103.2, 105.)); // opens above 103, below 103 + 0.3 ATR (ATR about 4.4)
        let x = Ctx::new(&r, &r, &[]);
        let t = run("sczon_hotzone", &x, 0, 14, &Params::new()).unwrap();
        assert_eq!(t.len(), 1);
        assert_eq!(t[0].dir, 1.);
        assert!(t[0].legs[0].px > 103. && t[0].legs[0].px < 105. && t[0].exit_px == 105.);
    }
}

// Instrument admission: Scorpio Hot Zones 2026-11-24 (lane K2, tier 5).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "sczon" && reviewed(def.id),
    label: "Scorpio tests the sample system on DAX futures 8/16/91-9/20/02 (115 trades, +28,045 EUR, profit factor 2.20, as printed in the article); run on the five NSE index series the independent reference covers (the gate row allows only those, R4 #378).",
    basis: "Technical Analysis of Stocks & Commodities V.20:12 pp.30-31 (Developing Hot Zones); independent check authors/conv-1123/k2w/qi/sczon_ref.py",
};
