//! Gary Antonacci, "Dual Momentum Investing" (McGraw-Hill 2014). Book id `gadm`.
//! Source copy: Books/Gary Antonacci/Gary Antonacci - Dual Momentum Investing ... (2014) (1).epub. Printed page refs
//! from the epub page anchors. Transcription: library/gadm/.
//! Every model switches monthly between total-return indices and needs Treasury bills and/or a bond index, which the
//! vault does not hold. No proxies are used (S&P price index without T-bills/bonds would not be the book's rule), so
//! every entry is NeedsData or NotTestable and `run` returns None.
use crate::connors::{ParamDef, Params, Status, StrategyDef, Ctx, Trade};

pub const BOOK: &str = "gadm";

const TB_AGG: &str = "Needs monthly total returns of 90-day US Treasury bills and the Barclays US Aggregate Bond Index (plus S&P 500 total return); the vault has only price OHLC for SPX/SPY and no T-bill or bond series.";
const ACWI: &str = "Needs monthly total returns of MSCI ACWI ex-U.S. (MSCI World ex-US before 1988), S&P 500, 90-day US Treasury bills and the Barclays US Aggregate Bond Index; the vault has none of these except S&P price OHLC.";

macro_rules! nd {
    ($id:expr, $name:expr, $group:expr, $status:expr, $rules:expr, $notes:expr) => {
        StrategyDef { id: $id, name: $name, group: $group, session: 0, dir: "Long", status: $status, needs_volume: false,
            needs_vix: false, needs_series: &[], book: BOOK, rules: $rules, notes: $notes, claim: None }
    };
}

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        nd!("gadm_gem", "Global Equities Momentum (GEM)", "Ch. 8 · Global Equities Momentum", Status::NeedsData(ACWI),
            &["F|\"We first compare the S&P 500 to the ACWI ex-U.S. over the past year and select whichever one has performed better\" (p.101)",
              "E|\"We then check to see if our selected index has done better than U.S. Treasury bills. If it has, we invest in that index\" (p.101)",
              "X|\"If it has not, we invest instead in U.S. aggregate bonds. We repeat this procedure every month\" (p.101)",
              "F|Look-back 12 months, no skip month (p.94-95); \"Its only parameter is a 12-month look-back period\" (p.112)"],
            "Published (p.102-103): 1974-2013, 41% S&P 500 / 29% ACWI ex-U.S. / 30% aggregate bonds, 1.35 switches per year, 17.43% average annual return, 12.64% SD, Sharpe 0.87, max drawdown 22.7%. Table 8.5 tests 3-12 month look-backs. ETF route (p.112): 252-day returns of US stock, non-US stock and T-bill ETFs."),
        nd!("gadm_abs_sp500", "S&P 500 absolute momentum (Table 8.1)", "Ch. 8 · Applied absolute momentum", Status::NeedsData(TB_AGG),
            &["E|\"if the S&P 500 shows a positive excess return (return less the Treasury bill rate) during the past 12 months, we stay invested in it\" (p.95)",
              "X|\"If the prior 12-month excess return of the S&P 500 is negative, we exit the S&P 500 Index and hold the Barclays U.S. Aggregate Bond Index instead. We stay in aggregate bonds until the excess return of the S&P 500 is again positive\" (p.95)"],
            "Table 8.1, 1974-2013 (image). Text p.122: 70% of the time in stocks, 31 trades in 40 years (0.83/yr)."),
        nd!("gadm_rel_acwi", "ACWI relative momentum (Table 8.3)", "Ch. 8 · Relative vs absolute momentum", Status::NeedsData(ACWI),
            &["E|\"We apply relative momentum to ACWI by selecting the stronger of its two components [S&P 500, ACWI ex-U.S.] based on their relative performance over the preceding 12 months\" (p.98)", "X|Re-evaluated monthly (p.98)"],
            "Text p.98: +556 bp/yr over ACWI 1974-2013."),
        nd!("gadm_abs_acwi", "ACWI absolute momentum (Table 8.3)", "Ch. 8 · Relative vs absolute momentum", Status::NeedsData(ACWI),
            &["E|\"apply absolute momentum to ACWI by switching between it and the Barclays U.S. Aggregate Bond Index based on whether the excess return of the S&P 500 has been positive or negative during the past 12 months\" (p.98)"],
            "Text p.98-99: +381 bp/yr over ACWI, SD -3.6%, max drawdown cut >60%."),
        nd!("gadm_ma10_sp500", "S&P 500 10-month moving average, bonds when out (Table 9.1)", "Ch. 9 · Trend following with moving averages", Status::NeedsData(TB_AGG),
            &["E|\"one holds a long position when the price is above its 10-month moving average and exits the position when the price falls below\" (p.122, Faber's rule as tested)",
              "X|\"When not invested in stocks, all three strategies are in U.S. aggregate bonds\" (p.122)"],
            "Table 9.1 1974-2013: in stocks 74% of the time, 49 trades (1.2/yr) (p.122). The out-of-market leg is the bond index, which the vault lacks. The 12-month MA variant of Table 9.1 has the same data gap."),
        nd!("gadm_abs_r1000", "Russell 1000 absolute momentum (Table 9.2)", "Ch. 9 · Relative momentum revisited", Status::NeedsData(
            "Needs Russell 1000 total returns, 90-day T-bills and the Barclays US Aggregate Bond Index (1980-2013); none are in the vault."),
            &["E|\"12-month absolute momentum (with aggregate bonds as a safe harbor) applied to the Russell 1000 Index\" (p.124)"], ""),
        nd!("gadm_gbm", "Global Balanced Momentum (GBM)", "Ch. 9 · Global Balanced Momentum", Status::NotTestable(
            "proprietary model: the dual-momentum selection among the four fixed-income alternatives (how many are held, how T-bills compare) is not specified; also needs bond index data the vault lacks"),
            &["F|\"70% allocated to the same equities holdings as GEM, but its fixed-income holdings, including a permanent 30% allocation to fixed income, are selected from the previous list of fixed-income alternatives using dual momentum\" (p.129)",
              "F|Fixed-income alternatives: Barclays US Long Treasury, BofA ML Global Government, BofA ML US Cash Pay High Yield, 90-day T-bills (p.129)"],
            "Called proprietary by the author (p.129). Table 9.4 is an image."),
        nd!("gadm_dmsr", "Dual Momentum Sector Rotation (DMSR)", "Ch. 9 · Dual Momentum Sector Rotation", Status::NotTestable(
            "the number of 'top-performing sectors' held is never stated (proprietary); also needs the 11 Morningstar US sector indices and the aggregate bond index"),
            &["E|\"We can select an equally weighted basket of the top-performing sectors using relative strength momentum\" (p.131)",
              "X|\"When the U.S. stock market is in a downtrend, according to absolute momentum, DMSR moves all of its assets into the Barclays Capital U.S. Aggregate Bond Index\" (p.131)"],
            "1993-2013, Table 9.5 (image)."),
        nd!("gadm_appb_abs", "Absolute momentum, single asset vs T-bills (Appendix B)", "Appendix B · Absolute momentum", Status::NeedsData(
            "Needs monthly total returns (interest and dividends) of the asset and of 90-day US Treasury bills; the vault has price OHLC only and no T-bills."),
            &["E|\"absolute momentum as being positive when the excess return (asset return less the Treasury bill return) over the formation (look-back) period is positive. We hold a long position in our selected assets during these times\" (p.148)",
              "X|\"When absolute momentum turns negative ... exit the asset and switch into 90-day U.S. Treasury bills until absolute momentum again becomes positive\" (p.148)",
              "F|\"We reevaluate and adjust positions monthly\"; 12-month formation period; 20 bp per switch (p.148-149)"],
            "Applied to MSCI US, MSCI EAFE, Barclays bond indices, REITs, GSCI, gold, 1974-2012 (Table B.2, image)."),
        nd!("gadm_appb_portfolios", "60/40 and parity portfolios with absolute momentum (Appendix B)", "Appendix B · Absolute momentum", Status::NeedsData(
            "Needs MSCI US, US Treasury, credit bond, REIT and gold monthly total returns plus T-bills; none in the vault."),
            &["F|60/40 MSCI US / US Treasury, or equal-weight MSCI US, long Treasury, REITs, credit bonds, gold (p.157-161)", "E|12-month absolute momentum applied to each asset (p.157-161)"], ""),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let f = |v| Some(("gadm", "Dual Momentum (Antonacci)", v));
    match id {
        "gadm_gem" => f("GEM"),
        "gadm_abs_sp500" => f("S&P 500 absolute momentum"),
        "gadm_rel_acwi" => f("ACWI relative momentum"),
        "gadm_abs_acwi" => f("ACWI absolute momentum"),
        "gadm_ma10_sp500" => f("10-month MA"),
        "gadm_abs_r1000" => f("Russell 1000 absolute momentum"),
        "gadm_gbm" => f("GBM · not testable"),
        "gadm_dmsr" => f("DMSR · not testable"),
        "gadm_appb_abs" => f("Appendix B single asset"),
        "gadm_appb_portfolios" => f("Appendix B portfolios"),
        _ => None,
    }
}

pub fn params(_id: &str) -> Option<Vec<ParamDef>> {
    None
}

pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> {
    None
}
