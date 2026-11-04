//! EP4 (lane K51, decision #223 (1)): a second instrument ("Data2") aligned to the traded instrument's (Data1) bars.
//! Same-date join on Data1's calendar; Data2 bars on dates Data1 lacks drop out of the aligned view. A Data1 date with no
//! Data2 bar carries Data2's last real bar (TradeStation/MultiCharts Data2, MetaStock Security(), AmiBroker Foreign():
//! E4-DESIGN.md section 2). Nothing before Data2's first bar or after its last; a gap longer than CAP Data1 bars is missing
//! data, not a fill; more than 5% carried bars in a run's window withholds the result (`carried_issue`).
#![allow(dead_code)] // exact / padded_ab / sym: the EP4 API the K52 cards call (decision #223 plan)
use crate::{connors::Ctx, readiness::Issue, Row};
use std::collections::BTreeMap;

/// Longest run of Data1 bars a Data2 value may be carried over (#223 (1)).
pub const CAP: usize = 5;

#[derive(Clone, Copy)]
pub enum Field { O, H, L, C, V }
fn val(r: &Row, f: Field) -> f64 {
    match f { Field::O => r.open, Field::H => r.high, Field::L => r.low, Field::C => r.close, Field::V => r.volume.unwrap_or(f64::NAN) }
}

pub struct Second<'a> {
    pub sym: &'a str,
    pub rows: &'a [Row],
    /// Data1 bar i -> the last Data2 bar dated <= date(i), None before Data2's start, after its end and inside a gap > CAP
    pub at: Vec<Option<usize>>,
    /// Data2 has a bar on exactly date(i)
    pub fresh: Vec<bool>,
}

/// The join itself (both inputs in date order, dates YYYY-MM-DD).
pub fn align<'a>(sym: &'a str, d1: &[Row], rows: &'a [Row]) -> Second<'a> {
    let n = d1.len();
    let (mut at, mut fresh, mut more) = (vec![None; n], vec![false; n], vec![false; n]);
    let mut j = 0;
    for (i, r) in d1.iter().enumerate() {
        while j < rows.len() && rows[j].date <= r.date { j += 1; }
        at[i] = j.checked_sub(1);
        fresh[i] = at[i].is_some_and(|k| rows[k].date == r.date);
        more[i] = j < rows.len(); // Data2 still has a later bar: a hole, not its end
    }
    // a gap = consecutive non-fresh Data1 bars showing the same Data2 bar (a Data2-only date starts a new gap)
    let mut i = 0;
    while i < n {
        if fresh[i] { i += 1; continue; }
        let s = i;
        while i < n && !fresh[i] && at[i] == at[s] { i += 1; }
        if i - s > CAP || !more[i - 1] { at[s..i].iter_mut().for_each(|a| *a = None); }
    }
    Second { sym, rows, at, fresh }
}

impl Second<'_> {
    /// NaN on every Data1 date without its own Data2 bar (AmiBroker fixup 0).
    pub fn exact(&self, f: Field) -> Vec<f64> {
        self.at.iter().zip(&self.fresh).map(|(a, &fr)| a.filter(|_| fr).map_or(f64::NAN, |k| val(&self.rows[k], f))).collect()
    }
    /// TradeStation / MetaStock: a missing date shows the previous Data2 bar's fields.
    pub fn carried(&self, f: Field) -> Vec<f64> {
        self.at.iter().map(|a| a.map_or(f64::NAN, |k| val(&self.rows[k], f))).collect()
    }
    /// AmiBroker fixup 1: a padded bar has O = H = L = C = the previous close and V = 0.
    pub fn padded_ab(&self, f: Field) -> Vec<f64> {
        self.at.iter().zip(&self.fresh).map(|(a, &fr)| match (a, f) {
            (None, _) => f64::NAN,
            (Some(k), _) if fr => val(&self.rows[*k], f),
            (Some(_), Field::V) => 0.,
            (Some(k), _) => self.rows[*k].close,
        }).collect()
    }
    /// An indicator "of data2" (EasyLanguage Reference pp.47-48): computed on Data2's own bars, then aligned (carried).
    pub fn own(&self, ind: impl Fn(&[Row]) -> Vec<f64>) -> Vec<f64> {
        let v = ind(self.rows);
        self.at.iter().map(|a| a.map_or(f64::NAN, |k| v[k])).collect()
    }
    /// Data1 bars in from..=to that show a carried (not fresh) Data2 value.
    pub fn carried_bars(&self, from: usize, to: usize) -> usize {
        (from..=to).filter(|&i| self.at.get(i).is_some_and(|a| a.is_some()) && !self.fresh[i]).count()
    }
}

