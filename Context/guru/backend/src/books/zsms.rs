//! Zach Smith, "Zach's Market Score: Trade the Stock Market and Profit!" (workshop workbook, zachsmarketscore.com,
//! about 2004; scanned). The Score is a Nasdaq-Composite timing oscillator and Zach's Zoom Factor (ZZF) ranks $1-$2
//! stocks; buys happen "on the day of our signal" and are held to "the next cash signal". The book gives The Score's
//! formula but says "A discussion of market timing is beyond the scope of this workshop" (pdf p.41), so the long/cash
//! signal rule is never stated, and stock selection ranks a whole-market universe. Sources: library/zsms/.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

const BOOK: &str = "zsms";
const NO_SIGNAL: &str = "the long / cash signal rule of The Score is never stated ('A discussion of market timing is beyond the scope of this workshop', printed p.39); trades start 'on the day of our signal' and end at 'the next cash signal' given only as dates";
const SCORE: &str = "F|The Score (printed p.38): Graph0 = (EMA(Close,5) - EMA(Close,10)) + (EMA(Close,5) - EMA(Close,17)) + (EMA(Close,10) - EMA(Close,17)); Graph1 = MA(Graph0,10) \"is used to develop entry-exit signals\"; ScoreStdDev = StDev(Score,10), Graph2 = abs(Graph1/ScoreStdDev); applied to the Nasdaq Composite (^IXIC) (printed p.39)";
const ZZF: &str = "F|Zach's Zoom Factor: ZZF = 1/(2.718^(Close - EMA(Close,34))) (printed p.13, p.35)";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, dir, why, rules, notes| StrategyDef {
        id, name, group: "Zach's Market Score workshop", session: 0, dir, status: Status::NotTestable(why), needs_volume: true,
        needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None,
    };
    vec![
        d("zsms_score_timing", "The Score market timing (Nasdaq Composite)", "Long", NO_SIGNAL, &[SCORE],
            "The formula page carries a notice that it is copyrighted and for personal trading only. Only the oscillator is given; no crossing or threshold rule."),
        d("zsms_zzf_zoomers", "ZZF Zoomers ($1-$2 stocks, top 10 by ZZF/Price)", "Long",
            "cross-sectional: rank all US stocks priced $1-$2 (avg volume > 100,000) by ZZF/Price and buy the top 10 (a whole-market universe, not a per-ticker rule); entry/exit dates come from The Score signals, whose rule is not stated",
            &[ZZF, "F|Scan: previous close >= 1 and <= 2, average daily volume last quarter >= 100,000 (printed p.9); AmiBroker: Close >= 1 AND Close <= 2 AND Volume > 100000 (p.48)",
              "F|Rank by ZZF/Price, take the top 10 (p.16-18); AmiBroker version adds ZZF/Close > 0.50 on the signal day (p.49)",
              "E|Buy on the day after the long signal; hold to the next cash signal (p.20-21, p.47)"],
            "Printed: 10/14/2002 long signal to 11/11/2002 cash signal: 10 stocks averaged +92% (p.21); 4/16/2003-5/19/2003: +43% (p.24, p.47); other backtests p.24-28; Amibroker variant 37.9% (p.57)."),
        d("zsms_zzf_risers", "ZZF Risers (Price/ZZF inversion)", "Long",
            "cross-sectional ranking of a whole-market universe; entry/exit tied to unstated Score signals",
            &[ZZF, "F|Rank by Price/ZZF instead (the 'exact INVERSION', printed p.23)", "E|Buy at the long signal, hold to the cash signal (p.24)"],
            "Printed: 4/16/2003-5/19/2003 reverse sort +30% (p.24)."),
        d("zsms_zzf_downers", "ZZF Downers (short, $5-$15 stocks)", "Short",
            "cross-sectional universe ($5-$15 stocks, ZZF/Close filter tuned per test: 0.90, then 2.0) with dates set by unstated Score signals",
            &[ZZF, "F|Close >= 5 AND Close <= 15 AND Volume > 100000 AND ZZF/Close > 0.90 (printed p.63), adjusted to > 2.0 (p.64)", "E|Short on the signal day, cover at the next signal (p.63-64)"],
            "Printed: 5/28/2002-8/12/2002, 14 stocks, overall gain 40% (p.64)."),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    if !id.starts_with("zsms_") { return None; }
    Some(("zsms", "Zach's Market Score (not testable)", ""))
}

pub fn params(_id: &str) -> Option<Vec<ParamDef>> { None }

pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }

#[cfg(test)]
mod tests {
    #[test]
    fn nothing_is_live() {
        assert!(super::catalogue().iter().all(|d| !matches!(d.status, crate::connors::Status::Live)));
    }
}
