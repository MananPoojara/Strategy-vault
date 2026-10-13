//! Daryl Guppy's weekly newsletters 2000-2004 (guppytraders.com: "Tutorials In Technical Analysis" / "The Australian
//! Internet Trading Weekly"; 2000 complete, 2001 part, 2003, 2004 to June), research drive Trading Knowledge II/
//! @NewsLetters-Mags/Daryl Guppy Tutorials In Technical Analysis (2000-2001-2003-2004)/; copies in
//! Books/Daryl Guppy/Newsletters 2000-2004/. Book id "dgnl". Notes: library/dgnl/README.md.
//! Every complete mechanical system in the 131 issues is an Australian SPI-futures or NAB-share system whose stops are
//! in contract dollars or NAB cents, so nothing is Live; the catalogue records them (NeedsData) and the chart-reading
//! methods (NotTestable). No run code: there is no SPI / ASX data to run it on.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade, claim};

const SPI: &str = "SPI 200 share price index futures (SFE) daily data; the stops are in contract dollars at $25 per point";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, dir, status, rules, notes, claim| StrategyDef { id, name, group, session: 0, dir, status, rules, notes,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "dgnl", claim };
    vec![
        d("dgnl_spi_dow_bonds", "SPI day-of-week open-to-close with a bond 5-day-high filter (A Gibbs)", "Gibbs · SPI systems", "Long",
          Status::NeedsData("SPI 200 futures and SFE 3-year / 10-year bond futures daily data"),
          &["E|\"buying on the open of the day and selling on the close of the same day followed by the following day etc. The results are shown for holding a position from 1-4 days\" (18/08/2000 p25, 'Disproving Random Walk In The SPI')",
            "F|\"in order to take a trade on a specific day of the week 3-year bonds must have closed at a 5-day high\" (p25-26); 10-year bond variant on p26",
            "X|Exit on the close of the chosen exit day (table p25: enter Mon..Fri, exit Mon..Fri)"],
          "The unfiltered day-of-week grid (p25) is the open-to-close day-of-week study already in the vault (rgc_dow_oc, pbwts_dow_oc, lwlts_mon_3day); only the bond-filtered version is new and it needs Australian bond futures.",
          claim("", "Wed same day 55%, next day 56% (unfiltered); Wed 61% with the 3-yr bond filter", "", "same day to 4 days", "SPI futures March 1994 - 10/8/2000, no commission (p25-26)")),
        d("dgnl_spi_breakout", "SPI breakout: open + (low[5] - high[1]), Monday/Friday (A Gibbs)", "Gibbs · SPI systems", "Long", Status::NeedsData(SPI),
          &["E|\"If: low[5]-high[1]>0 and DayOfWeek(date)=5 or DayOfWeek(date)=1 / Then buy next bar on stop at: (open next bar + (low[5]-high[1]))\" (25/08/2000 p8-9, 'SPI Breakouts'; the prose says '(the low 5 days ago - low one day ago)', the code is used)",
            "X|\"Exit on the first profitable open. Exit on stop loss set at $-1500.00. Exit on a signal in the opposite direction.\" (p8); \"If: MarketPosition=1 and open next bar > entryprice / Then exit next bar at market / Money Management Stop = $1500\" (p9)",
            "F|\"The general rule however is only take signals from the above system when bonds make a five day high\" (p9)"],
          "SPI 1989-2000, $40 commission, one contract (p9). The $1500 stop is 60 SPI points; it has no index equivalent.", None),
        d("dgnl_spi_12_10", "SPI 12-day high / 10-day low breakout (A Gibbs)", "Gibbs · SPI systems", "Long/Short", Status::NeedsData(SPI),
          &["E|\"buy the next day on open if the SPI makes a 12-day high and to sell the next day on open if a 10-day low in the SPI is made\" (01/09/2000 p5-6, 'Testing SPI Breakouts')",
            "X|\"A stop loss of $1500 will be used and commission costs will be set to $40\" (p5)",
            "F|Variants: enter on any day except Tuesday (p6); buy/sell on 10-year bond 12-day high / 10-day low instead of the SPI's (p7, needs SFE 10-year bond futures)"],
          "SPI since 1983 (p5). 12/10 are stated as the optimised inputs. Results only in images.", None),
        d("dgnl_spi_dom", "SPI day-of-month seasonal trades (A Gibbs)", "Gibbs · SPI systems", "Long/Short", Status::NeedsData(SPI),
          &["E|Long \"Buy on Open\" when ((Month=3 and DayOfMonth=1) or DayOfMonth=31) or (Month=4 and DayOfMonth=30) or ((Month=6 and DayOfMonth=1) or DayOfMonth=30) or (Month=8 and DayOfMonth=1 or DayOfMonth=31) or (Month=9 and DayOfMonth=30) or (Month=10 and DayOfMonth=30) or (Month=11 and DayOfMonth=30); Short when \"DayOfMonth(date)=8 and Month(date)=5 or Month(date)=6 or Month(date)=8 or Month(date)=9 or Month(date)=10 or Month(date)=12\" (08/09/2000 p22 'Rules for Day of the Month SPI Daily')",
            "X|\"Exit on First Profitable open: MarketPosition=1 and open next>entryprice / Exit next bar at market\" (short: mirror); \"uses a money management stop of $1500\" (p21)"],
          "1983-2000: 185 trades, 166 winners, $63,217 one contract (p20-21). The and/or grouping of the printed formula is ambiguous (SuperCharts precedence).",
          claim("185", "166 winners (89.7%)", "$63,217 net, one contract", "", "SPI 1983 - Sept 2000 (p20-21)")),
        d("dgnl_spi_seasonal_q1", "SPI seasonal dates January-March (A Gibbs)", "Gibbs · SPI systems", "Long", Status::NeedsData(SPI),
          &["E|\"If: (DayofMonth(date)=1 and month(date)=2 or month(date)=3) or (DayOfMonth(date)=9 and month(date)=1 or month(date)=3) or (dayofmonth(date)=13 and month(date)=1 or month(date)=3) or (dayofmonth(date)=22 and month(date=3) or (dayofmonth(date)=24 and month(date)=1 or month(date)=2) or (dayofmonth(date)=25 and month(date)=1) or (dayofmonth(date)=30 and month(date)=3) or (dayofmonth(date)=31 and month(date)=3) then: Buy on Stop: open next bar\" (29/09/2000 p13, 'Seasonality And The SPI')",
            "X|\"MarketPosition=1 and open next bar > entryprice then: Sell next bar at market / Stop=$1500\" (p14); dates within 2 days of a previous signal are dropped (25 Jan, 30 Mar)"],
          "The dates are fitted on the SPI's own history; Sept/Oct 2000 dates are listed on 13/10/2000 p19-20 (Sept: 7 of 7 winners, $4,125).", None),
        d("dgnl_spi_vb70_month", "SPI volatility breakout open + 70% of yesterday's range, May/Jun/Jul/Dec (A Gibbs)", "Gibbs · SPI systems", "Long", Status::NeedsData(SPI),
          &["E|\"(market open + (yesterdays high-yesterdays low)*70%)\" buy stop; \"If: Month(date)=1 (January) then buy on open next bar + ((h-l)*.7)\" (20/10/2000 p16-17, 'Using Seasonality To Filter A Trading System')",
            "F|\"the best months to trade this system are May, June, July and December these are the only month in which we will trade this system\" (p17)",
            "X|\"The exit used for this system is simply first profitable open ... a stop loss of $400 will be used\" (p16)"],
          "16 years of SPI, $45 brokerage. Unfiltered: average $75/trade, risk/return 1.49, 770% ROI; month-filtered: >$100/trade, 1.8, 670% (p17).", None),
        d("dgnl_spi_oops", "Larry Williams Oops on the SPI, 16-point trailing stop (A Gibbs)", "Gibbs · SPI systems", "Long/Short",
          Status::NeedsData("SPI 200 futures intraday bars (the 16-point trailing stop needs the intraday path) and SPI points"),
          &["E|\"If open next bar >high then sell next bar at high on stop; If open next bar <low then buy next bar at low on stop;\" (02/04/2001, 'Larry Williams Oops')",
            "X|\"an exit on the close of the day in conjunction with a 16 point trailing stop\" (same article)",
            "F|Long-only variants (09/04/2001): only when \"the 4 day RSI of the close is below 30\" (or below 70); exit \"on the first profitable open and the stop loss is extended by $100 dollars to 500 dollars ... a flat money management stop\""],
          "SPI 1993-2000, $25 brokerage: 57% winners, average trade $82 (02/04/2001); RSI<30 + first profitable open: 73% (09/04/2001). The Oops pattern itself is in the vault (lwlts_oops_*, ss_study_oops_*).",
          claim("", "57%", "$82", "intraday", "SPI 1993-2000 (02/04/2001)")),
        d("dgnl_ag_nab", "AG_NAB short-term CFD system on National Australia Bank (A Gibbs)", "Gibbs · CFD system", "Long/Short",
          Status::NeedsData("National Australia Bank (ASX: NAB) daily data; the exits are fixed NAB cents (30c trailing offset, 90c entry-day stop)"),
          &["F|\"Median=(High-Low)/2+L; If close > Median then Buy Open next Bar+Average(high-open,5)*GSV+0.02 stop; If c<Median then Sell Open Next Bar-Average(open-low,5)*GSV-0.02 stop;\" (28/04/2003 p7, Inputs GSV(2.0), Cents(0.3); prose 'twice the USV'); 05/05/2003 p8 repeats the code with GSV(1.0)",
            "X|\"If marketposition=1 Exitlong at High-Cents stop; If marketposition=-1 ExitShort at Low+Cents stop;\" (05/05/2003 p8) \"once we have held the position overnight\"",
            "X|\"If marketposition =1 and barssinceentry>0 and close<entryprice then exitlong at close\" (short: mirror) and \"a stop loss on the day of entry ... very wide for NAB at 90 cents\" (05/05/2003 p8-9)"],
          "NAB 1996 - May 2003, one share: net $39.87, 50% profitable, average win/loss 2.77, average trade 16 cents, largest loss 90c (05/05/2003 p9, 16/06/2003 p10). GSV is 2.0 in the first listing and prose, 1.0 in the second listing.",
          claim("", "50%", "16 cents (one share)", "overnight +", "NAB 1996 - Mar 2003 (05/05/2003 p9)")),
        d("dgnl_gibbs_indicator_system", "RSI(14) / ROC(7) / volume / trend-line / 14-day MA system (A Gibbs)", "Gibbs · system design", "Long/Short",
          Status::NotTestable("every signal 'must be confirmed by a break in your trend line', and the RSI/ROC parts are divergence readings"),
          &["E|\"A buy signal is issued when the RSI rises above 30 and a sell signal is issued when the RSI falls below 70 ... All signals from the RSI must be confirmed by a break in your trend line\" (12/05/2000 p8)",
            "E|\"a buy signal is issued when price closes above the 14-day moving average and a sell signal is issued when the price closes below the 14-day single moving average\" (19/05/2000 p10)",
            "F|9-day stochastic crossovers above 20 / below 80 \"Confirm all signals with a break in the trend-line\" (26/05/2000 p12, 01/06/2000 p12)"],
          "Series 'Designing / Developing / Back Testing / Computer Testing a Technical Trading System', 05/05-01/06/2000.", None),
        d("dgnl_mma180", "Guppy MMA cluster above a 180-day average (A Gibbs)", "Gibbs · stock methods", "Long", Status::NotTestable("the setup is a visual cluster of the GMMA ('a visual representation'), the entry is the 'most recent swing high', and the exits were never published"),
          &["F|\"price must close above the 180 day moving average of the high. The Short-term moving averages should be clustering around the long-term moving averages ... This is a visual representation\" (17/11/2000 p14-15)",
            "E|\"A buy on stop order is then placed at the most recent swing high\" (p15)"],
          "17/11/2000; the promised exit articles do not appear in the issues held.", None),
        d("dgnl_rel_strength", "Relative market strength 2/21/90-day averages (A Gibbs)", "Gibbs · stock methods", "Long", Status::NotTestable("the entry is a break of a chart-pattern neckline confirmed by the sector index's swing high, both chart readings"),
          &["F|\"The up trend is defined as the 2 day average being higher than the 21 day average and the closing price being higher than the 90 day moving average. The chart must also show a higher low and a higher high in both the stock and underlying index\" (07/05/2001)",
            "X|\"the exit point is when the 2 day moving average crosses below the 21-day moving average\"; stop at the swing low"],
          "07/05/2001 'Using Relative Market Strength' (Macquarie Bank vs the banking index).", None),
        d("dgnl_exhaustion_spike", "Exhaustion spike (Yoni Meinrath)", "Contributors", "Long", Status::NotTestable("the trade also requires 'Confirm that the exhaustion spike has tested a (major) support level' (chart reading) and intraday volume, and the stop is 'below support'"),
          &["F|YONI'S EXHAUSTION SPIKE FORMULA (MetaStock, 3- and 4-day versions: closes in the lower half, opens near prior lows, H < Ref(L,-2) ..., O < L + (H-L)/2) (04/12/2000)",
            "E|\"Use YONI'S EXHAUSTION SPIKE FORMULA ... Confirm that the exhaustion spike has tested a (major) support level. Use intraday data to confirm rally with volume to close. Open trade at market open on the next trading day.\"",
            "X|\"Use tight stop-loss below support level and a 3 - 5 day inactivity stop.\""],
          "04/12/2000 'Finding And Trading Exhaustion Spikes'. The inactivity stop is a 3-5 day range, and the support level is judged by eye.", None),
        d("dgnl_weekly_trend", "Trading the weekly trend with 5/30-week SMAs (C Miller)", "Contributors", "Long/Short", Status::NotTestable("a weekly filter only: 'Our exit is still based on our daily technique', which is never specified"),
          &["F|\"for trading long we like a rising 30 week MA, and we look for entries when the weekly price closes above the 5 MA and the 5 MA turns up. When the weekly close is below the 5 MA, if we are still in a trade we bring our stops close and may look to exit on strength\" (11/12/2000; series 17/11-18/12/2000)"],
          "Adapted from Dawn Bolton-Smith (SFE 'Your Trading Edge'); the entry and exit are left to 'whatever our daily mechanism is'.", None),
        d("dgnl_guppy_methods", "Guppy's own newsletter methods (GMMA, count back line, Darvas, chart patterns, case-study portfolios)", "Guppy", "Long", Status::NotTestable("discretionary chart reading (trend lines, pattern recognition, GMMA 'compression' and 'separation' judged by eye); no complete objective rule"),
          &["F|Case Study Portfolio, Six to Twenty One portfolio, Sample Portfolio, Swiss Roll portfolio, chart briefs, flags/triangles/head-and-shoulders/stirrups, count back line stops, GMMA trend judgement (all issues 2000-2004)"],
          "The mechanical Guppy pieces already in the vault are dgmf (MetaStock formulas), dggw, dgstt, dgppt, dgmtt, dgdwb. 'Shares magazine' basket (17/02/2003 p10, 24/02/2003) = buying a magazine's stock picks: no rule. Hewitt's 'Trading Research' (13/03-17/04/2004) = hindsight study of ASX 100%+ movers.", None),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "dgnl_spi_dow_bonds" | "dgnl_spi_breakout" | "dgnl_spi_12_10" | "dgnl_spi_dom" | "dgnl_spi_seasonal_q1" | "dgnl_spi_vb70_month" | "dgnl_spi_oops" =>
            ("dgnl_spi", "Gibbs SPI futures systems (Guppy newsletters)", match id {
                "dgnl_spi_dow_bonds" => "Day of week + bonds", "dgnl_spi_breakout" => "low[5]-high[1] breakout", "dgnl_spi_12_10" => "12/10-day breakout",
                "dgnl_spi_dom" => "Day of month", "dgnl_spi_seasonal_q1" => "Seasonal dates Q1", "dgnl_spi_vb70_month" => "70% volatility breakout by month", _ => "Oops" }),
        "dgnl_ag_nab" => ("dgnl_ag_nab", "AG_NAB CFD system (Gibbs)", "NAB"),
        "dgnl_gibbs_indicator_system" | "dgnl_mma180" | "dgnl_rel_strength" | "dgnl_exhaustion_spike" | "dgnl_weekly_trend" | "dgnl_guppy_methods" => ("dgnl_methods", "Guppy newsletter methods (not testable)", match id {
            "dgnl_gibbs_indicator_system" => "RSI/ROC/trend-line system", "dgnl_mma180" => "MMA + 180-day MA", "dgnl_rel_strength" => "Relative strength",
            "dgnl_exhaustion_spike" => "Exhaustion spike", "dgnl_weekly_trend" => "Weekly 5/30 SMA trend", _ => "Guppy methods" }),
        _ => return None,
    })
}

pub fn params(_id: &str) -> Option<Vec<ParamDef>> { None }
pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }
