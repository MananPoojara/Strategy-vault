//! Steve Primo / Connors Research Group, "The Raptor-10 Momentum Methodology" course (method by David Weilmuenster,
//! Aug/Sep 2005): intro.pdf (rules primer), RaptorPresentation.pdf (37 slides), AmiBroker RaptorExplore.afl, and the
//! research workbooks with the full test AFL (Raptor Update Aug-05.xls, sheet AFL). Book id "prap1".
//! Library notes: library/prap1/. A long-only stock method tested on all US stocks 1/1995-8/2005 (portfolio results only).
#![allow(dead_code)]
use crate::connors::*;

pub const BOOK: &str = "prap1";

const RULES: &[&str] = &[
    "F|\"A. Stock must be above it's 200-day SMA.\" (deck 'RAPTOR-10 METHOD RULES'; intro Rule #2)",
    "F|\"B. The stock's 100-day HV must be at least 2 x greater than the 100-day HV of the S&P 500 Cash Index.\" (AFL: Hv100 >= Hvmult*SP500HV, HV = StDev(log(C/Ref(C,-1)),100)*(252^.5)*100, AmiBroker population StDev)",
    "F|\"C. Stock's 10-period ADX must be 35 or greater.\" (AFL: ADX(10) > 35)",
    "E|\"D. Yesterday's close must be the fifth consecutive day that the stock closed below it's 2-period SMA.\" (AFL: BarsSince(C>=MA(C,2))==5, exactly 5; RSI2 < 75 also required by the AFL)",
    "F|\"The 20-day ma of the Close*Volume of stock greater than the (65-day ma of the Close*Volume of SP500) times .0003.\" (deck FILTER, intro Rule #6; the AFL's 3/1000 depends on its vendor's volume units)",
    "E|\"E. Enter on the stock's next day opening.\"",
    "A|\"(A second entry occurs if the stock closes under the 2-period SMA the next day.)\" - intro Rule #8: \"the next day only\"; added at the following open (AFL Ref(Buy,-1) AND C<MA(C,2) -> sigScaleIn)",
    "X|\"F. Exit full position when the 2-period RSI closes above 75.\" - intro Rule #10: \"on the morning following the day the stock's 2-period RSI closes above 75\"",
];

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    let d = |id, name, status, needs_series, rules, notes, claim| StrategyDef {
        id, name, group: "Raptor-10 Momentum Method (2005) · stocks", session: 0, dir: "Long", status, needs_volume: true,
        needs_vix: false, needs_series, book: BOOK, rules, notes, claim,
    };
    vec![
        d("prap1_raptor10", "Raptor-10 (5 closes under SMA2, ADX > 35, HV >= 2x S&P 500)",
          NeedsData("the 100-day HV of the S&P 500 cash index aligned to each stock (NIFTY 50 for NSE stocks), series BENCH_HV100: the engine passes the benchmark only into the Liquidity Filter, and no extra_data file holds it"),
          &["BENCH_HV100"], RULES,
          concat!("Coded against BENCH_HV100 = HV(100) of the benchmark index on the same date (population st.dev of 100 ln returns x sqrt(252) x 100, as the AFL). ",
                  "Liquidity Filter = 1,000,000 x MA20(close x volume) / MA65(benchmark close x volume), so the book's x .0003 is 'greater than 300'. ",
                  "Entry/add/exit all at the next open (AFL SetTradeDelays(1), BuyPrice = O). Two units max (0.5 + 0.5). ",
                  "Book results: portfolio of all US stocks; not checkable (no US stock data)."),
          claim("2,047 (Raptor-10 portfolio)", "78.02%", "3.72%", "6.75 bars", "1/1995-8/2005 US stocks, 10 slots: CAR 111.06%, max system DD -35.3% (Raptor Update Aug-05.xls); deck: 'averaged yearly gains of 111.1%', 85% of months profitable")),
        d("prap1_portfolio", "Raptor-10 / -20 / -50 portfolios", NeedsData("all US stocks 1995-2005 on one equity curve with highest-HV(100) tie-break; the engine backtests one ticker per run and holds no US stocks"), &[],
          &["F|\"Divide your account into 10 slots ... If a second entry trade occurs, then those additional shares occupy 1 slot ... the tie-breaker will be to trade the stocks with the highest Historical Volatility.\" (deck; Raptor-20: 20 slots; Raptor-50 named in intro)",
            "F|Research variants (Raptor Lap1.xls: double size on a lap-down open; Raptor Stop Loss.xls: 5-20% stops) are portfolio studies"],
          "Portfolio layer of prap1_raptor10.", None),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let f = |v| Some(("prap1", "Raptor-10 Momentum Method", v));
    match id {
        "prap1_raptor10" => f("Single stock · needs S&P 500 HV"),
        "prap1_portfolio" => f("Portfolio · needs a universe"),
        _ => None,
    }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    if id != "prap1_raptor10" {
        return None;
    }
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(vec![
        p("trend", "Close above N-day SMA", 200., 100., 300., 50.),
        p("hv_mult", "HV(100) at least x times the benchmark's HV(100)", 2., 0., 4., 0.5),
        p("adx", "ADX(10) above", 35., 0., 50., 5.),
        p("days", "Exactly N consecutive closes below the 2-day SMA", 5., 2., 8., 1.),
        p("lf", "Liquidity Filter above (book x .0003 = 300)", 300., 0., 3000., 100.),
        p("add", "Second entry if the next day also closes below the 2-day SMA (1 = book, 0 = off)", 1., 0., 1., 1.),
        p("exit_rsi", "Exit next open after RSI(2) closes above", 75., 50., 95., 5.),
    ])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if id != "prap1_raptor10" {
        return None;
    }
    let bhv = x.series("BENCH_HV100")?;
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let (c, n) = (&x.c, x.c.len());
    let to = to.min(n - 1);
    let trend = x.sma_n(g("trend") as usize);
    let ma2 = sma(c, 2);
    let hv = hv_ddof(c, 100, 0.);
    let r = &x.rsi2;
    let under = streak(n, |i| c[i] < ma2[i]); // BarsSince(C >= MA(C,2)) = consecutive closes below
    let (hm, adx, days, lf, ex) = (g("hv_mult"), g("adx"), g("days") as usize, g("lf"), g("exit_rsi"));
    let setup = |i: usize| {
        c[i] > trend[i] && r[i] < ex && x.adx10[i] > adx && under[i] == days && x.lf[i] > lf && bhv[i].is_finite() && hv[i] >= hm * bhv[i]
    };
    let (mut out, mut cur): (Vec<Trade>, Option<Trade>) = (vec![], None);
    let (mut buy, mut add, mut exit, mut sig) = (false, false, false, 0usize);
    for i in from..=to {
        if exit {
            let mut t = cur.take().unwrap();
            (t.exit_i, t.exit_px) = (i, x.o[i]);
            out.push(t);
            exit = false;
        }
        if add {
            cur.as_mut().unwrap().legs.push(Leg { i, px: x.o[i], w: 0.5 });
            add = false;
        }
        if buy {
            cur = Some(Trade { dir: 1., legs: vec![Leg { i, px: x.o[i], w: 0.5 }], exit_i: i, exit_px: f64::NAN, reason: String::new(),
                entry_kind: "open", signal_i: i, open: false, exits: vec![] });
            buy = false;
        }
        if let Some(t) = cur.as_mut() {
            if i < to && r[i] > ex {
                exit = true;
                t.reason = format!("RSI(2) > {}, sold next open", ex);
                t.signal_i = i;
            } else if i < to && g("add") >= 0.5 && t.legs.len() == 1 && i == sig + 1 && c[i] < ma2[i] {
                add = true;
            }
            continue;
        }
        if i < to && setup(i) {
            buy = true;
            sig = i;
        }
    }
    if let Some(mut t) = cur {
        (t.exit_i, t.exit_px, t.reason, t.signal_i, t.open) = (to, c[to], "open".into(), to, true);
        out.push(t);
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn needs_benchmark_hv() {
        let r: Vec<crate::Row> = (0..300).map(|i| crate::Row { ticker: "T".into(), date: format!("d{:04}", i), open: 10., high: 10., low: 10., close: 10., volume: Some(1.) }).collect();
        let x = Ctx::new(&r, &r, &[]);
        assert!(run("prap1_raptor10", &x, 0, 299, &Params::new()).is_none(), "no BENCH_HV100 -> not run");
        assert!(catalogue().iter().all(|d| d.status != Status::Live));
    }
}
