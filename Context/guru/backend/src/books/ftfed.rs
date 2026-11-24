//! Douglas S. Roberts, "Follow the Fed to Investment Success" (Wiley, 2008), Chapter 11 "Follow the Fed: The Easy Strategy for True Wealth", the Standard Strategy (pp.115-131).
//! Research share Trading Books/Douglas Roberts - Follow the Fed to Investment Success (text PDF, read in full). Book id `ftfed`. Lane K3 (25 Nov 2026, R4 #398 rows R160/R161).
//! The rule is evaluated on the rotation account (books::rotation, "ftfed_small_large"); this module is its catalogue entry. Independent check authors/conv-1123/k3w/ftfed/ftfed_ref.py.
use crate::connors::{ParamDef, Params, Status, StrategyDef, Trade, Ctx};

pub const BOOK: &str = "ftfed";
pub const ID: &str = "ftfed_small_large";

const NOTE: &str = "[Declared adaptation, R4 #339/#340/#400; 4 labelled readings; unverified] The Standard Strategy (Ch.11 pp.115-131) holds large stocks or small stocks and switches monthly: T-Bill = the month's 30-day Treasury bill return, T-Bill-12m = the last 12 monthly bill returns compounded, Inflation-12m = the 12-month change of the CPI; when T-Bill-12m is below Inflation-12m (negative real short rate) hold small stocks, at or above hold large stocks; the book's 1928-2006 test has about 35 switches (one per 2.26 years), no figure here is compared (the book's total-return indexes and window differ). Held series: SPY (large; the ETF the vault uses for the S&P 500, from 1993) and IWM (small; the Russell 2000 ETF, from 2000-05), price only, FRED DTB3 (3-month bill, secondary market; the book's 30-day bill is not held: stand-in, labelled, not counted) and the CPI 12-month change with its release date (CPIAUCNS_yoy_release). Unstated choices, numbered one per item (R4 #400; 4): (1) a month's bill return = the last DTB3 print of that month / 12; (2) the CPI value used for month t is the latest one released by the last close of month t+1 (release dates honoured); (3) the decision at that close is executed at the first open of month t+2 (the book gives no fill price); (4) a month end with no bill or CPI value is skipped and the earlier holding stays. Declared adaptations, labelled apart and not counted: SPY for the large-stock basket, IWM (from 2000-05) for the small-stock basket, the 3-month bill DTB3 for the 30-day bill, price returns for total returns; the account therefore starts at 2000-06. Near-twin live card: maetf_small_large (Appel, yearly IWM/SPY switch on the prior year's return; a different signal, same two funds). No transaction costs beyond the engine cost setting.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: ID, name: "Follow the Fed Standard Strategy (small stocks when the real bill rate is negative, else large)", group: "Roberts · Follow the Fed", session: 0, dir: "Long",
        status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &["DTB3", "CPIAUCNS_yoy_release"], book: BOOK,
        rules: &[
            "F|T-Bill-12m = the compounded last 12 monthly T-bill returns; Inflation-12m = the compounded last 12 monthly CPI changes (Table 11.1 grid, pp.117-120)",
            "E|If T-Bill-12m is below Inflation-12m the month is Small, at or above it Large (Standard Strategy, pp.115-131); hold the large-cap or small-cap basket until the signal changes",
            "X|The month t signal is acted on two months later (the revision lag of the inflation figure); about 35 switches in 1928-2006 (p.116)",
        ], notes: NOTE, claim: None }]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { (id == ID).then_some(("ftfed", "Follow the Fed", "Standard Strategy")) }
pub fn params(_id: &str) -> Option<Vec<ParamDef>> { None }
pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }

#[cfg(test)]
mod tests {
    use super::*;
    /// Reference dump on the real files (by hand): VAULT_US=<us_data.csv> FTFED_EXTRA=<data/extra_data> FTFED_OUT=<tsv> cargo test --release ftfed_dump -- --ignored
    #[test]
    #[ignore]
    fn ftfed_dump() {
        let (Ok(us), Ok(dir), Ok(out)) = (std::env::var("VAULT_US"), std::env::var("FTFED_EXTRA"), std::env::var("FTFED_OUT")) else { return };
        let data: &'static _ = Box::leak(Box::new(crate::load_csv(&us).unwrap()));
        let _ = crate::books::rotation::PRELOADED.set(data);
        let rows = &data["SPY"];
        let extra = crate::load_extra(&dir);
        let mut x = Ctx::new(rows, rows, &[]);
        for k in ["DTB3", "CPIAUCNS_yoy_release"] { x.extra.insert(k.into(), crate::release::column(k, rows.iter().map(|r| r.date.as_str()), extra.get(&("*".to_string(), k.to_string())).unwrap())); }
        let a = crate::books::rotation::account(ID, &x, 0, rows.len() - 1, 0.0, &Params::new(), Some(data)).unwrap().unwrap();
        let mut s = String::new();
        for t in &a.trades { s += &format!("{}\t{}\t{}\t{}\t{}\n", rows[t.legs[0].i].date, t.legs[0].px, rows[t.exit_i].date, t.exit_px, t.reason); }
        std::fs::write(out, s).unwrap();
    }
    #[test]
    fn catalogue_row_present_and_rotation_routed() {
        assert!(crate::books::rotation::is_rotation(ID));
        assert_eq!(catalogue()[0].needs_series, &["DTB3", "CPIAUCNS_yoy_release"]);
    }
}
