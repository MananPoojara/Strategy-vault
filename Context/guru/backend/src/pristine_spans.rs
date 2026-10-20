//! Connect accepted source events to accounting spans. This does not accept
//! diagnostic events from a rejected source run as partial financial results.
use crate::{
    pristine_daily::{Event, Ledger, RunFailure, Setup},
    pristine_report::{EntryPhase, Error, ExitPhase, Span},
};

fn shape(at: usize) -> Error {
    Error {
        at,
        code: "pristine_source_span_shape",
    }
}
fn price(event: &Event) -> Result<f64, Error> {
    event
        .price
        .filter(|p| p.is_finite() && *p > 0. && event.exact_price.is_some())
        .ok_or_else(|| shape(event.i))
}

fn same_setup(a: &Setup, b: &Setup) -> bool {
    let equal = |a: &crate::unit_exposure_arithmetic::ExactValue,
                 b: &crate::unit_exposure_arithmetic::ExactValue| {
        a.signed_cmp(b) == std::cmp::Ordering::Equal
    };
    let optional = |a: &Option<crate::unit_exposure_arithmetic::ExactValue>,
                    b: &Option<crate::unit_exposure_arithmetic::ExactValue>| {
        match (a, b) {
            (None, None) => true,
            (Some(a), Some(b)) => equal(a, b),
            _ => false,
        }
    };
    a.signal_i == b.signal_i
        && optional(&a.trigger, &b.trigger)
        && equal(&a.stop, &b.stop)
        && optional(&a.target, &b.target)
        && equal(&a.signal_close, &b.signal_close)
}

fn audited_order(ledger: &Ledger, event: &Event) -> bool {
    event.entry_i.is_none()
        && event.setup.as_ref().is_some_and(|setup| {
            setup.signal_i == event.signal_i
                && ledger
                    .signals
                    .iter()
                    .any(|signal| signal.flat_permission && same_setup(setup, &signal.setup))
        })
}

