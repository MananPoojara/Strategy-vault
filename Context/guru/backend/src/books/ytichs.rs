//! YT Strategies · Ichimoku (stocks): the three "INDEX ATHC RSI ADX" AFLs in "Old Topics/Ichimoku" (the team's
//! stock versions of the Ichimoku TK-cross AFLs coded for an index in `ytich.rs`). Coded from the AFLs as written,
//! with the AFLs' own default settings; there is no book PDF and nothing here is checked against the team's
//! workbooks.
//!
//! Each AFL is its index counterpart (same Ichimoku lines, setups, stop / target and gap fills, low-close /
//! high-close deferral; see `ytich.rs`) plus stock filters: RSI / ADX / ROC entry thresholds (all off by default),
//! an all-time-high-close (ATHC) regime with a % close swing (default off), turnover / penny / dead-stock rules
//! and, in two of them, a market index filter (default off). As written:
//! - the root AFL and the forward-cloud AFL leave the ATHC array at 0 when the ATHC choice is OFF (their `else`
//!   branch is `ATHCbuysig==1;`, a comparison, not an assignment) and require it for a long entry, so with the
//!   default settings they take no long trade; the current-cloud AFL assigns it (`ATHCbuysig=1;`);
//! - the forward-cloud AFL's long exit reads `FLTbuy`, which that AFL never defines: AmiBroker stops with an error
//!   the first time a long position is checked, so its long side gives no result (the ATHC-OFF default takes no
//!   long trade anyway); its short side runs;
//! - the current-cloud AFL's short entry reads `Deadstock1`, which it never defines: its short side stops with an
//!   error and is not offered;
//! - `deadstock1` (root and forward AFLs) is true when the next 30 bars repeat this bar's open, high, low and close:
//!   it blocks entries and forces exits (reads up to 30 bars ahead, as written); `HHV(C, BarCount)` is the highest
//!   close so far; the in-condition assignment `athcbuysig[i]=0` of the root / forward long exit is false;
//! - the index filter (FILTER TYPE INDEX; default NONE) is computed on NIFTY 50, the market benchmark, in place
//!   of the AFL's NSENIFTY / NSENIFTY1 / NIFTY / NSE500, on the chosen timeframe and expanded with expandLast; the
//!   AFL's Swing% index filter is expanded with `inDaily`, which only lines up on the daily timeframe, so it is
//!   offered there only; before NIFTY data begins the filter is off;
//! - the turnover filter (off) reads OpenInt in the AFL, where the team stored turnover: here close x volume.
#![allow(dead_code)]
use crate::books::ytd7::ma;
use crate::books::ytich::{backtest, lines, Lines, Sigs};
use crate::books::ytsw::{close_swing, compress, expand_last, supertrend, Bars};
use crate::connors::{dmi, rsi, Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "yt_ichs";
pub const ROOT: &str = "ytichs_tk_hslpt_index_athc";
pub const FWD: &str = "ytichs_fwd_hsl_live_index_athc";
pub const CUR: &str = "ytichs_cur_cloud_index_athc";
pub const IDS: [&str; 3] = [ROOT, FWD, CUR];

fn def(id: &'static str, name: &'static str, dir: &'static str, rules: &'static [&'static str], notes: &'static str) -> StrategyDef {
    StrategyDef { id, name, group: "YT Strategies · Ichimoku (stocks)", session: 0, dir, status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None }
}

const LINES: &str = "F|Lines: turning line = mid of the 9-bar high-low, base line = mid of the 26-bar high-low, span A = (turning + base) / 2, span B = mid of the 52-bar high-low, both unshifted; delayed line = the close 26 bars back";
const SETUP: &str = "F|Setups as the index AFLs: setup 1 on a turning-line cross above the base line with the close above the cloud; setup 2 on such a cross inside the cloud, becoming setup 3 once the close is above the cloud with turning > base; a close below the cloud clears them (short mirrored)";
const STOCK_F: &str = "F|Stock filters: no long entry on a bar closing at its high (short: at its low); close above the penny price (0); optional RSI(14) > 30, ADX(8) > 20, ROC(1) above / below a threshold (all off); turnover filter (off)";
const ATHC_F: &str = "F|ATHC regime (choice 'swing', default OFF): on from a close above every earlier close until the 21% close swing turns down";
const UNIVERSE: &str = " Runs on one stock at a time; the team ran it over stock universes (NSE cash / F&O stocks); the ROC ranking (PositionScore, Rank type off by default) and MAXIMUM POSITION (20) are not modelled.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        def(ROOT, "Ichimoku TK + HSL / PT with Index, ATHC, RSI, ADX Filters", "Long/Short", &[
            LINES, SETUP, STOCK_F, ATHC_F,
            "F|Index filter (default NONE): NIFTY 50 ST (monthly, period 20, factor 1, mid point), SMA (200, close vs the previous period's average) or Swing% (bull 44 / bear 23, daily only); Bull market: longs need the index up and exit when it turns down (Bear: the reverse)",
            "E|Long: flat, setup 1 or 3, turning > base, close above the delayed line and the cloud, the filters and the ATHC regime on, not a dead-stock bar; at the close. Stop = entry less Hard SL % (1), target = entry plus Profit % (3). Short (version 1): setup mirrored, close below the delayed line and the cloud, not closing at its low, not a dead-stock bar; stop / target mirrored",
            "X|Long: low reaches the stop (stop, or open less buffer % 0.25 on a gap), high reaches the target (target, or open less buffer % on a gap), the 21% close swing down (always, as written), a TK cross down, the index filter turning, or a dead-stock bar; a bar closing at its low defers the exit to the next bar that does not",
            "X|Short: high reaches the stop, low reaches the target (target price wins when both), a TK cross up or a dead-stock bar; a bar closing at its high defers the cover",
        ], concat!("ICHIMOKU Conversion Line Base Line Based Entry Exit + HSL+PT_long and short_INDEX ATHC RSI ADX.afl (folder root), coded as the AFL is written. With the defaults (Long Only, ATHC OFF) it takes no trade: the ATHC array stays 0 (see module notes); set ATHC on for longs or version 1 for shorts.", " Runs on one stock at a time; the team ran it over stock universes (NSE cash / F&O stocks); the ROC ranking (PositionScore, Rank type off by default) and MAXIMUM POSITION (20) are not modelled.", " The index filter uses NIFTY 50 (the market benchmark) in place of the AFL's NSENIFTY / NSE500 choices; the turnover filter uses close x volume (the AFL reads OpenInt). The dead-stock rule reads 30 bars ahead.")),
        def(FWD, "Ichimoku HSL Live / PT, Forward Cloud, with ATHC, RSI, ADX Filters", "Long/Short", &[
            LINES, SETUP, STOCK_F, ATHC_F,
            "E|Long: as the root AFL without the index filter (needs the ATHC regime). Short (version 1): flat, setup 1 or 3, turning < base, close below the delayed line and the cloud, not closing at its low, not a dead-stock bar; stop = entry plus Hard SL % (1), target = entry less Profit % (3)",
            "X|Long: the AFL's long exit reads an undefined array (FLTbuy), an AmiBroker error: no long result",
            "X|Short: high reaches the stop (stop, or open plus buffer % 0.25 on a gap), low reaches the target (target, or open plus buffer % on a gap; target price wins when both), a TK cross up, or a dead-stock bar; a bar closing at its high defers the cover",
        ], concat!("ICHIMOKU Conversion Line Base Line Based Entry Exit + HSL+PT_long and short - HSL@LIVE - v1_INDEX ATHC RSI ADX.afl (Basic Version + Forward looking cloud), coded as the AFL is written. With the defaults (Long Only) it takes no trade; only its Short Only version runs.", " Runs on one stock at a time; the team ran it over stock universes (NSE cash / F&O stocks); the ROC ranking (PositionScore, Rank type off by default) and MAXIMUM POSITION (20) are not modelled.", " The turnover filter uses close x volume (the AFL reads OpenInt). The dead-stock rule reads 30 bars ahead.")),
        def(CUR, "Ichimoku HSL Live / PT, Current Cloud, with Index, ATHC, RSI, ADX Filters", "Long", &[
            LINES, SETUP, STOCK_F, ATHC_F,
            "F|Current cloud = the cloud top 26 bars back; index filter as the root AFL (default NONE)",
            "E|Long: flat, setup 1 or 3, turning > base, close above the delayed line, the cloud and the current cloud, the filters on (and the ATHC regime when chosen); at the close. Stop = entry less Hard SL % (1), target = entry plus Profit % (3)",
            "X|Long: low reaches the stop (stop, or open less buffer % 0.25 on a gap), high reaches the target (target, or open less buffer % on a gap), a TK cross down, the index filter turning, and with ATHC chosen the 21% close swing down or the regime ending; a bar closing at its low defers the exit",
        ], concat!("ICHIMOKU Conversion Line Base Line Based Entry Exit + HSL+PT_long and short_CURRENT CLOUD_INDEX ATHC RSI ADX.afl (Basic Version + Current Cloud), coded as the AFL is written. Its Short Only version reads an undefined array (Deadstock1), an AmiBroker error, and is not offered. The AFL's delayed-line slider range is 8-18 with default 26; the default is kept.", " Runs on one stock at a time; the team ran it over stock universes (NSE cash / F&O stocks); the ROC ranking (PositionScore, Rank type off by default) and MAXIMUM POSITION (20) are not modelled.", " The index filter uses NIFTY 50 (the market benchmark) in place of the AFL's NSENIFTY / NSE500 choices; the turnover filter uses close x volume (the AFL reads OpenInt).")),
    ]
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum V {
    Root,
    Fwd,
    Cur,
}

