//! Perry J. Kaufman, *Smarter Trading: Improving Performance in Changing Markets* (McGraw-Hill 1995, 268 scanned
//! pages). Page refs are printed pages (PDF page = printed + 13). Only Chapters 5, 6, 8 hold complete mechanical
//! systems (with the book's own TeleTrac / TradeStation code); the rest is testing method and discussion (see
//! library/kfst/README.md). Every default is the book's literal number or is labelled.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

/// Admitted 2026-10-24 (authors/kaufman/admission/SOURCE-CONTRACT.md): independent ledgers, US and India, 0 mismatches.
pub const REVIEWED: &[&str] = &["kfst_exp_pt", "kfst_exp_stop"]; // kfst_ama held: its Box 8-5 filter is in price points (engine guard)
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

pub const BOOK: &str = "kfst";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, dir, status, rules, notes| StrategyDef {
        id, name, group, session: 0, dir, status, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None,
    };
    use Status::*;
    vec![
        d("kfst_exp_pt", "Exponential trend with profit-taking", "Ch 5 · Profit-taking", "Long/Short", Live, &[
            "F|\"@exp_ma(price,period) = ema[1] + sc*(price - ema[1]) ... sc = 2/(period+1)\" (p.76)",
            "E|\"1. Buy on the close when the exponential moving average turns up; sell when it turns down.\" (p.76); code: buy = exp-exp[1]>0 & exp[1]-exp[2]<0, sell = exp-exp[1]<0 & exp[1]-exp[2]>0 (Box 5-1, p.78)",
            "X|\"2. Close out the trade if the unrealized (open) profits, as of the closing price, are greater than a preset profit level, PT = p × close, where p is a percentage. Exit the trade at the closing price of the day.\" (p.76); code: ptrule = openbs*100/close > ptlevel; strategy = Trade(buy, sell | ptrule, sell, buy | ptrule) (Box 5-1)",
            "F|\"3. Do not reset a trade in the same direction once profits have been taken.\" (p.76)",
        ], "Defaults = Box 5-1 code: speed 40, ptlevel 3.0 (%). The tests (Table 5-1, p.79) sweep speeds 5-75 and levels 100%-1.5% on the Hang Seng index, Deutsche mark and Unilever (none in the vault). The opposite trend turn reverses; after a profit exit the system waits for the next turn. The book's high/low variant (profit taken intraday, Box 5-1 last lines) is not coded."),
        d("kfst_exp_stop", "Exponential trend with percentage stop-loss", "Ch 6 · Stops", "Long/Short", Live, &[
            "E|\"A buy signal was given when the trendline turned up, @Exp_MA > @Exp_MA[1]. A sell signal was given when the trendline turned down, @Exp_MA < @Exp_MA[1].\" (p.94)",
            "X|\"The trade was exited if the loss, at the close, exceeded a preset percentage level (the stop-loss), or when a reverse buy or sell signal occurred.\" (p.94)",
            "F|\"All orders were executed on the close of the local trading day. There were no commissions or slippage charged to any trade.\" (p.94)",
        ], "Readings: a signal is the turn of the trendline (the same events as Box 5-1); after a stop the system waits for a new turn ('there is no new signal to buy or sell', p.104). Loss % = open loss x 100 / close (the book's percentage convention in Box 5-1). Defaults: speed 40 [book silent; the Box 5-1 speed], stop 0.50% [Ch 6 highlights '.50 percent', p.95, 100]. Table 6-3/6-4 test speeds 5-300 and stops .02-10% on Chrysler, Siemens, Deutsche mark and Eurodollars (none in the vault)."),
        d("kfst_ama", "Adaptive Moving Average, turn-filter system (Box 8-5)", "Ch 8 · Smarter trend-following", "Long/Short", Live, &[
            "F|\"signal = @AbsValue(close - close[period]); noise = @Summation(diff,period); efratio = signal/noise; smooth = @Power(efratio*(fastend - slowend) + slowend,2); AMA = AMA[1] + smooth*(close - AMA[1])\" period(10), fastend(.666), slowend(.0645) (Box 8-5, p.151)",
            "F|\"if(AMA > AMA[1] and AMA[1] < AMA[2]) then extlow = AMA[1]; if(AMA < AMA[1] and AMA[1] > AMA[2]) then exthigh = AMA[1]\" (Box 8-5)",
            "E|\"if(currentbar > period + 5) then begin if(AMA > AMA[1] and AMA - extlow > filter) then buy on close; if(AMA < AMA[1] and exthigh - AMA > filter) then sell on close\" filter(.1) (Box 8-5); \"Buy when AMA - @lowest(AMA,n) > filter; Sell when @highest(AMA,n) - AMA > filter\" (p.145)",
            "X|Always in: \"enter and hold a long position when the trendline moves up, and reverse to a short position when the trendline turns down\" (p.143)",
        ], "Default = the Box 8-5 system code: filter .1 in price points (on an index this is almost no filter). The text defines the filter as a percentage of one standard deviation of the AMA changes ('filter = percentage x @std_dev(AMA - AMA[1],n)', p.144, n normally 20, .10 in the Box 8-3 spreadsheet): p.fmode=1 uses .1 x population std dev of the last 20 one-day AMA changes. Differs from kftsm_kama (TSM4 CD driver: one-day change against the filter)."),
        d("kfst_multi_pt", "Several profit objectives", "Ch 5 · Profit-taking", "Long/Short", NotTestable("illustrative arithmetic only; no entry rule or target levels for a system (p.83-88)"), &[
            "X|\"exit at $53, $55, and $57\" style multiple objectives (p.83-86)",
        ], ""),
        d("kfst_er_profit", "Efficiency-ratio profit-taking", "Ch 8 · Smarter trend-following", "Long/Short", NotTestable("'take profits whenever the value exceeds a preset level' - no level given (p.146)"), &[
            "X|\"It would be best simply to take profits whenever the value exceeds a preset level\" (p.146)",
        ], ""),
        d("kfst_filters", "Price-level and volatility entry filters", "Ch 11 · Improving performance", "Long/Short", NotTestable("filter levels are read visually from scatter plots (p.227-235)"), &[
            "F|\"Visually identify the levels of highest risk and consistent losses\" (p.235)",
        ], ""),
        d("kfst_reverse_opt", "Reversing the optimisation (fade the fast model)", "Ch 11 · Improving performance", "Long/Short", NotTestable("the long- and short-term models are not specified (p.235-236)"), &[
            "E|\"Enter 1/3 of the trade when the long-term model gives a (buy) signal. Then enter 1/3 ... when the short-term model closes out a (long) position. Finally enter 1/3 ... when the short-term model enters an opposite (short) position.\" (p.236)",
        ], ""),
        d("kfst_step_forward", "CAC-40 step-forward test", "Ch 10 · Testing for robustness", "Long/Short", NotTestable("a parameter re-selection procedure on MATIF CAC-40 futures with a filter in CAC points (p.190-191)"), &[
            "E|\"A buy signal that occurred when the trend turned up by the amount of the filter; a sell signal that was generated when the trend turned down by the amount of the filter value\" (Box 10-2, p.190)",
        ], "The base rule is the exponential trend with a points filter (kftsm_exp family); the annual re-optimisation is a research method, not a strategy."),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "kfst_exp_pt" | "kfst_exp_stop" | "kfst_multi_pt" => ("kfst_exp", "Smarter Trading exponential trend", ""),
        s if s.starts_with("kfst_") => ("kfst_other", "Smarter Trading", ""),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(match id {
        "kfst_exp_pt" => vec![p("speed", "Exponential trend speed (days)", 40., 2., 200., 1.), p("ptlevel", "Profit-taking level (% of close)", 3., 0.5, 100., 0.5)],
        "kfst_exp_stop" => vec![p("speed", "Exponential trend speed (days)", 40., 2., 300., 1.), p("stop", "Stop-loss (% loss at the close)", 0.5, 0.02, 10., 0.01)],
        "kfst_ama" => vec![p("period", "Efficiency ratio period", 10., 2., 50., 1.), p("filter", "Filter (points, or x std dev when fmode=1)", 0.1, 0., 5., 0.01), p("fmode", "0 = code (points), 1 = text (x std dev of AMA changes)", 0., 0., 1., 1.), p("sdays", "Std-dev days (fmode=1)", 20., 5., 60., 1.)],
        "kfst_multi_pt" | "kfst_er_profit" | "kfst_filters" | "kfst_reverse_opt" | "kfst_step_forward" => vec![],
        _ => return None,
    })
}

