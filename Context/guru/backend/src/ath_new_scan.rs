//! ATH sub-variant from "AFL ATH checking/": `ATHC 5pct to 40 pct scanner.afl` (id `ytath_chk_5pct_to_40pct_scanner`).
//!
//! The AFL is an Explore scanner (`Filter = C>ATHC5 AND C<ATHC30`, stocks 5 % to `% RANGE` above the first close of a running all-time-high
//! state) but it also carries a complete Buy / Sell block, which is what "Run backtest" uses:
//!   Buy  = (ATHCbuysig OR C > Ref(Hcl,-1)) AND NOT UPcircuit AND DeadStockBuySig AND C > pennyflt AND turnover
//!   Sell = ((trend1 == -1 OR ATHCbuysig == 0) AND NOT LOCircuit) OR DeadStockSellSig
//!   Buy = ExRem(Buy, Sell); Sell = ExRem(Sell, Buy); BuyPrice = ValueWhen(Buy, C); SellPrice = ValueWhen(Sell, C)
//! The %swing line is built on `STCAL` (the SWING CALCULATION POINT) with `exit = STCAL * EXITPCT / 100` (default 18).
//! `% RANGE` (`athrange`, default 40) only feeds the Explore filter (ATHC30); it has no effect on the backtest.
//!
//! No team trade sheet exists for this file: see `ATH/checked/ytath_chk_5pct_to_40pct_scanner/RESULT.md` for what was checked.
#![allow(dead_code)]
use crate::ath_core::*;
use crate::ath_ind::ma;

/// The AFL's parameters (all defaults are the AFL's own).
#[derive(Clone, Copy, Debug)]
pub struct Params {
    pub exit_pct: f32,
    /// 1 CLOSE, 2 MIDD POINT, 3 HIGH, 4 PIVOT, 5 MP CL PIVOT AVG, 6 3DAYS MA, 7 5DAYS MA (ParamList index + 1)
    pub swingcal: u8,
    pub penny: f32,
    pub turnover_filter: bool,
    /// "DEAD STOCK BACKTEST" = YES: no buy on the last bar, sell on the last bar of every symbol.
    pub dead_stock: bool,
    /// `% RANGE`: only the Explore filter (ATHC30).
    pub range_pct: f32,
}
impl Default for Params {
    fn default() -> Self {
        Params { exit_pct: 18.0, swingcal: 1, penny: 0.0, turnover_filter: false, dead_stock: false, range_pct: 40.0 }
    }
}

/// Yearly turnover threshold of the AFL (`volsig`), compared with `V * C`.
fn volsig(year: i32) -> f32 {
    match year {
        y if y < 2000 => 30_000.0,
        2000..=2004 => 50_000.0,
        2005..=2009 => 1_800_000.0,
        2010..=2014 => 2_600_000.0,
        2015..=2019 => 6_000_000.0,
        _ => 13_000_000.0,
    }
}

/// `STCAL`: starts as `MSTSCAL = MP = (H+L)/2`; the AFL's if/else chain compares the list text with "MID POINT(H+L/2)" while the list item is
/// "MIDD POINT(H+L/2)", so the second item matches no branch and keeps the initial mid point (same value as the intended one).
pub fn stcal(s: &Series, mode: u8) -> Vec<f32> {
    let n = s.bars.len();
    let (c, h, l) = (s.close(), s.high(), s.low());
    let mp: Vec<f32> = (0..n).map(|i| (h[i] + l[i]) / 2.0).collect();
    let pivot: Vec<f32> = (0..n).map(|i| (h[i] + l[i] + c[i]) / 3.0).collect();
    match mode {
        1 => c,
        3 => h,
        4 => pivot,
        5 => (0..n).map(|i| (mp[i] + pivot[i] + c[i]) / 3.0).collect(),
        6 => ma(&c, 3),
        7 => ma(&c, 5),
        _ => mp, // 2 = MIDD POINT (no branch matches) and anything else
    }
}

/// `trend1` of the AFL's loop (float32 arrays; Null = NaN compares false).
pub fn swing_trend(c: &[f32], st: &[f32], exit_pct: f32) -> Vec<i8> {
    let n = c.len();
    let exit: Vec<f32> = st.iter().map(|&x| (x * exit_pct) / 100.0).collect();
    let mut up: Vec<f32> = (0..n).map(|i| st[i] + exit[i]).collect();
    let mut dn: Vec<f32> = (0..n).map(|i| st[i] - exit[i]).collect();
    let mut trend = vec![0i8; n];
    if n > 0 {
        trend[0] = 1;
    }
    for i in 1..n {
        trend[i] = 1;
        if c[i] > up[i - 1] {
            trend[i] = 1;
        } else if c[i] < dn[i - 1] {
            trend[i] = -1;
        } else if trend[i - 1] == 1 {
            trend[i] = 1;
        } else if trend[i - 1] == -1 {
            trend[i] = -1;
        }
        let flag1 = trend[i] < 0 && trend[i - 1] > 0;
        let flagh1 = trend[i] > 0 && trend[i - 1] < 0;
        if trend[i] > 0 && dn[i] < dn[i - 1] {
            dn[i] = dn[i - 1];
        }
        if trend[i] < 0 && up[i] > up[i - 1] {
            up[i] = up[i - 1];
        }
        if flag1 {
            up[i] = st[i] + exit[i];
        }
        if flagh1 {
            dn[i] = st[i] - exit[i];
        }
    }
    trend
}

/// The ATHC latch loop: (ATHCbuysig, ATHC5, ATHC30).
pub fn athc_latch(c: &[f32], hcl: &[f32], trend: &[i8], range_pct: f32) -> (Vec<bool>, Vec<f32>, Vec<f32>) {
    let n = c.len();
    let mut sig = vec![false; n];
    let (mut a5, mut a30) = (vec![f32::NAN; n], vec![f32::NAN; n]);
    for i in 1..n {
        if c[i] > hcl[i - 1] && !sig[i - 1] {
            sig[i] = true;
            a5[i] = c[i] + (c[i] * 5.0) / 100.0;
            a30[i] = c[i] + (c[i] * range_pct) / 100.0;
        } else if trend[i] == -1 && sig[i - 1] {
            sig[i] = false;
            a5[i] = f32::NAN;
            a30[i] = f32::NAN;
        } else {
            sig[i] = sig[i - 1];
            a5[i] = a5[i - 1];
            a30[i] = a30[i - 1];
        }
    }
    (sig, a5, a30)
}

/// The Explore filter `C > ATHC5 AND C < ATHC30` (not used by the backtest; Null compares false).
pub fn scan_hits(s: &Series, p: &Params) -> Vec<bool> {
    let c = s.close();
    let hcl = running_high(&c);
    let st = stcal(s, p.swingcal);
    let trend = swing_trend(&c, &st, p.exit_pct);
    let (_, a5, a30) = athc_latch(&c, &hcl, &trend, p.range_pct);
    (0..c.len()).map(|i| c[i] > a5[i] && c[i] < a30[i]).collect()
}

fn running_high(c: &[f32]) -> Vec<f32> {
    let mut h = vec![0f32; c.len()];
    for i in 0..c.len() {
        h[i] = if i == 0 || c[i] > h[i - 1] { c[i] } else { h[i - 1] };
    }
    h
}

pub fn signals(s: &Series, p: &Params) -> Signals {
    let n = s.bars.len();
    let (c, o, h, l, v) = (s.close(), s.open(), s.high(), s.low(), s.volume());
    let hcl = running_high(&c);
    let st = stcal(s, p.swingcal);
    let trend = swing_trend(&c, &st, p.exit_pct);
    let (athc, _, _) = athc_latch(&c, &hcl, &trend, p.range_pct);
    let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
    for i in 0..n {
        let upc = prec2(o[i]) == prec2(c[i]) && prec2(o[i]) == prec2(h[i]);
        let loc = prec2(o[i]) == prec2(c[i]) && prec2(o[i]) == prec2(l[i]);
        let turnover = if p.turnover_filter { v[i] * c[i] > volsig(s.bars[i].date / 10000) } else { true };
        let dead_buy = if p.dead_stock { i != n - 1 } else { i != 0 };
        let dead_sell = p.dead_stock && i == n - 1;
        // C > Ref(Hcl,-1): on bar 0 there is no previous bar and a comparison with Null is false (verified on the team sheets of ytath_plus_4pct_buy_*;
        // reading it as true gave a bogus buy on the first bar of every new listing with dead stock YES: 1,264 -> 1,354 exact trades on the 18 % sheet)
        let fresh = i > 0 && c[i] > hcl[i - 1];
        buy[i] = (athc[i] || fresh) && !upc && dead_buy && c[i] > p.penny && turnover;
        sell[i] = ((trend[i] == -1 || !athc[i]) && !loc) || dead_sell;
    }
    let buy = exrem(&buy, &sell);
    let sell = exrem(&sell, &buy);
    // BuyPrice / SellPrice = ValueWhen(signal, C): on the signal bar that is the bar's own Close (inside [Low, High])
    let bp: Vec<f32> = (0..n).map(|i| c[i].min(h[i]).max(l[i])).collect();
    Signals::new(buy, sell, bp.clone(), bp)
}

