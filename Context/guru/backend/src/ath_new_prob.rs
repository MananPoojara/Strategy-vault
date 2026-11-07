//! ATH "ATHC checking" variant `ytath_chk_19pct_swing_sl_probability`: AFL `AFL ATH checking/ATHC with 19%swing sl with probability.afl`.
//!
//! Closest AFL: `ATHC with %swing sl with probability.afl` (Running ATH Buy copy, engine `ath_loss2::signals_v4`, id ytath_running_pctswing_exit_after_2_5).
//! Differences coded here (all of them, from a line diff of the two files):
//!  * defaults: EXIT % 19 (Param, was Optimize 10), MIN PROFIT % 2 range 1..3 step .05 (was 3, 1..5), CONSICUTIVE LOSS 1 range 0..2 (was 2, 1..2),
//!    TURNOVER Filter default YES (was NO); Initial Equity / MAXIMUM POSITION defaults and ranges are runner (common) parameters;
//!  * the virtual %swing trades (the loss streak) buy on `C > Ref(Hcl,-1)` (a fresh ATH close) instead of the ATHCbuysig state;
//!  * an unused `EMA` section at the end (plot only).
//! The loss-streak machinery (swing line, ATHC state, concount) is a copy of the verified `ath_loss2` code (private there; that file is not edited).
//! Standard library only. Verification notes: `ATH/checked/ytath_chk_19pct_swing_sl_probability/RESULT.md`.
#![allow(dead_code)]
use crate::ath_core::*;

/// AmiBroker's Null (EMPTY) value inside arrays.
const NULLV: f32 = -1.0e10;

// ---------------------------------------------------------------- copied from ath_loss2.rs (verified)

/// The %swing line of the AFL (identical in the three files): trend1 and TrendUp1 (Null where not set).
struct Swing {
    trend: Vec<i8>,
    trendup: Vec<f32>,
}

fn swing(c: &[f32], exit_pct: f32) -> Swing {
    let n = c.len();
    let exit: Vec<f32> = c.iter().map(|&x| (x * exit_pct) / 100.0).collect();
    let mut up: Vec<f32> = (0..n).map(|i| c[i] + exit[i]).collect();
    let mut dn: Vec<f32> = (0..n).map(|i| c[i] - exit[i]).collect();
    let mut trend = vec![0i8; n];
    let mut trendup = vec![NULLV; n];
    let mut trenddown = vec![NULLV; n];
    if n > 0 {
        trend[0] = 1;
    }
    let mut change = false;
    for i in 1..n {
        trend[i] = 1;
        if c[i] > up[i - 1] {
            trend[i] = 1;
            if trend[i - 1] == -1 {
                change = true;
            }
        } else if c[i] < dn[i - 1] {
            trend[i] = -1;
            if trend[i - 1] == 1 {
                change = true;
            }
        } else if trend[i - 1] == 1 {
            trend[i] = 1;
            change = false;
        } else if trend[i - 1] == -1 {
            trend[i] = -1;
            change = false;
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
            up[i] = c[i] + exit[i];
        }
        if flagh1 {
            dn[i] = c[i] - exit[i];
        }
        if trend[i] == 1 {
            trendup[i] = dn[i];
            if change {
                trendup[i - 1] = trenddown[i - 1];
                change = false;
            }
        } else if trend[i] == -1 {
            trenddown[i] = up[i];
            if change {
                trenddown[i - 1] = trendup[i - 1];
                change = false;
            }
        }
    }
    Swing { trend, trendup }
}

/// `Hcl = HHV(C, BarCount)`: highest close up to the bar.
fn running_high(c: &[f32]) -> Vec<f32> {
    let mut h = vec![0f32; c.len()];
    for i in 0..c.len() {
        h[i] = if i == 0 || c[i] > h[i - 1] { c[i] } else { h[i - 1] };
    }
    h
}

/// ATHCbuysig: 1 on a close above all earlier closes, 0 when the swing trend is -1 while it was 1, else carried.
fn athc_state(c: &[f32], hcl: &[f32], trend: &[i8]) -> Vec<bool> {
    let n = c.len();
    let mut a = vec![false; n];
    for i in 1..n {
        a[i] = if c[i] > hcl[i - 1] {
            true
        } else if trend[i] == -1 && a[i - 1] {
            false
        } else {
            a[i - 1]
        };
    }
    a
}

