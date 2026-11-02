//! Anthony Trongone, "Moving Averages: Long On Talk, Short On Action", Technical Analysis of Stocks & Commodities V.25:5 (May
//! 2007) pp.28-33 (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V25/C05/
//! 090TRON.pdf). Book id `sctrtma`: the Trongone moving average (TMA) day trade. Lane K13 (titles loop, 2 Nov 2026), to-code row
//! 214, lead decision 79; independent check authors/titles-1030/K13w/k13_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::msws::sma;
use super::tselhs::Book;

pub const REVIEWED: &[&str] = &["sctrtma_qqq"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] At the open: the opening price above the 34-day SMA of the closes up to yesterday, yesterday's volume above 100,000,000 shares, and an overnight decline (today's open below yesterday's close) -> buy at the open; sell at the same day's close. The 100-million-share threshold is the article's own value (about the 60th percentile of QQQ volume then) and is kept as printed, never scaled (decision 79). The article's ETF is QQQ (QQQQ), which is held. Other instruments run only where their own volume exists, as declared applications; on most of them 100 million shares is rarely reached, so they rarely trade. Article (QQQQ, 31 October 2002 - 31 December 2006): 71% winners, $124 average per 1,000-share trade. Our QQQ run over the same span: 68 trades, 47 winners (69%).";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "sctrtma_qqq", name: "Trongone TMA (QQQ): after an overnight dip, buy the open above the 34-day SMA when yesterday's volume topped 100 million; sell at the close", group: "S&C: Moving Averages, Long On Talk, Short On Action (Trongone, V.25:5)", session: 0, dir: "Long", status: Status::Live,
        needs_volume: true, needs_vix: false, needs_series: &[], book: "sctrtma", rules: &[
            "E|\"OPENING PRICE > 34-DAY SMA + PREVIOUS DAY'S TRADING VOLUME > 100,000,000 + TODAY'S OVERNIGHT SESSION < $0.00\"; \"Today's overnight session is today's opening price - yesterday's close.\" (V.25:5 p.31)",
            "X|\"This system takes a 1,000-share long position in the cues at the opening of trading when the opening price is above the 34-day signal. The position is offset at the close of trading (16:00 ET).\" (p.29)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let id = REVIEWED.iter().find(|r| **r == id)?;
    Some((*id, "Trongone moving average day trade (S&C)", "QQQ, 34-day SMA, 100M volume"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    reviewed(id).then(|| vec![ParamDef { key: "vol", label: "Yesterday's volume above, million shares (100 printed)", def: 100., min: 0., max: 1000., step: 10. }])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let m = sma(&x.c, 34);
    let vol = p.get("vol").copied().unwrap_or(100.0) * 1e6;
    let mut pos = Book::new();
    for t in from + 1..=to {
        let v = x.r[t - 1].volume.filter(|v| *v > 0.0);
        if m[t - 1].is_finite() && v.is_some_and(|v| v > vol) && x.o[t] > m[t - 1] && x.o[t] < x.c[t - 1] {
            pos.enter(1.0, t, x.o[t], "open");
            pos.exit(t, x.c[t], "close");
        }
    }
    Some(pos.done(to, x.c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tma_day_trades() {
        let r = super::super::sctraub::tests::bars(3000);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("sctrtma_qqq", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(!t.is_empty() && t.iter().all(|z| z.exit_i == z.legs[0].i && z.legs[0].px == x.o[z.exit_i] && z.exit_px == x.c[z.exit_i]));
    }
}

// Instrument admission: S&C Trongone TMA 2026-11-02 (titles lane K13).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "sctrtma" && reviewed(def.id),
    label: "Anthony Trongone tested the TMA on QQQ (2002-2006), the default. Other indices, ETFs and stocks with their own volume, US and India, are a declared application; the 100-million-share threshold is kept as printed.",
    basis: "Technical Analysis of Stocks & Commodities V.25:5 pp.28-33 (Moving Averages: Long On Talk, Short On Action); independent check authors/titles-1030/K13w/k13_ref.py",
};
