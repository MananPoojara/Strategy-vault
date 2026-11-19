//! Isolated Star/Navarte price-excess research candidate.
//! Source and two declared execution/cross readings: STAR-SOURCE-READY.md.
//! Narrow NIFTY 50 / SPY candidate only; independent API review pending.
use crate::connors::{Ctx, Leg, Trade};

pub const ID: &str = "bstar_price_excess_cross_24_12";

#[derive(Debug, PartialEq)]
pub struct Signals {
    pub buy: Vec<bool>,
    pub sell: Vec<bool>,
}

fn crosses(previous: i128, current: i128) -> (bool, bool) {
    (current > 0 && previous <= 0, current < 0 && previous >= 0)
}

/// Reconstruct shortest round-trip decimal prices from the already-loaded f64.
/// This preserves those decimal values exactly; it cannot recover precision
/// discarded by the upstream parser. Reject excessive scale/overflow.
fn decimal_closes(values: &[f64]) -> Result<Vec<i128>, String> {
    let parts: Vec<(i128, u32)> = values
        .iter()
        .map(|v| {
            let s = v.to_string();
            let (whole, fraction) = s.split_once('.').unwrap_or((&s, ""));
            if fraction.len() > 12
                || !whole.bytes().all(|b| b.is_ascii_digit())
                || !fraction.bytes().all(|b| b.is_ascii_digit())
            {
                return Err("close precision exceeds candidate profile".to_string());
            }
            let digits = format!("{whole}{fraction}");
            let value = digits
                .parse::<i128>()
                .map_err(|_| "close decimal overflow".to_string())?;
            Ok((value, fraction.len() as u32))
        })
        .collect::<Result<_, String>>()?;
    let scale = parts.iter().map(|(_, p)| *p).max().unwrap_or(0);
    parts
        .into_iter()
        .map(|(n, p)| {
            n.checked_mul(10_i128.pow(scale - p))
                .ok_or_else(|| "close decimal overflow".to_string())
        })
        .collect()
}

fn checked_sum(values: &[i128]) -> Result<i128, String> {
    values.iter().try_fold(0_i128, |a, v| {
        a.checked_add(*v)
            .ok_or_else(|| "indicator decimal overflow".to_string())
    })
}

/// Indicators use the actual prefix through `to`, including the last 24 closes
/// and the last 12 defined oscillator values. No values after `to` are read.
pub fn signals(x: &Ctx, from: usize, to: usize) -> Result<Signals, String> {
    if from < 35 || from > to || to >= x.r.len() || to >= x.c.len() {
        return Err("invalid window or insufficient actual prefix".into());
    }
    let rows = &x.r[..=to];
    if rows.iter().any(|r| {
        !crate::baskets::valid_date(&r.date)
            || !r.close.is_finite()
            || r.close <= 0.0
            || r.ticker != rows[0].ticker
    }) || rows.windows(2).any(|p| p[0].date >= p[1].date)
        || rows.iter().zip(&x.c).any(|(r, c)| r.close != *c)
    {
        return Err("invalid or unordered close prefix".into());
    }

    let close = decimal_closes(&x.c[..=to])?;
    // osc24 is 24 * E in common-scale integer units. The crossing
    // difference is 288 * (E - SMA12(E)); its sign is exact.
    let mut osc24 = vec![0_i128; to + 1];
    for i in 23..=to {
        osc24[i] = close[i]
            .checked_mul(24)
            .and_then(|v| v.checked_sub(checked_sum(&close[i - 23..=i]).ok()?))
            .ok_or("indicator decimal overflow")?;
    }
    let mut delta = vec![0_i128; to + 1];
    for i in 34..=to {
        delta[i] = osc24[i]
            .checked_mul(12)
            .and_then(|v| v.checked_sub(checked_sum(&osc24[i - 11..=i]).ok()?))
            .ok_or("indicator decimal overflow")?;
    }
    let (mut buy, mut sell) = (vec![false; to + 1], vec![false; to + 1]);
    for i in from..=to {
        (buy[i], sell[i]) = crosses(delta[i - 1], delta[i]);
    }

    Ok(Signals { buy, sell })
}

