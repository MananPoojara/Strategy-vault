//! Jose Silva, "Reversals pattern II v2.0" (MetaStock indicator documentation, research share USB HDD 1/MetaStock/@Systems & Indicators/MACDH
//! Divergence Kit (metastocktools.com)/Bonus Rev2 kit/Documentation/Indicators.pdf, pp. 2-5 and 8): the weekday two-day reversal. Book id `jsrev2`.
//! Lane K2 (24 Nov 2026, queue R4 #382, Q385 re-read of a 30 Oct NO-RULES row). Daily bars; the author's data are ASX and US shares and indices,
//! here the NSE index series as a labelled #340 adaptation, UNVERIFIED.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "jsrev2";
const IDS: [&str; 5] = ["jsrev2_mon", "jsrev2_tue", "jsrev2_wed", "jsrev2_thu", "jsrev2_fri"];
const DAYS: [&str; 5] = ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday"];

const NOTE: &str = "A|Declared readings (lane K2), 4: (1) the reversal weekday is the author's input 'Reversal day [0=Optimal, 1~5=Mon~Fri]' and the default 0 is an in-sample optimiser output, not a rule, so each weekday is its own card and none is chosen by optimisation; (2) the entry is the open of the reversal day (the author's 'morrow's Open' choice; the other choice, the previous close, trades before the weekday is certain, and the 'optimal' choice is again an optimiser output), the exit is the close of the reversal day (the author's own Win/Profit test); the SmartStop exit of the 'Long-term' variant calls compiled DLLs that are not printed and is not used; (3) trades dated before the series' real-open start (NIFTY 50 2011-01-01, NIFTYMIDCAP100 2013-04-01, where the file's open is about the previous close) are skipped, the history shown is not clamped; (4) the missing-bar guard 'Oki' (p. 3) is read as: a signal needs the two sessions before the reversal day to be the two immediately preceding weekdays, so a day lost to an exchange holiday cancels the signal. The author's 0.1% cost is replaced by no costs. Percent rule, no point scaling.";

