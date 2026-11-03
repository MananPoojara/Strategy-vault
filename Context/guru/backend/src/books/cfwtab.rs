//! Charles F. Wright, "Trading as a Business" (1998; book id `cfwtab`; printed pages = PDF page + 7).
//! Reading: claude-continuation-20261020/authors/cfwright (agents/cfwtab) and authors/lanec/admission3/lanec3_ref.py. Every admitted rule is the
//! book's printed TradeStation EasyLanguage strategy (checked against the page images): 9/18 average cross (p.46), stochastic
//! cross (p.49), gap-open volatility breakout (p.53), channel breakout on close (p.57-59), "S&E 4 modified" average direction +
//! %R + 50-bar failsafe (p.78-89), the up-key-reversal breakout (p.113-118) and the weekly HHLL breakout of Ch.7 (p.135-143;
//! lane K9, 1 Nov 2026, to-code row 159; independent check authors/titles-1030/K9w/k9_ref.py).
//! Held: RangeLeader breakout (p.60-62; $500 money stop and $1,500 target are S&P futures contract dollars), its 30-minute
//! versions (intraday), the UKR $5,000 stop/target versions (contract dollars).
//! Book: Disney, Caterpillar, IBM, Coca-Cola stocks and S&P 500 futures; other instruments are a declared application.
use crate::books::lc3::{bars, high_first, hi, lo, sma, Pos};
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const REVIEWED: &[&str] = &["cfwtab_ma918", "cfwtab_stoch", "cfwtab_gap", "cfwtab_chan", "cfwtab_se4", "cfwtab_ukr", "cfwtab_hhll"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const TS: &str = "[Declared readings] TradeStation conventions: 'crosses over' = above today and at or below yesterday; 'on Close' = filled at the signal bar's close; 'next bar at market' = the next bar's open; a stop order lives for the next bar only and fills at its price, or at the open when the open is already beyond it; when both a buy and a sell stop are hit in one bar, the extreme nearer the open is taken as reached first (TradeStation's bar path). '1 point' = one tick of the instrument (0.01 US, 0.05 NSE). One position; an opposite entry reverses; a same-direction entry while in a position is ignored. No costs or slippage.";

fn d(id: &'static str, name: &'static str, group: &'static str, dir: &'static str, rules: &'static [&'static str], notes: &'static str) -> StrategyDef {
    StrategyDef { id, name, group, session: 0, dir, status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "cfwtab", rules, notes, claim: None }
}

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        d("cfwtab_ma918", "9/18-bar moving average crossover, always in", "Ch.3 Markets, Strategies & Time Frames (p.46-48)", "Long/Short",
          &["E|\"IF CurrentBar > 1 and Average(Close,Length1) crosses over Average(Close,Length2) Then Buy on Close;\" Input: Length1(9),Length2(18) (Chart 6, p.46)",
            "X|\"IF CurrentBar > 1 and Average(Close,Length1) crosses below Average(Close,Length2) Then Sell on Close;\" (p.46) - reversal only (SPF 1: 'Stops None')"],
          "Book: Disney daily 1/2/90-7/11/97, 39% profitable (p.46). Simple averages of the close. " ),
        d("cfwtab_stoch", "Stochastic crossover: SlowK crosses SlowD below 35 (buy) / above 65 (sell)", "Ch.3 Markets, Strategies & Time Frames (p.49-50)", "Long/Short",
          &["E|\"IF CurrentBar > 1 and SlowD(Length) < 35 and SlowK(Length) crosses above SlowD(Length) then Buy on Close;\" Input: Length(10) (Chart 7, p.49)",
            "X|\"IF CurrentBar > 1 and Slowd(Length) > 65 and SlowK(Length) crosses below SlowD(Length) then Sell on Close;\" (p.49) - reversal only"],
          "Book: Caterpillar daily 1/2/91-4/11/97, 68% profitable (PS 2, p.50). [Declared reading, not printed] SlowK = 3-bar simple average of the raw 10-bar %K ((close - lowest low) / (highest high - lowest low) x 100, 0 when the range is 0, as the book's own PercentR function p.81); SlowD = 3-bar simple average of SlowK."),
        d("cfwtab_gap", "Gap open volatility breakout: stop at yesterday's close +/- yesterday's range, out next open", "Ch.3 Markets, Strategies & Time Frames (p.52-53)", "Long/Short",
          &["E|\"IF Open of next bar > High of this bar or Open of next bar < Low of this bar then Buy next bar at Close of this bar + Range of this bar Stop;\" (Chart 9, p.53)",
            "E|\"IF Open of next bar < Low of this bar or Open of next bar > High of this bar then Sell next bar at Close of this bar - Range of this bar Stop;\" (p.53)",
            "X|\"Exitlong next bar at market; Exitshort next bar at market;\" (p.53) = out at the open of the day after entry"],
          "Book: S&P 500 futures daily 1/1/90-4/2/97, average trade $191.13 without costs (p.53). Either gap direction arms both stops (as printed). Range = high - low."),
        d("cfwtab_chan", "Channel breakout on close (50-day highest high / lowest low), always in", "Ch.3 Markets, Strategies & Time Frames (p.57-59)", "Long/Short",
          &["E|\"IF CurrentBar > 1 and Close > Highest(High,Length)[1] Then Buy on Close;\" (PS 4, p.58)",
            "X|\"IF CurrentBar > 1 and Close < Lowest(Low,Length)[1] Then Sell on Close;\" (p.58); SPF 4: 'Set-Up 50-Day Highest High and Lowest Low Channel', 'Exits Reversal'"],
          "Book: IBM daily 1/5/70-7/24/97 (73 trades, 44% profitable, PS 4) and weekly (SPF 5, 'outperformed'). The SPF says 50 days; the code box under PS 4 shows Length(10), the weekly chart's input (Chart 12). Default = the SPF's 50 days (the text); p.len=10 with p.weekly=1 is the weekly test. Weekly bars = Monday-based calendar weeks; weekly signals fill at the week's last close."),
        d("cfwtab_se4", "S&E 4 modified: 30-bar average direction + %R(10) 20/80 cross, plus 50-bar high/low failsafe stops (weekly)", "Ch.4 Profile of a Winning Strategy (p.78-89)", "Long/Short",
          &["E|\"IF CurrentBar > 1 and Average(Close,AvgLen) > Average(Close,AvgLen)[1] and PercentR(PrctRLen) crosses above 20 Then Buy(\"%R Buy\") at market;\" AvgLen(30), PrctRLen(10) (SPF 3, p.86)",
            "E|\"IF CurrentBar > 1 and Average(Close,AvgLen) < Average(Close,AvgLen)[1] and PercentR(PrctRLen) crosses below 80 Then Sell(\"%R Sell\") at market;\" (p.86)",
            "E|\"Sell(\"LL\") at Lowest(low,50) - 1 point stop; Buy(\"HH\") at Highest(high,50) + 1 point stop;\" (p.86)",
            "F|PercentR = 100 - ((Highest(High,Length) - Close) / (Highest(High,Length) - Lowest(Low,Length))) x 100, 0 when the range is 0 (Chart 2, p.81)",
            "X|Reversal only (SPF 3: 'Stops None, Exits None')"],
          "Book: Coca-Cola weekly 1/9/70-8/8/97 (Table 1 p.89: 25 trades, 44% profitable). Default weekly bars (Monday-based calendar weeks); p.weekly=0 runs the same code on daily bars. 'At market' after a bar's close = the next bar's first open. The 50-bar stops are computed from the 50 completed bars before the current one and are checked day by day inside the week (buy stop first if one day touches both). 'IncludeStrategy: Exit on 8/1/97' only ends the book's test and is not applied."),
        d("cfwtab_ukr", "Up-key-reversal breakout: new 10-day low with an up close, buy 1 tick over its high next day, out on the 8th close", "Ch.6 The Art of Strategy Design - In Practice (p.113-118)", "Long",
          &["E|\"Condition1 = Low < Lowest(Low,LL)[1]; Condition2 = Close > Close[1]; If Currentbar > 1 and Condition1 and Condition2 then Buy at High + 1 point stop;\" Inputs: BSI(5),LL(10) (UKR Breakouts 2, SPF 2, p.117); optimised 'low of today being lower than the lowest low of the last 10 days; coupled with holding the trade for 8 days' (PS 5, p.118)",
            "X|\"IF BarsSinceEntry(0) = BSI then Exitlong on Close;\" (p.117) with BSI = 8 (PS 5): out on the close 8 bars after the entry bar",
            "F|\"the breakout must occur on the day following the UKR\" (p.114); 'If TradeStation is long on an UKR breakout, it will ignore any subsequent signals until it is flat' (p.120)"],
          "Book: S&P 500 futures daily 4/21/82-4/2/97 (PS 5: the 10-low / 8-day version, no stop, no target). The later $5,000 money-stop and $5,000 target versions (PS 7-10) are S&P contract dollars and are held; the 1987 crash-date exclusion belongs to those versions. p.bsi / p.ll change the hold and the low look-back (book's first test: 5 days)."),
        d("cfwtab_hhll", "HHLL breakouts (weekly): buy stop over the 20-week high, reverse short under the 50-week low, always in", "Ch.7 Optimization, The Double-Edged Sword (p.135-143)", "Long/Short",
          &["E|\"Buy(\"hh\") at Highest(High,HH) + 1 point Stop;\" (PS 1, p.137); SPF 1: 'Stops None, Exits None'",
            "X|\"Sell(\"ll\") at Lowest(Low,LL) - 1 point Stop;\" (p.137) - stop and reverse",
            "F|\"So, now let's put together the whole picture for the 20/50\"; \"Would you rather trade the 20/50 than the 50/50. I would.\" (p.140-143)"],
          "Book: DJIA weekly 1/9/70-8/8/97: the 50/50 captured 4,642 Dow points (PS 1, p.138); 20/50 = 17 trades, 53% profitable, net 6,146.20 points, profit factor 7.20 (Opt Tables 1 and 5, p.139-141). [Declared readings] Default HH = 20 and LL = 50 weeks, the author's chosen lengths (SPF 1's 50/50 and other lengths are options). The channel is the highest high (lowest low) of the HH (LL) completed weekly bars before the current week, i.e. up to and including the signal week; both stops work every day of the current week, filled at the stop or at the open on a gap; when one day touches both, the buy stop is taken first (as the S&E 4 card). Weekly bars = Monday-based calendar weeks. 'IncludeStrategy: Exit on 8/8/97' only ends the book's test and is not applied. Options from the text (p.142): long only ('Get long on the 20-week upside breakout and get out of the market on the 50-week downside breakout') and close-confirmed entries ('insist that the week's close be above the 20-week high or below the 50-week low', filled at that weekly close). The author also ran the 30 Dow stocks and other weeklies; other indices, ETFs and stocks are a declared application."),
        StrategyDef { status: Status::NotTestable("The $500 money stop and $1,500 profit target (SPF 6) are S&P 500 futures contract dollars (1993); the book gives no amount for other instruments. The 30-minute versions need intraday data."),
            ..d("cfwtab_rangeleader", "RangeLeader breakout (held: fixed futures-dollar stop and target)", "Ch.3 Markets, Strategies & Time Frames (p.60-62)", "Long/Short",
          &["E|\"Value1 = (High + Low) / 2 ; Cond1 = Value1 > High[1] OR Value1 < Low[1] ; Cond2 = Range[0] > Range[1] ;\" (Chart 14, p.61); \"If RangeLeader = 1 then begin Buy at High + 1 point stop; Sell at Low - 1 point stop; end;\" (SPF 6, p.61)",
            "X|\"Exitlong next bar on Open; Exitshort next bar on Open;\" (p.61); SPF 6: '$500 MMS', '$1,500 PT' (S&P futures 1993)"], "Held.") },
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let f = match id {
        "cfwtab_ma918" => "9/18 MA cross", "cfwtab_stoch" => "Stochastic cross", "cfwtab_gap" => "Gap open breakout", "cfwtab_chan" => "Channel breakout",
        "cfwtab_se4" => "MA direction + %R", "cfwtab_ukr" => "Up key reversal", "cfwtab_hhll" => "HHLL weekly breakout", "cfwtab_rangeleader" => "RangeLeader",
        _ => return None,
    };
    let id = REVIEWED.iter().chain(["cfwtab_rangeleader"].iter()).copied().find(|r| *r == id)?;
    Some((id, "Wright Trading as a Business", f))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    match id {
        "cfwtab_chan" => Some(vec![p("len", "Channel bars (SPF 4: 50; weekly test: 10)", 50., 2., 200., 1.), p("weekly", "Weekly bars (1) - the book's second test", 0., 0., 1., 1.)]),
        "cfwtab_se4" => Some(vec![p("weekly", "Weekly bars (1 = book) / daily (0)", 1., 0., 1., 1.)]),
        "cfwtab_ukr" => Some(vec![p("bsi", "Bars held after entry (book optimum 8)", 8., 1., 30., 1.), p("ll", "Low look-back (book 10)", 10., 1., 50., 1.)]),
        "cfwtab_hhll" => Some(vec![p("hh", "Buy-stop channel weeks HH (author's choice 20; SPF 1: 50)", 20., 2., 100., 1.), p("ll", "Sell-stop channel weeks LL (book 50)", 50., 2., 100., 1.),
            p("long", "Long only (1): the 50-week low only exits (p.142)", 0., 0., 1., 1.), p("close", "Close-confirmed entries (1): the week's close beyond the channel, filled at that close (p.142)", 0., 0., 1., 1.)]),
        _ if reviewed(id) || id == "cfwtab_rangeleader" => Some(vec![]),
        _ => None,
    }
}

