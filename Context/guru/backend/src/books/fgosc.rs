//! Optionetics System Center, "Build Winning Systems" course workbooks (Optionetics 2008; George Fontanills & Tom Gentile
//! course, research share 'Trading Courses,Seminars,Videos/George Fontanills & Tom Gentile - Optionetics System Center').
//! Book id `fgosc`. Two complete ProfitSource systems:
//!   `fgosc_1010`        Vol. II, System 5 "The 1010 Trading System" (book pp.32-54, PDF pp.38-60).
//!   `fgosc_four_alarm`  Vol. I, System 3 "The Four Alarm Buy System" (book pp.51-77, PDF pp.58-86).
//! Reading record: authors/batch-i/I5w (opt_1010.txt, opt_fouralarm.txt); lane K21 (titles loop, 2 Nov 2026), lead decisions
//! 149-150. Independent check: authors/titles-1030/K21w/k21_ref.py (written from the workbook text and screenshots).
use crate::connors::{claim, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "fgosc";
pub const REVIEWED: &[&str] = &["fgosc_1010", "fgosc_four_alarm"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE_1010: &str = "[Readings, labelled] 'New Low within the last 10 days' (ProfitSource New Low hi-lite, formula not printed) = today's low is below the low of each of the 9 days before it, i.e. the lowest low of the last 10 days including today: this reading reproduces the entry dates of all 27 trades the workbook lists for SPX (Fig 5.28), while 'below the previous 10 days' finds only 23 of them; that one is the option prev = 10. The exit is tested from the day after the entry; an exit day that is also a signal day closes the old trade and opens a new one at the same close (the list's trades 14 and 15, 4/20/2004 at 1118.15). One position at a time (decision 149): the workbook's RUT test stacks one unit per signal day; that stacking is held. No stop ('Stops Not Set', p.36). The workbook sizes at 20% of the balance with no costs; here one unit, no costs. The author built it for index markets ('this system was designed for the index markets only', p.40): SPX is the book's market; other indices, ETFs and stocks, US and India, are a declared application. An open trade is marked at the window's last close.";

const NOTE_FOUR: &str = "[Readings, labelled] Rule #4 is printed twice as '3-day RSI ... less than or equal to 36' (p.51, p.70), but the scan the workbook builds and tests (Fig 3.30, p.69) is 'RSI of the current day Is Greater Than 36'. The default follows the tested scan (the printed code, as in lead decisions 133/143): on the vault's own US stock files, 22 of the 23 listed 1998 entries we hold (Fig 3.43) are signal days with RSI(3) above 36 and none with RSI(3) at or below 36. The rule text is the option rsi_side = 1. Oscillator(5, 35, Close) = 5-day minus 35-day simple average of the close (ProfitSource oscillator, formula not printed). MACD = EMA(12) - EMA(25) of the close, signal = EMA(9) of MACD, EMAs 2/(n+1) started at the first close of the file. ROC(28) = close / close 28 days ago; its 28-day simple average. RSI(3) = Wilder smoothing, started with the simple average of the first 3 changes. Exit at the close 25 bars after the entry bar (Time Stop, Fig 3.39-3.40); the listed exits are all exactly 25 bars later on our data. A signal on the exit day re-enters at that close. One position per instrument, one unit, no costs. The workbook test is a cash-limited portfolio of the 30 Dow stocks ($100,000), so its trade count is not comparable with one-instrument runs. Built for stocks; indices, ETFs and stocks, US and India, are a declared application. An open trade is marked at the window's last close.";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, rules, notes, claim| StrategyDef { id, name, group, session: 0, dir: "Long", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim };
    vec![
        d("fgosc_1010", "1010 system: buy the close of a new 10-day low, sell the close when the high crosses above the 10-day average", "System 5 · The 1010 Trading System (Vol. II p.32-54)",
          &["E|\"see the initial 10-day low in the SPY followed by a new low in the last 10 days ... Buy the index market at the close of the signal day\" (p.32); scan \"New Low Within the Last 10 Days\", Number of Days Previous 10; \"Enter on Signal Day, Enter at Close of the signal day\" (p.39-42)",
            "X|\"exit the market when the index crosses above the 10-day moving average\" (p.32); exit scan \"High of the current day Is Greater Than MA 1 of the current day\", MA 1 = simple 10-day average of the close; \"Exit at Close of the signal day\" (p.43, 48)",
            "X|\"the only exit on this system occurs when the SPX moves above the 10-day SMA\" (p.46); stops not set"],
          NOTE_1010, claim("65", "70.77%", "$92.77 at 20% of $100,000", "5.0 days winners / 8.7 days losers", "SPX 3/2/2003-12/31/2007, no costs (Fig 5.29, p.46)")),
        d("fgosc_four_alarm", "Four Alarm Buy: MACD, 5/35 oscillator, 28-day ROC and 3-day RSI all true, buy the close, exit 25 bars later", "System 3 · The Four Alarm Buy System (Vol. I p.51-77)",
          &["E|\"Rule #1: MACD (12, 25, 9) line is greater than MACD signal line (i.e., MACD histogram is positive)\" (p.51)",
            "E|\"Rule #2: Oscillator (5, 35, Close) today is greater than the Oscillator 4 days ago\" (p.51)",
            "E|\"Rule #3: The 28-day Rate-of-Change indicator (ROC) is greater than the 28-day moving average of the 28-day rate-of-change\" (p.51)",
            "E|\"Rule #4: The 3-day RSI Index is less than or equal to 36\" (p.51, 70); the tested scan: \"RSI of the current day Is Greater Than 36\" (Fig 3.30, p.69). \"All four rules ... must be met\"; \"Enter on Signal Day, Enter at Close of the signal day\" (p.74)",
            "X|Time Stop: \"Exit at the Close 25 bars after the activation bar\" (Fig 3.39, p.74); \"This system had no exits other than time\" (p.76)"],
          NOTE_FOUR, claim("694", "54.56%", "$89.75", "about 36 calendar days", "30 Dow stocks 3/2/1998-3/22/2008, $100,000 cash-limited portfolio, no costs (Fig 3.42, p.75)")),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    catalogue().into_iter().find(|d| d.id == id).map(|d| (d.id, d.name, ""))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(match id {
        "fgosc_1010" => vec![
            p("prev", "New low: below the lows of this many days before today (9 = lowest low of 10 days, reproduces the book list; 10 = option)", 9., 1., 50., 1.),
            p("ma", "Exit average of the close (book: 10)", 10., 2., 100., 1.)],
        "fgosc_four_alarm" => vec![
            p("rsi_side", "RSI(3) test: 0 = above 36 (the tested scan, Fig 3.30), 1 = at or below 36 (rule text)", 0., 0., 1., 1.),
            p("hold", "Time stop: exit at the close this many bars after entry (book: 25)", 25., 1., 100., 1.)],
        _ => return None,
    })
}

fn sma(v: &[f64], n: usize) -> Vec<f64> {
    (0..v.len()).map(|i| if i + 1 >= n { v[i + 1 - n..=i].iter().sum::<f64>() / n as f64 } else { f64::NAN }).collect()
}

/// EMA 2/(n+1) started at the first finite value.
fn ema(v: &[f64], n: usize) -> Vec<f64> {
    let k = 2. / (n as f64 + 1.);
    let mut out = vec![f64::NAN; v.len()];
    let mut prev = f64::NAN;
    for (i, &x) in v.iter().enumerate() {
        if !x.is_finite() { continue; }
        prev = if prev.is_finite() { prev + k * (x - prev) } else { x };
        out[i] = prev;
    }
    out
}

/// Wilder RSI, seeded with the simple average of the first n changes.
fn rsi(c: &[f64], n: usize) -> Vec<f64> {
    let mut out = vec![f64::NAN; c.len()];
    if c.len() <= n { return out; }
    let ch = |i: usize| c[i] - c[i - 1];
    let (mut au, mut ad) = ((1..=n).map(|i| ch(i).max(0.)).sum::<f64>() / n as f64, (1..=n).map(|i| (-ch(i)).max(0.)).sum::<f64>() / n as f64);
    for i in n..c.len() {
        if i > n { au = (au * (n - 1) as f64 + ch(i).max(0.)) / n as f64; ad = (ad * (n - 1) as f64 + (-ch(i)).max(0.)) / n as f64; }
        out[i] = if ad == 0. { if au == 0. { 50. } else { 100. } } else { 100. - 100. / (1. + au / ad) };
    }
    out
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if !reviewed(id) || from > to || to >= x.c.len() { return None; }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let ni = |k: &str| g(k).round().max(1.) as usize;
    let (h, l, c) = (&x.h, &x.l, &x.c);
    // entry signal per bar, and the exit test for a position entered at bar e
    let sig: Vec<bool>;
    let exit: Box<dyn Fn(usize, usize) -> bool + '_>;
    let why: &str;
    match id {
        "fgosc_1010" => {
            let n = ni("prev");
            sig = (0..c.len()).map(|i| i >= n && (i - n..i).all(|j| l[i] < l[j])).collect();
            let ma = sma(c, ni("ma"));
            exit = Box::new(move |_, i| ma[i].is_finite() && h[i] > ma[i]);
            why = "high above the 10-day average";
        }
        "fgosc_four_alarm" => {
            let (e12, e25) = (ema(c, 12), ema(c, 25));
            let macd: Vec<f64> = e12.iter().zip(&e25).map(|(a, b)| a - b).collect();
            let sg = ema(&macd, 9);
            let (s5, s35) = (sma(c, 5), sma(c, 35));
            let osc: Vec<f64> = s5.iter().zip(&s35).map(|(a, b)| a - b).collect();
            let roc: Vec<f64> = (0..c.len()).map(|i| if i >= 28 { c[i] / c[i - 28] } else { f64::NAN }).collect();
            let mroc = sma(&roc, 28);
            let r3 = rsi(c, 3);
            let below = g("rsi_side") >= 0.5;
            sig = (0..c.len()).map(|i| i >= 4 && osc[i - 4].is_finite() && mroc[i].is_finite() && r3[i].is_finite()
                && macd[i] > sg[i] && osc[i] > osc[i - 4] && roc[i] > mroc[i] && if below { r3[i] <= 36. } else { r3[i] > 36. }).collect();
            let hold = ni("hold");
            exit = Box::new(move |e, i| i == e + hold);
            why = "time stop";
        }
        _ => return None,
    }
    let mk = |e: usize, xi: usize, why: &str, open: bool| Trade { dir: 1., legs: vec![Leg { i: e, px: c[e], w: 1. }], exit_i: xi, exit_px: c[xi],
        reason: why.into(), entry_kind: "close", signal_i: xi, open, exits: vec![] };
    let (mut out, mut cur) = (vec![], None::<usize>);
    for i in from..=to {
        if let Some(e) = cur { if i > e && exit(e, i) { out.push(mk(e, i, why, false)); cur = None; } }
        if cur.is_none() && sig[i] { cur = Some(i); }
    }
    if let Some(e) = cur { out.push(mk(e, to, "open", true)); }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn ctx(b: &[(f64, f64, f64)]) -> Vec<Row> {
        b.iter().enumerate().map(|(i, &(h, l, c))| Row { ticker: "T".into(), date: format!("2003-{:02}-{:02}", 1 + i / 28, 1 + i % 28), open: c, high: h, low: l, close: c, volume: None }).collect()
    }
    #[test]
    fn ten_ten_new_low_in_high_above_average_out() {
        // 10 flat bars (low 99), bar 10 makes a new low (98.5) -> buy its close 99; bar 11 high 101 > 10-day average -> exit at 100.5
        let mut b = vec![(101., 99., 100.); 10];
        b.push((100., 98.5, 99.));
        b.push((101., 99.5, 100.5));
        let r = ctx(&b);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("fgosc_1010", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert_eq!(t.iter().map(|z| (z.legs[0].i, z.legs[0].px, z.exit_i, z.exit_px, z.open)).collect::<Vec<_>>(), vec![(10, 99., 11, 100.5, false)]);
        assert!(run("fgosc_x", &x, 0, r.len() - 1, &Params::new()).is_none());
    }
    #[test]
    fn four_alarm_exits_25_bars_later_and_rsi_side_flips() {
        // two sine waves on a slow rise: both RSI sides fire; every closed trade lasts exactly `hold` bars
        let b: Vec<(f64, f64, f64)> = (0..300).map(|i| { let t = i as f64; let c = 100. + 10. * (t / 5.).sin() + 5. * (t / 2.3).sin() + 0.1 * t; (c + 1., c - 1., c) }).collect();
        let r = ctx(&b);
        let x = Ctx::new(&r, &r, &[]);
        let up = run("fgosc_four_alarm", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(!up.is_empty());
        assert!(up.iter().filter(|t| !t.open).all(|t| t.exit_i == t.legs[0].i + 25));
        let mut p = Params::new();
        p.insert("rsi_side".into(), 1.);
        let dn = run("fgosc_four_alarm", &x, 0, r.len() - 1, &p).unwrap();
        assert!(!dn.is_empty() && up.iter().all(|t| !dn.iter().any(|u| u.legs[0].i == t.legs[0].i)));
    }
}

// Instrument admission: Optionetics System Center (lane K21, 2 Nov 2026). Runs on actual indices, ETFs and stocks, US and
// India (declared application); authors/titles-1030/K21w/k21_ref.py is the independent check.
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "fgosc" && crate::books::fgosc::reviewed(def.id),
    label: "The workbooks test the 1010 system on the SPX and RUT cash indices and the Four Alarm system on the 30 Dow stocks; other index, ETF and stock runs, US and India, are a declared application, not the authors' test.",
    basis: "Optionetics System Center, Build Winning Systems workbooks (2008): Vol. II System 5 'The 1010 Trading System' (p.32-54), Vol. I System 3 'The Four Alarm Buy System' (p.51-77); readings in authors/batch-i/I5w; independent reference authors/titles-1030/K21w/k21_ref.py",
};
