//! Dave Landry, "Swing Trading For A Living" (TradingMarkets seminar workbook, 2002; 257 scanned pages).
//! Source: Books/Dave Landry/Dave Landry - Swing Trading For A Living.pdf. Page refs are PDF pages (the workbook has no printed numbers).
//! Only the two market-timing systems that are new to the vault are coded here; the workbook's other systems repeat rules already
//! in the vault (Oscillator Swing System + CVR III-Modified = dlost_oss / dlost_cvr3m, CVR-III = ats_cvr3, TRIN Reversals =
//! ptjmt_trin_reversals, TKO / Bow Ties = dlost_tko / dlost_bow_tie). Extraction and verification: ~/.local/share/strategy-vault/library/dlstfl/.
use crate::connors::{claim, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

const BOOK: &str = "dlstfl";
/// Admitted ids (authors/landry/laneb/SOURCE-CONTRACT.md).
pub const REVIEWED: &[&str] = &["dlstfl_close_count", "dlstfl_vix_hl"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    let d = |id, name, group, dir, status, rules, notes, c| StrategyDef {
        id, name, group, session: 0, dir, status, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: c,
    };
    let vx = |s: StrategyDef| StrategyDef { needs_vix: true, ..s };
    vec![
        d("dlstfl_close_count", "Close Count", "Market Timing Systems", "Long/Short", Live,
          &["F|\"Yesterday, the market must have closed lower for four consecutive days\" (p.41): the four closes ending yesterday are each below the close before",
            "E|\"Go long today if the market closes above its open\": buy on today's close",
            "X|\"Risk 2% of index's value\": protective stop 2% below the entry, working from the next day (filled at the stop, or at the open on a gap through it)",
            "X|\"Exit on first profitable close\": the first close above the entry price",
            "E|Shorts reversed: four consecutive higher closes ending yesterday, today closes below its open: sell short on the close; stop 2% above; exit on the first close below the entry"],
          "Book: TradeStation, S&P 500 futures (CME daily) 01/05/1990-06/03/2002, one contract. Here SPX / SPY / NIFTY 50 (instrument swap). The stop is read as an intraday stop order (TradeStation SetStopLoss); stop_on_close = 1 is the [Not from the book] closing-basis reading. One position at a time; signals while in a trade are ignored. Landry: 'Not intended to be a stand alone system' (p.41).",
          claim("144 (L 73 / S 71)", "88% (L 86% / S 89%)", "$952.26 per contract (L $1,209.42 / S $687.85)", "3 bars winners, 7 losers", "S&P futures 01/05/1990-06/03/2002")),
        vx(d("dlstfl_vix_hl", "VIX High/Low System", "Market Timing Systems", "Long/Short", Live,
          &["E|\"Buy Market When VIX Hits 10-day High\" (p.54): the VIX high is the highest high of the last 10 days (today included); buy on the close",
            "X|\"Exit When VIX Hits 5-day Low\": the VIX low is the lowest low of the last 5 days; exit on the close",
            "E|\"Short Market When VIX Hits 10-day Low\"; \"Exit When VIX Hits 5-Day High\"",
            "X|An exit and the opposite entry on the same close: exit, then enter the new side"],
          "Book: S&P 500 futures (CME daily) 01/05/1990-06/10/2002 with the CBOE VIX of the time (OEX-based, now VXO: src = 1 reads VXO_OHLC). 'Hits' read as the VIX's intraday high/low touching the N-day extreme (hit_on_close = 1 = [Not from the book] closing-basis reading). Fills on the S&P close (the workbook's other VIX systems trade on the close). Landry: 'Intended only to show negative correlation (i.e., not intended to be a stand alone system)' (p.57). India: India VIX + NIFTY 50.",
          claim("243 (L 125 / S 118)", "68% (L 73% / S 63%)", "$1,566.72 per contract (L $1,555.60 / S $1,578.50)", "5 bars winners, 9 losers", "S&P futures 01/05/1990-06/10/2002"))),
        d("dlstfl_witch_hat", "Witch Hats", "Trend Resumption Patterns", "Long/Short", NotTestable("not testable: 'strong uptrend, preferably an accelerating uptrend', a pivot low that is 'often obvious after the stock begins to retrace' and a retrace 'sharply to near the level of the pivot low' are chart judgements (p.94)"),
          &["F|Strong uptrend; pivot low; trend resumes; retrace sharply to near the pivot low", "E|Go long .10 above today's high", "X|Protective stop .10 below the pivot low or the high of (5)"],
          "Also Dave Landry's 10 Best (2003) ch.4.", None),
        d("dlstfl_first_thrust", "First Thrust (transitional pattern)", "Trend Transition", "Long/Short", NotTestable("not testable: 'Stock must have recently made major new low' and 'then rally sharply' have no stated lookback or size (p.110)"),
          &["F|Recent major new low, then a sharp rally", "E|Lower low and lower high; go long above the high of #3", "X|Protective stop below the low of #3"],
          "Also Dave Landry's 10 Best (2003).", None),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "dlstfl_close_count" => ("dlstfl_timing", "Landry market timing (2002 workbook)", "Close Count"),
        "dlstfl_vix_hl" => ("dlstfl_timing", "Landry market timing (2002 workbook)", "VIX High/Low"),
        "dlstfl_witch_hat" => ("dlstfl_patterns", "Landry transition patterns", "Witch Hat"),
        "dlstfl_first_thrust" => ("dlstfl_patterns", "Landry transition patterns", "First Thrust"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let side = || p("side", "Side: 0 = long and short (book), 1 = long only, 2 = short only", 0.0, 0.0, 2.0, 1.0);
    Some(match id {
        "dlstfl_close_count" => vec![p("days", "Consecutive closes against the trade", 4.0, 2.0, 7.0, 1.0), p("risk", "Stop % from the entry", 2.0, 0.5, 5.0, 0.5),
            p("stop_on_close", "[Not from the book] 1 = stop only on a close beyond it", 0.0, 0.0, 1.0, 1.0), side()],
        "dlstfl_vix_hl" => vec![p("entry_n", "VIX N-day high/low for entries", 10.0, 5.0, 30.0, 1.0), p("exit_n", "VIX N-day low/high for exits", 5.0, 2.0, 15.0, 1.0),
            p("hit_on_close", "[Not from the book] 1 = the VIX close (not its range) must make the extreme", 0.0, 0.0, 1.0, 1.0),
            p("src", "Volatility index: 0 = server VIX, 1 = VXO_OHLC (book's OEX VIX)", 0.0, 0.0, 1.0, 1.0), side()],
        _ => return None,
    })
}

fn close_trade(dir: f64, i: usize, px: f64) -> Trade {
    Trade { dir, legs: vec![Leg { i, px, w: 1.0 }], exit_i: i, exit_px: px, reason: String::new(), entry_kind: "close", signal_i: i, open: false, exits: Vec::new() }
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if from > to || to >= x.c.len() { return Some(vec![]); }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let u = |k: &str| g(k).round().max(1.0) as usize;
    let side = g("side").round() as i64;
    let (wl, ws) = (side != 2, side != 1);
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let mut out = Vec::new();
    let mut cur: Option<Trade> = None;
    match id {
        "dlstfl_close_count" => {
            let (n, risk, on_close) = (u("days"), g("risk") / 100.0, g("stop_on_close") >= 0.5);
            let run = |i: usize, down: bool| i > n && (i - n..i).all(|k| if down { c[k] < c[k - 1] } else { c[k] > c[k - 1] });
            for i in from..=to {
                if let Some(mut t) = cur.take() {
                    let (d, e) = (t.dir, t.legs[0].px);
                    let sp = e * (1.0 - d * risk);
                    let stop = if on_close { (d * (c[i] - sp) <= 0.0).then(|| c[i]) } else if d > 0.0 { (l[i] <= sp).then(|| o[i].min(sp)) } else { (h[i] >= sp).then(|| o[i].max(sp)) };
                    if let Some(px) = stop {
                        t.exit_i = i; t.exit_px = px; t.signal_i = i; t.reason = "2% stop".into(); out.push(t);
                    } else if d * (c[i] - e) > 0.0 {
                        t.exit_i = i; t.exit_px = c[i]; t.signal_i = i; t.reason = "First profitable close".into(); out.push(t);
                    } else {
                        cur = Some(t);
                    }
                    continue;
                }
                if i >= to { continue; }
                if wl && run(i, true) && c[i] > o[i] { cur = Some(close_trade(1.0, i, c[i])); }
                else if ws && run(i, false) && c[i] < o[i] { cur = Some(close_trade(-1.0, i, c[i])); }
            }
        }
        "dlstfl_vix_hl" => {
            let n = c.len();
            let b = if g("src").round() as i64 == 1 { "VXO_OHLC" } else { "VIX" };
            let col = |k: &str| x.series(&format!("{b}.{k}")).map(<[f64]>::to_vec).unwrap_or_else(|| vec![f64::NAN; n]);
            let (vh, vl, vc) = (col("High"), col("Low"), col("Close"));
            let on_close = g("hit_on_close") >= 0.5;
            // the VIX's own consecutive bars
            let days: Vec<usize> = (0..n).filter(|&i| vh[i].is_finite() && vl[i].is_finite() && vc[i].is_finite()).collect();
            let mut at = vec![None; n];
            for (k, &i) in days.iter().enumerate() { at[i] = Some(k); }
            let (hs, ls): (Vec<f64>, Vec<f64>) = if on_close { (days.iter().map(|&i| vc[i]).collect(), days.iter().map(|&i| vc[i]).collect()) }
                else { (days.iter().map(|&i| vh[i]).collect(), days.iter().map(|&i| vl[i]).collect()) };
            let hi = |k: usize, m: usize| k + 1 >= m && hs[k + 1 - m..=k].iter().all(|&v| hs[k] >= v);
            let lo = |k: usize, m: usize| k + 1 >= m && ls[k + 1 - m..=k].iter().all(|&v| ls[k] <= v);
            let (en, ex) = (u("entry_n"), u("exit_n"));
            for i in from..=to {
                let Some(k) = at[i] else { continue };
                if let Some(mut t) = cur.take() {
                    let out_now = if t.dir > 0.0 { lo(k, ex) } else { hi(k, ex) };
                    if out_now {
                        t.exit_i = i; t.exit_px = c[i]; t.signal_i = i;
                        t.reason = format!("VIX {}-day {}", ex, if t.dir > 0.0 { "low" } else { "high" });
                        out.push(t);
                    } else {
                        cur = Some(t);
                        continue;
                    }
                }
                if i >= to { continue; }
                if wl && hi(k, en) { cur = Some(close_trade(1.0, i, c[i])); }
                else if ws && lo(k, en) { cur = Some(close_trade(-1.0, i, c[i])); }
            }
        }
        _ => return None,
    }
    if let Some(mut t) = cur { t.exit_i = to; t.exit_px = c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true; out.push(t); }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn bars(v: &[(f64, f64, f64, f64)]) -> Vec<Row> {
        v.iter().enumerate().map(|(i, &(o, h, l, c))| Row { ticker: "T".into(), date: format!("2000-01-{:02}", i + 1), open: o, high: h, low: l, close: c, volume: None }).collect()
    }
    #[test]
    fn close_count_buys_after_four_down_closes_and_exits_on_first_profit() {
        let r = bars(&[(100., 101., 99., 100.), (99., 100., 98., 99.), (98., 99., 97., 98.), (97., 98., 96., 97.), (96., 97., 95., 96.),
            (95., 97., 94., 96.5), (96., 96.6, 95., 96.0), (96., 98., 95.5, 97.0)]);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("dlstfl_close_count", &x, 0, 7, &Params::new()).unwrap();
        assert_eq!(t.len(), 1);
        assert_eq!((t[0].legs[0].i, t[0].legs[0].px), (5, 96.5)); // 4 lower closes to bar 4, bar 5 closes above its open
        assert_eq!((t[0].exit_i, t[0].exit_px), (7, 97.0)); // bar 6 close 96.0 not profitable, bar 7 is
    }
}

// Instrument admission: Dave Landry 2002 workbook 2026-10-26 (lane B) block, moved from instrument_policy.rs on 31 Oct 2026 (lane ST).
// Admission only for the module REVIEWED lists (authors/landry/laneb/SOURCE-CONTRACT.md); kept in one block to merge.
// Runs on actual indices, ETFs and stocks, US and India (index_etf_or_stock, nifty50 basket, declared application).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book=="dlstfl" && crate::books::dlstfl::reviewed(def.id),
    label: "Landry tested these market-timing rules on S&P 500 futures (1990-2002) and calls them 'not intended to be a stand alone system'; runs on indices, ETFs and stocks (US with the CBOE VIX, India with India VIX) are a declared application, one unit per trade; cash-index shorts are hypothetical.",
    basis: "Dave Landry, Swing Trading For A Living (TradingMarkets workbook 2002), p.41 and p.54",
};
