//! Gerald Appel, Winning Market Systems: 83 Ways to Beat the Market (Signalert / Windsor Books scan).
//! Only systems whose buy AND sell rules are stated mechanically are Live. Trendlines, chart
//! formations, cycles, "judgment" calls and indicators without an exit are NotTestable; systems
//! needing data the vault lacks are NeedsData. Page refs are printed pages (PDF page = printed + 6).
//! Sources, transcription and checks: library/apwms/. Shared helpers here are reused by apsmts/apoi.
use super::apta::bars;
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

/// Admitted 2026-10-24 (authors/appel/admission/SOURCE-CONTRACT.md): independent ledgers, US and India, 0 mismatches.
pub const REVIEWED: &[&str] = &["apwms_trin5", "apwms_trin5_nse", "apwms_updown_vol", "apwms_updown_vol_nse", "apwms_shadow", "apwms_ad10", "apwms_ad10_nse", "apwms_haurlan_long", "apwms_haurlan_int", "apwms_haurlan_short", "apwms_haurlan_long_nse", "apwms_haurlan_int_nse", "apwms_haurlan_short_nse"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

pub const BOOK: &str = "apwms";

// ------------------------------------------------------------------ shared helpers (also used by apsmts, apoi)

/// Exponential average with a given smoothing constant, started from the first finite value
/// ("Take your first data entry as the most recent exponential average", SMTS p.8; Haurlan, WMS p.144).
/// NaN inputs leave the average unchanged and give NaN on that bar.
pub fn ema_a(v: &[f64], a: f64) -> Vec<f64> {
    let mut e = f64::NAN;
    v.iter()
        .map(|&x| {
            if !x.is_finite() {
                return f64::NAN;
            }
            e = if e.is_finite() { e + a * (x - e) } else { x };
            e
        })
        .collect()
}

/// Simple moving average summed afresh over each window (left to right), NaN if any day in the window
/// is missing. Unlike a running sum it gives exact ties (close == average) when the arithmetic is exact,
/// so "crosses above/below" is not triggered by rounding drift.
pub fn sma(v: &[f64], n: usize) -> Vec<f64> {
    (0..v.len())
        .map(|i| if i + 1 >= n && v[i + 1 - n..=i].iter().all(|x| x.is_finite()) { v[i + 1 - n..=i].iter().sum::<f64>() / n as f64 } else { f64::NAN })
        .collect()
}

/// Linearly weighted moving average, weights 1..n (SMTS p.6-7 worksheet).
pub fn wma(v: &[f64], n: usize) -> Vec<f64> {
    let den = (n * (n + 1) / 2) as f64;
    (0..v.len())
        .map(|i| if i + 1 >= n { (0..n).map(|k| v[i + 1 - n + k] * (k + 1) as f64).sum::<f64>() / den } else { f64::NAN })
        .collect()
}

/// Number of finite values seen up to and including each bar (for stabilization periods).
pub fn seen(v: &[f64]) -> Vec<usize> {
    let mut n = 0;
    v.iter().map(|x| { n += x.is_finite() as usize; n }).collect()
}

fn trade(dir: f64, i: usize, px: f64) -> Trade {
    Trade { dir, legs: vec![Leg { i, px, w: 1.0 }], exit_i: i, exit_px: px, reason: String::new(), entry_kind: "close", signal_i: i, open: false, exits: vec![] }
}

/// Long/short (or long-only) runner on bars `idx`, every fill at the close of the signal bar.
/// `step(k, i)` returns the position the rules want (+1 long, -1 short, 0 flat) or None (no change).
/// With `long_only`, -1 means flat. Rule state runs from the first bar; a position the rules already
/// hold at `from` is not reported (it starts virtual), and no entry is taken on the last bar `to`.
pub fn run_ls(x: &Ctx, idx: &[usize], from: usize, to: usize, long_only: bool, mut step: impl FnMut(usize, usize) -> Option<(i8, String)>) -> Vec<Trade> {
    let (mut out, mut cur, mut pos) = (vec![], None::<Trade>, 0i8);
    for (k, &i) in idx.iter().enumerate() {
        if i > to {
            break;
        }
        let Some((want, why)) = step(k, i) else { continue };
        let want = if long_only { want.max(0) } else { want };
        if want == pos {
            continue;
        }
        if let Some(mut t) = cur.take() {
            t.exit_i = i;
            t.exit_px = x.c[i];
            t.signal_i = i;
            t.reason = why;
            out.push(t);
        }
        pos = want;
        if want != 0 && i >= from && i < to {
            cur = Some(trade(want as f64, i, x.c[i]));
        }
    }
    if let Some(mut t) = cur {
        t.exit_i = to;
        t.exit_px = x.c[to];
        t.signal_i = to;
        t.reason = "open".into();
        t.open = true;
        out.push(t);
    }
    out
}

pub const NYSE_AD: &[&str] = &["NYSE_ADV_DEC.Advances", "NYSE_ADV_DEC.Declines"];
pub const NSE_AD: &[&str] = &["NSE_ADV_DEC.Advances", "NSE_ADV_DEC.Declines"];
const NYSE_TRIN: &[&str] = &["NYSE_ADV_DEC.Advances", "NYSE_ADV_DEC.Declines", "NYSE_ADV_DEC.AdvVolume", "NYSE_ADV_DEC.DecVolume"];
const NSE_TRIN: &[&str] = &["NSE_ADV_DEC.Advances", "NSE_ADV_DEC.Declines", "NSE_ADV_DEC.AdvVolume", "NSE_ADV_DEC.DecVolume"];
const NYSE_VOL: &[&str] = &["NYSE_ADV_DEC.AdvVolume", "NYSE_ADV_DEC.DecVolume"];
const NSE_VOL: &[&str] = &["NSE_ADV_DEC.AdvVolume", "NSE_ADV_DEC.DecVolume"];

/// Daily advances minus declines (NYSE, or NSE for `_nse` ids); NaN on days without breadth data.
pub fn a_minus_d(x: &Ctx, nse: bool) -> Option<Vec<f64>> {
    let s = if nse { NSE_AD } else { NYSE_AD };
    let (a, d) = (x.series(s[0])?, x.series(s[1])?);
    Some(a.iter().zip(d).map(|(a, d)| a - d).collect())
}

/// Moving total of the last n finite-or-not values (NaN if any day in the window lacks data).
pub fn moving_total(v: &[f64], n: usize) -> Vec<f64> {
    sma(v, n).iter().map(|m| m * n as f64).collect()
}

/// Days 1..=n of `v` are all finite and `pred` holds for each.
fn all_last(v: &[f64], i: usize, n: usize, pred: impl Fn(f64) -> bool) -> bool {
    i + 1 >= n && (0..n).all(|k| v[i - k].is_finite() && pred(v[i - k]))
}

// ------------------------------------------------------------------ catalogue

const fn side_note() -> &'static str {
    "Long and short (stop-and-reverse) by default: the book scores its own buy/sell systems as long AND short trades (Table I p.106-107: \"Profitable Signals: 13 long, 9 short\"; A-D System #2 p.138: \"Six of sixteen sell signals were profitable\"). p.side=1 = long only (sell = go flat). Fills at the close of the signal day."
}

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, dir, status, series: &'static [&'static str], rules, notes| StrategyDef {
        id, name, group, session: 0, dir, status, needs_volume: false, needs_vix: false, needs_series: series,
        book: BOOK, rules, notes, claim: None,
    };
    use Status::*;
    let mut v = vec![
        // ---- Ch 2 Volume
        d("apwms_trin5", "Short Term Trading Index (TRIN), 5-day average · NYSE", "Ch 2 · Volume System #10", "Long/Short", Live, NYSE_TRIN,
            &["F|\"Number of Stocks Advancing / Number of Stocks Declining ÷ Advancing Volume / Declining Volume = Short Term Trading Index\" (p.36); 800/400 ÷ 5,000,000/2,000,000 = .80 (p.37)",
              "F|\"plotting a five-day moving average of the Short Term Trading Index, based on total daily activity\" (p.37)",
              "E|\"a buy when it falls below 1.00\" (p.37). Buy (cover short) on the close.",
              "X|\"A sell signal is generated when the five day moving average of the Short Term Trading Index rises above 1.00\" (p.37). Sell (and sell short) on the close."],
            side_note()),
        d("apwms_trin5_nse", "Short Term Trading Index (TRIN), 5-day average · NSE", "Ch 2 · Volume System #10", "Long/Short", Live, NSE_TRIN,
            &["F|TRIN = (advances / declines) ÷ (advancing volume / declining volume) (p.36-37), from NSE breadth",
              "E|Buy when the 5-day average falls below 1.00 (p.37)",
              "X|Sell when the 5-day average rises above 1.00 (p.37)"],
            "India mapping: NSE advances, declines and advancing/declining volume instead of NYSE. Long and short by default (see the NYSE version); p.side=1 long only."),
        d("apwms_updown_vol", "Up volume vs down volume, 10-day averages · NYSE", "Ch 2 · Volume System #16", "Long/Short", Live, NYSE_VOL,
            &["F|\"Plots of ten-day moving averages of up volume and down volume\" (p.42). Up volume = volume of advancing issues, down volume = volume of declining issues.",
              "E|\"Market buy and sell signals are often generated by a crossing of the up and down volume lines ... So long as the up volume line remains above the down volume line a bullish condition continues to exist.\" (p.42) Buy when the 10-day up-volume average crosses above the down-volume average.",
              "X|\"when the ten-day moving average of down volume exceeds the ten-day moving average of up volume, a sell signal is generated\" (p.42)"],
            side_note()),
        d("apwms_updown_vol_nse", "Up volume vs down volume, 10-day averages · NSE", "Ch 2 · Volume System #16", "Long/Short", Live, NSE_VOL,
            &["E|Buy when the 10-day average of advancing-issue volume crosses above that of declining-issue volume (p.42)",
              "X|Sell when the 10-day average of down volume exceeds that of up volume (p.42)"],
            "India mapping: NSE advancing/declining volume. Long and short by default; p.side=1 long only."),
        // ---- Ch 5 moving averages
        d("apwms_shadow", "Larry Williams' Shadow Line (10-week MA vs its one-week-ahead shadow)", "Ch 5 · Moving Average System #5", "Long/Short", Live, &[],
            &["F|\"Employ a standard ten-week moving average line, plotted at the last price of the series. Draw a second line, parallel to the first but projected one week ahead (the shadow line).\" (p.114) Weekly closes (last trading day of each week); shadow this week = last week's average.",
              "E|\"A buy signal is generated when the moving average crosses from below to above its 'shadow line'.\" (p.114) i.e. this week's 10-week average rises above last week's. Buy on the weekly close.",
              "X|\"A sell signal is generated when it crosses from above to below its 'shadow line'.\" (p.114)"],
            "Book example: Natomas 1970-72 (Figure 11); results are only described (\"a clear majority\" of good signals). Run on the indices (instrument swap). Long and short by default (book's convention, see Table I p.106); p.side=1 long only."),
        // ---- Ch 7 A-D line
        d("apwms_ad10", "A-D overbought-oversold indicator: 10-day total of advances minus declines, zero crossings · NYSE", "Ch 7 · Advance-Decline System #2", "Long/Short", Live, NYSE_AD,
            &["F|\"Plot a 10-day moving total of the advances minus declines ... On the eleventh day, add the figures for that day; subtract the first day's figures.\" (p.138)",
              "E|\"Use of a crossing of the zero line to generate buy and sell signals\" (p.138): buy when the 10-day total crosses above 0.",
              "X|Sell (and sell short) when it crosses below 0 (p.138)."],
            "Book test: Sept 1970-March 1972, 32 signals; 10 of 16 long signals profitable (+197.58 Dow points), 6 of 16 sell signals profitable (+20.48); total +218.06 vs market +189.24 (p.138). Run on DJI to compare. Long and short (as tested); p.side=1 long only."),
        d("apwms_ad10_nse", "A-D 10-day total zero crossings · NSE", "Ch 7 · Advance-Decline System #2", "Long/Short", Live, NSE_AD,
            &["F|10-day moving total of NSE advances minus declines (p.138)",
              "E|Buy when it crosses above 0 (p.138)", "X|Sell (and sell short) when it crosses below 0 (p.138)"],
            "India mapping: NSE breadth. Long and short by default; p.side=1 long only."),
    ];
    for (nse, term) in [(false, "long"), (false, "int"), (false, "short"), (true, "long"), (true, "int"), (true, "short")] {
        let id: &'static str = match (nse, term) {
            (false, "long") => "apwms_haurlan_long",
            (false, "int") => "apwms_haurlan_int",
            (false, "short") => "apwms_haurlan_short",
            (true, "long") => "apwms_haurlan_long_nse",
            (true, "int") => "apwms_haurlan_int_nse",
            _ => "apwms_haurlan_short_nse",
        };
        let name: &'static str = match id {
            "apwms_haurlan_long" => "Haurlan Index, long term (1%) zero crossings · NYSE",
            "apwms_haurlan_int" => "Haurlan Index, intermediate term (10%) zero crossings · NYSE",
            "apwms_haurlan_short" => "Haurlan Index, short term (50%) zero crossings, 2-day confirmation · NYSE",
            "apwms_haurlan_long_nse" => "Haurlan Index, long term (1%) · NSE",
            "apwms_haurlan_int_nse" => "Haurlan Index, intermediate term (10%) · NSE",
            _ => "Haurlan Index, short term (50%), 2-day confirmation · NSE",
        };
        let rules: &'static [&'static str] = match term {
            "long" => &["F|\"Long Term: Today's index = C + .01 (P - C) where C = yesterday's Long Term Haurlan Index, P = today's N.Y.S.E. advances minus declines\" (p.142); \"200 days to stabilize the data\" (p.144). Started from the first day's A-D; no signals during the first 200 breadth days.",
                "E|\"For each of these, the basic buy and sell indicator is a cross over and below the zero line\" (p.144): buy when the index crosses above 0.",
                "X|Sell (and sell short) when it crosses below 0 (p.144)."],
            "int" => &["F|\"Intermediate Term: Today's index = B + .1 (P - B)\" (p.144); \"the stabilization period required is 20 days\" (p.144)",
                "E|Buy when the index crosses above the zero line (p.144)", "X|Sell (and sell short) when it crosses below the zero line (p.144)"],
            _ => &["F|\"Short Term: Today's index = A + .5 (P - A) or (A+P)/2\" (p.144); \"The stabilization period is 3 days\" (p.144)",
                "E|\"in the case of the short term index, the reading must remain above or below zero for two days to confirm the signal\" (p.144): buy on the second consecutive day above 0.",
                "X|Sell (and sell short) on the second consecutive day below 0 (p.144)."],
        };
        v.push(d(id, name, "Ch 7 · Advance-Decline System #4 (Haurlan Index)", "Long/Short", Live, if nse { NSE_AD } else { NYSE_AD }, rules,
            if nse { "India mapping: NSE advances minus declines. Long and short by default; p.side=1 long only." } else { "Book shows the three indices for 1970-72 (Figure 4, p.143), e.g. week of June 19-23 1972: short -150 -54 -59 -195 -216, intermediate -125 -108 -104 -127 -142, long -32 -31 -31 -34 -36. No trade results are printed. Long and short by default (book's convention, Table I p.106, A-D #2 p.138); p.side=1 long only." }));
    }
    let nt = |id, name, group, why| d(id, name, group, "Long", NotTestable(why), &[], &[], "");
    let nd = |id, name, group, why, rules| d(id, name, group, "Long/Short", NeedsData(why), &[], rules, "");
    v.extend([
        nt("apwms_trendlines", "Trendline systems #1-#8 (incl. Key-Volume chart segments)", "Ch 1 · Trendlines", "discretionary: every signal needs a hand-drawn trendline (p.1-20)"),
        nt("apwms_volume_judgement", "Volume systems #1-#9, #15, #17 (climaxes, OBV and A/D divergences, Martel, Worden, VPT, Equivolume, NVI)", "Ch 2 · Volume", "discretionary: divergences, trendlines and \"previous high/low\" are not defined; NVI needs \"some discretion\" (p.21-44)"),
        nt("apwms_trin10_obos", "10-day TRIN overbought/oversold (.70-.80 / 1.15-1.30) and 10/25-day TRIN crossings", "Ch 2 · Volume System #11", "no trade rule: levels are areas that shift between bull and bear markets; the 10/25-day crossing gives no direction (p.37)"),
        nt("apwms_trin_intraday", "Intraday TRIN readings (175-200, 1.20, below 1.00)", "Ch 2 · Volume System #10", "intraday and judgemental (\"down five to six points\"), no exit (p.37, p.132)"),
        nd("apwms_oar", "Odd-lot / option activity ratio (OAR) top and bottom signals", "Ch 2 · Volume System #13", "needs weekly NYSE volume and listed option volume (not in the vault); put/call 4-week change rule also needs put/call history (p.38)",
            &["F|Weekly NYSE volume ÷ option volume, 10-week average (p.38)", "E|Bottom signal: both readings below 0.75 (p.38)", "X|Top signal: both readings above 1.25 (p.38)"]),
        nd("apwms_blocks_ase", "Big block activity; AMEX/NYSE volume ratio", "Ch 2 · Volume Systems #12, #14", "needs block-trade counts and AMEX volume history (p.38-39)", &[]),
        nt("apwms_chart_patterns", "Chart formations (flags, triangles, head and shoulders, gaps ...)", "Ch 3 · Chart formations", "discretionary chart reading (p.45-66)"),
        nt("apwms_point_figure", "Point and figure systems #1-#8 (Chartcraft 3-point reversal)", "Ch 4 · Point and figure", "#1-#7 are counts and judgement; #8 (\"ascending column exceeds the previous ascending column by at least one square\", p.89) duplicates kftsm_pf (box p.box; WMS scale: 2 points above 100) - not re-coded"),
        nt("apwms_ma_tactics", "Moving average tactics, +-2.00 oscillator levels, 40-week flattening (Gordon), 3/10 stock oscillator (System #2)", "Ch 5 · Moving averages", "no complete rule: overbought/oversold levels and \"flattening\" are judgement; System #2 gives no exit (\"I do not recommend ... selling upon oscillator moves through zero alone\", p.108-111)"),
        nt("apwms_ma_projection", "Centered moving average channels and price objectives (Systems #3, #4)", "Ch 5 · Moving averages", "price projections, not trades (p.111-113)"),
        nt("apwms_cycles", "Time cycles, Hurst, Elliott, price/time objectives, trading by the clock, best months", "Ch 6 · Time cycles", "cycle lows and projections are judgement; Hirsch month table is a statistic without a trade rule; first/last half hour is intraday (p.115-133)"),
        nt("apwms_breadth_climax", "Breadth climaxes, unchanged issues, McClellan oscillator, plurality, industry groups", "Ch 7 · Advance-Decline Systems #1, #5, #6", "no trade rule: \"a retracement ... may be expected\" (p.138); McClellan: \"buy as soon as it turns up from a multiple bottom formation\" (judgement, p.145-146)"),
        nt("apwms_haller", "Haller theory (weekly A-D legs, net new highs, Dow yield)", "Ch 7 · Advance-Decline System #3", "discretionary: \"A leg may consist of a zig-zag\" and peaks are picked by eye; also needs weekly new highs, issues traded and the Dow yield (p.141-142)"),
        nt("apwms_fundamental", "Fundamental systems #1-#10 (growth, P/E, payout time, money and credit, muni yields, fund cash, Dow yield, utilities, balance sheets, Value Line)", "Ch 8 · Fundamental", "fundamental data and judgement; no complete buy/sell rule except the Confidence Index (p.147-170)"),
        nd("apwms_confidence", "Barron's Confidence Index vs its 10-month average", "Ch 8 · Fundamental (Gordon)", "needs Barron's Confidence Index monthly history (not in the vault)",
            &["E|\"Buy signals are generated by a 1 1/2 point penetration up through the moving average\" (10-month, p.164)", "X|\"sell signals by the same penetration down\" (p.164)"]),
        nt("apwms_sentiment", "Short interest, specialist and odd-lot short ratios, Zweig sell signal, Investors Intelligence", "Ch 9 · Potpourri", "levels without exits (\"There is no automatic signal level\", p.183) and data not in the vault"),
        nd("apwms_glamex", "Glamour index (Glamex) vs its 10-day average", "Ch 9 · Misc System #4", "needs daily closes of the nine 1972 glamour stocks (Burroughs, DEC, Disney, IBM, Memorex, Polaroid, Telex, Teleprompter, Xerox)",
            &["F|Sum of the nine prices with multipliers (Disney x2, Telex x5, Teleprompter x4), divided by 9 (p.182)", "E|\"A buy signal is generated when the glamour index rises above its ten day moving average\" (p.183)", "X|\"a sell when it closes below its ten day moving average\" (p.183)"]),
        nd("apwms_mas", "Most Active Stocks (MAS) indicator, +-3 crossings", "Ch 9 · Misc System #7", "needs the daily list of the fifteen most active NYSE stocks and their direction",
            &["F|Daily net of the fifteen most active stocks up minus down; MAS = 30-day moving total / 3 (p.188)", "E|\"Intermediate buy signals are generated when the 30-day moving average crosses above plus three.\" (p.188)", "X|\"Sell signals are generated when it crosses below minus three.\" (p.188)"]),
        nt("apwms_nhnl_actives", "New highs/new lows levels; most actives interpretation; composites", "Ch 9 · Misc Systems #6-#8", "levels without exits and composites without weights (p.186-189)"),
        nt("apwms_momentum_5ddo", "Maxwell 5-day oscillator (5DDO)", "Ch 9 · Misc System #9", "\"does not render automatic signals ... require judgment\" (p.190-195)"),
        nt("apwms_runaways", "Key-Volume two-day runaways", "Ch 9 · Misc System #10", "series length \"four or five\", entry \"within two trading days\" and exit \"until the next short term reversal\" are not defined (p.196-197)"),
        nd("apwms_trading_range", "Long-term trading range stocks (Villella/Robertson)", "Ch 9 · Misc System #11", "needs a stock universe with five-year highs and lows; selling is in increments by judgement (p.197-198)", &[]),
        nt("apwms_money_savers", "Convertibles, warrants, hedging, options, no-load funds; the Master System checklist", "Ch 10-11", "not market-timing rules (p.199-223)"),
        nt("apwms_dow_200_deviation", "Dow deviation from its 200-day average (+-130 points)", "Preface to the second edition", "no exit: \"Simply watch for the extremes and take action\" (Preface)"),
    ]);
    v
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "apwms_trin5" => ("apwms_trin", "Short Term Trading Index (TRIN)", "5-day · NYSE"),
        "apwms_trin5_nse" => ("apwms_trin", "Short Term Trading Index (TRIN)", "5-day · NSE"),
        "apwms_updown_vol" => ("apwms_vol", "Up/down volume averages", "NYSE"),
        "apwms_updown_vol_nse" => ("apwms_vol", "Up/down volume averages", "NSE"),
        "apwms_shadow" => ("apwms_ma", "Moving average systems", "Shadow line"),
        "apwms_ad10" => ("apwms_ad", "A-D oscillators", "10-day total · NYSE"),
        "apwms_ad10_nse" => ("apwms_ad", "A-D oscillators", "10-day total · NSE"),
        "apwms_haurlan_long" => ("apwms_haurlan", "Haurlan Index", "Long term · NYSE"),
        "apwms_haurlan_int" => ("apwms_haurlan", "Haurlan Index", "Intermediate · NYSE"),
        "apwms_haurlan_short" => ("apwms_haurlan", "Haurlan Index", "Short term · NYSE"),
        "apwms_haurlan_long_nse" => ("apwms_haurlan", "Haurlan Index", "Long term · NSE"),
        "apwms_haurlan_int_nse" => ("apwms_haurlan", "Haurlan Index", "Intermediate · NSE"),
        "apwms_haurlan_short_nse" => ("apwms_haurlan", "Haurlan Index", "Short term · NSE"),
        s if s.starts_with("apwms_") => ("apwms_other", "Other Winning Market Systems material", ""),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let side = || p("side", "1 = long only (sell = flat); 0 = long and short (book)", 0., 0., 1., 1.);
    Some(match id {
        "apwms_trin5" | "apwms_trin5_nse" => vec![p("n", "TRIN average days", 5., 2., 20., 1.), p("level", "Signal level", 1., 0.7, 1.3, 0.05), side()],
        "apwms_updown_vol" | "apwms_updown_vol_nse" => vec![p("n", "Volume average days", 10., 3., 30., 1.), side()],
        "apwms_shadow" => vec![p("n", "Moving average weeks", 10., 3., 40., 1.), p("lead", "Shadow lead weeks", 1., 1., 8., 1.), side()],
        "apwms_ad10" | "apwms_ad10_nse" => vec![p("n", "Moving total days", 10., 3., 30., 1.), side()],
        "apwms_haurlan_long" | "apwms_haurlan_long_nse" => vec![p("k", "Smoothing constant", 0.01, 0.005, 0.05, 0.005), p("stab", "Stabilization days", 200., 0., 400., 10.), side()],
        "apwms_haurlan_int" | "apwms_haurlan_int_nse" => vec![p("k", "Smoothing constant", 0.1, 0.05, 0.3, 0.01), p("stab", "Stabilization days", 20., 0., 100., 5.), side()],
        "apwms_haurlan_short" | "apwms_haurlan_short_nse" => vec![p("k", "Smoothing constant", 0.5, 0.2, 0.8, 0.05), p("stab", "Stabilization days", 3., 0., 20., 1.),
            p("confirm", "Days above/below zero to confirm", 2., 1., 5., 1.), side()],
        _ => return None,
    })
}

