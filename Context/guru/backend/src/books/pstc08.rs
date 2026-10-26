//! Steve Primo / Larry Connors, "The TradingMarkets Swing Trading College" 2008 (12 PowerPoint sessions, The Connors
//! Group 2008). Book id "pstc08". Source copy: Books/Steven Primo/Professional Swing Trading College 2008/Session N.ppt;
//! slide images library/pstc08/pages/sNN-MM.png (session NN, slide MM). Notes, duplicates and the oracle: library/pstc08/.
//! Only the rules this course states that are NOT already in the vault are coded here (R4/R5 2008, RS Meteors
//! portfolio, RSI 25 with the 20 scale-in). Duplicates (stc9, rsi25, pctb, pstc, r3r4) are listed in the README.
#![allow(dead_code)]
use crate::connors::*;

pub const BOOK: &str = "pstc08";
/// Admitted ids (authors/primo/admission/SOURCE-CONTRACT.md).
pub const REVIEWED: &[&str] = &["pstc08_r4", "pstc08_r5", "pstc08_meteors", "pstc08_meteors5", "pstc08_rsi25_20", "pstc08_rsi25_20_noma"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const R4: &[&str] = &[
    "F|\"1. Stock is above its 200-day MA.\" (Session 3, slide 8)",
    "F|\"2. LF is greater than 40.\" (Liquidity Factor = vault Liquidity Filter: 1e6 x SMA20(close x volume) / SMA65(benchmark close x volume))",
    "F|\"3. 100 day HV is greater than 40%.\"",
    "E|\"4. 2-period RSI falls 4 days in a row.\" = RSI(2) lower than the prior day's RSI(2) on each of the last 4 closes",
    "E|\"5. The first falling day (day #1) of the 2-period RSI must be below the 60 benchmark.\" = RSI(2) of the first of the 4 falling days < 60",
    "E|\"Entry: 1. Buy on the next day's open (especially for indices and ETF's) or 2. Limit order X% below previous days close (preferred for individual stocks)\" (slide 9). Default = 1 (the vault runs indices); 2 is p.entry_mode=1 with p.limit_pct [X not given; 3/5/7% are the slide-12..14 chart titles]",
    "A|\"Scale-In: 1. Buy a second unit if the RSI falls again the next day only.\" (slide 10) = RSI(2) lower again on the close of the entry day; [book silent on the fill] bought at that close",
    "X|\"Exit: 1. RSI2 > 75\" (slide 10); [timing not printed] exit at the close on which RSI(2) > 75, the course's own wording for R3 (Session 1 slide 37: \"exit at close when RSI2 closes above 75\")",
];
const R5: &[&str] = &[
    "F|\"1. Stock is above its 200-day MA.\" (Session 3, slide 16)",
    "E|\"2. 2-period RSI falls 5 days in a row.\" = RSI(2) lower than the prior day's RSI(2) on each of the last 5 closes",
    "E|\"3. The first falling day (day #1) of the 2-period RSI must be below the 60 benchmark.\"",
    "E|\"Entry: 1. Buy on the next day's open (especially for indices and ETF's) or 2. Limit order X% below previous days close\" (slide 17). Default = 1; 2 = p.entry_mode=1 with p.limit_pct [X not given]",
    "A|\"Scale-In: 1. Day after buy RSI2 falls again.\" (slide 18) = default: RSI(2) lower again on the entry-day close (same as the R4 wording \"the next day only\"); p.add_lag=2 reads 'day after the fill day'. [book silent on the fill] bought at that close",
    "X|\"Exit: 1. RSI2 > 75\" (slide 18); exit at the close on which RSI(2) > 75 (course wording for R3, Session 1 slide 37)",
];
const MET10: &[&str] = &[
    "E|\"1. Each of the last two days have been down 10% or more.\" (Session 4, slide 7) = close <= prior close x 0.90 on each of the last 2 days",
    "F|\"2. C > 200MA\"",
    "F|\"3. LF > 3\"",
    "E|\"4. Enter with 10% of portfolio at the next day's open.\" (single instrument: the full allocation)",
    "X|\"Exit: 1. Up one day. 2. Exit next open.\" (slide 8) = first close above the prior close (the entry day counts), exit at the next open",
];
const MET5: &[&str] = &[
    "E|\"1. Each of the last two days have been down 5% or more.\" (Session 4, slide 12)",
    "F|\"2. C > 200MA\"",
    "F|\"3. LF > 3\"",
    "E|\"4. Enter with 10% of portfolio at the next days open.\"",
    "X|\"Exit: 1. Up one day. 2. Exit next open.\" (slide 13)",
];
const RSI25_MA: &[&str] = &[
    "E|\"1. The 4-Day RSI of the market closes below 25.\" (Session 7, slide 16)",
    "F|\"2. The ETF closes above its 200-day MA.\"",
    "E|\"3. Buy at the close.\"",
    "X|\"4. Exit at the close when the 4-Day RSI closes above 55.\"",
    "A|\"5. Scale-In Version-Buy a second unit if the RSI goes under 20 after entering the first position.\" = first later close with RSI(4) < 20, bought at that close; one add",
];
const RSI25_NOMA: &[&str] = &[
    "E|\"1. The 4-Day RSI of the market closes below 25.\" (Session 7, slide 21)",
    "F|\"2. No 200-day MA rule.\"",
    "E|\"3. Buy at the Close.\"",
    "X|\"4. Exit at the Close when the 4-Day RSI of the market closes above 55.\"",
    "A|\"5. Scale-In Version-Buy a second unit if the RSI goes under 20 after entering the first position.\"",
];

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    let d = |id, name, group, dir, status, rules, notes, claim| StrategyDef {
        id, name, group, session: 0, dir, status, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim,
    };
    vec![
        StrategyDef { needs_volume: true, ..d("pstc08_r4", "R4 Rules (2008: LF > 40, HV > 40, RSI(2) falls 4 days)", "Session 3 · R4", "Long", Live, R4,
          "Differs from pstc_r4_long (2005: RSI(2) < 10 today, buy at the close, add on 5 falls) and r3r4 (R3/R4 Momentum: 3 of 4 drops, ADX, 5% limit). HV(100) as stc9 (sample sd x sqrt(252) x 100). No published statistics (\"Let's go to the spreadsheets\"). Course instrument: stocks, indices and ETFs (entry 1 is 'especially for indices'); India: instrument swap only. On an index the LF compares the index's dollar volume with the NIFTY 50 / benchmark itself, so LF > 40 always passes there.", None) },
        d("pstc08_r5", "R5 Rules (2008: RSI(2) falls 5 days, first day < 60)", "Session 3 · R5", "Long", Live, R5,
          "The 2005 course (pstc_live_r5) only named R5; the 2008 Session 3 states it. Slide 20: \"R4s for ETFs and indices, R5s for stocks\". No published statistics. India: instrument swap only.", None),
        StrategyDef { needs_volume: true, ..d("pstc08_meteors", "RS Meteors Portfolio (two days down 10%+)", "Session 4 · RS Meteors", "Long", Live, MET10,
          "Stock method (\"Not enough signals\", slide 10): two consecutive 10% down closes are essentially absent on indices, so run it on NSE basket stocks (basket:<list>:<SYMBOL>). Portfolio sizing (10% per position, many stocks) is not modelled; one instrument per run. Session 4 slide 9: \"90 of the past 100 months have been profitable\" (portfolio, period not printed).",
          claim("-", "-", "-", "-", "Portfolio since 1995: \"90 of the past 100 months have been profitable\" (Session 4 slide 9)")) },
        StrategyDef { needs_volume: true, ..d("pstc08_meteors5", "RS Meteors Portfolio 5% Pullback (two days down 5%+)", "Session 4 · RS Meteors", "Long", Live, MET5,
          "As pstc08_meteors with 5% days. Stock method; run on NSE basket stocks. No published statistics.", None) },
        d("pstc08_rsi25_20", "RSI 25 with 200-day MA · scale-in under 20", "Session 7 · RSI 25", "Long", Live, RSI25_MA,
          "The base rule (no scale-in) is a duplicate of rsi25 r25_rsi25_official_sma200; the 2008 course's scale-in level 20 differs from the RSI 25 book's 15 (r25_rsi25_15). Results 'SPX 1995 thru 2007' were in spreadsheets, not printed. India: instrument swap only.", None),
        d("pstc08_rsi25_20_noma", "RSI 25 without 200-day MA · scale-in under 20", "Session 7 · RSI 25", "Long", Live, RSI25_NOMA,
          "Base rule = rsi25 r25_rsi25_basic_no_filter (duplicate); only the 20-level scale-in is new. India: instrument swap only.", None),
        d("pstc08_meteors_menu", "RS Meteors (exit 'your choice')", "Session 2 · RS Meteors", "Long",
          NotTestable("the exit is a menu (\"Exit (your choice): 1. RSI(2) > 70. 2. Close > 5-day moving average. 3. Up one day.\") with no default and no timing; the Session 4 portfolio version with a timed exit is pstc08_meteors"),
          &["E|\"1. Each of the last two days have been down 10% or more. 2. Close > 200-day moving average. 3. Liquidity factor > 3. 4. Enter at the next open.\" (Session 2 slide 12)",
            "X|\"Exit (your choice) 1. RSI(2) > 70. 2. Close > 5-day moving average. 3. Up one day.\" (slide 13)"], "", None),
        d("pstc08_target", "Target Trading (liquid low-volatility stocks)", "Session 2 · Target Trading", "Long",
          NotTestable("the exit threshold X is never given (\"If close is X% above or below entry price then exit next open\")"),
          &["F|\"1. LF > 20 2. Close > 200MA 3. RSI(2) < 5 4. HV100 < 20 and HV100 > 15 5. Close > 10\" (Session 2 slide 17)", "E|\"6. Buy at next open.\"",
            "X|\"1. If close is X% above or below entry price then exit next open.\" (slide 18)"], "", None),
        d("pstc08_espy", "E-SPY (SPY core + R5 5% limit stock sleeve)", "Session 4 · E-SPY", "Long",
          NeedsData("a US stock universe on one portfolio equity curve (10 R5 positions ranked by HV next to a 90% SPY holding); the engine runs one instrument at a time"),
          &["F|Long 100% SPY, replace 10% with R5 + 5% limit (Session 4 slides 17-23)", "E|\"R5 with 5% limit: 1 LF>3 (or LF>10 for bigger caps) 2 HV100>30 3 enter when limit price = 5% below close of setup\"",
            "X|\"1 RSI(2)>75 2 exit on open of following bar\"; highest HV first, max 10 positions"], "", None),
        d("pstc08_options", "ETF options, STC Options Income Program, Busted Options", "Session 8 · Options", "Long/Short",
          NotTestable("needs option chains (deep ITM calls, bull put spreads, option RSI(2), earnings)"), &[], "", None),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "pstc08_r4" => ("pstc08_r45", "STC 2008 · R4 / R5", "R4"),
        "pstc08_r5" => ("pstc08_r45", "STC 2008 · R4 / R5", "R5"),
        "pstc08_meteors" => ("pstc08_meteors", "STC 2008 · RS Meteors", "10% days"),
        "pstc08_meteors5" => ("pstc08_meteors", "STC 2008 · RS Meteors", "5% pullback"),
        "pstc08_meteors_menu" => ("pstc08_meteors", "STC 2008 · RS Meteors", "Exit menu (not testable)"),
        "pstc08_rsi25_20" => ("pstc08_rsi25", "STC 2008 · RSI 25 scale-in 20", "With 200-day MA"),
        "pstc08_rsi25_20_noma" => ("pstc08_rsi25", "STC 2008 · RSI 25 scale-in 20", "Without 200-day MA"),
        "pstc08_target" | "pstc08_espy" | "pstc08_options" => ("pstc08_other", "STC 2008 · other", "Not testable / needs data"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let rx = |days: f64| {
        let mut v = vec![
            p("trend", "200-day MA filter length", 200., 50., 300., 50.),
            p("days", "RSI(2) falls N days in a row", days, 2., 7., 1.),
            p("first_lt", "RSI(2) on the first falling day below", 60., 30., 90., 5.),
            p("entry_mode", "0 = buy next open (book entry 1); 1 = limit X% below the setup close (book entry 2, X not given) [labelled option]", 0., 0., 1., 1.),
            p("limit_pct", "X for entry 2 (not in the book; slide charts show 3/5/7%)", 5., 1., 10., 1.),
            p("scale_in", "1 = scale-in rule on (book); 0 = off", 1., 0., 1., 1.),
            p("exit_gt", "Exit at the close when RSI(2) above", 75., 50., 95., 5.),
        ];
        if days == 4. {
            v.insert(1, p("lf", "Liquidity Factor above", 40., 0., 100., 5.));
            v.insert(2, p("hv", "HV(100) above (%)", 40., 0., 80., 5.));
        } else {
            v.push(p("add_lag", "Scale-in day: 1 = the entry day's close (default); 2 = the day after the fill day [labelled option]", 1., 1., 2., 1.));
        }
        v
    };
    Some(match id {
        "pstc08_r4" => rx(4.),
        "pstc08_r5" => rx(5.),
        "pstc08_meteors" | "pstc08_meteors5" => vec![
            p("down_pct", "Each of the last two days down at least %", if id == "pstc08_meteors" { 10. } else { 5. }, 1., 20., 1.),
            p("trend", "200-day MA filter length", 200., 50., 300., 50.),
            p("lf", "Liquidity Factor above", 3., 0., 40., 1.),
        ],
        "pstc08_rsi25_20" | "pstc08_rsi25_20_noma" => {
            let mut v = vec![
                p("entry_lt", "Buy when RSI(4) closes below", 25., 5., 40., 5.),
                p("add_lt", "Scale-in when RSI(4) goes under (0 = off)", 20., 0., 30., 5.),
                p("exit_gt", "Exit when RSI(4) closes above", 55., 40., 80., 5.),
            ];
            if id == "pstc08_rsi25_20" { v.insert(0, p("trend", "200-day MA filter length", 200., 50., 300., 50.)); }
            v
        }
        _ => return None,
    })
}

/// One position at a time. Entry on the setup close (`next = false`) or on the next bar (`entry_px(s, e)` returns
/// the fill or None = not filled, order cancelled). One added unit at a close. Exit at the close of the exit
/// signal (`exit_next = false`) or at the next open.
fn sim(x: &Ctx, from: usize, to: usize, next: bool, setup: &dyn Fn(usize) -> bool, entry_px: &dyn Fn(usize, usize) -> Option<f64>,
       kind: &'static str, add: Option<&dyn Fn(usize, usize, usize) -> bool>, exit: &dyn Fn(usize, usize) -> bool, exit_next: bool, why: &str) -> Vec<Trade> {
    let to = to.min(x.c.len() - 1);
    let w = if add.is_some() { 0.5 } else { 1. };
    let (mut out, mut cur): (Vec<Trade>, Option<(Trade, usize)>) = (vec![], None);
    let (mut pend_entry, mut pend_exit): (Option<usize>, bool) = (None, false);
    for i in from..=to {
        if pend_exit {
            let (mut t, _) = cur.take().unwrap();
            (t.exit_i, t.exit_px, t.signal_i, t.reason) = (i, x.o[i], i - 1, why.to_string());
            out.push(t);
            pend_exit = false;
        }
        if let Some(s) = pend_entry.take() {
            if let Some(px) = entry_px(s, i) {
                cur = Some((Trade { dir: 1., legs: vec![Leg { i, px, w }], exit_i: i, exit_px: f64::NAN, reason: String::new(),
                    entry_kind: kind, signal_i: i, open: false, exits: vec![] }, s));
            }
        }
        if let Some((t, s)) = cur.as_mut() {
            let e = t.legs[0].i;
            if exit(i, e) {
                if exit_next {
                    if i < to { pend_exit = true; }
                } else {
                    let (mut t, _) = cur.take().unwrap();
                    (t.exit_i, t.exit_px, t.signal_i, t.reason) = (i, x.c[i], i, why.to_string());
                    out.push(t);
                }
                continue;
            }
            if let Some(a) = add {
                if t.legs.len() == 1 && a(i, *s, e) { t.legs.push(Leg { i, px: x.c[i], w }); }
            }
            continue;
        }
        if setup(i) {
            if next {
                if i < to { pend_entry = Some(i); }
            } else {
                cur = Some((Trade { dir: 1., legs: vec![Leg { i, px: x.c[i], w }], exit_i: i, exit_px: f64::NAN, reason: String::new(),
                    entry_kind: "close", signal_i: i, open: false, exits: vec![] }, i));
            }
        }
    }
    if let Some((mut t, _)) = cur {
        (t.exit_i, t.exit_px, t.reason, t.signal_i, t.open) = (to, x.c[to], "open".into(), to, true);
        out.push(t);
    }
    out
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let (c, o, l) = (&x.c, &x.o, &x.l);
    let ok = |v: f64| v.is_finite();
    Some(match id {
        "pstc08_r4" | "pstc08_r5" => {
            let r4 = id == "pstc08_r4";
            let ma = x.sma_n(g("trend") as usize);
            let r = &x.rsi2;
            let k = g("days") as usize;
            let (first, ex) = (g("first_lt"), g("exit_gt"));
            let (lf, hv) = if r4 { (g("lf"), g("hv")) } else { (f64::NAN, f64::NAN) };
            let falls = |i: usize| i >= k && (i + 1 - k..=i).all(|j| ok(r[j]) && ok(r[j - 1]) && r[j] < r[j - 1]);
            let setup = |i: usize| ok(ma[i]) && c[i] > ma[i] && falls(i) && r[i + 1 - k] < first
                && (!r4 || (x.lf[i] > lf && x.hv100[i] > hv));
            let limit = g("entry_mode") >= 0.5;
            let lp = g("limit_pct") / 100.;
            let entry_px = |s: usize, e: usize| if limit {
                let lv = c[s] * (1. - lp);
                (l[e] <= lv).then(|| o[e].min(lv))
            } else { Some(o[e]) };
            let lag = if r4 { 1 } else { g("add_lag") as usize };
            // "falls again": RSI(2) lower than the previous close's RSI(2), on the bar `lag` days after the setup / fill
            let add = |i: usize, s: usize, e: usize| { let day = if lag == 1 { s + 1 } else { e + 1 }; i == day && r[i] < r[i - 1] };
            let exit = |i: usize, _e: usize| r[i] > ex;
            sim(x, from, to, true, &setup, &entry_px, if limit { "limit" } else { "open" }, if g("scale_in") >= 0.5 { Some(&add) } else { None }, &exit, false, "RSI(2) > exit level")
        }
        "pstc08_meteors" | "pstc08_meteors5" => {
            let ma = x.sma_n(g("trend") as usize);
            let (dn, lf) = (1. - g("down_pct") / 100., g("lf"));
            let setup = |i: usize| i >= 2 && ok(ma[i]) && c[i] <= c[i - 1] * dn && c[i - 1] <= c[i - 2] * dn && c[i] > ma[i] && x.lf[i] > lf;
            let exit = |i: usize, _e: usize| c[i] > c[i - 1];
            sim(x, from, to, true, &setup, &|_s, e| Some(o[e]), "open", None, &exit, true, "up day, out next open")
        }
        "pstc08_rsi25_20" | "pstc08_rsi25_20_noma" => {
            let ma = if id == "pstc08_rsi25_20" { Some(x.sma_n(g("trend") as usize)) } else { None };
            let r = &x.rsi4;
            let (lt, alt, ex) = (g("entry_lt"), g("add_lt"), g("exit_gt"));
            let setup = |i: usize| ok(r[i]) && r[i] < lt && ma.as_ref().map_or(true, |m| ok(m[i]) && c[i] > m[i]);
            let add = |i: usize, _s: usize, e: usize| i > e && r[i] < alt;
            let exit = |i: usize, _e: usize| r[i] > ex;
            sim(x, from, to, false, &setup, &|_s, _e| None, "close", if alt > 0. { Some(&add) } else { None }, &exit, false, "RSI(4) > exit level")
        }
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn rows(cl: &[f64]) -> Vec<Row> {
        cl.iter().enumerate().map(|(i, &c)| Row { ticker: "T".into(), date: format!("d{:04}", i), open: c, high: c * 1.01, low: c * 0.99, close: c, volume: Some(1e6) }).collect()
    }
    #[test]
    fn r5_five_falls_next_open_add_exit() {
        // rising 230 bars, 1 small dip (day #1 < 60), 4 more drops (5 falls) -> setup on bar 234, buy open 235;
        // bar 235 falls again -> add at its close; then a rally -> exit at the close RSI(2) > 75.
        let mut cl: Vec<f64> = (0..230).map(|i| 100. + i as f64 * 0.5).collect();
        let mut v = *cl.last().unwrap() - 1.5;
        cl.push(v);
        for _ in 0..4 { v -= 1.; cl.push(v); }
        v -= 1.; cl.push(v);
        for _ in 0..3 { v += 4.; cl.push(v); }
        let r = rows(&cl);
        let x = Ctx::new(&r, &r, &[]);
        assert!(x.rsi2[230] < 60.);
        let t = run("pstc08_r5", &x, 0, r.len() - 1, &Params::new()).unwrap();
        let t = t.iter().find(|t| t.legs[0].i == 235).expect("entry at the open after 5 falls");
        assert_eq!(t.entry_kind, "open");
        assert_eq!(t.legs.len(), 2);
        assert_eq!(t.legs[1].i, 235);
        assert!(x.rsi2[t.exit_i] > 75. && t.exit_px == cl[t.exit_i]);
    }
}
