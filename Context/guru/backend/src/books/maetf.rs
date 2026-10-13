//! Marvin Appel, Investing with Exchange Traded Funds Made Easy (FT Press / Pearson, 2007).
//! Every timing model in the book switches between two asset classes (large/small caps, growth/value,
//! EAFE/S&P 500, S&P 500/Treasury bills) or holds a fixed multi-asset mix. The vault holds single equity
//! indices only (no Russell 1000/2000 or style indices, no EAFE, no REIT fund average, no bond or
//! Treasury yield series), so every rule is NeedsData with the exact gap. Page refs are printed pages.
//! Sources: library/maetf/.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "maetf";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, dir, why, rules, notes| StrategyDef {
        id, name, group, session: 0, dir, status: Status::NeedsData(why), needs_volume: false, needs_vix: false, needs_series: &[],
        book: BOOK, rules, notes, claim: None,
    };
    vec![
        d("maetf_one_decision", "One-decision portfolio (annual rebalance)", "Ch 7 · The one-decision portfolio (p.115-129)", "Long",
            "needs monthly total returns of the REIT mutual-fund average, Russell 2000 Value, Lehman (Barclays) Aggregate Bond index and 90-day Treasury bills (the vault has S&P 500 price data only)",
            &["E|\"20 percent S&P 500, 20 percent REITs, 10 percent small-cap value, 20 percent investment-grade bonds, 30 percent cash (90-day U.S. Treasury bills)\" (p.118)",
              "X|\"I recommend rebalancing after holding for one year and one day\" (p.126); less-conservative version without cash: 2/7 S&P 500, 2/7 REITs, 1/7 small-cap value, 2/7 bonds (p.123)"],
            "Book: 1981-2005 calendar-year returns in Table 7.1 (p.120); 10.7%/yr compounded, drawdown 11.2% (5/1980-2/2006)."),
        d("maetf_small_large", "Small- vs large-cap yearly switch (Russell 2000 / Russell 1000)", "Ch 8 · When to live large (p.138-143)", "Long",
            "needs Russell 1000 and Russell 2000 total-return index history (from 1979); the vault's IWM starts 2000 and has no Russell 1000, and the engine runs one ticker at a time",
            &["E|\"Invest for the coming year in whichever of the two alternatives returned more in the previous year.\" (p.138)",
              "F|\"On the last trading day of the year, calculate the total returns for each of these indexes. Take whichever of the two returned more as your investment selection for the coming year.\" (p.139)"],
            "Book: 1980-3/2006, 19 of 27 selections correct, 15.3%/yr vs 13.1% (R1000) and 12.6% (R2000), max drawdown -21% (Table 8.1, p.140-141)."),
        d("maetf_growth_value", "Growth vs value relative-strength switch (10% rule)", "Ch 9 · Boring bargains or hot prospects? (p.151-173)", "Long",
            "needs growth and value style index total returns (Russell 2000 Growth/Value, Russell 1000 or S&P 500/Barra Growth/Value); not in the vault",
            &["F|\"Growth versus value rule: A disparity of 10 percent in relative performance between growth and value defines the current long-term trend.\" (p.162)",
              "E|Switch when the relative strength ratio rises 10% off its most recent trough (or falls 10% off its peak) (p.163-165)"],
            "Book applies it to small-cap and large-cap value vs growth (p.166-170)."),
        d("maetf_eafe_sp", "EAFE vs S&P 500 switch (15% rule)", "Ch 10 · When is it safe to drink the water? (p.181-187)", "Long",
            "needs MSCI EAFE (US$) total-return history; not in the vault",
            &["E|\"when the S&P 500 index lags the EAFE by more than 15 percent, it is time to invest abroad. Conversely, when the EAFE lags the S&P 500 by 15 percent or more, invest in the United States.\" (p.181-182)",
              "F|Measured from the latest peak or trough of the EAFE/S&P 500 ratio (Figure 10.4, p.183)"],
            "Book: trade list in Tables 10.2a/b (p.184-185), 9 of 17 switches successful."),
        d("maetf_rising_rates", "Out of stocks while 10-year yields are rising", "Ch 11 · An indicator of rising interest rates (p.196-197)", "Long",
            "needs monthly average 10-year constant-maturity Treasury yields (Federal Reserve H.15); bond data is not in the vault",
            &["X|Switch from the S&P 500 into Treasury bills when \"10-year Treasury yields being higher than six months previously, based on monthly average data from the Federal Reserve\" (p.197)",
              "E|Otherwise hold the S&P 500"],
            "Book: S&P 500 1965-2006 (Figure 11.2)."),
        d("maetf_yield_curve", "Yield curve indicator", "Ch 11 · The rules for the yield curve indicator (p.199-202)", "Long",
            "needs monthly average 90-day Treasury bill and 10-year Treasury note yields (Federal Reserve H.15); bond data is not in the vault",
            &["F|\"At the start of each month, you check the Federal Reserve website for the monthly data on 90-day Treasury bill yields (secondary market) and on 10-year constant maturity Treasury note yields\" (p.200)",
              "X|\"if the 90-day yield exceeds the 10-year yield by any amount, you should exit your equity investments\" (p.200)",
              "E|\"Do not re-enter the stock market until the Fed's monthly data indicates that the yield curve has returned to normal.\" (p.200)"],
            "Book: nine inversions 1965-3/2006 listed in Table 11.2 (p.202); S&P 500 -13.9% vs T-bills +58.6% during them."),
        d("maetf_composite_rates", "Composite interest rate indicator", "Ch 11 · Composite interest rate indicator (p.203-204)", "Long",
            "needs monthly average 90-day T-bill and 10-year Treasury yields (Federal Reserve H.15) and T-bill returns; bond data is not in the vault",
            &["X|\"The investor should be in Treasury bills if the yield curve is inverted or if 10-year Treasury yields are higher than they were six months earlier.\" (p.203)",
              "E|\"Otherwise, the investor holds the S&P 500. Check these conditions each month\" (p.203)"],
            "Book: S&P 500 1/1965-3/2006, 11.5%/yr vs 10.4% buy and hold, drawdown 23% vs 45% (Table 11.3, p.204)."),
        d("maetf_ultimate", "Ultimate ETF investment program", "Ch 13 · The ultimate ETF investment program (p.223-235)", "Long",
            "combines the REIT/bond core, the EAFE/S&P 500 switch, the composite interest-rate indicator and the growth/value switch: needs all of their data (REITs, bonds, EAFE, style indices, Treasury yields)",
            &["F|\"50 percent of your investment capital should be invested for the long term, equally balanced between REITs and U.S. investment-grade bonds\" (p.227)",
              "E|25%: EFA if the Chapter 10 model favors foreign stocks; else SPY unless \"the yield curve is inverted or interest rates are rising\", then money market (p.227-228)",
              "E|25%: the growth/value model of Chapter 9 picks the style ETF (p.228-230)"],
            "Book results: Table 13.3 and p.231-232."),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    id.starts_with("maetf_").then_some(("maetf_alloc", "ETF asset-allocation models", "Model"))
}

pub fn params(_id: &str) -> Option<Vec<ParamDef>> {
    None
}

pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> {
    None
}
