//! Alberto Sabatini, "Trading Toolkit Collection Vol.3: Gli Swing Chart di W.D. Gann" (Italian; scanned pages; book id `astk3`;
//! printed pages 3-x). Reading: claude-continuation-20261020/authors/sabatini (OCR text/ocr-all.txt checked against the page
//! images; authors/lanec/admission3/lanec3_ref.py). Admitted: the one-bar Gann swing chart trend method, "di per se ... un sistema di trading
//! completo" (p.3-22). Held: outside-bar breakout orders (how long the order stays live is not stated), double/triple tops and
//! bottoms and Gann angles (judgement). Book: Italian stocks; other instruments are a declared application.
use crate::books::lc3::{high_first, Pos};
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const REVIEWED: &[&str] = &["astk3_swing"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        StrategyDef { id: "astk3_swing", name: "Gann one-bar swing chart: buy above the last swing top, out below the last swing bottom", group: "Gli swing chart (p.3-9 to 3-22)",
            session: 0, dir: "Long", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "astk3",
            rules: &["F|\"basta partire dal punto di massimo di una barra e se nella barra seguente il massimo e maggiore di quello della precedente e il minimo e maggiore anch'esso del precedente, la linea del grafico si sposta dal massimo della prima a quello della seconda, viceversa nella situazione opposta\" (p.3-9)",
                "F|Inside bars: \"lo swing non si aggiorna ... la barra antecedente l'inside rimane l'ultima barra attiva\" (p.3-9). Outside bars: \"quando disponiamo di serie storiche di dati daily ... e corretto assumere che il prezzo piu vicino all'apertura si sia verificato per primo, mentre quello vicino alla chiusura per ultimo\" (p.3-10)",
                "E|\"Se il mercato supera uno swing top il trend passa da negativo a positivo, viceversa la penetrazione di uno swing bottom inverte il trend da positivo a negativo ... non cambia fino quando non viene perforato il precedente swing bottom oppure lo swing top.\" (p.3-9)",
                "X|\"Se avessimo comprato il titolo sulla rottura di R1, allora lo stop loss avrebbe dovuto essere posizionato sotto S1 ... si sarebbe usciti sulla rottura al ribasso di S2\" (Fig.15, p.3-18): the stop follows the last swing bottom; p.short=1 also sells short there (\"stop and reverse\", p.3-20)"],
            notes: "[Declared readings] Up bar = higher high and a low not below the active bar's low; down bar = lower low and a high not above it; inside bar (neither) = ignored, the active bar stays; outside bar = both, taken high-then-low when the high is nearer the open (ties: high first), else low-then-high. Swing top = the swing line's high where it turns down; swing bottom = its low where it turns up. Orders use the last swing top/bottom known at the previous close; 'supera'/'rottura' = the price trades beyond the level: filled at the level, or at the open when the day opens beyond it. If one day crosses both levels, the one nearer the open comes first. Long or flat by default (the book's stock examples); p.short=1 reverses into shorts. One unit, no costs.",
            claim: None },
        StrategyDef { id: "astk3_outside", name: "Outside-bar breakout (held: order life not stated)", group: "Barre di outside (p.3-10 to 3-20)", session: 0, dir: "Long/Short",
            status: Status::NotTestable("The book defines bullish and bearish outside bars exactly and buys a break of the bullish bar's high with a stop under its low (Fig.18 fills five days later), but never says how long the order stays live or when a long without a bearish outside bar ends."),
            needs_volume: false, needs_vix: false, needs_series: &[], book: "astk3",
            rules: &["F|\"Outside rialzista: H > Ref(H,-1) AND L < Ref(L,-1) AND Ref(H,-1) < Ref(H,-2) AND Ref(L,-1) < Ref(L,-2)\"; \"Outside ribassista: H > Ref(H,-1) AND L < Ref(L,-1) AND Ref(H,-1) > Ref(H,-2) AND Ref(L,-1) > Ref(L,-2)\" (p.3-20)",
                "E|bullish bar: \"il valore massimo di barra (H) della barra stessa che se violato al rialzo consente di effettuare un primo acquisto speculativo\" (p.3-20)",
                "X|bearish bar: \"se il mercato viola il minimo di barra (L) allora bisogna uscire dalla posizione speculativa intrapresa in precedenza, oppure aprirne una al ribasso (stop and reverse)\" (p.3-20)"],
            notes: "Held.", claim: None },
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id { "astk3_swing" => Some(("astk3_swing", "Sabatini Gann swing chart", "Swing trend")), "astk3_outside" => Some(("astk3_outside", "Sabatini Gann swing chart", "Outside bars")), _ => None }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    match id {
        "astk3_swing" => Some(vec![ParamDef { key: "short", label: "Also sell short on a swing-bottom break (1) - the book's 'stop and reverse'", def: 0., min: 0., max: 1., step: 1. }]),
        "astk3_outside" => Some(vec![]),
        _ => None,
    }
}

/// Swing levels known at the close of each bar: (last swing top, last swing bottom), NaN before the first.
pub fn swings(x: &Ctx, to: usize) -> Vec<(f64, f64)> {
    let (o, h, l) = (&x.o, &x.h, &x.l);
    let (mut dir, mut ext, mut ah, mut al) = (0i8, f64::NAN, h[0], l[0]);
    let (mut top, mut bot) = (f64::NAN, f64::NAN);
    let mut out = vec![(f64::NAN, f64::NAN)];
    for i in 1..=to {
        let (up, dn) = (h[i] > ah, l[i] < al);
        let mut go = |d: i8, v: f64| {
            if d == 1 { if dir == -1 { bot = ext; } if dir == 1 { ext = ext.max(v); } else { ext = v; } }
            else { if dir == 1 { top = ext; } if dir == -1 { ext = ext.min(v); } else { ext = v; } }
            dir = d;
        };
        if up && dn { if high_first(o[i], h[i], l[i]) { go(1, h[i]); go(-1, l[i]); } else { go(-1, l[i]); go(1, h[i]); } }
        else if up { go(1, h[i]); } else if dn { go(-1, l[i]); }
        if up || dn { ah = h[i]; al = l[i]; }
        out.push((top, bot));
    }
    out
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let short = p.get("short").copied().unwrap_or(0.) > 0.5;
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let sw = swings(x, to);
    let mut pos = Pos::new();
    for i in from.max(1)..=to {
        let (t, b) = sw[i - 1];
        let buy = |pos: &mut Pos| if t.is_finite() && h[i] > t && pos.side() != 1. { pos.enter(1., i, o[i].max(t), "stop"); };
        let sell = |pos: &mut Pos| if b.is_finite() && l[i] < b && pos.side() != -1. {
            if short { pos.enter(-1., i, o[i].min(b), "stop"); } else if pos.side() > 0. { pos.exit(i, o[i].min(b), "swing bottom broken"); }
        };
        if high_first(o[i], h[i], l[i]) { buy(&mut pos); sell(&mut pos); } else { sell(&mut pos); buy(&mut pos); }
    }
    Some(pos.done(to, c[to]))
}
