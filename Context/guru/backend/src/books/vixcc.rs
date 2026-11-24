//! Connors Group, "VIX Options Trading Course" (2007) slides 73, 76, 78 (short-VIX sets RSI80, 4HC, 5HH) and 97, 99, 101 (long-VIX sets RSI20, 5LC, 5LL)
//! (research share Trading Courses ... vixclass.pdf, 119 slides). Book id `vixcc`. Lane K2 (24 Nov 2026, tier 5 row I608: the six sets no live card implements).
//! Independent check authors/conv-1123/k2w/qi/vixcc_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::tselhs::Book;

pub const REVIEWED: &[&str] = &["vixcc_rsi80", "vixcc_4hc", "vixcc_5hh", "vixcc_rsi20", "vixcc_5lc", "vixcc_5ll", "vixcc_percb2", "vixcc_down1", "vixcc_rsi5", "vixcc_dmi1", "vixcc_dmi2", "vixcc_dmi3", "vixcc_dmi5", "vixcc_v1l", "vixcc_v2l", "vixcc_up2l", "vixcc_cvr2l", "vixcc_cvr3s", "vixcc_cvr3l", "vixcc_cvr6s", "vixcc_cvr6l", "vixcc_cvr8s", "vixcc_cvr8l", "vixcc_cvr11s", "vixcc_cvr11l"];
/// the twelve VIX-moving-average and long-VIX sets (slides 62-71, 81-95), run by run_ma
const MA_SETS: &[&str] = &["vixcc_v1l", "vixcc_v2l", "vixcc_up2l", "vixcc_cvr2l", "vixcc_cvr3s", "vixcc_cvr3l", "vixcc_cvr6s", "vixcc_cvr6l", "vixcc_cvr8s", "vixcc_cvr8l", "vixcc_cvr11s", "vixcc_cvr11l"];
/// the seven price-only course sets (slides 34, 38, 48, 54, 56, 58, 60): index prices only, no VIX
const PRICE_SETS: &[&str] = &["vixcc_percb2", "vixcc_down1", "vixcc_rsi5", "vixcc_dmi1", "vixcc_dmi2", "vixcc_dmi3", "vixcc_dmi5"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "A|[Declared readings, lane K2, 3] Adaptation (R4 #340): the course trades VIX options with SPY as the filter instrument and names no equity vehicle for these signals beyond 'Long SPY'; here India VIX with NIFTY 50: the short-VIX sets (RSI80, 4HC, 5HH) buy NIFTY 50 when the SMA(200) filter is above, the long-VIX sets (RSI20, 5LC, 5LL) sell NIFTY 50 short below it. Unstated, counted: (1) RSI(2) is Wilder's, taken from the VIX close; (2) a run of N days includes the signal day, on days both the VIX and NIFTY traded; (3) entries and exits are at the close of the signal day (the slides give no order timing; Connors' convention).";

const NOTE_M: &str = "A|[Declared readings, lane K2] Adaptation (R4 #340): the course trades VIX options with SPY as the filter; here India VIX with NIFTY 50: the short-VIX sets BUY NIFTY 50 and the long-VIX sets (the course buys VIX options; no equity vehicle is named) SELL NIFTY 50 SHORT. Each card's own unstated details (at most 4 per card) are numbered in its gate row: moving-average window including the signal day, the percent tests, VIX highs/lows, run counts, EMA seeding, RSI smoothing. Followed as printed: entries and exits at the close; CVR11 long-VIX prints no SPY filter (none applied); the CVR2 short-VIX slide is not coded (entry and exit both read 'VIX closes below its 10-day MA'). Live near-twins: ats_cvr3 (CVR III) and ats_cvr2 (CVR II) from the 2001 book.";

fn catalogue_ma() -> Vec<StrategyDef> {
    let d = |id: &'static str, name: &'static str, dir: &'static str, rules: &'static [&'static str]| StrategyDef { id, name, group: "Connors VIX Options Trading Course: VIX moving-average and long-VIX sets (slides 62-95)", session: 0, dir, status: Status::Live,
        needs_volume: false, needs_vix: true, needs_series: &[], book: "vixcc", rules, notes: NOTE_M, claim: None };
    vec![
        d("vixcc_v1l", "V1 Long (long-VIX signal, sells NIFTY 50 short): index below its 200-day SMA, RSI(2) above 80, VIX 5% below its 10-day MA three days, close at or above the 5-day SMA: sell short; exit RSI(2) below 35", "Short", &["E|Slide 81: close below the 200-day MA; RSI(2) above 80; VIX close at least 5% below its 10-day MA three days in a row; close equal or above the 5-day MA", "X|Exit when RSI(2) closes below 35 (slide 81)", NOTE_M]),
        d("vixcc_v2l", "V2 Long (long-VIX signal, sells NIFTY 50 short): index below its 200-day SMA, RSI(2) above 80, VIX 10% below its 10-day MA, close at or above the 5-day SMA: sell short; exit RSI(2) below 35", "Short", &["E|Slide 83: close below the 200-day MA; RSI(2) above 80; VIX close at least 10% below its 10-day MA; close equal or above the 5-day MA", "X|Exit when RSI(2) closes below 35 (slide 83)", NOTE_M]),
        d("vixcc_up2l", "Up2 (long-VIX signal, sells NIFTY 50 short): index below its 200-day SMA, two-day rise over 3%, RSI(2) above 80, close at or above the 5-day SMA: sell short; exit RSI(2) below 35", "Short", &["E|Slide 85: close below the 200-day MA; rise over the last 2 days more than 3%; RSI(2) above 80; close equal or above the 5-day MA", "X|Exit when RSI(2) closes below 35 (slide 85)", NOTE_M]),
        d("vixcc_cvr2l", "CVR2 (long-VIX signal, sells NIFTY 50 short): VIX RSI(5) below 30 and rising, index below its 200-day SMA: sell short; exit when the VIX closes above its 10-day MA", "Short", &["E|Slide 87: VIX RSI(5) < 30; VIX RSI(5) today > yesterday; close below the 200-day MA", "X|Exit when the VIX closes above its 10-day MA (slide 87)", NOTE_M]),
        d("vixcc_cvr3s", "CVR3 (short-VIX): VIX low above its 10-day MA, VIX close 10% above it, index above its 200-day SMA: buy; exit when the VIX closes below its 10-day MA", "Long", &["E|Slide 64: VIX low greater than its 10-day MA; VIX close at least 10% above its 10-day MA; close above the 200-day MA", "X|Exit when the VIX closes below its 10-day MA (slide 64)", NOTE_M]),
        d("vixcc_cvr3l", "CVR3 (long-VIX signal, sells NIFTY 50 short): VIX high below its 10-day MA, VIX close 10% below it, index below its 200-day SMA: sell short; exit when the VIX closes above its 10-day MA", "Short", &["E|Slide 89: VIX high below its 10-day MA; VIX close at least 10% below its 10-day MA; close below the 200-day MA", "X|Exit when the VIX closes above its 10-day MA (slide 89)", NOTE_M]),
        d("vixcc_cvr6s", "CVR6 (short-VIX): two of the last three days a new VIX 20-day high, index above its 200-day SMA: buy; exit when the VIX closes below its 10-day MA", "Long", &["E|Slide 67: two out of the last three days the VIX makes a new 20-day high; close above the 200-day MA", "X|Exit when the VIX closes below its 10-day MA (slide 67)", NOTE_M]),
        d("vixcc_cvr6l", "CVR6 (long-VIX signal, sells NIFTY 50 short): two of the last three days a new VIX 20-day low, index below its 200-day SMA: sell short; exit when the VIX closes above its 10-day MA", "Short", &["E|Slide 91: the VIX makes a new 20-day low two out of the past three days; close below the 200-day MA", "X|Exit when the VIX closes above its 10-day MA (slide 91)", NOTE_M]),
        d("vixcc_cvr8s", "CVR8 (short-VIX): change of the VIX 10-day EMA minus change of its 100-day EMA above 0.2 and highest in 40 days, index above its 200-day SMA: buy; exit when the VIX closes below its 10-day MA", "Long", &["E|Slide 69: step (3) = (10-day EMA minus its yesterday value) minus (100-day EMA minus its yesterday value) > 0.2 and the highest of the last 40 days; close above the 200-day MA", "X|Exit when the VIX closes below its 10-day MA (slide 69)", NOTE_M]),
        d("vixcc_cvr8l", "CVR8 (long-VIX signal, sells NIFTY 50 short): change of the VIX 100-day EMA minus change of its 10-day EMA above 0.2 and highest in 40 days, index below its 200-day SMA: sell short; exit when the VIX closes above its 10-day MA", "Short", &["E|Slide 93: step (3) = (100-day EMA change) minus (10-day EMA change) > 0.2 and the highest of the last 40 days; close below the 200-day MA", "X|Exit when the VIX closes above its 10-day MA (slide 93)", NOTE_M]),
        d("vixcc_cvr11s", "CVR11 (short-VIX): VIX high highest of 10 days and yesterday's 3-day MA more than 1 above the 10-day MA, index above its 200-day SMA: buy; exit when the VIX closes below its 10-day MA", "Long", &["E|Slide 71: VIX high is the highest over the last 10 days; yesterday's 3-day MA minus yesterday's 10-day MA of the VIX > 1; close above the 200-day MA", "X|Exit when the VIX closes below its 10-day MA (slide 71)", NOTE_M]),
        d("vixcc_cvr11l", "CVR11 (long-VIX signal, sells NIFTY 50 short): VIX low lowest of 10 days and yesterday's 10-day MA more than 1 above the 3-day MA: sell short; exit when the VIX closes above its 10-day MA", "Short", &["E|Slide 95: VIX low is the lowest over the last 10 days; yesterday's 10-day MA minus yesterday's 3-day MA of the VIX > 1 (no SPY filter is printed)", "X|Exit when the VIX closes above its 10-day MA (slide 95)", NOTE_M]),
    ]
}

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id: &'static str, name: &'static str, rules: &'static [&'static str], dir: &'static str| StrategyDef { id, name, group: "Connors VIX Options Trading Course (slides 73-101)", session: 0, dir, status: Status::Live,
        needs_volume: false, needs_vix: true, needs_series: &[], book: "vixcc", rules, notes: NOTE, claim: None };
    let mut v = catalogue_price();
    v.extend(catalogue_ma());
    v.extend(vec![
        d("vixcc_rsi80", "VIX RSI(2) above 80 three days in a row, index above its 200-day SMA: buy; exit when VIX RSI(2) falls below 25", &["E|Slide 73 (RSI80): VIX RSI(2) > 80 three days in a row and SPY above its 200-day moving average", "X|Exit when VIX RSI(2) < 25 (slide 73)", NOTE], "Long"),
        d("vixcc_4hc", "VIX four higher closes in a row, index above its 200-day SMA: buy; exit when VIX RSI(2) falls below 25", &["E|Slide 76 (4HC): the VIX closes higher than the previous close four days in a row and SPY is above its 200-day moving average", "X|Exit when VIX RSI(2) < 25 (slide 76)", NOTE], "Long"),
        d("vixcc_5hh", "VIX five higher highs in a row, index above its 200-day SMA: buy; exit when VIX RSI(2) falls below 25", &["E|Slide 78 (5HH): the VIX high is above the previous day's high five days in a row and SPY is above its 200-day moving average", "X|Exit when VIX RSI(2) < 25 (slide 78)", NOTE], "Long"),
        d("vixcc_rsi20", "VIX RSI(2) below 20 three days in a row, index below its 200-day SMA: sell short; exit when VIX RSI(2) rises above 70", &["E|Slide 97 (RSI20, long-VIX set): VIX RSI(2) < 20 three days in a row and SPY below its 200-day moving average", "X|Exit when VIX RSI(2) > 70 (slide 97)", NOTE], "Short"),
        d("vixcc_5lc", "VIX five lower closes in a row, index below its 200-day SMA: sell short; exit when VIX RSI(2) rises above 70", &["E|Slide 99 (5LC, long-VIX set): the VIX closes lower than the previous close five days in a row and SPY is below its 200-day moving average", "X|Exit when VIX RSI(2) > 70 (slide 99)", NOTE], "Short"),
        d("vixcc_5ll", "VIX five lower lows in a row, index below its 200-day SMA: sell short; exit when VIX RSI(2) rises above 70", &["E|Slide 101 (5LL, long-VIX set): the VIX low is below the previous day's low five days in a row and SPY is below its 200-day moving average", "X|Exit when VIX RSI(2) > 70 (slide 101)", NOTE], "Short"),
    ]);
    v
}
const NOTE_P: &str = "A|[Declared readings, lane K2, 2] The slides trade SPY with its own prices; here the index itself (NIFTY 50 and the four other NSE index series, R4 #364). Unstated, counted: (1) RSI(2), RSI(4) and the plus DI are Wilder's; percent B is the 5-period, 1 standard deviation band (slide 29); (2) 'bottom N% of the range' is (close - low) / (high - low) at or below N%. Entries and exits at the close of the signal day (slide: 'we enter at the close and exit at the close'); the RSI(2) cap is below 65 and the exit is RSI(2) above 75, as printed.";

