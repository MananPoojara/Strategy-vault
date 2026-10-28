//! YT Strategies · Ichimoku: the team's AmiBroker AFLs in "Old Topics/Ichimoku" that trade one index on daily bars
//! (the three "INDEX ATHC RSI ADX" AFLs need a stock universe and are not coded here). Coded from the AFLs as written,
//! with the AFLs' own default settings; there is no book PDF.
//!
//! Ichimoku lines as every one of these AFLs computes them (all from the current and past bars only):
//! - turning (Tenkan) line TL = (HHV(H, 9) + LLV(L, 9)) / 2, base (Kijun) line BL = the same over 26 bars;
//! - span A = (TL + BL) / 2 and span B = (HHV(H, 52) + LLV(L, 52)) / 2 are NOT shifted: the AFLs commented out the
//!   `Ref(.., -26)` versions and compare the close with today's spans, the cloud that the chart plots 26 bars ahead
//!   (the team's "forward looking cloud"). No future bar is read; the forward shift exists only in the plot;
//! - delayed (Chikou) line = Ref(C, -26), the close 26 bars back;
//! - "current cloud" AFLs add Cloud = Ref(Max(span A, span B), -26): the upper cloud plotted under today's bar
//!   (again past bars only).
//!
//! AmiBroker behaviour followed: the AFL loops run from bar 1 over the full history; a Null array element read in a
//! loop is -1e10 (so in the first ~52 bars of the whole history a line that is still Null compares as -1e10, as in
//! AmiBroker); Null propagates through arithmetic and Max/Min; fills at the close unless the AFL sets the sell or
//! cover price; the backtester exits first and then enters on the same bar; trade prices are kept inside the bar's
//! low-high range (AmiBroker's default price bound check), which matters for the AFLs' gap prices.
#![allow(dead_code)]
use crate::books::ytsw::trade;
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "yt_ich";
pub const VER1: &str = "ytich_tk_v1";
pub const VER2: &str = "ytich_tk_v2";
pub const VER3: &str = "ytich_cloud_v3";
pub const HSLPT: &str = "ytich_tk_hslpt";
pub const FWD_LIVE: &str = "ytich_fwd_hsl_live_v1";
pub const FWD_CLOSE: &str = "ytich_fwd_hsl_close_v2";
pub const FWD_RE: &str = "ytich_fwd_reentry_v3";
pub const CUR_LIVE: &str = "ytich_cur_hsl_live_v1";
pub const CUR_CLOSE: &str = "ytich_cur_hsl_close_v2";
pub const CUR_RE: &str = "ytich_cur_reentry_v3";
pub const IDS: [&str; 10] = [VER1, VER2, VER3, HSLPT, FWD_LIVE, FWD_CLOSE, FWD_RE, CUR_LIVE, CUR_CLOSE, CUR_RE];

fn def(id: &'static str, name: &'static str, rules: &'static [&'static str], notes: &'static str) -> StrategyDef {
    StrategyDef { id, name, group: "YT Strategies · Ichimoku", session: 0, dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None }
}

