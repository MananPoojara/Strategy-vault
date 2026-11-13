//! Rick Martinelli, "Harnessing The (Mis)Behavior Of Markets", Technical Analysis of Stocks & Commodities V.24:6 (2006) pp.21-30
//! (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V24/C06/104MART.pdf).
//! Book id `scmart`: the alpha one-day predictor. Lane PA2 (13 Nov 2026); independent check authors/pa2/w/pa2_ref.py.
//! Admission: sclang.rs (PA2_BOOKS).
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::tselhs::Book;

pub const REVIEWED: &[&str] = &["scmart_alpha"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[No unstated detail; VERIFIED on the article's Figure 11 (R4 #168, see the gate row)] On each close n: P'(n+1) = Excel FORECAST of the next close from the last three closes P(n-2), P(n-1), P(n) (least-squares line through x = 1, 2, 3, read at x = 4); predicted change dP' = P'(n+1) - P(n); sigma = STDEVP (population standard deviation) of the last seven close-to-close changes; alpha = dP' / sigma. alpha > C -> one unit long from this close to the next close; alpha < -C -> one unit short; otherwise no wager. Every day is a separate wager ('after the next close, the position is canceled and a profit/loss taken. Then tomorrow's position is calculated and the procedure repeats'), so two long days in a row are two trades. Default C = 1: 'A buy signal occurs when the predicted price change exceeds the standard deviation of the most recent changes' (alpha > 1). The article also chose C per stock (1.00-4.00 in steps of 0.01) to maximise the last-day fortune of the same year it reports, a choice made after seeing the data; those C values are options here (Figure 11: MSFT 1.00, AMZN 1.00, QQQQ 1.10, DIA 1.35, DJI 1.40, AAPL 1.38, GM 1.04). No commissions. A day with sigma = 0 gets no wager. Runs on indices, ETFs and stocks, US and India, are a declared application (the article: 28 US stocks and indices, about one year each, 2004-2006).";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "scmart_alpha", name: "Martinelli alpha: 3-close forecast change / 7-day sigma beyond +-1 -> one-day long or short wager, close to close", group: "S&C: harnessing the (mis)behavior of markets (Martinelli, V.24:6)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "scmart", rules: &[
            "F|\"Use the built-in FORECAST function to calculate Pn+1', the estimate of Pn+1, based on the previous three prices: Pn, Pn-1, Pn-2 · Calculate dPn' = Pn+1' - Pn, the estimated price change for day n + 1 · Use the built-in function STDEVP to find sn, the standard deviation of the last seven price changes · Calculate an = dPn' / sn\" (V.24:6, PDF p.4)",
            "E|\"A buy signal occurs when the predicted price change exceeds the standard deviation of the most recent changes. A wager of one unit is placed and the resulting profit/loss taken at the close on the following day (there is no explicit sell signal).\" (PDF p.4)",
            "E|\"An = +1 if an > C, An = -1 if an < -C, An = Zero if -C <= an <= C\" (PDF p.5)",
            "X|\"On a buy signal, a position of one investing unit is taken, and after the next close, the position is canceled and a profit/loss taken. Then tomorrow's position is calculated and the procedure repeats.\" (PDF p.4)",
            "F|\"The Excel spreadsheet was programmed to test values of C between 1 and 4 in increments of 0.01. The value that maximized the fortune on the last day (the LDF) was chosen as optimal C\" (PDF p.5); Figure 11 (PDF p.6) lists C, wagers and win ratio per stock"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { reviewed(id).then_some(("scmart_alpha", "Martinelli alpha predictor (S&C)", "C")) }
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    reviewed(id).then(|| vec![
        ParamDef { key: "c", label: "Alpha cutoff C (1 = 'exceeds the standard deviation'; the article's per-stock optimum 1.00-2.42)", def: 1., min: 0.5, max: 4., step: 0.01 },
        ParamDef { key: "short", label: "1 = also take the short wagers (article); 0 = long wagers only", def: 1., min: 0., max: 1., step: 1. }])
}

/// alpha on each close (NaN until seven changes exist, or when sigma = 0).
pub fn alpha(c: &[f64]) -> Vec<f64> {
    (0..c.len()).map(|n| {
        if n < 7 { return f64::NAN; }
        let mean3 = (c[n - 2] + c[n - 1] + c[n]) / 3.;
        let fc = mean3 + (c[n] - c[n - 2]) / 2. * 2.; // FORECAST(4, y, {1,2,3}) = mean + slope x (4 - 2)
        let d: Vec<f64> = (n - 6..=n).map(|k| c[k] - c[k - 1]).collect();
        let m = d.iter().sum::<f64>() / 7.;
        let sd = (d.iter().map(|z| (z - m) * (z - m)).sum::<f64>() / 7.).sqrt();
        if sd > 0. { (fc - c[n]) / sd } else { f64::NAN }
    }).collect()
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from > to || to >= x.r.len() { return None; }
    let cut = p.get("c").copied().unwrap_or(1.);
    let shorts = p.get("short").copied().unwrap_or(1.) > 0.5;
    let a = alpha(&x.c);
    let mut pos = Book::new();
    for t in from..to {
        let d = if a[t] > cut { 1. } else if a[t] < -cut && shorts { -1. } else { 0. };
        if d != 0. { pos.enter(d, t, x.c[t], "close"); pos.exit(t + 1, x.c[t + 1], "next close"); }
    }
    Some(pos.done(to, x.c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn alpha_matches_the_spreadsheet_steps_and_wagers_last_one_day() {
        // three rising closes 10, 11, 12.5: FORECAST at x = 4 = 13.6667, change +1.1667
        let c = [10., 10.2, 9.9, 10.1, 10., 10., 11., 12.5];
        let d: Vec<f64> = (1..8).map(|k| c[k] - c[k - 1]).collect();
        let m = d.iter().sum::<f64>() / 7.;
        let sd = (d.iter().map(|z| (z - m).powi(2)).sum::<f64>() / 7.).sqrt();
        assert!((alpha(&c)[7] - (13.0 + 2. / 3. - 12.5) / sd).abs() < 1e-12);
        let r = super::super::sctraub::tests::bars(1500);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("scmart_alpha", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() > 20 && t.iter().any(|z| z.dir < 0.) && t.iter().all(|z| z.exit_i == z.legs[0].i + 1));
        assert!(run("scmart_alpha", &x, 0, r.len() - 1, &Params::from([("short".into(), 0.)])).unwrap().iter().all(|z| z.dir > 0.));
    }
}
