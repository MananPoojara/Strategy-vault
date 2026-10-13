//! Perry J. Kaufman, *The Best (Public) Systems I Have Known* (TAG 1999 workshop, Las Vegas; "The Best (Public)
//! Trading Methods I've Found for Futures & Equities", INO, 63 scanned pages). Slide numbers = the workshop's printed
//! page numbers. The workshop prints the TSM Divergence code (p.56-58) and names the TradeStation programs it used;
//! the 2-Systems programs are the same programs printed on the TSM 4th-edition CD (Chapter21, "not included in text"),
//! whose inputs match the workshop charts (e.g. "TSM 2sys TR/CT(1,60,1,8)"). Other workshop systems (MA, EXP, LRS,
//! NDB, Swing, Moon trading) are already in kftsm and are listed as duplicates in library/kfbpm/README.md.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "kfbpm";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, dir, status, rules, notes| StrategyDef {
        id, name, group, session: 0, dir, status, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None,
    };
    use Status::*;
    vec![
        d("kfbpm_ma_ct", "Moving average countertrend (8-day)", "S&P 500 countertrend", "Long/Short", Live, &[
            "E|\"Use a simple moving average system; Buy when the moving average turns down; Sell when the moving average turns up\" (slide 31); \"Test using TSM Mavg CT\" = -TSMMAsignal(period,0,0)",
            "F|\"S&P 500 Best Countertrend... 8 Day MA\"; chart input \"TSM Mavg CT(8,0,0)\"; \"Take the opposite position each time the 8-day moving average changes direction\" (slide 33)",
            "X|Always in: each change of direction reverses the position (slide 33)",
        ], "TSMMAsignal (TSM4 CD Ch08): change = (MA - MA[1])*100/close[1]; +1 if change > 0, -1 if change < 0, else unchanged; the countertrend system takes the negative. Orders on the close. Slide 32: S&P 500 futures, 10 years through 6/99, $25 commission, CT periods 1-20 (8: 392 trades, 70% profitable)."),
        d("kfbpm_2sys_trct", "2-Systems trend / countertrend (60-day trend, 8-day CT entry)", "Trend / countertrend", "Long/Short", Live, &[
            "F|\"the first system is treated as a trend signal and the second system is treated as a countertrend signal. Typically, the first system serves as the underlying trend while the entry comes from the second system, e.g., buying when a faster trend gives a sell signal.\" (TSM 2Sys TR/CT, TSM4 CD Ch21)",
            "E|\"if signal[1] <> 1 and signal1=1 and signal2=1 then buy on close\"; \"if signal[1] <> -1 and signal1=-1 and signal2=-1 then sell on close\" with signal2 = -TSMMAsignal(period2,0,0) (CD code)",
            "X|\"if signal[1] <> 0 and signal1 <> signal2 then exitlong on close; exitshort on close\" (CD code)",
            "F|S&P inputs \"TSM 2sys TR/CT(1,60,1,8)\": 60-day MA trend, 8-day MA countertrend (slide 36); \"entering trades in the direction of the long-term trend but on short-term countertrend signals\" (slide 39)",
        ], "Defaults = the workshop's S&P test (trend MA 60, countertrend MA 8; slide 35-36 and 40) [the CD code's own inputs are 1,20,4,20]. System types: 1 MA (TSMMAsignal), 2 exponential (TSMExpSignal, SmoothedAverage), 3 linear regression slope (TSMLRSsignal), 4 N-day breakout (TSMNDBsignal). Slide 35: S&P 500 10 years, $25 commission, trend 60 with CT 2-20 (8: 244 trades, 62% profitable). Orders on the close."),
        d("kfbpm_2sys", "2-Systems combined trend (both systems agree)", "Trend / countertrend", "Long/Short", Live, &[
            "F|\"This program combine 2 popular trading systems and nets signals\" (TSM 2-Systems, TSM4 CD Ch21); \"Although all long-term trends should generate profits, I prefer a linear regression ... For short-term trends, I only look at breakouts\" (slide 15)",
            "E|\"if signal[1] <> 1 and signal1=1 and signal2=1 then buy on close\"; \"if signal[1] <> -1 and signal1=-1 and signal2=-1 then sell on close\" (CD code)",
            "X|\"if signal[1] <> 0 and signal1 <> signal2 then exitlong on close; exitshort on close\" (CD code)",
        ], "Defaults = the CD program inputs systype1(1), period1(20), systype2(4), period2(20) [the workshop prints no S&P setting]. Workshop 'Kaufman' choices (slide 29): US bonds LRS 90 / BO 30-18, Japanese yen LRS 70 / BO 27 or 18, Amazon LRS 110 / BO 15, AOL LRS 110 / BO 9 or 15-30, IBM LRS 100 / BO 3, DELL LRS 90 / BO 3 (e.g. p.type1=3 p.period1=90 p.type2=4 p.period2=30, the bonds chart 'TSM 2-Systems(3,90,4,30)', slide 14)."),
        d("kfbpm_divergence", "TSM Divergence (stochastic / price swing divergence)", "Divergence", "Long/Short", Live, &[
            "F|Swings: a swing high is fixed when the low falls below curhigh - curhigh*swing%, a swing low when the high rises above curlow + curlow*swing%; SThigh/STlow = SlowD(length) at the swing extreme (TSM Divergence code, slides 56-58)",
            "E|Single divergence sell: \"last = -1 and exittype <> -1 and close > highclose and high = chighprice and STslowk > fastx and (STtoday[1] > 50+exit or STtoday < STtoday[1]) and STtoday < SThigh - strength then sell on close\" (buy mirror) (slide 57)",
            "X|\"if (STtoday > SThigh or STtoday=100) and signal = -1 then exitshort on close\"; \"if STslowk <= fastx or (STtoday[1] < 50+exit and STtoday > STtoday[1]) and signal = -1 then exitshort on close\" (long mirror) (slide 58)",
            "F|Rules in words: SlowK greater than extreme (90%), SlowD above exit level (50%) or declining, SlowD lower than last SlowD at the swing high by the minimum strength, current price high the high for the move and greater than the previous swing high (slide 44); exit if SlowD moves above prior SlowD swing high, SlowK below low extreme (10%), SlowD below exit level then up (slide 45)",
        ], "Defaults = the printed code inputs diverge(1), swing(2.0), strength(5), length(5), type(0), exit(0), fastx(10). Slide 47 (S&P 500, 10 years to 6/30/99): swing 2.5, stochastic length 10, strength 0-10 -> 25 to 13 trades. SlowK/SlowD = legacy TradeStation smoothing ((2*prev + new)/3, seeded) [p.smooth=1: 3-bar averages]. EasyLanguage precedence (AND before OR) kept: 'STslowk <= fastx' exits a short whatever the signal variable. Orders in a bar are applied in code order. p.diverge=2 = the printed double divergence. Type 0 (prices) only; the 3-month-rate and bond inversions do not apply to an index. The code itself warns: 'THERE IS NO ASSURANCE THAT ALL COMBINATIONS ... WILL RUN CORRECTLY'."),
        d("kfbpm_2sys_overview", "Choosing trend speeds by 'ocular test'", "Trend / countertrend", "Long/Short", NotTestable("parameter selection by visual inspection of optimisation scatter plots (slides 16-29)"), &[
            "F|\"The 'ocular test' using a sensible visual presentation gives more robust results\" (slide 29)",
        ], "The resulting per-market settings are listed in kfbpm_2sys notes."),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "kfbpm_2sys" | "kfbpm_2sys_trct" | "kfbpm_2sys_overview" => ("kfbpm_2sys", "Kaufman 2-Systems", ""),
        "kfbpm_ma_ct" => ("kfbpm_2sys", "Kaufman 2-Systems", "Countertrend only"),
        "kfbpm_divergence" => ("kfbpm_divergence", "TSM Divergence", ""),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let two = |t1: f64, p1: f64, t2: f64, p2: f64| vec![
        p("type1", "System 1: 1 MA, 2 exponential, 3 regression slope, 4 N-day breakout", t1, 1., 4., 1.), p("period1", "System 1 period", p1, 1., 250., 1.),
        p("type2", "System 2: 1 MA, 2 exponential, 3 regression slope, 4 N-day breakout", t2, 1., 4., 1.), p("period2", "System 2 period", p2, 1., 250., 1.)];
    Some(match id {
        "kfbpm_ma_ct" => vec![p("period", "Moving average period", 8., 1., 50., 1.)],
        "kfbpm_2sys_trct" => two(1., 60., 1., 8.),
        "kfbpm_2sys" => two(1., 20., 4., 20.),
        "kfbpm_divergence" => vec![p("diverge", "1 single, 2 double divergence", 1., 1., 2., 1.), p("swing", "Price swing (%)", 2., 0.5, 10., 0.25), p("strength", "Divergence strength (stochastic points)", 5., 0., 20., 1.),
            p("length", "Stochastic length", 5., 3., 30., 1.), p("exit", "Exit level offset from 50", 0., 0., 40., 1.), p("fastx", "SlowK close-out extreme", 10., 0., 40., 1.), p("smooth", "0 legacy TradeStation SlowK/SlowD, 1 = 3-bar averages", 0., 0., 1., 1.), p("ext", "SlowK entry extreme: 0 code (> fastx), 1 slide 44 (> 100-fastx)", 0., 0., 1., 1.)],
        "kfbpm_2sys_overview" => vec![],
        _ => return None,
    })
}

// ---------------------------------------------------------------- order book

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
    fn exit(&mut self, i: usize, px: f64, why: &str) {
        if let Some(mut t) = self.cur.take() {
            t.exit_i = i; t.exit_px = px; t.signal_i = i; t.reason = why.into();
            self.out.push(t);
        }
    }
    fn at_close(&mut self, i: usize, dir: i8, why: &str) {
        if i < self.from || dir == self.pos() { return; }
        let px = self.x.c[i];
        if self.cur.is_some() { self.exit(i, px, why); }
        if dir != 0 && i < self.to {
            self.cur = Some(Trade { dir: dir as f64, legs: vec![Leg { i, px, w: 1. }], exit_i: i, exit_px: px, reason: String::new(), entry_kind: "close", signal_i: i, open: false, exits: vec![] });
        }
    }
    fn done(mut self) -> Vec<Trade> {
        if let Some(mut t) = self.cur.take() {
            t.exit_i = self.to; t.exit_px = self.x.c[self.to]; t.signal_i = self.to; t.reason = "open".into(); t.open = true;
            self.out.push(t);
        }
        self.out
    }
}

// ---------------------------------------------------------------- TSM signal functions (TSM4 CD Ch08)

fn latch(ch: &[f64]) -> Vec<i8> {
    let mut s = vec![0i8; ch.len()];
    let mut cur = 0;
    for i in 0..ch.len() { if ch[i] > 0. { cur = 1 } else if ch[i] < 0. { cur = -1 } s[i] = cur; }
    s
}
/// change of a trendline in % of the previous close (TSMMAsignal / TSMExpSignal with filter 0, lag 0).
fn pct_change(c: &[f64], tr: &[f64]) -> Vec<f64> {
    (0..c.len()).map(|i| if i == 0 { f64::NAN } else { (tr[i] - tr[i - 1]) * 100. / c[i - 1] }).collect()
}
/// TradeStation SmoothedAverage: n-bar average first, then (prev*(n-1)+price)/n.
fn smoothed(v: &[f64], n: usize) -> Vec<f64> {
    let mut out = vec![f64::NAN; v.len()];
    if v.len() < n || n == 0 { return out; }
    let mut prev = v[..n].iter().sum::<f64>() / n as f64;
    out[n - 1] = prev;
    for i in n..v.len() { prev = (prev * (n as f64 - 1.) + v[i]) / n as f64; out[i] = prev; }
    out
}
/// TSMLRslope: least-squares slope of the last n closes, x = 1..n with the newest = n.
fn lr_slope(v: &[f64], n: usize, i: usize) -> f64 {
    if i + 1 < n || n < 2 { return f64::NAN; }
    let nf = n as f64;
    let sumx = nf * (nf + 1.) / 2.;
    let sumx2 = nf * (nf + 1.) * (2. * nf + 1.) / 6.;
    let (mut sumy, mut sumxy) = (0., 0.);
    for k in 0..n { let y = v[i - k]; sumy += y; sumxy += (nf - k as f64) * y; }
    let bot = nf * sumx2 - sumx * sumx;
    if bot != 0. { (nf * sumxy - sumx * sumy) / bot } else { 0. }
}
fn tsm_signal(x: &Ctx, kind: usize, pn: usize) -> Vec<i8> {
    let (h, l, c, n) = (&x.h, &x.l, &x.c, x.c.len());
    let pn = pn.max(1);
    match kind {
        2 => latch(&pct_change(c, &smoothed(c, pn))),
        3 => latch(&(0..n).map(|i| lr_slope(c, pn.max(2), i)).collect::<Vec<_>>()),
        4 => {
            let mut s = vec![0i8; n];
            let mut cur = 0;
            for i in 0..n {
                if i >= pn {
                    let nh = h[i - pn..i].iter().copied().fold(f64::NEG_INFINITY, f64::max);
                    let nl = l[i - pn..i].iter().copied().fold(f64::INFINITY, f64::min);
                    if h[i] > nh && c[i] > c[i - 1] { cur = 1; }
                    if l[i] < nl && c[i] < c[i - 1] { cur = -1; }
                }
                s[i] = cur;
            }
            s
        }
        // MA[i] - MA[i-1] = (c[i] - c[i-n])/n exactly: the sign is taken from the closes, so a flat average is never
        // turned into a signal by floating-point noise (TradeStation's Average sums the window afresh each bar)
        _ => latch(&(0..n).map(|i| if i >= pn { c[i] - c[i - pn] } else { f64::NAN }).collect::<Vec<_>>()),
    }
}

// ---------------------------------------------------------------- stochastic (legacy TradeStation SlowK / SlowD)

fn slow_kd(x: &Ctx, n: usize, avg3: bool) -> (Vec<f64>, Vec<f64>) {
    let len = x.c.len();
    let mut fk = vec![f64::NAN; len];
    for i in 0..len {
        if i + 1 < n { continue; }
        let hh = x.h[i + 1 - n..=i].iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let ll = x.l[i + 1 - n..=i].iter().copied().fold(f64::INFINITY, f64::min);
        fk[i] = if hh > ll { (x.c[i] - ll) / (hh - ll) * 100. } else { 0. };
    }
    if avg3 {
        let a3 = |v: &[f64]| -> Vec<f64> { (0..v.len()).map(|i| if i >= 2 { (v[i - 2] + v[i - 1] + v[i]) / 3. } else { f64::NAN }).collect() };
        let k = a3(&fk); let d = a3(&k); return (k, d);
    }
    let sm = |v: &[f64]| -> Vec<f64> {
        let mut o = vec![f64::NAN; v.len()];
        for i in 0..v.len() { if v[i].is_finite() { o[i] = if i > 0 && o[i - 1].is_finite() { (2. * o[i - 1] + v[i]) / 3. } else { v[i] }; } }
        o
    };
    let k = sm(&fk);
    let d = sm(&k);
    (k, d)
}

