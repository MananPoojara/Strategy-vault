//! Ernest P. Chan, *Quantitative Trading: How to Build Your Own Algorithmic Trading Business* (Wiley, 2008/2009).
//! Book id `ecqt`. Source copy: Books/Ernest Chan/Ernest Chan - Quatitative Trading.pdf.
//! Transcription and testability table: library/ecqt/README.md.
//!
//! Every fully specified strategy in the book trades a basket (S&P 500 / S&P 600 stocks), an ETF pair (GLD/GDX) or a
//! futures contract (RB, NG). The vault engine backtests one ticker per run; the NSE nifty50/nse500 baskets exist but
//! cannot be combined into one portfolio, so these are NeedsData with the exact gap. Kelly / optimal leverage
//! (Ch.6) is position sizing, not a strategy.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "ecqt";

const PORT: &str = "a cross-sectional stock portfolio on one equity curve (daily weights across every basket member). The engine backtests one ticker per run; NSE nifty50/nse500 basket prices are in the vault (current membership, the same survivorship bias Chan accepts) but there is no multi-asset portfolio simulator to combine them";

macro_rules! def {
    ($id:expr, $name:expr, $g:expr, $dir:expr, $st:expr, $rules:expr, $notes:expr) => {
        StrategyDef { id: $id, name: $name, group: $g, session: 0, dir: $dir, status: $st, needs_volume: false, needs_vix: false,
            needs_series: &[], book: BOOK, rules: $rules, notes: $notes, claim: None }
    };
}

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    vec![
        def!("ecqt_gld_gdx_pair", "GLD/GDX pair trade, z-score 2 / 1 (Example 3.6)", "Ch.3 · Backtesting examples", "Long/Short",
            Live,
            &["F|trainset = first 252 common days; hedgeRatio = ols(GLD, GDX) on the trainset; spread = GLD - hedgeRatio*GDX (p.57)",
              "F|zscore = (spread - mean(spread(trainset))) / std(spread(trainset)) (p.57)",
              "E|\"buy spread when its value drops below 2 standard deviations\": zscore <= -2 -> +1 GLD, -1 GDX; zscore >= 2 -> -1 GLD, +1 GDX (p.57-58)",
              "X|\"exit any spread position when its value is within 1 standard deviation of its mean\": abs(zscore) <= 1 (p.57)",
              "X|positions carried forward until an exit; pnl = sum(lag1(positions) .* dailyret) (p.58)"],
            PAIR_NOTE),
        def!("ecqt_kl_mr_close", "Khandani-Lo daily mean reversion, close to close (Example 3.7)", "Ch.3 · Backtesting examples", "Long/Short",
            NeedsData(PORT),
            &["F|universe: S&P 500 stocks (list of 21 Nov 2007), daily closes (p.61-63)",
              "E|weights = -(dailyret - marketDailyret) / (number of stocks with prices); marketDailyret = equal-weighted mean return (p.63)",
              "X|held one day: dailypnl = smartsum(lag1(weights) .* dailyret) (p.63); 5 bp per one-way trade on weight changes (p.64-65)"],
            "Published 2006: Sharpe about 0.25 before costs, about -3.19 after 5 bp (p.63-65)."),
        def!("ecqt_kl_mr_open", "Khandani-Lo daily mean reversion on opens (Example 3.8)", "Ch.3 · Backtesting examples", "Long/Short",
            NeedsData(PORT),
            &["E|Example 3.7 with \"cl\" replaced by \"op\" everywhere: \"update the positions at the market open instead of the close\" (p.66)"],
            "Published 2006: Sharpe 4.43 before costs, 0.78 after 5 bp (p.66)."),
        def!("ecqt_pca_factor", "PCA factor model, top/bottom 50 (Example 7.4)", "Ch.7 · Factor models", "Long/Short",
            NeedsData(PORT),
            &["F|S&P 600 small caps; lookback 252 days; numFactors 5 = eigenvectors of the return covariance with the top eigenvalues (p.137-138)",
              "E|Rexp = avgR + X*b (b = ols of the last day's returns on X); short the topN = 50 lowest, buy the 50 highest; held one day (p.138)"],
            "Published: annualized average return -1.8099 (as printed, p.138) - Chan's point is that it fails."),
        def!("ecqt_january", "January effect, small caps (Example 7.6)", "Ch.7 · Seasonal", "Long/Short",
            NeedsData(PORT),
            &["F|S&P 600 small caps; prior calendar-year return per stock (p.144-145)",
              "E|on the last trading day of December buy the lowest decile, short the highest decile (topN = round(N/10)) (p.145)",
              "X|exit on the last trading day of January; 5 bp one way (p.145)"],
            "Published portfolio returns: Jan 2006 -0.0244, Jan 2007 -0.0068, Jan 2008 0.0881 (p.145)."),
        def!("ecqt_yoy_seasonal", "Year-on-year seasonal trend (Example 7.7, Heston-Sadka)", "Ch.7 · Seasonal", "Long/Short",
            NeedsData(PORT),
            &["E|each month buy the top decile of stocks by their return in the same month a year earlier, short the bottom decile (p.146-147)",
              "X|hold one month (month-end to month-end) (p.147)"],
            "Published: avg annual return -0.9167, Sharpe -0.1055 (as printed, S&P 500 list of Nov 2007, p.147)."),
        def!("ecqt_rb_seasonal", "Gasoline futures spring trade", "Ch.7 · Seasonal", "Long",
            NeedsData("NYMEX RB May unleaded-gasoline futures contract daily settlements 1995-2008 (no futures in the vault; an Indian index is not a gasoline contract)"),
            &["E|\"buy 1 contract of RB ... that expires in May at the close of April 13 (or the following trading day if it is a holiday)\" (p.149)",
              "X|\"and sell it at the close of April 25 (or the previous trading day if it is a holiday)\" (p.149)"],
            "Published P&L per contract 1995-2008: every year positive (1995 $1,037 ... 2008* $4,741; *actual trading as 2 x QU) (p.149)."),
        def!("ecqt_ng_seasonal", "Natural gas futures spring trade", "Ch.7 · Seasonal", "Long",
            NeedsData("NYMEX NG June natural-gas futures contract daily settlements 1995-2008 (no futures in the vault)"),
            &["E|\"long a June contract of NYMEX natural gas futures (Symbol: NG) at the close of February 25 (or the following trading day if it is a holiday)\" (p.150)",
              "X|\"exit this position on April 15 (or the previous trading day if it is a holiday)\" (p.150)"],
            "Published P&L 1995-2008, profitable 14 consecutive years (1995 $1,970 ... 2008* $10,137) (p.150)."),
        def!("ecqt_gs_regime_ml", "GS regime switch via perceptron (Example 7.1)", "Ch.7 · Regime switching", "Long/Short",
            NotTestable("the trade decision is a perceptron's weighting of rule/holding-period combinations trained on a moving window in Alphacet Discovery; the weights and training window are not given (p.122-125)"),
            &["E|R3: 1-day % change >= 1 AND 10-day moving low >= Low -> 1; % change >= -1 AND 10-day moving high <= High -> -1 (Fig 7.2, p.124)",
              "X|holding periods 1, 5, 10, 20, 40, 60 days combined by a perceptron (p.123-124)"], ""),
        def!("ecqt_es_bb_5m", "ES Bollinger mean reversion on 5-minute bars", "Ch.2 · Transaction costs", "Long/Short",
            NotTestable("moving-average / standard-deviation lookback is not given and the rule runs on 5-minute E-mini bars (p.23)"),
            &["E|\"every time the price exceeds plus or minus 2 moving standard deviations of its moving average, short or buy, respectively\" (p.23)",
              "X|\"Exit the position when the price reverts back to within 1 moving standard deviation of the moving average\" (p.23)"],
            "Published: Sharpe about 3 without costs, -3 with 1 bp (p.23)."),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    if id == PAIR { return Some((PAIR, "Chan GLD/GDX pair z-score (Example 3.6)", "2 / 1")); } // lane P1: a strategy of its own
    let d = catalogue().into_iter().find(|d| d.id == id)?;
    Some(("ecqt", "Chan · Quantitative Trading", d.name))
}
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    (id == PAIR).then(|| vec![p("entry", "Entry |z| (book 2; the book's alternative 1)", 2., 0.5, 4., 0.1), p("exit", "Exit |z| (book 1; the book's alternative 0.5)", 1., 0., 3., 0.1),
        p("train", "Training days for the hedge ratio, mean and sd (book 252)", 252., 30., 1000., 1.)])
}

