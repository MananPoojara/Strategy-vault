//! Chuck LeBeau's System Traders Club (traderclub.com) manuals and bulletins:
//! "25 x 25" Bond System (LeBeau & Tan, 1998), "Crossbow" Swiss Franc System v1.0 (LeBeau & Elden, 1999),
//! "How To Design, Test, Evaluate and Implement Profitable Trading Systems" workbook (High Beta Stock
//! system, Chandelier / Yo-Yo exits), "Combining RSI and ADX" bulletin, and the "Exit Strategies for
//! Stocks and Futures" TradeStationWorld slides. Sources, oracle and verification: library/lebtc/.
//!
//! The 25x25 and Crossbow code reproduces the manuals' own trade lists exactly on the data files shipped
//! with them (see tests), but both are futures systems with dollar/tick rules, so they stay NeedsData here.
use crate::connors::{rsi, sma, true_range, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

const BOOK: &str = "lebtc";
const BOND: &str = "US 30-year T-bond futures (back-adjusted, day session). The rules are in bond points: entry 18/32 above the close and a $2,500 = 2.5-point money stop; an equity index has no equivalent of either";
const SWISS: &str = "Swiss franc futures. The $2,500 money stop (= 2.00 SF points at $1,250 per point) has no index equivalent, and it is the only loss exit";

pub fn catalogue() -> Vec<StrategyDef> {
    let make = |id, name, group, dir, status, rules, notes| StrategyDef {
        id, name, group, session: 0, dir, status, needs_volume: false, needs_vix: false,
        needs_series: &[], book: BOOK, rules, notes, claim: None,
    };
    let mut v = vec![
        make("lebtc_highbeta", "High Beta Stock Trading System", "Workbook · High Beta Stock system (2001)", "Long", Status::Live,
            &["F|\"ADX(14) > 15 and ADX > ADX[1] + 0.40 and +DI(14) > -DI(14)\" (EasyLanguage, workbook 'CODE FOR TRADESTATION'; prose: ADX \"equal to or greater than 15\", \"at least 0.40 higher than yesterday\", \"Plus DI ... above the Minus DI\")",
              "E|\"buy the next trading day at the open using a 'market' order\" (prose); code: buy (\"Long Entry\") at market. Only when flat.",
              "X|Loss exit: \"sellStop = open * (1 - riskInPercent/100)\" set on the entry bar (= entry price x 0.75); \"if close <= sellStop then exitLong at market\" -> exit at the next open. Prose: \"If the closing price shows a loss of 25% or more from our entry we will exit the following market day on the opening.\"",
              "X|Profit exit: \"barsSinceEntry >= profitTakingTime - 1\" (15 - 1 = 14 bars after the entry bar), \"openPositionProfit >= 2 * average(trueRange, 5)\" and \"RSI(close, 14) >= 75\" -> exit at the next open. Prose: \"at least fifteen market days\", \"two average true ranges above our entry\", \"14-day RSI reaches 75 or higher\".",
              "X|No other exit; a trade still open on the last bar is marked to market."],
            "Workbook pp. 'High Beta Stock Trading System' + 'CODE FOR TRADESTATION' (Fred Schutzman's EasyLanguage, Aug 2001). Where code and prose differ the code is used (ADX > 15 vs >= 15; RSI >= 75 vs 'closes above 75'; barsSinceEntry >= 14 = the 15th bar counting the entry bar). Open profit is per share (prose: 'two average true ranges above our entry'). ATR = simple 5-bar average of true range (code). Built for fast US stocks (CSCO, MSFT, AIG...); run here on the selected index or NSE stock (instrument swap). No results are printed in the workbook."),
        make("lebtc_25x25", "25 x 25 Bond System v2.0", "25x25 Bond System manual (1998)", "Long", Status::NeedsData(BOND),
            &["F|\"The 14-day +DI must be above the 14-day -DI. The 14-day ADX must be above 20. The 4-day RSI must be below 50.\" (code: ADX(14) > 20 and DMIPlus(14) > DMIMinus(14) and RSI(C,4) < 50)",
              "E|\"buy tomorrow only if and when prices rise 18 ticks (18/32) above today's close. Enter on a buy stop order.\" (code: buy C + 18 points stop)",
              "X|\"A stop order at $2,500 below the entry price\" ($2,500 = 2.5 bond points)",
              "X|\"a stop order at the lowest low of the last 25 days\" (code: Exitlong (\"L25\") Lowest(L, 25) stop)",
              "X|\"After 25 days (count entry day as day 1), change stop #2 ... to the lowest low of 2 days\" (code: if barssinceentry >= 24 then exitlong Lowest(L, 2) stop)",
              "X|\"after any close where the open profit is greater than 5 Average True Ranges the exit stop should be at the lowest low of 2 days. (Important: use 45 days to calculate the ATR)\" (code: if C > entryprice + 5*Avgtruerange(45) then exitlong lowest(l,2) stop). p.v1=1 drops this rule (= the v1.0 trade list printed in the manual).",
              "X|Whichever stop is closest to the market applies; stops fill at the stop or the open if it gaps through."],
            "Reproduced exactly on the manual's own Us.txt (1/4/88-1/16/98, MaxBarsBack 50, $100 per trade): v2.0 = 32 trades, $55,112.50, 72% (Table 1); v1.0 (p.v1=1) = the printed 29-trade list, $53,068.75, 76% (Table 2, 18 ticks). See tests and library/lebtc/verify."),
        make("lebtc_crossbow", "Crossbow Swiss Franc System v1.0", "Crossbow manual (1999)", "Long/Short", Status::NeedsData(SWISS),
            &["F|Long: \"The 20 day moving average must be above the 40 day moving average\", \"The close must be within 4 Average True Ranges of the highest high in the last 15 days\", \"at least 1.5 Average True Ranges below the highest high of the last five days\", \"The 14 day ADX must be less than 30\" (ATR = 15 bars)",
              "E|Long: \"buy tomorrow on a stop at the highest high of the last two days\" (code: Buy Highest(H,2) Stop)",
              "X|Long: \"If the highest close since entry is greater than the entry price plus 1.5 ATRs then we will exit on a stop at the highest close minus 1.5 ATRs\"",
              "F|Short: 20-day MA below 40-day MA; close within 2 ATRs of the 15-day lowest low; close at least 1 ATR above the 3-day lowest low; ADX(14) < 30",
              "E|Short: \"sell tomorrow on a stop at today's low\" (code: Sell Lowest(L,1) Stop)",
              "X|Short: \"If the lowest close since entry is less than the entry price minus 2 Average True Ranges then we will exit on a stop at 1 ATR above the lowest close\"",
              "X|\"Use a $2,500 money management stop for both long and short trades.\" \"the system will also exit if there is a trade signaled in the opposite direction\" (stop-and-reverse).",
              "X|Stop prices round to the tick (buy stops up, sell stops down); same-bar fills are taken in order of distance from the open (TradeStation bar-path assumption)."],
            "Reproduced exactly on the manual's own SF.LNG (10/17/88-9/30/99, MaxBarsBack 50, $100 per trade): 67/67 trades identical, net $101,300, 52 winners (78%), profit factor 4.53, largest win $10,337.50. See tests and library/lebtc/verify."),
        make("lebtc_rsi_adx", "Combining RSI and ADX", "Bulletin · Combining RSI and ADX", "Long",
            Status::NotTestable("the article leaves the loss exit to the reader (\"Use your favorite loss-limiting exit or you might want to exit when the price goes below the 20-day moving average or when the 20-day moving average turns down\") and the open-profit threshold open (\"pick some amount - maybe 4 ATRs\")"),
            &["F|\"The 20-bar moving average must be rising.\"",
              "E|\"If the ADX is rising (ADX today is 0.20 or more higher than yesterday) then buy if the 14 bar RSI is less than 85. If the ADX is not rising ... buy if the 14 bar RSI is less than 50.\"",
              "X|\"If the ADX is not rising ... sell if the 9-bar RSI is greater than 75.\" \"If the ADX is rising ... and the open profit is greater than (pick some amount - maybe 4 ATRs ...) then sell if the 9-bar RSI is greater than 75.\"",
              "X|\"You need some additional exit rule for the losing trades.\" (not given)"],
            "Not run: two exits are left to the trader. The author also says the parameters 'have not been tested or optimized'."),
    ];
    let exit_only = |id, name, rule: &'static [&'static str]| make(id, name, "Exit Strategies slides / Workbook · exits", "Long/Short",
        Status::NotTestable("exit technique only: the source gives no entry rule to attach it to"), rule,
        "Workbook Section 5 and the 'Exit Strategies for Stocks and Futures' slides. Random entries are suggested as a test bed but no entry is specified.");
    v.push(exit_only("lebtc_chandelier", "Chandelier Exit", &[
        "X|\"The exit stop is placed at a multiple of average true ranges from the highest high or highest close since the entry of the trade. ... it never moves downward.\" Examples: \"highest high since entry minus 3 ATR\", \"highest close since entry minus 2.5 ATR\" (workbook); slides: \"3 (?) ATRs\", tighten to 2 ATRs after 4 ATRs of profit"]));
    v.push(exit_only("lebtc_yoyo", "Yo-Yo Exit", &[
        "X|\"the ATR stop is always pegged to the most recent close\", \"typically set at only 1.5 to 2.0 ATRs from the most recent close\"; \"should never be our only loss protection\""]));
    v.push(exit_only("lebtc_modified_parabolic", "Modified Parabolic Exit", &["X|\"This trailing stop moves closer and closer to recent price as new highs are made.\" (formula not given)"]));
    v.push(make("lebtc_parabolic_adx", "Parabolic + ADX V-top signal", "Exit Strategies slides", "Long/Short",
        Status::NotTestable("judgement: \"be alert for a major change in direction\"; Parabolic settings and exits not given"),
        &["F|\"When the ADX rises above both the Plus DI and the Minus DI - be alert for a major change in direction\"", "E|\"Crossing the Parabolic signals that the direction has reversed\""], "Slides only; no complete rule set."));
    v.push(make("lebtc_random_chandelier", "Random entry + Chandelier exit", "Workbook · Chandelier Exit", "Long/Short",
        Status::NotTestable("random entries are not specified (generator, frequency, direction) and the ATR multiple is a range (2.5-4.0)"),
        &["E|\"you can enter futures markets at random and if you use this exit the results over time will be profitable\"", "X|Chandelier exit, \"best values ... between 2.5 and 4.0\""], ""));
    v
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "lebtc_highbeta" => ("lebtc_adx", "LeBeau ADX systems", "High Beta Stock (ADX rise)"),
        "lebtc_25x25" => ("lebtc_adx", "LeBeau ADX systems", "25x25 Bond (ADX dip-buy)"),
        "lebtc_crossbow" => ("lebtc_crossbow", "Crossbow Swiss Franc", "v1.0"),
        "lebtc_rsi_adx" => ("lebtc_adx", "LeBeau ADX systems", "RSI + ADX (incomplete)"),
        "lebtc_chandelier" | "lebtc_yoyo" | "lebtc_modified_parabolic" | "lebtc_parabolic_adx" | "lebtc_random_chandelier" =>
            ("lebtc_exits", "LeBeau exits", ""),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(match id {
        "lebtc_highbeta" => vec![
            p("adx_n", "ADX / DI length", 14., 7., 28., 1.), p("adx_min", "ADX must be above", 15., 0., 40., 1.),
            p("adx_inc", "ADX must rise by at least", 0.4, 0., 2., 0.1), p("risk", "Loss exit: % below entry (close basis)", 25., 5., 50., 1.),
            p("hold", "Profit taking after this many bars (counting the entry bar)", 15., 1., 40., 1.), p("atr_mult", "Minimum profit in ATRs", 2., 0., 6., 0.5),
            p("atr_n", "ATR length (simple average)", 5., 2., 20., 1.), p("rsi_n", "RSI length", 14., 2., 30., 1.), p("rsi_ob", "RSI overbought level", 75., 50., 95., 1.),
        ],
        "lebtc_25x25" => vec![
            p("offset", "Entry: points above the close (18/32)", 0.5625, 0., 2., 1. / 32.), p("mm", "Money stop in points ($2,500)", 2.5, 0.5, 5., 0.25),
            p("adx_min", "ADX(14) above", 20., 0., 40., 1.), p("rsi_max", "RSI(4) below", 50., 10., 90., 1.),
            p("ch", "Channel stop days", 25., 5., 50., 1.), p("switch", "Switch to 2-day low after bars since entry >=", 24., 1., 60., 1.),
            p("atr_mult", "Open profit in ATRs for the 2-day-low stop", 5., 1., 10., 0.5), p("atr_n", "ATR days (simple)", 45., 10., 90., 1.),
            p("v1", "1 = v1.0 (no 5-ATR rule; the printed trade list)", 0., 0., 1., 1.),
        ],
        "lebtc_crossbow" => vec![
            p("mm", "Money stop in points ($2,500 = 2.00 SF points)", 2., 0.5, 5., 0.25), p("tick", "Tick size", 0.01, 0.0001, 1., 0.0001),
            p("atr_n", "ATR days (simple)", 15., 5., 40., 1.), p("adx_max", "ADX(14) below", 30., 10., 60., 1.),
        ],
        _ => return None,
    })
}