fn cross_up(a: &[f64], b: &[f64], i: usize) -> bool { i > 0 && a[i] > b[i] && a[i - 1] <= b[i - 1] }
fn cross_dn(a: &[f64], b: &[f64], i: usize) -> bool { i > 0 && a[i] < b[i] && a[i - 1] >= b[i - 1] }

/// Raw %K / the book's PercentR: (close - lowest low) / (highest high - lowest low) x 100 over n bars, 0 when the range is 0.
pub fn pct_r(h: &[f64], l: &[f64], c: &[f64], n: usize) -> Vec<f64> {
    (0..c.len()).map(|i| if i + 1 < n { f64::NAN } else { let (hh, ll) = (hi(h, i + 1 - n, i), lo(l, i + 1 - n, i)); if hh - ll != 0. { 100. - (hh - c[i]) / (hh - ll) * 100. } else { 0. } }).collect()
}

fn close_sar(x: &Ctx, from: usize, to: usize, sig: &dyn Fn(usize) -> f64) -> Vec<Trade> {
    let mut p = Pos::new();
    for i in from.max(1)..=to { let s = sig(i); if s != 0. { p.enter(s, i, x.c[i], "close"); } }
    p.done(to, x.c[to])
}

fn gap(x: &Ctx, from: usize, to: usize) -> Vec<Trade> {
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let mut p = Pos::new();
    for k in from.max(1)..=to {
        if p.side() != 0. { p.exit(k, o[k], "next open"); }
        if !(o[k] > h[k - 1] || o[k] < l[k - 1]) { continue; }
        let (bs, ss) = (c[k - 1] + (h[k - 1] - l[k - 1]), c[k - 1] - (h[k - 1] - l[k - 1]));
        if o[k] >= bs { p.enter(1., k, o[k], "stop"); } else if o[k] <= ss { p.enter(-1., k, o[k], "stop"); }
        let legs: [(f64, f64); 2] = if high_first(o[k], h[k], l[k]) { [(1., bs), (-1., ss)] } else { [(-1., ss), (1., bs)] };
        for (dd, lv) in legs {
            if dd > 0. && h[k] >= bs && o[k] < bs { p.enter(1., k, lv, "stop"); }
            if dd < 0. && l[k] <= ss && o[k] > ss { p.enter(-1., k, lv, "stop"); }
        }
    }
    p.done(to, c[to])
}

fn chan(x: &Ctx, from: usize, to: usize, n: usize, weekly: bool) -> Vec<Trade> {
    let b = bars(x, to, weekly);
    let (bh, bl): (Vec<f64>, Vec<f64>) = (b.iter().map(|z| z.3).collect(), b.iter().map(|z| z.4).collect());
    let mut p = Pos::new();
    for w in n..b.len() {
        let e = b[w].1;
        if e < from { continue; }
        let cl = b[w].5;
        if cl > hi(&bh, w - n, w - 1) { p.enter(1., e, cl, "close"); } else if cl < lo(&bl, w - n, w - 1) { p.enter(-1., e, cl, "close"); }
    }
    p.done(to, x.c[to])
}

fn se4(x: &Ctx, from: usize, to: usize, weekly: bool) -> Vec<Trade> {
    let b = bars(x, to, weekly);
    let bc: Vec<f64> = b.iter().map(|z| z.5).collect();
    let (bh, bl): (Vec<f64>, Vec<f64>) = (b.iter().map(|z| z.3).collect(), b.iter().map(|z| z.4).collect());
    let ma = sma(&bc, 30);
    let pr = pct_r(&bh, &bl, &bc, 10);
    let t = x.market_tick();
    let mut p = Pos::new();
    let mut pend = 0.;
    for w in 1..b.len() {
        let (s, e) = (b[w].0, b[w].1);
        if pend != 0. && s >= from { p.enter(pend, s, x.o[s], "open"); }
        pend = 0.;
        if w >= 50 {
            let (bs, ss) = (hi(&bh, w - 50, w - 1) + t, lo(&bl, w - 50, w - 1) - t);
            for dd in s.max(from)..=e {
                if p.side() != 1. && x.h[dd] >= bs { p.enter(1., dd, x.o[dd].max(bs), "stop"); }
                if p.side() != -1. && x.l[dd] <= ss { p.enter(-1., dd, x.o[dd].min(ss), "stop"); }
            }
        }
        if w < 30 || ma[w - 1].is_nan() || pr[w - 1].is_nan() { continue; }
        if ma[w] > ma[w - 1] && pr[w] > 20. && pr[w - 1] <= 20. { pend = 1.; }
        if ma[w] < ma[w - 1] && pr[w] < 80. && pr[w - 1] >= 80. { pend = -1.; }
    }
    p.done(to, x.c[to])
}

fn hhll(x: &Ctx, from: usize, to: usize, hh: usize, ll: usize, long_only: bool, on_close: bool) -> Vec<Trade> {
    let b = bars(x, to, true);
    let (bh, bl): (Vec<f64>, Vec<f64>) = (b.iter().map(|z| z.3).collect(), b.iter().map(|z| z.4).collect());
    let t = x.market_tick();
    let mut p = Pos::new();
    for w in 1..b.len() {
        let (s, e) = (b[w].0, b[w].1);
        if on_close {
            if e < from { continue; }
            let cl = b[w].5;
            if w >= hh && cl > hi(&bh, w - hh, w - 1) { p.enter(1., e, cl, "close"); }
            else if w >= ll && cl < lo(&bl, w - ll, w - 1) {
                if !long_only { p.enter(-1., e, cl, "close"); } else if p.side() > 0. { p.exit(e, cl, "close under the LL-week low"); }
            }
            continue;
        }
        for dd in s.max(from)..=e {
            if w >= hh && p.side() != 1. {
                let bs = hi(&bh, w - hh, w - 1) + t;
                if x.h[dd] >= bs { p.enter(1., dd, x.o[dd].max(bs), "stop"); }
            }
            if w >= ll && p.side() != -1. {
                let ss = lo(&bl, w - ll, w - 1) - t;
                if x.l[dd] <= ss {
                    if !long_only { p.enter(-1., dd, x.o[dd].min(ss), "stop"); } else if p.side() > 0. { p.exit(dd, x.o[dd].min(ss), "LL-week low stop"); }
                }
            }
        }
    }
    p.done(to, x.c[to])
}

