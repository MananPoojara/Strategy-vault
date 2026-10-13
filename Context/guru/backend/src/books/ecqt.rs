//! Ernest P. Chan, *Quantitative Trading: How to Build Your Own Algorithmic Trading Business* (Wiley, 2008/2009).
//! Book id `ecqt`. Source copy: Books/Ernest Chan/Ernest Chan - Quatitative Trading.pdf.
//! Transcription and testability table: library/ecqt/README.md.
//!
//! Every fully specified strategy in the book trades a basket (S&P 500 / S&P 600 stocks), an ETF pair (GLD/GDX) or a
//! futures contract (RB, NG). The vault engine backtests one ticker per run; the NSE nifty50/nse500 baskets exist but
//! cannot be combined into one portfolio, so these are NeedsData with the exact gap. Kelly / optimal leverage
//! (Ch.6) is position sizing, not a strategy.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "ecqt";

const PORT: &str = "a cross-sectional stock portfolio on one equity curve (daily weights across every basket member). The engine backtests one ticker per run; NSE nifty50/nse500 basket prices are in the vault (current membership, the same survivorship bias Chan accepts) but there is no multi-asset portfolio simulator to combine them";

macro_rules! def {
    ($id:expr, $name:expr, $g:expr, $dir:expr, $st:expr, $rules:expr, $notes:expr) => {
        StrategyDef { id: $id, name: $name, group: $g, session: 0, dir: $dir, status: $st, needs_volume: false, needs_vix: false,
            needs_series: &[], book: BOOK, rules: $rules, notes: $notes, claim: None }
    };
}

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    vec![
        def!("ecqt_gld_gdx_pair", "GLD/GDX pair trade, z-score 2 / 1 (Example 3.6)", "Ch.3 · Backtesting examples", "Long/Short",
            NeedsData("GLD and GDX daily adjusted closes traded as one two-leg spread (neither ETF is in the vault; the engine runs one ticker per run). An NSE swap would need a pair the book does not name - Chan picks GLD/GDX by its own cointegration study (Ex 7.2-7.3)"),
            &["F|trainset = first 252 common days; hedgeRatio = ols(GLD, GDX) on the trainset; spread = GLD - hedgeRatio*GDX (p.57)",
              "F|zscore = (spread - mean(spread(trainset))) / std(spread(trainset)) (p.57)",
              "E|\"buy spread when its value drops below 2 standard deviations\": zscore <= -2 -> +1 GLD, -1 GDX; zscore >= 2 -> -1 GLD, +1 GDX (p.57-58)",
              "X|\"exit any spread position when its value is within 1 standard deviation of its mean\": abs(zscore) <= 1 (p.57)",
              "X|positions carried forward until an exit; pnl = sum(lag1(positions) .* dailyret) (p.58)"],
            "Published: Sharpe about 2.3 on the training set and about 1.5 on the test set; with entry 1 / exit 0.5 standard deviation 2.9 and 2.1 (p.58-59). Half-life of the spread about 10 days (Ex 7.5, p.141-142)."),
        def!("ecqt_kl_mr_close", "Khandani-Lo daily mean reversion, close to close (Example 3.7)", "Ch.3 · Backtesting examples", "Long/Short",
            NeedsData(PORT),
            &["F|universe: S&P 500 stocks (list of 21 Nov 2007), daily closes (p.61-63)",
              "E|weights = -(dailyret - marketDailyret) / (number of stocks with prices); marketDailyret = equal-weighted mean return (p.63)",
              "X|held one day: dailypnl = smartsum(lag1(weights) .* dailyret) (p.63); 5 bp per one-way trade on weight changes (p.64-65)"],
            "Published 2006: Sharpe about 0.25 before costs, about -3.19 after 5 bp (p.63-65)."),
        def!("ecqt_kl_mr_open", "Khandani-Lo daily mean reversion on opens (Example 3.8)", "Ch.3 · Backtesting examples", "Long/Short",
            NeedsData(PORT),
            &["E|Example 3.7 with \"cl\" replaced by \"op\" everywhere: \"update the positions at the market open instead of the close\" (p.66)"],
            "Published 2006: Sharpe 4.43 before costs, 0.78 after 5 bp (p.66)."),
        def!("ecqt_pca_factor", "PCA factor model, top/bottom 50 (Example 7.4)", "Ch.7 · Factor models", "Long/Short",
            NeedsData(PORT),
            &["F|S&P 600 small caps; lookback 252 days; numFactors 5 = eigenvectors of the return covariance with the top eigenvalues (p.137-138)",
              "E|Rexp = avgR + X*b (b = ols of the last day's returns on X); short the topN = 50 lowest, buy the 50 highest; held one day (p.138)"],
            "Published: annualized average return -1.8099 (as printed, p.138) - Chan's point is that it fails."),
        def!("ecqt_january", "January effect, small caps (Example 7.6)", "Ch.7 · Seasonal", "Long/Short",
            NeedsData(PORT),
            &["F|S&P 600 small caps; prior calendar-year return per stock (p.144-145)",
              "E|on the last trading day of December buy the lowest decile, short the highest decile (topN = round(N/10)) (p.145)",
              "X|exit on the last trading day of January; 5 bp one way (p.145)"],
            "Published portfolio returns: Jan 2006 -0.0244, Jan 2007 -0.0068, Jan 2008 0.0881 (p.145)."),
        def!("ecqt_yoy_seasonal", "Year-on-year seasonal trend (Example 7.7, Heston-Sadka)", "Ch.7 · Seasonal", "Long/Short",
            NeedsData(PORT),
            &["E|each month buy the top decile of stocks by their return in the same month a year earlier, short the bottom decile (p.146-147)",
              "X|hold one month (month-end to month-end) (p.147)"],
            "Published: avg annual return -0.9167, Sharpe -0.1055 (as printed, S&P 500 list of Nov 2007, p.147)."),
        def!("ecqt_rb_seasonal", "Gasoline futures spring trade", "Ch.7 · Seasonal", "Long",
            NeedsData("NYMEX RB May unleaded-gasoline futures contract daily settlements 1995-2008 (no futures in the vault; an Indian index is not a gasoline contract)"),
            &["E|\"buy 1 contract of RB ... that expires in May at the close of April 13 (or the following trading day if it is a holiday)\" (p.149)",
              "X|\"and sell it at the close of April 25 (or the previous trading day if it is a holiday)\" (p.149)"],
            "Published P&L per contract 1995-2008: every year positive (1995 $1,037 ... 2008* $4,741; *actual trading as 2 x QU) (p.149)."),
        def!("ecqt_ng_seasonal", "Natural gas futures spring trade", "Ch.7 · Seasonal", "Long",
            NeedsData("NYMEX NG June natural-gas futures contract daily settlements 1995-2008 (no futures in the vault)"),
            &["E|\"long a June contract of NYMEX natural gas futures (Symbol: NG) at the close of February 25 (or the following trading day if it is a holiday)\" (p.150)",
              "X|\"exit this position on April 15 (or the previous trading day if it is a holiday)\" (p.150)"],
            "Published P&L 1995-2008, profitable 14 consecutive years (1995 $1,970 ... 2008* $10,137) (p.150)."),
        def!("ecqt_gs_regime_ml", "GS regime switch via perceptron (Example 7.1)", "Ch.7 · Regime switching", "Long/Short",
            NotTestable("the trade decision is a perceptron's weighting of rule/holding-period combinations trained on a moving window in Alphacet Discovery; the weights and training window are not given (p.122-125)"),
            &["E|R3: 1-day % change >= 1 AND 10-day moving low >= Low -> 1; % change >= -1 AND 10-day moving high <= High -> -1 (Fig 7.2, p.124)",
              "X|holding periods 1, 5, 10, 20, 40, 60 days combined by a perceptron (p.123-124)"], ""),
        def!("ecqt_es_bb_5m", "ES Bollinger mean reversion on 5-minute bars", "Ch.2 · Transaction costs", "Long/Short",
            NotTestable("moving-average / standard-deviation lookback is not given and the rule runs on 5-minute E-mini bars (p.23)"),
            &["E|\"every time the price exceeds plus or minus 2 moving standard deviations of its moving average, short or buy, respectively\" (p.23)",
              "X|\"Exit the position when the price reverts back to within 1 moving standard deviation of the moving average\" (p.23)"],
            "Published: Sharpe about 3 without costs, -3 with 1 bp (p.23)."),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let d = catalogue().into_iter().find(|d| d.id == id)?;
    Some(("ecqt", "Chan · Quantitative Trading", d.name))
}
pub fn params(_id: &str) -> Option<Vec<ParamDef>> { None }
pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }
