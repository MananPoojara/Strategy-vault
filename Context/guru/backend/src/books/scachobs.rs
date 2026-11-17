//! Steven B. Achelis, "Forecasting the market with the overbought/oversold indicator", Technical Analysis of Stocks & Commodities
//! V.5:7 (Jul 1987) pp.231-232 (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V05/C07/FORCAST.pdf).
//! Book id `scachobs`. Lane C5 (second pass, 17 Nov 2026); independent check authors/convert-1117/C5/ref/c5_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::rbtie::target_trades;

pub const REVIEWED: &[&str] = &["scachobs_200"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }
pub const SERIES: &[&str] = &["NYSE_ADV_DEC.Advances", "NYSE_ADV_DEC.Declines"];

const NOTE: &str = "[Adapted: 1 unstated detail, R4 #297] OB/OS = the 10-day exponential average of NYSE advancing minus declining issues, as printed: OB/OS = 0.18 x today's (advances - declines) + 0.82 x yesterday's OB/OS, first value = the first day's difference; the article says to calculate it 'a minimum of 10 days before the moving average comes up to speed', so no signal is taken in the first 10 days of data. Buy when OB/OS fell below and then rose above -200 (it crosses -200 upward); sell when it rose above and then fell below +200 (it crosses +200 downward); the article's stop-loss: 'Short positions were closed anytime the OB/OS indicator rose above the +200 and long positions were closed anytime the OB/OS indicator fell below -200', after which the card is flat until the next buy or sell. Signals act at the close of the day the line crosses. [Unstated detail] The article draws 'buy' and 'sell' arrows and measures both sides on the Dow and the OEX; a sell signal while long is read as selling the long and going short (a buy while short likewise), not as going flat; this reading is labelled here and is why the card is UNVERIFIED. [Book result] The article's chart year (1986, DJIA) 'would have yielded a gain of 459 Dow points during a period in which the Dow gained only 130 points'; OEX study: 2.352 points per trade, 39.99 cumulative; shown on the card as the book's own figures, not reproduced exactly (no trade table is printed). NYSE advances and declines are the vault's NYSE_ADV_DEC series (1965-03-01..2020-02-10), so the card runs on the US indices and ETFs only.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "scachobs_200", name: "Achelis OB/OS: 10-day average of NYSE advances minus declines crosses back through -200 (buy) or +200 (sell), with the 200-line stop", group: "S&C: Overbought/oversold indicator (Achelis, V.5:7)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: SERIES, book: "scachobs", rules: &[
            "F|\"The OB/OS indicator is defined as the 10-day exponential moving average of advancing minus declining issues ... multiplying the value in column 4 by 0.18 and adding the result to the value of the previous day's column 5 multiplied by 0.82.\" (V.5:7 p.231)",
            "E|\"'Buy' arrows pointing up were drawn on the chart each time the OB/OS indicator fell below and then rose above the oversold level of -200. Similarly, 'sell' arrows pointing down were drawn when the OB/OS indicator rose above and then fell below the overbought level of +200.\" (p.231-232)",
            "X|\"Short positions were closed anytime the OB/OS indicator rose above the +200 and long positions were closed anytime the OB/OS indicator fell below -200.\" (p.232)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    reviewed(id).then_some(("scachobs_200", "Achelis OB/OS (S&C)", "+/-200 lines"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("level", "Overbought/oversold level (article +/-200)", 200., 50., 400., 25.)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from > to || to >= x.c.len() { return None; }
    let lv = p.get("level").copied().unwrap_or(200.);
    let s: Vec<&[f64]> = match SERIES.iter().map(|n| x.series(n)).collect::<Option<Vec<_>>>() { Some(v) => v, None => return Some(vec![]) };
    let mut ob = vec![f64::NAN; to + 1];
    let (mut prev, mut first) = (f64::NAN, usize::MAX);
    for i in 0..=to {
        let d = s[0][i] - s[1][i];
        if !d.is_finite() { continue; }
        if first == usize::MAX { first = i; }
        prev = if prev.is_finite() { 0.18 * d + 0.82 * prev } else { d };
        ob[i] = prev;
    }
    let mut tgt = vec![0i8; to + 1];
    let mut pos = 0i8;
    for i in 1..=to {
        if first == usize::MAX || i < first + 10 || !ob[i].is_finite() || !ob[i - 1].is_finite() { tgt[i] = pos; continue; }
        let (a, b) = (ob[i - 1], ob[i]);
        if a < -lv && b > -lv { pos = 1; }
        else if a > lv && b < lv { pos = -1; }
        else if pos == 1 && b < -lv { pos = 0; }
        else if pos == -1 && b > lv { pos = 0; }
        tgt[i] = pos;
    }
    Some(target_trades(x, &tgt, from, to, false, "OB/OS signal"))
}

pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "scachobs" && reviewed(def.id),
    label: "Steven Achelis showed the OB/OS lines on the Dow (1986, chart arrows) and an OEX study; it reads NYSE advancing and declining issues and runs on the US indices and ETFs SPX, SPY, DJI, DIA, NDX and QQQ only (NYSE breadth, R4 #306), with the unstated detail read as labelled on the card.",
    basis: "Technical Analysis of Stocks & Commodities V.5:7 pp.231-232 (Forecasting the market with the overbought/oversold indicator); independent check authors/convert-1117/C5/ref/c5_ref.py",
};

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn obos_has_catalogue_and_params() {
        assert!(family_of("scachobs_200").is_some() && params("scachobs_200").is_some());
        let r = super::super::sctraub::tests::bars(300);
        let x = Ctx::new(&r, &r, &[]);
        assert_eq!(run("scachobs_200", &x, 0, r.len() - 1, &Params::new()).unwrap().len(), 0); // no NYSE series: no trades, no panic
    }
}
