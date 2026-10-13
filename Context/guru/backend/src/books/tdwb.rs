//! Tom DeMark, "Trading Indicators for the 21st Century" workbooks 1-6 (Market Studies, Inc. 1997; traderslibrary.com
//! video course). Book id "tdwb". Source copy: Books/Thomas DeMark/Trading Indicators for the 21st Century Workbook/
//! Tom DeMark 1..6.pdf (image scans). Page refs "WBn <section> p" = workbook n, the section's printed page.
//! The only rule here with an exact entry AND exit is the fade of a DISQUALIFIED TD Line break (WB5 TD Points & TD
//! Lines p.7 and p.14): "the position should be closed at the end of the trading bar". Everything else in the
//! workbooks is an indicator description without an exit (or duplicates tdperl / tdnsta / tdnmtt); see the README.
#![allow(dead_code)]
use crate::connors::*;

pub const BOOK: &str = "tdwb";

const FADE_S: &[&str] = &[
    "F|TD Point High (Level 1): \"a price bar whose HIGH is ABOVE both the preceding bar's and succeeding bar's HIGHS\" (WB5 TD Points & TD Lines p.1); known after the succeeding bar closes",
    "F|TD Supply Line: \"Starting with the two most recent TD Point Highs, a DESCENDING line from left to right is formed and projected into the future\" (p.3). Default: the most recent TD Point High and the nearest earlier TD Point High that is higher (DeMark's definition, so the line descends); p.pair=1 = the two most recent only, skipped when not descending [labelled option]",
    "F|Qualifier #1: \"the bar before the breakout closes LOWER than the close two bars before the breakout\" (p.11)",
    "F|Qualifier #2: \"the market OPENS ABOVE the TD Supply Line and ABOVE the previous bar's close and trades ABOVE the open by one or two price ticks\" (p.12)",
    "F|Qualifier #3: (close - low) of the previous bar \"is added to today's CLOSE. If this value is BELOW the TD Supply Line, then Qualifier #3 is met\" (p.13) = C[-1] + (C[-1] - L[-1]) < line at the breakout bar",
    "E|\"When NONE of the three Qualifiers are met and a TD Supply Line is penetrated, the line becomes Disqualified indicating the current trend will continue and a low risk SELL opportunity exists.\" (p.14) Sell at the line when the high first exceeds it; a gap open above the line sells at the open. [Book silent, necessary] when the bar opens above the line AND above the previous close, Qualifier #2 cannot be ruled out at the moment of penetration, so no trade",
    "X|\"If a low risk SELL is placed with the penetration of a Disqualified TD Supply Line, the position should be closed at the end of the trading bar or followed closely with a tight stop loss.\" (p.14) Default = closed at the close of the entry bar (the tight stop has no size)",
];
const FADE_L: &[&str] = &[
    "F|TD Point Low (Level 1): \"a price bar whose LOW is BELOW both the preceding bar's and the succeeding bar's LOWS\" (WB5 TD Points & TD Lines p.1)",
    "F|TD Demand Line: from the two most recent TD Point Lows \"an ASCENDING line from left to right is formed\" (p.3). Default: the most recent TD Point Low and the nearest earlier lower TD Point Low; p.pair=1 = the two most recent only [labelled option]",
    "F|Qualifier #1: \"the bar before the breakout closes HIGHER than the close two bars before the breakout\" (p.4)",
    "F|Qualifier #2: \"the market OPENS BELOW the TD Demand Line and BELOW the previous bar's close and trades below the open by one or two price ticks\" (p.5)",
    "F|Qualifier #3: \"the difference between the HIGH and the CLOSE of the current bar ... subtracted from that day's CLOSE. If this value is ABOVE the TD Demand Line, then Qualifier #3 is met\" (p.6) = C[-1] - (H[-1] - C[-1]) > line at the breakout bar",
    "E|\"When NONE of the three Qualifiers are met and a TD Demand Line is penetrated, the line becomes Disqualified indicating that the current trend will continue and a low risk BUY (NOT sell) may be placed.\" (p.7) Buy at the line when the low first falls below it; a gap open below the line buys at the open. [Book silent, necessary] an open below the line AND below the previous close leaves Qualifier #2 open at the penetration: no trade",
    "X|\"If a low risk BUY entry is placed with the penetration of a Disqualified TD Demand Line, the position should be closed at the end of the trading bar or followed closely with a tight stop loss.\" (p.7) Default = sold at the close of the entry bar",
];

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    let d = |id, name, group, dir, status, rules: &'static [&'static str], notes| StrategyDef {
        id, name, group, session: 0, dir, status, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None,
    };
    let nt = |id, name, group, why, rules| d(id, name, group, "Long/Short", NotTestable(why), rules, "");
    vec![
        d("tdwb_tdline_fade_short", "Disqualified TD Supply Line break · sell, out on the close", "WB5 · TD Points & TD Lines", "Short", Live, FADE_S,
          "Day trade fading a disqualified upside break of the TD Supply Line. Points and lines from daily bars; each line (pair of points) trades only its first penetration [book silent, necessary]. The qualified breakout of the same lines (with its three cancellations and price projection, p.8-10, 15-18) duplicates tdperl_tdline_long/short and is not repeated here. The workbook prints no statistics. India: instrument swap only."),
        d("tdwb_tdline_fade_long", "Disqualified TD Demand Line break · buy, out on the close", "WB5 · TD Points & TD Lines", "Long", Live, FADE_L,
          "Mirror of tdwb_tdline_fade_short. India: instrument swap only."),
        nt("tdwb_open_trap_clop_clopwin", "TD Open, TD Trap, TD Clop, TD ClopWin (workbook 1)", "WB1",
           "the workbook gives the entry patterns and how often 'indications closed below the previous high' (S&P 1996-97 tallies, TD Open p.7-12) but no exit; the same patterns are in tdperl (clop/clopwin) and tdnmtt (trap, with DeMark's close exit)",
           &["E|TD Open: \"When a current trading bar's OPEN is BELOW the previous trading bar's LOW, and price trades ABOVE that LOW, a potential low risk BUY indication is given\" (WB1 TD Open p.1; 2- and 5-bars-back variants p.2-3)",
             "E|TD Trap: open within the previous bar's range and price trades above the high / below the low (TD Trap p.1)",
             "E|TD Clop: open below the previous open and close, then trades one tick above both (TD Clop p.1); TD ClopWin: open and close within the previous open-close, entry the NEXT bar (TD ClopWin p.1-3)"]),
        nt("tdwb_sequential", "TD Sequential · entry after Countdown and the 'minimum price' stop (workbook 2)", "WB2",
           "entry (\"a possible time ... the first bar that closes ABOVE its OPEN\", p.23) and a protective stop (p.22, 25) are given, but no profit exit: \"it is important to apply other TD indicators to initiate entries\" (p.8). DeMark's own profit level is coded in tdnsta/tdnmtt",
           &["F|Setup: 9 consecutive closes below (above) the close 4 bars earlier after a price flip (WB2 TD Sequential p.1-4); Intersection p.5; Countdown close <= low (>= high) 2 bars earlier, 13 bars, bar 13 vs close of bar 8 (p.6-8, 17-18); recycling options p.10-16",
             "E|\"after a BUY Countdown is complete, a possible time to enter a trade is on the first bar that closes ABOVE its OPEN\" (p.23)",
             "X|stop: the lowest Countdown bar's low minus its true range; \"If the market closes beyond this price, the low risk BUY zone is cancelled\" (p.22, 25); no profit exit"]),
        nt("tdwb_tdst_combo", "TDST and TD Combo (workbook 3)", "WB3",
           "TDST is a trend-confirmation level (\"a low risk BUY zone is indicated once price closes ABOVE the TDST Level ... confirmed when price opens ABOVE\", TD ST p.1) with no exit; the TD Combo pages define the countdown only (TD Combo p.1-5). Duplicates tdnmtt_tdst / tdnmtt_combo*",
           &[]),
        nt("tdwb_trend_factor", "TD Trend Factor (workbook 4)", "WB4", "support/resistance projections (x 0.9444 / 1.0556 from an initialised high/low, TD Trend Factor p.1-8); no entry or exit", &[]),
        nt("tdwb_double_range", "TD Double Range (workbook 5)", "WB5", "a low-risk zone (close -/+ 2 x true range, TD Double Range p.1-2) with no exit", &[]),
        nt("tdwb_diff", "TD Diff (workbook 5)", "WB5", "buying/selling-pressure zones after two down/up closes (TD Diff p.1-4) with no exit; tdperl_diff_* is the vault's version", &[]),
        nt("tdwb_ma_channels", "TD Moving Average 1, TD Channel 1 and 2 (workbook 6)", "WB6",
           "TDMA1 (5-bar low average plotted 4 bars after a low above the 12 previous lows) is \"effective as a trailing stop loss indicator\" (TD Moving Average 1 p.6), no entry; TD Channels (3-bar high/low averages x 97%/103%, 99.5%/100.5%) only say a close beyond the channel 'may' bring a reversal (TD Channel 1 p.3-5, TD Channel 2 p.3)", &[]),
        nt("tdwb_rei_poq_plurality", "TD REI with TDPOQ, TD Plurality (workbook 6)", "WB6", "low-risk zones from the oscillators (TD REI p.1-4, TD Plurality p.1-3) with no exit; tdperl_rei duplicates REI/POQ", &[]),
        nt("tdwb_rebo_spring", "TD REBO, TD Spring (workbook 6)", "WB6",
           "breakout levels (open +/- 61.8% of the prior range; close +/- 61.8% of the recent 5-bar true range when it is < 50% of the prior 13-bar range) with no exit and filters left open (\"various filters can be applied\", TD REBO p.2); tdperl_rebo is the vault's REBO", &[]),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "tdwb_tdline_fade_short" => ("tdwb_tdline_fade", "DeMark workbook · disqualified TD Line fade", "Sell (supply line)"),
        "tdwb_tdline_fade_long" => ("tdwb_tdline_fade", "DeMark workbook · disqualified TD Line fade", "Buy (demand line)"),
        _ if id.starts_with("tdwb_") => ("tdwb_other", "DeMark workbooks · indicators", "Not testable"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    match id {
        "tdwb_tdline_fade_short" | "tdwb_tdline_fade_long" => Some(vec![
            p("pair", "Line points: 0 = most recent + nearest earlier higher (lower) point (DeMark); 1 = the two most recent only [labelled option]", 0., 0., 1., 1.),
        ]),
        _ => None,
    }
}

/// Disqualified TD Line fade. `s` = +1 supply line (sell the upside break), -1 demand line (buy the downside break).
/// Prices are mirrored (x -> -x) for the demand line so one code path serves both.
fn fade(x: &Ctx, from: usize, to: usize, sup: bool, pair: bool) -> Vec<Trade> {
    let n = x.c.len();
    let to = to.min(n - 1);
    let m = if sup { 1. } else { -1. };
    // mirrored series: for the demand line highs become -lows, lows become -highs
    let (hi, lo): (Vec<f64>, Vec<f64>) = if sup { (x.h.clone(), x.l.clone()) } else { (x.l.iter().map(|v| -v).collect(), x.h.iter().map(|v| -v).collect()) };
    let (o, c): (Vec<f64>, Vec<f64>) = (x.o.iter().map(|v| m * v).collect(), x.c.iter().map(|v| m * v).collect());
    let mut pts: Vec<usize> = vec![]; // confirmed point highs (mirrored)
    let mut spent: Option<(usize, usize)> = None;
    let mut out = vec![];
    for i in 2..=to {
        // point at i-2 confirmed by the close of i-1
        let j = i - 2;
        if j >= 1 && hi[j] > hi[j - 1] && hi[j] > hi[j + 1] { pts.push(j); }
        let Some(&p2) = pts.last() else { continue };
        let p1 = if pair {
            if pts.len() < 2 { continue; }
            let q = pts[pts.len() - 2];
            if hi[q] <= hi[p2] { continue; }
            q
        } else {
            match pts.iter().rev().skip(1).find(|&&q| hi[q] > hi[p2]) { Some(&q) => q, None => continue }
        };
        if spent == Some((p1, p2)) { continue; }
        let line = |k: usize| hi[p2] + (hi[p2] - hi[p1]) / (p2 - p1) as f64 * (k as f64 - p2 as f64);
        let v = line(i);
        if !(hi[i] > v) { continue; }
        spent = Some((p1, p2));
        if i < from { continue; }
        let q1 = c[i - 1] < c[i - 2];
        let q3 = c[i - 1] + (c[i - 1] - lo[i - 1]) < v;
        if q1 || q3 { continue; } // qualified (or possibly qualified) break: not this trade
        let px = if o[i] > v {
            if o[i] > c[i - 1] { continue; } // Qualifier #2 cannot be ruled out at the penetration
            o[i]
        } else { v };
        let dir = -m; // supply: sell; demand: buy
        out.push(Trade { dir, legs: vec![Leg { i, px: m * px, w: 1. }], exit_i: i, exit_px: x.c[i], reason: "close of the entry bar".into(),
            entry_kind: "stop", signal_i: i, open: false, exits: vec![] });
    }
    out
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    Some(match id {
        "tdwb_tdline_fade_short" => fade(x, from, to, true, g("pair") >= 0.5),
        "tdwb_tdline_fade_long" => fade(x, from, to, false, g("pair") >= 0.5),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn bar(i: usize, o: f64, h: f64, l: f64, c: f64) -> Row { Row { ticker: "T".into(), date: format!("d{:04}", i), open: o, high: h, low: l, close: c, volume: None } }
    #[test]
    fn supply_fade_hand_checked() {
        // point highs at bar 1 (h 110) and bar 4 (h 106): line slope -4/3; at bar 7 = 106 - 4 = 102.
        // bar 5: c 100, bar 6: c 101 (> c5 so Q1 false), l6 = 98 -> Q3: 101 + 3 = 104 >= 102 false? 104 < 102 is false -> not met.
        // bar 7 opens 100.5 (< line, Q2 false), high 103 > 102 -> sell at 102, cover at the close 101.
        let v = vec![
            bar(0, 100., 101., 99., 100.), bar(1, 105., 110., 104., 108.), bar(2, 104., 105., 100., 101.),
            bar(3, 101., 103., 100., 102.), bar(4, 102., 106., 101., 104.), bar(5, 103., 104., 99., 100.),
            bar(6, 100., 101.5, 98., 101.), bar(7, 100.5, 103., 100., 101.),
        ];
        let x = Ctx::new(&v, &v, &[]);
        let t = run("tdwb_tdline_fade_short", &x, 0, 7, &Params::new()).unwrap();
        assert_eq!(t.len(), 1);
        assert_eq!(t[0].legs[0].i, 7);
        assert!((t[0].legs[0].px - 102.).abs() < 1e-9 && t[0].exit_px == 101. && t[0].dir == -1.);
    }
}