const LINES: &str = "F|Lines: turning line = mid of the 9-bar high-low, base line = mid of the 26-bar high-low, span A = (turning + base) / 2, span B = mid of the 52-bar high-low, both spans unshifted (today's values, the cloud the chart plots 26 bars ahead); delayed line = the close 26 bars back";
const SETUP: &str = "F|Setup (checked every bar, also in a trade): long setup 1 on a turning-line cross above the base line with the close above the cloud; setup 2 on such a cross with the close inside the cloud, becoming setup 3 once the close is above the cloud with turning > base; a close below the cloud clears them (short side mirrored)";
macro_rules! side_note {
    () => {
        " Version 0 runs the AFL's Long Only side, 1 its Short Only side. Template guards as written: no long entry on a bar whose high equals its close, no short entry when the low equals the close; a long exit on a bar whose low equals its close (a cover when the high equals the close) waits for the first later bar without it and then fills at that close."
    };
}

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        def(VER1, "Ichimoku TK Cross with Cloud Setups (Ver 1)", &[
            LINES, SETUP,
            "E|Long: flat, setup 1 or 3 on, turning > base and close > delayed line, at the close; setups cleared. Short: setup 1 or 3, turning < base, close < delayed line",
            "X|Sell on a turning-line cross below the base line; cover on a cross above, at the close; setups cleared",
        ], concat!("ICHIMOKU Conversion Line Base Line Based Entry Exit_Ver 1 (Basic version), coded as the AFL is written.", side_note!())),
        def(VER2, "Ichimoku TK Cross, Simple Setup (Ver 2)", &[
            LINES,
            "F|Setup: any turning-line cross above the base line (short: below) arms the setup; it stays armed until an entry or exit",
            "E|Long: flat, setup armed, close above the cloud (above both spans), turning > base and close > delayed line. Short: close below the cloud, turning < base, close < delayed line",
            "X|Sell on a turning-line cross below the base line; cover on a cross above, at the close",
        ], concat!("ICHIMOKU Conversion Line Base Line Based Entry Exit_Ver 2 (Basic version), coded as the AFL is written: unlike Ver 1, a close back inside or below the cloud after the cross does not cancel the setup.", side_note!())),
        def(VER3, "Ichimoku Cloud Breakout, Base-Line Exit (Ver 3)", &[
            "F|Lines: base line = mid of the 26-bar high-low; cloud top = the larger of span A ((9-bar mid + base) / 2) and span B (52-bar mid), unshifted",
            "E|Long: the close crosses above the cloud top and is above the base line. Short: the close crosses below the cloud TOP (as the AFL is written) and is below the base line",
            "X|Sell when the close is below the base line; cover when it is above, at the close",
        ], concat!("ICHIMOKU Clouds Based Entry and BaseLine Based Exit_Ver 3 (Basic version), coded as the AFL is written. Quirk: the AFL's description says shorts enter below the lower cloud line, but its code uses the upper one (Max span); coded as the code reads.", side_note!())),
        def(HSLPT, "Ichimoku Ver 1 + Hard SL / Profit Target (Long Side)", &[
            LINES, SETUP,
            "E|As Ver 1. Long entry sets a hard stop at the entry close less the Hard SL % (default 1) and a target at the entry close plus the Profit Booking % (default 3)",
            "X|Long: the day's low reaches the hard stop (fill at the stop, or at the open less the buffer % (0.25) when the open is already below it), the day's high reaches the target (fill at the target, or at the open LESS the buffer % on a gap above it, as written), or a turning-line cross below the base line at the close",
            "X|Short side: as Ver 1, no stops",
        ], concat!("ICHIMOKU Conversion Line Base Line Based Entry Exit + HSL+PT(Long Only), coded as the AFL is written. Fill prices are kept inside the day's range (AmiBroker's price bound check). With Profit Booking % 0 the AFL's target stays 0 on the first trade, so it exits on the next bar (as written; not the default).", side_note!())),
        def(FWD_LIVE, "Ichimoku Ver 1 + HSL / PT, Forward Cloud, HSL Live (v1)", &[
            LINES, SETUP,
            "E|As Ver 1 plus close above the cloud (long) / below the cloud (short). Long stop = entry close less Hard SL % (1), target = plus Profit Booking % (3); short stop = entry close plus Hard SL %, target = less Profit Booking %",
            "X|Long: low reaches the stop (fill at the stop, or open less buffer % 0.25 on a gap below), high reaches the target (target, or open less buffer % on a gap above), or a TK cross down at the close",
            "X|Short: high reaches the stop (stop, or open plus buffer % on a gap above), low reaches the target (target, or open plus buffer % on a gap below; when both are hit the target price is used, as written), or a TK cross up at the close",
        ], concat!("ICHIMOKU Conversion Line Base Line Based Entry Exit + HSL+PT_long and short - HSL@LIVE - v1 (Basic Version + Forward looking cloud), coded as the AFL is written. 'Forward cloud' = today's unshifted spans; no future bar is read. Fill prices are kept inside the day's range.", side_note!())),
        def(FWD_CLOSE, "Ichimoku Ver 1 + HSL at Close / PT, Forward Cloud (v2)", &[
            LINES, SETUP,
            "E|As the forward-cloud v1 (close above / below the cloud, stop and target from the entry close)",
            "X|Long: the close at or below the stop (fill at that close), the high reaching the target (target, or open less buffer % 0.25 on a gap above), or a TK cross down at the close",
            "X|Short: the close at or above the stop (at that close), the low reaching the target (target, or open plus buffer % on a gap below), or a TK cross up",
        ], concat!("ICHIMOKU Conversion Line Base Line Based Entry Exit + HSL+PT_long and short - HSL@Close - v2 (Basic Version + Forward looking cloud), coded as the AFL is written. Fill prices are kept inside the day's range.", side_note!())),
        def(FWD_RE, "Ichimoku HSL at Close + Re-entry, Forward Cloud (v3)", &[
            LINES, SETUP,
            "E|As the forward-cloud v2. Re-entry: after a hard-stop exit, within the re-entry period (3 bars), buy when the close is above the high of the original entry bar (short: below its low), with no other condition",
            "X|As the forward-cloud v2. A re-entered trade carries no stop or target (the AFL does not set them again), so it exits on the TK cross only",
        ], concat!("ICHIMOKU Conversion Line Base Line Based Entry Exit + HSL+PT_long and short HSL@close and reentry-v3 (Basic Version + Forward looking cloud), coded as the AFL is written. Quirk: the re-entry is a separate `if`, so a TK-cross exit can fire on the re-entry bar itself; AmiBroker then keeps the position until the AFL's next exit signal. Fill prices are kept inside the day's range.", side_note!())),
        def(CUR_LIVE, "Ichimoku HSL Live + Current-Cloud Filter (v1)", &[
            LINES, SETUP,
            "F|Current cloud = the cloud top 26 bars back (the cloud plotted under today's bar)",
            "E|As the forward-cloud v1; longs also need the close above the current cloud (shorts have no current-cloud condition, as written)",
            "X|Long: as the forward-cloud v1. Short: triggered when the high reaches the stop, the low reaches the target or a TK cross up; the stop price is used only when the close is also at or above the stop, otherwise the fill is the close (as written)",
        ], concat!("ICHIMOKU Conversion Line Base Line Based Entry Exit + HSL+PT_long and short_ CURRENT CLOUD- HSL@Live- v1 (Basic Version + Current Cloud), coded as the AFL is written. Fill prices are kept inside the day's range.", side_note!())),
        def(CUR_CLOSE, "Ichimoku HSL at Close + Current-Cloud Filter (v2)", &[
            LINES, SETUP,
            "F|Current cloud = the cloud top 26 bars back",
            "E|As the forward-cloud v2; longs also need the close above the current cloud",
            "X|As the forward-cloud v2 (stop on the close, target intraday, TK cross)",
        ], concat!("ICHIMOKU Conversion Line Base Line Based Entry Exit + HSL+PT_long and short_CURRENT CLOUD-HSL@close- v2 (Basic Version + Current Cloud), coded as the AFL is written. Fill prices are kept inside the day's range.", side_note!())),
        def(CUR_RE, "Ichimoku HSL at Close + Re-entry, Current Cloud (v3)", &[
            LINES, SETUP,
            "F|Current cloud = the cloud top 26 bars back",
            "E|As the current-cloud v2. Long re-entry after a hard-stop exit, within the re-entry period (3 bars): close above the original entry bar's high, turning > base; it sets a new stop and target. Short re-entry: close below the original entry bar's low, no stop or target",
            "X|As the current-cloud v2",
        ], concat!("ICHIMOKU Conversion Line Base Line Based Entry Exit + HSL+PT_long and short_ CURRENT CLOUD-HSL@close  Reentry - v3 (Basic Version + Current Cloud), coded as the AFL is written. The AFL's delayed-line slider range is 8-18 but its default is 26; the default is kept and the range here is 8-26. Fill prices are kept inside the day's range.", side_note!())),
    ]
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let cur = matches!(id, CUR_LIVE | CUR_CLOSE | CUR_RE);
    let mut v = vec![
        p("side_version", "Version (0 Long Only, 1 Short Only)", 0.0, 0.0, 1.0, 1.0),
        p("tl_prds", "Turning (conversion) line period", 9.0, 1.0, if cur { 15.0 } else { 100.0 }, 1.0),
        p("bl_prds", "Base line period", 26.0, 1.0, if cur { 35.0 } else { 100.0 }, 1.0),
        p("slb_prds", "Span B high-low period", 52.0, if id == CUR_RE { 40.0 } else { 1.0 }, if cur { 65.0 } else { 100.0 }, 1.0),
    ];
    match id {
        VER1 | VER2 | HSLPT | FWD_LIVE | FWD_CLOSE | FWD_RE => v.push(p("dl_prds", "Delayed line period", 26.0, 1.0, 100.0, 1.0)),
        CUR_LIVE => v.push(p("dl_prds", "Delayed line / current cloud period", 26.0, 1.0, 60.0, 1.0)),
        CUR_CLOSE => v.push(p("dl_prds", "Delayed line / current cloud period", 26.0, 1.0, 50.0, 1.0)),
        CUR_RE => v.push(p("dl_prds", "Delayed line / current cloud period", 26.0, 8.0, 26.0, 1.0)),
        VER3 => {}
        _ => return None,
    }
    if !matches!(id, VER1 | VER2 | VER3) {
        let sl_max = match id { CUR_LIVE | CUR_RE => 5.0, CUR_CLOSE => 20.0, _ => 50.0 };
        v.extend([
            p("buffer_pct", "Buffer % (gap fills)", 0.25, 0.0, 10.0, 0.05),
            p("profit_pct", "Profit Booking %", 3.0, 0.0, 50.0, 1.0),
            p("hard_sl_pct", "Hard SL %", 1.0, 0.0, sl_max, 1.0),
        ]);
    }
    match id {
        FWD_RE => v.push(p("reentry_days", "Re-entry period (bars)", 3.0, 2.0, 8.0, 1.0)),
        CUR_RE => v.push(p("reentry_days", "Re-entry period (bars)", 3.0, 0.0, 100.0, 1.0)),
        _ => {}
    }
    Some(v)
}

