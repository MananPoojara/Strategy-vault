//! Andrew Goodwin, "Trading Secrets of the Inner Circle" (Traders Press / Goodwin, 1997; book id `agtsic`; printed pages).
//! Reading: claude-continuation-20261020/authors/goodwin (admission/goodwin_ref.py). Every rule is the book's own TradeStation
//! code (S&P 500 futures, daily bars, Prophet continuous contract 1982-1997): basic 8-day RSI (Ch.1 p.11-19), options-expiration
//! volatility disturbance (Ch.4 p.39-47), daily patterns #5, #6, #8, #10, #12 (Ch.5 p.53-63) and the 6-day reversal (Ch.6 p.85-91).
//! Breadth-filtered 8-day RSI (Ch.1 p.11-12, code "Chapter1 A/D Filter" p.22): coded by K25 (3 Nov 2026) on the vault's NYSE
//! breadth (US) and NSE breadth (India twin), admitted by ADMISSION_BREADTH below.
//! Held (not coded): interest-rate filtered RSI (T-note and T-bill futures), VIX model (the VIX is not
//! traded), crash filter (note and dollar futures), patterns with fixed S&P point amounts (#1, #3, #4, #7, #9, #11, #13, #14),
//! pattern #2 (no code, trend test undefined), silver symmetry (silver futures), range-expansion search (a search, not a rule),
//! Value Line earnings method (news data).
//! Book: S&P 500 futures; indices, ETFs and stocks are a declared application.
use crate::connors::{Leg, Ctx, ParamDef, Params, Status, StrategyDef, Trade};

/// Admitted ids (authors/goodwin/admission/goodwin_ref.py (header)).
pub const REVIEWED: &[&str] = &["agtsic_rsi8", "agtsic_expiry", "agtsic_p5", "agtsic_p6", "agtsic_p8", "agtsic_p10", "agtsic_p12", "agtsic_rev6"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }
/// K25: breadth-filtered RSI on NYSE breadth (US instruments) and NSE breadth (Indian instruments); own admission entry.
pub const REVIEWED_BREADTH: &[&str] = &["agtsic_rsi8_breadth", "agtsic_rsi8_breadth_nse"];
pub fn reviewed_breadth(id: &str) -> bool { REVIEWED_BREADTH.contains(&id) }
const NYSE_AD: &[&str] = &["NYSE_ADV_DEC.Advances", "NYSE_ADV_DEC.Declines"];
const NSE_AD: &[&str] = &["NSE_ADV_DEC.Advances", "NSE_ADV_DEC.Declines"];
const BREADTH_RULES: &[&str] = &[
    "F|\"Calculate the 10 day simple moving average of the number of advancing issues on the NYSE\" and of the declining issues; \"Divide the 10 day average of advances by the 10 day average of declines\" (p.11-12); code: \"adv=average(close,10) of data2; dec=average(close,10) of data3; quotient=adv/dec;\" (Chapter1 A/D Filter, p.22)",
    "F|\"If the value of the division is greater than .75 or 75% then our market breadth filter is considered positive. If the value of the division is less than .50 then our market breadth filter is considered negative\" (p.12); code: \"if quotient>.75 then condition1=true; if quotient<.50 then condition1=false;\" - between the two levels the filter keeps its last state (starts negative, TradeStation's initial false)",
    "E|\"If the breadth filter is positive then buy the S&P 500 contracts on the close of the day when the 8 day RSI crosses from below to above 30\" (p.12); code: \"if condition1=true and rsi(close,8) crosses above 30 then buy on close;\"",
    "X|\"Exit your long position if the 8 day RSI crosses from above to below 65\" and \"if the market breadth filter changes from positive to negative\" (p.12); code: \"if rsi(close,8) crosses below 65 then exitlong on close; if condition1=false then exitlong on close;\"",
];
const BREADTH_NOTE: &str = "Book: S&P 500 futures 4/82-4/97 with NYSE breadth; profit factor 4.55 (basic system 3.29), W/L ratio .71, net profit $28,000 above the basic system, maximum drawdown nearly 65% smaller (p.12); the full record is an image page. RSI(8), crossings, fills at the close and one position at a time exactly as agtsic_rsi8. A day without a breadth figure leaves the filter unchanged for the ten days its averages need [convention]. NYSE breadth in the vault runs 1965-03-01 to 2020-02-10, so US results end on 2020-02-10 at the latest; it is missing on a few days (for example 1999-11-26, 2002-06-20, 2019-08-20), so a full-history run uses the longest stretch without a gap (2002-06-21 to 2019-08-19 on SPX, SPY and the other ETFs). Costs are not modelled.";

const NOTE_DAY: &str = "[Declared readings] TradeStation conventions: range = high - low of the bar; 'tomorrow at X stop' is a one-day buy stop: it fills at X, or at the open when the open is already above X; 'Set TradeStation to close trade at end of day' = the trade exits at the close of the entry day. Day of week: TradeStation 1 = Monday ... 5 = Friday, on the signal bar; 'tomorrow' is the next trading day. One trade at a time. Costs ($75 per trade in the book) are not modelled.";

fn day(id: &'static str, name: &'static str, group: &'static str, rules: &'static [&'static str]) -> StrategyDef {
    StrategyDef { id, name, group, session: 0, dir: "Long", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "agtsic", rules, notes: NOTE_DAY, claim: None }
}

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        StrategyDef { id: "agtsic_rsi8", name: "Basic 8-day RSI (buy the cross above 30, sell the cross below 65)", group: "Ch.1 Market Wizard Filter (p.11-19)",
            session: 0, dir: "Long", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "agtsic",
            rules: &["E|\"If rsi(close,8) crosses above 30 then buy on close;\" (Chapter1 Basic 8 day, p.18)",
                "X|\"if rsi(close,8) crosses below 65 then exitlong on close;\" (p.18)",
],
            notes: "The book shows this basic system only to improve it with breadth and interest-rate filters (p.11-14); the breadth-filtered version is agtsic_rsi8_breadth (NYSE) / agtsic_rsi8_breadth_nse (NSE); the Treasury version needs T-note and T-bill futures and is held. [Declared readings] RSI(8) = TradeStation RSI: Wilder averages of the up and down close-to-close changes, seeded with the 8-change simple average (the text's 'average of the last 8 up days', p.11, read as this). 'Crosses above 30' = RSI above 30 today and at or below 30 yesterday; 'crosses below 65' = below 65 today and at or above 65 yesterday. Entry and exit at the close; one position at a time; no stop (the book has none). Costs are not modelled.",
            claim: None },
        StrategyDef { id: "agtsic_rsi8_breadth", name: "Breadth-filtered 8-day RSI · NYSE breadth (advances/declines 10-day ratio above .75 / below .50)", group: "Ch.1 Market Wizard Filter (p.11-19)",
            session: 0, dir: "Long", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: NYSE_AD, book: "agtsic",
            rules: BREADTH_RULES, notes: BREADTH_NOTE, claim: None },
        StrategyDef { id: "agtsic_rsi8_breadth_nse", name: "Breadth-filtered 8-day RSI · NSE breadth (India)", group: "Ch.1 Market Wizard Filter (p.11-19)",
            session: 0, dir: "Long", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: NSE_AD, book: "agtsic",
            rules: BREADTH_RULES, notes: "India mapping (declared application): NSE advances and declines (extra_data/india NSE_ADV_DEC, 1999-01-01 to 2026-10-06) instead of NYSE breadth, on Indian indices, ETFs and stocks; the book's .75 / .50 levels unchanged (set for NYSE breadth). Otherwise identical to agtsic_rsi8_breadth.", claim: None },
        StrategyDef { id: "agtsic_expiry", name: "Options expiration: unusual range five days before expiration, hold to expiration", group: "Ch.4 Trading the Options Expiration Method (p.39-47)",
            session: 0, dir: "Long", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "agtsic",
            rules: &["E|\"If the range of the day five trading days before expiration day is less than 70% of the average range of the previous three days or is greater than 140% of the average range of the previous three days then an unusual volatility disturbance has taken place.\" (p.40-41)",
                "E|\"Given an unusual volatility disturbance five days before expiration, buy the S&P 500 futures on the close of the unusual disturbance day.\" (p.41)",
                "X|\"Hold the long futures position until the close of options expiration day.\" (p.41)"],
            notes: "[Declared readings] Expiration day = the third Friday of the month (US monthly equity/index options); when the third Friday has no trading bar (exchange holiday) the last trading day before it is used, as the book's calendar does for holiday Thursdays (code comment, p.45). Five trading days before = five bars before the expiration bar. Range = high - low; the average range of the previous three days = the three bars before the signal day. Buy at that close, sell at the expiration day's close. Runs on US instruments only: Indian options expire on a different calendar. Costs are not modelled.",
            claim: None },
        day("agtsic_p5", "Daily pattern #5 'Pick Up the Plunger' (open + 45% of the smaller recent range)", "Ch.5 Reactive Day Trading (p.53-63)",
            &["E|\"value1=lowest(range,2)*.45; if high[2]<low[5] and high>highest(high,2)[1] and close<open then buy (\"Buy D5\") tomorrow at open tomorrow + value1 stop;\" (p.59)",
              "X|\"{Set TradeStation to close trade at end of day}\" (p.59)"]),
        day("agtsic_p6", "Daily pattern #6 'The Fakeout Follow Up' (gap above yesterday's high that closes lower)", "Ch.5 Reactive Day Trading (p.53-63)",
            &["E|\"value1=highest(range,3)*.3; if dayofweek(date)<>2 and open>high[1] and close<close[1] and high-close>close-low then buy (\"Buy D6\") tomorrow at open tomorrow + value1 stop;\" (p.59)",
              "X|\"{Set TradeStation to close trade at end of day}\" (p.59)"]),
        day("agtsic_p8", "Daily pattern #8 'Another Friday-Monday Effect' (two up closes, narrow day, up opening)", "Ch.5 Reactive Day Trading (p.53-63)",
            &["E|\"If (dayofweek(date)=4 or dayofweek(date)=5) and close[1]>close[2] and close>close[1] and range<range[1]*.75 and open tomorrow>close then buy (\"Buy D8\") tomorrow at open tomorrow stop;\" (p.60)",
              "X|\"{Set TradeStation to close trade at end of day}\" (p.60)"]),
        day("agtsic_p10", "Daily pattern #10 'Leveraged Buyout' (Friday above the high two days earlier, Monday rally)", "Ch.5 Reactive Day Trading (p.53-63)",
            &["E|\"valuel=lowest(range,2)*.6; if low>high[2] and dayofweek(date)=5 and range<range[1]*.95 then buy (\"Buy D10\") tomorrow at open tomorrow + valuel stop;\" (p.61)",
              "X|\"{Set TradeStation to close trade at end of day }\" (p.61)"]),
        StrategyDef { notes: "[Declared readings] The printed code (p.62) tests 'high<close[1]'; the text above it says 'the high of the day is greater than the previous day's close'. The code is the default (it produced the printed record); the text is option 'Text reading'. TradeStation conventions: range = high - low; average(range,3)[1] = the mean range of the three bars before the signal bar; 'buy tomorrow at open tomorrow stop' = buy at the next open; exit at the close of the entry day. Day of week on the signal bar (2 = Tuesday, 5 = Friday). Costs are not modelled.",
            ..day("agtsic_p12", "Daily pattern #12 'Tuesday/Friday Correlation' (narrow Tuesday or Friday, buy the next open)", "Ch.5 Reactive Day Trading (p.53-63)",
            &["E|\"if (dayofweek(date)=2 or dayofweek(date)=5) and range<average(range,3)[1] and high<close[1] then buy (\"Buy D12\") tomorrow at open tomorrow stop;\" (p.62)",
              "X|\"{Set TradeStation to close trade at end of day)\" (p.62)"]) },
        StrategyDef { id: "agtsic_rev6", name: "Short-term reversal: 6-day extreme close, stop at the 3-day close extreme, out after five days", group: "Ch.6 Short-Term Reversal Systems (p.85-91)",
            session: 0, dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "agtsic",
            rules: &["E|\"If close<lowest(low,6)[1] then buy tomorrow at highest(close,3)[1] stop;\" \"if close>highest(high,6)[1] then sell tomorrow at lowest(close,3)[1] stop;\" (Chap6 Reversals, p.91)",
                "X|\"if barssinceentry=4 then exitlong on close;\" \"if barssinceentry=4 then exitshort on close;\" (p.91) - \"we stay short the market for five days including the day of entry and exit on the close of the fifth day\" (p.86-87)"],
            notes: "[Declared readings] The printed code (p.91) is the default: the stop is the highest (short: lowest) close of the three days BEFORE the signal day. The text (p.86) places it 'at the highest close of the preceding three days' just before the next open, which also counts the signal day: option 'Text reading'. The stop order lives one day and is renewed after every new signal close. It fills at the stop or at a worse open. Exit at the close of the fifth day counting the entry day. A buy signal while long (sell while short) is ignored; an opposite entry reverses the position. Shorts on a cash index are hypothetical. Costs are not modelled.",
            claim: None },
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let f = match id {
        "agtsic_rsi8_breadth" => return Some(("agtsic_rsi8_breadth", "Goodwin Inner Circle", "Breadth-filtered RSI(8) · NYSE")),
        "agtsic_rsi8_breadth_nse" => return Some(("agtsic_rsi8_breadth", "Goodwin Inner Circle", "Breadth-filtered RSI(8) · NSE")),
        "agtsic_rsi8" => "RSI(8) 30/65", "agtsic_expiry" => "Expiration range", "agtsic_p5" => "Pattern #5", "agtsic_p6" => "Pattern #6",
        "agtsic_p8" => "Pattern #8", "agtsic_p10" => "Pattern #10", "agtsic_p12" => "Pattern #12", "agtsic_rev6" => "6-day reversal",
        _ => return None,
    };
    let id = REVIEWED.iter().copied().find(|r| *r == id)?;
    Some((id, "Goodwin Inner Circle", f))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    match id {
        "agtsic_p12" => Some(vec![p("text", "Text reading: high above the previous close (0 = printed code, book)", 0., 0., 1., 1.)]),
        "agtsic_rev6" => Some(vec![p("text", "Text reading: stop counts the signal day's close (0 = printed code, book)", 0., 0., 1., 1.)]),
        _ if reviewed(id) || reviewed_breadth(id) => Some(vec![]),
        _ => None,
    }
}

