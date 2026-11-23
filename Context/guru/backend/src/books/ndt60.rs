//! Neural DT 60 (Mike Bryant, Breakout Futures, 2007), TradeStation: research share `USB HDD 1/TradeStation Systems & Indicators/Neural DT System for Tradestation
//! (breakoutfutures.com)/.../Code.rar::NeuralDT60.txt` (read-only copy read in full). Book id `ndt60`. Lane K4 (Q385, 24 Nov 2026): the 60-minute system (the file name) on the
//! NIFTY 50 / NIFTYBANK futures 1-minute series as a labelled R4 #340 adaptation, one scaling rule R4 #380. Helper books::k4x. Independent check authors/code-1123/ref/ndt60_ref.py.
use super::k4x::{fut_bars, hhmm, o, run_fb, A, P};
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use std::collections::HashMap;

const ID: &str = "ndt60_neural_dt60";
const NOTE: &str = "[Declared adaptation, R4 #340 and R4 #382] Neural DT 60 (Breakout Futures 2007), the 60-minute twin of Neural DT 1 (ndt1_neural_dt1) with its own printed inputs. 60-minute bars; ATR = Average(TrueRange,81); AMA = AdaptiveMovAvg(C,10,2,30); six raw inputs I1r = (C - C[1])/ATR, I2r = (C - C[2])/ATR, I3r = (C - AMA)/ATR, I4r = (AMA - Average(C,81))/ATR, I5r = (C - DayMode)/ATR, I6r = (TrueRange - ATR)/ATR, each scaled to [-1,1] by its running maximum and minimum (initial max 0, min 99999999 as printed); NNOut = tanh(w1 I1 + ... + w6 I6) with the printed weights -0.5, -1, 1, -1, 1, -1. Long when NNOut >= 0.5, MarketPosition <> 1, no entry today and EntTime1 (session start + 60 min) <= time < EntTime2 (session start + 300 min): Buy next bar at Highest(High,2) stop; stop = Highest(High,2) - StopFr (1.5) x ATR; short when NNOut <= -0.5: Sell short next bar at Lowest(Low,2) stop, stop = Lowest(Low,2) + 1.5 x ATR. There is no profit target and no time exit: once the close is FloorFr (1) x ATR beyond the entry the stop is the better of itself and the entry plus TrailPr (25%) of the open profit; exit at the session close. Labelled readings (3): (1) SUBSTITUTE, NOT THE AUTHOR'S FUNCTION: PriceMode(DayH,DayL,DayC,DayMode) is a proprietary function that is not in the file (its body and its 'mode' definition are unknown); the vault substitutes DayMode = the most frequent 60-minute close (rounded to the 0.05 tick, ties to the lower price) of the previous session, which feeds input I5 with the printed weight 1; (2) every indicator runs continuously over the 60-minute futures series from its first bar (AMA seeded with the first close, the running input maxima and minima start at the first bar, no entry before 81 bars); (3) the session clock is the exchange clock (session opens 09:15, EntTime1 10:15, EntTime2 14:15), compared on the bar's END time stamp (TradeStation stamps a bar with its close time, fb.stamp = the bucket end): the first 60-minute bar (09:15-10:15, stamp 10:15) is the first bar allowed to signal and the bar ending 14:15 is the first excluded, which is exactly the code's own 'time >= EntTime1 and time < EntTime2'. The code has no fixed point amounts, so no scaling rule applies. No result is printed.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef {
        id: ID, name: "Neural DT 60 (tanh filter on six ATR-scaled inputs, breakout entry between 60 and 300 minutes after the open, ATR stop with percent trail, 60-minute NIFTY / BANKNIFTY futures)", group: "TradeStation code: Neural DT (Breakout Futures)", session: 0,
        dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "ndt60",
        rules: &[
            "F|\"NNOut = tanh(w1*I1 + w2*I2 + w3*I3 + w4*I4 + w5*I5 + w6*I6)\" with I1r = (C - C[1])/ATR, I2r = (C - C[2])/ATR, I3r = (C - AMA)/ATR, I4r = (AMA - Average(C, 81))/ATR, I5r = (C - DayMode)/ATR, I6r = (TrueRange - ATR)/ATR; ATR = Average(TrueRange, 81); AMA = AdaptiveMovAvg(C, 10, 2, 30); weights -0.5, -1, 1, -1, 1, -1",
            "E|\"If time >= EntTime1 and time < EntTime2 and NNOut >= 0.5 and MarketPosition <> 1 and EntriesToday(Date) < 1 then Buy next bar at highest(H, 2) stop\" (EntTime1 = session start + 60 min, EntTime2 = session start + 300 min); short: \"NNOut <= -0.5 ... Sell short next bar at lowest(L, 2) stop\"",
            "X|\"LStop = highest(H, 2) - StopFr * ATR\" (StopFr 1.5); \"if C - EntryPrice > FloorFr * ATR then LTrailOn = True; NewLStop = EntryPrice + TrailPr/100 * (C - EntryPrice); LStop = MaxList(LStop, NewLStop); Sell next bar at LStop stop\" (FloorFr 1, TrailPr 25); no target, no time exit; SetExitOnClose",
        ],
        notes: NOTE, claim: None,
    }]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { (id == ID).then_some(("ndt60", "Neural DT 60 (Breakout Futures)", "60-minute NIFTY / BANKNIFTY futures")) }
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    (id == ID).then(|| vec![p("floor", "FloorFr", 1., 0.5, 10., 0.5), p("trail", "TrailPr (percent)", 25., 0., 100., 5.), p("stop", "StopFr", 1.5, 0.5, 10., 0.5), p("atr", "ATR length", 81., 5., 200., 1.),
        p("t1", "EntTime1 (minutes after the open)", 60., 1., 400., 1.), p("t2", "EntTime2 (minutes after the open)", 300., 1., 400., 1.)])
}
pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if id != ID || to >= x.c.len() || from > to { return None; }
    let g = |k: &str| *p.get(k).unwrap_or(&params(id).unwrap().iter().find(|d| d.key == k).unwrap().def);
    let Some(fb) = fut_bars(x, from, to, 60) else { return Some(vec![]) };
    let (floor, trail, stopf, na) = (g("floor"), g("trail") / 100.0, g("stop"), g("atr").round() as usize);
    let w = [-0.5f64, -1.0, 1.0, -1.0, 1.0, -1.0];
    let n = fb.o.len();
    let clock = |m: i64| { let t = 9 * 60 + 15 + m; t / 60 * 100 + t % 60 };
    let (t1, t2) = (clock(g("t1").round() as i64), clock(g("t2").round() as i64));
    let (mut ama, mut imax, mut imin) = (0.0f64, [0.0f64; 6], [99999999.0f64; 6]);
    let mut cur: HashMap<i64, u32> = HashMap::new();
    let mut day_mode = f64::NAN;
    let (mut lstop, mut sstop, mut ltrail, mut strail) = (0.0f64, 0.0f64, false, false);
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
        if !seeded { ama = c; seeded = true; } else if k >= 10 {
            let ch = (c - fb.c[k - 10]).abs();
            let vol: f64 = (k - 9..=k).map(|j| (fb.c[j] - fb.c[j - 1]).abs()).sum();
            let er = if vol > 0.0 { ch / vol } else { 0.0 };
            let sc = (er * (2.0 / 3.0 - 2.0 / 31.0) + 2.0 / 31.0).powi(2);
            ama += sc * (c - ama);
        } else { ama = c; }
        if k + 1 < na || k < 80 { return; }
        let atr: f64 = tr[k + 1 - na..=k].iter().sum::<f64>() / na as f64;
        let ma81: f64 = fb.c[k - 80..=k].iter().sum::<f64>() / 81.0;
        let raw = [(c - fb.c[k - 1]) / atr, (c - fb.c[k - 2]) / atr, (c - ama) / atr, (ama - ma81) / atr, (c - day_mode) / atr, (tr[k] - atr) / atr];
        let mut s = 0.0;
        for i in 0..6 {
            imax[i] = imax[i].max(raw[i]); imin[i] = imin[i].min(raw[i]);
            let iv = if (imax[i] - imin[i]).abs() > 0.0 { (2.0 * raw[i] - (imax[i] + imin[i])) / (imax[i] - imin[i]) } else { raw[i] };
            s += w[i] * iv;
        }
        let nn = s.tanh();
        let t = hhmm(fb.stamp[k]);
        let entered_today = st.last_entry.map_or(false, |e| fb.date[e] == fb.date[k]);
        if !day_mode.is_nan() && t >= t1 && t < t2 && !entered_today {
            if st.pos != 1 && nn >= 0.5 {
                let ent = fb.h[k].max(fb.h[k - 1]);
                next.push(o(A::Buy, P::Stop(ent), "Buy"));
                lstop = ent - stopf * atr; ltrail = false;
            }
            if st.pos != -1 && nn <= -0.5 {
                let ent = fb.l[k].min(fb.l[k - 1]);
                next.push(o(A::Sell, P::Stop(ent), "Sell short"));
                sstop = ent + stopf * atr; strail = false;
            }
        }
        if st.pos == 1 {
            if c - st.ep > floor * atr { ltrail = true; }
            let nl = if ltrail { st.ep + trail * (c - st.ep) } else { lstop };
            lstop = lstop.max(nl);
            next.push(o(A::Xl, P::Stop(lstop), "Stop-L1"));
        }
        if st.pos == -1 {
            if st.ep - c > floor * atr { strail = true; }
            let ns = if strail { st.ep - trail * (st.ep - c) } else { sstop };
            sstop = sstop.min(ns);
            next.push(o(A::Xs, P::Stop(sstop), "Stop-S1"));
        }
        if fb.last[k] && st.pos != 0 { now.push(o(if st.pos > 0 { A::Xl } else { A::Xs }, P::Mkt, "end of day")); }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn defaults() {
        assert_eq!(params(ID).unwrap().iter().map(|p| p.def).collect::<Vec<_>>(), vec![1., 25., 1.5, 81., 60., 300.]);
        assert!(params("x").is_none());
    }
}
