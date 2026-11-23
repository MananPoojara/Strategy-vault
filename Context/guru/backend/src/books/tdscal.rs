//! "DeMark Scalper System" (_IXOG-TDScalper), Mark A. Simms / Index Options Group, Inc. (c.2000), TradeStation open code.
//! Book id `tdscal`. Source: research share `USB HDD 1/TradeStation Systems & Indicators/_Misc/_Tom Demark Scalper System
//! for TS, Excel/OpenCode/TDScalper-signal.txt, TDC.SEQ.SETUP2.txt, TDScalper-indicator.txt` (text copies: authors/india-1123/w/tdscal-src/).
//! Lane I2 (23 Nov 2026, lead instruction): coded literally on the NIFTY 50 / NIFTYBANK futures 1-minute series with volume
//! (Ctx::fut, lane DI) as 5-minute session-aligned bars, a labelled R4 #340 adaptation.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

const ID: &str = "tdscal_scalper";

const NOTE: &str = "A|Adapted (R4 #340, lead 23 Nov 2026), UNVERIFIED: coded literally from TDScalper-signal.txt and TDC.SEQ.SETUP2.txt (TradeStation open code; every input is a percentage, a bar count or a volume difference, no currency amount; the profit tests are percent of entry so BigPointValue cancels). The code's own test was S&P 500 5-minute futures 1/96-6/00 and EUR/USD 5-minute 2008 (a result report, not reproducible here); it runs on the NIFTY 50 / NIFTYBANK FUTURES 1-minute series staged by lane DI (NIFTY_50.FUT1 2018-12-28 to 2026-11-12, NIFTYBANK.FUT1 2023-09-06 to 2026-08-05; nearest-expiry continuous, NOT back-adjusted: a position carried over a contract roll includes the roll gap; days with a blank volume are absent from the series). Session 09:15-15:30 IST, 5-minute bars aligned to the session open (75 a day; the 15:30 auction minute dropped), bar volume = the sum of the minutes' traded volume, one continuous series across days as in TradeStation (counts, averages and positions carry overnight). Readings, 7, all labelled; under R4 #343 FOUR count as unstated details: (1) built-in VolumeOsc(4,12) = Average(Volume,4) - Average(Volume,12) in volume units (TradeStation's help describes it as the slow/fast difference with the labels swapped; the oscillator convention fast minus slow is used; help.tradestation.com VolumeOsc function, text copy in w/tdscal-src), (2) built-in SwingHighBar/SwingLowBar(1,price,8,9) = the bar 8 bars ago is a swing bar when it is at least as high (low) as the 8 bars before it and strictly higher (lower) than the 8 bars after it (TradeStation help, copy in w/tdscal-src), (3) the tie rule: of several next-bar orders the market order fills first at the open, else the stop the bar reaches first from its open, and the bar's other orders are dropped, (4) the literal port of the code's own quirks: the short MAE exit is written ExitLong (never exits a short), and the quick-profit and profit-stop exits sit under MktPosit > 0 (their short branches never run); the nested If/Else in the entry block binds the Else to the inner If, which is the code's evident intent (stop entry for 15 bars, market on the 15th). Engine conventions, not counted: (5) a stop order fills at its level, or at the next bar's open when the bar opens through it; a market order fills at the next bar's open; (6) no signal before the 20th bar of the series (MaxBarsBack); (7) a position open at the end of the data is closed at the last close and marked open. No costs. The book shows no result for NIFTY; the code's S&P and EUR/USD report is not comparable.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef {
        id: ID, name: "TD Scalper (Sequential setup flip, 5-minute NIFTY / BANKNIFTY futures)", group: "TradeStation open code", session: 0,
        dir: "Long/Short",
        status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "tdscal",
        rules: &[
            "F|_IXTDC.SEQ.Setup(4, 9, UseEquals=false, Close, RecountRev=true): count of closes below (above) the close 4 bars earlier; with RecountRev a count restarts only after an opposite bar; returns -count (buy) / +count (sell)",
            "F|DMCond = -1 when the count <= -9, +1 when >= 9, else 0; a flip out of -1 sets GoLong, out of +1 sets GoShort",
            "F|Volume filter on the first trigger bar: VolOsc = Average(VolumeOsc(4,12),3); cancel GoLong if VolOsc and VolOsc[4] are both < 0; cancel GoShort if both > -1",
            "E|Within MaxSignalWait = 15 bars: Buy at Highest(High,3) - 0.2% of Close stop (Sell at Lowest(Low,3) - 0.2% of Close stop); on the 15th bar go in at market (\"Alley-Oop\")",
            "X|Opposite signal after 15 bars and not on the entry date: exit next bar at market",
            "X|Key-reversal exit (needs VolOsc > 3); MAE exit between bars 55 and 85 when profit < -0.75% (swing exception); swing bail-out after 100 bars; quick-profit stop between bars 35 and 70 when profit > 1.5%; profit stop when profit > 3.5% long / 2.5% short",
            "X|As printed, the quick-profit and profit stops sit under `If MktPosit > 0`, so their short branches can never run, and the short MAE exit is written `ExitLong` - a literal port must keep these",
            NOTE],
        notes: "Built on DeMark's TD Sequential setup by Mark Simms (Index Options Group), not by DeMark himself. Published results: 5-minute S&P 500 futures 1/96-6/00 equity curve (~230 trades, peak Oct 98, \"did not adapt to a BIG change\"); EUR/USD 5-min 23.10.2008-31.12.2008: 57 trades, 45.61% profitable, profit factor 1.62, avg 208.8 bars per trade. Not reproduced: different market and period.",
        claim: None,
    }]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == ID).then_some(("tdscal", "TD Scalper", "5-minute NIFTY / BANKNIFTY futures"))
}
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    (id == ID).then(|| vec![p("bar", "Bar size (minutes; 5 per the code's test)", 5., 1., 30., 1.), p("wait", "MaxSignalWait (bars)", 15., 2., 40., 1.)])
}
pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if id != ID || to >= x.c.len() || from > to { return None; }
    let g = |k: &str| *p.get(k).unwrap_or(&params(id).unwrap().iter().find(|d| d.key == k).unwrap().def);
    Some(scalper(x, from, to, g("bar").round().max(1.0) as u16, g("wait").round().max(1.0) as usize))
}

struct B { i: usize, o: f64, h: f64, l: f64, c: f64, v: f64 }
/// (entry, buy, level) : level None = market.
#[derive(Clone, Copy)]
struct Od { entry: bool, buy: bool, lv: Option<f64> }

fn avg(v: &[f64], k: usize, n: usize) -> f64 { v[k + 1 - n..=k].iter().sum::<f64>() / n as f64 }

fn scalper(x: &Ctx, from: usize, to: usize, n: u16, wait: usize) -> Vec<Trade> {
    let Some(f) = x.fut else { return vec![] };
    let mut b: Vec<B> = vec![];
    for i in from..=to {
        let r = &x.r[i];
        let Some((m, mv)) = f.fut_day(&r.date) else { continue };
        let (open, close) = (crate::intraday::session_open(&r.ticker, &r.date), crate::intraday_ep::session_close(&r.ticker));
        for z in crate::intraday_ep::nbars(&m, open, n, close) { b.push(B { i, o: z.o, h: z.h, l: z.l, c: z.c, v: mv[z.a..z.b].iter().sum() }); }
    }
    let len = b.len();
    let vols: Vec<f64> = b.iter().map(|z| z.v).collect();
    let vo: Vec<f64> = (0..len).map(|k| if k >= 11 { avg(&vols, k, 4) - avg(&vols, k, 12) } else { f64::NAN }).collect();
    let vosc = |k: usize| if k >= 13 { (vo[k - 2] + vo[k - 1] + vo[k]) / 3.0 } else { f64::NAN };
    let mut tr: Vec<f64> = vec![f64::NAN; len];
    for k in 0..len { tr[k] = if k == 0 { b[k].h - b[k].l } else { (b[k].h - b[k].l).max((b[k].h - b[k - 1].c).abs()).max((b[k].l - b[k - 1].c).abs()) }; }
    let atr3 = |k: usize| avg(&tr, k, 3);
    let mut out: Vec<Trade> = vec![];
    // (dir, entry price, entry bar, entry day row)
    let mut pos: Option<(f64, f64, usize, usize)> = None;
    let mut orders: Vec<Od> = vec![];
    let (mut dn_cnt, mut up_cnt, mut prev_dn, mut prev_up) = (0usize, 0usize, false, false);
    let (mut dm_cond, mut go_long, mut go_short, mut first_bar) = (0i8, false, false, None::<usize>);
    let (mut sw_hi, mut sw_lo, mut prev_mkt) = (0usize, 0usize, 0i8);
    for k in 0..len {
        let z = &b[k];
        // 1. fills of the orders placed at the previous bar's close
        if !orders.is_empty() {
            let mut best: Option<(bool, f64, Od, f64)> = None;
            for &od in &orders {
                let (hit, px) = match od.lv { None => (true, z.o), Some(lv) if od.buy => (z.h >= lv, z.o.max(lv)), Some(lv) => (z.l <= lv, z.o.min(lv)) };
                if !hit { continue; }
                let key = (od.lv.is_some(), if od.buy { od.lv.unwrap_or(0.0) } else { -od.lv.unwrap_or(0.0) }); // market first, then the level reached first from the open
                let cand = (key.0, key.1, od, px);
                if best.map_or(true, |bb| (cand.0, cand.1) < (bb.0, bb.1)) { best = Some(cand); }
            }
            if let Some((_, _, od, px)) = best {
                if od.entry { pos = Some((if od.buy { 1.0 } else { -1.0 }, px, k, z.i)); }
                else if let Some(p) = pos.take() { out.push(crate::intraday::day_trade(p.0, p.3, p.1, z.i, px, "exit", if od.lv.is_some() { "limit" } else { "close" }, false)); }
            }
            orders.clear();
        }
        // 2. state at this bar's close (TradeStation order of the code)
        let mkt: i8 = pos.map_or(0, |p| p.0 as i8);
        if mkt == 0 && prev_mkt != 0 { sw_hi = 0; sw_lo = 0; }
        let since = pos.map_or(0, |p| k - p.2);
        if since > 0 { go_long = false; go_short = false; }
        let ep = pos.map_or(0.0, |p| p.1);
        // swing counters: SwingHighBar(1,high,8,9) = 8, i.e. bar k-8 is a swing bar
        if k > 8 && k >= 16 {
            let j = k - 8;
            if (1..=8).all(|d| b[j].h >= b[j - d].h) && (1..=8).all(|d| b[j].h > b[j + d].h) && since > 8 && (ep - b[j].h).abs() / ep * 100.0 < 0.5 { sw_hi += 1; }
            if (1..=8).all(|d| b[j].l <= b[j - d].l) && (1..=8).all(|d| b[j].l < b[j + d].l) && since > 8 && (ep - b[j].l).abs() / ep * 100.0 < 0.5 { sw_lo += 1; }
        }
        // 3. TD Sequential setup (TDC.SEQ.SETUP2, RecountRev = true, equals = false)
        let (dn, up) = if k >= 4 { (b[k].c < b[k - 4].c, b[k].c > b[k - 4].c) } else { (false, false) };
        if dn && (dn_cnt > 0 || (dn_cnt == 0 && prev_up)) { dn_cnt += 1; } else { dn_cnt = 0; }
        if up && (up_cnt > 0 || (up_cnt == 0 && prev_dn)) { up_cnt += 1; } else { up_cnt = 0; }
        prev_dn = dn; prev_up = up;
        let ret: i64 = if dn_cnt > 0 { -(dn_cnt as i64) } else if up_cnt > 0 { up_cnt as i64 } else { 0 };
        let v2: i8 = if ret <= -9 { -1 } else if ret >= 9 { 1 } else { 0 };
        let flip = (v2 != -1 && dm_cond == -1) || (v2 != 1 && dm_cond == 1);
        if flip && dm_cond == -1 { go_long = true; first_bar = Some(k); } else if flip && dm_cond == 1 { go_short = true; first_bar = Some(k); }
        dm_cond = v2;
        // 4. volume filter on the first trigger bar
        if first_bar == Some(k) && k >= 4 {
            if vosc(k) < 0.0 && vosc(k - 4) < 0.0 { go_long = false; }
            if vosc(k) > -1.0 && vosc(k - 4) > -1.0 { go_short = false; }
        }
        // 5. waiting period
        let mut within = 0i8;
        if mkt == 0 {
            if let Some(fb) = first_bar {
                if go_long && k - fb <= wait { within = 1; } else if go_short && k - fb <= wait { within = -1; }
            }
            if first_bar.map_or(true, |fb| k - fb > wait) { go_long = false; go_short = false; first_bar = None; }
        }
        prev_mkt = mkt;
        if k + 1 >= len { break; }
        // 6. entries (MaxBarsBack = 20)
        if mkt == 0 && k >= 20 {
            let fb = first_bar.unwrap_or(0);
            if within == 1 { if k - fb == wait { orders.push(Od { entry: true, buy: true, lv: None }); } else { let hi = b[k - 2..=k].iter().map(|v| v.h).fold(f64::NEG_INFINITY, f64::max); orders.push(Od { entry: true, buy: true, lv: Some(hi - 0.2 / 100.0 * z.c) }); } }
            if within == -1 { if k - fb == wait { orders.push(Od { entry: true, buy: false, lv: None }); } else { let lo = b[k - 2..=k].iter().map(|v| v.l).fold(f64::INFINITY, f64::min); orders.push(Od { entry: true, buy: false, lv: Some(lo - 0.2 / 100.0 * z.c) }); } }
        }
        // 7. exits
        if mkt != 0 {
            let d = mkt as f64;
            let pp = d * (z.c - ep) / ep * 100.0;
            if since > wait && z.i > pos.map_or(0, |p| p.3) {
                if mkt == 1 && go_short { orders.push(Od { entry: false, buy: false, lv: None }); } else if mkt == -1 && go_long { orders.push(Od { entry: false, buy: true, lv: None }); }
            }
            if k >= 8 {
                let hh = b[k - 8..k].iter().map(|v| v.h).fold(f64::NEG_INFINITY, f64::max); // HighestFC(High,8)[1]
                let ll = b[k - 8..k].iter().map(|v| v.l).fold(f64::INFINITY, f64::min);
                if mkt == 1 && z.h - 0.001 * z.h > hh && z.c < b[k - 1].c && z.l < b[k - 1].l && vosc(k) > 3.0 { orders.push(Od { entry: false, buy: false, lv: Some(z.l + 0.1 / 100.0 * z.c) }); }
                if mkt == -1 && z.l + 0.001 * z.l < ll && z.c > b[k - 1].c && z.h > b[k - 1].h && vosc(k) > 3.0 { orders.push(Od { entry: false, buy: true, lv: Some(z.l - 0.0 / 100.0 * z.c) }); }
            }
            if since > 55 && since < 85 && pp < -0.75 && mkt == 1 && sw_lo < 2 { orders.push(Od { entry: false, buy: false, lv: None }); } // short branch is ExitLong in the code: never exits a short
            if since > 100 {
                if mkt == 1 && sw_hi > 3 && pp < -0.5 { orders.push(Od { entry: false, buy: false, lv: Some(z.c + 0.1 / 100.0 * z.c) }); }
                if mkt == -1 && sw_lo > 3 && pp < -0.5 { orders.push(Od { entry: false, buy: true, lv: Some(z.c + 0.0) }); }
            }
            if mkt > 0 && k >= 2 {
                if since < 70 && since > 35 && pp > 1.5 { let lh = b[k - 2..=k].iter().map(|v| v.h).fold(f64::INFINITY, f64::min); orders.push(Od { entry: false, buy: false, lv: Some(lh + atr3(k) / 3.0) }); }
                if pp > 3.5 { let ah = b[k - 2..=k].iter().map(|v| v.h).sum::<f64>() / 3.0; orders.push(Od { entry: false, buy: false, lv: Some(ah - atr3(k) / 4.0) }); }
            }
        }
    }
    if let (Some(p), Some(z)) = (pos, b.last()) { out.push(crate::intraday::day_trade(p.0, p.3, p.1, z.i, z.c, "open", "limit", true)); }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn params_defaults() {
        assert_eq!(params(ID).unwrap().iter().map(|p| p.def).collect::<Vec<_>>(), vec![5., 15.]);
        assert!(params("x").is_none());
        assert_eq!(avg(&[1., 2., 3., 4.], 3, 2), 3.5);
    }
}
