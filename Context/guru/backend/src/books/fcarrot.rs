//! 'Forex Carrot Scalper System for TS' (TradeStation EasyLanguage code, Code.rar; research share
//! USB HDD 1/TradeStation Systems & Indicators/Forex Carrot Scalper System for TS). Book id `fcarrot`.
//! 5/9 moving-average cross filtered by RSI(14) with a stepping trailing stop, written for a currency pair; run here on real
//! 1-minute bars aggregated to 5 minutes. Lane K4 (25 Nov 2026, R4 #398 R1057).
//! Independent pandas reference: authors/code-1123/ref/fcarrot_ref.py.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};
use crate::intraday::{fill_px, touched, Bar1, Level};

pub const BOOK: &str = "fcarrot";
const ID: &str = "fcarrot_scalper";

const NOTE: &str = "[Declared readings] Forex Carrot Scalper (TradeStation code, read in full). Fast 5 and slow 9 simple averages of the price, Wilder RSI(14). Long signal: fast crosses above slow (fast > slow and fast[1] <= slow[1]) with RSI > 50 and RSI > RSI[1]; short mirrored (RSI < 50 and falling). Entry or reversal at the next bar's open; a cross whose RSI test fails closes the opposite position at the next bar's open. After every bar close in a position the stop is set at entryLevel - risk (long) / entryLevel + risk (short), where entryLevel is the entry price and steps by 'move' once the bar's close (long, Bid) or close + spread (short, Ask) is 'target' beyond it; the stop is live from the next bar, so the entry bar itself cannot be stopped. A passing signal resets entryLevel to the entry price (printed). A position is closed at the session end. Four labelled unstated details, each its own: (1) bars are 5 minutes, measured from the session's first minute (the code names no timeframe); (2) the averages and RSI run inside the session with no overnight carry (RSI Wilder, first value from the mean of the first 14 changes), so the first signal is the candle with 15 closes; (3) the printed distances are in price units of a currency pair (risk 0.0015, target 0.0010, step 0.0010, spread 0.0005) and are scaled to percentages of the price at a 1.30 reference (0.115%, 0.077%, 0.077%, 0.038%); (4) the session 9:01-16:06 is the NSE session and the position closes at the last candle's close (the code closes at the close of the first bar after the session). Not counted: the printed Spread shifts the averages and the RSI by a constant, which changes no cross and no RSI (it is dropped there) but is kept in the short trailing trigger (Ask); a signal while in a position in the same direction only resets entryLevel; at most one position; a market order at the next open replaces the stop of that bar; no two live levels exist in any minute (one stop, no target), so the minute-order ambiguity share is 0.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: ID, name: "Forex Carrot Scalper: 5/9 average cross with RSI(14) filter, stepping trailing stop", group: "Carrot · Forex Carrot Scalper (TradeStation)", session: 0,
        dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
        rules: &[
            "F|\"If ( thisFast>thisSlow and thisFast[1]<=thisSlow[1] ) ... If ( thisRSI>canBuy and thisRSI>thisRSI[1] )\"  (and mirrored: thisFast<thisSlow and thisFast[1]>=thisSlow[1], thisRSI<canSell and thisRSI<thisRSI[1]); FastMovAvg(5), SlowMovAvg(9), RSIPeriod(14), CanBuy(50), CanSell(50)",
            "E|\"Buy (LE) next bar at Market ... Buy (Reverse LE) next bar at Market ... SellShort (SE) next bar at Market ... SellShort (Reverse SE) next bar at Market\"",
            "X|\"Sell (Trailing LX) next bar at entryLevel-RiskPoints Stop ... If ( BidPrice>=entryLevel+TargetPoints ) then entryLevel=entryLevel+MovePoints ... else BuyToCover (SX) / Sell (LX) next bar at Market ... EOD exit this bar at Close\""],
        notes: NOTE, claim: None }]
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    if id != ID { return None; }
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(vec![p("fast", "Fast average (printed FastMovAvg = 5)", 5., 2., 20., 1.),
              p("slow", "Slow average (printed SlowMovAvg = 9)", 9., 3., 40., 1.),
              p("rsi", "RSI period (printed RSIPeriod = 14)", 14., 5., 30., 1.),
              p("lvl", "RSI level for both sides (printed CanBuy = CanSell = 50)", 50., 30., 70., 5.),
              p("risk", "Stop distance, % of the entry price (printed RiskPoints 0.0015 = 0.115% at the 1.30 reference)", 0.115, 0.05, 0.5, 0.005),
              p("tgt", "Trail trigger, % of the entry price (printed TargetPoints 0.0010 = 0.077%)", 0.077, 0.03, 0.3, 0.005),
              p("mv", "Trail step, % of the entry price (printed MovePoints 0.0010 = 0.077%)", 0.077, 0.03, 0.3, 0.005),
              p("spr", "Spread in the short trail trigger, % of the price (printed Spread 0.0005 = 0.038%)", 0.038, 0., 0.2, 0.005)])
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == ID).then_some(("fcarrot_scalper", "Forex Carrot Scalper", "5/9 average cross with RSI filter, stepping trailing stop"))
}

const BAR: u16 = 5;

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if id != ID { return None; }
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d);
    let (nf, ns, nr, lvl) = (g("fast", 5.) as usize, g("slow", 9.) as usize, g("rsi", 14.) as usize, g("lvl", 50.));
    let (risk, tgt, mv, spr) = (g("risk", 0.115) / 100., g("tgt", 0.077) / 100., g("mv", 0.077) / 100., g("spr", 0.038) / 100.);
    let mut out = Vec::new();
    for j in from..=to.min(x.r.len().saturating_sub(1)) {
        let Some(b) = crate::intraday_w2b::day(x, j) else { continue };
        let Some(first) = b.first() else { continue };
        let m0 = first.m;
        // candles: (first minute index, last minute index, close)
        let mut cs: Vec<(usize, usize, f64)> = vec![];
        let mut kk: Vec<u16> = vec![];
        for (ix, z) in b.iter().enumerate() {
            let k = (z.m - m0) / BAR;
            if kk.last() == Some(&k) { let c = cs.last_mut().unwrap(); c.1 = ix; c.2 = z.c; } else { kk.push(k); cs.push((ix, ix, z.c)); }
        }
        let n = cs.len();
        let cl: Vec<f64> = cs.iter().map(|c| c.2).collect();
        // averages compared in exact integer arithmetic on prices in hundredths (a cross is a strict comparison and flat closes tie exactly)
        let ic: Vec<i64> = cl.iter().map(|v| (v * 100.).round() as i64).collect();
        let sum = |len: usize, i: usize| -> Option<i64> { (i + 1 >= len).then(|| ic[i + 1 - len..=i].iter().sum::<i64>()) };
        // Wilder RSI: first value at index nr from the mean of the first nr changes
        let mut rsi = vec![f64::NAN; n];
        if n > nr {
            let (mut ag, mut al) = (0., 0.);
            for k in 1..=nr { let d = cl[k] - cl[k - 1]; ag += d.max(0.); al += (-d).max(0.); }
            ag /= nr as f64; al /= nr as f64;
            let val = |ag: f64, al: f64| if al == 0. { if ag == 0. { 50. } else { 100. } } else { 100. - 100. / (1. + ag / al) };
            rsi[nr] = val(ag, al);
            for k in nr + 1..n { let d = cl[k] - cl[k - 1]; ag = (ag * (nr as f64 - 1.) + d.max(0.)) / nr as f64; al = (al * (nr as f64 - 1.) + (-d).max(0.)) / nr as f64; rsi[k] = val(ag, al); }
        }
        // position: (dir, entry price, entryLevel (0 = unset), live stop for the next candle)
        let mut pos: Option<(f64, f64, f64, Option<f64>)> = None;
        // pending market action from the previous close: 0 none, 1 buy (enter / reverse), -1 sell short (enter / reverse), 2 cover shorts, -2 sell longs
        let mut pend: i8 = 0;
        let push = |out: &mut Vec<Trade>, dir: f64, entry: f64, xp: f64, why: &str| out.push(Trade { dir, legs: vec![Leg { i: j, px: entry, w: 1.0 }], exit_i: j, exit_px: xp, reason: why.into(), entry_kind: "open", signal_i: j, open: false, exits: vec![] });
        for i in 0..n {
            let (s, e, c) = cs[i];
            let mut market = false;
            if pend != 0 {
                let o = b[s].o;
                match (pend, pos) {
                    (1, Some((d, en, ..))) if d < 0. => { push(&mut out, d, en, o, "reverse"); pos = Some((1., o, 0., None)); market = true; }
                    (1, None) => { pos = Some((1., o, 0., None)); market = true; }
                    (-1, Some((d, en, ..))) if d > 0. => { push(&mut out, d, en, o, "reverse"); pos = Some((-1., o, 0., None)); market = true; }
                    (-1, None) => { pos = Some((-1., o, 0., None)); market = true; }
                    (2, Some((d, en, ..))) if d < 0. => { push(&mut out, d, en, o, "signal exit"); pos = None; market = true; }
                    (-2, Some((d, en, ..))) if d > 0. => { push(&mut out, d, en, o, "signal exit"); pos = None; market = true; }
                    _ => {}
                }
                pend = 0;
            }
            // stop placed at the previous close, live through this candle's minutes (void if a market order filled at the open)
            if !market {
                if let Some((d, en, el, Some(sp))) = pos {
                    let lv = Level { up: d < 0., px: sp };
                    if let Some(ix) = (s..=e).find(|&ix| touched(&b[ix], lv)) {
                        push(&mut out, d, en, fill_px(&b[ix], lv), "trailing stop"); pos = None; let _ = el;
                    }
                }
            }
            if let Some((d, en, el, _)) = pos {
                if i == n - 1 { push(&mut out, d, en, b[e].c, "day end"); pos = None; continue; }
                // trailing block at this close
                let mut el = if el == 0. { en } else { el };
                if d > 0. { if c >= el + tgt * en { el += mv * en; } } else if c * (1. + spr) <= el - tgt * en { el -= mv * en; }
                pos = Some((d, en, el, Some(if d > 0. { el - risk * en } else { el + risk * en })));
            }
            if i == n - 1 { continue; }
            // signals at this close
            if i >= 1 && i >= ns.max(nf) && !rsi[i].is_nan() && !rsi[i - 1].is_nan() {
                let (Some(f0), Some(s0), Some(f1), Some(s1)) = (sum(nf, i), sum(ns, i), sum(nf, i - 1), sum(ns, i - 1)) else { continue };
                // fast > slow  <=>  sum_fast / nf > sum_slow / ns
                let (a0, a1) = (f0 * ns as i64 - s0 * nf as i64, f1 * ns as i64 - s1 * nf as i64);
                if a0 > 0 && a1 <= 0 {
                    if rsi[i] > lvl && rsi[i] > rsi[i - 1] { pend = 1; if let Some(q) = pos.as_mut() { q.2 = 0.; } } else { pend = 2; }
                } else if a0 < 0 && a1 >= 0 {
                    if rsi[i] < lvl && rsi[i] < rsi[i - 1] { pend = -1; if let Some(q) = pos.as_mut() { q.2 = 0.; } } else { pend = -2; }
                }
            }
        }
    }
    Some(out)
}
