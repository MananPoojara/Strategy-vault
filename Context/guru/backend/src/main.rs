// Guru backtest API — Rust does the computation only, returns JSON; the UI lives in ../frontend.
// std only, same style as Context/main.rs.
//   cargo run --release -- [idx_data.csv] [port=8090] [india_vix dir]
mod books;
mod connors;
mod readiness;
mod gate;
mod accounting;
mod baskets;
mod basket_portfolio;
mod instrument_policy;
mod trade_validation;

use connors::{Ctx, Params, Status, Trade};
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct Row {
    pub ticker: String,
    pub date: String, // YYYY-MM-DD
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: Option<f64>,
}

#[cfg(test)]
mod guru_ownership_tests {
    use super::*;
    #[test]
    fn catalogues_are_separate_and_unsupported_ross_methods_have_no_claims() {
        let connors = guru_catalogue("connors");
        let ross = guru_catalogue("ross");
        assert!(!connors.is_empty());
        assert!(!ross.is_empty());
        assert!(connors.iter().all(|d| !d.book.starts_with("jr_")));
        assert!(connors.iter().all(|d| d.book != books::chande::BOOK));
        assert!(ross.iter().all(|d| d.book.starts_with("jr_")));
        assert!(ross.iter().all(|d| d.status != Status::Live && d.claim.is_none()));
        assert!(guru_catalogue("unknown").is_empty());
        let chande= guru_catalogue("chande");
        assert_eq!(chande.len(),5);
        assert!(chande.iter().all(|d|d.book==books::chande::BOOK && d.claim.is_none()));
    }
    #[test]
    fn caller_cannot_override_the_route_owner() {
        let q = BTreeMap::from([("_guru".into(), "connors".into())]);
        assert_eq!(owned_query(&q, "ross").get("_guru").unwrap(), "ross");
    }
}

const BENCHMARK: &str = "NIFTY 50"; // Liquidity Filter denominator (course used the S&P 500)

// ---------------------------------------------------------------- data

/// DD-MM-YYYY (Indian default), YYYY-MM-DD or D/M/YYYY -> YYYY-MM-DD.
fn normalize_date(s: &str) -> Option<String> {
    let p: Vec<i64> = s.trim().split(|c| c == '-' || c == '/').map(|x| x.trim().parse().ok()).collect::<Option<_>>()?;
    if p.len() != 3 {
        return None;
    }
    let (y, d, m) = if p[0] > 31 { (p[0], p[2], p[1]) } else { (p[2], p[0], p[1]) };
    ((1..=12).contains(&m) && (1..=31).contains(&d)).then(|| format!("{y:04}-{m:02}-{d:02}"))
}

// parse_csv_fields / parse_csv_number: copied verbatim from Context/main.rs
// (quoted fields, "22,331.40"-style thousands separators).
fn parse_csv_fields(line: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '"' => {
                if in_quotes && matches!(chars.peek(), Some('"')) {
                    current.push('"');
                    chars.next();
                } else {
                    in_quotes = !in_quotes;
                }
            }
            ',' if !in_quotes => {
                fields.push(current.trim().to_string());
                current.clear();
            }
            _ => current.push(ch),
        }
    }

    fields.push(current.trim().to_string());
    fields
}

fn parse_csv_number(value: &str) -> Result<f64, std::num::ParseFloatError> {
    value.trim_matches('"').replace(',', "").parse::<f64>()
}

fn load_csv(path: &str) -> Result<BTreeMap<String, Vec<Row>>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
    let mut lines = text.lines();
    let head: Vec<&str> = lines.next().ok_or("empty csv")?.split(',').map(str::trim).collect();
    let col = |n: &str| head.iter().position(|h| *h == n).ok_or(format!("column {n} missing"));
    let (ti, di, oi, hi, li, ci) = (col("Ticker")?, col("Date")?, col("Open")?, col("High")?, col("Low")?, col("Close")?);
    let vi = col("Volume").ok();
    let mut out: BTreeMap<String, Vec<Row>> = BTreeMap::new();
    let mut dropped = 0;
    for line in lines.filter(|l| !l.trim().is_empty()) {
        let f = parse_csv_fields(line);
        let num = |i: usize| f.get(i).and_then(|v| parse_csv_number(v).ok());
        let (Some(date), Some(o), Some(h), Some(l), Some(c)) =
            (f.get(di).and_then(|d| normalize_date(d)), num(oi), num(hi), num(li), num(ci))
        else {
            dropped += 1;
            continue;
        };
        let volume = vi.and_then(num).filter(|v| *v > 0.0);
        let ticker = f[ti].clone();
        // User-approved Nifty research cutoff applies BEFORE indicator warm-up.
        // The original source CSV is retained unchanged.
        if ticker=="NIFTY 50" && date.as_str()<"2000-01-01" {continue;}
        out.entry(ticker.clone()).or_default().push(Row { ticker, date, open: o, high: h, low: l, close: c, volume });
    }
    for rows in out.values_mut() {
        rows.sort_by(|a, b| a.date.cmp(&b.date));
        rows.dedup_by(|a, b| a.date == b.date);
    }
    if dropped > 0 {
        eprintln!("warning: dropped {dropped} unparseable rows");
    }
    Ok(out)
}

/// India VIX 1-minute files (Ticker,Date,Time,Expiry,Open,High,Low,Close,...[,Padding Flag])
/// -> daily OHLC: open = first minute, high/low = extremes, close = last minute.
/// Rows with Padding Flag = 1 are vendor fillers, not prints, so they are skipped.
fn load_vix_dir(dir: &str) -> Vec<Row> {
    let mut bars: BTreeMap<(String, String), [f64; 4]> = BTreeMap::new(); // (date, time) dedups file overlap
    let mut identity:Option<String>=None;
    let Ok(entries) = std::fs::read_dir(dir) else { return Vec::new() };
    for e in entries.flatten().filter(|e| e.path().extension().map_or(false, |x| x == "csv")) {
        let Ok(text) = std::fs::read_to_string(e.path()) else { continue };
        let mut lines = text.lines();
        let head: Vec<&str> = lines.next().unwrap_or("").split(',').map(str::trim).collect();
        let col = |n: &str| head.iter().position(|h| *h == n);
        let (Some(si), Some(di), Some(ti), Some(oi), Some(hi), Some(li), Some(ci)) = (col("Ticker"), col("Date"), col("Time"), col("Open"), col("High"), col("Low"), col("Close")) else { continue };
        let pad = col("Padding Flag");
        for line in lines {
            let f: Vec<&str> = line.split(',').collect();
            if pad.and_then(|p| f.get(p)).map_or(false, |v| v.trim() == "1") {
                continue;
            }
            let num = |i: usize| f.get(i).and_then(|v| v.trim().parse::<f64>().ok());
            let (Some(d), Some(t), Some(o), Some(h), Some(l), Some(c)) = (f.get(di).and_then(|d| normalize_date(d)), f.get(ti), num(oi), num(hi), num(li), num(ci)) else { continue };
            let Some(symbol)=f.get(si).map(|s|s.trim()).filter(|s|!s.is_empty()) else {return Vec::new()};
            if identity.as_deref().is_some_and(|old|old!=symbol){eprintln!("warning: volatility source contains mixed instrument identities; admission disabled");return Vec::new();}
            if identity.is_none(){identity=Some(symbol.into());}
            bars.insert((d, t.trim().to_string()), [o, h, l, c]);
        }
    }
    // The 09:15 bar of India VIX repeats the previous close (NSE's opening print): it keeps the day's open but does not set
    // the high/low when later minutes exist (it set the low on 78% of days). Daily one-bar files are unaffected.
    let mut out: Vec<Row> = Vec::new();
    let mut open_print = false;
    for ((date, time), [o, h, l, c]) in bars {
        match out.last_mut() {
            Some(r) if r.date == date => {
                if open_print {
                    (r.high, r.low, open_print) = (h, l, false);
                } else {
                    r.high = r.high.max(h);
                    r.low = r.low.min(l);
                }
                r.close = c;
            }
            _ => {
                open_print = identity.as_deref()==Some("INDIA VIX")&&time.starts_with("09:15");
                out.push(Row { ticker: identity.clone().unwrap_or_default(), date, open: o, high: h, low: l, close: c, volume: None });
            }
        }
    }
    out
}

/// External data (extra_data/): `<SERIES>.csv` applies to every ticker, `<TICKER>__<SERIES>.csv` to one ticker
/// (spaces in the ticker written as `_`). Columns: Date + one or more numeric columns; one column -> key SERIES,
/// several -> SERIES.<Column>. Returns (ticker or "*", key) -> (date -> value).
fn load_extra(dir: &str) -> BTreeMap<(String, String), BTreeMap<String, f64>> {
    let mut out: BTreeMap<(String, String), BTreeMap<String, f64>> = BTreeMap::new();
    let Ok(entries) = std::fs::read_dir(dir) else { return out };
    for e in entries.flatten().filter(|e| e.path().extension().map_or(false, |x| x.eq_ignore_ascii_case("csv"))) {
        let stem = e.path().file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
        let (ticker, series) = match stem.split_once("__") {
            Some((t, s)) => (t.replace('_', " "), s.to_string()),
            None => ("*".to_string(), stem),
        };
        let Ok(text) = std::fs::read_to_string(e.path()) else { continue };
        let mut lines = text.lines();
        let head = parse_csv_fields(lines.next().unwrap_or(""));
        let Some(di) = head.iter().position(|h| h.trim().eq_ignore_ascii_case("date")) else { continue };
        let cols: Vec<usize> = (0..head.len()).filter(|&i| i != di).collect();
        for line in lines {
            let f = parse_csv_fields(line);
            let Some(date) = f.get(di).and_then(|d| normalize_date(d)) else { continue };
            for &i in &cols {
                if let Some(v) = f.get(i).and_then(|x| parse_csv_number(x).ok()) {
                    let key = if cols.len() == 1 { series.clone() } else { format!("{series}.{}", head[i].trim()) };
                    out.entry((ticker.clone(), key)).or_default().insert(date.clone(), v);
                }
            }
        }
    }
    out
}

// ---------------------------------------------------------------- stats

#[derive(Debug)]
struct Run {
    trades: Vec<Trade>,
    from: usize,
    to: usize,
    equity: Vec<f64>, // strategy equity, one value per bar in [from, to]
    bh: Vec<f64>,     // buy & hold equity (long; short strategies still compare to B&H)
    source_account:Option<books::elcim_account::Account>,
}

fn backtest(ctx: &Ctx, id: &str, from: usize, to: usize, cost: f64, p: &Params) -> Result<Run,String> {
    if id==books::elcim_account::ID {
        let account=books::elcim_account::run(ctx,from,to,cost,p)?;
        trade_validation::check(&account.trades,from,to,cost)?;
        return Ok(Run{trades:account.trades.clone(),from,to,equity:account.days.iter().map(|d|d.equity).collect(),bh:(from..=to).map(|k|ctx.c[k]/ctx.c[from]).collect(),source_account:Some(account)});
    }
    let trades = connors::run(id, ctx, from, to, p).ok_or("Strategy implementation unavailable")?;
    trade_validation::check(&trades,from,to,cost)?;
    let equity = equity_curve(&ctx.c, &trades, from, to, cost);
    if equity.iter().any(|v|!v.is_finite()||*v<=0.) {return Err("calculation_unavailable: Unsupported nonpositive or nonfinite normalized equity; no reset, clipping or invented funding is applied.".into());}
    let bh = (from..=to).map(|k| ctx.c[k] / ctx.c[from]).collect();
    Ok(Run { trades, from, to, equity, bh,source_account:None })
}

/// Daily equity: each trade's legs marked to the close (scale-out pieces at their fill once out).
fn equity_curve(close: &[f64], trades: &[Trade], from: usize, to: usize, cost: f64) -> Vec<f64> {
    let n = to - from + 1;
    let mut equity = vec![1.0; n];
    let mut cur = 1.0;
    let mut next = from;
    for t in trades {
        let start = t.legs[0].i;
        for k in next..start {
            equity[k - from] = cur;
        }
        let e0 = cur;
        for k in start..=t.exit_i {
            let px = if k == t.exit_i { t.exit_px } else { t.mark(k, close[k]) };
            let mut v = 0.0;
            for l in t.legs.iter().filter(|l| l.i <= k) {
                v += l.w * (t.dir * (px / l.px - 1.0) - cost);
                if k == t.exit_i {
                    v -= l.w * cost;
                }
            }
            equity[k - from] = e0 * (1.0 + v);
        }
        cur = equity[t.exit_i - from];
        next = t.exit_i + 1;
    }
    for k in next..=to {
        equity[k - from] = cur;
    }
    equity
}

fn max_dd(eq: &[f64]) -> (f64, usize) {
    // Position zero is the declared pre-event unit baseline, not a fabricated quote/date.
    let (mut peak, mut dd, mut peak_i, mut longest) = (1.0, 0.0f64, 0, 0);
    for (i, &v) in eq.iter().enumerate() {
        if v >= peak {
            peak = v;
            peak_i = i+1;
        }
        dd = dd.min(v / peak - 1.0);
        longest = longest.max(i+1 - peak_i);
    }
    (dd, longest)
}

/// Calendar days since 1970-01-01 (Howard Hinnant's days_from_civil).
fn day_number(date: &str) -> i64 {
    let (y, m, d): (i64, i64, i64) = (date[0..4].parse().unwrap(), date[5..7].parse().unwrap(), date[8..10].parse().unwrap());
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (m + if m > 2 { -3 } else { 9 }) + 2) / 5 + d - 1;
    era * 146_097 + (yoe * 365 + yoe / 4 - yoe / 100 + doy) - 719_468
}

/// Test length in years (actual calendar days / 365.25), used for CAGR.
fn years(ctx: &Ctx, from: usize, to: usize) -> f64 {
    ((day_number(&ctx.r[to].date) - day_number(&ctx.r[from].date)) as f64 / 365.25).max(1.0 / 365.25)
}

fn stats(ctx: &Ctx, run: &Run, cost: f64) -> BTreeMap<&'static str, f64> {
    let mut s = BTreeMap::new();
    let eq = &run.equity;
    let yrs = years(ctx, run.from, run.to);
    let rets: Vec<f64> = std::iter::once(eq[0]-1.).chain(eq.windows(2).map(|w| w[1] / w[0] - 1.0)).collect();
    let nr = rets.len().max(1) as f64;
    let mean = rets.iter().sum::<f64>() / nr;
    let sd = (rets.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / (nr - 1.0).max(1.0)).sqrt();
    let dsd = (rets.iter().map(|r| r.min(0.0).powi(2)).sum::<f64>() / nr).sqrt();
    let total = eq.last().unwrap() - 1.0;
    let cagr = eq.last().unwrap().powf(1.0 / yrs) - 1.0;
    let (mdd, dd_days) = max_dd(eq);
    let bh_last = *run.bh.last().unwrap();
    // Exposure: share of bars on which a position was held (a bar shared by an exit at the
    // open and a new entry at the close counts once).
    let mut held = vec![false; eq.len()];
    for t in &run.trades {
        held[t.legs[0].i - run.from..=t.exit_i - run.from].iter_mut().for_each(|h| *h = true);
    }
    let in_mkt = held.iter().filter(|h| **h).count();

    // Trade statistics use CLOSED trades only; a still-open trade is in the equity curve
    // (marked to market) but has no final result yet.
    let closed: Vec<&Trade> = run.trades.iter().filter(|t| !t.open).collect();
    let tr: Vec<f64> = closed.iter().map(|t| t.ret(cost)).collect();
    let n = tr.len() as f64;
    let wins: Vec<f64> = tr.iter().copied().filter(|r| *r > 0.0).collect();
    let losses: Vec<f64> = tr.iter().copied().filter(|r| *r <= 0.0).collect();
    let avg = tr.iter().sum::<f64>() / n;
    let tsd = (tr.iter().map(|r| (r - avg).powi(2)).sum::<f64>() / (n - 1.0)).sqrt();
    let gross_win: f64 = wins.iter().sum();
    let gross_loss: f64 = -losses.iter().sum::<f64>();
    let mut sorted = tr.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let (mut streak, mut max_streak) = (0, 0);
    for r in &tr {
        streak = if *r <= 0.0 { streak + 1 } else { 0 };
        max_streak = max_streak.max(streak);
    }
    let avg_win = wins.iter().sum::<f64>() / wins.len() as f64;
    let avg_loss = losses.iter().sum::<f64>() / losses.len() as f64;

    s.insert("total_return", total);
    s.insert("cagr", cagr);
    s.insert("vol", sd * 252f64.sqrt());
    s.insert("sharpe", mean / sd * 252f64.sqrt());
    s.insert("sortino", mean / dsd * 252f64.sqrt());
    s.insert("max_dd", mdd);
    s.insert("max_dd_days", dd_days as f64);
    s.insert("mar", cagr / -mdd); // MAR ratio: CAGR / |max DD| over the whole test (Calmar uses 36 months)
    s.insert("exposure", in_mkt as f64 / eq.len() as f64);
    s.insert("bh_total", bh_last - 1.0);
    s.insert("bh_cagr", bh_last.powf(1.0 / yrs) - 1.0);
    s.insert("bh_max_dd", max_dd(&run.bh).0);
    s.insert("years", yrs);
    s.insert("trades", n);
    s.insert("win_rate", wins.len() as f64 / n);
    s.insert("avg_trade", avg);
    let m = sorted.len();
    s.insert("median_trade", match m {
        0 => f64::NAN,
        _ if m % 2 == 1 => sorted[m / 2],
        _ => (sorted[m / 2 - 1] + sorted[m / 2]) / 2.0,
    });
    s.insert("avg_win", avg_win);
    s.insert("avg_loss", avg_loss);
    s.insert("payoff", avg_win / -avg_loss);
    s.insert("profit_factor", gross_win / gross_loss);
    s.insert("best", sorted.last().copied().unwrap_or(f64::NAN));
    s.insert("worst", sorted.first().copied().unwrap_or(f64::NAN));
    s.insert("t_stat", avg / (tsd / n.sqrt()));
    s.insert("max_consec_losses", max_streak as f64);
    s.insert("avg_bars", closed.iter().map(|t| (t.exit_i - t.legs[0].i) as f64).sum::<f64>() / n);
    s.insert("avg_deployed", closed.iter().map(|t| t.deployed()).sum::<f64>() / n);
    s.insert("open_trade", run.trades.iter().find(|t| t.open).map_or(f64::NAN, |t| t.ret(cost)));
    s.insert("trades_per_year", n / yrs);
    // Conclusion stats (the user's definitions; the Excel "Formulas" sheet recomputes each from the trade list):
    // net profit = sum of closed-trade net returns; the separately declared
    // compounded model supplies CAGR and CAR/MDD (CAGR / |max DD|).
    // expectancy ratio = (win% x avg win - loss% x |avg loss|) / |avg loss|  (Van Tharp: expectancy per unit of average loss);
    // monthly thickness = avg trade return / avg holding days x 30, holding days = calendar days entry -> exit (a same-day trade counts 1).
    let hold_days = closed.iter().map(|t| ((day_number(&ctx.r[t.exit_i].date) - day_number(&ctx.r[t.legs[0].i].date)) as f64).max(1.0)).sum::<f64>() / n;
    let (wr, al) = (wins.len() as f64 / n, -avg_loss);
    let exp_ratio = if losses.is_empty() { f64::NAN } else { (wr * if wins.is_empty() { 0.0 } else { avg_win } - (1.0 - wr) * al) / al };
    s.insert("net_profit",tr.iter().sum());
    s.insert("closed_return_sum",tr.iter().sum());
    s.insert("compounded_return",total);
    s.insert("car_mdd", cagr / -mdd);
    s.insert("expectancy_ratio", exp_ratio);
    s.insert("avg_hold_days", hold_days);
    s.insert("monthly_thickness", avg / hold_days * 30.0);
    s
}

/// Additive return accounting. Monetary sizing and exact intraday MAE/MFE are
/// deliberately unavailable; changing a cost input does not manufacture them.
fn financial_stats(ctx: &Ctx, run: &Run, costs: accounting::Costs) -> BTreeMap<&'static str,f64> {
    let mut s=stats(ctx,run,costs.side_rate());
    let gross=if let Some(account)=&run.source_account {
        let zero=Run{trades:run.trades.clone(),from:run.from,to:run.to,equity:account.days.iter().map(|d|d.gross).collect(),bh:run.bh.clone(),source_account:None};
        stats(ctx,&zero,0.)
    } else if costs.side_rate()==0. {s.clone()} else {
        let zero=Run{trades:run.trades.clone(),from:run.from,to:run.to,equity:equity_curve(&ctx.c,&run.trades,run.from,run.to,0.),bh:run.bh.clone(),source_account:None};
        stats(ctx,&zero,0.)
    };
    for (key,source) in [("gross_total_return","total_return"),("gross_avg_trade","avg_trade"),("gross_win_rate","win_rate"),("gross_sharpe","sharpe"),("gross_max_dd","max_dd")] {s.insert(key,gross[source]);}
    s.insert("avg_net_pnl",s["avg_trade"]); // average net return per closed trade, not an INR amount
    let (mut allocation,mut ledger_gross,mut fee_return,mut slippage_return)=(1.,0.,0.,0.);
    for (i,t) in run.trades.iter().enumerate() {
        if let Some(account)=&run.source_account {
            let audit=&account.trade_audit[i];let sides=if t.open {1.}else{2.};
            ledger_gross+=audit.gross;fee_return+=sides*audit.notional*costs.fees_bps/10000.;slippage_return+=sides*audit.notional*costs.slippage_bps/10000.;
        } else {
            // Preserve the legacy O(trades) accounting path; summary/optimization
            // must not gain a full held-bar excursion pass for this source batch.
            ledger_gross+=allocation*t.ret(0.);fee_return+=allocation*2.*t.deployed()*costs.fees_bps/10000.;slippage_return+=allocation*2.*t.deployed()*costs.slippage_bps/10000.;allocation*=1.+t.ret(costs.side_rate());
        }
    }
    if let Some(account)=&run.source_account {s.insert("open_trade",account.trades.iter().enumerate().find(|(_,t)|t.open).map_or(f64::NAN,|(i,_)|account.trade_audit[i].net/account.trade_audit[i].entry_equity));}
    s.insert("ledger_gross_return",ledger_gross);
    s.insert("fee_return",fee_return);s.insert("slippage_return",slippage_return);
    s
}

fn benchmark_stats(ctx:&Ctx,run:&Run)->BTreeMap<&'static str,f64>{
    let benchmark=Run{trades:vec![],from:run.from,to:run.to,equity:run.bh.clone(),bh:run.bh.clone(),source_account:None};
    let all=stats(ctx,&benchmark,0.);
    ["total_return","cagr","sharpe","max_dd","vol","sortino","mar"].into_iter().map(|k|(k,all[k])).collect()
}

// ---------------------------------------------------------------- json

fn num(v: f64) -> String {
    if v.is_finite() { format!("{}", (v * 1e6).round() / 1e6) } else { "null".into() }
}
fn full_precision_trade_return(v:f64)->String{
    if v.is_finite(){v.to_string()}else{"null".into()}
}
fn esc(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}
fn stats_json(s: &BTreeMap<&str, f64>) -> String {
    let body: Vec<String> = s.iter().map(|(k, v)| format!("\"{k}\":{}", num(*v))).collect();
    format!("{{{}}}", body.join(","))
}

fn guru_catalogue(guru: &str) -> Vec<connors::StrategyDef> {
    connors::catalogue().into_iter().filter(|d| match guru {
        "connors" => !d.book.starts_with("jr_") && d.book!=books::chande::BOOK,
        "ross" => d.book.starts_with("jr_"),
        "chande" => d.book==books::chande::BOOK,
        _ => false,
    }).collect()
}
fn owned_query(q: &BTreeMap<String, String>, guru: &str) -> BTreeMap<String, String> {
    let mut owned=q.clone();
    owned.insert("_guru".into(), guru.into());
    owned
}
fn query_catalogue(q: &BTreeMap<String, String>) -> Vec<connors::StrategyDef> {
    guru_catalogue(q.get("_guru").map_or("connors", String::as_str))
}
fn experiment_json(id:&str)->String {
    if !id.starts_with("ch_"){return "null".into();}
    format!("{{\"kind\":\"market_adaptation\",\"source_study_reproduced\":false,\"source_edition\":1997,\"source_partial\":true,\"basis\":\"{}\"}}",esc(books::chande::BASIS))
}
fn catalogue_json(guru: &str) -> String {
    let items: Vec<String> = guru_catalogue(guru)
        .iter()
        .map(|d| {
            let gated = gate::get(d.id);
            let (status, why) = match d.status {
                Status::Live if gated.is_some_and(|g| g.verdict == "withheld") && !instrument_policy::reviewed_cash_rule(d) => ("not_testable", gated.map_or("", |g| g.reason)),
                Status::Live if instrument_policy::policy(d).requirement=="source_review_required" => ("not_testable", "Source-specific rules and required instrument have not completed admission review; no backtest is published."),
                Status::Live => ("live", ""),
                Status::NeedsData(w) => ("needs_data", w),
                Status::NotTestable(w) => ("not_testable", w),
            };
            let rules: Vec<String> = d.rules.iter().map(|r| format!("\"{}\"", esc(r))).collect();
            let claim = d.claim.as_ref().map_or("null".into(), |c| {
                format!(
                    "{{\"trades\":\"{}\",\"win\":\"{}\",\"avg\":\"{}\",\"hold\":\"{}\",\"period\":\"{}\"}}",
                    esc(c.trades), esc(c.win), esc(c.avg), esc(c.hold), esc(c.period)
                )
            });
            format!(
                "{{\"id\":\"{}\",\"name\":\"{}\",\"group\":\"{}\",\"session\":{},\"dir\":\"{}\",\"status\":\"{}\",\"status_note\":\"{}\",\"needs_volume\":{},\"needs_vix\":{},\"needs_series\":[{}],\"book\":\"{}\",\"family\":\"{}\",\"family_name\":\"{}\",\"variant\":\"{}\",\"rules\":[{}],\"notes\":\"{}\",\"claim\":{},\"instrument_policy\":{},\"gate\":{}}}",
                d.id, esc(d.name), esc(d.group), d.session, d.dir, status, esc(why), d.needs_volume, d.needs_vix, d.needs_series.iter().map(|x| format!("\"{}\"", esc(x))).collect::<Vec<_>>().join(","), d.book, connors::family_of(d.id).0, esc(connors::family_of(d.id).1), esc(connors::family_of(d.id).2), rules.join(","), esc(d.notes), claim, instrument_policy::metadata_json(d), gated.map_or("null".into(), |g| format!("{{\"verdict\":\"{}\",\"note\":\"{}\",\"allowed\":{},\"open_from\":{},\"book\":\"{}\",\"ours\":\"{}\"}}", if g.verdict=="verified" {"table_compared"} else {g.verdict}, esc(g.reason), g.allowed.map_or("null".into(), |a| format!("[{}]", a.iter().map(|t| format!("\"{}\"", t)).collect::<Vec<_>>().join(","))), g.clamp_open, esc(g.book), esc(g.ours)))
            )
        })
        .collect();
    format!("[{}]", items.join(","))
}

/// Market/date-specific data admission; this does not claim book-result reproduction.
fn readiness_json(st: &State, q: &BTreeMap<String, String>) -> Result<String, String> {
    let selected=select_context(st,q)?;let held=selected.context(st);
    let (ctx,from,to,_)=resolve_context(&held,q)?;
    let costs=accounting::Costs::parse(q)?;
    let p=checked_params_of(q)?;
    let cat=query_catalogue(q);
    if q.get("id").is_some_and(|id| !cat.iter().any(|d| d.id==id)) { return Err("unknown strategy".into()); }
    let rows:Vec<String>=cat.iter().filter(|d| q.get("id").map_or(true,|id| d.id==id)).map(|d| {
        let issue=readiness::check(ctx,d,from,to,&p).or_else(||native_admission(ctx,d,from,to,&selected,st,q));
        let (mut status,note)=issue.as_ref().map_or(("ready","Required data checks passed. This is a new run on the selected instrument, not verification of the book's published results."),|i| (i.status,i.note.as_str()));
        let note=if let Some(roll)=costs.rollover_issue(){if issue.is_none(){status="needs_data";}format!("{note} {roll}")}else{note.into()};
        format!("{{\"id\":\"{}\",\"status\":\"{}\",\"note\":\"{}\"}}",d.id,status,esc(&note))
    }).collect();
    Ok(format!("{{\"ticker\":\"{}\",\"from\":\"{}\",\"to\":\"{}\",\"strategies\":[{}],\"costs\":{},\"provenance\":{}}}",esc(&ctx.r[0].ticker),ctx.r[from].date,ctx.r[to].date,rows.join(","),costs.json(),source_provenance(st,q)))
}

// ---------------------------------------------------------------- http

struct State {
    ctxs: BTreeMap<String, Ctx<'static>>,
    baskets:baskets::Catalogue,
    vix:Vec<Row>,
    market_extra:BTreeMap<(String,String),BTreeMap<String,f64>>,
}