// ---------------------------------------------------------------- the variant this module exposes

/// The AFL's own parameters. Toggles use the AFL's raw value (0/1) like the other ATH engines; `swingcal` is the ParamList item number 1..7.
fn schema() -> Vec<P> {
    vec![
        p("deadstock", "DEAD STOCK BACKTEST (0 YES, 1 NO)", 1.0, 0.0, 1.0, 1.0),
        p("hardstop", "Hard Stoploss % (0 = off; ApplyStop percent loss, not checked against a team sheet)", 0.0, 0.0, 50.0, 1.0),
        p("pennyflt", "Penny Stock Price", 0.0, 0.0, 100.0, 1.0),
        p("turnflt", "TURNOVER Filter (0 NO, 1 YES; V*C > yearly table)", 0.0, 0.0, 1.0, 1.0),
        p("exitpct", "EXIT %", 18.0, 5.0, 35.0, 1.0),
        p("swingcal", "SWING CALCULATION POINT (1 CLOSE; 2 MIDD POINT H+L/2; 3 HIGH; 4 PIVOT H+L+C/3; 5 MP CL PIVOT AVG; 6 3DAYS MA; 7 5DAYS MA)", 1.0, 1.0, 7.0, 1.0),
        p("athrange", "% RANGE (Explore filter ATHC+x% only; no effect on the backtest)", 40.0, 1.0, 50.0, 1.0),
    ]
}

fn params_of(g: G) -> Params {
    Params {
        exit_pct: g("exitpct") as f32,
        swingcal: (g("swingcal").round() as i64).clamp(1, 7) as u8,
        penny: g("pennyflt") as f32,
        turnover_filter: g("turnflt").round() == 1.0,
        dead_stock: g("deadstock").round() == 0.0,
        range_pct: g("athrange") as f32,
    }
}

fn all_signals(series: &[Series], _env: &Env, g: G) -> Vec<Signals> {
    let ep = params_of(g);
    let hard = g("hardstop") as f32;
    series
        .iter()
        .map(|s| {
            let mut sg = signals(s, &ep);
            // ApplyStop(stopTypeLoss, stopModePercent, hardstop): not checked against a team sheet
            if hard > 0.0 {
                sg.stop = Some(StopSpec { loss_pct: hard, ..StopSpec::default() });
            }
            sg
        })
        .collect()
}

