//! Scott W. Barrie, "The Complete Idiot's Guide to Market Timing" (Alpha Books, 2003; book id `sbcig`; printed pages,
//! PDF = printed + 21). Reading: claude-continuation-20261020/authors/barrie (agents/, admission/cig_ref.py).
//! Live: the January Barometer 12-month switch (p.270-271), trading days 8-11 of the month (p.157) and the presidential
//! best 18 months (p.172-173). Not coded again (same calculation already in the vault): the 200-day average with a 2.5%
//! band (p.64) = sglr_dow_ma200_band with band 2.5; last day + first four days (p.155-156) = kftsm_month_end (day 1 for
//! the five study days, day 0 for "from the last day"); January Barometer February-December (p.148) = kpss_janbar;
//! Best Six Months = hsta_best6. Held: the S&P 500 / NASDAQ Composite switch (p.145-147) needs NASDAQ Composite prices
//! and two instruments in one run; Dogs of the Dow, O'Higgins, Fed steps (fundamental / rate data); MACD best six months
//! and the Stoken 15 months (dates not fixed); Dow Theory (discretionary).
//! Book: DJIA (S&P 500 for p.64); indices, ETFs and US/India stocks here are a declared application.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};
/// Admitted ids (authors/barrie/admission/cig_ref.py (header)).
pub const REVIEWED: &[&str] = &["sbcig_janbar_12m", "sbcig_mid_8_11", "sbcig_pres18"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] Daily closes; long or cash; every trade buys and sells at a close; no stop. The last trading day of a month is the bar whose next bar falls in another month (the exchange calendar is known in advance). A trade starts only on an entry day inside the test window; a position still held on the last bar is marked at its close. Cash earns nothing here (the book's T-bill or money-market interest is not modelled). The election calendar is the US one on every market.";

const G: &str = "Barrie calendar timing";
fn def(id: &'static str, name: &'static str, rules: &'static [&'static str]) -> StrategyDef {
    StrategyDef { id, name, group: G, session: 0, dir: "Long", status: Status::Live, needs_volume: false, needs_vix: false,
        needs_series: &[], book: "sbcig", rules, notes: NOTE, claim: None }
}

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        def("sbcig_janbar_12m", "January Barometer switch: hold the Dow from an up January to the next down January", &[
            "F|January up = the last January close is above the last December close (\"as January goes so goes the year\", p.269)",
            "E|\"investing in stocks at the end of January, following a strong January and holding your investment until the following January\" (p.270): buy at the last January close after an up January",
            "X|\"staying fully invested in the stock market until a weak January surfaced, then switching to one-year T-Bills until the following January\" (p.270): sell at the last January close of a down January; cash in place of T-bills"]),
        def("sbcig_mid_8_11", "Mid-month: hold the Dow on trading days 8 to 11 of each month", &[
            "E|\"if you were to invest in the Dow Jones Industrial Average only on the eighth to the eleventh business days of the month\" (p.157): buy at the close of trading day 7",
            "X|Sell at the close of trading day 11 of the same month (day numbers count the trading days of the calendar month)",
            "F|Other day ranges are labelled options, not from the book"]),
        def("sbcig_pres18", "Presidential cycle: hold the Dow from the end of October two years before the election to the end of April of the election year", &[
            "E|\"the 18-month period beginning at the end of October, two years before the election\" (p.172): buy at the last October close of each midterm year (1998, 2002, ...)",
            "X|\"and lasting through April of the election year\"; \"invested at the end of October 1998 and held the position until the end of April 2000\" (p.172): sell at the last April close of the election year"]),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let v = match id {
        "sbcig_janbar_12m" => "January Barometer 12 months", "sbcig_mid_8_11" => "Trading days 8-11", "sbcig_pres18" => "Best 18 months", _ => return None,
    };
    Some(("sbcig_calendar", "Barrie calendar timing", v))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(match id {
        "sbcig_janbar_12m" | "sbcig_pres18" => vec![],
        "sbcig_mid_8_11" => vec![p("first", "First trading day held (book 8)", 8., 2., 20., 1.), p("last", "Last trading day held (book 11)", 11., 2., 23., 1.)],
        _ => return None,
    })
}

