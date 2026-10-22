//! Private request bridge for the two fixed Pristine research branches. No
//! public routes are registered here; legacy parsing/execution is unchanged.
use crate::{
    accounting::Costs,
    baskets::{
        pristine_data::{DataError, ResearchMode, SourceRow},
        Catalogue,
    },
    pristine_controller::RunError,
    pristine_daily::Method,
    pristine_wire,
    unit_exposure_arithmetic::ExactValue,
};
use std::{cmp::Ordering, collections::BTreeMap};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Failure {
    pub stage: &'static str,
    pub code: &'static str,
    pub at: Option<usize>,
    pub signal_i: Option<usize>,
    pub source_i: Option<usize>,
}
impl Failure {
    fn new(stage: &'static str, code: &'static str) -> Self {
        Self {
            stage,
            code,
            at: None,
            signal_i: None,
            source_i: None,
        }
    }
    fn indexed(stage: &'static str, code: &'static str, at: usize) -> Self {
        Self {
            stage,
            code,
            at: Some(at),
            signal_i: None,
            source_i: Some(at),
        }
    }
}

pub(crate) fn applies(id: &str) -> bool {
    matches!(id, "ptpm1_daily_cbs_cog" | "ptpm1_bullish_gap_surprise")
}
struct Request<'a> {
    ticker: &'a str,
    from: Option<&'a str>,
    to: Option<&'a str>,
    method: Method,
    costs: Costs,
}
fn request(code: &'static str) -> Failure {
    Failure::new("request", code)
}
fn date(value: Option<&str>) -> Result<Option<&str>, Failure> {
    match value {
        None | Some("") => Ok(None),
        Some(v) if crate::baskets::valid_date(v) => Ok(Some(v)),
        _ => Err(request("pristine_invalid_date")),
    }
}
fn costs(q: &BTreeMap<String, String>) -> Result<Costs, Failure> {
    let scalar = |key: &str| -> Result<f64, Failure> {
        let Some(text) = q.get(key) else {
            return Ok(0.);
        };
        let value = text
            .parse::<f64>()
            .ok()
            .filter(|v| v.is_finite() && (0. ..=5000.).contains(v))
            .ok_or_else(|| request("pristine_invalid_costs"))?;
        // Nonzero decimal settings cannot disappear into the zero-fee default.
        if value == 0.
            && text
                .split(['e', 'E'])
                .next()
                .unwrap_or("")
                .bytes()
                .any(|b| matches!(b, b'1'..=b'9'))
        {
            return Err(request("pristine_cost_input_underflow"));
        }
        Ok(value)
    };
    let c = Costs {
        fees_bps: scalar("cost")?,
        slippage_bps: scalar("slippage")?,
        rollover_bps: scalar("rollover")?,
    };
    if c.rollover_bps != 0. {
        return Err(request("pristine_rollover_unsupported"));
    }
    let exact = |v| ExactValue::canonical(v).map_err(|_| request("pristine_invalid_costs"));
    if exact(c.fees_bps)?
        .add(&exact(c.slippage_bps)?)
        .signed_cmp(&exact(5000.)?)
        == Ordering::Greater
    {
        return Err(request("pristine_invalid_costs"));
    }
    Ok(c)
}
impl<'a> Request<'a> {
    fn parse(q: &'a BTreeMap<String, String>) -> Result<Self, Failure> {
        for key in q.keys() {
            if key == "capital" {
                return Err(request("pristine_capital_unsupported"));
            }
            if key.starts_with("p.") || key.starts_with("g.") {
                return Err(request("pristine_fixed_source_rules"));
            }
            if !matches!(
                key.as_str(),
                "id" | "ticker"
                    | "from"
                    | "to"
                    | "cost"
                    | "slippage"
                    | "rollover"
                    | "research_mode"
                    | "_guru"
            ) {
                return Err(request("pristine_unknown_query_field"));
            }
        }
        if q.get("_guru").is_some_and(|v| v != "pristine") {
            return Err(request("pristine_owner_mismatch"));
        }
        let method = match q.get("id").map(String::as_str) {
            Some("ptpm1_daily_cbs_cog") => Method::CbsCog,
            Some("ptpm1_bullish_gap_surprise") => Method::BullishGapSurprise,
            _ => return Err(request("pristine_unknown_method")),
        };
        match q.get("research_mode").map(String::as_str) {
            Some("supplied_usd_series_units") => {}
            Some("native_historical_dollars") => {
                return Err(Failure::new("data", "native_historical_dollars_unverified"))
            }
            None => return Err(Failure::new("data", "explicit_research_mode_required")),
            _ => return Err(Failure::new("data", "pristine_unknown_research_mode")),
        }
        let ticker = q
            .get("ticker")
            .map(String::as_str)
            .filter(|v| !v.is_empty() && v.len() <= 256 && !v.chars().any(char::is_control))
            .ok_or_else(|| request("pristine_qualified_ticker_required"))?;
        let from = date(q.get("from").map(String::as_str))?;
        let to = date(q.get("to").map(String::as_str))?;
        if from.zip(to).is_some_and(|(a, b)| a > b) {
            return Err(request("pristine_invalid_range"));
        }
        Ok(Self {
            ticker,
            from,
            to,
            method,
            costs: costs(q)?,
        })
    }
    fn range(&self, rows: &[SourceRow]) -> Result<(usize, usize), Failure> {
        let first = rows
            .first()
            .ok_or_else(|| Failure::new("data", "pristine_empty_source"))?;
        let last = rows.last().unwrap();
        if self
            .from
            .is_some_and(|d| d < first.date.as_str() || d > last.date.as_str())
            || self
                .to
                .is_some_and(|d| d < first.date.as_str() || d > last.date.as_str())
        {
            return Err(Failure::new(
                "data",
                "pristine_requested_window_outside_coverage",
            ));
        }
        let from = self
            .from
            .map_or(0, |d| rows.partition_point(|r| r.date.as_str() < d));
        let end = self.to.map_or(rows.len(), |d| {
            rows.partition_point(|r| r.date.as_str() <= d)
        });
        if from >= end {
            return Err(Failure::new("data", "pristine_empty_covered_range"));
        }
        Ok((from, end - 1))
    }
}
fn data_error(error: DataError) -> Failure {
    Failure::new(
        "data",
        match error {
            DataError::ExplicitResearchModeRequired => "explicit_research_mode_required",
            DataError::NativeHistoricalDollarsUnverified => "native_historical_dollars_unverified",
            DataError::UnknownQualifiedTicker => "pristine_unknown_qualified_ticker",
            DataError::UnsupportedNativeEvidence => "pristine_unsupported_instrument",
            // Do not disclose private source paths/tokens through transport errors.
            DataError::Source(_) => "pristine_data_source_unavailable",
        },
    )
}
fn wire_error(error: pristine_wire::Error) -> Failure {
    match error {
        pristine_wire::Error::Costs => request("pristine_invalid_costs"),
        pristine_wire::Error::Run(RunError::Source {
            failure,
            volume_failure,
        }) => {
            if let Some(volume) = volume_failure {
                Failure {
                    stage: "source",
                    code: failure.code,
                    at: Some(failure.at),
                    signal_i: Some(volume.signal_i),
                    source_i: volume.source_i,
                }
            } else {
                Failure::indexed("source", failure.code, failure.at)
            }
        }
        pristine_wire::Error::Run(RunError::Spans(error)) => {
            Failure::indexed("spans", error.code, error.at)
        }
        pristine_wire::Error::Run(RunError::Accounting(error)) => {
            Failure::indexed("accounting", error.code, error.at)
        }
        pristine_wire::Error::Statistics(error) => {
            Failure::indexed("statistics", error.code, error.at)
        }
        pristine_wire::Error::Serialization(code) => Failure::new("serialization", code),
    }
}

/// Admission precedes every legacy context/row conversion. Error results contain
/// no partial financial payload and never trigger a legacy execution fallback.
pub(crate) fn execute(
    catalogue: &Catalogue,
    query: &BTreeMap<String, String>,
) -> Result<String, Failure> {
    execute_with(catalogue, query, |prepared| prepared.json())
}

/// Scoped projections reuse this exact admission and immutable financial run.
/// A callback cannot change the selected source, costs, report or statistics.
pub(crate) fn execute_with<T>(
    catalogue: &Catalogue,
    query: &BTreeMap<String, String>,
    render: impl FnOnce(&pristine_wire::Prepared<'_>) -> Result<T, pristine_wire::Error>,
) -> Result<T, Failure> {
    let request = Request::parse(query)?;
    let series = catalogue
        .pristine_series(request.ticker, Some(ResearchMode::SuppliedUsdSeriesUnits))
        .map_err(data_error)?;
    let (from, to) = request.range(series.rows())?;
    let prepared = pristine_wire::prepare(&series, from, to, request.method, request.costs)
        .map_err(wire_error)?;
    render(&prepared).map_err(wire_error)
}
