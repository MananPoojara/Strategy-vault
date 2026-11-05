//! "Moving Average" sample expert advisor (MetaQuotes Software Corp., 2005), the MQL4 sample shipped with MetaTrader 4. Research
//! share USB HDD 1/META TRADER/MetaTrader/BuruTrader (burutrader.com)/Original Files/IBFX-AU-BURU.exe -> experts/Moving Average.mq4
//! (copy authors/batch-x/keep/501/experts_Moving_Average.mq4.txt, md5 e7686fd3). Book id `mqma`. Lane K30 (titles loop, 3 Nov 2026),
//! to-code row 370, lead decisions 67, 77, 181; independent check authors/titles-1030/K30w/k30_ref.py.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};
use super::rrsim::{g, pd, sdef};

pub const REVIEWED: &[&str] = &["mqma_moving_average"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] The MQL4 sample, read literally and run on daily bars (lead decisions 67, 77: a timeframe-free vendor sample with a complete rule). MA = iMA(NULL,0,MovingPeriod 12,MovingShift 6,MODE_SMA,PRICE_CLOSE,0): on the new bar, the 12-bar simple average of the close ending 6 bars before it (the line shifted 6 bars right). The EA acts only on the first tick of a new bar (Volume[0] > 1 returns), so each decision uses the bar just closed (Open[1], Close[1]) and fills at the new bar's open. Flat: sell short when that bar opened above MA and closed below it; else buy when it opened below MA and closed above it. Long: close when it opened above MA and closed below it; short: close when it opened below and closed above. start() runs CheckForClose while a position exists and CheckForOpen only when flat, and later ticks of the bar return, so an exit and a new entry never fall on the same bar: after an exit a new entry needs a later crossing bar. No stop, no target. Needs Bars >= 100 (the new bar is at least the 100th). LotsOptimized (MaximumRisk 0.02, DecreaseFactor 3 after losses) is position sizing, not applied: one unit; Bid/Ask spread not modelled; the vault's standard costs. MetaQuotes shows no test.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![sdef("mqma", "mqma_moving_average", "MetaQuotes Moving Average sample EA: a bar that opens and closes on opposite sides of the 12-bar SMA shifted 6 bars opens a trade in the close's direction at the next open, or closes the trade against it",
        "Moving Average.mq4 (MetaQuotes sample EA, MetaTrader 4)", "Long/Short", Status::Live, &[
            r##"F|"extern double MovingPeriod = 12; extern double MovingShift = 6;" "ma=iMA(NULL,0,MovingPeriod,MovingShift,MODE_SMA,PRICE_CLOSE,0);" "if(Volume[0]>1) return;""##,
            r##"E|CheckForOpen: "if(Open[1]>ma && Close[1]<ma) { res=OrderSend(Symbol(),OP_SELL,LotsOptimized(),Bid,3,0,0,"",MAGICMA,0,Red); return; } if(Open[1]<ma && Close[1]>ma) { res=OrderSend(Symbol(),OP_BUY,LotsOptimized(),Ask,3,0,0,"",MAGICMA,0,Blue); return; }""##,
            r##"X|CheckForClose: "if(OrderType()==OP_BUY) { if(Open[1]>ma && Close[1]<ma) OrderClose(OrderTicket(),OrderLots(),Bid,3,White); break; } if(OrderType()==OP_SELL) { if(Open[1]<ma && Close[1]>ma) OrderClose(OrderTicket(),OrderLots(),Ask,3,White); break; }""##,
            r##"F|start(): "if(Bars<100 || IsTradeAllowed()==false) return; if(CalculateCurrentOrders(Symbol())==0) CheckForOpen(); else CheckForClose();""##],
        NOTE)]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    reviewed(id).then_some(("mqma_moving_average", "MetaQuotes Moving Average sample EA", "SMA 12, shift 6"))
}
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    reviewed(id).then(|| vec![pd("period", "MovingPeriod (SMA bars)", 12., 2., 100., 1.), pd("shift", "MovingShift (bars the average is shifted right)", 6., 0., 30., 1.)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let per = g(p, "period", 12.).round().max(1.) as usize;
    let sh = g(p, "shift", 6.).round().max(0.) as usize;
    let m = crate::books::msws::sma(&x.c, per);
    let (mut out, mut cur): (Vec<Trade>, Option<Trade>) = (vec![], None);
    for i in from..to {
        // i = the bar just closed (Open[1], Close[1]); the EA acts on the first tick of bar i + 1 (Bars = i + 2)
        if i + 2 < 100 || i + 1 < sh { continue; }
        let ma = m[i + 1 - sh];
        if !ma.is_finite() { continue; }
        let (sell, buy) = (x.o[i] > ma && x.c[i] < ma, x.o[i] < ma && x.c[i] > ma);
        match cur.as_ref().map(|t| t.dir) {
            None if sell || buy => cur = Some(Trade { dir: if sell { -1. } else { 1. }, legs: vec![Leg { i: i + 1, px: x.o[i + 1], w: 1. }], exit_i: i + 1, exit_px: x.o[i + 1],
                reason: String::new(), entry_kind: "open", signal_i: i, open: false, exits: vec![] }),
            Some(d) if (d > 0. && sell) || (d < 0. && buy) => {
                let mut t = cur.take().unwrap();
                (t.exit_i, t.exit_px, t.signal_i) = (i + 1, x.o[i + 1], i);
                t.reason = "bar crossed the shifted MA · next open".into();
                out.push(t);
            }
            _ => {}
        }
    }
    if let Some(mut t) = cur { (t.exit_i, t.exit_px, t.signal_i, t.open) = (to, x.c[to], to, true); t.reason = "open".into(); out.push(t); }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn moving_average_sample_ea() {
        let r = super::super::nijat::tests::bars(1500);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("mqma_moving_average", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() >= 4 && t.iter().any(|z| z.dir > 0.) && t.iter().any(|z| z.dir < 0.));
        let m = crate::books::msws::sma(&x.c, 12);
        for z in &t {
            let s = z.legs[0].i - 1;
            assert!(s + 2 >= 100 && z.legs[0].px == r[z.legs[0].i].open);
            let ma = m[s - 5];
            assert!(if z.dir > 0. { x.o[s] < ma && x.c[s] > ma } else { x.o[s] > ma && x.c[s] < ma });
        }
        // no exit and entry on one bar
        for w in t.windows(2) { assert!(w[1].legs[0].i > w[0].exit_i); }
        assert!(run("mqma_moving_average", &x, 5, 5, &Params::new()).is_none());
    }
}

// Instrument admission: MetaQuotes Moving Average sample EA 2026-11-03 (titles lane K30).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "mqma" && reviewed(def.id),
    label: "MetaQuotes' timeframe-free MetaTrader 4 sample EA, with no market or test shown; runs on daily indices, ETFs and stocks, US and India, are a declared application.",
    basis: "Moving Average.mq4 (MetaQuotes Software Corp., 2005), MetaTrader 4 experts folder in the IBFX installer (research share USB HDD 1/META TRADER/MetaTrader/BuruTrader); independent check authors/titles-1030/K30w/k30_ref.py",
};