struct Bk<'a> {
    x: &'a Ctx<'a>,
    from: usize,
    to: usize,
    out: Vec<Trade>,
    cur: Option<Trade>,
}
impl<'a> Bk<'a> {
    fn new(x: &'a Ctx<'a>, from: usize, to: usize) -> Self { Bk { x, from, to, out: vec![], cur: None } }
    fn pos(&self) -> i8 { self.cur.as_ref().map_or(0, |t| t.dir as i8) }
    fn at_close(&mut self, i: usize, dir: i8, why: &str) {
        if i < self.from || dir == self.pos() { return; }
        let px = self.x.c[i];
        if let Some(mut t) = self.cur.take() { t.exit_i = i; t.exit_px = px; t.signal_i = i; t.reason = why.into(); self.out.push(t); }
        if dir != 0 && i < self.to {
            self.cur = Some(Trade { dir: dir as f64, legs: vec![Leg { i, px, w: 1. }], exit_i: i, exit_px: px, reason: String::new(), entry_kind: "close", signal_i: i, open: false, exits: vec![] });
        }
    }
    fn entry(&self) -> f64 { self.cur.as_ref().map_or(f64::NAN, |t| t.legs[0].px) }
    fn done(mut self) -> Vec<Trade> {
        if let Some(mut t) = self.cur.take() {
            t.exit_i = self.to; t.exit_px = self.x.c[self.to]; t.signal_i = self.to; t.reason = "open".into(); t.open = true;
            self.out.push(t);
        }
        self.out
    }
}

