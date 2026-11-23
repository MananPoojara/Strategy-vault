//! David S. Landry, "The 2/20-Day EMA Breakout System", Technical Analysis of Stocks & Commodities V.14:12 (Dec 1996), pp. 526-529
//! (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008)/VOLUMES/V14/C12/THE220D.pdf, rules box on p. 2 and
//! SIDE220.pdf p. 1). Book id `dl220`. Lane K2 (23 Nov 2026). Daily bars.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "dl220";
const ID: &str = "dl220_ema_breakout";

const NOTE: &str = "A|Declared readings (lane K2), 4: (1) '10 ticks' is 10 real ticks of the traded instrument (Ctx::market_tick: 0.05 on NSE indices/stocks, 0.01 on US series), the article's markets being commodity and currency futures; (2) the 20-day EMA is the usual alpha 2/21 seeded with the first close (the article prints no seed), the alert compares each day's low with that day's EMA; (3) the protective stop at the 20-day EMA is the previous day's EMA, trailed daily, and is live from the day after entry (the entry day's own path is not known from a daily bar); (4) no re-entry on the day of a stop-out. The article prints no result for the system on these series and notes it 'will have its own nuances' per market. No costs.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: ID, name: "2/20-Day EMA Breakout System", group: "The 2/20-Day EMA Breakout System (Landry, 1996)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
        rules: &["E|Buy alert: today's low and yesterday's low are both above the 20-day EMA; valid until the low touches or falls below the EMA. Buy entry: a stop 10 ticks above the two-day high, kept while the alert is valid; a gap opening beyond it fills at the open",
            "E|Sell alert: today's high and yesterday's high both below the 20-day EMA; sell entry a stop 10 ticks below the two-day low",
            "X|Long exit: a stop equal to the 20-day EMA, updated daily (trailing); short exit the same stop above", NOTE],
        notes: "Landry, S&C V.14:12, rules box (THE220D.pdf p. 2, SIDE220.pdf). No printed result for these markets.", claim: None }]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { (id == ID).then_some(("dl220", "2/20-day EMA breakout", "")) }
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    (id == ID).then(|| vec![ParamDef { key: "ema", label: "EMA length (days)", def: 20., min: 5., max: 60., step: 1. }, ParamDef { key: "ticks", label: "Ticks beyond the two-day extreme", def: 10., min: 0., max: 40., step: 1. }])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if id != ID || to >= x.c.len() || from >= to { return None; }
    let n = p.get("ema").copied().unwrap_or(20.0).round().max(1.0) as usize;
    let off = p.get("ticks").copied().unwrap_or(10.0) * x.market_tick();
    let a = 2.0 / (n as f64 + 1.0);
    let mut ema = vec![0.0; x.c.len()];
    for i in 0..x.c.len() { ema[i] = if i == 0 { x.c[0] } else { ema[i - 1] + a * (x.c[i] - ema[i - 1]) }; }
    let mut out: Vec<Trade> = vec![];
    let mut pos: Option<(f64, usize, f64)> = None;
    let leg = |ei: usize, ep: f64| vec![Leg { i: ei, px: ep, w: 1.0 }];
    for d in from.max(1)..=to {
        if let Some((dir, ei, ep)) = pos {
            if d > ei {
                let stop = ema[d - 1];
                let hit = if dir > 0.0 { x.l[d] <= stop } else { x.h[d] >= stop };
                if hit {
                    let px = if dir > 0.0 { x.o[d].min(stop) } else { x.o[d].max(stop) };
                    out.push(Trade { dir, legs: leg(ei, ep), exit_i: d, exit_px: px, reason: "stop at the 20-day EMA".into(), entry_kind: "limit", signal_i: ei, open: false, exits: Vec::new() });
                    pos = None;
                }
            }
            continue;
        }
        if d < 2 { continue; }
        // orders placed at the close of day d-1 from the alert on days d-2 and d-1
        let (t, y) = (d - 1, d - 2);
        if x.l[t] > ema[t] && x.l[y] > ema[y] {
            let lv = x.h[t].max(x.h[y]) + off;
            if x.o[d] >= lv { pos = Some((1.0, d, x.o[d])); } else if x.h[d] >= lv { pos = Some((1.0, d, lv)); }
        } else if x.h[t] < ema[t] && x.h[y] < ema[y] {
            let lv = x.l[t].min(x.l[y]) - off;
            if x.o[d] <= lv { pos = Some((-1.0, d, x.o[d])); } else if x.l[d] <= lv { pos = Some((-1.0, d, lv)); }
        }
    }
    if let Some((dir, ei, ep)) = pos { out.push(Trade { dir, legs: leg(ei, ep), exit_i: to, exit_px: x.c[to], reason: "open".into(), entry_kind: "limit", signal_i: ei, open: true, exits: Vec::new() }); }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sane_runs() { super::super::nijat::tests::sane(run, &[ID]); }
    /// hand-run dump for the pandas reference: DL220_CSV=<idx_data.csv> DL220_OUT=<file> cargo test --release dl220_dump -- --ignored
    #[test]
    #[ignore]
    fn dl220_dump() {
        use std::io::Write;
        let data = crate::load_csv(&std::env::var("DL220_CSV").unwrap()).unwrap();
        let mut f = std::io::BufWriter::new(std::fs::File::create(std::env::var("DL220_OUT").unwrap()).unwrap());
        writeln!(f, "id,ticker,dir,entry_date,entry_px,exit_date,exit_px,open").unwrap();
        for (t, rows) in data.iter() {
            let x = Ctx::new(rows, &[], &[]);
            for z in run(ID, &x, 0, rows.len() - 1, &Params::new()).unwrap() {
                writeln!(f, "{ID},{t},{},{},{},{},{},{}", z.dir, rows[z.legs[0].i].date, z.legs[0].px, rows[z.exit_i].date, z.exit_px, z.open).unwrap();
            }
        }
    }
}

pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "dl220",
    label: "Daily EMA-breakout rule with no currency amount (ticks of the instrument); run and independently checked on the five NSE index series only (NIFTY 50, NIFTYBANK, NIFTYIT, NIFTYMIDCAP100, NIFTYMIDCAPSELECT).",
    basis: "The 2/20-Day EMA Breakout System (Landry, S&C V.14:12), rules box; independent check authors/conv-1123/k2w/dl220_ref.py",
};