enum SelectedContext<'a>{Preloaded(&'a Ctx<'static>),Native{rows:Vec<Row>,evidence:&'a baskets::NativeEvidence}}
enum ContextRef<'a>{Preloaded(&'a Ctx<'a>),Native(Ctx<'a>)}
impl<'a> std::ops::Deref for ContextRef<'a>{type Target=Ctx<'a>;fn deref(&self)->&Self::Target{match self{Self::Preloaded(c)=>c,Self::Native(c)=>c}}}
impl SelectedContext<'_>{
    fn context<'a>(&'a self,st:&'a State)->ContextRef<'a>{
        match self {
            Self::Preloaded(c)=>ContextRef::Preloaded(c),
            Self::Native{rows,evidence}=>{
                let benchmark=native_benchmark(evidence.market());
                let bench=st.ctxs.get(benchmark).filter(|c|!c.r.is_empty()&&c.r.iter().all(|r|r.ticker==benchmark)).map_or(&[][..],|c|c.r);
                let (source_vix,_)=market_vix(&st.vix,Some(evidence.market()));
                let mut ctx=Ctx::new(rows,bench,source_vix);
                let vix:BTreeMap<&str,&Row>=source_vix.iter().map(|r|(r.date.as_str(),r)).collect();
                for (name,field) in [("Open",0),("High",1),("Low",2),("Close",3)]{
                    let values:Vec<f64>=rows.iter().map(|r|vix.get(r.date.as_str()).map_or(f64::NAN,|v|[v.open,v.high,v.low,v.close][field])).collect();
                    ctx.extra.insert(format!("VIX_OHLC.{name}"),values.clone());ctx.extra.insert(format!("VIX.{name}"),values);
                }
                for ((symbol,key),values) in &st.market_extra {
                    if is_vix_series(key)&&self.volatility_source(st).is_none(){continue;}
                    if symbol=="*"||rows.first().is_some_and(|r|symbol==&r.ticker){ctx.extra.insert(key.clone(),rows.iter().map(|r|values.get(&r.date).copied().unwrap_or(f64::NAN)).collect());}
                }
                ContextRef::Native(ctx)
            }
        }
    }
    fn native(&self)->bool{matches!(self,Self::Native{..})}
    fn evidence(&self)->Option<&baskets::NativeEvidence>{match self {Self::Native{evidence,..}=>Some(evidence),_=>None}}
    fn volatility_source<'a>(&self,st:&'a State)->Option<&'a str>{
        let market=match self {Self::Native{evidence,..}=>Some(evidence.market()),Self::Preloaded(c)=>preloaded_market(&c.r[0].ticker)};
        market_vix(&st.vix,market).1
    }
}
fn native_benchmark(market:baskets::Market)->&'static str {match market {baskets::Market::India=>BENCHMARK,baskets::Market::UnitedStates=>"SPX"}}
fn preloaded_market(ticker:&str)->Option<baskets::Market>{
    match ticker {
        "NIFTY 50"|"NIFTY BANK"|"NIFTYBANK"|"NIFTY IT"|"NIFTYIT"|"NIFTY MIDCAP 100"|"NIFTYMIDCAP100"=>Some(baskets::Market::India),
        "SPX"|"NDX"|"DJI"|"SPY"|"QQQ"|"IWM"|"MDY"|"SMH"|"DIA"=>Some(baskets::Market::UnitedStates),
        _=>None,
    }
}
fn is_vix_series(key:&str)->bool {key=="VIX"||key=="VIX_OHLC"||key.starts_with("VIX.")||key.starts_with("VIX_OHLC.")}
fn market_vix(rows:&[Row],market:Option<baskets::Market>)->(&[Row],Option<&str>){
    let expected=match market {Some(baskets::Market::India)=>"INDIA VIX",Some(baskets::Market::UnitedStates)=>"CBOE VIX",None=>return (&[],None)};
    if !rows.is_empty()&&rows.iter().all(|r|r.ticker==expected){(rows,Some(expected))}else{(&[],None)}
}
fn select_context<'a>(st:&'a State,q:&BTreeMap<String,String>)->Result<SelectedContext<'a>,String>{
    let ticker=q.get("ticker").map(String::as_str).unwrap_or(BENCHMARK);
    if let Some(c)=st.ctxs.get(ticker){return Ok(SelectedContext::Preloaded(c));}
    if let Some(evidence)=st.baskets.native_evidence(ticker){
        for key in ["from","to"] {if q.get(key).is_some_and(|value|!value.is_empty()&&!baskets::valid_date(value)){return Err(format!("needs_data: Native requested {key} must be a valid YYYY-MM-DD date; no date is repaired."));}}
        return st.baskets.rows(ticker).map(|rows|SelectedContext::Native{rows,evidence});
    }
    Err(format!("unknown ticker {ticker}"))
}
fn source_provenance(st:&State,q:&BTreeMap<String,String>)->String{st.baskets.provenance_json(q.get("ticker").map(String::as_str).unwrap_or(BENCHMARK)).unwrap_or("null".into())}
/// Read-only chart history from the same admitted source used by strategy calculations.
/// It intentionally retains pre-period bars so indicators can use the engine's warmup.
fn chart_ohlc_json(st:&State,q:&BTreeMap<String,String>)->Result<String,String>{
    fn arr(out:&mut String,values:&mut impl Iterator<Item=String>){out.push('[');for (i,value) in values.enumerate(){if i>0{out.push(',');}out.push_str(&value);}out.push(']');}
    fn source_num(v:f64)->String{if v.is_finite(){v.to_string()}else{"null".into()}}
    let selected=select_context(st,q)?;
    let rows=match &selected {SelectedContext::Preloaded(ctx)=>ctx.r,SelectedContext::Native{rows,..}=>rows.as_slice()};
    if rows.is_empty(){return Err("Chart source has no admitted prices".into());}
    let mut out=format!("{{\"ticker\":\"{}\",\"full_history\":true,\"provenance\":{},\"d\":",esc(&rows[0].ticker),source_provenance(st,q));
    arr(&mut out,&mut rows.iter().map(|r|format!("\"{}\"",r.date)));
    for (name,field) in [("o",0),("h",1),("l",2),("c",3)]{let _=write!(out,",\"{name}\":");arr(&mut out,&mut rows.iter().map(|r|source_num([r.open,r.high,r.low,r.close][field])));}
    out.push_str(",\"v\":");arr(&mut out,&mut rows.iter().map(|r|r.volume.map_or("null".into(),source_num)));out.push('}');Ok(out)
}
fn native_admission(ctx:&Ctx,def:&connors::StrategyDef,from:usize,to:usize,selected:&SelectedContext,st:&State,q:&BTreeMap<String,String>)->Option<readiness::Issue>{
    let Some(evidence)=selected.evidence() else {return instrument_policy::check(def,&ctx.r[0].ticker,false)};
    if let Some(issue)=instrument_policy::check_native(def,&ctx.r[0].ticker,evidence){return Some(issue);}
    if q.get("from").is_some_and(|f|!f.is_empty()&&f.as_str()<ctx.r[0].date.as_str()){return Some(readiness::Issue{status:"needs_data",note:format!("Selected start precedes admitted native source coverage beginning {}. No unsupported early history is filled; review source prefix exclusions and choose covered dates.",ctx.r[0].date)});}
    let last=&ctx.r.last().unwrap().date;
    if q.get("to").is_some_and(|t|!t.is_empty()&&t.as_str()>last.as_str()){return Some(readiness::Issue{status:"needs_data",note:format!("Selected end exceeds admitted native source coverage ending {last}. Later unsupported prices are unavailable; the requested end is not silently shortened.")});}
    let benchmark=native_benchmark(evidence.market());
    let benchmark_source=st.ctxs.get(benchmark).filter(|c|!c.r.is_empty()&&c.r.iter().all(|r|r.ticker==benchmark)).map(|_|benchmark);
    if let Some(issue)=instrument_policy::check_native_market_inputs(def,evidence,benchmark_source,selected.volatility_source(st)){return Some(issue);}
    if connors::params(def.id).iter().any(|p|p.key=="lf")&&ctx.lf[from.max(19).min(to)..=to].iter().any(|v|!v.is_finite()){
        return Some(readiness::Issue{status:"needs_data",note:format!("This stock strategy requires the Liquidity Filter denominator from actual {benchmark} benchmark close and volume. Benchmark volume/coverage is missing; stock OHLC or unknown auxiliary fields cannot replace it.")});
    }
    None
}

fn query(path: &str) -> (String, BTreeMap<String, String>) {
    let (p, q) = path.split_once('?').unwrap_or((path, ""));
    let params = q
        .split('&')
        .filter_map(|kv| kv.split_once('='))
        .map(|(k, v)| (k.to_string(), pct_decode(&v.replace('+', " "))))
        .collect();
    (p.to_string(), params)
}

fn pct_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        match (b[i], s.get(i + 1..i + 3).and_then(|h| u8::from_str_radix(h, 16).ok())) {
            (b'%', Some(v)) => {
                out.push(v);
                i += 3;
            }
            (c, _) => {
                out.push(c);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Strategy parameters from `p.<key>=<value>` query params (missing = the book's value).
fn params_of(q: &BTreeMap<String, String>) -> Params {
    if q.get("id").is_some_and(|id|id==books::elcim_account::ID) {
        // Only this bounded source admission changes raw parsing. Invalid values
        // cannot silently disappear and inherit the reviewed default settings.
        return q.iter().filter_map(|(k,v)|k.strip_prefix("p.").map(|key|(key.to_owned(),v.parse().unwrap_or(f64::NAN)))).collect();
    }
    q.iter().filter_map(|(k, v)| Some((k.strip_prefix("p.")?.to_string(), v.parse().ok()?))).collect()
}

/// New exact-default reviews must not silently discard malformed query inputs.
/// Existing methods keep their previous parsing contract until separately reviewed.
fn checked_params_of(q: &BTreeMap<String, String>) -> Result<Params,String> {
    let p=params_of(q);
    if let Some(id)=q.get("id").filter(|id|books::sbma::four_reviewed_stock_rule(id)) {
        for (key,text) in q {
            if let Some(name)=key.strip_prefix("p.") {
                let value=text.parse::<f64>().map_err(|_|format!("Invalid reviewed Burns parameter: {key}"))?;
                if let Some(note)=books::sbma::validate_four_reviewed(id,&Params::from([(name.into(),value)])) {return Err(note.into());}
            }
            if let Some(name)=key.strip_prefix("g.") {
                for value in four_burns_grid(text)? {
                    if let Some(note)=books::sbma::validate_four_reviewed(id,&Params::from([(name.into(),value)])) {return Err(note.into());}
                }
            }
        }
        if let Some(note)=books::sbma::validate_four_reviewed(id,&p) {return Err(note.into());}
        return Ok(p);
    }
    if q.get("id").is_some_and(|id|id==books::elcim_account::ID) {
        if let Some(note)=books::elcim_account::validate(&p) {return Err(note);}
        let schema=books::elcim::params(books::elcim_account::ID).ok_or("Unknown protected Elder configuration")?;
        for (key,text) in q {
            if let Some(name)=key.strip_prefix("g.") {
                let Some(setting)=schema.iter().find(|d|d.key==name) else {return Err(format!("Unknown protected Elder grid parameter: {key}"));};
                protected_elder_grid(text,setting.def)?;
            }
        }
        return Ok(p);
    }
    let Some(id)=q.get("id").filter(|id|id.as_str()!="ehrs_sinetrend" && books::ehrs::reviewed_sinetrend(id)) else {return Ok(p);};
    let schema=books::ehrs::params(id).ok_or("Unknown reviewed Ehlers configuration")?;
    for (key,text) in q {
        if let Some(name)=key.strip_prefix("p.") {
            if !schema.iter().any(|d|d.key==name) || text.parse::<f64>().is_err() {
                return Err(format!("Invalid reviewed Ehlers parameter: {key}"));
            }
        }
        if let Some(name)=key.strip_prefix("g.") {
            let Some(setting)=schema.iter().find(|d|d.key==name) else {return Err(format!("Unknown reviewed Ehlers grid parameter: {key}"));};
            let unreviewed=||format!("Unreviewed Ehlers grid setting: {key}; only the exact configuration default is admitted");
            // Bound this reviewed request before the legacy grid parser casts
            // its floating count to usize. Nonfinite endpoints or huge counts
            // can otherwise overflow even though no alternate value is admitted.
            // Keep malformed syntax and ordinary legacy errors with grid().
            if let Ok(values)=text.split(':').map(|v|v.trim().parse::<f64>()).collect::<Result<Vec<_>,_>>() {
                if values.iter().any(|v|!v.is_finite()) {return Err(unreviewed());}
                if let [value]=values[..] {
                    if value!=setting.def {return Err(unreviewed());}
                }
                if let [lo,hi,step]=values[..] {
                    if step>0. && hi>=lo {
                        let steps=((hi-lo)/step+1e-9).floor();
                        if !steps.is_finite() || steps>999. {return Err(unreviewed());}
                        // Exact review applies before grid() rounds to 1e-9.
                        if (0..=steps as usize).any(|k|lo+k as f64*step!=setting.def) {return Err(unreviewed());}
                    }
                }
            }
            if grid(text)?.iter().any(|value|!value.is_finite() || *value!=setting.def) {
                return Err(unreviewed());
            }
        }
    }
    if let Some(note)=books::ehrs::validate(id,&p) {return Err(note);}
    Ok(p)
}

/// Bound only the four newly reviewed Burns grids before legacy integer conversion.
fn four_burns_grid(text:&str)->Result<Vec<f64>,String> {
    let bad=||"Reviewed Burns grid requires finite values and at most 1000 steps".to_string();
    let values:Vec<f64>=text.split(':').map(|v|v.trim().parse::<f64>()).collect::<Result<_,_>>().map_err(|_|bad())?;
    if values.iter().any(|v|!v.is_finite()) {return Err(bad());}
    let count=match values.as_slice() {
        [_]=>0., [lo,hi,step] if *step>0. && hi>=lo => (hi-lo)/step, _=>return Err(bad()),
    };
    if !count.is_finite() || count>999. {return Err(bad());}
    // Validate the unrounded values. Legacy grid() rounds to1e-9; a changed
    // near-default value must not acquire review by rounding into the default.
    let (lo,step)=match values.as_slice() {[a]=>(*a,1.),[a,_,s]=>(*a,*s),_=>unreachable!()};
    Ok((0..=(count+1e-9).floor() as usize).map(|i|lo+i as f64*step).collect())
}

/// Only this protected account gets a raw exact-default grid contract. Validate
/// before the legacy parser's rounding and floating-count-to-usize conversion.
fn protected_elder_grid(text:&str,expected:f64)->Result<(),String> {
    let bad=||"Protected Elder grid requires finite exact reviewed defaults and at most1000 values".to_string();
    let values=text.split(':').map(|v|v.trim().parse::<f64>()).collect::<Result<Vec<_>,_>>().map_err(|_|bad())?;
    if values.iter().any(|v|!v.is_finite()) {return Err(bad());}
    let (lo,step,steps)=match values.as_slice() {
        [value]=>(*value,1.,0.),
        [lo,hi,step] if *step>0. && hi>=lo => (*lo,*step,((hi-lo)/step+1e-9).floor()),
        _=>return Err(bad()),
    };
    if !steps.is_finite() || steps>999. {return Err(bad());}
    // There is one admitted value per setting: range bounds cannot encode an
    // alternate raw value even when float step/count rounding omits that bound.
    if let [lo,hi,_]=values.as_slice() {if *lo!=expected || *hi!=expected {return Err(bad());}}
    if (0..=steps as usize).any(|k|lo+k as f64*step!=expected) {return Err(bad());}
    if grid(text)?.iter().any(|v|!v.is_finite() || *v!=expected) {return Err(bad());}
    Ok(())
}

/// Resolve ticker + [from, to] (inclusive bar indices) + cost from query params.
fn resolve_context<'a>(ctx:&'a Ctx<'a>, q: &BTreeMap<String, String>) -> Result<(&'a Ctx<'a>, usize, usize, f64), String> {
    let dates: Vec<&str> = ctx.r.iter().map(|r| r.date.as_str()).collect();
    let from = q.get("from").filter(|s| !s.is_empty()).map_or(0, |f| dates.partition_point(|d| *d < f.as_str()));
    let to = q
        .get("to")
        .filter(|s| !s.is_empty())
        .map_or(dates.len() - 1, |t| dates.partition_point(|d| *d <= t.as_str()).saturating_sub(1));
    if from >= to {
        return Err("empty date range".into());
    }
    let cost = accounting::Costs::parse(q)?.side_rate(); // fees + slippage, bps per side exactly once
    Ok((ctx, from, to, cost))
}
#[cfg(test)]
fn resolve<'a>(st:&'a State,q:&BTreeMap<String,String>)->Result<(&'a Ctx<'a>,usize,usize,f64),String>{
    let t=q.get("ticker").map(String::as_str).unwrap_or(BENCHMARK);resolve_context(st.ctxs.get(t).ok_or("unknown ticker")?,q)
}

/// VIX strategies start where India VIX data starts, so their CAGR is not diluted by years they could never trade.
fn window_start(ctx: &Ctx, d: &connors::StrategyDef, from: usize) -> Option<usize> {
    // Accuracy gate: rules that fill at the open start where this index's opens are real (stale earlier).
    let from = match (gate::get(d.id), ctx.r.get(from).and_then(|r| gate::real_open_from(&r.ticker))) {
        (Some(g), Some(cutoff)) if g.clamp_open => from.max(ctx.r.partition_point(|r| r.date.as_str() < cutoff)),
        _ => from,
    };
    if !d.needs_vix {
        return Some(from);
    }
    ctx.vix.first.map(|f| from.max(f))
}

fn summary(st: &State, q: &BTreeMap<String, String>) -> Result<String, String> {
    let selected=select_context(st,q)?;let held=selected.context(st);
    let (ctx, from, to, cost) = resolve_context(&held, q)?;
    let costs=accounting::Costs::parse(q)?;costs.ensure_rollover()?;
    let mut rows = Vec::new();
    for d in query_catalogue(q).iter() {
        if let Some(issue) = readiness::check(ctx, d, from, to, &Params::new()).or_else(||native_admission(ctx,d,from,to,&selected,st,q)) {
            rows.push(format!("{{\"id\":\"{}\",\"blocked\":\"{}\",\"readiness\":\"{}\"}}", d.id, esc(&issue.note), issue.status));
            continue;
        }
        let Some(from) = window_start(ctx, d, from).filter(|&f| f < to) else {
            rows.push(format!("{{\"id\":\"{}\",\"blocked\":\"Required volatility-index data does not cover this period\",\"readiness\":\"needs_data\"}}", d.id));
            continue;
        };
        let run = match backtest(ctx,d.id,from,to,cost,&Params::new()){Ok(run)=>run,Err(note)=>{rows.push(format!("{{\"id\":\"{}\",\"blocked\":\"{}\",\"readiness\":\"calculation_unavailable\"}}",d.id,esc(&note)));continue;}};
        // ~120-point equity sparkline for the strategy list
        let step = (run.equity.len() / 120).max(1);
        let spark: Vec<String> = run.equity.iter().step_by(step).map(|v| num(*v)).collect();
        rows.push(format!("{{\"id\":\"{}\",\"stats\":{},\"spark\":[{}],\"costs\":{}}}", d.id, stats_json(&financial_stats(ctx, &run, costs)), spark.join(","),if run.source_account.is_some(){costs.source_account_json()}else{costs.json()}));
        if d.book==books::chande::BOOK {let item=rows.last_mut().unwrap();item.pop();let _=write!(item,",\"experiment\":{}}}",experiment_json(d.id));}
        if selected.native(){let item=rows.last_mut().unwrap();item.pop();let _=write!(item,",\"provenance\":{}}}",source_provenance(st,q));}
    }
    Ok(format!("[{}]", rows.join(",")))
}

/// Typed diagnostic periods come from the method's effective schema defaults.
/// An unlabelled or differently typed moving average is not an SMA rule check.
struct DiagnosticSeries {
    rsi: Option<(usize, Vec<f64>)>,
    ma: Option<(usize, Vec<f64>)>,
    trend: Option<(usize, Vec<f64>)>,
}
fn diagnostic_series(ctx: &Ctx, id: &str, p: &Params) -> DiagnosticSeries {
    let schema = connors::params(id);
    let period = |key: &str, kind: &str| {
        let def = schema.iter().find(|item| item.key == key)?;
        if !def.label.to_ascii_lowercase().contains(kind) { return None; }
        let value = p.get(key).copied().unwrap_or(def.def);
        // Diagnostic bounds do not alter the strategy's own parameter checks or calculations.
        if !value.is_finite() || !(1.0..=10_000.0).contains(&value) { return None; }
        Some(value.round().max(1.0) as usize)
    };
    DiagnosticSeries {
        rsi: period("rsi_n", "rsi").map(|n| (n, ctx.rsi_n(n).into_owned())),
        ma: period("ma", "sma").map(|n| (n, ctx.sma_n(n).into_owned())),
        trend: period("trend", "sma").map(|n| (n, ctx.sma_n(n).into_owned())),
    }
}

/// Completed-bar context labelled by actual date, indicator kind and period.
/// This describes a bar; it does not certify the source strategy or execution price.
fn snapshot(ctx: &Ctx, i: usize, series: &DiagnosticSeries) -> String {
    if i >= ctx.r.len() { return "null".into(); }
    let pct = |a: f64, b: f64| num((a / b - 1.0) * 100.0);
    let period = |value: &Option<(usize, Vec<f64>)>| value.as_ref().map_or("null".into(), |(n, _)| n.to_string());
    let kind = |value: &Option<(usize, Vec<f64>)>, name: &str| if value.is_some() { format!("\"{name}\"") } else { "null".into() };
    let rsi = series.rsi.as_ref().map_or("null".into(), |(_, values)| num(values[i]));
    let distance = |value: &Option<(usize, Vec<f64>)>| value.as_ref().map_or("null".into(), |(_, values)| pct(ctx.c[i], values[i]));
    format!(
        "{{\"date\":\"{}\",\"close\":{},\"rsi2\":{},\"vs_sma200\":{},\"vs_sma5\":{},\"vix_rsi2\":{},\"rsi_p\":{},\"rsi_period\":{},\"rsi_kind\":{},\"vs_ma_p\":{},\"ma_period\":{},\"ma_kind\":{},\"vs_trend_p\":{},\"trend_period\":{},\"trend_kind\":{},\"basis\":\"Completed daily close indicator context; not a source-rule or execution-price verification\"}}",
        esc(&ctx.r[i].date), num(ctx.c[i]), num(ctx.rsi2[i]), pct(ctx.c[i], ctx.ma200[i]), pct(ctx.c[i], ctx.ma5[i]), num(ctx.vix.rsi2.get(i).copied().unwrap_or(f64::NAN)),
        rsi, period(&series.rsi), kind(&series.rsi, "Wilder"), distance(&series.ma), period(&series.ma), kind(&series.ma, "SMA"), distance(&series.trend), period(&series.trend), kind(&series.trend, "SMA")
    )
}

fn detail(st: &State, q: &BTreeMap<String, String>) -> Result<String, String> {
    let selected=select_context(st,q)?;let held=selected.context(st);
    let (ctx, from, to, cost) = resolve_context(&held, q)?;
    let costs=accounting::Costs::parse(q)?;
    let id = q.get("id").ok_or("id missing")?;
    let cat = query_catalogue(q);
    let def = cat.iter().find(|d| d.id == id.as_str()).ok_or("unknown strategy")?;
    let p = checked_params_of(q)?;
    if let Some(issue) = readiness::check(ctx, def, from, to, &p).or_else(||native_admission(ctx,def,from,to,&selected,st,q)) {
        return Err(format!("{}: {}", issue.status, issue.note));
    }
    costs.ensure_rollover()?;
    let from = window_start(ctx, def, from).filter(|&f| f < to).ok_or("Required volatility-index data does not cover this period")?;
    let run = backtest(ctx, id, from, to, cost, &p)?;
    let diagnostics = diagnostic_series(ctx, id, &p);
    let mut o = String::with_capacity(1 << 20);
    let arr = |v: &mut String, it: &mut dyn Iterator<Item = String>| {
        v.push('[');
        v.push_str(&it.collect::<Vec<_>>().join(","));
        v.push(']');
    };
    o.push_str("{\"dates\":");
    arr(&mut o, &mut (from..=to).map(|k| format!("\"{}\"", ctx.r[k].date)));
    o.push_str(",\"close\":");
    arr(&mut o, &mut (from..=to).map(|k| num(ctx.c[k])));
    o.push_str(",\"equity\":");
    arr(&mut o, &mut run.equity.iter().map(|v| num(*v)));
    o.push_str(",\"bh\":");
    arr(&mut o, &mut run.bh.iter().map(|v| num(*v)));
    o.push_str(",\"trades\":");
    let mut allocation=1.;
    arr(&mut o, &mut run.trades.iter().enumerate().map(|(i,t)| {
        if let Some(account)=&run.source_account {allocation=account.trade_audit[i].entry_equity;}
        let ledger=if run.source_account.is_some(){accounting::TradeLedger::source_account(ctx,t,allocation,costs)}else{accounting::TradeLedger::new(ctx,t,allocation,costs)};
        allocation*=1.+ledger.net_ret;
        let legs: Vec<String> = t.legs.iter().map(|l| format!("[{},{},{}]", l.i - from, num(l.px), num(l.w))).collect();
        format!(
            "{{\"dir\":{},\"legs\":[{}],\"entry\":\"{}\",\"exit\":\"{}\",\"exit_i\":{},\"exit_px\":{},\"avg_px\":{},\"ret\":{},\"ret_full_precision\":{},\"bars\":{},\"open\":{},\"reason\":\"{}\",\"entry_kind\":\"{}\",\"at_entry\":{},\"at_exit\":{},\"signal\":\"{}\",\"exits\":[{}],\"accounting\":{}}}",
            t.dir, legs.join(","), ctx.r[t.legs[0].i].date, ctx.r[t.exit_i].date, t.exit_i - from, num(t.exit_px),
            num(t.avg_px()), num(ledger.net_ret), full_precision_trade_return(ledger.net_ret), t.exit_i - t.legs[0].i, t.open, esc(&t.reason), t.entry_kind,
            snapshot(ctx, t.legs[0].i, &diagnostics), snapshot(ctx, t.signal_i, &diagnostics), ctx.r[t.signal_i].date,
            t.exits.iter().map(|(i, px, f)| format!("[{},{},{}]", i - from, num(*px), num(*f))).collect::<Vec<_>>().join(","),ledger.json()
        )
    }));
    if selected.native(){let _=write!(o,",\"provenance\":{}",source_provenance(st,q));}
    if selected.native() || def.book==books::chande::BOOK {
        o.push_str(",\"ohlc\":{");
        o.push_str("\"d\":");arr(&mut o,&mut (from..=to).map(|k|format!("\"{}\"",ctx.r[k].date)));
        for (name,field) in [("o",0),("h",1),("l",2),("c",3)]{let _=write!(o,",\"{name}\":");arr(&mut o,&mut (from..=to).map(|k|num([ctx.o[k],ctx.h[k],ctx.l[k],ctx.c[k]][field])));}
        o.push_str(",\"v\":");arr(&mut o,&mut (from..=to).map(|k|ctx.r[k].volume.map_or("null".into(),num)));o.push('}');
    }
    let gross_equity=run.source_account.as_ref().map_or_else(||equity_curve(&ctx.c,&run.trades,from,to,0.),|a|a.days.iter().map(|d|d.gross).collect());
    o.push_str(",\"gross_equity\":");arr(&mut o,&mut gross_equity.iter().copied().map(num));
    let gross=Run{trades:run.trades.clone(),from,to,equity:gross_equity,bh:run.bh.clone(),source_account:None};
    if let Some(account)=&run.source_account {
        let _=write!(o,",\"source_account\":{},\"model_initial_equity\":1,\"stats\":{},\"gross_stats\":{},\"benchmark_symbol\":\"{}\",\"benchmark_stats\":{},\"costs\":{},\"accounting_basis\":\"{}\"}}",account.json(ctx),stats_json(&financial_stats(ctx,&run,costs)),stats_json(&stats(ctx,&gross,0.)),esc(&ctx.r[from].ticker),stats_json(&benchmark_stats(ctx,&run)),costs.source_account_json(),esc(books::elcim_account::BASIS));
        return Ok(o);
    }
    let _ = write!(o, ",\"model_initial_equity\":1,\"stats\":{},\"gross_stats\":{},\"benchmark_symbol\":\"{}\",\"benchmark_stats\":{},\"costs\":{},\"accounting_basis\":\"Return-only report. The normalized compounding model begins at unit equity 1 before the first event. The first daily model return is equity[0]/1 - 1, including an initial fee or loss, and risk/drawdown reports retain that initial event. Net profit/closed_return_sum sums actual closed-trade net percentage returns and equals unrounded avg_net_pnl times closed trades. Open marks are excluded from that sum. total_return/compounded_return and charts use a separately labelled sequential unit-exposure compounding model including open liquidation estimates. No sized portfolio result is implied. Gross equity/stats independently compound the same fills at zero cost. Ledger gross/fee/slippage returns use the actual net pretrade allocation, so ledger_gross_return minus fee_return minus slippage_return reconciles net total_return. Open trades include modeled liquidation costs. No monetary sizing, statutory tax schedule or verified futures roll events are supplied.\"}}", stats_json(&financial_stats(ctx, &run, costs)),stats_json(&stats(ctx,&gross,0.)),esc(&ctx.r[from].ticker),stats_json(&benchmark_stats(ctx,&run)),costs.json());
    if def.book==books::chande::BOOK {o.pop();let _=write!(o,",\"experiment\":{}}}",experiment_json(id));}
    Ok(o)
}

/// Basket jobs use their own bounded, fail-fast permit. The shared single-stock
/// strategy engine and its calculation guards are unchanged.
fn basket_json(st:&State,q:&BTreeMap<String,String>,stream:&mut TcpStream){
    match basket_portfolio::run(st,q){
        Ok(body)=>respond(stream,"200 OK","application/json; charset=utf-8",body.as_bytes()),
        Err(note) if note.starts_with("busy:")=>respond(stream,"429 Too Many Requests","text/plain; charset=utf-8",note.as_bytes()),
        Err(note)=>respond(stream,"400 Bad Request","text/plain; charset=utf-8",note.as_bytes()),
    }
}

fn params_json(guru: &str) -> String {
    let items: Vec<String> = guru_catalogue(guru)
        .iter()
        .filter(|d| d.status == Status::Live)
        .map(|d| {
            let ps: Vec<String> = connors::params(d.id)
                .iter()
                .map(|p| format!("{{\"key\":\"{}\",\"label\":\"{}\",\"def\":{},\"min\":{},\"max\":{},\"step\":{}}}", p.key, esc(p.label), num(p.def), num(p.min), num(p.max), num(p.step)))
                .collect();
            format!("\"{}\":[{}]", d.id, ps.join(","))
        })
        .collect();
    format!("{{{}}}", items.join(","))
}

/// "min:max:step" (or a single value) -> grid values, rounded to 1e-9 so 0.1 steps land on clean numbers.
fn grid(spec: &str) -> Result<Vec<f64>, String> {
    let bad = || format!("bad range '{spec}', expected min:max:step");
    let v: Vec<f64> = spec.split(':').map(|x| x.trim().parse::<f64>()).collect::<Result<_, _>>().map_err(|_| bad())?;
    let (lo, hi, step) = match v[..] {
        [a] => (a, a, 1.0),
        [a, b, s] if s > 0.0 && b >= a => (a, b, s),
        _ => return Err(bad()),
    };
    let n = ((hi - lo) / step + 1e-9).floor() as usize + 1;
    if n > 1000 {
        return Err(format!("'{spec}' gives {n} values; use a bigger step"));
    }
    Ok((0..n).map(|k| ((lo + k as f64 * step) * 1e9).round() / 1e9).collect())
}

const MAX_COMBOS: usize = 50_000;
const FULL_COLS: &[&str] = &["total_return", "cagr", "sharpe", "sortino", "max_dd", "mar", "trades", "win_rate", "profit_factor", "avg_trade", "exposure", "t_stat", "avg_bars", "expectancy_ratio", "monthly_thickness", "avg_hold_days", "net_profit", "avg_net_pnl", "gross_total_return", "gross_avg_trade", "gross_win_rate", "gross_sharpe", "gross_max_dd", "ledger_gross_return", "fee_return", "slippage_return"];
const PART_COLS: &[&str] = &["total_return", "cagr", "sharpe", "max_dd", "trades", "win_rate", "t_stat"];

/// Every combination of the requested parameter ranges, run in parallel. Rows come in
/// mixed-radix order (last dimension changes fastest) so the UI can find grid neighbours.
/// With `split`, each combo is also run on [from, split) (in-sample) and [split, to] (out-of-sample).
fn optimize(st: &State, q: &BTreeMap<String, String>) -> Result<String, String> {
    let t0 = std::time::Instant::now();
    let selected=select_context(st,q)?;let held=selected.context(st);
    let (ctx, from, to, cost) = resolve_context(&held, q)?;
    let costs=accounting::Costs::parse(q)?;
    let id = q.get("id").ok_or("id missing")?.as_str();
    let cat = query_catalogue(q);
    let def = cat.iter().find(|d| d.id == id).ok_or("unknown strategy")?;
    let base = checked_params_of(q)?;
    if let Some(issue) = readiness::check(ctx, def, from, to, &base).or_else(||native_admission(ctx,def,from,to,&selected,st,q)) {
        return Err(format!("{}: {}", issue.status, issue.note));
    }
    costs.ensure_rollover()?;
    // A requested grid may select a different volatility series than the base setting.
    if let Some(spec) = q.get("g.src") {
        for value in grid(spec)? {
            let mut candidate=base.clone();candidate.insert("src".into(),value);
            if let Some(issue)=readiness::check(ctx,def,from,to,&candidate) {
                return Err(format!("{}: Grid src={value}: {}",issue.status,issue.note));
            }
        }
    }
    let from = window_start(ctx, def, from).filter(|&f| f < to).ok_or("Required volatility-index data does not cover this period")?;
    let mut dims: Vec<(&str, Vec<f64>)> = Vec::new();
    for p in connors::params(id) {
        if let Some(spec) = q.get(&format!("g.{}", p.key)) {
            dims.push((p.key, grid(spec)?));
        }
    }
    let combos: usize = dims.iter().map(|d| d.1.len()).product();
    if id==books::elcim_account::ID {
        for (key,values) in &dims {for &v in values {let mut p=base.clone();p.insert((*key).into(),v);if let Some(note)=books::elcim_account::validate(&p){return Err(note);}}}
        if q.keys().any(|k|k.starts_with("g.")&&!dims.iter().any(|(key,_)|k==&format!("g.{key}"))){return Err("Unknown protected Elder grid parameter".into());}
    }
    if def.book==books::chande::BOOK {
        for (key,values) in &dims {
            for &value in values {let mut candidate=base.clone();candidate.insert((*key).into(),value);
                if let Some(note)=books::chande::validate(id,&candidate) {return Err(note);}}
        }
        if q.keys().any(|k|k.starts_with("g.") && !dims.iter().any(|(name,_)|k==&format!("g.{name}"))) {
            return Err("Unknown Chande grid parameter".into());
        }
    }
    if combos > MAX_COMBOS {
        return Err(format!("{combos} combinations; the limit is {MAX_COMBOS}. Narrow a range or raise a step."));
    }
    let dates: Vec<&str> = ctx.r.iter().map(|r| r.date.as_str()).collect();
    let split = q
        .get("split")
        .filter(|s| !s.is_empty())
        .map(|s| dates.partition_point(|d| *d < s.as_str()))
        .filter(|&s| s > from + 20 && s + 20 < to);
    let metrics_v = |p: &Params| -> Result<Vec<f64>,String> {
        if let Some(issue)=readiness::check(ctx,def,from,to,p){return Err(issue.note);}
        let full = backtest(ctx, id, from, to, cost, p)?;
        let s = financial_stats(ctx, &full, costs);
        let mut out: Vec<f64> = FULL_COLS.iter().map(|k| s[k]).collect();
        for (a, b) in split.map(|s| [(from, s - 1), (s, to)]).into_iter().flatten() {
            let r = backtest(ctx, id, a, b, cost, p)?;
            let s = financial_stats(ctx, &r, costs);
            out.extend(PART_COLS.iter().map(|k| s[k]));
        }
        Ok(out)
    };
    let book_metrics=metrics_v(&Params::new())?;
    let metrics = book_metrics.iter().map(|v|num(*v)).collect::<Vec<_>>().join(",");
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get()).min(combos);
    // each row: parameter values, then FULL_COLS (and IS/OOS PART_COLS)
    let rows_v: Vec<Vec<f64>> = std::thread::scope(|sc| -> Result<Vec<Vec<f64>>,String> {
        let hs: Vec<_> = (0..threads)
            .map(|t| {
                let (dims, base, metrics_v) = (&dims, &base, &metrics_v);
                sc.spawn(move || {
                    (t..combos)
                        .step_by(threads)
                        .map(|k| {
                            let (mut p, mut r, mut vals) = (base.clone(), k, vec![0.0; dims.len()]);
                            for (j, (key, vs)) in dims.iter().enumerate().rev() {
                                let v = vs[r % vs.len()];
                                r /= vs.len();
                                p.insert(key.to_string(), v);
                                vals[j] = v;
                            }
                            vals.extend(metrics_v(&p)?);
                            Ok((k, vals))
                        })
                        .collect::<Result<Vec<_>,String>>()
                })
            })
            .collect();
        let mut all: Vec<(usize, Vec<f64>)> = Vec::new();
        for h in hs {all.extend(h.join().map_err(|_|"Optimization worker failed safely; no results published".to_string())??);}
        all.sort_by_key(|r| r.0);
        Ok(all.into_iter().map(|r| r.1).collect())
    })?;
    let col = |name: &str| dims.len() + FULL_COLS.iter().position(|c| *c == name).unwrap();
    // a setting must trade regularly: at least half the book setting's yearly rate, clamped to 1..5 trades a year, and 10 in total
    let yrs = years(ctx, from, to);
    let book_tpy = book_metrics[FULL_COLS.iter().position(|c| *c == "trades").unwrap()] / yrs;
    let per_year = (book_tpy * 0.5).clamp(MIN_PER_YEAR.0, MIN_PER_YEAR.1);
    let min_trades = (per_year * yrs).ceil().max(MIN_TRADES);
    // settings with identical results are one candidate, represented by the one with the fewest changes from the book
    let defs: Vec<f64> = dims.iter().map(|(k, _)| connors::params(id).iter().find(|p| p.key == *k).map_or(f64::NAN, |p| p.def)).collect();
    // Additive accounting columns must not change the historical outcome-
    // deduplication population, percentiles or picks at unchanged side costs.
    let outcome_cols:Vec<usize>=(dims.len()..dims.len()+16).chain(dims.len()+FULL_COLS.len()..rows_v.first().map_or(0,Vec::len)).collect();
    let (rank_json, pool) = conclude_rank_with_outcomes(&rows_v, col("trades"), &RANK_STATS.map(|(k, _)| col(k)), min_trades, per_year, &defs,Some(&outcome_cols));
    // slim=1: only the ranked candidates' rows (as {"index": row}), so the conclusion view does not download every combo
    let slim = q.get("slim").is_some_and(|v| v == "1");
    let row_s = |r: &Vec<f64>| format!("[{}]", r.iter().map(|v| num(*v)).collect::<Vec<_>>().join(","));
    let rows: Vec<String> = if slim { vec![] } else { rows_v.iter().map(row_s).collect() };
    let picked: Vec<String> = if slim { pool.iter().map(|&i| format!("\"{i}\":{}", row_s(&rows_v[i]))).collect() } else { vec![] };
    let mut cols: Vec<String> = dims.iter().map(|d| format!("\"p.{}\"", d.0)).collect();
    cols.extend(FULL_COLS.iter().map(|c| format!("\"{c}\"")));
    for part in split.map(|_| ["is_", "oos_"]).into_iter().flatten() {
        cols.extend(PART_COLS.iter().map(|c| format!("\"{part}{c}\"")));
    }
    let dims_json: Vec<String> = dims.iter().map(|(k, v)| format!("{{\"key\":\"{k}\",\"values\":[{}]}}", v.iter().map(|x| num(*x)).collect::<Vec<_>>().join(","))).collect();
    let fixed: Vec<String> = base.iter().map(|(k, v)| format!("\"{}\":{}", esc(k), num(*v))).collect();
    Ok(format!(
        "{{\"id\":\"{id}\",\"from\":\"{}\",\"to\":\"{}\",\"split\":{},\"combos\":{combos},\"threads\":{threads},\"ms\":{},\"dims\":[{}],\"fixed\":{{{}}},\"cols\":[{}],\"book\":[{}],\"rank\":{rank_json},\"picked\":{{{}}},\"rows\":[{}],\"costs\":{},\"provenance\":{},\"experiment\":{}}}",
        dates[from], dates[to], split.map_or("null".into(), |s| format!("\"{}\"", dates[s])), t0.elapsed().as_millis(),
        dims_json.join(","), fixed.join(","), cols.join(","), metrics, picked.join(","), rows.join(","),if id==books::elcim_account::ID{costs.source_account_json()}else{costs.json()},source_provenance(st,q),experiment_json(id)
    ))
}

/// The five stats the conclusion ranks on (column name, label).
const RANK_STATS: [(&str, &str); 5] = [("cagr", "CAGR"), ("sharpe", "Sharpe"), ("monthly_thickness", "Monthly thickness"), ("expectancy_ratio", "Expectancy ratio"), ("mar", "CAR/MDD")];
const MIN_TRADES: f64 = 10.0; // below this, Sharpe / expectancy of a combo are noise
const MIN_PER_YEAR: (f64, f64) = (1.0, 5.0); // required trades a year = half the book's rate, kept within these bounds
const GROUP_N: usize = 5;

/// Claude's conclusion ranking over every optimizer combination:
/// 1. eligible = one row per distinct outcome (identical stats), kept as the setting with the fewest changes from the book, with enough closed trades (min_trades: per_year a year over the window, at least MIN_TRADES) and all five stats finite;
/// 2. percentile of each stat across the eligible combos (rank-based, ties averaged, 0-100);
/// 3. five groups: the top GROUP_N combos by each stat;
/// 4. cross-comparison: every candidate in the pooled groups is compared on all five stats with each group's leader
///    (its stat as a share of the leader's value, and its percentile);
/// 5. score = mean of the five percentiles (a candidate must be good on all five, not one); ties -> higher weakest percentile, then CAGR.
/// The top 3 by score are the picks. Returns JSON with everything needed to show / audit each step.
fn conclude_rank(rows: &[Vec<f64>], trades_col: usize, stat_cols: &[usize; 5], min_trades: f64, per_year: f64, defs: &[f64]) -> (String, Vec<usize>) {
    conclude_rank_with_outcomes(rows,trades_col,stat_cols,min_trades,per_year,defs,None)
}
fn conclude_rank_with_outcomes(rows: &[Vec<f64>], trades_col: usize, stat_cols: &[usize; 5], min_trades: f64, per_year: f64, defs: &[f64],outcome_cols:Option<&[usize]>) -> (String, Vec<usize>) {
    let nd = defs.len();
    let changes = |i: usize| (0..nd).filter(|&j| (rows[i][j] - defs[j]).abs() > 1e-9).count();
    let mut by_outcome: BTreeMap<Vec<u64>, usize> = BTreeMap::new(); // identical stats = identical trades
    for i in (0..rows.len()).filter(|&i| rows[i][trades_col] >= min_trades && stat_cols.iter().all(|&c| rows[i][c].is_finite())) {
        let key: Vec<u64> = if let Some(cols)=outcome_cols {cols.iter().map(|&j|(rows[i][j] as f32).to_bits() as u64).collect()}else{rows[i][nd..].iter().map(|v| (*v as f32).to_bits() as u64).collect()}; // historical summary identity, not proof of identical trades
        let e = by_outcome.entry(key).or_insert(i);
        if changes(i) < changes(*e) { *e = i; }
    }
    let mut elig: Vec<usize> = by_outcome.into_values().collect();
    elig.sort();
    if elig.is_empty() {
        return (format!("{{\"eligible\":0,\"distinct\":0,\"total\":{},\"min_trades\":{},\"per_year\":{},\"groups\":[],\"pool\":[],\"top3\":[]}}", rows.len(), num(min_trades), num(per_year)), vec![]);
    }
    let m = elig.len();
    // percentile per stat: position among eligible sorted ascending, ties share the average rank, scaled to 0..100
    let pct: Vec<Vec<f64>> = stat_cols.iter().map(|&c| {
        let mut ord: Vec<usize> = (0..m).collect();
        ord.sort_by(|&a, &b| rows[elig[a]][c].partial_cmp(&rows[elig[b]][c]).unwrap());
        let mut p = vec![0.0; m];
        let mut k = 0;
        while k < m {
            let mut j = k;
            while j + 1 < m && rows[elig[ord[j + 1]]][c] == rows[elig[ord[k]]][c] { j += 1; }
            let r = (k + j) as f64 / 2.0;
            for t in k..=j { p[ord[t]] = if m == 1 { 100.0 } else { r / (m - 1) as f64 * 100.0 }; }
            k = j + 1;
        }
        p
    }).collect();
    let groups: Vec<Vec<usize>> = stat_cols.iter().map(|&c| {
        let mut ord: Vec<usize> = (0..m).collect();
        ord.sort_by(|&a, &b| rows[elig[b]][c].partial_cmp(&rows[elig[a]][c]).unwrap().then(a.cmp(&b)));
        ord.truncate(GROUP_N);
        ord
    }).collect();
    let mut pool: Vec<usize> = groups.iter().flatten().copied().collect();
    pool.sort();
    pool.dedup();
    let score = |e: usize| (0..5).map(|s| pct[s][e]).sum::<f64>() / 5.0;
    let weakest = |e: usize| (0..5).map(|s| pct[s][e]).fold(f64::MAX, f64::min);
    let mut ranked = pool.clone();
    ranked.sort_by(|&a, &b| score(b).partial_cmp(&score(a)).unwrap()
        .then(weakest(b).partial_cmp(&weakest(a)).unwrap())
        .then(rows[elig[b]][stat_cols[0]].partial_cmp(&rows[elig[a]][stat_cols[0]]).unwrap()));
    let cand = |e: usize| {
        let r = &rows[elig[e]];
        let vs_leader: Vec<String> = (0..5).map(|s| { let l = rows[elig[groups[s][0]]][stat_cols[s]]; num(if l != 0.0 { r[stat_cols[s]] / l } else { f64::NAN }) }).collect();
        let in_groups: Vec<String> = (0..5).filter(|&s| groups[s].contains(&e)).map(|s| format!("\"{}\"", RANK_STATS[s].0)).collect();
        format!("{{\"row\":{},\"score\":{},\"weakest\":{},\"pct\":[{}],\"vs_leader\":[{}],\"groups\":[{}]}}",
            elig[e], num(score(e)), num(weakest(e)), (0..5).map(|s| num(pct[s][e])).collect::<Vec<_>>().join(","), vs_leader.join(","), in_groups.join(","))
    };
    let groups_json: Vec<String> = groups.iter().enumerate().map(|(s, g)| format!("{{\"stat\":\"{}\",\"label\":\"{}\",\"rows\":[{}]}}", RANK_STATS[s].0, RANK_STATS[s].1, g.iter().map(|&e| elig[e].to_string()).collect::<Vec<_>>().join(","))).collect();
    (format!("{{\"eligible\":{m},\"distinct\":{m},\"total\":{},\"min_trades\":{},\"per_year\":{},\"stats\":[{}],\"groups\":[{}],\"pool\":[{}],\"top3\":[{}]}}",
        rows.len(), num(min_trades), num(per_year), RANK_STATS.iter().map(|(k, _)| format!("\"{k}\"")).collect::<Vec<_>>().join(","), groups_json.join(","),
        ranked.iter().map(|&e| cand(e)).collect::<Vec<_>>().join(","), ranked.iter().take(3).map(|&e| cand(e)).collect::<Vec<_>>().join(",")), ranked.iter().map(|&e| elig[e]).collect())
}

/// YT's own written conclusions, one text file per key (e.g. "rsi2_4" or "rsi2_4.us").
fn note_path(q: &BTreeMap<String, String>) -> Result<String, String> {
    let k = q.get("key").ok_or("key missing")?;
    if k.is_empty() || k.len() > 80 || !k.chars().all(|c| c.is_ascii_alphanumeric() || "_-.".contains(c)) || k.starts_with('.') {
        return Err("bad key".into());
    }
    let dir = std::env::var("VAULT_NOTES").unwrap_or_else(|_| concat!(env!("CARGO_MANIFEST_DIR"), "/../notes").into());
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(format!("{dir}/{k}.txt"))
}

fn respond(stream: &mut TcpStream, status: &str, ctype: &str, body: &[u8]) {
    let head = format!("HTTP/1.1 {status}\r\nContent-Type: {ctype}\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\nConnection: close\r\n\r\n", body.len());
    let _ = stream.write_all(head.as_bytes());
    let _ = stream.write_all(body);
}

