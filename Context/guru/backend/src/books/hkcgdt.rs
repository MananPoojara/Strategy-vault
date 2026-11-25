//! Markus Heitkoetter, "The Complete Guide to Day Trading" (BookSurge 2008; text PDF, printed pages).
//! Book id `hkcgdt`. Source and checks: library/hkcgdt/. A how-to-build-a-strategy book: the "popular
//! trading approaches" (p.135-146) are entry signals only ("Please note that we just defined an entry
//! signal. You still need to apply profit targets and stop losses", p.139), with "e.g." parameters on
//! intraday charts; the exit chapter (p.151-170) is a menu of stop/target types. Lane R1-1116 (R4 #339/#343) runs five of the entry signals with labelled unstated details (daily bars, stop-and-reverse); the rest stay held.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "hkcgdt";

macro_rules! nt {
    ($id:expr, $name:expr, $why:expr, $rules:expr) => {
        StrategyDef { id: $id, name: $name, group: "Part 2 · Your Trading Strategy", session: 0, dir: "Long/Short", status: Status::NotTestable($why),
            needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules: $rules,
            notes: "Read in the text layer (library/hkcgdt/README.md). Meant for intraday charts (10-, 15-, 60-minute examples).", claim: None }
    };
}

#[allow(dead_code)]
const NO_EXIT: &str = "entry signal only: 'You still need to apply profit targets and stop losses' (p.139); parameters are examples ('e.g.'), and no exit is given; shown on short-period charts";

const LIVE_IDS: &[&str] = &["hkcgdt_turtle20", "hkcgdt_wpr", "hkcgdt_rsi", "hkcgdt_bollinger"];
const N_TURTLE: &str = "[R1-1116, R4 #297/#339/#343: adapted, 3 unstated details, UNVERIFIED] Unstated in the book: (1) the window: 'the previous 20 bars' (p.139) = the 20 days before today, today excluded; (2) 'current prices move higher than the high' read on daily bars as the close above the highest high (below the lowest low for a sell); (3) the sell signal reverses to short (no stop or target values in the book). Reads highs and lows, so SPX, DJI and NDX results start where their real opens begin (clamp_open). Engine conventions, labelled and not counted: a signal on a bar fills at that bar's close, one position at a time, indicators start at their first full lookback. Declared application: the book shows these signals on 10-minute and other short charts; the vault runs daily bars of indices, ETFs and stocks. The book prints no result, market or test period.";
const N_WPR: &str = "[R1-1116, R4 #297/#339/#343/#344: adapted, 3 unstated details, UNVERIFIED] Printed on p.142 in words (no symbols): the formula takes the close against the lowest low of the last X days, divided by the highest high minus the lowest low, giving a value between 0 and 100 that is high when the close is high in the range; 14 bars; above 80 is a sell signal, below 20 a buy signal. Unstated: (1) the sign of the printed wording ('subtracts the close from the lowest low') read as 100 x (close - lowest low) / (highest high - lowest low), which is the printed 0-100 range with a high value when the close is high; (2) the window of 14 includes today; (3) a sell signal reverses to short. Holding the position until the opposite signal is part of the printed reverse rule (R4 #344), not counted. Reads highs and lows (clamp_open). Engine conventions, labelled and not counted: a signal on a bar fills at that bar's close, one position at a time, indicators start at their first full lookback. Declared application: the book shows these signals on 10-minute and other short charts; the vault runs daily bars of indices, ETFs and stocks. The book prints no result, market or test period.";
const N_RSI: &str = "[R1-1116, R4 #297/#339/#343/#344: adapted, 3 unstated details, UNVERIFIED] Stated: 14 periods, a sell when the RSI crosses the 70 line from above, a buy when it crosses 30 from below (p.144). Unstated: (1) the RSI definition: Wilder's, 100 x U/(U+D) with U and D smoothed averages of up and down closes, new = (prev x (n-1) + x)/n, seeded with the mean of the first n changes (https://en.wikipedia.org/wiki/Relative_strength_index, copy in authors/relax-339/R1/w/defs/); (2) a 'cross' = the previous RSI strictly beyond the line and today's on or inside it; (3) a sell reverses to short; (4) a day with no cross keeps the position. Engine conventions, labelled and not counted: a signal on a bar fills at that bar's close, one position at a time, indicators start at their first full lookback. Declared application: the book shows these signals on 10-minute and other short charts; the vault runs daily bars of indices, ETFs and stocks. The book prints no result, market or test period.";
const N_BB: &str = "[R1-1116, R4 #297/#339/#343/#344: adapted, 5 unstated choices (honest count, R4 #400/#408; more than 4), UNVERIFIED] Stated: 21-bar moving average, 2 standard deviations, a buy when prices move below the lower band, a sell above the upper (p.145). Unstated: (1) the standard deviation: population form (divide by n), Bollinger's own convention (https://en.wikipedia.org/wiki/Bollinger_Bands gives no divisor, copy in authors/relax-339/R1/w/defs/); (2) 'prices move' = the close; (3) a sell reverses to short; (4) the window of 21 includes today; (5) daily bars instead of the book's short charts. Further engine conventions, labelled: a signal on a bar fills at that bar's close, one position at a time, indicators start at their first full lookback. Declared application: the book shows these signals on 10-minute and other short charts; the vault runs daily bars of indices, ETFs and stocks. The book prints no result, market or test period.";

pub fn catalogue() -> Vec<StrategyDef> {
    let live = |id, name, rules: &'static [&'static str], notes: &'static str| StrategyDef { id, name, group: "Part 2 · Your Trading Strategy", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None };
    vec![
        nt!("hkcgdt_ma_close", "Close crosses a moving average", "over 4 unstated choices under R4 #343: the period ('such as a 5- or 10-day', p.136), simple versus other average, whether the average includes today, what counts as a cross, the short reversal, plus the 'trending markets only' caution (p.137) that has no measure",
            &["E|\"A buy signal is generated when the closing price moves above the moving average. A sell signal is generated when the closing price dips below the moving average.\" (p.136); period: \"such as a 5- or 10-day\" (p.136); \"only apply it in trending markets\" (p.137)"]),
        nt!("hkcgdt_ma_cross", "Fast / slow moving-average crossover (e.g. 14 / 20)", "the printed direction is contradictory: 'a buy signal is generated when the fast-moving average crosses the slow moving average from above' (p.138), the opposite of the usual rule, and the example chart is not readable; reading either way is a further vault choice on top of the unstated parameters (count over 4)",
            &["E|\"a 'fast' moving average (e.g. 14 bars) and a 'slow' moving average (e.g. 20 bars)\"; \"A buy signal is generated when the fast-moving average crosses the slow moving average from above\" (p.138, wording as printed)"]),
        live("hkcgdt_turtle20", "20-bar high / low breakout", &["E|\"A buy signal is generated when the current prices move higher than the high of the previous 20 bars.\" (p.139)", "X|\"A sell signal is generated when the current prices move lower than the low of the previous 20 bars.\" (p.139)"], N_TURTLE),
        nt!("hkcgdt_macd", "MACD signal-line crossover (12/26/9)", "the printed direction is contradictory: a buy when 'the signal line crosses the MACD from below' (p.141) is the usual sell, and the chart is not readable; reading either way is a further vault choice (count over 4 with the MACD definition, short side and window)",
            &["E|\"A buy signal is generated when the signal line ... crosses the MACD ... from below\" (p.141)"]),
        live("hkcgdt_wpr", "Williams %R 80 / 20", &["E|\"A buy signal is generated when the indicator has a value below 20.\" (p.142), 14 bars", "X|\"A sell signal is generated when the indicator has a value above 80.\" (p.142)"], N_WPR),
        live("hkcgdt_rsi", "RSI 70 / 30 crossings", &["E|\"A buy signal is generated when the RSI crosses the 30-line ... from below.\" (p.144)", "X|\"A sell signal is generated when the RSI crosses the 70-line ... from above.\" (p.144)"], N_RSI),
        live("hkcgdt_bollinger", "Bollinger band fade (21, 2)", &["E|\"A buy signal is generated when prices move below the lower Bollinger Band.\" (p.145)", "X|\"A sell signal is generated when prices move above the upper Bollinger Band.\" (p.145)"], N_BB),
        nt!("hkcgdt_exits", "Stop-loss, profit-target, trailing and time-stop menu", "a menu of exit types with example values ($ amount, 1% of price, 50% of ATR, support/resistance, 3x the bar interval), not tied to an entry, so there is no trade rule (p.151-170)",
            &["X|\"Stop loss and profit-taking exit rules can be expressed in four ways\" (p.151)"]),
        nt!("hkcgdt_300_200", "$300 target / $200 stop strategy", "the entry rules of the strategy are not given in this book (p.12-14)",
            &["X|\"a profit target of $300 per contract and a stop loss of $200 per contract\" (p.12)"]),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    if LIVE_IDS.contains(&id) { return catalogue().into_iter().find(|d| d.id == id).map(|d| (d.id, d.name, "")); }
    id.starts_with("hkcgdt_").then_some(("hkcgdt_nt", "Heitkoetter Complete Guide (not testable)", "-"))
}
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let (def, min, max, label) = match id {
        "hkcgdt_ma_close" => (10., 2., 250., "Average days (book: 5 or 10)"),
        "hkcgdt_turtle20" => (20., 2., 250., "Breakout days (book: 20)"),
        "hkcgdt_wpr" => (14., 2., 250., "%R bars (book: 14)"),
        "hkcgdt_rsi" => (14., 2., 100., "RSI periods (book: 14)"),
        "hkcgdt_bollinger" => (21., 2., 250., "Average and standard-deviation days (book: 21)"),
        _ => return None,
    };
    Some(vec![ParamDef { key: "n", label, def, min, max, step: 1. }])
}

