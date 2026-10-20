//! Pristine normalized-unit accounting. Source events and instrument/data
//! admission are separate prerequisites; this component cannot certify either.
//! Existing DEMA report validation and serialization are not changed.
use crate::{
    accounting::Costs,
    pristine_daily::{self, Bar},
    unit_exposure_arithmetic as exact,
    unit_exposure_report::{Day, TradeReturn},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EntryPhase {
    Open,
    Stop,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ExitPhase {
    OpenStop,
    OpenTarget,
    OpenFullGap,
    Stop,
    Target,
    Close,
    Mark,
}

/// Authoritative fills/marks supplied by the separately validated source model.
/// Synthetic accounting tests may construct spans; that does not admit a trade
/// as a book signal or as a result on an actual instrument.
#[derive(Clone, Debug)]
pub(crate) struct Span {
    pub signal_i: usize,
    pub entry_i: usize,
    pub exit_i: usize,
    pub entry_price: f64,
    pub exit_or_mark_price: f64,
    pub open: bool,
    pub entry_phase: EntryPhase,
    pub exit_or_mark_phase: ExitPhase,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Error {
    pub at: usize,
    pub code: &'static str,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct DaySupplement {
    pub observed_exposure_session: bool,
    pub paid_fee_contribution: f64,
    pub paid_slippage_contribution: f64,
    pub estimated_exit_fee_contribution: f64,
    pub estimated_exit_slippage_contribution: f64,
}

#[derive(Clone, Debug)]
pub(crate) struct TradeSupplement {
    pub gross_factor: f64,
    pub net_factor: f64,
    pub gross_curve_entry_allocation: f64,
    pub paid_sides: u8,
    pub estimated_exit_sides: u8,
    pub calendar_days_held: i64,
    pub observed_exposure_sessions: usize,
    pub paid_fee_return: f64,
    pub paid_fee_contribution: f64,
    pub paid_slippage_return: f64,
    pub paid_slippage_contribution: f64,
    pub estimated_exit_fee_return: f64,
    pub estimated_exit_fee_contribution: f64,
    pub estimated_exit_slippage_return: f64,
    pub estimated_exit_slippage_contribution: f64,
}

#[derive(Clone, Debug)]
pub(crate) struct ClosedView {
    pub closed_count: usize,
    pub closed_wins: usize,
    pub closed_nonwins: usize,
    pub closed_return_sum: f64,
    pub closed_average_return: Option<f64>,
    pub closed_win_rate: Option<f64>,
    pub closed_compound_equity: f64,
    pub closed_compound_return: f64,
}

#[derive(Clone, Debug)]
pub(crate) struct ClosedStats {
    pub net: ClosedView,
    pub gross: ClosedView,
    pub net_max_drawdown: f64,
    pub gross_max_drawdown: f64,
    pub observed_exposure_sessions: usize,
    pub selected_sessions: usize,
    pub exposure: f64,
}

#[derive(Clone, Debug)]
pub(crate) struct Report {
    pub days: Vec<Day>,
    pub trade_returns: Vec<TradeReturn>,
    pub day_supplements: Vec<DaySupplement>,
    pub trade_supplements: Vec<TradeSupplement>,
    pub closed_stats: ClosedStats,
}

fn arithmetic<T>(at: usize, value: Result<T, exact::Error>) -> Result<T, Error> {
    value.map_err(|e| Error { at, code: e.code() })
}
fn positive(at: usize, value: f64) -> Result<f64, Error> {
    if value.is_finite() && value > 0. {
        Ok(value)
    } else {
        Err(Error {
            at,
            code: "unit_exposure_equity_domain",
        })
    }
}
fn product(at: usize, a: f64, b: f64) -> Result<f64, Error> {
    arithmetic(at, exact::checked_product(a, b))
}
fn add(at: usize, a: f64, b: f64) -> Result<f64, Error> {
    let value = a + b;
    if value.is_finite() {
        Ok(value)
    } else {
        Err(Error {
            at,
            code: "reporting_arithmetic_overflow",
        })
    }
}
fn scalars(
    at: usize,
    entry: f64,
    mark: f64,
    costs: Costs,
    sides: u8,
) -> Result<exact::ModelScalars, Error> {
    arithmetic(
        at,
        exact::evaluate(entry, mark, costs.fees_bps, costs.slippage_bps, sides),
    )
}
fn relative(at: usize, previous: f64, current: f64) -> Result<f64, Error> {
    arithmetic(
        at,
        scalars(at, previous, current, Costs::default(), 0)?.gross,
    )
}

fn validate_span(
    bars: &[Bar],
    from: usize,
    to: usize,
    next: usize,
    s: &Span,
    last: bool,
) -> Result<(), Error> {
    let invalid = || Error {
        at: s.entry_i,
        code: "pristine_report_shape",
    };
    if s.entry_i < next
        || s.entry_i < from
        || s.entry_i > s.exit_i
        || s.exit_i > to
        || s.signal_i < from
        || s.signal_i > s.entry_i
        || s.open != (s.exit_or_mark_phase == ExitPhase::Mark)
        || s.open && (!last || s.exit_i != to)
        || ![s.entry_price, s.exit_or_mark_price]
            .iter()
            .all(|p| p.is_finite() && *p > 0.)
    {
        return Err(invalid());
    }
    let (entry, exit) = (&bars[s.entry_i], &bars[s.exit_i]);
    let entry_valid = match s.entry_phase {
        EntryPhase::Open => s.entry_price == entry.o,
        EntryPhase::Stop => s.entry_price >= entry.l && s.entry_price <= entry.h,
    };
    let exit_valid = match s.exit_or_mark_phase {
        ExitPhase::OpenStop | ExitPhase::OpenTarget | ExitPhase::OpenFullGap => {
            s.exit_or_mark_price == exit.o
        }
        ExitPhase::Stop | ExitPhase::Target => {
            s.exit_or_mark_price >= exit.l && s.exit_or_mark_price <= exit.h
        }
        ExitPhase::Close | ExitPhase::Mark => s.exit_or_mark_price == exit.c,
    };
    let exit_precedes_entry = s.entry_i == s.exit_i
        && s.entry_phase == EntryPhase::Stop
        && matches!(
            s.exit_or_mark_phase,
            ExitPhase::OpenStop | ExitPhase::OpenTarget | ExitPhase::OpenFullGap
        );
    if !entry_valid || !exit_valid || exit_precedes_entry {
        return Err(invalid());
    }
    Ok(())
}

fn summarize(
    rows: &[TradeReturn],
    supplements: &[TradeSupplement],
    gross: bool,
    at: usize,
) -> Result<ClosedView, Error> {
    let (mut count, mut wins, mut sum, mut compound) = (0usize, 0usize, 0., 1.);
    for (row, extra) in rows.iter().zip(supplements).filter(|(row, _)| !row.open) {
        let value = if gross {
            row.gross_return
        } else {
            row.net_return
        };
        let factor = if gross {
            extra.gross_factor
        } else {
            extra.net_factor
        };
        count += 1;
        wins += usize::from(value > 0.);
        sum = add(at, sum, value)?;
        compound = positive(at, product(at, compound, factor)?)?;
    }
    let average = if count == 0 {
        None
    } else {
        let value = sum / count as f64;
        if sum != 0. && value == 0. {
            return Err(Error {
                at,
                code: "reporting_arithmetic_underflow",
            });
        }
        Some(value)
    };
    Ok(ClosedView {
        closed_count: count,
        closed_wins: wins,
        closed_nonwins: count - wins,
        closed_return_sum: sum,
        closed_average_return: average,
        closed_win_rate: (count > 0).then(|| wins as f64 / count as f64),
        closed_compound_equity: compound,
        closed_compound_return: relative(at, 1., compound)?,
    })
}

fn max_drawdown(values: &[f64], from: usize) -> Result<f64, Error> {
    let (mut peak, mut worst) = (1.0f64, 0.0f64);
    for (k, &value) in values.iter().enumerate() {
        peak = peak.max(value);
        worst = worst.min(relative(from + k, peak, value)?);
    }
    Ok(worst)
}

/// Fresh selected account at1. A held close mark deducts one entry side;
/// terminal-open marks include a separately identified estimated second side.
/// Both paid sides of a same-session round trip appear on that session.
pub(crate) fn build(
    bars: &[Bar],
    from: usize,
    to: usize,
    spans: &[Span],
    costs: Costs,
) -> Result<Report, Error> {
    if from > to || to >= bars.len() {
        return Err(Error {
            at: from,
            code: "invalid_range",
        });
    }
    pristine_daily::validate(bars, to).map_err(|e| Error {
        at: e.at,
        code: e.code,
    })?;
    if !costs.fees_bps.is_finite()
        || costs.fees_bps < 0.
        || !costs.slippage_bps.is_finite()
        || costs.slippage_bps < 0.
    {
        return Err(Error {
            at: from,
            code: "invalid_domain",
        });
    }
    if !costs.rollover_bps.is_finite() || costs.rollover_bps != 0. {
        return Err(Error {
            at: from,
            code: "unsupported_cash_stock_rollover",
        });
    }
    let n = to - from + 1;
    let (mut net, mut gross) = (vec![1.; n], vec![1.; n]);
    let mut day_extra = vec![DaySupplement::default(); n];
    let (mut allocation, mut gross_allocation, mut next) = (1., 1., from);
    let (mut rows, mut extras) = (Vec::new(), Vec::new());
    for (ti, s) in spans.iter().enumerate() {
        validate_span(bars, from, to, next, s, ti + 1 == spans.len())?;
        for k in next..s.entry_i {
            net[k - from] = allocation;
            gross[k - from] = gross_allocation;
        }
        for k in s.entry_i..=s.exit_i {
            let (mark, sides) = if k == s.exit_i {
                (s.exit_or_mark_price, 2)
            } else {
                (bars[k].c, 1)
            };
            let values = scalars(k, s.entry_price, mark, costs, sides)?;
            let (nf, gf) = (
                arithmetic(k, values.net_factor)?,
                arithmetic(k, values.gross_factor)?,
            );
            net[k - from] = positive(k, product(k, allocation, nf)?)?;
            gross[k - from] = positive(k, product(k, gross_allocation, gf)?)?;
            day_extra[k - from].observed_exposure_session = true;
        }
        let at = s.exit_i;
        let final_values = scalars(at, s.entry_price, s.exit_or_mark_price, costs, 2)?;
        let gross_return = arithmetic(at, final_values.gross)?;
        let net_return = arithmetic(at, final_values.net)?;
        let fee_return = arithmetic(at, final_values.fee)?;
        let slippage_return = arithmetic(at, final_values.slippage)?;
        let (gross_factor, net_factor) = (
            arithmetic(at, final_values.gross_factor)?,
            arithmetic(at, final_values.net_factor)?,
        );
        let paid_sides = if s.open { 1 } else { 2 };
        let paid = scalars(at, s.entry_price, s.exit_or_mark_price, costs, paid_sides)?;
        let estimate = scalars(
            at,
            s.entry_price,
            s.exit_or_mark_price,
            costs,
            u8::from(s.open),
        )?;
        let paid_fee = arithmetic(at, paid.fee)?;
        let paid_slippage = arithmetic(at, paid.slippage)?;
        let estimate_fee = arithmetic(at, estimate.fee)?;
        let estimate_slippage = arithmetic(at, estimate.slippage)?;
        let one = scalars(s.entry_i, s.entry_price, s.entry_price, costs, 1)?;
        let one_fee = product(s.entry_i, allocation, arithmetic(s.entry_i, one.fee)?)?;
        let one_slippage = product(s.entry_i, allocation, arithmetic(s.entry_i, one.slippage)?)?;
        let entry_day = &mut day_extra[s.entry_i - from];
        entry_day.paid_fee_contribution = add(s.entry_i, entry_day.paid_fee_contribution, one_fee)?;
        entry_day.paid_slippage_contribution = add(
            s.entry_i,
            entry_day.paid_slippage_contribution,
            one_slippage,
        )?;
        let exit_day = &mut day_extra[at - from];
        if s.open {
            exit_day.estimated_exit_fee_contribution =
                add(at, exit_day.estimated_exit_fee_contribution, one_fee)?;
            exit_day.estimated_exit_slippage_contribution = add(
                at,
                exit_day.estimated_exit_slippage_contribution,
                one_slippage,
            )?;
        } else {
            exit_day.paid_fee_contribution = add(at, exit_day.paid_fee_contribution, one_fee)?;
            exit_day.paid_slippage_contribution =
                add(at, exit_day.paid_slippage_contribution, one_slippage)?;
        }
        rows.push(TradeReturn {
            trade_index: ti,
            entry_source_index: s.entry_i,
            exit_source_index: at,
            open: s.open,
            entry_allocation: allocation,
            gross_return,
            net_return,
            fee_return,
            slippage_return,
            gross_contribution: product(at, allocation, gross_return)?,
            net_contribution: product(at, allocation, net_return)?,
            fee_contribution: product(at, allocation, fee_return)?,
            slippage_contribution: product(at, allocation, slippage_return)?,
        });
        extras.push(TradeSupplement {
            gross_factor,
            net_factor,
            gross_curve_entry_allocation: gross_allocation,
            paid_sides,
            estimated_exit_sides: u8::from(s.open),
            calendar_days_held: crate::day_number(&bars[at].date)
                - crate::day_number(&bars[s.entry_i].date),
            observed_exposure_sessions: at - s.entry_i + 1,
            paid_fee_return: paid_fee,
            paid_fee_contribution: product(at, allocation, paid_fee)?,
            paid_slippage_return: paid_slippage,
            paid_slippage_contribution: product(at, allocation, paid_slippage)?,
            estimated_exit_fee_return: estimate_fee,
            estimated_exit_fee_contribution: product(at, allocation, estimate_fee)?,
            estimated_exit_slippage_return: estimate_slippage,
            estimated_exit_slippage_contribution: product(at, allocation, estimate_slippage)?,
        });
        allocation = net[at - from];
        gross_allocation = gross[at - from];
        next = at + 1;
    }
    for k in next..=to {
        net[k - from] = allocation;
        gross[k - from] = gross_allocation;
    }
    let benchmark = (from..=to)
        .map(|k| {
            let value = scalars(k, bars[from].c, bars[k].c, Costs::default(), 0)?;
            positive(k, arithmetic(k, value.gross_factor)?)
        })
        .collect::<Result<Vec<_>, Error>>()?;
    let (mut previous, mut previous_gross, mut previous_benchmark) = (1., 1., 1.);
    let mut days = Vec::with_capacity(n);
    for k in from..=to {
        let j = k - from;
        days.push(Day {
            source_index: k,
            date: bars[k].date.clone(),
            equity: net[j],
            gross_equity: gross[j],
            benchmark_equity: benchmark[j],
            net_daily_return: relative(k, previous, net[j])?,
            gross_daily_return: relative(k, previous_gross, gross[j])?,
            benchmark_daily_return: relative(k, previous_benchmark, benchmark[j])?,
        });
        previous = net[j];
        previous_gross = gross[j];
        previous_benchmark = benchmark[j];
    }
    let exposure = day_extra
        .iter()
        .filter(|d| d.observed_exposure_session)
        .count();
    let stats = ClosedStats {
        net: summarize(&rows, &extras, false, to)?,
        gross: summarize(&rows, &extras, true, to)?,
        net_max_drawdown: max_drawdown(&net, from)?,
        gross_max_drawdown: max_drawdown(&gross, from)?,
        observed_exposure_sessions: exposure,
        selected_sessions: n,
        exposure: exposure as f64 / n as f64,
    };
    Ok(Report {
        days,
        trade_returns: rows,
        day_supplements: day_extra,
        trade_supplements: extras,
        closed_stats: stats,
    })
}
