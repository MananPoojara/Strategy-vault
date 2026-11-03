//! Walter Bressert, The Cycle Trading Pattern Manual (RSI3M3 mechanical buy signal, PDF p.6-7) with the two-price-bar
//! reversal trailing stop of his ProfitTrader 7.0 manuals (TradeStation PT7_Manual p.16-17; MetaStock PT7 manual p.iii, p.5);
//! research share USB HDD 1/TradeStation Systems & Indicators/_Misc/Walter Bressert Indicator for OR/WalterBressetStuff/
//! Manual.PDF and the two PT7 manuals. Book id `wbctp`. Lane K7 (titles loop, 1 Nov 2026); rule text
//! authors/batch-c/C4w/bressert-rsi3m3.txt; independent check authors/titles-1030/K7w/k7_ref.py (header).
use crate::connors::{rsi, Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::mpts::sma;
use super::rbtie::mk;

/// Admitted ids (authors/titles-1030/K7w/k7_ref.py (header)).
pub const REVIEWED: &[&str] = &["wbctp_rsi3m3"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] RSI3M3 = 3-day simple average of Wilder's 3-day RSI of closes. Setup bar: RSI3M3 rises on a bar after a bar on which it was below the buy line 30 ('The price bar that turned the oscillator up ... is a setup bar'). Buy stop one tick above the setup bar's high, working until filled or until a newer setup bar replaces it (option: cancel it when RSI3M3 turns down). Protective sell stop one tick below the cycle bottom = the lowest low from the bar on which RSI3M3 last dropped below 30 through the entry bar (in his example B the bottom came between the setup and the entry). Trailing stop: the two-price-bar reversal, one tick below the lower low of the two previous bars, from the bar after entry, never lowered; the higher of the two stops applies, filled at the stop or a lower open (3- or 4-bar reversals are options, 'You may find a 3 or 4-bar ... more suited'). A tick is the instrument's own (0.01 US, 0.05 NSE). Long only (option: the mirrored sell signal with a sell line of 70, the PT7 SellLine default). Not applied: 'Buy Signals should only be taken in the direction of trend', the timing bands and the cycle judgement. The manual's '75% could have made money' are chart counts, not a test; his markets are futures and stocks; US and India indices, ETFs and stocks are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "wbctp_rsi3m3", name: "RSI3M3 buy signal: oscillator turns up from below 30, buy stop above the setup bar, cycle-bottom stop and two-bar reversal trail",
        group: "Bressert cycle trading", session: 0, dir: "Long", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "wbctp", rules: &[
            "E|\"1. The RSI3M3 drops below the buy line at 30 ... 2. The oscillator turns up ... The price bar that turned the oscillator up is ... a setup bar. 3. A buy stop to go long is placed one tick above the high of the setup bar.\" (Manual p.6-7)",
            "X|\"4. Once a market is entered a protective sell stop is placed one tick below the cycle bottom\" (p.7); \"An effective tighter trailing stop is a two-price bar reversal that stops you out whenever the high or low of the two previous price bars are taken out (highs if short; lows if long)\" (PT7 MetaStock manual p.iii)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == "wbctp_rsi3m3").then_some(("wbctp_rsi3m3", "Bressert RSI3M3 signal", "buy line 30, 2-bar reversal trail"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    (id == "wbctp_rsi3m3").then(|| vec![
        ParamDef { key: "bars", label: "Trailing stop: reversal of the last N bars (book 2; 3 or 4 options)", def: 2., min: 1., max: 6., step: 1. },
        ParamDef { key: "cancel", label: "1 = cancel the buy stop when RSI3M3 turns down", def: 0., min: 0., max: 1., step: 1. },
        ParamDef { key: "short", label: "1 = also the mirrored sell signal (sell line 70)", def: 0., min: 0., max: 1., step: 1. },
    ])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d);
    let (nb, cancel, shorts) = (g("bars", 2.).round().max(1.) as usize, g("cancel", 0.) >= 0.5, g("short", 0.) >= 0.5);
    let tick = x.market_tick();
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let r = sma(&rsi(c, 3), 3);
    // start bar of the latest drop below 30 (above 70 for sells)
    let (mut lo_start, mut hi_start) = (vec![usize::MAX; c.len()], vec![usize::MAX; c.len()]);
    for i in 1..c.len() {
        lo_start[i] = if r[i] < 30. && !(r[i - 1] < 30.) { i } else { lo_start[i - 1] };
        hi_start[i] = if r[i] > 70. && !(r[i - 1] > 70.) { i } else { hi_start[i - 1] };
    }
    let mut out = vec![];
    let mut pos: Option<(f64, usize, f64, f64)> = None; // (dir, entry bar, price, stop)
    let mut pend: Option<(f64, f64, usize)> = None;      // (dir, stop price, episode start)
    for i in from..=to {
        if let Some((d, e, ep, st)) = pos {
            if i > e {
                let w = &(if d > 0. { l } else { h })[i.saturating_sub(nb)..i];
                let trail = if d > 0. { w.iter().copied().fold(f64::INFINITY, f64::min) - tick } else { w.iter().copied().fold(f64::NEG_INFINITY, f64::max) + tick };
                let st = if d > 0. { st.max(trail) } else { st.min(trail) };
                if (d > 0. && l[i] <= st) || (d < 0. && h[i] >= st) {
                    out.push(mk(d, e, ep, "stop", i, if d > 0. { o[i].min(st) } else { o[i].max(st) }, "stop", false, i)); pos = None;
                } else { pos = Some((d, e, ep, st)); }
            }
        } else if let Some((d, px, s0)) = pend {
            if (d > 0. && h[i] >= px) || (d < 0. && l[i] <= px) {
                let ep = if d > 0. { o[i].max(px) } else { o[i].min(px) };
                let st = if d > 0. { l[s0..=i].iter().copied().fold(f64::INFINITY, f64::min) - tick } else { h[s0..=i].iter().copied().fold(f64::NEG_INFINITY, f64::max) + tick };
                pos = Some((d, i, ep, st)); pend = None;
            }
        }
        if i >= to || i == 0 { continue; }
        if pos.is_some() { pend = None; continue; }
        let up = r[i] > r[i - 1] && r[i - 1] < 30.;
        let dn = shorts && r[i] < r[i - 1] && r[i - 1] > 70.;
        if up { pend = Some((1., h[i] + tick, lo_start[i - 1])); }
        else if dn { pend = Some((-1., l[i] - tick, hi_start[i - 1])); }
        else if cancel { if let Some((d, ..)) = pend { if (d > 0. && r[i] < r[i - 1]) || (d < 0. && r[i] > r[i - 1]) { pend = None; } } }
    }
    if let Some((d, e, ep, _)) = pos { out.push(mk(d, e, ep, "stop", to, c[to], "open", true, to)); }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn buys_above_a_setup_bar_after_rsi3m3_below_30_and_trails_two_bar_lows() {
        let c: Vec<f64> = (0..600).map(|i| 50. + 5. * (i as f64 / 6.).sin() + 1.5 * (i as f64 / 1.7).cos() + i as f64 * 0.01).collect();
        let rows: Vec<Row> = c.iter().enumerate().map(|(i, &v)| Row { ticker: "T".into(), date: format!("{}", i), open: v, high: v + 0.4, low: v - 0.4, close: v, volume: None }).collect();
        let x = Ctx::new(&rows, &rows, &[]);
        let r = sma(&rsi(&c, 3), 3);
        let t = run("wbctp_rsi3m3", &x, 0, rows.len() - 1, &Params::new()).unwrap();
        assert!(t.len() > 5 && t.iter().all(|z| z.dir > 0.));
        for z in t.iter().filter(|z| !z.open) {
            let b = &rows[z.exit_i];
            assert!(z.exit_px <= b.open + 1e-9 && z.exit_px >= b.low - 1e-9 && z.exit_i > z.legs[0].i);
        }
        assert!((1..r.len()).any(|i| r[i - 1] < 30. && r[i] > r[i - 1]));
        assert!(run("wbctp_x", &x, 0, 5, &Params::new()).is_none());
    }
}

// Instrument admission: Walter Bressert (Cycle Trading Pattern Manual; ProfitTrader 7.0) 2026-11-01 (titles lane K7).
// Admission only for the module REVIEWED lists (authors/titles-1030/K7w/k7_ref.py (header)).
// Runs on actual indices, ETFs and stocks, US and India (index_etf_or_stock, nifty50 basket, declared application).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "wbctp" && reviewed(def.id),
    label: "Walter Bressert taught the RSI3M3 signal on futures and stock charts with counts read off the charts; US and India indices, ETFs and stocks are a declared application.",
    basis: "Walter Bressert, The Cycle Trading Pattern Manual p.6-7 (RSI3M3 signal); ProfitTrader 7.0 manuals, TradeStation p.16-17 and MetaStock p.iii, p.5 (two-bar reversal stop)",
};
