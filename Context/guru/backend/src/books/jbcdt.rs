//! Jake Bernstein, "The Compleat Day Trader: Trading Systems, Strategies, Timing Indicators, and Analytical Methods"
//! (McGraw-Hill, 1995). Book id `jbcdt`. Source: the author's chapter files (Chap01-24 .doc/.rtf; chapter 17 is missing
//! from the set), printed page numbers embedded in the text. Transcription and checks: library/jbcdt/.
//! Almost every method in this book runs on 5- to 30-minute bars; only the gap-open (GO) day trade is a daily-bar rule.
use super::jb624::{bar_path, finish, getp, mark_open, new_trade, stop_fill};
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "jbcdt";

macro_rules! def {
    ($id:expr, $name:expr, $group:expr, $dir:expr, $status:expr, $rules:expr, $notes:expr) => {
        StrategyDef { id: $id, name: $name, group: $group, session: 0, dir: $dir, status: $status, needs_volume: false,
            needs_vix: false, needs_series: &[], book: BOOK, rules: $rules, notes: $notes, claim: None }
    };
}

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::{Live, NeedsData, NotTestable};
    vec![
        def!("jbcdt_gap_go", "Gap open (GO) day trade · $2,500 S&P stop", "Ch.6 · Gaps, Gaps, Gaps", "Long/Short", Live,
            &["F|\"An opening gap occurs when today's opening price is either greater than yesterday's high price or less than yesterday's low price\" (p.56)",
              "E|\"If a lower opening price gap occurs, then you will place an order to buy once the market has come back up and penetrated the previous day's low by a given number of ticks ... a buy stop ... two ticks above the low of the previous day which would be 408.10 (408.00 + 0.10)\" (p.58-59); sell mirrored two ticks below yesterday's high (p.60-61)",
              "X|\"You will use a stop loss either at a predetermined dollar amount or several ticks below the day's low ... I prefer the money management dollar stop loss\"; \"a stop loss of as much as $2500 in S&P futures (500 points) is preferable\" (p.59-60): 5.00 S&P points from the entry",
              "X|\"both the gap lower buy trade and gap higher sell trade are closed out by the end of the trading session\" (p.62)"],
            "Two ticks = 0.10 S&P points as in the book's example. Points kept on every index (instrument swap only). Entry-day stop counted along the bar path (open, nearer extreme, other extreme, close). The alternative 'several ticks below the day's low at the time of the fill' needs intraday data."),
        def!("jbcdt_gap_dgo", "Delayed gap open (DGO)", "Ch.6 · Gaps, Gaps, Gaps", "Long/Short", NeedsData("intraday bars (price at the end of the first hour)"),
            &["E|\"If at the end of the first hour of trading the buy stop has not been elected ... If the current price is higher than the opening price by two ticks after the end of the first hour of trading, you will enter the long position at market\" (p.64-65)"], ""),
        def!("jbcdt_pop", "Intraday stochastic POP", "Ch.5 · Intraday Stochastics", "Long/Short", NeedsData("intraday bars (5- to 30-minute)"),
            &["E|75/25 stochastic POP on intraday bars, exit by the close (p.47-55)"], "The daily-bar POP of the 6-2-4 report is jb624_pop."),
        def!("jbcdt_mac", "Intraday MAC (two 5-minute bars outside the channel)", "Ch.7 · Support and Resistance: the MAC", "Long/Short", NeedsData("intraday 5-minute bars"),
            &["E|\"If, at any time after market opening or on market opening, the market develops two successive 5-minute price bars completely above the top of the channel\" (p.78)"], "Daily MAC versions: jb624_mac*."),
        def!("jbcdt_rsi", "RSI 25/75 and RSI vs its derivative", "Ch.8 · Wilder's RSI", "Long/Short", NotTestable("RSI length and levels are left to the trader (\"you can vary the values\"); the example is a 9-period RSI on 30-minute S&P data at 15/85; stops are 'under the low' or dollar amounts without values"),
            &["E|\"When RSI drops to 25 or lower and then turns back above 25 you buy ... When RSI has been at 75 or higher then sell short when it declines below 75\" (p.92)"], ""),
        def!("jbcdt_mom_roc", "Intraday momentum / ROC with moving average", "Ch.9-10 · Momentum, Rate of Change", "Long/Short", NeedsData("intraday bars (the 21-period MOM/ROC with 18-period MA is shown on intraday charts)"),
            &["E|\"by buying and selling when MOM crosses above and below its MA line\" (Fig. 9-2: 18-period MA of the 21-period MOM)"], ""),
        def!("jbcdt_cbo", "Channel breakout (CBO)", "Ch.11 · Intraday Channel Breakout", "Long/Short", NotTestable("length and penetration are the trader's choice (\"if you are using a 14-period length\"; \"x is defined as one or two ticks\"); exit is open (\"Risk or stop loss can be defined in many ways, or the system can be used as a reversing system\")"),
            &["E|\"a buy signal will occur when the high of the current bar exceeds the highest high of the previous 14 price bars by x number of ticks\" (p.107)"], "A fully specified 16-bar version is jbudt_tb16."),
        def!("jbcdt_oc_osc", "Open/close oscillator", "Ch.13 · A Unique Oscillator", "Long/Short", NeedsData("intraday bars (5- to 20-minute open/close averages)"),
            &["E|Moving averages of 5-minute opens versus closes, S&P intraday (ch.13)"], "The daily eight-bar open/close is jb30d_oc8."),
        def!("jbcdt_ctod", "Critical time of day (CTOD)", "Ch.20 · CTOD", "Long/Short", NeedsData("intraday 5-minute closes for the first two hours"),
            &["E|\"An upper breakout and buy signal will occur when prices have had a 5-minute close above the highest 5-minute closing plot established during the first 2 hours\" (ch.20)"], ""),
        def!("jbcdt_cc", "Consecutive closes system", "Ch.18 · Consecutive Closes", "Long/Short", NotTestable("the number of closes is \"a given number\" (5 is only an example) and the bar length is the trader's choice"),
            &["E|\"A buy signal and an up trend are defined when the market has made a given number of up closes in a row\" (p.170); \"you exit the long and reverse to short when you get a CCD sell signal\" (p.171)"], ""),
        def!("jbcdt_keydates", "Key seasonal dates and daily seasonals", "Ch.16 · Seasonality", "Long/Short", NotTestable("the S&P daily seasonal listings (Fig. 16-8) are images not present in the chapter files, and entries need an unspecified intraday timing confirmation"),
            &["F|\"Percentage of time up close 75 percent of the time or more\" (p.150)", "E|\"do not buy unless timing confirms the direction of the move\" (p.150)"], ""),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "jbcdt_gap_go" => ("jbcdt_gap", "Gap trades (Compleat Day Trader)", "GO · $2,500 stop, exit on the close"),
        "jbcdt_gap_dgo" => ("jbcdt_gap", "Gap trades (Compleat Day Trader)", "Delayed GO · needs intraday"),
        "jbcdt_pop" => ("jbcdt_intraday", "Compleat Day Trader intraday methods", "Stochastic POP"),
        "jbcdt_mac" => ("jbcdt_intraday", "Compleat Day Trader intraday methods", "MAC 5-minute"),
        "jbcdt_rsi" => ("jbcdt_intraday", "Compleat Day Trader intraday methods", "RSI · not testable"),
        "jbcdt_mom_roc" => ("jbcdt_intraday", "Compleat Day Trader intraday methods", "MOM/ROC with MA"),
        "jbcdt_cbo" => ("jbcdt_intraday", "Compleat Day Trader intraday methods", "Channel breakout · not testable"),
        "jbcdt_oc_osc" => ("jbcdt_intraday", "Compleat Day Trader intraday methods", "Open/close oscillator"),
        "jbcdt_ctod" => ("jbcdt_intraday", "Compleat Day Trader intraday methods", "CTOD"),
        "jbcdt_cc" => ("jbcdt_intraday", "Compleat Day Trader intraday methods", "Consecutive closes · not testable"),
        "jbcdt_keydates" => ("jbcdt_seasonal", "Key seasonal dates", "Not testable"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(match id {
        "jbcdt_gap_go" => vec![p("pa", "Penetration beyond yesterday's low/high (points; 2 ticks)", 0.1, 0., 2., 0.05), p("stop", "Stop loss (points; $2,500 at $500 per point)", 5., 1., 20., 0.5)],
        _ => return None,
    })
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if id != "jbcdt_gap_go" || to >= x.c.len() || from > to {
        return if id.starts_with("jbcdt_") && params(id).is_some() { Some(vec![]) } else { None };
    }
    let (pa, sl) = (getp(id, p, "pa"), getp(id, p, "stop"));
    let mut out = Vec::new();
    for i in from.max(1)..=to {
        let (d, lvl) = if x.o[i] < x.l[i - 1] { (1.0, x.l[i - 1] + pa) } else if x.o[i] > x.h[i - 1] { (-1.0, x.h[i - 1] - pa) } else { continue };
        let Some(px) = stop_fill(x, i, d, lvl) else { continue };
        let stop = px - d * sl;
        let path = bar_path(x, i);
        let k = (0..4).find(|&k| (path[k] - lvl) * d >= 0.0).unwrap_or(0);
        let t = new_trade(d, i, px, "stop", i - 1);
        out.push(if (k..4).any(|m| (path[m] - stop) * d <= 0.0) { finish(t, i, stop, i, "Stop loss ($2,500)") } else { finish(t, i, x.c[i], i, "End of day") });
    }
    mark_open(&mut out, None, x, to);
    Some(out)
}
