//! 'The Outsider Method' (X-Trader, 1 page; research share Trading Knowledge III/!PDFs/The Outsider Method.pdf). Book id `outsider`.
//! 15-minute 9-period EMA pattern rule written for GBP/USD, run here on real 1-minute bars aggregated to 15 minutes. Lane K4 (24 Nov 2026, R4 #397 I584).
//! Independent pandas reference: authors/code-1123/ref/outsider_ref.py.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};
use crate::intraday::{fill_px, touched, Bar1, Level};

pub const BOOK: &str = "outsider";
const BAR: u16 = 15;

const NOTE: &str = "[Declared readings] The Outsider Method (X-Trader), the whole page. A 9-period EMA on a 15-minute chart. Pattern candle: body and shadows do not touch the EMA (bullish: the low is above the EMA; bearish: the high is below it), and it closes above the previous candle's high (bullish) or below the previous candle's low (bearish). Entry at the open of the next candle. Stop at the pattern candle's low (bullish) / high (bearish); target = entry +/- n x the pattern candle's body (n = 2, 'a good starting point'); once the profit reaches 20 points the stop moves to breakeven. Four labelled unstated details, each its own: (1) 15-minute bars are measured from the session's first minute; (2) the EMA runs inside the session, seeded with the first bar's close (no overnight carry); (3) the '20 points' of the GBP/USD page are scaled to 0.15% of the entry price for the breakeven trigger; (4) a trade still open at the session end closes at the last close (the page names no other exit). Not counted: the 'at least 1 pip from the EMA' is only 'ideally' and is not applied; a signal while a trade is open is ignored (one position with its stop and target live, as printed); a minute touching two live levels uses the level nearer its open and the breakeven stop is live from the next minute (the lane's minute-path convention).";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "outsider_ema9", name: "The Outsider Method: 15-minute candle clear of the 9 EMA that closes beyond the previous extreme, 2 x body target", group: "X-Trader · The Outsider Method", session: 0,
        dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
        rules: &[
            "F|\"Place a 9-period EMA on a 15 minutes GBP/USD chart ... Body and shadows can't touch the EMA ... Close should be above the previous high (bullish case) or below the previous low (bearish case)\"",
            "E|\"Bullish case: long at the opening of the next candle. Bearish case: short at the opening of the next candle.\"",
            "X|\"Stop loss is set at the low (bullish case) or high (bearish case) of the candle pattern ... Profit target should be set to n times the size of the pattern candle body ... n = 2 ... If we have a profit of 20 points or more, move your stop loss to breakeven\""],
        notes: NOTE, claim: None }]
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    if id != "outsider_ema9" { return None; }
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(vec![p("n", "Target = n x the pattern candle body (printed: n = 2 suggested)", 2., 1., 4., 0.5),
              p("be", "Move the stop to breakeven at this profit, % of the entry price (the printed 20 pips = 0.15%)", 0.15, 0.05, 0.5, 0.05)])
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == "outsider_ema9").then_some(("outsider_ema9", "The Outsider Method", "15-minute 9 EMA pattern, 2 x body target"))
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if id != "outsider_ema9" { return None; }
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d);
    let (n, be) = (g("n", 2.), g("be", 0.15) / 100.);
    let mut out = Vec::new();
    for j in from..=to.min(x.r.len().saturating_sub(1)) {
        let Some(b) = crate::intraday_w2b::day(x, j) else { continue };
        let Some(first) = b.first() else { continue };
        let m0 = first.m;
        // 15-minute candles: (first minute index, o, h, l, c)
        let mut cs: Vec<(u16, usize, f64, f64, f64, f64)> = vec![]; // (candle number, first minute index, o, h, l, c): existing candles only
        for (ix, z) in b.iter().enumerate() {
            let k = (z.m - m0) / BAR;
            match cs.last_mut() { Some(c) if c.0 == k => { c.3 = c.3.max(z.h); c.4 = c.4.min(z.l); c.5 = z.c; } _ => cs.push((k, ix, z.o, z.h, z.l, z.c)) }
        }
        let mut ema = f64::NAN; let a = 2. / 10.;
        let mut pos: Option<(f64, f64, f64, f64, bool, usize)> = None; // dir, entry, stop, target, be done, from minute
        let mut ci = 0usize; // next candle whose signal is evaluated
        let mut ix = 0usize;
        while ix < b.len() {
            let m = b[ix];
            // evaluate the signal of candle ci when its last minute has just passed (we are at the first minute of candle ci+1)
            while ci + 1 < cs.len() && cs[ci + 1].1 == ix {
                let c = cs[ci];
                ema = if ema.is_nan() { c.5 } else { ema + a * (c.5 - ema) };
                if ci > 0 && pos.is_none() {
                    let pv = cs[ci - 1];
                    let bull = c.4 > ema && c.5 > pv.3; let bear = c.3 < ema && c.5 < pv.4;
                    if bull || bear {
                        let dir = if bull { 1.0 } else { -1.0 };
                        let entry = m.o; let body = (c.5 - c.2).abs();
                        let (stop, tgt) = if bull { (c.4, entry + n * body) } else { (c.3, entry - n * body) };
                        if (bull && stop < entry && tgt > entry) || (bear && stop > entry && tgt < entry) { pos = Some((dir, entry, stop, tgt, false, ix)); }
                    }
                }
                ci += 1;
            }
            if let Some((dir, entry, stop, tgt, bed, f)) = pos {
                if ix > f {
                    let mut cand: Vec<(u8, f64, bool)> = vec![(0, stop, dir < 0.), (1, tgt, dir > 0.)];
                    if !bed { cand.push((2, entry * (1. + dir * be), dir > 0.)); }
                    let mut hit: Vec<&(u8, f64, bool)> = cand.iter().filter(|c| touched(&m, Level { up: c.2, px: c.1 })).collect();
                    hit.sort_by(|p1, p2| (p1.1 - m.o).abs().total_cmp(&(p2.1 - m.o).abs()));
                    if let Some(c) = hit.first() {
                        let fp = fill_px(&m, Level { up: c.2, px: c.1 });
                        match c.0 {
                            2 => { pos = Some((dir, entry, entry, tgt, true, f)); }
                            k => { out.push(Trade { dir, legs: vec![Leg { i: j, px: entry, w: 1.0 }], exit_i: j, exit_px: fp, reason: if k == 1 { "target".into() } else if bed { "breakeven stop".into() } else { "stop".into() }, entry_kind: "open", signal_i: j, open: false, exits: vec![] }); pos = None; }
                        }
                    }
                }
            }
            ix += 1;
        }
        if let Some((dir, entry, _, _, _, _)) = pos { out.push(Trade { dir, legs: vec![Leg { i: j, px: entry, w: 1.0 }], exit_i: j, exit_px: b[b.len() - 1].c, reason: "day end".into(), entry_kind: "open", signal_i: j, open: false, exits: vec![] }); }
    }
    Some(out)
}
