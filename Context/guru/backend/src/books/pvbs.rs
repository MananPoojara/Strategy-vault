//! Franklin Ochoa, "Secrets of a Pivot Boss" (Wiley / Pivot Boss, 2012), Camarilla fourth-layer breakout, PDF pp. 217, 221, 231-235
//! (research share Trading Books/Franklin Ochoa - Secrets of a Pivot Boss). Book id `pvbs`. Lane K2 (23 Nov 2026, R4 #340): NIFTY 50 /
//! NIFTYBANK futures 5-minute bars (the book's example frame, Figure 7.10); levels from the previous futures day.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "pvbs";
const ID: &str = "pvbs_cam_h4";

const NOTE: &str = "A|Adapted (R4 #340, lane K2 23 Nov 2026), UNVERIFIED: the book trades US futures and stocks and prints chart examples, not a result for this rule; it runs on the NIFTY 50 / NIFTYBANK FUTURES series (lane DI 1-minute; nearest-expiry continuous, not back-adjusted; days with a blank volume absent, so 'previous day' is the previous day present), 5-minute bars aligned to the 09:15 open. Levels from the previous session's high/low/close: RANGE = H - L; H4 = C + RANGE*1.1/2, H3 = C + RANGE*1.1/4, L3 = C - RANGE*1.1/4, L4 = C - RANGE*1.1/2, H5 = (H/L)*C, L5 = C - (H5 - C) (p. 217, 221). Long when a bar closes above H4 with the stop at H3 and the target at H5; short when a bar closes below L4 with the stop at L3 and the target at L5 (pp. 231-235). The H3/L3 reversal trades need candlestick confirmation (discretionary) and are not coded. Readings, 4, all labelled: (1) 5-minute bars (the book's example frame); (2) entry at the close of the breakout bar; (3) one long and one short trade at most per day, anything open is closed at the day's last bar close; (4) '(High/Low) x Close' in H5 uses the previous session's high, low and close, and when a bar reaches both the stop and the target the stop is taken first. No costs.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: ID, name: "Camarilla H4 / L4 breakout (5-minute futures)", group: "Secrets of a Pivot Boss (2012)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
        rules: &["F|RANGE = prior high - low; H4 = C + RANGE*1.1/2; H3 = C + RANGE*1.1/4; L3 = C - RANGE*1.1/4; L4 = C - RANGE*1.1/2; H5 = (H/L)*C; L5 = C - (H5-C) (pp. 217, 221)",
            "E|Buy when a bar closes above H4; sell short when a bar closes below L4 (pp. 231-235)", "X|Stop at H3 (shorts: L3), target H5 (shorts: L5); flat at the day's close", NOTE],
        notes: "Camarilla fourth-layer breakout, Secrets of a Pivot Boss pp. 231-235. H3/L3 reversal setups are not coded (candlestick confirmation).", claim: None }]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { (id == ID).then_some(("pvbs", "Camarilla breakout", "")) }
pub fn params(id: &str) -> Option<Vec<ParamDef>> { (id == ID).then(|| vec![ParamDef { key: "bar", label: "Bar size (minutes; 5 per Figure 7.10)", def: 5., min: 1., max: 30., step: 1. }]) }

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if id != ID || to >= x.c.len() || from > to { return None; }
    let n = p.get("bar").copied().unwrap_or(5.0).round().max(1.0) as u16;
    let f = x.fut?;
    let mut out = vec![];
    let mut prev: Option<(f64, f64, f64)> = None; // previous day's high, low, close
    for i in from..=to {
        let r = &x.r[i];
        let Some((m, _)) = f.fut_day(&r.date) else { continue };
        if m.is_empty() { continue; }
        let (open, close) = (crate::intraday::session_open(&r.ticker, &r.date), crate::intraday_ep::session_close(&r.ticker));
        let bars = crate::intraday_ep::nbars(&m, open, n, close);
        let today = (m.iter().map(|b| b.h).fold(f64::MIN, f64::max), m.iter().map(|b| b.l).fold(f64::MAX, f64::min), m[m.len() - 1].c);
        if let Some((h, l, c)) = prev {
            let rg = h - l;
            let (h4, h3, l3, l4) = (c + rg * 1.1 / 2.0, c + rg * 1.1 / 4.0, c - rg * 1.1 / 4.0, c - rg * 1.1 / 2.0);
            let h5 = h / l * c;
            let l5 = c - (h5 - c);
            let (mut done_l, mut done_s) = (false, false);
            let mut pos: Option<(f64, f64)> = None; // dir, entry
            for (k, b) in bars.iter().enumerate() {
                if let Some((d, ep)) = pos {
                    let (stop, tgt) = if d > 0.0 { (h3, h5) } else { (l3, l5) };
                    let hit_stop = if d > 0.0 { b.l <= stop } else { b.h >= stop };
                    let hit_tgt = if d > 0.0 { b.h >= tgt } else { b.l <= tgt };
                    let xp = if hit_stop { Some((if d > 0.0 { b.o.min(stop) } else { b.o.max(stop) }, "stop at H3/L3")) }
                        else if hit_tgt { Some((if d > 0.0 { b.o.max(tgt) } else { b.o.min(tgt) }, "target H5/L5")) }
                        else if k + 1 == bars.len() { Some((b.c, "exit at the close")) } else { None };
                    if let Some((px, why)) = xp { out.push(crate::intraday::day_trade(d, i, ep, i, px, why, "close", false)); pos = None; }
                    continue;
                }
                if !done_l && b.c > h4 { pos = Some((1.0, b.c)); done_l = true; }
                else if !done_s && b.c < l4 { pos = Some((-1.0, b.c)); done_s = true; }
            }
        }
        prev = Some(today);
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn defaults() { assert_eq!(params(ID).unwrap()[0].def, 5.0); assert!(params("x").is_none()); }
}
