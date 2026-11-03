//! Optionetics System Center, "Build Winning Systems" course workbooks (Optionetics 2008; George Fontanills & Tom Gentile
//! course, research share 'Trading Courses,Seminars,Videos/George Fontanills & Tom Gentile - Optionetics System Center').
//! Book id `fgosc`. Five complete ProfitSource systems:
//!   `fgosc_1010`        Vol. II, System 5 "The 1010 Trading System" (book pp.32-54, PDF pp.38-60).
//!   `fgosc_four_alarm`  Vol. I, System 3 "The Four Alarm Buy System" (book pp.51-77, PDF pp.58-86).
//!   `fgosc_crossover`   Vol. I, System 1 "George Fontanills' Crossover Buy/Sell System" (book pp.1-30, PDF pp.8-37).
//!   `fgosc_adx_dmi`     Vol. II, System 4 "ADX/DMI Trading Systems" (book pp.1-30, PDF pp.6-36).
//!   `fgosc_volume_spike` Vol. III, System 7 "Volume Spike Trading System" (book pp.3-32, PDF pp.8-37).
//! Reading record: authors/batch-i/I5w (opt_1010.txt, opt_fouralarm.txt) and I6w (opt_s1_crossover.txt, opt_s4_adxdmi.txt,
//! opt_s7_volspike.txt); lanes K21 and K22 (titles loop, 2-3 Nov 2026), lead decisions 149-150, 154. Independent checks:
//! authors/titles-1030/K21w/k21_ref.py and K22w/k22_ref.py (written from the workbook text and screenshots).
use crate::connors::{claim, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "fgosc";
pub const REVIEWED: &[&str] = &["fgosc_1010", "fgosc_four_alarm", "fgosc_crossover", "fgosc_adx_dmi", "fgosc_volume_spike"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE_1010: &str = "[Readings, labelled] 'New Low within the last 10 days' (ProfitSource New Low hi-lite, formula not printed) = today's low is below the low of each of the 9 days before it, i.e. the lowest low of the last 10 days including today: this reading reproduces the entry dates of all 27 trades the workbook lists for SPX (Fig 5.28), while 'below the previous 10 days' finds only 23 of them; that one is the option prev = 10. The exit is tested from the day after the entry; an exit day that is also a signal day closes the old trade and opens a new one at the same close (the list's trades 14 and 15, 4/20/2004 at 1118.15). One position at a time (decision 149): the workbook's RUT test stacks one unit per signal day; that stacking is held. No stop ('Stops Not Set', p.36). The workbook sizes at 20% of the balance with no costs; here one unit, no costs. The author built it for index markets ('this system was designed for the index markets only', p.40): SPX is the book's market; other indices, ETFs and stocks, US and India, are a declared application. An open trade is marked at the window's last close.";

const NOTE_FOUR: &str = "[Readings, labelled] Rule #4 is printed twice as '3-day RSI ... less than or equal to 36' (p.51, p.70), but the scan the workbook builds and tests (Fig 3.30, p.69) is 'RSI of the current day Is Greater Than 36'. The default follows the tested scan (the printed code, as in lead decisions 133/143): on the vault's own US stock files, 22 of the 23 listed 1998 entries we hold (Fig 3.43) are signal days with RSI(3) above 36 and none with RSI(3) at or below 36. The rule text is the option rsi_side = 1. Oscillator(5, 35, Close) = 5-day minus 35-day simple average of the close (ProfitSource oscillator, formula not printed). MACD = EMA(12) - EMA(25) of the close, signal = EMA(9) of MACD, EMAs 2/(n+1) started at the first close of the file. ROC(28) = close / close 28 days ago; its 28-day simple average. RSI(3) = Wilder smoothing, started with the simple average of the first 3 changes. Exit at the close 25 bars after the entry bar (Time Stop, Fig 3.39-3.40); the listed exits are all exactly 25 bars later on our data. A signal on the exit day re-enters at that close. One position per instrument, one unit, no costs. The workbook test is a cash-limited portfolio of the 30 Dow stocks ($100,000), so its trade count is not comparable with one-instrument runs. Built for stocks; indices, ETFs and stocks, US and India, are a declared application. An open trade is marked at the window's last close.";

const NOTE_CROSS: &str = "[Readings, labelled] A cross = the 10-day simple average of the close is above the 30-day average today and was not above it yesterday (equal is not above); a cross down is the mirror. The first trade is the first cross after both averages exist inside the window; from then on the system is always long or short ('in the market 100% of the time', p.19), reversing at the close of each cross day. No stop ('Stops Not Set', p.19). One unit, no costs; the workbook sizes at 50% of a $100,000 account. On the vault's GOOGL file (GOOG class A) the reading reproduces the workbook's GOOG trades 2 to 26 exactly, dates and prices (Fig 1.32); the workbook's trade 1 is entered at the IPO-day close, before 30 days of prices exist, which no 10/30 reading can reproduce. The author trades options on the signals (p.29); that is not coded. Built for single stocks ('works best on volatile stocks'); indices, ETFs and other stocks, US and India, are a declared application. Cash-index shorts are hypothetical. An open trade is marked at the window's last close.";

const NOTE_ADX: &str = "[Readings, labelled] ADX and the DIs are Wilder's (directional movement and true range smoothed with Wilder's running average), over 13 days: the chapter says 'you will be using 13 days' for DMI and ADX (p.2-3); the test pages do not print the period, and 14 is an option. 'ADX crosses above 20 within the last 1 days' = ADX above 20 today and at or below 20 yesterday. The scan's close > 10 and volume-average > 300,000 filters are applied on US instruments only (a dollar price and a US share count; India runs without them, as decision 58); on the vault's split-adjusted US stock files a stock can sit under $10 in the past although it traded higher then. The volume-average period is not printed: 65 days is the only period under 130 at which every printed Fig 4.10 volume average is a whole number of shares divided by the period; the option vma changes it. Entry: a stop order 1 tick above the set-up day's high (short: 1 tick below its low) on the next 3 days (Extreme Point Rule), filled at the level or at a gapping open; a newer set-up while flat replaces the order; set-ups while in a trade are ignored. Exit at the close of a day on which ADX has fallen on each of the last 3 days (tested from the entry day's close; option exit_day). Volatility trailing stop: the highest high (short: lowest low) since the entry minus (plus) 2.5 x the 9-day average true range, recomputed every day and not ratcheted (this reproduces the listed QCOM exit of 3/26/2008), live from the day after the entry. Profit/Loss stop 'Entry Price - 5%', live from the fill: the default follows the printed trade list (decision 154): the level entry x 0.95 is a stop loss on a long and a profit target on a short (the list's shorts exit at +5.0%, some on the entry day); the text's reading, a 5% stop loss on both sides, is the option pl = 1. Levels hit on the same day: a gap fills at the open, otherwise the level nearest the open fills first. One position, one unit, no costs. Of the 19 listed 2008 trades (Fig 4.26) in stocks the vault holds, 6 entries are found on the vault data (ProfitSource's own ADX arithmetic is not printed; no standard smoothing found more than 6). Built for stocks; indices, ETFs and stocks, US and India, are a declared application; cash-index shorts are hypothetical. An open trade is marked at the window's last close.";

const NOTE_SPIKE: &str = "[Readings, labelled] Spike day: volume above 3 x its 30-day simple average, the average including that day (scan 'Volume of the current day Is Greater Than Moving Average of the current day * 3'). Entry: a buy stop 1 tick above the spike day's high on the next 10 days (VOLSYS1: 'Enter on Signal Day + 1 < 10'), filled at the level or at a gapping open; a newer spike while flat replaces the order; spikes while in a trade are ignored (the workbook: trades skipped while 'open when another volume spike occurred', p.31). Exit: the 1-1 Profit/Loss stops, +10% target and -10% stop from the entry price, live from the fill (on the entry day they fill at the level; later a gap fills at the open; when both are reached on one bar the one nearer the bar's starting price fills first). No other exit. Default = long only with 1-1 stops, the author's choice ('we prefer the 75% choice', p.31); both sides (VOLSYS: also a sell stop 1 tick below the spike day's low) and the 2-1 version (target 20) are options. One position, one unit, no costs; the workbook sizes at 20% of a $100,000 account across the 30 Dow stocks. Needs actual volume. Built for stocks (the Dow 30); indices, ETFs and stocks, US and India, are a declared application. Not the same rule as fgitsm_volspike (that card enters on a close beyond the spike range and exits on a break of the spike low). The workbook's options rules (p.32) are not coded. An open trade is marked at the window's last close.";

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
        StrategyDef { dir: "Long/Short", ..d("fgosc_crossover", "Crossover Buy/Sell: 10-day average crosses the 30-day, long or short at the close, always in the market", "System 1 · George Fontanills' Crossover Buy/Sell System (Vol. I p.1-30)",
          &["E|\"1030 Buy: Buy whenever a 10-day moving average crosses above a 30-day moving average at the close of the day the signal occurred\" (p.1); scan Mov Avg - Simple MA 1 (10, Close) Crosses Above MA 2 (30, Close) (Fig 1.16, p.12); \"Enter on Signal Day, Enter at Close of the signal day\" (Fig 1.28, p.19)",
            "X|\"1030 Sell: Sell whenever a 10-day moving average crosses below a 30-day moving average at the close of the day the signal occurred\" (p.1); long exit = 1030 Sell scan, at the close (Fig 1.28)",
            "E|\"This system is in the market 100% of the time - either on the long side or the short side ... For the Short Entry Setup, select 1030 Sell; For a Short Exit, select 1030 Buy\" (p.19); stops not set"],
          NOTE_CROSS, claim("26", "66.67%", "$3,956.73 at 50% of $100,000", "57.6 days winners / 21.0 days losers", "GOOG 1/2/2004-12/31/2007, no costs (Fig 1.33, p.22)")) },
        StrategyDef { dir: "Long/Short", ..d("fgosc_adx_dmi", "ADX/DMI breakout: ADX under 20 for 15 days crosses 20, stop entry beyond the set-up day, exit when ADX falls 3 days", "System 4 · ADX/DMI Trading Systems (Vol. II p.1-30)",
          &["F|\"ADX of 5 days ago Is Less Than 20; ADX of 10 days ago Is Less Than 20; ADX of 15 days ago Is Less Than 20; ADX of the current day Crosses Above 20 Within The Last 1 Days; +DI of the current day Is Greater Than -DI of the current day; Close of the current day Is Greater Than 10; Volume Moving Average of the current day Is Greater Than 300000\" (Fig 4.22, p.25); bearish entry scan the mirror (-DI above +DI, p.24); DMI and ADX \"you will be using 13 days\" (p.2-3)",
            "E|\"Honor the 'Extreme Point Rule' and require the stock to trade above (or below) the high (or low) of the signal day, and allow up to 3 days\" (p.23); \"Enter on Signal Day + 1 < 3, Enter at 1 Cent(s) above the High/Low of the signal day\" (Fig 4.21, p.24)",
            "X|exit scan: ADX \"decreasing for the past 3 days\" (Fig 4.23, p.25-26); \"Exit on Signal Day, Exit at Close of the signal day\" (Fig 4.21)",
            "X|stops: \"Profit/Loss Stop Immediate Entry Price - 5%\"; \"Trailing Stop - trails by 'x' ATR multiples ... 2.5 ATR Multiple(s) using 9 period ATR, measured from the High/Low\", active immediately (Fig 4.24, p.26)",
            "F|improved version: \"+DI of the current day is Increasing for the past 3 days\" (bearish: -DI) (p.29, Fig 4.28)"],
          NOTE_ADX, claim("323", "44.75%", "$5.03 at 5% of $100,000", "11.9 days winners / 9.1 days losers", "ProfitSource 'US Indexes' quote list of large Nasdaq stocks, 1/1/2006-3/31/2008, no costs (Fig 4.25, p.27)")) },
        StrategyDef { needs_volume: true, ..d("fgosc_volume_spike", "Volume Spike: volume over 3x its 30-day average, buy stop above the spike day's high for 10 days, +10% target / -10% stop", "System 7 · Volume Spike Trading System (Vol. III p.3-32)",
          &["F|scan \"Volume of the current day Is Greater Than Moving Average of the current day\", multiply by 3, MA Periods 30 (Volume [30], Fig 7.10, p.7; scan p.10-12)",
            "E|\"Enter on Signal Day + 1 < 10, Enter at 1 Cent(s) above the High/Low of the signal day\" (Fig 7.46, p.28); \"buy the market if the high of the signal day is taken out\" (p.20)",
            "X|\"Profit/Loss Stop Immediate Entry Price + 10%\" and \"Entry Price - 10%\": \"a 10% profit and 10% stop, or a 1-1 reward-to-risk ratio\" (p.23, Fig 7.37); the 2-1 version: + 20% / - 10% (Fig 7.49, p.30)",
            "E|long only (VOLSYS1): \"clear out all of the short trade setups\"; \"we prefer the 75% choice\" (p.28, 31); both sides (VOLSYS): sell stop \"1 Cents Below the High/Low of the Signal Day\" (Fig 7.40, p.24)"],
          NOTE_SPIKE, claim("29", "75.86%", "$1,052.63 at 20% of $100,000", "113.2 days winners / 76.9 days losers", "30 Dow stocks 1/3/2005-12/31/2007, long only, no costs (Fig 7.47, p.29)")) },
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
        "fgosc_crossover" => vec![
            p("s", "Short simple average of the close (book: 10)", 10., 2., 100., 1.),
            p("l", "Long simple average of the close (book: 30)", 30., 3., 300., 1.)],
        "fgosc_adx_dmi" => vec![
            p("n", "DMI / ADX days (chapter: 13; 14 = the common setting)", 13., 5., 30., 1.),
            p("improved", "Improved version: the leading DI also rose on each of the last 3 days (Fig 4.28): 0 = off (first version), 1 = on", 0., 0., 1., 1.),
            p("pl", "5% Profit/Loss level: 0 = entry price - 5% on both sides (stop on a long, target on a short, as the printed trade list), 1 = a 5% stop loss on both sides (rule text)", 0., 0., 1., 1.),
            p("filters", "Scan filters close > 10 and volume average > 300,000 (US only): 1 = on (book), 0 = off", 1., 0., 1., 1.),
            p("vma", "Volume average days for the 300,000 filter (not printed; 65 fits the printed scan figures)", 65., 5., 200., 1.),
            p("exit_day", "ADX exit tested: 0 = from the entry day's close, 1 = from the day after the entry", 0., 0., 1., 1.),
            p("tick", "Entry offset beyond the set-up day's high / low in ticks (book: 1 cent = 1 tick; 0 = none)", 1., 0., 10., 1.)],
        "fgosc_volume_spike" => vec![
            p("mult", "Spike = volume above this many times its average (book: 3)", 3., 1.5, 10., 0.5),
            p("n", "Volume average days, including the spike day (book: 30)", 30., 5., 200., 1.),
            p("target", "Profit target % above the entry (book: 10; VOLSYS2: 20)", 10., 1., 100., 1.),
            p("stop", "Stop loss % below the entry (book: 10)", 10., 1., 50., 1.),
            p("short", "Sides: 0 = long only (the author's choice, VOLSYS1), 1 = long and short (VOLSYS)", 0., 0., 1., 1.),
            p("tick", "Entry offset beyond the spike day's high / low in ticks (book: 1 cent = 1 tick; 0 = none)", 1., 0., 10., 1.)],
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

fn tr1(d: f64, e: usize, epx: f64, kind: &'static str, xi: usize, xpx: f64, why: &str, open: bool) -> Trade {
    Trade { dir: d, legs: vec![Leg { i: e, px: epx, w: 1. }], exit_i: xi, exit_px: xpx, reason: why.into(), entry_kind: kind, signal_i: xi, open, exits: vec![] }
}

/// System 1: 10/30 simple-average crossover, stop and reverse at the close of the cross day.
fn crossover(x: &Ctx, from: usize, to: usize, s: usize, l: usize) -> Vec<Trade> {
    let c = &x.c;
    let (f, sl) = (sma(c, s), sma(c, l));
    let (mut out, mut pos) = (vec![], None::<(f64, usize)>);
    for i in from.max(1)..=to {
        if ![f[i], sl[i], f[i - 1], sl[i - 1]].iter().all(|v| v.is_finite()) { continue; }
        let d = if f[i] > sl[i] && f[i - 1] <= sl[i - 1] { 1. } else if f[i] < sl[i] && f[i - 1] >= sl[i - 1] { -1. } else { continue };
        if pos.is_some_and(|(pd, _)| pd == d) { continue; }
        if let Some((pd, e)) = pos { out.push(tr1(pd, e, c[e], "close", i, c[i], "the averages crossed the other way (reverse)", false)); }
        pos = Some((d, i));
    }
    if let Some((pd, e)) = pos { out.push(tr1(pd, e, c[e], "close", to, c[to], "open", true)); }
    out
}

/// Wilder's directional system: (+DI, -DI, ADX), all smoothed over n with Wilder's running average.
fn dmi3(x: &Ctx, n: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let k = x.c.len();
    let (mut pdm, mut mdm) = (vec![f64::NAN; k], vec![f64::NAN; k]);
    for i in 1..k {
        let (up, dn) = (x.h[i] - x.h[i - 1], x.l[i - 1] - x.l[i]);
        pdm[i] = if up > dn && up > 0. { up } else { 0. };
        mdm[i] = if dn > up && dn > 0. { dn } else { 0. };
    }
    let mut tr = crate::connors::true_range(x.r);
    if k > 0 { tr[0] = f64::NAN; }
    let w = crate::connors::wilder;
    let (sp, sm, st) = (w(&pdm, n), w(&mdm, n), w(&tr, n));
    let (mut pdi, mut mdi, mut dx) = (vec![f64::NAN; k], vec![f64::NAN; k], vec![f64::NAN; k]);
    for i in 0..k {
        if st[i] > 0. {
            pdi[i] = 100. * sp[i] / st[i];
            mdi[i] = 100. * sm[i] / st[i];
            dx[i] = if pdi[i] + mdi[i] > 0. { 100. * (pdi[i] - mdi[i]).abs() / (pdi[i] + mdi[i]) } else { 0. };
        }
    }
    (pdi, mdi, w(&dx, n))
}

/// v rose (up) or fell on each of the last 3 days.
fn run3(v: &[f64], i: usize, up: bool) -> bool {
    i >= 3 && (0..3).all(|j| v[i - j].is_finite() && v[i - j - 1].is_finite() && if up { v[i - j] > v[i - j - 1] } else { v[i - j] < v[i - j - 1] })
}

/// Of the price levels reached on one bar (level, fill, why), the one nearest the bar's starting price fills first
/// (the first listed wins a tie).
fn first_hit<'a>(start: f64, hits: &[(f64, f64, &'a str)]) -> (f64, &'a str) {
    let mut best = hits[0];
    for &h in &hits[1..] { if (h.0 - start).abs() < (best.0 - start).abs() { best = h; } }
    (best.1, best.2)
}