/// Long/flat, start flat at `from`, close fills on the signal bar. A position
/// entered on the final bar is returned as an open mark, not discarded.
fn ledger(close: &[f64], from: usize, to: usize, s: &Signals) -> Vec<Trade> {
    let mut out = Vec::new();
    let mut entry: Option<usize> = None;
    for i in from..=to {
        if let Some(e) = entry {
            if s.sell[i] {
                out.push(trade(close, e, i, false));
                entry = None;
            }
        } else if s.buy[i] {
            entry = Some(i);
        }
    }
    if let Some(e) = entry {
        out.push(trade(close, e, to, true));
    }
    out
}

fn trade(close: &[f64], entry: usize, exit: usize, open: bool) -> Trade {
    Trade {
        dir: 1.0,
        legs: vec![Leg {
            i: entry,
            px: close[entry],
            w: 1.0,
        }],
        exit_i: exit,
        exit_px: close[exit],
        reason: if open {
            "open"
        } else {
            "oscillator crossed below average"
        }
        .into(),
        entry_kind: "close",
        signal_i: exit,
        open,
        exits: vec![],
    }
}

pub fn candidate_run(x: &Ctx, from: usize, to: usize) -> Result<Vec<Trade>, String> {
    let s = signals(x, from, to)?;
    Ok(ledger(&x.c, from, to, &s))
}

// Candidate catalogue for isolated review; no global publication is implied.
pub const BOOK: &str = "bstar";
const NOTE: &str = "Star, Oscillator Variations, Stocks & Commodities 11:4 pp.174–177, PDF pp.1–2: close minus 24-day simple close average and its 12-day simple average; the NYSE Composite chart has no labeled trades. Navarte, Reading between the lines, 3:6 pp.204–206, PDF pp.1,4, discusses buying lows, selling highs and confirmation by a cross: upward-buy/downward-sell long/cash is the lead-approved inference (R4 #357). Unstated: prior equality is accepted while current cross is strict; fills use the signal close with zero delay. Computation uses checked decimal arithmetic on reconstructed runtime prices, refusing unsupported precision or overflow; exact historical MetaStock output is unverified. The actual supplied prefix initializes indicators, the selected interval starts flat, and an end-bar entry stays open. A cash-index simulation is not a stock or ETF trade. No source performance or independent tested-binary ledger comparison is claimed.";
pub fn catalogue() -> Vec<crate::connors::StrategyDef> {
    vec![crate::connors::StrategyDef { id: ID, name: "Price-excess oscillator crossing its average (24/12, long/cash)", group: "Star Oscillator Variations", session: 0, dir: "Long", status: crate::connors::Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
        rules: &["F|Price excess = close minus its 24-day simple moving average; trigger = 12-day simple average of price excess.", "E|Upward cross of price excess above the trigger: buy (inference from Navarte's low-buy crossing confirmation).", "X|Downward cross below the trigger: exit long (inference from high-sell crossing confirmation)."], notes: NOTE, claim: None }]
}
pub fn params(id: &str) -> Option<Vec<crate::connors::ParamDef>> {
    (id == ID).then(Vec::new)
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == ID).then_some((ID, "Star / Navarte price-excess crossing", "24 / 12"))
}
pub fn reviewed(id: &str) -> bool {
    id == ID
}

pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |d| d.book == BOOK && reviewed(d.id),
    label: "Star and Navarte show an NYSE Composite chart; NIFTY 50 index and SPY ETF are declared, unverified research applications. NIFTY is a cash-index simulation, not a stock or ETF trade.",
    basis: "Star Oscillator Variations PDF pp.1-2; Navarte Reading between the lines PDF pp.1,4; R4 #357. Independently reviewed numeric signals; independent API ledger review remains pending.",
};