pub fn variants() -> Vec<VariantDef> {
    vec![VariantDef { id: "ytath_chk_5pct_to_40pct_scanner", data: Data::Cash22 { watchlist: "NSE 500 C&P.tls" }, schema, signals: all_signals }]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bar(date: i32, o: f32, h: f32, l: f32, c: f32) -> Bar {
        Bar { date, c, o, h, l, v: 1000.0, oi: 1.0e7, aux1: 1.0, aux2: 0.0 }
    }
    fn series(closes: &[f32]) -> Series {
        let bars = closes.iter().enumerate().map(|(i, &c)| bar(20200101 + i as i32, c, c + 1.0, c - 1.0, c)).collect();
        Series { name: "T".into(), bars }
    }

    #[test]
    fn defaults_are_the_afls() {
        let d = Params::default();
        assert_eq!((d.exit_pct, d.swingcal, d.range_pct, d.penny), (18.0, 1, 40.0, 0.0));
        let sc = schema();
        let get = |k: &str| sc.iter().find(|q| q.key == k).unwrap().def;
        assert_eq!((get("exitpct"), get("swingcal"), get("athrange"), get("deadstock"), get("turnflt")), (18.0, 1.0, 40.0, 1.0, 0.0));
        assert_eq!(variants()[0].id, "ytath_chk_5pct_to_40pct_scanner");
    }

    #[test]
    fn midd_point_item_keeps_the_initial_mid_point() {
        let s = series(&[100.0, 110.0, 120.0]);
        let mp = stcal(&s, 2);
        assert_eq!(mp[1], (111.0f32 + 109.0) / 2.0);
        assert_eq!(stcal(&s, 1), s.close());
        assert_eq!(stcal(&s, 3), s.high());
        assert_eq!(stcal(&s, 4)[1], (111.0f32 + 109.0 + 110.0) / 3.0);
        assert!(stcal(&s, 6)[1].is_nan() && stcal(&s, 6)[2] == 110.0);
    }

    #[test]
    fn latch_levels_use_first_ath_close_and_the_range_param() {
        // ATH closes 101, 102, 103 then a 25 % fall (18 % swing broken)
        let s = series(&[100.0, 101.0, 102.0, 103.0, 77.0, 76.0]);
        let c = s.close();
        let hcl = running_high(&c);
        let tr = swing_trend(&c, &c, 18.0);
        let (sig, a5, a30) = athc_latch(&c, &hcl, &tr, 40.0);
        assert!(sig[1] && sig[2] && sig[3] && !sig[4]);
        assert_eq!(a5[3], 101.0 + (101.0f32 * 5.0) / 100.0, "levels stay at the first ATH close");
        assert_eq!(a30[2], 101.0 + (101.0f32 * 40.0) / 100.0);
        assert!(a5[4].is_nan());
        let (_, _, a30b) = athc_latch(&c, &hcl, &tr, 25.0);
        assert_eq!(a30b[1], 101.0 + (101.0f32 * 25.0) / 100.0);
    }

    #[test]
    fn range_param_changes_only_the_scan_not_the_signals() {
        let s = series(&[100.0, 101.0, 102.0, 103.0, 104.0, 105.0, 106.5, 77.0, 76.0]);
        let a = signals(&s, &Params { range_pct: 40.0, ..Params::default() });
        let b = signals(&s, &Params { range_pct: 5.0, ..Params::default() });
        assert_eq!((&a.buy, &a.sell), (&b.buy, &b.sell));
        let h40 = scan_hits(&s, &Params::default());
        let h10 = scan_hits(&s, &Params { range_pct: 10.0, ..Params::default() });
        assert!(h40[6] && !h40[1]); // 106.5 > 101 * 1.05 = 106.05
        assert!(h10[6]);
        let h5 = scan_hits(&s, &Params { range_pct: 5.0, ..Params::default() });
        assert!(!h5[6], "with a 5 % cap the band 5 %..5 % is empty");
    }

    #[test]
    fn buy_on_ath_close_and_sell_when_swing_breaks() {
        let s = series(&[100.0, 101.0, 102.0, 103.0, 104.0, 80.0, 79.0]);
        let sg = signals(&s, &Params::default());
        assert!(sg.buy[1] && !sg.buy[2]);
        assert!(sg.sell[5], "80 < 104 * 0.82 = 85.3 breaks the 18 % swing line");
        assert_eq!(sg.buy_price[1], 101.0);
        assert_eq!(sg.sell_price[5], 80.0);
    }

    #[test]
    fn swing_line_depends_on_the_exit_percent() {
        // 10 % fall: broken by a 10 % swing, not by the default 18 %
        let s = series(&[100.0, 101.0, 102.0, 103.0, 104.0, 92.0, 91.0, 90.0]);
        let d = signals(&s, &Params::default());
        let t = signals(&s, &Params { exit_pct: 10.0, ..Params::default() });
        assert!(!d.sell[5] && t.sell[5]);
    }

    #[test]
    fn dead_stock_yes_sells_last_bar_never_buys_there_and_penny_blocks() {
        let s = series(&[100.0, 101.0, 102.0, 103.0]);
        let sg = signals(&s, &Params { dead_stock: true, ..Params::default() });
        assert!(sg.sell[3] && !sg.buy[3]);
        let cheap = series(&[5.0, 6.0, 7.0]);
        assert!(signals(&cheap, &Params { penny: 10.0, ..Params::default() }).buy.iter().all(|&b| !b));
    }

    #[test]
    fn turnover_filter_uses_volume_times_close() {
        let mut s = series(&[100.0, 101.0, 102.0, 103.0]); // V*C about 100,000 < 13,000,000 (2020)
        let off = signals(&s, &Params::default());
        let on = signals(&s, &Params { turnover_filter: true, ..Params::default() });
        assert!(off.buy.iter().any(|&b| b) && on.buy.iter().all(|&b| !b));
        for b in s.bars.iter_mut() {
            b.v = 1.0e6;
        }
        assert!(signals(&s, &Params { turnover_filter: true, ..Params::default() }).buy[1]);
    }

    #[test]
    fn upper_circuit_bar_is_not_bought() {
        let mut s = series(&[100.0, 101.0, 102.0]);
        s.bars[1].o = 101.0;
        s.bars[1].h = 101.0;
        s.bars[1].l = 101.0;
        let sg = signals(&s, &Params::default());
        assert!(!sg.buy[1] && sg.buy[2]);
    }

    #[test]
    fn no_buy_on_bar_zero_with_dead_stock_yes() {
        // Ref(Hcl,-1) is Null on bar 0 and a comparison with Null is false: a first bar must not trigger the running ATH buy
        let s = series(&[100.0, 99.0, 98.0]);
        let sg = signals(&s, &Params { dead_stock: true, ..Params::default() });
        assert!(!sg.buy[0] && !sg.buy[1]);
    }
}