fn ym(x: &Ctx, i: usize) -> (i32, i32) {
    let d = &x.r[i].date;
    (d[0..4].parse().unwrap_or(0), d[5..7].parse().unwrap_or(0))
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let n = x.r.len();
    let c = &x.c;
    let last_of_month = |i: usize| i + 1 < n && ym(x, i + 1) != ym(x, i);
    let mk = |ei: usize, xi: usize, why: &str, open: bool| Trade { dir: 1., legs: vec![Leg { i: ei, px: c[ei], w: 1. }], exit_i: xi, exit_px: c[xi],
        reason: why.into(), entry_kind: "close", signal_i: xi, open, exits: vec![] };
    let (mut out, mut cur) = (vec![], None::<usize>);
    match id {
        "sbcig_janbar_12m" => {
            for t in from..=to {
                let (y, m) = ym(x, t);
                if m != 1 || !last_of_month(t) { continue; }
                let mut s = t;
                while s > 0 && ym(x, s - 1) == (y, 1) { s -= 1; }
                if s == 0 { continue; }
                let up = c[t] > c[s - 1];
                match cur {
                    None if up && t < to => cur = Some(t),
                    Some(e) if !up => { out.push(mk(e, t, "weak January", false)); cur = None; }
                    _ => {}
                }
            }
        }
        "sbcig_mid_8_11" => {
            let (f, l) = (g("first").round() as i64, g("last").round() as i64);
            let (mut cnt, mut mo) = (0i64, (0, 0));
            for t in 0..=to {
                cnt = if t > 0 && ym(x, t) == ym(x, t - 1) { cnt + 1 } else { 1 };
                if let Some(e) = cur {
                    if t > e && ym(x, t) != mo { out.push(mk(e, t, "new month before the last day (data gap)", false)); cur = None; }
                    else if t > e && cnt == l { out.push(mk(e, t, "last mid-month day", false)); cur = None; }
                }
                if cur.is_none() && t >= from && t < to && cnt == f - 1 { cur = Some(t); mo = ym(x, t); }
            }
        }
        "sbcig_pres18" => {
            let mut ey = 0;
            for t in from..=to {
                if !last_of_month(t) { continue; }
                let (y, m) = ym(x, t);
                if let Some(e) = cur {
                    if m == 4 && y == ey + 2 { out.push(mk(e, t, "end of April of the election year", false)); cur = None; }
                } else if m == 10 && y.rem_euclid(4) == 2 && t < to { cur = Some(t); ey = y; }
            }
        }
        _ => return None,
    }
    if let Some(e) = cur { out.push(mk(e, to, "open", true)); }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    /// Weekday bars 1997-01-01..2001-12-31; close = 100 + bar index, except January 2000 falls (a weak January).
    fn rows() -> Vec<Row> {
        let mut out = vec![];
        let (mut y, mut m, mut d, mut wd) = (1997, 1, 1, 2); // 1 Jan 1997 = Wednesday (0 = Monday)
        while y < 2002 {
            if wd < 5 {
                let i = out.len() as f64;
                let c = if y == 2000 && m == 1 { 50. - i * 0.01 } else { 100. + i };
                out.push(Row { ticker: "T".into(), date: format!("{y}-{m:02}-{d:02}"), open: c, high: c, low: c, close: c, volume: None });
            }
            let dim = [31, if y % 4 == 0 { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31][m - 1];
            wd = (wd + 1) % 7; d += 1;
            if d > dim { d = 1; m += 1; if m > 12 { m = 1; y += 1; } }
        }
        out
    }
    #[test]
    fn hand_checked_calendar_trades() {
        let r = rows();
        let x = Ctx::new(&r, &r, &[]);
        let to = r.len() - 1;
        let date = |i: usize| r[i].date.as_str();
        // January 1997 is up (first bar has no prior December close, so no 1997 decision): no entry until 1998.
        let j = run("sbcig_janbar_12m", &x, 0, to, &Params::new()).unwrap();
        assert_eq!(j.iter().map(|t| (date(t.legs[0].i), date(t.exit_i), t.open)).collect::<Vec<_>>(),
            vec![("1998-01-30", "2000-01-31", false), ("2001-01-31", "2001-12-31", true)]);
        // March 1997 (1 Mar = Saturday): trading day 7 = Tue 11 Mar, day 11 = Mon 17 Mar.
        let m = run("sbcig_mid_8_11", &x, 0, to, &Params::new()).unwrap();
        assert_eq!((date(m[2].legs[0].i), date(m[2].exit_i), m[2].reason.as_str()), ("1997-03-11", "1997-03-17", "last mid-month day"));
        assert_eq!(m.len(), 60);
        // midterm 1998: last October bar Fri 30 Oct 1998 -> last April bar Fri 28 Apr 2000.
        let p = run("sbcig_pres18", &x, 0, to, &Params::new()).unwrap();
        assert_eq!(p.iter().map(|t| (date(t.legs[0].i), date(t.exit_i))).collect::<Vec<_>>(), vec![("1998-10-30", "2000-04-28")]);
        assert!(run("sbcig_x", &x, 0, to, &Params::new()).is_none());
    }
}

// Instrument admission: Scott W. Barrie 2026-10-29 (lane B) block, moved from instrument_policy.rs on 31 Oct 2026 (lane ST).
// Admission only for the module REVIEWED lists (authors/barrie/admission/cig_ref.py (header)); kept in one block to merge.
// Runs on actual indices, ETFs and stocks, US and India (index_etf_or_stock, nifty50 basket, declared application).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book=="sbcig" && crate::books::sbcig::reviewed(def.id),
    label: "Scott Barrie measures these calendar rules on the Dow Jones Industrial Average with the US election calendar; runs on other indices, ETFs and US/India stocks are a declared application, not the author's test.",
    basis: "Scott W. Barrie, The Complete Idiot's Guide to Market Timing (2003), Ch.12 p.157, Ch.13 p.172-173, Ch.20 p.269-271",
};
