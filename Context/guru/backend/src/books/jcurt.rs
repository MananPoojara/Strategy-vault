//! J. Curtis package of TradeStation systems (JK-20 .. JK-31): `T:/J.Curtis Package of Systems for TS/Codes.rar` -> `Codes.txt`.
//! `T:` = research drive `USB HDD 1/TradeStation Systems & Indicators`. Book id "jcurt". Notes: library/jcurt/README.md.
//! Each system body is repeated under two password values (JK1 = 012279 / 053152) with identical code.
use crate::books::ajmisc::{back, first_ok, highest, lowest, ord, Sim, Ty, K, POINT};
use crate::connors::{sma, Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "jcurt";
/// Admitted ids (authors/vendor/laneb/SOURCE-CONTRACT.md).
pub const REVIEWED: &[&str] = &["jcurt_jk24_hl_bands", "jcurt_jk31_pattern"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }
const J24: &str = "jcurt_jk24_hl_bands";
const J31: &str = "jcurt_jk31_pattern";

const FUT: &str = "The system is named for one futures market; the vault has no futures contracts (vault convention: non-equity futures systems are NeedsData).";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, status, rules, notes| StrategyDef { id, name, group: "J.Curtis package (Codes.txt)", session: 0, dir: "Long/Short", status, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None };
    use Status::*;
    vec![
        d(J24, "JK-24 (listed as 'JK-28-SC-DualBB'): low above the 45-bar MA of highs / high below the 45-bar MA of lows", Live, &[
            "E|\"IF L > Average(H,Input2) Then Buy Next Bar at Market\" (Input2 45)",
            "E|\"IF H < Average(L,Input1) Then Sell Short Next Bar at Market\" (Input1 45)",
            "X|Stop and reverse at the next open; no other exit in the code."],
            "The block is headed 'JK-28-SC-DualBB' but prints to C:\\pa\\24 and has no data2 (the second 'JK-28-SC-DualBB' block does), so it is taken as JK-24; no market is named. The same rule with other lengths is JK-23 (JY 65/50), JK-25 (DM 45/60), JK-26 (BP 45/45). \
Daily cash indices (instrument swap). Fills: market orders at the next open. India: instrument swap; index shorts hypothetical."),
        d(J31, "JK-31-SC-PatBB: three falling (rising) bars -> stop beyond the 2-bar open extreme", Live, &[
            "E|\"IF H<H[1] and H[1]<H[2] and L<L[1] and L[1]<L[2] Then Buy Next Bar Highest(o,2)+1 point Stop\"",
            "E|\"IF H>H[1] and H[1]>H[2] and L>L[1] and L[1]>L[2] Then Sell Short Next Bar Lowest(o,2)- 1 point Stop\"",
            "X|Stop and reverse; no other exit in the code."],
            "1 point = 1/PriceScale = 0.01. No market named. Daily cash indices (instrument swap). Fills: vault TradeStation daily-bar convention (ajmisc.rs header). India: instrument swap; index shorts hypothetical."),
        d("jcurt_jk20_usb", "JK-20-SC-USBB (US T-Bonds): close up -> buy C + 0.9 range stop, 5-bar exit", NeedsData("US T-Bond futures (the block is named USBB); the vault has no bond futures"), &[
            "E|\"IF C > C[1] Then Buy Next Bar C + (Range*0.9) Stop\"; \"IF C < C[1] Then Sell Short Next Bar C-(Range*1.8) Stop\"",
            "X|\"IF MarketPosition=1 and BarsSinceEntry>=VarA-1 Then Sell Next Bar at Market\" (VarA 5); short mirror"], FUT),
        d("jcurt_jk21_sp_dt", "JK-21-SC-SP-DTBB (S&P day trade): close in lower 5/6 and up -> buy stop at the high", NotTestable("named a day-trade (DT) system but the code has no exit except the opposite entry; the day-trade exit is outside the code (its DT sibling JK-27 has no exit at all)"), &[
            "E|\"IF C < H-(Range/6) and C > C[1] Then Buy Next Bar h Stop\"; \"IF C > L+(Range/6) and C < C[1] Then Sell Short Next Bar l Stop\""], ""),
        d("jcurt_jk22_index", "JK-22-SC-INDEXBB: Friday close below its 3-bar MA -> buy", NotTestable("no exit in the code"), &[
            "E|\"IF DayOfWeek(Date)=5 and C < Average(C,3) Then Buy Next Bar at Market\""], ""),
        d("jcurt_jk23_jy", "JK-23-SC-JY-LTBB (Japanese yen): L > MA(H,50) buy, H < MA(L,65) sell short", NeedsData("Japanese yen futures (block named JY)"), &["E|\"IF L > Average(H,Input2) Then Buy\" (50); \"IF H<Average(L,Input1) Then Sell Short\" (65)"], FUT),
        d("jcurt_jk25_dm", "JK-25-SC-DM-LTBB (Deutsche mark): L > MA(H,60) buy, H < MA(L,45) sell short", NeedsData("Deutsche mark futures (block named DM)"), &["E|\"IF L > Average(H,Input2) Then Buy\" (60); \"IF H<Average(L,Input1) Then Sell Short\" (45)"], FUT),
        d("jcurt_jk26_bp", "JK-26-SC-BP-LTBB (British pound): L > MA(H,45) buy, H < MA(L,45) sell short", NeedsData("British pound futures (block named BP)"), &["E|\"IF L > Average(H,Input2) Then Buy\" (45); \"IF H<Average(L,Input1) Then Sell Short\" (45)"], FUT),
        d("jcurt_jk27_nyse_dt", "JK-27-SC-NYSE-DTBB: day-of-week x 3-bar MA buys", NotTestable("no exit in the code (day-trade exit not stated)"), &[
            "E|\"IF (DayOfWeek(date)=VarA and C < Average(C,3)) OR (DayOfWeek(date)=VarB and C > Average(C,3)) OR (DayOfWeek(Date)=VarC and C > Average(C,3)) Then Buy Next Bar at Market\" (5/4/3)"], ""),
        d("jcurt_jk28_dual", "JK-28-SC-DualBB: data2 close vs its 9-bar MA", NotTestable("trades on data2, a second market the code never names"), &[
            "E|\"IF C of data2 < Average(C of data2,VarA) Then Buy Next Bar at Market; IF C of data2 > Average(C of data2,VarA) Then Sell Short Next Bar at Market\" (VarA 9)"], ""),
        d("jcurt_jk29_bp_lw19", "JK-29-SC-BP-LW-19BB (British pound): pivot-channel stops", NeedsData("British pound futures (block named BP)"), &[
            "E|\"Buy Next Bar Highest(PH,Length)+1 point Stop; Sell Next Bar Lowest(L,Length2) Stop; Sell Short Next Bar Lowest(PL,Length)- 1 point Stop; Buy to Cover Next Bar Highest(H,Length2) Stop\" (28/28)"], FUT),
        d("jcurt_jk30_dual", "JK-30-SC-DualBB: data2 9-bar channel close breakout", NotTestable("trades on data2, a second market the code never names"), &[
            "E|\"IF (C of data2 > Highest(High of data2,Length)[1]) and (C[1] of data2 < Highest(High of data2,Length)[2]) Then Buy Next Bar at Market\"; mirror 'Sell Next Bar at Market' (exit)"], ""),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    id.starts_with("jcurt_").then(|| (match id { J24 => J24, J31 => J31, _ => "jcurt_other" }, "J.Curtis package", "code"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    match id { J24 => Some(vec![p("input1", "Input1 (MA of lows)", 45., 5., 150., 1.), p("input2", "Input2 (MA of highs)", 45., 5., 150., 1.)]), _ => None }
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if from >= to || to >= x.r.len() { return None; }
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d);
    let mut s = Sim::new();
    match id {
        J24 => {
            let (ml, mh) = (sma(&x.l, g("input1", 45.).max(1.) as usize), sma(&x.h, g("input2", 45.).max(1.) as usize));
            for i in first_ok(x.c.len(), from, &[&ml, &mh])..to {
                let mut q = vec![];
                if x.l[i] > mh[i] { q.push(ord(K::Buy, Ty::Mkt, "L > Average(H,45)")); }
                if x.h[i] < ml[i] { q.push(ord(K::Short, Ty::Mkt, "H < Average(L,45)")); }
                s.bar(x, i + 1, &q);
            }
        }
        J31 => {
            let (ho, lo) = (highest(&x.o, 2), lowest(&x.o, 2));
            for i in first_ok(x.c.len(), from, &[&back(&x.c, 2), &ho])..to {
                let (h, l) = (&x.h, &x.l);
                let mut q = vec![];
                if h[i] < h[i - 1] && h[i - 1] < h[i - 2] && l[i] < l[i - 1] && l[i - 1] < l[i - 2] { q.push(ord(K::Buy, Ty::Stop(ho[i] + POINT), "Highest(o,2)+1 point")); }
                if h[i] > h[i - 1] && h[i - 1] > h[i - 2] && l[i] > l[i - 1] && l[i - 1] > l[i - 2] { q.push(ord(K::Short, Ty::Stop(lo[i] - POINT), "Lowest(o,2)-1 point")); }
                s.bar(x, i + 1, &q);
            }
        }
        _ => return None,
    }
    Some(s.done(x, to))
}

// Instrument admission: J. Curtis systems 2026-10-26 (lane B) block, moved from instrument_policy.rs on 31 Oct 2026 (lane ST).
// Admission only for the module REVIEWED lists (authors/vendor/laneb/SOURCE-CONTRACT.md); kept in one block to merge.
// Runs on actual indices, ETFs and stocks, US and India (index_etf_or_stock, nifty50 basket, declared application).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book=="jcurt" && crate::books::jcurt::reviewed(def.id),
    label: "Vendor and newsletter code with no per-market test the vault can compare; runs on indices, ETFs and single stocks (US and India) are a declared application, one unit per trade.",
    basis: "J. Curtis TradeStation system code (JK-24, JK-31)",
};
