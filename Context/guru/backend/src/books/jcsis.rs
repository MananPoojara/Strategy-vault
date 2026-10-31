//! José Luis Cava, "Sistemas de Especulación en Bolsa" (2001; research share USB HDD 1/TRADING KNOWLEDGE-2/_Libros y
//! Cursos en Español/José Luis Cava - Sistemas de Especulación en Bolsa/DBZ_J.Cava- Sistemas de Especulación en Bolsa.pdf,
//! scan, OCR authors/titles-1030/R2w/txt/5845c.txt). Book id `jcsis`. One rule: section IX.1.3 'Tres indicadores' (PDF
//! p.184-187, printed p.192-195), Bryan Strain's line + volume indicator + the MBO of Mark Whitley and Bryan Strain.
//! `djcs_tfs` is James Taylor's variation of the same TFS without the MBO: not changed, this is a separate id.
//! Lane K3 (titles loop, 30 Oct 2026); independent check authors/jlcava/admission/k3c_ref.py (header).
use crate::connors::{sma, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

/// Admitted ids (authors/jlcava/admission/k3c_ref.py (header)).
pub const REVIEWED: &[&str] = &["jcsis_tres"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] The book gives no fill: signals and fills at the close. 'Pendiente positiva/negativa' = today's value above/below yesterday's. Volume indicator = 7-day simple average of +volume (close > open), -volume (close < open), 0 (equal), per the text (the OCR of the printed MetaStock line reads C>0). MBO = Mov(C,25,S)-Mov(C,200,S), the printed formula. The summary IX.1.3.3 prints 'pendiente positiva' for shorts too; the final rules IX.1.3.4 print 'pendiente negativa', used here. A close beyond the line ends the position and may open the opposite one on the same close if its filters hold. [Reading, open question] 'se coloque por encima' = any close above the line while flat with the other conditions true (p.cross = 1: only on the close that crosses the line). [Reading, open question] 'MBO cortado al alza' = MBO above zero (p.mbo_zero = 0: only its rising slope). The book applies it to very liquid stocks (largest IBEX names); US and NSE stocks and ETFs with volume are a declared application; series without volume cannot run. No stops are printed in the rule (the book refers to its stop chapters), so none is applied. [Not from the book] An exact tie is not a slope: MBO differences under 1e-9 count as zero (float rounding).";

/// Float rounding of the MBO (real differences are at least a price tick / (25 x 200), about 2e-8 at 4 decimals).
const TIE: f64 = 1e-9;

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "jcsis_tres", name: "Tres indicadores (Strain): close beyond the 50-bar mid line with the 7-day volume average and the MBO sloping the same way", group: "Cava Sistemas de Especulación",
        session: 0, dir: "Long/Short", status: Status::Live, needs_volume: true, needs_vix: false, needs_series: &[], book: "jcsis", rules: &[
            "F|\"La fórmula para Metastock es: (HHV(H,50)+LLV(L,50))/2\" (PDF p.185); volume indicator \"una media simple de 7 días\" of signed volume (PDF p.186); MBO \"Mov(C,25,S)-Mov(C,200,S)\" (PDF p.187)",
            "E|\"Se adoptarán posiciones largas cuando el precio de cierre del periodo elegido se coloque por encima de la primera línea, el indicador de volumen presente pendiente positiva y el MBO esté cortado al alza y presente pendiente positiva.\" (PDF p.187)",
            "X|\"En tanto en cuanto el precio se mantenga por encima de la primera línea, las posiciones largas se mantendrán, cuando el precio se coloque por debajo deberían ser canceladas automáticamente.\" (PDF p.187)",
            "E|\"Se adoptarán posiciones cortas cuando el precio de cierre ... se coloque por debajo de la línea, el indicador de volumen presente pendiente negativa y el MBO esté cortado a la baja y presente una pendiente negativa.\" (PDF p.187); the short ends on a close back above the line"],
        notes: NOTE, claim: None }]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == "jcsis_tres").then_some(("jcsis_tres", "Cava / Strain tres indicadores", "50-bar line, volume, MBO"))
}
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max| ParamDef { key, label, def, min, max, step: 1. };
    (id == "jcsis_tres").then(|| vec![p("n", "Line, bars (book 50)", 50., 2., 250.), p("vo", "Volume average, days (book 7)", 7., 1., 50.),
        p("fast", "MBO short average (book 25)", 25., 2., 100.), p("slow", "MBO long average (book 200)", 200., 10., 400.),
        p("cross", "0 = any close beyond the line (reading), 1 = only the crossing close", 0., 0., 1.), p("mbo_zero", "1 = MBO beyond zero and sloping (reading), 0 = sloping only", 1., 0., 1.)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
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
}

// Instrument admission: José Luis Cava (Sistemas de Especulación en Bolsa) 2026-10-30 (titles lane K3) block, moved from instrument_policy.rs on 31 Oct 2026 (lane ST).
// Admission only for the module REVIEWED lists (authors/jlcava/admission/k3c_ref.py (header)); kept in one block to merge.
// Runs on actual indices, ETFs and stocks, US and India (index_etf_or_stock, nifty50 basket, declared application).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book=="jcsis" && crate::books::jcsis::reviewed(def.id),
    label: "The book applies this system to very liquid Spanish stocks; US and NSE stocks, ETFs and any index with volume are a declared application, not the author's test.",
    basis: "José Luis Cava, Sistemas de Especulación en Bolsa (2001), section IX.1.3, PDF p.184-187",
};