pub fn catalogue() -> Vec<StrategyDef> {
    IDS.iter().zip(DAYS).map(|(id, day)| StrategyDef { id, name: Box::leak(format!("Two-day reversal on {day} (Silva Rev2)").into_boxed_str()), group: "Reversals pattern II (Silva)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
        rules: Box::leak(vec![Box::leak(format!("E|After two consecutive closes down by at least minCh % (BaseCh 0.5, p. 2) go long at the open of the next {day}; after two consecutive closes up by at least minCh % go short (Rev2 - Rev day, p. 4: long := revD AND Ref(Ref(C,-1)/C,-1)>=minCh AND Ref(Ref(C,-1)/C,-2)>=minCh)").into_boxed_str()) as &'static str,
            "X|Exit at the close of the reversal day (Win/Profit %, p. 8)", NOTE].into_boxed_slice()),
        notes: "Silva, Rev2 kit documentation pp. 2-8: code only, no result printed.", claim: None }).collect()
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { IDS.contains(&id).then_some(("jsrev2", "weekday two-day reversal", "")) }
pub fn params(id: &str) -> Option<Vec<ParamDef>> { IDS.contains(&id).then(|| vec![ParamDef { key: "min_ch", label: "Minimum daily change (%)", def: 0.5, min: 0.1, max: 5., step: 0.1 }]) }

/// First date from which the series' opens are real (before it the file's open is about the previous close, so open-to-close is not an observation);
/// measured on the vault index files, see kzsea.rs.
fn real_open_from(ticker: &str) -> Option<&'static str> { match ticker { "NIFTY 50" => Some("2011-01-01"), "NIFTYMIDCAP100" => Some("2013-04-01"), _ => None } }

/// Days since 1970-01-01 (proleptic Gregorian) of YYYY-MM-DD, and its weekday (Monday = 0).
fn dn(d: &str) -> Option<(i64, i64)> {
    let (y, m, dd): (i64, i64, i64) = (d.get(0..4)?.parse().ok()?, d.get(5..7)?.parse().ok()?, d.get(8..10)?.parse().ok()?);
    let y2 = if m <= 2 { y - 1 } else { y };
    let era = y2.div_euclid(400); let yoe = y2 - era * 400;
    let doy = (153 * (m + if m > 2 { -3 } else { 9 }) + 2) / 5 + dd - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let n = era * 146097 + doe - 719468;
    Some((n, (n + 3).rem_euclid(7)))
}
fn prev_weekday(n: i64, wd: i64) -> i64 { if wd == 0 { n - 3 } else if wd == 6 { n - 2 } else { n - 1 } }

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let k = IDS.iter().position(|v| *v == id)? as i64;
    if to >= x.c.len() || from >= to { return None; }
    let m = 1.0 + p.get("min_ch").copied().unwrap_or(0.5) / 100.0;
    let mut out: Vec<Trade> = vec![];
    let cut = x.r.first().and_then(|r| real_open_from(&r.ticker));
    for t in from.max(3)..=to {
        if cut.is_some_and(|c| x.r[t].date.as_str() < c) { continue; } // synthetic open: the entry price is not real
        let (Some((n0, w0)), Some((n1, w1)), Some((n2, _))) = (dn(&x.r[t].date), dn(&x.r[t - 1].date), dn(&x.r[t - 2].date)) else { continue };
        if w0 != k || n1 != prev_weekday(n0, w0) || n2 != prev_weekday(n1, w1) { continue; }
        let dir = if x.c[t - 2] / x.c[t - 1] >= m && x.c[t - 3] / x.c[t - 2] >= m { 1.0 }
            else if x.c[t - 1] / x.c[t - 2] >= m && x.c[t - 2] / x.c[t - 3] >= m { -1.0 } else { continue };
        out.push(Trade { dir, legs: vec![Leg { i: t, px: x.o[t], w: 1.0 }], exit_i: t, exit_px: x.c[t], reason: "close of the reversal day".into(), entry_kind: "open", signal_i: t, open: false, exits: Vec::new() });
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn five_weekday_cards_are_one_family() {
        let f: std::collections::HashSet<_> = IDS.iter().map(|i| family_of(i).unwrap()).collect();
        assert_eq!(f.len(), 1);
        assert_eq!(f.into_iter().next().unwrap().0, "jsrev2");
        assert!(family_of("jsrev2_sat").is_none());
    }
    #[test]
    fn dates() { assert_eq!(dn("1970-01-01"), Some((0, 3))); assert_eq!(dn("2026-11-24").unwrap().1, 1); assert_eq!(prev_weekday(dn("2026-11-23").unwrap().0, 0), dn("2026-11-20").unwrap().0); }
    #[test]
    fn sane_runs() {
        // the shared synthetic bars carry 28-day months, so give them consecutive weekday dates from 2000-01-03
        let mut r = super::super::nijat::tests::bars(1500);
        let mut n = dn("2000-01-03").unwrap().0;
        for row in r.iter_mut() {
            let (y, m, d) = civil(n); row.date = format!("{y:04}-{m:02}-{d:02}");
            n += if dn(&row.date).unwrap().1 == 4 { 3 } else { 1 };
        }
        let x = Ctx::new(&r, &r, &[]);
        let mut total = 0;
        for id in IDS {
            let t = run(id, &x, 0, r.len() - 1, &Params::new()).unwrap();
            for z in &t { let i = z.legs[0].i; assert!(z.legs[0].px == r[i].open && z.exit_px == r[i].close && z.exit_i == i); }
            total += t.len();
        }
        assert!(total >= 5, "{total} trades");
        assert!(run(IDS[0], &x, 5, 5, &Params::new()).is_none());
    }
    fn civil(n: i64) -> (i64, i64, i64) { // days since 1970-01-01 -> y, m, d
        let z = n + 719468; let era = z.div_euclid(146097); let doe = z - era * 146097;
        let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365; let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153; let d = doy - (153 * mp + 2) / 5 + 1; let m = if mp < 10 { mp + 3 } else { mp - 9 };
        (yoe + era * 400 + if m <= 2 { 1 } else { 0 }, m, d)
    }
    /// hand-run dump for the pandas reference: JSREV2_CSV=<idx_data.csv> JSREV2_OUT=<file> cargo test --release jsrev2_dump -- --ignored
    #[test]
    #[ignore]
    fn jsrev2_dump() {
        use std::io::Write;
        let data = crate::load_csv(&std::env::var("JSREV2_CSV").unwrap()).unwrap();
        let mut f = std::io::BufWriter::new(std::fs::File::create(std::env::var("JSREV2_OUT").unwrap()).unwrap());
        writeln!(f, "id,ticker,dir,entry_date,entry_px,exit_date,exit_px,open").unwrap();
        for (t, rows) in data.iter() {
            let x = Ctx::new(rows, &[], &[]);
            for id in IDS { for z in run(id, &x, 0, rows.len() - 1, &Params::new()).unwrap() {
                writeln!(f, "{id},{t},{},{},{},{},{},{}", z.dir, rows[z.legs[0].i].date, z.legs[0].px, rows[z.exit_i].date, z.exit_px, z.open).unwrap();
            } }
        }
    }
}

pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "jsrev2",
    label: "Daily percent-change reversal with no currency amount, read on the real open: run and independently checked on the five NSE index series only from the real-open start (see the gate rows).",
    basis: "Silva, Reversals pattern II (MetaStock Rev2 kit documentation pp. 2-8); independent check authors/conv-1123/k2w/jsrev2_ref.py",
};
