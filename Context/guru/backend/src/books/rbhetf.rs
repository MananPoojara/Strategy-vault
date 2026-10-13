//! Robert B. Hanna, *The Hanna ETF Money Flow System* (TradingMarkets / Connors Group, January 2005). Book id `rbhetf`.
//! Source: research drive "Trading Knowledge II/@TradingMarkets.com & TradersLibrary.com/Robert B.Hanna - The Hanna ETF
//! Money Flow System/Hanna ETF 15th January 2005.pdf" (text PDF, read in place; copy refused - COPY PENDING, see
//! library/rbhetf/README.md). Page refs are the manual's printed pages.
//! The four scenarios are spread trades between eight indices chosen by their ReIS rank each day, scaled in over up to
//! three VXO triggers. The VXO exists in extra_data (VXO_OHLC), but the system also needs the PHLX Semiconductor,
//! AMEX Pharmaceutical, AMEX Biotechnology, Russell 2000 and S&P 400 index histories from 1995 and a multi-instrument
//! spread engine that picks the instrument by cross-sectional rank; the vault runs one instrument at a time.
use crate::connors::{claim, Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "rbhetf";

const GAP: &str = "needs daily closes of all eight indices of p.14 (S&P 500, Russell 2000, Nasdaq 100, DJIA, PHLX Semiconductor SOX, AMEX Pharmaceutical DRG, AMEX Biotechnology BTK, S&P 400 Midcap; 1995-) to rank ReIS each day, plus a multi-leg spread engine; the vault has SPX/NDX/DJI only (IWM/SMH/MDY ETFs start 1993-2000, no DRG/BTK) and runs one instrument at a time. VXO OHLC itself is available (extra_data/us/VXO_OHLC, 1986-2021)";
const NOTE: &str = "ReIS (p.16-17): Measured Period starts at 3 on the day the S&P 500's close vs its close 3 days earlier changes sign (upleg/downleg) and grows by 1 each day the leg continues; ReIS = % change of each index over the Measured Period. Scale-in: 1/3 of the portfolio per valid VXO trigger, up to three (p.24-25). Book test: indices (not ETFs), 1/1/1996-6/14/2004: 349 triggers, 65.9% profitable, average +1.24%, max drawdown -8.9%, 17.8% compound annual gain 1/1/95-6/14/04 (p.4, 47-50).";

macro_rules! nd {
    ($id:expr, $name:expr, $dir:expr, $rules:expr, $claim:expr) => {
        StrategyDef { id: $id, name: $name, group: "Part 2 · System trading rules (p.19-25)", session: 0, dir: $dir, status: Status::NeedsData(GAP),
            needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules: $rules, notes: NOTE, claim: $claim }
    };
}

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        nd!("rbhetf_s1", "Scenario #1: low VXO stretch, S&P 500 below its 200-day SMA", "Long/Short",
            &["F|\"If S&P 500 is below its 200 day simple moving average\" (p.19)",
              "E|\"The VXO must close at least 10% below its 10-day moving average or trade at least 12% below its 10-day moving average intraday.\"",
              "E|\"Short one unit of the SPY's and buy one unit of ETF with the lowest ReIS rating.\"",
              "X|\"Exit both positions when the VXO closes above its 10-day moving average.\""],
            claim("43", "69.77%", "+1.26%", "", "Scenario #1, 1/1/1996-6/14/2004, 3-tier scale-in (p.51)")),
        nd!("rbhetf_s2", "Scenario #2: high VXO stretch, S&P 500 below its 200-day SMA", "Long/Short",
            &["F|\"If S&P 500 is below its 200 day simple moving average\" (p.19)",
              "E|\"The VXO must close at least 10% above its 10-day moving average or trade at least 12% above its 10-day moving average intraday.\"",
              "E|\"Short one unit of the ETF with the highest ReIS rating and buy one unit of the ETF with the lowest ReIS rating.\"",
              "X|\"Exit when VXO closes below its 10-day moving average.\""],
            claim("80", "57.50%", "+0.95%", "", "Scenario #2, 1/1/1996-6/14/2004 (p.51)")),
        nd!("rbhetf_s3", "Scenario #3: low VXO stretch, S&P 500 above its 200-day SMA", "Long/Short",
            &["F|\"If S&P 500 is above its 200 day simple moving average\" (p.19)",
              "E|\"The VXO must close at least 10% below its 10-day moving average or trade at least 12% below its 10-day moving average intraday.\"",
              "E|\"Short one unit of the SPY's and buy one unit of the ETF with the lowest ReIS rating.\"",
              "X|\"Exit both positions when the VXO closes above its 10-day moving average.\""],
            claim("84", "55.95%", "+0.34%", "", "Scenario #3, 1/1/1996-6/14/2004 (p.52)")),
        nd!("rbhetf_s4", "Scenario #4: high VXO stretch, S&P 500 above its 200-day SMA", "Long",
            &["F|\"If S&P 500 is above its 200 day simple moving average\" (p.19)",
              "E|\"The VXO must close at least 10% above its 10-day moving average or trade at least 12% above its 10-day moving average intraday.\"",
              "E|\"Buy 2 units of the ETF with the lowest ReIS rating.\"",
              "X|\"Exit when the VXO closes below its 10-day moving average.\""],
            claim("142", "75.35%", "+1.92%", "", "Scenario #4, 1/1/1996-6/14/2004 (p.52)")),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "rbhetf_s1" => ("rbhetf_money_flow", "Hanna ETF Money Flow", "Scenario 1"),
        "rbhetf_s2" => ("rbhetf_money_flow", "Hanna ETF Money Flow", "Scenario 2"),
        "rbhetf_s3" => ("rbhetf_money_flow", "Hanna ETF Money Flow", "Scenario 3"),
        "rbhetf_s4" => ("rbhetf_money_flow", "Hanna ETF Money Flow", "Scenario 4"),
        _ => return None,
    })
}
pub fn params(_id: &str) -> Option<Vec<ParamDef>> { None }
pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }
