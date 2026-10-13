//! Working Money 2000-2004 (working-money.com; research drive Trading Knowledge II/@NewsLetters-Mags/Working Money 2000-2004,
//! copy in Books/Working Money/). Book id "wmny": the fully specified systems (entry AND exit) printed in the 2000-2004 articles.
//! Page refs = PDF page of the yearly compilation (e.g. "WM2003 p.194-205"). Notes, oracle and book checks: library/wmny/README.md.
//! Own simulators (no jkbwts::sim); helpers from atm03.
use crate::books::atm03::{by_open, cci_of, day_num, done, finish, hh, ll, new_trade, stdev, xavg};
use crate::connors::{atr, rsi, sma, Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use Status::*;

pub const BOOK: &str = "wmny";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, dir, status, vol: bool, series: &'static [&'static str], rules, notes, claim| StrategyDef { id, name, group, session: 0, dir, status, needs_volume: vol, needs_vix: false, needs_series: series, book: BOOK, rules, notes, claim };
    let c = crate::connors::claim;
    vec![
        d("wmny_belt_hold", "Bullish belt hold: buy next open, sell the open after a non-belt-hold day (Dahlquist)", "WM2001 p.46-53 · Should You Trade With Technical Indicators?", "Long", Live, false, &[],
          &["F|\"This type of white-bodied candle with no bottom wick is a bullish belt hold signal.\" (\"the stock opened at its lowest price for the day\") -> open = low and close > open",
            "E|\"whenever Ted sees a bullish belt hold, he buys the stock at the opening price ... on the following day.\"",
            "X|\"If by the end of that day a bullish belt hold candle does not occur, Ted will sell the stock when the market opens on the day after.\" (Fig.2: \"You would only sell on the day following a non-belt hold.\")"],
          "The intro's \"long white body with a small wick on top\" is not quantified; the operative definition (p.47) is used: open equal to the low, close above the open. Book: AT&T stock Q1-1999 and DJIA-30 stock stats; run on the indices.",
          c("3 (AT&T Q1-1999)", "100% (AT&T); typical 60-70%", "", "1-2 days", "AT&T Q1-1999: $10,000 -> $11,148 (buy & hold $10,332)")),
        d("wmny_lazy_year", "The Lazy Man's Guide: yearly high/low comparison on the last trading day (Sidewitz)", "WM2001 p.858-860 · The Lazy Man's Guide To Long-Term Investing", "Long", Live, false, &[],
          &["F|\"On the last trading day of each year, review the year's prices for one of the following four conditions\"",
            "E|\"1. If the high for the current year is above the high of the preceding year and the low for the year is above the low for the previous year ... conditions are favorable for committing funds.\"",
            "F|\"2. If the high for the year is below the high of the preceding year and the low for the year is above the low for the previous year ... not favorable for committing new funds, but previously allocated monies may remain fully invested.\"",
            "X|\"3. If the high for the year is below the high of the preceding year and the low for the year is below the low for the previous year ... all investments should be liquidated.\"",
            "E|\"4. If the high for the year is above the high of the preceding year, the low for the year is below the low for the previous year, and the lower low preceded the higher high ... conditions are favorable for committing funds.\""],
          "Acts at the close of the last trading day of the year [B: no order type]. Years with a tie, or an outside year whose higher high came first, match no condition: the position is kept [B]. A year counts only if the data has its first January session (within Jan 1-7). Out-of-market T-bill interest is not modelled.",
          c("n/a", "n/a", "", "years", "DJIA since 1920: ~90% of the gain; S&P 500 1958-2001: drawdowns 1975 (lesser 1963, 1971)")),
        d("wmny_engulf", "Engulfing pattern alone: buy after bullish, sell after bearish engulfing (Yamanaka, test 2)", "WM2001 p.968-977 · Candlestick Corner: The Engulfing Pattern", "Long", Live, false, &[],
          &["F|\"The second body must begin and end above and below the first body, respectively ... and the two bodies must be different colors. The shadows or 'wicks' are irrelevant\"",
            "E|\"I went back and traded MSFT using the engulfing pattern only, leaving out any trend formations\"; \"I decided I would buy in on the following day at approximately the opening prices.\"",
            "X|Sell at the next open after a bearish engulfing pattern (the sell side of the same test)"],
          "Bullish: black body then white body with open < prior close and close > prior open; bearish mirror. Long only (buy/sell of the stock). Test 1 (trend judged) and test 3 (\"$5 trailing stop\", reference price not given) are not testable. Book: MSFT 5/2000-8/2001; run on the indices.",
          c("7 closed + 1 open (MSFT)", "n/a", "", "", "MSFT 5/2000-8/2001: $1,000 -> $1,706")),
        d("wmny_zscore", "Z-score: 3-day SMA of z(20) crosses its 5-day SMA (Valcu)", "WM2002 p.106-111 · Z-Score Indicator", "Long", Live, false, &[],
          &["F|z = (C - Mov(C,20,S)) / Stdev(C,20) (sidebar; Excel STDEVP); A = 3-day SMA of z; B = 5-day SMA of A",
            "E|\"good long tradable moves took place at ... when the three-day simple moving average crossed above the five-day simple moving average of the three-day simple moving average\"",
            "X|Long moves end at the cross below (the listed long moves end on 3/12/02, 5/21/02, 8/23/02 = the listed short dates); \"crossings of z-score averages may signal the start or the end of a tradable trend\"",
            "E|Option p.short = 1: \"good shorting opportunities initiated when the three-day simple moving average crossed below\" (no short exit printed; with the option the next cross above ends it)"],
          "Fills at the close of the crossing day [B]. Population SD. Book dates are for the Nasdaq Composite (NDX/QQQ used).",
          c("4 long moves", "n/a", "", "2-4 weeks", "Nasdaq Composite: 9/21/01-10/17/01, 2/25/02-3/12/02, 5/8/02-5/21/02, 8/8/02-8/23/02")),
        d("wmny_candle_short", "Candlestick short: bullish engulf-type day after a negative regression slope, 2-day short (Peterson)", "WM2003 p.194-205 · Trading Candlestick Patterns (sidebar)", "Short", Live, false, &[],
          &["F|body1:=Abs(C-O); body2:=Abs(Ref(C,-1)-Ref(O,-1)); cond1:=LinRegSlope(C,opt1)<opt2; cond2:=body1>body2; cond3:=body2>opt3*(Ref(H,-1)-Ref(L,-1)); cond4:=C-O>0 AND Ref(C,-1)-Ref(O,-1)<0",
            "E|Short Entry = cond1 AND cond2 AND cond3 AND cond4; \"Delay entry one day\" (\"The entry is made on the open of the next day following the signal\")",
            "X|Short Exit = Ref(signal,-opt4), opt4 = 2; \"exit on the same day\" (\"the exit is always two days later\")",
            "F|\"Opt1 4 to 24 in steps of 4, Opt2 -0.5 to 0 in steps of 0.1, and Opt3 0.2 to 0.7 in steps of 0.1. Opt 4 is set to 2.\" Default = \"the same setup\" as the long test: seven-day slope negative, first body no less than 30% of its range"],
          "One position at a time (MetaStock ignores entries while in a trade): short at the open after the signal, cover at the close two bars after the signal. LinRegSlope = least-squares slope of the last opt1 closes (points per bar). The best grid point is not printed; defaults = the stated setup (7, 0, 0.3).",
          c("32 (best grid point)", "71.9% (23 W / 9 L)", "avg win/avg loss 2.46", "2 days", "QQQ ~1999-2003, profit for every grid combination")),
        d("wmny_trend_rsi21", "Trendilator T1: RSI(21) 30/70 crossover stop-and-reverse (Blackman)", "WM2003 p.304-315 · In Search Of The Trendilator", "Long/Short", Live, false, &[],
          &["E|Enter Long / Close Short: Cross(RSI(21), 30)", "E|Enter Short / Close Long: Cross(70, RSI(21))",
            "X|Stop-and-reverse: the Enter Short signal closes the long and the Enter Long signal closes the short (\"Enter Long / Close Short\", \"Enter Short / Close Long\")",
            "F|\"Trades were entered at the open the day after signals were generated.\""],
          "Wilder RSI. Starts flat; the first signal opens. Book: 50 US/Canadian stocks, 250 days (Jan 2002-Jan 2003); no index results.",
          c("n/a", "n/a", "", "", "Top scorer CERG +242% in three trades")),
        d("wmny_trend_rsi_ma", "Trendilator T2: RSI(15) crosses SMA(10) of RSI(5), stop-and-reverse (Blackman)", "WM2003 p.304-315 · In Search Of The Trendilator", "Long/Short", Live, false, &[],
          &["E|Enter Long / Close Short: Cross(RSI(Opt1), Mov(RSI(Opt2),Opt3,S))", "E|Enter Short / Close Long: Cross(Mov(RSI(Opt2),Opt3,S), RSI(Opt1))",
            "X|Stop-and-reverse: the Enter Short signal closes the long and the Enter Long signal closes the short (\"Enter Long / Close Short\", \"Enter Short / Close Long\")",
            "F|\"The top optimization period was 15, 5, and 10\"; entries at the next open"],
          "Defaults = the printed top optimization (15, 5, 10).", c("n/a", "n/a", "", "", "50 stocks: average annual group return 31.26%")),
        d("wmny_trend_srsi", "Trendilator T3: StochRSI(10) 20/80 crossover stop-and-reverse (Blackman)", "WM2003 p.304-315 · In Search Of The Trendilator", "Long/Short", Live, false, &[],
          &["F|SR = (Sum(RSI(10)-LLV(RSI(10),10),3)/Sum(HHV(RSI(10),10)-LLV(RSI(10),10),3))*100",
            "E|Enter Long / Close Short: Cross(SR, 20)", "E|Enter Short / Close Long: Cross(80, SR); entries at the next open", "X|Stop-and-reverse: the Enter Short signal closes the long and the Enter Long signal closes the short (\"Enter Long / Close Short\", \"Enter Short / Close Long\")"],
          "Wilder RSI; LLV/HHV include the current bar.", c("n/a", "n/a", "", "", "50 stocks: 21 with positive returns")),
        d("wmny_trend_srsi_ma", "Trendilator T4: StochRSI(8) crosses SMA(5) of StochRSI(25), stop-and-reverse (Blackman)", "WM2003 p.304-315 · In Search Of The Trendilator", "Long/Short", Live, false, &[],
          &["E|Enter Long / Close Short: Cross(StochRSI(opt1), Mov(StochRSI(opt2),opt3,S))", "E|Enter Short / Close Long: Cross(MASRSI, StochRSI)",
            "X|Stop-and-reverse: the Enter Short signal closes the long and the Enter Long signal closes the short (\"Enter Long / Close Short\", \"Enter Short / Close Long\")",
            "F|\"The highest-scoring group was for the periods of 8, 25, and 5 days\"; entries at the next open"],
          "StochRSI(n) = Sum(RSI(n)-LLV(RSI(n),n),3)/Sum(HHV(RSI(n),n)-LLV(RSI(n),n),3)*100. Defaults = printed best (8, 25, 5).", None),
        d("wmny_willr", "Williams %R entry and exit (Peterson)", "WM2003 p.503-510 · Adjusting Indicators For Entry And Exit", "Long", Live, false, &[],
          &["E|cond1:=Alert(WillR(opt1)<opt2,opt9); cond2:=WillR(opt3)>opt4 -> enter long, \"entered with a one-day delay\"",
            "X|cond1:=Alert(WillR(opt5)>opt6,opt10); cond2:=WillR(opt7)<opt8 -> exit long",
            "F|\"The answer for opt1, . . . , opt10 is 12, -70, 4, -100, 20, -30, 8, -40, 2, and 2.\""],
          "WillR = -100 x (HHV(H,n) - C)/(HHV(H,n) - LLV(L,n)); Alert(x,2) = x today or yesterday. Exit timing not printed in this article; default = the author's stated convention (\"Delay entry one day, exit on the same day\", WM2003 p.205, WM2002 p.78): exit at the signal day's close. p.exit_next_open = 1 exits at the next open instead.",
          c("10", "90% (9 of 10)", "", "", "Nasdaq Composite ~10/2002-11/2003: 150+% annual vs 87% buy & hold")),
        d("wmny_pullback", "Three-day pullback (Palmquist)", "WM2004 p.47-54 · Trading Pullbacks In Trends (Figure 2)", "Long", Live, true, &[],
          &["F|\"1. The stock must be in an established uptrend.\" = \"closed above the 200-day simple moving average, and the 20-day slopes of the 28-, 50-, and 20-day simple moving averages were positive.\"",
            "F|\"2. The price must be above $15.00. 3. The 21-day average volume must be above 300,000 shares. 4. The stock must have made three consecutive lower highs.\"",
            "E|\"5. The setup triggers if rules 1-4 were true yesterday, and today's price rises above yesterday's high. 6. If the setup triggers, enter at the open the following day.\"",
            "X|\"7. Hold the position for three days, then exit the next day at the open.\""],
          "Slope = 20-bar least-squares slope of each SMA (AIQ Slope) [B]; p.slope_diff = 1 uses SMA - SMA 20 bars ago. Three lower highs: H[y] < H[y-1] < H[y-2] < H[y-3]. Held on the entry day and the next two days, sold at the following open. One position at a time. Book: US stock universe (2,371 trades, 64% winners, 9/5-12/4/2003). NIFTY has no volume (rule 3 cannot be checked).",
          c("2,371 (stocks)", "64%", "", "3 days", "US stocks 9/5/2003-12/4/2003")),
        d("wmny_dji_5_21m", "DJIA monthly 5/21-month SMA crossover (Blackman)", "WM2004 p.169-174 · Do P/E Ratios Really Matter?", "Long", Live, false, &[],
          &["E|\"A buy was generated when the shorter-period signal line crossed above a longer-period base line.\" \"a good combination was a five- and 21-month simple moving average (SMA) crossover\"",
            "X|\"The position was exited when the signal line crossed below the base line.\""],
          "Monthly closes = last session of each calendar month; trades at that close [B]. Short side only reported for the P/E version (Shiller data, not in the vault). DJI data starts 1970.",
          c("22 buys / 21 exits", "n/a", "", "49.8 yrs in market", "DJIA 1920-1/30/2004: first signal Oct 1924, 9.33%/yr while in the market, +10,574.22 Dow points")),
        d("wmny_nhnl_ratio", "Nasdaq new-high/new-low ratio MACD system (Peterson, A)", "WM2002 p.71-81 · Trading Systems Using Multiple Indicators", "Long/Short", Live, false, &["NASDAQ_NEW_HIGHS", "NASDAQ_NEW_LOWS"],
          &["F|ratio:=nh/(nh+nl); diff:=Mov(ratio,opt1,E)-Mov(ratio,opt2,E); signal:=Mov(diff,opt3,E)",
            "E|Enter Long: ratio>opt4 AND signal>diff; Enter Short: ratio<opt5 AND signal<diff",
            "X|Close Long: ratio<opt6 AND signal<diff; Close Short: ratio>opt7 AND signal<diff (as printed)",
            "F|\"opt1 through opt 7 were 7, 16, 2, 0.4, 1, 0.3, and 0.6\"; \"a one-day delay on entry but exits on the same day as the day of the close signal\""],
          "Series: NASDAQ_NEW_HIGHS / NASDAQ_NEW_LOWS = daily count of Nasdaq issues at 52-week highs / lows (eSignal x.nasd-h / x.nasd-l). Not in the vault: data gap. MetaStock EMA seeded with the first value [B]. An opposite entry while in a trade reverses at the next open.",
          c("n/a", "n/a", "", "", "QQQ ~mid-1999-2002: $1,000 -> +$20,000 (50% margin)")),
        d("wmny_nhnl_cci", "NH/NL ratio + CCI(6) system (Peterson, B)", "WM2002 p.71-81 · Trading Systems Using Multiple Indicators", "Long/Short", Live, false, &["NASDAQ_NEW_HIGHS", "NASDAQ_NEW_LOWS"],
          &["E|Long: (nhnlratio>.4 AND Alert(CCI(opt1)<opt5,opt4)) OR (signal>diff AND Alert(CCI(opt1)<opt5,opt4)); diff = Mov(ratio,7,E)-Mov(ratio,16,E), signal = Mov(diff,2,E)",
            "X|Close long: nhnlratio<.6 AND signal<diff AND CCI(opt1)>opt8",
            "E|Enter short: Alert(CCI(opt2)>opt7,opt6)", "X|Close short: nhnlratio>.6 AND signal>diff AND CCI(opt2)<opt9",
            "F|\"opt1=6, opt2=6, opt4=9, opt5=-140, opt6=4, opt7=30, opt8=180, and opt9=-130\"; \"Entries are at the open of the day following the signal, while positions are closed on the same day.\""],
          "Same series as wmny_nhnl_ratio (data gap). CCI = MetaStock CCI (typical price, mean deviation). Long entry has priority when both entries fire [B].",
          c("161", "65.2% (105 W / 56 L)", "avg win $1,755.16 / avg loss $500.14", "", "QQQ ~1999-2002: $1,000 -> $160,000 (50% margin); max consecutive W 14 / L 4")),
        d("wmny_pct_nh", "% new highs: 10-day SMA of NH/(NH+NL) crosses 30 up / 70 down (Hurley, Appel)", "WM2003 p.147-156 · When The Market Speaks, It Pays To Listen", "Long", Live, false, &["NYSE_NEW_HIGHS", "NYSE_NEW_LOWS"],
          &["F|\"% new highs: A 10-day simple moving average of new 52-week highs divided by the total of new 52-week highs and new 52-week lows (NH / (NH+NL)).\"",
            "E|\"The best signals come when the indicator crosses ... from below to above 30\" (\"flashed a buy signal on August 19 when it crossed back above 30\")",
            "X|\"crosses from above to below 70\" (\"flashed a sell signal by slipping through the key 70% mark\"; a decline before reaching 70 gives no sell: \"it has no ability to register a sell signal\")"],
          "Series: NYSE_NEW_HIGHS / NYSE_NEW_LOWS (daily NYSE 52-week highs / lows); the exchange is not named (examples: S&P 500, Nasdaq) [B]. Not in the vault: data gap. Fills at the close [B].", None),
        d("wmny_bond_band", "Bonds above Wilder volatility-band average: own the S&P 500 (Blackman / Williams)", "WM2004 p.61-63 · Are Stocks And Bonds Recoupling?", "Long", Live, false, &["TBOND_FUT_HIGH", "TBOND_FUT_LOW", "TBOND_FUT_CLOSE"],
          &["F|x:=Input(\"MA Periods\",2,1000,10); z:=Input(\"VolatilityFactor\",1,5,2.5); LowerB:=(Ref(LLV(C,7),-1)+(Ref(ATR(7),-1)*z)); UpperB:=(Ref(HHV(C,7),-1)-(Ref(ATR(7),-1)*z)); Mov((LowerB+UpperB)/2,x,S) (weekly T-bond chart)",
            "E|\"buying the S&P 500 index when bonds move above the volatility band\"", "X|\"and exiting when bonds move below the magenta line\""],
          "Weekly bars (Monday-Sunday) built from TBOND_FUT_* daily continuous 30-year T-bond futures; signal at the week's last close, S&P traded at that close [B]. Not in the vault: data gap.",
          c("3 (since 4/2003)", "n/a", "", "", "weekly SPX vs T-bonds, 'only three trades'")),
        d("wmny_not_testable", "Other Working Money 2000-2004 rules seen but not testable", "WM 2000-2004", "Long/Short", NotTestable("discretionary trend/pattern judgement, missing parameters or no exit (see README)"), false, &[],
          &["E|Moody 50/150/5/10/15 EMA retracements (\"series of higher lows and higher highs\" unquantified, stop \"just beyond\")",
            "E|Peterson %R entry + close < SMA(n) exit (n not printed); Peterson CCI alone (thresholds not printed)",
            "E|Yamanaka engulfing test 1 (trend judged) and test 3 (\"$5 trailing stop\" reference not given)",
            "E|Blackman P/E 5/21-month crossover (needs Shiller 10-year P/E; short side ditto); Teseo P&F 5/10 MA (box size not given)"],
          "Duplicates of vault strategies were skipped: Merriman 100-day SMA = apta_ma100_dji; Zweig 4% weekly = mzwws/mbsot; Zweig prime rate = mzwws_prime_rate; Boucher T-bill = mbsot_monetary_models; Wu CCI +/-100 = rwmts_cci_trend; Weissman = rwmts.", None),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let v: &'static str = catalogue().into_iter().find(|d| d.id == id)?.name;
    Some(match id {
        "wmny_belt_hold" | "wmny_engulf" | "wmny_candle_short" => ("wmny_candles", "Working Money · candlestick systems", v),
        "wmny_trend_rsi21" | "wmny_trend_rsi_ma" | "wmny_trend_srsi" | "wmny_trend_srsi_ma" => ("wmny_trendilator", "Working Money · Trendilator (Blackman)", v),
        "wmny_nhnl_ratio" | "wmny_nhnl_cci" | "wmny_pct_nh" | "wmny_bond_band" => ("wmny_breadth", "Working Money · breadth / intermarket (data gap)", v),
        "wmny_lazy_year" | "wmny_dji_5_21m" | "wmny_zscore" | "wmny_willr" | "wmny_pullback" => ("wmny_timing", "Working Money · timing systems", v),
        "wmny_not_testable" => ("wmny_other", "Working Money · not testable", v),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(match id {
        "wmny_belt_hold" | "wmny_lazy_year" | "wmny_engulf" | "wmny_not_testable" => vec![],
        "wmny_zscore" => vec![p("n", "Z-score length", 20., 5., 60., 1.), p("a", "Fast SMA of z", 3., 1., 10., 1.), p("b", "SMA of the fast SMA", 5., 2., 20., 1.), p("short", "1 = also short on the cross below (option)", 0., 0., 1., 1.)],
        "wmny_candle_short" => vec![p("opt1", "Regression slope length", 7., 4., 24., 1.), p("opt2", "Slope below", 0., -0.5, 0., 0.1), p("opt3", "Prior body > x of prior range", 0.3, 0.2, 0.7, 0.1), p("opt4", "Exit bars after the signal", 2., 1., 5., 1.)],
        "wmny_trend_rsi21" => vec![p("n", "RSI length", 21., 5., 50., 1.), p("lo", "Buy cross above", 30., 10., 50., 1.), p("hi", "Sell cross below", 70., 50., 90., 1.)],
        "wmny_trend_rsi_ma" => vec![p("o1", "RSI length (line)", 15., 3., 50., 2.), p("o2", "RSI length (averaged)", 5., 3., 60., 2.), p("o3", "SMA of RSI", 10., 5., 20., 5.)],
        "wmny_trend_srsi" => vec![p("n", "StochRSI length", 10., 3., 50., 1.), p("lo", "Buy cross above", 20., 5., 50., 1.), p("hi", "Sell cross below", 80., 50., 95., 1.)],
        "wmny_trend_srsi_ma" => vec![p("o1", "StochRSI length (line)", 8., 3., 50., 2.), p("o2", "StochRSI length (averaged)", 25., 3., 60., 2.), p("o3", "SMA of StochRSI", 5., 5., 20., 5.)],
        "wmny_willr" => vec![p("opt1", "Entry %R length", 12., 2., 40., 1.), p("opt2", "Entry: %R was below", -70., -100., 0., 5.), p("opt3", "Entry trigger %R length", 4., 2., 40., 1.), p("opt4", "Entry trigger %R above", -100., -100., 0., 5.),
            p("opt5", "Exit %R length", 20., 2., 40., 1.), p("opt6", "Exit: %R was above", -30., -100., 0., 5.), p("opt7", "Exit trigger %R length", 8., 2., 40., 1.), p("opt8", "Exit trigger %R below", -40., -100., 0., 5.),
            p("opt9", "Entry alert bars", 2., 1., 10., 1.), p("opt10", "Exit alert bars", 2., 1., 10., 1.), p("exit_next_open", "1 = exit at the next open (option)", 0., 0., 1., 1.)],
        "wmny_pullback" => vec![p("ma", "Trend SMA", 200., 50., 300., 10.), p("slope_n", "Slope length", 20., 5., 50., 1.), p("price", "Minimum price", 15., 0., 100., 1.), p("vol_n", "Volume average length", 21., 5., 50., 1.), p("vol", "Minimum average volume", 300000., 0., 5000000., 50000.),
            p("lh", "Consecutive lower highs", 3., 1., 6., 1.), p("hold", "Days held before the exit open", 3., 1., 10., 1.), p("slope_diff", "1 = slope as SMA minus SMA n bars ago (option)", 0., 0., 1., 1.)],
        "wmny_dji_5_21m" => vec![p("fast", "Fast SMA (months)", 5., 2., 12., 1.), p("slow", "Slow SMA (months)", 21., 10., 40., 1.)],
        "wmny_nhnl_ratio" => vec![p("opt1", "Fast EMA", 7., 2., 20., 1.), p("opt2", "Slow EMA", 16., 5., 40., 1.), p("opt3", "Signal EMA", 2., 1., 10., 1.), p("opt4", "Long: ratio above", 0.4, 0., 1., 0.05), p("opt5", "Short: ratio below", 1., 0., 1., 0.05), p("opt6", "Close long: ratio below", 0.3, 0., 1., 0.05), p("opt7", "Close short: ratio above", 0.6, 0., 1., 0.05)],
        "wmny_nhnl_cci" => vec![p("opt1", "CCI length (long)", 6., 3., 30., 1.), p("opt2", "CCI length (short)", 6., 3., 30., 1.), p("opt4", "Long alert bars", 9., 1., 20., 1.), p("opt5", "Long: CCI below", -140., -300., 0., 10.), p("opt6", "Short alert bars", 4., 1., 20., 1.),
            p("opt7", "Short: CCI above", 30., -100., 300., 10.), p("opt8", "Close long: CCI above", 180., 0., 300., 10.), p("opt9", "Close short: CCI below", -130., -300., 0., 10.)],
        "wmny_pct_nh" => vec![p("n", "SMA length", 10., 2., 30., 1.), p("lo", "Buy cross above (%)", 30., 5., 50., 1.), p("hi", "Sell cross below (%)", 70., 50., 95., 1.)],
        "wmny_bond_band" => vec![p("x", "MA periods (weeks)", 10., 2., 50., 1.), p("z", "Volatility factor", 2.5, 1., 5., 0.1), p("k", "LLV/HHV/ATR length", 7., 3., 20., 1.)],
        _ => return None,
    })
}

/// a crosses above b at i (MetaStock Cross).
fn up(a: &[f64], b: &[f64], i: usize) -> bool { i > 0 && a[i] > b[i] && a[i - 1] <= b[i - 1] && !a[i - 1].is_nan() && !b[i - 1].is_nan() }
/// Least-squares slope of the last n values ending at i (MetaStock LinRegSlope, AIQ Slope).
fn lr_slope(v: &[f64], n: usize) -> Vec<f64> {
    let nf = n as f64;
    let mx = (nf - 1.) / 2.;
    let sxx: f64 = (0..n).map(|k| (k as f64 - mx).powi(2)).sum();
    (0..v.len()).map(|i| if i + 1 < n { f64::NAN } else { (0..n).map(|k| (k as f64 - mx) * v[i + 1 - n + k]).sum::<f64>() / sxx }).collect()
}
fn willr(x: &Ctx, n: usize) -> Vec<f64> {
    (0..x.c.len()).map(|i| { let (a, b) = (hh(&x.h, i, n), ll(&x.l, i, n)); if a.is_nan() || !(a > b) { f64::NAN } else { -100. * (a - x.c[i]) / (a - b) } }).collect()
}
/// MetaStock Alert(cond, n): cond true on this bar or any of the previous n-1 bars.
fn alert(c: &[bool], n: usize) -> Vec<bool> { (0..c.len()).map(|i| c[i.saturating_sub(n - 1)..=i].iter().any(|&b| b)).collect() }
/// Simple average recomputed over each window (MetaStock Mov(...,S)); no running sum, so equal inputs give equal outputs.
fn avg(v: &[f64], n: usize) -> Vec<f64> { (0..v.len()).map(|i| if i + 1 < n { f64::NAN } else { v[i + 1 - n..=i].iter().sum::<f64>() / n as f64 }).collect() }
/// Blackman StochRSI: Sum(RSI-LLV(RSI,n),3)/Sum(HHV(RSI,n)-LLV(RSI,n),3)*100.
fn srsi3(c: &[f64], n: usize) -> Vec<f64> {
    let r = rsi(c, n);
    // only full windows of valid RSI values (a partial window would put +/-inf into the running sums)
    let ok = |i: usize| i + 1 >= n && !r[i + 1 - n].is_nan();
    let num: Vec<f64> = (0..c.len()).map(|i| if ok(i) { r[i] - ll(&r, i, n) } else { f64::NAN }).collect();
    let den: Vec<f64> = (0..c.len()).map(|i| if ok(i) { hh(&r, i, n) - ll(&r, i, n) } else { f64::NAN }).collect();
    // Sum(...,3)/Sum(...,3): ratio of the 3-bar sums, exactly as printed
    let sum3 = |v: &[f64], i: usize| if i < 2 { f64::NAN } else { v[i - 2..=i].iter().sum::<f64>() };
    (0..c.len()).map(|i| { let (a, b) = (sum3(&num, i), sum3(&den, i)); if b > 0. { 100. * a / b } else { f64::NAN } }).collect()
}
/// Market-on-close state machine: decide(i, pos) at the close of bar i; a change fills at that close.
fn by_close(x: &Ctx, from: usize, to: usize, decide: impl Fn(usize, i8) -> Option<i8>) -> Vec<Trade> {
    let (mut out, mut cur, mut pos) = (vec![], None::<Trade>, 0i8);
    for i in from..=to {
        let Some(want) = decide(i, pos) else { continue };
        if want == pos { continue; }
        if let Some(t) = cur.take() { out.push(done(t, i, x.c[i], i, "signal: exit at the close")); }
        if want != 0 { cur = Some(new_trade(want as f64, i, x.c[i], "close")); }
        pos = want;
    }
    finish(&mut out, cur, x, to);
    out
}
/// Entry at the next open, exit at the close of the exit-signal day (MetaStock "delay entry one day, exit on the same day").
/// sig(i) -> wanted entry direction on bar i's close; ex(i, dir) -> exit on bar i's close. An opposite entry reverses at the next open.
fn open_in_close_out(x: &Ctx, from: usize, to: usize, sig: impl Fn(usize) -> i8, ex: impl Fn(usize, i8) -> bool) -> Vec<Trade> {
    let (mut out, mut cur, mut pend) = (vec![], None::<Trade>, 0i8);
    for i in from..=to {
        if pend != 0 {
            if let Some(t) = cur.take() { out.push(done(t, i, x.o[i], i - 1, "reverse at the open")); }
            cur = Some(new_trade(pend as f64, i, x.o[i], "open"));
            pend = 0;
        }
        if let Some(t) = cur.take() {
            let d = t.dir as i8;
            if ex(i, d) { out.push(done(t, i, x.c[i], i, "exit signal: same-day close")); } else { cur = Some(t); }
        }
        if i == to { break; }
        let s = sig(i);
        let held = cur.as_ref().map_or(0, |t| t.dir as i8);
        if s != 0 && s != held { pend = s; }
    }
    finish(&mut out, cur, x, to);
    out
}
fn year(d: &str) -> i64 { d[0..4].parse().unwrap_or(0) }
fn month(d: &str) -> i64 { d[0..4].parse::<i64>().unwrap_or(0) * 12 + d[5..7].parse::<i64>().unwrap_or(0) }

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    let n = x.r.len();
    if !p.keys().all(|k| schema.iter().any(|d| d.key == k)) || from > to || to >= n { return None; }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(0., |d| d.def));
    let u = |k: &str| g(k).max(1.) as usize;
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    Some(match id {
        "wmny_belt_hold" => {
            let belt = |i: usize| o[i] <= l[i] && c[i] > o[i];
            by_open(x, from, to, |i, pos, _| if pos == 0 { if belt(i) { Some(1) } else { None } } else if !belt(i) { Some(0) } else { None })
        }
        "wmny_lazy_year" => {
            // per year: (first index, last index, high, low, index of first high, index of first low, complete?)
            let mut ys: Vec<(i64, usize, usize, f64, f64, usize, usize)> = vec![];
            for i in 0..n {
                let y = year(&x.r[i].date);
                match ys.last_mut() {
                    Some(e) if e.0 == y => { e.2 = i; if h[i] > e.3 { e.3 = h[i]; e.5 = i; } if l[i] < e.4 { e.4 = l[i]; e.6 = i; } }
                    _ => ys.push((y, i, i, h[i], l[i], i, i)),
                }
            }
            let full = |e: &(i64, usize, usize, f64, f64, usize, usize)| { let d = &x.r[e.1].date; &d[5..7] == "01" && d[8..10].parse::<u32>().unwrap_or(99) <= 7 && e.2 + 1 < n };
            let (mut out, mut cur) = (vec![], None::<Trade>);
            for k in 1..ys.len() {
                let (pv, cy) = (ys[k - 1], ys[k]);
                let i = cy.2;
                if i < from || i > to || !full(&pv) || !full(&cy) { continue; }
                let (hh_, hl, lh, ll_) = (cy.3 > pv.3, cy.4 > pv.4, cy.3 < pv.3, cy.4 < pv.4);
                let want = if hh_ && hl { Some(true) } else if lh && ll_ { Some(false) } else if hh_ && ll_ && cy.6 < cy.5 { Some(true) } else { None };
                match (want, cur.is_some()) {
                    (Some(true), false) => cur = Some(new_trade(1., i, c[i], "close")),
                    (Some(false), true) => out.push(done(cur.take().unwrap(), i, c[i], i, "lower high and lower low: liquidate")),
                    _ => {}
                }
            }
            finish(&mut out, cur, x, to);
            out
        }
        "wmny_engulf" => {
            let bull = |i: usize| i > 0 && c[i - 1] < o[i - 1] && c[i] > o[i] && o[i] < c[i - 1] && c[i] > o[i - 1];
            let bear = |i: usize| i > 0 && c[i - 1] > o[i - 1] && c[i] < o[i] && o[i] > c[i - 1] && c[i] < o[i - 1];
            by_open(x, from, to, |i, pos, _| if pos == 0 && bull(i) { Some(1) } else if pos == 1 && bear(i) { Some(0) } else { None })
        }
        "wmny_zscore" => {
            let nn = u("n");
            let (m, sd) = (sma(c, nn), stdev(c, nn));
            let z: Vec<f64> = (0..n).map(|i| (c[i] - m[i]) / sd[i]).collect();
            let a = sma(&z, u("a"));
            let b = sma(&a, u("b"));
            let sh = g("short") > 0.5;
            by_close(x, from, to, |i, pos| if up(&a, &b, i) { Some(1) } else if up(&b, &a, i) { Some(if sh { -1 } else { 0 }) } else { let _ = pos; None })
        }
        "wmny_candle_short" => {
            let sl = lr_slope(c, u("opt1"));
            let (o2, o3, o4) = (g("opt2"), g("opt3"), u("opt4"));
            let sig = |i: usize| i > 0 && sl[i] < o2 && (c[i] - o[i]).abs() > (c[i - 1] - o[i - 1]).abs() && (c[i - 1] - o[i - 1]).abs() > o3 * (h[i - 1] - l[i - 1]) && c[i] - o[i] > 0. && c[i - 1] - o[i - 1] < 0.;
            let (mut out, mut i) = (vec![], from);
            while i < to {
                if !sig(i) { i += 1; continue; }
                let t = new_trade(-1., i + 1, o[i + 1], "open");
                let xi = i + o4;
                if xi > to { finish(&mut out, Some(t), x, to); break; }
                out.push(done(t, xi, c[xi], xi, "exit two bars after the signal, same-day close"));
                i = xi; // entries are ignored while the trade is open; a signal on the exit bar can enter next open
            }
            out
        }
        "wmny_trend_rsi21" | "wmny_trend_srsi" => {
            let r = if id == "wmny_trend_rsi21" { rsi(c, u("n")) } else { srsi3(c, u("n")) };
            let (lo, hi) = (vec![g("lo"); n], vec![g("hi"); n]);
            by_open(x, from, to, |i, _, _| if up(&r, &lo, i) { Some(1) } else if up(&hi, &r, i) { Some(-1) } else { None })
        }
        "wmny_trend_rsi_ma" | "wmny_trend_srsi_ma" => {
            let f = |k: usize| if id == "wmny_trend_rsi_ma" { rsi(c, k) } else { srsi3(c, k) };
            let a = f(u("o1"));
            let b = avg(&f(u("o2")), u("o3"));
            by_open(x, from, to, |i, _, _| if up(&a, &b, i) { Some(1) } else if up(&b, &a, i) { Some(-1) } else { None })
        }
        "wmny_willr" => {
            let (w1, w3, w5, w7) = (willr(x, u("opt1")), willr(x, u("opt3")), willr(x, u("opt5")), willr(x, u("opt7")));
            let (o2, o4, o6, o8) = (g("opt2"), g("opt4"), g("opt6"), g("opt8"));
            let en = alert(&w1.iter().map(|&v| v < o2).collect::<Vec<_>>(), u("opt9"));
            let exa = alert(&w5.iter().map(|&v| v > o6).collect::<Vec<_>>(), u("opt10"));
            let entry = |i: usize| en[i] && w3[i] > o4;
            let exit = |i: usize| exa[i] && w7[i] < o8;
            if g("exit_next_open") > 0.5 {
                by_open(x, from, to, |i, pos, _| if pos == 0 && entry(i) { Some(1) } else if pos == 1 && exit(i) { Some(0) } else { None })
            } else {
                open_in_close_out(x, from, to, |i| if entry(i) { 1 } else { 0 }, |i, _| exit(i))
            }
        }
        "wmny_pullback" => {
            let sn = u("slope_n");
            let sl = |m: Vec<f64>| if g("slope_diff") > 0.5 { (0..n).map(|i| if i >= sn { m[i] - m[i - sn] } else { f64::NAN }).collect::<Vec<f64>>() } else { lr_slope(&m, sn) };
            let (ma, s28, s50, s20) = (sma(c, u("ma")), sl(sma(c, 28)), sl(sma(c, 50)), sl(sma(c, 20)));
            let v: Vec<f64> = x.r.iter().map(|r| r.volume.unwrap_or(f64::NAN)).collect();
            let va = sma(&v, u("vol_n"));
            let (lh, hold, minp, minv) = (u("lh"), u("hold"), g("price"), g("vol"));
            let setup = |y: usize| y >= lh && c[y] > ma[y] && s28[y] > 0. && s50[y] > 0. && s20[y] > 0. && c[y] > minp && va[y] > minv && (0..lh).all(|k| h[y - k] < h[y - k - 1]);
            let (mut out, mut t) = (vec![], from.max(1));
            while t < to {
                if !(setup(t - 1) && h[t] > h[t - 1]) { t += 1; continue; }
                let e = t + 1;
                let tr = new_trade(1., e, o[e], "open");
                let xi = e + hold;
                if xi > to { finish(&mut out, Some(tr), x, to); break; }
                out.push(done(tr, xi, o[xi], xi - 1, "held three days: exit at the next open"));
                t = xi; // flat again from the exit open
            }
            out
        }
        "wmny_dji_5_21m" => {
            let ends: Vec<usize> = (0..n).filter(|&i| i + 1 < n && month(&x.r[i + 1].date) != month(&x.r[i].date)).collect();
            let mc: Vec<f64> = ends.iter().map(|&i| c[i]).collect();
            let (f, s) = (sma(&mc, u("fast")), sma(&mc, u("slow")));
            let (mut out, mut cur) = (vec![], None::<Trade>);
            for k in 1..ends.len() {
                let i = ends[k];
                if i < from || i > to { continue; }
                if cur.is_none() && up(&f, &s, k) { cur = Some(new_trade(1., i, c[i], "close")); }
                else if cur.is_some() && up(&s, &f, k) { out.push(done(cur.take().unwrap(), i, c[i], i, "5-month SMA crossed below 21-month SMA")); }
            }
            finish(&mut out, cur, x, to);
            out
        }
        "wmny_nhnl_ratio" | "wmny_nhnl_cci" => {
            let (Some(nh), Some(nl)) = (x.series("NASDAQ_NEW_HIGHS"), x.series("NASDAQ_NEW_LOWS")) else { return Some(vec![]) };
            let ratio: Vec<f64> = (0..n).map(|i| nh[i] / (nh[i] + nl[i])).collect();
            let a = id == "wmny_nhnl_ratio";
            let (e1, e2, e3) = if a { (u("opt1"), u("opt2"), u("opt3")) } else { (7, 16, 2) };
            let (f1, f2) = (xavg(&ratio, e1), xavg(&ratio, e2));
            let diff: Vec<f64> = (0..n).map(|i| f1[i] - f2[i]).collect();
            let sig = xavg(&diff, e3);
            if a {
                let (o4, o5, o6, o7) = (g("opt4"), g("opt5"), g("opt6"), g("opt7"));
                open_in_close_out(x, from, to, |i| if ratio[i] > o4 && sig[i] > diff[i] { 1 } else if ratio[i] < o5 && sig[i] < diff[i] { -1 } else { 0 },
                    |i, d| if d > 0 { ratio[i] < o6 && sig[i] < diff[i] } else { ratio[i] > o7 && sig[i] < diff[i] })
            } else {
                let (cl, cs) = (cci_of(h, l, c, u("opt1")), cci_of(h, l, c, u("opt2")));
                let al = alert(&cl.iter().map(|&v| v < g("opt5")).collect::<Vec<_>>(), u("opt4"));
                let as_ = alert(&cs.iter().map(|&v| v > g("opt7")).collect::<Vec<_>>(), u("opt6"));
                let (o8, o9) = (g("opt8"), g("opt9"));
                open_in_close_out(x, from, to, |i| if (ratio[i] > 0.4 && al[i]) || (sig[i] > diff[i] && al[i]) { 1 } else if as_[i] { -1 } else { 0 },
                    |i, d| if d > 0 { ratio[i] < 0.6 && sig[i] < diff[i] && cl[i] > o8 } else { ratio[i] > 0.6 && sig[i] > diff[i] && cs[i] < o9 })
            }
        }
        "wmny_pct_nh" => {
            let (Some(nh), Some(nl)) = (x.series("NYSE_NEW_HIGHS"), x.series("NYSE_NEW_LOWS")) else { return Some(vec![]) };
            let r: Vec<f64> = (0..n).map(|i| 100. * nh[i] / (nh[i] + nl[i])).collect();
            let m = sma(&r, u("n"));
            let (lo, hi) = (vec![g("lo"); n], vec![g("hi"); n]);
            by_close(x, from, to, |i, pos| if pos == 0 && up(&m, &lo, i) { Some(1) } else if pos == 1 && up(&hi, &m, i) { Some(0) } else { None })
        }
        "wmny_bond_band" => {
            let (Some(bh), Some(bl), Some(bc)) = (x.series("TBOND_FUT_HIGH"), x.series("TBOND_FUT_LOW"), x.series("TBOND_FUT_CLOSE")) else { return Some(vec![]) };
            // weekly bond bars (Monday-Sunday), last daily index of each week
            let mut wk: Vec<(usize, f64, f64, f64)> = vec![];
            let mut key = i64::MIN;
            for i in 0..n {
                let dn = day_num(&x.r[i].date);
                let k = dn - (crate::books::atm03::weekday(dn) + 6) % 7;
                if bc[i].is_nan() { continue; }
                if k != key { wk.push((i, bh[i], bl[i], bc[i])); key = k; } else { let w = wk.last_mut().unwrap(); w.0 = i; w.1 = w.1.max(bh[i]); w.2 = w.2.min(bl[i]); w.3 = bc[i]; }
            }
            let rows: Vec<crate::Row> = wk.iter().map(|w| crate::Row { ticker: String::new(), date: String::new(), open: w.3, high: w.1, low: w.2, close: w.3, volume: None }).collect();
            let k = u("k");
            let wa = atr(&rows, k);
            let wc: Vec<f64> = wk.iter().map(|w| w.3).collect();
            let mid: Vec<f64> = (0..wk.len()).map(|j| if j == 0 { f64::NAN } else { (ll(&wc, j - 1, k) + wa[j - 1] * g("z") + hh(&wc, j - 1, k) - wa[j - 1] * g("z")) / 2. }).collect();
            let band = sma(&mid, u("x"));
            let (mut out, mut cur) = (vec![], None::<Trade>);
            for j in 1..wk.len() {
                let i = wk[j].0;
                if i < from || i > to || band[j].is_nan() { continue; }
                if cur.is_none() && wc[j] > band[j] { cur = Some(new_trade(1., i, c[i], "close")); }
                else if cur.is_some() && wc[j] < band[j] { out.push(done(cur.take().unwrap(), i, c[i], i, "bonds below the band average")); }
            }
            finish(&mut out, cur, x, to);
            out
        }
        "wmny_not_testable" => vec![],
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn ctx(b: &[(f64, f64, f64, f64)]) -> Vec<Row> {
        b.iter().enumerate().map(|(i, &(o, h, l, c))| Row { ticker: "T".into(), date: format!("2001-01-{:02}", i + 1), open: o, high: h, low: l, close: c, volume: None }).collect()
    }
    #[test]
    fn belt_hold_buys_next_open_and_sells_after_non_belt_day() {
        // day1 belt hold, day2 belt hold (keeps), day3 not -> sell at day4 open
        let r = ctx(&[(10., 10., 10., 10.), (10., 11., 10., 10.8), (11., 12., 11., 11.5), (11.5, 12., 11.2, 11.6), (11.7, 12., 11.5, 11.8), (11.8, 12., 11.6, 11.7)]);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("wmny_belt_hold", &x, 0, 5, &Params::new()).unwrap();
        assert_eq!((t[0].legs[0].i, t[0].legs[0].px, t[0].exit_i, t[0].exit_px), (2, 11., 4, 11.7));
    }
    #[test]
    fn candle_short_enters_next_open_exits_close_two_bars_later() {
        let mut b: Vec<(f64, f64, f64, f64)> = (0..10).map(|k| { let p = 100. - k as f64; (p + 0.2, p + 0.5, p - 0.5, p) }).collect();
        b.push((89.5, 89.6, 88.0, 88.2)); // black, body 1.3 of range 1.6
        b.push((88.0, 90.5, 87.9, 90.0)); // white, body 2.0 > 1.3 -> signal at 11
        b.push((90.0, 91., 89., 90.5));
        b.push((90.5, 91., 89., 89.0));
        let r = ctx(&b);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("wmny_candle_short", &x, 0, b.len() - 1, &Params::new()).unwrap();
        assert_eq!(t.len(), 1);
        assert_eq!((t[0].dir, t[0].legs[0].i, t[0].legs[0].px, t[0].exit_i, t[0].exit_px), (-1., 12, 90., 13, 89.));
    }
}
