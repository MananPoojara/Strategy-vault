//! Steve Palmquist, "Effective Trading Strategies" (daisydogger.com seminar book, AIQ-tested 2002-2008).
//! Stock systems with next-open entry and a fixed holding time. Run per instrument (NSE basket stocks
//! basket:<list>:<SYMBOL>, or an index/ETF with volume); the book scanned ~2,500 US stocks. Dollar prices and
//! share volumes are kept literally (India = instrument swap). Sources and checks: library/palets/.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

const BOOK: &str = "palets";
/// Admitted ids (lane C, authors/palmquist/admission): US instruments only (dollar price and share-volume filters).
pub const REVIEWED_LANEC: &[&str] = &["palets_abovebb_basic", "palets_abovebb", "palets_belowbb_inverse", "palets_belowbb", "palets_dvpullback", "palets_avbottom", "palets_avtop"];
pub fn reviewed_lanec(id: &str) -> bool { REVIEWED_LANEC.contains(&id) }
const NOTE_STOCKS: &str = "Book universe: about 2,500 US stocks in AIQ Trading Expert Pro; results are portfolio statistics of every signal (annualized ROI, win %). This engine runs one instrument at a time, so the book's numbers cannot be reproduced; the rules are run per stock (NSE basket tickers) or per index/ETF that has volume (NIFTY 50 has no volume -> no trades). $ thresholds and share-volume thresholds are kept literally in the instrument's own currency/units. One position at a time; a signal while a position is open is ignored.";
const BB: &str = "F|Bollinger Bands: 20-day simple moving average of the close +/- 2 standard deviations (p.5: \"around the 20-day moving average\", \"95% of the time the prices will be contained within the Bollinger Bands\" = 2 sigma; population SD as in Bollinger's definition)";
const HOLD: &str = "X|\"Hold the position for 3 days, then close the position on the next open\" (p.6, p.75): enter at the open of day 1, exit at the open of day 4 (example p.7: in 7/1, held 7/1-7/3, out at the next open)";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, dir, status, rules, notes| StrategyDef {
        id, name, group, session: 0, dir, status, needs_volume: true, needs_vix: false, needs_series: &[],
        book: BOOK, rules, notes, claim: None,
    };
    vec![
        d("palets_abovebb_basic", "AboveBB (initial, no volume filter)", "AboveBB · shorting extensions above the upper band", "Short", Status::Live,
            &["E|\"If today's high was more than 4% above the upper Band, and\" (p.6)", "F|\"The close is greater than $10, and\" (p.6)",
              "F|\"The average volume is at least 300,000 shares, then\" (p.6). Read as the 21-day SMA of volume, the only volume average this rule set defines (p.26, p.75 put it on the line above) [reading]",
              BB, "E|\"Enter a short position at the opening tomorrow\" (p.6)", HOLD],
            NOTE_STOCKS),
        d("palets_abovebb", "AboveBB (final, below-average volume)", "AboveBB · shorting extensions above the upper band", "Short", Status::Live,
            &["E|\"If today's high was more than 4% above the upper Band, and\" (p.26, p.75)",
              "F|\"The volume today was below the 21-day simple moving average, and\" (p.26) / \"The volume is less than the 21-day simple moving average\" (p.75)",
              "F|\"The close is greater than $10, and\" (p.26)", "F|\"The average volume is at least 300,000 shares\" (p.26) = 21-day SMA of volume [reading]",
              BB, "E|\"Enter a short position at the opening tomorrow\" (p.26)", HOLD,
              "F|Book also tested holding 3-7 days (p.56), 3% / 5% extensions (p.67) and volume < 90% of average (p.68): p.hold, p.ext, p.vol_pct"],
            NOTE_STOCKS),
        d("palets_abovebb_5x20", "AboveBB with the 5x20 MA market filter", "AboveBB · shorting extensions above the upper band", "Short",
            Status::NotTestable("market filter not exactly defined: p.52-53 '5 MA below the 20 MA' of the NASDAQ without saying simple or exponential, while the same seminar defines the 5x20 filter elsewhere as a 5-period EMA of highs vs a 20-period MA of lows (p.93) or of closes (p.128)"),
            &["F|Only take AboveBB trades when the market's 5 MA is below its 20 MA (NASDAQ chart drawn red) (p.52-55)", "E|AboveBB rules (p.75)"],
            "Printed: 2002-2008 ROI 79 -> 232, win 55% -> 62% with the filter (p.55); holding 3-7 days table p.56."),
        d("palets_belowbb_inverse", "BelowBB as the inverse of AboveBB", "BelowBB · longs below the lower band", "Long", Status::Live,
            &["E|\"If today's low was more than 4% below the lower Band, and\" (p.78)", "F|\"The volume is less than the 21-day simple moving average, and\" (p.78); p.vol_above=1 = the p.80 test \"requiring the volume to be above average\"",
              "F|\"The close is greater than $10, and\" (p.78)", "F|\"The average volume is at least 300,000 shares\" (p.78) = 21-day SMA of volume [reading]",
              BB, "E|\"Enter a long position at the opening tomorrow\" (p.78)", HOLD],
            "The book reports this inversion is poor: under 50% winners, negative annualized ROI (p.79). Kept because the rules are exact."),
        d("palets_belowbb", "BelowBB (revised definition)", "BelowBB · longs below the lower band", "Long", Status::Live,
            &["E|\"If today's close was more than 5% below the lower Band, and\" (p.87)", "F|\"The close is greater than $5, and\" (p.87)",
              "F|\"The average volume is at least 300,000 shares, and\" (p.87) = 21-day SMA of volume [reading]",
              "F|\"The close is in the bottom 20% of the days range, and\" (p.87): (close - low) / (high - low) <= 0.20; a zero-range day never qualifies",
              "F|\"The open is >= yesterday's close, then\" (p.87)", BB, "E|\"Enter a long position at the opening tomorrow\" (p.87)", HOLD,
              "F|Variants from the text: p.82 version (4% close extension, no range / gap filters): p.ext=4 p.range_pct=100 p.nogap=0; extension table 3-7% (p.86); holding 3-6 days (p.92)"],
            "Printed (2,500 stocks 2002-2008, AIQ): ext 3%: 56.8% win, ROI 162, 3195 trades; 4%: 58.9%, 232, 1998; 5%: 60.5%, 292, 1288; 6%: 62.2%, 370, 860; 7%: 62.6%, 407, 603 (p.86). Hold 3/4/5/6 days: 60.5/61.5/56.5/54.1% (p.92). Not reproducible per instrument."),
        d("palets_belowbb_5x20", "BelowBB with the 5x20 market filter", "BelowBB · longs below the lower band", "Long",
            Status::NotTestable("filter half-defined: '5-period exponential moving average of the highs' vs 'the 20-period moving average of the lows' (p.93) does not say simple or exponential for the 20; the book prints only chart screenshots (p.95-96)"),
            &["F|Bearish market: 5-period EMA of the highs below the 20-period moving average of the lows (p.93)", "E|BelowBB rules (p.87)"],
            "p.95: bullish-filter results slightly lower; p.96: bearish-filter results slightly better (no numbers in the text)."),
        d("palets_dvpullback", "DVpullback (declining volume pullback, base definition)", "DVpullback · longs on declining-volume pullbacks", "Long", Status::Live,
            &["F|\"A DVpullback occurs if: 3 consecutive lower highs, and 3 consecutive days of lower volume, and The close is >5, and The 21-day simple average of the volume is > 200,000.\" (p.103). Read: on the setup day s, high[s]<high[s-1]<high[s-2]<high[s-3] and the same for volume (three days each lower than the day before, p.107 'the down arrows mark 3 consecutive days of lower highs and lower volume')",
              "E|\"A trigger occurs if: Today's high is greater than yesterday's high. If a pullback occurred yesterday, and a trigger occurs today, then Buy at the open tomorrow.\" (p.103)",
              "X|\"Hold for 3 days. Sell at the next open.\" (p.103): exit at the open of day 4 (example p.107: in 12/20/2007, out 12/26/2007)",
              "F|p.trig_vol=1: trigger-day volume larger than the previous day's (tested p.119: ROI 67 -> 79%)"],
            "Book: 2007, ~2,500 stocks: 2345 trades, 55% winners, ROI 42% (p.105, p.108). The up-trend, room-to-run and below-5MA filters (p.106-121) are listed separately as not testable."),
        d("palets_dvpullback_full", "DVpullback with up-trend, room-to-run and 5MA filters", "DVpullback · longs on declining-volume pullbacks", "Long",
            Status::NotTestable("filters not exactly defined: '2 weeks' windows (bars or calendar weeks?) and the day they are measured (p.106); 'at least 2 daily ranges below the upper Bollinger Band' does not say which price or which day's range (p.110-111); 'trigger below the 5MA' does not say which price (p.120)"),
            &["F|Up trend: highest high of the last 2 weeks > highest high of the previous 2 weeks and lowest low of the last 2 weeks > lowest low of the previous 2 weeks (p.106)", "F|Room to run: at least 2 daily ranges below the upper Bollinger Band (p.111)", "F|Pullback triggers below the 5-period SMA (p.120)", "E|DVpullback rules (p.103)"],
            "Printed 2007: up-trend filter 962 trades, 57%, ROI 48% (p.108); + room to run 362 trades, 67%, ROI 67% (p.111); + below 5MA 243 trades, ~64%, ROI 80% (p.121). Hold 2-6 days p.112. Market-period tables p.123-125, 2000-2008 test p.127."),
        d("palets_dvpullback_5x20", "DVpullback with the 5x20 market filter", "DVpullback · longs on declining-volume pullbacks", "Long",
            Status::NeedsData("market index series aligned to the stock: the filter is '5-period exponential average of the highs above the 20-period simple average of the close' of the market (NASDAQ; India: NIFTY 50) (p.128). The engine passes no market OHLC to a stock run (would need series MKT.High and MKT.Close)."),
            &["F|Market 5-period EMA of the highs above the market 20-period SMA of the close (p.128)", "E|DVpullback rules (p.103)"],
            "Printed 2000-2008: win 57% -> 59%, ROI 75 -> 91% with the filter (p.129)."),
        d("palets_dvretrace", "DVretrace (declining volume retracement, short)", "DVretrace · shorts on declining-volume bounces", "Short",
            Status::NotTestable("first condition 'The stock is in a downtrend' is never defined (p.135, p.148); dropping it would change the system"),
            &["F|\"The stock is in a downtrend, and 3 consecutive higher lows, and 3 consecutive days of lower volume, and The close is >5, and The 21 day simple average of the volume is > 200,000\" (p.148)",
              "E|\"A trigger occurs if: Today's low is less than yesterday's low, and Today's volume is larger than yesterdays volume. ... Short at the open tomorrow.\" (p.148)", "X|\"Hold for 3 days. Buy to cover at the next open.\" (p.148)"],
            "Printed: 2002-2008 1800+ trades, 54% winners; with 5x20 filter 913 trades, 59%, ROI 63 -> 122% (p.145-146)."),
        d("palets_avbottom", "AVBottom (accumulation near lows)", "AVBottom / AVTop · volume near annual extremes", "Long", Status::Live,
            &["F|\"Strong accumulation occurs if: Today's close is greater than yesterdays close, and Today's volume is more than 150% of the 21-day simple moving average of the volume.\" (p.151)",
              "F|\"AVBottom occurs if: Today's close is >$5, and Average volume is > 200,000, and At least three of the last 10 sessions have shown strong accumulation, and Today's close is less than the lowest value in the previous 200 days, starting 5 days ago.\" (p.151). Read: average volume = the 21-day SMA defined above; last 10 sessions include today; lowest value = lowest close of bars t-5 ... t-204 (AIQ LoVal(close,200,5) form) [reading]",
              "E|\"When an AVBottom occurs a position is entered at the next open\" (p.151)",
              "X|\"held for 4 days, then sold at the following open. The entry is day 1 of the 4 day holding period.\" (p.151): exit at the open of day 5",
              "F|Tested variants: p.dist_max=3 or 2 limits strong distribution days (down, volume at least 150% of average) in the last 10 sessions (p.154-155); p.hold=6 for trading ranges (p.166); p.min_close=25 (p.158); p.acc_pct=120/170 (p.156-157)"],
            "Printed: 2007: 250 trades, 61% winners, ROI 108% (p.152, p.154); 3 years: ROI 78% (p.162). Not reproducible per instrument."),
        d("palets_avtop", "AVTop (distribution near highs, final definition)", "AVBottom / AVTop · volume near annual extremes", "Short", Status::Live,
            &["F|\"Strong distribution occurs if: Today's close is less than yesterday's close, and Today's volume is more than 130% of the 21-day simple moving average of the volume.\" (p.177)",
              "F|\"AVTop occurs if: Today's close is >$25, and Average volume is > 200,000, and At least three of the last 10 sessions have shown strong distribution, and Today's close is higher than the Highest value in the previous 200 days, starting 5 days ago.\" (p.177); same readings as AVBottom",
              "E|\"When an AVTop occurs a position is entered at the next open\" (p.177); short (\"AVTop: short positions in topping stocks\", p.181)",
              "X|\"held for 4 days, then sold at the following open. The entry is day 1 of the 4 day holding period.\" (p.177)",
              "F|Initial definition (p.169): p.dist_pct=150 p.min_close=5; 4 distribution days: p.min_days=4 (p.179); hold 5-7 days (p.178)"],
            "Printed 2008: 197 trades, 72% winners, ROI 237% (p.172, p.179); 4 days: 55 trades, 81%, ROI 307% (p.179). Not reproducible per instrument."),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "palets_abovebb_basic" => ("palets_abovebb", "Palmquist AboveBB", "initial (p.6)"),
        "palets_abovebb" => ("palets_abovebb", "Palmquist AboveBB", "final, below-average volume (p.75)"),
        "palets_abovebb_5x20" => ("palets_abovebb", "Palmquist AboveBB", "5x20 market filter (not testable)"),
        "palets_belowbb_inverse" => ("palets_belowbb", "Palmquist BelowBB", "inverse of AboveBB (p.78)"),
        "palets_belowbb" => ("palets_belowbb", "Palmquist BelowBB", "revised (p.87)"),
        "palets_belowbb_5x20" => ("palets_belowbb", "Palmquist BelowBB", "5x20 market filter (not testable)"),
        "palets_dvpullback" => ("palets_dv", "Palmquist declining-volume", "DVpullback base (p.103)"),
        "palets_dvpullback_full" => ("palets_dv", "Palmquist declining-volume", "DVpullback + filters (not testable)"),
        "palets_dvpullback_5x20" => ("palets_dv", "Palmquist declining-volume", "DVpullback + 5x20 (needs market series)"),
        "palets_dvretrace" => ("palets_dv", "Palmquist declining-volume", "DVretrace (not testable)"),
        "palets_avbottom" => ("palets_av", "Palmquist AVBottom / AVTop", "AVBottom (p.151)"),
        "palets_avtop" => ("palets_av", "Palmquist AVBottom / AVTop", "AVTop (p.177)"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let bb = || vec![p("bb_n", "Bollinger SMA days", 20., 10., 30., 1.), p("bb_k", "Bollinger standard deviations", 2., 1., 3., 0.5)];
    let hold = |d| p("hold", "Holding days (exit at the next open)", d, 1., 10., 1.);
    let avg = |d| p("min_avgvol", "Minimum 21-day average volume (shares)", d, 0., 2_000_000., 50_000.);
    let mut v = match id {
        "palets_abovebb_basic" | "palets_abovebb" | "palets_belowbb_inverse" => {
            let mut v = vec![p("ext", "Extension beyond the band, % (high/low)", 4., 1., 10., 1.), p("min_close", "Minimum close ($)", 10., 0., 50., 1.), avg(300_000.), hold(3.)];
            if id != "palets_abovebb_basic" { v.push(p("vol_pct", "Volume must be below this % of its 21-day SMA", 100., 50., 150., 10.)); }
            if id == "palets_belowbb_inverse" { v.push(p("vol_above", "1 = volume ABOVE average instead (p.80 test)", 0., 0., 1., 1.)); }
            v
        }
        "palets_belowbb" => vec![p("ext", "Close below the lower band by more than, %", 5., 1., 10., 1.), p("min_close", "Minimum close ($)", 5., 0., 50., 1.), avg(300_000.),
            p("range_pct", "Close in the bottom % of the day's range (100 = off)", 20., 10., 100., 10.), p("nogap", "1 = open >= yesterday's close required", 1., 0., 1., 1.), hold(3.)],
        "palets_dvpullback" => vec![p("days", "Consecutive lower highs / lower-volume days", 3., 2., 5., 1.), p("min_close", "Minimum close ($)", 5., 0., 50., 1.),
            p("min_avgvol", "21-day average volume must exceed (shares)", 200_000., 0., 2_000_000., 50_000.), p("trig_vol", "1 = trigger-day volume > previous day (p.119)", 0., 0., 1., 1.), hold(3.)],
        "palets_avbottom" => vec![p("acc_pct", "Strong accumulation: volume more than % of 21-day SMA", 150., 100., 250., 10.), p("min_close", "Minimum close ($)", 5., 0., 50., 1.),
            p("min_avgvol", "21-day average volume must exceed (shares)", 200_000., 0., 2_000_000., 50_000.), p("min_days", "Strong accumulation days in the last 10", 3., 1., 6., 1.),
            p("lookback", "Close below the lowest close of N days", 200., 50., 250., 10.), p("offset", "... starting this many days ago", 5., 1., 20., 1.),
            p("dist_max", "Max strong distribution days in the last 10 (10 = off; book tested 3, 2)", 10., 0., 10., 1.), hold(4.)],
        "palets_avtop" => vec![p("dist_pct", "Strong distribution: volume more than % of 21-day SMA", 130., 100., 250., 10.), p("min_close", "Minimum close ($)", 25., 0., 100., 1.),
            p("min_avgvol", "21-day average volume must exceed (shares)", 200_000., 0., 2_000_000., 50_000.), p("min_days", "Strong distribution days in the last 10", 3., 1., 6., 1.),
            p("lookback", "Close above the highest close of N days", 200., 50., 250., 10.), p("offset", "... starting this many days ago", 5., 1., 20., 1.), hold(4.)],
        _ => return None,
    };
    if id.contains("bb") { v.extend(bb()); }
    Some(v)
}

/// Simple average from a fresh window sum on every bar (no running-sum drift at exact ties).
fn sma(v: &[f64], n: usize) -> Vec<f64> {
    (0..v.len()).map(|i| if n > 0 && i + 1 >= n { v[i + 1 - n..=i].iter().sum::<f64>() / n as f64 } else { f64::NAN }).collect()
}

/// Bollinger bands: SMA(n) +/- k * population standard deviation of the last n closes.
fn bands(c: &[f64], n: usize, k: f64) -> (Vec<f64>, Vec<f64>) {
    let m = sma(c, n);
    let (mut up, mut lo) = (vec![f64::NAN; c.len()], vec![f64::NAN; c.len()]);
    for i in n.saturating_sub(1)..c.len() {
        if !m[i].is_finite() { continue; }
        let sd = (c[i + 1 - n..=i].iter().map(|x| (x - m[i]).powi(2)).sum::<f64>() / n as f64).sqrt();
        up[i] = m[i] + k * sd;
        lo[i] = m[i] - k * sd;
    }
    (up, lo)
}

/// Next-open entry, exit at the open `hold` bars after entry; one position at a time; a signal on bar i
/// needs bar i+1 inside the window. A position still open at `to` is marked at the close of `to`.
fn hold_trades(x: &Ctx, from: usize, to: usize, dir: f64, hold: usize, sig: impl Fn(usize) -> bool) -> Vec<Trade> {
    let mut out = vec![];
    let mut i = from;
    while i < to {
        if !sig(i) { i += 1; continue; }
        let e = i + 1;
        let (xi, open) = if e + hold <= to { (e + hold, false) } else { (to, true) };
        out.push(Trade { dir, legs: vec![Leg { i: e, px: x.o[e], w: 1. }], exit_i: xi, exit_px: if open { x.c[to] } else { x.o[xi] },
            reason: if open { "open".into() } else { format!("{hold}-day hold, next open") }, entry_kind: "open",
            signal_i: if open { to } else { xi - 1 }, open, exits: vec![] });
        i = xi; // next signal can come on the exit bar at the earliest
    }
    out
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if from > to || to >= x.r.len() { return None; }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let v: Vec<f64> = x.r.iter().map(|r| r.volume.unwrap_or(f64::NAN)).collect();
    let av = sma(&v, 21);
    let hold = g("hold").round().max(1.) as usize;
    let (minc, minav) = (g("min_close"), g("min_avgvol"));
    let liq = |i: usize, strict: bool| x.c[i] > minc && av[i].is_finite() && if strict { av[i] > minav } else { av[i] >= minav };
    let out = match id {
        "palets_abovebb_basic" | "palets_abovebb" | "palets_belowbb_inverse" | "palets_belowbb" => {
            let (up, lo) = bands(&x.c, g("bb_n").round().max(2.) as usize, g("bb_k"));
            let ext = g("ext") / 100.;
            match id {
                "palets_abovebb_basic" => hold_trades(x, from, to, -1., hold, |i| x.h[i] > up[i] * (1. + ext) && liq(i, false)),
                "palets_abovebb" => {
                    let vp = g("vol_pct") / 100.;
                    hold_trades(x, from, to, -1., hold, |i| x.h[i] > up[i] * (1. + ext) && v[i] < vp * av[i] && liq(i, false))
                }
                "palets_belowbb_inverse" => {
                    let (vp, above) = (g("vol_pct") / 100., g("vol_above") >= 0.5);
                    hold_trades(x, from, to, 1., hold, |i| x.l[i] < lo[i] * (1. - ext) && (if above { v[i] > av[i] } else { v[i] < vp * av[i] }) && liq(i, false))
                }
                _ => {
                    let (rp, nogap) = (g("range_pct") / 100., g("nogap") >= 0.5);
                    hold_trades(x, from, to, 1., hold, |i| i > 0 && x.c[i] < lo[i] * (1. - ext) && liq(i, false)
                        && (rp >= 1. || x.close_loc(i) <= rp) && (!nogap || x.o[i] >= x.c[i - 1]))
                }
            }
        }
        "palets_dvpullback" => {
            let k = g("days").round().max(1.) as usize;
            let tv = g("trig_vol") >= 0.5;
            let setup = |s: usize| s >= k && (s + 1 - k..=s).all(|j| x.h[j] < x.h[j - 1] && v[j] < v[j - 1]) && liq(s, true);
            hold_trades(x, from, to, 1., hold, |i| i > 0 && setup(i - 1) && x.h[i] > x.h[i - 1] && (!tv || v[i] > v[i - 1]))
        }
        "palets_avbottom" | "palets_avtop" => {
            let bottom = id == "palets_avbottom";
            let m = g(if bottom { "acc_pct" } else { "dist_pct" }) / 100.;
            let (need, lb, off) = (g("min_days").round() as usize, g("lookback").round().max(1.) as usize, g("offset").round().max(1.) as usize);
            let strong = |j: usize, up: bool| j > 0 && (if up { x.c[j] > x.c[j - 1] } else { x.c[j] < x.c[j - 1] }) && v[j] > m * av[j];
            let dmax = if bottom { g("dist_max").round() as usize } else { 10 };
            let dist150 = |j: usize| j > 0 && x.c[j] < x.c[j - 1] && v[j] >= 1.5 * av[j]; // p.154: "at least 150% of average"
            hold_trades(x, from, to, if bottom { 1. } else { -1. }, hold, |i| {
                if i + 1 < off + lb || i < 10 { return false; }
                let w = &x.c[i + 1 - off - lb..=i - off];
                let ext = if bottom { x.c[i] < w.iter().copied().fold(f64::INFINITY, f64::min) } else { x.c[i] > w.iter().copied().fold(f64::NEG_INFINITY, f64::max) };
                ext && liq(i, true) && (i - 9..=i).filter(|&j| strong(j, bottom)).count() >= need
                    && (!bottom || dmax >= 10 || (i - 9..=i).filter(|&j| dist150(j)).count() <= dmax)
            })
        }
        _ => return None,
    };
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn rows(n: usize) -> Vec<Row> {
        (0..n).map(|i| Row { ticker: "T".into(), date: format!("{:05}", i), open: 20., high: 20.5, low: 19.5, close: 20., volume: Some(1_000_000.) }).collect()
    }
    #[test]
    fn abovebb_shorts_next_open_and_covers_three_days_later() {
        let mut r = rows(60);
        r[40].high = 30.; r[40].close = 20.2; r[40].volume = Some(900_000.); // high far above the flat band, volume below average
        for (k, i) in (41..46).enumerate() { r[i].open = 21. + k as f64; }
        let x = Ctx::new(&r, &r, &[]);
        let ts = run("palets_abovebb", &x, 25, 59, &Params::new()).unwrap();
        assert_eq!(ts.len(), 1);
        assert_eq!((ts[0].dir, ts[0].legs[0].i, ts[0].legs[0].px), (-1., 41, 21.));
        assert_eq!((ts[0].exit_i, ts[0].exit_px), (44, 24.));
        // above-average volume on the signal day kills the final version but not the initial one
        r[40].volume = Some(2_000_000.);
        let x = Ctx::new(&r, &r, &[]);
        assert!(run("palets_abovebb", &x, 25, 59, &Params::new()).unwrap().is_empty());
        assert_eq!(run("palets_abovebb_basic", &x, 25, 59, &Params::new()).unwrap().len(), 1);
    }
    #[test]
    fn dvpullback_needs_three_lower_highs_and_volumes_then_a_higher_high() {
        let mut r = rows(40);
        for (k, i) in (20..24).enumerate() { r[i].high = 25. - k as f64; r[i].volume = Some(900_000. - 100_000. * k as f64); }
        r[24].high = 23.; // trigger: above 22
        let x = Ctx::new(&r, &r, &[]);
        let ts = run("palets_dvpullback", &x, 0, 39, &Params::new()).unwrap();
        assert_eq!(ts.len(), 1);
        assert_eq!((ts[0].legs[0].i, ts[0].exit_i), (25, 28));
    }
}