/// Loss streak of the virtual %swing trades (in on ATHC buysig, out on trend -1): `concount`.
fn concount(c: &[f32], buysig: &[bool], sw: &Swing, losspct: f32, comm: f32) -> Vec<f32> {
    let n = c.len();
    let (mut bcount, mut bprice, mut sprice) = (vec![0i32; n], vec![NULLV; n], vec![NULLV; n]);
    for i in 1..n {
        if buysig[i] && bcount[i - 1] < 1 {
            bprice[i] = c[i];
            sprice[i] = sprice[i - 1];
            bcount[i] = bcount[i - 1] + 1;
        } else if sw.trend[i] == -1 && bcount[i - 1] == 1 {
            sprice[i] = c[i];
            bprice[i] = bprice[i - 1];
            bcount[i] = 0;
        } else {
            bprice[i] = bprice[i - 1];
            sprice[i] = sprice[i - 1];
            bcount[i] = bcount[i - 1];
        }
    }
    // Cross(Ref(TrendUp1,-1), C): TrendUp1 of the previous bar rises above C now, while it was not above C one bar earlier
    let a = |i: usize| -> f32 { if i >= 1 { sw.trendup[i - 1] } else { NULLV } };
    let mut cc = vec![0f32; n];
    for i in 1..n {
        let cross = i >= 2 && a(i) > c[i] && a(i - 1) <= c[i - 1];
        let ev = cross && bcount[i - 1] == 1;
        let profit = if ev { (sprice[i] - (sprice[i] * comm) / 100.0) - (bprice[i] + (bprice[i] * comm) / 100.0) } else { 0.0 };
        let pct = if ev { (((sprice[i] - (sprice[i] * comm) / 100.0) - (bprice[i] + (bprice[i] * comm) / 100.0)) / bprice[i]) * 100.0 } else { 0.0 };
        cc[i] = if profit != 0.0 {
            if pct <= losspct { cc[i - 1] + 1.0 } else { 0.0 }
        } else {
            cc[i - 1]
        };
    }
    cc
}

fn volsig4(year: i32) -> f32 {
    match year {
        y if y < 2000 => 30_000.0,
        2000..=2004 => 50_000.0,
        2005..=2009 => 1_800_000.0,
        2010..=2014 => 2_600_000.0,
        2015..=2019 => 6_000_000.0,
        _ => 13_000_000.0,
    }
}

// ---------------------------------------------------------------- this AFL

