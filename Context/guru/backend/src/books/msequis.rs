//! Equis sample System Tester records "Equis - ..." at the start of MS11PRFT.DTA (research share USB HDD 1/MetaStock/MetaStock 11
//! Indicators, Explorers, System Testers & Experts for Metastock 10/; formulas read as text: authors/titles-1030/K15w/src/ms11.txt).
//! Book id `msequis`. Lane K15 (titles loop, 2 Nov 2026), to-code row 297. Run on the shared MetaStock engine `msws` as in nkms3:
//! MetaStock tester defaults (trade at the close, no delay), an opposite Enter reverses (MetaStock 7.2 manual, The System Editor
//! Dialog). Independent check authors/titles-1030/K15w/k15_ref.py. HELD from the same file: "Equis - Directional Movement -
//! Expert System" (its Close Short formula is not readable in the record), the "w/Opt" records (periods from the optimiser).
use crate::books::msws::{self, Book, Eng, Sys, EXEC_DEFAULT};
use crate::connors::{Ctx, ParamDef, Params, StrategyDef, Trade};
use crate::connors::Status::Live;
use std::sync::OnceLock;

pub const BOOK: &str = "msequis";
/// Admitted ids (authors/titles-1030/K15w/k15_ref.py (header)).
pub const REVIEWED: &[&str] = &["msequis_bb", "msequis_cci", "msequis_stoch"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const CONV: &str = "MetaStock System Tester defaults (the records store no fill or stop settings): signals at the close, filled at that close; a Close formula is checked from the bar after entry; MetaStock System Tester reverses on an opposite entry (7.2 manual, Order of Evaluation; Pro 8.0 p.332 Portfolio Bias Single): an opposite Enter closes the position and opens the other side at the close (Portfolio Bias Single; MS11PRFT.DTA is a MetaStock 11 file and versions 9-11 are not documented in the share; p.reverse = 0 turns that off); after an exit no same-direction entry on that bar. Indicators: MetaStock definitions (module msws; Bollinger bands with the population standard deviation, Stoch(5,3) = 100 x Sum(C - LLV(L,5), 3) / Sum(HHV(H,5) - LLV(L,5), 3), CCI = Lambert's on (H+L+C)/3). Equis sample records name no market; daily indices, ETFs and US/India stocks are a declared application; the vault's standard costs.";
const GROUP: &str = "Equis MetaStock 11 sample systems (MS11PRFT.DTA)";
const SYS: &[Sys] = &[
    Sys { id: "msequis_bb", name: "Equis - Bollinger Bands: long when the close crosses back above the lower band, out at the 1-SD upper band; short mirrored", group: GROUP, family: ("msequis_bb", "Equis Bollinger Bands sample"), page: "MS11PRFT.DTA, record 'Equis - Bollinger Bands'", status: Live,
        f: [Some("Cross(CLOSE,  BBandBot(CLOSE, 20, SIMPLE, 2))"), Some("Cross(CLOSE,  BBandTop(CLOSE, 20, SIMPLE, 1))"), Some("Cross( BBandTop(CLOSE, 20, SIMPLE, 2), CLOSE)"), Some("Cross( BBandBot(CLOSE, 20, SIMPLE, 1), CLOSE)")],
        exec: EXEC_DEFAULT, notes: "[Declared readings] The record's description says positions are closed 'when the close crosses above/below the opposite band'; its stored Close formulas use the 1-standard-deviation bands (BBandTop/BBandBot(CLOSE, 20, SIMPLE, 1)), and the formulas are run as stored." },
    Sys { id: "msequis_cci", name: "Equis - CCI +100/-100 Crossover: long when CCI(14) crosses above -100, short when it crosses below +100", group: GROUP, family: ("msequis_cci", "Equis CCI +100/-100 sample"), page: "MS11PRFT.DTA, record 'Equis - CCI +100/-100 Crossover'", status: Live,
        f: [Some("Cross( CCI(14),-100)"), None, Some("Cross( 100, CCI(14))"), None],
        exec: EXEC_DEFAULT, notes: "[Declared readings] The record has no Close formulas ('Enter long/short when the 14-period CCI crosses above/below -100/+100'): a position ends only when the opposite Enter reverses it." },
    Sys { id: "msequis_stoch", name: "Equis - Stochastic 20/80: long when Stoch(5,3) rises above 20, short when it falls through 80; out on the opposite zone", group: GROUP, family: ("msequis_stoch", "Equis Stochastic 20/80 sample"), page: "MS11PRFT.DTA, record 'Equis - Stochastic 20/80 w/Optimization'", status: Live,
        f: [Some("Ref( Stoch(5,3), -1) <= 20 AND Stoch(5,3) > 20"), Some("Stoch(5,3) < 20"), Some("Ref(Stoch(5,3), -1) >= 80 AND Stoch(5,3) < 80"), Some("Stoch(5,3) > 80")],
        exec: EXEC_DEFAULT, notes: "[Declared readings] The record's levels are optimiser variables OPT1/OPT2; 20 and 80 are the values its name and description give ('rises above OPT1 (e.g., 20) ... falls through OPT2 (e.g., 80)'); the other 'w/Opt' records give no values and are held." },
];
static EQ: Book = Book { id: BOOK, sys: SYS, fml: &[], conv: CONV, cat: OnceLock::new(), par: OnceLock::new() };

pub fn catalogue() -> Vec<StrategyDef> {
    EQ.catalogue().into_iter().map(|mut d| {
        if d.id == "msequis_cci" { d.rules = &["E|Enter Long: Cross( CCI(14),-100)", "E|Enter Short: Cross( 100, CCI(14))", "X|No Close formulas in the record: the opposite Enter reverses the position (MetaStock 7.2 manual, The System Editor Dialog)"]; }
        d
    }).collect()
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { EQ.family_of(id) }
pub fn params(id: &str) -> Option<Vec<ParamDef>> { EQ.params(id) }
pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let s = SYS.iter().find(|s| s.id == id)?;
    if from >= to || to >= x.r.len() { return None; }
    let side = p.get("side").copied().unwrap_or(0.).round() as i32;
    let raw: Vec<&str> = s.f.iter().flatten().copied().collect();
    let map: Vec<(f64, f64)> = msws::constants(&raw).into_iter().map(|c| (c, p.get(&msws::const_key(c)).copied().unwrap_or(c))).collect();
    let ex: Vec<Option<String>> = s.f.iter().map(|f| f.map(crate::books::mslib::expand)).collect();
    let f: [Option<&str>; 4] = [ex[0].as_deref(), ex[1].as_deref(), ex[2].as_deref(), ex[3].as_deref()];
    let fl: &dyn Fn(&str) -> Option<String> = &|_| None;
    let sig = msws::signals(&mut Eng::new(x, fl), f, &map, side).ok()?;
    Some(msws::simulate_conv(x, from, to, &sig, s.exec, msws::reverse_on(p)))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn equis_samples_trade() {
        let r = super::super::ctm::tests::walk(3000);
        let x = Ctx::new(&r, &r, &[]);
        for id in REVIEWED {
            let t = run(id, &x, 0, r.len() - 1, &Params::new()).unwrap();
            assert!(t.len() > 5 && t.iter().any(|z| z.dir > 0.) && t.iter().any(|z| z.dir < 0.), "{id}");
            assert!(family_of(id).is_some() && params(id).is_some());
        }
    }
}

// Instrument admission: Equis MetaStock 11 samples 2026-11-02 (titles lane K15).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "msequis" && reviewed(def.id),
    label: "Equis's sample System Tester records name no market; runs on indices, ETFs and single stocks (US and India) are a declared application, not the author's test.",
    basis: "MS11PRFT.DTA (MetaStock 11 pack, research share USB HDD 1/MetaStock), records 'Equis - Bollinger Bands', 'Equis - CCI +100/-100 Crossover', 'Equis - Stochastic 20/80 w/Optimization'; MetaStock 7.2 manual, The System Editor Dialog; independent check authors/titles-1030/K15w/k15_ref.py",
};
