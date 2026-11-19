//! Lane CV6 (audit Q19, R4 #340): the NSE monthly index/stock option expiry calendar, DERIVED at load from the real NIFTY 50
//! option chain the vault already holds (options/india/NIFTY_50.opt.csv: Date,Expiry,...). The monthly expiry of a calendar month
//! is the latest expiry listed in that month (weekly contracts expire earlier; the real chain already carries the exchange's
//! holiday shifts and the 2025 Thursday -> Tuesday move). No data file is added or edited; US contexts never read this set.
use std::collections::BTreeSet;
use std::io::{BufRead, BufReader};
use std::sync::OnceLock;

static SET: OnceLock<BTreeSet<String>> = OnceLock::new();

/// Latest expiry per expiry month from `Date,Expiry,...` rows (Expiry = YYYYMMDD), as `YYYY-MM-DD`.
pub fn derive<R: BufRead>(rows: R) -> BTreeSet<String> {
    let mut latest: std::collections::BTreeMap<String, String> = Default::default();
    for line in rows.lines().skip(1).map_while(Result::ok) {
        let mut f = line.splitn(3, ',');
        let (_, Some(e)) = (f.next(), f.next()) else { continue };
        if e.len() != 8 || !e.bytes().all(|b| b.is_ascii_digit()) { continue; }
        let slot = latest.entry(e[..6].to_string()).or_default();
        if e > slot.as_str() { *slot = e.to_string(); }
    }
    latest.into_values().map(|e| format!("{}-{}-{}", &e[..4], &e[4..6], &e[6..])).collect()
}

/// Install the calendar from the NIFTY 50 chain file; a missing file installs nothing (the Indian expiry rules then stay refused).
pub fn install(path: &std::path::Path) -> usize {
    let Ok(f) = std::fs::File::open(path) else { return 0 };
    let s = derive(BufReader::with_capacity(1 << 20, f));
    let n = s.len();
    if n > 0 { let _ = SET.set(s); }
    n
}
pub fn available() -> bool { SET.get().is_some() }
/// True when `date` (YYYY-MM-DD) is a derived NSE monthly expiry day.
pub fn is_monthly_expiry(date: &str) -> bool { SET.get().is_some_and(|s| s.contains(date)) }

pub const LABEL: &str = "[R4 #340 declared adaptation, Indian instruments] The expiration day is the NSE monthly option expiry, derived at load from the real NIFTY 50 option chain held in the vault (the latest expiry listed in each calendar month, 2001-06 on; it carries the exchange's holiday shifts and the 2025 move from Thursday to Tuesday). Stock options use the same day. Days before 2001-06 have no expiry and so no trade. UNVERIFIED on India.";

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn latest_expiry_of_each_month_is_the_monthly_one() {
        let csv = "Date,Expiry,Type\n2024-06-03,20240606,C\n2024-06-03,20240627,C\n2024-06-03,20240613,P\n2024-06-03,20240725,C\n2024-07-01,20240725,P\nbad,2024,C\n";
        let s = derive(csv.as_bytes());
        assert_eq!(s.into_iter().collect::<Vec<_>>(), vec!["2024-06-27", "2024-07-25"]);
    }
}
