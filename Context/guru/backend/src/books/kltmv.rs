//! George Kleinman, "The Most Valuable Technical Tool" (TMVTT) in two editions: (1) Commodity Futures and Options (FT, 2004) ch.10 "How I use TMVTT" pp.193-206 (WMA rules 1-4);
//! (2) Trading Commodities and Financial Futures, 3rd ed. (FT Prentice Hall 2005) ch.10 pp.187-196 (23/30 EMA band, set-up bar). Research share `USB HDD 2/tRADING KNO-2/George Kleinman - ...`
//! (scans, RapidOCR text of the chapters in authors/conv-1123/K1/rc/). Book id `kltmv`. Lane K1 (24 Nov 2026, R4 #398 tier 6): the continuous contracts of the markets the books show
//! (Norgate, futures::CARDS). Signals are read at a close and filled at the next close (R4 #374). Helper books::tselhs::Book. Reference authors/code-1123/ref/kltmv_ref.py.
use super::tselhs::Book;
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const IDS: &[&str] = &["kltmv_wma", "kltmv_ema_band"];
const NOTE_WMA: &str = "[Declared adaptation, R4 #340 / #398] Kleinman, Commodity Futures and Options ch.10 pp.193-206: Rule 1 the 30-day weighted moving average (WMA, weights 1..n) designates an up trend after 2 consecutive closes above it, Rule 2 a down trend after 2 consecutive closes below it, each holding until the opposite designation; Rule 3 the 2-day WMA crossing above the 9-day is a buy signal, accepted if the trend is up; Rule 4 the cross below is a sell signal, accepted if the trend is down; a sell signal in an up trend liquidates the long and the book goes short only once the down designation is met (p.190-191). Run on the Norgate continuous contracts (live cattle, wheat: the markets of the book's printed examples). Conventions declared, not counted: signals are read at a close and filled at the NEXT close (R4 #374); the weighted average uses weights 1..n (a standard WMA); no signal before 30 closes exist (data warm-up). Unstated details (3, each one choice, #400): (1) the book's 'if rule 2 is subsequently met ... we then go short' is read as: flat, the 2-day WMA still below the 9-day, and the down trend newly designated on that bar gives the short; the book's 'and vice versa' (p.191: 'it is possible to have a sell signal and no short position and vice versa') is the source of the mirror long case, nothing is invented for it; (2) no trend is designated before two closes of the same side exist; (3) the opposite accepted signal is the only exit (the book prints no stop). Near-twins: none in the live catalogue for the Kleinman 30-day WMA / 2-9 WMA rule (kltmv_ema_band is the other edition's different 23/30 EMA band system).";
const NOTE_EMA: &str = "[Declared adaptation, R4 #340 / #398] Kleinman, Trading Commodities and Financial Futures 3rd ed. ch.10 pp.190-196: daily charts with a 23- and a 30-day EMA band; buy signal: the market closes above BOTH averages (the set-up bar) and subsequently exceeds the set-up bar's high; sell signal: closes below both, then exceeds the set-up bar's low; always in the market (hypothetically reverse on every signal), printed per-contract results for crude oil, soybeans and the euro in 2003. Run on the Norgate continuous contracts of those markets. Conventions declared, not counted: signals are read at a close and filled at the NEXT close (R4 #374); EMAs use 2/(n+1) smoothing started at the first close, no signal before 30 closes exist (data warm-up). Unstated details (4, each one choice, #400): (1) the signal is the bar whose high (low) trades beyond the set-up bar's extreme (the printed buy/sell stop is read at bar level, filled at the next close); (2) the set-up bar is the MOST RECENT close beyond both averages, so a later close beyond the band replaces it, on either side; (3) a close inside the band neither cancels nor replaces the set-up bar; (4) one signal per set-up bar, then the set-up is cleared. The books' dollar figures are per delivery contract while ours is the back-adjusted continuous series (a data statement, not a rule choice). Near-twins: none in the live catalogue for the Kleinman 23/30 EMA band rule (kltmv_wma is the other edition's different weighted-average system).";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, rules, notes| StrategyDef { id, name, group: "Kleinman TMVTT (moving-average systems)", session: 0, dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "kltmv", rules, notes, claim: None };
    vec![
        d("kltmv_wma", "Kleinman TMVTT (Commodity Futures and Options): 30-day WMA trend designation (2 closes) and 2/9-day WMA crossover signals accepted with the trend", &[
            "F|\"If the market is trading (on the close) above the 30WMA line for at least 2 consecutive days, the long-term trend is designated as up. It has to close 2 consecutive days below the line to signal the long-term trend down.\" (p.194)",
            "E|\"When the 2-day WMA crosses the 9-day WMA in an upward direction, a buy signal is generated: the trade is accepted if the long-term trend is up\"; the cross down is a sell signal accepted if the trend is down (p.195-196)",
            "X|\"If we are in a long-term uptrend ... and the 2-day crosses down, we have a sell signal but will not go short. We will liquidate the long position and remain on the sidelines. If rule 2 is subsequently met ... we then go short.\" (p.196)"], NOTE_WMA),
        d("kltmv_ema_band", "Kleinman TMVTT (Trading Commodities and Financial Futures 3rd ed.): 23/30-day EMA band, set-up bar and breakout of its extreme, stop and reverse", &[
            "F|\"On the daily chart, the market must first close above the band. (That's both the 23 and 30 moving averages.) This day forms what is called the set-up bar.\" (p.191)",
            "E|\"The market subsequently must exceed the high price of the set-up bar\" (buy); sell: closes below both averages, then exceeds the low of the set-up bar (p.191)",
            "X|\"If you hypothetically took every buy signal reverse every time a sell signal was generated, and then reverse again on a new buy\" (p.191); no other exit"], NOTE_EMA),
    ]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    IDS.contains(&id).then(|| ("kltmv", "Kleinman TMVTT", if id == "kltmv_wma" { "30-day WMA trend + 2/9 WMA cross" } else { "23/30 EMA band set-up bar" }))
}
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    match id {
        "kltmv_wma" => Some(vec![p("fast", "Fast WMA (2)", 2., 1., 10., 1.), p("mid", "Middle WMA (9)", 9., 3., 30., 1.), p("slow", "Trend WMA (30)", 30., 10., 100., 1.), p("confirm", "Closes to designate the trend (2)", 2., 1., 5., 1.)]),
        "kltmv_ema_band" => Some(vec![p("fast", "Fast EMA (23)", 23., 5., 60., 1.), p("slow", "Slow EMA (30)", 30., 10., 100., 1.)]),
        _ => None,
    }
}

fn wma(c: &[f64], n: usize) -> Vec<f64> {
    let den = (n * (n + 1) / 2) as f64;
    (0..c.len()).map(|t| if t + 1 < n { f64::NAN } else { (0..n).map(|k| (k + 1) as f64 * c[t + 1 - n + k]).sum::<f64>() / den }).collect()
}
fn ema(c: &[f64], n: usize) -> Vec<f64> {
    let a = 2.0 / (n as f64 + 1.0);
    let mut o: Vec<f64> = Vec::with_capacity(c.len());
    for (i, v) in c.iter().enumerate() { o.push(if i == 0 { *v } else { a * v + (1.0 - a) * o[i - 1] }); }
    o
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !IDS.contains(&id) || from >= to || to >= x.r.len() { return None; }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| params(id).unwrap().iter().find(|d| d.key == k).unwrap().def).round().max(1.0) as usize;
    let (c, h, l) = (&x.c, &x.h, &x.l);
    let mut pos = Book::new();
    let mut pend = 0.0f64; // +1 / -1 reverse into, 2 = liquidate only
    let apply = |pos: &mut Book, pend: &mut f64, t: usize| {
        if *pend == 2.0 { pos.exit(t, c[t], "liquidated by the sell signal"); } else if *pend != 0.0 { pos.enter(*pend, t, c[t], "close"); }
        *pend = 0.0;
    };
    if id == "kltmv_wma" {
        let (n1, n2, n3, k) = (g("fast"), g("mid"), g("slow"), g("confirm"));
        let (w1, w2, w3) = (wma(c, n1), wma(c, n2), wma(c, n3));
        let mut trend = 0.0f64;
        for t in 0..=to {
            if t >= from { apply(&mut pos, &mut pend, t); }
            if t < n3 { continue; }
            let prev_trend = trend;
            if t + 1 >= k && (t + 1 - k..=t).all(|j| !w3[j].is_nan() && c[j] > w3[j]) { trend = 1.0; }
            else if t + 1 >= k && (t + 1 - k..=t).all(|j| !w3[j].is_nan() && c[j] < w3[j]) { trend = -1.0; }
            if t < from || t >= to { continue; }
            let up = w1[t] > w2[t] && w1[t - 1] <= w2[t - 1];
            let dn = w1[t] < w2[t] && w1[t - 1] >= w2[t - 1];
            let sig = if up && trend > 0.0 { 1.0 } else if dn && trend < 0.0 { -1.0 } else { 0.0 };
            if sig != 0.0 { if sig != pos.side() { pend = sig; } }
            else if dn && pos.side() > 0.0 { pend = 2.0; }
            else if up && pos.side() < 0.0 { pend = 2.0; }
            else if pos.side() == 0.0 && trend != prev_trend && ((trend < 0.0 && w1[t] < w2[t]) || (trend > 0.0 && w1[t] > w2[t])) { pend = trend; }
        }
    } else {
        let (n1, n2) = (g("fast"), g("slow"));
        let (e1, e2) = (ema(c, n1), ema(c, n2));
        let start = n1.max(n2);
        let mut setup: Option<(f64, f64)> = None; // (side, extreme of the set-up bar)
        for t in 0..=to {
            if t >= from { apply(&mut pos, &mut pend, t); }
            if t < start { continue; }
            if t >= from && t < to {
                if let Some((s, ex)) = setup {
                    // a bar after the set-up bar whose high (low) trades beyond the set-up extreme
                    if (s > 0.0 && h[t] > ex) || (s < 0.0 && l[t] < ex) { if s != pos.side() { pend = s; } setup = None; }
                }
            }
            if c[t] > e1[t] && c[t] > e2[t] { setup = Some((1.0, h[t])); }
            else if c[t] < e1[t] && c[t] < e2[t] { setup = Some((-1.0, l[t])); }
        }
    }
    Some(pos.done(to, c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn weighted_average_weights_recent_days_more() {
        // 3-day WMA of 1,2,3 = (1*1 + 2*2 + 3*3) / 6
        assert!((wma(&[1.0, 2.0, 3.0], 3)[2] - 14.0 / 6.0).abs() < 1e-12);
        assert!(params("kltmv_wma").is_some() && params("x").is_none() && family_of("kltmv_ema_band").is_some());
    }
    #[test]
    fn both_rules_trade_both_sides() {
        let r = super::super::sctraub::tests::bars(1500);
        let x = Ctx::new(&r, &r, &[]);
        for id in IDS {
            let t = run(id, &x, 0, r.len() - 1, &Params::new()).unwrap();
            assert!(t.len() > 3 && t.iter().any(|z| z.dir > 0.) && t.iter().any(|z| z.dir < 0.), "{id}");
        }
    }
}
