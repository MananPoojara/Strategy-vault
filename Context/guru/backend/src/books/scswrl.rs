//! John Sweeney, "Reversing Your Losses", Technical Analysis of Stocks & Commodities V.10:4 (1992) pp.184-186 (research share
//! Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V10/C04/REVERSI.pdf), with the
//! underlying T-bond system as printed in his column "Testing Trading Rules Over Different Time Periods", V.10:1 (1992)
//! pp.43-45, Figure 1 (VOLUMES/V10/C01/TESTING.pdf, PDF p.2: System Writer Plus model 'S&S w Stops.MDL').
//! Book id `scswrl`. Lane PG (13 Nov 2026, handover of lane PA2's 'not done' row); independent check authors/pg/w/pg_fut_ref.py.
//! Runs on the Norgate T-bond continuous contract only (futures::CARDS_PG, R4 #293 (1)).
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::msws::sma;
use super::tselhs::Book;

pub const REVIEWED: &[&str] = &["scswrl_underlying", "scswrl_reverse"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

macro_rules! fut { () => { " Runs only on the Norgate T-bond continuous contract (the columns test the December T-bond contract of each year): signals on the back-adjusted series, each trade priced in the contract held at entry (R4 #293, #301). Norgate continuous futures, ends 2020-09-11; roll method not stated by the vendor. No costs (Figure 5 nets $100 a trade). From 1999-12 to 2000-10 Norgate's &ZB open sits at the previous close (median gap <= 1 tick, a third of days exactly equal), so open fills in those 11 months are not true opening prices; 0 trades of this card are affected (R4 #331)." } }
const NOTE_U: &str = concat!("[Literal printed rules, 0 unstated details; unverified] V.10:1 Figure 1 (System Writer Plus): diff35 = today's 35-day simple average of the close minus yesterday's, trend35 = the 17-day simple average of diff35. Buy at the close when diff35 > trend35 + .005 and yesterday's diff35 < yesterday's trend35; sell short at the close on the mirror condition. A long is closed at the close when diff35 < trend35 - .005, a short when diff35 > trend35 + .005, and from the day after entry by a .7-point stop from the entry price ('exitlong tomorrow at entryprice - .7 stop'; 'Use a $700 stop', V.10:4 p.184); a day that opens through the stop fills at the open. The stop is touched during the day, before any exit or entry at that day's close. A new signal in the direction already held is ignored (one contract, as System Writer Plus trades it). Book figures (December contract only): V.10:1 'December 1986 Treasury bonds ... the system with stops generated a $6,922 profit'; no trade list.", fut!());
const NOTE_R: &str = concat!("[Adapted: 2 unstated details, R4 #297; unverified] V.10:4 p.185: 'the rule was to reverse at every MAE stop and exit at the end of the underlying trade.' The underlying trade is scswrl_underlying (V.10:1 Figure 1); its MAE stop is the .7-point ($700) stop. When it fills, a position the other way is opened at the stop's fill price and closed at the close of the first day (the stop day included: 'many exited the same day as the close') on which the underlying trade's own exit rule fires (a stopped long: diff35 < trend35 - .005; a stopped short: diff35 > trend35 + .005). Unstated: (1) a stop on the reversed position - vault reading: none ('Risk on the second trade is limited to the O level' describes the expected outcome, not an order); (2) the underlying system keeps trading its own signals while a reversal is open - vault reading: the reversal card holds only the reversal trades, one at a time. #168 against Figure 5 (1986, December contract, $100 a trade): 15 trades, 11 winners, net $14,440 ($963 a trade net, $1,063 gross).", fut!());

fn card(id: &'static str, name: &'static str, dir: &'static str, rules: &'static [&'static str], notes: &'static str) -> StrategyDef {
    StrategyDef { id, name, group: "S&C: Sweeney's T-bond system and reversing at the MAE stop (V.10:1, V.10:4)", session: 0, dir, status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "scswrl", rules, notes, claim: None }
}

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        card("scswrl_underlying", "Sweeney T-bond trend system: 35-day average change vs its 17-day average, at the close, .7-point stop (T-bond futures)", "Long/Short", &[
            "F|\"the underlying bond trading system trades solely off the 35-day simple moving average of closing bond prices in the December contract. The trend is determined by the 17-day average of the day-to-day difference in the moving average. When the one-day difference is greater than the trend, go long. When less, go short. Use a $700 stop.\" (V.10:4, PDF p.1)",
            "E|\"if (diff35 > trend35+.005 and diff35[1] < trend35[1]) then buy today at the close; ... if (diff35 < trend35-.005 and diff35[1] > trend35[1]) then sell today at the close;\" (V.10:1 Figure 1, PDF p.2; \"The [1] refers to yesterday's values\")",
            "X|\"exitlong tomorrow at entryprice - .7 stop; if diff35 < trend35-.005 then exitlong today at the close; ... exitshort tomorrow at entryprice + .7 stop; if diff35 > trend35+.005 then exitshort today at the close;\" (V.10:1 Figure 1)",
            "F|\"The 'diff 35' is the one-day change in the 35-day simple moving average. The 'trend 35' is the 17-day simple moving average of the 'diff 35'.\" (V.10:3 Figure 1 caption)"], NOTE_U),
        card("scswrl_reverse", "Sweeney reversing at the MAE stop: when the T-bond system's .7-point stop fills, take the other side until its trade would have ended (T-bond futures)", "Long/Short", &[
            "E|\"I made a suggestion in the January 1991 issue: reverse the bad trades at the stop-loss point.\" (V.10:4, PDF p.3); \"Recall that the rule was to reverse at every MAE stop and exit at the end of the underlying trade.\" (PDF p.3)",
            "X|\"exit at the end of the underlying trade\" (PDF p.3); \"Only four trades took more than two days and many exited the same day as the close.\" (PDF p.3)",
            "F|Figure 5 'Reversing at MAE stop point' (PDF p.4, 1986): 15 trades, net $14,440 ($100 a trade); 11 winners, 4 losers"], NOTE_R),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id { "scswrl_underlying" => Some(("scswrl_underlying", "Sweeney T-bond diff35 system (S&C)", ".7 stop")), "scswrl_reverse" => Some(("scswrl_reverse", "Sweeney reversing at the MAE stop (S&C)", ".7 stop")), _ => None }
}
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    reviewed(id).then(|| vec![ParamDef { key: "stop", label: "Stop in points from the entry (column .7 = $700)", def: 0.7, min: 0.1, max: 5., step: 0.1 }])
}

/// diff35 and trend35 of V.10:1 Figure 1.
pub fn lines(x: &Ctx) -> (Vec<f64>, Vec<f64>) {
    let s = sma(&x.c, 35);
    let d: Vec<f64> = (0..s.len()).map(|i| if i == 0 { f64::NAN } else { s[i] - s[i - 1] }).collect();
    let tr = sma(&d, 17);
    (d, tr)
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from > to || to >= x.c.len() { return None; }
    let stop = p.get("stop").copied().unwrap_or(0.7);
    let (d, tr) = lines(x);
    let up = |t: usize| d[t] > tr[t] + 0.005;
    let dn = |t: usize| d[t] < tr[t] - 0.005;
    let mut sys = Book::new(); // the underlying system
    let mut rev = Book::new(); // reversal trades (scswrl_reverse)
    let mut rev_end = 0.; // the stopped underlying side whose end closes the reversal
    for t in from..=to {
        let s = sys.side();
        if s != 0. && sys.entry_bar().is_some_and(|e| t > e) {
            let lvl = sys.entry_px() - s * stop;
            if (s > 0. && x.l[t] <= lvl) || (s < 0. && x.h[t] >= lvl) {
                let px = if (s > 0. && x.o[t] <= lvl) || (s < 0. && x.o[t] >= lvl) { x.o[t] } else { lvl };
                sys.exit(t, px, "stop");
                if rev.side() == 0. && t < to { rev.enter(-s, t, px, "stop"); rev_end = s; }
            }
        }
        if rev.side() != 0. && ((rev_end > 0. && dn(t)) || (rev_end < 0. && up(t))) { rev.exit(t, x.c[t], "end of the underlying trade"); }
        let s = sys.side();
        if (s > 0. && dn(t)) || (s < 0. && up(t)) { sys.exit(t, x.c[t], "trend exit at the close"); }
        if t >= 1 && t < to && sys.side() == 0. {
            if up(t) && d[t - 1] < tr[t - 1] { sys.enter(1., t, x.c[t], "close"); }
            else if dn(t) && d[t - 1] > tr[t - 1] { sys.enter(-1., t, x.c[t], "close"); }
        }
    }
    Some(if id == "scswrl_reverse" { rev.done(to, x.c[to]) } else { sys.done(to, x.c[to]) })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stops_reverse_and_close_exits() {
        let r = super::super::sctraub::tests::bars(2500);
        let x = Ctx::new(&r, &r, &[]);
        let u = run("scswrl_underlying", &x, 0, r.len() - 1, &Params::new()).unwrap();
        let v = run("scswrl_reverse", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(u.len() > 5 && u.iter().any(|t| t.reason == "stop"));
        // every reversal opens on the bar and at the price of an underlying stop, on the other side
        for z in &v {
            let s = u.iter().find(|s| s.reason == "stop" && s.exit_i == z.legs[0].i).expect("reversal without a stop");
            assert!(s.dir == -z.dir && (s.exit_px - z.legs[0].px).abs() < 1e-12);
        }
        for t in &u { if t.reason == "stop" { assert!((t.dir > 0. && t.exit_px <= t.legs[0].px - 0.7 + 1e-9) || (t.dir < 0. && t.exit_px >= t.legs[0].px + 0.7 - 1e-9)); } }
    }
}
