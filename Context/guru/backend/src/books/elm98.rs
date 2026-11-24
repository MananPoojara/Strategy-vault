//! Elm Trader, "The 98% System" (FTSE 100 option selling, Elm Trader 2004-05): research share `USB HDD 2/tRADING KNO-2/_ElmTrader.co.uk/Elm Trader - FTSE 98% System
//! (elmtrader.co.uk)/ftse98% elm trader.pdf` ch.5-7 (read in full). Book id `elm98`. Lane K1 (24 Nov 2026, R4 #397 tier 5): the monthly short-option system on the
//! NIFTY 50 / NIFTYBANK monthly option chain (crate::options, NSE F&O bhavcopy) as a labelled R4 #340 adaptation. Independent check authors/code-1123/ref/elm98_ref.py.
use crate::connors::{claim, Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use crate::options::{self as opt, Contract, Exit};

pub const BOOK: &str = "elm98";
pub const IDS: &[&str] = &["elm98_full", "elm98_six", "elm98_four", "elm98_two"];
/// Opening months (the month of the monthly expiry day on which the book opens) of each variant, p.20-21.
fn months(id: &str) -> &'static [u32] {
    match id { "elm98_six" => &[1, 3, 5, 7, 9, 11], "elm98_four" => &[2, 5, 8, 11], "elm98_two" => &[5, 11], _ => &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12] }
}

const RULES: &[&str] = &[
    "F|\"The 98% System works on the FTSE100 index and it is these options with which you will trade.\" (p.16)",
    "E|\"a Put option sale for positions opened in September, October, November, December, January, February, March and April, and a Call option sale for positions opened in May, June, July and August\"; \"Ascertain the FTSE100 index level shortly before opening a new position and multiply it by 0.9 if you are trading a Put or multiply it by 1.1 if you are trading a Call. The resulting figure should then be rounded to a figure divisible by 25\" (Put 3,690 -> 3,675 down, Call 4,510 -> 4,525 up); \"The expiry month selected will always be three months hence\"; opened on the monthly expiry day around 10:20 (p.16-17)",
    "X|\"Options expire automatically on the third Friday of the expiry month ... there is no activity required to close your position\"; the book uses no stop loss (p.17, p.22-23)",
];
const NOTE: &str = "[Declared adaptation, R4 #340] Elm Trader's 98% System sells FTSE 100 index options on the monthly expiry day (third Friday), a put in the opening months September-April and a call in May-August, strike 0.9 x (put) or 1.1 x (call) the index rounded away from it to a multiple of 25, expiring three months later, held to expiry (cash settlement at the index). Run here on the NIFTY 50 / NIFTYBANK monthly chains (NSE F&O bhavcopy, real contract prices, no model price, a contract that did not trade that day is no trade). The variants of p.20-21: the Six System opens in January, March, May, July, September and November; the Four System in February, May, August and November; the Two System in May and November (the first of the three alternatives the book lists). Labelled readings (4): (1) the NSE lists the series three months out only from the day AFTER the monthly expiry (on the expiry day the farthest listed series is two months out), so the position opens on the first trading day after the monthly expiry, at that day's option close (the book's 10:20 index level and option quote are intraday; the vault holds end-of-day option prices) with the strike taken from that day's index close; the month that decides put or call is the month of the expiry day just passed; (2) 'rounded to a figure divisible by 25' is the NSE's own strike grid: the put strike is the highest listed strike at or below 0.9 x the index close, the call strike the lowest listed at or above 1.1 x (the book's examples round away from the index), the series expiring in the farthest listed month; (3) held to the contract's last trading day and settled at the index close there (intrinsic value), the premium received being the traded close of the opening day; (4) P&L in index points per one unit, valued on the index level at entry as everywhere in the options account (book stakes are pounds per point, not used). No stop. The book's printed result is a table of the FTSE 100's three-monthly changes and the win/loss of the Full, Six, Four and Two Systems month by month (Appendix D, Jan 1984 - Nov 2003): the NIFTY run is a different index and period, so only the win rate is set against the book's.";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, notes: &'static str, c| StrategyDef {
        id, name, group: "Elm Trader FTSE 98% System (monthly short options)", session: 0, dir: "Long", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[],
        book: BOOK, rules: RULES, notes, claim: c,
    };
    vec![
        d("elm98_full", "Elm Trader 98% System, Full: every month sell the 3-month put (Sep-Apr) / call (May-Aug) 10 percent out of the money, held to expiry (NIFTY / BANKNIFTY monthly options)", NOTE, claim("234 monthly positions (Jan 1984 - Nov 2003)", "98.3% (230 of 234; losses Sep-87, Oct-87, Aug-92, Apr-02)", "-", "3 months", "FTSE 100 options, Appendix D p.33-")),
        d("elm98_six", "Elm Trader 98% System, Six: the same sale in January, March, May, July, September and November", NOTE, claim("120 monthly positions over 20 years", "99% (119 of 120)", "-", "3 months", "FTSE 100 options p.20")),
        d("elm98_four", "Elm Trader 98% System, Four: the same sale in February, May, August and November", NOTE, claim("80 positions over 20 years", "99% (79 of 80)", "-", "3 months", "FTSE 100 options p.20")),
        d("elm98_two", "Elm Trader 98% System, Two: the same sale in May and November", NOTE, claim("40 positions over 20 years", "100%", "-", "3 months", "FTSE 100 options p.21")),
    ]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    IDS.contains(&id).then(|| ("elm98", "Elm Trader 98% System", match id { "elm98_six" => "Six System", "elm98_four" => "Four System", "elm98_two" => "Two System", _ => "Full System" }))
}
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    IDS.contains(&id).then(|| vec![ParamDef { key: "otm", label: "Strike distance from the index (percent)", def: 10.0, min: 2.0, max: 20.0, step: 1.0 }])
}
pub fn is_rule(id: &str) -> bool { IDS.contains(&id) }

/// Option positions of an elm98 rule (None = not an elm98 rule; Err = no chain for this instrument).
pub fn option_positions(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Result<Vec<opt::Pos>, String>> {
    if !is_rule(id) { return None; }
    let Some(chain) = x.options else { return Some(Err(crate::books::oiss::NO_CHAIN.into())) };
    let otm = p.get("otm").copied().unwrap_or(10.0) / 100.0;
    let mut w = opt::Window::new(chain, x, 8);
    let mut out = Vec::new();
    let mut i = from.max(1);
    while i <= to {
        let prev = w.at(i - 1).iter().map(|q| q.expiry).max();
        let (day, far) = { let d = w.at(i); (d.iter().map(|q| q.expiry).max(), d.to_vec()) };
        let (Some(f0), Some(f1)) = (prev, day) else { i += 1; continue };
        let near = opt::expiries(w.at(i - 1)).first().copied();
        // the series three months out is listed for the first time today, the day after the monthly expiry
        if f1 <= f0 || near.map_or(true, |e| opt::ltd(x, e) != opt::ymd(&x.r[i - 1].date)) { i += 1; continue; }
        let month: u32 = x.r[i - 1].date[5..7].parse().unwrap_or(0);
        if !months(id).contains(&month) { i += 1; continue; }
        let put = !(5..=8).contains(&month);
        let s = x.c[i];
        let ks = opt::strikes(&far, f1, put);
        let strike = if put { ks.iter().copied().filter(|k| *k <= s * (1.0 - otm) + 1e-9).last() } else { ks.iter().copied().find(|k| *k >= s * (1.0 + otm) - 1e-9) };
        let c = strike.map(|strike| Contract { expiry: f1, put, strike });
        let Some((c, px0)) = c.and_then(|c| w.quote(i, c).and_then(|q| q.fill_close()).map(|v| (c, v))) else { i += 1; continue };
        let (j, pnl, why, open, marks) = opt::follow(&mut w, x, c, -1.0, i, px0, to, &mut |_| None::<(Exit, String)>);
        out.push(opt::Pos { dir: if put { 1.0 } else { -1.0 }, s0: s, entry_i: i, exit_i: j, pnl, open,
            reason: format!("{why} · sold {} {} exp {} on {} at {px0:.2} · {}", c.strike, if put { "put" } else { "call" }, c.expiry, x.r[i].date, marks.text()) });
        i += 1;
    }
    Some(Ok(out))
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    option_positions(id, x, from, to, p).map(|r| opt::account(r.unwrap_or_default(), from, to, 0.0).trades)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn variants_open_in_the_books_months() {
        assert_eq!(months("elm98_six").len(), 6);
        assert_eq!(months("elm98_four"), &[2, 5, 8, 11]);
        assert!(params("elm98_full").is_some() && params("x").is_none() && !is_rule("x"));
    }
}