fn volume(x: &Ctx) -> Vec<f64> { x.r.iter().map(|r| r.volume.filter(|v| v.is_finite() && *v > 0.).unwrap_or(f64::NAN)).collect() }

/// System 4: ADX/DMI breakout with the Extreme Point entry, ADX-falling exit, ATR trailing stop and the 5% level.
fn adx_dmi(x: &Ctx, from: usize, to: usize, g: &dyn Fn(&str) -> f64) -> Vec<Trade> {
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let ni = |k: &str| g(k).round().max(1.) as usize;
    let (pdi, mdi, adx) = dmi3(x, ni("n"));
    let atr = sma(&crate::connors::true_range(x.r), 9);
    let vma = sma(&volume(x), ni("vma"));
    let us_filter = g("filters") >= 0.5 && !crate::books::tselhs::india(x);
    let (improved, loss_mode, same_day) = (g("improved") >= 0.5, g("pl") >= 0.5, g("exit_day") < 0.5);
    let tk = g("tick") * x.market_tick();
    let setup = |i: usize| -> f64 {
        if i < 15 || ![5, 10, 15].iter().all(|&j| adx[i - j] < 20.) || !(adx[i] > 20. && adx[i - 1] <= 20.) { return 0.; }
        if us_filter && !(c[i] > 10. && vma[i] > 300_000.) { return 0.; }
        if pdi[i] > mdi[i] && (!improved || run3(&pdi, i, true)) { 1. } else if mdi[i] > pdi[i] && (!improved || run3(&mdi, i, true)) { -1. } else { 0. }
    };
    // pending order (dir, level, signal bar); position (dir, entry bar, entry price, extreme, trailing level)
    let (mut out, mut pend, mut pos) = (vec![], None::<(f64, f64, usize)>, None::<(f64, usize, f64, f64, f64)>);
    for i in from..=to {
        if let Some((d, lv, sg)) = pend.filter(|_| pos.is_none()) {
            if i > sg + 3 { pend = None; }
            else if (d > 0. && h[i] >= lv) || (d < 0. && l[i] <= lv) {
                let px = if d > 0. { o[i].max(lv) } else { o[i].min(lv) };
                pend = None;
                let (pl, ls) = (px * 0.95, px * 1.05);
                let why = if d > 0. { "5% stop loss" } else if loss_mode { "5% stop loss" } else { "5% Profit/Loss level (target on a short)" };
                // the Profit/Loss level is live from the fill: on the entry day it fills at the level itself
                let hit = if d > 0. { (l[i] <= pl).then_some(pl) } else if loss_mode { (h[i] >= ls).then_some(ls) } else { (l[i] <= pl).then_some(pl) };
                if let Some(xp) = hit { out.push(tr1(d, i, px, "stop", i, xp, why, false)); }
                else if same_day && run3(&adx, i, false) { out.push(tr1(d, i, px, "stop", i, c[i], "ADX fell 3 days", false)); }
                else {
                    let ext = if d > 0. { h[i] } else { l[i] };
                    let trail = if atr[i].is_finite() { ext - d * 2.5 * atr[i] } else { f64::NAN };
                    pos = Some((d, i, px, ext, trail));
                }
                if pos.is_none() { let s = setup(i); if s != 0. { pend = Some((s, if s > 0. { h[i] + tk } else { l[i] - tk }, i)); } }
                continue;
            }
        }
        if let Some((d, e, px, ext, trail)) = pos {
            let mut hits: Vec<(f64, f64, &str)> = vec![];
            let lv = if d > 0. || !loss_mode { px * 0.95 } else { px * 1.05 };
            let lv_why = if d < 0. && !loss_mode { "5% Profit/Loss level (target on a short)" } else { "5% stop loss" };
            if lv < px { if l[i] <= lv { hits.push((lv, o[i].min(lv), lv_why)); } } else if h[i] >= lv { hits.push((lv, o[i].max(lv), lv_why)); }
            if trail.is_finite() {
                if d > 0. && l[i] <= trail { hits.push((trail, o[i].min(trail), "2.5 x ATR(9) trailing stop")); }
                if d < 0. && h[i] >= trail { hits.push((trail, o[i].max(trail), "2.5 x ATR(9) trailing stop")); }
            }
            if !hits.is_empty() {
                let (xp, why) = first_hit(o[i], &hits);
                out.push(tr1(d, e, px, "stop", i, xp, why, false));
                pos = None;
            } else if run3(&adx, i, false) {
                out.push(tr1(d, e, px, "stop", i, c[i], "ADX fell 3 days", false));
                pos = None;
            } else {
                let ext = if d > 0. { ext.max(h[i]) } else { ext.min(l[i]) };
                let nt = if atr[i].is_finite() { ext - d * 2.5 * atr[i] } else { f64::NAN };
                // not ratcheted: the extreme since the entry -/+ 2.5 x today's ATR(9) (reproduces the listed QCOM exit)
                let trail = if nt.is_finite() { nt } else { trail };
                pos = Some((d, e, px, ext, trail));
            }
        }
        if pos.is_none() { let s = setup(i); if s != 0. { pend = Some((s, if s > 0. { h[i] + tk } else { l[i] - tk }, i)); } }
    }
    if let Some((d, e, px, _, _)) = pos { out.push(tr1(d, e, px, "stop", to, c[to], "open", true)); }
    out
}

