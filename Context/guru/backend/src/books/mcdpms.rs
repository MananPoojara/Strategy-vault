//! Michael McDonald, "Predict Market Swings With Technical Analysis" (Wiley, 2002; book id `mcdpms`). Reading:
//! claude-continuation-20261020/authors/lanec3 (text layer; Ch.8 "Trading Range Investment Strategies" read in full, the rest by
//! contents + keyword scan); independent reference authors/lanec3/ref/lanec3_ref.py. Admitted: the moving-average timing rule
//! of the 70-year study (p.173-178) and the Bollinger-band Strategy 2 of the 1966-1982 study (p.184-188). The 60/40 and 50/50
//! mixes of these strategies are portfolio splits, not separate rules; T-bill interest while out and dividends are not modelled.
use crate::books::lc3::{sma, Pos};
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const REVIEWED: &[&str] = &["mcdpms_ma", "mcdpms_bb"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        StrategyDef { id: "mcdpms_ma", name: "Moving-average timing: in while the close is above 98.5% of the 130-day average, out below; trade at the next day's close", group: "Moving-average studies (Ch.8)",
            session: 0, dir: "Long", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "mcdpms",
            rules: &["E|\"When a moving average was positive, meaning that the closing price of the S&P 500 was above the moving average, that account was assumed to be invested in the S&P 500 ... When the closing price of the S&P 500 again moved above the corresponding moving average, the computer moved the account back into the S&P 500\" (p.174)",
                "X|\"When the S&P 500 closed below the moving average, the computer sold the S&P 500 in that account and the funds went into a T-bill account\" (p.174)",
                "F|\"The price you buy or sell at is always the next day's price after a signal is given ... the next day's closing price will apply\" (p.176); \"I obtained the best results by shifting all the moving averages downward by 1.5% ... whether the S&P 500 was higher or lower than 98.5% of the moving average\" (p.177); \"I chose the 130-day moving average (downshifted 1.5%)\" (Figure 8.6, p.178)",
                "A|1966-1982 study (Strategy 1, p.185-188): 72- or 132-day average raised by 1% (p.shift = 1), traded at the close of the signal day (p.delay = 0), 35 basis points round trip (not modelled)"],
            notes: "Simple average of daily closes. A close exactly on the line changes nothing. The account is in or out by the last close that was clearly above or below the line; if the window opens while it is 'in', the first purchase is at the first fill day in the window. Book: S&P 500 1929-1998 with dividends and T-bill interest while out, mutual-fund free exchange, no costs: 12.5% a year vs 10.3% buy and hold (p.177). Here: price only, flat (no interest) while out, one unit, no costs.",
            claim: None },
        StrategyDef { id: "mcdpms_bb", name: "Trading-range Bollinger strategy: moving-average timing, but sell above the upper 2-sd band and buy below the lower band", group: "Moving-average studies (Ch.8)",
            session: 0, dir: "Long", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "mcdpms",
            rules: &["F|\"Strategy 2: Test Bollinger bands that are twice the standard deviation away from their moving average\" (p.185); averages \"raised upward on the price scale by 1%\"; \"the 72-day and the 132-day\" (p.187-188)",
                "X|\"when the S&P 500 went above the top Bollinger band, the S&P 500 was sold and the money put into a T-bill account. It stayed there until prices either went below and then above the moving average or until the lower Bollinger band was penetrated. At these times, the money was again invested\" (p.185)",
                "E|\"when the S&P 500 went below the lower Bollinger band, the S&P 500 was purchased. It stayed invested in the S&P 500 until prices either went above and then below the moving average or until the upper Bollinger band was penetrated\" (p.185-186); otherwise it \"followed the moving-average strategy\" (p.185)",
                "F|\"the price of buying or selling the S&P 500 would be the closing price for the same day\"; \"a round-trip expense ... of 35 basis points\" (p.186)"],
            notes: "[Declared readings] Bands = simple n-day average of closes +/- 2 population standard deviations of the same closes; the moving-average buy/sell line is that average x (1 + shift%) (book: +1%). Band touches and line crossings are judged on the close and filled at that close. After a sale above the upper band the account stays out until a close below the line followed later by a close above it (or a close below the lower band); after a purchase below the lower band it stays in until a close above the line followed later by a close below it (or a close above the upper band); then plain moving-average timing resumes. A close beyond a band always takes priority (a close above the upper band is a sale even right after a dip below the line). The book ran this on 40% of the money and Strategy 1 (card mcdpms_ma with p.n 132, p.shift 1, p.delay 0) on 60%, and combined the 72- and 132-day results 50:50 (1966-1982: 10.3% a year vs 5.06% buy and hold, Table 8.1) - those are portfolio splits; this card is Strategy 2 alone, one unit, no costs, flat while out.",
            claim: None },
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id {
        "mcdpms_ma" => Some(("mcdpms_ma", "McDonald moving-average studies", "Moving-average timing")),
        "mcdpms_bb" => Some(("mcdpms_bb", "McDonald moving-average studies", "Bollinger trading-range strategy")),
        _ => None,
    }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let n = |d: f64| ParamDef { key: "n", label: "Moving-average days (70-year study: 130; 1966-1982 study: 72 or 132)", def: d, min: 5., max: 200., step: 1. };
    let s = |d: f64| ParamDef { key: "shift", label: "Shift the average by this % (70-year study: -1.5; 1966-1982 study: +1)", def: d, min: -5., max: 5., step: 0.5 };
    match id {
        "mcdpms_ma" => Some(vec![n(130.), s(-1.5), ParamDef { key: "delay", label: "Trade at the close 1 day after the signal (book, 70-year study) or 0 = the signal day's close (1966-1982 study)", def: 1., min: 0., max: 1., step: 1. }]),
        "mcdpms_bb" => Some(vec![n(132.), s(1.)]),
        _ => None,
    }
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d);
    let bb = id == "mcdpms_bb";
    let n = g("n", if bb { 132. } else { 130. }).round().max(2.) as usize;
    let k = 1. + g("shift", if bb { 1. } else { -1.5 }) / 100.;
    let delay = if bb { 0 } else { g("delay", 1.).round().clamp(0., 1.) as usize };
    let c = &x.c;
    let m = sma(&c[..=to], n);
    // desired state (true = invested) at each close
    let mut want = vec![false; to + 1];
    let (mut inn, mut mode, mut flag) = (false, 0i8, false); // mode 0 = plain timing, 1 = out after the upper band, -1 = in after the lower band
    for i in 0..=to {
        if m[i].is_finite() {
            let line = m[i] * k;
            if bb {
                let w = &c[i + 1 - n..=i];
                let sd = (w.iter().map(|v| (v - m[i]).powi(2)).sum::<f64>() / n as f64).sqrt();
                let (up, lo) = (m[i] + 2. * sd, m[i] - 2. * sd);
                if c[i] > up { inn = false; mode = 1; flag = false; }
                else if c[i] < lo { inn = true; mode = -1; flag = false; }
                else if mode == 1 { if c[i] < line { flag = true; } else if flag && c[i] > line { inn = true; mode = 0; } }
                else if mode == -1 { if c[i] > line { flag = true; } else if flag && c[i] < line { inn = false; mode = 0; } }
                else if c[i] > line { inn = true; } else if c[i] < line { inn = false; }
            } else if c[i] > line { inn = true; } else if c[i] < line { inn = false; }
        }
        want[i] = inn;
    }
    let mut pos = Pos::new();
    for i in from.max(delay)..=to {
        let w = want[i - delay];
        if w && pos.side() == 0. { pos.enter(1., i, c[i], "close"); }
        else if !w && pos.side() > 0. { pos.exit(i, c[i], if bb { "band / moving-average sell" } else { "close below the shifted average" }); }
    }
    Some(pos.done(to, c[to]))
}

#[cfg(test)]
mod tests {
    #[test]
    fn ids_reviewed() { assert!(super::reviewed("mcdpms_ma") && super::reviewed("mcdpms_bb") && !super::reviewed("mcdpms_x")); }
}