/// `vstate`: virtual trades buy on the ATHC state (the older Running ATH Buy AFL) instead of on a fresh ATH close (this AFL). Only for the
/// differential check; `ucblock` is not part of any AFL and stays off here.
pub fn run_signals(s: &Series, g: G, vstate: bool, ucblock: bool) -> Signals {
    let n = s.bars.len();
    let (c, o, h, l, v) = (s.close(), s.open(), s.high(), s.low(), s.volume());
    let exit_pct = g("exitpct") as f32;
    let (losspct, noofloss, comm, penny) = (g("losspct") as f32, g("noofloss") as f32, g("comm") as f32, g("pennyflt") as f32);
    let turnflt = g("turnflt").round() == 1.0;
    let dead_yes = g("deadstock").round() == 0.0;
    // ParamToggle("Aux1"/"Aux2", "ON|OFF", 1): value 0 = ON = `IIf(AuxToggle==0, Aux1 == 1, 1)` in the real Buy only (the virtual trades stay unfiltered, as in the AFL)
    let (aux1_on, aux2_on) = (g("auxtoggle").round() == 0.0, g("auxtoggle1").round() == 0.0);
    let aux_ok = |i: usize| (!aux1_on || s.bars[i].aux1 == 1.0) && (!aux2_on || s.bars[i].aux2 == 1.0);
    let hcl = running_high(&c);
    let sw = swing(&c, exit_pct);
    let athc = athc_state(&c, &hcl, &sw.trend);
    let upc: Vec<bool> = (0..n).map(|i| prec2(o[i]) == prec2(c[i]) && prec2(o[i]) == prec2(h[i])).collect();
    let loc: Vec<bool> = (0..n).map(|i| prec2(o[i]) == prec2(c[i]) && prec2(o[i]) == prec2(l[i])).collect();
    let turnover: Vec<bool> = (0..n).map(|i| if !turnflt { true } else { v[i] * c[i] > volsig4(s.bars[i].date / 10000) }).collect();
    let dead_buy = |i: usize| if dead_yes { i != n - 1 } else { i != 0 };
    let fresh = |i: usize| i >= 1 && c[i] > hcl[i - 1];
    // buysig = C > Ref(Hcl,-1) AND NOT UPCIRCUIT AND DEADSTOCKBUYSIG AND C > pennyflt AND turnover   (loop runs from bar 1)
    let buysig_v: Vec<bool> = (0..n).map(|i| (if vstate { athc[i] } else { fresh(i) }) && !upc[i] && dead_buy(i) && c[i] > penny && turnover[i]).collect();
    let cc = concount(&c, &buysig_v, &sw, losspct, comm);
    let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
    for i in 0..n {
        buy[i] = (athc[i] || fresh(i)) && cc[i] >= noofloss && !upc[i] && !(ucblock && h[i] == c[i]) && dead_buy(i) && c[i] > penny && turnover[i] && aux_ok(i);
        sell[i] = ((sw.trend[i] == -1 || !athc[i]) && !loc[i]) || (dead_yes && i == n - 1);
    }
    // bar 0: Ref(Hcl,-1) is Null there and AmiBroker's C > Null is true; concount/athc are 0 on bar 0
    if n > 0 {
        buy[0] = cc[0] >= noofloss && !upc[0] && dead_buy(0) && c[0] > penny && turnover[0] && aux_ok(0);
    }
    let buy = exrem(&buy, &sell);
    let sell = exrem(&sell, &buy);
    let (mut bp, mut sp) = (c.clone(), c.clone());
    for i in 0..n {
        bp[i] = bp[i].min(h[i]).max(l[i]);
        sp[i] = sp[i].min(h[i]).max(l[i]);
    }
    let mut sg = Signals::new(buy, sell, bp, sp);
    let hard = g("hardstop") as f32;
    if hard > 0.0 {
        sg.stop = Some(StopSpec { loss_pct: hard, ..StopSpec::default() });
    }
    sg
}

/// Keys as in the catalog entry of this id; values are the raw AFL values (toggles 0/1), defaults are the AFL's own.
pub fn schema() -> Vec<P> {
    vec![
        p("deadstock", "DEAD STOCK BACKTEST (0 YES, 1 NO)", 1.0, 0.0, 1.0, 1.0),
        p("hardstop", "Hard Stoploss % (0 = off; ApplyStop, not checked against a sheet)", 0.0, 0.0, 50.0, 1.0),
        p("pennyflt", "Penny Stock Price", 0.0, 0.0, 100.0, 1.0),
        p("turnflt", "TURNOVER Filter (0 NO, 1 YES; V*C > yearly table)", 1.0, 0.0, 1.0, 1.0),
        p("auxtoggle", "Aux1 (0 ON = only Aux1 == 1; 1 OFF)", 1.0, 0.0, 1.0, 1.0),
        p("auxtoggle1", "Aux2 (0 ON = only Aux2 == 1; 1 OFF)", 1.0, 0.0, 1.0, 1.0),
        p("exitpct", "EXIT %", 19.0, 5.0, 25.0, 1.0),
        p("losspct", "MIN PROFIT %", 2.0, 1.0, 3.0, 0.05),
        p("noofloss", "CONSICUTIVE LOSS", 1.0, 0.0, 2.0, 1.0),
        p("rankmode", "RANKING MODE (0 NONE; other modes not coded for this variant)", 0.0, 0.0, 10.0, 1.0),
        p("rocpd", "ROC PERIOD for RANKING/NO OF POSITIVE CLOSE", 10.0, 30.0, 400.0, 30.0),
        p("rsipd", "RSI PERIOD for RANKING", 14.0, 10.0, 1000.0, 10.0),
        p("stdpd", "STD PERIOD for RANKING", 14.0, 10.0, 1000.0, 10.0),
        p("swingpd", "SWING PERIOD", 180.0, 10.0, 1000.0, 1.0),
        p("comm", "BROKERAGE % used by the AFL's virtual-trade profit test (the Vault's brokerage)", 0.65, 0.0, 1.0, 0.01),
    ]
}

