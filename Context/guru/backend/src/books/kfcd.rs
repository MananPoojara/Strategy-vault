//! Perry Kaufman, *New Trading Systems and Methods*, 4th ed. (Wiley 2005): the companion-CD TradeStation systems that
//! the `kftsm` module left out (CD Chapter12 volume systems, Chapter20 FC Orson, Chapter22/23 sizing programs).
//! Source: Books/Perry Kaufman/TSM4 CD/ChapterNN/TSM(S) <name>.txt (copied from research `Trading Knowledge I/Perry
//! J.Kaufman - New Trading Systems & Methods (4th Ed & Cd)/`). The code is the rule: every default is the code's input
//! default. The CD prints no results for these programs.
//!
//! TradeStation conventions used: `average` = simple mean, `stddev` = population standard deviation, `on close` = fill
//! at this bar's close, `at market` = next bar's open, `at X stop` = next bar only, fill at X or at the open if the open
//! is already through X. Intrabar order for stop orders = TradeStation's bar path: open, the extreme nearer the open,
//! the other extreme, close. A `sell`/`buy` entry reverses an opposite position. All state starts at the first bar of
//! data; trades entered before `from` are dropped. No bar after `to` is read.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

const BOOK: &str = "kfcd";

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    let d = |id, name, group, status, vol, rules, notes| StrategyDef {
        id, name, group, session: 0, dir: "Long/Short", status, needs_volume: vol, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None,
    };
    const SIZING: &str = "not testable: a position-sizing study measured in futures contracts (and, for Anti-Martingales 1 and the equity trend, in dollars of margin/equity); the vault books one normalized position per trade, so the varying contract counts that are the whole point of the program are not represented. The underlying signal is a plain 5/20-day moving-average crossover, always in the market";
    vec![
        d("kfcd_fc_orson", "FC Orson: breakout with low volatility", "CD Ch20 · Volatility", Live, false,
          &["F|vola = (TSMFCVolatility(close,length) - lowest(TSMFCVolatility(close,length),60)) / ((highest(..,60) - lowest(..,60))*0.01); condition1 = vola < filter (length 20, filter 20)",
            "F|TSMFCVolatility = (average(log(close/close[1]),len) + stddev(log(close/close[1]),len)) * squareroot(252) (CD Ch20 TSM(F) FC Volatility)",
            "E|if condition1: buy at highest(high,length) stop; sell at lowest(low,length) stop",
            "X|if marketposition > 0 then exitlong at lowest(low,length) stop; if marketposition < 0 then exitshort at highest(high,length) stop"],
          "Code by Francesco Cavasino on the TSM4 CD (Ch20, 'not in text'); meant for futures. Orders are for the next bar only, re-issued each bar. Starts once 80 bars exist (20 for the volatility + 60 for its range). India = instrument swap."),
        d("kfcd_high_volume", "TSM High Volume filter on a moving-average trend", "CD Ch12 · Volume", Live, true,
          &["F|uplimit = average(volume,length) + 2*factor*stddev(volume,length) (length 70, factor 1.0); mavg = average(close,length/2)",
            "E|option 3 (default): if mavg > mavg[1] and volume < uplimit and position <> 1 then buy on close; if mavg < mavg[1] and volume < uplimit and position <> -1 then sell on close ('Don't reenter after high volume exit': position resets to 0 only when the average turns against it)",
            "X|exitlong on close if mavg < mavg[1] or (close > close[1] and volume > uplimit); exitshort on close if mavg > mavg[1] or (close < close[1] and volume > uplimit)",
            "F|options: 0 = no filter, 1 = entry filter only, 2 = exit filter only, 3 = both"],
          "Literal code quirk kept: the 'replace high volume days' line sets adjvol to savevol, which already equals today's volume, so it never changes anything. Volume = the index's daily volume."),
        d("kfcd_low_volume", "TSM Low Volume filter on a moving-average trend", "CD Ch12 · Volume", Live, true,
          &["F|lowlimit = average(volume,length) - factor*stddev(volume,length); vavgn = average(volume,nlow) (length 70, nlow 5, factor 1.0); mavg = average(close,length/2)",
            "E|options 1 (default) or 3: if mavg > mavg[1] and vavgn > lowlimit then buy on close; if mavg < mavg[1] and vavgn > lowlimit then sell on close. options 2: buy/sell on close only on the turn (mavg[1] <= mavg[2])",
            "X|options 1: exitlong on close if mavg < mavg[1]; exitshort if mavg > mavg[1]. options 2/3: also exit when PositionProfit > 0 and vavgn <= lowlimit"],
          "Same no-op 'replace high volume' line as High Volume. Volume = the index's daily volume."),
        d("kfcd_volume_filters", "TSM Volume Filters (volume indicator divergence on a moving-average trend)", "CD Ch12 · Volume", Live, true,
          &["F|ttype 1 (default) On-Balance Volume; 2 Chaikin volume accumulator; 3 negative volume index; 4 positive volume index; 5 price-volume trend; 6 volume oscillator; 7 volume rate of change; 8 low-volume exit (9 Herrick needs open interest: not coded)",
            "F|map = average(close,period); px = (map - map[ddays])*100/map[ddays]; ma = average(close,2*period); madiff = ma - ma[ddays] (period 20, ddays 3)",
            "F|types 1-5: mav = average(Kvol,period); vx = (mav - mav[ddays])*100/mav[ddays]; dvg = px - vx when vx*px < 0, else 0",
            "E|madiff crosses above 0: exitshort at market and, if vx >= maxdn and |dvg| <= maxdvg (both 0), buy at market; crosses below 0: exitlong at market and, under the same test, sell at market",
            "X|types 1-5: if vx < maxdn and |dvg| > maxdvg then exitlong and exitshort at market",
            "X|types 6-7: also exit when Kvol <= average(Kvol,period) - npsd*stddev(Kvol,period) (npsd .5); type 8: when average(volume,ddays) <= average(volume,period)*npsd",
            "F|Kopt 1: types 1-5 add volume only when |close - close[1]| > npsd*stddev(|change|,period)"],
          "'at market' = next open. Only the exit harness is coded (positive ttype); the code's entry-filter mode (negative ttype) is not exposed. Volume = the index's daily volume."),
        d("kfcd_martingales", "TSM Martingales (contracts doubled after a loss)", "CD Ch22 · Betting strategies", NotTestable(SIZING), false,
          &["E|average(close,5) > average(close,20): buy ncontr contracts at close; < : sell", "A|after a loss ncontr = intportion(ncontr*factor) (factor 2); after a profit ncontr = size (2)"], SIZING),
        d("kfcd_antimartingales1", "TSM Anti-Martingales 1 (reinvest profits by margin)", "CD Ch22 · Betting strategies", NotTestable(SIZING), false,
          &["E|5/20 MA crossover at close", "A|after a profit ncontr = size + intportion(profitrun*factor/cmargin) up to maxseq 3 profits (size 10, factor 1.0, cmargin 1000); after a loss size"], SIZING),
        d("kfcd_antimartingales2", "TSM Anti-Martingales 2 (multiply after a profit)", "CD Ch22 · Betting strategies", NotTestable(SIZING), false,
          &["E|5/20 MA crossover at close", "A|after a profit ncontr = intportion(ncontr*factor) (factor 2) up to maxseq 3; then size 10"], SIZING),
        d("kfcd_equity_trend", "TSM Equity Trend (cut size when equity trends down)", "CD Ch23 · Risk control", NotTestable(SIZING), false,
          &["E|5/20 MA crossover at close", "A|if average(equity,10) is falling, trade size*reduce/100 (50%) contracts, else size"], SIZING),
        d("kfcd_liquidity", "TSM Liquidity Management (scale size by volume)", "CD Ch23 · Risk control", NotTestable(SIZING), false,
          &["E|average(close,20) turns up: buy ncontr at close; turns down: sell", "A|ncontr = size*average(volume,3)/average(volume,20), capped at max 2; option 1 scale down only"], SIZING),
        d("kfcd_shock_study", "TSM Shock study (price-shock events)", "CD Ch13/Ch22 · Events", NotTestable("not testable: an event study that writes shock statistics to a file; its single 'fade the first shock' order is a by-product, and the volatility it compares against freezes once its 3000-day array fills, so results depend on where the chart starts, which the book does not fix"), false,
          &["F|TR >= minshk (3) * average true range of the prior vperiod (50) days", "E|first shock: sell at market after an up close, buy at market after a down close", "X|exit at market when TR <= normal volatility * normfact (1.25)"], ""),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "kfcd_fc_orson" => ("kfcd_volatility", "Kaufman CD volatility systems", "FC Orson low-volatility breakout"),
        "kfcd_high_volume" => ("kfcd_volume", "Kaufman CD volume filters", "High volume"),
        "kfcd_low_volume" => ("kfcd_volume", "Kaufman CD volume filters", "Low volume"),
        "kfcd_volume_filters" => ("kfcd_volume", "Kaufman CD volume filters", "Volume indicator divergence"),
        "kfcd_martingales" | "kfcd_antimartingales1" | "kfcd_antimartingales2" => ("kfcd_betting", "Kaufman CD betting strategies", "Sizing"),
        "kfcd_equity_trend" | "kfcd_liquidity" => ("kfcd_risk", "Kaufman CD risk control", "Sizing"),
        "kfcd_shock_study" => ("kfcd_events", "Kaufman CD event study", "Shock"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(match id {
        "kfcd_fc_orson" => vec![p("length", "length: volatility and breakout period", 20.0, 5.0, 60.0, 1.0),
            p("filter", "filter: enter only while vola (0-100) is below", 20.0, 5.0, 50.0, 5.0)],
        "kfcd_high_volume" => vec![p("option", "option: 0 none, 1 entry filter, 2 exit filter, 3 both", 3.0, 0.0, 3.0, 1.0),
            p("length", "length: volume period (price MA = length/2)", 70.0, 20.0, 140.0, 10.0),
            p("factor", "factor: high volume = average + 2*factor*sd", 1.0, 0.5, 2.0, 0.25)],
        "kfcd_low_volume" => vec![p("options", "options: 1 entry filter, 2 exit filter, 3 both", 1.0, 1.0, 3.0, 1.0),
            p("length", "length: volume period (price MA = length/2)", 70.0, 20.0, 140.0, 10.0),
            p("nlow", "nlow: short volume average", 5.0, 2.0, 20.0, 1.0),
            p("factor", "factor: low volume = average - factor*sd", 1.0, 0.5, 2.0, 0.25)],
        "kfcd_volume_filters" => vec![p("period", "period", 20.0, 5.0, 60.0, 1.0),
            p("ttype", "ttype: 1 OBV, 2 accumulator, 3 NVI, 4 PVI, 5 PVT, 6 oscillator, 7 ROC, 8 low volume", 1.0, 1.0, 8.0, 1.0),
            p("kopt", "Kopt: 1 = count volume only on large price changes", 0.0, 0.0, 1.0, 1.0),
            p("maxdn", "maxdn", 0.0, -5.0, 5.0, 0.5),
            p("maxdvg", "maxdvg", 0.0, 0.0, 5.0, 0.5),
            p("npsd", "npsd", 0.5, 0.0, 2.0, 0.1),
            p("ddays", "ddays", 3.0, 1.0, 10.0, 1.0)],
        "kfcd_martingales" | "kfcd_antimartingales1" | "kfcd_antimartingales2" | "kfcd_equity_trend" | "kfcd_liquidity" | "kfcd_shock_study" => vec![],
        _ => return None,
    })
}

fn mean(v: &[f64], i: usize, n: usize) -> f64 {
    if n == 0 || i + 1 < n { return f64::NAN; }
    v[i + 1 - n..=i].iter().sum::<f64>() / n as f64
}
fn sd_pop(v: &[f64], i: usize, n: usize) -> f64 {
    let m = mean(v, i, n);
    if !m.is_finite() { return f64::NAN; }
    (v[i + 1 - n..=i].iter().map(|x| (x - m).powi(2)).sum::<f64>() / n as f64).sqrt()
}
fn hi(v: &[f64], i: usize, n: usize) -> f64 { if i + 1 < n { f64::NAN } else { v[i + 1 - n..=i].iter().cloned().fold(f64::NEG_INFINITY, f64::max) } }
fn lo(v: &[f64], i: usize, n: usize) -> f64 { if i + 1 < n { f64::NAN } else { v[i + 1 - n..=i].iter().cloned().fold(f64::INFINITY, f64::min) } }

/// Position book: one position at a time, reversals close and open on the same bar.
struct Book { pos: Option<(f64, usize, f64, &'static str)>, out: Vec<Trade> }
impl Book {
    fn dir(&self) -> f64 { self.pos.map_or(0.0, |p| p.0) }
    fn close(&mut self, i: usize, px: f64, why: &str) {
        if let Some((d, e, epx, kind)) = self.pos.take() {
            self.out.push(Trade { dir: d, legs: vec![Leg { i: e, px: epx, w: 1.0 }], exit_i: i, exit_px: px, reason: why.into(),
                entry_kind: kind, signal_i: i, open: false, exits: Vec::new() });
        }
    }
    fn open(&mut self, d: f64, i: usize, px: f64, kind: &'static str) { self.pos = Some((d, i, px, kind)); }
    fn finish(mut self, x: &Ctx, from: usize, to: usize) -> Vec<Trade> {
        if let Some((d, e, epx, kind)) = self.pos.take() {
            self.out.push(Trade { dir: d, legs: vec![Leg { i: e, px: epx, w: 1.0 }], exit_i: to, exit_px: x.c[to], reason: "open".into(),
                entry_kind: kind, signal_i: to, open: true, exits: Vec::new() });
        }
        self.out.into_iter().filter(|t| t.legs[0].i >= from).collect()
    }
}

/// FC Orson. Stop orders on the TradeStation bar path (open, nearer extreme, farther extreme, close).
fn fc_orson(x: &Ctx, to: usize, len: usize, filter: f64) -> Book {
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let lx: Vec<f64> = (0..=to).map(|i| if i == 0 { f64::NAN } else { (c[i] / c[i - 1]).ln() }).collect();
    let fcv: Vec<f64> = (0..=to).map(|i| if i >= len { (mean(&lx, i, len) + sd_pop(&lx, i, len)) * 252f64.sqrt() } else { 0.0 }).collect();
    let mut b = Book { pos: None, out: Vec::new() };
    // orders for the next bar: (is_buy_side, level, enters) ; buy side = buy entry or exitshort
    let mut orders: Vec<(bool, f64, bool)> = Vec::new();
    for i in 0..=to {
        if !orders.is_empty() {
            let mut used = vec![false; orders.len()];
            // fill every unused order whose level lies between a and b on the move a -> b (at the open: already through)
            let mut step = |b: &mut Book, a: f64, z: f64, at_open: bool, up: bool| {
                let mut idx: Vec<usize> = (0..orders.len()).filter(|&k| !used[k] && orders[k].0 == up).collect();
                idx.sort_by(|&p, &q| { let (u, v) = (orders[p].1, orders[q].1); if up { u.partial_cmp(&v).unwrap() } else { v.partial_cmp(&u).unwrap() } });
                for k in idx {
                    let (buy, lv, enters) = orders[k];
                    let hit = if at_open { if buy { lv <= z } else { lv >= z } } else if up { lv > a && lv <= z } else { lv < a && lv >= z };
                    if !hit { continue; }
                    used[k] = true;
                    let px = if at_open { z } else { lv };
                    let d = if buy { 1.0 } else { -1.0 };
                    if b.dir() == -d { b.close(i, px, if enters { "Reversed by stop entry" } else { "Exit stop" }); }
                    if enters && b.dir() == 0.0 { b.open(d, i, px, "stop"); }
                }
            };
            step(&mut b, o[i], o[i], true, true);
            step(&mut b, o[i], o[i], true, false);
            let high_first = h[i] - o[i] <= o[i] - l[i];
            let (p1, p2) = if high_first { (h[i], l[i]) } else { (l[i], h[i]) };
            for (a, z) in [(o[i], p1), (p1, p2), (p2, c[i])] { step(&mut b, a, z, false, z >= a); }
        }
        orders.clear();
        if i < len + 60 { continue; }
        let (fh, fl) = (hi(&fcv, i, 60), lo(&fcv, i, 60));
        let v1 = (fh - fl) * 0.01;
        let vola = if v1 != 0.0 { (fcv[i] - fl) / v1 } else { 0.0 };
        let (hh, ll) = (hi(h, i, len), lo(l, i, len));
        if vola < filter { orders.push((true, hh, true)); orders.push((false, ll, true)); }
        if b.dir() > 0.0 { orders.push((false, ll, false)); }
        if b.dir() < 0.0 { orders.push((true, hh, false)); }
    }
    b
}

/// Daily volume; a missing day is 0, as TradeStation sees a bar without volume.
fn volumes(x: &Ctx, to: usize) -> Vec<f64> { (0..=to).map(|i| x.r[i].volume.filter(|v| v.is_finite()).unwrap_or(0.0)).collect() }
/// `adjvol`: a zero-volume day takes the last non-zero volume (savevol); the code's "replace high volume days" line
/// sets adjvol to savevol, which already equals today's volume, so it changes nothing and is not repeated here.
fn adjvol(vol: &[f64]) -> Vec<f64> {
    let mut save = 0.0;
    vol.iter().map(|&v| { if v != 0.0 { save = v; } save }).collect()
}

/// TSM High Volume (option, length, factor).
fn high_volume(x: &Ctx, to: usize, option: i64, length: usize, factor: f64) -> Book {
    let c = &x.c;
    let vol = volumes(x, to);
    let adj = adjvol(&vol);
    let half = length / 2;
    let mut b = Book { pos: None, out: Vec::new() };
    let mut position = 0i32;
    for i in 1..=to {
        let (vavg, vsd) = (mean(&adj, i, length), sd_pop(&adj, i, length));
        let up = vavg + 2.0 * factor * vsd;
        let (m, m1) = (mean(c, i, half), mean(c, i - 1, half));
        if !(up.is_finite() && m.is_finite() && m1.is_finite()) { continue; }
        if m > m1 && position == -1 { position = 0; }
        if m < m1 && position == 1 { position = 0; }
        let volok = option == 0 || option == 2 || vol[i] < up;
        if m > m1 && volok && position != 1 { if b.dir() < 0.0 { b.close(i, c[i], "Reversed"); } if b.dir() == 0.0 { b.open(1.0, i, c[i], "close"); } position = 1; }
        if m < m1 && volok && position != -1 { if b.dir() > 0.0 { b.close(i, c[i], "Reversed"); } if b.dir() == 0.0 { b.open(-1.0, i, c[i], "close"); } position = -1; }
        let xf = option == 2 || option == 3;
        if b.dir() > 0.0 && (m < m1 || (xf && c[i] > c[i - 1] && vol[i] > up)) { b.close(i, c[i], if m < m1 { "MA turned down" } else { "High-volume exit" }); }
        if b.dir() < 0.0 && (m > m1 || (xf && c[i] < c[i - 1] && vol[i] > up)) { b.close(i, c[i], if m > m1 { "MA turned up" } else { "High-volume exit" }); }
    }
    b
}

/// TSM Low Volume (options, length, nlow, factor).
fn low_volume(x: &Ctx, to: usize, options: i64, length: usize, nlow: usize, factor: f64) -> Book {
    let c = &x.c;
    let adj = adjvol(&volumes(x, to));
    let half = length / 2;
    let mut b = Book { pos: None, out: Vec::new() };
    for i in 2..=to {
        let lowlimit = mean(&adj, i, length) - factor * sd_pop(&adj, i, length);
        let vavgn = mean(&adj, i, nlow);
        let (m, m1, m2) = (mean(c, i, half), mean(c, i - 1, half), mean(c, i - 2, half));
        if !(lowlimit.is_finite() && vavgn.is_finite() && m2.is_finite()) { continue; }
        let (buy, sell) = if options == 2 { (m > m1 && m1 <= m2, m < m1 && m1 >= m2) } else { (m > m1 && vavgn > lowlimit, m < m1 && vavgn > lowlimit) };
        // orders at the close, in program order: entries, then exits (an exit for the position held before this bar)
        let held = b.dir();
        if buy && held <= 0.0 { if held < 0.0 { b.close(i, c[i], "Reversed"); } b.open(1.0, i, c[i], "close"); }
        if sell && held >= 0.0 && b.dir() >= 0.0 { if b.dir() > 0.0 { b.close(i, c[i], "Reversed"); } b.open(-1.0, i, c[i], "close"); }
        if held != 0.0 && b.dir() == held {
            let profit = held * (c[i] - b.pos.unwrap().2) > 0.0;
            let lowx = options != 1 && profit && vavgn <= lowlimit;
            if held > 0.0 && (m < m1 || lowx) { b.close(i, c[i], if m < m1 { "MA turned down" } else { "Low-volume exit" }); }
            if held < 0.0 && (m > m1 || lowx) { b.close(i, c[i], if m > m1 { "MA turned up" } else { "Low-volume exit" }); }
        }
    }
    b
}

/// TSM Volume Filters, exit harness (ttype 1-8), orders at market = next open.
#[allow(clippy::too_many_arguments)]
fn volume_filters(x: &Ctx, to: usize, period: usize, ttype: i64, kopt: bool, maxdn: f64, maxdvg: f64, npsd: f64, ddays: usize) -> Book {
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let vol = volumes(x, to);
    let nd = ddays.max(1);
    let n = to + 1;
    let pchg: Vec<f64> = (0..n).map(|i| if i == 0 { f64::NAN } else { (c[i] - c[i - 1]).abs() }).collect();
    let mut kv = vec![0.0f64; n];
    for i in 1..n {
        let prev = kv[i - 1];
        let big = !kopt || pchg[i] > npsd * sd_pop(&pchg, i, period);
        kv[i] = match ttype {
            1 => if big { if c[i] > c[i - 1] { prev + vol[i] } else if c[i] < c[i - 1] { prev - vol[i] } else { prev } } else { prev },
            2 => if big { prev + ((c[i] - l[i]) / (h[i] - l[i]) - 0.5) * 2.0 * vol[i] } else { prev },
            3 => if vol[i] < vol[i - 1] && big { prev + (c[i] - c[i - 1]) * prev / c[i - 1] } else if prev == 0.0 { vol[i] } else { prev },
            4 => if vol[i] > vol[i - 1] && big { prev + (c[i] - c[i - 1]) * prev / c[i - 1] } else if prev == 0.0 { vol[i] } else { prev },
            5 => if big { prev + vol[i] * (c[i] - c[i - 1]) / c[i - 1] } else { prev },
            6 => { let a = mean(&vol, i, nd); (a - mean(&vol, i, period)) * 100.0 / a }
            7 => { let k0 = if prev == 0.0 { vol[i] } else { prev }; if i < period { k0 } else if vol[i - period] == 0.0 { prev } else { (vol[i] - vol[i - period]) * 100.0 / vol[i - period] } }
            _ => 0.0,
        };
        if !kv[i].is_finite() { kv[i] = prev; } // 0/0 (e.g. high == low for type 2, zero volume for 6/7): keep the prior value
    }
    let map: Vec<f64> = (0..n).map(|i| mean(c, i, period)).collect();
    let ma: Vec<f64> = (0..n).map(|i| mean(c, i, 2 * period)).collect();
    let mav: Vec<f64> = (0..n).map(|i| mean(&kv, i, period)).collect();
    let md = |i: usize| if i >= nd { ma[i] - ma[i - nd] } else { f64::NAN };
    let mut b = Book { pos: None, out: Vec::new() };
    // pending next-open orders: (exit long, exit short, entry dir)
    let mut pend: Option<(bool, bool, f64)> = None;
    for i in 0..n {
        if let Some((xl, xs, ent)) = pend.take() {
            if (xl && b.dir() > 0.0) || (xs && b.dir() < 0.0) { b.close(i, o[i], "Exit at market"); }
            if ent != 0.0 && b.dir() != ent { if b.dir() != 0.0 { b.close(i, o[i], "Reversed"); } b.open(ent, i, o[i], "open"); }
        }
        if i < nd + 1 { continue; }
        let (mdi, md1) = (md(i), md(i - 1));
        if !(mdi.is_finite() && md1.is_finite()) { continue; }
        let (cu, cd) = (mdi > 0.0 && md1 <= 0.0, mdi < 0.0 && md1 >= 0.0);
        let (mut xl, mut xs, mut ent) = (false, false, 0.0);
        match ttype {
            1..=5 => {
                let px = if map[i - nd] == 0.0 { 0.0 } else { (map[i] - map[i - nd]) * 100.0 / map[i - nd] };
                let vx = if mav[i - nd] == 0.0 || !mav[i - nd].is_finite() { 0.0 } else { (mav[i] - mav[i - nd]) * 100.0 / mav[i - nd] };
                if !(px.is_finite() && vx.is_finite()) { continue; }
                let dvg = if vx * px < 0.0 { px - vx } else { 0.0 };
                let ok = vx >= maxdn && dvg.abs() <= maxdvg;
                if cu { xs = true; if ok { ent = 1.0; } }
                if cd { xl = true; if ok { ent = -1.0; } }
                if vx < maxdn && dvg.abs() > maxdvg { xl = true; xs = true; }
            }
            6 | 7 => {
                let (a, s) = (mean(&kv, i, period), sd_pop(&kv, i, period));
                let low = kv[i] <= a - npsd * s;
                if cu || low { xs = true; }
                if cd || low { xl = true; }
                if cu { ent = 1.0; }
                if cd { ent = -1.0; }
            }
            8 => {
                let low = mean(&vol, i, nd) <= mean(&vol, i, period) * npsd;
                if cu || low { xs = true; }
                if cd || low { xl = true; }
                if cu { ent = 1.0; }
                if cd { ent = -1.0; }
            }
            _ => {}
        }
        if (xl || xs || ent != 0.0) && i < to { pend = Some((xl, xs, ent)); }
    }
    b
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if schema.is_empty() { return Some(vec![]); }
    if from > to || to >= x.c.len() { return Some(vec![]); }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let u = |k: &str| g(k).round().max(1.0) as usize;
    let b = match id {
        "kfcd_fc_orson" => fc_orson(x, to, u("length"), g("filter")),
        "kfcd_high_volume" => high_volume(x, to, g("option").round() as i64, u("length").max(2), g("factor")),
        "kfcd_low_volume" => low_volume(x, to, g("options").round() as i64, u("length").max(2), u("nlow"), g("factor")),
        "kfcd_volume_filters" => volume_filters(x, to, u("period"), g("ttype").round() as i64, g("kopt") >= 0.5, g("maxdn"), g("maxdvg"), g("npsd"), u("ddays")),
        _ => return None,
    };
    Some(b.finish(x, from, to))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn rows(v: &[(f64, f64, f64, f64, f64)]) -> Vec<Row> {
        v.iter().enumerate().map(|(i, &(o, h, l, c, vo))| Row { ticker: "T".into(), date: format!("{:04}-{:02}-{:02}", 2000 + i / 300, 1 + (i / 25) % 12, 1 + i % 25),
            open: o, high: h, low: l, close: c, volume: Some(vo) }).collect()
    }
    /// High Volume, length 10 (price MA of 5). Bar 9: the MA falls, volume 101 < 100.5 + 2*0.5 -> sell on the close (11).
    /// Bar 12: the MA turns up -> cover and buy at 14. Bar 14: up close on volume 1000 > average + 2 sd (~730) -> exit at 16;
    /// no re-entry while the MA keeps rising ("Don't reenter after high volume exit").
    #[test]
    fn high_volume_exit_and_no_reentry() {
        let cl = [20.0, 19.0, 18.0, 17.0, 16.0, 15.0, 14.0, 13.0, 12.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0, 16.0, 15.0];
        let vo = [100.0, 101.0, 100.0, 101.0, 100.0, 101.0, 100.0, 101.0, 100.0, 101.0, 100.0, 101.0, 100.0, 101.0, 1000.0, 100.0, 101.0, 100.0];
        let v: Vec<_> = cl.iter().zip(vo).map(|(&c, v)| (c, c + 0.5, c - 0.5, c, v)).collect();
        let r = rows(&v);
        let x = Ctx::new(&r, &r, &[]);
        let mut p = Params::new();
        p.insert("length".into(), 10.0);
        let t = run("kfcd_high_volume", &x, 0, 17, &p).unwrap();
        let got: Vec<_> = t.iter().map(|t| (t.dir, t.legs[0].i, t.legs[0].px, t.exit_i, t.exit_px)).collect();
        assert_eq!(got, vec![(-1.0, 9, 11.0, 12, 14.0), (1.0, 12, 14.0, 14, 16.0)]);
        assert_eq!(t[1].reason, "High-volume exit");
    }
}