/// Lane P1 (13 Nov 2026, R4 #294 H3 row): Example 3.6 runs on the two-leg spread account (crate::spread) in the GLD context, with GDX
/// joined by exact date. Independent reference authors/titles-1030/P1-1113w/p1_ref.py.
pub const PAIR: &str = "ecqt_gld_gdx_pair";
pub const GDX: &str = "GDX.Close";
const PAIR_NOTE: &str = "Lane P1 (13 Nov 2026). Chan's printed MATLAB (Example 3.6, p.55-58) as written: the two series are intersected by date; the first 252 common days are the training set; hedge ratio = OLS of GLD on GDX without intercept (the spatialeconometrics ols with one regressor); spread = GLD - hedgeRatio x GDX; z = (spread - training mean) / training standard deviation (MATLAB std: n - 1), fixed for the whole run; z <= -2: +1 GLD, -1 GDX (long the spread); z >= 2: -1 GLD, +1 GDX; |z| <= 1: flat; otherwise the position is carried; the position taken at a close earns the next day's returns (lag1), each leg's money stays the same every day (pnl = sum(lag1(positions) .* dailyret)), so a trade's return = the sum of its daily (GLD return - GDX return) x direction / 2 (two legs of money 1). No unstated details. Data (labelled): GLD from the vault's US price file and GDX from the Norgate US Equities snapshot (split-adjusted, price only, ends 2020-09-11; Chan uses adjusted closes from his GLD.xls/GDX.xls), so the common days run 2006-05-22..2020-09-11 and the training set is 2006-05-22.. its 252nd common day; the run carries the training-set parameters to the end of the data, as the book's test set does. Published: Sharpe about 2.3 on the training set and about 1.5 on the test set; with entry 1 / exit 0.5 standard deviation 2.9 and 2.1 (p.58-59). Half-life of the spread about 10 days (Ex 7.5, p.141-142). Costs: none in the book ('I have not incorporated transaction costs', p.59); the vault's fee setting applies per leg.";
/// GDX daily closes from the Norgate CSV ("Date" yyyymmdd, "Close"); None if the file is missing.
pub fn gdx_series(path: &std::path::Path) -> Option<std::collections::BTreeMap<String, f64>> {
    let text = std::fs::read_to_string(path).map_err(|e| eprintln!("warning: GDX not loaded from {}: {e}", path.display())).ok()?;
    let mut lines = text.lines();
    let head = crate::parse_csv_fields(lines.next()?);
    let (di, ci) = (head.iter().position(|h| h == "Date")?, head.iter().position(|h| h == "Close")?);
    Some(lines.filter_map(|l| {
        let f = crate::parse_csv_fields(l);
        let d = f.get(di)?;
        let date = (d.len() == 8 && d.bytes().all(|b| b.is_ascii_digit())).then(|| format!("{}-{}-{}", &d[..4], &d[4..6], &d[6..]))?;
        Some((date, crate::parse_csv_number(f.get(ci)?).ok().filter(|v| v.is_finite() && *v > 0.)?))
    }).collect())
}
/// The pair's positions (None = not this rule).
pub fn positions(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Result<Vec<crate::options::Pos>, String>> {
    (id == PAIR).then(|| pair(x, from, to, p))
}
fn pair(x: &Ctx, from: usize, to: usize, p: &Params) -> Result<Vec<crate::options::Pos>, String> {
    if x.r.first().map(|r| r.ticker.as_str()) != Some("GLD") {
        return Err("not_testable: Chan's GLD/GDX pair runs only in the GLD context, with GDX as the second leg (the trades hold both ETFs)".into());
    }
    let gdx = x.series(GDX).ok_or("needs_data: the GDX prices (Norgate US Equities) are not loaded in this engine; no price is filled in")?;
    let g = |k: &str, d: f64| p.get(k).copied().filter(|v| v.is_finite()).unwrap_or(d);
    let (entry, exit, train) = (g("entry", 2.), g("exit", 1.), g("train", 252.).round().max(2.) as usize);
    let j = crate::spread::joint(&[&x.c, gdx]);
    if j.len() <= train { return Err("needs_data: fewer common GLD/GDX days than the training set".into()); }
    let (a, b): (Vec<f64>, Vec<f64>) = j[..train].iter().map(|&i| (x.c[i], gdx[i])).unzip();
    let beta = a.iter().zip(&b).map(|(y, x)| y * x).sum::<f64>() / b.iter().map(|x| x * x).sum::<f64>();
    let s: Vec<f64> = j.iter().map(|&i| x.c[i] - beta * gdx[i]).collect();
    let mean = s[..train].iter().sum::<f64>() / train as f64;
    let sd = (s[..train].iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (train - 1) as f64).sqrt();
    if !(sd > 0.) { return Err("calculation_unavailable: the training-set spread has no variation".into()); }
    let mut out = Vec::new();
    let mut pos: Option<(usize, f64, f64)> = None; // (bar, direction, z at entry)
    let close = |e: usize, xi: usize, dir: f64, z0: f64, why: String, open: bool| {
        let side = if dir > 0. { "long the spread: +1 GLD, -1 GDX" } else { "short the spread: -1 GLD, +1 GDX" };
        let reason = format!("{side} at the {} close (z = {z0:.2}; hedge ratio {beta:.4} on the first {train} common days); {why}", x.r[e].date);
        crate::spread::rebalanced(&[(&x.c, 1.), (gdx, -1.)], dir, e, xi, reason, open).ok_or_else(|| format!("calculation_unavailable: no price on {}", x.r[e].date))
    };
    for (k, &i) in j.iter().enumerate() {
        if i > to { break; }
        let z = (s[k] - mean) / sd;
        let held = pos.map_or(0., |q| q.1);
        let want = if z >= entry { -1. } else if z <= -entry { 1. } else if z.abs() <= exit { 0. } else { held };
        if want == held { continue; }
        if let Some((e, dir, z0)) = pos.take() { out.push(close(e, i, dir, z0, format!("{} at the {} close (z = {z:.2})", if want == 0. { "exit" } else { "reversed" }, x.r[i].date), false)?); }
        if want != 0. && i >= from { pos = Some((i, want, z)); }
    }
    if let Some((e, dir, z0)) = pos {
        let xi = j[..j.partition_point(|&i| i <= to)].last().copied().unwrap_or(e);
        out.push(close(e, xi, dir, z0, "still open at the last bar (marked at its close)".into(), true)?);
    }
    Ok(out.into_iter().filter(|q| q.entry_i >= from).collect())
}
pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn gld_gdx_pair_trains_on_the_first_days_and_trades_the_z_bands() {
        // GDX = GLD / 2 with a wave, so spread = GLD - beta x GDX swings through +/-2 and back inside 1
        let n = 120;
        // a wave inside the training days, then a rise (z >> 2: short) and a fall (z << -2: long), each back inside 1 sd
        let bump = |i: usize| if (70..76).contains(&i) { 15. } else if (90..96).contains(&i) { -15. } else { 0. };
        let gld: Vec<f64> = (0..n).map(|i| 100. + (i as f64 * 0.3).sin() * 5. + bump(i)).collect();
        let gdx: Vec<f64> = (0..n).map(|i| 50. + (i as f64 * 0.11).cos()).collect();
        let r: Vec<Row> = gld.iter().enumerate().map(|(i, &c)| Row { ticker: "GLD".into(), date: format!("2007-{:02}-{:02}", 1 + i / 28, 1 + i % 28), open: c, high: c, low: c, close: c, volume: None }).collect();
        let mut x = Ctx::new(&r, &r, &[]);
        x.extra.insert(GDX.into(), gdx.clone());
        let p: Params = [("train".to_string(), 60.)].into_iter().collect();
        let pos = pair(&x, 0, n - 1, &p).unwrap();
        assert!(pos.iter().any(|q| q.dir > 0.) && pos.iter().any(|q| q.dir < 0.));
        let beta = gld[..60].iter().zip(&gdx[..60]).map(|(a, b)| a * b).sum::<f64>() / gdx[..60].iter().map(|b| b * b).sum::<f64>();
        let s: Vec<f64> = (0..n).map(|i| gld[i] - beta * gdx[i]).collect();
        let m = s[..60].iter().sum::<f64>() / 60.;
        let sd = (s[..60].iter().map(|v| (v - m) * (v - m)).sum::<f64>() / 59.).sqrt();
        for q in &pos {
            let z = (s[q.entry_i] - m) / sd;
            assert!(if q.dir > 0. { z <= -2. } else { z >= 2. });
            let want: f64 = (q.entry_i + 1..=q.exit_i).map(|k| q.dir * ((gld[k] / gld[k - 1] - 1.) - (gdx[k] / gdx[k - 1] - 1.))).sum();
            assert!((q.pnl.last().unwrap() - want).abs() < 1e-12 && q.s0 == 2.);
        }
        let q = vec![Row { ticker: "SPY".into(), ..r[0].clone() }];
        assert!(pair(&Ctx::new(&q, &q, &[]), 0, 0, &p).err().unwrap().starts_with("not_testable"));
    }
}