// ---------------------------------------------------------------- helpers

/// Wilder directional system with both DIs: (+DI, -DI, ADX). Same arithmetic as connors::dmi, which
/// returns only +DI and ADX.
pub(crate) fn dmi3(x: &Ctx, n: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let len = x.c.len();
    let trs = true_range(x.r);
    let (mut pdm, mut mdm, mut tr) = (vec![f64::NAN; len], vec![f64::NAN; len], vec![f64::NAN; len]);
    for i in 1..len {
        let (up, dn) = (x.h[i] - x.h[i - 1], x.l[i - 1] - x.l[i]);
        pdm[i] = if up > dn && up > 0. { up } else { 0. };
        mdm[i] = if dn > up && dn > 0. { dn } else { 0. };
        tr[i] = trs[i];
    }
    let w = crate::connors::wilder;
    let (sp, sm, st) = (w(&pdm, n), w(&mdm, n), w(&tr, n));
    let (mut pdi, mut mdi, mut dx) = (vec![f64::NAN; len], vec![f64::NAN; len], vec![f64::NAN; len]);
    for i in 0..len {
        if st[i] > 0. {
            let (p, m) = (100. * sp[i] / st[i], 100. * sm[i] / st[i]);
            pdi[i] = p;
            mdi[i] = m;
            dx[i] = if p + m > 0. { 100. * (p - m).abs() / (p + m) } else { 0. };
        }
    }
    (pdi, mdi, w(&dx, n))
}

/// TradeStation AvgTrueRange(n) = simple average of TrueRange.
pub(crate) fn atr_sma(x: &Ctx, n: usize) -> Vec<f64> {
    sma(&true_range(x.r), n)
}

fn gt(a: f64, b: f64) -> bool { a > b } // NaN compares false
fn lowest(v: &[f64], k: usize, n: usize) -> f64 { v[k + 1 - n.min(k + 1)..=k].iter().copied().fold(f64::INFINITY, f64::min) }
fn highest(v: &[f64], k: usize, n: usize) -> f64 { v[k + 1 - n.min(k + 1)..=k].iter().copied().fold(f64::NEG_INFINITY, f64::max) }

fn open_trade(dir: f64, i: usize, px: f64, kind: &'static str) -> Trade {
    Trade { dir, legs: vec![Leg { i, px, w: 1. }], exit_i: i, exit_px: px, reason: String::new(), entry_kind: kind, signal_i: i, open: false, exits: vec![] }
}
fn close_trade(mut t: Trade, i: usize, px: f64, signal_i: usize, reason: &str) -> Trade {
    t.exit_i = i;
    t.exit_px = px;
    t.signal_i = signal_i;
    t.reason = reason.into();
    t
}
fn finish(out: &mut Vec<Trade>, cur: Option<Trade>, x: &Ctx, to: usize) {
    if let Some(t) = cur {
        let mut t = close_trade(t, to, x.c[to], to, "open");
        t.open = true;
        out.push(t);
    }
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if from > to || to >= x.r.len() {
        return None;
    }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap().def);
    let u = |k: &str| g(k).round().max(1.) as usize;
    let mut out = vec![];
    match id {
        "lebtc_highbeta" => {
            let (pdi, mdi, adx) = dmi3(x, u("adx_n"));
            let r = rsi(&x.c, u("rsi_n"));
            let atr = atr_sma(x, u("atr_n"));
            let (adx_min, inc, risk, hold, am, ob) = (g("adx_min"), g("adx_inc"), g("risk") / 100., u("hold"), g("atr_mult"), g("rsi_ob"));
            let mut cur: Option<(Trade, f64)> = None;
            for i in from.max(1)..=to {
                let k = i - 1;
                if let Some((t, ss)) = cur.as_ref() {
                    let e = t.legs[0].i;
                    let px = t.legs[0].px;
                    let loss = k >= e && x.c[k] <= *ss;
                    let profit = k >= e && k - e + 1 >= hold && x.c[k] - px >= am * atr[k] && r[k] >= ob;
                    if loss || profit {
                        let why = if loss { "Loss exit: close 25% below entry · next open" } else { "Profit exit: RSI overbought after 15 bars and 2 ATRs · next open" };
                        out.push(close_trade(cur.take().unwrap().0, i, x.o[i], k, why));
                    }
                    continue;
                }
                if gt(adx[k], adx_min) && gt(adx[k], adx[k - 1] + inc) && gt(pdi[k], mdi[k]) {
                    let mut t = open_trade(1., i, x.o[i], "open");
                    t.signal_i = k;
                    cur = Some((t, x.o[i] * (1. - risk)));
                }
            }
            finish(&mut out, cur.map(|c| c.0), x, to);
        }
        "lebtc_25x25" => {
            let (pdi, mdi, adx) = dmi3(x, 14);
            let r4 = rsi(&x.c, 4);
            let atr = atr_sma(x, u("atr_n"));
            let (off, mm, amin, rmax, ch, sw, am, v1) = (g("offset"), g("mm"), g("adx_min"), g("rsi_max"), u("ch"), u("switch"), g("atr_mult"), g("v1") >= 0.5);
            let mut cur: Option<Trade> = None;
            for i in from.max(1)..=to {
                let k = i - 1;
                let mut exited = false;
                if let Some(t) = cur.as_ref() {
                    let (e, epx) = (t.legs[0].i, t.legs[0].px);
                    let mut stop = epx - mm;
                    let mut s = lowest(&x.l, k, ch);
                    if k - e >= sw || (!v1 && x.c[k] > epx + am * atr[k]) {
                        s = s.max(lowest(&x.l, k, 2));
                    }
                    stop = stop.max(s);
                    if x.l[i] <= stop {
                        let why = if stop == epx - mm { "$2,500 money stop" } else { "Channel / 2-day low stop" };
                        out.push(close_trade(cur.take().unwrap(), i, x.o[i].min(stop), i, why));
                        exited = true;
                    }
                }
                // the buy-stop order exists whenever the setup was true at the prior close; it can fill after a same-bar exit
                if cur.is_none() && gt(adx[k], amin) && gt(pdi[k], mdi[k]) && r4[k] < rmax {
                    let lvl = x.c[k] + off;
                    if x.h[i] >= lvl {
                        let px = x.o[i].max(lvl);
                        let mut t = open_trade(1., i, px, "stop");
                        t.signal_i = k;
                        if !exited && x.l[i] <= px - mm {
                            out.push(close_trade(t, i, px - mm, i, "$2,500 money stop"));
                        } else {
                            cur = Some(t);
                        }
                    }
                }
            }
            finish(&mut out, cur, x, to);
        }
        "lebtc_crossbow" => {
            let atr = atr_sma(x, u("atr_n"));
            let adx = dmi3(x, 14).2;
            let (m20, m40) = (sma(&x.c, 20), sma(&x.c, 40));
            let (mm, tick, amax) = (g("mm"), g("tick"), g("adx_max"));
            let up = |v: f64| (v / tick - 1e-9).ceil() * tick;
            let dn = |v: f64| (v / tick + 1e-9).floor() * tick;
            let mut cur: Option<(Trade, f64)> = None; // trade, best close since entry
            for i in from.max(41)..=to {
                let k = i - 1;
                let a = atr[k];
                let dir_now = cur.as_ref().map_or(0., |c| c.0.dir);
                let long_set = x.c[k] > highest(&x.h, k, 15) - 4. * a && x.c[k] < highest(&x.h, k, 5) - 1.5 * a && gt(m20[k], m40[k]) && adx[k] < amax;
                let short_set = adx[k] < amax && x.c[k] < lowest(&x.l, k, 15) + 2. * a && x.c[k] > lowest(&x.l, k, 3) + a && m20[k] < m40[k];
                let mut ev: Vec<(char, f64)> = vec![];
                if let Some((t, best)) = cur.as_ref() {
                    let epx = t.legs[0].px;
                    let mut stop = epx - t.dir * mm;
                    if t.dir > 0. && *best > epx + 1.5 * a { stop = stop.max(dn(best - 1.5 * a)); }
                    if t.dir < 0. && *best < epx - 2. * a { stop = stop.min(up(best + a)); }
                    if t.dir > 0. && x.l[i] <= stop { ev.push(('x', x.o[i].min(stop))); }
                    if t.dir < 0. && x.h[i] >= stop { ev.push(('x', x.o[i].max(stop))); }
                }
                if long_set && dir_now <= 0. {
                    let lvl = highest(&x.h, k, 2);
                    if x.h[i] >= lvl { ev.push(('b', x.o[i].max(lvl))); }
                }
                if short_set && dir_now >= 0. {
                    let lvl = x.l[k];
                    if x.l[i] <= lvl { ev.push(('s', x.o[i].min(lvl))); }
                }
                ev.sort_by(|a, b| (a.1 - x.o[i]).abs().total_cmp(&(b.1 - x.o[i]).abs()));
                let mut changed = false;
                for (kind, px) in ev {
                    if kind == 'x' {
                        if !changed && cur.is_some() {
                            out.push(close_trade(cur.take().unwrap().0, i, px, i, "$2,500 money stop / ATR trailing stop"));
                            changed = true;
                        }
                        continue;
                    }
                    let d = if kind == 'b' { 1. } else { -1. };
                    if cur.as_ref().is_some_and(|c| c.0.dir == d) { continue; }
                    if let Some((t, _)) = cur.take() {
                        out.push(close_trade(t, i, px, i, "Reversed by opposite entry"));
                    }
                    let mut t = open_trade(d, i, px, "stop");
                    t.signal_i = k;
                    cur = Some((t, x.c[i]));
                    changed = true;
                }
                if let Some((t, best)) = cur.as_mut() {
                    *best = if t.dir > 0. { best.max(x.c[i]) } else { best.min(x.c[i]) };
                }
            }
            finish(&mut out, cur.map(|c| c.0), x, to);
        }
        _ => return None,
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;

    fn load(name: &str) -> Option<Vec<Row>> {
        let dir = std::env::var("LEBTC_DATA").unwrap_or("/home/server/.local/share/strategy-vault/work/lebtc/data".into());
        let txt = std::fs::read_to_string(format!("{dir}/{name}")).ok()?;
        Some(txt.lines().skip(1).map(|l| {
            let f: Vec<&str> = l.split(',').collect();
            let n = |k: usize| f[k].parse::<f64>().unwrap();
            Row { ticker: "F".into(), date: f[0].into(), open: n(1), high: n(2), low: n(3), close: n(4), volume: None }
        }).collect())
    }
    fn window(r: &[Row], a: &str, b: &str) -> Vec<Row> {
        r.iter().filter(|x| x.date.as_str() >= a && x.date.as_str() <= b).cloned().collect()
    }
    fn usd(ts: &[Trade], per_point: f64) -> (usize, f64, usize) {
        let closed: Vec<f64> = ts.iter().filter(|t| !t.open).map(|t| (t.exit_px - t.legs[0].px) * t.dir * per_point - 100.).collect();
        (closed.len(), closed.iter().sum(), closed.iter().filter(|v| **v > 0.).count())
    }

    #[test]
    fn x25_reproduces_manual_tables() {
        let Some(all) = load("US_bond_25x25.csv") else { return };
        let r = window(&all, "1988-01-04", "1998-01-16");
        let x = Ctx::new(&r, &r, &[]);
        let ts = run("lebtc_25x25", &x, 0, r.len() - 1, &Params::new()).unwrap();
        let (n, net, w) = usd(&ts, 1000.);
        assert_eq!((n, w), (32, 23)); // Table 1 v2.0: 32 trades, 23 winners (72%)
        assert!((net - 55112.5).abs() < 1e-6, "{net}");
        let v1 = run("lebtc_25x25", &x, 0, r.len() - 1, &Params::from([("v1".into(), 1.)])).unwrap();
        let (n, net, w) = usd(&v1, 1000.);
        assert_eq!((n, w), (29, 22)); // printed trade list / Table 2 at 18 ticks
        assert!((net - 53068.75).abs() < 1e-6, "{net}");
        assert_eq!(r[v1[0].legs[0].i].date, "1988-06-22");
    }

    #[test]
    fn crossbow_reproduces_manual() {
        let Some(all) = load("SF_crossbow.csv") else { return };
        let r = window(&all, "1988-10-17", "1999-09-30");
        let x = Ctx::new(&r, &r, &[]);
        let ts = run("lebtc_crossbow", &x, 0, r.len() - 1, &Params::new()).unwrap();
        let (n, net, w) = usd(&ts, 1250.);
        assert_eq!((n, w), (67, 52));
        assert!((net - 101300.).abs() < 1e-3, "{net}");
    }

    #[test]
    fn highbeta_loss_exit_at_next_open() {
        let mut r: Vec<Row> = (0..80).map(|i| {
            let c = if i < 40 { 100. + (i % 2) as f64 } else { 100. + 2. * (i - 39) as f64 }; Row { ticker: "T".into(), date: format!("{i:05}"), open: c, high: c + 1., low: c - 1., close: c, volume: None }
        }).collect();
        let x0 = Ctx::new(&r, &r, &[]);
        let first = run("lebtc_highbeta", &x0, 0, 79, &Params::new()).unwrap();
        let e = first[0].legs[0].i;
        r[e + 3].close = r[e].open * 0.74; // close 26% below the entry
        let x = Ctx::new(&r, &r, &[]);
        let ts = run("lebtc_highbeta", &x, 0, 79, &Params::new()).unwrap();
        assert_eq!(ts[0].exit_i, e + 4);
        assert_eq!(ts[0].exit_px, r[e + 4].open);
    }
}
