//! Nelson Freeburg, "Building High-Performance Trading Systems: What Works & What Doesn't"
//! (Live@TAG / INO workshop, TAG 2000, 27 pp. scan). Source and checks: library/nfhp/.
//! Live: the Quadram S&P system (Fig. 46) on NYSE / NSE advance-decline breadth.
//! Everything else needs currency / metals / bond futures, intermarket series or gives results only.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "nfhp";

const FX: &str = "daily Swiss franc, D-mark, British pound and Japanese yen FUTURES with contract point values ($ stops); the vault has no currency futures (index points cannot stand in for a $2,000 currency-contract stop)";
const METALS: &str = "Comex silver/gold futures, Swiss franc futures, US T-bill yield and CPI inflation (real T-bill rate); none in the vault";
const TBOND: &str = "T-Bond / 10-year note futures, LME nickel, Dow Jones Utilities, Fidelity Select Chemicals (FSCHX) and Financial (FIDSX) prices; none in the vault";
const NYSE: &[&str] = &["NYSE_ADV_DEC.Advances", "NYSE_ADV_DEC.Declines"];
const NSE: &[&str] = &["NSE_ADV_DEC.Advances", "NSE_ADV_DEC.Declines"];

const Q_ENTRY_CT: &[&str] = &[
    "F|CHAD = 5-day total of NYSE advances minus 5-day total of NYSE declines (Quadram calculations, TAG XVIII formulas p.6; HP chart Fig. 42 'CHAD')",
    "E|\"Long Entry A. CounterTrend Mode: CHAD Crosses Above 4-DMA Minus 0.8 Standard Deviation\" (Fig. 46, p.22)",
    "E|\"Short Entry A. CounterTrend Mode: CHAD Crosses Below 4-DMA Plus 0.8 Standard Deviation\" (Fig. 46, p.22)",
    "E|Entry at the next day's open: \"Countertrend Entry--Next Day Open\" (author's TAG XVIII Quadram rules, Ground-Up workbook p.6; the 2000 handout gives no order type)",
    "X|\"III. Exit A. $500 Initial Stop B. $500 Trailing Stop\" (Fig. 46). Book test: S&P 500 futures, $250/pt (Fig. 44 header) -> 2.0 index points each",
    "X|Trailing stop as the author codes it (Channel 25 TradeStation code, Ground-Up p.20): once (highest close since entry - entry) >= $500, stop at that highest close - $500",
    "X|An opposite-mode entry signal while in a trade reverses the position at the next open (TradeStation long/short entries)",
];
const Q_ENTRY_TF: &[&str] = &[
    "F|CHAD = 5-day total of NYSE advances minus 5-day total of NYSE declines (Quadram calculations, TAG XVIII formulas p.6)",
    "E|\"Long Entry B. Trend-Following Mode: CHAD Crosses Above 25-DMA Plus 0.8 Standard Deviation\" (Fig. 46, p.22)",
    "E|\"Short Entry B. Trend-Following Mode: CHAD Crosses Below 25-DMA Minus 0.8 Standard Deviation\" (Fig. 46, p.22)",
    "E|Entry at the next day's open: \"Trend-Following Entry--Next Day Open\" (TAG XVIII Quadram rules, Ground-Up p.6)",
    "X|\"III. Exit A. $500 Initial Stop B. $500 Trailing Stop\" (Fig. 46). Book test: S&P 500 futures, $250/pt (Fig. 44 header) -> 2.0 index points each",
    "X|Trailing stop as the author codes it (Channel 25 TradeStation code, Ground-Up p.20): once (highest close since entry - entry) >= $500, stop at that highest close - $500",
    "X|An opposite entry signal while in a trade reverses the position at the next open (TradeStation long/short entries)",
];
const Q_NOTE: &str = "Book (Fig. 44, p.21): S&P 500 futures 1982-2000, 2 lots, $250/pt, $150 slippage+commission: 827 trades, 40% profitable, avg $357.32, avg bars in winners 1 / losers 0, PF 1.94 - the two modes together (max 2 contracts held). Here one mode per strategy, one unit. Bands = n-day SMA of CHAD +/- 0.8 x population standard deviation (TradeStation StdDev / Bollinger, Figs 42-43 'Bollinger Bands (4, 0.8)' / '(25, 0.8)'). 'Crosses above' = CHAD > band today and CHAD <= band yesterday. Stops in index points = $ / pt_usd (S&P futures $250; same points on SPX - instrument swap only; on NIFTY the same 2-point stop is used, as the vault's lwlts precedent). The $500 initial stop is armed on the entry day (book: losers average 0 bars = stopped on the entry bar); stops fill at the stop or at the open if it gaps through. Trailing-stop formula is the author's own Channel 25 code (the Quadram page gives only '$500 Trailing Stop').";

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    let d = |id, name, group, dir, status, needs_series, rules, notes| StrategyDef {
        id, name, group, session: 0, dir, status, needs_volume: false, needs_vix: false, needs_series, book: BOOK, rules, notes, claim: None,
    };
    let q = |id, name, ns, rules, notes| StrategyDef {
        id, name, group: "Quadram S&P Futures Trading System", session: 0, dir: "Long/Short", status: Live, needs_volume: false, needs_vix: false,
        needs_series: ns, book: BOOK, rules, notes,
        claim: crate::connors::claim("827 (both modes)", "40%", "$357.32 / trade (2 lots)", "1 bar winners, 0 losers", "S&P 500 futures 1982-2000"),
    };
    vec![
        q("nfhp_quadram_ct", "Quadram countertrend mode · NYSE breadth", NYSE, Q_ENTRY_CT, Q_NOTE),
        q("nfhp_quadram_ct_nse", "Quadram countertrend mode · NSE breadth", NSE, Q_ENTRY_CT,
          "India mapping: NSE_ADV_DEC (NSE equity advances/declines) instead of NYSE, run on NIFTY indices; stop points as on SPX. Otherwise identical to nfhp_quadram_ct."),
        q("nfhp_quadram_tf", "Quadram trend-following mode · NYSE breadth", NYSE, Q_ENTRY_TF, Q_NOTE),
        q("nfhp_quadram_tf_nse", "Quadram trend-following mode · NSE breadth", NSE, Q_ENTRY_TF,
          "India mapping: NSE_ADV_DEC instead of NYSE, run on NIFTY indices. Otherwise identical to nfhp_quadram_tf."),
        d("nfhp_channel25", "Channel 25 Currency Trading System", "Channel 25 Currency System", "Long/Short", NeedsData(FX), &[],
          &["E|\"Enter Long Tomorrow on a Stop Placed at Highest High of Past 25 Days. ... Enter Short ... Lowest Low of Past 25 Days. If Order Not Filled, Adjust Stops and Place New Order for Next Session\" (Fig. 20, p.12)",
            "X|\"Protective Stop $2,000; Trailing Stop $2,000; Reversing Signal\" (Fig. 20)"],
          "Book (Figs 16-19, 1978-2000): SF 221 trades 49%, DM 178 44%, BP 259 42%, JY 208 42%. Not run: $ stops on currency contracts."),
        d("nfhp_r2_metals", "R-Squared Precious Metals System", "R-Squared Precious Metals", "Long/Short", NeedsData(METALS), &[],
          &["E|Long: Silver > 12-day SMA, real T-bill rate < 1.0%, Swiss franc > 4-day SMA; short: the reverse (Fig. 24, p.15)",
            "X|Protective stop $2,000; trailing stop $2,500 (Fig. 24)"],
          "Book: silver 1975-2000 205 trades 48%; gold (3 lots) 119 trades 47% (Figs 22-23)."),
        d("nfhp_cqiii", "CQIII Currency Trading System", "CQIII Currency System", "Long/Short", NeedsData(FX), &[],
          &["E|Long next day open: Close > 10-day MA, 10-day MA today > 10-day MA of ten days ago, Close > close of 40 days ago; short the reverse (Fig. 41, p.19)",
            "X|\"Protective Stop: $2000\" (Fig. 41)"],
          "Book (Figs 37-40, 1978-1999): SF 93 trades 44%, DM 97 46%, BP 115 33%, JY 87 54%."),
        d("nfhp_tbond_intermarket", "Intermarket T-Bond Trading System", "Intermarket T-Bond System", "Long/Short", NeedsData(TBOND), &[],
          &["E|Points: Nickel < Nickel[20]; DJUA/S&P > DJUA/S&P[32]; FSCHX/S&P < FSCHX/S&P[28]; FIDSX/S&P > FIDSX/S&P[28]; long count >= 3, short count <= 1 (Fig. 35, p.16)",
            "X|\"$2,000 Initial Stop\" (Fig. 35)"],
          "Book: T-Bond 1986-2000 190 trades 56% (Fig. 32); 10-yr note 175 trades 55% (Fig. 33). Same rules as nftm_tbond_intermarket."),
        d("nfhp_vl_swing", "Value Line Percent Swing System", "Value Line Percent Swing", "Long", NotTestable("Rule given only as table labels 'Buy on N-Percent Rally / Sell on N-Percent Pullback' (p.3): the reference point, data frequency and price basis are not stated; also needs the Value Line Arithmetic index (not in the vault)."), &[],
          &["E|Buy on N-percent rally; sell (to money market) on N-percent pullback; table of N = 2/4/6/8% since 1963 (p.3)"], "Not coded."),
        d("nfhp_results_only", "Silver system, Sparkling S&P system, sector rotation tables", "Examples / results only", "Long/Short", NotTestable("Shown only as curve-fitting examples or result tables (pp.6-9, 13); no rules are printed."), &[],
          &["E|No entry/exit rules printed"], "Not coded. AccuTrack RSI (Fig. 30) is the same formula as nftm_accutrack (coded there)."),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let q = ("nf_quadram", "Freeburg Quadram S&P system");
    Some(match id {
        "nfhp_quadram_ct" => (q.0, q.1, "Countertrend · NYSE (2000)"),
        "nfhp_quadram_ct_nse" => (q.0, q.1, "Countertrend · NSE (2000)"),
        "nfhp_quadram_tf" => (q.0, q.1, "Trend-following · NYSE (2000)"),
        "nfhp_quadram_tf_nse" => (q.0, q.1, "Trend-following · NSE (2000)"),
        "nfhp_channel25" => ("nf_channel25", "Freeburg Channel 25", "2000 · Data missing"),
        "nfhp_cqiii" => ("nf_cqiii", "Freeburg CQIII", "2000 · Data missing"),
        "nfhp_r2_metals" => ("nf_r2", "Freeburg R-Squared metals", "2000 · Data missing"),
        "nfhp_tbond_intermarket" => ("nf_tbond", "Freeburg intermarket T-Bond", "2000 · Data missing"),
        "nfhp_vl_swing" => ("nf_vlswing", "Value Line percent swing", "2000 · not testable"),
        "nfhp_results_only" => ("nf_results", "Freeburg results-only examples", "2000"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let tf = match id { "nfhp_quadram_ct" | "nfhp_quadram_ct_nse" => false, "nfhp_quadram_tf" | "nfhp_quadram_tf_nse" => true, _ => return None };
    Some(vec![
        p("chad_n", "CHAD days (totals of advances / declines)", 5., 2., 20., 1.),
        p("len", "Band moving-average days", if tf { 25. } else { 4. }, 2., 50., 1.),
        p("k", "Band width (standard deviations)", 0.8, 0.2, 3., 0.1),
        p("stop_usd", "Initial stop ($)", 500., 0., 5000., 100.),
        p("trail_usd", "Trailing stop ($)", 500., 0., 5000., 100.),
        p("pt_usd", "Dollars per index point (book: S&P futures $250)", 250., 50., 500., 50.),
        p("eds", "Initial stop armed on the entry day (1 = yes, book: losers 0 bars)", 1., 0., 1., 1.),
    ])
}

fn new_trade(dir: f64, i: usize, px: f64, sig: usize) -> Trade {
    Trade { dir, legs: vec![Leg { i, px, w: 1. }], exit_i: i, exit_px: px, reason: String::new(), entry_kind: "open", signal_i: sig, open: false, exits: vec![] }
}

/// n-day mean and population standard deviation (NaN while any input in the window is NaN).
fn bands(v: &[f64], n: usize) -> (Vec<f64>, Vec<f64>) {
    let mut m = vec![f64::NAN; v.len()];
    let mut s = vec![f64::NAN; v.len()];
    for i in n.saturating_sub(1)..v.len() {
        let w = &v[i + 1 - n..=i];
        if w.iter().all(|x| x.is_finite()) {
            let mu = w.iter().sum::<f64>() / n as f64;
            m[i] = mu;
            s[i] = (w.iter().map(|x| (x - mu) * (x - mu)).sum::<f64>() / n as f64).sqrt();
        }
    }
    (m, s)
}

pub struct Quad { pub chad_n: usize, pub len: usize, pub k: f64, pub stop: f64, pub trail: f64, pub eds: bool, pub tf: bool }

/// Signal on each bar's close: +1 long entry, -1 short entry, 0 none.
pub fn quadram_signals(a: &[f64], d: &[f64], q: &Quad) -> Vec<i8> {
    let n = a.len();
    let mut chad = vec![f64::NAN; n];
    for i in q.chad_n.saturating_sub(1)..n {
        chad[i] = (i + 1 - q.chad_n..=i).map(|j| a[j] - d[j]).sum();
    }
    let (m, s) = bands(&chad, q.len);
    let up: Vec<f64> = (0..n).map(|i| m[i] + q.k * s[i]).collect();
    let lo: Vec<f64> = (0..n).map(|i| m[i] - q.k * s[i]).collect();
    let above = |b: &[f64], i: usize| chad[i] > b[i] && chad[i - 1] <= b[i - 1];
    let below = |b: &[f64], i: usize| chad[i] < b[i] && chad[i - 1] >= b[i - 1];
    (0..n).map(|i| {
        if i == 0 { return 0; }
        let (l, sh) = if q.tf { (above(&up, i), below(&lo, i)) } else { (above(&lo, i), below(&up, i)) };
        if l { 1 } else if sh { -1 } else { 0 }
    }).collect()
}

pub fn quadram_run(x: &Ctx, from: usize, to: usize, sig: &[i8], q: &Quad) -> Vec<Trade> {
    let mut out = vec![];
    let mut cur: Option<Trade> = None;
    let (mut init, mut stop) = (0.0, f64::NAN); // initial stop level, stop level in force for the next bar
    let mut pending: i8 = 0;
    for i in from..=to {
        if pending != 0 {
            let dir = pending as f64;
            pending = 0;
            if let Some(mut t) = cur.take() {
                t.exit_i = i; t.exit_px = x.o[i]; t.signal_i = i - 1; t.reason = "Reversing signal · next open".into();
                out.push(t);
            }
            cur = Some(new_trade(dir, i, x.o[i], i - 1));
            init = x.o[i] - dir * q.stop;
            stop = if q.eds { init } else { f64::NAN };
        }
        if let Some(t) = cur.as_mut() {
            let (dir, e, ei) = (t.dir, t.legs[0].px, t.legs[0].i);
            let hit = stop.is_finite() && if dir > 0. { x.l[i] <= stop } else { x.h[i] >= stop };
            if hit {
                let px = if dir > 0. { x.o[i].min(stop) } else { x.o[i].max(stop) };
                t.exit_i = i; t.exit_px = px; t.signal_i = i;
                t.reason = if (stop - init).abs() < 1e-12 { "Initial stop".into() } else { "Trailing stop".into() };
                out.push(cur.take().unwrap());
            } else {
                // stops for the next bar (author's Channel 25 code: Highest(c, BarsSinceEntry))
                let w = if i == ei { &x.c[i..=i] } else { &x.c[ei + 1..=i] };
                let ext = if dir > 0. { w.iter().cloned().fold(f64::MIN, f64::max) } else { w.iter().cloned().fold(f64::MAX, f64::min) };
                stop = init;
                if dir * (ext - e) >= q.trail {
                    let tr = ext - dir * q.trail;
                    stop = if dir > 0. { stop.max(tr) } else { stop.min(tr) };
                }
            }
        }
        if i < to && sig[i] != 0 && cur.as_ref().map_or(true, |t| t.dir != sig[i] as f64) {
            pending = sig[i];
        }
    }
    if let Some(mut t) = cur { t.exit_i = to; t.exit_px = x.c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true; out.push(t); }
    out
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if from > to || to >= x.r.len() { return None; }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap().def);
    let u = |k: &str| g(k).round().max(1.) as usize;
    let keys = if id.ends_with("_nse") { NSE } else { NYSE };
    let (Some(a), Some(d)) = (x.series(keys[0]), x.series(keys[1])) else { return Some(vec![]) };
    let pt = g("pt_usd").max(1e-9);
    let q = Quad { chad_n: u("chad_n"), len: u("len").max(2), k: g("k"), stop: g("stop_usd") / pt, trail: g("trail_usd") / pt, eds: g("eds") > 0.5, tf: id.contains("_tf") };
    let sig = quadram_signals(a, d, &q);
    Some(quadram_run(x, from, to, &sig, &q))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn rows(o: &[(f64, f64, f64, f64)]) -> Vec<Row> {
        o.iter().enumerate().map(|(i, &(op, h, l, c))| Row { ticker: "T".into(), date: format!("2001-01-{:02}", i + 1), open: op, high: h, low: l, close: c, volume: None }).collect()
    }
    #[test]
    fn countertrend_cross_and_band_math() {
        // CHAD with chad_n=1, len=2, k=0: band = 2-day mean. chad: 0,-10,-20,0 -> mean -5,-15,-10 ; cross above at i=3 (0 > -10, -20 <= -15)
        let a = [0., 0., 0., 10.];
        let d = [0., 10., 20., 10.];
        let q = Quad { chad_n: 1, len: 2, k: 0., stop: 2., trail: 2., eds: true, tf: false };
        assert_eq!(quadram_signals(&a, &d, &q), vec![0, 0, 0, 1]);
        let (m, s) = bands(&[1., 3.], 2);
        assert_eq!(m[1], 2.); assert_eq!(s[1], 1.); // population SD
    }
    #[test]
    fn entry_next_open_stops_and_trailing() {
        let r = rows(&[(100., 101., 99., 100.), (100., 101., 99., 100.), (100., 106., 99.5, 105.), (105., 105.5, 102.5, 104.), (104., 104., 98., 99.)]);
        let x = Ctx::new(&r, &r, &[]);
        let q = Quad { chad_n: 1, len: 2, k: 0., stop: 2., trail: 2., eds: true, tf: false };
        let sig = vec![0, 1, 0, 0, 0];
        let t = quadram_run(&x, 0, 4, &sig, &q);
        assert_eq!(t.len(), 1);
        assert_eq!(t[0].legs[0].i, 2); assert_eq!(t[0].legs[0].px, 100.); // next open
        // entry-day low 99.5 > 98 initial stop; close 105 -> trail active: 105-2 = 103; bar 3 low 102.5 -> stopped at 103
        assert_eq!(t[0].exit_i, 3); assert_eq!(t[0].exit_px, 103.); assert_eq!(t[0].reason, "Trailing stop");
        // entry-day stop: low 97.5 hits 98 on the entry bar
        let r = rows(&[(100., 101., 99., 100.), (100., 101., 99., 100.), (100., 100.5, 97.5, 99.)]);
        let x = Ctx::new(&r, &r, &[]);
        let t = quadram_run(&x, 0, 2, &[0, 1, 0], &q);
        assert_eq!((t[0].exit_i, t[0].exit_px), (2, 98.));
    }
}
