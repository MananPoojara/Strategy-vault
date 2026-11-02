//! Trend Owl 3 system (strun.com), TradeStation open code. Research share USB HDD 1/TradeStation Systems & Indicators/Trend Owl 3
//! System for TS (strun.com)/Open Code.rar::Open Code.txt (copy authors/batch-j/J1w/trendowl3.txt). Book id `trowl`. Lane K17
//! (titles loop, 2 Nov 2026), to-code row 310; independent check authors/titles-1030/K17w/k17_ref.py. Orders run in `rrsim`.
use crate::connors::{dmi, Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::rrsim::{highest, lowest, o, run_ts, sdef, sma, true_range, A, P};

pub const REVIEWED: &[&str] = &["trowl_3"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] The printed code. ATR = AvgTrueRange(15) = TradeStation's simple 15-bar average of the true range; ADX(14) = Wilder's ADX as the vault computes it (TradeStation's built-in source is not in the share); Highest/Lowest include today's bar. Long setup at the close: C > Highest(H,15) - 4 ATR, C < Highest(H,5) - 1.5 ATR, Average(C,20) > Average(C,40), ADX(14) < 30 -> unless long, buy next bar at Highest(H,2) stop. Long exit: once the highest close since entry (MaxTradeClose, read as the highest close from the entry bar to today; the file does not define it) is above EntryPrice + 1.5 ATR, sell next bar at MaxTradeClose - 1.5 ATR stop, re-set every close. Short setup: ADX(14) < 30, C < Lowest(L,15) + 2 ATR, C > Lowest(L,3) + 1 ATR, Average(C,20) < Average(C,40) -> unless short, sell short next bar at Lowest(L,1) (today's low) stop; exit once the lowest close since entry is below EntryPrice - 2 ATR, at MinTradeClose + 1 ATR stop. An entry stop reverses an opposite position; there is no initial loss stop in the code. One unit. The vendor names no market or test; daily indices, ETFs and US/India stocks are a declared application; the vault's standard costs.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![sdef("trowl", "trowl_3", "Trend Owl 3: pullback in a 20/40-day trend with ADX under 30, stop entry at the 2-day high (1-day low); ATR profit-lock trailing exit",
        "Trend Owl 3 (strun.com, TradeStation code)", "Long/Short", Status::Live, &[
            "F|\"Condition1= C > Highest(H,15) - 4 * AvgTrueRange(15); Condition2= C < Highest(H,5) - 1.5 * AvgTrueRange(15); Condition3= Average(C,20) > Average(C,40); Condition4= Adx(14) < 30;\"",
            "E|\"IF MarketPosition<>1 THEN BEGIN IF Condition1 and Condition2 and Condition3 and Condition4 THEN Buy(\\\"Buy\\\") Next Bar Highest(H,2) Stop;\"",
            "E|\"IF MarketPosition<>-1 THEN BEGIN IF Condition4 and Condition5 and Condition6 and Condition7 THEN Sell Short(\\\"Sell\\\") Next Bar Lowest(L,1) Stop;\" with \"Condition5= C < Lowest(L,15) + 2 * AvgTrueRange(15); Condition6= C > Lowest(L,3) + 1 * AvgTrueRange(15); Condition7= Average(C,20) < Average(C,40);\"",
            "X|\"IF MaxTradeClose > EntryPrice + 1.5 * AvgTrueRange(15) THEN Sell(\\\"LX\\\") Next Bar MaxTradeClose - 1.5 * AvgTrueRange(15) Stop;\" \"IF MinTradeClose < EntryPrice - 2 * AvgTrueRange(15) THEN Buy to Cover(\\\"SX\\\") Next Bar MinTradeClose + 1 * AvgTrueRange(15) Stop;\""],
        NOTE)]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    reviewed(id).then_some(("trowl_3", "Trend Owl 3", "ATR 15, ADX 14 < 30"))
}
pub fn params(id: &str) -> Option<Vec<ParamDef>> { reviewed(id).then(Vec::new) }

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, _p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let a = sma(&true_range(x), 15);
    let adx = dmi(x.r, 14).1;
    let (m20, m40) = (sma(&x.c, 20), sma(&x.c, 40));
    let (h, l, c) = (&x.h, &x.l, &x.c);
    Some(run_ts(x, from, to, |i, st, _now, next| {
        if i < 39 { return; }
        let c4 = adx[i] < 30.;
        let long = c[i] > highest(h, 15, i) - 4. * a[i] && c[i] < highest(h, 5, i) - 1.5 * a[i] && m20[i] > m40[i] && c4;
        let short = c4 && c[i] < lowest(l, 15, i) + 2. * a[i] && c[i] > lowest(l, 3, i) + a[i] && m20[i] < m40[i];
        if st.pos != 1 && long { next.push(o(A::Buy, P::Stop(highest(h, 2, i)), "Buy")); }
        if st.pos == 1 {
            let mx = c[st.entry_i..=i].iter().copied().fold(f64::NEG_INFINITY, f64::max);
            if mx > st.entry_px + 1.5 * a[i] { next.push(o(A::Xl, P::Stop(mx - 1.5 * a[i]), "LX")); }
        }
        if st.pos != -1 && short { next.push(o(A::Sell, P::Stop(lowest(l, 1, i)), "Sell")); }
        if st.pos == -1 {
            let mn = c[st.entry_i..=i].iter().copied().fold(f64::INFINITY, f64::min);
            if mn < st.entry_px - 2. * a[i] { next.push(o(A::Xs, P::Stop(mn + a[i]), "SX")); }
        }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn trend_owl() { super::super::nijat::tests::sane(run, REVIEWED); }
}

// Instrument admission: Trend Owl 3 2026-11-02 (titles lane K17).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "trowl" && reviewed(def.id),
    label: "Vendor TradeStation code with no market or test named; runs on daily indices, ETFs and stocks, US and India, are a declared application.",
    basis: "Trend Owl 3 System for TS (strun.com), Open Code.txt; independent check authors/titles-1030/K17w/k17_ref.py",
};
