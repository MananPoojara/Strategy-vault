//! Tom Bierovic, *Playing for Keeps in Stocks & Futures* (Wiley, 2002).
//! All three setups (First Prize, R2D2, Triple Play) have objective tools but leave the exit to a
//! discretionary trailing stop ("trail a stop ... while still giving the market some room to run"),
//! and the pattern anchors (impulse wave start, divergence lows, the three declines) are chosen by eye.
//! No exit is added. Source and transcription: library/tbpk/.
//! Lane R2D (13 Nov 2026, R4 #297/#304): First Prize is coded with two labelled readings (impulse start, trailing stop);
//! R2D2 and Triple Play stay held (pattern points chosen by eye, #297 (2)).
use crate::connors::{true_range, wilder, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};
pub const REVIEWED: &[&str] = &["tbpk_first_prize"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

pub const BOOK: &str = "tbpk";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, why, rules, notes| StrategyDef {
        id, name, group, session: 0, dir: "Long/Short", status: Status::NotTestable(why), needs_volume: false,
        needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None,
    };
    vec![
        StrategyDef { status: Status::Live, ..d("tbpk_first_prize", "First Prize", "Ch 4 · First Prize", "",
          &["F|Tools: candlestick chart, 13- and 34-day EMAs, 13-day DMI spread, 3-8-13 MACD, 21-day ATR (p.68)",
            "F|Long setup: 1. 13-day EMA above 34-day EMA. 2. The market makes a new 21-day high, suggesting an impulse wave. 3. The DMI spread is greater than +15 on the day the market makes its most recent 21-day high. 4. Today's close is between the 38.2% and 61.8% retracements of the impulse wave. 5. The MACD line crosses below its signal line for the first time since the signal line crossed above zero, and the signal line remains above zero. 6. The MACD histogram is rising (greater than yesterday). (p.69)",
            "E|\"Buy tomorrow above today's high\" (p.69): one tick above (\"any small amount above the high (e.g., 12 cents for a stock or one tick for a commodity)\"); a gap above fills at the open; the order lives one day",
            "X|\"1. Place an initial protective stop below the low of the corrective wave\" (one tick below the lowest low since the 21-day high) \"2. As prices rise, trail a stop\" (p.69-70). Short setup mirror (p.70-71)",
            "F|Unstated in the book: where the impulse wave starts; vault reading: the lowest low (shorts: highest high) of the 21 bars ending at the 21-day high bar, the book's 'usually' (p.64-67)",
            "X|Unstated in the book: the trailing stop; vault reading: the first worked example's (Aflac, p.72-73): highest close of the trade minus 2 x ATR(21) of the day before entry, raised after each close (p.atr_mult)",
            "X|Unstated in the book: a new setup on the day a trailing or initial stop is hit; vault reading: no new entry on a day the position was stopped out after the open (the daily bar cannot show whether the exit or the new entry stop came first); a stop gapped through at the open comes first and still allows one (lane X2)",
            "F|Adapted: 2 unstated details (R4 #297, #304). Entry-day order of the entry stop and the initial stop on the real 1-minute path where minutes exist (EP6, p.order)"],
          "Readings (declared): EMAs alpha 2/(n+1) seeded with the first close; DMI spread = +DI(13) - -DI(13), Wilder; MACD = EMA3 - EMA8, signal = EMA13 of the MACD, histogram = MACD - signal; ATR(21) = mean of 21 true ranges; a 21-day high = a high above the 20 before it; the setup holds on any close where every condition holds (the worked examples: the MACD cross first, the histogram turn later, p.72). One position at a time; long checked before short. The book's other examples choose exits after the fact (p.71) and print no statistics.") },
        d("tbpk_r2d2", "R2D2 (reverse divergence)", "Ch 5 · R2D2",
          "the reverse-divergence points A, B, C have no numeric swing definition (p.97-101) and after breakeven the book says 'trail a tight protective stop ... while still giving the market some room to run' (p.102) with no rule",
          &["F|Tools: 55-day EMA, 13-day DMI spread, 3-8-13 MACD (p.101)",
            "F|Long setup: 1. Closing prices above the EMA throughout the setup. 2. MACD line forms a bullish reverse divergence (price low C above low A, MACD at C below MACD at A). 3. DMI spread > +15 at point B. 4. MACD histogram higher than yesterday. (p.101-102)",
            "E|\"Buy today above yesterday's high\" (p.102)",
            "X|Initial stop below the higher of the two lows (point C) or the low of the entry day, whichever is lower; after a close above the highest high between the two lows raise the stop to breakeven and trail a tight stop (p.102). Short mirror (p.102-103)"],
          "Worked examples use different exits per trade (2-day low, trendline, DMI spread, MACD cross, measured move, parabolic)."),
        d("tbpk_triple_play", "Triple Play (multiple divergence)", "Ch 6 · Triple Play",
          "the three (or more) declines/rallies and the multiple divergence are identified on the chart with no numeric definition (p.135-139, Figs 6.1-6.2), entries may come early from 'gaps, candles, trendlines', and the exit is a discretionary trailing stop (p.139)",
          &["F|Tools: candlestick chart, EMA channel of the 13-day EMA of highs and 13-day EMA of lows, 3-8-13 MACD (p.139)",
            "F|Long setup: 1. The lows of the three (or more) declines are all below the EMA channel. 2. The MACD line forms a multiple bullish divergence (p.139)",
            "E|\"1. Buy when prices penetrate the top of the EMA channel. 2. Consider gaps, candles and trendlines for an earlier entry\" (p.139)",
            "X|\"1. Initial stop below the low of candle C. 2. Trail a stop to lock in some profits while still giving the market some room to run.\" (p.139-140). Short mirror (p.140-141)"],
          "Chart-reading pattern; no exit is added."),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id {
        "tbpk_first_prize" | "tbpk_r2d2" | "tbpk_triple_play" => Some(("tbpk_setups", "Bierovic setups", "Not testable")),
        _ => None,
    }
}
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    if id != "tbpk_first_prize" { return None; }
    let mut v = vec![ParamDef { key: "atr_mult", label: "Trailing stop [unstated, R4 #297 reading, Aflac example]: highest close - this x ATR(21) of the day before entry", def: 2.0, min: 0.5, max: 6.0, step: 0.5 }];
    crate::intraday_ep6::add_param(id, &mut v);
    Some(v)
}

