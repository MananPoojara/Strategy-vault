//! Dave Landry, "Dave Landry on Swing Trading" (M. Gordon Publishing / TradingMarkets, 2001; text completed late 2000).
//! Source: Books/Dave Landry/David Landry - David Landry on Swing Trading.pdf (251 pages, scanned; printed page = PDF page - 14)
//! and the companion "Documentation for the TradeStation 2000i ... Add-On" (SwingTrade TS2000i Doc2.PDF, 10 pages, (c) Dave Landry).
//! Extraction, page refs and verification: ~/.local/share/strategy-vault/library/dlost/.
//!
//! Conventions: conditions are read on the close of bar i with data <= i. Pattern entries are buy (sell) stops for the next bar,
//! filled at max(open, level) (short: min). Pattern exits = the book's position management (Ch 2 rule 7 p.13, Ch 16 Q&A p.206):
//! initial stop, half off when the open profit equals the initial risk with the stop on the rest moved to breakeven, the rest trailed
//! 1/16 below the lowest low of the prior two bars. Same bar: the stop is checked before the target (conservative); a gap through
//! a stop or target fills at the open. Market-timing systems (Ch 10) enter and exit on the close.
use crate::connors::{claim, sma, true_range, wilder, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

const BOOK: &str = "dlost";

const MM: &str = "X|Initial stop: \"1/16 below the lowest bar\" of the setup (TKO: below the knockout bar; DT-KO: 1/16 below yesterday's low); \"If this is more than 5% away from your entry, risk no more than 5% of the stock's value\" (pp.12, 44, 62)";
const MM2: &str = "X|2-for-1: \"as soon as your profits are equal to or greater than your initial risk, you should lock in half of your profits and move your protective stop on your remaining shares to breakeven\" (p.13)";
const MM3: &str = "X|Remainder: \"trail a stop below the lowest low of the prior two bars (for longs)\" (p.206; Fig 15.3 p.194 \"Here we used a simple two-bar low\"); the stop only ratchets in the trade's favour";
const TREND: &str = "F|\"The market should be in a strong trend as defined by a computer-based indicator such as ADX >= 30 and +DMI > -DMI\" (shorts: -DMI > +DMI); ADX = Wilder 14-day (p.230)";
const PAT_NOTE: &str = "Patterns are for stocks and indices (Ch 10 applies them to the Nasdaq and S&P). Run on NSE basket stocks (basket:<list>:<SYMBOL>) or indices; India = instrument swap only. Book prices in 1/16ths: tick (default 1/16 = 0.0625 price units) is a parameter. Not applied: the Ch 4 stock-selection screen (price >= $20, 10-day volume >= 100,000, range >= 2 points; dollar thresholds have no instrument-swap equivalent) except HV >= 40 as the optional min_hv; the discretionary 'continue to scale out' and the 'usually out by the seventh day unless in a strong trend' time stop (p.206). The book publishes no results for the patterns.";

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    let d = |id, name, group, dir, status, rules, notes| StrategyDef {
        id, name, group, session: 0, dir, status, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None,
    };
    let with = |mut s: StrategyDef, c| { s.claim = c; s };
    let vx = |s: StrategyDef| StrategyDef { needs_vix: true, ..s };
    let disc = |id, name, group, why: &'static str, rules| d(id, name, group, "Long/Short", NotTestable(why), rules, why);
    vec![
        d("dlost_simple_pb", "Simple Pullback", "Ch 5 · Pullbacks", "Long/Short", Live,
          &[TREND,
            "F|\"The new high should be at least a two-month calendar high (approximately 43 trading days)\" (p.37): the bar's high is the highest of the last 43 bars",
            "E|\"After at least three, but no more than seven lower highs, look to go long tomorrow if the stock can trade 1/16 above today's high\" (p.43): 3-7 consecutive lower highs, the first one right after the new-high bar",
            "E|Shorts reversed: 3-7 consecutive higher lows after a 43-day low; sell 1/16 below today's low",
            MM, MM2, MM3],
          PAT_NOTE),
        d("dlost_od_pb", "Outside-Day Pullback", "Ch 5 · Pullbacks", "Long/Short", Live,
          &[TREND,
            "F|A pullback as in the Simple Pullback (43-day high, then 3-7 consecutive lower highs) ending yesterday",
            "E|\"An outside day occurs when a stock trades both below the prior day's low and above the prior day's high\" (p.53); go long tomorrow 1/16 above the outside day's high (Figs 5.17-5.18, p.54-55)",
            "E|Shorts reversed",
            MM, MM2, MM3],
          "Read from the examples (p.53-55) and Landry's TradeStation add-on: \"DL OutDay PB-Buy - Same as Pullback Buy, but the last bar of setup must be an outside day\" (TS doc p.7). \"Especially true when the stock closes strongly\" is not required (book: 'especially'). Same pattern notes as the Simple Pullback."),
        d("dlost_tko", "Trend Knockout (TKO)", "Ch 6 · Fakeouts and False Moves", "Long/Short", Live,
          &[TREND,
            "E|\"The market should make at least a two-bar low. Buy tomorrow or the next day (allow yourself two days to get filled), 1/16 above today's high\" (p.62): today's low below the lows of the two prior bars",
            "X|\"Place a protective stop below the low of the knockout bar\" (coded 1/16 below), 5% cap (p.62)",
            "E|Shorts reversed: today's high above the two prior highs; sell 1/16 below today's low",
            MM2, MM3],
          "A newer knockout bar replaces a pending order. Same pattern notes as the Simple Pullback."),
        d("dlost_dtko", "Double Top Knockout (DT-KO)", "Ch 6 · Fakeouts and False Moves", "Long/Short", Live,
          &[TREND,
            "F|\"The market must make a new two-month calendar high\" and \"another new high no-sooner-than three trading days later\" (p.68): the latest 43-day high comes at least 3 bars after the previous 43-day high",
            "E|\"The market must then make (at least) a two-bar low ... Buy tomorrow 1/16 above today's high\" (p.69); no new high between the second high and the two-bar low",
            "X|\"If filled, place a protective stop 1/16 below yesterday's low\" (the knockout bar), 5% cap (p.69)",
            "E|Shorts reversed",
            MM2, MM3],
          "Reading: 'no-sooner-than three trading days later' = the bars between the two highs are not new highs (the false double top), which is what the figure shows. Same pattern notes as the Simple Pullback."),
        d("dlost_tpp", "Trend Pivot (False Rally) Pullback", "Ch 6 · Fakeouts and False Moves", "Long/Short", Live,
          &[TREND,
            "F|\"The market must make a two-month calendar high\"; \"must subsequently make at least one, but no more than five, lower highs\" (p.75)",
            "E|\"The market must stage a one-day rally (a high greater than the prior day's high) followed by a lower high. This is known as a pivot high. Go long tomorrow or the next day 1/16 above this pivot high\" (p.76)",
            "X|\"If filled, place a protective stop 1/16 below the lowest bar in the setup\", 5% cap (p.76)",
            "E|Shorts reversed",
            MM2, MM3],
          "Setup completes on the lower high after the pivot; the order works that day +1 and +2. Same pattern notes as the Simple Pullback."),
        d("dlost_bow_tie", "Bow Tie", "Ch 7 · Bow Ties", "Long/Short", Live,
          &["F|10-period simple, 20-period exponential and 30-period exponential moving averages of the close",
            "F|\"The moving averages should converge and spread out again, shifting from proper downtrend order (10-SMA < 20-EMA < 30-EMA) to proper uptrend order (10-SMA > 20-EMA > 30-EMA). Ideally, this should happen over a period of three to four days\" (p.84): uptrend order today, downtrend order on one of the previous `shift` (4) days",
            "E|\"The market must make a lower low. That is, today's low must be less than yesterday's low\" (p.85); within `max_wait` (20) bars of the shift (Landry's TS add-on Max_Wait(20), TS doc p.8)",
            "E|\"Place a buy order 1/16 above today's high good for tomorrow. Continue to work an order above today's high good for the next trading day until filled or the market trades below its 20-period exponential moving average\" (p.85)",
            "X|\"Place a protective stop below the lowest bar in the setup\" (1/16 below the lowest low since the shift), 5% cap (p.85)",
            "E|Shorts reversed",
            MM2, MM3],
          "EMA = TradeStation XAverage (alpha 2/(n+1), seeded with the first close). Same pattern notes as the Simple Pullback."),
        with(d("dlost_oss", "Oscillator Swing System (OSS)", "Ch 10 · Market Timing", "Long/Short", Live,
          &["F|3-10 oscillator = 3-period simple moving average minus 10-period simple moving average of the close (p.129)",
            "F|Buys: \"The market must not be in a downtrend, as defined by an ADX >= 30 and -DMI > +DMI\" (ADX 14, TS add-on ADXlen(14))",
            "E|Buys: \"The 3-10 oscillator must have at least five consecutive down days. If the market closes above the open, go long on the close\" (the 5 down days end today)",
            "E|Shorts: not in an uptrend (ADX >= 30 and +DMI > -DMI); oscillator up at least five consecutive days; close below the open: short on the close",
            "X|\"Exit on the close three to seven days later\"; the published test exits \"on the seventh day\" (p.131): close of the 7th bar after entry",
            "X|An opposite signal reverses the position (TradeStation one-contract test; Fig 10.4 shows a short reversed to long); a same-direction signal is ignored"],
          "Book: continuous S&P 500 futures ($250/pt), 01/05/1990-10/02/2000, no stops. Here SPX / NIFTY 50 cash index (instrument swap)."), claim("143 (L 83 / S 60)", "65% (L 71% / S 57%)", "$1,453.85 (L $2,018.52 / S $672.71)", "7 days", "S&P futures 01/05/1990-10/02/2000, exit 7th day, no stops")),
        vx(with(d("dlost_cvr3m", "CVR III-Modified", "Ch 10 · Market Timing", "Long/Short", Live,
          &["E|Buys: \"Today, the low of the VIX must be above its 10-day moving average. Today, the VIX must close at least 10% above its 10-day moving average. Today, the VIX must close below its open. Once rules 1 through 3 have been met, buy the market on the close\" (p.139)",
            "X|Buys: \"Exit (on the close) the day the VIX trades (intraday) below yesterday's 10-day moving average\"",
            "E|Shorts: VIX high below its 10-day MA, VIX close at least 10% below it, VIX close above its open: sell short on the close",
            "X|Shorts: exit on the close the day the VIX trades above yesterday's 10-day MA",
            "X|An opposite signal reverses; no stops (p.141)"],
          "10-day MA = simple average of the VIX close. 'Its 10-day moving average' on the entry day = today's MA (literal; ma_lag = 1 is the [Not from the book] yesterday's-MA reading other vault modules fitted to the ATS CVR III list). Book: S&P futures 04/26/1993-10/02/2000 with the CBOE VIX (then the OEX-based index, now VXO: src = 1 reads VXO_OHLC). India: India VIX + NIFTY 50."), claim("63 (L 45 / S 18)", "76% (L 84% / S 56%)", "$2,389.09 (L $2,354.17 / S $2,476.39)", "3 bars avg", "S&P futures 04/26/1993-10/02/2000, no stops"))),
        disc("dlost_snapback", "Snapbacks (deep pullbacks)", "Ch 5 · Pullbacks", "not testable: depth is 'say 15% or greater' and the entry day inside the pullback is chosen by eye (p.46-50)",
          &["F|Deep pullback, \"say 15% or greater\", of a stock in a strong trend after a two-month high", "E|Buy tomorrow 1/16 above today's high (day picked from the chart)"]),
        disc("dlost_ipo_retrace", "IPO retracements (38.2/50/61.8%)", "Ch 5 · Pullbacks", "not testable: the low and high are picked by hand ('I had to manually pick the low and high', p.57) and IPO data is a stock-universe gap",
          &["E|Buy 1/16 above the high of the bar that retraces 38.2/50/61.8% of an IPO's first run"]),
        disc("dlost_micro_db", "High-Level Micro Double Bottom", "Ch 8 · Micro Patterns", "not testable: 'still a discretionary pattern' (p.94); second pivot low 'at or near' the first is undefined",
          &["F|Strong trend (ADX >= 30, +DMI > -DMI); two-month high", "E|Two pivot lows at or near the same level; go long 1/16 above the high of the right side of the second pivot"]),
        disc("dlost_micro_cup", "High-Level Micro Cup and Handle", "Ch 8 · Micro Patterns", "not testable: cup, base and handle are read from the chart (p.100-106)",
          &["E|Cup and handle within a strong trend above the 50-day MA, traded 'in the same fashion as pullbacks'"]),
        disc("dlost_hv_ratio", "Explosive moves: 6/100 HV ratio", "Ch 9 · Volatility", "not testable: a filter only; the book gives no entry or exit of its own ('it does not tell you in which direction the move will be', p.115)",
          &["F|HV(6) / HV(100) below 0.5 marks a low-volatility setup; combine with the patterns"]),
        disc("dlost_discretionary_timing", "Discretionary market analysis", "Ch 10 · Market Timing", "not testable: discretionary (trend qualifiers, 50/200-day MAs, sector analysis, p.125-128)",
          &["F|Trend qualifiers, 10-SMA/20-EMA/30-EMA order and 50/200-day SMAs on the indices"]),
        disc("dlost_options", "Options with swing trades", "Ch 11 · Options", "not testable: needs option chains; guidelines only (p.161-164)",
          &["E|Buy in-the-money front-month calls/puts 1-2 strikes in, only after the setup triggers; sell half on a double"]),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "dlost_simple_pb" => ("dlost_pullbacks", "Landry pullbacks", "Simple Pullback"),
        "dlost_od_pb" => ("dlost_pullbacks", "Landry pullbacks", "Outside-day pullback"),
        "dlost_snapback" => ("dlost_pullbacks", "Landry pullbacks", "Snapback"),
        "dlost_ipo_retrace" => ("dlost_pullbacks", "Landry pullbacks", "IPO retracement"),
        "dlost_tko" => ("dlost_fakeouts", "Landry fakeouts", "Trend Knockout"),
        "dlost_dtko" => ("dlost_fakeouts", "Landry fakeouts", "Double Top Knockout"),
        "dlost_tpp" => ("dlost_fakeouts", "Landry fakeouts", "Trend Pivot Pullback"),
        "dlost_bow_tie" => ("dlost_bow_tie", "Bow Tie", "Bow Tie"),
        "dlost_micro_db" => ("dlost_micro", "Micro patterns", "Micro Double Bottom"),
        "dlost_micro_cup" => ("dlost_micro", "Micro patterns", "Micro Cup and Handle"),
        "dlost_oss" => ("dlost_timing", "Landry market timing", "Oscillator Swing System"),
        "dlost_cvr3m" => ("dlost_timing", "Landry market timing", "CVR III-Modified"),
        "dlost_discretionary_timing" => ("dlost_timing", "Landry market timing", "Discretionary analysis"),
        "dlost_hv_ratio" => ("dlost_hv_ratio", "Explosive moves (HV ratio)", "6/100 HV ratio"),
        "dlost_options" => ("dlost_options", "Options", "Options"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let side = || p("side", "Side: 0 = long and short (book), 1 = long only, 2 = short only", 0.0, 0.0, 2.0, 1.0);
    let common = || vec![
        p("adx_n", "ADX / DMI length (Wilder)", 14.0, 7.0, 28.0, 1.0),
        p("adx_min", "Minimum ADX for a strong trend", 30.0, 20.0, 40.0, 5.0),
        p("tick", "Order offset (book 1/16 point)", 0.0625, 0.0, 1.0, 0.0625),
        p("max_risk", "Stop no further than % of the entry", 5.0, 2.0, 10.0, 1.0),
        p("min_hv", "[Not from the pattern rules] Ch 4 stock screen: HV(50) at least % (0 = off)", 0.0, 0.0, 80.0, 10.0),
        side(),
    ];
    let mut v = match id {
        "dlost_simple_pb" | "dlost_od_pb" => vec![
            p("nh", "New high = highest of N bars (two months)", 43.0, 20.0, 65.0, 1.0),
            p("min_lh", "Minimum consecutive lower highs", 3.0, 1.0, 5.0, 1.0),
            p("max_lh", "Maximum consecutive lower highs", 7.0, 5.0, 10.0, 1.0)],
        "dlost_tko" => vec![p("ko", "Knockout = low below the prior N lows", 2.0, 1.0, 5.0, 1.0), p("valid", "Days the entry order works", 2.0, 1.0, 3.0, 1.0)],
        "dlost_dtko" => vec![p("nh", "New high = highest of N bars (two months)", 43.0, 20.0, 65.0, 1.0),
            p("gap", "Second high at least N bars after the first", 3.0, 2.0, 10.0, 1.0), p("ko", "Knockout = low below the prior N lows", 2.0, 1.0, 5.0, 1.0)],
        "dlost_tpp" => vec![p("nh", "New high = highest of N bars (two months)", 43.0, 20.0, 65.0, 1.0),
            p("max_lh", "Maximum lower highs before the pivot", 5.0, 1.0, 8.0, 1.0), p("valid", "Days the entry order works", 2.0, 1.0, 3.0, 1.0)],
        "dlost_bow_tie" => vec![p("fast", "Simple MA length", 10.0, 5.0, 20.0, 1.0), p("mid", "Middle EMA length", 20.0, 10.0, 30.0, 1.0),
            p("slow", "Slow EMA length", 30.0, 20.0, 50.0, 1.0), p("shift", "Down-order to up-order within N days (book: 'ideally three to four')", 4.0, 1.0, 10.0, 1.0),
            p("max_wait", "Lower low within N bars of the shift (TS Max_Wait)", 20.0, 5.0, 40.0, 1.0)],
        "dlost_oss" => return Some(vec![p("fast", "Oscillator fast SMA", 3.0, 2.0, 5.0, 1.0), p("slow", "Oscillator slow SMA", 10.0, 5.0, 20.0, 1.0),
            p("days", "Consecutive oscillator days", 5.0, 3.0, 8.0, 1.0), p("adx_n", "ADX / DMI length (Wilder)", 14.0, 7.0, 28.0, 1.0),
            p("adx_min", "ADX level of a trend", 30.0, 20.0, 40.0, 5.0), p("hold", "Exit on the close N days after entry (book 3-7; tested 7)", 7.0, 3.0, 7.0, 1.0), side()]),
        "dlost_cvr3m" => return Some(vec![p("ma", "VIX SMA length", 10.0, 5.0, 20.0, 1.0), p("pct", "VIX close at least % from its MA", 10.0, 5.0, 20.0, 1.0),
            p("ma_lag", "[Not from the book] entry MA as of N days ago (book: today's = 0)", 0.0, 0.0, 1.0, 1.0),
            p("src", "Volatility index: 0 = server VIX, 1 = VXO_OHLC (book's OEX VIX)", 0.0, 0.0, 1.0, 1.0), side()]),
        _ => return None,
    };
    v.extend(common());
    Some(v)
}

// ------------------------------------------------------------------ indicators

/// Wilder +DI, -DI, ADX (same arithmetic as connors::dmi, which returns only +DI and ADX).
fn dmi3(x: &Ctx, n: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let len = x.c.len();
    let (mut pdm, mut mdm, mut tr) = (vec![f64::NAN; len], vec![f64::NAN; len], vec![f64::NAN; len]);
    let trs = true_range(x.r);
    for i in 1..len {
        let (up, dn) = (x.h[i] - x.h[i - 1], x.l[i - 1] - x.l[i]);
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
    (pdi, mdi, wilder(&dx, n))
}

/// TradeStation XAverage: alpha = 2/(n+1), seeded with the first value.
fn ema(v: &[f64], n: usize) -> Vec<f64> {
    let a = 2.0 / (n as f64 + 1.0);
    let mut out = vec![f64::NAN; v.len()];
    for i in 0..v.len() {
        out[i] = if i == 0 { v[0] } else { out[i - 1] + a * (v[i] - out[i - 1]) };
    }
    out
}

/// HV as the book's appendix: stdev(ln C/C[-1], n) * 100 * sqrt(256) (p.211).
fn hv256(c: &[f64], n: usize) -> Vec<f64> {
    let mut out = vec![f64::NAN; c.len()];
    for i in n..c.len() {
        let r: Vec<f64> = (i + 1 - n..=i).map(|k| (c[k] / c[k - 1]).ln()).collect();
        let m = r.iter().sum::<f64>() / n as f64;
        let var = r.iter().map(|v| (v - m).powi(2)).sum::<f64>() / (n as f64 - 1.0);
        out[i] = var.sqrt() * 100.0 * 16.0;
    }
    out
}

// ------------------------------------------------------------------ pattern engine

/// A stop order working on bar `i`: enter at `level` (long: buy stop; short: sell stop), initial stop base `stop`.
#[derive(Clone, Copy)]
struct Order { dir: f64, level: f64, stop: f64, signal: usize }

/// Landry position management on a sequence of working orders (one position at a time).
fn manage(x: &Ctx, from: usize, to: usize, orders: &[Vec<Order>], tick: f64, max_risk: f64) -> Vec<Trade> {
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let mut out = Vec::new();
    let mut i = from;
    let mut used: Vec<(f64, usize)> = Vec::new(); // a setup is traded once even when its order works on several days
    while i <= to {
        // entry: first working order that trades
        let fill = orders[i].iter().filter(|od| !used.contains(&(od.dir, od.signal))).find_map(|od| {
            let px = if od.dir > 0.0 { (h[i] >= od.level).then(|| o[i].max(od.level)) } else { (l[i] <= od.level).then(|| o[i].min(od.level)) };
            px.map(|px| (*od, px))
        });
        let Some((od, e)) = fill else { i += 1; continue };
        used.push((od.dir, od.signal));
        let d = od.dir;
        let cap = e * (1.0 - d * max_risk / 100.0);
        let s0 = if d > 0.0 { od.stop.max(cap) } else { od.stop.min(cap) };
        let risk = d * (e - s0);
        let mut t = Trade { dir: d, legs: vec![Leg { i, px: e, w: 1.0 }], exit_i: to, exit_px: c[to], reason: "open".into(),
            entry_kind: "stop", signal_i: od.signal, open: true, exits: Vec::new() };
        if !(risk > 0.0) { i += 1; continue; } // fill at or beyond the stop: no trade possible
        let target = e + d * risk;
        let (mut stop, mut half) = (s0, false);
        let mut k = i;
        let mut closed = false;
        loop {
            // stop first (on the entry bar only the part of the range after the fill can hit it: assume it can)
            let hit = if d > 0.0 { l[k] <= stop } else { h[k] >= stop };
            if hit {
                let px = if k > i { if d > 0.0 { o[k].min(stop) } else { o[k].max(stop) } } else { stop };
                let why = if !half { "Initial stop" } else if d * (stop - e) > 1e-12 { "Two-bar trailing stop" } else { "Breakeven stop" };
                t.exits.push((k, px, if half { 0.5 } else { 1.0 }));
                t.reason = why.into();
                closed = true;
                break;
            }
            if !half && (if d > 0.0 { h[k] >= target } else { l[k] <= target }) {
                let px = if k > i { if d > 0.0 { o[k].max(target) } else { o[k].min(target) } } else { target };
                t.exits.push((k, px, 0.5));
                half = true;
                stop = e; // breakeven from the next bar
            } else if half && k >= 1 {
                // trail 1/16 beyond the extreme of the prior two bars, effective next bar, ratchet only
                let tr = if d > 0.0 { l[k].min(l[k - 1]) - tick } else { h[k].max(h[k - 1]) + tick };
                stop = if d > 0.0 { stop.max(tr) } else { stop.min(tr) };
            }
            if k >= to { break; }
            k += 1;
        }
        if closed {
            let (sum_w, sum_px): (f64, f64) = t.exits.iter().fold((0.0, 0.0), |a, e| (a.0 + e.2, a.1 + e.1 * e.2));
            t.exit_i = k;
            t.exit_px = sum_px / sum_w;
            t.signal_i = k;
            t.open = false;
            if t.exits.len() == 1 { t.exits.clear(); }
            out.push(t);
            i = k + 1;
        } else {
            // open at the end: mark the remainder at the last close
            let done: f64 = t.exits.iter().map(|e| e.2).sum();
            if done > 0.0 {
                t.exits.push((to, c[to], 1.0 - done));
                t.exit_px = t.exits.iter().map(|e| e.1 * e.2).sum();
            }
            out.push(t);
            break;
        }
    }
    out
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if from > to || to >= x.c.len() { return Some(vec![]); }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let u = |k: &str| g(k).round().max(0.0) as usize;
    let side = u("side");
    let (want_l, want_s) = (side != 2, side != 1);
    let n = x.c.len();
    match id {
        "dlost_oss" => return Some(oss(x, from, to, u("fast").max(1), u("slow").max(1), u("days").max(1), u("adx_n").max(1), g("adx_min"), u("hold").max(1), want_l, want_s)),
        "dlost_cvr3m" => return Some(cvr3m(x, from, to, u("ma").max(1), g("pct") / 100.0, u("ma_lag"), u("src"), want_l, want_s)),
        _ => {}
    }
    let (tick, max_risk, min_hv) = (g("tick"), g("max_risk"), g("min_hv"));
    let (pdi, mdi, adx) = dmi3(x, u("adx_n").max(1));
    let amin = g("adx_min");
    let hvv = if min_hv > 0.0 { hv256(&x.c, 50) } else { vec![f64::INFINITY; n] };
    let trend = |i: usize, d: f64| adx[i] >= amin && (if d > 0.0 { pdi[i] > mdi[i] } else { mdi[i] > pdi[i] }) && hvv[i] >= min_hv;
    let (h, l) = (&x.h, &x.l);
    // mirrored price helpers: for shorts, highs become lows
    let hi = |i: usize, d: f64| if d > 0.0 { h[i] } else { -l[i] };
    let lo = |i: usize, d: f64| if d > 0.0 { l[i] } else { -h[i] };
    let new_hi = |i: usize, d: f64, nb: usize| i + 1 >= nb && (i + 1 - nb..=i).all(|k| hi(i, d) >= hi(k, d));
    // consecutive lower highs ending at i
    let lh_run = |i: usize, d: f64| { let mut k = 0; while i > k && hi(i - k, d) < hi(i - k - 1, d) { k += 1; } k };
    let knock = |i: usize, d: f64, ko: usize| i >= ko && (1..=ko).all(|j| lo(i, d) < lo(i - j, d));
    let lowest = |a: usize, b: usize, d: f64| (a..=b).map(|k| lo(k, d)).fold(f64::INFINITY, f64::min);
    // order for a mirrored entry level `lv` (price above for longs) and mirrored stop base `sb`
    let mk = |d: f64, lv: f64, sb: f64, sig: usize| Order { dir: d, level: d * lv, stop: d * sb, signal: sig };
    let mut orders: Vec<Vec<Order>> = vec![Vec::new(); n + 1];
    let dirs: Vec<f64> = [(want_l, 1.0), (want_s, -1.0)].iter().filter(|a| a.0).map(|a| a.1).collect();
    let start = from.saturating_sub(250); // setups that began before `from` may still produce orders inside it
    for &d in &dirs {
        match id {
            "dlost_simple_pb" | "dlost_od_pb" => {
                let (nb, a, b) = (u("nh").max(1), u("min_lh"), u("max_lh"));
                let od = id == "dlost_od_pb";
                for t in start.max(2)..n {
                    let end = if od { t - 1 } else { t };
                    if od && !(hi(t, d) > hi(t - 1, d) && lo(t, d) < lo(t - 1, d)) { continue; }
                    let k = lh_run(end, d);
                    if k < a || k > b || !new_hi(end - k, d, nb) || !trend(t, d) { continue; }
                    orders[t + 1].push(mk(d, hi(t, d) + tick, lowest(end - k + 1, t, d) - tick, t));
                }
            }
            "dlost_tko" => {
                let (ko, valid) = (u("ko").max(1), u("valid").max(1));
                let mut last: Option<usize> = None;
                for t in start..n {
                    if knock(t, d, ko) && trend(t, d) { last = Some(t); }
                    // working on t+1: the most recent knockout bar within `valid` days
                    if let Some(s) = last.filter(|&s| t + 1 - s <= valid) {
                        orders[t + 1].push(mk(d, hi(s, d) + tick, lo(s, d) - tick, s));
                    }
                }
            }
            "dlost_dtko" => {
                let (nb, gap, ko) = (u("nh").max(1), u("gap").max(1), u("ko").max(1));
                let (mut prev_nh, mut armed): (Option<usize>, bool) = (None, false);
                for t in start..n {
                    if new_hi(t, d, nb) {
                        armed = prev_nh.is_some_and(|p| t - p >= gap);
                        prev_nh = Some(t);
                        continue;
                    }
                    if armed && knock(t, d, ko) && trend(t, d) {
                        orders[t + 1].push(mk(d, hi(t, d) + tick, lo(t, d) - tick, t));
                    }
                }
            }
            "dlost_tpp" => {
                let (nb, mx, valid) = (u("nh").max(1), u("max_lh").max(1), u("valid").max(1));
                for t in start.max(3)..n {
                    // t = lower high after pivot P = t-1; P = one-day rally; before P: k lower highs after a new high
                    let pv = t - 1;
                    if !(hi(t, d) < hi(pv, d) && hi(pv, d) > hi(pv - 1, d)) { continue; }
                    let k = lh_run(pv - 1, d);
                    if k < 1 || k > mx || !new_hi(pv - 1 - k, d, nb) || !trend(t, d) { continue; }
                    let o = mk(d, hi(pv, d) + tick, lowest(pv - k, t, d) - tick, t);
                    for w in 1..=valid { if t + w <= n { orders[t + w].push(o); } }
                }
            }
            "dlost_bow_tie" => {
                let (f, m, s) = (sma(&x.c, u("fast").max(1)), ema(&x.c, u("mid").max(1)), ema(&x.c, u("slow").max(1)));
                let (shift, wait) = (u("shift").max(1), u("max_wait"));
                let up = |i: usize| if d > 0.0 { f[i] > m[i] && m[i] > s[i] } else { f[i] < m[i] && m[i] < s[i] };
                let dn = |i: usize| if d > 0.0 { f[i] < m[i] && m[i] < s[i] } else { f[i] > m[i] && m[i] > s[i] };
                let below = |i: usize| if d > 0.0 { l[i] < m[i] } else { h[i] > m[i] };
                let mut t = start.max(1);
                while t < n {
                    // shift day: up-order today, down-order within the previous `shift` days, not up-order yesterday
                    if !(up(t) && !up(t - 1) && (t.saturating_sub(shift)..t).any(|k| dn(k))) { t += 1; continue; }
                    let sday = t;
                    let ll = (sday..n.min(sday + wait + 1)).find(|&k| k >= 1 && lo(k, d) < lo(k - 1, d));
                    let Some(mut k) = ll else { t += 1; continue };
                    // work an order above each day's high until filled or the market trades through the 20-EMA
                    loop {
                        if below(k) || k + 1 > n { break; }
                        orders[k + 1].push(mk(d, hi(k, d) + tick, lowest(sday, k, d) - tick, k));
                        let filled = k + 1 < n && hi(k + 1, d) >= hi(k, d) + tick;
                        k += 1;
                        if filled || k >= n { break; }
                    }
                    t = k.max(t + 1);
                }
            }
            _ => return None,
        }
    }
    // one position at a time; when both sides have an order on a bar the earlier-listed (long) is tried first
    Some(manage(x, from, to, &orders, tick, max_risk))
}

// ------------------------------------------------------------------ Chapter 10 systems (on the close)

fn oss(x: &Ctx, from: usize, to: usize, fast: usize, slow: usize, days: usize, adx_n: usize, amin: f64, hold: usize, wl: bool, ws: bool) -> Vec<Trade> {
    let (a, b) = (sma(&x.c, fast), sma(&x.c, slow));
    let osc: Vec<f64> = a.iter().zip(&b).map(|(a, b)| a - b).collect();
    let (pdi, mdi, adx) = dmi3(x, adx_n);
    let run = |i: usize, up: bool| i >= days && (i + 1 - days..=i).all(|k| if up { osc[k] > osc[k - 1] } else { osc[k] < osc[k - 1] });
    let sig: Vec<i8> = (0..x.c.len()).map(|i| {
        let down_trend = adx[i] >= amin && mdi[i] > pdi[i];
        let up_trend = adx[i] >= amin && pdi[i] > mdi[i];
        if wl && !down_trend && run(i, false) && x.c[i] > x.o[i] { 1 } else if ws && !up_trend && run(i, true) && x.c[i] < x.o[i] { -1 } else { 0 }
    }).collect();
    close_sar(x, from, to, &sig, &|_, _, _| None, hold)
}

fn cvr3m(x: &Ctx, from: usize, to: usize, len: usize, pct: f64, lag: usize, src: usize, wl: bool, ws: bool) -> Vec<Trade> {
    let n = x.c.len();
    let b = if src == 1 { "VXO_OHLC" } else { "VIX" };
    let col = |k: &str| x.series(&format!("{b}.{k}")).map(<[f64]>::to_vec).unwrap_or_else(|| vec![f64::NAN; n]);
    let (o, h, l, c) = (col("Open"), col("High"), col("Low"), col("Close"));
    // the VIX's own consecutive bars
    let days: Vec<usize> = (0..n).filter(|&i| [o[i], h[i], l[i], c[i]].iter().all(|v| v.is_finite())).collect();
    let vc: Vec<f64> = days.iter().map(|&i| c[i]).collect();
    let ma = sma(&vc, len);
    let mut at = vec![None; n];
    for (k, &i) in days.iter().enumerate() { at[i] = Some(k); }
    let m = |k: usize| if k >= lag { ma[k - lag] } else { f64::NAN };
    let sig: Vec<i8> = (0..n).map(|i| at[i].map_or(0, |k| {
        if wl && l[i] > m(k) && c[i] >= m(k) * (1.0 + pct) && c[i] < o[i] { 1 }
        else if ws && h[i] < m(k) && c[i] <= m(k) * (1.0 - pct) && c[i] > o[i] { -1 } else { 0 }
    })).collect();
    let back = |i: usize, d: f64, _e: usize| at[i].and_then(|k| (k > 0 && if d > 0.0 { l[i] < ma[k - 1] } else { h[i] > ma[k - 1] })
        .then(|| "VIX traded back through yesterday's 10-day MA".to_string()));
    close_sar(x, from, to, &sig, &back, 0)
}

/// Enter on the signal close; exit on the close when `xf` fires or `hold` bars after entry (0 = no time exit);
/// an opposite signal reverses on the same close; same-direction signals are ignored. Re-entry on the exit close is allowed.
fn close_sar(x: &Ctx, from: usize, to: usize, sig: &[i8], xf: &dyn Fn(usize, f64, usize) -> Option<String>, hold: usize) -> Vec<Trade> {
    let c = &x.c;
    let open = |i: usize, s: i8| Trade { dir: s as f64, legs: vec![Leg { i, px: c[i], w: 1.0 }], exit_i: i, exit_px: c[i], reason: String::new(),
        entry_kind: "close", signal_i: i, open: false, exits: Vec::new() };
    let mut out = Vec::new();
    let mut cur: Option<Trade> = None;
    for i in from..=to {
        let s = sig[i];
        if let Some(mut t) = cur.take() {
            let e = t.legs[0].i;
            let why = if s != 0 && s as f64 == -t.dir { Some("Opposite signal: stop and reverse".to_string()) }
                else { xf(i, t.dir, e).or_else(|| (hold > 0 && i - e >= hold).then(|| format!("Close {hold} days after entry"))) };
            match why {
                Some(w) => {
                    t.exit_i = i; t.exit_px = c[i]; t.signal_i = i; t.reason = w;
                    out.push(t);
                    if s != 0 && i < to { cur = Some(open(i, s)); }
                }
                None => cur = Some(t),
            }
            continue;
        }
        if s != 0 && i < to { cur = Some(open(i, s)); }
    }
    if let Some(mut t) = cur { t.exit_i = to; t.exit_px = c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true; out.push(t); }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn bars(v: &[(f64, f64, f64, f64)]) -> Vec<Row> {
        v.iter().enumerate().map(|(i, &(o, h, l, c))| Row { ticker: "T".into(), date: format!("2000-01-{:02}", i + 1), open: o, high: h, low: l, close: c, volume: None }).collect()
    }
    /// Book Ch 16 / persistent-pullback worked example: entry 45, stop 43, target 47; half at 47, rest stopped on the trail.
    #[test]
    fn two_for_one_then_two_bar_trail() {
        let r = bars(&[(44.0, 44.9, 44.0, 44.5), (44.8, 46.2, 44.7, 46.0), (46.0, 47.5, 45.8, 46.5), (46.5, 48.2, 46.4, 48.0), (47.9, 48.0, 45.0, 45.5)]);
        let x = Ctx::new(&r, &r, &[]);
        let mut orders = vec![Vec::new(); 6];
        orders[1].push(Order { dir: 1.0, level: 45.0, stop: 43.0, signal: 0 });
        let t = manage(&x, 0, 4, &orders, 0.0, 5.0);
        assert_eq!(t.len(), 1);
        assert_eq!(t[0].legs[0].px, 45.0);
        // 5% cap: 45 * 0.95 = 42.75 < 43 so stop 43, risk 2, target 47 hit on bar 2
        assert_eq!(t[0].exits[0], (2, 47.0, 0.5));
        // trail after bar 3 = min(low[3], low[2]) = 45.8 -> stopped on bar 4 at 45.8
        assert_eq!(t[0].exits[1].0, 4);
        assert!((t[0].exits[1].1 - 45.8).abs() < 1e-9);
        assert!((t[0].exit_px - (47.0 + 45.8) / 2.0).abs() < 1e-9);
    }
    #[test]
    fn five_percent_cap_and_gap_stop() {
        let r = bars(&[(100.0, 101.0, 99.0, 100.0), (100.5, 102.0, 100.0, 101.0), (90.0, 91.0, 89.0, 90.0)]);
        let x = Ctx::new(&r, &r, &[]);
        let mut orders = vec![Vec::new(); 4];
        orders[1].push(Order { dir: 1.0, level: 101.0, stop: 80.0, signal: 0 });
        let t = manage(&x, 0, 2, &orders, 0.0, 5.0);
        assert_eq!(t[0].legs[0].px, 101.0);
        assert_eq!(t[0].exit_px, 90.0); // stop 95.95 gapped through: filled at the open
        assert_eq!(t[0].reason, "Initial stop");
    }
}
