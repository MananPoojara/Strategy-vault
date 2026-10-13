//! The Best of the Professional Traders Journal: Day Trading (Connors, M. Gordon 1998). Book id "ptjdt".
//! Five PTJ articles, all intraday. The 15-Minute ADX Breakout (futures and equities versions) is mechanical:
//! it is coded here against per-ticker daily series built from intraday bars (extra_data/, see
//! library/ptjdt/build_series.py and implementation-report.md), with the daily ADX/DI filter from daily bars.
//! The S&P Momentum system needs the full 20-minute bar series and has no defined exit; Front Running the
//! S&P's and Torpedoes are discretionary by the book's own words. Those three are catalogued with their rules.
#![allow(unused_imports, dead_code)]
use crate::connors::*;

pub const BOOK: &str = "ptjdt";

const ADX: &str = "ptjdt_adx15_breakout";
const ADX_EQ: &str = "ptjdt_adx15_equities";
const SPM: &str = "ptjdt_sp_momentum";
const FRONT: &str = "ptjdt_front_running";
const TORP: &str = "ptjdt_torpedoes";

/// Daily per-ticker series the ADX breakout reads (definitions: implementation-report.md / build_series.py).
const SERIES: &[&str] = &["OPEN15_HIGH", "OPEN15_LOW", "LONG_FILLS", "LONG_STOPS", "SHORT_FILLS", "SHORT_STOPS", "CLOSE_LAST"];

const ADX_RULES: &[&str] = &[
    "F|\"We will use this method only in the strongest trending markets! Therefore, we want the daily 14-period ADX reading to be above 30 and the 14-period +DI to be greater than the -DI.\" (sells: -DI > +DI)",
    "E|\"Wait for the first 15 minutes of trading to pass.\"",
    "E|\"Place a buy stop one tick above the first 15-minute bar high.\" (sells: a sell stop one tick below the first 15-minute bar low)",
    "X|\"(This is critical.) Place a protective sell stop one tick below or at the first bar 15-minute low.\" (sells: a buy stop one tick above the first-bar high)",
    "X|\"Where you take your profits is a personal choice. Ideally, you will use trailing stops to help lock in the gains ... lock in profits on half the position and let the other half run.\"",
    "X|\"Close all positions out before the end of the day.\" [Not from the book: the exit price is the close of the day's last intraday bar (CLOSE_LAST); exit_at = 1 uses the daily file's close instead, which on NIFTY is NSE's 30-minute weighted average, not a traded price]",
    "X|[Not from the book] Same-bar order: when one intraday bar both fills the entry stop and crosses the protective level, the extreme nearer the bar's open is taken to come first (reproduces Examples 5 and 11); the old 'fill then stop' reading is build_series.py --same-bar fill_first",
    "A|\"If you are stopped out, it is correct to re-enter. These second entries tend to be the best trades.\" / \"I will not enter a trade three times in a day. Two losses are my psychological limit.\"",
];

const ADX_EQ_RULES: &[&str] = &[
    "F|For buys: \"ADX > 30, +DI > -DI\"; for short sales: \"ADX > 30, -DI > +DI\"",
    "F|\"Price must be above $40/share.\"",
    "F|\"100-day historical volatility of 40 percent or higher.\"",
    "E|Entries, stops and the end-of-day exit as in the 15-Minute ADX Breakout Method: buy stop one tick above the first 15-minute bar high (short: sell stop one tick below its low)",
    "X|Protective stop one tick below the first 15-minute bar low (short: one tick above its high); close all positions before the end of the day [Not from the book: at the last intraday bar's close, CLOSE_LAST]",
    "A|Re-enter after a stop-out; at most two losses a day",
];

const SPM_RULES: &[&str] = &[
    "F|\"We will use 20-minute bars for the explanation. Obviously, 5-, 10-, or 15-minute bars can also be used.\"",
    "F|\"If the bars are trading above the 20-period moving average, the trend is up, and if they are trading under the 20-period moving average, the trend is down.\"",
    "F|\"If the trend is up, we will wait for the three-period RSI to move under the 30 level. If the trend is down, we will wait for the three-period RSI to trade above the 70 level.\"",
    "E|\"For buys, when prices are above the 20-period moving average and the RSI is below 30, we will buy 1 tick above the previous bar on the next two bars only. For sells, when the bars are below the 20-period moving average and the RSI is above 70, we will sell 1 tick below the previous bar on the next two bars only.\"",
    "X|\"Risk no more than the size of the previous bar. This means if the previous bars high is 1,072.50 and its low is 1,070.90 and you go long at 1,072.60, your protective stop should be near 1,070.90.\"",
    "X|\"Where you exit is a personal choice. I prefer to use the 2-for-1 Money Management Method (June 1996 PTJ) as it allows you to lock in profits on half and trail your stops on the other half.\"",
];

