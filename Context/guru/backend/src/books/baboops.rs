//! Bruce Babcock, "The Best Mechanical Day Trading System I Know" (Reality Based Trading Co., 1997). Book id
//! `baboops`. Source copy: Books/Bruce Babcock/Bruce Babcock - The Best Mechanical DayTrading System I Know.pdf
//! (23 scanned pages; PDF page = printed page). Transcription and checks: library/baboops/.
//!
//! Babcock's tests of Larry Williams' OOPS idea (p.12-19): "IF Open is Greater than Yesterday's High + Gap Ticks,
//! Then SELL at High - Hi/Lo Ticks; IF Open is Less than Yesterday's Low - Gap Ticks, Then BUY at Low + Hi/Lo
//! Ticks" (table headers p.16-19), no stops, $100 per trade deducted, 07/01/87-06/27/97, 14 futures markets.
//! S&P tick = 0.05 index points ("15 ticks (75 points)", p.13); the vault uses the same tick on any index
//! (the NSE tick is also 0.05).
use crate::connors::{claim, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "baboops";

const HDR: &str = "F|\"IF Open is Greater than Yesterday's High + Gap Ticks, Then SELL at High - Hi/Lo Ticks; IF Open is Less than Yesterday's Low - Gap Ticks, Then BUY at Low + Hi/Lo Ticks\" (table headers p.16-19): buy stop at prior low + HiLo ticks, sell stop at prior high - HiLo ticks, filled when the day trades there";
const NOSTOP: &str = "X|\"ALL TESTS PERFORMED WITHOUT STOPS\" (p.16-19); \"my research assumed you did not use a stop on the day of entry\" (p.13)";
const NOTE: &str = "Book: S&P 500 futures (and 13 other futures), 07/01/87-06/27/97, $100 per trade deducted; tick 0.05 = $25. Run on any index with the same tick (instrument swap only). The gap is measured from the day's open, so cash-index opens matter: SPX opens before ~1999 are often stale (equal to the prior close), so SPX gap counts in the book's window are not comparable with the futures; SPY (1993-) has traded opens. Larry Williams' day-of-week filters are mentioned but not given (p.14), so they are not coded. Similar trades already in the vault: ss_study_oops_* / lwlts_oops_* / ats_10pct_oops (no minimum gap, other filters); this version adds Babcock's tested minimum gap and entry offset.";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, status, rules, notes, claim| StrategyDef {
        id, name, group, session: 0, dir: "Long/Short", status, needs_volume: false, needs_vix: false,
        needs_series: &[], book: BOOK, rules, notes, claim,
    };
    vec![
        d("baboops_oops", "OOPS day trade · 15-tick gap, entry at yesterday's extreme", "OOPS (p.12-17)", Status::Live,
          &["F|\"You will have a potential trade any time a market opens by gapping above the previous day's high or gapping below the previous day's low\" (p.12); \"I have been trading the 15 tick minimum gap parameter\" (p.15)",
            "E|\"Larry recommends entering at the previous day's extreme, exactly at the point where the gap is closed\" (p.12): HiLo = 0 (p.17 tests -2, 0, 2, 4, 6)",
            HDR, NOSTOP, "X|\"EXIT ON CLOSE OF ENTRY DAY\" (p.16-17)",
            "X|[intraday wave 1] Intraday wave 1: run on real 1-minute bars by default (p.path=1): fills on the first minute that reaches the level, at the level or at that minute's open when it opened through it; out on the official daily close. Instrument: SPX only, because the levels are the book's fixed S&P ticks (15 x 0.05) (no conversion to SPY or NSE prices is invented); from 2014, where SPX daily opens are real, to 2026-08. Days without minute data make no trade. p.path=0 = the old daily-bar reading [Not from the book]."],
          NOTE, claim("148", "62%", "$473 (after $100)", "same day", "S&P 07/01/87-06/27/97, Gap 15, HiLo 0 (p.16-17)")),
        d("baboops_stoops", "STOOPS (tested) · 15-tick gap, enter 2 ticks early, exit next open", "STOOPS (p.13-19)", Status::Live,
          &["F|\"1) Trade when there is an opening gap of at least 15 ticks (75 points)\" (p.13)",
            "E|\"Enter 2 ticks before the gap is closed\" (p.13): HiLo = -2 (p.19 tests -2, 0, 2, 4, 6)",
            HDR, NOSTOP, "X|\"EXIT ON OPEN OF NEXT DAY\" (p.18-19): the hypothetical STOOPS test always exits on the next day's open"],
          NOTE, claim("148", "62%", "$764 (after $100)", "overnight", "S&P 07/01/87-06/27/97, Gap 15, HiLo -2 (p.19)")),
        d("baboops_stoops_rules", "STOOPS as traded (rules 1-3, p.13)", "STOOPS (p.13, 20-21)",
          Status::NeedsData("Intraday bars of the traded market: the protective stop is 'one tick above the opposite extreme at the time of entry' (the day's high/low reached before the entry fill), which daily bars cannot give."),
          &["F|\"1) Trade when there is an opening gap of at least 15 ticks (75 points). Enter 2 ticks before the gap is closed\" (p.13)",
            "X|\"2) Use the following protective stop. One tick above the opposite extreme at the time of entry (the day's high on shorts and the day's low on longs) or $1,500, whichever is less\" (p.13)",
            "X|\"3) Exit on the same day's close if the trade is not profitable at that point. Otherwise exit on the next day's open\" (p.13)",
            "F|\"effective July 8, 1997 ... increase the size of the required opening gap to 150 points or 30 ticks\" (p.15)"],
          "Real-time record Feb 1996-Jun 1997 on S&P futures: 41 trades, $32,375 adjusted / $30,275 actual, no commissions (p.20-21).",
          claim("41", "51% (21 of 41)", "$790 adjusted (no comm.)", "0-1 days", "S&P futures 2/29/96-6/12/97 (p.20)")),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id {
        "baboops_oops" => Some(("baboops_oops", "Babcock OOPS / STOOPS", "Day trade")),
        "baboops_stoops" => Some(("baboops_oops", "Babcock OOPS / STOOPS", "STOOPS tested (next open)")),
        "baboops_stoops_rules" => Some(("baboops_oops", "Babcock OOPS / STOOPS", "STOOPS as traded")),
        _ => None,
    }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let tick = || p("tick", "Tick size in index points (S&P 0.05)", 0.05, 0.01, 1.0, 0.01);
    Some(match id {
        "baboops_oops" => vec![p("gap", "Minimum opening gap, ticks (book 15; tested 5-25)", 15.0, 0.0, 60.0, 5.0),
            p("hilo", "Entry offset from yesterday's extreme, ticks (book 0; tested -2..6)", 0.0, -10.0, 10.0, 1.0), tick(),
            // intraday wave 1
            p("path", "1 = real 1-minute bars (fills at the level or the minute's open; days without minute data make no trade), 0 = [Not from the book] daily bars", 1.0, 0.0, 1.0, 1.0)],
        "baboops_stoops" => vec![p("gap", "Minimum opening gap, ticks (book 15; tested 5-25; 30 from July 1997)", 15.0, 0.0, 60.0, 5.0),
            p("hilo", "Entry offset from yesterday's extreme, ticks (book -2; tested -2..6)", -2.0, -10.0, 10.0, 1.0), tick()],
        _ => return None,
    })
}

/// One trade per gap day: buy (sell) stop at yesterday's low + hilo (high - hilo) ticks when the open gapped more
/// than `gap` ticks beyond it; exit at the close (day trade) or the next bar's open.
fn oops(x: &Ctx, from: usize, to: usize, gap: f64, hilo: f64, next_open: bool) -> Vec<Trade> {
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let mut out = vec![];
    for i in from.max(1)..=to {
        let (dir, lv) = if o[i] < l[i - 1] - gap - 1e-9 { (1.0, l[i - 1] + hilo) }
            else if o[i] > h[i - 1] + gap + 1e-9 { (-1.0, h[i - 1] - hilo) }
            else { continue };
        let hit = if dir > 0.0 { h[i] >= lv - 1e-9 } else { l[i] <= lv + 1e-9 };
        if !hit { continue; }
        // a stop already through at the open (only possible with a large positive hilo) fills at the open
        let px = if dir * (o[i] - lv) >= 0.0 { o[i] } else { lv };
        let (k, ex, why, open) = if !next_open { (i, c[i], "exit on the close", false) }
            else if i + 1 <= to { (i + 1, o[i + 1], "exit on the next open", false) }
            else { (to, c[to], "open", true) };
        out.push(Trade { dir, legs: vec![Leg { i, px, w: 1.0 }], exit_i: k, exit_px: ex, reason: why.into(), entry_kind: "stop",
            signal_i: if next_open && !open { i } else { k }, open, exits: vec![] });
    }
    out
}

// ---- intraday wave 1
/// OOPS day trade on real 1-minute bars: same gap test on the daily open; the stop at yesterday's extreme (+/- hilo)
/// fills on the first minute that reaches it, at the level or at the minute's open; no stop; out on the close.
fn oops_minutes(x: &Ctx, from: usize, to: usize, gap: f64, hilo: f64) -> Vec<Trade> {
    use crate::intraday::{day_trade, stop_entry_day, Level};
    let Some(mm) = x.minutes else { return vec![] };
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let mut out = vec![];
    for i in from.max(1)..=to {
        let (dir, lv) = if o[i] < l[i - 1] - gap - 1e-9 { (1.0, l[i - 1] + hilo) } else if o[i] > h[i - 1] + gap + 1e-9 { (-1.0, h[i - 1] - hilo) } else { continue };
        let Some(b) = mm.day(&x.r[i].date) else { continue };
        let Some((_, px, _)) = stop_entry_day(&b, 0, &[Level { up: dir > 0.0, px: lv }], &|_, _| None) else { continue };
        out.push(day_trade(dir, i, px, i, c[i], "exit on the close", "stop", false));
    }
    out
}
// ---- end intraday wave 1

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if from > to || to >= x.r.len() { return None; }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let t = g("tick");
    // gap and hilo are whole ticks; round the price distances to kill float noise
    let (gap, hilo) = ((g("gap") * t * 1e6).round() / 1e6, (g("hilo") * t * 1e6).round() / 1e6);
    if id == "baboops_oops" && g("path") >= 0.5 { return Some(oops_minutes(x, from, to, gap, hilo)); } // intraday wave 1
    Some(oops(x, from, to, gap, hilo, id == "baboops_stoops"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn bar(d: &str, o: f64, h: f64, l: f64, c: f64) -> Row {
        Row { ticker: "T".into(), date: d.into(), open: o, high: h, low: l, close: c, volume: None }
    }
    #[test]
    fn gap_down_buy_and_gap_up_sell() {
        let r = vec![bar("1997-01-02", 100., 101., 99., 100.),
            bar("1997-01-03", 98.0, 99.5, 97.0, 99.2),   // open 1.00 below low 99 (20 ticks) -> buy at 99 - 0.10 = 98.90
            bar("1997-01-06", 99.5, 100., 99., 99.8),    // next open 99.5
            bar("1997-01-07", 100.5, 100.6, 99.0, 99.3), // open 0.50 above high 100 (10 ticks) -> no trade at 15 ticks
        ];
        let x = Ctx::new(&r, &r, &[]);
        let t = run("baboops_stoops", &x, 0, 3, &Params::new()).unwrap();
        assert_eq!(t.len(), 1);
        assert!((t[0].legs[0].px - 98.9).abs() < 1e-9 && t[0].exit_i == 2 && t[0].exit_px == 99.5);
        let t = run("baboops_oops", &x, 0, 3, &Params::from([("path".to_string(), 0.0)])).unwrap(); // intraday wave 1: daily path
        assert_eq!((t.len(), t[0].legs[0].px, t[0].exit_px), (1, 99.0, 99.2));
        let p = Params::from([("gap".to_string(), 5.0), ("path".to_string(), 0.0)]);
        let t = run("baboops_oops", &x, 0, 3, &p).unwrap();
        assert_eq!((t.len(), t[1].dir, t[1].legs[0].px, t[1].exit_px), (2, -1.0, 100.0, 99.3));
    }
}