fn handle(mut stream: TcpStream, st: Arc<State>) {
    // Read the head, then the body up to Content-Length (POST /api/notes only; 1 MB cap).
    let (mut buf, mut chunk) = (Vec::new(), [0u8; 8192]);
    let head_end = loop {
        let n = stream.read(&mut chunk).unwrap_or(0);
        buf.extend_from_slice(&chunk[..n]);
        if let Some(h) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            break h;
        }
        if n == 0 || buf.len() > 1 << 16 {
            return;
        }
    };
    let head = String::from_utf8_lossy(&buf[..head_end]).into_owned();
    let len: usize = head
        .lines()
        .find_map(|l| l.split_once(':').filter(|(k, _)| k.eq_ignore_ascii_case("content-length")).and_then(|(_, v)| v.trim().parse().ok()))
        .unwrap_or(0);
    if len > 1 << 20 {
        return respond(&mut stream, "413 Payload Too Large", "text/plain", b"note too large");
    }
    while buf.len() < head_end + 4 + len {
        match stream.read(&mut chunk) {
            Ok(n) if n > 0 => buf.extend_from_slice(&chunk[..n]),
            _ => break,
        }
    }
    let body = &buf[(head_end + 4).min(buf.len())..];
    let mut parts = head.split_whitespace();
    let (method, target) = (parts.next().unwrap_or(""), parts.next().unwrap_or("/"));
    let (path, q) = query(target);
    if method == "POST" && path == "/api/notes" {
        let r = note_path(&q).and_then(|f| {
            let tmp = format!("{f}.tmp");
            std::fs::write(&tmp, body).and_then(|_| std::fs::rename(&tmp, &f)).map_err(|e| e.to_string())
        });
        return match r {
            Ok(()) => respond(&mut stream, "200 OK", "text/plain", b"saved"),
            Err(e) => respond(&mut stream, "400 Bad Request", "text/plain", e.as_bytes()),
        };
    }
    if method != "GET" {
        return respond(&mut stream, "405 Method Not Allowed", "text/plain", b"GET only");
    }
    let json = |r: Result<String, String>, s: &mut TcpStream| match r {
        Ok(b) => respond(s, "200 OK", "application/json; charset=utf-8", b.as_bytes()),
        Err(e) => respond(s, "400 Bad Request", "text/plain; charset=utf-8", e.as_bytes()),
    };
    match path.as_str() {
        "/api/ohlc" => json(chart_ohlc_json(&st,&q), &mut stream),
        "/api/tickers" => {
            let mut items: Vec<String> = st
                .ctxs
                .iter()
                .map(|(t, c)| format!("{{\"ticker\":\"{}\",\"first\":\"{}\",\"last\":\"{}\",\"has_volume\":{},\"series\":[{}]}}", esc(t), c.r[0].date, c.r[c.r.len() - 1].date, c.has_volume, c.extra.keys().filter(|k| c.series(k).is_some()).map(|k| format!("\"{}\"", esc(k))).collect::<Vec<_>>().join(",")))
                .collect();
            items.extend(st.baskets.tickers_json());
            json(Ok(format!("[{}]", items.join(","))), &mut stream)
        }
        "/api/gurus" => {
            let cat = guru_catalogue("connors");
            let ross = guru_catalogue("ross");
            let live = cat.iter().filter(|d| d.status == Status::Live).count();
            let thesis: Vec<String> = connors::THESIS.iter().map(|(h, b)| format!("{{\"title\":\"{}\",\"body\":\"{}\"}}", esc(h), esc(b))).collect();
            json(Ok(format!(
                "[{{\"id\":\"{}\",\"name\":\"{}\",\"tagline\":\"{}\",\"book\":\"{}\",\"strategies\":{},\"testable\":{},\"thesis\":[{}]}},{{\"id\":\"ross\",\"name\":\"Joe Ross\",\"tagline\":\"Price-action methods, source evidence and discretionary trade management. Complete backtests remain unavailable.\",\"book\":\"Reviewed core sources; full corpus review in progress\",\"strategies\":{},\"testable\":0,\"thesis\":[]}},{{\"id\":\"chande\",\"name\":\"Tushar Chande\",\"tagline\":\"Trend reversal and breakout-pullback rules with independently checked OHLC execution. Original futures studies are not reproduced.\",\"book\":\"Beyond Technical Analysis (1997, supplied partial edition)\",\"strategies\":5,\"testable\":2,\"thesis\":[]}}]",
                connors::GURU_ID, esc(connors::GURU_NAME), esc(connors::TAGLINE), esc(connors::BOOK), cat.len(), live, thesis.join(","), ross.len()
            )), &mut stream)
        }
        "/api/guru/connors/strategies" => json(Ok(catalogue_json("connors")), &mut stream),
        "/api/guru/connors/readiness" => json(readiness_json(&st, &owned_query(&q,"connors")), &mut stream),
        "/api/guru/connors/summary" => json(summary(&st, &owned_query(&q,"connors")), &mut stream),
        "/api/guru/connors/run" => json(detail(&st, &owned_query(&q,"connors")), &mut stream),
        "/api/guru/connors/basket-run" => basket_json(&st,&owned_query(&q,"connors"),&mut stream),
        "/api/guru/connors/params" => json(Ok(params_json("connors")), &mut stream),
        "/api/guru/connors/optimize" => json(optimize(&st, &owned_query(&q,"connors")), &mut stream),
        "/api/guru/chande/strategies" => json(Ok(catalogue_json("chande")), &mut stream),
        "/api/guru/chande/params" => json(Ok(params_json("chande")), &mut stream),
        "/api/guru/chande/readiness" => json(readiness_json(&st, &owned_query(&q,"chande")), &mut stream),
        "/api/guru/chande/summary" => json(summary(&st, &owned_query(&q,"chande")), &mut stream),
        "/api/guru/chande/run" => json(detail(&st, &owned_query(&q,"chande")), &mut stream),
        "/api/guru/chande/basket-run" => basket_json(&st,&owned_query(&q,"chande"),&mut stream),
        "/api/guru/chande/optimize" => json(optimize(&st, &owned_query(&q,"chande")), &mut stream),
        "/api/guru/ross/strategies" => json(Ok(catalogue_json("ross")), &mut stream),
        "/api/guru/ross/readiness" => json(readiness_json(&st, &owned_query(&q,"ross")), &mut stream),
        "/api/guru/ross/summary" => json(summary(&st, &owned_query(&q,"ross")), &mut stream),
        "/api/guru/ross/run" => json(detail(&st, &owned_query(&q,"ross")), &mut stream),
        "/api/guru/ross/basket-run" => basket_json(&st,&owned_query(&q,"ross"),&mut stream),
        "/api/guru/ross/params" => json(Ok(params_json("ross")), &mut stream),
        "/api/guru/ross/optimize" => json(optimize(&st, &owned_query(&q,"ross")), &mut stream),
        "/api/notes" => match note_path(&q) {
            Ok(f) => respond(&mut stream, "200 OK", "text/plain; charset=utf-8", &std::fs::read(f).unwrap_or_default()),
            Err(e) => respond(&mut stream, "400 Bad Request", "text/plain", e.as_bytes()),
        },
        _ => respond(&mut stream, "404 Not Found", "text/plain", b"not found"),
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let csv = args.get(1).cloned().unwrap_or_else(|| concat!(env!("CARGO_MANIFEST_DIR"), "/../../../idx_data.csv").into());
    let port = args.get(2).cloned().unwrap_or_else(|| "8090".into());
    let vix_dir = args.get(3).cloned().unwrap_or_else(|| concat!(env!("CARGO_MANIFEST_DIR"), "/../../../india_vix").into());
    let t0 = std::time::Instant::now();
    let vix = load_vix_dir(&vix_dir);
    match (vix.first(), vix.last()) {
        (Some(a), Some(b)) => println!("Volatility source {}: {} daily bars {} → {} ({:.1?}); market admission checks preserve the actual source identity",a.ticker, vix.len(), a.date, b.date, t0.elapsed()),
        _ => eprintln!("warning: no consistent volatility-index data in {vix_dir}; VIX strategies will be blocked"),
    }
    let data = load_csv(&csv).unwrap_or_else(|e| panic!("load data: {e}"));
    // ponytail: rows leaked for a 'static borrow — data is loaded once for the server's lifetime.
    let data: &'static BTreeMap<String, Vec<Row>> = Box::leak(Box::new(data));
    let extra_dir = args.get(4).cloned().unwrap_or_else(|| concat!(env!("CARGO_MANIFEST_DIR"), "/../../../extra_data").into());
    let extra = load_extra(&extra_dir);
    println!("extra data: {} series from {extra_dir}", extra.len());
    let ctxs = data
        .iter()
        .map(|(t, rows)| {
            let market=preloaded_market(t);
            let bench=market.and_then(|m|data.get(native_benchmark(m))).map(Vec::as_slice).unwrap_or(&[]);
            let (source_vix,volatility_source)=market_vix(&vix,market);
            let mut c = Ctx::new(rows, bench, source_vix);
            // the VIX's own daily OHLC as built-in series (VIX.Open/High/Low/Close), aligned by date
            let vx: BTreeMap<&str, &Row> = source_vix.iter().map(|v| (v.date.as_str(), v)).collect();
            for (k, f) in [("Open", 0), ("High", 1), ("Low", 2), ("Close", 3)] {
                let pick = |v: &Row| [v.open, v.high, v.low, v.close][f];
                let col: Vec<f64> = rows.iter().map(|r| vx.get(r.date.as_str()).map_or(f64::NAN, |v| pick(v))).collect();
                c.extra.insert(format!("VIX_OHLC.{k}"), col.clone()); // name used by the book modules
                c.extra.insert(format!("VIX.{k}"), col);
            }
            for ((tk, key), m) in &extra {
                if is_vix_series(key)&&volatility_source.is_none(){continue;}
                if tk == "*" || tk == t {
                    c.extra.insert(key.clone(), rows.iter().map(|r| m.get(&r.date).copied().unwrap_or(f64::NAN)).collect());
                }
            }
            (t.clone(), c)
        })
        .collect();
    let baskets=std::env::var("VAULT_BASKET_DATA_DIR").ok().map_or_else(baskets::Catalogue::default,|directory|baskets::Catalogue::load(std::path::Path::new(&directory)).unwrap_or_else(|error|{eprintln!("warning: basket admission unavailable: {error}");baskets::Catalogue::default()}));
    let st = Arc::new(State { ctxs,baskets,vix,market_extra:extra });
    let listener = TcpListener::bind(format!("0.0.0.0:{port}")).expect("bind");
    println!("Guru API on http://localhost:{port}/api/gurus  ({} tickers from {csv})", data.len());
    for s in listener.incoming().flatten() {
        let st = st.clone();
        std::thread::spawn(move || handle(s, st));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reviewed_four_burns_queries_and_grids_fail_closed_without_changing_legacy() {
        for id in books::sbma::FOUR_REVIEWED_STOCK_IDS {
            let defaults=BTreeMap::from([("id".into(),(*id).into())]);
            assert!(checked_params_of(&defaults).is_ok());
            for (key,value) in [("p.fast","garbage"),("p.fast",""),("p.fast","NaN"),("p.slow","inf"),("p.always_in","1"),("p.fill","0.5"),("p.unknown","garbage"),("p.unknown","0"),("g.fast","1:2:1"),("g.always_in","0:1:1"),("g.fill","0:1:0.5"),("g.fill","NaN"),("g.fill","0:inf:1"),("g.fill","0:1:1e-300"),("g.fill","-1e308:1e308:1"),("g.fill","0:1:inf"),("g.fill","0.00000000001"),("g.unknown","0")] {
                let mut bad=defaults.clone();bad.insert(key.into(),value.into());
                assert!(checked_params_of(&bad).is_err(),"{id}: {key}={value}");
            }
            let mut fills=defaults.clone();fills.insert("g.fill".into(),"0:1:1".into());assert!(checked_params_of(&fills).is_ok());
            for setting in books::sbma::params(id).unwrap() {
                let mut exact=defaults.clone();exact.insert(format!("p.{}",setting.key),setting.def.to_string());assert!(checked_params_of(&exact).is_ok());
                if setting.key!="fill" {exact.insert(format!("g.{}",setting.key),setting.def.to_string());assert!(checked_params_of(&exact).is_ok());}
            }
        }
        let old=BTreeMap::from([("id".into(),"sbma_x_5e50s".into()),("p.fast".into(),"garbage".into())]);
        assert!(checked_params_of(&old).unwrap().is_empty());
    }
    #[test]
    fn protected_elder_raw_grid_validation_precedes_legacy_grid_rounding_and_counts() {
        let base=BTreeMap::from([("id".into(),books::elcim_account::ID.into())]);
        for (key,value) in [("g.ema","0:inf:1"),("g.ema","0:1:1e-300"),("g.ema","13.00000000001"),("g.ema","13:13.00000000001:0.00000000001"),("g.ema","NaN"),("g.ema","-inf"),("g.ema","-1e308:1e308:1"),("g.ema","0:1:inf"),("g.ema","0:1:0"),("g.ema","13:13:-1"),("g.ema","bad"),("g.unknown","0"),("g.unknown","bad"),("p.ema","13.00000000001"),("p.ema","bad"),("p.unknown","0")] {
            let mut q=base.clone();q.insert(key.into(),value.into());assert!(checked_params_of(&q).is_err(),"{key}={value}");
        }
        assert!(checked_params_of(&base).is_ok());
        for setting in books::elcim::params(books::elcim_account::ID).unwrap() {
            for (key,value) in [(format!("p.{}",setting.key),setting.def.to_string()),(format!("g.{}",setting.key),setting.def.to_string()),(format!("g.{}",setting.key),format!("{}:{}:1",setting.def,setting.def))] {
                let mut q=base.clone();q.insert(key,value);assert!(checked_params_of(&q).is_ok());
            }
        }
        let legacy=BTreeMap::from([("id".into(),"f1a".into()),("p.ema".into(),"bad".into()),("g.ema".into(),"0:inf:1".into())]);
        assert!(checked_params_of(&legacy).unwrap().is_empty());
    }
    #[test]
    fn reviewed_ehlers_queries_reject_malformed_alternate_and_unknown_settings() {
        for (id,cp,mbb) in [("ehrs_sinetrend_trend","1","50"),("ehrs_sinetrend_cp115","1.15","57"),("ehrs_sinetrend_cp115_trend","1.15","57")] {
            let defaults=BTreeMap::from([("id".into(),id.into()),("p.cyc_part".into(),cp.into()),("p.max_bars_back".into(),mbb.into())]);
            assert!(checked_params_of(&defaults).is_ok());
            for (key,value) in [("p.cyc_part","garbage"),("p.cyc_part",""),("p.cyc_part","NaN"),("p.cyc_part","inf"),("p.cyc_part","1.10"),("p.max_bars_back","60"),("p.unknown","garbage"),("g.cyc_part","1.10"),("g.max_bars_back","60"),("g.unknown","1")] {
                let mut bad=defaults.clone();bad.insert(key.into(),value.into());
                assert!(checked_params_of(&bad).is_err(),"{id}: {key}={value}");
            }
            let mut exact_grid=defaults.clone();exact_grid.insert("g.cyc_part".into(),cp.into());assert!(checked_params_of(&exact_grid).is_ok());
        }
        let legacy=BTreeMap::from([("id".into(),"ehrs_sinetrend_cp110".into()),("p.cyc_part".into(),"garbage".into())]);
        assert!(checked_params_of(&legacy).unwrap().is_empty());
    }

    #[test]
    fn reviewed_ehlers_grid_overflow_inputs_return_errors_without_panicking() {
        for id in ["ehrs_sinetrend_trend","ehrs_sinetrend_cp115","ehrs_sinetrend_cp115_trend"] {
            for spec in ["0:inf:1","0:1:1e-300","-1e308:1e308:1","NaN","1:1:inf","1:1:1e-400","1:1001:1"] {
                let query=BTreeMap::from([("id".into(),id.into()),("g.cyc_part".into(),spec.into())]);
                assert!(checked_params_of(&query).is_err(),"{id}: {spec}");
            }
            let cp=if id=="ehrs_sinetrend_trend" {"1"} else {"1.15"};
            for spec in [cp.to_string(),format!("{cp}:{cp}:0.05"),format!("{cp}:{cp}:1e-300")] {
                let query=BTreeMap::from([("id".into(),id.into()),("g.cyc_part".into(),spec)]);
                assert!(checked_params_of(&query).is_ok());
            }
        }
    }

    #[test]
    fn reviewed_ehlers_grid_rejects_raw_values_that_only_round_to_defaults() {
        for id in ["ehrs_sinetrend_trend","ehrs_sinetrend_cp115","ehrs_sinetrend_cp115_trend"] {
            let cp=if id=="ehrs_sinetrend_trend" {"1.00000000001"} else {"1.15000000001"};
            let mbb=if id=="ehrs_sinetrend_trend" {"50.00000000001"} else {"57.00000000001"};
            for (key,value) in [("g.cyc_part",cp.to_string()),("g.cyc_part",format!("{cp}:{cp}:0.05")),("g.max_bars_back",mbb.to_string())] {
                let query=BTreeMap::from([("id".into(),id.into()),(key.into(),value)]);
                assert!(checked_params_of(&query).is_err(),"{id}: {key}");
            }
        }
    }

    #[test]
    fn reviewed_index_methods_keep_distinct_open_and_close_data_windows() {
        let r:Vec<Row>=["2013-12-30","2013-12-31","2014-01-02","2014-01-03"].into_iter().map(|date|Row {
            ticker:"SPX".into(),date:date.into(),open:100.,high:101.,low:99.,close:100.,volume:None,
        }).collect();
        let ctx=Ctx::new(&r,&r,&[]);
        let cat=connors::catalogue();
        let radge=cat.iter().find(|d|d.id=="radge_oc_long").unwrap();
        assert_eq!(window_start(&ctx,radge,0),Some(2));
        for id in ["bllstr_vma_1_50_0","bllstr_fma_1_50_0"] {
            let d=cat.iter().find(|d|d.id==id).unwrap();
            assert_eq!(window_start(&ctx,d,0),Some(0));
            assert!(instrument_policy::reviewed_cash_rule(d));
        }
        let metadata=catalogue_json("connors");
        for id in ["radge_oc_long","bllstr_vma_1_50_0","bllstr_fma_1_50_0"] {assert!(metadata.contains(id));}
        for id in ["radge_oc_long_stop","radge_oc_short_stop"] {
            let d=cat.iter().find(|d|d.id==id).unwrap();
            assert!(!instrument_policy::reviewed_cash_rule(d));
            assert!(readiness::check(&ctx,d,0,3,&Params::new()).is_some());
        }
    }

    #[test]
    fn diagnostic_snapshot_burns_schema_default_is_independent_of_fill_query_presence() {
        let (_fixture,state,q)=native_us_fixture();let selected=select_context(&state,&q).unwrap();let ctx=selected.context(&state);
        let base=diagnostic_series(&ctx,"sbma_sma50_long",&Params::new());
        let fill=diagnostic_series(&ctx,"sbma_sma50_long",&Params::from([("fill".into(),1.)]));
        let explicit=diagnostic_series(&ctx,"sbma_sma50_long",&Params::from([("ma".into(),50.),("fill".into(),1.)]));
        assert_eq!(base.ma.as_ref().unwrap().0,50);assert!(base.rsi.is_none()&&base.trend.is_none());
        assert_eq!(snapshot(&ctx,210,&base),snapshot(&ctx,210,&fill));assert_eq!(snapshot(&ctx,210,&base),snapshot(&ctx,210,&explicit));
        let expected=ctx.c[161..=210].iter().sum::<f64>()/50.;assert!((base.ma.as_ref().unwrap().1[210]-expected).abs()<1e-10);
        let json=snapshot(&ctx,210,&base);assert!(json.contains("\"ma_period\":50,")&&json.contains("\"ma_kind\":\"SMA\"")&&json.contains("\"date\":"));
    }
    #[test]
    fn diagnostic_snapshot_held_short_has_200_period_without_opening_admission() {
        let (_fixture,state,mut q)=native_us_fixture();let selected=select_context(&state,&q).unwrap();let ctx=selected.context(&state);
        let base=diagnostic_series(&ctx,"sbma_sma200_short",&Params::new());
        let explicit=diagnostic_series(&ctx,"sbma_sma200_short",&Params::from([("ma".into(),200.)]));
        assert_eq!(base.ma.as_ref().unwrap().0,200);assert_eq!(snapshot(&ctx,210,&base),snapshot(&ctx,210,&explicit));
        let expected=ctx.c[11..=210].iter().sum::<f64>()/200.;assert!((base.ma.as_ref().unwrap().1[210]-expected).abs()<1e-10);
        q.insert("id".into(),"sbma_sma200_short".into());assert!(detail(&state,&q).is_err());
    }
    #[test]
    fn diagnostic_snapshot_labels_rsi_and_sma_overrides_and_excludes_unknown_keys() {
        let (_fixture,state,q)=native_us_fixture();let selected=select_context(&state,&q).unwrap();let ctx=selected.context(&state);
        let custom=diagnostic_series(&ctx,"rsi2_5d",&Params::from([("rsi_n".into(),4.),("ma".into(),10.),("trend".into(),100.)]));
        assert_eq!(custom.rsi.as_ref().unwrap().0,4);assert_eq!(custom.ma.as_ref().unwrap().0,10);assert_eq!(custom.trend.as_ref().unwrap().0,100);
        let json=snapshot(&ctx,210,&custom);assert!(json.contains("\"rsi_period\":4,")&&json.contains("\"rsi_kind\":\"Wilder\""));
        let unrelated=diagnostic_series(&ctx,"sbma_x_5s20s",&Params::from([("ma".into(),50.),("rsi_n".into(),14.)]));
        assert!(unrelated.ma.is_none()&&unrelated.rsi.is_none()&&unrelated.trend.is_none());
    }
    #[test]
    fn diagnostic_snapshot_invalid_periods_and_unavailable_dates_fail_closed() {
        let (_fixture,state,q)=native_us_fixture();let selected=select_context(&state,&q).unwrap();let ctx=selected.context(&state);
        for ma in [f64::NAN,f64::INFINITY,0.,-1.,10001.] {assert!(diagnostic_series(&ctx,"sbma_sma50_long",&Params::from([("ma".into(),ma)])).ma.is_none());}
        let series=diagnostic_series(&ctx,"sbma_sma50_long",&Params::new());
        assert_eq!(snapshot(&ctx,ctx.r.len(),&series),"null");assert!(snapshot(&ctx,0,&series).contains("\"vs_ma_p\":null,"));
    }


    #[test]
    fn protected_elder_raw_parameters_never_fall_back_from_invalid_values(){
        for raw in ["","bad","NaN","inf","-inf","13.5","14"] {
            let q=BTreeMap::from([("id".into(),books::elcim_account::ID.into()),("p.ema".into(),raw.into())]);assert!(books::elcim_account::validate(&params_of(&q)).is_some(),"{raw}");
        }
        let q=BTreeMap::from([("id".into(),books::elcim_account::ID.into()),("p.typo".into(),"bad".into())]);assert!(books::elcim_account::validate(&params_of(&q)).is_some());
        let q=BTreeMap::from([("id".into(),books::elcim_account::ID.into()),("p.ema".into(),"13".into())]);assert!(books::elcim_account::validate(&params_of(&q)).is_none());
    }

    /// Opt-in actual supplied prices; output stays in a caller-selected private
    /// directory. This is candidate/reference comparison, not self-admission.
    #[test]
    #[ignore]
    fn elder_actual_account_evidence(){
        let path=std::env::var("ELDER_ACTUAL_CSV").expect("ELDER_ACTUAL_CSV required");
        let symbol=std::env::var("ELDER_ACTUAL_SYMBOL").unwrap();let dir=std::env::var("ELDER_EVIDENCE_DIR").unwrap();
        let start=std::env::var("ELDER_ACTUAL_FROM").unwrap_or("2010-01-01".into());let end=std::env::var("ELDER_ACTUAL_TO").unwrap_or("2020-09-11".into());
        let text=std::fs::read_to_string(path).unwrap();let mut r=vec![];
        for line in text.lines().skip(1){let f=parse_csv_fields(line);if f[0]>end{break;}let v=|i:usize|f[i].parse::<f64>().unwrap();r.push(Row{ticker:symbol.clone(),date:f[0].clone(),open:v(1),high:v(2),low:v(3),close:v(4),volume:Some(v(5))});}
        let from=r.partition_point(|r|r.date<start);let to=r.len()-1;let x=Ctx::new(&r,&r,&[]);let st=books::elcim_account::states(&x);
        let mut states="date,ema13,histogram,weekly_direction_at_open,safezone_long_at_open,safezone_short_at_open,green,red\n".to_string();
        for i in 0..r.len(){let _=writeln!(states,"{},{},{},{},{},{},{},{}",r[i].date,st.ema13[i],st.histogram[i],st.weekly[i],st.long_stop[i],st.short_stop[i],st.green[i],st.red[i]);}
        std::fs::create_dir_all(&dir).unwrap();std::fs::write(format!("{dir}/{symbol}-states.csv"),states).unwrap();
        for bps in [0.,5.] {
            let costs=accounting::Costs{fees_bps:0.,slippage_bps:bps,rollover_bps:0.};let run=backtest(&x,books::elcim_account::ID,from,to,costs.side_rate(),&Params::new()).unwrap();let a=run.source_account.as_ref().unwrap();
            let mut curve="date,net_equity,gross_same_quantity_equity,costs_paid,free_cash,reserved_short_principal,restricted_short_proceeds,quantity,direction,stop,reference,monthly_realized_loss,outstanding_planned_risk,halted,liquidation_pending\n".to_string();
            for d in &a.days{let _=writeln!(curve,"{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",r[d.i].date,d.equity,d.gross,d.costs,d.cash,d.reserve,d.proceeds,d.quantity,d.direction,d.stop,d.reference,d.losses,d.outstanding,d.halted,d.pending);}
            let mut ledger="entry_date,exit_date,direction,entry_price,exit_price,entry_weight,entry_equity,quantity,notional,initial_stop,final_stop,planned_risk,risk_budget,month_reference,entry_fee,exit_fee,gross_account_contribution,net_account_contribution,net_fraction_of_entry_equity,open,exit_reason\n".to_string();
            for (t,audit) in a.trades.iter().zip(&a.trade_audit){let _=writeln!(ledger,"{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",r[t.legs[0].i].date,r[t.exit_i].date,t.dir,t.legs[0].px,t.exit_px,t.legs[0].w,audit.entry_equity,audit.quantity,audit.notional,audit.initial_stop,audit.final_stop,audit.planned_risk,audit.risk_budget,audit.month_reference,audit.entry_fee,audit.exit_fee,audit.gross,audit.net,audit.net/audit.entry_equity,t.open,t.reason);}
            let s=financial_stats(&x,&run,costs);assert!((s["ledger_gross_return"]-s["fee_return"]-s["slippage_return"]-s["total_return"]).abs()<1e-9);assert!((s["avg_net_pnl"]*s["trades"]-s["net_profit"]).abs()<1e-10);
            std::fs::write(format!("{dir}/{symbol}-cost{bps}-equity.csv"),curve).unwrap();std::fs::write(format!("{dir}/{symbol}-cost{bps}-trades.csv"),ledger).unwrap();std::fs::write(format!("{dir}/{symbol}-cost{bps}-stats.json"),stats_json(&s)).unwrap();std::fs::write(format!("{dir}/{symbol}-cost{bps}-account.json"),a.json(&x)).unwrap();
            eprintln!("Elder actual{symbol} bps{bps}:{} closed{} open;final{}",a.trades.iter().filter(|t|!t.open).count(),a.trades.iter().filter(|t|t.open).count(),run.equity.last().unwrap());
        }
    }

    struct NativeFixture(std::path::PathBuf);
    impl Drop for NativeFixture{fn drop(&mut self){let _=std::fs::remove_dir_all(&self.0);}}
    fn native_us_fixture()->(NativeFixture,State,BTreeMap<String,String>){
        let dir=std::env::temp_dir().join(format!("vault-us-context-{}-{}",std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));std::fs::create_dir_all(&dir).unwrap();
        let mut csv="date,open,high,low,close,volume\n".to_string();
        for i in 0..220 {let c=100.+i as f64/2.;let _=writeln!(csv,"2020-{:02}-{:02},{},{},{},{},1000",i/28+1,i%28+1,c,c+1.,c-1.,c);}
        std::fs::write(dir.join("quote.csv"),&csv).unwrap();
        let mut hash=std::process::Command::new("sha256sum").stdin(std::process::Stdio::piped()).stdout(std::process::Stdio::piped()).spawn().unwrap();hash.stdin.take().unwrap().write_all(csv.as_bytes()).unwrap();let out=hash.wait_with_output().unwrap();assert!(out.status.success());let sha=String::from_utf8(out.stdout).unwrap().split_whitespace().next().unwrap().to_string();
        std::fs::write(dir.join("index.csv"),format!("basket,ticker,path,first,last,bars,source,source_sha256,csv_sha256,membership,status,has_volume,trusted_market,currency,asset_type,timeframe,adjustment_status,source_price_units_verified,membership_kind,membership_source,membership_source_sha256,mapped_index\nus_sp500,AAPL,quote.csv,2020-01-01,2020-08-24,220,Test stock source,{}, {},Test current snapshot,ready,true,us,USD,individual_stock,D,not_independently_verified,false,current_snapshot,Exact test list,{},SPX\n","a".repeat(64),sha,"b".repeat(64))).unwrap();
        let benchmark=|ticker:&str,c:f64|->&'static [Row]{Box::leak((0..220).map(|i|Row{ticker:ticker.into(),date:format!("2020-{:02}-{:02}",i/28+1,i%28+1),open:c,high:c+1.,low:c-1.,close:c,volume:Some(1000.)}).collect::<Vec<_>>().into_boxed_slice())};
        let spx=benchmark("SPX",200.);let nifty=benchmark("NIFTY 50",9999.);
        let india_vix=spx.iter().map(|r|Row{ticker:"INDIA VIX".into(),date:r.date.clone(),open:20.,high:21.,low:19.,close:20.,volume:None}).collect();
        let extras=BTreeMap::from([(("*".into(),"VIX.Close".into()),spx.iter().map(|r|(r.date.clone(),999999.)).collect())]);
        let state=State{ctxs:BTreeMap::from([("SPX".into(),Ctx::new(spx,spx,&[])),("NIFTY 50".into(),Ctx::new(nifty,nifty,&[]))]),baskets:baskets::Catalogue::load(&dir).unwrap(),vix:india_vix,market_extra:extras};
        let q=BTreeMap::from([("ticker".into(),"basket:us_sp500:AAPL".into()),("id".into(),"f1a".into()),("cost".into(),"3".into()),("slippage".into(),"2".into())]);
        (NativeFixture(dir),state,q)
    }
    #[test]
    fn native_us_context_uses_spx_and_never_india_vix_or_client_currency(){
        let (_fixture,state,mut q)=native_us_fixture();q.insert("currency".into(),"INR".into());q.insert("native".into(),"false".into());
        let selected=select_context(&state,&q).unwrap();assert_eq!(selected.evidence().unwrap().currency(),"USD");let held=selected.context(&state);
        // SMA20(close*1000), divided by actual SPX's SMA65(200*1000).
        let stock_mean=(81..=100).map(|i|100.+i as f64/2.).sum::<f64>()/20.;let expected=1_000_000.*stock_mean/200.;assert!((held.lf[100]-expected).abs()<1e-9);
        assert!(held.vix.first.is_none());assert!(held.extra["VIX.Close"].iter().all(|v|!v.is_finite()),"Generic extra VIX values cannot restore a rejected market source");
        assert!(readiness_json(&state,&q).unwrap().contains("\"status\":\"ready\""));
        let run=detail(&state,&q).unwrap();assert!(run.contains("\"currency\":\"USD\"")&&run.contains("\"benchmark_symbol\":\"AAPL\""));assert!(!run.contains("999999"));
        q.insert("id".into(),"r3".into());assert!(detail(&state,&q).unwrap_err().contains("index"));
        q.insert("id".into(),"hpetf_3dhl_long".into());assert!(detail(&state,&q).unwrap_err().contains("ETF"));
    }
    #[test]
    fn detail_publishes_full_trade_return_precision_without_changing_legacy_rounding(){
        let (_fixture,state,q)=native_us_fixture();let selected=select_context(&state,&q).unwrap();let held=selected.context(&state);let (ctx,from,to,cost)=resolve_context(&held,&q).unwrap();
        let run=backtest(ctx,"f1a",from,to,cost,&Params::new()).unwrap();let t=run.trades.iter().find(|t|!t.open).expect("fixture needs a closed actual source trade");let value=t.ret(cost);
        assert_ne!(num(value),value.to_string(),"fixture must exercise rounding loss");let body=detail(&state,&q).unwrap();
        assert!(body.contains(&format!("\"ret\":{},\"ret_full_precision\":{}",num(value),value)));
        assert_eq!(full_precision_trade_return(f64::NAN),"null");assert_eq!(full_precision_trade_return(f64::INFINITY),"null");
    }
    #[test]
    fn native_requested_coverage_is_not_silently_extended_or_shortened(){
        let (_fixture,state,mut q)=native_us_fixture();
        for (key,value,reason) in [("from","2019-12-31","precedes"),("to","2020-08-25","exceeds")] {
            q.insert(key.into(),value.into());let assessment=readiness_json(&state,&q).unwrap();assert!(assessment.contains("needs_data")&&assessment.contains(reason));
            assert!(detail(&state,&q).unwrap_err().contains(reason));assert!(optimize(&state,&q).unwrap_err().contains(reason));
            assert!(!summary(&state,&q).unwrap().contains("\"stats\""));q.remove(key);
        }
        for (key,value) in [("from","2020-02-31"),("to","2020-99-01")] {q.insert(key.into(),value.into());assert!(readiness_json(&state,&q).unwrap_err().contains("valid YYYY-MM-DD"));q.remove(key);}
        q.insert("from".into(),"2020-01-01".into());q.insert("to".into(),"2020-08-24".into());assert!(detail(&state,&q).is_ok());
        q.insert("g.hold".into(),"4:5:1".into());let result=optimize(&state,&q).unwrap();assert!(result.contains("\"combos\":2"));assert!(result.contains("\"slippage_bps\":2"));
    }
    #[test]
    fn native_us_liquidity_filter_cannot_fall_back_to_nifty_or_missing_spx_volume(){
        let (_fixture,mut state,mut q)=native_us_fixture();q.insert("id".into(),"tps_stocks".into());q.insert("from".into(),"2020-04-01".into());
        assert!(readiness_json(&state,&q).unwrap().contains("\"status\":\"ready\""));
        state.ctxs.remove("SPX");let assessment=readiness_json(&state,&q).unwrap();assert!(assessment.contains("needs_data")&&assessment.contains("SPX"));assert!(detail(&state,&q).unwrap_err().contains("SPX"));
        let mut missing=state.ctxs["NIFTY 50"].r.to_vec();for row in &mut missing{row.ticker="SPX".into();row.volume=None;}
        let rows=Box::leak(missing.into_boxed_slice());state.ctxs.insert("SPX".into(),Ctx::new(rows,rows,&[]));
        assert!(detail(&state,&q).unwrap_err().contains("Benchmark volume/coverage"));
    }
    #[test]
    fn volatility_loader_preserves_actual_identity_and_market_mismatch_is_withheld(){
        let (fixture,state,_q)=native_us_fixture();
        std::fs::write(fixture.0.join("vix.csv"),"Ticker,Date,Time,Open,High,Low,Close\nVIX,2020-01-01,15:00:00,20,20,20,20\nVIX,2020-01-02,15:00:00,21,21,21,21\n").unwrap();
        let rows=load_vix_dir(fixture.0.to_str().unwrap());assert_eq!(rows[0].ticker,"VIX","A generic source label must not be renamed India/CBOE VIX");
        assert!(market_vix(&rows,Some(baskets::Market::UnitedStates)).0.is_empty());assert!(market_vix(&state.vix,Some(baskets::Market::UnitedStates)).0.is_empty());
        assert_eq!(market_vix(&state.vix,Some(baskets::Market::India)).1,Some("INDIA VIX"));
        let c=Ctx::new(state.ctxs["SPX"].r,state.ctxs["SPX"].r,market_vix(&state.vix,preloaded_market("SPX")).0);let def=connors::catalogue().into_iter().find(|d|d.id=="vix2").unwrap();
        assert_eq!(readiness::check(&c,&def,0,c.r.len()-1,&Params::new()).unwrap().status,"needs_data");
        std::fs::write(fixture.0.join("vix.csv"),"Ticker,Date,Time,Open,High,Low,Close\nINDIA VIX,2020-01-01,15:00:00,20,21,19,20\nVIX,2020-01-02,15:00:00,21,22,20,21\n").unwrap();assert!(load_vix_dir(fixture.0.to_str().unwrap()).is_empty(),"Mixed instruments cannot be stitched into one VIX source");
    }

    fn accounting_state()->State {
        let rows:&'static [Row]=Box::leak((0..80).map(|i|{let close=100.+i as f64;Row{ticker:"SPX".into(),date:format!("2020-{:02}-{:02}",i/28+1,i%28+1),open:close+0.5,high:close+2.,low:close-1.,close,volume:Some(100.)}}).collect::<Vec<_>>().into_boxed_slice());
        State{ctxs:BTreeMap::from([("SPX".into(),Ctx::new(rows,rows,&[]))]),baskets:Default::default(),vix:vec![],market_extra:Default::default()}
    }
    fn accounting_query()->BTreeMap<String,String>{
        BTreeMap::from([("ticker".into(),"SPX".into()),("id".into(),"ch_65sma3cc".into()),("_guru".into(),"chande".into()),("cost".into(),"3".into()),("slippage".into(),"2".into())])
    }
    #[test]
    fn chart_ohlc_keeps_full_admitted_warmup_and_rejects_unknown_symbols(){
        let state=accounting_state();let mut q=accounting_query();
        q.insert("from".into(),"2020-03-01".into());
        q.insert("to".into(),"2020-03-10".into());
        let full=chart_ohlc_json(&state,&q).unwrap();
        assert!(full.contains("\"full_history\":true"));
        assert!(full.contains("2020-01-01"));
        assert!(full.contains("2020-03-24"));
        assert!(full.contains("\"ticker\":\"SPX\""));
        assert!(full.contains("\"o\":[100.5,101.5"));
        q.insert("ticker".into(),"../../unknown.csv".into());
        assert!(chart_ohlc_json(&state,&q).is_err());
    }
    #[test]
    fn chart_source_prices_preserve_indicator_precision(){
        let rows:&'static [Row]=Box::leak((1..=3).map(|i|Row{ticker:"SPX".into(),date:format!("2020-01-0{i}"),open:100.123456789,high:101.,low:99.,close:100.234567891,volume:None}).collect::<Vec<_>>().into_boxed_slice());
        let mut state=accounting_state();state.ctxs.insert("SPX".into(),Ctx::new(rows,rows,&[]));
        let json=chart_ohlc_json(&state,&accounting_query()).unwrap();
        assert!(json.contains("100.123456789"));assert!(json.contains("100.234567891"));
        assert!(json.contains("\"v\":[null,null,null]"));
    }
    #[test]
    fn execution_costs_are_validated_consistently_at_every_endpoint(){
        let state=accounting_state();let mut q=accounting_query();
        assert!((resolve(&state,&q).unwrap().3-0.0005).abs()<1e-15);
        for key in ["cost","slippage","rollover"]{
            q.insert(key.into(),"NaN".into());
            assert!(detail(&state,&q).is_err());assert!(summary(&state,&q).is_err());assert!(optimize(&state,&q).is_err());assert!(readiness_json(&state,&q).is_err());
            q=accounting_query();
        }
        q.insert("rollover".into(),"1".into());
        assert!(readiness_json(&state,&q).unwrap().contains("\"status\":\"needs_data\""));
        for error in [detail(&state,&q).unwrap_err(),summary(&state,&q).unwrap_err(),optimize(&state,&q).unwrap_err()]{assert!(error.contains("needs_data")&&error.contains("contract"),"{error}");}
        q.insert("id".into(),"jr_hook_l".into());q.insert("_guru".into(),"ross".into());
        let assessment=readiness_json(&state,&q).unwrap();assert!(assessment.contains("not_testable")&&assessment.contains("Rollover costs"));
        assert!(detail(&state,&q).unwrap_err().starts_with("not_testable"));
    }
    #[test]
    fn additive_returns_keep_old_statistics_and_reconcile_fees_without_money(){
        let state=accounting_state();let ctx=&state.ctxs["SPX"];
        let trade=Trade{dir:1.,legs:vec![connors::Leg{i:0,px:100.,w:1.}],exit_i:5,exit_px:110.,reason:"arithmetic close".into(),entry_kind:"close",signal_i:5,open:false,exits:vec![]};
        let costs=accounting::Costs::parse(&accounting_query()).unwrap();
        let run=Run{trades:vec![trade],from:0,to:5,equity:equity_curve(&ctx.c,&[Trade{dir:1.,legs:vec![connors::Leg{i:0,px:100.,w:1.}],exit_i:5,exit_px:110.,reason:String::new(),entry_kind:"close",signal_i:5,open:false,exits:vec![]}],0,5,costs.side_rate()),bh:vec![1.;6],source_account:None};
        let legacy=stats(ctx,&run,costs.side_rate());let added=financial_stats(ctx,&run,costs);
        for (key,value) in legacy {assert!(added[key].to_bits()==value.to_bits(),"Legacy statistic {key} changed");}
        assert!((added["ledger_gross_return"]-added["fee_return"]-added["slippage_return"]-added["total_return"]).abs()<1e-12);
        assert_eq!(added["avg_net_pnl"],added["avg_trade"]);
        let payload=detail(&state,&accounting_query()).unwrap();
        assert!(payload.contains("\"benchmark_symbol\":\"SPX\""));assert!(payload.contains("\"taxes\":null"));assert!(!payload.contains("\"initial_capital\""));
        assert!(payload.contains("\"gross_equity\""));
    }

    #[test]
    fn closed_return_sum_is_not_compounding_or_an_open_mark(){
        let state=accounting_state();let ctx=&state.ctxs["SPX"];
        let make=|start,end,exit_px,open|Trade{dir:1.,legs:vec![connors::Leg{i:start,px:100.,w:1.}],exit_i:end,exit_px,reason:String::new(),entry_kind:"close",signal_i:start,open,exits:vec![]};
        let trades=vec![make(0,2,110.,false),make(3,5,80.,false),make(6,7,150.,true)];
        let run=Run{equity:equity_curve(&ctx.c,&trades,0,7,0.),trades,from:0,to:7,bh:vec![1.;8],source_account:None};
        let s=financial_stats(ctx,&run,accounting::Costs{fees_bps:0.,slippage_bps:0.,rollover_bps:0.});
        assert_eq!(s["trades"],2.);
        assert!((s["net_profit"]+0.1).abs()<1e-12);
        assert!((s["net_profit"]-s["avg_net_pnl"]*s["trades"]).abs()<1e-12);
        assert!((s["compounded_return"]-0.32).abs()<1e-12);
        assert_eq!(s["total_return"],s["compounded_return"]);
    }
    #[test]
    fn initial_loss_or_fee_is_in_daily_risk_and_drawdown(){
        let state=accounting_state();let ctx=&state.ctxs["SPX"];
        let run=Run{trades:vec![],from:0,to:2,equity:vec![0.9,0.99,0.98],bh:vec![1.;3],source_account:None};
        let s=stats(ctx,&run,0.);
        assert!((s["total_return"]+0.02).abs()<1e-12);
        assert!((s["max_dd"]+0.1).abs()<1e-12);
        assert!((s["sharpe"]+0.5335886135030962).abs()<1e-6);
        assert!((s["sortino"]+0.9210858546307442).abs()<1e-6);
        assert_eq!(max_dd(&[0.9,0.99,0.98]).1,3);
        assert_eq!(max_dd(&[1.,1.,1.]),(0.,0));
        assert!((max_dd(&[1.1,0.99]).0+0.1).abs()<1e-12);
    }
    #[test]
    fn day_count_and_years() {
        assert_eq!(day_number("1970-01-01"), 0);
        assert_eq!(day_number("2000-03-01") - day_number("2000-02-28"), 2); // leap year
        assert_eq!(day_number("2026-01-01") - day_number("2025-01-01"), 365);
    }

    #[test]
    fn conclusion_rank_prefers_all_round_combos() {
        // cols: trades, then the five rank stats. Row 0 is best on CAGR only and worst elsewhere;
        // row 1 is second-best on everything; row 3 has too few trades and must be ignored.
        let rows = vec![
            vec![50.0, 0.30, 0.1, 0.1, 0.1, 0.1],
            vec![50.0, 0.20, 1.9, 1.9, 1.9, 1.9],
            vec![50.0, 0.10, 2.0, 2.0, 2.0, 2.0],
            vec![5.0, 9.00, 9.0, 9.0, 9.0, 9.0],
            vec![50.0, 0.05, 0.5, 0.5, 0.5, 0.5],
        ];
        let (j, _) = conclude_rank(&rows, 0, &[1, 2, 3, 4, 5], 10.0, 1.0, &[]);
        assert!(j.contains("\"eligible\":4"), "{j}");
        let top = &j[j.find("\"top3\":[").unwrap()..];
        // scores (mean percentile): row 2 86.7, row 1 66.7, row 4 26.7, row 0 20 (CAGR leader, worst elsewhere); row 3 excluded
        let order: Vec<usize> = top.match_indices("\"row\":").map(|(i, _)| top[i + 6..].split(|c: char| !c.is_ascii_digit()).next().unwrap().parse().unwrap()).collect();
        assert_eq!(order, vec![2, 1, 4], "{j}");
    }

    #[test]
    fn conclusion_rank_collapses_identical_outcomes() {
        // cols: param, trades, five stats. Rows 0 and 1 have identical results; row 1 is the book value (param 4) and must represent them.
        let rows = vec![vec![9.0, 50.0, 0.3, 2.0, 2.0, 2.0, 2.0], vec![4.0, 50.0, 0.3, 2.0, 2.0, 2.0, 2.0], vec![5.0, 50.0, 0.1, 1.0, 1.0, 1.0, 1.0]];
        let (j, _) = conclude_rank(&rows, 1, &[2, 3, 4, 5, 6], 10.0, 1.0, &[4.0]);
        assert!(j.contains("\"eligible\":2"), "{j}");
        let top = &j[j.find("\"top3\":[").unwrap()..];
        assert!(top.starts_with("\"top3\":[{\"row\":1,"), "{j}");
    }

    #[test]
    fn additive_accounting_columns_do_not_change_legacy_ranking(){
        let original=vec![vec![9.,50.,0.3,2.,2.,2.,2.],vec![4.,50.,0.3,2.,2.,2.,2.],vec![5.,50.,0.1,1.,1.,1.,1.]];
        let mut added=original.clone();added[0].push(123.);added[1].push(456.);added[2].push(789.);
        let before=conclude_rank(&original,1,&[2,3,4,5,6],10.,1.,&[4.]);
        let after=conclude_rank_with_outcomes(&added,1,&[2,3,4,5,6],10.,1.,&[4.],Some(&[1,2,3,4,5,6]));
        assert_eq!(before,after);
    }

    #[test]
    fn scale_out_piece_is_banked_mid_trade() {
        // long 1 @100 on bar 0; half out @110 on bar 1; rest out @90 on bar 3 (exit_px 100)
        let t = Trade { dir: 1.0, legs: vec![connors::Leg { i: 0, px: 100.0, w: 1.0 }], exit_i: 3, exit_px: 100.0, reason: String::new(),
            entry_kind: "close", signal_i: 3, open: false, exits: vec![(1, 110.0, 0.5), (3, 90.0, 0.5)] };
        let close = [100.0, 108.0, 120.0, 95.0];
        let eq = equity_curve(&close, std::slice::from_ref(&t), 0, 3, 0.0);
        // bar 1: 0.5*110 + 0.5*108 = 109; bar 2: 0.5*110 + 0.5*120 = 115; bar 3: exit_px
        let want = [1.0, 1.09, 1.15, 1.0];
        assert!(eq.iter().zip(want).all(|(a, b)| (a - b).abs() < 1e-12), "{eq:?}");
        assert_eq!(*eq.last().unwrap(), 1.0 + t.ret(0.0));
        // no pieces: bar 2 marked to the close (old behaviour)
        let one = Trade { exits: Vec::new(), ..t };
        assert!((equity_curve(&close, &[one], 0, 3, 0.0)[2] - 1.2).abs() < 1e-12);
    }

    #[test]
    fn equity_equals_compounded_trade_returns() {
        // Every live strategy on real data: the final equity must
        // equal the product of (1 + trade return). Catches any drift between trade P&L and the curve.
        // VAULT_IDX overrides the data path, so the test also runs from a private copy of the backend.
        let path = std::env::var("VAULT_IDX").unwrap_or_else(|_| concat!(env!("CARGO_MANIFEST_DIR"), "/../../../idx_data.csv").into());
        let data = load_csv(&path).unwrap();
        let rows = &data["NIFTY 50"];
        let ctx = Ctx::new(rows, rows, &[]);
        for d in connors::catalogue().iter().filter(|d| d.status == Status::Live && !d.needs_vix) {
            if d.id==books::elcim_account::ID {
                // This source account requires actual stocks plus real weekly
                // warmup. Its opt-in AAPL/RELIANCE evidence test covers account
                // conservation; a Nifty/from-zero corpus shortcut cannot do so.
                assert!(instrument_policy::check(d,"NIFTY 50",false).is_some());
                continue;
            }
            for cost in [0.0, 0.0005] {
                let raw = connors::run(d.id, &ctx, 0, rows.len()-1, &Params::new()).unwrap();
                if let Err(reason) = trade_validation::check(&raw,0,rows.len()-1,cost) {
                    assert!(backtest(&ctx,d.id,0,rows.len()-1,cost,&Params::new()).unwrap_err().contains(&reason),"{}",d.id);
                    continue;
                }
                let marks=equity_curve(&ctx.c,&raw,0,rows.len()-1,cost);
                if marks.iter().any(|v|!v.is_finite()||*v<=0.) {
                    assert!(backtest(&ctx,d.id,0,rows.len()-1,cost,&Params::new()).unwrap_err().contains("nonpositive or nonfinite"),"{}",d.id);
                    continue;
                }
                let run = backtest(&ctx, d.id, 0, rows.len() - 1, cost, &Params::new()).unwrap();
                // an open trade is marked as if closed at the last close, exit cost included
                let want: f64 = run.trades.iter().map(|t| 1.0 + t.ret(cost)).product();
                let got = *run.equity.last().unwrap();
                assert!((got / want - 1.0).abs() < 1e-9, "{} cost {}: equity {} vs trades {}", d.id, cost, got, want);
            }
        }
    }
}