/// System 7: volume spike, buy stop above the spike day's high for 10 days, +10% target / -10% stop.
fn volume_spike(x: &Ctx, from: usize, to: usize, g: &dyn Fn(&str) -> f64) -> Vec<Trade> {
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let v = volume(x);
    let vma = sma(&v, g("n").round().max(1.) as usize);
    let (mult, tg, sl, both) = (g("mult"), g("target") / 100., g("stop") / 100., g("short") >= 0.5);
    let tk = g("tick") * x.market_tick();
    let (mut out, mut pend, mut pos) = (vec![], None::<(f64, f64, usize)>, None::<(f64, usize, f64)>);
    for i in from..=to {
        if pos.is_none() {
            if let Some((up, dn, sg)) = pend {
                if i > sg + 10 { pend = None; }
                else {
                    let (hu, hd) = (h[i] >= up, dn.is_finite() && l[i] <= dn);
                    let d = if hu && hd { if (up - o[i]).abs() <= (o[i] - dn).abs() { 1. } else { -1. } } else if hu { 1. } else if hd { -1. } else { 0. };
                    if d != 0. { pos = Some((d, i, if d > 0. { o[i].max(up) } else { o[i].min(dn) })); pend = None; }
                }
            }
        }
        if let Some((d, e, px)) = pos {
            let (tgt, stp) = (px * (1. + d * tg), px * (1. - d * sl));
            let first = i == e;
            let start = if first { px } else { o[i] };
            let mut hits: Vec<(f64, f64, &str)> = vec![];
            if (d > 0. && h[i] >= tgt) || (d < 0. && l[i] <= tgt) { hits.push((tgt, if first { tgt } else if d > 0. { o[i].max(tgt) } else { o[i].min(tgt) }, "profit target")); }
            if (d > 0. && l[i] <= stp) || (d < 0. && h[i] >= stp) { hits.push((stp, if first { stp } else if d > 0. { o[i].min(stp) } else { o[i].max(stp) }, "stop loss")); }
            if !hits.is_empty() {
                let (xp, why) = first_hit(start, &hits);
                out.push(tr1(d, e, px, "stop", i, xp, why, false));
                pos = None;
            }
        }
        if pos.is_none() && vma[i].is_finite() && v[i].is_finite() && v[i] > mult * vma[i] {
            pend = Some((h[i] + tk, if both { l[i] - tk } else { f64::NAN }, i));
        }
    }
    if let Some((d, e, px)) = pos { out.push(tr1(d, e, px, "stop", to, c[to], "open", true)); }
    out
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if !reviewed(id) || from > to || to >= x.c.len() { return None; }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let ni = |k: &str| g(k).round().max(1.) as usize;
    match id {
        "fgosc_crossover" => return Some(crossover(x, from, to, ni("s"), ni("l"))),
        "fgosc_adx_dmi" => return Some(adx_dmi(x, from, to, &g)),
        "fgosc_volume_spike" => return Some(volume_spike(x, from, to, &g)),
        _ => {}
    }
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
    fn rows(b: &[(f64, f64, f64, f64, f64)]) -> Vec<Row> {
        b.iter().enumerate().map(|(i, &(o, h, l, c, v))| Row { ticker: "T".into(), date: format!("{}-{:02}-{:02}", 2000 + i / 336, 1 + i / 28 % 12, 1 + i % 28), open: o, high: h, low: l, close: c, volume: Some(v) }).collect()
    }
    #[test]
    fn crossover_reverses_at_each_cross_close() {
        // down 40 bars, up 40, down 40: long and short alternate, each exit is the next entry at the same close
        let c: Vec<f64> = (0..120).map(|i| 100. + if i < 40 { -(i as f64) } else if i < 80 { i as f64 - 80. } else { 80. - i as f64 } * 0.5 + if i < 40 { 0. } else { 0. }).collect();
        let b: Vec<_> = c.iter().map(|&c| (c, c + 1., c - 1., c, 1e6)).collect();
        let r = rows(&b);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("fgosc_crossover", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() >= 2);
        assert_eq!(t[0].dir, 1.);
        for w in t.windows(2) { assert!(w[0].exit_i == w[1].legs[0].i && w[0].exit_px == w[1].legs[0].px && w[0].dir == -w[1].dir); }
        assert!(t.last().unwrap().open);
    }
    #[test]
    fn volume_spike_buys_above_the_spike_high_and_takes_ten_percent() {
        // 40 quiet bars, a 5x-volume spike (high 101), then a rise of 1 a day: buy stop 101.01, target 111.111
        let mut b: Vec<_> = (0..40).map(|_| (100., 100.5, 99.5, 100., 1e6)).collect();
        b.push((100., 101., 99.5, 100.5, 5e6));
        for k in 1..15 { let c = 100.5 + k as f64; b.push((c - 0.5, c + 0.5, c - 1., c, 1e6)); }
        let r = rows(&b);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("fgosc_volume_spike", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert_eq!(t.len(), 1);
        assert_eq!((t[0].legs[0].i, t[0].dir), (41, 1.));
        assert!((t[0].legs[0].px - 101.01).abs() < 1e-9 && (t[0].exit_px - 101.01 * 1.1).abs() < 1e-9 && t[0].reason == "profit target");
        assert!(r[t[0].exit_i].high >= 101.01 * 1.1 && r[t[0].exit_i - 1].high < 101.01 * 1.1);
    }
    #[test]
    fn adx_breakout_enters_beyond_the_set_up_day_and_obeys_its_exits() {
        // a long quiet zig-zag (ADX low), then a steady climb: ADX crosses 20 with +DI on top -> buy stop above that day's high
        let mut b = vec![];
        for i in 0..80 { let c = 50. + if i % 2 == 0 { 0.4 } else { -0.4 }; b.push((c, c + 0.5, c - 0.5, c, 1e6)); }
        for k in 1..40 { let c = 50. + 0.8 * k as f64; b.push((c - 0.3, c + 0.5, c - 0.5, c, 1e6)); }
        let r = rows(&b);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("fgosc_adx_dmi", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(!t.is_empty() && t[0].dir == 1.);
        let (pdi, mdi, adx) = dmi3(&x, 13);
        let e = t[0].legs[0].i;
        let s = (e.saturating_sub(3)..e).rev().find(|&s| adx[s] > 20. && adx[s - 1] <= 20. && pdi[s] > mdi[s]).unwrap();
        assert!((t[0].legs[0].px - (r[e].open.max(r[s].high + 0.01))).abs() < 1e-9);
        assert!(run3(&[1., 2., 3., 4.], 3, true) && !run3(&[1., 2., 2., 4.], 3, true) && run3(&[4., 3., 2., 1.], 3, false));
        assert_eq!(first_hit(10., &[(9., 9., "a"), (10.5, 10.5, "b")]), (10.5, "b"));
    }
}

// Instrument admission: Optionetics System Center (lanes K21-K22, 2-3 Nov 2026). Runs on actual indices, ETFs and stocks, US and
// India (declared application); authors/titles-1030/K21w/k21_ref.py and K22w/k22_ref.py are the independent checks.
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "fgosc" && crate::books::fgosc::reviewed(def.id),
    label: "The workbooks test the 1010 system on the SPX and RUT cash indices, the Four Alarm and Volume Spike systems on the 30 Dow stocks, the Crossover system on GOOG, CME and MSFT and the ADX/DMI system on a list of large Nasdaq stocks; other index, ETF and stock runs, US and India, are a declared application, not the authors' test.",
    basis: "Optionetics System Center, Build Winning Systems workbooks (2008): Vol. I System 1 'Crossover Buy/Sell' (p.1-30) and System 3 'The Four Alarm Buy System' (p.51-77), Vol. II System 4 'ADX/DMI Trading Systems' (p.1-30) and System 5 'The 1010 Trading System' (p.32-54), Vol. III System 7 'Volume Spike Trading System' (p.3-32); readings in authors/batch-i/I5w and I6w; independent references authors/titles-1030/K21w/k21_ref.py and K22w/k22_ref.py",
};
