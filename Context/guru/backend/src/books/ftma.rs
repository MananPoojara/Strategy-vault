//! Lundy Stafford Hill, "The 200 Day Moving Average as an Overbought Indicator", Futures Truth Magazine issue 5-2009 (research share
//! TradersWorld/Futures Truth 2007-2010 issues, row R268). Book id `ftma`. Lane K2 (25 Nov 2026, tier 6). Independent check authors/conv-1123/k2w/qi/ftma_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::tselhs::Book;

pub const REVIEWED: &[&str] = &["ftma_ma200_ob"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "A|[Declared readings, lane K2, 3 counted] The source says: buy on today's open if yesterday closed a chosen percentage above the 200-day moving average, and go flat at the 200-day moving average (read literally: a stop-type exit at the average price, so the exit fills when the day's low touches the average). Unstated, numbered: (1) the 200-day moving average is the simple average of closes, and the level of the exit is the average as of the previous close (the order rests during the day); it fills at that price, or at the open when a bar opens below it; (2) a new entry needs a fresh day that closed the chosen percentage above the average (and the entry test is at or above); (3) the average and the percentage are on the instrument's own closes (the article's S&P 500 futures run does not say cash or spliced; labelled adaptation on SPY and the NSE indices). The percentage is a parameter (the article prints 0, 5, 10 and 15; 10 is the default). Live 200-day near-twins with different rules: capttm_price_ma, apta_ma200_dji, sglr_dow_ma200_band, lydpes_ma200_8pct.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "ftma_ma200_ob", name: "Hill: buy at the open after a close 10% above the 200-day average, go flat at the average", group: "Futures Truth: the 200-day average as an overbought indicator (Stafford Hill)", session: 0, dir: "Long", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "ftma", rules: &[
            "E|\"Buy on today's open if yesterday closed a certain percentage above the 200 day moving average\" (0, 5, 10 and 15 percent tested; S&P 500 futures 1983 to 31 Oct 2009)",
            "X|\"Liquidating (go flat) at the 200 day moving average\" (read as a stop-type exit at the average, filled when the low touches it); commission and slippage 100 USD per trade in the article",
            NOTE], notes: NOTE, claim: None }]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { reviewed(id).then_some(("ftma", "Hill 200-day overbought", "Close above the 200-day average, flat at it")) }
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    reviewed(id).then(|| vec![ParamDef { key: "pct", label: "Close above the 200-day SMA, % (the article tests 0, 5, 10, 15)", def: 10., min: 0., max: 30., step: 1. }])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let k = 1. + p.get("pct").copied().unwrap_or(10.) / 100.;
    let ma = x.sma_n(200);
    let mut pos = Book::new();
    for t in from.max(1)..=to {
        if pos.side() == 0. && ma[t - 1].is_finite() && x.c[t - 1] >= k * ma[t - 1] { pos.enter(1., t, x.o[t], "open"); }
        if pos.side() != 0. && ma[t - 1].is_finite() && x.l[t] <= ma[t - 1] { pos.exit(t, x.o[t].min(ma[t - 1]), "touch of the 200-day average"); }
    }
    Some(pos.done(to, x.c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    /// hand-run dump for the pandas reference: FTMA_CSV=<csv> FTMA_OUT=<file> FTMA_TICKERS=<comma list> cargo test --release ftma_dump -- --ignored
    #[test]
    #[ignore]
    fn ftma_dump() {
        use std::io::Write;
        let data = crate::load_csv(&std::env::var("FTMA_CSV").unwrap()).unwrap();
        let want: Vec<String> = std::env::var("FTMA_TICKERS").unwrap().split(',').map(String::from).collect();
        let mut f = std::io::BufWriter::new(std::fs::File::create(std::env::var("FTMA_OUT").unwrap()).unwrap());
        writeln!(f, "id,ticker,pct,dir,entry_date,entry_px,exit_date,exit_px,open").unwrap();
        for (t, rows) in data.iter().filter(|(t, _)| want.contains(t)) {
            let x = Ctx::new(rows, rows, &[]);
            for pct in [0., 5., 10., 15.] {
                let mut p = Params::new(); p.insert("pct".into(), pct);
                for z in run("ftma_ma200_ob", &x, 0, rows.len() - 1, &p).unwrap() {
                    writeln!(f, "ftma_ma200_ob,{t},{pct},{},{},{},{},{},{}", z.dir, rows[z.legs[0].i].date, z.legs[0].px, rows[z.exit_i].date, z.exit_px, z.open).unwrap();
                }
            }
        }
    }
    #[test]
    fn enters_at_the_open_after_the_stretched_close_and_exits_at_the_average() {
        use crate::Row;
        let mut r: Vec<Row> = (0..230).map(|i| { let c = 100.; Row { ticker: "T".into(), date: format!("{i:04}"), open: c, high: c + 1., low: c - 1., close: c, volume: None } }).collect();
        r[220].close = 112.; r[220].high = 113.; // 12% above the 200-day average (about 100.06)
        r[221].open = 111.; r[221].close = 111.; r[221].low = 110.; r[221].high = 112.; r[222].high = 111.; r[222].close = 95.; r[222].low = 94.; r[222].open = 110.;
        let x = Ctx::new(&r, &r, &[]);
        let t = run("ftma_ma200_ob", &x, 0, 229, &Params::new()).unwrap();
        assert_eq!(t[0].legs[0].i, 221);
        assert_eq!(t[0].legs[0].px, 111.);
        assert_eq!(t[0].exit_i, 222);
        assert!((t[0].exit_px - 100.).abs() < 1.0); // filled at the average, not at the close
    }
}

// Instrument admission: Hill 200-day overbought 2026-11-25 (lane K2, tier 6).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "ftma" && reviewed(def.id),
    label: "The article tests S&P 500 futures 1983 to 31 Oct 2009; run on SPY (substitute, 1993-) for the comparison and on the five NSE index series; the gate row allows only those (R4 #378).",
    basis: "Futures Truth Magazine 5-2009, Lundy Stafford Hill, The 200 Day Moving Average as an Overbought Indicator; independent check authors/conv-1123/k2w/qi/ftma_ref.py",
};
