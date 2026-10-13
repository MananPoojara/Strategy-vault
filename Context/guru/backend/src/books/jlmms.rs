//! "Livermore Market Method Signal" (G. H. Kramer, v1.1 11-Oct-2003, after J. L. Livermore "How to Trade in Stocks" ch. VIII-IX),
//! TradeStation 2000i strategy distributed by stockmarketsolutions.org. Source (research drive, read in place, read-only):
//! `USB HDD 1/TradeStation Systems & Indicators/Jesse Livermore Market Method Signal for TS (stockmarketsolutions.org)/Other Version/
//! Other Version.rar` -> `Signal.txt` + `Signal_Readme.txt` (extracted as data). Book id `jlmms`. Notes: library/jlmms/README.md.
//! Engine: books::rrsim.
use crate::books::rrsim::{g, o, pd, run_ts, sdef, sma, true_range, A, P};
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "jlmms";
const ID: &str = "jlmms_signal";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![sdef(BOOK, ID, "Livermore Market Method signal (6-point threshold, trade trends only)", "Livermore Market Method (Kramer, TS 2000i)", "Long/Short", Status::Live, &[
        "F|Inputs PtsPctATR(0), Threshold(6), ATRLength(14), TradeTrends(1); \"Livermore's system used a threshold of 6 points for stocks priced over $30. This is the default (PtsPctATR = 0, Threshold = 6).\" (Signal.txt)",
        "F|Init: CurrentBar 1: Thresh = Threshold, HalfThresh = Thresh/2 (points mode); SecondaryRally = NaturalRally = UpTrend = SecondaryReaction = NaturalReaction = DnTrend = Close. CurrentBar <= 21: MA10 = Average(Close,10); at CurrentBar 21: if MA10 > MA10[10] then InUpTrend, UpTrend = C else InDnTrend, DnTrend = C",
        "F|Main (CurrentBar > 21): six states Up Trend / Natural Rally / Secondary Rally / Down Trend / Natural Reaction / Secondary Reaction with the code's transitions on the close (Rules 4a-6h, 8, 9b-c): e.g. Up Trend -> Natural Reaction when Close < UpTrend - Thresh (UpTrendRL = UpTrend); Natural Rally -> Up Trend when Close > UpTrend or Close > NaturalRallyBL + HalfThresh; ... (full code in library/jlmms/README.md); 'Close > NaturalReaction + Threshold' uses the input Threshold as printed",
        "E|TS 2000i trading logic, TradeTrends = 1: \"if InUpTrend then Buy next bar at open ... if InDnTrend then Sell next bar at open\" (Sell = sell short in TS 2000i; an entry reverses the opposite position)",
        "X|\"... else if (Marketposition = 1) then ExitLong next bar at open; ... else if (Marketposition = -1) then ExitShort next bar at open\" (out when the state is no longer Up Trend / Down Trend)",
        "E|TradeTrends = 0 (non-default input): \"if (InUpTrend or InNatRally or InSecRally) then Buy next bar at open; ExitShort next bar at open; if (InDnTrend or InNatReact or InSecReact) then Sell next bar at open; ExitLong next bar at open\"",
        "F|UseRule10 = false and PtsPctATR = 0 are the code's defaults (Readme: 'the system becomes unstable when rule 10 is used'); PtsPctATR 1 = Threshold percent of Close[1], 2 = Threshold x AvgTrueRange(14) are non-default inputs",
    ], "Code literal. The threshold is in price points (6 points, meant for stocks over $30); on an index it is the same 6 points (instrument swap, \
no rescaling; the code's percent / ATR modes are non-default inputs). CurrentBar 1 = the first bar on which Average(Close,10) exists (parameter mbb, \
TradeStation MaxBarsBack; the state machine is path dependent, so a different start can change early states). Orders fill at the next open; an entry \
reverses an opposite position; no pyramiding. The Readme calls it 'not a turnkey strategy ... for educational purposes'. India: instrument swap; \
index shorts hypothetical.")]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == ID).then_some(("jlmms", "Livermore Market Method (Kramer)", "TS 2000i signal"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    (id == ID).then(|| vec![
        pd("threshold", "Threshold (points; percent if mode 1; ATR multiples if mode 2)", 6., 0.5, 50., 0.5),
        pd("mode", "PtsPctATR: 0 points, 1 percent of Close[1], 2 x AvgTrueRange(14)", 0., 0., 2., 1.),
        pd("trade_trends", "TradeTrends: 1 trade Up/Down Trends only, 0 also rallies/reactions", 1., 0., 1., 1.),
        pd("rule10", "UseRule10 (0 = off as in the code)", 0., 0., 1., 1.),
        pd("mbb", "Bars before CurrentBar 1 (MaxBarsBack)", 9., 9., 100., 1.),
    ])
}

#[derive(Default)]
struct S {
    sec_rally: f64, nat_rally: f64, up: f64, sec_react: f64, nat_react: f64, dn: f64,
    dn_bl: f64, nat_rally_bl: f64, up_rl: f64, nat_react_rl: f64,
    in_sr: bool, in_nr: bool, in_up: bool, in_sx: bool, in_nx: bool, in_dn: bool,
    res_up: bool, res_dn: bool, thresh: f64, half: f64,
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if id != ID || from >= to || to >= x.r.len() { return None; }
    let threshold = g(p, "threshold", 6.);
    let mode = g(p, "mode", 0.).round() as i32;
    let trends = g(p, "trade_trends", 1.) >= 0.5;
    let rule10 = g(p, "rule10", 0.) >= 0.5;
    let mbb = g(p, "mbb", 9.).max(9.) as usize;
    let c = &x.c;
    let ma = sma(c, 10);
    let atr14 = sma(&true_range(x), 14);
    let mut s = S::default();
    let mut ma10 = vec![0f64; c.len()];
    Some(run_ts(x, from, to, |i, st, _now, next| {
        if i < mbb { return; }
        let cb = i - mbb + 1;
        let cl = c[i];
        if cb == 1 {
            if mode == 0 { s.thresh = threshold; s.half = s.thresh / 2.; }
            s.sec_rally = cl; s.nat_rally = cl; s.up = cl; s.sec_react = cl; s.nat_react = cl; s.dn = cl;
        }
        if cb <= 21 {
            ma10[i] = ma[i];
            if cb == 21 {
                if ma10[i] > ma10[i - 10] { s.in_up = true; s.up = cl; } else { s.in_dn = true; s.dn = cl; }
            }
        } else {
            if mode == 1 { s.thresh = threshold * c[i - 1] / 100.; s.half = s.thresh / 2.; }
            else if mode == 2 { s.thresh = threshold * atr14[i]; s.half = s.thresh / 2.; }
            if s.in_up {
                if cl > s.nat_react + threshold { s.nat_react_rl = s.nat_react; }
                if s.res_up {
                    if cl > s.up_rl + s.half { s.res_up = false; s.up = cl; }
                    else if cl < s.up_rl - s.half { s.res_up = false; s.in_up = false; s.in_nx = true; s.nat_react = cl; }
                } else if cl < s.up - s.thresh {
                    s.in_up = false; s.in_nx = true; s.nat_react = cl; s.up_rl = s.up; s.res_up = false;
                } else if cl > s.up { s.up = cl; }
            } else if s.in_nr {
                if cl > s.nat_react + threshold { s.nat_react_rl = s.nat_react; }
                if cl > s.up { s.in_up = true; s.in_nr = false; s.up = cl; if rule10 { s.res_up = true; } }
                else if cl > s.nat_rally_bl + s.half { s.in_up = true; s.in_nr = false; s.up = cl; if rule10 { s.res_up = true; } }
                else if cl < s.dn { s.in_nr = false; s.in_dn = true; s.dn = cl; s.nat_rally_bl = cl; }
                else if cl < s.nat_rally - s.thresh {
                    if cl < s.nat_react { s.in_nr = false; s.in_nx = true; s.nat_react = cl; s.nat_rally_bl = cl; }
                    else { s.in_nr = false; s.in_sx = true; s.sec_react = cl; }
                } else if cl > s.nat_rally { s.nat_rally = cl; }
            } else if s.in_sr {
                if cl > s.up { s.in_up = true; s.in_sr = false; s.up = cl; if rule10 { s.res_up = true; } }
                else if cl > s.nat_rally_bl + s.half { s.in_up = true; s.in_sr = false; s.up = cl; if rule10 { s.res_up = true; } }
                // literal: "InSecReact = false; InNatRally = true" — InSecRally is not cleared here (Node 11)
                else if cl > s.nat_rally { s.in_sx = false; s.in_nr = true; s.nat_rally = cl; }
                else if cl < s.dn { s.in_sr = false; s.in_dn = true; s.dn = cl; s.nat_rally_bl = cl; }
                else if cl > s.sec_rally { s.sec_rally = cl; }
            } else if s.in_dn {
                if cl < s.nat_rally - threshold { s.nat_rally_bl = s.nat_rally; }
                if s.res_dn {
                    if cl < s.dn_bl - s.half { s.res_dn = false; s.dn = cl; }
                    else if cl > s.dn_bl + s.half { s.res_dn = false; s.in_dn = false; s.in_nr = true; s.nat_rally = cl; }
                } else if cl > s.dn + s.thresh {
                    s.in_dn = false; s.in_nr = true; s.nat_rally = cl; s.dn_bl = s.dn; s.res_dn = false;
                } else if cl < s.dn { s.dn = cl; }
            } else if s.in_nx {
                if cl < s.nat_rally - threshold { s.nat_rally_bl = s.nat_rally; }
                if cl < s.dn { s.in_dn = true; s.in_nx = false; s.dn = cl; if rule10 { s.res_dn = true; } }
                else if cl < s.nat_react_rl - s.half { s.in_dn = true; s.in_nx = false; s.dn = cl; if rule10 { s.res_dn = true; } }
                else if cl > s.up { s.in_nx = false; s.in_up = true; s.up = cl; s.nat_react_rl = cl; }
                else if cl > s.nat_react + s.thresh {
                    if cl > s.nat_rally { s.in_nx = false; s.in_nr = true; s.nat_rally = cl; s.nat_react_rl = cl; }
                    else { s.in_nx = false; s.in_sr = true; s.sec_rally = cl; }
                } else if cl < s.nat_react { s.nat_react = cl; }
            } else if s.in_sx {
                if cl < s.dn { s.in_dn = true; s.in_sx = false; s.dn = cl; if rule10 { s.res_dn = true; } }
                else if cl < s.nat_react_rl - s.half { s.in_dn = true; s.in_sx = false; s.dn = cl; if rule10 { s.res_dn = true; } }
                else if cl > s.up { s.in_sx = false; s.in_up = true; s.up = cl; s.nat_react_rl = cl; }
                else if cl < s.nat_react { s.in_sx = false; s.in_nx = true; s.nat_react = cl; }
                else if cl < s.sec_react { s.sec_react = cl; }
            }
        }
        if trends {
            if s.in_up { next.push(o(A::Buy, P::Mkt, "Up Trend")); } else if st.pos == 1 { next.push(o(A::Xl, P::Mkt, "ExitLong")); }
            if s.in_dn { next.push(o(A::Sell, P::Mkt, "Down Trend")); } else if st.pos == -1 { next.push(o(A::Xs, P::Mkt, "ExitShort")); }
        } else {
            if s.in_up || s.in_nr || s.in_sr { next.push(o(A::Buy, P::Mkt, "Up/Rally")); next.push(o(A::Xs, P::Mkt, "ExitShort")); }
            if s.in_dn || s.in_nx || s.in_sx { next.push(o(A::Sell, P::Mkt, "Down/Reaction")); next.push(o(A::Xl, P::Mkt, "ExitLong")); }
        }
    }))
}
