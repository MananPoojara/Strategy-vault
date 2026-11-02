//! EMA_CROSS expert advisor (Coders Guru, forex-tsd.com), MQL4 source shipped in the Forexmentor "Mastering Metatrader 4 in 90
//! Minutes" members' toolkit. Research share USB HDD 2/tRADING KNO-2/_Forexmentor.com/Alan Benefield - Mastering Metatrader 4 in
//! 90 Minutes & Members Site/BONUS 1 - Metatrader Toolkit/EMA_CROSS.mq4 (copy authors/batch-j/J1w/emacross.mq4). Book id `mt4ec`.
//! Lane K17 (titles loop, 2 Nov 2026), to-code row 319, lead decisions 19, 88, 114; independent check
//! authors/titles-1030/K17w/k17_ref.py. Orders run in `rrsim`.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::rrsim::{g, o, pd, run_ts, sdef, xavg, A, P};

pub const REVIEWED: &[&str] = &["mt4ec_ema_cross"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] The MQL4 source, read literally. SEma = EMA(Close,10), LEma = EMA(Close,80) (alpha 2/(n+1), started at the first close). The EA calls Crossed(LEma, SEma): its static direction is 1 while the FIRST argument (the 80-bar EMA) is above the second, 2 while it is below (unchanged when equal), and it reports a direction only when it changes; the first call reports the starting direction. When no order is open: direction 1 -> BUY, direction 2 -> SELL. So the code as printed buys when the 10-bar EMA crosses BELOW the 80-bar EMA and sells short when it crosses above (the comment 'up' sits on the reversed arguments). An open buy is closed when direction 2 is reported, a sell when 1 is reported, and the EA does not reopen on that same change: after a close it waits for the next change. It runs from the 100th bar (the 'Bars<100' check). Daily declared application (lead decisions 88, 114): the EA reads the current bar (shift 0); here the direction is taken at each close and orders fill at the next open. p.conv = 1 swaps the arguments, Crossed(SEma, LEma): the conventional buy on the 10-bar EMA crossing above the 80-bar. HELD, in pips, not converted (lead decisions 19, 88): TakeProfit 100 pips and TrailingStop 35 pips. With them held, a trade ends only on the next direction change, after which the EA waits for the change after that, so within a run every trade is on the side of the first one; in the EA the pip take-profit closes trades early and lets the other side open. Lots ignored; one unit; the vault's standard costs. Coders Guru wrote it as a teaching EA; no test is shown.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![sdef("mt4ec", "mt4ec_ema_cross", "EMA_CROSS EA (Coders Guru): 10/80 EMA direction change opens a trade when flat and closes it on the next change (code's argument order: buys when the 10 EMA falls under the 80)",
        "EMA_CROSS.mq4 (Coders Guru, Forexmentor MetaTrader toolkit)", "Long/Short", Status::Live, &[
            "F|\"extern int ShortEma = 10; extern int LongEma = 80;\" \"SEma = iMA(NULL,0,ShortEma,0,MODE_EMA,PRICE_CLOSE,0); LEma = iMA(NULL,0,LongEma,0,MODE_EMA,PRICE_CLOSE,0);\"",
            "F|\"int Crossed (double line1 , double line2) { static int last_direction = 0; static int current_direction = 0; if(line1>line2)current_direction = 1; //up if(line1<line2)current_direction = 2; //down if(current_direction != last_direction) //changed { last_direction = current_direction; return (last_direction); } else { return (0); } }\" called as \"isCrossed = Crossed (LEma,SEma);\"",
            "E|\"if(total < 1) { if(isCrossed == 1) { ticket=OrderSend(Symbol(),OP_BUY,Lots,Ask,3,0,Ask+TakeProfit*Point,...); ... if(isCrossed == 2) { ticket=OrderSend(Symbol(),OP_SELL,Lots,Bid,3,0,Bid-TakeProfit*Point,...);\"",
            "X|\"if(OrderType()==OP_BUY) { if(isCrossed == 2) { OrderClose(...); return(0); }\" (sell: isCrossed == 1); TakeProfit=100 and TrailingStop=35 pips held"],
        NOTE)]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    reviewed(id).then_some(("mt4ec_ema_cross", "EMA_CROSS expert advisor", "EMA 10 / 80"))
}
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    reviewed(id).then(|| vec![pd("conv", "Crossed() arguments: 0 = (LEma, SEma) as printed; 1 = (SEma, LEma), the conventional reading", 0., 0., 1., 1.)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let (s, l) = (xavg(&x.c, 10), xavg(&x.c, 80));
    let (a, b) = if g(p, "conv", 0.) >= 0.5 { (s, l) } else { (l, s) };
    let (mut last, mut cur) = (0u8, 0u8);
    Some(run_ts(x, from, to, |i, st, _now, next| {
        if i + 1 < 100 { return; }
        if a[i] > b[i] { cur = 1; }
        if a[i] < b[i] { cur = 2; }
        let crossed = if cur != last { last = cur; cur } else { 0 };
        match (st.pos, crossed) {
            (0, 1) => next.push(o(A::Buy, P::Mkt, "BUY")),
            (0, 2) => next.push(o(A::Sell, P::Mkt, "SELL")),
            (1, 2) => next.push(o(A::Xl, P::Mkt, "close")),
            (-1, 1) => next.push(o(A::Xs, P::Mkt, "close")),
            _ => {}
        }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ema_cross_ea() {
        let r = super::super::nijat::tests::bars(1500);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("mt4ec_ema_cross", &x, 0, r.len() - 1, &Params::new()).unwrap();
        // literal: one side only alternates with flat until a take-profit would intervene (held), so every trade has the first trade's side
        assert!(t.len() >= 2 && t.iter().all(|z| z.dir == t[0].dir && z.legs[0].px == r[z.legs[0].i].open));
        let mut p = Params::new();
        p.insert("conv".into(), 1.);
        let u = run("mt4ec_ema_cross", &x, 0, r.len() - 1, &p).unwrap();
        assert!(u.len() >= 2 && u[0].dir == -t[0].dir);
        assert!(run("mt4ec_ema_cross", &x, 5, 5, &Params::new()).is_none());
    }
}

// Instrument admission: EMA_CROSS EA 2026-11-02 (titles lane K17).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "mt4ec" && reviewed(def.id),
    label: "A timeframe-free MetaTrader 4 forex EA with no test shown (pip take-profit and trailing stop held); runs on daily indices, ETFs and stocks, US and India, are a declared application.",
    basis: "EMA_CROSS.mq4 (Coders Guru), Forexmentor MetaTrader toolkit; independent check authors/titles-1030/K17w/k17_ref.py",
};