/// Day of week of YYYY-MM-DD, 0 = Sunday (TradeStation numbering).
fn dow(d: &str) -> i64 {
    let (y, m, dd): (i64, i64, i64) = (d[0..4].parse().unwrap_or(0), d[5..7].parse().unwrap_or(1), d[8..10].parse().unwrap_or(1));
    let t = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    let y = if m < 3 { y - 1 } else { y };
    (y + y / 4 - y / 100 + y / 400 + t[(m - 1) as usize] + dd) % 7
}

fn mk(d: f64, ei: usize, ep: f64, kind: &'static str, xi: usize, xp: f64, why: &str, open: bool) -> Trade {
    Trade { dir: d, legs: vec![Leg { i: ei, px: ep, w: 1. }], exit_i: xi, exit_px: xp, reason: why.into(), entry_kind: kind, signal_i: xi, open, exits: vec![] }
}

/// K25: the 8-day RSI system with the A/D filter (Chapter1 A/D Filter, p.22). `ad` = (advances, declines) series names.
fn rsi8_breadth(x: &Ctx, from: usize, to: usize, ad: &[&str]) -> Vec<Trade> {
    let (Some(a), Some(d)) = (x.series(ad[0]), x.series(ad[1])) else { return vec![] };
    let sma10 = |v: &[f64], i: usize| if i >= 9 { v[i - 9..=i].iter().sum::<f64>() / 10. } else { f64::NAN };
    let r = crate::connors::rsi(&x.c, 8);
    let (mut out, mut ent, mut pos_filter) = (vec![], None::<usize>, false);
    for i in 0..=to {
        let q = sma10(a, i) / sma10(d, i); // NaN (a missing day in the window) leaves the filter unchanged
        if q > 0.75 { pos_filter = true; } else if q < 0.5 { pos_filter = false; }
        if i < from.max(1) || r[i].is_nan() || r[i - 1].is_nan() { continue; }
        match ent {
            Some(e) => if r[i] < 65. && r[i - 1] >= 65. { out.push(mk(1., e, x.c[e], "close", i, x.c[i], "RSI(8) crossed below 65", false)); ent = None; }
                else if !pos_filter { out.push(mk(1., e, x.c[e], "close", i, x.c[i], "breadth filter negative", false)); ent = None; },
            None => if i < to && pos_filter && r[i] > 30. && r[i - 1] <= 30. { ent = Some(i); },
        }
    }
    if let Some(e) = ent { out.push(mk(1., e, x.c[e], "close", to, x.c[to], "open", true)); }
    out
}

fn rsi8(x: &Ctx, from: usize, to: usize) -> Vec<Trade> {
    let r = crate::connors::rsi(&x.c, 8);
    let (mut out, mut ent) = (vec![], None::<usize>);
    for i in from.max(1)..=to {
        if r[i].is_nan() || r[i - 1].is_nan() { continue; }
        match ent {
            Some(e) => if r[i] < 65. && r[i - 1] >= 65. { out.push(mk(1., e, x.c[e], "close", i, x.c[i], "RSI(8) crossed below 65", false)); ent = None; },
            None => if i < to && r[i] > 30. && r[i - 1] <= 30. { ent = Some(i); },
        }
    }
    if let Some(e) = ent { out.push(mk(1., e, x.c[e], "close", to, x.c[to], "open", true)); }
    out
}

/// Third Friday (day of month) of the month of YYYY-MM-DD.
fn third_friday(d: &str) -> i64 { let first = dow(&format!("{}-01", &d[..7])); 1 + (5 - first + 7) % 7 + 14 }

/// Expiration bar: the last bar on or before the month's third Friday, in that week (a holiday Friday moves it to Thursday).
fn is_expiration(x: &Ctx, i: usize) -> bool {
    let d = &x.r[i].date;
    let (fri, dd): (i64, i64) = (third_friday(d), d[8..10].parse().unwrap_or(0));
    if dd > fri || dd <= fri - 7 { return false; }
    if dd == fri { return true; }
    match x.r.get(i + 1) { Some(n) => n.date[..7] != d[..7] || n.date[8..10].parse::<i64>().unwrap_or(99) > fri, None => false }
}

