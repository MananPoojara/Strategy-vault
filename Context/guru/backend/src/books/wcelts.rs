//! Win Consulting, "Easy Language e costruzione di Trading Systems" (Italian TradeStation course; research share USB HDD 1/
//! TradeStation/_Knowleage/Win Consulting - Easy Language e costruzione di Trading Systems (Italian)/TRADESTATION - EasyLanguage -
//! Costruzione di Trading System I.pdf, ch.7 worked example, PDF p.138-160 = printed 141-163; ocr2 text authors/batch-e/E1w/
//! wincons_ch7_ma3.txt). Book id `wcelts`. Lane K10 (titles loop, 1 Nov 2026), to-code row 175; independent check
//! authors/titles-1030/K10w/k10_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::msws::sma;
use super::rbtie::mk;

/// Admitted ids (authors/titles-1030/K10w/k10_ref.py (header)).
pub const REVIEWED: &[&str] = &["wcelts_ma3"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] TradeStation semantics: orders are placed at a bar's close for the next bar. Entries: buy next bar at market (the open) when the 10-, 40- and 100-day simple averages are each higher than one bar earlier, sell short when all three are lower; a short is reversed by the buy and a long by the short sale. The five exits work together: (1) moving-average exit, next bar at market when one or none of the three averages is still rising (falling for shorts); (2) YO-YO exit, a stop at close - 2 x ATR(10) for the next bar; (3) ATR protective stop at entry price - 3 x ATR(10); (4) ATR breakeven: once the highest close since entry (entry day included) is above entry price + 4 x ATR(10), a stop at the entry price; (5) ATR trailing stop at the highest high since entry - 4 x ATR(10); shorts mirrored. ATR(10) = TradeStation AvgTrueRange, the simple average of the last 10 true ranges, today included, taken on the bar that places the order. The highest of the active long stops (lowest for shorts) is the one hit first; a gap through it fills at the open. A new position gets its first stops at the close of its entry day. The 5-bar time exit (p.142) was a test step and the volume filter (p.159, code not printed) is not coded. Printed (p.160): MIB30 futures (FIBPEP) daily Mar 1996 - Sep 2000, 30 trades, 40% profitable, profit factor 2.31, net $18,824.53 (not in the vault's data). Indices, ETFs and US/India stocks are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "wcelts_ma3", name: "Three rising (falling) averages 10/40/100 with five ATR and average exits, next-bar orders (Win Consulting)", group: "EasyLanguage course ch.7 worked example",
        session: 0, dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "wcelts", rules: &[
            "E|\"Inputs: L1(10), L2(40), L3(100); Condition1= Average(Close, L1) > Average(Close, L1)[1]; ... If Condition1 And Condition2 And Condition3 Then Buy Next Bar At Market; If Condition4 And Condition5 And Condition6 Then Sell Next Bar At Market;\" (PDF p.142)",
            "X|\"If Contatorelong <= 1 Then Exitlong Next Bar At Market; If Contatoreshort <= 1 Then Exitshort Next Bar At Market;\" (p.150-151); \"ExitLong next bar at Close - 2 * AvgTrueRange(10) Stop;\" (YO-YO, p.152)",
            "X|\"ATRVal = AvgTrueRange(10) * 3; If MarketPosition = 1 Then ExitLong Next Bar at EntryPrice - ATRVal Stop;\" (p.153); \"ATRVal = AvgTrueRange(10) * 4; If BarsSinceEntry = 0 Then PosHL = Close; ... If PosHL > EntryPrice + ATRVal Then ExitLong Next Bar at EntryPrice Stop;\" (p.154)",
            "X|\"ExitLong next bar at MaxTradeHigh - 4 * AvgTrueRange(10) Stop; ExitShort next bar at MinTradeLow + 4 * AvgTrueRange(10) Stop;\" (p.155-156)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == "wcelts_ma3").then_some(("wcelts_ma3", "Win Consulting three averages", "10/40/100, five exits"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> { reviewed(id).then(Vec::new) }

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, _p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let (o, h, l, c, n) = (&x.o, &x.h, &x.l, &x.c, x.c.len());
    let ma = [sma(c, 10), sma(c, 40), sma(c, 100)];
    let tr: Vec<f64> = (0..n).map(|i| if i == 0 { h[0] - l[0] } else { h[i].max(c[i - 1]) - l[i].min(c[i - 1]) }).collect();
    let atr = sma(&tr, 10);
    let cnt = |i: usize, d: f64| if i == 0 { 0 } else { ma.iter().filter(|m| d * (m[i] - m[i - 1]) > 0.).count() };
    let mut out = vec![];
    let mut pos: Option<(f64, usize, f64, f64, f64)> = None; // dir, entry bar, entry price, best close, best high/low
    let mut pend: Option<f64> = None; // market order for the next open: +1 / -1 entry (reversing), 0 exit
    let mut stop = f64::NAN;
    for i in from..=to {
        if let Some(a) = pend.take() {
            if let Some((d, e, ep, _, _)) = pos { if a != d { out.push(mk(d, e, ep, "open", i, o[i], if a == 0. { "averages turned" } else { "reversed" }, false, i - 1)); pos = None; } }
            if a != 0. && pos.is_none() { pos = Some((a, i, o[i], f64::NAN, f64::NAN)); }
        } else if let Some((d, e, ep, _, _)) = pos {
            if stop.is_finite() && (if d > 0. { l[i] <= stop } else { h[i] >= stop }) {
                out.push(mk(d, e, ep, "open", i, if d > 0. { o[i].min(stop) } else { o[i].max(stop) }, "ATR stop", false, i - 1));
                pos = None;
            }
        }
        stop = f64::NAN;
        if let Some((d, e, ep, bc, bx)) = pos.as_mut() {
            let first = *e == i;
            *bc = if first { c[i] } else if *d > 0. { bc.max(c[i]) } else { bc.min(c[i]) };
            *bx = if first { if *d > 0. { h[i] } else { l[i] } } else if *d > 0. { bx.max(h[i]) } else { bx.min(l[i]) };
            let _ = ep;
        }
        if i == to { break; }
        let (up, dn) = (cnt(i, 1.), cnt(i, -1.));
        match pos {
            None => { if up == 3 { pend = Some(1.); } else if dn == 3 { pend = Some(-1.); } }
            Some((d, _, ep, bc, bx)) => {
                let (same, opp) = if d > 0. { (up, dn) } else { (dn, up) };
                if opp == 3 { pend = Some(-d); }
                else if same <= 1 { pend = Some(0.); }
                else {
                    let a = atr[i];
                    let mut lv = vec![c[i] - d * 2. * a, ep - d * 3. * a, bx - d * 4. * a];
                    if d * (bc - (ep + d * 4. * a)) > 0. { lv.push(ep); }
                    stop = lv.into_iter().filter(|v| v.is_finite()).fold(f64::NAN, |s, v| if s.is_nan() || d * (v - s) > 0. { v } else { s });
                }
            }
        }
    }
    if let Some((d, e, ep, _, _)) = pos { out.push(mk(d, e, ep, "open", to, c[to], "open", true, to)); }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn three_averages_with_market_and_stop_exits() {
        let r: Vec<Row> = (0..1500).map(|i| { let f = i as f64; let c = 100. + 30. * (f / 120.).sin() + 3. * (f / 7.).cos();
            Row { ticker: "T".into(), date: format!("{:05}", i), open: c - 0.5, high: c + 1.5, low: c - 1.5, close: c, volume: None } }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let t = run("wcelts_ma3", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() > 4 && t.iter().any(|z| z.dir > 0.) && t.iter().any(|z| z.dir < 0.));
        for z in &t { assert_eq!(z.legs[0].px, x.o[z.legs[0].i]); }
        for w in t.windows(2) { assert!(w[1].legs[0].i >= w[0].exit_i); }
        assert!(run("wcelts_x", &x, 0, 5, &Params::new()).is_none());
    }
}

// Instrument admission: Win Consulting, Easy Language e costruzione di Trading Systems 2026-11-01 (titles lane K10).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "wcelts" && reviewed(def.id),
    label: "Win Consulting tested this course example on MIB30 futures and Italian stocks. Runs on indices, ETFs and stocks, US and India, are a declared application.",
    basis: "Win Consulting, Easy Language e costruzione di Trading Systems, ch.7 PDF p.138-160 (printed 141-163); independent check authors/titles-1030/K10w/k10_ref.py",
};
