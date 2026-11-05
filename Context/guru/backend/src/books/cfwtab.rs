//! Charles F. Wright, "Trading as a Business" (1998; book id `cfwtab`; printed pages = PDF page + 7).
//! Reading: claude-continuation-20261020/authors/cfwright (agents/cfwtab) and authors/lanec/admission3/lanec3_ref.py. Every admitted rule is the
//! book's printed TradeStation EasyLanguage strategy (checked against the page images): 9/18 average cross (p.46), stochastic
//! cross (p.49), gap-open volatility breakout (p.53), channel breakout on close (p.57-59), "S&E 4 modified" average direction +
//! %R + 50-bar failsafe (p.78-89), the up-key-reversal breakout (p.113-118) and the weekly HHLL breakout of Ch.7 (p.135-143;
//! lane K9, 1 Nov 2026, to-code row 159; independent check authors/titles-1030/K9w/k9_ref.py).
//! RangeLeader breakout, daily SPF 6 (p.60-62): coded by lane K54 (decision #229) for SPX (the $500 stop and $1,500 target are
//! 1.00 / 3.00 points of the named 1993 S&P contract), entry day on SPX 1-minute bars; HELD: 30% of its trades depend on the
//! order inside one minute (decision #175). Held too: its 30-minute versions, the UKR $5,000 stop/target versions (contract dollars).
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
          "Book: Caterpillar daily 1/2/91-4/11/97, 68% profitable (PS 2, p.50). [Declared reading, not printed] The code calls TradeStation's built-in SlowK/SlowD without printing them; for TradeStation-era code the vault uses the legacy TradeStation smoothing (lead decision 191): raw 10-bar %K = (close - lowest low) / (highest high - lowest low) x 100 (0 when the range is 0), SlowK = (2 x previous SlowK + %K) / 3, SlowD = (2 x previous SlowD + SlowK) / 3, each seeded with its first value. Option p.smooth = 1: 3-bar simple averages (SlowK = average of %K, SlowD = average of SlowK)."),
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
        StrategyDef { status: Status::NotTestable("coded on real SPX 1-minute bars (lane K54, decision #229), but 235 of 781 SPX trades (30.1%, 2014-01-02..2026-11-03; 221 of 736, 30.0%, on 2014-2025; 307 of 1,159, 26.5%, from 2008) are touched by a minute whose fills differ between the two bar paths (open-high-low-close vs open-low-high-close), the count used for depending on the order inside one minute: the 1.00-point money stop and the 3.00-point target (SPF 6's $500 and $1,500 at the 1993 S&P contract's $500 a point, p.61-62) are often reached in the same minute as the entry stop. Decision #175 holds a rule above 5%; the #177b check on SPY minutes with the points scaled passed (2014-2025 average trade 0.040% SPX vs 0.007% SPY), so only the ambiguity holds it"),
          ..d("cfwtab_rangeleader", "RangeLeader breakout (daily, SPF 6): stop entries 1 tick beyond a range-leader bar, $500 stop / $1,500 target = 1.00 / 3.00 S&P points, out next open", "Ch.3 Markets, Strategies & Time Frames (p.60-62)", "Long/Short",
          &["F|\"Value1 = (High + Low) / 2 ; Cond1 = Value1 > High[1] OR Value1 < Low[1] ; Cond2 = Range[0] > Range[1] ; If Cond1 and Cond2 then RangeLeader = 1\" (Chart 14, p.61)",
            "E|\"If RangeLeader = 1 then begin Buy at High + 1 point stop; Sell at Low - 1 point stop; end;\" (SPF 6 'Daily RL Breakouts', p.61); 1 point = one tick of the traded instrument (decision #57)",
            "X|SPF 6: 'Stops $500 MMS', 'Exits Next day on open, $1,500 PT'; \"when it gets filled, it immediately puts a stop loss $500 away from the entry price ... if the price moved $1,500 in my favor, I would take the profit\" (p.62); \"Exitlong next bar on Open; Exitshort next bar on Open;\" (p.61)",
            "F|$500 and $1,500 at the named contract's $500 per S&P point (S&P futures daily, Omega CD, 1/1/93-12/31/93: SPF 6) = 1.00 and 3.00 S&P 500 index points (decision #196(2)); SPX only"],
          "Book: S&P 500 futures daily 1993, $35 slippage and $25 commission per trade (SPF 6; PS 6 is a report image). [Declared readings] Coded for the S&P 500 index (SPX) only, because the stop and target are the named S&P contract's points (decision #196(2), #229); results would start in 2014, where the index's daily opens are real. The stop and target act on the entry day, so the entry day is run on SPX's real 1-minute bars (2008-2026): both entry stops live all day, each fills at most once; after a fill the 1.00-point stop, the 3.00-point target and the other side's entry stop (a reversal) are live; a level the minute opens through fills at that open. Inside one minute the TradeStation bar path is used (open, the nearer extreme, the farther extreme, close); a minute whose fills would differ with the other extreme first is counted as ambiguous and the trades it touches say so (a proxy count: the other path's later consequences are not followed). A position still open at the entry day's close exits at the next day's open (daily bar). A day without minute bars makes no trade. Simple ranges (high - low). No costs (the book's $60 is not deducted). Lane K54.") },
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
        "cfwtab_stoch" => Some(vec![p("smooth", "0 = legacy TradeStation SlowK/SlowD (default, decision 191), 1 = 3-bar simple averages", 0., 0., 1., 1.)]),
        "cfwtab_hhll" => Some(vec![p("hh", "Buy-stop channel weeks HH (author's choice 20; SPF 1: 50)", 20., 2., 100., 1.), p("ll", "Sell-stop channel weeks LL (book 50)", 50., 2., 100., 1.),
            p("long", "Long only (1): the 50-week low only exits (p.142)", 0., 0., 1., 1.), p("close", "Close-confirmed entries (1): the week's close beyond the channel, filled at that close (p.142)", 0., 0., 1., 1.)]),
        "cfwtab_rangeleader" => Some(vec![p("stop", "Money stop in S&P points ($500 at $500 per point, SPF 6)", 1., 0.25, 10., 0.25), p("target", "Profit target in S&P points ($1,500 at $500 per point)", 3., 0.5, 30., 0.5)]),
        _ if reviewed(id) => Some(vec![]),
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

