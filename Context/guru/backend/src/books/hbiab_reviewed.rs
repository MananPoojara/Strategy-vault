//! Three primary-source examples with an explicit normalized fractional cash account.
//! Source/defaults and declared conventions: docs/hbiab-three-review.md.
use crate::connors::{Ctx, Leg, Params, Trade};
use std::fmt::Write;
#[path = "hbiab_decimal.rs"]
mod decimal;
pub const IDS: &[&str] = &["hbiab_ma5_20", "hbiab_ma7_4", "hbiab_macd_example"];
pub const MODEL: &str = "bandy_fixed_fraction_fractional_cash_v1";
pub const CONVENTION: &str = "bandy_sma_seed_fractional_cash_v1";
pub const BASIS:&str="Bandy Introduction to AmiBroker (2008), printed78/85/93-97/224-225/287/308/317/502-503. Exact ordered 5/20 and 7/4 SMA crosses; MACD 12/26/9 with declared SMA seeds. MA5/20 idealized same-close fills follow printed 503; MA 7/4 same-close fills and 100% fractional cash allocation are declared conventions. MACD uses source next-actual-open fills and 25% of current equity. MA entry fees reserve cash, reducing actual deployed fraction. Normalized account starts at 1, flat selected start, retains full supplied history, one long position, no pyramiding/stops/ranking or portfolio reproduction. Actual selected stocks and ordinary equity ETFs are separate research applications; no cash-index substitute. Open marks pay only actual entry costs; hypothetical liquidation costs are separate. Gross equity removes actual paid costs from identical accepted quantities. Source figures/capital, historical universes and native AmiBroker binary parity are not reproduced.";
pub fn reviewed(id: &str) -> bool {
    IDS.contains(&id)
}
pub fn validate(id: &str, p: &Params) -> Option<String> {
    let defaults: &[(&str, f64)] = match id {
        "hbiab_ma5_20" => &[("fast", 5.), ("slow", 20.)],
        "hbiab_ma7_4" => &[("fast", 7.), ("slow", 4.)],
        "hbiab_macd_example" => &[("fast", 12.), ("slow", 26.), ("sig", 9.)],
        _ => return Some("Outside the three reviewed Bandy examples".into()),
    };
    for (k, v) in p {
        if !defaults
            .iter()
            .any(|(key, value)| k == key && v.is_finite() && v == value)
        {
            return Some(format!("Only exact reviewed Bandy defaults are supported; parameter {k} is changed, unknown or invalid. Alternate grids do not inherit review."));
        }
    }
    None
}
fn sma(v: &[f64], n: usize) -> Vec<f64> {
    (0..v.len())
        .map(|i| {
            if i + 1 < n {
                f64::NAN
            } else {
                let w = &v[i + 1 - n..=i];
                let anchor = w[0];
                anchor + w.iter().map(|value| value - anchor).sum::<f64>() / n as f64
            }
        })
        .collect()
}
fn ema(v: &[f64], n: usize) -> Vec<f64> {
    let mut out = vec![f64::NAN; v.len()];
    let (mut count, mut sum, mut last, mut anchor) = (0, 0., f64::NAN, 0.);
    let alpha = 2. / (n as f64 + 1.);
    for (i, &value) in v.iter().enumerate() {
        if !value.is_finite() {
            continue;
        }
        count += 1;
        if count == 1 {
            anchor = value;
        }
        if count < n {
            sum += value - anchor;
        } else if count == n {
            last = anchor + (sum + (value - anchor)) / n as f64;
        } else {
            last += alpha * (value - last);
        }
        out[i] = last;
    }
    out
}
#[derive(Debug)]
pub struct States {
    pub first: Vec<f64>,
    pub second: Vec<f64>,
    pub buy: Vec<bool>,
    pub sell: Vec<bool>,
}
pub fn states(id: &str, x: &Ctx) -> Result<States, String> {
    states_for_prices(id, &x.c)
}
fn states_for_prices(id: &str, c: &[f64]) -> Result<States, String> {
    let (a, b) = match id {
        "hbiab_ma5_20" => (sma(c, 5), sma(c, 20)),
        "hbiab_ma7_4" => (sma(c, 7), sma(c, 4)),
        "hbiab_macd_example" => {
            let fast = ema(c, 12);
            let slow = ema(c, 26);
            let line: Vec<f64> = fast.iter().zip(slow).map(|(f, s)| f - s).collect();
            let signal = ema(&line, 9);
            (line, signal)
        }
        _ => return Err("Outside reviewed Bandy scope".into()),
    };
    let cross = |a: &[f64], b: &[f64]| {
        (0..a.len())
            .map(|i| {
                i > 0
                    && [a[i], b[i], a[i - 1], b[i - 1]]
                        .iter()
                        .all(|n| n.is_finite())
                    && a[i] > b[i]
                    && a[i - 1] <= b[i - 1]
            })
            .collect()
    };
    let (buy, sell) = if id != "hbiab_macd_example" {
        let (first, second) = if id == "hbiab_ma5_20" {
            (5, 20)
        } else {
            (7, 4)
        };
        let prices: Vec<_> = c.iter().map(|&v| decimal::Decimal::new(v)).collect();
        let order: Vec<_> = (0..prices.len())
            .map(|i| {
                if i + 1 < first.max(second) {
                    None
                } else {
                    Some(decimal::mean_order(&prices, i, first, second))
                }
            })
            .collect();
        let up = (0..prices.len())
            .map(|i| {
                i > 0
                    && order[i] == Some(std::cmp::Ordering::Greater)
                    && matches!(
                        order[i - 1],
                        Some(std::cmp::Ordering::Less | std::cmp::Ordering::Equal)
                    )
            })
            .collect();
        let down = (0..prices.len())
            .map(|i| {
                i > 0
                    && order[i] == Some(std::cmp::Ordering::Less)
                    && matches!(
                        order[i - 1],
                        Some(std::cmp::Ordering::Greater | std::cmp::Ordering::Equal)
                    )
            })
            .collect();
        (up, down)
    } else {
        (cross(&a, &b), cross(&b, &a))
    };
    Ok(States {
        buy,
        sell,
        first: a,
        second: b,
    })
}
#[derive(Clone, Debug)]
pub struct TradeAudit {
    pub entry_equity: f64,
    pub quantity: f64,
    pub notional: f64,
    pub exit_notional: Option<f64>,
    pub entry_cost: f64,
    pub exit_cost: f64,
    pub gross: f64,
    pub net: f64,
    pub entry_signal: usize,
    pub exit_signal: Option<usize>,
    pub liquidation_estimate: f64,
}
#[derive(Clone, Debug)]
pub struct Day {
    pub i: usize,
    pub equity: f64,
    pub gross: f64,
    pub paid: f64,
    pub cash: f64,
    pub quantity: f64,
    pub position_value: f64,
}
#[derive(Clone, Debug)]
pub struct Account {
    pub trades: Vec<Trade>,
    pub trade_audit: Vec<TradeAudit>,
    pub days: Vec<Day>,
    pub terminal_pending: Option<&'static str>,
    pub target_fraction: f64,
    pub next_open: bool,
}
struct Holding {
    entry: usize,
    signal: usize,
    price: f64,
    quantity: f64,
    equity: f64,
    notional: f64,
    fee: f64,
}
fn record(
    out: &mut Account,
    p: Holding,
    i: usize,
    price: f64,
    signal: Option<usize>,
    cost: f64,
    exact_rate: &decimal::Decimal,
) {
    let open = signal.is_none();
    let exit_notional = p.quantity * price;
    let exit_cost = if open { 0. } else { exit_notional * cost };
    let gross = decimal::gross_contribution(p.price, price, p.quantity);
    let net = if open {
        gross - p.fee
    } else {
        decimal::net_contribution(p.price, price, exact_rate, p.quantity)
    };
    out.trades.push(Trade {
        dir: 1.,
        legs: vec![Leg {
            i: p.entry,
            px: p.price,
            w: p.notional / p.equity,
        }],
        exit_i: i,
        exit_px: price,
        reason: if open {
            "open mark"
        } else {
            if out.next_open {
                "MACD / signal crossover exit"
            } else {
                "moving average crossover exit"
            }
        }
        .into(),
        entry_kind: if out.next_open { "open" } else { "close" },
        signal_i: signal.unwrap_or(i),
        open,
        exits: vec![],
    });
    out.trade_audit.push(TradeAudit {
        entry_equity: p.equity,
        quantity: p.quantity,
        notional: p.notional,
        exit_notional: if open { None } else { Some(exit_notional) },
        entry_cost: p.fee,
        exit_cost,
        gross,
        net,
        entry_signal: p.signal,
        exit_signal: signal,
        liquidation_estimate: if open { exit_notional * cost } else { 0. },
    });
}
fn execute(
    x: &Ctx,
    from: usize,
    to: usize,
    buy: &[bool],
    sell: &[bool],
    delay: usize,
    fraction: f64,
    cost: f64,
) -> Result<Account, String> {
    execute_rate(
        x,
        from,
        to,
        buy,
        sell,
        delay,
        fraction,
        cost,
        &decimal::Decimal::new(cost),
    )
}
fn execute_rate(
    x: &Ctx,
    from: usize,
    to: usize,
    buy: &[bool],
    sell: &[bool],
    delay: usize,
    fraction: f64,
    cost: f64,
    exact_rate: &decimal::Decimal,
) -> Result<Account, String> {
    let mut out = Account {
        trades: vec![],
        trade_audit: vec![],
        days: vec![],
        terminal_pending: None,
        target_fraction: fraction,
        next_open: delay == 1,
    };
    let (mut cash, mut paid, mut gross_realized, mut pos) = (1., 0., 0., None::<Holding>);
    for i in from..=to {
        let signal = i.checked_sub(delay).filter(|&s| s >= from);
        let price = if delay == 1 { x.o[i] } else { x.c[i] };
        if let Some(s) = signal {
            if pos.is_some() && sell[s] {
                let p = pos.take().unwrap();
                let proceeds = p.quantity * price;
                let fee = proceeds * cost;
                let entry_equity = p.equity;
                paid += fee;
                record(&mut out, p, i, price, Some(s), cost, exact_rate);
                cash = entry_equity + out.trade_audit.last().unwrap().net;
                gross_realized += out.trade_audit.last().unwrap().gross;
            }
            if pos.is_none() && buy[s] {
                let equity = cash;
                let notional = (equity * fraction).min(cash / (1. + cost));
                let fee = notional * cost;
                cash -= notional + fee;
                if cash.abs() < 1e-14 {
                    cash = 0.;
                }
                paid += fee;
                pos = Some(Holding {
                    entry: i,
                    signal: s,
                    price,
                    quantity: {
                        let q = notional / price;
                        if !q.is_finite() || q <= 0. {
                            return Err(
                                "Unsupported Bandy fractional quantity arithmetic range".into()
                            );
                        }
                        q
                    },
                    equity,
                    notional,
                    fee,
                });
            }
        }
        let quantity = pos.as_ref().map_or(0., |p| p.quantity);
        let value = quantity * x.c[i];
        let equity = pos.as_ref().map_or(cash, |p| {
            p.equity + decimal::gross_contribution(p.price, x.c[i], p.quantity) - p.fee
        });
        if ![cash, paid, quantity, value, equity]
            .iter()
            .all(|v| v.is_finite())
            || cash < -1e-12
            || equity <= 0.
        {
            return Err("Unsupported nonfinite, negative-cash or nonpositive Bandy account; no funding or clipping is invented".into());
        }
        out.days.push(Day {
            i,
            equity,
            gross: 1.
                + gross_realized
                + pos.as_ref().map_or(0., |p| {
                    decimal::gross_contribution(p.price, x.c[i], p.quantity)
                }),
            paid,
            cash,
            quantity,
            position_value: value,
        });
    }
    if delay == 1 {
        out.terminal_pending = if pos.is_some() && sell[to] {
            Some("sell")
        } else if pos.is_none() && buy[to] {
            Some("buy")
        } else {
            None
        };
    }
    if let Some(p) = pos {
        record(&mut out, p, to, x.c[to], None, cost, exact_rate);
    }
    Ok(out)
}
pub fn run(
    id: &str,
    x: &Ctx,
    from: usize,
    to: usize,
    cost: f64,
    p: &Params,
) -> Result<Account, String> {
    if !cost.is_finite() || cost < 0. {
        return Err("Invalid Bandy side cost".into());
    }
    run_rate(id, x, from, to, cost, &decimal::Decimal::new(cost), p)
}
pub fn run_with_bps(
    id: &str,
    x: &Ctx,
    from: usize,
    to: usize,
    fees_bps: f64,
    slippage_bps: f64,
    p: &Params,
) -> Result<Account, String> {
    if ![fees_bps, slippage_bps]
        .iter()
        .all(|v| v.is_finite() && *v >= 0.)
    {
        return Err("Invalid Bandy fee/slippage input".into());
    }
    run_rate(
        id,
        x,
        from,
        to,
        (fees_bps + slippage_bps) / 10000.,
        &decimal::rate(fees_bps, slippage_bps),
        p,
    )
}
fn run_rate(
    id: &str,
    x: &Ctx,
    from: usize,
    to: usize,
    cost: f64,
    exact_rate: &decimal::Decimal,
    p: &Params,
) -> Result<Account, String> {
    if let Some(note) = validate(id, p) {
        return Err(note);
    }
    if !cost.is_finite() || !(0. ..=0.5).contains(&cost) {
        return Err("Invalid Bandy side cost".into());
    }
    if from > to || to >= x.r.len() {
        return Err("Actual Bandy full-period warmup/selected window unavailable".into());
    }
    if x.r[..=to].iter().any(|r| {
        ![r.open, r.high, r.low, r.close]
            .iter()
            .all(|v| v.is_finite() && *v > 0.)
            || r.low > r.high
            || r.open < r.low
            || r.open > r.high
            || r.close < r.low
            || r.close > r.high
    }) || x.r[..=to]
        .windows(2)
        .any(|w| w[0].date >= w[1].date || w[0].ticker != w[1].ticker)
    {
        return Err("Full actual Bandy history contains invalid OHLC, chronology or mixed instruments; no filling/reseeding".into());
    }
    let s = states_for_prices(id, &x.c[..=to])?;
    execute_rate(
        x,
        from,
        to,
        &s.buy,
        &s.sell,
        usize::from(id == "hbiab_macd_example"),
        if id == "hbiab_macd_example" { 0.25 } else { 1. },
        cost,
        exact_rate,
    )
}
fn n(v: f64) -> String {
    if v.is_finite() {
        v.to_string()
    } else {
        "null".into()
    }
}
impl Account {
    pub fn json(&self, x: &Ctx) -> String {
        let mut out=format!("{{\"model\":\"{MODEL}\",\"book_performance_reproduced\":false,\"basis\":\"{}\",\"account_contribution_basis\":\"Dimensionless units of initial account1; trade fractions divide by actual net entry equity. Open marks charge entry only; estimated liquidation is separate.\",\"rejected_entries\":0,\"sizing\":{{\"target_equity_fraction\":{},\"origin\":\"{}\",\"cash_reserved\":true,\"fractional_positions\":true}},\"terminal_entry_pending\":{},\"terminal_exit_pending\":{},\"pending_order\":",crate::esc(BASIS),n(self.target_fraction),if self.target_fraction==0.25{"source_rule"}else{"declared_research_convention"},self.terminal_pending==Some("buy"),self.terminal_pending==Some("sell"));
        if let Some(side) = self.terminal_pending {
            let _=write!(out,"{{\"side\":\"{side}\",\"signal_date\":\"{}\",\"fill_phase\":\"open\",\"filled\":false}}",x.r[self.days.last().unwrap().i].date);
        } else {
            out.push_str("null");
        }
        out.push_str(",\"days\":[");
        for (i, d) in self.days.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            let _=write!(out,"{{\"date\":\"{}\",\"source_index\":{},\"equity\":{},\"gross_same_quantity_equity\":{},\"costs_paid\":{},\"free_cash\":{},\"quantity\":{},\"position_value\":{},\"direction\":{}}}",x.r[d.i].date,d.i,n(d.equity),n(d.gross),n(d.paid),n(d.cash),n(d.quantity),n(d.position_value),if d.quantity>0.{1}else{0});
        }
        out.push_str("],\"trade_accounts\":[");
        for (i, (t, a)) in self.trades.iter().zip(&self.trade_audit).enumerate() {
            if i > 0 {
                out.push(',');
            }
            let _=write!(out,"{{\"entry\":\"{}\",\"exit\":\"{}\",\"open\":{},\"entry_signal_date\":\"{}\",\"exit_signal_date\":{},\"entry_phase\":\"{}\",\"exit_phase\":\"{}\",\"raw_instrument_gross_return\":{},\"entry_equity_units\":{},\"entry_allocation_fraction\":{},\"quantity\":{},\"entry_notional_units\":{},\"exit_notional_units\":{},\"gross_account_contribution\":{},\"net_account_contribution\":{},\"entry_cost_account_contribution\":{},\"exit_cost_account_contribution\":{},\"gross_fraction_of_actual_entry_equity\":{},\"net_fraction_of_actual_entry_equity\":{},\"open_liquidation_cost_estimate\":{}}}",x.r[t.legs[0].i].date,x.r[t.exit_i].date,t.open,x.r[a.entry_signal].date,a.exit_signal.map_or("null".into(),|i|format!("\"{}\"",x.r[i].date)),t.entry_kind,if t.open{"mark"}else{t.entry_kind},n(t.exit_px/t.legs[0].px-1.),n(a.entry_equity),n(a.notional/a.entry_equity),n(a.quantity),n(a.notional),a.exit_notional.map_or("null".into(),n),n(a.gross),n(a.net),n(a.entry_cost),n(a.exit_cost),n(a.gross/a.entry_equity),n(a.net/a.entry_equity),n(a.liquidation_estimate));
        }
        out.push_str("]}");
        out
    }
    pub fn ledger_json(&self, i: usize, x: &Ctx, fees_bps: f64, slippage_bps: f64) -> String {
        let t = &self.trades[i];
        let a = &self.trade_audit[i];
        let turn = a.notional + a.exit_notional.unwrap_or(0.);
        let fee = turn * fees_bps / 10000. / a.entry_equity;
        let slip = turn * slippage_bps / 10000. / a.entry_equity;
        format!("{{\"symbol\":\"{}\",\"signal_side\":\"Long\",\"pts\":{},\"days_held\":{},\"gross_ret\":{},\"net_ret\":{},\"fee_ret\":{},\"slippage_ret\":{},\"gross_pnl\":null,\"fees\":null,\"slippage\":null,\"taxes\":null,\"net_pnl\":null,\"mae\":null,\"mfe\":null,\"mae_ret_observed\":null,\"mfe_ret_observed\":null,\"money_status\":\"Normalized fractional units; monetary quantities and taxes unavailable\",\"return_basis\":\"Actual Bandy cash-account contribution divided by actual net entry equity\",\"pts_basis\":\"Exit or mark price minus entry price; not money\",\"cost_status\":\"{}\",\"excursion_bars\":0,\"excursion_status\":\"unavailable\",\"excursion_basis\":\"Daily execution-bar extrema do not establish exact excursions\"}}",crate::esc(&x.r[t.legs[0].i].ticker),n(t.exit_px-t.legs[0].px),crate::day_number(&x.r[t.exit_i].date)-crate::day_number(&x.r[t.legs[0].i].date),n(a.gross/a.entry_equity),n(a.net/a.entry_equity),n(fee),n(slip),if t.open{"actual_entry_cost_only_open_mark"}else{"modeled_filled_side_costs"})
    }
}
#[cfg(test)]
#[path = "hbiab_reviewed_tests.rs"]
mod tests;
