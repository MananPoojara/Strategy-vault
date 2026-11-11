//! Stanley Kroll, "Dragons and Bulls: Profitable Investment Strategies for Trading Stocks and Commodities" (Addison-Wesley,
//! 1994; book id `skdab`) and "Kroll on Futures Trading Strategy" (Dow Jones-Irwin, 1988; book id `skfts`). Reading:
//! claude-continuation-20261020/authors/kroll/agents/. Live: the Ch.11 long-term trend strategy of Dragons and Bulls (p.59-61):
//! the 50-day average and its slope set the trend, close > 10 > 20 > 50-day averages enters with the trend, close < 4 < 9 <
//! 18-day averages stops it out (mirror for shorts), and the short-term signal re-enters while the trend holds.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};
use crate::books::mpts::sma;

pub const BOOK: &str = "skdab";
pub const REVIEWED: &[&str] = &["skdab_trend3ma"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }
/// Lane K74 (R4 #277 (2)): Kroll on Futures Trading Strategy p.97, the 19-day close cross (not a duplicate of kftsm_ma, which
/// trades the average's direction).
pub const REVIEWED_SKFTS: &[&str] = &["skfts_ma19_close"];

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    let nt = |id, name, group, book, why, rules| StrategyDef { id, name, group, session: 0, dir: "Long/Short", status: NotTestable(why), needs_volume: false,
        needs_vix: false, needs_series: &[], book, rules, notes: "Held after the 2026-10-24 reading (authors/kroll/agents).", claim: None };
    vec![
        StrategyDef { id: "skdab_trend3ma", name: "50-day trend filter with 10/20/50 entries and 4/9/18 stops (Ch.11)", group: "Ch.11 · Long-term versus short-term trading (p.59-61)",
            session: 0, dir: "Long/Short", status: Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
            rules: &["F|\"The trend is up if: (a) The closing price is above the moving average line and (b) The slope of the moving average line is up\" (50-day simple average; down: below and sloping down; otherwise sideways, 'avoided') (p.59-60)",
                     "E|\"In an uptrend (using the 50-day simple moving average) buy on the long-term signal, that is; close > 10-day > 20-day > 50-day\" (p.60); downtrend: sell short on close < 10-day < 20-day < 50-day",
                     "X|\"Stop out the trade (liquidate only, do not go short) using the short-term signal, that is; close < 4-day < 9 day < 18-day\" (p.60); shorts: close > 4-day > 9-day > 18-day (p.61)",
                     "E|\"If you are stopped out and the trend continues up (based on the 50-day simple moving average) re-enter long on the short-term buy signal, that is; close > 4-day > 9-day > 18-day\" (p.60); mirror for shorts (p.61)",
                     "F|\"In an uptrend, carry long positions only, or else stand aside. No short positions.\" (p.60)"],
            notes: "Kroll: 'the particular moving average values presented here are not the ones I specifically use. They are presented for illustration only' (p.59), but they are the rules he gives for the discussion. [Declared readings, not from the book] all averages are simple averages of closes; slope up = today's 50-day average above yesterday's; orders fill at the close of the signal day; re-entry is allowed while the trend has stayed in the same direction since the stop-out (a trend change ends it); the optional exit stop at 70% of futures margin is not used; flat at the window start. Indices, ETFs and stocks, US and India, are a declared application (book: NIKKEI and futures examples).",
            claim: None },
        nt("skdab_9_18_50", "Close vs 9/18/50-day averages with the 50-day trend (Ch.6 NIKKEI example)", "Ch.6 · That's the way you want to bet (p.32-35)", BOOK,
           "the 9/18/50 signals are given as entry and exit signals but the exit (opposite signal, or loss of the buy condition) and the 'get back aboard on the close of the next two days' re-entry are not defined exactly",
           &["E|\"Buy: close > 9-SMA > 18-SMA > 50-SMA. Sell: close < 9-SMA < 18-SMA < 50 SMA.\" (p.32)"]),
        nt("skdab_strategy", "Pyramiding, margin, news and trader-discipline chapters", "Dragons and Bulls", BOOK, "money management and judgement, no complete rule", &[]),
        StrategyDef { id: "skfts_ma19_close", name: "19-day moving average close cross (Ch.12 example)", group: "Kroll on Futures Trading Strategy · Ch.12 (p.95-100)",
            session: 0, dir: "Long", status: Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "skfts",
            rules: &["E|\"For example, using a 19-day m.a., you would buy when the price closes above the 19-day m.a. ...\" (p.97)",
                     "X|\"... and sell (possibly reversing as well) when the price closes below\" (p.97)",
                     "F|Read [lane K74, R4 #277 (2)]: a simple average of closes; the order fills at the close of the signal day; flat at the start, the first close above the average buys; 'possibly reversing' is the labelled option reverse = 1 (long and short, a close below also sells short and a close above covers and buys); a close equal to the average changes nothing"],
            notes: "Kroll gives this as an example of how a moving-average system is used ('you would buy ... and sell'), with the length, the entry and the exit printed; the book's examples are futures charts with no results, so there is no book comparison. Indices, ETFs and stocks, US and India, are a declared application. Not a duplicate of kftsm_ma, which trades the direction of the average (Kaufman Table 8.2).",
            claim: None },
        nt("skfts_ma_examples", "Moving average examples (5/20, 4-9-18) and Hochheimer's best averages", "Kroll on Futures Trading Strategy · Ch.12 (p.95-100)", "skfts",
           "illustrations of common systems already in the vault (e.g. kftsm_4_9_18); Hochheimer's lengths are 1970-76 optimisations per futures market. The 19-day close cross is coded as skfts_ma19_close (lane K74)",
           &["E|\"you would buy when the price closes above the 19-day m.a. and sell (possibly reversing as well) when the price closes below\" (p.97)"]),
        nt("skfts_strategy", "Long-term trend trading, pyramiding and money management", "Kroll on Futures Trading Strategy", "skfts",
           "discretionary strategy and tactics around unspecified long-term computer systems", &[]),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "skdab_trend3ma" => ("skdab_trend3ma", "Kroll trend strategy", "10/20/50 + 4/9/18"),
        "skfts_ma19_close" => ("skfts_ma19_close", "Kroll 19-day close cross", "19-day MA"),
        _ if id.starts_with("skdab_") || id.starts_with("skfts_") => ("skdab_nt", "Kroll (held)", "-"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    Some(match id {
        "skdab_trend3ma" => vec![],
        "skfts_ma19_close" => vec![ParamDef { key: "n", label: "Moving average days (book example 19)", def: 19., min: 2., max: 100., step: 1. },
            ParamDef { key: "reverse", label: "'possibly reversing as well': 0 = long/flat (default), 1 = long and short", def: 0., min: 0., max: 1., step: 1. }],
        _ if id.starts_with("skdab_") || id.starts_with("skfts_") => vec![],
        _ => return None,
    })
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, _p: &Params) -> Option<Vec<Trade>> {
    if id == "skfts_ma19_close" { return ma_close_cross(x, from, to, _p); }
    if id != "skdab_trend3ma" || from > to || to >= x.r.len() { return None; }
    let c = &x.c;
    let (m4, m9, m18, m10, m20, m50) = (sma(c, 4), sma(c, 9), sma(c, 18), sma(c, 10), sma(c, 20), sma(c, 50));
    let ok = |i: usize| [m4[i], m9[i], m18[i], m10[i], m20[i], m50[i]].iter().all(|v| v.is_finite()) && i >= 1 && m50[i - 1].is_finite();
    let trend = |i: usize| -> i8 {
        if c[i] > m50[i] && m50[i] > m50[i - 1] { 1 } else if c[i] < m50[i] && m50[i] < m50[i - 1] { -1 } else { 0 }
    };
    let long_term = |i: usize, d: f64| if d > 0. { c[i] > m10[i] && m10[i] > m20[i] && m20[i] > m50[i] } else { c[i] < m10[i] && m10[i] < m20[i] && m20[i] < m50[i] };
    let short_term = |i: usize, d: f64| if d > 0. { c[i] > m4[i] && m4[i] > m9[i] && m9[i] > m18[i] } else { c[i] < m4[i] && m4[i] < m9[i] && m9[i] < m18[i] };
    let (mut out, mut cur, mut reentry) = (vec![], None::<Trade>, 0.0_f64);
    for i in from.max(1)..=to {
        if !ok(i) { continue; }
        let t = trend(i) as f64;
        if reentry != 0. && t != reentry { reentry = 0.; }   // a re-entry needs the trend unchanged since the stop-out
        if let Some(mut tr) = cur.take() {
            if short_term(i, -tr.dir) {
                tr.exit_i = i; tr.exit_px = c[i]; tr.signal_i = i; tr.reason = "short-term signal against the position (4/9/18)".into();
                reentry = if t == tr.dir { tr.dir } else { 0. };
                out.push(tr);
            } else {
                cur = Some(tr);
            }
            continue;
        }
        if t == 0. { continue; }
        if long_term(i, t) || (reentry == t && short_term(i, t)) {
            cur = Some(Trade { dir: t, legs: vec![Leg { i, px: c[i], w: 1. }], exit_i: i, exit_px: c[i], reason: String::new(), entry_kind: "close", signal_i: i, open: false, exits: vec![] });
            reentry = 0.;
        }
    }
    if let Some(mut tr) = cur { tr.exit_i = to; tr.exit_px = c[to]; tr.signal_i = to; tr.reason = "open".into(); tr.open = true; out.push(tr); }
    Some(out)
}

/// Kroll p.97: buy when the close is above the n-day simple average, sell when it is below (reverse = 1: also short). Fills at
/// the signal day's close. Close vs average compared exactly on the printed decimals (n x close vs the window sum) where the
/// series is decimal-exact, as connors::exact_avg_vs; floats otherwise.
fn ma_close_cross(x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d);
    let (n, rev) = (g("n", 19.), g("reverse", 0.));
    if from > to || to >= x.r.len() || !(2.0..=100.0).contains(&n) || n.fract() != 0.0 || !(rev == 0.0 || rev == 1.0) { return None; }
    if p.keys().any(|k| k != "n" && k != "reverse") { return None; }
    let (n, c) = (n as usize, &x.c);
    let m = crate::connors::sma(c, n);
    let ex = crate::connors::ExactSeries::new(&[c]);
    let sums = ex.as_ref().map(|e| e.sums(0, n));
    // +1 close above, -1 below, 0 equal or no average yet
    let side = |i: usize| -> i8 {
        let o = match (&ex, &sums) { (Some(e), Some(s)) => crate::connors::exact_avg_vs(e, s, n, 0, i).map(|o| o.reverse()), _ => c[i].partial_cmp(&m[i]).filter(|_| m[i].is_finite()) };
        match o { Some(std::cmp::Ordering::Greater) => 1, Some(std::cmp::Ordering::Less) => -1, _ => 0 }
    };
    let (mut out, mut cur) = (vec![], None::<Trade>);
    for i in from..=to {
        let s = side(i) as f64;
        if s == 0.0 { continue; }
        if let Some(mut t) = cur.take() {
            if t.dir == s { cur = Some(t); continue; }
            t.exit_i = i; t.exit_px = c[i]; t.signal_i = i; t.reason = if s > 0.0 { "close above the average".into() } else { "close below the average".into() };
            out.push(t);
        }
        if s > 0.0 || rev == 1.0 {
            cur = Some(Trade { dir: s, legs: vec![Leg { i, px: c[i], w: 1. }], exit_i: i, exit_px: c[i], reason: String::new(), entry_kind: "close", signal_i: i, open: false, exits: vec![] });
        }
    }
    if let Some(mut t) = cur { t.exit_i = to; t.exit_px = c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true; out.push(t); }
    Some(out)
}

