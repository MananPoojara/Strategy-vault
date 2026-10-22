//! Scoped research statistics over the accepted Pristine report. Undefined
//! ratios retain explicit reasons; source returns/factors are never rebuilt.
use crate::{
    pristine_report::{ClosedView, Report, Span},
    unit_exposure_arithmetic as exact,
};
use std::collections::BTreeMap;

type Value = Result<f64, &'static str>;

#[derive(Clone, Debug, Default)]
pub(crate) struct StatMap {
    pub values: BTreeMap<&'static str, Option<f64>>,
    pub null_reasons: BTreeMap<&'static str, &'static str>,
}
impl StatMap {
    fn put(&mut self, key: &'static str, value: Value) {
        match value.and_then(finite) {
            Ok(value) => {
                self.values.insert(key, Some(value));
            }
            Err(reason) => {
                self.values.insert(key, None);
                self.null_reasons.insert(key, reason);
            }
        }
    }
    fn alias(&mut self, key: &'static str, other: &Self, source: &'static str) {
        self.put(
            key,
            other.values[source].ok_or_else(|| other.null_reasons[source]),
        );
    }
}

#[derive(Clone, Debug)]
pub(crate) struct Statistics {
    pub net: StatMap,
    pub gross: StatMap,
    pub financial: StatMap,
    pub benchmark: StatMap,
}

#[derive(Clone, Debug)]
pub(crate) struct StatsError {
    pub at: usize,
    pub code: &'static str,
}

fn finite(value: f64) -> Value {
    value
        .is_finite()
        .then_some(value)
        .ok_or("arithmetic_nonfinite")
}
fn multiply(a: f64, b: f64) -> Value {
    let value = finite(a * b)?;
    if a != 0. && b != 0. && value == 0. {
        Err("arithmetic_underflow")
    } else {
        Ok(value)
    }
}
fn divide(a: f64, b: f64) -> Value {
    let value = finite(a / b)?;
    if a != 0. && value == 0. {
        Err("arithmetic_underflow")
    } else {
        Ok(value)
    }
}
fn sum(values: impl IntoIterator<Item = f64>) -> Value {
    let mut total = 0.;
    for value in values {
        total = finite(total + value)?;
    }
    Ok(total)
}
fn average(values: &[f64], empty: &'static str) -> Value {
    if values.is_empty() {
        return Err(empty);
    }
    divide(sum(values.iter().copied())?, values.len() as f64)
}
fn relative(previous: f64, current: f64) -> Value {
    let map = |error: exact::Error| {
        if error.code().contains("underflow") {
            "arithmetic_underflow"
        } else {
            "arithmetic_nonfinite"
        }
    };
    exact::evaluate(previous, current, 0., 0., 0)
        .map_err(map)?
        .gross
        .map_err(map)
}
fn annualized(equity: f64, years: f64) -> Value {
    if years == 0. {
        return Err("zero_elapsed_years");
    }
    let power = finite(equity.powf(1. / years))?;
    if equity != 1. && power == 1. {
        return Err("arithmetic_underflow");
    }
    finite(power - 1.)
}
fn sample_sd(values: &[f64], few: &'static str) -> Value {
    if values.len() < 2 {
        return Err(few);
    }
    let mean = average(values, few)?;
    let mut squares = 0.;
    for &value in values {
        let deviation = finite(value - mean)?;
        squares = finite(squares + multiply(deviation, deviation)?)?;
    }
    let variance = divide(squares, (values.len() - 1) as f64)?;
    finite(variance.sqrt())
}
fn downside_sd(values: &[f64]) -> Value {
    if values.len() < 2 {
        return Err("fewer_two_daily_observations");
    }
    let mut squares = 0.;
    for &value in values {
        let downside = value.min(0.);
        squares = finite(squares + multiply(downside, downside)?)?;
    }
    finite(divide(squares, values.len() as f64)?.sqrt())
}
fn risk_ratio(mean: Value, deviation: Value, zero: &'static str) -> Value {
    let deviation = deviation?;
    if deviation == 0. {
        return Err(zero);
    }
    multiply(divide(mean?, deviation)?, 252f64.sqrt())
}
fn drawdown(curve: &[f64]) -> Value {
    let (mut peak, mut worst) = (1f64, 0f64);
    for &value in curve {
        peak = peak.max(value);
        worst = worst.min(relative(peak, value)?);
    }
    Ok(worst)
}
fn drawdown_sessions(curve: &[f64]) -> f64 {
    let (mut peak, mut peak_position, mut worst) = (1., 0, 0);
    for (i, &value) in curve.iter().enumerate() {
        let position = i + 1;
        if value >= peak {
            peak = value;
            peak_position = position;
        } else {
            worst = worst.max(position - peak_position);
        }
    }
    worst as f64
}

