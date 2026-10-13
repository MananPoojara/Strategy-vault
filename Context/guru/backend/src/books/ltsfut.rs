//! LTS-Futures (Lapis Trading Systems) systems recovered from RAR archives (research drive, read in place):
//! - Stingray (c) 2003: `T:/LTS Stingray System for TS (lts-futures.com)/Open Code.rar` -> `Open Code.txt` (Live).
//! - Breakout, Optimum, Barracuda (TS 7, LapisTs.dll): `Open_Code.rar` -> `Open_Code.txt` (intraday: NeedsData).
//! Book id "ltsfut". Helpers from jcfeed. Notes: library/ltsfut/README.md.
use super::jcfeed::{gp, Sim};
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "ltsfut";
const STG: &str = "ltsfut_stingray";

fn def(id: &'static str, name: &'static str, status: Status, rules: &'static [&'static str], notes: &'static str) -> StrategyDef {
    StrategyDef { id, name, group: "LTS-Futures (Open Code.rar)", session: 0, dir: "Long/Short", status, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None }
}

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        def(STG, "Stingray (30-bar regression forecast, % swing reversal)", Status::Live, &[
            "F|xplus = least-squares line of the last lslen (30) closes projected one bar ahead: \"b0=b1*sumxt-b2*sumx; a0=(sumx/j)-b0*tave; xplus=a0+b0*jp1\"",
            "F|\"if bopos[1] >= 0: if xplus > h_ep then h_ep=xplus, sellstop=h_ep*oneminus; if xplus <= sellstop and prc<xplus[1] then bopos=-1, l_ep=xplus, buystop=l_ep*oneplus\" (pctdn 1.425%)",
            "F|\"if bopos[1] <= 0: if xplus < l_ep then l_ep=xplus, buystop=l_ep*oneplus; if xplus >= buystop and prc>xplus[1] then bopos=1, h_ep=xplus, sellstop=h_ep*oneminus\" (pctup 1.425%)",
            "E|\"if marketposition<=0: if bopos>0 and bopos[1]<>bopos then Buy Next Bar at market; if xplus>xplus[2]*oneplus2 then bopos=1 ... Buy Next Bar at the market\" (pctjup 0.35%)",
            "E|\"if marketposition>=0: if bopos<0 and bopos[1]<>bopos then Sell Short Next Bar at market; if xplus<xplus[2]*oneminus2 then bopos=-1 ... Sell Short Next Bar at market\" (pctjdn 0.50%)",
            "X|No exit order (mark 0): an opposite entry reverses",
        ],
        "Code inputs are the defaults (lslen 30, pctup 1.425, pctdn 1.425, pctjup 0.35, pctjdn 0.50, prc close). The first two bars with 30 closes run the initialisation block (h_ep = l_ep = xplus). \
The swing state bopos changes inside the order blocks, which run only for the stated market position, so the state depends on the position (as in the code). \
Timeframe and market are not stated (Open Code.rar, LTS STINGRAY.ELA only); the rules are in percent with no intraday construct, so it is run on daily bars [Assumption]. \
Two orders on one bar fill in code order at the same open. India: instrument swap; index shorts hypothetical."),
        def("ltsfut_breakout", "LTS Breakout (intraday)", Status::NeedsData("intraday bars: orders only between strtTime 9:35 and endTime 13:20, thrust stops from the session open, MOC exit at SessionEndTime 16:15 (LapisTs.dll session functions)"),
            &["E|\"stb = Round2Fraction(OPEN + thrust1)\" at the first bar of the session (TIME = SESS1STARTTIME + BARINTERVAL)"], "TS 7 code with LapisTs.dll licence calls; intraday only."),
        def("ltsfut_optimum", "LTS Optimum (intraday)", Status::NeedsData("intraday bars: entries only between strtTime 11:00 and endTime 11:45, MOC alert/exit at 16:15, IntelliStop from LapisTs.dll from 2009"),
            &["E|Hilbert-transform trendline (alpha .33) signals inside the 11:00-11:45 window"], "TS 7 code; intraday only."),
        def("ltsfut_barracuda", "LTS Barracuda (intraday)", Status::NeedsData("intraday bars: pivots (pp, R1/S1, R2/S2) from the prior session's intraday high/low and today's open, session-time exits (SessionEndTime 16:15)"),
            &["E|\"pp = (hday + lday + OPEN) / 3; S1 = 2*pp - hday; R1 = 2*pp - lday\" recomputed at each new date on intraday bars"], "TS 7 code; intraday only."),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let v = match id { STG => "Stingray", "ltsfut_breakout" => "Breakout", "ltsfut_optimum" => "Optimum", "ltsfut_barracuda" => "Barracuda", _ => return None };
    Some(("ltsfut", "LTS-Futures systems", v))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    (id == STG).then(|| vec![p("lslen", "lslen (regression length)", 30., 5., 100., 1.), p("pctup", "pctup (% up swing)", 1.425, 0.2, 5., 0.025),
        p("pctdn", "pctdn (% down swing)", 1.425, 0.2, 5., 0.025), p("pctjup", "pctjup (% jump over 2 bars, buy)", 0.35, 0.05, 2., 0.05),
        p("pctjdn", "pctjdn (% jump over 2 bars, sell)", 0.50, 0.05, 2., 0.05)])
}

