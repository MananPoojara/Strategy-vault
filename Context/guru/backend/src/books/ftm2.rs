//! Futures Magazine (futuresmag.com) 2001-2005, second pass. Book id `ftm2` (extends `ftm`, which is not edited).
//! Covers the Trading Techniques / Technology & Trading columns that `ftm` only triaged (see library/ftm/README.md,
//! "Remaining"). Source: research drive "Trading Knowledge II/@NewsLetters-Mags/Futures Magazine (2001-2005)
//! (futuresmag.com) (449 MB)_ON DVD37_/<yyyymm>.pdf"; copies in Book-to-rust/Books/Futures Magazine/.
//! Page refs are "issue, PDF page (printed page)". Only articles that state a complete mechanical system (entry AND
//! exit) are Live; everything else is NeedsData / NotTestable with the reason. Systems were tested in the magazine on
//! futures or US stocks; here they run on the selected index (instrument swap only). No costs in the engine.
//! Transcription, oracle and verification: library/ftm2/.
use crate::connors::{claim, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "ftm2";
/// Admitted ids (authors/futuresmag/admission/SOURCE-CONTRACT.md).
pub const REVIEWED: &[&str] = &["ftm2_zg_nday", "ftm2_rg_santa", "ftm2_rg_santa_up", "ftm2_mm_macd_r1", "ftm2_mm_macd_r2", "ftm2_ac_vote", "ftm2_dm_skew" /* intraday wave 2b: daily bars decide every trade */];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

// ------------------------------------------------------------------ helpers

fn mk(dir: f64, i: usize, px: f64, sig: usize, kind: &'static str) -> Trade {
    Trade { dir, legs: vec![Leg { i, px, w: 1. }], exit_i: i, exit_px: px, reason: String::new(), entry_kind: kind, signal_i: sig, open: false, exits: vec![] }
}
fn fin(mut t: Trade, i: usize, px: f64, sig: usize, why: &str) -> Trade {
    t.exit_i = i; t.exit_px = px; t.signal_i = sig; t.reason = why.into(); t
}
fn mark_open(out: &mut Vec<Trade>, cur: Option<Trade>, x: &Ctx, to: usize) {
    if let Some(mut t) = cur { t.exit_i = to; t.exit_px = x.c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true; out.push(t); }
}
fn mean(v: &[f64]) -> f64 { v.iter().sum::<f64>() / v.len() as f64 }
fn hi(v: &[f64]) -> f64 { v.iter().copied().fold(f64::NEG_INFINITY, f64::max) }
fn lo(v: &[f64]) -> f64 { v.iter().copied().fold(f64::INFINITY, f64::min) }
fn sgn(v: f64) -> i32 { if v > 0. { 1 } else if v < 0. { -1 } else { 0 } }
/// EMA with K = 2/(n+1), seeded with the first value (Meissner eq. 1: EMA_t = (P_t K - EMA_t-1 K) + EMA_t-1).
fn ema(v: &[f64], n: usize) -> Vec<f64> {
    let k = 2. / (n as f64 + 1.);
    let mut out = Vec::with_capacity(v.len());
    for (i, &p) in v.iter().enumerate() { out.push(if i == 0 { p } else { (p * k - out[i - 1] * k) + out[i - 1] }); }
    out
}
fn ymd(d: &str) -> (i64, i64, i64) { (d[0..4].parse().unwrap_or(0), d[5..7].parse().unwrap_or(0), d[8..10].parse().unwrap_or(0)) }
/// Days since 1970-01-01.
fn days(y: i64, m: i64, d: i64) -> i64 {
    let y2 = if m <= 2 { y - 1 } else { y };
    let era = y2.div_euclid(400); let yoe = y2 - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    era * 146097 + yoe * 365 + yoe / 4 - yoe / 100 + doy - 719468
}
/// US Thanksgiving: fourth Thursday of November, as days since epoch.
fn thanksgiving(y: i64) -> i64 {
    let nov1 = days(y, 11, 1);
    let wd = (nov1 + 3).rem_euclid(7); // 0 = Monday
    nov1 + (3 - wd).rem_euclid(7) + 21
}
fn day_of(x: &Ctx, i: usize) -> i64 { let (y, m, d) = ymd(&x.r[i].date); days(y, m, d) }

// ------------------------------------------------------------------ catalogue

struct S { id: &'static str, name: &'static str, group: &'static str, dir: &'static str, status: Status,
           rules: &'static [&'static str], notes: &'static str, claim: Option<crate::connors::Claim> }

const G_ZG: &str = "Oct 2001 · Zamansky & Goldcamp · Walking a fine line";
const G_RGG: &str = "Oct-Dec 2001 · Ruggiero · game-theory trading robot";
const G_RGS: &str = "Feb 2002 · Ruggiero · The seasons of trading";
const G_DM: &str = "Apr / Oct 2002 · McNicholl · moving skewness filter";
const G_MM: &str = "Jul 2002 · Meissner, Alex & Nolte · Building a better MACD indicator";
const G_AC: &str = "Jul 2005 · Art Collins · The two-edged sword of optimization";
const G_ND: &str = "Other articles (data or rules missing)";

fn defs() -> Vec<S> {
    use Status::*;
    vec![
        S { id: "ftm2_zg_nday", name: "N-day close breakout, N/2 close exit (fixed N = 50)", group: G_ZG, dir: "Long/Short", status: Live,
            rules: &["E|\"Buy next open if the close > highest high value of the last N days\"; \"Sell next open if the close < lowest low of the last N days\" (200110 PDF 50, p.50)",
                     "X|\"Exit a long position if the close < lowest low of the last N / 2 days (same N as in entry signal)\"; \"Exit a short position if the close > highest high of the last N / 2 days\" (p.50)",
                     "F|N = 50: the article's 'Fixed input: 50' portfolio (table 'Hold on & look back', PDF 49, p.49); inputs 10-100 step 10 were tested"],
            notes: "Readings (article silent): 'the last N days' = the N days before the signal bar (the close cannot exceed a high that includes its own bar); exits are filled at the next open like the entries; an opposite entry while in a position reverses at that open. The article's subject (walk-forward re-optimisation of N across a six-futures portfolio) is ftm2_zg_walkforward (data missing). Book: BP, DX, JY, MB, SF, US continuous futures (Tick Data), no costs.",
            claim: claim("196 (portfolio)", "", "$968.16", "", "6 interest-rate/currency futures, fixed N=50, net $189,760, PF 1.85 (dates of the fixed-input test not printed; walk-forward charts 1990-2001)") },
        S { id: "ftm2_zg_walkforward", name: "Walk-forward portfolio optimisation of N (best / worst input)", group: G_ZG, dir: "Long/Short",
            status: NeedsData("needs the six continuous futures contracts of the study (British pound, dollar index, Japanese yen, municipal bonds, Swiss franc, T-bonds): the input N is re-chosen every 60 (30) days from the PORTFOLIO's net profit / average trade / Sharpe over the previous 120 (60) days, so one index cannot reproduce the selection"),
            rules: &["F|Base system as ftm2_zg_nday with N in 10..100 step 10 (p.49-50)",
                     "E|\"The walk-forward studies begin with a 120-day lookback and a 60-day holding period, and with a 60-day lookback and a 30-day holding period. The objectives are to switch between the best and worst strategies over the lookback period\"; criteria: Sharpe ratio for daily returns, average trade profit, total net profit (p.50)"],
            notes: "Published (p.49): 120/60 best avg trade $89,261 (211 trades) vs worst $125,285 (349); 60/30 best avg trade $19,797 vs worst $252,916; fixed 50 $189,760; average of all inputs $150,964 (281 trades).", claim: None },
        S { id: "ftm2_rg_santa", name: "Santa Claus rally: Monday after Thanksgiving to the 2nd trading day of February", group: G_RGS, dir: "Long", status: Live,
            rules: &["E|\"To test the 'holiday effect,' we will buy on the Monday after Thanksgiving\" (200202 PDF 50, p.51)",
                     "X|\"and hold until the second trading day of February\" (PDF 51, p.52)"],
            notes: "Fill (article silent): at the close of both days by default (the test used the S&P 500 cash index from 1962, whose early history has closing prices only); option fill=1 uses the opens. Thanksgiving = fourth Thursday of November (US calendar, also used on Indian indices: instrument swap only); if the Monday is not a trading day, the next trading day. Book: S&P 500 cash index 1962 - Dec 27, 2001.",
            claim: claim("39 (+ open 2001 trade)", "rallies > 70% of the time", "477 S&P points in total", "22.20% of the time in the market", "S&P 500 cash index 1962-12/27/2001 (~44% of the index's total gain)") },
        S { id: "ftm2_rg_santa_up", name: "Santa Claus rally, only in years up from Jan. 1 to Thanksgiving", group: G_RGS, dir: "Long", status: Live,
            rules: &["F|\"If we include only years that are up from Jan. 1 to Thanksgiving, we find results that are even more impressive\" (200202 PDF 51, p.52)",
                     "E|Buy on the Monday after Thanksgiving (as ftm2_rg_santa)", "X|Hold until the second trading day of February"],
            notes: "Reading of 'up from Jan. 1 to Thanksgiving': the close of the last trading day before Thanksgiving is above the close of the last trading day of the previous year (the price on Jan. 1). Fills as ftm2_rg_santa.",
            claim: claim("28 qualifying years", "21 of 28 up", "401.52 S&P points in total", "14.35% of the time in the market", "S&P 500 cash index 1962-2001; outperforms buy-and-hold by more than 2.5-to-1") },
        S { id: "ftm2_rg_seasonal_fut", name: "Corn and T-note seasonal trades (incl. corn seasonal window breakout)", group: G_RGS, dir: "Long/Short",
            status: NeedsData("needs corn and 10-year T-note futures (daily, back-adjusted) and their contract point values"),
            rules: &["E|Corn: short \"around June 20\" exit \"the last trading day of November\"; long \"Feb. 26\" exit \"June 20\" (200202 PDF 51, p.52)",
                     "E|Corn window: \"buy on a breakout after Feb. 7 but before the end of the month. We will sell between June 15 and June 30. We will enter on a 10-day breakout\" (p.52)",
                     "E|T-notes: \"selling on Feb. 2 and exiting the short position on April 26\"; \"go long T-notes on April 26 and exit that long position on the open of Oct. 2\" (PDF 53, p.54)"],
            notes: "Published: corn short avg $675/yr (~65%), long avg $518/yr; window trade 37 trades 70% $1,054.89 DD -$4,100; T-note short 18 trades 72% $1,177.78; long 19 trades 79% $2,907.89 (1983-2001).", claim: None },
        S { id: "ftm2_rg_adaptive_cbo", name: "Adaptive channel breakout (n = Hilbert dominant cycle)", group: G_RGG, dir: "Long/Short",
            status: NotTestable("n is 'the length of the current dominant cycle' from Ehlers' Hilbert transform; the article gives no formula (it refers to Rocket Science for Traders) and the code calls an unprinted function hilbertcycle((H+L)/2); taking the formula from another book is not allowed"),
            rules: &["E|\"we are defining n as the length of the current dominant cycle\"; \"Buy at highest(high,value1) + 1 point stop; Sell at lowest(low,value1) - 1 point stop\" (200111 PDF 63, p.61; 200112 PDF 56, p.56)"],
            notes: "Published (Japanese yen, 16 years): net $121,587, 124 trades, 40%, avg $980.54, DD -$12,969.50.", claim: None },
        S { id: "ftm2_rg_yen_tbill", name: "Intermarket yen / T-bill divergence system", group: G_RGG, dir: "Long/Short",
            status: NeedsData("needs Japanese yen and 3-month T-bill futures (daily)"),
            rules: &["E|\"If T-bill futures are in an uptrend and the yen is falling, we buy the yen. If T-bill futures are in a downtrend and the yen is rising, we sell the yen\"; trend = close above / below a simple MA; \"a four-day moving average for the yen and a 20-day moving average for T-bills\" (200111 PDF 64, p.62)",
                     "E|Code: \"If close < Average(close,TrMk) and close of Data(2) > Average(close,IntMk) of Data(2) then buy at open\" (mirror sell); TrMk 4, IntMk 20 (200112 PDF 56, p.56)"],
            notes: "Stop-and-reverse (opposite signal).", claim: None },
        S { id: "ftm2_rg_game_model", name: "Game-theory combined model (channel / triple MA / intermarket by ADX trend mode)", group: G_RGG, dir: "Long/Short",
            status: NotTestable("the code calls adaptivetrendmode(25,15,65) and hilbertcycle((H+L)/2), whose formulas are not printed (adaptive ADX of half the Hilbert dominant cycle); it also needs yen and T-bill futures (Data(2))"),
            rules: &["F|\"If ADX > 25 and Mode = 0 then Mode = 1; If ADX < 15 and Mode > 0 then Mode = 0; If Mode = 1 and ADX > 65 then Mode = 0.5; If Mode = 0.5 and ADX[1] < 25 and ADX > 25 then Mode = 1\" (200112 PDF 55, p.55)",
                     "E|Mode 1: adaptive channel breakout + triple MA (8, 20, 30); mode 0.5: triple MA; mode 0: intermarket (code PDF 56, p.56)"],
            notes: "Published Jan. 1, 1987 - Sept. 28, 2001 (yen): net $180,170, 226 trades, 50.44%, avg $797.21, DD $18,901. The stand-alone triple MA (8/20/30, 'If MA1 > MA2 and MA2 > MA3 then buy at open') is the vault's rgt_ma3 / rwmts_ma3x idea (duplicate, not repeated).", claim: None },
        S { id: "ftm2_dm_skew", name: "Moving-skewness pulse (SL 4.4) · two-day trade", group: G_DM, dir: "Long", status: Live,
            rules: &["F|Moving mean: \"α = 0.1, St = α CLOSEt + (1 - α) St-1, Dt = α St + (1 - α) Dt-1, Mt = ((2 - α) St - Dt) / (1 - α)\"; DEVt = CLOSEt - Mt (200210 PDF 49, p.49, eq. 1-2)",
                     "F|\"β = 0.05, SMDt = β |DEVt| + (1 - β) SMDt-1, DMDt = β SMDt + (1 - β) DMDt-1, MDt = ((2 - β) SMDt - DMDt) / (1 - β), STDt = 1.25 MDt\"; \"M3t = Υ (DEVt)^3 + (1 - Υ) M3t-1, G1t = M3t / (STDt)^3\", Υ = 0.333 (eq. 3-5)",
                     "F|\"CLAMPt = G1t, G1t >= 0; = 0, G1t < 0\"; \"CLIPt = CLAMPt, CLAMPt > CLAMPt-1; = 0, CLAMPt <= CLAMPt-1\"; \"SL = 4.4\"; \"SPSt = 1, CLIPt-1 < SL <= CLIPt\" (eq. 6-9); the crossing day is Day 0 (PDF 50, p.50)",
                     "E|\"Trade entry: Buy at the market ... at the opening price on Day 1\" (p.50)",
                     "X|\"Trade exit: Exit from the trade at the market on the close of Day 1 if a profit exists at that point. If a loss exists at the close on Day 1, do not exit on Day 1 but hold on for another day and exit the trade at the close on Day 2\" (p.50)"],
            notes: "Long only. Initialisation (article silent; it points to McNicholl's book): S = D = the first close of the history, SMD = DMD = M3 = 0, and no signal in the first 60 bars ('a startup period of 60 days to initialize the equations', 200204 PDF 33, p.31). G1 = 0 while STD = 0. 'Profit' = Day 1 close above the Day 1 open (break-even holds to Day 2). One trade at a time: a signal whose Day 1 is not after the current trade's exit day is ignored. Book: 26 DJIA stocks, 4/7/1999-10/18/1999, $25,000 per trade.",
            claim: claim("35", "71% (25 W / 10 L)", "1.623%", "1-2 days", "26 DJIA stocks 4/7/1999-10/18/1999; avg win $787.58, avg loss $548.76 on $25,000") },
        S { id: "ftm2_mm_macd_r1", name: "MACD-R1: crossing confirmed 3 days, profit target or next crossing", group: G_MM, dir: "Long/Short", status: Live,
            rules: &["F|MACD-1 = 12-day EMA - 26-day EMA; \"the signal indicator, is the nine-day moving average of the MACD-1 indicator\"; EMA \"K = 2 / (number of periods + 1)\" (200207 PDF 54-55, pp.52-53)",
                     "E|\"A buy signal is given when the more volatile MACD-1 crosses the less-volatile signal indicator from beneath\" (sell: from above); MACD-R1: \"the trading signal is given three days after the actual crossing if the trend is still intact. Thus, the position is opened at the closing price of the third day after the crossing if no crossing has appeared on day two and day three\" (PDF 55, p.53)",
                     "X|\"the models give a signal to close an open position when a 3% or 5% target gain has been reached or if another crossing occurs before the target is reached\" (p.53)"],
            notes: "Prices are daily closes ('314,645 daily closing prices'), so the target and crossings are checked on closes and filled at that close. Default target 3% (5% = parameter value). 'Trend still intact' = no crossing on days 1-3 after the crossing (day 3 = entry). EMAs seeded with the first price; no signal before bar 35 (26 + 9). Book: Nasdaq-100 stocks out-of-sample (DJIA stocks in-sample), May 30, 1989 - May 30, 1999, no costs.",
            claim: claim("", "61.62% (3% target) / 52.17% (5%)", "4.92% (3%) / 6.88% (5%)", "", "Nasdaq-100 stocks 1989-1999 (bullish 65.88% / bearish 57.36% at 3%)") },
        S { id: "ftm2_mm_macd_r2", name: "MACD-R2: R1 plus |MACD - signal| >= 1.5% of price on day 3", group: G_MM, dir: "Long/Short", status: Live,
            rules: &["E|\"the buy or sell signal is given if the difference between the moving averages is bigger than or equal to a certain percentage of the stock price at the end of the third day after a crossing. Crossing levels from 0.5% to 3.5% were tested\"; example: price $100, MACD-1 = 2, signal = 1 -> 1% (PDF 56, p.54)",
                     "E|\"The concept of MACD-R2 is the same as in model MACD-R1\" (entry at the third day's close)",
                     "X|Target gain or the next crossing, as MACD-R1 (p.53)"],
            notes: "Default level 1.5%, the level the article analyses (\"for the 1.5% crossing level, 70.76% of all trades generated an average profit of 5.94%\"); the table's 0.5-3.5% are parameter values. The R2 target is not restated: default 3% (the first R1 value), 5% as parameter. Readings as MACD-R1.",
            claim: claim("79.9 signals/yr (all Nasdaq-100 stocks, 1.5%)", "70.76% (1.5%); 61.80% (0.5%) ... 89.39% (3.5%)", "5.94% (1.5%)", "5.06 days", "Nasdaq-100 stocks 1989-1999") },
        S { id: "ftm2_ac_vote", name: "Collins three-vote day trade (close vs 40-day avg, 5- vs 2-day avg, 50-day high/low order)", group: G_AC, dir: "Long/Short", status: Live,
            rules: &["E|\"an 'almost-there' system that would get you long or short on every daily opening and out on the close. It looks at three events that happen every day in the market in an either/or fashion. Each possibility gets assigned a +1 or -1 value\" (200507 PDF 40, p.39)",
                     "F|\"1. The close can be above (+1) or below (-1) the average 40-day close. 2. The average five-day close can be above (+1) or below (-1) the average two-day close. 3. The highest close of the last 50 days can occur either before (+1) or after (-1) the lowest close of the last 50 days.\"",
                     "E|\"Each day produces a net positive or negative number. If it's plus, we buy. If it's minus, we sell.\"",
                     "X|Exit on the close of the same day (\"out on the close\")"],
            notes: "Votes use the closes up to the previous day (the order is placed for the opening). A value exactly equal to its average votes 0; equal highest (lowest) closes: the most recent counts. Per-year walk-forward variables (table 'Outside analysis', PDF 41, p.40) are parameter values, e.g. S&P 2001: 4 / 10 / 30 / 40. Book: futures (S&P, Nasdaq, Russell, bonds, notes, currencies), 2000-2004 optimised per market; printed rule values 2 / 5 / 40 / 50 are the defaults.",
            claim: claim("2001-04: 248 / 252 / 252 / 252 (S&P)", "54.03% / 50.4% / 52.38% / 51.98%", "net $90,812 / $37,325 / $20,500 / $8,550", "1 day", "S&P 500 futures out-of-sample years with re-optimised variables (table p.40)") },
        S { id: "ftm2_ac_spdow", name: "S&P vs Dow strength filter (MA crossover / breakout, 5-15-minute bars)", group: G_ND, dir: "Long/Short",
            status: NeedsData("needs intraday (5/15-minute) S&P 500 and Dow futures bars"),
            rules: &["E|\"buy at the market if the S&P is stronger than the Dow\"; \"a simple 50-bar perpetual moving average crossover system combined with the Dow/S&P spread filter\"; \"Hold the position until the signal reverses, or exit on a 40-point, $10,000 money management stop\" (200304 PDF 47-50, pp.46-49)"],
            notes: "Published: 1,409 trades, $233,150, $165.47 per trade (5-minute bars).", claim: None },
        S { id: "ftm2_sc_spty", name: "S&P / 10-yr T-note intraday mean-reversion spread", group: G_ND, dir: "Long/Short",
            status: NeedsData("needs 5-minute S&P 500 and 10-year T-note futures"),
            rules: &["F|\"Intermarket Spread = ln (close of SP using five-minute bars/yesterday's close of SP) - ln (close of TY using five-minute data/yesterday's close of TY)\"; bands from the 30-day portfolio standard deviation (200402 PDF 48, p.49)",
                     "E|\"Buy SP and sell TY when the intermarket spread crosses above the lower volatility band, which is set at minus three standard deviations\" (mirror) (PDF 49, p.50)",
                     "X|\"The intermarket spread crosses zero\" or crosses beyond five standard deviations (p.50)"],
            notes: "Published SP side 10/29/2002-10/31/2003: 27 trades, 74.07%, PF 3.66, net $49,050.", claim: None },
        S { id: "ftm2_sa_pcn", name: "Path Classification Number system (PCN 2121 / 1212)", group: G_ND, dir: "Long/Short",
            status: NeedsData("needs tick (time and sales) data: the PCN is built from every new high / low print within the day"),
            rules: &["E|\"If (Yesterday was an Up-day && PCN == 2121) Then Sell; Stop loss: Buy on stop at today's High+1 tick; Close Out: Buy Market on Close (MOC)\" (mirror for 1212) (200403 PDF 56, p.56)"],
            notes: "S&P MidCap 400 futures 7/1993-2003, equity curve only.", claim: None },
        S { id: "ftm2_ac_ideal_1min", name: "Collins idealized 1-minute limit model", group: G_ND, dir: "Long/Short",
            status: NeedsData("needs 1-minute E-mini S&P bars; the author states the results are 'not realistically achievable in real time'"),
            rules: &["E|\"if time>=(b*100) and time<=(x*100) then buy next bar at o of tomorrow-(j) limit\"; j = average(u*(o-l),uu), uu 5, b 8.45, x 14.55; exit \"if time=1500 then sell next bar at market\" (mirror short) (200503 PDF 54, p.54)"],
            notes: "", claim: None },
        S { id: "ftm2_jt_energy_seasonals", name: "October energy seasonals (heating oil, natural gas, crude, gasoline)", group: G_ND, dir: "Short",
            status: NeedsData("needs heating oil / natural gas / crude / gasoline futures contracts (specific delivery months)"),
            rules: &["E|e.g. \"January heating oil futures prices have actually closed lower on about Oct. 29 than on about Oct. 16 in 13 of the last 15 years\" (200310 PDF 48, p.48)"],
            notes: "Dates are 'about' and per delivery month.", claim: None },
        S { id: "ftm2_hs_eqti", name: "Equity Tension Index (EqTI) trading rule matrix", group: G_ND, dir: "Long/Short",
            status: NotTestable("the three components (AMA trend oscillator, log(Vix/HLC volatility), fear/regret index) are 'blended' with no stated weights, and the HLC-volatility formula is an illegible image (200309 PDF 38, p.39)"),
            rules: &["E|Trending (AMA >= 21): buy \"EqTI > 0 & EqTI > EqTI(t-1)\"; transitional (11 <= AMA < 21); sideways: buy \"EqTI > 0\", sell \"EqTI < 0\" (200309 PDF 38, p.39)"],
            notes: "", claim: None },
        S { id: "ftm2_jb_dsi", name: "Bernstein DSI extremes + 10/8 moving-average channel", group: G_ND, dir: "Long/Short",
            status: NotTestable("no exit is stated (and the Daily Sentiment Index history is not in the vault); the author: 'They must be used with discretion and confirmation'"),
            rules: &["E|\"When daily sentiment surpasses 75%, a sell signal is not triggered until sentiment drops back below 75%\"; \"The short position is triggered by two consecutive daily price bars entirely below the lower moving average\" (10-day MA of highs, 8-day MA of lows) (200306 PDF 44-46, pp.45-47)"],
            notes: "", claim: None },
        S { id: "ftm2_cn_fade_breadth", name: "Fade the breadth (advancers vs decliners, one-week returns)", group: G_ND, dir: "Long/Short",
            status: NotTestable("a statistic, not a trading rule: average one-week index performance after breadth conditions, with no entry order or price stated; same study as How Markets Really Work ch.5 (hmrw, documented there as statistics)"),
            rules: &["F|\"when NYSE Advancers are greater than NYSE Decliners for at least three days in a row\"; Nasdaq two days in a row; \"Advancers are at least two times ... Decliners today\" (200501 PDF 42-43, pp.42-43)"],
            notes: "", claim: None },
        S { id: "ftm2_rg_casebased", name: "Case-based (nearest-neighbour) forecast · buy/sell next open", group: G_ND, dir: "Long/Short",
            status: NotTestable("the minimum number of similar cases ('specified in a dialog box') and the distance-to-similarity mapping for the 80% threshold are not printed (Oct 2003 - Feb 2004 series)"),
            rules: &["E|\"if the prediction column is less than zero, we could sell tomorrow's open. If it is greater than zero, we could buy tomorrow's open\"; three-day pattern, similarity 0.80 (200402 PDF 53, p.54)"],
            notes: "Published Nasdaq futures 11/4/1997-11/13/2003: 50.9% direction, 1,234.30 points.", claim: None },
        S { id: "ftm2_ac_200dma_fx", name: "Cofnas 200-day MA forex rules", group: G_ND, dir: "Long/Short",
            status: NotTestable("no exit is stated, and '10% from the average, get ready' is a judgement step"),
            rules: &["E|\"1. If price is 10% from the average, get ready to trade it. 2. If price breaks the average, put on half of your size. 3. When price breaks 10% above or below the average put on the rest of your size.\" (200208 PDF 35, p.33)"],
            notes: "", claim: None },
    ]
}

pub fn catalogue() -> Vec<StrategyDef> {
    defs().into_iter().map(|s| StrategyDef { id: s.id, name: s.name, group: s.group, session: 0, dir: s.dir, status: s.status,
        needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules: s.rules, notes: s.notes, claim: s.claim }).collect()
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "ftm2_zg_nday" => ("ftm2_zg", "Zamansky & Goldcamp N-day breakout", "Fixed N = 50"),
        "ftm2_zg_walkforward" => ("ftm2_zg", "Zamansky & Goldcamp N-day breakout", "Walk-forward · data missing"),
        "ftm2_rg_santa" => ("ftm2_rg_seas", "Ruggiero seasonals", "Santa Claus rally"),
        "ftm2_rg_santa_up" => ("ftm2_rg_seas", "Ruggiero seasonals", "Santa · up years only"),
        "ftm2_rg_seasonal_fut" => ("ftm2_rg_seas", "Ruggiero seasonals", "Corn / T-notes · data missing"),
        "ftm2_rg_adaptive_cbo" | "ftm2_rg_yen_tbill" | "ftm2_rg_game_model" => ("ftm2_rg_game", "Ruggiero game-theory robot", ""),
        "ftm2_dm_skew" => ("ftm2_dm", "McNicholl moving skewness", ""),
        "ftm2_mm_macd_r1" => ("ftm2_mm", "Meissner refined MACD", "MACD-R1"),
        "ftm2_mm_macd_r2" => ("ftm2_mm", "Meissner refined MACD", "MACD-R2"),
        "ftm2_ac_vote" => ("ftm2_ac_vote", "Collins three-vote day trade", ""),
        "ftm2_ac_spdow" | "ftm2_sc_spty" | "ftm2_sa_pcn" | "ftm2_ac_ideal_1min" | "ftm2_jt_energy_seasonals" =>
            ("ftm2_nd", "Futures Magazine · data missing", ""),
        "ftm2_hs_eqti" | "ftm2_jb_dsi" | "ftm2_cn_fade_breadth" | "ftm2_rg_casebased" | "ftm2_ac_200dma_fx" =>
            ("ftm2_nt", "Futures Magazine · not testable", ""),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let macd = || vec![p("fast", "MACD fast EMA", 12., 2., 50., 1.), p("slow", "MACD slow EMA", 26., 5., 100., 1.), p("sig", "Signal EMA", 9., 2., 30., 1.),
                       p("conf", "Entry: close of day N after the crossing", 3., 1., 10., 1.), p("target", "Profit target % (book 3 or 5)", 3., 0.5, 20., 0.5)];
    Some(match id {
        "ftm2_zg_nday" => vec![p("n", "Breakout look-back N (exit uses N/2)", 50., 10., 100., 10.)],
        "ftm2_rg_santa" | "ftm2_rg_santa_up" => vec![p("fill", "Fill: 0 = closes (default), 1 = opens", 0., 0., 1., 1.)],
        "ftm2_dm_skew" => vec![p("alpha", "Moving-mean smoothing α", 0.1, 0.01, 0.5, 0.01), p("beta", "Moving-SD smoothing β", 0.05, 0.01, 0.5, 0.01),
                               p("gamma", "Skewness smoothing Υ", 0.333, 0.05, 0.9, 0.01), p("sl", "Pulse selection level SL", 4.4, 1., 10., 0.1),
                               p("k", "STD = k x MD", 1.25, 1., 2., 0.05), p("startup", "Start-up bars without signals", 60., 0., 250., 10.)],
        "ftm2_mm_macd_r1" => macd(),
        "ftm2_mm_macd_r2" => { let mut v = macd(); v.push(p("level", "Crossing level, % of price", 1.5, 0.5, 3.5, 0.5)); v }
        "ftm2_ac_vote" => vec![p("a", "Short average ('two-day')", 2., 1., 20., 1.), p("b", "Longer average ('five-day')", 5., 2., 30., 1.),
                               p("ma", "Close vs N-day average ('40-day')", 40., 5., 100., 5.), p("ext", "High/low order look-back ('50 days')", 50., 5., 100., 5.)],
        _ => return None,
    })
}

// ------------------------------------------------------------------ run

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if from > to || to >= x.r.len() { return None; }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(0., |d| d.def));
    let u = |k: &str| g(k).max(0.).round() as usize;
    Some(match id {
        "ftm2_zg_nday" => nday(x, from, to, u("n").max(2)),
        "ftm2_rg_santa" => santa(x, from, to, g("fill") >= 0.5, false),
        "ftm2_rg_santa_up" => santa(x, from, to, g("fill") >= 0.5, true),
        "ftm2_dm_skew" => skew(x, from, to, g("alpha"), g("beta"), g("gamma"), g("sl"), g("k"), u("startup")),
        "ftm2_mm_macd_r1" => macd_r(x, from, to, u("fast").max(1), u("slow").max(2), u("sig").max(1), u("conf").max(1), g("target"), None),
        "ftm2_mm_macd_r2" => macd_r(x, from, to, u("fast").max(1), u("slow").max(2), u("sig").max(1), u("conf").max(1), g("target"), Some(g("level"))),
        "ftm2_ac_vote" => vote(x, from, to, u("a").max(1), u("b").max(1), u("ma").max(1), u("ext").max(2)),
        _ => return None,
    })
}

