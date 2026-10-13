//! Greg Cook, Ph.D., *PLAD* (Pivot Low Advance-Decline) S&P 500 timing system (Global Investment Analysts / SECTOR *WIZ*, 1996).
//! Source: research drive TradeStation Systems & Indicators/Greg Cook - PLAD System/ (PLAD.TXT, README.TXT, PLAD.ELA);
//! copies in Books/Greg Cook/PLAD/. Book id "gcplad". Notes: library/gcplad/README.md.
use crate::connors::{claim, Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef {
        id: "gcplad_plad", name: "PLAD: pivot-low NYSE advance-decline S&P 500 timing", group: "PLAD.TXT / README.TXT", session: 0, dir: "Long/Short",
        status: Status::NotTestable("the rules exist only inside the compressed TradeStation archive PLAD.ELA (Omega ORCMP format, not readable as text); the text files give only data setup (Data1 S&P 500, Data2 NYSE advancing issues, Data3 NYSE declining issues, settings 6/6) and results. It would also need daily NYSE advancing/declining issues"),
        needs_volume: false, needs_vix: false, needs_series: &[], book: "gcplad",
        rules: &[
            "F|\"the system is based on NYSE advance-decline data\"; \"All you need is daily data for advancing and declining issues on the NYSE and the S&P 500 index\" (README.TXT)",
            "F|\"SP500 ADVAN DECLI ... PLAD 6/6\"; \"There are no system settings to enter for the *PLAD* system. Just use the settings (6/6) that come with the system.\" (README.TXT)",
            "E|Entry/exit formulas: not in any readable file (PLAD.ELA only)",
        ],
        notes: "PLAD.TXT: SPX daily 01/02/86-06/28/96, 18 trades (9 long all winners, 9 short 6 winners), 83% profitable, net $7,401.20 (one S&P point = $10 in the report: 740 points), stop-and-reverse at the close, first trade Buy 02/28/86 at 226.92, last Buy 05/09/96 at 645.44. Can be compared once the ELA rules and NYSE A/D data are available.",
        claim: claim("18", "83%", "$411.18", "long 224 bars, short 62 bars (winners)", "SPX daily 01/02/86 - 06/28/96 (PLAD.TXT)"),
    }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == "gcplad_plad").then_some(("gcplad", "Greg Cook PLAD (not testable)", "-"))
}
pub fn params(_id: &str) -> Option<Vec<ParamDef>> { None }
pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }
