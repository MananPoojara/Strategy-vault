//! enLight DX II Trader 1.02 (enLight Trading Solutions, tradingsolutions.ru, 2006-2009), MetaTrader 4 expert advisor on D1 bars
//! (research share USB HDD 1/META TRADER/MetaTrader/enLight DX II (fxchannel.ru)/enLight_DX_II_Trader_1_02.mq4). Book id `mtenl`.
//! Lane K18 (titles loop, 2 Nov 2026), to-code row 328; independent check authors/titles-1030/K18w/k18_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::rrsim::{o, run_ts_ep6, sma, true_range, A, P};
use super::sctraub::ema0;

pub const REVIEWED: &[&str] = &["mtenl_dx2"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] MetaTrader 4 expert advisor that reads daily bars itself (iMA/iATR on PERIOD_D1); the vault runs it on daily indices, ETFs and stocks as a declared application (decision 114). Signal from the last closed day: ema_h / ema_l = EMA(5) of the highs / lows with ma_shift 21 (the value from 21 bars earlier; EMAs seeded with the first bar); BUY when High > ema_h > Low and Close > Open, else SELL when Low < ema_l < High and Close < Open. ATR(14) = 14-bar simple average of the true range (MetaTrader). At the start of each day (the EA's Hour() < 2, read as the open, with the open as the first price): an open position is closed at the open on the opposite signal (ReverseClose); a pending stop is deleted on the opposite signal, otherwise moved to the last day's high (low) and stop to its low (high) when the old levels are wider and the open is inside that range; with no position or pending order and a signal, a buy stop goes at the last day's high with its stop at that day's low and target 9.87 x ATR above (sell stop mirrored) if the open is inside the day's range. A pending stop stays until filled, moved or deleted. Open trades: the trailing stop (2.2 x ATR from the price once the profit exceeds 2.2 x ATR) and the break-even move (stop to entry once the profit reaches 0.6 x ATR) are updated at each close and act from the next bar; stops, targets and stop entries fill on the vault's TradeStation-style bar path (rrsim). As printed, the first sell stop's target uses the fixed TakeProfit of 987 points instead of the ATR multiple, so it is held (no target) until a later modification gives it the ATR target. HELD: the 5-point offsets (StopLossLevel, BezubSize), the 10-point trail step and the 987-point sell target (point amounts, decision 19); the code's rounding of ATR distances to whole points and the broker stop level / spread checks are not applied. Lots fixed (the martingale counters are unused). No results are printed. Runs on indices, ETFs and stocks, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "mtenl_dx2", name: "enLight DX II EA: day straddling its shifted 5-day EMA of highs (lows) -> stop at its high (low), ATR target, trail and break-even", group: "MT4 EA: enLight DX II Trader", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "mtenl", rules: &[
            "E|if(high1>ema_h && low1<ema_h && close1>open1) Signal=OP_BUY; else if(low1<ema_l && high1>ema_l && close1<open1) Signal=OP_SELL; ema = iMA(D1, 5, shift 21, EMA, High/Low, 1)",
            "E|OP_BUYSTOP at high1 (+5 points held), stop low1, target high1 + 9.87 x ATR(14); OP_SELLSTOP mirrored (its first target is the fixed 987 points: held)",
            "X|ReverseClose on the opposite signal; trailing stop 2.2 x ATR once that far in profit; stop to entry at 0.6 x ATR profit (UseATR true)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { reviewed(id).then_some(("mtenl_dx2", "enLight DX II EA (MT4)", "EMA 5 shift 21, ATR 14")) }

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let mut v = reviewed(id).then(|| vec![p("tpk", "Target x ATR (9.87)", 9.87, 0.5, 20., 0.01), p("tsk", "Trailing x ATR (2.2)", 2.2, 0.5, 10., 0.1), p("nlk", "Break-even at x ATR profit (0.6)", 0.6, 0.1, 5., 0.1)])?;
    crate::intraday_ep6::add_param(id, &mut v); // EP6 (#243)
    Some(v)
}

#[derive(Clone, Copy)]
struct Pend { d: i8, lv: f64, sl: f64, tp: f64 }

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let Some((from, to)) = crate::intraday_ep6::window(id, x, from, to, p) else { return Some(vec![]) }; // EP6 #243 (c)
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d);
    let (tpk, tsk, nlk) = (g("tpk", 9.87), g("tsk", 2.2), g("nlk", 0.6));
    let (o_, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let n = c.len();
    let (eh, el) = (ema0(h, 5), ema0(l, 5));
    let atr = sma(&true_range(x), 14);
    let sig: Vec<i8> = (0..n).map(|t| {
        if t < 21 { return 0; }
        let (a, b) = (eh[t - 21], el[t - 21]);
        if h[t] > a && l[t] < a && c[t] > o_[t] { 1 } else if l[t] < b && h[t] > b && c[t] < o_[t] { -1 } else { 0 }
    }).collect();
    let mut pend: Option<Pend> = None;
    let (mut sl, mut tp) = (f64::NAN, f64::NAN);
    Some(run_ts_ep6(x, from, to, crate::intraday_ep6::on(id, p), |t, st, _now, next| {
        if t < from { return; }
        // the pending stop of bar t: filled when the bar reached it; the position then carries its stop and target
        if let Some(q) = pend {
            if (q.d > 0 && h[t] >= q.lv) || (q.d < 0 && l[t] <= q.lv) {
                pend = None;
                if st.pos == q.d { sl = q.sl; tp = q.tp; }
            }
        }
        let mut pos = st.pos;
        let ep = st.entry_px;
        if pos != 0 {
            let (tsd, nld) = (tsk * atr[t], nlk * atr[t]);
            if pos > 0 {
                if c[t] - ep > tsd && sl < c[t] - tsd { sl = c[t] - tsd; } else if sl < ep && c[t] - ep >= nld { sl = ep; }
            } else if ep - c[t] > tsd && sl > c[t] + tsd { sl = c[t] + tsd; } else if sl > ep && ep - c[t] >= nld { sl = ep; }
        }
        if t + 1 >= n { return; }
        let (s, op, tpd) = (sig[t], o_[t + 1], tpk * atr[t]);
        if pos == -1 && s == 1 { next.push(o(A::Xs, P::Mkt, "reverse close")); pos = 0; }
        if pos == 1 && s == -1 { next.push(o(A::Xl, P::Mkt, "reverse close")); pos = 0; }
        if let Some(q) = pend {
            if (q.d > 0 && s == -1) || (q.d < 0 && s == 1) { pend = None; }
            else if q.d > 0 && (q.lv > h[t] || q.sl < l[t]) && op < h[t] && op > l[t] { pend = Some(Pend { d: 1, lv: h[t], sl: l[t], tp: h[t] + tpd }); }
            else if q.d < 0 && (q.lv < l[t] || q.sl > h[t]) && op < h[t] && op > l[t] { pend = Some(Pend { d: -1, lv: l[t], sl: h[t], tp: l[t] - tpd }); }
        }
        if pos == 0 && pend.is_none() && s != 0 && h[t] > op && l[t] < op {
            pend = Some(if s > 0 { Pend { d: 1, lv: h[t], sl: l[t], tp: h[t] + tpd } } else { Pend { d: -1, lv: l[t], sl: h[t], tp: f64::NAN } });
        }
        if pos > 0 { next.push(o(A::Xl, P::Stop(sl), "stop")); if tp.is_finite() { next.push(o(A::Xl, P::Lim(tp), "target")); } }
        if pos < 0 { next.push(o(A::Xs, P::Stop(sl), "stop")); if tp.is_finite() { next.push(o(A::Xs, P::Lim(tp), "target")); } }
        if let Some(q) = pend {
            let (e, xa) = if q.d > 0 { (A::Buy, A::Xl) } else { (A::Sell, A::Xs) };
            next.push(o(e, P::Stop(q.lv), "pending stop"));
            next.push(o(xa, P::Stop(q.sl), "stop"));
            if q.tp.is_finite() { next.push(o(xa, P::Lim(q.tp), "target")); }
        }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn enlight_trades() {
        let r = super::super::sctraub::tests::bars(2500);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("mtenl_dx2", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() > 3 && t.iter().any(|z| z.dir > 0.) && t.iter().any(|z| z.dir < 0.));
        for w in t.windows(2) { assert!(w[1].legs[0].i >= w[0].exit_i); }
        assert!(run("mtenl_x", &x, 0, 5, &Params::new()).is_none());
    }
}

// Instrument admission: enLight DX II EA 2026-11-02 (titles lane K18).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "mtenl" && reviewed(def.id),
    label: "MetaTrader 4 forex expert advisor on daily bars, with no published results. Runs on indices, ETFs and stocks, US and India, are a declared application.",
    basis: "enLight_DX_II_Trader_1_02.mq4 (printed code); independent check authors/titles-1030/K18w/k18_ref.py",
};
