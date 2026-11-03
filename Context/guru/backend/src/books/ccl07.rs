//! Larry Connors / TradingMarkets, "Chairman's Club December 2007" (Las Vegas, Dec 1-2 2007; research share Trading
//! Knowledge II/@TradingMarkets.com & TradersLibrary.com/Chairman's Club December-2007 (tradingmarkets.com)/CC 2007 (Dec
//! 1,2) - Las Vegas Manual.pdf, image slides; refs sN = printed slide number = PDF page). Book id `ccl07`.
//! Live: RSI 10 Scale-In (s78-80) and 10 Worst Performers (s214-216). Not coded: the Short Selling Methodology (s197-201)
//! is the live STC IX rule `h10` (same LF > 20, RSI(2) > 90, HV(100) > 40, ADX(10) > 40, close > 200-day MA, 10-day
//! high, limit close + 0.4 x ATR(10), cover next open after RSI(2) < 40) - a duplicate, so no copy here.
//! Lane K3 (titles loop, 30 Oct 2026); independent check authors/ccl07/admission/k3_ref.py (header).
//! Lane K5 (31 Oct 2026) adds the AmiBroker class project (s97-102) and Curt Carlson's 'A Breakout Study' (s229-237);
//! excerpts authors/titles-1030/R3w/ccl07_class_project.txt, ccl07_carlson_breakout.txt; independent check
//! authors/ccl07/admission/k5/k5_ref.py (header).
use crate::connors::{hv_ddof, sma, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

/// Admitted ids (authors/ccl07/admission/k3_ref.py (header)).
pub const REVIEWED: &[&str] = &["ccl07_rsi10_scale", "ccl07_worst10", "ccl07_class_project", "ccl07_carlson_bo"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE_R: &str = "[Declared readings] The slide trades S&P e-minis with SPY as the filter; here the traded series is its own filter (SPY, the S&P 500 and other indices, ETFs and stocks are a declared application). RSI(2) is the vault's Connors RSI (Wilder), the 200-day MA a simple average of closes. Units 2/3/5 = 20%/30%/50% of a full position. [Reading, open question] The slide does not say when the adds are bought: here at the close that is lower than the earlier entry price (the entry day's own close counts); on a close where the exit rule fires, nothing is added. One position at a time; a position still open at the window end is marked at the last close.";
const NOTE_W: &str = "[Declared readings] A stock method: the slide's results (s216) are a portfolio of US stocks and the rules do not explain the '10' in the title (open question: ten slots, or a pick of the ten worst). Here it runs per instrument on actual US and NSE stocks; indices and ETFs are a declared application. 'LF 3' is printed without an operator and is read as LF > 3 like the other Chairman's Club decks; LF = the vault Liquidity Filter (TradingMarkets 1/19/10: 1,000,000 x SMA20(close x volume) / SMA65(benchmark close x volume), benchmark S&P 500 / NIFTY 50). 'Trading for 1 year' and 'worst performance in the past year' = today's 5-day % change (close vs the close 5 days earlier) is the lowest (ties count) of the last 252 daily 5-day changes, today included, all of which must exist. The exit is checked from the entry day's close. One position at a time.";

const NOTE_P: &str = "[Declared readings] A stock method (the class project of the AmiBroker session). Setup on the close: close above the 200-day simple average, RSI(2) (Wilder) under 10, HV(100) above 30, three lower closes in a row, LF above 90; buy at the next open. HV(100) = the slide's AmiBroker code StDev(ln(C/C1),100) x sqrt(252) x 100, with AmiBroker's population standard deviation. [Reading, open question] LF: the slide's template prints LF = 1000 x MA(C x V,20)/sp500dolvol65 with LF > 3, but the project says LF > 90 and 'sp500dolvol65' is not defined; here LF = the vault Liquidity Filter (TradingMarkets 1/19/10: 1,000,000 x SMA20(close x volume) / SMA65(benchmark close x volume), benchmark S&P 500 / NIFTY 50) and LF > 90 is the default (p.lf = 3 is the option). The default exit is Sell (1), the one coded on slide 102: after RSI(2) closes above 70, sell at the next open. Options (p.sell) = the slide-99 menu: 2 = 2.5% profit target and 5% trailing stop [reading: the target is a limit at entry x 1.025 filled at the target or a better open; the trailing stop sits 5% under the highest close since entry and fills at the stop or a worse open; both work from the day after entry; if one day could touch both, the stop is taken]; 3 = RSI(2) > 70 or after 10 days [reading: the 10th close in the trade, entry day included, then out at the next open]; 4 = after RSI(2) has closed above 70, out at the next open after a close under 65. The exit is checked from the entry day's close. One position at a time; setups while holding are ignored. PositionScore (HV100 ranking of a portfolio) is not modelled: one instrument per run. No results are printed. Indices and ETFs with volume are a declared application.";
const NOTE_C: &str = "[Declared readings] Curt Carlson's study (S&P 500 stocks, AmiBroker portfolio, 3/27/2000-10/12/2007). Breakout = a close at a 35-day high (the highest close of the last 35 days, today included; ties count) while the 15-day simple average is above the 160-day; Reference Volume = the 150-day average volume on the day before the breakout, above 100,000 shares. Entry and exit at the signal close (the study gives no fill). Exit = the 14/125 DMA cross: the 14-day average closes below the 125-day after being at or above it the day before (checked from the day after entry). One position at a time. Instruments without any volume (most indices) run without the volume filter; a series with volume but a gap in the last 150 days has no Reference Volume, so no breakout counts then. The 100,000-share floor is a share count on US and NSE stocks alike (declared application). Option p.ft = 1: Follow-through entry = the first close at a new high since the breakout (above every close from the breakout day on) at least 10 trading days after the breakout [readings: the breakout must pass the filters; a 14/125 cross before the follow-through cancels it; later breakouts while waiting are ignored]. Not coded: the V_BO > 16 and V_FT > 3.5 / > 7.5 volume variants (the volume-ratio units are not defined). Printed CAR (portfolio, not comparable to one instrument): breakout any volume 17.05%, follow-through any volume 23.37%, S&P 500 0.33%.";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id: &'static str, name: &'static str, vol: bool, rules: &'static [&'static str], notes: &'static str| StrategyDef { id, name, group: "Connors Chairman's Club Dec 2007",
        session: 0, dir: "Long", status: Status::Live, needs_volume: vol, needs_vix: false, needs_series: &[], book: "ccl07", rules, notes, claim: None };
    vec![
        d("ccl07_rsi10_scale", "RSI 10 Scale-In: above the 200-day, buy 2-3-5 units after RSI(2) closes under 10, out next open after RSI(2) > 75", false, &[
            "F|\"1. SPY is above the 200-day MA.\" (s79)",
            "E|\"2. Buy 2 e-minis on the next days open if the 2-period RSI closes under 10.\" (s79)",
            "A|\"3. Buy 3 more e-minis if prices closes lower than entry price anytime you're in the initial position.\" (s79) [reading] bought at that close",
            "A|\"4. Buy 5 more e-mini's if prices closes lower than second entry price anytime you're in the second position.\" (s79) [reading] bought at that close",
            "X|\"5. Exit RSI 75 on the next days open.\" (s79) = after RSI(2) closes above 75, sell everything at the next open"], NOTE_R),
        d("ccl07_worst10", "10 Worst Performers: above the 200-day, 5-day change the worst of the past year, buy next open, out next open after a close above the 5-day MA", true, &[
            "F|\"1. Close is above the 200MA\" (s215)",
            "F|\"2. LF 3\" (s215) [reading] LF > 3",
            "F|\"3. Stock has to have been trading for 1 Year\" (s215)",
            "E|\"4. The 5 day performance of the stock is its worst performance in the past year.\" (s215)",
            "E|\"5. Buy at next open\" (s215)",
            "X|\"6. Sell when the close is above the 5 ma (next days open).\" (s215)"], NOTE_W),
        d("ccl07_class_project", "AmiBroker class project: above the 200-day, RSI(2) under 10, HV(100) over 30, three down closes, LF over 90; buy next open, out next open after RSI(2) > 70", true, &[
            "F|\"Close > 200MA\" (s98)",
            "F|\"100 period HV > 30\" (s98); \"HV100 = StDev(log(C/Ref(C,-1)),100)*(252^.5)*100\" (s97)",
            "F|\"LF > 90\" (s98) [reading] the vault Liquidity Filter; the s97 template's 'LF > 3' is an option",
            "E|\"2 Period RSI < 10\", \"3 Down Days\" (s98) = \"C < Ref(C, -1) AND Ref(C, -1) < Ref(C, -2) AND Ref(C, -2) < Ref(C, -3)\" (s100)",
            "E|\"Buy = Ref(Setup, -1) ... BuyPrice = O\" (s97) = buy at the next open",
            "X|\"(1) 2 period RSI > 70\" (s99); \"SellSignal = RSI(2) > 70; Sell = Ref(SellSignal, -1)\", \"SellPrice = O\" (s97, s102) = sell at the next open",
            "X|Options (s99): \"(2) 2.5% profit target and 5% trailing stop\", \"(3) 2 period RSI > 70 or after 10 days\", \"(4) After 2 period RSI >70, then exit after RSI < 65\""], NOTE_P),
        d("ccl07_carlson_bo", "A Breakout Study (Curt Carlson): close at a 35-day high with the 15-day over the 160-day average and volume over 100,000 shares, out on the 14/125 cross", false, &[
            "E|\"Breakout: Close at 35 day high\" (s231)",
            "F|\"Filters: 15DMA > 160DMA at breakout; Reference Volume > 100,000 Shares\"; \"Reference Volume: 150 DMA of Volume on the day before the breakout\" (s231)",
            "X|\"Exit: 14/125 DMA cross\" (s231)",
            "E|Option: \"Follow-through: Closes at new high since breakout at least 10 days after the breakout\" (s231)"], NOTE_C),
    ]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id {
        "ccl07_rsi10_scale" => Some(("ccl07_rsi10_scale", "Chairman's Club 2007 RSI 10 Scale-In", "2-3-5 units")),
        "ccl07_worst10" => Some(("ccl07_worst10", "Chairman's Club 2007 10 Worst Performers", "5-day change, 1 year")),
        "ccl07_class_project" => Some(("ccl07_class_project", "Chairman's Club 2007 AmiBroker class project", "RSI(2) < 10, 3 down days, HV, LF")),
        "ccl07_carlson_bo" => Some(("ccl07_carlson_bo", "Chairman's Club 2007 Carlson breakout study", "35-day high, 14/125 exit")),
        _ => None,
    }
}
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    match id {
        "ccl07_rsi10_scale" => Some(vec![p("trend", "Trend average, days (slide 200)", 200., 10., 300., 10.), p("rsi_lt", "Entry: RSI(2) closes under (slide 10)", 10., 1., 50., 1.),
            p("exit_rsi", "Exit: RSI(2) closes above (slide 75)", 75., 50., 95., 5.)]),
        "ccl07_worst10" => Some(vec![p("trend", "Trend average, days (slide 200)", 200., 10., 300., 10.), p("lf", "Liquidity Filter above (slide 3)", 3., 0., 40., 1.),
            p("perf", "Performance period, days (slide 5)", 5., 1., 20., 1.), p("year", "Past year, trading days (reading 252)", 252., 20., 504., 1.), p("ma", "Exit average, days (slide 5)", 5., 2., 20., 1.)]),
        "ccl07_class_project" => Some(vec![p("trend", "Trend average, days (slide 200)", 200., 10., 300., 10.), p("rsi_lt", "Entry: RSI(2) under (slide 10)", 10., 1., 50., 1.),
            p("hv", "HV(100) above (slide 30)", 30., 0., 100., 5.), p("down", "Down closes in a row (slide 3)", 3., 1., 6., 1.), p("lf", "Liquidity Filter above (project 90; template 3)", 90., 0., 200., 1.),
            p("sell", "Exit: 1 = RSI(2) > 70 (s102), 2 = 2.5% target + 5% trailing stop, 3 = RSI > 70 or 10 days, 4 = after RSI > 70, RSI < 65", 1., 1., 4., 1.),
            p("exit_rsi", "Exit: RSI(2) closes above (slide 70)", 70., 50., 95., 5.), p("rsi_back", "Exit 4: then RSI(2) closes under (slide 65)", 65., 30., 90., 5.),
            p("target", "Exit 2: profit target % (slide 2.5)", 2.5, 0.5, 20., 0.5), p("trail", "Exit 2: trailing stop % (slide 5)", 5., 0.5, 20., 0.5), p("days", "Exit 3: days in the trade (slide 10)", 10., 1., 50., 1.)]),
        "ccl07_carlson_bo" => Some(vec![p("hi", "Breakout: close at an N-day high (slide 35)", 35., 5., 200., 1.), p("fast", "Filter: fast average (slide 15)", 15., 2., 100., 1.),
            p("slow", "Filter: slow average (slide 160)", 160., 10., 400., 10.), p("vol_n", "Reference Volume average, days (slide 150)", 150., 10., 300., 10.),
            p("min_vol", "Reference Volume above, shares (slide 100,000)", 100000., 0., 10000000., 10000.), p("xf", "Exit: fast average (slide 14)", 14., 2., 100., 1.),
            p("xs", "Exit: slow average (slide 125)", 125., 10., 400., 5.), p("ft", "Entry: 0 = breakout close, 1 = follow-through (slide 231)", 0., 0., 1., 1.),
            p("ft_days", "Follow-through: at least N days after the breakout (slide 10)", 10., 1., 50., 1.)]),
        _ => None,
    }
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let s = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| s.iter().find(|d| d.key == k).unwrap().def);
    let u = |k: &str| g(k).round().max(1.) as usize;
    match id { "ccl07_class_project" => return Some(class_project(x, from, to, &g)), "ccl07_carlson_bo" => return Some(carlson(x, from, to, &g)), _ => {} }
    let (o, c) = (&x.o, &x.c);
    let tr = x.sma_n(u("trend"));
    let n = c.len();
    // entry signal on bar i's close (bought at the next open); exit signal on a close (sold at the next open)
    let (entry, exit, w, why): (Vec<bool>, Vec<bool>, &[f64], String) = if id == "ccl07_rsi10_scale" {
        let (rs, lt, ex) = (x.rsi_n(2), g("rsi_lt"), g("exit_rsi"));
        ((0..n).map(|i| c[i] > tr[i] && rs[i] < lt).collect(), (0..n).map(|i| rs[i] > ex).collect(), &[0.2, 0.3, 0.5], format!("RSI(2) > {ex}, sold next open"))
    } else {
        let (lf, k, yr, ma) = (g("lf"), u("perf"), u("year"), x.sma_n(u("ma")));
        let r: Vec<f64> = (0..n).map(|i| if i >= k { c[i] / c[i - k] - 1. } else { f64::NAN }).collect();
        let worst = |i: usize| i + 1 >= yr + k && r[i + 1 - yr..=i].iter().all(|&v| r[i] <= v);
        ((0..n).map(|i| c[i] > tr[i] && x.lf[i] > lf && worst(i)).collect(), (0..n).map(|i| c[i] > ma[i]).collect(), &[1.0], format!("Close > {}-day MA, sold next open", u("ma")))
    };
    let (mut out, mut cur, mut pend_in) = (vec![], None::<Trade>, false);
    for i in from..=to {
        if pend_in {
            pend_in = false;
            cur = Some(Trade { dir: 1., legs: vec![Leg { i, px: o[i], w: w[0] }], exit_i: i, exit_px: o[i], reason: String::new(), entry_kind: "open", signal_i: i, open: false, exits: vec![] });
        }
        if let Some(t) = cur.as_mut() {
            if exit[i] {
                if i < to { t.exit_i = i + 1; t.exit_px = o[i + 1]; t.reason = why.clone(); t.signal_i = i; out.push(cur.take().unwrap()); }
            } else if t.legs.len() < w.len() && c[i] < t.legs.last().unwrap().px {
                let k = t.legs.len();
                t.legs.push(Leg { i, px: c[i], w: w[k] });
            }
            continue;
        }
        if i < to && entry[i] { pend_in = true; }
    }
    if let Some(mut t) = cur { t.exit_i = to; t.exit_px = c[to]; t.reason = "open".into(); t.signal_i = to; t.open = true; out.push(t); }
    Some(out)
}

fn tr1(e: usize, ep: f64, kind: &'static str) -> Trade {
    Trade { dir: 1., legs: vec![Leg { i: e, px: ep, w: 1. }], exit_i: e, exit_px: ep, reason: String::new(), entry_kind: kind, signal_i: e, open: false, exits: vec![] }
}
fn shut(mut t: Trade, i: usize, px: f64, sig: usize, why: String, open: bool) -> Trade { t.exit_i = i; t.exit_px = px; t.signal_i = sig; t.reason = why; t.open = open; t }

/// Slides 97-102: setup on the close, buy at the next open; exits per the slide-99 menu (p.sell).
fn class_project(x: &Ctx, from: usize, to: usize, g: &dyn Fn(&str) -> f64) -> Vec<Trade> {
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let (tr, rs, hv) = (x.sma_n(g("trend").round().max(1.) as usize), x.rsi_n(2), hv_ddof(c, 100, 0.));
    let (lt, hvmin, down, lf, ex, back) = (g("rsi_lt"), g("hv"), g("down").round().max(1.) as usize, g("lf"), g("exit_rsi"), g("rsi_back"));
    let (sell, tgt, trail, days) = (g("sell").round() as i64, 1. + g("target") / 100., 1. - g("trail") / 100., g("days").round().max(1.) as usize);
    let setup = |i: usize| i >= down && c[i] > tr[i] && rs[i] < lt && hv[i] > hvmin && x.lf[i] > lf && (i + 1 - down..=i).all(|k| c[k] < c[k - 1]);
    let (mut out, mut cur, mut pend, mut armed, mut best) = (vec![], None::<Trade>, false, false, 0f64);
    for i in from..=to {
        if pend {
            pend = false;
            cur = Some(tr1(i, o[i], "open"));
            armed = false;
            best = f64::NAN;
        }
        if let Some(t) = cur.take() {
            let (e, ep) = (t.legs[0].i, t.legs[0].px);
            // option 2: intraday target / trailing stop from the day after entry (stop level from closes before today)
            let hit = if sell == 2 && i > e {
                let (lim, st) = (ep * tgt, best * trail);
                if o[i] <= st { Some((o[i], "trailing stop (gap)")) } else if o[i] >= lim { Some((o[i], "profit target (gap)")) }
                else if l[i] <= st { Some((st, "trailing stop")) } else if h[i] >= lim { Some((lim, "profit target")) } else { None }
            } else { None };
            if let Some((px, why)) = hit {
                out.push(shut(t, i, px, i, why.into(), false)); // flat again: today's close may set up a new trade
            } else {
                best = if best.is_nan() { c[i] } else { best.max(c[i]) };
                if rs[i] > ex { armed = true; }
                let fire = match sell { 1 => rs[i] > ex, 3 => rs[i] > ex || i + 1 - e >= days, 4 => armed && rs[i] < back, _ => false };
                if fire && i < to {
                    let why = match sell { 4 => format!("RSI(2) < {back} after > {ex}, sold next open"), 3 if !(rs[i] > ex) => format!("{days} days, sold next open"), _ => format!("RSI(2) > {ex}, sold next open") };
                    out.push(shut(t, i + 1, o[i + 1], i, why, false));
                } else { cur = Some(t); }
                continue; // still holding at this close (a next-open exit included): no new setup
            }
        }
        if i < to && setup(i) { pend = true; }
    }
    if let Some(t) = cur { out.push(shut(t, to, c[to], to, "open".into(), true)); }
    out
}

/// Slides 229-237: breakout (or follow-through) entry at the close, out at the close of the 14/125 cross.
fn carlson(x: &Ctx, from: usize, to: usize, g: &dyn Fn(&str) -> f64) -> Vec<Trade> {
    let u = |k: &str| g(k).round().max(1.) as usize;
    let c = &x.c;
    let (hi, fa, sl, xf, xs) = (u("hi"), x.sma_n(u("fast")), x.sma_n(u("slow")), x.sma_n(u("xf")), x.sma_n(u("xs")));
    let vol: Vec<f64> = x.r.iter().map(|r| r.volume.filter(|v| *v > 0.).unwrap_or(f64::NAN)).collect();
    let rv = sma(&vol, u("vol_n"));
    let (minv, ft, ftd) = (g("min_vol"), g("ft") >= 0.5, u("ft_days"));
    let breakout = |i: usize| i + 1 >= hi && i >= 1 && c[i + 1 - hi..=i].iter().all(|&v| c[i] >= v) && fa[i] > sl[i] && (!x.has_volume || rv[i - 1] > minv);
    let cross = |i: usize| i >= 1 && xf[i] < xs[i] && xf[i - 1] >= xs[i - 1];
    let (mut out, mut cur, mut wait) = (vec![], None::<Trade>, None::<(usize, f64)>); // wait = (breakout bar, highest close since)
    for i in from..=to {
        if let Some(t) = cur.take() {
            if cross(i) { out.push(shut(t, i, c[i], i, format!("{}/{} DMA cross", u("xf"), u("xs")), false)); } else { cur = Some(t); continue; }
        }
        if i == to { break; }
        if !ft {
            if breakout(i) { cur = Some(tr1(i, c[i], "close")); }
            continue;
        }
        if let Some((b, hh)) = wait {
            if cross(i) { wait = None; }
            else if i >= b + ftd && c[i] > hh { cur = Some(tr1(i, c[i], "close")); wait = None; continue; }
            else { wait = Some((b, hh.max(c[i]))); continue; }
        }
        if breakout(i) { wait = Some((i, c[i])); }
    }
    if let Some(t) = cur { out.push(shut(t, to, c[to], to, "open".into(), true)); }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn rsi10_scale_adds_on_lower_closes_and_exits_next_open() {
        // 220 rising bars (above the 200-day), then a slide (RSI(2) < 10), more lower closes, then a rally (RSI(2) > 75)
        let mut cl: Vec<f64> = (0..220).map(|i| 100. + i as f64).collect();
        cl.extend([310., 300., 299., 298., 330., 340.]);
        let r: Vec<Row> = cl.iter().enumerate().map(|(i, &c)| Row { ticker: "T".into(), date: format!("2001-01-{:02}", i % 28 + 1), open: c + 0.5, high: c + 1., low: c - 1., close: c, volume: None }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let t = run("ccl07_rsi10_scale", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert_eq!(t.len(), 1);
        let z = &t[0];
        // signal at 221 (close 300), bought at 222's open 299.5; 222 closes 299 < 299.5 -> add; 223 closes 298 < 299 -> add
        assert_eq!(z.legs.iter().map(|l| (l.i, l.px, l.w)).collect::<Vec<_>>(), vec![(222, 299.5, 0.2), (222, 299., 0.3), (223, 298., 0.5)]);
        assert_eq!((z.exit_i, z.exit_px, z.open), (225, 340.5, false));
        assert!(run("ccl07_x", &x, 0, r.len() - 1, &Params::new()).is_none());
    }
    fn vrows(cl: &[f64]) -> Vec<Row> {
        cl.iter().enumerate().map(|(i, &c)| Row { ticker: "T".into(), date: format!("{}-{:02}-{:02}", 2001 + i / 336, i / 28 % 12 + 1, i % 28 + 1), open: c * 1.001, high: c * 1.01, low: c * 0.99, close: c, volume: Some(200_000.) }).collect()
    }
    #[test]
    fn class_project_setup_and_exits() {
        // rising zig-zag (HV(100) well above 30), three sharp down closes, then a rally
        let mut cl: Vec<f64> = (0..260).map(|i| 100. * 1.003f64.powi(i) * if i % 2 == 0 { 1.03 } else { 0.97 }).collect();
        let k = *cl.last().unwrap();
        cl.extend([k * 0.93, k * 0.86, k * 0.80, k * 0.90, k * 1.0, k * 1.05, k * 1.0]);
        let r = vrows(&cl);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("ccl07_class_project", &x, 0, r.len() - 1, &Params::new()).unwrap();
        let (rs, hv) = (x.rsi_n(2), hv_ddof(&x.c, 100, 0.));
        assert!(!t.is_empty());
        for t in t.iter().filter(|t| !t.open) {
            let s = t.legs[0].i - 1;
            assert!(x.c[s] > x.ma200[s] && rs[s] < 10. && hv[s] > 30. && x.lf[s] > 90. && (s - 2..=s).all(|k| x.c[k] < x.c[k - 1]));
            assert_eq!(t.legs[0].px, x.o[s + 1]);
            assert!(rs[t.signal_i] > 70. && t.exit_i == t.signal_i + 1 && t.exit_px == x.o[t.exit_i]);
            assert!((t.legs[0].i..t.signal_i).all(|k| !(rs[k] > 70.)));
        }
    }
    #[test]
    fn carlson_breakout_and_cross_exit() {
        let mut cl: Vec<f64> = (0..200).map(|i| 100. + i as f64).collect();
        cl.extend((0..150).map(|i| 299. - 2. * i as f64));
        let r = vrows(&cl);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("ccl07_carlson_bo", &x, 0, r.len() - 1, &Params::new()).unwrap();
        let (f, s) = (x.sma_n(14), x.sma_n(125));
        let cross = (161..r.len()).find(|&i| f[i] < s[i] && f[i - 1] >= s[i - 1]).unwrap();
        assert_eq!((t[0].legs[0].i, t[0].exit_i, t[0].open), (159, cross, false));
        // the 100,000-share floor blocks it when the reference volume is lower
        let mut p = Params::new();
        p.insert("min_vol".into(), 300_000.);
        assert!(run("ccl07_carlson_bo", &x, 0, r.len() - 1, &p).unwrap().is_empty());
        // follow-through: first close above every close since the breakout, at least 10 days later
        p.insert("min_vol".into(), 100_000.);
        p.insert("ft".into(), 1.);
        assert_eq!(run("ccl07_carlson_bo", &x, 0, r.len() - 1, &p).unwrap()[0].legs[0].i, 169);
    }
}

// Instrument admission: Larry Connors (Chairman's Club Dec 2007) 2026-10-30 (titles lane K3) block, moved from instrument_policy.rs on 31 Oct 2026 (lane ST).
// Admission only for the module REVIEWED lists (authors/ccl07/admission/k3_ref.py (header)); kept in one block to merge.
// Runs on actual indices, ETFs and stocks, US and India (index_etf_or_stock, nifty50 basket, declared application).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book=="ccl07" && matches!(def.id, "ccl07_rsi10_scale" | "ccl07_worst10"),
    label: "The RSI 10 Scale-In is shown on S&P e-minis with SPY as the filter and the 10 Worst Performers on US stocks; other indices, ETFs, single stocks and India are a declared application (the traded series is its own filter).",
    basis: "Larry Connors / TradingMarkets, Chairman's Club December 2007 (Las Vegas) manual, slides 78-80 and 214-216",
};
// Lane K5 (31 Oct 2026): the class project and Carlson's breakout study (authors/ccl07/admission/k5/k5_ref.py (header)).
pub const ADMISSION_K5: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book=="ccl07" && matches!(def.id, "ccl07_class_project" | "ccl07_carlson_bo") && crate::books::ccl07::reviewed(def.id),
    label: "The AmiBroker class project and Curt Carlson's breakout study are stock rules (Carlson tested S&P 500 stocks as a portfolio); single US and NSE stocks run as the rule is written, indices and ETFs are a declared application, not the authors' test.",
    basis: "Larry Connors / TradingMarkets, Chairman's Club December 2007 (Las Vegas) manual, slides 97-102 (class project) and 229-237 (Curt Carlson, A Breakout Study)",
};
