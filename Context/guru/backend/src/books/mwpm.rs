//! "Profit Maximiser System" (Most Wanted Forex Robot, mostwantedforexrobot.com; research share Trading Knowledge III/Profit Maximiser System (mostwantedforexrobot.com)/Profit Maximiser System.pdf, 10 pages, text layer, read in full).
//! Book id `mwpm`. Two MetaTrader indicators, SMA 21 and the Awesome Oscillator: BUY when the price closes above the SMA 21 and the Awesome Oscillator is green, enter at the next
//! candle; exit when the price retraces and closes back through the SMA 21; stop = the previous candle's low minus 50 pips (short: high plus 50 pips); sells mirrored. The text
//! recommends the 4-hour chart and allows "1H, 4H or daily". Lane K1 (23 Nov 2026, queue Q381 row 137). Independent check authors/conv-1123/K1/mwpm_ref.py.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "mwpm";
pub const REVIEWED: &[&str] = &["mwpm_daily"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared adaptation, R4 #340/#380] The system is a forex (EUR/USD, GBP/USD) MetaTrader system; this card runs its rules on the NSE indices NIFTY 50 and NIFTY BANK (and the US indices) on DAILY bars, a timeframe the text allows (\"1H, 4H or daily\"). The book prints no results (two chart examples of +300 pips each), nothing is compared, UNVERIFIED. [Declared readings, each an unstated detail] (1) The Awesome Oscillator is the standard 5/34 simple averages of the bar midpoints and a bar is GREEN when it is above the previous bar's value, RED otherwise (the text: 'green, up or down doesn't matter' = the colour of the bar, not its side of zero). (2) The exit 'when price retraces and touch back SMA 21' is read as the worked examples do: the first close back through the SMA 21 ends the trade at that close. (3) The signal is read at a close and the trade enters at the next bar's open; one position at a time. (4) The 50-pip stop buffer is scaled to a percentage by ONE rule: 50 pips = 50 x 0.0001 / 1.3650 (the entry price of the book's EUR/USD example) = 0.3663 percent of the stop reference price, so the stop sits 0.3663 percent beyond the signal bar's low (long) or high (short); a gap through the stop fills at the open.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "mwpm_daily", name: "Profit Maximiser: close beyond SMA 21 with a green/red Awesome Oscillator, enter next bar, exit on the close back through the SMA (daily, indices)",
        group: "Most Wanted Forex Profit Maximiser", session: 0, dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "mwpm",
        rules: &[
            "E|\"BUYING RULES - The price closes above the SMA 21 - The Awesome Oscillator is GREEN (up or down doesn't matter) - Enter at the next candle\" (p.4); \"SELLING RULES - The price closes below the SMA 21 - The Awesome Oscillator is RED - Enter at the next candle\"",
            "X|\"EXIT RULES - Exit your LONG trade when price retraces and touch back SMA 21\" (short mirrored); worked examples: the trade is closed when the price closes back through the SMA 21 (p.7)",
            "X|\"STOP LOSS - The previous lowest price of the candle - 50 pips\" (short: previous highest price + 50 pips)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id { "mwpm_daily" => Some(("mwpm", "Profit Maximiser (SMA 21 + Awesome Oscillator)", "SMA 21, AO 5/34, stop 0.3663 percent beyond the signal bar")), _ => None }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    match id {
        "mwpm_daily" => Some(vec![p("sma", "Moving average length (book 21)", 21., 5., 100., 1.),
            p("buffer", "Stop buffer beyond the signal bar, percent (50 pips scaled by 0.0001 / 1.3650)", 0.3663, 0.0, 3.0, 0.05)]),
        _ => None,
    }
}

fn sma(v: &[f64], n: usize) -> Vec<f64> {
    let mut o = vec![f64::NAN; v.len()];
    for i in n.saturating_sub(1)..v.len() { if i + 1 >= n { o[i] = v[i + 1 - n..=i].iter().sum::<f64>() / n as f64; } }
    o
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let s = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| s.iter().find(|d| d.key == k).unwrap().def);
    let (n, buf) = (g("sma").round().max(2.0) as usize, g("buffer") / 100.0);
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let mid: Vec<f64> = (0..c.len()).map(|i| (h[i] + l[i]) / 2.0).collect();
    let (ma, a5, a34) = (sma(c, n), sma(&mid, 5), sma(&mid, 34));
    let ao: Vec<f64> = (0..c.len()).map(|i| a5[i] - a34[i]).collect();
    let mut out = vec![];
    let mut pos: Option<(f64, usize, f64, f64)> = None; // dir, entry bar, entry price, stop
    let mut pend: Option<(f64, f64)> = None; // direction, stop level from the signal bar
    for t in 0..=to {
        if t >= from {
            if let Some((d, stop)) = pend.take() { if pos.is_none() { pos = Some((d, t, o[t], stop)); } }
            if let Some((d, e, ep, stop)) = pos {
                let hit = if d > 0.0 { l[t] <= stop } else { h[t] >= stop };
                let back = !ma[t].is_nan() && if d > 0.0 { c[t] < ma[t] } else { c[t] > ma[t] };
                let end = if hit { Some((if d > 0.0 { o[t].min(stop) } else { o[t].max(stop) }, "stop beyond the signal bar", "stop")) }
                          else if back { Some((c[t], "price closed back through the SMA 21", "close")) } else { None };
                if let Some((px, why, kind)) = end {
                    let _ = kind;
                    out.push(Trade { dir: d, legs: vec![Leg { i: e, px: ep, w: 1. }], exit_i: t, exit_px: px, reason: why.into(), entry_kind: "open", signal_i: e.saturating_sub(1), open: false, exits: vec![] });
                    pos = None;
                }
            }
            if pos.is_none() && t < to && !ma[t].is_nan() && ao[t].is_finite() && t > 0 && ao[t - 1].is_finite() {
                let green = ao[t] > ao[t - 1];
                if c[t] > ma[t] && green { pend = Some((1.0, l[t] * (1.0 - buf))); }
                else if c[t] < ma[t] && !green { pend = Some((-1.0, h[t] * (1.0 + buf))); }
            }
        }
    }
    if let Some((d, e, ep, _)) = pos {
        out.push(Trade { dir: d, legs: vec![Leg { i: e, px: ep, w: 1. }], exit_i: to, exit_px: c[to], reason: "open".into(), entry_kind: "open", signal_i: e.saturating_sub(1), open: true, exits: vec![] });
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn trades_and_rejects_other_ids() {
        let r: Vec<Row> = (0..200).map(|i| { let c = 100. + (i as f64 * 0.15).sin() * 10. + i as f64 * 0.05; Row { ticker: "NIFTY 50".into(), date: format!("2010-{:03}", i), open: c - 0.3, high: c + 1., low: c - 1., close: c, volume: None } }).collect();
        let x = Ctx::new(&r, &r, &[]);
        assert!(run("mwpm_x", &x, 0, 199, &Params::default()).is_none());
        let t = run("mwpm_daily", &x, 0, 199, &Params::default()).unwrap();
        assert!(!t.is_empty());
        for z in &t { assert!(z.exit_i >= z.legs[0].i); }
    }
}

// Instrument admission: indices only (never single stocks or ETFs), declared adaptation.
pub fn index_only() -> &'static str { "Profit Maximiser (forex MetaTrader system): run on actual indices only (NIFTY 50 / NIFTY BANK and the US indices) as a declared adaptation (R4 #340/#380); UNVERIFIED" }