/// Zamansky & Goldcamp N-day close breakout with N/2 close exit; all orders at the next open; reversals allowed.
fn nday(x: &Ctx, from: usize, to: usize, n: usize) -> Vec<Trade> {
    let m = (n / 2).max(1);
    let mut out = vec![];
    let mut cur: Option<Trade> = None;
    for t in n..to {
        let pos = cur.as_ref().map_or(0., |c| c.dir);
        let long_x = pos > 0. && x.c[t] < lo(&x.l[t - m..t]);
        let short_x = pos < 0. && x.c[t] > hi(&x.h[t - m..t]);
        let go_long = t >= from && x.c[t] > hi(&x.h[t - n..t]);
        let go_short = t >= from && x.c[t] < lo(&x.l[t - n..t]);
        let e = t + 1;
        if pos > 0. && (long_x || go_short) { out.push(fin(cur.take().unwrap(), e, x.o[e], t, if long_x { "close < N/2-day low" } else { "reverse" })); }
        if pos < 0. && (short_x || go_long) { out.push(fin(cur.take().unwrap(), e, x.o[e], t, if short_x { "close > N/2-day high" } else { "reverse" })); }
        if cur.is_none() {
            if go_long { cur = Some(mk(1., e, x.o[e], t, "open")); } else if go_short { cur = Some(mk(-1., e, x.o[e], t, "open")); }
        }
    }
    mark_open(&mut out, cur, x, to);
    out
}

/// Santa Claus rally: buy the Monday after Thanksgiving, sell the 2nd trading day of February.
fn santa(x: &Ctx, from: usize, to: usize, open_fill: bool, up_only: bool) -> Vec<Trade> {
    let px = |i: usize| if open_fill { x.o[i] } else { x.c[i] };
    let mut out = vec![];
    let n = x.r.len();
    let (y0, _, _) = ymd(&x.r[0].date);
    let (y1, _, _) = ymd(&x.r[n - 1].date);
    for y in y0..=y1 {
        let tg = thanksgiving(y);
        let Some(e) = (0..n).find(|&i| day_of(x, i) >= tg + 4) else { continue };
        if ymd(&x.r[e].date).0 != y || e < from || e > to { continue; }
        if up_only {
            // last bar before Thanksgiving vs last bar of the previous year
            let Some(b) = (0..e).rev().find(|&i| day_of(x, i) < tg) else { continue };
            let Some(pz) = (0..=b).rev().find(|&i| ymd(&x.r[i].date).0 < y) else { continue };
            if ymd(&x.r[b].date).0 != y || !(x.c[b] > x.c[pz]) { continue; }
        }
        let t = mk(1., e, px(e), e, if open_fill { "open" } else { "close" });
        let mut feb = (e + 1..n).filter(|&i| { let (yy, mm, _) = ymd(&x.r[i].date); yy == y + 1 && mm == 2 });
        let _ = feb.next();
        match feb.next() {
            Some(xi) if xi <= to => out.push(fin(t, xi, px(xi), xi, "2nd trading day of February")),
            _ => mark_open(&mut out, Some(t), x, to),
        }
    }
    out
}

/// McNicholl moving-skewness pulse filter with the article's two-day trading rule (long only).
#[allow(clippy::too_many_arguments)]
fn skew(x: &Ctx, from: usize, to: usize, a: f64, b: f64, gm: f64, sl: f64, k: f64, startup: usize) -> Vec<Trade> {
    let c = &x.c;
    let nn = c.len();
    let mut clip = vec![0.; nn];
    let (mut s, mut d) = (c[0], c[0]);
    let (mut smd, mut dmd, mut m3, mut prev_clamp) = (0., 0., 0., 0.);
    for i in 0..nn {
        s = a * c[i] + (1. - a) * s;
        d = a * s + (1. - a) * d;
        let m = ((2. - a) * s - d) / (1. - a);
        let dev = c[i] - m;
        smd = b * dev.abs() + (1. - b) * smd;
        dmd = b * smd + (1. - b) * dmd;
        let md = ((2. - b) * smd - dmd) / (1. - b);
        let std = k * md;
        m3 = gm * dev.powi(3) + (1. - gm) * m3;
        let g1 = if std > 0. { m3 / std.powi(3) } else { 0. };
        let clamp = if g1 >= 0. { g1 } else { 0. };
        clip[i] = if clamp > prev_clamp { clamp } else { 0. };
        prev_clamp = clamp;
    }
    let mut out = vec![];
    let mut busy_until: Option<usize> = None;
    for t in startup.max(1)..to {
        if t < from || !(clip[t - 1] < sl && sl <= clip[t]) { continue; }
        let e = t + 1;
        if busy_until.is_some_and(|b| e <= b) { continue; }
        let tr = mk(1., e, x.o[e], t, "open");
        if c[e] > x.o[e] { out.push(fin(tr, e, c[e], e, "Day 1 close, in profit")); busy_until = Some(e); }
        else if e + 1 <= to { out.push(fin(tr, e + 1, c[e + 1], e + 1, "Day 2 close")); busy_until = Some(e + 1); }
        else { mark_open(&mut out, Some(tr), x, to); busy_until = Some(to); }
    }
    out
}

/// Meissner MACD-R1 / R2: crossing, confirmation for `conf` days, entry at that close; exit at the close on the target or the next crossing.
#[allow(clippy::too_many_arguments)]
fn macd_r(x: &Ctx, from: usize, to: usize, fast: usize, slow: usize, sig: usize, conf: usize, target: f64, level: Option<f64>) -> Vec<Trade> {
    let c = &x.c;
    let (ef, es) = (ema(c, fast), ema(c, slow));
    let macd: Vec<f64> = ef.iter().zip(&es).map(|(f, s)| f - s).collect();
    let sl = ema(&macd, sig);
    let dif: Vec<f64> = macd.iter().zip(&sl).map(|(m, s)| m - s).collect();
    // TradeStation 'crosses above/below': value > 0 and previous <= 0 (and mirror)
    let cross = |i: usize| -> i32 { if i == 0 { 0 } else if dif[i] > 0. && dif[i - 1] <= 0. { 1 } else if dif[i] < 0. && dif[i - 1] >= 0. { -1 } else { 0 } };
    let warm = slow + sig;
    let mut out = vec![];
    let mut cur: Option<Trade> = None;
    for t in warm..=to {
        if let Some(tr) = cur.take() {
            let ep = tr.legs[0].px;
            if cross(t) != 0 { out.push(fin(tr, t, c[t], t, "next crossing")); }
            else if tr.dir * (c[t] / ep - 1.) * 100. >= target { out.push(fin(tr, t, c[t], t, "target")); }
            else { cur = Some(tr); }
            continue;
        }
        if t < from || t >= to || t < warm + conf { continue; }
        let t0 = t - conf;
        let dir = cross(t0);
        if dir == 0 || (t0 + 1..=t).any(|k| cross(k) != 0) { continue; }
        if let Some(lv) = level { if dif[t].abs() < lv / 100. * c[t] { continue; } }
        cur = Some(mk(dir as f64, t, c[t], t, "close"));
    }
    mark_open(&mut out, cur, x, to);
    out
}

/// Collins three-vote system: decided on yesterday's closes, in at the open, out at the close.
fn vote(x: &Ctx, from: usize, to: usize, a: usize, b: usize, ma: usize, ext: usize) -> Vec<Trade> {
    let c = &x.c;
    let need = a.max(b).max(ma).max(ext);
    let mut out = vec![];
    for t in from.max(need)..=to {
        let v1 = sgn(c[t - 1] - mean(&c[t - ma..t]));
        let v2 = sgn(mean(&c[t - b..t]) - mean(&c[t - a..t]));
        let w = &c[t - ext..t];
        let (mut ih, mut il) = (0, 0);
        for (j, &v) in w.iter().enumerate() { if v >= w[ih] { ih = j; } if v <= w[il] { il = j; } }
        let v3 = if ih < il { 1 } else if ih > il { -1 } else { 0 };
        let s = v1 + v2 + v3;
        if s == 0 { continue; }
        out.push(fin(mk(s.signum() as f64, t, x.o[t], t - 1, "open"), t, c[t], t, "close"));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn rows(n: usize) -> Vec<Row> {
        (0..n).map(|i| Row { ticker: "T".into(), date: format!("{:04}-{:02}-{:02}", 2000 + i / 300, 1 + (i / 25) % 12, 1 + i % 25), open: 100., high: 101., low: 99., close: 100., volume: None }).collect()
    }
    #[test]
    fn thanksgiving_dates() {
        assert_eq!(thanksgiving(2001), days(2001, 11, 22));
        assert_eq!(thanksgiving(2024), days(2024, 11, 28));
        assert_eq!(thanksgiving(1983), days(1983, 11, 24));
    }
    #[test]
    fn vote_goes_long_when_high_came_before_low_and_close_above_avg() {
        let mut r = rows(60);
        // falling then sharply up at the end: close > 40-day avg (+1), 5-day avg < 2-day avg (-1), 50-day high? set
        for (i, row) in r.iter_mut().enumerate() { let v = 200. - i as f64; row.close = v; row.open = v; row.high = v + 1.; row.low = v - 1.; }
        r[58].close = 300.; // yesterday spikes: v1 +1, v2 -1 (2-day avg > 5-day avg), highest close (bar 58) after lowest (bar 57) -> -1
        let x = Ctx::new(&r, &r, &[]);
        let ts = vote(&x, 59, 59, 2, 5, 40, 50);
        assert_eq!(ts.len(), 1); assert_eq!(ts[0].dir, -1.); assert_eq!(ts[0].legs[0].px, r[59].open); assert_eq!(ts[0].exit_px, r[59].close);
    }
    #[test]
    fn nday_enters_next_open_and_exits_on_half_channel() {
        let mut r = rows(80);
        r[60].close = 103.; r[60].high = 103.5;       // close > 50-day high (101) -> buy open 61
        r[61].open = 102.;
        r[70].close = 98.; r[70].low = 97.5;          // close < 25-day low (99) -> exit open 71
        r[71].open = 98.5;
        let x = Ctx::new(&r, &r, &[]);
        let ts = nday(&x, 0, 79, 50);
        assert_eq!(ts[0].legs[0].i, 61); assert_eq!(ts[0].legs[0].px, 102.);
        assert_eq!(ts[0].exit_i, 71); assert_eq!(ts[0].exit_px, 98.5);
    }
    #[test]
    fn catalogue_ids_unique_and_live_have_params() {
        let c = catalogue();
        let mut ids: Vec<_> = c.iter().map(|d| d.id).collect(); ids.sort(); ids.dedup();
        assert_eq!(ids.len(), c.len());
        for d in &c { assert!(d.id.starts_with("ftm2_") && family_of(d.id).is_some(), "{}", d.id); }
        for d in c.iter().filter(|d| d.status == Status::Live) { assert!(params(d.id).is_some(), "{}", d.id); }
    }
}
