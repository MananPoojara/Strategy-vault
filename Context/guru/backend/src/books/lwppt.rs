//! Larry Williams, "Picture Perfect Trading" (manual 2007, 139 pp, and the 2005/06 ADEST Sydney seminar book). Book id `lwppt`.
//! Sources (read in place; COPY PENDING): research drive "Trading Courses,Seminars,Videos/Larry Williams - Picture Perfect
//! Trading (Video & Manuals, 576 MB), $2895 (ireallytrade.com)/Manual/Picture Perfect Trading Manual.pdf" and ".../PicturePerfect.pdf".
//! Catalogue only; see library/lwppt/README.md.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "lwppt";

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::{NeedsData, NotTestable};
    let d = |id, name, group, status, rules, notes| StrategyDef { id, name, group, session: 0, dir: "Long/Short", status, needs_volume: false,
        needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None };
    vec![
        d("lwppt_cot_pattern", "Picture Perfect COT pattern (commercials with the 6/12-month trend)", "Section 2 · Commercials",
            NeedsData("Needs weekly CFTC COT data (commercial net position, 13/26/52-week index by sector) for the futures market."),
            &["F|\"The price of the commodity is in an up trend, higher than 6 months ago, and ideally 12 months ago. Then, we have the ideal pattern of price to COT activity\" (p.27)",
              "F|Sector look-backs: currencies/metals/cotton/grains 26 weeks, meats/softs/dollar/stocks/energies 13, bonds 52 (p.49)"],
            "A set-up; entries are chosen afterwards by the trader (proxy crossings, trend lines). No exit."),
        d("lwppt_proxies", "COT proxy index and Large Trader proxy index", "Section 2 · Proxies",
            NotTestable("The buy/sell zones are set by eye per market (\"I simply look back over the last three or four years to see which levels have been the best\", p.37); a proxy crossing \"does not mean to buy right now\" (p.60); no exit."),
            &["F|COT proxy = MA(open - close, 8 or 9) / MA(range, same) x 50 + 50 (p.37)", "F|Large trader proxy = MovingAvgMod(close - close 8 ago, n) / MA(true range, n) x 50 + 50; n = 8 weekly, 40 daily (p.44, 47)"], ""),
        d("lwppt_valuation_seasonals", "WillVal valuation, seasonals, Williams POIV A/D", "Sections 3-5",
            NeedsData("Needs commodity, gold, T-Bond and dollar futures and open interest; seasonals are a \"tie breaker\" (p.75); POIV needs open interest and is used for divergences only (p.103)."),
            &["F|Valuation = commodity/gold x 100, 2 vs 22 period averages, 3-year %R; highs ~75-85, lows ~15 (p.66-67)", "F|POIV = CumulativeSum(OI x (C - C1) / (true high - true low)) + OBV (p.100)"], ""),
        d("lwppt_blastoff_entries_exits", "Blast Off!, WillTrend, universal entry (Williams true trend reversal), qualified trend line, dollar stop, WillStop, target shooter", "Sections 6-8",
            NotTestable("Blast Off! is an indicator with no signal level or trade (p.105); WillTrend is shown on charts only (crossing rule and order not stated, p.112); trend reversals, trend lines and targets are drawn by eye; WillStop is used only in a \"Rock and Roll\" market (p.130)."),
            &["F|Blast Off! = MA(|close - open|, X) / MA(true range, X) x 100, X = 1 (p.106)", "F|WillTrend: highest/lowest mid price of the trend so far -/+ 2.236 x the 66-bar average range, never backing off (p.112)"], ""),
        d("lwppt_money_mgmt", "Money management and specialist trap", "Section 9 / bonus",
            NotTestable("Sizing formula, not a rule ((balance x risk %) / largest loss, p.137); the specialist trap has an entry but no exit (lwicw_failure_days)."), &[], ""),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    id.starts_with("lwppt_").then_some(("lwppt", "Picture Perfect Trading", if id.contains("cot_pattern") || id.contains("valuation") { "Needs data" } else { "Not testable" }))
}
pub fn params(_id: &str) -> Option<Vec<ParamDef>> { None }
pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }
