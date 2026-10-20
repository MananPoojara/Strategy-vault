//! Scoped API projections for the two reviewed Pristine daily adaptations.
//! No legacy context conversion, index substitution or financial recomputation.
use crate::{
    baskets::Catalogue,
    pristine_request::{self, Failure},
    pristine_stats::StatMap,
    pristine_wire::{self, Prepared},
};
use std::collections::BTreeMap;

pub(crate) const OWNER: &str = "pristine";
pub(crate) const BOOK: &str = "p_tpm1";
pub(crate) const IDS: [&str; 2] = ["ptpm1_daily_cbs_cog", "ptpm1_bullish_gap_surprise"];
pub(crate) const SUMMARY_MODEL: &str = "pristine_daily_supplied_series_summary_v1";

fn q(s: &str) -> String {
    let mut result = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => result.push_str("\\\""),
            '\\' => result.push_str("\\\\"),
            c if c.is_control() => {
                use std::fmt::Write;
                write!(result, "\\u{:04x}", c as u32).unwrap();
            }
            c => result.push(c),
        }
    }
    result.push('"');
    result
}
fn obj(fields: impl IntoIterator<Item = (&'static str, String)>) -> String {
    format!(
        "{{{}}}",
        fields
            .into_iter()
            .map(|(k, v)| format!("{}:{v}", q(k)))
            .collect::<Vec<_>>()
            .join(",")
    )
}
fn strings(values: &[&str]) -> String {
    format!(
        "[{}]",
        values.iter().map(|v| q(v)).collect::<Vec<_>>().join(",")
    )
}
fn number(v: f64) -> Result<String, pristine_wire::Error> {
    if v.is_finite() {
        Ok(v.to_string())
    } else {
        Err(pristine_wire::Error::Serialization(
            "pristine_api_nonfinite",
        ))
    }
}
fn values(s: &StatMap) -> Result<String, pristine_wire::Error> {
    Ok(obj(s
        .values
        .iter()
        .map(|(key, value)| Ok((*key, value.map_or(Ok("null".into()), number)?)))
        .collect::<Result<Vec<_>, pristine_wire::Error>>()?))
}
fn reasons(s: &StatMap) -> String {
    obj(s.null_reasons.iter().map(|(key, value)| (*key, q(value))))
}
pub(crate) fn failure_json(f: &Failure) -> String {
    let index = |v: Option<usize>| v.map_or("null".into(), |v| v.to_string());
    obj([
        ("error", q(note(f))),
        (
            "pristine_failure",
            obj([
                ("stage", q(f.stage)),
                ("code", q(f.code)),
                ("at", index(f.at)),
                ("signal_i", index(f.signal_i)),
                ("source_i", index(f.source_i)),
            ]),
        ),
    ])
}
fn note(f: &Failure) -> &'static str {
    match f.code {
        "explicit_research_mode_required" => "Select supplied-series research explicitly. Historical-dollar price equivalence is unverified.",
        "native_historical_dollars_unverified" => "The supplied source does not establish historical-dollar equivalence. No alternative mode was substituted.",
        "pristine_unsupported_instrument" | "pristine_unknown_qualified_ticker" | "pristine_qualified_ticker_required" => "Select an actual US stock from the supplied daily USD datasets. An index or ETF cannot substitute for it.",
        "pristine_requested_window_outside_coverage" | "pristine_empty_covered_range" => "The requested dates are outside covered source sessions. Choose dates within the selected stock's coverage.",
        "pristine_invalid_date" | "pristine_invalid_range" => "Choose a valid date range. The requested window has not been shortened or replaced.",
        "pristine_rollover_unsupported" => "Rollover is unsupported for this cash-stock research model.",
        "pristine_fixed_source_rules" => "This branch uses fixed source rules. Parameter grids and overrides are unavailable.",
        "pristine_server_busy" => "A Pristine research request is already running. Retry after it finishes; no calculation was started for this request.",
        _ if f.stage == "source" => "Required source rules or a candidate's data dependencies could not be resolved. No result is published for this run.",
        _ if f.stage == "accounting" || f.stage == "statistics" || f.stage == "spans" => "This run did not pass financial-report admission. No partial performance result is published.",
        _ if f.stage == "data" => "The selected source could not be admitted. No substitute dataset or fabricated observation was used.",
        _ => "The request is unavailable under this fixed Pristine research model. Review the selected stock, dates, mode and costs.",
    }
}

const CBS_RULES: &[&str] = &[
    "Signal: the selected daily branch requires a maximal run of exactly five red candles followed by a green candle. The signal close is at least 8% below its inclusive 20-session SMA.",
    "Entry: only the next session may buy at the signal high + 0.05 supplied-series units; a gap above that trigger fills at the actual open.",
    "Protection: signal low - 0.05. The target is the fixed signal-day SMA. From the third held session, prior-session low - 0.05 replaces the old stop only when it raises that stop.",
    "Exit: protective stop, fixed target, or the mature full-gap opening exit. Declared daily-OHLC ordering resolves intrabar ambiguity adversely.",
];
const BGS_RULES: &[&str] = &[
    "Signal: the last two daily candles are red (a longer red run is allowed). Signal range is at least 1.75 supplied-series units, with open in the top 20% and close in the bottom 20%.",
    "Volume: on a completed price candidate only, signal volume must exceed the mean of the previous 20 sessions. All 21 required observations must be usable.",
    "Entry: the immediately following open must be at least 0.50 supplied-series units above the signal close. This branch buys at that actual open.",
    "Exit: signal low - 0.05 stop, actual entry + 2 target, otherwise the second held session's close. Declared daily-OHLC ordering resolves intrabar ambiguity adversely.",
];
const SOURCE_BASIS: &str = "Trading the Pristine Method, Book 1, printed pp. 173–174 and 267–268. These are selected daily branches on broad supplied US stocks. Constants use stored series units; preferred source universes, intraday execution and historical-dollar price equivalence are not reproduced. Book-native performance remains unverified.";

pub(crate) fn catalogue_json() -> String {
    let rows = IDS.iter().enumerate().map(|(i,id)| {
        let (name,rules,variant) = if i == 0 {
            ("Pristine · Daily CBS / COG", CBS_RULES, "Exactly five red candles")
        } else { ("Pristine · Bullish Gap Surprise", BGS_RULES, "Immediate open entry") };
        obj([
            ("id",q(id)), ("owner",q(OWNER)), ("name",q(name)), ("group",q("Pristine · Daily stock patterns")),
            ("session","0".into()), ("dir",q("Long")), ("status",q("live")),
            ("status_note",q("Source-reviewed daily research adaptation. Choose an actual supplied US stock and explicit research mode.")),
            ("needs_volume",(i == 1).to_string()), ("needs_vix","false".into()), ("needs_series","[]".into()),
            ("volume_requirement",q(if i == 1 {"candidate_signal_and_prior_20_sessions"} else {"none"})),
            ("book",q(BOOK)), ("family",q(id)), ("family_name",q(name)), ("variant",q(variant)),
            ("rules",strings(rules)), ("notes",q(SOURCE_BASIS)), ("claim","null".into()), ("gate","null".into()),
            ("instrument_policy",obj([
                ("requirement",q("individual_stock")), ("allowed_types",strings(&["individual_stock"])), ("allowed_markets",strings(&["us"])),
                ("execution_scope",q("single_symbol_supplied_series_research")),
                ("source_convention",q(pristine_wire::CONVENTION)), ("research_mode",q("supplied_usd_series_units")),
                ("source_rule_reviewed","true".into()), ("independent_selected_ledger_checked","true".into()),
                ("source_performance_reproduced","false".into()), ("market_adaptation","true".into()),
                ("source_basis",q(SOURCE_BASIS)),
            ])),
        ])
    }).collect::<Vec<_>>();
    format!("[{}]", rows.join(","))
}
pub(crate) fn params_json() -> String {
    obj(IDS.into_iter().map(|id| (id, "[]".into())))
}

fn selected_queries(
    query: &BTreeMap<String, String>,
) -> Result<Vec<BTreeMap<String, String>>, Failure> {
    let ids: Vec<&str> = match query.get("id") {
        Some(id) if pristine_request::applies(id) => vec![id],
        Some(_) => {
            return Err(Failure {
                stage: "request",
                code: "pristine_unknown_method",
                at: None,
                signal_i: None,
                source_i: None,
            })
        }
        None => IDS.to_vec(),
    };
    Ok(ids
        .into_iter()
        .map(|id| {
            let mut q = query.clone();
            q.insert("id".into(), id.into());
            q
        })
        .collect())
}
fn cost_json(p: &Prepared<'_>) -> Result<String, pristine_wire::Error> {
    let c = p.costs();
    Ok(obj([
        ("fees_bps", number(c.fees_bps)?),
        ("slippage_bps", number(c.slippage_bps)?),
        ("rollover_bps", number(c.rollover_bps)?),
        ("total_side_bps", number(c.fees_bps + c.slippage_bps)?),
        ("basis", q("Separate return deductions on deployed normalized entry allocation; slippage does not rewrite fills. Closed trades pay two sides. Terminal open marks retain one paid entry side and one estimated exit side; no exit transaction.")),
    ]))
}
fn summary_row(p: &Prepared<'_>) -> Result<String, pristine_wire::Error> {
    let series = p.series();
    let (from, to) = p.bounds();
    let days = &p.report().days;
    // Spark samples are unchanged observations for list presentation only.
    // Source admission and every financial statistic use the complete run.
    let mut indices: Vec<usize> = (0..days.len())
        .step_by(days.len().div_ceil(120).max(1))
        .collect();
    if indices.last().copied() != days.len().checked_sub(1) {
        indices.push(days.len() - 1);
    }
    let dated = indices
        .iter()
        .map(|&i| {
            Ok(obj([
                ("date", q(&days[i].date)),
                ("equity", number(days[i].equity)?),
            ]))
        })
        .collect::<Result<Vec<_>, pristine_wire::Error>>()?;
    let spark = indices
        .iter()
        .map(|&i| number(days[i].equity))
        .collect::<Result<Vec<_>, _>>()?;
    let stats = &p.statistics().financial;
    Ok(obj([
        ("id", q(p.id())),
        ("stats", values(stats)?),
        ("null_reasons", reasons(stats)),
        ("spark", format!("[{}]", spark.join(","))),
        ("costs", cost_json(p)?),
        (
            "pristine_summary",
            obj([
                ("model", q(SUMMARY_MODEL)),
                ("report_model", q(pristine_wire::MODEL)),
                ("owner", q(OWNER)),
                ("book", q(BOOK)),
                ("id", q(p.id())),
                ("research_mode", q("supplied_usd_series_units")),
                ("convention", q(pristine_wire::CONVENTION)),
                ("market", q("us")),
                ("currency", q("USD")),
                ("asset_type", q("individual_stock")),
                ("qualified_ticker", q(series.qualified_ticker())),
                ("symbol", q(series.symbol())),
                ("from", q(&series.rows()[from].date)),
                ("to", q(&series.rows()[to].date)),
                ("selected_sessions", days.len().to_string()),
                ("basis", q(series.basis())),
                ("provenance_json", q(series.provenance_json())),
                ("book_performance_reproduced", "false".into()),
                ("native_historical_dollars_verified", "false".into()),
                ("historical_membership_verified", "false".into()),
                ("dated_spark", format!("[{}]", dated.join(","))),
            ]),
        ),
    ]))
}
fn blocked(id: &str, f: &Failure) -> String {
    obj([
        ("id", q(id)),
        ("blocked", q(note(f))),
        (
            "readiness",
            q(if f.stage == "request" {
                "not_testable"
            } else {
                "needs_data"
            }),
        ),
        ("failure", failure_json(f)),
    ])
}
pub(crate) fn summary(
    catalogue: &Catalogue,
    query: &BTreeMap<String, String>,
) -> Result<String, Failure> {
    let rows = selected_queries(query)?
        .iter()
        .map(|query| {
            pristine_request::execute_with(catalogue, query, summary_row)
                .unwrap_or_else(|f| blocked(&query["id"], &f))
        })
        .collect::<Vec<_>>();
    Ok(format!("[{}]", rows.join(",")))
}
pub(crate) fn readiness(
    catalogue: &Catalogue,
    query: &BTreeMap<String, String>,
) -> Result<String, Failure> {
    let rows=selected_queries(query)?.iter().map(|query| {
        pristine_request::execute_with(catalogue,query,|p| Ok(obj([
            ("id",q(p.id())), ("status",q("ready")),
            ("note",q("Source, required data and accounting checks passed for this supplied-series run. Original book performance and historical-dollar equivalence remain unverified.")),
        ]))).unwrap_or_else(|f|obj([
            ("id",q(&query["id"])), ("status",q(if f.stage=="request" {"not_testable"} else {"needs_data"})),
            ("note",q(note(&f))), ("failure",failure_json(&f)),
        ]))
    }).collect::<Vec<_>>();
    Ok(obj([
        ("ticker", q(query.get("ticker").map_or("", String::as_str))),
        ("strategies", format!("[{}]", rows.join(","))),
    ]))
}