// ---------------------------------------------------------------- lines

/// AmiBroker's Null (EMPTY) as a loop element reads it.
const NULL: f64 = -1e10;
const BIG: f64 = 9_999_999.0;

/// (HHV(H, n) + LLV(L, n)) / 2, Null until n bars exist.
fn mid_hl(h: &[f64], l: &[f64], n: usize) -> Vec<Option<f64>> {
    let n = n.max(1);
    (0..h.len())
        .map(|i| {
            if i + 1 < n {
                return None;
            }
            let hh = h[i + 1 - n..=i].iter().cloned().fold(f64::MIN, f64::max);
            let ll = l[i + 1 - n..=i].iter().cloned().fold(f64::MAX, f64::min);
            Some((hh + ll) / 2.0)
        })
        .collect()
}

fn both(a: &[Option<f64>], b: &[Option<f64>], f: impl Fn(f64, f64) -> f64) -> Vec<Option<f64>> {
    a.iter().zip(b).map(|(x, y)| Some(f((*x)?, (*y)?))).collect()
}

fn back(a: &[Option<f64>], k: usize) -> Vec<Option<f64>> {
    (0..a.len()).map(|i| if i >= k { a[i - k] } else { None }).collect()
}

fn loopv(a: &[Option<f64>]) -> Vec<f64> {
    a.iter().map(|v| v.unwrap_or(NULL)).collect()
}

/// The AFLs' Ichimoku arrays as loop values (Null = -1e10).
pub(crate) struct Lines {
    pub tl: Vec<f64>,
    pub bl: Vec<f64>,
    pub maxs: Vec<f64>,
    pub mins: Vec<f64>,
    pub delayed: Vec<f64>,
    pub cloud: Vec<f64>,
}

pub(crate) fn lines(h: &[f64], l: &[f64], c: &[f64], tlp: usize, blp: usize, slbp: usize, dlp: usize) -> Lines {
    let tl = mid_hl(h, l, tlp);
    let bl = mid_hl(h, l, blp);
    let sb = mid_hl(h, l, slbp);
    let sa = both(&tl, &bl, |a, b| (a + b) / 2.0);
    let maxs = both(&sa, &sb, f64::max);
    let mins = both(&sa, &sb, f64::min);
    let cs: Vec<Option<f64>> = c.iter().map(|&v| Some(v)).collect();
    Lines { tl: loopv(&tl), bl: loopv(&bl), maxs: loopv(&maxs), mins: loopv(&mins), delayed: loopv(&back(&cs, dlp)), cloud: loopv(&back(&maxs, dlp)) }
}

// ---------------------------------------------------------------- AmiBroker backtest of loop signals

/// One side's loop output: entry / exit flags, the AFL's exit price (default the close) and the exit's reason.
pub(crate) struct Sigs {
    pub dir: f64,
    pub entry: Vec<bool>,
    pub exit: Vec<bool>,
    pub exit_px: Vec<f64>,
    pub why: Vec<&'static str>,
}

impl Sigs {
    pub(crate) fn new(dir: f64, c: &[f64]) -> Self {
        let n = c.len();
        Sigs { dir, entry: vec![false; n], exit: vec![false; n], exit_px: c.to_vec(), why: vec![""; n] }
    }
}

/// Regular backtest in from..=to: entries at the close; an exit is processed before an entry on the same bar; the
/// exit price is kept within the bar's low-high range (AmiBroker's price bound check).
pub(crate) fn backtest(x: &Ctx, s: &Sigs, from: usize, to: usize) -> Vec<Trade> {
    let mut out = Vec::new();
    let mut pos: Option<usize> = None;
    for i in from..=to {
        if let Some(e) = pos {
            if s.exit[i] {
                let px = s.exit_px[i].max(x.l[i]).min(x.h[i]);
                out.push(trade(s.dir, e, x.c[e], i, px, s.why[i], false));
                pos = None;
            }
        }
        if pos.is_none() && s.entry[i] {
            pos = Some(i);
        }
    }
    if let Some(e) = pos {
        out.push(trade(s.dir, e, x.c[e], to, x.c[to], "open", true));
    }
    out
}

// ---------------------------------------------------------------- the AFL loops

#[derive(Clone, Copy, PartialEq, Debug)]
enum V {
    Ver1,
    Ver2,
    Ver3,
    HslPt,
    FwdLive,
    FwdClose,
    FwdRe,
    CurLive,
    CurClose,
    CurRe,
}
use V::*;

impl V {
    fn of(id: &str) -> Option<V> {
        Some(match id {
            VER1 => Ver1,
            VER2 => Ver2,
            VER3 => Ver3,
            HSLPT => HslPt,
            FWD_LIVE => FwdLive,
            FWD_CLOSE => FwdClose,
            FWD_RE => FwdRe,
            CUR_LIVE => CurLive,
            CUR_CLOSE => CurClose,
            CUR_RE => CurRe,
            _ => return None,
        })
    }
    /// Long side has the hard stop and target.
    fn long_stops(self) -> bool {
        !matches!(self, Ver1 | Ver2 | Ver3)
    }
    /// Short side has them too (not the "(Long Only)" HSL+PT AFL).
    fn short_stops(self) -> bool {
        !matches!(self, Ver1 | Ver2 | Ver3 | HslPt)
    }
    fn fwd_entry(self) -> bool {
        matches!(self, FwdLive | FwdClose | FwdRe | CurLive | CurClose | CurRe)
    }
    fn cur(self) -> bool {
        matches!(self, CurLive | CurClose | CurRe)
    }
    fn re(self) -> bool {
        matches!(self, FwdRe | CurRe)
    }
    fn stop_at_close(self) -> bool {
        matches!(self, FwdClose | FwdRe | CurClose | CurRe)
    }
    fn setups(self) -> bool {
        !matches!(self, Ver2 | Ver3)
    }
}

struct Cfg {
    v: V,
    buf: f64,
    profit: f64,
    hsl_pct: f64,
    reentry: f64,
}

fn ich_long(x: &Ctx, ln: &Lines, k: &Cfg) -> Sigs {
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let Lines { tl, bl, maxs, mins, delayed, cloud } = ln;
    let v = k.v;
    let n = c.len();
    let mut s = Sigs::new(1.0, c);
    let (mut pos, mut skip) = (false, false);
    let (mut s1, mut s2, mut s3) = (false, false, false);
    let (mut hsl, mut pt) = (0.0f64, 0.0f64);
    let (mut buyhigh, mut chk, mut dsl) = (0.0f64, false, 0i64);
    let tlup = |i: usize| tl[i] > bl[i] && tl[i - 1] <= bl[i - 1];
    let tldn = |i: usize| tl[i] < bl[i] && tl[i - 1] >= bl[i - 1];
    let bf = |i: usize| h[i] != c[i] && c[i] > 0.0;
    for i in 1..n {
        dsl = if chk { dsl + 1 } else { 0 };
        match v {
            Ver3 => {}
            Ver2 => {
                if tlup(i) {
                    s1 = true;
                }
            }
            _ => {
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
            }
        }
        let mut done = false;
        if v.re() && !pos && chk && c[i] > buyhigh && (dsl as f64) <= k.reentry && (v != CurRe || (tl[i] > bl[i] && bf(i))) {
            s.entry[i] = true;
            pos = true;
            chk = false;
            if v == CurRe {
                if k.profit > 0.0 {
                    pt = (1.0 + k.profit * 0.01) * c[i];
                }
                if k.hsl_pct > 0.0 {
                    hsl = (1.0 - k.hsl_pct * 0.01) * c[i];
                }
                // the current-cloud v3 chains its entries with `else if`: no exit check on this bar
                done = true;
            }
        }
        if done {
            continue;
        }
        let normal = !pos
            && bf(i)
            && match v {
                Ver1 | HslPt => (s1 || s3) && tl[i] > bl[i] && c[i] > delayed[i],
                Ver2 => s1 && c[i] > maxs[i] && tl[i] > bl[i] && c[i] > delayed[i],
                Ver3 => c[i] > maxs[i] && c[i - 1] <= maxs[i - 1] && c[i] > bl[i],
                _ => (s1 || s3) && tl[i] > bl[i] && c[i] > delayed[i] && c[i] > maxs[i] && (!v.cur() || c[i] > cloud[i]),
            };
        let reset = |s1: &mut bool, s2: &mut bool, s3: &mut bool| match v {
            Ver3 => {}
            Ver2 => *s1 = false,
            _ => {
                *s1 = false;
                *s2 = false;
                *s3 = false;
            }
        };
        if normal {
            s.entry[i] = true;
            pos = true;
            if v.long_stops() {
                if k.profit > 0.0 {
                    pt = (1.0 + k.profit * 0.01) * c[i];
                }
                if k.hsl_pct > 0.0 {
                    hsl = (1.0 - k.hsl_pct * 0.01) * c[i];
                }
            }
            reset(&mut s1, &mut s2, &mut s3);
            if v.re() {
                buyhigh = h[i];
                chk = false;
            }
        } else if pos && !skip {
            let gap = |lvl: f64, gapped: bool| if gapped { o[i] - o[i] * k.buf / 100.0 } else { lvl };
            let hit: Option<(f64, &'static str)> = match v {
                Ver1 | Ver2 => tldn(i).then_some((c[i], "Turning line crossed below base line")),
                Ver3 => (c[i] < bl[i]).then_some((c[i], "Close below base line")),
                _ if v.stop_at_close() => {
                    if c[i] <= hsl {
                        Some((c[i], "Hard SL (close)"))
                    } else if h[i] >= pt {
                        Some((gap(pt, o[i] > pt), "Profit target"))
                    } else {
                        tldn(i).then_some((c[i], "Turning line crossed below base line"))
                    }
                }
                _ => {
                    if l[i] <= hsl {
                        Some((gap(hsl, o[i] < hsl), "Hard SL"))
                    } else if h[i] >= pt {
                        Some((gap(pt, o[i] > pt), "Profit target"))
                    } else {
                        tldn(i).then_some((c[i], "Turning line crossed below base line"))
                    }
                }
            };
            if let Some((px, why)) = hit {
                if v.re() && c[i] <= hsl {
                    chk = true;
                }
                if l[i] != c[i] {
                    s.exit[i] = true;
                    s.exit_px[i] = px;
                    s.why[i] = why;
                    pos = false;
                    reset(&mut s1, &mut s2, &mut s3);
                    if v.long_stops() {
                        hsl = 0.0;
                        pt = BIG;
                    }
                } else {
                    skip = true;
                }
            }
        } else if pos && skip && l[i] != c[i] {
            s.exit[i] = true;
            s.why[i] = "Deferred exit (earlier bar closed at its low)";
            pos = false;
            skip = false;
            reset(&mut s1, &mut s2, &mut s3);
            if v.long_stops() {
                hsl = 0.0;
                pt = BIG;
            }
        }
    }
    s
}

