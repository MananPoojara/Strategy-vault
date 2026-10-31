//! INVESTools, Advanced Technical Analysis Workshop Manual (2005), ch.6 'Backtesting, Money Management and Application':
//! the MACD crossover with the 30-day trend example on General Electric (printed p.148-151, Fig. 6.1-6.4); research share
//! USB HDD 1/TRADING KNOWLEDGE-2/_InvesTools.com/Advanced Technical Analysis. Know Exactly What the Market is Telling You
//! (Video & Manuals 1.06 GB)/4A - INVESTools Advanced Technical Analysis - Workshop Manuals.pdf. Book id `itatw`; admitted
//! with `it5sf`. Lane K7 (titles loop, 1 Nov 2026); rule text authors/batch-c/C1w/investools4a-p139-162.txt; independent
//! check authors/titles-1030/K7w/k7_ref.py (header).
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::mkits::ema;
use super::mpts::sma;
use super::rbtie::mk;

/// Admitted ids (authors/titles-1030/K7w/k7_ref.py (header)).
pub const REVIEWED: &[&str] = &["itatw_macd_ma30_usd", "itatw_macd_ma30_pct"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }
/// Dollar exits ($1.00 trail, $1.00 target, $0.50 stop): US instruments only (instrument_policy.rs).
pub const US_ONLY: &[&str] = &["itatw_macd_ma30_usd"];

const NOTE: &str = "[Declared readings] MACD 8/17/9 (Fig. 6.1, 'MACD (Hist. + 2 lines) (8,17,9)'), exponential averages seeded with the simple average of their first values; 30-day simple average (Fig. 6.1, 'Simple Moving Avg. (30)'). Long when the MACD line crosses above its signal line (above today, at or below yesterday) and the 30-day average is higher than yesterday; short on the mirror cross with a falling average; crosses against the trend are ignored. Entry at the next bar's open ('you are always getting in at the beginning of the next period after the signal', p.149). One trade at a time ('a few of your trades overlapped', p.151). Exits from the entry bar on: default the trailing stop the manual judges better (Fig. 6.4): the stop trails the entry price and the highest high since entry (lowest low for shorts) by the trail amount, tested against the level reached by the bar before, filled at the stop or a worse open; option: profit target and stop loss (Fig. 6.3), the stop tested first when both are inside one bar. The manual's amounts are dollars on General Electric at about $30: $1.00 trail, $1.00 target, $0.50 stop; those run on US instruments only (dollar amounts). The percentage version uses the same rule with 3.3% of the entry price for the trail and target and 1.7% for the stop (the dollar amounts over GE's ~$30), labelled, on US and India. A teaching example on one stock (eight entries in 2004), shown without a claim; indices, ETFs and other stocks are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name| StrategyDef { id, name, group: "INVESTools backtesting example", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "itatw", rules: &[
            "F|\"MACD (Hist. + 2 lines) (8,17,9)\" and \"Simple Moving Avg. (30)\" (Fig. 6.1, p.149)",
            "E|\"If the MACD indicator crosses above its signal line and the value for the 30-day moving average is higher than it was yesterday, you will enter a long trade\"; \"If the MACD indicator crosses below its signal line and the value for the 30-day moving average today is lower than it was yesterday, you will enter a short trade\" (p.148); \"getting in at the beginning of the next period after the signal\" (p.149)",
            "X|\"a trailing stop loss and no profit target ... a $1.00 trailing stop loss\" (Fig. 6.4, p.151); option \"a profit target of $1.00 and a stop loss of $0.50\" (Fig. 6.3, p.150-151)"],
        notes: NOTE, claim: None };
    vec![d("itatw_macd_ma30_usd", "MACD 8/17/9 cross with the 30-day trend, $1.00 trailing stop (US, as printed)"),
         d("itatw_macd_ma30_pct", "MACD 8/17/9 cross with the 30-day trend, 3.3% trailing stop (percentage reading)")]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "itatw_macd_ma30_usd" => ("itatw_macd_ma30", "INVESTools MACD + 30-day trend", "dollar exits (US)"),
        "itatw_macd_ma30_pct" => ("itatw_macd_ma30", "INVESTools MACD + 30-day trend", "percentage exits"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let usd = id == "itatw_macd_ma30_usd";
    reviewed(id).then(|| vec![
        ParamDef { key: "exit", label: "0 = trailing stop (Fig. 6.4), 1 = profit target + stop loss (Fig. 6.3)", def: 0., min: 0., max: 1., step: 1. },
        if usd { ParamDef { key: "trail", label: "Trailing stop ($, book 1.00)", def: 1., min: 0.05, max: 20., step: 0.05 } }
        else { ParamDef { key: "trail", label: "Trailing stop (% of entry, reading 3.3)", def: 3.3, min: 0.1, max: 30., step: 0.1 } },
        if usd { ParamDef { key: "target", label: "Profit target ($, book 1.00)", def: 1., min: 0.05, max: 20., step: 0.05 } }
        else { ParamDef { key: "target", label: "Profit target (% of entry, reading 3.3)", def: 3.3, min: 0.1, max: 30., step: 0.1 } },
        if usd { ParamDef { key: "stop", label: "Stop loss ($, book 0.50)", def: 0.5, min: 0.05, max: 20., step: 0.05 } }
        else { ParamDef { key: "stop", label: "Stop loss (% of entry, reading 1.7)", def: 1.7, min: 0.1, max: 30., step: 0.1 } },
    ])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let usd = id == "itatw_macd_ma30_usd";
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d);
    let fixed = g("exit", 0.) >= 0.5;
    let (tr, tg, sl) = if usd { (g("trail", 1.), g("target", 1.), g("stop", 0.5)) } else { (g("trail", 3.3), g("target", 3.3), g("stop", 1.7)) };
    let amt = |v: f64, ep: f64| if usd { v } else { ep * v / 100. };
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let (e8, e17) = (ema(c, 8), ema(c, 17));
    let m: Vec<f64> = e8.iter().zip(&e17).map(|(a, b)| a - b).collect();
    let sg = ema(&m, 9);
    let ma = sma(c, 30);
    let sig = |i: usize| -> f64 {
        if i == 0 { return 0.; }
        if m[i] > sg[i] && m[i - 1] <= sg[i - 1] && ma[i] > ma[i - 1] { 1. } else if m[i] < sg[i] && m[i - 1] >= sg[i - 1] && ma[i] < ma[i - 1] { -1. } else { 0. }
    };
    let mut out = vec![];
    let mut pos: Option<(f64, usize, f64, f64)> = None; // (dir, entry bar, price, best price so far)
    let mut pend = 0.;
    for i in from..=to {
        if pend != 0. { pos = Some((pend, i, o[i], o[i])); pend = 0.; }
        if let Some((d, e, ep, best)) = pos {
            let exit = if fixed {
                let (st, tp) = (ep - d * amt(sl, ep), ep + d * amt(tg, ep));
                if (d > 0. && l[i] <= st) || (d < 0. && h[i] >= st) { Some((if d > 0. { o[i].min(st) } else { o[i].max(st) }, "stop loss")) }
                else if (d > 0. && h[i] >= tp) || (d < 0. && l[i] <= tp) { Some((if d > 0. { o[i].max(tp) } else { o[i].min(tp) }, "profit target")) }
                else { None }
            } else {
                let st = best - d * amt(tr, ep);
                if (d > 0. && l[i] <= st) || (d < 0. && h[i] >= st) { Some((if d > 0. { o[i].min(st) } else { o[i].max(st) }, "trailing stop")) } else { None }
            };
            match exit {
                Some((px, why)) => { out.push(mk(d, e, ep, "open", i, px, why, false, i)); pos = None; }
                None => pos = Some((d, e, ep, if d > 0. { best.max(h[i]) } else { best.min(l[i]) })),
            }
        }
        if pos.is_none() && i < to { pend = sig(i); }
    }
    if let Some((d, e, ep, _)) = pos { out.push(mk(d, e, ep, "open", to, c[to], "open", true, to)); }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn entries_follow_the_trend_filtered_cross_and_exits_respect_the_amounts() {
        let c: Vec<f64> = (0..600).map(|i| 30. + 3. * (i as f64 / 17.).sin() + 1.5 * (i as f64 / 4.3).cos() + i as f64 * 0.004).collect();
        let rows: Vec<Row> = c.iter().enumerate().map(|(i, &v)| Row { ticker: "T".into(), date: format!("{}", i), open: v, high: v + 0.3, low: v - 0.3, close: v, volume: None }).collect();
        let x = Ctx::new(&rows, &rows, &[]);
        for id in REVIEWED {
            let t = run(id, &x, 0, rows.len() - 1, &Params::new()).unwrap();
            assert!(t.len() > 3);
            let mut p = Params::new(); p.insert("exit".into(), 1.);
            for z in run(id, &x, 0, rows.len() - 1, &p).unwrap().iter().filter(|z| !z.open) {
                let ep = z.legs[0].px; let r = z.dir * (z.exit_px - ep);
                let (tg, st) = if *id == "itatw_macd_ma30_usd" { (1., 0.5) } else { (ep * 0.033, ep * 0.017) };
                assert!(r >= tg - 1e-9 || r <= -st + 1e-9, "{id} {r}");
            }
            for w in t.windows(2) { assert!(w[1].legs[0].i > w[0].exit_i); }
        }
        assert!(run("itatw_x", &x, 0, 5, &Params::new()).is_none());
    }
}
