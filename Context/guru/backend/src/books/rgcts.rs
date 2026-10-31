//! Murray A. Ruggiero, Jr., "Cybernetic Trading Strategies" (Wiley, 1997). Book id `rgcts`.
//! Only rules the book states exactly are Live; intermarket/futures/fundamental rules are NeedsData,
//! proprietary (MEM/TradeCycles), garbled or judgement rules are NotTestable. Book printed pages (p.) cited.
//! Sources, extraction and independent verification: library/rgcts/.
//! Conventions: TradeStation 4 bar logic. Orders are computed on the close of bar i-1 and act on bar i.
//! "buy at open" = next bar's open. Stop orders fill at the stop, or at the open if the bar gaps through it.
//! "1 point" = 1/PriceScale = 0.01 price units (PriceScale 100). Positions start flat at the selected start;
//! earlier bars only warm up indicators. A bar that would trigger both a long and a short entry while flat is
//! skipped (order unknown); at most one reversal per bar.
use crate::connors::{true_range, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "rgcts";
/// Admitted ids (authors/ruggiero/admission/SOURCE-CONTRACT.md).
pub const REVIEWED: &[&str] = &["rgc_cdow_mon_long", "rgc_tdom17_long", "rgc_tdom2_short", "rgc_cbo20", "rgc_volbo", "rgc_cbo17_10", "rgc_mom_first", "rgc_elliott", "rgc_ga1723", "rgc_cdow_fri_short", "rgc_dow_oc", "rgc_dowm_mon_dec_long", "rgc_dowm_thu_dec_short", "rgc_dowm_thu_jul_long", "rgc_runaway_gap", "rgc_runaway_gap_f5" /* intraday wave 2b: daily bars decide every trade */];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const FUT: &str = "needs the book's futures/intermarket series (T-Bond, Eurodollar, T-Bill, Dollar index, CRB, XAU, XOI, UTY, gold, crude, D-Mark, Yen, silver, Kodak etc.) - none is in the vault (only NIFTY indices, SPX/NDX/DJI, VIX/India VIX, NSE breadth)";
const FUND: &str = "needs monthly/weekly US fundamental series (CPI, T-Bill and long-bond yields, M2, consumer sentiment, unemployment duration/claims, prime and discount rates) - not in the vault";
const COT: &str = "needs CFTC Commitment of Traders weekly positions and T-Bond/coffee futures - not in the vault";
const DOLLAR_STOP: &str = "dollar stop on a futures contract: needs the futures contract and its point value; the vault has no futures data (cash index points are not substituted)";
const MEM: &str = "needs the proprietary MEM/TradeCycles functions (RSMemCycle1.2, RSMemPred, RSCycleTrend) whose code and internal settings the book does not give";

pub fn catalogue() -> Vec<StrategyDef> {
    let mk = |id, name, group, dir, status, rules: &'static [&'static str], notes| StrategyDef {
        id, name, group, session: 0, dir, status, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None,
    };
    let live = Status::Live;
    vec![
        // ---------------- Chapter 2: seasonal / calendar (S&P500, open-to-close) ----------------
        mk("rgc_dow_oc", "Day-of-week open-to-close (buy Mondays and Wednesdays)", "Ch. 2 · Day of week", "Long", live,
            &["F|\"The day-of-week bias in the S&P500 is revealed by measuring the difference between the close and open on a given day of the week\" (p.48)",
              "E|\"you can outperform buy and hold simply by buying on Mondays and Wednesdays\" (p.48): buy the open of every Monday and Wednesday (p.mon .. p.fri = 1 selects a day)",
              "X|Exit on the same day's close (open-to-close measurement, p.48)"],
            "Table 2.3 (p.49), S&P500 back-adjusted futures 1/3/83-4/12/96, points: Mon 282.69 (.43/day), Tue 8.45, Wed 168.60, Thu 42.35, Fri -117.26; buy and hold 367.60. Run on the cash index (SPX, NIFTY 50)."),
        mk("rgc_dowm_thu_jul_long", "Day of week in month: S&P long Thursdays in July", "Ch. 2 · Day of week in month", "Long", live,
            &["E|Table 2.4 (p.51): S&P500, start 1/3/83, Long, Thursday, July: buy the open of every July Thursday",
              "X|Exit on the close (\"how the month affects the day-of-week analysis on an open-to-close basis\", p.50)"],
            "Table 2.4: 69% win, avg $427.73, net $23,525.00 (1/3/83-4/12/96, no costs, $500/point)."),
        mk("rgc_dowm_mon_dec_long", "Day of week in month: S&P long Mondays in December", "Ch. 2 · Day of week in month", "Long", live,
            &["E|Table 2.4 (p.51): S&P500, 1/3/83, Long, Monday, December: buy the open", "X|Exit on the same day's close (open-to-close, p.50)"],
            "Table 2.4: 65% win, avg $536.82, net $29,525.00."),
        mk("rgc_dowm_thu_dec_short", "Day of week in month: S&P short Thursdays in December", "Ch. 2 · Day of week in month", "Short", live,
            &["E|Table 2.4 (p.51): S&P500, 1/3/83, Short, Thursday, December: sell short the open", "X|Cover on the same day's close (open-to-close, p.50)"],
            "Table 2.4: 63% win, avg $374.54, net $20,225.00."),
        mk("rgc_cdow_mon_long", "Conditional day of week: Monday after two down days (long)", "Ch. 2 · Conditional day of week", "Long", live,
            &["F|\"Let a 1 mean that a market finishes higher than it opens; -1 means a lower amount, and a zero (0) means we do not care\" (p.50); D1..D5 = the previous 1..5 trading days",
              "F|Monday: D1 = -1 (prior day close < open) and D2 = -1 (two days ago close < open); D3-D5 = 0 (p.50)",
              "E|Long the Monday open", "X|Exit the Monday close [reading: the chapter's open-to-close basis, p.48-50; the table gives no order type]"],
            "p.50: S&P500 4/21/82-4/23/96, $50 costs: net $25,125.00, avg $405.24, 68% win. Open-to-close is a documented reading."),
        mk("rgc_cdow_fri_short", "Conditional day of week: Friday after a down day (short)", "Ch. 2 · Conditional day of week", "Short", live,
            &["F|Friday: D1 = -1 (Thursday close < open), D2-D5 = 0 (p.50)",
              "E|Short the Friday open", "X|Cover the Friday close [reading: open-to-close basis of the chapter]"],
            "p.50: S&P500 4/21/82-4/23/96, $50 costs: net $61,470.00, avg $193.91, 54% win."),
        mk("rgc_tdom17_long", "Trading day of month 17, long 5 days", "Ch. 2 · Trading day of month", "Long", live,
            &["F|Trading day of the month = 1 for the first trading day of each calendar month (Larry Williams' concept, p.51)",
              "E|\"entering a position on the next open after a given trading day of the month\" (p.51): after the close of trading day 17, buy the next open",
              "X|\"and exiting on the open a given number of days later\" (p.51): hold 5 -> exit at the open 5 bars after entry"],
            "Table 2.5 (p.51): S&P500 4/21/82, Long, TDOM 17, hold 5: net $140,850.00, 68% win, avg $1,354.33 (no costs)."),
        mk("rgc_tdom2_short", "Trading day of month 2, short 2 days", "Ch. 2 · Trading day of month", "Short", live,
            &["E|After the close of trading day 2 of the month, sell short the next open (p.51)", "X|Cover at the open 2 bars after entry (Table 2.5: hold 2)"],
            "Table 2.5 (p.51): S&P500 4/21/82, Short, TDOM 2, hold 2: net $47,775.00, 55% win, avg $459.38."),
        mk("rgc_mon_tb26", "Buy Monday when T-Bonds are above their 26-day MA", "Ch. 2/8/9 · Intermarket Monday", "Long", Status::NeedsData(FUT),
            &["F|T-Bond futures close > 26-day SMA of T-Bond closes (p.50, 120)", "E|Buy the S&P500 Monday open", "X|Exit on the close (p.120)",
              "F|Variants: 20-day S&P/T-Bond correlation > .40 (Table 8.4); T-Bonds closed above open on Friday (Table 8.5); 35-day predictive correlation > 0 (Table 8.6); adaptive adverse-movement stop (Table 9.6)"],
            "Tables 8.3-8.6, 9.7: $89,100/417 trades; $88,200/268; $75,825/244; $55,050/88; adaptive stop $92,950."),
        mk("rgc_seasonal_sp", "S&P500 day-of-year seasonal (avg return > .03%, 8-day hold)", "Ch. 2 · Mechanical seasonality", "Long",
            Status::NotTestable("the seasonal is not defined exactly: trading vs calendar day of year, how the 8-day 'seasonal return' and the '70 percent reliable' rate are computed, and the entry/exit prices are not stated"),
            &["F|S&P500 cash from 1960; wait 2,500 days; seasonal at least 70% reliable; 8-day hold; seasonal return >= .03%; S&P below its 6-day SMA (p.53)"],
            "Table 2.6 (p.54): 3/24/71-6/28/96, 270.39 points, 69% win."),
        mk("rgc_rb_seasonal", "Ruggiero/Barna Seasonal Index systems", "Ch. 2 · Ruggiero/Barna index", "Long",
            Status::NotTestable("Table 2.2 step 3 \"Multiply this 5-day return by the proper percentage\" and the scaling window are not defined; the index is part of a commercial add-in"),
            &["E|S&P500: 5-day hold, trigger -.20, S&P below 10-day MA (Table 2.7); T-Bonds: 8-year window, 6-day hold, trigger > -.20, 6-period momentum < 0 (Table 2.8)"],
            "Table 2.7: 761.75 points, 60% win; Table 2.8: $57,593.75, 71% win."),
        mk("rgc_corn_counterseasonal", "Corn seasonal short and counterseasonal long", "Ch. 2 · Counterseasonal", "Long/Short", Status::NeedsData("needs cash/futures corn prices"),
            &["E|Sell corn on the first trading day after June 20; exit November 1 (p.57)", "E|If the short is not profitable after July 21, go long the first trading day after July 21 until the first trading day of the next year (p.57)"],
            "p.57: $28,487.50 (65% win) for the short; $38,537.50 (58%) with the counterseasonal long."),
        // ---------------- Chapter 1 / 8: intermarket ----------------
        mk("rgc_im_sp_tbond", "S&P500 vs T-Bonds: trend, ratio and divergence models", "Ch. 1, 7, 8, 18 · Intermarket S&P500", "Long/Short", Status::NeedsData(FUT),
            &["E|Long S&P when T-Bonds > 26-day MA, short below (p.27-28); T-Bond/S&P ratio vs 28-day average (p.29); Eurodollar/S&P ratio vs 10-day MA (p.29)",
              "E|Divergence: if S&P < Average(S&P,12) and T-Bonds > Average(T-Bonds,26) buy at open; reverse for sells (p.265; Table 1.4 lists 16/26, 12/30, 14/26, 12/26)",
              "E|Open-to-close divergence with T-Bills/T-Bonds (Table 1.5); MEM-confirmed version (Table 7.5)"],
            "Table 1.4 $348,175 (130 trades, 68%); Table 18.4 $321,275 dev set; Table 7.6 $326,775 (54 trades, 80%)."),
        mk("rgc_im_tbond", "T-Bond intermarket models (Utilities, Eurodollars, XAU, CRB, copper)", "Ch. 1, 8, 20 · Intermarket T-Bonds", "Long/Short", Status::NeedsData(FUT),
            &["E|Buy T-Bonds when the Philadelphia Utility average crosses above its 40-day SMA, sell below (p.29)",
              "E|T-Bond/Eurodollar divergence 24/32 (Table 1.6), T-Bond/XAU 6/34 (Table 1.7), CRB 10/16, T-Bond/UTY 8/24 (Table 1.8) with 12-day correlation > .5 filter (Table 8.2)",
              "E|GA-evolved rules using Eurodollars and XAU (generations 1197, 1909, 2329; Table 20.6)"],
            "Tables 1.6-1.8, 8.2, 20.5."),
        mk("rgc_im_commodities", "Gold, crude, Yen and stock intermarket models", "Ch. 1, 8 · Intermarket commodities and stocks", "Long/Short", Status::NeedsData(FUT),
            &["E|Crude vs Dollar index 40-day MA (Table 1.3) and divergence 12/18 (Table 1.10); crude/XOI 2/16; Dollar 40-day correlation < -.5 filter (Table 8.1)",
              "E|Gold: XAU > XAverage(XAU,12), gold < XAverage(gold,10), D-Mark > XAverage(D-Mark,20) buy at open; reverse to sell (Table 1.9)",
              "E|Yen/T-Bond 34-day difference divergence (Table 1.11); Kodak/silver 18/48 momentum divergence (Table 1.12); US Life 22/28; Pegasus 12/30",
              "E|Gold/CRB ratio 12/30 crossover with 24-day predictive correlation > .6, limit at XAverage(Close,80) (Table 8.7)"],
            "Tables 1.3, 1.9-1.12, 8.1, 8.8."),
        // ---------------- Chapter 3 / 5: fundamentals, COT ----------------
        mk("rgc_fund_djia", "Fundamental DJIA timing model (prime, discount, long bond)", "Ch. 3 · Fundamental timing", "Long", Status::NeedsData(FUND),
            &["E|If Prime < 12, prime rate is cut and last Fed discount move was lower, buy at close; if long-bond yield sets a 42-week low, buy at close (Table 3.8)", "X|If the prime rate is raised, go flat (Table 3.8)"],
            "Table 3.8: DJIA weekly 8/11/44-4/12/96, 90% win, 21 trades. Interest-rate forecasting models (Tables 3.1-3.7) are forecasts of yields, not trades on vault instruments."),
        mk("rgc_cot", "COT commercials/small traders index model", "Ch. 5 · Commitment of Traders", "Long/Short", Status::NeedsData(COT),
            &["F|COT Index = 100 x (Net - Lowest(Net,N))/(Highest(Net,N) - Lowest(Net,N)) (p.89)", "E|(COT commercials)[Lag1] > Ctrigger and (COT small) < Strigger then buy at open; reverse for sells (p.90; book prints 'buy' twice)"],
            "Table 5.1 T-Bonds $110,412.50 (28 trades); Table 5.2 coffee $100,983.75 (24 trades)."),
        // ---------------- Chapter 4: technical analysis ----------------
        mk("rgc_runaway_gap", "Runaway gaps (S&P500)", "Ch. 4 · Gap analysis", "Long/Short", live,
            &["E|\"If NextOpen>High then buy at NextOpen+Brakeout*Average(TrueRange,3) stop\" (Table 4.2, p.75): today opens above yesterday's high -> buy stop at today's open + k x 3-day average true range (through yesterday)",
              "E|\"If NextOpen<Low then sell at NextOpen+Brakeout*Average(TrueRange,3) stop\" (Table 4.2): printed with '+'; a sell stop above the market fills at the open, so the literal code sells every gap below yesterday's low at the open. p.sign = -1 gives the open - k x ATR reading [labelled option]",
              "X|\"ExitLong at close; ExitShort at close\" (Table 4.2): no overnight positions",
              "F|k: code input Breakout(.3); tested \"a breakout equal to .40\" (p.74) = default"],
            "Table 4.3 (p.75): S&P500 4/21/82-5/31/96, $50 costs: $80,150, long $17,650, short $62,500, 384 trades, avg $208.72, 55% win. Day trade on the cash index."),
        mk("rgc_runaway_gap_f5", "Runaway gaps with close-lower-than-5-days-ago filter", "Ch. 4 · Gap analysis", "Long/Short", live,
            &["F|\"We will require the S&P500 to close lower than 5 days ago\" (p.74), for both up and down gaps: yesterday's close < close 5 bars before it",
              "E|Runaway gap entries as in Table 4.2 (k = .40; literal '+' on the short side; p.sign = -1 option)", "X|Exit on the close"],
            "Table 4.4 (p.75): $86,150, long $23,650, short $62,500, 237 trades, avg $363.50, 59% win."),
        mk("rgc_oops_600", "OOPS gap fill with $600 money-management stop", "Ch. 4 · Gap analysis", "Long/Short", Status::NeedsData(DOLLAR_STOP),
            &["E|Sell when we open above yesterday's high on a stop at the high; buy when we gap below yesterday's low on a stop at yesterday's low (p.75)", "X|Exit on the close; $600.00 money-management stop (p.75)"],
            "Table 4.6: $145,950, 588 trades. The base OOPS without the stop (Table 4.5) duplicates ss_study_oops_buy/sell with exit 0 (close) and is not added again."),
        mk("rgc_cbo20", "Channel breakout 20 days (stop and reverse)", "Ch. 4 · Breakout systems", "Long/Short", live,
            &["E|\"Buy at the Highest (High, 20) + 1 point stop\" (p.77): highest high of the last 20 bars through yesterday + 0.01",
              "E|\"Sell at the Lowest (Low, 20) - 1 point stop\" (p.77)", "X|Stop and reverse: always in the market after the first breakout (no other exit)"],
            "Table 4.7 (p.77): futures (D-Mark $56,663.75/50% win, T-Bonds, crude, coffee...), start-5/17/96, $50 costs. Run on the index instead of futures."),
        mk("rgc_volbo", "Volatility breakout from the open (.6 x 3-day ATR, stop and reverse)", "Ch. 4 · Breakout systems", "Long/Short", live,
            &["E|\"Buy at opening price+.6 x Average (True Range, 3) stop\" (p.78): today's open + .6 x average true range of the last 3 bars through yesterday",
              "E|\"Sell at opening price-.6 x Average (True Range, 3) stop\" (p.78)", "X|Stop and reverse (the opposite order is the exit)"],
            "Table 4.8 (p.78): T-Bonds 9/27/79-5/31/96, $50 costs: $163,868.75, 1,282 trades, 43% win, avg $127.82. Index instead of T-Bonds."),
        mk("rgc_cbo17_10", "Modified channel breakout 17/10", "Ch. 4 · Market modes", "Long/Short", live,
            &["E|\"use a 17-day breakout to enter a trade\" (p.79): buy stop at the highest high of the last 17 bars, sell-short stop at the lowest low of the last 17 bars",
              "X|\"and a 10-day breakout to exit a trade\" (p.79): exit long on a stop at the 10-bar lowest low, exit short at the 10-bar highest high (from the bar after entry)"],
            "Table 4.10 (p.80): T-Bonds 9/28/79-5/31/96: $46,562.50 (OCR '$46,X2.50'), 180 trades, avg $258.68, 40% win, DD -$21,012.50. Table 4.11 (p.83): D-Mark $74,587.50, 204 trades, 48%."),
        mk("rgc_cbo17_10_adx", "Modified channel breakout with ADX trend-mode filter", "Ch. 4 · Market modes", "Long/Short",
            Status::NotTestable("Table 4.9 rule 4 ('rises from below 10 on 3 out of 4 days') and how rules 1-5 combine into one state are not defined"),
            &["F|14-day ADX: crosses above 25 = trending; crosses below 20 or below 45 from above = consolidating; rises from below 10 on 3 of 4 days = trend starting until the 5-day ADX difference < 0 (Table 4.9, p.79)"],
            "Table 4.10 filtered: $66,125, 85 trades, avg $777.94."),
        mk("rgc_cbo17_10_bomode", "Modified channel breakout with breakout-mode index", "Ch. 4 · Market modes", "Long/Short",
            Status::NotTestable("breakout-mode components are not exact: the efficiency condition is printed as 'when its value is +.20', TradeStation's Volatility(10) is unspecified, and the average length is optimized (4-30)"),
            &["F|Confusion: 5/10/20 momenta disagree; 9 and 14 SlowK between 40 and 60; Volatility(10) at its 20-bar low; efficiency(10) (p.80-82)", "E|Take 17/10 breakouts only when the averaged count >= 2 (p.82)"],
            "Table 4.12."),
        mk("rgc_mom_first", "Momentum precedes price (3-10 oscillator)", "Ch. 4 · Momentum precedes price", "Long/Short", live,
            &["F|\"Osc = Average(Close,3)-Average(Close,10)\" (Table 4.13, p.84)",
              "E|\"If Osc = Highest(Osc,LookBack) and Osc > 0 then buy at open; If Osc = Lowest(Osc,LookBack) and Osc < 0 then sell at open\" (LookBack 32): next bar's open, reverses an opposite position",
              "X|\"ExitShort at Highest(High,StopLen) stop; ExitLong at Lowest(Low,StopLen) stop\" (StopLen 12; window includes the signal bar, active from the bar after entry)"],
            "Table 4.14 (p.85): Yen 8/2/76-6/28/96, $50 costs: $135,550, long $78,812.50, short $56,737.50, 192 trades, 43% win, avg $705.99, DD -$11,437.50. Index instead of Yen."),
        mk("rgc_classic_examples", "Classic MA crossover, 30/70 oscillator and key-reversal rules", "Ch. 4 · Misused classic rules", "Long/Short",
            Status::NotTestable("presented as misuse examples: the oscillator length is not given, the MA crossover has no order type, and the key reversal has no exit"),
            &["E|MA crossover buy/sell (p.71, Table 4.1 D-Mark pairs 6/20, 10/20, 10/22); oscillator cross above 30 / below 70 (p.72); key reversal stop at yesterday's high/low (p.72)"], ""),
        // ---------------- Chapters 7, 9, 10, 14, 15 ----------------
        mk("rgc_mem_systems", "MEM cycle systems (adaptive channel breakout, MEM prediction, cycle trend)", "Ch. 7, 15 · Cycle-based trading", "Long/Short", Status::NotTestable(MEM),
            &["E|Buy at Highest(High, dominant cycle) stop; sell at Lowest(Low, dominant cycle) stop (p.114, Table 15.8)", "E|MEMPred turning-point system (Table 7.3); cycle trend filter (Table 7.1)"],
            "Tables 7.2, 7.4, 15.4-15.10."),
        mk("rgc_dmark_stops", "Channel breakout with scatter-chart stops (D-Mark)", "Ch. 9 · Intelligent exits", "Long/Short", Status::NeedsData(DOLLAR_STOP),
            &["E|20-day channel breakout +1 point stop and reverse (Table 9.2)", "X|-$1,800 protective stop (Table 9.3); $500 or breakeven stop on day 5 (Table 9.5)"],
            "Tables 9.3, 9.5 (D-Mark futures dollar stops)."),
        mk("rgc_equity_feedback", "Channel breakout with equity-curve feedback", "Ch. 10 · System feedback", "Long/Short",
            Status::NotTestable("the printed code contradicts itself and the prose: TrendChanLong accumulates SellEntry-BuyEntry when flipping short->long (the short trade), exits read 'Lowest(High,20)+1 point', entry signs are missing/inverted and the functions are called with an extra argument; the exact run cannot be reconstructed"),
            &["E|After 10 trades, take shorts only when long-side equity rose over LLen=120 bars and short-side equity fell over SLen=180 bars (Table 10.2); MA-of-equity filter 130/150 (Table 10.4)"],
            "Tables 10.3, 10.5 (D-Mark)."),
        mk("rgc_dark_cloud", "Dark cloud cover + 10-day RSI > 50 (gold)", "Ch. 14 · Candlesticks", "Short",
            Status::NotTestable("FuzzySmall, CandleColor and the Fuzzy functions' coefficients used by DarkCloud(15,1,1) are not printed (only FuzzyLong is); the CRB-correlation exit also needs CRB/gold data"),
            &["E|If DC>.5 and RSI(close,10)>50 then sell at open; exitshort at highest(high,5) stop (Table 14.7)"], "Table 14.7: gold 13 trades $150."),
        mk("rgc_nn_ml", "Neural network, machine induction and GA-evolution models", "Ch. 11, 17-20 · Advanced technologies", "Long/Short",
            Status::NotTestable("trained network weights / induced rules are not published; only the frameworks"), &["E|S&P500 intermarket neural network (Tables 18.7-18.11); C4.5/rough-set rules (ch. 19)"], ""),
        // ---------------- Chapter 13: Elliott Wave ----------------
        mk("rgc_elliott", "Elliott Wave counter system (wave 3/5 entries)", "Ch. 13 · Building the wave", "Long", live,
            &["F|\"Price=(H+L)/2; Osc535=Average(Price,5)-Average(Price,35)\" (Table 13.2, p.189)",
              "F|ElliottTrend: Osc=Highest(Osc,Len) and Trend=0 -> 1; Osc=Lowest(Osc,Len) and Trend=0 -> -1; Lowest(Osc,Len)<0 and Trend=-1 and Osc>-1*Trigger*Lowest(Osc,Len) -> 1; Highest(Osc,Len)>0 and Trend=1 and Osc<-1*Trigger*Highest(Osc,Len) -> -1 (Table 13.2)",
              "F|Wave345Up count exactly as Table 13.3 (wave 3 on ET flip up with Osc>0; wave 4 when Osc<=0 in wave 3; wave 5 when Price=Highest(Price,5) and Osc>=0 in wave 4; relabel 3 on new osc+price highs; -3 whenever ET=-1)",
              "E|\"If WavCount=3 and WavCount[1]<=0 then buy at open; If WavCount=5 and WavCount[1]=4 then buy at open; If WavCount=3 and WavCount[1]=5 then buy at open\" (Table 13.5)",
              "X|\"If Osc<0 then exitlong at open\" (Table 13.5)",
              "F|Len/Trig: the book's tested set \"a length of 20 and a trigger of .66\" (p.192) = default; the printed code inputs Len(80),Trig(.7) are a labelled option"],
            "Table 13.6 (p.193): D-Mark 2/13/75-3/18/96: $35,350, 57 trades, 51%, avg $690.35, DD -$10,237.50, PF 2.10. Table 13.7 Yen 8/2/76-3/18/96: $89,800, 51 trades, 51%, PF 4.16. Index instead of currencies."),
        // ---------------- Chapter 20: GA rule ----------------
        mk("rgc_ga1723", "GA-evolved T-Bond rule, generation 1723", "Ch. 20 · Genetic algorithms", "Long", live,
            &["E|\"If FastD(14)<61.9 and FastD(14)>31.9 and XAverage(C,7)>XAverage(C,15) then buy at open\" (Table 20.6, p.305)",
              "X|\"If BarsSinceEntry>4 then exitlong at Low-.5*Average(TrueRange,3) stop\" (Table 20.6): from the 5th bar after entry, a stop at that bar's low - .5 x 3-bar ATR for the next bar",
              "F|FastD(14): TradeStation built-in, not defined in the book. Default p.fastd=0 = 3-bar SMA of FastK(14); 1 = Lane's 3-bar sum ratio; 2 = exponential (alpha .5). XAverage = EMA alpha 2/(n+1) seeded on the first close [platform definitions, documented]"],
            "Table 20.5 (p.304): T-Bonds dev 1/1/86-12/31/94: $32,175, 74 trades, 66% win, avg $434.80, DD -$6,400, PF 1.90; test 1/1/95-10/4/96: $11,900, 17 trades, 65%. Only gen 1723 uses T-Bond data alone; 1197/1909/2329 need Eurodollars/XAU (rgc_im_tbond)."),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "rgc_dow_oc" => ("rgc_calendar", "Calendar effects (S&P500)", "Day of week"),
        "rgc_dowm_thu_jul_long" => ("rgc_calendar", "Calendar effects (S&P500)", "Thursday in July · long"),
        "rgc_dowm_mon_dec_long" => ("rgc_calendar", "Calendar effects (S&P500)", "Monday in December · long"),
        "rgc_dowm_thu_dec_short" => ("rgc_calendar", "Calendar effects (S&P500)", "Thursday in December · short"),
        "rgc_cdow_mon_long" => ("rgc_calendar", "Calendar effects (S&P500)", "Conditional Monday · long"),
        "rgc_cdow_fri_short" => ("rgc_calendar", "Calendar effects (S&P500)", "Conditional Friday · short"),
        "rgc_tdom17_long" => ("rgc_calendar", "Calendar effects (S&P500)", "TDOM 17 · long 5"),
        "rgc_tdom2_short" => ("rgc_calendar", "Calendar effects (S&P500)", "TDOM 2 · short 2"),
        "rgc_mon_tb26" | "rgc_seasonal_sp" | "rgc_rb_seasonal" | "rgc_corn_counterseasonal" => ("rgc_calendar", "Calendar effects (S&P500)", "Not runnable"),
        "rgc_runaway_gap" => ("rgc_gap", "Gap patterns", "Runaway gap"),
        "rgc_runaway_gap_f5" => ("rgc_gap", "Gap patterns", "Runaway gap + 5-day filter"),
        "rgc_oops_600" => ("rgc_gap", "Gap patterns", "OOPS + $600 stop (data missing)"),
        "rgc_cbo20" => ("rgc_breakout", "Breakout systems", "20-day channel, stop and reverse"),
        "rgc_cbo17_10" => ("rgc_breakout", "Breakout systems", "17-day entry / 10-day exit"),
        "rgc_volbo" => ("rgc_breakout", "Breakout systems", "Volatility breakout from the open"),
        "rgc_cbo17_10_adx" | "rgc_cbo17_10_bomode" | "rgc_dmark_stops" | "rgc_equity_feedback" | "rgc_mem_systems" => ("rgc_breakout", "Breakout systems", "Not runnable"),
        "rgc_mom_first" => ("rgc_momentum", "Momentum precedes price", "3-10 oscillator"),
        "rgc_elliott" => ("rgc_elliott", "Mechanical Elliott Wave", "Wave 3/5 counter"),
        "rgc_ga1723" => ("rgc_ga", "GA-evolved rules", "Generation 1723"),
        "rgc_im_sp_tbond" | "rgc_im_tbond" | "rgc_im_commodities" => ("rgc_intermarket", "Intermarket analysis", "Data missing"),
        "rgc_fund_djia" | "rgc_cot" => ("rgc_fundamental", "Fundamental and COT models", "Data missing"),
        "rgc_classic_examples" | "rgc_dark_cloud" | "rgc_nn_ml" => ("rgc_other", "Other methods", "Not testable"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(match id {
        "rgc_dow_oc" => vec![p("mon", "Trade Mondays (1/0)", 1., 0., 1., 1.), p("tue", "Trade Tuesdays", 0., 0., 1., 1.), p("wed", "Trade Wednesdays", 1., 0., 1., 1.),
            p("thu", "Trade Thursdays", 0., 0., 1., 1.), p("fri", "Trade Fridays", 0., 0., 1., 1.)],
        "rgc_dowm_thu_jul_long" | "rgc_dowm_mon_dec_long" | "rgc_dowm_thu_dec_short" | "rgc_cdow_mon_long" | "rgc_cdow_fri_short" => vec![],
        "rgc_tdom17_long" => vec![p("tdom", "Trading day of month (signal)", 17., 1., 23., 1.), p("hold", "Bars held (open to open)", 5., 1., 10., 1.)],
        "rgc_tdom2_short" => vec![p("tdom", "Trading day of month (signal)", 2., 1., 23., 1.), p("hold", "Bars held (open to open)", 2., 1., 10., 1.)],
        "rgc_runaway_gap" | "rgc_runaway_gap_f5" => vec![p("k", "Breakout x 3-day ATR (tested .40; code input .3)", 0.4, 0.1, 1.0, 0.05),
            p("sign", "Short stop sign: 1 = printed '+', -1 = open - k x ATR [labelled option]", 1., -1., 1., 2.)],
        "rgc_cbo20" => vec![p("n", "Channel bars", 20., 5., 60., 1.), p("pt", "One point (price units)", 0.01, 0., 1., 0.01)],
        "rgc_volbo" => vec![p("k", "x 3-day average true range", 0.6, 0.2, 1.5, 0.05), p("atr", "ATR bars", 3., 1., 10., 1.)],
        "rgc_cbo17_10" => vec![p("entry", "Entry breakout bars", 17., 5., 60., 1.), p("exit", "Exit breakout bars", 10., 3., 40., 1.)],
        "rgc_mom_first" => vec![p("lookback", "Oscillator high/low lookback", 32., 20., 40., 2.), p("stoplen", "Stop channel bars", 12., 2., 20., 2.),
            p("fast", "Fast SMA", 3., 2., 10., 1.), p("slow", "Slow SMA", 10., 5., 30., 1.)],
        "rgc_elliott" => vec![p("len", "Len (tested 20; code input 80)", 20., 20., 180., 10.), p("trig", "Trigger (tested .66; code input .7)", 0.66, 0.5, 1.0, 0.02)],
        "rgc_ga1723" => vec![p("dlo", "FastD lower bound", 31.9, 0., 100., 0.1), p("dhi", "FastD upper bound", 61.9, 0., 100., 0.1),
            p("ef", "Fast EMA", 7., 2., 30., 1.), p("es", "Slow EMA", 15., 5., 60., 1.), p("hold", "BarsSinceEntry before the stop", 4., 0., 20., 1.),
            p("m", "Stop = low - m x 3-bar ATR", 0.5, 0., 2., 0.05), p("fastd", "FastD: 0 SMA3 of FastK, 1 Lane sums, 2 EMA(.5) [platform]", 0., 0., 2., 1.)],
        _ => return None,
    })
}

// ------------------------------------------------------------------ helpers

pub fn days(d: &str) -> i64 {
    let y: i64 = d[0..4].parse().unwrap_or(1970);
    let m: i64 = d[5..7].parse().unwrap_or(1);
    let dd: i64 = d[8..10].parse().unwrap_or(1);
    let y2 = if m <= 2 { y - 1 } else { y };
    let era = y2.div_euclid(400);
    let yoe = y2 - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + dd - 1;
    era * 146097 + yoe * 365 + yoe / 4 - yoe / 100 + doy - 719468
}
/// 0 = Monday .. 6 = Sunday
pub fn weekday(d: &str) -> i64 { (days(d) + 3).rem_euclid(7) }
fn month(d: &str) -> i64 { d[5..7].parse().unwrap_or(0) }
/// Trading day of the month: 1 on the first bar of each calendar month.
pub fn tdom(x: &Ctx) -> Vec<usize> {
    let mut v = vec![0; x.r.len()];
    for i in 0..x.r.len() {
        v[i] = if i > 0 && x.r[i].date[0..7] == x.r[i - 1].date[0..7] { v[i - 1] + 1 } else { 1 };
    }
    v
}
pub fn sma(v: &[f64], n: usize) -> Vec<f64> { crate::connors::sma(v, n) }
/// TradeStation XAverage: alpha 2/(n+1), seeded with the first value.
pub fn xavg(v: &[f64], n: usize) -> Vec<f64> {
    let a = 2. / (n as f64 + 1.);
    let mut out = vec![f64::NAN; v.len()];
    for i in 0..v.len() {
        out[i] = if i == 0 || out[i - 1].is_nan() { v[i] } else { out[i - 1] + a * (v[i] - out[i - 1]) };
    }
    out
}
fn hh(v: &[f64], i: usize, n: usize) -> f64 { v[i + 1 - n..=i].iter().copied().fold(f64::NEG_INFINITY, f64::max) }
fn ll(v: &[f64], i: usize, n: usize) -> f64 { v[i + 1 - n..=i].iter().copied().fold(f64::INFINITY, f64::min) }

fn new_trade(dir: f64, i: usize, px: f64, kind: &'static str, sig: usize) -> Trade {
    Trade { dir, legs: vec![Leg { i, px, w: 1. }], exit_i: i, exit_px: px, reason: String::new(), entry_kind: kind, signal_i: sig, open: false, exits: vec![] }
}
fn close(mut t: Trade, i: usize, px: f64, why: &str, out: &mut Vec<Trade>) {
    t.exit_i = i;
    t.exit_px = px;
    t.signal_i = i;
    t.reason = why.into();
    out.push(t);
}
fn finish(cur: Option<Trade>, x: &Ctx, to: usize, out: &mut Vec<Trade>) {
    if let Some(mut t) = cur {
        t.exit_i = to;
        t.exit_px = x.c[to];
        t.signal_i = to;
        t.reason = "open".into();
        t.open = true;
        out.push(t);
    }
}
/// Buy stop touched on bar i -> fill at max(open, stop); sell stop -> min(open, stop).
fn buy_fill(x: &Ctx, i: usize, s: f64) -> Option<f64> { (s.is_finite() && x.h[i] >= s).then(|| x.o[i].max(s)) }
fn sell_fill(x: &Ctx, i: usize, s: f64) -> Option<f64> { (s.is_finite() && x.l[i] <= s).then(|| x.o[i].min(s)) }

/// Stop orders live on bar i (computed on the close of i-1). NaN = no order.
pub struct Orders { pub buy: f64, pub sell: f64, pub xlong: f64, pub xshort: f64 }

/// Stop-and-reverse engine with optional exit stops. `orders(i)` must use data up to i-1 only.
pub fn stop_engine(x: &Ctx, from: usize, to: usize, warm: usize, orders: &dyn Fn(usize) -> Orders, out: &mut Vec<Trade>) {
    let mut cur: Option<Trade> = None;
    for i in from.max(warm).max(1)..=to {
        let o = orders(i);
        let pos = cur.as_ref().map_or(0., |t| t.dir);
        let fresh = cur.as_ref().is_some_and(|t| t.legs[0].i == i);
        if pos > 0. && !fresh {
            let px_x = sell_fill(x, i, o.xlong);
            let px_r = sell_fill(x, i, o.sell);
            if px_x.is_some() || px_r.is_some() {
                let px = px_x.unwrap_or(f64::NEG_INFINITY).max(px_r.unwrap_or(f64::NEG_INFINITY));
                let why = if px_r.is_some_and(|r| r >= px) { "reverse: sell stop" } else { "exit long stop" };
                close(cur.take().unwrap(), i, px, why, out);
                if let Some(r) = px_r { cur = Some(new_trade(-1., i, r, "stop", i - 1)); }
            }
        } else if pos < 0. && !fresh {
            let px_x = buy_fill(x, i, o.xshort);
            let px_r = buy_fill(x, i, o.buy);
            if px_x.is_some() || px_r.is_some() {
                let px = px_x.unwrap_or(f64::INFINITY).min(px_r.unwrap_or(f64::INFINITY));
                let why = if px_r.is_some_and(|r| r <= px) { "reverse: buy stop" } else { "exit short stop" };
                close(cur.take().unwrap(), i, px, why, out);
                if let Some(r) = px_r { cur = Some(new_trade(1., i, r, "stop", i - 1)); }
            }
        } else if pos == 0. && i < to + 1 {
            match (buy_fill(x, i, o.buy), sell_fill(x, i, o.sell)) {
                (Some(b), None) => cur = Some(new_trade(1., i, b, "stop", i - 1)),
                (None, Some(s)) => cur = Some(new_trade(-1., i, s, "stop", i - 1)),
                _ => {}
            }
        }
    }
    finish(cur, x, to, out);
}

/// FastD(14) under the three platform readings (see rules text).
pub fn fastd(x: &Ctx, n: usize, mode: i32) -> Vec<f64> {
    let len = x.r.len();
    let mut num = vec![f64::NAN; len];
    let mut den = vec![f64::NAN; len];
    let mut k = vec![f64::NAN; len];
    for i in n - 1..len {
        let (h, l) = (hh(&x.h, i, n), ll(&x.l, i, n));
        num[i] = x.c[i] - l;
        den[i] = h - l;
        k[i] = if h > l { 100. * (x.c[i] - l) / (h - l) } else { f64::NAN };
    }
    match mode {
        1 => (0..len).map(|i| if i + 3 > n + 1 && i >= n + 1 {
                let d: f64 = den[i - 2..=i].iter().sum();
                if d > 0. { 100. * num[i - 2..=i].iter().sum::<f64>() / d } else { f64::NAN }
            } else { f64::NAN }).collect(),
        2 => {
            let mut out = vec![f64::NAN; len];
            for i in 0..len {
                out[i] = if k[i].is_nan() { f64::NAN } else if i == 0 || out[i - 1].is_nan() { k[i] } else { out[i - 1] + 0.5 * (k[i] - out[i - 1]) };
            }
            out
        }
        _ => sma(&k, 3),
    }
}

// ------------------------------------------------------------------ run

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if from > to || to >= x.r.len() {
        return None;
    }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let u = |k: &str| g(k).round().max(1.) as usize;
    let n = x.r.len();
    let mut out = vec![];
    let wd: Vec<i64> = x.r.iter().map(|r| weekday(&r.date)).collect();
    let mo: Vec<i64> = x.r.iter().map(|r| month(&r.date)).collect();
    let oc = |i: usize, dir: f64, why: &str, out: &mut Vec<Trade>| {
        let t = new_trade(dir, i, x.o[i], "open", i);
        close(t, i, x.c[i], why, out);
    };
    match id {
        "rgc_dow_oc" => {
            let on = [g("mon"), g("tue"), g("wed"), g("thu"), g("fri")];
            for i in from..=to {
                if wd[i] < 5 && on[wd[i] as usize] >= 0.5 { oc(i, 1., "open-to-close", &mut out); }
            }
        }
        "rgc_dowm_thu_jul_long" | "rgc_dowm_mon_dec_long" | "rgc_dowm_thu_dec_short" => {
            let (d, m, dir) = match id { "rgc_dowm_thu_jul_long" => (3, 7, 1.), "rgc_dowm_mon_dec_long" => (0, 12, 1.), _ => (3, 12, -1.) };
            for i in from..=to {
                if wd[i] == d && mo[i] == m { oc(i, dir, "open-to-close", &mut out); }
            }
        }
        "rgc_cdow_mon_long" | "rgc_cdow_fri_short" => {
            let (d, k, dir) = if id == "rgc_cdow_mon_long" { (0, 2, 1.) } else { (4, 1, -1.) };
            for i in from.max(k)..=to {
                if wd[i] == d && (1..=k).all(|j| x.c[i - j] < x.o[i - j]) { oc(i, dir, "open-to-close", &mut out); }
            }
        }
        "rgc_tdom17_long" | "rgc_tdom2_short" => {
            let dir = if id == "rgc_tdom17_long" { 1. } else { -1. };
            let (day, hold) = (u("tdom"), u("hold"));
            let td = tdom(x);
            let mut i = from;
            while i < to {
                if td[i] == day {
                    let e = i + 1;
                    let t = new_trade(dir, e, x.o[e], "open", i);
                    if e + hold <= to {
                        close(t, e + hold, x.o[e + hold], "open after hold", &mut out);
                        i = e + hold; // the exit bar may itself signal (cannot: same month day differs) -> continue scanning from it
                        continue;
                    }
                    finish(Some(t), x, to, &mut out);
                    break;
                }
                i += 1;
            }
        }
        "rgc_runaway_gap" | "rgc_runaway_gap_f5" => {
            let (k, sign) = (g("k"), if g("sign") < 0. { -1. } else { 1. });
            let atr3 = sma(&true_range(x.r), 3);
            for i in from.max(7)..=to {
                let a = atr3[i - 1];
                if !a.is_finite() { continue; }
                if id == "rgc_runaway_gap_f5" && !(x.c[i - 1] < x.c[i - 6]) { continue; }
                if x.o[i] > x.h[i - 1] {
                    if let Some(px) = buy_fill(x, i, x.o[i] + k * a) {
                        close(new_trade(1., i, px, "stop", i - 1), i, x.c[i], "exit on close", &mut out);
                    }
                } else if x.o[i] < x.l[i - 1] {
                    if let Some(px) = sell_fill(x, i, x.o[i] + sign * k * a) {
                        close(new_trade(-1., i, px, "stop", i - 1), i, x.c[i], "exit on close", &mut out);
                    }
                }
            }
        }
        "rgc_cbo20" => {
            let (len, pt) = (u("n"), g("pt"));
            stop_engine(x, from, to, len, &|i| Orders { buy: hh(&x.h, i - 1, len) + pt, sell: ll(&x.l, i - 1, len) - pt, xlong: f64::NAN, xshort: f64::NAN }, &mut out);
        }
        "rgc_volbo" => {
            let (k, m) = (g("k"), u("atr"));
            let atr = sma(&true_range(x.r), m);
            stop_engine(x, from, to, m + 1, &|i| Orders { buy: x.o[i] + k * atr[i - 1], sell: x.o[i] - k * atr[i - 1], xlong: f64::NAN, xshort: f64::NAN }, &mut out);
        }
        "rgc_cbo17_10" => {
            let (en, ex) = (u("entry"), u("exit"));
            stop_engine(x, from, to, en.max(ex), &|i| Orders { buy: hh(&x.h, i - 1, en), sell: ll(&x.l, i - 1, en), xlong: ll(&x.l, i - 1, ex), xshort: hh(&x.h, i - 1, ex) }, &mut out);
        }
        "rgc_mom_first" => {
            let (lb, sl) = (u("lookback"), u("stoplen"));
            let (f, s) = (sma(&x.c, u("fast")), sma(&x.c, u("slow")));
            let osc: Vec<f64> = (0..n).map(|i| f[i] - s[i]).collect();
            let ok = |i: usize| i + 1 >= lb && osc[i + 1 - lb..=i].iter().all(|v| v.is_finite());
            let sig = |i: usize| -> f64 {
                if !ok(i) { return 0.; }
                if osc[i] == hh(&osc, i, lb) && osc[i] > 0. { 1. } else if osc[i] == ll(&osc, i, lb) && osc[i] < 0. { -1. } else { 0. }
            };
            let mut cur: Option<Trade> = None;
            for i in from.max(sl).max(1)..=to {
                let s = sig(i - 1);
                let pos = cur.as_ref().map_or(0., |t| t.dir);
                if s != 0. && s != pos {
                    if let Some(t) = cur.take() { close(t, i, x.o[i], "reverse at open", &mut out); }
                    cur = Some(new_trade(s, i, x.o[i], "open", i - 1));
                    continue;
                }
                if let Some(t) = cur.as_ref() {
                    if t.legs[0].i < i {
                        let fill = if t.dir > 0. { sell_fill(x, i, ll(&x.l, i - 1, sl)) } else { buy_fill(x, i, hh(&x.h, i - 1, sl)) };
                        if let Some(px) = fill { close(cur.take().unwrap(), i, px, "StopLen channel stop", &mut out); }
                    }
                }
            }
            finish(cur, x, to, &mut out);
        }
        "rgc_elliott" => {
            let (len, trig) = (u("len"), g("trig"));
            let wave = elliott_wave(x, len, trig);
            let osc = elliott_osc(x);
            let mut cur: Option<Trade> = None;
            for i in from.max(1)..=to {
                let s = i - 1;
                if let Some(t) = cur.as_ref() {
                    if osc[s] < 0. && t.legs[0].i <= s { close(cur.take().unwrap(), i, x.o[i], "Osc<0 exit at open", &mut out); }
                    continue;
                }
                if s >= 1 {
                    let (w, w1) = (wave[s], wave[s - 1]);
                    if (w == 3 && w1 <= 0) || (w == 5 && w1 == 4) || (w == 3 && w1 == 5) {
                        cur = Some(new_trade(1., i, x.o[i], "open", s));
                    }
                }
            }
            finish(cur, x, to, &mut out);
        }
        "rgc_ga1723" => {
            let fd = fastd(x, 14, g("fastd").round() as i32);
            let (ef, es) = (xavg(&x.c, u("ef")), xavg(&x.c, u("es")));
            let atr3 = sma(&true_range(x.r), 3);
            let (dlo, dhi, hold, m) = (g("dlo"), g("dhi"), g("hold").round().max(0.) as usize, g("m"));
            let mut cur: Option<Trade> = None;
            for i in from.max(16)..=to {
                let s = i - 1;
                if let Some(t) = cur.as_ref() {
                    let e = t.legs[0].i;
                    if s > e && s - e > hold {
                        if let Some(px) = sell_fill(x, i, x.l[s] - m * atr3[s]) { close(cur.take().unwrap(), i, px, "BarsSinceEntry>4 stop", &mut out); }
                    }
                    continue;
                }
                if fd[s] < dhi && fd[s] > dlo && ef[s] > es[s] {
                    cur = Some(new_trade(1., i, x.o[i], "open", s));
                }
            }
            finish(cur, x, to, &mut out);
        }
        _ => return None,
    }
    Some(out)
}

/// Table 13.2 oscillator; 0 until the 35-bar average exists (EasyLanguage variable default).
pub fn elliott_osc(x: &Ctx) -> Vec<f64> {
    let price: Vec<f64> = (0..x.r.len()).map(|i| (x.h[i] + x.l[i]) / 2.).collect();
    let (a5, a35) = (sma(&price, 5), sma(&price, 35));
    (0..x.r.len()).map(|i| if a35[i].is_finite() && a35[i] != 0. { a5[i] - a35[i] } else { 0. }).collect()
}

/// Tables 13.2-13.4: ElliottTrend and Wave345Up, bar by bar from the first bar.
pub fn elliott_wave(x: &Ctx, len: usize, trig: f64) -> Vec<i32> {
    let n = x.r.len();
    let osc = elliott_osc(x);
    let price: Vec<f64> = (0..n).map(|i| (x.h[i] + x.l[i]) / 2.).collect();
    let mut et = vec![0i32; n];
    let mut trend = 0i32;
    let mut out = vec![0i32; n];
    let (mut wave, mut hi_osc, mut hi_osc2, mut hi_px, mut hi_px2) = (0i32, -999., -999., -999., -999.);
    for i in 0..n {
        if i + 1 >= len {
            let (h, l) = (hh(&osc, i, len), ll(&osc, i, len));
            let o = osc[i];
            if o == h && trend == 0 { trend = 1; }
            if o == l && trend == 0 { trend = -1; }
            if l < 0. && trend == -1 && o > -1. * trig * l { trend = 1; }
            if h > 0. && trend == 1 && o < -1. * trig * h { trend = -1; }
        }
        et[i] = trend;
        let (o, pr) = (osc[i], price[i]);
        if i >= 1 && et[i] == 1 && et[i - 1] == -1 && o > 0. { hi_osc = o; hi_px = pr; wave = 3; }
        if wave == 3 && hi_osc < o { hi_osc = o; }
        if wave == 3 && hi_px < pr { hi_px = pr; }
        if wave == 3 && o <= 0. && et[i] == 1 { wave = 4; }
        if wave == 4 && i >= 4 && pr == hh(&price, i, 5) && o >= 0. { wave = 5; hi_osc2 = o; hi_px2 = pr; }
        if wave == 5 && hi_osc2 < o { hi_osc2 = o; }
        if wave == 5 && hi_px2 < pr { hi_px2 = pr; }
        if hi_osc2 > hi_osc && hi_px2 > hi_px && wave == 5 && et[i] == 1 { wave = 3; hi_osc = hi_osc2; hi_px = hi_px2; hi_osc2 = -999.; hi_px2 = -999.; }
        if et[i] == -1 { wave = -3; hi_osc = -999.; hi_px = -999.; hi_osc2 = -999.; hi_px2 = -999.; }
        out[i] = wave;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn rows(n: usize) -> Vec<Row> {
        // consecutive calendar days starting Monday 2024-01-01 (weekends included; only dates matter here)
        (0..n).map(|i| {
            let d = days("2024-01-01") + i as i64;
            Row { ticker: "T".into(), date: civil(d), open: 100., high: 101., low: 99., close: 100., volume: None }
        }).collect()
    }
    fn civil(z: i64) -> String {
        let z = z + 719468;
        let era = z.div_euclid(146097);
        let doe = z - era * 146097;
        let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d = doy - (153 * mp + 2) / 5 + 1;
        let m = if mp < 10 { mp + 3 } else { mp - 9 };
        let y = yoe + era * 400 + if m <= 2 { 1 } else { 0 };
        format!("{y:04}-{m:02}-{d:02}")
    }
    #[test]
    fn weekday_and_tdom() {
        assert_eq!(weekday("2024-01-01"), 0);
        assert_eq!(weekday("1987-10-19"), 0);
        let r = rows(40);
        let x = Ctx::new(&r, &r, &[]);
        let t = tdom(&x);
        assert_eq!(t[0], 1);
        assert_eq!(t[30], 31);
        assert_eq!(t[31], 1); // Feb 1
    }
    #[test]
    fn runaway_gap_literal_short_sells_the_open() {
        let mut r = rows(20);
        r[10].open = 98.; r[10].low = 97.; r[10].high = 99.5; r[10].close = 97.5; // gap below 99
        r[11].open = 99.4; // no gap the next day
        let x = Ctx::new(&r, &r, &[]);
        let ts = run("rgc_runaway_gap", &x, 0, 19, &Params::new()).unwrap();
        assert_eq!(ts.len(), 1);
        assert_eq!(ts[0].dir, -1.); assert_eq!(ts[0].legs[0].px, 98.); assert_eq!(ts[0].exit_px, 97.5);
        let ts = run("rgc_runaway_gap", &x, 0, 19, &Params::from([("sign".into(), -1.)])).unwrap();
        // open - .4 x ATR(2) = 97.2 >= low 97 -> filled at 97.2
        assert_eq!(ts.len(), 1); assert!((ts[0].legs[0].px - 97.2).abs() < 1e-9);
    }
    #[test]
    fn cbo20_reverses_on_stop() {
        let mut r = rows(60);
        r[30].high = 103.; r[30].close = 102.5; // > 101 + .01
        r[45].low = 95.; r[45].open = 98.5; // < 99 - .01
        let x = Ctx::new(&r, &r, &[]);
        let ts = run("rgc_cbo20", &x, 25, 59, &Params::new()).unwrap();
        assert_eq!(ts.len(), 2);
        assert_eq!(ts[0].legs[0].i, 30); assert!((ts[0].legs[0].px - 101.01).abs() < 1e-9);
        assert_eq!(ts[0].exit_i, 45); assert_eq!(ts[0].exit_px, 98.5); // gap below the stop: open
        assert_eq!(ts[1].dir, -1.); assert!(ts[1].open);
    }
    #[test]
    fn tdom_long_exits_at_the_open_after_hold() {
        let mut r = rows(60);
        for (i, row) in r.iter_mut().enumerate() { row.open = 100. + i as f64; }
        let x = Ctx::new(&r, &r, &[]);
        let ts = run("rgc_tdom17_long", &x, 0, 59, &Params::new()).unwrap();
        assert_eq!(ts[0].legs[0].i, 17); assert_eq!(ts[0].exit_i, 22); assert_eq!(ts[0].exit_px, 122.);
    }
}

// Instrument admission: Ruggiero 2026-10-24 block, moved from instrument_policy.rs on 31 Oct 2026 (lane ST).
// Admission only for the module REVIEWED lists (authors/ruggiero/admission/SOURCE-CONTRACT.md); kept in one block to merge.
// Runs on actual indices, ETFs and stocks, US and India (index_etf_or_stock, nifty50 basket, declared application).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| matches!(def.book,"rgcts") && (crate::books::rgcts::reviewed(def.id)),
    label: "Murray Ruggiero tested these rules on futures (S&P 500, T-Bonds, D-Mark, Yen and others); runs on indices, ETFs and stocks, US and India, are a declared application, not the author's test.",
    basis: "Murray A. Ruggiero Jr., Cybernetic Trading Strategies (1997): conditional day of week p.50, trading day of month p.51 (Table 2.5), channel and volatility breakouts p.77-78, 17/10 breakout p.79, momentum precedes price Table 4.13 p.84, Elliott Wave counter Tables 13.2-13.5 p.189-192, GA rule generation 1723 Table 20.6 p.305. TradeStation order model. Reviewed in authors/ruggiero/admission/SOURCE-CONTRACT.md with independent chronological ledgers; no costs.",
};
