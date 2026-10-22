//! Jake Bernstein, "How to Trade the New Single Stock Futures" (Dearborn Trade, 2002). Book id `jbssf`.
//! Source copy: Books/Jake Bernstein/How to Trade the New Single Stock Futures.pdf (printed page = PDF page - 24).
//! Transcription and checks: library/jbssf/. The MAC, channel surfing, momentum divergence and 30-minute breakout
//! chapters repeat methods already in jb624 / jbudt / jb30d and are not re-coded. Helpers come from `jb624` / `jb30d`.
//! The Ch.8 three-moving-average reversal (4/9/18) and the Ch.11 MOM/MA timing method are coded 2026-10-22
//! (bernstein-g1 full reading, authors/bernstein/agents/jbssf/methods.json).
use super::jb30d::seasonal;
use super::jbcaf::machine;
use super::jb624::{finish, getp, mark_open, new_trade, stop_fill};
use crate::connors::{claim, sma, Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "jbssf";

/// Rules admitted after the 2026-10-22 full reading (authors/bernstein/g1-admission/SOURCE-CONTRACT.md) and an
/// independent chronological Python ledger check. The book trades single stock futures (and S&P futures for the
/// seasonal): actual stocks, indices and ETFs are a declared application. Nothing is held in this book's coded list.
pub const REVIEWED: &[&str] = &["jbssf_sp_jan", "jbssf_gap", "jbssf_mom_ma", "jbssf_ma3"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }
/// The gap trade reads the open (gap test, stop entry at a gapped open).
pub fn reads_open(id: &str) -> bool { id == "jbssf_gap" }

macro_rules! def {
    ($id:expr, $name:expr, $group:expr, $dir:expr, $status:expr, $rules:expr, $notes:expr) => {
        StrategyDef { id: $id, name: $name, group: $group, session: 0, dir: $dir, status: $status, needs_volume: false,
            needs_vix: false, needs_series: &[], book: BOOK, rules: $rules, notes: $notes, claim: None }
    };
}

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::Live;
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
        def!("jbssf_mom_ma", "MOM/MA 28 timing (two closes), stop-and-reverse", "Ch.11 · Advanced Technical Methods", "Long/Short", Live,
            &["F|\"Calculate a 28-day momentum indicator. Calculate a 28-day moving average of the momentum indicator\" (p.136); momentum = today's close minus the close 28 days ago (p.133)",
              "E|\"When the 28-day momentum indicator rises above the 28-day moving average ... a change in trend to the upside has likely started ... Buy and sell signals are generated accordingly\" (p.136); \"two consecutive closes by the momentum above its MA signals a buy, whereas two consecutive closes below its MA signals a sell\" (Figure 11.16, p.140)",
              "X|[Read, B] The opposite signal closes the position and reverses it; fills on the close of the second close",
              "X|No stop: the author: \"this approach is not a trading system but rather a timing method. In order to function as a complete trading system, the method needs additional features, such as a risk management stop loss\" (p.139); none is invented here"],
            "Declared reading: the book's signals in both directions run as a stop-and-reverse with no stop (the author calls it a timing method). p.closes=1 uses the text's single cross (p.136) instead of the Figure 11.16 two-close rule. A long-only MOM/MA 28 with a single-cross exit is jbsmsw_mom_ma."),
        def!("jbssf_ma3", "Three moving averages 4/9/18, reversing", "Ch.8 · Moving Averages", "Long/Short", Live,
            &["F|\"the 4-day, 9-day, and 18-day moving averages seem to work best together\" (p.90); a moving average is the average of the last N closes (p.87)",
              "E|\"Buy when all three moving averages have crossed in an upward direction\" (p.92): [Read, B] the 4-day above the 9-day above the 18-day, filled on the close of the first bar with that order",
              "E|\"Sell and sell short when all three moving averages have crossed in a downward direction\" (p.92): 4-day below 9-day below 18-day",
              "X|\"Moving average systems are based on reversals. In other words, when a long position is closed out, a short position is entered\" (p.92)"],
            "Simple moving averages of closes. Equal averages are not an order. No stop is given (the book: \"you don't know your stop loss before entering a trade\", p.93). Differs from kftsm_4_9_18 (Kaufman's own reading)."),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "jbssf_sp_jan" => ("jbssf_seasonal", "S&P key-date seasonal (SSF book)", "Jan 12-Jan 18, 3% close stop"),
        "jbssf_gap" => ("jbssf_gap", "Gap trade (SSF book)", "5% of range, exit on the close"),
        "jbssf_mom_ma" => ("jbssf_mom_ma", "MOM/MA 28 (SSF book)", "Two closes, stop-and-reverse"),
        "jbssf_ma3" => ("jbssf_ma3", "Three moving averages (SSF book)", "4/9/18, reversing"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(match id {
        "jbssf_sp_jan" => vec![p("entry_m", "Entry month", 1., 1., 12., 1.), p("entry_d", "Entry day", 12., 1., 31., 1.), p("exit_m", "Exit month", 1., 1., 12., 1.),
            p("exit_d", "Exit day", 18., 1., 31., 1.), p("stop_pct", "Close-only stop (% below entry)", 3., 0.5, 10., 0.5)],
        "jbssf_gap" => vec![p("gs_pct", "Gap size, % of yesterday's range (at least)", 5., 0., 25., 1.)],
        "jbssf_mom_ma" => vec![p("mom", "Momentum length", 28., 5., 60., 1.), p("ma", "MA of momentum (bars)", 28., 5., 60., 1.),
            p("closes", "Consecutive closes beyond the MA (2 = Figure 11.16, 1 = text)", 2., 1., 3., 1.)],
        "jbssf_ma3" => vec![p("fast", "Fast MA", 4., 2., 20., 1.), p("mid", "Middle MA", 9., 3., 50., 1.), p("slow", "Slow MA", 18., 5., 100., 1.)],
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
        "jbssf_mom_ma" => {
            let u = |k: &str| g(k).round().max(1.0) as usize;
            let (k, need) = (u("mom"), u("closes"));
            let mom: Vec<f64> = (0..x.c.len()).map(|i| if i >= k { x.c[i] - x.c[i - k] } else { f64::NAN }).collect();
            let ma = sma(&mom, u("ma"));
            let (mut last, mut ab, mut be) = (0i8, 0usize, 0usize);
            let sig: Vec<Option<i8>> = (0..x.c.len()).map(|i| {
                if !ma[i].is_finite() { ab = 0; be = 0; return None; }
                let e = 1e-9 * x.c[i].abs(); // a float-noise tie is not a cross
                ab = if mom[i] - ma[i] > e { ab + 1 } else { 0 };
                be = if ma[i] - mom[i] > e { be + 1 } else { 0 };
                let s = if ab >= need { 1 } else if be >= need { -1 } else { 0 };
                (s != 0 && s != last).then(|| { last = s; s })
            }).collect();
            Some(machine(x, from, to, &sig, None, false, ["momentum above its 28-day MA (2 closes)", "momentum below its 28-day MA (2 closes)", ""]))
        }
        "jbssf_ma3" => {
            let u = |k: &str| g(k).round().max(1.0) as usize;
            let (a, b, c) = (sma(&x.c, u("fast")), sma(&x.c, u("mid")), sma(&x.c, u("slow")));
            let mut last = 0i8;
            let sig: Vec<Option<i8>> = (0..x.c.len()).map(|i| {
                if !(a[i].is_finite() && b[i].is_finite() && c[i].is_finite()) { return None; }
                let gt = |p: f64, q: f64| p - q > 1e-9 * q.abs(); // equal averages (within float noise) are not an order
                let s = if gt(a[i], b[i]) && gt(b[i], c[i]) { 1 } else if gt(b[i], a[i]) && gt(c[i], b[i]) { -1 } else { 0 };
                (s != 0 && s != last).then(|| { last = s; s })
            }).collect();
            Some(machine(x, from, to, &sig, None, false, ["4 > 9 > 18-day MAs", "4 < 9 < 18-day MAs", ""]))
        }
        _ => Some(vec![]),
    }
}