fn market(x: &Ctx) -> Option<crate::baskets::Market> {
    x.native_market.or_else(|| x.r.first().and_then(|r| crate::preloaded_market(&r.ticker)))
}
/// SPX for US instruments, NIFTY 50 for Indian ones; None on the benchmark itself (#223 (7): never run against itself).
pub fn benchmark(x: &Ctx) -> Option<&'static str> {
    let b = match market(x)? { crate::baskets::Market::UnitedStates => "SPX", crate::baskets::Market::India => "NIFTY 50" };
    (x.r.first()?.ticker != b).then_some(b)
}
/// STR_DATA2 = the index a market is compared with (E4.tsv strmm_rs1 / mpts_rs rows): NSE sector and mid-cap indices -> NIFTY 50;
/// the US equity index family and its ETFs -> SPX; a stock -> its market's benchmark. Other ETFs (country, bond, gold) have none.
pub fn parent(x: &Ctx) -> Option<&'static str> {
    if x.native_market.is_some() { return benchmark(x); }
    match x.r.first()?.ticker.as_str() {
        "NIFTYBANK" | "NIFTY BANK" | "NIFTYIT" | "NIFTY IT" | "NIFTYMIDCAP100" | "NIFTY MIDCAP 100" | "NIFTYMIDCAPSELECT" => Some("NIFTY 50"),
        "NDX" | "DJI" | "MDY" | "IWM" | "QQQ" | "SPY" | "DIA" | "SMH" | "XLB" | "XLE" | "XLF" | "XLI" | "XLK" | "XLP" | "XLU" | "XLV" | "XLY" => Some("SPX"),
        _ => None,
    }
}
/// The five markets of Stridsman's index version (Ch 11-12): S&P 500, NASDAQ 100, DJIA, S&P 400 MidCap and Russell 2000 with
/// MDY / IWM as labelled signal stand-ins (#223 (2)); India: the five NSE indices (labelled instrument swap).
const US5: [&str; 5] = ["SPX", "NDX", "DJI", "MDY", "IWM"];
const IN5: [&str; 5] = ["NIFTY 50", "NIFTYBANK", "NIFTYIT", "NIFTYMIDCAP100", "NIFTYMIDCAPSELECT"];
/// XMKT.C2..C5 of a five-market member: the other four, in the order above.
pub fn xmkt(ticker: &str) -> Option<Vec<&'static str>> {
    let set = [US5, IN5].into_iter().find(|s| s.contains(&ticker))?;
    Some(set.into_iter().filter(|s| *s != ticker).collect())
}

enum Kind { Exact, Close, Hv100 }
/// The series names the already-coded modules read (E4.tsv), with the Data2 symbol each one is built from.
fn legacy(x: &Ctx) -> Vec<(String, &'static str, Kind)> {
    let Some(t) = x.r.first().map(|r| r.ticker.as_str()) else { return vec![] };
    let us = market(x) == Some(crate::baskets::Market::UnitedStates);
    let mut v = vec![];
    // hfm Ch.6 NDX-SPX (unchanged since K34): exact same-date closes, NaN holes, also on NDX/SPX themselves
    if us { v.push(("LEADER_CLOSE".into(), "NDX", Kind::Exact)); v.push(("FOLLOWER_CLOSE".into(), "SPX", Kind::Exact)); }
    if us && t != "SPX" { v.push(("SPX_CLOSE".into(), "SPX", Kind::Close)); } // mkits_rs (Ch.16); India: cross-market, later (#223 (6))
    if let Some(p) = parent(x) { v.push(("STR_DATA2".into(), p, Kind::Close)); }
    if let Some(b) = benchmark(x) { v.push(("BENCH_HV100".into(), b, Kind::Hv100)); } // prap1: HV of the S&P 500 symbol's own bars (AFL)
    if x.native_market.is_none() {
        for (k, s) in xmkt(t).unwrap_or_default().into_iter().enumerate() { v.push((format!("XMKT.C{}", k + 2), s, Kind::Close)); }
    }
    v
}

/// Names only EP4 builds: always from the engine's price file, so a same-named extra_data file cannot bypass the
/// benchmark-against-itself rule or the 5% count (hfm's LEADER/FOLLOWER keep their old extra_data precedence).
const OWN: [&str; 7] = ["SPX_CLOSE", "STR_DATA2", "BENCH_HV100", "XMKT.C2", "XMKT.C3", "XMKT.C4", "XMKT.C5"];
/// Adds the legacy series to a context built from `data` (the engine's own price file). A Data2 symbol the file does not
/// hold adds nothing (the rule reports needs_data).
pub fn inject(ctx: &mut Ctx, data: &BTreeMap<String, Vec<Row>>) {
    for name in OWN { ctx.extra.remove(name); }
    for (name, sym, kind) in legacy(ctx) {
        if ctx.extra.contains_key(&name) { continue; }
        let Some(rows) = data.get(sym) else { continue };
        let s = align(sym, ctx.r, rows);
        let v = match kind {
            Kind::Exact => s.exact(Field::C),
            Kind::Close => s.carried(Field::C),
            Kind::Hv100 => s.own(|r| crate::connors::hv_ddof(&r.iter().map(|b| b.close).collect::<Vec<_>>(), 100, 0.)),
        };
        ctx.extra.insert(name, v);
    }
}

/// The second series a run reads when a parameter picks it (readiness checks exactly these): nftm_accutrack p.xcol picks
/// XMKT.C2..C5; Stridsman's p.markets = 4 drops XMKT.C5.
pub fn param_series(id: &str, p: &crate::connors::Params, required: &mut Vec<&'static str>) {
    const X: [&str; 4] = ["XMKT.C2", "XMKT.C3", "XMKT.C4", "XMKT.C5"];
    match id {
        "nftm_accutrack" => { *required = vec![X[(p.get("xcol").copied().unwrap_or(2.).round().clamp(2., 5.) as usize) - 2]]; }
        "strmm_rsbands" | "strmm_rotation" if p.get("markets").is_some_and(|v| (v.round().max(1.) as usize).clamp(4, 5) == 4) => required.retain(|n| *n != X[3]), // as strmm.rs u(): round
        _ => {}
    }
}

