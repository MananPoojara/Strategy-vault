//! Robert "The Hawk" Dorfman, "Hedge Fund Trading Secrets Revealed" (2008; book id `rdhft`; printed pages). Reading:
//! claude-continuation-20261020/authors/dorfman (admission/hf_ref.py). Live: the Chapter 22 3-period EMA method
//! ("Remember the Importance of the Number 3?", p.252-254): after three bars of lower highs and lower lows, a close
//! above the 3-period EMA; buy when the next bar trades above that bar's high; stop under that bar's low; half out at the
//! nearer of the 10-day / 20-day moving average, stop to break-even, the rest at the next moving average (short: the
//! mirror). Not coded: the candlestick-pattern alternative setups, the 5-to-40 cent target zone, the oscillator
//! confirmations; the Samurai Crossover (p.278-279) is a 3-day EMA / 10-period average cross, the same calculation as
//! dgmf_ema_5_21 with fast 3 / slow 10 (no new copy). Book: stocks (weekly or daily) and forex pairs; indices, ETFs and
//! US/India stocks here are a declared application.
use crate::connors::{sma, Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::apta::ema;
use super::vrsdd::manage;
/// Admitted ids (authors/dorfman/admission/hf_ref.py (header)).
pub const REVIEWED: &[&str] = &["rdhft_ema3_turn"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] Daily bars. Setup bar: the three bars before it each made a lower high and a lower low than the bar before them, and the setup bar closes above the 3-period exponential average of closes (short: three higher highs and higher lows, close below the average). Entry: the next bar only, when it trades above the setup bar's high (filled at that high, or at the open when it opens above; short: below the low). Stop: the setup bar's low ('the prior bar's highs or lows as stop loss points from your entry price candle', p.254; short: its high). The averages are the book's chart set (p.186): 10-period exponential, 20-, 50- and 200-day simple, read on the setup bar and kept fixed for the trade. First target: the nearer of the 10-day EMA and the 20-day SMA beyond the entry price; half is sold there and the stop moves to the entry price. Second target: the nearest of the four averages beyond the first target; if none is beyond it, 'another 5% target' (p.254). If neither the 10 nor the 20 is beyond the entry ('if there isn't a moving average serving as resistance/target above your entry point', p.254): half at 5% (the low end of the book's 5-10%, option) and the rest another 5% further, stop to the entry price after the first half; the upper Bollinger band alternative is not applied. The '5-to-40 cent or 10-30 pip zone' around an average is judgement and not applied (targets are the average itself). Exits are checked from the day after entry (stop first when both are touched), each filled at its level or a gapped open. Averages start once they have enough bars (the exponential averages from the simple average of their first bars). One position at a time; setups during a trade are ignored.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "rdhft_ema3_turn", name: "Number 3: close back over the 3-period EMA after three lower bars, half out at the 10/20-day average",
        group: "Dorfman Hedge Fund Trading Secrets", session: 0, dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "rdhft",
        rules: &["F|\"you can enter a long position when the price closes above the 3-period EMA, and where at least the previous three bars have made lower highs and lower lows\" (p.253); short: \"closes below its 3-day EMA after at least three consecutive higher highs and higher lows\" (p.252)",
            "E|\"You can then enter the trade when the price crosses above the previous candle's high which completed the pattern\" (p.253)",
            "X|\"Use the prior bar's highs or lows as stop loss points from your entry price candle\" (p.254)",
            "X|\"looking for the 10- or 20-day moving average line to serve as the first resistance/target profit area where you would sell a portion (usually half) of your position, and hold the rest for further gains (using the next moving average as the next target/resistance profit area)\" (p.253); \"You can then move your stop to breakeven until the price moves down to the next moving average\" (p.253)",
            "X|\"if there isn't a moving average serving as resistance/target above your entry point ... For taking profits, use a 5-10% price target; then when that's hit, set another 5% target and move your stop to the break-even point\" (p.254)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == "rdhft_ema3_turn").then_some(("rdhft_ema3_turn", "Dorfman 3-period EMA turn", "10/20-day targets"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    (id == "rdhft_ema3_turn").then(|| vec![p("side", "Sides: 0 = long and short (book), 1 = long only, 2 = short only", 0., 0., 2., 1.),
        p("pct", "First target when no 10/20-day average lies ahead, % (book 5-10)", 5., 5., 10., 1.)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let (side, pct) = (g("side").round() as i64, g("pct") / 100.);
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let (e3, e10, s20, s50, s200) = (ema(c, 3), ema(c, 10), sma(c, 20), sma(c, 50), sma(c, 200));
    let mut out = vec![];
    let mut t = from.max(4);
    while t < to {
        let dn = (1..=3).all(|k| h[t - k] < h[t - k - 1] && l[t - k] < l[t - k - 1]);
        let up = (1..=3).all(|k| h[t - k] > h[t - k - 1] && l[t - k] > l[t - k - 1]);
        let d = if dn && c[t] > e3[t] { 1. } else if up && c[t] < e3[t] { -1. } else { 0. };
        if d == 0. || (side == 1 && d < 0.) || (side == 2 && d > 0.) { t += 1; continue; }
        let e = t + 1;
        let lvl = if d > 0. { h[t] } else { l[t] };
        if !((d > 0. && h[e] > lvl) || (d < 0. && l[e] < lvl)) { t += 1; continue; }
        let (ep, kind) = if d * (o[e] - lvl) > 0. { (o[e], "open") } else { (lvl, "stop") };
        let stop = if d > 0. { l[t] } else { h[t] };
        let nearest = |from_px: f64, set: &[f64]| set.iter().copied().filter(|v| v.is_finite() && d * (v - from_px) > 0.)
            .min_by(|a, b| (d * (a - from_px)).partial_cmp(&(d * (b - from_px))).unwrap());
        let (t1, t2) = match nearest(ep, &[e10[t], s20[t]]) {
            Some(t1) => (t1, nearest(t1, &[e10[t], s20[t], s50[t], s200[t]]).unwrap_or(t1 * (1. + d * 0.05))),
            None => { let t1 = ep * (1. + d * pct); (t1, t1 * (1. + d * 0.05)) }
        };
        let tr = manage(x, d, e, ep, kind, stop, &[(0.5, t1), (0.5, t2)], true, usize::MAX, to);
        let (xi, op) = (tr.exit_i, tr.open);
        out.push(tr);
        if op { break; }
        t = xi + 1;
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn hand_checked_turn_with_average_targets() {
        // 30 flat bars, three lower bars, a close back over the 3-EMA, then a rally through the 10-EMA (~97.97) and 20-SMA (99.3)
        let mut r: Vec<Row> = (0..30).map(|i| Row { ticker: "T".into(), date: format!("2020-{:02}-{:02}", 1 + i / 28, 1 + i % 28), open: 100., high: 101., low: 99., close: 100., volume: None }).collect();
        let mut push = |o: f64, h: f64, l: f64, c: f64| { let i = r.len(); r.push(Row { ticker: "T".into(), date: format!("2020-03-{:02}", i - 29), open: o, high: h, low: l, close: c, volume: None }); };
        push(100., 100.5, 98., 98.5); push(98.5, 99., 96., 96.5); push(96.5, 97., 94., 94.5); // three lower highs and lows
        push(94.5, 96.8, 94.2, 96.5); // setup: close 96.5 over the 3-EMA (~96.34); high 96.8, stop at the low 94.2
        push(96.6, 97.5, 96.4, 97.2); // long at 96.8 (buy stop over the setup high)
        push(97.2, 101.5, 97., 101.); // half at 97.97, half at 99.3
        let x = Ctx::new(&r, &r, &[]);
        let t = run("rdhft_ema3_turn", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert_eq!(t.len(), 1);
        let z = &t[0];
        assert_eq!((z.dir, z.legs[0].i, (z.legs[0].px * 100.).round(), z.entry_kind, z.exit_i, z.reason.as_str(), z.open), (1., 34, 9680., "stop", 35, "target", false));
        assert_eq!(z.exits.len(), 2);
        assert!(run("rdhft_x", &x, 0, r.len() - 1, &Params::new()).is_none());
    }
}
