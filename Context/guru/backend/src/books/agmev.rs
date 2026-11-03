//! Francisco Fernandez Martinez (AGMercados), "Estrategias de Inversion con Visual Chart" (2001, Spanish; research share
//! USB HDD 1/TRADING KNOWLEDGE-2/_Libros y Cursos en Español/Agmercados - Estrategias de Inversion con Visual Chart/). Book id
//! `agmev`. Eleven of the book's 25 teaching systems, each with its printed default values (summary cards, the VBA parameter
//! comments and the worked examples); the other 14 are held or duplicates (see the K8 TSV). Lane K8 (titles loop, 1 Nov 2026);
//! independent check authors/titles-1030/K8w/k8_ref.py (header).
use crate::connors::{rsi, sma, Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::mkits::{ema, hhv, llv};
use super::rbtie::{mk, target_trades};

/// Admitted ids (authors/titles-1030/K8w/k8_ref.py (header)).
pub const REVIEWED: &[&str] = &["agmev_media_movil", "agmev_comparacion", "agmev_cruce", "agmev_bandas", "agmev_consecutivos",
    "agmev_permanencia", "agmev_fornext", "agmev_alcista", "agmev_medias_rsi", "agmev_sksd", "agmev_pivots"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

macro_rules! common { () => { " [Teaching model] The author: 'No se trata de exponer sistemas brillantes' (the aim is not to show brilliant systems): each one teaches a programming element of Visual Chart, tested on the Ibex 35 and Spanish stocks, with no results worth a claim. Orders 'al cierre' fill at that day's close; stop orders are set at a close for the next day (a gap through the level fills at the open); a buy while short (or a sell while long) reverses; one unit, no accumulation. Daily bars on ETFs, US/India indices and stocks are a declared application." } }

fn notes(id: &str) -> &'static str {
    match id {
        "agmev_media_movil" => concat!("[Declared readings] 30-day simple average (the worked example; the book then optimises it). The flow diagram's condition 'Cierre >= Indicador(AvSimpleData)': long while the close is at or above the average, 'Cerrar Largo' at the close when it is below. Long only.", common!()),
        "agmev_comparacion" => concat!("[Declared readings] n = 6 (the worked example). Diagram 'Cierre >= Cierre(n)': long while today's close is at or above the close 6 bars back, out at the close otherwise. Long only.", common!()),
        "agmev_cruce" => concat!("[Declared readings] Exponential averages of 10 and 30 days (the worked example), seeded with the simple average of the first n closes. On the crossing bar (fast above slow today, below it the bar before) 'Techo' = that bar's high; while the fast average stays above the slow one, a buy stop at Techo + 1% waits for the next bars (the author anchors it so that it does not move). Out at the close on the first day the fast average is not above the slow one. No order before the first crossing (the book's Techo starts at 0). Long only.", common!()),
        "agmev_bandas" => concat!("[Declared readings] 10 days (GetHighest/GetLowest Length 10). BandaS/BandaI are assigned after the conditions, so today's close is compared with the highest high / lowest low of the 10 bars before today: a close at or above it buys at the close, at or below the lower band sells (short) at the close; always in the market after the first signal.", common!()),
        "agmev_consecutivos" => concat!("[Declared readings] N = NB = 3 (the chart's values, with the author's 'Desde i=0 Hasta N-1' fix): buy at the close after 3 highs in a row each at or above the one before (the loop rejects only a lower high), close the long at the close after 3 lows in a row each at or below the one before. A day that has both closes the long. Long only.", common!()),
        "agmev_permanencia" => concat!("[Declared readings] The VBA defaults n = 5 bars, X = 1, GetLowest 10 bars of lows (the chart's n = 9, X = 6, 12 are an option). Buy at the close when the close is above the high X bars back; close the long at the close when 5 bars have passed since entry; until then a stop at the lowest low of the last 10 bars, set at each close for the next day. A day stopped out may buy again at its close.", common!()),
        "agmev_fornext" => concat!("[Declared readings] The pattern the author wants (3 closes in a row each above the previous bar's high to buy, 2 closes in a row each below the previous bar's low to sell); the VBA defaults NAlza = 3, NBaja = 2 count one more bar ('los valores de NAlza y NBaja deberian ser respectivamente 2 y 1'), so 3 and 2 repetitions are the default and 4/3 an option. Buy and exit at the close; while long and no exit pattern, a stop at the lowest low of the last 10 bars for the next day. Long only.", common!()),
        "agmev_alcista" => concat!("[Declared readings] The VBA defaults N = NB = 3; the loop runs from 0 to N, so 4 bars in a row closing at or above their open buy at the close ('si se repite cuatro veces seguidas'), and 4 bars closing at or below their open close the long at the close (a day with both closes it). The chart's N = NB = 2 is an option. Long only.", common!()),
        "agmev_medias_rsi" => concat!("[Declared readings] RSI 14 (Wilder), its 10-bar simple average and 50-bar weighted average (the VBA defaults). When the simple average crosses above the weighted one, Up = the highest high of the last 10 bars and a buy stop at Up is placed each day while the simple average stays above; when it crosses below, Down = the lowest low of the last 10 bars and a sell stop at Down (stop-and-reverse). No order before the first crossing (Up starts at 0 in the code).", common!()),
        "agmev_sksd" => concat!("[Declared readings] Stochastic 14 with exponential SK 3 and SD 3 (the VBA defaults; the raw %K = 100 (C - 14-bar lowest low) / (14-bar highest high - lowest low), SK = its 3-bar exponential average, SD = the 3-bar exponential average of SK). Buy at the close when SK crosses above SD below 20; sell short at the close when SK crosses below SD above 80 (the text's 'en principio 20 y 80'; the code comment shows 60 for NB, an option). Exits: a long's stop at the 10-bar lowest low minus 1%, a short's at the 10-bar highest high plus 1%, set at each close for the next day.", common!()),
        "agmev_pivots" => concat!("[Declared readings] Pivots with 5 bars on each side, searched in the last 25 bars, the first one back (the worked example). A pivot high = a high above each of the 5 highs before and after it (strictly; a pivot low mirrored), known once its 5 right-hand bars have closed. Each day a buy stop at the latest pivot high and a sell stop at the latest pivot low; stop-and-reverse; a flat day that trades both levels takes neither.", common!()),
        _ => "",
    }
}

const RULES: &[(&str, &str, &str, &[&str])] = &[
    ("agmev_media_movil", "Media Movil: long while the close is at or above the 30-day average, at the close", "Media Movil (p.47-58)",
        &["E|\"Puntos Entrada. Al cierre cuando la barra en su campo de cierre sea superior al valor de la media movil\"", "X|\"Puntos Salida. Al cierre cuando la barra en su campo de cierre sea inferior al valor de la media movil\""]),
    ("agmev_comparacion", "Comparacion: long while the close is at or above the close 6 bars back", "Comparacion (p.59-63)",
        &["E|\"Al cierre de la barra actual si el cierre es superior al cierre de la barra n\"", "X|\"Al cierre de la barra actual si el cierre es inferior al cierre de la barra n\""]),
    ("agmev_cruce", "Cruce: EMA 10/30 crossing, buy stop 1% above the crossing day's high, out at the close when it reverses", "Cruce (p.64-70)",
        &["E|\"Puntos Entrada. En stop: Techo+Techo*1/100\" (Techo = the high of the crossing bar)", "X|\"Puntos Salida. Al cierre cuando la media rapida cruce a la baja a la media lenta\""]),
    ("agmev_bandas", "Bandas: close at or beyond the 10-day high or low band, stop-and-reverse at the close", "Bandas (p.77-80)",
        &["E|\"Al cierre cuando el campo de la barra precio de cierre iguale o supere a la Banda Superior\"", "X|\"Al cierre cuando el campo de la barra precio de cierre iguale o se ponga por debajo del valor de Banda Inferior\" (Venta)"]),
    ("agmev_consecutivos", "Consecutivos: buy after 3 rising highs, sell after 3 falling lows, at the close", "Consecutivos (p.80-83)",
        &["E|Maximas consecutivas: \"Compra 1 Cierre\"", "X|Minimas consecutivas: \"Cerrar Largo 1 Cierre\""]),
    ("agmev_permanencia", "Permanencia: buy a close above yesterday's high, out after 5 bars or on the 10-day low stop", "Permanencia (p.163-169)",
        &["E|\"If .Close > .High(1) Then .Buy AtClose, 1\"", "X|\"If .GetBarsSinceEntry(0) = n Then .ExitLong AtClose, 1 Else .ExitLong AtStop, 1, .GetLowest(...)\""]),
    ("agmev_fornext", "For Next: buy after 3 closes above the previous highs, sell after 2 closes below the previous lows, 10-day low stop", "For Next (p.170-173)",
        &["E|\"For X = 0 To NAlza: If .Close(X) < .High(X + 1) Then Control = False ... If Control = True Then .Buy AtClose, 1\"", "X|\"If .Close(X) > .Low(X + 1) Then ControlB = False ... .ExitLong AtClose, 1 Else .ExitLong AtStop, 1, .GetLowest(...)\""]),
    ("agmev_alcista", "Alcista/Bajista: buy after 4 white bars, sell after 4 black bars, at the close", "Alcista/Bajista (p.174-176)",
        &["E|\"For i = 0 To N: If .Close(i) < .Open(i) Then control = False ... .Buy AtClose, 1\"", "X|\"For i = 0 To NB: If .Close(i) > .Open(i) Then controlB = False ... .ExitLong AtClose, 1\""]),
    ("agmev_medias_rsi", "Medias RSI: 10-bar simple vs 50-bar weighted average of RSI(14), stop entries at the 10-day high or low", "Medias RSI (p.186-190)",
        &["E|\"If .GetIndicatorValue(AvSimpleRSIData) > .GetIndicatorValue(AvWeightedRSIData) Then ... Up = .GetHighest(...) ... .Buy AtStop, 1, Up\"", "X|\"Else ... Down = .GetLowest(...) ... .Sell AtStop, 1, Down\""]),
    ("agmev_sksd", "SKSD Estocastico: SK/SD crossing below 20 buys, above 80 sells, 10-day low/high stops 1% beyond", "SKSD Estocastico (p.181-186)",
        &["E|SK crosses above SD \"And .GetIndicatorValue(StochasticData, 0, 1) < NA Then .Buy AtClose, 1\"; mirror \"> NB Then .Sell AtClose, 1\"", "X|\".ExitLong AtStop, 1, .GetLowest(...) - .GetLowest(...) * PorcentajeLargo / 100\"; short mirror with GetHighest"]),
    ("agmev_pivots", "Pivots: buy stop at the latest 5/5 pivot high, sell stop at the latest pivot low, stop-and-reverse", "Pivots (p.71-77)",
        &["E|\"Puntos entrada. En Stop en el valor de los pivots\"", "X|\"Puntos salida. En Stop en el valor de los pivots\""]),
];

pub fn catalogue() -> Vec<StrategyDef> {
    RULES.iter().map(|&(id, name, _, rules)| StrategyDef { id, name, group: "Agmercados Visual Chart teaching systems", session: 0,
        dir: if matches!(id, "agmev_bandas" | "agmev_medias_rsi" | "agmev_sksd" | "agmev_pivots") { "Long/Short" } else { "Long" },
        status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "agmev", rules, notes: notes(id), claim: None })
        .collect()
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    RULES.iter().find(|r| r.0 == id).map(|r| (r.0, r.2, "book defaults"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(match id {
        "agmev_media_movil" => vec![p("n", "Average days (30)", 30., 2., 200., 1.)],
        "agmev_comparacion" => vec![p("n", "Bars back (6)", 6., 1., 60., 1.)],
        "agmev_bandas" => vec![p("n", "Band days (10)", 10., 2., 100., 1.)],
        "agmev_consecutivos" => vec![p("n", "Rising highs in a row to buy (3)", 3., 1., 10., 1.), p("nb", "Falling lows in a row to sell (3)", 3., 1., 10., 1.)],
        "agmev_alcista" => vec![p("n", "N: buy after N+1 bars closing at or above the open (3)", 3., 0., 10., 1.), p("nb", "NB: sell after NB+1 bars closing at or below the open (3)", 3., 0., 10., 1.)],
        "agmev_cruce" => vec![p("fast", "Fast EMA (10)", 10., 2., 100., 1.), p("slow", "Slow EMA (30)", 30., 3., 300., 1.), p("pct", "Stop above the crossing high, % (1)", 1., 0., 5., 0.1)],
        "agmev_permanencia" => vec![p("n", "Bars in the trade (5)", 5., 1., 50., 1.), p("x", "Close above the high of X bars back (1)", 1., 1., 20., 1.), p("len", "Stop: lowest low of this many bars (10)", 10., 2., 50., 1.)],
        "agmev_fornext" => vec![p("up", "Closes above the previous high in a row to buy (3)", 3., 1., 10., 1.), p("dn", "Closes below the previous low in a row to sell (2)", 2., 1., 10., 1.)],
        "agmev_medias_rsi" => vec![p("len", "Simple average of RSI (10)", 10., 2., 100., 1.), p("wlen", "Weighted average of RSI (50)", 50., 2., 200., 1.)],
        "agmev_sksd" => vec![p("na", "Buy crossing below (20)", 20., 0., 50., 1.), p("nb", "Sell crossing above (80; the code comment: 60)", 80., 50., 100., 1.), p("pct", "Stop beyond the 10-day low/high, % (1)", 1., 0., 5., 0.1)],
        "agmev_pivots" => vec![p("bars", "Bars each side of a pivot (5)", 5., 1., 20., 1.), p("period", "Search the last bars (25)", 25., 5., 200., 1.)],
        _ => return None,
    })
}

fn wma(v: &[f64], n: usize) -> Vec<f64> {
    let den = (n * (n + 1) / 2) as f64;
    (0..v.len()).map(|i| if i + 1 >= n && v[i + 1 - n..=i].iter().all(|x| x.is_finite()) {
        v[i + 1 - n..=i].iter().enumerate().map(|(k, x)| x * (k + 1) as f64).sum::<f64>() / den } else { f64::NAN }).collect()
}

fn states(id: &str, x: &Ctx, g: &dyn Fn(&str) -> usize) -> Vec<i8> {
    let (o, h, l, c, n_) = (&x.o, &x.h, &x.l, &x.c, x.c.len());
    let mut w = vec![0i8; n_];
    match id {
        "agmev_media_movil" => { let m = sma(c, g("n")); for i in 0..n_ { w[i] = (c[i] >= m[i]) as i8; } }
        "agmev_comparacion" => { let n = g("n"); for i in n..n_ { w[i] = (c[i] >= c[i - n]) as i8; } }
        "agmev_bandas" => {
            let n = g("n");
            for i in 1..n_ {
                w[i] = w[i - 1];
                if i >= n && c[i] >= h[i - n..i].iter().copied().fold(f64::NEG_INFINITY, f64::max) { w[i] = 1; }
                else if i >= n && c[i] <= l[i - n..i].iter().copied().fold(f64::INFINITY, f64::min) { w[i] = -1; }
            }
        }
        _ => {
            let (n, nb) = (g("n"), g("nb"));
            for i in 1..n_ {
                let (up, dn) = if id == "agmev_consecutivos" {
                    (i >= n && (0..n).all(|k| h[i - k] >= h[i - k - 1]), i >= nb && (0..nb).all(|k| l[i - k] <= l[i - k - 1]))
                } else {
                    (i >= n && (0..=n).all(|k| c[i - k] >= o[i - k]), i >= nb && (0..=nb).all(|k| c[i - k] <= o[i - k]))
                };
                w[i] = if dn { 0 } else if up { 1 } else { w[i - 1] };
            }
        }
    }
    w
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let schema = params(id)?;
    let gf = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let g = |k: &str| gf(k).round().max(0.) as usize;
    if matches!(id, "agmev_media_movil" | "agmev_comparacion" | "agmev_bandas" | "agmev_consecutivos" | "agmev_alcista") {
        return Some(target_trades(x, &states(id, x, &g), from, to, false, "rule changed"));
    }
    let (o, h, l, c, n_) = (&x.o, &x.h, &x.l, &x.c, x.c.len());
    let (ll10, hh10) = (llv(l, 10), hhv(h, 10));
    let (mut ef, mut es, mut s_, mut w_, mut k_, mut d_) = (vec![], vec![], vec![], vec![], vec![], vec![]);
    let (mut piv_h, mut piv_l) = (vec![], vec![]);
    let (nbar, per) = (g("bars"), g("period"));
    match id {
        "agmev_cruce" => { ef = ema(c, g("fast").max(1)); es = ema(c, g("slow").max(1)); }
        "agmev_medias_rsi" => { let r = rsi(c, 14); s_ = sma(&r, g("len").max(1)); w_ = wma(&r, g("wlen").max(1)); }
        "agmev_sksd" => {
            let (h14, l14) = (hhv(h, 14), llv(l, 14));
            let raw: Vec<f64> = (0..n_).map(|i| if h14[i].is_finite() && h14[i] != l14[i] { 100. * (c[i] - l14[i]) / (h14[i] - l14[i]) } else { f64::NAN }).collect();
            k_ = ema(&raw, 3); d_ = ema(&k_, 3);
        }
        "agmev_pivots" => {
            let piv = |v: &[f64], k: usize, hi: bool| k >= nbar && k + nbar < n_ && (k - nbar..k).chain(k + 1..=k + nbar).all(|j| if hi { v[k] > v[j] } else { v[k] < v[j] });
            piv_h = (0..n_).map(|k| piv(h, k, true)).collect(); piv_l = (0..n_).map(|k| piv(l, k, false)).collect();
        }
        _ => {}
    }
    let (mut out, mut pos) = (vec![], None::<(f64, usize, f64)>);
    let (mut bs, mut ss, mut ls) = (None::<f64>, None::<f64>, None::<f64>);
    let (mut techo, mut up, mut down) = (None::<f64>, None::<f64>, None::<f64>);
    let close = |out: &mut Vec<Trade>, q: (f64, usize, f64), i: usize, px: f64| out.push(mk(q.0, q.1, q.2, "stop", i, px, "exit", false, i));
    for i in from..=to {
        if let (Some(st), Some(q)) = (ls, pos) {
            if q.0 > 0. && l[i] <= st { close(&mut out, q, i, o[i].min(st)); pos = None; }
            else if q.0 < 0. && h[i] >= st { close(&mut out, q, i, o[i].max(st)); pos = None; }
        }
        let mut hb = bs.map_or(false, |b| pos.map_or(true, |q| q.0 < 0.) && h[i] >= b);
        let mut hs = ss.map_or(false, |s| pos.map_or(true, |q| q.0 > 0.) && l[i] <= s);
        if pos.is_none() && hb && hs { hb = false; hs = false; }
        if hb { let px = o[i].max(bs.unwrap()); if let Some(q) = pos { close(&mut out, q, i, px); } pos = Some((1., i, px)); }
        else if hs { let px = o[i].min(ss.unwrap()); if let Some(q) = pos { close(&mut out, q, i, px); } pos = Some((-1., i, px)); }
        (bs, ss, ls) = (None, None, None);
        let last = i == to;
        match id {
            "agmev_cruce" => {
                if i > 0 && ef[i] > es[i] && ef[i - 1] < es[i - 1] { techo = Some(h[i]); }
                if let Some(q) = pos { if !(ef[i] > es[i]) { close(&mut out, q, i, c[i]); pos = None; } }
                if pos.is_none() && ef[i] > es[i] { if let Some(t) = techo { bs = Some(t + t * gf("pct") / 100.); } }
            }
            "agmev_permanencia" => {
                let (n, xb, ln) = (g("n"), g("x"), g("len").max(1));
                let mut fresh = false;
                if pos.is_none() && !last && i >= xb && c[i] > h[i - xb] { pos = Some((1., i, c[i])); fresh = true; }
                if let Some(q) = pos {
                    if !fresh && i - q.1 == n { close(&mut out, q, i, c[i]); pos = None; }
                    else { ls = Some(if i + 1 >= ln { l[i + 1 - ln..=i].iter().copied().fold(f64::INFINITY, f64::min) } else { f64::NAN }); }
                }
            }
            "agmev_fornext" => {
                let (nu, nd) = (g("up"), g("dn"));
                let upc = i >= nu && (0..nu).all(|k| c[i - k] >= h[i - k - 1]);
                let dnc = i >= nd && (0..nd).all(|k| c[i - k] <= l[i - k - 1]);
                if pos.is_none() && !last && upc { pos = Some((1., i, c[i])); }
                if let Some(q) = pos { if dnc { close(&mut out, q, i, c[i]); pos = None; } else { ls = Some(ll10[i]); } }
            }
            "agmev_medias_rsi" => {
                if s_[i] > w_[i] { if i > 0 && s_[i - 1] < w_[i - 1] { up = Some(hh10[i]); } bs = up; }
                else { if i > 0 && s_[i - 1] > w_[i - 1] { down = Some(ll10[i]); } ss = down; }
            }
            "agmev_sksd" => {
                let pc = gf("pct") / 100.;
                let cu = i > 0 && k_[i - 1] < d_[i - 1] && k_[i] > d_[i];
                let cd = i > 0 && k_[i - 1] > d_[i - 1] && k_[i] < d_[i];
                if !last && cu && k_[i] < gf("na") && pos.map_or(true, |q| q.0 < 0.) {
                    if let Some(q) = pos { close(&mut out, q, i, c[i]); } pos = Some((1., i, c[i]));
                } else if !last && cd && k_[i] > gf("nb") && pos.map_or(true, |q| q.0 > 0.) {
                    if let Some(q) = pos { close(&mut out, q, i, c[i]); } pos = Some((-1., i, c[i]));
                }
                if let Some(q) = pos { ls = Some(if q.0 > 0. { ll10[i] * (1. - pc) } else { hh10[i] * (1. + pc) }); }
            }
            _ => { // agmev_pivots
                let find = |v: &Vec<bool>| (i.saturating_sub(per - 1)..=i - nbar).rev().find(|&k| k + nbar <= i && v[k]);
                if i >= nbar { bs = find(&piv_h).map(|k| h[k]); ss = find(&piv_l).map(|k| l[k]); }
            }
        }
        if last { (bs, ss, ls) = (None, None, None); }
    }
    if let Some(q) = pos { out.push(mk(q.0, q.1, q.2, "stop", to, c[to], "open", true, to)); }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn rows(c: &[f64]) -> Vec<Row> {
        c.iter().enumerate().map(|(i, &v)| Row { ticker: "T".into(), date: format!("2001-{:02}-{:02}", 1 + i / 28, 1 + i % 28), open: v - 0.2, high: v + 0.5, low: v - 0.5, close: v, volume: None }).collect()
    }
    #[test]
    fn alcista_needs_four_white_bars() {
        let r = rows(&[10., 10., 10., 10., 10., 10.]); // every bar closes above its open (open = close - 0.2)
        let x = Ctx::new(&r, &r, &[]);
        let t = run("agmev_alcista", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert_eq!((t[0].legs[0].i, t[0].open), (3, true));
    }
    #[test]
    fn wma_weights_recent_bars() {
        assert_eq!(wma(&[1., 2., 3.], 3)[2], (1. + 4. + 9.) / 6.);
    }
    #[test]
    fn pivot_stop_and_reverse() {
        let mut c: Vec<f64> = (0..12).map(|i| 100. + i as f64).collect();
        c.extend((0..12).map(|i| 110. - i as f64)); // pivot high at bar 11 (111.5)
        c.extend([97., 115.]);
        let r = rows(&c);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("agmev_pivots", &x, 0, r.len() - 1, &Params::new()).unwrap();
        // the pivot high 111.5 (bar 11) is known from bar 16; bar 25 opens at 114.8, above the stop: long at the open
        assert!(t.iter().any(|z| z.dir == 1. && z.legs[0].i == 25 && (z.legs[0].px - 114.8).abs() < 1e-9));
    }
}

// Instrument admission: Francisco Fernandez Martinez / AGMercados (Visual Chart teaching systems) 2026-11-01 (titles lane K8).
// Admission only for the module REVIEWED lists (authors/titles-1030/K8w/k8_ref.py (header)).
// Runs on actual indices, ETFs and stocks, US and India (index_etf_or_stock, nifty50 basket, declared application).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "agmev" && crate::books::agmev::reviewed(def.id),
    label: "Teaching systems shown on the Ibex 35 and Spanish stocks with no claim; ETFs, US/India indices and stocks are a declared application.",
    basis: "Fernandez Martinez (AGMercados), Estrategias de Inversion con Visual Chart (2001) printed p.47-83 (PDV systems) and 163-190 (VBA systems), Anexo II p.303-310; independent check authors/titles-1030/K8w/k8_ref.py",
};