const FRONT_RULES: &[&str] = &[
    "F|\"I run a scan every three to five days ... I only look at stocks whose price is above $40.\" \"NYSE and NASDAQ stocks are included.\"",
    "F|\"My scan looks for the strongest 12 to 16 stocks available as measured by the 14-day RSI. The stocks with the highest RSI qualify. (I eliminate all merger stocks ...)\"",
    "F|\"When you have your 12 to 16 names, create an index out of them. This index should then be plotted daily and its direction should be compared to the overall market.\"",
    "E|\"The index will front-run the overall market by one to two days. This means if the S&P market is up big and your momentum index is weak, the S&P's will likely be weak within one to two days.\"",
    "A|\"Adjust your list at least once a week.\"",
    "X|\"Most important - Don't trade this blindly or mechanically. It should be used as an indicator to help guide you with your market direction decisions.\" \"No matter how powerful a signal is, always use protective stops!\"",
];

const TORP_RULES: &[&str] = &[
    "F|\"Identify the recent market leaders. These are usually higher priced, big cap names such as Microsoft, IBM, 3COM, Intel, Dell ...\" (there are no short-sale Torpedoes)",
    "F|\"Wait for the stock market to be down at least 1 percent in the first hour of trading. At that time, look for any market leader or highly volatile stock that is defying the trend. This means stocks which are up for the day or down only a small amount.\"",
    "E|\"Here is where it gets a bit subjective. If the overall market begins to bounce back, buy the strongest stock(s) from Rule 2.\"",
    "X|\"Your risk should be only one point from your entry. If you are stopped out, it is OK to re-enter if the stock begins rising again.\"",
    "X|\"Remember to keep your initial stop tight and let your profits run when they occur.\"",
];

const ADX_NOTE: &str = "Intraday rule coded against per-ticker daily series built from intraday bars (library/ptjdt/build_series.py): \
OPEN15_HIGH/LOW = first 15-minute bar; LONG_FILLS/LONG_STOPS (SHORT_*) = how many times the buy stop at OPEN15_HIGH + 1 tick filled (bar high >= it) and the protective stop at OPEN15_LOW - 1 tick hit (bar low <= it), \
re-entering at the same level after every stop-out, no cap; the tick parameter must equal the builder's --tick. Default stop is the book's 'one tick below'; stop_at_low = 1 is the book's other reading 'or at the first bar low' (series *_AT, stop at OPEN15_LOW itself). \
Exit at CLOSE_LAST, the close of the day's last intraday bar [Not from the book; exit_at = 1 = the daily file's close, the old reading]. \
[Not from the book] When one intraday bar both fills and crosses the stop level, the builder takes the extreme nearer the bar's open as first (fits Examples 5 and 11); on real NIFTY 1-minute bars that happens on 0.3% of days. A low below OPEN15_LOW before the fill does not cancel the buy (Example 11). Day N's filter uses the ADX/DI at day N-1's close (known before the open); Wilder DMI, 14. \
Re-entry after a stop-out at the same breakout level, at most max_losses (2) losses a day; the profit-taking ('personal choice', half off, trailing) is not mechanical, so the whole position is held to the close. \
Book tested July 1997 coffee (no stats, 12 dated chart examples); the vault runs it on indices/ETFs (India: NIFTY 50 etc., 09:15-09:30 first bar, tick 0.05) once the series exist.";