fn ich_short(x: &Ctx, ln: &Lines, k: &Cfg) -> Sigs {
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let Lines { tl, bl, maxs, mins, delayed, .. } = ln;
    let v = k.v;
    let n = c.len();
    let mut s = Sigs::new(-1.0, c);
    let (mut pos, mut skip) = (false, false);
    let (mut s1, mut s2, mut s3) = (false, false, false);
    let (mut hsl, mut pt) = (0.0f64, 0.0f64);
    let (mut selllow, mut chk, mut dsl) = (0.0f64, false, 0i64);
    let tlup = |i: usize| tl[i] > bl[i] && tl[i - 1] <= bl[i - 1];
    let tldn = |i: usize| tl[i] < bl[i] && tl[i - 1] >= bl[i - 1];
    let sf = |i: usize| l[i] != c[i] && c[i] > 0.0;
    for i in 1..n {
        dsl = if chk { dsl + 1 } else { 0 };
        match v {
            Ver3 => {}
            Ver2 => {
                if tldn(i) {
                    s1 = true;
                }
            }
            _ => {
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
            }
        }
        // both v3 AFLs: a separate `if`, no other condition, no new stop or target
        if v.re() && !pos && chk && c[i] < selllow && (dsl as f64) <= k.reentry {
            s.entry[i] = true;
            pos = true;
            chk = false;
            hsl = 99_999_999.0;
        }
        let normal = !pos
            && sf(i)
            && match v {
                Ver1 | HslPt => (s1 || s3) && tl[i] < bl[i] && c[i] < delayed[i],
                Ver2 => s1 && c[i] < mins[i] && tl[i] < bl[i] && c[i] < delayed[i],
                Ver3 => c[i] < maxs[i] && c[i - 1] >= maxs[i - 1] && c[i] < bl[i],
                _ => (s1 || s3) && tl[i] < bl[i] && c[i] < delayed[i] && c[i] < mins[i],
            };
        let reset = |s1: &mut bool, s2: &mut bool, s3: &mut bool| match v {
            Ver3 => {}
            Ver2 => *s1 = false,
            _ => {
                *s1 = false;
                *s2 = false;
                *s3 = false;
            }
        };
        if normal {
            s.entry[i] = true;
            pos = true;
            if v.short_stops() {
                if k.hsl_pct > 0.0 {
                    hsl = c[i] * (1.0 + k.hsl_pct / 100.0);
                }
                if k.profit > 0.0 {
                    pt = c[i] * (1.0 - k.profit / 100.0);
                }
            }
            reset(&mut s1, &mut s2, &mut s3);
            if v.re() {
                selllow = l[i];
                chk = false;
            }
        } else if pos && !skip {
            let up_gap = |lvl: f64, gapped: bool| if gapped { o[i] + o[i] * k.buf / 100.0 } else { lvl };
            const TK: &str = "Turning line crossed above base line";
            let hit: Option<(f64, &'static str)> = match v {
                Ver1 | Ver2 | HslPt => tlup(i).then_some((c[i], TK)),
                Ver3 => (c[i] > bl[i]).then_some((c[i], "Close above base line")),
                FwdLive => {
                    if h[i] >= hsl || l[i] <= pt || tlup(i) {
                        // the AFL sets the stop price, then (a second `if`) the target price over it
                        let mut r = (c[i], TK);
                        if h[i] >= hsl {
                            r = (up_gap(hsl, o[i] > hsl), "Hard SL");
                        }
                        if l[i] <= pt {
                            r = (up_gap(pt, o[i] < pt), "Profit target");
                        }
                        Some(r)
                    } else {
                        None
                    }
                }
                CurLive => {
                    if h[i] >= hsl || l[i] <= pt || tlup(i) {
                        Some(if c[i] >= hsl {
                            (up_gap(hsl, o[i] > hsl), "Hard SL")
                        } else if l[i] <= pt {
                            (up_gap(pt, o[i] < pt), "Profit target")
                        } else if h[i] >= hsl {
                            (c[i], "Hard SL touched (filled at the close, as written)")
                        } else {
                            (c[i], TK)
                        })
                    } else {
                        None
                    }
                }
                _ => {
                    if c[i] >= hsl {
                        Some((c[i], "Hard SL (close)"))
                    } else if l[i] <= pt {
                        Some((up_gap(pt, o[i] < pt), "Profit target"))
                    } else {
                        tlup(i).then_some((c[i], TK))
                    }
                }
            };
            if let Some((px, why)) = hit {
                if v.re() && c[i] >= hsl {
                    chk = true;
                }
                if h[i] != c[i] {
                    s.exit[i] = true;
                    s.exit_px[i] = px;
                    s.why[i] = why;
                    pos = false;
                    reset(&mut s1, &mut s2, &mut s3);
                    if v.short_stops() {
                        pt = 0.0;
                        hsl = BIG;
                    }
                } else {
                    skip = true;
                }
            }
        } else if pos && skip && h[i] != c[i] {
            s.exit[i] = true;
            s.why[i] = "Deferred cover (earlier bar closed at its high)";
            pos = false;
            skip = false;
            reset(&mut s1, &mut s2, &mut s3);
            if v.short_stops() {
                pt = 0.0;
                hsl = BIG;
            }
        }
    }
    s
}

