//! George Pruitt, "DayTurbo" (FT Daytrade, Stafford Trading; research share `USB HDD 1/TradeStation Systems & Indicators/_FT Daytrade, $1200
//! (stafforddaytrading.com)/DayTurbo_OpenSource.txt`, complete EasyLanguage code read). Book id `dturbo`. Lane K1 (25 Nov 2026, R4 #398 R397): 5-minute NIFTY 50 /
//! NIFTYBANK futures bars as a labelled R4 #340 adaptation, one scaling rule R4 #380. Own two-contract walker (partial exit, break-even stop). Reference authors/conv-1123/K1/dturbo_ref.py.
use super::k4x::fut_bars;
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

const ID: &str = "dturbo_ft";
const NOTE: &str = "[Declared adaptation, R4 #340 / #380] George Pruitt, DayTurbo (FT Daytrade, open EasyLanguage code for the S&P day session on 5-minute bars). Rule as coded: atr = average of the last 10 daily true ranges (prior days); small range = yesterday's true range below atr; if yesterday closed at or above the day before and the range was small, buy 2 contracts at a stop of today's open + 0.2 x atr; if it closed below, sell 2 contracts at today's open - 0.2 x atr; one entry per day, no new order from 15:30 on; stop 10 points for the position; once the day's high is more than 5 points above the entry one contract exits at a stop 5 points under the day's high (short: mirrored), the other contract then has a break-even stop at the entry price; flat at the close of the session. Labelled readings (4): (1) the daily bars the code reads (OpenD/HighD/LowD/CloseD) are the sessions' own 1-minute bars of the futures series, the bars are 5-minute session-aligned bars; (2) ONE scaling rule (#380): a point = entry price / 1,300 (the S&P 500 stood near 1,300 when the code's June 2011 backtest was run), so the 10-point stop is 0.77 percent, the 5-point trigger and trail 0.385 percent of the entry price; the 0.2 x atr offset needs no scaling; (3) several orders touched inside one 5-minute bar fill in the engine's usual order (the extreme nearer the open first, then the other, then the close), a stop fills at its level or at the open when the bar opens through it, and orders placed at a bar's close act from the next bar; (4) 'time < 1530' is read as printed in the series' own (NSE) clock, so it only forbids an order placed on the last bar of the session; the open position leaves at the session's last close (SetExitOnClose). The two contracts are one position of two equal lots (the result is the average of the two lots). The vendor prints no result for this rule.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef {
        id: ID, name: "DayTurbo: small-range day, stop entry at open +- 0.2 ATR, 2 contracts (partial at day high -5, break-even), 5-minute NIFTY / BANKNIFTY futures", group: "FT Daytrade (Pruitt / Stafford Trading)", session: 0,
        dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "dturbo",
        rules: &[
            "F|\"smallRange = yesTrueRange < atrAmt\" (atrAmt = average true range of the last 10 daily bars, atrLen 10)",
            "E|\"if(closeD(1)>=closeD(2) and smallRange) then canBuy = true ... buy(\\\"SFOBuy\\\") 2 contracts next bar stb stop\" (stb = openD(0) + 0.2 x atrAmt; the mirror image sells at openD(0) - 0.2 x atrAmt when closeD(1) < closeD(2)); one entry per day, time < 1530",
            "X|\"sell(\\\"LongProf1\\\") 1 contract next bar at highD(0) - 5 stop once highD(0) > entryPrice + 5; if currentContracts = 1 then sell next bar at entryPrice stop; sell at entryPrice - stopAmtPoints (10) stop; setExitOnClose\"",
        ],
        notes: NOTE, claim: None,
    }]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { (id == ID).then_some(("dturbo", "DayTurbo (FT Daytrade)", "5-minute NIFTY / BANKNIFTY futures")) }
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    (id == ID).then(|| vec![p("len", "Average true range length (days)", 10., 3., 30., 1.), p("off", "Entry offset (x ATR)", 0.2, 0., 1., 0.05),
        p("stop", "Stop (S&P points)", 10., 1., 40., 0.5), p("profit", "Partial-exit trigger above entry (S&P points)", 5., 1., 20., 0.5), p("trail", "Partial-exit trail below the day high (S&P points)", 5., 1., 20., 0.5),
        p("scale", "S&P index level that defines one point (#380)", 1300., 500., 3000., 50.)])
}

#[derive(Clone, Copy, PartialEq)]
enum K { Entry, Trail, Be, Mm }

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if id != ID || to >= x.c.len() || from > to { return None; }
    let g = |k: &str| *p.get(k).unwrap_or(&params(id).unwrap().iter().find(|d| d.key == k).unwrap().def);
    let Some(fb) = fut_bars(x, from, to, 5) else { return Some(vec![]) };
    let (len, off, stp, prf, trl, scale) = (g("len").round() as usize, g("off"), g("stop"), g("profit"), g("trail"), g("scale"));
    let n = fb.o.len();
    // day table: (first bar, last bar, open, high, low, close)
    let mut dt: Vec<(usize, usize, f64, f64, f64, f64)> = vec![];
    for k in 0..n {
        if k == 0 || fb.day[k] != fb.day[k - 1] { dt.push((k, k, fb.o[k], fb.h[k], fb.l[k], fb.c[k])); }
        let z = dt.last_mut().unwrap();
        z.1 = k; z.3 = z.3.max(fb.h[k]); z.4 = z.4.min(fb.l[k]); z.5 = fb.c[k];
    }
    let mut out: Vec<Trade> = vec![];
    for t in len + 1..dt.len() {
        let tr = |j: usize| dt[t - j].3.max(dt[t - j - 1].5) - dt[t - j].4.min(dt[t - j - 1].5); // true range of the day j back
        let atr = (1..=len).map(tr).sum::<f64>() / len as f64;
        if !(tr(1) < atr) { continue; }
        let s: f64 = if dt[t - 1].5 >= dt[t - 2].5 { 1.0 } else { -1.0 };
        let (k0, k1) = (dt[t].0, dt[t].1);
        let open0 = dt[t].2;
        let lvl = s * open0 + off * atr; // buy-stop level in the signed space (price x s)
        let (mut lots, mut ep, mut pt, mut entered) = (0u8, 0.0f64, 0.0f64, false);
        let mut dh = f64::NEG_INFINITY; // day high in the signed space
        let mut pend: Vec<(K, f64)> = vec![];
        let mut pieces: Vec<(usize, f64, &'static str)> = vec![]; // (day row, real price, reason) one per lot
        let (mut sig, mut fill_bar, mut kind): (usize, usize, &'static str) = (k0, k0, "stop");
        for k in k0..=k1 {
            let (op, hi, lo, cl) = (fb.o[k], fb.h[k], fb.l[k], fb.c[k]);
            // execute the orders placed at the previous close over bar k
            if !pend.is_empty() {
                let (ops, his, los, cls) = (s * op, if s > 0.0 { hi } else { -lo }, if s > 0.0 { lo } else { -hi }, s * cl);
                let first_hi = hi - op < op - lo; // the engine's path: the extreme nearer the open first (original space)
                let path: [f64; 3] = if first_hi { [hi, lo, cl] } else { [lo, hi, cl] };
                let path = [s * path[0], s * path[1], s * path[2]];
                let _ = (his, los, cls);
                let mut done = vec![false; pend.len()];
                let valid = |kd: K, lots: u8| match kd { K::Entry => lots == 0, K::Trail => lots == 2, K::Be => lots == 1, K::Mm => lots >= 1 };
                let mut fill = |j: usize, px_s: f64, why: &'static str, lots: &mut u8, ep: &mut f64, pt: &mut f64, kind: &mut &'static str, fill_bar: &mut usize, pieces: &mut Vec<(usize, f64, &'static str)>| {
                    let real = s * px_s;
                    match pend[j].0 {
                        K::Entry => { *lots = 2; *ep = real; *pt = real / scale; *kind = why; *fill_bar = k; }
                        K::Trail => { *lots = 1; pieces.push((fb.day[k], real, "trail under the day high")); }
                        K::Be | K::Mm => { for _ in 0..*lots { pieces.push((fb.day[k], real, if pend[j].0 == K::Be { "break-even stop" } else { "stop" })); } *lots = 0; }
                    }
                };
                // gap through a level: fill at the open
                for j in 0..pend.len() {
                    if done[j] || !valid(pend[j].0, lots) { continue; }
                    let hit = if pend[j].0 == K::Entry { ops >= pend[j].1 } else { ops <= pend[j].1 };
                    if hit { done[j] = true; fill(j, ops, "open", &mut lots, &mut ep, &mut pt, &mut kind, &mut fill_bar, &mut pieces); }
                }
                let (mut at, mut seg) = (ops, 0);
                while seg < 3 {
                    let tg = path[seg];
                    let up = tg > at;
                    let mut best: Option<(usize, f64)> = None;
                    for j in 0..pend.len() {
                        if done[j] || !valid(pend[j].0, lots) { continue; }
                        let lv = pend[j].1;
                        let rising = pend[j].0 == K::Entry;
                        if rising != up { continue; }
                        let inside = if up { lv > at && lv <= tg } else { lv < at && lv >= tg };
                        if inside && best.map_or(true, |(_, b)| (lv - at).abs() < (b - at).abs()) { best = Some((j, lv)); }
                    }
                    match best { Some((j, lv)) => { done[j] = true; fill(j, lv, "stop", &mut lots, &mut ep, &mut pt, &mut kind, &mut fill_bar, &mut pieces); at = lv; } None => { at = tg; seg += 1; } }
                }
                pend.clear();
            }
            dh = dh.max(if s > 0.0 { hi } else { -lo });
            if lots > 0 { entered = true; }
            if k == k1 {
                if lots > 0 { for _ in 0..lots { pieces.push((fb.day[k], cl, "end of day")); } lots = 0; }
                break;
            }
            // orders for the next bar (placed at this close)
            if lots == 0 {
                if !entered && crate::books::k4x::hhmm(fb.stamp[k]) < 1530 { pend.push((K::Entry, lvl)); sig = k; }
            } else {
                if lots == 2 && dh > s * ep + prf * pt { pend.push((K::Trail, dh - trl * pt)); }
                if lots == 1 { pend.push((K::Be, s * ep)); }
                pend.push((K::Mm, s * ep - stp * pt));
            }
        }
        if pieces.len() == 2 {
            let l = pieces.last().unwrap().clone();
            let fin_day = pieces.iter().map(|z| z.0).max().unwrap();
            let w = 0.5;
            out.push(Trade { dir: s, legs: vec![Leg { i: fb.day[fill_bar], px: ep, w: 1. }], exit_i: fin_day, exit_px: w * pieces[0].1 + w * pieces[1].1, reason: l.2.into(), entry_kind: kind, signal_i: fb.day[sig], open: false,
                exits: pieces.iter().map(|z| (z.0, z.1, w)).collect() });
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn defaults() {
        assert_eq!(params(ID).unwrap().iter().map(|p| p.def).collect::<Vec<_>>(), vec![10., 0.2, 10., 5., 5., 1300.]);
        assert!(params("x").is_none());
    }
}