#[derive(Clone, Copy)]
struct CurveStats {
    total: Value,
    cagr: Value,
    vol: Value,
    sharpe: Value,
    sortino: Value,
    dd: Value,
    dd_sessions: f64,
    mar: Value,
}
fn curve_stats(curve: &[f64], daily: &[f64], years: f64, dd: Value) -> CurveStats {
    let mean = average(daily, "fewer_two_daily_observations");
    let sd = sample_sd(daily, "fewer_two_daily_observations");
    let cagr = annualized(*curve.last().expect("validated nonempty report"), years);
    let mar = cagr.and_then(|cagr| {
        dd.and_then(|dd| {
            if dd < 0. {
                divide(cagr, -dd)
            } else {
                Err("zero_drawdown")
            }
        })
    });
    CurveStats {
        total: relative(1., *curve.last().unwrap()),
        cagr,
        vol: sd.and_then(|sd| multiply(sd, 252f64.sqrt())),
        sharpe: risk_ratio(mean, sd, "zero_daily_standard_deviation"),
        sortino: risk_ratio(mean, downside_sd(daily), "zero_downside_deviation"),
        dd,
        dd_sessions: drawdown_sessions(curve),
        mar,
    }
}

fn common(
    report: &Report,
    spans: &[Span],
    summary: &ClosedView,
    gross: bool,
    curves: CurveStats,
    benchmark: CurveStats,
    years: f64,
) -> StatMap {
    let closed: Vec<_> = report.trade_returns.iter().filter(|r| !r.open).collect();
    let returns: Vec<_> = closed
        .iter()
        .map(|r| if gross { r.gross_return } else { r.net_return })
        .collect();
    let wins: Vec<_> = returns.iter().copied().filter(|r| *r > 0.).collect();
    let nonwins: Vec<_> = returns.iter().copied().filter(|r| *r <= 0.).collect();
    let avg_win = average(&wins, "no_winning_closed_trades");
    let avg_loss = average(&nonwins, "no_nonwinning_closed_trades");
    let avg = summary.closed_average_return.ok_or("no_closed_trades");
    let win_rate = summary.closed_win_rate.ok_or("no_closed_trades");
    let n = summary.closed_count;
    let mut sorted = returns.clone();
    sorted.sort_by(f64::total_cmp);
    let median = match sorted.len() {
        0 => Err("no_closed_trades"),
        n if n % 2 == 1 => Ok(sorted[n / 2]),
        n => finite(sorted[n / 2 - 1] + sorted[n / 2]).and_then(|sum| divide(sum, 2.)),
    };
    let payoff = if wins.is_empty() || nonwins.is_empty() {
        Err("no_winners_or_negative_average_nonwin")
    } else {
        avg_loss.and_then(|loss| {
            if loss < 0. {
                divide(avg_win?, -loss)
            } else {
                Err("no_winners_or_negative_average_nonwin")
            }
        })
    };
    let loss_sum = sum(nonwins.iter().copied());
    let profit_factor = loss_sum.and_then(|loss| {
        if loss < 0. {
            divide(sum(wins.iter().copied())?, -loss)
        } else {
            Err("zero_closed_loss_magnitude")
        }
    });
    let t_stat = sample_sd(&returns, "fewer_two_closed_trades").and_then(|sd| {
        if sd == 0. {
            Err("zero_closed_standard_deviation")
        } else {
            divide(avg?, divide(sd, (n as f64).sqrt())?)
        }
    });
    let (mut streak, mut longest) = (0usize, 0usize);
    for &value in &returns {
        streak = if value <= 0. { streak + 1 } else { 0 };
        longest = longest.max(streak);
    }
    let lengths: Vec<_> = spans
        .iter()
        .filter(|s| !s.open)
        .map(|s| (s.exit_i - s.entry_i) as f64)
        .collect();
    let calendar: Vec<_> = report
        .trade_returns
        .iter()
        .zip(&report.trade_supplements)
        .filter(|(r, _)| !r.open)
        .map(|(_, s)| s.calendar_days_held as f64)
        .collect();
    let hold = average(&calendar, "no_closed_trades");
    let thickness = match hold {
        Ok(hold) if hold > 0. => avg
            .and_then(|avg| divide(avg, hold))
            .and_then(|rate| multiply(rate, 30.)),
        Ok(_) | Err("no_closed_trades") => Err("no_positive_average_calendar_hold"),
        Err(reason) => Err(reason),
    };
    let expectancy = match avg_loss {
        Ok(loss) if n > 0 && loss < 0. => {
            let calculate = || -> Value {
                let magnitude = -loss;
                let w = summary.closed_wins as f64 / n as f64;
                let aw = if wins.is_empty() { 0. } else { avg_win? };
                let win_part = multiply(w, aw)?;
                let loss_part = multiply(1. - w, magnitude)?;
                divide(finite(win_part - loss_part)?, magnitude)
            };
            calculate()
        }
        Ok(_) | Err("no_nonwinning_closed_trades") => Err("no_negative_average_nonwin"),
        Err(reason) => Err(reason),
    };
    let open = report.trade_returns.iter().rev().find(|r| r.open);
    let mut s = StatMap::default();
    for (key, value) in [
        ("total_return", curves.total),
        ("compounded_return", curves.total),
        ("cagr", curves.cagr),
        ("vol", curves.vol),
        ("sharpe", curves.sharpe),
        ("sortino", curves.sortino),
        ("max_dd", curves.dd),
        ("max_dd_days", Ok(curves.dd_sessions)),
        ("mar", curves.mar),
        ("car_mdd", curves.mar),
        ("exposure", Ok(report.closed_stats.exposure)),
        ("bh_total", benchmark.total),
        ("bh_cagr", benchmark.cagr),
        ("bh_max_dd", benchmark.dd),
        ("years", Ok(years)),
        ("trades", Ok(n as f64)),
        ("win_rate", win_rate),
        ("avg_trade", avg),
        ("median_trade", median),
        ("avg_win", avg_win),
        ("avg_loss", avg_loss),
        ("payoff", payoff),
        ("profit_factor", profit_factor),
        ("best", sorted.last().copied().ok_or("no_closed_trades")),
        ("worst", sorted.first().copied().ok_or("no_closed_trades")),
        ("t_stat", t_stat),
        ("max_consec_losses", Ok(longest as f64)),
        ("avg_bars", average(&lengths, "no_closed_trades")),
        (
            "avg_deployed",
            if n > 0 {
                Ok(1.)
            } else {
                Err("no_closed_trades")
            },
        ),
        (
            "open_trade",
            open.map(|r| if gross { r.gross_return } else { r.net_return })
                .ok_or("no_open_trade"),
        ),
        (
            "trades_per_year",
            if years > 0. {
                divide(n as f64, years)
            } else {
                Err("zero_elapsed_years")
            },
        ),
        ("net_profit", Ok(summary.closed_return_sum)),
        ("closed_return_sum", Ok(summary.closed_return_sum)),
        ("expectancy_ratio", expectancy),
        ("avg_hold_days", hold),
        ("monthly_thickness", thickness),
    ] {
        s.put(key, value);
    }
    s
}

