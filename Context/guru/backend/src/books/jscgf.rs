//! Jack D. Schwager, "A Complete Guide to the Futures Market" (2nd ed., Wiley 2017), Ch.17 "Examples of Original
//! Trading Systems" (printed p.261-277; PDF = printed + 4) with the run-day / wide-ranging-day definitions of Ch.9 (p.118-119).
//! Source copy: Books/Jack D. Schwager/Jack Schwager- Complete Guide to Futures Market-.pdf. README and oracle: library/jscgf/.
use crate::connors::{true_range, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "jscgf";

/// Rules admitted after the 2026-10-23 full reading (authors/schwager/admission/SOURCE-CONTRACT.md) and independent
/// chronological Python ledgers (0 mismatches, US and India). Indices, ETFs and stocks are a declared application.
pub const REVIEWED: &[&str] = &["jscgf_wrd", "jscgf_runday_bo", "jscgf_runday_cc", "jscgf_ma", "jscgf_ma_x", "jscgf_bo", "jscgf_fade", "jscgf_ma_turn", "jscgf_ma6", "jscgf_wk4"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "Futures systems run on the daily index (instrument swap; the book's examples are copper, WTI crude and soybean continuous futures, which the vault does not have). Stop-and-reverse, always in after the first signal. Signals are taken at the signal day's close (the daily checklist acts on \"today's close\"; the testing chapter assumes \"order entry on the close\", p.294); p.fill = 1 fills at the next open [Not from the book]. True high = max(high, previous close), true low = min(low, previous close) (p.261 note 1). Each run starts flat at the selected start date; earlier bars only warm the definitions. Full position, one unit.";
const GENERIC_GAP: &str = "Ch.16 (p.247-256) also describes market-characteristic adjustments (volatility classes with no class boundaries or N per class), different buy/sell conditions (no values), a pyramiding plan (p.252-253: up to two added units after a 10-day reaction, separate pyramid stops) and trade-exit/re-entry rules (none specified). They are illustrations without a complete rule set, or (pyramiding) need per-unit position accounting this runner does not publish; they are not run. The complete Ch.16 rules run as jscgf_ma, jscgf_ma_x, jscgf_bo and jscgf_fade.";
const CH16_NOTE: &str = "System run as the book describes it (Ch.6, 16 and 19; the weekly breakout from the 1st edition, 1984): signals on closes (\"The crossover should be determined based on closing prices\", p.238), stop-and-reverse, always in after the first signal. The book shows each one as an illustration on futures charts (WTI crude, E-mini Nasdaq, silver, coffee, natural gas, gold continuous futures); indices, ETFs and stocks are a declared application. Fills at the signal close (testing chapter: \"order entry on the close\", p.294); p.fill = 1 fills at the next open [Not from the book]. Each run starts flat at the selected start date; earlier bars only warm the averages and channels. Full position, one unit; the book gives no stop for these trend-following systems (\"Stop-loss conditions are optional for most trend-following systems\", p.255).";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, status, rules, notes| StrategyDef {
        id, name, group: "Ch 17 · Examples of Original Trading Systems", session: 0, dir: "Long/Short", status,
        needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None,
    };
    let c = |id, name, rules| StrategyDef {
        id, name, group: "Ch 6, 16, 19 · Technical trading systems", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes: CH16_NOTE, claim: None,
    };
    use Status::*;
    vec![
        d("jscgf_wrd", "Wide-ranging-day system (k 2.0, N 10, N1 0, N2 0)", Live, &[
            "F|\"Wide-ranging day. A day on which the volatility ratio (VR) is greater than k (e.g., k = 2). The VR is equal to today's true range divided by the average true range of the past N-day period (e.g., N = 10).\" p.262 (past N days = the N days before today [Book ambiguous])",
            "F|\"Price trigger range (PTR). The range defined by the highest true high and lowest true low in the interval between N1 days before the most recent wide-ranging day to N2 days after.\" p.262",
            "E|\"1. If short and today's close is above the high of the PTR, liquidate short and go long. 2. If long and today's close is below the low of the PTR, liquidate long and go short. 3. Check whether exactly N2 days have elapsed since the most recent wide-ranging day. If this condition is met, redefine the PTR.\" p.262 (steps in this order)",
            "X|Exit = the opposite signal: \"Buy case. On a close above the high of the PTR, reverse from short to long. Sell case. On a close below the low of the PTR, reverse from long to short.\" p.262",
            "F|Illustrated parameter set N1 = 0, N2 = 0 (Figures 17.1-17.5, p.264); Table 17.1 lists k 1.6/2.0/2.4 x N1, N2 in 0/2/4 p.263",
        ], NOTE),
        d("jscgf_runday_bo", "Run-day breakout system (N1 5, N2 4)", Live, &[
            "F|\"Up run day. 1. The true high of the run day is greater than the maximum true high of the past N days ... 2. The true low on the run day is less than the minimum true low on the subsequent N days.\" (down run day mirrored) p.118; \"Days that fulfill both the up and down run day definitions are not considered run days.\" p.269 note 2",
            "E|\"Buy case. Reverse to long whenever both ... 1. The close is above the maximum true high of the most recent N2 down run days ... 2. The most recent run day is an up run day.\" (sell case mirrored) p.269",
            "X|Exit = the opposite signal (\"Reverse to short whenever ...\", sell case mirrored) p.269",
            "F|\"Check whether the trading day N1 days prior to the current day can be defined as an up or a down run day\" each day before the signal checks p.269; illustrated N1 = 5, N2 = 4 (Figures 17.6-17.9, p.270)",
        ], NOTE),
        d("jscgf_runday_cc", "Run-day consecutive count system (N1 5, N2 3)", Live, &[
            "F|\"Buy count ... starts at zero and increases by one whenever a new up run day is defined. The count is reset to zero whenever there is a down run day ... The buy count is closed when a buy signal is received.\" (sell count mirrored) p.273",
            "E|\"Buy case. Reverse to long whenever the buy count reaches N2.\" \"Sell case. Reverse to short whenever the sell count reaches N2.\" p.273-274",
            "X|Exit = the opposite count signal (\"cover short, go long ... cover long, go short\") p.274",
            "F|Illustrated N1 = 5, N2 = 3 (Figures 17.10-17.14, p.275). Before the first signal both counts run [Book silent: \"Either the buy or sell count is always active, depending on whether the current position is short or long\"].",
        ], NOTE),
        c("jscgf_ma", "Simple moving-average system (35-day, optional MA-direction filter)", &[
            "F|\"The moving average for a given day is equal to the average of that day's closing price and the closing prices on the preceding N - 1 days\" p.237",
            "E|\"a buy signal is indicated when prices cross the moving average from below; a sell signal is indicated when prices cross the moving average from above. The crossover should be determined based on closing prices.\" p.238",
            "X|Exit = the opposite crossover (always in, stop-and-reverse; \"the simple moving average system is always in the market\" p.251)",
            "F|Default N = 35 (Figure 16.1, June 2015 WTI crude, p.239); the book also uses 10 (Table 16.1), 12 and 48 (Figures 16.2-16.3). [Book gives no single N]",
            "F|Option p.filter = 1 (p.250-251): \"price crossing the moving average from below and closing above the moving average would be accepted as a buy signal only if the moving average was up relative to the previous day's level\"; \"A rejected signal could be activated later if the moving average subsequently turned in the direction of the signal before an opposite-direction crossover\" (sell mirrored)",
        ]),
        c("jscgf_ma_x", "Crossover moving-average system (12/48)", &[
            "E|\"a buy signal is generated when the shorter moving average crosses above the longer moving average; a sell signal is generated when the shorter moving average crosses below the longer moving average\" p.241",
            "X|Exit = the opposite crossover (stop-and-reverse)",
            "F|12-day and 48-day simple averages of closes (Figure 16.4, E-mini Nasdaq 100, p.242)",
        ]),
        c("jscgf_bo", "Breakout system (close beyond the prior 20-day high/low, optional confirmation)", &[
            "E|\"1. cover short and go long if today's close exceeds the prior N-day high. 2. cover long and go short if today's close is below the prior N-day low.\" p.243",
            "X|Exit = the opposite breakout (stop-and-reverse)",
            "F|Default N = 20 (\"close beyond the 20-day high or low\" p.237; \"breakout system, with N = 20\" p.251, p.257); the book also shows 7, 10, 12 and 40",
            "F|Option p.confirm (p.247-249; \"If these conditions are not realized before an opposite direction signal is received, no trade occurs\"): 1 = penetration, \"a close that exceeds the prior N-day high or low by at least 3 percent\" (p.pct); 2 = time delay, \"closes beyond the signal price ... at any time six or more days beyond the original signal date\" (p.delay); 3 = pattern, \"three subsequent thrust days beyond the signal price\" (p.thrusts; thrust day = close above the previous day's high / below its low, p.249 note 4)",
            "F|Signal price = the prior N-day high (low) the signal close broke; a pending signal keeps the first unconfirmed signal's level and date [Book silent: declared reading]",
        ]),
        c("jscgf_fade", "Countertrend fading-minimum-move system (7.5%)", &[
            "E|\"A sell signal is indicated each time the market rallies by a certain minimum amount above the low point since the last countertrend buy signal. Similarly, a buy signal is indicated whenever the market declines by a minimum amount below the high point since the last countertrend sell signal.\" p.255",
            "X|Exit = the opposite countertrend signal (stop-and-reverse); the book names no stop level",
            "F|Default 7.5 percent (Figure 16.13, natural gas, p.256: \"B = buy signal: 7.5% decline from prior high; S = sell signal: 7.5% advance from prior low\")",
            "F|Moves are measured on closes; before the first signal the high and low points run from the start date [Book silent: declared reading]",
            "F|Option p.confirm = 1 (p.255): \"a one-thrust-day confirmation might be required to validate countertrend signals\": enter on the first later thrust day in the signal's direction, cancelled by an opposite countertrend signal",
        ]),
        c("jscgf_ma_turn", "Moving-average turn system (40-day, 10-tick turn)", &[
            "E|\"buy signals at points at which the moving average reversed to the upside by at least 10 ticks and sell signals at points at which the moving average turned down by the same minimum amount\" p.79; \"Buy (B) = 10-tick rise in moving average off its low. Sell (S) = 10-tick decline in moving average off its high.\" Figure 6.37 p.80",
            "X|Exit = the opposite turn (stop-and-reverse; \"buying on upturns in the moving average and selling on downturns\" p.80)",
            "F|40-day simple average of closes (Figures 6.35-6.38, Canadian dollar and oat continuous futures, p.79-80)",
            "F|10 ticks = p.turn percent of the average's low/high [Not from the book: futures ticks have no stock or index equivalent; default 0.1% = 10 ticks of 0.01%]. The low/high is measured from the last signal (from the start date before the first one).",
        ]),
        c("jscgf_ma6", "Six-day moving-average direction system (the magazine system of Ch.19)", &[
            "E|\"1. If the six-day moving average is higher than the previous day's corresponding value, cover short and go long. 2. If the six-day moving average is lower than the previous day's corresponding value, cover long and go short.\" p.290",
            "X|Exit = the opposite signal (stop-and-reverse); an unchanged average keeps the position [Book silent]",
            "F|Schwager quotes this system from a 1983 trade-magazine article as a warning: tested 1976-1983 on 25 markets it \"lost money in 19 of the 25 markets\" (p.290-291). It runs here as printed.",
        ]),
        c("jscgf_wk4", "Weekly 4-week breakout system (1st edition)", &[
            "E|\"goes long on a weekly close above the high of the previous 4-week period and ... goes short on a weekly close below the low of the prior 4-week period\" (A Complete Guide to the Futures Markets, 1st ed. 1984, p.410)",
            "X|Exit = the opposite weekly signal (stop-and-reverse)",
            "F|Weeks run Monday to Friday; the weekly close is the last trading day's close in the price file, the 4-week high/low the highest high and lowest low of the four previous weeks' daily bars [declared reading]",
        ]),
        d("jscgf_generic", "Ch.16 generic MA and breakout systems", NotTestable(GENERIC_GAP), &["E|\"close above the 20-day high\" basic breakout system, confirmation and filter variations p.235-260"], GENERIC_GAP),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "jscgf_wrd" => ("jscgf", "Schwager original systems", "Wide-ranging day"),
        "jscgf_runday_bo" => ("jscgf", "Schwager original systems", "Run-day breakout"),
        "jscgf_runday_cc" => ("jscgf", "Schwager original systems", "Run-day consecutive count"),
        "jscgf_ma" => ("jscgf", "Schwager Ch.16 systems", "Simple moving average"),
        "jscgf_ma_x" => ("jscgf", "Schwager Ch.16 systems", "Crossover moving average"),
        "jscgf_bo" => ("jscgf", "Schwager Ch.16 systems", "Breakout"),
        "jscgf_fade" => ("jscgf", "Schwager Ch.16 systems", "Countertrend fade"),
        "jscgf_ma_turn" => ("jscgf", "Schwager Ch.16 systems", "Moving-average turn"),
        "jscgf_ma6" => ("jscgf", "Schwager Ch.16 systems", "Six-day MA direction"),
        "jscgf_wk4" => ("jscgf", "Schwager Ch.16 systems", "Weekly 4-week breakout"),
        "jscgf_generic" => ("jscgf", "Schwager Ch.16 systems", "Pyramiding and other Ch.16 ideas · not testable"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let fill = p("fill", "Fill: 0 = signal close (book), 1 = next open [Not from the book]", 0.0, 0.0, 1.0, 1.0);
    Some(match id {
        "jscgf_wrd" => vec![p("k", "Volatility ratio above (k)", 2.0, 1.6, 2.4, 0.4), p("n", "ATR days (N)", 10.0, 5.0, 20.0, 5.0),
            p("n1", "Days before the WRD in the PTR (N1)", 0.0, 0.0, 4.0, 2.0), p("n2", "Days after the WRD in the PTR (N2)", 0.0, 0.0, 4.0, 2.0), fill],
        "jscgf_runday_bo" => vec![p("n1", "Run-day definition days (N1)", 5.0, 3.0, 7.0, 2.0), p("n2", "Run days in the breakout extreme (N2)", 4.0, 2.0, 5.0, 1.0), fill],
        "jscgf_runday_cc" => vec![p("n1", "Run-day definition days (N1)", 5.0, 3.0, 7.0, 2.0), p("n2", "Consecutive run days (N2)", 3.0, 1.0, 4.0, 1.0), fill],
        "jscgf_ma" => vec![p("n", "Moving average days (N)", 35.0, 5.0, 100.0, 1.0), p("filter", "MA-direction filter: 0 off, 1 on (p.250)", 0.0, 0.0, 1.0, 1.0), fill],
        "jscgf_ma_x" => vec![p("short", "Short average days", 12.0, 2.0, 50.0, 1.0), p("long", "Long average days", 48.0, 5.0, 200.0, 1.0), fill],
        "jscgf_bo" => vec![p("n", "Breakout days (N)", 20.0, 5.0, 150.0, 1.0),
            p("confirm", "Confirmation: 0 none, 1 penetration, 2 time delay, 3 thrust days (p.247-249)", 0.0, 0.0, 3.0, 1.0),
            p("pct", "Penetration percent (confirm 1)", 3.0, 0.5, 10.0, 0.5), p("delay", "Delay days (confirm 2)", 6.0, 1.0, 20.0, 1.0),
            p("thrusts", "Thrust days (confirm 3)", 3.0, 1.0, 5.0, 1.0), fill],
        "jscgf_ma_turn" => vec![p("n", "Moving average days", 40.0, 5.0, 100.0, 1.0), p("turn", "Turn threshold, percent (10 ticks)", 0.1, 0.02, 2.0, 0.01), fill],
        "jscgf_ma6" => vec![p("n", "Moving average days", 6.0, 2.0, 50.0, 1.0), fill],
        "jscgf_wk4" => vec![p("weeks", "Prior weeks", 4.0, 2.0, 26.0, 1.0), fill],
        "jscgf_fade" => vec![p("pct", "Minimum move percent", 7.5, 2.0, 20.0, 0.5), p("confirm", "One-thrust-day confirmation: 0 off, 1 on (p.255)", 0.0, 0.0, 1.0, 1.0), fill],
        _ => return None,
    })
}

pub fn validate(id: &str, p: &Params) -> Option<String> {
    let schema = params(id)?;
    for (k, &v) in p {
        let Some(d) = schema.iter().find(|d| d.key == k) else { return Some(format!("unknown parameter {k}")) };
        let int = d.def.fract() == 0.0 && d.step.fract() == 0.0;
        if !v.is_finite() || v < d.min || v > d.max || (int && v.fract() != 0.0) { return Some(format!("{k} must be in {}..{}", d.min, d.max)); }
    }
    None
}

/// Monday-based week number of each bar's date (days since 1970-01-01 + 3, divided by 7).
fn week_index(x: &Ctx) -> Vec<i64> {
    x.r.iter().map(|r| {
        let p: Vec<i64> = r.date.split('-').filter_map(|v| v.parse().ok()).collect();
        let (y, m, d) = if p.len() == 3 { (p[0], p[1], p[2]) } else { (1970, 1, 1) };
        let y2 = if m <= 2 { y - 1 } else { y };
        let era = y2.div_euclid(400);
        let yoe = y2 - era * 400;
        let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
        let days = era * 146097 + yoe * 365 + yoe / 4 - yoe / 100 + doy - 719468;
        (days + 3).div_euclid(7)
    }).collect()
}

/// Exponential average, a = 2/(n+1), seeded with the first value [Book silent on the seed].
fn ema(v: &[f64], n: usize) -> Vec<f64> {
    let a = 2.0 / (n as f64 + 1.0);
    let mut out = vec![f64::NAN; v.len()];
    for i in 0..v.len() { out[i] = if i == 0 { v[0] } else { a * v[i] + (1.0 - a) * out[i - 1] }; }
    out
}

/// Simple average of the last n values, summed over the window each day (no running sum).
fn wsma(v: &[f64], n: usize) -> Vec<f64> {
    (0..v.len()).map(|i| if n == 0 || i + 1 < n { f64::NAN } else { v[i + 1 - n..=i].iter().sum::<f64>() / n as f64 }).collect()
}

fn true_hl(x: &Ctx) -> (Vec<f64>, Vec<f64>) {
    let th = (0..x.c.len()).map(|i| if i == 0 { x.h[0] } else { x.h[i].max(x.c[i - 1]) }).collect();
    let tl = (0..x.c.len()).map(|i| if i == 0 { x.l[0] } else { x.l[i].min(x.c[i - 1]) }).collect();
    (th, tl)
}

/// Run-day type of bar d given N (1 up, -1 down, 0 none); defined only once d + n <= last bar.
pub(crate) fn run_day(th: &[f64], tl: &[f64], d: usize, n: usize) -> i8 {
    if d < n || d + n >= th.len() { return 0; }
    let max = |v: &[f64]| v.iter().copied().fold(f64::MIN, f64::max);
    let min = |v: &[f64]| v.iter().copied().fold(f64::MAX, f64::min);
    let up = th[d] > max(&th[d - n..d]) && tl[d] < min(&tl[d + 1..=d + n]);
    let dn = tl[d] < min(&tl[d - n..d]) && th[d] > max(&th[d + 1..=d + n]);
    match (up, dn) { (true, false) => 1, (false, true) => -1, _ => 0 }
}

/// Stop-and-reverse on the bars where `sigs` is +1/-1 (fill at that close, or the next open).
fn sar(x: &Ctx, from: usize, to: usize, next_open: bool, sigs: &[i8]) -> Vec<Trade> {
    let mut out = vec![];
    let mut cur: Option<Trade> = None;
    for i in from..=to {
        let s = sigs[i] as f64;
        if s == 0.0 || cur.as_ref().is_some_and(|t| t.dir == s) { continue; }
        let (j, px) = if next_open { if i == to { break; } (i + 1, x.o[i + 1]) } else { (i, x.c[i]) };
        if let Some(mut t) = cur.take() { t.exit_i = j; t.exit_px = px; t.signal_i = i; t.reason = "reverse".into(); out.push(t); }
        cur = Some(Trade { dir: s, legs: vec![Leg { i: j, px, w: 1.0 }], exit_i: j, exit_px: px, reason: String::new(),
            entry_kind: if next_open { "open" } else { "close" }, signal_i: i, open: false, exits: vec![] });
    }
    if let Some(mut t) = cur { t.exit_i = to; t.exit_px = x.c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true; out.push(t); }
    out
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if validate(id, p).is_some() || from > to || to >= x.r.len() { return None; }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap().def);
    Some(sim(id, x, from, to, &g))
}

/// Shared system walk (also used by `jsgsta`, Getting Started in Technical Analysis, with that book's defaults).
/// `kind` is a jscgf id or "macd"; `g` returns each parameter value.
pub(crate) fn sim(kind: &str, x: &Ctx, from: usize, to: usize, g: &dyn Fn(&str) -> f64) -> Vec<Trade> {
    let id = kind;
    let u = |k: &str| g(k) as usize;
    let (th, tl) = true_hl(x);
    // Signals are computed bar by bar in time order with the held direction (needed for the "if short"/"if long" checks),
    // so each system walks forward itself and records +1/-1 on the bars where it reverses.
    let mut sig = vec![0i8; x.c.len()];
    let mut held = 0i8;
    match id {
        "jscgf_wrd" => {
            let (k, n, n1, n2) = (g("k"), u("n"), u("n1"), u("n2"));
            let tr = true_range(x.r);
            let (mut ptr, mut last_w): (Option<(f64, f64)>, Option<usize>) = (None, None);
            for i in 0..=to {
                if i >= from {
                    if let Some((hi, lo)) = ptr {
                        let s = if held <= 0 && x.c[i] > hi { 1 } else if held >= 0 && x.c[i] < lo { -1 } else { 0 };
                        if s != 0 { sig[i] = s; held = s; }
                    }
                }
                if i >= n && tr[i] / (tr[i - n..i].iter().sum::<f64>() / n as f64) > k { last_w = Some(i); }
                if let Some(w) = last_w {
                    if i - w == n2 && w >= n1 {
                        let hi = th[w - n1..=i].iter().copied().fold(f64::MIN, f64::max);
                        let lo = tl[w - n1..=i].iter().copied().fold(f64::MAX, f64::min);
                        ptr = Some((hi, lo));
                    }
                }
            }
        }
        "jscgf_ma" | "jscgf_ma_x" => {
            // side = +1 while the (short) average / close is above, -1 below; equal keeps the previous side
            let (fast, slow, filter) = if id == "jscgf_ma" { (None, wsma(&x.c, u("n")), g("filter") == 1.0) } else { (Some(wsma(&x.c, u("short"))), wsma(&x.c, u("long")), false) };
            let (mut side, mut pend) = (0i8, 0i8);
            for i in 0..=to {
                let a = fast.as_ref().map_or(x.c[i], |f| f[i]);
                if !(a.is_finite() && slow[i].is_finite()) { continue; }
                let prev = side;
                if a > slow[i] { side = 1 } else if a < slow[i] { side = -1 }
                if i < from { continue; }
                if prev != 0 && side != prev { pend = side; } // a crossover inside the window
                if pend == 0 || pend == held { continue; }
                // p.250: the signal is accepted only while the average moves its way (compared with yesterday's average)
                let ok = !filter || (i > 0 && slow[i - 1].is_finite() && (slow[i] - slow[i - 1]) * pend as f64 > 0.0);
                if ok { sig[i] = pend; held = pend; }
            }
        }
        "jscgf_ma_turn" => {
            // p.79-80: buy when the average has risen at least the threshold off its low, sell when it has fallen that much
            // off its high; the low/high run from the last signal (from the start date before the first one)
            let (ma, turn) = (wsma(&x.c, u("n")), g("turn") / 100.0);
            let (mut hi, mut lo) = (f64::NAN, f64::NAN);
            for i in from..=to {
                if !ma[i].is_finite() { continue; }
                if !hi.is_finite() { hi = ma[i]; lo = ma[i]; }
                hi = hi.max(ma[i]); lo = lo.min(ma[i]);
                let s = if held <= 0 && ma[i] >= lo * (1.0 + turn) { 1 } else if held >= 0 && ma[i] <= hi * (1.0 - turn) { -1 } else { 0 };
                if s != 0 { sig[i] = s; held = s; hi = ma[i]; lo = ma[i]; }
            }
        }
        "jscgf_ma6" | "macd" => {
            // six-day: the average against its own previous value (p.290); MACD: 12/26 EMA difference against its 9-day line
            let (a, b) = if id == "macd" {
                let line: Vec<f64> = ema(&x.c, u("fast")).iter().zip(ema(&x.c, u("slow"))).map(|(f, s)| f - s).collect();
                let sl = if g("sig_ema") == 1.0 { ema(&line, u("signal")) } else { wsma(&line, u("signal")) };
                (line, sl)
            } else {
                let m = wsma(&x.c, u("n"));
                let prev = (0..m.len()).map(|i| if i == 0 { f64::NAN } else { m[i - 1] }).collect();
                (m, prev)
            };
            let mut side = 0i8;
            for i in 0..=to {
                if !(a[i].is_finite() && b[i].is_finite()) { continue; }
                let prev = side;
                if a[i] > b[i] { side = 1 } else if a[i] < b[i] { side = -1 }
                if i < from { continue; }
                // six-day rule acts on the state each day; MACD on a crossover inside the window
                let s = if id == "macd" { if prev != 0 && side != prev { side } else { 0 } } else { side };
                if s != 0 && s != held { sig[i] = s; held = s; }
            }
        }
        "jscgf_wk4" => {
            let w = week_index(x);
            let k = u("weeks");
            // per finished week: (high, low); a week is finished on the bar whose next bar starts a new week
            let mut weeks: Vec<(f64, f64)> = vec![];
            let (mut wh, mut wl) = (f64::MIN, f64::MAX);
            for i in 0..=to {
                wh = wh.max(x.h[i]); wl = wl.min(x.l[i]);
                if i + 1 >= x.c.len() || w[i + 1] == w[i] { continue; }
                if i >= from && weeks.len() >= k {
                    let hh = weeks[weeks.len() - k..].iter().map(|v| v.0).fold(f64::MIN, f64::max);
                    let ll = weeks[weeks.len() - k..].iter().map(|v| v.1).fold(f64::MAX, f64::min);
                    let s = if held <= 0 && x.c[i] > hh { 1 } else if held >= 0 && x.c[i] < ll { -1 } else { 0 };
                    if s != 0 { sig[i] = s; held = s; }
                }
                weeks.push((wh, wl)); wh = f64::MIN; wl = f64::MAX;
            }
        }
        "jscgf_bo" => {
            let (n, mode, pct, delay, need) = (u("n"), u("confirm"), g("pct") / 100.0, u("delay"), u("thrusts"));
            // pending unconfirmed signal: (direction, level broken, signal bar, thrust days counted)
            let mut pend: Option<(i8, f64, usize, usize)> = None;
            for i in from.max(n)..=to {
                let hh = x.h[i - n..i].iter().copied().fold(f64::MIN, f64::max);
                let ll = x.l[i - n..i].iter().copied().fold(f64::MAX, f64::min);
                let base = if x.c[i] > hh { 1 } else if x.c[i] < ll { -1 } else { 0 };
                if base != 0 {
                    // an opposite signal cancels the pending one; a signal in the held direction leaves nothing pending
                    if base == held { pend = None; }
                    else if pend.map_or(true, |q| q.0 != base) { pend = Some((base, if base == 1 { hh } else { ll }, i, 0)); }
                }
                let Some((d, lvl, s, mut k)) = pend else { continue };
                let df = d as f64;
                let go = match mode {
                    0 => true,
                    1 => df * (x.c[i] - lvl * (1.0 + df * pct)) >= 0.0,
                    2 => i - s >= delay && df * (x.c[i] - lvl) > 0.0,
                    _ => {
                        if i > s && df * (x.c[i] - lvl) > 0.0 && (if d == 1 { x.c[i] > x.h[i - 1] } else { x.c[i] < x.l[i - 1] }) { k += 1; }
                        pend = Some((d, lvl, s, k));
                        k >= need
                    }
                };
                if go { sig[i] = d; held = d; pend = None; }
            }
        }
        "jscgf_fade" => {
            let (pct, confirm) = (g("pct") / 100.0, g("confirm") == 1.0);
            // last countertrend signal (sets which extreme is watched) and, with confirmation, the bar it came on
            let (mut last, mut s) = (0i8, from);
            let (mut hi, mut lo) = (x.c[from], x.c[from]);
            for i in from..=to {
                hi = hi.max(x.c[i]); lo = lo.min(x.c[i]);
                let t = if last >= 0 && x.c[i] >= lo * (1.0 + pct) { -1 } else if last <= 0 && x.c[i] <= hi * (1.0 - pct) { 1 } else { 0 };
                if t != 0 { last = t; s = i; if t == 1 { lo = x.c[i]; } else { hi = x.c[i]; } }
                if last == 0 || last == held { continue; }
                let thrust = i > s && (if last == 1 { x.c[i] > x.h[i - 1] } else { x.c[i] < x.l[i - 1] });
                if !confirm || thrust { sig[i] = last; held = last; }
            }
        }
        _ => {
            let (n1, n2) = (u("n1"), u("n2"));
            let cc = id == "jscgf_runday_cc";
            let (mut ups, mut dns): (Vec<usize>, Vec<usize>) = (vec![], vec![]);
            let mut last = 0i8;
            let (mut buy, mut sell) = (0usize, 0usize);
            for i in 0..=to {
                let rd = if i >= n1 { run_day(&th[..=i], &tl[..=i], i - n1, n1) } else { 0 };
                if rd == 1 { ups.push(i - n1); } else if rd == -1 { dns.push(i - n1); }
                if rd != 0 { last = rd; }
                if i < from { continue; }
                let s = if cc {
                    if rd == 1 { if held <= 0 { buy += 1; } if held >= 0 { sell = 0; } }
                    if rd == -1 { if held >= 0 { sell += 1; } if held <= 0 { buy = 0; } }
                    if held <= 0 && buy == n2 { 1 } else if held >= 0 && sell == n2 { -1 } else { 0 }
                } else {
                    let ext_hi = (dns.len() >= n2).then(|| dns[dns.len() - n2..].iter().map(|&d| th[d]).fold(f64::MIN, f64::max));
                    let ext_lo = (ups.len() >= n2).then(|| ups[ups.len() - n2..].iter().map(|&d| tl[d]).fold(f64::MAX, f64::min));
                    if held <= 0 && last == 1 && ext_hi.is_some_and(|h| x.c[i] > h) { 1 }
                    else if held >= 0 && last == -1 && ext_lo.is_some_and(|l| x.c[i] < l) { -1 } else { 0 }
                };
                if s != 0 { sig[i] = s; held = s; buy = 0; sell = 0; }
            }
        }
    }
    sar(x, from, to, g("fill") == 1.0, &sig)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn bar(o: f64, h: f64, l: f64, c: f64) -> Row { Row { ticker: "T".into(), date: String::new(), open: o, high: h, low: l, close: c, volume: None } }
    #[test]
    fn wrd_reverses_on_closes_beyond_the_wide_day() {
        let mut r: Vec<Row> = (0..12).map(|_| bar(100., 101., 99., 100.)).collect();
        r.push(bar(100., 106., 98., 105.)); // 12: TR 8 > 2 x 2 -> PTR 98..106 at its close
        r.push(bar(105., 107., 104., 106.5)); // 13: close 106.5 > 106 -> long at close
        r.push(bar(106., 106., 97., 97.5)); // 14: TR 9.5 vs avg(10)=~2.8 -> new WRD, but signal first: close < 98 -> short
        let x = Ctx::new(&r, &r, &[]);
        let t = run("jscgf_wrd", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert_eq!(t.len(), 2);
        assert_eq!((t[0].dir, t[0].legs[0].i, t[0].exit_i, t[0].exit_px), (1.0, 13, 14, 97.5));
        assert_eq!(t[1].dir, -1.0);
    }
    #[test]
    fn run_day_definition() {
        let th = [10., 11., 12., 15., 13., 13.5, 14.];
        let tl = [9., 10., 11., 10.5, 11., 11.5, 12.];
        assert_eq!(run_day(&th, &tl, 3, 2), 1); // high above prior 2, low below next 2
        assert_eq!(run_day(&th, &tl, 3, 3), 1);
        assert_eq!(run_day(&th, &tl, 4, 3), 0); // not definable yet
    }
}
