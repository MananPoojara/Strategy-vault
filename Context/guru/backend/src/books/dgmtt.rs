//! Daryl Guppy, "Market Trading Tactics: Beating the Odds through Technical Analysis and Money Management" (Wiley, 2000).
//! Book id "dgmtt". Catalogue only. Source copy: Books/Daryl Guppy/Guppy, Daryl - Market Trading Tactics.pdf (422 scanned
//! pages incl. duplicates; every page read). The searches are watch-list scans with no exit; trades are set on hand-drawn
//! support/resistance. Duplicates not catalogued: MACD 12/26/9 buy/sell crossover (p.128) = long legs of lkqts_macd;
//! NR4 + 6/100 HV (p.119) = Street Smarts (ss); GMMA rules (p.152-153) = dgmf_gmma / dggw.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, dir, status, rules, notes| StrategyDef { id, name, group, session: 0, dir, status, rules, notes,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "dgmtt", claim: None };
    use Status::NotTestable as NT;
    vec![
        d("dgmtt_scans", "R-plot searches (260-day high, inside day, doji, gaps, % change)", "Ch 8 · Technical true love", "Long",
          NT("watch-list scans: no entry order and no exit; candidates are then 'eyeballed'"),
          &["F|New 260-day high: \"Column A High, Column B HHV(High,260), Filter (Col A - Col B) = 0\" (p.103); ROC(CLOSE,1..5,PERCENT) ranking (p.103)",
            "F|Inside day \"Inside()\" (p.106-108), \"further confirmation ... is required\"; dragonfly doji \"LongLeggedDoji() AND LongLowerShadow() AND Doji()\" then \"watch the stock the next day for confirmation\" (p.108-109); gap days GapUp()/GapDown() (p.111)"],
          "pp.100-111."),
        d("dgmtt_price_volume", "Price and volume breakout search", "Ch 8 · Technical true love", "Long",
          NT("search list only: 'Not all are real candidates ... rejected due to weak chart patterns'; no entry order or exit"),
          &["F|\"ColC ROC(CLOSE,1,PERCENT), ColD VOLUME, ColE MOV(VOLUME,50,EXPONENTIAL) ... Filter WHEN(COLC>=5) AND WHEN(COLD>=COLE*1.5)\" (p.115-116)"],
          "pp.114-117."),
        d("dgmtt_ma_cross", "Moving-average crossover buy", "Ch 9 · Performance searches", "Long",
          NT("no average lengths and no exit are given"),
          &["E|\"When the current price is higher than the short-term average, and when the short-term average has crossed over, or turned above, the longer term average, a buying opportunity is signaled\" (p.128)"],
          "pp.121-128. The 3/10 EMA crossover lists (p.121-127, Fig 10.1) are search output; the Boeing crossovers are 'agreement' points that are 'usually ignored'."),
        d("dgmtt_laidlaw_mma", "Laidlaw MMA TOTAL search", "Ch 10 · An assay test", "Long",
          NT("the book says the search only gets 'to the right area ... you will still need to eyeball each of the results'; no exit"),
          &["F|\"MMA 10/45 If(OscP(10,45,E,%)>0,+1,-1)\" and likewise 12/50, 15/60, 3/30, 5/35, 8/40; \"MMA TOTAL\" = their sum (p.157)",
            "E|Exploration \"When (colD,>,0) AND When(colE,<=,0)\", colD = MMA TOTAL, colE = yesterday's MMA TOTAL (p.158)"],
          "pp.155-158."),
        d("dgmtt_oscillator_zones", "Stochastic / RSI overbought-oversold", "Ch 9-11", "Long/Short",
          NT("interpretation only: 'our interpretation of the potential for future price action is very subjective' (p.160); no trade rule"),
          &["F|%K 10 slowed 5 with 20/80 lines, RSI 30/70 (p.132-133); \"If the stochastic falls below the 20 percent line we believe the security is oversold\" (p.160)"],
          "pp.130-137, 159-160."),
        d("dgmtt_sr_trades", "Support/resistance and trend-line trades", "Ch 3, 7, 11-16", "Long",
          NT("levels and trend lines are drawn by eye ('Recognizing any of these patterns is a judgment call', p.86)"),
          &["E|General Property Trust: entry at the band midpoint $2.28, stop on a close below $2.26 (Ch 3)",
            "E|DMR plan: \"Stop loss - $0.095 for $2,000 maximum / Entry conditions - on support / Exit conditions - at resistance or reassess after six weeks\" (p.220, 237)",
            "X|Goodyear plan: exit when \"The price level is equivalent to 2 percent risk of equity\", \"If you are in the trade for a 10 percent return, then exit when it is achieved\", or \"When other indicators call a trend reversal\" (p.279)"],
          "pp.39-55, 84-99, 159-171, 197-281. Inverted-chart bias test (Ch 11) and index 'bell curve' placement (Ch 12) are judgement."),
        d("dgmtt_money_mgmt", "2 percent rule and trade planning", "Ch 13-14, 18", "Long",
          NT("money management only, no entry rule"),
          &["F|\"no single trade should put at risk more than 2 percent of your total trading equity\" (p.223); break-even with brokerage, position size from the chart stop (Figs 13.4-14.5, 18.1)"],
          "pp.197-237, 301-310. Fig 17.2 (p.293): Guppy's own 30 stock trades in 12 months, 22 wins / 8 losses, 73% win, 66.62% return on $100,000 nominal equity (discretionary, ASX stocks)."),
        d("dgmtt_depth", "Depth-of-market tactics", "Ch 15", "Long",
          NT("needs Level 2/4 order-book snapshots and judgement; no rule"),
          &["E|\"If our financial analysis allows us to chase the price to $1.65, then we shift the day's balance of probability in our favor by meeting the offer\" (p.256)"],
          "pp.239-268."),
    ]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { catalogue().into_iter().find(|d| d.id == id).map(|d| (d.id, d.name, "")) }
pub fn params(_id: &str) -> Option<Vec<ParamDef>> { None }
pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }
