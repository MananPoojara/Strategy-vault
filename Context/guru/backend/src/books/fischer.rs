//! Robert Fischer (with Jens Fischer) - Fibonacci books. Book ids `rfcfcp` (Candlesticks, Fibonacci, and Chart Pattern
//! Trading Tools, 2003), `rffaps` (Fibonacci Applications and Strategies for Traders, 1993), `rfnft` (The New Fibonacci
//! Trader, 2001, and its Workbook). Reading: claude-continuation-20261020/authors/fischer/agents/. Catalogue only: the five
//! systems rest on a minimum swing size in fixed futures points; R4 #277 (1) makes the default the percent those points
//! were at the book's own test, but lane K77 (12 Nov 2026) found each one still held for its own reason (per card).
//! PHI-ellipses, channels, spirals and candle patterns are judgement.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, book, why, rules| StrategyDef { id, name, group, session: 0, dir: "Long/Short", status: Status::NotTestable(why), needs_volume: false,
        needs_vix: false, needs_series: &[], book, rules, notes: "Read 2026-10-24 (authors/fischer/agents).", claim: None };
    let mut v = vec![
        d("rfcfcp_dtdb", "Double-top / double-bottom swing breakout", "Ch.3 (p.45-51)", "rfcfcp", "Lane K77 (R4 #277 (1)): the swing default would be the percent of price the book's 300 points were at its own test (DJ EuroStoxx 50 futures, Aug 2001 - Aug 2002), but the vault holds no EuroStoxx 50 prices, so that percent cannot be derived from data (Table 4.1's three rounded trade prices, 3,727 / 3,500 / 2,700, are not a period level and disagree with each other: exit 3,550 vs re-entry 3,500), and the 3-trade table cannot be re-run. Held",
          &["E|Long on a close above the last swing peak, short below the last swing valley (p.45-51)", "X|Stop on a close beyond the previous opposite swing point; target one swing size; opposite signal reverses"]),
        d("rfcfcp_fib618", "Fibonacci 61.8% correction system", "Ch.4 (p.52-66); also The New Fibonacci Trader Ch.3", "rfcfcp", "Lane K77 (R4 #277 (1)): the S&P sample rules (p.60) do not say when the 4-day trailing stop starts, and the stop is given two ways (p.55: the peak or valley before the entry; p.60: the low of the impulse's starting day). The plainest reading (zigzag swings of 80 S&P points on daily highs and lows, trail from the day after the entry, stop at the impulse start) on SPX Jan-Nov 2000 gives 17 trades and 7 winners against Table 4.3's 9 trades and 8 winners (p.62), and the table's prices (1,427.50, 1,449.80, 1,517.50) are not SPX prints (futures-like ticks). No reading fixed by the book reproduces the table. Held",
          &["E|After an impulse of at least the minimum swing and a 61.8% correction, buy stop at the previous day's high (p.57-60)", "X|Stop at the low of the impulse's first day; target 0.618 x impulse; trail the lowest low of the previous 4 days"]),
        d("rfcfcp_patterns", "Candlestick entries, chart patterns, PHI-channels and PHI-ellipses", "Ch.1-2, 5-9", "rfcfcp",
          "'end of a downtrend' is never defined and there is no exit besides the stop; the authors say PHI-ellipse signals cannot be computerised (p.146)", &["E|Judgement"]),
        d("rffaps_breakout", "Swing breakout with 0.618 target", "Ch.4 (p.54-58)", "rffaps", "Lane PH (14 Nov 2026, R4 #293 (4), #297, #240): Norgate holds British pound futures (&6B, 1979-2020), so the book's 400 points can run literally, and the 7-trade table of p.57-58 is legible in the OCR (buy 175.40 / sell 182.38 +6.98, sell 178.90 / buy 176.10 +2.78, buy 176.80 / sell 173.12 -3.68, sell 173.10 / buy 175.98 -2.88, buy 177.10 / sell 179.57 +2.47, sell 172.50 / buy 169.16 +3.34, buy 172.06 / sell 174.98 +2.92). Still held: (1) more than two details are unstated (how swings are measured - highs/lows or closes; whether the breakout needs a tick through the swing point; whether the 2-day trailing stop is live on the entry day; what happens on an opposite breakout), and the plainest reading (zigzag on highs/lows, stop through the swing point, 2-day stop live from the entry day) gives 7 trades and 4 winners on 10/1991-5/1992 but none of the book's trades (its first buy, 175.40, is a Norgate swing LOW, not a breakout level); (2) 6.6% of that reading's trades over 1979-2020 enter and touch the 2-day stop inside the entry bar, which daily bars cannot order, and no pound minute bars exist (R4 #240, limit 5%). Verifier question filed. Held",
          &["E|Buy a break of the previous swing high (p.54-58)", "X|Target breakout level + 0.618 x the prior swing; stop the previous 2-day low, trailed"]),
        d("rffaps_wave3", "Wave-3 extension counter-trend trade", "Ch.5 (p.73-86); band from the NFT Workbook (p.60)", "rffaps", "Lane K77 (R4 #277 (1)): examples only (no trade table) on soybeans, the British pound and the yen, none of them held in the vault for the percent derivation; the cancellation of a setup is not stated and the p.77 target example disagrees with itself ($6.24 computed vs $6.39 printed). Held",
          &["E|At 1.618 x wave 1, sell on a break of the previous two lows or a close below the low of the highest day (p.73-86)", "X|Stop one tick above the previous high; target 50% of the move or a 2-day trailing stop"]),
        d("rffaps_timegoal", "Fibonacci time-goal-day system", "Ch.7 (p.103-125)", "rffaps", "Lane K77 (R4 #277 (1)): the stop is a 'price square' drawn on the chart (side = five business days, p.123), so it depends on the chart's scale; the p.107 product table's S&P line (filter 400, stop 400) is a 1993 futures setting with no S&P test to derive a percent from (the tests are on the D-mark and the pound). Held",
          &["E|After a passed time-goal day, sell on a close below the low of the highest day or a break of the previous 4 lows (p.103-125)", "X|Fixed-point stop from the product table (p.107); reverse on the opposite signal"]),
        d("rfnft_tools", "Time-goal days, 3- and 5-wave extensions, PHI-spirals", "The New Fibonacci Trader", "rfnft",
          "'We do not explicitly impose entry rules ... exit rules' (p.276); the complete correction system is the same as rfcfcp_fib618", &["E|Judgement"]),
    ];
    // lane PH (R4 #293 (4), #297): the double-top / double-bottom swing breakout runs literally on Norgate EURO STOXX 50 futures
    v[0].status = Status::Live;
    v[0].rules = &["F|\"The 13 turning points marked on the chart are based on a swing size of 300 points\" (p.47; DJ EuroStoxx 50 futures, daily)",
        "E|\"Traders get a long or short signal if the swing size is broken by the daily close (on daily data)\" (p.48; Figure 4.2: buy above the prior swing peak, sell below the prior swing valley); \"A double top pattern is only complete if the valley between the two peaks is broken by a daily close\" (p.46)",
        "X|\"The investor gets stopped out of a position if, after the entry, a previous peak or a previous valley is broken based on a closing price\" (p.48; Figure 4.3)",
        "X|\"we use the total swing size as the profit target (e.g., if the swing size is 300 points, our profit target is 300 points)\" (p.49); Table 4.1 'Sell reverse': an opposite signal reverses"];
    v[0].notes = DTDB_NOTE;
    v[0].claim = crate::connors::claim("3", "67%", "+141 points", "-", "DJ EuroStoxx 50 futures 08/2001-08/2002, 300-point swing (Table 4.1)");
    v
}
const DTDB_NOTE: &str = "[Adapted: 2 unstated details, R4 #297; R4 #293 (1), (4)] Runs only on the Norgate EURO STOXX 50 continuous contract (back-adjusted daily bars; points survive the roll adjustment), with the book's 300 index points literal (p.size). Swing points: unstated in the book (it gives only the minimum size) - vault reading: a zigzag on daily highs and lows: a peak is confirmed when the low falls 300 points below the highest high since the last valley, a valley when the high rises 300 points above the lowest low since the last peak; a point counts from the day after it is confirmed. Long when the close crosses above the last confirmed peak, short when it crosses below the last confirmed valley (fill at that close); stop: a close beyond the swing point opposite the entry at entry time (fill at that close); an opposite signal reverses. Profit target 300 points from the entry - unstated how it fills - vault reading: a limit order from the next day, filled at the target or at the open when the market opens beyond it. Close entries and close stops leave no order question inside a day (R4 #240). Book check (Table 4.1, Aug 2001 - Aug 2002, 3 trades, -177 / +300 / +300 points; trade 2's entry '3,500' is a misprint for the 3,550 reverse of trade 1): started on the chart's first day (2001-08-01) the same window gives 4 closed trades, -609 (a short of 2001-09-20 stopped 2001-10-23), -91 (a long 3,608 -> 3,517 where the book has 3,727 -> 3,550), +300 (3,541 -> 3,241 vs the book's 3,550 -> 3,250) and +300 (2,703 -> 2,403 vs 2,700 -> 2,400); with the swing history before it, 5 (a further +300 short from 2001-08-10): count 4-5 vs 3 fails R4 #168, so unverified. Norgate continuous futures, ends 2020-09-11; roll method not stated by the vendor. EURO STOXX 50 points: EUR 10 per point (no US$ amount is used).";
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id.starts_with("rfcfcp_") || id.starts_with("rffaps_") || id.starts_with("rfnft_")).then_some(("rf_nt", "Fischer Fibonacci systems", "-"))
}
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    (id == "rfcfcp_dtdb").then(|| vec![ParamDef { key: "size", label: "Swing size and profit target (index points; the book: 300 on EuroStoxx 50 futures)", def: 300., min: 1., max: 5000., step: 10. }])
}

