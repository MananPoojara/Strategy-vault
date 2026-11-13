//! Jeffrey Owen Katz with Donna L. McCormick, "A Rule-Based Approach To Trading" (the thrust retracement system), Technical
//! Analysis of Stocks & Commodities V.14:12 (December 1996) pp.509-514 (research share Trading Knowledge I/Stocks & Commodities DVD
//! 11.26 (1982 - 2008) (traders.com)/VOLUMES/V14/C12/ARULEBA.pdf), Figure 1 EasyLanguage. Book id `sckzthr`. Lane PA1 (13 Nov
//! 2026). 30-minute S&P bars from the vault's SPX 1-minute file; orders resolved minute by minute. Independent check
//! authors/pa1/pa1_katz_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use crate::intraday::day_trade;

pub const INTRA: &[&str] = &["sckzthr_sp30"];
pub fn reviewed(id: &str) -> bool { INTRA.contains(&id) }

const NOTE: &str = "[Adapted: 2 unstated details, R4 #297; unverified] The printed EasyLanguage (Figure 1) on 30-minute bars of the S&P 500 (SPX 1-minute file, buckets from 09:30 ET; the 20-bar average range runs across days). Each day the highest high / lowest low so far and their bars are tracked from the day's first bar. Long set-up (when flat or short, not on the day's last bar): the low came after the day's start, the high after the low and before this bar, and the thrust hp - lp exceeds Thrust (1) x the 20-bar average range x sqrt(bars from the low to the high); RetLevel = lp + (hp - lp) x 70.5% (RetPct, the article's optimum); if this bar's and the previous bar's lows are above RetLevel, buy at RetLevel limit on the next bar. Short mirrored. Exits: a limit at the entry +/- 50% of the day's hp - lp (from the bar after the entry), market after 16 bars (never reached inside one day), the $350 stop the article used on the S&P (0.70 S&P points at 1996's $500 a point; the S&P points are literal, so the card runs on SPX only, R4 #196 (2)), and the close of the day ('If neither the stop nor the profit target were hit, we exited at the close of the day'). Unstated in the article: (1) when the money-management stop starts working; vault reading: from the minute after the entry fill; (2) a minute that reaches both the stop and the target; vault reading: the stop first (counted on the gate row, #175). The T-bond version (15-minute bars, $110 stop) needs T-bond intraday data the vault does not hold. Article (S&P 500, 15-minute bars, 03/01/95-11/01/95, TradeStation): 91 trades, 46% profitable, +$14,000; T-bonds 15-minute: 85 trades, 52%, +$5,718.75 (Figures 4-6). The vault's SPX minutes start in 2008, so no book check.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "sckzthr_sp30", name: "Katz thrust retracement: after a day's thrust, buy a 70.5% retracement with a limit, half-thrust target, $350 stop, out by the close (SPX 30-minute)", group: "S&C: A Rule-Based Approach To Trading (Katz & McCormick, V.14:12)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "sckzthr", rules: &[
            "E|\"If lb >= fb And hb > lb And hb < CurrentBar Then Begin If hp-lp > Thrust * Norm * SquareRoot(hb-lb) Then Begin RetLevel = lp + (hp - lp) * RetPct / 100; If L > RetLevel And L[1] > RetLevel Then Begin Buy At RetLevel Limit;\" Norm = Average(Range,20); RetPct 70.5, Thrust 1 (V.14:12 Figure 1, p.511)",
            "X|\"ExitLong At EntryPrice(0) + (hp - lp) * 0.50 Limit; If BarsSinceEntry > 16 Then ExitLong At Market;\" (Figure 1); \"If neither the stop nor the profit target were hit, we exited at the close of the day\" (p.510)",
            "X|\"The same system is used on the 30-minute bars of the S&P 500, except the stop-loss is $350.\" (Figure 5)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    reviewed(id).then_some(("sckzthr_sp30", "Katz thrust retracement (S&C)", "S&P 30-minute"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("ret", "RetPct (article 70.5)", 70.5, 30., 80., 0.5), p("thrust", "Thrust (article 1)", 1., 0.25, 3., 0.25),
        p("stop", "Stop, S&P points ($350 at $500 a point = 0.70)", 0.70, 0.1, 10., 0.05)])
}

