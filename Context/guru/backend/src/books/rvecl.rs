//! Ralph Vince, Eclipse-ER2 for Russell 2000 (New Moon Soon), recovered from
//! `T:/Ralph Vince - Eclipse-ER2 for Russell 2000 (New Moon Soon)/Open_Code.rar` -> `Open_Code.txt` (research drive, read in place).
//! Book id "rvecl". Helpers from jcfeed. Notes: library/rvecl/README.md.
use super::jcfeed::{ext, gp, Sim};
use crate::connors::{sma, Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "rvecl";
/// Admitted ids (authors/lanea/vendor1/decisions.json).
pub const REVIEWED_LANEA: &[&str] = &["rvecl_eclipse"];
pub fn reviewed_lanea(id: &str) -> bool { REVIEWED_LANEA.contains(&id) }
const EC: &str = "rvecl_eclipse";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: EC, name: "Eclipse (8/50-day moving averages, 21-day extreme exit)", group: "Eclipse-ER2 (Open_Code.rar)", session: 0, dir: "Long/Short",
        status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules: &[
            "F|\"VALUE1=AVERAGE(PRICE,MA1); VALUE2=AVERAGE(PRICE,MA1)[1]; VALUE3=AVERAGE(PRICE,MA2); VALUE4=AVERAGE(PRICE,MA2)[1]\" (MA1 8, MA2 50, PRICE close)",
            "E|\"IF CONDITION1 AND CONDITION3 AND CONDITION5 THEN Buy This Bar C\" (VALUE1>=VALUE2, VALUE3>=VALUE4, VALUE1>=VALUE3)",
            "E|\"IF CONDITION2 AND CONDITION4 AND CONDITION6 THEN Sell Short This Bar C\" (VALUE1<=VALUE2, VALUE3<=VALUE4, VALUE1<=VALUE3)",
            "X|\"IF MARKETPOSITION=1: IF L<=LOWEST(LOW,21) THEN Sell This Bar AT C; IF CONDITION6 THEN Sell This Bar AT C\"",
            "X|\"IF MARKETPOSITION=-1: IF H>=HIGHEST(HIGH,21) THEN Buy to Cover This Bar AT C; IF CONDITION5 THEN Buy to Cover This Bar AT C\"",
        ],
        notes: "Code inputs MA1 8 and MA2 50 are the defaults; LOWEST(LOW,21) includes today, so the exit fires when today's low is the 21-day low. \
Order handling [TradeStation code order, all at the close]: MARKETPOSITION is the position before today's close orders; the entries are applied first (an entry in the held direction is ignored, an opposite one reverses), then the exits for the position held before the close. \
Built for Russell 2000 futures (ER2); no RUT/IWM-futures results are printed in the archive. Timeframe not stated; the code has no intraday construct and is run on daily bars [Assumption]. \
The $mgrdata function only writes equity files (send$mgr 0). India: instrument swap; index shorts hypothetical.",
        claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == EC).then_some((EC, "Eclipse (Ralph Vince)", "code literal"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    (id == EC).then(|| vec![p("ma1", "MA1 (fast)", 8., 2., 30., 1.), p("ma2", "MA2 (slow)", 50., 10., 200., 5.), p("xlen", "Exit extreme length", 21., 5., 60., 1.)])
}

fn run_ec(x: &Ctx, from: usize, to: usize, p: &Params) -> Vec<Trade> {
    let (f, s, xl) = (sma(&x.c, gp(p, "ma1", 8.).max(1.) as usize), sma(&x.c, gp(p, "ma2", 50.).max(1.) as usize), (gp(p, "xlen", 21.) as usize).max(1));
    let mut sim = Sim::new();
    for i in from.max(1)..=to {
        if [f[i], f[i - 1], s[i], s[i - 1]].iter().any(|v| v.is_nan()) || i + 1 < xl { continue; }
        let (c1, c2, c3, c4, c5, c6) = (f[i] >= f[i - 1], f[i] <= f[i - 1], s[i] >= s[i - 1], s[i] <= s[i - 1], f[i] >= s[i], f[i] <= s[i]);
        let mp = sim.pos();
        let c = x.c[i];
        if c1 && c3 && c5 { sim.enter(1, i, c, i, "close", "Buy"); }
        if c2 && c4 && c6 { sim.enter(-1, i, c, i, "close", "Sell Short"); }
        if mp == 1 {
            if x.l[i] <= ext(&x.l, i, xl, false) { sim.exit(1, i, c, i, "LCEXIT (21-day low)"); }
            if c6 { sim.exit(1, i, c, i, "fast MA <= slow MA"); }
        }
        if mp == -1 {
            if x.h[i] >= ext(&x.h, i, xl, true) { sim.exit(-1, i, c, i, "SCEXIT (21-day high)"); }
            if c5 { sim.exit(-1, i, c, i, "fast MA >= slow MA"); }
        }
    }
    sim.finish(x, to)
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if id != EC || from >= to || to >= x.r.len() { return None; }
    Some(run_ec(x, from, to, p))
}
