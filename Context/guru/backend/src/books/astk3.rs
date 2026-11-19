//! Alberto Sabatini, "Trading Toolkit Collection Vol.3: Gli Swing Chart di W.D. Gann" (Italian; scanned pages; book id `astk3`;
//! printed pages 3-x). Reading: claude-continuation-20261020/authors/sabatini (OCR text/ocr-all.txt checked against the page
//! images; authors/lanec/admission3/lanec3_ref.py). Admitted: the one-bar Gann swing chart trend method, "di per se ... un sistema di trading
//! completo" (p.3-22). Held: outside-bar breakout orders (how long the order stays live is not stated), double/triple tops and
//! bottoms and Gann angles (judgement). Book: Italian stocks; other instruments are a declared application.
use crate::books::lc3::{high_first, Pos};
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const REVIEWED: &[&str] = &["astk3_swing", "astk3_outside" /* lane R2A, R4 #297 */];
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
        StrategyDef { id: "astk3_outside", name: "Outside-bar breakout: buy above a bullish outside bar, stop under its low, trail to bearish outside-bar lows", group: "Barre di outside (p.3-10 to 3-21)", session: 0, dir: "Long",
            status: Status::Live,
            needs_volume: false, needs_vix: false, needs_series: &[], book: "astk3",
            rules: &["F|\"Outside rialzista: H > Ref(H,-1) AND L < Ref(L,-1) AND Ref(H,-1) < Ref(H,-2) AND Ref(L,-1) < Ref(L,-2)\"; \"Outside ribassista: H > Ref(H,-1) AND L < Ref(L,-1) AND Ref(H,-1) > Ref(H,-2) AND Ref(L,-1) > Ref(L,-2)\" (p.3-20)",
                "E|bullish bar: \"il valore massimo di barra (H) della barra stessa che se violato al rialzo consente di effettuare un primo acquisto speculativo\" (p.3-20); Fig.18: bar of 5/3/2001, high 5.94, filled on 12/3/2001 at 5.94",
                "X|\"lo stop loss posto al di sotto del minimo di barra del 5/3/2001\" (Fig.18, p.3-20): the initial stop is under the bullish bar's low",
                "X|bearish bar: \"se il mercato viola il minimo di barra (L) allora bisogna uscire dalla posizione speculativa intrapresa in precedenza, oppure aprirne una al ribasso (stop and reverse)\" (p.3-20); \"il primo trailing stop ci viene fornito dall'outside ribassista\" (Fig.20, p.3-21); p.short=1 also sells short there"],
            notes: concat!("Adapted: 2 unstated details (R4 #297). Unstated in the book: how long the buy order stays live (Fig.18 fills five days after the bar); vault reading: the buy stop at the bullish bar's high stays live until it fills or the next outside bar forms (a newer bullish bar replaces it, a bearish bar cancels it). ",
                "Unstated in the book: when a long ends if no bearish outside bar comes; vault reading: only on its stop, which starts at the bullish bar's low and moves up to the low of each later bearish outside bar (never down). ",
                "Module conventions (not new readings): 'violato' = the price trades beyond the level, filled at the level or at the open when the day opens beyond it; if one day crosses the entry and the stop, the extreme nearer the open comes first (the module's daily-bar order, p.3-10). Long or flat by default (the book's stock examples). ",
                "p.short=1 (labelled option): a bearish outside bar's low break also sells short (when flat, or reversing a long whose stop is that low), stop above that bar's high, trailed down to later bullish outside-bar highs; a bullish bar's high break then reverses back. Book: Italian stocks; indices, ETFs and stocks are a declared application. No results printed."),
            claim: None },
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id { "astk3_swing" => Some(("astk3_swing", "Sabatini Gann swing chart", "Swing trend")), "astk3_outside" => Some(("astk3_outside", "Sabatini Gann swing chart", "Outside bars")), _ => None }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    match id {
        "astk3_swing" => Some(vec![ParamDef { key: "short", label: "Also sell short on a swing-bottom break (1) - the book's 'stop and reverse'", def: 0., min: 0., max: 1., step: 1. }]),
        "astk3_outside" => Some(vec![ParamDef { key: "short", label: "Also sell short on a bearish outside bar's low break (1) - the book's 'stop and reverse'", def: 0., min: 0., max: 1., step: 1. }]),
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

/// 1 = bullish outside bar (after two falling bars), -1 = bearish outside bar (after two rising bars), 0 = neither (p.3-20).
pub fn outside(h: &[f64], l: &[f64], i: usize) -> i8 {
    if i < 2 || !(h[i] > h[i - 1] && l[i] < l[i - 1]) { return 0; }
    if h[i - 1] < h[i - 2] && l[i - 1] < l[i - 2] { 1 } else if h[i - 1] > h[i - 2] && l[i - 1] > l[i - 2] { -1 } else { 0 }
}

fn run_outside(x: &Ctx, from: usize, to: usize, short: bool) -> Vec<Trade> {
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let mut pos = Pos::new();
    // pend: (dir, entry level, protective stop) set at a close for the next bars; stop/rev: the open position's stop and,
    // when hitting it reverses (p.short), the new position's stop.
    let (mut pend, mut stop, mut rev): (Option<(f64, f64, f64)>, f64, Option<f64>) = (None, f64::NAN, None);
    for j in from.max(2)..=to {
        // path: the open first (a gap beyond a level fills there), then the extreme nearer the open, then the other one (p.3-10)
        let hf = high_first(o[j], h[j], l[j]);
        let path: [(f64, f64); 4] = [(1., o[j]), (-1., o[j]), if hf { (1., h[j]) } else { (-1., l[j]) }, if hf { (-1., l[j]) } else { (1., h[j]) }];
        for (side, v) in path {
            let d = pos.side();
            let beyond = |lv: f64| if side > 0. { v > lv } else { v < lv };
            let px = |lv: f64| if side > 0. { o[j].max(lv) } else { o[j].min(lv) };
            if d == 0. {
                if let Some((pd, lv, pr)) = pend { if pd == side && beyond(lv) { pos.enter(pd, j, px(lv), "stop"); stop = pr; rev = None; pend = None; } }
            } else if d == -side && beyond(stop) {
                let f = px(stop);
                if let Some(pr) = rev { pos.enter(side, j, f, "stop"); stop = pr; rev = None; }
                else { pos.exit(j, f, if d > 0. { "stop: low broken" } else { "stop: high broken" }); }
            }
        }
        let (k, d) = (outside(h, l, j), pos.side());
        if k == 1 {
            if d == 0. { pend = Some((1., h[j], l[j])); }
            else if d < 0. && h[j] <= stop { stop = h[j]; rev = Some(l[j]); }
        } else if k == -1 {
            if d > 0. && l[j] >= stop { stop = l[j]; rev = if short { Some(h[j]) } else { None }; }
            else if d == 0. { pend = if short { Some((-1., l[j], h[j])) } else { None }; }
        }
    }
    pos.done(to, c[to])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    if id == "astk3_outside" { return Some(run_outside(x, from, to, p.get("short").copied().unwrap_or(0.) > 0.5)); }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn outside_bar_breakout_hand_checked() {
        // lane R2A (R4 #297): bar 3 = bullish outside bar (after two falling bars) -> buy stop 10.5, stop 7.5;
        // bar 4 fills at 10.5; bar 6 = bearish outside bar -> stop up to its low 10.4; bar 7 opens 10.3 below it -> out at the open.
        let b = [(10., 11., 9., 10.), (9.5, 10.5, 8.5, 9.), (9., 10., 8., 9.), (9., 10.5, 7.5, 10.), (10.2, 11., 10., 10.8),
                 (10.8, 11.5, 10.6, 11.2), (11., 12., 10.4, 10.5), (10.3, 10.5, 9.8, 10.), (10., 10.2, 9.9, 10.1)];
        let r: Vec<Row> = b.iter().enumerate().map(|(i, &(o, h, l, c))| Row { ticker: "T".into(), date: format!("2001-01-{:02}", i + 1), open: o, high: h, low: l, close: c, volume: None }).collect();
        let x = Ctx::new(&r, &r, &[]);
        assert_eq!((outside(&x.h, &x.l, 3), outside(&x.h, &x.l, 6), outside(&x.h, &x.l, 4)), (1, -1, 0));
        let t = run("astk3_outside", &x, 0, 8, &Params::new()).unwrap();
        let got: Vec<(f64, usize, f64, usize, f64)> = t.iter().map(|t| (t.dir, t.legs[0].i, t.legs[0].px, t.exit_i, t.exit_px)).collect();
        assert_eq!(got, vec![(1., 4, 10.5, 7, 10.3)]);
    }
}
