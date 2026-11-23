//! Thomas Katz (Katz & McCormick), "Seasonality and Trading", Technical Analysis of Stocks & Commodities V.15:4 (Apr 1997), pp. 162-169
//! (research share Trading Knowledge I/Stocks & Commodities DVD 11.26/VOLUMES/V15/C04/SEASONA.pdf), the date-specific open-to-close system
//! TASC_ART5_1 (Figure 1, p. 164). Book id `kzsea`. Lane K2 (23 Nov 2026, queue R4 #381). The article trades the S&P 500; here the NSE index
//! daily bars are used as a labelled #340 adaptation, UNVERIFIED. The second, end-of-day system (Figure 2) is not coded: the figure's
//! IntChg definition is not printed in the text layer.
use std::collections::HashMap;
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "kzsea";
const ID: &str = "kzsea_date";

const NOTE: &str = "A|Declared readings (lane K2), 5 (declared adaptation #340): (1) ONE scaling rule for the S&P point threshold: Thr = 0.5 S&P point at an S&P level of about 700 = 0.0714% of the price, so HistChg (the mean open-to-close change over the past instances of the date) is taken as a percentage of the open; (2) 'the same date' is the calendar date of the bar being traded in each of the previous Wind years, an instance counts only if the market traded that day, and YCount = Wind needs the data to start at or before Wind years earlier (the article's YCount); (3) past instances dated before the series' real-open start (NIFTY 50 2011-01-01, NIFTYMIDCAP100 2013-04-01; the file's earlier open is about the previous close) are skipped in HistCnt, HistPct and HistChg, the history shown is not clamped; (4) the trade is made at the open of the bar and exited at its close ('exit all trades at the close'); the profit target and money-management stop of the article's test are not printed and are omitted; (5) no costs. The article's S&P 500 result is not comparable.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: ID, name: "Date-specific open-to-close seasonal (Katz)", group: "Seasonality and Trading (Katz, 1997)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
        rules: &["E|\"Inputs: Disp(1), Wind(6), Thr(.5) ... If HistCnt > 2 And YCount=Wind Then Begin If HistPct > 70 And HistChg > Thr Then Buy At Market; If HistPct < 30 And HistChg < -Thr Then Sell At Market; End;\" (Figure 1, p. 164): HistCnt = past instances of tomorrow's date on which the market traded, HistPct = percentage of them that closed above the open, HistChg = their mean open-to-close change",
            "X|Exit at the close of the same day (p. 164: \"enter at tomorrow's open and exit at the close\")", NOTE],
        notes: "Katz, S&C V.15:4 pp. 162-169, Figure 1. The article reports the S&P 500 1983-96 as 'somewhat profitable' before costs; not comparable with an index run.", claim: None }]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { (id == ID).then_some(("kzsea", "date-specific seasonal", "")) }
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    (id == ID).then(|| vec![ParamDef { key: "wind", label: "Lookback (years)", def: 6., min: 3., max: 15., step: 1. }, ParamDef { key: "thr", label: "Threshold (% of the open; 0.5 S&P pts)", def: 0.0714, min: 0., max: 0.5, step: 0.01 }])
}

/// First date from which the series' opens are real (before it the file's open is about the previous close, so an open-to-close change is not an observation).
/// Measured on the vault index files: median |open / previous close - 1| is 0.02% on NIFTY 50 until 2010 (0.3% after) and 0.01% on NIFTYMIDCAP100 until 2013-03 (0.2-0.3% after).
fn real_open_from(ticker: &str) -> Option<&'static str> { match ticker { "NIFTY 50" => Some("2011-01-01"), "NIFTYMIDCAP100" => Some("2013-04-01"), _ => None } }

fn ymd(d: &str) -> Option<(i32, u32, u32)> {
    let mut it = d.split('-');
    Some((it.next()?.parse().ok()?, it.next()?.parse().ok()?, it.next()?.parse().ok()?))
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if id != ID || to >= x.c.len() || from >= to { return None; }
    let wind = p.get("wind").copied().unwrap_or(6.0).round().max(1.0) as i32;
    let thr = p.get("thr").copied().unwrap_or(0.0714) / 100.0;
    let dates: Vec<Option<(i32, u32, u32)>> = x.r.iter().map(|r| ymd(&r.date)).collect();
    let cut = x.r.first().and_then(|r| real_open_from(&r.ticker));
    let first_year = dates.iter().flatten().map(|d| d.0).min()?;
    let idx: HashMap<(i32, u32, u32), usize> = dates.iter().enumerate().filter_map(|(i, d)| d.map(|d| (d, i))).collect();
    let mut out: Vec<Trade> = vec![];
    for d in from.max(1)..=to {
        let Some((y, m, dd)) = dates[d] else { continue };
        if first_year > y - wind { continue; }
        let (mut cnt, mut up, mut chg) = (0usize, 0usize, 0.0);
        for k in 1..=wind {
            if let Some(&j) = idx.get(&(y - k, m, dd)) {
                if cut.is_some_and(|c| x.r[j].date.as_str() < c) { continue; } // synthetic open: not an observation
                cnt += 1;
                if x.c[j] > x.o[j] { up += 1; }
                chg += (x.c[j] - x.o[j]) / x.o[j];
            }
        }
        if cnt <= 2 { continue; }
        let (pct, mean) = (100.0 * up as f64 / cnt as f64, chg / cnt as f64);
        let dir = if pct > 70.0 && mean > thr { 1.0 } else if pct < 30.0 && mean < -thr { -1.0 } else { continue };
        out.push(Trade { dir, legs: vec![Leg { i: d, px: x.o[d], w: 1.0 }], exit_i: d, exit_px: x.c[d], reason: "close of the day".into(), entry_kind: "open", signal_i: d, open: false, exits: Vec::new() });
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sane_runs() {
        // the synthetic bars span 5 years, so the 6-year lookback is shortened to 3 for this check
        let r = super::super::nijat::tests::bars(1500);
        let x = Ctx::new(&r, &r, &[]);
        let mut p = Params::new(); p.insert("wind".into(), 3.0); p.insert("thr".into(), 0.0);
        let t = run(ID, &x, 0, r.len() - 1, &p).unwrap();
        assert!(t.len() >= 3, "{} trades", t.len());
        for z in &t { let i = z.legs[0].i; assert!(z.legs[0].px == r[i].open && z.exit_px == r[i].close && z.exit_i == i); }
        assert!(run(ID, &x, 5, 5, &p).is_none());
        // default 6-year window: the 5-year history is too short, no trades
        assert!(run(ID, &x, 0, r.len() - 1, &Params::new()).unwrap().is_empty());
    }
    /// hand-run dump for the pandas reference: KZSEA_CSV=<idx_data.csv> KZSEA_OUT=<file> cargo test --release kzsea_dump -- --ignored
    #[test]
    #[ignore]
    fn kzsea_dump() {
        use std::io::Write;
        let data = crate::load_csv(&std::env::var("KZSEA_CSV").unwrap()).unwrap();
        let mut f = std::io::BufWriter::new(std::fs::File::create(std::env::var("KZSEA_OUT").unwrap()).unwrap());
        writeln!(f, "id,ticker,dir,entry_date,entry_px,exit_date,exit_px,open").unwrap();
        for (t, rows) in data.iter() {
            let x = Ctx::new(rows, &[], &[]);
            for z in run(ID, &x, 0, rows.len() - 1, &Params::new()).unwrap() {
                writeln!(f, "{ID},{t},{},{},{},{},{},{}", z.dir, rows[z.legs[0].i].date, z.legs[0].px, rows[z.exit_i].date, z.exit_px, z.open).unwrap();
            }
        }
    }
}

pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "kzsea",
    label: "Daily open-to-close date-seasonal rule; the S&P point threshold is a declared percentage of the price; run and independently checked on the five NSE index series only (NIFTY 50, NIFTYBANK, NIFTYIT, NIFTYMIDCAP100, NIFTYMIDCAPSELECT).",
    basis: "Katz, Seasonality and Trading (S&C V.15:4), Figure 1 p. 164; independent check authors/conv-1123/k2w/kzsea_ref.py",
};
