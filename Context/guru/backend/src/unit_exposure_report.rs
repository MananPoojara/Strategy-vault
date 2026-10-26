//! Explicit reviewed unit-exposure reporting. No funded cash/share accounting.
use crate::{
    accounting::Costs,
    connors::{Ctx, Trade},
    unit_exposure_arithmetic as exact,
};
use std::{collections::BTreeMap, fmt::Write};
pub const MODEL: &str = "canonical_decimal_unit_exposure_v2";
pub const BOOK: &str = "hbqts";
pub const ID: &str = "hbqts_dema16_8";
pub const CONVENTION: &str = "qts_dema16_8_first_close_index_unit_exposure_v2";
pub const BASIS:&str="QTS Figure A.7 printed322-323 source signals and printed324 SPX cash-index illustration; NIFTY 50 is a separate declared index adaptation. Daily finalized same-close fills, flat selected start, first-close DEMA seeds and one long normalized unit exposure are declared conventions. Exact rational returns use pinned Rust Display decimal interpretations of loaded prices and separate fee/slippage components, rounded once to binary64 nearest-even including subnormals. Net factors are independently rounded from exact1+return. Gross separately compounds the same fills at zero cost; ledger contributions use net pretrade allocation. Terminal-open second-side costs are liquidation estimates, not filled exits. No funded capital/share quantities, book performance or native AmiBroker parity. Exposure/drawdown duration use observed sessions; Days Held uses calendar differences; exact-zero closed returns are non-wins.";
#[derive(Clone, Debug)]
pub struct Day {
    pub source_index: usize,
    pub date: String,
    pub equity: f64,
    pub gross_equity: f64,
    pub benchmark_equity: f64,
    pub net_daily_return: f64,
    pub gross_daily_return: f64,
    pub benchmark_daily_return: f64,
}
#[derive(Clone, Debug)]
pub struct TradeReturn {
    pub trade_index: usize,
    pub entry_source_index: usize,
    pub exit_source_index: usize,
    pub open: bool,
    pub entry_allocation: f64,
    pub gross_return: f64,
    pub net_return: f64,
    pub fee_return: f64,
    pub slippage_return: f64,
    pub gross_contribution: f64,
    pub net_contribution: f64,
    pub fee_contribution: f64,
    pub slippage_contribution: f64,
}
#[derive(Clone, Copy, Debug)]
struct Drawdown {
    value: f64,
    days: usize,
}
#[derive(Clone, Debug)]
pub struct Report {
    pub days: Vec<Day>,
    pub trade_returns: Vec<TradeReturn>,
    net_dd: Drawdown,
    gross_dd: Drawdown,
    benchmark_dd: Drawdown,
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum View {
    Net,
    Gross,
    Benchmark,
}
fn arithmetic<T>(value: Result<T, exact::Error>) -> Result<T, String> {
    value.map_err(|e| {
        format!(
            "{}: QTS canonical unit reporting arithmetic; no substituted price or return",
            e.code()
        )
    })
}
fn positive(value: f64) -> Result<f64, String> {
    if value.is_finite() && value > 0. {
        Ok(value)
    } else {
        Err("unit_exposure_equity_domain: unsupported nonpositive/nonfinite normalized equity; no reset or clipping".into())
    }
}
fn returns(entry: f64, mark: f64, costs: Costs, sides: u8) -> Result<exact::ModelScalars, String> {
    arithmetic(exact::evaluate(
        entry,
        mark,
        costs.fees_bps,
        costs.slippage_bps,
        sides,
    ))
}
fn product(a: f64, b: f64) -> Result<f64, String> {
    arithmetic(exact::checked_product(a, b))
}
fn relative(previous: f64, current: f64) -> Result<f64, String> {
    arithmetic(arithmetic(exact::evaluate(previous, current, 0., 0., 0))?.gross)
}
fn daily(values: &[f64]) -> Result<Vec<f64>, String> {
    let mut previous = 1.;
    values
        .iter()
        .map(|&current| {
            let r = relative(previous, current)?;
            previous = current;
            Ok(r)
        })
        .collect()
}
fn drawdown(values: &[f64]) -> Result<Drawdown, String> {
    let (mut peak, mut peak_position, mut worst, mut longest) = (1., 0, 0.0f64, 0);
    for (i, &v) in values.iter().enumerate() {
        if v >= peak {
            peak = v;
            peak_position = i + 1;
        }
        worst = worst.min(relative(peak, v)?);
        longest = longest.max(i + 1 - peak_position);
    }
    Ok(Drawdown {
        value: worst,
        days: longest,
    })
}
pub fn build(
    ctx: &Ctx,
    trades: &[Trade],
    from: usize,
    to: usize,
    costs: Costs,
) -> Result<Report, String> {
    if from > to || to >= ctx.r.len() {
        return Err("unit_exposure_report_shape: unavailable selected source window".into());
    }
    if !costs.fees_bps.is_finite()
        || costs.fees_bps < 0.
        || !costs.slippage_bps.is_finite()
        || costs.slippage_bps < 0.
    {
        return Err("invalid_domain: nonfinite or negative canonical cost component".into());
    }
    if !costs.rollover_bps.is_finite() || costs.rollover_bps != 0. {
        return Err(
            "needs_data: nonzero or invalid rollover has no cash-index contract-event mapping"
                .into(),
        );
    }
    costs.ensure_rollover()?;
    let n = to - from + 1;
    let (mut net, mut gross) = (vec![1.; n], vec![1.; n]);
    let (mut allocation, mut gross_allocation, mut next) = (1., 1., from);
    let mut audits = Vec::new();
    for (i, t) in trades.iter().enumerate() {
        if t.dir != 1.
            || t.legs.len() != 1
            || t.legs[0].w != 1.
            || !t.exits.is_empty()
            || t.entry_kind != "close"
        {
            return Err("unit_exposure_report_shape: only one actual long weight1 close-entry leg without scale-outs is reviewed".into());
        }
        let entry = t.legs[0].i;
        let entry_px = t.legs[0].px;
        if entry >= ctx.c.len()
            || t.exit_i >= ctx.c.len()
            || entry_px != ctx.c[entry]
            || t.exit_px != ctx.c[t.exit_i]
        {
            return Err("unit_exposure_report_shape: source close fill or terminal mark disagrees with the observed close".into());
        }

        if entry < next
            || entry < from
            || entry > t.exit_i
            || t.exit_i > to
            || t.signal_i != t.exit_i
            || t.open && t.exit_i != to
        {
            return Err(
                "unit_exposure_report_shape: source event/terminal coordinates disagree".into(),
            );
        }
        for k in next..entry {
            net[k - from] = allocation;
            gross[k - from] = gross_allocation;
        }
        let final_values = returns(entry_px, t.exit_px, costs, 2)?;
        let gross_return = arithmetic(final_values.gross)?;
        let net_return = arithmetic(final_values.net)?;
        let fee_return = arithmetic(final_values.fee)?;
        let slippage_return = arithmetic(final_values.slippage)?;
        // The exact factor is independently rounded; never form1+rounded return.
        let net_factor = arithmetic(final_values.net_factor)?;
        let gross_factor = arithmetic(final_values.gross_factor)?;
        audits.push(TradeReturn {
            trade_index: i,
            entry_source_index: entry,
            exit_source_index: t.exit_i,
            open: t.open,
            entry_allocation: allocation,
            gross_return,
            net_return,
            fee_return,
            slippage_return,
            gross_contribution: product(allocation, gross_return)?,
            net_contribution: product(allocation, net_return)?,
            fee_contribution: product(allocation, fee_return)?,
            slippage_contribution: product(allocation, slippage_return)?,
        });
        for k in entry..=t.exit_i {
            let (nf, gf) = if k == t.exit_i {
                (net_factor, gross_factor)
            } else {
                let mark = returns(entry_px, ctx.c[k], costs, 1)?;
                (arithmetic(mark.net_factor)?, arithmetic(mark.gross_factor)?)
            };
            net[k - from] = positive(product(allocation, nf)?)?;
            gross[k - from] = positive(product(gross_allocation, gf)?)?;
        }
        allocation = net[t.exit_i - from];
        gross_allocation = gross[t.exit_i - from];
        next = t.exit_i + 1;
    }
    for k in next..=to {
        net[k - from] = allocation;
        gross[k - from] = gross_allocation;
    }
    let benchmark = (from..=to)
        .map(|i| {
            positive(arithmetic(
                returns(ctx.c[from], ctx.c[i], Costs::default(), 0)?.gross_factor,
            )?)
        })
        .collect::<Result<Vec<_>, String>>()?;
    let nd = daily(&net)?;
    let gd = daily(&gross)?;
    let bd = daily(&benchmark)?;
    let net_dd = drawdown(&net)?;
    let gross_dd = drawdown(&gross)?;
    let benchmark_dd = drawdown(&benchmark)?;
    let days = (from..=to)
        .map(|i| {
            let j = i - from;
            Day {
                source_index: i,
                date: ctx.r[i].date.clone(),
                equity: net[j],
                gross_equity: gross[j],
                benchmark_equity: benchmark[j],
                net_daily_return: nd[j],
                gross_daily_return: gd[j],
                benchmark_daily_return: bd[j],
            }
        })
        .collect();
    Ok(Report {
        days,
        trade_returns: audits,
        net_dd,
        gross_dd,
        benchmark_dd,
    })
}
impl Report {
    pub fn common_stats(
        &self,
        ctx: &Ctx,
        trades: &[Trade],
        from: usize,
        to: usize,
        view: View,
    ) -> BTreeMap<&'static str, f64> {
        let mut s = BTreeMap::new();
        let eq = self
            .days
            .iter()
            .map(|d| match view {
                View::Net => d.equity,
                View::Gross => d.gross_equity,
                View::Benchmark => d.benchmark_equity,
            })
            .collect::<Vec<_>>();
        let rets = self
            .days
            .iter()
            .map(|d| match view {
                View::Net => d.net_daily_return,
                View::Gross => d.gross_daily_return,
                View::Benchmark => d.benchmark_daily_return,
            })
            .collect::<Vec<_>>();
        let dd = match view {
            View::Net => self.net_dd,
            View::Gross => self.gross_dd,
            View::Benchmark => self.benchmark_dd,
        };
        let yrs = crate::years(ctx, from, to);
        let nr = rets.len().max(1) as f64;
        let mean = rets.iter().sum::<f64>() / nr;
        let sd = (rets.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / (nr - 1.).max(1.)).sqrt();
        let dsd = (rets.iter().map(|r| r.min(0.).powi(2)).sum::<f64>() / nr).sqrt();
        let total = eq.last().unwrap() - 1.;
        let cagr = eq.last().unwrap().powf(1. / yrs) - 1.;
        let bh_last = self.days.last().unwrap().benchmark_equity;
        let selected_trades = if view == View::Benchmark {
            &[] as &[Trade]
        } else {
            trades
        };
        let mut held = vec![false; eq.len()];
        for t in selected_trades {
            held[t.legs[0].i - from..=t.exit_i - from]
                .iter_mut()
                .for_each(|h| *h = true);
        }
        let closed = selected_trades
            .iter()
            .filter(|t| !t.open)
            .collect::<Vec<_>>();
        let tr = if view == View::Benchmark {
            vec![]
        } else {
            self.trade_returns
                .iter()
                .filter(|a| !a.open)
                .map(|a| {
                    if view == View::Gross {
                        a.gross_return
                    } else {
                        a.net_return
                    }
                })
                .collect::<Vec<_>>()
        };
        let n = tr.len() as f64;
        let wins = tr.iter().copied().filter(|r| *r > 0.).collect::<Vec<_>>();
        let losses = tr.iter().copied().filter(|r| *r <= 0.).collect::<Vec<_>>();
        let avg = tr.iter().sum::<f64>() / n;
        let tsd = (tr.iter().map(|r| (r - avg).powi(2)).sum::<f64>() / (n - 1.)).sqrt();
        let gross_win: f64 = wins.iter().sum();
        let gross_loss = -losses.iter().sum::<f64>();
        let mut sorted = tr.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let (mut streak, mut max_streak) = (0, 0);
        for r in &tr {
            streak = if *r <= 0. { streak + 1 } else { 0 };
            max_streak = max_streak.max(streak);
        }
        let avg_win = wins.iter().sum::<f64>() / wins.len() as f64;
        let avg_loss = losses.iter().sum::<f64>() / losses.len() as f64;
        s.insert("total_return", total);
        s.insert("cagr", cagr);
        s.insert("vol", sd * 252f64.sqrt());
        s.insert("sharpe", mean / sd * 252f64.sqrt());
        s.insert("sortino", mean / dsd * 252f64.sqrt());
        s.insert("max_dd", dd.value);
        s.insert("max_dd_days", dd.days as f64);
        s.insert("mar", cagr / -dd.value);
        s.insert(
            "exposure",
            held.iter().filter(|h| **h).count() as f64 / eq.len() as f64,
        );
        s.insert("bh_total", bh_last - 1.);
        s.insert("bh_cagr", bh_last.powf(1. / yrs) - 1.);
        s.insert("bh_max_dd", self.benchmark_dd.value);
        s.insert("years", yrs);
        s.insert("trades", n);
        s.insert("win_rate", wins.len() as f64 / n);
        s.insert("avg_trade", avg);
        let m = sorted.len();
        s.insert(
            "median_trade",
            match m {
                0 => f64::NAN,
                _ if m % 2 == 1 => sorted[m / 2],
                _ => (sorted[m / 2 - 1] + sorted[m / 2]) / 2.,
            },
        );
        s.insert("avg_win", avg_win);
        s.insert("avg_loss", avg_loss);
        s.insert("payoff", avg_win / -avg_loss);
        s.insert("profit_factor", gross_win / gross_loss);
        s.insert("best", sorted.last().copied().unwrap_or(f64::NAN));
        s.insert("worst", sorted.first().copied().unwrap_or(f64::NAN));
        s.insert("t_stat", avg / (tsd / n.sqrt()));
        s.insert("max_consec_losses", max_streak as f64);
        s.insert(
            "avg_bars",
            closed
                .iter()
                .map(|t| (t.exit_i - t.legs[0].i) as f64)
                .sum::<f64>()
                / n,
        );
        s.insert(
            "avg_deployed",
            closed.iter().map(|t| t.deployed()).sum::<f64>() / n,
        );
        let open = if view == View::Benchmark {
            f64::NAN
        } else {
            self.trade_returns
                .iter()
                .find(|a| a.open)
                .map_or(f64::NAN, |a| {
                    if view == View::Gross {
                        a.gross_return
                    } else {
                        a.net_return
                    }
                })
        };
        s.insert("open_trade", open);
        s.insert("trades_per_year", n / yrs);
        let hold_days = closed
            .iter()
            .map(|t| {
                ((crate::day_number(&ctx.r[t.exit_i].date)
                    - crate::day_number(&ctx.r[t.legs[0].i].date)) as f64)
                    .max(1.)
            })
            .sum::<f64>()
            / n;
        let (wr, al) = (wins.len() as f64 / n, -avg_loss);
        let exp_ratio = if losses.is_empty() {
            f64::NAN
        } else {
            (wr * if wins.is_empty() { 0. } else { avg_win } - (1. - wr) * al) / al
        };
        s.insert("net_profit", tr.iter().sum());
        s.insert("closed_return_sum", tr.iter().sum());
        s.insert("compounded_return", total);
        s.insert("car_mdd", cagr / -dd.value);
        s.insert("expectancy_ratio", exp_ratio);
        s.insert("avg_hold_days", hold_days);
        s.insert("monthly_thickness", avg / hold_days * 30.);
        s
    }
    pub fn financial_stats(
        &self,
        ctx: &Ctx,
        trades: &[Trade],
        from: usize,
        to: usize,
    ) -> BTreeMap<&'static str, f64> {
        let mut net = self.common_stats(ctx, trades, from, to, View::Net);
        let gross = self.common_stats(ctx, trades, from, to, View::Gross);
        for (key, source) in [
            ("gross_total_return", "total_return"),
            ("gross_avg_trade", "avg_trade"),
            ("gross_win_rate", "win_rate"),
            ("gross_sharpe", "sharpe"),
            ("gross_max_dd", "max_dd"),
        ] {
            net.insert(key, gross[source]);
        }
        net.insert("avg_net_pnl", net["avg_trade"]);
        net.insert(
            "ledger_gross_return",
            self.trade_returns
                .iter()
                .map(|a| a.gross_contribution)
                .sum(),
        );
        net.insert(
            "fee_return",
            self.trade_returns.iter().map(|a| a.fee_contribution).sum(),
        );
        net.insert(
            "slippage_return",
            self.trade_returns
                .iter()
                .map(|a| a.slippage_contribution)
                .sum(),
        );
        net
    }
    pub fn benchmark_stats(
        &self,
        ctx: &Ctx,
        from: usize,
        to: usize,
    ) -> BTreeMap<&'static str, f64> {
        let all = self.common_stats(ctx, &[], from, to, View::Benchmark);
        [
            "total_return",
            "cagr",
            "sharpe",
            "max_dd",
            "vol",
            "sortino",
            "mar",
        ]
        .into_iter()
        .map(|k| (k, all[k]))
        .collect()
    }
}
pub fn number(v: f64) -> String {
    if v.is_finite() {
        v.to_string()
    } else {
        "null".into()
    }
}
pub fn statistic_number(v: f64) -> String {
    if !v.is_finite() {
        return "null".into();
    }
    let scaled = v * 1e6;
    if scaled.is_finite() {
        (scaled.round() / 1e6).to_string()
    } else {
        v.to_string()
    }
}
pub fn stats_json(s: &BTreeMap<&'static str, f64>) -> String {
    format!(
        "{{{}}}",
        s.iter()
            .map(|(k, v)| format!("\"{k}\":{}", statistic_number(*v)))
            .collect::<Vec<_>>()
            .join(",")
    )
}

impl Report {
    pub fn json(&self) -> String {
        let mut out=format!("{{\"model\":\"{MODEL}\",\"book\":\"{BOOK}\",\"id\":\"{ID}\",\"source_convention\":\"{CONVENTION}\",\"source_performance_reproduced\":false,\"native_amibroker_parity\":false,\"basis\":\"{}\",\"arithmetic\":\"pinned Rust Display canonical-decimal rational; binary64 nearest-even including subnormals; nonzero-to-zero underflow and overflow rejected\",\"initial_equity\":1,\"open_exit_cost\":\"estimated_liquidation\",\"days\":[",crate::esc(BASIS));
        for (i, d) in self.days.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            let _=write!(out,"{{\"source_index\":{},\"date\":\"{}\",\"equity\":{},\"gross_equity\":{},\"benchmark_equity\":{},\"net_daily_return\":{},\"gross_daily_return\":{},\"benchmark_daily_return\":{}}}",d.source_index,d.date,number(d.equity),number(d.gross_equity),number(d.benchmark_equity),number(d.net_daily_return),number(d.gross_daily_return),number(d.benchmark_daily_return));
        }
        out.push_str("],\"trade_returns\":[");
        for (i, a) in self.trade_returns.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            let _=write!(out,"{{\"trade_index\":{},\"entry_source_index\":{},\"exit_source_index\":{},\"open\":{},\"entry_allocation\":{},\"gross_return\":{},\"net_return\":{},\"fee_return\":{},\"slippage_return\":{},\"gross_contribution\":{},\"net_contribution\":{},\"fee_contribution\":{},\"slippage_contribution\":{}}}",a.trade_index,a.entry_source_index,a.exit_source_index,a.open,number(a.entry_allocation),number(a.gross_return),number(a.net_return),number(a.fee_return),number(a.slippage_return),number(a.gross_contribution),number(a.net_contribution),number(a.fee_contribution),number(a.slippage_contribution));
        }
        out.push_str("]}");
        out
    }
    pub fn trade_accounting_json(
        &self,
        ctx: &Ctx,
        t: &Trade,
        index: usize,
    ) -> Result<String, String> {
        let a = &self.trade_returns[index];
        // the shared fill-aware path (accounting::path_points), in this report's exact decimal arithmetic
        let path = crate::accounting::path_points(ctx, t);
        let bars = path.iter().map(|p| p.0).collect::<std::collections::BTreeSet<_>>().len();
        let lo = path.iter().map(|p| p.1).reduce(f64::min);
        let hi = path.iter().map(|p| p.1).reduce(f64::max);
        let mae = lo.map(|v| relative(t.legs[0].px, v)).transpose()?;
        let mfe = hi.map(|v| relative(t.legs[0].px, v)).transpose()?;
        let pts = t.exit_px - t.legs[0].px;
        let days =
            crate::day_number(&ctx.r[t.exit_i].date) - crate::day_number(&ctx.r[t.legs[0].i].date);
        Ok(format!("{{\"symbol\":\"{}\",\"signal_side\":\"Long\",\"pts\":{},\"days_held\":{},\"gross_ret\":{},\"net_ret\":{},\"fee_ret\":{},\"slippage_ret\":{},\"gross_pnl\":null,\"fees\":null,\"slippage\":null,\"taxes\":null,\"net_pnl\":null,\"money_status\":\"No position quantities or monetary sizing supplied\",\"return_basis\":\"Canonical-decimal unit exposure; audited return fractions, not contract quantities\",\"pts_basis\":\"Actual close exit or mark minus actual close entry; not monetary profit\",\"cost_status\":\"{}\",\"mae\":null,\"mfe\":null,\"mae_ret_observed\":{},\"mfe_ret_observed\":{},\"excursion_bars\":{},\"excursion_status\":\"{}\",\"excursion_basis\":\"{}\"}}",crate::esc(&ctx.r[t.legs[0].i].ticker),number(pts),days,number(a.gross_return),number(a.net_return),number(a.fee_return),number(a.slippage_return),if t.open{"open_liquidation_estimate"}else{"modeled_round_trip_cost"},mae.map_or("null".into(),number),mfe.map_or("null".into(),number),bars,if bars>0{crate::accounting::EXCURSION_STATUS}else{"unavailable"},crate::accounting::EXCURSION_BASIS))
    }
}
pub fn costs_json(costs: Costs) -> String {
    format!("{{\"fees_bps\":{},\"slippage_bps\":{},\"rollover_bps\":{},\"total_side_bps\":{},\"taxes\":null,\"rolls\":null,\"basis\":\"Separate user fee/slippage components interpreted by pinned Rust Display as exact decimal rates; normalized entry-allocation deductions per side, not funded cash fees or a statutory tax schedule. total_side_bps is their binary64 display sum and is not used to reconstruct exact rates.\",\"rollover_status\":\"No verified contract roll-event mapping\",\"slippage_status\":\"User-entered return deduction; fill prices are not rewritten\",\"money_status\":\"No position quantities or monetary sizing supplied; monetary P&L and taxes remain unavailable\",\"open_trade_cost_basis\":\"Terminal-open normalized return includes an explicitly estimated second-side liquidation cost; no sell is fabricated\"}}",number(costs.fees_bps),number(costs.slippage_bps),number(costs.rollover_bps),number(costs.fees_bps+costs.slippage_bps))
}

#[cfg(test)]
#[path = "unit_exposure_report_tests.rs"]
mod tests;
