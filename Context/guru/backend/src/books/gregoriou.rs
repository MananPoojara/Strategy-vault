//! Greg N. Gregoriou (editor) - three edited volumes with complete price-only rules. Book ids:
//!   `ggth`  The Handbook of Trading (McGraw-Hill 2010; printed page = pdf page - 35): Ch.7 Kenourgios & Papathanasiou
//!           (1,n) moving-average rule, Ch.11 Lento dual-moving-average crossover, traditional and exponentially smoothed.
//!   `ggeai` Encyclopedia of Alternative Investments (CRC 2009), "Systematic CTA" (K. H. Black, p.468): 5/30 crossover.
//!   `gghf`  Hedge Funds: Insights in Performance Measurement, Risk Analysis, and Portfolio Allocation (Gregoriou, Hübner, Papageorgiou, Rouah, eds., Wiley 2005), Ch.10
//!           p.230: the Calyon 80-day range breakout and 80-day moving-average indexes (Burghardt, Duncan and Liu).
//! Reading: claude-continuation-20261020/authors/gregoriou/agents/. The other chapters are academic or performance
//! studies, or need futures, fund, credit or constituent data (authors/gregoriou/gregoriou-REPORT.md).
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};
use crate::books::mpts::sma;

pub const REVIEWED: &[&str] = &["ggth_sma_ls", "ggth_dmaco", "ggth_dmaco_es", "ggeai_ma5_30", "gghf_rb80", "gghf_ma80", "gghf_mom15", "ggfohf_ma"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

macro_rules! note { ($s:expr) => { concat!($s, "[Declared conventions, not from the book] Signals are read on the daily close and filled at that close; the indicators are built from the start of the data file and a run starts flat, taking the first signal inside the window. An average equal to the other (or a close equal to the old high/low) is no signal: the position is kept. No commissions, bid-ask spread or cash interest.") }; }

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    let live = |id, name, group, book, dir, rules, notes| StrategyDef { id, name, group, session: 0, dir, status: Live, needs_volume: false,
        needs_vix: false, needs_series: &[], book, rules, notes, claim: None };
    let held = |id, name, group, book, st, rules| StrategyDef { id, name, group, session: 0, dir: "Long/Short", status: st, needs_volume: false,
        needs_vix: false, needs_series: &[], book, rules, notes: "Held after the 2026-10-24 reading (authors/gregoriou/agents).", claim: None };
    vec![
        live("ggth_sma_ls", "Price vs n-day moving average, long above / short below (FTSE/ASE 20 study)", "Ch.7 · Kenourgios & Papathanasiou (p.100-105)", "ggth", "Long/Short",
            &["F|\"Moving averages give buy signal when the short-term moving average crossover the long-term one. On the other side, we have a sell signal when the long-term moving average crossover the short-term one\" (p.100)",
              "F|Rules tested: \"(1,9), (1,15), (1,30), (1,60), (1,90), (1,120), and (1,150). The first number in each pair indicates the days in the short period and the second number shows the days in the long period\" (p.103); daily closes of the FTSE/ASE 20 index",
              "E|Buy = long, sell = short (Table 7.2 columns \"Buy (Long Strategy)\", \"Sell (Short Strategy)\", p.105)",
              "X|Reverse on the opposite crossover"],
            note!("Simple averages of daily closes. Default (1,9), the first rule listed; the book tests all seven equally and recommends none, so p.n = 15, 30, 60, 90, 120 or 150 runs the others. The study charges 0.2% per entry and exit (p.100), which this ledger does not deduct. The book's figures are mean daily returns, not trade lists. Cash-index shorts are hypothetical. ")),
        live("ggth_dmaco", "Traditional dual-moving-average crossover (VMA), long or cash", "Ch.11 · Lento (p.157-160)", "ggth", "Long",
            &["F|\"The VMA generates a buy (sell) signal whenever the STMA is above (below) the LTMA. After a buy (sell) signal, the investor will be long (out of) the market until a sell (buy) signal is generated\" (p.157)",
              "F|\"The following STMA,LTMA combinations will be used: (1,50), (10,50), and (1,200)\" (p.160); traditional = averages of the raw prices",
              "E|Long while the STMA is above the LTMA (VMA, p.157)",
              "X|\"This test does not rely on short sales. Rather, after a sell signal ... the capital will be placed in savings account\" (p.160)"],
            note!("Simple averages of daily closes. Default (1,50), the first pair listed; p.s = 10 / p.l = 200 give the other pairs. Equations 11.1-11.2 write the long average with a t-1 subscript, but the text defines both as averages over their own day counts; both are read on the same day. Lento's 3% savings-account interest, spread and brokerage are not modelled, so his Table 11.1 returns are not claimed. Tested by the book on the S&P 500, NASDAQ and DJIA, 1999-2009. ")),
        live("ggth_dmaco_es", "Exponentially smoothed dual-moving-average crossover (alpha 0.1), long or cash", "Ch.11 · Lento (p.159-164)", "ggth", "Long",
            &["F|\"The smoothing scheme ... begins by defining S2 to y1 ... St = alpha y(t-1) + (1 - alpha) S(t-1), t >= 3\" (Eq.11.3, p.159); \"this study utilizes an alpha of 10 percent\" (p.159-160)",
              "F|\"The STMA and LTMA will be calculated on the exponentially smoothed time-series\"; pairs (1,50), (10,50), (1,200) (p.160)",
              "E|Long while the STMA is above the LTMA (VMA, p.157)",
              "X|Out (savings account, no short sales) while the STMA is below the LTMA (p.157, 160)"],
            note!("The smoothed series uses only earlier closes (S of today = 0.1 x yesterday's close + 0.9 x yesterday's S), started at the second bar of the data file with S = the first close; both averages are simple averages of S. Default (1,50); p.s, p.l and p.alpha change the pair and the smoothing (Figure 11.1 shows alpha 0.1-1.0 and a (10,20) pair that the text does not list). Interest, spread and brokerage are not modelled. ")),
        held("ggth_momentum", "Cross-sectional stock momentum (top five long, bottom five short)", "Ch.3-4 (p.39-52, 55-56)", "ggth",
            NeedsData("ranks every index constituent each month and holds overlapping long/short portfolios; this needs historical index membership and a ranking portfolio, while the site's stock baskets replay one rule per stock on a current member list"),
            &["E|\"buy the top five and short the bottom five\" after a one-month skip (Ch.3)"]),
        held("ggth_futures_mom", "Financial futures momentum (best six long, worst six short)", "Ch.5 (p.69-71)", "ggth",
            NeedsData("needs 18 rolled financial futures contracts; cash indices cannot stand in for futures prices"), &["E|Rank 18 financial futures by past returns"]),
        held("ggth_ewma_portfolio", "Monthly EWMA crossover stock portfolios", "Ch.8 Pavlov & Hurn (p.116-124)", "ggth",
            NotTestable("the study drops stocks that will delist in the next month (p.118), information an investor did not have, and holds monthly long/short stock portfolios"), &["E|Equal-weight buy-signal stocks long, sell-signal stocks short, one month"]),
        held("ggth_fx_templates", "Bull-flag and double-bottom template matching (FX)", "Ch.9 Repkine (p.131-135)", "ggth",
            NotTestable("the fit threshold is taken from the best fit over the whole sample and the stop/target pair from a search of a million combinations in FX points, so no single rule a trader could have used is given"), &["E|Fit a 10x10 template to 40 closes"]),
        held("ggth_volume_premium", "High-volume return premium portfolios", "Ch.23 (p.343-348)", "ggth",
            NeedsData("needs market capitalisation groups and earnings-announcement dates, and the text does not say when positions close"), &["E|Long high-volume, short low-volume stocks within size groups"]),
        held("ggth_logperiodic", "Log-periodic anti-bubble forecast trading", "Ch.25 Fantazzini (p.381-387)", "ggth",
            NotTestable("follows the sign of a fitted log-periodic model whose non-linear calibration (starting values, optimiser) is not specified closely enough to reproduce"), &["E|Long/short from the model forecast slope"]),
        held("ggth_macd_tool", "MACD buy/sell columns of the commodity risk tool", "Ch.21 (p.311-313)", "ggth",
            NotTestable("the VBA tool marks buy and sell points from 12/26-day simple averages and a 9-day signal line but does not say what makes a signal; the code is only available from the authors"), &["E|\"Generate a column indicating buy signals\" (Fig 21.7)"]),
        live("ggeai_ma5_30", "5-day / 30-day moving-average crossover, long and short (Systematic CTA example)", "Systematic CTA (K. H. Black, p.468)", "ggeai", "Long/Short",
            &["E|\"the trader takes a long position when a short-term (perhaps 5 day) moving average crosses above a long-term (perhaps 30 day) moving average\" (p.468)",
              "X|\"A short position is initiated when the short moving average crosses below the long-term moving average\" (p.468)"],
            note!("An encyclopedia example of a simple trend-following CTA rule, not a tested programme: the lengths are 'perhaps' 5 and 30 days and the average type is not named, so simple averages of daily closes are a declared reading; p.s / p.l change the lengths. The entry describes futures CTAs; cash-index shorts are hypothetical. ")),
        held("ggeai_mom_stack", "Stacked commodity trend portfolios mom(3,6,12)", "Commodity entries (p.484-485)", "ggeai",
            NeedsData("needs a portfolio of commodity futures"), &["E|Hold three overlapping monthly portfolios"]),
        held("ggeai_pairs", "Pairs trading and volatility pumping", "Pairs trading (p.351), statistical arbitrage (p.449)", "ggeai",
            NotTestable("no lookback, entry threshold, exit or universe is given"), &["E|Long the undervalued, short the overvalued stock"]),
        live("gghf_rb80", "80-day range breakout, always long or short (Calyon index rule)", "Ch.10 (p.230)", "gghf", "Long/Short",
            &["E|\"the 80-day range breakout, goes long if the price goes over the 80-day high\" (p.230)",
              "X|\"and goes short if the price drops below the 80-day low. These are the only conditions allowing a change in position\" (p.230)"],
            note!("Breakouts are read on closing prices against the highest (lowest) close of the previous 80 days [declared reading: the Calyon indexes are built from daily futures prices and the text says only 'the price']. p.n changes the 80 days. The chapter studies index return profiles (Table 10.12) and prints no trades. Applied to futures in 40 markets by Burghardt, Duncan and Liu (2004); cash-index shorts are hypothetical. ")),
        live("gghf_ma80", "80-day moving average, always long or short (Calyon index rule)", "Ch.10 (p.230)", "gghf", "Long/Short",
            &["E|\"an 80-day moving average strategy. This strategy goes long when the price rises above the 80-day moving average\" (p.230)",
              "X|\"and goes short when the price goes below the average\" (p.230)"],
            note!("Simple 80-day average of daily closes [declared reading: the average type is not named]; p.n changes the length. Cash-index shorts are hypothetical. ")),
        live("gghf_mom15", "15-day momentum, long if above the price 15 days ago, otherwise short (Spurgin, Schneeweis and Georgiev benchmark)", "Ch.23 Till & Eagleeye (p.482)", "gghf", "Long/Short",
            &["E|\"if a futures price is greater than its price t days ago, where t = 15, 25, and 55 days, a long position in a particular market is adopted\" (p.482)",
              "X|\"Otherwise, a short position is adopted\" (p.482)"],
            note!("Daily closes; an unchanged price counts as 'otherwise' (short), as printed. Default 15 days, the first lookback; the middle lookback is printed as both 27 and 25 days on the same page, so p.n = 25, 27 or 55 runs the others. The chapter reports a benchmark fitted to CTA indices, not trades; applied to futures in four sectors. Cash-index shorts are hypothetical. ")),
        live("ggfohf_ma", "Price vs m-day moving average, long above / short below (trend-following factor, 224 days)", "Ch.4 Bacmann, Jeanneret & Scholz (p.61)", "ggfohf", "Long/Short",
            &["E|\"B(t-1) = 1 if P(t-1) > (1/m) sum P(t-i) (long position)\" (p.61)",
              "X|\"B(t-1) = -1 if P(t-1) < (1/m) sum P(t-i) (short position) ... P(t-1) is the closing price of the futures at day t-1 and m is the number of days in the moving average\" (p.61)",
              "F|\"The strategy for a given futures market is composed of four moving averages: 224 days, 117 days, 61 days, and 32 days\" (p.61)"],
            note!("Simple average of the last m closes including today's, as printed; the position for the next day is set at today's close (R(t) = B(t-1) X(t)). Default m = 224, the first listed; p.n = 117, 61 or 32 runs the others. How the four averages are combined within one market is not stated, and the inverse-volatility weights across 38 futures markets have no window, so each average runs on its own. Cash-index shorts are hypothetical. ")),
        held("gghf_persistence", "Hedge-fund return-persistence portfolios", "Performance-persistence chapters (p.297-316)", "gghf",
            NeedsData("ranks hedge-fund managers; needs FRM fund return histories, not market prices"), &["E|Hold the best and worst 20-manager blocks"]),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "ggth_sma_ls" => ("ggth_sma_ls", "Handbook of Trading (1,n) average", "1,9"),
        "ggth_dmaco" => ("ggth_dmaco", "Lento DMACO", "1,50"),
        "ggth_dmaco_es" => ("ggth_dmaco_es", "Lento smoothed DMACO", "1,50 alpha 0.1"),
        "ggeai_ma5_30" => ("ggeai_ma5_30", "Systematic CTA crossover", "5/30"),
        "gghf_rb80" => ("gghf_rb80", "Calyon 80-day breakout", "80"),
        "gghf_ma80" => ("gghf_ma80", "Calyon 80-day average", "80"),
        "gghf_mom15" => ("gghf_mom15", "Spurgin-Schneeweis-Georgiev momentum", "15"),
        "ggfohf_ma" => ("ggfohf_ma", "Bacmann-Jeanneret-Scholz trend factor", "224"),
        _ if id.starts_with("ggth_") || id.starts_with("ggeai_") || id.starts_with("gghf_") || id.starts_with("ggfohf_") => ("gg_nt", "Gregoriou volumes (held)", "-"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max| ParamDef { key, label, def, min, max, step: 1. };
    Some(match id {
        "ggth_sma_ls" => vec![p("n", "Long average (book: 9, 15, 30, 60, 90, 120, 150)", 9., 2., 250.)],
        "ggth_dmaco" => vec![p("s", "Short average (book: 1 or 10)", 1., 1., 50.), p("l", "Long average (book: 50 or 200)", 50., 2., 250.)],
        "ggth_dmaco_es" => vec![p("s", "Short average (book: 1 or 10)", 1., 1., 50.), p("l", "Long average (book: 50 or 200)", 50., 2., 250.),
            ParamDef { key: "alpha", label: "Smoothing factor (book: 0.1)", def: 0.1, min: 0.1, max: 1., step: 0.1 }],
        "ggeai_ma5_30" => vec![p("s", "Short average (book: perhaps 5)", 5., 1., 50.), p("l", "Long average (book: perhaps 30)", 30., 2., 250.)],
        "gghf_rb80" | "gghf_ma80" => vec![p("n", "Days (book: 80)", 80., 2., 250.)],
        "gghf_mom15" => vec![p("n", "Lookback days (book: 15, 27 or 25, 55)", 15., 1., 250.)],
        "ggfohf_ma" => vec![p("n", "Average days (book: 224, 117, 61, 32)", 224., 2., 300.)],
        _ => return None,
    })
}

/// Lento's smoothed series (Eq.11.3): S[0] undefined, S[1] = y[0], S[t] = a*y[t-1] + (1-a)*S[t-1].
pub fn lento_smooth(y: &[f64], a: f64) -> Vec<f64> {
    let mut s = vec![f64::NAN; y.len()];
    for t in 1..y.len() { s[t] = if t == 1 { y[0] } else { a * y[t - 1] + (1.0 - a) * s[t - 1] }; }
    s
}

/// Position state after each bar: +1 / -1, kept on ties, None before the first signal.
fn states(x: &Ctx, id: &str, p: &Params) -> Vec<Option<i8>> {
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d);
    let n_of = |k: &str, d: f64| g(k, d).round().max(1.) as usize;
    let c = &x.c;
    let sign = |a: f64, b: f64| if !(a.is_finite() && b.is_finite()) || a == b { 0 } else if a > b { 1 } else { -1 };
    let raw: Vec<i8> = match id {
        "ggth_sma_ls" | "gghf_ma80" | "ggfohf_ma" => {
            let ma = sma(c, n_of("n", match id { "gghf_ma80" => 80., "ggfohf_ma" => 224., _ => 9. }));
            (0..c.len()).map(|i| sign(c[i], ma[i])).collect()
        }
        "ggth_dmaco" | "ggeai_ma5_30" | "ggth_dmaco_es" => {
            let (ds, dl) = if id == "ggeai_ma5_30" { (5., 30.) } else { (1., 50.) };
            let base = if id == "ggth_dmaco_es" { lento_smooth(c, g("alpha", 0.1).clamp(0.01, 1.)) } else { c.clone() };
            let (a, b) = (sma(&base, n_of("s", ds)), sma(&base, n_of("l", dl)));
            (0..c.len()).map(|i| sign(a[i], b[i])).collect()
        }
        "gghf_rb80" => {
            let n = n_of("n", 80.);
            (0..c.len()).map(|i| if i < n { 0 } else {
                let w = &c[i - n..i];
                let (hi, lo) = (w.iter().cloned().fold(f64::MIN, f64::max), w.iter().cloned().fold(f64::MAX, f64::min));
                if c[i] > hi { 1 } else if c[i] < lo { -1 } else { 0 }
            }).collect()
        }
        "gghf_mom15" => {
            let n = n_of("n", 15.);
            (0..c.len()).map(|i| if i < n { 0 } else if c[i] > c[i - n] { 1 } else { -1 }).collect()
        }
        _ => return vec![],
    };
    let mut cur = None;
    raw.into_iter().map(|r| { if r != 0 { cur = Some(r); } cur }).collect()
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from > to || to >= x.r.len() { return None; }
    let st = states(x, id, p);
    let long_only = matches!(id, "ggth_dmaco" | "ggth_dmaco_es");
    let (up, down) = match id {
        "gghf_rb80" => ("close above the prior high", "close below the prior low"),
        "ggth_sma_ls" | "gghf_ma80" | "ggfohf_ma" => ("close crossed above the average", "close crossed below the average"),
        "gghf_mom15" => ("close above the close n days ago", "close not above the close n days ago"),
        _ => ("short average crossed above the long", "short average crossed below the long"),
    };
    let (mut out, mut cur) = (vec![], None::<Trade>);
    for i in from.max(1)..=to {
        let (s, prev) = (st[i], st[i - 1]);
        if !matches!((s, prev), (Some(a), Some(b)) if a != b) { continue; }
        let d = s.unwrap();
        if let Some(mut t) = cur.take() {
            t.exit_i = i; t.exit_px = x.c[i]; t.signal_i = i; t.reason = (if d > 0 { up } else { down }).into(); out.push(t);
        }
        if d > 0 || !long_only {
            cur = Some(Trade { dir: d as f64, legs: vec![Leg { i, px: x.c[i], w: 1. }], exit_i: i, exit_px: x.c[i], reason: String::new(),
                entry_kind: "close", signal_i: i, open: false, exits: vec![] });
        }
    }
    if let Some(mut t) = cur { t.exit_i = to; t.exit_px = x.c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true; out.push(t); }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn lento_smoothing_uses_only_earlier_closes() {
        let s = lento_smooth(&[10., 20., 30.], 0.1);
        assert!(s[0].is_nan());
        assert_eq!(s[1], 10.);
        assert!((s[2] - (0.1 * 20. + 0.9 * 10.)).abs() < 1e-12);
    }
    #[test]
    fn catalogue_admits_exactly_the_reviewed_ids() {
        let live: Vec<_> = catalogue().into_iter().filter(|d| d.status == Status::Live).map(|d| d.id).collect();
        assert_eq!(live, REVIEWED);
    }
}
