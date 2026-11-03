//! Alan R. Del Aguila (Futuros Trading LLC, futuros.com), "Manual Avanzado de como invertir en Futuros" (1994), chapter
//! "Poniendolo todo Junto", section "A Corto Plazo" (research share USB HDD 1/TRADING KNOWLEDGE-2/_Libros y Cursos en Español/
//! Manual Avanzado de como inverir en Futuros (futuros.com)/curso sobre futuros.pdf, PDF p.69-71 = printed p.68-70; extract
//! authors/batch-e/E1w/delaguila_rsi5.txt). Book id `adrsi5`. Lane K10 (titles loop, 1 Nov 2026), to-code row 176;
//! independent check authors/titles-1030/K10w/k10_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::rbtie::mk;
use super::msws::rsi;

/// Admitted ids (authors/titles-1030/K10w/k10_ref.py (header)).
pub const REVIEWED: &[&str] = &["adrsi5_rsi5"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Stop in $ per contract held] The $500 initial stop per futures contract is not applied and not converted (lead decisions 19, 31, 41). [Declared readings] Wilder RSI(5) on daily closes. Signal day = the first close with RSI(5) at or below 20 after a close above 20 ('espere hasta que el indicador entre en la zona de 20 para abajo'); buy at the next day's close if that close is below the signal day's close ('al día siguiente ... si es que los precios siguieron bajando'), otherwise no trade. Short is the exact opposite (RSI(5) entering 80 or above, a higher close the next day). Profit is taken at the close of the 2nd day after the entry day ('al segundo día de entrado al mercado'; the 3rd day, which the author says suits some markets, is p.hold = 3). Breakeven: after any close beyond the entry price, a stop at the entry price works from the next day (filled at the entry price, or at the open on a gap through it), checked before the time exit. One position; no entry on a day a position closes. Options: the author's 30/70 bands for quiet markets. The author trades futures (best on the S&P 500; examples yen and heating oil, no statistics); indices, ETFs and US/India stocks are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "adrsi5_rsi5", name: "Del Aguila short-term RSI(5) 20/80: buy the day after RSI enters 20 on a lower close, out on the 2nd day (breakeven stop; $500 stop held)", group: "Manual Avanzado de Futuros (A Corto Plazo)",
        session: 0, dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "adrsi5", rules: &[
            "F|\"1. Utilice el RSI a cinco días en los cuadros diarios. 2. Las bandas para la compra y venta tienen que ser 20 y 80 respectivamente.\" (p.68)",
            "E|\"3. Para la compra espere hasta que el indicador entre en la zona de 20 para abajo. 4. La compra se realizará al día siguiente que el indicador entró a la zona de los 20 y si es que los precios siguieron bajando. 5. La venta es exactamente lo opuesto.\" (p.68)",
            "X|\"6. Ganancias se tomaran al segundo día de entrado al mercado. ... 7. El stop inicial debe situarse a $500 dólares del precio de entrada. 8. Cuando los precios se muevan a nuestro favor el stop debe moverse al precio de entrada.\" (p.68)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == "adrsi5_rsi5").then_some(("adrsi5_rsi5", "Del Aguila RSI(5)", "20/80, 2nd day"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("lo", "Buy zone: RSI(5) at or below (20; quiet markets 30)", 20., 5., 45., 1.),
        p("hi", "Sell zone: RSI(5) at or above (80; quiet markets 70)", 80., 55., 95., 1.), p("hold", "Exit at the close of day N after entry (2; 3 for some markets)", 2., 1., 10., 1.)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap().def);
    let (lo, hi, hold) = (g("lo"), g("hi"), g("hold").round().max(1.) as usize);
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let r = rsi(c, 5);
    let (mut out, mut cur, mut be) = (vec![], None::<(f64, usize, f64)>, false);
    for i in from.max(2)..=to {
        if let Some((d, e, px)) = cur {
            let hit = be && (if d > 0. { l[i] <= px } else { h[i] >= px });
            if hit || i >= e + hold {
                let (xp, why) = if hit { (if d > 0. { o[i].min(px) } else { o[i].max(px) }, "breakeven stop") } else { (c[i], "day N after entry") };
                out.push(mk(d, e, px, "close", i, xp, why, false, i));
                cur = None; be = false;
            } else if d * (c[i] - px) > 0. { be = true; }
            continue;
        }
        let s = i - 1;
        if r[s] <= lo && r[s - 1] > lo && c[i] < c[s] { cur = Some((1., i, c[i])); }
        else if r[s] >= hi && r[s - 1] < hi && c[i] > c[s] { cur = Some((-1., i, c[i])); }
    }
    if let Some((d, e, px)) = cur { out.push(mk(d, e, px, "close", to, c[to], "open", true, to)); }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn enters_after_the_zone_and_exits_on_day_two_or_breakeven() {
        let r: Vec<Row> = (0..900).map(|i| { let f = i as f64; let c = 100. + 6. * (f / 5.).sin() + 2. * (f / 1.7).cos();
            Row { ticker: "T".into(), date: format!("{:05}", i), open: c + 0.3, high: c + 1., low: c - 1., close: c, volume: None } }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let t = run("adrsi5_rsi5", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() > 4 && t.iter().any(|z| z.dir > 0.) && t.iter().any(|z| z.dir < 0.));
        for z in t.iter().filter(|z| !z.open) { assert!(z.exit_i > z.legs[0].i && z.exit_i <= z.legs[0].i + 2); }
        assert!(run("adrsi5_x", &x, 0, 5, &Params::new()).is_none());
    }
}

// Instrument admission: Alan R. Del Aguila, Manual Avanzado de Futuros 2026-11-01 (titles lane K10).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "adrsi5" && reviewed(def.id),
    label: "Alan Del Aguila's short-term RSI system is for futures (best on the S&P 500); its $500 stop is held. Runs on indices, ETFs and stocks, US and India, are a declared application.",
    basis: "Alan R. Del Aguila, Manual Avanzado de como invertir en Futuros (futuros.com, 1994) printed p.68-70 (PDF p.69-71); independent check authors/titles-1030/K10w/k10_ref.py",
};