fn var(id: &str) -> Option<V> {
    Some(match id {
        ROOT => V::Root,
        FWD => V::Fwd,
        CUR => V::Cur,
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let v = var(id)?;
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let cur = v == V::Cur;
    let mut out = Vec::new();
    if !cur {
        out.push(p("side_version", "Version (0 Long Only, 1 Short Only)", 0.0, 0.0, 1.0, 1.0));
    }
    out.extend([
        p("tl_prds", "Turning (conversion) line period", 9.0, 1.0, if cur { 15.0 } else { 100.0 }, 1.0),
        p("bl_prds", "Base line period", 26.0, 1.0, if cur { 35.0 } else { 100.0 }, 1.0),
        p("slb_prds", "Span B high-low period", 52.0, if cur { 40.0 } else { 1.0 }, if cur { 65.0 } else { 100.0 }, 1.0),
        p("dl_prds", if cur { "Delayed line / current cloud period" } else { "Delayed line period" }, 26.0, if cur { 8.0 } else { 1.0 }, if cur { 26.0 } else { 100.0 }, 1.0),
        p("buffer_pct", "Buffer % (gap fills)", 0.25, 0.0, 10.0, 0.05),
        p("profit_pct", "Profit Booking %", 3.0, 0.0, 50.0, 1.0),
        p("hard_sl_pct", "Hard SL %", 1.0, 0.0, if cur { 5.0 } else { 50.0 }, 1.0),
        p("athc", "ATHC regime (0 OFF, 1 ATHC)", 0.0, 0.0, 1.0, 1.0),
        p("athc_exit", "Swing exit %", 21.0, 2.0, 25.0, 1.0),
        p("rsi_on", "RSI filter (0 off, 1 on)", 0.0, 0.0, 1.0, 1.0),
        p("rsi_n", "RSI period", 14.0, 1.0, 50.0, 1.0),
        p("rsi_val", "RSI above", 30.0, 10.0, 90.0, 1.0),
        p("adx_on", "ADX filter (0 off, 1 on)", 0.0, 0.0, 1.0, 1.0),
        p("adx_n", "ADX period", 8.0, 1.0, 30.0, 1.0),
        p("adx_val", "ADX above", 20.0, 5.0, 90.0, 5.0),
        p("roc_on", "ROC filter (0 off, 1 on)", 0.0, 0.0, 1.0, 1.0),
        p("roc_below", "ROC threshold side (0 above +thr, 1 below -thr)", 0.0, 0.0, 1.0, 1.0),
        p("roc_days", "ROC days", 1.0, 1.0, 14.0, 1.0),
        p("roc_thr", "ROC threshold %", 1.0, 0.0, 100.0, 1.0),
        p("penny", "Penny stock price (close must be above)", 0.0, 0.0, 100.0, 1.0),
        p("turnover_flt", "Turnover filter (0 off, 1 on)", 0.0, 0.0, 1.0, 1.0),
        p("dead_stock_bt", "Dead stock backtest (0 NO, the default; 1 YES: no entry and a forced exit on the last bar)", 0.0, 0.0, 1.0, 1.0),
    ]);
    if v != V::Fwd {
        out.extend([
            p("mkt_flt", "Market filter (0 NONE, 1 INDEX = NIFTY 50)", 0.0, 0.0, 1.0, 1.0),
            p("flt_tf", "Index filter timeframe (1 monthly, 2 weekly, 3 daily)", 1.0, 1.0, 3.0, 1.0),
            p("mkt_scene", "Market condition (0 Bull, 1 Bear)", 0.0, 0.0, 1.0, 1.0),
            p("flt_kind", "Index filter (1 ST, 2 SMA, 3 Swing% on daily only)", 1.0, 1.0, 3.0, 1.0),
            p("flt_point", "Index ST calculation point (1 mid point, 2 close, 3 pivot)", 1.0, 1.0, 3.0, 1.0),
            p("flt_period", "Index ST period", 20.0, 1.0, 20.0, 1.0),
            p("flt_factor", "Index ST factor", 1.0, 0.0, 20.0, 1.0),
            p("flt_ma", "Index SMA period", 200.0, 1.0, 300.0, 1.0),
            p("swing_bear", "Index swing % for bear market (down move)", 23.0, 2.0, 50.0, 1.0),
            p("swing_bull", "Index swing % for bull market (up move)", 44.0, 2.0, 50.0, 1.0),
        ]);
    }
    Some(out)
}

// ---------------------------------------------------------------- filters

/// Ichimoku-AFL turnover thresholds by calendar year (the AFL compares them with OpenInt = turnover).
fn volsig(date: &str) -> f64 {
    let y: i64 = date.get(..4).and_then(|s| s.parse().ok()).unwrap_or(0);
    match y {
        ..=1999 => 30_000.0,
        2000..=2003 => 50_000.0,
        2004 => 200_000.0,
        2005 => 400_000.0,
        2006 => 600_000.0,
        2007..=2009 => 800_000.0,
        2010 => 1_000_000.0,
        2011 => 2_000_000.0,
        2012 => 2_500_000.0,
        2013 => 3_000_000.0,
        2014 => 3_500_000.0,
        2015..=2016 => 4_000_000.0,
        2017 => 5_000_000.0,
        2018 => 6_000_000.0,
        2019 => 7_000_000.0,
        2020 => 8_000_000.0,
        _ => 9_000_000.0,
    }
}

/// `deadstock1`: this bar's O, H, L, C equal those of each of the next 30 bars (false near the end of the data).
fn deadstock(x: &Ctx) -> Vec<bool> {
    let n = x.c.len();
    (0..n)
        .map(|i| i + 30 < n && (1..=30).all(|k| x.o[i + k] == x.o[i] && x.h[i + k] == x.h[i] && x.l[i + k] == x.l[i] && x.c[i + k] == x.c[i]))
        .collect()
}

/// % swing with separate up / down percentages (the AFL's index Swing% loop: Up = C + up%, Dn = C - dn%).
fn swing2(c: &[f64], up_pct: f64, dn_pct: f64) -> Vec<i8> {
    let n = c.len();
    let mut up: Vec<f64> = c.iter().map(|v| v + v * up_pct / 100.0).collect();
    let mut dn: Vec<f64> = c.iter().map(|v| v - v * dn_pct / 100.0).collect();
    let mut tr = vec![1i8; n];
    for i in 1..n {
        tr[i] = if c[i] > up[i - 1] { 1 } else if c[i] < dn[i - 1] { -1 } else { tr[i - 1] };
        if tr[i] > 0 && dn[i] < dn[i - 1] {
            dn[i] = dn[i - 1];
        }
        if tr[i] < 0 && up[i] > up[i - 1] {
            up[i] = up[i - 1];
        }
        if tr[i] < 0 && tr[i - 1] > 0 {
            up[i] = c[i] + c[i] * up_pct / 100.0;
        }
        if tr[i] > 0 && tr[i - 1] < 0 {
            dn[i] = c[i] - c[i] * dn_pct / 100.0;
        }
    }
    tr
}

fn ffill(v: &[f64]) -> Vec<f64> {
    let mut last = f64::NAN;
    v.iter().map(|&x| { if x.is_finite() { last = x; } last }).collect()
}

/// The index filter as (FLTBuy = long exit, FLTShort = long entry condition) on daily bars, or None when the
/// setting cannot run (missing NIFTY data, Swing% on a weekly / monthly timeframe).
fn index_filter(x: &Ctx, g: &dyn Fn(&str) -> f64) -> Option<(Vec<bool>, Vec<bool>)> {
    let n = x.c.len();
    let gi = |k: &str| g(k).round() as i64;
    if gi("mkt_flt") != 1 {
        return Some((vec![false; n], vec![true; n]));
    }
    let bc = ffill(x.series("BENCH.Close")?);
    let first = bc.iter().position(|v| v.is_finite())?;
    let bh = ffill(x.series("BENCH.High").unwrap_or(&bc));
    let bl = ffill(x.series("BENCH.Low").unwrap_or(&bc));
    let tf = match gi("flt_tf") { 2 => 'W', 3 => 'D', _ => 'M' };
    let bear = gi("mkt_scene") == 1;
    let d = compress(x, tf);
    let m = d.c.len();
    // the benchmark compressed on the stock's own periods
    let mut b = Bars { h: vec![f64::NAN; m], l: vec![f64::NAN; m], c: vec![f64::NAN; m], last: d.last.clone(), group: d.group.clone() };
    for i in 0..n {
        let k = d.group[i];
        b.h[k] = if b.h[k].is_finite() { b.h[k].max(bh[i]) } else { bh[i] };
        b.l[k] = if b.l[k].is_finite() { b.l[k].min(bl[i]) } else { bl[i] };
        b.c[k] = bc[i];
    }
    let (buy, short): (Vec<bool>, Vec<bool>) = match gi("flt_kind") {
        2 => {
            let ma = ma(&b.c, g("flt_ma").round().max(1.0) as usize);
            let prev = |k: usize| if k == 0 { None } else { ma[k - 1] };
            let up: Vec<bool> = (0..m).map(|k| prev(k).map_or(false, |v| b.c[k] > v)).collect();
            let dn: Vec<bool> = (0..m).map(|k| prev(k).map_or(false, |v| b.c[k] < v)).collect();
            (expand_last(&up, &b), expand_last(&dn, &b))
        }
        3 => {
            if tf != 'D' {
                return None;
            }
            let tr = swing2(&bc, g("swing_bull"), g("swing_bear"));
            (tr.iter().map(|&t| if bear { t == 1 } else { t == -1 }).collect(), tr.iter().map(|&t| if bear { t == -1 } else { t == 1 }).collect())
        }
        _ => {
            let calc: Vec<f64> = (0..m)
                .map(|k| match gi("flt_point") { 2 => b.c[k], 3 => (b.h[k] + b.l[k] + b.c[k]) / 3.0, _ => (b.h[k] + b.l[k]) / 2.0 })
                .collect();
            let f = g("flt_factor");
            let (tr, _) = supertrend(&calc, &b.h, &b.l, &b.c, g("flt_period").round().max(1.0) as usize, f, f);
            let t = expand_last(&tr, &b);
            (t.iter().map(|&t| if bear { t == 1 } else { t == -1 }).collect(), t.iter().map(|&t| if bear { t == -1 } else { t == 1 }).collect())
        }
    };
    let off = |v: Vec<bool>| -> Vec<bool> { v.into_iter().enumerate().map(|(i, b)| i >= first && b).collect() };
    Some((off(buy), off(short)))
}

// ---------------------------------------------------------------- the AFL loops

struct Cfg {
    v: V,
    buf: f64,
    profit: f64,
    hsl_pct: f64,
    athc_on: bool,
}

const BIG: f64 = 9_999_999.0;

#[allow(clippy::too_many_arguments)]
fn ich_long(x: &Ctx, ln: &Lines, k: &Cfg, cash: &[bool], extra: &[bool], athc: &[bool], trend1: &[i8], flt_buy: &[bool], ds1: &[bool], dsss: &[bool]) -> Sigs {
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let Lines { tl, bl, maxs, mins, delayed, cloud } = ln;
    let n = c.len();
    let rf = k.v != V::Cur; // root / forward AFLs: dead-stock rules in the loop
    let mut s = Sigs::new(1.0, c);
    let (mut pos, mut skip) = (false, false);
    let (mut s1, mut s2, mut s3) = (false, false, false);
    let (mut hsl, mut pt) = (0.0f64, 0.0f64);
    let tlup = |i: usize| tl[i] > bl[i] && tl[i - 1] <= bl[i - 1];
    let tldn = |i: usize| tl[i] < bl[i] && tl[i - 1] >= bl[i - 1];
    for i in 1..n {
        if tlup(i) && c[i] > maxs[i] {
            s1 = true;
        } else if !s2 && tlup(i) && c[i] < maxs[i] && c[i] > mins[i] {
            s1 = false;
            s2 = true;
        } else if s2 && c[i] > maxs[i] && tl[i] > bl[i] {
            s2 = false;
            s3 = true;
        } else if (s1 || s2 || s3) && c[i] < mins[i] {
            s1 = false;
            s2 = false;
            s3 = false;
        }
        let entry = !pos
            && (s1 || s3)
            && tl[i] > bl[i]
            && c[i] > delayed[i]
            && c[i] > maxs[i]
            && cash[i]
            && (k.v != V::Cur || c[i] > cloud[i])
            && extra[i]
            && athc[i]
            && !(rf && ds1[i]);
        if entry {
            s.entry[i] = true;
            pos = true;
            if k.profit > 0.0 {
                pt = (1.0 + k.profit * 0.01) * c[i];
            }
            if k.hsl_pct > 0.0 {
                hsl = (1.0 - k.hsl_pct * 0.01) * c[i];
            }
            s1 = false;
            s2 = false;
            s3 = false;
        } else if pos && !skip {
            let swing_out = if rf { trend1[i] == -1 } else { k.athc_on && trend1[i] == -1 };
            let athc_out = !rf && !athc[i];
            let hit = l[i] <= hsl || h[i] >= pt || swing_out || athc_out || tldn(i) || dsss[i] || flt_buy[i] || (rf && ds1[i]);
            if hit {
                if l[i] != c[i] || dsss[i] || (rf && ds1[i]) {
                    let gap = |lvl: f64, gapped: bool| if gapped { o[i] - o[i] * k.buf / 100.0 } else { lvl };
                    let (px, why) = if l[i] <= hsl {
                        (gap(hsl, o[i] < hsl), "Hard SL")
                    } else if h[i] >= pt {
                        (gap(pt, o[i] > pt), "Profit target")
                    } else if dsss[i] {
                        (c[i], "Last bar")
                    } else if rf && ds1[i] {
                        (c[i], "Dead stock (next 30 bars unchanged)")
                    } else if tldn(i) {
                        (c[i], "Turning line crossed below base line")
                    } else if flt_buy[i] {
                        (c[i], "Index filter turned")
                    } else {
                        (c[i], "ATHC swing exit")
                    };
                    s.exit[i] = true;
                    s.exit_px[i] = px;
                    s.why[i] = why;
                    pos = false;
                    s1 = false;
                    s2 = false;
                    s3 = false;
                    hsl = 0.0;
                    pt = BIG;
                } else {
                    skip = true;
                }
            }
        } else if (pos && l[i] != c[i] && skip) || (pos && dsss[i]) || (rf && ds1[i]) {
            if pos {
                s.exit[i] = true;
                s.why[i] = if dsss[i] { "Last bar" } else if rf && ds1[i] { "Dead stock (next 30 bars unchanged)" } else { "Deferred exit (earlier bar closed at its low)" };
            }
            pos = false;
            skip = false;
            s1 = false;
            s2 = false;
            s3 = false;
            hsl = 0.0;
            pt = BIG;
        }
    }
    s
}

fn ich_short(x: &Ctx, ln: &Lines, k: &Cfg, cash: &[bool], ds1: &[bool], dsss: &[bool]) -> Sigs {
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let Lines { tl, bl, maxs, mins, delayed, .. } = ln;
    let n = c.len();
    let mut s = Sigs::new(-1.0, c);
    let (mut pos, mut skip) = (false, false);
    let (mut s1, mut s2, mut s3) = (false, false, false);
    let (mut hsl, mut pt) = (0.0f64, 0.0f64);
    let tlup = |i: usize| tl[i] > bl[i] && tl[i - 1] <= bl[i - 1];
    let tldn = |i: usize| tl[i] < bl[i] && tl[i - 1] >= bl[i - 1];
    for i in 1..n {
        if tldn(i) && c[i] < mins[i] {
            s1 = true;
        } else if !s2 && tldn(i) && c[i] > mins[i] && c[i] < maxs[i] {
            s1 = false;
            s2 = true;
        } else if s2 && c[i] < mins[i] && tl[i] < bl[i] {
            s2 = false;
            s3 = true;
        } else if (s1 || s2 || s3) && c[i] > maxs[i] {
            s1 = false;
            s2 = false;
            s3 = false;
        }
        if !pos && (s1 || s3) && tl[i] < bl[i] && c[i] < delayed[i] && c[i] < mins[i] && cash[i] && !ds1[i] {
            s.entry[i] = true;
            pos = true;
            if k.hsl_pct > 0.0 {
                hsl = c[i] * (1.0 + k.hsl_pct / 100.0);
            }
            if k.profit > 0.0 {
                pt = c[i] * (1.0 - k.profit / 100.0);
            }
            s1 = false;
            s2 = false;
            s3 = false;
        } else if pos && !skip {
            if h[i] >= hsl || l[i] <= pt || tlup(i) || dsss[i] || ds1[i] {
                if h[i] != c[i] || dsss[i] || ds1[i] {
                    let up_gap = |lvl: f64, gapped: bool| if gapped { o[i] + o[i] * k.buf / 100.0 } else { lvl };
                    let mut r = (c[i], if dsss[i] { "Last bar" } else if ds1[i] { "Dead stock (next 30 bars unchanged)" } else { "Turning line crossed above base line" });
                    if h[i] >= hsl {
                        r = (up_gap(hsl, o[i] > hsl), "Hard SL");
                    }
                    if l[i] <= pt {
                        r = (up_gap(pt, o[i] < pt), "Profit target");
                    }
                    s.exit[i] = true;
                    s.exit_px[i] = r.0;
                    s.why[i] = r.1;
                    pos = false;
                    s1 = false;
                    s2 = false;
                    s3 = false;
                    pt = 0.0;
                    hsl = BIG;
                } else {
                    skip = true;
                }
            }
        } else if (pos && h[i] != c[i] && skip) || (pos && dsss[i]) || ds1[i] {
            if pos {
                s.exit[i] = true;
                s.why[i] = if dsss[i] { "Last bar" } else if ds1[i] { "Dead stock (next 30 bars unchanged)" } else { "Deferred cover (earlier bar closed at its high)" };
            }
            pos = false;
            skip = false;
            s1 = false;
            s2 = false;
            s3 = false;
            pt = 0.0;
            hsl = BIG;
        }
    }
    s
}

// ---------------------------------------------------------------- run

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let v = var(id)?;
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(0.0, |d| d.def));
    let gi = |k: &str| g(k).round() as i64;
    let gu = |k: &str| g(k).round().max(1.0) as usize;
    let (h, l, c) = (&x.h, &x.l, &x.c);
    let n = c.len();
    if n == 0 || from > to || to >= n {
        return Some(Vec::new());
    }
    let short = v != V::Cur && gi("side_version") == 1;
    // forward-cloud AFL, long side: its exit check reads an undefined array (AmiBroker error), so no result
    if v == V::Fwd && !short {
        return Some(Vec::new());
    }
    let ln = lines(h, l, c, gu("tl_prds"), gu("bl_prds"), gu("slb_prds"), g("dl_prds").round().max(0.0) as usize);
    let k = Cfg { v, buf: g("buffer_pct"), profit: g("profit_pct"), hsl_pct: g("hard_sl_pct"), athc_on: gi("athc") == 1 };
    let rf = v != V::Cur;
    let ds1 = if rf { deadstock(x) } else { vec![false; n] };
    let dead_bt = gi("dead_stock_bt") == 1;
    let dsss: Vec<bool> = (0..n).map(|i| dead_bt && i == n - 1).collect();
    let penny = g("penny");
    let tflt = gi("turnover_flt") == 1;
    let turnover = |i: usize| !tflt || x.r[i].volume.map_or(false, |vol| vol * c[i] > volsig(&x.r[i].date));
    let buy_sig = |i: usize| if dead_bt { i != n - 1 } else { i != 0 };
    if short {
        let cash: Vec<bool> = (0..n).map(|i| l[i] != c[i] && turnover(i) && c[i] > penny && buy_sig(i) && !ds1[i]).collect();
        let s = ich_short(x, &ln, &k, &cash, &ds1, &dsss);
        return Some(open_last(backtest(x, &s, from, to), x, to));
    }
    let cash: Vec<bool> = (0..n).map(|i| h[i] != c[i] && turnover(i) && c[i] > penny && buy_sig(i) && !ds1[i]).collect();
    // RSI / ADX / ROC entry thresholds
    let r = if gi("rsi_on") == 1 { Some(rsi(c, gu("rsi_n"))) } else { None };
    let a = if gi("adx_on") == 1 { Some(dmi(x.r, gu("adx_n")).1) } else { None };
    let rd = gu("roc_days");
    let (roc_on, below, thr) = (gi("roc_on") == 1, gi("roc_below") == 1, g("roc_thr"));
    let extra: Vec<bool> = (0..n)
        .map(|i| {
            let roc = if i >= rd { (c[i] / c[i - rd] - 1.0) * 100.0 } else { f64::NAN };
            r.as_ref().map_or(true, |r| r[i] > g("rsi_val"))
                && a.as_ref().map_or(true, |a| a[i] > g("adx_val"))
                && (!roc_on || if below { roc < -thr } else { roc > thr })
        })
        .collect();
    let (flt_buy, flt_short) = if v == V::Fwd {
        (vec![false; n], vec![true; n])
    } else {
        match index_filter(x, &g) {
            Some(f) => f,
            None => return Some(Vec::new()),
        }
    };
    let extra: Vec<bool> = (0..n).map(|i| extra[i] && flt_short[i]).collect();
    let trend1 = close_swing(c, g("athc_exit"));
    let athc: Vec<bool> = if k.athc_on {
        let mut a = vec![false; n];
        let mut hcl = c[0];
        for i in 1..n {
            a[i] = if c[i] > hcl { true } else if trend1[i] == -1 && a[i - 1] { false } else { a[i - 1] };
            hcl = hcl.max(c[i]);
        }
        a
    } else {
        // root / forward AFLs: `ATHCbuysig==1;` leaves the array at 0; current cloud: `ATHCbuysig=1;`
        vec![v == V::Cur; n]
    };
    let s = ich_long(x, &ln, &k, &cash, &extra, &athc, &trend1, &flt_buy, &ds1, &dsss);
    Some(open_last(backtest(x, &s, from, to), x, to))
}

