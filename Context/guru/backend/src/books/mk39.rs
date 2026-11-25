//! John J. Murphy, "Ten Laws of Technical Trading" (article; research share USB HDD 2/tRADING KNO-2/John J.Murphy - Ten Laws of
//! Technical Trading (Article)/), section "K39 - The Last Stochastic Technique" (a StockCharts add-on citing the Encyclopedia of
//! Technical Market Indicators). Book id `mk39`. Lane K3 (24 Nov 2026). Independent check authors/conv-1123/k3w/mk39_ref.py.
//! The other snippets of the article (gap trading, holiday trades, EMA/MACD crosses) are not coded: each needs more than four readings.
#![allow(dead_code)]
use crate::connors::*;

pub const BOOK: &str = "mk39";
pub const REVIEWED: &[&str] = &["mk39_k39"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "mk39_k39", name: "Murphy K39: unsmoothed 39-period stochastic crosses 50% with a close beyond the previous week's high / low close", group: "Murphy · Ten Laws of Technical Trading", session: 0, dir: "Long",
        status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
        rules: &[
            "E|\"A buy signal is generated when K crosses above 50% and the closing price is above the previous week's high close.\" (K39 = unsmoothed 39 period stochastic, no signal line)",
            "X|\"Sell and/or sell short signals are created when the K line crosses below 50% and the closing price is below the previous week's low close.\" (the long position is closed on the sell signal; p.short = 1 also reverses into a short)",
            "F|\"Weekly, Daily and Hourly through Minute data can all be used effectively for the 39 period stochastic.\" (daily bars here)"],
        notes: "[Article text; unverified] Unstated card-specific choices (6, each one choice, #400): (1) K = 100 x (close - lowest low of 39 bars) / (highest high of 39 bars - lowest low), from bar highs and lows (the formula's image is not in the text); (2) the 'previous week's high (low) close' = the highest (lowest) close of the 5 sessions before the signal bar; (3) a cross means K was at or below (at or above) 50 on the bar before and is above (below) it on the signal bar; (4) the close condition must hold on that same bar, else the cross gives no signal; (5) the trade is taken at the signal bar's close; (6) shorting on the sell signal is an option (p.short, default 0 = long only); the long position (the article's 'buy signal') is closed on the sell signal. Honest total: 6 card-specific choices. Engine-wide defaults / declared adaptations (labelled, not counted, R4 #404): one position at a time; the first K needs 39 bars (warm-up); no costs; the data window; NSE indices and stocks in place of the article's US charts (declared application, #364/#378). The article prints no results; its MSFT/CSCO examples are chart illustrations.",
        claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { reviewed(id).then_some(("mk39", "Murphy K39 stochastic", "K39")) }
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    reviewed(id).then(|| vec![ParamDef { key: "period", label: "Stochastic period", def: 39., min: 10., max: 80., step: 1. },
        ParamDef { key: "short", label: "Also reverse into a short on the sell signal (1) or long only (0)", def: 0., min: 0., max: 1., step: 1. }])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) { return None; }
    let n = (p.get("period").copied().unwrap_or(39.).round().max(2.)) as usize;
    let short = p.get("short").copied().unwrap_or(0.) >= 0.5;
    let (h, l, c, len) = (&x.h, &x.l, &x.c, x.c.len());
    let to = to.min(len - 1);
    let k: Vec<f64> = (0..len).map(|i| if i + 1 >= n { let (a, b) = (l[i + 1 - n..=i].iter().copied().fold(f64::MAX, f64::min), h[i + 1 - n..=i].iter().copied().fold(f64::MIN, f64::max)); if b > a { 100. * (c[i] - a) / (b - a) } else { f64::NAN } } else { f64::NAN }).collect();
    let mut out: Vec<Trade> = vec![];
    let mut cur: Option<Trade> = None;
    let close = |t: Trade, i: usize, why: &str| -> Trade { let mut t = t; (t.exit_i, t.exit_px, t.signal_i, t.reason) = (i, c[i], i, why.to_string()); t };
    for i in from.max(n + 5)..=to {
        let up = k[i - 1] <= 50. && k[i] > 50. && c[i] > c[i - 5..i].iter().copied().fold(f64::MIN, f64::max);
        let dn = k[i - 1] >= 50. && k[i] < 50. && c[i] < c[i - 5..i].iter().copied().fold(f64::MAX, f64::min);
        let open = |dir: f64| Trade { dir, legs: vec![Leg { i, px: c[i], w: 1. }], exit_i: i, exit_px: f64::NAN, reason: String::new(), entry_kind: "close", signal_i: i, open: false, exits: vec![] };
        match cur.as_ref().map(|t| t.dir) {
            Some(d) if d > 0. && dn => { out.push(close(cur.take().unwrap(), i, "K39 sell signal")); if short { cur = Some(open(-1.)); } }
            Some(d) if d < 0. && up => { out.push(close(cur.take().unwrap(), i, "K39 buy signal")); cur = Some(open(1.)); }
            None if up => cur = Some(open(1.)),
            None if dn && short => cur = Some(open(-1.)),
            _ => {}
        }
    }
    if let Some(mut t) = cur { (t.exit_i, t.exit_px, t.signal_i, t.open, t.reason) = (to, c[to], to, true, "open".into()); out.push(t); }
    Some(out)
}

pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == BOOK && reviewed(def.id),
    label: "Murphy's K39 is a price-only technique shown on US stocks (MSFT, CSCO) and mutual funds; it runs here only on actual NSE indices (NIFTY 50, NIFTYBANK, NIFTYIT, NIFTYMIDCAP100, NIFTYMIDCAPSELECT) and NSE stocks, the two classes the independent reference covers, as a declared application. US stocks, ETFs and funds are not admitted.",
    basis: "Murphy, Ten Laws of Technical Trading, section K39 (StockCharts add-on); independent check authors/conv-1123/k3w/mk39_ref.py on the five NSE indices and three NSE stocks",
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn long_on_cross_and_flat_on_sell() {
        let r: Vec<Row> = (0..300).map(|i| { let p = 100. + (i as f64 * 0.09).sin() * 20.; Row { ticker: "T".into(), date: format!("d{:04}", i), open: p, high: p + 1., low: p - 1., close: p, volume: Some(1.) } }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let t = run("mk39_k39", &x, 0, 299, &Params::new()).unwrap();
        assert!(!t.is_empty() && t.iter().all(|z| z.dir > 0.));
    }
    /// hand-run dump for the pandas reference: MK39_IDX=<idx_data.csv> MK39_STK=<csv,..> MK39_OUT=<file> cargo test --release mk39_dump -- --ignored
    #[test]
    #[ignore]
    fn mk39_dump() {
        use std::io::Write;
        let idx = crate::load_csv(&std::env::var("MK39_IDX").unwrap()).unwrap();
        let mut f = std::io::BufWriter::new(std::fs::File::create(std::env::var("MK39_OUT").unwrap()).unwrap());
        writeln!(f, "short,series,dir,entry_date,entry_px,exit_date,exit_px,open").unwrap();
        let mut series: Vec<(String, Vec<Row>)> = idx.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
        for path in std::env::var("MK39_STK").unwrap().split(',') {
            let rows: Vec<Row> = std::fs::read_to_string(path).unwrap().lines().skip(1).map(|l| { let v: Vec<&str> = l.split(',').collect(); let n = |i: usize| v[i].parse::<f64>().unwrap();
                Row { ticker: "STK".into(), date: v[0].into(), open: n(1), high: n(2), low: n(3), close: n(4), volume: None } }).collect();
            series.push((path.rsplit('/').next().unwrap().to_string(), rows));
        }
        for (name, rows) in &series {
            let x = Ctx::new(rows, &[], &[]);
            for sh in [0., 1.] {
                let mut pr = Params::new(); pr.insert("short".into(), sh);
                for z in run("mk39_k39", &x, 0, rows.len() - 1, &pr).unwrap() {
                    writeln!(f, "{sh},{name},{},{},{},{},{},{}", z.dir, rows[z.legs[0].i].date, z.legs[0].px, rows[z.exit_i].date, z.exit_px, z.open).unwrap();
                }
            }
        }
    }
}
