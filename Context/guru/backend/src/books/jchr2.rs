//! Jeff Cooper, "Hit and Run Trading II: Capturing Explosive Short-Term Moves in Stocks" (M. Gordon, 1998).
//! Book id "jchr2". Source: Books/Jeff Cooper/Hit and Run Trading II.pdf (OCR text + page images; page refs =
//! PDF pages). No setup has a mechanical exit: Section Four says where to lock in profits "is much more of an
//! art (gut feeling) than a science" (PDF p.98). Nothing is Live; the rules are listed so the page shows them.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "jchr2";
const TRAIL: &str = "exit is a discretionary trailing stop (\"use a trailing stop\"; Ch 17: profit locking is \"much more of an art (gut feeling) than a science\", PDF p.98)";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, dir, status, rules| StrategyDef {
        id, name, group, session: 0, dir, status, needs_volume: false, needs_vix: false,
        needs_series: &[], book: BOOK, rules, notes: "Hit and Run Trading II (1998). Built for US momentum stocks; no published statistics.", claim: None,
    };
    use Status::*;
    vec![
        d("jchr2_hot_ipo", "Hot IPO Pullbacks", "Ch 1-2 · Hot IPO Pullbacks", "Long",
          NeedsData("IPO calendar with offering prices; rules 1-3 are on a page missing from the scan; exit is trailing stops"),
          &["F|A \"hot issue\" trades more than 15 percent above its offering price (examples, PDF p.6-10)", "X|\"Place your initial protective stop at the previous day's low.\" \"Hold the position from one to five days using trailing stops.\" (PDF p.6)"]),
        d("jchr2_flat_top_xbo", "Flat-Top Expansion Breakouts", "Ch 3 · Flat-Top Expansion Breakouts", "Long/Short",
          NotTestable("discretionary: a 'flat-top' (stock traded sideways) is chart judgement; trailing-stop exit"),
          &["E|An Expansion Breakout out of a sideways period after new highs (PDF p.12-17)"]),
        d("jchr2_reversal_new_highs", "Reversal New Highs Method", "Ch 4 · Reversal New Highs Method", "Long/Short",
          NotTestable("rules 1-3 are on a page missing from the scan; exit is a trailing stop (rule 4, PDF p.19)"),
          &["E|Examples: an outside day, the largest range of five days and a new 60-day high; buy 1/16 above that day's high (PDF p.19-23)", "X|\"Use a trailing stop and stay in the position until you are stopped out.\" (PDF p.19)"]),
        d("jchr2_irsts", "Intraday Relative Strength Trading Strategy", "Ch 5 · Intraday Relative Strength", "Long/Short",
          NotTestable("discretionary intraday tape reading of NASDAQ stocks vs S&P futures"),
          &["E|Buy a NASDAQ leader holding firm while the S&P futures fall, when the futures turn (PDF p.24-30)"]),
        d("jchr2_elb", "Extended Level Boomers", "Ch 6 · Extended Level Boomers", "Long/Short", NotTestable(TRAIL),
          &["E|Day one makes a 60-day high; the next two days and beyond trade inside the day-one range; buy stop (GTC) one tick above the day-one high; cancel if the day-one low trades first (PDF p.31)",
            "X|\"When filled, risk 1 point and use a trailing stop to lock in profits.\" (PDF p.31)"]),
        d("jchr2_non_adx_1234", "Non-ADX 1-2-3-4s", "Ch 7 · Non-ADX 1-2-3-4s", "Long/Short",
          NotTestable("discretionary trend identification (\"your eyes can also detect the trend\", PDF p.36)"),
          &["E|1-2-3-4 pullback in a stock judged to be trending (ideally above/below its 50-day MA) without the ADX > 30 filter (PDF p.36-41)"]),
        d("jchr2_jack_in_the_box", "Jack-In-The-Box", "Ch 8 · Jack-In-The-Box", "Long/Short",
          NotTestable("exit undefined: only \"If the stock closes strongly in the top of the day's range, hold at least half your position overnight\" (PDF p.42)"),
          &["E|Expansion Breakout: 60-day high and the largest range of the previous nine sessions; next day an inside day; the day after, buy 1/16 above the XBO high and risk 1 point (PDF p.42)"]),
        d("jchr2_v_thrust", "V-Thrusts", "Ch 9 · V-Thrusts", "Long",
          NotTestable("entry rule 2 \"A sharp three- to six-day sell-off\" is not defined"),
          &["F|60-day high within the past seven sessions; a sharp three- to six-day sell-off; yesterday rose above the previous day's high (PDF p.48)",
            "E|\"Today only, buy 1/16 above yesterday's high and risk 1 point.\" (PDF p.48)", "X|\"Exit on the close. If the stock closes near the top of its range, you may want to hold one-half overnight.\" (PDF p.48)"]),
        d("jchr2_sifos_new_highs", "Stepping In Front Of Size on New Highs and New Lows", "Ch 10 · Stepping In Front Of Size", "Long/Short",
          NotTestable("needs Level II bid sizes; day-trade exit at the trader's discretion"), &["E|Two consecutive higher bids with size on a thin stock at a 60-day high (PDF p.55)"]),
        d("jchr2_sifos_substantial", "Stepping In Front Of \"Substantial\" Size", "Ch 11 · Stepping In Front Of Size", "Long",
          NotTestable("needs Level II bid sizes (10,000+ share bids)"), &["E|Buy on the offer when a 10,000+ share bid appears; stop 1/16 under the bid (PDF p.59)"]),
        d("jchr2_offering_knockouts", "Offering Knockouts", "Ch 12 · Offering Knockouts", "Long",
          NotTestable("needs Level II offer sizes; \"Exit on your own discretion and before the close\" (PDF p.64)"),
          &["F|NYSE/ASE, 10-day average volume under 200,000, price > 30, ADX > 30, +DI > -DI or RS >= 95, up on the day (PDF p.63-64)", "E|Buy stop 1/16 above a 5,000+ share offer; risk no more than 1/2 point (PDF p.64)"]),
        d("jchr2_erds", "Expansion Range Double Sticks", "Ch 13 · Expansion Range Double Sticks", "Short", NotTestable(TRAIL),
          &["E|Day one 60-day high with at least the third largest range of the past 10 days; day two closes below its open with at least the third largest range of the past 10 days (PDF p.71)",
            "E|\"we will sell short tomorrow, one tick under the day-two low and we will risk 1 point with a protective stop\" (PDF p.71)", "X|\"Use a trailing stop to lock in profits.\" (PDF p.71)"]),
        d("jchr2_boomerangs", "Boomerangs", "Ch 14 · Boomerangs", "Long/Short",
          NotTestable("discretionary: \"somewhat more subjective\"; consolidation 'in a relatively tight range' is undefined (PDF p.77-78)"),
          &["E|Strong trend, 5-20 day tight consolidation, breakout day, then short 1/16 below the breakout-day low on day two, three or four (PDF p.77-78)"]),
        d("jchr2_turtle_soup_expansion", "Turtle Soup Expansions", "Ch 15 · Turtle Soup Expansions", "Long/Short", NotTestable(TRAIL),
          &["E|New 20-day high, previous 20-day high at least four trading days earlier; it reverses below the prior 20-day high today or tomorrow with a range greater than any of the prior four days; short 1 tick below the previous day's low (PDF p.84-85)",
            "X|\"Use a 1 to 1 1/2-point stop initially and a trailing stop as the position moves in your favor.\" (PDF p.85)"]),
        d("jchr2_iguanas", "Iguanas (weekly)", "Ch 16 · Iguanas", "Short",
          NotTestable("rule 1 is on a page missing from the scan; exit is a discretionary trailing stop / holding half"),
          &["E|Week's open and close in the bottom 25% of the weekly range; gap open to a new four-week high on Monday or close below the week's open on Friday; next week short 1/16 below the prior week's low Monday or Tuesday only, risk 1 point (PDF p.92)"]),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    id.starts_with("jchr2_").then_some(("jchr2", "Hit and Run Trading II setups", "Not testable"))
}
pub fn params(_id: &str) -> Option<Vec<ParamDef>> { None }
pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }
