//! Alberto Vivanti & Perry J. Kaufman, *Global Equity Investing* (319 scanned pages, no text layer). Page refs are
//! printed pages (Ch 3-4: PDF = printed + 20; Ch 6-7: PDF = printed + 22). The mechanical systems are the six
//! long-only weekly systems A-F (Ch 6, p.189-203, S&P 500 / Nikkei / FAZ 1971-1995, with full trade lists) and the
//! "UC" system (p.209-210). C, D and F duplicate vault strategies (nfpk_ma20w_f1 with 40 / 20 weeks, kpss_janbar)
//! and are not re-coded; see library/kfgei/README.md. Every default is the book's literal number or is labelled.
use crate::books::swsp::week_ends;
use crate::connors::{sma, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "kfgei";


pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, status, rules, notes| StrategyDef {
        id, name, group, session: 0, dir: "Long", status, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None,
    };
    use Status::*;
    vec![
        d("kfgei_sys_a", "System A · 18-week breakout (weekly)", "Ch 6 · Long-term trading techniques", Live, &[
            "E|\"Buy when the weekly closing price moves above the high of the previous 18 weeks.\" (p.191)",
            "X|\"Sell when the weekly closing price moves below the low of the previous 18 weeks.\" (p.191)",
            "F|Table 6-5 heading: \"Buy when the weekly close moves up to a new 18-week high; sell when the weekly close moves down to a new 18-week low.\" (p.197)",
        ], concat!("Default reading = the text: weekly close above the highest weekly HIGH of the 18 weeks before this one (below the lowest weekly LOW to sell); filled at that weekly close. p.hl=0 uses the highest/lowest weekly CLOSE instead (the Table 6-5 heading's 'new 18-week high'). Book S&P 1971-95 (Table 6-5): 13 trades, 77% winners, avg +12.50%. ",
            "Weekly bars: a week closes on the last trading day of each Monday-Sunday week (shared helper swsp::week_ends); weekly high/low = the extreme of that week's daily bars. Long only, cash earns nothing, no commissions (p.189). India: NIFTY indices (instrument swap).")),
        d("kfgei_sys_b", "System B · 10-week breakout, 7% drawdown exit (weekly)", "Ch 6 · Long-term trading techniques", Live, &[
            "E|\"Buy when the weekly closing price moves above the high of the previous 10 weeks.\" (p.191)",
            "X|\"Sell when weekly close falls more than 7% from the highest price recorded during the trade.\" (p.191)",
            "X|\"The exit applies the money management criteria of a 7% drawdown in total equity and requires that the position be liquidated and funds held in cash until the next buy signal.\" (p.192)",
        ], concat!("Default reading: entry as System A with 10 weeks (highest weekly HIGH of the 10 prior weeks; p.hl=0 = highest weekly close); the 'highest price recorded during the trade' = the highest daily high from the entry bar's close onward (the entry close itself counts) (p.peak=1 = highest weekly close). Exit when the weekly close is more than 7% below it, at that close. 'until the next buy signal': after an exit a new entry needs a fresh weekly close above the 10-week high. Book S&P 1971-95 (Table 6-6): 27 trades, 56% winners, avg +6.53%; called the best of the six (p.193). ",
            "Weekly bars: a week closes on the last trading day of each Monday-Sunday week (shared helper swsp::week_ends). Long only, cash earns nothing, no commissions (p.189). India: NIFTY indices (instrument swap).")),
        d("kfgei_sys_e", "System E · 40-week average with a 5-week breakout stop (weekly)", "Ch 6 · Long-term trading techniques", Live, &[
            "E|\"If the weekly close is above the average of the past 40 weekly closes, place a buy stop order for next week just above the high of the past five weeks.\" (p.192)",
            "X|\"If the weekly close is below the average of the past 40 weekly closes, place a sell stop order for next week just below the low of the past five weeks.\" (p.192)",
            "F|Table 6-9 heading: \"put a buy order with a stop at the highest of 5 weeks ... a sell order with a stop at the lowest of 5 weeks\" (p.202)",
        ], concat!("The 40-week average includes this week's close; the 5 weeks are this week and the four before it. 'Just above/below' has no size: the stop is placed AT the 5-week high/low (Table 6-9 heading 'a stop at the highest of 5 weeks'). The order is good for the next week only and is re-placed each week while the condition holds; a stop fills at the stop price, or at the day's open if the market gaps through it. Long only: the sell stop only exits a long. Book S&P 1971-95 (Table 6-9): 27 trades, 48% winners, avg +5.30%. ",
            "Weekly bars: a week closes on the last trading day of each Monday-Sunday week (shared helper swsp::week_ends). Cash earns nothing, no commissions (p.189). India: NIFTY indices (instrument swap).")),
        d("kfgei_uc", "System UC · buy each January, exit on 3% loss or 15% profit", "Ch 6 · Using multiple methods to reduce risk", Live, &[
            "E|\"Buy the market at the beginning of the year.\" (p.209)",
            "X|\"Sell only if one of the following two cases occurs: a 3% loss or a 15% profit.\" (p.209)",
            "F|\"If the long position is closed out because of either profits or losses, the system will not reenter the market again before the next January.\" (p.209)",
        ], concat!("Readings: 'beginning of the year' = the open of the year's first trading day [book silent on the order type]; loss and profit are checked on weekly closes like the other Ch 6 systems (p.189) and filled at that close (p.daily=1 checks every daily close). The book gives no year-end exit, so a position that hits neither level is carried into the next year; Table 6-12 agrees: US 1971 = +10.8% (= buy and hold) and 1972 = +4.0% (the carried 1971 position reaching +15% early in 1972). Book (Table 6-12, p.210): US 1971-95 total 248%, 5.1% a year, yearly sd 9%; yearly returns include no trade list. ",
            "Cash earns nothing, no commissions. India: NIFTY indices (instrument swap).")),
        d("kfgei_fx_hedge", "Currency hedge on the 10-week breakout", "Ch 4 · How to profit from periods of strength in foreign currencies", NeedsData("weekly exchange rates (DEM, JPY, GBP, CHF per USD) and the foreign index in USD; the vault has no FX series"), &[
            "E|\"Hedge the currency of your foreign stock investment when its price moves above the 10-week high\" (p.121)",
            "X|\"'unhedge' (lift the hedge) when the currency price moves below the 10-week low.\" (p.121)",
        ], "Book: German mark 1976-95, 20 trades, 60% profitable, avg +5.48% (Table 4-4, p.123); Japanese yen 18 trades, 83%, avg +9.45% (Table 4-5, p.124). The price is the foreign-exchange quote in units per dollar (p.121)."),
        d("kfgei_world_b", "System B on an equally weighted US / Japan / Germany portfolio", "Ch 6 · Diversification applied to trading methods", NeedsData("Nikkei 225 and FAZ (German) index history; the engine runs one instrument per strategy"), &[
            "E|\"If we kept each country allocation at one-third, and we traded each market using system B ... the result ... is given in Table 6-11.\" (p.207)",
        ], "Book (Table 6-11, p.208): 1971-95 total 623%, 8.2% a year, sd 13%, risk-adjusted 0.63 (buy and hold 706%, 8.7%, 16%, 0.54). Also Table 6-12: 50% B + 50% UC portfolio 396%, 6.6% a year."),
        d("kfgei_alloc_vol", "Volatility allocation rule (US / Japan / Germany)", "Ch 7 · Building an international portfolio", NeedsData("Nikkei 225, FAZ and USD/JPY, USD/DEM yearly returns 1973-1995"), &[
            "E|\"Allocate two-thirds of the investment to the one market (out of three) that had the lowest standard deviation of yearly returns, in U.S. dollars, based on the average of the last three years. Allocate the remaining one-third equally to the other two markets\" (p.217)",
        ], "Yearly rebalancing (Table 7-2 shows one allocation per year). Book 1976-95: total 1092%, 13.2% a year (Table 7-2, p.218)."),
        d("kfgei_alloc_rar", "Risk-adjusted-ratio allocation rule", "Ch 7 · Building an international portfolio", NeedsData("Nikkei 225, FAZ and USD/JPY, USD/DEM yearly returns 1973-1995"), &[
            "E|\"Allocate two-thirds of the portfolio investment to the market with the best risk-adjusted ratio, calculated over the past three yearly results. Allocate the remaining one-third equally to the other two markets\" (p.217-218)",
        ], "Book 1976-95: total 1103%, 13.7% a year (Table 7-3, p.219)."),
        d("kfgei_alloc_corr", "Correlation allocation rule", "Ch 7 · Building an international portfolio", NeedsData("Nikkei 225, FAZ and USD/JPY, USD/DEM returns 1973-1995"), &[
            "E|\"We always allocate 70% of our investment to the U.S. market and the remaining 30% to the one other market, either the Japanese or German, that has shown the least correlation to the Dow over the past three years\" (p.220)",
        ], "The correlation's sampling (yearly / monthly returns) is not stated. Book 1976-95: total 593%, 10.1% a year (Table 7-4, p.221)."),
        d("kfgei_alloc_combined", "Combined allocation (average of the three rules)", "Ch 7 · Combining methods", NeedsData("Nikkei 225, FAZ and USD/JPY, USD/DEM returns 1973-1995"), &[
            "E|\"We now combined the three allocation methods, volatility, risk-adjusted ratio, and correlation, by simply averaging the allocations that we would get for each market according to the three methods\" (p.221)",
        ], "Book 1976-95: total 926%, 12.4% a year (Table 7-5, p.222)."),
        d("kfgei_rel_strength", "Six-month relative strength vs the world index", "Ch 3 · Relative strength and cyclic lag", NotTestable("an indicator read from charts; the book gives no buy or sell rule (p.81-84), and the vault has no world index"), &[
            "F|\"RS = ROC(country index) - ROC(world index)\", ROC = today's value - value six months ago (p.81)",
        ], ""),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "kfgei_sys_a" => ("kfgei_weekly", "Global Equity Investing weekly systems", "A · 18-week breakout"),
        "kfgei_sys_b" => ("kfgei_weekly", "Global Equity Investing weekly systems", "B · 10-week breakout, 7% exit"),
        "kfgei_sys_e" => ("kfgei_weekly", "Global Equity Investing weekly systems", "E · 40-week MA + 5-week stop"),
        "kfgei_uc" => ("kfgei_weekly", "Global Equity Investing weekly systems", "UC · January, 3% / 15%"),
        "kfgei_world_b" => ("kfgei_weekly", "Global Equity Investing weekly systems", "B · three markets"),
        s if s.starts_with("kfgei_alloc") => ("kfgei_alloc", "Global Equity Investing allocation rules", ""),
        s if s.starts_with("kfgei_") => ("kfgei_other", "Global Equity Investing", ""),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let hl = p("hl", "Breakout level: 1 = weekly highs/lows (text), 0 = weekly closes (table heading)", 1., 0., 1., 1.);
    Some(match id {
        "kfgei_sys_a" => vec![p("weeks", "Breakout weeks", 18., 2., 60., 1.), hl],
        "kfgei_sys_b" => vec![p("weeks", "Breakout weeks", 10., 2., 60., 1.), p("dd", "Exit drawdown from the trade's highest price (%)", 7., 1., 30., 0.5), hl,
            p("peak", "Highest price: 0 = daily highs (text), 1 = weekly closes", 0., 0., 1., 1.)],
        "kfgei_sys_e" => vec![p("ma", "Average of weekly closes (weeks)", 40., 5., 100., 1.), p("weeks", "Stop at the high/low of the past N weeks", 5., 1., 30., 1.)],
        "kfgei_uc" => vec![p("loss", "Loss exit (%)", 3., 0.5, 20., 0.5), p("profit", "Profit exit (%)", 15., 1., 50., 1.),
            p("daily", "Check exits on: 0 = weekly closes, 1 = daily closes", 0., 0., 1., 1.)],
        "kfgei_fx_hedge" | "kfgei_world_b" | "kfgei_alloc_vol" | "kfgei_alloc_rar" | "kfgei_alloc_corr" | "kfgei_alloc_combined" | "kfgei_rel_strength" => vec![],
        _ => return None,
    })
}

/// Weekly bars built from the daily rows: (last daily index, open, high, low, close).
struct Wk { i: Vec<usize>, first: Vec<usize>, h: Vec<f64>, l: Vec<f64>, c: Vec<f64> }
fn weekly(x: &Ctx, to: usize) -> Wk {
    let we: Vec<usize> = week_ends(x).into_iter().filter(|&i| i <= to).collect();
    let mut w = Wk { i: vec![], first: vec![], h: vec![], l: vec![], c: vec![] };
    let mut s = 0;
    for &e in &we {
        w.i.push(e);
        w.first.push(s);
        w.h.push((s..=e).map(|k| x.h[k]).fold(f64::MIN, f64::max));
        w.l.push((s..=e).map(|k| x.l[k]).fold(f64::MAX, f64::min));
        w.c.push(x.c[e]);
        s = e + 1;
    }
    w
}
fn hi(v: &[f64], a: usize, b: usize) -> f64 { v[a..b].iter().cloned().fold(f64::MIN, f64::max) }
fn lo(v: &[f64], a: usize, b: usize) -> f64 { v[a..b].iter().cloned().fold(f64::MAX, f64::min) }

fn open_trade(i: usize, px: f64, kind: &'static str) -> Trade {
    Trade { dir: 1., legs: vec![Leg { i, px, w: 1. }], exit_i: i, exit_px: px, reason: String::new(), entry_kind: kind, signal_i: i, open: false, exits: vec![] }
}
fn close_out(out: &mut Vec<Trade>, cur: &mut Option<Trade>, i: usize, px: f64, sig: usize, why: &str) {
    if let Some(mut t) = cur.take() { t.exit_i = i; t.exit_px = px; t.signal_i = sig; t.reason = why.into(); out.push(t); }
}
fn finish(x: &Ctx, mut out: Vec<Trade>, cur: Option<Trade>, to: usize) -> Vec<Trade> {
    if let Some(mut t) = cur { t.exit_i = to; t.exit_px = x.c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true; out.push(t); }
    out
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if from > to || to >= x.r.len() { return None; }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(0., |d| d.def));
    let u = |k: &str| g(k).round().max(0.) as usize;
    let mut out = vec![];
    let mut cur: Option<Trade> = None;
    match id {
        "kfgei_sys_a" | "kfgei_sys_b" => {
            let w = weekly(x, to);
            let n = u("weeks").max(1);
            let use_hl = g("hl") >= 0.5;
            let (bh, bl) = if use_hl { (&w.h, &w.l) } else { (&w.c, &w.c) };
            let dd = g("dd") / 100.;
            let peak_close = g("peak") >= 0.5;
            let mut peak = f64::NAN;
            for k in n..w.i.len() {
                let i = w.i[k];
                if cur.is_some() && id == "kfgei_sys_b" {
                    // highest price recorded during the trade: daily highs after the entry bar (or weekly closes)
                    let m = if peak_close { w.c[k] } else { (w.first[k]..=i).filter(|&d| d > cur.as_ref().unwrap().legs[0].i).map(|d| x.h[d]).fold(f64::MIN, f64::max) };
                    peak = peak.max(m);
                }
                if i < from { continue; }
                match cur {
                    Some(_) => {
                        if id == "kfgei_sys_a" && w.c[k] < lo(bl, k - n, k) {
                            close_out(&mut out, &mut cur, i, w.c[k], i, "weekly close below the 18-week low");
                        } else if id == "kfgei_sys_b" && w.c[k] < peak * (1. - dd) {
                            close_out(&mut out, &mut cur, i, w.c[k], i, "weekly close more than 7% below the trade's highest price");
                        }
                    }
                    None => if w.c[k] > hi(bh, k - n, k) && i < to {
                        cur = Some(open_trade(i, w.c[k], "close"));
                        peak = w.c[k];
                    },
                }
            }
        }
        "kfgei_sys_e" => {
            let w = weekly(x, to);
            let (m, n) = (u("ma").max(1), u("weeks").max(1));
            let ma = sma(&w.c, m);
            for k in 0..w.i.len().saturating_sub(1) {
                if !ma[k].is_finite() || k + 1 < n { continue; }
                let long = cur.is_some();
                let (buy, stop) = if !long && w.c[k] > ma[k] { (true, hi(&w.h, k + 1 - n, k + 1)) }
                    else if long && w.c[k] < ma[k] { (false, lo(&w.l, k + 1 - n, k + 1)) } else { continue };
                // the order works during next week's days
                for d in w.first[k + 1]..=w.i[k + 1] {
                    if buy && x.h[d] >= stop {
                        if d >= from && d < to { cur = Some(open_trade(d, x.o[d].max(stop), "stop")); }
                        break;
                    }
                    if !buy && x.l[d] <= stop {
                        close_out(&mut out, &mut cur, d, x.o[d].min(stop), d, "sell stop at the 5-week low (weekly close below the 40-week average)");
                        break;
                    }
                }
            }
        }
        "kfgei_uc" => {
            let w = weekly(x, to);
            let (loss, prof) = (g("loss") / 100., g("profit") / 100.);
            let daily = g("daily") >= 0.5;
            let is_check: Vec<bool> = if daily { vec![true; to + 1] } else {
                let mut v = vec![false; to + 1];
                for &i in &w.i { v[i] = true; }
                v
            };
            let year = |i: usize| &x.r[i].date[0..4];
            for i in from..=to {
                if i > 0 && year(i) != year(i - 1) && cur.is_none() && i < to {
                    cur = Some(open_trade(i, x.o[i], "open"));
                }
                if let Some(t) = &cur {
                    if is_check[i] {
                        let r = x.c[i] / t.legs[0].px - 1.;
                        if r <= -loss { close_out(&mut out, &mut cur, i, x.c[i], i, "3% loss"); }
                        else if r >= prof { close_out(&mut out, &mut cur, i, x.c[i], i, "15% profit"); }
                    }
                }
            }
        }
        _ => return None,
    }
    Some(finish(x, out, cur, to))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn rows(v: &[f64], start: i64) -> Vec<Row> {
        // consecutive weekdays from a Monday (days since epoch `start`)
        let mut out = vec![];
        let mut d = start;
        for &c in v {
            while (d + 3).rem_euclid(7) >= 5 { d += 1; }
            let (y, m, dd) = civil(d);
            out.push(Row { ticker: "T".into(), date: format!("{y:04}-{m:02}-{dd:02}"), open: c, high: c, low: c, close: c, volume: None });
            d += 1;
        }
        out
    }
    fn civil(z: i64) -> (i64, u32, u32) {
        let z = z + 719468;
        let era = z.div_euclid(146097);
        let doe = z - era * 146097;
        let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
        let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
        (yoe + era * 400 + (m <= 2) as i64, m, d)
    }
    #[test]
    fn system_b_exits_7pct_below_peak() {
        // 12 flat weeks at 100, a week at 110 (breakout), up to 120, then down to 111 (< 120*0.93 = 111.6)
        let mut v = vec![100.; 60];
        v.extend(vec![110.; 5]);
        v.extend(vec![120.; 5]);
        v.extend(vec![111.; 5]);
        let r = rows(&v, 10962); // 2000-01-03 (Monday)
        let x = Ctx::new(&r, &r, &[]);
        let ts = run("kfgei_sys_b", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert_eq!(ts.len(), 1);
        assert_eq!(ts[0].legs[0].px, 110.);
        assert_eq!(ts[0].exit_px, 111.);
        assert!(!ts[0].open);
    }
}