pub fn catalogue_price() -> Vec<StrategyDef> {
    let d = |id: &'static str, name: &'static str, rules: &'static [&'static str]| StrategyDef { id, name, group: "Connors VIX Options Trading Course: short-VIX price sets (slides 34-60)", session: 0, dir: "Long", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "vixcc", rules, notes: NOTE_P, claim: None };
    vec![
        d("vixcc_percb2", "Percent B 2: %B below .2 three days, close in the bottom 25% of the range, RSI(2) below 65, above the 200-day SMA; exit RSI(2) above 75", &["E|Slide 34: close above the 200-day MA; PercB < .2 three days in a row; closes in the bottom 25% of today's range; RSI(2) below 65", "X|Exit when RSI(2) closes above 75 (slide 34)", NOTE_P]),
        d("vixcc_down1", "Down 1: three closes below the open, close in the bottom 10% of the range, RSI(2) below 65, above the 200-day SMA; exit RSI(2) above 75", &["E|Slide 38: close lower than its open three days in a row; close above the 200-day MA; closes in the bottom 10% of its range; RSI(2) below 65", "X|Exit when RSI(2) closes above 75 (slide 38)", NOTE_P]),
        d("vixcc_rsi5", "RSI5: RSI(4) below 25 and RSI(2) below 65, above the 200-day SMA; exit RSI(2) above 75", &["E|Slide 48: close above the 200-day MA; RSI(4) below 25; RSI(2) below 65", "X|Exit when RSI(2) closes above 75 (slide 48)", NOTE_P]),
        d("vixcc_dmi1", "DMI1: plus DI(3) below 5 and RSI(2) below 65, above the 200-day SMA; exit RSI(2) above 75", &["E|Slide 54: close above the 200-day MA; Plus DI(3) below 5; RSI(2) below 65", "X|Exit when RSI(2) closes above 75 (slide 54)", NOTE_P]),
        d("vixcc_dmi2", "DMI2: plus DI(2) below 3 and RSI(2) below 65, above the 200-day SMA; exit RSI(2) above 75", &["E|Slide 56: close above the 200-day MA; Plus DI(2) below 3; RSI(2) below 65", "X|Exit when RSI(2) closes above 75 (slide 56)", NOTE_P]),
        d("vixcc_dmi3", "DMI3: plus DI(2) below 10 two days in a row and RSI(2) below 65, above the 200-day SMA; exit RSI(2) above 75", &["E|Slide 58: close above the 200-day MA; Plus DI(2) below 10 for two days in a row; RSI(2) below 65", "X|Exit when RSI(2) closes above 75 (slide 58)", NOTE_P]),
        d("vixcc_dmi5", "DMI5: plus DI(2) below 15 four days in a row and RSI(2) below 65, above the 200-day SMA; exit RSI(2) above 75", &["E|Slide 60: close above the 200-day MA; Plus DI(2) below 15 for four days in a row; RSI(2) below 65", "X|Exit when RSI(2) closes above 75 (slide 60)", NOTE_P]),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { reviewed(id).then_some(("vixcc", "Connors VIX course sets", "VIX RSI(2) / run-of-days signals")) }
pub fn params(_id: &str) -> Option<Vec<ParamDef>> { None }

/// days in a row (ending at t) with cur[k] > prev (up) or < prev (down); NaN breaks the run
fn run_len(v: &[f64], t: usize, up: bool) -> usize {
    let mut n = 0;
    while t >= n + 1 && v[t - n].is_finite() && v[t - n - 1].is_finite() && (if up { v[t - n] > v[t - n - 1] } else { v[t - n] < v[t - n - 1] }) { n += 1; }
    n
}
fn all_in_a_row(v: &[f64], t: usize, n: usize, f: &dyn Fn(f64) -> bool) -> bool { t + 1 >= n && v[t + 1 - n..=t].iter().all(|&z| z.is_finite() && f(z)) }

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, _p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let ma = x.sma_n(200);
    if PRICE_SETS.contains(&id) { return Some(run_price(id, x, from, to, &ma)); }
    if MA_SETS.contains(&id) { return Some(run_ma(id, x, from, to, &ma)); }
    let r = &x.vix.rsi2;
    let mut pos = Book::new();
    for t in from..=to {
        if !ma[t].is_finite() || !r[t].is_finite() && pos.side() == 0. { continue; }
        let (long, up) = (matches!(id, "vixcc_rsi80" | "vixcc_4hc" | "vixcc_5hh"), x.c[t] > ma[t]);
        if pos.side() != 0. {
            if (long && r[t] < 25.) || (!long && r[t] > 70.) { pos.exit(t, x.c[t], "VIX RSI(2) exit"); }
            continue;
        }
        let trend = if long { up } else { x.c[t] < ma[t] };
        let sig = match id {
            "vixcc_rsi80" => all_in_a_row(r, t, 3, &|z| z > 80.),
            "vixcc_4hc" => run_len(&x.vix.close, t, true) >= 4,
            "vixcc_5hh" => run_len(&x.vix.high, t, true) >= 5,
            "vixcc_rsi20" => all_in_a_row(r, t, 3, &|z| z < 20.),
            "vixcc_5lc" => run_len(&x.vix.close, t, false) >= 5,
            _ => run_len(&x.vix.low, t, false) >= 5,
        };
        if sig && trend && t < to { pos.enter(if long { 1. } else { -1. }, t, x.c[t], "close"); }
    }
    Some(pos.done(to, x.c[to]))
}

fn roll(v: &[f64], t: usize, n: usize, f: fn(&[f64]) -> f64) -> f64 {
    if t + 1 < n || v[t + 1 - n..=t].iter().any(|z| !z.is_finite()) { f64::NAN } else { f(&v[t + 1 - n..=t]) }
}
fn ema_carry(v: &[f64], n: usize) -> Vec<f64> {
    let a = 2. / (n as f64 + 1.);
    let mut e = f64::NAN;
    v.iter().map(|&z| { if z.is_finite() { e = if e.is_finite() { e + a * (z - e) } else { z }; } e }).collect()
}

fn run_ma(id: &str, x: &Ctx, from: usize, to: usize, ma200: &[f64]) -> Vec<Trade> {
    let (vc, vh, vl) = (&x.vix.close, &x.vix.high, &x.vix.low);
    let mean = |s: &[f64]| s.iter().sum::<f64>() / s.len() as f64;
    let (mx, mn): (fn(&[f64]) -> f64, fn(&[f64]) -> f64) = (|s| s.iter().cloned().fold(f64::NEG_INFINITY, f64::max), |s| s.iter().cloned().fold(f64::INFINITY, f64::min));
    let n = x.c.len();
    let m10: Vec<f64> = (0..n).map(|t| roll(vc, t, 10, |s| s.iter().sum::<f64>() / s.len() as f64)).collect();
    let m3: Vec<f64> = (0..n).map(|t| roll(vc, t, 3, |s| s.iter().sum::<f64>() / s.len() as f64)).collect();
    let _ = mean;
    let (e10, e100) = (ema_carry(vc, 10), ema_carry(vc, 100));
    let step = |t: usize, sign: f64| if t >= 1 { sign * ((e10[t] - e10[t - 1]) - (e100[t] - e100[t - 1])) } else { f64::NAN };
    let s3: Vec<f64> = (0..n).map(|t| if id.ends_with("8s") { step(t, 1.) } else { step(t, -1.) }).collect();
    let vrsi5 = crate::connors::rsi(&vc.iter().map(|z| *z).collect::<Vec<f64>>(), 5);
    let short = !matches!(id, "vixcc_cvr3s" | "vixcc_cvr6s" | "vixcc_cvr8s" | "vixcc_cvr11s");
    let mut pos = Book::new();
    for t in from..=to {
        if !m10[t].is_finite() || !ma200[t].is_finite() { continue; }
        let lt = x.c[t] < ma200[t];
        if pos.side() != 0. {
            let ex = if short { vc[t] > m10[t] && !matches!(id, "vixcc_v1l" | "vixcc_v2l" | "vixcc_up2l") } else { vc[t] < m10[t] };
            let ex = if matches!(id, "vixcc_v1l" | "vixcc_v2l" | "vixcc_up2l") { x.rsi2[t] < 35. } else { ex };
            if ex { pos.exit(t, x.c[t], "exit"); }
            continue;
        }
        let two_of_three = |f: &dyn Fn(usize) -> bool| t >= 2 && (t - 2..=t).filter(|&k| f(k)).count() >= 2;
        let sig = match id {
            "vixcc_v1l" => lt && x.rsi2[t] > 80. && (t >= 2 && (t - 2..=t).all(|k| vc[k].is_finite() && m10[k].is_finite() && vc[k] <= 0.95 * m10[k])) && x.c[t] >= x.ma5[t],
            "vixcc_v2l" => lt && x.rsi2[t] > 80. && vc[t] <= 0.9 * m10[t] && x.c[t] >= x.ma5[t],
            "vixcc_up2l" => lt && t >= 2 && x.c[t] / x.c[t - 2] - 1. > 0.03 && x.rsi2[t] > 80. && x.c[t] >= x.ma5[t],
            "vixcc_cvr2l" => lt && t >= 1 && vrsi5[t] < 30. && vrsi5[t] > vrsi5[t - 1],
            "vixcc_cvr3s" => x.c[t] > ma200[t] && vl[t] > m10[t] && vc[t] >= 1.1 * m10[t],
            "vixcc_cvr3l" => lt && vh[t] < m10[t] && vc[t] <= 0.9 * m10[t],
            "vixcc_cvr6s" => x.c[t] > ma200[t] && two_of_three(&|k| { let w = roll(vh, k, 20, mx); w.is_finite() && vh[k] >= w }),
            "vixcc_cvr6l" => lt && two_of_three(&|k| { let w = roll(vl, k, 20, mn); w.is_finite() && vl[k] <= w }),
            "vixcc_cvr8s" | "vixcc_cvr8l" => {
                let w = roll(&s3, t, 40, mx);
                let ok = id.ends_with("8s") && x.c[t] > ma200[t] || id.ends_with("8l") && lt;
                ok && s3[t].is_finite() && s3[t] > 0.2 && w.is_finite() && s3[t] >= w
            }
            "vixcc_cvr11s" => { let w = roll(vh, t, 10, mx); x.c[t] > ma200[t] && t >= 1 && w.is_finite() && vh[t] >= w && m3[t - 1] - m10[t - 1] > 1. }
            _ => { let w = roll(vl, t, 10, mn); t >= 1 && w.is_finite() && vl[t] <= w && m10[t - 1] - m3[t - 1] > 1. }
        };
        if sig && t < to { pos.enter(if short { -1. } else { 1. }, t, x.c[t], "close"); }
    }
    pos.done(to, x.c[to])
}

fn run_price(id: &str, x: &Ctx, from: usize, to: usize, ma: &[f64]) -> Vec<Trade> {
    let (r2, r4) = (&x.rsi2, &x.rsi4);
    let (p2, p3, pb) = (x.pdi_n(2), x.pdi_n(3), x.pctb_n(5));
    let row = |t: usize, n: usize, f: &dyn Fn(usize) -> bool| t + 1 >= n && (t + 1 - n..=t).all(|k| f(k));
    let mut pos = Book::new();
    for t in from..=to {
        if !ma[t].is_finite() || !r2[t].is_finite() { continue; }
        if pos.side() != 0. { if r2[t] > 75. { pos.exit(t, x.c[t], "RSI(2) > 75"); } continue; }
        let sig = r2[t] < 65. && x.c[t] > ma[t] && match id {
            "vixcc_percb2" => row(t, 3, &|k| pb[k] < 0.2) && x.close_loc(t) <= 0.25,
            "vixcc_down1" => row(t, 3, &|k| x.c[k] < x.o[k]) && x.close_loc(t) <= 0.10,
            "vixcc_rsi5" => r4[t] < 25.,
            "vixcc_dmi1" => p3[t] < 5.,
            "vixcc_dmi2" => p2[t] < 3.,
            "vixcc_dmi3" => row(t, 2, &|k| p2[k] < 10.),
            _ => row(t, 4, &|k| p2[k] < 15.),
        };
        if sig && t < to { pos.enter(1., t, x.c[t], "close"); }
    }
    pos.done(to, x.c[to])
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn runs_count_consecutive_moves_and_nan_breaks_them() {
        let v = [1., 2., 3., 4., 5., f64::NAN, 6., 7.];
        assert_eq!(run_len(&v, 4, true), 4);
        assert_eq!(run_len(&v, 7, true), 1);
        assert_eq!(run_len(&v, 4, false), 0);
    }
    /// hand-run dump for the pandas reference: VIXCC_CSV=<idx_data.csv> VIXCC_VIX=<india_vix dir> VIXCC_OUT=<file> cargo test --release vixcc_dump -- --ignored
    #[test]
    #[ignore]
    fn vixcc_dump() {
        use std::io::Write;
        let data = crate::load_csv(&std::env::var("VIXCC_CSV").unwrap()).unwrap();
        let vix = crate::load_vix_dir(&std::env::var("VIXCC_VIX").unwrap());
        let mut f = std::io::BufWriter::new(std::fs::File::create(std::env::var("VIXCC_OUT").unwrap()).unwrap());
        writeln!(f, "id,ticker,dir,entry_date,entry_px,exit_date,exit_px,open").unwrap();
        for (t, rows) in data.iter() {
            let x = Ctx::new(rows, rows, &vix);
            for id in REVIEWED { if t != "NIFTY 50" && !PRICE_SETS.contains(id) { continue; } for z in run(id, &x, 0, rows.len() - 1, &Params::new()).unwrap() {
                writeln!(f, "{id},{t},{},{},{},{},{},{}", z.dir, rows[z.legs[0].i].date, z.legs[0].px, rows[z.exit_i].date, z.exit_px, z.open).unwrap();
            } }
        }
    }
}

// Instrument admission: Connors VIX course sets 2026-11-24 (lane K2, tier 5).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "vixcc" && reviewed(def.id),
    label: "The course prints no results for these sets; run on NIFTY 50 with India VIX (from 2009-03-02) as a labelled adaptation of SPY and the CBOE VIX; the gate rows allow NIFTY 50 only (R4 #378).",
    basis: "Connors Group, VIX Options Trading Course (2007) slides 73, 76, 78, 97, 99, 101; independent check authors/conv-1123/k2w/qi/vixcc_ref.py",
};
