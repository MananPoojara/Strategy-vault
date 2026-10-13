//! Jake Bernstein, "How to Trade the New Single Stock Futures" (Dearborn Trade, 2002). Book id `jbssf`.
//! Source copy: Books/Jake Bernstein/How to Trade the New Single Stock Futures.pdf (printed page = PDF page - 24).
//! Transcription and checks: library/jbssf/. The MAC, channel surfing, momentum divergence and 30-minute breakout
//! chapters repeat methods already in jb624 / jbudt / jb30d and are not re-coded. Helpers come from `jb624` / `jb30d`.
use super::jb30d::seasonal;
use super::jb624::{finish, getp, mark_open, new_trade, stop_fill};
use crate::connors::{claim, Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "jbssf";

macro_rules! def {
    ($id:expr, $name:expr, $group:expr, $dir:expr, $status:expr, $rules:expr, $notes:expr) => {
        StrategyDef { id: $id, name: $name, group: $group, session: 0, dir: $dir, status: $status, needs_volume: false,
            needs_vix: false, needs_series: &[], book: BOOK, rules: $rules, notes: $notes, claim: None }
    };
}

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::{Live, NotTestable};
    vec![
        StrategyDef { claim: claim("19", "94.73%", "9.78 pts avg profit", "1/12-1/18", "Mar S&P futures 1983-2001"),
          ..def!("jbssf_sp_jan", "S&P key-date seasonal: long Jan 12 to Jan 18", "Ch.12 · A Seasonal Strategy", "Long", Live,
            &["E|\"MAR S&P 500 LONG Enter: 1/12 Exit: 1/18 Stop: 3\" (Figure 12.1, p.142): \"buying on the close of trading January 12 every year ... and exiting the position on the close of trading January 18\" (p.142-143)",
              "E|\"For cases in which the market was closed on the ideal entry date, the trade would be executed on the close of business the next day\" (p.143); the printed list rolls exits the same way (1/20/86, 1/19/87)",
              "X|\"the trade carries a 3 percent closing basis stop loss\" (p.143)"],
            "March S&P futures replaced by the index (same underlying). Printed trade list 1983-2001 (19 trades) is compared trade by trade in library/jbssf/.") },
        def!("jbssf_gap", "Gap trade (5% of yesterday's range), exit on the close", "Ch.11 · Advanced Technical Methods", "Long/Short", Live,
            &["E|\"If an SSF opens above the high of the previous day by at least 5 percent of the previous daily trading range (i.e., high - low of day), then sell short on a penetration back below the high of the previous day\" (p.132)",
              "E|\"If an SSF contract opens on a gap below the low of the previous day by 5 percent or more of the previous daily trading range, then sell short [sic] on a penetration back above the low of the previous day\" (p.132): read as a buy, per the book's own gap-down description (\"there is a tendency for the rest of the day to be higher\", p.128)",
              "X|\"Exit the trade at the end of the day\" (p.132)",
              "X|\"Use a risk management or dollar stop loss\" (p.132): no level given, so none is applied"],
            "Entry = a stop order at yesterday's high (sell) / low (buy). The scale-out and intraday trailing stop suggestions have no levels and are not coded."),
        def!("jbssf_mom_ma", "MOM/MA 28 timing (two closes)", "Ch.11 · Advanced Technical Methods", "Long/Short", NotTestable("the author: \"this approach is not a trading system but rather a timing method. In order to function as a complete trading system, the method needs additional features, such as a risk management stop loss\" (p.139)"),
            &["E|\"Calculate a 28-day momentum indicator. Calculate a 28-day moving average of the momentum indicator ... Buy and sell signals are generated accordingly\" (p.136); \"two consecutive closes by the momentum above its MA signals a buy\" (Figure 11.16)"],
            "A long-only MOM/MA 28 with the exit stated is live as jbsmsw_mom_ma."),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "jbssf_sp_jan" => ("jbssf_seasonal", "S&P key-date seasonal (SSF book)", "Jan 12-Jan 18, 3% close stop"),
        "jbssf_gap" => ("jbssf_gap", "Gap trade (SSF book)", "5% of range, exit on the close"),
        "jbssf_mom_ma" => ("jbssf_mom_ma", "MOM/MA 28 (SSF book)", "Not testable"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(match id {
        "jbssf_sp_jan" => vec![p("entry_m", "Entry month", 1., 1., 12., 1.), p("entry_d", "Entry day", 12., 1., 31., 1.), p("exit_m", "Exit month", 1., 1., 12., 1.),
            p("exit_d", "Exit day", 18., 1., 31., 1.), p("stop_pct", "Close-only stop (% below entry)", 3., 0.5, 10., 0.5)],
        "jbssf_gap" => vec![p("gs_pct", "Gap size, % of yesterday's range (at least)", 5., 0., 25., 1.)],
        _ => return None,
    })
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !id.starts_with("jbssf_") || params(id).is_none() || to >= x.c.len() || from > to {
        return None;
    }
    let g = |k: &str| getp(id, p, k);
    match id {
        "jbssf_sp_jan" => Some(seasonal(x, from, to, (g("entry_m"), g("entry_d")), (g("exit_m"), g("exit_d")), g("stop_pct"))),
        "jbssf_gap" => {
            let mut out = Vec::new();
            for i in from.max(1)..=to {
                let gs = g("gs_pct") / 100.0 * (x.h[i - 1] - x.l[i - 1]);
                let (d, lvl) = if x.o[i] > x.h[i - 1] && x.o[i] >= x.h[i - 1] + gs { (-1.0, x.h[i - 1]) } else if x.o[i] < x.l[i - 1] && x.o[i] <= x.l[i - 1] - gs { (1.0, x.l[i - 1]) } else { continue };
                if let Some(px) = stop_fill(x, i, d, lvl) {
                    out.push(finish(new_trade(d, i, px, "stop", i - 1), i, x.c[i], i, "End of day"));
                }
            }
            mark_open(&mut out, None, x, to);
            Some(out)
        }
        _ => Some(vec![]),
    }
}
