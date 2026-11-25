//! Nelson F. Freeburg, "Portfolio-level commodity trading: Maximizing risk-adjusted gains with trade management", Formula Research
//! Vol. V No. 9 (21 June 1997) (research share Trading Knowledge II/@NewsLetters-Mags/Futures Truth-style archive; text authors/conv-1123/k3w/text/txt2/1215_*).
//! Two complete futures systems: A (the "simple reversal system": two lower lows then a higher low, 28-day SMA filter, 10-day low trailing stop)
//! and B (George Bagsarian's ATR system: 84-day high buy stop, tighter of a 4 x true-range trailing stop and a 144/21-day relative-strength band).
//! Book id `frmtm`. Lane K3 (23 Nov 2026). Daily bars; both systems are price-unit-free (ticks only for the one-tick buy-stop offset).
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "frmtm";
const REV: &str = "frmtm_rev";
const ATR: &str = "frmtm_atr84";
pub const REVIEWED: &[&str] = &[REV, ATR];

const NOTE_A: &str = "A|Declared readings (lane K3): Card-specific unstated choices (6), each independent: (1) 'two lower lows followed by a higher low' = low[t-2] < low[t-3], low[t-1] < low[t-2], low[t] > low[t-1] on the setup day t (each low compared with the previous bar's); (2) 'close above the 28-day SMA' = the setup day's close above the simple average of the 28 closes ending on the setup day; (3) the entry fills at the close of the day after the setup, only when that day's open is above the setup day's close ('enter long on tomorrow's close'); (4) 'the lowest low of the past ten days' = the ten bars before the stop-testing day (the stop day itself excluded; the entry day included); (5) the trailing stop is only ever raised (shorts: highest high, only lowered); (6) shorts are the stated mirror and are on by default (parameter short=0 for long only). Honest total: 6 card-specific choices. Engine-wide defaults / declared adaptations (labelled, not counted, R4 #404): the stop is active from the day after the entry and fills at the stop or at the open if it gaps through; no costs; the five NSE cash indices as the declared application of the futures system (#340). Book test: four currencies 1984-88 on single contracts +63,087 USD, 18.2% a year, 29% drawdown (not comparable: currency futures, 100 USD slippage and commission per trade).";
const NOTE_B: &str = "A|Declared readings (lane K3): Card-specific unstated choices (9), each independent: (1) 'one tick above the 84-day high' = one tick of the series (0.05 on the NIFTY series, 0.01 otherwise), the indices having no contract tick; (2) the 84-day window is the 84 bars before the trading day (the day itself excluded) and the buy stop is re-placed at every close while flat; (3) when both the buy stop and the sell stop are hit on the same day, the one nearer the open is taken; (4) true range = higher of today's high and yesterday's close minus lower of today's low and yesterday's close, times 4; (5) P = 8-day exponential average (alpha 2/9) of 4 x true range, seeded with its first value; (6) the first stop is entry minus P with P at the entry day's close; each later close raises it to close minus P (shorts mirrored), never lowering it; (7) the 3% bands are multiplicative: lower band = 0.97 x the 21-day SMA of R, upper band = 1.03 x that SMA; (8) 'whichever stop is closest' = whichever fires first (the price stop intraday, the band stop at the next open); (9) shorts are the stated mirror and are on by default (parameter short=0 for long only). Honest total: 9 card-specific choices. Engine-wide defaults / declared adaptations (labelled, not counted, R4 #404): fills at the stop or at the open if it gaps through; the band stop exits at the next open (as the source); no costs; the five NSE cash indices as the declared application of the futures system (#340). The sizing of the study (risk 3.5% per trade, 11% total) is not modelled. Book test: 27 futures 1981-1997, 19 profitable, 46% winners, 17.4% a year, 45% drawdown (not comparable).";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        StrategyDef { id: REV, name: "Simple reversal system: two lower lows then a higher low, 28-day SMA, 10-day low trail", group: "Formula Research V:9 trade management (Freeburg, 1997)", session: 0, dir: "Long/Short", status: Status::Live,
            needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
            rules: &["E|\"The setup for a buy signal is two lower lows followed by a higher low. Assume today the setup was satisfied. You enter long on tomorrow's close if two other conditions are met. First, today's close must be above a 28-day simple moving average. Second, tomorrow's open must be greater than today's close.\"",
                "X|\"Once long, trail a stop at the lowest low of the past ten days. Short trades are the mirror image of long trades.\"", NOTE_A],
            notes: "Freeburg, Formula Research V:9 (21 Jun 1997), simple reversal system. #340 declared application: the source is a futures system run here on NSE cash indices; the rule uses only price (no contract-specific amount), so it is market-agnostic.", claim: None },
        StrategyDef { id: ATR, name: "ATR system (Bagsarian): 84-day high buy stop, 4 x true-range trail or relative-strength band exit", group: "Formula Research V:9 trade management (Freeburg, 1997)", session: 0, dir: "Long/Short", status: Status::Live,
            needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
            rules: &["E|\"Place a buy stop tomorrow one tick above the highest high of the past 84 days. If you are filled there are two ways to get out. Choose the tighter of the two stops.\"",
                "X|\"The first exit is a dynamic trailing stop. ... Multiply the true range by four. Calculate an 8-day exponential average of this product, which we will call P. On the first day of the trade your stop is the entry price minus P. Thereafter, subtract P from each day's close. If the result is higher than the current exit, raise the stop to the newly computed level.\"",
                "X|\"The second exit is a volatility stop. Each day divide the close by its 144-day simple moving average. The result is a relative strength indicator we can call R. ... Take a 21-day simple moving average of R. Plot bands at intervals 3% above and below the 21-day smoothing. If R drops below the lower band, exit the trade on tomorrow's open.\" Sell signals are the exact opposite.", NOTE_B],
            notes: "Freeburg, Formula Research V:9 (21 Jun 1997), ATR system by George Bagsarian. #340 declared application: the source is a futures system run here on NSE cash indices; the rule uses only price (no contract-specific amount), so it is market-agnostic.", claim: None },
    ]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id { REV => Some(("frmtm_rev", "Simple reversal system (Freeburg)", "28-day SMA, 10-day trail")), ATR => Some(("frmtm_atr84", "ATR 84-day breakout (Bagsarian)", "84-day high, 4 x TR trail, 144/21 band")), _ => None }
}
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    match id {
        REV => Some(vec![
            ParamDef { key: "sma", label: "SMA length (days)", def: 28., min: 10., max: 100., step: 1. },
            ParamDef { key: "trail", label: "Trailing stop: lowest low of the past N days", def: 10., min: 3., max: 40., step: 1. },
            ParamDef { key: "short", label: "1 = also the mirrored short side", def: 1., min: 0., max: 1., step: 1. }]),
        ATR => Some(vec![
            ParamDef { key: "hh", label: "Breakout: highest high of the past N days", def: 84., min: 20., max: 200., step: 1. },
            ParamDef { key: "ema", label: "P: exponential average length of 4 x true range", def: 8., min: 2., max: 30., step: 1. },
            ParamDef { key: "mult", label: "True-range multiple", def: 4., min: 1., max: 8., step: 0.5 },
            ParamDef { key: "ma", label: "Relative strength R: SMA length of the close", def: 144., min: 50., max: 300., step: 1. },
            ParamDef { key: "sm", label: "Band centre: SMA length of R", def: 21., min: 5., max: 60., step: 1. },
            ParamDef { key: "band", label: "Band width (percent of the centre)", def: 3., min: 1., max: 10., step: 0.5 },
            ParamDef { key: "short", label: "1 = also the mirrored short side", def: 1., min: 0., max: 1., step: 1. }]),
        _ => None,
    }
}