/// #223 (1): withheld when more than 5% of the window's bars carry a Data2 value the rule reads.
pub fn carried_issue(x: &Ctx, names: &[&str], from: usize, to: usize) -> Option<Issue> {
    let data = crate::books::rotation::PRELOADED.get()?;
    if to < from { return None; }
    let n = to - from + 1;
    legacy(x).into_iter().filter(|(name, _, kind)| !matches!(kind, Kind::Exact) && names.contains(&name.as_str())).find_map(|(name, sym, _)| {
        let k = align(sym, x.r, data.get(sym)?).carried_bars(from, to);
        (k * 20 > n).then(|| Issue { status: "needs_data", note: format!(
            "{name} ({sym}) has no bar of its own on {k} of the {n} trading days in this period, so its last value is carried on more than 5% of the bars (decision #223); the result is withheld. Choose a period in which {sym} trades on the same days.") })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn rows(t: &str, days: &[u32]) -> Vec<Row> {
        days.iter().map(|&d| Row { ticker: t.into(), date: format!("2024-01-{d:02}"), open: d as f64 + 0.1, high: d as f64 + 0.5,
            low: d as f64 - 0.5, close: d as f64, volume: Some(1000. + d as f64) }).collect()
    }
    #[test]
    fn a_short_gap_is_carried_with_each_platforms_fields() {
        let d1 = rows("A", &[2, 3, 4, 5, 8]);
        let d2 = rows("B", &[2, 4, 6, 8]); // 3 missing (holiday on B), 5 missing; 6 only on B (dropped from the view)
        let s = align("B", &d1, &d2);
        assert_eq!(s.fresh, vec![true, false, true, false, true]);
        assert_eq!(s.at, vec![Some(0), Some(0), Some(1), Some(1), Some(3)]);
        assert_eq!(s.exact(Field::C)[..3].iter().map(|v| v.is_nan()).collect::<Vec<_>>(), vec![false, true, false]);
        assert_eq!(s.carried(Field::C), vec![2., 2., 4., 4., 8.]);
        assert_eq!(s.carried(Field::O)[1], 2.1); // the previous bar's own open
        assert_eq!((s.padded_ab(Field::O)[1], s.padded_ab(Field::H)[1], s.padded_ab(Field::V)[1]), (2., 2., 0.));
        assert_eq!(s.padded_ab(Field::V)[0], 1002.);
        assert_eq!(s.carried_bars(0, 4), 2);
        assert_eq!(s.carried_bars(2, 2), 0);
    }
    #[test]
    fn a_gap_over_five_bars_is_missing_not_carried() {
        let d1 = rows("A", &[2, 3, 4, 5, 8, 9, 10, 11, 12]);
        let b1 = rows("B", &[2, 3, 10, 11, 12]);
        let s = align("B", &d1, &b1); // 4,5,8,9: 4 bars carried
        assert_eq!(s.carried(Field::C)[2..6], [3., 3., 3., 3.]);
        let b2 = rows("B", &[2, 11, 12]);
        let s = align("B", &d1, &b2); // 3..10: 6 bars > CAP
        assert!(s.carried(Field::C)[1..7].iter().all(|v| v.is_nan()));
        assert_eq!(s.carried(Field::C)[7], 11.);
        assert_eq!(s.carried_bars(0, 8), 0);
    }
    #[test]
    fn nothing_before_the_first_or_after_the_last_bar() {
        let d1 = rows("A", &[2, 3, 4, 5, 8]);
        let b3 = rows("B", &[3, 4]);
        let s = align("B", &d1, &b3);
        let c = s.carried(Field::C);
        assert!(c[0].is_nan() && c[3].is_nan() && c[4].is_nan(), "{c:?}");
        assert_eq!(c[1..3], [3., 4.]);
        assert!(align("B", &d1, &[]).carried(Field::C).iter().all(|v| v.is_nan()));
        assert_eq!(align("B", &[], &[]).carried_bars(0, 0), 0);
        // Data2 starts on a date Data1 lacks (4): Data1's 5 carries it; a Data2-only date (9) inside a Data1 run starts a new gap
        let d1 = rows("A", &[2, 3, 5, 8, 10, 11, 12, 15, 16, 17, 18, 19]);
        let b9 = rows("B", &[4, 8, 9, 19]);
        let c = align("B", &d1, &b9).carried(Field::C);
        assert!(c[0].is_nan() && c[1].is_nan(), "{c:?}");
        assert_eq!(c[2..4], [4., 8.]);
        assert!(c[4..11].iter().all(|v| v.is_nan()), "{c:?}"); // 10..18 show bar 9 for 7 bars > CAP
        assert_eq!(c[11], 19.);
    }
    #[test]
    fn own_bars_indicator_differs_from_one_on_the_aligned_series() {
        // Data2 trades on a day Data1 lacks (6): a 2-bar change "of data2" sees 4->6 on day 8; on Data1's dates it would be 4->8
        let d1 = rows("A", &[2, 4, 8]);
        let b4 = rows("B", &[2, 4, 6, 8]);
        let s = align("B", &d1, &b4);
        let chg = |r: &[Row]| (0..r.len()).map(|k| if k == 0 { f64::NAN } else { r[k].close - r[k - 1].close }).collect::<Vec<_>>();
        assert_eq!(s.own(chg)[2], 2.);
        let al = s.carried(Field::C);
        assert_eq!(al[2] - al[1], 4.);
    }
    #[test]
    fn param_series_follow_the_modules_parameter_reading() {
        let req = |id: &str, k: &str, v: f64| { let mut r = vec!["XMKT.C2", "XMKT.C3", "XMKT.C4", "XMKT.C5"]; param_series(id, &crate::connors::Params::from([(k.to_string(), v)]), &mut r); r };
        assert_eq!(req("strmm_rsbands", "markets", 4.), ["XMKT.C2", "XMKT.C3", "XMKT.C4"]);
        assert_eq!(req("strmm_rotation", "markets", 4.5).len(), 4); // strmm.rs rounds 4.5 to 5
        assert_eq!(req("strmm_rsbands", "markets", 4.4).len(), 3);
        assert_eq!(req("nftm_accutrack", "xcol", 3.), ["XMKT.C3"]);
        assert_eq!(req("nftm_accutrack", "xcol", 9.), ["XMKT.C5"]);
    }
    #[test]
    fn legacy_names_follow_the_e4_map() {
        let days: Vec<u32> = (1..=28).collect();
        let mk = |t: &str| rows(t, &days);
        let (spx, iwm, n50, bank) = (mk("SPX"), mk("IWM"), mk("NIFTY 50"), mk("NIFTYBANK"));
        let names = |r: &Vec<Row>| legacy(&Ctx::new(r, &[], &[])).into_iter().map(|(n, s, _)| format!("{n}={s}")).collect::<Vec<_>>();
        assert_eq!(names(&spx), ["LEADER_CLOSE=NDX", "FOLLOWER_CLOSE=SPX", "XMKT.C2=NDX", "XMKT.C3=DJI", "XMKT.C4=MDY", "XMKT.C5=IWM"]);
        assert_eq!(names(&iwm), ["LEADER_CLOSE=NDX", "FOLLOWER_CLOSE=SPX", "SPX_CLOSE=SPX", "STR_DATA2=SPX", "BENCH_HV100=SPX", "XMKT.C2=SPX", "XMKT.C3=NDX", "XMKT.C4=DJI", "XMKT.C5=MDY"]);
        assert_eq!(names(&n50), ["XMKT.C2=NIFTYBANK", "XMKT.C3=NIFTYIT", "XMKT.C4=NIFTYMIDCAP100", "XMKT.C5=NIFTYMIDCAPSELECT"]);
        assert_eq!(names(&bank), ["STR_DATA2=NIFTY 50", "BENCH_HV100=NIFTY 50", "XMKT.C2=NIFTY 50", "XMKT.C3=NIFTYIT", "XMKT.C4=NIFTYMIDCAP100", "XMKT.C5=NIFTYMIDCAPSELECT"]);
        let aapl = mk("AAPL");
        let mut x = Ctx::new(&aapl, &[], &[]);
        x.native_market = Some(crate::baskets::Market::UnitedStates);
        assert_eq!(legacy(&x).into_iter().map(|(n, s, _)| format!("{n}={s}")).collect::<Vec<_>>(),
            ["LEADER_CLOSE=NDX", "FOLLOWER_CLOSE=SPX", "SPX_CLOSE=SPX", "STR_DATA2=SPX", "BENCH_HV100=SPX"]);
        x.native_market = Some(crate::baskets::Market::India);
        assert_eq!(legacy(&x).into_iter().map(|(n, s, _)| format!("{n}={s}")).collect::<Vec<_>>(), ["STR_DATA2=NIFTY 50", "BENCH_HV100=NIFTY 50"]);
    }
    #[test]
    fn inject_keeps_an_extra_data_file_and_uses_own_bar_hv() {
        let d: Vec<u32> = (1..=28).collect();
        let (a, mut b) = (rows("IWM", &d), rows("SPX", &d));
        for (k, r) in b.iter_mut().enumerate() { r.close = 100. * (1. + 0.01 * ((k * 7 % 5) as f64)); }
        let data = BTreeMap::from([("SPX".to_string(), b.clone())]);
        let mut x = Ctx::new(&a, &[], &[]);
        x.extra.insert("STR_DATA2".into(), vec![7.; a.len()]);
        x.extra.insert("FOLLOWER_CLOSE".into(), vec![7.; a.len()]);
        inject(&mut x, &data);
        assert_eq!(x.extra["STR_DATA2"][0], b[0].close); // EP4 names: always the engine's own series
        assert_eq!(x.extra["FOLLOWER_CLOSE"][0], 7.); // hfm keeps its extra_data precedence
        assert_eq!(x.extra["SPX_CLOSE"], b.iter().map(|r| r.close).collect::<Vec<_>>());
        assert!(x.extra["BENCH_HV100"].iter().all(|v| v.is_nan())); // 28 bars < 100: no HV yet, nothing invented
        assert!(!x.extra.contains_key("LEADER_CLOSE")); // NDX not in this file
    }
}
