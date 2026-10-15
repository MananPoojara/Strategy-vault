//! Daily → weekly / monthly bars. Weeks run Monday–Sunday; a higher-timeframe value becomes usable on the
//! last daily bar of its period, after that bar's close (AmiBroker expandLast, TradingView expr[1]+lookahead).
use super::bars::Bars;

/// Daily, calendar week (Mon–Sun), month, quarter, year, or a block of n sessions counted from the first bar (AmiBroker n*inDaily).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tf { D, W, M, Q, Y, Days(u16) }
pub const MAX_DAYS: u16 = 63;
pub const TF_HELP: &str = "D, W, M, Q, Y or nD (2D to 63D)";
impl Tf {
    pub fn parse(s: &str) -> Option<Tf> {
        match s {
            "D" => Some(Tf::D), "W" => Some(Tf::W), "M" => Some(Tf::M), "Q" => Some(Tf::Q), "Y" => Some(Tf::Y),
            _ => s.strip_suffix('D').filter(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit())).and_then(|n| n.parse::<u16>().ok()).filter(|n| (2..=MAX_DAYS).contains(n)).map(Tf::Days),
        }
    }
    /// The spec spelling ("W", "5D").
    pub fn name(self) -> String { match self { Tf::Days(n) => format!("{n}D"), t => format!("{t:?}") } }
}
/// How a higher-timeframe value reaches daily bars (AmiBroker TimeFrameExpand; spec §5.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Expand { Last, First, Point }
impl Expand {
    pub fn parse(s: &str) -> Option<Expand> { match s { "last" => Some(Expand::Last), "first" => Some(Expand::First), "point" => Some(Expand::Point), _ => None } }
    pub fn key(self) -> &'static str { match self { Expand::Last => "last", Expand::First => "first", Expand::Point => "point" } }
}
/// Day number (days since 1970-01-01) → "YYYY-MM-DD" (proleptic Gregorian).
pub fn civil(z: i64) -> String {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    format!("{:04}-{m:02}-{d:02}", yoe + era * 400 + i64::from(m <= 2))
}
pub fn day_number(date: &str) -> i64 {
    let (y, m, d): (i64, i64, i64) = (date[0..4].parse().unwrap_or(1970), date[5..7].parse().unwrap_or(1), date[8..10].parse().unwrap_or(1));
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (m + if m > 2 { -3 } else { 9 }) + 2) / 5 + d - 1;
    era * 146_097 + (yoe * 365 + yoe / 4 - yoe / 100 + doy) - 719_468
}
fn key(date: &str, tf: Tf) -> i64 {
    match tf {
        Tf::D | Tf::Days(_) => day_number(date),
        Tf::Q => period_key(date, 3),
        Tf::Y => period_key(date, 4),
        Tf::W => { let n = day_number(date); n - (n + 3).rem_euclid(7) } // 1970-01-01 was a Thursday
        Tf::M => date[0..4].parse::<i64>().unwrap_or(0) * 12 + date[5..7].parse::<i64>().unwrap_or(0),
    }
}
/// Calendar period of a "YYYY-MM-DD" date: 1 = week (Monday–Sunday), 2 = month, 3 = quarter, 4 = year.
pub fn period_key(date: &str, anchor: usize) -> i64 {
    let num = |r: std::ops::Range<usize>| date.get(r).and_then(|s| s.parse::<i64>().ok()).unwrap_or(0);
    let (y, m) = (num(0..4), num(5..7));
    match anchor { 1 => key(date, Tf::W), 2 => y * 12 + m, 3 => y * 4 + (m - 1).div_euclid(3), _ => y }
}
pub fn resample(d: &Bars, tf: Tf) -> Bars {
    if tf == Tf::D { let mut b = d.clone(); b.end = (0..d.len()).collect(); return b; }
    let mut out = Bars::default();
    let mut vol: Vec<f64> = Vec::new();
    let mut last: Option<i64> = None;
    for i in 0..d.len() {
        let k = if let Tf::Days(n) = tf { (i / usize::from(n)) as i64 } else { key(&d.date[i], tf) };
        let v = d.volume.as_ref().map(|v| v[i]).unwrap_or(0.);
        if last != Some(k) {
            out.date.push(d.date[i].clone()); out.open.push(d.open[i]); out.high.push(d.high[i]);
            out.low.push(d.low[i]); out.close.push(d.close[i]); out.end.push(i); vol.push(v); last = Some(k);
        } else {
            let j = out.len() - 1;
            out.date[j] = d.date[i].clone(); out.high[j] = out.high[j].max(d.high[i]); out.low[j] = out.low[j].min(d.low[i]);
            out.close[j] = d.close[i]; out.end[j] = i; vol[j] += v;
        }
    }
    out.volume = d.volume.as_ref().map(|_| vol);
    out
}
/// Spreads higher-timeframe values onto daily bars. The last, possibly in-progress week/month shows the value
/// from its latest available daily bar (causal, but it can change when the period completes).
pub fn expand(values: &[f64], htf: &Bars, n: usize) -> Vec<f64> {
    let mut out = vec![f64::NAN; n];
    for w in 0..htf.len() {
        let to = if w + 1 < htf.len() { htf.end[w + 1] } else { n };
        for slot in out.iter_mut().take(to).skip(htf.end[w]) { *slot = values[w]; }
    }
    out
}

/// `expand` with a mode: First shows period p's value on every daily bar of p; Point only on p's last daily bar.
/// First is causal only for values known by the close of p's first bar; the rule parser enforces that.
pub fn expand_mode(values: &[f64], htf: &Bars, n: usize, mode: Expand) -> Vec<f64> {
    if mode == Expand::Last { return expand(values, htf, n); }
    let mut out = vec![f64::NAN; n];
    for w in 0..htf.len() {
        let end = htf.end[w];
        let start = if mode == Expand::Point { end } else if w == 0 { 0 } else { htf.end[w - 1] + 1 };
        for slot in &mut out[start..=end] { *slot = values[w]; }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lab::testdata::assert_series;
    fn daily(dates: &[&str], closes: &[f64]) -> Bars {
        let mut b = Bars::default();
        for (i, (d, c)) in dates.iter().zip(closes).enumerate() {
            b.date.push(d.to_string()); b.open.push(*c); b.high.push(c + 1.); b.low.push(c - 1.); b.close.push(*c); b.end.push(i);
        }
        b
    }
    #[test] fn period_keys_split_weeks_months_quarters_years() {
        assert_eq!(period_key("2024-01-07", 1), period_key("2024-01-01", 1)); // Sunday closes the Monday week
        assert_ne!(period_key("2024-01-07", 1), period_key("2024-01-08", 1));
        assert_ne!(period_key("2024-01-31", 2), period_key("2024-02-01", 2));
        assert_eq!(period_key("2024-01-15", 3), period_key("2024-03-31", 3));
        assert_ne!(period_key("2024-03-31", 3), period_key("2024-04-01", 3));
        assert_ne!(period_key("2023-12-31", 4), period_key("2024-01-01", 4));
    }
    #[test] fn civil_round_trips() { for d in ["2021-01-04", "2024-02-29", "2000-03-01", "2020-01-01"] { assert_eq!(civil(day_number(d)), d); } }
    #[test] fn day_number_epoch() { assert_eq!(day_number("1970-01-01"), 0); assert_eq!(day_number("2024-03-01"), 19783); }
    #[test] fn weekly_groups_by_monday_and_dates_last_day() {
        // Fri 2024-01-05, Mon 08, Tue 09, Fri 12 (holiday-short week), Mon 15
        let d = daily(&["2024-01-05", "2024-01-08", "2024-01-09", "2024-01-12", "2024-01-15"], &[1., 2., 3., 4., 5.]);
        let w = resample(&d, Tf::W);
        assert_eq!(w.date, vec!["2024-01-05", "2024-01-12", "2024-01-15"]);
        assert_eq!(w.end, vec![0, 3, 4]);
        assert_eq!(w.open, vec![1., 2., 5.]); assert_eq!(w.close, vec![1., 4., 5.]);
        assert_eq!(w.high, vec![2., 5., 6.]); assert_eq!(w.low, vec![0., 1., 4.]);
    }
    #[test] fn week_across_year_boundary() {
        let d = daily(&["2024-12-30", "2024-12-31", "2025-01-01", "2025-01-02", "2025-01-03", "2025-01-06"], &[1., 2., 3., 4., 5., 6.]);
        assert_eq!(resample(&d, Tf::W).end, vec![4, 5]);
    }
    #[test] fn daily_is_identity_and_parse() {
        let d = daily(&["2024-01-05", "2024-01-08"], &[1., 2.]);
        let r = resample(&d, Tf::D);
        assert_eq!(r.close, d.close); assert_eq!(r.end, vec![0, 1]);
        assert_eq!((Tf::parse("D"), Tf::parse("W"), Tf::parse("M"), Tf::parse("X")), (Some(Tf::D), Some(Tf::W), Some(Tf::M), None));
        assert_eq!((Tf::parse("Q"), Tf::parse("Y"), Tf::parse("5D"), Tf::parse("63D")), (Some(Tf::Q), Some(Tf::Y), Some(Tf::Days(5)), Some(Tf::Days(63))));
        for bad in ["1D", "64D", "+5D", "D5", "5d", ""] { assert_eq!(Tf::parse(bad), None, "{bad}"); }
        assert_eq!((Tf::Days(5).name(), Tf::Q.name()), ("5D".to_string(), "Q".to_string()));
    }
    #[test] fn monthly_groups() {
        let d = daily(&["2024-01-30", "2024-01-31", "2024-02-01"], &[1., 2., 3.]);
        assert_eq!(resample(&d, Tf::M).end, vec![1, 2]);
    }
    #[test] fn expand_never_looks_ahead() {
        let d = daily(&["2024-01-05", "2024-01-08", "2024-01-09", "2024-01-12", "2024-01-15"], &[1., 2., 3., 4., 5.]);
        let w = resample(&d, Tf::W);
        // weekly value of week 2 (close 4) is not visible on Mon 08 / Tue 09, only from Fri 12
        assert_series(&expand(&w.close, &w, d.len()), &[1., 1., 1., 4., 5.]);
    }
    #[test] fn volume_sums_and_none_stays_none() {
        let mut d = daily(&["2024-01-08", "2024-01-09"], &[1., 2.]);
        assert!(resample(&d, Tf::W).volume.is_none());
        d.volume = Some(vec![10., 5.]);
        assert_eq!(resample(&d, Tf::W).volume, Some(vec![15.]));
    }

    // ---- Phase 7: AmiBroker TimeFrame semantics (spec §5.1)
    #[test] fn quarters_years_and_session_blocks_group_correctly() {
        let d = daily(&["2023-12-29", "2024-01-02", "2024-03-28", "2024-04-01", "2024-12-31", "2025-01-02"], &[1., 2., 3., 4., 5., 6.]);
        assert_eq!(resample(&d, Tf::Q).end, vec![0, 2, 3, 4, 5]);
        assert_eq!(resample(&d, Tf::Y).end, vec![0, 4, 5]);
        let b = resample(&d, Tf::Days(4)); // sessions 0-3 and 4-5, whatever their dates
        assert_eq!((b.end.clone(), b.open.clone(), b.close.clone(), b.high.clone()), (vec![3, 5], vec![1., 5.], vec![4., 6.], vec![5., 7.]));
        // across the Good Friday and New Year holidays each quarter's value appears on its own last session
        let (q, x) = (resample(&d, Tf::Q), f64::NAN);
        assert_series(&expand_mode(&q.close, &q, 6, Expand::Last), &[1., 1., 3., 4., 5., 6.]);
        assert_series(&expand_mode(&q.close, &q, 6, Expand::Point), &[1., x, 3., 4., 5., 6.]);
        assert_series(&expand_mode(&q.open, &q, 6, Expand::First), &[1., 2., 2., 4., 5., 6.]);
        assert_series(&expand_mode(&b.close, &b, 6, Expand::Last), &[x, x, x, 4., 4., 6.]); // a 4-session block closes on its 4th session
    }
    #[test] fn expand_modes_place_values_where_amibroker_does() {
        let d = daily(&["2024-01-05", "2024-01-08", "2024-01-09", "2024-01-12", "2024-01-15"], &[1., 2., 3., 4., 5.]);
        let w = resample(&d, Tf::W); // weeks end at daily 0, 3, 4
        let x = f64::NAN;
        assert_series(&expand_mode(&w.close, &w, 5, Expand::Last), &[1., 1., 1., 4., 5.]);
        assert_series(&expand_mode(&w.close, &w, 5, Expand::Point), &[1., x, x, 4., 5.]);
        assert_series(&expand_mode(&w.open, &w, 5, Expand::First), &[1., 2., 2., 2., 5.]); // the week's open from its first day
    }
}
