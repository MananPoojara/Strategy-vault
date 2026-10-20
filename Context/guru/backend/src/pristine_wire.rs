//! Private Pristine protocol. A prepared response owns its run, costs and
//! derived statistics together. This module registers no public route/method.
use crate::{
    accounting::Costs,
    baskets::pristine_data::{Series, Volume},
    pristine_controller::{self, ResearchRun},
    pristine_daily::{self, Method},
    pristine_report::{ClosedView, EntryPhase, ExitPhase},
    pristine_stats::{self, StatMap, Statistics},
    unit_exposure_arithmetic::{ExactAudit, ExactValue},
};
use std::cmp::Ordering;

pub(crate) const MODEL: &str = "pristine_daily_supplied_series_unit_exposure_v1";
pub(crate) const CONVENTION: &str = "tpm1_daily_source_adaptation_v1";

#[derive(Debug)]
pub(crate) enum Error {
    Costs,
    Run(pristine_controller::RunError),
    Statistics(pristine_stats::StatsError),
    Serialization(&'static str),
}

pub(crate) struct Prepared<'a> {
    run: ResearchRun<'a>,
    statistics: Statistics,
    costs: Costs,
}

/// A typed Series has already passed the explicit-mode/instrument admission.
/// Costs cannot be changed independently of the resulting financial report.
pub(crate) fn prepare(
    series: &Series,
    from: usize,
    to: usize,
    method: Method,
    costs: Costs,
) -> Result<Prepared<'_>, Error> {
    if ![costs.fees_bps, costs.slippage_bps]
        .iter()
        .all(|v| v.is_finite() && (0. ..=5000.).contains(v))
        || !costs.rollover_bps.is_finite()
        || costs.rollover_bps != 0.
    {
        return Err(Error::Costs);
    }
    let fee = ExactValue::canonical(costs.fees_bps).map_err(|_| Error::Costs)?;
    let slip = ExactValue::canonical(costs.slippage_bps).map_err(|_| Error::Costs)?;
    let bound = ExactValue::canonical(5000.).map_err(|_| Error::Costs)?;
    if fee.add(&slip).signed_cmp(&bound) == Ordering::Greater {
        return Err(Error::Costs);
    }
    let run = pristine_controller::run(series, from, to, method, costs).map_err(Error::Run)?;
    let statistics = pristine_stats::build(&run.report, &run.spans).map_err(Error::Statistics)?;
    Ok(Prepared {
        run,
        statistics,
        costs,
    })
}

fn q(value: &str) -> String {
    let mut out = String::from("\"");
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if (c as u32) < 0x20 => {
                use std::fmt::Write;
                write!(&mut out, "\\u{:04x}", c as u32).unwrap();
            }
            _ => out.push(ch),
        }
    }
    out.push('"');
    out
}
fn n(value: f64) -> Result<String, Error> {
    if value.is_finite() {
        Ok(value.to_string())
    } else {
        Err(Error::Serialization("pristine_wire_nonfinite"))
    }
}
fn optional<T>(
    value: Option<T>,
    encode: impl FnOnce(T) -> Result<String, Error>,
) -> Result<String, Error> {
    value.map_or(Ok("null".into()), encode)
}
fn obj<const N: usize>(fields: [(&str, String); N]) -> String {
    format!(
        "{{{}}}",
        fields
            .into_iter()
            .map(|(k, v)| format!("{}:{v}", q(k)))
            .collect::<Vec<_>>()
            .join(",")
    )
}
fn array(values: impl IntoIterator<Item = Result<String, Error>>) -> Result<String, Error> {
    Ok(format!(
        "[{}]",
        values.into_iter().collect::<Result<Vec<_>, _>>()?.join(",")
    ))
}
fn audit(a: &ExactAudit) -> String {
    obj([
        ("negative", a.negative.to_string()),
        ("numerator_hex", q(&a.numerator_hex)),
        ("denominator_hex", q(&a.denominator_hex)),
    ])
}
fn setup(s: &pristine_daily::Setup) -> String {
    obj([
        ("signal_i", s.signal_i.to_string()),
        (
            "trigger",
            s.trigger
                .as_ref()
                .map_or("null".into(), |v| audit(&v.audit())),
        ),
        ("stop", audit(&s.stop.audit())),
        (
            "target",
            s.target
                .as_ref()
                .map_or("null".into(), |v| audit(&v.audit())),
        ),
        ("signal_close", audit(&s.signal_close.audit())),
    ])
}
fn position(p: &pristine_daily::Position) -> Result<String, Error> {
    Ok(obj([
        ("signal_i", p.signal_i.to_string()),
        ("entry_i", p.entry_i.to_string()),
        ("entry_price", n(p.entry_price)?),
        ("stop", audit(&p.stop.audit())),
        ("target", audit(&p.target.audit())),
    ]))
}
fn event(e: &pristine_daily::Event) -> Result<String, Error> {
    Ok(obj([
        ("i", e.i.to_string()),
        ("kind", q(e.kind)),
        ("reason", q(e.reason)),
        ("phase", q(e.phase)),
        ("signal_i", e.signal_i.to_string()),
        (
            "entry_i",
            e.entry_i.map_or("null".into(), |v| v.to_string()),
        ),
        ("price", optional(e.price, n)?),
        (
            "exact_price",
            e.exact_price.as_ref().map_or("null".into(), audit),
        ),
        ("stop", e.stop.as_ref().map_or("null".into(), audit)),
        ("target", e.target.as_ref().map_or("null".into(), audit)),
        (
            "previous_stop",
            e.previous_stop.as_ref().map_or("null".into(), audit),
        ),
        ("setup", e.setup.as_ref().map_or("null".into(), setup)),
        ("ambiguity", array(e.ambiguity.iter().map(|v| Ok(q(v))))?),
    ]))
}
fn closed(v: &ClosedView) -> Result<String, Error> {
    Ok(obj([
        ("closed_count", v.closed_count.to_string()),
        ("closed_wins", v.closed_wins.to_string()),
        ("closed_nonwins", v.closed_nonwins.to_string()),
        ("closed_return_sum", n(v.closed_return_sum)?),
        (
            "closed_average_return",
            optional(v.closed_average_return, n)?,
        ),
        ("closed_win_rate", optional(v.closed_win_rate, n)?),
        ("closed_compound_equity", n(v.closed_compound_equity)?),
        ("closed_compound_return", n(v.closed_compound_return)?),
    ]))
}
fn statistics(s: &StatMap) -> Result<String, Error> {
    let values = s
        .values
        .iter()
        .map(|(key, value)| Ok(format!("{}:{}", q(key), optional(*value, n)?)))
        .collect::<Result<Vec<_>, Error>>()?
        .join(",");
    let display = s
        .values
        .iter()
        .map(|(key, value)| {
            format!(
                "{}:{}",
                q(key),
                value.map_or("null".into(), pristine_stats::statistic_number)
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    let reasons = s
        .null_reasons
        .iter()
        .map(|(key, value)| format!("{}:{}", q(key), q(value)))
        .collect::<Vec<_>>()
        .join(",");
    Ok(obj([
        ("values", format!("{{{values}}}")),
        ("display_values", format!("{{{display}}}")),
        ("null_reasons", format!("{{{reasons}}}")),
    ]))
}
fn entry_phase(p: EntryPhase) -> &'static str {
    match p {
        EntryPhase::Open => "open",
        EntryPhase::Stop => "stop",
    }
}
fn exit_phase(p: ExitPhase) -> &'static str {
    match p {
        ExitPhase::OpenStop => "open_stop",
        ExitPhase::OpenTarget => "open_target",
        ExitPhase::OpenFullGap => "open_full_gap",
        ExitPhase::Stop => "stop",
        ExitPhase::Target => "target",
        ExitPhase::Close => "close",
        ExitPhase::Mark => "mark",
    }
}
fn volume(v: &Volume) -> Result<String, Error> {
    let (state, value, token) = match v {
        Volume::Value(v) => ("value", n(*v)?, "null".into()),
        Volume::Missing => ("missing", "null".into(), "null".into()),
        Volume::Malformed(v) => ("malformed", "null".into(), q(v)),
        Volume::NonFinite(v) => ("nonfinite", "null".into(), q(v)),
        Volume::Negative(v) => ("negative", n(*v)?, "null".into()),
    };
    Ok(obj([
        ("state", q(state)),
        ("value", value),
        ("token", token),
    ]))
}

impl Prepared<'_> {
    /// Read-only API projections share the already constructed run and costs;
    /// callers cannot replace financial fields or select a different identity.
    pub(crate) fn id(&self) -> &'static str {
        match self.run.method {
            Method::CbsCog => "ptpm1_daily_cbs_cog",
            Method::BullishGapSurprise => "ptpm1_bullish_gap_surprise",
        }
    }
    pub(crate) fn series(&self) -> &Series {
        self.run.series
    }
    pub(crate) fn bounds(&self) -> (usize, usize) {
        (self.run.from, self.run.to)
    }
    pub(crate) fn costs(&self) -> Costs {
        self.costs
    }
    pub(crate) fn report(&self) -> &crate::pristine_report::Report {
        &self.run.report
    }
    pub(crate) fn statistics(&self) -> &Statistics {
        &self.statistics
    }
    pub(crate) fn json(&self) -> Result<String, Error> {
        let r = &self.run;
        let report = &r.report;
        let id = match r.method {
            Method::CbsCog => "ptpm1_daily_cbs_cog",
            Method::BullishGapSurprise => "ptpm1_bullish_gap_surprise",
        };
        let days = array(report.days.iter().map(|d| {
            Ok(obj([
                ("source_index", d.source_index.to_string()),
                ("date", q(&d.date)),
                ("equity", n(d.equity)?),
                ("gross_equity", n(d.gross_equity)?),
                ("benchmark_equity", n(d.benchmark_equity)?),
                ("net_daily_return", n(d.net_daily_return)?),
                ("gross_daily_return", n(d.gross_daily_return)?),
                ("benchmark_daily_return", n(d.benchmark_daily_return)?),
            ]))
        }))?;
        let returns = array(report.trade_returns.iter().map(|t| {
            Ok(obj([
                ("trade_index", t.trade_index.to_string()),
                ("entry_source_index", t.entry_source_index.to_string()),
                ("exit_source_index", t.exit_source_index.to_string()),
                ("open", t.open.to_string()),
                ("entry_allocation", n(t.entry_allocation)?),
                ("gross_return", n(t.gross_return)?),
                ("net_return", n(t.net_return)?),
                ("fee_return", n(t.fee_return)?),
                ("slippage_return", n(t.slippage_return)?),
                ("gross_contribution", n(t.gross_contribution)?),
                ("net_contribution", n(t.net_contribution)?),
                ("fee_contribution", n(t.fee_contribution)?),
                ("slippage_contribution", n(t.slippage_contribution)?),
            ]))
        }))?;
        let day_supplements = array(report.day_supplements.iter().map(|d| {
            Ok(obj([
                (
                    "observed_exposure_session",
                    d.observed_exposure_session.to_string(),
                ),
                ("paid_fee_contribution", n(d.paid_fee_contribution)?),
                (
                    "paid_slippage_contribution",
                    n(d.paid_slippage_contribution)?,
                ),
                (
                    "estimated_exit_fee_contribution",
                    n(d.estimated_exit_fee_contribution)?,
                ),
                (
                    "estimated_exit_slippage_contribution",
                    n(d.estimated_exit_slippage_contribution)?,
                ),
            ]))
        }))?;
        let trade_supplements = array(report.trade_supplements.iter().map(|t| {
            Ok(obj([
                ("gross_factor", n(t.gross_factor)?),
                ("net_factor", n(t.net_factor)?),
                (
                    "gross_curve_entry_allocation",
                    n(t.gross_curve_entry_allocation)?,
                ),
                ("paid_sides", t.paid_sides.to_string()),
                ("estimated_exit_sides", t.estimated_exit_sides.to_string()),
                ("calendar_days_held", t.calendar_days_held.to_string()),
                (
                    "observed_exposure_sessions",
                    t.observed_exposure_sessions.to_string(),
                ),
                ("paid_fee_return", n(t.paid_fee_return)?),
                ("paid_fee_contribution", n(t.paid_fee_contribution)?),
                ("paid_slippage_return", n(t.paid_slippage_return)?),
                (
                    "paid_slippage_contribution",
                    n(t.paid_slippage_contribution)?,
                ),
                ("estimated_exit_fee_return", n(t.estimated_exit_fee_return)?),
                (
                    "estimated_exit_fee_contribution",
                    n(t.estimated_exit_fee_contribution)?,
                ),
                (
                    "estimated_exit_slippage_return",
                    n(t.estimated_exit_slippage_return)?,
                ),
                (
                    "estimated_exit_slippage_contribution",
                    n(t.estimated_exit_slippage_contribution)?,
                ),
            ]))
        }))?;
        let cs = &report.closed_stats;
        let closed_stats = obj([
            ("net", closed(&cs.net)?),
            ("gross", closed(&cs.gross)?),
            ("net_max_drawdown", n(cs.net_max_drawdown)?),
            ("gross_max_drawdown", n(cs.gross_max_drawdown)?),
            (
                "observed_exposure_sessions",
                cs.observed_exposure_sessions.to_string(),
            ),
            ("selected_sessions", cs.selected_sessions.to_string()),
            ("exposure", n(cs.exposure)?),
        ]);
        let spans = array(r.spans.iter().map(|s| {
            Ok(obj([
                ("signal_i", s.signal_i.to_string()),
                ("entry_i", s.entry_i.to_string()),
                ("exit_i", s.exit_i.to_string()),
                ("entry_price", n(s.entry_price)?),
                ("exit_or_mark_price", n(s.exit_or_mark_price)?),
                ("open", s.open.to_string()),
                ("entry_phase", q(entry_phase(s.entry_phase))),
                ("exit_or_mark_phase", q(exit_phase(s.exit_or_mark_phase))),
            ]))
        }))?;
        let ledger = obj([
            (
                "signals",
                array(r.ledger.signals.iter().map(|s| {
                    Ok(obj([
                        ("setup", setup(&s.setup)),
                        ("flat_permission", s.flat_permission.to_string()),
                    ]))
                }))?,
            ),
            ("events", array(r.ledger.events.iter().map(event))?),
            (
                "open_position",
                optional(r.ledger.open_position.as_ref(), position)?,
            ),
            (
                "pending_order",
                r.ledger.pending_order.as_ref().map_or("null".into(), setup),
            ),
        ]);
        let rows = array(
            r.series.rows()[r.from..=r.to]
                .iter()
                .enumerate()
                .map(|(i, b)| {
                    Ok(obj([
                        ("source_index", (r.from + i).to_string()),
                        ("date", q(&b.date)),
                        ("open", n(b.open)?),
                        ("high", n(b.high)?),
                        ("low", n(b.low)?),
                        ("close", n(b.close)?),
                        ("volume", volume(&b.volume)?),
                    ]))
                }),
        )?;
        let costs = obj([("fees_bps",n(self.costs.fees_bps)?),("slippage_bps",n(self.costs.slippage_bps)?),
            ("rollover_bps",n(self.costs.rollover_bps)?),("total_side_bps",n(self.costs.fees_bps+self.costs.slippage_bps)?),
            ("basis",q("Separate return deductions on deployed normalized entry allocation; slippage does not rewrite fills. Closed trades pay two sides. Terminal open marks retain one paid entry side and one estimated exit side; no exit transaction."))]);
        let benchmark = obj([
            ("kind", q("selected_symbol_buy_hold")),
            ("qualified_ticker", q(r.series.qualified_ticker())),
            ("symbol", q(r.series.symbol())),
        ]);
        let statistics = obj([
            ("net", statistics(&self.statistics.net)?),
            ("gross", statistics(&self.statistics.gross)?),
            ("financial", statistics(&self.statistics.financial)?),
            ("benchmark", statistics(&self.statistics.benchmark)?),
        ]);
        let body = obj([
            ("model", q(MODEL)),
            ("convention", q(CONVENTION)),
            ("kind", q("single_symbol_supplied_series_research")),
            ("owner", q("pristine")),
            ("book", q("p_tpm1")),
            ("id", q(id)),
            ("research_mode", q("supplied_usd_series_units")),
            ("context", q("broad_us_supplied_series")),
            ("market", q("us")),
            ("currency", q("USD")),
            ("asset_type", q("individual_stock")),
            ("timeframe", q("D")),
            ("qualified_ticker", q(r.series.qualified_ticker())),
            ("symbol", q(r.series.symbol())),
            ("from_source_index", r.from.to_string()),
            ("to_source_index", r.to.to_string()),
            ("source_row_count", r.series.rows().len().to_string()),
            ("source_prefix_row_count", (r.to + 1).to_string()),
            (
                "volume_column_present",
                r.series.volume_column_present().to_string(),
            ),
            ("basis", q(r.series.basis())),
            ("provenance_json", q(r.series.provenance_json())),
            ("claim", "null".into()),
            ("book_performance_reproduced", "false".into()),
            ("native_platform_parity", "false".into()),
            ("native_historical_dollars_verified", "false".into()),
            ("historical_membership_verified", "false".into()),
            ("benchmark", benchmark),
            ("costs", costs),
            ("rows", rows),
            ("source", ledger),
            ("spans", spans),
            ("days", days),
            ("trade_returns", returns),
            ("day_supplements", day_supplements),
            ("trade_supplements", trade_supplements),
            ("closed_stats", closed_stats),
            ("statistics", statistics),
        ]);
        Ok(obj([("pristine_report", body)]))
    }
}
