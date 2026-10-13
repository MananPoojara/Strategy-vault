//! Murray A. Ruggiero, Jr. & Adrienne Laris Toghraie, "Traders' Secrets: Psychological & Technical Analyses"
//! (On Target Press, 1999). Book id `rgtsec`. Only Ruggiero's "Analytical Modeling Study" sections contain rules;
//! the interviews are discretionary. Printed pages (p.) cited. Sources and verification: library/rgtsec/.
//! TradeStation conventions as in rgcts: orders computed on the close act on the next bar; stops fill at the stop
//! or the open if gapped; "1 point" = 0.01; flat at the selected start.
use crate::books::rgcts::{sma, stop_engine, Orders};
use crate::connors::{true_range, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "rgtsec";

pub fn catalogue() -> Vec<StrategyDef> {
    let mk = |id, name, group, dir, status, rules: &'static [&'static str], notes| StrategyDef {
        id, name, group, session: 0, dir, status, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None,
    };
    let live = Status::Live;
    vec![
        mk("rgt_vol_cbo", "Volatility-adjusted channel breakout (N=5, VolX=.5)", "Ch. 4 · Sheldon Knight analysis", "Long/Short", live,
            &["E|\"Buy at Highest(High,N)+VolX*Average(TrueRange,3) + 1 point stop\" (p.118)", "E|\"Sell at Lowest(Low,N)-VolX*Average(TrueRange,3) - 1 point stop\" (p.118)",
              "X|Stop and reverse (no other exit given)", "F|N=5, VolX=.50: \"50% of the three-day average true range was a good value\" (p.118)"],
            "p.118: coffee 1/1/80-9/14/98, no costs: N=5,VolX=.5 $357,360, 298 trades, 42% win, avg 1149.01, PF 2.48. Index instead of coffee."),
        mk("rgt_cbo5", "Standard channel breakout N=5 (VolX=0)", "Ch. 4 · Sheldon Knight analysis", "Long/Short", live,
            &["E|Same rules with VolX=0: Buy at Highest(High,5)+1 point stop; Sell at Lowest(Low,5)-1 point stop (p.118)", "X|Stop and reverse"],
            "p.118: coffee N=5 VolX=0: $290,995, 513 trades, 43% win, avg 567.24, PF 1.74."),
        mk("rgt_taylor", "Taylor shell system (3-day rhythm)", "Ch. 6 · Bob Watson analysis", "Long/Short", live,
            &["F|\"If (High-Low)>0 then value1=(Close-Low)/(High-Low); If Low=Lowest(Low,Key) and value1<.2 then Lowbar=CurrentBar; Seq=Mod((CurrentBar-LowBar),Ct)\" (Key 10, Ct 3; p.181)",
              "E|\"If value1<BLev and Seq[Pt1]=0 then buy at low -NPts points limit; If value1<BLev and Seq[Pt2]=0 then buy at low -NPts points limit\" (BLev .40, Pt1 0, Pt2 1)",
              "E|\"If value1>SLev and Seq[Pt3]=0 then sell at High +NPts points Limit\" (SLev .80, Pt3 2); the sell reverses a long, a buy reverses a short",
              "X|Hold=-1: \"If BarsSinceEntry>1 then exitlong at close; ... exitshort at close\" -> exit at the close 2 bars after entry (the code wins over the comment 'exit at next day's close')",
              "F|Pts(3) points = 3 x 0.01 below the low / above the high (EasyLanguage points at PriceScale 100). BuyFilt/SellFilt are declared but unused in the printed code"],
            "p.181-182. A shell 'missing two patterns ... plus several filters'; no results printed. Before the first 10-day low with value1<.2, LowBar=0 (EasyLanguage default) and CurrentBar counts from the first data bar."),
        mk("rgt_cbo14_18", "Classic channel breakout 14-day entry / 18-day exit", "Ch. 7 · Ed Pomeranz analysis", "Long/Short", live,
            &["E|Buy Highest(High,EntLen) + 1 point stop; Sell Lowest(Low,EntLen) - 1 point stop (p.205) with EntLen 14 (p.206)",
              "X|Exitlong at Lowest(Low,ExtLen)-1 point stop; Exitshort at Highest(High,ExtLen)+1 point stop, ExtLen 18 (p.205-206)",
              "F|The Dollar-index filter (DXMom) of the printed code is not applied: this is the 'classic channel breakout' the book tests first"],
            "p.206: Yen 1/2/86-9/14/98, no costs: $116,570, 114 trades, 45% win, avg 1022.54, DD $16,668.75, PF 2.05."),
        mk("rgt_cbo_dollar", "Channel breakout with Dollar-index confirmation", "Ch. 7 · Ed Pomeranz analysis", "Long/Short",
            Status::NeedsData("needs the US Dollar index (Data2) and currency futures"),
            &["E|If DXMom<0 then Buy Highest(High,EntLen)+1 point stop; if DXMom>0 then Sell Lowest(Low,EntLen)-1 point stop (p.205); 20-day MA filter (p.206)"],
            "p.206: Yen with the filter $123,397, 85 trades."),
        mk("rgt_ma3", "Three moving average crossover 10/15/25 (stop and reverse)", "Ch. 9 · John Fritz analysis", "Long/Short", live,
            &["E|\"IF (Average(Close,Length1)[1] < Average(Close,Length2)[1] OR Average(Close,Length2)[1] < Average(Close,Length3)[1]) AND (Average(Close,Length1) > Average(Close,Length2) AND Average(Close,Length2) > Average(Close,Length3)) Then Buy on Close\" (p.269-270)",
              "E|Mirror rule: Sell on Close (reverses a long)", "F|Length1=10, Length2=15, Length3=25 (p.270)", "X|Stop and reverse only"],
            "p.270: 1/1/80-9/14/98, no costs: Yen $181,775, 147 trades, 49%; crude $49,086, 124 trades, 43%."),
        mk("rgt_cbo12", "Channel breakout N=12 (stop and reverse)", "Ch. 11 · Al Gerebizza analysis", "Long/Short", live,
            &["E|\"Buy at Highest (high,N) stop; Sell at Lowest (Low,N) stop;\" (p.319), N=12", "X|Stop and reverse"],
            "p.319: Yen 1/2/80-9/14/98, no costs: $169,638.75, 189 trades, 45% win, avg 897.56, DD $12,787.50, PF 2.26 (Swiss franc best N=24)."),
        mk("rgt_discretionary", "Interviewed traders' methods (CANSLIM, Gann, Market Profile, Keltner, Fibonacci, momentum retracement, Micro-M/W)", "Interviews", "Long/Short",
            Status::NotTestable("judgement: base patterns, divergences, Gann levels, Market Profile value areas, proprietary Keltner channels, unspecified indicator lengths and discretionary exits"),
            &["E|Herb Drechsler: MACD 17/9/5, 5-day SlowK, 9- and 3-day RSI all rising + discretion; exits 'an art form' (p.79-80)",
              "E|Bierovic Momentum Retracement and Micro-M/W: EMA lengths, MACD/RSI periods and profit objective not fixed (p.398-418)"],
            "Interview chapters 1-14; Monte Carlo drawdown (p.120) is an analysis tool, not a strategy."),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "rgt_vol_cbo" => ("rgt_breakout", "Channel breakouts", "N=5 + .5 x ATR3"),
        "rgt_cbo5" => ("rgt_breakout", "Channel breakouts", "N=5 standard"),
        "rgt_cbo14_18" => ("rgt_breakout", "Channel breakouts", "14 entry / 18 exit"),
        "rgt_cbo12" => ("rgt_breakout", "Channel breakouts", "N=12"),
        "rgt_cbo_dollar" => ("rgt_breakout", "Channel breakouts", "Dollar filter (data missing)"),
        "rgt_ma3" => ("rgt_ma", "Moving average crossover", "10/15/25"),
        "rgt_taylor" => ("rgt_taylor", "Taylor trading", "Shell system"),
        "rgt_discretionary" => ("rgt_other", "Interviews", "Not testable"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(match id {
        "rgt_vol_cbo" => vec![p("n", "Channel bars N", 5., 2., 40., 1.), p("volx", "VolX x 3-day ATR", 0.5, 0., 1.5, 0.05), p("pt", "One point (price units)", 0.01, 0., 1., 0.01)],
        "rgt_cbo5" => vec![p("n", "Channel bars N", 5., 2., 40., 1.), p("volx", "VolX x 3-day ATR", 0., 0., 1.5, 0.05), p("pt", "One point (price units)", 0.01, 0., 1., 0.01)],
        "rgt_cbo14_18" => vec![p("ent", "EntLen", 14., 5., 40., 1.), p("ext", "ExtLen", 18., 5., 40., 1.), p("pt", "One point (price units)", 0.01, 0., 1., 0.01)],
        "rgt_cbo12" => vec![p("n", "Channel bars N", 12., 5., 40., 1.)],
        "rgt_ma3" => vec![p("l1", "Length1", 10., 2., 50., 1.), p("l2", "Length2", 15., 3., 100., 1.), p("l3", "Length3", 25., 5., 200., 1.)],
        "rgt_taylor" => vec![p("key", "Key (low lookback)", 10., 3., 30., 1.), p("pts", "NPts (price units, 3 points)", 0.03, 0., 5., 0.01),
            p("pt1", "Pt1", 0., 0., 4., 1.), p("pt2", "Pt2", 1., 0., 4., 1.), p("pt3", "Pt3", 2., 0., 4., 1.), p("blev", "BLev", 0.4, 0.1, 0.9, 0.05),
            p("slev", "SLev", 0.8, 0.1, 0.95, 0.05), p("ct", "Ct (days in sequence)", 3., 2., 7., 1.), p("hold", "BarsSinceEntry > hold -> exit at close", 1., 0., 10., 1.)],
        _ => return None,
    })
}

fn hh(v: &[f64], i: usize, n: usize) -> f64 { v[i + 1 - n..=i].iter().copied().fold(f64::NEG_INFINITY, f64::max) }
fn ll(v: &[f64], i: usize, n: usize) -> f64 { v[i + 1 - n..=i].iter().copied().fold(f64::INFINITY, f64::min) }
fn trade(dir: f64, i: usize, px: f64, kind: &'static str, sig: usize) -> Trade {
    Trade { dir, legs: vec![Leg { i, px, w: 1. }], exit_i: i, exit_px: px, reason: String::new(), entry_kind: kind, signal_i: sig, open: false, exits: vec![] }
}
fn close(mut t: Trade, i: usize, px: f64, why: &str, out: &mut Vec<Trade>) {
    t.exit_i = i; t.exit_px = px; t.signal_i = i; t.reason = why.into(); out.push(t);
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if from > to || to >= x.r.len() {
        return None;
    }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let u = |k: &str| g(k).round().max(1.) as usize;
    let n = x.r.len();
    let mut out = vec![];
    match id {
        "rgt_vol_cbo" | "rgt_cbo5" => {
            let (len, vx, pt) = (u("n"), g("volx"), g("pt"));
            let atr3 = sma(&true_range(x.r), 3);
            stop_engine(x, from, to, len.max(4), &|i| Orders { buy: hh(&x.h, i - 1, len) + vx * atr3[i - 1] + pt, sell: ll(&x.l, i - 1, len) - vx * atr3[i - 1] - pt, xlong: f64::NAN, xshort: f64::NAN }, &mut out);
        }
        "rgt_cbo14_18" => {
            let (en, ex, pt) = (u("ent"), u("ext"), g("pt"));
            stop_engine(x, from, to, en.max(ex), &|i| Orders { buy: hh(&x.h, i - 1, en) + pt, sell: ll(&x.l, i - 1, en) - pt, xlong: ll(&x.l, i - 1, ex) - pt, xshort: hh(&x.h, i - 1, ex) + pt }, &mut out);
        }
        "rgt_cbo12" => {
            let len = u("n");
            stop_engine(x, from, to, len, &|i| Orders { buy: hh(&x.h, i - 1, len), sell: ll(&x.l, i - 1, len), xlong: f64::NAN, xshort: f64::NAN }, &mut out);
        }
        "rgt_ma3" => {
            let (a, b, c) = (sma(&x.c, u("l1")), sma(&x.c, u("l2")), sma(&x.c, u("l3")));
            let mut cur: Option<Trade> = None;
            for i in from.max(1)..=to {
                let up = (a[i - 1] < b[i - 1] || b[i - 1] < c[i - 1]) && (a[i] > b[i] && b[i] > c[i]);
                let dn = (a[i - 1] > b[i - 1] || b[i - 1] > c[i - 1]) && (a[i] < b[i] && b[i] < c[i]);
                let s = if up { 1. } else if dn { -1. } else { 0. };
                let pos = cur.as_ref().map_or(0., |t| t.dir);
                if s != 0. && s != pos && i < to {
                    if let Some(t) = cur.take() { close(t, i, x.c[i], "reverse on close", &mut out); }
                    cur = Some(trade(s, i, x.c[i], "close", i));
                }
            }
            if let Some(mut t) = cur { t.exit_i = to; t.exit_px = x.c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true; out.push(t); }
        }
        "rgt_taylor" => {
            let (key, pts, blev, slev, ct, hold) = (u("key"), g("pts"), g("blev"), g("slev"), u("ct"), g("hold").round().max(0.) as usize);
            let (p1, p2, p3) = (g("pt1").round() as usize, g("pt2").round() as usize, g("pt3").round() as usize);
            // value1 and Seq from the first bar (EasyLanguage CurrentBar counts from 1)
            let mut v1 = vec![0.; n];
            let mut seq = vec![0usize; n];
            let mut lowbar = 0usize;
            for i in 0..n {
                v1[i] = if x.h[i] - x.l[i] > 0. { (x.c[i] - x.l[i]) / (x.h[i] - x.l[i]) } else if i > 0 { v1[i - 1] } else { 0. };
                let cb = i + 1;
                if i + 1 >= key && x.l[i] == ll(&x.l, i, key) && v1[i] < 0.2 { lowbar = cb; }
                seq[i] = (cb - lowbar) % ct;
            }
            let lag = |i: usize, k: usize| i >= k && seq[i - k] == 0;
            let mut cur: Option<Trade> = None;
            for i in from.max(p1.max(p2).max(p3) + 1)..=to {
                let s = i - 1;
                let buy = v1[s] < blev && (lag(s, p1) || lag(s, p2));
                let sell = v1[s] > slev && lag(s, p3);
                let pos = cur.as_ref().map_or(0., |t| t.dir);
                let mut entered = false;
                if buy && pos <= 0. {
                    let lim = x.l[s] - pts;
                    if x.l[i] <= lim {
                        let px = x.o[i].min(lim);
                        if let Some(t) = cur.take() { close(t, i, px, "reverse: buy limit", &mut out); }
                        cur = Some(trade(1., i, px, "limit", s));
                        entered = true;
                    }
                } else if sell && pos >= 0. {
                    let lim = x.h[s] + pts;
                    if x.h[i] >= lim {
                        let px = x.o[i].max(lim);
                        if let Some(t) = cur.take() { close(t, i, px, "reverse: sell limit", &mut out); }
                        cur = Some(trade(-1., i, px, "limit", s));
                        entered = true;
                    }
                }
                if !entered {
                    if let Some(t) = cur.as_ref() {
                        if i - t.legs[0].i > hold { close(cur.take().unwrap(), i, x.c[i], "BarsSinceEntry>1 exit at close", &mut out); }
                    }
                }
            }
            if let Some(mut t) = cur { t.exit_i = to; t.exit_px = x.c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true; out.push(t); }
        }
        _ => return None,
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn rows(n: usize) -> Vec<Row> {
        (0..n).map(|i| Row { ticker: "T".into(), date: format!("2000-01-{:02}", 1 + i % 28), open: 100., high: 101., low: 99., close: 100., volume: None }).collect()
    }
    #[test]
    fn taylor_buys_below_the_low_and_exits_two_bars_later_at_the_close() {
        let mut r = rows(40);
        // bar 15: 10-day low, close in the lower 20% -> LowBar; Seq = 0 on bar 15
        r[15].low = 95.; r[15].close = 95.5; r[15].high = 100.;
        // bar 16 trades below 95 - .03 -> buy limit filled at min(open, 94.97)
        r[16].low = 94.; r[16].open = 96.;
        r[18].close = 97.;
        let x = Ctx::new(&r, &r, &[]);
        let ts = run("rgt_taylor", &x, 12, 39, &Params::new()).unwrap();
        let t = &ts[0];
        assert_eq!(t.dir, 1.); assert_eq!(t.legs[0].i, 16); assert!((t.legs[0].px - 94.97).abs() < 1e-9);
        assert_eq!(t.exit_i, 18); assert_eq!(t.exit_px, 97.);
    }
    #[test]
    fn ma3_reverses_on_alignment() {
        let mut r = rows(120);
        for i in 0..120 { let c = if i < 60 { 100. + i as f64 } else { 160. - 2. * (i as f64 - 60.) }; r[i].close = c; r[i].open = c; r[i].high = c + 1.; r[i].low = c - 1.; }
        let x = Ctx::new(&r, &r, &[]);
        let ts = run("rgt_ma3", &x, 30, 119, &Params::new()).unwrap();
        assert!(ts.iter().any(|t| t.dir < 0.));
    }
}