// ---- RangeLeader daily SPF 6 on the entry day's 1-minute bars (lane K54, decision #229) ----
#[derive(Clone, Copy, PartialEq, Debug)]
enum Lk { Long, Short, Stop, Target }
/// One minute: `state` = (position (dir, entry), long stop used, short stop used); returns the fills (kind, price) in order.
/// `ohlc` = the path open-high-low-close (else open-low-high-close).
fn rl_minute(z: &crate::intraday::Bar1, st: &mut (Option<(f64, f64)>, bool, bool), lv: (f64, f64, f64, f64), ohlc: bool) -> Vec<(Lk, f64)> {
    let (bs, ss, stop, tgt) = lv;
    let mut fills = vec![];
    // live levels: (kind, up, price); exits before the reversal entry when both are satisfied at once
    let live = |st: &(Option<(f64, f64)>, bool, bool)| -> Vec<(Lk, bool, f64)> {
        let mut v = vec![];
        if let Some((d, e)) = st.0 { v.push((Lk::Stop, d < 0., e - d * stop)); v.push((Lk::Target, d > 0., e + d * tgt)); }
        if !st.1 && st.0.map_or(true, |p| p.0 < 0.) { v.push((Lk::Long, true, bs)); }
        if !st.2 && st.0.map_or(true, |p| p.0 > 0.) { v.push((Lk::Short, false, ss)); }
        v
    };
    let fire = |st: &mut (Option<(f64, f64)>, bool, bool), k: Lk, px: f64, fills: &mut Vec<(Lk, f64)>| {
        fills.push((k, px));
        match k { Lk::Stop | Lk::Target => st.0 = None, Lk::Long => { st.1 = true; st.0 = Some((1., px)); } Lk::Short => { st.2 = true; st.0 = Some((-1., px)); } }
    };
    // levels the open already satisfies fill at the open
    loop {
        let k = live(st).into_iter().find(|&(_, up, px)| if up { z.o >= px } else { z.o <= px }).map(|v| v.0);
        match k { Some(k) => fire(st, k, z.o, &mut fills), None => break }
    }
    let mut cur = z.o;
    for t in if ohlc { [z.h, z.l, z.c] } else { [z.l, z.h, z.c] } {
        loop {
            let up = t > cur;
            // inclusive at `cur`: an order that became live at the price just reached fills there too (each fill removes its level)
            let hit = live(st).into_iter().filter(|&(_, u, px)| u == up && if up { px >= cur && px <= t } else { px <= cur && px >= t })
                .min_by(|a, b| (a.2 - cur).abs().total_cmp(&(b.2 - cur).abs()));
            match hit { Some((k, _, px)) => { fire(st, k, px, &mut fills); cur = px; } None => { cur = t; break; } }
        }
    }
    fills
}

