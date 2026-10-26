//! Stanley Kroll, "Dragons and Bulls: Profitable Investment Strategies for Trading Stocks and Commodities" (Addison-Wesley,
//! 1994; book id `skdab`) and "Kroll on Futures Trading Strategy" (Dow Jones-Irwin, 1988; book id `skfts`). Reading:
//! claude-continuation-20261020/authors/kroll/agents/. Live: the Ch.11 long-term trend strategy of Dragons and Bulls (p.59-61):
//! the 50-day average and its slope set the trend, close > 10 > 20 > 50-day averages enters with the trend, close < 4 < 9 <
//! 18-day averages stops it out (mirror for shorts), and the short-term signal re-enters while the trend holds.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};
use crate::books::mpts::sma;

pub const BOOK: &str = "skdab";
pub const REVIEWED: &[&str] = &["skdab_trend3ma"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    let nt = |id, name, group, book, why, rules| StrategyDef { id, name, group, session: 0, dir: "Long/Short", status: NotTestable(why), needs_volume: false,
        needs_vix: false, needs_series: &[], book, rules, notes: "Held after the 2026-10-24 reading (authors/kroll/agents).", claim: None };
    vec![
        StrategyDef { id: "skdab_trend3ma", name: "50-day trend filter with 10/20/50 entries and 4/9/18 stops (Ch.11)", group: "Ch.11 · Long-term versus short-term trading (p.59-61)",
            session: 0, dir: "Long/Short", status: Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
            rules: &["F|\"The trend is up if: (a) The closing price is above the moving average line and (b) The slope of the moving average line is up\" (50-day simple average; down: below and sloping down; otherwise sideways, 'avoided') (p.59-60)",
                     "E|\"In an uptrend (using the 50-day simple moving average) buy on the long-term signal, that is; close > 10-day > 20-day > 50-day\" (p.60); downtrend: sell short on close < 10-day < 20-day < 50-day",
                     "X|\"Stop out the trade (liquidate only, do not go short) using the short-term signal, that is; close < 4-day < 9 day < 18-day\" (p.60); shorts: close > 4-day > 9-day > 18-day (p.61)",
                     "E|\"If you are stopped out and the trend continues up (based on the 50-day simple moving average) re-enter long on the short-term buy signal, that is; close > 4-day > 9-day > 18-day\" (p.60); mirror for shorts (p.61)",
                     "F|\"In an uptrend, carry long positions only, or else stand aside. No short positions.\" (p.60)"],
            notes: "Kroll: 'the particular moving average values presented here are not the ones I specifically use. They are presented for illustration only' (p.59), but they are the rules he gives for the discussion. [Declared readings, not from the book] all averages are simple averages of closes; slope up = today's 50-day average above yesterday's; orders fill at the close of the signal day; re-entry is allowed while the trend has stayed in the same direction since the stop-out (a trend change ends it); the optional exit stop at 70% of futures margin is not used; flat at the window start. Indices, ETFs and stocks, US and India, are a declared application (book: NIKKEI and futures examples).",
            claim: None },
        nt("skdab_9_18_50", "Close vs 9/18/50-day averages with the 50-day trend (Ch.6 NIKKEI example)", "Ch.6 · That's the way you want to bet (p.32-35)", BOOK,
           "the 9/18/50 signals are given as entry and exit signals but the exit (opposite signal, or loss of the buy condition) and the 'get back aboard on the close of the next two days' re-entry are not defined exactly",
           &["E|\"Buy: close > 9-SMA > 18-SMA > 50-SMA. Sell: close < 9-SMA < 18-SMA < 50 SMA.\" (p.32)"]),
        nt("skdab_strategy", "Pyramiding, margin, news and trader-discipline chapters", "Dragons and Bulls", BOOK, "money management and judgement, no complete rule", &[]),
        nt("skfts_ma_examples", "Moving average examples (19-day close cross, 5/20, 4-9-18) and Hochheimer's best averages", "Kroll on Futures Trading Strategy · Ch.12 (p.95-100)", "skfts",
           "illustrations of common systems already in the vault (e.g. kftsm_4_9_18, kftsm_ma); Hochheimer's lengths are 1970-76 optimisations per futures market",
           &["E|\"you would buy when the price closes above the 19-day m.a. and sell (possibly reversing as well) when the price closes below\" (p.97)"]),
        nt("skfts_strategy", "Long-term trend trading, pyramiding and money management", "Kroll on Futures Trading Strategy", "skfts",
           "discretionary strategy and tactics around unspecified long-term computer systems", &[]),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "skdab_trend3ma" => ("skdab_trend3ma", "Kroll trend strategy", "10/20/50 + 4/9/18"),
        _ if id.starts_with("skdab_") || id.starts_with("skfts_") => ("skdab_nt", "Kroll (held)", "-"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    Some(match id {
        "skdab_trend3ma" => vec![],
        _ if id.starts_with("skdab_") || id.starts_with("skfts_") => vec![],
        _ => return None,
    })
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, _p: &Params) -> Option<Vec<Trade>> {
    if id != "skdab_trend3ma" || from > to || to >= x.r.len() { return None; }
    let c = &x.c;
    let (m4, m9, m18, m10, m20, m50) = (sma(c, 4), sma(c, 9), sma(c, 18), sma(c, 10), sma(c, 20), sma(c, 50));
    let ok = |i: usize| [m4[i], m9[i], m18[i], m10[i], m20[i], m50[i]].iter().all(|v| v.is_finite()) && i >= 1 && m50[i - 1].is_finite();
    let trend = |i: usize| -> i8 {
        if c[i] > m50[i] && m50[i] > m50[i - 1] { 1 } else if c[i] < m50[i] && m50[i] < m50[i - 1] { -1 } else { 0 }
    };
    let long_term = |i: usize, d: f64| if d > 0. { c[i] > m10[i] && m10[i] > m20[i] && m20[i] > m50[i] } else { c[i] < m10[i] && m10[i] < m20[i] && m20[i] < m50[i] };
    let short_term = |i: usize, d: f64| if d > 0. { c[i] > m4[i] && m4[i] > m9[i] && m9[i] > m18[i] } else { c[i] < m4[i] && m4[i] < m9[i] && m9[i] < m18[i] };
    let (mut out, mut cur, mut reentry) = (vec![], None::<Trade>, 0.0_f64);
    for i in from.max(1)..=to {
        if !ok(i) { continue; }
        let t = trend(i) as f64;
        if reentry != 0. && t != reentry { reentry = 0.; }   // a re-entry needs the trend unchanged since the stop-out
        if let Some(mut tr) = cur.take() {
            if short_term(i, -tr.dir) {
                tr.exit_i = i; tr.exit_px = c[i]; tr.signal_i = i; tr.reason = "short-term signal against the position (4/9/18)".into();
                reentry = if t == tr.dir { tr.dir } else { 0. };
                out.push(tr);
            } else {
                cur = Some(tr);
            }
            continue;
        }
        if t == 0. { continue; }
        if long_term(i, t) || (reentry == t && short_term(i, t)) {
            cur = Some(Trade { dir: t, legs: vec![Leg { i, px: c[i], w: 1. }], exit_i: i, exit_px: c[i], reason: String::new(), entry_kind: "close", signal_i: i, open: false, exits: vec![] });
            reentry = 0.;
        }
    }
    if let Some(mut tr) = cur { tr.exit_i = to; tr.exit_px = c[to]; tr.signal_i = to; tr.reason = "open".into(); tr.open = true; out.push(tr); }
    Some(out)
}