fn expiry(x: &Ctx, from: usize, to: usize) -> Vec<Trade> {
    let rg = |i: usize| x.h[i] - x.l[i];
    let mut out = vec![];
    for s in from.max(3)..to {
        let e = s + 5;
        if e >= x.r.len() || !is_expiration(x, e) { continue; }
        let avg = (rg(s - 1) + rg(s - 2) + rg(s - 3)) / 3.;
        if !(rg(s) < 0.7 * avg || rg(s) > 1.4 * avg) { continue; }
        out.push(if e <= to { mk(1., s, x.c[s], "close", e, x.c[e], "expiration day close", false) } else { mk(1., s, x.c[s], "close", to, x.c[to], "open", true) });
    }
    out
}

/// One-day patterns: signal on bar i (`setup`), entry on bar i+1 at a buy stop (`stop` given the next open), exit at that close.
fn pattern(x: &Ctx, from: usize, to: usize, warm: usize, setup: &dyn Fn(usize) -> bool, stop: &dyn Fn(usize, f64) -> Option<f64>, why: &str) -> Vec<Trade> {
    let mut out = vec![];
    for i in from.max(warm)..to {
        if !setup(i) { continue; }
        let k = i + 1;
        let Some(s) = stop(i, x.o[k]) else { continue };
        let px = if x.o[k] >= s { x.o[k] } else if x.h[k] >= s { s } else { continue };
        out.push(mk(1., k, px, "stop", k, x.c[k], why, false));
    }
    out
}

