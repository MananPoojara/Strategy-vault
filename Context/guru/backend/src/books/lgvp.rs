//! Lee E. Gettess, "VOLPAT Trading System" (S&P 500 manual + "Volpat for T-Bonds" + addenda; leegettess.com).
//! Book id `lgvp`. Source copy: Books/Lee E. Gettess/volpat.pdf (52 scanned pages; refs = PDF page numbers).
//! Library: library/lgvp/.
//!
//! System #1 (volatility, p.8-13) + pattern recognition (#3, #2, #1; p.14-20) give one BUY and one SELL stop for the
//! day; "When one order is executed the other becomes the stop-loss point for the trade ... place an order to exit the
//! position Market On Close" (p.21). Day trade, one trade per day. Which of two orders filled first on a bar that hit
//! both: the stop at/through the open first, else the vault path (open -> nearer extreme -> other extreme).
use crate::books::lg3k::{oco_day, roundup};
use crate::connors::{claim, Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "lgvp";

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    let d = |id, name, group, status, rules, notes, claim| StrategyDef { id, name, group, session: 0, dir: "Long/Short", status,
        needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim };
    vec![
        d("lgvp_volpat", "VOLPAT S&P day trade (volatility + patterns, OCO, exit MOC)", "VOLPAT S&P 500 (p.8-25)", Live,
          &["F|Three-day total true range = HIGHEST HIGH minus LOWEST CLOSE of the last three days, or HIGHEST CLOSE minus LOWEST LOW if greater (p.8)",
            "F|x.70, rounded UP to 0 or 5 in the second decimal, added to / subtracted from yesterday's CLOSE = BUY / SELL points; x.75 likewise from today's OPEN (p.8-9)",
            "F|\"whichever is LOWER becomes the actual BUY point ... whichever is HIGHER is the actual SELL point\"; but \"If the OPEN is HIGHER than the BUY point you calculated from the CLOSE you must use the BUY point calculated from the OPEN\" (and mirror for SELL) (p.9, p.13)",
            "F|Patterns, checked #3 then #2 then #1: \"Once you find a pattern, do not look for the next lower number pattern also\" (p.14). A = yesterday, B, C, D = 2, 3, 4 days ago",
            "F|#3: HIGH A > HIGH B and CLOSE A < CLOSE B and OPEN < CLOSE B -> BUY point CLOSE B; LOW A < LOW B and CLOSE A > CLOSE B and OPEN > CLOSE B -> SELL point CLOSE B (p.19)",
            "F|#2: LOW B < LOW C and LOW A > LOW B and OPEN < CLOSE B -> BUY point CLOSE B; HIGH B > HIGH C and HIGH A < HIGH B and OPEN > CLOSE B -> SELL point CLOSE B (p.17)",
            "F|#1: CLOSE C < CLOSE D, CLOSE B > CLOSE C, CLOSE A < CLOSE B -> BUY point at HIGH A provided the OPEN is below it; mirror '+ - +' -> SELL point at LOW A provided the OPEN is above it (p.15)",
            "F|A pattern BUY point replaces the BUY point only if it is lower; a pattern SELL point replaces the SELL point only if it is higher (p.16, 18, 20)",
            "E|\"Each order should be entered every day. When one order is executed the other becomes the stop-loss point for the trade\" (p.21) - buy stop / sell stop",
            "X|\"Whenever you are filled on one of your orders you should place an order to exit the position Market On Close\" (p.21); the other order is the stop (OCO)",
            "X|[intraday wave 2b] Default p.path=1: real 1-minute bars decide which order filled first and whether the other (the stop) was reached after the fill (fills at the level, or the minute's open when it opened through it). A minute that reaches both points without opening through exactly one: order unknown, no trade [convention]; fill minute that also reaches the stop: filled then stopped, unless that minute opened beyond the stop, then stopped only if it closed beyond it [convention]. Runs on SPY, SPX (from 2014, real daily opens), NIFTY 50, NIFTYBANK; the 0.05 rounding is the book's S&P tick, kept on every instrument. NIFTY 50 days from 2026-08-11 skipped (bad closing minutes). p.path=0 = the old daily bar path [Not from the book]."],
          "Book trade listing: S&P INDEX D0691 (June 1991 futures) 3/1-6/12/1991, $0 costs, 31 trades, total +$11,900 = 23.8 points at $500 (p.29); D0991 6/19-7/1/1991 (p.30). 1991-2001 summary (3 contracts): 955 trades, 61% (p.52) - that table is for the later first-profitable-open version, not this one. Run on SPX/SPY/NIFTY (instrument swap); rounding tick 0.05 (S&P / NSE). Pattern precedence: by default the first pattern number found on either side is the only pattern used that day (p.14 literal); param pat_scope = 1 looks per side.",
          claim("31 (list)", "", "23.8 pts total", "same day", "S&P June 1991 futures 3/1-6/12/1991 (p.29)")),
        d("lgvp_volpat_fpo", "VOLPAT with first-profitable-open exit (addendum)", "VOLPAT addenda (p.49-52)",
          NotTestable("the addendum states only the profit exit ('exit whenever I get an open that is profitable basis my entry point') and 'my normal stop loss orders'; which stop applies on the days after entry is not stated"),
          &["X|\"I simply exit whenever I get an open that is profitable basis my entry point. I find being willing to hold positions overnight ... while employing my normal stop loss orders\" (p.51)"],
          "Printed 1991-2001 summaries (3 contracts/trade): S&P 955 trades 61.0% net $862,839; T-Bonds 1,251 trades 63.1%; NASDAQ 434 trades 63.1% (p.52).", None),
        d("lgvp_tbond_patterns", "Volpat for T-Bonds: basic pattern, gaps, volatility (85% of range)", "Volpat for T-Bonds (p.31-46)",
          NeedsData("T-Bond futures day-session data: rules are written for T-Bonds with 20-tick (1/32) stops; the vault has no bond futures"),
          &["E|Two down closes then a lower day-session open: buy the open; two up closes then a higher open: sell the open (p.33-34)",
            "E|Open above yesterday's high: sell stop one tick under yesterday's high; open below yesterday's low: buy stop one tick above it (p.35-36)",
            "E|Volatility: 85% of yesterday's day-session range, rounded up, from the open; pattern or volatility target, whichever occurs first (p.41-43)",
            "X|Maximum stop 20 ticks (or the other volatility target if closer); exit MOC; short-term variant holds overnight with a limit at entry +/- 2 ticks (p.37-39)"],
          "Near-duplicate entry idea of Gettess30 (jkbwts_gettess30: two down closes, open below the close), with a different exit.", None),
        d("lgvp_discretion", "Additional day trading ideas (exit before the close, trailing stops, trend)", "VOLPAT addenda (p.45-50)",
          NotTestable("discretionary guidelines ('These are all general guidelines rather than hard and fast rules', p.45)"), &["F|General guidelines only (p.45-50)"], "", None),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    catalogue().into_iter().find(|d| d.id == id).map(|d| (d.id, d.name, ""))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    (id == "lgvp_volpat").then(|| vec![
        p("k_close", "Fraction of the 3-day range from the close", 0.70, 0.1, 2.0, 0.05),
        p("k_open", "Fraction of the 3-day range from the open", 0.75, 0.1, 2.0, 0.05),
        p("patterns", "Pattern recognition on (1, book) / off (0 = System #1 only)", 1., 0., 1., 1.),
        p("pat_scope", "Pattern precedence: 0 = first pattern found on either side (book p.14), 1 = per side", 0., 0., 1., 1.),
        p("tick", "Rounding increment (0.05 = S&P / NSE)", 0.05, 0.0, 1.0, 0.01),
        p("path", "1 = real 1-minute bars (default; days without minute data make no trade), 0 = [Not from the book] daily bar path", 1., 0., 1., 1.)]) // intraday wave 2b
}

/// (buy, sell) points for bar i (uses bars <= i-1 and o[i]); NaN when not enough history.
pub(crate) fn volpat_points(x: &Ctx, i: usize, kc: f64, ko: f64, tick: f64, patterns: bool, per_side: bool) -> (f64, f64) {
    if i < 4 { return (f64::NAN, f64::NAN); }
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let w = i - 3..i;
    let hh = w.clone().map(|j| h[j]).fold(f64::MIN, f64::max); let ll = w.clone().map(|j| l[j]).fold(f64::MAX, f64::min);
    let hc = w.clone().map(|j| c[j]).fold(f64::MIN, f64::max); let lc = w.clone().map(|j| c[j]).fold(f64::MAX, f64::min);
    let ttr = (hh - lc).max(hc - ll);
    let (fc, fo) = (roundup(ttr * kc, tick), roundup(ttr * ko, tick));
    let op = o[i];
    let (bc, sc, bo, so) = (c[i - 1] + fc, c[i - 1] - fc, op + fo, op - fo);
    let mut buy = if op > bc { bo } else { bc.min(bo) };
    let mut sell = if op < sc { so } else { sc.max(so) };
    if patterns {
        let (a, b, cc, d) = (i - 1, i - 2, i - 3, i - 4);
        let p3 = (h[a] > h[b] && c[a] < c[b] && op < c[b], l[a] < l[b] && c[a] > c[b] && op > c[b], c[b], c[b]);
        let p2 = (l[b] < l[cc] && l[a] > l[b] && op < c[b], h[b] > h[cc] && h[a] < h[b] && op > c[b], c[b], c[b]);
        let p1 = (c[cc] < c[d] && c[b] > c[cc] && c[a] < c[b] && op < h[a], c[cc] > c[d] && c[b] < c[cc] && c[a] > c[b] && op > l[a], h[a], l[a]);
        let pats = [p3, p2, p1];
        let (pb, ps) = if per_side {
            (pats.iter().find(|q| q.0).map(|q| q.2), pats.iter().find(|q| q.1).map(|q| q.3))
        } else {
            match pats.iter().find(|q| q.0 || q.1) { Some(q) => (q.0.then_some(q.2), q.1.then_some(q.3)), None => (None, None) }
        };
        if let Some(v) = pb { if v < buy { buy = v; } }
        if let Some(v) = ps { if v > sell { sell = v; } }
    }
    (buy, sell)
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if from > to || to >= x.c.len() { return None; }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let (kc, ko, tick, pat, side) = (g("k_close"), g("k_open"), g("tick"), g("patterns") >= 0.5, g("pat_scope") >= 0.5);
    if g("path") >= 0.5 { return Some(oco_minutes(x, from, to, |i| volpat_points(x, i, kc, ko, tick, pat, side))); } // intraday wave 2b
    Some(oco_day(x, from, to, |i| volpat_points(x, i, kc, ko, tick, pat, side)))
}

// ---- intraday wave 2b block: VOLPAT OCO on real 1-minute bars
/// Both stops live from the first minute; the first one touched fills (at the level, or the minute's open when it opened
/// through it) and the other becomes the stop-loss; otherwise market on close. A minute reaching both entry levels without
/// opening through exactly one: order unknown, no trade [wave-1 convention]. Days without minute data make no trade.
fn oco_minutes(x: &Ctx, from: usize, to: usize, lv: impl Fn(usize) -> (f64, f64)) -> Vec<Trade> {
    use crate::intraday::{day_trade, stop_entry_day, Level};
    let mut out = vec![];
    for i in from..=to {
        let (b, s) = lv(i);
        if !b.is_finite() || !s.is_finite() { continue; }
        let Some(m) = crate::intraday_w2b::day(x, i) else { continue };
        let lvls = [Level { up: true, px: b }, Level { up: false, px: s }];
        let Some((k, px, st)) = stop_entry_day(&m, 0, &lvls, &|k, _| Some(lvls[1 - k])) else { continue };
        let dir = if k == 0 { 1.0 } else { -1.0 };
        out.push(match st {
            Some(xp) => day_trade(dir, i, px, i, xp, if k == 0 { "Stopped by the sell point" } else { "Stopped by the buy point" }, "stop", false),
            None => day_trade(dir, i, px, i, x.c[i], "Market on close", "stop", false),
        });
    }
    out
}
// ---- end intraday wave 2b block ----

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn book_example_day4() {
        // p.10-11: days 1-3 -> day 4 open 253.10: BUY 262.15 (from close), SELL 244.35 (from close)
        let v = [(243.55, 244.20, 240.60, 242.15), (243.50, 247.70, 243.40, 246.75), (248.45, 254.15, 248.30, 253.25), (253.10, 254.85, 252.50, 253.55)];
        // prepend one dummy bar so i >= 4 (patterns use day D); dummy chosen so no pattern fires
        let mut r: Vec<Row> = vec![Row { ticker: "T".into(), date: "2024-01-01".into(), open: 240., high: 241., low: 239., close: 240., volume: None }];
        r.extend(v.iter().enumerate().map(|(k, &(o, h, l, c))| Row { ticker: "T".into(), date: format!("2024-01-{:02}", k + 2), open: o, high: h, low: l, close: c, volume: None }));
        let x = Ctx::new(&r, &r, &[]);
        let (b, s) = volpat_points(&x, 4, 0.70, 0.75, 0.05, false, false);
        assert!((b - 262.15).abs() < 1e-6 && (s - 244.35).abs() < 1e-6, "{b} {s}");
    }
}