fn ema_first(v: &[f64], n: usize) -> Vec<f64> {
    let a = 2.0 / (n as f64 + 1.0);
    let mut e = f64::NAN;
    v.iter().map(|&x| { e = if e.is_nan() { x } else { e + a * (x - e) }; e }).collect()
}

/// Wilder +DI(n) - -DI(n) (the same DMI as books::ss dmi3).
fn dmi_spread(x: &Ctx, n: usize) -> Vec<f64> {
    let r = x.r;
    let len = r.len();
    let (mut pdm, mut mdm, mut tr) = (vec![f64::NAN; len], vec![f64::NAN; len], vec![f64::NAN; len]);
    let trs = true_range(r);
    for i in 1..len {
        let up = r[i].high - r[i - 1].high;
        let dn = r[i - 1].low - r[i].low;
        pdm[i] = if up > dn && up > 0.0 { up } else { 0.0 };
        mdm[i] = if dn > up && dn > 0.0 { dn } else { 0.0 };
        tr[i] = trs[i];
    }
    let (sp, sm, st) = (wilder(&pdm, n), wilder(&mdm, n), wilder(&tr, n));
    (0..len).map(|i| if st[i] > 0.0 { 100.0 * (sp[i] - sm[i]) / st[i] } else { f64::NAN }).collect()
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    use crate::intraday_ep6::{bar_segs, mark, resolve, ts_high_first, Day, Ev, Seg};
    let schema = params(id)?;
    if from > to || to >= x.c.len() { return None; }
    for k in p.keys() { if !schema.iter().any(|d| d.key == k) { return None; } }
    let Some((from, to)) = crate::intraday_ep6::window(id, x, from, to, p) else { return Some(vec![]) };
    let km = p.get("atr_mult").copied().unwrap_or(2.0);
    let on = crate::intraday_ep6::on(id, p);
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let n = c.len();
    let tick = x.market_tick();
    let (e13, e34) = (ema_first(c, 13), ema_first(c, 34));
    let sp = dmi_spread(x, 13);
    let (e3, e8) = (ema_first(c, 3), ema_first(c, 8));
    let m: Vec<f64> = e3.iter().zip(&e8).map(|(a, b)| a - b).collect();
    let sg = ema_first(&m, 13);
    let hs: Vec<f64> = m.iter().zip(&sg).map(|(a, b)| a - b).collect();
    let trs = true_range(x.r);
    let atr: Vec<f64> = (0..n).map(|i| if i >= 20 { trs[i - 20..=i].iter().sum::<f64>() / 21.0 } else { f64::NAN }).collect();
    let setup = |t: usize, d: f64| -> Option<(f64, f64)> {
        if t < 41 || !sp[t].is_finite() || !(d * (e13[t] - e34[t]) > 0.0 && d * (hs[t] - hs[t - 1]) > 0.0) { return None; }
        let bj = (21..=t).rev().find(|&j| { let w = &x.r[j - 20..j]; if d > 0.0 { h[j] > w.iter().map(|r| r.high).fold(f64::NEG_INFINITY, f64::max) } else { l[j] < w.iter().map(|r| r.low).fold(f64::INFINITY, f64::min) } })?;
        if bj == t || !sp[bj].is_finite() || !(d * sp[bj] > 15.0) { return None; }
        let (top, a) = if d > 0.0 { (h[bj], l[bj - 20..=bj].iter().cloned().fold(f64::INFINITY, f64::min)) } else { (l[bj], h[bj - 20..=bj].iter().cloned().fold(f64::NEG_INFINITY, f64::max)) };
        let rng = top - a;
        let (z1, z2) = (top - d * 0.618 * rng, top - d * 0.382 * rng);
        let (lo, hi) = if z1 <= z2 { (z1, z2) } else { (z2, z1) };
        if !(lo <= c[t] && c[t] <= hi) { return None; }
        let z = (1..=t).rev().find(|&j| d * sg[j] > 0.0 && !(d * sg[j - 1] > 0.0))?;
        if !(d * sg[t] > 0.0) { return None; }
        if !((z + 1)..=t).any(|k| d * (m[k] - sg[k]) < 0.0 && !(d * (m[k - 1] - sg[k - 1]) < 0.0)) { return None; }
        let cw = if d > 0.0 { l[bj + 1..=t].iter().cloned().fold(f64::INFINITY, f64::min) } else { h[bj + 1..=t].iter().cloned().fold(f64::NEG_INFINITY, f64::max) };
        Some(if d > 0.0 { (h[t] + tick, cw - tick) } else { (l[t] - tick, cw + tick) })
    };
    let mk = |d: f64, e: usize, px: f64, k: usize, xp: f64, why: &str, open: bool| Trade { dir: d, legs: vec![Leg { i: e, px, w: 1.0 }], exit_i: k, exit_px: xp, reason: why.into(), entry_kind: "stop", signal_i: k, open, exits: vec![] };
    let mut out = vec![];
    // position: dir, entry bar, entry px, stop, best close, ATR(21) of the day before entry, entry-day mark
    let mut pos: Option<(f64, usize, f64, f64, f64, f64, Day)> = None;
    for k in from.max(1)..=to {
        // lane X2: a stop hit after the open leaves its order against a new entry stop the same day unknown on a daily bar
        // (held days are not walked on minutes), so no new entry that day; an exit at the open came first and allows one
        let mut stopped = false;
        if let Some((d, e, px, stop, _, _, dm)) = pos {
            if k > e {
                let hit = if d * (o[k] - stop) <= 0.0 { Some((o[k], "stop (gapped open)")) } else if (d > 0.0 && l[k] <= stop) || (d < 0.0 && h[k] >= stop) { Some((stop, "stop")) } else { None };
                if let Some((xp, why)) = hit { let mut t = mk(d, e, px, k, xp, why, false); mark(&mut t, dm); out.push(t); pos = None; stopped = why == "stop"; }
            }
        }
        if pos.is_none() && !stopped && k > from.max(1) {
            for d in [1.0, -1.0] {
                let Some((lv, st0)) = setup(k - 1, d) else { continue };
                if !((d > 0.0 && h[k] >= lv) || (d < 0.0 && l[k] <= lv)) { continue; }
                let gapped = if d > 0.0 { o[k] >= lv } else { o[k] <= lv };
                let px = if gapped { o[k] } else { lv };
                // entry-day order: (entered, exit price) walked along the day's path; EP6 resolves it on minutes
                let walk = |s: &mut (bool, f64, f64), segs: &[Seg]| -> Vec<Ev> {
                    let mut ev = vec![];
                    for g in segs {
                        if !s.0 { if g.up() == (d > 0.0) && g.inside(lv) { s.0 = true; s.2 = g.fill(lv); ev.push(Ev { k: 0, px: s.2, up: d > 0.0 }); } }
                        else if s.1.is_nan() && g.up() != (d > 0.0) && g.inside(st0) { s.1 = g.fill(st0); ev.push(Ev { k: 1, px: s.1, up: d < 0.0 }); }
                    }
                    ev
                };
                let mut s = (gapped, f64::NAN, px);
                let hf = ts_high_first(o[k], h[k], l[k]);
                let day = if gapped { walk(&mut s, &bar_segs(o[k], h[k], l[k], c[k], hf)); Day::Plain } else { resolve(x, k, on, hf, &mut s, |z, hf| walk(z, &bar_segs(o[k], h[k], l[k], c[k], hf)), &walk) };
                if !s.0 { break; }
                let px = s.2;
                if s.1.is_finite() { let mut t = mk(d, k, px, k, s.1, "initial stop (entry day)", false); mark(&mut t, day); out.push(t); }
                else { pos = Some((d, k, px, st0, c[k], atr[k - 1], day)); }
                break;
            }
        }
        if let Some((d, e, px, mut stop, mut best, a0, dm)) = pos {
            best = if d > 0.0 { best.max(c[k]) } else { best.min(c[k]) };
            if a0.is_finite() { let tl = best - d * km * a0; stop = if d > 0.0 { stop.max(tl) } else { stop.min(tl) }; }
            pos = Some((d, e, px, stop, best, a0, dm));
        }
    }
    if let Some((d, e, px, _, _, _, dm)) = pos { let mut t = mk(d, e, px, to, c[to], "open", true); mark(&mut t, dm); out.push(t); }
    Some(out)
}

