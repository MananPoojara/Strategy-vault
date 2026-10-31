//! Jose Silva (metastocktools.com), the URSC3 toolkit articles for MetaStock (research share USB HDD 1/MetaStock/@Systems &
//! Indicators/URSC3 TooKit for Metastock (metastocktools.com)/Strategies/sysdev.htm (MSTT article Feb 2006, 'Measuring
//! profitability' code box) and sysdev2.htm (Mar 2006, 'Price breakout backtest results')). Book id `jsursc`. Two author
//! examples, both shown by the author as weak: the price breakout (C > C 5 bars ago in, C < C 10 bars ago out) and the
//! 'Contrarian breakout system example - do not trade!'. Excerpt authors/titles-1030/R3w/ursc_demo_rules.txt.
//! Lane K5 (titles loop, 31 Oct 2026; lead decision R3 #1: code both as labelled author examples, no claim);
//! independent check authors/jsilva/admission/k5_ref.py (header).
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

/// Admitted ids (authors/jsilva/admission/k5_ref.py (header)).
pub const REVIEWED: &[&str] = &["jsursc_breakout", "jsursc_contrarian"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE_B: &str = "[Author's example, shown below its benchmark] The March 2006 article runs this 'simple price breakout strategy' to show that price alone underperforms: on 490 ASX All Ordinaries stocks (9 Jul 2001-3 Mar 2006, 0.2% costs, no shorting, no compounding) it made a median +1.1% a year net, 6.8% a year below the benchmark, median trade 16 calendar days. Not coded: the sector version (the same signals on the stock's GICS sector index, +13.1% a year), which needs a stock-to-sector map. [Declared readings] Long only, signals and fills at the close (the MetaStock indicator values trades at the close). MetaStock binary-flag logic as printed: a flat trader buys on a close where the entry is true and the exit is not; a holder sells on a close where the exit is true and the entry is not; one position at a time. The run starts flat at the window start (MetaStock's first-bar initialisation is not reproduced). The articles' 0.2% total costs are the user's cost setting here, not built in.";
const NOTE_C: &str = "[Author's example - 'Contrarian breakout system example - do not trade!'] The February 2006 article uses this 'simple (but usually unprofitable) strategy' only to demonstrate its Profit Long % indicator; it prints no results. [Declared readings] Long only, signals and fills at the close (the MetaStock indicator values trades at the close). MetaStock binary-flag logic as printed: a flat trader buys on a close where the entry is true and the exit is not; a holder sells on a close where the exit is true and the entry is not; one position at a time. The run starts flat at the window start (MetaStock's first-bar initialisation is not reproduced). The articles' 0.2% total costs are the user's cost setting here, not built in.";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id: &'static str, name: &'static str, rules: &'static [&'static str], notes: &'static str| StrategyDef { id, name, group: "Silva URSC3 MetaStock examples",
        session: 0, dir: "Long", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "jsursc", rules, notes, claim: None };
    vec![
        d("jsursc_breakout", "Price breakout (author's example): buy when the close is above the close 5 days ago, sell when it is below the close 10 days ago", &[
            "E|\"Entry:=C>Ref(C,-5);\" (sysdev2.htm, 'a simple price breakout strategy')",
            "X|\"Exit:=C<Ref(C,-10);\" (sysdev2.htm)",
            "F|\"flag:=ValueWhen(1,entry-exit<>0 OR init,entry); entry:=flag*(Alert(flag=0,2) OR init); exit:=(flag=0)*(Alert(flag,2) OR init);\" (sysdev.htm, the articles' trade logic)"],
            NOTE_B),
        d("jsursc_contrarian", "Contrarian breakout (author's 'do not trade' example): buy a close below the lowest close of the previous 21 days, sell a close above the highest of the previous 10", &[
            "E|\"{ Contrarian breakout system example - do not trade! } entry:=C<Ref(LLV(C,21),-1);\" (sysdev.htm)",
            "X|\"exit:=C>Ref(HHV(C,10),-1);\" (sysdev.htm)",
            "F|\"flag:=ValueWhen(1,entry-exit<>0 OR init,entry); entry:=flag*(Alert(flag=0,2) OR init); exit:=(flag=0)*(Alert(flag,2) OR init);\" (sysdev.htm)"],
            NOTE_C),
    ]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id {
        "jsursc_breakout" => Some(("jsursc_breakout", "Silva price breakout example", "C > C[5] in, C < C[10] out")),
        "jsursc_contrarian" => Some(("jsursc_contrarian", "Silva contrarian breakout example", "21-day low in, 10-day high out")),
        _ => None,
    }
}
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max| ParamDef { key, label, def, min, max, step: 1. };
    match id {
        "jsursc_breakout" => Some(vec![p("n_in", "Entry: close above the close N days ago (article 5)", 5., 1., 100.), p("n_out", "Exit: close below the close N days ago (article 10)", 10., 1., 100.)]),
        "jsursc_contrarian" => Some(vec![p("n_in", "Entry: close below the lowest close of the previous N days (article 21)", 21., 2., 200.), p("n_out", "Exit: close above the highest close of the previous N days (article 10)", 10., 2., 200.)]),
        _ => None,
    }
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let s = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| s.iter().find(|d| d.key == k).unwrap().def);
    let (a, b) = (g("n_in").round().max(1.) as usize, g("n_out").round().max(1.) as usize);
    let c = &x.c;
    let lo = |i: usize, n: usize| c[i - n..i].iter().cloned().fold(f64::INFINITY, f64::min);
    let hi = |i: usize, n: usize| c[i - n..i].iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let (entry, exit): (Box<dyn Fn(usize) -> bool>, Box<dyn Fn(usize) -> bool>) = if id == "jsursc_breakout" {
        (Box::new(move |i| i >= a && c[i] > c[i - a]), Box::new(move |i| i >= b && c[i] < c[i - b]))
    } else {
        (Box::new(move |i| i >= a && c[i] < lo(i, a)), Box::new(move |i| i >= b && c[i] > hi(i, b)))
    };
    let mk = |e: usize, xi: usize, why: &str, open: bool| Trade { dir: 1., legs: vec![Leg { i: e, px: c[e], w: 1. }], exit_i: xi, exit_px: c[xi], reason: why.into(), entry_kind: "close", signal_i: xi, open, exits: vec![] };
    let why = if id == "jsursc_breakout" { format!("close below the close {b} days ago") } else { format!("close above the {b}-day high close") };
    let (mut out, mut cur) = (vec![], None::<usize>);
    for i in from..=to {
        let (en, ex) = (entry(i), exit(i));
        match cur {
            Some(e) if ex && !en => { out.push(mk(e, i, &why, false)); cur = None; }
            None if en && !ex && i < to => cur = Some(i),
            _ => {}
        }
    }
    if let Some(e) = cur { out.push(mk(e, to, "open", true)); }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn binary_flag_entries_and_exits_at_the_close() {
        let cl = [10., 10., 10., 10., 10., 10., 10., 10., 10., 10., 11., 12., 11.5, 9., 8., 8.5, 9.5, 10., 9.];
        let r: Vec<Row> = cl.iter().enumerate().map(|(i, &c)| Row { ticker: "T".into(), date: format!("2001-01-{:02}", i + 1), open: c, high: c, low: c, close: c, volume: None }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let t = run("jsursc_breakout", &x, 0, r.len() - 1, &Params::new()).unwrap();
        let got: Vec<_> = t.iter().map(|t| (t.legs[0].i, t.exit_i, t.open)).collect();
        assert_eq!(got[0], (10, 13, false));
        for t in &t { let e = t.legs[0].i; assert!(cl[e] > cl[e - 5] && !(cl[e] < cl[e - 10])); }
        let k = run("jsursc_contrarian", &x, 0, r.len() - 1, &{ let mut p = Params::new(); p.insert("n_in".into(), 3.); p.insert("n_out".into(), 2.); p }).unwrap();
        assert_eq!(k.iter().map(|t| (t.legs[0].i, t.exit_i)).collect::<Vec<_>>()[0], (13, 16));
        assert!(run("jsursc_x", &x, 0, r.len() - 1, &Params::new()).is_none());
    }
}

pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "jsursc" && crate::books::jsursc::reviewed(def.id),
    label: "Author's examples, not recommendations: the price breakout was run on ASX All Ordinaries stocks and shown below its benchmark, the contrarian breakout is labelled 'do not trade!'. Runs on indices, ETFs and stocks, US and India, are a declared application.",
    basis: "Jose Silva, metastocktools.com URSC3 toolkit articles: sysdev.htm (Feb 2006, 'Measuring profitability') and sysdev2.htm (Mar 2006, 'Price breakout backtest results')",
};