fn rev6(x: &Ctx, from: usize, to: usize, text: bool) -> Vec<Trade> {
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let (mut out, mut cur, mut pend) = (vec![], None::<(f64, usize, f64)>, None::<(f64, f64)>); // (dir, entry bar, price); pending (dir, stop)
    for i in from.max(7)..=to {
        if let Some((pd, s)) = pend.take() {
            let busy = matches!(cur, Some((d, _, _)) if d == pd);
            if !busy {
                let fill = if pd > 0. { if o[i] >= s { Some(o[i]) } else if h[i] >= s { Some(s) } else { None } }
                           else if o[i] <= s { Some(o[i]) } else if l[i] <= s { Some(s) } else { None };
                if let Some(px) = fill {
                    if let Some((d, ei, ep)) = cur { out.push(mk(d, ei, ep, "stop", i, px, "reversed", false)); }
                    cur = Some((pd, i, px));
                }
            }
        }
        if let Some((d, ei, ep)) = cur { if i - ei == 4 { out.push(mk(d, ei, ep, "stop", i, c[i], "close of the fifth day", false)); cur = None; } }
        if i >= to { break; }
        let lo6 = (i - 6..i).map(|j| l[j]).fold(f64::INFINITY, f64::min);
        let hi6 = (i - 6..i).map(|j| h[j]).fold(f64::NEG_INFINITY, f64::max);
        let w = if text { i - 2..i + 1 } else { i - 3..i };
        if c[i] < lo6 { pend = Some((1., w.clone().map(|j| c[j]).fold(f64::NEG_INFINITY, f64::max))); }
        if c[i] > hi6 { pend = Some((-1., w.map(|j| c[j]).fold(f64::INFINITY, f64::min))); }
    }
    if let Some((d, ei, ep)) = cur { out.push(mk(d, ei, ep, "stop", to, c[to], "open", true)); }
    out
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !(reviewed(id) || reviewed_breadth(id)) || from >= to || to >= x.r.len() { return None; }
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let rg = |i: usize| h[i] - l[i];
    let wd = |i: usize| dow(&x.r[i].date);
    Some(match id {
        "agtsic_rsi8" => rsi8(x, from, to),
        "agtsic_rsi8_breadth" => rsi8_breadth(x, from, to, NYSE_AD), // K25
        "agtsic_rsi8_breadth_nse" => rsi8_breadth(x, from, to, NSE_AD), // K25
        "agtsic_expiry" => expiry(x, from, to),
        "agtsic_p5" => pattern(x, from, to, 5, &|i| h[i - 2] < l[i - 5] && h[i] > h[i - 1].max(h[i - 2]) && c[i] < o[i],
            &|i, ok| Some(ok + rg(i).min(rg(i - 1)) * 0.45), "close of the entry day"),
        "agtsic_p6" => pattern(x, from, to, 2, &|i| wd(i) != 2 && o[i] > h[i - 1] && c[i] < c[i - 1] && h[i] - c[i] > c[i] - l[i],
            &|i, ok| Some(ok + rg(i).max(rg(i - 1)).max(rg(i - 2)) * 0.3), "close of the entry day"),
        "agtsic_p8" => pattern(x, from, to, 2, &|i| (wd(i) == 4 || wd(i) == 5) && c[i - 1] > c[i - 2] && c[i] > c[i - 1] && rg(i) < rg(i - 1) * 0.75,
            &|i, ok| (ok > c[i]).then_some(ok), "close of the entry day"),
        "agtsic_p10" => pattern(x, from, to, 2, &|i| l[i] > h[i - 2] && wd(i) == 5 && rg(i) < rg(i - 1) * 0.95,
            &|i, ok| Some(ok + rg(i).min(rg(i - 1)) * 0.6), "close of the entry day"),
        "agtsic_p12" => { let text = g("text") > 0.5;
            pattern(x, from, to, 3, &move |i| (wd(i) == 2 || wd(i) == 5) && rg(i) < (rg(i - 1) + rg(i - 2) + rg(i - 3)) / 3. && if text { h[i] > c[i - 1] } else { h[i] < c[i - 1] },
            &|_, ok| Some(ok), "close of the entry day") }
        "agtsic_rev6" => rev6(x, from, to, g("text") > 0.5),
        _ => return None,
    })
}

