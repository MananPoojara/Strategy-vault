//! Ned Davis, "The Triumph of Contrarian Investing" (McGraw-Hill, 2003/2004). Book id "ndtci". Catalogue only.
//! Source copy: Books/Ned Davis/The Triumph of Contrarian Investing.pdf (192 scanned pages, printed page = pdf page - 15;
//! every page read). The one complete trading rule (Fig 5-2) needs Investors Intelligence sentiment and a short-term
//! government bond series that the vault does not have.
use crate::connors::{claim, Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, dir, status, rules, notes| StrategyDef { id, name, group, session: 0, dir, status, rules, notes,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "ndtci", claim: None };
    vec![
        StrategyDef { claim: claim("not printed", "100% profitable long trades", "14.3%/yr vs buy-hold 6.4%/yr", "", "S&P 500, weekly, 7/19/1968-2/21/2003"),
          ..d("ndtci_ii_brackets", "Investors Intelligence dynamic brackets", "Ch 5 · Indicators of crowd psychology", "Long",
            Status::NeedsData("weekly Investors Intelligence bulls and bears (1968-) and a short-term government bond price series for the 26-week rate of change; neither is in the vault (India: no equivalent advisor survey)"),
            &["F|Sentiment = Bulls/(Bulls + Bears), Investors Intelligence, weekly (Fig 5-2, p.50)",
              "F|\"(Monetary Conditions Based on 26-Week Rate of Change in Short-Term Government Bonds)\" (Fig 5-2); \"when rates are falling ... When rates are rising\" (p.49)",
              "E|\"Monetary Positive: Buy When Bulls/(Bulls + Bears) < 51\"; \"Monetary Negative: Buy When ... < 31\" (Fig 5-2)",
              "X|\"Monetary Positive: ... Sell When Bulls/(Bulls + Bears) > 81\"; \"Monetary Negative: ... Sell When ... > 58\"; \"S = Switch into Commercial Paper\" (Fig 5-2, p.49-50)"],
            "pp.48-50. Latest signal printed: 7/19/2002 = 847.76. 'Monetary positive' read as a positive 26-week change in the bond price (falling rates).") },
        d("ndtci_zone_tables", "Sentiment and valuation zone tables", "Ch 1, 5", "Long", Status::NotTestable("conditional 'gain per annum when in zone' statistics, not trading rules; most series are proprietary or unavailable"),
          &["F|AAII bulls/(bulls+bears) 2-week average: above 61 = 0.0%/yr, 44-61 = 8.2%/yr, 44 and below = 20.6%/yr S&P gain (Fig 5-1, p.48-49)",
            "F|Rydex bull/bear assets (Fig 5-3), COT speculator index as % of 78-week range, 6-week smoothing, below 40 / above 60 (Fig 5-4, p.52), Wall Street strategist sentiment (Fig 5-5), consumer confidence (Fig 1-8), mutual fund cash (Fig 1-10), Gallup approval (Fig 1-14), P/E and 10-year yield/earnings yield (Figs 5-8, 5-9), household equity share (Fig 5-10)"],
          "pp.1-17, 47-60."),
        d("ndtci_crowd_poll", "NDR Crowd Sentiment Poll extremes", "Ch 5", "Long/Short", Status::NotTestable("proprietary composite; the extremes table is 'determined in hindsight' (p.55)"),
          &["F|\"Sentiment must reverse by 10 percentage points to signal an extreme\" (Fig 5-6, 5-7, p.54-55)"], "pp.53-56."),
        d("ndtci_covers", "Magazine-cover contrarian study", "Ch 4", "Long/Short", Status::NotTestable("cover classification is judgement and no cover data set exists"),
          &["E|\"if you had invested in the stock market for the 30-day period following the cover stories, your investment would have gained at a rate of 30 percent per annum ... by acting contrary to the magazine covers after 30 days, you would have beaten buy-and-hold by about five times over the next 11 months\" (Montgomery study, p.35)"], "pp.35-46."),
        d("ndtci_stock_brackets", "Addendum: individual-stock sentiment and valuation brackets", "Addendum", "Long", Status::NotTestable("brackets 'developed with the benefit of hindsight'; price/sales, P/E, P/CF need stock fundamentals; the 39-week MA use is advice ('watch for'), not a rule"),
          &["F|Figs A-1 to A-100: US stocks' price vs valuation zones; combine with a 39-week moving average (p.65-69)"], "pp.65-170."),
    ]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { catalogue().into_iter().find(|d| d.id == id).map(|d| (d.id, d.name, "")) }
pub fn params(_id: &str) -> Option<Vec<ParamDef>> { None }
pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }
