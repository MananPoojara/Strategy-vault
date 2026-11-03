//! J. Welles Wilder Jr., New Concepts in Technical Trading Systems (1978), read in the Spanish edition
//! "Nuevos Conceptos Sobre Sistemas Técnicos de Operación en Bolsa" (65-sheet scan, two printed pages per sheet).
//! Page refs are the book's printed pages. Only rules the book states exactly are Live; every default is the
//! book's literal value; where the book is silent the neutral reading is labelled in `notes`.
//! Sources, testability table and the independent oracle: library/wwnc/.
use crate::connors::{true_range, wilder, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "wwnc";
/// Admitted ids (authors/wilder/admission/SOURCE-CONTRACT.md).
pub const REVIEWED: &[&str] = &["wwnc_vol", "wwnc_dmi", "wwnc_tbp", "wwnc_rt"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, status, rules, notes| StrategyDef {
        id, name, group, session: 0, dir: "Long/Short", status, needs_volume: false, needs_vix: false, needs_series: &[],
        book: BOOK, rules, notes, claim: None,
    };
    use Status::*;
    vec![
        d("wwnc_vol", "Volatility System (SAR at ARC from the significant close)", "Sec III · Volatility System", Live, &[
            "F|\"ATR ... se obtiene tomando la suma de las amplitudes de los siete primeros días y dividiendo ... entre siete ... Para el octavo y sucesivos ... ATR del día anterior, multiplicarlo por seis, añadir la amplitud auténtica del último día y dividir ... entre siete\" (p.25-27)",
            "F|\"C - CONSTANTE Cualquier cifra comprendida entre 2,80 y 3,10\"; \"He llegado a la conclusión que el multiplicador idóneo es 3,0\" (p.25, p.27). ARC = ATR x C",
            "F|\"SIC - Cierre Significativo; la cotización favorable extrema del cierre alcanzada durante una operación\"; \"SAR - punto definido por la distancia de la ARC desde el SIC\" (p.27)",
            "E|\"La ENTRADA se hace en el cierre cuando la cotización cierra de modo contrario al SAR\" (Regla 1, p.27)",
            "X|\"De Subida a Bajada: en el Cierre, cuando el cierre está por debajo del SAR\"; \"De Bajada a Subida: en el Cierre, cuando ... el cierre está por encima del SAR\" (Regla 2, p.27): stop and reverse, always in the market",
        ], "Worksheet p.28-30 reproduced exactly in the unit test (long day 16 at 52.50, short day 46 at 56.05). Start-up: the book's worksheet assumes the current trade is short and takes the lowest close of the first 7 days as the SIC (p.30, Día 7); p.init=1 starts from a virtual long instead. The state machine runs from the first bar of the series; trades are reported from the first reversal inside the window. Futures system in the book (gold, p.32); run on the index (instrument swap). The book adds 'trade the markets high on the CSI' (p.30): market selection, not a rule of the system."),
        d("wwnc_dmi", "Directional Movement System (+DI14/-DI14 cross with the extreme point rule)", "Sec IV · Directional Movement", Live, &[
            "F|+DM = today's high - yesterday's high, -DM = yesterday's low - today's low; only the larger counts, inside day = 0; TR14/+DM14/-DM14 smoothed \"X - X/14 + hoy\"; +DI14 = +DM14/TR14, -DI14 = -DM14/TR14, \"hagamos caso omiso del decimal\" (p.36-44)",
            "E|\"Cuando +DI14 cruza por encima de -DI14 se adopta una posición Al Alza. Esta posición se invierte cuando -DI14 cruza por encima de +DI14\" (p.48)",
            "E|Regla del Punto Máximo: \"El día en que +DI14 y -DI14 se crucen utilice como punto de cambio la cotización máxima alcanzada ese día. Si se está Al Alza el punto de cambio es el Mínimo ... A la Baja ... el Máximo\" (p.48-49)",
            "X|\"Manténgase en este punto, si no lo saca un stop de la operación, incluso en el caso de que los índices permanezcan cruzados a contrario de la posición ... durante varios días\" (p.49): the reverse point is a stop-and-reverse order; always in the market after the first fill",
        ], "Reading: the reverse point is a stop order from the day after the cross; it is cancelled if the DIs cross back to agree with the position (p.49 example: '+DI14 igualó al -DI14 pero no lo cruzó ... no se señaló acción', and the 21-Jan short was kept because the cross-day high was never penetrated). 'Penetrated' = traded strictly beyond the point; a gap through it fills at the open. DIs are truncated to integers as the book does (p.44); p.int_di=0 uses full precision. A first cross from flat sets the same entry point. Optional ADX turning-point profit taking (p.49) is a separate, not-testable item. Futures (wheat, p.50; flour, p.52); run on the index."),
        d("wwnc_tbp", "Trend Balance Point System (momentum factor)", "Sec V · Momentum", Live, &[
            "F|\"El factor momento es la diferencia entre la cotización al cierre de hoy y la cotización al cierre de hace dos días\" (p.55)",
            "E|\"Ir Al Alza en el cierre de hoy cuando el factor momento del día sea una cifra superior al factor momento de los dos días anteriores\"; \"Ir A la Baja ... cuando ... sea una cifra menor\" (p.55-56)",
            "E|TBP: long -> \"seleccione el más bajo de los dos factores momento anteriores y súmelo al cierre de ayer\" (stop and reverse at the close below it); short -> \"el más alto\" (reverse at the close above it); \"Siempre que el cierre sea exactamente igual al TBP continuaremos operando en la misma dirección\" (p.57, p.59)",
            "X|STOP (no cambia): long X̄ - TR, short X̄ + TR, X̄ = (H+L+C)/3, computed after the close for the next day (p.58-59)",
            "X|OBJETIVO (salida sólo, no cambio): long 2X̄ - L, short 2X̄ - H, computed after the close for the next day (p.58-59)",
            "A|REENTRADA: \"Vuelva a entrar al cierre según indique el Punto de Equilibrio de la Tendencia después de haber salido en el Stop o en el Objetivo\" (p.59)",
        ], "Worksheet p.62 (Aug-Sep 1977) reproduced in the unit test: short 19-Aug at 210.80, targets 208.00 / 200.50 / 197.87, long at the 25-Aug close 196.30, target 200.87. Stop and target on the same day: stop assumed first (book silent). Stop is touched (low <= stop); target reached (high >= target); a gap beyond either fills at the open. After an intraday exit the re-entry at the close uses the TBP of the side just exited (p.57: 'iremos Al Alza si el cierre está por encima de 49,00 y A la Baja si es igual o inferior')."),
        d("wwnc_rt", "Reaction Trend System (B/O/S phases, HBOP/LBOP breakouts)", "Sec VII · Reaction Trend", Live, &[
            "F|X̄ = (H+L+C)/3; B1 = 2X̄ - H; S1 = 2X̄ - L; HBOP = 2X̄ - 2L + H; LBOP = 2X̄ - 2H + L, from yesterday's bar for today (p.73-75)",
            "F|FASES: days named B, O, S in rotation; \"Siempre que la cotización penetre uno de los puntos ... HBOP o LBOP ... (A) ... Al Alza en el MODO DE TENDENCIA, bautizar el día más alto como 'S' ... (B) ... A la Baja ... el día más bajo como 'B'\"; \"La fase inicial puede determinarse también a través de la Regla (3)\" (p.77)",
            "E|MODO DE REACCIÓN: \"Al Alza en B1, solamente día 'B'\"; \"A la Baja en S1, solamente en un día 'S'\" (p.77)",
            "X|Long: \"En S1 en un día 'O'\"; \"AL CIERRE en un día 'S' si S1 ... no ha sido alcanzado\"; reverse \"En S1 en un día 'S'\" or \"En el LBOP cualquier día\"; \"No salir del mercado en un día de entrada excepto en el LBOP\" (p.77-78)",
            "X|Short: \"AL CIERRE en un día 'B' si B1 ... no ha sido alcanzado\"; reverse \"En B1 en un día 'B'\" or \"En el HBOP cualquier día\"; no exit on the entry day except HBOP (p.77-78)",
            "E|MODO DE TENDENCIA: \"Al Alza en el HBOP en cualquier día\"; \"A la Baja en el LBOP en cualquier día\"; \"Cambiar al MODO DE TENDENCIA cualquier día en que la cotización cruce un punto de ruptura HBOP o LBOP\" (p.77-78)",
            "X|Trend mode exit: trailing stop = lowest low of the two previous days (long) / highest high of the two previous days (short), \"stop solamente, no es de cambio\"; then adjust the phases and restart in reaction mode (p.77-78)",
        ], "Start-up: the book's first phase is either a judged 'significant low' (not testable) or Regla (3); only Regla (3) is used, so the system trades breakouts only until the first trend-mode trade ends and fixes the phases. Intraday order sequence (book silent): TradeStation's bar path (open, then the nearer of high/low, then the other, then the close); stops before limits when both are already through at the open. Trailing stop starts the day after entry; after a trailing-stop exit no new order the same day. Ties for the highest/lowest day take the latest. The book's 'add a few cents to HBOP/LBOP/trailing stop' (p.82) is a live-trading allowance, not modelled (p.margin, default 0). Worksheet p.80-81 checked in the unit test."),
        d("wwnc_rsi", "Relative Strength Index (interpretation)", "Sec VI · RSI", NotTestable("The book defines RSI(14) and five chart-reading uses (tops/bottoms beyond 70/30, chart formations, failure swings, support/resistance, divergence) but gives no exit rule, so no mechanical system exists (p.63-70)."), &[
            "F|RSI = 100 - 100/(1+RS), RS = average up close / average down close over 14 days, Wilder smoothing (p.66-68)",
            "E|\"Oscilaciones de fallo ('Failure Swings'): Cuando rebasan 70 o caen por debajo de 30 son fuertes indicaciones del final de la reacción\" (p.69): no exit given",
        ], "Not testable: no exit (user rule: a rule without an exit is not testable)."),
        d("wwnc_asi", "Swing Index System (ASI swing points, 60-point trailing SAR)", "Sec VIII · Swing Index", NotTestable("The Swing Index divides by the futures daily limit move L (SI = 50 x N/R x K/L, p.88-92); a cash index has no limit move, so the 60-ASI-point trailing SAR has no defined scale, and the entries use the 'significant' previous HSP/LSP, a judgement (p.98-102)."), &[
            "F|SI = 50 [C2 - C1 + 0.5(C2 - O2) + 0.25(C1 - O1)] / R x K / L, L = limit move (p.88-92)",
            "E|\"Entrar en posición Al Alza cuando el ASI cruza por encima del anterior HSP significativo\" (p.102)",
            "X|Index SAR: the previous LSP (HSP) after the switch, then the first LSP (HSP) after a new HSP (LSP); trailing SAR 60 ASI points from the most favourable ASI (p.98-102)",
        ], "Same reason as stad6_asi in the vault."),
        d("wwnc_csi", "Commodity Selection Index", "Sec IX · CSI", NotTestable("Ranks futures by ADXR x ATR14 with each contract's dollar value per point, margin and commission (p.104-116): needs a futures universe with contract specifications, not an index."), &[
            "F|CSI = ADXR x ATR14 x [V / sqrt(M) x 1/(150 + C)] x 100; trade the five or six commodities highest on the CSI (p.104-116)",
        ], "Market selection across a futures portfolio."),
        d("wwnc_dmi_adx_exit", "DMI with ADX turning-point exit (optional)", "Sec IV · Directional Movement", NotTestable("The ADX turning-point exit is offered as an option ('No hay nada de malo en salir ... y volver a entrar en la dirección del siguiente cruce ... o si la línea del ADX vuelve a ascender', p.49) without an exit price or a re-entry price/direction rule."), &[
            "X|\"el punto de inflexión se produce frecuentemente a la vez que la primera inflexión descendente de la línea del ADX después de que éste haya cruzado por encima de las dos líneas de los DI\" (p.48-49)",
        ], "Not testable: re-entry rule incomplete."),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "wwnc_vol" => ("wwnc_vol", "Wilder Volatility System", "Book"),
        "wwnc_dmi" | "wwnc_dmi_adx_exit" => ("wwnc_dmi", "Wilder Directional Movement", if id == "wwnc_dmi" { "Extreme point rule" } else { "ADX exit · Not testable" }),
        "wwnc_tbp" => ("wwnc_tbp", "Wilder Trend Balance Point", "Book"),
        "wwnc_rt" => ("wwnc_rt", "Wilder Reaction Trend", "Book"),
        "wwnc_rsi" => ("wwnc_rsi", "Wilder RSI", "Not testable"),
        "wwnc_asi" => ("wwnc_asi", "Wilder Swing Index", "Not testable"),
        "wwnc_csi" => ("wwnc_csi", "Wilder CSI", "Not testable"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(match id {
        "wwnc_vol" => vec![p("atr_n", "ATR days", 7., 3., 30., 1.), p("c", "Constant C (ARC = ATR x C)", 3.0, 2.8, 3.1, 0.05),
            p("init", "Start-up virtual trade (-1 short as the book's worksheet, 1 long)", -1., -1., 1., 2.)],
        "wwnc_dmi" => vec![p("n", "DI days", 14., 5., 40., 1.), p("int_di", "Drop DI decimals as the book (1) or not (0)", 1., 0., 1., 1.)],
        "wwnc_tbp" => vec![p("mf_lag", "Momentum factor: close minus close N days ago", 2., 1., 10., 1.)],
        "wwnc_rt" => vec![p("trail_n", "Trend-mode trailing stop: extreme of the previous N days", 2., 1., 10., 1.),
            p("margin", "Points added beyond HBOP/LBOP and the trailing stop (book p.82: 'varias centésimas'; 0 = exact)", 0., 0., 5., 0.05)],
        _ => return None,
    })
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if from > to || to >= x.r.len() { return None; }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let u = |k: &str| g(k).round().max(1.) as usize;
    Some(match id {
        "wwnc_vol" => vol_system(x, from, to, u("atr_n"), g("c"), if g("init") > 0. { 1. } else { -1. }),
        "wwnc_dmi" => dmi_system(x, from, to, u("n"), g("int_di") >= 0.5),
        "wwnc_tbp" => tbp_system(x, from, to, u("mf_lag")),
        "wwnc_rt" => rt_system(x, from, to, u("trail_n"), g("margin"), None),
        _ => return None,
    })
}

// ---------------------------------------------------------------- helpers

fn open_t(dir: f64, i: usize, px: f64, kind: &'static str, sig: usize) -> Trade {
    Trade { dir, legs: vec![Leg { i, px, w: 1. }], exit_i: i, exit_px: px, reason: String::new(), entry_kind: kind, signal_i: sig, open: false, exits: vec![] }
}
fn close_t(cur: &mut Option<Trade>, out: &mut Vec<Trade>, i: usize, px: f64, why: &str) {
    if let Some(mut t) = cur.take() {
        t.exit_i = i;
        t.exit_px = px;
        t.signal_i = i;
        t.reason = why.into();
        out.push(t);
    }
}
fn finish(cur: Option<Trade>, out: &mut Vec<Trade>, x: &Ctx, to: usize) {
    if let Some(mut t) = cur {
        t.exit_i = to;
        t.exit_px = x.c[to];
        t.signal_i = to;
        t.reason = "open".into();
        t.open = true;
        out.push(t);
    }
}

// ---------------------------------------------------------------- Sec III Volatility System

fn vol_system(x: &Ctx, from: usize, to: usize, n: usize, k: f64, init: f64) -> Vec<Trade> {
    let atr = wilder(&true_range(x.r), n);
    let (mut out, mut cur) = (vec![], None);
    if to < n { return out; }
    let start = n - 1;
    let mut pos = init;
    let w = &x.c[..=start];
    let mut sic = if pos < 0. { w.iter().copied().fold(f64::INFINITY, f64::min) } else { w.iter().copied().fold(f64::NEG_INFINITY, f64::max) };
    let mut sar = sic - pos * k * atr[start];
    for i in start + 1..=to {
        let c = x.c[i];
        if (pos < 0. && c > sar) || (pos > 0. && c < sar) {
            close_t(&mut cur, &mut out, i, c, if pos > 0. { "Close below SAR · reverse" } else { "Close above SAR · reverse" });
            pos = -pos;
            sic = c;
            if i >= from { cur = Some(open_t(pos, i, c, "close", i)); }
        } else {
            sic = if pos > 0. { sic.max(c) } else { sic.min(c) };
        }
        sar = sic - pos * k * atr[i];
    }
    finish(cur, &mut out, x, to);
    out
}

// ---------------------------------------------------------------- Sec IV Directional Movement System

fn dmi_system(x: &Ctx, from: usize, to: usize, n: usize, int_di: bool) -> Vec<Trade> {
    let (pdi, mdi, _) = super::lebtc::dmi3(x, n);
    let f = |v: f64| if int_di { v.trunc() } else { v };
    let (mut out, mut cur) = (vec![], None);
    let (mut pos, mut last) = (0f64, 0f64);
    let mut pending: Option<(f64, f64, usize)> = None; // (direction, reverse point, cross bar)
    for i in 1..=to {
        if let Some((d, lvl, ci)) = pending {
            if (d > 0. && x.h[i] > lvl) || (d < 0. && x.l[i] < lvl) {
                let px = if d > 0. { x.o[i].max(lvl) } else { x.o[i].min(lvl) };
                close_t(&mut cur, &mut out, i, px, "DI cross · extreme point penetrated · reverse");
                pos = d;
                pending = None;
                if i >= from { cur = Some(open_t(d, i, px, "stop", ci)); }
            }
        }
        if !(pdi[i].is_finite() && mdi[i].is_finite()) { continue; }
        let s = (f(pdi[i]) - f(mdi[i])).signum() * ((f(pdi[i]) != f(mdi[i])) as i32 as f64);
        if s != 0. {
            if last != 0. && s != last {
                pending = if s != pos { Some((s, if s > 0. { x.h[i] } else { x.l[i] }, i)) } else { None };
            }
            last = s;
        }
    }
    finish(cur, &mut out, x, to);
    out
}

// ---------------------------------------------------------------- Sec V Trend Balance Point System

fn tbp_system(x: &Ctx, from: usize, to: usize, lag: usize) -> Vec<Trade> {
    let tr = true_range(x.r);
    let mf = |i: usize| x.c[i] - x.c[i - lag];
    let (mut out, mut cur) = (vec![], None);
    let mut pos = 0f64;
    let mut entered = 0usize;
    for i in lag + 2..=to {
        let mut exited = 0f64;
        if pos != 0. && i > entered {
            let k = i - 1;
            let xb = (x.h[k] + x.l[k] + x.c[k]) / 3.;
            let hit = if pos > 0. {
                let (stop, tgt) = (xb - tr[k], 2. * xb - x.l[k]);
                if x.l[i] <= stop { Some((x.o[i].min(stop), "Stop X̄ - TR")) } else if x.h[i] >= tgt { Some((x.o[i].max(tgt), "Target 2X̄ - L")) } else { None }
            } else {
                let (stop, tgt) = (xb + tr[k], 2. * xb - x.h[k]);
                if x.h[i] >= stop { Some((x.o[i].max(stop), "Stop X̄ + TR")) } else if x.l[i] <= tgt { Some((x.o[i].min(tgt), "Target 2X̄ - H")) } else { None }
            };
            if let Some((px, why)) = hit {
                close_t(&mut cur, &mut out, i, px, why);
                exited = pos;
                pos = 0.;
            }
        }
        let (m0, m1, m2) = (mf(i), mf(i - 1), mf(i - 2));
        let (up, dn) = (m0 > m1.max(m2), m0 < m1.min(m2));
        let side = if exited != 0. { exited } else { pos };
        let want = if side > 0. { if dn { -1. } else { 1. } } else if side < 0. { if up { 1. } else { -1. } } else if up { 1. } else if dn { -1. } else { 0. };
        if want != 0. && want != pos {
            if pos != 0. { close_t(&mut cur, &mut out, i, x.c[i], "Close through TBP · reverse"); }
            pos = want;
            entered = i;
            if i >= from { cur = Some(open_t(want, i, x.c[i], "close", i)); }
        }
    }
    finish(cur, &mut out, x, to);
    out
}

// ---------------------------------------------------------------- Sec VII Reaction Trend System

#[derive(Clone, Copy, PartialEq)]
enum Mode { Wait, React, Trend }
#[derive(Clone, Copy, PartialEq)]
enum Act { TrendLong, TrendShort, ReactLong, ReactShort, LongTarget, LongRev, ShortRev, Trail }

struct Rt { pos: f64, mode: Mode, entry: usize, anchor: (usize, usize), ext: usize }

/// B=0, O=1, S=2.
fn phase(a: (usize, usize), i: usize) -> usize { (a.1 + i - a.0) % 3 }

fn rt_system(x: &Ctx, from: usize, to: usize, trail_n: usize, margin: f64, anchor: Option<(usize, usize)>) -> Vec<Trade> {
    let (mut out, mut cur): (Vec<Trade>, Option<Trade>) = (vec![], None);
    let mut s = Rt { pos: 0., mode: if anchor.is_some() { Mode::React } else { Mode::Wait }, entry: 0, anchor: anchor.unwrap_or((0, 0)), ext: 0 };
    for i in 1..=to {
        let k = i - 1;
        let xb = (x.h[k] + x.l[k] + x.c[k]) / 3.;
        let (b1, s1) = (2. * xb - x.h[k], 2. * xb - x.l[k]);
        let (hbop, lbop) = (2. * xb - 2. * x.l[k] + x.h[k] + margin, 2. * xb - 2. * x.h[k] + x.l[k] - margin);
        let tn = trail_n.min(i);
        let trail_l = x.l[i - tn..i].iter().copied().fold(f64::INFINITY, f64::min) - margin;
        let trail_s = x.h[i - tn..i].iter().copied().fold(f64::NEG_INFINITY, f64::max) + margin;
        let mut done = false; // a trailing-stop exit ends the day
        // orders: (level, buy?, stop?, action)
        let orders = |s: &Rt, done: bool| -> Vec<(f64, bool, bool, Act)> {
            let mut v = vec![];
            if done { return v; }
            match s.mode {
                Mode::Trend => if i > s.entry { v.push(if s.pos > 0. { (trail_l, false, true, Act::Trail) } else { (trail_s, true, true, Act::Trail) }); },
                Mode::Wait => { v.push((hbop, true, true, Act::TrendLong)); v.push((lbop, false, true, Act::TrendShort)); }
                Mode::React => {
                    v.push((hbop, true, true, Act::TrendLong));
                    v.push((lbop, false, true, Act::TrendShort));
                    let ph = phase(s.anchor, i);
                    if s.pos == 0. {
                        if ph == 0 { v.push((b1, true, false, Act::ReactLong)); }
                        if ph == 2 { v.push((s1, false, false, Act::ReactShort)); }
                    } else if i > s.entry {
                        if s.pos > 0. && ph == 1 { v.push((s1, false, false, Act::LongTarget)); }
                        if s.pos > 0. && ph == 2 { v.push((s1, false, false, Act::LongRev)); }
                        if s.pos < 0. && ph == 0 { v.push((b1, true, false, Act::ShortRev)); }
                    }
                }
            }
            v
        };
        let crossed = |o: &(f64, bool, bool, Act), px: f64| match (o.1, o.2) {
            (true, true) => px >= o.0, (false, true) => px <= o.0, (true, false) => px <= o.0, (false, false) => px >= o.0,
        };
        let fire = |s: &mut Rt, a: Act, px: f64, done: &mut bool, cur: &mut Option<Trade>, out: &mut Vec<Trade>| {
            let open = |s: &mut Rt, dir: f64, mode: Mode, kind: &'static str, cur: &mut Option<Trade>| {
                s.pos = dir; s.mode = mode; s.entry = i; s.ext = i;
                if i >= from { *cur = Some(open_t(dir, i, px, kind, i)); }
            };
            match a {
                Act::TrendLong | Act::TrendShort => {
                    let d = if a == Act::TrendLong { 1. } else { -1. };
                    if s.pos == d { s.mode = Mode::Trend; s.ext = s.entry; if let Some(t) = cur.as_mut() { t.reason = "→ trend mode".into(); } }
                    else {
                        if s.pos != 0. { close_t(cur, out, i, px, if d > 0. { "HBOP · reverse to trend mode" } else { "LBOP · reverse to trend mode" }); }
                        open(s, d, Mode::Trend, "stop", cur);
                    }
                }
                Act::ReactLong => open(s, 1., Mode::React, "limit", cur),
                Act::ReactShort => open(s, -1., Mode::React, "limit", cur),
                Act::LongTarget => { close_t(cur, out, i, px, "S1 on an O day"); s.pos = 0.; }
                Act::LongRev => { close_t(cur, out, i, px, "S1 on an S day · reverse"); open(s, -1., Mode::React, "limit", cur); }
                Act::ShortRev => { close_t(cur, out, i, px, "B1 on a B day · reverse"); open(s, 1., Mode::React, "limit", cur); }
                Act::Trail => {
                    // highest (lowest) day of the trend trade becomes S (B): Regla (3)
                    let (lo, hi) = (s.entry, i);
                    let e = if s.pos > 0 as f64 { (lo..=hi).fold(lo, |m, j| if x.h[j] >= x.h[m] { j } else { m }) } else { (lo..=hi).fold(lo, |m, j| if x.l[j] <= x.l[m] { j } else { m }) };
                    s.anchor = (e, if s.pos > 0. { 2 } else { 0 });
                    close_t(cur, out, i, px, "Trailing stop (two-day extreme)");
                    s.pos = 0.; s.mode = Mode::React; *done = true;
                }
            }
        };
        // TradeStation bar path: open, nearer extreme, farther extreme, close
        let (o, h, l, c) = (x.o[i], x.h[i], x.l[i], x.c[i]);
        let path = if h - o < o - l { [o, h, l, c] } else { [o, l, h, c] };
        let mut a = o;
        for &b in &path {
            for _ in 0..12 {
                let ords = orders(&s, done);
                // already through at the current price (gap / new order): stops first
                let now = ords.iter().filter(|q| crossed(q, a)).min_by_key(|q| !q.2);
                let next = if b < a {
                    ords.iter().filter(|q| ((q.1 && !q.2) || (!q.1 && q.2)) && q.0 < a && q.0 >= b).max_by(|p, q| p.0.total_cmp(&q.0))
                } else {
                    ords.iter().filter(|q| ((!q.1 && !q.2) || (q.1 && q.2)) && q.0 > a && q.0 <= b).min_by(|p, q| p.0.total_cmp(&q.0))
                };
                let (q, px) = match (now, next) { (Some(q), _) => (*q, a), (None, Some(q)) => (*q, q.0), _ => break };
                fire(&mut s, q.3, px, &mut done, &mut cur, &mut out);
                a = px;
            }
            a = b;
        }
        // end of day: reaction-mode exits at the close
        if s.mode == Mode::React && s.pos != 0. && i > s.entry {
            let ph = phase(s.anchor, i);
            if (s.pos > 0. && ph == 2) || (s.pos < 0. && ph == 0) {
                close_t(&mut cur, &mut out, i, c, if s.pos > 0. { "Close on an S day (S1 not reached)" } else { "Close on a B day (B1 not reached)" });
                s.pos = 0.;
            }
        }
        if s.mode == Mode::Trend && s.pos != 0. {
            if (s.pos > 0. && x.h[i] >= x.h[s.ext]) || (s.pos < 0. && x.l[i] <= x.l[s.ext]) { s.ext = i; }
        }
    }
    finish(cur, &mut out, x, to);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn rows(v: &[[f64; 4]]) -> Vec<Row> {
        v.iter().enumerate().map(|(i, b)| Row { ticker: "W".into(), date: format!("2000-01-{:02}", i + 1), open: b[0], high: b[1], low: b[2], close: b[3], volume: None }).collect()
    }
    const VOL: [[f64; 4]; 47] = [
        [52.80, 53.00, 52.50, 52.70], [52.60, 52.75, 52.25, 52.55], [52.00, 52.35, 51.85, 52.30], [52.20, 52.45, 52.15, 52.40],
        [52.10, 52.35, 51.75, 51.90], [51.90, 52.10, 51.50, 51.65], [51.50, 51.80, 51.00, 51.10], [51.15, 51.60, 51.25, 51.55],
        [51.50, 51.70, 51.40, 51.65], [51.60, 51.60, 51.10, 51.15], [51.00, 51.40, 50.75, 50.75], [51.35, 51.75, 51.35, 51.65],
        [51.70, 51.90, 51.40, 51.80], [51.60, 51.70, 51.15, 51.55], [51.55, 51.80, 51.50, 51.80], [51.90, 52.50, 51.80, 52.50],
        [52.40, 52.70, 52.10, 52.70], [52.20, 52.45, 52.00, 52.10], [52.00, 52.65, 51.50, 52.65], [52.50, 52.95, 52.40, 52.90],
        [53.10, 53.60, 53.05, 53.55], [53.95, 54.50, 53.80, 54.50], [55.20, 55.70, 55.00, 55.55], [57.55, 57.55, 57.55, 57.55],
        [57.90, 58.55, 57.90, 58.55], [57.75, 57.75, 57.30, 57.65], [57.50, 58.05, 57.15, 57.95], [57.80, 57.90, 57.45, 57.80],
        [58.00, 58.30, 57.85, 58.20], [58.45, 58.65, 57.75, 58.65], [57.80, 57.80, 57.00, 57.30], [57.00, 57.15, 56.25, 56.60],
        [56.30, 56.35, 55.35, 56.30], [56.20, 56.60, 56.05, 56.60], [56.50, 57.25, 56.40, 57.25], [57.25, 57.50, 57.00, 57.00],
        [57.50, 57.50, 57.25, 57.25], [57.00, 57.15, 56.50, 56.60], [56.50, 56.60, 56.25, 56.45], [56.50, 56.90, 56.45, 56.90],
        [57.00, 57.25, 56.75, 57.10], [57.40, 57.60, 57.00, 57.40], [57.20, 57.25, 56.90, 57.05], [57.25, 58.00, 57.25, 57.90],
        [58.00, 58.50, 57.00, 57.00], [56.50, 56.50, 55.50, 56.05], [56.00, 56.35, 55.75, 55.85],
    ];
    #[test]
    fn volatility_worksheet_p28_reproduces() {
        let r = rows(&VOL);
        let x = Ctx::new(&r, &r, &[]);
        let ts = run("wwnc_vol", &x, 0, 46, &Params::new()).unwrap();
        assert_eq!(ts.len(), 2);
        assert_eq!((ts[0].dir, ts[0].legs[0].i, ts[0].legs[0].px), (1., 15, 52.50)); // Día 16 long at 52.50
        assert_eq!((ts[0].exit_i, ts[0].exit_px), (45, 56.05)); // Día 46 short at 56.05
        assert!(ts[1].dir < 0. && ts[1].open);
    }
    #[test]
    fn tbp_worksheet_p62_reproduces() {
        let r = rows(&[
            [206.50, 207.80, 206.20, 206.80], [206.80, 208.00, 206.30, 206.50], [208.40, 212.50, 208.40, 212.00], [211.00, 212.50, 210.00, 210.50],
            [211.00, 212.00, 208.00, 210.80], [211.00, 213.00, 209.00, 209.50], [208.50, 208.50, 202.50, 202.50], [202.00, 204.00, 195.80, 203.00],
            [203.00, 203.00, 196.00, 196.30], [195.50, 202.00, 195.00, 199.50], [200.00, 206.50, 200.00, 206.00],
        ]);
        let x = Ctx::new(&r, &r, &[]);
        let ts = run("wwnc_tbp", &x, 0, 10, &Params::new()).unwrap();
        let near = |a: f64, b: f64| (a - b).abs() < 0.01;
        assert_eq!((ts[0].dir, ts[0].legs[0].i, ts[0].legs[0].px), (-1., 4, 210.80)); // 19-Aug short 210.80
        assert!(ts[0].exit_i == 6 && near(ts[0].exit_px, 208.00)); // 23-Aug target 208.00
        assert!(ts[1].legs[0].px == 202.50 && ts[1].exit_i == 7 && near(ts[1].exit_px, 200.50)); // 24-Aug target 200.50
        assert!(ts[2].legs[0].px == 203.00 && ts[2].exit_i == 8 && near(ts[2].exit_px, 197.867)); // 25-Aug target (book 197.80)
        assert!(ts[3].dir > 0. && ts[3].legs[0].px == 196.30 && ts[3].exit_i == 9 && near(ts[3].exit_px, 200.867)); // long 196.30, target 200.86
        assert!(ts[4].dir > 0. && ts[4].legs[0].i == 9 && ts[4].legs[0].px == 199.50);
    }
    #[test]
    fn reaction_trend_worksheet_p80_reproduces() {
        let r = rows(&[
            [51.00, 51.50, 50.50, 50.50], [50.50, 51.00, 50.00, 51.00], [51.00, 51.20, 50.50, 51.00], [51.10, 51.50, 50.50, 50.50],
            [51.00, 51.00, 50.10, 51.00], [50.50, 50.50, 49.00, 49.50], [49.50, 49.50, 48.00, 48.00], [48.00, 48.50, 47.50, 47.80],
            [47.20, 48.20, 47.00, 48.20], [48.50, 49.50, 47.70, 49.50], [49.80, 50.60, 49.00, 49.20],
        ]);
        let x = Ctx::new(&r, &r, &[]);
        let ts = rt_system(&x, 0, 10, 2, 0., Some((0, 2))); // Día 1 = "S" as on the worksheet
        let near = |a: f64, b: f64| (a - b).abs() < 0.011;
        assert!(ts[0].dir > 0. && ts[0].legs[0].i == 1 && near(ts[0].legs[0].px, 50.16)); // L 50.16
        assert!(ts[0].exit_i == 3 && near(ts[0].exit_px, 51.30)); // S 51.30 (reverse)
        assert!(ts[1].dir < 0. && ts[1].exit_i == 4 && near(ts[1].exit_px, 50.17)); // B1 day 5 (book 50.16)
        assert!(ts[2].dir > 0. && ts[2].exit_i == 5 && near(ts[2].exit_px, 49.50)); // LBOP 49.50 (S 49.50)
        assert!(ts[3].dir < 0. && ts[3].legs[0].i == 5 && ts[3].exit_i == 9 && near(ts[3].exit_px, 48.50)); // trailing stop (book 'Salida 48.55')
    }
    #[test]
    fn dmi_stop_waits_for_the_cross_day_extreme() {
        // flat start, +DI crosses above -DI on some bar; entry only once the cross-day high is exceeded
        let mut v: Vec<[f64; 4]> = (0..40).map(|i| { let c = 100. - i as f64 * 0.5; [c, c + 1., c - 1., c] }).collect();
        for i in 40..60 { let c = 80. + (i - 40) as f64 * 1.0; v.push([c, c + 1., c - 1., c]); }
        let r = rows(&v);
        let x = Ctx::new(&r, &r, &[]);
        let ts = run("wwnc_dmi", &x, 0, 59, &Params::new()).unwrap();
        assert_eq!(ts.len(), 1);
        let (pd, md, _) = super::super::lebtc::dmi3(&x, 14);
        let ci = (15..60).find(|&i| pd[i].trunc() > md[i].trunc() && pd[i - 1].trunc() < md[i - 1].trunc()).unwrap();
        let t = &ts[0];
        assert!(t.dir > 0. && t.entry_kind == "stop" && t.legs[0].i > ci);
        assert_eq!(t.legs[0].px, x.o[t.legs[0].i].max(x.h[ci]));
    }
}

// Instrument admission: J. Welles Wilder 2026-10-25 block, moved from instrument_policy.rs on 31 Oct 2026 (lane ST).
// Admission only for the module REVIEWED lists (authors/wilder/admission/SOURCE-CONTRACT.md); kept in one block to merge.
// Runs on actual indices, ETFs and stocks, US and India (index_etf_or_stock, nifty50 basket, declared application).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book=="wwnc" && crate::books::wwnc::reviewed(def.id),
    label: "Wilder worked these systems on 1977-78 commodity futures; runs on indices, ETFs and single stocks (US and India) are a declared application, one unit per trade, always-in systems reverse as printed.",
    basis: "J. Welles Wilder Jr., New Concepts in Technical Trading Systems (1978), Spanish edition",
};
