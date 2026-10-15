//! Phase 8 checked foreign semantics; hand-computed fixtures, not platform-export parity.
use super::{markets, rules::LabError};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Align { SameDate, PriorDate }
impl Align { pub fn name(self) -> &'static str { match self { Self::SameDate => "same_date", Self::PriorDate => "prior_date" } } }
pub const VIX: [&str; 2] = ["INDIA VIX", "CBOE VIX"];

/// A conservative daily-session policy, not intraday timestamp parity. API market admission precedes this helper.
pub fn alignment(run: &str, other: &str, close_fills: bool) -> Align {
    if run == other { return Align::SameDate; }
    match (markets::gl(run), markets::gl(other)) {
        (Some(a),Some(b)) if a.timezone == b.timezone && a.calendar == b.calendar => Align::SameDate,
        (Some(_),_) | (_,Some(_)) => Align::PriorDate,
        // Cboe's daily VIX calculation continues after the US stock close; do not read that day's final value at a close fill.
        _ if other == "CBOE VIX" && close_fills => Align::PriorDate,
        _ => Align::SameDate,
    }
}

pub(super) fn validate_dates(dates: &[String], at: &str) -> Result<(),LabError> {
    let valid = |d: &str| d.len() == 10 && d.bytes().enumerate().all(|(i,c)| if i == 4 || i == 7 {c == b'-'} else {c.is_ascii_digit()})
        && crate::baskets::valid_date(d);
    if dates.iter().any(|d| !valid(d)) || dates.windows(2).any(|w| w[0] >= w[1]) {
        return Err(LabError::Invalid(format!("{at}: dates must be valid ISO YYYY-MM-DD, strictly increasing and unique")));
    }
    Ok(())
}

/// One value per run date. PriorDate uses the latest earlier foreign bar on/after the previous run date;
/// fixup removes that lower bound (or carries the latest on/before SameDate). NaN at that latest eligible
/// bar remains NaN: fixup fills dates, never an indicator's own missing/sparse output. No future bar is read.
pub fn align(run_dates: &[String], other_dates: &[String], values: &[f64], rule: Align, fixup: bool) -> Result<Vec<f64>,LabError> {
    validate_dates(run_dates,"run calendar")?; validate_dates(other_dates,"foreign calendar")?;
    if values.len() != other_dates.len() { return Err(LabError::Invalid("foreign values and dates must have equal lengths".into())); }
    if values.iter().any(|v| v.is_infinite()) { return Err(LabError::Invalid("foreign values cannot contain infinity".into())); }
    let mut out = vec![f64::NAN;run_dates.len()]; let mut k = 0;
    for (i,t) in run_dates.iter().enumerate() {
        while k < other_dates.len() && (other_dates[k] < *t || (rule == Align::SameDate && other_dates[k] == *t)) {k += 1;}
        let Some(j) = k.checked_sub(1) else {continue};
        let eligible = fixup || match rule { Align::SameDate => other_dates[j] == *t, Align::PriorDate => i > 0 && other_dates[j] >= run_dates[i-1] };
        if eligible {out[i] = values[j];}
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lab::testdata::assert_series;
    const N: f64 = f64::NAN;
    fn d(v: &[&str]) -> Vec<String> { v.iter().map(|s| s.to_string()).collect() }

    #[test] fn same_date_drops_other_only_dates_and_leaves_holes_empty() {
        // AmiBroker Foreign(): bars that exist only in the foreign symbol are removed; fixup 0 leaves holes Null.
        let run = d(&["2024-01-01", "2024-01-02", "2024-01-03", "2024-01-05"]);
        let other = d(&["2024-01-01", "2024-01-03", "2024-01-04", "2024-01-05"]);
        assert_series(&align(&run, &other, &[1., 3., 4., 5.], Align::SameDate, false).unwrap(), &[1., N, 3., 5.]);
        // fixup previous (AmiBroker fixup 1 analogue): the hole takes the latest earlier bar
        assert_series(&align(&run, &other, &[1., 3., 4., 5.], Align::SameDate, true).unwrap(), &[1., 1., 3., 5.]);
    }
    #[test] fn prior_date_reads_the_latest_bar_since_the_previous_run_bar() {
        let run = d(&["2024-01-01", "2024-01-02", "2024-01-03", "2024-01-04"]);
        let other = d(&["2024-01-01", "2024-01-02", "2024-01-04"]);
        // first run bar: nothing before it; 02 reads 01; 03 reads 02; 04: latest before is 02 < previous run date 03 → hole
        assert_series(&align(&run, &other, &[1., 2., 4.], Align::PriorDate, false).unwrap(), &[N, 1., 2., N]);
        assert_series(&align(&run, &other, &[1., 2., 4.], Align::PriorDate, true).unwrap(), &[N, 1., 2., 2.]);
    }
    #[test] fn crypto_weekend_reaches_monday_on_a_session_market() {
        let nyse = d(&["2024-01-05", "2024-01-08"]);
        let crypto = d(&["2024-01-05", "2024-01-06", "2024-01-07", "2024-01-08"]);
        assert_series(&align(&nyse, &crypto, &[5., 6., 7., 8.], Align::PriorDate, false).unwrap(), &[N, 7.]);
    }
    #[test] fn alignment_rules_follow_session_and_fill_mode() {
        use Align::*;
        for (run, other, close, want) in [
            ("basket:nifty50:TCS", "NIFTY 50", true, SameDate), ("NIFTY 50", "INDIA VIX", true, SameDate),
            ("SPY", "SPX", true, SameDate), ("SPY", "CBOE VIX", true, PriorDate), ("SPY", "CBOE VIX", false, SameDate),
            ("HSI:HSI", "IG:DAX", false, PriorDate), ("IG:DAX", "IG:FTSE", false, PriorDate),
            ("CRYPTO:BTCUSD", "CRYPTO:ETHUSD", true, SameDate), ("HSI:HSI", "HSI:HSI", true, SameDate),
            ("NIFTY 50", "IG:DAX", false, PriorDate),
        ] { assert_eq!(alignment(run, other, close), want, "{run} reading {other} (close fills {close})"); }
        assert_eq!((SameDate.name(), PriorDate.name()), ("same_date", "prior_date"));
    }
    #[test] fn aligned_values_never_come_from_a_later_date() {
        // pseudo-random calendars: every non-NaN output equals a value whose date is <= (same) or < (prior) the run date
        let mut s = 7u64;
        let mut next = || { s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407); (s >> 33) % 3 };
        let (mut run, mut other) = (vec![], vec![]);
        for day in 0..400 {
            let date = crate::lab::resample::civil(crate::lab::resample::day_number("2020-01-01") + day);
            if next() != 0 { run.push(date.clone()); }
            if next() != 0 { other.push(date); }
        }
        let values: Vec<f64> = (0..other.len()).map(|k| k as f64).collect();
        for rule in [Align::SameDate, Align::PriorDate] {
            for fixup in [false, true] {
                for (i, v) in align(&run, &other, &values, rule, fixup).unwrap().iter().enumerate() {
                    if v.is_nan() { continue; }
                    let src = &other[*v as usize];
                    assert!(if rule == Align::SameDate { *src <= run[i] } else { *src < run[i] }, "{rule:?} fixup {fixup}: {} read {src}", run[i]);
                }
            }
        }
    }

    #[test] fn malformed_calendars_lengths_and_infinities_fail_explicitly() {
        let good=d(&["2024-01-01","2024-01-02"]);
        for bad in [d(&["2024-01-02","2024-01-01"]),d(&["2024-01-01","2024-01-01"]),d(&["2024-02-30"]),d(&["2024-1-01"]),d(&["2023-02-29"]),d(&["1899-12-31"]),d(&["2201-01-01"]),d(&["+024-01-01"])] {
            assert_eq!(align(&bad,&good,&[1.,2.],Align::SameDate,false).unwrap_err().status(),"invalid");
            assert_eq!(align(&good,&bad,&vec![1.;bad.len()],Align::SameDate,false).unwrap_err().status(),"invalid");
        }
        assert!(align(&good,&good,&[1.],Align::SameDate,false).is_err());
        for inf in [f64::INFINITY,f64::NEG_INFINITY] {assert!(align(&good,&good,&[1.,inf],Align::SameDate,false).is_err());}
        assert!(align(&[],&[],&[],Align::SameDate,true).unwrap().is_empty());
        assert!(align(&d(&["2024-02-29"]),&d(&["2024-02-29"]),&[1.],Align::SameDate,false).is_ok());
    }
    #[test] fn fixup_fills_missing_dates_but_never_replaces_the_latest_sparse_nan() {
        let run=d(&["2024-01-01","2024-01-02","2024-01-03","2024-01-04"]); let other=d(&["2024-01-01","2024-01-02","2024-01-04"]);
        assert_series(&align(&run,&other,&[1.,N,4.],Align::SameDate,true).unwrap(),&[1.,N,N,4.]);
        assert_series(&align(&run,&other,&[1.,N,4.],Align::PriorDate,true).unwrap(),&[N,1.,N,N]);
        assert_eq!(alignment("CBOE VIX","CBOE VIX",true),Align::SameDate);
    }
    #[test] fn later_dates_and_values_do_not_change_an_aligned_prefix() {
        let run=d(&["2024-01-02","2024-01-04"]);let other=d(&["2024-01-01","2024-01-03","2024-01-05"]);
        for rule in [Align::SameDate,Align::PriorDate] {for fixup in [false,true] {
            let a=align(&run,&other,&[1.,3.,5.],rule,fixup).unwrap();
            let b=align(&run,&other,&[1.,3.,99999.],rule,fixup).unwrap();assert_series(&a,&b);
        }}
    }
}
