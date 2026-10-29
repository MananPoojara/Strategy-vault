//! Edward (Ed) Burke, "How I Won: Making Money in Stock Trends" (Big Wednesday Capital Management, 2009, CNBC Million
//! Dollar Challenge e-book; book id `ebmst`; printed page = PDF page - 3). Reading: claude-continuation-20261020/authors/
//! burke (admission/hiw_ref.py). Live: the 3-6-18 exponential average continuation system of Ch.5 "Basic System Rules"
//! (p.23; logic Ch.3-4 p.10-22): buy when the 3-day EMA crosses above the 6-day EMA while the 6-day is above the 18-day,
//! sell when the 3 crosses back below the 6; the short side is the mirror. Not coded: the tutorials' trade management,
//! IBD RS 99 / float screens (ranking data not held), the other EMA suites (5-10-50, 3-13-39, 5-20-100) are options only
//! by length. Book: US stocks on daily charts (CAT, IBM); indices, ETFs and US/India stocks here are a declared application.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};
use super::tilma::ema;
/// Admitted ids (authors/burke/admission/hiw_ref.py (header)).
pub const REVIEWED: &[&str] = &["ebmst_ema_3_6_18"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] EMA alpha = 2/(n+1), started at the first close the vault holds (full history warms it; the book does not give a start value). Cross above = today's 3-EMA above the 6-EMA after yesterday's at or below it (below: the mirror). The 6-versus-18 test is read on the cross day. The book gives no order type: signals are taken at the close and filled at the next day's open (fill 1 = the signal close). A cross down exits a long and, when the 6-EMA is below the 18-EMA, opens the short at the same price (and the mirror). A signal on the last bar is not filled; a position still open on the last bar is marked at its close. No stop or target: the tutorials' breakeven management is not in this e-book.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "ebmst_ema_3_6_18", name: "3-6-18 EMA continuation: 3 crosses the 6 on the side of the 18, exit on the cross back", group: "Burke stock trends",
        session: 0, dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "ebmst",
        rules: &["F|\"For the exact system supplied by this eBook, we will use the 3 EMA. The SHORT TERM TREND will use the 6 EMA ... the LONG TERM TREND EMA is 18 (3 x 6 = 18)\" (p.10)",
            "E|\"Enter LONG (Buy) when the 3 EMA crosses above the 6 EMA in an established UP TREND (the 6 EMA is trading above the 18 EMA).\" (p.23)",
            "X|\"CLOSE (exit) the LONG position when the PRICE EMA crosses BELOW the SHORT TERM EMA ... Exit when the 3 EMA crosses BELOW the 6 EMA.\" (p.23)",
            "E|\"Enter SHORT (Sell) when the 3 EMA crosses below the 6 EMA in an established DOWN TREND (the 6 EMA is trading below the 18 EMA).\" (p.23)",
            "X|\"Exit when the 3 EMA crosses ABOVE the 6 EMA.\" (short exit, p.23)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id { "ebmst_ema_3_6_18" => Some(("ebmst_ema_3_6_18", "Burke 3-6-18 EMA continuation", "3 / 6 / 18")), _ => None }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    match id {
        "ebmst_ema_3_6_18" => Some(vec![p("side", "Sides: 0 = long and short (book), 1 = long only, 2 = short only", 0., 0., 2., 1.),
            p("fast", "Price EMA (book 3)", 3., 1., 20., 1.), p("mid", "Short-term trend EMA (book 6)", 6., 2., 50., 1.),
            p("slow", "Long-term trend EMA (book 18 = 3 x 6)", 18., 3., 200., 1.),
            p("fill", "Fill: 0 = next day's open, 1 = signal close [book silent]", 0., 0., 1., 1.)]),
        _ => None,
    }
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let u = |k: &str| g(k).round().max(1.) as usize;
    let side = g("side").round() as i64;
    let at_close = g("fill").round() >= 1.;
    let (f, m, s) = (ema(&x.c, u("fast")), ema(&x.c, u("mid")), ema(&x.c, u("slow")));
    let kind = if at_close { "close" } else { "open" };
    let fill = |t: usize| if at_close { (t, x.c[t]) } else { (t + 1, x.o[t + 1]) };
    let (mut out, mut cur) = (vec![], None::<(f64, usize, f64)>);
    for t in from.max(1)..to {
        let up = f[t] > m[t] && f[t - 1] <= m[t - 1];
        let dn = f[t] < m[t] && f[t - 1] >= m[t - 1];
        if let Some((d, ei, ep)) = cur {
            if (d > 0. && dn) || (d < 0. && up) {
                let (xi, xp) = fill(t);
                out.push(Trade { dir: d, legs: vec![Leg { i: ei, px: ep, w: 1. }], exit_i: xi, exit_px: xp, reason: "3 EMA crossed back over the 6 EMA".into(),
                    entry_kind: kind, signal_i: t, open: false, exits: vec![] });
                cur = None;
            }
        }
        if cur.is_none() {
            let (xi, xp) = fill(t);
            if up && m[t] > s[t] && side != 2 { cur = Some((1., xi, xp)); }
            else if dn && m[t] < s[t] && side != 1 { cur = Some((-1., xi, xp)); }
        }
    }
    if let Some((d, ei, ep)) = cur {
        out.push(Trade { dir: d, legs: vec![Leg { i: ei, px: ep, w: 1. }], exit_i: to, exit_px: x.c[to], reason: "open".into(), entry_kind: kind, signal_i: to, open: true, exits: vec![] });
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn hand_checked_continuation_trades() {
        // EMAs start at the first close. Bar 1: 3 crosses above 6 with 6 above 18 -> buy bar 2's open (11.75); bar 18:
        // 3 (19.27) under 6 (19.51) -> sell bar 19's open; bar 26: 3 back above 6 but 6 (13.47) under 18 (14.55) -> no
        // long; bar 27: 3 crosses under 6 with 6 under 18 -> short bar 28's open (4.75), still open on the last bar.
        let cl = [10., 11., 12., 13., 14., 15., 16., 17., 18., 19., 20., 19., 18., 19., 20., 21., 22., 20., 18., 16., 14., 12., 10., 11., 9., 8., 20., 6., 5., 5.];
        let r: Vec<Row> = cl.iter().enumerate().map(|(i, &c)| Row { ticker: "T".into(), date: format!("2020-02-{:02}", i + 1), open: c - 0.25, high: c + 1., low: c - 1., close: c, volume: None }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let t = run("ebmst_ema_3_6_18", &x, 0, r.len() - 1, &Params::new()).unwrap();
        let got: Vec<_> = t.iter().map(|z| (z.dir, z.legs[0].i, z.legs[0].px, z.signal_i, z.exit_i, z.exit_px, z.open)).collect();
        assert_eq!(got, vec![(1., 2, 11.75, 18, 19, 15.75, false), (-1., 28, 4.75, 29, 29, 5., true)]);
        assert!(run("ebmst_x", &x, 0, r.len() - 1, &Params::new()).is_none());
    }
}