#[cfg(test)]
mod tests {
    #[test]
    fn nothing_is_live_and_nothing_runs() {
        let c = super::catalogue();
        assert_eq!(c.len(), 3);
        // lane R2D (R4 #304): First Prize live with labelled readings; R2D2 and Triple Play held
        assert!(c.iter().all(|d| matches!(d.status, crate::connors::Status::NotTestable(_)) != (d.id == "tbpk_first_prize")));
        assert!(super::reviewed("tbpk_first_prize") && super::params("tbpk_r2d2").is_none());
        // R4 #308 / lane X2: refused above 5% order-dependent (X2w/tbpk_share.tsv), on any unmeasured ticker and on single stocks
        for t in ["DIA", "IWM", "DJI", "XLF", "XLE", "MDY", "TLT", "AAPL", "MSFT", "RELIANCE", "UNKNOWN"] { assert!(crate::intraday_ep6::held_note("tbpk_first_prize", t).is_some(), "{t}"); }
        for t in ["SPY", "QQQ", "SPX", "NDX", "NIFTY 50", "NIFTYBANK", "NIFTY BANK", "GLD", "XLY"] { assert!(crate::intraday_ep6::held_note("tbpk_first_prize", t).is_none(), "{t}"); }
        let def = c.iter().find(|d| d.id == "tbpk_first_prize").unwrap();
        assert!(crate::instrument_policy::tradable_check_issue(def, "AAPL", true).is_some_and(|i| i.status == "not_testable" && i.note.contains("single stock")));
        assert!(crate::instrument_policy::tradable_check_issue(def, "SPY", false).is_none());
    }

