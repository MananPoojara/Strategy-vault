//! John Sweeney, "Campaign Trading: Tactics and Strategies to Exploit the Markets" (Wiley, 1996).
//! Source copy: Books/John Sweeney/Campaign Trading.pdf. Notes: library/jswct/README.md.
//! Only the chapter-2 two-moving-average trend rule is complete; every other tactic depends on a
//! crude-oil MAE stop level (.31 $/bbl etc.), an undefined "projected" average value, or judgement.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

const BOOK: &str = "jswct";
/// Admitted ids (authors/lanea/singles/decisions.json).
pub const REVIEWED_LANEA: &[&str] = &["jswct_trend2ma"];
pub fn reviewed_lanea(id: &str) -> bool { REVIEWED_LANEA.contains(&id) }
const MAE: &str = "level is a crude-oil MAE stop (.31 trading points = $310/contract, fitted on 1984-94 crude); the book gives no index value and says stops must be derived per market, so any index number would be invented";

pub fn catalogue() -> Vec<StrategyDef> {
    let make = |id, name, group, dir, status, rules: &'static [&'static str], notes| StrategyDef {
        id, name, group, session: 0, dir, status, needs_volume: false, needs_vix: false,
        needs_series: &[], book: BOOK, rules, notes, claim: None,
    };
    vec![
        make("jswct_trend2ma", "Two moving averages: 12-day and 58-day both up / both down", "Ch 2 · Trading the Trend", "Long/Short", Status::Live,
            &["F|Short average 12 days, long average 12 weeks = \"60 trading days\" (p.37); the averages used in the exemplary trading are \"12-day and 58-day simple moving averages\" (Appendix p.227; also p.157). Default 58, p.long=60 is the other printed value. Averages of the close [book silent on the price field]",
              "F|\"'up' is a one-period increase in an average's value and 'down' is a one-period decrease in the average's value.\" (p.38)",
              "E|\"If both are moving up, we go long at the close; if both down, short at the close.\" (p.38)",
              "X|\"If in disagreement, we're out at the close.\" (p.38) An unchanged average is neither up nor down, so it counts as disagreement.",
              "X|\"No stops are used.\" (p.39, Fig 2.10 test). Non-default p.mae_stop > 0: \"place a stop at [N] ticks opposite the direction of the trade\" (p.55) in index points from the entry; after a stop the next trade needs a fresh agreement [book silent]."],
            "Book: CSI perpetual New York Light Crude, 1983/84-1994, one contract, no costs (p.37-39, 61). Published: equity curve only (Fig 2.10), 252 trend trades (p.61), in a trend 1,530 of 2,800 days = 55% (p.153 fn); with a .31 MAE stop 3,600 points (Table 5.1). Crude results are not comparable to an index. Run on the index instead of crude (instrument swap). The .31 stop is crude-specific, so the default is the chapter-2 no-stop rule."),
        make("jswct_addon_day", "Add-on day trades at the short average", "Ch 5 · Piling On", "Long/Short",
            Status::NotTestable("entry is a stop at the average's \"projected value\", which the book leaves to judgement (\"Any imaginative analyst ... I usually extend the five-day trend of changes\", p.131 fn); a buy stop at a level price must fall to is not a defined order; results only with crude MAE stops"),
            &["F|An underlying trend trade is under way (p.111)", "E|\"a buy or sell stop was placed at the day's projected value for the shorter average (12 days)\" (p.111)", "X|\"the trade was closed at the end of the day\" (p.111)"],
            "Book: crude 1984-94, 2,500 points with a .31 stop (Table 5.1)."),
        make("jswct_addon_position", "Add-on position trades at the long average", "Ch 5 · Piling On", "Long/Short",
            Status::NotTestable("entry at the long average's undefined \"projected value\" and \"when the low or high exceeded the average's value by two or more points\" (crude ticks, p.131); no index equivalent"),
            &["F|An underlying trend trade is under way (p.130)", "E|Buy or sell stop at the day's projected value of the longer average; entered when the low/high exceeded it by two or more points (p.130-131)", "X|Closed at the end of the underlying trade (p.131)"],
            "Book: crude 1984-94, 640 points with a .31 stop (Table 5.1)."),
        make("jswct_reversal", "Reverse bad trend trades at the MAE stop", "Ch 6 · Reversing Bad Trades", "Long/Short",
            Status::NotTestable(MAE),
            &["E|\"If long or short, place a reversing stop (sell or buy two contracts) at .31 trading points.\" (p.141)", "X|\"place no protective stop. Exit on the end-of-trade date for the underlying trade.\" (p.141)"],
            "Book: crude 1984-94, 68 winners / 31 losers (p.142), 1,827 points (Table 7.1)."),
        make("jswct_range", "Trade into trading ranges", "Ch 7 · Switching Modes: Trading Ranges", "Long/Short",
            Status::NotTestable("range sides are a \"guesstimate\" and a reaction high/low the book does not define; \"A choice exists in declaring a range\"; rules \"specific to the example of Crude oil\" (p.161-163)"),
            &["F|Trend of at least six days, broken by price crossing an average; lowest/highest close of the trend = one side (p.161)", "F|Reaction high/low within 12 days, then price recrosses the average: range declared (p.161-163)", "E|Trade into the range at the two levels (p.163)", "X|Exit at the opposite side, after six days of breakout, or when trend is re-established (p.163); .3 MAE stop (p.167)"],
            "Book: crude 1984-94, 550 points (Table 7.1)."),
        make("jswct_range_reverse", "Reverse out of ranges", "Ch 8 · Reversing Out of Ranges", "Long/Short",
            Status::NotTestable("needs the chapter-7 range, which is not mechanical, and a .51 crude MAE reversal level (p.182)"),
            &["E|Reverse a range trade that hits its MAE stop (.31 / .5 crude points) (p.173-182)"],
            "Book: crude 1984-94, 35-38 trades, 2,229 points at .51 (Table 8.1)."),
        make("jswct_minfe", "Minimum favorable excursion stop management", "Ch 9 · Minimum Favorable Excursion", "Long/Short",
            Status::NotTestable("author: \"I haven't found hard and fast rules using this information\" (p.197); breakeven at MaxFE > .3 is offered as \"an artful call\" in crude points"),
            &["X|Move the stop to breakeven if MaxFE > .3 while MinFE < .3 (an \"artful call\", p.197)"], "No firm rule."),
        make("jswct_options", "Skipping stops using options", "Ch 10 · Shifting the Odds", "Long/Short",
            Status::NotTestable("needs option chains"),
            &["E|Use a long option instead of the MAE stop (p.205-222)"], "Options."),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let v = match id {
        "jswct_trend2ma" => "Trend: two moving averages",
        "jswct_addon_day" => "Add-on day trades",
        "jswct_addon_position" => "Add-on position trades",
        "jswct_reversal" => "Reversal at MAE stop",
        "jswct_range" => "Range trading",
        "jswct_range_reverse" => "Reversing out of ranges",
        "jswct_minfe" => "MinFE stop management",
        "jswct_options" => "Options instead of stops",
        _ => return None,
    };
    Some(("jswct_campaign", "Sweeney campaign trading", v))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    match id {
        "jswct_trend2ma" => Some(vec![
            p("short", "Short SMA days", 12., 5., 30., 1.),
            p("long", "Long SMA days (58 = appendix, 60 = 12 weeks)", 58., 40., 80., 1.),
            p("mae_stop", "MAE stop in price points from entry (0 = none, the book's ch.2 test) [non-default option]", 0., 0., 1000., 1.),
        ]),
        _ => None,
    }
}

fn close(mut t: Trade, i: usize, px: f64, reason: &str) -> Trade {
    t.exit_i = i;
    t.exit_px = px;
    t.signal_i = i;
    t.reason = reason.into();
    t
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if id != "jswct_trend2ma" || from > to || to >= x.r.len() {
        return None;
    }
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap().def);
    let (ns, nl, stop) = (g("short").round().max(1.) as usize, g("long").round().max(1.) as usize, g("mae_stop").max(0.));
    // One-period change of an n-day SMA = (c[i] - c[i-n]) / n: its sign is taken from the closes so an
    // unchanged average is exactly 0 (a running-sum SMA can leave float dust). NaN compares false -> 0.
    let d = |i: usize, n: usize| if i >= n { x.c[i] - x.c[i - n] } else { f64::NAN };
    let st = |i: usize| -> i8 {
        let (s, l) = (d(i, ns), d(i, nl));
        if s > 0. && l > 0. { 1 } else if s < 0. && l < 0. { -1 } else { 0 }
    };
    let mut out = vec![];
    let mut cur: Option<Trade> = None;
    let mut stopped = 0i8; // direction stopped out of; wait for the state to leave it
    for i in from..=to {
        let s = st(i);
        if let Some(t) = cur.as_ref() {
            if stop > 0. && i > t.legs[0].i {
                let lvl = t.legs[0].px - t.dir * stop;
                let hit = if t.dir > 0. { x.l[i] <= lvl } else { x.h[i] >= lvl };
                if hit {
                    let px = if t.dir > 0. { x.o[i].min(lvl) } else { x.o[i].max(lvl) };
                    stopped = t.dir as i8;
                    out.push(close(cur.take().unwrap(), i, px, "MAE stop"));
                }
            }
        }
        if let Some(t) = cur.as_ref() {
            if s as f64 != t.dir {
                let why = if s == 0 { "Averages disagree · close" } else { "Both averages reversed · close" };
                out.push(close(cur.take().unwrap(), i, x.c[i], why));
            }
        }
        if s != stopped { stopped = 0; }
        if cur.is_none() && s != 0 && s != stopped && i < to {
            cur = Some(Trade { dir: s as f64, legs: vec![Leg { i, px: x.c[i], w: 1. }], exit_i: i, exit_px: x.c[i],
                reason: String::new(), entry_kind: "close", signal_i: i, open: false, exits: vec![] });
        }
    }
    if let Some(t) = cur {
        let mut t = close(t, to, x.c[to], "open");
        t.open = true;
        out.push(t);
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn rows(c: &[f64]) -> Vec<Row> {
        c.iter().enumerate().map(|(i, &c)| Row { ticker: "T".into(), date: format!("{:05}", i), open: c, high: c + 1., low: c - 1., close: c, volume: None }).collect()
    }
    #[test]
    fn long_when_both_rise_out_on_disagreement_short_when_both_fall() {
        // 3-bar / 5-bar SMAs on a rise, a one-bar dip, then a long fall
        let c: Vec<f64> = (0..10).map(|i| 100. + i as f64).chain([105.]).chain((0..10).map(|i| 104. - 2. * i as f64)).collect();
        let r = rows(&c);
        let x = Ctx::new(&r, &r, &[]);
        let p = Params::from([("short".into(), 3.), ("long".into(), 5.)]);
        let ts = run("jswct_trend2ma", &x, 0, r.len() - 1, &p).unwrap();
        assert_eq!(ts[0].dir, 1.);
        assert_eq!(ts[0].legs[0].i, 5); // first bar where both 3- and 5-SMA rose
        assert_eq!(ts[0].exit_i, 10); // 3-SMA turns down on the dip
        assert_eq!(ts[0].exit_px, 105.);
        assert!(ts.iter().any(|t| t.dir == -1.));
        for t in &ts { assert_eq!(t.legs[0].px, x.c[t.legs[0].i]); }
    }
    #[test]
    fn mae_stop_exits_and_waits_for_a_new_agreement() {
        let c: Vec<f64> = (0..12).map(|i| 100. + i as f64).collect();
        let mut r = rows(&c);
        r[8].low = 90.; r[8].open = 106.; // entry at 5 (close 105), stop 10 points -> level 95, fill 95
        let x = Ctx::new(&r, &r, &[]);
        let p = Params::from([("short".into(), 3.), ("long".into(), 5.), ("mae_stop".into(), 10.)]);
        let ts = run("jswct_trend2ma", &x, 0, 11, &p).unwrap();
        assert_eq!(ts.len(), 1);
        assert_eq!((ts[0].exit_i, ts[0].exit_px), (8, 95.));
    }
}

// Instrument admission: John Sweeney (jswct, lane A) 2026-10-26 (lane B) block, moved from instrument_policy.rs on 31 Oct 2026 (lane ST).
// Admission only for the module REVIEWED lists (authors/lanea/singles/decisions.json); kept in one block to merge.
// Runs on actual indices, ETFs and stocks, US and India (index_etf_or_stock, nifty50 basket, declared application).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book=="jswct" && crate::books::jswct::reviewed_lanea(def.id),
    label: "Sweeney's two-average trend rule was written for New York light crude futures; Runs on indices, ETFs and single stocks (US and India) are a declared application, one unit per trade.",
    basis: "John Sweeney / Campaign Trading (1996)",
};
