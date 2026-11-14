//! YT Strategies · 2% Scalein - Nifty RedGreen Candle: the team's AmiBroker AFLs in "Old Topics/2% Scalein - Nifty
//! RedGreen Candle", run on spot Nifty on daily bars. Coded from the AFLs as written, with their own default settings;
//! there is no book PDF.
//!
//! The folder holds two AFLs, each twice ("AFL" and "Buy on Red and Sell on Green Candle - To be confirmed + Second
//! Check"; the copies differ only in Param / Optimize ranges and checker names, not in defaults or logic):
//! - Regim 1: long only. Buy a red candle that closes ENTRY % below the previous close while the monthly Supertrend
//!   is up; add (sigScaleIn) on every further such candle; take one piece off (sigScaleOut) on a green candle that
//!   closes EXIT % above the previous close; sell everything when the monthly ST turns down (or, with one piece, on
//!   the green candle).
//! - Regim 2: the same with a "Bull Trades or Bear Trades" ParamList. Its default BULL branch is the Regim 1 AFL
//!   line for line, so both are one id here with the option `trade_ver` (1 BULL = Regim 1 = Regim 2's default,
//!   2 BEAR = Regim 2's bear branch: longs and shorts while the monthly ST is down, no up-trend gate).
//!
//! AmiBroker behaviour followed:
//! - SetBarsRequired(100000, 0): the loop runs over the whole history; positions are opened only in the test
//!   window; all fills at the bar's close; no ExRem (the loop's position counter keeps the signals consistent);
//! - the monthly Supertrend is the team's (as in `ytsw.rs`): mean of the last 5 ATR(1), bands on (H+L)/2, flip and
//!   ratchet, TimeFrameExpand(expandLast): a month's value reaches the daily bars on its last bar;
//! - the BULL branch waits until the expanded ST up-line (TrendUp) has been positive once (the AFL's `abc` flag);
//! - sigScaleIn / sigScaleOut: every entry and every scale-out is SetPositionSize(SIZE/50, spsPercentOfEquity) of
//!   equity. One Trade holds all its entries as legs and its scale-outs as exit pieces: a scale-out sells shares
//!   worth one entry's money at that bar's close (never more than are held; selling the rest closes the trade, and
//!   later adds and exits of that loop position are ignored); the final sell closes the rest. The trade's P&L is the
//!   AmiBroker share arithmetic exactly; its exit price is the share-weighted average of the pieces.
#![allow(dead_code)]
use crate::books::ytsw::{calc_point, compress, expand_last, true_range, tf_of};
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "yt_rgs";
pub const REGIM: &str = "ytrgs_red_green_scale";
pub const IDS: [&str; 1] = [REGIM];

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef {
        id: REGIM,
        name: "Red Candle Buy / Green Candle Sell with Scale-in and Scale-out",
        group: "YT Strategies · 2% Scalein - Nifty RedGreen Candle",
        session: 0,
        dir: "Long/Short",
        status: Status::Live,
        needs_volume: false,
        needs_vix: false,
        needs_series: &[],
        book: BOOK,
        rules: &[
            "F|Monthly Supertrend: period 5, factor 1, on (High + Low) / 2 (the team's ST, expanded to daily bars on each month's last bar)",
            "F|Red-drop candle: Open > Close and Close < previous Close − 1% (ENTRY %); green-rise candle: Close > Open and Close > previous Close + 1% (EXIT %)",
            "F|No buy on a bar whose High = Close, no short on a bar whose Low = Close, none on the first bar",
            "E|BULL (default; Regim 1): once the ST up-line has been positive, buy a red-drop candle at the Close while the ST is up",
            "E|BULL: add (scale in) at the Close on every further red-drop candle while the ST is up",
            "X|BULL, one piece held: sell all at the Close on a green-rise candle or when the ST turns down",
            "X|BULL, more than one piece: scale out one piece on a green-rise candle while the ST is up; sell all when the ST turns down",
            "X|No sell / scale-out on a bar whose Low = Close",
            "E|BEAR (option, Regim 2): while the ST is down, buy red-drop candles (add on more), and short green candles closing 1% (ENTRY %) above the previous close (add on green-rise candles)",
            "X|BEAR longs: as BULL but scale out while the ST is down and sell all when it turns up; shorts: cover on a red-drop candle (one piece) or when the ST turns up, scale out on red-drop candles; no cover on a bar whose High = Close",
            "F|Each entry and each scale-out is one equal piece of equity (the AFL's SIZE / 50 % of equity; 2% at SIZE 100, the folder's 2%)",
        ],
        notes: "Red Candle Buy and Green Candle Sell with Scalein and Scaleout (Regim 1).afl (trade_ver 1) and Red Candle Buy(Cover) and Green Candle Sell(Short) with Scalein and Scaleout (Regim 2).afl (its default BULL branch is identical to Regim 1; trade_ver 2 = its BEAR branch), both copies of each, coded as the AFLs are written. Sizing [declared adaptation]: the AFLs size every entry and scale-out at SIZE / 50 % of equity; at the default SIZE 1 that is 0.02% of equity, under one Nifty unit with RoundLotSize 1, so the defaults could not trade as literally set. Each piece is coded as 2% of the allocation (SIZE 100, the folder's '2% scale-in'), at most 50 pieces; a scale-out sells shares worth 2% of the allocation (AmiBroker: of current equity), fractional shares. The trade's P&L is exact for that share arithmetic; equity marks between the pieces are approximate. In BEAR the short scale-in uses EXIT % and the short cover / scale-out ENTRY %, as written. Loop positions opened before the test window are not traded, nor their adds and exits.",
        claim: None,
    }]
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    if id != REGIM {
        return None;
    }
    Some(vec![
        p("trade_ver", "Trades (1 BULL = Regim 1, 2 BEAR = Regim 2 bear branch)", 1.0, 1.0, 2.0, 1.0),
        p("st_tf", "ST filter timeframe (1 monthly, 2 weekly, 3 daily)", 1.0, 1.0, 3.0, 1.0),
        p("st_period", "ST filter period", 5.0, 1.0, 20.0, 1.0),
        p("st_factor", "ST filter factor", 1.0, 0.0, 20.0, 0.5),
        p("st_point", "ST calculation point (1 mid point, 2 close, 3 pivot)", 1.0, 1.0, 3.0, 1.0),
        p("entry_pct", "Entry %", 1.0, 0.0, 10.0, 0.001),
        p("exit_pct", "Exit %", 1.0, 0.0, 10.0, 0.001),
        p("leg_pct", "Each entry / scale-out, % of the allocation", 2.0, 0.1, 100.0, 0.1),
    ])
}

/// The AFL's ST loop on the period bars: trend (1 / -1) and whether TrendUp (the up-trend line) is > 0.
fn st_trend_up(calc: &[f64], h: &[f64], l: &[f64], c: &[f64], period: usize, f: f64) -> (Vec<i8>, Vec<bool>) {
    let n = c.len();
    let trr = true_range(h, l, c);
    let p = period.max(1);
    let satr: Vec<Option<f64>> = (0..n)
        .map(|i| if i + 1 < p { None } else { trr[i + 1 - p..=i].iter().try_fold(0.0, |s, v| v.map(|v| s + v)).map(|s| s / p as f64) })
        .collect();
    let mut up: Vec<Option<f64>> = (0..n).map(|i| satr[i].map(|s| calc[i] + f * s)).collect();
    let mut dn: Vec<Option<f64>> = (0..n).map(|i| satr[i].map(|s| calc[i] - f * s)).collect();
    let mut tr = vec![1i8; n];
    let mut tu = vec![false; n];
    for i in 1..n {
        tr[i] = match (up[i - 1], dn[i - 1]) {
            (Some(u), _) if c[i] > u => 1,
            (_, Some(d)) if c[i] < d => -1,
            _ => tr[i - 1],
        };
        if let (true, Some(a), Some(b)) = (tr[i] > 0, dn[i], dn[i - 1]) {
            if a < b {
                dn[i] = Some(b);
            }
        }
        if let (true, Some(a), Some(b)) = (tr[i] < 0, up[i], up[i - 1]) {
            if a > b {
                up[i] = Some(b);
            }
        }
        if tr[i] < 0 && tr[i - 1] > 0 {
            up[i] = satr[i].map(|s| calc[i] + f * s);
        }
        if tr[i] > 0 && tr[i - 1] < 0 {
            dn[i] = satr[i].map(|s| calc[i] - f * s);
        }
        // TrendUp = Dn on up-trend bars (the AFL's retro Null on a flip only touches a bar that is already Null)
        tu[i] = tr[i] == 1 && dn[i].is_some_and(|d| d > 0.0);
    }
    (tr, tu)
}

/// One AmiBroker position being built from the loop's signals.
struct Build {
    dir: f64,
    legs: Vec<Leg>,
    outs: Vec<(usize, f64, f64)>, // bar, price, shares
    shares: f64,
}

impl Build {
    fn finish(self, exit_i: usize, px: f64, reason: &str, open: bool) -> Trade {
        let total: f64 = self.legs.iter().map(|l| l.w / l.px).sum();
        let mut exits: Vec<(usize, f64, f64)> = self.outs.iter().map(|&(i, p, q)| (i, p, q / total)).collect();
        let exit_px = if exits.is_empty() {
            px
        } else {
            exits.push((exit_i, px, self.shares / total));
            exits.iter().map(|e| e.1 * e.2).sum()
        };
        Trade { dir: self.dir, legs: self.legs, exit_i, exit_px, reason: reason.into(), entry_kind: "close", signal_i: exit_i, open, exits }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Sig {
    Open,
    In,
    Out,
    Close(&'static str),
}

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
    let b = compress(x, tf_of(gi("st_tf"), true));
    let (tr, tu) = st_trend_up(&calc_point(&b, gi("st_point")), &b.h, &b.l, &b.c, gu("st_period"), g("st_factor"));
    let (mt, tup) = (expand_last(&tr, &b), expand_last(&tu, &b));
    let (ep, xp) = (g("entry_pct"), g("exit_pct"));
    let w = (g("leg_pct") / 100.0).clamp(1e-6, 1.0);
    let max_legs = ((1.0 / w) + 1e-9).floor().max(1.0) as usize;
    let bear = gi("trade_ver") == 2;

    let red_drop = |i: usize| o[i] > c[i] && c[i] < c[i - 1] - c[i - 1] * ep / 100.0;
    let green_up = |i: usize, pct: f64| c[i] > o[i] && c[i] > c[i - 1] + c[i - 1] * pct / 100.0;
    let bf = |i: usize| h[i] != c[i] && c[i] > 0.0; // BuyFilter (not the first bar: the loop starts at 1)
    let sf = |i: usize| l[i] != c[i] && c[i] > 0.0; // ShortFilter
    let lc = |i: usize| l[i] == c[i];
    let uc = |i: usize| h[i] == c[i];

    let mut out = Vec::new();
    let mut cur: Option<Build> = None; // the traded position (only when the loop entry was in the window)
    let (mut pos, mut pos1, mut abc) = (0i64, 0i64, false);
    for i in 1..=to {
        // the loop: (direction, signal) for this bar
        let mut ev: Option<(f64, Sig)> = None;
        if !bear {
            if tup[i] {
                abc = true;
            }
            if pos == 0 && abc && mt[i] == 1 && red_drop(i) && bf(i) {
                ev = Some((1.0, Sig::Open));
                pos = 1;
            } else if pos == 1 {
                if red_drop(i) && mt[i] == 1 && bf(i) {
                    ev = Some((1.0, Sig::In));
                    pos += 1;
                } else if (green_up(i, xp) || mt[i] == -1) && !lc(i) {
                    ev = Some((1.0, Sig::Close(if mt[i] == -1 { "ST turned down" } else { "Green candle up EXIT %" })));
                    pos = 0;
                }
            } else if pos > 1 {
                if red_drop(i) && mt[i] == 1 && bf(i) {
                    ev = Some((1.0, Sig::In));
                    pos += 1;
                } else if green_up(i, xp) && mt[i] == 1 && !lc(i) {
                    ev = Some((1.0, Sig::Out));
                    pos -= 1;
                } else if mt[i] == -1 && !lc(i) {
                    ev = Some((1.0, Sig::Close("ST turned down")));
                    pos = 0;
                }
            }
        } else if pos1 == 0 && pos == 0 && mt[i] == -1 && red_drop(i) && bf(i) {
            ev = Some((1.0, Sig::Open));
            pos1 = 1;
        } else if pos1 == 1 {
            if red_drop(i) && mt[i] == -1 && bf(i) {
                ev = Some((1.0, Sig::In));
                pos1 += 1;
            } else if (green_up(i, xp) || mt[i] == 1) && !lc(i) {
                ev = Some((1.0, Sig::Close(if mt[i] == 1 { "ST turned up" } else { "Green candle up EXIT %" })));
                pos1 = 0;
            }
        } else if pos1 > 1 {
            if red_drop(i) && mt[i] == -1 && bf(i) {
                ev = Some((1.0, Sig::In));
                pos1 += 1;
            } else if green_up(i, xp) && mt[i] == -1 && !lc(i) {
                ev = Some((1.0, Sig::Out));
                pos1 -= 1;
            } else if mt[i] == 1 && !lc(i) {
                ev = Some((1.0, Sig::Close("ST turned up")));
                pos1 = 0;
            }
        } else if pos1 == 0 && pos == 0 && mt[i] == -1 && green_up(i, ep) && sf(i) {
            ev = Some((-1.0, Sig::Open));
            pos1 = -1;
        } else if pos1 == -1 {
            if green_up(i, xp) && mt[i] == -1 && sf(i) {
                ev = Some((-1.0, Sig::In));
                pos1 -= 1;
            } else if (red_drop(i) || mt[i] == 1) && !uc(i) {
                ev = Some((-1.0, Sig::Close(if mt[i] == 1 { "ST turned up" } else { "Red candle down ENTRY %" })));
                pos1 = 0;
            }
        } else if pos1 < -1 {
            if green_up(i, xp) && mt[i] == -1 && sf(i) {
                ev = Some((-1.0, Sig::In));
                pos1 -= 1;
            } else if red_drop(i) && mt[i] == -1 && !uc(i) {
                ev = Some((-1.0, Sig::Out));
                pos1 += 1;
            } else if mt[i] == 1 && !uc(i) {
                ev = Some((-1.0, Sig::Close("ST turned up")));
                pos1 = 0;
            }
        }

        // the backtester
        let Some((dir, sig)) = ev else { continue };
        match sig {
            Sig::Open => {
                if i >= from {
                    cur = Some(Build { dir, legs: vec![Leg { i, px: c[i], w }], outs: Vec::new(), shares: w / c[i] });
                }
            }
            Sig::In => {
                if let Some(t) = cur.as_mut() {
                    if t.legs.len() < max_legs {
                        t.legs.push(Leg { i, px: c[i], w });
                        t.shares += w / c[i];
                    }
                }
            }
            Sig::Out => {
                if let Some(mut t) = cur.take() {
                    let q = w / c[i];
                    if q >= t.shares * (1.0 - 1e-12) {
                        // the rest is sold: later adds and exits of this loop position find no open trade
                        out.push(t.finish(i, c[i], "Scale-out sold the rest", false));
                    } else {
                        t.outs.push((i, c[i], q));
                        t.shares -= q;
                        cur = Some(t);
                    }
                }
            }
            Sig::Close(why) => {
                if let Some(t) = cur.take() {
                    out.push(t.finish(i, c[i], why, false));
                }
            }
        }
    }
    if let Some(t) = cur {
        out.push(t.finish(to, c[to], "open", true));
    }
    Some(out)
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let d = catalogue().into_iter().find(|d| d.id == id)?;
    Some((d.id, d.name, "2% Scalein - Nifty RedGreen Candle"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::books::ytwh::testbars::*;

    fn trades(r: &[crate::Row], kv: &[(&str, f64)]) -> Vec<Trade> {
        let x = Ctx::new(r, r, &[]);
        run(REGIM, &x, 0, r.len() - 1, &ps(kv)).unwrap()
    }

    /// Daily ST (st_tf 3, period 1) so the trend follows the bars: a rise, then red-drop and green-rise candles.
    fn series() -> Vec<(f64, f64, f64, f64)> {
        let mut v: Vec<(f64, f64, f64, f64)> = (0..10).map(|k| { let c = 100.0 + k as f64; (c - 0.5, c + 0.5, c - 1.0, c) }).collect();
        v.push((109.5, 110.0, 107.0, 107.5)); // bar 10: red, -1.4% -> buy at 107.5
        v.push((107.5, 108.0, 105.5, 106.0)); // bar 11: red, -1.4% -> add at 106
        v.push((106.0, 108.5, 105.8, 108.0)); // bar 12: green, +1.9% -> scale out one piece at 108
        v.push((108.0, 110.0, 107.8, 109.5)); // bar 13: green, +1.4% -> one piece left: sell the rest at 109.5
        v
    }

    #[test]
    fn scale_in_scale_out_and_sell() {
        let r = bars(&series());
        let kv = [("st_tf", 3.0), ("st_period", 1.0), ("st_factor", 10.0)];
        let t = trades(&r, &kv);
        assert_eq!(t.len(), 1);
        let t = &t[0];
        assert_eq!(t.legs.iter().map(|l| (l.i, l.px)).collect::<Vec<_>>(), vec![(10, 107.5), (11, 106.0)]);
        assert_eq!(t.exit_i, 13);
        // shares: 0.02/107.5 + 0.02/106; scale-out sells 0.02/108; the rest at 109.5
        let s = 0.02 / 107.5 + 0.02 / 106.0;
        let q = 0.02 / 108.0;
        assert_eq!(t.exits.len(), 2);
        assert!((t.exits[0].2 - q / s).abs() < 1e-12 && (t.exits[0].0, t.exits[0].1) == (12, 108.0));
        let pnl = q * 108.0 + (s - q) * 109.5 - 0.04;
        assert!((t.ret(0.0) - pnl).abs() < 1e-12);
    }

    #[test]
    fn st_down_sells_all_and_bear_branch_trades() {
        let mut v = series();
        v.truncate(12);
        // bar 12: a drop below the ratcheted daily ST line (93.75): the ST turns down -> the BULL position (two
        // pieces) is sold at the close
        v.push((105.0, 105.2, 60.0, 60.5));
        let r = bars(&v);
        let kv = [("st_tf", 3.0), ("st_period", 1.0), ("st_factor", 10.0)];
        let t = trades(&r, &kv);
        assert!(!t.is_empty());
        assert_eq!(t.len(), 1);
        assert_eq!((t[0].legs.len(), t[0].exit_i, t[0].exit_px), (2, 12, 60.5));
        assert!(t[0].reason.contains("down") && t[0].exits.is_empty());
        // BEAR: nothing is bought while the ST is up
        let b = trades(&r, &[kv[0], kv[1], kv[2], ("trade_ver", 2.0)]);
        assert!(b.iter().all(|t| mt_down_at(&r, t.legs[0].i, &kv)));
    }

    fn mt_down_at(r: &[crate::Row], i: usize, kv: &[(&str, f64)]) -> bool {
        let x = Ctx::new(r, r, &[]);
        let g = |k: &str| kv.iter().find(|(a, _)| *a == k).map_or(1.0, |v| v.1);
        let b = compress(&x, tf_of(g("st_tf") as i64, true));
        let (tr, _) = st_trend_up(&calc_point(&b, 1), &b.h, &b.l, &b.c, g("st_period") as usize, g("st_factor"));
        expand_last(&tr, &b)[i] == -1
    }

    #[test]
    fn every_strategy_runs() {
        let r = wave(600);
        let x = Ctx::new(&r, &r, &[]);
        for (from, to) in [(0, r.len() - 1), (50, 300)] {
            for t in run(REGIM, &x, from, to, &Params::new()).unwrap() {
                assert!(t.legs[0].i >= from && t.exit_i <= to && t.exit_i >= t.legs[0].i);
                assert!(t.legs.iter().all(|l| l.i <= to && l.px == x.c[l.i]));
                assert!(t.exits.iter().all(|e| e.0 <= t.exit_i && e.1 == x.c[e.0]));
            }
        }
        for kv in [vec![], vec![("trade_ver", 2.0)], vec![("st_tf", 3.0), ("trade_ver", 2.0)], vec![("st_tf", 2.0), ("entry_pct", 0.5), ("exit_pct", 0.5)]] {
            for t in trades(&r, &kv) {
                assert!(t.legs.windows(2).all(|w| w[0].i < w[1].i) && t.exit_i >= t.legs.last().unwrap().i);
                if !t.exits.is_empty() {
                    assert!((t.exits.iter().map(|e| e.2).sum::<f64>() - 1.0).abs() < 1e-9);
                    assert_eq!(t.exits.last().unwrap().0, t.exit_i);
                }
                assert!(t.legs.iter().map(|l| l.w).sum::<f64>() <= 1.0 + 1e-9);
            }
        }
    }
}