/// Zigzag swing points on daily highs and lows: (last confirmed peak, last confirmed valley) as known at each bar's close.
fn zigzag(x: &Ctx, to: usize, size: f64) -> Vec<(Option<f64>, Option<f64>)> {
    let (mut peak, mut val, mut d, mut hi, mut lo) = (None, None, 0i8, x.h[0], x.l[0]);
    (0..=to).map(|i| {
        if d >= 0 { hi = hi.max(x.h[i]); }
        if d <= 0 { lo = lo.min(x.l[i]); }
        if d >= 0 && hi - x.l[i] >= size { peak = Some(hi); d = -1; lo = x.l[i]; } // the high is a peak: the low fell `size` below it
        else if d <= 0 && x.h[i] - lo >= size { val = Some(lo); d = 1; hi = x.h[i]; } // the low is a valley
        (peak, val)
    }).collect()
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if id != "rfcfcp_dtdb" || from > to || to >= x.r.len() { return None; }
    let size = p.get("size").copied().unwrap_or(300.);
    let z = zigzag(x, to, size);
    let mk = |d: i8, ei: usize, ep: f64, xi: usize, xp: f64, why: &str, open: bool| Trade { dir: d as f64, legs: vec![Leg { i: ei, px: ep, w: 1. }], exit_i: xi, exit_px: xp,
        reason: why.into(), entry_kind: "close", signal_i: xi, open, exits: vec![] };
    let (mut out, mut pos, mut ep, mut ei, mut stop) = (vec![], 0i8, 0., 0, None::<f64>);
    for i in from.max(1)..=to {
        let (pk, vl) = z[i - 1]; // swing points known before today
        let (o, c, c1) = (x.o[i], x.c[i], x.c[i - 1]);
        if pos != 0 {
            let tgt = ep + pos as f64 * size;
            if (pos == 1 && x.h[i] >= tgt) || (pos == -1 && x.l[i] <= tgt) {
                out.push(mk(pos, ei, ep, i, if pos == 1 { tgt.max(o) } else { tgt.min(o) }, "profit target (one swing size)", false));
                pos = 0;
            }
        }
        let mut sig = 0i8;
        if pk.is_some_and(|p| c > p && c1 <= p) { sig = 1; }
        if vl.is_some_and(|v| c < v && c1 >= v) { sig = -1; }
        if pos != 0 && stop.is_some_and(|s| (pos == 1 && c < s) || (pos == -1 && c > s)) { out.push(mk(pos, ei, ep, i, c, "stop: close beyond the previous swing point", false)); pos = 0; }
        if sig != 0 && sig != pos {
            if pos != 0 { out.push(mk(pos, ei, ep, i, c, "opposite signal (reverse)", false)); }
            (pos, ep, ei, stop) = (sig, c, i, if sig == 1 { vl } else { pk });
        }
    }
    if pos != 0 { out.push(mk(pos, ei, ep, to, x.c[to], "open", true)); }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn dtdb_zigzag_close_breaks_target() {
        // highs/lows +-1 around closes 100 110 120 105 95 (peak 121 confirmed when the low 94 is 27 below), 100 125 (close crosses the
        // peak 121 -> long at 125), 149.5 (high 150.5: target 125 + 25 = 150 filled), swing size 25
        let c = [100., 110., 120., 105., 95., 100., 125., 149.5];
        let r: Vec<Row> = c.iter().enumerate().map(|(i, &v)| Row { ticker: "EURO STOXX 50 (Norgate &FESX)".into(), date: format!("2001-01-{:02}", i + 2), open: v, high: v + 1., low: v - 1., close: v, volume: None }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let mut p = Params::new(); p.insert("size".into(), 25.);
        let t = run("rfcfcp_dtdb", &x, 0, 7, &p).unwrap();
        assert_eq!(t.len(), 1);
        assert!(t[0].dir == 1. && t[0].legs[0].i == 6 && t[0].legs[0].px == 125. && t[0].exit_i == 7 && t[0].exit_px == 150. && !t[0].open, "{:?}", (t[0].legs[0].i, t[0].exit_i, t[0].exit_px));
        assert!(run("rffaps_breakout", &x, 0, 7, &p).is_none());
    }
}
