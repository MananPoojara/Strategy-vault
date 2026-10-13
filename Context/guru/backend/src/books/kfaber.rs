//! Keith Fitschen's Aberration, as found only in third-party code on the research drive (the original is not there):
//! TradeStation Systems & Indicators/Aberration (trade-system.com)/ABBERATION.TXT (EasyLanguage, rlen 34) and
//! MetaStock/@Systems & Indicators/Aberration for MS/MS70PRFT.DTA ("Translated for MS by SR", lengths opt1 and 35).
//! Copies in Books/Keith Fitschen/Aberration (third-party copies)/. Book id "kfaber". Notes: library/kfaber/README.md.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "kfaber";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, status, rules, notes| StrategyDef { id, name, group: "Aberration (third-party code)", session: 0, dir: "Long/Short", status,
        needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None };
    vec![
        d("kfaber_ts34", "Aberration, ABBERATION.TXT copy: Bollinger(34,2) close breakout, exit at the 34-day SMA, next open", Status::Live,
          &["F|\"input:rlen(34),fctr(99999),mult(1)\"; \"value3=fctr*average(range,rlen); value1=bollingerband(c,rlen,2); value2=bollingerband(c,rlen,-2); value4=average(c,rlen); condition2=absvalue(c-o)<value3;\"",
            "E|\"if condition2 and c>=value1 then Buy Next Bar at open; if condition2 and c<=value2 then Sell Short Next Bar at open;\"",
            "X|\"if marketposition=-1 then begin if c>value4*mult then Buy to Cover at tomorrow open;\" / \"if marketposition=1 then begin if c<value4/mult then Sell at tomorrow open;\"",
            "X|The 10-day channel and 2 x ATR(14) stops are commented out in the code ({...}) and are not used."],
          "A third-party copy (trade-system.com folder), not Fitschen's original. Bollinger = SMA(rlen) +- 2 x population standard deviation (TradeStation BollingerBand/StdDev). With fctr = 99999 the |C - O| filter is practically always true; it is kept as printed. An opposite entry reverses at the same open (Buy/Sell Short Next Bar). Results for Aberration elsewhere in the vault (putg_vendor_systems) are Futures Truth vendor tables, not reproducible here. India: instrument swap; index shorts are hypothetical."),
        d("kfaber_ms", "Aberration, MetaStock copy (opt1 / 35)", Status::NotTestable("the MetaStock copy's note says 'Entries ... tomorrow at market price, Exits tomorrow at Closing price', which is not a MetaStock trade-price setting that can be read exactly; its first system uses an unspecified length 'opt1'; and the two copies disagree on the length (34 vs 35) and the exit fill (open vs close), with no original to settle it"),
          &["E|\"UpBand:= BBandTop(C, 35, S, 2); C>UpBand\" (enter long); \"DnBand:= BBandBot(C, 35, S, 2); C<DnBand\" (enter short)",
            "X|\"aver:= Mov(C,35,S); C<aver\" (close long); \"C>aver\" (close short)",
            "F|\"Translated for MS by SR. Entries (long or short) are tomorrow at market price. Exits tomorrow at Closing price\""],
          "MS70PRFT.DTA holds 'Aberration w/opt' (length opt1) and 'Aberration' (length 35)."),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id {
        "kfaber_ts34" => Some(("kfaber", "Aberration (third-party code)", "TradeStation copy, 34")),
        "kfaber_ms" => Some(("kfaber", "Aberration (third-party code)", "MetaStock copy · not testable")),
        _ => None,
    }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    (id == "kfaber_ts34").then(|| vec![p("rlen", "Band / average length", 34., 10., 80., 1.), p("k", "Band width (std devs)", 2., 1., 3., 0.25),
        p("fctr", "|C-O| filter: multiple of the average range", 99999., 0.5, 99999., 0.5), p("mult", "Exit average multiplier", 1., 0.9, 1.1, 0.01)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if id != "kfaber_ts34" || from >= to || to >= x.r.len() { return None; }
    let f = |k: &str, d: f64| p.get(k).copied().unwrap_or(d);
    let (n, k, fctr, mult) = (f("rlen", 34.).max(2.) as usize, f("k", 2.), f("fctr", 99999.), f("mult", 1.));
    let avg = x.sma_n(n);
    let rng: Vec<f64> = x.h.iter().zip(&x.l).map(|(h, l)| h - l).collect();
    let arng = crate::connors::sma(&rng, n);
    let sd: Vec<f64> = (0..x.c.len()).map(|i| {
        if i + 1 < n { return f64::NAN; }
        let w = &x.c[i + 1 - n..=i];
        (w.iter().map(|v| (v - avg[i]).powi(2)).sum::<f64>() / n as f64).sqrt()
    }).collect();
    let mut out = vec![];
    let mut cur: Option<Trade> = None;
    let mut pending: Option<(f64, usize, &'static str)> = None; // target position after the next open
    for i in from..=to {
        if let Some((want, s, why)) = pending.take() {
            let have = cur.as_ref().map_or(0., |t| t.dir);
            if want != have {
                if let Some(mut t) = cur.take() { t.exit_i = i; t.exit_px = x.o[i]; t.signal_i = s; t.reason = why.into(); out.push(t); }
                if want != 0. {
                    cur = Some(Trade { dir: want, legs: vec![Leg { i, px: x.o[i], w: 1. }], exit_i: i, exit_px: x.o[i], reason: String::new(), entry_kind: "open", signal_i: s, open: false, exits: vec![] });
                }
            }
        }
        if i == to { break; }
        let c = x.c[i];
        let cond2 = (c - x.o[i]).abs() < fctr * arng[i];
        let (up, dn) = (avg[i] + k * sd[i], avg[i] - k * sd[i]);
        let have = cur.as_ref().map_or(0., |t| t.dir);
        let mut want = have;
        let mut why = "";
        if have < 0. && c > avg[i] * mult { want = 0.; why = "Close above the average · cover next open"; }
        if have > 0. && c < avg[i] / mult { want = 0.; why = "Close below the average · sell next open"; }
        if cond2 && c >= up { want = 1.; why = "Close at/above the upper band · reverse next open"; }
        else if cond2 && c <= dn { want = -1.; why = "Close at/below the lower band · reverse next open"; }
        if want != have { pending = Some((want, i, why)); }
    }
    if let Some(mut t) = cur { t.exit_i = to; t.exit_px = x.c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true; out.push(t); }
    Some(out)
}
