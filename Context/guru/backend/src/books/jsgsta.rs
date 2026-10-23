//! Jack D. Schwager, "Getting Started in Technical Analysis" (Wiley 1999). Ch.14 "Technical Trading Systems:
//! Structure and Design" (printed p.227-262), Ch.3 moving-average turns (p.47-49), Ch.6 MACD (p.113-114) and the
//! Ch.15 six-day moving-average example (p.254). OCR of the research-share scan:
//! claude-continuation-20261020/authors/schwager/ocr/; reading in authors/schwager/agents/gsta/.
//! The systems are the same ones the Complete Guide (2nd ed.) prints, with this book's own example values, so each id
//! runs the shared `jscgf::sim` walk with the defaults below.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "jsgsta";

/// Rules admitted after the 2026-10-23 full reading (authors/schwager/admission/SOURCE-CONTRACT.md) and independent
/// chronological Python ledgers (0 mismatches, US and India). Indices, ETFs and stocks are a declared application.
pub const REVIEWED: &[&str] = &["jsgsta_ma", "jsgsta_ma_x", "jsgsta_bo", "jsgsta_fade", "jsgsta_ma_turn", "jsgsta_ma6", "jsgsta_macd"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "System run as the book describes it: signals on closes (\"The crossover should be determined based on closing prices\", p.230), stop-and-reverse, always in after the first signal. The book shows each one on futures or stock charts (T-bonds, Procter & Gamble, gold, natural gas, cocoa, 3Com); indices, ETFs and stocks are a declared application where the book used futures. Fills at the signal close; p.fill = 1 fills at the next open [Not from the book]. Each run starts flat at the selected start date; earlier bars only warm the averages and channels. Full position, one unit, no stop (the book gives none for these systems).";

const IDS: &[(&str, &str)] = &[("jsgsta_ma", "jscgf_ma"), ("jsgsta_ma_x", "jscgf_ma_x"), ("jsgsta_bo", "jscgf_bo"), ("jsgsta_fade", "jscgf_fade"),
    ("jsgsta_ma_turn", "jscgf_ma_turn"), ("jsgsta_ma6", "jscgf_ma6"), ("jsgsta_macd", "macd")];

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, rules| StrategyDef {
        id, name, group, session: 0, dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false,
        needs_series: &[], book: BOOK, rules, notes: NOTE, claim: None,
    };
    vec![
        d("jsgsta_ma", "Simple moving-average system (35-day, optional MA-direction filter)", "Ch 14 · Technical trading systems", &[
            "E|\"A buy signal is indicated when prices cross the moving average from below; a sell signal is indicated when prices cross the moving average from above. The crossover should be determined based on closing prices.\" p.230",
            "X|Exit = the opposite crossover (\"the simple moving average system is always in the market\" p.244)",
            "F|Default 35 days (Figure 14.1, December 1993 T-bond, p.230); the book also shows 10 (Table 14.1), 12 and 48 (Figures 14.2-14.3)",
            "F|Option p.filter = 1 (p.244): \"prices crossing the moving average from below and closing above the moving average would be accepted as a buy signal only if the moving average was up relative to the previous day's level\"; \"A rejected signal could be activated at a later point, if the moving average subsequently turned in the direction of the signal before an opposite direction crossover\"",
        ]),
        d("jsgsta_ma_x", "Crossover moving-average system (12/48)", "Ch 14 · Technical trading systems", &[
            "E|\"A buy signal is generated when the shorter-term (e.g., 10-day) moving average crosses above the longer-term (e.g., 30-day) moving average; a sell signal is generated when the shorter-term moving average crosses below the longer-term moving average.\" p.232",
            "X|Exit = the opposite crossover (stop-and-reverse)",
            "F|Default 12/48 (Figure 14.4, Procter & Gamble, p.234); 10/30 is the text's example (options)",
        ]),
        d("jsgsta_bo", "Breakout system (close beyond the prior 20-day high/low, optional confirmation)", "Ch 14 · Technical trading systems", &[
            "E|\"1. Cover short and go long if today's close exceeds the prior N-day high. 2. Cover long and go short if today's close is below the prior N-day low.\" p.234",
            "X|Exit = the opposite breakout (stop-and-reverse)",
            "F|Default N = 20 (book examples 7, 10, 12, 20, 40)",
            "F|Option p.confirm (p.240-242): 1 = \"a close that penetrates the prior N-day high (or low) by at least 2%\" (p.pct); 2 = \"closes beyond the signal price ... at any time six or more days beyond the original signal date\" (p.delay); 3 = \"three subsequent thrust days beyond the signal price\" (p.thrusts); a confirmation not met before an opposite signal means no trade",
            "F|Signal price = the prior N-day high (low) the signal close broke; a pending signal keeps the first unconfirmed signal's level and date [Book silent: declared reading]",
        ]),
        d("jsgsta_fade", "Countertrend fading-minimum-move system (4%)", "Ch 14 · Technical trading systems", &[
            "E|\"A sell signal is indicated each time the market rallies by a certain minimum amount above the low point since the last countertrend buy signal. Similarly, a buy signal is indicated whenever the market declines by a minimum amount below the high point since the last countertrend sell signal.\" p.248",
            "X|Exit = the opposite countertrend signal (stop-and-reverse); the book names no stop level",
            "F|Default 4% (Figure 14.13, gold, p.249: \"B = buy signal: 4% decline from prior high; S = sell signal: 4% advance from prior low\"); moves measured on closes [Book silent: declared reading]",
            "F|Option p.confirm = 1: one-thrust-day confirmation (\"fading minimum move with confirmation delay\", p.248)",
        ]),
        d("jsgsta_ma_turn", "Moving-average turn system (40-day, 10-tick turn)", "Ch 3 · Trends", &[
            "E|\"buy signals at points at which the moving average reversed to the upside by at least 10 ticks and sell signals at points at which the moving average turned down by the same minimum amount\" p.48; \"Buy = 10-tick rise in moving average off its low. Sell = 10-tick decline in moving average off its high.\" Figures 3.20-3.21",
            "X|Exit = the opposite turn (stop-and-reverse)",
            "F|40-day simple average (natural gas and cocoa figures, p.47-49); 10 ticks = p.turn percent of the average's low/high [Not from the book: futures ticks; default 0.1%]",
        ]),
        d("jsgsta_ma6", "Six-day moving-average direction system (the magazine system of Ch.15)", "Ch 15 · Testing and optimizing", &[
            "E|\"1. If the six-day moving average is higher than the previous day's corresponding value, cover short and go long. 2. If the six-day moving average is lower than the previous day's corresponding value, cover long and go short.\" p.254",
            "X|Exit = the opposite signal (stop-and-reverse); an unchanged average keeps the position [Book silent]",
            "F|Quoted from a 1983 magazine article as a warning: \"the system lost money in 19 of the 25 markets\" (p.255). Run as printed.",
        ]),
        d("jsgsta_macd", "MACD signal-line crossover (12/26/9)", "Ch 6 · Oscillators", &[
            "F|\"It takes the difference between two exponential moving averages ... 12 days and 26 days—and then calculates a 9-day moving average of this difference ... called the signal line.\" p.114",
            "E|\"A basic MACD trading signal is to buy when the primary MACD line moves above the 9-day signal line and sell when it crosses below the 9-day signal line, in much the same fashion as a moving average crossover.\" p.114",
            "X|Exit = the opposite crossover (stop-and-reverse, as the moving-average crossover system) [declared reading]",
            "F|Exponential average a = 2/(n+1) (Appendix, p.323), seeded with the first close [Book silent]; the 9-day signal line is a simple average as written (\"9-day moving average\"); p.sig_ema = 1 uses a 9-day exponential average [Not from the book]",
        ]),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "jsgsta_ma" => ("jsgsta", "Schwager Getting Started systems", "Simple moving average"),
        "jsgsta_ma_x" => ("jsgsta", "Schwager Getting Started systems", "Crossover moving average"),
        "jsgsta_bo" => ("jsgsta", "Schwager Getting Started systems", "Breakout"),
        "jsgsta_fade" => ("jsgsta", "Schwager Getting Started systems", "Countertrend fade"),
        "jsgsta_ma_turn" => ("jsgsta", "Schwager Getting Started systems", "Moving-average turn"),
        "jsgsta_ma6" => ("jsgsta", "Schwager Getting Started systems", "Six-day MA direction"),
        "jsgsta_macd" => ("jsgsta", "Schwager Getting Started systems", "MACD crossover"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let fill = p("fill", "Fill: 0 = signal close (book), 1 = next open [Not from the book]", 0.0, 0.0, 1.0, 1.0);
    Some(match id {
        "jsgsta_ma" => vec![p("n", "Moving average days (N)", 35.0, 5.0, 100.0, 1.0), p("filter", "MA-direction filter: 0 off, 1 on (p.244)", 0.0, 0.0, 1.0, 1.0), fill],
        "jsgsta_ma_x" => vec![p("short", "Short average days", 12.0, 2.0, 50.0, 1.0), p("long", "Long average days", 48.0, 5.0, 200.0, 1.0), fill],
        "jsgsta_bo" => vec![p("n", "Breakout days (N)", 20.0, 5.0, 150.0, 1.0),
            p("confirm", "Confirmation: 0 none, 1 penetration, 2 time delay, 3 thrust days (p.240-242)", 0.0, 0.0, 3.0, 1.0),
            p("pct", "Penetration percent (confirm 1)", 2.0, 0.5, 10.0, 0.5), p("delay", "Delay days (confirm 2)", 6.0, 1.0, 20.0, 1.0),
            p("thrusts", "Thrust days (confirm 3)", 3.0, 1.0, 5.0, 1.0), fill],
        "jsgsta_fade" => vec![p("pct", "Minimum move percent", 4.0, 2.0, 20.0, 0.5), p("confirm", "One-thrust-day confirmation: 0 off, 1 on", 0.0, 0.0, 1.0, 1.0), fill],
        "jsgsta_ma_turn" => vec![p("n", "Moving average days", 40.0, 5.0, 100.0, 1.0), p("turn", "Turn threshold, percent (10 ticks)", 0.1, 0.02, 2.0, 0.01), fill],
        "jsgsta_ma6" => vec![p("n", "Moving average days", 6.0, 2.0, 50.0, 1.0), fill],
        "jsgsta_macd" => vec![p("fast", "Fast EMA days", 12.0, 2.0, 50.0, 1.0), p("slow", "Slow EMA days", 26.0, 5.0, 100.0, 1.0),
            p("signal", "Signal-line days", 9.0, 2.0, 30.0, 1.0), p("sig_ema", "Signal line: 0 simple (book), 1 exponential [Not from the book]", 0.0, 0.0, 1.0, 1.0), fill],
        _ => return None,
    })
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    let kind = IDS.iter().find(|(i, _)| *i == id)?.1;
    if from > to || to >= x.r.len() { return None; }
    for (k, &v) in p {
        let d = schema.iter().find(|d| d.key == k)?;
        let int = d.def.fract() == 0.0 && d.step.fract() == 0.0;
        if !v.is_finite() || v < d.min || v > d.max || (int && v.fract() != 0.0) { return None; }
    }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap().def);
    Some(super::jscgf::sim(kind, x, from, to, &g))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn every_id_runs_with_its_own_defaults() {
        let r: Vec<Row> = (0..300).map(|i| { let c = 100.0 + 30.0 * ((i as f64) / 15.0).sin();
            Row { ticker: "T".into(), date: format!("2001-{:02}-{:02}", i / 28 + 1, i % 28 + 1), open: c, high: c + 1.0, low: c - 1.0, close: c, volume: None } }).collect();
        let x = Ctx::new(&r, &r, &[]);
        for d in catalogue() { assert!(!run(d.id, &x, 60, r.len() - 1, &Params::new()).unwrap().is_empty(), "{}", d.id); }
        assert_eq!(params("jsgsta_bo").unwrap().iter().find(|d| d.key == "pct").unwrap().def, 2.0);
    }
}
