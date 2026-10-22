//! Selected TPM1 daily rule branches. This module is not a data-admission gate:
//! the caller must separately qualify the actual stock, currency and price basis.
//! Source alternatives and research defaults are described in the Pristine review.
use crate::unit_exposure_arithmetic::{Error as ArithmeticError, ExactAudit, ExactValue};
use std::cmp::Ordering;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Method {
    CbsCog,
    BullishGapSurprise,
}

#[derive(Clone, Debug)]
pub(crate) struct Bar {
    pub date: String,
    pub o: f64,
    pub h: f64,
    pub l: f64,
    pub c: f64,
    /// Genuine zero is retained. Missing volume remains missing.
    pub v: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Failure {
    pub at: usize,
    pub code: &'static str,
}

#[derive(Clone, Debug)]
pub(crate) struct Setup {
    pub signal_i: usize,
    pub trigger: Option<ExactValue>,
    pub stop: ExactValue,
    /// CBS fixes its target at the signal SMA; BGS sets it at actual entry+2.
    pub target: Option<ExactValue>,
    pub signal_close: ExactValue,
}

#[derive(Clone, Debug)]
pub(crate) struct Signal {
    pub setup: Setup,
    pub flat_permission: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct Position {
    pub signal_i: usize,
    pub entry_i: usize,
    pub entry_price: f64,
    pub stop: ExactValue,
    pub target: ExactValue,
}

/// A causal event is separate from a filled trade or a hypothetical mark. Exact
/// order evidence survives conversion to the numeric chart/report fill price.
#[derive(Clone, Debug)]
pub(crate) struct Event {
    pub i: usize,
    pub kind: &'static str,
    pub reason: &'static str,
    pub phase: &'static str,
    pub signal_i: usize,
    pub entry_i: Option<usize>,
    pub price: Option<f64>,
    pub exact_price: Option<ExactAudit>,
    pub stop: Option<ExactAudit>,
    pub target: Option<ExactAudit>,
    pub previous_stop: Option<ExactAudit>,
    pub setup: Option<Setup>,
    pub ambiguity: Vec<&'static str>,
}

impl Event {
    fn order(
        i: usize,
        setup: &Setup,
        kind: &'static str,
        reason: &'static str,
        phase: &'static str,
    ) -> Self {
        Self {
            i,
            kind,
            reason,
            phase,
            signal_i: setup.signal_i,
            entry_i: None,
            price: None,
            exact_price: None,
            stop: None,
            target: None,
            previous_stop: None,
            setup: Some(setup.clone()),
            ambiguity: vec![],
        }
    }
    fn position(
        i: usize,
        p: &Position,
        kind: &'static str,
        reason: &'static str,
        phase: &'static str,
        price: Option<f64>,
        exact_price: Option<ExactAudit>,
    ) -> Self {
        Self {
            i,
            kind,
            reason,
            phase,
            signal_i: p.signal_i,
            entry_i: Some(p.entry_i),
            price,
            exact_price,
            stop: Some(p.stop.audit()),
            target: Some(p.target.audit()),
            previous_stop: None,
            setup: None,
            ambiguity: vec![],
        }
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct Ledger {
    pub signals: Vec<Signal>,
    pub events: Vec<Event>,
    pub open_position: Option<Position>,
    pub pending_order: Option<Setup>,
}

#[derive(Clone, Debug)]
pub(crate) struct RunFailure {
    pub failure: Failure,
    /// Diagnostic events are not a financial payload or a partial backtest.
    pub diagnostics: Ledger,
}

fn arithmetic<T>(at: usize, value: Result<T, ArithmeticError>) -> Result<T, Failure> {
    value.map_err(|error| Failure {
        at,
        code: error.code(),
    })
}
fn exact(at: usize, value: f64) -> Result<ExactValue, Failure> {
    arithmetic(at, ExactValue::canonical(value))
}
fn positive_level(at: usize, value: &ExactValue) -> Result<f64, Failure> {
    if !value.positive() {
        return Err(Failure {
            at,
            code: "arithmetic_nonpositive_level",
        });
    }
    arithmetic(at, value.rounded())
}

/// Validate the genuine price prefix without treating unused volume as a
/// dependency. No sorting, deduplication, filling or price repair occurs here.
pub(crate) fn validate(bars: &[Bar], to: usize) -> Result<(), Failure> {
    if to >= bars.len() {
        return Err(Failure {
            at: to,
            code: "invalid_range",
        });
    }
    for (i, bar) in bars[..=to].iter().enumerate() {
        if !crate::baskets::valid_date(&bar.date)
            || i > 0 && bars[i - 1].date >= bar.date
            || ![bar.o, bar.h, bar.l, bar.c]
                .iter()
                .all(|p| p.is_finite() && *p > 0.)
            || bar.l > bar.h
            || bar.o < bar.l
            || bar.o > bar.h
            || bar.c < bar.l
            || bar.c > bar.h
        {
            return Err(Failure {
                at: i,
                code: "source_invalid_OHLC",
            });
        }
    }
    Ok(())
}

/// The caller validates the price prefix once, then evaluates completed selected
/// sessions. Position permission does not change the source setup calculation.
pub(crate) fn setup(
    bars: &[Bar],
    i: usize,
    method: Method,
    preferred_price: bool,
) -> Result<Option<Setup>, Failure> {
    if i >= bars.len() {
        return Err(Failure {
            at: i,
            code: "invalid_range",
        });
    }
    let b = &bars[i];
    let close = exact(i, b.c)?;
    let offset = arithmetic(i, ExactValue::ratio(1, 20))?;
    let red = |k: usize| bars[k].c < bars[k].o;
    match method {
        Method::CbsCog => {
            if i < 19 || b.c <= b.o || !(i - 5..i).all(red) || i > 5 && red(i - 6) {
                return Ok(None);
            }
            let closes = bars[i - 19..=i].iter().map(|bar| bar.c).collect::<Vec<_>>();
            let mean = arithmetic(i, ExactValue::mean20(&closes))?;
            if mean.sub(&close).scale(100).signed_cmp(&mean.scale(8)) == Ordering::Less {
                return Ok(None);
            }
            let trigger = exact(i, b.h)?.add(&offset);
            let stop = exact(i, b.l)?.sub(&offset);
            for level in [&trigger, &stop, &mean] {
                positive_level(i, level)?;
            }
            Ok(Some(Setup {
                signal_i: i,
                trigger: Some(trigger),
                stop,
                target: Some(mean),
                signal_close: close,
            }))
        }
        Method::BullishGapSurprise => {
            if i < 20 || !red(i - 1) || !red(i) {
                return Ok(None);
            }
            if preferred_price
                && close.signed_cmp(&arithmetic(i, ExactValue::ratio(35, 1))?) != Ordering::Greater
            {
                return Ok(None);
            }
            let high = exact(i, b.h)?;
            let low = exact(i, b.l)?;
            let range = high.sub(&low);
            if !range.positive()
                || range.signed_cmp(&arithmetic(i, ExactValue::ratio(7, 4))?) == Ordering::Less
            {
                return Ok(None);
            }
            let fifth = arithmetic(i, range.divide(5))?;
            if exact(i, b.o)?.signed_cmp(&high.sub(&fifth)) == Ordering::Less
                || close.signed_cmp(&low.add(&fifth)) == Ordering::Greater
            {
                return Ok(None);
            }

            // Only a completed price candidate requires this local21-value
            // window. Older unused or noncandidate-session volume is irrelevant.
            let volumes = bars[i - 20..=i]
                .iter()
                .map(|bar| match bar.v {
                    None => Err(Failure {
                        at: i,
                        code: "source_volume_missing",
                    }),
                    Some(v) if !v.is_finite() => Err(Failure {
                        at: i,
                        code: "source_volume_nonfinite",
                    }),
                    Some(v) if v < 0. => Err(Failure {
                        at: i,
                        code: "source_volume_negative",
                    }),
                    Some(v) => Ok(v),
                })
                .collect::<Result<Vec<_>, _>>()?;
            let mean = arithmetic(i, ExactValue::mean20(&volumes[..20]))?;
            if exact(i, volumes[20])?.signed_cmp(&mean) != Ordering::Greater {
                return Ok(None);
            }
            let stop = low.sub(&offset);
            positive_level(i, &stop)?;
            Ok(Some(Setup {
                signal_i: i,
                trigger: None,
                stop,
                target: None,
                signal_close: close,
            }))
        }
    }
}

/// Order comparisons retain exact levels; accounting uses the authoritative
/// executed binary64 fill. A collapsed initial bracket is unsupported, not fixed
/// by moving a stop/target or adding an epsilon. Later trails may exceed entry.
fn initial_bracket(
    at: usize,
    entry: f64,
    stop: &ExactValue,
    target: &ExactValue,
) -> Result<(), Failure> {
    let entry_exact = exact(at, entry)?;
    let stop_px = positive_level(at, stop)?;
    let target_px = positive_level(at, target)?;
    if stop.signed_cmp(&entry_exact) != Ordering::Less
        || target.signed_cmp(&entry_exact) != Ordering::Greater
    {
        return Err(Failure {
            at,
            code: "level_precision",
        });
    }
    if stop_px >= entry || target_px <= entry {
        return Err(Failure {
            at,
            code: "level_precision",
        });
    }
    Ok(())
}

fn price_exit(
    bars: &[Bar],
    i: usize,
    method: Method,
    p: &Position,
    entry_at_breakout: bool,
) -> Result<Option<Event>, Failure> {
    let b = &bars[i];
    let open = exact(i, b.o)?;
    // An opening observation has known priority. For an intraday breakout
    // entry, that opening occurred before the position existed.
    if !entry_at_breakout {
        let reason = if open.signed_cmp(&p.stop) != Ordering::Greater {
            Some("protective_at_open")
        } else if open.signed_cmp(&p.target) != Ordering::Less {
            Some("target_at_open")
        } else if method == Method::CbsCog
            && i - p.entry_i >= 2
            && open.signed_cmp(&exact(i, bars[i - 1].h)?) == Ordering::Greater
        {
            Some("cbs_full_gap_at_open")
        } else {
            None
        };
        if let Some(reason) = reason {
            return Ok(Some(Event::position(
                i,
                p,
                "exit",
                reason,
                "open",
                Some(b.o),
                Some(open.audit()),
            )));
        }
    }
    let stop_hit = exact(i, b.l)?.signed_cmp(&p.stop) != Ordering::Greater;
    let target_hit = exact(i, b.h)?.signed_cmp(&p.target) != Ordering::Less;
    let (level, reason, phase) = if stop_hit {
        (&p.stop, "protective_intraday", "intraday_assumed")
    } else if target_hit {
        (&p.target, "target_intraday", "intraday_assumed")
    } else {
        if method == Method::BullishGapSurprise && i > p.entry_i {
            return Ok(Some(Event::position(
                i,
                p,
                "exit",
                "second_held_day_close",
                "close",
                Some(b.c),
                Some(exact(i, b.c)?.audit()),
            )));
        }
        return Ok(None);
    };
    let mut event = Event::position(
        i,
        p,
        "exit",
        reason,
        phase,
        Some(positive_level(i, level)?),
        Some(level.audit()),
    );
    if stop_hit && entry_at_breakout {
        event
            .ambiguity
            .push("entry_day_low_may_precede_breakout_conservative_stop_assumed");
    }
    if stop_hit && target_hit {
        event
            .ambiguity
            .push("daily_stop_target_order_unseen_stop_first");
    }
    Ok(Some(event))
}

fn run_inner(
    bars: &[Bar],
    from: usize,
    to: usize,
    method: Method,
    preferred_price: bool,
    ledger: &mut Ledger,
) -> Result<(), Failure> {
    if from > to {
        return Err(Failure {
            at: from,
            code: "invalid_range",
        });
    }
    validate(bars, to)?;
    let offset = arithmetic(from, ExactValue::ratio(1, 20))?;
    let half = arithmetic(from, ExactValue::ratio(1, 2))?;
    let two = arithmetic(from, ExactValue::ratio(2, 1))?;
    for i in from..=to {
        let b = &bars[i];
        if let Some(mut p) = ledger.open_position.clone() {
            if method == Method::CbsCog && i - p.entry_i >= 2 {
                let candidate = exact(i, bars[i - 1].l)?.sub(&offset);
                if candidate.signed_cmp(&p.stop) == Ordering::Greater {
                    // A lower candidate, including a nonpositive one, cannot
                    // loosen or invalidate the already valid protective stop.
                    positive_level(i, &candidate)?;
                    let previous = p.stop.audit();
                    p.stop = candidate;
                    let mut event = Event::position(
                        i,
                        &p,
                        "stop_update",
                        "prior_completed_low_after_two_bars",
                        "before_open",
                        Some(positive_level(i, &p.stop)?),
                        Some(p.stop.audit()),
                    );
                    event.previous_stop = Some(previous);
                    ledger.events.push(event);
                }
            }
            if let Some(exit) = price_exit(bars, i, method, &p, false)? {
                ledger.events.push(exit);
                ledger.open_position = None;
            } else {
                ledger.open_position = Some(p);
            }
        } else if let Some(order) = ledger.pending_order.take() {
            debug_assert_eq!(order.signal_i + 1, i);
            let open = exact(i, b.o)?;
            let (entry_price, exact_entry, phase) = match method {
                Method::CbsCog => {
                    let trigger = order.trigger.as_ref().expect("CBS setup trigger");
                    let target = order.target.as_ref().expect("CBS setup target");
                    if open.signed_cmp(trigger) != Ordering::Less {
                        if open.signed_cmp(target) != Ordering::Less {
                            let mut skipped = Event::order(
                                i,
                                &order,
                                "order_skipped",
                                "no_positive_target_interval",
                                "open",
                            );
                            skipped.price = Some(b.o);
                            skipped.exact_price = Some(open.audit());
                            ledger.events.push(skipped);
                            (None, None, "open")
                        } else {
                            (Some(b.o), Some(open.audit()), "open")
                        }
                    } else if exact(i, b.h)?.signed_cmp(trigger) != Ordering::Less {
                        (
                            Some(positive_level(i, trigger)?),
                            Some(trigger.audit()),
                            "intraday_stop",
                        )
                    } else {
                        ledger.events.push(Event::order(
                            i,
                            &order,
                            "order_expired",
                            "next_session_trigger_unreached",
                            "close",
                        ));
                        (None, None, "close")
                    }
                }
                Method::BullishGapSurprise => {
                    if open.sub(&order.signal_close).signed_cmp(&half) != Ordering::Less {
                        (Some(b.o), Some(open.audit()), "open")
                    } else {
                        ledger.events.push(Event::order(
                            i,
                            &order,
                            "order_expired",
                            "next_open_gap_below_half",
                            "open",
                        ));
                        (None, None, "open")
                    }
                }
            };
            if let Some(price) = entry_price {
                let target = match &order.target {
                    Some(target) => target.clone(),
                    None => exact(i, price)?.add(&two),
                };
                initial_bracket(i, price, &order.stop, &target)?;
                let p = Position {
                    signal_i: order.signal_i,
                    entry_i: i,
                    entry_price: price,
                    stop: order.stop,
                    target,
                };
                ledger.events.push(Event::position(
                    i,
                    &p,
                    "entry",
                    if method == Method::CbsCog {
                        "queued_buy_stop"
                    } else {
                        "half_gap_immediate_open"
                    },
                    phase,
                    Some(price),
                    exact_entry,
                ));
                if let Some(exit) = price_exit(bars, i, method, &p, phase == "intraday_stop")? {
                    ledger.events.push(exit);
                } else {
                    ledger.open_position = Some(p);
                }
            }
        }

        // Source audits use the completed close even while held. A same-day
        // exit permits a new next-session order, never a same-session re-entry.
        if let Some(candidate) = setup(bars, i, method, preferred_price)? {
            let flat = ledger.open_position.is_none();
            ledger.signals.push(Signal {
                setup: candidate.clone(),
                flat_permission: flat,
            });
            if flat {
                let no_target = candidate
                    .trigger
                    .as_ref()
                    .zip(candidate.target.as_ref())
                    .is_some_and(|(trigger, target)| trigger.signed_cmp(target) != Ordering::Less);
                if no_target {
                    ledger.events.push(Event::order(
                        i,
                        &candidate,
                        "order_skipped",
                        "no_positive_target_interval",
                        "signal_close",
                    ));
                } else {
                    ledger.events.push(Event::order(
                        i,
                        &candidate,
                        "order_queued",
                        "completed_source_setup",
                        "close",
                    ));
                    ledger.pending_order = Some(candidate);
                }
            }
        }
    }
    if let Some(p) = &ledger.open_position {
        ledger.events.push(Event::position(
            to,
            p,
            "terminal_mark",
            "selected_last_close_not_filled_exit",
            "close",
            Some(bars[to].c),
            Some(exact(to, bars[to].c)?.audit()),
        ));
    }
    if let Some(order) = &ledger.pending_order {
        ledger.events.push(Event::order(
            to,
            order,
            "pending_at_end",
            "next_session_not_in_selected_window",
            "close",
        ));
    }
    Ok(())
}

/// One genuine selected session is supported. Prices use the genuine prefix;
/// selected positions/orders begin flat and cannot inherit a pre-window signal.
/// This event study has no monetary, NAV or verified-market claim by itself.
pub(crate) fn run(
    bars: &[Bar],
    from: usize,
    to: usize,
    method: Method,
    preferred_price: bool,
) -> Result<Ledger, RunFailure> {
    let mut ledger = Ledger::default();
    match run_inner(bars, from, to, method, preferred_price, &mut ledger) {
        Ok(()) => Ok(ledger),
        Err(failure) => Err(RunFailure {
            failure,
            diagnostics: ledger,
        }),
    }
}
