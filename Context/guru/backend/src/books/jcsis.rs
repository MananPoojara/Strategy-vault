//! José Luis Cava, "Sistemas de Especulación en Bolsa" (2001; research share USB HDD 1/TRADING KNOWLEDGE-2/_Libros y
//! Cursos en Español/José Luis Cava - Sistemas de Especulación en Bolsa/DBZ_J.Cava- Sistemas de Especulación en Bolsa.pdf,
//! scan, OCR authors/titles-1030/R2w/txt/5845c.txt). Book id `jcsis`. One rule: section IX.1.3 'Tres indicadores' (PDF
//! p.184-187, printed p.192-195), Bryan Strain's line + volume indicator + the MBO of Mark Whitley and Bryan Strain.
//! `djcs_tfs` is James Taylor's variation of the same TFS without the MBO: not changed, this is a separate id.
//! Lane K3 (titles loop, 30 Oct 2026); independent check authors/jlcava/admission/k3c_ref.py (header).
//! Lane K5 (31 Oct 2026) adds IX.1.4.1 RSI + Momentum (PDF p.188-189, printed 196-197) and VII.2 the Three-Bar Rule
//! (PDF p.148-153, printed 154-158); excerpts authors/titles-1030/R3w/cava_rsi_momentum.txt, cava_three_bar.txt;
//! independent check authors/jlcava/admission/k5/k5_ref.py (header).
use crate::connors::{sma, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

/// Admitted ids (authors/jlcava/admission/k3c_ref.py (header)).
pub const REVIEWED: &[&str] = &["jcsis_tres", "jcsis_rsi_mom", "jcsis_tres_barras"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] The book gives no fill: signals and fills at the close. 'Pendiente positiva/negativa' = today's value above/below yesterday's. Volume indicator = 7-day simple average of +volume (close > open), -volume (close < open), 0 (equal), per the text (the OCR of the printed MetaStock line reads C>0). MBO = Mov(C,25,S)-Mov(C,200,S), the printed formula. The summary IX.1.3.3 prints 'pendiente positiva' for shorts too; the final rules IX.1.3.4 print 'pendiente negativa', used here. A close beyond the line ends the position and may open the opposite one on the same close if its filters hold. [Reading, open question] 'se coloque por encima' = any close above the line while flat with the other conditions true (p.cross = 1: only on the close that crosses the line). [Reading, open question] 'MBO cortado al alza' = MBO above zero (p.mbo_zero = 0: only its rising slope). The book applies it to very liquid stocks (largest IBEX names); US and NSE stocks and ETFs with volume are a declared application; series without volume cannot run. No stops are printed in the rule (the book refers to its stop chapters), so none is applied. [Not from the book] An exact tie is not a slope: MBO differences under 1e-9 count as zero (float rounding).";

/// Float rounding of the MBO (real differences are at least a price tick / (25 x 200), about 2e-8 at 4 decimals).
const TIE: f64 = 1e-9;

const NOTE_RM: &str = "[Declared readings] The book prints no periods: RSI(14) (Wilder's own period, the vault's Wilder RSI) and Momentum(10) = close minus the close 10 days earlier (Murphy's 10-day momentum; the book quotes Wilder and Murphy) are labelled defaults, other periods are options. Levels as printed: long when 50 < RSI < 70 and Momentum > 0, out when RSI < 50 or RSI > 70 or Momentum < 0; short when 30 < RSI < 50 and Momentum < 0, out when RSI > 50 or RSI < 30 or Momentum > 0. Signals and fills at the close (the book gives no fill). After an exit, a new trade needs all entry conditions again (so no re-entry while RSI stays above 70 / below 30); an exit may open the opposite side at the same close if its conditions hold. No stop is printed. Option p.mode = 1: the book's plain Momentum zero-line system (p.196: long above zero, short below zero, stop-and-reverse). The book applies its systems to Spanish stocks and futures; indices, ETFs and stocks, US and India, are a declared application.";
const NOTE_3B: &str = "[Declared readings] Daily bars only, as the book says. Bottom: the middle day's low is strictly below the lows of the day before and the day after (no overlap at that level), each of the three days has a range (high > low), and the third day closes above the middle day's high: buy at that close. Top: the mirror (middle day's high strictly above both neighbours, third day closes below the middle day's low): short at that close. [Reading] 'con el correspondiente stop' = a stop at the middle day's low (short: its high), no offset, working from the next day; a stop is filled at the stop price, or at the open if the day opens beyond it. An opposite three-bar signal closes the position at that close and reverses (fig.158). A same-side signal while in a trade changes nothing. The optional early entry before the close (p.157) needs intraday data and is not coded. The book applies the rule mainly to futures and calls cash markets 'less reliable'; indices, ETFs and stocks, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    let k5 = |id: &'static str, name: &'static str, rules: &'static [&'static str], notes: &'static str| StrategyDef { id, name, group: "Cava Sistemas de Especulación",
        session: 0, dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "jcsis", rules, notes, claim: None };
    vec![k5("jcsis_rsi_mom", "RSI + Momentum: long with RSI between 50 and 70 and Momentum above zero, short with RSI between 30 and 50 and Momentum below zero", &[
            "F|\"restar al precio de cierre actual, el precio de cierre de hace n periodos\" (PDF p.188) = Momentum; RSI of J. Welles Wilder (p.188) [reading] RSI(14), Momentum(10)",
            "E|\"Adoptaremos posiciones largas cuando el RSI supere el nivel 50 pero no haya llegado a 70 y el indicador de momento se encuentre por encima de cero.\" (PDF p.189)",
            "X|\"Cancelaremos las posiciones largas cuando el RSI caiga por debajo de 50, o supere el nivel 70, o el indicador de momento se sitúe por debajo de cero.\" (PDF p.189)",
            "E|\"Adoptaremos posiciones cortas cuando el RSI caiga por debajo de 50, pero sin llegar a 30 y el indicador de momento se sitúe por debajo de cero.\" (PDF p.189)",
            "X|\"Cancelaremos las posiciones cortas cuando el RSI suba por encima de 50 o el RSI caiga por debajo de 30 o el indicador de momento se sitúe por encima de cero.\" (PDF p.189)"], NOTE_RM),
        k5("jcsis_tres_barras", "Regla de las Tres Barras: three-day exhaustion turn, in at the third day's close, stop at the middle day's extreme, reverse on the opposite signal", &[
            "F|\"Esta regla sólo es de aplicación cuando trabajamos con gráficos diarios.\" \"En cada uno de los tres días, el precio debe fluctuar\" (PDF p.148)",
            "E|\"El mercado debería marcar un mínimo claro justo en el día intermedio ... no deberá producirse ningún solape a este nivel de precios con el rango de precios del día anterior ni con el posterior\" \"el precio de cierre del tercer día debería situarse por encima del máximo alcanzado en el día intermedio\" (PDF p.149); \"se deberían adoptar posiciones largas con el correspondiente stop\" (PDF p.151)",
            "E|Top: \"un máximo claro justo en el día intermedio\" (PDF p.149) and \"el precio de cierre del tercer día debería situarse por debajo del mínimo marcado el día intermedio\" (PDF p.151): short at the close with its stop (PDF p.152)",
            "X|[reading] stop at the middle day's low (short: its high); fig.158 \"Punto de entrada con stop por debajo de los mínimos de los días previos\" (PDF p.153)",
            "X|\"Los puntos 1, 3 y 5: Cancelación de posiciones largas y apertura de posiciones cortas. Los puntos 2, 4 y 6: Cancelación de posiciones cortas y adopción de posiciones largas.\" (fig.158, PDF p.153) = the opposite signal reverses"], NOTE_3B),
        StrategyDef { id: "jcsis_tres", name: "Tres indicadores (Strain): close beyond the 50-bar mid line with the 7-day volume average and the MBO sloping the same way", group: "Cava Sistemas de Especulación",
        session: 0, dir: "Long/Short", status: Status::Live, needs_volume: true, needs_vix: false, needs_series: &[], book: "jcsis", rules: &[
            "F|\"La fórmula para Metastock es: (HHV(H,50)+LLV(L,50))/2\" (PDF p.185); volume indicator \"una media simple de 7 días\" of signed volume (PDF p.186); MBO \"Mov(C,25,S)-Mov(C,200,S)\" (PDF p.187)",
            "E|\"Se adoptarán posiciones largas cuando el precio de cierre del periodo elegido se coloque por encima de la primera línea, el indicador de volumen presente pendiente positiva y el MBO esté cortado al alza y presente pendiente positiva.\" (PDF p.187)",
            "X|\"En tanto en cuanto el precio se mantenga por encima de la primera línea, las posiciones largas se mantendrán, cuando el precio se coloque por debajo deberían ser canceladas automáticamente.\" (PDF p.187)",
            "E|\"Se adoptarán posiciones cortas cuando el precio de cierre ... se coloque por debajo de la línea, el indicador de volumen presente pendiente negativa y el MBO esté cortado a la baja y presente una pendiente negativa.\" (PDF p.187); the short ends on a close back above the line"],
        notes: NOTE, claim: None }]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id {
        "jcsis_tres" => Some(("jcsis_tres", "Cava / Strain tres indicadores", "50-bar line, volume, MBO")),
        "jcsis_rsi_mom" => Some(("jcsis_rsi_mom", "Cava RSI + Momentum", "RSI 50/70/30, Momentum zero line")),
        "jcsis_tres_barras" => Some(("jcsis_tres_barras", "Cava Regla de las Tres Barras", "three-day exhaustion turn")),
        _ => None,
    }
}
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max| ParamDef { key, label, def, min, max, step: 1. };
    if id == "jcsis_rsi_mom" {
        return Some(vec![p("rsi", "RSI period (reading 14)", 14., 2., 50.), p("mom", "Momentum period (reading 10)", 10., 1., 50.),
            p("mid", "RSI middle line (book 50)", 50., 30., 70.), p("hi", "RSI upper level (book 70)", 70., 51., 95.), p("lo", "RSI lower level (book 30)", 30., 5., 49.),
            p("mode", "0 = RSI + Momentum (book), 1 = Momentum zero line only (book p.196)", 0., 0., 1.)]);
    }
    if id == "jcsis_tres_barras" { return Some(vec![]); }
    (id == "jcsis_tres").then(|| vec![p("n", "Line, bars (book 50)", 50., 2., 250.), p("vo", "Volume average, days (book 7)", 7., 1., 50.),
        p("fast", "MBO short average (book 25)", 25., 2., 100.), p("slow", "MBO long average (book 200)", 200., 10., 400.),
        p("cross", "0 = any close beyond the line (reading), 1 = only the crossing close", 0., 0., 1.), p("mbo_zero", "1 = MBO beyond zero and sloping (reading), 0 = sloping only", 1., 0., 1.)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    match id { "jcsis_rsi_mom" => return Some(rsi_mom(x, from, to, p)), "jcsis_tres_barras" => return Some(three_bars(x, from, to)), _ => {} }
    let s = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| s.iter().find(|d| d.key == k).unwrap().def);
    let u = |k: &str| g(k).round().max(1.) as usize;
    let (c, o, h, l, n) = (&x.c, &x.o, &x.h, &x.l, u("n"));
    let (cross, zero) = (g("cross") >= 0.5, g("mbo_zero") >= 0.5);
    let line: Vec<f64> = (0..c.len()).map(|i| if i + 1 < n { f64::NAN } else {
        (h[i + 1 - n..=i].iter().cloned().fold(f64::MIN, f64::max) + l[i + 1 - n..=i].iter().cloned().fold(f64::MAX, f64::min)) / 2. }).collect();
    let signed: Vec<f64> = (0..c.len()).map(|i| x.r[i].volume.filter(|v| *v > 0.).unwrap_or(f64::NAN) * if c[i] > o[i] { 1. } else if c[i] < o[i] { -1. } else { 0. }).collect();
    let (vol, fast, slow) = (sma(&signed, u("vo")), x.sma_n(u("fast")), x.sma_n(u("slow")));
    let mbo: Vec<f64> = (0..c.len()).map(|i| fast[i] - slow[i]).collect();
    let mk = |d: f64, e: usize, xi: usize, why: &str, open: bool| Trade { dir: d, legs: vec![Leg { i: e, px: c[e], w: 1. }], exit_i: xi, exit_px: c[xi], reason: why.into(), entry_kind: "close", signal_i: xi, open, exits: vec![] };
    let (mut out, mut cur) = (vec![], None::<(f64, usize)>);
    for i in from.max(1)..=to {
        if let Some((d, e)) = cur {
            if (c[i] - line[i]) * d < 0. { out.push(mk(d, e, i, "close beyond the line", false)); cur = None; }
        }
        if cur.is_some() || i == to { continue; }
        for d in [1., -1.] {
            let beyond = (c[i] - line[i]) * d > 0. && (!cross || (c[i - 1] - line[i - 1]) * d <= 0.);
            // the average's slope is exactly the sign of new minus dropped volume; MBO ties within rounding are not a slope
            let vol_ok = vol[i].is_finite() && vol[i - 1].is_finite() && (signed[i] - signed[i - u("vo")]) * d > 0.;
            if beyond && vol_ok && (mbo[i] - mbo[i - 1]) * d > TIE && (!zero || mbo[i] * d > TIE) { cur = Some((d, i)); }
        }
    }
    if let Some((d, e)) = cur { out.push(mk(d, e, to, "open", true)); }
    Some(out)
}

