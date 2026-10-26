//! A bounded portfolio adapter over unchanged, admitted single-stock ledgers.
//!
//! This is regular-mode candidate replay, not a reproduction of any author's
//! cross-sectional portfolio. Capacity/cash-rejected candidates are not retried.
//! All prices remain actual observations. Unsupported multi-leg accounting and
//! member data windows are decided before results; rule/accounting holds remain global.
use crate::{
    accounting::Costs,
    baskets::{self, BasketMember},
    connors::{Ctx, Trade},
    Row, State,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

const MAX_SYMBOLS: usize = 2500;
// Large source baskets can produce well over 100,000 independent candidates.
// Keep explicit request bounds without truncating their member/trade lists.
const MAX_CANDIDATES: usize = 2_000_000;
const MAX_MARKS: usize = 16_000_000;
const TIME_LIMIT: Duration = Duration::from_secs(90);
static RUNNING: AtomicBool = AtomicBool::new(false);
struct Permit;
impl Permit {
    fn take() -> Result<Self, String> {
        RUNNING
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map(|_| Self)
            .map_err(|_| {
                "busy: A basket backtest is already running; retry when it finishes.".into()
            })
    }
}
impl Drop for Permit {
    fn drop(&mut self) {
        RUNNING.store(false, Ordering::Release);
    }
}

fn quote(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
fn array(values: impl IntoIterator<Item = String>) -> String {
    format!("[{}]", values.into_iter().collect::<Vec<_>>().join(","))
}
fn exact(v: f64) -> String {
    if v.is_finite() {
        v.to_string()
    } else {
        "null".into()
    }
}

struct Request<'a> {
    id: &'a str,
    basket: &'a str,
    max_positions: usize,
    membership: &'a str,
    from: Option<&'a str>,
    to: Option<&'a str>,
}
impl<'a> Request<'a> {
    fn parse(q: &'a BTreeMap<String, String>) -> Result<Self, String> {
        let id = q.get("id").filter(|s| !s.is_empty()).ok_or("id missing")?;
        let basket = q
            .get("basket")
            .filter(|s| !s.is_empty() && s.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_'))
            .ok_or("A source-qualified basket is required")?;
        let max_positions = q.get("max_positions").map_or(Ok(10), |s| {
            s.parse::<usize>()
                .map_err(|_| "max_positions must be an integer from 1 to 100")
        })?;
        if !(1..=100).contains(&max_positions) {
            return Err("max_positions must be an integer from 1 to 100".into());
        }
        let membership = q.get("membership").map_or("current", String::as_str);
        if !matches!(membership, "current" | "past") {
            return Err("membership must be current or past".into());
        }
        let date = |name: &str| -> Result<Option<&str>, String> {
            match q.get(name) {
                None => Ok(None),
                Some(s) if s.is_empty() => Ok(None),
                Some(s) if baskets::valid_date(s) => Ok(Some(s)),
                _ => Err(format!(
                    "{name} must be a valid YYYY-MM-DD date; no date is repaired"
                )),
            }
        };
        let (from, to) = (date("from")?, date("to")?);
        if from.zip(to).is_some_and(|(f, t)| f >= t) {
            return Err("Choose at least two distinct covered trading dates".into());
        }
        for (key, value) in q.iter().filter(|(key, _)| key.starts_with("p.")) {
            if !value
                .parse::<f64>()
                .is_ok_and(|n| n.is_finite() && n.abs() <= 1_000_000.)
            {
                return Err(format!("{key} must be a finite bounded strategy parameter"));
            }
        }
        Ok(Self {
            id,
            basket,
            max_positions,
            membership,
            from,
            to,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Phase {
    Open,
    Close,
}
impl Phase {
    fn name(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Close => "close",
        }
    }
}

/// Exact executor paths reviewed for portfolio event sequencing. A coincidental
/// equality between an intraday stop and a day's close cannot certify a phase.
/// Extending this list requires reviewing that source runner's fill operations.
fn reviewed_execution_contract(id: &str, book: &str) -> Option<(Phase, Phase)> {
    match (book, id) {
        ("stc9", "f1a" | "f3a" | "rsi2_5d") => Some((Phase::Close, Phase::Close)), // connors::simulate, Entry::Close / Exit::Close
        (
            "hbqts",
            "hbqts_ma5_25"
            | "hbqts_ma1_36"
            | "hbqts_fixed_interval"
            | "hbqts_entry_id1"
            | "hbqts_bb_breakout"
            | "hbqts_cgo"
            | "hbqts_stoch_rsi"
            | "hbqts_dpo"
            | "hbqts_dom"
            | "hbqts_macd_daily",
        ) => Some((Phase::Close, Phase::Close)), // hbqts::reviewed, close price arrays, no target/stop path
        ("ch_bta", "ch_cbpb_time") => Some((Phase::Open, Phase::Close)),
        ("ch_bta", "ch_65sma3cc") => Some((Phase::Open, Phase::Open)), // prior completed-close signal; dated next open
        (
            "kftsm",
            "kftsm_ma" | "kftsm_exp" | "kftsm_lrs" | "kftsm_ndb" | "kftsm_swg" | "kftsm_swing"
            | "kftsm_kama" | "kftsm_adapt_exp" | "kftsm_vidya" | "kftsm_adapt_r2"
            | "kftsm_adapt_rsi" | "kftsm_month_end" | "kftsm_hirsch",
        ) => Some((Phase::Close, Phase::Close)), // run_signal, adaptive_driver and seasonal Bk::at_close paths
        _ => None,
    }
}

const CONSERVATIVE_TIMING: &str = "Portfolio timing: this rule's fill times inside the day are not individually reviewed, so cash is reserved from the open of the entry day and released at the close of the exit day (conservative). This can skip trades that finer timing would allow. Single-stock results are unchanged.";

#[derive(Clone, Copy, Debug)]
struct ExecutionContract {
    phases: (Phase, Phase),
    conservative: bool,
}
impl From<(Phase, Phase)> for ExecutionContract {
    fn from(phases: (Phase, Phase)) -> Self {
        Self {
            phases,
            conservative: false,
        }
    }
}
/// Reservation boundaries for otherwise admitted single-stock ledgers. The
/// fallback does not infer intraday fill times or change source fill prices.
fn execution_contract(id: &str, book: &str) -> ExecutionContract {
    reviewed_execution_contract(id, book)
        .map(Into::into)
        .unwrap_or(ExecutionContract {
            phases: (Phase::Open, Phase::Close),
            conservative: true,
        })
}

#[derive(Clone, Debug)]
struct Candidate {
    symbol: String,
    key: String,
    entry: String,
    exit: String,
    entry_px: f64,
    exit_px: f64,
    dir: f64,
    weight: f64,
    entry_phase: Phase,
    exit_phase: Phase,
    entry_kind: &'static str,
    open: bool,
    reason: String,
    bars: usize,
    marks: BTreeMap<String, f64>,
    /// shared fill-aware MAE/MFE (accounting::excursion) on this stock's own bars
    excursion: Option<(f64, f64, usize)>,
}
impl Candidate {
    fn from_source(
        key: &str,
        ctx: &Ctx,
        t: &Trade,
        contract: ExecutionContract,
    ) -> Result<Self, String> {
        if t.legs.len() != 1 || !t.exits.is_empty() {
            return Err("calculation_unavailable: Basket replay does not yet support scale-in/scale-out event ledgers. No legs are removed or approximated.".into());
        }
        let l = &t.legs[0];
        let (entry_phase, contract_exit) = contract.phases;
        let exit_phase = if t.open { Phase::Close } else { contract_exit };
        if !contract.conservative {
            let entry_phase=match t.entry_kind{"open"=>Phase::Open,"close"=>Phase::Close,_=>return Err("needs_data: Basket stop/limit entries require verified intraday ordering across competing stock signals. Daily OHLC does not order fills; no portfolio path is invented.".into())};
            let equal = |a: f64, b: f64| (a - b).abs() <= 1e-10 * a.abs().max(b.abs()).max(1.);
            if entry_phase != contract.phases.0 {
                return Err("calculation_unavailable: Source entry phase disagrees with the reviewed portfolio executor contract.".into());
            }
            let exit_phase = if t.open {
                Phase::Close
            } else {
                contract.phases.1
            };
            let expected_exit = match exit_phase {
                Phase::Open => ctx.o[t.exit_i],
                Phase::Close => ctx.c[t.exit_i],
            };
            if !equal(t.exit_px, expected_exit) {
                return Err("calculation_unavailable: Source exit price disagrees with its reviewed open/close executor contract; an intraday target cannot become a close by approximation.".into());
            }
            if !t.open && contract.phases.1 == Phase::Open && t.signal_i >= t.exit_i {
                return Err("needs_data: Reviewed next-open exit lacks its prior completed-close signal; release timing remains unresolved.".into());
            }
            if entry_phase == Phase::Open && !equal(l.px, ctx.o[l.i])
                || entry_phase == Phase::Close && !equal(l.px, ctx.c[l.i])
            {
                return Err("calculation_unavailable: Entry kind disagrees with its observed open/close fill price.".into());
            }
        }
        if t.exit_i == l.i && exit_phase < entry_phase {
            return Err(
                "calculation_unavailable: A source exit occurs before its same-day entry phase."
                    .into(),
            );
        }
        Ok(Self {
            symbol: ctx.r[l.i].ticker.clone(),
            key: key.into(),
            entry: ctx.r[l.i].date.clone(),
            exit: ctx.r[t.exit_i].date.clone(),
            entry_px: l.px,
            exit_px: t.exit_px,
            dir: t.dir,
            weight: l.w,
            entry_phase,
            exit_phase,
            entry_kind: t.entry_kind,
            open: t.open,
            reason: t.reason.clone(),
            bars: t.exit_i - l.i,
            marks: (l.i..=t.exit_i)
                .map(|i| (ctx.r[i].date.clone(), ctx.c[i]))
                .collect(),
            excursion: crate::accounting::excursion(ctx, t),
        })
    }
    fn gross_return(&self) -> f64 {
        self.weight * self.dir * (self.exit_px / self.entry_px - 1.)
    }
    fn net_return(&self, cost: f64) -> f64 {
        self.gross_return() - self.weight * cost * if self.open { 1. } else { 2. }
    }
}

#[derive(Debug)]
struct Accepted {
    candidate: usize,
    allocation: f64,
}
#[derive(Debug)]
struct Replay {
    equity: Vec<f64>,
    gross_equity: Vec<f64>,
    cash: Vec<f64>,
    positions: Vec<usize>,
    exposed: Vec<bool>,
    accepted: Vec<Accepted>,
    rejected_capacity: usize,
    rejected_cash: usize,
    /// Exits whose net value was negative, i.e. the loss exceeded the slot and
    /// was paid from free cash.
    slot_loss_settlements: Vec<Settlement>,
    /// Lowest cash after any single cash event (not just end of day).
    minimum_event_cash: f64,
}
#[derive(Debug, Clone)]
struct Settlement {
    symbol: String,
    date: String,
    phase: Phase,
    cash_before: f64,
    cash_change: f64,
}
const FUNDING_DISCLOSURE: &str = "Losses beyond a position's slot are paid from free cash; no borrowing.";

/// Net cash returned by closing a position. Negative when the loss exceeds the
/// reserved slot; the caller settles that from free cash.
fn release_value(t: &Candidate, principal: f64, cost: f64) -> Result<f64, String> {
    let net = principal * (1. + t.dir * (t.exit_px / t.entry_px - 1.)) - principal * cost;
    if !net.is_finite() {
        return Err("calculation_unavailable: Closed position value is not a finite number; no portfolio result is published.".into());
    }
    Ok(net)
}

/// Apply one exit's cash. Cash never goes below zero: no borrowing.
fn settle(
    cash: &mut f64,
    result: &mut Replay,
    t: &Candidate,
    date: &str,
    phase: Phase,
    net: f64,
) -> Result<(), String> {
    let before = *cash;
    let after = before + net;
    if after < -1e-12 {
        return Err(format!("calculation_unavailable: The {} {} closed on {} ({}) lost more than its slot, and the portfolio does not have enough free cash to pay the difference. Cash before {:.6}, change {:.6}, after {:.6}. No borrowing is used, so no portfolio result is published.",t.symbol,if t.dir<0.{"short"}else{"long"},date,phase.name(),before,net,after));
    }
    *cash = after.max(0.);
    if net < 0. {
        result.slot_loss_settlements.push(Settlement { symbol: t.symbol.clone(), date: date.into(), phase, cash_before: before, cash_change: net });
    }
    result.minimum_event_cash = result.minimum_event_cash.min(*cash);
    Ok(())
}

/// Fixed initial-equity slots. Long principal is invested; short principal is
/// reserved as collateral and short-sale proceeds cannot fund another entry.
fn replay(
    candidates: &[Candidate],
    dates: &[String],
    max_positions: usize,
    cost: f64,
) -> Result<Replay, String> {
    if !(1..=100).contains(&max_positions) || !cost.is_finite() || cost < 0. || cost > 0.5 {
        return Err("Invalid portfolio allocation or per-side cost".into());
    }
    if dates.len() < 2 || dates.windows(2).any(|w| w[0] >= w[1]) {
        return Err("Portfolio needs at least two sorted, distinct actual market dates".into());
    }
    let calendar: BTreeSet<_> = dates.iter().map(String::as_str).collect();
    let mut entries: BTreeMap<(&str, Phase), Vec<usize>> = BTreeMap::new();
    for (i, t) in candidates.iter().enumerate() {
        if ![-1., 1.].contains(&t.dir)
            || !t.weight.is_finite()
            || t.weight <= 0.
            || t.weight > 1.
            || !t.entry_px.is_finite()
            || t.entry_px <= 0.
            || !t.exit_px.is_finite()
            || t.exit_px <= 0.
            || !calendar.contains(t.entry.as_str())
            || !calendar.contains(t.exit.as_str())
            || t.entry > t.exit
            || t.entry == t.exit && t.entry_phase > t.exit_phase
        {
            return Err("calculation_unavailable: Candidate fill/chronology is outside the actual portfolio calendar or supported unit allocation".into());
        }
        entries
            .entry((&t.entry, t.entry_phase))
            .or_default()
            .push(i);
    }
    for entry in entries.values_mut() {
        entry.sort_by(|&a, &b| {
            candidates[a]
                .symbol
                .cmp(&candidates[b].symbol)
                .then_with(|| a.cmp(&b))
        });
    }
    let slot = 1. / max_positions as f64;
    let (mut cash, mut gross_cash) = (1., 1.);
    let mut active: BTreeMap<usize, f64> = BTreeMap::new();
    let mut result = Replay {
        equity: vec![],
        gross_equity: vec![],
        cash: vec![],
        positions: vec![],
        exposed: vec![],
        accepted: vec![],
        rejected_capacity: 0,
        rejected_cash: 0,
        slot_loss_settlements: vec![],
        minimum_event_cash: 1.,
    };
    for date in dates {
        let mut exposed = !active.is_empty();
        for phase in [Phase::Open, Phase::Close] {
            let mut closing = vec![];
            for i in active.keys().copied().filter(|&i| {
                let t = &candidates[i];
                !t.open && t.exit == *date && t.exit_phase == phase
            }).collect::<Vec<_>>() {
                let principal = active.remove(&i).unwrap();
                closing.push((i, principal, release_value(&candidates[i], principal, cost)?));
            }
            // Receipts in this phase are banked before losses in this phase are
            // paid; a later phase's receipts cannot fund this phase.
            // Stable: without a loss, settlement order is unchanged.
            closing.sort_by_key(|c| c.2 < 0.);
            for (i, principal, net) in closing {
                let t = &candidates[i];
                settle(&mut cash, &mut result, t, date, phase, net)?;
                gross_cash += principal * (1. + t.dir * (t.exit_px / t.entry_px - 1.));
            }
            if let Some(new) = entries.get(&(date.as_str(), phase)) {
                for &i in new {
                    let t = &candidates[i];
                    if active.len() >= max_positions
                        || active.keys().any(|&j| candidates[j].symbol == t.symbol)
                    {
                        result.rejected_capacity += 1;
                        continue;
                    }
                    let principal = slot * t.weight;
                    let debit = principal * (1. + cost);
                    if cash + 1e-12 < debit {
                        result.rejected_cash += 1;
                        continue;
                    }
                    cash -= debit;
                    gross_cash -= principal;
                    if cash < 0. && cash > -1e-12 {
                        cash = 0.;
                    }
                    result.minimum_event_cash = result.minimum_event_cash.min(cash);
                    result.accepted.push(Accepted {
                        candidate: i,
                        allocation: slot,
                    });
                    exposed = true;
                    if !t.open && t.entry == t.exit && t.exit_phase == phase {
                        settle(&mut cash, &mut result, t, date, phase, release_value(t, principal, cost)?)?;
                        gross_cash += principal * (1. + t.dir * (t.exit_px / t.entry_px - 1.));
                    } else {
                        active.insert(i, principal);
                    }
                }
            }
        }
        let (mut net, mut gross) = (cash, gross_cash);
        for (&i, &principal) in &active {
            let t = &candidates[i];
            let mark=t.marks.get(date).copied().filter(|px|px.is_finite()&&*px>0.).ok_or_else(||format!("needs_data: Held {} position has no actual closing quote on {}. No stale-price carry, guessed delisting exit or forward-filled portfolio value is published.",t.symbol,date))?;
            // A negative open mark lowers equity; no cash moves until the exit.
            let value = principal * (1. + t.dir * (mark / t.entry_px - 1.));
            if !value.is_finite() {
                return Err("calculation_unavailable: Held position value is not a finite number; no portfolio result is published.".into());
            }
            net += value;
            gross += value;
        }
        if !net.is_finite()
            || net <= 0.
            || !gross.is_finite()
            || gross <= 0.
            || !cash.is_finite()
            || cash < -1e-10
        {
            return Err("calculation_unavailable: Portfolio requires nonpositive equity or unsupported funding/margin; no equity reset or invented cash is applied.".into());
        }
        result.equity.push(net);
        result.gross_equity.push(gross);
        result.cash.push(cash);
        result.positions.push(active.len());
        result.exposed.push(exposed);
    }
    Ok(result)
}

struct Coverage {
    symbol: String,
    key: String,
    first: String,
    last: String,
    status: &'static str,
    note: String,
    used_from: Option<String>,
    used_to: Option<String>,
}
impl Coverage {
    fn json(&self) -> String {
        format!(
            "{{\"symbol\":{},\"ticker\":{},\"first\":{},\"last\":{},\"status\":{},\"note\":{},\"used_from\":{},\"used_to\":{}}}",
            quote(&self.symbol),
            quote(&self.key),
            quote(&self.first),
            quote(&self.last),
            quote(self.status),
            quote(&self.note),
            self.used_from.as_deref().map_or("null".into(),quote),
            self.used_to.as_deref().map_or("null".into(),quote)
        )
    }
}
fn coverage(member: &BasketMember, status: &'static str, note: String) -> Coverage {
    Coverage {
        symbol: member.symbol.clone(),
        key: member.key.clone(),
        first: member.first.clone(),
        last: member.last.clone(),
        status,
        note,
        used_from: None,
        used_to: None,
    }
}
fn complete_held_report(members: &[BasketMember], report: &mut Vec<Coverage>) {
    let seen: BTreeSet<_> = report.iter().map(|c| c.key.clone()).collect();
    report.extend(members.iter().filter(|m|!seen.contains(&m.key)).map(|m|coverage(m,"not_processed","The full basket was withheld before this member was computed. No pass/fail, trades or performance is inferred for it.".into())));
}
fn hold(req: &Request, status: &str, note: &str, coverage: &[Coverage]) -> String {
    format!("{{\"kind\":\"basket\",\"status\":{},\"blocked\":{},\"basket\":{},\"max_positions\":{},\"membership\":{},\"stats\":null,\"trades\":[],\"dates\":[],\"equity\":[],\"coverage\":{}}}",quote(status),quote(note),quote(req.basket),req.max_positions,quote(req.membership),array(coverage.iter().map(Coverage::json)))
}

fn portfolio_stats(
    candidates: &[Candidate],
    replay: &Replay,
    dates: &[String],
    cost: f64,
    gross: bool,
) -> BTreeMap<&'static str, f64> {
    let eq = if gross {
        &replay.gross_equity
    } else {
        &replay.equity
    };
    let closed: Vec<_> = replay
        .accepted
        .iter()
        .filter(|a| !candidates[a.candidate].open)
        .collect();
    let tr: Vec<_> = closed
        .iter()
        .map(|a| candidates[a.candidate].net_return(if gross { 0. } else { cost }))
        .collect();
    let n = tr.len() as f64;
    let sum = tr.iter().sum::<f64>();
    let avg = if n > 0. { sum / n } else { f64::NAN };
    let contribution = closed
        .iter()
        .map(|a| a.allocation * candidates[a.candidate].net_return(if gross { 0. } else { cost }))
        .sum::<f64>();
    let wins: Vec<_> = tr.iter().copied().filter(|r| *r > 0.).collect();
    let losses: Vec<_> = tr.iter().copied().filter(|r| *r <= 0.).collect();
    let avg_win = wins.iter().sum::<f64>() / wins.len() as f64;
    let avg_loss = losses.iter().sum::<f64>() / losses.len() as f64;
    let ret: Vec<_> = std::iter::once(eq[0] - 1.)
        .chain(eq.windows(2).map(|w| w[1] / w[0] - 1.))
        .collect();
    let mean = ret.iter().sum::<f64>() / ret.len() as f64;
    let sd = (ret.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / (ret.len() - 1).max(1) as f64)
        .sqrt();
    let dsd = (ret.iter().map(|r| r.min(0.).powi(2)).sum::<f64>() / ret.len() as f64).sqrt();
    let years = ((crate::day_number(dates.last().unwrap()) - crate::day_number(&dates[0])) as f64
        / 365.25)
        .max(1. / 365.25);
    let total = eq.last().unwrap() - 1.;
    let cagr = eq.last().unwrap().powf(1. / years) - 1.;
    let (dd, dd_days) = crate::max_dd(eq);
    let held_days = closed
        .iter()
        .map(|a| {
            let t = &candidates[a.candidate];
            (crate::day_number(&t.exit) - crate::day_number(&t.entry)).max(1) as f64
        })
        .sum::<f64>()
        / n;
    let mut s = BTreeMap::from([
        ("net_profit", contribution),
        ("closed_return_sum", sum),
        ("total_return", total),
        ("compounded_return", total),
        ("avg_net_pnl", avg),
        ("avg_trade", avg),
        ("trades", n),
        ("win_rate", wins.len() as f64 / n),
        ("avg_win", avg_win),
        ("avg_loss", avg_loss),
        (
            "best",
            tr.iter().copied().reduce(f64::max).unwrap_or(f64::NAN),
        ),
        (
            "worst",
            tr.iter().copied().reduce(f64::min).unwrap_or(f64::NAN),
        ),
        ("expectancy_ratio", avg / -avg_loss),
        ("sharpe", mean / sd * 252f64.sqrt()),
        ("sortino", mean / dsd * 252f64.sqrt()),
        ("vol", sd * 252f64.sqrt()),
        ("max_dd", dd),
        ("max_dd_days", dd_days as f64),
        ("cagr", cagr),
        ("years", years),
        ("mar", cagr / -dd),
        ("car_mdd", cagr / -dd),
        (
            "profit_factor",
            wins.iter().sum::<f64>() / -losses.iter().sum::<f64>(),
        ),
        ("payoff", avg_win / -avg_loss),
        ("avg_hold_days", held_days),
        ("monthly_thickness", avg / held_days * 30.),
        (
            "avg_bars",
            closed
                .iter()
                .map(|a| candidates[a.candidate].bars as f64)
                .sum::<f64>()
                / n,
        ),
        (
            "avg_deployed",
            closed
                .iter()
                .map(|a| candidates[a.candidate].weight)
                .sum::<f64>()
                / n,
        ),
        ("trades_per_year", n / years),
        (
            "exposure",
            replay.exposed.iter().filter(|&&held| held).count() as f64 / dates.len() as f64,
        ),
    ]);
    let mut sorted = tr;
    sorted.sort_by(f64::total_cmp);
    s.insert(
        "median_trade",
        match sorted.len() {
            0 => f64::NAN,
            n if n % 2 == 0 => (sorted[n / 2 - 1] + sorted[n / 2]) / 2.,
            n => sorted[n / 2],
        },
    );
    s
}

/// Last-gap selection is based solely on actual market dates, before results.
fn covered_window(rows: &[Row], calendar: &[String]) -> Option<(usize, usize, Option<String>)> {
    let first = rows.first()?.date.as_str().max(calendar.first()?.as_str());
    let last = rows.last()?.date.as_str().min(calendar.last()?.as_str());
    if first > last {
        return None;
    }
    let end = rows
        .partition_point(|r| r.date.as_str() <= last)
        .checked_sub(1)?;
    let observed: BTreeSet<_> = rows.iter().map(|r| r.date.as_str()).collect();
    let gap = calendar
        .iter()
        .rev()
        .find(|d| d.as_str() >= first && d.as_str() <= last && !observed.contains(d.as_str()));
    let market_dates: BTreeSet<_> = calendar.iter().map(String::as_str).collect();
    let outside = rows
        .iter()
        .rev()
        .find(|r| {
            r.date.as_str() >= first
                && r.date.as_str() <= last
                && !market_dates.contains(r.date.as_str())
        })
        .map(|r| r.date.clone());
    let gap = gap.cloned().into_iter().chain(outside).max();
    let start = gap.as_ref().map_or_else(
        || rows.partition_point(|r| r.date.as_str() < first),
        |g| rows.partition_point(|r| r.date <= *g),
    );
    (start <= end).then(|| (start, end, gap))
}

/// Observation requirements of the reviewed runners, including chained
/// windows. This is an admission floor, never a signal/performance test.
/// Other rules retain their own readiness and NaN warm-up guards.
fn member_warmup(id: &str, p: &crate::connors::Params) -> usize {
    let schema = crate::connors::params(id);
    let u = |key: &str| {
        p.get(key)
            .copied()
            .or_else(|| schema.iter().find(|d| d.key == key).map(|d| d.def))
            .unwrap_or(0.)
            .round()
            .max(0.) as usize
    };
    match id {
        "f1a" | "f3a" => u("trend").max(u("low_n")),
        "ss_anti" | "ss_anti_trigger" => {
            let fast = (u("k_len") + u("k_smooth")).saturating_sub(1);
            let slow = if u("d_mode") == 0 {
                (fast + u("d_len")).saturating_sub(1)
            } else {
                u("d_len") + 4
            };
            slow.max(fast) + u("trend_bars").max(2)
        }
        "jb624_mac2_stop" => u("mah").max(u("mal")) + u("bars"),
        "jr_123_std_l" | "jr_123_std_s" => {
            if u("exit") == 1 {
                5
            } else {
                4
            }
        }
        "eltfl_fi2" => u("ema").max(u("fi") + 1),
        _ => schema
            .iter()
            .filter(|d| {
                matches!(
                    d.key,
                    "fast"
                        | "slow"
                        | "trend"
                        | "ma"
                        | "len"
                        | "ema"
                        | "rsi_n"
                        | "atr_n"
                        | "look"
                        | "lookback"
                )
            })
            .map(|d| u(d.key))
            .max()
            .unwrap_or(0),
    }
}

/// Replay consumes unchanged source fills, not an independently compounded
/// single-stock account. Only accepted portfolio positions use cash/collateral;
/// every source event still passes structural and multi-leg admission.
fn member_trades(
    ctx: &Ctx,
    id: &str,
    start: usize,
    end: usize,
    cost: f64,
    params: &crate::connors::Params,
) -> Result<Vec<Trade>, String> {
    if crate::books::hbiab_reviewed::reviewed(id)
        || id == crate::books::elcim_account::ID
        || crate::books::hbqts::reviewed::is_dema_index(id)
    {
        return crate::backtest(ctx, id, start, end, cost, params).map(|r| r.trades);
    }
    let trades = crate::connors::run(id, ctx, start, end, params)
        .ok_or("Strategy implementation unavailable")?;
    crate::trade_validation::check_structure(&trades, start, end)?;
    Ok(trades)
}

pub fn run(st: &State, q: &BTreeMap<String, String>) -> Result<String, String> {
    let req = Request::parse(q)?;
    let costs = Costs::parse(q)?;
    costs.ensure_rollover()?;
    let catalogue = crate::query_catalogue(q);
    let def = catalogue
        .iter()
        .find(|d| d.id == req.id)
        .ok_or("unknown strategy")?;
    match def.status {
        crate::connors::Status::NeedsData(note) => {
            return Ok(hold(
                &req,
                "needs_data",
                &format!("Full basket withheld by rule admission: {note}"),
                &[],
            ))
        }
        crate::connors::Status::NotTestable(note) => {
            return Ok(hold(
                &req,
                "not_testable",
                &format!("Full basket withheld by rule admission: {note}"),
                &[],
            ))
        }
        crate::connors::Status::Live => {}
    }
    if crate::instrument_policy::reviewed_kestner_stock(def) {
        return Ok(hold(&req,"not_testable","The Kestner review covers one actual stock at a time. Simultaneous long/short basket selection, capacity and portfolio accounting have not been reviewed for these methods; no basket result is published.",&[]));
    }
    let members = st.baskets.source_members(req.basket);
    if members.is_empty() {
        return Ok(hold(
            &req,
            "needs_data",
            "No actual admitted stock quotes are loaded for this source-qualified basket.",
            &[],
        ));
    }
    if members.len() > MAX_SYMBOLS {
        return Ok(hold(&req,"unavailable","Basket exceeds the bounded 2,500-symbol research request limit; no truncated universe is published.",&[]));
    }
    if req.membership == "past"
        || members
            .iter()
            .any(|m| m.membership_kind != "current_snapshot")
    {
        return Ok(hold(&req,"needs_data","Past basket backtests require a reviewed, per-date membership contract and covered former/delisted stock prices. Current & Past watchlist unions and arbitrary Aux1/Aux2 columns do not establish historical eligibility. No survivorship-free result is claimed.",&members.iter().map(|m|coverage(m,"needs_data","Dated historical membership is unverified".into())).collect::<Vec<_>>()));
    }
    let contract = execution_contract(def.id, def.book);
    let _permit = Permit::take()?;
    let started = Instant::now();
    let params = crate::params_of(q);
    let market = members[0].market;
    if members.iter().any(|m| m.market != market) {
        return Ok(hold(
            &req,
            "needs_data",
            "The basket mixes trusted market identities; no mixed currency result is published.",
            &[],
        ));
    }
    let benchmark_symbol = crate::native_benchmark(market);
    let Some(benchmark) = st
        .ctxs
        .get(benchmark_symbol)
        .filter(|c| !c.r.is_empty() && c.r.iter().all(|r| r.ticker == benchmark_symbol))
    else {
        return Ok(hold(&req,"needs_data","An actual market benchmark calendar is required to detect missing daily marks; no calendar or benchmark prices are invented.",&[]));
    };
    let mut actual_first: Option<String> = None;
    let mut actual_last: Option<String> = None;
    if req.from.is_none() || req.to.is_none() {
        for member in &members {
            if let Ok(rows) = st.baskets.basket_rows(&member.key) {
                let first = &rows[0].date;
                let last = &rows.last().unwrap().date;
                if actual_first.as_ref().is_none_or(|f| first < f) {
                    actual_first = Some(first.clone());
                }
                if actual_last.as_ref().is_none_or(|l| last > l) {
                    actual_last = Some(last.clone());
                }
            }
        }
    }
    let from = req.from.unwrap_or_else(|| {
        actual_first
            .as_deref()
            .unwrap_or(&benchmark.r[0].date)
            .max(benchmark.r[0].date.as_str())
    });
    let to = req.to.unwrap_or_else(|| {
        actual_last
            .as_deref()
            .unwrap_or(&benchmark.r.last().unwrap().date)
            .min(benchmark.r.last().unwrap().date.as_str())
    });
    if from >= to
        || from < benchmark.r[0].date.as_str()
        || to > benchmark.r.last().unwrap().date.as_str()
    {
        return Ok(hold(&req,"needs_data","Requested dates exceed the actual market calendar coverage. Choose a fully covered market period; the requested end is not silently shortened.",&[]));
    }
    let calendar: Vec<String> = benchmark
        .r
        .iter()
        .filter(|r| r.date.as_str() >= from && r.date.as_str() <= to)
        .map(|r| r.date.clone())
        .collect();
    if calendar.len() < 2 {
        return Ok(hold(
            &req,
            "needs_data",
            "Fewer than two actual market dates are covered by this request.",
            &[],
        ));
    }
    let mut candidates = Vec::new();
    let mut report = Vec::new();
    let mut total_marks = 0usize;
    // Complete every data decision before generating any member's candidates.
    let mut admitted = Vec::new();
    for member in &members {
        if started.elapsed() > TIME_LIMIT {
            complete_held_report(&members, &mut report);
            return Ok(hold(&req,"unavailable","Basket admission reached its checked 90-second resource budget; no partial result is published.",&report));
        }
        if let Some(evidence) = st.baskets.native_evidence(&member.key) {
            if let Some(issue) =
                crate::instrument_policy::check_native(def, &member.symbol, evidence)
            {
                return Ok(hold(
                    &req,
                    issue.status,
                    &format!(
                        "Full basket withheld by rule instrument admission: {}",
                        issue.note
                    ),
                    &report,
                ));
            }
        }
        let mut item = coverage(member, "excluded", String::new());
        let rows = match st.baskets.basket_rows(&member.key) {
            Ok(rows) => rows,
            Err(note) => {
                item.first.clear();
                item.last.clear();
                item.note = format!("Excluded: {note}. No price, bar or signal is filled.");
                report.push(item);
                continue;
            }
        };
        item.first = rows[0].date.clone();
        item.last = rows.last().unwrap().date.clone();
        let Some((start, end, gap)) = covered_window(&rows, &calendar) else {
            item.note = "Excluded: actual prices do not overlap the requested period.".into();
            report.push(item);
            continue;
        };
        let mut notes = Vec::new();
        if item.first.as_str() > from || item.last.as_str() < to {
            notes.push(format!(
                "Actual source coverage is {} to {} within the requested period.",
                item.first, item.last
            ));
        }
        if item.first != member.first || item.last != member.last {
            notes.push("The file's actual first/last bars differ from catalogue dates; actual bars determine this window.".into());
        }
        if let Some(date) = &gap {
            notes.push(format!("Starts after the last missing market date or stock/calendar date mismatch, {date}; earlier signals are not replayed."));
        }
        if rows[end].date < *calendar.last().unwrap() {
            notes.push(format!("Portfolio convention: any position still open is closed at the last available price (data ends {}), using that actual close and normal exit costs. The source strategy is unchanged.",rows[end].date));
        }
        let warmup = member_warmup(req.id, &params);
        // Keep the full real prefix for unchanged indicator calculations. A gap
        // does not create a new seeded series; the suffix must itself be long enough.
        if (gap.is_some() || start == 0) && end - start + 1 < warmup + 2 {
            item.note=format!("Excluded: continuous window {} to {} has {} actual bars; this rule requires {warmup} warm-up bars plus 2 bars. {}",rows[start].date,rows[end].date,end-start+1,notes.join(" "));
            report.push(item);
            continue;
        }
        let Some(evidence) = st.baskets.native_evidence(&member.key) else {
            unreachable!()
        };
        let selected = crate::SelectedContext::Native { rows, evidence };
        let held = selected.context(st);
        let ctx: &Ctx = &held;
        let mut single = q.clone();
        single.insert("ticker".into(), member.key.clone());
        single.insert("from".into(), ctx.r[start].date.clone());
        single.insert("to".into(), ctx.r[end].date.clone());
        let admission = crate::readiness::auto_start(ctx, start, end, &single["from"], |f| {
            crate::readiness::check(ctx, def, f, end, &params)
                .or_else(|| crate::native_admission(ctx, def, f, end, &selected, st, &single))
        });
        let (start, auto) = match admission {
            Ok(value) => value,
            Err(issue) if issue.status == "needs_data" => {
                item.note = format!("Excluded: {} {}", issue.note, notes.join(" "));
                report.push(item);
                continue;
            }
            Err(issue) => {
                item.status = issue.status;
                item.note = issue.note.clone();
                report.push(item);
                complete_held_report(&members, &mut report);
                return Ok(hold(
                    &req,
                    issue.status,
                    &format!("Full basket withheld by rule admission: {}", issue.note),
                    &report,
                ));
            }
        };
        if let Some(note) = auto {
            notes.push(note);
        }
        let Some(start) = crate::window_start(ctx, def, start).filter(|&i| i < end) else {
            item.note =
                "Excluded: source warm-up or volatility does not cover two trading bars.".into();
            report.push(item);
            continue;
        };
        item.used_from = Some(ctx.r[start].date.clone());
        item.used_to = Some(ctx.r[end].date.clone());
        item.status = if notes.is_empty() {
            "ready"
        } else {
            "adjusted"
        };
        item.note = if notes.is_empty() {
            "Actual source bars and unchanged strategy guards passed.".into()
        } else {
            notes.join(" ")
        };
        // Record dates, not indicators or results; bounded per-member arrays drop here.
        admitted.push((
            member.key.clone(),
            single,
            ctx.r[start].date.clone(),
            ctx.r[end].date.clone(),
        ));
        report.push(item);
    }
    let expected = st
        .baskets
        .expected_member_count(req.basket)
        .unwrap_or(members.len())
        .max(members.len());
    for i in members.len()..expected {
        report.push(Coverage{symbol:format!("Unnamed source member {}",i+1),key:String::new(),first:String::new(),last:String::new(),status:"excluded",note:"Excluded: the source count includes this member, but its identity and price file are absent from the supplied catalogue. No identity or prices are invented.".into(),used_from:None,used_to:None});
    }
    let used = admitted.len();
    if used == 0 {
        return Ok(hold(
            &req,
            "needs_data",
            "No basket member has a usable overlapping price window.",
            &report,
        ));
    }
    for (key, mut single, used_from, used_to) in admitted {
        if started.elapsed() > TIME_LIMIT {
            return Ok(hold(&req,"unavailable","Basket request reached its checked 90-second resource budget; no partial result is published.",&report));
        }
        single.insert("from".into(), used_from);
        single.insert("to".into(), used_to);
        let rows = st.baskets.basket_rows(&key)?;
        let selected = crate::SelectedContext::Native {
            rows,
            evidence: st.baskets.native_evidence(&key).unwrap(),
        };
        let held = selected.context(st);
        let (ctx, start, end, cost) = crate::resolve_context(&held, &single)?;
        let trades = match member_trades(ctx, req.id, start, end, cost, &params) {
            Ok(trades) => trades,
            Err(note) => {
                return Ok(hold(
                    &req,
                    "calculation_unavailable",
                    &format!("Full basket withheld by rule calculation for {key}: {note}"),
                    &report,
                ))
            }
        };
        for trade in &trades {
            let mut candidate = match Candidate::from_source(&key, ctx, trade, contract) {
                Ok(candidate) => candidate,
                Err(note) => {
                    return Ok(hold(
                        &req,
                        "calculation_unavailable",
                        &format!("Full basket withheld by rule accounting: {note}"),
                        &report,
                    ))
                }
            };
            if candidate.open && candidate.exit < *calendar.last().unwrap() {
                candidate.open = false;
                candidate.exit_px = ctx.c[trade.exit_i];
                candidate.exit_phase = Phase::Close;
                candidate.reason = format!(
                    "closed at the last available price (data ends {})",
                    candidate.exit
                );
            }
            total_marks += candidate.marks.len();
            if total_marks > MAX_MARKS || candidates.len() >= MAX_CANDIDATES {
                return Ok(hold(&req,"unavailable","Basket exceeds its bounded candidate/quote memory budget; no truncated stock or trade list is published.",&report));
            }
            candidates.push(candidate);
        }
    }
    let adjusted = report.iter().filter(|c| c.status == "adjusted").count();
    let excluded = expected - used;
    let member_report=format!("{{\"expected_members\":{expected},\"used_members\":{used},\"adjusted_members\":{adjusted},\"excluded_members\":{excluded},\"summary\":{},\"basis\":\"Member windows and exclusions are decided from actual data and rule readiness before any trade results. No prices, bars or signals are filled.\"}}",quote(&format!("{used} of {expected} members used; {adjusted} adjusted; {excluded} excluded.")));
    let replay = match replay(&candidates, &calendar, req.max_positions, costs.side_rate()) {
        Ok(run) => run,
        Err(note) => {
            return Ok(hold(
                &req,
                if note.starts_with("calculation_unavailable") {
                    "calculation_unavailable"
                } else {
                    "needs_data"
                },
                &note,
                &report,
            ))
        }
    };
    if started.elapsed() > TIME_LIMIT {
        return Ok(hold(&req,"unavailable","Basket request reached its checked 90-second resource budget; no partial result is published.",&report));
    }
    let stats = portfolio_stats(&candidates, &replay, &calendar, costs.side_rate(), false);
    let gross_stats = portfolio_stats(&candidates, &replay, &calendar, 0., true);
    let bh: Vec<_> = benchmark
        .r
        .iter()
        .filter(|r| r.date.as_str() >= from && r.date.as_str() <= to)
        .map(|r| r.close)
        .collect();
    let bh: Vec<_> = bh.iter().map(|c| c / bh[0]).collect();
    let benchmark_run = crate::Run {
        source_account: None,
        bandy_account: None,
        unit_exposure_report: None,
        option_account: None,
        trades: vec![],
        from: 0,
        to: calendar.len() - 1,
        equity: bh.clone(),
        bh: bh.clone(),
    };
    let bench_rows: Vec<Row> = benchmark
        .r
        .iter()
        .filter(|r| r.date.as_str() >= from && r.date.as_str() <= to)
        .cloned()
        .collect();
    let bench_ctx = Ctx::new(&bench_rows, &bench_rows, &[]);
    let benchmark_stats = crate::benchmark_stats(&bench_ctx, &benchmark_run);
    let portfolio_costs=costs.json().replace("\"open_trade_cost_basis\":\"Legacy equity includes estimated exit costs on marked-open trades\"","\"open_trade_cost_basis\":\"Portfolio open positions deduct entry costs only; exit costs apply to actual closed positions\",\"open_exit_cost_estimated\":false");
    let trade_json=replay.accepted.iter().map(|accepted|{
        let t=&candidates[accepted.candidate];let entry_i=calendar.binary_search(&t.entry).unwrap();let exit_i=calendar.binary_search(&t.exit).unwrap();
        let gross=t.gross_return();let net=t.net_return(costs.side_rate());
        format!("{{\"symbol\":{},\"ticker\":{},\"dir\":{},\"legs\":[[{},{},{}]],\"entry\":{},\"exit\":{},\"exit_i\":{},\"exit_px\":{},\"avg_px\":{},\"ret\":{},\"bars\":{},\"open\":{},\"reason\":{},\"entry_kind\":{},\"entry_phase\":{},\"exit_phase\":{},\"allocation\":{},\"portfolio_contribution\":{},\"exits\":[],\"accounting\":{{\"symbol\":{},\"signal_side\":{},\"pts\":{},\"days_held\":{},\"gross_ret\":{},\"net_ret\":{},\"slippage_ret\":{},\"fee_ret\":{},\"gross_pnl\":null,\"net_pnl\":null,\"taxes\":null,\"slippage\":null,\"fees\":null,\"mae\":null,\"mfe\":null,\"mae_ret_observed\":{},\"mfe_ret_observed\":{},\"excursion_bars\":{},\"excursion_status\":{},\"excursion_basis\":{},\"cost_status\":{},\"return_basis\":\"Unchanged source-unit return; portfolio contribution is fixed initial-equity slot times this return. Fractional normalized units, no monetary capital.\"}}}}",quote(&t.symbol),quote(&t.key),t.dir,entry_i,exact(t.entry_px),exact(t.weight),quote(&t.entry),quote(&t.exit),exit_i,exact(t.exit_px),exact(t.entry_px),exact(net),t.bars,t.open,quote(&t.reason),quote(t.entry_kind),quote(t.entry_phase.name()),quote(t.exit_phase.name()),exact(accepted.allocation),exact(accepted.allocation*net),quote(&t.symbol),quote(if t.dir>0.{"Long"}else{"Short"}),exact(t.dir*(t.exit_px-t.entry_px)),crate::day_number(&t.exit)-crate::day_number(&t.entry),exact(gross),exact(net),exact(t.weight*costs.slippage_bps/10000.*if t.open{1.}else{2.}),exact(t.weight*costs.fees_bps/10000.*if t.open{1.}else{2.}),t.excursion.map_or("null".into(),|e|exact(e.0)),t.excursion.map_or("null".into(),|e|exact(e.1)),t.excursion.map_or(0,|e|e.2),quote(if t.excursion.is_some(){crate::accounting::EXCURSION_STATUS}else{"unavailable"}),quote(crate::accounting::EXCURSION_BASIS),quote(if t.open{"entry_cost_only_actual_open_mark"}else{"modeled_round_trip_cost"}))
    });
    Ok(format!("{{\"kind\":\"basket\",\"status\":\"ready\",\"basket\":{},\"membership\":\"current\",\"model_initial_equity\":1,\"dates\":{},\"equity\":{},\"gross_equity\":{},\"bh\":{},\"close\":[],\"ohlc\":null,\"trades\":{},\"stats\":{},\"gross_stats\":{},\"benchmark_symbol\":{},\"benchmark_stats\":{},\"costs\":{},\"coverage\":{},\"member_report\":{},\"portfolio\":{{\"effective_from\":{},\"effective_to\":{},\"window_basis\":\"Explicit dates are never silently shortened. Default dates intersect admitted member history with the actual benchmark calendar; full earlier stock history is not implied.\",\"max_positions\":{},\"slot_allocation\":{},\"closed_contribution\":{},\"accepted_positions\":{},\"rejected_capacity\":{},\"rejected_cash\":{},\"slot_loss_settlements\":{},\"funding_disclosure\":{},\"source_members\":{},\"cash\":{},\"positions\":{},\"historical_membership_verified\":false,\"compounding_supported\":false,\"ranking\":\"Alphabetical symbol order within each observed open/close phase\",\"signal_mode\":\"Regular-mode replay of unchanged independent-symbol candidate ledgers; rejected entries are not deferred or retried\",\"allocation_basis\":\"Each full position reserves 1/N of initial normalized equity; partially deployed source legs use their observed weight. Unused slots remain cash. Fractional research units; no share lot rounding, leverage, margin financing or interest. Short principal is reserved and short proceeds cannot fund entries.\",\"timing_contract_reviewed\":{},\"timing_disclosure\":{},\"timing_basis\":{},\"gross_basis\":\"Zero-cost accounting on the identical accepted fills; gross does not independently select a different candidate list\"}},\"notices\":[\"Source-listed current membership is a static snapshot. A current snapshot alone does not establish historical membership; retrospective selection has survivorship bias and this is not a historical membership reproduction.\",\"This is a declared portfolio application of an admitted stock rule, not the author's published portfolio, rankings or performance.\",\"Net Profit is closed allocated contribution relative to initial unit equity. Avg net pnl is the actual source-unit net return per closed trade; multiplying it by trade count gives the unallocated trade-return sum, not basket Net Profit.\",\"Equity and risk statistics use cash plus simultaneously held actual daily marks. Positions still open at the basket end deduct entry costs only. If a member's data ends sooner, its remaining position closes at its last actual close with normal exit costs, as disclosed in its member report and trade list.\"],\"accounting_basis\":\"Fixed initial-equity slots, chronological cash and actual held-position marks. Basket Net Profit excludes open positions. Overlapping stock returns are never compounded in exit order. No capital-related statistics are supplied.\"}}",quote(req.basket),array(calendar.iter().map(|s|quote(s))),array(replay.equity.iter().map(|v|exact(*v))),array(replay.gross_equity.iter().map(|v|exact(*v))),array(bh.iter().map(|v|exact(*v))),array(trade_json),crate::stats_json(&stats),crate::stats_json(&gross_stats),quote(benchmark_symbol),crate::stats_json(&benchmark_stats),portfolio_costs,array(report.iter().map(Coverage::json)),member_report,quote(calendar.first().unwrap()),quote(calendar.last().unwrap()),req.max_positions,exact(1./req.max_positions as f64),exact(stats["net_profit"]),replay.accepted.len(),replay.rejected_capacity,replay.rejected_cash,replay.slot_loss_settlements.len(),quote(FUNDING_DISCLOSURE),expected,array(replay.cash.iter().map(|v|exact(*v))),array(replay.positions.iter().map(|v|v.to_string())), !contract.conservative,
        if contract.conservative { quote(CONSERVATIVE_TIMING) } else { "null".into() },
        quote(if contract.conservative { "Cash and slots are reserved from entry-day open through exit-day close. Source fill prices are unchanged; multi-leg positions are withheld." } else { "Observed prior-signal next-open exits before open entries; observed close exits before close entries. Intraday stop/limit ordering and multi-leg positions are withheld." })))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn dates() -> Vec<String> {
        (1..=5).map(|i| format!("2020-01-0{i}")).collect()
    }
    fn candidate(symbol: &str, entry: usize, exit: usize, px: f64) -> Candidate {
        let d = dates();
        Candidate {
            symbol: symbol.into(),
            key: format!("basket:nifty50:{symbol}"),
            entry: d[entry].clone(),
            exit: d[exit].clone(),
            entry_px: 100.,
            exit_px: px,
            dir: 1.,
            weight: 1.,
            entry_phase: Phase::Close,
            exit_phase: Phase::Close,
            entry_kind: "close",
            open: false,
            reason: "arithmetic fixture".into(),
            bars: exit - entry,
            marks: (entry..=exit)
                .map(|i| (d[i].clone(), if i == exit { px } else { 100. }))
                .collect(),
            excursion: None,
        }
    }
    fn near(a: f64, b: f64) {
        assert!((a - b).abs() < 1e-10, "{a} != {b}");
    }
    #[test]
    fn overlapping_positions_use_weighted_cash_marks_not_exit_order_product() {
        let a = candidate("A", 0, 3, 120.);
        let b = candidate("B", 0, 4, 90.);
        let r = replay(&[a, b], &dates(), 2, 0.).unwrap();
        near(r.equity[3], 1.10);
        near(r.equity[4], 1.05);
        near(r.cash[4], 1.05);
        assert_eq!(r.positions[0], 2);
    }
    #[test]
    fn alphabetical_cap_rejects_candidate_without_deferring_it() {
        let r = replay(
            &[
                candidate("Z", 0, 4, 180.),
                candidate("A", 0, 1, 110.),
                candidate("B", 2, 4, 120.),
            ],
            &dates(),
            1,
            0.,
        )
        .unwrap();
        assert_eq!(
            r.accepted.iter().map(|a| a.candidate).collect::<Vec<_>>(),
            vec![1, 2]
        );
        assert_eq!(r.rejected_capacity, 1);
        near(r.equity[4], 1.30);
    }
    #[test]
    fn unused_slots_remain_cash_and_loss_can_prevent_next_full_slot() {
        let r = replay(
            &[candidate("A", 0, 1, 90.), candidate("B", 2, 4, 120.)],
            &dates(),
            1,
            0.,
        )
        .unwrap();
        assert_eq!(r.rejected_cash, 1);
        assert_eq!(r.accepted.len(), 1);
        near(r.cash[4], 0.90);
        let r = replay(&[candidate("A", 0, 4, 120.)], &dates(), 10, 0.).unwrap();
        near(r.cash[0], 0.90);
        near(r.equity[4], 1.02);
    }
    #[test]
    fn both_sides_charge_once_and_entry_fees_are_not_funded_with_margin() {
        let r = replay(&[candidate("A", 0, 4, 110.)], &dates(), 2, 0.001).unwrap();
        near(r.equity[0], 0.9995);
        near(r.equity[4], 1.049);
        let source = vec![candidate("A", 0, 4, 110.), candidate("B", 0, 4, 110.)];
        let r = replay(&source, &dates(), 2, 0.001).unwrap();
        assert_eq!(r.rejected_cash, 1);
        // Gross comparison retains the net-admitted fills. A fresh zero-cost
        // selection would accept two and is deliberately not that comparison.
        near(r.gross_equity[4], 1.05);
        let zero = replay(&source, &dates(), 2, 0.).unwrap();
        near(zero.gross_equity[4], 1.10);
    }
    #[test]
    fn shorts_reserve_principal_and_cannot_spend_sale_proceeds() {
        let mut a = candidate("A", 0, 4, 80.);
        a.dir = -1.;
        let r = replay(
            &[a, candidate("B", 0, 4, 120.), candidate("C", 0, 4, 130.)],
            &dates(),
            2,
            0.,
        )
        .unwrap();
        assert_eq!(r.accepted.len(), 2);
        assert_eq!(r.rejected_capacity, 1);
        near(r.cash[0], 0.);
        near(r.equity[4], 1.20);
    }
    #[test]
    fn open_exits_free_slots_before_open_entries_but_close_exit_cannot_fund_earlier_open() {
        let mut a = candidate("A", 0, 2, 110.);
        a.exit_phase = Phase::Open;
        let mut b = candidate("B", 2, 4, 120.);
        b.entry_phase = Phase::Open;
        b.entry_kind = "open";
        let r = replay(&[a.clone(), b.clone()], &dates(), 1, 0.).unwrap();
        assert_eq!(r.accepted.len(), 2);
        near(r.equity[4], 1.30);
        a.exit_phase = Phase::Close;
        let r = replay(&[a, b], &dates(), 1, 0.).unwrap();
        assert_eq!(r.accepted.len(), 1);
        assert_eq!(r.rejected_capacity, 1);
    }
    #[test]
    fn close_exit_releases_cash_before_close_entry_and_same_day_round_trip_is_closed() {
        let r = replay(
            &[candidate("A", 0, 2, 110.), candidate("B", 2, 4, 120.)],
            &dates(),
            1,
            0.,
        )
        .unwrap();
        assert_eq!(r.accepted.len(), 2);
        let r = replay(&[candidate("A", 0, 0, 100.)], &dates(), 2, 0.001).unwrap();
        near(r.cash[0], 0.999);
        assert_eq!(r.positions[0], 0);
    }
    #[test]
    fn a_missing_held_mark_or_post_source_open_position_withholds_everything() {
        let mut a = candidate("A", 0, 4, 110.);
        a.marks.remove(&dates()[2]);
        assert!(replay(&[a], &dates(), 2, 0.)
            .unwrap_err()
            .contains("no actual closing quote"));
        let mut a = candidate("A", 0, 2, 110.);
        a.open = true;
        assert!(replay(&[a], &dates(), 2, 0.)
            .unwrap_err()
            .contains("2020-01-04"));
    }
    fn short(symbol: &str, entry: usize, exit: usize, px: f64) -> Candidate {
        let mut t = candidate(symbol, entry, exit, px);
        t.dir = -1.;
        t
    }
    #[test]
    fn losses_beyond_partial_and_same_day_slots_are_paid_from_free_cash() {
        let mut a = short("A", 0, 2, 400.);
        a.weight = 0.5;
        let b = short("B", 3, 3, 250.);
        let r = replay(&[a, b], &dates(), 10, 0.).unwrap();
        assert_eq!(r.slot_loss_settlements.len(), 2);
        near(r.slot_loss_settlements[0].cash_before, 0.95);
        near(r.slot_loss_settlements[0].cash_change, -0.1);
        near(r.slot_loss_settlements[1].cash_before, 0.75);
        near(r.slot_loss_settlements[1].cash_change, -0.05);
        near(r.cash[2], 0.85);
        near(r.cash[4], 0.70);
        near(r.equity[4], 0.70);
        near(r.minimum_event_cash, 0.70);
        assert_eq!(r.positions[3], 0);
    }
    #[test]
    fn a_negative_cash_settlement_refuses_with_auditable_amounts() {
        let e = replay(&[short("A", 0, 2, 400.)], &dates(), 1, 0.).unwrap_err();
        assert!(e.starts_with("calculation_unavailable"), "{e}");
        assert!(e.contains("A short closed on 2020-01-03 (close)"), "{e}");
        assert!(e.contains("Cash before 0.000000, change -2.000000, after -2.000000"), "{e}");
        assert!(e.contains("No borrowing"), "{e}");
    }
    #[test]
    fn same_phase_receipts_can_settle_a_loss_but_later_phase_receipts_cannot() {
        // Short loses 1.0 x slot beyond its 0.5 slot; the long's close-phase
        // receipt on the same day pays it.
        let a = short("A", 0, 2, 300.);
        let b = candidate("B", 0, 2, 120.);
        let r = replay(&[a.clone(), b.clone()], &dates(), 2, 0.).unwrap();
        assert_eq!(r.slot_loss_settlements.len(), 1);
        near(r.slot_loss_settlements[0].cash_before, 0.6);
        near(r.cash[2], 0.1);
        assert!(r.minimum_event_cash >= 0.);
        // Same loss settled at the open: the long's close receipt comes later.
        let mut a = a;
        a.exit_phase = Phase::Open;
        let e = replay(&[a, b], &dates(), 2, 0.).unwrap_err();
        assert!(e.contains("(open)") && e.contains("Cash before 0.000000, change -0.500000"), "{e}");
    }
    #[test]
    fn a_negative_open_mark_reduces_equity_without_inventing_or_borrowing_cash() {
        let mut a = short("A", 0, 4, 100.);
        a.marks.insert(dates()[2].clone(), 300.);
        let mut b = candidate("B", 0, 4, 200.);
        b.marks.insert(dates()[2].clone(), 200.);
        let r = replay(&[a, b], &dates(), 4, 0.).unwrap();
        near(r.cash[2], 0.5);
        near(r.equity[2], 0.75);
        assert!(r.slot_loss_settlements.is_empty());
        near(r.minimum_event_cash, 0.5);
        near(r.cash[4], 1.25);
    }
    #[test]
    fn open_marks_charge_entry_only_and_are_excluded_from_closed_profit() {
        let mut a = candidate("A", 0, 4, 120.);
        a.open = true;
        let c = vec![a];
        let d = dates();
        let r = replay(&c, &d, 2, 0.001).unwrap();
        near(r.equity[4], 1.0995);
        let s = portfolio_stats(&c, &r, &d, 0.001, false);
        near(s["net_profit"], 0.);
        near(s["trades"], 0.);
        assert!(s["avg_net_pnl"].is_nan());
    }
    #[test]
    fn closed_contributions_reconcile_equal_initial_slots_and_risk_keeps_first_cost() {
        let c = vec![candidate("A", 0, 3, 120.), candidate("B", 1, 4, 90.)];
        let d = dates();
        let r = replay(&c, &d, 10, 0.001).unwrap();
        let s = portfolio_stats(&c, &r, &d, 0.001, false);
        near(s["avg_net_pnl"], 0.048);
        near(s["closed_return_sum"], 0.096);
        near(s["net_profit"], 0.0096);
        near(s["total_return"], 0.0096);
        assert!(s["max_dd"] < 0.);
    }
    #[test]
    fn input_bounds_and_membership_cannot_be_reinterpreted() {
        let base = BTreeMap::from([
            ("id".into(), "f1a".into()),
            ("basket".into(), "nifty50".into()),
        ]);
        assert_eq!(Request::parse(&base).unwrap().max_positions, 10);
        for (key, value) in [
            ("max_positions", "0"),
            ("max_positions", "101"),
            ("max_positions", "2.5"),
            ("max_positions", "-1"),
            ("membership", "dead"),
            ("from", "2020-02-30"),
            ("p.trend", "NaN"),
            ("basket", "../../data"),
        ] {
            let mut q = base.clone();
            q.insert(key.into(), value.into());
            assert!(Request::parse(&q).is_err(), "{key}={value}");
        }
        let mut q = base;
        q.insert("from".into(), "2020-01-03".into());
        q.insert("to".into(), "2020-01-01".into());
        assert!(Request::parse(&q).is_err());
    }
    #[test]
    fn unsupported_execution_is_held_not_silently_removed() {
        let rows: Vec<_> = dates()
            .into_iter()
            .map(|date| Row {
                ticker: "A".into(),
                date,
                open: 99.,
                high: 110.,
                low: 90.,
                close: 100.,
                volume: Some(100.),
            })
            .collect();
        let ctx = Ctx::new(&rows, &rows, &[]);
        let t = Trade {
            dir: 1.,
            legs: vec![crate::connors::Leg {
                i: 0,
                px: 100.,
                w: 1.,
            }],
            exit_i: 4,
            exit_px: 100.,
            reason: "fixture".into(),
            entry_kind: "close",
            signal_i: 4,
            open: false,
            exits: vec![],
        };
        assert!(Candidate::from_source(
            "basket:nifty50:A",
            &ctx,
            &t,
            (Phase::Close, Phase::Close).into()
        )
        .is_ok());
        let mut x = t.clone();
        x.entry_kind = "limit";
        assert!(Candidate::from_source(
            "basket:nifty50:A",
            &ctx,
            &x,
            (Phase::Close, Phase::Close).into()
        )
        .unwrap_err()
        .contains("intraday"));
        let mut x = t.clone();
        x.legs.push(crate::connors::Leg {
            i: 1,
            px: 100.,
            w: 0.25,
        });
        assert!(Candidate::from_source(
            "basket:nifty50:A",
            &ctx,
            &x,
            (Phase::Close, Phase::Close).into()
        )
        .unwrap_err()
        .contains("scale-in"));
        let mut x = t.clone();
        x.exits.push((2, 102., 0.5));
        assert!(Candidate::from_source(
            "basket:nifty50:A",
            &ctx,
            &x,
            execution_contract("unreviewed", "fixture")
        )
        .unwrap_err()
        .contains("scale-in/scale-out"));
        let mut x = t;
        x.exit_px = 105.;
        assert!(Candidate::from_source(
            "basket:nifty50:A",
            &ctx,
            &x,
            (Phase::Close, Phase::Close).into()
        )
        .unwrap_err()
        .contains("executor contract"));
    }
    #[test]
    fn a_target_equal_to_daily_close_still_has_no_reviewed_portfolio_fill_phase() {
        assert!(reviewed_execution_contract("hbqts_profit_target", "hbqts").is_none());
        assert!(reviewed_execution_contract("elcim_impulse", "elcim").is_none());
        assert!(reviewed_execution_contract("hbqts_limit_entry", "hbqts").is_none());
        assert!(reviewed_execution_contract("hbqts_ma5_25", "another_book").is_none());
        assert!(reviewed_execution_contract("a_new_rule", "hbqts").is_none());
        assert_eq!(
            reviewed_execution_contract("hbqts_ma5_25", "hbqts"),
            Some((Phase::Close, Phase::Close))
        );
    }
    #[test]
    fn conservative_contract_covers_unreviewed_rules_without_relabelling_fills() {
        let contract = execution_contract("ss_the_anti", "ss");
        assert!(contract.conservative);
        assert_eq!(contract.phases, (Phase::Open, Phase::Close));
        let rows: Vec<_> = dates()
            .into_iter()
            .map(|date| Row {
                ticker: "A".into(),
                date,
                open: 99.,
                high: 110.,
                low: 90.,
                close: 100.,
                volume: Some(100.),
            })
            .collect();
        let ctx = Ctx::new(&rows, &rows, &[]);
        for entry_kind in ["close", "open", "stop", "limit"] {
            let mut trade = Trade {
                dir: 1.,
                legs: vec![crate::connors::Leg {
                    i: 0,
                    px: 101.,
                    w: 1.,
                }],
                exit_i: 0,
                exit_px: 105.,
                reason: "target fixture".into(),
                entry_kind,
                signal_i: 0,
                open: false,
                exits: vec![],
            };
            let c = Candidate::from_source("basket:nifty50:A", &ctx, &trade, contract).unwrap();
            assert_eq!((c.entry_phase, c.exit_phase), contract.phases);
            assert_eq!(c.entry_kind, entry_kind);
            near(c.entry_px, 101.);
            near(c.exit_px, 105.);
            let replay = replay(&[c], &dates(), 1, 0.).unwrap();
            assert_eq!(replay.accepted.len(), 1);
            assert_eq!(replay.positions[0], 0);
            near(replay.cash[0], 105. / 101.);
            trade.legs.push(crate::connors::Leg {
                i: 0,
                px: 102.,
                w: 0.25,
            });
            assert!(
                Candidate::from_source("basket:nifty50:A", &ctx, &trade, contract)
                    .unwrap_err()
                    .contains("scale-in/scale-out")
            );
        }
    }
    #[test]
    fn all_reviewed_contracts_keep_their_phases_and_reviewed_flag() {
        for (book, ids, phases) in [
            (
                "stc9",
                vec!["f1a", "f3a", "rsi2_5d"],
                (Phase::Close, Phase::Close),
            ),
            (
                "hbqts",
                vec![
                    "hbqts_ma5_25",
                    "hbqts_ma1_36",
                    "hbqts_fixed_interval",
                    "hbqts_entry_id1",
                    "hbqts_bb_breakout",
                    "hbqts_cgo",
                    "hbqts_stoch_rsi",
                    "hbqts_dpo",
                    "hbqts_dom",
                    "hbqts_macd_daily",
                ],
                (Phase::Close, Phase::Close),
            ),
            ("ch_bta", vec!["ch_cbpb_time"], (Phase::Open, Phase::Close)),
            ("ch_bta", vec!["ch_65sma3cc"], (Phase::Open, Phase::Open)),
            (
                "kftsm",
                vec![
                    "kftsm_ma",
                    "kftsm_exp",
                    "kftsm_lrs",
                    "kftsm_ndb",
                    "kftsm_swg",
                    "kftsm_swing",
                    "kftsm_kama",
                    "kftsm_adapt_exp",
                    "kftsm_vidya",
                    "kftsm_adapt_r2",
                    "kftsm_adapt_rsi",
                    "kftsm_month_end",
                    "kftsm_hirsch",
                ],
                (Phase::Close, Phase::Close),
            ),
        ] {
            for id in ids {
                let contract = execution_contract(id, book);
                assert!(!contract.conservative, "{book}/{id}");
                assert_eq!(contract.phases, phases, "{book}/{id}");
            }
        }
    }
    #[test]
    fn conservative_entry_cannot_spend_cash_released_later_that_day() {
        let mut trades = vec![
            candidate("A", 0, 2, 110.),
            candidate("B", 0, 4, 110.),
            candidate("C", 2, 4, 110.),
        ];
        for t in &mut trades {
            t.entry_phase = Phase::Open;
        }
        let r = replay(&trades, &dates(), 3, 0.001).unwrap();
        assert_eq!(r.accepted.len(), 2);
        assert_eq!(r.rejected_cash, 1);
        assert_eq!(r.rejected_capacity, 0);
        assert!(r.cash.iter().all(|&cash| cash >= 0.));
    }
    #[test]
    fn conservative_cash_and_slots_wait_for_close_even_for_same_day_round_trips() {
        let mut a = candidate("A", 0, 2, 110.);
        let mut b = candidate("B", 2, 2, 120.);
        let mut c = candidate("C", 3, 3, 100.);
        for t in [&mut a, &mut b, &mut c] {
            t.entry_phase = Phase::Open;
            t.exit_phase = Phase::Close;
        }
        let r = replay(&[a, b, c], &dates(), 1, 0.).unwrap();
        assert_eq!(
            r.accepted.iter().map(|a| a.candidate).collect::<Vec<_>>(),
            vec![0, 2]
        );
        assert_eq!(r.rejected_capacity, 1);
        near(r.cash[1], 0.);
        near(r.cash[2], 1.10);
        assert_eq!(r.positions[3], 0);
        assert!(r.cash.iter().all(|&cash| cash >= 0.));
    }
    #[test]
    fn invalid_dates_and_unsupported_funding_are_never_repaired() {
        assert!(replay(&[], &dates(), 0, 0.).is_err());
        assert!(replay(&[], &dates(), 101, 0.).is_err());
        assert!(replay(&[], &dates(), 1, f64::NAN).is_err());
        let mut c = candidate("A", 0, 4, 100.);
        c.entry = "2020-01-06".into();
        assert!(replay(&[c], &dates(), 1, 0.).is_err());
    }
    #[test]
    fn scientific_hold_has_no_zero_performance_claims_and_json_strings_escape() {
        let q = BTreeMap::from([
            ("id".into(), "f1a".into()),
            ("basket".into(), "nifty50".into()),
        ]);
        let req = Request::parse(&q).unwrap();
        let body = hold(&req, "needs_data", "Missing \"quote\"\n", &[]);
        assert!(body.contains("\"stats\":null"));
        assert!(body.contains("\\n"));
        assert!(!body.contains("\"net_profit\":0"));
    }
    static ENDPOINT_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    struct Fixture(std::path::PathBuf);
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    fn fixture() -> (Fixture, State, BTreeMap<String, String>) {
        use std::io::Write;
        let directory = std::env::temp_dir().join(format!(
            "vault-basket-portfolio-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&directory).unwrap();
        let mut csv = String::from("date,open,high,low,close,volume,aux1,aux2\n");
        let mut benchmark = Vec::new();
        for i in 0..300 {
            let date = format!("2020-{:02}-{:02}", i / 28 + 1, i % 28 + 1);
            let c = 100. + 20. * (i as f64 / 7.).sin();
            csv.push_str(&format!("{date},{c},{},{},{c},5000,0,0\n", c + 1., c - 1.));
            benchmark.push(Row {
                ticker: "NIFTY 50".into(),
                date,
                open: 200.,
                high: 201.,
                low: 199.,
                close: 200.,
                volume: Some(5000.),
            });
        }
        std::fs::write(directory.join("stock.csv"), &csv).unwrap();
        let mut process = std::process::Command::new("sha256sum")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        process
            .stdin
            .take()
            .unwrap()
            .write_all(csv.as_bytes())
            .unwrap();
        let output = process.wait_with_output().unwrap();
        assert!(output.status.success());
        let sha = String::from_utf8(output.stdout)
            .unwrap()
            .split_whitespace()
            .next()
            .unwrap()
            .to_string();
        let mut index="basket,ticker,path,first,last,bars,source,source_sha256,csv_sha256,membership,status,has_volume\n".to_string();
        for symbol in ["A", "Z"] {
            index.push_str(&format!("nifty50,{symbol},stock.csv,2020-01-01,2020-11-20,300,Synthetic arithmetic test source,{}, {},Test source-listed current snapshot,ready,true\n","a".repeat(64),sha));
        }
        std::fs::write(directory.join("index.csv"), index).unwrap();
        let rows: &'static [Row] = Box::leak(benchmark.into_boxed_slice());
        let state = State {
            ctxs: BTreeMap::from([("NIFTY 50".into(), Ctx::new(rows, rows, &[]))]),
            baskets: baskets::Catalogue::load(&directory).unwrap(),
            vix: vec![],
            market_extra: Default::default(),
        };
        let q = BTreeMap::from([
            ("_guru".into(), "connors".into()),
            ("id".into(), "hbqts_ma5_25".into()),
            ("basket".into(), "nifty50".into()),
            ("max_positions".into(), "10".into()),
            ("from".into(), "2020-03-01".into()),
            ("to".into(), "2020-11-20".into()),
        ]);
        (Fixture(directory), state, q)
    }
    #[test]
    fn actual_endpoint_uses_stock_ledger_and_never_manufactures_a_price_chart() {
        let _lock = ENDPOINT_LOCK.lock().unwrap();
        let (_fixture, state, q) = fixture();
        let body = run(&state, &q).unwrap();
        assert!(body.contains("\"status\":\"ready\""), "{body}");
        assert!(body.contains("\"symbol\":\"A\""));
        assert!(body.contains("\"symbol\":\"Z\""));
        assert!(body.contains("\"ohlc\":null"));
        assert!(body.contains("\"close\":[]"));
        assert!(body.contains("\"source_members\":2"));
        assert!(body.contains("\"historical_membership_verified\":false"));
        assert!(body.contains("\"compounding_supported\":false"));
        assert!(body.contains("\"benchmark_symbol\":\"NIFTY 50\""));
        assert!(body.contains("\"open_exit_cost_estimated\":false"));
        assert!(!body.contains("Legacy equity includes estimated exit costs"));
    }
    #[test]
    fn actual_endpoint_discloses_only_conservative_reservation_timing() {
        let _lock = ENDPOINT_LOCK.lock().unwrap();
        let (_fixture, state, mut q) = fixture();
        let reviewed = run(&state, &q).unwrap();
        assert!(reviewed.contains("\"timing_contract_reviewed\":true"));
        assert!(reviewed.contains("\"timing_disclosure\":null"));
        q.insert("id".into(), "hbqts_profit_target".into());
        let conservative = run(&state, &q).unwrap();
        assert!(
            conservative.contains("\"status\":\"ready\""),
            "{conservative}"
        );
        assert!(conservative.contains("\"timing_contract_reviewed\":false"));
        assert!(conservative.contains(CONSERVATIVE_TIMING));
        assert!(conservative.contains("\"entry_phase\":\"open\""));
        assert!(conservative.contains("\"exit_phase\":\"close\""));
    }
    #[test]
    fn actual_endpoint_preserves_instrument_readiness_past_and_checksum_guards() {
        let _lock = ENDPOINT_LOCK.lock().unwrap();
        let (fixture, state, mut q) = fixture();
        q.insert("membership".into(), "past".into());
        let body = run(&state, &q).unwrap();
        assert!(body.contains("\"stats\":null"));
        assert!(body.contains("per-date membership"));
        q.insert("membership".into(), "current".into());
        q.insert("id".into(), "dmi_1".into());
        let body = run(&state, &q).unwrap();
        assert!(body.contains("\"stats\":null"));
        assert!(body.contains("Full basket withheld"));
        q.insert("id".into(), "hbqts_ma5_25".into());
        std::fs::write(fixture.0.join("stock.csv"), "tampered").unwrap();
        let body = run(&state, &q).unwrap();
        assert!(body.contains("SHA-256 mismatch"));
        assert!(body.contains("\"stats\":null"));
    }
    #[test]
    fn default_history_intersects_actual_calendar_but_explicit_history_is_never_shortened() {
        let _lock = ENDPOINT_LOCK.lock().unwrap();
        let (_fixture, mut state, mut q) = fixture();
        let rows = state.ctxs.get("NIFTY 50").unwrap().r;
        state
            .ctxs
            .insert("NIFTY 50".into(), Ctx::new(&rows[28..], &rows[28..], &[]));
        q.remove("from");
        q.remove("to");
        let body = run(&state, &q).unwrap();
        assert!(body.contains("\"status\":\"ready\""), "{body}");
        assert!(body.contains("\"effective_from\":\"2020-02-01\""));
        assert!(body.contains("Default dates intersect"));
        q.insert("from".into(), "2020-01-01".into());
        let body = run(&state, &q).unwrap();
        assert!(body.contains("\"stats\":null"));
        assert!(body.contains("calendar coverage"));
    }
    #[test]
    fn a_flat_missing_source_day_uses_disclosed_continuous_suffix() {
        use std::io::Write;
        let _lock = ENDPOINT_LOCK.lock().unwrap();
        let (fixture, state, q) = fixture();
        let mut csv = String::from("date,open,high,low,close,volume,aux1,aux2\n");
        for row in state.ctxs["NIFTY 50"].r {
            if row.date != "2020-06-07" {
                csv.push_str(&format!("{},100,101,99,100,5000,0,0\n", row.date));
            }
        }
        // Constant closes generate no MA crossover; the missing flat bar must
        // move admission after the gap before any trade performance is known.
        let mut process = std::process::Command::new("sha256sum")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        process
            .stdin
            .take()
            .unwrap()
            .write_all(csv.as_bytes())
            .unwrap();
        let output = process.wait_with_output().unwrap();
        assert!(output.status.success());
        let sha = String::from_utf8(output.stdout)
            .unwrap()
            .split_whitespace()
            .next()
            .unwrap()
            .to_string();
        std::fs::write(fixture.0.join("stock.csv"), csv).unwrap();
        let old = std::fs::read_to_string(fixture.0.join("index.csv")).unwrap();
        let old_sha = old
            .lines()
            .nth(1)
            .unwrap()
            .split(',')
            .nth(8)
            .unwrap()
            .trim();
        std::fs::write(
            fixture.0.join("index.csv"),
            old.replace(old_sha, &sha).replace(",300,", ",299,"),
        )
        .unwrap();
        let mut state = state;
        state.baskets = baskets::Catalogue::load(&fixture.0).unwrap();
        let body = run(&state, &q).unwrap();
        assert!(body.contains("2020-06-07"));
        assert!(body.contains("last missing market date"));
        assert!(body.contains("\"status\":\"ready\""), "{body}");
        assert!(body.contains("\"used_from\":\"2020-06-08\""));
    }
    #[test]
    fn named_source_basket_discloses_missing_constituent_and_publishes() {
        let _lock = ENDPOINT_LOCK.lock().unwrap();
        let (fixture, mut state, q) = fixture();
        let old = std::fs::read_to_string(fixture.0.join("index.csv")).unwrap();
        let mut index = format!("{},basket_expected_members\n", old.lines().next().unwrap());
        for row in old.lines().skip(1) {
            index.push_str(&format!("{row},3\n"));
        }
        std::fs::write(fixture.0.join("index.csv"), index).unwrap();
        state.baskets = baskets::Catalogue::load(&fixture.0).unwrap();
        let body = run(&state, &q).unwrap();
        assert!(body.contains("\"status\":\"ready\""), "{body}");
        assert!(body.contains("\"expected_members\":3"));
        assert!(body.contains("\"used_members\":2"));
        assert!(body.contains("\"excluded_members\":1"));
    }
    #[test]
    fn member_window_uses_actual_late_first_and_early_last() {
        let rows: Vec<_> = dates()
            .into_iter()
            .map(|date| Row {
                ticker: "A".into(),
                date,
                open: 1.,
                high: 1.,
                low: 1.,
                close: 1.,
                volume: None,
            })
            .collect();
        let calendar = dates();
        let (start, end, gap) = covered_window(&rows[1..4], &calendar).unwrap();
        assert_eq!((start, end, gap), (0, 2, None));
    }
    #[test]
    fn member_window_uses_suffix_after_last_interior_gap_and_excludes_no_overlap() {
        let calendar = dates();
        let rows: Vec<_> = calendar
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != 1 && *i != 3)
            .map(|(_, date)| Row {
                ticker: "A".into(),
                date: date.clone(),
                open: 1.,
                high: 1.,
                low: 1.,
                close: 1.,
                volume: None,
            })
            .collect();
        assert_eq!(
            covered_window(&rows, &calendar),
            Some((2, 2, Some("2020-01-04".into())))
        );
        assert!(covered_window(&rows, &["1999-01-01".into()]).is_none());
    }
    #[test]
    fn member_bar_outside_benchmark_calendar_uses_common_suffix_before_results() {
        let rows: Vec<_> = dates()
            .into_iter()
            .map(|date| Row {
                ticker: "A".into(),
                date,
                open: 1.,
                high: 1.,
                low: 1.,
                close: 1.,
                volume: None,
            })
            .collect();
        let calendar = vec![
            rows[0].date.clone(),
            rows[1].date.clone(),
            rows[3].date.clone(),
            rows[4].date.clone(),
        ];
        assert_eq!(
            covered_window(&rows, &calendar),
            Some((3, 4, Some(rows[2].date.clone())))
        );
    }
    #[test]
    fn rule_warmup_floor_excludes_short_suffix_before_results() {
        assert_eq!(member_warmup("f1a", &Default::default()), 200);
        assert_eq!(
            member_warmup("f1a", &BTreeMap::from([("trend".into(), 50.)])),
            50
        );
    }
    fn change_first_member(
        fixture: &Fixture,
        state: &mut State,
        edit: impl FnOnce(String) -> String,
    ) {
        use std::io::Write;
        let csv = edit(std::fs::read_to_string(fixture.0.join("stock.csv")).unwrap());
        std::fs::write(fixture.0.join("changed.csv"), &csv).unwrap();
        let mut hash = std::process::Command::new("sha256sum")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        hash.stdin
            .take()
            .unwrap()
            .write_all(csv.as_bytes())
            .unwrap();
        let sha = String::from_utf8(hash.wait_with_output().unwrap().stdout)
            .unwrap()
            .split_whitespace()
            .next()
            .unwrap()
            .to_string();
        let old = std::fs::read_to_string(fixture.0.join("index.csv")).unwrap();
        let index = old
            .lines()
            .map(|line| {
                if line.starts_with("nifty50,A,") {
                    let mut fields: Vec<_> = line.split(',').map(str::to_string).collect();
                    fields[2] = "changed.csv".into();
                    fields[8] = sha.clone();
                    fields.join(",")
                } else {
                    line.into()
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
        std::fs::write(fixture.0.join("index.csv"), index).unwrap();
        state.baskets = baskets::Catalogue::load(&fixture.0).unwrap();
    }
    #[test]
    fn endpoint_uses_actual_first_bar_despite_metadata_mismatch() {
        let _lock = ENDPOINT_LOCK.lock().unwrap();
        let (f, mut st, mut q) = fixture();
        q.remove("from");
        change_first_member(&f, &mut st, |s| {
            s.lines()
                .enumerate()
                .filter(|(i, _)| *i != 1)
                .map(|(_, l)| l)
                .collect::<Vec<_>>()
                .join("\n")
        });
        assert!(st.baskets.rows("basket:nifty50:A").is_err());
        let body = run(&st, &q).unwrap();
        assert!(body.contains("\"status\":\"ready\""), "{body}");
        assert!(body.contains("\"used_from\":\"2020-01-02\""));
        assert!(body.contains("differ from catalogue dates"));
    }
    #[test]
    fn endpoint_excludes_short_suffix_and_missing_file_without_losing_other_member() {
        let _lock = ENDPOINT_LOCK.lock().unwrap();
        let (f, mut st, q) = fixture();
        change_first_member(&f, &mut st, |s| {
            s.lines()
                .filter(|l| !l.starts_with("2020-11-18,"))
                .collect::<Vec<_>>()
                .join("\n")
        });
        let body = run(&st, &q).unwrap();
        assert!(body.contains("\"status\":\"ready\""), "{body}");
        assert!(body.contains("\"used_members\":1"));
        assert!(body.contains("warm-up bars plus 2"));
        std::fs::remove_file(f.0.join("changed.csv")).unwrap();
        let body = run(&st, &q).unwrap();
        assert!(body.contains("\"status\":\"ready\""), "{body}");
        assert!(body.contains("CSV unavailable"));
        assert!(body.contains("\"excluded_members\":1"));
    }
    #[test]
    fn endpoint_auto_starts_volume_before_producing_any_candidates() {
        let _lock = ENDPOINT_LOCK.lock().unwrap();
        let (f, mut st, mut q) = fixture();
        q.remove("from");
        q.insert("id".into(), "eltfl_fi2".into());
        change_first_member(&f, &mut st, |s| {
            s.lines()
                .enumerate()
                .map(|(i, l)| {
                    if i > 0 && i <= 100 {
                        l.replace(",5000,", ",0,")
                    } else {
                        l.into()
                    }
                })
                .collect::<Vec<_>>()
                .join("\n")
        });
        let body = run(&st, &q).unwrap();
        assert!(body.contains("\"status\":\"ready\""), "{body}");
        assert!(body.contains("\"used_from\":\"2020-04-17\""));
        assert!(body.contains("Required volume data"));
    }
    #[test]
    fn endpoint_rule_level_source_review_holds_even_with_other_usable_members() {
        let _lock = ENDPOINT_LOCK.lock().unwrap();
        let (_f, st, mut q) = fixture();
        q.insert("id".into(), "elcim_fi2".into());
        let body = run(&st, &q).unwrap();
        assert!(body.contains("\"stats\":null"));
        assert!(body.contains("Full basket withheld by rule"), "{body}");
    }
    #[test]
    fn endpoint_member_data_end_liquidates_at_actual_close_with_exit_costs() {
        let _lock = ENDPOINT_LOCK.lock().unwrap();
        let (f, mut st, mut q) = fixture();
        q.insert("id".into(), "jb624_mac2_stop".into());
        q.insert("slippage".into(), "10".into());
        change_first_member(&f, &mut st, |s| {
            s.lines().take(201).collect::<Vec<_>>().join("\n")
        });
        let body = run(&st, &q).unwrap();
        assert!(body.contains("\"status\":\"ready\""), "{body}");
        assert!(
            body.contains(
                "\"reason\":\"closed at the last available price (data ends 2020-08-04)\""
            ),
            "{body}"
        );
        assert!(!body.contains("Held A position has no actual closing quote"));
    }
    #[test]
    fn permit_is_fail_fast_and_released_on_every_exit() {
        let _lock = ENDPOINT_LOCK.lock().unwrap();
        let permit = Permit::take().unwrap();
        assert!(Permit::take().is_err());
        drop(permit);
        assert!(Permit::take().is_ok());
    }
}
