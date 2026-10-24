//! Perry J. Kaufman, New Trading Systems and Methods, 4th ed. (Wiley 2005, Books24x7 print, 942 PDF pages)
//! and its companion CD of TradeStation code (TSM4 CD/ChapterNN/TSM(S|F) *.txt). Page refs are PDF pages.
//! Catalogue Live denotes an implemented runner; live API admission requires a separate source review.
//! Seventeen methods reviewed 2026-10-13 have independent actual-data ledger checks. Defaults omitted
//! by the source, fill assumptions and instrument adaptations remain explicit, never source-study claims.
//! Review and independent oracles: library/kaufman/REVIEW-2026-10-13.md,
//! library/kaufman/REVIEW-2026-10-13-ADAPTIVE.md and their scripts/ directory.
use crate::connors::{rsi, sma, true_range, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

/// Admitted 2026-10-24 (authors/kaufman/admission/SOURCE-CONTRACT.md): independent ledgers, US and India, 0 mismatches.
pub const REVIEWED: &[&str] = &["kftsm_parabolic", "kftsm_donchian_4wk", "kftsm_moon", "kftsm_multi_tf"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

pub const BOOK: &str = "kftsm";


pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, dir, status, rules, notes| StrategyDef {
        id, name, group, session: 0, dir, status, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None,
    };
    use Status::*;
    vec![
        // ------------------------------------------------ Ch 8: six major trend systems (Table 8.2, p.278-293)
        d("kftsm_ma", "Moving average trendline direction", "Ch 8 · Comparison of major trend systems", "Long/Short", Live, &[
            "E|\"BUY if MAchange > 0 / SELL if MAchange < 0 (the moving average turns up or down)\" MA = Average(close,n) (Table 8.2, p.278)",
            "E|TSMMAsignal (CD Ch08): change = (ma - ma[1])*100/close[1]; signal = 1 if change[lag] > filter, -1 if change[lag] < -filter, else unchanged; \"if signal = 1 then buy at close; if signal = -1 then sell at close\" (TSM Moving average, inputs period(20), filter(0), lag(0))",
            "X|\"All six systems are always in the market ... they always reverse on a new signal ... no stop-losses\" (p.278)",
        ], "Defaults = CD system inputs period 20, filter 0, lag 0. The book's profile example uses a 63-day period on the Nasdaq 100 (p.263-264, Fig 8.5): run p.period=63 to compare. Orders on the close (TradeStation 'buy at close'). Instrument swap only."),
        d("kftsm_exp", "Exponential smoothing trendline direction", "Ch 8 · Comparison of major trend systems", "Long/Short", Live, &[
            "E|\"EXP=EXP[1]+s*(close-EXP[1]) ... BUY if EXPchange > 0, SELL if EXPchange < 0\" (Table 8.2, p.278)",
            "E|KEXP (printed p.287) / TSMExpSignal (CD): Exp = SmoothedAverage(close,period); change = (Exp - Exp[1])*100/close[1]; buy/sell at close when change[lag] > filter / < -filter",
            "X|Always in the market, reverse on the opposite signal, no stops (p.278)",
        ], "The CD calls TradeStation SmoothedAverage; this research convention seeds its first full value with the n-bar mean, then uses (prev*(n-1)+close)/n. The supplied call does not specify platform MaxBarsBack or initialization history, so this convention is declared rather than certified platform parity. p.kind=1 uses s = 2/(n+1) (the Ch 7 conversion, p.278 'when 2/(n + 1) is used'), p.kind=2 uses s = 1/(n+1) (the CD spreadsheet code, TSM(E) Trend systems.txt). Period: the CD system input is 0 (must be set) and the book gives no single value; 20 is used, the companion MA/LRS/NDB CD default [book silent]."),
        d("kftsm_lrs", "Linear regression slope", "Ch 8 · Comparison of major trend systems", "Long/Short", Live, &[
            "E|\"LRS = (n*Sum(x*c) - Sum(x)*Sum(c)) / (n*Sum(x*x) - Sum(x)*Sum(x)) ... BUY if LRS > 0, SELL if LRS < 0\" (Table 8.2, p.278)",
            "E|TSMLRslope + TSMLRSsignal (CD Ch08; printed KLRS/KLRSignal p.287-288): buy/sell at close when slope[lag] > filter / < -filter. TSM LinReg slope inputs period(20), filter(0), lag(0)",
            "X|Always in the market, no stops (p.278)",
        ], "Defaults = CD inputs. x = 1..n, newest bar = n."),
        d("kftsm_ndb", "N-day breakout (high/low with close confirmation)", "Ch 5/8 · N-day breakout", "Long/Short", Live, &[
            "E|\"BUY when high is above N-day high and close > prior close; SELL when low is below N-day low and close < prior close\" (TSMNDBsignal, CD Ch05/Ch08; Table 8.2 p.278)",
            "E|CD code: nhigh = highest(high,period)[1], nlow = lowest(low,period)[1]; \"if signal = 1 then buy at close\" (TSM N-Day Breakout / TSM N-Day Breakout 2, period(20))",
            "X|Always in the market, reverses on the opposite signal (p.278)",
        ], "Default window = the CD code (previous N bars), N = 20 (TSM N-Day Breakout 2 input). The printed Table 8.2 and KNDB listing (p.278, p.288) use highest(high[1], n-1), i.e. the previous N-1 bars: p.printed=1. Table 5.11 (p.178-179) tested this system on stocks and indices for the 5 years ending 30 May 2003: SPX best N=88 (4 trades, 2 profitable), Nasdaq 100 best N=40 (10 trades, 5 profitable), DJIA N=1 (331 trades, 129 profitable)."),
        d("kftsm_swg", "Swing breakout (KSWG)", "Ch 8 · Comparison of major trend systems", "Long/Short", Live, &[
            "E|\"swhigh is the highest price that occurs before a price decline of p * swhigh ... BUY if high > swhigh, SELL if low < swlow\" (Table 8.2, p.278)",
            "E|KSWG (printed p.288-289) = TSMSWGsignal (CD Ch05): minswing = swing*close/100; swing points fixed after a reversal of minswing; signal = -1 while in a down leg if curlow < swlow, +1 while in an up leg if curhigh > swhigh; buy/sell at close",
            "X|Always in the market (p.278)",
        ], "Swing % default 2.5: the TSM Swing System input is 0 (must be set) and the book prints no single value; 2.5 is the book's Ch 5 TSM Swing default [book silent]. No signal until the first swing high/low has been fixed (the code's zero initial swing values would otherwise fire on bar 1)."),
        d("kftsm_swing", "Percent swing method (TSM Swing)", "Ch 5 · Swing trading", "Long/Short", Live, &[
            "E|\"if marketposition <= 0 and xhigh > swhigh then buy on close else if marketposition >= 0 and xlow < swlow then sell on close\" evaluated before today's swing update (TSM Swing, CD Ch05, swing(2.5))",
            "F|A swing high is fixed when the low falls below curhigh - curhigh*swing%; a swing low when the high rises above curlow + curlow*swing% (favour reversals) (CD Ch05; p.147-150)",
            "X|Reverse on the opposite breakout",
        ], "Default swing 2.5% = CD input. Orders are skipped until the first swing high/low exists (code initial value 0 would buy on bar 1)."),
        d("kftsm_pf", "Point-and-figure breakout", "Ch 5 · Point-and-figure", "Long/Short", Live, &[
            "E|\"a buy occurs when new prices rise above the high of the previous high column, and a sell signal occurs when the new prices fall below the low of the previous low column\" (TSM Point & Figure, CD Ch05)",
            "F|Boxes = intportion(price/box); reversal = revboxes (3) boxes (CD code; p.155-175)",
            "F|Box size: Table 5.2 (p.160) 'Point-and-Figure Box Sizes—Futures 2003': e-mini SP 5, e-mini NQ 5, 3-box reversal",
            "X|Always in the market: a position is closed only by the opposite signal, which reverses it (the code's only exit; no stop or target is given)",
        ], "Default box = 5 index points (Table 5.2 value for the S&P and Nasdaq e-minis); the book gives no box for Indian indices, so NIFTY runs use the same 5 points unless changed. The CD code's quirks are kept literally (initial column from close >= (high-low)/2; 'if mod(xlow,box)=0 then boxlow = boxlow - 1')."),
        // ------------------------------------------------ Ch 8: bands and other single-trend systems (CD Ch08)
        d("kftsm_bb_ma", "Moving average with Bollinger bands", "Ch 8 · Bands and channels", "Long/Short", Live, &[
            "E|\"if close crosses above bandup then buy next bar market; if close crosses below banddn then sell next bar market\" bandup = BollingerBand(close,20,2) (TSM MA BBands, CD Ch08; rules p.269)",
            "X|\"if marketposition > 0 and close crosses below ma then exitlong next bar market\" (short mirror) (CD; p.269 'Close out longs when prices reverse and go below the moving average')",
        ], "Defaults period 20, nsd 2 (CD inputs; p.268 'two standard deviations ... over the past 20 periods'). TradeStation StdDev is the population standard deviation. Fills at the next open."),
        d("kftsm_fc_bb_abx", "FC Bollinger ABX (two bands, profit-taking)", "Ch 8 · Bands and channels", "Long/Short", Live, &[
            "E|\"IF CLOSE CROSSES ABOVE outupper THEN BUY 2 CONTRACTS NEXT BAR AT MARKET\" outupper = BollingerBand(close,21,2); short mirror (TSM FC Bollinger ABX, F. Cavasino, CD Ch08)",
            "X|\"IF CLOSE CROSSES BELOW inupper THEN EXITLONG 1 CONTRACT NEXT BAR AT MARKET\" inupper = BollingerBand(close,21,1)",
            "X|\"IF CLOSE CROSSES BELOW mavg THEN EXITLONG 1 CONTRACT NEXT BAR AT MARKET\" mavg = Average(close,21)",
        ], "CD length21, outer2 SD, inner1 SD. Two contracts are represented by equal halves of a single research allocation; this is not contract-dollar accounting. No pyramiding. Inner and midpoint crossings each close one remaining half at the next open; simultaneous crossings close both. An opposite entry closes any remainder and starts two new halves at that open. A terminal remainder is marked, not liquidated; its final exit piece is a mark identified by open=true. A final-close signal has no next-open fill. Independent selected-baseline ledger scope: library/kaufman/REVIEW-2026-10-13-ADAPTIVE.md."),
        d("kftsm_ma_pct_band", "Moving average with percent bands", "Ch 8 · Bands and channels", "Long/Short", Live, &[
            "E|\"if close > ma + band then buy on close; if close < ma - band then sell on close\" band = ma*pcband/100 (TSM MA percent bands, CD Ch08, period(20), pcband(2.0)); p.267-268",
            "X|Reversal: always in once started",
        ], "Defaults = CD inputs."),
        d("kftsm_ma_tr_band", "Moving average with true-range bands", "Ch 8 · Bands and channels", "Long/Short", Live, &[
            "E|\"ma = average(close[1],period); avgtr = average(trange[1],period); if close > ma + avgtr*band then buy on close; if close < ma - avgtr*band then sell on close\" (TSM MA TR bands, CD Ch08, period(20), band(5))",
            "X|Always in the market: a position is closed only by the opposite signal, which reverses it (the code's only exit; no stop or target is given)",
        ], "Defaults = CD inputs (the description says band < 1.0 for a fast system; the coded default 5 is kept)."),
        d("kftsm_triple_smooth", "Triple exponential smoothing", "Ch 8 · Triple smoothing", "Long/Short", Live, &[
            "E|\"savg1 = savg1[1] + (2/(length1+1))*(close - savg1[1]); savg2 ... of savg1; savg3 ... of savg2; if savg3 > savg3[1] then buy on close else if savg3 < savg3[1] then sell on close\" (TSM Triple Smoothing, CD Ch08, lengths 20, 10, 5)",
            "X|Always in the market: a position is closed only by the opposite signal, which reverses it (the code's only exit; no stop or target is given)",
        ], "Seeds: all three = first close (code). Hutson's TRIX variant on p.273 (log prices, same constant, two-day rule) gives no constant: not coded."),
        d("kftsm_vol_breakout", "Volatility system (Bookstaber)", "Ch 8/20 · Volatility system", "Long/Short", Live, &[
            "E|\"Buy if the close rises by more than k × V(n)t-1 from the previous close. Sell if the close drops by more than k × V(n)t-1\" V = average true range (p.273-274; p.656-657)",
            "E|\"The value of k is generally about 3.0.\" \"the current price change is compared to the previous volatility calculation\" (p.273-274)",
            "X|Reverse on the opposite signal (TSM Volatility Bkout, CD Ch08/Ch20)",
        ], "Defaults length 20, factor 3.0 (CD Ch08 inputs; text k about 3.0), volatility of the previous bar (text and CD Ch20). p.vlag=0 = the CD Ch08 code, which includes today's true range; the Ch20 CD copy uses factor 1.4 (p.factor=1.4)."),
        d("kftsm_raschke_first_cross", "Raschke First Cross", "Ch 8 · Raschke's First Cross", "Long/Short", Live, &[
            "F|osc = MA(3) - MA(10), trend = MA(osc,16); after trend crosses above 0 (below 0) wait for the first pullback (TSM Raschke 1st Cross, CD Ch08; p.276)",
            "E|\"if owait = 0 and fastosc[1] > trend and fastosc <= trend then owait = -1; if owait = -1 and low > low[1] then buy on close\" (short mirror with high < high[1])",
            "X|\"X1. close t-1 > close t-2 close of bar turns up (confirms trend) X2. close t < close t-1 X3. exit long\" (p.276) = CD stage logic",
        ], "CD defaults3/10/16, vfilter0; unused nvlty is omitted because its volatility test is commented out. The printed S1 typo is resolved by the CD >= comparison. Exit is Kaufman's suggested completion, not an exit supplied by Raschke. State comparisons use the position before today's close orders; a simultaneous opposite entry and old-position exit resolve as one reversal at that same close. Flat at the selected start; prior bars warm setup state. Only selected-baseline source-rule/ledger verification is claimed."),
        d("kftsm_keltner", "Keltner 10-day moving average rule", "Ch 8 · Applications of single trends", "Long/Short", Live, &[
            "E|\"a 10-day moving average based on the average of the daily high, low, and closing prices, with a band on each side formed from the 10-day moving average of the high-low range. A buy signal occurs on penetration of the upper band and a sell signal when the lower band is broken; positions are always reversed.\" (p.273)",
            "F|Penetration = close: \"Buy (close out shorts and go long) when the price penetrates (closes above) the upper band\" (Rules for using bands, p.269)",
            "X|Always in the market: a position is closed only by the opposite signal, which reverses it (the code's only exit; no stop or target is given)",
        ], "Bands from today's 10-day values, signal on the close, order at the close (book's convention for close-based rules, p.263)."),
        d("kftsm_parabolic", "Parabolic Time/Price System (Wilder)", "Ch 8/17 · Parabolic", "Long/Short", Live, &[
            "E|\"always in the market; whenever a position is closed out, it is also reversed ... A reversal will occur on a new intraday low that penetrates the SAR\" (p.274-275)",
            "F|SAR = SAR[1] + AF*(extreme - SAR[1]); AF starts .02, +.02 on each new extreme, max .20; SAR never inside the last two bars' range (p.274-275; TSMParabolic, CD Ch17)",
            "X|Always in the market: a position is closed only by the opposite signal, which reverses it (the code's only exit; no stop or target is given)",
        ], "Coded from TSMParabolic (CD). Stop fills at yesterday's SAR, or at the open if the bar gaps through it [book silent on fill]. Flat until the first reversal inside the window."),
        // ------------------------------------------------ Ch 4/5
        d("kftsm_outside_day", "Outside day with an outside close", "Ch 4 · Outside days", "Long/Short", Live, &[
            "E|\"1. Buy on the close of an outside day if the close is above the prior high; sell if the close is below the prior low.\" (Arnold, p.125)",
            "X|\"2. If buying, place a stop-loss just below the low of the outside day; if selling, place the stop just above the high.\" (p.125)",
            "X|\"3. Close out the position on the close three days after entry.\" (p.125)",
        ], "Reviewed baseline follows PDF125: length1, hold3, outside-day extreme stop. Entry is at the completed outside day's close; its stop first becomes active next day, so the entry day's earlier low/high cannot stop it. The omitted tick offset is explicitly adapted to strict penetration with boundary fill, or observed open if gapped. Stop precedes the later close/time orders. Same-direction signals do not pyramid or reset the holding clock/stop; opposite close signals reverse. length/hold/stop_ref variations are research variants: stop_ref1 changes the boundary to the prior range, but does not claim literal CD parity (CD2/2 also resets entrybar on repeated signals)."),
        d("kftsm_donchian_4wk", "Donchian Four-Week Rule", "Ch 5 · N-day breakout", "Long/Short", Live, &[
            "E|\"1. Go long (and cover shorts) when the current price exceeds the highs of the previous four full calendar weeks.\" (p.177)",
            "E|\"2. Sell short (and liquidate longs) when the current price falls below the lows of the previous four full calendar weeks.\" (p.177)",
            "X|Always in the market: a position is closed only by the opposite signal, which reverses it (the code's only exit; no stop or target is given)",
        ], "Calendar weeks Monday-Sunday; the four weeks before the current week. Stop entry at the level, or the open if gapped. Rule 3 (futures roll) does not apply to an index. If a flat bar breaks both levels the bar is skipped [order unknown]. Differs from cov_tc_4week_reversal (20 trading days, Covel) by using calendar weeks."),
        // ------------------------------------------------ Ch 17 adaptive (TSM Adaptive systems driver)
        d("kftsm_kama", "KAMA adaptive moving average system", "Ch 17 · Adaptive techniques", "Long/Short", Live, &[
            "F|KAMA = KAMA[1] + sc*(close - KAMA[1]); sc = [ER*(fastest - slowest) + slowest]^2, fastest = 2/(2+1), slowest = 2/(30+1), ER over 10 periods (p.603-605; TSMKAMA CD Ch17: fastend .666, slowend .0645)",
            "E|\"if currentbar > period+5 and absvalue(change) > sdfilter*SD and absvalue(trend - trend[1]) > mfilter then: if trend > trend[1] and signal <> 1 then buy on close; if trend < trend[1] and signal <> -1 then sell on close\" change = trend - trend[1], SD = stddev(change[1],20) (TSM Adaptive systems, CD Ch17, system(1), period(10), mfilter(0), sdfilter(.1))",
            "X|Always in the market: a position is closed only by the opposite signal, which reverses it (the code's only exit; no stop or target is given)",
        ], "CD driver defaults period10, sdfilter.1, mfilter0; CD function constants.666/.0645, not rounded prose replacements. First observed close seed; unavailable initial ER history uses ER1. The CD assumes platform prehistory/MaxBarsBack that is not supplied: this initial-history convention is declared. Zero noise retains the CD ER1. Filter SD is population SD of the previous20 trend changes, excluding today's change. Nonzero mfilter is an instrument-local point research threshold, not a converted USD amount. Selected baselines only, no original-study reproduction."),
        d("kftsm_adapt_exp", "Adaptive driver: exponential benchmark", "Ch 17 · Adaptive techniques", "Long/Short", Live, &[
            "F|\"system = 0, exponential smoothing benchmark\": trend = SmoothedAverage(close,period) (TSM Adaptive systems, CD Ch17)",
            "E|Same standard-deviation-filtered direction rules as kftsm_kama",
            "X|Always in the market: a position is closed only by the opposite signal, which reverses it (the code's only exit; no stop or target is given)",
        ], "CD driver period10, sdfilter.1, mfilter0. SmoothedAverage has no supplied platform initialization: first full-period mean and alpha1/period are the declared engine convention. Previous20 trend changes supply the population SD filter. This filtered driver is distinct from the unfiltered Chapter8 benchmark. Selected-baseline verification only."),
        d("kftsm_vidya", "Adaptive driver: Chande VIDYA", "Ch 17 · Adaptive techniques", "Long/Short", Live, &[
            "F|VIDYA = k*sc*close + (1 - k*sc)*VIDYA[1], k = stddev(close,period)/stddev(close,histper), sc = 2/(period+1) (TSMVIDYA, CD Ch17; p.606)",
            "E|system = 3, TSMVIDYA(period, per2) with the kftsm_kama direction rules (TSM Adaptive systems, period(10), per2(50), sdfilter(.1))",
            "X|Always in the market: a position is closed only by the opposite signal, which reverses it (the code's only exit; no stop or target is given)",
        ], "CD driver period10, per2=50, sdfilter.1, mfilter0. Full historical volatility window is required; first complete window seeds with its observed close and earlier values stay unavailable. Later zero historical variance carries the prior trend without a signal (declared 0/0 convention). No volatility-ratio clipping. histper must exceed period. The CD has no MaxBarsBack settings, so initial-history parity with TradeStation is not certified. Selected-baseline verification only."),
        d("kftsm_adapt_r2", "Adaptive driver: correlation R2 trend", "Ch 17 · Adaptive techniques", "Long/Short", Live, &[
            "F|AR2 = AR2[1] + R2*(close - AR2[1]), R2 = power(correlation(currentbar, close, period), 2) (TSMAdaptiveR2, CD Ch17)",
            "E|system = 2 with the kftsm_kama direction rules (TSM Adaptive systems, period(10), sdfilter(.1))",
            "X|Always in the market: a position is closed only by the opposite signal, which reverses it (the code's only exit; no stop or target is given)",
        ], "CD function uses squared Pearson price/time correlation, not the printed (r+1)/2 or residual-series variant on PDF608. Until a full correlation window exists, observed closes seed the trend; zero price variance uses R2=0 and carries the prior trend. These unavailable-history/degeneracy conventions are explicit; original MaxBarsBack parity is not certified. Driver defaults10/.1/0 and previous20-change population SD filter. Selected-baseline verification only."),
        d("kftsm_adapt_rsi", "Adaptive driver extension: Wilder RSI trend", "Ch 17 · Adaptive techniques", "Long/Short", Live, &[
            "F|TSMAdaptiveRSI (CD Ch17): sc = absvalue(RSI(close,period)/100 - .5)*2; ARSI = ARSI[1] + sc*(close - ARSI[1]); first bar ARSI=close (PDF608-609)",
            "E|The CD Adaptive systems introduction explicitly describes adding Adaptive RSI by inserting its function call into the driver. Apply the same prior20-change SD filter and close direction orders as the supplied four systems.",
            "X|Close reversal on the next opposite eligible trend change; no added stop, target, sizing or portfolio rule.",
        ], "Author-described driver extension, not a fifth selector supplied in the CD system. Configurable period10/sdfilter.1/mfilter0 inherit the driver defaults; the printed indicator uses20. Reuse engine Wilder RSI: first n actual changes seed gain/loss means; flat gain/loss gives RSI50. ARSI starts at the first close and carries until RSI exists. These absent-platform-history/zero-loss conventions are explicit, not TradeStation initialization parity. Selected-baseline verification only."),
        // ------------------------------------------------ Ch 21 Hochheimer studies and CD crossovers
        d("kftsm_hh_ma", "Hochheimer single moving average (close penetration)", "Ch 21 · Comprehensive studies", "Long/Short", Live, &[
            "E|\"2. A buy signal occurred on an upwards penetration of the moving average by the closing price; a sell signal occurred on a downward penetration\" \"1. The trend calculation used the closing price only.\" (p.747)",
            "X|\"3. The model is always in the market.\" (p.747)",
            "F|\"4. A trade cannot be executed when the day's high and low price are the same.\" (p.747)",
        ], "No single period is given: Table 21.5 (p.748) lists the best period per market; default 35 = the S&P 500 row (1986-1996: 200 trades, 68 profitable, 34%) [fitted to the book's own table]. Orders at the close on a crossing event; a cross on a high=low day is ignored, without a delayed fill. This research window starts flat and waits for its first eligible crossing, rather than inventing a pre-window position."),
        d("kftsm_hh_cross", "Hochheimer two moving average crossover", "Ch 21 · Comprehensive studies", "Long/Short", Live, &[
            "E|\"2. A buy signal occurred when the short-term average moved above the long-term average; a sell signal occurred when the short-term average moved below the long term.\" (p.749)",
            "X|\"3. The model was always in the market.\" \"4. If the high and low prices of the day were equal ... no trading signals were generated.\" (p.749)",
        ], "Default 17/25 = Table 21.9 S&P 500 row (1996-2003: 154 trades, 42 profitable); Nasdaq 100 row 13/60 (59 trades, 22 profitable) (p.752). Orders at the close on a crossing event; a cross on a high=low day is ignored, without a delayed fill. The selected research window starts flat until its first eligible crossing."),
        d("kftsm_hh_channel", "Hochheimer closing-price channel (N-day breakout)", "Ch 21 · Comprehensive studies", "Long/Short", Live, &[
            "E|\"The penetration, which causes a buy or sell signal, occurs if the daily price closes above or below the high or low, respectively, of the previous N days.\" (p.752)",
            "X|Always in (reversal), as the other Hochheimer models",
        ], "Default N = 100: Table 21.11 (1997-2003) best period for both the Nasdaq 100 (8 trades, 4 profitable) and the S&P 500 (10 trades, 4 profitable); Dow best 35 (20 trades, 7 profitable) (p.753)."),
        d("kftsm_mod_3ma", "Modified 3 moving average crossover", "Ch 21 · Modified 3-crossover", "Long/Short", Live, &[
            "E|\"if ma2 > ma1 and ma3 > ma3[1] then buy at close; if ma2 < ma1 and ma3 < ma3[1] then sell at close\" (TSM Mod 3MA cross, CD Ch21 and p.754, length1(20), length2(5), length3(2))",
            "F|\"Do not enter a new long (or short) position unless the slope of the confirming moving average ... was up (or down).\" (p.754)",
            "X|Always in the market: a position is closed only by the opposite signal, which reverses it (the code's only exit; no stop or target is given)",
        ], "Defaults = code inputs."),
        d("kftsm_4_9_18", "4-9-18 crossover", "Ch 21 · 4-9-18 crossover", "Long/Short", Live, &[
            "E|\"This model, using the same rules as the Modified 3-Crossover System\" with 4, 9 and 18 days (p.754-756): ma1 = 18-day, ma2 = 9-day crossover, ma3 = 4-day confirming slope",
            "X|Always in the market: a position is closed only by the opposite signal, which reverses it (the code's only exit; no stop or target is given)",
        ], "Mapping: the confirming average must be 'equal to or faster than the fastest' (p.754), so 4 confirms the 9/18 crossover. Table 21.13 (1996-2003): Nasdaq 100 79 trades/28 profitable, S&P 500 84/36, Dow 58/23 (p.756)."),
        // ------------------------------------------------ Ch 14, 19, 10
        d("kftsm_moon", "Moon trading (countertrend at new/full moon)", "Ch 14 · Behavioral · The Moon", "Long/Short", Live, &[
            "E|\"A countertrend position is set if the fast stochastic is overbought or oversold and there is a new or full moon\": sell on close if fastk(14) >= 80, buy if <= 20, on a moon day (TSM Moon Trading, CD Ch14)",
            "X|\"if stoch >= 100 - exit then exitlong\" (exit 50); \"if holddays > 0 and ndays >= holddays then exitlong\" (holddays 5); \"if moon = 1 then exitlong\" / \"if moon = -1 then exitshort\" (CD)",
            "F|New/full moon: JDE = 2451550.09765 + 29.530588853k + 0.0001337T^2 - 0.000000150T^3 + 0.00000000073T^4 (k+.5 for full), converted to a calendar date; the moon bar is the first bar dated after the moon's date (CD; p.526)",
        ], "Defaults = CD inputs (fstoch 14, overbot 80, oversold 20, exit 50, holddays 5). The code's start-up indexing can skip all new moons depending on the chart's first date; every new and full moon is used here (the program's stated intent)."),
        d("kftsm_multi_tf", "Weekly regression slope + daily stochastic", "Ch 19 · Multiple time frames", "Long/Short", Live, &[
            "E|\"long signal: slope must be up and stochastic below low threshold\": slope = LinearRegSlope(weekly close, 50) > 0 and slowk(5) daily < 20 -> buy on close; short mirror with > 80 (TSM Multiple Periods, CD Ch19)",
            "X|\"if slope > 0 then exitshort on close; if slope < 0 then exitlong on close\" (CD)",
        ], "Defaults fast 5, slow 50, 20/80 (CD). Weekly bars are built from the daily data; a week counts once its last bar (Friday or later) has closed, otherwise from the next week's first bar. SlowK = 3-bar average of FastK (TradeStation 'original'; the 2000i legacy smoothing may differ slightly)."),
        d("kftsm_month_end", "Month-end effect", "Ch 10 · Seasonality and the stock market", "Long", Live, &[
            "E|\"buys on the last day of the month, or the second to the last day\" (Merrill, Fosback, Freeburg; p.374)",
            "X|\"then exits the trade on the fourth trading day of the new month\" (p.374)",
        ], "Fills on the close [time of day not given]. p.day=1 buys on the second-to-last day. Last trading day of a month = the next bar falls in a new month (the exchange calendar is known in advance)."),
        d("kftsm_hirsch", "Hirsch November-April", "Ch 10 · Seasonality and the stock market", "Long", Live, &[
            "E|\"He simply bought on the first day of November\" (p.375)",
            "X|\"and sold on the last day of April, holding the position for six months.\" (p.375)",
        ], "Fills on the close of the first/last trading day [time of day not given]."),
        // ------------------------------------------------ Needs data
        d("kftsm_first_hour", "First-hour breakout", "Ch 16/19 · Day trading", "Long/Short", NeedsData("10-minute and 60-minute intraday bars (opening-hour high/low)"), &[
            "E|\"if close[1] < Sess1FirstBarHigh then buy at Sess1FirstBarHigh + 20 point stop\" (TSM 1stHour Breakout, CD Ch16)",
            "X|exit long when high >= first-hour low + 10-day average range (CD)",
        ], "Intraday."),
        d("kftsm_zones", "Jackson trading zones", "Ch 18 · Zones", "Long/Short", NeedsData("10-minute intraday bars"), &[
            "E|\"if low <= low1 and low[1] > low1 then buy at close; exitlong at low2 stop; exitlong at midpt limit\" zones from yesterday's daily bar (TSM Zones, CD Ch18)",
        ], "DATA1 is 10-minute data."),
        d("kftsm_event_trading", "Economic event trading (Warwick)", "Ch 14 · Event trading", "Long/Short", NeedsData("dated economic releases (durable goods, CPI, PPI, NAPM, GDP, employment) with expectations"), &[
            "E|\"Buy on the close if the market closes in the upper 20% of the trading range on an event day, and if prices have shown an historic continuation pattern\" (p.483)",
            "X|\"Hold for a predetermined number of days\" \"Use a stop-loss\" (p.483)",
        ], "Holding period and stop are left to the trader as well: also not fully specified."),
        // ------------------------------------------------ Not testable
        d("kftsm_mptdi", "MPTDI step-weighted average", "Ch 8 · Applications of single trends", "Long/Short", NotTestable("Table 8.1 gives steps only for gold in points; no table for any index"), &["F|Table 8.1 MPTDI variables for gold (p.272)"], "Futures-specific table."),
        d("kftsm_mart", "Mart's Master Trading Formula", "Ch 8 · Applications of single trends", "Long/Short", NotTestable("the smoothing-constant lookup table and band formula are not printed"), &["E|\"look up a smoothing constant in a modified log-scale table\" (p.276)"], ""),
        d("kftsm_trix", "Hutson triple smoothing (TRIX)", "Ch 8/9 · Triple smoothing", "Long/Short", NotTestable("smoothing constant not given (only 'less than a 20-day equivalent')"), &["E|\"A buy signal was generated when the triple-smoothed trendline rose for two consecutive days\" (p.273)"], ""),
        d("kftsm_weekly_channel", "Weekly price channel (Hochheimer)", "Ch 5 · N-day breakout", "Long/Short", NotTestable("N weeks not given"), &["E|\"Buy if the closing price on Friday exceeds the highest closing price of the past N weeks\" (p.178)"], ""),
        d("kftsm_4wk_final3m", "Four-Week Rule, final three months", "Ch 5 · N-day breakout", "Long/Short", NeedsData("individual futures contracts with delivery months"), &["E|Plot the four-week high/low from 3 months before delivery, 2.5% stop, exit the last day of the month before delivery (p.177)"], ""),
        d("kftsm_dunnigan", "Dunnigan thrust / one-way formula", "Ch 4 · Dunnigan", "Long/Short", NotTestable("'predetermined percentage' and thrust size per market not given"), &["E|Two of four patterns followed by a thrust (p.121-122)"], ""),
        d("kftsm_nofri", "Nofri congestion-phase system", "Ch 4 · Nofri", "Long/Short", NotTestable("congestion area needs judgement ('almost unchanged', 'adequate time has elapsed')"), &["E|In a congestion range after two closes in the same direction take the opposite position on the close; exit the next close (p.124)"], ""),
        d("kftsm_regression_channel", "Regression channel breakout", "Ch 4 · Channel breakout", "Long/Short", NotTestable("minimum swing value (MSV) and 'within 15%' entries not fixed"), &["E|Trend turns when the close breaks the projected regression channel (p.132-133)"], ""),
        d("kftsm_moving_channel", "Moving channel (midpoint + range)", "Ch 4 · Moving channels", "Long/Short", NotTestable("n and the scaling factor w are not given"), &["E|Long when P > M + (M - M[1]) + wR (p.135)"], ""),
        d("kftsm_momentum_fade", "Momentum extremes fade", "Ch 9 · Momentum", "Long/Short", NotTestable("bands 'selected visually'; no span or threshold given"), &["E|Aggressive / minor / major confirmation entries; exit when momentum reaches zero (p.315-316)"], ""),
        d("kftsm_macd_threshold", "MACD crossover with thresholds", "Ch 9 · MACD", "Long/Short", NotTestable("thresholds 'based on observing the MACD movement for less than one year' on one chart"), &["E|Sell only after MACD above +2.00, buy only below -2.00 (p.318-319)"], ""),
        d("kftsm_holiday", "Pre-holiday strategy (Fosback)", "Ch 10 · Seasonality", "Long", NotTestable("'major holiday' list not defined"), &["E|\"bought two days prior to a major holiday and exited on the day prior to the holiday\" (p.374)"], ""),
        d("kftsm_january_barometer", "January barometer", "Ch 10 · Seasonality", "Long/Short", NotTestable("Table 10.15 gives an expectation (bullish/bearish), not entry and exit orders"), &["F|First 5 days and January direction -> expectation for Feb-Dec (p.375-376)"], ""),
        d("kftsm_directional_parabolic", "Directional Parabolic System", "Ch 9 · Directional parabolic", "Long/Short", NotTestable("book: 'The revised rules are unclear'"), &["E|If ADX is up take only long Parabolic trades (p.341)"], ""),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    // Reviewed source-distinct systems each count once. Chapter headings remain
    // presentation groups; a threshold, parameter set or extra market run does
    // not create another method. Pending systems keep their historical groups.
    let reviewed = match id {
        "kftsm_ma" => Some(("kftsm_ma", "Moving average trendline direction", "")),
        "kftsm_exp" => Some(("kftsm_exp", "Exponential smoothing trendline direction", "")),
        "kftsm_lrs" => Some(("kftsm_lrs", "Linear regression slope", "")),
        "kftsm_ndb" => Some(("kftsm_ndb", "N-day breakout with close confirmation", "")),
        "kftsm_bb_ma" => Some(("kftsm_bb_ma", "Moving average with Bollinger bands", "")),
        "kftsm_ma_pct_band" => Some(("kftsm_ma_pct_band", "Moving average with percent bands", "")),
        "kftsm_ma_tr_band" => Some(("kftsm_ma_tr_band", "Moving average with true-range bands", "")),
        "kftsm_triple_smooth" => Some(("kftsm_triple_smooth", "Triple exponential smoothing", "")),
        "kftsm_vol_breakout" => Some(("kftsm_vol_breakout", "Volatility breakout", "")),
        "kftsm_keltner" => Some(("kftsm_keltner", "Keltner ten-day moving average rule", "")),
        "kftsm_hh_ma" => Some(("kftsm_hh_ma", "Hochheimer single moving average", "")),
        "kftsm_hh_cross" => Some(("kftsm_hh_cross", "Hochheimer two-average crossover", "")),
        "kftsm_hh_channel" => Some(("kftsm_hh_channel", "Hochheimer closing-price breakout", "")),
        "kftsm_mod_3ma" => Some(("kftsm_mod_3ma", "Modified three-average confirmation", "")),
        "kftsm_4_9_18" => Some(("kftsm_4_9_18", "Four-nine-eighteen confirmation", "")),
        "kftsm_month_end" => Some(("kftsm_month_end", "Month-end effect", "")),
        "kftsm_hirsch" => Some(("kftsm_hirsch", "Hirsch November–April", "")),
        "kftsm_kama" => Some(("kftsm_kama", "KAMA with CD change filter", "")),
        "kftsm_adapt_exp" => Some(("kftsm_adapt_exp", "Exponential benchmark with CD change filter", "")),
        "kftsm_vidya" => Some(("kftsm_vidya", "VIDYA with CD change filter", "")),
        "kftsm_adapt_r2" => Some(("kftsm_adapt_r2", "Squared-correlation adaptive driver", "")),
        "kftsm_adapt_rsi" => Some(("kftsm_adapt_rsi", "Wilder-RSI adaptive driver extension", "")),
        "kftsm_swg" => Some(("kftsm_swg", "Close-scaled swing breakout", "")),
        "kftsm_swing" => Some(("kftsm_swing", "Extremum-scaled percent swing", "")),
        "kftsm_raschke_first_cross" => Some(("kftsm_raschke_first_cross", "First Cross with Kaufman exit", "")),
        "kftsm_outside_day" => Some(("kftsm_outside_day", "Outside day with outside close", "")),
        "kftsm_fc_bb_abx" => Some(("kftsm_fc_bb_abx", "Two-band half-position profit taking", "")),
        _ => None,
    };
    if reviewed.is_some() { return reviewed; }
    Some(match id {
        "kftsm_ma" => ("kftsm_trend6", "Six major trend systems", "Moving average"),
        "kftsm_exp" => ("kftsm_trend6", "Six major trend systems", "Exponential smoothing"),
        "kftsm_lrs" => ("kftsm_trend6", "Six major trend systems", "Linear regression slope"),
        "kftsm_ndb" => ("kftsm_trend6", "Six major trend systems", "N-day breakout"),
        "kftsm_swg" => ("kftsm_trend6", "Six major trend systems", "Swing breakout"),
        "kftsm_pf" => ("kftsm_trend6", "Six major trend systems", "Point-and-figure"),
        "kftsm_swing" => ("kftsm_trend6", "Six major trend systems", "Percent swing (Ch 5)"),
        "kftsm_bb_ma" | "kftsm_fc_bb_abx" | "kftsm_ma_pct_band" | "kftsm_ma_tr_band" | "kftsm_keltner" => ("kftsm_bands", "Trend bands", ""),
        "kftsm_triple_smooth" | "kftsm_trix" => ("kftsm_smooth", "Multiple smoothing", ""),
        "kftsm_kama" | "kftsm_adapt_exp" | "kftsm_vidya" | "kftsm_adapt_r2" => ("kftsm_adaptive", "Adaptive trend driver", ""),
        "kftsm_hh_ma" | "kftsm_hh_cross" | "kftsm_hh_channel" | "kftsm_mod_3ma" | "kftsm_4_9_18" => ("kftsm_hochheimer", "Hochheimer / crossover studies", ""),
        "kftsm_month_end" | "kftsm_hirsch" | "kftsm_holiday" | "kftsm_january_barometer" => ("kftsm_seasonal", "Stock-market seasonality", ""),
        "kftsm_donchian_4wk" | "kftsm_weekly_channel" | "kftsm_4wk_final3m" => ("kftsm_breakout", "Calendar breakouts", ""),
        s if s.starts_with("kftsm_") => (s_static(s), "Kaufman TSM", ""),
        _ => return None,
    })
}
fn s_static(s: &str) -> &'static str {
    catalogue().into_iter().find(|d| d.id == s).map_or("kftsm_other", |d| d.id)
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let fl = |n: f64| vec![p("period", "Calculation period (days)", n, 2., 250., 1.), p("filter", "Trend change filter (whole %, 0 = none)", 0., 0., 2., 0.05), p("lag", "Entry lag (bars)", 0., 0., 5., 1.)];
    let adapt = |extra: Vec<ParamDef>| { let mut v = vec![p("period", "Calculation period", 10., 2., 50., 1.), p("sdfilter", "Std-dev filter on trend change", 0.1, 0., 2., 0.05), p("mfilter", "Minimum trend change (points)", 0., 0., 50., 0.5)]; v.extend(extra); v };
    Some(match id {
        "kftsm_ma" => fl(20.),
        "kftsm_exp" => { let mut v = fl(20.); v.push(p("kind", "Smoothing: 0 SmoothedAverage (book code), 1 = 2/(n+1), 2 = 1/(n+1) spreadsheet", 0., 0., 2., 1.)); v }
        "kftsm_lrs" => vec![p("period", "Regression period", 20., 2., 250., 1.), p("filter", "Slope filter", 0., 0., 10., 0.1), p("lag", "Entry lag (bars)", 0., 0., 5., 1.)],
        "kftsm_ndb" => vec![p("period", "Breakout period N", 20., 1., 250., 1.), p("printed", "Window: 0 CD code (N bars), 1 printed Table 8.2 (N-1 bars)", 0., 0., 1., 1.)],
        "kftsm_swg" | "kftsm_swing" => vec![p("swing", "Minimum swing (%)", 2.5, 0.25, 10., 0.25)],
        "kftsm_pf" => vec![p("box", "Box size (points)", 5., 0.05, 500., 0.05), p("rev", "Reversal boxes", 3., 1., 5., 1.)],
        "kftsm_bb_ma" => vec![p("period", "MA / std-dev period", 20., 5., 100., 1.), p("nsd", "Band std devs", 2., 0.5, 4., 0.1)],
        "kftsm_fc_bb_abx" => vec![p("period", "MA / std-dev period", 21., 5., 100., 1.), p("nsd", "Outer band std devs (inner = half)", 2., 0.5, 4., 0.1)],
        "kftsm_ma_pct_band" => vec![p("period", "MA period", 20., 2., 250., 1.), p("pcband", "Band (% of MA)", 2., 0.1, 10., 0.1)],
        "kftsm_ma_tr_band" => vec![p("period", "MA / ATR period", 20., 2., 250., 1.), p("band", "Band (x average true range)", 5., 0.1, 10., 0.1)],
        "kftsm_triple_smooth" => vec![p("len1", "First smoothing period", 20., 2., 100., 1.), p("len2", "Second smoothing period", 10., 2., 100., 1.), p("len3", "Third smoothing period", 5., 2., 100., 1.)],
        "kftsm_vol_breakout" => vec![p("length", "ATR period", 20., 2., 100., 1.), p("factor", "Volatility factor k", 3., 0.5, 5., 0.1), p("vlag", "1 = previous bar's ATR (text), 0 = include today (CD Ch08)", 1., 0., 1., 1.)],
        "kftsm_raschke_first_cross" => vec![p("osc1", "Fast MA", 3., 2., 10., 1.), p("osc2", "Slow MA", 10., 5., 30., 1.), p("ntrend", "Oscillator smoothing", 16., 5., 40., 1.), p("vfilter", "Trend reach filter", 0., 0., 100., 1.)],
        "kftsm_keltner" => vec![p("period", "Average period", 10., 2., 50., 1.)],
        "kftsm_parabolic" => vec![p("afmin", "Starting AF", 0.02, 0.01, 0.1, 0.01), p("afinc", "AF increment", 0.02, 0.01, 0.1, 0.01), p("afmax", "Maximum AF", 0.2, 0.05, 0.5, 0.01)],
        "kftsm_outside_day" => vec![p("length", "Prior range (days)", 1., 1., 5., 1.), p("hold", "Exit on the close N days after entry", 3., 1., 10., 1.), p("stop_ref", "Stop: 0 outside day's extreme (text), 1 prior range (CD code)", 0., 0., 1., 1.)],
        "kftsm_donchian_4wk" => vec![p("weeks", "Calendar weeks", 4., 1., 12., 1.)],
        "kftsm_kama" | "kftsm_adapt_exp" | "kftsm_adapt_r2" | "kftsm_adapt_rsi" => adapt(vec![]),
        "kftsm_vidya" => adapt(vec![p("per2", "VIDYA history period", 50., 10., 200., 1.)]),
        "kftsm_hh_ma" => vec![p("period", "MA period", 35., 2., 200., 1.)],
        "kftsm_hh_cross" => vec![p("fast", "Fast MA", 17., 2., 100., 1.), p("slow", "Slow MA", 25., 3., 250., 1.)],
        "kftsm_hh_channel" => vec![p("period", "Channel period N", 100., 2., 250., 1.)],
        "kftsm_mod_3ma" => vec![p("length1", "Slow MA", 20., 3., 250., 1.), p("length2", "Fast MA", 5., 2., 100., 1.), p("length3", "Confirming MA", 2., 1., 50., 1.)],
        "kftsm_4_9_18" => vec![p("length1", "Slow MA", 18., 3., 250., 1.), p("length2", "Fast MA", 9., 2., 100., 1.), p("length3", "Confirming MA", 4., 1., 50., 1.)],
        "kftsm_moon" => vec![p("fstoch", "Fast stochastic period", 14., 3., 30., 1.), p("overbot", "Overbought", 80., 50., 95., 1.), p("oversold", "Oversold", 20., 5., 50., 1.), p("exit", "Exit level (long exits at 100-exit)", 50., 0., 80., 1.), p("holddays", "Exit after N days (0 = off)", 5., 0., 20., 1.)],
        "kftsm_multi_tf" => vec![p("fast", "Daily slow-K period", 5., 3., 30., 1.), p("slow", "Weekly regression period", 50., 5., 100., 1.), p("overbot", "Overbought", 80., 50., 95., 1.), p("oversold", "Oversold", 20., 5., 50., 1.)],
        "kftsm_month_end" => vec![p("day", "Entry: 0 last trading day, 1 second-to-last", 0., 0., 1., 1.), p("exit_day", "Exit on trading day N of the new month", 4., 1., 10., 1.)],
        "kftsm_hirsch" => vec![p("in_month", "Entry month", 11., 1., 12., 1.), p("out_month", "Exit month", 4., 1., 12., 1.)],
        _ => return None,
    })
}

// ---------------------------------------------------------------- order book (one position, full equity)

struct Bk<'a> {
    x: &'a Ctx<'a>,
    from: usize,
    to: usize,
    out: Vec<Trade>,
    cur: Option<Trade>,
}

