//! Gene Carey, "Trading Multiple Time Frames", Technical Analysis of Stocks & Commodities V.13:5 (May 1995) pp.209-215
//! (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V13/C05/TRADING.PDF).
//! Book id `sccarsi`. Lane PA1 (13 Nov 2026, pool A of authors/a1/A1.tsv). The daily RSI system of Figure 3 (printed
//! EasyLanguage); the 60-minute MultRSI version is held. Independent check authors/pa1/pa1_ref.py.
use crate::connors::{rsi, Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::tselhs::Book;

pub const REVIEWED: &[&str] = &["sccarsi_rsi6"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Complete rule (printed EasyLanguage, Figure 3); unverified] RSI(6) of the close (MYRSI 'generates the exact same graph' as TradeStation's RSI: Wilder smoothing). At each close, in the code's order, with the position held before that close: (1) RSI crosses over 25 and not long -> buy on the close (reversing a short), MaxH = the bar's high; (2) RSI crosses under 75 and not short -> sell short on the close (reversing a long), MinL = the bar's low; (3) if long: MaxH = max(MaxH, high); a close below MinL (the lowest low of the previous short) -> sell short on the close and MinL is reset (the code's MinL = Date, a number above any price); (4) if short: MinL = min(MinL, low); a close above MaxH (the highest high of the previous long) -> buy on the close and MaxH = 0. Always in the market after the first signal; no other stop. [Declared reading] The code's Date start value of MaxH (a number above any price, e.g. 941119) is read as +infinity. Article (S&P 500 futures daily 3/10/94-11/23/94, $20 commission and $75 slippage a round turn): results table Figure 4 (an image, not transcribed). The 60-minute MultRSI system (seven RSIs on the bars' end times) is held: its code is not printed. Runs on indices, ETFs and stocks, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "sccarsi_rsi6", name: "Carey RSI(6) 25/75 stop and reverse, with a close beyond the last opposite swing extreme also reversing", group: "S&C: Trading Multiple Time Frames (Carey, V.13:5)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "sccarsi", rules: &[
            "E|\"If MYRSI(P,Len) crosses over BZ and MarketPosition <> 1 then Begin Buy on Close; MaxH = H; End; If MYRSI(P,Len) crosses under SZ and MarketPosition <> -1 then Begin Sell on Close; MinL = L; End;\" Inputs P(C), Len(6), BZ(25), SZ(75) (V.13:5 Figure 3)",
            "X|\"If MarketPosition = 1 then Begin If H > MaxH then MaxH = H; If C < MinL then Begin Sell on Close; MinL = Date; End; End; If MarketPosition = -1 then Begin If L < MinL then MinL = L; If C > MaxH then Begin Buy on Close; MaxH = 0; End; End;\" (Figure 3)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    reviewed(id).then_some(("sccarsi_rsi6", "Carey RSI swing reversal (S&C)", "RSI 6, 25/75"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("len", "RSI length (article 6)", 6., 2., 30., 1.), p("bz", "Buy zone (article 25)", 25., 5., 50., 1.), p("sz", "Sell zone (article 75)", 75., 50., 95., 1.)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d);
    let (n, bz, sz) = (g("len", 6.).round().max(1.) as usize, g("bz", 25.), g("sz", 75.));
    let r = rsi(&x.c, n);
    let mut pos = Book::new();
    let (mut max_h, mut min_l) = (f64::INFINITY, 0.);
    for t in from.max(1)..=to {
        let mp = pos.side();
        let mut want = 0.;
        if r[t] > bz && r[t - 1] <= bz && mp != 1. { want = 1.; max_h = x.h[t]; }
        if r[t] < sz && r[t - 1] >= sz && mp != -1. { want = -1.; min_l = x.l[t]; }
        if mp == 1. {
            if x.h[t] > max_h { max_h = x.h[t]; }
            if x.c[t] < min_l { want = -1.; min_l = f64::INFINITY; }
        }
        if mp == -1. {
            if x.l[t] < min_l { min_l = x.l[t]; }
            if x.c[t] > max_h { want = 1.; max_h = 0.; }
        }
        if want != 0. { pos.enter(want, t, x.c[t], "close"); }
    }
    Some(pos.done(to, x.c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rsi_swing_reversal_always_in() {
        let r = super::super::sctraub::tests::bars(3000);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("sccarsi_rsi6", &x, 0, r.len() - 1, &Params::new()).unwrap();
        let rs = rsi(&x.c, 6);
        assert!(t.len() > 10 && t.iter().any(|z| z.dir < 0.));
        for w in t.windows(2) { assert_eq!(w[1].legs[0].i, w[0].exit_i); assert_eq!(w[1].dir, -w[0].dir); }
        // the first trade starts on an RSI zone cross (no stored extreme exists yet)
        let e = t[0].legs[0].i;
        assert!(if t[0].dir > 0. { rs[e] > 25. && rs[e - 1] <= 25. } else { rs[e] < 75. && rs[e - 1] >= 75. });
    }
}

// Instrument admission: S&C Carey RSI system 2026-11-13 (lane PA1).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "sccarsi" && reviewed(def.id),
    label: "Gene Carey showed the daily RSI system on S&P 500 futures (1994); indices, ETFs and stocks, US and India, are a declared application (no fixed amounts).",
    basis: "Technical Analysis of Stocks & Commodities V.13:5 pp.209-215 (Trading Multiple Time Frames), Figure 3; independent check authors/pa1/pa1_ref.py",
};
