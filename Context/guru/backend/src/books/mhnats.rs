//! Matthew Hanson, *The Alchemy of Trading Systems* (2004/2005), "An Actual System" (printed p.85-91).
//! Source: research drive Trading Knowledge I/Matthew Hanson - The Alchemy of Trading Systems/The Alchemy of Trading
//! Systems (Matthew Hanson,2006).pdf (scanned, read from page images); copy in Books/Matthew Hanson/. Book id "mhnats".
//! Lane I1 (13 Nov 2026): runs on NSE index futures open interest (Indian application of a US-market rule).
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "mhnats";
/// Admitted ids (lane I1, Indian application; gate `allowed` = NIFTY 50, NIFTYBANK).
pub const REVIEWED: &[&str] = &["mhnats_oi7"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }
/// Daily open interest of the index's NSE futures, all expiries summed (NSE F&O bhavcopy).
pub const FUT_OI: &str = "FUT_OI";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        StrategyDef {
            id: "mhnats_oi7", name: "S&P open-interest system: 7-day average OI vs today's OI, always in, open to open", group: "An Actual System (p.85-87)",
            session: 0, dir: "Long/Short", status: Status::Live,
            needs_volume: false, needs_vix: false, needs_series: &[FUT_OI], book: BOOK,
            rules: &[
                "F|\"The system graphed in table 1.2 measures differences in the open interest of the S&P 500 futures contract on a daily basis.\" (p.85)",
                "E|\"=IF (AVERAGE (G3:G9)>G9, 1, -1) This means if the average open interest of the previous seven days of trading is greater than today's open interest, then go long (+1), if not, go short (-1).\" (p.86)",
                "X|\"If the signal stayed the same, you would keep your existing position.\" Data for the close of the 12th is traded \"on the opening of the 13th and holding the position overnight (i.e. one full 24 hour period trading open to open)\" (p.87-88)",
                "F|[Indian application, lane I1] Open interest = the NSE futures on the traded index (NIFTY 50: FUTIDX NIFTY; NIFTYBANK: FUTIDX BANKNIFTY), all expiries summed, from the NSE F&O bhavcopy (end-of-day). A day without an OI value gives no signal: the position is kept [reading].",
            ],
            notes: "Indian application of a US-market rule (lane I1, 13 Nov 2026): the book used the S&P 500 futures contract (open interest and 'S&P Continuous Futures' prices, Tables 1.2-1.4). Here the signal reads the open interest of the index's NSE futures (all expiries summed, so a roll does not move it; the book does not say which month it read [reading]) and the trades are on the index itself, open to open (the futures price is not used). G3:G9 is the 7-row window ending today (G9). The book's improved 'adjacent columns' version (p.88, Table 1.4) is not specified, so it is not catalogued as a strategy. Book: chart results only (Tables 1.2-1.7), no trade statistics; the printed version lost money (p.86).",
            claim: None,
        },
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == "mhnats_oi7").then_some(("mhnats_oi7", "Hanson S&P open-interest system", "p.86 spreadsheet rule"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    (id == "mhnats_oi7").then(|| vec![ParamDef { key: "n", label: "Open-interest average length (rows G3:G9 = 7, ending today)", def: 7., min: 2., max: 30., step: 1. }])
}

/// +1 when the n-day average OI (window ending today) is above today's OI, -1 otherwise; 0 when a value is missing.
fn signal(oi: &[f64], n: usize, i: usize) -> f64 {
    if i + 1 < n { return 0.; }
    let w = &oi[i + 1 - n..=i];
    if w.iter().any(|v| !v.is_finite()) { return 0.; }
    if w.iter().sum::<f64>() / n as f64 > oi[i] { 1. } else { -1. }
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if id != "mhnats_oi7" { return None; }
    if from > to || to >= x.r.len() { return Some(vec![]); }
    let Some(oi) = x.series(FUT_OI) else { return Some(vec![]) };
    let n = p.get("n").copied().filter(|v| v.is_finite()).map_or(7, |v| v.round().max(2.) as usize);
    let (mut out, mut cur): (Vec<Trade>, Option<Trade>) = (vec![], None);
    for i in from..to {
        let s = signal(oi, n, i);
        if s == 0. || cur.as_ref().is_some_and(|t| t.dir == s) { continue; }
        let j = i + 1; // the signal from today's close is traded at the next open
        if let Some(mut t) = cur.take() {
            t.exit_i = j; t.exit_px = x.o[j]; t.signal_i = i; t.reason = "signal reversed (next open)".into();
            out.push(t);
        }
        cur = Some(Trade { dir: s, legs: vec![Leg { i: j, px: x.o[j], w: 1. }], exit_i: j, exit_px: x.o[j], reason: String::new(),
            entry_kind: "open", signal_i: i, open: false, exits: vec![] });
    }
    if let Some(mut t) = cur {
        t.exit_i = to; t.exit_px = x.c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true;
        out.push(t);
    }
    Some(out)
}

// Instrument admission (lane I1): India index futures open interest, NIFTY 50 and NIFTYBANK only (gate `allowed`).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == BOOK && reviewed(def.id),
    label: "Indian application of a US-market rule: the book read the S&P 500 futures open interest and traded S&P futures; here the signal is the open interest of the index's NSE futures (NSE F&O bhavcopy, all expiries summed) and the trades are on the NIFTY 50 / NIFTYBANK index, open to open. Not the author's test.",
    basis: "The Alchemy of Trading Systems, 'An Actual System' p.85-88 (page images read again by OCR, lane I1): =IF(AVERAGE(G3:G9)>G9,1,-1) on daily S&P futures open interest, end-of-day signal traded at the next open, held open to open. Independent Python reference authors/i1/w/i1_ref.py.",
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn rows(n: usize) -> Vec<Row> {
        (0..n).map(|i| Row { ticker: "NIFTY 50".into(), date: format!("2020-01-{:02}", i + 1), open: 100. + i as f64, high: 102. + i as f64, low: 99. + i as f64, close: 101. + i as f64, volume: None }).collect()
    }
    #[test]
    fn oi7_always_in_reverses_at_the_next_open() {
        let r = rows(14);
        let mut x = Ctx::new(&r, &r, &[]);
        // rising OI -> average below today -> short; a drop on bar 9 -> long; NaN on bar 11 keeps the position
        let mut oi: Vec<f64> = (0..14).map(|i| 1000. + 10. * i as f64).collect();
        oi[9] = 900.; oi[11] = f64::NAN;
        x.extra.insert(FUT_OI.into(), oi);
        let t = run("mhnats_oi7", &x, 0, 13, &Params::new()).unwrap();
        assert_eq!(t.len(), 3);
        assert_eq!((t[0].dir, t[0].legs[0].i, t[0].legs[0].px), (-1., 7, 107.)); // first full window ends on bar 6
        assert_eq!((t[0].exit_i, t[0].exit_px, t[1].dir, t[1].legs[0].i), (10, 110., 1., 10));
        // bar 10: mean of bars 4..10 (1040..1080, 900, 1100) = 1042.9 < 1100 -> short again at the open of bar 11
        assert_eq!((t[1].exit_i, t[2].dir, t[2].legs[0].i), (11, -1., 11));
        assert!(t[2].open && t[2].exit_i == 13 && t[2].exit_px == 114.);
    }
}
