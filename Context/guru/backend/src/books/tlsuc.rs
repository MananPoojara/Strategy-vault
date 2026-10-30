//! "Success System" - an anonymous trader's post to a TradeStation mailing list, saved as SuccessSystem.doc with its
//! TradeStation archive SYSTEM1.ELA and report System1.dat (research drive USB HDD 1/TradeStation Systems & Indicators/
//! _Misc/Success System (For TS)/; read-only). Book id `tlsuc`, lane K1 30 Oct 2026. The post prints the rules as
//! "Uncoded version" items 1-8 (the .ELA is a compressed archive the vault cannot open). Its report: FTSE futures
//! 05/03/84-11/09/98, 251 trades, 63% profitable, long only, one contract. Item 8 (a $3,500 money-management stop on one
//! FTSE contract) has no price equivalent on an index, ETF or share and is left out (stated on the card).
use crate::books::apta::ema_first;
use crate::connors::{rsi, sma, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};
/// Admitted ids (authors/tlsuc/admission/succ_ref.py (header)).
pub const REVIEWED: &[&str] = &["tlsuc_success"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }
const ID: &str = "tlsuc_success";

const NOTE: &str = "[Declared readings] Signals on the daily close. 36-day EMA of the close started from the first close (TradeStation XAverage); 5-day RSI = Wilder's; 'highest 5-day RSI over the last 9 days' includes today. Item 4: a buy stop for the next day only at today's high, filled at the high or at the open if the day opens above it; set-ups are acted on only while flat. Items 6-7: from the day after entry, each close sets the next day's sell stop at the close minus 1.75 x the 4-day simple average of the true range, or at the close minus 0.5 x that average when the close is more than 0.5 x the average above the entry price (both orders are live; the higher one is the binding stop); a stop fills at its level or at the open if the day opens below it. Item 5: otherwise out at the close of the 6th bar after the entry bar. [Left out] Item 8, the $3,500 money-management stop: one FTSE contract in dollars has no price equivalent here. One unit, long only; a position still open is marked at the last close. The author's report is on FTSE futures (no FTSE data in the vault); indices, ETFs and US/India stocks are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: ID, name: "Success System: high above the 36-day EMA after a 5-day RSI drop of 30 points from a 9-day peak above 70; buy stop at the high, ATR stops, out on the 6th day",
        group: "Success System (TradeStation list)", session: 0, dir: "Long", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "tlsuc", rules: &[
            "F|\"1. High today must be > 36 day EMA.(robust for 20 to 60day EMA)\"",
            "F|\"2. Highest 5 day RSI over last 9 days must be greater then 70\"",
            "F|\"3. Today's 5 day RSI must be more than 30 point below #2\"",
            "E|\"4. If above 3 condition fulfilled, buy tomorrow at today's high on a stop.\"",
            "X|\"5. If still in the trade, exit on the close of the 6th bar after entry.\"",
            "X|\"6. Exit tomorrow on a stop at 175% of 4 day SMA of the truerange below today's close.\"",
            "X|\"7. If todays close is greater than the entry price plus 50% of the 4 day SMA of the truerange, place a stop to exit tomorrow at today's close minus 50% of the 4 day average truerange.\"",
            "F|\"8. Moneymanagement stop of $3500.\" - left out: a dollar amount on one FTSE contract",
            "F|Author's report System1.dat: FTSE futures 05/03/84-11/09/98, 251 trades, 63% profitable, net $62,775 on one contract, long only"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == ID).then_some((ID, "Success System", "EMA 36, RSI 5, 9-day peak > 70, 30-point drop"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    (id == ID).then(|| vec![
        p("ema", "EMA length (36; the post: robust for 20 to 60)", 36., 20., 60., 1.), p("rsi_n", "RSI length (5)", 5., 2., 14., 1.),
        p("look", "Days for the highest RSI (9)", 9., 3., 20., 1.), p("peak", "Highest RSI above (70)", 70., 50., 90., 1.),
        p("drop", "RSI points below the peak (30)", 30., 10., 50., 1.), p("hold", "Exit on the close of bar N after entry (6)", 6., 1., 20., 1.),
        p("atr_n", "True-range average length (4)", 4., 2., 20., 1.), p("stop_k", "Stop: true-range averages below the close (1.75)", 1.75, 0.5, 4., 0.25),
        p("trail_k", "Profit stop: averages in profit / below the close (0.5)", 0.5, 0.1, 2., 0.1)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if id != ID || from >= to || to >= x.r.len() { return None; }
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let u = |k: &str| g(k).round().max(1.0) as usize;
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let (e, r) = (ema_first(c, u("ema")), rsi(c, u("rsi_n")));
    let trs: Vec<f64> = (0..c.len()).map(|i| if i == 0 { f64::NAN } else { (h[i] - l[i]).max((h[i] - c[i - 1]).abs()).max((l[i] - c[i - 1]).abs()) }).collect();
    let atr = sma(&trs, u("atr_n"));
    let (look, peak, drop, hold, k1, k2) = (u("look"), g("peak"), g("drop"), u("hold"), g("stop_k"), g("trail_k"));
    let setup = |i: usize| -> bool {
        if i + 1 < look { return false; }
        let hr = r[i + 1 - look..=i].iter().copied().fold(f64::NAN, f64::max);
        h[i] > e[i] && hr > peak && r[i] < hr - drop
    };
    let (mut out, mut pos) = (vec![], None::<(usize, f64)>);
    let mut t = from;
    while t <= to {
        if let Some((ei, ep)) = pos {
            // stop for bar t from the close of bar t-1 (from the day after entry)
            let (pc, a) = (c[t - 1], atr[t - 1]);
            let mut stop = pc - k1 * a;
            if pc > ep + k2 * a { stop = stop.max(pc - k2 * a); }
            if stop.is_finite() && l[t] <= stop {
                let px = o[t].min(stop);
                out.push(Trade { dir: 1., legs: vec![Leg { i: ei, px: ep, w: 1. }], exit_i: t, exit_px: px, reason: "True-range stop".into(), entry_kind: "stop", signal_i: t - 1, open: false, exits: vec![] });
                pos = None;
            } else if t == ei + hold {
                out.push(Trade { dir: 1., legs: vec![Leg { i: ei, px: ep, w: 1. }], exit_i: t, exit_px: c[t], reason: format!("Close of bar {hold} after entry"), entry_kind: "stop", signal_i: t, open: false, exits: vec![] });
                pos = None;
            }
        }
        if pos.is_none() && t < to && setup(t) && h[t + 1] >= h[t] {
            pos = Some((t + 1, o[t + 1].max(h[t])));
            t += 2; // the entry day itself: no protective stop yet (the first one is set at its close), no new set-up
            continue;
        }
        t += 1;
    }
    if let Some((ei, ep)) = pos {
        out.push(Trade { dir: 1., legs: vec![Leg { i: ei, px: ep, w: 1. }], exit_i: to, exit_px: c[to], reason: "open".into(), entry_kind: "stop", signal_i: to, open: true, exits: vec![] });
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn stop_entry_time_exit_and_stops() {
        assert_eq!(params(ID).unwrap()[0].def, 36.);
        // rally, sharp 3-day drop (RSI falls > 30 from a peak > 70) while the high stays above the EMA, then a bounce
        let mut v: Vec<f64> = (0..60).map(|i| 100. + i as f64).collect();
        v.extend([150., 147., 158., 159., 160., 161., 162., 163., 164., 165., 166.]);
        let r: Vec<Row> = v.iter().enumerate().map(|(i, &c)| Row { ticker: "SPX".into(), date: format!("2000-01-{:02}", i % 28 + 1), open: c - 0.5, high: c + 1., low: c - 1., close: c, volume: None }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let t = run(ID, &x, 10, r.len() - 1, &Params::new()).unwrap();
        assert!(!t.is_empty());
        let a = &t[0];
        assert!(a.legs[0].px >= x.h[a.legs[0].i - 1] - 1e-9 && a.exit_i <= a.legs[0].i + 6);
    }
}