/// TSM Divergence (workshop slides 56-58 = TSM4 CD Ch09 'Peak Divergence System'), type 0 (prices).
fn divergence(x: &Ctx, from: usize, to: usize, diverge: u8, swing: f64, strength: f64, length: usize, exit: f64, fastx: f64, avg3: bool, slide_ext: bool) -> Vec<Trade> {
    // entry extreme on SlowK: code 'STslowk > fastx' (sell) / '< 100-fastx' (buy); slide 44 'SlowK must be greater than extreme (90%)'
    let (sell_k, buy_k) = if slide_ext { (100. - fastx, fastx) } else { (fastx, 100. - fastx) };
    let (h, l, c) = (&x.h, &x.l, &x.c);
    let (sk, sd) = slow_kd(x, length, avg3);
    let pc = swing / 100.;
    let mut b = Bk::new(x, from, to);
    let (mut last, mut curhigh, mut curlow, mut swhigh, mut swlow, mut swhigh1, mut swlow1) = (0i8, h[0], l[0], 0f64, 0f64, 0f64, 0f64);
    let (mut highbar, mut highbar1, mut lowbar, mut lowbar1, mut chighbar, mut clowbar) = (0usize, 0usize, 0usize, 0usize, 0usize, 0usize);
    let (mut exittype, mut signal) = (0i8, 0i8);
    let (mut sthigh, mut stlow, mut cursthigh, mut curstlow, mut highclose, mut lowclose) = (0f64, 0f64, 0f64, 0f64, 0f64, 0f64);
    let gt = |a: f64, b_: f64| a > b_; // NaN-safe comparisons: any NaN -> false
    for i in 0..=to {
        let (st, st1, sk_) = (sd[i], if i > 0 { sd[i - 1] } else { f64::NAN }, sk[i]);
        // search for a new high
        if last != 1 {
            if h[i] > curhigh { curhigh = h[i]; cursthigh = st; chighbar = i; }
            if l[i] < curhigh - curhigh * pc {
                last = 1;
                if exittype == -1 { exittype = 0; }
                swhigh1 = swhigh; highbar1 = highbar; swhigh = curhigh; highbar = chighbar; curlow = l[i];
                sthigh = cursthigh; highclose = swhigh; clowbar = i;
            }
        }
        // search for a new low
        if last != -1 {
            if l[i] < curlow { curlow = l[i]; curstlow = st; clowbar = i; }
            if h[i] > curlow + curlow * pc {
                last = -1;
                if exittype == 1 { exittype = 0; }
                swlow1 = swlow; lowbar1 = lowbar; swlow = curlow; lowbar = clowbar; curhigh = h[i];
                stlow = curstlow; lowclose = swlow; chighbar = i;
            }
        }
        let (chighprice, clowprice) = (curhigh, curlow);
        let dn_ok = gt(st1, 50. + exit) || gt(st1, st); // STtoday[1] > 50+exit or STtoday < STtoday[1]
        let up_ok = gt(50. - exit, st1) || gt(st, st1);
        if diverge == 1 {
            if last == -1 && exittype != -1 && c[i] > highclose && h[i] == chighprice && gt(sk_, sell_k) && dn_ok && gt(sthigh - strength, st) {
                b.at_close(i, -1, "bearish divergence"); signal = -1; exittype = 0;
            }
            if last == 1 && exittype != 1 && c[i] < lowclose && l[i] == clowprice && gt(buy_k, sk_) && up_ok && gt(st, stlow + strength) {
                b.at_close(i, 1, "bullish divergence"); signal = 1; exittype = 0;
            }
        } else {
            if last == -1 && h[i] == curhigh && gt(sk_, sell_k) && dn_ok
                && c[i] > swhigh && swhigh > swhigh1 && gt(sthigh - strength, st) && gt(sd[highbar1], sthigh) {
                b.at_close(i, -1, "double bearish divergence"); signal = -1; exittype = 0;
            }
            if last == 1 && l[i] == curlow && gt(buy_k, sk_) && up_ok
                && c[i] < swlow && swlow < swlow1 && gt(st, stlow + strength) && gt(stlow, sd[lowbar1]) {
                b.at_close(i, 1, "double bullish divergence"); // the printed code does not set signal here
            }
        }
        // divergence disappears
        if (gt(st, sthigh) || st == 100.) && signal == -1 {
            if b.pos() == -1 { b.at_close(i, 0, "stochastic above its swing-high value"); }
            signal = 0; exittype = -1;
        }
        if (gt(stlow, st) || st == 0.) && signal == 1 {
            if b.pos() == 1 { b.at_close(i, 0, "stochastic below its swing-low value"); }
            signal = 0; exittype = 1;
        }
        // stochastic reverses after crossing thresholds (AND binds tighter than OR)
        if !gt(sk_, fastx) && sk_.is_finite() || (gt(50. + exit, st1) && gt(st, st1) && signal == -1) {
            if b.pos() == -1 { b.at_close(i, 0, "stochastic reversed"); }
            exittype = -1; signal = 0;
        }
        if !gt(100. - fastx, sk_) && sk_.is_finite() || (gt(st1, 50. - exit) && gt(st1, st) && signal == 1) {
            if b.pos() == 1 { b.at_close(i, 0, "stochastic reversed"); }
            signal = 0; exittype = 1;
        }
        let _ = (lowbar, highbar);
    }
    b.done()
}

// ---------------------------------------------------------------- run

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if from > to || to >= x.r.len() { return None; }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(0., |d| d.def));
    let u = |k: &str| g(k).round().max(0.) as usize;
    Some(match id {
        "kfbpm_ma_ct" => {
            let s = tsm_signal(x, 1, u("period").max(1));
            let mut b = Bk::new(x, from, to);
            for i in from..=to { if s[i] != 0 { b.at_close(i, -s[i], "8-day MA changed direction"); } }
            b.done()
        }
        "kfbpm_2sys" | "kfbpm_2sys_trct" => {
            let s1 = tsm_signal(x, u("type1"), u("period1"));
            let mut s2 = tsm_signal(x, u("type2"), u("period2"));
            if id == "kfbpm_2sys_trct" { for v in s2.iter_mut() { *v = -*v; } }
            let mut b = Bk::new(x, from, to);
            for i in from..=to {
                let t = if s1[i] == s2[i] { s1[i] } else { 0 };
                b.at_close(i, t, if t == 0 { "systems disagree" } else { "both systems agree" });
            }
            b.done()
        }
        "kfbpm_divergence" => divergence(x, from, to, if u("diverge") == 2 { 2 } else { 1 }, g("swing"), g("strength"), u("length").max(1), g("exit"), g("fastx"), u("smooth") == 1, u("ext") == 1),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn ma_ct_takes_the_opposite_of_the_ma_turn() {
        let v: Vec<f64> = (0..40).map(|i| 100. + if i < 20 { i as f64 } else { 40. - i as f64 }).collect();
        let r: Vec<Row> = v.iter().enumerate().map(|(i, &c)| Row { ticker: "T".into(), date: format!("2000-01-{:02}", i % 28 + 1), open: c, high: c + 1., low: c - 1., close: c, volume: None }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let ts = run("kfbpm_ma_ct", &x, 1, 39, &Params::from([("period".into(), 3.)])).unwrap();
        assert_eq!(ts[0].dir, -1.); // rising MA -> short
        assert_eq!(ts[1].dir, 1.); // MA turns down -> long
        assert!(ts[1].legs[0].i > 20 && ts[1].legs[0].i <= 23);
    }
}