#[cfg(test)]
mod k74_tests {
    use super::*;
    use crate::Row;
    #[test]
    fn kroll_19_day_close_cross_long_flat_and_reversing() {
        // 3-day average for a short check: closes 10,10,10 (avg 10, equal: nothing), 11 (avg 10.33: buy), 9 (avg 10: sell), 12 (buy)
        let c = [10., 10., 10., 11., 9., 12.];
        let r: Vec<Row> = c.iter().enumerate().map(|(i, &v)| Row { ticker: "T".into(), date: format!("2020-01-{:02}", i + 1), open: v, high: v, low: v, close: v, volume: None }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let p: Params = [("n".to_string(), 3.0)].into_iter().collect();
        let t = run("skfts_ma19_close", &x, 0, 5, &p).unwrap();
        assert_eq!(t.iter().map(|t| (t.dir, t.legs[0].i, t.exit_i, t.open)).collect::<Vec<_>>(), vec![(1., 3, 4, false), (1., 5, 5, true)]);
        let mut p2 = p.clone(); p2.insert("reverse".into(), 1.0);
        let t = run("skfts_ma19_close", &x, 0, 5, &p2).unwrap();
        assert_eq!(t.iter().map(|t| (t.dir, t.legs[0].i, t.exit_i)).collect::<Vec<_>>(), vec![(1., 3, 4), (-1., 4, 5), (1., 5, 5)]);
        assert_eq!(params("skfts_ma19_close").unwrap()[0].def, 19.0);
    }
}

// Lane K74 (11 Nov 2026, R4 #277 (2)): Kroll on Futures Trading Strategy p.97, the 19-day close cross.
pub const ADMISSION_SKFTS: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "skfts" && REVIEWED_SKFTS.contains(&def.id),
    label: "Stanley Kroll gave this 19-day moving-average close cross as an example for futures, with no test or results; runs on indices, ETFs and stocks, US and India, are a declared application, not the author's test.",
    basis: "Stanley Kroll, Kroll on Futures Trading Strategy (Dow Jones-Irwin 1988), Ch.12 p.97: 'using a 19-day m.a., you would buy when the price closes above the 19-day m.a. and sell (possibly reversing as well) when the price closes below'; fills at the close (declared); decision #277 (2); independent check authors/titles-1030/K74w (k74_daily.py).",
};

// Instrument admission: Kroll 2026-10-24 block, moved from instrument_policy.rs on 31 Oct 2026 (lane ST).
// Admission only for the module REVIEWED lists (authors/kroll/admission/SOURCE-CONTRACT.md); kept in one block to merge.
// Runs on actual indices, ETFs and stocks, US and India (index_etf_or_stock, nifty50 basket, declared application).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| matches!(def.book,"skdab") && (crate::books::skdab::reviewed(def.id)),
    label: "Stanley Kroll illustrated this strategy on the NIKKEI and futures; runs on indices, ETFs and stocks, US and India, are a declared application, not the author's test. Cash-index shorts are hypothetical exposure.",
    basis: "Stanley Kroll, Dragons and Bulls (Addison-Wesley 1994), Ch.11 Long-term versus Short-term Trading (p.59-61): 50-day average trend filter, 10/20/50-day entries, 4/9/18-day stop-outs and re-entries. Reviewed in authors/kroll/admission/SOURCE-CONTRACT.md with independent chronological ledgers; fills at the close; no costs.",
};
