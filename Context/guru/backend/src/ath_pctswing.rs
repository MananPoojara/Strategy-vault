//! ATH "ATHC: %Swing exit" engine: the signals of the team's AFL `ATHC EXIT WITH %SWING with turnover runnig buy LATEST.afl`
//! (buy a new all-time-high close or while the all-time-high state runs, sell when the %swing line breaks), computed on
//! float32 arrays exactly as AmiBroker holds them. The generic parts (bars, `ExRem`, `Signals`, the portfolio) are in `ath_core`.
//!
//! Checked against the team's trade-list sheets (database C&P 06-09-2021 WITH T.O, watchlist NSE 500 C&P, penny 10, turnover
//! filter on, dead stock YES, 50 positions of 2 %, commission 0.65 %): see `ATH/verify/pctswing/RESULT.md`.
#![allow(dead_code)]
pub use crate::ath_core::*;

/// The AFL's parameters (defaults of the AFL; the team's runs set `exit_pct`, `penny`, `turnover`).
#[derive(Clone, Copy, Debug)]
pub struct Params {
    pub exit_pct: f32,
    pub buy_buffer_pct: f32,
    pub sell_buffer_pct: f32,
    pub penny: f32,
    pub turnover_filter: bool,
    /// "DEAD STOCK BACKTEST" = YES: no buy on the last bar and a sell on the last bar of every symbol.
    pub dead_stock: bool,
}
impl Default for Params {
    fn default() -> Self {
        Params { exit_pct: 10.0, buy_buffer_pct: 0.25, sell_buffer_pct: 0.25, penny: 0.0, turnover_filter: false, dead_stock: false }
    }
}

/// Yearly turnover threshold of the AFL (`Volsig`), compared with `OpenInt`.
fn volsig(year: i32) -> f32 {
    match year {
        y if y < 2000 => 30_000.0,
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
        2015 | 2016 => 4_000_000.0,
        2017 => 5_000_000.0,
        2018 => 6_000_000.0,
        2019 => 7_000_000.0,
        2020 => 8_000_000.0,
        _ => 9_000_000.0,
    }
}

pub fn signals(s: &Series, p: &Params) -> Signals {
    let n = s.bars.len();
    let c: Vec<f32> = s.bars.iter().map(|b| b.c).collect();
    let o: Vec<f32> = s.bars.iter().map(|b| b.o).collect();
    let h: Vec<f32> = s.bars.iter().map(|b| b.h).collect();
    let l: Vec<f32> = s.bars.iter().map(|b| b.l).collect();
    let oi: Vec<f32> = s.bars.iter().map(|b| b.oi).collect();
    // Hcl = HHV(C, BarCount): highest close up to the bar
    let mut hcl = vec![0f32; n];
    for i in 0..n {
        hcl[i] = if i == 0 || c[i] > hcl[i - 1] { c[i] } else { hcl[i - 1] };
    }
    // %swing line: EXIT = C*EXITPCT/100; Up = C + EXIT; Dn = C - EXIT (float32 array operations)
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
    let turnover: Vec<bool> = (0..n).map(|i| if p.turnover_filter { oi[i] > volsig(s.bars[i].date / 10000) } else { true }).collect();
    // buy / sell state machine (buysig, buycount, sellcount, turnovercheck)
    let (mut buysig, mut buycount, mut sellcount, mut tc) = (vec![false; n], vec![0i32; n], vec![0i32; n], vec![false; n]);
    for i in 1..n {
        if c[i] > hcl[i - 1] {
            buysig[i] = true;
            buycount[i] = buycount[i - 1] + 1;
            sellcount[i] = 0;
        } else if trend[i] == -1 {
            buysig[i] = false;
            buycount[i] = 0;
            sellcount[i] = sellcount[i - 1] + 1;
        } else {
            buysig[i] = buysig[i - 1];
            buycount[i] = if buysig[i] { buycount[i - 1] + 1 } else { 0 };
            sellcount[i] = sellcount[i - 1];
        }
        tc[i] = if buysig[i] && turnover[i] {
            true
        } else if !buysig[i] {
            false
        } else {
            tc[i - 1]
        };
    }
    // circuits and the Buy / Sell expressions
    let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
    let mut upc = vec![false; n]; // UPcircuit
    for i in 0..n {
        let uc = h[i] == c[i];
        let lc = l[i] == c[i];
        let upcircuit = prec2(o[i]) == prec2(c[i]) && prec2(o[i]) == prec2(h[i]);
        let locircuit = prec2(o[i]) == prec2(c[i]) && prec2(o[i]) == prec2(l[i]);
        upc[i] = upcircuit;
        let prev_tc = if i > 0 { tc[i - 1] } else { tc[i] };
        let sigbuy = if !prev_tc { uc } else { upcircuit };
        let dead_buy = if p.dead_stock { i != n - 1 } else { i != 0 };
        let dead_sell = p.dead_stock && i == n - 1;
        buy[i] = ((buysig[i] && buycount[i] == 1 && !uc && tc[i]) || (buysig[i] && buycount[i] > 1 && !sigbuy && tc[i])) && dead_buy && c[i] > p.penny;
        sell[i] = (!buysig[i] && sellcount[i] == 1 && !lc) || (!buysig[i] && sellcount[i] > 1 && !locircuit) || dead_sell;
    }
    let buy = exrem(&buy, &sell);
    let sell = exrem(&sell, &buy);
    // BuyPrice / SellPrice
    let mut buy_price = vec![0f32; n];
    let mut sell_price = vec![0f32; n];
    for i in 0..n {
        let prev_tc = if i > 0 { tc[i - 1] } else { tc[i] };
        buy_price[i] = if buysig[i] && buycount[i] > 1 && !upc[i] && o[i] > p.penny && prev_tc { o[i] + (o[i] * p.buy_buffer_pct) / 100.0 } else { c[i] };
        let lc = l[i] == c[i];
        sell_price[i] = if !buysig[i] && sellcount[i] == 1 && !lc { c[i] } else { o[i] - (o[i] * p.sell_buffer_pct) / 100.0 };
    }
    // AmiBroker keeps a fill price inside the bar's low-high range
    for i in 0..n {
        buy_price[i] = buy_price[i].min(h[i]).max(l[i]);
        sell_price[i] = sell_price[i].min(h[i]).max(l[i]);
    }
    Signals::new(buy, sell, buy_price, sell_price)
}

// ---------------------------------------------------------------- the variant this module exposes

/// The AFL's own parameters (keys as in `afl-params-ath.ts`); defaults are the AFL's.
fn schema() -> Vec<P> {
    vec![
        p("tstart", "TRADING START BAR (not used by this AFL's signals)", 0.0, 0.0, 500.0, 1.0),
        p("deadstock", "DEAD STOCK BACKTEST (0 YES, 1 NO)", 1.0, 0.0, 1.0, 1.0),
        p("hardstop", "Hard Stoploss % (0 = off; ApplyStop percent loss, not yet checked against a team sheet)", 0.0, 0.0, 50.0, 1.0),
        p("pennyflt", "Penny Stock Price", 0.0, 0.0, 100.0, 1.0),
        p("turnflt", "TURNOVER Filter (0 NO, 1 YES)", 0.0, 0.0, 1.0, 1.0),
        p("exitpct", "EXIT %", 10.0, 1.0, 50.0, 1.0),
        p("buffer_pct", "Buying Buffer %", 0.25, 0.0, 10.0, 0.05),
        p("bufferpct", "Selling Buffer %", 0.25, 0.0, 10.0, 0.05),
    ]
}

fn all_signals(series: &[Series], _env: &Env, g: G) -> Vec<Signals> {
    let ep = Params {
        exit_pct: g("exitpct") as f32,
        buy_buffer_pct: g("buffer_pct") as f32,
        sell_buffer_pct: g("bufferpct") as f32,
        penny: g("pennyflt") as f32,
        turnover_filter: g("turnflt").round() == 1.0,
        dead_stock: g("deadstock").round() == 0.0,
    };
    let hard = g("hardstop") as f32;
    series
        .iter()
        .map(|s| {
            let mut sg = signals(s, &ep);
            // ApplyStop(stopTypeLoss, stopModePercent, hardstop): not yet checked against a team sheet
            if hard > 0.0 {
                sg.stop = Some(StopSpec { loss_pct: hard, ..StopSpec::default() });
            }
            sg
        })
        .collect()
}

pub fn variants() -> Vec<VariantDef> {
    vec![VariantDef { id: "ytath_pctswing_exit_turnover_running_buy_2", data: Data::Cash { watchlist: "NSE 500 C&P.tls" }, schema, signals: all_signals }]
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
    fn exrem_keeps_the_first_signal_until_the_other_side_fires() {
        let a = [true, true, false, true, true, false];
        let b = [false, false, true, false, false, false];
        assert_eq!(exrem(&a, &b), vec![true, false, false, true, false, false]);
    }

    #[test]
    fn file_names_are_percent_encoded_like_the_converter() {
        assert_eq!(file_name("ABAN 2006-12"), "ABAN%202006-12.ad1");
        assert_eq!(file_name("M&M"), "M%26M.ad1");
        assert_eq!(file_name("GET&D"), "GET%26D.ad1");
        assert_eq!(file_name("NIFTY 50"), "NIFTY%2050.ad1");
    }

    #[test]
    fn a_new_all_time_high_close_buys_and_a_fall_through_the_swing_line_sells() {
        // rises to a new high each bar, then falls 20 % in one bar (the 10 % swing line is broken)
        let s = series(&[100.0, 101.0, 102.0, 103.0, 104.0, 83.0, 82.0]);
        let sg = signals(&s, &Params::default());
        assert!(sg.buy[1], "the first close above all earlier closes is a buy");
        assert!(!sg.buy[2], "ExRem: no second buy while the position is open");
        assert!(sg.sell[5], "the close below the swing line sells");
        assert_eq!(sg.buy_price[1], 101.0, "a first new high is bought at the close");
        assert!(sg.sell_price[5] >= s.bars[5].l && sg.sell_price[5] <= s.bars[5].h, "the fill stays inside the bar");
    }

    #[test]
    fn the_penny_filter_blocks_cheap_stocks() {
        let s = series(&[5.0, 6.0, 7.0]);
        let p = Params { penny: 10.0, ..Params::default() };
        assert!(signals(&s, &p).buy.iter().all(|&b| !b));
    }

    #[test]
    fn dead_stock_yes_sells_on_the_last_bar_and_never_buys_there() {
        let s = series(&[100.0, 101.0, 102.0, 103.0]);
        let p = Params { dead_stock: true, ..Params::default() };
        let sg = signals(&s, &p);
        assert!(sg.sell[3] && !sg.buy[3]);
    }

    #[test]
    fn the_portfolio_sizes_a_position_as_a_percent_of_equity_and_charges_commission_both_ways() {
        let s = series(&[100.0, 101.0, 102.0, 103.0, 104.0, 83.0, 82.0]);
        let sg = signals(&s, &Params::default());
        let pp = PortfolioParams { initial_equity: 100_000.0, pct_of_equity: 10.0, fixed_amount: None, max_positions: 5, commission_pct: 0.5, start: 20200101, end: 20201231, amibroker_profit: false, amibroker_full: false, current_equity_sizing: false };
        let t = backtest(&[(&s, &sg)], &[0], &pp);
        assert_eq!(t.len(), 1);
        let x = &t[0];
        // 10 % of 100,000 = 10,000 -> floor(10,000 / 101) = 99 shares
        assert_eq!(x.shares, 99.0);
        let expect = 99.0 * (x.exit_price - x.entry_price) - 0.005 * (99.0 * x.entry_price + 99.0 * x.exit_price);
        assert!((x.profit - expect).abs() < 1e-6);
    }

    #[test]
    fn a_candidate_that_cannot_be_funded_ends_the_entries_of_the_bar() {
        // equity 1,000: A (price 101) takes 9 shares and leaves 91; C (price 200) cannot be funded and ends the bar's entries,
        // so B (price 50, one share would fit) is not entered
        let a = series(&[100.0, 101.0, 102.0, 90.0]);
        let mut c = series(&[199.0, 200.0, 201.0, 180.0]);
        c.name = "C".into();
        let mut b = series(&[49.0, 50.0, 51.0, 45.0]);
        b.name = "B".into();
        let (sa, sc, sb) = (signals(&a, &Params::default()), signals(&c, &Params::default()), signals(&b, &Params::default()));
        let pp = PortfolioParams { initial_equity: 1_000.0, pct_of_equity: 100.0, fixed_amount: None, max_positions: 5, commission_pct: 0.0, start: 20200101, end: 20201231, amibroker_profit: false, amibroker_full: false, current_equity_sizing: false };
        let t = backtest(&[(&a, &sa), (&c, &sc), (&b, &sb)], &[0, 1, 2], &pp);
        assert!(t.iter().any(|x| x.sym == 0), "A is entered");
        assert!(t.iter().all(|x| x.sym != 1 && x.sym != 2), "C cannot be funded and B comes after it");
        // without C in between, B gets the one share that fits
        let t2 = backtest(&[(&a, &sa), (&b, &sb)], &[0, 1], &pp);
        assert!(t2.iter().any(|x| x.sym == 1 && x.shares == 1.0));
    }
}
