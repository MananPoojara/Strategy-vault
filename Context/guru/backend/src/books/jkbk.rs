//! Joe Krutsinger product pack (etrackrecords.com / joekrut.com): "joe_krut.pdf" (45 pp: 2008 TRADERS' magazine
//! interview with TS 8.3 code, system operation sheets, the SAFE worksheet, the Sept 2000 Active Trader article
//! "Trading the Equity Curve"), the Boredom Killer Readme.txt and "Joe K's notes for DCP.doc" (Dealer's Choice Pro).
//! Book id `jkbk`. Source copies: Books/Joe Krutsinger/{Boredom Killer,First Hour Breakout,Dealers Choice Pro}/.
//! Page refs = PDF page of joe_krut.pdf. Library: library/jkbk/. Simulator conventions: see `jkbwts`.
use super::jkbwts::{avg, buy_monday, dow, entry, exit, sim, Ty};
use super::rrsim::{o as tso, run_ts, A, P};
use crate::connors::{claim, rsi, Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "jkbk";
/// Admitted ids (authors/krutsinger/admission/SOURCE-CONTRACT.md).
pub const REVIEWED: &[&str] = &["jkbk_safe", "jkbk_call_buyer", "jkbk_buy_monday_ts83" /* intraday wave 2b: daily bars decide every trade */,
    "jkbk_focal_point", "jkbk_starting_point" /* lane K47 (decision 217): vendor-sheet code read by OCR */];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }
const SOLD: &str = "rules not disclosed: the source gives only the optimisation check-list (input names and ranges) of a commercial system; the code is sold separately or only in a compiled .ELD";

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    let d = |id, name, group, dir, status, rules, notes, claim| StrategyDef { id, name, group, session: 0, dir, status,
        needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim };
    let nt = |id, name, rules, notes| d(id, name, "Commercial systems (rules undisclosed)", "Long/Short", NotTestable(SOLD), rules, notes, None);
    vec![
        d("jkbk_buy_monday_ts83", "Buy Monday (TradeStation 8.3 code)", "Disclosed systems", "Long", Live,
          &["E|If DayOfWeek(Date) = 5 then Buy Next Bar at Market (p.4 code)",
            "X|SetExitOnClose", "X|SetStopLoss(Amount 1000), PositionBasis true: $1,000 per position = 4 S&P points at $250/point (big contract after 1997; 'you lose $1000 on a big contract or $200 on the mini', p.4)"],
          "p.3-4 (2008 interview, system written 1989, published 1992). Book: @SP.P daily 04/21/82-04/15/08: 1,310 trades, 38.63% profitable (506 win / 799 loss / 5 even), net $323,375, avg $246.85. pt_usd 250 is the 2008 big-contract value (it was $500 before Nov 1997) - a single point value cannot match the whole futures history.",
          claim("1310", "38.63%", "$246.85", "same day", "@SP.P 4/1982-4/2008 (p.4)")),
        d("jkbk_safe", "SAFE daily worksheet", "Disclosed systems", "Long/Short", Live,
          &["F|1-4: today's close vs the close of the day before yesterday (C[2]): larger -> SELL, smaller -> BUY (p.22)",
            "E|BUY: tomorrow's open + 40% of today's range (High - Low) as a buy stop",
            "E|SELL: tomorrow's open - 60% of today's range as a sell stop",
            "X|'Once filled, exit on next days OPEN; REPEAT'"],
          "p.22 ('Daily WorkSheet For SAFE'). Market not named on the sheet (the Safe Ez / Safer 2005 systems were sold for stock indexes). Equal closes: neither BUY nor SELL is circled, no order. No published results.", None),
        d("jkbk_call_buyer", "JK-Call Buyer", "Disclosed systems", "Long", Live,
          &["E|When you have no current position: RSI(C,5) > RSI(C,5)[1] and C < C[5] and C <= Average(C,5) -> Buy ('Buy Calls') tomorrow at Market (p.34-35)",
            "X|If MarketPosition = 1 and BarsSinceEntry >= 10 then ExitLong at Market ('exits all trades after 10 days')",
            "X|If MarketPosition = 1 and C > Average(C,5) then ExitLong at Market"],
          "Active Trader Sept 2000, 'Trading the Equity Curve' (p.34-35). Book: S&P 500 futures April 1999-June 2000: 24 trades, 70.83% profitable, net $75,550, avg $3,147.92, PF 4.65. RSI = Wilder RSI (TradeStation RSI).",
          claim("24", "70.83%", "$3,147.92", "", "S&P 500 futures 4/1999-6/2000 (p.35)")),
        d("jkbk_jkm_filter", "JKM / JKD equity-curve filter", "Filters", "Long/Short", NotTestable("incomplete definition: 'JKM = 30-day average of the strategy's equity curve', trade only while it rises; which equity (closed or open), how the paper-traded curve is built and when trading resumes are not specified"),
          &["F|JKM indicator = 30-day moving average of your strategy's equity curve; JKD = day-to-day difference in JKM; when JKD is negative do not trade the strategy in real time (p.34-36)"],
          "Applied to JK-Call Buyer in the article (Figure 2), results given only as equity-trough anecdotes.", None),
        nt("jkbk_boredom_killer", "Boredom Killer", &["F|5-minute bars of mini Russell 2000 (TF), session 9:30-3:59 ET; inputs StopLossAmt 140, PctTrailingFloorAmt 140, PctTrailingPct 85, ExitOnClose true; profit target $650; Timer Exit 10:25 (Readme.txt)"], "Intraday and the entry logic is only in BOREDOM_KILLER.ELD (compiled)."),
        nt("jkbk_first_hour_breakout", "First Hour Breakout (FHBO)", &["F|60-minute bars of big-cap stocks; strategies First Hour Breakout, Percent Exit Pair, WalkAway; Profit Target Percent 0-0.15, Stop Loss Percent 0-0.08, WalkAway 1-50 (p.19)"], "Intraday; logic only in FIRST HOUR.ELD (compiled)."),
        nt("jkbk_timeany", "Time Any", &["F|60-minute bars, inputs Length 2-12, Tim1/Tim2 830-1530 (p.19); S&P 400 E-mini 60 min 2002-2008: 1,386 trades, 59.96%, avg $49.42 (p.10)"], "Intraday."),
        d("jkbk_english_channel", "English Channel (book text; code version: upg18_english_channel)", "Commercial systems (rules undisclosed)", "Long/Short", NotTestable("duplicate of upg18_english_channel (decision #244): the product pack gives only this system's optimisation check-list (p.22); the same English Channel system runs in the vault from its vendor code (EnglishChannel.pdf, Length 8) as upg18_english_channel"), &["F|60-minute or daily bars, input Length 2-9, trd 1 (p.22)"], "", None),
        d("jkbk_focal_point", "Focal Point (vendor sheet code)", "Disclosed systems", "Long/Short", Live,
          &["F|\"Input: B1(1),S1(5),B2(0),S2(4),Walk(35); Condition1=B1<>S1 and B1<>S2; Condition2=B2<>S1 and B2<>S2; Condition3=S1<>B1 and S1<>S2; Condition4=S2<>B1 and S2<>B2;\" (FocalPoint.pdf; all four true at the defaults)",
            "E|\"IF (DayOfWeek(date)=B1 and C >= C[2]) and MarketPosition<>1 and TradesToday(date)<1 and Condition1..4 Then Buy(\\\"B1\\\") tomorrow at market;\" and \"IF (DayOfWeek(date)=B2 and C < C[2]) ... Buy(\\\"B2\\\") tomorrow at market;\"",
            "E|\"IF (DayOfWeek(date)=S1 and C < C[2]) and MarketPosition<>-1 and TradesToday(date)<1 ... Sell Short(\\\"S1\\\") tomorrow at market;\" and \"IF (DayOfWeek(date)=S2 and C >= C[2]) ... Sell Short(\\\"S2\\\") tomorrow at market;\"",
            "X|\"if (BarsSinceEntry>=Walk) and MarketPosition=1 Then Sell(\\\"LXW\\\") Next Bar at market;\" (short mirror \"SXW\"); an opposite entry reverses",
            "F|[Readings] TradeStation DayOfWeek: 0 = Sunday .. 5 = Friday, so B1(1) = Monday, S1(5) = Friday, S2(4) = Thursday and B2(0) fires only on a Sunday bar; TradesToday(date)<1 = no entry filled on the bar just closed (as upg18_english_channel); the checklist's 'Today is Monday (S1=1)' contradicts S1(5): the code wins"],
          "Source: research USB HDD 1/TradeStation Systems & Indicators/Joe Krutsinger - Focal Point Trading System (Treasury Bonds 30) (joekrut.com)/From other source/FocalPoint.pdf p.1 (one-page vendor sheet, scan; RapidOCR 4 Nov 2026, authors/batch-x/keep/h2-krut/focalpoint.ocr.txt). Sold for 30-year T-bonds, '60-minute or daily bars' (joe_krut.pdf p.22): run on daily bars of indices, ETFs and stocks (declared application). The code has no stop; the 'Focal Point 2002 Optimisation.doc' only offers a $500-$1000 stop range, not applied. Market orders fill at the next open. No published results. Independent check authors/titles-1030/K47w/k47_ref.py.", None),
        d("jkbk_gearshift", "GearShift (indexes / forex) (code version: upg18_gearshift)", "Commercial systems (rules undisclosed)", "Long/Short", NotTestable("duplicate of upg18_gearshift (decision #244): the product pack gives only the check-list (p.25); the same GearShift system runs in the vault from its open code (GearShift.pdf p.1-2) as upg18_gearshift"), &["F|60-minute bars, inputs LenRate, LenAver, LenBol1 2-14, LenBol2 3, LenBol3 2 (p.25)"], "", None),
        d("jkbk_parrot_bbe", "Parrot BBE (code version: upg18_parrot_bbe)", "Commercial systems (rules undisclosed)", "Long/Short", NotTestable("duplicate of upg18_parrot_bbe (decision #244): the product pack gives only the check-list (p.25); the code version upg18_parrot_bbe (GearShift.pdf p.6-7) is itself held, because its break-even exit is a dollar profit ('Move to BE 500') of an unstated contract"), &["F|60-minute or daily bars, AfStep 0.0005-0.03, Move to BE 500, PT 24 (p.25)"], "", None),
        d("jkbk_starting_point", "Starting Point (vendor sheet code)", "Disclosed systems", "Long/Short", Live,
          &["F|\"Input: VarA(2),VarB(7),VarC(6);\" (StartingPoint.pdf p.1)",
            "E|\"IF C <= C[2] Then Buy Next Bar C + (Range * 0.10*VarA) Stop;\" (checklist: today's close plus today's RANGE times 20%)",
            "E|\"IF C > C[2] Then Sell Short Next Bar C - (Range * 0.10*VarB) Stop;\" (checklist: minus today's RANGE times 70%)",
            "X|\"IF MarketPosition=1 and BarsSinceEntry >=VarC Then Sell Next Bar at Market;\" (short mirror): 'Exit all trades on the open after being in the trade 6 days'",
            "X|HELD: the checklist's 'Exit all trades with a $200 loss' is a dollar amount per contract of an unstated contract and is not in the code; it is not converted (as upg18_gearshift's $500 stop, decisions 19, 138, 217). An opposite stop reverses the position (no position test on the entries, as upg18_trend_walker)"],
          "Source: research USB HDD 1/TradeStation Systems & Indicators/Joe Krutsinger - Starting Point System for TS (joekrut.com)/StartingPoint.pdf p.1 (vendor sheet: code + checklists, scan; RapidOCR 4 Nov 2026, authors/batch-x/keep/h2-krut/startingpoint.ocr.txt). p.2 prints a different 60-minute swing code, not this card. Daily bars ('exit ... after being in the trade 6 days'); same family as upg18_trend_walker without its 10-day average filter. Stops fill at their level, or at the open when the open is already through; one bar runs open, nearer extreme, other extreme. No published results. Independent check authors/titles-1030/K47w/k47_ref.py.", None),
        nt("jkbk_time_charger", "Time Charger", &["F|60-minute bars, Length 2-9, Tim1/Tim2 830-1630 (p.28)"], "Intraday."),
        d("jkbk_dealers_choice_pro", "Dealer's Choice Pro (morning breakout) (code version: upg18_dealers_choice_pro)", "Commercial systems (rules undisclosed)", "Long/Short", NotTestable("duplicate of upg18_dealers_choice_pro (decision #244): the notes (Joe K's notes for DCP.doc) give only the inputs; the code version upg18_dealers_choice_pro is itself held (7-minute bars and dollar stops of an unstated contract)"), &["F|7-minute bars; morning breakout based on the close TWO days ago; inputs Trad(4) trades, BO(1) = 10% breakout, Tim(1200) last entry time, Amount $2,300 trailing stop; no entries in the last hour; last trade is the 'trend trade' (Joe K's notes for DCP.doc)"], "Intraday; the breakout formula itself is not given (DlrChoicePro.ela undecodable, Open Code.rar not opened).", None),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    if id == "jkbk_buy_monday_ts83" { return Some(("jk_buy_monday", "Buy Monday (Krutsinger)", "TS 8.3 code 2008, $1,000 stop")); }
    catalogue().into_iter().find(|d| d.id == id).map(|d| (d.id, d.name, ""))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(match id {
        "jkbk_buy_monday_ts83" => vec![p("stop_usd", "SetStopLoss amount $ (0 = none)", 1000., 0., 5000., 50.), p("pt_usd", "$ per index point (big S&P contract, 2008)", 250., 1., 1000., 50.)],
        "jkbk_safe" => vec![p("buy_pct", "Buy stop: % of range above the open", 40., 0., 200., 5.), p("sell_pct", "Sell stop: % of range below the open", 60., 0., 200., 5.), p("lag", "Compare with the close N days ago", 2., 1., 10., 1.)],
        "jkbk_call_buyer" => vec![p("rsi", "RSI length", 5., 2., 20., 1.), p("roc", "Close below the close N days ago", 5., 1., 20., 1.), p("ma", "Average length", 5., 2., 20., 1.), p("hold", "Time exit: BarsSinceEntry >=", 10., 1., 30., 1.)],
        "jkbk_focal_point" => vec![p("b1", "B1: buy day (TS DayOfWeek, 1 = Monday), close >= close 2 bars ago", 1., 0., 6., 1.), p("s1", "S1: short day, close < close 2 bars ago", 5., 0., 6., 1.),
            p("b2", "B2: buy day, close < close 2 bars ago (0 = Sunday)", 0., 0., 6., 1.), p("s2", "S2: short day, close >= close 2 bars ago", 4., 0., 6., 1.), p("walk", "Walk: exit when BarsSinceEntry >=", 35., 1., 60., 1.)],
        "jkbk_starting_point" => vec![p("vara", "VarA: buy stop = close + VarA x 10% of the range", 2., 0., 20., 1.), p("varb", "VarB: short stop = close - VarB x 10% of the range", 7., 0., 20., 1.), p("varc", "VarC: exit when BarsSinceEntry >=", 6., 1., 20., 1.)],
        _ => return None,
    })
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if from > to || to >= x.c.len() { return None; }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let u = |k: &str| g(k).max(1.0).round() as usize;
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    Some(match id {
        "jkbk_buy_monday_ts83" => buy_monday(x, from, to, if g("stop_usd") > 0.0 { g("stop_usd") / g("pt_usd") } else { 0.0 }),
        "jkbk_safe" => {
            let (bp, sp, lag) = (g("buy_pct") / 100.0, g("sell_pct") / 100.0, u("lag"));
            sim(x, from, to, 0.0, false, |i, pos, _| {
                let mut v = vec![];
                if let Some(q) = pos { v.push(exit(q.dir, Ty::Mkt, 0.0, "Next day's open")); }
                if i >= lag {
                    let r = h[i] - l[i];
                    if c[i] > c[i - lag] { v.push(entry(-1, Ty::Stop, o[i + 1] - sp * r, "SAFE sell stop")); }
                    else if c[i] < c[i - lag] { v.push(entry(1, Ty::Stop, o[i + 1] + bp * r, "SAFE buy stop")); }
                }
                v
            })
        }
        "jkbk_call_buyer" => {
            let (r, n, ma, hold) = (rsi(c, u("rsi")), u("roc"), avg(c, u("ma")), u("hold"));
            sim(x, from, to, 0.0, false, |i, pos, _| {
                let mut v = vec![];
                match pos {
                    Some(q) => {
                        if i - q.i >= hold { v.push(exit(1, Ty::Mkt, 0.0, "Exit Calls Time")); }
                        if c[i] > ma[i] { v.push(exit(1, Ty::Mkt, 0.0, "Exit Calls Price")); }
                    }
                    None => if i >= n && i >= 1 && r[i] > r[i - 1] && c[i] < c[i - n] && c[i] <= ma[i] { v.push(entry(1, Ty::Mkt, 0.0, "Buy Calls")); }
                }
                v
            })
        }
        "jkbk_focal_point" => {
            let day = |k: &str| g(k).round() as i64;
            let (b1, s1, b2, s2, walk) = (day("b1"), day("s1"), day("b2"), day("s2"), u("walk"));
            let cond = b1 != s1 && b1 != s2 && b2 != s1 && b2 != s2 && s1 != b1 && s1 != s2 && s2 != b1 && s2 != b2;
            run_ts(x, from, to, |i, st, _now, next| {
                if i < 2 || !cond { return; }
                let (wd, tt) = (dow(&x.r[i].date), st.pos != 0 && st.entry_i == i); // TradesToday: an entry filled on this bar
                let (bb1, bb2, ss1, ss2) = (wd == b1 && c[i] >= c[i - 2], wd == b2 && c[i] < c[i - 2], wd == s1 && c[i] < c[i - 2], wd == s2 && c[i] >= c[i - 2]);
                if !tt && st.pos != 1 && (bb1 || bb2) { next.push(tso(A::Buy, P::Mkt, if bb1 { "B1" } else { "B2" })); }
                if !tt && st.pos != -1 && (ss1 || ss2) { next.push(tso(A::Sell, P::Mkt, if ss1 { "S1" } else { "S2" })); }
                if st.pos == 1 && st.bse(i) >= walk { next.push(tso(A::Xl, P::Mkt, "LXW")); }
                if st.pos == -1 && st.bse(i) >= walk { next.push(tso(A::Xs, P::Mkt, "SXW")); }
            })
        }
        "jkbk_starting_point" => {
            let (va, vb, vc) = (g("vara"), g("varb"), u("varc"));
            run_ts(x, from, to, |i, st, _now, next| {
                if i < 2 { return; }
                let r = h[i] - l[i];
                if c[i] <= c[i - 2] { next.push(tso(A::Buy, P::Stop(c[i] + r * 0.10 * va), "LE")); }
                if st.pos == 1 && st.bse(i) >= vc { next.push(tso(A::Xl, P::Mkt, "LX")); }
                if c[i] > c[i - 2] { next.push(tso(A::Sell, P::Stop(c[i] - r * 0.10 * vb), "SE")); }
                if st.pos == -1 && st.bse(i) >= vc { next.push(tso(A::Xs, P::Mkt, "SX")); }
            })
        }
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn safe_buy_stop_then_next_open_exit() {
        let v = [(10., 10., 10., 10.), (10., 10., 10., 10.), (10., 12., 8., 9.), (10., 13., 9.5, 12.), (11.5, 12., 11., 11.)];
        let r: Vec<Row> = v.iter().enumerate().map(|(i, &(o, h, l, c))| Row { ticker: "T".into(), date: format!("2024-01-{:02}", i + 1), open: o, high: h, low: l, close: c, volume: None }).collect();
        let x = Ctx::new(&r, &r, &[]);
        // bar 2: close 9 < close[0] 10 -> buy stop at open(3) 10 + 0.4*4 = 11.6, filled bar 3 (high 13); exit bar 4 open 11.5
        let t = run("jkbk_safe", &x, 0, 4, &Params::new()).unwrap();
        assert_eq!(t.len(), 1);
        assert_eq!((t[0].legs[0].i, t[0].exit_i, t[0].exit_px), (3, 4, 11.5));
        assert!((t[0].legs[0].px - 11.6).abs() < 1e-9);
    }
}