fn expo(v: &[f64], a: f64) -> Vec<f64> {
    let mut out = vec![f64::NAN; v.len()];
    if v.is_empty() { return out; }
    out[0] = v[0];
    for i in 1..v.len() { out[i] = out[i - 1] + a * (v[i] - out[i - 1]); }
    out
}
/// +1 when the trendline turns up today, -1 when it turns down (Box 5-1: exp-exp[1] vs exp[1]-exp[2]).
fn turns(e: &[f64]) -> Vec<i8> {
    (0..e.len()).map(|i| if i < 2 { 0 } else {
        let (d0, d1) = (e[i] - e[i - 1], e[i - 1] - e[i - 2]);
        if d0 > 0. && d1 < 0. { 1 } else if d0 < 0. && d1 > 0. { -1 } else { 0 }
    }).collect()
}

/// Trend turns with an open-trade exit rule evaluated on the close; `exit_if(dir, entry, close)`.
fn turn_system(x: &Ctx, from: usize, to: usize, speed: usize, exit_if: impl Fn(f64, f64, f64) -> bool, why: &str) -> Vec<Trade> {
    let c = &x.c;
    let t = turns(&expo(c, 2. / (speed as f64 + 1.)));
    // While the strategy holds a trade it shares the base system's entry (Box 5-1 'openbs'), so the open result is
    // measured from the strategy's own entry; once out, it waits for the next turn.
    let mut b = Bk::new(x, from, to);
    for i in 0..=to {
        if t[i] != 0 { b.at_close(i, t[i], "trend turned"); }
        else if b.pos() != 0 && exit_if(b.pos() as f64, b.entry(), c[i]) { b.at_close(i, 0, why); }
    }
    b.done()
}

