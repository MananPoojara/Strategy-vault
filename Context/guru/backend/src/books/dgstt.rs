//! Vladimir Daragan, How to Win the Stock Market Game / Developing Short-Term Stock Trading Strategies (same text) and Short-Term Trading Analysis (1999). Source: library/dgstt/README.md.
//! Since lane K56 (5 Nov 2026, EP5; decision #20) the low risk strategy (STTA p.24-27, 31-38) runs in books::rotation on the
//! vault's stock baskets as a declared application (check authors/titles-1030/K56w/k56_ref.py); the rest stay catalogue only.
use crate::connors::{claim, Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, dir, status, rules, notes| StrategyDef { id, name, group, session: 0, dir, status, rules, notes,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "dgstt", claim: None };
    vec![
        d("dgstt_basic", "Basic Trading Strategy", "STTA ch.5", "Long", Status::NeedsData("cross-sectional selection: each day buy the 2 stocks with the lowest CLO1/CLO0 among the potentially bullish list of ~1,000 US active stocks (volume > 200,000, price > $10), with T normalised against all active stocks that day; the engine runs one instrument at a time and the vault has no such universe"),
          &["F|Potentially bullish = active stock in T-D cell 13 (T < -1 and D < -1, normalised) in the 16 or 32 day frame (STTA p.24-27)", "E|Before the close of day 1 buy the two stocks with maximal % drop on day 1 (p.31)", "X|Sell at the open of day 4 (p.31)"],
          "Book: 11/1995-5/1999, 861 days, 3.25% per purchase +/- 8.2, 67.8% positive (p.33)."),
        StrategyDef { claim: claim("112 trading days", "77.7%", "6.95% per purchase (+/- 10.5)", "sell at the open of day 4", "Nov 1995 - May 1999, ~1,000 US active stocks a day (p.38)"),
          ..d("dgstt_low_risk", "Low risk strategy (both picks down > 10%)", "STTA ch.6", "Long", Status::Live,
          &["F|Active stocks: \"Trading volume on the day of analysis is more than 200,000. The closing price is more than $10. Stocks should have at least a 32 day trading history before the day of analysis.\" (p.25)",
            "F|Trend and deviation over N closes P1..PN: fit L(i) = A + Bi; T = B/(A + B*N) * 100%; sigma^2 = <(Pi - A - Bi)^2>; D = (PN - A - BN)/sigma (p.11-14); T -> (T - <T>)/sigma_T and D -> (D - <D>)/sigma_D over all active stocks of the day (p.16, p.25)",
            "F|\"Potentially bullish stocks - active stocks which are located in DT cell #13 in the 16 or 32 day frame\" (p.27); cell #13: T < -1 and D < -1 (p.16-17)",
            "E|\"Before market closing on the next day after the analysis check the stocks in the potentially bullish stock list. Find two stocks with maximum price (in %) decline during day #1, i.e., the ratio CLO1/CLO0 should be minimal. If the price decline of both stocks is more than 10%, buy these stocks.\" (Table 6.1, p.36)",
            "X|\"Hold these stocks for two days. Sell the stocks at the market opening on day #4.\" (Table 6.1, p.36)",
            "F|\"the trading capital should be divided into three equal parts. Every part of the capital should be used to buy two stocks\" (p.31): part A buys on day 1 and again on day 4"],
          "EP5 basket ranking (lane K56; decision #20: code it on the S&P 500 and NIFTY baskets as a declared application). The book's universe is ~1,000 US active stocks a day, 1995-1999; here every stock in the vault that was ever in the S&P 500 (us_sp500_past on the SPY calendar, to 11 Sep 2020: current members and former members with their own price files); membership dates are not known, so a stock is ranked whenever it has prices (decision #239). p.members = 1 ranks today's list only (us_sp500, Norgate snapshot of 8 Sep 2020; labelled option, survivorship bias). India: withheld, the NSE basket is the 2026 member list; a ranking over 20 years of today's members is dominated by survivorship bias (#239 (2)); large-cap S&P stocks rarely fall 10% in a day, so far fewer trading days than the book's 112. The $10 price and 200,000-share volume filters are not applied: the basket stands in for the active-stock list, and fixed amounts on split-adjusted prices are not the book's amounts (#166); the 32-day history is applied (32 bars before the day of analysis). Readings: T and D from each stock's own last 16 and 32 closes (least squares, i = 1..N; sigma = root mean square residual), normalised over the basket stocks that have them that day (population standard deviation); the list is built at day 0's close; day 1 is the next trading day; the two smallest CLO1/CLO0 ratios among the listed stocks with bars on both days (ties by ticker), both below 0.90 (p.drop = 10), are bought at day 1's close and sold at the open three trading days later (day #4), or at the stock's next real open. Capital: three equal parts by trading day (bar number mod 3), each split equally between its two stocks; a part whose day has no trade stays in cash. Price only (no dividends).") },
        d("dgstt_very_low_risk", "Very low risk strategy (two-day drop)", "STTA ch.6.3", "Long", Status::NeedsData("same cross-sectional universe; K1/K2 choice only tabulated"),
          &["E|Buy at CLO2 if CLO1 < K1*CLO0 and CLO2 < K2*CLO1 (p.39)", "X|Sell on the morning of day 5 (p.40)"],
          "Returns tabulated by K1, K2 (Tables 6.5-6.6)."),
        d("dgstt_short", "Selling short overbought stocks", "STTA ch.10", "Short", Status::NotTestable("universe-relative T/D thresholds and no single exit (close of day 3 or 4) are given; event study tables only"),
          &["E|Overbought in 16 and 32 day frames, rose the previous day, OPE1 > K1*CLO0; short at the open (p.51-53)", "X|\"The optimal time to close your short position is the market closing on day #3 or #4\" (p.53)"],
          ""),
        d("dgstt_oversold16", "Oversold 16-day (T < -0.5581, D < -1.185) event study", "How to Win part 4", "Long", Status::NotTestable("an event study: the holding period N = 1..9 days is the study variable, no exit rule is fixed (p.44)"),
          &["E|Oversold within 16 days: T < -0.5581 and D < -1.185 (p.44)"],
          ""),
        d("dgstt_upgrades", "Analyst upgrade pullback", "STTA ch.11.3", "Long", Status::NotTestable("discretionary (\"buy this stock when pullback is over\") and needs a dated feed of analyst-upgrade news"),
          &[],
          ""),
    ]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { catalogue().into_iter().find(|d| d.id == id).map(|d| (d.id, d.name, "")) }
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    (id == "dgstt_low_risk").then(|| vec![ParamDef { key: "drop", label: "Level of selection: both picks fell more than this % on day 1 (book 10; Table 6.2 also 5)", def: 10., min: 0., max: 20., step: 5. }, ParamDef { key: "members", label: "Universe: 0 = every stock ever in the index (us_sp500_past, default, #239), 1 = today's members only (us_sp500, labelled option)", def: 0., min: 0., max: 1., step: 1. }])
}
pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }
