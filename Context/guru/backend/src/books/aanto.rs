//! Peter W. Aan, "Trend One" (PWA Futures, 1999). Book id `aanto`. Source copy: Books/Peter W. Aan/Trend One/
//! (TrendOne.pdf = 3-page sales brochure; TREND ONE.ELD + Code.rar = TradeStation code; Code.txt re-read 23 Nov 2026).
//! The brochure gives no rules. The code in Code.rar (Code.txt) resets its swing high/low at every new DATE and only
//! places entries while DATE TOMORROW = DATE, i.e. it is written for intraday bars (the uploader's Readme: "use it with
//! 15min S&P e-mini data"). Lane I2 (23 Nov 2026, lead instruction): coded literally from Code.txt on the NIFTY 50 /
//! NIFTYBANK futures 1-minute series (Ctx::fut, lane DI), 15-minute session-aligned bars, as a labelled R4 #340 adaptation.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "aanto";
const ID: &str = "aanto_trend_one";

const NOTE: &str = "A|Adapted (R4 #340, lead 23 Nov 2026), UNVERIFIED: the brochure (TrendOne.pdf) prints no rules and no result for this code; the rules are Code.txt (TradeStation 8 export; its dollar inputs PositionBasis / LongTargetAmt / StopLossAmt are declared but never read by the code, so the rule has no currency amount). The code was written for 15-minute S&P e-mini bars (Readme.txt); it runs here on the NIFTY 50 / NIFTYBANK FUTURES 1-minute series staged by lane DI (NIFTY_50.FUT1 2018-12-28 to 2026-11-12, NIFTYBANK.FUT1 2023-09-06 to 2026-08-05; nearest-expiry continuous, NOT back-adjusted: a position carried over a contract roll includes the roll gap; days with a blank volume are absent from the series, so a bar after such a gap follows the previous held day). Session 09:15-15:30 IST (09:00 before 2010-10-18, 09:55 before 2010-01-04; the NIFTY futures file starts 2018, so 09:15), 15-minute bars aligned to the session open (09:15, 09:30 ... 15:15, 25 bars a day; the 15:30 auction minute is dropped), one continuous series across days as in TradeStation (Average(Range,20), LOW[1] and BARSSINCEENTRY carry across days; the swing high/low reset at each new date). Readings of what Code.txt leaves to the platform, 6, all labelled (R4 #408: the honest numbered list of the card-specific choices, which is longer than the three counted under R4 #343, is at the end of this note): (1) a limit order for the next bar fills if that bar trades through its level, at the level, or at the bar's open when the bar opens through it; (2) a market order fills at the next bar's open, and market orders are tried before limits; (3) of two limit orders alive in one bar the one the bar reaches first from its open fills (both at the open: the exit before the entry), and after one fill the bar's other orders are dropped; (4) SELLSHORT LX3 / SE while long and BUY LE while short REVERSE the position (TradeStation strategy semantics: the exit and the new entry at the same price), and BUYTOCOVER SX2 only exits; (5) entry orders are placed only when the next bar is on the same date; exit orders are placed on every bar, so a position held at a day's last bar exits (or is reversed) on the next day's first bar; (6) Average(Range,20) needs 20 bars: no signal before the 20th bar of the series; a position still open at the end of the data is closed at the last close and marked open. Card-specific unstated choices (R4 #408): (1) the tie rule between two limit orders alive in one bar (the one the bar reaches first from its open fills; both at the open: the exit before the entry); (2) market orders are tried before limit orders; (3) SELLSHORT and BUY reverse the position, BUYTOCOVER only exits; (4) exit orders are placed on every bar, so a held position exits or reverses on the next day first bar; (5) 15-minute bars aligned to the 09:15 session open, the 15:30 auction minute dropped; (6) the nearest-expiry series is not back-adjusted, so the roll gap is carried; Honest total: 6 card-specific choices (audit count 6). Engine-wide defaults / declared adaptations (labelled, not counted, R4 #404): limit and market fill at the level or the open, the 20-bar warm-up, the open position closed at the last close, no costs. The book shows no result for these settings; a 15-minute S&P e-mini result and a NIFTY result are different series. No costs.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: ID, name: "Trend One (TradeStation code, 15-minute NIFTY / BANKNIFTY futures)", group: "Trend One (1999)", session: 0,
        dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
        rules: &["F|Inputs BuyRetPcnt 80, BuyThrust 1.3, SellRetPcnt 71, SellThrust 0.8 (Code.txt)",
            "F|Each new date: reset hh/ll; track the day's highest high (bar hb) and lowest low (bar lb); Norm = Average(Range, 20)",
            "E|If flat/short and next bar on the same date and hb > lb and hb < CurrentBar and hh-ll > BuyThrust*Norm*SquareRoot(hb-lb): BuyRetLevel = ll+(hh-ll)*80/100; if Low and Low[1] > BuyRetLevel, Buy next bar at BuyRetLevel limit",
            "E|Mirror for shorts: lb > hb, hh-ll > SellThrust*Norm*SquareRoot(lb-hb), SellRetLevel = hh-(hh-ll)*71/100, SellShort next bar at SellRetLevel limit when High and High[1] < it",
            "X|Long: Sell next bar at EntryPrice+(hh-ll)*0.5 limit; if BarsSinceEntry > 16 SellShort next bar at market (LX3, reverses)",
            "X|Short: BuyToCover next bar at EntryPrice-(hh-ll)*0.5 limit; if BarsSinceEntry > 16 BuyToCover next bar at market",
            NOTE],
        notes: "Brochure (TrendOne.pdf) shows only a 9-market portfolio equity chart (largest drawdown $48,147, average year $51,555) and no rules; that chart is not reproduced. Rules are Code.txt, whose intraday structure does not match the brochure's daily position-trading description; provenance uncertain, so the card is UNVERIFIED and runs on NIFTY 50 / NIFTYBANK futures only.",
        claim: None }]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { (id == ID).then_some(("aanto_trend_one", "Trend One", "")) }
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    (id == ID).then(|| vec![p("bar", "Bar size (minutes; 15 per Readme.txt)", 15., 5., 30., 5.), p("buy_ret", "BuyRetPcnt", 80., 10., 100., 1.), p("buy_thrust", "BuyThrust", 1.3, 0.1, 5., 0.1),
        p("sell_ret", "SellRetPcnt", 71., 10., 100., 1.), p("sell_thrust", "SellThrust", 0.8, 0.1, 5., 0.1)])
}
pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if id != ID || to >= x.c.len() || from > to { return None; }
    let g = |k: &str| *p.get(k).unwrap_or(&params(id).unwrap().iter().find(|d| d.key == k).unwrap().def);
    Some(trend_one(x, from, to, g("bar").round().max(1.0) as u16, g("buy_ret"), g("buy_thrust"), g("sell_ret"), g("sell_thrust")))
}

struct B { i: usize, o: f64, h: f64, l: f64, c: f64, last: bool }
#[derive(Clone, Copy, PartialEq)]
enum K { Le, Se, Lx2, Sx1, Lx3, Sx2 }

/// Code.txt bar by bar (see NOTE). Positions: (dir, entry price, entry bar, entry day row).
fn trend_one(x: &Ctx, from: usize, to: usize, n: u16, bret: f64, bthr: f64, sret: f64, sthr: f64) -> Vec<Trade> {
    let Some(f) = x.fut else { return vec![] };
    let mut b: Vec<B> = vec![];
    for i in from..=to {
        let r = &x.r[i];
        let Some((m, _)) = f.fut_day(&r.date) else { continue };
        let (open, close) = (crate::intraday::session_open(&r.ticker, &r.date), crate::intraday_ep::session_close(&r.ticker));
        let bars = crate::intraday_ep::nbars(&m, open, n, close);
        let len = bars.len();
        for (k, z) in bars.iter().enumerate() { b.push(B { i, o: z.o, h: z.h, l: z.l, c: z.c, last: k + 1 == len }); }
    }
    let mut out: Vec<Trade> = vec![];
    let mut pos: Option<(f64, f64, usize, usize)> = None;
    let mut orders: Vec<(K, f64)> = vec![];
    let (mut hh, mut ll, mut hb, mut lb) = (f64::NEG_INFINITY, f64::INFINITY, 0usize, 0usize);
    let close = |out: &mut Vec<Trade>, p: (f64, f64, usize, usize), xi: usize, px: f64, why: &str| out.push(crate::intraday::day_trade(p.0, p.3, p.1, xi, px, why, "limit", false));
    for k in 0..b.len() {
        let z = &b[k];
        // fills of the orders placed at the previous bar's close
        if !orders.is_empty() {
            let mut fill: Option<(K, f64)> = None;
            if let Some(&(kind, _)) = orders.iter().find(|(kd, _)| matches!(kd, K::Lx3 | K::Sx2)) { fill = Some((kind, z.o)); } else {
                let mut best: Option<(bool, f64, bool, K, f64)> = None; // (not at open, order key, is entry, kind, px)
                for &(kd, lv) in &orders {
                    let sell = matches!(kd, K::Se | K::Lx2);
                    let (hit, px) = if sell { (z.h >= lv, z.o.max(lv)) } else { (z.l <= lv, z.o.min(lv)) };
                    if !hit { continue; }
                    let key = (z.o != px, if sell { lv } else { -lv }, matches!(kd, K::Le | K::Se), kd, px);
                    if best.map_or(true, |bb| (key.0, key.1, key.2) < (bb.0, bb.1, bb.2)) { best = Some(key); }
                }
                if let Some((_, _, _, kd, px)) = best { fill = Some((kd, px)); }
            }
            if let Some((kd, px)) = fill {
                if let Some(p) = pos.take() { close(&mut out, p, z.i, px, match kd { K::Lx2 | K::Sx1 => "profit target (half the day's range)", K::Lx3 | K::Sx2 => "17 bars after entry at market", _ => "reversed by the opposite entry" }); }
                match kd {
                    K::Le => pos = Some((1.0, px, k, z.i)),
                    K::Se | K::Lx3 => pos = Some((-1.0, px, k, z.i)),
                    _ => {}
                }
            }
            orders.clear();
        }
        // day state and indicators at this bar's close
        if k == 0 || b[k - 1].i != z.i { hh = f64::NEG_INFINITY; ll = f64::INFINITY; }
        if z.h > hh { hh = z.h; hb = k; }
        if z.l < ll { ll = z.l; lb = k; }
        if k + 1 >= b.len() { break; }
        let same_day = b[k + 1].i == z.i && !z.last;
        let norm = if k >= 19 { b[k - 19..=k].iter().map(|v| v.h - v.l).sum::<f64>() / 20.0 } else { f64::NAN };
        let prev = if k > 0 { Some(&b[k - 1]) } else { None };
        let dir = pos.map_or(0.0, |p| p.0);
        if let Some((d, ep, ek, _)) = pos {
            if d > 0.0 { orders.push((K::Lx2, ep + (hh - ll) * 0.5)); if k - ek > 16 { orders.push((K::Lx3, 0.0)); } }
            else { orders.push((K::Sx1, ep - (hh - ll) * 0.5)); if k - ek > 16 { orders.push((K::Sx2, 0.0)); } }
        }
        if let (true, true, Some(pv)) = (same_day, norm.is_finite(), prev) {
            if dir <= 0.0 && hb > lb && hb < k && hh - ll > bthr * norm * ((hb - lb) as f64).sqrt() {
                let lv = ll + (hh - ll) * bret / 100.0;
                if z.l > lv && pv.l > lv { orders.push((K::Le, lv)); }
            }
            if dir >= 0.0 && lb > hb && lb < k && hh - ll > sthr * norm * ((lb - hb) as f64).sqrt() {
                let lv = hh - (hh - ll) * sret / 100.0;
                if z.h < lv && pv.h < lv { orders.push((K::Se, lv)); }
            }
        }
    }
    if let (Some(p), Some(z)) = (pos, b.last()) { out.push(crate::intraday::day_trade(p.0, p.3, p.1, z.i, z.c, "open", "limit", true)); }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn params_defaults_are_code_txt_inputs() {
        let d: Vec<f64> = params(ID).unwrap().iter().map(|p| p.def).collect();
        assert_eq!(d, vec![15., 80., 1.3, 71., 0.8]);
        assert!(params("x").is_none());
    }
}
