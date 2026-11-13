//! M.H. Pee, "Trend Detection Index" ("Are You In A Trend?"), Technical Analysis of Stocks & Commodities V.19:10 (2001) p.54-61,
//! with the Excel sidebar (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/
//! V19/C10/112TREN.pdf). Book id `mhptdi`, author M.H. Pee. Lane PB (13 Nov 2026, pool B row 426 of authors/a1/A1.tsv; R4 #297);
//! independent check authors/pb/w/pb_ref3.py. Runs on the Norgate continuous contracts of the article's Figure 2 basket.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::rbtie::target_trades;

pub const REVIEWED: &[&str] = &["mhptdi_tdi"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Adapted: 2 unstated details, R4 #297; unverified] The sidebar's spreadsheet, one row per day: momentum = close - close 20 days earlier; direction indicator DI = sum of the last 20 momenta; TDI = |DI| + (sum of the last 20 absolute momenta) - (sum of the last 40 absolute momenta). Position: when TDI > 0, long if DI > 0, otherwise short; when TDI <= 0 the position is kept (=IF(I60>0,IF(E60>0,1,-1),J60)); always in after the first signal. Unstated: (1) the fill; the sidebar's instruction column says 'GO LONG/SHORT AT OPEN TOMORROW' while its price column shows the signal day's close; vault reading: the next open (option: the signal close); (2) the article says protective and trailing stops 'must be' used but gives no size; vault reading: none (Figure 2's spreadsheet results carry no stop rule). Figure 2 (15 futures, 16 years, period not printed, results in dollars per contract): the vault runs 13 of them: not the Deutschemark (not held) and not crude oil (a TDI trade enters near the negative 2020-04-20 settlement, which leaves no % base, so the engine refuses the run) on Norgate continuous contracts, back-adjusted; momentum differences are roll-free. Norgate continuous futures, ends 2020-09-11; roll method not stated by the vendor.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "mhptdi_tdi", name: "Pee trend detection index: trade the 20-day momentum direction while the TDI is positive (always in)", group: "S&C: Trend Detection Index (Pee, V.19:10)",
        session: 0, dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "mhptdi", rules: &[
            "F|\"20-day TDI = (AV20) - {(SumAM40) - (SumAM20)}\": AV20 = absolute value of the sum of the last 20 20-day momenta; SumAM40 / SumAM20 = sums of the 20-day absolute momenta of the last 40 / 20 days (p.55; sidebar =(F60 + H60) - G60)",
            "E|\"Go long at the open if both the TDI and direction indicator are positive\"; \"go short at the open if the TDI is positive and the direction indicator is negative\" (p.55); sidebar =IF(I60>0,IF(E60>0,1,-1),J60)",
            "X|Reverse on the opposite signal (the position column keeps its value until the TDI is positive again with the other direction)"],
        notes: NOTE, claim: crate::connors::claim("39-51 per market", "34.8%-51.3%", "$134.84 (oats) to $2,326.16 (yen) per contract", "n/a", "15 futures, 16 years (Figure 2)") }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    reviewed(id).then_some(("mhptdi_tdi", "Pee trend detection index", "20-day"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("n", "Momentum / sum length in days (20)", 20., 5., 60., 1.), p("close", "Fills: 0 = next open (default), 1 = signal day's close", 0., 0., 1., 1.)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from > to || to >= x.c.len() { return None; }
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap().def);
    let n = g("n").round().max(2.) as usize;
    let c = &x.c;
    let mom: Vec<f64> = (0..=to).map(|i| if i >= n { c[i] - c[i - n] } else { f64::NAN }).collect();
    let sum = |i: usize, m: usize, f: &dyn Fn(f64) -> f64| if i + 1 >= m && i + 1 - m >= n { (i + 1 - m..=i).map(|k| f(mom[k])).sum::<f64>() } else { f64::NAN };
    let mut tgt = vec![0i8; to + 1];
    let mut pos = 0i8;
    for i in 0..=to {
        let di = sum(i, n, &|v| v);
        let tdi = di.abs() + sum(i, n, &|v| v.abs()) - sum(i, 2 * n, &|v| v.abs());
        if tdi > 0. { pos = if di > 0. { 1 } else { -1 }; }
        tgt[i] = pos;
    }
    Some(target_trades(x, &tgt, from, to, g("close") < 0.5, "TDI signal"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn tdi_goes_with_the_direction_while_positive() {
        // flat 45 days, then a steady rise: with n = 5 the TDI turns positive once the rise beats the older noise
        let px: Vec<f64> = (0..60).map(|i| if i < 45 { 100. + if i % 2 == 0 { 1. } else { -1. } } else { 100. + 2. * (i - 44) as f64 }).collect();
        let r: Vec<Row> = px.iter().enumerate().map(|(i, &c)| Row { ticker: "Gold (Norgate &GC)".into(), date: format!("2001-{:02}-{:02}", 1 + i / 28, 1 + i % 28), open: c - 0.5, high: c + 1., low: c - 1., close: c, volume: None }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let p = Params::from([("n".to_string(), 5.)]);
        let t = run("mhptdi_tdi", &x, 0, 59, &p).unwrap();
        let last = t.last().unwrap();
        assert!(last.dir > 0. && last.open && last.entry_kind == "open" && last.legs[0].i > 45 && last.legs[0].px == x.o[last.legs[0].i]);
        assert!(run("mhptdi_x", &x, 0, 5, &p).is_none());
    }
}

// Admission: a futures card (futures::CARDS_PB, policy requirement "futures" via futures::card_basis), like lane F3's cards.
