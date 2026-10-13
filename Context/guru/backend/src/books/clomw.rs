//! John F. Clayburg, "Parallel Functions in Self Adaptive Systems" (Omega World presentation, c. 2000). Book id
//! `clomw`. Source: research drive ".../John Clayburg - Systems & Indicators for TS + Manual (clayburg.com)/
//! OmegaWorldAbstract.doc" (also under "John Clayburg - Parallel Functions in Self Adaptive Systems/"); copy in
//! Books/John Clayburg/. Transcription: library/clomw/.
//!
//! The paper's base system is complete EasyLanguage but trades the break of the intraday range fixed `Delay`
//! minutes after the session open, with an intraday limit target and stop; it needs intraday bars.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "clomw";

const GAP: &str = "intraday bars of the session (the high/low reached Delay=45 minutes after the open, then the intrabar order of the entry stop, the Tgt=7 limit, the Stp=4 stop and the session close). Daily OHLC cannot tell which of these was hit first, and no intraday history is in the vault";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "clomw_early_range_breakout", name: "Early-range breakout base system (Omega World)", group: "Base system · Parallel functions paper",
        session: 0, dir: "Long/Short", status: Status::NeedsData(GAP), needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
        rules: &[
            "F|NuHi/NuLo = the day's running high and low; at T = CalcTime(Sess1StartTime, Delay) set Bpt = NuHi + .1, Spt = NuLo - .1 (code inputs Delay(45), Tgt(7), Stp(4)).",
            "E|If TradesToday(d) = 0: BUY Bpt stop; SELL Spt stop (one trade per day).",
            "X|Long: ExitLong entryprice + Tgt limit; ExitLong entryprice - Stp stop. Short: ExitShort entryprice - Tgt limit; ExitShort entryprice + Stp stop.",
            "X|The paper's parallel function closes an open trade at the session close (T = Sess1EndTime, value C - Bpt).",
        ],
        notes: "The paper prints two input sets: the code Delay(45), Tgt(7), Stp(4) with Bpt = NuHi + .1, and an explanatory copy Delay(30), Tgt(2), Stp(4) without the .1 tick; the first listing is the system. Futures (currency / S&P) day-trading example; no results are printed. The 'parallel function' self-optimisation is a programming technique, not a separate rule set.",
        claim: None }]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == "clomw_early_range_breakout").then_some(("clomw_orb", "Clayburg early-range breakout", "Omega World base system · data missing"))
}
pub fn params(_id: &str) -> Option<Vec<ParamDef>> { None }
pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }
