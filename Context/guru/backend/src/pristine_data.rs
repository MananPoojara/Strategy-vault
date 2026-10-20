//! Explicit supplied-USD-series-unit research data; no native-dollar certification.
use super::{Market, NativeEvidence};
use std::{error::Error, fmt};

/// This is an explicit model selection, never inferred from a ticker or currency query.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ResearchMode {
    SuppliedUsdSeriesUnits,
    /// Named so a caller cannot silently substitute the research adaptation for this request.
    NativeHistoricalDollars,
}

pub(crate) const BASIS: &str = "Explicit supplied-USD-series-unit research on unchanged vendor prices. Constants are stored series units; historical-dollar quote/adjustment equivalence and book-native performance are not verified. Current/Past watchlists remain snapshots, not historical membership.";

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum DataError {
    ExplicitResearchModeRequired,
    NativeHistoricalDollarsUnverified,
    UnknownQualifiedTicker,
    UnsupportedNativeEvidence,
    Source(String),
}
impl fmt::Display for DataError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ExplicitResearchModeRequired => f.write_str("An explicit supplied-USD-series-unit research mode is required"),
            Self::NativeHistoricalDollarsUnverified => f.write_str("Native historical-dollar price equivalence is not verified; no research fallback was selected"),
            Self::UnknownQualifiedTicker => f.write_str("Unknown qualified native stock ticker"),
            Self::UnsupportedNativeEvidence => f.write_str("Supplied-USD-series-unit research requires server-owned individual US stock/USD/daily source evidence"),
            Self::Source(message) => f.write_str(message),
        }
    }
}
impl Error for DataError {}

/// Unusable values are retained as source states, never fabricated as zero. The
/// strategy controller asks for its exact dependency window only after it has a
/// completed price candidate. Older/unused rows do not create a volume gate.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Volume {
    Value(f64),
    Missing,
    Malformed(String),
    NonFinite(String),
    Negative(f64),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum VolumeIssue {
    Missing,
    Malformed,
    NonFinite,
    Negative,
    WindowUnavailable,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct VolumeFailure {
    pub signal_i: usize,
    pub source_i: Option<usize>,
    pub issue: VolumeIssue,
}
impl VolumeFailure {
    pub(crate) fn code(&self) -> &'static str {
        match self.issue {
            VolumeIssue::Missing => "source_volume_missing",
            VolumeIssue::Malformed => "source_volume_malformed",
            VolumeIssue::NonFinite => "source_volume_nonfinite",
            VolumeIssue::Negative => "source_volume_negative",
            VolumeIssue::WindowUnavailable => "source_volume_window_unavailable",
        }
    }
}
#[derive(Clone, Debug)]
pub(crate) struct SourceRow {
    pub date: String,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: Volume,
}
#[derive(Debug)]
pub(crate) struct Series {
    qualified_ticker: String,
    symbol: String,
    evidence: NativeEvidence,
    provenance_json: String,
    rows: Vec<SourceRow>,
    volume_column_present: bool,
}
impl Series {
    pub(crate) fn qualified_ticker(&self) -> &str {
        &self.qualified_ticker
    }
    pub(crate) fn symbol(&self) -> &str {
        &self.symbol
    }
    pub(crate) fn evidence(&self) -> &NativeEvidence {
        &self.evidence
    }
    /// The original provenance is retained byte-for-byte. BASIS is separate and
    /// does not rewrite false verification flags or the source's price status.
    pub(crate) fn provenance_json(&self) -> &str {
        &self.provenance_json
    }
    pub(crate) fn research_mode(&self) -> ResearchMode {
        ResearchMode::SuppliedUsdSeriesUnits
    }
    pub(crate) fn basis(&self) -> &'static str {
        BASIS
    }
    pub(crate) fn rows(&self) -> &[SourceRow] {
        &self.rows
    }
    pub(crate) fn volume_column_present(&self) -> bool {
        self.volume_column_present
    }
    pub(crate) fn required_volume_window(
        &self,
        signal_i: usize,
    ) -> Result<[f64; 21], VolumeFailure> {
        if signal_i < 20 || signal_i >= self.rows.len() {
            return Err(VolumeFailure {
                signal_i,
                source_i: None,
                issue: VolumeIssue::WindowUnavailable,
            });
        }
        let mut values = [0.; 21];
        for (offset, row) in self.rows[signal_i - 20..=signal_i].iter().enumerate() {
            let issue = match &row.volume {
                Volume::Value(value) if value.is_finite() && *value >= 0. => {
                    values[offset] = *value;
                    continue;
                }
                Volume::Value(value) if !value.is_finite() => VolumeIssue::NonFinite,
                Volume::Value(_) | Volume::Negative(_) => VolumeIssue::Negative,
                Volume::Missing => VolumeIssue::Missing,
                Volume::Malformed(_) => VolumeIssue::Malformed,
                Volume::NonFinite(_) => VolumeIssue::NonFinite,
            };
            return Err(VolumeFailure {
                signal_i,
                source_i: Some(signal_i - 20 + offset),
                issue,
            });
        }
        Ok(values)
    }
}

pub(super) fn mode(mode: Option<ResearchMode>) -> Result<(), DataError> {
    match mode {
        Some(ResearchMode::SuppliedUsdSeriesUnits) => Ok(()),
        Some(ResearchMode::NativeHistoricalDollars) => {
            Err(DataError::NativeHistoricalDollarsUnverified)
        }
        None => Err(DataError::ExplicitResearchModeRequired),
    }
}
pub(super) fn evidence(evidence: &NativeEvidence) -> Result<(), DataError> {
    // NativeEvidence has private fields and is constructed only by the fixed
    // local manifest parser, which also enforces timeframe D. No query-supplied
    // currency, symbol/type guess, venue or listing assertion participates.
    if evidence.market() == Market::UnitedStates
        && evidence.currency() == "USD"
        && evidence.asset_type() == "individual_stock"
    {
        Ok(())
    } else {
        Err(DataError::UnsupportedNativeEvidence)
    }
}
fn decode_volume(token: Option<&str>) -> Volume {
    let Some(token) = token else {
        return Volume::Missing;
    };
    let token = token.trim();
    if token.is_empty() {
        return Volume::Missing;
    }
    match token.parse::<f64>() {
        Err(_) => Volume::Malformed(token.into()),
        Ok(value) if !value.is_finite() => Volume::NonFinite(token.into()),
        Ok(value) if value < 0. => Volume::Negative(value),
        Ok(value) => Volume::Value(value),
    }
}
/// Validate the daily record grammar before using the legacy field decoder.
/// That decoder intentionally has lenient quote handling; changing it globally
/// would change protected loaders. Embedded newlines/multiline records fail
/// closed here, and whitespace may surround an otherwise valid quoted field.
fn checked_fields(line: &str, source_line: usize) -> Result<Vec<String>, DataError> {
    #[derive(Clone, Copy)]
    enum State {
        Start,
        Plain,
        Quoted,
        Closed,
        Trailing,
    }
    use State::*;
    let invalid = || DataError::Source(format!("Native CSV quoting invalid at row {source_line}"));
    let mut state = Start;
    for ch in line.chars() {
        if ch == '\r' || ch == '\n' {
            return Err(invalid());
        }
        state = match (state, ch) {
            (Start, ',') => Start,
            (Start, '"') => Quoted,
            (Start, c) if c.is_whitespace() => Start,
            (Start, _) => Plain,
            (Plain, ',') => Start,
            (Plain, '"') => return Err(invalid()),
            (Plain, _) => Plain,
            (Quoted, '"') => Closed,
            (Quoted, _) => Quoted,
            (Closed, '"') => Quoted, // doubled quote inside a quoted field
            (Closed, ',') => Start,
            (Closed, c) if c.is_whitespace() => Trailing,
            (Closed, _) => return Err(invalid()),
            (Trailing, ',') => Start,
            (Trailing, c) if c.is_whitespace() => Trailing,
            (Trailing, _) => return Err(invalid()),
        };
    }
    if matches!(state, Quoted) {
        return Err(invalid());
    }
    Ok(crate::parse_csv_fields(line))
}
pub(super) struct Decoded {
    pub rows: Vec<SourceRow>,
    pub volume_column_present: bool,
}
pub(super) fn decode(text: &str) -> Result<Decoded, DataError> {
    let error = |message: String| DataError::Source(message);
    let mut lines = text.lines();
    let header = checked_fields(
        lines
            .next()
            .ok_or_else(|| error("Native CSV is empty".into()))?,
        1,
    )?;
    if header
        .iter()
        .any(|name| name.is_empty() || name.chars().any(char::is_control))
        || header.iter().enumerate().any(|(i, name)| {
            header[..i]
                .iter()
                .any(|other| other.eq_ignore_ascii_case(name))
        })
    {
        return Err(error("Native CSV has unsafe or duplicate columns".into()));
    }
    let column = |name: &str| {
        header
            .iter()
            .position(|key| key.eq_ignore_ascii_case(name))
            .ok_or_else(|| error(format!("Native CSV missing {name}")))
    };
    let [di, oi, hi, li, ci] = [
        column("date")?,
        column("open")?,
        column("high")?,
        column("low")?,
        column("close")?,
    ];
    let vi = header
        .iter()
        .position(|key| key.eq_ignore_ascii_case("volume"));
    let mut rows: Vec<SourceRow> = Vec::new();
    for (line_number, line) in lines.enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let fields = checked_fields(line, line_number + 2)?;
        if fields.len() != header.len() {
            return Err(error(format!(
                "Native CSV field count invalid at row {}",
                line_number + 2
            )));
        }
        let date = &fields[di];
        if !super::valid_date(date) || rows.last().is_some_and(|row| row.date >= *date) {
            return Err(error(format!(
                "Native date invalid, duplicated or unsorted at row {}",
                line_number + 2
            )));
        }
        let number = |i: usize| {
            fields[i]
                .parse::<f64>()
                .ok()
                .filter(|value| value.is_finite())
                .ok_or_else(|| {
                    error(format!(
                        "Native numeric price field invalid at row {}",
                        line_number + 2
                    ))
                })
        };
        let (open, high, low, close) = (number(oi)?, number(hi)?, number(li)?, number(ci)?);
        if open <= 0.
            || high <= 0.
            || low <= 0.
            || close <= 0.
            || low > high
            || open < low
            || open > high
            || close < low
            || close > high
        {
            return Err(error(format!(
                "Native OHLC invalid at row {}",
                line_number + 2
            )));
        }
        rows.push(SourceRow {
            date: date.clone(),
            open,
            high,
            low,
            close,
            volume: decode_volume(vi.map(|i| fields[i].as_str())),
        });
        if rows.len() > 100_000 {
            return Err(error(
                "Native series exceeds the bounded daily-series limit".into(),
            ));
        }
    }
    if rows.len() < 2 {
        return Err(error(
            "Native series has fewer than two usable source bars".into(),
        ));
    }
    Ok(Decoded {
        rows,
        volume_column_present: vi.is_some(),
    })
}
pub(super) fn from_verified_catalogue(
    qualified_ticker: &str,
    symbol: &str,
    evidence: &NativeEvidence,
    provenance_json: String,
    decoded: Decoded,
) -> Series {
    Series {
        qualified_ticker: qualified_ticker.into(),
        symbol: symbol.into(),
        evidence: evidence.clone(),
        provenance_json,
        rows: decoded.rows,
        volume_column_present: decoded.volume_column_present,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::baskets::{Catalogue, Market};
    use std::{
        fs,
        path::PathBuf,
        sync::atomic::{AtomicUsize, Ordering},
    };
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    struct Fixture(PathBuf);
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    fn fixture(csv: &str, basket: &str, symbol: &str) -> Fixture {
        let root = std::env::temp_dir().join(format!(
            "vault-pristine-data-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("quote.csv"), csv).unwrap();
        let lines: Vec<_> = csv
            .lines()
            .skip(1)
            .filter(|line| !line.trim().is_empty())
            .collect();
        let first = lines.first().unwrap().split(',').next().unwrap();
        let last = lines.last().unwrap().split(',').next().unwrap();
        let hash = crate::baskets::sha256_hex(csv.as_bytes()).unwrap();
        let (market, currency, mapped, kind) = if basket == "nifty50" {
            ("in", "INR", "NIFTY 50", "current_snapshot")
        } else {
            (
                "us",
                "USD",
                if basket.starts_with("us_nasdaq100") {
                    "NDX"
                } else {
                    "SPX"
                },
                if basket.ends_with("_past") {
                    "current_plus_past_snapshot"
                } else {
                    "current_snapshot"
                },
            )
        };
        fs::write(root.join("index.csv"), format!("basket,ticker,path,first,last,bars,source,source_sha256,csv_sha256,membership,status,has_volume,trusted_market,currency,asset_type,timeframe,adjustment_status,source_price_units_verified,membership_kind,membership_source,membership_source_sha256,mapped_index\n{basket},{symbol},quote.csv,{first},{last},{},Synthetic source only,{}, {hash},Supplied snapshot only,ready,true,{market},{currency},individual_stock,D,not_independently_verified,false,{kind},Synthetic source list,{}, {mapped}\n",lines.len(),"a".repeat(64),"b".repeat(64))).unwrap();
        Fixture(root)
    }
    fn csv(volumes: &[&str]) -> String {
        let mut out = "date,open,high,low,close,volume\n".to_string();
        for (i, v) in volumes.iter().enumerate() {
            out.push_str(&format!("2020-01-{:02},100,110,90,105,{v}\n", i + 1));
        }
        out
    }
    fn load(f: &Fixture, key: &str) -> Result<Series, DataError> {
        Catalogue::load(&f.0)
            .map_err(DataError::Source)?
            .pristine_series(key, Some(ResearchMode::SuppliedUsdSeriesUnits))
    }
    #[test]
    fn explicit_mode_is_required_and_native_dollar_claim_is_not_a_fallback() {
        let f = fixture(&csv(&["1", "2"]), "us_sp500", "ACME");
        let c = Catalogue::load(&f.0).unwrap();
        let k = "basket:us_sp500:ACME";
        assert!(matches!(
            c.pristine_series(k, None),
            Err(DataError::ExplicitResearchModeRequired)
        ));
        assert!(matches!(
            c.pristine_series(k, Some(ResearchMode::NativeHistoricalDollars)),
            Err(DataError::NativeHistoricalDollarsUnverified)
        ));
        let s = c
            .pristine_series(k, Some(ResearchMode::SuppliedUsdSeriesUnits))
            .unwrap();
        assert_eq!(s.research_mode(), ResearchMode::SuppliedUsdSeriesUnits);
        assert!(s.basis().contains("supplied"));
        assert!(!s.evidence().source_price_units_verified());
    }
    #[test]
    fn only_exact_server_owned_us_stock_evidence_admits_not_symbol_or_query_spelling() {
        let f = fixture(&csv(&["1", "2"]), "nifty50", "AMZN");
        assert!(matches!(
            load(&f, "basket:nifty50:AMZN"),
            Err(DataError::UnsupportedNativeEvidence)
        ));
        let f = fixture(&csv(&["1", "2"]), "us_sp500", "ACME");
        let c = Catalogue::load(&f.0).unwrap();
        for key in [
            "ACME",
            "SPX",
            "SPY",
            "NIFTY 50",
            "basket:us_sp500:ACME?currency=USD",
            "basket:us_sp500:UNKNOWN",
        ] {
            assert!(
                matches!(
                    c.pristine_series(key, Some(ResearchMode::SuppliedUsdSeriesUnits)),
                    Err(DataError::UnknownQualifiedTicker)
                ),
                "{key}"
            );
        }
        let original = fs::read_to_string(f.0.join("index.csv")).unwrap();
        for (old, new) in [
            ("us,USD", "us,INR"),
            ("individual_stock,D", "etf,D"),
            ("individual_stock,D", "index,D"),
            ("individual_stock,D", "individual_stock,weekly"),
            (
                "not_independently_verified,false",
                "not_independently_verified,true",
            ),
        ] {
            fs::write(f.0.join("index.csv"), original.replace(old, new)).unwrap();
            assert!(Catalogue::load(&f.0).is_err(), "{new}");
        }
    }
    #[test]
    fn zero_and_each_unusable_volume_remain_distinct_without_global_volume_rejection() {
        let f = fixture(
            &csv(&["0", "-0", "", "bogus", "NaN", "inf", "-3", "2"]),
            "us_sp500",
            "ACME",
        );
        let s = load(&f, "basket:us_sp500:ACME").unwrap();
        let r = s.rows();
        assert!(matches!(r[0].volume,Volume::Value(x) if x==0.));
        assert!(matches!(r[1].volume,Volume::Value(x) if x.to_bits()==(-0f64).to_bits()));
        assert!(matches!(r[2].volume, Volume::Missing));
        assert!(matches!(r[3].volume, Volume::Malformed(_)));
        assert!(matches!(r[4].volume, Volume::NonFinite(_)));
        assert!(matches!(r[5].volume, Volume::NonFinite(_)));
        assert!(matches!(r[6].volume,Volume::Negative(x) if x == -3.));
        assert!(matches!(r[7].volume, Volume::Value(2.)));
    }
    #[test]
    fn required_window_is_exactly_signal_plus_twenty_prior_rows_and_keeps_zero() {
        let mut v = vec!["1"; 23];
        v[0] = "NaN";
        v[22] = "-7";
        v[21] = "0";
        let f = fixture(&csv(&v), "us_sp500", "ACME");
        let s = load(&f, "basket:us_sp500:ACME").unwrap();
        let window = s.required_volume_window(21).unwrap();
        assert_eq!(window.len(), 21);
        assert_eq!(window[20], 0.);
        assert_eq!(
            s.required_volume_window(20).unwrap_err(),
            VolumeFailure {
                signal_i: 20,
                source_i: Some(0),
                issue: VolumeIssue::NonFinite
            }
        );
        assert_eq!(
            s.required_volume_window(22).unwrap_err(),
            VolumeFailure {
                signal_i: 22,
                source_i: Some(22),
                issue: VolumeIssue::Negative
            }
        );
        assert_eq!(
            s.required_volume_window(19).unwrap_err().issue,
            VolumeIssue::WindowUnavailable
        );
        assert_eq!(
            s.required_volume_window(99).unwrap_err().issue,
            VolumeIssue::WindowUnavailable
        );
    }
    #[test]
    fn candidate_controller_can_distinguish_each_required_volume_failure() {
        for (token, issue) in [
            ("", VolumeIssue::Missing),
            ("bad", VolumeIssue::Malformed),
            ("NaN", VolumeIssue::NonFinite),
            ("-1", VolumeIssue::Negative),
        ] {
            let mut v = vec!["1"; 21];
            v[20] = token;
            let f = fixture(&csv(&v), "us_sp500", "ACME");
            let s = load(&f, "basket:us_sp500:ACME").unwrap();
            assert_eq!(
                s.required_volume_window(20).unwrap_err(),
                VolumeFailure {
                    signal_i: 20,
                    source_i: Some(20),
                    issue
                }
            );
        }
    }
    #[test]
    fn absent_volume_column_is_missing_not_synthesized_zero() {
        let text =
            "date,open,high,low,close\n2020-01-01,100,110,90,105\n2020-01-02,100,110,90,105\n";
        let f = fixture(text, "us_sp500", "ACME");
        let s = load(&f, "basket:us_sp500:ACME").unwrap();
        assert!(!s.volume_column_present());
        assert!(s.rows().iter().all(|r| matches!(r.volume, Volume::Missing)));
    }
    #[test]
    fn qualified_sources_are_general_and_current_past_stay_snapshot_evidence() {
        for (basket, symbol) in [
            ("us_sp500", "ACME"),
            ("us_nasdaq100", "OTHER"),
            ("us_sp500_past", "OLD-201910"),
            ("us_nasdaq100_past", "RETIRED"),
        ] {
            let f = fixture(&csv(&["1", "2"]), basket, symbol);
            let key = format!("basket:{basket}:{symbol}");
            let s = load(&f, &key).unwrap();
            assert_eq!(s.qualified_ticker(), key);
            assert_eq!(s.symbol(), symbol);
            assert_eq!(s.evidence().market(), Market::UnitedStates);
            assert_eq!(s.evidence().currency(), "USD");
            assert!(s
                .provenance_json()
                .contains("\"historical_membership_verified\":false"));
            assert_eq!(
                s.evidence().membership_kind(),
                if basket.ends_with("_past") {
                    "current_plus_past_snapshot"
                } else {
                    "current_snapshot"
                }
            );
        }
    }
    #[test]
    fn malformed_prices_order_dates_and_duplicate_headers_are_still_rejected() {
        let good = csv(&["0", "1"]);
        for text in [
            good.replace("100,110,90,105", "100,90,110,105"),
            good.replace("2020-01-02", "2020-01-01"),
            good.replace("2020-01-02", "2020-02-31"),
            good.replace("100,110,90,105", "100,110,90,NaN"),
            good.replace("100,110,90,105", "0,110,0,105"),
            good.replace("close,volume", "close,volume,Volume"),
        ] {
            let f = fixture(&text, "us_sp500", "ACME");
            assert!(load(&f, "basket:us_sp500:ACME").is_err());
        }
    }
    #[test]
    fn hash_coverage_path_and_memory_guards_survive_the_scoped_path() {
        let f = fixture(&csv(&["1", "2"]), "us_sp500", "ACME");
        let key = "basket:us_sp500:ACME";
        let c = Catalogue::load(&f.0).unwrap();
        fs::write(f.0.join("quote.csv"), "tamper").unwrap();
        assert!(c
            .pristine_series(key, Some(ResearchMode::SuppliedUsdSeriesUnits))
            .unwrap_err()
            .to_string()
            .contains("SHA-256"));
        let f = fixture(&csv(&["1", "2"]), "us_sp500", "ACME");
        let index = fs::read_to_string(f.0.join("index.csv")).unwrap();
        fs::write(
            f.0.join("index.csv"),
            index.replace("2020-01-02,2", "2020-01-03,2"),
        )
        .unwrap();
        assert!(load(&f, key).unwrap_err().to_string().contains("coverage"));
        fs::write(
            f.0.join("index.csv"),
            index.replace("quote.csv", "../quote.csv"),
        )
        .unwrap();
        assert!(Catalogue::load(&f.0).is_err());
        let f = fixture(&csv(&["1", "2"]), "us_sp500", "ACME");
        let c = Catalogue::load(&f.0).unwrap();
        fs::OpenOptions::new()
            .write(true)
            .open(f.0.join("quote.csv"))
            .unwrap()
            .set_len(32 * 1024 * 1024 + 1)
            .unwrap();
        assert!(c
            .pristine_series(key, Some(ResearchMode::SuppliedUsdSeriesUnits))
            .unwrap_err()
            .to_string()
            .contains("memory"));
    }
    #[cfg(unix)]
    #[test]
    fn source_symlink_cannot_escape_whitelist_root() {
        let f = fixture(&csv(&["1", "2"]), "us_sp500", "ACME");
        let outside = fixture(&csv(&["1", "2"]), "us_sp500", "OTHER");
        fs::remove_file(f.0.join("quote.csv")).unwrap();
        std::os::unix::fs::symlink(outside.0.join("quote.csv"), f.0.join("quote.csv")).unwrap();
        assert!(load(&f, "basket:us_sp500:ACME")
            .unwrap_err()
            .to_string()
            .contains("escapes"));
    }
    #[test]
    fn legacy_zero_and_positive_volume_behavior_and_metadata_bytes_are_unchanged() {
        let text = csv(&["0", "2"]);
        let f = fixture(&text, "us_sp500", "ACME");
        let c = Catalogue::load(&f.0).unwrap();
        let key = "basket:us_sp500:ACME";
        let before = c.provenance_json(key).unwrap();
        let tickers = c.tickers_json();
        let old = c.rows(key).unwrap();
        assert_eq!(old[0].volume, None);
        assert_eq!(old[1].volume, Some(2.));
        let fresh = c
            .pristine_series(key, Some(ResearchMode::SuppliedUsdSeriesUnits))
            .unwrap();
        assert_eq!(fresh.provenance_json(), before);
        assert_eq!(c.provenance_json(key).unwrap(), before);
        assert_eq!(c.tickers_json(), tickers);
        for (a, b) in old.iter().zip(fresh.rows()) {
            assert_eq!(
                (&a.date, a.open, a.high, a.low, a.close),
                (&b.date, b.open, b.high, b.low, b.close)
            );
        }
        assert_eq!(c.rows(key).unwrap()[0].volume, None);
    }
    #[test]
    fn valid_quoted_zero_whitespace_and_escaped_quoted_text_keep_their_meaning() {
        let mut text = csv(&["0", "unused"]);
        text = text
            .replace(",105,0\n", ",105,  \"0\"  \n")
            .replace(",105,unused\n", ",105,\"0\"\"0\"\n");
        let f = fixture(&text, "us_sp500", "ACME");
        let s = load(&f, "basket:us_sp500:ACME").unwrap();
        assert!(matches!(s.rows()[0].volume,Volume::Value(x) if x==0.));
        assert_eq!(s.rows()[1].volume, Volume::Malformed("0\"0".into()));
        let text = csv(&["0", "1"]).replace(
            "date,open,high,low,close,volume",
            "\"date\",open,high,low,close,  \"volume\"  ",
        );
        let f = fixture(&text, "us_sp500", "ACME");
        assert!(load(&f, "basket:us_sp500:ACME").is_ok());
    }
    #[test]
    fn malformed_record_quotes_never_become_numeric_zero_or_repaired_prices() {
        for volume in [
            "\"0",
            "0\"",
            "0\"\"",
            "0\"0\"",
            "\"0\"junk",
            "\"0\" \"\"",
            "\"0\n0\"",
        ] {
            let f = fixture(&csv(&[volume, "1"]), "us_sp500", "ACME");
            assert!(load(&f, "basket:us_sp500:ACME").is_err(), "{volume:?}");
        }
        let text = csv(&["0", "1"]).replace(
            "date,open,high,low,close,volume",
            "date,open,high,low,close,\"volume",
        );
        let f = fixture(&text, "us_sp500", "ACME");
        assert!(load(&f, "basket:us_sp500:ACME").is_err());
    }
    #[test]
    fn no_permanent_price_array_cache_is_created() {
        let f = fixture(&csv(&["0", "2"]), "us_sp500", "ACME");
        let c = Catalogue::load(&f.0).unwrap();
        let key = "basket:us_sp500:ACME";
        assert_eq!(
            c.pristine_series(key, Some(ResearchMode::SuppliedUsdSeriesUnits))
                .unwrap()
                .rows()
                .len(),
            2
        );
        fs::remove_file(f.0.join("quote.csv")).unwrap();
        assert!(c
            .pristine_series(key, Some(ResearchMode::SuppliedUsdSeriesUnits))
            .is_err());
    }
}