/// Only successful source runs can supply accounting spans. Pending, expired
/// and skipped orders never become positions. A final mark ends an OPEN span;
/// its estimated liquidation costs do not create a filled source exit.
pub(crate) fn extract(
    source: &Result<Ledger, RunFailure>,
    from: usize,
    to: usize,
) -> Result<Vec<Span>, Error> {
    let ledger = source.as_ref().map_err(|e| Error {
        at: e.failure.at,
        code: e.failure.code,
    })?;
    if from > to {
        return Err(shape(from));
    }
    let mut spans = Vec::new();
    let mut active: Option<Span> = None;
    let mut pending: Option<usize> = None;
    let mut last_exit = None;
    let mut previous_event = from;
    let (mut terminal_seen, mut pending_seen) = (false, false);
    for e in &ledger.events {
        if e.i < from || e.i > to || e.i < previous_event || terminal_seen || pending_seen {
            return Err(shape(e.i));
        }
        previous_event = e.i;
        if matches!(
            e.kind,
            "order_queued" | "order_skipped" | "order_expired" | "pending_at_end"
        ) && !audited_order(ledger, e)
        {
            return Err(shape(e.i));
        }
        match e.kind {
            "order_queued" => {
                if e.phase != "close"
                    || e.reason != "completed_source_setup"
                    || e.i != e.signal_i
                    || active.is_some()
                    || pending.is_some()
                    || !ledger
                        .signals
                        .iter()
                        .any(|s| s.setup.signal_i == e.signal_i && s.flat_permission)
                {
                    return Err(shape(e.i));
                }
                pending = Some(e.signal_i);
            }
            "order_skipped" => {
                if active.is_some() || e.reason != "no_positive_target_interval" {
                    return Err(shape(e.i));
                }
                match e.phase {
                    "signal_close" if e.i == e.signal_i && pending.is_none() => {}
                    "open" if pending == Some(e.signal_i) && e.i == e.signal_i + 1 => {
                        price(e)?;
                        pending = None;
                    }
                    _ => return Err(shape(e.i)),
                }
            }
            "order_expired" => {
                if active.is_some()
                    || pending != Some(e.signal_i)
                    || e.i != e.signal_i + 1
                    || !matches!(
                        (e.reason, e.phase),
                        ("next_session_trigger_unreached", "close")
                            | ("next_open_gap_below_half", "open")
                    )
                {
                    return Err(shape(e.i));
                }
                pending = None;
            }
            "entry" => {
                if active.is_some()
                    || pending != Some(e.signal_i)
                    || e.signal_i < from
                    || e.i != e.signal_i + 1
                    || e.entry_i != Some(e.i)
                    || last_exit.is_some_and(|i| e.i <= i)
                {
                    return Err(shape(e.i));
                }
                let phase = match (e.reason, e.phase) {
                    ("queued_buy_stop", "open") | ("half_gap_immediate_open", "open") => {
                        EntryPhase::Open
                    }
                    ("queued_buy_stop", "intraday_stop") => EntryPhase::Stop,
                    _ => return Err(shape(e.i)),
                };
                let fill = price(e)?;
                active = Some(Span {
                    signal_i: e.signal_i,
                    entry_i: e.i,
                    exit_i: e.i,
                    entry_price: fill,
                    exit_or_mark_price: fill,
                    open: false,
                    entry_phase: phase,
                    exit_or_mark_phase: ExitPhase::Close,
                });
                pending = None;
            }
            "stop_update" => {
                let p = active.as_ref().ok_or_else(|| shape(e.i))?;
                if e.signal_i != p.signal_i
                    || e.entry_i != Some(p.entry_i)
                    || e.phase != "before_open"
                    || e.reason != "prior_completed_low_after_two_bars"
                    || e.previous_stop.is_none()
                    || e.i < p.entry_i + 2
                {
                    return Err(shape(e.i));
                }
                price(e)?;
            }
            "exit" | "terminal_mark" => {
                let mut p = active.take().ok_or_else(|| shape(e.i))?;
                if e.signal_i != p.signal_i || e.entry_i != Some(p.entry_i) || e.i < p.entry_i {
                    return Err(shape(e.i));
                }
                let phase = match (e.kind, e.reason, e.phase) {
                    ("exit", "protective_at_open", "open") => ExitPhase::OpenStop,
                    ("exit", "target_at_open", "open") => ExitPhase::OpenTarget,
                    ("exit", "cbs_full_gap_at_open", "open") => ExitPhase::OpenFullGap,
                    ("exit", "protective_intraday", "intraday_assumed") => ExitPhase::Stop,
                    ("exit", "target_intraday", "intraday_assumed") => ExitPhase::Target,
                    ("exit", "second_held_day_close", "close") => ExitPhase::Close,
                    ("terminal_mark", "selected_last_close_not_filled_exit", "close")
                        if e.i == to =>
                    {
                        ExitPhase::Mark
                    }
                    _ => return Err(shape(e.i)),
                };
                p.exit_i = e.i;
                p.exit_or_mark_price = price(e)?;
                p.open = e.kind == "terminal_mark";
                p.exit_or_mark_phase = phase;
                terminal_seen = p.open;
                if !p.open {
                    last_exit = Some(e.i);
                }
                spans.push(p);
            }
            "pending_at_end" => {
                if active.is_some()
                    || pending != Some(e.signal_i)
                    || e.i != to
                    || e.i != e.signal_i
                    || e.phase != "close"
                    || e.reason != "next_session_not_in_selected_window"
                {
                    return Err(shape(e.i));
                }
                pending_seen = true;
            }
            _ => return Err(shape(e.i)),
        }
    }
    if active.is_some() || pending.is_some() != pending_seen {
        return Err(shape(to));
    }
    if ledger.pending_order.as_ref().map(|p| p.signal_i) != pending
        || ledger.pending_order.as_ref().is_some_and(|order| {
            !ledger
                .signals
                .iter()
                .any(|signal| signal.flat_permission && same_setup(order, &signal.setup))
        })
    {
        return Err(shape(to));
    }
    match (&ledger.open_position, spans.last().filter(|p| p.open)) {
        (None, None) => {}
        (Some(position), Some(span))
            if terminal_seen
                && position.signal_i == span.signal_i
                && position.entry_i == span.entry_i
                && position.entry_price == span.entry_price => {}
        _ => return Err(shape(to)),
    }
    Ok(spans)
}
