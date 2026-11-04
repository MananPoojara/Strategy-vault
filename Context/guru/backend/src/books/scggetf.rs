//! Gerald Gardner and Trent Gardner, "Profit With ETFs", Technical Analysis of Stocks & Commodities V.26:6 (2008) pp.20-24, with the
//! sidebar "Wealth-Lab code for timing model trading systems" (research share Trading Knowledge I/Stocks & Commodities DVD 11.26
//! (1982 - 2008) (traders.com)/VOLUMES/V26/C06/102GARD.pdf). Book id `scggetf`. Lane K41 (batch T7a, 4 Nov 2026); independent check
//! authors/titles-1030/K41w/k41_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::msws::{rsi, sma};
use super::tselhs::Book;

pub const REVIEWED: &[&str] = &["scggetf_rsi2h"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] The printed Wealth-Lab script, one instrument at a time: RSI(2) of the HIGHS (Wilder smoothing, seeded with the mean of the first 2 changes); after a close with RSI < 2, the previous RSI > 1 and the close above its 200-day SMA, BuyAtLimit(Bar + 1, 1.025 x that bar's low): filled the next day at the limit, or at the open if it opens below it. While long, after each close: if Bar + 1 - entry bar >= 6, sell at the next open ('Time-based'); else if the close is above the high of two bars ago, sell at the next open ('H2'); else a limit sell for the next day at 1.075 x the high of two bars ago (at the open if it opens above). Orders are placed after the bar that filled the entry; no entry order while a position is open; the script starts 200 bars into the data. The article traded a portfolio of ETFs, at most three at a time, bought alphabetically; the single-instrument runs here do not reproduce that portfolio. Article (five years, 2/2003-10/2007, $50,000): 296 trades, 218 winners, +90.0%, 26.4% exposure. Runs on indices, ETFs and stocks, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "scggetf_rsi2h", name: "RSI(2) of the highs dips below 2 above the 200-day SMA: limit buy 2.5% over the low; exit above the high of two days ago, +7.5% or after 6 days", group: "S&C: profit with ETFs (Gardner, V.26:6)", session: 0, dir: "Long", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "scggetf", rules: &[
            "E|\"if PriceClose( Bar ) > SMA( Bar, #Close, 200 ) then if (RSI(Bar - 1, #High, 2) > 1) and (RSI(Bar, #High, 2) < 2) then BuyAtLimit(Bar + 1, 1.025 * PriceLow(Bar), '');\" (V.26:6, PDF p.2, sidebar code)",
            "X|\"H2 := PriceHigh(Bar - 2); if Bar + 1 - PositionEntryBar( p ) >= 6 then SellAtMarket(Bar + 1, p, 'Time-based') else if PriceClose(Bar) > H2 then SellAtMarket(Bar + 1, p, 'H2') else SellAtLimit(Bar + 1, 1.075 * H2, p, 'Limit Target');\" (PDF p.2, sidebar code)",
            "F|\"We used the highs of the ETF price series to calculate the RSI values\"; \"Only three ETFs are held at any one time ... the ETFs are purchased in alphabetical order\" (PDF p.2-3)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { reviewed(id).then_some(("scggetf_rsi2h", "ETF timing model (Gardner, S&C)", "RSI(2) of highs")) }
pub fn params(id: &str) -> Option<Vec<ParamDef>> { reviewed(id).then(Vec::new) }

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, _p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let (r, m) = (rsi(&x.h, 2), sma(&x.c, 200));
    let mut pos = Book::new();
    // order for the next bar: (true = buy limit, false = sell) with level (NaN = at market)
    let mut ord: Option<(bool, f64)> = None;
    for t in from..=to {
        if let Some((buy, lv)) = ord.take() {
            if buy { if x.l[t] <= lv { pos.enter(1.0, t, x.o[t].min(lv), "limit"); } }
            else if lv.is_nan() { pos.exit(t, x.o[t], "Time-based or H2 · next open"); }
            else if x.h[t] >= lv { pos.exit(t, x.o[t].max(lv), "Limit target"); }
        }
        if t >= to || t < 200 || t < 2 { continue; }
        if let Some(e) = pos.entry_bar() {
            let h2 = x.h[t - 2];
            ord = Some((false, if t + 1 - e >= 6 || x.c[t] > h2 { f64::NAN } else { 1.075 * h2 }));
        } else if x.c[t] > m[t] && r[t - 1] > 1.0 && r[t] < 2.0 {
            ord = Some((true, 1.025 * x.l[t]));
        }
    }
    Some(pos.done(to, x.c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rsi_of_highs_dips() {
        let mut r = super::super::sctraub::tests::bars(2500);
        for (i, b) in r.iter_mut().enumerate() { if i % 37 == 0 { b.high -= 3.0; } } // force occasional RSI(2) dips
        let x = Ctx::new(&r, &r, &[]);
        let t = run("scggetf_rsi2h", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.iter().all(|z| z.dir > 0.0 && (z.open || z.exit_i - z.legs[0].i <= 6)));
    }
}
