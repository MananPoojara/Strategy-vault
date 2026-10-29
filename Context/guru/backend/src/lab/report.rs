//! AmiBroker-style backtest report for lab runs (AmiBroker "Stats" page): All / Long / Short trade blocks with winners and
//! losers, system metrics and per-trade MAE/MFE. Lab-only — book statistics are untouched.
//! Formulas follow the AmiBroker report guide (https://www.amibroker.com/guide/h_report.html); where the guide leaves a
//! detail open, the choice is stated in NOTES and shown with the report.
use crate::Row;

/// AmiBroker Settings › Report defaults: "Risk-free rate for Sharpe ratio" 5 %, "Risk-free rate for UPI" 5.4 % (treasury).
/// A spec sets its own with `execution.report = {"rf_sharpe": %, "rf_upi": %}`; the used pair is echoed as `report.rates`.
pub const SHARPE_RF_PCT: f64 = 5.0;
pub const UPI_RF_PCT: f64 = 5.4;
pub const DEFAULT_RATES: (f64, f64) = (SHARPE_RF_PCT, UPI_RF_PCT);
/// `execution.report` of a spec: (Sharpe, UPI) risk-free rates in percent, each optional (AmiBroker default) and 0 to 100.
pub fn rates(v: &crate::lab::json::Json) -> Result<(f64, f64), super::rules::LabError> {
    let crate::lab::json::Json::Obj(kv) = v else { return Err(super::rules::LabError::Invalid("execution.report must be an object".into())) };
    if let Some((k, _)) = kv.iter().find(|(k, _)| k != "rf_sharpe" && k != "rf_upi") { return Err(super::rules::LabError::Invalid(format!("unknown field '{k}' in execution.report"))); }
    let rate = |k: &str, default: f64| v.get(k).map_or(Ok(default), |x| x.as_f64().filter(|r| (0. ..=100.).contains(r))
        .ok_or_else(|| super::rules::LabError::Invalid(format!("execution.report.{k} must be a percentage from 0 to 100"))));
    Ok((rate("rf_sharpe", SHARPE_RF_PCT)?, rate("rf_upi", UPI_RF_PCT)?))
}
pub const NOTES: &[&str] = &[
    "Returns are per unit of initial equity (1.0); money mode reports the same ratios on its cash equity.",
    "MAE and MFE use the highs and lows of every bar from the entry bar to the exit bar, as fractions of the entry price.",
    "Max. trade drawdown is the deepest close-to-close peak-to-valley fall inside one trade.",
    "Years and bars per year as AmiBroker: calendar days from the first to the last bar / 365, and bars / years. CAR = final equity^(1/years) − 1; CAR/MaxDD and the UPI follow it.",
    "Exposure as AmiBroker: bars in a position from the entry bar to the bar before the exit, over all bars of the run. Net RAR, RAR and RAR/MaxDD follow it.",
    "Sharpe ratio of trades: average trade % return × (bars per year / average bars held), minus {sharpe} % risk-free (Settings › Report; AmiBroker default 5 %), divided by the population standard deviation of trade % returns × √(bars per year / average bars held). Risk-reward ratio: slope × bars per year / standard error.",
    "Ulcer performance index uses a {upi} % risk-free rate (Settings › Report; AmiBroker default 5.4 %). K-ratio as AmiBroker prints it: slope / (standard error of the slope × bars), on the linear equity line.",
    "Trade statistics count every trade of the trade list, the last one still open included, valued at the last close as in the tradesheet; the run's headline stats count closed trades only.",
    "Profit factor, payoff ratio and Profit are from each trade's money profit (× initial equity: the ledger P&L in money mode, otherwise equity before the trade × its return), as AmiBroker does; Sum of trade % adds the trade % returns.",
    "Bars held count the entry and the exit bar (AmiBroker # bars): a same-bar trade is 1 bar. The trade list's bars column is exit bar − entry bar.",
    "Buy&Hold: the same system metrics on the instrument's close from the first bar, always invested; its statistics column is one trade from the first to the last bar of the run, marked at the last close.",
];

/// One trade as the report needs it: bar indices into `rows`, entry price, return on the position, direction and money
/// profit as a fraction of initial equity.
#[derive(Clone, Copy)]
pub struct ReportTrade { pub dir: f64, pub entry_i: usize, pub exit_i: usize, pub entry_px: f64, pub ret: f64, pub profit: f64 }

/// Maximum adverse and favourable excursion (fractions of the entry price; MAE ≤ 0 ≤ MFE).
pub fn excursion(rows: &[Row], t: &ReportTrade) -> (f64, f64) {
    let (mut hi, mut lo) = (f64::MIN, f64::MAX);
    for r in &rows[t.entry_i..=t.exit_i] { hi = hi.max(r.high); lo = lo.min(r.low); }
    let (up, down) = (hi / t.entry_px - 1., lo / t.entry_px - 1.);
    if t.dir > 0. { (down.min(0.), up.max(0.)) } else { ((-up).min(0.), (-down).max(0.)) }
}

/// Deepest close-to-close peak-to-valley fall of the open position, as a fraction (≤ 0).
fn trade_drawdown(rows: &[Row], t: &ReportTrade) -> f64 {
    let (mut peak, mut worst) = (1.0_f64, 0.0_f64);
    for r in &rows[t.entry_i..=t.exit_i] {
        let mark = if t.dir > 0. { r.close / t.entry_px } else { 2. - r.close / t.entry_px };
        peak = peak.max(mark);
        worst = worst.min(mark / peak - 1.);
    }
    worst
}

fn group(ts: &[&ReportTrade], total: usize) -> Vec<(&'static str, f64)> {
    let n = ts.len() as f64;
    let rets: Vec<f64> = ts.iter().map(|t| t.ret).collect();
    let bars = |t: &&ReportTrade| (t.exit_i - t.entry_i + 1) as f64;
    let largest = ts.iter().max_by(|a, b| a.ret.abs().total_cmp(&b.ret.abs()));
    vec![
        ("count", n), ("pct", if total == 0 { f64::NAN } else { n / total as f64 }), ("total", rets.iter().sum()), ("profit", ts.iter().map(|t| t.profit).sum()),
        ("avg", if n == 0. { f64::NAN } else { rets.iter().sum::<f64>() / n }),
        ("avg_bars", if n == 0. { f64::NAN } else { ts.iter().map(bars).sum::<f64>() / n }),
        ("largest", largest.map_or(f64::NAN, |t| t.ret)), ("bars_in_largest", largest.map_or(f64::NAN, bars)),
    ]
}