pub fn run(
    id: &str,
    x: &Ctx,
    from: usize,
    to: usize,
    p: &crate::connors::Params,
) -> Option<Vec<Trade>> {
    if !reviewed(id)
        || !p.is_empty()
        || x.native_market.is_some()
        || from >= to
        || !x
            .r
            .first()
            .is_some_and(|r| matches!(r.ticker.as_str(), "NIFTY 50" | "SPY"))
    {
        return None;
    }
    candidate_run(x, from, to).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;

    // Mathematical unit fixtures only; these are not market observations.
    fn rows(n: usize) -> Vec<Row> {
        (0..n)
            .map(|i| {
                let c = 100.0 + (i % 17) as f64;
                Row {
                    ticker: "fixture".into(),
                    date: format!("2020-{:02}-{:02}", i / 25 + 1, i % 25 + 1),
                    open: c,
                    high: c + 1.0,
                    low: c - 1.0,
                    close: c,
                    volume: None,
                }
            })
            .collect()
    }

    #[test]
    fn candidate_route_admits_only_declared_symbols_and_parameters() {
        let defs = catalogue();
        assert_eq!(defs.len(), 1);
        assert!(matches!(defs[0].status, crate::connors::Status::Live));
        let mut r = rows(40);
        for v in &mut r {
            v.ticker = "NIFTY 50".into();
        }
        let x = Ctx::new(&r, &r, &[]);
        assert!(run(ID, &x, 35, 39, &crate::connors::Params::new()).is_some());
        assert!(run(ID, &x, 34, 39, &crate::connors::Params::new()).is_none());
        assert!(run(ID, &x, 35, 39, &[("side".into(), 1.)].into_iter().collect()).is_none());
        assert!(run("bstar_unknown", &x, 35, 39, &crate::connors::Params::new()).is_none());
        for v in &mut r {
            v.ticker = "SPX".into();
        }
        let x = Ctx::new(&r, &r, &[]);
        assert!(run(ID, &x, 35, 39, &crate::connors::Params::new()).is_none());
    }

    #[test]
    fn crosses_accept_prior_equality_but_not_current_equality() {
        assert_eq!(crosses(-1, 1), (true, false));
        assert_eq!(crosses(1, -1), (false, true));
        assert_eq!(crosses(0, 1), (true, false));
        assert_eq!(crosses(0, -1), (false, true));
        assert_eq!(crosses(-1, 0), (false, false));
    }

    #[test]
    fn warmup_prefix_and_no_future_access() {
        let r = rows(55);
        let x = Ctx::new(&r, &r, &[]);
        assert!(signals(&x, 34, 48).is_err());
        let a = signals(&x, 35, 48).unwrap();
        let mut later = r.clone();
        later[52].close = f64::NAN;
        let y = Ctx::new(&later, &later, &[]);
        assert_eq!(a, signals(&y, 35, 48).unwrap());
        assert!(signals(&y, 35, 54).is_err());
    }

    #[test]
    fn invalid_and_out_of_order_prefix_rejected() {
        let mut r = rows(50);
        r[12].date = r[11].date.clone();
        let x = Ctx::new(&r, &r, &[]);
        assert!(signals(&x, 35, 49).is_err());
        r[12].date = "2020-01-13".into();
        r[15].close = 0.;
        let x = Ctx::new(&r, &r, &[]);
        assert!(signals(&x, 35, 49).is_err());
    }

    #[test]
    fn flat_start_redundancy_and_terminal_entry() {
        let c = vec![100., 101., 102., 103., 104., 105.];
        let s = Signals {
            buy: vec![true, false, true, true, false, true],
            sell: vec![false, true, false, false, true, false],
        };
        let t = ledger(&c, 2, 5, &s);
        assert_eq!(t.len(), 2);
        assert_eq!(t[0].legs[0].i, 2);
        assert_eq!(t[0].exit_i, 4);
        assert!(!t[0].open);
        assert_eq!(t[1].legs[0].i, 5);
        assert_eq!(t[1].exit_i, 5);
        assert!(t[1].open);
    }

    #[test]
    fn linear_decimal_series_has_exact_equality_without_false_crosses() {
        let mut r = rows(100);
        for (i, v) in r.iter_mut().enumerate() {
            v.close = format!("100.{:02}", i).parse().unwrap();
        }
        let x = Ctx::new(&r, &r, &[]);
        let s = signals(&x, 35, 99).unwrap();
        assert!(!s.buy.iter().any(|v| *v));
        assert!(!s.sell.iter().any(|v| *v));
    }

    #[test]
    fn tiny_nonzero_crosses_are_not_rounded_away() {
        let mut r = rows(40);
        for v in &mut r {
            v.close = 100.0;
        }
        r[35].close = 100.000000001;
        r[36].close = 99.999999999;
        let x = Ctx::new(&r, &r, &[]);
        let s = signals(&x, 35, 36).unwrap();
        assert!(s.buy[35] && !s.sell[35]);
        assert!(s.sell[36] && !s.buy[36]);
    }

    #[test]
    fn first_defined_prior_equality_can_cross_and_reverse() {
        let mut r = rows(40);
        for v in &mut r {
            v.close = 100.;
        }
        r[35].close = 112.;
        r[36].close = 88.;
        let x = Ctx::new(&r, &r, &[]);
        let s = signals(&x, 35, 36).unwrap();
        assert!(s.buy[35] && !s.sell[35]);
        assert!(s.sell[36] && !s.buy[36]);
        let t = candidate_run(&x, 35, 36).unwrap();
        assert_eq!(t.len(), 1);
        assert_eq!(t[0].legs[0].i, 35);
        assert_eq!(t[0].exit_i, 36);
        assert!(!t[0].open);
    }

    /// Opt-in diagnostic emits source-prefix signals and trades for an
    /// independent file comparison. It streams one actual ticker from one CSV.
    #[test]
    #[ignore = "requires explicit actual CSV, ticker and date window"]
    fn emit_actual_reference_comparison() {
        use std::io::{BufRead, BufReader};
        let env = |k| std::env::var(k).expect(k);
        let path = env("STAR_CSV");
        let ticker = env("STAR_TICKER");
        let first = env("STAR_HISTORY_FROM");
        let start = env("STAR_SELECTED_FROM");
        let end = env("STAR_TO");
        let mut rows = Vec::new();
        for line in BufReader::new(std::fs::File::open(path).unwrap())
            .lines()
            .skip(1)
        {
            let line = line.unwrap();
            let f = crate::parse_csv_fields(&line);
            if f.first().map(String::as_str) != Some(ticker.as_str()) {
                continue;
            }
            assert!(f.len() >= 6, "incomplete input row");
            let date = crate::normalize_date(&f[1]).expect("valid date");
            if date < first || date > end {
                continue;
            }
            let number = |i: usize| crate::parse_csv_number(f[i].as_str()).expect("valid price");
            rows.push(Row {
                ticker: ticker.clone(),
                date,
                open: number(2),
                high: number(3),
                low: number(4),
                close: number(5),
                volume: None,
            });
        }
        let begin = rows
            .iter()
            .position(|r| r.date >= start)
            .expect("selected start");
        let to = rows.len().checked_sub(1).expect("actual input");
        let x = Ctx::new(&rows, &rows, &[]);
        let s = signals(&x, begin, to).unwrap();
        for i in begin..=to {
            println!("STAR-SIGNAL|{}|{}|{}", rows[i].date, s.buy[i], s.sell[i]);
        }
        for t in candidate_run(&x, begin, to).unwrap() {
            println!(
                "STAR-TRADE|{}|{}|{}|{}|{}",
                rows[t.legs[0].i].date, t.legs[0].px, rows[t.exit_i].date, t.exit_px, t.open
            );
        }
    }

    #[test]
    fn constant_prefix_never_crosses_and_selected_window_starts_flat() {
        let mut r = rows(50);
        for v in &mut r {
            v.close = 100.;
        }
        let x = Ctx::new(&r, &r, &[]);
        let s = signals(&x, 35, 49).unwrap();
        assert!(!s.buy.iter().any(|v| *v) && !s.sell.iter().any(|v| *v));
        assert!(candidate_run(&x, 35, 49).unwrap().is_empty());
    }
}