impl<'a> Bk<'a> {
    fn new(x: &'a Ctx<'a>, from: usize, to: usize) -> Self { Bk { x, from, to, out: vec![], cur: None } }
    fn pos(&self) -> i8 { self.cur.as_ref().map_or(0, |t| t.dir as i8) }
    fn exit(&mut self, i: usize, px: f64, why: &str) {
        if let Some(mut t) = self.cur.take() {
            t.exit_i = i; t.exit_px = px; t.signal_i = i; t.reason = why.into();
            self.out.push(t);
        }
    }
    /// Target position `dir` at price px on bar i (close-out then entry). Entries only inside [from, to).
    fn go(&mut self, i: usize, dir: i8, px: f64, kind: &'static str, why: &str) {
        if i < self.from || dir == self.pos() { return; }
        if self.cur.is_some() { self.exit(i, px, why); }
        if dir != 0 && i < self.to {
            self.cur = Some(Trade { dir: dir as f64, legs: vec![Leg { i, px, w: 1. }], exit_i: i, exit_px: px, reason: String::new(), entry_kind: kind, signal_i: i, open: false, exits: vec![] });
        }
    }
    fn at_close(&mut self, i: usize, dir: i8, why: &str) { let c = self.x.c[i]; self.go(i, dir, c, "close", why); }
    fn done(mut self) -> Vec<Trade> {
        if let Some(mut t) = self.cur.take() {
            t.exit_i = self.to; t.exit_px = self.x.c[self.to]; t.signal_i = self.to; t.reason = "open".into(); t.open = true;
            self.out.push(t);
        }
        self.out
    }
}