// ---------------------------------------------------------------- run

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let v = V::of(id)?;
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(0.0, |d| d.def));
    let gu = |k: &str| g(k).round().max(1.0) as usize;
    let n = x.c.len();
    if n == 0 || from > to || to >= n {
        return Some(Vec::new());
    }
    let dl = if v == Ver3 { 26 } else { g("dl_prds").round().max(0.0) as usize };
    let ln = lines(&x.h, &x.l, &x.c, gu("tl_prds"), gu("bl_prds"), gu("slb_prds"), dl);
    let k = Cfg { v, buf: g("buffer_pct"), profit: g("profit_pct"), hsl_pct: g("hard_sl_pct"), reentry: g("reentry_days") };
    let s = if g("side_version").round() as i64 == 1 { ich_short(x, &ln, &k) } else { ich_long(x, &ln, &k) };
    Some(backtest(x, &s, from, to))
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let d = catalogue().into_iter().find(|d| d.id == id)?;
    Some((d.id, d.name, "Ichimoku"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::books::ytmr::day_number;
    use crate::Row;

    fn civil(z: i64) -> String {
        let z = z + 719468;
        let era = z.div_euclid(146097);
        let doe = z - era * 146097;
        let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d = doy - (153 * mp + 2) / 5 + 1;
        let m = if mp < 10 { mp + 3 } else { mp - 9 };
        format!("{:04}-{:02}-{:02}", yoe + era * 400 + i64::from(m <= 2), m, d)
    }
    /// Consecutive weekdays from Monday 2024-01-01; open = close, high = close + 1, low = close - 1.
    pub(crate) fn rows(closes: &[f64]) -> Vec<Row> {
        let mut d = day_number("2024-01-01").unwrap();
        closes
            .iter()
            .map(|&c| {
                let z = d;
                d += if (z + 4).rem_euclid(7) == 5 { 3 } else { 1 };
                Row { ticker: "T".into(), date: civil(z), open: c, high: c + 1.0, low: c - 1.0, close: c, volume: None }
            })
            .collect()
    }
    fn go(id: &str, r: &[Row], kv: &[(&str, f64)]) -> Vec<(usize, f64, usize, f64, f64)> {
        let x = Ctx::new(r, r, &[]);
        // short periods so a few bars show the rules: TL 1 bar (= the mid point), BL and span B 2 bars, delayed 1 bar
        let mut p: Params = [("tl_prds", 1.0), ("bl_prds", 2.0), ("slb_prds", 2.0), ("dl_prds", 1.0)].iter().map(|(k, v)| (k.to_string(), *v)).collect();
        p.extend(kv.iter().map(|(k, v)| (k.to_string(), *v)));
        run(id, &x, 0, r.len() - 1, &p).unwrap().into_iter().map(|t| (t.legs[0].i, t.legs[0].px, t.exit_i, (t.exit_px * 1e6).round() / 1e6, t.dir)).collect()
    }
    const UP: [f64; 8] = [100.0, 100.0, 100.0, 102.0, 104.0, 106.0, 104.0, 102.0];
    const DN: [f64; 8] = [100.0, 100.0, 100.0, 98.0, 96.0, 94.0, 96.0, 98.0];

    #[test]
    fn lines_are_unshifted_and_current_cloud_looks_back() {
        let h = [10.0, 12.0, 14.0, 13.0];
        let l = [8.0, 9.0, 11.0, 10.0];
        let c = [9.0, 11.0, 13.0, 12.0];
        let ln = lines(&h, &l, &c, 1, 2, 3, 1);
        assert_eq!(ln.tl, vec![9.0, 10.5, 12.5, 11.5]);
        assert_eq!(ln.bl, vec![NULL, 10.0, 11.5, 12.0]);
        // span A = (TL + BL) / 2, span B = 3-bar mid (Null on bars 0-1): max / min Null until both exist
        assert_eq!(ln.maxs, vec![NULL, NULL, 12.0, 11.75]);
        assert_eq!(ln.mins, vec![NULL, NULL, 11.0, 11.5]);
        assert_eq!(ln.delayed, vec![NULL, 9.0, 11.0, 13.0]);
        assert_eq!(ln.cloud, vec![NULL, NULL, NULL, 12.0]);
    }

    #[test]
    fn tk_cross_versions_long_and_short() {
        // bar 3: TL 102 crosses above BL 101 with the close above the cloud (101.5) and above the delayed close;
        // bar 6: TL 104 < BL 105 after TL >= BL -> sell at the close
        for id in [VER1, VER2, VER3, FWD_CLOSE, CUR_CLOSE] {
            assert_eq!(go(id, &rows(&UP), &[("profit_pct", 50.0), ("hard_sl_pct", 50.0)]), vec![(3, 102.0, 6, 104.0, 1.0)], "{id}");
        }
        for id in [VER1, VER2, VER3, HSLPT, FWD_CLOSE] {
            assert_eq!(go(id, &rows(&DN), &[("side_version", 1.0), ("profit_pct", 50.0), ("hard_sl_pct", 50.0)]), vec![(3, 98.0, 6, 96.0, -1.0)], "{id}");
        }
    }

    #[test]
    fn live_stops_and_gap_prices() {
        // long at 102: target 105.06; bar 5 opens 106 above it -> open less 0.25% = 105.735 (inside 105-107)
        for id in [HSLPT, FWD_LIVE, CUR_LIVE, FWD_CLOSE] {
            assert_eq!(go(id, &rows(&UP), &[]), vec![(3, 102.0, 5, 105.735, 1.0)], "{id}");
        }
        // short at 98: target 95.06, bar 4's low 95 reaches it, open 96 is not below it -> filled at 95.06
        for id in [FWD_LIVE, CUR_LIVE, FWD_CLOSE, CUR_CLOSE] {
            assert_eq!(go(id, &rows(&DN), &[("side_version", 1.0)]), vec![(3, 98.0, 4, 95.06, -1.0)], "{id}");
        }
        // a low stop gapped through: long at 102 with stop 100.98; bar 4 opens 99 (low 98) -> 99 x 0.9975 = 98.7525
        let mut r = rows(&[100.0, 100.0, 100.0, 102.0, 99.0]);
        r[4].low = 98.0;
        assert_eq!(go(FWD_LIVE, &r, &[]), vec![(3, 102.0, 4, 98.7525, 1.0)]);
        // the same gap clamped to the bar's low when the open is the low
        r[4].low = 99.0;
        r[4].close = 99.5;
        assert_eq!(go(FWD_LIVE, &r, &[]), vec![(3, 102.0, 4, 99.0, 1.0)]);
    }

    #[test]
    fn reentry_after_close_stop() {
        // buy 102 (bar 3, high 103); bar 4 closes 100.5 <= stop 100.98 -> out; bar 5 closes 104 > 103 -> re-entry.
        // forward v3: the re-entered trade has no target, so bar 6's high 109 (> 104 x 1.03) does not exit it; the TK
        // cross down on bar 7 does. Current v3 sets a new target: out at bar 6 at open 108 less 0.25% (gap above)
        let r = rows(&[100.0, 100.0, 100.0, 102.0, 100.5, 104.0, 108.0, 106.0]);
        assert_eq!(go(FWD_RE, &r, &[]), vec![(3, 102.0, 4, 100.5, 1.0), (5, 104.0, 7, 106.0, 1.0)]);
        assert_eq!(go(CUR_RE, &r, &[]), vec![(3, 102.0, 4, 100.5, 1.0), (5, 104.0, 6, 107.73, 1.0)]);
        assert_eq!(go(FWD_CLOSE, &r, &[]), vec![(3, 102.0, 4, 100.5, 1.0), (5, 104.0, 6, 107.73, 1.0)]);
    }

    #[test]
    fn exit_waits_when_low_equals_close() {
        // bar 6 is the TK cross down but closes at its low: the sell waits for bar 7, at its close
        let mut r = rows(&UP);
        r[6].low = 104.0;
        assert_eq!(go(VER1, &r, &[]), vec![(3, 102.0, 7, 102.0, 1.0)]);
    }

    #[test]
    fn every_strategy_runs() {
        let closes: Vec<f64> = (0..400).map(|k| 1000.0 + 120.0 * ((k as f64) / 17.0).sin() + 40.0 * ((k as f64) / 5.0).cos() + k as f64 * 0.3).collect();
        let r = rows(&closes);
        let x = Ctx::new(&r, &r, &[]);
        for id in IDS {
            assert!(params(id).is_some() && family_of(id).is_some(), "{id}");
            for side in [0.0, 1.0] {
                let p: Params = [("side_version".to_string(), side)].into_iter().collect();
                let t = run(id, &x, 60, r.len() - 1, &p).unwrap();
                assert!(t.iter().all(|t| t.exit_i >= t.legs[0].i && t.legs[0].i >= 60), "{id}");
                assert!(t.iter().filter(|t| t.open).count() <= 1, "{id}");
            }
        }
        assert_eq!(catalogue().len(), IDS.len());
    }
}
