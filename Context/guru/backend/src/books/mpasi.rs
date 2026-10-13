//! Martin J. Pring, "The All-Season Investor" (Wiley 1992; scan, 352 pdf pages; printed page = pdf page - 13). Book id `mpasi`.
//! Source and checks: library/mpasi/. Business-cycle asset allocation: six stages from bonds, stocks and
//! commodities, identified by economic/financial barometers or 12-month MA crossovers, with suggested
//! stock/bond/cash weights per stage. No rule set is both mechanical and runnable on vault data.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "mpasi";

macro_rules! d {
    ($id:expr, $name:expr, $group:expr, $st:expr, $rules:expr) => {
        StrategyDef { id: $id, name: $name, group: $group, session: 0, dir: "Long", status: $st,
            needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules: $rules,
            notes: "Read in the scan (library/mpasi/README.md).", claim: None }
    };
}

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    vec![
        d!("mpasi_six_stages_ma12", "Six-stage allocation from 12-month MA crossovers of bonds, stocks, commodities", "Ch.9-10, 12 (p.181-189, 205-206, 254-262)",
           NotTestable("needs monthly US long Treasury bond prices and the CRB Composite (not in the vault); 2 of the 8 above/below combinations are no stage; allocations are 'gradual', 'a range rather than a specific number' (p.182) and can depart from the targets ('The Outer Ranges', p.261-262)"),
           &["F|\"technicians use moving average crossovers rather than reversals\"; \"12 months is one of the most consistently reliable\" (p.185-186)",
             "F|Stage I bonds bullish, stocks and commodities bearish; II bonds and stocks bullish; III all bullish; IV bonds bearish; V commodities bullish only; VI all bearish (Fig 10.1, p.206)",
             "E|Stage weights stocks/bonds/cash: I 30/50/20 ('recommended 30 percent equity exposure', p.256), II 65/30/5, III 75/20/5, IV 75/15/10, V 30/15/55 (cash \"raised to 55 percent\", p.260), VI 20/30/50 (Figs 12.2-12.6, p.257-261)"]),
        d!("mpasi_barometers", "Bond, Stock and Inflation Barometers", "Ch.10, Appendices A-C (p.204-209, 281-307)",
           NotTestable("economic and money data (housing starts, vendor deliveries, LEI diffusion, M2, CRB spot raw materials, commercial paper and AAA yields, discount rate) are not in the vault; several weights are unspecified ('a special weighting', p.295)"),
           &["F|\"It starts off with 100 percent of its components in bullish mode. As a specific indicator goes negative its contribution is removed. Readings of 50 percent or lower signal periods of risk.\" (Stock Barometer, p.290)",
             "F|\"One point is subtracted when the S&P falls below its nine-month MA\" (p.299)"]),
        d!("mpasi_conservative_aggressive", "Conservative and aggressive allocation policies", "Ch.12 (p.263-273)",
           NotTestable("same stage model with different asset classes (intermediate bonds, utilities, convertibles, gold shares); the allocations are 'guidelines for an average investor' (p.264)"),
           &["E|e.g. conservative Stage I: stocks 10%, bonds 40%, intermediate bonds 20%, cash 30% (Fig 12.7, p.264)"]),
        d!("mpasi_gold", "Gold and the business cycle", "Ch.11 (p.231-245)",
           NotTestable("background factors and judgement; no trading rule"), &["E|Narrative only"]),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    id.starts_with("mpasi_").then_some(("mpasi_nt", "Pring All-Season Investor (not testable)", "-"))
}
pub fn params(_id: &str) -> Option<Vec<ParamDef>> { None }
pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }
