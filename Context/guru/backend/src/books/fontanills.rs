//! George A. Fontanills (with Tom Gentile) - four works with complete daily rules. Book ids:
//!   `fgitc`  The Index Trading Course (Wiley 2006; printed page = pdf page - 21): Volex bands (stop and reverse), 20-day
//!            ROC / ROC-average with ADX filter, QQQQ rising-volume system, 10/20 EMA crossover with two-close exit.
//!   `fgitsm` Interactive Technical Trading Seminar manual (Optionetics; chapter-page cites): Reverse VolEx, volume spike.
//!   `fgsmc`  The Stock Market Course (Wiley 2001): 50/200-day moving-average crossover.
//!   `fgtofd` Trading Options For Dummies (Wiley 2008): 20/50-day EMA crossover (XLF example).
//! Reading: claude-continuation-20261020/authors/fontanills/agents/. Seasonal rules repeat kpss / hsta; rotation, VIX,
//! expiration and option rules need data we do not hold (authors/fontanills/fontanills-REPORT.md).
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};
use crate::books::mpts::sma;

pub const REVIEWED: &[&str] = &["fgitc_volex", "fgitc_rocadx", "fgitc_qqqvol", "fgitc_ema1020", "fgitsm_revvolex", "fgitsm_volspike", "fgsmc_ma50_200", "fgtofd_ema20_50"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

macro_rules! note { ($s:expr) => { concat!($s, " [Declared conventions, not from the book] A run starts flat; history before the window builds the indicators; an open trade is marked at the window's last close; one unit, no commissions. Index, ETF and stock runs (US and India) are a declared application where the book used another market; cash-index shorts are hypothetical.") }; }

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    let live = |id, name, group, book, dir, vol, rules, notes| StrategyDef { id, name, group, session: 0, dir, status: Live, needs_volume: vol,
        needs_vix: false, needs_series: &[], book, rules, notes, claim: None };
    let held = |id, name, group, book, st, rules| StrategyDef { id, name, group, session: 0, dir: "Long", status: st, needs_volume: false,
        needs_vix: false, needs_series: &[], book, rules, notes: "Held after the 2026-10-24 reading (authors/fontanills/agents).", claim: None };
    vec![
        live("fgitc_volex", "Volex bands: buy a break above the upper band, sell short below the lower, stop and reverse", "Ch.9 · Volex (p.162-169)", "fgitc", "Long/Short", false,
            &["F|\"Upper band value = Current day high + (ATR x ATR multiple); Lower band value = Current day low - (ATR x ATR multiple)\" (p.165); \"Volex indicator settings of nine days with an ATR multiple of 1.3\" (p.166); ATR = \"the sum of 14 TR values divided by 14\" style simple average (p.164)",
              "E|\"A break to the upside initiates a long position ... A break of the lower band initiates a short sale\" (p.166)",
              "X|\"In the traditional Volex system, trades are exited either long or short when the Volex System triggers another signal in the opposite direction\" (p.167)"],
            note!("Today's bands are built from yesterday's high, low and 9-day average true range (a band from today's own high can never be broken today); a break fills at the band, or at the open if the day opens beyond it. A day that breaks both bands is skipped. The book takes signals from the S&P 500 index and trades SPY; here signal and trade are the same instrument. Gentile's point-and-figure exit (p.168) is not run: the page gives two different exit rules and no box size.")),
        live("fgitc_rocadx", "20-day ROC crossing its 5-day average with ADX above 18, long only", "Ch.9 · Momentum (p.171-176, Table 9.3)", "fgitc", "Long", false,
            &["F|\"The ROC is calculated by dividing the latest price by the price from n periods ago\" (p.385)",
              "E|\"Buy when 20-day ROC crosses up above the 5-day SMA on the ROC. ADX must be above 18\" (Table 9.3, p.172)",
              "X|\"Sell when 20-day ROC crosses down below the 5-day SMA on the ROC. No ADX requirement\" (Table 9.3, p.172)"],
            note!("The base rule of Table 9.3. ADX length is not given: Wilder's 14-day ADX [declared]. Signals and fills at the close [book silent]. Equal ROC and average is no cross. The book tested S&P 100 stocks 1980-1998 with $50,000 accounts; the Table 9.7 and SPY versions (13/20, rising ADX, 13-15% stops) are not run.")),
        live("fgitc_qqqvol", "QQQQ volume system: buy an up close on two days of rising volume, exit at +3.5% or a rise on falling volume", "Ch.10 · Volume (p.187-188)", "fgitc", "Long", true,
            &["E|\"enter the QQQQ when it has moved higher in price and volume has been rising for two consecutive days - in other words, the volume is greater than yesterday, and yesterday's volume is higher than the day before. In addition, the close of the current day is greater than the close of the day before\" (p.187)",
              "X|\"We exit the trade when either (1) the QQQQ has moved 3.5 percent higher or (2) the QQQQ rises on falling volume. If the QQQQ moves higher but volume has declined for two consecutive days, the position is closed\" (p.187)"],
            note!("Entry at the signal close. From the next day a sell limit 3.5% above the entry fills when the high reaches it (at the open if the day opens above it); otherwise the trade closes at the close of a day that rises on two days of falling volume. No stop ('losses are allowed to run', p.187). One trade at a time; no entry on an exit day. Needs actual volume. Book: QQQQ, 119 trades, 64.7% winners (not claimed here).")),
        live("fgitc_ema1020", "10/20-day EMA crossover: buy next open, exit next open after two closes below the 10-day or a cross down, 15% stop", "Ch.14 · ETF trading (p.321-323)", "fgitc", "Long", false,
            &["E|\"When the 10-day exponential moving average (EMA) crosses up above the 20-day EMA, the trader buys the underlying ETF at the open on the next trading day\" (p.321)",
              "X|\"When the ETF price closes below the 10-day EMA for two consecutive days or the 10-day EMA crosses down below the 20-day EMA, the trader exits the position at the open on the following day\" (p.321)",
              "X|GTC stop 15% below the entry price: \"$30.60 [$36 - ($36 x 0.15)]\" (p.322)"],
            note!("Standard exponential averages, 2/(n+1), started at the first close of the data file [declared: the book's own EMA description on p.44 is a weighted average]. The stop is live from the entry day after the open fill and fills at the stop or a gapping open. A signal on the window's last bar is not filled. Book example: QQQQ bought 13 May 2005 at $36, sold 9 Jun 2005 at $37.66.")),
        live("fgitsm_revvolex", "Reverse VolEx: buy a close after the low breaks the lower band, sell short after the high breaks the upper band, 10% stop", "Ch.12 · Reverse VolEx (p.12-1 to 12-18)", "fgitsm", "Long/Short", false,
            &["F|\"Top Line Value = current intervals High + (ATR x ATR Multiple); Bottom Line Value = current interval's Low - (ATR x ATR Multiple) ... we will be using a 1.3 multiple\"; Volex (9, 1.300) (p.12-2, 12-3)",
              "E|\"Bullish VolEx Entry: Daily Low < VolEx Low; Bearish VolEx Entry: Daily High > VolEx High\"; \"Enter at Close of the signal day\" (p.12-4, 12-9)",
              "X|\"The Reverse VolEx System will use these very entry signals as exit signals ... always have a trade in play\"; \"If the trade moves against us by more than 10%, we will exit the trade\": \"Current Price <= Entry Price - (Entry Price * 10%)\" (p.12-4, 12-5)"],
            note!("The band is yesterday's (today's low can never be below a band built from today's own low). The 10% stop is checked on the close and exits at that close; the book prints the long formula and applies the stop to shorts too (p.12-12). A day that breaks both bands is skipped; after a stop exit there is no new entry the same day. Repeat signals in the same direction add positions in the book (p.12-4); this one-unit ledger ignores them. Book: DIA 2003-2005, 16 trades, 56.25% winners (not claimed).")),
        live("fgitsm_volspike", "Volume spike: buy a close above the spike day's high, exit on a break of its low (mirror short)", "Ch.10 · Volume spikes (p.10-1 to 10-24)", "fgitsm", "Long/Short", true,
            &["F|\"300%+ volume spike\" (p.10-1); scan: \"Volume of the current day Is Greater Than Moving Average of the current day * 3\", \"MA Periods ... (30 days)\" (p.10-18 to 10-20)",
              "E|\"Bullish entry: Close above high of day\"; \"Bearish entry: Close below low of day\" (p.10-1)",
              "X|\"Exit at break below low of day\" (long); \"Exit at break above high of day\" (short) (p.10-1)"],
            note!("'Day' = the spike day. The spike is volume above 3 x the 30-day average volume including that day. Entry at the close of the first later day that closes beyond the spike day's range; the exit is a stop at the spike day's low (high), filled at the stop or a gapping open, from the day after entry. A later spike replaces the levels while flat; spikes during a trade are ignored; levels are used up by an entry. The $9 price and 300,000-share filters of the scan are not applied. No profit exit in the book ('volume spikes really don't provide clear-cut targets', p.10-9). Needs actual volume.")),
        live("fgsmc_ma50_200", "50-day / 200-day moving-average crossover, long or cash", "Technical analysis (p.94, 256-257)", "fgsmc", "Long", false,
            &["E|\"My favorites are the 50-day and 200-day moving averages ... if it moves above, it's a buy signal\" (p.257)",
              "X|\"If the 50-day drops below the 200-day, it's a sell signal\" (p.257)"],
            note!("Simple averages ('most technicians seem to favor the simple moving average', p.256); signal and fill at the close [book silent]; long or cash (a sell signal is not said to open a short). Equal averages keep the position.")),
        live("fgtofd_ema20_50", "20/50-day EMA crossover, entry and exit at the next open (XLF example)", "Ch.5 · Technical analysis (p.100-101)", "fgtofd", "Long", false,
            &["E|\"enter a long position in XLF (financials ETF) the day after the 20-day EMA crosses up over the 50-day EMA\" (p.100)",
              "X|\"One exit signal includes a cross of the 20-day EMA down below the 50-day EMA\" (p.100); example exit \"at the next open\" (p.101)"],
            note!("Exponential averages 2/(n+1) started at the first close of the file. The book's loss exit is a support line drawn by eye ($36.58) and is not run. A signal on the window's last bar is not filled.")),
        held("fgitc_rotation", "Pure Momentum sector / country fund rotation (top five by 240-day gain)", "Ch.11 (p.249-252)", "fgitc",
            NeedsData("ranks Fidelity sector funds or country ETFs each month; the vault holds neither universe"), &["E|Hold the five funds that advanced most over 240 days"]),
        held("fgitc_seasonal", "Presidential cycle, Kaeppel seasonal windows, Santa Claus rally", "Ch.10 (p.213-216)", "fgitc",
            NotTestable("the same rules as Kaeppel's Seasonal Stock Market Trends (kpss) and the Stock Trader's Almanac (hsta) already on the site; the book's turn-of-month window is the last 1 + first 4 days (kpss: last 2 + first 4)"), &["E|Buy the year before the election"]),
        held("fgitcwb_dia_expiry", "DIA option-expiration Friday short", "Workbook (p.118-119)", "fgitc",
            NeedsData("needs DIA prices (not in the vault) and is a classroom exercise 'created for this exercise (but not implemented)' from one week of data"), &["E|Short DIA at the close before expiration Friday after a down close"]),
        held("fgsmc_vix", "VIX above 30 buy, below 20 out", "p.292-294", "fgsmc",
            NeedsData("the book's VIX is the old S&P 100 (OEX) volatility index, now VXO; the vault holds the newer VIX, a different index"), &["E|\"When the VIX is high (>30), it's time to buy\" (p.293)"]),
        held("fgtofd_sector_tilt", "Select Sector SPDR tilt (weekly 3-month rank, SPY 50/200 filter)", "Ch.13 (p.230-234)", "fgtofd",
            NeedsData("needs the nine Select Sector SPDR ETFs (XLB ... XLY), not in the vault"), &["E|Hold the top-ranked sector ETF"]),
        held("fgitsm_elliott", "ProfitSource Elliott-wave trades", "Seminar manual", "fgitsm", NotTestable("depends on proprietary Elliott-wave counts"), &["E|Proprietary"]),
        held("fg_options", "Option strategies (straddles, spreads, covered calls ...)", "Options Course, Trading Options For Dummies, seminar manuals", "fgtofd",
            NeedsData("needs option prices (none in the vault)"), &["E|Option positions"]),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "fgitc_volex" => ("fgitc_volex", "Fontanills Volex", "9, 1.3"),
        "fgitc_rocadx" => ("fgitc_rocadx", "Fontanills ROC/ADX", "20/5, ADX 18"),
        "fgitc_qqqvol" => ("fgitc_qqqvol", "Fontanills volume system", "3.5%"),
        "fgitc_ema1020" => ("fgitc_ema1020", "Fontanills 10/20 EMA", "10/20, 15%"),
        "fgitsm_revvolex" => ("fgitsm_revvolex", "Reverse VolEx", "9, 1.3, 10%"),
        "fgitsm_volspike" => ("fgitsm_volspike", "Volume spike", "3x 30-day"),
        "fgsmc_ma50_200" => ("fgsmc_ma50_200", "Fontanills 50/200", "50/200"),
        "fgtofd_ema20_50" => ("fgtofd_ema20_50", "Fontanills 20/50 EMA", "20/50"),
        _ if id.starts_with("fgitc_") || id.starts_with("fgitcwb_") || id.starts_with("fgitsm_") || id.starts_with("fgsmc_") || id.starts_with("fgtofd_") || id.starts_with("fg_") => ("fg_nt", "Fontanills (held)", "-"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(match id {
        "fgitc_volex" | "fgitsm_revvolex" => vec![p("n", "ATR days (book: 9)", 9., 2., 50., 1.), p("m", "ATR multiple (book: 1.3)", 1.3, 0.1, 5., 0.1)],
        "fgitc_rocadx" => vec![p("roc", "ROC days (book: 20)", 20., 2., 100., 1.), p("ma", "ROC average days (book: 5)", 5., 1., 50., 1.), p("adx", "ADX above (book: 18)", 18., 0., 60., 1.)],
        "fgitc_qqqvol" => vec![p("target", "Profit target % (book: 3.5)", 3.5, 0.5, 20., 0.5)],
        "fgitc_ema1020" => vec![p("s", "Short EMA (book: 10)", 10., 2., 100., 1.), p("l", "Long EMA (book: 20)", 20., 3., 250., 1.), p("stop", "Stop % (book: 15)", 15., 1., 50., 1.)],
        "fgitsm_volspike" => vec![p("mult", "Spike = volume above x times average (book: 3)", 3., 1.5, 10., 0.5), p("n", "Average volume days (book: 30)", 30., 5., 200., 1.)],
        "fgsmc_ma50_200" => vec![p("s", "Short average (book: 50)", 50., 2., 200., 1.), p("l", "Long average (book: 200)", 200., 3., 400., 1.)],
        "fgtofd_ema20_50" => vec![p("s", "Short EMA (book: 20)", 20., 2., 100., 1.), p("l", "Long EMA (book: 50)", 50., 3., 250., 1.)],
        _ => return None,
    })
}

pub fn ema(c: &[f64], n: usize) -> Vec<f64> {
    let k = 2. / (n as f64 + 1.);
    let mut out = vec![f64::NAN; c.len()];
    for i in 0..c.len() { out[i] = if i == 0 { c[0] } else { k * c[i] + (1. - k) * out[i - 1] }; }
    out
}

/// Simple n-day average of the daily true range (bar 0: high - low).
fn atr(x: &Ctx, n: usize) -> Vec<f64> {
    let tr: Vec<f64> = (0..x.c.len()).map(|i| if i == 0 { x.h[0] - x.l[0] } else {
        (x.h[i] - x.l[i]).max((x.h[i] - x.c[i - 1]).abs()).max((x.l[i] - x.c[i - 1]).abs()) }).collect();
    sma(&tr, n)
}

/// +1 / -1 by a > b / a < b, kept on ties; None until the first difference.
fn state(a: &[f64], b: &[f64]) -> Vec<Option<i8>> {
    let mut cur = None;
    a.iter().zip(b).map(|(&u, &v)| { if u.is_finite() && v.is_finite() && u != v { cur = Some(if u > v { 1 } else { -1 }); } cur }).collect()
}

fn trade(d: f64, i: usize, px: f64, kind: &'static str, sig: usize) -> Trade {
    Trade { dir: d, legs: vec![Leg { i, px, w: 1. }], exit_i: i, exit_px: px, reason: String::new(), entry_kind: kind, signal_i: sig, open: false, exits: vec![] }
}
fn close(mut t: Trade, i: usize, px: f64, sig: usize, why: &str, out: &mut Vec<Trade>) { t.exit_i = i; t.exit_px = px; t.signal_i = sig; t.reason = why.into(); out.push(t); }

fn vol(x: &Ctx) -> Vec<f64> { x.r.iter().map(|r| r.volume.unwrap_or(f64::NAN)).collect() }

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from > to || to >= x.r.len() { return None; }
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d);
    let ni = |k: &str, d: f64| g(k, d).round().max(1.) as usize;
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let mut out = vec![];
    let mut cur: Option<Trade> = None;
    let start = from.max(1);
    match id {
        "fgitc_volex" | "fgitsm_revvolex" => {
            let (a, m) = (atr(x, ni("n", 9.)), g("m", 1.3));
            let rev = id == "fgitsm_revvolex";
            for i in start..=to {
                if !a[i - 1].is_finite() { continue; }
                let (ub, lb) = (h[i - 1] + m * a[i - 1], l[i - 1] - m * a[i - 1]);
                let (up, dn) = (h[i] > ub, l[i] < lb);
                if !rev {
                    if up == dn { continue; }
                    let (d, px) = if up { (1., o[i].max(ub)) } else { (-1., o[i].min(lb)) };
                    if cur.as_ref().is_some_and(|t| t.dir == d) { continue; }
                    if let Some(t) = cur.take() { close(t, i, px, i, "opposite band broken", &mut out); }
                    cur = Some(trade(d, i, px, "stop", i));
                } else {
                    if let Some(t) = cur.take() {
                        let e = t.legs[0].px;
                        if (t.dir > 0. && c[i] <= e * 0.9) || (t.dir < 0. && c[i] >= e * 1.1) { close(t, i, c[i], i, "10% stop", &mut out); continue; }
                        cur = Some(t);
                    }
                    if up == dn { continue; }
                    let d = if dn { 1. } else { -1. };
                    if cur.as_ref().is_some_and(|t| t.dir == d) { continue; }
                    if let Some(t) = cur.take() { close(t, i, c[i], i, "opposite VolEx signal", &mut out); }
                    cur = Some(trade(d, i, c[i], "close", i));
                }
            }
        }
        "fgitc_rocadx" => {
            let n = ni("roc", 20.);
            let roc: Vec<f64> = (0..c.len()).map(|i| if i >= n { c[i] / c[i - n] } else { f64::NAN }).collect();
            let st = state(&roc, &sma(&roc, ni("ma", 5.)));
            let (_, adx) = crate::connors::dmi(x.r, 14);
            let lvl = g("adx", 18.);
            for i in start..=to {
                if !matches!((st[i], st[i - 1]), (Some(a), Some(b)) if a != b) { continue; }
                if st[i] == Some(1) { if cur.is_none() && adx[i] > lvl { cur = Some(trade(1., i, c[i], "close", i)); } }
                else if let Some(t) = cur.take() { close(t, i, c[i], i, "ROC crossed below its average", &mut out); }
            }
        }
        "fgitc_qqqvol" => {
            let v = vol(x);
            let tg = g("target", 3.5) / 100.;
            for i in start.max(2)..=to {
                if let Some(t) = cur.take() {
                    let tp = t.legs[0].px * (1. + tg);
                    if h[i] >= tp { close(t, i, o[i].max(tp), i, "3.5% target", &mut out); continue; }
                    if c[i] > c[i - 1] && v[i] < v[i - 1] && v[i - 1] < v[i - 2] { close(t, i, c[i], i, "rise on falling volume", &mut out); continue; }
                    cur = Some(t); continue;
                }
                if c[i] > c[i - 1] && v[i] > v[i - 1] && v[i - 1] > v[i - 2] { cur = Some(trade(1., i, c[i], "close", i)); }
            }
        }
        "fgitc_ema1020" => {
            let (es, el) = (ema(c, ni("s", 10.)), ema(c, ni("l", 20.)));
            let st = state(&es, &el);
            let stop_f = 1. - g("stop", 15.) / 100.;
            let mut pend_entry: Option<usize> = None; // signal bar
            let mut pend_exit: Option<usize> = None;
            for i in start..=to {
                if let (Some(s), true) = (pend_exit.take(), cur.is_some()) { close(cur.take().unwrap(), i, o[i], s, "next open after the exit signal", &mut out); }
                if let Some(s) = pend_entry.take() { cur = Some(trade(1., i, o[i], "open", s)); }
                if let Some(t) = cur.take() {
                    let sp = t.legs[0].px * stop_f;
                    if l[i] <= sp { close(t, i, o[i].min(sp), i, "15% stop", &mut out); }
                    else {
                        let down = matches!((st[i], st[i - 1]), (Some(-1), Some(1)));
                        if (c[i] < es[i] && c[i - 1] < es[i - 1]) || down { if i < to { pend_exit = Some(i); } }
                        cur = Some(t);
                    }
                    continue;
                }
                if matches!((st[i], st[i - 1]), (Some(1), Some(-1))) && i < to { pend_entry = Some(i); }
            }
        }
        "fgitsm_volspike" => {
            let v = vol(x);
            let (mult, av) = (g("mult", 3.), sma(&v, ni("n", 30.)));
            let mut lv: Option<(f64, f64)> = None;
            for i in start..=to {
                if let Some(t) = cur.take() {
                    let (sh, sl) = lv.unwrap();
                    if t.dir > 0. && l[i] < sl { close(t, i, o[i].min(sl), i, "spike-day low broken", &mut out); lv = None; }
                    else if t.dir < 0. && h[i] > sh { close(t, i, o[i].max(sh), i, "spike-day high broken", &mut out); lv = None; }
                    else { cur = Some(t); }
                    continue;
                }
                if let Some((sh, sl)) = lv {
                    if c[i] > sh { cur = Some(trade(1., i, c[i], "close", i)); continue; }
                    if c[i] < sl { cur = Some(trade(-1., i, c[i], "close", i)); continue; }
                }
                if av[i].is_finite() && v[i] > mult * av[i] { lv = Some((h[i], l[i])); }
            }
        }
        "fgsmc_ma50_200" | "fgtofd_ema20_50" => {
            let st = if id == "fgsmc_ma50_200" { state(&sma(c, ni("s", 50.)), &sma(c, ni("l", 200.))) } else { state(&ema(c, ni("s", 20.)), &ema(c, ni("l", 50.))) };
            let nx = id == "fgtofd_ema20_50";
            for i in start..=to {
                if !matches!((st[i], st[i - 1]), (Some(a), Some(b)) if a != b) { continue; }
                if nx && i == to { continue; }
                let (fi, px, kind) = if nx { (i + 1, o[i + 1], "open") } else { (i, c[i], "close") };
                if st[i] == Some(1) { cur = Some(trade(1., fi, px, kind, i)); }
                else if let Some(t) = cur.take() { close(t, fi, px, i, "short average crossed below the long", &mut out); }
            }
        }
        _ => return None,
    }
    if let Some(mut t) = cur { t.exit_i = to; t.exit_px = c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true; out.push(t); }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn catalogue_admits_exactly_the_reviewed_ids() {
        let live: Vec<_> = catalogue().into_iter().filter(|d| d.status == Status::Live).map(|d| d.id).collect();
        assert_eq!(live, REVIEWED);
    }
}

// Instrument admission: Fontanills 2026-10-24 block, moved from instrument_policy.rs on 31 Oct 2026 (lane ST).
// Admission only for the module REVIEWED lists (authors/fontanills/admission/SOURCE-CONTRACT.md); kept in one block to merge.
// Runs on actual indices, ETFs and stocks, US and India (index_etf_or_stock, nifty50 basket, declared application).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| matches!(def.book,"fgitc"|"fgitsm"|"fgsmc"|"fgtofd") && (crate::books::fontanills::reviewed(def.id)),
    label: "Fontanills and Gentile show these rules on SPY, the S&P 500, QQQQ, DIA, XLF and US stocks; other index, ETF and stock runs (US and India) are a declared application, not the authors' test.",
    basis: "George A. Fontanills and Tom Gentile: The Index Trading Course (2006) Volex bands, ROC/ADX momentum, QQQQ volume and 10/20 EMA systems (p.162-188, 321-323); Interactive Technical Trading Seminar manual Reverse VolEx and volume-spike systems (Ch.10, 12); The Stock Market Course 50/200 crossover (p.257); Trading Options For Dummies 20/50 EMA crossover (p.100-101); reviewed in authors/fontanills/admission/SOURCE-CONTRACT.md with independent chronological ledgers. Band-day, fill, ADX length and EMA start are declared readings; one unit, no pyramiding. Cash-index shorts are hypothetical exposure.",
};