/// A named value from `group` (looked up by name so inserting a field cannot shift another).
fn field(g: &[(&'static str, f64)], key: &str) -> f64 { g.iter().find(|(k, _)| *k == key).map_or(f64::NAN, |(_, v)| *v) }

fn max_consecutive(ts: &[&ReportTrade], win: bool) -> f64 {
    let (mut run, mut best) = (0, 0);
    for t in ts { if (t.ret > 0.) == win { run += 1; best = best.max(run); } else { run = 0; } }
    best as f64
}

fn obj(pairs: &[(&str, f64)]) -> String {
    format!("{{{}}}", pairs.iter().map(|(k, v)| format!("\"{k}\":{}", crate::num(*v))).collect::<Vec<_>>().join(","))
}

/// One column of the AmiBroker statistics table (All trades, Long trades or Short trades).
fn side_json(ts: &[&ReportTrade]) -> String {
    let n = ts.len();
    let winners: Vec<&ReportTrade> = ts.iter().copied().filter(|t| t.ret > 0.).collect();
    let losers: Vec<&ReportTrade> = ts.iter().copied().filter(|t| t.ret <= 0.).collect();
    let mut w = group(&winners, n); w.push(("max_consecutive", max_consecutive(ts, true)));
    let mut l = group(&losers, n); l.push(("max_consecutive", max_consecutive(ts, false)));
    let (gain, loss): (f64, f64) = (winners.iter().map(|t| t.profit).sum(), losers.iter().map(|t| t.profit).sum());
    let all = group(ts, n);
    format!("{{\"trades\":{},\"avg\":{},\"avg_bars\":{},\"profit_factor\":{},\"payoff\":{},\"winners\":{},\"losers\":{}}}",
        n, crate::num(field(&all, "avg")), crate::num(field(&all, "avg_bars")), crate::num(if loss == 0. { f64::NAN } else { gain / -loss }),
        crate::num(if losers.is_empty() || winners.is_empty() { f64::NAN } else { (gain / winners.len() as f64) / (-loss / losers.len() as f64) }),
        obj(&w), obj(&l))
}

/// Least-squares fit of `y` on the bar number: (slope per bar, standard error of the fit, Σ(x - x̄)²).
fn regression(y: &[f64]) -> (f64, f64, f64) {
    let n = y.len() as f64;
    let xm = (n - 1.) / 2.;
    let ym = y.iter().sum::<f64>() / n;
    let sxx: f64 = (0..y.len()).map(|i| (i as f64 - xm).powi(2)).sum();
    let sxy: f64 = y.iter().enumerate().map(|(i, v)| (i as f64 - xm) * (v - ym)).sum();
    let slope = sxy / sxx;
    let sse: f64 = y.iter().enumerate().map(|(i, v)| (v - (ym + slope * (i as f64 - xm))).powi(2)).sum();
    (slope, (sse / (n - 2.)).sqrt(), sxx)
}

/// AmiBroker's year length for a run: calendar days from the first to the last bar / 365 (CAR, bars per year).
pub fn years(rows: &[Row], from: usize, to: usize) -> f64 {
    let day = |i: usize| super::resample::day_number(&rows[i].date);
    ((day(to) - day(from)) as f64 / 365.).max(1. / 365.)
}
/// AmiBroker CAR of an equity line starting at 1.0.
pub fn car(equity: &[f64], years: f64) -> f64 { equity.last().map_or(f64::NAN, |e| e.powf(1. / years) - 1.) }
/// AmiBroker exposure: bars in a position from the entry bar to the bar before the exit, over the run's bars.
/// shortcut: a held bar counts 1 (one unit fully invested), not position value / equity; upgrade for money mode with partial sizing.
pub fn exposure(trades: &[ReportTrade], from: usize, to: usize) -> f64 {
    let mut held = vec![false; to + 1 - from];
    for t in trades { for i in t.entry_i.max(from)..t.exit_i.min(to + 1) { held[i - from] = true; } }
    held.iter().filter(|h| **h).count() as f64 / held.len() as f64
}

/// System metrics on the equity line (starting at 1.0) and the trade list, with AmiBroker's `cagr`, `exposure` and bars per year.
pub fn system(rows: &[Row], equity: &[f64], trades: &[ReportTrade], cagr: f64, exposure: f64, bars_per_year: f64, (rf_sharpe, rf_upi): (f64, f64)) -> Vec<(&'static str, f64)> {
    let net = equity.last().copied().unwrap_or(1.) - 1.;
    let (mut peak, mut dd_pct, mut dd_abs, mut ulcer_sum) = (f64::MIN, 0.0_f64, 0.0_f64, 0.0);
    for &e in equity {
        peak = peak.max(e);
        let d = e / peak - 1.;
        dd_pct = dd_pct.min(d); dd_abs = dd_abs.max(peak - e); ulcer_sum += (d * 100.).powi(2);
    }
    let ulcer = (ulcer_sum / equity.len() as f64).sqrt();
    let (slope, se, sxx) = if equity.len() > 2 { regression(equity) } else { (f64::NAN, f64::NAN, f64::NAN) };
    let rar = if exposure > 0. { cagr / exposure } else { f64::NAN };
    let rets: Vec<f64> = trades.iter().map(|t| t.ret * 100.).collect();
    let n = rets.len() as f64;
    let avg = rets.iter().sum::<f64>() / n;
    let sd = (rets.iter().map(|r| (r - avg).powi(2)).sum::<f64>() / n).sqrt(); // population SD (AmiBroker)
    let avg_bars = trades.iter().map(|t| (t.exit_i - t.entry_i + 1) as f64).sum::<f64>() / n;
    let f = bars_per_year / avg_bars;
    let trade_dd = trades.iter().map(|t| trade_drawdown(rows, t)).fold(0.0_f64, f64::min);
    vec![
        ("net_profit", net), ("exposure", exposure), ("net_rar", if exposure > 0. { net / exposure } else { f64::NAN }),
        ("cagr", cagr), ("rar", rar), ("max_trade_dd", trade_dd), ("max_sys_dd", dd_pct), ("max_sys_dd_abs", -dd_abs),
        ("recovery_factor", if dd_abs > 0. { net / dd_abs } else { f64::NAN }),
        ("car_mdd", if dd_pct < 0. { cagr / -dd_pct } else { f64::NAN }), ("rar_mdd", if dd_pct < 0. { rar / -dd_pct } else { f64::NAN }),
        ("std_error", se), ("rrr", if se > 0. { slope * bars_per_year / se } else { f64::NAN }),
        ("ulcer_index", ulcer), ("upi", if ulcer > 0. { (cagr * 100. - rf_upi) / ulcer } else { f64::NAN }),
        ("sharpe_trades", if n > 1. && sd > 0. { (avg * f - rf_sharpe) / (sd * f.sqrt()) } else { f64::NAN }),
        ("k_ratio", if se > 0. && sxx > 0. { slope * sxx.sqrt() / se / equity.len() as f64 } else { f64::NAN }),
    ]
}

/// The `report` object of a lab run: AmiBroker statistics columns plus system metrics (the strategy's and buy-and-hold's on
/// `bh`), the risk-free rates used and the declared conventions. `trades` are the trade list's trades, an open one marked at the last close.
/// Contract (Report.tsx): `report.buyhold` has the same keys as `report.all` (null when not applicable) — the Buy&Hold
/// statistics column: one long trade from the first to the last bar of the run on the same instrument, marked at the last close.
/// `report.rates` = {"sharpe": %, "upi": %}, the risk-free rates the Sharpe ratio of trades and the UPI used.
pub fn report_json(rows: &[Row], equity: &[f64], trades: &[ReportTrade], cagr: f64, exposure: f64, bars_per_year: f64, bh: &[f64], bh_cagr: f64, rates: (f64, f64)) -> String {
    let all: Vec<&ReportTrade> = trades.iter().collect();
    let long: Vec<&ReportTrade> = trades.iter().filter(|t| t.dir > 0.).collect();
    let short: Vec<&ReportTrade> = trades.iter().filter(|t| t.dir < 0.).collect();
    // bh starts at 1.0 on the first bar; only ret, profit and the bar count reach side_json
    let ret = bh.last().map_or(f64::NAN, |v| v - 1.);
    let hold = ReportTrade { dir: 1., entry_i: 0, exit_i: bh.len().saturating_sub(1), entry_px: 1., ret, profit: ret };
    let held: Vec<&ReportTrade> = if bh.is_empty() { vec![] } else { vec![&hold] };
    format!("{{\"all\":{},\"long\":{},\"short\":{},\"buyhold\":{},\"system\":{},\"buy_hold\":{},\"rates\":{{\"sharpe\":{},\"upi\":{}}},\"notes\":[{}]}}",
        side_json(&all), side_json(&long), side_json(&short), side_json(&held), obj(&system(rows, equity, trades, cagr, exposure, bars_per_year, rates)),
        obj(&system(rows, bh, &[], bh_cagr, 1., bars_per_year, rates)), crate::num(rates.0), crate::num(rates.1),
        NOTES.iter().map(|n| format!("\"{}\"", n.replace("{sharpe}", &crate::num(rates.0)).replace("{upi}", &crate::num(rates.1)))).collect::<Vec<_>>().join(","))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn row(o: f64, h: f64, l: f64, c: f64) -> Row { Row { ticker: "T".into(), date: "2020-01-01".into(), open: o, high: h, low: l, close: c, volume: None } }

    #[test] fn excursions_follow_direction() {
        let rows = vec![row(100., 104., 98., 101.), row(101., 110., 95., 105.)];
        let long = ReportTrade { dir: 1., entry_i: 0, exit_i: 1, entry_px: 100., ret: 0.05, profit: 0.05 };
        let (mae, mfe) = excursion(&rows, &long);
        assert!((mae + 0.05).abs() < 1e-12 && (mfe - 0.10).abs() < 1e-12);
        let short = ReportTrade { dir: -1., ..long };
        let (mae, mfe) = excursion(&rows, &short);
        assert!((mae + 0.10).abs() < 1e-12 && (mfe - 0.05).abs() < 1e-12);
    }

    #[test] fn blocks_split_winners_losers_and_streaks() {
        let t = |ret: f64, dir: f64, bars: usize| ReportTrade { dir, entry_i: 0, exit_i: bars, entry_px: 1., ret, profit: ret };
        let ts = [t(0.1, 1., 2), t(0.2, 1., 4), t(-0.05, -1., 1), t(-0.1, 1., 3), t(0.3, -1., 5)];
        let all: Vec<&ReportTrade> = ts.iter().collect();
        let j = side_json(&all);
        assert!(j.contains("\"trades\":5"), "{j}");
        // the All-trades average and bars held (bars count the entry bar: 3, 5, 2, 4, 6 → 4)
        assert!(j.contains("\"trades\":5,\"avg\":0.09,\"avg_bars\":4,"), "{j}");
        // bars count the entry and the exit bar (AmiBroker # bars)
        assert!(j.contains("\"winners\":{\"count\":3,\"pct\":0.6,\"total\":0.6,\"profit\":0.6,\"avg\":0.2,\"avg_bars\":4.666667,\"largest\":0.3,\"bars_in_largest\":6,\"max_consecutive\":2}"), "{j}");
        assert!(j.contains("\"losers\":{\"count\":2,\"pct\":0.4,\"total\":-0.15,\"profit\":-0.15,\"avg\":-0.075,\"avg_bars\":3,\"largest\":-0.1,\"bars_in_largest\":4,\"max_consecutive\":2}"), "{j}");
        assert!(j.contains("\"profit_factor\":4"), "{j}");
    }

    #[test] fn system_metrics_on_a_known_line() {
        // straight line: no drawdown, zero standard error → ratios that divide by them are null, not infinite
        let rows: Vec<Row> = (0..5).map(|i| row(1., 1., 1., 1. + i as f64)).collect();
        let eq = [1.0, 1.1, 1.2, 1.3, 1.4];
        let m: std::collections::BTreeMap<_, _> = system(&rows, &eq, &[], 0.1, 0.5, 252., DEFAULT_RATES).into_iter().collect();
        assert!((m["net_profit"] - 0.4).abs() < 1e-12 && (m["rar"] - 0.2).abs() < 1e-12 && m["max_sys_dd"] == 0.);
        assert!(m["recovery_factor"].is_nan() && m["ulcer_index"] == 0. && m["std_error"] < 1e-12);
        // one 50 % fall from the peak: max system dd -50 %, absolute 1.0 of initial equity, ulcer = sqrt(50² / 3)
        let m: std::collections::BTreeMap<_, _> = system(&rows, &[1.0, 2.0, 1.0], &[], 0.0, 1.0, 252., DEFAULT_RATES).into_iter().collect();
        assert!((m["max_sys_dd"] + 0.5).abs() < 1e-12 && (m["max_sys_dd_abs"] + 1.0).abs() < 1e-12);
        assert!((m["ulcer_index"] - (2500.0_f64 / 3.).sqrt()).abs() < 1e-9 && m["recovery_factor"] == 0.);
    }

    #[test] fn sharpe_of_trades_annualises_mean_linearly_and_deviation_by_square_root() {
        // two trades of +2 % and 0 % over 10 bars each (entry and exit bar included), 250 bars a year: f = 25, mean 1 % → 25 %, population sd 1 % → 5 %
        let rows: Vec<Row> = (0..21).map(|_| row(1., 1., 1., 1.)).collect();
        let t = |ret: f64, a: usize| ReportTrade { dir: 1., entry_i: a, exit_i: a + 9, entry_px: 1., ret, profit: ret };
        let m: std::collections::BTreeMap<_, _> = system(&rows, &[1.0, 1.02, 1.02], &[t(0.02, 0), t(0.0, 10)], 0.1, 1.0, 250., DEFAULT_RATES).into_iter().collect();
        assert!((m["sharpe_trades"] - (25. - 5.) / 5.).abs() < 1e-9, "{}", m["sharpe_trades"]);
    }

    #[test] fn amibroker_years_car_exposure_and_k_ratio() {
        let rows: Vec<Row> = ["2020-01-01", "2020-07-01", "2021-01-01", "2021-01-02"].iter().map(|d| Row { date: d.to_string(), ..row(1., 1., 1., 1.) }).collect();
        assert!((years(&rows, 0, 2) - 366. / 365.).abs() < 1e-12);
        assert!((car(&[1., 1.21], 2.) - 0.1).abs() < 1e-12);
        // entry bar 1 to exit bar 3 holds bars 1 and 2 (exit bar not counted); bars 0..=3 → 2/4
        let t = ReportTrade { dir: 1., entry_i: 1, exit_i: 3, entry_px: 1., ret: 0., profit: 0. };
        assert_eq!(exposure(&[t, t], 0, 3), 0.5);
        // K-ratio = slope / (SE of the slope × bars), SE of the slope = SE / √Σ(x − x̄)²
        let eq = [1., 1.2, 1.1, 1.4, 1.3];
        let (slope, se, sxx) = regression(&eq);
        let m: std::collections::BTreeMap<_, _> = system(&rows, &eq, &[], 0., 1., 252., DEFAULT_RATES).into_iter().collect();
        assert!((m["k_ratio"] - slope / (se / sxx.sqrt()) / 5.).abs() < 1e-12);
    }

    #[test] fn trade_drawdown_is_close_to_close_inside_the_trade() {
        let rows = vec![row(100., 100., 100., 100.), row(0., 0., 0., 120.), row(0., 0., 0., 90.), row(0., 0., 0., 110.)];
        let t = ReportTrade { dir: 1., entry_i: 0, exit_i: 3, entry_px: 100., ret: 0.1, profit: 0.1 };
        assert!((trade_drawdown(&rows, &t) - (0.9 / 1.2 - 1.)).abs() < 1e-12);
        // a short marks 2 − close/entry: closes 100, 80, 95 → 1, 1.2, 1.05
        let rows = vec![row(0., 0., 0., 100.), row(0., 0., 0., 80.), row(0., 0., 0., 95.)];
        let t = ReportTrade { dir: -1., entry_i: 0, exit_i: 2, entry_px: 100., ret: 0.05, profit: 0.05 };
        assert!((trade_drawdown(&rows, &t) - (1.05 / 1.2 - 1.)).abs() < 1e-12);
    }

    #[test] fn profit_factor_weights_trades_by_money_and_sides_split_by_direction() {
        // a 10 % loss on 1.0 of equity, then a 10 % win on 2.0: the % sum says 1, the money says 2 (AmiBroker)
        let t = |ret: f64, profit: f64, dir: f64| ReportTrade { dir, entry_i: 0, exit_i: 1, entry_px: 1., ret, profit };
        let ts = [t(-0.1, -0.1, 1.), t(0.1, 0.2, -1.)];
        let rows: Vec<Row> = (0..2).map(|_| row(1., 1., 1., 1.)).collect();
        let j = report_json(&rows, &[1., 1.1], &ts, 0.1, 1., 252., &[1., 1.], 0., DEFAULT_RATES);
        let v = crate::lab::json::parse(&j).unwrap();
        let get = |path: &[&str]| path.iter().fold(Some(&v), |o, k| o.and_then(|o| o.get(k))).and_then(|x| x.as_f64());
        assert_eq!((get(&["all", "profit_factor"]), get(&["all", "payoff"])), (Some(2.), Some(2.)));
        assert_eq!((get(&["all", "trades"]), get(&["long", "trades"]), get(&["short", "trades"])), (Some(2.), Some(1.), Some(1.)));
        assert_eq!((get(&["short", "losers", "count"]), get(&["long", "winners", "count"])), (Some(0.), Some(0.)));
        assert_eq!((get(&["buy_hold", "net_profit"]), get(&["buy_hold", "exposure"])), (Some(0.), Some(1.)));
    }

    #[test] fn buyhold_column_has_the_all_keys_as_one_trade() {
        let t = ReportTrade { dir: 1., entry_i: 0, exit_i: 1, entry_px: 1., ret: 0.1, profit: 0.1 };
        let rows: Vec<Row> = (0..4).map(|_| row(1., 1., 1., 1.)).collect();
        let v = crate::lab::json::parse(&report_json(&rows, &[1., 1.1], &[t], 0.1, 1., 252., &[1., 0.9, 1.2, 1.25], 0.2, DEFAULT_RATES)).unwrap();
        let keys = |o: &crate::lab::json::Json| match o { crate::lab::json::Json::Obj(kv) => kv.iter().map(|(k, v)| (k.clone(), matches!(v, crate::lab::json::Json::Obj(_)))).collect::<Vec<_>>(), _ => panic!() };
        let (all, bh) = (v.get("all").unwrap(), v.get("buyhold").unwrap());
        assert_eq!(keys(all), keys(bh));
        assert_eq!(keys(all.get("winners").unwrap()), keys(bh.get("winners").unwrap()));
        let get = |k: &[&str]| k.iter().fold(Some(bh), |o, k| o.and_then(|o| o.get(k))).cloned().unwrap();
        assert_eq!((get(&["trades"]).as_f64(), get(&["avg_bars"]).as_f64(), get(&["winners", "count"]).as_f64()), (Some(1.), Some(4.), Some(1.)));
        assert!((get(&["avg"]).as_f64().unwrap() - 0.25).abs() < 1e-9);
        // one winning trade: no loser, so profit factor and payoff do not apply
        assert_eq!((get(&["profit_factor"]), get(&["payoff"]), get(&["losers", "avg"])), (crate::lab::json::Json::Null, crate::lab::json::Json::Null, crate::lab::json::Json::Null));
    }

    #[test] fn risk_free_rates_default_to_amibroker_and_are_echoed() {
        use crate::lab::json::parse;
        assert_eq!(rates(&parse("{}").unwrap()).unwrap(), (5., 5.4));
        assert_eq!(rates(&parse(r#"{"rf_sharpe":0,"rf_upi":3}"#).unwrap()).unwrap(), (0., 3.));
        for bad in [r#"{"rf_sharpe":-1}"#, r#"{"rf_upi":101}"#, r#"{"rf_upi":"5"}"#, r#"{"rf":5}"#, "5"] { assert!(rates(&parse(bad).unwrap()).is_err(), "{bad}"); }
        // the Sharpe of trades and UPI move by exactly the rate change
        let rows: Vec<Row> = (0..21).map(|_| row(1., 1., 1., 1.)).collect();
        let t = |ret: f64, a: usize| ReportTrade { dir: 1., entry_i: a, exit_i: a + 9, entry_px: 1., ret, profit: ret };
        let sys = |r| -> std::collections::BTreeMap<_, _> { system(&rows, &[1.0, 1.02, 0.9, 1.02], &[t(0.02, 0), t(0.0, 10)], 0.1, 1.0, 250., r).into_iter().collect() };
        let (a, b) = (sys(DEFAULT_RATES), sys((0., 0.)));
        assert!((b["sharpe_trades"] - a["sharpe_trades"] - 5. / 5.).abs() < 1e-9);
        assert!((b["upi"] - a["upi"] - 5.4 / a["ulcer_index"]).abs() < 1e-9);
        let j = report_json(&rows, &[1., 1.1], &[], 0.1, 1., 252., &[1., 1.], 0., (2.5, 3.));
        assert!(j.contains(r#""rates":{"sharpe":2.5,"upi":3}"#) && j.contains("minus 2.5 % risk-free") && j.contains("uses a 3 % risk-free"), "{j}");
        assert!(report_json(&rows, &[1., 1.1], &[], 0.1, 1., 252., &[1., 1.], 0., DEFAULT_RATES).contains("minus 5 % risk-free"));
    }
}
