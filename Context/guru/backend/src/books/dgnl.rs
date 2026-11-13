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
          Status::Live,
          &["E|\"buying on the open of the day and selling on the close of the same day followed by the following day etc. The results are shown for holding a position from 1-4 days\" (18/08/2000 p25, 'Disproving Random Walk In The SPI')",
            "F|\"in order to take a trade on a specific day of the week 3-year bonds must have closed at a 5-day high\" (p25-26); 10-year bond variant on p26",
            "X|Exit on the close of the chosen exit day (table p25: enter Mon..Fri, exit Mon..Fri)"],
          "[Adapted: 1 reading, R4 #297; lane PF, 14 Nov 2026; review PF2 B4] Every cell of the article's printed grid is a complete rule, so the card codes the study with options: p.day = entry weekday (1 Monday .. 5 Friday; default 3 = Wednesday, 'by far the best day'), p.hold = extra days held (0 = out at the same day's close .. 3), p.bond = filter (0 none = the first table; 3 = 3-year bonds 'must have closed at a 5-day high' = the second table, Norgate &YYT; 10 = 10-year bonds, &YXT). Long at the entry day's open, out at the close; no stop. Reading: '5-day high' = the bond's close on the previous trading day (the last close known at the open) above the highest close of its 4 bars before, on the bond's own bars, joined by exact date (no bond close = no trade). Runs only on Norgate continuous SPI (&YAP, back-adjusted), published 1983-02-16..1991-12-31 (R4 #333; the article's tables are 1994-2000, after the cap, so unverified).",
          claim("", "Wed same day 55%, next day 56% (unfiltered); Wed 61% with the 3-yr bond filter", "", "same day to 4 days", "SPI futures March 1994 - 10/8/2000, no commission (p25-26)")),
        d("dgnl_spi_breakout", "SPI breakout: open + (low[5] - high[1]), Monday/Friday (A Gibbs)", "Gibbs · SPI systems", "Long", Status::Live,
          &["E|\"If: low[5]-high[1]>0 and DayOfWeek(date)=5 or DayOfWeek(date)=1 / Then buy next bar on stop at: (open next bar + (low[5]-high[1]))\" (25/08/2000 p8-9, 'SPI Breakouts'; the prose says '(the low 5 days ago - low one day ago)', the code is used)",
            "X|\"Exit on the first profitable open. Exit on stop loss set at $-1500.00. Exit on a signal in the opposite direction.\" (p8); \"If: MarketPosition=1 and open next bar > entryprice / Then exit next bar at market / Money Management Stop = $1500\" (p9)",
            "F|\"The general rule however is only take signals from the above system when bonds make a five day high\" (p9)"],
          "[Adapted: 1 reading, R4 #297; lane PF, 14 Nov 2026] SPI 1989-2000, $40 commission, one contract (p9); results printed as images (unverified). The $1,500 stop is 60 SPI points ('The tick value of the SPI is set at $25 per point', p.9). Reading: the printed 'low[5]-high[1]>0 and DayOfWeek(date)=5 or DayOfWeek(date)=1' requires the setup on both days (signals on Friday and Monday bars, trades on Monday and Tuesday: 'd1=5 ... will give Mondays performance'). The bond filter ('only take signals ... when bonds make a five day high') is not in the default: the article's tested system could not include it; p.bond = 3 / 10 applies it as a labelled option (3-year / 10-year SFE bonds, Norgate &YYT / &YXT: the bond's close on the signal bar above its 4 closes before; review PF2 M29). A buy stop at the next open + (low[5] - high[1]); exit at the first later open above the entry; 'exit on a signal in the opposite direction' has no coded short side. Runs only on Norgate continuous SPI (&YAP, back-adjusted), published 1983-02-16..1991-12-31 (R4 #333); 0 of 64 trades there fill the entry stop and the money stop on one bar (#240).", None),
        d("dgnl_spi_12_10", "SPI 12-day high / 10-day low breakout (A Gibbs)", "Gibbs · SPI systems", "Long/Short", Status::Live,
          &["E|\"buy the next day on open if the SPI makes a 12-day high and to sell the next day on open if a 10-day low in the SPI is made\" (01/09/2000 p5-6, 'Testing SPI Breakouts')",
            "X|\"A stop loss of $1500 will be used and commission costs will be set to $40\" (p5)",
            "F|Variants: enter on any day except Tuesday (p6); buy/sell on 10-year bond 12-day high / 10-day low instead of the SPI's (p7, needs SFE 10-year bond futures)"],
          "[Adapted: 2 unstated details, R4 #297; lane PF, 14 Nov 2026] SPI since 1983 (p5); 12/10 are stated as the optimised inputs; results only in images (unverified). Runs only on Norgate continuous SPI (&YAP, back-adjusted), published 1983-02-16..1991-12-31 (R4 #333: the &YAP open changes in Jan 1992 and every entry is at the open). Unstated in the article: (1) 'makes a 12-day high': today's high is the highest high of the last 12 bars (10-day low: today's low is the lowest of the last 10); (2) 'sell': a reversal to short (the system is in the market both ways). A same-direction signal while in that position is ignored; both on one bar: no change. $1,500 money stop = 60 points from the entry (the article's $25 per point), from the entry day, at the stop or the open if gapped; after a stop the system waits for the next signal. No costs (the article charged $40).", None),
        d("dgnl_spi_dom", "SPI day-of-month seasonal trades (A Gibbs)", "Gibbs · SPI systems", "Long/Short", Status::Live,
          &["E|Long \"Buy on Open\" when ((Month=3 and DayOfMonth=1) or DayOfMonth=31) or (Month=4 and DayOfMonth=30) or ((Month=6 and DayOfMonth=1) or DayOfMonth=30) or (Month=8 and DayOfMonth=1 or DayOfMonth=31) or (Month=9 and DayOfMonth=30) or (Month=10 and DayOfMonth=30) or (Month=11 and DayOfMonth=30); Short when \"DayOfMonth(date)=8 and Month(date)=5 or Month(date)=6 or Month(date)=8 or Month(date)=9 or Month(date)=10 or Month(date)=12\" (08/09/2000 p22 'Rules for Day of the Month SPI Daily')",
            "X|\"Exit on First Profitable open: MarketPosition=1 and open next>entryprice / Exit next bar at market\" (short: mirror); \"uses a money management stop of $1500\" (p21)"],
          "[Adapted: 2 readings labelled, R4 #297; lane PF, 14 Nov 2026] Runs only on the Norgate continuous SPI contract (&YAP, back-adjusted daily bars; ASX SPI 200, the All Ordinaries SPI before 2000; ends 2020-09-11; roll method not stated by the vendor). Date grouping: each date belongs to its stated month (long 1 Mar, 31 Mar, 30 Apr, 1 Jun, 30 Jun, 1 Aug, 31 Aug, 30 Sep, 30 Oct, 30 Nov; short 8 May, 8 Jun, 8 Aug, 8 Sep, 8 Oct, 8 Dec) - the article's reader count (185 trades in 205 months, 'no trades made in January or February') rules out the literal SuperCharts grouping, which would trade every 30th/31st and every June day. Readings: (1) the order placed on the dated bar fills at the NEXT open (a strategy cannot fill at the open of the bar it is evaluated on); (2) the $1,500 money stop = 60 SPI points (the article's own $25 per point, p.8), from the entry, filled at the stop or at the open when gapped. Exit at the first later open beyond the entry (long: above, short: below), at that open; an opposite dated signal reverses. No costs (the reader's test used none stated).",
          claim("185", "166 winners (89.7%)", "$63,217 net, one contract", "", "SPI 1983 - Sept 2000 (p20-21)")),
        d("dgnl_spi_seasonal_q1", "SPI seasonal dates January-March (A Gibbs)", "Gibbs · SPI systems", "Long", Status::Live,
          &["E|\"If: (DayofMonth(date)=1 and month(date)=2 or month(date)=3) or (DayOfMonth(date)=9 and month(date)=1 or month(date)=3) or (dayofmonth(date)=13 and month(date)=1 or month(date)=3) or (dayofmonth(date)=22 and month(date=3) or (dayofmonth(date)=24 and month(date)=1 or month(date)=2) or (dayofmonth(date)=25 and month(date)=1) or (dayofmonth(date)=30 and month(date)=3) or (dayofmonth(date)=31 and month(date)=3) then: Buy on Stop: open next bar\" (29/09/2000 p13, 'Seasonality And The SPI')",
            "X|\"MarketPosition=1 and open next bar > entryprice then: Sell next bar at market / Stop=$1500\" (p14); dates within 2 days of a previous signal are dropped (25 Jan, 30 Mar)"],
          "[Literal rules; lane PF, 14 Nov 2026] The dates are fitted on the SPI's own history; Sept/Oct 2000 dates are listed on 13/10/2000 p19-20 (not part of this card). Dates = the 'y' cells of the article's own table (p.13-14), less the two it drops ('within 2 days of a previous signal ... exclude the 25th Jan and the 30th March'): 9, 13, 24 Jan; 1, 24 Feb; 1, 9, 13, 22, 31 Mar (the printed and/or formula is read through that table). The dated bar's order 'Buy on Stop: open next bar' fills at the next open. The $1,500 stop = 60 SPI points: this article sets 'the Universe ... of the price data to 25 value per point' (p.13; review PF2 M28: 0 unstated details). Exit at the first later open above the entry. Runs only on Norgate continuous SPI (&YAP, back-adjusted), published 1983-02-16..1991-12-31 (R4 #333). The report is an image: unverified.", None),
        d("dgnl_spi_vb70_month", "SPI volatility breakout open + 70% of yesterday's range, May/Jun/Jul/Dec (A Gibbs)", "Gibbs · SPI systems", "Long", Status::NotTestable("held under R4 #240 (lane PF, 14 Nov 2026): on Norgate continuous SPI (&YAP) over the publishable 1983-02-16..1991-12-31 window (R4 #333), 16 of its 211 trades (7.6%) fill the open + 70% buy stop and reach the $400 (16-point) money stop inside the same daily bar with the open through neither, which daily bars cannot order, and no SPI minute bars exist (limit 5%)"),
          &["E|\"(market open + (yesterdays high-yesterdays low)*70%)\" buy stop; \"If: Month(date)=1 (January) then buy on open next bar + ((h-l)*.7)\" (20/10/2000 p16-17, 'Using Seasonality To Filter A Trading System')",
            "F|\"the best months to trade this system are May, June, July and December these are the only month in which we will trade this system\" (p17)",
            "X|\"The exit used for this system is simply first profitable open ... a stop loss of $400 will be used\" (p16)"],
          "16 years of SPI, $45 brokerage. Unfiltered: average $75/trade, risk/return 1.49, 770% ROI; month-filtered: >$100/trade, 1.8, 670% (p17).", None),
        d("dgnl_spi_oops", "Larry Williams Oops on the SPI, 16-point trailing stop (A Gibbs)", "Gibbs · SPI systems", "Long/Short",
          Status::NeedsData("SPI 200 futures intraday bars: the daily SPI is now held (Norgate &YAP), but the 16-point trailing stop next to an exit at the close needs the intraday path, and no SPI minute bars exist (R4 #240) (lane PF, 14 Nov 2026)"),
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

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    match id {
        "dgnl_spi_dom" | "dgnl_spi_seasonal_q1" => Some(vec![ParamDef { key: "stop", label: "Money stop in SPI points ($1,500 at $25)", def: 60., min: 10., max: 200., step: 5. }]),
        "dgnl_spi_breakout" => Some(vec![ParamDef { key: "stop", label: "Money stop in SPI points ($1,500 at $25)", def: 60., min: 10., max: 200., step: 5. },
            ParamDef { key: "bond", label: "Bond 5-day-high filter: 0 none (tested), 3 = 3-year, 10 = 10-year", def: 0., min: 0., max: 10., step: 1. }]),
        "dgnl_spi_dow_bonds" => Some(vec![ParamDef { key: "day", label: "Entry weekday (1 Monday .. 5 Friday)", def: 3., min: 1., max: 5., step: 1. },
            ParamDef { key: "hold", label: "Extra days held (0 = out at the same close)", def: 0., min: 0., max: 3., step: 1. },
            ParamDef { key: "bond", label: "Bond 5-day-high filter: 0 none, 3 = 3-year, 10 = 10-year", def: 0., min: 0., max: 10., step: 1. }]),
        "dgnl_spi_12_10" => Some(vec![ParamDef { key: "stop", label: "Money stop in SPI points ($1,500 at $25)", def: 60., min: 10., max: 200., step: 5. },
            ParamDef { key: "hi", label: "Buy on an N-day high", def: 12., min: 2., max: 60., step: 1. }, ParamDef { key: "lo", label: "Sell on an N-day low", def: 10., min: 2., max: 60., step: 1. }]),
        _ => None,
    }
}

/// Lane PF: Gibbs' day-of-month SPI rules (08/09/2000 p.22), dates grouped by their stated month (see the card note).
const DOM_LONG: &[(u32, u32)] = &[(3, 1), (3, 31), (4, 30), (6, 1), (6, 30), (8, 1), (8, 31), (9, 30), (10, 30), (11, 30)];
const DOM_SHORT: &[(u32, u32)] = &[(5, 8), (6, 8), (8, 8), (9, 8), (10, 8), (12, 8)];
/// Lane PF: Gibbs' January-March dates (29/09/2000 p.13-14, the table's 'y' cells less 25 Jan and 30 Mar), long only.
const Q1_LONG: &[(u32, u32)] = &[(1, 9), (1, 13), (1, 24), (2, 1), (2, 24), (3, 1), (3, 9), (3, 13), (3, 22), (3, 31)];

/// Lane PF: true on bar i when the SFE bond (p.bond 3 / 10) closed above the highest close of its 4 bars before, on its own bars,
/// joined by exact date (no bond close that day = false); all true when p.bond = 0.
fn bond_high(x: &Ctx, p: &Params) -> Vec<bool> {
    let n = x.r.len();
    let sym = match p.get("bond").copied().unwrap_or(0.).round() as i64 { 3 => "&YYT", 10 => "&YXT", _ => return vec![true; n] };
    let Some(rows) = crate::futures::rows(sym) else { return vec![false; n] };
    let s = crate::second::align(sym, x.r, rows);
    let c = s.exact(crate::second::Field::C);
    let hi4 = s.own(|r| (0..r.len()).map(|k| if k >= 4 { r[k - 4..k].iter().map(|q| q.close).fold(f64::MIN, f64::max) } else { f64::NAN }).collect());
    (0..n).map(|i| c[i] > hi4[i]).collect()
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if id == "dgnl_spi_dow_bonds" {
        if from > to || to >= x.r.len() { return None; }
        let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d);
        let (day, hold) = ((g("day", 3.).round() as i64 - 1).clamp(0, 4), g("hold", 0.).round().clamp(0., 3.) as usize);
        let ok = bond_high(x, p);
        let mut out = vec![];
        let mut k = from.max(1);
        while k + hold <= to {
            if super::jbcdt2::weekday(&x.r[k].date) == day && ok[k - 1] {
                out.push(super::rbtie::mk(1., k, x.o[k], "open", k + hold, x.c[k + hold], "close", false, k - 1));
                k += hold + 1;
            } else { k += 1; }
        }
        return Some(out);
    }
    if !matches!(id, "dgnl_spi_dom" | "dgnl_spi_12_10" | "dgnl_spi_seasonal_q1" | "dgnl_spi_breakout") || from > to || to >= x.r.len() { return None; }
    let dated = id != "dgnl_spi_12_10"; // the dated rules and the breakout exit at the first profitable open
    let ek = if id == "dgnl_spi_breakout" { "stop" } else { "open" };
    let bonds = bond_high(x, p);
    let stop_pts = p.get("stop").copied().unwrap_or(60.);
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let md = |i: usize| -> (u32, u32) { let d = &x.r[i].date; (d[5..7].parse().unwrap_or(0), d[8..10].parse().unwrap_or(0)) };
    let (nh, nl) = (p.get("hi").copied().unwrap_or(12.).max(1.) as usize, p.get("lo").copied().unwrap_or(10.).max(1.) as usize);
    let want = |i: usize| -> i32 {
        if id == "dgnl_spi_dom" { return if DOM_LONG.contains(&md(i)) { 1 } else if DOM_SHORT.contains(&md(i)) { -1 } else { 0 }; }
        if id == "dgnl_spi_seasonal_q1" { return Q1_LONG.contains(&md(i)) as i32; }
        if id == "dgnl_spi_breakout" { // Friday or Monday bar with low[5] - high[1] > 0 (p.bond: and the bond at a 5-day high that day)
            let wd = super::jbcdt2::weekday(&x.r[i].date);
            return (i >= 5 && (wd == 4 || wd == 0) && l[i - 5] - h[i - 1] > 0. && bonds[i]) as i32;
        }
        // dgnl_spi_12_10: today's high is the N-day high / today's low the N-day low; both at once = no signal
        let up = i + 1 >= nh && h[i] >= h[i + 1 - nh..=i].iter().cloned().fold(f64::MIN, f64::max);
        let dn = i + 1 >= nl && l[i] <= l[i + 1 - nl..=i].iter().cloned().fold(f64::MAX, f64::min);
        if up && !dn { 1 } else if dn && !up { -1 } else { 0 }
    };
    let mut out = vec![];
    let mut pos: Option<(i32, usize, f64)> = None; // (dir, entry bar, entry price)
    for k in from + 1..=to {
        let sig = want(k - 1);
        // 1) at the open: first profitable open, or an opposite dated signal from yesterday
        let was = pos.map(|p| p.0);
        if let Some((d, e, ep)) = pos {
            if (dated && k > e && (d as f64) * (o[k] - ep) > 0.) || (sig != 0 && sig != d) {
                out.push(super::rbtie::mk(d as f64, e, ep, ek, k, o[k], if sig != 0 && sig != d { "reversed at the open" } else { "first profitable open" }, false, k - 1));
                pos = None;
            }
        }
        // 2) entry at this open on yesterday's date signal
        // (a same-direction signal while already in that position is ignored, as TradeStation without pyramiding)
        if pos.is_none() && sig != 0 && was != Some(sig) {
            if id == "dgnl_spi_breakout" { // buy stop at this open + (low[5] - high[1]) of the signal bar
                let lvl = o[k] + (l[k - 6] - h[k - 2]);
                if h[k] >= lvl { pos = Some((1, k, lvl)); }
            } else { pos = Some((sig, k, o[k])); }
        }
        // 3) the money stop during the day
        if let Some((d, e, ep)) = pos {
            let st = ep - (d as f64) * stop_pts;
            if (d > 0 && l[k] <= st) || (d < 0 && h[k] >= st) {
                let px = if d > 0 { o[k].min(st) } else { o[k].max(st) };
                out.push(super::rbtie::mk(d as f64, e, ep, ek, k, px, "money stop ($1,500)", false, k));
                pos = None;
            }
        }
    }
    if let Some((d, e, ep)) = pos { out.push(super::rbtie::mk(d as f64, e, ep, ek, to, c[to], "open", true, to)); }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{connors::Params, Row};
    #[test]
    fn day_of_month_entry_next_open_first_profitable_open_and_stop() {
        // Unit-test fixture only (no market data): the bar dated 1 Mar (a long date)
        // signals a long at the 2 Mar open (100); 5 Mar opens at 99, 6 Mar opens at 101 -> exit at 101 (first profitable open).
        let bar = |d: &str, o: f64, l: f64| Row { ticker: "T".into(), date: d.into(), open: o, high: o + 1., low: l, close: o, volume: None };
        let r = vec![bar("1990-02-28", 100., 99.), bar("1990-03-01", 100., 99.), bar("1990-03-02", 100., 99.), bar("1990-03-05", 99., 98.), bar("1990-03-06", 101., 100.)];
        let t = run("dgnl_spi_dom", &Ctx::new(&r, &r, &[]), 0, 4, &Params::new()).unwrap();
        assert_eq!(t.iter().map(|t| (t.dir, t.legs[0].i, t.exit_i, t.exit_px)).collect::<Vec<_>>(), vec![(1.0, 2, 4, 101.)]);
        // a 60-point money stop: the 5 Mar low at 39 hits 100 - 60 = 40 -> filled at the stop
        let r2 = vec![bar("1990-02-28", 100., 99.), bar("1990-03-01", 100., 99.), bar("1990-03-02", 100., 99.), bar("1990-03-05", 99., 39.)];
        let t2 = run("dgnl_spi_dom", &Ctx::new(&r2, &r2, &[]), 0, 3, &Params::new()).unwrap();
        assert_eq!((t2[0].exit_i, t2[0].exit_px), (3, 40.));
    }
}
