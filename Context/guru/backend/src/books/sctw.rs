//! Shay Campbell (Timing Charts, LLC), "Trend Weaver Trading System" (2006): Trend Weaver Trading Rules.pdf (10 pp)
//! plus the system's own TradeStation code (Code.rar: "TradeStation - 8.txt" / "TradeStation 2000i.txt",
//! "(c) Copyright 2006"), and "Trend Weaver Performance 1980-2006.xls" (TS summary reports per futures market).
//! Book id `sctw`. Source: research drive "USB HDD 1/TradeStation Systems & Indicators/Shay Campbell - Trend Weaver
//! Trading System (timingcharts.com)/". The code is the exact statement and wins over the prose where they differ
//! (trend test: code `high > Top_Range` with Top_Range = 99% of the 150-bar range INCLUDING today; prose p.3 says
//! "today's high is greater than the high of the last 150 days").
//! Order handling is TradeStation's: all orders are next-bar stops computed on the close; the trend-change stop
//! (Top_Range / Bottom_Range) is an opposite-side entry that reverses the position; entries in the direction already
//! held are ignored (TS default: no pyramiding); exits only exist when the position was held at the close.
//! Several stops on one daily bar fill in TradeStation's bar-path order: open, then the extreme nearer the open
//! (high on a tie), then the other extreme; an order already through the open fills at the open first. Each order
//! fires at most once per bar and only if it still applies to the position at the moment it is touched.
use crate::connors::{claim, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "sctw";
/// Admitted ids (authors/campbell/admission/SOURCE-CONTRACT.md).
pub const REVIEWED: &[&str] = &["sctw_trend_weaver"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef {
        id: "sctw_trend_weaver", name: "Trend Weaver (150-day trend, 20-day range stops)",
        group: "Trend Weaver Trading Rules pp.3-9 + TradeStation code", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
        rules: &[
            "F|Trend (code): Top_Range = Lowest(Low,150) + (Highest(High,150) - Lowest(Low,150)) * .99; Bottom_Range = ... * .01; \"If high > Top_Range then TrendDirection = 1; if low < Bottom_Range then TrendDirection = -1\" (prose p.3: \"If today's high is greater than the high of the last 150 days, the trend is UP ... The trend will stay either UP or DOWN until the opposite position is proven true\")",
            "F|TR = Highest(H,20) - Lowest(L,20) (p.6 \"TR = high [20] - low [20]\"); all orders are \"added to the lowest price of the last 20 days\" (p.8)",
            "E|Trend UP: \"Buy next bar at Long_Entry_Price stop\", Long_Entry_Price = Lowest(L,20) + 90% of TR (p.8 \"Buy stop = 90% of TR\")",
            "E|Trend DOWN: \"Sell short Next Bar at Short_Entry_Price stop\", Lowest(L,20) + 10% of TR (p.8 \"Sell stop = 10% of TR\")",
            "E|Trend change (p.5 \"placing stop orders 1% inside the 150 day range\"): trend DOWN -> \"Buy next bar at Top_Range stop\" (150-day low + 99% of the 150-day range); trend UP -> \"Sell short Next Bar at Bottom_Range stop\" (150-day low + 1%); these reverse an open position",
            "X|Long: \"Sell next bar at LongExit stop\", LongExit = Lowest(L,20) + 40% of TR (p.8 \"Long exit = 40% of TR\")",
            "X|Short: \"Buy to Cover Next Bar at ShortExit stop\", ShortExit = Lowest(L,20) + 60% of TR (p.8 \"Short exit = 60% of TR\")",
            "F|Published (xls, TS summary, 1 contract per market, 1/1/1980-1/1/2007, TRENDWEAVER(150,20,0.1,0.9,0.4,0.6)): 37-market consolidated 6,087 trades, 36.75% profitable, avg trade $213.16, PF 1.40; Nikkei 225 futures (NK) 98 trades, 44.9%, avg $595.66, PF 1.52. No S&P/Dow/NIFTY market was tested.",
        ],
        notes: "Futures system (diversified portfolio) run on the cash index as an instrument swap: percentage returns, one unit, flat at the selected start; earlier bars warm the 150-day trend state. Positions only open from orders placed on or after the start bar; the last position stays open (marked to market). Fill and sequencing rules are TradeStation's daily-bar conventions (no Look-Inside-Bar): stops fill at the stop or at the open if gapped through; multiple stops on one bar in bar-path order (open -> extreme nearer the open, high on a tie -> other extreme). The published futures results cannot be compared: the vault has no futures and none of the 37 tested markets.",
        claim: claim("6,087 (37 futures)", "36.75%", "$213.16", "winners 32 bars / losers 10 bars", "1/1/1980-1/1/2007, futures portfolio, 1 contract"),
    }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == "sctw_trend_weaver").then_some(("sctw_trend_weaver", "Trend Weaver", "Book code defaults"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    (id == "sctw_trend_weaver").then(|| vec![
        p("trend_len", "TrendLength (bars)", 150., 50., 300., 25.),
        p("trade_len", "TradeLength (bars)", 20., 10., 60., 5.),
        p("long_pct", "LongPercent of TR (buy stop)", 90., 60., 100., 5.),
        p("short_pct", "ShortPercent of TR (sell stop)", 10., 0., 40., 5.),
        p("exit_long_pct", "ExitLong_Percent of TR", 40., 10., 70., 5.),
        p("exit_short_pct", "ExitShort_Percent of TR", 60., 30., 90., 5.),
        p("change_pct", "Trend-change stop % inside the 150-day range", 1., 0., 5., 0.5),
    ])
}

/// Bar-path time at which a stop is touched (None = not touched). 0 = at the open (gap fill at the open).
/// `buy` = stop above the market (buy / buy-to-cover), else sell stop.
pub(crate) fn touch(o: f64, h: f64, l: f64, level: f64, buy: bool) -> Option<f64> {
    let high_first = h - o <= o - l;
    let frac = |a: f64, b: f64| if b > 0.0 { a / b } else { 0.0 };
    if buy {
        if level <= o { return Some(0.0); }
        if level > h { return None; }
        Some(if high_first { 1.0 + frac(level - o, h - o) } else { 2.0 + frac(level - l, h - l) })
    } else {
        if level >= o { return Some(0.0); }
        if level < l { return None; }
        Some(if high_first { 2.0 + frac(h - level, h - l) } else { 1.0 + frac(o - level, o - l) })
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Kind { BuyEntry, ShortEntry, ExitLong, ExitShort }

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap().def);
    let (tl, rl) = (g("trend_len").round().max(1.0) as usize, g("trade_len").round().max(1.0) as usize);
    let (lp, sp, xl, xs, cp) = (g("long_pct") / 100., g("short_pct") / 100., g("exit_long_pct") / 100., g("exit_short_pct") / 100., g("change_pct") / 100.);
    let n = x.c.len();
    let to = to.min(n.saturating_sub(1));
    let hh = |i: usize, k: usize| x.h[i + 1 - k..=i].iter().copied().fold(f64::MIN, f64::max);
    let ll = |i: usize, k: usize| x.l[i + 1 - k..=i].iter().copied().fold(f64::MAX, f64::min);
    let mut out = Vec::new();
    let mut trend = 0i8;
    let mut pos: Option<Trade> = None;
    let start = tl.max(rl) - 1;
    for t in start..to {
        // orders computed on bar t's close, for bar t+1
        let (l150, h150) = (ll(t, tl), hh(t, tl));
        let top = l150 + (h150 - l150) * (1.0 - cp);
        let bottom = l150 + (h150 - l150) * cp;
        if x.h[t] > top { trend = 1; }
        if x.l[t] < bottom { trend = -1; }
        if t < from { continue; }
        let (l20, tr) = (ll(t, rl), hh(t, rl) - ll(t, rl));
        let held = pos.as_ref().map_or(0.0, |q| q.dir);
        let mut orders: Vec<(Kind, f64)> = Vec::new();
        if trend == 1 { orders.push((Kind::BuyEntry, l20 + tr * lp)); }
        if trend == -1 { orders.push((Kind::ShortEntry, l20 + tr * sp)); }
        if trend == -1 { orders.push((Kind::BuyEntry, top)); }
        if trend == 1 { orders.push((Kind::ShortEntry, bottom)); }
        if held > 0.0 { orders.push((Kind::ExitLong, l20 + tr * xl)); }
        if held < 0.0 { orders.push((Kind::ExitShort, l20 + tr * xs)); }
        let j = t + 1;
        let (o, h, l) = (x.o[j], x.h[j], x.l[j]);
        let mut ev: Vec<(f64, usize, Kind, f64)> = orders.iter().enumerate().filter_map(|(k, &(kind, lvl))| {
            let buy = matches!(kind, Kind::BuyEntry | Kind::ExitShort);
            touch(o, h, l, lvl, buy).map(|tm| (tm, k, kind, if tm == 0.0 { o } else { lvl }))
        }).collect();
        ev.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap().then(a.1.cmp(&b.1)));
        for (_, _, kind, px) in ev {
            let cur = pos.as_ref().map_or(0.0, |q| q.dir);
            let (close_it, open_dir, why) = match kind {
                Kind::BuyEntry if cur <= 0.0 => (cur < 0.0, 1.0, "Long entry stop reverses short"),
                Kind::ShortEntry if cur >= 0.0 => (cur > 0.0, -1.0, "Short entry stop reverses long"),
                Kind::ExitLong if cur > 0.0 => (true, 0.0, "LongExit stop (40% of 20-day range)"),
                Kind::ExitShort if cur < 0.0 => (true, 0.0, "ShortExit stop (60% of 20-day range)"),
                _ => continue,
            };
            if close_it {
                let mut q = pos.take().unwrap();
                q.exit_i = j; q.exit_px = px; q.signal_i = t; q.reason = why.into();
                out.push(q);
            }
            if open_dir != 0.0 {
                pos = Some(Trade { dir: open_dir, legs: vec![Leg { i: j, px, w: 1.0 }], exit_i: j, exit_px: px, reason: String::new(),
                    entry_kind: "stop", signal_i: t, open: false, exits: vec![] });
            }
        }
    }
    if let Some(mut q) = pos {
        q.exit_i = to; q.exit_px = x.c[to]; q.signal_i = to; q.reason = "open".into(); q.open = true;
        out.push(q);
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn bar_path_and_gaps() {
        // open nearer the low -> low first
        assert_eq!(touch(100., 110., 98., 100., true), Some(0.0));
        assert!(touch(100., 110., 98., 105., true).unwrap() > 2.0);
        assert!(touch(100., 110., 98., 99., false).unwrap() < 2.0);
        assert_eq!(touch(100., 110., 98., 111., true), None);
    }
    #[test]
    fn enters_long_on_buy_stop_and_exits_on_long_exit() {
        // 160 flat bars 100-110, then a breakout bar sets trend UP; next bar trades through the 90% buy stop, then
        // a bar falls through the 40% exit.
        let mut r: Vec<Row> = (0..200).map(|i| Row { ticker: "T".into(), date: format!("{:04}-01-01", 1800 + i), open: 105., high: 110., low: 100., close: 105., volume: None }).collect();
        r[160] = Row { high: 112., low: 101., close: 111., open: 106., ..r[160].clone() }; // trend -> 1 (112 > 99% level)
        // bar 161: open 111 >= buy stop 100 + .9*12 = 110.8 -> fills at the open
        r[161] = Row { open: 111., high: 113., low: 110.5, close: 112., ..r[161].clone() };
        // bar 163: 20-day low 100, TR 13 -> LongExit 105.2; open 104 below -> exit at the open
        r[162] = Row { open: 112., high: 113., low: 111., close: 112., ..r[162].clone() };
        r[163] = Row { open: 104., high: 104.5, low: 103., close: 104., ..r[163].clone() };
        let x = Ctx::new(&r, &r, &[]);
        let ts = run("sctw_trend_weaver", &x, 160, 165, &Params::new()).unwrap();
        assert_eq!(ts[0].dir, 1.0);
        assert_eq!(ts[0].legs[0].i, 161);
        assert_eq!(ts[0].legs[0].px, 111.);
        assert_eq!(ts[0].exit_i, 163);
        assert_eq!(ts[0].exit_px, 104.);
    }
}
