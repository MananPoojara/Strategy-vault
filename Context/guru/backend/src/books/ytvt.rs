//! YT Strategies · Van Tharp: the team's AmiBroker AFLs in "Old Topics/Van Tharp" that trade an index on daily bars.
//! Only the two Index "Channeling" AFLs in the "Without filter" folder (run on spot Nifty / Bank Nifty) are here; the
//! other Van Tharp AFLs and the "With and without filter" versions are stock-universe runs. Coded from the AFLs as
//! written, with their own default settings; there is no book PDF. AmiBroker conventions as in `ytsw.rs` / `ytwh.rs`.
#![allow(dead_code)]
use crate::books::ytsw::sma;
use crate::books::ytwh::{bound, collect, hhv, llv, loop_val, prec2, Ev};
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "yt_vt";
pub const COOL_OFF: &str = "ytvt_channeling_cool_off";
pub const OVERSOLD: &str = "ytvt_channeling";
pub const IDS: [&str; 2] = [COOL_OFF, OVERSOLD];

fn def(id: &'static str, name: &'static str, dir: &'static str, rules: &'static [&'static str], notes: &'static str) -> StrategyDef {
    StrategyDef { id, name, group: "YT Strategies · Van Tharp", session: 0, dir, status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None }
}

const EXITS: [&str; 3] = [
    "X|Checked from the bar after entry, the first of: (a) low <= hard SL (buy - 2%), high >= profit target (buy + 10%) or low <= trailing SL (5% below the highest high since entry, as of the previous bar): filled at the level, or at the open less the 0.25% buffer when the open is beyond it (also on a gap above the target); (b) close below the 200-day MA: at the close; (c) the previous bar's Williams %R(10) above -30: at today's open less the buffer; (d) the 8th bar in the trade: at the close",
    "X|An exit blocked by a circuit bar ((a), (c): open = low = close to 2 decimals; (b), (d): low = close) is taken at the next bar's open less the buffer (when that bar is not a lower circuit)",
    "F|No entry on a bar whose high equals its close, and none on the first bar; no exit on the entry bar and no entry on an exit bar",
];

const COMMON: &str = "The trailing SL % default 5 lies above its declared maximum 4 and is used as written. Inside the AFL loop the 200-day MA is Null (-1e10) for the first 199 bars, so 'close above the MA' holds and 'close below the MA' never does there (AmiBroker loop behaviour). Fills are kept within the bar's range (price bound checking). Dead-stock, penny and turnover options are stock-template settings left at their defaults (off).";

pub fn catalogue() -> Vec<StrategyDef> {
    let _ = COMMON; // repeated in the notes below (they must be &'static str literals)
    vec![
        def(COOL_OFF, "Index Channeling (Williams %R Cool-off)", "Long", &[
            "E|Buy at the close when the close is above its 200-day MA and Williams %R(10) crosses below -80 (below today, above yesterday)",
            EXITS[0], EXITS[1], EXITS[2],
        ], "Index \u{201c}Channeling\u{201d} system Strategy with William %R cool off.afl (Without filter folder), coded as the AFL is written. The trailing SL % default 5 lies above its declared maximum 4 and is used as written. Inside the AFL loop the 200-day MA is Null (-1e10) for the first 199 bars, so 'close above the MA' holds and 'close below the MA' never does there (AmiBroker loop behaviour). Fills are kept within the bar's range (price bound checking). Dead-stock, penny and turnover options are stock-template settings left at their defaults (off)."),
        def(OVERSOLD, "Index Channeling (Williams %R Oversold)", "Long", &[
            "E|Buy at the close when the close is above its 200-day MA and Williams %R(10) is below -80",
            EXITS[0], EXITS[1], EXITS[2],
        ], "Index \u{201c}Channeling\u{201d} system Strategy without William %R cool off.afl (Without filter folder), coded as the AFL is written; its exit days, profit and SL % are Optimize() with the same defaults. The trailing SL % default 5 lies above its declared maximum 4 and is used as written. Inside the AFL loop the 200-day MA is Null (-1e10) for the first 199 bars, so 'close above the MA' holds and 'close below the MA' never does there (AmiBroker loop behaviour). Fills are kept within the bar's range (price bound checking). Dead-stock, penny and turnover options are stock-template settings left at their defaults (off)."),
    ]
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    if id != COOL_OFF && id != OVERSOLD {
        return None;
    }
    Some(vec![
        p("wr_period", "Williams %R period", 10.0, 1.0, 500.0, 1.0),
        p("entry_wr", "Entry Williams %R -value", 80.0, 1.0, 100.0, 1.0),
        p("exit_wr", "Exit Williams %R -value", 30.0, 1.0, 100.0, 1.0),
        p("exit_days", "Exit days count", 8.0, 5.0, 15.0, 1.0),
        p("buffer_pct", "Buffer %", 0.25, 0.0, 5.0, 0.01),
        p("ma_period", "Entry MA period", 200.0, 1.0, 500.0, 1.0),
        p("pt_pct", "Profit booking % (0 off)", 10.0, 0.0, 15.0, 1.0),
        p("hsl_pct", "Hard SL % (0 off)", 2.0, 0.0, 4.0, 1.0),
        p("tsl_pct", "Trailing SL % (0 off)", 5.0, 0.0, 5.0, 1.0),
    ])
}

const BIG: f64 = 9_999_999.0;

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(0.0, |d| d.def));
    let gi = |k: &str| g(k).round() as i64;
    let gu = |k: &str| g(k).round().max(1.0) as usize;
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let n = c.len();
    if n == 0 || from > to || to >= n {
        return Some(Vec::new());
    }
    let wp = gu("wr_period");
    let (hh, ll) = (hhv(h, wp), llv(l, wp));
    let wr: Vec<f64> = (0..n).map(|i| (hh[i] - c[i]) / (hh[i] - ll[i]) * -100.0).collect(); // NaN on a zero range
    let ma: Vec<f64> = sma(c, gu("ma_period")).into_iter().map(loop_val).collect();
    let (ent, ext) = (-g("entry_wr"), -g("exit_wr"));
    let exit_days = gi("exit_days");
    let buf = g("buffer_pct") / 100.0;
    let (pt_pct, hsl_pct, tsl_pct) = (g("pt_pct"), g("hsl_pct"), g("tsl_pct"));
    let cool = id == COOL_OFF;
    let locirc = |i: usize| prec2(o[i]) == prec2(c[i]) && prec2(o[i]) == prec2(l[i]);
    let lc = |i: usize| l[i] == c[i];
    let at_open = |i: usize| o[i] - o[i] * buf;

    let mut evs: Vec<Ev> = Vec::new();
    let mut pos = false;
    let (mut fix_sl, mut trail_sl, mut count, mut skip) = (0.0f64, 0.0f64, 0i64, false);
    let (mut pt, mut hh_af) = (BIG, BIG);
    for i in 1..n {
        let buy_filter = h[i] != c[i] && c[i] > 0.0;
        if !pos && c[i] > ma[i] && wr[i] < ent && (!cool || wr[i - 1] > ent) && buy_filter {
            if pt_pct > 0.0 {
                pt = (1.0 + pt_pct * 0.01) * c[i];
            }
            if hsl_pct > 0.0 {
                fix_sl = (1.0 - hsl_pct * 0.01) * c[i];
            }
            if tsl_pct > 0.0 {
                hh_af = h[i];
                trail_sl = (1.0 - tsl_pct * 0.01) * hh_af;
            }
            pos = true;
            evs.push(Ev { dir: 1.0, e: i, epx: c[i], exit: None });
        } else if pos && !skip {
            if count <= exit_days {
                count += 1;
            }
            if tsl_pct > 0.0 {
                hh_af = h[i].max(hh_af);
            }
            let mut sold: Option<(f64, usize, &'static str)> = None;
            if (l[i] <= fix_sl || h[i] >= pt || l[i] <= trail_sl) && count <= exit_days {
                if !locirc(i) {
                    sold = Some(if l[i] <= fix_sl {
                        (if o[i] < fix_sl { at_open(i) } else { fix_sl }, i, "Hard SL")
                    } else if h[i] >= pt {
                        (if o[i] > pt { at_open(i) } else { pt }, i, "Profit target")
                    } else {
                        (if o[i] < trail_sl { at_open(i) } else { trail_sl }, i, "Trailing SL")
                    });
                } else {
                    skip = true;
                }
            } else if c[i] < ma[i] {
                if !lc(i) {
                    sold = Some((c[i], i, "Close below MA"));
                } else {
                    skip = true;
                }
            } else if wr[i - 1] > ext && count <= exit_days {
                if !locirc(i) {
                    sold = Some((at_open(i), i - 1, "Williams %R exit"));
                } else {
                    skip = true;
                }
            } else if count == exit_days {
                if !lc(i) {
                    sold = Some((c[i], i, "Exit days"));
                } else {
                    skip = true;
                }
            }
            if let Some((px, s, r)) = sold {
                if let Some(v) = evs.last_mut() {
                    v.exit = Some((i, bound(x, i, px), s, r));
                }
                pos = false;
                fix_sl = 0.0;
                trail_sl = 0.0;
                count = 0;
                hh_af = BIG;
                pt = BIG;
            }
            if tsl_pct > 0.0 {
                trail_sl = (1.0 - tsl_pct * 0.01) * hh_af;
            }
        } else if skip && !locirc(i) {
            if let Some(v) = evs.last_mut() {
                v.exit = Some((i, bound(x, i, at_open(i)), i - 1, "Blocked exit, next open"));
            }
            pos = false;
            skip = false;
            fix_sl = 0.0;
            trail_sl = 0.0;
            count = 0;
            hh_af = BIG;
            pt = BIG;
        }
    }
    Some(collect(x, evs, from, to))
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let name = catalogue().into_iter().find(|d| d.id == id)?.name;
    Some((match id {
        COOL_OFF => "ytvt_channeling_cool_off",
        OVERSOLD => "ytvt_channeling",
        _ => return None,
    }, name, "Van Tharp"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::books::ytwh::testbars::*;

    /// Rising bars (close near the high) for a while, then a pullback to the bottom of the 10-bar range.
    fn setup() -> Vec<(f64, f64, f64, f64)> {
        let mut v: Vec<(f64, f64, f64, f64)> = (0..12).map(|k| { let c = 100.0 + k as f64; (c - 0.5, c + 0.5, c - 1.0, c) }).collect();
        // bar 12: close 103 near the 10-bar low (low 102.5 vs high 111.5) -> %R about -94
        v.push((104.0, 104.5, 102.5, 103.0));
        v
    }

    #[test]
    fn ma_filter_blocks_entry() {
        // MA(1) is the close itself: "close above the MA" is never true
        let t = go(run, OVERSOLD, &bars(&setup()), &[("ma_period", 1.0)]);
        assert!(t.is_empty());
        // MA(5) at bar 12 = 108.2 > 103: no entry either
        assert!(go(run, OVERSOLD, &bars(&setup()), &[("ma_period", 5.0)]).is_empty());
    }

    #[test]
    fn entry_while_ma_is_null_and_exits() {
        // MA period 200 with only ~20 bars: the loop compares against Null (-1e10), so the close is "above" it
        let mut v = setup();
        v.push((103.5, 104.5, 103.0, 104.0)); // bar 13: %R yesterday about -94: no exit; no stop hit
        v.push((104.5, 111.6, 104.0, 111.0)); // bar 14: high 111.6 < target 113.3; %R(bar 13) low -> no exit
        v.push((111.0, 112.0, 110.5, 111.5)); // bar 15: %R(bar 14) above -30 -> sell at the open less 0.25%
        let r = bars(&v);
        let t = go(run, OVERSOLD, &r, &[("hsl_pct", 0.0), ("tsl_pct", 0.0)]);
        assert_eq!((t[0].0, t[0].1), (12, 103.0));
        assert_eq!(t[0].2, 15);
        assert!((t[0].3 - 111.0 * 0.9975).abs() < 1e-9);
        // the cool-off version needs %R to cross below -80: bar 11's %R is near 0, so bar 12 also crosses
        let t2 = go(run, COOL_OFF, &r, &[("hsl_pct", 0.0), ("tsl_pct", 0.0)]);
        assert_eq!(t2[0].0, 12);
        // defaults: the 2% hard SL (100.94) and the 5% trailing SL are never reached; same trade
        let t3 = go(run, OVERSOLD, &r, &[]);
        assert_eq!(t3[0].0, 12);
        assert_eq!(t3[0].2, 15);
    }

    #[test]
    fn hard_stop_gap_fills_at_open_less_buffer() {
        let mut v = setup();
        v.push((100.0, 100.5, 99.0, 99.5)); // bar 13 opens 100 < 103 x 0.98 = 100.94 -> open less 0.25%
        let t = go(run, OVERSOLD, &bars(&v), &[]);
        assert_eq!((t[0].0, t[0].2), (12, 13));
        assert!((t[0].3 - 100.0 * 0.9975).abs() < 1e-9);
    }

    #[test]
    fn exit_days_count() {
        let mut v = setup();
        // 8 quiet bars that keep %R low and hit nothing: the 8th bar after entry sells at the close
        for k in 0..9 {
            v.push((103.0, 103.6, 102.9, 103.2 + 0.01 * k as f64));
        }
        let t = go(run, OVERSOLD, &bars(&v), &[("tsl_pct", 0.0)]);
        assert_eq!((t[0].0, t[0].2), (12, 20));
    }

    #[test]
    fn every_strategy_runs() {
        smoke(run, &IDS);
    }
}