/// Signal per bar: +1 buy, -1 sell, 0 none.
fn signals(id: &str, x: &Ctx, n: usize) -> Vec<i8> {
    let (h, l, c) = (&x.h, &x.l, &x.c);
    let len = c.len();
    let mut s = vec![0i8; len];
    match id {
        "hkcgdt_ma_close" => {
            let ma: Vec<f64> = (0..len).map(|i| if i + 1 >= n { c[i + 1 - n..=i].iter().sum::<f64>() / n as f64 } else { f64::NAN }).collect();
            for i in n..len {
                if c[i] > ma[i] && c[i - 1] <= ma[i - 1] { s[i] = 1; } else if c[i] < ma[i] && c[i - 1] >= ma[i - 1] { s[i] = -1; }
            }
        }
        "hkcgdt_turtle20" => for i in n..len {
            let hh = h[i - n..i].iter().copied().fold(f64::NEG_INFINITY, f64::max);
            let ll = l[i - n..i].iter().copied().fold(f64::INFINITY, f64::min);
            if c[i] > hh { s[i] = 1; } else if c[i] < ll { s[i] = -1; }
        },
        "hkcgdt_wpr" => for i in n - 1..len {
            let hh = h[i + 1 - n..=i].iter().copied().fold(f64::NEG_INFINITY, f64::max);
            let ll = l[i + 1 - n..=i].iter().copied().fold(f64::INFINITY, f64::min);
            if hh > ll { let r = 100. * (c[i] - ll) / (hh - ll); if r > 80. { s[i] = -1; } else if r < 20. { s[i] = 1; } }
        },
        "hkcgdt_rsi" => if len > n {
            let ch = |i: usize| c[i] - c[i - 1];
            let (mut u, mut d) = ((1..=n).map(|i| ch(i).max(0.)).sum::<f64>() / n as f64, (1..=n).map(|i| (-ch(i)).max(0.)).sum::<f64>() / n as f64);
            let rs = |u: f64, d: f64| if u + d == 0. { 50. } else { 100. * u / (u + d) };
            let mut prev = rs(u, d);
            for i in n + 1..len {
                u = (u * (n as f64 - 1.) + ch(i).max(0.)) / n as f64; d = (d * (n as f64 - 1.) + (-ch(i)).max(0.)) / n as f64;
                let r = rs(u, d);
                if prev < 30. && r >= 30. { s[i] = 1; } else if prev > 70. && r <= 70. { s[i] = -1; }
                prev = r;
            }
        },
        "hkcgdt_bollinger" => for i in n - 1..len {
            let w = &c[i + 1 - n..=i];
            let m = w.iter().sum::<f64>() / n as f64;
            let sd = (w.iter().map(|v| (v - m) * (v - m)).sum::<f64>() / n as f64).sqrt();
            if c[i] < m - 2. * sd { s[i] = 1; } else if c[i] > m + 2. * sd { s[i] = -1; }
        },
        _ => {}
    }
    s
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !LIVE_IDS.contains(&id) || from > to || to >= x.r.len() { return None; }
    for (k, v) in p { let d = params(id)?.into_iter().find(|d| d.key == k)?; if !v.is_finite() || *v < d.min || *v > d.max || v.fract() != 0. { return None; } }
    let n = p.get("n").map_or_else(|| params(id).unwrap()[0].def, |v| *v) as usize;
    let sig = signals(id, x, n);
    let c = &x.c;
    let mut out: Vec<Trade> = vec![];
    let mut cur: Option<Trade> = None;
    for i in from..=to {
        let dir = match sig[i] { 1 => 1., -1 => -1., _ => continue };
        if cur.as_ref().map_or(false, |t| t.dir == dir) { continue; }
        if let Some(mut t) = cur.take() { t.exit_i = i; t.exit_px = c[i]; t.signal_i = i; t.reason = "opposite signal".into(); out.push(t); }
        if i < to { cur = Some(Trade { dir, legs: vec![Leg { i, px: c[i], w: 1. }], exit_i: i, exit_px: c[i], reason: String::new(), entry_kind: "close", signal_i: i, open: false, exits: vec![] }); }
    }
    if let Some(mut t) = cur.take() { t.exit_i = to; t.exit_px = c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true; out.push(t); }
    Some(out)
}

pub const ADMISSION_R1: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == BOOK && LIVE_IDS.contains(&def.id),
    label: "The book shows these entry signals on short intraday charts and names no market; daily bars of indices, ETFs and stocks, US and India, are a declared application. Adapted: 3-4 unstated details per card (parameter or definition, window, stop-and-reverse exit, hold between signals), labelled on each card.",
    basis: "Heitkoetter, The Complete Guide to Day Trading p.135-146; R4 #297/#339/#343; independent check authors/relax-339/R1/w/hk_ref.py",
};

#[cfg(test)]
mod tests {
    #[test]
    fn five_signals_live_rest_held() {
        for d in super::catalogue() {
            assert_eq!(super::LIVE_IDS.contains(&d.id), matches!(d.status, crate::connors::Status::Live), "{}", d.id);
            assert!(super::family_of(d.id).is_some());
        }
        assert!(super::catalogue().iter().filter(|d| matches!(d.status, crate::connors::Status::Live)).all(|d| d.notes.contains("UNVERIFIED")));
    }
}
