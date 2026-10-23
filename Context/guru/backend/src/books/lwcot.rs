//! Larry Williams, "Trade Stocks and Commodities with the Insiders: Secrets of the COT Report" (Wiley, 2005). Book id `lwcot`.
//! Source (read in place; COPY PENDING): research drive "Trading Books/Larry Williams - Trade Stocks and Commodities with the
//! Insiders. Secrets of the COT Report/Trade Stocks and Commodities With the Insiders - Secrets of the COT Report.pdf" (text,
//! 205 pp, read in full). Transcription and checks: library/lwcot/. Page refs = printed page numbers.
//!
//! Most of the book needs CFTC Commitments of Traders, open interest or futures data (NeedsData). The live rules are the
//! S&P studies of chapters 7 and 10, run on the index (instrument swap): $ amounts at $250 per S&P point (as lwlts/lwicw).
//! Uses the lwicw simulator (open entries, dollar stop armed on the entry day, first profitable close after the entry bar).
use super::lwicw::{getter, sim, Cfg, Ord};
use crate::connors::{claim, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "lwcot";

/// Larry Williams block: rules admitted after the 23 Oct 2026 reading and independent ledgers
/// (authors/lwilliams/admission/SOURCE-CONTRACT.md). SPX_POINTS = dollar stops kept as S&P points: actual SPX only.
pub const REVIEWED: &[&str] = &["lwcot_vol_p5up_v5up", "lwcot_vol_p5up_v5dn", "lwcot_vol_dn_vdn", "lwcot_vol_dn_vup", "lwcot_vol_t50dn_p8up_v8up", "lwcot_vol_t50dn_p8up_v8dn", "lwcot_vol_t50dn_p8dn_v8up", "lwcot_vol_t50dn_p8dn_v8dn", "lwcot_down_closes_daytrade", "lwcot_every_day_bail"];
pub const SPX_POINTS: &[&str] = &["lwcot_vol_p5up_v5up", "lwcot_vol_p5up_v5dn", "lwcot_vol_dn_vdn", "lwcot_vol_dn_vup", "lwcot_vol_t50dn_p8up_v8up", "lwcot_vol_t50dn_p8up_v8dn", "lwcot_vol_t50dn_p8dn_v8up", "lwcot_vol_t50dn_p8dn_v8dn"];

macro_rules! sd {
    ($id:expr, $name:expr, $group:expr, $dir:expr, $status:expr, $vol:expr, $rules:expr, $notes:expr, $claim:expr) => {
        StrategyDef { id: $id, name: $name, group: $group, session: 0, dir: $dir, status: $status, needs_volume: $vol,
            needs_vix: false, needs_series: &[], book: BOOK, rules: $rules, notes: $notes, claim: $claim }
    };
}

const COT: &str = "Needs weekly CFTC Commitments of Traders data (commercial / large / small trader net positions) for the futures market; the vault has no COT series.";
const VOL_X: &str = "X|\"using an entry of buying on the open and exiting on the first profitable close or using a $3,500 stop\" (p.66): $3,500 = 14 S&P points, armed on the entry day; first profitable close = first close after the entry day above the entry price (author's code 'Exit on Entry Bar: False', MDC manual p.99, as lwicw)";
const VOL_N: &str = "Figs 7.1-7.8 (p.66-71): S&P 500, 1990-2004, equity curves only (no numbers printed). \"None of the following patterns are tradable as such\" (p.66): studies. Volume = the index's own daily volume; days without volume give no signal. A rising/falling n-day moving average equals close (volume) above/below its value n days ago, which is how it is computed.";

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::{Live, NeedsData, NotTestable};
    let v = |id, name, rule: &'static [&'static str]| sd!(id, name, "Ch 7 · Price and volume (S&P)", "Long", Live, true, rule, VOL_N, None);
    vec![
        v("lwcot_vol_p5up_v5up", "Buy: 5-day MA of price up, 5-day MA of volume up",
            &["F|\"a five-day moving average of price is greater than the prior day and at the same time the five-day moving average of volume is greater than the prior day\" (Fig 7.1, p.66)", "E|Buy on the next open", VOL_X]),
        v("lwcot_vol_p5up_v5dn", "Buy: 5-day MA of price up, 5-day MA of volume down",
            &["F|\"the five-day moving average of price is greater than the prior day while the five-day moving average of volume is less than the prior day\" (Fig 7.2, p.67)", "E|Buy on the next open", VOL_X]),
        v("lwcot_vol_dn_vdn", "Buy: down close on lower volume",
            &["F|\"both price and volume are down for the day\" (Fig 7.3, p.68)", "E|\"buy on the next opening\" (p.68)", VOL_X]),
        v("lwcot_vol_dn_vup", "Buy: down close on higher volume",
            &["F|\"a down close day while volume increases or expands from the prior day\" (Fig 7.4, p.68)", "E|Buy on the next opening", VOL_X]),
        v("lwcot_vol_t50dn_p8up_v8up", "Buy: 50-day MA down, 8-day MA of price up, 8-day MA of volume up",
            &["F|\"a 50-day moving average of price is down and over the past eight days both the moving averages of price and volume are higher\" (Fig 7.5, p.69); compared with the prior day (Fig 7.6 wording)", "E|Buy on the next open", VOL_X]),
        v("lwcot_vol_t50dn_p8up_v8dn", "Buy: 50-day MA down, 8-day MA of price up, 8-day MA of volume down",
            &["F|\"a 50-day downtrend in price with a shorter-term 8-day uptrend in price, but this time the 8-day moving average of volume is lower than the prior day\" (Fig 7.6, p.69-70)", "E|Buy on the next open", VOL_X]),
        v("lwcot_vol_t50dn_p8dn_v8up", "Buy: price below 50 days ago, 8-day MA of price down, 8-day MA of volume up",
            &["F|\"the same price lower than 50 days ago but also requires the 8-day moving average of price to be down while the 8-day moving average of volume has been up\" (Fig 7.7, p.70-71)", "E|Buy on the next open", VOL_X]),
        v("lwcot_vol_t50dn_p8dn_v8dn", "Buy: price below 50 days ago, 8-day MA of price down, 8-day MA of volume down",
            &["F|\"the same setup as Figure 7.7 but the 8-day moving average of volume has been down\" (Fig 7.8, p.71)", "E|Buy on the next open", VOL_X]),
        sd!("lwcot_down_closes_daytrade", "Buy the open after N consecutive down closes, exit the same close", "Ch 10 · Randomness (S&P)", "Long", Live, false,
            &["F|\"a string of consecutive down closes\" of N days (Table 10.2, p.118); the table's N = 1 row has 2,507 trades = every down close of Table 10.1, so N means at least N",
              "E|\"buying on the opening of the day after a string of consecutive down closes\" (p.118)", "X|\"exiting on the close that same day\" (p.118); no stop"],
            "Table 10.2 (p.118), S&P 500 \"since 1982\" (to 2004): N=1 2,507 trades -$165,990; N=2 1,119 -$46,618; N=3 480 $38,563; N=4 182 $93,210; N=5 59 $56,783. Default N = 1 (the table's first row); verify runs every row.",
            claim("2,507", "", "-$165,990 total (N=1)", "0 days", "S&P since 1982")),
        sd!("lwcot_every_day_bail", "Buy the open every day, exit on the first profitable opening", "Ch 10 · Randomness (S&P)", "Long", Live, false,
            &["E|\"had you bought on the open every day since 1982\" (p.120); one position at a time",
              "X|\"and exited on the first profitable opening\" (p.120): the first opening after the entry day above the entry price; no stop",
              "E|Read: after an exit at an opening the next entry is the next day's opening (an order placed while long is not a new position; the book does not say)"],
            "p.120, S&P 500 since 1982: \"you would have made $95,563 on 1,420 trades. The problem is that the drawdown of $210,955 ...\".",
            claim("1,420", "", "$95,563 total", "", "S&P 1982-2004")),
        // ---- needs data / not testable
        sd!("lwcot_cot_index", "COT index extremes, 26-week/3-year look-back, WILLCO, public and large-trader indexes", "Ch 3-6, 10 · COT", "Long/Short", NeedsData(COT), false,
            &["F|COT index = (this week's net - lowest of 3 years) / (highest - lowest of 3 years) x 100; above 80 bullish, below 20 bearish (p.33-34)", "F|WILLCO = stochastic of commercial net / open interest over 26 weeks (p.105)", "F|Large-trader index > 80 sell, < 20 buy (p.63)"],
            "Set-ups, not trades: \"The index is not a timing tool\" (p.34). No entry order or exit is given for these.", None),
        sd!("lwcot_cot_sp_bias", "S&P: buy the open when the 6-month COT index is above 50%", "Ch 10 · COT", "Long", NeedsData(COT), false,
            &["F|\"buy the S&P 500 on the opening each day only if the COT commercials index, using a six-month time frame\" was above 50 percent (p.121)"],
            "p.121: COT > 50% 371 trades $76,605, DD $39,145; < 50% 846 trades -$170; T-Bonds 606 trades $28,418 / 633 trades -$13,985. Exit not stated.", None),
        sd!("lwcot_one_minute", "One-minute trader: COT index > 80 with a rising 52-week MA", "Ch 13 · COT and trend", "Long/Short", NeedsData(COT), false,
            &["F|\"this week's 52-week moving average was greater than two weeks ago, and ... the six-month COT index was greater than 80 percent\" (p.151)"],
            "Table 13.1 (p.151): British Pound $51,311 21 tr 71%; Bonds $51,736 26 69%; Swiss Franc $19,093 16 56%; S&P 500 $82,683 24 54%; Gold $13,285 13 69%. Exit not stated.", None),
        sd!("lwcot_oi_volume_futures", "Open-interest stochastic, OI vs commercials, price/volume tests on pound and bond futures, gold vs dollar", "Ch 7-9, 13 · OI and futures", "Long/Short",
            NeedsData("Needs futures open interest, futures volume (British pound, T-Bonds) and the US Dollar Index / gold futures; none are in the vault."), false,
            &["F|12-month stochastic of open interest (p.83-89); 20-day exit after price/volume MA patterns on the pound and bonds (p.72-78); gold/dollar spread 3-21 week > 30% (p.154-155)"],
            "The pound/bond tests print totals (e.g. price down/volume up 102 trades $42,673, p.75) but the MA length is given only as an example (\"for example, a 60-day\").", None),
        sd!("lwcot_judgement", "ProGo divergence, trend-line entries, 17/13-day trend stops, chart patterns, Fibonacci study, proxy index", "Ch 10-12, 14", "Long/Short",
            NotTestable("Judgement: divergences and trend lines are drawn by eye; the 17-day stop applies only \"if you believe\" a major trend is under way (p.145); the stock proxy index is not revealed (p.131); the Table 10.3/10.4 system is not described."), false,
            &["F|ProGo = 14-day averages of close-to-open (public) and open-to-close (professional) changes (p.110)", "X|\"use the lowest low of the last 17 days as your protective stop ... reenter at the highest high of the last 13 days if it looks like the trend is again exerting itself\" (p.145)"],
            "The Fibonacci tables (p.191-194) and candlestick comments (p.169) are studies, not rules.", None),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let f = |a, b, c| Some((a, b, c));
    match id {
        "lwcot_vol_p5up_v5up" => f("lwcot_vol", "Price and volume studies", "5d price up, volume up"),
        "lwcot_vol_p5up_v5dn" => f("lwcot_vol", "Price and volume studies", "5d price up, volume down"),
        "lwcot_vol_dn_vdn" => f("lwcot_vol", "Price and volume studies", "Down day, volume down"),
        "lwcot_vol_dn_vup" => f("lwcot_vol", "Price and volume studies", "Down day, volume up"),
        "lwcot_vol_t50dn_p8up_v8up" => f("lwcot_vol", "Price and volume studies", "50d down, 8d price up, volume up"),
        "lwcot_vol_t50dn_p8up_v8dn" => f("lwcot_vol", "Price and volume studies", "50d down, 8d price up, volume down"),
        "lwcot_vol_t50dn_p8dn_v8up" => f("lwcot_vol", "Price and volume studies", "50d down, 8d price down, volume up"),
        "lwcot_vol_t50dn_p8dn_v8dn" => f("lwcot_vol", "Price and volume studies", "50d down, 8d price down, volume down"),
        "lwcot_down_closes_daytrade" => f("lwcot_random", "Randomness studies", "N down closes, day trade"),
        "lwcot_every_day_bail" => f("lwcot_random", "Randomness studies", "Every day, bailout"),
        "lwcot_cot_index" | "lwcot_cot_sp_bias" | "lwcot_one_minute" => f("lwcot_cot", "COT set-ups", "Needs COT data"),
        "lwcot_oi_volume_futures" => f("lwcot_cot", "COT set-ups", "Needs futures data"),
        "lwcot_judgement" => f("lwcot_cot", "COT set-ups", "Not testable"),
        _ => None,
    }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let stop = || p("stop_usd", "Dollar stop (book $3,500)", 3500.0, 0.0, 10000.0, 250.0);
    let pt = || p("pt_usd", "Dollars per index point (book: S&P $250)", 250.0, 50.0, 2500.0, 50.0);
    let trend = || p("trend_n", "Price below N days ago = N-day MA down (book 50)", 50.0, 10.0, 200.0, 5.0);
    Some(match id {
        "lwcot_vol_p5up_v5up" | "lwcot_vol_p5up_v5dn" => vec![p("ma_n", "Moving average days (book 5)", 5.0, 2.0, 20.0, 1.0), stop(), pt()],
        "lwcot_vol_dn_vdn" | "lwcot_vol_dn_vup" => vec![stop(), pt()],
        "lwcot_vol_t50dn_p8up_v8up" | "lwcot_vol_t50dn_p8up_v8dn" | "lwcot_vol_t50dn_p8dn_v8up" | "lwcot_vol_t50dn_p8dn_v8dn" =>
            vec![trend(), p("ma_n", "Short moving average days (book 8)", 8.0, 2.0, 20.0, 1.0), stop(), pt()],
        "lwcot_down_closes_daytrade" => vec![p("n", "Consecutive down closes, at least (book rows 1-5)", 1.0, 1.0, 5.0, 1.0)],
        "lwcot_every_day_bail" => vec![],
        _ => return None,
    })
}

/// Every-day entry with the bailout: buy the open when flat at the prior close; exit at the first opening after the entry
/// day that is above the entry price. No same-open re-entry.
fn every_day(x: &Ctx, from: usize, to: usize) -> Vec<Trade> {
    let mut out = vec![];
    let mut pos: Option<(usize, f64)> = None;
    let mk = |i: usize, px: f64, j: usize, ex: f64, why: &str, open: bool| Trade { dir: 1.0, legs: vec![Leg { i, px, w: 1.0 }],
        exit_i: j, exit_px: ex, reason: why.into(), entry_kind: "open", signal_i: j, open, exits: vec![] };
    for i in from..=to {
        match pos {
            Some((e, px)) if i > e && x.o[i] > px => { out.push(mk(e, px, i, x.o[i], "bailout: first profitable opening", false)); pos = None; }
            Some(_) => {}
            None => pos = Some((i, x.o[i])),
        }
    }
    if let Some((e, px)) = pos { out.push(mk(e, px, to, x.c[to], "open", true)); }
    out
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if from > to || to >= x.r.len() { return None; }
    let g = getter(&schema, p);
    let c = &x.c;
    let vol = |k: usize| x.r[k].volume.filter(|v| *v > 0.0);
    // close and volume above (dir > 0) / below (dir < 0) their value n bars earlier, on bar k
    let up = |k: usize, n: usize| k >= n && c[k] > c[k - n];
    let dn = |k: usize, n: usize| k >= n && c[k] < c[k - n];
    // an n-day volume average needs volume on every day of [k-n, k] (a day without volume gives no signal)
    let vcmp = |k: usize, n: usize, dir: f64| k >= n && (k - n..=k).all(|j| vol(j).is_some())
        && match (vol(k), vol(k - n)) { (Some(a), Some(b)) => dir * (a - b) > 0.0, _ => false };
    let t = match id {
        "lwcot_every_day_bail" => every_day(x, from, to),
        "lwcot_down_closes_daytrade" => {
            let n = g("n").max(1.0) as usize;
            let mut cfg = Cfg::new(f64::NAN);
            cfg.hold = Some(0);
            sim(x, from, to, &cfg, |i| if i >= n + 1 && (1..=n).all(|k| c[i - k] < c[i - k - 1]) { vec![(1.0, Ord::Open)] } else { vec![] })
        }
        _ => {
            let mut cfg = Cfg::new(g("stop_usd") / g("pt_usd"));
            cfg.prof_close = true;
            let m = g("ma_n") as usize;
            let tn = g("trend_n") as usize;
            let cond: Box<dyn Fn(usize) -> bool> = match id {
                "lwcot_vol_p5up_v5up" => Box::new(move |k| up(k, m) && vcmp(k, m, 1.0)),
                "lwcot_vol_p5up_v5dn" => Box::new(move |k| up(k, m) && vcmp(k, m, -1.0)),
                "lwcot_vol_dn_vdn" => Box::new(move |k| dn(k, 1) && vcmp(k, 1, -1.0)),
                "lwcot_vol_dn_vup" => Box::new(move |k| dn(k, 1) && vcmp(k, 1, 1.0)),
                "lwcot_vol_t50dn_p8up_v8up" => Box::new(move |k| dn(k, tn) && up(k, m) && vcmp(k, m, 1.0)),
                "lwcot_vol_t50dn_p8up_v8dn" => Box::new(move |k| dn(k, tn) && up(k, m) && vcmp(k, m, -1.0)),
                "lwcot_vol_t50dn_p8dn_v8up" => Box::new(move |k| dn(k, tn) && dn(k, m) && vcmp(k, m, 1.0)),
                "lwcot_vol_t50dn_p8dn_v8dn" => Box::new(move |k| dn(k, tn) && dn(k, m) && vcmp(k, m, -1.0)),
                _ => return None,
            };
            sim(x, from, to, &cfg, |i| if i >= 1 && cond(i - 1) { vec![(1.0, Ord::Open)] } else { vec![] })
        }
    };
    Some(t)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn bar(d: &str, o: f64, h: f64, l: f64, c: f64, v: f64) -> Row {
        Row { ticker: "T".into(), date: d.into(), open: o, high: h, low: l, close: c, volume: Some(v) }
    }
    #[test]
    fn down_close_daytrade_and_every_day() {
        let r = vec![bar("2000-01-03", 10., 11., 9., 10., 1.), bar("2000-01-04", 10., 10., 8., 9., 1.), bar("2000-01-05", 9., 9.5, 7., 8., 1.),
            bar("2000-01-06", 8.5, 10., 8., 9.5, 1.), bar("2000-01-07", 9., 10., 8.8, 9.8, 1.)];
        let x = Ctx::new(&r, &r, &[]);
        // two down closes (01-04, 01-05): N=2 buys the 01-06 open 8.5, exits its close 9.5
        let t = run("lwcot_down_closes_daytrade", &x, 1, 4, &Params::from([("n".to_string(), 2.0)])).unwrap();
        assert_eq!(t.len(), 1);
        assert_eq!((t[0].legs[0].i, t[0].legs[0].px, t[0].exit_i, t[0].exit_px), (3, 8.5, 3, 9.5));
        // every day: buy 01-04 open 10; first profitable opening after it: none (9, 8.5, 9) -> still open
        let t = run("lwcot_every_day_bail", &x, 1, 4, &Params::new()).unwrap();
        assert_eq!((t.len(), t[0].open), (1, true));
        // from 01-05: buy 9; 01-06 opens 8.5 (no); 01-07 opens 9 (not above 9) -> open
        let t = run("lwcot_every_day_bail", &x, 2, 4, &Params::new()).unwrap();
        assert_eq!((t[0].legs[0].px, t[0].open), (9.0, true));
        // from 01-06: buy 8.5, 01-07 opens 9 > 8.5 -> exit 9, then buy is not at the same open
        let t = run("lwcot_every_day_bail", &x, 3, 4, &Params::new()).unwrap();
        assert_eq!((t.len(), t[0].exit_i, t[0].exit_px), (1, 4, 9.0));
    }
}
