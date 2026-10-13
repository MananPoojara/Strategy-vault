//! Charles B. Schaap, ADXcellence: Power Trend Strategies (2006). Catalogue only: no rule in this source is both exact and complete. Source: library/csadx/README.md.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, dir, status, rules, notes| StrategyDef { id, name, group, session: 0, dir, status, rules, notes,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "csadx", claim: None };
    vec![
        d("csadx_adxodus", "ADXodus", "Section Three/Four", "Long/Short", Status::NotTestable("consolidation \"with a price pattern\", trendline breaks, \"at or near 25\" and price pivots are chart judgement; no exit beyond a stop under the pivot (p.172)"),
          &[],
          ""),
        d("csadx_adxtra", "ADXtra", "Section Three/Four", "Long/Short", Status::NotTestable("trendline breakout and DMI \"cross and hold\" judgement; no exit beyond a stop (p.180)"),
          &[],
          ""),
        d("csadx_adxpress", "ADXpress", "Section Three/Four", "Long/Short", Status::NotTestable("\"two to five bar pause near the 5 or 10 SMA\" is judgement and no exit or stop is given (p.188)"),
          &[],
          ""),
        d("csadx_adxtender", "ADXtender", "Section Three/Four", "Long/Short", Status::NotTestable("\"retraces near the 20 EMA\" judgement; exit only a stop and an optional discretionary target (p.198)"),
          &[],
          ""),
        d("csadx_adxplosive", "ADXplosive", "Section Three/Four", "Long/Short", Status::NotTestable("dual-timeframe trendline/triangle patterns; \"trail your stop\" unspecified (p.239)"),
          &[],
          ""),
        d("csadx_adxquisite", "ADXquisite", "Section Three/Four", "Long/Short", Status::NotTestable("Elliott ABC pullback identification is discretionary (p.250)"),
          &[],
          ""),
    ]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { catalogue().into_iter().find(|d| d.id == id).map(|d| (d.id, d.name, "")) }
pub fn params(_id: &str) -> Option<Vec<ParamDef>> { None }
pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }
