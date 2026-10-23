//! Alexander Elder, "Sell and Sell Short" (Wiley, 2008). Book id "elsss". Catalogue only: the book teaches
//! discretionary selling (targets, stops, "engine noise", shorting); no rule has both an exact entry and an exit.
//! Source copy: Books/Alexander Elder/Alexander Elder - Sell and Sell Short (Wiley Trading) (2008).pdf (text layer).
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, dir, status, rules, notes| StrategyDef { id, name, group, session: 0, dir, status, rules, notes,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "elsss", claim: None };
    vec![
        d("elsss_nhnl_downspike", "Weekly NH-NL downspike below -1,000 in a bull market", "Ch.1 New High-New Low Index", "Long",
          Status::NotTestable("no exit, and \"bull market\" is not defined; it also needs NYSE (or NSE) daily new highs / new lows, which the vault lacks"),
          &["F|NH-NL = New Highs - New Lows (52-week); weekly NH-NL = 5-day running total of daily NH-NL (p.18, 158)",
            "E|\"In an ongoing bull market, a weekly downspike below -1,000 (minus one thousand) identifies the end of a downtrend and a great buying opportunity.\" (p.20)",
            "E|\"Whenever the daily NH-NL falls below zero, while the bull market is going on, it gives a buy signal.\" (p.21)"], ""),
        d("elsss_targets", "Selling at a moving average, envelope or resistance", "Ch.4 Selling at a Target", "Long",
          Status::NotTestable("exit-only (profit targets); entries are discretionary 'value zone' buys and envelope width is fitted by eye (\"A well-drawn channel fits like a good shirt\", p.16)"), &[], ""),
        d("elsss_stops", "Nic's stop, SafeZone, Parabolic, Volatility-Drop and other stops", "Ch.5 Selling on a Stop", "Long/Short",
          Status::NotTestable("stops only, no entry; SafeZone is already coded as an option of elcim_ts_fi; the Volatility-Drop switch is discretionary (\"I need to see a positive enough action to switch to a trailing stop\", p.133)"),
          &["X|Nic's stop: \"a little below the lower of those two bars\" bracketing the low (p.117-118)",
            "X|Parabolic: Stop(tomorrow) = Stop(today) + AF x (EP - Stop(today)), AF 0.02 step 0.02 max 0.20 (p.129)",
            "X|Volatility-Drop: after the 2.7-SD Autoenvelope target, sell when price closes inside a 1.7-SD envelope (p.132-135)"], ""),
        d("elsss_engine_noise", "Engine-noise exits, the market bell, earnings exits", "Ch.6 Selling Engine Noise", "Long",
          Status::NotTestable("discretionary (divergences, 'acting sloppy', psychological signals)"), &[], ""),
        d("elsss_shorting", "Shorting tops, downtrends, fundamentals, futures, options, forex", "Ch.7-8 How to Sell Short", "Short",
          Status::NotTestable("discretionary chart reading (false breakouts, divergences, shorting near value and covering 'to or below the lower channel line', p.186) and option writing"), &[], ""),
    ]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { catalogue().into_iter().find(|d| d.id == id).map(|d| (d.id, d.name, "")) }
pub fn params(_id: &str) -> Option<Vec<ParamDef>> { None }
pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }
