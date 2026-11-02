//! Aspen 500 system (newbeach.net), TradeStation open code. Research share USB HDD 1/TradeStation Systems & Indicators/Aspen 500
//! System for TS (newbeach.net)/Open_Code.rar::Open_Code.txt (copy authors/batch-j/J1w/aspen500.txt). Book id `aspen`.
//! Lane K17 (titles loop, 2 Nov 2026), to-code row 312; independent check authors/titles-1030/K17w/k17_ref.py. Orders run in `rrsim`.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::rrsim::{o, run_ts, sdef, sma, A, P};

pub const REVIEWED: &[&str] = &["aspen_500"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] The printed code ('Commodity Futures Trading System Algorithm'): Average = simple moving average of the close. When flat at the close and the 25-day and the 50-day averages have both risen two bars running (A > A[1] > A[2]), buy next bar at market; when both have fallen two bars running, sell short next bar at market. While long, the 25-day average falling two bars running sells next bar at market; while short, rising two bars running covers. Entries only when flat (MarketPosition = 0), so an exit and a new entry never happen on one bar. Lengths 25 and 50 as printed. One unit. Daily indices, ETFs and US/India stocks are a declared application; the vault's standard costs.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![sdef("aspen", "aspen_500", "Aspen 500: enter at the next open when the 25- and 50-day averages both rise (fall) two days running; out when the 25-day turns for two days",
        "Aspen 500 (newbeach.net, TradeStation code)", "Long/Short", Status::Live, &[
            "E|\"If (MarketPosition = 0) and Average(Close,25) > Average(Close,25)[1] and Average(Close,50) > Average(Close,50)[1] and Average(Close,25)[1] > Average(Close,25)[2] and Average(Close,50)[1] > Average(Close,50)[2] then Buy (\\\"LE\\\") Next Bar at Market;\" (short entry mirrored)",
            "X|\"If (MarketPosition = +1) and Average(Close,25) < Average(Close,25)[1] and Average(Close,25)[1] < Average(Close,25)[2] then Sell (\\\"LX\\\") Next Bar at Market;\" (short exit mirrored)"],
        NOTE)]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    reviewed(id).then_some(("aspen_500", "Aspen 500 average slopes", "SMA 25 / 50"))
}
pub fn params(id: &str) -> Option<Vec<ParamDef>> { reviewed(id).then(Vec::new) }

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, _p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let (a, b) = (sma(&x.c, 25), sma(&x.c, 50));
    Some(run_ts(x, from, to, |i, st, _now, next| {
        if i < 2 { return; }
        let (au, ad) = (a[i] > a[i - 1] && a[i - 1] > a[i - 2], a[i] < a[i - 1] && a[i - 1] < a[i - 2]);
        let (bu, bd) = (b[i] > b[i - 1] && b[i - 1] > b[i - 2], b[i] < b[i - 1] && b[i - 1] < b[i - 2]);
        if st.pos == 0 && au && bu { next.push(o(A::Buy, P::Mkt, "LE")); }
        if st.pos == 0 && ad && bd { next.push(o(A::Sell, P::Mkt, "SE")); }
        if st.pos == 1 && ad { next.push(o(A::Xl, P::Mkt, "LX")); }
        if st.pos == -1 && au { next.push(o(A::Xs, P::Mkt, "SX")); }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn aspen_500() { super::super::nijat::tests::sane(run, REVIEWED); }
}

// Instrument admission: Aspen 500 2026-11-02 (titles lane K17).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "aspen" && reviewed(def.id),
    label: "Vendor futures code with no test shown; runs on daily indices, ETFs and stocks, US and India, are a declared application.",
    basis: "Aspen 500 System for TS (newbeach.net), Open_Code.txt; independent check authors/titles-1030/K17w/k17_ref.py",
};