pub fn signals(series: &[Series], _env: &Env, g: G) -> Vec<Signals> {
    series.iter().map(|s| run_signals(s, g, false, false)).collect()
}

pub fn variants() -> Vec<VariantDef> {
    vec![VariantDef { id: "ytath_chk_19pct_swing_sl_probability", data: Data::Cash22 { watchlist: "NSE 500 C&P.tls" }, schema, signals }]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bar(d: i32, o: f32, h: f32, l: f32, c: f32) -> Bar {
        Bar { date: d, c, o, h, l, v: 1.0e9, oi: 0.0, aux1: 0.0, aux2: 0.0 }
    }
    fn series(closes: &[f32]) -> Series {
        let bars = closes.iter().enumerate().map(|(i, &c)| bar(20200101 + i as i32, c - 0.5, c + 1.0, c - 1.0, c)).collect();
        Series { name: "T".into(), bars }
    }
    fn dflt(k: &str) -> f64 {
        let d = schema().into_iter().find(|x| x.key == k).unwrap();
        d.def.clamp(d.min, d.max)
    }

    #[test]
    fn defaults_are_the_afls() {
        assert_eq!((dflt("exitpct"), dflt("losspct"), dflt("noofloss"), dflt("turnflt"), dflt("deadstock")), (19.0, 2.0, 1.0, 1.0, 1.0));
        // Aux1 / Aux2 default OFF (ParamToggle default 1)
        assert_eq!((dflt("auxtoggle"), dflt("auxtoggle1")), (1.0, 1.0));
        assert_eq!(volsig4(2019), 6_000_000.0);
        assert_eq!(volsig4(2021), 13_000_000.0);
    }

    #[test]
    fn fills_stay_inside_the_bar_and_no_overlap() {
        let s = series(&[100.0, 101.0, 103.0, 90.0, 80.0, 82.0, 95.0, 120.0, 90.0, 60.0]);
        let g = |k: &str| if k == "turnflt" { 0.0 } else { dflt(k) };
        let sg = run_signals(&s, &g, false, false);
        let mut open = false;
        for i in 0..s.bars.len() {
            if sg.buy[i] {
                assert!(!open);
                open = true;
                assert!(sg.buy_price[i] >= s.bars[i].l && sg.buy_price[i] <= s.bars[i].h);
            }
            if sg.sell[i] {
                open = false;
            }
        }
    }

    #[test]
    fn aux_toggles_filter_only_the_real_buy() {
        // flags: Aux1 = 1 on every bar, Aux2 = 0 on every bar
        let mut s = series(&[100.0, 101.0, 102.0, 103.0, 104.0]);
        for b in s.bars.iter_mut() { b.aux1 = 1.0; b.aux2 = 0.0; }
        let with = |a1: f64, a2: f64| move |k: &str| match k { "turnflt" => 0.0, "noofloss" => 0.0, "auxtoggle" => a1, "auxtoggle1" => a2, _ => dflt(k) };
        let any = |sg: &Signals| sg.buy.iter().any(|&b| b);
        assert!(any(&run_signals(&s, &with(1.0, 1.0), false, false)), "both OFF: no filter");
        assert!(any(&run_signals(&s, &with(0.0, 1.0), false, false)), "Aux1 ON, Aux1 flag is 1 everywhere");
        assert!(!any(&run_signals(&s, &with(1.0, 0.0), false, false)), "Aux2 ON, Aux2 flag is 0 everywhere");
        assert!(!any(&run_signals(&s, &with(0.0, 0.0), false, false)), "both ON needs both flags");
        // flag only on the later bars: the buy moves to the first flagged bar
        for (i, b) in s.bars.iter_mut().enumerate() { b.aux2 = if i >= 3 { 1.0 } else { 0.0 }; }
        let sg = run_signals(&s, &with(1.0, 0.0), false, false);
        assert_eq!(sg.buy.iter().position(|&b| b), Some(3));
    }

    #[test]
    fn zero_losses_required_lets_first_ath_close_buy() {
        let s = series(&[100.0, 101.0, 102.0, 103.0]);
        let g = |k: &str| if k == "turnflt" { 0.0 } else if k == "noofloss" { 0.0 } else { dflt(k) };
        assert!(run_signals(&s, &g, false, false).buy[1]);
    }
}
