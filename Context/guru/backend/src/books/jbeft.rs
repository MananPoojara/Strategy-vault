//! Jake Bernstein, "Strategies for the Electronic Futures Trader" (McGraw-Hill, 2000). Book id `jbeft`.
//! Source: a 258-page scan, read as page images (printed page = PDF page - 17). Transcription and checks: library/jbeft/.
//! The systems of ch.7-11 are built on 15- or 60-minute (or 1-minute) S&P data; the only daily-bar systems are the
//! S&P day-of-week systems of ch.8, which use the DBO simulator in `jbcdt2`.
use super::jb624::getp;
use super::jbcdt2::dbo;
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "jbeft";

macro_rules! def {
    ($id:expr, $name:expr, $group:expr, $dir:expr, $status:expr, $rules:expr, $notes:expr) => {
        StrategyDef { id: $id, name: $name, group: $group, session: 0, dir: $dir, status: $status, needs_volume: false,
            needs_vix: false, needs_series: &[], book: BOOK, rules: $rules, notes: $notes, claim: None }
    };
}

const DOW_RULES: &str = "Setup: \"If the close on Friday is greater than the open on Friday, then a buy condition is set for Monday ... less than the open ... a sell condition\"; trigger: \"on Monday buy on stop x ticks above the Friday high ... sell short on stop x ticks below the Friday low\"; \"Exit either at the indicated stop loss or on the first profitable opening\" (p.131); FPO = \"the first opening after you enter that gives you a profit compared to your entry price\" (p.132). Friday = last trading day of the week, Monday = the next trading day [the book does not cover holidays]; the stop counts on the entry day only if the bar path reaches it after the fill [daily-bar convention].";

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::{Live, NeedsData, NotTestable};
    vec![
        StrategyDef { claim: crate::connors::claim("50", "78%", "$1,066.50 (1 S&P contract)", "2 bars (winners)", "S&P futures 11/03/97-08/31/99, PF 1.87"),
          ..def!("jbeft_dow_fm97", "S&P day-of-week Friday-Monday · 1997-99 inputs (4 / 2 ticks, $5,500)", "Ch.8 · The S&P Day-of-Week System", "Long/Short", Live,
            &["F|Friday close versus Friday open sets up Monday (p.126, 131)",
              "E|\"a 4-tick penetration of Friday's high on Monday for buy signals and a 2-tick penetration of Friday's low on Monday for sell signals\" (p.127) = 0.40 / 0.20 S&P points [Not from the book: the book never states the tick size; 0.10 point is the contract specification of the $250-per-point contract]",
              "X|\"with a $5500 stop loss\" (p.127) = 22 S&P points (\"using the $250 per 100 points S&P value\", p.127; every Fig 8-6 loss is -$5,575); \"Remember that exit is on FPO\" (p.127)"],
            DOW_RULES) },
        StrategyDef { claim: crate::connors::claim("384", "80%", "$370.12 (1 S&P contract)", "2 bars (winners)", "S&P futures 4/21/82-10/31/97, PF 1.58"),
          ..def!("jbeft_dow_wt", "S&P day-of-week Wednesday-Thursday (6 / 22 ticks, $3,850)", "Ch.8 · The S&P Day-of-Week System", "Long/Short", Live,
            &["F|\"Figure 8-7 shows the Wednesday-Thursday relationship in S&P from 1982 to 1997\" (p.127): Wednesday's close versus open sets up Thursday",
              "E|\"based on a 6-tick penetration of Wednesday's high on Thursday and a 22-tick penetration of Wednesday's low on Thursday\" (p.131) = 0.30 / 1.10 S&P points [Not from the book: 0.05-point tick of the $500-per-point contract, from the contract specification]",
              "X|\"with a $3850 stop loss\" (p.131) = 7.7 S&P points at $500 per point; exit on the first profitable opening (p.131)"],
            "Needs a Wednesday bar followed by a Thursday bar. Fills, entry-day stop and FPO as jbcdt2_dbo_thu (CDT II prints other Thursday inputs: 6 / 21 ticks, $3,500).") },
        def!("jbeft_dow_fm82", "S&P day-of-week Friday-Monday · 1982-97 inputs (3 / 26 ticks, $3,400)", "Ch.8 · The S&P Day-of-Week System", "Long/Short", NotTestable("duplicate: identical inputs to jbcdt2_dbo_mon (CDT II Figure 5-8); this book's 1982-97 record (392 trades, 78%, $425.57) is compared there"),
            &["E|\"the trade trigger on Monday a 3-tick penetration of Friday's high for buy signals and a 26-penetration of Friday's low for sell signals, with a $3400 stop loss\" (p.127)"], ""),
        def!("jbeft_parabolic", "Parabolic system for short-term trading", "Ch.7 · Two Trend-Following Strategies", "Long/Short", NotTestable("re-read in lane K44 with the SPX 1-minute bars at hand: the inputs give one acceleration factor per side (15-minute .0171 buy / .0015 sell, 60-minute .026 / .006, p.109), but Wilder's parabolic needs a starting factor, a step and a maximum; the book does not say which of these the number is or what the others are, and the TradeStation 2000i strategy's own inputs are not printed"),
            &["E|\"Buy when price penetrates its parabolic buy number. Sell short or reverse position when price penetrates its parabolic sell number\" (p.107)",
              "X|Money management stop, floor level, trailing stop of x% of open profit (p.107); 15-minute inputs: AF .0171 buy / .0015 sell, $5,000 stop, $2,500 floor, 62% trail; 60-minute: AF .026 / .006, $3,500 long / $5,300 short, $2,600 floor, 90% trail (p.109)"],
            "Book: 101 trades 72% $1,342.57 (15-min, 10/7/98-3/18/99); 117 trades 70% $1,129.70 (60-min, 11/27/97-3/18/99)."),
        def!("jbeft_mom_ma", "MOM/MA system (60-minute S&P)", "Ch.7 · Two Trend-Following Strategies", "Long/Short", NotTestable("re-read in lane K44 with the SPX 1-minute bars at hand: the $3,100 / $4,400 stops and the $2,100 floor (p.119) are S&P contract dollars, and the contract's value per point depends on the test window ($500 a point until 3 Nov 1997, $250 after), but the window of the Fig 7-10 test is not printed, so the amounts cannot be turned into exact points (decision #202)"),
            &["E|\"If momentum crosses above its MA for at least two consecutive postings, buy. If momentum crosses below its MA for at least two consecutive postings, sell\" (p.113); momentum 16, MA 19 (p.114)",
              "X|$3,100 long / $4,400 short stop, $2,100 floor, 66% trail (p.119)"], "Daily MOM/MA versions: jbsmsw_mom_ma, jbssf_mom_ma. Book: 176 trades 60% $705.68 (Fig 7-10)."),
        def!("jbeft_mom_rise_fall", "Rising and falling momentum (60-minute S&P)", "Ch.7 · Two Trend-Following Strategies", "Long/Short", NotTestable("re-read in lane K44: the rule is only named (\"use rising and falling momentum ... without using the moving average of momentum\"; \"Buy signal MA: 7 periods of the closing price; Sell signal MA: 28 periods\", p.119); what crosses what is not stated"),
            &["E|\"use rising and falling momentum ... to generate buy and sell signals without using the moving average of momentum\" (p.119)"], "Book: 171 trades 65% $940.50 (Fig 7-11)."),
        StrategyDef { claim: crate::connors::claim("135", "62%", "$858.70 (1 S&P contract)", "29 bars (winners) / 12 (losers)", "15-min S&P futures 3/1/1997-11/1/1997"),
          ..def!("jbeft_oc_osc", "Open/close oscillator system (15-minute S&P)", "Ch.8 · Two Market Pattern Systems", "Long/Short", Live,
            &["F|\"construct a moving average of the open and a moving average of the close. The second step is to subtract the moving average of the open from the moving average of the close. And the third step is to construct a moving average of the difference between these two\" (p.120, 123); MA of open 46, MA of close 34, MA of difference 39 (p.123)",
              "E|\"when the MA of the open minus the MA of the close crosses above its moving average, the market tends to move higher. When the MA of the open minus the MA of the close moves below its moving average, a sell signal develops\" (p.123): buy on the cross above, sell short (or reverse) on the cross below",
              "X|\"Money management stop on short position: $3500\", \"on long position: $3700\", \"Floor level: $3900\", \"Percent trail: 75\" (p.123); the trailing stop is used \"as soon as the minimum profit target is hit. This target is called the floor level ... The trailing stop loss is a percentage of the open profit ... As soon as a new high is made in the open profit, the trailing stop changes. If the trade is stopped out, then the system goes flat until the next signal develops\" (p.107, 109)",
              "A|Read: the difference is MA(close, 34) - MA(open, 46) as p.120 defines it; p.123 names the oscillator \"MA of the open minus the MA of the close\", but Figure 8-3 (p.124, \"MA Open-Close\") plots it rising with price and marks the buys where it crosses up, which only MA(close) - MA(open) does [Declared reading]. Simple moving averages; a cross = above now and at or below on the bar before [Declared reading]",
              "A|Read: the dollar amounts are the book's 3/1/1997-11/1/1997 contract at $500 a point: 7.40 / 7.00 points money stop, 7.80 points floor, run on SPX only in S&P index points (decision 196(2)); percent trail = the TradeStation built-in: once the peak open profit reaches the floor, the stop risks 75% of it (entry + 25% of the peak), as Pruitt & Hill p.73-74 explain SetPercentTrailing (\"TradeStation will risk a specific percentage of maximum open profit\") [Declared reading]",
              "A|Read: session-aligned 15-minute bars built from the 1-minute file (09:30-16:00 ET), one continuous series across days (positions carry overnight); signals at a bar's close fill at the next bar's open; the money stop and the trail are built-in stops, live from the fill and walked minute by minute (the peak open profit uses the minute extremes); a stop fills at its level, or at the minute's open when it opens through it [Declared reading]. Inside a minute: open, the nearer extreme, the farther extreme, close (a tie: the extreme against the position first) [convention, counted as ambiguous]"],
            "Daily eight-bar open/close: jb30d_oc8. Book: 135 trades, 62% winners, $858.70 a trade (Fig 8-4, 15-minute S&P futures 3/1/1997-11/1/1997), before the vault's minute history; long 67 / short 68. No costs in the vault's run. Cash-index shorts are hypothetical.") },
        def!("jbeft_channel_break", "Channel break system (60-minute S&P)", "Ch.9 · Two Breakout Systems", "Long/Short", NotTestable("coded on real SPX 1-minute bars in lane K45 (60-minute bars; $3,400 / $2,900 = 13.6 / 11.6 points, $1,500 floor = 6 points, 90% trail at the book's $250-a-point 1998-99 contract), but 413 of 2,725 SPX trades (15.2%, 2014-2026) depend on the order inside one minute (a minute that sets the new open-profit peak or fills an entry and also reaches the raised trailing stop), above the 5% limit set before results (decision 175); and on tradable SPY with the points scaled (decision 177b, 2014-01-02..2025-10-21) the SPX result is 16x the SPY result (+1,148.9% vs +71.0% compounded). No result is published."),
            &["E|\"Intrabar channel break ... buys and sells on any intraday or intrabar penetration\"; \"Bars of low: 12\", \"Bars of high: 8\" (p.133-134)",
              "X|\"Stop loss short exit: $2900\", \"Stop loss long exit: $3400\", \"Floor level: $1500\", \"Percent trailing stop: 90\" (p.134); trailing stop as p.107, 109"],
            "Book: 136 trades, 75% winners, $1,030.88 a trade (Fig 9-2, 60-minute S&P futures 1/27/1998-3/18/1999); long 71 (83%) / short 65 (66%). The text says the sell lookback should be the shorter one (p.134), the printed inputs are 8 bars for the high and 12 for the low: the inputs are coded as printed."),
        def!("jbeft_range_breakout", "Range breakout system", "Ch.9 · Two Breakout Systems", "Long/Short", NotTestable("\"Percentage penetration: 96\" of the 15-bar window and the 21-bar \"momentum filter\" are not defined, and no initial stop is given (p.139)"),
            &["E|\"a volatility breakout system that buys or sells on a penetration above or below the true range of a give number of price bars\" (p.139)"], "Book: 72 trades 72% $1,154.17, hourly data (Fig 9-3)."),
        def!("jbeft_3_3_channel", "3/3 high/low channel with momentum trend", "Ch.10 · Support and Resistance", "Long/Short", NotTestable("the author: \"The 3/3 channel method is not a trading system per se\" (p.153); its rules conflict on bar size (\"Be out of all trades by the end of the day\", p.144, versus \"designed for short-term trading on daily data\", p.153), and the momentum length behind the trend (14, p.141-142) comes with no entry order rule beyond \"buy at support\""),
            &["E|\"If momentum has been above the zero line for at least two postings, then the trend is considered up. If the trend is up, then buy at support\" = 3-period MA of lows; \"take profit at resistance\" = 3-period MA of highs (p.144)",
              "X|\"Use a stop loss that is 100 percent of the average of the last 3 trading ranges-close only\"; \"Be out of all trades by the end of the day\" (p.144)"], "The daily 3-bar high/low channel with a MACD trend is jb624_3hl."),
        def!("jbeft_dsi", "Daily Sentiment Index extremes", "Ch.12 · Additional Methods", "Long/Short", NeedsData("Daily Sentiment Index history (MBH), plus intraday timing"),
            &["E|\"I prefer to use the 90 and 10 percent levels as trigger points\"; best \"looking for short-term sell signals on intraday data\" (p.169)"], ""),
        def!("jbeft_gold_kd", "October gold key date · long Sep 4 to Sep 24, 4% stop", "Ch.4 · Seasonality", "Long", NeedsData("October gold futures contract histories (the seasonal belongs to gold; the vault has no metal futures); the book also does not say whether fills are at the open or the close"),
            &["F|Figure 4-1 (p.62): \"OCT GOLD LONG Enter: 9/4 Exit: 9/24 Stop: 4\"; 24 trades 1975-98, 79.17%"], ""),
        def!("jbeft_add", "Advance/decline derivative (ADD)", "Ch.12 · Additional Methods", "Long/Short", NotTestable("\"objective, but not entirely systematic ... you will need to add a risk management stop loss\" (p.173); the daily ADD of another book is jbsmsw_add"),
            &["E|A/D oscillator against its moving average (p.172-173)"], ""),
    ]
}

