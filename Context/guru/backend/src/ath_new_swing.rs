//! ATH "ATHC checking" variant `ytath_chk_pctswing_turnover_running_buy`: AFL `AFL ATH checking/ATHC EXIT WITH %SWING with turnover runnig buy.afl`
//! (the plain running-ATHC engine with a Close-based %swing exit, fills at the Close, plus the Aux1/Aux2 toggles).
//!
//! Buy  = (buysig OR C > Ref(Hcl,-1)) AND NOT UPcircuit AND DEADSTOCKBUYSIG AND C > pennyflt AND turnover AND Aux filters
//! Sell = ((trend1 == -1 OR buysig == 0) AND NOT locircuit) OR deadstocksellsig
//! BuyPrice = ValueWhen(Buy, C), SellPrice = ValueWhen(Sell, C)  (no buffers). Turnover = (V*C) > Volsig (5-step table, NOT OpenInt).
//! All arithmetic on float32 arrays like AmiBroker. No team trade sheet exists for this file: see `ATH/checked/<id>/RESULT.md`.
#![allow(dead_code)]
pub use crate::ath_core::*;

#[derive(Clone, Copy, Debug)]
pub struct Params {
    pub exit_pct: f32,
    pub penny: f32,
    pub turnover_filter: bool,
    /// "DEAD STOCK BACKTEST" = YES: no buy on the last bar and a sell on the last bar of every symbol.
    pub dead_stock: bool,
    /// Aux1 toggle ON (AFL value 0): only symbols with Aux1 == 1
    pub aux1_on: bool,
    /// Aux2 toggle ON (AFL value 0): only symbols with Aux2 == 1
    pub aux2_on: bool,
}
impl Default for Params {
    /// The AFL's defaults: EXIT % 18, penny 20, turnover filter YES, dead stock NO, Aux1/Aux2 OFF.
    fn default() -> Self {
        Params { exit_pct: 18.0, penny: 20.0, turnover_filter: true, dead_stock: false, aux1_on: false, aux2_on: false }
    }
}

/// The AFL's `volsig` table (compared with V*C).
fn volsig(year: i32) -> f32 {
    match year {
        y if y < 2000 => 30_000.0,
        2000..=2004 => 50_000.0,
        2005..=2009 => 1_800_000.0,
        2010..=2014 => 2_600_000.0,
        2015..=2019 => 6_000_000.0,
        _ => 2_500_000.0,
    }
}

pub fn signals(s: &Series, p: &Params) -> Signals {
    let n = s.bars.len();
    let c: Vec<f32> = s.bars.iter().map(|b| b.c).collect();
    let o: Vec<f32> = s.bars.iter().map(|b| b.o).collect();
    let h: Vec<f32> = s.bars.iter().map(|b| b.h).collect();
    let l: Vec<f32> = s.bars.iter().map(|b| b.l).collect();
    // Hcl = HHV(C, BarCount)
    let mut hcl = vec![0f32; n];
    for i in 0..n {
        hcl[i] = if i == 0 || c[i] > hcl[i - 1] { c[i] } else { hcl[i - 1] };
    }
    // %swing line: EXIT = C*EXITPCT/100; Up = C + EXIT; Dn = C - EXIT
    let exit: Vec<f32> = c.iter().map(|&x| (x * p.exit_pct) / 100.0).collect();
    let mut up: Vec<f32> = (0..n).map(|i| c[i] + exit[i]).collect();
    let mut dn: Vec<f32> = (0..n).map(|i| c[i] - exit[i]).collect();
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
        } else {
            trend[i] = trend[i - 1];
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
    }
    // buysig: set on C > Hcl[i-1]; cleared when trend1 == -1 and the previous buysig was 1; else carried
    let mut buysig = vec![false; n];
    for i in 1..n {
        buysig[i] = if c[i] > hcl[i - 1] {
            true
        } else if trend[i] == -1 && buysig[i - 1] {
            false
        } else {
            buysig[i - 1]
        };
    }
    let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
    for i in 0..n {
        let upcircuit = prec2(o[i]) == prec2(c[i]) && prec2(o[i]) == prec2(h[i]);
        let locircuit = prec2(o[i]) == prec2(c[i]) && prec2(o[i]) == prec2(l[i]);
        let turnover = if p.turnover_filter { s.bars[i].v * c[i] > volsig(s.bars[i].date / 10000) } else { true };
        let dead_buy = if p.dead_stock { i != n - 1 } else { i != 0 };
        let dead_sell = p.dead_stock && i == n - 1;
        let running_new_high = i >= 1 && c[i] > hcl[i - 1]; // Ref(Hcl,-1) is Null on bar 0: compares false
        let aux_ok = (!p.aux1_on || s.bars[i].aux1 == 1.0) && (!p.aux2_on || s.bars[i].aux2 == 1.0);
        buy[i] = (buysig[i] || running_new_high) && !upcircuit && dead_buy && c[i] > p.penny && turnover && aux_ok;
        sell[i] = ((trend[i] == -1 || !buysig[i]) && !locircuit) || dead_sell;
    }
    let buy = exrem(&buy, &sell);
    let sell = exrem(&sell, &buy);
    // BuyPrice = ValueWhen(Buy, C), SellPrice = ValueWhen(Sell, C): the Close of the signal bar (kept inside [Low, High])
    let bp: Vec<f32> = (0..n).map(|i| c[i].min(h[i]).max(l[i])).collect();
    Signals::new(buy, sell, bp.clone(), bp)
}

