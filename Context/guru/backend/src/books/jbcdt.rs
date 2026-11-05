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
              "X|\"both the gap lower buy trade and gap higher sell trade are closed out by the end of the trading session\" (p.62)",
              "X|[intraday wave 2b] Default p.path=1: real SPX 1-minute bars decide the entry fill (at the level, or the minute's open when it opened through it) and whether the 5.00-point stop from the fill was reached after it. Fill minute that also reaches the stop: filled then stopped, unless it opened beyond the stop, then stopped only if it closed beyond it [convention]. SPX only (the book's fixed S&P points and 2 ticks of 0.05; no conversion is invented); from 2014, where SPX daily opens are real. p.path=0 = the old daily bar path [Not from the book]."],
            "Two ticks = 0.10 S&P points as in the book's example. Points kept on every index (instrument swap only). Entry-day stop counted along the bar path (open, nearer extreme, other extreme, close). The alternative 'several ticks below the day's low at the time of the fill' needs intraday data."),
        def!("jbcdt_gap_dgo", "Delayed gap open (DGO)", "Ch.6 · Gaps, Gaps, Gaps", "Long/Short", NeedsData("intraday bars (price at the end of the first hour)"),
            &["E|\"If at the end of the first hour of trading the buy stop has not been elected ... If the current price is higher than the opening price by two ticks after the end of the first hour of trading, you will enter the long position at market\" (p.64-65)"], ""),
        def!("jbcdt_pop", "Intraday stochastic POP (14-bar slow stochastic, 75/25, 5-minute bars)", "Ch.5 · Intraday Stochastics", "Long/Short", Live,
            &["F|\"Using a 14-period slow stochastic indicator, a buy signal will be triggered when percent K is at 75 percent or higher at the end of the period you are using\" (p.48); \"I prefer either 5- or 10-minute data\" (p.49). Slow %K = fast %D, slow %D = a moving average of slow %K, \"usually 3-period smoothed\" with the smoothed moving average SM = (previous sum - previous average + price) / period, first value a plain average (Appendix p.220-222)",
              "E|\"you will be buying immediately upon that 5- or 10-minute posting. Your buy will always be at the market\" (p.49); the sell \"once percent K falls below 25 percent after first having been above it\" (p.51); after an exit, %K under 75 and back to 75 or higher at a later bar's end: \"Simply enter a long position again\" (p.51)",
              "X|\"exit your position at the market as soon as percent K and percent D have crossed one another at the end of your 5- or 10-minute segment\" (p.49); \"Exit of the SP sell will be at the market once percent K and percent D have crossed\"; \"You must be out of your SP trade by the end of the day\" (p.51)",
              "A|Read: 5-minute bars (p.bar = 10 is the author's other choice); fast %K(14) on the bar's close, high and low (50 when the 14-bar range is 0), slow %K and %D = 3-bar smoothed averages (Appendix p.221); one continuous bar series across days. A buy = slow %K at or above 75 at a bar's close after a close below it; a sell = %K below 25 after a close at or above it; both fill at the next bar's open; the exit = the first bar close after the fill at which %K and %D have changed sides, filled at the next bar's open; out at the day's last 5-minute close; a signal on the day's last bar is dropped [Declared reading]. The optional 'risk management stop loss' has no value and is not modelled; the hit-and-run units (100 S&P points) and the aggressive SP (an unspecified close trailing stop, p.53-54) are variations, not this card.", "A|Minute path: market orders fill at the next bar's open (its first minute's open); a limit fills at its level, or at the minute's open when the minute opens through it; the bars are built from the vault's 1-minute file (intraday_ep::day_bars, session-aligned) [convention]. SPY, SPX, NIFTY 50 and NIFTYBANK: the book traded S&P and other futures, so these runs are a declared application; cash-index shorts are hypothetical."],
            "Lane K55 (decision #230). The author: 'more of a method than a solid mechanical system' (p.55); the stated rules are complete, so it is coded with the readings labelled. The daily-bar POP of the 6-2-4 report is jb624_pop. No costs in the vault's run."),
        def!("jbcdt_mac", "Intraday MAC (two 5-minute bars outside the channel, buy the channel midpoint)", "Ch.7 · Support and Resistance: the MAC", "Long/Short", Live,
            &["F|The channel = a moving average of highs (MAH) and of lows (MAL) (p.74-75); the lengths are printed only on Figures 7-1/7-5: a 10-period average of highs and an 8-period average of lows (the same 10/8 simple averages as the 6-2-4 report p.32, 'daily or weekly or intraday')",
              "F|\"If, at any time after market opening or on market opening, the market develops two successive 5-minute price bars completely above the top of the channel (i.e., above the moving average of the high), then the trend is presumed to be up\" (p.78); two bars completely below the MAL = down; \"take your direction from the very last signal. This signal may have occurred the day before\" (p.80)",
              "E|With the bullish bias: buy at the MAL (LBP, conservative), the midpoint (MBP) or the MAH (HBP, aggressive): \"Most traders are better off calculating the midpoint of the MAC by adding the MAL and the MAH and dividing by 2 ... you will want to maintain resting orders at the MBP\" (p.78); short mirror at the MSP (p.78-80)",
              "X|\"If you enter a long position on a decline to the LBP, then you can exit if there are two consecutive bars outside the MAL, which would, of course, constitute a sell signal ... A happy medium is buying at the MBP ... I prefer to use technical action in the form of a reversing signal to exit my trades as opposed to the use of a dollar risk stop loss\" (p.81); \"you will be out of your trades by the end of the day ... get out either MOC\" (p.82)",
              "A|Read: 5-minute bars, MAH = simple 10-bar average of highs, MAL = simple 8-bar average of lows, 'completely above' = the bar's low above the MAH (below: the high under the MAL), one continuous bar series, the bias carried across days. While the bias holds and no position is open, a buy limit at the bar's MBP works through the next bar (short: a sell limit at the MSP); the opposite two-bar signal exits at the next bar's open, and the new bias's limit works in that bar; out at the day's last 5-minute close; trades per day unlimited (p.80) [Declared reading]. p.level 0 = LBP/USP, 1 = HBP/LSP (the author's other two entry levels). The dollar stop ('varies from market to market') and the trailing stop ('Do not use a trailing stop loss until you show a reasonable profit', no amount) are not modelled.", "A|Minute path: market orders fill at the next bar's open (its first minute's open); a limit fills at its level, or at the minute's open when the minute opens through it; the bars are built from the vault's 1-minute file (intraday_ep::day_bars, session-aligned) [convention]. SPY, SPX, NIFTY 50 and NIFTYBANK: the book traded S&P and other futures, so these runs are a declared application; cash-index shorts are hypothetical."],
            "Lane K55 (decision #230). The author: 'a trading method and not a system' (p.83); the stated rules are complete with the readings labelled. Daily MAC versions: jb624_mac*. No costs in the vault's run."),
        def!("jbcdt_rsi", "RSI 25/75 and RSI vs its derivative", "Ch.8 · Wilder's RSI", "Long/Short", NotTestable("RSI length and levels are left to the trader (\"you can vary the values\"); the example is a 9-period RSI on 30-minute S&P data at 15/85; stops are 'under the low' or dollar amounts without values"),
            &["E|\"When RSI drops to 25 or lower and then turns back above 25 you buy ... When RSI has been at 75 or higher then sell short when it declines below 75\" (p.92)"], ""),
        def!("jbcdt_mom_roc", "Intraday momentum / ROC with moving average", "Ch.9-10 · Momentum, Rate of Change", "Long/Short", NotTestable("no exit and no stop in the text: the 18-period MA of the 21-period MOM (p.95) and of the 21-period ROC (p.101) give entries only; exits appear only as chart notes ('EXIT ON CLOSE', Fig. 9-1/9-2), and the author calls them 'merely applications and not systems' (p.94) needing 'considerably more research' (p.105). Held (decision #230 (4); Compleat Day Trader I printed p.94-105 = PDF 105-116, batch-h3/ocr/cdt1_ch09-10_mom_roc_pdf105-116.txt)"),
            &["E|\"by buying and selling when MOM crosses above and below its MA line\" (Fig. 9-2: 18-period MA of the 21-period MOM)"], ""),
        def!("jbcdt_cbo", "Channel breakout (CBO)", "Ch.11 · Intraday Channel Breakout", "Long/Short", NotTestable("length and penetration are the trader's choice (\"if you are using a 14-period length\"; \"x is defined as one or two ticks\"); exit is open (\"Risk or stop loss can be defined in many ways, or the system can be used as a reversing system\")"),
            &["E|\"a buy signal will occur when the high of the current bar exceeds the highest high of the previous 14 price bars by x number of ticks\" (p.107)"], "A fully specified 16-bar version is jbudt_tb16."),
        def!("jbcdt_oc_osc", "Open/close oscillator", "Ch.13 · A Unique Oscillator", "Long/Short", NotTestable("lengths, bar size and signal threshold are left to the trader: 'a smoothed moving average of the opening prices consisting of 6 to 10 periods on 5-, 10-, 15-, or 20-minute data' and closes of '12 to 24 periods'; 'You will need to adjust the lengths ... There are no hard and fast rules' (p.125); 'You will need to determine the crossover amount or threshold'; 'this is not a totally mechanical system' (p.126). Held (decision #230 (4); Compleat Day Trader I printed p.121-129 = PDF 132-140, batch-h3/ocr/cdt1_ch13_oc_pdf132-140.txt)"),
            &["E|Moving averages of 5-minute opens versus closes, S&P intraday (ch.13)"], "The daily eight-bar open/close is jb30d_oc8."),
        def!("jbcdt_ctod", "Critical time of day (CTOD): 5-minute close beyond the first two hours' closes", "Ch.20 · CTOD", "Long/Short", Live,
            &["F|\"Plot the 5-minute closing prices for the first 2 hours of trading regardless of market ... The price used is the last price, not the intraday high or low price\" (p.186); \"the highest price will be used as a buying breakout point, whereas the lowest price will be used as a selling breakout point\" (p.188)",
              "E|\"An upper breakout and buy signal will occur when prices have had a 5-minute 'close' (i.e., plot) above the highest 5-minute closing plot established during the first 2 hours. You will then buy at the market using a 5-minute closing stop loss below the lowest closing 5-minute plot of the first 2 hours\"; sell mirrored (p.188)",
              "X|\"You will use a trailing stop loss which consists of a 5-minute close below the lowest 5-minute close of each previous hour for longs and above the highest 5-minute close of the previous hour for short positions\"; \"You will exit positions either by the end of the day or by being stopped out\" (p.188); \"CTOD can give two signals each day, a buy and a sell, or it may ... give no signals whatsoever\" (p.190)",
              "A|Read: session-aligned 5-minute bars; the window = the bars ending within the first 120 minutes of the session (p.window 60/90 = the author's 'Experiment with different time frames such as 1 hour, or 1½ hours', p.188). 'Buy at the market' = the next bar's open after the signal close; a closing stop acts on a 5-minute close and fills at the next bar's open; 'previous hour' = the last completed hour counted from the session open (bars ending at open+60, +120, ...); the stop = the window's lowest close (short: highest), raised to each completed hour's lowest close and never lowered (trailing); at most one buy and one sell a day; a stop-out that is also the opposite breakout reverses at that open; out at the day's last 5-minute close; a signal on the day's last bar is dropped [Declared reading]. The 'exceptions later on' promised on p.186 never appear; the money-management stop and the two-unit scaling are alternatives, not this card.", "A|Minute path: market orders fill at the next bar's open (its first minute's open); a limit fills at its level, or at the minute's open when the minute opens through it; the bars are built from the vault's 1-minute file (intraday_ep::day_bars, session-aligned) [convention]. SPY, SPX, NIFTY 50 and NIFTYBANK: the book traded S&P and other futures, so these runs are a declared application; cash-index shorts are hypothetical."],
            "Lane K55 (decision #230). The author: 'a method, it is not a system, although it is systematic and has the potential to be used as a system' (p.186). No costs in the vault's run."),
        def!("jbcdt_cc", "Consecutive closes system", "Ch.18 · Consecutive Closes", "Long/Short", NotTestable("the number of closes is \"a given number\" (5 is only an example) and the bar length is the trader's choice"),
            &["E|\"A buy signal and an up trend are defined when the market has made a given number of up closes in a row\" (p.170); \"you exit the long and reverse to short when you get a CCD sell signal\" (p.171)"], ""),
        def!("jbcdt_keydates", "Key seasonal dates and daily seasonals", "Ch.16 · Seasonality", "Long/Short", NotTestable("entries need a timing confirmation the book never defines (\"do not buy unless timing confirms the direction of the move\", p.150); the S&P daily seasonal listings (Fig. 16-8, pp.154-156) cover only Aug 9 to Nov 30 from 6-8 years of data"),
            &["F|\"Percentage of time up close 75 percent of the time or more\" (p.150)", "E|\"do not buy unless timing confirms the direction of the move\" (p.150)"], ""),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "jbcdt_gap_go" => ("jbcdt_gap", "Gap trades (Compleat Day Trader)", "GO · $2,500 stop, exit on the close"),
        "jbcdt_gap_dgo" => ("jbcdt_gap", "Gap trades (Compleat Day Trader)", "Delayed GO · needs intraday"),
        "jbcdt_pop" => ("jbcdt_intraday", "Compleat Day Trader intraday methods", "Stochastic POP 5-minute"),
        "jbcdt_mac" => ("jbcdt_intraday", "Compleat Day Trader intraday methods", "MAC 5-minute midpoint"),
        "jbcdt_rsi" => ("jbcdt_intraday", "Compleat Day Trader intraday methods", "RSI · not testable"),
        "jbcdt_mom_roc" => ("jbcdt_intraday", "Compleat Day Trader intraday methods", "MOM/ROC with MA · no exit, held"),
        "jbcdt_cbo" => ("jbcdt_intraday", "Compleat Day Trader intraday methods", "Channel breakout · not testable"),
        "jbcdt_oc_osc" => ("jbcdt_intraday", "Compleat Day Trader intraday methods", "Open/close oscillator · trader's lengths, held"),
        "jbcdt_ctod" => ("jbcdt_intraday", "Compleat Day Trader intraday methods", "CTOD 5-minute"),
        "jbcdt_cc" => ("jbcdt_intraday", "Compleat Day Trader intraday methods", "Consecutive closes · not testable"),
        "jbcdt_keydates" => ("jbcdt_seasonal", "Key seasonal dates", "Not testable"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(match id {
        "jbcdt_gap_go" => vec![p("pa", "Penetration beyond yesterday's low/high (points; 2 ticks)", 0.1, 0., 2., 0.05), p("stop", "Stop loss (points; $2,500 at $500 per point)", 5., 1., 20., 0.5),
            p("path", "1 = real 1-minute bars (default; SPX only; days without minute data make no trade), 0 = [Not from the book] daily bar path", 1., 0., 1., 1.)], // intraday wave 2b
        // intraday EP block (K55)
        "jbcdt_ctod" => vec![p("window", "Opening window (minutes; the book's 2 hours, 60/90 the author's alternatives)", 120., 60., 120., 30.)],
        "jbcdt_pop" => vec![p("bar", "Bar size (minutes; 5 default, 10 the author's other choice)", 5., 5., 10., 5.), p("len", "Stochastic length (bars)", 14., 2., 50., 1.),
            p("upper", "Buy at or above (%K)", 75., 50., 95., 5.), p("lower", "Sell below (%K)", 25., 5., 50., 5.)],
        "jbcdt_mac" => vec![p("mah", "Average of highs (bars)", 10., 2., 50., 1.), p("mal", "Average of lows (bars)", 8., 2., 50., 1.),
            p("level", "Entry level inside the channel: 0 = LBP/USP, 0.5 = MBP/MSP (default), 1 = HBP/LSP", 0.5, 0., 1., 0.5)],
        _ => return None,
    })
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if crate::intraday_ep::id(id) && id.starts_with("jbcdt_") { // intraday EP block (K55)
        if to >= x.c.len() || from > to { return Some(vec![]); }
        let g = |k: &str| getp(id, p, k);
        let u = |k: &str| g(k).round().max(1.0) as usize;
        return Some(match id {
            "jbcdt_ctod" => ctod(x, from, to, g("window").round() as u16),
            "jbcdt_pop" => pop(x, from, to, u("bar") as u16, u("len"), g("upper"), g("lower")),
            _ => mac(x, from, to, u("mah"), u("mal"), g("level")),
        });
    }
    if id != "jbcdt_gap_go" || to >= x.c.len() || from > to {
        return if id.starts_with("jbcdt_") && params(id).is_some() { Some(vec![]) } else { None };
    }
    let (pa, sl) = (getp(id, p, "pa"), getp(id, p, "stop"));
    let mut out = Vec::new();
    for i in from.max(1)..=to {
        let (d, lvl) = if x.o[i] < x.l[i - 1] { (1.0, x.l[i - 1] + pa) } else if x.o[i] > x.h[i - 1] { (-1.0, x.h[i - 1] - pa) } else { continue };
        // ---- intraday wave 2b block: real SPX 1-minute bars (path = 1, default)
        if getp(id, p, "path") >= 0.5 {
            use crate::intraday::{stop_entry_day, Level};
            let Some(b) = crate::intraday_w2b::day(x, i) else { continue };
            let Some((_, px, st)) = stop_entry_day(&b, 0, &[Level { up: d > 0.0, px: lvl }], &|_, f| Some(Level { up: d < 0.0, px: f - d * sl })) else { continue };
            let t = new_trade(d, i, px, "stop", i - 1);
            out.push(match st { Some(xp) => finish(t, i, xp, i, "Stop loss ($2,500)"), None => finish(t, i, x.c[i], i, "End of day") });
            continue;
        }
        // ---- end intraday wave 2b block ----
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

// ---- intraday EP block (lane K55, 5 Nov 2026, decision #230; authors/titles-1030/K55.tsv)
// Compleat Day Trader I methods on N-minute bars built from the vault's 1-minute file (n1intra::series over
// intraday_ep::day_bars, session-aligned), one continuous series across days (EP2). Independent check: K55w/k55_ref.py.
use crate::intraday_ep::Book;

/// CTOD (ch.20, p.186-190): the first `win` minutes' highest / lowest 5-minute close; a later close beyond it buys / sells at
/// the next open, once per side a day; closing stops (window extreme, trailed to each completed hour's extreme close) fill
/// at the next open; out at the day's last close.
fn ctod(x: &Ctx, from: usize, to: usize, win: u16) -> Vec<Trade> {
    let b = super::n1intra::series(x, from, to, 5);
    let mut bk = Book::default();
    let mut k0 = 0;
    while k0 < b.len() {
        let k1 = (k0..b.len()).find(|&k| b[k].last).unwrap_or(b.len() - 1);
        let (day, open) = (&b[k0..=k1], b[k0].open);
        k0 = k1 + 1;
        let w: Vec<f64> = day.iter().filter(|z| z.s <= open + win).map(|z| z.c).collect();
        if w.is_empty() { continue; }
        let h2 = w.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let l2 = w.iter().copied().fold(f64::INFINITY, f64::min);
        let (mut lo, mut hi) = (vec![f64::INFINITY; 8], vec![f64::NEG_INFINITY; 8]); // each hour's lowest / highest close
        let (mut used_l, mut used_s, mut stop, mut pend_exit, mut pend) = (false, false, f64::NAN, false, 0.0);
        for z in day {
            if pend_exit { bk.exit(z.i, z.o, "closing stop"); pend_exit = false; }
            if pend != 0.0 { bk.enter(pend, z.i, z.o, "open", "breakout"); stop = if pend > 0.0 { l2 } else { h2 }; pend = 0.0; }
            let hr = ((z.s - 1 - open) / 60) as usize;
            let d = bk.dir();
            if d != 0.0 && hr >= 1 && hr - 1 < 8 {
                let prev = if d > 0.0 { lo[hr - 1] } else { hi[hr - 1] };
                if prev.is_finite() { stop = if d > 0.0 { stop.max(prev) } else { stop.min(prev) }; }
            }
            if hr < 8 { lo[hr] = lo[hr].min(z.c); hi[hr] = hi[hr].max(z.c); }
            if z.last { bk.exit(z.i, z.c, "end of day"); break; }
            if z.s <= open + win { continue; }
            if (d > 0.0 && z.c < stop) || (d < 0.0 && z.c > stop) { pend_exit = true; }
            if z.c > h2 && !used_l && d <= 0.0 { pend = 1.0; used_l = true; } else if z.c < l2 && !used_s && d >= 0.0 { pend = -1.0; used_s = true; }
        }
    }
    bk.out
}

/// Stochastic POP (ch.5, p.47-55; Appendix p.220-222): slow %K / %D (3-bar smoothed averages of the fast %K) on n-minute
/// bars; %K reaching `up` buys, falling under `lo` sells, at the next open; a %K/%D cross at a close exits at the next open;
/// out at the day's last close.
fn pop(x: &Ctx, from: usize, to: usize, n: u16, len: usize, up: f64, lo: f64) -> Vec<Trade> {
    let b = super::n1intra::series(x, from, to, n);
    let fast: Vec<f64> = (0..b.len()).map(|k| if k + 1 < len { f64::NAN } else {
        let w = &b[k + 1 - len..=k];
        let hh = w.iter().map(|z| z.h).fold(f64::NEG_INFINITY, f64::max);
        let ll = w.iter().map(|z| z.l).fold(f64::INFINITY, f64::min);
        if hh > ll { 100.0 * (b[k].c - ll) / (hh - ll) } else { 50.0 } }).collect();
    let sk = crate::connors::wilder(&fast, 3); // CQG smoothed average (Appendix p.221) = Wilder's, seeded with a plain mean
    let sd = crate::connors::wilder(&sk, 3);
    let mut bk = Book::default();
    let (mut pend_exit, mut pend) = (false, 0.0);
    for k in 0..b.len() {
        let z = &b[k];
        if pend_exit { bk.exit(z.i, z.o, "%K/%D cross"); pend_exit = false; }
        if pend != 0.0 { bk.enter(pend, z.i, z.o, "open", "reverse"); pend = 0.0; }
        if z.last { bk.exit(z.i, z.c, "end of day"); continue; }
        if k == 0 || !sd[k - 1].is_finite() { continue; }
        let mut d = bk.dir();
        if d != 0.0 && (sk[k] > sd[k]) != (sk[k - 1] > sd[k - 1]) { pend_exit = true; d = 0.0; }
        if sk[k] >= up && sk[k - 1] < up && d != 1.0 { pend = 1.0; } else if sk[k] < lo && sk[k - 1] >= lo && d != -1.0 { pend = -1.0; }
    }
    bk.out
}

/// MAC (ch.7, p.74-86): 10-bar average of highs / 8-bar average of lows on 5-minute bars; two successive bars wholly
/// outside the channel set the bias; with no position a limit at `lvl` of the channel (0.5 = midpoint) works through the next
/// bar; the opposite signal exits at the next open; out at the day's last close.
fn mac(x: &Ctx, from: usize, to: usize, nh: usize, nl: usize, lvl: f64) -> Vec<Trade> {
    use crate::intraday::{first_touch, Level, Touch};
    let b = super::n1intra::series(x, from, to, 5);
    // plain window sums (no running sum: over ~500k 5-minute bars its rounding drift moves the limit level by ~1e-7 and can flip
    // an exact touch of a minute's low, K55w/cmp_dump.txt)
    let avg = |v: Vec<f64>, n: usize| -> Vec<f64> { (0..v.len()).map(|k| if k + 1 < n { f64::NAN } else { v[k + 1 - n..=k].iter().sum::<f64>() / n as f64 }).collect() };
    let mah = avg(b.iter().map(|z| z.h).collect(), nh);
    let mal = avg(b.iter().map(|z| z.l).collect(), nl);
    let mut bk = Book::default();
    let (mut bias, mut up_run, mut dn_run, mut pend_exit, mut lim) = (0.0, 0usize, 0usize, false, None::<(f64, f64)>);
    for k in 0..b.len() {
        let z = &b[k];
        if pend_exit { bk.exit(z.i, z.o, "MAC reversing signal"); pend_exit = false; }
        if let Some((d, lv)) = lim.take() {
            // a buy limit is reached from above (touched when the low reaches it), a sell limit from below
            if let Touch::Hit { px, .. } = first_touch(&z.m, 0, &[Level { up: d < 0.0, px: lv }]) { bk.enter(d, z.i, px, "limit", "reverse"); }
        }
        // strictly outside the channel (jb624::mac's definition; a tie within float noise touches the line)
        let above = z.l - mah[k] > 1e-9 * mah[k].abs();
        let below = mal[k] - z.h > 1e-9 * mal[k].abs();
        up_run = if above { up_run + 1 } else { 0 };
        dn_run = if below { dn_run + 1 } else { 0 };
        if up_run >= 2 && bias != 1.0 { bias = 1.0; } else if dn_run >= 2 && bias != -1.0 { bias = -1.0; }
        if z.last { bk.exit(z.i, z.c, "end of day"); } else if bk.dir() != 0.0 && bk.dir() != bias { pend_exit = true; }
        if bias != 0.0 && mah[k].is_finite() && mal[k].is_finite() && (bk.dir() == 0.0 || pend_exit) {
            lim = Some((bias, if bias > 0.0 { mal[k] + lvl * (mah[k] - mal[k]) } else { mah[k] - lvl * (mah[k] - mal[k]) }));
        }
    }
    bk.out
}
// ---- end intraday EP block

#[cfg(test)]
mod k55_tests {
    use crate::connors::{Ctx, Params};
    fn minutes(name: &str, days: &[(&str, Vec<String>)]) -> &'static crate::intraday::Minutes {
        let dir = std::env::temp_dir().join(format!("k55{name}{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let (mut body, mut idx) = (String::from("Date,Time,Open,High,Low,Close\n"), String::from("Date,offset,bytes\n"));
        for (d, lines) in days {
            let start = body.len();
            for l in lines { body.push_str(&format!("{d},{l}\n")); }
            idx.push_str(&format!("{d},{start},{}\n", body.len() - start));
        }
        std::fs::write(dir.join("T.m1.csv"), &body).unwrap();
        std::fs::write(dir.join("T.m1idx.csv"), &idx).unwrap();
        Box::leak(Box::new(crate::intraday::Minutes::load(dir.to_str().unwrap(), "T").unwrap()))
    }
    fn line(m: u16, o: f64, h: f64, l: f64, c: f64) -> String { format!("{:02}:{:02},{o},{h},{l},{c}", m / 60, m % 60) }
    fn row(d: &str, o: f64, h: f64, l: f64, c: f64) -> crate::Row { crate::Row { ticker: "T".into(), date: d.into(), open: o, high: h, low: l, close: c, volume: None } }

    /// CTOD by hand: the first two hours close between 99 and 101 (H2 101, L2 99); the 12:00-12:05 bar closes at 102 -> buy at
    /// the next bar's open (102). The stop starts at 99 and trails to each completed hour's lowest close: 100.4 (10:30-11:30),
    /// 100.5 (11:30-12:30), 101.4 (12:30-13:30). The 13:50-13:55 bar closes 101.3 < 101.4 -> out at the next open, 101.2.
    #[test]
    fn ctod_breakout_and_hourly_trailing_closing_stop() {
        let px = |m: u16| -> f64 { let t = m - 570;
            if t < 60 { if t % 10 < 5 { 99.0 } else { 101.0 } } else if t < 120 { 100.0 + (t % 5) as f64 * 0.1 } else if t < 150 { 100.5 }
            else if t < 175 { 102.0 } else if t < 180 { 101.6 } else if t < 195 { 101.5 + (t % 5) as f64 * 0.01 } else if t < 200 { 101.4 }
            else if t < 260 { 101.45 } else if t < 265 { 101.3 } else { 101.2 } };
        let d1: Vec<String> = (570..960u16).map(|m| { let p = px(m); line(m, p, p + 0.01, p - 0.01, p) }).collect();
        let r = vec![row("2020-01-02", 1., 1., 1., 1.)];
        let mut x = Ctx::new(&r, &r, &[]);
        x.minutes = Some(minutes("ctod", &[("2020-01-02", d1)]));
        let t = super::run("jbcdt_ctod", &x, 0, 0, &Params::new()).unwrap();
        let v: Vec<(f64, f64, f64, String)> = t.iter().map(|z| (z.dir, z.legs[0].px, z.exit_px, z.reason.clone())).collect();
        assert_eq!(v, vec![(1.0, 102.0, 101.2, "closing stop".to_string())]);
    }

    /// Opening Gap by hand (10-minute bars, multi-unit): day 2 is in a daily bull trend and its first bar closes under day 1's
    /// low -> one unit at that close; the second bar places the swing buy at the first bar's high, which the third bar reaches
    /// -> a second unit; the StopPrice trail then exits both together.
    #[test]
    fn opengap_close_entry_then_second_unit_then_trail() {
        let r = vec![row("2020-01-02", 100., 101., 99., 100.5), row("2020-01-03", 98., 99., 96., 97.)];
        let mut x = Ctx::new(&r, &r, &[]);
        let flat = |a: u16, b: u16, p: f64| -> Vec<String> { (a..b).map(|m| line(m, p, p + 0.05, p - 0.05, p)).collect() };
        let mut d2 = flat(570, 580, 98.0); // bar 1 (9:30-9:40): high 98.05, close 98 < 99
        d2.extend(flat(580, 590, 97.5)); // bar 2: places buy stop at bar 1's high 98.05 for bar 3
        d2.extend(flat(590, 595, 97.6)); d2.extend(flat(595, 600, 98.3)); // bar 3 reaches 98.05: second unit at the minute's open 98.3
        d2.extend(flat(600, 610, 98.4)); d2.extend(flat(610, 960, 90.0)); // later bars fall through the trail
        x.minutes = Some(minutes("og", &[("2020-01-02", flat(570, 960, 100.5)), ("2020-01-03", d2)]));
        let t = crate::books::stad::run("stad2_opengap", &x, 0, 1, &Params::new()).unwrap();
        assert_eq!(t.len(), 1);
        assert_eq!(t[0].legs.iter().map(|l| (l.i, l.px)).collect::<Vec<_>>(), vec![(1, 98.0), (1, 98.3)]);
        assert!(t[0].dir == 1.0 && t[0].exit_px == 90.0 && t[0].reason == "StopPrice trail" && t[0].entry_kind == "close", "{:?}", t[0]);
    }

    /// The reference comparison (run by hand): VAULT_US=<us_data.csv> VAULT_IDX=<idx_data.csv> VAULT_M1_DIR=<data/intraday>
    /// K55_OUT=<csv> cargo test --release k55_dump -- --ignored; then authors/titles-1030/K55w/cmp_dump.py. One line per unit.
    #[test]
    #[ignore]
    fn k55_dump() {
        use std::io::Write;
        let mut f = std::io::BufWriter::new(std::fs::File::create(std::env::var("K55_OUT").unwrap()).unwrap());
        for (file, t) in [("VAULT_US", "SPY"), ("VAULT_US", "SPX"), ("VAULT_IDX", "NIFTY 50"), ("VAULT_IDX", "NIFTYBANK")] {
            let rows = crate::load_csv(&std::env::var(file).unwrap()).unwrap().remove(t).unwrap();
            let mut x = Ctx::new(&rows, &rows, &[]);
            x.minutes = Some(Box::leak(Box::new(crate::intraday::Minutes::load(&std::env::var("VAULT_M1_DIR").unwrap(), t).unwrap())));
            let from = rows.partition_point(|r| r.date.as_str() < x.minutes.unwrap().first());
            for id in crate::intraday_ep::K55 {
                for z in crate::connors::run(id, &x, from, rows.len() - 1, &Params::new()).unwrap() {
                    for l in &z.legs {
                        writeln!(f, "{id},{t},{},{},{},{},{},{},{}", z.dir, rows[l.i].date, l.px, rows[z.exit_i].date, z.exit_px, z.reason, z.open).unwrap();
                    }
                }
            }
            if t == "SPX" { // the prose's 2-point entry-bar stop option (S&P points: SPX only)
                let p = Params::from([("ebstop".to_string(), 2.0)]);
                for z in crate::connors::run("stad2_opengap", &x, from, rows.len() - 1, &p).unwrap() {
                    for l in &z.legs { writeln!(f, "stad2_opengap@eb2,{t},{},{},{},{},{},{},{}", z.dir, rows[l.i].date, l.px, rows[z.exit_i].date, z.exit_px, z.reason, z.open).unwrap(); }
                }
            }
        }
    }
}
