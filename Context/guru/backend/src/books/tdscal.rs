//! "DeMark Scalper System" (_IXOG-TDScalper), Mark A. Simms / Index Options Group, Inc. (c.2000), TradeStation open code.
//! Book id `tdscal`. Source: research drive `USB HDD 1/TradeStation Systems & Indicators/_Misc/_Tom Demark Scalper System
//! for TS, Excel/OpenCode/TDScalper-signal.txt, TDC.SEQ.SETUP2.txt, TDScalper-indicator.txt` (+ `tdscalper-equity.gif`,
//! `Report EURUSD 5 Min.mht`). Notes: library/tdscal/README.md.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef {
        id: "tdscal_scalper", name: "TD Scalper (Sequential setup flip, 5-minute bars)", group: "TradeStation open code", session: 0,
        dir: "Long/Short",
        status: Status::NeedsData("5-minute S&P 500 futures bars (the code's own test: \"S&P 500 - Jan96->Jun00, 5 min Bars\"; second report EUR/USD 5 min). Its exits are counted in bars (MAE window 55-85 bars, quick-profit 35-70 bars, swing bail-out after 100 bars) and it forbids an exit on the entry day, so it is an intraday-bar system; daily bars would change every exit"),
        needs_volume: true, needs_vix: false, needs_series: &[], book: "tdscal",
        rules: &[
            "F|_IXTDC.SEQ.Setup(4, 9, UseEquals=false, Close, RecountRev=true): count of closes below (above) the close 4 bars earlier; with RecountRev a count restarts only after an opposite bar; returns -count (buy) / +count (sell)",
            "F|DMCond = -1 when the count <= -9, +1 when >= 9, else 0; a flip out of -1 sets GoLong, out of +1 sets GoShort",
            "F|Volume filter on the first trigger bar: VolOsc = Average(VolumeOsc(4,12),3); cancel GoLong if VolOsc and VolOsc[4] are both < 0; cancel GoShort if both > -1",
            "E|Within MaxSignalWait = 15 bars: Buy at Highest(High,3) - 0.2% of Close stop (Sell at Lowest(Low,3) - 0.2% of Close stop); on the 15th bar go in at market (\"Alley-Oop\")",
            "X|Opposite signal after 15 bars and not on the entry date: exit next bar at market",
            "X|Key-reversal exit (needs VolOsc > 3); MAE exit between bars 55 and 85 when profit < -0.75% (swing exception); swing bail-out after 100 bars; quick-profit stop between bars 35 and 70 when profit > 1.5%; profit stop when profit > 3.5% long / 2.5% short",
            "X|As printed, the quick-profit and profit stops sit under `If MktPosit > 0`, so their short branches can never run, and the short MAE exit is written `ExitLong` - a literal port must keep these"],
        notes: "Built on DeMark's TD Sequential setup by Mark Simms (Index Options Group), not by DeMark himself. Published results: 5-minute S&P 500 futures 1/96-6/00 equity curve (~230 trades, peak Oct 98, \"did not adapt to a BIG change\"); EUR/USD 5-min 23.10.2008-31.12.2008: 57 trades, 45.61% profitable, profit factor 1.62, avg 208.8 bars per trade. The vault has no intraday bars.",
        claim: None,
    }]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == "tdscal_scalper").then_some(("tdscal", "TD Scalper", "5-minute (needs data)"))
}
pub fn params(_: &str) -> Option<Vec<ParamDef>> { None }
pub fn run(_: &str, _: &Ctx, _: usize, _: usize, _: &Params) -> Option<Vec<Trade>> { None }