fn close_trade(d: f64, e: usize, ep: f64, xi: usize, xp: f64, why: &str, open: bool) -> Trade {
    Trade { dir: d, legs: vec![Leg { i: e, px: ep, w: 1. }], exit_i: xi, exit_px: xp, reason: why.into(), entry_kind: "close", signal_i: xi, open, exits: vec![] }
}

/// IX.1.4.1: entries and exits at the close; a flat trader takes a side only when all its entry conditions hold.
fn rsi_mom(x: &Ctx, from: usize, to: usize, p: &Params) -> Vec<Trade> {
    let s = params("jcsis_rsi_mom").unwrap();
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| s.iter().find(|d| d.key == k).unwrap().def);
    let (rn, mn) = (g("rsi").round().max(1.) as usize, g("mom").round().max(1.) as usize);
    let (mid, hi, lo, zero) = (g("mid"), g("hi"), g("lo"), g("mode") >= 0.5);
    let (c, rs) = (&x.c, x.rsi_n(rn));
    let mom = |i: usize| if i >= mn { c[i] - c[i - mn] } else { f64::NAN };
    // side wanted by a flat trader, and whether a position of side d must close (NaN compares false: no signal)
    let entry = |i: usize| if zero { if mom(i) > 0. { 1. } else if mom(i) < 0. { -1. } else { 0. } }
        else if rs[i] > mid && rs[i] < hi && mom(i) > 0. { 1. } else if rs[i] < mid && rs[i] > lo && mom(i) < 0. { -1. } else { 0. };
    let exit = |i: usize, d: f64| if zero { mom(i) * d < 0. } else if d > 0. { rs[i] < mid || rs[i] > hi || mom(i) < 0. } else { rs[i] > mid || rs[i] < lo || mom(i) > 0. };
    let (mut out, mut cur) = (vec![], None::<(f64, usize)>);
    for i in from..=to {
        if let Some((d, e)) = cur {
            if exit(i, d) { out.push(close_trade(d, e, c[e], i, c[i], if zero { "Momentum crossed zero" } else { "RSI / Momentum exit" }, false)); cur = None; }
        }
        if cur.is_none() && i < to {
            let d = entry(i);
            if d != 0. { cur = Some((d, i)); }
        }
    }
    if let Some((d, e)) = cur { out.push(close_trade(d, e, c[e], to, c[to], "open", true)); }
    out
}

/// VII.2: the three-bar exhaustion turn on bars i-2, i-1, i (signal at bar i's close): +1 bottom, -1 top.
fn three_bar_signal(x: &Ctx, i: usize) -> f64 {
    let (h, l, c) = (&x.h, &x.l, &x.c);
    if i < 2 || (i - 2..=i).any(|k| !(h[k] > l[k])) { return 0.; }
    if l[i - 1] < l[i - 2] && l[i - 1] < l[i] && c[i] > h[i - 1] { 1. } else if h[i - 1] > h[i - 2] && h[i - 1] > h[i] && c[i] < l[i - 1] { -1. } else { 0. }
}
fn three_bars(x: &Ctx, from: usize, to: usize) -> Vec<Trade> {
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let (mut out, mut cur) = (vec![], None::<(f64, usize, f64)>); // (side, entry bar, stop)
    for i in from..=to {
        if let Some((d, e, st)) = cur {
            if d > 0. && l[i] <= st { out.push(close_trade(d, e, c[e], i, o[i].min(st), "stop at the middle day's low", false)); cur = None; }
            else if d < 0. && h[i] >= st { out.push(close_trade(d, e, c[e], i, o[i].max(st), "stop at the middle day's high", false)); cur = None; }
        }
        let s = three_bar_signal(x, i);
        if s == 0. { continue; }
        if let Some((d, e, _)) = cur {
            if d == s { continue; }
            out.push(close_trade(d, e, c[e], i, c[i], "opposite three-bar signal", false));
            cur = None;
        }
        if i < to { cur = Some((s, i, if s > 0. { l[i - 1] } else { h[i - 1] })); }
    }
    if let Some((d, e, _)) = cur { out.push(close_trade(d, e, c[e], to, c[to], "open", true)); }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn long_needs_line_volume_and_mbo_exit_on_close_below_line() {
        // accelerating rise: above the line, MBO > 0 and rising; volume grows only from bar 240, so the entry waits for it
        let px = |i: usize| 100. + 0.002 * (i * i) as f64;
        let mut b: Vec<(f64, f64)> = (0..240).map(|i| (px(i), 100.)).collect();
        b.extend((240..245).map(|i| (px(i), 100. + (i - 239) as f64 * 100.)));
        b.extend((0..30).map(|i| (215. - i as f64 * 3., 100.))); // fall through the 50-bar mid line
        let r: Vec<Row> = b.iter().enumerate().map(|(i, &(c, v))| Row { ticker: "T".into(), date: format!("2001-01-{:02}", i % 28 + 1), open: c - 0.25, high: c + 0.5, low: c - 0.5, close: c, volume: Some(v) }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let t = run("jcsis_tres", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert_eq!(t[0].dir, 1.);
        assert_eq!(t[0].legs[0].i, 240);
        assert!(!t[0].open && t[0].exit_i > 245 && t[0].exit_i < 275);
        assert!(run("jcsis_x", &x, 0, r.len() - 1, &Params::new()).is_none());
    }
    fn rows(b: &[(f64, f64, f64, f64)]) -> Vec<Row> {
        b.iter().enumerate().map(|(i, &(o, h, l, c))| Row { ticker: "T".into(), date: format!("2001-{:02}-{:02}", i / 28 + 1, i % 28 + 1), open: o, high: h, low: l, close: c, volume: None }).collect()
    }
    #[test]
    fn three_bars_buy_reverse_and_stop() {
        let r = rows(&[(10., 11., 9., 10.), (10., 10.5, 8., 9.), (9., 12., 8.5, 11.5), (11.5, 12.5, 11., 12.), (12., 13., 11.5, 12.5),
            (12.5, 12.8, 10., 10.2), (10., 13.5, 9.8, 13.), (12., 12., 11., 11.5)]);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("jcsis_tres_barras", &x, 0, r.len() - 1, &Params::new()).unwrap();
        let got: Vec<_> = t.iter().map(|t| (t.dir, t.legs[0].i, t.legs[0].px, t.exit_i, t.exit_px, t.open)).collect();
        assert_eq!(got, vec![(1., 2, 11.5, 5, 10.2, false), (-1., 5, 10.2, 6, 13., false)]);
    }
    #[test]
    fn rsi_momentum_entries_hold_the_printed_conditions() {
        let cl: Vec<f64> = (0..400).map(|i| 100. + 10. * ((i as f64) / 9.).sin() + 0.05 * i as f64).collect();
        let r = rows(&cl.iter().map(|&c| (c, c + 1., c - 1., c)).collect::<Vec<_>>());
        let x = Ctx::new(&r, &r, &[]);
        let t = run("jcsis_rsi_mom", &x, 0, r.len() - 1, &Params::new()).unwrap();
        let rs = x.rsi_n(14);
        assert!(t.iter().any(|t| t.dir > 0.) && t.iter().any(|t| t.dir < 0.));
        for t in &t {
            let (e, m) = (t.legs[0].i, cl[t.legs[0].i] - cl[t.legs[0].i - 10]);
            if t.dir > 0. { assert!(rs[e] > 50. && rs[e] < 70. && m > 0.); } else { assert!(rs[e] < 50. && rs[e] > 30. && m < 0.); }
        }
        // zero-line option: always in after the first signal, each trade ends where the next begins
        let mut p = Params::new();
        p.insert("mode".into(), 1.);
        let z = run("jcsis_rsi_mom", &x, 0, r.len() - 1, &p).unwrap();
        assert!(z.windows(2).all(|w| w[0].exit_i == w[1].legs[0].i && w[0].dir == -w[1].dir));
    }
}

// Instrument admission: José Luis Cava (Sistemas de Especulación en Bolsa) 2026-10-30 (titles lane K3) block, moved from instrument_policy.rs on 31 Oct 2026 (lane ST).
// Admission only for the module REVIEWED lists (authors/jlcava/admission/k3c_ref.py (header)); kept in one block to merge.
// Runs on actual indices, ETFs and stocks, US and India (index_etf_or_stock, nifty50 basket, declared application).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book=="jcsis" && def.id == "jcsis_tres",
    label: "The book applies this system to very liquid Spanish stocks; US and NSE stocks, ETFs and any index with volume are a declared application, not the author's test.",
    basis: "José Luis Cava, Sistemas de Especulación en Bolsa (2001), section IX.1.3, PDF p.184-187",
};
// Lane K5 (31 Oct 2026): RSI + Momentum and the Three-Bar Rule (authors/jlcava/admission/k5/k5_ref.py (header)).
pub const ADMISSION_K5: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book=="jcsis" && def.id != "jcsis_tres" && crate::books::jcsis::reviewed(def.id),
    label: "The book shows these rules on Spanish stocks and futures (the Three-Bar Rule mainly on futures, 'less reliable' on cash markets); indices, ETFs and stocks, US and India, are a declared application, not the author's test.",
    basis: "José Luis Cava, Sistemas de Especulación en Bolsa (2001), IX.1.4.1 RSI + Momentum (PDF p.188-189) and VII.2 Regla de las Tres Barras (PDF p.148-153)",
};
