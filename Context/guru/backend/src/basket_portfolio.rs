//! A bounded portfolio adapter over unchanged, admitted single-stock ledgers.
//!
//! This is regular-mode candidate replay, not a reproduction of any author's
//! cross-sectional portfolio. Capacity/cash-rejected candidates are not retried.
//! All prices remain actual observations. Unsupported execution ordering and
//! missing held-position marks withhold the whole portfolio, never selected losers.
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
const MAX_CANDIDATES: usize = 100_000;
const MAX_MARKS: usize = 2_000_000;
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
fn execution_contract(id: &str, book: &str) -> Option<(Phase, Phase)> {
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
}
impl Candidate {
    fn from_source(
        key: &str,
        ctx: &Ctx,
        t: &Trade,
        contract: (Phase, Phase),
    ) -> Result<Self, String> {
        if t.legs.len() != 1 || !t.exits.is_empty() {
            return Err("calculation_unavailable: Basket replay does not yet support scale-in/scale-out event ledgers. No legs are removed or approximated.".into());
        }
        let l = &t.legs[0];
        let entry_phase=match t.entry_kind{"open"=>Phase::Open,"close"=>Phase::Close,_=>return Err("needs_data: Basket stop/limit entries require verified intraday ordering across competing stock signals. Daily OHLC does not order fills; no portfolio path is invented.".into())};
        let equal = |a: f64, b: f64| (a - b).abs() <= 1e-10 * a.abs().max(b.abs()).max(1.);
        if entry_phase != contract.0 {
            return Err("calculation_unavailable: Source entry phase disagrees with the reviewed portfolio executor contract.".into());
        }
        let exit_phase = if t.open { Phase::Close } else { contract.1 };
        let expected_exit = match exit_phase {
            Phase::Open => ctx.o[t.exit_i],
            Phase::Close => ctx.c[t.exit_i],
        };
        if !equal(t.exit_px, expected_exit) {
            return Err("calculation_unavailable: Source exit price disagrees with its reviewed open/close executor contract; an intraday target cannot become a close by approximation.".into());
        }
        if !t.open && contract.1 == Phase::Open && t.signal_i >= t.exit_i {
            return Err("needs_data: Reviewed next-open exit lacks its prior completed-close signal; release timing remains unresolved.".into());
        }
        if entry_phase == Phase::Open && !equal(l.px, ctx.o[l.i])
            || entry_phase == Phase::Close && !equal(l.px, ctx.c[l.i])
        {
            return Err("calculation_unavailable: Entry kind disagrees with its observed open/close fill price.".into());
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
}

fn release_value(t: &Candidate, principal: f64, cost: f64) -> Result<f64, String> {
    let gross = principal * (1. + t.dir * (t.exit_px / t.entry_px - 1.));
    let net = gross - principal * cost;
    if !gross.is_finite() || gross <= 0. || !net.is_finite() || net <= 0. {
        return Err("calculation_unavailable: Closed position exhausts its reserved principal or requires unsupported exit funding/margin. Other slots' cash cannot silently finance the loss; no portfolio result is published.".into());
    }
    Ok(net)
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
    };
    for date in dates {
        let mut exposed = !active.is_empty();
        for phase in [Phase::Open, Phase::Close] {
            let closing: Vec<_> = active
                .keys()
                .copied()
                .filter(|&i| {
                    let t = &candidates[i];
                    !t.open && t.exit == *date && t.exit_phase == phase
                })
                .collect();
            for i in closing {
                let principal = active.remove(&i).unwrap();
                let t = &candidates[i];
                cash += release_value(t, principal, cost)?;
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
                    result.accepted.push(Accepted {
                        candidate: i,
                        allocation: slot,
                    });
                    exposed = true;
                    if !t.open && t.entry == t.exit && t.exit_phase == phase {
                        cash += release_value(t, principal, cost)?;
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
            let value = principal * (1. + t.dir * (mark / t.entry_px - 1.));
            if !value.is_finite() || value <= 0. {
                return Err("calculation_unavailable: Held position exhausts its reserved principal. A supported margin/default model is unavailable; positive cash in other slots cannot silently fund it.".into());
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
}
impl Coverage {
    fn json(&self) -> String {
        format!(
            "{{\"symbol\":{},\"ticker\":{},\"first\":{},\"last\":{},\"status\":{},\"note\":{}}}",
            quote(&self.symbol),
            quote(&self.key),
            quote(&self.first),
            quote(&self.last),
            quote(self.status),
            quote(&self.note)
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

pub fn run(st: &State, q: &BTreeMap<String, String>) -> Result<String, String> {
    let req = Request::parse(q)?;
    let costs = Costs::parse(q)?;
    costs.ensure_rollover()?;
    let catalogue = crate::query_catalogue(q);
    let def = catalogue
        .iter()
        .find(|d| d.id == req.id)
        .ok_or("unknown strategy")?;
    let members = st.baskets.members(req.basket);
    if let Some(expected) = st.baskets.expected_member_count(req.basket) {
        if members.len() != expected {
            return Ok(hold(&req,"needs_data",&format!("The source basket expects {expected} members, but only {} actual current members have admitted price data. The full basket is withheld; missing constituent prices are not silently omitted or replaced.",members.len()),&members.iter().map(|m|coverage(m,"not_processed","Named basket coverage is incomplete; no member was selected for a partial published portfolio.".into())).collect::<Vec<_>>()));
        }
    }
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
    let from = req.from.unwrap_or_else(|| {
        members
            .iter()
            .map(|m| m.first.as_str())
            .min()
            .unwrap()
            .max(benchmark.r[0].date.as_str())
    });
    let to = req.to.unwrap_or_else(|| {
        members
            .iter()
            .map(|m| m.last.as_str())
            .max()
            .unwrap()
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
    for member in &members {
        if started.elapsed() > TIME_LIMIT {
            complete_held_report(&members, &mut report);
            return Ok(hold(&req,"unavailable","Basket request reached its checked 90-second resource budget; no partial result is published.",&report));
        }
        if member.first.as_str() > to || member.last.as_str() < from {
            report.push(coverage(member,"no_coverage","No quotes overlap this requested period; no signals or missing history are invented. This member remains part of the documented source list.".into()));
            continue;
        }
        let mut single = q.clone();
        single.insert("ticker".into(), member.key.clone());
        // Explicit per-member coverage, not a silent rewrite of the basket window.
        single.insert("from".into(), from.max(&member.first).into());
        single.insert("to".into(), to.min(&member.last).into());
        let attempt = (|| -> Result<Vec<Candidate>, String> {
            let selected = crate::select_context(st, &single)?;
            let held = selected.context(st);
            let (ctx, start, end, cost) = crate::resolve_context(&held, &single)?;
            let observed: BTreeSet<_> = ctx.r.iter().map(|r| r.date.as_str()).collect();
            if let Some(date) = calendar.iter().find(|date| {
                date.as_str() >= single["from"].as_str()
                    && date.as_str() <= single["to"].as_str()
                    && !observed.contains(date.as_str())
            }) {
                return Err(format!("needs_data: {} has no actual source bar on covered market date {}. A flat-day gap can remove signals or change indicator/bar-count logic, so no sparse-calendar assumption, missing signal or flat return is invented.",member.symbol,date));
            }
            if let Some(issue) = crate::readiness::check(ctx, def, start, end, &params)
                .or_else(|| crate::native_admission(ctx, def, start, end, &selected, st, &single))
            {
                return Err(format!("{}: {}", issue.status, issue.note));
            }
            let contract=contract.ok_or("needs_data: This exact rule has no reviewed portfolio executor phase contract. Single-stock results remain separate; a stop/target price coinciding with the daily close does not establish cash or slot release timing.")?;
            let start = crate::window_start(ctx, def, start)
                .filter(|&i| i < end)
                .ok_or(
                    "needs_data: Source rule warmup/volatility does not cover this member window",
                )?;
            let source = crate::backtest(ctx, req.id, start, end, cost, &params)?;
            source
                .trades
                .iter()
                .map(|t| Candidate::from_source(&member.key, ctx, t, contract))
                .collect()
        })();
        match attempt {
            Ok(mut ledger) => {
                total_marks += ledger.iter().map(|t| t.marks.len()).sum::<usize>();
                if total_marks > MAX_MARKS || candidates.len() + ledger.len() > MAX_CANDIDATES {
                    complete_held_report(&members, &mut report);
                    return Ok(hold(&req,"unavailable","Basket exceeds its bounded candidate/quote memory budget; no truncated stock or trade list is published.",&report));
                }
                let partial = member.first.as_str() > from || member.last.as_str() < to;
                report.push(coverage(member,if partial{"partial_coverage"}else{"ready"},if partial{format!("Signals use actual source coverage {} to {} within the requested basket period. First/last quotes do not prove listing/delisting dates; held positions still require every actual market-day mark.",member.first,member.last)}else{"Actual source bars and unchanged strategy guards passed; current membership is a snapshot with survivorship bias.".into()}));
                candidates.append(&mut ledger);
            }
            Err(note) => {
                let status = if note.starts_with("calculation_unavailable") {
                    "calculation_unavailable"
                } else if note.starts_with("not_testable") {
                    "not_testable"
                } else {
                    "needs_data"
                };
                report.push(coverage(member, status, note.clone()));
                complete_held_report(&members, &mut report);
                return Ok(hold(&req,status,&format!("Full basket withheld because {} failed source-rule, data or accounting admission: {}. Other members are not cherry-picked into a result.",member.symbol,note),&report));
            }
        }
        // Native SelectedContext and all indicator arrays dropped at this boundary.
    }
    if !report
        .iter()
        .any(|c| matches!(c.status, "ready" | "partial_coverage"))
    {
        return Ok(hold(
            &req,
            "needs_data",
            "No basket member has a usable overlapping price window.",
            &report,
        ));
    }
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
        source_account:None,bandy_account:None,
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
        format!("{{\"symbol\":{},\"ticker\":{},\"dir\":{},\"legs\":[[{},{},{}]],\"entry\":{},\"exit\":{},\"exit_i\":{},\"exit_px\":{},\"avg_px\":{},\"ret\":{},\"bars\":{},\"open\":{},\"reason\":{},\"entry_kind\":{},\"entry_phase\":{},\"exit_phase\":{},\"allocation\":{},\"portfolio_contribution\":{},\"exits\":[],\"accounting\":{{\"symbol\":{},\"signal_side\":{},\"pts\":{},\"days_held\":{},\"gross_ret\":{},\"net_ret\":{},\"slippage_ret\":{},\"fee_ret\":{},\"gross_pnl\":null,\"net_pnl\":null,\"taxes\":null,\"slippage\":null,\"fees\":null,\"mae\":null,\"mfe\":null,\"mae_ret_observed\":null,\"mfe_ret_observed\":null,\"cost_status\":{},\"return_basis\":\"Unchanged source-unit return; portfolio contribution is fixed initial-equity slot times this return. Fractional normalized units, no monetary capital.\"}}}}",quote(&t.symbol),quote(&t.key),t.dir,entry_i,exact(t.entry_px),exact(t.weight),quote(&t.entry),quote(&t.exit),exit_i,exact(t.exit_px),exact(t.entry_px),exact(net),t.bars,t.open,quote(&t.reason),quote(t.entry_kind),quote(t.entry_phase.name()),quote(t.exit_phase.name()),exact(accepted.allocation),exact(accepted.allocation*net),quote(&t.symbol),quote(if t.dir>0.{"Long"}else{"Short"}),exact(t.dir*(t.exit_px-t.entry_px)),crate::day_number(&t.exit)-crate::day_number(&t.entry),exact(gross),exact(net),exact(t.weight*costs.slippage_bps/10000.*if t.open{1.}else{2.}),exact(t.weight*costs.fees_bps/10000.*if t.open{1.}else{2.}),quote(if t.open{"entry_cost_only_actual_open_mark"}else{"modeled_round_trip_cost"}))
    });
    Ok(format!("{{\"kind\":\"basket\",\"status\":\"ready\",\"basket\":{},\"membership\":\"current\",\"model_initial_equity\":1,\"dates\":{},\"equity\":{},\"gross_equity\":{},\"bh\":{},\"close\":[],\"ohlc\":null,\"trades\":{},\"stats\":{},\"gross_stats\":{},\"benchmark_symbol\":{},\"benchmark_stats\":{},\"costs\":{},\"coverage\":{},\"portfolio\":{{\"effective_from\":{},\"effective_to\":{},\"window_basis\":\"Explicit dates are never silently shortened. Default dates intersect admitted member history with the actual benchmark calendar; full earlier stock history is not implied.\",\"max_positions\":{},\"slot_allocation\":{},\"closed_contribution\":{},\"accepted_positions\":{},\"rejected_capacity\":{},\"rejected_cash\":{},\"source_members\":{},\"cash\":{},\"positions\":{},\"historical_membership_verified\":false,\"compounding_supported\":false,\"ranking\":\"Alphabetical symbol order within each observed open/close phase\",\"signal_mode\":\"Regular-mode replay of unchanged independent-symbol candidate ledgers; rejected entries are not deferred or retried\",\"allocation_basis\":\"Each full position reserves 1/N of initial normalized equity; partially deployed source legs use their observed weight. Unused slots remain cash. Fractional research units; no share lot rounding, leverage, margin financing or interest. Short principal is reserved and short proceeds cannot fund entries.\",\"timing_basis\":\"Observed prior-signal next-open exits before open entries; observed close exits before close entries. Intraday stop/limit ordering and multi-leg positions are withheld.\",\"gross_basis\":\"Zero-cost accounting on the identical accepted fills; gross does not independently select a different candidate list\"}},\"notices\":[\"Source-listed current membership is a static snapshot. A current snapshot alone does not establish historical membership; retrospective selection has survivorship bias and this is not a historical membership reproduction.\",\"This is a declared portfolio application of an admitted stock rule, not the author's published portfolio, rankings or performance.\",\"Net Profit is closed allocated contribution relative to initial unit equity. Avg net pnl is the actual source-unit net return per closed trade; multiplying it by trade count gives the unallocated trade-return sum, not basket Net Profit.\",\"Equity and risk statistics use cash plus simultaneously held actual daily marks. Open positions deduct entry costs only; no guessed closing transaction is applied.\"],\"accounting_basis\":\"Fixed initial-equity slots, chronological cash and actual held-position marks. Basket Net Profit excludes open positions. Overlapping stock returns are never compounded in exit order. No capital-related statistics are supplied.\"}}",quote(req.basket),array(calendar.iter().map(|s|quote(s))),array(replay.equity.iter().map(|v|exact(*v))),array(replay.gross_equity.iter().map(|v|exact(*v))),array(bh.iter().map(|v|exact(*v))),array(trade_json),crate::stats_json(&stats),crate::stats_json(&gross_stats),quote(benchmark_symbol),crate::stats_json(&benchmark_stats),portfolio_costs,array(report.iter().map(Coverage::json)),quote(calendar.first().unwrap()),quote(calendar.last().unwrap()),req.max_positions,exact(1./req.max_positions as f64),exact(stats["net_profit"]),replay.accepted.len(),replay.rejected_capacity,replay.rejected_cash,members.len(),array(replay.cash.iter().map(|v|exact(*v))),array(replay.positions.iter().map(|v|v.to_string()))))
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
    fn closed_partial_short_cannot_draw_on_other_slots_after_exhausting_its_collateral() {
        let mut short = candidate("A", 0, 4, 300.);
        short.dir = -1.;
        short.weight = 0.25;
        // The original source-unit return is -50%; that does not authorize a
        // 200% loss on the smaller filled principal plus borrowing other cash.
        near(short.net_return(0.), -0.5);
        assert!(replay(&[short], &dates(), 10, 0.)
            .unwrap_err()
            .contains("reserved principal"));
        let mut same = candidate("A", 0, 0, 300.);
        same.dir = -1.;
        same.weight = 0.25;
        assert!(replay(&[same], &dates(), 10, 0.).is_err());
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
        assert!(
            Candidate::from_source("basket:nifty50:A", &ctx, &t, (Phase::Close, Phase::Close))
                .is_ok()
        );
        let mut x = t.clone();
        x.entry_kind = "limit";
        assert!(
            Candidate::from_source("basket:nifty50:A", &ctx, &x, (Phase::Close, Phase::Close))
                .unwrap_err()
                .contains("intraday")
        );
        let mut x = t.clone();
        x.legs.push(crate::connors::Leg {
            i: 1,
            px: 100.,
            w: 0.25,
        });
        assert!(
            Candidate::from_source("basket:nifty50:A", &ctx, &x, (Phase::Close, Phase::Close))
                .unwrap_err()
                .contains("scale-in")
        );
        let mut x = t;
        x.exit_px = 105.;
        assert!(
            Candidate::from_source("basket:nifty50:A", &ctx, &x, (Phase::Close, Phase::Close))
                .unwrap_err()
                .contains("executor contract")
        );
    }
    #[test]
    fn a_target_equal_to_daily_close_still_has_no_reviewed_portfolio_fill_phase() {
        assert!(execution_contract("hbqts_profit_target", "hbqts").is_none());
        assert!(execution_contract("elcim_impulse", "elcim").is_none());
        assert!(execution_contract("hbqts_limit_entry", "hbqts").is_none());
        assert!(execution_contract("hbqts_ma5_25", "another_book").is_none());
        assert!(execution_contract("a_new_rule", "hbqts").is_none());
        assert_eq!(
            execution_contract("hbqts_ma5_25", "hbqts"),
            Some((Phase::Close, Phase::Close))
        );
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
    fn a_flat_missing_source_day_cannot_silently_remove_signals_from_basket() {
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
        // still hold the portfolio instead of publishing an apparent zero.
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
        assert!(body.contains("flat-day gap"));
        assert!(body.contains("\"stats\":null"));
        assert!(body.contains("\"status\":\"not_processed\""));
    }
    #[test]
    fn named_source_basket_with_missing_constituent_cannot_publish_an_admitted_subset() {
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
        assert!(body.contains("expects 3 members"));
        assert!(body.contains("only 2 actual current members"));
        assert!(body.contains("\"stats\":null"));
        assert!(body.contains("\"trades\":[]"));
        assert!(body.contains("\"status\":\"not_processed\""));
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
