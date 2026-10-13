//! Larry Williams Inner Circle Seminar Indicators for TS, "Other Version": `lwinsidebar.rar` -> `lwinsidebar.txt`
//! ("{system: LW Inside Bar}"). Source (research drive, read in place, read-only): `USB HDD 1/TradeStation Systems &
//! Indicators/Larry Williams Inner Circle Seminar Indicators for TS/Other Version/lwinsidebar.rar` (extracted as data).
//! Book id `lwinb`. Notes: library/lwinb/README.md. Engine: books::rrsim.
use crate::books::rrsim::{g, o, pd, run_ts, sdef, sma, A, P};
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "lwinb";
const ID: &str = "lwinb_inside_bar";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![sdef(BOOK, ID, "LW Inside Bar (rising regression slope, buy above the mother bar)", "LW Inside Bar (lwinsidebar.txt)", "Long", Status::Live, &[
        "F|\"slope = LinearRegSlopeFC(close,SlopeLen)\" SlopeLen 80; \"ok2buy = slope >=slope[SlopeLen/4]\"",
        "E|\"If ok2buy and H<H[1] and L>L[1] and C[1]>O[1] then buy ncontr contracts at h[1] stop\" (next bar, at the mother bar's high)",
        "X|\"If MP=1 and BarsSinceEntry > ExitWaitBars then exitlong all contracts at Average(low,5) - .1/100*close stop\" ExitWaitBars 30",
    ], "Complete code: long only, one contract, no other stop (the exit stop exists only after 30 bars in the trade). Daily bars assumed (Inner Circle seminar material). \
LinearRegSlopeFC = least-squares slope of the last 80 closes per bar. TradeStation old-style 'buy ... at X stop' = next bar. Fills: stop at its level or at the open when gapped through (rrsim). \
India: instrument swap (NIFTY 50 instead of the futures/stocks of the seminar).")]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == ID).then_some(("lwinb", "LW Inside Bar (Inner Circle TS code)", "code literal"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    (id == ID).then(|| vec![pd("slope_len", "SlopeLen (slope lag = SlopeLen/4)", 80., 20., 200., 4.), pd("wait", "ExitWaitBars", 30., 1., 100., 1.),
        pd("ma", "Exit average of lows (bars)", 5., 2., 20., 1.), pd("pct", "Exit offset (% of close)", 0.1, 0., 2., 0.05)])
}

/// Least-squares slope of v over the last n bars (x = bar number).
pub fn lr_slope(v: &[f64], n: usize) -> Vec<f64> {
    let nf = n as f64;
    let sx = nf * (nf - 1.) / 2.;
    let sxx = (nf - 1.) * nf * (2. * nf - 1.) / 6.;
    (0..v.len()).map(|i| if i + 1 < n || n < 2 { f64::NAN } else {
        let (mut sy, mut sxy) = (0., 0.);
        for k in 0..n { let y = v[i + 1 - n + k]; sy += y; sxy += k as f64 * y; }
        (nf * sxy - sx * sy) / (nf * sxx - sx * sx)
    }).collect()
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if id != ID || from >= to || to >= x.r.len() { return None; }
    let sl = g(p, "slope_len", 80.).max(2.) as usize;
    let lag = sl / 4;
    let wait = g(p, "wait", 30.).max(0.) as usize;
    let pct = g(p, "pct", 0.1);
    let slope = lr_slope(&x.c, sl);
    let avl = sma(&x.l, g(p, "ma", 5.).max(1.) as usize);
    let (h, l, c, op) = (&x.h, &x.l, &x.c, &x.o);
    Some(run_ts(x, from, to, |i, st, _now, next| {
        if i < 1 { return; }
        let ok = i >= lag && slope[i] >= slope[i - lag];
        if ok && h[i] < h[i - 1] && l[i] > l[i - 1] && c[i - 1] > op[i - 1] { next.push(o(A::Buy, P::Stop(h[i - 1]), "LW inside")); }
        if st.pos == 1 && st.bse(i) > wait && avl[i].is_finite() { next.push(o(A::Xl, P::Stop(avl[i] - pct / 100. * c[i]), "after 30 bars: Average(low,5) - 0.1% stop")); }
    }))
}
