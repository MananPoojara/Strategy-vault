//! Zeeport system (www.alfanetsys.com / alfaranda.com), TradeStation open code. Research share USB HDD 1/TradeStation Systems &
//! Indicators/Zeeport System (alfaranda.com)/Open Code.rar::Open Code.txt (copy authors/batch-j/J1w/zeeport.txt). Book id
//! `alfzp` (same vendor as `alfar`). Lane K17 (titles loop, 2 Nov 2026), to-code row 311, lead decisions 19, 88; independent
//! check authors/titles-1030/K17w/k17_ref.py. Orders run in `rrsim`.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::rrsim::{o, run_ts, sdef, xavg, A, P};

pub const REVIEWED: &[&str] = &["alfzp_zeeport"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] The printed code: HighOsc = High - XAverage(High,10), LowOsc = Low - XAverage(Low,10), CloseAvg = XAverage(Close,60) (EMAs seeded with the first value). BuySetup = CloseAvg rising, Close > CloseAvg and LowOsc < 0; with BuySetup and LowOsc > LowOsc[1], buy next bar at market (reversing a short). While long, HighOsc crossing under 0 (below 0 now, at or above 0 one bar earlier) sells next bar at market. Short side mirrored (SellSetup = CloseAvg falling, Close < CloseAvg, HighOsc > 0; sell short when HighOsc < HighOsc[1]; cover when LowOsc crosses over 0). Len1 10 and Len2 60 as printed. STOP HELD: the trailing stop TrailStp 700 is a dollar amount per contract ('Close-(TrailStp/BigPointValue)'), not converted (lead decisions 19, 88); the rest is coded. One unit. Daily indices, ETFs and US/India stocks are a declared application; the vault's standard costs.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![sdef("alfzp", "alfzp_zeeport", "Zeeport (alfanetsys): in a rising (falling) 60-day EMA trend, buy (short) next open when the low (high) oscillator turns back; out on the opposite oscillator crossing zero",
        "Zeeport (alfanetsys.com, TradeStation code)", "Long/Short", Status::Live, &[
            "F|\"Input: Len1(10), Len2(60), TrailStp(700);\" \"HighOsc = High - XAverage(High, Len1); LowOsc = Low - XAverage(Low, Len1); CloseAvg = XAverage(Close, Len2);\"",
            "E|\"BuySetup = CloseAvg > CloseAvg[1] AND Close > CloseAvg AND LowOsc < 0;\" \"If BuySetup AND LowOsc > LowOsc[1] then Buy next bar at Market;\" (short: \"SellSetup = CloseAvg < CloseAvg[1] AND Close < CloseAvg AND HighOsc > 0;\" \"If SellSetup AND HighOsc < HighOsc[1] then Sell Short next bar at Market;\")",
            "X|\"If HighOsc Crosses Under 0 then Sell (\\\"LX\\\") next bar at Market;\" \"If LowOsc crosses Over 0 then Buy to Cover(\\\"SX\\\") next bar at Market;\" (the $700 trailing stop is held)"],
        NOTE)]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    reviewed(id).then_some(("alfzp_zeeport", "Zeeport oscillator pullback", "EMA 10 / 60"))
}
pub fn params(id: &str) -> Option<Vec<ParamDef>> { reviewed(id).then(Vec::new) }

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, _p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let (eh, el, ca) = (xavg(&x.h, 10), xavg(&x.l, 10), xavg(&x.c, 60));
    let ho: Vec<f64> = x.h.iter().zip(&eh).map(|(a, b)| a - b).collect();
    let lo: Vec<f64> = x.l.iter().zip(&el).map(|(a, b)| a - b).collect();
    Some(run_ts(x, from, to, |i, st, _now, next| {
        if i < 1 { return; }
        let c = x.c[i];
        if ca[i] > ca[i - 1] && c > ca[i] && lo[i] < 0. && lo[i] > lo[i - 1] { next.push(o(A::Buy, P::Mkt, "LE")); }
        if st.pos == 1 && ho[i] < 0. && ho[i - 1] >= 0. { next.push(o(A::Xl, P::Mkt, "LX")); }
        if ca[i] < ca[i - 1] && c < ca[i] && ho[i] > 0. && ho[i] < ho[i - 1] { next.push(o(A::Sell, P::Mkt, "SE")); }
        if st.pos == -1 && lo[i] > 0. && lo[i - 1] <= 0. { next.push(o(A::Xs, P::Mkt, "SX")); }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn zeeport() { super::super::nijat::tests::sane(run, REVIEWED); }
}

// Instrument admission: Zeeport 2026-11-02 (titles lane K17).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "alfzp" && reviewed(def.id),
    label: "Vendor futures code with no test shown (dollar trailing stop held); runs on daily indices, ETFs and stocks, US and India, are a declared application.",
    basis: "Zeeport System (alfaranda.com), Open Code.txt; independent check authors/titles-1030/K17w/k17_ref.py",
};
