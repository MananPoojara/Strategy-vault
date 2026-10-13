//! Mark Crisp, "The Ultimate Swing Trading Indicator" (STIS.pdf + MetaStock expert EXPT0001.DTA), MS:/Mark Crisp - Swing
//! Trading Indicator & System for Metastock (swingtradingindicator.com)/; copies in Books/Mark Crisp/. Book id "mcstis".
//! Notes: library/mcstis/README.md. Crisp's other two titles are module mcmts.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "mcstis_arrows", name: "Swing Trading Indicator arrows + 20 DEMA filter, 12% stop, ringed-low exit",
        group: "Swing Trading Indicator (MetaStock)", session: 0, dir: "Long/Short",
        status: Status::NotTestable("the arrow formula (recovered from EXPT0001.DTA) is built on MetaStock Zig/Peak/Trough/PeakBars, whose value on a bar changes with later bars; the source does not say how a pivot is confirmed at the last bar, so a no-look-ahead arrow cannot be computed without inventing that rule. 'Fraction above/below' is 0.1 for stocks but 'three ticks' for commodities (no index equivalent)"),
        needs_volume: false, needs_vix: false, needs_series: &[], book: "mcstis",
        rules: &["E|Arrow (EXPT0001.DTA 'buy signal'): lb:=TroughBars(1,L,.1); hb:=PeakBars(1,H,.1); plot:=Zig(If(lb<hb,L,H),1,%); new:=PeakBars(1,plot,1)=0 OR TroughBars(1,plot,1)=0; bc:=If(new,Cross(H,Peak(2,plot,1)),Cross(H,Peak(1,plot,1))); sc:=If(new,Cross(Trough(2,plot,1),L),Cross(Trough(1,plot,1),L)); bc AND Ref(BarsSince(bc)>BarsSince(sc),-1)",
            "F|\"we only class buy arrows valid IF the price bar's high is above the 20 d.e.m.a and sell arrows are only valid IF the occur BELOW the 20 d.e.m.a\" (STIS p.3)",
            "E|\"place your buy stop order a fraction above that price bar. We are talking about 0.1 above the high for stocks and three ticks for commodities ... keep the order in for one more day. If you are not filled on your position on the second day then cancel\" (p.3-4)",
            "X|\"When filled place a catastrophic stop loss (+/- 12% from your fill)\" (p.5)",
            "X|\"Wait for a reversal arrow to be shown on the chart. Then place your stop a fraction below this arrow\"; move it only if a later exit arrow protects more profit (p.5-7). Exits do not use the 20 DEMA.",
            "X|\"A three day ringed low is simply a low bar which is not exceeded for AT LEAST three days\" (chart caption p.9); \"place your exit order just below this three-day low bar\" (p.9). \"No one exit has priority over the other ... apply ALL three exits\" (p.9)"],
        notes: "Short = rules reversed (p.9). No published results.", claim: None }]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == "mcstis_arrows").then_some(("mcstis", "Crisp Swing Trading Indicator", "Arrows + 20 DEMA"))
}
pub fn params(_id: &str) -> Option<Vec<ParamDef>> { None }
pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }
