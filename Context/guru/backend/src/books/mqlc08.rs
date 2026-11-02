//! 2008.mq4 (Sergejs Bogdanovs), one of the "33 Experts from 2008 MQL Championship" MQL4 sources. Research share USB HDD 1/
//! META TRADER/MetaTrader/33 Experts from 2008 MQL Championship/2008.mq4 (copy authors/batch-j/J1w/rsi4060.mq4). Book id
//! `mqlc08`. Lane K17 (titles loop, 2 Nov 2026), to-code row 320, lead decisions 88, 114; independent check
//! authors/titles-1030/K17w/k17_ref.py. Orders run in `rrsim`.
use crate::connors::{rsi, Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::rrsim::{o, run_ts, sdef, A, P};

pub const REVIEWED: &[&str] = &["mqlc08_rsi_40_60"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] The MQL4 source: Signal() = +1 when iRSI(14) of the last closed bar (shift 1) is above 40 and the bar before (shift 2) was below 40; -1 when shift 1 is below 60 and shift 2 was above 60 (strict inequalities as printed). Each tick: with an open order, ClosePosition() closes a buy on -1 and a sell on +1; with no order, OpenPosition() buys on +1 and sells on -1. As the signal holds for the whole bar, an order closed on the bar's first tick is replaced by the opposite one on the next tick: a stop-and-reverse at the next open after the signal close (fills at that open). No stop or target. RSI = Wilder's RSI(14) as the vault computes it (MetaTrader's built-in source is not printed). Lots() (margin-based lot size, MR 0.6) is money management, not applied: one unit. Timeframe-free EA (the championship ran on hourly forex charts); daily indices, ETFs and US/India stocks are a declared application (lead decisions 88, 114); the vault's standard costs. The championship result is not shown.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![sdef("mqlc08", "mqlc08_rsi_40_60", "MQL Championship 2008 EA (Bogdanovs): RSI(14) back up through 40 buys, back down through 60 sells short, stop and reverse at the next open",
        "2008.mq4 (33 Experts from the 2008 MQL Championship)", "Long/Short", Status::Live, &[
            "F|\"extern int p = 14; extern int levup = 40; extern int levdn = 60;\" \"rsi = iRSI(NULL,0,p,0,1); rsi2= iRSI(NULL,0,p,0,2);\"",
            "E|\"if((rsi2 < levup) && (rsi > levup)) { return(1); } if((rsi2 > levdn) && (rsi < levdn)) { return(-1); }\" \"if(Signal() > 0) { OrderSend(Symbol(),OP_BUY,lots,Ask,10,0,0,\\\"Buy\\\",0,0,Blue); ... if(Signal() < 0) { OrderSend(Symbol(),OP_SELL,lots,Bid,10,0,0,\\\"Sell\\\",0,0,Red);\"",
            "X|\"if(OrderType() == OP_BUY) { if(Signal() < 0) { OrderClose(OrderTicket(),OrderLots(),Bid,0,Red);\" (sell closed on Signal() > 0)"],
        NOTE)]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    reviewed(id).then_some(("mqlc08_rsi_40_60", "MQL Championship 2008 RSI EA", "RSI 14, 40 / 60"))
}
pub fn params(id: &str) -> Option<Vec<ParamDef>> { reviewed(id).then(Vec::new) }

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, _p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let r = rsi(&x.c, 14);
    Some(run_ts(x, from, to, |i, _st, _now, next| {
        if i < 1 { return; }
        if r[i - 1] < 40. && r[i] > 40. { next.push(o(A::Buy, P::Mkt, "Buy")); } else if r[i - 1] > 60. && r[i] < 60. { next.push(o(A::Sell, P::Mkt, "Sell")); }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rsi_40_60() { super::super::nijat::tests::sane(run, REVIEWED); }
}

// Instrument admission: MQL Championship 2008 EA 2026-11-02 (titles lane K17).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "mqlc08" && reviewed(def.id),
    label: "A timeframe-free MetaTrader 4 forex EA from the 2008 MQL Championship, no result shown; runs on daily indices, ETFs and stocks, US and India, are a declared application.",
    basis: "2008.mq4 (Sergejs Bogdanovs), 33 Experts from 2008 MQL Championship; independent check authors/titles-1030/K17w/k17_ref.py",
};