fn ukr(x: &Ctx, from: usize, to: usize, bsi: usize, ll: usize) -> Vec<Trade> {
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let t = x.market_tick();
    let mut p = Pos::new();
    for k in from.max(ll + 1)..=to {
        let i = k - 1;
        if p.side() == 0. && l[i] < lo(l, i - ll, i - 1) && c[i] > c[i - 1] {
            let lv = h[i] + t;
            if o[k] >= lv { p.enter(1., k, o[k], "stop"); } else if h[k] >= lv { p.enter(1., k, lv, "stop"); }
        }
        if let Some(e) = p.entry_bar() { if k - e == bsi { p.exit(k, c[k], "8th close after entry"); } }
    }
    p.done(to, c[to])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let c = &x.c;
    Some(match id {
        "cfwtab_ma918" => { let (f, s) = (sma(c, 9), sma(c, 18)); close_sar(x, from, to, &|i| if cross_up(&f, &s, i) { 1. } else if cross_dn(&f, &s, i) { -1. } else { 0. }) }
        "cfwtab_stoch" => {
            let k = sma(&pct_r(&x.h, &x.l, c, 10), 3); let dd = sma(&k, 3);
            close_sar(x, from, to, &|i| if dd[i] < 35. && cross_up(&k, &dd, i) { 1. } else if dd[i] > 65. && cross_dn(&k, &dd, i) { -1. } else { 0. })
        }
        "cfwtab_gap" => gap(x, from, to),
        "cfwtab_chan" => chan(x, from, to, g("len").round().max(1.) as usize, g("weekly") > 0.5),
        "cfwtab_se4" => se4(x, from, to, g("weekly") > 0.5),
        "cfwtab_hhll" => hhll(x, from, to, g("hh").round().max(1.) as usize, g("ll").round().max(1.) as usize, g("long") > 0.5, g("close") > 0.5),
        "cfwtab_ukr" => ukr(x, from, to, g("bsi").round().max(1.) as usize, g("ll").round().max(1.) as usize),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn hhll_reverses_on_the_weekly_channel_stops() {
        // 1,500 days on 28-day months (monotonic dates; weeks come from the calendar) with a slow cycle
        let r: Vec<Row> = (0..1500).map(|k| { let f = k as f64; let c = 100. + 30. * (f / 160.).sin() + 0.01 * f;
            Row { ticker: "T".into(), date: format!("{:04}-{:02}-{:02}", 2001 + k / 336, 1 + k % 336 / 28, 1 + k % 28), open: c, high: c + 1., low: c - 1., close: c, volume: None } }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let t = run("cfwtab_hhll", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.iter().any(|z| z.dir > 0.) && t.iter().any(|z| z.dir < 0.));
        for w in t.windows(2) { assert_eq!(w[0].exit_i, w[1].legs[0].i); assert!(w[0].dir != w[1].dir); }
        let lo_ = run("cfwtab_hhll", &x, 0, r.len() - 1, &Params::from([("long".to_string(), 1.)])).unwrap();
        assert!(lo_.iter().all(|z| z.dir > 0.));
    }
}