// ---------------------------------------------------------------- helpers (TradeStation conventions)

fn hi(v: &[f64], a: usize, b: usize) -> f64 { v[a..b].iter().copied().fold(f64::NEG_INFINITY, f64::max) }
fn lo(v: &[f64], a: usize, b: usize) -> f64 { v[a..b].iter().copied().fold(f64::INFINITY, f64::min) }
/// Population standard deviation (TradeStation StdDev) of v[i+1-n..=i].
fn sd_at(v: &[f64], n: usize, i: usize) -> f64 {
    if i + 1 < n { return f64::NAN; }
    let w = &v[i + 1 - n..=i];
    let m = w.iter().sum::<f64>() / n as f64;
    (w.iter().map(|x| (x - m) * (x - m)).sum::<f64>() / n as f64).sqrt()
}
/// TradeStation SmoothedAverage: n-bar average on the first full bar, then (prev*(n-1)+price)/n.
fn smoothed(v: &[f64], n: usize) -> Vec<f64> {
    let mut out = vec![f64::NAN; v.len()];
    if v.len() < n || n == 0 { return out; }
    let mut prev = v[..n].iter().sum::<f64>() / n as f64;
    out[n - 1] = prev;
    for i in n..v.len() { prev = (prev * (n as f64 - 1.) + v[i]) / n as f64; out[i] = prev; }
    out
}
fn expo(v: &[f64], a: f64) -> Vec<f64> {
    let mut out = vec![f64::NAN; v.len()];
    if v.is_empty() { return out; }
    out[0] = v[0];
    for i in 1..v.len() { out[i] = out[i - 1] + a * (v[i] - out[i - 1]); }
    out
}
/// TSMLRslope: least-squares slope of the last n values, x = 1..n with the newest = n.
fn lr_slope(v: &[f64], n: usize, i: usize) -> f64 {
    if i + 1 < n || n < 2 { return f64::NAN; }
    let nf = n as f64;
    let sumx = nf * (nf + 1.) / 2.;
    let sumx2 = nf * (nf + 1.) * (2. * nf + 1.) / 6.;
    let (mut sumy, mut sumxy) = (0., 0.);
    for k in 0..n { let y = v[i - k]; sumy += y; sumxy += (nf - k as f64) * y; }
    let bot = nf * sumx2 - sumx * sumx;
    if bot != 0. { (nf * sumxy - sumx * sumy) / bot } else { 0. }
}
fn crosses_above(a: &[f64], b: &[f64], i: usize) -> bool { i > 0 && a[i] > b[i] && a[i - 1] <= b[i - 1] }
fn crosses_below(a: &[f64], b: &[f64], i: usize) -> bool { i > 0 && a[i] < b[i] && a[i - 1] >= b[i - 1] }
/// Days since 1970-01-01 of "YYYY-MM-DD".
fn days(d: &str) -> i64 {
    let y: i64 = d[0..4].parse().unwrap_or(1970);
    let m: i64 = d[5..7].parse().unwrap_or(1);
    let dd: i64 = d[8..10].parse().unwrap_or(1);
    let y2 = if m <= 2 { y - 1 } else { y };
    let era = y2.div_euclid(400);
    let yoe = y2 - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + dd - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}
