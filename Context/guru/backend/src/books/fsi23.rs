//! ForexSuperInvestor.com (L. K. "Lynndal" Daniels), "RSI 2.3 System" for TradeStation 8 (2007): "Ebay RSI 2.3
//! Instructions.doc" (input-by-input manual) and the strategy text "Ebay RSI 2.3.txt" from Code.rar. Book id `fsi23`.
//! Source: research drive "USB HDD 1/TradeStation Systems & Indicators/RSI 2.3 Strategy for TS (forexsuperinvestor.com)/".
//! Daily swing system "originally designed for trading futures indices" (eBay listing): nothing in the rules is
//! FX-specific, so it runs on the index as an instrument swap.
//! The code is the exact statement and wins over the manual where they differ: the code has MAFilterSwitch(0), so
//! the 200-day filter is OFF for the initial entry by default (the manual describes MovAvgLength as an active filter
//! and lists no switch; that reading is the labelled option `ma_filter` = 1). The add-on always requires
//! close > MA (long) / close < MA (short) in the code. DayTrade / EOD inputs only act on intraday bars.
//! Code mechanics reproduced: entry signal only when flat on the close; limit order next bar at
//! Close -/+ LongCushion x ATR, ATR = Average(TrueRange, 15) taken on the signal bar; add-on limit (one, on the bar
//! after the entry signal, i.e. the entry bar, if long at its close and RSI < 15) at that close - 0.2 x the same ATR;
//! RSI exit "THIS BAR CLOSE"; SetStopContract + SetStopLoss / SetProfitTarget of 3 x ATR (signal bar) per contract.
//! Assumptions where TradeStation's daily-bar sequencing is not given by the source (documented, class B):
//! stop/target act from the bar after the entry fill, on the average entry price of the position held at the prior
//! close; a gap through either fills at the open; stop before target on the same bar; the add-on limit is cancelled
//! on a bar where the stop/target closed the position; legs are equal weights (NumOfContracts 1 each).
use crate::connors::{true_range, sma, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "fsi23";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef {
        id: "fsi23_rsi23", name: "RSI 2.3 (RSI(4) 25/75, ATR-cushion limit entry, add-on, 3-ATR stop and target)",
        group: "RSI 2.3 Instructions + TradeStation code", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
        rules: &[
            "F|RSI.Length(4) (Wilder RSI of the close); MovAvgLength(200); code: MAFilterSwitch(0) - \"IF MAFilterSwitch = 1 THEN Variable15 = CLOSE > Variable11 AND Variable13 ELSE ...\" (manual: \"If the closing price is above this number, the system can take long trades. If the closing price is below this number it can take short trades.\")",
            "E|Long: flat and RSI(4) < RSI.BuyValue(25) on RSIConsecBars4Entry(1) bars -> \"BUY ... NEXT BAR [Close - ATR x LongCushion(0.2)] LIMIT\" (EntryOrderType(1) = limit); ATR = Average(TrueRange, ATRLength(15))",
            "E|Short: flat and RSI(4) > RSI.ShortValue(75) -> \"SELLSHORT ... NEXT BAR [Close + ATR x ShortCushion(0.2)] LIMIT\"",
            "A|AggressiveAddOn(1), AddOnWithinBars(1): long and RSI < RSI.BuyAddValue(15) and Close > MA on the bar after the entry signal -> one more contract next bar at Close - 0.2 x ATR limit (short: RSI > 85 and Close < MA, Close + 0.2 x ATR)",
            "X|\"IF Variable10 = 1 AND [RSI > RSI.LongExitValue(55)] THEN SELL ALL CONTRACT THIS BAR CLOSE\"; short: RSI < RSI.ShortExitValue(45) -> buy to cover this bar close (RSIConsecBars4Exit(1))",
            "X|\"SETSTOPCONTRACT; IF StopNumATRs > 0 THEN SETSTOPLOSS(3 x ATR x BigPointValue); IF TargetNumATRs > 0 THEN SETPROFITTARGET(3 x ATR x BigPointValue)\" (StopNumATRs(3), TargetNumATRs(3); manual: \"I personally set my initial stop loss at a setting of 3 ATR\")",
            "F|eBay listing: \"designed for daily charts and trades approximately twice per month on most symbols\"; \"Profit factors generally exceed 2.0+ and more than 50% of trades are profitable\" (S&P E-mini and GBPUSD, 3 years of data; summaries were images, not in the source)",
        ],
        notes: "Futures/forex system run on the cash index (instrument swap; percentage returns; legs weighted 1/2 + 1/2 when the add-on is enabled). Code defaults are used (filter off for the first entry). TradeStation daily-bar sequencing assumptions (class B): stop/target active from the bar after the entry fill, computed on the average entry price of the position held at the prior close, gap fills at the open, stop before target, add-on cancelled on a bar where stop/target exits. The vendor states no numeric results for an index, so none can be compared.",
        claim: None,
    }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == "fsi23_rsi23").then_some(("fsi23_rsi23", "RSI 2.3 System", "Code defaults"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    (id == "fsi23_rsi23").then(|| vec![
        p("ma_filter", "MAFilterSwitch (0 = code default off, 1 = manual: close vs MA gates entries)", 0., 0., 1., 1.),
        p("ma_len", "MovAvgLength", 200., 50., 250., 50.),
        p("rsi_len", "RSI.Length", 4., 2., 14., 1.),
        p("buy", "RSI.BuyValue", 25., 5., 40., 5.),
        p("buy_add", "RSI.BuyAddValue", 15., 5., 30., 5.),
        p("long_exit", "RSI.LongExitValue", 55., 40., 80., 5.),
        p("short", "RSI.ShortValue", 75., 60., 95., 5.),
        p("short_add", "RSI.ShortAddValue", 85., 70., 95., 5.),
        p("short_exit", "RSI.ShortExitValue", 45., 20., 60., 5.),
        p("long_on", "LongTrade", 1., 0., 1., 1.),
        p("short_on", "ShortTrade", 1., 0., 1., 1.),
        p("add_on", "AggressiveAddOn", 1., 0., 1., 1.),
        p("add_within", "AddOnWithinBars", 1., 1., 5., 1.),
        p("consec_entry", "RSIConsecBars4Entry", 1., 1., 3., 1.),
        p("consec_exit", "RSIConsecBars4Exit", 1., 1., 3., 1.),
        p("limit", "EntryOrderType (1 = limit, 0 = market next open)", 1., 0., 1., 1.),
        p("cushion", "Long/ShortCushion (x ATR)", 0.2, 0., 1., 0.1),
        p("atr_len", "ATRLength", 15., 5., 30., 5.),
        p("stop_atr", "StopNumATRs (0 = off)", 3., 0., 6., 0.5),
        p("target_atr", "TargetNumATRs (0 = off)", 3., 0., 6., 0.5),
    ])
}

struct Pend { dir: f64, px: f64, add: bool }

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap().def);
    let ui = |k: &str| g(k).round().max(1.0) as usize;
    let (ma_on, long_on, short_on, add_on, limit) = (g("ma_filter") >= 0.5, g("long_on") >= 0.5, g("short_on") >= 0.5, g("add_on") >= 0.5, g("limit") >= 0.5);
    let (ce, cx, within) = (ui("consec_entry"), ui("consec_exit"), ui("add_within"));
    let (cush, sk, tk) = (g("cushion"), g("stop_atr"), g("target_atr"));
    let to = to.min(x.c.len().saturating_sub(1));
    let c = &x.c;
    let ma = x.sma_n(ui("ma_len"));
    let rsi = x.rsi_n(ui("rsi_len"));
    let atr = sma(&true_range(x.r), ui("atr_len"));
    let all = |t: usize, k: usize, f: &dyn Fn(usize) -> bool| t + 1 >= k && (t + 1 - k..=t).all(|i| f(i));
    let w = if add_on { 0.5 } else { 1.0 };
    let mut out = Vec::new();
    let mut cur: Option<Trade> = None;
    let mut pend: Option<Pend> = None;
    let (mut atr_s, mut win) = (f64::NAN, (usize::MAX, 0usize));
    for t in from..=to {
        // 1. intrabar: stop / target on the position held at the prior close, then pending limit/market orders
        let mut exited = false;
        if let Some(q) = cur.as_ref().filter(|q| q.legs.iter().all(|l| l.i < t)) {
            let avg = q.avg_px();
            let d = q.dir;
            let stop = if sk > 0.0 { Some(avg - d * sk * atr_s) } else { None };
            let tgt = if tk > 0.0 { Some(avg + d * tk * atr_s) } else { None };
            let (o, h, l) = (x.o[t], x.h[t], x.l[t]);
            let through = |lvl: f64, adverse: bool| if (d > 0.0) == adverse { o <= lvl } else { o >= lvl };
            let fill = if let Some(s) = stop.filter(|&s| through(s, true)) { Some((o, "Stop 3 ATR (gap, open)", s)) }
                else if let Some(tg) = tgt.filter(|&tg| through(tg, false)) { Some((o, "Target 3 ATR (gap, open)", tg)) }
                else if let Some(s) = stop.filter(|&s| if d > 0.0 { l <= s } else { h >= s }) { Some((s, "Stop 3 ATR", s)) }
                else if let Some(tg) = tgt.filter(|&tg| if d > 0.0 { h >= tg } else { l <= tg }) { Some((tg, "Target 3 ATR", tg)) }
                else { None };
            if let Some((px, why, _)) = fill {
                let mut q = cur.take().unwrap();
                q.exit_i = t; q.exit_px = px; q.signal_i = t - 1; q.reason = why.into();
                out.push(q);
                exited = true;
            }
        }
        if let Some(pd) = pend.take() {
            let ok = !(pd.add && (exited || cur.is_none()));
            let fill = if !ok { None } else if pd.px.is_nan() { Some(x.o[t]) }
                else if pd.dir > 0.0 { (x.l[t] <= pd.px).then(|| x.o[t].min(pd.px)) } else { (x.h[t] >= pd.px).then(|| x.o[t].max(pd.px)) };
            if let Some(px) = fill {
                if pd.add {
                    cur.as_mut().unwrap().legs.push(Leg { i: t, px, w });
                } else if cur.is_none() {
                    cur = Some(Trade { dir: pd.dir, legs: vec![Leg { i: t, px, w }], exit_i: t, exit_px: px, reason: String::new(),
                        entry_kind: if limit { "limit" } else { "open" }, signal_i: t - 1, open: false, exits: vec![] });
                }
            }
        }
        if t == to { break; }
        // 2. on the close
        let mp = cur.as_ref().map_or(0.0, |q| q.dir);
        let valid = rsi[t].is_finite() && atr[t].is_finite();
        if long_on && valid {
            let sig = all(t, ce, &|i| rsi[i] < g("buy")) && (!ma_on || c[t] > ma[t]);
            if sig && mp == 0.0 {
                atr_s = atr[t]; win = (t + 1, t + within);
                pend = Some(Pend { dir: 1.0, px: if limit { c[t] - atr_s * cush } else { f64::NAN }, add: false });
            }
            if add_on && mp > 0.0 && rsi[t] < g("buy_add") && c[t] > ma[t] && t >= win.0 && t <= win.1 {
                pend = Some(Pend { dir: 1.0, px: if limit { c[t] - atr_s * cush } else { f64::NAN }, add: true });
            }
            if mp > 0.0 && all(t, cx, &|i| rsi[i] > g("long_exit")) {
                let mut q = cur.take().unwrap();
                q.exit_i = t; q.exit_px = c[t]; q.signal_i = t; q.reason = "RSI > 55, this bar close".into();
                out.push(q);
                pend = None;
            }
        }
        if short_on && valid {
            let sig = all(t, ce, &|i| rsi[i] > g("short")) && (!ma_on || c[t] < ma[t]);
            if sig && mp == 0.0 {
                atr_s = atr[t]; win = (t + 1, t + within);
                pend = Some(Pend { dir: -1.0, px: if limit { c[t] + atr_s * cush } else { f64::NAN }, add: false });
            }
            if add_on && mp < 0.0 && rsi[t] > g("short_add") && c[t] < ma[t] && t >= win.0 && t <= win.1 {
                pend = Some(Pend { dir: -1.0, px: if limit { c[t] + atr_s * cush } else { f64::NAN }, add: true });
            }
            if mp < 0.0 && all(t, cx, &|i| rsi[i] < g("short_exit")) {
                let mut q = cur.take().unwrap();
                q.exit_i = t; q.exit_px = c[t]; q.signal_i = t; q.reason = "RSI < 45, this bar close".into();
                out.push(q);
                pend = None;
            }
        }
    }
    if let Some(mut q) = cur {
        q.exit_i = to; q.exit_px = c[to]; q.signal_i = to; q.reason = "open".into(); q.open = true;
        out.push(q);
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn runs_and_fills_inside_the_bar() {
        let r: Vec<Row> = (0..400).map(|i| {
            let c = 100.0 + 10.0 * ((i as f64) / 7.0).sin();
            Row { ticker: "T".into(), date: format!("{:04}-01-01", 1600 + i), open: c, high: c + 1.0, low: c - 1.0, close: c, volume: None }
        }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let ts = run("fsi23_rsi23", &x, 250, 399, &Params::new()).unwrap();
        assert!(!ts.is_empty());
        for t in &ts {
            for l in &t.legs { assert!(l.px >= r[l.i].low - 1e-9 && l.px <= r[l.i].high + 1e-9); }
            assert!(t.exit_px >= r[t.exit_i].low - 1e-9 && t.exit_px <= r[t.exit_i].high + 1e-9);
        }
    }
}
