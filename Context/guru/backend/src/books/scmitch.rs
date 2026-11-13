//! Lane PE (13 Nov 2026; released by lane PC, authors/pc/PC.tsv 'CODABLE-RELEASED'; R4 #297, #240, #319, #325):
//! Clifton Mitchell, "Position Trading The S&P", Technical Analysis of Stocks & Commodities V.19:10 (2001) pp.26-33
//! (research share 'Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V19/C10/109POS.pdf').
//! Book id `scmitch`. Coded: the linear-regression-of-lows setup with the close-stop entry and the three optimised exits
//! (Figure 2 and 'Optimization', PDF p.3-4), on the real 1-minute path (EP6) of SPY/SPX only (minute window: lead decision R4 #325, #240 (3) + the 15-25% same-day share).
//! scmitch_atr is HELD (R4 #325: 3 unstated details); its run code stays for a re-admission.
//! Independent reference: authors/pe/w/pe_ref.py (from the article).
use crate::connors::{true_range, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};
use crate::intraday_ep6::{bar_segs, remark, resolve, ts_high_first, Day, Ev, Seg};

pub const IDS: &[&str] = &["scmitch_lowstop", "scmitch_lrexit", "scmitch_atr"];
pub fn reviewed(id: &str) -> bool { IDS.contains(&id) }

macro_rules! common { () => { " Linear regression value = the least-squares line through the last X lows, read at the current bar (the article's target 0; TradeStation LinearRegValue). Setup: today's low above yesterday's regression value of the last X lows. Entry: from the next day, a buy stop at the previous day's close, renewed each day until it fills (a gap open above it fills at the open). Exits are stop orders computed from yesterday's bar; a stop the open gaps through fills at the open, and a stop already at or above the entry fill on the entry day exits at the fill. Long only, one position. Unstated in the article (R4 #297): (1) a setup while a position is open does not carry over (vault reading: only a setup on a day that ends flat arms the close stop); (2) whether the exit works on the entry day (vault reading: yes, as TradeStation/SuperCharts exit orders on the entry bar; option p.same_day = 0: from the day after). Mitchell tested the S&P 500 index January 1995 - May 2001 with 0.5 point slippage and commission per trade. Published only on the real 1-minute window of SPY (2000-2025) (the low-stop SPX run has 5.66% ambiguous minutes, #175; the regression-exit SPX run fails #177a; the ATR exit is held, R4 #325): SPX daily opens are stale before 2014 (#240 (3)) and the stop entry and the stop exit trade the same day on 15-25% of trades (lead decision R4 #325); the daily convention is kept as p.order = 0 (book check). One unit per trade, no costs." } }

/// R4 #325: the ATR exit has a third unstated detail (its reference day), one more than #297 admits.
const ATR_HELD: &str = "withheld (R4 #325, held under #297): the ATR exit has 3 unstated details, one more than #297 admits: (1) a setup while a position is open, (2) the exit on the entry day, and (3) the exit's reference day - the article says only 'exit if today's price < highest high of last X days - (ATR of last X days times multiplier)' (PDF p.3); the vault's intraday stop from yesterday's 4-day high and ATR(8) is a reading (yesterday vs today, a stop vs a close test). No results are published; the low-stop and regression-exit variants stay admitted.";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, x: &'static str, note| StrategyDef { id, name, group: "S&C: Position Trading The S&P (Mitchell, V.19:10)", session: 0, dir: "Long", status: if id == "scmitch_atr" { Status::NotTestable(ATR_HELD) } else { Status::Live },
        needs_volume: false, needs_vix: false, needs_series: &[], book: "scmitch", claim: None, notes: note,
        rules: match x {
            "low" => &["E|\"If today's low > yesterday's linear regression of last X lows (buy setup), then buy at subsequent close stop.\" (PDF p.3); \"the entry linear regression lookback period optimized at nine days\" (PDF p.3)",
                "X|\"exit if the price reaches or goes below the low of the previous day\" (PDF p.3)",
                "F|Figure 2 (S&P 500 index 1995-2001): 341 trades, 40% profitable, 1,201.47 points"] as &[&str],
            "lr" => &["E|Setup and close-stop entry as above; \"the entry regression lookback period optimized at six days\" (PDF p.3)",
                "X|\"you exited your position if today's low dropped below yesterday's linear regression price (target), calculated from the last X days' lows\"; \"the exit lookback period optimizing at three days\" (PDF p.3)",
                "F|Figure 2: 345 trades, 43% profitable, 1,217.61 points"],
            _ => &["E|Setup and close-stop entry as above; \"an entry regression lookback period of six days\" (PDF p.4)",
                "X|\"exit if today's price < highest high of last X days - (ATR of last X days times multiplier)\" (PDF p.3); \"an ATR exit lookback period of eight days, multiplied by a factor of 1.2 and subtracted from the highest high of the last four days\" (PDF p.4)",
                "F|Figure 2: 224 trades, 47% profitable, 1,206.67 points"],
        } };
    vec![
        d("scmitch_lowstop", "Mitchell regression-of-lows entry (9 days), close-stop entry, exit at the previous day's low", "low", concat!("[Adapted: 2 unstated details, R4 #297; unverified] Exit: a stop at the previous day's low.", common!())),
        d("scmitch_lrexit", "Mitchell regression-of-lows entry (6 days), exit below yesterday's 3-day regression of lows", "lr", concat!("[Adapted: 2 unstated details, R4 #297; unverified] Exit: a stop at yesterday's regression value of the last 3 lows.", common!())),
        d("scmitch_atr", "Mitchell regression-of-lows entry (6 days), exit below the 4-day high minus 1.2 x ATR(8)", "atr", concat!("[Held: 3 unstated details, R4 #325] Exit (reading of the unstated reference day): a stop at yesterday's highest high of the last 4 days minus 1.2 x the 8-day average true range (simple average of true ranges, TradeStation AvgTrueRange).", common!())),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    reviewed(id).then_some(("scmitch_lr", "Mitchell regression-of-lows S&P system (S&C)", "three optimised exits"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    if !reviewed(id) { return None; }
    let x = if id == "scmitch_lowstop" { 9. } else { 6. };
    let mut v = vec![
        ParamDef { key: "x", label: "Entry regression lookback X (days of lows; the article's optimised value)", def: x, min: 3., max: 30., step: 1. },
        ParamDef { key: "same_day", label: "Exit stop on the entry day [unstated, R4 #297]: 1 = active (TradeStation), 0 = from the day after", def: 1., min: 0., max: 1., step: 1. },
    ];
    crate::intraday_ep6::add_param(id, &mut v);
    Some(v)
}

/// Least-squares line through v[t+1-n..=t] read at t (NaN while warming up).
pub fn lr_value(v: &[f64], n: usize) -> Vec<f64> {
    let xm = (n as f64 - 1.) / 2.;
    let sxx: f64 = (0..n).map(|i| (i as f64 - xm).powi(2)).sum();
    (0..v.len()).map(|t| if t + 1 < n || n < 2 { f64::NAN } else {
        let w = &v[t + 1 - n..=t];
        let ym = w.iter().sum::<f64>() / n as f64;
        let m = w.iter().enumerate().map(|(i, y)| (i as f64 - xm) * (y - ym)).sum::<f64>() / sxx;
        ym + m * xm }).collect()
}

/// pos: in a trade (entry bar, price, mark); done: closed today (entry bar, price, exit price, mark, reason).
#[derive(Clone)]
struct St { pos: Option<(usize, f64, Day)>, done: Vec<(usize, f64, f64, Day, &'static str)> }

fn walk(s: &mut St, segs: &[Seg], buy: Option<f64>, stop: f64, live: bool, k: usize) -> Vec<Ev> {
    let mut ev = vec![];
    let mut entered = s.pos.is_some_and(|p| p.0 == k);
    for g in segs {
        let mut cur = g.a;
        loop {
            let seg = Seg { a: cur, b: g.b, gap: g.gap };
            if s.pos.is_none() && !entered { if let Some(b) = buy { if seg.up() && seg.inside(b) {
                let px = seg.fill(b);
                s.pos = Some((k, px, Day::Plain)); entered = true; ev.push(Ev { k: 0, px, up: true });
                if live && stop >= px { s.done.push((k, px, px, Day::Plain, "stop (at or above the entry fill)")); s.pos = None; ev.push(Ev { k: 1, px, up: false }); }
                cur = if g.gap { g.b } else { b }; continue; } } }
            if let Some((e, px0, m)) = s.pos { if (e < k || live) && !seg.up() && seg.inside(stop) {
                let px = seg.fill(stop);
                s.done.push((e, px0, px, m, "exit stop")); s.pos = None; ev.push(Ev { k: 1, px, up: false });
                cur = if g.gap { g.b } else { stop }; continue; } }
            break;
        }
    }
    ev
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if from > to || to >= x.c.len() { return None; }
    for k in p.keys() { if !schema.iter().any(|d| d.key == k) { return None; } }
    let Some((from, to)) = crate::intraday_ep6::window(id, x, from, to, p) else { return Some(vec![]) };
    let xn = p.get("x").copied().unwrap_or(if id == "scmitch_lowstop" { 9. } else { 6. }).round().max(2.) as usize;
    let same_day = p.get("same_day").copied().unwrap_or(1.) >= 0.5;
    let on = crate::intraday_ep6::on(id, p);
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let n = c.len();
    let lr = lr_value(l, xn);
    let lr3 = lr_value(l, 3);
    let tr = true_range(x.r);
    let stop_at = |t: usize| -> f64 { match id {
        "scmitch_lowstop" => l[t],
        "scmitch_lrexit" => lr3[t],
        _ => if t >= 8 { h[t - 3..=t].iter().cloned().fold(f64::NEG_INFINITY, f64::max) - 1.2 * tr[t - 7..=t].iter().sum::<f64>() / 8. } else { f64::NAN },
    } };
    let mut out = vec![];
    let mut s = St { pos: None, done: vec![] };
    let mut armed = false;
    for k in from..=to.min(n - 1) {
        if k > from {
            let t = k - 1;
            let buy = (armed && s.pos.is_none()).then_some(c[t]);
            let stop = stop_at(t);
            if s.pos.is_some() || buy.is_some() {
                // the open: a held position's stop the open reaches fills at the open; then the entry stop
                if let Some((e, px0, m)) = s.pos { if o[k] <= stop { s.done.push((e, px0, o[k], m, "exit stop (open)")); s.pos = None; } }
                let mut opened = false;
                if let Some(b) = buy { if o[k] >= b { s.pos = Some((k, o[k], Day::Plain)); opened = true;
                    if same_day && stop >= o[k] { s.done.push((k, o[k], o[k], Day::Plain, "stop (at or above the entry fill)")); s.pos = None; } } }
                let buy_left = if opened { None } else { buy.filter(|_| s.pos.is_none()) };
                let hf = ts_high_first(o[k], h[k], l[k]);
                let stop_live = stop.is_finite();
                let day = resolve(x, k, on, hf, &mut s, |z, hf| walk(z, &bar_segs(o[k], h[k], l[k], c[k], hf), buy_left, if stop_live { stop } else { f64::NAN }, same_day, k), |z, g| walk(z, g, buy_left, if stop_live { stop } else { f64::NAN }, same_day, k));
                if let Some(pz) = s.pos.as_mut() { pz.2 = if pz.0 == k { day } else { pz.2.worst(day) }; }
                if s.done.iter().any(|d| d.0 == k) || s.pos.is_some_and(|z| z.0 == k) { armed = false; }
                for (e, px, xp, m, why) in s.done.drain(..) {
                    let mut t = Trade { dir: 1., legs: vec![Leg { i: e, px, w: 1. }], exit_i: k, exit_px: xp, reason: why.into(), entry_kind: "stop", signal_i: k, open: false, exits: vec![] };
                    remark(&mut t, m); remark(&mut t, day); out.push(t);
                }
            }
        }
        // setup at today's close, counted only when the day ends flat (reading 1)
        if k >= 1 && s.pos.is_none() && lr[k - 1].is_finite() && l[k] > lr[k - 1] { armed = true; }
    }
    if let Some((e, px, m)) = s.pos { let mut t = Trade { dir: 1., legs: vec![Leg { i: e, px, w: 1. }], exit_i: to, exit_px: c[to], reason: "open".into(), entry_kind: "stop", signal_i: to, open: true, exits: vec![] }; remark(&mut t, m); out.push(t); }
    Some(out)
}

#[cfg(test)]
mod tests {
    use crate::connors::{Ctx, Params};
    use crate::Row;
    #[test]
    fn regression_value_is_the_fitted_line_at_the_last_bar() {
        let v = super::lr_value(&[1., 2., 3., 10.], 3); // last 3: 2, 3, 10 -> slope 4, mean 5, value at the end 9
        assert!(v[1].is_nan() && (v[2] - 3.).abs() < 1e-12 && (v[3] - 9.).abs() < 1e-12);
    }
    /// A setup arms the close stop; it fills the next day at the previous close, and the previous day's low stop exits.
    #[test]
    fn close_stop_entry_and_low_stop_exit() {
        let r = |d: usize, o: f64, h: f64, l: f64, c: f64| Row { ticker: "SPY".into(), date: format!("2001-02-{d:02}"), open: o, high: h, low: l, close: c, volume: None };
        // falling lows (regression falls), then day 10's low 93 is above the regression of the 9 lows to day 9
        let mut v: Vec<Row> = (1..=9).map(|d| { let p = 110. - 2. * d as f64; r(d, p + 1., p + 2., p, p + 1.) }).collect();
        v.push(r(10, 94., 96., 93., 95.)); // setup (93 > yesterday's regression value 92)
        v.push(r(11, 94.5, 96., 94., 95.5)); // buy stop at 95 fills at 95 (exit stop = day 10's low 93, not hit)
        v.push(r(12, 95., 95.5, 92., 92.5)); // stop at day 11's low 94: exit at 94
        let x = Ctx::new(&v, &[], &[]);
        let t = super::run("scmitch_lowstop", &x, 0, v.len() - 1, &Params::from([("order".to_string(), 0.)])).unwrap();
        assert_eq!(t.iter().map(|z| (z.legs[0].i, z.legs[0].px, z.exit_i, z.exit_px)).collect::<Vec<_>>(), vec![(10, 95., 11, 94.)]);
        for tk in ["SPY", "SPX"] { assert!(crate::intraday_ep6::held_note("scmitch_atr", tk).is_none()); }
        assert!(crate::intraday_ep6::held_note("scmitch_atr", "QQQ").is_some());
        // R4 #325: the ATR exit is held (3 unstated details); the other two exits stay live
        for d in super::catalogue() { assert_eq!(matches!(d.status, crate::connors::Status::NotTestable(_)), d.id == "scmitch_atr", "{}", d.id); assert_eq!(crate::gate::get(d.id).unwrap().verdict == "withheld", d.id == "scmitch_atr"); }
    }
}

pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "scmitch" && reviewed(def.id),
    label: "Mitchell tested the S&P 500 index (1995-2001, points). SPY and SPX are a declared application with prices of the traded series, and the same-day order of the close-stop entry and the exit stop is settled on their real 1-minute bars; only the minute window is published (lead decision R4 #325: #240 (3) and the 15-25% same-day share). Other instruments have no minute files and are refused.",
    basis: "Technical Analysis of Stocks & Commodities V.19:10 pp.26-33, Figure 2 and 'Optimization' (PDF p.3-4); R4 #297, #240, #325; independent check authors/pe/w/pe_ref.py",
};
