//! YT Strategies · Index Channeling (Short Only): the team's AmiBroker AFL in "Old Topics/Index “Channeling” system
//! Strategy without William %R cool off(With and without filter,Short Only)", run on an index on daily bars (the
//! folder's copy sits under "Index Shorting without cooloff"). The two copies of the AFL differ only in Param /
//! Optimize ranges; their defaults are the same, so one id is coded. The long-only Channeling AFLs are in `ytvt.rs`;
//! this module codes only the short side, as written, with the AFL's own default settings; there is no book PDF.
//!
//! AmiBroker behaviour followed (as in `ytvt.rs`):
//! - SetBarsRequired(100000, 0): the loop runs over the whole history; trades are taken only in the test window;
//! - entries and the closing exits fill at the bar's close; stop / target exits at the level, or at Open + the buffer
//!   when the bar opens beyond it; the Williams %R exit at the next Open + the buffer; fills are kept inside the bar's
//!   range (price bound checking);
//! - an exit blocked by an upper-circuit bar (Open = High = Close to 2 decimals) or, for the close exits, a High =
//!   Close bar, is taken at the next bar's Open + the buffer that is not an upper circuit;
//! - inside the loop the 200-day MA is Null (-1e10) on the first 199 bars, so 'close below the MA' never holds there;
//!   HHV / LLV use the bars available so far near the start.
//!
//! The "With ST Filter" option (Param, default "Without Filter") adds the AFL's monthly Supertrend: short only in a
//! down trend, cover in the close-exit branch on an up trend; that branch starts the trailing low at the entry bar's
//! High (as written; the default branch starts it at the Low).
#![allow(dead_code)]
use crate::books::ytsw::{sma, st_filter, tf_of};
use crate::books::ytwh::{bound, collect, hhv, llv, loop_val, prec2, Ev};
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "yt_chs";
pub const SHORT: &str = "ytchs_channeling_short";
pub const IDS: [&str; 1] = [SHORT];

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef {
        id: SHORT,
        name: "Index Channeling Short (Williams %R, no cool-off)",
        group: "YT Strategies · Index Channeling (Short Only)",
        session: 0,
        dir: "Short",
        status: Status::Live,
        needs_volume: false,
        needs_vix: false,
        needs_series: &[],
        book: BOOK,
        rules: &[
            "F|Close < its 200-day MA",
            "F|Williams %R(10) > -20",
            "F|No entry on a bar whose Low = Close, and none on the first bar",
            "E|Short at the Close",
            "X|Checked from the bar after entry; the first of (a) to (d)",
            "X|(a) High ≥ hard SL (short + 2%), Low ≤ profit target (short − 10%), or High ≥ trailing SL (5% above the lowest Low since entry, as of the previous bar), while the trade is at most 8 bars old",
            "X|(a) fills at the level, or at Open + the 0.25% buffer when the Open is beyond it (the target too: a gap below it covers at Open + the buffer)",
            "X|(b) previous bar's Williams %R(10) < -70, while the trade is at most 8 bars old: at today's Open + the buffer",
            "X|(c) Close > 200-day MA, or the 8th bar in the trade: at the Close",
            "X|A cover blocked by an upper-circuit bar (Open = High = Close, 2 decimals; for (c): High = Close) is taken at the next bar's Open + the buffer",
            "F|Option 'With ST Filter' (off by default): short only while the monthly Supertrend(5, 1, mid point) is down; (c) also covers when it turns up",
        ],
        notes: "Index \u{201c}Channeling\u{201d} system Strategy without William %R cool off(With and without filter,Short Only).afl (both copies; they differ only in Param ranges), coded as the AFL is written. Defaults outside their declared ranges are used as written: Williams %R period 10 (range 22-26 in one copy), entry -20 (9-12), exit -70 (Optimize 60-68), MA 200 (50-70), profit 10% (0-8). After the 8th bar the stop, target and %R exits stop being checked (the count stops at 9) and only Close > MA covers, as in the AFL. Inside the AFL loop the MA is Null (-1e10) for the first 199 bars of the history, so no short is taken there. The 'With ST Filter' branch starts the trailing low at the entry bar's High (the default branch uses the Low), as written. Fills are kept within the bar's range. Dead-stock, penny and turnover options are stock-template settings left at their defaults (off).",
        claim: None,
    }]
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    if id != SHORT {
        return None;
    }
    Some(vec![
        p("filter", "Filter (0 without filter, 1 with ST filter)", 0.0, 0.0, 1.0, 1.0),
        p("st_tf", "ST filter timeframe (1 monthly, 2 weekly, 3 daily)", 1.0, 1.0, 3.0, 1.0),
        p("st_period", "ST filter period", 5.0, 1.0, 20.0, 1.0),
        p("st_factor", "ST filter factor", 1.0, 0.0, 20.0, 1.0),
        p("st_point", "ST calculation point (1 mid point, 2 close, 3 pivot)", 1.0, 1.0, 3.0, 1.0),
        p("wr_period", "Williams %R period", 10.0, 1.0, 500.0, 1.0),
        p("entry_wr", "Entry Williams %R -value", 20.0, 1.0, 100.0, 1.0),
        p("exit_wr", "Exit Williams %R -value", 70.0, 1.0, 100.0, 1.0),
        p("exit_days", "Exit days count", 8.0, 0.0, 1000.0, 1.0),
        p("buffer_pct", "Buffer %", 0.25, 0.0, 5.0, 0.01),
        p("ma_period", "Entry MA period", 200.0, 1.0, 500.0, 1.0),
        p("pt_pct", "Profit booking % (0 off)", 10.0, 0.0, 15.0, 1.0),
        p("hsl_pct", "Hard SL % (0 off)", 2.0, 0.0, 8.0, 1.0),
        p("tsl_pct", "Trailing SL % (0 off)", 5.0, 0.0, 8.0, 1.0),
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
    let with_st = gi("filter") == 1;
    let mtrend: Vec<i8> = if with_st { st_filter(x, tf_of(gi("st_tf"), true), gi("st_point"), gu("st_period"), g("st_factor")) } else { vec![0; n] };
    let upcirc = |i: usize| prec2(o[i]) == prec2(c[i]) && prec2(o[i]) == prec2(h[i]);
    let uc = |i: usize| h[i] == c[i];
    let at_open = |i: usize| o[i] + o[i] * buf;

    let mut evs: Vec<Ev> = Vec::new();
    let mut pos = false;
    let (mut pt, mut count, mut skip) = (0.0f64, 0i64, false);
    let (mut fix_sl, mut trail_sl, mut ll_af) = (BIG, BIG, BIG);
    for i in 1..n {
        // ShortFilter: Low != Close, Close > penny (0), not the first bar
        let short_filter = l[i] != c[i] && c[i] > 0.0;
        if !pos && c[i] < ma[i] && wr[i] > ent && (!with_st || mtrend[i] == -1) && short_filter {
            if pt_pct > 0.0 {
                pt = (1.0 - pt_pct * 0.01) * c[i];
            }
            if hsl_pct > 0.0 {
                fix_sl = (1.0 + hsl_pct * 0.01) * c[i];
            }
            if tsl_pct > 0.0 {
                ll_af = if with_st { h[i] } else { l[i] };
                trail_sl = (1.0 + tsl_pct * 0.01) * ll_af;
            }
            pos = true;
            evs.push(Ev { dir: -1.0, e: i, epx: c[i], exit: None });
        } else if pos && !skip {
            if count <= exit_days {
                count += 1;
            }
            if tsl_pct > 0.0 {
                ll_af = l[i].min(ll_af);
            }
            let mut cov: Option<(f64, usize, &'static str)> = None;
            if (h[i] >= fix_sl || l[i] <= pt || h[i] >= trail_sl) && count <= exit_days {
                if !upcirc(i) {
                    cov = Some(if h[i] >= fix_sl {
                        (if o[i] > fix_sl { at_open(i) } else { fix_sl }, i, "Hard SL")
                    } else if l[i] <= pt {
                        (if o[i] < pt { at_open(i) } else { pt }, i, "Profit target")
                    } else {
                        (if o[i] > trail_sl { at_open(i) } else { trail_sl }, i, "Trailing SL")
                    });
                } else {
                    skip = true;
                }
            } else if wr[i - 1] < ext && count <= exit_days {
                if !upcirc(i) {
                    cov = Some((at_open(i), i - 1, "Williams %R exit"));
                } else {
                    skip = true;
                }
            } else if c[i] > ma[i] || count == exit_days || (with_st && mtrend[i] == 1) {
                if !uc(i) {
                    let why = if c[i] > ma[i] {
                        "Close above MA"
                    } else if count == exit_days {
                        "Exit days"
                    } else {
                        "ST filter turned up"
                    };
                    cov = Some((c[i], i, why));
                } else {
                    skip = true;
                }
            }
            if let Some((px, s, r)) = cov {
                if let Some(v) = evs.last_mut() {
                    v.exit = Some((i, bound(x, i, px), s, r));
                }
                pos = false;
                pt = 0.0;
                count = 0;
                ll_af = BIG;
                fix_sl = BIG;
                trail_sl = BIG;
            }
            if tsl_pct > 0.0 {
                trail_sl = (1.0 + tsl_pct * 0.01) * ll_af;
            }
        } else if skip && !upcirc(i) {
            if let Some(v) = evs.last_mut() {
                v.exit = Some((i, bound(x, i, at_open(i)), i - 1, "Blocked cover, next open"));
            }
            pos = false;
            skip = false;
            pt = 0.0;
            count = 0;
            ll_af = BIG;
            fix_sl = BIG;
            trail_sl = BIG;
        }
    }
    Some(collect(x, evs, from, to))
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let d = catalogue().into_iter().find(|d| d.id == id)?;
    Some((d.id, d.name, "Index Channeling (Short Only)"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::books::ytwh::testbars::*;

    /// Falling bars (close near the low) for a while, then a bounce to the top of the 10-bar range.
    fn setup() -> Vec<(f64, f64, f64, f64)> {
        let mut v: Vec<(f64, f64, f64, f64)> = (0..12).map(|k| { let c = 120.0 - k as f64; (c + 0.5, c + 1.0, c - 0.5, c) }).collect();
        // bar 12: close 116 near the 10-bar high (high 117.5 vs low 108.5) -> %R about -17
        v.push((115.0, 116.5, 114.5, 116.0));
        v
    }

    #[test]
    fn ma_blocks_and_null_ma_blocks() {
        // the MA(200) is Null (-1e10) on these bars: close below it never holds -> no short
        assert!(go(run, SHORT, &bars(&setup()), &[]).is_empty());
        // MA(1) = the close: never below it
        assert!(go(run, SHORT, &bars(&setup()), &[("ma_period", 1.0)]).is_empty());
    }

    /// 30 falling bars (close near the low), then bar 30 bounces to the top of the 10-bar range: %R(10) = -15,
    /// close 178.5 < MA(30) = 184.78 -> short at 178.5.
    fn short_setup() -> Vec<(f64, f64, f64, f64)> {
        let mut w: Vec<(f64, f64, f64, f64)> = (0..30).map(|k| { let c = 200.0 - k as f64; (c + 0.5, c + 1.0, c - 0.5, c) }).collect();
        w.push((171.0, 179.0, 170.0, 178.5));
        w
    }
    const MA30: (&str, f64) = ("ma_period", 30.0);

    #[test]
    fn short_and_williams_exit() {
        let mut w = short_setup();
        w.push((178.0, 178.5, 177.0, 177.5)); // bar 31: %R(30) -15, no stop
        w.push((177.0, 177.2, 170.5, 170.6)); // bar 32: %R(31) about -17
        w.push((170.5, 171.0, 170.0, 170.2)); // bar 33: %R(32) about -93 < -70 -> cover at the open + 0.25%
        let t = go(run, SHORT, &bars(&w), &[MA30, ("tsl_pct", 0.0)]);
        assert_eq!((t[0].0, t[0].1), (30, 178.5));
        assert_eq!(t[0].2, 33);
        assert!((t[0].3 - 170.5 * 1.0025).abs() < 1e-9);
        // the ST filter: on these few bars the monthly ST is still up (it starts up), so no short
        assert!(go(run, SHORT, &bars(&w), &[MA30, ("filter", 1.0)]).is_empty());
    }

    #[test]
    fn hard_stop_gap_covers_at_open_plus_buffer() {
        let mut w = short_setup();
        w.push((183.0, 184.0, 182.5, 183.5)); // bar 31 opens 183 > 178.5 x 1.02 = 182.07 -> open + 0.25%
        let t = go(run, SHORT, &bars(&w), &[MA30]);
        assert_eq!((t[0].0, t[0].2), (30, 31));
        assert!((t[0].3 - 183.0 * 1.0025).abs() < 1e-9);
    }

    #[test]
    fn exit_days_count() {
        let mut w = short_setup();
        // quiet bars, %R stays near -18, no stop / target, close below the MA: the 8th bar covers at the close
        for k in 0..9 {
            w.push((177.0, 177.6, 176.9, 177.4 - 0.01 * k as f64));
        }
        let t = go(run, SHORT, &bars(&w), &[MA30, ("tsl_pct", 0.0)]);
        assert_eq!((t[0].0, t[0].2), (30, 38));
        assert!((t[0].3 - w[38].3).abs() < 1e-9);
    }

    #[test]
    fn every_strategy_runs() {
        smoke(run, &IDS);
        let r = wave(400);
        let x = Ctx::new(&r, &r, &[]);
        let t = run(SHORT, &x, 0, r.len() - 1, &ps(&[("filter", 1.0)])).unwrap();
        assert!(t.iter().all(|t| t.dir < 0.0));
    }
}
