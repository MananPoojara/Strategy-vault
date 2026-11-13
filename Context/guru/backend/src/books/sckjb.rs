//! Jay Kaeppel, "The January Barometer: Myth And Reality", Technical Analysis of Stocks & Commodities V.8:7 (1990) pp.252-254
//! (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V08/C07/JANBARO.pdf;
//! Figure 2, the six January patterns, is an image: read with authors/ocr2, PDF p.4). Book id `sckjb`. Lane PA2 (13 Nov 2026);
//! independent check authors/pa2/w/pa2_ref.py. Admission: sclang.rs (PA2_BOOKS).
//! Patterns 3, 4 and 5 together are 'January as a whole up' = the live kpss_janbar rule (Kaeppel's 2009 book), so only the single
//! patterns are coded here; the default is pattern 4, the one the article singles out.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::tselhs::Book;

pub const REVIEWED: &[&str] = &["sckjb_p4"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Calendar rule, no unstated detail; unverified] Figure 2 (PDF p.4) defines six patterns from the S&P 500's first five trading days of the year and the month of January: 1 first 5 days decline, January declines further; 2 first 5 days decline, January less decline but still a loss; 3 first 5 days decline, January a gain; 4 first 5 days advance, January gains even more; 5 first 5 days advance, January less gain than the first 5 days but still a gain; 6 first 5 days advance, January a loss. Each is judged on closes: first 5 days = the 5th January close against the last close of the previous year; January = the last January close against the same close; 'even more' / 'less' = the last January close against the 5th January close (an exact tie fits no pattern). The forecast is for 'the S&P 500 index between January 31 and December 31', so the trade buys at the close of the last January trading day and sells at the close of the last trading day of the year. Default pattern 4 ('The most favorable pattern has been 4 ... 21 occasions ... 90% of the time ... The average 11-month gain has been 13.6%', S&P 500 1937-1989); option pattern = 3 or 5 (the other bullish patterns, each 'correct five out of six times, with an average February-December gain of about 10%'). Patterns 3, 4 and 5 together are the plain January barometer (live kpss_janbar), not coded again. The bearish patterns name no trade ('could look to sell short stock-index futures following market rallies'), so they are not coded. Our SPX history starts in 1983, so the article's 1938-1989 record cannot be reproduced; runs on other indices, ETFs and stocks, US and India, are a declared application of the S&P rule.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "sckjb_p4", name: "January pattern 4: first five days up and January higher still -> long from the last January close to the last close of the year", group: "S&C: the January barometer, myth and reality (Kaeppel, V.8:7)", session: 0, dir: "Long", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "sckjb", rules: &[
            "F|Figure 2 'January Barometer patterns' (PDF p.4, ocr2): \"4 Advances | Gains even more | Bullish\"; \"3 Declines | Gain for month | Bullish\"; \"5 Advances | Less gain than first 5 days but still a gain for month | Bullish\" (columns: during first 5 days of year / by end of Jan. / implication for Feb.-Dec, all S&P 500)",
            "F|\"The technique involves comparing the action of the first five trading days of the year and January as a whole.\" (PDF p.2)",
            "E|\"Since 1937, there have been 21 occasions when the S&P 500 stock index has advanced in the first five days of January and then advanced even further by the end of the month. On 19 of those occasions, or 90% of the time, the S&P rose between the end of January and the end of December. The average 11-month gain has been 13.6%.\" (PDF p.2)",
            "X|\"to forecast action of the S&P 500 index between January 31 and December 31 of each year\" (PDF p.2): sell at the year's last close",
            "E|\"An investor could wait for patterns 3, 4 or 5 to develop, buy stocks or stock mutual funds\" (PDF p.3)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { reviewed(id).then_some(("sckjb_p4", "Kaeppel January patterns (S&C 1990)", "Pattern")) }
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    reviewed(id).then(|| vec![ParamDef { key: "pattern", label: "January pattern traded (4 = article's best; 3 or 5 = the other bullish patterns)", def: 4., min: 3., max: 5., step: 1. }])
}

/// Kaeppel's pattern 1-6 from the previous year's last close c0, the 5th January close c5 and the last January close cj (0 = none).
pub fn pattern(c0: f64, c5: f64, cj: f64) -> u8 {
    if c5 < c0 { if cj < c5 { 1 } else if cj > c0 { 3 } else if cj < c0 && cj > c5 { 2 } else { 0 } }
    else if c5 > c0 { if cj > c5 { 4 } else if cj > c0 && cj < c5 { 5 } else if cj < c0 { 6 } else { 0 } }
    else { 0 }
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from > to || to >= x.r.len() { return None; }
    let want = p.get("pattern").copied().unwrap_or(4.).round().clamp(3., 5.) as u8;
    let ym = |i: usize| -> (i32, u32) { let d = &x.r[i].date; (d[0..4].parse().unwrap_or(0), d[5..7].parse().unwrap_or(0)) };
    let mut pos = Book::new();
    let n = x.r.len();
    let mut i = 1;
    while i < n {
        let (y, m) = ym(i);
        // the first bar of a new year whose previous bar is the previous year's last close
        if m == 1 && ym(i - 1).0 == y - 1 {
            let jan: Vec<usize> = (i..n).take_while(|&k| ym(k) == (y, 1)).collect();
            let last = (i..n).take_while(|&k| ym(k).0 == y).last().unwrap_or(i);
            if jan.len() >= 5 && *jan.last().unwrap() + 1 < n {
                let (c0, c5, jl) = (x.c[i - 1], x.c[jan[4]], *jan.last().unwrap());
                if pattern(c0, c5, x.c[jl]) == want && jl >= from && jl < to {
                    pos.enter(1., jl, x.c[jl], "close");
                    if last < to || (last == to && last + 1 < n) { pos.exit(last, x.c[last], "last close of the year"); }
                    else { return Some(pos.done(to, x.c[to])); }
                }
            }
            i = last + 1;
        } else { i += 1; }
    }
    Some(pos.done(to, x.c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn patterns_and_calendar_trades() {
        assert_eq!([pattern(100., 99., 98.), pattern(100., 98., 99.), pattern(100., 99., 101.), pattern(100., 101., 102.), pattern(100., 102., 101.), pattern(100., 101., 99.)], [1, 2, 3, 4, 5, 6]);
        assert_eq!(pattern(100., 100., 105.), 0);
        let r = super::super::sctraub::tests::bars(2600); // 2001-2010 business days
        let x = Ctx::new(&r, &r, &[]);
        for k in 3..=5 {
            let t = run("sckjb_p4", &x, 0, r.len() - 1, &Params::from([("pattern".into(), k as f64)])).unwrap();
            for z in &t {
                let (e, xx) = (&r[z.legs[0].i].date, &r[z.exit_i].date);
                assert_eq!(&e[5..7], "01"); assert_eq!(&r[z.legs[0].i + 1].date[5..7], "02");
                assert!(z.open || (&xx[5..7] == "12" && r[z.exit_i + 1].date[0..4] != xx[0..4] && xx[0..4] == e[0..4]));
            }
        }
    }
}
