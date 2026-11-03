//! Murray A. Ruggiero Jr., "Inside Advantage" newsletter: Vol.1 Issue 5 (October 1996, scanned pages 45-56) and the
//! May 1998 Special Intermarket Classic Issue. Book id `mrina`.
//! Source: research drive "Trading Knowledge II/@NewsLetters-Mags/Murray A.Ruggiero - Inside Advantage Magazine (Oct. 96
//! & May 98)/" (~hpa0007-0018.jpg = Oct 1996 pp.1-12, tt0502.pdf = May 1998); copies in Book-to-rust/Books/Murray A.
//! Ruggiero Jr/. Only the standard 14-period Slow K extreme rule is fully specified; the adaptive (dominant-cycle) versions
//! are not, and the intermarket study needs data the vault lacks.
use crate::connors::{sma, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "mrina";
/// Admitted ids (authors/lanea/newsletters/decisions.json).
pub const REVIEWED_LANEA: &[&str] = &["mrina_xstoch14"];
pub fn reviewed_lanea(id: &str) -> bool { REVIEWED_LANEA.contains(&id) }

fn mk(dir: f64, i: usize, px: f64, sig: usize) -> Trade {
    Trade { dir, legs: vec![Leg { i, px, w: 1. }], exit_i: i, exit_px: px, reason: String::new(), entry_kind: "open", signal_i: sig, open: false, exits: vec![] }
}
fn fin(mut t: Trade, i: usize, px: f64, sig: usize, why: &str) -> Trade { t.exit_i = i; t.exit_px = px; t.signal_i = sig; t.reason = why.into(); t }

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    let d = |id, name, group, dir, status, rules, notes, claim| StrategyDef { id, name, group, session: 0, dir, status, needs_volume: false, needs_vix: false,
        needs_series: &[], book: BOOK, rules, notes, claim };
    vec![
        d("mrina_xstoch14", "Extreme stochastic rule · standard 14-period Slow K", "Oct 1996 · Trading stochastics mechanically", "Long/Short", Live,
          &["F|\"If Slow K >95 then set a flag to buy; If Slow K<5 then set a flag to sell\" (Oct 1996 p.5, scan ~hpa0011)",
            "E|\"If Buy Flag is True and SlowK has retraced to below 50 then buy when Slow K is higher than 2 days ago\"; \"If Sell Flag is True and SlowK has retraces up to 50 then sell when Slow K is lower than 2 days ago\"",
            "X|\"Exit long when SlowK has risen to 85; Exit Short when SlowK has fallen to 15\"",
            "F|\"We tested these rules using both the standard 14 period Slow K and our adaptive Slow K\" - this is the standard one"],
          "Readings (book silent): Slow K = 3-bar simple average of the 14-bar Fast K (TradeStation SlowK); 'has retraced to below 50' = Slow K below 50 on the entry bar (above 50 for sells); a flag is cleared when its trade is entered; orders at the next open (the same issue's coded pullback system trades 'at open'); an opposite entry reverses. Book: T-bond futures 1/1/1986-11/10/1996.",
          crate::connors::claim("37", "84%", "$203.38", "", "T-bond futures 1/1/86-11/10/96; net $7,525, DD -$40,093, PF 1.20")),
        d("mrina_xstoch_adaptive", "Extreme stochastic rule · adaptive Slow K", "Oct 1996 · Trading stochastics mechanically", "Long/Short",
          NotTestable("the adaptive Slow K is 'tuned to 50% of the dominant cycle' found with MESA/TradeCycles; the cycle method is not given"),
          &["E|Same flags and exits with \"our adaptive Slow K\" (p.5)"], "", crate::connors::claim("35", "80%", "$978.57", "", "T-bond futures 1/1/86-11/10/96, PF 7.97")),
        d("mrina_pullback_adaptive", "Buy pullbacks in a trend with adaptive Slow K", "Oct 1996 · Trading stochastics mechanically", "Long",
          NotTestable("uses the undefined adaptive (dominant-cycle) Slow K; the $1,750 variant is a futures dollar stop"),
          &["E|\"If Slow K<30 and Average(Close,10)>Average(Close,20) then buy at open\" (p.6)", "X|\"If Slow K>80 then exitlong at open\""], "",
          crate::connors::claim("62", "74%", "$677.82", "", "T-bonds 1/1/86-11/19/96 (no stop)")),
        d("mrina_im_groups", "S&P 500 stock groups predicting T-bonds and crude (weekly)", "May 1998 · Special Intermarket Classic Issue", "Long/Short",
          NeedsData("weekly S&P 500 industry-group indices (steel, chemicals, capital goods, oil) and T-bond / crude futures"),
          &["E|\"intermarket divergence concept of price relative to a moving average for different S&P500 stock groups using weekly data for the period 1/1/86 to 12/30/97\" (tt0502 p.1)"], "", None),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "mrina_xstoch14" => ("mrina_xstoch", "Extreme stochastic rule", "Standard 14"),
        "mrina_xstoch_adaptive" => ("mrina_xstoch", "Extreme stochastic rule", "Adaptive"),
        "mrina_pullback_adaptive" => ("mrina_pullback", "Stochastic pullback", ""),
        "mrina_im_groups" => ("mrina_im", "Intermarket stock groups", ""),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    match id {
        "mrina_xstoch14" => Some(vec![p("len", "Fast K length", 14., 5., 40., 1.), p("hi", "Buy-flag level", 95., 80., 99., 1.), p("lo", "Sell-flag level", 5., 1., 20., 1.),
                                      p("mid", "Retrace level", 50., 30., 70., 5.), p("xl", "Long exit level", 85., 60., 99., 1.), p("xs", "Short exit level", 15., 1., 40., 1.)]),
        _ => None,
    }
}

pub fn slowk(x: &Ctx, len: usize) -> Vec<f64> {
    let n = x.r.len();
    let fk: Vec<f64> = (0..n).map(|i| if i + 1 < len { f64::NAN } else {
        let h = x.h[i + 1 - len..=i].iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let l = x.l[i + 1 - len..=i].iter().copied().fold(f64::INFINITY, f64::min);
        if h > l { 100. * (x.c[i] - l) / (h - l) } else { 50. } }).collect();
    sma(&fk, 3)
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if from > to || to >= x.r.len() { return None; }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(0., |d| d.def));
    let sk = slowk(x, (g("len").round() as usize).max(2));
    let (fh, fl, mid, xl, xs) = (g("hi"), g("lo"), g("mid"), g("xl"), g("xs"));
    let (mut bf, mut sf) = (false, false);
    let mut out = vec![];
    let mut cur: Option<Trade> = None;
    for t in 2..to {
        let n = t + 1;
        if !sk[t].is_finite() || !sk[t - 2].is_finite() { continue; }
        if sk[t] > fh { bf = true; }
        if sk[t] < fl { sf = true; }
        if let Some(tr) = cur.take() {
            if (tr.dir > 0. && sk[t] >= xl) || (tr.dir < 0. && sk[t] <= xs) { out.push(fin(tr, n, x.o[n], t, "Slow K exit level · next open")); } else { cur = Some(tr); }
        }
        if t < from { continue; }
        let d = if bf && sk[t] < mid && sk[t] > sk[t - 2] { 1. } else if sf && sk[t] > mid && sk[t] < sk[t - 2] { -1. } else { continue };
        if d > 0. { bf = false } else { sf = false }
        match cur.take() {
            Some(tr) if tr.dir == d => cur = Some(tr),
            Some(tr) => { out.push(fin(tr, n, x.o[n], t, "reversed by opposite entry")); cur = Some(mk(d, n, x.o[n], t)); }
            None => cur = Some(mk(d, n, x.o[n], t)),
        }
    }
    if let Some(mut t) = cur { t.exit_i = to; t.exit_px = x.c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true; out.push(t); }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn flag_then_retrace_then_turn_up_buys_next_open() {
        // closes: rally to a 14-day high (Slow K -> 100), then a dip, then a turn up
        let mut cl: Vec<f64> = (0..30).map(|i| 100. + i as f64).collect();
        cl.extend([120., 112., 110., 111., 113., 116., 118.]);
        let r: Vec<Row> = cl.iter().enumerate().map(|(i, &c)| Row { ticker: "T".into(), date: format!("2000-01-{:02}", 1 + i % 28), open: c, high: c + 0.5, low: c - 0.5, close: c, volume: None }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let sk = slowk(&x, 14);
        let ts = run("mrina_xstoch14", &x, 0, r.len() - 1, &Params::new()).unwrap();
        let e = ts[0].legs[0].i - 1;
        assert!(sk[e] < 50. && sk[e] > sk[e - 2]);
        assert!((0..e).any(|i| sk[i] > 95.));
        assert_eq!(ts[0].legs[0].i, e + 1, "{:?}", ts); assert_eq!(ts[0].dir, 1.);
    }
}

// Instrument admission: Lane A newsletters 2026-10-26 (lane B) block, moved from instrument_policy.rs on 31 Oct 2026 (lane ST).
// Admission only for the module REVIEWED lists (authors/lanea/newsletters/decisions.json); kept in one block to merge.
// Runs on actual indices, ETFs and stocks, US and India (index_etf_or_stock, nifty50 basket, declared application).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| (def.book=="mrina" && crate::books::mrina::reviewed_lanea(def.id)) || (def.book=="tsx" && crate::books::tsx::reviewed_lanea(def.id)) || (def.book=="jrfsp" && crate::books::jrfsp::reviewed_lanea(def.id)) || (def.book=="bpqst" && crate::books::bpqst::reviewed_lanea(def.id)) || (def.book=="wmmts" && crate::books::wmmts::reviewed_lanea(def.id)),
    label: "Newsletter / course rule with no per-market test the vault can compare; runs on indices, ETFs and single stocks (US and India) are a declared application, one unit per trade.",
    basis: "Newsletters and courses: Futures Magazine (Ruggiero, Aug 2003), Inside Advantage (Ruggiero, Oct 1996), TS Express (Brower, Sep/Oct 1997), FirstStrike Plus (Rensink), Quantum Swing Trader (Poulos), Market Turn Strategy (McWilliams)",
};