fn ymd(d: &str) -> (i64, i64, i64) { (d[0..4].parse().unwrap_or(0), d[5..7].parse().unwrap_or(0), d[8..10].parse().unwrap_or(0)) }
fn week(d: &str) -> i64 { (days(d) + 3).div_euclid(7) } // Monday-start weeks
fn weekday(d: &str) -> i64 { (days(d) + 3).rem_euclid(7) } // 0 = Monday

/// Signal-state systems (TSMxxsignal): sig[i] = 1/-1 latched, 0 before the first signal. Orders at the close.
fn run_signal(x: &Ctx, from: usize, to: usize, sig: &[i8], why: &str) -> Vec<Trade> {
    let mut b = Bk::new(x, from, to);
    for i in from..=to { if sig[i] != 0 { b.at_close(i, sig[i], why); } }
    b.done()
}
/// change = (trend - trend[1])*100/close[1], latched by filter and lag (TSMMAsignal / TSMExpSignal).
fn latch_change(c: &[f64], tr: &[f64], filter: f64, lag: usize, pct: bool) -> Vec<i8> {
    let n = c.len();
    let mut ch = vec![f64::NAN; n];
    for i in 1..n { ch[i] = if pct { (tr[i] - tr[i - 1]) * 100. / c[i - 1] } else { tr[i] }; }
    let mut s = vec![0i8; n];
    let mut cur = 0;
    for i in 0..n {
        if i >= lag { let v = ch[i - lag]; if v > filter { cur = 1 } else if v < -filter { cur = -1 } }
        s[i] = cur;
    }
    s
}

