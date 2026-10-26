//! Victor Sperandeo, "Trader Vic on Commodities: What's Unknown, Misunderstood, and Too Good to Be True"
//! (Wiley, 2008). Mechanical content: Ch. 6 S&P 500 moving-average studies (Tables 6.3-6.5, pp. 52-64) and
//! the S&P DTI position rule (Appendix A, pp. 155, 171-174). Ch. 4 (2B) is chart judgement.
//! Source and checks: library/svtvc/.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "svtvc";
/// Admitted ids (authors/sperandeo/admission/SOURCE-CONTRACT.md).
pub const REVIEWED: &[&str] = &["svtvc_ma150_t1", "svtvc_ma200_t1", "svtvc_ma250_t1", "svtvc_ma150_t2", "svtvc_ma200_t2", "svtvc_ma250_t2", "svtvc_lsm7"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const MA_NOTE: &str = "Book test: S&P 500 cash index, Aug/Oct 1928 - Dec 2006, price return, no costs (Tables 6.3-6.5, pp. 52-64). \
Simple MA of daily closes (\"simple moving average tests ... without any weighting\", p. 51). Sloping up/down = today's MA above/below yesterday's MA \
[book silent: one-day slope]. Trades at the close of the signal day [fitted: reproduces the book's yearly returns on SPX 1985-2006, see README]. \
Between the long and flat conditions the position is kept. Flat at the start of the selected window. India: same rule on NIFTY 50 (instrument swap only). \
Test #3 (long above / flat below) is not repeated here: it is the vault's apta_ma200_dji with p.ma=150/200/250.";

const T1: &[&str] = &[
    "F|\"SP MA Test #1\" - \"simple moving average tests: three each for 150 days, 200 days, and 250 days without any weighting\" (p. 51)",
    "E|\"Positioned long when the price is above the MA and the MA is sloping up\" (Tables 6.3-6.5 note a, pp. 52-64)",
    "X|\"positioned flat when price is below the MA and the MA is sloping down\" (note a)",
];
const T2: &[&str] = &[
    "F|\"SP MA Test #2\" - simple moving average, no weighting (p. 51)",
    "E|\"Positioned long when price is above the MA\" (Tables 6.3-6.5 note b, pp. 52-64)",
    "X|\"positioned flat when price is below MA and the MA is sloping down\" (note b)",
];
const LSM: &[&str] = &[
    "F|\"Position is determined on the second to the last business day of the month (defined as the position determination day or PDD)\" (App. A p. 155)",
    "E|\"Long positions are tracked when a component's current price input is equal to or greater than an exponential average of the past seven price inputs\" (p. 155)",
    "E|\"Short positions are tracked when a component's current price input is less than an exponential average of the past seven price inputs\" (p. 155)",
    "F|Average = (CMSR[t-6] + 1.6 CMSR[t-5] + 1.6^2 CMSR[t-4] + ... + 1.6^6 CMSR[t]) / 43.07258, CMSR = cumulative return at the PDDs (pp. 173-174; Table A.11 p. 171)",
    "X|\"After the market closes on the trade activity date (TAD), active S&P DTI contracts are replaced ... The TAD is randomly selected and is one of the first five business days of each month\" (p. 155)",
    "X|\"Track a flat (zero weight) position for the energy sector when a short position is indicated\" (p. 155) - available as short=0",
];

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, dir, status, rules, notes| StrategyDef {
        id, name, group, session: 0, dir, status, needs_volume: false, needs_vix: false, needs_series: &[],
        book: BOOK, rules, notes, claim: None,
    };
    use Status::*;
    let g6 = "Ch. 6 A Challenge to the Random Walk Theory";
    vec![
        d("svtvc_ma150_t1", "150-day MA Test #1 (long above & rising, flat below & falling)", g6, "Long", Live, T1, MA_NOTE),
        d("svtvc_ma200_t1", "200-day MA Test #1 (long above & rising, flat below & falling)", g6, "Long", Live, T1, MA_NOTE),
        d("svtvc_ma250_t1", "250-day MA Test #1 (long above & rising, flat below & falling)", g6, "Long", Live, T1, MA_NOTE),
        d("svtvc_ma150_t2", "150-day MA Test #2 (long above, flat below & falling)", g6, "Long", Live, T2, MA_NOTE),
        d("svtvc_ma200_t2", "200-day MA Test #2 (long above, flat below & falling)", g6, "Long", Live, T2, MA_NOTE),
        d("svtvc_ma250_t2", "250-day MA Test #2 (long above, flat below & falling)", g6, "Long", Live, T2, MA_NOTE),
        d("svtvc_lsm7", "S&P DTI / LSM 7-month 1.6-weighted trend rule (single market)", "Appendix A S&P DTI Methodology", "Long/Short", Live, LSM,
          "The book applies this per component/sector of a futures basket (LSM = same algorithm, components equally weighted, p. 51). Here it runs on one index \
(instrument swap; the DTI holds no equity-index futures). Price input = the index close (cumulative return is a constant multiple of price, so the comparison is identical). \
Weights 1.6^0..1.6^6 oldest to newest over the current and previous six PDDs. TAD: the book draws it at random from business days 1-5; default tad=1 \
[book silent on a fixed day - necessary assumption], tad=2..5 selectable. Entry/exit at the TAD close. short=1 (book rule for all non-energy sectors) \
or short=0 (book's energy-sector rule: flat instead of short). No published S&P 500 result for this rule; Table 6.1/6.2 results are futures baskets (see svtvc_lsm_futures)."),
        d("svtvc_lsm_futures", "LSM futures baskets 1929-1934 and 1961-1984", "Ch. 6 A Challenge to the Random Walk Theory", "Long/Short",
          NeedsData("daily/monthly futures prices for cocoa, cotton, wheat, coffee, corn, sugar, rubber (1929-34) and the CRB-era LSM basket (1961-84); the vault has no futures"),
          &["E|\"Simulated Results, LSM Unweighted 7-month 1.6 weighted (S&P DTI Algorithm)\" on \"Futures on: Cocoa, Cotton, Wheat, Coffee, Corn, Sugar, and Rubber\" (Table 6.1 p. 50)",
            "E|\"The LSM uses the same algorithm as the S&P DTI. However, the individual components are equally weighted and are not formed into sectors\" (p. 51)"],
          "Book: 10.28% compounded price return 1929-1934 (Table 6.1); 11.52% price return 1961-2006 (Table 6.2). Per-market rule = svtvc_lsm7."),
        d("svtvc_dti", "S&P Diversified Trends Indicator (24 futures, 14 sectors)", "Appendix A S&P DTI Methodology", "Long/Short",
          NeedsData("24 commodity and financial futures with roll schedule (Table A.7), sector/component weights (Table A.1), T-bill rates; the vault has no futures"),
          &["F|24 components, 14 sectors (8 financial, 6 commodity), 50/50 commodities/financials; sectors rebalanced monthly, components annually (pp. 148-156)",
            "E|Sector long/short by the 7-month 1.6-weighted rule (p. 155, pp. 173-174); energy flat instead of short; energy weight redistributed",
            "X|Position changes after the close on the TAD (random business day 1-5 of the month) (p. 155)"],
          "Same per-sector rule as svtvc_lsm7; the index itself needs the futures basket."),
        d("svtvc_cti_fti", "S&P Commodity Trends / Financial Trends Indicators", "Ch. 12 S&P DTI Subindexes", "Long/Short",
          NeedsData("16 commodity futures (CTI) / financial futures (FTI) with roll schedule; the vault has no futures"),
          &["E|CTI: 16 commodity futures in six sectors, each long or short (energy flat) by the DTI rule (App. A pp. 173-174)"], "Subindexes of svtvc_dti."),
        d("svtvc_2b", "2B rule (classic rule revisited)", "Ch. 4 2B or Not 2B", "Long/Short",
          NotTestable("the 'previous high/low' is a minor/intermediate/major swing chosen on the chart; the profit exit is judgement (half 'once I saw it approach a level of support', the rest on 'a new 2B sell signal or another technical sell indication', p. 33); only the stop is exact"),
          &["E|\"In an uptrend, if a higher high is made but fails to carry through, and then prices drop below the previous high, then the trend is apt to reverse\" (p. 25)",
            "X|\"If the market trades back above the new high, or below the new low, you need to admit defeat immediately and take the loss\" (p. 25)",
            "X|\"half the position as a quick profit ... once I saw it approach a level of support. The other half I could let ride\" (p. 33)"],
          "No complete exit; not coded."),
        d("svtvc_123", "1-2-3 change of trend", "Ch. 4 2B or Not 2B", "Long/Short",
          NotTestable("trendline drawn by eye between 'minor' lows/highs and 'short-term high/low' are chart judgement; no exit rule"),
          &["F|\"1. A trendline is broken. 2. The trend stops making higher highs ... 3. Prices go above a previous short-term high\" (p. 23)"],
          "Not coded."),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let f = ("svtvc_ma", "Sperandeo S&P moving-average studies");
    Some(match id {
        "svtvc_ma150_t1" => (f.0, f.1, "150-day · Test #1"),
        "svtvc_ma200_t1" => (f.0, f.1, "200-day · Test #1"),
        "svtvc_ma250_t1" => (f.0, f.1, "250-day · Test #1"),
        "svtvc_ma150_t2" => (f.0, f.1, "150-day · Test #2"),
        "svtvc_ma200_t2" => (f.0, f.1, "200-day · Test #2"),
        "svtvc_ma250_t2" => (f.0, f.1, "250-day · Test #2"),
        "svtvc_lsm7" => ("svtvc_dti", "S&P DTI trend rule", "Single market"),
        "svtvc_lsm_futures" => ("svtvc_dti", "S&P DTI trend rule", "LSM futures baskets · needs data"),
        "svtvc_dti" => ("svtvc_dti", "S&P DTI trend rule", "Full DTI · needs data"),
        "svtvc_cti_fti" => ("svtvc_dti", "S&P DTI trend rule", "CTI/FTI · needs data"),
        "svtvc_2b" | "svtvc_123" => ("svtvc_trend_change", "Sperandeo change of trend", "not testable"),
        _ => return None,
    })
}

fn ma_spec(id: &str) -> Option<(f64, u8)> {
    Some(match id {
        "svtvc_ma150_t1" => (150., 1), "svtvc_ma200_t1" => (200., 1), "svtvc_ma250_t1" => (250., 1),
        "svtvc_ma150_t2" => (150., 2), "svtvc_ma200_t2" => (200., 2), "svtvc_ma250_t2" => (250., 2),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    if let Some((n, _)) = ma_spec(id) {
        return Some(vec![ParamDef { key: "ma", label: "Simple moving average days", def: n, min: 50., max: 300., step: 10. }]);
    }
    match id {
        "svtvc_lsm7" => Some(vec![
            ParamDef { key: "months", label: "Price inputs in the average (current + previous PDDs)", def: 7., min: 3., max: 12., step: 1. },
            ParamDef { key: "base", label: "Exponential weight base", def: 1.6, min: 1.0, max: 2.0, step: 0.1 },
            ParamDef { key: "tad", label: "Trade activity day (business day of month, book: random 1-5)", def: 1., min: 1., max: 5., step: 1. },
            ParamDef { key: "short", label: "1 = short below the average (book), 0 = flat (book's energy rule)", def: 1., min: 0., max: 1., step: 1. },
        ]),
        _ => None,
    }
}

fn open_trade(dir: f64, i: usize, px: f64) -> Trade {
    Trade { dir, legs: vec![Leg { i, px, w: 1. }], exit_i: i, exit_px: px, reason: String::new(), entry_kind: "close", signal_i: i, open: false, exits: vec![] }
}
fn close_trade(out: &mut Vec<Trade>, cur: &mut Option<Trade>, i: usize, px: f64, why: &str) {
    if let Some(mut t) = cur.take() { t.exit_i = i; t.exit_px = px; t.signal_i = i; t.reason = why.into(); out.push(t); }
}
fn finish(out: &mut Vec<Trade>, cur: Option<Trade>, x: &Ctx, to: usize) {
    if let Some(mut t) = cur { t.exit_i = to; t.exit_px = x.c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true; out.push(t); }
}

/// Ch. 6 Tests #1/#2: long/flat state machine evaluated at each close, filled at that close.
fn run_ma(x: &Ctx, from: usize, to: usize, n: usize, test: u8) -> Vec<Trade> {
    // Exact window sums; slope from the bar entering vs the bar leaving the window (a flat MA is neither up nor down).
    let m = sma_exact(&x.c, n);
    let (mut out, mut cur) = (vec![], None::<Trade>);
    for i in from.max(n)..=to {
        if !m[i].is_finite() || !m[i - 1].is_finite() { continue; }
        let (c, up, dn) = (x.c[i], x.c[i] > x.c[i - n], x.c[i] < x.c[i - n]);
        let long = c > m[i] && (test == 2 || up);
        let flat = c < m[i] && dn;
        if cur.is_none() && long { cur = Some(open_trade(1., i, c)); }
        else if cur.is_some() && flat { close_trade(&mut out, &mut cur, i, c, "Close below the MA and the MA sloping down"); }
    }
    finish(&mut out, cur, x, to);
    out
}

/// Simple MA by exact window sums (no running-sum rounding drift at exact ties).
pub fn sma_exact(v: &[f64], n: usize) -> Vec<f64> {
    (0..v.len()).map(|i| if i + 1 >= n && n > 0 { v[i + 1 - n..=i].iter().sum::<f64>() / n as f64 } else { f64::NAN }).collect()
}

fn ym<'a>(x: &'a Ctx, i: usize) -> &'a str { &x.r[i].date[..7] }

/// App. A position rule on one market. PDD = second-to-last bar of a calendar month; the position set at a PDD
/// takes effect at the close of the tad-th bar of the next month.
fn run_lsm(x: &Ctx, from: usize, to: usize, months: usize, base: f64, tad: usize, short: bool) -> Vec<Trade> {
    let n = x.r.len();
    let mut pdd = vec![];
    for i in 0..n.saturating_sub(2) {
        if ym(x, i) == ym(x, i + 1) && ym(x, i + 1) != ym(x, i + 2) { pdd.push(i); }
    }
    let wsum: f64 = (0..months).map(|k| base.powi(k as i32)).sum();
    let (mut out, mut cur) = (vec![], None::<Trade>);
    for k in months - 1..pdd.len() {
        let p = pdd[k];
        let avg: f64 = (0..months).map(|j| base.powi((months - 1 - j) as i32) * x.c[pdd[k - j]]).sum::<f64>() / wsum;
        let want = if x.c[p] >= avg { 1. } else if short { -1. } else { 0. };
        // tad-th bar of the next month
        let mut i = p + 1;
        while i < n && ym(x, i) == ym(x, p) { i += 1; }
        let t = i + tad - 1;
        if i >= n || t >= n || ym(x, t) != ym(x, i) || t > to { continue; }
        if t < from { continue; }
        let have = cur.as_ref().map_or(0., |t| t.dir);
        if have != want {
            close_trade(&mut out, &mut cur, t, x.c[t], if want > 0. { "PDD price >= weighted average" } else { "PDD price < weighted average" });
            if want != 0. { cur = Some(open_trade(want, t, x.c[t])); cur.as_mut().unwrap().signal_i = p; }
        }
    }
    finish(&mut out, cur, x, to);
    out
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if from >= to || to >= x.r.len() { return None; }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    if let Some((_, test)) = ma_spec(id) {
        return Some(run_ma(x, from, to, g("ma").round().max(2.) as usize, test));
    }
    match id {
        "svtvc_lsm7" => Some(run_lsm(x, from, to, g("months").round().max(2.) as usize, g("base"), g("tad").round().clamp(1., 5.) as usize, g("short") >= 0.5)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn rows(v: &[(&str, f64)]) -> Vec<Row> {
        v.iter().map(|&(d, c)| Row { ticker: "T".into(), date: d.into(), open: c, high: c, low: c, close: c, volume: None }).collect()
    }
    #[test]
    fn test1_holds_between_conditions() {
        // MA(3): rising then price dips below a still-rising MA -> Test #1 stays long; then MA falls with price below -> flat
        let c = [10., 10., 10., 12., 13., 11.5, 9., 8.];
        let r: Vec<Row> = c.iter().enumerate().map(|(i, &c)| Row { ticker: "T".into(), date: format!("2001-01-{:02}", i + 1), open: c, high: c, low: c, close: c, volume: None }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let t = run_ma(&x, 0, r.len() - 1, 3, 1);
        assert_eq!(t.len(), 1);
        assert_eq!(t[0].legs[0].i, 3); // 12 > MA 10.67 and MA rising
        assert_eq!(t[0].exit_i, 6); // 9 < MA 11.17, MA falling (11.5 -> 11.17)
    }
    #[test]
    fn lsm_switches_at_tad_close() {
        // 9 months of 2 bars each... use 3 bars per month so PDD (2nd-to-last) exists
        let mut v = vec![];
        let px = [100., 101., 102., 103., 104., 105., 106., 90., 80., 70.];
        for (m, &p) in px.iter().enumerate() { for d in 1..=3 { v.push((format!("2001-{:02}-{:02}", m + 1, d), p)); } }
        let vv: Vec<(&str, f64)> = v.iter().map(|(d, p)| (d.as_str(), *p)).collect();
        let r = rows(&vv); let x = Ctx::new(&r, &r, &[]);
        let t = run_lsm(&x, 0, r.len() - 1, 7, 1.6, 1, true);
        // first PDD with 7 inputs = month 7 (106 >= avg) -> long at month 8 bar 1 (90)
        assert_eq!(t[0].dir, 1.); assert_eq!(t[0].legs[0].i, 21); assert_eq!(t[0].legs[0].px, 90.);
        // month 8 PDD price 90 < avg -> short from month 9 bar 1
        assert_eq!(t[0].exit_i, 24); assert_eq!(t[1].dir, -1.);
    }
}
