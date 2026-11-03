//! Scot Lowry, "The Magic of Moving Averages" (self-published, c.1998; book id `slmma`; scanned, no text layer, PDF page
//! numbers). Reading: claude-continuation-20261020/authors/lowry (admission/mma_ref.py; OCR of PDF 3-31, 45-46, 58-63).
//! Live: the basic "Delphic Phenomenon" system of Ch.3 (PDF 14-17, chart key PDF 25 upside down): after the 18-day average
//! crosses the 40-day, wait for the first dip of price through the 18-day, then a stop order just beyond the 18-day;
//! protective stop just beyond the 40-day, after about a week trailed halfway between the two averages.
//! Not coded: the "quick dip" time judgement (PDF 19-21, "not etched in stone"), the 4-day average convergence of Ch.4 and
//! the fast-market stop between the 4- and 18-day averages (PDF 24, 61: judgement on "breakneck speed"), report days and
//! options (futures only). Book: US commodity futures (soybeans, cattle, corn, cocoa); indices, ETFs and US/India stocks
//! here are a declared application.
use crate::connors::{sma, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};
/// Admitted ids (authors/lowry/admission/mma_ref.py (header)).
pub const REVIEWED: &[&str] = &["slmma_delphic"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] Simple averages of the close. A cross = the 18-day above the 40-day today and on or below it yesterday (mirror for shorts); it replaces any setup still waiting. The price must first close above the 18-day after the cross (it usually already is), then the first close below it is the dip (option: the first low below it). From the next day a buy stop is placed one tick (0.01) above the previous day's 18-day average and renewed every day until filled or the averages cross back; it fills at that level or at the open if the day opens above it. Only the first dip after a cross is traded, one position at a time; a setup whose dip comes while a position is open waits for the position to close. From the day after the fill, a sell stop one tick below the previous day's 40-day average; once five days have passed after the entry day ('about one week'), halfway between the previous day's 18-day and 40-day averages. The stop only moves in the trade's favour. Stopped out when the low reaches the stop, at the stop or at the open if the day opens below it. Shorts mirror everything. Not coded: the judgement on how quickly the dip follows the cross, the 4-day average rules, futures report days and options.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "slmma_delphic", name: "Delphic Phenomenon: first dip through the 18-day after an 18/40 cross, stop entry, 40-day then halfway stop",
        group: "Lowry moving averages", session: 0, dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[],
        book: "slmma", rules: &[
            "F|\"wait for the eighteen day moving average to cross over the forty day moving average\"; \"it is not a buying situation until the eighteen day moving average is on top of the forty day moving average\" (PDF 14, 17)",
            "E|\"wait for the actual price of the market to go above the eighteen day moving average, and then drop below it, for the first time ... place a buy order just above the eighteen day moving average\" (PDF 15); \"you only place your trade the first time\" (PDF 16)",
            "X|\"place a protective sell stop just below the forty day moving average\" (PDF 15); \"give the market about one week ... I move my protective stop to about half the distance between the eighteen day and forty day moving averages ... on a daily basis, until I am eventually stopped out\" (PDF 16-17)",
            "F|Sell signal is the mirror: 18-day crosses below the 40-day, first rise above the 18-day, sell stop just below it, protective buy stop just above the 40-day (PDF 15-16, chart key PDF 25)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == "slmma_delphic").then_some(("slmma_delphic", "Lowry Delphic Phenomenon", "18/40 days, week then halfway stop"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    (id == "slmma_delphic").then(|| vec![
        p("fast", "Fast average days (book 18)", 18., 2., 100., 1.), p("slow", "Slow average days (book 40)", 40., 3., 250., 1.),
        p("week", "Days after the entry day before the halfway stop (book 'about one week' = 5)", 5., 0., 30., 1.),
        p("dip", "Dip: 0 = a close through the 18-day (reading), 1 = a low (high for shorts) through it", 0., 0., 1., 1.),
        p("tick", "Tick added beyond each average, price units (reading 0.01)", 0.01, 0., 10., 0.01)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let (fast, slow) = (sma(&x.c, g("fast").round().max(1.) as usize), sma(&x.c, g("slow").round().max(1.) as usize));
    let (week, low_dip, tick) = (g("week").round().max(0.) as usize, g("dip").round() >= 1., g("tick"));
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    // setup: (side, seen beyond the 18-day, dipped); position: (side, entry bar, entry px, stop)
    let (mut setup, mut pos, mut out) = (None::<(f64, bool, bool)>, None::<(f64, usize, f64, f64)>, vec![]);
    for t in 1..=to {
        // 1) today's orders, placed at yesterday's close
        if t >= from {
            if let Some((d, ei, ep, st)) = pos {
                if t > ei && d * (if d > 0. { l[t] } else { h[t] }) <= d * st {
                    let px = if d > 0. { o[t].min(st) } else { o[t].max(st) };
                    out.push(Trade { dir: d, legs: vec![Leg { i: ei, px: ep, w: 1. }], exit_i: t, exit_px: px, reason: "protective stop".into(),
                        entry_kind: "stop", signal_i: t, open: false, exits: vec![] });
                    pos = None; // no new entry on the bar of an exit (else branch below)
                }
            } else if let Some((d, _, true)) = setup {
                let lvl = fast[t - 1] + d * tick;
                if t < to && d * (if d > 0. { h[t] } else { l[t] }) >= d * lvl {
                    let px = if d > 0. { o[t].max(lvl) } else { o[t].min(lvl) };
                    pos = Some((d, t, px, f64::NAN));
                    setup = None;
                }
            }
        }
        // 2) today's close: new stop for tomorrow, crosses, dips
        if let Some((d, ei, ep, st)) = pos {
            let base = if t - ei >= week { (fast[t] + slow[t]) / 2. } else { slow[t] - d * tick };
            let ns = if st.is_nan() { base } else if d > 0. { st.max(base) } else { st.min(base) };
            pos = Some((d, ei, ep, if ns.is_nan() { st } else { ns }));
        }
        let (f0, s0, f1, s1) = (fast[t - 1], slow[t - 1], fast[t], slow[t]);
        if f1 > s1 && f0 <= s0 { setup = Some((1., false, false)); } else if f1 < s1 && f0 >= s0 { setup = Some((-1., false, false)); }
        if let Some((d, above, dipped)) = setup {
            if !dipped {
                let thru = if low_dip { if d > 0. { l[t] } else { h[t] } } else { c[t] };
                if above && d * thru < d * f1 { setup = Some((d, true, true)); } else if d * c[t] > d * f1 { setup = Some((d, true, false)); }
            }
        }
    }
    if let Some((d, ei, ep, _)) = pos {
        out.push(Trade { dir: d, legs: vec![Leg { i: ei, px: ep, w: 1. }], exit_i: to, exit_px: c[to], reason: "open".into(), entry_kind: "stop", signal_i: to, open: true, exits: vec![] });
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_reviewed_ids_run() {
        assert!(reviewed("slmma_delphic") && !reviewed("slmma_other"));
        assert_eq!(catalogue().len(), 1);
        assert_eq!(params("slmma_delphic").unwrap().len(), 5);
    }
}
