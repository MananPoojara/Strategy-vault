//! William Tell Coffee Strategy (Market Analytics, marketanalytics.net), TradeStation code "WT Coffe Strategy.txt" (research share
//! USB HDD 1/TradeStation Systems & Indicators/Market Analytics - William Tell Coffee Trading System (marketanalytics.net)/WT Coffe
//! Strategy.txt). Book id `tswtc`. Lane K18 (titles loop, 2 Nov 2026), to-code row 330; independent check
//! authors/titles-1030/K18w/k18_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::rrsim::{highest, o, run_ts, A, P};
use super::sctraub::{civil, days, ticks, weekday};

pub const REVIEWED: &[&str] = &["tswtc_coffee"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] Daily price-pattern system for coffee futures; the vault holds no coffee, so it runs on daily indices, ETFs and stocks as a declared application. The printed TradeStation code is the rule: H, L, O, C are the signal day, [n] earlier days, day-of-week and calendar tests use the signal day's date, and 'open of next bar' is the next open (the code reads it when it places its orders). Orders are placed only when flat, or when a long (short) will open above (below) its entry price; then the position is closed at that open (Sell / Buy to Cover next bar at open) and the fourteen entry orders are issued in code order: (1) Thu/Fri, L > H[1]: sell short stop at H[1]; (2) October Friday: buy at the open; (3) not Tue/Fri, L > H[2], L[1] >= H[3], next open < (H - L) x 0.3 + H: buy stop at H + 0.3 (H - L); (4) Mon/Wed/Thu, L > L[1] < L[2] > L[3] < L[4]: sell short stop at L; (5) H[1] > H[2] and H[1] > H: sell short stop at L - 1.7 (H - L); (6) Friday, O[1] > C[2], O < C[1]: buy stop at next open + 0.8 (H - L); (7a) Mon/Tue, four down closes (C < O): buy limit at H - (H - L); (7b) Tuesday, up day 3 back then three down days: buy limit at H - 0.9 (H - L); (8) not Tue/Wed, outside day with C > C[1]: sell short stop at next open - 0.5 (H - L); (9) Monday, next open < L: buy stop at L; (10) not Mon/Tue, C < L[1]: sell short stop at next open - 0.9 (H - L); (11) C > H[1]: sell short stop at next open - 0.85 (H - L); (12) Thu/Fri, next open > Highest(H, 4): sell short stop 1 point under that high (1 point = 1 tick of the instrument, decision 57; option tick 0 = no offset); (13a/b) on Dec 23 if a Thursday or Friday, Dec 22 if a Friday, or any Dec 24: buy (sell short) at the open after an up (down) day; (13c/d) the Wednesday 20 to 26 days into November (month(date - 20 days) = November and month(date - 27 days) is not, the eve of Thanksgiving): buy (sell short) at the open after an up (down) day. An entry in the direction already held is not placed (TradeStation's no-pyramiding rule), an opposite entry reverses. There is no stop loss in the code: a losing position stays open until a later order reverses it. Fills on the vault's TradeStation daily-bar engine (rrsim). No results are printed. Runs on indices, ETFs and stocks, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "tswtc_coffee", name: "William Tell Coffee: fourteen day-of-week price patterns with stop / limit / open entries, exit at the first profitable open", group: "TradeStation code: William Tell Coffee (Market Analytics)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "tswtc", rules: &[
            "E|if marketposition=0 or (marketposition=1 and open of next bar > entryprice) or (marketposition=-1 and open of next bar < entryprice) then begin ... entries (1) to (13d) ... end",
            "X|if open of next bar > entryprice and marketposition=1 then Sell next bar at open; if open of next bar < entryprice and marketposition=-1 then Buy to Cover next bar at open",
            "X|No stop loss: a losing trade is held until a later entry reverses it."],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { reviewed(id).then_some(("tswtc_coffee", "William Tell Coffee (TradeStation)", "code as printed")) }

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("tick", "Rule 12 offset in ticks (1 point = 1 tick)", 1., 0., 5., 1.)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let pt = ticks(x, p);
    let (o_, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let n = c.len();
    Some(run_ts(x, from, to, |t, st, _now, next| {
        if t < 4 || t + 1 >= n { return; }
        let nx = o_[t + 1];
        let mp = st.pos;
        if !(mp == 0 || (mp == 1 && nx > st.entry_px) || (mp == -1 && nx < st.entry_px)) { return; }
        if nx > st.entry_px && mp == 1 { next.push(o(A::Xl, P::Mkt, "exit at a profitable open")); }
        if nx < st.entry_px && mp == -1 { next.push(o(A::Xs, P::Mkt, "exit at a profitable open")); }
        let date = x.r[t].date.as_str();
        let dw = weekday(date) + 1; // TradeStation DayOfWeek: 1 = Monday ... 5 = Friday
        let (_, m, d) = civil(days(date));
        let mut v: Vec<(A, P, &'static str)> = vec![];
        let r = h[t] - l[t];
        if (dw == 4 || dw == 5) && l[t] > h[t - 1] { v.push((A::Sell, P::Stop(h[t - 1]), "1")); }
        if m == 10 && dw == 5 { v.push((A::Buy, P::Mkt, "2")); }
        if dw != 2 && dw != 5 && l[t] > h[t - 2] && l[t - 1] >= h[t - 3] && nx < (h[t] - l[t]) * 0.3 + h[t] { v.push((A::Buy, P::Stop(h[t] + 0.3 * r), "3")); }
        if (dw == 1 || dw == 3 || dw == 4) && l[t] > l[t - 1] && l[t - 1] < l[t - 2] && l[t - 2] > l[t - 3] && l[t - 3] < l[t - 4] { v.push((A::Sell, P::Stop(l[t]), "4")); }
        if h[t - 1] > h[t - 2] && h[t - 1] > h[t] { v.push((A::Sell, P::Stop(l[t] - 1.7 * r), "5")); }
        if dw == 5 && o_[t - 1] > c[t - 2] && o_[t] < c[t - 1] { v.push((A::Buy, P::Stop(nx + 0.8 * r), "6")); }
        if (dw == 1 || dw == 2) && c[t - 3] < o_[t - 3] && c[t - 2] < o_[t - 2] && c[t - 1] < o_[t - 1] && c[t] < o_[t] { v.push((A::Buy, P::Lim(h[t] - r), "7a")); }
        if dw == 2 && c[t - 3] > o_[t - 3] && c[t - 2] < o_[t - 2] && c[t - 1] < o_[t - 1] && c[t] < o_[t] { v.push((A::Buy, P::Lim(h[t] - 0.9 * r), "7b")); }
        if dw != 2 && dw != 3 && h[t] > h[t - 1] && l[t] < l[t - 1] && c[t] > c[t - 1] { v.push((A::Sell, P::Stop(nx - 0.5 * r), "8")); }
        if dw == 1 && nx < l[t] { v.push((A::Buy, P::Stop(l[t]), "9")); }
        if dw != 1 && dw != 2 && c[t] < l[t - 1] { v.push((A::Sell, P::Stop(nx - r * 0.9), "10")); }
        if c[t] > h[t - 1] { v.push((A::Sell, P::Stop(nx - r * 0.85), "11")); }
        let h4 = highest(h, 4, t);
        if (dw == 4 || dw == 5) && nx > h4 { v.push((A::Sell, P::Stop(h4 - pt), "12")); }
        let xmas = m == 12 && ((d == 23 && dw == 5) || d == 24 || (d == 22 && dw == 5) || (d == 23 && dw == 4));
        let tg = m == 11 && dw == 3 && civil(days(date) - 20).1 == 11 && civil(days(date) - 27).1 != 11;
        if xmas || tg {
            if c[t] > o_[t] { v.push((A::Buy, P::Mkt, "13")); }
            if c[t] < o_[t] { v.push((A::Sell, P::Mkt, "13")); }
        }
        for (a, pr, tag) in v {
            let d = if a == A::Buy { 1 } else { -1 };
            if d != mp { next.push(o(a, pr, tag)); }
        }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn coffee_patterns_trade() {
        let r = super::super::sctraub::tests::bars(2500);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("tswtc_coffee", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() > 5 && t.iter().any(|z| z.dir > 0.) && t.iter().any(|z| z.dir < 0.));
        for w in t.windows(2) { assert!(w[1].legs[0].i >= w[0].exit_i); }
        assert!(run("tswtc_x", &x, 0, 5, &Params::new()).is_none());
    }
}

// Instrument admission: William Tell Coffee 2026-11-02 (titles lane K18).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "tswtc" && reviewed(def.id),
    label: "TradeStation pattern system for coffee futures, with no printed results. Runs on indices, ETFs and stocks, US and India, are a declared application.",
    basis: "WT Coffe Strategy.txt (Market Analytics, printed TradeStation code); independent check authors/titles-1030/K18w/k18_ref.py",
};
