//! YT Strategies · 30 Min Breakout: the team's AmiBroker AFLs in "Old Topics/30 Min Breakout"
//! (30 MIN Breakout ANALYSIS/FINAL_06.12.2021: INTRADAY, POSITIONAL and TEMP, 23 AFLs). None is coded: every one is
//! written for 30-minute bars ("Timeframe: intraday 30 mins") and the engine has daily bars only.
//!
//! The rules read the day's first (and, in the VER 1 / VER 2 "WITH LAST BAR" AFLs, also the previous day's last)
//! 30-minute candle: `STARTBAR = Day() != Ref(Day(), -1)`, `ENDBAR = Day() != Ref(Day(), 1)`,
//! `BARHIGH = ValueWhen(STARTBAR, H)` (VER 1: Max of first / last bar highs and Min of their lows; VER 2: Max of
//! both), buy / cover when a 30-minute close breaks above it and sell / short below the low, filtered by a daily
//! SuperTrend(5, 4) computed with `TimeFrameSet(86400)`; the INTRADAY AFLs also square off on the day's last bar.
//! On daily bars every bar is both STARTBAR and ENDBAR, so the breakout level is the bar's own high / low (the
//! normal AFLs could never trade: C > H is never true) or the previous day's (VER 1 / VER 2 positional): a
//! different strategy from the one written, so nothing is run on daily data.
//!
//! Not coded (all: needs intraday 30-minute bars, not available):
//! - POSITIONAL/NORMAL: 30 MIN BREAKOUT POSITIONAL WITH DAILY FLT (+ FOR BULL MKT / FOR BEAR MKT (L,S,L-S));
//! - POSITIONAL/VERSION1 and VERSION2: 30 MIN BREAKOUT WITH LAST BAR WITH DAILY FLT VER 1 / VER 2 FOR POSITIONAL
//!   (+ FOR BULL / BEAR MKT);
//! - INTRADAY/NORMAL, VERSION1, VERSION2: the same three rule sets with the end-of-day square-off and dead-stock exit
//!   (+ FOR BULL / BEAR MKT);
//! - TEMP: earlier drafts of the positional NORMAL / VER 1 / VER 2 AFLs (filter not lagged, no filter exits).
#![allow(dead_code)]
use crate::connors::{Ctx, ParamDef, Params, StrategyDef, Trade};

pub const BOOK: &str = "yt_30m";
pub const IDS: [&str; 0] = [];

pub fn catalogue() -> Vec<StrategyDef> {
    vec![]
}
pub fn params(_id: &str) -> Option<Vec<ParamDef>> {
    None
}
pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> {
    None
}
pub fn family_of(_id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nothing_is_coded_without_intraday_bars() {
        assert!(catalogue().is_empty() && IDS.is_empty());
        assert!(params("yt30m_breakout").is_none() && family_of("yt30m_breakout").is_none());
    }
}
