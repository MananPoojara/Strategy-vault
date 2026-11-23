//! Neural DT 1 (Mike Bryant, Breakout Futures, 2007), TradeStation: research share `USB HDD 1/TradeStation Systems & Indicators/Neural DT System for Tradestation
//! (breakoutfutures.com)/.../Code.rar::NeuralDT1.txt` (read-only copy read in full). Book id `ndt1`. Lane K4 (Q385, 24 Nov 2026): the 1-minute system (the file name) on the
//! NIFTY 50 / NIFTYBANK futures 1-minute series as a labelled R4 #340 adaptation, one scaling rule R4 #380. Helper books::k4x. Independent check authors/code-1123/ref/ndt1_ref.py.
use super::k4x::{fut_bars, hhmm, o, run_fb, A, P};
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use std::collections::HashMap;

const ID: &str = "ndt1_neural_dt1";
const NOTE: &str = "[Declared adaptation, R4 #340 and R4 #382] Neural DT 1 (Breakout Futures 2007; the 60-minute twin is NeuralDT60.txt). 1-minute bars; ATR = Average(TrueRange,20); AMA = AdaptiveMovAvg(C,10,2,30) (Kaufman); six raw inputs I1r = volume - Average(volume,20), I2r = (C - C[2])/ATR, I3r = (C - AMA)/ATR, I4r = (AMA - Average(C,81))/ATR, I5r = (C - DayMode)/ATR, I6r = (TrueRange - ATR)/ATR, each scaled to [-1,1] by its running maximum and minimum (I = (2 x Ir - (max + min))/(max - min), initial max 0 and min 99999999 as printed); NNOut = tanh(w1 I1 + ... + w6 I6) with the printed weights 1, 1, 0, -1, 1, 0. Long when MarketPosition <> 1, time < EntTime (session start + TmEnt 40 min), no entry today, Close > Average(Close,675) and NNOut >= 0.5: Buy next bar at Highest(High,2) + 1 point stop, stop = entry - StopFr (4.5) x ATR, target = entry + max(1, TargFr (5) x ATR); short mirrored (Lowest(Low,2), no offset). Trailing: once the close is FloorFr (3.5) x ATR beyond the entry the stop is the better of itself and the entry plus TrailPr (10%) of the open profit; exit at market at EntTime + TmExt (10 min); exit at the session close. Labelled readings (4): (1) SUBSTITUTE, NOT THE AUTHOR'S FUNCTION: PriceMode(DayH,DayL,DayC,DayMode) is a proprietary function that is not in the file (its body and its 'mode' definition are unknown); the vault substitutes DayMode = the most frequent 1-minute close (rounded to the 0.05 tick, ties to the lower price) of the previous session, which feeds input I5 with the printed weight 1; (2) every indicator runs continuously over the futures series from its first bar (AMA seeded with the first close, the running input maxima and minima start at the first bar, no entry before bar 675); (3) the two fixed amounts of one point (the +1 entry offset and the 1-point minimum target) follow ONE scaling rule, amount x price / 1,500 (an S&P level of about 1,500 in 2007), counted as one detail; (4) the session clock is the exchange clock (session opens 09:15, EntTime 09:55, ExtTime 10:05), compared on the bar's END time stamp (TradeStation stamps a bar with its close time, fb.stamp = the bucket end): the bar ending 09:55 is the first bar excluded from signalling and the bar ending 10:05 the first to trigger the time exit, as in the code's own 'time < EntTime' and 'time >= ExtTime'. The file's Print / Commentary lines and the dollar stop value are not trading rules. No result is printed. Order-dependence (R4 #175 / #240): on 1-minute bars a target limit and a stop were both touched inside one pending bar in 1 of 19,433 pending exit-pair bars on NIFTY 50 (0.005%) and 0 of 8,399 on NIFTYBANK; the long and short entry stops are never pending together (the 675-bar average side separates them): resolved along the open-to-extreme path.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef {
        id: ID, name: "Neural DT 1 (tanh filter on six ATR-scaled inputs, first-40-minutes breakout entry, ATR stop / target / trail, 1-minute NIFTY / BANKNIFTY futures)", group: "TradeStation code: Neural DT (Breakout Futures)", session: 0,
        dir: "Long/Short", status: Status::Live, needs_volume: true, needs_vix: false, needs_series: &[], book: "ndt1",
        rules: &[
            "F|\"NNOut = tanh(w1*I1 + w2*I2 + w3*I3 + w4*I4 + w5*I5 + w6*I6)\" with I1r = volume - average(volume, 20), I2r = (C - C[2])/ATR, I3r = (C - AMA)/ATR, I4r = (AMA - Average(C, 81))/ATR, I5r = (C - DayMode)/ATR, I6r = (TrueRange - ATR)/ATR; ATR = Average(TrueRange, 20); AMA = AdaptiveMovAvg(C, 10, 2, 30); weights 1, 1, 0, -1, 1, 0",
            "E|\"If MarketPosition <> 1 and time < EntTime and EntriesToday(Date) < 1 and C > average(C, NAve) and NNOut >= 0.5 then Buy next bar at highest(H, 2) + 1 point stop\" (NAve 675, EntTime = session start + 40 min); short: \"C < average(C, NAve) and NNOut <= -0.5 then Sell short next bar at lowest(L, 2) stop\"",
            "X|\"TargPts = MaxList(1, TargFr * ATR); XPts1 = StopFr * ATR; LStop = EntH - XPts1\" (TargFr 5, StopFr 4.5); \"Sell next bar at entryprice + TargPts limit\"; \"if C - EntryPrice > FloorFr * ATR then LTrailOn = True; NewLStop = EntryPrice + TrailPr/100 * (C - EntryPrice); LStop = MaxList(LStop, NewLStop); Sell next bar at LStop stop\" (FloorFr 3.5, TrailPr 10); \"if time >= ExtTime then Sell next bar at market\" (ExtTime = EntTime + 10 min); SetExitOnClose",
        ],
        notes: NOTE, claim: None,
    }]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { (id == ID).then_some(("ndt1", "Neural DT 1 (Breakout Futures)", "1-minute NIFTY / BANKNIFTY futures")) }
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    (id == ID).then(|| vec![p("targ", "TargFr (fraction of ATR)", 5., 0.5, 10., 0.5), p("floor", "FloorFr", 3.5, 0.5, 10., 0.5), p("trail", "TrailPr (percent)", 10., 0., 100., 5.), p("stop", "StopFr", 4.5, 0.5, 10., 0.5),
        p("tment", "TmEnt (minutes from the open)", 40., 1., 120., 1.), p("tmext", "TmExt", 10., 1., 60., 1.), p("nave", "NAve", 675., 10., 2000., 1.), p("ref", "level the code's one-point amounts belong to (S&P about 1,500)", 1500., 100., 5000., 100.)])
}
pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if id != ID || to >= x.c.len() || from > to { return None; }
    let g = |k: &str| *p.get(k).unwrap_or(&params(id).unwrap().iter().find(|d| d.key == k).unwrap().def);
    let Some(fb) = fut_bars(x, from, to, 1) else { return Some(vec![]) };
    let (targ, floor, trail, stopf, tment, tmext, nave, rf) = (g("targ"), g("floor"), g("trail") / 100.0, g("stop"), g("tment").round() as i64, g("tmext").round() as i64, g("nave").round() as usize, g("ref"));
    let w = [1.0f64, 1.0, 0.0, -1.0, 1.0, 0.0];
    let n = fb.o.len();
    let ent_t = |m: i64| { let t = 9 * 60 + 15 + m; t / 60 * 100 + t % 60 };
    let (ent_time, ext_time) = (ent_t(tment), ent_t(tment + tmext));
    let (mut ama, mut imax, mut imin) = (0.0f64, [0.0f64; 6], [99999999.0f64; 6]);
    let mut cur: HashMap<i64, u32> = HashMap::new();
    let mut day_mode = f64::NAN;
    let (mut lstop, mut sstop, mut targ_pts, mut ltrail, mut strail) = (0.0f64, 0.0f64, 0.0f64, false, false);
    let mut tr = vec![0.0f64; n];
    let mut seeded = false;
    Some(run_fb(&fb, |k, st, now, next| {
        let (h, l, c) = (fb.h[k], fb.l[k], fb.c[k]);
        let pc = if k == 0 { c } else { fb.c[k - 1] };
        tr[k] = if k == 0 { h - l } else { h.max(pc) - l.min(pc) };
        if k > 0 && fb.date[k] != fb.date[k - 1] {
            let (mut best, mut bc) = (i64::MAX, 0u32);
            for (&pr, &cnt) in cur.iter() { if cnt > bc || (cnt == bc && pr < best) { bc = cnt; best = pr; } }
            day_mode = if bc > 0 { best as f64 / 20.0 } else { f64::NAN };
            cur.clear();
        }
        *cur.entry((c * 20.0).round() as i64).or_insert(0) += 1;
        // Kaufman AMA(10, 2, 30)
        if !seeded { ama = c; seeded = true; } else if k >= 10 {
            let ch = (c - fb.c[k - 10]).abs();
            let vol: f64 = (k - 9..=k).map(|j| (fb.c[j] - fb.c[j - 1]).abs()).sum();
            let er = if vol > 0.0 { ch / vol } else { 0.0 };
            let (fa, sl) = (2.0 / 3.0, 2.0 / 31.0);
            let sc = (er * (fa - sl) + sl).powi(2);
            ama += sc * (c - ama);
        } else { ama = c; }
        if k < 20 { return; }
        let atr: f64 = tr[k - 19..=k].iter().sum::<f64>() / 20.0;
        let vavg: f64 = fb.v[k - 19..=k].iter().sum::<f64>() / 20.0;
        let ma81 = if k >= 80 { fb.c[k - 80..=k].iter().sum::<f64>() / 81.0 } else { f64::NAN };
        let raw = [fb.v[k] - vavg, (c - fb.c[k - 2]) / atr, (c - ama) / atr, (ama - ma81) / atr, (c - day_mode) / atr, (tr[k] - atr) / atr];
        let mut s = 0.0;
        for i in 0..6 {
            imax[i] = imax[i].max(raw[i]); imin[i] = imin[i].min(raw[i]);
            let iv = if (imax[i] - imin[i]).abs() > 0.0 { (2.0 * raw[i] - (imax[i] + imin[i])) / (imax[i] - imin[i]) } else { raw[i] };
            s += w[i] * iv;
        }
        let nn = s.tanh();
        let t = hhmm(fb.stamp[k]);
        let entered_today = st.last_entry.map_or(false, |e| fb.date[e] == fb.date[k]);
        let pt = c / rf;
        if k >= nave && !day_mode.is_nan() && !ma81.is_nan() && t < ent_time && !entered_today {
            let avg_n = fb.c[k + 1 - nave..=k].iter().sum::<f64>() / nave as f64;
            if st.pos != 1 && c > avg_n && nn >= 0.5 {
                let ent = fb.h[k].max(fb.h[k - 1]) + pt;
                next.push(o(A::Buy, P::Stop(ent), "Buy"));
                targ_pts = (pt).max(targ * atr); lstop = ent - stopf * atr; ltrail = false;
            }
            if st.pos != -1 && c < avg_n && nn <= -0.5 {
                let ent = fb.l[k].min(fb.l[k - 1]);
                next.push(o(A::Sell, P::Stop(ent), "Sell short"));
                targ_pts = (pt).max(targ * atr); sstop = ent + stopf * atr; strail = false;
            }
        }
        if st.pos == 1 {
            next.push(o(A::Xl, P::Lim(st.ep + targ_pts), "Targ-L1"));
            if c - st.ep > floor * atr { ltrail = true; }
            let nl = if ltrail { st.ep + trail * (c - st.ep) } else { lstop };
            lstop = lstop.max(nl);
            next.push(o(A::Xl, P::Stop(lstop), "Stop-L2"));
            if t >= ext_time { next.push(o(A::Xl, P::Mkt, "Time-L3")); }
        }
        if st.pos == -1 {
            next.push(o(A::Xs, P::Lim(st.ep - targ_pts), "Targ-S1"));
            if st.ep - c > floor * atr { strail = true; }
            let ns = if strail { st.ep - trail * (st.ep - c) } else { sstop };
            sstop = sstop.min(ns);
            next.push(o(A::Xs, P::Stop(sstop), "Stop-S2"));
            if t >= ext_time { next.push(o(A::Xs, P::Mkt, "Time-S3")); }
        }
        if fb.last[k] && st.pos != 0 { now.push(o(if st.pos > 0 { A::Xl } else { A::Xs }, P::Mkt, "end of day")); }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn defaults() {
        assert_eq!(params(ID).unwrap().iter().map(|p| p.def).collect::<Vec<_>>(), vec![5., 3.5, 10., 4.5, 40., 10., 675., 1500.]);
        assert!(params("x").is_none());
    }
}

