//! Mark W. Helweg & David C. Stendahl, *Dynamic Trading Indicators* (Wiley 2002), ch.6 "Designing Trading Systems with
//! Value Charts". Source: research drive USB HDD 2/tRADING KNO-2/Mark W.Helweg, David C.Stendahl - Dynamic Trading
//! Indicators/Dynamic Trading Indicators by Mark W. Helweg.pdf (scanned two-up, read from page images); copy in
//! Books/Mark W. Helweg/. Book id "hwdti". Notes: library/hwdti/README.md.
use crate::books::apwms::wma;
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "hwdti";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        StrategyDef {
            id: "hwdti_ma_sar", name: "Moving average trend-following system: WMA(20) x SMA(56), stop-and-reverse at the close", group: "Ch.6 Analyzing a Treasury Bonds trend-following trading system (Figure 6.4)",
            session: 0, dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
            rules: &[
                "F|\"Waverage = Weighted Moving Average; Average = Simple Moving Average\" (Figure 6.4, p.130)",
                "E|\"(Buy Signal) If waverage (close, 20) crosses above average (close, 56), then buy one contract at the close.\" (Figure 6.4)",
                "E|\"(Sell Signal) If waverage (close, 20) crosses below average (close, 56), then sell one contract at the close.\" (Figure 6.4)",
                "X|\"This trading system is a stop-and-reverse system. In other words, if the trading system is long the Treasury bonds market and generates a sell signal, it will exit the long position and initiate a short position.\" (p.130-131)",
            ],
            notes: "Cross = WMA20 above SMA56 today and not above yesterday (mirror for below); the first trade is the first cross after both averages exist. WMA = linear weights 1..20. Book test: continuous back-adjusted T-bond futures 22 Aug 1977-31 Dec 1999, Table 6.1 (p.137): 105 trades, 38.1% profitable, net $63,407.40, average trade $603.88 - a bond-futures result, not comparable with an index. India: instrument swap; index shorts are hypothetical.",
            claim: None,
        },
        StrategyDef {
            id: "hwdti_value_chart_sar", name: "Enhanced moving average system with Value Charts (Figure 6.9)", group: "Ch.6 Enhancing a Treasury Bonds trend-following trading system with Value Charts",
            session: 0, dir: "Long/Short", status: Status::NotTestable("The entry needs the bar's Value Chart price at or below -2 (buy) / at or above +2 (sell), but the Value Chart floating axis and volatility interval include the current, still-forming bar; the book says 'we assume that the small degree of error is negligible' (p.140), so a daily-bar fill cannot be computed without look-ahead, and how long the -2/+2 order stays open after the crossover is not stated"),
            needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
            rules: &[
                "F|\"Value Chart Floating Axis = average((high + low)/2,5); Interval = the greater of the two: Close - Close[1] ... or High - Low ...; Volatility Intervals = average (interval,5) x (0.2)\" (Figure 6.9, p.136)",
                "E|\"(Buy Signal) If waverage (close, 20) crosses above average (close, 56), then buy one contract when the Value Chart price trades at or below the -2 Value Chart price level.\" (Figure 6.9)",
                "E|\"(Sell Signal) If waverage (close, 20) crosses below average (close, 56), then sell one contract when the Value Chart price trades at or above the +2 Value Chart price level.\" (Figure 6.9)",
            ],
            notes: "Book: Table 6.2 (p.141) 105 trades, 41.9% profitable, net $91,461.72 on T-bond futures.",
            claim: None,
        },
        StrategyDef {
            id: "hwdti_crude_30min", name: "Crude oil Value Chart strategy (Figure 5.4)", group: "Ch.5",
            session: 0, dir: "Long/Short", status: Status::NeedsData("30-minute crude oil futures bars (intraday)"),
            needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
            rules: &["F|Value Chart levels on 30-minute crude oil bars (ch.5)"],
            notes: "Intraday futures data not in the vault.",
            claim: None,
        },
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id {
        "hwdti_ma_sar" | "hwdti_value_chart_sar" => Some(("hwdti_ma", "Helweg WMA20 x SMA56 trend system", if id == "hwdti_ma_sar" { "Figure 6.4" } else { "Figure 6.9 Value Chart entry" })),
        "hwdti_crude_30min" => Some(("hwdti_crude", "Helweg crude oil Value Chart", "30-minute")),
        _ => None,
    }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    (id == "hwdti_ma_sar").then(|| vec![p("fast", "Weighted MA of the close (bars)", 20., 5., 60., 1.), p("slow", "Simple MA of the close (bars)", 56., 20., 200., 2.)])
}

fn open(dir: f64, i: usize, px: f64) -> Trade {
    Trade { dir, legs: vec![Leg { i, px, w: 1. }], exit_i: i, exit_px: px, reason: String::new(), entry_kind: "close", signal_i: i, open: false, exits: vec![] }
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if id != "hwdti_ma_sar" || from >= to || to >= x.r.len() { return None; }
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d).max(1.) as usize;
    let (w, s) = (wma(&x.c, g("fast", 20.)), crate::connors::sma(&x.c, g("slow", 56.)));
    let mut out = vec![];
    let mut cur: Option<Trade> = None;
    for i in from.max(1)..=to {
        if !(w[i].is_finite() && s[i].is_finite() && w[i - 1].is_finite() && s[i - 1].is_finite()) { continue; }
        let up = w[i] > s[i] && !(w[i - 1] > s[i - 1]);
        let dn = w[i] < s[i] && !(w[i - 1] < s[i - 1]);
        let want = if up { 1. } else if dn { -1. } else { continue };
        if let Some(mut t) = cur.take() {
            if t.dir == want { cur = Some(t); continue; }
            t.exit_i = i; t.exit_px = x.c[i]; t.signal_i = i;
            t.reason = if want > 0. { "WMA20 crossed above SMA56 · reverse at the close" } else { "WMA20 crossed below SMA56 · reverse at the close" }.into();
            out.push(t);
        }
        cur = Some(open(want, i, x.c[i]));
    }
    if let Some(mut t) = cur { t.exit_i = to; t.exit_px = x.c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true; out.push(t); }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn reverses_at_the_close_of_the_cross_bar() {
        let mut r: Vec<Row> = (0..200).map(|i| Row { ticker: "T".into(), date: format!("{:04}", i), open: 100., high: 101., low: 99., close: 100. - i as f64 * 0.1, volume: None }).collect();
        for k in 100..200 { r[k].close = 90. + (k - 100) as f64 * 0.5; }
        let x = Ctx::new(&r, &r, &[]);
        let ts = run("hwdti_ma_sar", &x, 0, 199, &Params::new()).unwrap();
        let (w, s) = (wma(&x.c, 20), crate::connors::sma(&x.c, 56));
        let c = (56..200).find(|&i| w[i] > s[i] && !(w[i - 1] > s[i - 1])).unwrap();
        assert_eq!(ts[0].dir, 1.); assert_eq!(ts[0].legs[0].i, c); assert_eq!(ts[0].legs[0].px, x.c[c]);
    }
}