fn ama(c: &[f64], pn: usize) -> Vec<f64> {
    let n = c.len();
    let mut a = vec![f64::NAN; n];
    for i in 0..n {
        if i + 1 <= pn { a[i] = c[i]; continue; } // currentbar <= period: AMA = close
        let sig = (c[i] - c[i - pn]).abs();
        let noise: f64 = (i + 1 - pn..=i).map(|k| (c[k] - c[k - 1]).abs()).sum();
        let er = if noise > 0. { sig / noise } else { 0. };
        let sm = (er * (0.666 - 0.0645) + 0.0645f64).powi(2);
        a[i] = a[i - 1] + sm * (c[i] - a[i - 1]);
    }
    a
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if from > to || to >= x.r.len() { return None; }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(0., |d| d.def));
    let u = |k: &str| g(k).round().max(0.) as usize;
    Some(match id {
        "kfst_exp_pt" => {
            let pt = g("ptlevel");
            turn_system(x, from, to, u("speed").max(1), |d, e, c| d * (c - e) * 100. / c > pt, "profit objective reached")
        }
        "kfst_exp_stop" => {
            let st = g("stop");
            turn_system(x, from, to, u("speed").max(1), |d, e, c| -d * (c - e) * 100. / c > st, "stop-loss at the close")
        }
        "kfst_ama" => {
            let pn = u("period").max(1);
            let c = &x.c;
            let a = ama(c, pn);
            let (fil, mode, sdays) = (g("filter"), u("fmode"), u("sdays").max(2));
            let mut b = Bk::new(x, from, to);
            let (mut extlow, mut exthigh) = (0f64, 0f64);
            for i in 2..=to {
                if a[i] > a[i - 1] && a[i - 1] < a[i - 2] { extlow = a[i - 1]; }
                if a[i] < a[i - 1] && a[i - 1] > a[i - 2] { exthigh = a[i - 1]; }
                if i + 1 <= pn + 5 { continue; }
                let f = if mode == 1 {
                    if i < sdays { continue; }
                    let ch: Vec<f64> = (i + 1 - sdays..=i).map(|k| a[k] - a[k - 1]).collect();
                    let m = ch.iter().sum::<f64>() / sdays as f64;
                    fil * (ch.iter().map(|v| (v - m) * (v - m)).sum::<f64>() / sdays as f64).sqrt()
                } else { fil };
                if a[i] > a[i - 1] && a[i] - extlow > f { b.at_close(i, 1, "AMA up from its last low by more than the filter"); }
                if a[i] < a[i - 1] && exthigh - a[i] > f { b.at_close(i, -1, "AMA down from its last high by more than the filter"); }
            }
            b.done()
        }
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn profit_exit_waits_for_a_new_turn() {
        // V-shaped series: the EMA turns up after the bottom; a 2% profit closes the long; no re-entry while still up
        let v: Vec<f64> = (0..60).map(|i| if i < 20 { 120. - i as f64 } else { 100. + (i - 20) as f64 }).collect();
        let r: Vec<Row> = v.iter().enumerate().map(|(i, &c)| Row { ticker: "T".into(), date: format!("2000-02-{:02}", i % 28 + 1), open: c, high: c, low: c, close: c, volume: None }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let ts = run("kfst_exp_pt", &x, 2, 59, &Params::from([("speed".into(), 3.), ("ptlevel".into(), 2.)])).unwrap();
        assert_eq!(ts.len(), 1);
        let t = &ts[0];
        assert_eq!(t.dir, 1.);
        assert!(!t.open && (t.exit_px - t.legs[0].px) * 100. / t.exit_px > 2.);
    }
}
