//! Boris Schlossberg: "Technical Analysis of the Currency Market" (Wiley 2006; book id `bstacm`), Ch.10 "Setups! Setups!
//! Setups!" (p.161-181). Reading: claude-continuation-20261020/authors/schlossberg (text + admission/bs_ref.py). Live: trading pure
//! trend with Bollinger band "bands" (p.169-171) and the Tao of RSI with each of its three printed exits (p.176-178). The other
//! setups need a hand-picked swing point, a range "amplitude", scale-in units or an undefined "compression" and are held.
//! Book: currency pairs; indices, ETFs and stocks are a declared application.
use crate::connors::{rsi, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

/// Admitted ids (authors/schlossberg/admission/bs_ref.py (header)).
pub const REVIEWED: &[&str] = &["bstacm_bb_trend", "bstacm_rsi_be", "bstacm_rsi_half", "bstacm_rsi_70"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const TREND_NOTE: &str = "Bollinger bands as printed on p.162: typical price = (high + low + close) / 3, middle = its 20-day average, bands = middle +/- 1 and 2 standard deviations of the 20 typical prices (population deviation - [declared reading]). Buy zone = between the upper 1 and upper 2 SD bands, sell zone = between the lower 1 and lower 2 SD bands (Ch.5 p.73-74). [Declared readings] A signal is a close inside the buy zone (bands included) when the previous close was not inside it; buy at that close (short: sell zone). Stop = the entry bar's low (high) from the next bar, filled at the level when touched or at a worse open. Exit at the first close below the upper 1 SD band (short: above the lower 1 SD band). One position at a time; a new zone entry can re-enter after an exit, also on the exit close.";
const RSI_NOTE: &str = "Bollinger bands as printed on p.162: typical price = (high + low + close) / 3, middle = its 20-day average, bands = middle +/- 1 and 2 standard deviations of the 20 typical prices (population deviation - [declared reading]). Buy zone = between the upper 1 and upper 2 SD bands, sell zone = between the lower 1 and lower 2 SD bands (Ch.5 p.73-74). [Declared readings] RSI = Wilder RSI on closes (book: 14). Signal = the third bar of a new run of RSI >= 70 (short) or <= 30 (long), entered at its close. Short stop = high of bar 3 + (high of bar 3 - low of bar 1) as printed; the long stop mirrors it (low of bar 3 - (high of bar 1 - low of bar 3)) because the book's long text repeats the short wording. Risk R = distance entry-stop (no trade if R <= 0). Stops and targets fill at their level or a gapped open; when one bar touches both the stop is taken first. Break-even stop and band target start on the bar after the trigger bar; the band level is the one at the previous close. One position at a time.";

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    let d = |id, name, group, status, rules, notes| StrategyDef { id, name, group, session: 0, dir: "Long/Short", status, needs_volume: false,
        needs_vix: false, needs_series: &[], book: "bstacm", rules, notes, claim: None };
    let nt = |id, name, group, why, rules| d(id, name, group, NotTestable(why), rules, "Held after the 2026-10-27 reading (authors/schlossberg).");
    const RSI_E: &str = "E|\"If the price chart prints three consecutive bars with RSI readings of 70 or greater, short on the close of the third bar. Measure from the bottom of the first bar to the top of the third bar. Add that distance to the top of the third bar and place a stop there.\" Long: \"three consecutive bars with RSI readings of 30 or less, go long on the close of the third bar\" (p.176-177)";
    vec![
        d("bstacm_bb_trend", "Trading pure trend with Bollinger band \"bands\"", "Ch.10 Setups (p.169-171)", Live,
          &["F|Bollinger band \"bands\": 20-period, 1 and 2 standard deviations (p.162-164)",
            "E|\"1. Price must enter the Bollinger band buy zone. 2. Price should close within the buy zone to trigger a buy signal.\" (p.169)",
            "X|\"3. Place stop at the bottom of the entry bar. 4. If the trade moves in the right direction, exit only when price closes below the buy channel.\" Shorts: \"the rules for shorts are simply reversed\" (p.169-170)"], TREND_NOTE),
        d("bstacm_rsi_be", "Tao of RSI - exit 1: break-even after 1R, opposite 2 SD band", "Ch.10 Setups (p.176-177)", Live,
          &["F|RSI (14,1) and Bollinger band \"bands\" 20 / 1 and 2 SD (p.176)", RSI_E,
            "X|\"1. If the price moves in your direction by the value of your stop ..., move your stop to the break-even point and target the opposite 2 standard deviation Bollinger band as exit.\" (p.177)"], RSI_NOTE),
        d("bstacm_rsi_half", "Tao of RSI - exit 2: half at 70% of the risk, rest at the opposite 2 SD band", "Ch.10 Setups (p.176-177)", Live,
          &["F|RSI (14,1) and Bollinger band \"bands\" 20 / 1 and 2 SD (p.176)", RSI_E,
            "X|\"2. If the price moves in your direction by 70 percent the value of your stop ..., take profit on one-half of the position, move your stop to the break-even point, and target the opposite 2 standard deviation Bollinger band as exit for the rest.\" (p.177)"], RSI_NOTE),
        d("bstacm_rsi_70", "Tao of RSI - exit 3: full profit at 70% of the risk", "Ch.10 Setups (p.176-178)", Live,
          &["F|RSI (14,1) (p.176)", RSI_E,
            "X|\"3. If the price moves in your direction by 70 percent of the value of your stop ..., take full profit on the position.\" (p.177)"], RSI_NOTE),
        nt("bstacm_bb_range", "Range trading with Bollinger bands", "Ch.10 (p.162-164)", "the range needs 'at least one swing high and one swing low' picked on the chart and its amplitude sets the stop",
           &["E|\"Go long the next time the price hits the lower Bollinger band. Set a stop at the low of the entry bar plus half the value of the amplitude of the range\""]),
        nt("bstacm_bb_exhaustion", "Trend exhaustion with Bollinger band \"bands\" (three tactics)", "Ch.10 (p.164-169)", "the stop is 'the swing low minus 1 point and minus the spread' with no definition of the swing; the other tactics scale out in thirds or add a second half-position at a loss",
           &["E|\"If the closing price of the period is either above the preceding Bollinger band sell channel or below the preceding Bollinger band buy channel, a trend exhaustion signal is generated\" - enter at the next bar's open"]),
        nt("bstacm_scalp_trend", "Scalping to trend (1 SD pullback after a 2 SD tag)", "Ch.10 (p.171-173)", "not admitted in this pass: needs a reading of when the 1 SD 'retrace' fills inside a bar after the 2 SD tag; can be coded as a user option",
           &["E|\"1. Price enters the buy channel and tags the upper 2 standard deviation Bollinger band. 2. If price retraces down to 1 standard deviation Bollinger band, buy at market. 3. Place stop at 20-period SMA. 4. Exit trade on next tag of 2 standard deviation Bollinger band.\""]),
        nt("bstacm_macd_turn", "The MACD turn", "Ch.10 (p.173-176)", "entry halfway to 'the most recent swing high' (undefined swing) and the position is built in thirds against the move",
           &["E|\"For a short setup look for the MACD histogram to print a lower high bar ... Place a sell stop at halfway between the two points for one-third of total position\""]),
        nt("bstacm_three_sma", "Three simple moving averages (7/20/65)", "Ch.10 (p.180-181)", "'wait until volatility declines and 7, 20, and 65 SMAs all compress together' has no measure, and the stop is 'a few points below' the 65 SMA",
           &["E|\"As price retraces to the 7 SMA, place a stop a few points below the 65 SMA. Exit the trade when the 7 SMA crosses the 20 SMA.\""]),
        nt("bstacm_ch5_bands", "Bollinger band \"bands\" trend entry on a dip (Ch.5)", "Ch.5 Trend Is Your Friend? (p.73-77)", "the entry waits for a dip 'to the middle of the bands or to the 1 standard deviation band' - two different levels, no single rule; Ch.10 prints the complete version (bstacm_bb_trend)",
           &["E|\"we will look for an opportunity to buy on any small dip into the no-man's-land zone\""]),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "bstacm_bb_trend" => ("bstacm_bb_trend", "Schlossberg Bollinger band trend", "20 / 1-2 SD"),
        "bstacm_rsi_be" | "bstacm_rsi_half" | "bstacm_rsi_70" => ("bstacm_rsi_tao", "Schlossberg Tao of RSI", "three exits"),
        _ if id.starts_with("bstacm_") => ("bstacm_nt", "Schlossberg (held)", "-"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(match id {
        "bstacm_rsi_be" | "bstacm_rsi_half" | "bstacm_rsi_70" => vec![p("len", "RSI length", 14., 2., 50., 1.)],
        _ if id.starts_with("bstacm_") => vec![],
        _ => return None,
    })
}

/// (middle, standard deviation) of the typical price over n bars; population deviation.
fn tp_bands(x: &Ctx, n: usize) -> (Vec<f64>, Vec<f64>) {
    let tp: Vec<f64> = (0..x.c.len()).map(|i| (x.h[i] + x.l[i] + x.c[i]) / 3.).collect();
    let (mut m, mut s) = (vec![f64::NAN; tp.len()], vec![f64::NAN; tp.len()]);
    for i in n.saturating_sub(1)..tp.len() {
        let w = &tp[i + 1 - n..=i];
        let mu = w.iter().sum::<f64>() / n as f64;
        m[i] = mu;
        s[i] = (w.iter().map(|v| (v - mu) * (v - mu)).sum::<f64>() / n as f64).sqrt();
    }
    (m, s)
}

fn trade(d: f64, ei: usize, e: f64, xi: usize, xp: f64, why: &str, open: bool) -> Trade {
    Trade { dir: d, legs: vec![Leg { i: ei, px: e, w: 1. }], exit_i: xi, exit_px: xp, reason: why.into(), entry_kind: "close", signal_i: xi, open, exits: vec![] }
}

fn run_trend(x: &Ctx, from: usize, to: usize) -> Vec<Trade> {
    let (m, s) = tp_bands(x, 20);
    let zone = |i: usize| -> f64 {
        if !m[i].is_finite() { return 0.; }
        let c = x.c[i];
        if c >= m[i] + s[i] && c <= m[i] + 2. * s[i] { 1. } else if c >= m[i] - 2. * s[i] && c <= m[i] - s[i] { -1. } else { 0. }
    };
    let (mut out, mut cur) = (vec![], None::<(f64, usize, f64, f64)>); // dir, entry bar, entry price, stop
    for i in from.max(1)..=to {
        let (o, h, l, c) = (x.o[i], x.h[i], x.l[i], x.c[i]);
        if let Some((d, ei, e, st)) = cur {
            let hit = if (d > 0. && o <= st) || (d < 0. && o >= st) { Some((o, "stop: entry bar")) }
                else if (d > 0. && l <= st) || (d < 0. && h >= st) { Some((st, "stop: entry bar")) }
                else if (d > 0. && c < m[i] + s[i]) || (d < 0. && c > m[i] - s[i]) { Some((c, "close left the trend zone")) } else { None };
            if let Some((px, why)) = hit { out.push(trade(d, ei, e, i, px, why, false)); cur = None; }
        }
        if cur.is_none() && i < to {
            let z = zone(i);
            if z != 0. && zone(i - 1) != z { cur = Some((z, i, c, if z > 0. { l } else { h })); }
        }
    }
    if let Some((d, ei, e, _)) = cur { out.push(trade(d, ei, e, to, x.c[to], "open", true)); }
    out
}

#[derive(Clone, Copy, PartialEq)]
enum Exit { BreakEven, Half, Seventy }

struct Pos { d: f64, ei: usize, e: f64, stop: f64, r: f64, be: bool, band: bool, arm: bool, half: Option<(usize, f64)> }

fn run_rsi(x: &Ctx, from: usize, to: usize, n: usize, mode: Exit) -> Vec<Trade> {
    let k = rsi(&x.c, n);
    let (m, s) = tp_bands(x, 20);
    let close_out = |p: &Pos, i: usize, px: f64, why: &str, open: bool| -> Trade {
        let mut t = trade(p.d, p.ei, p.e, i, px, why, open);
        if let Some((hi, hp)) = p.half { t.exits = vec![(hi, hp, 0.5), (i, px, 0.5)]; t.exit_px = 0.5 * hp + 0.5 * px; }
        t
    };
    let (mut out, mut cur) = (vec![], None::<Pos>);
    for i in from.max(3)..=to {
        let (o, h, l) = (x.o[i], x.h[i], x.l[i]);
        if let Some(mut p) = cur.take() {
            let d = p.d;
            let st = if p.be { p.e } else { p.stop };
            let why_s = if p.be { "breakeven stop" } else { "stop: three-bar range" };
            let mut res: Option<(f64, &str)> = None;
            if (d > 0. && o <= st) || (d < 0. && o >= st) { res = Some((o, why_s)); }
            else if (d > 0. && l <= st) || (d < 0. && h >= st) { res = Some((st, why_s)); }
            else {
                if p.band {
                    let t = if d > 0. { m[i - 1] + 2. * s[i - 1] } else { m[i - 1] - 2. * s[i - 1] };
                    if (d > 0. && o >= t) || (d < 0. && o <= t) { res = Some((o, "target: opposite 2 SD band")); }
                    else if (d > 0. && h >= t) || (d < 0. && l <= t) { res = Some((t, "target: opposite 2 SD band")); }
                }
                if res.is_none() && mode != Exit::BreakEven && p.half.is_none() {
                    let t = p.e + d * 0.7 * p.r;
                    let px = if (d > 0. && o >= t) || (d < 0. && o <= t) { Some(o) } else if (d > 0. && h >= t) || (d < 0. && l <= t) { Some(t) } else { None };
                    if let Some(px) = px {
                        if mode == Exit::Seventy { res = Some((px, "target: 70% of the risk")); } else { p.half = Some((i, px)); p.arm = true; }
                    }
                }
                if res.is_none() && mode == Exit::BreakEven && !p.band && !p.arm && ((d > 0. && h >= p.e + p.r) || (d < 0. && l <= p.e - p.r)) { p.arm = true; }
            }
            match res {
                Some((px, why)) => out.push(close_out(&p, i, px, why, false)),
                None => { if p.arm && !p.band { p.be = true; p.band = true; } cur = Some(p); }
            }
        }
        if cur.is_none() && i < to && [i, i - 1, i - 2].iter().all(|&j| k[j].is_finite()) {
            for d in [-1., 1.] {
                let hit = |v: f64| if d < 0. { v >= 70. } else { v <= 30. };
                if hit(k[i]) && hit(k[i - 1]) && hit(k[i - 2]) && !(k[i - 3].is_finite() && hit(k[i - 3])) {
                    let e = x.c[i];
                    let stop = if d < 0. { x.h[i] + (x.h[i] - x.l[i - 2]) } else { x.l[i] - (x.h[i - 2] - x.l[i]) };
                    let r = (stop - e) * -d;
                    if r > 0. { cur = Some(Pos { d, ei: i, e, stop, r, be: false, band: false, arm: false, half: None }); }
                    break;
                }
            }
        }
    }
    if let Some(p) = cur { out.push(close_out(&p, to, x.c[to], "open", true)); }
    out
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    Some(match id {
        "bstacm_bb_trend" => run_trend(x, from, to),
        "bstacm_rsi_be" => run_rsi(x, from, to, g("len").round().max(2.) as usize, Exit::BreakEven),
        "bstacm_rsi_half" => run_rsi(x, from, to, g("len").round().max(2.) as usize, Exit::Half),
        "bstacm_rsi_70" => run_rsi(x, from, to, g("len").round().max(2.) as usize, Exit::Seventy),
        _ => return None,
    })
}

// Instrument admission: Boris Schlossberg 2026-10-26 (lane B) block, moved from instrument_policy.rs on 31 Oct 2026 (lane ST).
// Admission only for the module REVIEWED lists (authors/schlossberg/admission/bs_ref.py (header)); kept in one block to merge.
// Runs on actual indices, ETFs and stocks, US and India (index_etf_or_stock, nifty50 basket, declared application).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book=="bstacm" && crate::books::bstacm::reviewed(def.id),
    label: "Boris Schlossberg wrote these setups for currency pairs; runs on indices, ETFs and stocks, US and India, are a declared application, not the author's test. Cash-index shorts are hypothetical.",
    basis: "Boris Schlossberg, Technical Analysis of the Currency Market (Wiley 2006), Ch.10: trading pure trend with Bollinger band bands (p.169-171) and the Tao of RSI with its three printed exits (p.176-178)",
};