// ---------------------------------------------------------------- the variant this module exposes

/// Keys as in `afl-params-ath.ts` / the catalog; toggles are 0-based like the other ATH engines (0 = first item of the AFL's list).
fn schema() -> Vec<P> {
    vec![
        p("tstart", "TRADING START BAR (not used by this AFL's signals)", 0.0, 0.0, 500.0, 1.0),
        p("deadstock", "DEAD STOCK BACKTEST (0 YES, 1 NO)", 1.0, 0.0, 1.0, 1.0),
        p("hardstop", "Hard Stoploss % (0 = off; ApplyStop percent loss, not checked against a team sheet)", 0.0, 0.0, 50.0, 1.0),
        p("pennyflt", "Penny Stock Price", 20.0, 0.0, 100.0, 1.0),
        p("turnflt", "TURNOVER Filter (0 NO, 1 YES; V*C > Volsig table)", 1.0, 0.0, 1.0, 1.0),
        p("auxtoggle", "Aux1 (0 ON = only Aux1 == 1; 1 OFF)", 1.0, 0.0, 1.0, 1.0),
        p("auxtoggle1", "Aux2 (0 ON = only Aux2 == 1; 1 OFF)", 1.0, 0.0, 1.0, 1.0),
        p("exitpct", "EXIT %", 18.0, 5.0, 22.0, 1.0),
    ]
}

fn all_signals(series: &[Series], _env: &Env, g: G) -> Vec<Signals> {
    let ep = Params {
        exit_pct: g("exitpct") as f32,
        penny: g("pennyflt") as f32,
        turnover_filter: g("turnflt").round() == 1.0,
        dead_stock: g("deadstock").round() == 0.0,
        aux1_on: g("auxtoggle").round() == 0.0,
        aux2_on: g("auxtoggle1").round() == 0.0,
    };
    let hard = g("hardstop") as f32;
    series
        .iter()
        .map(|s| {
            let mut sg = signals(s, &ep);
            if hard > 0.0 {
                sg.stop = Some(StopSpec { loss_pct: hard, ..StopSpec::default() });
            }
            sg
        })
        .collect()
}

pub fn variants() -> Vec<VariantDef> {
    vec![VariantDef { id: "ytath_chk_pctswing_turnover_running_buy", data: Data::Cash22 { watchlist: "NSE 500 C&P.tls" }, schema, signals: all_signals }]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bar(date: i32, o: f32, h: f32, l: f32, c: f32) -> Bar {
        Bar { date, c, o, h, l, v: 1.0e6, oi: 0.0, aux1: 0.0, aux2: 1.0 }
    }
    fn series(closes: &[f32]) -> Series {
        let bars = closes.iter().enumerate().map(|(i, &c)| bar(20200101 + i as i32, c, c + 1.0, c - 1.0, c)).collect();
        Series { name: "T".into(), bars }
    }
    fn plain() -> Params {
        Params { penny: 0.0, turnover_filter: false, ..Params::default() }
    }

    #[test]
    fn new_high_buys_at_close_and_swing_break_sells_at_close() {
        // exit 18 %: 104 -> 80 breaks the line (104*0.82 = 85.3)
        let s = series(&[100.0, 101.0, 102.0, 103.0, 104.0, 80.0, 79.0]);
        let sg = signals(&s, &plain());
        assert!(sg.buy[1] && !sg.buy[2]);
        assert_eq!(sg.buy_price[1], 101.0);
        assert!(sg.sell[5] && sg.sell_price[5] == 80.0);
    }

    #[test]
    fn penny_filter_defaults_to_20_and_blocks_cheap_stocks() {
        let s = series(&[5.0, 6.0, 7.0]);
        assert!(signals(&s, &Params { turnover_filter: false, ..Params::default() }).buy.iter().all(|&b| !b));
    }

    #[test]
    fn turnover_filter_uses_volume_times_close() {
        let mut s = series(&[100.0, 101.0, 102.0]);
        for b in s.bars.iter_mut() {
            b.v = 100.0; // V*C = ~10,000 < 2,500,000 (2020)
        }
        let on = Params { penny: 0.0, ..Params::default() };
        assert!(signals(&s, &on).buy.iter().all(|&b| !b));
        assert!(signals(&s, &plain()).buy.iter().any(|&b| b));
    }

    #[test]
    fn volsig_table() {
        assert_eq!(volsig(1999), 30_000.0);
        assert_eq!(volsig(2004), 50_000.0);
        assert_eq!(volsig(2005), 1_800_000.0);
        assert_eq!(volsig(2014), 2_600_000.0);
        assert_eq!(volsig(2019), 6_000_000.0);
        assert_eq!(volsig(2020), 2_500_000.0);
    }

    #[test]
    fn aux_toggles_filter_only_when_on() {
        let s = series(&[100.0, 101.0, 102.0]); // aux1 = 0, aux2 = 1
        assert!(signals(&s, &Params { aux1_on: true, ..plain() }).buy.iter().all(|&b| !b), "Aux1 ON needs Aux1 == 1");
        assert!(signals(&s, &Params { aux2_on: true, ..plain() }).buy.iter().any(|&b| b));
        assert!(signals(&s, &plain()).buy.iter().any(|&b| b));
    }

    #[test]
    fn upper_circuit_blocks_buy_and_lower_circuit_blocks_sell() {
        let mut s = series(&[100.0, 101.0, 102.0, 70.0]);
        s.bars[1] = bar(20200102, 101.0, 101.0, 100.0, 101.0); // O == H == C
        assert!(!signals(&s, &plain()).buy[1]);
        assert!(signals(&s, &plain()).buy[2], "the running state buys on the next bar");
        let mut s2 = series(&[100.0, 101.0, 102.0, 70.0]);
        s2.bars[3] = bar(20200104, 70.0, 75.0, 70.0, 70.0); // O == C == L
        assert!(!signals(&s2, &plain()).sell[3]);
    }

    #[test]
    fn dead_stock_yes_sells_on_the_last_bar() {
        let s = series(&[100.0, 101.0, 102.0, 103.0]);
        let sg = signals(&s, &Params { dead_stock: true, ..plain() });
        assert!(sg.sell[3] && !sg.buy[3]);
    }

    #[test]
    fn schema_defaults_are_the_afls() {
        let sc = schema();
        let d = |k: &str| sc.iter().find(|x| x.key == k).unwrap().def;
        assert_eq!((d("exitpct"), d("pennyflt"), d("turnflt"), d("auxtoggle"), d("auxtoggle1"), d("deadstock")), (18.0, 20.0, 1.0, 1.0, 1.0, 1.0));
    }
}
