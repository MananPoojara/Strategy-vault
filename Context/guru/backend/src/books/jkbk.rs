//! Joe Krutsinger product pack (etrackrecords.com / joekrut.com): "joe_krut.pdf" (45 pp: 2008 TRADERS' magazine
//! interview with TS 8.3 code, system operation sheets, the SAFE worksheet, the Sept 2000 Active Trader article
//! "Trading the Equity Curve"), the Boredom Killer Readme.txt and "Joe K's notes for DCP.doc" (Dealer's Choice Pro).
//! Book id `jkbk`. Source copies: Books/Joe Krutsinger/{Boredom Killer,First Hour Breakout,Dealers Choice Pro}/.
//! Page refs = PDF page of joe_krut.pdf. Library: library/jkbk/. Simulator conventions: see `jkbwts`.
use super::jkbwts::{avg, buy_monday, entry, exit, sim, Ty};
use crate::connors::{claim, rsi, Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "jkbk";
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
        nt("jkbk_english_channel", "English Channel (book text; code version: upg18_english_channel)", &["F|60-minute or daily bars, input Length 2-9, trd 1 (p.22)"], ""),
        nt("jkbk_focal_point", "Focal Point", &["F|60-minute or daily bars, inputs B1,S1,B2,S2 0-5, Walk 1-20 (p.22)"], ""),
        nt("jkbk_gearshift", "GearShift (indexes / forex) (code version: upg18_gearshift)", &["F|60-minute bars, inputs LenRate, LenAver, LenBol1 2-14, LenBol2 3, LenBol3 2 (p.25)"], ""),
        nt("jkbk_parrot_bbe", "Parrot BBE (code version: upg18_parrot_bbe)", &["F|60-minute or daily bars, AfStep 0.0005-0.03, Move to BE 500, PT 24 (p.25)"], ""),
        nt("jkbk_starting_point", "Starting Point", &["F|60-minute or daily bars, VarA, VarB 2-20, VarC 1-20 (p.28)"], ""),
        nt("jkbk_time_charger", "Time Charger", &["F|60-minute bars, Length 2-9, Tim1/Tim2 830-1630 (p.28)"], "Intraday."),
        nt("jkbk_dealers_choice_pro", "Dealer's Choice Pro (morning breakout) (code version: upg18_dealers_choice_pro)", &["F|7-minute bars; morning breakout based on the close TWO days ago; inputs Trad(4) trades, BO(1) = 10% breakout, Tim(1200) last entry time, Amount $2,300 trailing stop; no entries in the last hour; last trade is the 'trend trade' (Joe K's notes for DCP.doc)"], "Intraday; the breakout formula itself is not given (DlrChoicePro.ela undecodable, Open Code.rar not opened)."),
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
