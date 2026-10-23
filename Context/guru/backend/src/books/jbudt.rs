//! Jacob Bernstein, "The Ultimate Day Trader: How to Achieve Consistent Day Trading Profits in Stocks, Forex,
//! and Commodities" (Adams Media / Simon & Schuster ebook, 2009). Book id `jbudt`.
//! Source copy: Books/Jake Bernstein/The_Ultimate_Day_Trader__How_to.pdf (reflowable ebook, no printed page
//! numbers; references are chapter + figure). Transcription and checks: library/jbudt/.
//! Uses the MAC / Williams A/D / stop helpers of `jb624` (same author, same definitions).
use super::jb624::{bar_path, finish, getp, mac, mark_open, new_trade, williams_ad};
use crate::connors::{sma, Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "jbudt";

macro_rules! def {
    ($id:expr, $name:expr, $group:expr, $dir:expr, $status:expr, $rules:expr, $notes:expr) => {
        StrategyDef { id: $id, name: $name, group: $group, session: 0, dir: $dir, status: $status, needs_volume: false,
            needs_vix: false, needs_series: &[], book: BOOK, rules: $rules, notes: $notes, claim: None }
    };
}

const MAC_RULES: [&str; 2] = [
    "F|\"The MAC uses two moving averages, one of the high for each price bar and one of the low. The input values of the moving averages are ten and eight periods respectively. The moving average (MA) is a simple, linear\" (ch.6)",
    "E|\"Two complete consecutive price bars above the MA high line is considered a buy pattern ... Two complete consecutive price bars below the 8 period MA of the low constitutes a sell pattern\" (ch.6, MAC Trading Method Applied)",
];

/// Daily rules admitted after the 2026-10-22 full reading (authors/bernstein/agents/jbudt) and an independent
/// chronological Python ledger check (authors/bernstein/g2-admission). The gap trades are held (no stop: one short
/// stays open for decades and loses more than the account). The rest are intraday, chart judgement or
/// left to per-market optimisation (held with the reason on the card).
pub const REVIEWED_DAILY: &[&str] = &["jbudt_mac_ad57", "jbudt_tb16", "jbudt_tb16_mom"];
pub fn reviewed_daily(id: &str) -> bool { REVIEWED_DAILY.contains(&id) }
/// The MAC / Williams A/D rule trades on the close only; the others read the open (gap setup, FPO, stop gaps).
pub fn reads_open(id: &str) -> bool { reviewed_daily(id) && id != "jbudt_mac_ad57" }

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::{Live, NeedsData, NotTestable};
    vec![
        def!("jbudt_gap_fpo", "Gap trade · exit on the first profitable opening", "Ch.5 · Gap Day Trading", "Long/Short", Live,
            &["F|\"Buy Set-Up: A gap buy setup occurs when the opening price today is lower than the low price yesterday. Sell Set-Up: ... the opening price today is higher than the high price yesterday\" (ch.5, Gap Set-Ups)",
              "E|\"A gap buy Trigger occurs when the price, after an opening drop, rises above the low of the previous day\"; \"a gap sell Trigger when the market price opens above the high of the previous day and then falls below the high of the previous day\" (ch.5, Gap Triggers): stop order at yesterday's low (high)",
              "X|\"Another method for exit is to hold the trade overnight and to exit on the first profitable opening (FPO). An FPO is simply an opening that is profitable based on your entry price\" (ch.5, First Profitable Opening Follow-Through)",
              "X|No stop is given for the FPO exit (\"Stops should be based on market behavior\", no level printed)"],
            "The end-of-day exit version of the same trade is already in the vault (ss_study_oops_buy / ss_study_oops_sell with p.exit=0), so only the FPO exit is added here. Book examples are stocks and futures; run on the indices (instrument swap). SPX opens before about 1999 often equal the prior close, so few gaps there."),
        def!("jbudt_gap_split", "Gap trade · losers out at the close, winners half at the close and half on the FPO", "Ch.5 · Gap Day Trading", "Long/Short", Live,
            &["F|Gap buy / sell setup as jbudt_gap_fpo (ch.5, Gap Set-Ups)",
              "E|Stop order at yesterday's low (buy) / high (sell) after the gap opening (ch.5, Gap Triggers)",
              "X|\"you would exit all shares if the trade is not profitable at the end of the day (EOD). On the other hand, if the trade is profitable you could exit half the position at the end of the day and keep the remainder of the position for an FPO exit\" (ch.5, First Profitable Opening Follow-Through)"],
            "Book example: 400 shares, half = 200. The half kept overnight has no printed stop; it leaves on the first opening beyond the entry price."),
        def!("jbudt_mac_ad57", "MAC with Williams A/D 57 confirmation (daily)", "Ch.6 · Moving Average Channel", "Long/Short", Live,
            &[MAC_RULES[0], MAC_RULES[1],
              "F|\"Some readers may be familiar with my MAC application on daily charts. In such cases I prefer to use ... Williams Accumulation/ Distribution (AD) with its 57 period moving average\" (ch.6)",
              "E|\"When the channel boundary is exceeded and the confirmation signal is giving or has already given the same signal, the trade is triggered\" (ch.6, Fig. 6.4 text): enter on the close of a MAC trigger bar when A/D is on the same side of its 57-bar MA",
              "X|\"The resulting trade can be closed out at support or resistance (i.e., the opposite signal)\" (ch.6): exit on the close of the opposite MAC trigger; it reverses only if A/D confirms the new side"],
            "This book never defines Williams A/D: the vault uses the price-only cumulative version of the 6-2-4 report (jb624, p.43), and reads its 57-period average as simple [vault reading]. A trigger = a change of MAC trend (two complete bars outside the channel). Entry on the trigger close [book silent on the order]. The intraday version with 28-period momentum is listed separately (needs 30-minute bars)."),
        def!("jbudt_tb16", "Trend breakout · new 16-bar high/low, stop and reverse", "Ch.13 · Trend Breakout", "Long/Short", Live,
            &["E|\"Buy if price makes a new 16-period high. Sell if price makes a new 16-period low\" (ch.13)",
              "E|\"Note the higher high and the lowest low of the last sixteen price bars. Place a buy stop one tick above the highest high. Please a sell stop below the lowest low of the last sixteen price bars\" (ch.13)",
              "X|\"Trail a stop loss or reverse your position on an opposite signal\" (ch.13): reverse on the opposite stop"],
            "Daily bars: the book shows the method on daily charts (Fig. 13.5-13.8, incl. S&P 500 daily) and on intraday charts. Last sixteen bars = the 16 completed bars before today. One tick = the instrument's real tick (0.05 NSE, 0.01 US; user rule 21 Oct 2026), or p.tick. If both stops are hit on one bar, the order follows the bar path (open, nearer extreme, other extreme, close)."),
        def!("jbudt_tb16_mom", "Trend breakout with momentum 14 / MA 14 filter", "Ch.13 · Trend Breakout", "Long/Short", Live,
            &["E|New 16-bar high / low with stop orders one tick beyond (ch.13)",
              "F|\"The trend filter I suggest is a 14-period moving average of the 14-period momentum. When the 14-period momentum crosses above its 14-period moving average the trend is up and we can buy on new buy triggers ... below ... we can only take the sell signals\" (ch.13, Filtering)",
              "X|\"Reverse your position on a new signal or if the relationship between momentum and its moving average changes\" (ch.13, Description of Method): reverse on the close when the filter flips (p.flip=0 = [old vault reading] exit flat instead, p.164); the opposite stop is armed only once the filter agrees"],
            "Momentum = close - close 14 bars ago (ch.9 definition); simple 14-bar MA of it. The book shows this filter only on intraday charts (10-, 30- and 240-minute); daily bars are a declared time-frame transfer. The filter state is read at the previous close when the stop is placed. Partial exits after the fifth bar and end-of-day exits (intraday) are alternatives, not coded."),
        def!("jbudt_mac_mom30", "MAC with momentum 28 / MA 28 confirmation (30-minute bars)", "Ch.6 · Moving Average Channel", "Long/Short", NeedsData("intraday 30-minute bars (the method is shown on 30-minute S&P futures with exit at the end of the day)"),
            &[MAC_RULES[0], MAC_RULES[1], "F|\"The trend is confirmed with the use of the 28 period Momentum indicator and its 28 period simple moving average\" (ch.6)", "X|\"Exit at the end of the day and/ or on a profit target\" (ch.6)"],
            "On daily charts the author swaps the confirmation for a 57-period MA (see jbudt_mac_ad57 and jbudt_mac_mom_daily)."),
        def!("jbudt_mac_mom_daily", "MAC with 57-period MA of momentum (daily)", "Ch.6 · Moving Average Channel", "Long/Short", NotTestable("the momentum length for the daily version is not given (\"a 57 period simple moving average of the MOM\")"),
            &[MAC_RULES[0], MAC_RULES[1], "F|\"In such cases I prefer to use a 57 period simple moving average of the MOM\" (ch.6)"],
            "The Williams A/D alternative in the same sentence is fully specified: jbudt_mac_ad57."),
        def!("jbudt_dow", "Day of week pattern (Friday-Monday)", "Ch.7 · Day Trading by the Day", "Long/Short", NotTestable("trigger offset and stop size are left to per-market optimisation (\"you can determine the best number of points by running an optimization program\"); the S&P values are printed in the 6-2-4 report: jb624_fri_mon"),
            &["F|\"If the closing price on the last trading day of the week is greater than the opening price, then the Set-Up is for a buy on Monday ... less than the opening price ... a sell\" (ch.7)",
              "E|\"For a Monday buy Set-Up, the Trigger is a buy stop rise of a given number of points on Monday above the high of Friday ... a sell stop price a given number of points below the low of Friday\" (ch.7)",
              "X|\"The Follow-Through consists of a stop loss and exit at the end of the day. Traders who are willing and able to carry positions overnight can do so\" (ch.7)"], ""),
        def!("jbudt_dow_stocks", "Day of week trades in GOOG and POT", "Ch.7 · Day Trading by the Day", "Long/Short", NeedsData("US single-stock daily OHLC (GOOG, POT); the vault has NSE stocks only"),
            &["E|GOOG Friday-Monday: \"Long positions buy stop 32 cents above the Friday high with stop loss of $4,700. Short positions sell stop 40 cents below Friday low with $3,800 stop loss\", 300 shares, FPO exit (Fig. 7.2)",
              "E|POT Friday-Monday: \"Buy trade triggers at Friday high plus 48 cents with a $2,700 stop loss and sell trades trigger at Friday low minus 48 cents with a $2,200 stop loss\", 300 shares, FPO (Fig. 7.3)",
              "E|GOOG Wednesday-Thursday: \"using 45 cents as the buy and sell trigger with $4,800 stop loss on longs and $3,400 stops on shorts and FPO exit\" (Fig. 7.4)"],
            "Printed: GOOG Fri-Mon PF 1.5, 75% winners, payout ratio .50, return 357% (Fig. 7.2); POT PF 1.89, 83.1% (Fig. 7.3); GOOG Wed-Thu 68.4% (Fig. 7.4). Test periods are not printed."),
        def!("jbudt_volspike", "Volume spike trigger", "Ch.8 · Volume Spikes", "Long/Short", NeedsData("intraday bars with volume (ten-minute chart preferred)"),
            &["F|\"we will define a VS as an increase of at least fourfold in volume over a period of a maximum of at least four price bars\" (ch.8)",
              "E|\"Isolate a volume spike on an intra-day chart. I prefer the ten-minute chart ... Buy on a bar close above the price high. Sell on a bar close below the price low\" (ch.8)",
              "X|\"Use the opposite side of the trade for a stop loss. Exit by the end of the session\" (ch.8)"], ""),
        def!("jbudt_divergence", "Momentum / MACD divergence (A-B-C-D-E)", "Ch.9-11 · Divergence", "Long/Short", NotTestable("chart judgement: \"divergence is not a trading system. It is, rather, a trading methodology incorporating some subjectivity\" (ch.10)"),
            &["F|28-period momentum and the one-line MACD 9/18 (ch.9, ch.11)", "E|\"A buy signal is generated when ... a new closing Momentum high is created that is higher than the highest Momentum during the period of bullish convergence\" (ch.10); three-month window, at least six bars (ch.10)",
              "X|\"The first target, P1, was one half the range of the highest price high and lowest price low during the time frame from A to D inclusive\" (ch.11)"],
            "The 6-2-4 report gives a cookbook version of the same setup (jb624_pmf), also not testable."),
        def!("jbudt_pop", "Stochastic POP day trading method (9, 3-bar smoothing, 70/30)", "Ch.12 · Stochastic POP", "Long/Short", NeedsData("intraday bars (a day-trading method with exit by the end of the day)"),
            &["F|\"Use a 9-period slow stochastic %K only with 3-bar smoothing. Use upper and lower boundaries of 30 percent and 70 percent\" (ch.12)",
              "E|\"When %K crosses above 70 percent on the close of any bar, then buy on close of that bar. When %K crosses below 30 percent on the close of any bar then sell short on the close of that bar\" (ch.12)",
              "X|\"Exit long when %K crosses back below 70 percent. Exit short when %K crosses back above 30 percent\"; \"Exit trades by the end of the day\" (ch.12)"],
            "The daily-bar POP of the 6-2-4 report (14 / 5, 75/25) is live as jb624_pop."),
        def!("jbudt_news", "Trading the news (15-minute triggers)", "Ch.14 · News-Driven Market", "Long/Short", NotTestable("the setup is a news event judged by the trader, and the triggers use 15-minute bars"),
            &["E|\"After a sharply lower opening based on news, buy the stock if it ends any [fifteen-minute bar] above ...\" (ch.14)"], ""),
        def!("jbudt_stf_example", "STF example: momentum zero line + 30-minute trigger", "Ch.4 · STF Framework", "Long/Short", NotTestable("the author presents it as \"merely an example and not specifically recommended as a trading strategy\"; it also needs 30-minute bars"),
            &["E|\"When Momentum rises above zero, it sets the market up for a move to the upside ... The Trigger to confirm a day trade buy will be a thirty-minute ending price above the high of the last thirty-minute price bar\" (ch.4)"], ""),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "jbudt_gap_fpo" => ("jbudt_gap", "Gap day trading", "First profitable opening exit"),
        "jbudt_gap_split" => ("jbudt_gap", "Gap day trading", "EOD if losing, half EOD + half FPO if winning"),
        "jbudt_mac_ad57" => ("jbudt_mac", "Moving Average Channel (Ultimate Day Trader)", "Daily · Williams A/D 57"),
        "jbudt_mac_mom30" => ("jbudt_mac", "Moving Average Channel (Ultimate Day Trader)", "30-minute · MOM 28 (needs data)"),
        "jbudt_mac_mom_daily" => ("jbudt_mac", "Moving Average Channel (Ultimate Day Trader)", "Daily · MOM/MA 57 (not testable)"),
        "jbudt_tb16" => ("jbudt_tb", "Trend breakout (16-bar)", "Stop and reverse"),
        "jbudt_tb16_mom" => ("jbudt_tb", "Trend breakout (16-bar)", "Momentum 14 / MA 14 filter"),
        "jbudt_dow" => ("jbudt_dow", "Day of week pattern", "S&P (not testable here)"),
        "jbudt_dow_stocks" => ("jbudt_dow", "Day of week pattern", "GOOG / POT (needs data)"),
        "jbudt_volspike" => ("jbudt_volspike", "Volume spikes", "Needs intraday"),
        "jbudt_divergence" => ("jbudt_divergence", "Momentum divergence", "Not testable"),
        "jbudt_pop" => ("jbudt_pop", "Stochastic POP (day trading)", "Needs intraday"),
        "jbudt_news" => ("jbudt_news", "News trading", "Not testable"),
        "jbudt_stf_example" => ("jbudt_stf", "STF example", "Not testable"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(match id {
        "jbudt_gap_fpo" | "jbudt_gap_split" => vec![],
        "jbudt_mac_ad57" => vec![p("mah", "MA of highs (bars)", 10., 5., 20., 1.), p("mal", "MA of lows (bars)", 8., 4., 20., 1.), p("bars", "Consecutive complete bars", 2., 1., 4., 1.), p("ad_ma", "MA of Williams A/D (bars)", 57., 20., 80., 1.)],
        "jbudt_tb16" => vec![p("n", "Breakout lookback (bars)", 16., 5., 50., 1.), p("tick", "One tick (-1 = the instrument's real tick: 0.05 on NSE indices and Indian stocks, 0.01 on US indices/ETFs/stocks)", -1., -1., 1., 0.01)],
        "jbudt_tb16_mom" => vec![p("n", "Breakout lookback (bars)", 16., 5., 50., 1.), p("tick", "One tick (-1 = the instrument's real tick: 0.05 on NSE indices and Indian stocks, 0.01 on US indices/ETFs/stocks)", -1., -1., 1., 0.01), p("mom", "Momentum length", 14., 5., 40., 1.), p("ma", "MA of momentum (bars)", 14., 5., 40., 1.),
            p("flip", "When momentum crosses its MA: 1 = reverse on the close (book, p.164), 0 = exit flat (earlier vault reading)", 1., 0., 1., 1.)],
        _ => return None,
    })
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !id.starts_with("jbudt_") || params(id).is_none() || to >= x.c.len() || from > to {
        return None;
    }
    let g = |k: &str| getp(id, p, k);
    let u = |k: &str| g(k).round().max(1.0) as usize;
    let mut out = Vec::new();
    let mut cur: Option<Trade> = None;
    match id {
        "jbudt_gap_fpo" | "jbudt_gap_split" => {
            let split = id == "jbudt_gap_split";
            for i in from.max(1)..=to {
                if let Some(t) = cur.as_mut().filter(|t| t.legs[0].i < i) {
                    if (x.o[i] - t.legs[0].px) * t.dir > 0.0 {
                        let mut t = cur.take().unwrap();
                        let px = if split { t.exits.push((i, x.o[i], 0.5)); t.exits.iter().map(|e| e.1 * e.2).sum() } else { x.o[i] };
                        out.push(finish(t, i, px, i, "First profitable opening"));
                    }
                }
                if cur.is_some() || i == to {
                    continue;
                }
                let d = if x.o[i] < x.l[i - 1] && x.h[i] >= x.l[i - 1] { 1.0 } else if x.o[i] > x.h[i - 1] && x.l[i] <= x.h[i - 1] { -1.0 } else { continue };
                let e = if d > 0.0 { x.l[i - 1] } else { x.h[i - 1] };
                let mut t = new_trade(d, i, e, "stop", i - 1);
                if split {
                    if (x.c[i] - e) * d > 0.0 {
                        t.exits.push((i, x.c[i], 0.5));
                    } else {
                        out.push(finish(t, i, x.c[i], i, "Not profitable at the end of the day"));
                        continue;
                    }
                }
                cur = Some(t);
            }
        }
        "jbudt_mac_ad57" => {
            let m = mac(x, u("mah"), u("mal"), u("bars"));
            let ad = williams_ad(x);
            let ma = sma(&ad, u("ad_ma"));
            for i in from..=to {
                let w = m.trig[i];
                if w == 0 {
                    continue;
                }
                if let Some(t) = cur.as_ref() {
                    if t.dir as i8 == w {
                        continue;
                    }
                    out.push(finish(cur.take().unwrap(), i, x.c[i], i, "Opposite MAC signal"));
                }
                let conf = if ad[i] > ma[i] { 1 } else if ad[i] < ma[i] { -1 } else { 0 };
                if conf == w && i < to {
                    cur = Some(new_trade(w as f64, i, x.c[i], "close", i));
                }
            }
        }
        "jbudt_tb16" | "jbudt_tb16_mom" => {
            let (n, tick) = (u("n"), match g("tick") { t if t < 0.0 => x.market_tick(), t => t });
            let filt: Vec<i8> = if id == "jbudt_tb16_mom" {
                let k = u("mom");
                let mom: Vec<f64> = (0..x.c.len()).map(|i| if i >= k { x.c[i] - x.c[i - k] } else { f64::NAN }).collect();
                let ma = sma(&mom, u("ma"));
                let mut st = 0i8;
                (0..x.c.len()).map(|i| { if mom[i] > ma[i] { st = 1 } else if mom[i] < ma[i] { st = -1 } st }).collect()
            } else {
                vec![2; x.c.len()] // 2 = no filter
            };
            for i in from.max(n)..=to {
                let hh = x.h[i - n..i].iter().cloned().fold(f64::MIN, f64::max) + tick;
                let ll = x.l[i - n..i].iter().cloned().fold(f64::MAX, f64::min) - tick;
                let f = filt[i - 1];
                let (buy, sell) = (if f == 2 || f == 1 { hh } else { f64::INFINITY }, if f == 2 || f == -1 { ll } else { f64::NEG_INFINITY });
                // walk the bar path: open, nearer extreme, other extreme, close
                let path = bar_path(x, i);
                let mut side = cur.as_ref().map_or(0i8, |t| t.dir as i8);
                let mut fills: Vec<(i8, f64)> = Vec::new();
                for k in 0..4 {
                    let (a, b) = if k == 0 { (path[0], path[0]) } else { (path[k - 1], path[k]) };
                    if side < 1 && b >= buy && (k == 0 || a < buy) {
                        fills.push((1, if k == 0 { a } else { buy }));
                        side = 1;
                    } else if side > -1 && b <= sell && (k == 0 || a > sell) {
                        fills.push((-1, if k == 0 { a } else { sell }));
                        side = -1;
                    }
                }
                if i == to && cur.is_none() {
                    break;
                }
                for (d, px) in fills {
                    if let Some(t) = cur.take() {
                        out.push(finish(t, i, px, i, "Opposite 16-bar breakout (reverse)"));
                    }
                    if i < to {
                        cur = Some(new_trade(d as f64, i, px, "stop", i - 1));
                    }
                }
                if f != 2 {
                    if let Some(t) = cur.as_ref() {
                        if filt[i] as f64 == -t.dir {
                            let d = -t.dir;
                            out.push(finish(cur.take().unwrap(), i, x.c[i], i, "Momentum / MA relationship changed"));
                            if g("flip") >= 0.5 && i < to {
                                cur = Some(new_trade(d, i, x.c[i], "close", i));
                            }
                        }
                    }
                }
            }
        }
        _ => return Some(vec![]),
    }
    mark_open(&mut out, cur, x, to);
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn row(d: &str, o: f64, h: f64, l: f64, c: f64) -> Row {
        Row { ticker: "T".into(), date: d.into(), open: o, high: h, low: l, close: c, volume: None }
    }
    #[test]
    fn gap_fpo_hand_checked() {
        let r = vec![
            row("2009-01-05", 100., 102., 99., 101.),
            row("2009-01-06", 98., 100., 97., 99.5), // opens below 99, trades back to 99 -> long at 99
            row("2009-01-07", 98.5, 99.5, 98., 99.), // open 98.5 not profitable
            row("2009-01-08", 99.2, 100., 99., 99.8), // open 99.2 > 99 -> exit
        ];
        let x = Ctx::new(&r, &r, &[]);
        let ts = run("jbudt_gap_fpo", &x, 0, 3, &Params::new()).unwrap();
        assert_eq!(ts.len(), 1);
        assert_eq!((ts[0].legs[0].i, ts[0].legs[0].px, ts[0].exit_i, ts[0].exit_px), (1, 99., 3, 99.2));
    }
}
