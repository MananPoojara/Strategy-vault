//! Protected daily Impulse application. See library/elcim/REVIEW-2026-10-15.md.
//! Normalized fractional cash account, not a broker/borrow or monetary return claim.
use super::elcim::{ema, macd_hist};
use crate::{
    connors::{Ctx, Leg, Params, Trade},
    day_number, num,
};
use std::fmt::Write;

pub const ID: &str = "elcim_impulse";
pub const BASIS:&str="Protected daily Impulse research application on the selected actual stock. Daily EMA13/MACD12-26-9; completed-week EMA26 entry filter; next actual open; prior-known SafeZone10/2 active on entry day; cost-aware2%/6% monthly risk limits. Normalized fractional cash account starts at1, without monetary capital, real share lots, borrowed-stock availability, interest or dividends. Shorts reserve principal and restrict sale proceeds. Supplied price adjustment basis, continuous stop levels and fixed10-session worksheet lookback are declared research conventions; the source p.179 discretionary turning-point resets are not mechanically reproduced. Net profit sums closed-trade net fractions of each actual entry equity; open holdings are excluded. Equity/risk statistics use the actual normalized cash account, including open marks with entry costs only. Gross equity retains the same accepted quantities and fills, removing paid costs only; it never independently resizes or selects trades. Gross trade fractions retain the actual net-entry-equity denominator. Daily-close6% detection liquidates at the next actual open; gaps may exceed planned limits. Source chart examples and illustrative risk figures are not these results.";

pub fn validate(p: &Params) -> Option<String> {
    let defaults = [
        ("ema", 13.),
        ("fast", 12.),
        ("slow", 26.),
        ("sig", 9.),
        ("wema", 26.),
        ("wfilter", 0.),
        ("next_open", 1.),
    ];
    for (key, value) in p {
        match defaults.iter().find(|(k,_)|k==key) {
            Some((_,expected)) if value.is_finite() && value==expected=>{},
            _=>return Some(format!("Protected Elder Impulse covers only exact reviewed defaults; {key} is unknown, invalid or changed. Other settings do not inherit this source/account review.")),
        }
    }
    None
}

pub struct States {
    pub ema13: Vec<f64>,
    pub histogram: Vec<f64>,
    pub weekly: Vec<i8>,
    pub weekly_ready: Vec<bool>,
    pub long_stop: Vec<f64>,
    pub short_stop: Vec<f64>,
    pub green: Vec<bool>,
    pub red: Vec<bool>,
}
/// A week commits at the next calendar-week open, without accessing that bar's
/// prices. This retains actual NSE weekend sessions and never promotes a terminal
/// Wednesday/Friday as a completed week using a future row or end-of-input test.
pub fn states(x: &Ctx) -> States {
    let ema13 = ema(&x.c, 13);
    let histogram = macd_hist(&x.c, 12, 26, 9);
    let n = x.c.len();
    let mut weekly = vec![0; n];
    let mut weekly_ready = vec![false; n];
    let (mut active, mut previous_close) = (None, 0.);
    let (mut count, mut seed, mut prev, mut direction) = (0usize, 0., f64::NAN, 0i8);
    for i in 0..n {
        let day = day_number(&x.r[i].date);
        let key = day - (day + 3).rem_euclid(7);
        if active.is_some_and(|a| a != key) {
            count += 1;
            let next = if count < 26 {
                seed += previous_close;
                f64::NAN
            } else if count == 26 {
                (seed + previous_close) / 26.
            } else {
                (2. / 27.) * previous_close + (1. - 2. / 27.) * prev
            };
            direction = if next > prev {
                1
            } else if next < prev {
                -1
            } else {
                0
            };
            prev = next;
        }
        weekly[i] = direction;
        weekly_ready[i] = count >= 27;
        active = Some(key);
        previous_close = x.c[i];
    }
    let (mut raw_long, mut raw_short) = (vec![f64::NAN; n], vec![f64::NAN; n]);
    for j in 11..n {
        let (mut down, mut up, mut nd, mut nu) = (0., 0., 0usize, 0usize);
        for t in j - 10..j {
            let d = x.l[t - 1] - x.l[t];
            if d > 0. {
                down += d;
                nd += 1;
            }
            let u = x.h[t] - x.h[t - 1];
            if u > 0. {
                up += u;
                nu += 1;
            }
        }
        if nd > 0 {
            raw_long[j] = x.l[j - 1] - 2. * down / nd as f64;
        }
        if nu > 0 {
            raw_short[j] = x.h[j - 1] + 2. * up / nu as f64;
        }
    }
    let (mut long_stop, mut short_stop) = (vec![f64::NAN; n], vec![f64::NAN; n]);
    for j in 2..n {
        if raw_long[j - 2..=j].iter().all(|v| v.is_finite()) {
            long_stop[j] = raw_long[j - 2..=j]
                .iter()
                .copied()
                .fold(f64::NEG_INFINITY, f64::max);
        }
        if raw_short[j - 2..=j].iter().all(|v| v.is_finite()) {
            short_stop[j] = raw_short[j - 2..=j]
                .iter()
                .copied()
                .fold(f64::INFINITY, f64::min);
        }
    }
    let (mut green, mut red) = (vec![false; n], vec![false; n]);
    for j in 1..n {
        green[j] = ema13[j] > ema13[j - 1] && histogram[j] > histogram[j - 1];
        red[j] = ema13[j] < ema13[j - 1] && histogram[j] < histogram[j - 1];
    }
    States {
        ema13,
        histogram,
        weekly,
        weekly_ready,
        long_stop,
        short_stop,
        green,
        red,
    }
}

#[derive(Clone)]
struct Position {
    entry_i: usize,
    dir: f64,
    px: f64,
    q: f64,
    notional: f64,
    entry_equity: f64,
    entry_fee: f64,
    initial_stop: f64,
    stop: f64,
    planned: f64,
    budget: f64,
    reference: f64,
    month_basis: f64,
    month_entry_fee: f64,
}
#[derive(Clone, Debug)]
pub struct TradeAudit {
    pub entry_equity: f64,
    pub quantity: f64,
    pub notional: f64,
    pub initial_stop: f64,
    pub final_stop: f64,
    pub planned_risk: f64,
    pub risk_budget: f64,
    pub month_reference: f64,
    pub entry_fee: f64,
    pub exit_fee: f64,
    pub gross: f64,
    pub net: f64,
}
#[derive(Debug)]
pub struct Day {
    pub i: usize,
    pub equity: f64,
    pub gross: f64,
    pub costs: f64,
    pub cash: f64,
    pub reserve: f64,
    pub proceeds: f64,
    pub quantity: f64,
    pub direction: f64,
    pub stop: f64,
    pub reference: f64,
    pub losses: f64,
    pub outstanding: f64,
    pub halted: bool,
    pub pending: bool,
}
#[derive(Debug)]
pub struct Account {
    pub trades: Vec<Trade>,
    pub trade_audit: Vec<TradeAudit>,
    pub days: Vec<Day>,
    pub rejected: usize,
    pub terminal_pending: bool,
}
impl Account {
    pub fn json(&self, x: &Ctx) -> String {
        let mut s=format!("{{\"model\":\"elder_protected_impulse_fractional_cash_v1\",\"book_performance_reproduced\":false,\"basis\":\"{}\",\"rejected_entries\":{},\"terminal_liquidation_pending\":{},\"days\":[",crate::esc(BASIS),self.rejected,self.terminal_pending);
        for (i, d) in self.days.iter().enumerate() {
            if i > 0 {
                s.push(',');
            }
            let _=write!(s,"{{\"date\":\"{}\",\"equity\":{},\"gross_same_quantity_equity\":{},\"costs_paid\":{},\"free_cash\":{},\"reserved_short_principal\":{},\"restricted_short_proceeds\":{},\"direction\":{},\"stop\":{},\"monthly_reference\":{},\"monthly_realized_losses\":{},\"outstanding_planned_risk\":{},\"monthly_halted\":{},\"liquidation_pending\":{}}}",x.r[d.i].date,num(d.equity),num(d.gross),num(d.costs),num(d.cash),num(d.reserve),num(d.proceeds),d.direction,num(d.stop),num(d.reference),num(d.losses),num(d.outstanding),d.halted,d.pending);
        }
        s.push_str("],\"trade_accounts\":[");
        for (i, (t, a)) in self.trades.iter().zip(&self.trade_audit).enumerate() {
            if i > 0 {
                s.push(',');
            }
            let _=write!(s,"{{\"entry\":\"{}\",\"exit\":\"{}\",\"open\":{},\"raw_instrument_gross_return\":{},\"entry_equity_units\":{},\"entry_allocation_fraction\":{},\"initial_stop\":{},\"final_stop\":{},\"planned_risk_fraction_of_month_reference\":{},\"risk_budget_fraction_of_month_reference\":{},\"gross_account_contribution\":{},\"net_account_contribution\":{},\"entry_cost_account_contribution\":{},\"exit_cost_account_contribution\":{},\"gross_fraction_of_actual_entry_equity\":{},\"net_fraction_of_actual_entry_equity\":{}}}",x.r[t.legs[0].i].date,x.r[t.exit_i].date,t.open,num(t.dir*(t.exit_px/t.legs[0].px-1.)),num(a.entry_equity),num(a.notional/a.entry_equity),num(a.initial_stop),num(a.final_stop),num(a.planned_risk/a.month_reference),num(a.risk_budget/a.month_reference),num(a.gross),num(a.net),num(a.entry_fee),num(a.exit_fee),num(a.gross/a.entry_equity),num(a.net/a.entry_equity));
        }
        s.push_str("],\"account_contribution_basis\":\"Contributions and account equity are dimensionless units of the initial normalized account1, not currency or actual share quantities. Trade fractions divide by actual net account equity immediately before entry. Raw instrument return is separate. Closed-only display modes never change source risk sizing or account execution.\"}");
        s
    }
}
struct Book {
    cash: f64,
    reserve: f64,
    proceeds: f64,
    paid: f64,
    gross_realized: f64,
    losses: f64,
    position: Option<Position>,
    trades: Vec<Trade>,
    audits: Vec<TradeAudit>,
}
impl Book {
    fn equity(&self, mark: f64) -> f64 {
        match &self.position {
            None => self.cash,
            Some(p) if p.dir > 0. => self.cash + p.q * mark,
            Some(p) => self.cash + self.reserve + self.proceeds - p.q * mark,
        }
    }
    fn gross(&self, mark: f64) -> f64 {
        1. + self.gross_realized
            + self
                .position
                .as_ref()
                .map_or(0., |p| p.q * p.dir * (mark - p.px))
    }
    fn finish(&mut self, j: usize, price: f64, reason: &str, rate: f64, open: bool) {
        let p = self.position.take().unwrap();
        let exit_fee = if open { 0. } else { p.notional * rate };
        let gross = p.q * p.dir * (price - p.px);
        if !open {
            self.cash += if p.dir > 0. {
                p.q * price - exit_fee
            } else {
                self.reserve + self.proceeds - p.q * price - exit_fee
            };
            self.losses +=
                (-(p.q * p.dir * (price - p.month_basis) - p.month_entry_fee - exit_fee)).max(0.);
            self.paid += exit_fee;
            self.gross_realized += gross;
            self.reserve = 0.;
            self.proceeds = 0.;
        }
        self.trades.push(Trade {
            dir: p.dir,
            legs: vec![Leg {
                i: p.entry_i,
                px: p.px,
                w: p.notional / p.entry_equity,
            }],
            exit_i: j,
            exit_px: price,
            reason: reason.into(),
            entry_kind: "open",
            signal_i: if open { j } else { j - 1 },
            open,
            exits: vec![],
        });
        self.audits.push(TradeAudit {
            entry_equity: p.entry_equity,
            quantity: p.q,
            notional: p.notional,
            initial_stop: p.initial_stop,
            final_stop: p.stop,
            planned_risk: p.planned,
            risk_budget: p.budget,
            month_reference: p.reference,
            entry_fee: p.entry_fee,
            exit_fee,
            gross,
            net: gross - p.entry_fee - exit_fee,
        });
        if open {
            self.position = Some(p);
        }
    }
}
pub fn risk_budget(reference: f64, losses: f64, other_risk: f64) -> f64 {
    (0.02 * reference)
        .min(0.06 * reference - losses - other_risk)
        .max(0.)
}
pub fn run(x: &Ctx, from: usize, to: usize, rate: f64, p: &Params) -> Result<Account, String> {
    if let Some(note) = validate(p) {
        return Err(note);
    }
    if from >= to || to >= x.c.len() || !rate.is_finite() || !(0.0..=0.5).contains(&rate) {
        return Err("Invalid protected Impulse range or execution rate".into());
    }
    if x.r[..=to].iter().any(|r| {
        ![r.open, r.high, r.low, r.close]
            .iter()
            .all(|v| v.is_finite() && *v > 0.)
            || r.low > r.open.min(r.close)
            || r.high < r.open.max(r.close)
            || r.low > r.high
    }) {
        return Err("needs_data: Protected Impulse requires valid actual OHLC throughout recursive warmup and the selected period".into());
    }
    let st = states(x);
    if !st.weekly_ready[from] || !st.ema13[from].is_finite() || !st.histogram[from].is_finite() {
        return Err("needs_data: Actual daily and completed-week26EMA warmup is unavailable at the selected start".into());
    }
    execute(x, from, to, rate, &st)
}
fn execute(x: &Ctx, from: usize, to: usize, rate: f64, st: &States) -> Result<Account, String> {
    let mut b = Book {
        cash: 1.,
        reserve: 0.,
        proceeds: 0.,
        paid: 0.,
        gross_realized: 0.,
        losses: 0.,
        position: None,
        trades: vec![],
        audits: vec![],
    };
    let (mut reference, mut month, mut halted, mut pending, mut rejected) =
        (1., x.r[from].date[..7].to_owned(), false, false, 0usize);
    let mut days = Vec::with_capacity(to - from + 1);
    for j in from..=to {
        let at_prior_close = b.position.is_some();
        if x.r[j].date[..7] != month {
            reference = b.equity(x.c[j - 1]);
            if reference <= 0. {
                return Err("calculation_unavailable: Nonpositive source account at month boundary; no reset or added funding".into());
            }
            b.losses = 0.;
            halted = false;
            month = x.r[j].date[..7].to_owned();
            if let Some(p) = &mut b.position {
                p.month_basis = x.c[j - 1];
                p.month_entry_fee = 0.;
                let allowed = 0.02 * reference - p.notional * rate;
                if allowed <= 0. {
                    pending = true;
                } else {
                    let bound = x.c[j - 1] - p.dir * allowed / p.q;
                    p.stop = if p.dir > 0. {
                        p.stop.max(bound)
                    } else {
                        p.stop.min(bound)
                    };
                }
            }
        }
        let daily_exit = b.position.as_ref().is_some_and(|p| {
            j > from
                && !(if p.dir > 0. {
                    st.green[j - 1]
                } else {
                    st.red[j - 1]
                })
        });
        if b.position.is_some() && (pending || daily_exit) {
            b.finish(
                j,
                x.o[j],
                if pending {
                    "source_risk_liquidation_next_open"
                } else {
                    "daily_impulse_disappeared_next_open"
                },
                rate,
                false,
            );
        }
        pending = false;
        if let Some(p) = &mut b.position {
            let level = if p.dir > 0. {
                st.long_stop[j]
            } else {
                st.short_stop[j]
            };
            if level.is_finite() {
                p.stop = if p.dir > 0. {
                    p.stop.max(level)
                } else {
                    p.stop.min(level)
                };
            }
        }
        if b.position.is_none() && !at_prior_close && !halted && j > from {
            let dir = if st.green[j - 1] && st.weekly[j] == 1 {
                1.
            } else if st.red[j - 1] && st.weekly[j] == -1 {
                -1.
            } else {
                0.
            };
            if dir != 0. {
                let stop = if dir > 0. {
                    st.long_stop[j]
                } else {
                    st.short_stop[j]
                };
                let budget = risk_budget(reference, b.losses, 0.);
                if !stop.is_finite() || stop <= 0. || dir * (x.o[j] - stop) <= 0. || budget <= 0. {
                    rejected += 1;
                } else {
                    let entry_equity = b.cash;
                    let distance = (x.o[j] - stop).abs() / x.o[j];
                    let notional =
                        (entry_equity / (1. + rate)).min(budget / (distance + 2. * rate));
                    let q = notional / x.o[j];
                    let entry_fee = notional * rate;
                    let planned = q * (x.o[j] - stop).abs() + 2. * entry_fee;
                    if !q.is_finite()
                        || q <= 0.
                        || planned > budget + 1e-10
                        || notional * (1. + rate) > entry_equity + 1e-10
                    {
                        return Err("calculation_unavailable: Source planned risk or cash cap failed; no trade weight is clipped".into());
                    }
                    b.cash -= notional + entry_fee;
                    b.reserve = if dir < 0. { notional } else { 0. };
                    b.proceeds = b.reserve;
                    b.paid += entry_fee;
                    b.position = Some(Position {
                        entry_i: j,
                        dir,
                        px: x.o[j],
                        q,
                        notional,
                        entry_equity,
                        entry_fee,
                        initial_stop: stop,
                        stop,
                        planned,
                        budget,
                        reference,
                        month_basis: x.o[j],
                        month_entry_fee: entry_fee,
                    });
                }
            }
        }
        if let Some(p) = &b.position {
            if (p.dir > 0. && x.l[j] <= p.stop) || (p.dir < 0. && x.h[j] >= p.stop) {
                let gap = if p.dir > 0. {
                    x.o[j] <= p.stop
                } else {
                    x.o[j] >= p.stop
                };
                let fill = if p.dir > 0. {
                    x.o[j].min(p.stop)
                } else {
                    x.o[j].max(p.stop)
                };
                b.finish(
                    j,
                    fill,
                    if gap {
                        "protective_stop_gap_open"
                    } else {
                        "protective_stop_known_before_open"
                    },
                    rate,
                    false,
                );
            }
        }
        let value = b.equity(x.c[j]);
        let gross = b.gross(x.c[j]);
        if !value.is_finite()
            || value <= 0.
            || b.cash < -1e-10
            || (gross - value - b.paid).abs() > 1e-9 * gross.abs().max(1.)
        {
            return Err("calculation_unavailable: Protected Impulse cash/equity/cost conservation failed; no reset, clipping or fabricated funding".into());
        }
        if value < 0.94 * reference {
            halted = true;
            pending = b.position.is_some();
        }
        if b.losses >= 0.06 * reference {
            halted = true;
        }
        let p = b.position.as_ref();
        let outstanding = p.map_or(0., |p| {
            (p.q * p.dir * (p.month_basis - p.stop) + p.month_entry_fee + p.notional * rate).max(0.)
        });
        days.push(Day {
            i: j,
            equity: value,
            gross,
            costs: b.paid,
            cash: b.cash,
            reserve: b.reserve,
            proceeds: b.proceeds,
            quantity: p.map_or(0., |p| p.q),
            direction: p.map_or(0., |p| p.dir),
            stop: p.map_or(f64::NAN, |p| p.stop),
            reference,
            losses: b.losses,
            outstanding,
            halted,
            pending,
        });
    }
    if b.position.is_some() {
        b.finish(to, x.c[to], "actual_open_holding_mark", rate, true);
    }
    Ok(Account {
        trades: b.trades,
        trade_audit: b.audits,
        days,
        rejected,
        terminal_pending: pending,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn rows(dates: &[&str]) -> Vec<Row> {
        dates
            .iter()
            .map(|d| Row {
                ticker: "TEST_STOCK".into(),
                date: d.to_string(),
                open: 100.,
                high: 101.,
                low: 99.,
                close: 100.,
                volume: Some(1000.),
            })
            .collect()
    }
    fn fixture(n: usize, dir: i8) -> States {
        States {
            ema13: vec![100.; n],
            histogram: vec![1.; n],
            weekly: vec![dir; n],
            weekly_ready: vec![true; n],
            long_stop: vec![95.; n],
            short_stop: vec![105.; n],
            green: vec![dir > 0; n],
            red: vec![dir < 0; n],
        }
    }
    #[test]
    fn source_monthly_loss_budget_never_replenishes_with_wins() {
        assert!((risk_budget(150000., 5000., 3600.) - 400.).abs() < 1e-9);
        assert_eq!(risk_budget(90000., 2400., 0.), 1800.);
        assert_eq!(risk_budget(100., 6., 0.), 0.);
    }
    #[test]
    fn exact_default_guards_reject_changed_nonfinite_and_unknown() {
        assert!(validate(&Params::new()).is_none());
        for (k, v) in [
            ("ema", 13.5),
            ("next_open", 0.),
            ("wfilter", 1.),
            ("sz_coef", 2.),
            ("junk", 0.),
            ("ema", f64::NAN),
            ("fast", f64::INFINITY),
        ] {
            assert!(validate(&Params::from([(k.into(), v)])).is_some());
        }
    }
    #[test]
    fn known_protection_is_active_on_market_entry_day() {
        let mut r = rows(&["2020-01-01", "2020-01-02", "2020-01-03"]);
        r[1].low = 94.;
        let x = Ctx::new(&r, &r, &[]);
        let a = execute(&x, 0, 2, 0., &fixture(3, 1)).unwrap();
        assert!(!a.trades[0].open);
        assert_eq!((a.trades[0].legs[0].i, a.trades[0].exit_i), (1, 1));
        assert_eq!(a.trades[0].exit_px, 95.);
        assert!((a.days[1].equity - 0.98).abs() < 1e-12);
    }
    #[test]
    fn short_proceeds_are_restricted_and_gap_losses_are_not_clipped() {
        let mut r = rows(&["2020-01-01", "2020-01-02", "2020-01-03"]);
        r[2].open = 110.;
        r[2].high = 111.;
        r[2].low = 109.;
        r[2].close = 110.;
        let x = Ctx::new(&r, &r, &[]);
        let a = execute(&x, 0, 2, 0., &fixture(3, -1)).unwrap();
        let d = &a.days[1];
        assert!((d.cash - 0.6).abs() < 1e-12);
        assert!((d.reserve - 0.4).abs() < 1e-12);
        assert_eq!(d.proceeds, d.reserve);
        assert_eq!(a.trades[0].exit_px, 110.);
        assert!(a.trade_audit[0].net < -a.trade_audit[0].planned_risk);
        assert!((a.days[2].equity - 0.96).abs() < 1e-12);
    }
    #[test]
    fn terminal_mark_pays_no_exit_fee_and_gross_keeps_same_quantity() {
        let r = rows(&["2020-01-01", "2020-01-02", "2020-01-03"]);
        let x = Ctx::new(&r, &r, &[]);
        let a = execute(&x, 0, 2, 0.0005, &fixture(3, 1)).unwrap();
        let t = &a.trade_audit[0];
        assert!(a.trades[0].open);
        assert_eq!(t.exit_fee, 0.);
        assert!((t.notional * (0.05 + 0.001) - 0.02).abs() < 1e-12);
        assert!((a.days[2].gross - a.days[2].equity - t.entry_fee).abs() < 1e-12);
    }
    #[test]
    fn new_month_uses_prior_mark_and_tightens_held_risk() {
        let mut r = rows(&["2020-01-30", "2020-01-31", "2020-02-03"]);
        r[1].high = 111.;
        r[1].close = 110.;
        r[2].open = 110.;
        r[2].high = 112.;
        r[2].low = 109.;
        r[2].close = 111.;
        let x = Ctx::new(&r, &r, &[]);
        let a = execute(&x, 0, 2, 0., &fixture(3, 1)).unwrap();
        let d = &a.days[2];
        assert!((d.reference - 1.04).abs() < 1e-12);
        assert!((d.stop - 104.8).abs() < 1e-12);
        assert!((d.outstanding - 0.02 * d.reference).abs() < 1e-12);
    }
    #[test]
    fn calendar_week_completion_is_causal_and_flat_warmup_is_valid() {
        let mut r = vec![];
        for m in 1..=8 {
            for d in 1..=28 {
                r.push(Row {
                    ticker: "TEST_STOCK".into(),
                    date: format!("2020-{m:02}-{d:02}"),
                    open: 100.,
                    high: 101.,
                    low: 99.,
                    close: 100.,
                    volume: Some(1000.),
                });
            }
        }
        let mon = r.iter().position(|r| r.date == "2020-07-20").unwrap();
        let sun = mon - 1;
        r[sun].close = 100.5;
        let x = Ctx::new(&r, &r, &[]);
        let full = states(&x);
        let prefix = Ctx::new(&r[..mon], &r[..mon], &[]);
        let prior = states(&prefix);
        assert_eq!(full.weekly[..mon], prior.weekly);
        assert!(full.weekly_ready[mon]);
        assert_eq!(full.weekly[mon - 1], 0);
        assert_eq!(full.weekly[mon], 1);
        r[mon].close = 0.5;
        let changed = Ctx::new(&r, &r, &[]);
        assert_eq!(
            states(&changed).weekly[mon],
            1,
            "Monday open never reads Monday close"
        );
        for row in &mut r {
            row.close = 100.;
        }
        let flatctx = Ctx::new(&r, &r, &[]);
        let a = run(&flatctx, mon, r.len() - 1, 0., &Params::new()).unwrap();
        assert!(a.trades.is_empty());
    }
    #[test]
    fn protected_levels_match_primary_table6_1_arithmetic() {
        // Printed175 high/low columns. Calendar and midpoint closes are fixture
        // scaffolding only; SafeZone consumes H/L. This is not a market backtest.
        let hl = [
            (115.90, 110.30),
            (116.40, 113.75),
            (114.05, 111.68),
            (114.75, 112.28),
            (114.85, 111.99),
            (116.70, 113.68),
            (116.90, 114.55),
            (118.05, 114.72),
            (118.65, 114.90),
            (118.95, 113.74),
            (115.10, 112.35),
            (115.86, 111.20),
            (117.25, 115.00),
            (117.75, 115.50),
            (118.18, 115.30),
            (118.90, 115.20),
            (114.15, 110.96),
            (113.18, 111.00),
            (114.15, 112.50),
            (115.80, 112.20),
        ];
        let r: Vec<_> = hl
            .iter()
            .enumerate()
            .map(|(i, &(h, l))| Row {
                ticker: "WORKSHEET_ONLY".into(),
                date: format!("2001-01-{:02}", i + 1),
                open: (h + l) / 2.,
                high: h,
                low: l,
                close: (h + l) / 2.,
                volume: None,
            })
            .collect();
        let x = Ctx::new(&r, &r, &[]);
        let st = states(&x);
        let rounded = |v: f64| (v * 100.).round() / 100.;
        for (i, v) in [
            (13, 113.01),
            (14, 113.51),
            (15, 113.51),
            (16, 113.60),
            (17, 113.60),
            (19, 109.75),
        ] {
            assert_eq!(rounded(st.long_stop[i]), v);
        }
        for (i, v) in [(13, 116.45), (14, 117.28), (15, 118.82), (16, 119.27)] {
            assert_eq!(rounded(st.short_stop[i]), v);
        }
    }
}
