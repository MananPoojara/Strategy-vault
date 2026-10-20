//! Private source-to-report path for explicitly selected supplied-series stock
//! research. Data admission is owned by the typed catalogue Series, not ticker
//! spelling or a query-supplied currency. No public strategy is registered here.
use crate::{
    accounting::Costs,
    baskets::pristine_data::{Series, Volume, VolumeFailure},
    pristine_daily::{self, Bar, Failure, Ledger, Method},
    pristine_report::{self, Report, Span},
    pristine_spans,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ResearchContext {
    /// The manual's preferred Nasdaq/$35 context is not being certified. The
    /// selected broader-US-stock branch uses constants in supplied series units.
    BroadUsSuppliedSeries,
}

#[derive(Debug)]
pub(crate) enum RunError {
    Source {
        failure: Failure,
        volume_failure: Option<VolumeFailure>,
    },
    Spans(pristine_report::Error),
    Accounting(pristine_report::Error),
}

#[derive(Debug)]
pub(crate) struct ResearchRun<'a> {
    pub series: &'a Series,
    pub context: ResearchContext,
    pub method: Method,
    pub from: usize,
    pub to: usize,
    pub ledger: Ledger,
    pub spans: Vec<Span>,
    pub report: Report,
}

fn source_error(series: &Series, failure: &Failure) -> RunError {
    let mut failure = failure.clone();
    let volume_failure = if matches!(
        failure.code,
        "source_volume_missing" | "source_volume_nonfinite" | "source_volume_negative"
    ) {
        // The core requests volume only after a completed price candidate. Its
        // Option adapter cannot retain lexical source errors, so recover the
        // exact original state and offending row from that dependency window.
        series.required_volume_window(failure.at).err()
    } else {
        None
    };
    if let Some(volume) = &volume_failure {
        failure.code = volume.code();
    }
    RunError::Source {
        failure,
        volume_failure,
    }
}

/// Full genuine source prefix, flat selected start, inclusive absolute indices.
/// All successful stages travel together. A failed source, span or report stage
/// returns no partial financial report and has no legacy/native-dollar fallback.
pub(crate) fn run<'a>(
    series: &'a Series,
    from: usize,
    to: usize,
    method: Method,
    costs: Costs,
) -> Result<ResearchRun<'a>, RunError> {
    let bars: Vec<_> = series
        .rows()
        .iter()
        .map(|row| Bar {
            date: row.date.clone(),
            o: row.open,
            h: row.high,
            l: row.low,
            c: row.close,
            v: match &row.volume {
                Volume::Value(value) if value.is_finite() && *value >= 0. => Some(*value),
                _ => None,
            },
        })
        .collect();
    let source = pristine_daily::run(&bars, from, to, method, false);
    source
        .as_ref()
        .map_err(|failure| source_error(series, &failure.failure))?;
    let spans = pristine_spans::extract(&source, from, to).map_err(RunError::Spans)?;
    let report =
        pristine_report::build(&bars, from, to, &spans, costs).map_err(RunError::Accounting)?;
    let ledger = source.map_err(|failure| source_error(series, &failure.failure))?;
    Ok(ResearchRun {
        series,
        context: ResearchContext::BroadUsSuppliedSeries,
        method,
        from,
        to,
        ledger,
        spans,
        report,
    })
}