/// The dead-stock backtest's forced exit on the data's last bar is reported as a position still open.
fn open_last(mut t: Vec<Trade>, x: &Ctx, to: usize) -> Vec<Trade> {
    for tr in t.iter_mut() {
        if tr.reason == "Last bar" {
            tr.open = true;
            tr.exit_px = x.c[to];
            tr.reason = "open".into();
        }
    }
    t
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let d = catalogue().into_iter().find(|d| d.id == id)?;
    Some((d.id, d.name, "Ichimoku"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::books::ytd7::tests::{bars, ps, wave};

    /// 60 flat bars, a dip, then a strong rally: the turning line crosses above the base line above the cloud.
    fn rally() -> Vec<(f64, f64, f64, f64)> {
        let mut v = Vec::new();
        for k in 0..60 {
            let c = 100.0 + if k % 2 == 0 { 0.5 } else { -0.5 };
            v.push((c - 0.2, c + 1.0, c - 1.0, c));
        }
        for k in 0..10 {
            let c = 99.0 - k as f64 * 0.5;
            v.push((c + 0.2, c + 1.0, c - 1.0, c));
        }
        for k in 0..30 {
            let c = 95.0 + k as f64 * 1.5;
            v.push((c - 0.3, c + 1.0, c - 1.0, c));
        }
        v
    }

    #[test]
    fn current_cloud_long_takes_trades_root_needs_athc() {
        let r = bars(&rally());
        let x = Ctx::new(&r, &r, &[]);
        let n = r.len();
        let t = run(CUR, &x, 0, n - 1, &ps(&[])).unwrap();
        assert!(!t.is_empty());
        assert!(t.iter().all(|t| t.dir > 0.0));
        // (the warm-up bars read still-Null lines as -1e10, as AmiBroker's loop does, so early entries can occur)
        // the rally entry: a 3% target on a rise of 1.5 a bar exits at the target price
        let rt = t.iter().find(|t| t.legs[0].i >= 70).expect("rally entry");
        assert_eq!(rt.reason, "Profit target");
        // root AFL: ATHC OFF leaves the regime array at 0 (as written): no long trade
        assert!(run(ROOT, &x, 0, n - 1, &ps(&[])).unwrap().is_empty());
        let t2 = run(ROOT, &x, 0, n - 1, &ps(&[("athc", 1.0)])).unwrap();
        assert!(!t2.is_empty());
        // forward AFL long side: AmiBroker error, no result
        assert!(run(FWD, &x, 0, n - 1, &ps(&[("athc", 1.0)])).unwrap().is_empty());
    }

    #[test]
    fn dead_stock_blocks_root_entries() {
        let mut v = rally();
        let last = *v.last().unwrap();
        v.extend(std::iter::repeat(last).take(40));
        let r = bars(&v);
        let x = Ctx::new(&r, &r, &[]);
        let ds = deadstock(&x);
        assert!(ds[99] && !ds[98] && !ds[r.len() - 1]);
    }

    #[test]
    fn short_side_runs_on_root_and_forward() {
        let v: Vec<(f64, f64, f64, f64)> = rally().into_iter().map(|(o, h, l, c)| (200.0 - o, 200.0 - l, 200.0 - h, 200.0 - c)).collect();
        let r = bars(&v);
        let x = Ctx::new(&r, &r, &[]);
        for id in [ROOT, FWD] {
            let t = run(id, &x, 0, r.len() - 1, &ps(&[("side_version", 1.0)])).unwrap();
            assert!(!t.is_empty(), "{id}");
            assert!(t.iter().all(|t| t.dir < 0.0));
        }
    }

    #[test]
    fn index_filter_on_monthly_st() {
        let r = wave(400);
        let mut x = Ctx::new(&r, &r, &[]);
        let up: Vec<f64> = (0..r.len()).map(|k| 100.0 + k as f64).collect();
        x.extra.insert("BENCH.Close".into(), up.clone());
        x.extra.insert("BENCH.High".into(), up.iter().map(|v| v + 1.0).collect());
        x.extra.insert("BENCH.Low".into(), up.iter().map(|v| v - 1.0).collect());
        let schema = params(CUR).unwrap();
        let p = ps(&[("mkt_flt", 1.0)]);
        let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap().def);
        let (buy, short) = index_filter(&x, &g).unwrap();
        assert!(short.iter().skip(5).all(|&b| b) && buy.iter().all(|&b| !b), "rising index: up trend (bull market longs allowed)");
        let p = ps(&[("mkt_flt", 1.0), ("flt_kind", 3.0)]);
        let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap().def);
        assert!(index_filter(&x, &g).is_none(), "Swing% filter is daily only");
    }

    #[test]
    fn every_strategy_runs() {
        let r = wave(400);
        for bench in [false, true] {
            let mut x = Ctx::new(&r, &r, &[]);
            if bench {
                for k in ["Open", "High", "Low", "Close"] {
                    x.extra.insert(format!("BENCH.{k}"), r.iter().map(|b| b.close * 1.5).collect());
                }
            }
            for id in IDS {
                for kv in [vec![], vec![("athc", 1.0), ("mkt_flt", 1.0)], vec![("side_version", 1.0), ("dead_stock_bt", 1.0)], vec![("rsi_on", 1.0), ("adx_on", 1.0), ("roc_on", 1.0), ("mkt_flt", 1.0), ("flt_kind", 2.0), ("flt_tf", 2.0)]] {
                    let t = run(id, &x, 60, r.len() - 1, &ps(&kv)).unwrap();
                    assert!(t.iter().all(|t| t.exit_i >= t.legs[0].i && t.legs[0].i >= 60), "{id}");
                    for w in t.windows(2) {
                        assert!(w[1].legs[0].i >= w[0].exit_i, "{id}: overlapping trades");
                    }
                }
            }
        }
        assert_eq!(catalogue().len(), IDS.len());
        for id in IDS {
            assert!(params(id).is_some() && family_of(id).is_some());
        }
    }
}
