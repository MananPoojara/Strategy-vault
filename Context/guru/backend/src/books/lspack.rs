//! LaSalle Street Trading Group System Pack (TradeStation/MultiCharts): LaSalleRsiSystem_Strategy.txt and
//! LaSalle_AtrVolRev_Strategy.txt. Research share USB HDD 1/TradeStation Systems & Indicators/LaSalle Street Trading Group System
//! Pack/ (copies authors/batch-j/J1w/lasalle_rsi.txt, lasalle_atrvolrev.txt). Book id `lspack` (RemaBands from the same pack is
//! `lsrema`). Lane K17 (titles loop, 2 Nov 2026), to-code rows 317-318; independent check authors/titles-1030/K17w/k17_ref.py.
//! Orders run in `rrsim`.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::rrsim::{g, o, pd, run_ts, sdef, sma, true_range, A, P};

pub const REVIEWED: &[&str] = &["lspack_rsi", "lspack_atr_rev"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const RS: &str = "[Declared readings] The printed code is a four-element Laguerre filter (the structure of Ehlers' Laguerre RSI): Var1 = (1 - Ceres) x Close + Ceres x Var1[1]; Var2 = -Ceres x Var1 + Var1[1] + Ceres x Var2[1]; Var3, Var4 likewise; Var5 = the sum of the rising gaps (Var1 - Var2, Var2 - Var3, Var3 - Var4 where >= 0), Var6 = the sum of the falling ones; Var7 = Var5/(Var5 + Var6), kept from the bar before when both are 0. The variables start at 0 before the first bar. Var7 crossing over Lowerband (above 0.18 now, at or below one bar earlier) buys at this bar's close; crossing under Upperband (below 0.75 now, at or above one bar earlier) sells short at this bar's close; each entry reverses the other side, so the system is always in after the first signal. Defaults Ceres 0.55, Lowerband 0.18, Upperband 0.75 as printed. Not the same card as Ehlers' Laguerre RSI 20/80 (ehmd_laguerre_rsi: gamma 0.5, next-open fills); a different source and settings (lead decision 48). The pack names no market or test; daily indices, ETFs and US/India stocks are a declared application; one unit; the vault's standard costs.";
const AT: &str = "[Declared readings] The printed code: Var1 = Ceres x AvgTrueRange(BarsBack), Ceres 0.65, BarsBack 5 (AvgTrueRange = TradeStation's simple average of the true range). 'BUY NEXT BAR OPEN NEXT BAR+Var1 STOP' = a buy stop at the next bar's open + Var1 (the order is placed at the open, so the open is known). 'SELL NEXT BAR OPEN NEXT BAR-Var1 STOP': the pack is written in TradeStation 8 / MultiCharts syntax (its other strategies use SELLSHORT for short entries), where SELL exits a long. So the default is literal: long only, a buy stop at open + Var1 and, while long at the close, an exit stop at open - Var1; after a stop exit the buy stop can fill again later that day. p.rev = 1 reads SELL as the pre-2004 short entry (the to-code row's 'always reversing' reading): a short stop at open - Var1 that reverses the position. Fills at the stop level or the open if the open is through it, along open -> nearer extreme -> other extreme. The pack names no market or test; daily indices, ETFs and US/India stocks are a declared application; one unit; the vault's standard costs.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        sdef("lspack", "lspack_rsi", "LaSalle RSI System: Laguerre-type RSI (Ceres 0.55) crosses up through 0.18 buy, down through 0.75 sell short, at the close",
            "LaSalle RSI System (System Pack)", "Long/Short", Status::Live, &[
                "F|\"Inputs: Ceres(0.550000), Lowerband(0.180000), Upperband(0.750000);\" \"Var1 = (1-Ceres)*CLOSE+Ceres*Var1[1] ; Var2 =-Ceres*Var1+Var1[1]+Ceres*Var2[1] ; Var3 =-Ceres*Var2+Var2[1]+Ceres*Var3[1] ; Var4 =-Ceres*Var3+Var3[1]+Ceres*Var4[1] ;\"",
                "F|\"if Var1 >= Var2 then Var5 = Var1-Var2 ELSE Var6 = Var2-Var1 ;\" (and the two other gaps) \"if Var5+Var6 <> 0 then Var7 = Var5/(Var5+Var6) ;\"",
                "E|\"Var8 = Var7 CROSS OVER Lowerband ; Var9 = Var7 CROSS UNDER Upperband ; if Var8 then BUY (\\\"Buy\\\") THIS BAR CLOSE ; if Var9 then SELLSHORT (\\\"Sell\\\") THIS BAR CLOSE ;\"",
                "X|No exit of its own: the opposite entry reverses the position"],
            RS),
        sdef("lspack", "lspack_atr_rev", "LaSalle ATR volatility breakout: buy a stop 0.65 x ATR(5) above the next open; long exit stop the same distance below it",
            "LaSalle ATR Volatility Reversal (System Pack)", "Long", Status::Live, &[
                "F|\"Inputs: BarsBack(5), Ceres(0.650000);\" \"Var1 = Ceres*AvgTrueRange (BarsBack) ;\"",
                "E|\"BUY NEXT BAR OPEN NEXT BAR+Var1 STOP ;\"",
                "X|\"SELL NEXT BAR OPEN NEXT BAR-Var1 STOP ;\" (SELL = exit long in the pack's TradeStation 8 syntax; p.rev = 1: a short entry)"],
            AT),
    ]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id { "lspack_rsi" => Some(("lspack_rsi", "LaSalle RSI System", "Ceres 0.55, 0.18 / 0.75")),
        "lspack_atr_rev" => Some(("lspack_atr_rev", "LaSalle ATR volatility reversal", "0.65 x ATR(5) from the open")), _ => None }
}
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    match id {
        "lspack_rsi" => Some(vec![]),
        "lspack_atr_rev" => Some(vec![pd("rev", "SELL: 0 = exit long (TradeStation 8 syntax, literal); 1 = short entry, always reversing", 0., 0., 1., 1.)]),
        _ => None,
    }
}

/// Var7 of the LaSalle RSI code (the Laguerre-type RSI), variables starting at 0.
pub fn lrsi(c: &[f64], g: f64) -> Vec<f64> {
    let (mut v1, mut v2, mut v3, mut v4, mut v7) = (0.0f64, 0.0f64, 0.0f64, 0.0f64, 0.0f64);
    c.iter().map(|&x| {
        let n1 = (1. - g) * x + g * v1;
        let n2 = -g * n1 + v1 + g * v2;
        let n3 = -g * n2 + v2 + g * v3;
        let n4 = -g * n3 + v3 + g * v4;
        (v1, v2, v3, v4) = (n1, n2, n3, n4);
        let (mut up, mut dn) = (0.0f64, 0.0f64);
        if v1 >= v2 { up = v1 - v2 } else { dn = v2 - v1 }
        if v2 >= v3 { up += v2 - v3 } else { dn += v3 - v2 }
        if v3 >= v4 { up += v3 - v4 } else { dn += v4 - v3 }
        if up + dn != 0. { v7 = up / (up + dn); }
        v7
    }).collect()
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    if id == "lspack_rsi" {
        let v = lrsi(&x.c, 0.55);
        return Some(run_ts(x, from, to, |i, _st, now, _next| {
            if i < 1 { return; }
            if v[i] > 0.18 && v[i - 1] <= 0.18 { now.push(o(A::Buy, P::Mkt, "Buy")); }
            if v[i] < 0.75 && v[i - 1] >= 0.75 { now.push(o(A::Sell, P::Mkt, "Sell")); }
        }));
    }
    let rev = g(p, "rev", 0.) >= 0.5;
    let v = sma(&true_range(x), 5);
    Some(run_ts(x, from, to, |i, st, _now, next| {
        let (Some(&on), true) = (x.o.get(i + 1), v[i].is_finite()) else { return };
        let d = 0.65 * v[i];
        next.push(o(A::Buy, P::Stop(on + d), "Buy"));
        if rev { next.push(o(A::Sell, P::Stop(on - d), "Sell")); } else if st.pos == 1 { next.push(o(A::Xl, P::Stop(on - d), "Sell")); }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn lasalle_pack() {
        super::super::nijat::tests::sane(run, REVIEWED);
        let r = super::super::nijat::tests::bars(800);
        let x = Ctx::new(&r, &r, &[]);
        let mut p = Params::new();
        p.insert("rev".into(), 1.);
        assert!(run("lspack_atr_rev", &x, 0, r.len() - 1, &p).unwrap().iter().any(|z| z.dir < 0.));
        assert!(run("lspack_atr_rev", &x, 0, r.len() - 1, &Params::new()).unwrap().iter().all(|z| z.dir > 0.));
        let ramp: Vec<f64> = (1..80).map(|k| k as f64).collect();
        assert!(*lrsi(&ramp, 0.55).last().unwrap() > 0.99);
    }
}

// Instrument admission: LaSalle Street Trading Group System Pack (RSI, ATR volatility reversal) 2026-11-02 (titles lane K17).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "lspack" && reviewed(def.id),
    label: "Vendor TradeStation/MultiCharts pack with no market or test named; runs on daily indices, ETFs and stocks, US and India, are a declared application.",
    basis: "LaSalle Street Trading Group System Pack, LaSalleRsiSystem_Strategy.txt and LaSalle_AtrVolRev_Strategy.txt; independent check authors/titles-1030/K17w/k17_ref.py",
};
