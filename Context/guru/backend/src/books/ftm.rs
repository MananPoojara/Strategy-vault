//! Futures Magazine (futuresmag.com), issues Jan 2001 - Jul 2005. Book id `ftm`.
//! Source: research drive "Trading Knowledge II/@NewsLetters-Mags/Futures Magazine (2001-2005) (futuresmag.com)
//! (449 MB)_ON DVD37_/<yyyymm>.pdf"; copies in Book-to-rust/Books/Futures Magazine/. Only articles that state a
//! complete mechanical system (entry AND exit) are Live; everything else is NeedsData / NotTestable with the reason.
//! Page refs are "issue, PDF page (printed page)". Transcription, oracle and verification: library/ftm/.
//! Systems were tested in the magazine on futures or US stocks; here they run on the selected index (instrument swap
//! only). Dollar figures in claims are as printed (per futures contract). No costs are deducted in the engine.
use crate::connors::{rsi, sma, true_range, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "ftm";
/// Admitted ids (authors/futuresmag/admission/SOURCE-CONTRACT.md).
pub const REVIEWED: &[&str] = &["ftm_km_co_open", "ftm_km_co_limit", "ftm_km_hh_open", "ftm_km_hh_limit", "ftm_rg_dip", "ftm_cvr9", "ftm_zg_weak2r", "ftm_kf_sd", "ftm_kf_sd_full", "ftm_mt_macd", "ftm_mt_macd_rsi", "ftm_mt_macd_stoch"];
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
/// Simple mean of the last n true ranges, including bar i (Katz & McCormick's ATR; TradeStation Average(TrueRange,n)).
fn atr_mean(x: &Ctx, n: usize) -> Vec<f64> {
    let tr = true_range(x.r);
    sma(&tr, n)
}
/// Population standard deviation of the last n closes including bar i (TradeStation StdDev).
fn stdev_pop(c: &[f64], n: usize) -> Vec<f64> {
    (0..c.len()).map(|i| if i + 1 < n { f64::NAN } else {
        let w = &c[i + 1 - n..=i]; let m = mean(w); (w.iter().map(|v| (v - m).powi(2)).sum::<f64>() / n as f64).sqrt()
    }).collect()
}
/// Days since 1970-01-01 for "YYYY-MM-DD".
fn days(d: &str) -> i64 {
    let (y, m, dd): (i64, i64, i64) = (d[0..4].parse().unwrap_or(1970), d[5..7].parse().unwrap_or(1), d[8..10].parse().unwrap_or(1));
    let y2 = if m <= 2 { y - 1 } else { y };
    let era = y2.div_euclid(400); let yoe = y2 - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + dd - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}
/// 0 = Monday .. 6 = Sunday
fn weekday(d: &str) -> i64 { (days(d) + 3).rem_euclid(7) }

// ------------------------------------------------------------------ catalogue

struct S { id: &'static str, name: &'static str, group: &'static str, dir: &'static str, status: Status, vix: bool,
           rules: &'static [&'static str], notes: &'static str, claim: Option<crate::connors::Claim> }

const G_KM: &str = "Feb 2001 · Katz & McCormick · Do breakout systems work on stocks?";
const G_RG02: &str = "May 2002 · Ruggiero · What makes a system a winner";
const G_RG03: &str = "Aug 2003 · Ruggiero · Learning from your system's mistakes";
const G_CV: &str = "Jun 2002 · Connors · Timing your S&P trades with the Vix";
const G_ZG: &str = "Feb 2002 · Zamansky & Goldcamp · Success is relative";
const G_KF: &str = "Jan 2002 · Fitschen sidebar 'Logical exclusions' (in Ruggiero, Debunking the drawdown myth)";
const G_MT: &str = "Jan 2001 · Matoba · A MACD combo to counter risk";
const G_ND: &str = "Other articles (data or rules missing)";

const SES: &str = "X|SES: \"The profit target was set to the entry price plus some multiple of the 50-day ATR\" - \"profit target at 10 ATR units above the entry price\" (limit order); \"stop loss at four ATR units below the entry price\"; \"If neither the profit target nor stop loss closes out the trade prior to the 20th bar, the trade is closed out by a market order at the open of the 21st bar\" (200102 PDF 16-17, pp.39-40)";
const KMNOTE: &str = "Long only, as in the article. Book: all optionable Nasdaq stocks, $5,000 per trade, $10-$100 price filter, $15 commission; here one index (instrument swap; price filter and sizing not applicable). Readings (book silent): ATR = simple mean of 50 true ranges at the signal bar; stop and target orders are first active on the bar after entry; stop gaps fill at the open; if stop and target both touch on one bar the stop is taken first; 'at least two days have passed' = entry bar >= exit bar + 2. Prices not rounded to ticks.";

fn defs() -> Vec<S> {
    use Status::*;
    vec![
        S { id: "ftm_km_co_open", name: "Close-only breakout (M=20) · next open · SES", group: G_KM, dir: "Long", status: Live, vix: false,
            rules: &["E|\"Buy tomorrow at the open ... if today's close is greater than any close of the preceding M days, and if yesterday's close was not higher than the highest close of the preceding M-1 days\" (200102 PDF 17, p.40); best M = 20", SES,
                     "F|\"no new trade can be entered until at least two days have passed since any previous trade on the same stock has been closed out\"; no pyramiding (p.40)"],
            notes: KMNOTE, claim: crate::connors::claim("IS 6,350 / OOS 3,830", "IS 53% / OOS 53%", "IS $82 / OOS $225 (per $5,000)", "~20 days", "Nasdaq optionable stocks IS 11/4/1996-6/10/1999, OOS -9/28/2000 (table p.39)") },
        S { id: "ftm_km_co_limit", name: "Close-only breakout (M=20) · limit at breakout-bar midpoint · SES", group: G_KM, dir: "Long", status: Live, vix: false,
            rules: &["E|Same breakout as the open entry; \"The price for limit entries was set as the average of the breakout bar's high and low\"; entry the day after the breakout (p.40)", SES,
                     "F|Two-day wait after an exit; no pyramiding (p.40)"],
            notes: KMNOTE, claim: crate::connors::claim("IS 4,705 / OOS 2,902", "IS 52% / OOS 54%", "IS $77 / OOS $224 (per $5,000)", "", "Nasdaq optionable stocks (table p.39)") },
        S { id: "ftm_km_hh_open", name: "Highest-high breakout (M=20) · next open · SES", group: G_KM, dir: "Long", status: Live, vix: false,
            rules: &["E|\"The highest-high breakout is identical to the close-only, except that the high of the current bar must break above the highest high of the last M days. Again, entry is on the next bar's open or at a limit\" (p.40)", SES,
                     "F|Two-day wait after an exit; no pyramiding (p.40)"],
            notes: KMNOTE, claim: crate::connors::claim("IS 6,177 / OOS 3,755", "IS 52% / OOS 53%", "IS $62 / OOS $202 (per $5,000)", "", "Nasdaq optionable stocks (table p.39)") },
        S { id: "ftm_km_hh_limit", name: "Highest-high breakout (M=20) · limit at breakout-bar midpoint · SES", group: G_KM, dir: "Long", status: Live, vix: false,
            rules: &["E|Highest-high breakout; limit at the average of the breakout bar's high and low, next bar only (p.40)", SES, "F|Two-day wait after an exit (p.40)"],
            notes: KMNOTE, claim: crate::connors::claim("IS 4,785 / OOS 2,963", "IS 51% / OOS 54%", "IS $50 / OOS $229 (per $5,000)", "", "Nasdaq optionable stocks (table p.39)") },
        S { id: "ftm_km_vol", name: "Volatility breakout (close + B x ATR) · Katz & McCormick", group: G_KM, dir: "Long",
            status: NotTestable("ambiguous entry: the article says the threshold is 'the current bar's closing price plus some multiple B of the M-day average true range' but not which later price (high or close, next bar or same bar) must cross it; the text says M=10, B=2.5 while the table says M=20, B=2.5"), vix: false,
            rules: &["E|\"the volatility breakout employs a threshold determined as the current bar's closing price plus some multiple B of the M-day average true range\" (p.40)", SES], notes: "", claim: None },
        S { id: "ftm_rg_oops", name: "Ruggiero OOPS (coded version) · S&P", group: G_RG02, dir: "Long/Short", status: Live, vix: false,
            rules: &["F|\"If nextopen20 > High + 0.02 * Average(TrueRange,3) then UpGap=TRUE; If nextopen20 < Low - 0.02 * Average(TrueRange,3) then DnGap=TRUE\" (200205 PDF 60, p.58)",
                     "E|\"If UpGap=TRUE and Close > Average(Close,10) then sell at Close - 2 points stop; If DnGap=TRUE and Close < Average(Close,10) then buy at Close + 2 points stop\"",
                     "X|\"If BarsSinceEntry > 2 then exitlong at open; If BarsSinceEntry > 2 then exitshort at open\""],
            notes: "TradeStation code wins over the prose ('sell on a stop at yesterday's high minus one tick'). nextopen20 = the next day's open (Ruggiero's look-ahead function for backtesting = the gap known at the open). Reading: the exit fires at the close of the bar where BarsSinceEntry (0 on the entry bar) exceeds 'hold' and executes at the next open; an opposite entry reverses (TradeStation). 2 points kept literally on any index (instrument swap). Book: S&P futures 1/1/1987-3/21/2002, $250/point, no costs.",
            claim: crate::connors::claim("279", "57.35%", "$619.18 (= 2.48 pts at $250)", "2 bars", "S&P 500 futures 1/1/1987-3/21/2002; net $172,752, DD -$46,262") },
        S { id: "ftm_rg_dip", name: "Ruggiero buy-the-dip (two down closes) · S&P", group: G_RG02, dir: "Long", status: Live, vix: false,
            rules: &["E|\"If Highest(Close-Close[1],Lookback)<0 then buy at nextopen20 + 0.4*Average(Range,3) stop\"; prose: \"buy the market if we have had two days that closed lower than the day before at the next open plus 0.40 times the average range over the past three days\" (200205 PDF 60, p.58)",
                     "X|\"If Highest(Close-Close[1],Lookback)>0 then exitlong at nextopen20 - 0.6 * average(range,3) stop\"; \"If the market has closed higher on at least one of the past two days, we will exit the long trade on the next day's open minus 0.60 times the average range of the past three days\""],
            notes: "Lookback = 2 (prose). Range = high - low (TradeStation), averaged over the last 3 bars including the signal bar. Long only, no other stop. Book: S&P futures since January 1986 at $250/point.",
            claim: crate::connors::claim("", "", "", "", "S&P futures 1/1986-~4/2002: +1,042 points ($260,000), max DD 216 points, equity peak 1,204 points on 11/7/2000, ~-$30,000 since Jan 2000") },
        S { id: "ftm_rg_feedback", name: "20-bar breakout with virtual-equity feedback filter", group: G_RG03, dir: "Long/Short", status: Live, vix: false,
            rules: &["F|Virtual system: \"VirtualBuy BuyBrk at HighPrice + MinMove STOP; VirtualSell SellBrk at LowPrice - MinMove STOP\", HighPrice = Highest(HIGH,20,0), LowPrice = Lowest(LOW,20,0); PLVir = VirGetPerformance(\"NetProfit\") (200308 PDF 52, p.56)",
                     "E|\"If PLVir<PLVir[LB] then Buy BuyBrk at HighPrice + MinMove STOP\"; \"If PLVir<PLVir[LB] then Sell SellBrk at LowPrice - MinMove STOP\"",
                     "X|\"ExitLong XBuyBrk at LowPrice - MinMove STOP; ExitShort XSellBrk at HighPrice + MinMove STOP\" - \"We always exit on a 20-day breakout regardless of whether there are profits\""],
            notes: "LB default 20 (the value used in the article's money-management study; the optimisation table covers 10-60). NetProfit = closed-trade net profit of the always-in virtual 20-bar stop-and-reverse system, in index points, minus 'vcost' points per trade (book $75 per trade on futures; default 0 because a dollar cost cannot be mapped to an index). MinMove = 'tick' (NSE 0.05 default). Highest/Lowest include the signal bar. If both stops are touched on a bar with no position, the one nearer the open fills (book silent). The unfiltered base 20-bar breakout is the vault's rgc_cbo20 (duplicate, not repeated). Book: basket of gold, yen, 10-yr T-note, coffee futures 1/1/1980-5/8/2003.",
            claim: crate::connors::claim("329 (LB=20, 4 markets)", "46.81%", "$1,241.63", "", "Gold/yen/T-note/coffee basket 1980-2003 net $408,496 DD -$19,071 (table p.56)") },
        S { id: "ftm_cvr9", name: "Connors VIX Reversal 9 (CVR 9)", group: G_CV, dir: "Long/Short", status: Live, vix: true,
            rules: &["E|Buy: \"1. Today the Vix must make a 10-period high and close below its open. 2. Yesterday the Vix must have closed greater than its open. 3. Today's Vix range must be the largest of the last three days. 4. ... buy S&P futures on the close and exit in three days\" (200206 PDF 46, p.46)",
                     "E|Sell: \"1. Today the Vix must make a 10-period low and close above its open. 2. Yesterday the Vix must have closed below its open. 3. Today's Vix range must be the largest of the last three days. 4. ... sell S&P futures on the close and exit in three days\"",
                     "X|Exit on the close three trading days after entry"],
            notes: "10-period high = today's VIX high is the highest of the last 10 VIX highs (ties count); range largest = strictly greater than each of the two previous ranges. One position at a time (signals while in a trade are ignored; book silent). 2002 'Vix' = the OEX-based index (now VXO); the vault uses CBOE VIX (US) / India VIX (India).",
            claim: crate::connors::claim("~1 every 3 weeks since 1993", "64.8%", "just over $1,400 per S&P contract", "3 days", "S&P 500 futures 1993-2002") },
        S { id: "ftm_zg_weak2r", name: "Buy intraday weakness: open - 2 x five-day range, exit fifth open", group: G_ZG, dir: "Long", status: Live, vix: false,
            rules: &["E|\"If price is less than the open minus twice the five-day range, then buy\" (table 'Expanding our range', row 4, 200202 PDF 38, p.38); definition: \"A move above or below the open that is X% of the last five day's average range. This move can occur anytime in the day\" (p.37)",
                     "X|\"Fifth open following entry\"; \"fixed-interval exit on the open Y periods from entry date, which typically is five opening periods later\""],
            notes: "Average five-day range = mean of (high - low) of the 5 completed days before the entry day. Filled at the level (a buy limit below the open). Trades non-overlapping. The ranked variants (top/bottom 8 or 15 of the Nasdaq 100 by 20/60-day momentum) need a stock universe: see ftm_zg_ranked. Book: Nasdaq 100 stocks 1/1/1999-10/15/2001.",
            claim: crate::connors::claim("419", "", "1.92%", "5 opens", "84 Nasdaq-100 stocks 1/1/1999-10/15/2001") },
        S { id: "ftm_kf_sd", name: "Fitschen SD-expansion breakout (base)", group: G_KF, dir: "Long/Short", status: Live, vix: false,
            rules: &["F|\"We'll use the standard deviation of the last 50 closes to measure volatility. When today's standard deviation figure is greater than it was some time ago, volatility is increasing and we should look to enter a position\" (200201 PDF 52, p.52)",
                     "E|\"If today's close is higher [than the close some period ago], we'll enter long. If today's close is lower, we'll go short.\" Look-back 22 bars",
                     "X|\"exit the position when today's standard deviation is less than it was some period ago\""],
            notes: "Readings (book silent): orders at the next open (the article's own pullback rule enters 'at the next open'); population standard deviation (TradeStation StdDev); an opposite entry signal reverses. Book: 46-commodity basket 1980-1990, $75 per trade.",
            claim: crate::connors::claim("2,434", "45.6% (1,111 W / 1,323 L)", "$152", "", "46 commodities 1980-1990, $371,025") },
        S { id: "ftm_kf_sd_full", name: "Fitschen SD-expansion with 1.5 SD stop, pullback entry and volatility filter", group: G_KF, dir: "Long/Short", status: Live, vix: false,
            rules: &["E|Base entry plus \"A pullback pattern ... When all conditions are met for a long trade, and price closes below yesterday's close, enter at the next open\" (short: closes above) (p.52)",
                     "F|\"A volatility filter. When today's standard deviation is greater than the average of the most recent three standard deviation numbers, a trade can be entered\"",
                     "X|\"A stop of 1.5 standard deviations of price from the entry. This is not an intraday stop. The exit in the event of an adverse close will be made on the next open.\" Plus the base SD exit",
                     "F|Non-default option skipwed=1: \"When trades originating on Wednesday were skipped\""],
            notes: "SD for the stop = the SD on the entry signal bar. Wednesday = the signal bar's weekday (option only).",
            claim: crate::connors::claim("1,463", "46% (673 W / 790 L)", "$277", "", "46 commodities 1980-1990, $405,737, max DD $58,652") },
        S { id: "ftm_mt_macd", name: "Matoba MACD (smoothing factors) always in · close", group: G_MT, dir: "Long/Short", status: Live, vix: false,
            rules: &["F|\"EMA = s x Price + (1 - s) x EMA[-1]\"; \"the average of the high, the low and the closing price ... is used as the asset price\" (200101 PDF 16, p.40)",
                     "E|\"the position is long one unit if MACD is positive and short one unit if MACD is negative\"; MACD = EMA(higher s) - EMA(lower s)",
                     "X|\"daily trading is conducted at the closing price of the day when a new signal is given\" (p.41)"],
            notes: "Defaults = the article's S&P 500 daily factors (0.95, 0.90); Nikkei 0.85/0.15 and Hang Seng 0.95/0.70 as parameter values. EMA seeded with the first price.",
            claim: crate::connors::claim("", "", "annual return 12.33%, risk 14.67%, Sharpe 0.84", "", "S&P 500 cash index daily 4/3/1989-7/31/2000; max DD 30.90% (8/23/1990-7/9/1992)") },
        S { id: "ftm_mt_macd_rsi", name: "Matoba MACD with RSI(10) position size", group: G_MT, dir: "Long/Short", status: Live, vix: false,
            rules: &["E|Direction exactly as MACD alone; \"the long or short position is determined according to ... (100 - |RSI x 2 - 100|)/100\"; \"RSI (10-day RSI ...)\" (p.41)", "X|Position re-sized at each close"],
            notes: "Implemented as one-day slices (close to next close) weighted by the size, so equity is exact; trade counts are per day. RSI = Wilder RSI(10) of closes (book silent on the price).",
            claim: crate::connors::claim("", "", "annual return 11.19%, risk 10.56%, Sharpe 1.06", "", "S&P 500 daily 4/3/1989-7/31/2000; max DD 21.84%") },
        S { id: "ftm_mt_macd_stoch", name: "Matoba MACD with %D(10) position size", group: G_MT, dir: "Long/Short", status: Live, vix: false,
            rules: &["E|Direction as MACD alone; size \"(100 - |%D x 2 - 100|)/100\"; \"stochastics (10-day %D ...)\" (p.41)", "X|Position re-sized at each close"],
            notes: "%D = 3-day simple average of 10-day %K (standard definition; the article does not print it). One-day slices as for the RSI version.",
            claim: crate::connors::claim("", "", "annual return 8.40%, risk 8.23%, Sharpe 1.02", "", "S&P 500 daily 4/3/1989-7/31/2000; max DD 15.75%") },
        // ---------------------------------------------------------- not runnable
        S { id: "ftm_hd_consensus", name: "Headley sentiment extremes (Consensus % bulls, 21-week BB)", group: G_ND, dir: "Long/Short",
            status: NeedsData("weekly Consensus Inc. % bullish readings per market (CONSENSUS_BULLS); not available for India"), vix: false,
            rules: &["E|\"Long Entry: c > Average(c,20) and close of Data2[1] < BollingerBand(c,21,-1.5)[1] and close of Data2 > BollingerBand(c,21,-1.5)\" (200102 PDF 20, p.45)",
                     "X|\"Long Exit: BarsSinceEntry=12 or (BarsSinceEntry = 5 and PositionProfit < 0) or close of Data2 > BollingerBand(c,21,1.5)\"; shorts mirror"],
            notes: "Weekly bars; Data2 = Consensus % bulls.", claim: crate::connors::claim("", "60%", "$-912", "", "S&P 500 weekly 4/23/1982-12/1/2000, net $-4,560") },
        S { id: "ftm_rg_reportday", name: "Ruggiero report-day volume system (10-yr T-note)", group: G_ND, dir: "Long/Short",
            status: NeedsData("US unemployment-report release calendar and 10-yr T-note futures"), vix: false,
            rules: &["E|\"If Volume>Average(Volume,20) and Close<Close[5] then buy at the next open\" (day before the unemployment report) (200206 PDF 53, p.53)", "X|\"If DaysSinceEntry>3 then Exitlong at yesterdays low - 0.03125 stop\""],
            notes: "Also: buy the morning after the two-year note auction if the 10-yr closes above its 40-day average, exit 8 days later (needs auction calendar).", claim: crate::connors::claim("124", "57.26%", "$286.65", "", "10-yr T-note 1/1990-3/8/2002") },
        S { id: "ftm_rg_cot", name: "Ruggiero COT commercials model (S&P)", group: G_ND, dir: "Long", status: NeedsData("CFTC Commitments of Traders net commercial positions"), vix: false,
            rules: &["F|\"Inputs: LookBack(50),Lev(0.80)\" ... \"If COTVal < 0.40 then exitlong at open\" (200204 PDF 59, p.59)"], notes: "", claim: None },
        S { id: "ftm_rg_intermarket", name: "Ruggiero intermarket models (silver/T-note, RBPI, CRB)", group: G_ND, dir: "Long/Short", status: NeedsData("silver, 10-yr T-note and commodity futures series"), vix: false,
            rules: &["E|\"If Ten Year>Average(Ten Year, 4) and Silver >Average(Silver, 28) then sell at open; If Ten Year<Average(Ten Year, 4) and Silver <Average(Silver, 28) then buy at open\" (200205 PDF 61, p.59)"],
            notes: "Jan 2001 RBPI (6th root of 6 commodity prices) and seasonal T-bond windows (Feb 2002) also need futures data.", claim: crate::connors::claim("256", "62.11%", "$337.24", "", "10-yr T-note since 1/1/1988") },
        S { id: "ftm_zg_ranked", name: "Zamansky & Goldcamp relative-strength + intraday range variants", group: G_ZG, dir: "Long", status: NeedsData("a stock universe ranked by 20- and 60-day returns (book: Nasdaq 100 members on 1/1/1999)"), vix: false,
            rules: &["E|\"Buy if the stock trades during the day from the open minus X% of Y times the stock's average five-day range for the Z highest ranked securities\" (X=150%, Z=15) (p.38)", "X|Exit at the fifth opening following entry"],
            notes: "", claim: crate::connors::claim("225", "", "4.13%", "5 opens", "Nasdaq 100 top-15 1999-2001") },
        S { id: "ftm_an_system_e", name: "Anthonisz System E (range expansion) / System I (hot level)", group: G_ND, dir: "Long/Short",
            status: NotTestable("intraday order of the first range-expansion print cannot be known from daily bars, the equity-index percentage Rp is not given, and System I needs tick data (most-traded price)"), vix: false,
            rules: &["E|\"IF (Yesterday was an Up-Day and Percentage Range > Rp and current price = the low) Then Sell; Stop loss: Buy on stop at today's High + 1 tick; Close: Buy Market on Close\" (200405 PDF 37, p.37)"], notes: "", claim: None },
        S { id: "ftm_carr_osc", name: "Carr oscillator systems (MACD/momentum/RSI/ROC/stochastics)", group: G_ND, dir: "Long/Short",
            status: NotTestable("indicator parameters are those of a separate Jan 2005 'Oscillators explained' article not on the drive; the 2.5% stop is a percentage of capital, not price"), vix: false,
            rules: &["E|\"RSI: When the reading of the indicator rises above 30 from below, a buy signal is taken. Sells are signaled when the indicator crosses below 70\" (FM June 2005 PDF 44, p.44)"], notes: "", claim: None },
    ]
}

pub fn catalogue() -> Vec<StrategyDef> {
    defs().into_iter().map(|s| StrategyDef { id: s.id, name: s.name, group: s.group, session: 0, dir: s.dir, status: s.status,
        needs_volume: false, needs_vix: s.vix, needs_series: &[], book: BOOK, rules: s.rules, notes: s.notes, claim: s.claim }).collect()
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "ftm_km_co_open" => ("ftm_km", "Katz & McCormick stock breakouts", "Close-only · open"),
        "ftm_km_co_limit" => ("ftm_km", "Katz & McCormick stock breakouts", "Close-only · limit"),
        "ftm_km_hh_open" => ("ftm_km", "Katz & McCormick stock breakouts", "Highest high · open"),
        "ftm_km_hh_limit" => ("ftm_km", "Katz & McCormick stock breakouts", "Highest high · limit"),
        "ftm_km_vol" => ("ftm_km", "Katz & McCormick stock breakouts", "Volatility · not testable"),
        "ftm_rg_oops" => ("ftm_rg_oops", "Ruggiero OOPS", ""),
        "ftm_rg_dip" => ("ftm_rg_dip", "Ruggiero buy the dip", ""),
        "ftm_rg_feedback" => ("ftm_rg_feedback", "Feedback breakout", ""),
        "ftm_cvr9" => ("ftm_cvr9", "Connors VIX Reversal 9", ""),
        "ftm_zg_weak2r" => ("ftm_zg", "Zamansky range expansion", "No ranking"),
        "ftm_zg_ranked" => ("ftm_zg", "Zamansky range expansion", "Ranked · data missing"),
        "ftm_kf_sd" => ("ftm_kf", "Fitschen SD expansion", "Base"),
        "ftm_kf_sd_full" => ("ftm_kf", "Fitschen SD expansion", "Stop + pullback + filter"),
        "ftm_mt_macd" => ("ftm_mt", "Matoba MACD", "MACD alone"),
        "ftm_mt_macd_rsi" => ("ftm_mt", "Matoba MACD", "RSI sizing"),
        "ftm_mt_macd_stoch" => ("ftm_mt", "Matoba MACD", "%D sizing"),
        "ftm_hd_consensus" => ("ftm_hd", "Headley sentiment", ""),
        "ftm_rg_reportday" => ("ftm_rg_report", "Report-day systems", ""),
        "ftm_rg_cot" => ("ftm_rg_cot", "COT model", ""),
        "ftm_rg_intermarket" => ("ftm_rg_im", "Intermarket models", ""),
        "ftm_an_system_e" => ("ftm_an", "Anthonisz Systems E/I", ""),
        "ftm_carr_osc" => ("ftm_carr", "Carr oscillators", ""),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let ses = || vec![p("m", "Breakout look-back M (days)", 20., 5., 60., 5.), p("stop", "Stop, ATR(50) units below entry", 4., 1., 8., 0.5),
                      p("target", "Profit target, ATR(50) units above entry", 10., 2., 20., 1.), p("hold", "Exit at the open of bar entry+N", 20., 5., 40., 1.),
                      p("cool", "Bars after an exit before the next entry", 2., 0., 5., 1.)];
    Some(match id {
        "ftm_km_co_open" | "ftm_km_co_limit" | "ftm_km_hh_open" | "ftm_km_hh_limit" => ses(),
        "ftm_rg_oops" => vec![p("gap", "Gap beyond high/low, x Average(TrueRange,3)", 0.02, 0., 0.5, 0.01), p("ma", "Trend filter MA (closes)", 10., 3., 50., 1.),
                              p("pts", "Entry stop distance from the close (points)", 2., 0., 20., 0.5), p("hold", "Exit at next open once BarsSinceEntry > N", 2., 0., 10., 1.)],
        "ftm_rg_dip" => vec![p("lb", "Look-back (consecutive down closes)", 2., 1., 5., 1.), p("ent", "Entry stop: open + k x Average(Range,3)", 0.4, 0., 1.5, 0.1),
                             p("ext", "Exit stop: open - k x Average(Range,3)", 0.6, 0., 1.5, 0.1), p("rng", "Range average length", 3., 1., 10., 1.)],
        "ftm_rg_feedback" => vec![p("n", "Channel length (bars)", 20., 5., 60., 1.), p("lb", "Feedback look-back LB (bars)", 20., 10., 100., 5.),
                                  p("tick", "MinMove added beyond the channel", 0.05, 0., 1., 0.01), p("vcost", "Virtual cost per trade (points)", 0., 0., 10., 0.5)],
        "ftm_cvr9" => vec![p("n", "VIX high/low look-back", 10., 5., 20., 1.), p("rng", "Range largest of last N days", 3., 2., 5., 1.), p("hold", "Exit on the close after N days", 3., 1., 10., 1.)],
        "ftm_zg_weak2r" => vec![p("k", "Multiple of the 5-day average range below the open (1.5 = main study)", 2., 0.5, 3., 0.25), p("n", "Range average days", 5., 2., 20., 1.),
                                p("hold", "Exit at the Nth open after entry", 5., 1., 20., 1.)],
        "ftm_kf_sd" => vec![p("sdlen", "Standard deviation length (closes)", 50., 10., 100., 5.), p("n", "Comparison look-back (bars)", 22., 5., 60., 1.)],
        "ftm_kf_sd_full" => vec![p("sdlen", "Standard deviation length (closes)", 50., 10., 100., 5.), p("n", "Comparison look-back (bars)", 22., 5., 60., 1.),
                                 p("stop", "Close-only stop, SD units", 1.5, 0.5, 4., 0.25), p("skipwed", "Skip Wednesday signals (1 = book's tweak)", 0., 0., 1., 1.)],
        "ftm_mt_macd" | "ftm_mt_macd_rsi" | "ftm_mt_macd_stoch" => vec![p("s1", "Higher smoothing factor", 0.95, 0.05, 1., 0.05), p("s2", "Lower smoothing factor", 0.90, 0.05, 1., 0.05),
                                 p("osc", "Oscillator length (sized variants)", 10., 2., 30., 1.)],
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
        "ftm_km_co_open" => katz(x, from, to, false, false, u("m").max(2), g("stop"), g("target"), u("hold").max(1), u("cool")),
        "ftm_km_co_limit" => katz(x, from, to, false, true, u("m").max(2), g("stop"), g("target"), u("hold").max(1), u("cool")),
        "ftm_km_hh_open" => katz(x, from, to, true, false, u("m").max(2), g("stop"), g("target"), u("hold").max(1), u("cool")),
        "ftm_km_hh_limit" => katz(x, from, to, true, true, u("m").max(2), g("stop"), g("target"), u("hold").max(1), u("cool")),
        "ftm_rg_oops" => oops(x, from, to, g("gap"), u("ma").max(1), g("pts"), u("hold")),
        "ftm_rg_dip" => dip(x, from, to, u("lb").max(1), g("ent"), g("ext"), u("rng").max(1)),
        "ftm_rg_feedback" => feedback(x, from, to, u("n").max(2), u("lb").max(1), g("tick"), g("vcost")),
        "ftm_cvr9" => cvr9(x, from, to, u("n").max(2), u("rng").max(2), u("hold").max(1))?,
        "ftm_zg_weak2r" => zg(x, from, to, g("k"), u("n").max(1), u("hold").max(1)),
        "ftm_kf_sd" => fitschen(x, from, to, u("sdlen").max(2), u("n").max(1), false, 0., false),
        "ftm_kf_sd_full" => fitschen(x, from, to, u("sdlen").max(2), u("n").max(1), true, g("stop"), g("skipwed") >= 0.5),
        "ftm_mt_macd" => matoba(x, from, to, g("s1"), g("s2"), 0, u("osc").max(2)),
        "ftm_mt_macd_rsi" => matoba(x, from, to, g("s1"), g("s2"), 1, u("osc").max(2)),
        "ftm_mt_macd_stoch" => matoba(x, from, to, g("s1"), g("s2"), 2, u("osc").max(2)),
        _ => return None,
    })
}

/// Katz & McCormick breakout + standard exit strategy (long only).
#[allow(clippy::too_many_arguments)]
fn katz(x: &Ctx, from: usize, to: usize, hl: bool, limit: bool, m: usize, sk: f64, tk: f64, hold: usize, cool: usize) -> Vec<Trade> {
    let atr = atr_mean(x, 50);
    let src: &[f64] = if hl { &x.h } else { &x.c };
    let mut out = vec![];
    let mut cur: Option<(Trade, f64, f64)> = None;
    let mut last_exit: Option<usize> = None;
    for t in m + 1..=to {
        if let Some((tr, st, tg)) = cur.take() {
            let e = tr.legs[0].i;
            let done = if t <= e { None } else if t >= e + hold { Some((x.o[t], "time exit · open of bar 21")) }
                else if x.o[t] <= st { Some((x.o[t], "stop (gap)")) } else if x.l[t] <= st { Some((st, "stop")) }
                else if x.o[t] >= tg { Some((x.o[t], "target (gap)")) } else if x.h[t] >= tg { Some((tg, "target")) } else { None };
            match done {
                Some((px, why)) => { out.push(fin(tr, t, px, t - 1, why)); last_exit = Some(t); }
                None => cur = Some((tr, st, tg)),
            }
        }
        if cur.is_some() || t < from || t >= to || !atr[t].is_finite() { continue; }
        let sig = src[t] > hi(&src[t - m..t]) && !(src[t - 1] > hi(&src[t - m..t - 1]));
        if !sig { continue; }
        let e = t + 1;
        if last_exit.is_some_and(|le| e < le + cool) { continue; }
        let px = if limit {
            let lim = (x.h[t] + x.l[t]) / 2.;
            if x.l[e] > lim { continue; }
            x.o[e].min(lim)
        } else { x.o[e] };
        cur = Some((mk(1., e, px, t, if limit { "limit" } else { "open" }), px - sk * atr[t], px + tk * atr[t]));
    }
    mark_open(&mut out, cur.map(|c| c.0), x, to);
    out
}

/// Ruggiero's coded OOPS (May 2002).
fn oops(x: &Ctx, from: usize, to: usize, gap: f64, ma: usize, pts: f64, hold: usize) -> Vec<Trade> {
    let atr3 = atr_mean(x, 3);
    let sm = sma(&x.c, ma);
    let mut out = vec![];
    let mut cur: Option<Trade> = None;
    for t in 1..to {
        let n = t + 1;
        // exit at the next open once BarsSinceEntry > hold (decided on this bar's close)
        if let Some(tr) = cur.take() {
            if t - tr.legs[0].i > hold { out.push(fin(tr, n, x.o[n], t, "BarsSinceEntry > hold · next open")); } else { cur = Some(tr); }
        }
        if t < from || !atr3[t].is_finite() || !sm[t].is_finite() { continue; }
        let up = x.o[n] > x.h[t] + gap * atr3[t] && x.c[t] > sm[t];
        let dn = x.o[n] < x.l[t] - gap * atr3[t] && x.c[t] < sm[t];
        let fill = if up { let sp = x.c[t] - pts; (x.l[n] <= sp).then(|| (-1., x.o[n].min(sp))) }
                   else if dn { let bp = x.c[t] + pts; (x.h[n] >= bp).then(|| (1., x.o[n].max(bp))) } else { None };
        if let Some((d, px)) = fill {
            match cur.take() {
                Some(tr) if tr.dir == d => { cur = Some(tr); }
                Some(tr) => { out.push(fin(tr, n, px, t, "reversed by opposite entry")); cur = Some(mk(d, n, px, t, "stop")); }
                None => cur = Some(mk(d, n, px, t, "stop")),
            }
        }
    }
    mark_open(&mut out, cur, x, to);
    out
}

/// Ruggiero buy-the-dip (May 2002).
fn dip(x: &Ctx, from: usize, to: usize, lb: usize, ent: f64, ext: f64, rl: usize) -> Vec<Trade> {
    let rng: Vec<f64> = (0..x.r.len()).map(|i| x.h[i] - x.l[i]).collect();
    let ar = sma(&rng, rl);
    let mut out = vec![];
    let mut cur: Option<Trade> = None;
    for t in lb..to {
        let n = t + 1;
        if !ar[t].is_finite() { continue; }
        let mx = (0..lb).map(|k| x.c[t - k] - x.c[t - k - 1]).fold(f64::NEG_INFINITY, f64::max);
        if let Some(tr) = cur.take() {
            let sp = x.o[n] - ext * ar[t];
            if mx > 0. && x.l[n] <= sp { out.push(fin(tr, n, sp.min(x.o[n]), t, "up close in look-back · open - 0.6 x range stop")); } else { cur = Some(tr); }
        } else if t >= from && mx < 0. {
            let bp = x.o[n] + ent * ar[t];
            if x.h[n] >= bp { cur = Some(mk(1., n, bp.max(x.o[n]), t, "stop")); }
        }
    }
    mark_open(&mut out, cur, x, to);
    out
}

/// 20-bar stop breakout filtered by the virtual system's net-profit change (Aug 2003).
fn feedback(x: &Ctx, from: usize, to: usize, n: usize, lb: usize, tick: f64, vcost: f64) -> Vec<Trade> {
    let len = x.r.len();
    let hh: Vec<f64> = (0..len).map(|i| if i + 1 < n { f64::NAN } else { hi(&x.h[i + 1 - n..=i]) }).collect();
    let ll: Vec<f64> = (0..len).map(|i| if i + 1 < n { f64::NAN } else { lo(&x.l[i + 1 - n..=i]) }).collect();
    let (mut vdir, mut vpx, mut vnp) = (0i8, 0., 0.);
    let mut np = vec![0.; len];
    let mut out = vec![];
    let mut cur: Option<Trade> = None;
    // which stop fills first when both are touched on a bar: the one nearer the open
    let both = |t: usize, bs: f64, ss: f64| if (bs - x.o[t]).abs() <= (x.o[t] - ss).abs() { 1i8 } else { -1 };
    for t in n..=to {
        let (bs, ss) = (hh[t - 1] + tick, ll[t - 1] - tick);
        let (hb, hs) = (x.h[t] >= bs, x.l[t] <= ss);
        // virtual always-in system
        let vfill = match vdir {
            1 if hs => Some(-1i8), -1 if hb => Some(1), 0 if hb && hs => Some(both(t, bs, ss)), 0 if hb => Some(1), 0 if hs => Some(-1), _ => None };
        if let Some(d) = vfill {
            let px = if d == 1 { x.o[t].max(bs) } else { x.o[t].min(ss) };
            if vdir != 0 { vnp += vdir as f64 * (px - vpx) - vcost; }
            vdir = d; vpx = px;
        }
        np[t] = vnp;
        // real system: filter from the previous bar's close
        let f = t >= from && t - 1 >= lb + n && np[t - 1] < np[t - 1 - lb];
        match cur.take() {
            Some(tr) if tr.dir > 0. && hs => {
                let px = x.o[t].min(ss);
                out.push(fin(tr, t, px, t - 1, "20-bar low stop"));
                if f { cur = Some(mk(-1., t, px, t - 1, "stop")); }
            }
            Some(tr) if tr.dir < 0. && hb => {
                let px = x.o[t].max(bs);
                out.push(fin(tr, t, px, t - 1, "20-bar high stop"));
                if f { cur = Some(mk(1., t, px, t - 1, "stop")); }
            }
            Some(tr) => cur = Some(tr),
            None if f => {
                let d = if hb && hs { both(t, bs, ss) } else if hb { 1 } else if hs { -1 } else { 0 };
                if d == 1 { cur = Some(mk(1., t, x.o[t].max(bs), t - 1, "stop")); }
                if d == -1 { cur = Some(mk(-1., t, x.o[t].min(ss), t - 1, "stop")); }
            }
            None => {}
        }
    }
    mark_open(&mut out, cur, x, to);
    out
}

/// Connors VIX Reversal 9 (Futures, June 2002).
fn cvr9(x: &Ctx, from: usize, to: usize, n: usize, rn: usize, hold: usize) -> Option<Vec<Trade>> {
    let (vo, vh, vl, vc) = (x.series("VIX.Open")?, x.series("VIX.High")?, x.series("VIX.Low")?, x.series("VIX.Close")?);
    let mut out = vec![];
    let mut busy_until = 0usize;
    for t in n.max(rn).max(from)..to {
        if t < busy_until { continue; }
        let w = t + 1 - n;
        if !(vo[t].is_finite() && vo[t - 1].is_finite() && vh[w..=t].iter().all(|v| v.is_finite()) && vl[w..=t].iter().all(|v| v.is_finite())) { continue; }
        let r = |k: usize| vh[k] - vl[k];
        if !(1..rn).all(|k| vh[t - k].is_finite() && r(t) > r(t - k)) { continue; }
        let buy = vh[t] >= hi(&vh[w..=t]) && vc[t] < vo[t] && vc[t - 1] > vo[t - 1];
        let sell = vl[t] <= lo(&vl[w..=t]) && vc[t] > vo[t] && vc[t - 1] < vo[t - 1];
        let d = if buy { 1. } else if sell { -1. } else { continue };
        let tr = mk(d, t, x.c[t], t, "close");
        let e = t + hold;
        if e > to { mark_open(&mut out, Some(tr), x, to); break; }
        out.push(fin(tr, e, x.c[e], e, "close 3 days after entry"));
        busy_until = e;
    }
    Some(out)
}

/// Buy intraday weakness below the open (Feb 2002, row 4).
fn zg(x: &Ctx, from: usize, to: usize, k: f64, n: usize, hold: usize) -> Vec<Trade> {
    let mut out = vec![];
    let mut t = from.max(n);
    while t <= to {
        let ar = (t - n..t).map(|i| x.h[i] - x.l[i]).sum::<f64>() / n as f64;
        let lvl = x.o[t] - k * ar;
        if x.l[t] <= lvl {
            let tr = mk(1., t, x.o[t].min(lvl), t, "limit");
            let e = t + hold;
            if e > to { mark_open(&mut out, Some(tr), x, to); break; }
            out.push(fin(tr, e, x.o[e], e - 1, "fifth open after entry"));
            t = e; // the exit bar may start the next trade
            continue;
        }
        t += 1;
    }
    out
}

/// Fitschen's standard-deviation expansion system (Jan 2002 sidebar).
#[allow(clippy::too_many_arguments)]
fn fitschen(x: &Ctx, from: usize, to: usize, sdlen: usize, n: usize, full: bool, sk: f64, skipwed: bool) -> Vec<Trade> {
    let sd = stdev_pop(&x.c, sdlen);
    let mut out = vec![];
    let mut cur: Option<(Trade, f64)> = None; // trade, SD at signal
    for t in (sdlen + n).max(2)..to {
        let nx = t + 1;
        if !sd[t - n].is_finite() { continue; }
        let rising = sd[t] > sd[t - n];
        if let Some((tr, s0)) = cur.take() {
            let e = tr.avg_px();
            let stop = full && (if tr.dir > 0. { x.c[t] < e - sk * s0 } else { x.c[t] > e + sk * s0 });
            if sd[t] < sd[t - n] { out.push(fin(tr, nx, x.o[nx], t, "SD below its value N bars ago · next open")); }
            else if stop { out.push(fin(tr, nx, x.o[nx], t, "close beyond 1.5 SD stop · next open")); }
            else { cur = Some((tr, s0)); }
        }
        if t < from || !rising { continue; }
        let mut d = if x.c[t] > x.c[t - n] { 1. } else if x.c[t] < x.c[t - n] { -1. } else { 0. };
        if full {
            let pull = if d > 0. { x.c[t] < x.c[t - 1] } else { x.c[t] > x.c[t - 1] };
            let volf = sd[t] > (sd[t] + sd[t - 1] + sd[t - 2]) / 3.;
            if !pull || !volf || (skipwed && weekday(&x.r[t].date) == 2) { d = 0.; }
        }
        if d == 0. { continue; }
        match cur.take() {
            Some((tr, s0)) if tr.dir == d => cur = Some((tr, s0)),
            Some((tr, _)) => { out.push(fin(tr, nx, x.o[nx], t, "reversed by opposite entry")); cur = Some((mk(d, nx, x.o[nx], t, "open"), sd[t])); }
            None => cur = Some((mk(d, nx, x.o[nx], t, "open"), sd[t])),
        }
    }
    mark_open(&mut out, cur.map(|c| c.0), x, to);
    out
}

/// Matoba's MACD with smoothing factors; mode 0 = always one unit, 1 = RSI size, 2 = %D size.
fn matoba(x: &Ctx, from: usize, to: usize, s1: f64, s2: f64, mode: u8, ol: usize) -> Vec<Trade> {
    let len = x.r.len();
    let pr: Vec<f64> = (0..len).map(|i| (x.h[i] + x.l[i] + x.c[i]) / 3.).collect();
    let ema = |s: f64| { let mut v = vec![0.; len]; for i in 0..len { v[i] = if i == 0 { pr[0] } else { s * pr[i] + (1. - s) * v[i - 1] }; } v };
    let (e1, e2) = (ema(s1), ema(s2));
    let macd: Vec<f64> = (0..len).map(|i| e1[i] - e2[i]).collect();
    let mut out = vec![];
    if mode == 0 {
        let mut cur: Option<Trade> = None;
        for t in from.max(1)..=to {
            let d = if macd[t] > 0. { 1. } else if macd[t] < 0. { -1. } else { 0. };
            if d == 0. || cur.as_ref().is_some_and(|c| c.dir == d) || t == to { continue; }
            if let Some(tr) = cur.take() { out.push(fin(tr, t, x.c[t], t, "MACD sign change · close")); }
            cur = Some(mk(d, t, x.c[t], t, "close"));
        }
        mark_open(&mut out, cur, x, to);
        return out;
    }
    let osc: Vec<f64> = if mode == 1 { rsi(&x.c, ol) } else {
        let k: Vec<f64> = (0..len).map(|i| if i + 1 < ol { f64::NAN } else {
            let (h, l) = (hi(&x.h[i + 1 - ol..=i]), lo(&x.l[i + 1 - ol..=i])); if h > l { 100. * (x.c[i] - l) / (h - l) } else { 50. } }).collect();
        sma(&k, 3)
    };
    for t in from.max(1)..to {
        let d = if macd[t] > 0. { 1. } else if macd[t] < 0. { -1. } else { 0. };
        if d == 0. || !osc[t].is_finite() { continue; }
        let w = (100. - (osc[t] * 2. - 100.).abs()) / 100.;
        if w <= 0. { continue; }
        let mut tr = mk(d, t, x.c[t], t, "close");
        tr.legs[0].w = w;
        out.push(fin(tr, t + 1, x.c[t + 1], t + 1, "re-sized at the close"));
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
    fn dip_buys_after_two_down_closes_on_open_plus_point4_range() {
        let mut r = rows(20);
        r[9].close = 99.5; r[10].close = 99.;            // two down closes ending bar 10
        r[11].open = 99.; r[11].high = 101.;             // stop = 99 + 0.4*2 = 99.8
        r[11].close = 100.5;                              // up close -> exit order for bar 12
        r[12].open = 100.; r[12].low = 98.;               // exit stop = 100 - 1.2 = 98.8
        let x = Ctx::new(&r, &r, &[]);
        let ts = dip(&x, 0, 19, 2, 0.4, 0.6, 3);
        assert_eq!(ts.len(), 1);
        assert_eq!(ts[0].legs[0].i, 11); assert!((ts[0].legs[0].px - 99.8).abs() < 1e-9);
        assert_eq!(ts[0].exit_i, 12); assert!((ts[0].exit_px - 98.8).abs() < 1e-9);
    }
    #[test]
    fn oops_sells_close_minus_two_after_gap_up_and_exits_after_hold() {
        let mut r = rows(40);
        for (i, row) in r.iter_mut().enumerate() { row.close = 100. + i as f64 * 0.1; row.open = row.close; row.high = row.close + 1.; row.low = row.close - 1.; }
        // bar 20 signal: next open gaps above high, close above its 10-day MA (rising series)
        r[21].open = r[20].high + 1.; r[21].low = r[20].close - 3.;
        let x = Ctx::new(&r, &r, &[]);
        let ts = oops(&x, 0, 39, 0.02, 10, 2., 2);
        let t = &ts[0];
        assert_eq!(t.dir, -1.); assert_eq!(t.legs[0].i, 21); assert!((t.legs[0].px - (r[20].close - 2.)).abs() < 1e-9);
        assert_eq!(t.exit_i, 25); assert_eq!(t.exit_px, r[25].open);
    }
    #[test]
    fn catalogue_ids_unique_and_live_have_params() {
        let c = catalogue();
        let mut ids: Vec<_> = c.iter().map(|d| d.id).collect(); ids.sort(); ids.dedup();
        assert_eq!(ids.len(), c.len());
        for d in c.iter().filter(|d| d.status == Status::Live) { assert!(params(d.id).is_some(), "{}", d.id); assert!(family_of(d.id).is_some()); }
    }
}
