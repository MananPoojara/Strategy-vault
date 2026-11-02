//! EuroTrader FX-4 (Euro Dynamics; fx-signals.com / simnet.is/eforex), TradeStation code with the vendor's description. Research
//! share USB HDD 1/TradeStation Systems & Indicators/EuroTrader Trading System (fx-signals.com)/Code.txt (copy
//! authors/batch-j/J1w/eurotrader.txt). Book id `eurtfx`. Lane K17 (titles loop, 2 Nov 2026), to-code row 306, lead decision 88;
//! independent check authors/titles-1030/K17w/k17_ref.py. Orders run in `rrsim`.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::rrsim::{g, o, pd, run_ts, sdef, sma, xavg, A, P};

pub const REVIEWED: &[&str] = &["eurtfx_fx4"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] The printed code: Mav21 = Average(Close,22); MACD(Close,11,22) = XAverage(C,11) - XAverage(C,22) and its signal XAverage(MACD,8) (EMAs seeded with the first value). 'MACD[1] CROSS OVER signal[1]' = MACD above its signal one bar ago and at or below it two bars ago, and MACD still above it today. Buy at this bar's close on that cross with Mav21 rising two bars running (Mav21 > Mav21[1] > Mav21[2]); sell short at the close on the mirror. Exits at this bar's close: a short when Mav21 rises two bars running or on the MACD cross over; a long when Mav21 falls two bars running or on the cross under (exits are taken before entries on one close; an entry reverses). Percent stop PcntStop 0.014: while long at a close, sell next bar at EntryPrice - Close x 0.014 stop (short: EntryPrice + Close x 0.014), so the stop starts the bar after the entry close. The vendor says daily data and the EUR/USD (Futures Truth tested CME EuroFX); daily indices, ETFs and US/India stocks are a declared application (lead decision 88); the vault's standard costs; one unit. The vendor's Futures Truth ranking is not shown.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![sdef("eurtfx", "eurtfx_fx4", "EuroTrader FX-4: MACD(11,22) signal cross with the 22-day average rising (falling) two days, close entries, 1.4% stop",
        "EuroTrader FX-4 (Euro Dynamics, TradeStation code)", "Long/Short", Status::Live, &[
            "F|\"Mav21 = Average (CLOSE, 22) ;\" \"Inputs: PcntStop(0.014000);\" \"Lstoploss = ENTRYPRICE-(CLOSE*PcntStop) ; Sstoploss = ENTRYPRICE+(CLOSE*PcntStop) ;\"",
            "E|\"IF CURRENTBAR > 1 AND MACD (CLOSE, 11, 22)[1] CROSS OVER XAverage (MACD (CLOSE, 11, 22), 8)[1] AND MACD (CLOSE, 11, 22) > XAverage (MACD (CLOSE, 11, 22), 8) AND Mav21 > Mav21[1] AND Mav21[1] > Mav21[2] THEN BUY this bar at CLOSE;\" (SELLSHORT on the mirror)",
            "X|\"IF Mav21 > Mav21[1] AND Mav21[1] > Mav21[2] THEN BUYTOCOVER this bar at CLOSE ;\" and on the MACD cross over; \"IF Mav21 < Mav21[1] AND Mav21[1] < Mav21[2] THEN SELL this bar at CLOSE ;\" and on the cross under",
            "X|\"IF MARKETPOSITION = 1 THEN BEGIN SELL (\\\"%Lstop\\\") next bar at Lstoploss STOP ; END ;\" (short mirror)"],
        NOTE)]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    reviewed(id).then_some(("eurtfx_fx4", "EuroTrader FX-4", "MACD 11/22/8, Mav 22, 1.4% stop"))
}
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    reviewed(id).then(|| vec![pd("pct", "PcntStop (fraction of the close; code 0.014)", 0.014, 0.002, 0.1, 0.001)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let pct = g(p, "pct", 0.014);
    let mav = sma(&x.c, 22);
    let (e11, e22) = (xavg(&x.c, 11), xavg(&x.c, 22));
    let macd: Vec<f64> = e11.iter().zip(&e22).map(|(a, b)| a - b).collect();
    let sig = xavg(&macd, 8);
    Some(run_ts(x, from, to, |i, st, now, next| {
        if i < 2 { return; }
        let c = x.c[i];
        let xo = macd[i - 1] > sig[i - 1] && macd[i - 2] <= sig[i - 2] && macd[i] > sig[i];
        let xu = macd[i - 1] < sig[i - 1] && macd[i - 2] >= sig[i - 2] && macd[i] < sig[i];
        let up = mav[i] > mav[i - 1] && mav[i - 1] > mav[i - 2];
        let dn = mav[i] < mav[i - 1] && mav[i - 1] < mav[i - 2];
        if st.pos == -1 { next.push(o(A::Xs, P::Stop(st.entry_px + c * pct), "%Sstop")); }
        if st.pos == 1 { next.push(o(A::Xl, P::Stop(st.entry_px - c * pct), "%Lstop")); }
        if up || xo { now.push(o(A::Xs, P::Mkt, "SX")); }
        if dn || xu { now.push(o(A::Xl, P::Mkt, "LX")); }
        if xo && up { now.push(o(A::Buy, P::Mkt, "LE")); }
        if xu && dn { now.push(o(A::Sell, P::Mkt, "SE")); }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn eurotrader_fx4() { super::super::nijat::tests::sane(run, REVIEWED); }
}

// Instrument admission: EuroTrader FX-4 2026-11-02 (titles lane K17).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "eurtfx" && reviewed(def.id),
    label: "Vendor EUR/USD system (daily data; Futures Truth tested CME EuroFX); runs on daily indices, ETFs and stocks, US and India, are a declared application.",
    basis: "EuroTrader FX-4 Code.txt (USB HDD 1/TradeStation Systems & Indicators/EuroTrader Trading System (fx-signals.com)); independent check authors/titles-1030/K17w/k17_ref.py",
};