fn sma_v(v: &[f64], n: usize) -> Vec<f64> {
    let mut o = vec![f64::NAN; v.len()];
    for i in 0..v.len() { if i + 1 >= n && !v[i + 1 - n..=i].iter().any(|a| a.is_nan()) { o[i] = v[i + 1 - n..=i].iter().sum::<f64>() / n as f64; } }
    o
}
fn t(dir: f64, ei: usize, ep: f64, kind: &'static str, xi: usize, xp: f64, why: &str, open: bool, sig: usize) -> Trade {
    Trade { dir, legs: vec![Leg { i: ei, px: ep, w: 1.0 }], exit_i: xi, exit_px: xp, reason: why.into(), entry_kind: kind, signal_i: sig, open, exits: Vec::new() }
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !REVIEWED.contains(&id) || to >= x.c.len() || from >= to { return None; }
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d);
    let shorts = g("short", 1.) >= 0.5;
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let mut out: Vec<Trade> = vec![];
    if id == REV {
        let (n, tr) = (g("sma", 28.).round().max(1.) as usize, g("trail", 10.).round().max(1.) as usize);
        let ma = sma_v(c, n);
        let mut pos: Option<(f64, usize, f64, f64)> = None; // dir, entry bar, entry px, stop
        for d in from..=to {
            if let Some((dir, e, ep, st)) = pos {
                if d > e {
                    let w = &(if dir > 0. { l } else { h })[d - tr..d];
                    let cand = if dir > 0. { w.iter().copied().fold(f64::INFINITY, f64::min) } else { w.iter().copied().fold(f64::NEG_INFINITY, f64::max) };
                    let st = if dir > 0. { st.max(cand) } else { st.min(cand) };
                    if (dir > 0. && l[d] <= st) || (dir < 0. && h[d] >= st) {
                        out.push(t(dir, e, ep, "close", d, if dir > 0. { o[d].min(st) } else { o[d].max(st) }, "trailing stop", false, d)); pos = None;
                    } else { pos = Some((dir, e, ep, st)); }
                    continue;
                }
                continue;
            }
            // setup day s = d-1, entry on the close of day d
            if d < 4 || d - 1 < n { continue; }
            let s = d - 1;
            if l[s - 2] < l[s - 3] && l[s - 1] < l[s - 2] && l[s] > l[s - 1] && c[s] > ma[s] && o[d] > c[s] {
                let st = l[d + 1 - tr.min(d + 1)..=d].iter().copied().fold(f64::INFINITY, f64::min);
                pos = Some((1., d, c[d], st));
            } else if shorts && h[s - 2] > h[s - 3] && h[s - 1] > h[s - 2] && h[s] < h[s - 1] && c[s] < ma[s] && o[d] < c[s] {
                let st = h[d + 1 - tr.min(d + 1)..=d].iter().copied().fold(f64::NEG_INFINITY, f64::max);
                pos = Some((-1., d, c[d], st));
            }
        }
        if let Some((dir, e, ep, _)) = pos { out.push(t(dir, e, ep, "close", to, c[to], "open", true, to)); }
        return Some(out);
    }
    // ATR system
    let (nh, ne, mult, nma, nsm, band) = (g("hh", 84.).round().max(2.) as usize, g("ema", 8.).round().max(1.) as usize, g("mult", 4.), g("ma", 144.).round().max(2.) as usize, g("sm", 21.).round().max(1.) as usize, g("band", 3.) / 100.);
    let tick = x.market_tick();
    let m = c.len();
    let mut pv = vec![f64::NAN; m]; // P
    let a = 2.0 / (ne as f64 + 1.0);
    for i in 1..m {
        let trv = mult * (h[i].max(c[i - 1]) - l[i].min(c[i - 1]));
        pv[i] = if pv[i - 1].is_nan() { trv } else { pv[i - 1] + a * (trv - pv[i - 1]) };
    }
    let ma = sma_v(c, nma);
    let rr: Vec<f64> = (0..m).map(|i| c[i] / ma[i]).collect();
    let sm = sma_v(&rr, nsm);
    let mut pos: Option<(f64, usize, f64, f64)> = None; // dir, entry bar, entry px, price stop
    let mut exit_next: Option<usize> = None; // signal bar of a next-open volatility exit
    for d in from..=to {
        if let Some((dir, e, ep, st)) = pos {
            if let Some(sig) = exit_next {
                out.push(t(dir, e, ep, "limit", d, o[d], "volatility stop", false, sig)); pos = None; exit_next = None;
                // flat again from this bar's close: fall through to look for a new entry only on later bars
                continue;
            }
            if d > e {
                if (dir > 0. && l[d] <= st) || (dir < 0. && h[d] >= st) {
                    out.push(t(dir, e, ep, "limit", d, if dir > 0. { o[d].min(st) } else { o[d].max(st) }, "trailing stop", false, d)); pos = None; continue;
                }
            }
            // close of day d: raise the stop, test the band
            let cand = if dir > 0. { c[d] - pv[d] } else { c[d] + pv[d] };
            let st = if d == e { if dir > 0. { ep - pv[d] } else { ep + pv[d] } } else if dir > 0. { st.max(cand) } else { st.min(cand) };
            pos = Some((dir, e, ep, st));
            if !sm[d].is_nan() && ((dir > 0. && rr[d] < sm[d] * (1. - band)) || (dir < 0. && rr[d] > sm[d] * (1. + band))) { exit_next = Some(d); }
            continue;
        }
        if d < nh + 1 { continue; }
        // stops placed at the close of d-1
        let hh = h[d - nh..d].iter().copied().fold(f64::NEG_INFINITY, f64::max) + tick;
        let ll = l[d - nh..d].iter().copied().fold(f64::INFINITY, f64::min) - tick;
        let (up, dn) = (h[d] >= hh, shorts && l[d] <= ll);
        let long_first = !dn || (up && (o[d] - hh).abs() <= (o[d] - ll).abs());
        if up && long_first {
            let ep = o[d].max(hh); pos = Some((1., d, ep, ep - pv[d])); // provisional: replaced at this bar's close
        } else if dn {
            let ep = o[d].min(ll); pos = Some((-1., d, ep, ep + pv[d]));
        }
        if let Some((dir, e, ep, _)) = pos { if e == d {
            let st = if dir > 0. { ep - pv[d] } else { ep + pv[d] }; // entry minus/plus P at the entry day's close
            pos = Some((dir, e, ep, st));
            if !sm[d].is_nan() && ((dir > 0. && rr[d] < sm[d] * (1. - band)) || (dir < 0. && rr[d] > sm[d] * (1. + band))) { exit_next = Some(d); }
        } }
    }
    if let Some((dir, e, ep, _)) = pos { out.push(t(dir, e, ep, "limit", to, c[to], "open", true, to)); }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sane_runs() { super::super::nijat::tests::sane(run, &[REV, ATR]); }
    /// hand-run dump for the pandas reference: FRMTM_CSV=<idx_data.csv> FRMTM_OUT=<file> cargo test --release frmtm_dump -- --ignored
    #[test]
    #[ignore]
    fn frmtm_dump() {
        use std::io::Write;
        let data = crate::load_csv(&std::env::var("FRMTM_CSV").unwrap()).unwrap();
        let mut f = std::io::BufWriter::new(std::fs::File::create(std::env::var("FRMTM_OUT").unwrap()).unwrap());
        writeln!(f, "id,ticker,dir,entry_date,entry_px,exit_date,exit_px,open").unwrap();
        for (tk, rows) in data.iter() {
            let x = Ctx::new(rows, &[], &[]);
            for id in REVIEWED {
                for z in run(id, &x, 0, rows.len() - 1, &Params::new()).unwrap() {
                    writeln!(f, "{id},{tk},{},{},{},{},{},{}", z.dir, rows[z.legs[0].i].date, z.legs[0].px, rows[z.exit_i].date, z.exit_px, z.open).unwrap();
                }
            }
        }
    }
}

pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "frmtm",
    label: "Daily futures systems written without a currency amount (a one-tick offset only); run and independently checked on the five NSE index series (NIFTY 50, NIFTYBANK, NIFTYIT, NIFTYMIDCAP100, NIFTYMIDCAPSELECT) as a declared application of the book's commodity and currency futures.",
    basis: "Freeburg, Formula Research V:9 (21 Jun 1997): the simple reversal system and Bagsarian's ATR system; independent check authors/conv-1123/k3w/frmtm_ref.py",
};
