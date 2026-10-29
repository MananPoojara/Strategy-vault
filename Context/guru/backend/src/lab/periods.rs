//! Periods and calendar family (TradingView / AmiBroker built-ins, Phase 6). Formulas: see each definition string.
use super::bars::Bars;
use super::catalogue::{int, len, n, IndicatorDef, Needs, Output, ParamSpec, Params, Placement};
use super::resample::{day_number, period_key, Tf};
use super::ta::NA;

const CAT: &str = "Periods & calendar";
fn div(a: f64, b: f64) -> f64 { if b == 0. || a.is_nan() || b.is_nan() { NA } else { a / b } }
#[allow(clippy::too_many_arguments)]
const fn def(id: &'static str, name: &'static str, params: &'static [ParamSpec], outputs: &'static [&'static str], placement: Placement,
             needs: Needs, definition: &'static str, first: &'static [usize], sparse: bool, f: fn(&Bars, &Params) -> Output) -> IndicatorDef {
    IndicatorDef { id, name, category: CAT, params, outputs, placement, needs, definition, first, sparse, compute: Some(f), note: "" }
}
const PR: Placement = Placement::Price;
const PN: Placement = Placement::Pane;
const NO: Needs = Needs::NONE;
const PV: &[ParamSpec] = &[int("type", "Type (0 Traditional, 1 Fibonacci, 2 Woodie, 3 Classic, 4 DM, 5 Camarilla)", 0., 0., 5.), int("period", "Period (1 week, 2 month, 3 quarter, 4 year)", 1., 1., 4.)];
const PE: &[ParamSpec] = &[int("period", "Period (1 week, 2 month, 3 quarter, 4 year)", 1., 1., 4.)];
const LN: &[ParamSpec] = &[len("length", "Length", 20.)];

pub(super) fn levels(t: usize, h: f64, l: f64, c: f64, o_prev: f64, o_cur: f64) -> [f64; 11] {
    let (p, r) = ((h + l + c) / 3., h - l);
    match t {
        0 => [p, 2. * p - l, p + r, h + 2. * (p - l), 3. * p + (h - 3. * l), 4. * p + (h - 4. * l), 2. * p - h, p - r, l - 2. * (h - p), 3. * p - (3. * h - l), 4. * p - (4. * h - l)],
        1 => [p, p + 0.382 * r, p + 0.618 * r, p + r, NA, NA, p - 0.382 * r, p - 0.618 * r, p - r, NA, NA],
        2 => { let p = (h + l + 2. * o_cur) / 4.; let (r3, s3) = (h + 2. * (p - l), l - 2. * (h - p)); [p, 2. * p - l, p + r, r3, r3 + r, NA, 2. * p - h, p - r, s3, s3 - r, NA] }
        3 => [p, 2. * p - l, p + r, p + 2. * r, p + 3. * r, NA, 2. * p - h, p - r, p - 2. * r, p - 3. * r, NA],
        4 => { let x = if c < o_prev { h + 2. * l + c } else if c > o_prev { 2. * h + l + c } else { h + l + 2. * c }; [x / 4., x / 2. - l, NA, NA, NA, NA, x / 2. - h, NA, NA, NA, NA] }
        _ => { let r5 = div(h, l) * c; [p, c + 1.1 * r / 12., c + 1.1 * r / 6., c + 1.1 * r / 4., c + 1.1 * r / 2., r5, c - 1.1 * r / 12., c - 1.1 * r / 6., c - 1.1 * r / 4., c - 1.1 * r / 2., c - (r5 - c)] }
    }
}
/// Per bar: (current period [o, h, l, c] so far, previous completed period if any).
fn periods(b: &Bars, per: usize) -> Vec<([f64; 4], Option<[f64; 4]>)> {
    let (mut last, mut cur, mut prev) = (None, [NA; 4], None);
    (0..b.len()).map(|i| {
        let k = period_key(&b.date[i], per);
        if last != Some(k) { if last.is_some() { prev = Some(cur); } cur = [b.open[i], b.high[i], b.low[i], b.close[i]]; last = Some(k); }
        else { cur[1] = cur[1].max(b.high[i]); cur[2] = cur[2].min(b.low[i]); cur[3] = b.close[i]; }
        (cur, prev)
    }).collect()
}
fn pivots_std(b: &Bars, prm: &Params) -> Output {
    let t = n(prm, "type");
    let mut out = vec![vec![NA; b.len()]; 11];
    for (i, (cur, prev)) in periods(b, n(prm, "period")).into_iter().enumerate() {
        if let Some(pv) = prev { for (j, v) in levels(t, pv[1], pv[2], pv[3], pv[0], cur[0]).into_iter().enumerate() { out[j][i] = v; } }
    }
    Ok(out)
}
fn mtp(b: &Bars, prm: &Params) -> Output {
    let mut out = vec![vec![NA; b.len()]; 6];
    for (i, (cur, prev)) in periods(b, n(prm, "period")).into_iter().enumerate() {
        out[0][i] = cur[0]; out[1][i] = cur[1]; out[2][i] = cur[2];
        if let Some(pv) = prev { out[3][i] = pv[1]; out[4][i] = pv[2]; out[5][i] = pv[3]; }
    }
    Ok(out)
}
fn performance(b: &Bars, _: &Params) -> Output {
    let c = &b.close;
    let roc = |k: usize| -> Vec<f64> { (0..c.len()).map(|i| if i >= k { div(100. * (c[i] - c[i - k]), c[i - k]) } else { NA }).collect() };
    let (mut base, mut year, mut prev_close) = (NA, None, NA);
    let ytd: Vec<f64> = (0..c.len()).map(|i| {
        let y = period_key(&b.date[i], 4);
        if year.is_some() && year != Some(y) { base = prev_close; }
        year = Some(y); prev_close = c[i];
        div(100. * (c[i] - base), base)
    }).collect();
    Ok(vec![roc(5), roc(21), roc(63), roc(126), ytd, roc(252)])
}
fn calendar(b: &Bars, _: &Params) -> Output {
    let len = b.len();
    let mut out = vec![vec![NA; len]; 7];
    for i in 0..len {
        let d = &b.date[i];
        let dn = day_number(d);
        let num = |r: std::ops::Range<usize>| d.get(r).and_then(|s| s.parse::<f64>().ok()).unwrap_or(f64::NAN);
        out[0][i] = (dn + 4).rem_euclid(7) as f64; // 1970-01-01 was a Thursday (4), Sunday = 0
        out[1][i] = num(8..10); out[2][i] = num(5..7); out[3][i] = num(0..4);
        out[4][i] = (dn - day_number(&format!("{}-01-01", d.get(0..4).unwrap_or("1970"))) + 1) as f64;
        out[5][i] = i as f64;
        let (y, m) = (out[3][i] as i64, out[2][i] as i64);
        let next = if m == 12 { format!("{:04}-01-01", y + 1) } else { format!("{y:04}-{:02}-01", m + 1) };
        out[6][i] = (day_number(&next) - dn - 1) as f64; // calendar days after today in this month (holidays and weekends count)
    }
    Ok(out)
}
/// Meeus, Astronomical Algorithms ch. 49: days since 1970-01-01 UTC of lunation k (k = 0: new moon 2000-01-06); full moon = k + 0.5.
fn phase_day(k: f64, full: bool) -> f64 {
    let k = if full { k + 0.5 } else { k };
    let t = k / 1236.85;
    let jde = 2451550.09766 + 29.530588861 * k + 0.00015437 * t * t - 0.000000150 * t.powi(3) + 0.00000000073 * t.powi(4);
    let r = f64::to_radians;
    let e = 1. - 0.002516 * t - 0.0000074 * t * t;
    let m = r(2.5534 + 29.10535670 * k - 0.0000014 * t * t);
    let mp = r(201.5643 + 385.81693528 * k + 0.0107582 * t * t);
    let f = r(160.7108 + 390.67050284 * k - 0.0016118 * t * t);
    let om = r(124.7746 - 1.56375588 * k + 0.0020672 * t * t);
    let c: [f64; 15] = if full { [-0.40614, 0.17302, 0.01614, 0.01043, 0.00734, -0.00515, 0.00209, -0.00111, -0.00057, 0.00056, -0.00042, 0.00042, 0.00038, -0.00024, -0.00017] }
                       else { [-0.40720, 0.17241, 0.01608, 0.01039, 0.00739, -0.00514, 0.00208, -0.00111, -0.00057, 0.00056, -0.00042, 0.00042, 0.00038, -0.00024, -0.00017] };
    let s: [f64; 15] = [mp.sin(), e * m.sin(), (2. * mp).sin(), (2. * f).sin(), e * (mp - m).sin(), e * (mp + m).sin(), e * e * (2. * m).sin(),
        (mp - 2. * f).sin(), (mp + 2. * f).sin(), e * (2. * mp + m).sin(), (3. * mp).sin(), e * (m + 2. * f).sin(), e * (m - 2. * f).sin(), e * (2. * mp - m).sin(), om.sin()];
    jde + c.iter().zip(&s).map(|(a, b)| a * b).sum::<f64>() - 2440587.5
}
fn moon_phases(b: &Bars, _: &Params) -> Output {
    let days: Vec<i64> = b.date.iter().map(|d| day_number(d)).collect();
    let (mut nm, mut fm) = (vec![0.; b.len()], vec![0.; b.len()]);
    for i in 0..days.len() {
        let (lo, hi) = ((if i == 0 { days[0] } else { days[i - 1] + 1 }) as f64, (days[i] + 1) as f64);
        if hi <= lo { continue; }
        let k0 = ((lo - 10962.6) / 29.530588861).floor() - 1.;
        let span = ((hi - lo) / 29.530588861).ceil() as i64 + 3;
        for j in 0..span {
            let k = k0 + j as f64;
            let t = phase_day(k, false); if lo <= t && t < hi { nm[i] = 1.; }
            let t = phase_day(k, true); if lo <= t && t < hi { fm[i] = 1.; }
        }
    }
    Ok(vec![nm, fm])
}
fn bars_since(x: &[f64], k: usize, high: bool) -> Vec<f64> {
    (0..x.len()).map(|i| {
        if i + 1 < k { return NA; }
        let mut best = 0; // offset from bar i; strict comparison keeps the most recent on ties
        for j in 1..k { if (high && x[i - j] > x[i - best]) || (!high && x[i - j] < x[i - best]) { best = j; } }
        best as f64
    }).collect()
}
fn highest_bars(b: &Bars, prm: &Params) -> Output { Ok(vec![bars_since(&b.high, n(prm, "length"), true)]) }
fn lowest_bars(b: &Bars, prm: &Params) -> Output { Ok(vec![bars_since(&b.low, n(prm, "length"), false)]) }

/// Trading-session counters (spec §5.2), all causal: the session's number in its month, quarter, year and week (1 = first
/// session), the week of the month, the quarter, and Monday–Friday dates left after this one in the month, quarter and year
/// (exchange holidays unknown). A counter is NaN until its first period boundary inside the data: the data's first, partial
/// period does not show where that period began (week_of_month waits for a month boundary).
fn session(b: &Bars, _: &Params) -> Output {
    let len = b.len();
    let mut out = vec![vec![NA; len]; 9];
    let (mut count, mut wom, mut seen) = ([0usize; 4], 0usize, [false; 4]);
    for i in 0..len {
        let d = &b.date[i];
        let changed = |anchor: usize| i == 0 || period_key(d, anchor) != period_key(&b.date[i - 1], anchor);
        for (k, anchor) in [2usize, 3, 4, 1].into_iter().enumerate() { seen[k] |= i > 0 && changed(anchor); count[k] = if changed(anchor) { 1 } else { count[k] + 1 }; }
        wom = if changed(2) { 1 } else if changed(1) { wom + 1 } else { wom };
        let (y, m): (i64, i64) = (d[0..4].parse().unwrap_or(1970), d[5..7].parse().unwrap_or(1));
        let start = |y: i64, m: i64| day_number(&format!("{:04}-{:02}-01", y + (m - 1) / 12, (m - 1) % 12 + 1));
        let today = day_number(d);
        let weekdays = |end: i64| (today + 1..=end).filter(|n| (1..=5).contains(&(n + 4).rem_euclid(7))).count() as f64;
        // week_of_month is known once a month boundary is (seen[0])
        for (k, (v, known)) in [(count[0], seen[0]), (count[1], seen[1]), (count[2], seen[2]), (count[3], seen[3]), (wom, seen[0])].into_iter().enumerate() {
            if known { out[k][i] = v as f64; }
        }
        out[5][i] = ((m - 1) / 3 + 1) as f64;
        out[6][i] = weekdays(start(y, m + 1) - 1);
        out[7][i] = weekdays(start(y, ((m - 1) / 3 + 1) * 3 + 1) - 1);
        out[8][i] = weekdays(start(y + 1, 1) - 1);
    }
    Ok(out)
}
/// Indicators that read future session *dates* (never prices); the generic truncation tests exempt them (spec §5.2).
pub const USES_SESSION_DATES: &[&str] = &["sessions_left", "period_end"];
/// Result-level note every run / optimize / series response carries when it reads one of USES_SESSION_DATES.
pub const SESSION_DATES_NOTE: &str = "sessions_left and period_end read this instrument's future session dates (published holidays, but also unscheduled closures, late-announced special sessions and data gaps); a live signal can differ — session.weekdays_left_* is the strictly causal alternative to sessions_left";
/// Sessions after this one in its month / quarter / year, from the instrument's own session dates (the published exchange
/// calendar); NaN throughout the data's last period, whose remaining sessions are not in the data.
fn sessions_left(b: &Bars, _: &Params) -> Output {
    let len = b.len();
    Ok([2usize, 3, 4].into_iter().map(|anchor| {
        let key: Vec<i64> = b.date.iter().map(|d| period_key(d, anchor)).collect();
        let (mut v, mut left) = (vec![NA; len], 0usize);
        for i in (0..len).rev() {
            left = if i + 1 < len && key[i + 1] == key[i] { left + 1 } else { 0 };
            if key[i] != key[len - 1] { v[i] = left as f64; }
        }
        v
    }).collect())
}
/// 1 on the last bar of its week (Mon–Sun) / calendar month, else 0: the next session falls in a later period. The data's
/// final bar is 0 (its period may continue). Same helper as exit.dual_atr period_end_only and the setup node's period_end.
fn period_end(b: &Bars, _: &Params) -> Output {
    Ok([Tf::W, Tf::M].into_iter().map(|tf| super::series::period_ends(&b.date, tf).into_iter().map(|e| if e { 1. } else { 0. }).collect()).collect())
}

pub const DEFS: &[IndicatorDef] = &[
    def("pivots_std", "Pivot Points Standard", PV, &["p", "r1", "r2", "r3", "r4", "r5", "s1", "s2", "s3", "s4", "s5"], PR, NO, "levels from the previous completed period's H, L, C (DM: and its open; Woodie: the current period's first open): Traditional P=(H+L+C)/3, R1=2P−L, R2=P+(H−L), R3=H+2(P−L), R4=3P+(H−3L), R5=4P+(H−4L), S mirror; Fibonacci P±0.382/0.618/1.0·(H−L); Woodie P=(H+L+2·O)/4, R3=H+2(P−L), R4=R3+(H−L); Classic P±1/2/3·(H−L) (R1/S1 as Traditional); DM X = C<O ? H+2L+C : C>O ? 2H+L+C : H+L+2C, P=X/4, R1=X/2−L, S1=X/2−H; Camarilla C±1.1(H−L)/12, /6, /4, /2, R5=(H/L)·C, S5=C−(R5−C); undefined levels NaN", &[5; 11], false, pivots_std),
    def("mtp", "Multi-Time Period Charts", PE, &["open", "high", "low", "prev_high", "prev_low", "prev_close"], PR, NO, "current period's first open and high/low so far; previous completed period's high, low, close", &[0,0,0,5,5,5], false, mtp),
    def("performance", "Performance", &[], &["w1", "m1", "m3", "m6", "ytd", "y1"], PN, NO, "roc(close, 5/21/63/126/252 bars) (declared bar horizons); ytd = 100·(close/last close of the previous calendar year − 1)", &[5,21,63,126,366,252], false, performance),
    def("calendar", "Calendar", &[], &["dow", "day", "month", "year", "doy", "bar_index", "days_left"], PN, NO, "AmiBroker DayOfWeek (0 Sunday … 6 Saturday), Day, Month, Year, DayOfYear (1–366), BarIndex (0-based) of the bar's date; days_left = calendar days after the bar's date in its month (days in month − Day)", &[0,0,0,0,0,0,0], false, calendar),
    def("session", "Trading Session Counters", &[], &["tdom", "tdoq", "tdoy", "tdow", "week_of_month", "quarter", "weekdays_left_m", "weekdays_left_q", "weekdays_left_y"], PN, NO, "session number in its month / quarter / year / week (1 = first; NaN until the data's first boundary of that period), week of the month (NaN until the first month boundary), quarter 1–4, Monday–Friday dates left after today in the month / quarter / year (holidays unknown; causal)", &[31, 91, 366, 5, 31, 0, 0, 0, 0], false, session),
    def("sessions_left", "Sessions Left in Period", &[], &["m", "q", "y"], PN, NO, "sessions after today in the month / quarter / year from the instrument's own session dates (the published exchange calendar), never prices; NaN in the data's last period (declared adaptation, spec §5.2)", &[0; 3], true, sessions_left),
    def("period_end", "Period End", &[], &["w", "m"], PN, NO, "1 on the last bar of its week (Monday–Sunday) / calendar month — the next session is in a later period — else 0; the data's final bar is 0 (its period may continue); reads the next session's date (declared adaptation, spec §5.2); AFL WeekEnd / MonthEnd for the N-candle exit: since_entry count when period_end > 0", &[0; 2], false, period_end),
    def("moon_phases", "Moon Phases", &[], &["new_moon", "full_moon"], PR, NO, "1 on the first bar dated on or after a new (full) moon, UTC, Meeus ch. 49 with 15 periodic terms; else 0 (declared)", &[0,0], false, moon_phases),
    def("highest_bars", "Bars Since Highest High", LN, &["bars"], PN, NO, "bars since the highest high of the last length bars (0 = this bar; ties → the most recent) (AmiBroker HHVBars, TradingView ta.highestbars as a positive count)", &[19], false, highest_bars),
    def("lowest_bars", "Bars Since Lowest Low", LN, &["bars"], PN, NO, "bars since the lowest low of the last length bars (0 = this bar; ties → the most recent)", &[19], false, lowest_bars),
];

#[cfg(test)]
mod tests {
    use crate::lab::bars::Bars;
    use crate::lab::testdata::{assert_series, close_eq, closes, compiles, d1, from_ohlc, go};
    const N: f64 = f64::NAN;
    fn dated(rows: &[(f64, f64, f64, f64)], dates: &[&str]) -> Bars { let mut b = from_ohlc(rows); b.date = dates.iter().map(|d| d.to_string()).collect(); b }
    #[test] fn pivot_types_from_the_previous_week() {
        // week 1 = d1 bars 0..6 (H 12.5, L 9, C 11.9, first open 10); bar 7 opens week 2 at 11.9
        let want: [[f64; 11]; 6] = [
            [11.133333333333, 13.266666666667, 14.633333333333, 16.766666666667, 18.9, 21.033333333333, 9.766666666667, 7.633333333333, 6.266666666667, 4.9, 3.533333333333],
            [11.133333333333, 12.470333333333, 13.296333333333, 14.633333333333, N, N, 9.796333333333, 8.970333333333, 7.633333333333, N, N],
            [11.325, 13.65, 14.825, 17.15, 20.65, N, 10.15, 7.825, 6.65, 3.15, N],
            [11.133333333333, 13.266666666667, 14.633333333333, 18.133333333333, 21.633333333333, N, 9.766666666667, 7.633333333333, 4.133333333333, 0.633333333333, N],
            [11.475, 13.95, N, N, N, N, 10.45, N, N, N, N],
            [11.133333333333, 12.220833333333, 12.541666666667, 12.8625, 13.825, 16.527777777778, 11.579166666667, 11.258333333333, 10.9375, 9.975, 7.272222222222],
        ];
        for (t, w) in want.iter().enumerate() {
            let o = go("pivots_std", &d1(), &[("type", t as f64), ("period", 1.)]);
            for j in 0..11 { assert!(o[j][..7].iter().all(|v| v.is_nan()), "type {t} level {j}"); assert!(close_eq(o[j][7], w[j]), "type {t} level {j}: {}", o[j][7]); }
        }
    }
    #[test] fn pivots_use_the_previous_completed_week_across_a_holiday_week() {
        // week A Tue–Fri plus a Sunday bar (Phase 5 CFDs); week B starts Tuesday after a Monday holiday
        let b = dated(&[(10., 12., 9., 11.), (11., 13., 10., 12.), (12., 12.5, 11., 11.5), (11.5, 12., 10.5, 11.), (11., 11.4, 10.8, 11.1), (11., 11.5, 10.5, 11.2), (11.2, 12., 11., 11.8)],
                      &["2024-01-02", "2024-01-03", "2024-01-04", "2024-01-05", "2024-01-07", "2024-01-09", "2024-01-10"]);
        let o = go("pivots_std", &b, &[("type", 0.), ("period", 1.)]);
        assert_series(&o[0], &[N, N, N, N, N, 11.033333333333, 11.033333333333]);
        assert_series(&o[1], &[N, N, N, N, N, 13.066666666667, 13.066666666667]);
        assert_series(&o[6], &[N, N, N, N, N, 9.066666666667, 9.066666666667]);
    }
    #[test] fn multi_time_period_week() {
        let o = go("mtp", &d1(), &[("period", 1.)]);
        assert_series(&o[0], &[10., 10., 10., 10., 10., 10., 10., 11.9]);
        assert_series(&o[1], &[11., 12., 12.5, 12.5, 12.5, 12.5, 12.5, 13.]);
        assert_series(&o[2], &[9., 9., 9., 9., 9., 9., 9., 11.5]);
        assert_series(&o[3], &[N, N, N, N, N, N, N, 12.5]);
        assert_series(&o[5], &[N, N, N, N, N, N, N, 11.9]);
    }
    #[test] fn performance_horizons_and_ytd() {
        let o = go("performance", &closes(&[1., 2., 3., 4., 5., 6., 7., 8.]), &[]);
        assert_series(&o[0], &[N, N, N, N, N, 500., 250., 166.666666666667]);
        let y = dated(&[(10., 10., 10., 10.), (11., 11., 11., 11.), (12., 12., 12., 12.), (9.9, 9.9, 9.9, 9.9)], &["2023-12-28", "2023-12-29", "2024-01-02", "2024-01-03"]);
        assert_series(&go("performance", &y, &[])[4], &[N, N, 9.090909090909, -10.]);
    }
    #[test] fn calendar_fields() {
        let o = go("calendar", &d1(), &[]);
        assert_eq!((o[0][0], o[1][0], o[2][0], o[3][0], o[4][0], o[5][0]), (1., 1., 1., 2024., 1., 0.)); // Monday 2024-01-01
        assert_eq!((o[0][7], o[1][7], o[4][7], o[5][7]), (1., 8., 8., 7.));
        let leap = go("calendar", &dated(&[(1., 1., 1., 1.)], &["2024-12-31"]), &[]);
        assert_eq!((leap[0][0], leap[4][0], leap[6][0]), (2., 366., 0.)); // Tuesday, day 366, last day of December
        let feb = go("calendar", &dated(&[(1., 1., 1., 1.); 3], &["2024-02-19", "2023-02-19", "2024-11-20"]), &[]);
        assert_eq!(feb[6], vec![10., 9., 10.]); // leap February has 29 days; November 30
    }
    #[test] fn moon_phases_january_2024() {
        let dates: Vec<String> = (1..=31).map(|d| format!("2024-01-{d:02}")).collect();
        let b = dated(&vec![(1., 1., 1., 1.); 31], &dates.iter().map(|s| s.as_str()).collect::<Vec<_>>());
        let o = go("moon_phases", &b, &[]);
        let days = |v: &[f64]| v.iter().enumerate().filter(|(_, x)| **x == 1.).map(|(i, _)| i + 1).collect::<Vec<_>>();
        assert_eq!(days(&o[0]), [11]); // new moon 2024-01-11 11:57 UTC
        assert_eq!(days(&o[1]), [25]); // full moon 2024-01-25 17:54 UTC
        let gap = go("moon_phases", &dated(&vec![(1., 1., 1., 1.); 4], &["2024-01-09", "2024-01-10", "2024-01-12", "2024-01-15"]), &[]);
        assert_series(&gap[0], &[0., 0., 1., 0.]); // no bar on the 11th: flagged on the next bar
    }
    #[test] fn bars_since_extremes() {
        assert_series(&go("highest_bars", &d1(), &[("length", 3.)])[0], &[N, N, 0., 1., 2., 2., 0., 0.]);
        assert_series(&go("lowest_bars", &d1(), &[("length", 3.)])[0], &[N, N, 2., 2., 0., 0., 1., 2.]);
    }
    #[test] fn rules_reach_periods() { for (id, out) in [("pivots_std", "s5"), ("mtp", "prev_close"), ("calendar", "dow"), ("moon_phases", "full_moon"), ("performance", "ytd"), ("period_end", "m")] { compiles(id, out).unwrap(); } }

    #[test] fn session_counters_are_causal_calendar_arithmetic() {
        let b = dated(&[(1., 1., 1., 1.); 4], &["2024-03-27", "2024-03-28", "2024-04-01", "2024-04-02"]);
        let o = go("session", &b, &[]);
        // the data's first, partial month/quarter/year/week shows no counter (where it began is unknown)
        for (k, want) in [[N, N, 1., 2.], [N, N, 1., 2.], [N, N, N, N], [N, N, 1., 2.], [N, N, 1., 1.], [1., 1., 2., 2.],
                          [2., 1., 21., 20.], [2., 1., 64., 63.], [199., 198., 196., 195.]].iter().enumerate() {
            assert_series(&o[k], want);
        }
        let w = go("session", &dated(&[(1., 1., 1., 1.); 4], &["2024-03-29", "2024-04-05", "2024-04-08", "2024-04-15"]), &[]);
        assert_series(&w[4], &[N, 1., 2., 3.]); // a new week inside the month
    }
    #[test] fn sessions_left_finite_values_never_change_when_later_sessions_arrive() {
        use crate::lab::resample::{civil, day_number};
        // a gapped, multi-year weekday calendar: every 17th weekday and one five-session closure are missing
        let dates: Vec<String> = (day_number("2021-11-01")..).map(civil).filter(|d| (day_number(d) + 3).rem_euclid(7) < 5)
            .enumerate().filter(|(k, _)| k % 17 != 16 && !(200..205).contains(k)).map(|(_, d)| d).take(460).collect();
        let (rows, refs) = (vec![(1., 1., 1., 1.); dates.len()], dates.iter().map(String::as_str).collect::<Vec<_>>());
        let full = go("sessions_left", &dated(&rows, &refs), &[]);
        assert!(full[2].iter().filter(|v| v.is_finite()).count() > 200 && dates[459].as_str() > "2023-06-01", "vacuous");
        for cut in (60..400usize).step_by(7) {
            let part = go("sessions_left", &dated(&rows[..cut], &refs[..cut]), &[]);
            for (k, out) in part.iter().enumerate() {
                for i in 0..cut { assert!(!out[i].is_finite() || out[i] == full[k][i], "output {k} bar {i} ({}) changes when sessions after {cut} exist", dates[i]); }
            }
        }
    }
    #[test] fn session_counters_wait_for_their_first_boundary() {
        use crate::lab::resample::{civil, day_number};
        // weekday sessions from Mon 2024-07-15 to 2025-01-10, with 2024-12-25 and 2025-01-01 as holidays
        let dates: Vec<String> = (day_number("2024-07-15")..=day_number("2025-01-10")).map(civil)
            .filter(|d| (day_number(d) + 3).rem_euclid(7) < 5 && d != "2024-12-25" && d != "2025-01-01").collect();
        let at = |d: &str| dates.iter().position(|x| x == d).unwrap();
        let o = go("session", &dated(&vec![(1., 1., 1., 1.); dates.len()], &dates.iter().map(String::as_str).collect::<Vec<_>>()), &[]);
        let first = |k: usize| o[k].iter().position(|v| !v.is_nan()).unwrap();
        assert_eq!((first(0), first(1), first(2), first(3), first(4)), (at("2024-08-01"), at("2024-10-01"), at("2025-01-02"), at("2024-07-22"), at("2024-08-01")));
        assert_eq!((o[2][at("2025-01-02")], o[2][at("2025-01-03")], o[0][at("2024-08-02")], o[1][at("2024-10-02")]), (1., 2., 2., 2.));
        assert!(o[2][at("2025-01-02")..].iter().all(|v| v.is_finite()));
    }
    #[test] fn period_end_marks_week_and_month_ends_and_never_the_final_bar() {
        // Thu, Fri, Mon, Thu (Friday 12th a holiday), Mon, Wed 31 Jan, Thu 1 Feb, Fri 2 Feb (the data's final bar)
        let dates = ["2024-01-04", "2024-01-05", "2024-01-08", "2024-01-11", "2024-01-15", "2024-01-31", "2024-02-01", "2024-02-02"];
        let o = go("period_end", &dated(&[(1., 1., 1., 1.); 8], &dates), &[]);
        assert_series(&o[0], &[0., 1., 0., 1., 1., 0., 0., 0.]); // 31 Jan and 1 Feb share the week of 29 Jan
        assert_series(&o[1], &[0., 0., 0., 0., 0., 1., 0., 0.]);
        // a Sunday session ends its Monday–Sunday week; the Friday before it does not
        assert_series(&go("period_end", &dated(&[(1., 1., 1., 1.); 3], &["2023-11-10", "2023-11-12", "2023-11-13"]), &[])[0], &[0., 1., 0.]);
    }
    #[test] fn sessions_left_reads_dates_only_and_is_unknown_in_the_last_period() {
        let x = f64::NAN;
        let dates = ["2024-01-30", "2024-01-31", "2024-02-01", "2024-02-02", "2024-03-01", "2024-04-01"];
        let o = go("sessions_left", &dated(&[(1., 1., 1., 1.); 6], &dates), &[]);
        assert_series(&o[0], &[1., 0., 1., 0., 0., x]);
        assert_series(&o[1], &[4., 3., 2., 1., 0., x]);
        assert_series(&o[2], &[x; 6]);
        let other = go("sessions_left", &dated(&[(5., 9., 2., 7.); 6], &dates), &[]); // other prices, same dates
        for k in 0..3 { assert_series(&other[k], &o[k]); }
    }
}
