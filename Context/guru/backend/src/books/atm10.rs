//! Active Trader Magazine, 2010 (partial issues Feb-May/June 2010 on the research drive Trading Knowledge II/@NewsLetters-Mags/
//! Active Trader Magazine/2010/; scanned PDFs read by OCR + page images). Book id "atm10". Notes: library/atm10/README.md.
//! Uses the atm03 helpers (own simulator; no jkbwts::sim).
use crate::books::atm03::{done, ema, finish, new_trade, stop_dn, stop_up};
use crate::connors::{atr, rsi, sma, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};
use Status::*;

pub const BOOK: &str = "atm10";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, dir, status, rules, notes, claim| StrategyDef { id, name, group, session: 0, dir, status, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim };
    let c = crate::connors::claim;
    vec![
        d("atm10_rsi_scaleout", "RSI scale-out system · buy RSI(14) < 30, sell quarters at RSI 45/60/75/90", "TSL Feb 2010 p.40-43 (Knapp)", "Long", Live,
          &["E|\"1. Buy at tomorrow's open when the 14-period RSI goes below 30.\"",
            "X|\"2. Exit a quarter of the position at tomorrow's open each time the 14-period RSI increases by 15 points (e.g., 45, 60, 75, and 90).\"",
            "X|\"3. Exit the entire position when price falls below the entry price by five times the 20-day ATR.\"",
            "X|\"4. Exit the entire position at the market after 300 days of inactivity (i.e., no exit condition has triggered).\""],
          "Readings [B]: 'goes below 30' = crosses below 30; rule 3 tested on the close with the current Wilder ATR(20), exit at the next open; 'inactivity' = 300 bars since the entry or the last partial exit; each RSI level is used once per trade. Scale-outs are the trade's exit pieces. Book: 17-stock AT portfolio 11/1999-10/2009, 20% of equity per position.",
          c("444", "70.05%", "+9.2%", "97.73 bars", "AT 17-stock portfolio 11/1999-10/2009: +75.9%, PF 1.65, max DD -21.13%")),
        d("atm10_vol_scalein", "Volatility scale-in system · EMA(20/50) cross above SMA(200), add every 3 x ATR(10)", "TSL Apr 2010 p.52-55 (Knapp)", "Long", Live,
          &["E|\"1. Buy tomorrow's open when the 20-day exponential moving average (EMA) of closing prices crosses above the 50-day EMA of closing prices and the daily close is above the 200-day simple moving average (SMA) of closing prices.\"",
            "A|\"2. Scale in by adding to the position every time prices advance three times the 10-day ATR above the last entry price. Limit the number of total positions in a stock to four (i.e., scale in no more than three times).\"",
            "X|\"3. Exit the entire position at the close when the 20-day EMA of closing prices crosses below the 50-day EMA of closing prices.\"",
            "X|\"4. Exit the whole position with a stop at six times the 10-period ATR below the most recent trade entry if price goes against the most recently entered position.\"",
            "F|\"Allocate 3 percent of account equity per entry (maximum of four entries per total position).\""],
          "Readings [B]: adds are buy stops at last entry + 3 x ATR(10) of the previous bar (gap = open); the protective stop is a stop order at last entry - 6 x ATR(10) of the previous bar, from the bar after the last entry; EMA = Wealth-Lab EMA (SMA seed). Leg weight p.w = 0.03 (book). Book: 17-stock AT portfolio 1/2000-12/2009.",
          c("345", "43%", "+6.6%", "81 days", "AT 17-stock portfolio 1/2000-12/2009: +86.95%, max DD -13.16%")),
        d("atm10_one_pct", "1-percent strategy (QQQQ) · buy the close of a 1% down day, +1% target / -1% stop next day", "Trading Strategies May 2010 p.28-31", "Long", Live,
          &["E|Down day: \"closes and lows at least 1 percent below the previous days' prices\" (both close-to-close and low-to-low); \"going long on the close of each down day\"",
            "X|\"1. Exit the next day 1 percent above the down day's closing price. 2. If the next day's opening price is more than 1 percent below the previous close, exit on the open. 3. If the 1-percent target is not reached and rule 2 is false, exit on the close or with a stop-loss 1 percent below the down day's closing price.\""],
          "Next day: an open more than 1% below the entry close exits at the open; otherwise the target (limit, an open above it fills at the open) and the 1% stop work intraday, stop first if both trade [B]; otherwise exit at the close. Book: QQQQ 2/25/2000-2/24/2010, 500 shares, $5 per trade.",
          c("411", "72.5% (298 of 411)", "net $29,627 / 500 sh", "1 day", "QQQQ 2/25/2000-2/24/2010")),
        d("atm10_weak_rotation", "Weak-stock rotation (Dow 30) · buy the 5 biggest 14-day decliners, hold >= 5 days", "TSL Mar 2010 p.44-47 (Knapp)", "Long",
          NeedsData("the 30 Dow stocks (daily cross-sectional ranking of 14-day percentage change); the engine runs one instrument at a time"),
          &["E|\"1. Go long tomorrow at the open the five stocks that made the largest percentage declines over the past 14 days. 2. Hold the stocks at least five days.\"", "X|\"3. Sell tomorrow at the open when a stock is no longer among the five biggest percentage decliners over the past 14 days.\""],
          "Book: Dow 30 12/1999-11/2009, 1,820 trades, 55.44%, +0.44%/trade, +176.65%.", c("1,820", "55.44%", "+0.44%", "8.22 days", "Dow 30 12/1999-11/2009")),
        d("atm10_not_testable", "Other 2010 strategy articles (System design series, Turtle revision, crude-oil breakouts, 30-day SMA with $ stops, VIXRSI, TTM momentum, Pinocchio bars)", "Feb-Jun 2010", "Long/Short",
          NotTestable("System design parts 10-13: the long exit (momentum length/threshold) is defined in earlier installments not on the drive; Turtle revision and crude-oil breakout: parameters optimized/illegible; SMA-30 system: $1,000 stop and $15,000 target are futures dollars; VIXRSI and Pinocchio bars: no exit rule; TTM momentum: 60-minute bars"),
          &["E|e.g. System design part 11 (Apr 2010): \"low[0]<low[1] and low[1]<low[2] and low[2]<low[3] and close<open and close<close[2] and close[1]<close[3] ... Then buy today on the close\""],
          "See README. The original Turtle rules are already in the vault (covel); the June 2010 TSL breakout (85/20 channels) is atm03_ch100_20 with other lengths.", None),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let v: &'static str = catalogue().into_iter().find(|d| d.id == id)?.name;
    match id { s if s.starts_with("atm10_") => Some(("atm10", "Active Trader 2010", v)), _ => None }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(match id {
        "atm10_rsi_scaleout" => vec![p("n", "RSI length", 14., 5., 30., 1.), p("entry", "Buy when RSI goes below", 30., 10., 40., 1.), p("step", "Scale-out step (RSI points)", 15., 5., 25., 1.),
            p("atr_n", "ATR length", 20., 5., 50., 1.), p("atr_k", "Stop: ATR multiple below entry", 5., 1., 10., 0.5), p("idle", "Exit after bars without an exit", 300., 20., 500., 10.)],
        "atm10_vol_scalein" => vec![p("fast", "Fast EMA", 20., 5., 50., 1.), p("slow", "Slow EMA", 50., 20., 200., 5.), p("trend", "Trend SMA", 200., 50., 300., 10.),
            p("add_k", "Add every x ATR above the last entry", 3., 0.5, 6., 0.5), p("stop_k", "Stop x ATR below the last entry", 6., 1., 10., 0.5), p("atr_n", "ATR length", 10., 5., 30., 1.),
            p("max", "Maximum entries", 4., 1., 8., 1.), p("w", "Weight per entry (fraction of equity)", 0.03, 0.01, 1., 0.01)],
        "atm10_one_pct" => vec![p("drop", "Down-day size %", 1., 0.25, 5., 0.25), p("tgt", "Target % above the entry close", 1., 0.25, 5., 0.25), p("stop", "Stop % below the entry close", 1., 0.25, 5., 0.25), p("tgt_first", "1 = target before stop when both trade in one bar (labelled option)", 0., 0., 1., 1.)],
        "atm10_weak_rotation" | "atm10_not_testable" => vec![],
        _ => return None,
    })
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    let n = x.r.len();
    if !p.keys().all(|k| schema.iter().any(|d| d.key == k)) || from > to || to >= n { return None; }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(0., |d| d.def));
    let u = |k: &str| g(k).max(1.) as usize;
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    Some(match id {
        "atm10_rsi_scaleout" => {
            let (r, a) = (rsi(c, u("n")), atr(x.r, u("atr_n")));
            let (ent, step, k, idle) = (g("entry"), g("step"), g("atr_k"), u("idle"));
            let (mut out, mut cur) = (vec![], None::<Trade>);
            let (mut done_q, mut last_ev) = (0usize, 0usize);
            for i in from.max(1)..to {
                let j = i + 1;
                if let Some(mut t) = cur.take() {
                    let e = t.legs[0].px;
                    let left = 1. - 0.25 * done_q as f64;
                    let full = c[i] < e - k * a[i] || i - last_ev >= idle;
                    if full {
                        t.exits.push((j, o[j], left));
                        let px = t.exits.iter().map(|q| q.1 * q.2).sum::<f64>();
                        let why = if c[i] < e - k * a[i] { "5 x ATR below entry: exit the rest next open" } else { "300 bars without an exit: exit the rest" };
                        out.push(done(t, j, px, i, why));
                        continue;
                    }
                    // RSI levels entry+step, +2 step, ... each once; a jump over several levels sells one quarter per level
                    let mut q = 0;
                    while done_q + q < 4 && r[i] >= ent + step * (done_q + q + 1) as f64 { q += 1; }
                    if q > 0 {
                        t.exits.push((j, o[j], 0.25 * q as f64));
                        done_q += q;
                        last_ev = i;
                        if done_q >= 4 {
                            let px = t.exits.iter().map(|q| q.1 * q.2).sum::<f64>();
                            out.push(done(t, j, px, i, "last quarter sold at RSI 90"));
                            continue;
                        }
                    }
                    cur = Some(t);
                } else if r[i] < ent && r[i - 1] >= ent {
                    cur = Some(new_trade(1., j, o[j], "open"));
                    done_q = 0;
                    last_ev = j;
                }
            }
            if let Some(mut t) = cur {
                let left = 1. - 0.25 * done_q as f64;
                t.exits.push((to, c[to], left));
                let px = t.exits.iter().map(|q| q.1 * q.2).sum::<f64>();
                t.exit_i = to; t.exit_px = px; t.signal_i = to; t.reason = "open".into(); t.open = true; out.push(t);
            }
            // a trade's exits list is only kept when it has more than one piece
            for t in out.iter_mut() { if t.exits.len() == 1 { t.exits.clear(); } }
            out
        }
        "atm10_vol_scalein" => {
            let (f, s, tr, a) = (ema(c, u("fast")), ema(c, u("slow")), sma(c, u("trend")), atr(x.r, u("atr_n")));
            let (ak, sk, mx, w) = (g("add_k"), g("stop_k"), u("max"), g("w"));
            let (mut out, mut cur) = (vec![], None::<Trade>);
            for j in from.max(2)..=to {
                let i = j - 1;
                if let Some(mut t) = cur.take() {
                    let lastl = t.legs.last().unwrap().clone();
                    // protective stop below the most recent entry (from the bar after it)
                    if j > lastl.i { if let Some(px) = stop_dn(x, j, lastl.px - sk * a[i]) { out.push(done(t, j, px, j, "6 x ATR stop below the last entry")); continue; } }
                    // add on a buy stop 3 x ATR above the last entry
                    if t.legs.len() < mx && j > lastl.i { if let Some(px) = stop_up(x, j, lastl.px + ak * a[i]) { t.legs.push(Leg { i: j, px, w }); } }
                    if f[j] < s[j] && f[j - 1] >= s[j - 1] { out.push(done(t, j, c[j], j, "EMA20 crossed below EMA50: exit at the close")); continue; }
                    cur = Some(t);
                    continue;
                }
                if f[i] > s[i] && f[i - 1] <= s[i - 1] && c[i] > tr[i] { let mut t = new_trade(1., j, o[j], "open"); t.legs[0].w = w; cur = Some(t); }
            }
            finish(&mut out, cur, x, to);
            out
        }
        "atm10_one_pct" => {
            let (dr, tg, st, tf) = (1. - g("drop") / 100., 1. + g("tgt") / 100., 1. - g("stop") / 100., g("tgt_first") > 0.5);
            let mut out = vec![];
            for i in from.max(1)..=to {
                if !(c[i] <= c[i - 1] * dr && l[i] <= l[i - 1] * dr) { continue; }
                let t = new_trade(1., i, c[i], "close");
                if i == to { finish(&mut out, Some(t), x, to); break; }
                let j = i + 1;
                let e = c[i];
                let (px, why) = if o[j] < e * st { (o[j], "open more than 1% lower: exit at the open") }
                    else if tf && h[j] >= e * tg { (o[j].max(e * tg), "1% target") }
                    else if let Some(v) = stop_dn(x, j, e * st) { (v, "1% stop") }
                    else if h[j] >= e * tg { (o[j].max(e * tg), "1% target") }
                    else { (c[j], "exit at the close") };
                out.push(done(t, j, px, j, why));
            }
            out
        }
        "atm10_weak_rotation" | "atm10_not_testable" => vec![],
        _ => return None,
    })
}