const ADX_EQ_NOTE: &str = "Same mechanics and series as the 15-Minute ADX Breakout; adds the stock filter: prior close > price_min ($40 in the book; INR on Indian data, so effectively always true on indices) \
and 100-day HV >= 40% (annualised sample stdev of daily log returns x sqrt(252), at the prior close). The book meant a stock universe scanned daily; the vault runs it on the indices, where HV100 >= 40 is rare, so expect few trades.";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, dir, status, needs_series, rules, notes| StrategyDef {
        id, name, group, session: 0, dir, status, needs_volume: false, needs_vix: false, needs_series, book: BOOK, rules, notes, claim: None,
    };
    vec![
        d(ADX, "15-Minute ADX Breakout", "15-Minute ADX Breakout Method", "Long/Short", Status::Live, SERIES, ADX_RULES, ADX_NOTE),
        d(ADX_EQ, "15-Minute ADX Breakout · equities (HV100 ≥ 40, price > $40)", "Trading the 15-Minute ADX Breakout Method with Equities", "Long/Short", Status::Live, SERIES, ADX_EQ_RULES, ADX_EQ_NOTE),
        d(SPM, "S&P Momentum Day-Trading System", "S&P Momentum Day-Trading System", "Long/Short",
          Status::NotTestable("needs the full 20-minute bar series (SMA20 and RSI3 on 20-minute bars, stop entries off the previous bar); exit is the undefined 2-for-1 Money Management Method"),
          &[], SPM_RULES,
          "Cannot be reduced to per-day values: the setup, the two-bar entry window and the stop all live on the 20-minute bar series. The exit refers to the 2-for-1 Money Management Method (June 1996 PTJ), not defined in this book. Book examples: June 1998 S&P futures, May 4-18 1998, no stats."),
        d(FRONT, "Front Running the S&P's (momentum index)", "Front Running the S&P's", "Long/Short", Status::NotTestable("discretionary; needs a stock universe"), &[], FRONT_RULES,
          "A market-direction indicator, not a system: the book forbids mechanical use and gives no entry or exit. Index = '+' sum (price-weighted) of the 12-16 highest 14-day-RSI stocks over $40, rebuilt weekly. Example: Bloomberg index 'FAST', April 14-21 1998."),
        d(TORP, "Torpedoes (Jeff Cooper)", "Torpedoes", "Long", Status::NotTestable("discretionary; needs intraday stock data and a stock universe"), &[], TORP_RULES,
          "The entry ('if the overall market begins to bounce back') is subjective by the book's own words. Examples: Vitesse (Jan 10 1997), Paccar (Dec 6 1996), Maxim (Dec 19 1997)."),
    ]
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let mut v = vec![
        p("adx_n", "ADX / DI period (daily)", 14.0, 5.0, 30.0, 1.0),
        p("adx_min", "ADX above", 30.0, 15.0, 50.0, 1.0),
        p("tick", "One tick (price units; must equal the series builder's --tick: NSE 0.05, US 0.01)", 0.05, 0.01, 1.0, 0.01),
        p("max_losses", "Losing trades a day before stopping (1 = no re-entry)", 2.0, 1.0, 5.0, 1.0),
        p("stop_at_low", "Protective stop: 0 = one tick beyond the first-bar extreme, 1 = at it (book offers both)", 0.0, 0.0, 1.0, 1.0),
        p("exit_at", "End-of-day exit: 0 = last intraday bar close [not from book], 1 = daily file close", 0.0, 0.0, 1.0, 1.0),
    ];
    match id {
        ADX => {}
        ADX_EQ => v.extend([
            p("price_min", "Price above", 40.0, 0.0, 500.0, 10.0),
            p("hv_n", "Historical volatility period", 100.0, 20.0, 200.0, 10.0),
            p("hv_min", "Historical volatility at least (%)", 40.0, 10.0, 80.0, 5.0),
        ]),
        _ => return None,
    }
    Some(v)
}

/// Wilder DMI returning (+DI, -DI, ADX); same formulas as connors::dmi, which does not expose -DI.
fn dmi3(r: &[crate::Row], n: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let len = r.len();
    let trs = true_range(r);
    let (mut pdm, mut mdm, mut tr) = (vec![f64::NAN; len], vec![f64::NAN; len], vec![f64::NAN; len]);
    for i in 1..len {
        let (up, dn) = (r[i].high - r[i - 1].high, r[i - 1].low - r[i].low);
        pdm[i] = if up > dn && up > 0.0 { up } else { 0.0 };
        mdm[i] = if dn > up && dn > 0.0 { dn } else { 0.0 };
        tr[i] = trs[i];
    }
    let (sp, sm, st) = (wilder(&pdm, n), wilder(&mdm, n), wilder(&tr, n));
    let (mut pdi, mut mdi, mut dx) = (vec![f64::NAN; len], vec![f64::NAN; len], vec![f64::NAN; len]);
    for i in 0..len {
        if st[i] > 0.0 {
            let (p, m) = (100.0 * sp[i] / st[i], 100.0 * sm[i] / st[i]);
            pdi[i] = p;
            mdi[i] = m;
            dx[i] = if p + m > 0.0 { 100.0 * (p - m).abs() / (p + m) } else { 0.0 };
        }
    }
    let adx = wilder(&dx, n);
    (pdi, mdi, adx)
}

