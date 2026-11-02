//! Gomu Vetrivel, "Improve Your Trading System With Speed", Technical Analysis of Stocks & Commodities V.23:11 (November 2005)
//! p.28 and the EasyLanguage sidebar (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008)
//! (traders.com)/VOLUMES/V23/C11/225VETR.pdf). Book id `scvet`: the base 10/5 SMA system and its two speed hypotheses. Lane K13
//! (titles loop, 2 Nov 2026), to-code row 218; independent check authors/titles-1030/K13w/k13_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::msws::sma;
use super::tselhs::Book;

pub const REVIEWED: &[&str] = &["scvet_base", "scvet_h1", "scvet_h2"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] The printed EasyLanguage, all orders 'at c' (this bar's close): speed x = |close - close[1]|; buy when the close crosses above its 10-bar average (c > average(c,10) and c[1] < average(c,10)[1]), sell short on the cross below; exit long when c < average(c,5), exit short when c > average(c,5). An opposite entry reverses; an entry on the side already held is ignored and that bar's exit still applies (TradeStation order handling). Hypothesis 1 takes the entries only when speed > speed[1], hypothesis 2 only when speed < speed[1]; exits unchanged. Article (TradeStation, 1 January 2001 - 31 May 2005, no costs): SPX base 182 trades, 36.8% profitable, profit factor 1.23; Russell 2000 base 194 trades (39.7%), hypothesis 1 143 trades (42.0%). Our SPX run over the same span: base 186 trades, 36.0% winners. Runs on other indices, ETFs and stocks, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, e| StrategyDef { id, name, group: "S&C: Improve Your Trading System With Speed (Vetrivel, V.23:11)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "scvet", rules: e, notes: NOTE, claim: None };
    vec![
        d("scvet_base", "Vetrivel base system: close crosses its 10-day average to enter (long or short), out on the 5-day average", &[
            "E|\"if c>average(c,10) and c[1]<average(c,10)[1] then buy at c; if c<average(c,10) and c[1]>average(c,10)[1] then sell at c;\" (sidebar, Base System)",
            "X|\"if marketposition=-1 and c>average(c,5) then exitshort at c; if marketposition=1 and c<average(c,5) then exitlong at c;\" (sidebar)"]),
        d("scvet_h1", "Vetrivel hypothesis 1: the 10-day cross entries only when speed increases; out on the 5-day average", &[
            "E|\"x=absvalue(c-c[1]); if x>x[1] and c>average(c,10) and c[1]<average(c,10)[1] then buy at c; if x>x[1] and c<average(c,10) and c[1]>average(c,10)[1] then sell at c;\" (sidebar, Hypothesis 1)",
            "X|exits as the base system (sidebar)"]),
        d("scvet_h2", "Vetrivel hypothesis 2: the 10-day cross entries only when speed decreases; out on the 5-day average", &[
            "E|\"if x<x[1] and c>average(c,10) and c[1]<average(c,10)[1] then buy at c; if x<x[1] and c<average(c,10) and c[1]>average(c,10)[1] then sell at c;\" (sidebar, Hypothesis 2)",
            "X|exits as the base system (sidebar)"]),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let v = match id { "scvet_base" => "Base system", "scvet_h1" => "Hypothesis 1 (speed up)", "scvet_h2" => "Hypothesis 2 (speed down)", _ => return None };
    Some(("scvet", "Vetrivel speed systems (S&C)", v))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> { reviewed(id).then(Vec::new) }

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, _p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let c = &x.c;
    let (s10, s5) = (sma(c, 10), sma(c, 5));
    let mut pos = Book::new();
    for i in from.max(11)..=to {
        let (x0, x1) = ((c[i] - c[i - 1]).abs(), (c[i - 1] - c[i - 2]).abs());
        let f = match id { "scvet_h1" => x0 > x1, "scvet_h2" => x0 < x1, _ => true };
        let up = c[i] > s10[i] && c[i - 1] < s10[i - 1] && f;
        let dn = c[i] < s10[i] && c[i - 1] > s10[i - 1] && f;
        let p0 = pos.side();
        if p0 != 1.0 && up { pos.enter(1.0, i, c[i], "close"); }
        else if p0 != -1.0 && dn { pos.enter(-1.0, i, c[i], "close"); }
        else if p0 > 0.0 && c[i] < s5[i] { pos.exit(i, c[i], "under the 5-day average"); }
        else if p0 < 0.0 && c[i] > s5[i] { pos.exit(i, c[i], "over the 5-day average"); }
    }
    Some(pos.done(to, c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn speed_systems() {
        let r = super::super::sctraub::tests::bars(3000);
        let x = Ctx::new(&r, &r, &[]);
        let n: Vec<usize> = REVIEWED.iter().map(|id| run(id, &x, 0, r.len() - 1, &Params::new()).unwrap().len()).collect();
        assert!(n[0] > 10 && n[1] <= n[0] && n[2] <= n[0], "{n:?}");
    }
}

// Instrument admission: S&C Vetrivel speed 2026-11-02 (titles lane K13).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "scvet" && reviewed(def.id),
    label: "Gomu Vetrivel tested the three systems on SPX and the Russell 2000 (2001-2005). Runs on other indices, ETFs and stocks, US and India, are a declared application.",
    basis: "Technical Analysis of Stocks & Commodities V.23:11 p.28 and sidebar (Improve Your Trading System With Speed); independent check authors/titles-1030/K13w/k13_ref.py",
};
