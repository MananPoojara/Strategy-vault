//! Markos Katsanos, "Penny Stock Breakout System", Technical Analysis of Stocks & Commodities V.23:1 (2005) pp.18-29, with the
//! sidebar "MetaStock code and Penny Stock Breakout System" (research share Trading Knowledge I/Stocks & Commodities DVD 11.26
//! (1982 - 2008) (traders.com)/VOLUMES/V23/C01/004KATS.pdf and 004SB.pdf). Book id `scmkpsb`. Lane K41 (batch T7a, 4 Nov 2026);
//! independent check authors/titles-1030/K41w/k41_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::msws::Eng;
use super::tselhs::Book;

pub const REVIEWED: &[&str] = &["scmkpsb_breakout"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

/// The sidebar's buy order, as printed (Typical() = (H+L+C)/3, MetaStock's typical price).
pub const BUY: &str = "MACDH:=MACD()-Mov(MACD(),9,E); SDC:=Stdev(C ,30)/Mov(C,30,S); PERIOD:=14; COEF:=.1; INTRA:=Log(H)-Log(L); VINTRA:=Stdev(INTRA,PERIOD); INTER:=Log(Typical())-Log(Ref(Typical(),-1)); VINTER:=Stdev(INTER,PERIOD); CUTOFF:=COEF*(VINTER+VINTRA)*C; MF:=C-(H+L)/2+Typical()-Ref(Typical(),-1); FVE:=Sum(If(MF>CUTOFF, +V,If(MF<-CUTOFF,-V,0)),PERIOD)/Mov(V,PERIOD,S)/PERIOD*100; FVE>-1 AND LinRegSlope(C,35)/Ref(C,-35)*100>-.35 AND LinRegSlope(C,35)/Ref(C,-35)*100<.4 AND LinRegSlope(C,70)/Ref(C,-70)*100>-.4 AND LinRegSlope(C,70)/Ref(C,-70)*100<.4 AND LinRegSlope(C,170)/Ref(C,-170)*100>-.2 AND MACDH>-.003 AND (Sum(SDC-LLV(SDC,150),3)/Sum(HHV(SDC,150)-LLV(SDC,150),3))*100<20 AND C>Mov(C,10,E) AND SDC*ADX(25)<1.3 AND Stoch(10,3)>30";

const NOTE: &str = "[Declared readings] The sidebar's MetaStock buy order is evaluated as printed by the vault's MetaStock formula engine (Typical() = (H+L+C)/3; MetaStock MACD, Stdev, LinRegSlope, ADX and Stoch definitions). 'select buy price at open and sell price at close. Then fill in one day for the delay': a buy signal at a close buys at the next open; a sell signal at a close sells at the next close. Sell signals: the printed 'Trailing stop condition: C<(1-18/100)*ref(C,-1)', or the time exit 'Simulation.CurrentPositionAge>70' (the entry bar counts as 1). Long only, one position; no buy signal is taken while a position is open or on the bar it is sold. MACDH > -.003 is the printed absolute threshold (written for low-priced Nasdaq stocks; on higher-priced instruments it acts as MACDH > about 0). The article's tests (low-priced Nasdaq stocks such as SIRI, 10 points commission, 3% interest) are tables of single stocks. Not the same as mkfve_div (the 2003 FVE divergence system). Runs on indices, ETFs and stocks with volume, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "scmkpsb_breakout", name: "Penny stock breakout: FVE, regression-slope, MACD, volatility and stochastic filters; exit on an 18% one-day drop or after 70 bars", group: "S&C: penny stock breakout (Katsanos, V.23:1)", session: 0, dir: "Long", status: Status::Live,
        needs_volume: true, needs_vix: false, needs_series: &[], book: "scmkpsb", rules: &[
            "F|\"FVE:=Sum(If(MF>CUTOFF, +V,If(MF<-CUTOFF,-V,0)),PERIOD)/Mov(V,PERIOD,S)/PERIOD*100\" with PERIOD:=14; COEF:=.1; INTRA, VINTRA, INTER, VINTER, CUTOFF, MF as printed (V.23:1, 004SB.pdf p.1)",
            "E|\"{Buy Conditions} FVE>-1 AND LinRegSlope(C,35)/Ref(C,-35)*100>-.35 AND LinRegSlope(C,35)/Ref(C,-35)*100<.4 AND LinRegSlope(C,70)/Ref(C,-70)*100>-.4 AND LinRegSlope(C,70)/Ref(C,-70)*100<.4 AND LinRegSlope(C,170)/Ref(C,-170)*100>-.2 AND MACDH>-.003 AND (Sum(SDC-LLV(SDC,150),3)/Sum(HHV(SDC,150) -LLV(SDC,150),3))*100<20 AND C>Mov(C,10,E) AND SDC*ADX(25)<1.3 AND Stoch(10,3)>30\" (004SB.pdf p.1)",
            "X|\"Sell order ... Trailing stop condition: C<(1-18/100)*ref(C,-1)\"; \"Simulation.CurrentPositionAge>70\"; \"select buy price at open and sell price at close. Then fill in one day for the delay.\" (004SB.pdf p.1)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { reviewed(id).then_some(("scmkpsb_breakout", "Penny stock breakout (Katsanos, S&C)", "MetaStock test")) }
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    reviewed(id).then(|| vec![
        ParamDef { key: "drop", label: "Sell after a one-day drop of more than this % (18 as printed)", def: 18., min: 5., max: 50., step: 1. },
        ParamDef { key: "age", label: "Sell when the position is older than this many bars (70 as printed)", def: 70., min: 5., max: 250., step: 1. }])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let (drop, age) = (p.get("drop").copied().unwrap_or(18.) / 100.0, p.get("age").copied().unwrap_or(70.).round() as usize);
    let src = BUY.replace("Typical()", "((H+L+C)/3)");
    let none = |_: &str| -> Option<String> { None };
    let fl: &dyn Fn(&str) -> Option<String> = &none;
    let buy = Eng::new(x, fl).eval_src(&src).ok()?;
    let mut pos = Book::new();
    let (mut pend_buy, mut pend_sell) = (false, false);
    for t in from..=to {
        if pend_buy { pos.enter(1.0, t, x.o[t], "open"); pend_buy = false; }
        if pend_sell { pos.exit(t, x.c[t], "Sell order · next close"); pend_sell = false; continue; }
        if t >= to { break; }
        if let Some(e) = pos.entry_bar() {
            if x.c[t] < (1.0 - drop) * x.c[t - 1] || t + 1 - e > age { pend_sell = true; }
        } else if buy[t].is_finite() && buy[t] != 0.0 { pend_buy = true; }
    }
    Some(pos.done(to, x.c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn buy_formula_evaluates() {
        let r = super::super::sctraub::tests::bars(1500);
        let x = Ctx::new(&r, &r, &[]);
        let none = |_: &str| -> Option<String> { None };
        let fl: &dyn Fn(&str) -> Option<String> = &none;
        let v = Eng::new(&x, fl).eval_src(&BUY.replace("Typical()", "((H+L+C)/3)")).unwrap();
        assert_eq!(v.len(), r.len());
        let t = run("scmkpsb_breakout", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.iter().all(|z| z.dir > 0.0 && (z.open || z.exit_i - z.legs[0].i <= 71)));
    }
}