fn run_stg(x: &Ctx, from: usize, to: usize, p: &Params) -> Vec<Trade> {
    let j = (gp(p, "lslen", 30.) as usize).max(2);
    let jf = j as f64;
    let bot = jf * (jf - 1.);
    let (b1, b2, tave) = (12. / (bot * (jf + 1.)), 6. / bot, (jf + 1.) / 2.);
    let (op, om) = (1. + gp(p, "pctup", 1.425) / 100., 1. - gp(p, "pctdn", 1.425) / 100.);
    let (op2, om2) = (1. + gp(p, "pctjup", 0.35) / 100., 1. - gp(p, "pctjdn", 0.50) / 100.);
    let n = x.c.len();
    let mut xp = vec![f64::NAN; n];
    for i in j - 1..n {
        let (mut sx, mut sxt) = (0., 0.);
        for ii in 1..=j { let v = x.c[i + 1 - ii]; sx += v; sxt += v * (j - ii + 1) as f64; }
        let b0 = b1 * sxt - b2 * sx;
        xp[i] = sx / jf - b0 * tave + b0 * (jf + 1.);
    }
    let f = j - 1; // currentbar 1
    let (mut h_ep, mut l_ep, mut buystop, mut sellstop, mut bopos) = (0f64, 0f64, 0f64, 0f64, 0i8);
    let mut sim = Sim::new();
    for i in f..to.min(n - 1) {
        if i < f + 2 {
            h_ep = xp[i]; l_ep = xp[i]; buystop = l_ep * op; sellstop = h_ep * om;
            continue;
        }
        let bp1 = bopos;
        let c = x.c[i];
        if bp1 >= 0 {
            if xp[i] > h_ep { h_ep = xp[i]; sellstop = h_ep * om; }
            if xp[i] <= sellstop && c < xp[i - 1] { bopos = -1; l_ep = xp[i]; buystop = l_ep * op; }
        }
        if bp1 <= 0 {
            if xp[i] < l_ep { l_ep = xp[i]; buystop = l_ep * op; }
            if xp[i] >= buystop && c > xp[i - 1] { bopos = 1; h_ep = xp[i]; sellstop = h_ep * om; }
        }
        let mp = sim.pos();
        let mut ords: Vec<i8> = vec![];
        if mp <= 0 {
            if bopos > 0 && bp1 != bopos { ords.push(1); }
            if xp[i] > xp[i - 2] * op2 { bopos = 1; h_ep = xp[i]; sellstop = h_ep * om; ords.push(1); }
        }
        if mp >= 0 {
            if bopos < 0 && bp1 != bopos { ords.push(-1); }
            if xp[i] < xp[i - 2] * om2 { bopos = -1; l_ep = xp[i]; buystop = l_ep * op; ords.push(-1); }
        }
        if i >= from {
            for d in ords { sim.enter(d, i + 1, x.o[i + 1], i, "open", if d > 0 { "Buy" } else { "Sell Short" }); }
        }
    }
    sim.finish(x, to)
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if id != STG || from >= to || to >= x.r.len() { return None; }
    Some(run_stg(x, from, to, p))
}
