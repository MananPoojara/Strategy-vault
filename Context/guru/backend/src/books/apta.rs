//! Gerald Appel, Technical Analysis: Power Tools for Active Investors (FT Prentice Hall, 2005).
//! Only the fully specified timing models are Live; chart reading, cycles, T-formations and
//! regime judgement are NotTestable; models needing data the vault lacks are NeedsData.
//! Page refs are printed pages (PDF page = printed + 24). Sources and checks: library/apta/.
use crate::connors::{sma, Claim, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

/// Admitted 2026-10-24 (authors/appel/admission/SOURCE-CONTRACT.md): independent ledgers, US and India, 0 mismatches.
pub const REVIEWED: &[&str] = &["apta_ma200_dji", "apta_ma100_dji", "apta_triple_mom", "apta_roc15", "apta_six_best", "apta_six_worst", "apta_breadth_impulse", "apta_breadth_impulse_nse", "apta_breadth_macd", "apta_breadth_macd_nse", "apta_vix35", "apta_macd"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

pub const BOOK: &str = "apta";

// ------------------------------------------------------------------ shared helpers (also used by apmacd)

/// Exponential average with smoothing constant 2/(n+1), started from the simple average of the first
/// n finite values (Appel's "preferable alternative", p.136; TAG96 booklet p.33). NaN inputs keep the
/// previous value; NaN until seeded.
pub fn ema(v: &[f64], n: usize) -> Vec<f64> {
    let k = 2.0 / (n as f64 + 1.0);
    let (mut out, mut e, mut cnt, mut sum) = (vec![f64::NAN; v.len()], f64::NAN, 0usize, 0.0);
    for (i, &x) in v.iter().enumerate() {
        if x.is_finite() {
            if cnt < n {
                cnt += 1;
                sum += x;
                if cnt == n {
                    e = sum / n as f64;
                }
            } else {
                e += k * (x - e);
            }
        }
        out[i] = e;
    }
    out
}

/// Same smoothing, started from the first finite value (the book's worked worksheet, p.144).
/// NaN days (no breadth data) stay NaN and leave the average unchanged.
pub fn ema_first(v: &[f64], n: usize) -> Vec<f64> {
    let k = 2.0 / (n as f64 + 1.0);
    let mut e = f64::NAN;
    v.iter()
        .map(|&x| {
            if !x.is_finite() {
                return f64::NAN;
            }
            e = if e.is_finite() { e + k * (x - e) } else { x };
            e
        })
        .collect()
}

fn ymd(s: &str) -> (i64, u32, u32) {
    (s[0..4].parse().unwrap_or(0), s[5..7].parse().unwrap_or(0), s[8..10].parse().unwrap_or(0))
}

/// Days since 1970-01-01 (Howard Hinnant's days_from_civil).
fn days(s: &str) -> i64 {
    let (y, m, d) = ymd(s);
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m as i64 + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d as i64 - 1;
    era * 146097 + yoe * 365 + yoe / 4 - yoe / 100 + doy - 719468
}

/// Bars the rules are evaluated on: tf 0 = every day, 1 = last bar of each Monday-Friday week,
/// 2 = last bar of each month. The final bar of the data counts as a week end only on a Friday
/// (an unfinished week/month is not a weekly/monthly close).
pub fn bars(x: &Ctx, tf: u8) -> Vec<usize> {
    let n = x.r.len();
    let key = |i: usize| -> i64 {
        let d = &x.r[i].date;
        match tf {
            1 => (days(d) + 3).div_euclid(7), // 1970-01-01 was a Thursday; weeks start Monday
            _ => {
                let (y, m, _) = ymd(d);
                y * 12 + m as i64
            }
        }
    };
    if tf == 0 {
        return (0..n).collect();
    }
    (0..n)
        .filter(|&i| if i + 1 < n { key(i + 1) != key(i) } else { tf == 1 && (days(&x.r[i].date) + 3).rem_euclid(7) == 4 })
        .collect()
}

fn mk(i: usize, px: f64) -> Trade {
    Trade { dir: 1.0, legs: vec![Leg { i, px, w: 1.0 }], exit_i: i, exit_px: px, reason: String::new(),
        entry_kind: "close", signal_i: i, open: false, exits: vec![] }
}

/// Long-only, one position, every fill at the close of the signal bar. `step(k, i, holding)` returns
/// Some(reason) to buy (flat) or sell (holding) on bar idx[k] = i. The rule state runs from the first
/// bar so signals never depend on the chosen start; a position the rules hold at `from` is not
/// reported (flat at the start) and entries on the last bar are not taken.
pub fn run_long(x: &Ctx, idx: &[usize], from: usize, to: usize, mut step: impl FnMut(usize, usize, bool) -> Option<String>) -> Vec<Trade> {
    let (mut out, mut cur, mut virt) = (vec![], None::<Trade>, false);
    for (k, &i) in idx.iter().enumerate() {
        if i > to {
            break;
        }
        let holding = virt || cur.is_some();
        if let Some(why) = step(k, i, holding) {
            if holding {
                if let Some(mut t) = cur.take() {
                    t.exit_i = i;
                    t.exit_px = x.c[i];
                    t.signal_i = i;
                    t.reason = why;
                    out.push(t);
                }
                virt = false;
            } else if i >= from && i < to {
                cur = Some(mk(i, x.c[i]));
            } else {
                virt = true;
            }
        }
    }
    if let Some(mut t) = cur {
        t.exit_i = to;
        t.exit_px = x.c[to];
        t.signal_i = to;
        t.reason = "open".into();
        t.open = true;
        out.push(t);
    }
    out
}

/// Appel's MACD buy/sell procedure (TA ch.8 p.169-196; TAG96 booklet p.34-35), one rule set applied
/// throughout (the choice of rule set by market trend is judgement and is left to the caller).
pub struct Macd {
    pub buy: (usize, usize, usize),  // fast EMA, slow EMA, signal EMA of the buying MACD
    pub sell: (usize, usize, usize), // same for the selling MACD
    pub both_above: bool,            // sell needs BOTH lines above 0 since the buy (booklet) instead of the selling line only (TA)
    pub sell_on_buy: bool,           // declining-trend rule: sell on the buying MACD
    pub stop: bool,                  // MACD stop: buying MACD below the low that immediately preceded the buy
    pub tf: u8,
}

pub fn macd_run(x: &Ctx, from: usize, to: usize, m: &Macd) -> Vec<Trade> {
    let idx = bars(x, m.tf);
    let cl: Vec<f64> = idx.iter().map(|&i| x.c[i]).collect();
    let line = |(f, s, g): (usize, usize, usize)| {
        let (a, b) = (ema(&cl, f), ema(&cl, s));
        let l: Vec<f64> = a.iter().zip(&b).map(|(a, b)| a - b).collect();
        let sg = ema(&l, g);
        (l, sg)
    };
    let ((bm, bs), (sm, ss)) = (line(m.buy), line(m.sell));
    let (mut below0, mut ok_s, mut ok_b, mut run_min, mut trough) = (false, false, false, f64::INFINITY, f64::NAN);
    let cross_up = |l: &[f64], s: &[f64], k: usize| k > 0 && l[k - 1] <= s[k - 1] && l[k] > s[k];
    let cross_dn = |l: &[f64], s: &[f64], k: usize| k > 0 && l[k - 1] >= s[k - 1] && l[k] < s[k];
    run_long(x, &idx, from, to, |k, _, holding| {
        if !(bm[k].is_finite() && bs[k].is_finite() && sm[k].is_finite() && ss[k].is_finite()) || k == 0 || !bs[k - 1].is_finite() || !ss[k - 1].is_finite() {
            return None;
        }
        // lowest buying-MACD level of the latest stretch below its signal line = "the low that immediately preceded the buy"
        if bm[k] <= bs[k] {
            run_min = if bm[k - 1] > bs[k - 1] { bm[k] } else { run_min.min(bm[k]) };
        }
        if !holding {
            if bm[k] < 0.0 {
                below0 = true;
            }
            if below0 && cross_up(&bm, &bs, k) {
                trough = run_min;
                ok_s = sm[k] > 0.0;
                ok_b = bm[k] > 0.0;
                return Some("buy".into());
            }
            return None;
        }
        ok_s |= sm[k] > 0.0;
        ok_b |= bm[k] > 0.0;
        let why = if m.stop && bm[k] < trough {
            Some("MACD stop: buying MACD below its pre-buy low")
        } else if m.sell_on_buy {
            (ok_b && cross_dn(&bm, &bs, k)).then_some("buying MACD crossed below its signal line")
        } else {
            (ok_s && (!m.both_above || ok_b) && cross_dn(&sm, &ss, k)).then_some("selling MACD crossed below its signal line")
        };
        why.map(|w| {
            below0 = false;
            w.to_string()
        })
    })
}

// ------------------------------------------------------------------ catalogue

const NYSE: &[&str] = &["NYSE_ADV_DEC.Advances", "NYSE_ADV_DEC.Declines"];
const NSE: &[&str] = &["NSE_ADV_DEC.Advances", "NSE_ADV_DEC.Declines"];

fn breadth<'a>(x: &'a Ctx, nse: bool) -> Option<(&'a [f64], &'a [f64])> {
    let s = if nse { NSE } else { NYSE };
    Some((x.series(s[0])?, x.series(s[1])?))
}

const IMPULSE: &[&str] = &[
    "F|\"Divide the number of issues that advance each day by the sum of the number of issues that advance and the number that decline\" (p.143)",
    "F|\"Maintain a ten-day exponential average of the daily ratios ... smoothing constant of .1818\" (p.143); started from the first day's ratio as in the worksheet (p.144)",
    "E|\"The capability of the ten-day exponential average of daily ratios to rise to .615 or higher represents a stock market buy signal.\" (p.143) Buy on that day's close.",
    "X|\"This buy signal remains in effect until the ten-day exponential average of the daily ratios declines to .490 or below\" (p.143). Sell on that day's close.",
];
const IMPULSE_MACD: &[&str] = &[
    "E|\"As before, you buy when the 10-day exponential average of the ratio, advancing issues/advancing + declining issues, rises to 61.5% or higher. There are no other buy entries.\" (p.197)",
    "X|\"you sell when the 10-day exponential average ... falls to 49% or lower. Providing That... The 19- to 39-day MACD of prices ... must not lie above 1%. Both conditions must exist simultaneously\" (p.197)",
    "F|MACD % = (19-day EMA - 39-day EMA) / 19-day EMA x 100 (p.197 text). The worked example divides by the 39-day EMA (100 / 6,400 = 1.56%); p.denom=39 selects the example's reading.",
    "F|Breadth average as in the Daily-Based Breadth Impulse Signal (p.143-144). The MACD is taken on the traded index (book: NYSE Index prices).",
];
const MACD_TA: &[&str] = &[
    "F|Buying MACD = 12-day EMA - 26-day EMA of closes with a 9-day EMA signal line; selling MACD = 19-day EMA - 39-day EMA with a 9-day signal line (\"Use the 12-26 combination for buying. Use the 19-39 combination for selling\", neutral to somewhat positive trends, p.175; signal lines p.174-175)",
    "E|Buy when the buying MACD crosses from below to above its signal line (p.169), provided it has been below 0 at some time since the most recent sell signal (p.170, p.196). Buy on the close.",
    "X|Sell when the selling MACD crosses from above to below its signal line, provided it has been above 0 at some time since the buy (\"the MACD line that you are using for selling must have crossed above 0 before generating a valid sell signal\", p.174; p.170, p.196). Sell on the close.",
    "X|\"Sell stops also take place if, following a purchase, the buying MACD level falls to below its previous low point, even if the area above 0 has not been reached.\" (p.196) Previous low = lowest buying-MACD value of the stretch below its signal line that ended with the buy.",
    "F|Exponential averages started from a simple average (p.136). p.tf=1/2 runs the same rules on weekly/monthly closes (p.186-189).",
];

fn c(t: &'static str, w: &'static str, a: &'static str, h: &'static str, p: &'static str) -> Option<Claim> {
    Some(Claim { trades: t, win: w, avg: a, hold: h, period: p })
}

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, status, series: &'static [&'static str], vix, rules, notes, claim| StrategyDef {
        id, name, group, session: 0, dir: "Long", status, needs_volume: false, needs_vix: vix, needs_series: series,
        book: BOOK, rules, notes, claim,
    };
    use Status::*;
    vec![
        // ---- Ch 3: moving averages, rates of change
        d("apta_ma200_dji", "Dow 200-day moving average crossings", "Ch 3 · Moving averages: myths and misconceptions", Live, &[], false,
            &["E|\"buying the Dow Industrials when its daily close exceeds its 200-day moving average\" (p.48). Buy on the close.",
              "X|\"and selling at the close of days when its close declines to below the 200-day moving average\" (p.48)",
              "F|Simple moving average of daily closes (Ch 3 uses simple averages unless stated)."],
            "Book tests the Dow Industrials 1/5/1970-1/13/2004 (p.48) to show crossings add little; run on DJI/SPX/NIFTY. Not a recommendation in the book.",
            c("120", "21.7%", "+14.1% winners / -1.2% losers", "68.6% invested", "Dow 1/5/1970-1/13/2004")),
        d("apta_ma100_dji", "Dow 100-day moving average crossings", "Ch 3 · Moving averages: myths and misconceptions", Live, &[], false,
            &["E|\"The second strategy employs the same rules but is applied to crossings of the 100-day moving average.\" (p.48) Buy on the close above the 100-day SMA.",
              "X|Sell at the close of a day that closes below the 100-day SMA (p.48)."],
            "Same test as the 200-day model (p.48).",
            c("195", "22.6%", "+18.7% winners / -1.1% losers", "65.5% invested", "Dow 1/5/1970-1/13/2004")),
        d("apta_triple_mom", "Triple Momentum Nasdaq Index Trading Model", "Ch 3 · Triple Momentum", Live, &[], false,
            &["F|\"a 5-day rate of change of the daily closing prices of the Nasdaq Composite, a 15-day rate of change measurement, and a 25-day rate of change measurement ... maintained as percentage changes\" (p.59); ROC = (close / close n days ago - 1) x 100",
              "F|\"add the percentage-based levels for the 5-day, 15-day, and 25-day rates of change ... the Triple Momentum figure for the day\" (p.59)",
              "E|\"You buy when the Triple Momentum Level ... crosses from below to above 4%.\" (p.60) Buy on the close.",
              "X|\"You sell when the Triple Momentum Level ... crosses from above to below 4%.\" (p.60) \"there are no further rules\" (p.60)"],
            "Book index: Nasdaq Composite 1972-May 2004 (not in the vault: run on NDX from 1990, SPX, NIFTY). Year-by-year table p.60-61.",
            c("288 (8.9/yr)", "54.4%", "+4.6% winners / -0.9% losers", "45.9% invested", "Nasdaq Composite 1972-5/5/2004")),
        d("apta_roc15", "15-day rate of change zero crossings", "Ch 3 · Triple Momentum (single-ROC comparison)", Live, &[], false,
            &["E|\"if the Nasdaq Composite was purchased on days that the 15-day rate of change alone rose from below to above 0\" (p.61). Buy on the close.",
              "X|\"and sold on days that it fell below 0\" (p.61)"],
            "Comparison case printed next to Triple Momentum (p.61).",
            c("307", "44.3%", "", "", "Nasdaq Composite 1972-2004 (+18.3%/yr, max DD -28.6%)")),
        // ---- Ch 5: seasonal
        d("apta_six_best", "Best six months (October close to May 4)", "Ch 5 · The best six-month period", Live, &[], false,
            &["E|\"Enter Market the Last Trading Day of October\" (Table 5.2, p.92). Buy on that day's close.",
              "X|\"Exit Closest Trading Day Before or on May 4\" (Table 5.2, p.92). Sell on that day's close."],
            "Table 5.2: S&P 500 excluding dividends, 1969-May 2004. Book credits Hirsch / Sy Harding (p.93). Similar idea to kftsm_hirsch (Nov 1-Apr 30), different dates.",
            c("35", "80.0%", "+7.48%", "", "S&P 500 1969-May 2004 (6.91%/yr compounded)")),
        d("apta_six_worst", "Worst six months (May 4 to October)", "Ch 5 · The worst six-month period", Live, &[], false,
            &["E|\"buying on the closing date closest to May 4\" (p.92); taken as the same day Table 5.2 exits (last trading day on or before May 4).",
              "X|\"and selling on the next-to-last trading day of October\" (p.92). Sell on the close."],
            "Table 5.3 (p.92), shown as the contrast to Table 5.2.",
            c("35", "62.9%", "+0.7%", "", "S&P 500 1970-Sep 2004 (0.3%/yr compounded)")),
        d("apta_turn_of_month", "Turn of the month / pre-holiday", "Ch 5 · Calendar-based cycles", NotTestable("no exact rule: \"the last and next-to-last trading days of each month and the first two to four trading days ... (the exact combinations have varied somewhat over the years)\" (p.90)"), &[], false,
            &["E|Greatest gains \"during the last and next-to-last trading days of each month and the first two to four trading days of subsequent months\" (p.90)", "E|Above-average strength the days before holidays and three days after (p.90)"],
            "Observation, no fixed entry/exit days.", None),
        d("apta_presidential", "Presidential election cycle", "Ch 5 · The presidential stock market cycle", NotTestable("no trading rule: Table 5.5 gives average gains by year of the cycle only; \"some portfolio managers\" hold only in favourable years (p.94) without dates"), &[], false,
            &["F|Pre-election years best, election years second, post-election and midterm years weakest (p.93-94, Table 5.5)"], "Statistics only.", None),
        d("apta_cycles_tform", "Time cycles and T-formations", "Ch 5 · Time cycles, T-formations", NotTestable("discretionary: cycle lows, A-B segments and T-formation centres are chosen by eye (\"something of a judgment call\", p.109)"), &[], false,
            &["F|T-formation: distance A-X equals distance X-B (p.108-109)"], "Chart reading.", None),
        // ---- Ch 2: mood indicators
        d("apta_nasdaq_nyse_rs", "Nasdaq/NYSE Index relative strength", "Ch 2 · Market mood indicators", NeedsData("weekly closes of the Nasdaq Composite and the NYSE Composite Index (neither is in the vault; NDX/SPX are different indices)"), &[], false,
            &["F|Weekly Nasdaq Composite close / NYSE Index close; ten-week simple average of the ratio (p.31)", "E|Ratio above its ten-week average = Nasdaq leads = invested (p.31)", "X|Ratio below its average = out; trades at the week's close (Table 2.1, p.32)"],
            "Needs series NASDAQ_COMP (weekly close) and NYSE_COMP (weekly close).",
            c("130", "54.6%", "+3.4%", "54.9% invested", "Nasdaq Composite 4/8/1971-12/12/2003")),
        d("apta_monetary", "Intermediate Monetary Filter (3- to 5-year note yields)", "Ch 2 · Market mood indicators", NeedsData("weekly yields of the 3-year and 5-year US Treasury notes (not in the vault)"), &[], false,
            &["F|Average the weekly 3-year and 5-year Treasury note yields (p.37)", "E|Average lower than 33 weeks earlier = favourable = invested (p.37)", "X|Average higher than 33 weeks earlier = out (p.37)"],
            "Needs series UST3Y, UST5Y (weekly). Combined with the RS indicator: 88 trades, 61.4% (p.39).",
            c("36", "63.9%", "+13.2%", "52.7% invested", "Nasdaq Composite 4/8/1971-12/12/2003")),
        d("apta_rs_funds", "Relative strength mutual fund selection", "Ch 1 · No-frills investment strategy", NeedsData("quarterly returns and volatility rankings of a mutual fund universe (not in the vault)"), &[], false,
            &["F|Rank funds of similar volatility by quarterly performance (p.18-19)", "E|Buy several funds in the top decile; rerank quarterly (p.18-21)"], "Fund universe needed.", None),
        // ---- Ch 6: breadth
        d("apta_breadth_impulse", "Daily-Based Breadth Impulse Signal · NYSE breadth", "Ch 6 · Daily-Based Breadth Impulse", Live, NYSE, false, IMPULSE,
            "NYSE advances/declines (extra_data/us, 1965-2020) drive the signal; any index can be held (book: NYSE Index, Nasdaq Composite). Trade list p.144-145.",
            c("26", "84.6%", "+5.24% winners / -2.37% losers", "7.6% invested", "NYSE Index 12/1970-6/14/2004")),
        d("apta_breadth_impulse_nse", "Daily-Based Breadth Impulse Signal · NSE breadth", "Ch 6 · Daily-Based Breadth Impulse", Live, NSE, false, IMPULSE,
            "India mapping: NSE advances/declines instead of NYSE (extra_data/india, 1999-), same thresholds.", None),
        d("apta_breadth_macd", "MACD-filtered Breadth Thrust · NYSE breadth", "Ch 8 · Converting the Daily Breadth Thrust Model", Live, NYSE, false, IMPULSE_MACD,
            "Trade list Table 8.1 (p.197-198, NYSE Index) and 8.2 (p.199, Nasdaq).",
            c("24", "75%", "+9.4% winners / -2.6% losers", "18.3% invested", "NYSE Index 1970-2004")),
        d("apta_breadth_macd_nse", "MACD-filtered Breadth Thrust · NSE breadth", "Ch 8 · Converting the Daily Breadth Thrust Model", Live, NSE, false, IMPULSE_MACD,
            "India mapping: NSE advances/declines, MACD of the traded Indian index.", None),
        d("apta_nhnl", "New highs / (new highs + new lows) 90% model", "Ch 6 · New high/new low indicator", NeedsData("daily NYSE new 52-week highs and new lows (not in extra_data)"), &[], false,
            &["F|Ten-day simple average of NH/(NH+NL) (p.124)", "E|\"Buy ... when the ten-day average ... reaches 90%\" (p.124)", "X|Sell when it falls below 90/85/80/75/70% (choice; Table 6.1 p.125)"],
            "Needs series NYSE_NH, NYSE_NL.", c("55 (sell <85%)", "49.1%", "", "21.0% invested", "S&P 500 1970-2004")),
        d("apta_weekly_impulse", "Weekly Impulse Continuation Signal", "Ch 6 · Weekly Impulse Signal", NeedsData("weekly NYSE advances/declines measured week-close to week-close and issues traded (not a sum of daily data; not in extra_data)"), &[], false,
            &["F|(weekly advances - declines) / issues traded; six-week EMA (.286) (p.138)", "E|Buy when the average rises to 25% or higher (p.139)", "X|Sell when it falls below -5% (p.139); fills at the next Monday's close (p.140)"],
            "Needs series NYSE_WADV, NYSE_WDECL, NYSE_WISSUES.", c("", "", "+16.62%", "21.9% invested", "NYSE Index 1970-2003")),
        d("apta_ad_roc", "21-day / 10-day rate of change of the A-D line", "Ch 6 · Advance-decline line", NotTestable("levels are approximate and drift (\"approached or fell to below -10,000 and then turned up\", \"in recent years\", p.130-133)"), &[], false,
            &["F|21-day ROC of the NYSE A-D line; oversold about -9,000 to -10,000 (p.130)"], "Interpretation guide, no fixed rule.", None),
        // ---- Ch 7: sentiment and volatility
        d("apta_trin", "TRIN climactic levels", "Ch 7 · TRIN", NotTestable("thresholds change over time (\"It might be necessary ... to re-evaluate buying parameters from time to time\", p.153); buy at the level or after a peak is optional"), &[], false,
            &["F|10-day average TRIN 1.30 (past) / 1.50 (recent), 35-day average 1.30-1.35 (p.152)"], "No fixed rule.", None),
        d("apta_vix35", "VIX above 35 buy zone, hold to 18", "Ch 7 · The Volatility Index", Live, &["VIX.Close"], true,
            &["E|\"Be prepared to enter the stock market as VIX readings cross from below to above 35\" (p.156); \"You can purchase as VIX rises through that area\" (p.155). Buy on the close of that day.",
              "X|\"it has been safe in the past to hold stocks at least until VIX declines to 18 or lower\" (p.155). Sell on the close of the first day VIX closes at or below 18. [The book calls this the end of the safe period, not a sell signal; it is the only exit it gives]"],
            "VIX daily close (CBOE VIX in the US run, India VIX in the India run); p.vxo=1 uses the old VIX (VXO) the book says was VIX before 2003 (p.154). Book: signals 1990-2004 profitable in 1990, 1991, 1997, 1998, 2001, 2002 (p.155).", None),
        d("apta_mrv", "Major Reversal Volatility Model", "Ch 7 · Major Reversal Volatility Model", NotTestable("no exit: \"There are no sell rules.\" (p.157). Table 7.1 only measures gains 5-52 weeks after each signal; the vault does not pick a holding period for the book"), &[], false,
            &["F|\"At the end of each week, calculate the weekly percentage change in the Nasdaq Composite ... the absolute percentage change\"; \"Maintain a ten-week moving average of the absolute weekly percentage changes\" (p.157)",
              "E|\"The ten-week average ... must rise to 3% or above, but not to above 6%\"; \"A buy signal is generated when the ten-week average declines by at least half of 1% from its peak reading.\" (p.157) Buy on the week's close.",
              "F|\"If the ten-week average ... rises to above 6%, no buy signal can take place until it has first fallen to below 3%, risen to above 3% (but not as high as 6%), and then fallen by half of 1% from its peak.\" (p.157)",
              "F|After a signal the next one needs a fresh peak: the average must turn up (rise week over week) and then fall 0.5 from its new peak. [Book silent; Table 7.1 shows repeat signals 12/24/87-2/12/88 and 3/28-5/10-7/5-12/20/02 without a drop below 3%]",
              "X|\"There are no sell rules.\" (p.157) Exit on the close p.hold weeks later (default 52, the longest horizon of Table 7.1; 5/10/20/40 also printed). [Book silent on exit]"],
            "Weekly bars = last trading day of each week. Book index: Nasdaq Composite (run on NDX/SPX/NIFTY). One position at a time: signals while holding are skipped (the book counts every signal). Table 7.1 p.162.",
            c("18", "82.4% (52 wk)", "+26.6% (52 wk)", "52 weeks", "Nasdaq Composite 1970-2004")),
        // ---- Ch 8: MACD
        d("apta_macd", "MACD 12-26 buy / 19-39 sell (neutral-trend rules)", "Ch 8 · MACD", Live, &[], false, MACD_TA,
            "Applies the book's neutral-to-somewhat-positive rule set at all times. Not included (judgement): choosing 6-19 / 12-26 by the 50-day MA trend, bypassing the first sell without negative divergences, the 50-day MA back-up stop, trendline and cycle confirmations (p.174-196). No published statistics.", None),
        d("apta_macd_judgement", "MACD trend-adaptive rules (divergences, bypassed sells)", "Ch 8 · MACD", NotTestable("trend is \"actually, a somewhat subjective exercise\" (p.174); divergences, trendlines and cycles are chart judgement"), &[], false,
            &["F|Trend by the 50-day MA: very positive -> 6-19 buy; neutral -> 12-26 buy, 19-39 sell; negative -> 12-26 both (p.175)", "X|Bypass the first sell if no negative divergences; always take the second (p.177, 196)"], "Judgement layers of the MACD method.", None),
        d("apta_macd_fullsell", "Full-fledged MACD sell (19-39 falling below 0)", "Ch 8 · More active selling", NotTestable("no re-entry rule (\"you might await buy signals from more rapid MACD combinations\", p.185)"), &[], false,
            &["X|19-39 MACD falling and below 0 = stay out; cancelled when it turns up (p.185)"], "Exit filter only.", None),
        // ---- Ch 4 and 9: chart patterns, channels
        d("apta_patterns", "Chart patterns, wedges, angle changes", "Ch 4 · Power tool chart patterns", NotTestable("discretionary chart reading (trendlines, wedges, head and shoulders)"), &[], false,
            &["E|Buy at lower wedge boundary / when the upper trendline is penetrated (p.70-71)"], "Chart reading.", None),
        d("apta_ma_channels", "Moving average trading channels", "Ch 9 · Moving average trading channels", NotTestable("offsets chosen to hold \"approximately 85-90%\" of prices and rules are tendencies (\"As a general rule ... probably\", p.203, 217)"), &[], false,
            &["E|\"it is safe to buy penetrations of the lower bands\" (p.217)"], "Interpretation guide.", None),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "apta_ma200_dji" => ("apta_ma", "Moving average crossings", "200-day"),
        "apta_ma100_dji" => ("apta_ma", "Moving average crossings", "100-day"),
        "apta_triple_mom" => ("apta_roc", "Rate of change models", "Triple Momentum 5+15+25 > 4%"),
        "apta_roc15" => ("apta_roc", "Rate of change models", "15-day ROC > 0"),
        "apta_six_best" => ("apta_season", "Seasonal six-month periods", "Best six months"),
        "apta_six_worst" => ("apta_season", "Seasonal six-month periods", "Worst six months"),
        "apta_turn_of_month" | "apta_presidential" | "apta_cycles_tform" => ("apta_cycles", "Calendar and time cycles", ""),
        "apta_nasdaq_nyse_rs" | "apta_monetary" | "apta_rs_funds" => ("apta_mood", "Market mood indicators", ""),
        "apta_breadth_impulse" => ("apta_impulse", "Daily Breadth Impulse", "NYSE breadth"),
        "apta_breadth_impulse_nse" => ("apta_impulse", "Daily Breadth Impulse", "NSE breadth"),
        "apta_breadth_macd" => ("apta_impulse", "Daily Breadth Impulse", "MACD-filtered · NYSE breadth"),
        "apta_breadth_macd_nse" => ("apta_impulse", "Daily Breadth Impulse", "MACD-filtered · NSE breadth"),
        "apta_nhnl" | "apta_weekly_impulse" | "apta_ad_roc" => ("apta_breadth", "Breadth models", ""),
        "apta_trin" | "apta_vix35" | "apta_mrv" => ("apta_vol", "Sentiment and volatility", ""),
        "apta_macd" | "apta_macd_judgement" | "apta_macd_fullsell" => ("apta_macd", "MACD", ""),
        "apta_patterns" | "apta_ma_channels" => ("apta_charts", "Chart reading", ""),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let impulse = || vec![p("ema_n", "Breadth EMA days", 10., 5., 20., 1.), p("buy", "Buy when EMA % rises to", 61.5, 55., 70., 0.5), p("sell", "Sell when EMA % falls to", 49., 40., 55., 0.5)];
    Some(match id {
        "apta_ma200_dji" => vec![p("ma", "Simple moving average days", 200., 50., 300., 10.)],
        "apta_ma100_dji" => vec![p("ma", "Simple moving average days", 100., 50., 300., 10.)],
        "apta_triple_mom" => vec![p("r1", "ROC 1 days", 5., 2., 10., 1.), p("r2", "ROC 2 days", 15., 10., 20., 1.), p("r3", "ROC 3 days", 25., 20., 40., 1.), p("level", "Sum crosses %", 4., 0., 10., 0.5)],
        "apta_roc15" => vec![p("n", "ROC days", 15., 5., 30., 1.), p("level", "Crosses %", 0., -2., 2., 0.5)],
        "apta_six_best" | "apta_six_worst" => vec![p("may_day", "May date (last trading day on or before)", 4., 1., 15., 1.)],
        "apta_breadth_impulse" | "apta_breadth_impulse_nse" => impulse(),
        "apta_breadth_macd" | "apta_breadth_macd_nse" => {
            let mut v = impulse();
            v.extend([p("fast", "MACD fast EMA", 19., 10., 26., 1.), p("slow", "MACD slow EMA", 39., 26., 52., 1.), p("macd_max", "Sell only with MACD % below", 1., 0., 3., 0.25),
                p("denom", "MACD % divided by EMA (19 text / 39 example)", 19., 19., 39., 20.)]);
            v
        }
        "apta_vix35" => vec![p("buy", "Buy when VIX crosses above", 35., 25., 45., 1.), p("safe", "Hold until VIX at or below", 18., 12., 25., 1.),
            p("vxo", "Use the old VIX (VXO, S&P 100 options; US only): 1 yes, 0 no", 0., 0., 1., 1.)],
        "apta_mrv" => vec![p("avg_n", "Weeks in average", 10., 5., 20., 1.), p("lo", "Average must reach %", 3., 2., 4., 0.25), p("hi", "Not above %", 6., 4., 8., 0.5),
            p("drop", "Buy after a decline from the peak of", 0.5, 0.25, 1., 0.25), p("hold", "Hold weeks [book silent]", 52., 5., 52., 1.)],
        "apta_macd" => vec![p("bf", "Buy MACD fast EMA", 12., 6., 13., 1.), p("bs", "Buy MACD slow EMA", 26., 19., 26., 1.), p("bsig", "Buy signal line EMA", 9., 6., 9., 1.),
            p("sf", "Sell MACD fast EMA", 19., 12., 19., 1.), p("ss", "Sell MACD slow EMA", 39., 26., 39., 1.), p("ssig", "Sell signal line EMA", 9., 6., 9., 1.),
            p("stop", "MACD stop (1 on, 0 off)", 1., 0., 1., 1.), p("tf", "Bars: 0 daily, 1 weekly, 2 monthly", 0., 0., 2., 1.)],
        _ => return None,
    })
}

