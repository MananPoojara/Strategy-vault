//! Release-date series (lane M3, R4 #346). An extra_data file with a `ReleaseDate` column (CFTC COT, CPI) is keyed by that
//! date, not by the date it describes: a row exists for a bar only on or after its release, and rows without a release date
//! (a gap, never a guess) are skipped. `column` carries the latest release forward to each bar, but never past the last
//! release plus the file's own longest gap between releases, so a series that has ended is not carried on for ever.
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Mutex;

static KEYED: Mutex<BTreeSet<String>> = Mutex::new(BTreeSet::new());
/// Event lists (a complete list of change dates, e.g. RBI repo-rate changes): key -> the as-of date of the list. Between its first
/// event and the as-of date a day without an event is 0, which is the list's own definition (nothing changed), not a filled price;
/// before the first event and after the as-of date the series is missing.
static EVENTS: Mutex<BTreeMap<String, String>> = Mutex::new(BTreeMap::new());
pub fn mark_event(key: &str, asof: &str) { EVENTS.lock().unwrap().insert(key.to_string(), asof.to_string()); }
pub fn mark(key: &str) { KEYED.lock().unwrap().insert(key.to_string()); }
fn is(key: &str) -> bool { KEYED.lock().unwrap().contains(key) }

/// Days since 1970-01-01 of an ISO date.
fn days(d: &str) -> i64 {
    let p: Vec<i64> = d.split('-').filter_map(|x| x.parse().ok()).collect();
    if p.len() != 3 { return 0; }
    let (y, m, dd) = (p[0] - (p[1] <= 2) as i64, p[1], p[2]);
    let era = y.div_euclid(400); let yoe = y - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + dd - 1;
    era * 146097 + yoe * 365 + yoe / 4 - yoe / 100 + doy - 719468
}

/// One value per bar date for `key`: exact date match, or for a release-keyed series the latest release on or before the bar.
pub fn column<'a>(key: &str, dates: impl Iterator<Item = &'a str>, m: &BTreeMap<String, f64>) -> Vec<f64> {
    if let Some(asof) = EVENTS.lock().unwrap().get(key).cloned() {
        let first = m.keys().next().cloned().unwrap_or_else(|| "9999-12-31".into());
        return dates.map(|d| if d < first.as_str() || d > asof.as_str() { f64::NAN } else { m.get(d).copied().unwrap_or(0.0) }).collect();
    }
    if !is(key) { return dates.map(|d| m.get(d).copied().unwrap_or(f64::NAN)).collect(); }
    let ks: Vec<&String> = m.keys().collect();
    let gap = ks.windows(2).map(|w| days(w[1]) - days(w[0])).max().unwrap_or(0);
    let last = ks.last().map_or(i64::MIN, |k| days(k) + gap);
    dates.map(|d| if days(d) > last { f64::NAN } else { m.range(..=d.to_string()).next_back().map_or(f64::NAN, |(_, v)| *v) }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn a_release_is_usable_from_its_release_date_and_stale_series_end() {
        mark("T_REL.A");
        let m: BTreeMap<String, f64> = [("2020-01-10", 1.0), ("2020-01-17", 2.0), ("2020-01-27", 3.0)].iter().map(|(d, v)| (d.to_string(), *v)).collect();
        let v = column("T_REL.A", ["2020-01-09", "2020-01-10", "2020-01-16", "2020-01-17", "2020-01-26", "2020-01-27", "2020-02-06", "2020-02-07"].into_iter(), &m);
        assert!(v[0].is_nan());
        assert_eq!(&v[1..6], &[1.0, 1.0, 2.0, 2.0, 3.0]);
        assert_eq!(v[6], 3.0); // 10 days after the last release = the longest gap
        assert!(v[7].is_nan());
        assert_eq!(column("OTHER", ["2020-01-10", "2020-01-11"].into_iter(), &m)[1].is_nan(), true);
        assert_eq!(days("1970-01-02"), 1);
        mark_event("T_EVT.A", "2020-01-20");
        let e = column("T_EVT.A", ["2020-01-09", "2020-01-10", "2020-01-11", "2020-01-17", "2020-01-20", "2020-01-21"].into_iter(), &m);
        assert!(e[0].is_nan() && e[1] == 1.0 && e[2] == 0.0 && e[3] == 2.0 && e[4] == 0.0 && e[5].is_nan()); // 0 between events, missing outside first..as-of
    }
}