// ---------------------------------------------------------------- moon phases (TSM Moon Trading, CD Ch14)

/// Julian Ephemeris Date -> (year, month, day with fraction): the CD's TSMJulianEDtoDate.
fn jde_to_date(j: f64) -> (i64, i64, f64) {
    let zz = (j + 0.5).trunc();
    let fract = j + 0.5 - zz;
    let aa = if zz < 2299161. { zz } else { let alpha = ((zz - 1867216.25) / 36524.25).trunc(); zz + 1. + alpha - (alpha / 4.).trunc() };
    let bb = aa + 1524.;
    let cc = ((bb - 122.1) / 365.25).trunc();
    let dd = (365.25 * cc).trunc();
    let ee = ((bb - dd) / 30.6001).trunc();
    let day = bb - dd - (30.6001 * ee).trunc() + fract;
    let mo = if ee < 13.5 { ee - 1. } else { ee - 13. };
    let yr = if mo > 2.5 { cc - 4716. } else { cc - 4715. };
    (yr as i64, mo as i64, day)
}
fn moon_jde(k: f64) -> f64 {
    let t = k / 1236.85;
    2451550.09765 + 29.530588853 * k + 0.0001337 * t * t - 0.000000150 * t.powi(3) + 0.00000000073 * t.powi(4)
}
/// +1 on the first bar dated after a full moon, -1 after a new moon, else 0.
pub fn moon_days(x: &Ctx) -> Vec<i8> {
    let n = x.r.len();
    let mut out = vec![0i8; n];
    if n == 0 { return out; }
    let (y0, _, _) = ymd(&x.r[0].date);
    let (y1, _, _) = ymd(&x.r[n - 1].date);
    let k0 = ((y0 - 2001) as f64 * 12.3685).floor() as i64;
    let k1 = ((y1 - 1999) as f64 * 12.3685).ceil() as i64;
    let dates: Vec<(i64, i64, i64)> = x.r.iter().map(|r| ymd(&r.date)).collect();
    for k in k0..=k1 {
        for (off, tag) in [(0.0, -1i8), (0.5, 1i8)] {
            let (my, mm, md) = jde_to_date(moon_jde(k as f64 + off));
            // TSMcomparedate(moon, today) < 0: (my,mm,md) < (y,m,d)
            let after = |&(y, m, d): &(i64, i64, i64)| (my, mm) < (y, m) || ((my, mm) == (y, m) && md < d as f64);
            if let Some(i) = dates.iter().position(after) {
                if i > 0 && !after(&dates[i - 1]) { out[i] = tag; }
            }
        }
    }
    out
}

// ---------------------------------------------------------------- run

/// Validate before allocating indicator windows or converting floating parameters to usize.
/// Integer bar counts must remain integers; fractional filters remain continuous.
pub fn validate(id: &str, p: &Params) -> Option<String> {
    let schema = params(id)?;
    for (key, &value) in p {
        let Some(def) = schema.iter().find(|d| d.key == key) else {
            return Some(format!("Unknown Kaufman TSM parameter: {key}"));
        };
        let integer = matches!(key.as_str(), "period" | "lag" | "kind" | "printed" | "rev" |
            "len1" | "len2" | "len3" | "length" | "vlag" | "osc1" | "osc2" | "ntrend" |
            "length1" | "length2" | "length3" | "hold" | "stop_ref" | "weeks" | "per2" |
            "fast" | "slow" | "fstoch" | "holddays" | "day" | "exit_day" | "in_month" | "out_month");
        if !value.is_finite() || value < def.min || value > def.max || (integer && value.fract() != 0.) {
            return Some(format!("Kaufman TSM parameter {key} must be {}from {} to {}",
                if integer { "an integer " } else { "finite " }, def.min, def.max));
        }
    }
    if matches!(id, "kftsm_hh_cross") {
        let value = |key: &str| p.get(key).copied().unwrap_or_else(|| schema.iter().find(|d|d.key==key).unwrap().def);
        if value("fast") >= value("slow") { return Some("Fast MA must be shorter than slow MA".into()); }
    }
    if matches!(id, "kftsm_vidya" | "kftsm_raschke_first_cross") {
        let value = |key: &str| p.get(key).copied().unwrap_or_else(|| schema.iter().find(|d|d.key==key).unwrap().def);
        if id == "kftsm_vidya" && value("per2") <= value("period") {
            return Some("VIDYA historical period must exceed the recent period".into());
        }
        if id == "kftsm_raschke_first_cross" && value("osc1") >= value("osc2") {
            return Some("First Cross fast MA must be shorter than slow MA".into());
        }
    }
    None
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if from > to || to >= x.r.len() || validate(id, p).is_some() { return None; }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(0., |d| d.def));
    let u = |k: &str| g(k).round().max(0.) as usize;
    let n = x.r.len();
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    Some(match id {
        "kftsm_ma" => {
            let ma = sma(c, u("period").max(1));
            run_signal(x, from, to, &latch_change(c, &ma, g("filter"), u("lag"), true), "trendline turned")
        }
        "kftsm_exp" => {
            let pn = u("period").max(1);
            let e = match u("kind") { 1 => expo(c, 2. / (pn as f64 + 1.)), 2 => expo(c, 1. / (pn as f64 + 1.)), _ => smoothed(c, pn) };
            run_signal(x, from, to, &latch_change(c, &e, g("filter"), u("lag"), true), "trendline turned")
        }
        "kftsm_lrs" => {
            let pn = u("period").max(2);
            let s: Vec<f64> = (0..n).map(|i| lr_slope(c, pn, i)).collect();
            run_signal(x, from, to, &latch_change(c, &s, g("filter"), u("lag"), false), "slope turned")
        }
        "kftsm_ndb" => {
            let pn = u("period").max(1);
            let w = if u("printed") == 1 { pn.saturating_sub(1).max(1) } else { pn };
            let mut s = vec![0i8; n];
            let mut cur = 0;
            for i in 0..n {
                if i >= w && i >= 1 {
                    if h[i] > hi(h, i - w, i) && c[i] > c[i - 1] { cur = 1; }
                    if l[i] < lo(l, i - w, i) && c[i] < c[i - 1] { cur = -1; }
                }
                s[i] = cur;
            }
            run_signal(x, from, to, &s, "opposite N-day breakout")
        }
        "kftsm_swg" => {
            let sw = g("swing");
            let mut s = vec![0i8; n];
            let (mut last, mut curh, mut curl, mut swh, mut swl, mut sig) = (0i8, c[0], c[0], f64::NAN, f64::NAN, 0i8);
            for i in 0..n {
                let ms = sw * c[i] / 100.;
                if last != 1 {
                    if h[i] > curh { curh = h[i]; }
                    if l[i] < curh - ms { last = 1; swh = curh; curl = l[i]; }
                } else {
                    if l[i] < curl { curl = l[i]; }
                    if h[i] > curl + ms { last = -1; swl = curl; curh = h[i]; }
                }
                if last == 1 && curl < swl { sig = -1; }
                if last == -1 && curh > swh { sig = 1; }
                s[i] = sig;
            }
            run_signal(x, from, to, &s, "opposite swing breakout")
        }
        "kftsm_swing" => {
            let pc = g("swing") / 100.;
            let mut b = Bk::new(x, from, to);
            let (mut last, mut curh, mut curl, mut swh, mut swl) = (0i8, c[0], c[0], f64::NAN, f64::NAN);
            for i in 0..=to {
                let mp = b.pos();
                if mp <= 0 && h[i] > swh { b.at_close(i, 1, "swing high broken"); } else if mp >= 0 && l[i] < swl { b.at_close(i, -1, "swing low broken"); }
                if last != 1 {
                    if l[i] < curh - curh * pc { last = 1; swh = curh; curl = l[i]; } else if h[i] > curh { curh = h[i]; }
                }
                if last != -1 {
                    if h[i] > curl + curl * pc { last = -1; swl = curl; curh = h[i]; } else if l[i] < curl { curl = l[i]; }
                }
            }
            b.done()
        }
        "kftsm_pf" => {
            let (bx, rev) = (g("box").max(1e-9), g("rev"));
            let ip = |v: f64| (v / bx).trunc();
            let md0 = |v: f64| (v % bx).abs() < 1e-9;
            let mut b = Bk::new(x, from, to);
            let mut bh = ip(c[0]);
            let mut bl = ip(c[0]) + 1.;
            if md0(c[0]) { bl -= 1.; }
            let mut last: i8 = if c[0] >= (h[0] - l[0]) / 2. { 1 } else { -1 };
            let (mut trend, mut lasth, mut lastl) = (0i8, 0f64, 0f64);
            for i in 0..=to {
                let nh = ip(h[i]);
                let nl = ip(l[i]) + 1.;
                if md0(l[i]) { bl -= 1.; }
                if last == 1 {
                    if nh > bh {
                        bh = nh; bl = bh + 1.;
                        if trend < 1 && nh > lasth { trend = 1; b.at_close(i, 1, "P&F buy"); }
                    } else if nl < bl {
                        bl = nl;
                        if bh - bl >= rev {
                            last = -1; lasth = bh; bh = bl - 1.;
                            if trend > -1 && bl < lastl { trend = -1; b.at_close(i, -1, "P&F sell"); }
                        }
                    }
                } else if last == -1 {
                    if nl < bl {
                        bl = nl; bh = bl - 1.;
                        if trend > -1 && nl < lastl { trend = -1; b.at_close(i, -1, "P&F sell"); }
                    } else if nh > bh {
                        bh = nh;
                        if bh - bl >= rev {
                            last = 1; lastl = bl; bl = bh + 1.;
                            if trend < 1 && bh > lasth { trend = 1; b.at_close(i, 1, "P&F buy"); }
                        }
                    }
                }
            }
            b.done()
        }
        "kftsm_bb_ma" => {
            let (pn, k) = (u("period").max(2), g("nsd"));
            let ma = sma(c, pn);
            let up: Vec<f64> = (0..n).map(|i| ma[i] + k * sd_at(c, pn, i)).collect();
            let dn: Vec<f64> = (0..n).map(|i| ma[i] - k * sd_at(c, pn, i)).collect();
            let mut b = Bk::new(x, from, to);
            let mut pend: Option<(i8, &str)> = None;
            for i in from..=to {
                if let Some((d, why)) = pend.take() { b.go(i, d, o[i], "open", why); }
                let mp = b.pos();
                pend = if crosses_above(c, &up, i) && mp != 1 { Some((1, "close crossed above upper band")) }
                    else if crosses_below(c, &dn, i) && mp != -1 { Some((-1, "close crossed below lower band")) }
                    else if (mp > 0 && crosses_below(c, &ma, i)) || (mp < 0 && crosses_above(c, &ma, i)) { Some((0, "close crossed the average")) }
                    else { None };
            }
            b.done()
        }
        "kftsm_fc_bb_abx" => fc_abx(x, from, to, u("period").max(2), g("nsd")),
        "kftsm_ma_pct_band" => {
            let ma = sma(c, u("period").max(1));
            let k = g("pcband") / 100.;
            let mut b = Bk::new(x, from, to);
            for i in from..=to {
                if c[i] > ma[i] + ma[i] * k { b.at_close(i, 1, "close above upper band"); }
                if c[i] < ma[i] - ma[i] * k { b.at_close(i, -1, "close below lower band"); }
            }
            b.done()
        }
        "kftsm_ma_tr_band" => {
            let pn = u("period").max(1);
            let (ma, at) = (sma(c, pn), sma(&true_range(x.r), pn));
            let k = g("band");
            let mut b = Bk::new(x, from, to);
            for i in from.max(1)..=to {
                if c[i] > ma[i - 1] + at[i - 1] * k { b.at_close(i, 1, "close above upper band"); }
                if c[i] < ma[i - 1] - at[i - 1] * k { b.at_close(i, -1, "close below lower band"); }
            }
            b.done()
        }
        "kftsm_triple_smooth" => {
            let a = |k: &str| 2. / (g(k).max(1.) + 1.);
            let s1 = expo(c, a("len1"));
            let s2 = expo(&s1, a("len2"));
            let s3 = expo(&s2, a("len3"));
            let mut b = Bk::new(x, from, to);
            for i in from.max(1)..=to {
                if s3[i] > s3[i - 1] { b.at_close(i, 1, "triple smoothing turned up"); } else if s3[i] < s3[i - 1] { b.at_close(i, -1, "triple smoothing turned down"); }
            }
            b.done()
        }
        "kftsm_vol_breakout" => {
            let v = sma(&true_range(x.r), u("length").max(1));
            let (k, lag) = (g("factor"), u("vlag").min(1));
            let mut b = Bk::new(x, from, to);
            for i in from.max(1)..=to {
                let vv = v[i - lag];
                if c[i] - c[i - 1] > k * vv { b.at_close(i, 1, "up close beyond k x ATR"); } else if c[i - 1] - c[i] > k * vv { b.at_close(i, -1, "down close beyond k x ATR"); }
            }
            b.done()
        }
        "kftsm_raschke_first_cross" => raschke(x, from, to, u("osc1").max(1), u("osc2").max(1), u("ntrend").max(1), g("vfilter")),
        "kftsm_keltner" => {
            let pn = u("period").max(1);
            let typ: Vec<f64> = (0..n).map(|i| (h[i] + l[i] + c[i]) / 3.).collect();
            let rng: Vec<f64> = (0..n).map(|i| h[i] - l[i]).collect();
            let (m, r) = (sma(&typ, pn), sma(&rng, pn));
            let mut b = Bk::new(x, from, to);
            for i in from..=to {
                if c[i] > m[i] + r[i] { b.at_close(i, 1, "close above upper band"); } else if c[i] < m[i] - r[i] { b.at_close(i, -1, "close below lower band"); }
            }
            b.done()
        }
        "kftsm_parabolic" => parabolic(x, from, to, g("afmin"), g("afinc"), g("afmax")),
        "kftsm_outside_day" => outside_day(x, from, to, u("length").max(1), u("hold").max(1), u("stop_ref") == 1),
        "kftsm_donchian_4wk" => donchian_weeks(x, from, to, u("weeks").max(1) as i64),
        "kftsm_kama" | "kftsm_adapt_exp" | "kftsm_vidya" | "kftsm_adapt_r2" | "kftsm_adapt_rsi" => {
            let pn = u("period").max(1);
            let tr = match id {
                "kftsm_kama" => kama(c, pn),
                "kftsm_adapt_exp" => smoothed(c, pn),
                "kftsm_vidya" => vidya(c, pn, u("per2").max(2)),
                "kftsm_adapt_rsi" => adapt_rsi(c, pn),
                _ => adapt_r2(c, pn),
            };
            adaptive_driver(x, from, to, &tr, pn, g("sdfilter"), g("mfilter"))
        }
        "kftsm_hh_ma" => {
            let ma = sma(c, u("period").max(1));
            let mut b = Bk::new(x, from, to);
            for i in from..=to {
                if h[i] == l[i] { continue; }
                if crosses_above(c, &ma, i) { b.at_close(i, 1, "close crossed above MA"); }
                else if crosses_below(c, &ma, i) { b.at_close(i, -1, "close crossed below MA"); }
            }
            b.done()
        }
        "kftsm_hh_cross" => {
            let (f, s) = (sma(c, u("fast").max(1)), sma(c, u("slow").max(1)));
            let mut b = Bk::new(x, from, to);
            for i in from..=to {
                if h[i] == l[i] { continue; }
                if crosses_above(&f, &s, i) { b.at_close(i, 1, "fast MA crossed above slow MA"); }
                else if crosses_below(&f, &s, i) { b.at_close(i, -1, "fast MA crossed below slow MA"); }
            }
            b.done()
        }
        "kftsm_hh_channel" => {
            let pn = u("period").max(1);
            let mut b = Bk::new(x, from, to);
            for i in from.max(pn)..=to {
                if c[i] > hi(h, i - pn, i) { b.at_close(i, 1, "close above N-day high"); } else if c[i] < lo(l, i - pn, i) { b.at_close(i, -1, "close below N-day low"); }
            }
            b.done()
        }
        "kftsm_mod_3ma" | "kftsm_4_9_18" => {
            let (m1, m2, m3) = (sma(c, u("length1").max(1)), sma(c, u("length2").max(1)), sma(c, u("length3").max(1)));
            let mut b = Bk::new(x, from, to);
            for i in from.max(1)..=to {
                if m2[i] > m1[i] && m3[i] > m3[i - 1] { b.at_close(i, 1, "crossover up, confirming MA rising"); }
                if m2[i] < m1[i] && m3[i] < m3[i - 1] { b.at_close(i, -1, "crossover down, confirming MA falling"); }
            }
            b.done()
        }
        "kftsm_moon" => moon_trading(x, from, to, u("fstoch").max(1), g("overbot"), g("oversold"), g("exit"), u("holddays")),
        "kftsm_multi_tf" => multi_tf(x, from, to, u("fast").max(1), u("slow").max(2), g("overbot"), g("oversold")),
        "kftsm_month_end" => month_end(x, from, to, u("day").min(1), u("exit_day").max(1)),
        "kftsm_hirsch" => hirsch(x, from, to, g("in_month").round() as i64, g("out_month").round() as i64),
        _ => return None,
    })
}

fn fc_abx(x: &Ctx, from: usize, to: usize, pn: usize, k: f64) -> Vec<Trade> {
    let (o, c, n) = (&x.o, &x.c, x.r.len());
    let ma = sma(c, pn);
    let sd: Vec<f64> = (0..n).map(|i| sd_at(c, pn, i)).collect();
    let band = |m: f64| -> Vec<f64> { (0..n).map(|i| ma[i] + m * sd[i]).collect() };
    let (ou, ol, iu, il) = (band(k), band(-k), band(k / 2.), band(-k / 2.));
    let mut out = vec![];
    let mut cur: Option<(Trade, u8)> = None; // trade, contracts left (of 2)
    let mut pend: (i8, u8) = (0, 0); // entry dir, exits count
    let close_part = |t: &mut Trade, i: usize, px: f64, f: f64| t.exits.push((i, px, f));
    let finish = |mut t: Trade, why: &str| -> Trade {
        let tot: f64 = t.exits.iter().map(|e| e.2).sum();
        t.exit_px = t.exits.iter().map(|e| e.1 * e.2).sum::<f64>() / tot;
        t.exit_i = t.exits.last().unwrap().0; t.signal_i = t.exit_i.saturating_sub(1); t.reason = why.into();
        if t.exits.len() == 1 { t.exits.clear(); }
        t
    };
    for i in from..=to {
        let (ed, ex) = std::mem::take(&mut pend);
        if ed != 0 {
            if let Some((mut t, left)) = cur.take() { close_part(&mut t, i, o[i], left as f64 / 2.); out.push(finish(t, "reversed by opposite band entry")); }
            if i <= to { cur = Some((Trade { dir: ed as f64, legs: vec![Leg { i, px: o[i], w: 1. }], exit_i: i, exit_px: o[i], reason: String::new(), entry_kind: "open", signal_i: i, open: false, exits: vec![] }, 2)); }
        } else if ex > 0 {
            if let Some((mut t, left)) = cur.take() {
                let k = ex.min(left);
                close_part(&mut t, i, o[i], k as f64 / 2.);
                if left - k == 0 { out.push(finish(t, "band exits")); } else { cur = Some((t, left - k)); }
            }
        }
        if i == to { break; }
        let mp = cur.as_ref().map_or(0, |t| t.0.dir as i8);
        if crosses_above(c, &ou, i) && mp != 1 { pend = (1, 0); continue; }
        if crosses_below(c, &ol, i) && mp != -1 { pend = (-1, 0); continue; }
        let mut e = 0u8;
        if mp > 0 { e += crosses_below(c, &iu, i) as u8 + crosses_below(c, &ma, i) as u8; }
        if mp < 0 { e += crosses_above(c, &il, i) as u8 + crosses_above(c, &ma, i) as u8; }
        pend = (0, e);
    }
    if let Some((mut t, left)) = cur {
        close_part(&mut t, to, c[to], left as f64 / 2.);
        let mut t = finish(t, "open");
        t.open = true;
        t.signal_i = to; // terminal mark, not a next-open liquidation signal
        out.push(t);
    }
    out
}

fn raschke(x: &Ctx, from: usize, to: usize, o1: usize, o2: usize, nt: usize, vf: f64) -> Vec<Trade> {
    let (h, l, c, n) = (&x.h, &x.l, &x.c, x.r.len());
    let (m1, m2) = (sma(c, o1), sma(c, o2));
    let fast: Vec<f64> = (0..n).map(|i| m1[i] - m2[i]).collect();
    let tr = sma(&fast, nt);
    let mut b = Bk::new(x, from, to);
    let (mut twait, mut owait, mut trmax, mut stage) = (0i8, 0i8, 0f64, 0i8);
    for i in 1..=to {
        if !tr[i - 1].is_finite() || !tr[i].is_finite() { continue; }
        let mp = b.pos();
        if tr[i - 1] <= 0. && tr[i] > 0. { twait = -1; owait = 0; trmax = tr[i]; }
        if tr[i - 1] >= 0. && tr[i] < 0. { twait = 1; owait = 0; trmax = -tr[i]; }
        if tr[i] > 0. && tr[i - 1] > 0. && tr[i] > trmax { trmax = tr[i]; }
        if tr[i] < 0. && tr[i - 1] < 0. && -tr[i] > trmax { trmax = -tr[i]; }
        let mut order: Option<i8> = None;
        if trmax > vf {
            if mp != 1 && twait < 0 {
                if owait == 0 && fast[i - 1] > tr[i] && fast[i] <= tr[i] { owait = -1; }
                if owait == -1 && l[i] > l[i - 1] { order = Some(1); stage = 0; }
            }
            if mp != -1 && twait > 0 {
                if owait == 0 && fast[i - 1] < tr[i] && fast[i] >= tr[i] { owait = 1; }
                if owait == 1 && h[i] < h[i - 1] { order = Some(-1); stage = 0; }
            }
        }
        let mut exit = false;
        if mp == 1 {
            if stage == 0 && c[i] > c[i - 1] { stage = 1; }
            if stage == 1 && c[i] < c[i - 1] { exit = true; twait = 0; }
        }
        if mp == -1 {
            if stage == 0 && c[i] < c[i - 1] { stage = 1; }
            if stage == 1 && c[i] > c[i - 1] { exit = true; twait = 0; }
        }
        if i < from { continue; }
        match order {
            Some(d) => b.at_close(i, d, "first cross"),
            None if exit => b.at_close(i, 0, "close turned against the trade"),
            None => {}
        }
    }
    b.done()
}

