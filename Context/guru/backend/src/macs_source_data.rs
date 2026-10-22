//! Strategy-specific raw-native exact-decimal binding. No legacy quote/parser changes.
use super::{NativeEvidence,parse_csv_fields,parse_rows,Row};
use std::fmt;

pub(crate) const NUMERIC_CONVENTION: &str = "raw_native_decimal_bounded_window_sums_v4";

#[derive(Clone,Debug,PartialEq,Eq)]
pub(crate) struct DecimalIdentity {
    pub(crate) digits: String,
    pub(crate) exponent10: i32,
}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub(crate) enum CloseIssue {
    InvalidDecimalToken,
    NonpositiveOrNonfinite,
    IngestedPriceMismatch,
}
impl CloseIssue {
    pub(crate) fn code(self)->&'static str {
        match self {
            Self::InvalidDecimalToken=>"macs_close_decimal_token_invalid",
            Self::NonpositiveOrNonfinite=>"macs_close_nonpositive_or_nonfinite",
            Self::IngestedPriceMismatch=>"macs_close_ingested_price_mismatch",
        }
    }
}
#[derive(Debug,PartialEq,Eq)]
pub(crate) enum DataError {
    Source(String),
    PrefixUnavailable,
    UnsupportedStockEvidence,
    Close { source_i: usize, issue: CloseIssue },
}
impl fmt::Display for DataError {
    fn fmt(&self,f:&mut fmt::Formatter<'_>)->fmt::Result {
        match self {
            Self::Source(message)=>f.write_str(message),
            Self::PrefixUnavailable=>f.write_str("MACS raw-decimal binding requires the genuine causal prefix through the selected end"),
            Self::UnsupportedStockEvidence=>f.write_str("MACS witness requires an actual server-qualified individual daily stock source; it grants no public admission"),
            Self::Close{source_i,issue}=>write!(f,"{} at source observation {}: no source price is reformatted, repaired or silently rounded into this research domain",issue.code(),source_i),
        }
    }
}
impl std::error::Error for DataError {}

/// Normalize the RAW native decimal/scientific token without rounding. Canonical
/// f64 Display equivalence is diagnostic only; exact raw values decide votes.
fn decimal_identity(token:&str)->Result<DecimalIdentity,CloseIssue> {
    let token=token.trim();
    // A bounded lexical witness, not a change to the legacy native parser.
    // All finite f64 Display spellings fit well inside this limit.
    if token.is_empty() || token.len()>1024 { return Err(CloseIssue::InvalidDecimalToken); }
    let token=token.strip_prefix('+').unwrap_or(token);
    if token.starts_with('-') { return Err(CloseIssue::NonpositiveOrNonfinite); }
    let (mantissa,power)=match token.split_once(['e','E']) {
        Some((m,e))=>(m,e.parse::<i32>().map_err(|_|CloseIssue::InvalidDecimalToken)?),
        None=>(token,0),
    };
    let (whole,fraction)=mantissa.split_once('.').unwrap_or((mantissa,""));
    let all=format!("{whole}{fraction}");
    if all.is_empty() || !all.bytes().all(|b|b.is_ascii_digit()) { return Err(CloseIssue::InvalidDecimalToken); }
    let nonzero=all.trim_start_matches('0');
    if nonzero.is_empty() { return Ok(DecimalIdentity{digits:"0".into(),exponent10:0}); }
    let digits=nonzero.trim_end_matches('0');
    let removed=nonzero.len()-digits.len();
    let exponent10=power.checked_sub(fraction.len() as i32)
        .and_then(|v|v.checked_add(removed as i32)).ok_or(CloseIssue::InvalidDecimalToken)?;
    Ok(DecimalIdentity{digits:digits.into(),exponent10})
}

/// Build an exact positive raw decimal using only the existing ExactValue API.
/// Coefficients are assembled in <=9-digit integer chunks (all fit u32), then
/// powers of ten are applied in <=1e9 steps. No float reparse decides the value.
fn exact_raw_decimal(identity:&DecimalIdentity)->Result<crate::unit_exposure_arithmetic::ExactValue,CloseIssue> {
    use crate::unit_exposure_arithmetic::ExactValue;
    if identity.digits.is_empty() || identity.digits.len()>1024 || identity.digits=="0"
        || !identity.digits.bytes().all(|b|b.is_ascii_digit()) || identity.exponent10.unsigned_abs()>4096 {
        return Err(CloseIssue::InvalidDecimalToken);
    }
    let mut value=ExactValue::ratio(0,1).map_err(|_|CloseIssue::InvalidDecimalToken)?;
    for chunk in identity.digits.as_bytes().chunks(9) {
        let text=std::str::from_utf8(chunk).map_err(|_|CloseIssue::InvalidDecimalToken)?;
        let integer=text.parse::<u32>().map_err(|_|CloseIssue::InvalidDecimalToken)?;
        value=value.scale(10u32.pow(chunk.len() as u32)).add(&ExactValue::ratio(integer,1).map_err(|_|CloseIssue::InvalidDecimalToken)?);
    }
    let mut exponent=identity.exponent10.unsigned_abs();
    while exponent>0 {
        let step=exponent.min(9);let power=10u32.pow(step);
        value=if identity.exponent10>=0 {value.scale(power)} else {value.divide(power).map_err(|_|CloseIssue::InvalidDecimalToken)?};
        exponent-=step;
    }
    if !value.positive() {return Err(CloseIssue::NonpositiveOrNonfinite);}
    Ok(value)
}

#[derive(Clone,Debug)]
pub(crate) struct CloseDecimalWitness {
    pub(crate) source_i: usize,
    pub(crate) date: String,
    pub(crate) raw_close_token: String,
    pub(crate) source_close: f64,
    pub(crate) canonical_display: String,
    pub(crate) raw_decimal: DecimalIdentity,
    pub(crate) canonical_decimal: DecimalIdentity,
    pub(crate) canonical_decimal_equivalent: bool,
    pub(crate) ingested_f64_bits_hex: String,
    pub(crate) exact_raw_close: crate::unit_exposure_arithmetic::ExactValue,
}
fn prove_close(token:&str,source_close:f64,source_i:usize,date:&str)->Result<CloseDecimalWitness,DataError> {
    let error=|issue|DataError::Close{source_i,issue};
    if !source_close.is_finite() || source_close<=0. { return Err(error(CloseIssue::NonpositiveOrNonfinite)); }
    let parsed=token.parse::<f64>().map_err(|_|error(CloseIssue::InvalidDecimalToken))?;
    if !parsed.is_finite() || parsed<=0. { return Err(error(CloseIssue::NonpositiveOrNonfinite)); }
    if parsed.to_bits()!=source_close.to_bits() { return Err(error(CloseIssue::IngestedPriceMismatch)); }
    let raw_decimal=decimal_identity(token).map_err(error)?;
    let canonical_display=source_close.to_string();
    let canonical_decimal=decimal_identity(&canonical_display).map_err(error)?;
    let canonical_decimal_equivalent=raw_decimal==canonical_decimal;
    let exact_raw_close=exact_raw_decimal(&raw_decimal).map_err(error)?;
    let ingested_f64_bits_hex=format!("{:016x}",source_close.to_bits());
    Ok(CloseDecimalWitness{source_i,date:date.into(),raw_close_token:token.into(),source_close,
                          canonical_display,raw_decimal,canonical_decimal,canonical_decimal_equivalent,ingested_f64_bits_hex,exact_raw_close})
}

#[derive(Clone,Debug)]
pub(crate) struct WitnessReport {
    pub(crate) arithmetic_convention: &'static str,
    pub(crate) qualified_ticker: String,
    pub(crate) symbol: String,
    pub(crate) csv_sha256: String,
    pub(crate) through_source_i: usize,
    pub(crate) checked_close_rows: usize,
    pub(crate) closes: Vec<CloseDecimalWitness>,
}
#[derive(Debug)]
pub(crate) struct ValidatedSeries {
    rows: Vec<Row>,
    report: WitnessReport,
    exact_raw_closes: Vec<crate::unit_exposure_arithmetic::ExactValue>,
    provenance_json: String,
}
impl ValidatedSeries {
    pub(crate) fn rows(&self)->&[Row] { &self.rows }
    pub(crate) fn report(&self)->&WitnessReport { &self.report }
    pub(crate) fn exact_raw_closes(&self)->&[crate::unit_exposure_arithmetic::ExactValue] { &self.exact_raw_closes }
    pub(crate) fn provenance_json(&self)->&str { &self.provenance_json }
}

fn witness_prefix(text:&str,rows:&[Row],to:usize)->Result<Vec<CloseDecimalWitness>,DataError> {
    if to>=rows.len() { return Err(DataError::PrefixUnavailable); }
    let mut lines=text.lines();
    let header=parse_csv_fields(lines.next().ok_or_else(||DataError::Source("Native CSV is empty".into()))?);
    let unique=|name:&str|->Result<usize,DataError> {
        let positions:Vec<_>=header.iter().enumerate().filter(|(_,key)|key.eq_ignore_ascii_case(name)).map(|(i,_)|i).collect();
        if positions.len()!=1 { return Err(DataError::Source(format!("MACS witness requires one unambiguous native {name} column"))); }
        Ok(positions[0])
    };
    let di=unique("date")?;let ci=unique("close")?;
    let mut witnesses=Vec::with_capacity(to+1);
    for (i,line) in lines.filter(|line|!line.trim().is_empty()).take(to+1).enumerate() {
        let fields=parse_csv_fields(line);
        let date=fields.get(di).ok_or_else(||DataError::Source("Native date missing in MACS witness".into()))?;
        let token=fields.get(ci).ok_or_else(||DataError::Source("Native close missing in MACS witness".into()))?;
        if date!=&rows[i].date { return Err(DataError::Source("MACS raw close/date witness does not align with the unchanged native decoder".into())); }
        witnesses.push(prove_close(token,rows[i].close,i,date)?);
    }
    if witnesses.len()!=to+1 { return Err(DataError::PrefixUnavailable); }
    Ok(witnesses)
}

/// Callable only after the parent catalogue verifies same-byte SHA/root/coverage.
/// Raw close/date/ordinal and parsed binary64 bits are bound for every actual
/// observation0..=to. Exact raw values are retained in the typed series. This
/// does not certify quote authenticity, source units, adjustments or performance.
pub(super) fn from_verified_catalogue(key:&str,symbol:&str,evidence:&NativeEvidence,
    provenance_json:String,csv_sha256:&str,text:&str,to:usize)->Result<ValidatedSeries,DataError> {
    if evidence.asset_type()!="individual_stock" || !key.starts_with("basket:") || symbol.is_empty() {
        return Err(DataError::UnsupportedStockEvidence);
    }
    let rows=parse_rows(text,symbol).map_err(DataError::Source)?;
    let closes=witness_prefix(text,&rows,to)?;
    let report=WitnessReport{arithmetic_convention:NUMERIC_CONVENTION,qualified_ticker:key.into(),symbol:symbol.into(),
                            csv_sha256:csv_sha256.into(),through_source_i:to,checked_close_rows:closes.len(),closes};
    let exact_raw_closes=report.closes.iter().map(|w|w.exact_raw_close.clone()).collect();
    Ok(ValidatedSeries{rows,report,exact_raw_closes,provenance_json})
}