/// Validated reports only. The shape guard prevents combining another source
/// window's spans with these returns; it does not replace source admission.
pub(crate) fn build(report: &Report, spans: &[Span]) -> Result<Statistics, StatsError> {
    let at = report.days.first().map_or(0, |d| d.source_index);
    let invalid = || StatsError {
        at,
        code: "pristine_stats_shape",
    };
    let Some(first) = report.days.first() else {
        return Err(invalid());
    };
    let last = report.days.last().unwrap();
    if report.days.len() != report.closed_stats.selected_sessions
        || spans.len() != report.trade_returns.len()
        || spans.len() != report.trade_supplements.len()
        || spans.iter().zip(&report.trade_returns).any(|(s, r)| {
            s.entry_i != r.entry_source_index
                || s.exit_i != r.exit_source_index
                || s.open != r.open
                || s.entry_i < first.source_index
                || s.exit_i > last.source_index
                || s.entry_i > s.exit_i
        })
        || report.days.iter().enumerate().any(|(i, d)| {
            d.source_index != first.source_index + i
                || !crate::baskets::valid_date(&d.date)
                || i > 0 && report.days[i - 1].date >= d.date
                || [d.equity, d.gross_equity, d.benchmark_equity]
                    .iter()
                    .any(|v| !v.is_finite() || *v <= 0.)
                || [
                    d.net_daily_return,
                    d.gross_daily_return,
                    d.benchmark_daily_return,
                ]
                .iter()
                .any(|v| !v.is_finite())
        })
        || report.trade_returns.iter().filter(|r| !r.open).count()
            != report.closed_stats.net.closed_count
        || report.closed_stats.gross.closed_count != report.closed_stats.net.closed_count
    {
        return Err(invalid());
    }
    let years = (crate::day_number(&last.date) - crate::day_number(&first.date)) as f64 / 365.25;
    let net_eq: Vec<_> = report.days.iter().map(|d| d.equity).collect();
    let gross_eq: Vec<_> = report.days.iter().map(|d| d.gross_equity).collect();
    let bench_eq: Vec<_> = report.days.iter().map(|d| d.benchmark_equity).collect();
    let net_daily: Vec<_> = report.days.iter().map(|d| d.net_daily_return).collect();
    let gross_daily: Vec<_> = report.days.iter().map(|d| d.gross_daily_return).collect();
    let bench_daily: Vec<_> = report
        .days
        .iter()
        .map(|d| d.benchmark_daily_return)
        .collect();
    let net_curve = curve_stats(
        &net_eq,
        &net_daily,
        years,
        Ok(report.closed_stats.net_max_drawdown),
    );
    let gross_curve = curve_stats(
        &gross_eq,
        &gross_daily,
        years,
        Ok(report.closed_stats.gross_max_drawdown),
    );
    let bench_curve = curve_stats(&bench_eq, &bench_daily, years, drawdown(&bench_eq));
    let net = common(
        report,
        spans,
        &report.closed_stats.net,
        false,
        net_curve,
        bench_curve,
        years,
    );
    let gross = common(
        report,
        spans,
        &report.closed_stats.gross,
        true,
        gross_curve,
        bench_curve,
        years,
    );
    let mut financial = net.clone();
    for (key, source) in [
        ("gross_total_return", "total_return"),
        ("gross_avg_trade", "avg_trade"),
        ("gross_win_rate", "win_rate"),
        ("gross_sharpe", "sharpe"),
        ("gross_max_dd", "max_dd"),
    ] {
        financial.alias(key, &gross, source);
    }
    financial.alias("avg_net_pnl", &net, "avg_trade");
    financial.put(
        "ledger_gross_return",
        sum(report.trade_returns.iter().map(|r| r.gross_contribution)),
    );
    financial.put(
        "fee_return",
        sum(report.trade_returns.iter().map(|r| r.fee_contribution)),
    );
    financial.put(
        "slippage_return",
        sum(report.trade_returns.iter().map(|r| r.slippage_contribution)),
    );
    let mut benchmark = StatMap::default();
    for (key, value) in [
        ("total_return", bench_curve.total),
        ("cagr", bench_curve.cagr),
        ("sharpe", bench_curve.sharpe),
        ("max_dd", bench_curve.dd),
        ("vol", bench_curve.vol),
        ("sortino", bench_curve.sortino),
        ("mar", bench_curve.mar),
    ] {
        benchmark.put(key, value);
    }
    Ok(Statistics {
        net,
        gross,
        financial,
        benchmark,
    })
}

/// Statistics display only; audited model returns and factors stay full precision.
pub(crate) fn statistic_number(value: f64) -> String {
    if !value.is_finite() {
        return "null".into();
    }
    let scaled = value * 1e6;
    if scaled.is_finite() {
        (scaled.round() / 1e6).to_string()
    } else {
        value.to_string()
    }
}