fn parabolic(x: &Ctx, from: usize, to: usize, afmin: f64, afinc: f64, afmax: f64) -> Vec<Trade> {
    let (o, h, l) = (&x.o, &x.h, &x.l);
    let mut b = Bk::new(x, from, to);
    let (mut pos, mut af, mut sar, mut hh, mut ll) = (1i8, afmin, l[0], h[0], l[0]);
    for i in 1..=to {
        let (prev_pos, prev_sar, prev_hh, prev_ll) = (pos, sar, hh, ll);
        if h[i] > hh { hh = h[i]; }
        if l[i] < ll { ll = l[i]; }
        if pos == 1 {
            if l[i] <= prev_sar { pos = -1; sar = hh; af = afmin; ll = l[i]; hh = h[i]; b.go(i, -1, o[i].min(prev_sar), "stop", "SAR penetrated"); }
        } else if h[i] >= prev_sar { pos = 1; sar = ll; af = afmin; ll = l[i]; hh = h[i]; b.go(i, 1, o[i].max(prev_sar), "stop", "SAR penetrated"); }
        if pos == 1 {
            if prev_pos == 1 { sar = prev_sar + af * (h[i - 1] - prev_sar); if hh > prev_hh && af < afmax { af += afinc; } }
            if sar > l[i] { sar = l[i]; }
            if sar > l[i - 1] { sar = l[i - 1]; }
        } else {
            if prev_pos == -1 { sar = prev_sar + af * (l[i - 1] - prev_sar); if ll < prev_ll && af < afmax { af += afinc; } }
            if sar < h[i] { sar = h[i]; }
            if sar < h[i - 1] { sar = h[i - 1]; }
        }
    }
    b.done()
}

fn outside_day(x: &Ctx, from: usize, to: usize, len: usize, hold: usize, code_stop: bool) -> Vec<Trade> {
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let mut b = Bk::new(x, from, to);
    let (mut stop, mut entry) = (0f64, 0usize);
    for i in from.max(len)..=to {
        if b.pos() != 0 && i > entry {
            let d = b.pos();
            if (d > 0 && l[i] < stop) || (d < 0 && h[i] > stop) {
                let px = if d > 0 { o[i].min(stop) } else { o[i].max(stop) };
                b.exit(i, px, "outside-day stop");
            } else if i - entry >= hold { b.at_close(i, 0, "time exit (close N days after entry)"); }
        }
        let (rh, rl) = (hi(h, i - len, i), lo(l, i - len, i));
        if h[i] > rh && l[i] < rl {
            let d = if c[i] > rh { 1 } else if c[i] < rl { -1 } else { 0 };
            if d != 0 && d != b.pos() {
                // A final-close reversal still liquidates the held position.
                // Bk suppresses only the new entry outside [from, to).
                b.at_close(i, d, "opposite outside day");
                entry = i;
                stop = match (d, code_stop) { (1, false) => l[i], (1, true) => rl, (_, false) => h[i], _ => rh };
            }
        }
    }
    b.done()
}

fn donchian_weeks(x: &Ctx, from: usize, to: usize, nw: i64) -> Vec<Trade> {
    let (o, h, l) = (&x.o, &x.h, &x.l);
    let wk: Vec<i64> = x.r.iter().map(|r| week(&r.date)).collect();
    let mut b = Bk::new(x, from, to);
    let (mut start, mut cur) = (0usize, 0usize); // first bar of the window / of the current week
    for i in 0..=to {
        if wk[i] != wk[cur] { cur = i; }
        while wk[start] < wk[i] - nw { start += 1; }
        // the window must hold nw full calendar weeks: data must begin before its first week
        if i < from || wk[0] >= wk[i] - nw || start >= cur { continue; }
        let (hh, ll) = (hi(h, start, cur), lo(l, start, cur));
        let (up, dn) = (h[i] > hh, l[i] < ll);
        match b.pos() {
            1 => if dn { b.go(i, -1, o[i].min(ll), "stop", "four-week low broken"); },
            -1 => if up { b.go(i, 1, o[i].max(hh), "stop", "four-week high broken"); },
            _ => if up && !dn { b.go(i, 1, o[i].max(hh), "stop", "four-week high broken"); } else if dn && !up { b.go(i, -1, o[i].min(ll), "stop", "four-week low broken"); },
        }
    }
    b.done()
}

fn kama(c: &[f64], pn: usize) -> Vec<f64> {
    let n = c.len();
    let mut a = vec![f64::NAN; n];
    if n == 0 { return a; }
    a[0] = c[0];
    for i in 1..n {
        let mut er = 1.;
        if i >= pn {
            let sig = (c[i] - c[i - pn]).abs();
            let noise: f64 = (i + 1 - pn..=i).map(|k| (c[k] - c[k - 1]).abs()).sum();
            if noise != 0. { er = sig / noise; }
        }
        let sc = (er * (0.666 - 0.0645) + 0.0645f64).powi(2);
        a[i] = a[i - 1] + sc * (c[i] - a[i - 1]);
    }
    a
}
fn vidya(c: &[f64], pn: usize, hp: usize) -> Vec<f64> {
    let n = c.len();
    let mut v = vec![f64::NAN; n];
    let sc = 2. / (pn as f64 + 1.);
    for i in 0..n {
        let (s1, s2) = (sd_at(c, pn, i), sd_at(c, hp, i));
        // The unavailable historical volatility window cannot become a close-based
        // trading signal. Seed once, then explicitly carry through the 0/0 case.
        if !s1.is_finite() || !s2.is_finite() { continue; }
        v[i] = if i == 0 || !v[i - 1].is_finite() { c[i] }
            else if s2 == 0. { v[i - 1] }
            else { let k = s1 / s2; k * sc * c[i] + (1. - k * sc) * v[i - 1] };
    }
    v
}
fn adapt_r2(c: &[f64], pn: usize) -> Vec<f64> {
    let n = c.len();
    let mut a = vec![f64::NAN; n];
    for i in 0..n {
        if i == 0 || i + 1 < pn { a[i] = c[i]; continue; }
        let (xs, ys): (Vec<f64>, Vec<f64>) = (i + 1 - pn..=i).map(|k| ((k + 1) as f64, c[k])).unzip();
        let (mx, my) = (xs.iter().sum::<f64>() / pn as f64, ys.iter().sum::<f64>() / pn as f64);
        let (mut sxy, mut sxx, mut syy) = (0., 0., 0.);
        for k in 0..pn { sxy += (xs[k] - mx) * (ys[k] - my); sxx += (xs[k] - mx).powi(2); syy += (ys[k] - my).powi(2); }
        let r = if sxx > 0. && syy > 0. { sxy / (sxx * syy).sqrt() } else { 0. };
        a[i] = a[i - 1] + r * r * (c[i] - a[i - 1]);
    }
    a
}
fn adapt_rsi(c: &[f64], pn: usize) -> Vec<f64> {
    let strength = rsi(c, pn);
    let mut trend = vec![f64::NAN; c.len()];
    if c.is_empty() { return trend; }
    trend[0] = c[0];
    for i in 1..c.len() {
        trend[i] = trend[i - 1]; // missing RSI prehistory is not a fabricated RSI
        if strength[i].is_finite() {
            let sc = (strength[i] / 100. - 0.5).abs() * 2.;
            trend[i] += sc * (c[i] - trend[i - 1]);
        }
    }
    trend
}
fn adaptive_driver(x: &Ctx, from: usize, to: usize, tr: &[f64], pn: usize, sdf: f64, mf: f64) -> Vec<Trade> {
    let n = tr.len();
    let mut ch = vec![f64::NAN; n];
    for i in 1..n { ch[i] = tr[i] - tr[i - 1]; }
    let mut b = Bk::new(x, from, to);
    for i in from.max(pn + 5).max(21)..=to {
        let sd = sd_at(&ch, 20, i - 1);
        if !(ch[i].abs() > sdf * sd && ch[i].abs() > mf) { continue; }
        if tr[i] > tr[i - 1] { b.at_close(i, 1, "trend turned up beyond filter"); }
        if tr[i] < tr[i - 1] { b.at_close(i, -1, "trend turned down beyond filter"); }
    }
    b.done()
}

fn fastk(x: &Ctx, len: usize, i: usize) -> f64 {
    if i + 1 < len { return f64::NAN; }
    let (hh, ll) = (hi(&x.h, i + 1 - len, i + 1), lo(&x.l, i + 1 - len, i + 1));
    if hh > ll { (x.c[i] - ll) / (hh - ll) * 100. } else { 0. }
}

fn moon_trading(x: &Ctx, from: usize, to: usize, len: usize, ob: f64, os: f64, ex: f64, hold: usize) -> Vec<Trade> {
    let moon = moon_days(x);
    let mut b = Bk::new(x, from, to);
    let mut nd = 0usize;
    for i in from..=to {
        let st = fastk(x, len, i);
        if !st.is_finite() { continue; }
        let mp = b.pos();
        nd += 1;
        let m = moon[i];
        let sell = mp > -1 && st >= ob && m != 0;
        let buy = mp < 1 && st <= os && m != 0;
        if sell || buy { nd = 0; }
        let mut exit = None;
        if mp == 1 {
            if st >= 100. - ex { exit = Some("stochastic reached exit level"); }
            if exit.is_none() && hold > 0 && nd >= hold { exit = Some("holding days"); }
            if exit.is_none() && m == 1 { exit = Some("full moon"); }
        }
        if mp == -1 {
            if st <= ex { exit = Some("stochastic reached exit level"); }
            if exit.is_none() && hold > 0 && nd >= hold { exit = Some("holding days"); }
            if exit.is_none() && m == -1 { exit = Some("new moon"); }
        }
        if sell { b.at_close(i, -1, "moon + overbought"); } else if buy { b.at_close(i, 1, "moon + oversold"); } else if let Some(w) = exit { b.at_close(i, 0, w); }
    }
    b.done()
}

fn multi_tf(x: &Ctx, from: usize, to: usize, fast: usize, slow: usize, ob: f64, os: f64) -> Vec<Trade> {
    let n = x.r.len();
    let wk: Vec<i64> = x.r.iter().map(|r| week(&r.date)).collect();
    // weekly closes in order, with the bar index from which each one is known
    let mut weekly: Vec<(usize, f64)> = vec![];
    for j in 0..n {
        let last = j + 1 == n || wk[j + 1] != wk[j];
        if !last { continue; }
        let known = if weekday(&x.r[j].date) >= 4 { j } else { j + 1 };
        weekly.push((known, x.c[j]));
    }
    let fk: Vec<f64> = (0..n).map(|i| fastk(x, fast, i)).collect();
    let sk = sma(&fk, 3);
    let mut b = Bk::new(x, from, to);
    let mut w = 0usize; // number of weekly bars known
    for i in 0..=to {
        while w < weekly.len() && weekly[w].0 <= i { w += 1; }
        if i < from || w < slow || !sk[i].is_finite() { continue; }
        let cl: Vec<f64> = weekly[w - slow..w].iter().map(|v| v.1).collect();
        let s = lr_slope(&cl, slow, slow - 1);
        let mp = b.pos();
        if s > 0. && sk[i] < os { b.at_close(i, 1, "weekly slope up, daily stochastic oversold"); }
        else if s < 0. && sk[i] > ob { b.at_close(i, -1, "weekly slope down, daily stochastic overbought"); }
        else if (s > 0. && mp < 0) || (s < 0. && mp > 0) { b.at_close(i, 0, "weekly slope turned"); }
    }
    b.done()
}

fn month_end(x: &Ctx, from: usize, to: usize, day: usize, exit_day: usize) -> Vec<Trade> {
    let n = x.r.len();
    let mo: Vec<i64> = x.r.iter().map(|r| { let (y, m, _) = ymd(&r.date); y * 12 + m }).collect();
    let mut b = Bk::new(x, from, to);
    let mut count = 0usize; // trading day number within the month
    let mut target = 0i64; // month in which to exit
    for i in 0..=to {
        count = if i > 0 && mo[i] == mo[i - 1] { count + 1 } else { 1 };
        if b.pos() == 1 && mo[i] == target && count == exit_day { b.at_close(i, 0, "trading day N of the new month"); }
        // entry bar = last (day 0) or second-to-last (day 1) trading day of its month
        let entry = i + day + 1 < n && mo[i + day] == mo[i] && mo[i + day + 1] != mo[i];
        if entry && b.pos() == 0 && i >= from && i < to { b.at_close(i, 1, ""); target = mo[i] + 1; }
    }
    b.done()
}