fn rangeleader(x: &Ctx, from: usize, to: usize, stop: f64, tgt: f64) -> Vec<Trade> {
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let t = x.market_tick();
    let rl = |i: usize| i >= 1 && { let m = (h[i] + l[i]) / 2.; (m > h[i - 1] || m < l[i - 1]) && h[i] - l[i] > h[i - 1] - l[i - 1] };
    let mk = |d: f64, i: usize, px: f64, xi: usize, xp: f64, why: String, open: bool| Trade { dir: d, legs: vec![crate::connors::Leg { i, px, w: 1. }], exit_i: xi, exit_px: xp, reason: why, entry_kind: "stop", signal_i: i - 1, open, exits: vec![] };
    let mut out = vec![];
    let mut carry: Option<(f64, usize, f64, bool)> = None; // (dir, entry bar, entry px, ambiguous)
    for j in from.max(2)..=to {
        if let Some((d, ei, ep, amb)) = carry.take() { out.push(mk(d, ei, ep, j, o[j], format!("next bar on open{}", if amb { AMB } else { "" }), false)); }
        if !rl(j - 1) { continue; }
        let Some(bars) = crate::intraday_w2b::day(x, j) else { continue };
        let lv = (h[j - 1] + t, l[j - 1] - t, stop, tgt);
        let mut st: (Option<(f64, f64)>, bool, bool) = (None, false, false);
        let mut pos: Option<(f64, f64, bool)> = None; // (dir, entry px, ambiguous)
        for z in &bars {
            let mut alt = st;
            let (f, g) = (rl_minute(z, &mut st, lv, z.h - z.o < z.o - z.l), rl_minute(z, &mut alt, lv, !(z.h - z.o < z.o - z.l)));
            let a = f != g;
            for (k, px) in f {
                match k {
                    Lk::Stop | Lk::Target => if let Some((d, e, aa)) = pos.take() {
                        let why = if k == Lk::Stop { format!("money stop {stop:.2} pt") } else { format!("profit target {tgt:.2} pts") };
                        out.push(mk(d, j, e, j, px, format!("{why}{}", if aa || a { AMB } else { "" }), false));
                    },
                    Lk::Long | Lk::Short => {
                        let d = if k == Lk::Long { 1. } else { -1. };
                        if let Some((pd, e, aa)) = pos.take() { out.push(mk(pd, j, e, j, px, format!("reversed by the other RangeLeader stop{}", if aa || a { AMB } else { "" }), false)); }
                        pos = Some((d, px, a));
                    }
                }
            }
            if let Some(p) = pos.as_mut() { p.2 |= a; }
        }
        if let Some((d, e, aa)) = pos {
            if j == to { out.push(mk(d, j, e, to, c[to], "open".into(), true)); } else { carry = Some((d, j, e, aa)); }
        }
    }
    out
}
const AMB: &str = " [ambiguous minute: TradeStation bar path]";
// ---- end RangeLeader ----

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if id == "cfwtab_rangeleader" && from < to && to < x.r.len() {
        let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d);
        return Some(rangeleader(x, from, to, g("stop", 1.), g("target", 3.)));
    }
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let c = &x.c;
    Some(match id {
        "cfwtab_ma918" => { let (f, s) = (sma(c, 9), sma(c, 18)); close_sar(x, from, to, &|i| if cross_up(&f, &s, i) { 1. } else if cross_dn(&f, &s, i) { -1. } else { 0. }) }
        "cfwtab_stoch" => {
            let (k, dd) = if g("smooth") > 0.5 { let k = sma(&pct_r(&x.h, &x.l, c, 10), 3); let dd = sma(&k, 3); (k, dd) } else { super::kfbpm::slow_kd(x, 10, false) };
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

    #[test]
    fn rangeleader_minute_path_fills_stops_targets_and_flags_order_dependent_minutes() {
        use crate::intraday::Bar1;
        // RangeLeader bar: midpoint outside the prior bar and a wider range
        let r: Vec<crate::Row> = [(100., 101., 99., 100.), (101., 104., 100.5, 103.), (103., 106., 102., 105.)].iter().enumerate().map(|(i, &(o, h, l, c))| crate::Row { ticker: "T".into(), date: format!("2000-01-0{}", i + 1), open: o, high: h, low: l, close: c, volume: None }).collect();
        let x = Ctx::new(&r, &r, &[]);
        assert!(rangeleader(&x, 0, 2, 1.0, 3.0).is_empty()); // bar 1 is a RangeLeader, but bar 2 has no minute bars: no trade
        let lv = (101.0, 98.0, 1.0, 3.0); // buy stop 101, sell stop 98, 1-point stop, 3-point target
        let m = |o, h, l, c| Bar1 { m: 570, o, h, l, c };
        // fill at 101 on the way up, target 104 later in the same minute (open nearer the low: O-L-H-C)
        let mut st = (None, false, false);
        assert_eq!(rl_minute(&m(100.0, 104.5, 99.8, 104.0), &mut st, lv, false), vec![(Lk::Long, 101.0), (Lk::Target, 104.0)]);
        assert_eq!(st, (None, true, false));
        // the same minute through the other path (high first): identical, so not order-dependent
        let mut st2 = (None, false, false);
        assert_eq!(rl_minute(&m(100.0, 104.5, 99.8, 104.0), &mut st2, lv, true), vec![(Lk::Long, 101.0), (Lk::Target, 104.0)]);
        // long at 101; a minute reaching both 102 and 99.9: stopped (low first) or not (high first): order-dependent
        let (mut a, mut b) = ((Some((1.0, 101.0)), true, false), (Some((1.0, 101.0)), true, false));
        let (fa, fb) = (rl_minute(&m(101.0, 102.0, 99.9, 101.0), &mut a, lv, false), rl_minute(&m(101.0, 102.0, 99.9, 101.0), &mut b, lv, true));
        assert_eq!(fa, vec![(Lk::Stop, 100.0)]);
        assert_eq!(fb, vec![(Lk::Stop, 100.0)]); // both paths reach 100 (the 1-point stop) before anything else: not order-dependent
        // a minute reaching both the 1-point stop and the 3-point target: the path decides (counted as ambiguous by rangeleader)
        let (mut a, mut b) = ((Some((1.0, 101.0)), true, false), (Some((1.0, 101.0)), true, false));
        assert_eq!(rl_minute(&m(101.0, 104.2, 99.9, 101.0), &mut a, lv, false), vec![(Lk::Stop, 100.0)]);
        assert_eq!(rl_minute(&m(101.0, 104.2, 99.9, 101.0), &mut b, lv, true), vec![(Lk::Target, 104.0)]);
        // a gap through the sell stop while long: the stop fills at the open, then the reversal short at the same open
        let mut g = (Some((1.0, 101.0)), true, false);
        assert_eq!(rl_minute(&m(97.0, 97.5, 96.0, 97.2), &mut g, lv, false), vec![(Lk::Stop, 97.0), (Lk::Short, 97.0)]);
        assert_eq!(g, (Some((-1.0, 97.0)), true, true));
    }
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