/// K25: the breadth-filtered RSI ids, NYSE breadth on US instruments, NSE breadth on Indian ones (the other market's engine
/// has no such series, so readiness refuses it with "Required series ... missing").
pub const ADMISSION_BREADTH: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "agtsic" && crate::books::agtsic::reviewed_breadth(def.id),
    label: "Andrew Goodwin tested this rule on S&P 500 futures (continuous contract, 1982-1997) with NYSE breadth; runs on US indices, ETFs and stocks use the vault's NYSE advances/declines (to 2020-02-10), runs on Indian instruments use NSE advances/declines (the _nse twin); both are a declared application, one unit per trade, no costs.",
    basis: "Andrew Goodwin, Trading Secrets of the Inner Circle (1997), Ch.1 Market Breadth Filtered 8 day RSI (p.11-12) and its printed TradeStation code 'Chapter1 A/D Filter' (p.22); independent reference titles-1030/K25w/agtsic_breadth_ref.py, 0 mismatches US and India.",
};

#[cfg(test)]
mod tests {
    #[test]
    fn breadth_filter_latches_and_exits() {
        use crate::connors::{Ctx, Params};
        // closes 100 -> 81 (RSI near 0), then +4 a day to 93 (RSI 36 on bar 20: the cross above 30), then flat (RSI stays ~66,
        // so the RSI exit never fires); advances 100 to bar 29, then 40, declines 100: the 10-day ratio is .70 on bar 34
        // (between the levels: the filter stays positive) and first falls under .50 on bar 38 (.46) -> exit at that close
        let c: Vec<f64> = (0..40).map(|i| if i < 20 { 100. - i as f64 } else if i < 23 { 81. + 4. * (i - 19) as f64 } else { 93. }).collect();
        let r: Vec<crate::Row> = c.iter().enumerate().map(|(i, &v)| crate::Row { ticker: "T".into(), date: format!("2020-{:02}-{:02}", 1 + i / 28, 1 + i % 28), open: v, high: v, low: v, close: v, volume: None }).collect();
        let mut x = Ctx::new(&r, &r, &[]);
        x.extra.insert("NYSE_ADV_DEC.Advances".into(), (0..40).map(|i| if i < 30 { 100. } else { 40. }).collect());
        x.extra.insert("NYSE_ADV_DEC.Declines".into(), vec![100.; 40]);
        let t = super::run("agtsic_rsi8_breadth", &x, 10, 39, &Params::new()).unwrap();
        assert_eq!(t.len(), 1, "{t:?}");
        assert!(t[0].legs[0].i == 20 && t[0].exit_i == 38 && t[0].reason == "breadth filter negative" && !t[0].open, "{:?}", t[0]);
        // the filter starts negative: with advances at 60 (ratio .60, never above .75) there is no trade
        x.extra.insert("NYSE_ADV_DEC.Advances".into(), vec![60.; 40]);
        assert!(super::run("agtsic_rsi8_breadth", &x, 10, 39, &Params::new()).unwrap().is_empty());
        assert!(super::run("agtsic_rsi8_breadth_nse", &x, 10, 39, &Params::new()).unwrap().is_empty()); // no NSE series here
    }
    #[test]
    fn dow_check() { assert_eq!(super::dow("2026-10-23"), 5); assert_eq!(super::dow("2026-10-19"), 1); assert_eq!(super::dow("1997-03-21"), 5); }
}