#[cfg(test)]
pub(crate) fn synthetic_raw_fixture(text:&str,to:usize)->ValidatedSeries {
    from_verified_catalogue("basket:us_sp500:AAPL","AAPL",&NativeEvidence::us_test_fixture(),"{}".into(),&"0".repeat(64),text,to).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cmp::Ordering;
    #[test]
    fn raw_integer_chunk_decimal_and_scientific_spellings_are_exact() {
        for token in ["100.1","100.1000","1.001e2","+000100100e-3"] {
            let w=prove_close(token,100.1,0,"2000-01-03").unwrap();
            assert!(w.canonical_decimal_equivalent);assert_eq!(w.raw_decimal,DecimalIdentity{digits:"1001".into(),exponent10:-1});
            assert_eq!(w.exact_raw_close.signed_cmp(&crate::unit_exposure_arithmetic::ExactValue::ratio(1001,10).unwrap()),Ordering::Equal);
        }
        let a=exact_raw_decimal(&decimal_identity("12345678901234567890123456789e-19").unwrap()).unwrap();
        let b=exact_raw_decimal(&decimal_identity("1234567890.1234567890123456789").unwrap()).unwrap();assert_eq!(a.signed_cmp(&b),Ordering::Equal);
        assert_eq!(decimal_identity(".1000"),decimal_identity("1e-1"));
    }
    #[test]
    fn identical_binary64_values_retain_distinct_raw_decimal_values() {
        let up=prove_close("0.20000000000000001",0.2_f64,7,"2000-01-03").unwrap();
        let down=prove_close("0.19999999999999999999",0.2_f64,7,"2000-01-03").unwrap();
        let canonical=crate::unit_exposure_arithmetic::ExactValue::ratio(1,5).unwrap();
        assert!(!up.canonical_decimal_equivalent&&!down.canonical_decimal_equivalent);assert_eq!(up.source_close.to_bits(),down.source_close.to_bits());
        assert_eq!(up.exact_raw_close.signed_cmp(&canonical),Ordering::Greater);assert_eq!(down.exact_raw_close.signed_cmp(&canonical),Ordering::Less);
        assert_eq!(prove_close("100.2",100.1,0,"2000-01-03").unwrap_err(),DataError::Close{source_i:0,issue:CloseIssue::IngestedPriceMismatch});
    }
    #[test]
    fn raw_binding_rejects_bad_domain_and_unbounded_or_malformed_tokens() {
        for token in ["NaN","inf","0","-0.1","1e-9999","1e9999","1e","1.2.3","1e2e3","1_0","+"] {assert!(prove_close(token,0.1,0,"2000-01-03").is_err(),"{token}");}
        assert!(decimal_identity(&"1".repeat(1025)).is_err());assert!(prove_close("100",f64::NAN,0,"2000-01-03").is_err());
    }
    #[test]
    fn raw_binding_covers_full_actual_prefix_without_dropping_noncanonical_rows() {
        let text="date,open,high,low,close,volume\n2000-01-03,0.1,0.3,0.05,0.1,0\n2000-01-04,0.1,0.3,0.05,0.10000000000000001,0\n";
        let rows=parse_rows(text,"AAPL").unwrap();let prefix=witness_prefix(text,&rows,1).unwrap();assert_eq!(prefix.len(),2);assert!(prefix[0].canonical_decimal_equivalent);assert!(!prefix[1].canonical_decimal_equivalent);
        assert_eq!(witness_prefix(text,&rows,2).unwrap_err(),DataError::PrefixUnavailable);
        let series=synthetic_raw_fixture(text,1);assert_eq!(series.exact_raw_closes().len(),2);assert_eq!(series.report().through_source_i,1);
    }
    #[test]
    fn raw_binding_rejects_duplicate_columns_and_wrong_row_mapping() {
        let duplicate="date,open,high,low,close,close,volume\n2000-01-03,1,2,0.5,1,1,0\n2000-01-04,1,2,0.5,1,1,0\n";assert!(witness_prefix(duplicate,&parse_rows(duplicate,"AAPL").unwrap(),0).is_err());
        let text="date,open,high,low,close,volume\n2000-01-03,1,2,0.5,1,0\n2000-01-04,1,2,0.5,1,0\n";let mut rows=parse_rows(text,"AAPL").unwrap();rows[0].date="2000-01-04".into();assert!(witness_prefix(text,&rows,0).is_err());
    }
    #[test]
    fn exact_decimal_exponent_application_uses_bounded_integer_operations() {
        let a=exact_raw_decimal(&decimal_identity("123456789012345678901e-30").unwrap()).unwrap();
        let b=exact_raw_decimal(&decimal_identity("0.000000000123456789012345678901").unwrap()).unwrap();assert_eq!(a.signed_cmp(&b),Ordering::Equal);
        let a=exact_raw_decimal(&decimal_identity("123456789012345678901e12").unwrap()).unwrap();let b=exact_raw_decimal(&decimal_identity("123456789012345678901000000000000").unwrap()).unwrap();assert_eq!(a.signed_cmp(&b),Ordering::Equal);
    }
}