/// Admission (bernstein-g3, 2026-10-22): the two daily S&P day-of-week systems, on the S&P 500 index itself only (the
/// offsets and stops are S&P points) and only where its opens are real (stop entries and first-profitable-opening exits).
pub const REVIEWED: &[&str] = &["jbeft_dow_fm97", "jbeft_dow_wt"];
pub fn reviewed_daily(id: &str) -> bool {
    REVIEWED.contains(&id)
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "jbeft_dow_fm97" => ("jbeft_dow", "S&P day-of-week system (EFT)", "Friday-Monday · 1997-99 inputs"),
        "jbeft_dow_wt" => ("jbeft_dow", "S&P day-of-week system (EFT)", "Wednesday-Thursday"),
        "jbeft_dow_fm82" => ("jbeft_dow", "S&P day-of-week system (EFT)", "Friday-Monday 1982-97 · see jbcdt2_dbo_mon"),
        "jbeft_oc_osc" | "jbeft_channel_break" => ("jbeft_intraday", "EFT intraday systems", "15/60-minute bars from the 1-minute file"),
        "jbeft_parabolic" | "jbeft_mom_ma" | "jbeft_mom_rise_fall" => ("jbeft_intraday", "EFT intraday systems", "Needs intraday bars"),
        "jbeft_3_3_channel" => ("jbeft_other", "EFT other methods", "Not testable / needs data"),
        "jbeft_range_breakout" | "jbeft_dsi" | "jbeft_add" | "jbeft_gold_kd" => ("jbeft_other", "EFT other methods", "Not testable / needs data"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let dbo = |b, s, st| vec![p("buy_off", "Buy stop above the setup day's high (points)", b, 0., 10., 0.05), p("sell_off", "Sell stop below the setup day's low (points)", s, 0., 10., 0.05),
        p("stop", "Stop loss from the entry (points)", st, 1., 40., 0.1), p("entry_day", "Entry-day stop: 0 = bar path O-near-far-C, 1 = any touch", 0., 0., 1., 1.)];
    Some(match id {
        "jbeft_dow_fm97" => dbo(0.4, 0.2, 22.0),
        "jbeft_dow_wt" => dbo(0.3, 1.1, 7.7),
        // intraday EP block (K45): S&P points at the test window's contract
        "jbeft_oc_osc" => vec![p("ma_open", "Moving average of open (bars)", 46., 2., 200., 1.), p("ma_close", "Moving average of close (bars)", 34., 2., 200., 1.),
            p("ma_diff", "Moving average of difference (bars)", 39., 2., 200., 1.), p("stop_long", "Money stop, long (S&P points; $3,700 at $500)", 7.4, 0.5, 50., 0.1),
            p("stop_short", "Money stop, short (S&P points; $3,500 at $500)", 7.0, 0.5, 50., 0.1), p("floor", "Floor level (S&P points of open profit; $3,900 at $500)", 7.8, 0.1, 50., 0.1),
            p("trail", "Percent trail: share of the peak open profit risked", 75., 1., 99., 1.)],
        "jbeft_channel_break" => vec![p("bars_high", "Bars of high", 8., 2., 100., 1.), p("bars_low", "Bars of low", 12., 2., 100., 1.),
            p("stop_long", "Stop loss long exit (S&P points; $3,400 at $250)", 13.6, 0.5, 50., 0.1), p("stop_short", "Stop loss short exit (S&P points; $2,900 at $250)", 11.6, 0.5, 50., 0.1),
            p("floor", "Floor level (S&P points of open profit; $1,500 at $250)", 6.0, 0.1, 50., 0.1), p("trail", "Percent trailing stop: share of the peak open profit risked", 90., 1., 99., 1.)],
        _ => return None,
    })
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !id.starts_with("jbeft_") || params(id).is_none() || to >= x.c.len() || from > to {
        return None;
    }
    let g = |k: &str| getp(id, p, k);
    if crate::intraday_ep::id(id) || id == "jbeft_channel_break" { // intraday EP block (K45)
        let u = |k: &str| g(k).round().max(1.0) as usize;
        let am = Amounts { sl: g("stop_long"), ss: g("stop_short"), floor: g("floor"), keep: 1.0 - g("trail") / 100.0 };
        return Some(if id == "jbeft_oc_osc" { oc_osc(x, from, to, u("ma_open"), u("ma_close"), u("ma_diff"), am) } else { channel(x, from, to, u("bars_high"), u("bars_low"), am) });
    }
    Some(dbo(x, from, to, id == "jbeft_dow_wt", g("buy_off"), g("sell_off"), g("stop"), g("entry_day") >= 0.5))
}

// ---- intraday EP block (lane K45, 4 Nov 2026, decision #214; authors/titles-1030/K45.tsv): the open/close oscillator (Ch.8
// p.120-125) on 15-minute and the channel break (Ch.9 p.133-136) on 60-minute bars built from the 1-minute file
// (intraday_ep::day_bars, session-aligned), one continuous series across days (EP2). Amounts in S&P points (SPX only).
#[derive(Clone, Copy)]
struct Amounts { sl: f64, ss: f64, floor: f64, keep: f64 }
/// One position: direction, entry price, entry day, peak open profit (points).
#[derive(Clone, Copy, PartialEq, Debug)]
struct Pos { d: f64, ep: f64, ei: usize, pk: f64 }
impl Amounts {
    /// The built-in stop of `p`: the money stop until the peak open profit reaches the floor, then entry + keep x peak.
    fn stop(&self, p: &Pos) -> (f64, &'static str) {
        if p.pk >= self.floor { (p.ep + p.d * self.keep * p.pk, "trailing stop") } else { (p.ep - p.d * if p.d > 0.0 { self.sl } else { self.ss }, "money stop") }
    }
}

/// One minute along `path` (open first): built-in stops of the position and the bar's entry stops `orders` (side, level;
/// `done` = filled this bar). The open fills every level it is through (exit first); later each segment fills the level
/// nearest its start in the direction of travel (exit first on a tie); the peak open profit rises at each segment's end.
fn walk(pos: &mut Option<Pos>, done: &mut [bool], orders: &[(f64, f64)], path: [f64; 4], am: Amounts, i: usize, kind: &'static str, out: &mut Vec<Trade>) {
    use crate::intraday::day_trade;
    for k in 0..4 {
        let to = path[k];
        let mut fr = if k == 0 { to } else { path[k - 1] };
        let mut at = false; // a fill happened at `fr`: a level at that same price is still reached (Codex review)
        let (up, down) = if k == 0 { (false, false) } else { (to > path[k - 1], to < path[k - 1]) }; // the segment's direction
        loop {
            // (key, side, level, order index: None = the position's stop)
            let mut hit: Option<((f64, usize), f64, f64, Option<usize>, &'static str)> = None;
            let mut cand: Vec<(usize, f64, f64, Option<usize>, &'static str)> = vec![];
            if let Some(p) = pos { let (st, why) = am.stop(p); cand.push((0, -p.d, st, None, why)); }
            for (j, &(side, lv)) in orders.iter().enumerate() {
                if !done[j] && pos.map_or(true, |p| p.d != side) { cand.push((1 + j, side, lv, Some(j), "reverse")); }
            }
            for (pri, side, lv, j, why) in cand {
                let (ok, key) = if k == 0 { ((side > 0.0 && to >= lv) || (side < 0.0 && to <= lv), (0.0, pri)) }
                    else { ((up && side > 0.0 && (fr < lv || (at && fr == lv)) && lv <= to) || (down && side < 0.0 && to <= lv && (lv < fr || (at && lv == fr))), ((lv - fr).abs(), pri)) };
                if ok && hit.map_or(true, |h| key.0 < h.0 .0 || (key.0 == h.0 .0 && key.1 < h.0 .1)) { hit = Some((key, side, lv, j, why)); }
            }
            let Some((_, side, lv, j, why)) = hit else { break };
            let px = if k == 0 { to } else { lv };
            match j {
                None => { let p = pos.take().unwrap(); out.push(day_trade(p.d, p.ei, p.ep, i, px, why, kind, false)); }
                Some(j) => {
                    done[j] = true;
                    if let Some(p) = pos.take() { out.push(day_trade(p.d, p.ei, p.ep, i, px, why, kind, false)); }
                    *pos = Some(Pos { d: side, ep: px, ei: i, pk: 0.0 });
                }
            }
            fr = px;
            at = true;
        }
        if let Some(p) = pos.as_mut() { if p.d * (to - p.ep) > p.pk { p.pk = p.d * (to - p.ep); } }
    }
}

/// The minute's path: the extreme nearer the open first; a tie: the extreme against the held position first (flat: low).
fn path(q: &crate::intraday::Bar1, pos: &Option<Pos>) -> [f64; 4] {
    let high_first = if q.h - q.o < q.o - q.l { true } else if q.o - q.l < q.h - q.o { false } else { pos.map_or(false, |p| p.d < 0.0) };
    if high_first { [q.o, q.h, q.l, q.c] } else { [q.o, q.l, q.h, q.c] }
}

/// Session-aligned n-minute bars of rows from..=to as one series: (daily row, minutes of the bar).
fn series(x: &Ctx, from: usize, to: usize, n: u16) -> Vec<(usize, Vec<crate::intraday::Bar1>)> {
    let mut b = vec![];
    for i in from..=to {
        if let Some((_, _, m, v)) = crate::intraday_ep::day_bars(x, i, n) { b.extend(v.iter().map(|z| (i, m[z.a..z.b].to_vec()))); }
    }
    b
}

fn finish(pos: Option<Pos>, b: &[(usize, Vec<crate::intraday::Bar1>)], kind: &'static str, mut out: Vec<Trade>) -> Vec<Trade> {
    if let (Some(p), Some((i, m))) = (pos, b.last()) { out.push(crate::intraday::day_trade(p.d, p.ei, p.ep, *i, m[m.len() - 1].c, "open", kind, true)); }
    out
}

/// Simple average of the last n values, a fresh left-to-right sum of the window; NaN until defined.
fn sma(v: &[f64], n: usize) -> Vec<f64> {
    (0..v.len()).map(|k| if k + 1 >= n && v[k + 1 - n..=k].iter().all(|z| z.is_finite()) { v[k + 1 - n..=k].iter().sum::<f64>() / n as f64 } else { f64::NAN }).collect()
}

/// Open/close oscillator: D = MA(close, mc) - MA(open, mo), signal line MA(D, md); a cross at a 15-minute close fills at the
/// next bar's open (reverses an opposite position); money stop / floor / percent trail as built-in stops.
fn oc_osc(x: &Ctx, from: usize, to: usize, mo: usize, mc: usize, md: usize, am: Amounts) -> Vec<Trade> {
    let b = series(x, from, to, 15);
    let o: Vec<f64> = b.iter().map(|z| z.1[0].o).collect();
    let c: Vec<f64> = b.iter().map(|z| z.1[z.1.len() - 1].c).collect();
    let (ao, ac) = (sma(&o, mo), sma(&c, mc));
    let d: Vec<f64> = ao.iter().zip(&ac).map(|(a, b)| b - a).collect();
    let k0 = d.iter().position(|v| v.is_finite()).unwrap_or(d.len());
    let s: Vec<f64> = std::iter::repeat(f64::NAN).take(k0).chain(sma(&d[k0..], md)).collect();
    let (mut out, mut pos, mut pend) = (vec![], None::<Pos>, 0.0);
    for (k, (i, m)) in b.iter().enumerate() {
        for (j, q) in m.iter().enumerate() {
            if j == 0 && pend != 0.0 {
                if let Some(p) = pos { // the open gaps through the old position's stop: that stop fills first (Codex review)
                    let (st, why) = am.stop(&p);
                    if p.d * (q.o - st) <= 0.0 { out.push(crate::intraday::day_trade(p.d, p.ei, p.ep, *i, q.o, why, "open", false)); pos = None; }
                }
                if pos.map_or(true, |p| p.d != pend) {
                    if let Some(p) = pos.take() { out.push(crate::intraday::day_trade(p.d, p.ei, p.ep, *i, q.o, "reverse", "open", false)); }
                    pos = Some(Pos { d: pend, ep: q.o, ei: *i, pk: 0.0 });
                }
                pend = 0.0;
            }
            let pth = path(q, &pos);
            walk(&mut pos, &mut [], &[], pth, am, *i, "open", &mut out);
        }
        if k >= 1 && s[k].is_finite() && s[k - 1].is_finite() {
            if d[k] > s[k] && d[k - 1] <= s[k - 1] { pend = 1.0; } else if d[k] < s[k] && d[k - 1] >= s[k - 1] { pend = -1.0; }
        }
    }
    finish(pos, &b, "open", out)
}

/// Channel break: at each 60-minute close, a buy stop at the highest high of the last `bh` bars and a sell stop at the lowest
/// low of the last `bl` bars (that bar included) for the next bar; stop and reverse; money stop / floor / percent trail.
fn channel(x: &Ctx, from: usize, to: usize, bh: usize, bl: usize, am: Amounts) -> Vec<Trade> {
    let b = series(x, from, to, 60);
    let (hi, lo): (Vec<f64>, Vec<f64>) = b.iter().map(|z| (z.1.iter().map(|q| q.h).fold(f64::MIN, f64::max), z.1.iter().map(|q| q.l).fold(f64::MAX, f64::min))).unzip();
    let (mut out, mut pos) = (vec![], None::<Pos>);
    for (k, (i, m)) in b.iter().enumerate() {
        let orders: Vec<(f64, f64)> = if k >= bh.max(bl) {
            vec![(1.0, hi[k - bh..k].iter().copied().fold(f64::MIN, f64::max)), (-1.0, lo[k - bl..k].iter().copied().fold(f64::MAX, f64::min))]
        } else { vec![] };
        let mut done = vec![false; orders.len()];
        for q in m { let pth = path(q, &pos); walk(&mut pos, &mut done, &orders, pth, am, *i, "stop", &mut out); }
    }
    finish(pos, &b, "stop", out)
}
// ---- end intraday EP block

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intraday::Bar1;
    fn q(o: f64, h: f64, l: f64, c: f64) -> Bar1 { Bar1 { m: 600, o, h, l, c } }
    const OC: Amounts = Amounts { sl: 7.4, ss: 7.0, floor: 7.8, keep: 0.25 };
    fn step(pos: &mut Option<Pos>, done: &mut [bool], orders: &[(f64, f64)], z: Bar1, out: &mut Vec<Trade>) { let pth = path(&z, pos); walk(pos, done, orders, pth, OC, 3, "stop", out); }
    fn t(v: &[Trade]) -> Vec<(f64, f64, f64, String)> { v.iter().map(|z| (z.dir, z.legs[0].px, (z.exit_px * 1e9).round() / 1e9, z.reason.clone())).collect() }
    #[test]
    fn k45_built_in_stops_by_hand() {
        let mut out = vec![];
        // long at 100: the peak open profit 10 passes the 7.8 floor, so the stop is 100 + 25% of 10 = 102.5 (75% of it risked)
        let mut pos = Some(Pos { d: 1.0, ep: 100.0, ei: 0, pk: 0.0 });
        step(&mut pos, &mut [], &[], q(100., 110., 100., 109.), &mut out);
        assert_eq!(pos.map(|p| p.pk), Some(10.0));
        step(&mut pos, &mut [], &[], q(105., 105.5, 101., 101.5), &mut out);
        assert_eq!((pos, t(&out)), (None, vec![(1., 100., 102.5, "trailing stop".into())]));
        // short at 100, the minute opens through the 7.0 money stop: out at the open
        let mut pos = Some(Pos { d: -1.0, ep: 100.0, ei: 0, pk: 0.0 });
        step(&mut pos, &mut [], &[], q(108., 108.5, 107.5, 108.), &mut out);
        assert_eq!(t(&out[1..]), vec![(-1., 100., 108., "money stop".into())]);
        // a raised trail reached in the same minute: high nearer the open goes first (the convention), so it stops at the raised level
        let mut pos = Some(Pos { d: 1.0, ep: 100.0, ei: 0, pk: 8.0 }); // stop 102
        step(&mut pos, &mut [], &[], q(106., 110., 101., 103.), &mut out); // high first: peak 10 -> stop 102.5, then down to 101
        assert_eq!((pos, t(&out[2..])), (None, vec![(1., 100., 102.5, "trailing stop".into())]));
    }
    #[test]
    fn k45_channel_orders_by_hand() {
        let mut out = vec![];
        // flat with a buy stop at 101 and a sell stop at 99: the low (nearer the open) first misses 99, then the high fills 101
        let (mut pos, mut done) = (None, vec![false, false]);
        step(&mut pos, &mut done, &[(1., 101.), (-1., 99.)], q(100., 101.5, 99.5, 101.), &mut out);
        assert_eq!((pos.map(|p| (p.d, p.ep, p.pk)), done.clone(), out.len()), (Some((1., 101., 0.5)), vec![true, false], 0));
        // long at 100: a fall reaches the 98 sell stop before the 92.6 money stop: reverse at 98; the short's peak is 1
        let (mut pos, mut done) = (Some(Pos { d: 1.0, ep: 100.0, ei: 0, pk: 0.0 }), vec![false, false]);
        step(&mut pos, &mut done, &[(1., 105.), (-1., 98.)], q(99., 99., 97., 97.5), &mut out);
        assert_eq!((pos.map(|p| (p.d, p.ep, p.pk)), t(&out)), (Some((-1., 98., 1.)), vec![(1., 100., 98., "reverse".into())]));
        // the open gaps through the buy stop: filled at the open
        let (mut pos, mut done) = (None, vec![false]);
        step(&mut pos, &mut done, &[(1., 101.)], q(102., 102.5, 101.8, 102.2), &mut out);
        assert_eq!(pos.map(|p| p.ep), Some(102.));
        let m = sma(&[1., 2., 3., 4.], 3);
        assert!(m[0].is_nan() && m[1].is_nan() && m[2..] == [2., 3.]);
    }
    /// Lane K45 engine-vs-reference dump (run by hand; K45w/cmp_dump.py compares with k45_ref.py):
    /// VAULT_US=<data/us_data.csv> VAULT_M1_DIR=<data/intraday> K45_OUT=<file> cargo test --release k45_dump -- --ignored
    #[test]
    #[ignore]
    fn k45_dump() {
        use std::io::Write;
        let rows = crate::load_csv(&std::env::var("VAULT_US").unwrap()).unwrap().remove("SPX").unwrap();
        let mut x = Ctx::new(&rows, &rows, &[]);
        x.minutes = Some(Box::leak(Box::new(crate::intraday::Minutes::load(&std::env::var("VAULT_M1_DIR").unwrap(), "SPX").unwrap())));
        let from = rows.partition_point(|r| r.date.as_str() < x.minutes.unwrap().first());
        let mut f = std::io::BufWriter::new(std::fs::File::create(std::env::var("K45_OUT").unwrap()).unwrap());
        for id in ["jbeft_oc_osc", "jbeft_channel_break"] {
            for (a, b) in [(from, rows.len() - 1), (rows.partition_point(|r| r.date.as_str() < "2014-01-02"), rows.len() - 1)] {
                for z in run(id, &x, a, b, &Params::new()).unwrap() {
                    writeln!(f, "{id},{},{},{},{},{},{},{},{}", rows[a].date, z.dir, rows[z.legs[0].i].date, z.legs[0].px, rows[z.exit_i].date, z.exit_px, z.reason, z.open).unwrap();
                }
            }
        }
    }
}
