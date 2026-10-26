//! Karl Dittmann: Forex Never Lose Trade (`kdfnlt`), Forex Unknown Secret (`kdfus`) and The Simplest Forex Pips System (`kdsfp`).
//! Reading: claude-continuation-20261020/authors/dittmann/agents/. Catalogue only (intraday forex, pip-sized stops and targets).
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub fn catalogue() -> Vec<StrategyDef> {
    let nt = |id, name, group, book, why, rules| StrategyDef { id, name, group, session: 0, dir: "Long/Short", status: Status::NotTestable(why),
        needs_volume: false, needs_vix: false, needs_series: &[], book, rules, notes: "Held after the 2026-10-24 reading (authors/dittmann/agents).", claim: None };
    vec![
        nt("kdfnlt_range79", "7-9 am range breakout", "Forex Never Lose Trade", "kdfnlt",
           "intraday GBP/USD session rule (15-minute bars, 7:00-9:00 am New York range) with a 15-pip stop and a 20-50 pip target or 15-pip trailing stop; the vault has no forex bars and a pip has no index equivalent",
           &["E|buy stop at the high and sell stop at the low of the 7:00-9:00 am bars (p.4)", "X|stop 15 pips; target 20-50 pips or a 15-pip trailing stop (p.4)"]),
        nt("kdfus_startbar", "Start-bar breakout reversal", "Forex Unknown Secret", "kdfus",
           "30-minute forex bars, a 15-pip minimum start-bar range and 1-pip entry/stop offsets; the trend and risk-reward filters are judgement",
           &["E|after the next bar breaks the start bar's low and crosses back over its close, buy 1 pip above (p.4)", "X|stop 1 pip below the entry bar's low; exit at the entry bar's close (p.4)"]),
        nt("kdsfp_inside", "Simplest Forex Pips (inside-close candle with trend filter)", "The Simplest Forex Pips System", "kdsfp",
           "the exit is a 10-30 pip target with an unspecified trailing stop, or 'the first strong resistance level' on the chart; the trend average is only an example (200 SMA or EMA) (user option: daily bars, 200-day SMA, standard exits)",
           &["F|\"The 2nd - signal candle must close 'inside' the 1st - start candle\" (p.8)", "E|buy at the breakout of the high of the signal candle + 1 pip if the trend is up and the signal candle is going up (p.10)", "X|stop 1 pip below the signal candle (p.13); profit target 10-30 pips with a trailing stop (p.12)"]),
    ]
}

fn mine(id: &str) -> bool { id.starts_with("kdfnlt_") || id.starts_with("kdfus_") || id.starts_with("kdsfp_") }

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    if mine(id) { Some(("kdittmann_nt", "Dittmann forex pips (held)", "-")) } else { None }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> { if mine(id) { Some(vec![]) } else { None } }

pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }
