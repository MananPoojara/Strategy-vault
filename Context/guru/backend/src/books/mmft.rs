//! Mark Boucher & Larry Connors, "Market & Mutual Fund Timing Trading Course" (TradingMarkets.com online course,
//! Nov-Dec 1999; Introduction + Weeks 1-4 on the research drive). Book id "mmft". Library notes: library/mmft/.
//! Weeks 1-2 (CVR I-VI, CHADTP, TRIN Thrusts) are already in the vault from other Connors books (see the README);
//! this module holds the course's new material: Boucher's Week 3-4 timing models.
use crate::connors::{claim, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "mmft";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, status, rules, notes, claim| StrategyDef {
        id, name, group: "Week 3-4 · Boucher timing models", session: 0, dir: "Long/Short", status,
        needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim,
    };
    vec![
        d("mmft_nasdaq79", "Nasdaq 7.9% momentum model (Model #1)", Status::Live, &[
            "E|\"goes long the Nasdaq index whenever that index closes up 7.9% or more from a trough close value. Similarly, it simply goes short whenever the Nasdaq closes down 7.9% or more from a peak value close.\" (Week 3, Model #1)",
            "E|\"if we start the index at 100 we'll go long on the first close over 107.9 or go short on the first close below 92.1, whichever happens first.\"",
            "X|\"If we go short first, then we'll exit and reverse to a long trade on any close that is up 7.9% or more from the lowest close since we initiated our short position.\"",
            "X|\"If we go long first, then we'll exit and go short on any close that is down 7.9% or more from the highest close since we initiated our long position. It's a very simple stop-and-reverse system\"",
        ], "Always in the market after the first signal; trades at the signal day's close (the rule is on closes; the course gives no other fill) [Book silent: fill at the close]. \
The start value is the first close of the selected window (the course's \"start the index at 100\" example), so results depend on the start date like the book's. \
Book: Nasdaq Composite (OTC) 8/23/63-10/27/97, no costs stated. The vault has no Nasdaq Composite history; run it on NDX (1990-) / SPX / DJI, or NIFTY 50 for India (instrument swap only, 7.9% kept). \
The 7.9% is the midpoint of the 7.6%-8.2% range the course's own optimisation found (Week 3, Steps 2-3); param pct varies it (course range 4-12% in 0.1 steps).",
          claim("-", "61% of trades", "-", "-", "Nasdaq Composite 8/23/63-10/27/97: CAR 18.2%, max DD 22.4%, 91% profitable years (buy-and-hold 11.8%, 59.5%, 70%) - Week 3 Table 1")),
        d("mmft_bond_roc", "Dow Jones 20 Bond Index rate-of-change (Model #2)",
          Status::NeedsData("Dow Jones 20 Bond Index monthly closes (1943-) - not in the vault; no Indian equivalent named by the course"), &[
            "F|\"whenever bond prices (measured by the Dow Jones 20 Bond Index) are not eroding fairly rapidly (that is, either rising or dropping at less than 1.5% over the latest year), stocks tend to move up\" (Week 3, Table 2: ROC > -1.5% vs ROC <= -1.5%)",
            "F|\"-1.5% DJ20 bond index monthly close annual return system\" (Week 3, Table 3 text)",
        ], "Table 2 (1/29/43-12/31/97): ROC > -1.5%: 52% of days invested, 17.4% annual rate; ROC <= -1.5%: 40%, -0.3%. The course does not say which stock index Table 2 holds.", None),
        d("mmft_nasdaq79_bond", "Combination: 7.9% model + DJ20 bond ROC (Models #1 + #2)",
          Status::NeedsData("Dow Jones 20 Bond Index monthly closes (see mmft_bond_roc)"), &[
            "E|\"we're only going to buy the index when both systems would be long, and we're only going to short the Nasdaq index when both systems would be short. When the systems disagree, we're going to park in T-bill money market funds.\" (Week 3)",
        ], "Week 3 Table 3, 1/29/43-10/27/97: max DD 13.25%, CAR 17.28%, 94% profitable years, 75% profitable trades. The bond model has no 'short' state defined beyond ROC <= -1.5% (read as bearish).", None),
        d("mmft_three_oscillators", "Boucher's three overbought-oversold oscillators (McClellan, TRIN, put-call premium)",
          Status::NotTestable("no exit rule (\"trailing stops\" and \"half profits\" are not specified), needs the S&P put-call premium ratio (no history in the vault) and first-hour highs/lows on gap days"), &[
            "F|McClellan Oscillator below -70 = oversold, above +70 = overbought; TRIN > 1.5 oversold, < 0.5 overbought; put-call premium ratio < 0.5 oversold, > 1.5 overbought (Week 2)",
            "E|Two or more oscillators oversold within three days, a low day within three days of them, then the high of the low day broken; a gap above it waits for the first-hour high (Week 2)",
            "X|\"Don't take more than one repeat signal ... until you have a locked-in profit ... via trailing stops\"; \"Take half profits as soon as any one of the above three oscillators registers an overbought\"",
        ], "Week 2: 39 of 48 two-oscillator signals (81%) and all 18 three-oscillator signals followed through (S&P, six years to 1999; judged on charts).", None),
        d("mmft_tpts", "TradingMarkets Proprietary Timing System (TPTS)",
          Status::NotTestable("the four component models are not disclosed (\"its exact formula will not be revealed\", Week 4)"), &[
            "E|Buy when 2 or more of the 4 models are bullish; exit long at 1 or fewer; short at 0, cover when any becomes bullish; 150% long at 4 until 3 or fewer (Week 4)",
        ], "Week 4 Table 1, OTC index 1978-1999: average annual 34% vs 20.54%, compound 32%, max DD 10% after trade, 20.60% within a profitable trade.", None),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let f = |v| Some(("mmft_boucher", "Boucher timing models (Market & Mutual Fund Timing)", v));
    match id {
        "mmft_nasdaq79" => f("7.9% momentum stop-and-reverse"),
        "mmft_bond_roc" => f("DJ20 bond ROC · needs data"),
        "mmft_nasdaq79_bond" => f("7.9% + bond combination · needs data"),
        "mmft_three_oscillators" => f("Three oscillators · not testable"),
        "mmft_tpts" => f("TPTS · not disclosed"),
        _ => None,
    }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    (id == "mmft_nasdaq79").then(|| vec![ParamDef { key: "pct", label: "Reversal % from the trough / peak close", def: 7.9, min: 4.0, max: 12.0, step: 0.1 }])
}

/// Stop-and-reverse on closes: long after a close >= trough x (1 + pct), short after a close <= peak x (1 - pct).
pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if id != "mmft_nasdaq79" { return None; }
    let k = p.get("pct").copied().unwrap_or(7.9) / 100.0;
    let c = &x.c;
    let to = to.min(c.len() - 1);
    let mut out: Vec<Trade> = vec![];
    let (mut dir, mut ext) = (0.0, c[from]); // flat: ext = the start value
    let open = |dir: f64, i: usize| Trade { dir, legs: vec![Leg { i, px: c[i], w: 1.0 }], exit_i: i, exit_px: f64::NAN,
        reason: String::new(), entry_kind: "close", signal_i: i, open: false, exits: vec![] };
    for i in from + 1..=to {
        let flip = if dir > 0.0 { c[i] <= ext * (1.0 - k) } else if dir < 0.0 { c[i] >= ext * (1.0 + k) } else { false };
        let start = if dir == 0.0 { if c[i] >= ext * (1.0 + k) { 1.0 } else if c[i] <= ext * (1.0 - k) { -1.0 } else { 0.0 } } else { 0.0 };
        if flip {
            let t = out.last_mut().unwrap();
            (t.exit_i, t.exit_px, t.signal_i) = (i, c[i], i);
            t.reason = format!("Close {}% from the {} close: stop and reverse", k * 100.0, if dir > 0.0 { "highest" } else { "lowest" });
            dir = -dir;
        } else if start != 0.0 {
            dir = start;
        } else {
            if dir > 0.0 { ext = ext.max(c[i]) } else if dir < 0.0 { ext = ext.min(c[i]) }
            continue;
        }
        out.push(open(dir, i));
        ext = c[i];
    }
    if let Some(t) = out.last_mut().filter(|t| t.exit_px.is_nan()) {
        (t.exit_i, t.exit_px, t.reason, t.signal_i, t.open) = (to, c[to], "open".into(), to, true);
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn hand_checked_stop_and_reverse() {
        let cl = [100.0, 104.0, 108.0, 112.0, 103.0, 101.0, 109.5];
        let r: Vec<Row> = cl.iter().enumerate().map(|(i, &c)| Row { ticker: "T".into(), date: format!("d{i}"), open: c, high: c, low: c, close: c, volume: None }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let t = run("mmft_nasdaq79", &x, 0, 6, &Params::new()).unwrap();
        // long at 108 (>= 107.9), peak 112, short at 103 (<= 112 x 0.921 = 103.152), trough 101, long at 109.5 (>= 108.979)
        let got: Vec<(f64, usize, usize, bool)> = t.iter().map(|t| (t.dir, t.legs[0].i, t.exit_i, t.open)).collect();
        assert_eq!(got, vec![(1.0, 2, 4, false), (-1.0, 4, 6, false), (1.0, 6, 6, true)]);
    }
}