/// Haurlan index: X = X + k (P - X), started at the first finite P. NaN on days without breadth.
pub fn haurlan(p: &[f64], k: f64) -> Vec<f64> {
    ema_a(p, k)
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, prm: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if from > to || to >= x.r.len() {
        return None;
    }
    let g = |k: &str| prm.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap().def);
    let u = |k: &str| g(k).round().max(1.0) as usize;
    let lo = g("side") > 0.5;
    let nse = id.ends_with("_nse");
    let daily: Vec<usize> = (0..x.r.len()).collect();
    let sgn = |v: f64, lv: f64| if v > lv { Some(1i8) } else if v < lv { Some(-1) } else { None };
    Some(match id {
        "apwms_trin5" | "apwms_trin5_nse" => {
            let s = if nse { NSE_TRIN } else { NYSE_TRIN };
            let (Some(a), Some(d), Some(av), Some(dv)) = (x.series(s[0]), x.series(s[1]), x.series(s[2]), x.series(s[3])) else { return Some(vec![]) };
            let t: Vec<f64> = (0..a.len()).map(|i| {
                let v = (a[i] / d[i]) / (av[i] / dv[i]);
                if v.is_finite() { v } else { f64::NAN }
            }).collect();
            let m = sma(&t, u("n"));
            let lv = g("level");
            // below the level = buy, above = sell
            run_ls(x, &daily, from, to, lo, |_, i| sgn(lv, m[i]).filter(|_| m[i].is_finite()).map(|s| (s, if s > 0 { "TRIN average below 1.00" } else { "TRIN average above 1.00" }.into())))
        }
        "apwms_updown_vol" | "apwms_updown_vol_nse" => {
            let s = if nse { NSE_VOL } else { NYSE_VOL };
            let (Some(uv), Some(dv)) = (x.series(s[0]), x.series(s[1])) else { return Some(vec![]) };
            let (mu, md) = (sma(uv, u("n")), sma(dv, u("n")));
            run_ls(x, &daily, from, to, lo, |_, i| {
                if !(mu[i].is_finite() && md[i].is_finite()) { return None; }
                sgn(mu[i], md[i]).map(|s| (s, if s > 0 { "up-volume average above down-volume average" } else { "down-volume average exceeds up-volume average" }.into()))
            })
        }
        "apwms_shadow" => {
            let idx = bars(x, 1);
            let cl: Vec<f64> = idx.iter().map(|&i| x.c[i]).collect();
            let ma = sma(&cl, u("n"));
            let lead = u("lead");
            run_ls(x, &idx, from, to, lo, |k, _| {
                if k < lead || !(ma[k].is_finite() && ma[k - lead].is_finite()) { return None; }
                sgn(ma[k], ma[k - lead]).map(|s| (s, if s > 0 { "10-week average crossed above its shadow" } else { "10-week average crossed below its shadow" }.into()))
            })
        }
        "apwms_ad10" | "apwms_ad10_nse" => {
            let Some(p) = a_minus_d(x, nse) else { return Some(vec![]) };
            let t = moving_total(&p, u("n"));
            run_ls(x, &daily, from, to, lo, |_, i| {
                if !t[i].is_finite() { return None; }
                sgn(t[i], 0.0).map(|s| (s, if s > 0 { "10-day A-D total crossed above 0" } else { "10-day A-D total crossed below 0" }.into()))
            })
        }
        _ if id.starts_with("apwms_haurlan_") => {
            let Some(p) = a_minus_d(x, nse) else { return Some(vec![]) };
            let h = haurlan(&p, g("k"));
            let n_seen = seen(&p);
            let stab = g("stab").round().max(0.0) as usize;
            let conf = if id.contains("_short") { u("confirm") } else { 1 };
            run_ls(x, &daily, from, to, lo, |_, i| {
                if !h[i].is_finite() || n_seen[i] <= stab { return None; }
                if all_last(&h, i, conf, |v| v > 0.0) {
                    Some((1, "Haurlan index above zero".into()))
                } else if all_last(&h, i, conf, |v| v < 0.0) {
                    Some((-1, "Haurlan index below zero".into()))
                } else {
                    None
                }
            })
        }
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn haurlan_matches_the_books_worked_examples() {
        // long term (p.144): 50, then 800-400 = +400 -> 50 + .01*350 = 53.5 ("54" rounded)
        assert!((haurlan(&[50., 400.], 0.01)[1] - 53.5).abs() < 1e-9);
        // intermediate: 250, then 400-800 = -400 -> 250 + .1*(-650) = 185
        assert!((haurlan(&[250., -400.], 0.1)[1] - 185.).abs() < 1e-9);
        // short: (260 + 140)/2 = 200
        assert!((haurlan(&[260., 140.], 0.5)[1] - 200.).abs() < 1e-9);
    }
    #[test]
    fn wma_matches_the_smts_worksheet() {
        // SMTS p.7: ten NYSE closes, weights 1..10, 2949.90 / 55 = 53.63
        let v = [53.00, 52.50, 52.00, 52.35, 52.75, 53.00, 54.00, 54.25, 54.25, 54.45];
        assert!((wma(&v, 10)[9] - 2949.90 / 55.0).abs() < 1e-9);
    }
    #[test]
    fn trin_worked_example() {
        // p.37: 800/400 ÷ 5,000,000/2,000,000 = .80
        assert!(((800.0f64 / 400.0) / (5e6 / 2e6) - 0.8).abs() < 1e-12);
    }
}
