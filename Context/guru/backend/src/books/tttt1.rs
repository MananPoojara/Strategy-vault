//! TradeTimer TT1 for TS (tradetimer.com). Source (research drive, read in place, read-only):
//! `USB HDD 1/TradeStation Systems & Indicators/TradeTimer TT1 for TS (tradetimer.com)/Open Code.rar` -> `Open Code.txt`
//! (extracted as data; the password/date lock lines are commented out in the file). Book id `tttt1`.
//! Notes: library/tttt1/README.md. Engine: books::rrsim.
use crate::books::rrsim::{g, o, of, pd, run_ts, sdef, sma, true_range, A, P};
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "tttt1";
/// Admitted ids (authors/lanea/vendor2/decisions.json).
pub const REVIEWED_LANEA: &[&str] = &["tttt1_tradetimer"];
pub fn reviewed_lanea(id: &str) -> bool { REVIEWED_LANEA.contains(&id) }
const ID: &str = "tttt1_tradetimer";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![sdef(BOOK, ID, "TradeTimer TT1 (20-day MA slope, ROC(7) pullback and range breakout stops)", "TradeTimer TT1 (Open Code.txt)", "Long/Short", Status::Live, &[
        "F|\"Input: Lng(7), VV(.75), BTH(0), STH(0), STP(.75), trlng(10), ma(20)\"; \"Value3=RateOfChange(c,Lng); Value22=Highest(truerange,trlng); Condition1=Value3<BTH; Condition2=Value3>STH; Value4=TrueRange*VV; Value5=(stp*Value22)\"",
        "E|\"If condition1=true and Average(c,ma)>average(c,ma)[1] then Buy (\\\"LE#1\\\") at OPEN of tomorrow + Value4 stop\"; \"IF CONDITION2=TRUE and Average(c,ma)<average(c,ma)[1] THEN Sell Short (\\\"SE#1\\\") at OPEN of tomorrow - Value4 Stop\"",
        "E|\"If Average(c,ma)>average(c,ma)[1] then Buy (\\\"LE#2\\\") Next Bar at HIGH + value5 Stop\"; \"If Average(c,ma)<average(c,ma)[1] then Sell Short (\\\"SE#2\\\") Next Bar at LOW - value5 Stop\"",
        "X|MA falling: \"Sell Next Bar FROM ENTRY (\\\"LE#1\\\") at LOW - (stp*Value22) Stop\" and the same FROM ENTRY (\\\"LE#2\\\"); MA rising: \"Buy to Cover Next Bar FROM ENTRY (\\\"SE#1\\\"/\\\"SE#2\\\") at HIGH + (stp*Value22) Stop\"",
        "X|\"If condition1=true and Average(c,ma)<average(c,ma)[1] then Buy to Cover FROM ENTRY (\\\"SE#1\\\"/\\\"SE#2\\\") at OPEN of tomorrow + Value4 stop\"; \"IF CONDITION2=TRUE and Average(c,ma)>average(c,ma)[1] THEN Sell FROM ENTRY (\\\"LE#1\\\"/\\\"LE#2\\\") at OPEN of tomorrow - Value4 Stop\"",
    ], "Daily code ('OPEN of tomorrow'). RateOfChange = (C/C[7] - 1) x 100. Exits are tied to the entry that opened the position (FROM ENTRY); the Sell Short / Buy entries also reverse. \
[Assumption, TradeStation defaults] one position at a time (no pyramiding, so LE#1 and LE#2 never stack); stop orders fill along O -> nearer extreme -> other extreme -> C and an exit can fill on the entry bar (rrsim). \
No manual in the folder. India: instrument swap; index shorts hypothetical.")]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == ID).then_some(("tttt1", "TradeTimer TT1 (tradetimer.com)", "code literal"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    (id == ID).then(|| vec![pd("lng", "Lng (rate-of-change length)", 7., 2., 30., 1.), pd("vv", "VV (x true range from the open)", 0.75, 0.1, 2., 0.05),
        pd("bth", "BTH (buy when ROC below)", 0., -10., 10., 0.5), pd("sth", "STH (sell when ROC above)", 0., -10., 10., 0.5),
        pd("stp", "STP (x highest true range)", 0.75, 0.1, 2., 0.05), pd("trlng", "trlng (highest true range length)", 10., 2., 40., 1.), pd("ma", "ma (average length)", 20., 5., 100., 1.)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if id != ID || from >= to || to >= x.r.len() { return None; }
    let lng = g(p, "lng", 7.).max(1.) as usize;
    let (vv, bth, sth, stp) = (g(p, "vv", 0.75), g(p, "bth", 0.), g(p, "sth", 0.), g(p, "stp", 0.75));
    let trl = g(p, "trlng", 10.).max(1.) as usize;
    let ma = sma(&x.c, g(p, "ma", 20.).max(1.) as usize);
    let tr = true_range(x);
    let (h, l, c) = (&x.h, &x.l, &x.c);
    Some(run_ts(x, from, to, |i, _st, _now, next| {
        if i < lng || i < trl || i < 1 || !ma[i - 1].is_finite() { return; }
        let Some(&on) = x.o.get(i + 1) else { return };
        let roc = (c[i] / c[i - lng] - 1.) * 100.;
        let v22 = tr[i + 1 - trl..=i].iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let (c1, c2) = (roc < bth, roc > sth);
        let (v4, v5) = (tr[i] * vv, stp * v22);
        let (up, dn) = (ma[i] > ma[i - 1], ma[i] < ma[i - 1]);
        if c1 && up { next.push(o(A::Buy, P::Stop(on + v4), "LE#1")); }
        if c2 && dn { next.push(o(A::Sell, P::Stop(on - v4), "SE#1")); }
        if up { next.push(o(A::Buy, P::Stop(h[i] + v5), "LE#2")); }
        if dn { next.push(o(A::Sell, P::Stop(l[i] - v5), "SE#2")); }
        if dn { next.push(of(A::Xl, P::Stop(l[i] - v5), "LX (MA down)", "LE#1")); }
        if up { next.push(of(A::Xs, P::Stop(h[i] + v5), "SX (MA up)", "SE#1")); }
        if dn { next.push(of(A::Xl, P::Stop(l[i] - v5), "LX (MA down)", "LE#2")); }
        if up { next.push(of(A::Xs, P::Stop(h[i] + v5), "SX (MA up)", "SE#2")); }
        if c1 && dn { next.push(of(A::Xs, P::Stop(on + v4), "SX (open + VV x TR)", "SE#1")); }
        if c2 && up { next.push(of(A::Xl, P::Stop(on - v4), "LX (open - VV x TR)", "LE#1")); }
        if c1 && dn { next.push(of(A::Xs, P::Stop(on + v4), "SX (open + VV x TR)", "SE#2")); }
        if c2 && up { next.push(of(A::Xl, P::Stop(on - v4), "LX (open - VV x TR)", "LE#2")); }
    }))
}
