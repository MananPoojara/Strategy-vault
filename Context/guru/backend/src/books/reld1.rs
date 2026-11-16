//! Lane D1 (16 Nov 2026, R4 #339/#340): held-for-intraday methods converted to the held SPX/SPY/NIFTY minute bars or to a
//! stated daily-bar reading. The cards themselves live in their own book modules (clomw.rs ...); this module only holds the
//! lane's reference-dump test (authors/relax-339/D1/w/d1_ref.py, compare d1_cmp.py). No catalogue rows of its own.
use crate::connors::{Ctx, ParamDef, Params, StrategyDef, Trade};

/// Minute-path cards of the lane: SPX only (S&P points).
pub const SPX_IDS: &[&str] = &["clomw_early_range_breakout", "lpgap_fade_spx"];
/// Minute-path cards of the lane on all four minute tickers (no fixed amount; a tick = #57).
pub const ALL_IDS: &[&str] = &[];
pub fn catalogue() -> Vec<StrategyDef> { vec![] }
pub fn family_of(_id: &str) -> Option<(&'static str, &'static str, &'static str)> { None }
pub fn params(_id: &str) -> Option<Vec<ParamDef>> { None }
pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }

#[cfg(test)]
mod tests {
    use super::*;
    /// The reference comparison (run by hand): VAULT_US=<us_data.csv> VAULT_IDX=<idx_data.csv> VAULT_M1_DIR=<data/intraday>
    /// D1_OUT=<csv> cargo test --release d1_dump -- --ignored; then authors/relax-339/D1/w/d1_cmp.py <csv>.
    #[test]
    #[ignore]
    fn d1_dump() {
        use std::io::Write;
        let mut f = std::io::BufWriter::new(std::fs::File::create(std::env::var("D1_OUT").unwrap()).unwrap());
        for (file, t) in [("VAULT_US", "SPY"), ("VAULT_US", "SPX"), ("VAULT_IDX", "NIFTY 50"), ("VAULT_IDX", "NIFTYBANK")] {
            let rows = crate::load_csv(&std::env::var(file).unwrap()).unwrap().remove(t).unwrap();
            let mut x = Ctx::new(&rows, &rows, &[]);
            x.minutes = Some(Box::leak(Box::new(crate::intraday::Minutes::load(&std::env::var("VAULT_M1_DIR").unwrap(), t).unwrap())));
            let from = rows.partition_point(|r| r.date.as_str() < x.minutes.unwrap().first());
            let ids = std::env::var("D1_IDS").map(|s| s.split(',').map(str::to_string).collect::<Vec<_>>()).unwrap_or_else(|_| SPX_IDS.iter().map(|s| s.to_string()).collect());
            for id in ids {
                if SPX_IDS.contains(&id.as_str()) && t != "SPX" { continue; }
                for z in crate::books::run(&id, &x, from, rows.len() - 1, &Params::new()).unwrap() {
                    writeln!(f, "{id},{t},{},{},{},{},{},{},{}", z.dir, rows[z.legs[0].i].date, z.legs[0].px, rows[z.exit_i].date, z.exit_px, z.reason, z.open).unwrap();
                }
            }
        }
    }
}