fn hirsch(x: &Ctx, from: usize, to: usize, inm: i64, outm: i64) -> Vec<Trade> {
    let n = x.r.len();
    let mo: Vec<i64> = x.r.iter().map(|r| ymd(&r.date).1).collect();
    let mut b = Bk::new(x, from, to);
    for i in from..=to {
        let first = i > 0 && mo[i] == inm && mo[i - 1] != inm;
        let last = mo[i] == outm && i + 1 < n && mo[i + 1] != outm;
        if b.pos() == 1 && last { b.at_close(i, 0, "last trading day of April"); }
        else if b.pos() == 0 && first { b.at_close(i, 1, "first trading day of November"); }
    }
    b.done()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn rows(n: usize) -> Vec<Row> {
        (0..n).map(|i| Row { ticker: "T".into(), date: format!("{}", days_to(i as i64 + 10957)), open: 100., high: 101., low: 99., close: 100., volume: None }).collect()
    }
    fn days_to(z: i64) -> String {
        let z = z + 719468;
        let era = z.div_euclid(146097);
        let doe = z - era * 146097;
        let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d = doy - (153 * mp + 2) / 5 + 1;
        let m = if mp < 10 { mp + 3 } else { mp - 9 };
        format!("{:04}-{:02}-{:02}", yoe + era * 400 + if m <= 2 { 1 } else { 0 }, m, d)
    }
    #[test]
    fn dates_and_moon_reference() {
        assert_eq!(days("2000-01-01"), 10957);
        assert_eq!(days_to(10957), "2000-01-01");
        assert_eq!(weekday("2024-01-05"), 4); // Friday
        // k = 0: new moon of 6 January 2000 (book p.526)
        let (y, m, d) = jde_to_date(moon_jde(0.));
        assert_eq!((y, m, d.floor() as i64), (2000, 1, 6));
    }
    #[test]
    fn outside_day_follows_arnold_rules() {
        let mut r = rows(30);
        r[10].high = 103.; r[10].low = 97.; r[10].close = 102.; // outside day, close above prior high 101
        r[13].close = 104.;
        let x = Ctx::new(&r, &r, &[]);
        let ts = run("kftsm_outside_day", &x, 2, 29, &Params::new()).unwrap();
        assert_eq!(ts[0].legs[0].i, 10); assert_eq!(ts[0].legs[0].px, 102.);
        assert_eq!(ts[0].exit_i, 13); assert_eq!(ts[0].exit_px, 104.); // close 3 days after entry
        let mut r2 = r.clone(); r2[12].low = 96.; r2[12].open = 98.;
        let x2 = Ctx::new(&r2, &r2, &[]);
        let ts = run("kftsm_outside_day", &x2, 2, 29, &Params::new()).unwrap();
        assert_eq!((ts[0].exit_i, ts[0].exit_px), (12, 97.)); // stop at the outside day's low
    }
    #[test]
    fn ma_direction_and_ndb_windows() {
        let mut r = rows(60);
        for (i, row) in r.iter_mut().enumerate() { let v = 100. + if i < 30 { i as f64 } else { 60. - i as f64 }; row.close = v; row.open = v; row.high = v + 1.; row.low = v - 1.; }
        let x = Ctx::new(&r, &r, &[]);
        let ts = run("kftsm_ma", &x, 5, 59, &Params::from([("period".into(), 5.)])).unwrap();
        assert_eq!(ts[0].dir, 1.); assert_eq!(ts[0].legs[0].i, 5);
        assert!(ts.iter().all(|t| t.exit_px >= x.l[t.exit_i] && t.exit_px <= x.h[t.exit_i]));
        assert_eq!(ts[1].dir, -1.);
    }
    #[test]
    fn hochheimer_locked_bar_cross_is_not_filled_later_as_an_above_average_state() {
        let mut r = rows(9);
        let closes = [100., 100., 90., 80., 100., 100., 50., 50., 50.];
        for (row, &close) in r.iter_mut().zip(&closes) {
            row.open=close; row.close=close; row.high=close+1.; row.low=close-1.;
        }
        r[4].high=100.; r[4].low=100.; // upward cross is not executable
        let x = Ctx::new(&r, &r, &[]);
        let ts=run("kftsm_hh_ma", &x, 3, 8, &Params::from([("period".into(), 2.)])).unwrap();
        assert_eq!(ts.len(), 1);
        assert_eq!(ts[0].legs[0].i, 6); assert_eq!(ts[0].dir, -1.);
        assert!(ts[0].open);
    }
    #[test]
    fn hochheimer_two_average_cross_waits_after_a_locked_signal() {
        let mut r=rows(10);
        let closes=[100.,100.,90.,80.,100.,100.,100.,50.,50.,50.];
        for (row,&close) in r.iter_mut().zip(&closes) {
            row.open=close; row.close=close; row.high=close+1.; row.low=close-1.;
        }
        r[5].high=100.; r[5].low=100.;
        let x=Ctx::new(&r,&r,&[]);
        let p=Params::from([("fast".into(),2.),("slow".into(),3.)]);
        let ts=run("kftsm_hh_cross",&x,4,9,&p).unwrap();
        assert_eq!(ts.len(),1);
        assert_eq!(ts[0].legs[0].i,7); assert_eq!(ts[0].dir,-1.);
    }
    #[test]
    fn invalid_parameters_are_rejected_before_window_allocation() {
        let r=rows(60); let x=Ctx::new(&r,&r,&[]);
        for value in [f64::NAN, f64::INFINITY, -1., 0., 2.5, 1_000_000.] {
            let p=Params::from([("period".into(),value)]);
            assert!(validate("kftsm_ma",&p).is_some());
            assert!(run("kftsm_ma",&x,0,59,&p).is_none());
        }
        assert!(validate("kftsm_ma",&Params::from([("unknown".into(),1.)])).is_some());
        assert!(validate("kftsm_hh_cross",&Params::from([("fast".into(),25.),("slow".into(),17.)])).is_some());
        assert!(validate("kftsm_ma",&Params::from([("filter".into(),0.15)])).is_none());
    }
    #[test]
    fn vidya_requires_history_and_does_not_substitute_price_for_zero_variance() {
        let mut r=rows(85);
        for (i,row) in r.iter_mut().enumerate() {
            let v=if i<25 {100.+i as f64} else {125.};
            row.open=v;row.close=v;row.high=v+1.;row.low=v-1.;
        }
        let x=Ctx::new(&r,&r,&[]);
        let v=vidya(&x.c,10,50);
        assert!(v[..49].iter().all(|n|n.is_nan()));
        assert_eq!(v[49],x.c[49]);
        assert_eq!(v[84],v[83]);
        assert!(run("kftsm_vidya",&x,0,48,&Params::new()).unwrap().is_empty());
        assert!(run("kftsm_vidya",&x,49,84,&Params::new()).unwrap().is_empty());
        assert!(validate("kftsm_vidya",&Params::from([("period".into(),20.),("per2".into(),20.)])).is_some());
    }
    #[test]
    fn adaptive_filter_is_strict_and_uses_only_prior_changes() {
        let r=rows(33);let x=Ctx::new(&r,&r,&[]);
        let mut trend=vec![100.;33];
        for i in 1..30 {trend[i]=trend[i-1]+if i%2==0 {1.} else {-1.};}
        trend[30]=trend[29]+1.;trend[31]=trend[30];trend[32]=trend[31];
        // Previous twenty changes are exactly ten +1 and ten -1, population SD1.
        assert!(adaptive_driver(&x,30,32,&trend,10,1.,0.).is_empty());
        assert!(adaptive_driver(&x,30,32,&trend,10,0.,1.).is_empty());
        trend[30]=trend[29]+1.000001;trend[31]=trend[30];trend[32]=trend[31];
        let ts=adaptive_driver(&x,30,32,&trend,10,1.,0.);
        assert_eq!(ts.len(),1);assert_eq!(ts[0].legs[0].i,30);assert!(ts[0].open);
    }
    #[test]
    fn outside_day_close_entry_precedes_next_day_stop_and_gaps_use_observed_open() {
        let mut r=rows(8);
        r[2].open=100.;r[2].high=110.;r[2].low=90.;r[2].close=109.;
        // The entry day's low90 is earlier than its close order; no same-bar stop.
        r[3].open=110.;r[3].high=112.;r[3].low=90.;r[3].close=111.; // equality is not penetration
        r[4].open=85.;r[4].high=89.;r[4].low=84.;r[4].close=86.;
        let x=Ctx::new(&r,&r,&[]);let ts=run("kftsm_outside_day",&x,1,7,&Params::new()).unwrap();
        assert_eq!(ts.len(),1);assert_eq!(ts[0].legs[0].i,2);assert_eq!(ts[0].exit_i,4);
        assert_eq!(ts[0].exit_px,85.);assert_eq!(ts[0].entry_kind,"close");
        // A fresh qualifying close on the final bar has no invented post-window trade.
        r[7].high=120.;r[7].low=80.;r[7].close=119.;
        let x=Ctx::new(&r,&r,&[]);let ts=run("kftsm_outside_day",&x,7,7,&Params::new()).unwrap();
        assert!(ts.is_empty());
    }
    #[test]
    fn percent_swing_both_extremes_uses_source_close_order_priority() {
        let mut r=rows(6);
        for (i,(hh,ll,cc)) in [(101.,99.,100.),(110.,100.,105.),(109.,95.,100.),(120.,90.,110.),(111.,100.,105.),(111.,100.,105.)].into_iter().enumerate() {
            r[i].open=cc;r[i].high=hh;r[i].low=ll;r[i].close=cc;
        }
        let x=Ctx::new(&r,&r,&[]);let ts=run("kftsm_swing",&x,3,5,&Params::new()).unwrap();
        // Both established levels are crossed on bar3. The source long IF comes
        // before the short ELSE, and both are close orders, not ordered stop fills.
        assert_eq!(ts[0].dir,1.);assert_eq!(ts[0].legs[0].i,3);assert_eq!(ts[0].legs[0].px,110.);
    }
    #[test]
    fn outside_day_final_opposite_close_liquidates_without_new_entry() {
        let mut r=rows(4);
        for (i,(oo,hh,ll,cc)) in [(100.,101.,99.,100.),(101.,105.,98.,104.),(104.,106.,100.,105.),(104.,107.,99.,99.5)].into_iter().enumerate() {
            r[i].open=oo;r[i].high=hh;r[i].low=ll;r[i].close=cc;
        }
        let x=Ctx::new(&r,&r,&[]);
        let ts=run("kftsm_outside_day",&x,0,3,&Params::new()).unwrap();
        assert_eq!(ts.len(),1);assert_eq!(ts[0].dir,1.);
        assert_eq!(ts[0].legs[0].i,1);assert_eq!(ts[0].exit_i,3);
        assert_eq!(ts[0].exit_px,99.5);assert!(!ts[0].open);
        assert_eq!(ts[0].reason,"opposite outside day");
    }
    #[test]
    fn abx_final_close_signal_is_unfilled_but_prior_signal_fills_final_open() {
        let mut r=rows(24);
        r[23].open=120.;r[23].high=121.;r[23].low=119.;r[23].close=120.;
        let x=Ctx::new(&r,&r,&[]);let p=Params::from([("period".into(),5.)]);
        assert!(run("kftsm_fc_bb_abx",&x,0,23,&p).unwrap().is_empty());
        // At nsd1 the outlier crosses; default nsd2 on five bars can equal the
        // band's upper edge, so use a valid explicitly requested parameter.
        let p=Params::from([("period".into(),5.),("nsd".into(),1.)]);
        assert!(run("kftsm_fc_bb_abx",&x,0,23,&p).unwrap().is_empty());
        r.push(Row{ticker:"T".into(),date:days_to(10981),open:123.,high:125.,low:122.,close:124.,volume:None});
        let x=Ctx::new(&r,&r,&[]);let ts=run("kftsm_fc_bb_abx",&x,0,24,&p).unwrap();
        assert_eq!(ts.len(),1);assert_eq!(ts[0].legs[0].i,24);assert_eq!(ts[0].legs[0].px,123.);
        assert!(ts[0].open);assert_eq!(ts[0].exit_px,124.);assert_eq!(ts[0].entry_kind,"open");
    }
    #[test]
    fn first_cross_rejects_inverted_oscillator_lengths() {
        assert!(validate("kftsm_raschke_first_cross",&Params::from([("osc1".into(),10.),("osc2".into(),5.)])).is_some());
    }
    #[test]
    fn adaptive_rsi_waits_for_actual_wilder_history_and_handles_flat_prices() {
        let prices:Vec<f64>=(0..32).map(|i|100.+i as f64).collect();
        let trend=adapt_rsi(&prices,10);
        assert!(trend[..10].iter().all(|v|*v==100.));
        assert_eq!(trend[10],110.); // ten actual rising changes => RSI100, sc1
        assert_eq!(trend[31],131.);
        let r=rows(50);let x=Ctx::new(&r,&r,&[]);
        assert!(run("kftsm_adapt_rsi",&x,0,49,&Params::new()).unwrap().is_empty());
    }
}