struct Bar { i: usize, h: f64, l: f64, c: f64, m: Vec<crate::intraday::Bar1>, last: bool, first: bool }

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from > to || to >= x.c.len() { return None; }
    if x.minutes.is_none() { return Some(vec![]); }
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d);
    let (ret, thr, stp) = (g("ret", 70.5), g("thrust", 1.), g("stop", 0.70));
    let mut bars: Vec<Bar> = vec![];
    for i in from.saturating_sub(5)..=to {
        let Some((_, _, m, v)) = crate::intraday_ep::day_bars(x, i, 30) else { continue };
        let n = v.len();
        for (j, b) in v.iter().enumerate() { bars.push(Bar { i, h: b.h, l: b.l, c: b.c, m: m[b.a..b.b].to_vec(), last: j + 1 == n, first: j == 0 }); }
    }
    let mut out = vec![];
    let mut pos: Option<(f64, usize, f64, usize)> = None; // dir, day, entry px, entry bar
    let (mut hp, mut lp, mut hb, mut lb, mut fb) = (f64::NEG_INFINITY, f64::INFINITY, 0usize, 0usize, 0usize);
    let (mut have_hb, mut have_lb) = (false, false);
    let (mut entry, mut target): (Option<(f64, f64)>, Option<f64>) = (None, None);
    for k in 0..bars.len() {
        let b = &bars[k];
        if b.first { hp = f64::NEG_INFINITY; lp = f64::INFINITY; have_hb = false; have_lb = false; fb = k; }
        if b.i >= from && (entry.is_some() || target.is_some()) {
            let mut filled: Option<usize> = None;
            for (mi, q) in b.m.iter().enumerate() {
                if let Some((d, di, ep, kb)) = pos {
                    if filled.map_or(true, |f| mi > f) {
                        let sl = ep - d * stp;
                        let hit_s = if d > 0. { q.l <= sl } else { q.h >= sl };
                        let hit_t = target.is_some_and(|t| kb < k && if d > 0. { q.h >= t } else { q.l <= t });
                        if hit_s { out.push(day_trade(d, di, ep, b.i, if d > 0. { q.o.min(sl) } else { q.o.max(sl) }, "stop", "limit", false)); pos = None; target = None; continue; }
                        if hit_t { let t = target.unwrap(); out.push(day_trade(d, di, ep, b.i, if d > 0. { q.o.max(t) } else { q.o.min(t) }, "target", "limit", false)); pos = None; target = None; continue; }
                    }
                }
                if let (Some((d, lv)), None) = (entry, filled) {
                    if (d > 0. && q.l <= lv) || (d < 0. && q.h >= lv) {
                        let px = if d > 0. { q.o.min(lv) } else { q.o.max(lv) };
                        if let Some((pd, di, ep, _)) = pos { if pd == -d { out.push(day_trade(pd, di, ep, b.i, px, "reversed", "limit", false)); pos = None; } }
                        if pos.is_none() { pos = Some((d, b.i, px, k)); filled = Some(mi); target = None; }
                        entry = None;
                    }
                }
            }
        }
        entry = None; target = None;
        if b.h > hp { hp = b.h; hb = k; have_hb = true; }
        if b.l < lp { lp = b.l; lb = k; have_lb = true; }
        if b.i < from { continue; }
        if b.last { if let Some((d, di, ep, _)) = pos.take() { out.push(day_trade(d, di, ep, b.i, b.c, "end of day", "limit", false)); } continue; }
        let norm = if k >= 19 { bars[k - 19..=k].iter().map(|z| z.h - z.l).sum::<f64>() / 20. } else { f64::NAN };
        let side = pos.map_or(0., |q| q.0);
        if have_hb && have_lb {
            if side <= 0. && lb >= fb && hb > lb && hb < k && hp - lp > thr * norm * ((hb - lb) as f64).sqrt() {
                let rl = lp + (hp - lp) * ret / 100.;
                if b.l > rl && bars[k - 1].l > rl { entry = Some((1., rl)); }
            }
            if side >= 0. && hb >= fb && lb > hb && lb < k && hp - lp > thr * norm * ((lb - hb) as f64).sqrt() {
                let rl = hp - (hp - lp) * ret / 100.;
                if b.h < rl && bars[k - 1].h < rl { entry = Some((-1., rl)); }
            }
        }
        if let Some((d, _, ep, _)) = pos { target = Some(ep + d * (hp - lp) * 0.5); }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn no_minutes_no_trades() {
        let r = super::super::sctraub::tests::bars(30);
        let x = Ctx::new(&r, &r, &[]);
        assert!(run("sckzthr_sp30", &x, 0, r.len() - 1, &Params::new()).unwrap().is_empty());
        assert!(params("sckzthr_sp30").unwrap().iter().any(|q| q.key == "stop" && q.def == 0.70));
    }
}

// Admission: the intraday block (intraday_ep::PA1, SPX only: S&P points literal, R4 #196 (2)).