/// One day's trades in one direction: (entry px, exit px, stopped?). Every fill is at the breakout level
/// +/- 1 tick, every stop at the opposite first-bar extreme -/+ 1 tick; the last fill that is not stopped
/// exits at the close. `fills`/`stops` count the uncapped re-entry cycle; trading stops after `max_losses` losses.
/// `stop_tick` is the protective stop's offset beyond the opposite extreme: `tick` (one tick) or 0 ("at the low").
fn day_trades(dir: f64, h15: f64, l15: f64, fills: f64, stops: f64, close: f64, tick: f64, stop_tick: f64, max_losses: usize) -> Vec<(f64, f64, bool)> {
    let (entry, stop) = if dir > 0.0 { (h15 + tick, l15 - stop_tick) } else { (l15 - tick, h15 + stop_tick) };
    let (fills, stops) = (fills.max(0.0) as usize, stops.max(0.0) as usize);
    (0..fills.min(max_losses)).map(|k| if k < stops { (entry, stop, true) } else { (entry, close, false) }).collect()
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap().def);
    let u = |k: &str| g(k).round().max(1.0) as usize;
    let (pdi, mdi, adx) = dmi3(x.r, u("adx_n"));
    let hv = (id == ADX_EQ).then(|| if u("hv_n") == 100 { x.hv100.clone() } else { hv(&x.c, u("hv_n")) });
    let s = |k: &str| x.series(k);
    let at = g("stop_at_low") >= 0.5;
    let sfx = if at { "_AT" } else { "" };
    let (Some(h15), Some(l15), Some(lf), Some(ls), Some(sf), Some(ss)) = (
        s("OPEN15_HIGH"),
        s("OPEN15_LOW"),
        s(&format!("LONG_FILLS{sfx}")),
        s(&format!("LONG_STOPS{sfx}")),
        s(&format!("SHORT_FILLS{sfx}")),
        s(&format!("SHORT_STOPS{sfx}")),
    ) else {
        return Some(vec![]);
    };
    let close: &[f64] = if g("exit_at") >= 0.5 { &x.c } else { match s("CLOSE_LAST") { Some(c) => c, None => return Some(vec![]) } };
    let (adx_min, tick, cap) = (g("adx_min"), g("tick"), u("max_losses"));
    let stop_tick = if at { 0.0 } else { tick };
    let mut out = Vec::new();
    for i in from.max(1)..=to {
        let k = i - 1; // filter at the prior close, known before the open
        if !(adx[k] > adx_min) {
            continue;
        }
        if let Some(hv) = &hv {
            if !(x.c[k] > g("price_min") && hv[k] >= g("hv_min")) {
                continue;
            }
        }
        let (dir, fills, stops) = if pdi[k] > mdi[k] {
            (1.0, lf[i], ls[i])
        } else if mdi[k] > pdi[k] {
            (-1.0, sf[i], ss[i])
        } else {
            continue;
        };
        if h15[i].is_nan() || l15[i].is_nan() || fills.is_nan() || stops.is_nan() || close[i].is_nan() {
            continue;
        }
        for (px, exit_px, stopped) in day_trades(dir, h15[i], l15[i], fills, stops, close[i], tick, stop_tick, cap) {
            out.push(Trade {
                dir,
                legs: vec![Leg { i, px, w: 1.0 }],
                exit_i: i,
                exit_px,
                reason: if stopped { "protective stop (first-bar extreme)".into() } else { "end of day".into() },
                entry_kind: "stop",
                signal_i: i,
                open: false, exits: Vec::new()
            });
        }
    }
    Some(out)
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id {
        ADX => Some(("ptjdt_adx15", "15-Minute ADX Breakout", "Futures / index (book)")),
        ADX_EQ => Some(("ptjdt_adx15", "15-Minute ADX Breakout", "Equities filter (HV100 ≥ 40, price > $40)")),
        SPM => Some(("ptjdt_spm", "S&P Momentum Day-Trading", "20-minute bars")),
        FRONT => Some(("ptjdt_front", "Front Running the S&P's", "Momentum index")),
        TORP => Some(("ptjdt_torp", "Torpedoes", "Jeff Cooper")),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// July 1997 coffee, tick 0.05 cents, $375 per cent. Exact where the book prints the prices; "~" = read off the chart.
    #[test]
    fn coffee_july_1997_examples() {
        let t = 0.05;
        let near = |a: f64, b: f64| (a - b).abs() < 1e-9;
        // Ex 4 (Apr 30): "buy stop is filled at 202.10 and the sell stop is near 200.40"; > 8 cents held to the close (~210.2 on the chart).
        let v = day_trades(1.0, 202.05, 200.45, 1.0, 0.0, 210.20, t, t, 2);
        assert_eq!(v.len(), 1);
        assert!(near(v[0].0, 202.10) && near(v[0].1, 210.20) && !v[0].2);
        assert!(v[0].1 - v[0].0 > 8.0 && (v[0].1 - v[0].0) * 375.0 > 3000.0);
        // Ex 11 (May 9): the second-bar low (below the first-bar low) comes before the fill -> still filled
        // (the chart's bar 2 dips to 214.72 and trades through 217.50 in the same bar, open nearer the low:
        // build_series.py's open_nearest order counts it as fill only; LONG_STOPS counts only stops after a fill). "filled at 217.50 ... closes at 217.90": a scratch.
        let v = day_trades(1.0, 217.45, 215.00, 1.0, 0.0, 217.90, t, t, 2);
        assert!(v.len() == 1 && near(v[0].0, 217.50) && near(v[0].1 - v[0].0, 0.40));
        // Ex 13 (May 13): "Filled at 234.10 and it closes for a better than $2,600 profit" (close ~241.5 on the chart).
        let v = day_trades(1.0, 234.05, 230.0, 1.0, 0.0, 241.50, t, t, 2);
        assert!(near(v[0].0, 234.10) && (v[0].1 - v[0].0) * 375.0 > 2600.0);
        // Ex 14 (May 14): "Filled at 246.10. Stopped out at 245": first-bar high 246.05, low 245.05.
        let v = day_trades(1.0, 246.05, 245.05, 1.0, 1.0, 239.0, t, t, 2);
        assert!(v.len() == 1 && near(v[0].0, 246.10) && near(v[0].1, 245.00) && v[0].2);
        // Ex 5 (May 1): filled, stopped for ~2 cents, re-entered, close ~10 cents above the entry.
        let v = day_trades(1.0, 213.0, 211.1, 2.0, 1.0, 223.0, t, t, 2);
        assert!(v.len() == 2 && v[0].2 && !v[1].2);
        assert!(((v[0].1 - v[0].0) + 2.0).abs() < 0.25 && near(v[1].1 - v[1].0, 9.95));
        // Ex 8 (May 6): two 2-cent losses; the 12:00 bar takes out the first-bar high a third time
        // (and drops through the low in the same bar) but "I will not enter a trade three times in a day".
        let v = day_trades(1.0, 218.6, 216.7, 3.0, 3.0, 213.5, t, t, 2);
        assert!(v.len() == 2 && v.iter().all(|x| x.2 && near(x.1 - x.0, -2.0)));
        // Ex 6 / 9: buy stop never filled -> no trade.
        assert!(day_trades(1.0, 230.0, 228.0, 0.0, 0.0, 225.0, t, t, 2).is_empty());
        // Shorts are reversed: sell one tick under the low, stop one tick over the high.
        let v = day_trades(-1.0, 100.0, 99.0, 1.0, 1.0, 95.0, t, t, 2);
        assert!(near(v[0].0, 98.95) && near(v[0].1, 100.05) && v[0].2);
        // max_losses = 1 disables re-entry.
        assert_eq!(day_trades(1.0, 213.0, 211.1, 2.0, 1.0, 223.0, t, t, 1).len(), 1);
        // "or at the first bar low" (stop_at_low = 1): stop exactly at the first-bar extreme.
        let v = day_trades(1.0, 246.05, 245.05, 1.0, 1.0, 239.0, t, 0.0, 2);
        assert!(near(v[0].1, 245.05) && v[0].2);
        let v = day_trades(-1.0, 100.0, 99.0, 1.0, 1.0, 95.0, t, 0.0, 2);
        assert!(near(v[0].0, 98.95) && near(v[0].1, 100.0));
    }
}