    /// Lane X2: no new entry on a day the position was stopped out after the open. NIFTY 50 (VAULT_IDX, convention, no minutes):
    /// the reference (R2Dw/tbpk_ref.py) found the old rule re-entering long on 2023-09-28 and 2024-01-23 after a stop that day.
    #[test]
    fn no_new_entry_on_a_stop_day() {
        use crate::connors::{Ctx, Params};
        let path = std::env::var("VAULT_IDX").unwrap_or_else(|_| concat!(env!("CARGO_MANIFEST_DIR"), "/../../../idx_data.csv").into());
        let data = crate::load_csv(&path).unwrap_or_else(|e| panic!("this test needs real data: set VAULT_IDX to idx_data.csv ({e})"));
        let rows = &data["NIFTY 50"];
        let x = Ctx::new(rows, rows, &[]);
        let t = super::run("tbpk_first_prize", &x, 0, rows.len() - 1, &Params::from([("order".to_string(), 0.0)])).unwrap();
        let stop_days: Vec<usize> = t.iter().filter(|z| !z.open && z.reason.starts_with("stop") && !z.reason.starts_with("stop (gapped")).map(|z| z.exit_i).collect();
        for d in ["2023-09-28", "2024-01-23"] {
            let i = rows.iter().position(|r| r.date == d).unwrap();
            assert!(stop_days.contains(&i), "{d}: a stop exit that day");
        }
        for z in &t { assert!(!stop_days.contains(&z.legs[0].i), "entry on a stop day {}", rows[z.legs[0].i].date); }
    }
}

// Instrument admission (lane R2D, 13 Nov 2026, R4 #297/#304): First Prize only.
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "tbpk" && crate::books::tbpk::reviewed(def.id),
    label: "Bierovic shows First Prize on stock and futures charts (Aflac, Southtrust and others); runs on indices and ETFs are a declared application, one unit per trade, with the entry-day order of the entry stop and the initial stop on real 1-minute bars where the minute files exist. Published only on the indices and ETFs where at most 5% of trades depend on that order (decisions #175, #240, #308); single stocks are refused: no stock has 1-minute files and the share is not measured per stock.",
    basis: "Tom Bierovic, Playing for Keeps in Stocks & Futures (Wiley, 2002), ch.4 First Prize p.64-73 (PDF p.41-45)",
};