fn roc(c: &[f64], n: usize) -> Vec<f64> {
    (0..c.len()).map(|i| if i >= n { (c[i] / c[i - n] - 1.0) * 100.0 } else { f64::NAN }).collect()
}

/// EMA of advances / (advances + declines); days without breadth data are NaN.
pub fn impulse_ema(a: &[f64], d: &[f64], n: usize) -> Vec<f64> {
    let r: Vec<f64> = a.iter().zip(d).map(|(a, d)| if a + d > 0.0 { a / (a + d) } else { f64::NAN }).collect();
    ema_first(&r, n)
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if from > to || to >= x.r.len() {
        return None;
    }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap().def);
    let u = |k: &str| g(k).round().max(1.0) as usize;
    let c = &x.c;
    let daily: Vec<usize> = (0..x.r.len()).collect();
    let on = |v: f64| v.is_finite();
    Some(match id {
        "apta_ma200_dji" | "apta_ma100_dji" => {
            // exact-tie safe window sum (apwms::sma): a close equal to the average is not "above" or "below" it (p.48); the
            // running sum of connors::sma drifts by 1e-14 and turned decimal ties into trades (fixed 2026-10-24, authors/appel)
            let ma = super::apwms::sma(c, u("ma"));
            run_long(x, &daily, from, to, |_, i, h| {
                if !on(ma[i]) {
                    None
                } else if !h && c[i] > ma[i] {
                    Some("close above the moving average".into())
                } else if h && c[i] < ma[i] {
                    Some("close below the moving average".into())
                } else {
                    None
                }
            })
        }
        "apta_triple_mom" | "apta_roc15" => {
            let s: Vec<f64> = if id == "apta_triple_mom" {
                let (a, b, d) = (roc(c, u("r1")), roc(c, u("r2")), roc(c, u("r3")));
                (0..c.len()).map(|i| a[i] + b[i] + d[i]).collect()
            } else {
                roc(c, u("n"))
            };
            let lv = g("level");
            run_long(x, &daily, from, to, |_, i, h| {
                if !on(s[i]) {
                    None
                } else if !h && s[i] > lv {
                    Some("momentum crossed above the level".into())
                } else if h && s[i] < lv {
                    Some("momentum crossed below the level".into())
                } else {
                    None
                }
            })
        }
        "apta_six_best" | "apta_six_worst" => {
            let md = (5u32, g("may_day").round() as u32);
            let n = x.r.len();
            let mon = |i: usize| ymd(&x.r[i].date).1;
            let may = |i: usize| {
                let (_, m, dd) = ymd(&x.r[i].date);
                let (_, m2, d2) = ymd(&x.r[i + 1].date);
                (m, dd) <= md && (m2, d2) > md
            };
            let last_oct = |i: usize, k: usize| mon(i) == 10 && i + k < n && mon(i + k) != 10 && (k == 1 || mon(i + 1) == 10);
            let best = id == "apta_six_best";
            run_long(x, &daily, from, to, |_, i, h| {
                if i + 1 >= n {
                    return None;
                }
                let (oct, mayx) = (last_oct(i, if best { 1 } else { 2 }), may(i));
                match (best, h) {
                    (true, false) if oct => Some("last trading day of October".into()),
                    (true, true) if mayx => Some("last trading day on or before May 4".into()),
                    (false, false) if mayx => Some("trading day closest to May 4".into()),
                    (false, true) if oct => Some("next-to-last trading day of October".into()),
                    _ => None,
                }
            })
        }
        "apta_breadth_impulse" | "apta_breadth_impulse_nse" | "apta_breadth_macd" | "apta_breadth_macd_nse" => {
            let Some((a, d)) = breadth(x, id.ends_with("_nse")) else { return Some(vec![]) };
            let e = impulse_ema(a, d, u("ema_n"));
            let (buy, sell) = (g("buy") / 100.0, g("sell") / 100.0);
            let filt: Vec<f64> = if id.contains("macd") {
                let (f, s) = (ema(c, u("fast")), ema(c, u("slow")));
                let den = if g("denom") >= 29.0 { &s } else { &f };
                (0..c.len()).map(|i| (f[i] - s[i]) / den[i] * 100.0).collect()
            } else {
                vec![f64::NEG_INFINITY; c.len()]
            };
            let mx = if id.contains("macd") { g("macd_max") } else { f64::INFINITY };
            run_long(x, &daily, from, to, |_, i, h| {
                if !on(e[i]) {
                    None
                } else if !h && e[i] >= buy {
                    Some("breadth EMA rose to the buy level".into())
                } else if h && e[i] <= sell && filt[i] < mx {
                    Some(if mx.is_finite() { "breadth EMA at/below sell level and MACD below 1%" } else { "breadth EMA fell to the sell level" }.into())
                } else {
                    None
                }
            })
        }
        "apta_vix35" => {
            let src = if g("vxo") > 0.5 { "VXO_OHLC.Close" } else { "VIX.Close" };
            let Some(v) = x.series(src) else { return Some(vec![]) };
            let (buy, safe) = (g("buy"), g("safe"));
            let mut prev = f64::NAN;
            run_long(x, &daily, from, to, |_, i, h| {
                if !on(v[i]) {
                    return None;
                }
                let (pv, now) = (prev, v[i]);
                prev = now;
                if !h && pv <= buy && now > buy {
                    Some("VIX crossed above the buy level".into())
                } else if h && now <= safe {
                    Some("VIX at or below the safe level".into())
                } else {
                    None
                }
            })
        }
        "apta_mrv" => {
            let idx = bars(x, 1);
            let w: Vec<f64> = (0..idx.len()).map(|k| if k > 0 { (c[idx[k]] / c[idx[k - 1]] - 1.0).abs() * 100.0 } else { f64::NAN }).collect();
            let avg = sma(&w, u("avg_n"));
            let (lo, hi, drop, hold) = (g("lo"), g("hi"), g("drop"), u("hold"));
            // state: 0 idle, 1 armed (tracking the peak), 2 blocked (was above hi), 3 after a signal (waiting for a turn up)
            let (mut st, mut peak, mut entry_k) = (0u8, f64::NAN, 0usize);
            run_long(x, &idx, from, to, |k, _, h| {
                let a = avg[k];
                let mut fire = false;
                if on(a) {
                    if a > hi {
                        st = 2;
                    } else {
                        match st {
                            2 if a < lo => st = 0,
                            3 if k > 0 && a > avg[k - 1] && a >= lo => (st, peak) = (1, a),
                            3 if a < lo => st = 0,
                            0 if a >= lo => (st, peak) = (1, a),
                            1 => {
                                peak = peak.max(a);
                                if a <= peak - drop + 1e-12 {
                                    (st, fire) = (3, true);
                                }
                            }
                            _ => {}
                        }
                    }
                }
                if h {
                    (k >= entry_k + hold).then(|| format!("{hold} weeks after the signal"))
                } else if fire {
                    entry_k = k;
                    Some("volatility average fell 0.5 from its peak".into())
                } else {
                    None
                }
            })
        }
        "apta_macd" => macd_run(x, from, to, &Macd {
            buy: (u("bf"), u("bs"), u("bsig")), sell: (u("sf"), u("ss"), u("ssig")),
            both_above: false, sell_on_buy: false, stop: g("stop") > 0.5, tf: g("tf").round().clamp(0., 2.) as u8,
        }),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn rows(cl: &[f64]) -> Vec<Row> {
        cl.iter().enumerate().map(|(i, &c)| Row { ticker: "T".into(), date: format!("{:04}-{:02}-{:02}", 2000 + i / 336, 1 + (i / 28) % 12, 1 + i % 28),
            open: c, high: c, low: c, close: c, volume: None }).collect()
    }
    #[test]
    fn ema_is_seeded_by_the_simple_average_and_uses_2_over_n_plus_1() {
        let e = ema(&[1., 2., 3., 4.], 3);
        assert!(e[1].is_nan());
        assert_eq!(e[2], 2.0);
        assert!((e[3] - 3.0).abs() < 1e-12); // 2 + 0.5*(4-2)
        // worksheet p.144: .517 then .621 -> .536
        let w = ema_first(&[0.517, 0.621], 10);
        assert!((w[1] - 0.5359).abs() < 1e-3);
    }
    #[test]
    fn triple_momentum_buys_above_and_sells_below_four_percent() {
        let mut cl = vec![100.0; 40];
        for (k, v) in cl.iter_mut().enumerate().skip(30) {
            *v = 100.0 + 2.0 * (k - 29) as f64; // rally
        }
        cl.extend([95.0; 30]);
        let r = rows(&cl);
        let x = Ctx::new(&r, &r, &[]);
        let ts = run("apta_triple_mom", &x, 0, cl.len() - 1, &Params::new()).unwrap();
        assert_eq!(ts.len(), 1);
        assert_eq!(ts[0].legs[0].i, 30); // 5d ROC 2% + 15d 2% + 25d 2% = 6% > 4 on the first up day
        assert!(!ts[0].open && ts[0].exit_i == 40);
    }
    #[test]
    fn weekly_bars_end_on_the_last_trading_day_of_each_week() {
        let r: Vec<Row> = ["2024-01-04", "2024-01-05", "2024-01-08", "2024-01-12", "2024-01-15"].iter()
            .map(|d| Row { ticker: "T".into(), date: d.to_string(), open: 1., high: 1., low: 1., close: 1., volume: None }).collect();
        let x = Ctx::new(&r, &r, &[]);
        assert_eq!(bars(&x, 1), vec![1, 3]); // Fri 5th, Fri 12th; Mon 15th is unfinished
        assert_eq!(bars(&x, 2), Vec::<usize>::new());
    }
}

// Instrument admission: Appel 2026-10-24 block, moved from instrument_policy.rs on 31 Oct 2026 (lane ST).
// Admission only for the module REVIEWED lists (authors/appel/admission/SOURCE-CONTRACT.md); kept in one block to merge.
// Runs on actual indices, ETFs and stocks, US and India (index_etf_or_stock, nifty50 basket, declared application).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| matches!(def.book,"apta"|"apmacd"|"apwms"|"apsmts"|"apoi") && (crate::books::apta::reviewed(def.id) || crate::books::apmacd::reviewed(def.id) || crate::books::apwms::reviewed(def.id) || crate::books::apsmts::reviewed(def.id) || crate::books::apoi::reviewed(def.id)),
    label: "Gerald Appel tested these models on the Dow, the NYSE Index, the Nasdaq Composite or the S&P 500 with NYSE breadth; index, ETF and stock runs (US and India, India with NSE breadth and India VIX) are a declared application, not the author's test. Cash-index shorts are hypothetical exposure.",
    basis: "Gerald Appel: Technical Analysis - Power Tools (2005) 200/100-day Dow crossings, Triple Momentum, 15-day ROC, best/worst six months, Daily Breadth Impulse (with the MACD filter), VIX 35/18, MACD 12-26/19-39; the 1996 MACD booklet rule sets and A-D super stay-in; Winning Market Systems TRIN, up/down volume, Shadow Line, 10-day A-D total and Haurlan indices; Stock Market Trading Systems (with Hitschler) penetration, confirming, shifting, MMACDTS, lead-time, chord, A-D overbought/oversold and 25-day channel systems; Opportunity Investing presidential strategies. Reviewed in authors/appel/admission/SOURCE-CONTRACT.md with independent chronological ledgers. Close fills; no costs; the book's statistics are not reproduced.",
};
