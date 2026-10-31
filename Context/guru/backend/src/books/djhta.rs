//! Darrell R. Jobman (ed.), "The Handbook of Technical Analysis" (Probus, 1995; book id `djhta`; scan, PDF page = printed + 19 in
//! Section II). Reading: claude-continuation-20261020/authors/jobman/agents/. An anthology of Futures-magazine articles: the
//! only complete rule is Dennis Dunn's seven-week moving average technique (ch.8 p.125-126): buy on a Thursday close over the
//! moving average, sell on a Thursday close below it, the average plotted two weeks ahead, always in the market. Everything else
//! is indicator description, chart reading, futures-unit rules (Donchian's 5- and 20-day method), sentiment/COT data or chapters by
//! authors carded elsewhere (Wilder DMI, Elder Triple Screen, DeMark Sequential, Bill Williams MFI).
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "djhta";
/// Admitted ids (authors/jobman/admission/SOURCE-CONTRACT.md).
pub const REVIEWED: &[&str] = &["djhta_dunn7w"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const DUNN_NOTE: &str = "[Declared readings, not from the book] Weekly bar = the last trading day from Monday to Thursday of each Friday-to-Thursday week (Thursday, or Wednesday when Thursday is a holiday, as the book says; a week with only a Friday is skipped). Seven-week average = simple average of the last seven such weekly closes; 'plotted two weeks in advance' = each Thursday close is compared with the average computed two weeks earlier. Close above = long, close below = short (always in the market, stop and reverse), filled at that Thursday close (the order for the coming Thursday is known a week ahead; the Friday-morning alternative is not modelled); an equal close keeps the position. Flat at the window start; no new position on the last bar; no costs. Option side=1: long or cash. Book: futures (no market named, no results printed); indices, ETFs and stocks, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    let nt = |id, name, group, why, rules| StrategyDef { id, name, group, session: 0, dir: "Long/Short", status: NotTestable(why), needs_volume: false,
        needs_vix: false, needs_series: &[], book: BOOK, rules, notes: "Held after the 2026-10-25 reading (authors/jobman/agents).", claim: None };
    vec![
        StrategyDef { id: "djhta_dunn7w", name: "Seven-week moving average, Thursday closes, plotted two weeks ahead (Dunn)", group: "Ch.8 Following the trend · Moving average applications (p.125-126)",
            session: 0, dir: "Long/Short", status: Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
            rules: &["F|\"a seven-week moving average technique\" (Dennis Dunn, Dunn & Hargitt, p.125-126)",
                     "E|\"Buy on a Thursday close over the moving average line and sell on a Thursday close below the moving average. If the market is closed on Thursday, use Wednesday's close.\" (p.126)",
                     "F|\"The seven-week moving average line was NOT plotted in coincidence with the close but was plotted two weeks in advance to make a market decision earlier.\" (p.126)",
                     "X|\"After a Thursday close, all you had to do was enter your order for the coming Thursday (or Friday morning). You were always in the market.\" (p.126): stop and reverse"],
            notes: DUNN_NOTE, claim: None },
        nt("djhta_donchian", "Donchian 5- and 20-day moving average method", "Ch.8 Following the trend (p.126-128)",
           "penetrations are counted in futures price units by price class (40 cents for commodities over 400, 'six price classes'), given in a 'condensed version', with weekday-dependent order timing; no unit exists for indices or stocks",
           &["E|Rule A: act on closes that cross the 20-day average by one full unit more than the previous penetration; Rules B-D, supplemental timing rules (p.126-128)"]),
        nt("djhta_indicators", "Moving-average crossovers, MACD, DMI, RSI, stochastics, %R, momentum, CCI", "Ch.8-9 Indicators (p.131-183)",
           "indicator descriptions with typical readings, no settings plus exits; the DMI crossover with the extreme-point rule is Wilder's own system (wwnc_dmi), the stochastics chapter is Lane's description",
           &["E|\"When +DI crosses above -DI, you buy; when -DI crosses over +DI, you sell ... On the day of the crossover, the extreme price that day is a reverse point for a stop\" (p.136)"]),
        nt("djhta_turns", "Sequential, accumulation-distribution, market facilitation index, Triple Screen, combining indicators", "Ch.10-11 Signaling the turns / Indicator applications (p.183-224)",
           "Sequential 'produces only entry signals' (p.187); A/D and MFI are indicator readings; Elder's Triple Screen is carded with Elder; Seehusen combines indicators by judgement",
           &[]),
        nt("djhta_charts", "Bar-chart, congestion, short-term reversal, point-and-figure, candlestick and Market Profile patterns", "Section I Charts (p.1-118)",
           "patterns without entry orders or exits (John Hill: 'should not use them alone'); chart reading",
           &["F|Nine short-term reversal patterns: island, reversal gap, pattern gap, three-day high, three- and four-day close reversals, tails, narrow range, five-day highs (p.58-63)"]),
        nt("djhta_sentiment_cycles", "Volume/open interest, COT, bullish consensus, seasonals, spreads, cycles, Elliott and Gann", "Sections III-V (p.225-383)",
           "needs futures open interest, COT positions, sentiment surveys or contract-month data, or reads cycles and waves by eye",
           &[]),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "djhta_dunn7w" => ("djhta_dunn7w", "Dunn seven-week moving average", "7 weeks, 2 weeks ahead"),
        _ if id.starts_with("djhta_") => ("djhta_nt", "Handbook of Technical Analysis (held)", "-"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(match id {
        "djhta_dunn7w" => vec![p("weeks", "Moving average length (weeks; book 7)", 7., 2., 52., 1.),
            p("ahead", "Plotted ahead (weeks; book 2)", 2., 0., 10., 1.),
            p("side", "Sides (0 long and short, 1 long or cash)", 0., 0., 1., 1.)],
        _ if id.starts_with("djhta_") => vec![],
        _ => return None,
    })
}

fn days(d: &str) -> i64 {
    let (y, m, dd): (i64, i64, i64) = (d[0..4].parse().unwrap_or(1970), d[5..7].parse().unwrap_or(1), d[8..10].parse().unwrap_or(1));
    let y2 = if m <= 2 { y - 1 } else { y };
    let era = y2.div_euclid(400);
    let yoe = y2 - era * 400;
    let doy = (153 * ((m + 9) % 12) + 2) / 5 + dd - 1;
    era * 146097 + yoe * 365 + yoe / 4 - yoe / 100 + doy - 719468
}

/// Bars that close a Friday-to-Thursday week: the last Monday-Thursday bar of each such week.
fn thursday_bars(x: &Ctx) -> Vec<usize> {
    let n = x.r.len();
    let wd: Vec<i64> = x.r.iter().map(|r| (days(&r.date) + 3).rem_euclid(7)).collect(); // 0 = Monday
    let key: Vec<i64> = x.r.iter().zip(&wd).map(|(r, &w)| days(&r.date) + (3 - w).rem_euclid(7)).collect(); // the Thursday on/after
    let mut out = vec![];
    for i in 0..n {
        let last_of_week = i + 1 == n || key[i + 1] != key[i] || wd[i + 1] > 3;
        if wd[i] <= 3 && last_of_week { out.push(i); }
    }
    out
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if id != "djhta_dunn7w" { return None; }
    let schema = params(id)?;
    if from > to || to >= x.r.len() { return Some(vec![]); }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap().def);
    let (nw, ahead, long_only) = (g("weeks").round().max(1.) as usize, g("ahead").round().max(0.) as usize, g("side") >= 0.5);
    let wk = thursday_bars(x);
    let wc: Vec<f64> = wk.iter().map(|&i| x.c[i]).collect();
    let ma: Vec<f64> = (0..wc.len()).map(|k| if k + 1 >= nw { wc[k + 1 - nw..=k].iter().sum::<f64>() / nw as f64 } else { f64::NAN }).collect();
    let mut out = vec![];
    let mut cur: Option<Trade> = None;
    for (k, &i) in wk.iter().enumerate() {
        if i < from || i > to || k < ahead { continue; }
        let m = ma[k - ahead];
        if !m.is_finite() { continue; }
        let want = if wc[k] > m { 1. } else if wc[k] < m { if long_only { 0. } else { -1. } } else { continue };
        let have = cur.as_ref().map_or(0., |t| t.dir);
        if want == have { continue; }
        if let Some(mut t) = cur.take() {
            t.exit_i = i; t.exit_px = x.c[i]; t.signal_i = i;
            t.reason = if want == 0. { "Thursday close below the average".into() } else { "Thursday close through the average · reverse".into() };
            out.push(t);
        }
        if want != 0. && i < to {
            cur = Some(Trade { dir: want, legs: vec![Leg { i, px: x.c[i], w: 1. }], exit_i: i, exit_px: x.c[i], reason: String::new(),
                entry_kind: "close", signal_i: i, open: false, exits: vec![] });
        }
    }
    if let Some(mut t) = cur { t.exit_i = to; t.exit_px = x.c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true; out.push(t); }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn thursday_or_wednesday_closes_and_two_week_shift() {
        // 2024-01-01 is a Monday; trading days Mon-Fri, Thursday 2024-01-25 closed
        let mut r = vec![];
        let mut d = days("2024-01-01");
        let civil = |z: i64| { let z = z + 719468; let era = z.div_euclid(146097); let doe = z - era * 146097;
            let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365; let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
            let mp = (5 * doy + 2) / 153; let dd = doy - (153 * mp + 2) / 5 + 1; let m = if mp < 10 { mp + 3 } else { mp - 9 };
            format!("{:04}-{:02}-{:02}", yoe + era * 400 + if m <= 2 { 1 } else { 0 }, m, dd) };
        let mut k = 0.;
        while r.len() < 120 {
            let w = (d + 3).rem_euclid(7);
            let s = civil(d);
            if w < 5 && s != "2024-01-25" { k += 1.; r.push(Row { ticker: "T".into(), date: s, open: 100. + k, high: 101. + k, low: 99. + k, close: 100. + k, volume: None }); }
            d += 1;
        }
        let x = Ctx::new(&r, &r, &[]);
        let wk = thursday_bars(&x);
        assert_eq!(x.r[wk[0]].date, "2024-01-04");
        assert!(wk.iter().any(|&i| x.r[i].date == "2024-01-24")); // Wednesday used when Thursday is closed
        let t = run("djhta_dunn7w", &x, 0, 119, &Params::new()).unwrap();
        // rising prices: long from the first week with an average two weeks back (week index 8), never reversed
        assert_eq!(t.len(), 1);
        assert_eq!(t[0].dir, 1.); assert_eq!(t[0].legs[0].i, wk[8]); assert!(t[0].open);
    }
}

// Instrument admission: Jobman 2026-10-24 block, moved from instrument_policy.rs on 31 Oct 2026 (lane ST).
// Admission only for the module REVIEWED lists (authors/jobman/admission/SOURCE-CONTRACT.md); kept in one block to merge.
// Runs on actual indices, ETFs and stocks, US and India (index_etf_or_stock, nifty50 basket, declared application).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| matches!(def.book,"djhta") && (crate::books::djhta::reviewed(def.id)),
    label: "Dennis Dunn's technique is quoted for futures (no market named, no results); runs on indices, ETFs and stocks, US and India, are a declared application, not the author's test.",
    basis: "Darrell R. Jobman (ed.), The Handbook of Technical Analysis (1995), ch.8 p.125-126: Dunn's seven-week moving average, Thursday closes over/under the average plotted two weeks ahead, always in the market. Signals and fills on the Thursday close. Reviewed in authors/jobman/admission/SOURCE-CONTRACT.md with independent chronological ledgers; no costs.",
};
