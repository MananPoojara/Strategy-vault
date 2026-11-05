//! Rob Keener, "Probability vs. profitability", Active Trader June 2000 pp.33-34 (research share Trading Books/Magazine - Active
//! Trader Magazine (activetradermag.com)/June 2000 Issue.pdf): the "Bigfoot" entry with the bail-out exit (Bigfoot 1) and with
//! fixed-dollar stops on 100 SPY shares (Bigfoot 2 and 3). Book id `atbig`. Lane K41 (batch T7a, 4 Nov 2026); independent check
//! authors/titles-1030/K41w/k41_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::tselhs::Book;

pub const REVIEWED: &[&str] = &["atbig_bailout", "atbig_stop"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] Entry: if today's open is below yesterday's close, a buy stop at yesterday's close for today; if the open is above it, a sell stop there (an open equal to yesterday's close gives no setup). Bail-out exit: a long is sold at the first later open above its entry price (a short is covered at the first later open below it); a setup in the opposite direction reverses the position at its stop ('stop and reverse'); a setup in the same direction is ignored. Each day the open is checked first (bail-out), then the stops; the bail-out 'exits the market' for that day, so no new entry is taken on the day of a bail-out (this reading gives 65% of days in the market on SPY 1998-1999 against the article's 67.41%; re-entering on the same day gives 87%). The test traded 100 SPY shares for two years (dates not printed), no costs. Bigfoot 4-6 (the 'dynamic profit target') are not coded: the target formula is not printed. Runs on indices, ETFs and stocks, US and India, are a declared application.";
const NOTE_STOP: &str = "[Declared readings] Bigfoot 2 and 3: the same entry and bail-out exit plus a fixed-dollar stop, $1,000 (default) or $2,000 (option usd) on 100 SPY shares = $10 or $20 a share below the entry (above it for a short), active from the day after entry, filled at the stop or at an open beyond it. When the stop and an opposite setup's stop are both hit on one day, the long exits at the higher of the two levels (the first reached) and the short is entered at yesterday's close. SPY has never split, so the dollar amount is the article's own on the vault's SPY prices; this card runs on SPY only (a fixed amount means nothing else on another instrument, decision 166 and 205). Article (SPY, two years, 100 shares, no costs): Bigfoot 2 ($1,000 stop) 146 trades, 78.77% winners, +$1,368.70, drawdown $1,000; Bigfoot 3 ($2,000 stop) 149 trades, 77.85%, +$1,060.00.";

pub fn catalogue() -> Vec<StrategyDef> {
    const R: [&str; 3] = [
        "E|\"If today's open is lower than yesterday's close, buy when today's price crosses above yesterday's close. The rules are reversed for sell signals.\" (Active Trader June 2000, p.33)",
        "X|\"The first risk-management approach, referred to as a 'bail-out' strategy, exits the market on the next profitable open or reverses if a signal is generated in the opposite direction of the current position ('stop and reverse').\" (p.34)",
        "F|Table 1: \"Bigfoot #1 (No stop) 78.47 $1,671.80 $1,500.00 144 67.41\"; \"The test spanned two years, using daily price data for the ... SPY ... The trade size was 100 shares. No deductions were made for slippage or commissions.\" (p.34)"];
    const S: [&str; 4] = [R[0], R[1],
        "X|\"The second and third approaches combine the bail-out strategy with fixed-dollar stops of $1,000 and $2,000, respectively.\" (p.34)",
        "F|Table 1: \"Bigfoot #2 ($1,000 stop) 78.77 $1,368.70 $1,000.00 146 66.99\"; \"Bigfoot #3 ($2,000 stop) 77.85 $1,060.00 $1,500.00 149 67.82\" (p.34)"];
    let d = |id, name, rules, notes| StrategyDef { id, name, group: "Active Trader: Bigfoot (Keener, June 2000)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "atbig", rules, notes, claim: None };
    vec![
        d("atbig_bailout", "Bigfoot: open below yesterday's close, buy back above it (short mirrored); exit on the next profitable open or reverse", &R, NOTE),
        d("atbig_stop", "Bigfoot with a $1,000 / $2,000 stop on 100 SPY shares (SPY only)", &S, NOTE_STOP),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let v = match id { "atbig_bailout" => "Bail-out, no stop (Bigfoot 1)", "atbig_stop" => "Bail-out and dollar stop (Bigfoot 2/3)", _ => return None };
    Some(("atbig", "Bigfoot (Active Trader)", v))
}
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    match id {
        "atbig_bailout" => Some(vec![]),
        "atbig_stop" => Some(vec![ParamDef { key: "usd", label: "Stop in dollars on 100 SPY shares (1000 = Bigfoot 2, 2000 = Bigfoot 3)", def: 1000., min: 1000., max: 2000., step: 1000. }]),
        _ => None,
    }
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let st = if id == "atbig_stop" { p.get("usd").copied().unwrap_or(1000.) / 100.0 } else { f64::INFINITY };
    let mut pos = Book::new();
    for t in from.max(1)..=to {
        let (o, h, l, y) = (x.o[t], x.h[t], x.l[t], x.c[t - 1]);
        let side = pos.side();
        let e = pos.entry_bar().unwrap_or(t);
        // bail-out at a profitable open
        if side != 0.0 && e < t && side * (o - pos.entry_px()) > 0.0 { pos.exit(t, o, "Profitable open"); continue; }
        let side = pos.side();
        let ep = pos.entry_px();
        if side > 0.0 && e < t && l <= ep - st {
            let lv = ep - st;
            let opp = o > y && l <= y;
            pos.exit(t, if o <= lv { o } else if opp { lv.max(y) } else { lv }, "Dollar stop");
            if opp { pos.enter(-1.0, t, y, "stop"); }
            continue;
        }
        if side < 0.0 && e < t && h >= ep + st {
            let lv = ep + st;
            let opp = o < y && h >= y;
            pos.exit(t, if o >= lv { o } else if opp { lv.min(y) } else { lv }, "Dollar stop");
            if opp { pos.enter(1.0, t, y, "stop"); }
            continue;
        }
        if o < y && h >= y && side <= 0.0 { pos.enter(1.0, t, y, "stop"); }
        else if o > y && l <= y && side >= 0.0 { pos.enter(-1.0, t, y, "stop"); }
    }
    Some(pos.done(to, x.c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bigfoot_entries_at_yesterdays_close() {
        let r = super::super::sctraub::tests::bars(1500);
        let x = Ctx::new(&r, &r, &[]);
        for id in REVIEWED {
            let t = run(id, &x, 0, r.len() - 1, &Params::new()).unwrap();
            assert!(t.len() > 5 && t.iter().all(|z| (z.legs[0].px - x.c[z.legs[0].i - 1]).abs() < 1e-9), "{id}");
        }
    }
}
