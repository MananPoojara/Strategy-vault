//! Lars N. Kestner, "Quantitative Trading Strategies" (McGraw-Hill, 2003). Book id `lkqts`.
//! Source: research drive "Trading Knowledge I/Lars N.Kestner - Quantitative Trading Strategies/
//! Quantitative Trading Strategies_0071412395.pdf" (367 pp., text layer). Transcription, page refs, oracle and
//! verification: library/lkqts/.
//!
//! Book conventions used by every Chapter 6-9 and 12 strategy (p.139): "Signals are generated at the close of each
//! trading day and executed on the next trading day's opening price"; no commissions or slippage (p.139). Standard
//! deviation is the population form, divided by N (p.41, Fig. 2.2). "Average true range" = simple average of the true
//! range (p.64). Windows such as "the highest close of the past 40 days" include today (p.62, p.140).
//! Long/short systems whose text gives only entries are stop-and-reverse: the book states it for Chapter 7
//! ("no rules for exiting positions, except for an entry in the other direction", pp.145, 155, 160, 165) and its own
//! SPX/NDX tables prove it for the Chapter 8 systems (trades x average bars held ~= every bar of 1990-2001); see README.
//! Chapter 10 (S&P timing) measures close-to-close returns, so those rules fill at the close of the signal day.
//! The book tested 29 futures + 34 stocks/indices with volatility-scaled size; here one unit of the selected index.
use crate::connors::{rsi, sma, true_range, Claim, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "lkqts";

// ------------------------------------------------------------------ indicators

/// EMA with alpha = 2/(1+n) (p.57), seeded with the first value (TradeStation XAverage).
fn ema(v: &[f64], n: usize) -> Vec<f64> {
    let a = 2.0 / (1.0 + n as f64);
    let mut out = vec![f64::NAN; v.len()];
    let mut prev = f64::NAN;
    for i in 0..v.len() {
        if v[i].is_nan() {
            continue;
        }
        prev = if prev.is_nan() { v[i] } else { a * v[i] + (1.0 - a) * prev };
        out[i] = prev;
    }
    out
}

/// Population standard deviation of the last n one-day changes (c[k]-c[k-1], k = i-n+1..=i).
fn sd_chg(c: &[f64], n: usize) -> Vec<f64> {
    let mut out = vec![f64::NAN; c.len()];
    for i in n..c.len() {
        let d: Vec<f64> = (i + 1 - n..=i).map(|k| c[k] - c[k - 1]).collect();
        if d.iter().any(|x| !x.is_finite()) {
            continue;
        }
        let m = d.iter().sum::<f64>() / n as f64;
        out[i] = (d.iter().map(|x| (x - m).powi(2)).sum::<f64>() / n as f64).sqrt();
    }
    out
}

fn hh(v: &[f64], i: usize, n: usize) -> f64 {
    v[i + 1 - n..=i].iter().cloned().fold(f64::NEG_INFINITY, f64::max)
}
fn ll(v: &[f64], i: usize, n: usize) -> f64 {
    v[i + 1 - n..=i].iter().cloned().fold(f64::INFINITY, f64::min)
}

/// 14-day Slow %K = 3-day SMA of raw %K (raw = (C - LL)/(HH - LL), pp.66-67; TradeStation SlowK).
fn slow_k(x: &Ctx, n: usize) -> Vec<f64> {
    let len = x.c.len();
    let mut raw = vec![f64::NAN; len];
    for i in n - 1..len {
        let (h, l) = (hh(&x.h, i, n), ll(&x.l, i, n));
        if h > l {
            raw[i] = 100.0 * (x.c[i] - l) / (h - l);
        }
    }
    sma(&raw, 3)
}

/// Simple average true range (p.64).
fn atr_s(x: &Ctx, n: usize) -> Vec<f64> {
    sma(&true_range(x.r), n)
}

// ------------------------------------------------------------------ simulator

#[derive(Clone, Copy, PartialEq)]
enum Fill {
    NextOpen,
    Close,
}

fn new_trade(dir: f64, i: usize, px: f64, units: i32, kind: &'static str, sig: usize) -> Trade {
    Trade {
        dir,
        legs: (0..units.max(1)).map(|_| Leg { i, px, w: 1.0 }).collect(),
        exit_i: i,
        exit_px: px,
        reason: String::new(),
        entry_kind: kind,
        signal_i: sig,
        open: false,
        exits: Vec::new(),
    }
}

/// Target-position simulator. `tgt(i, pos, trade)` is evaluated on the close of bar i (from..=to) and returns the
/// wanted signed unit count plus the rule that fired; a change is filled at the next open (Fill::NextOpen) or at
/// this close (Fill::Close). Units > 1 in the same direction add legs (pyramiding); a smaller same-sign target is
/// never requested by this module.
fn run_target(
    x: &Ctx,
    from: usize,
    to: usize,
    fill: Fill,
    mut tgt: impl FnMut(usize, i32, Option<&Trade>) -> Option<(i32, &'static str)>,
) -> Vec<Trade> {
    let mut out = Vec::new();
    let mut cur: Option<Trade> = None;
    let mut pos: i32 = 0;
    for i in from..=to {
        if fill == Fill::NextOpen && i == to {
            break;
        }
        let Some((want, why)) = tgt(i, pos, cur.as_ref()) else { continue };
        if want == pos {
            continue;
        }
        let (k, px, kind) = match fill {
            Fill::NextOpen => (i + 1, x.o[i + 1], "open"),
            Fill::Close => (i, x.c[i], "close"),
        };
        if fill == Fill::Close && i == to && (pos == 0 || want.signum() == pos.signum()) {
            continue; // no new exposure on the last bar
        }
        if pos != 0 && (want == 0 || want.signum() != pos.signum()) {
            let mut t = cur.take().unwrap();
            t.exit_i = k;
            t.exit_px = px;
            t.signal_i = i;
            t.reason = why.into();
            out.push(t);
            pos = 0;
        }
        if want != 0 && !(fill == Fill::Close && i == to) {
            if pos == 0 {
                cur = Some(new_trade(want.signum() as f64, k, px, want.abs(), kind, i));
            } else if want.abs() > pos.abs() {
                let t = cur.as_mut().unwrap();
                for _ in 0..(want.abs() - pos.abs()) {
                    t.legs.push(Leg { i: k, px, w: 1.0 });
                }
            }
            pos = want;
        }
    }
    if let Some(mut t) = cur {
        t.exit_i = to;
        t.exit_px = x.c[to];
        t.reason = "open".into();
        t.signal_i = to;
        t.open = true;
        out.push(t);
    }
    out
}

/// Stop-and-reverse on entry signals only: sig[i] = +1 long entry, -1 short entry, 0 none.
fn sar(x: &Ctx, from: usize, to: usize, sig: &[i8], lw: &'static str, sw: &'static str) -> Vec<Trade> {
    run_target(x, from, to, Fill::NextOpen, |i, _, _| match sig[i] {
        1 => Some((1, lw)),
        -1 => Some((-1, sw)),
        _ => None,
    })
}

// ------------------------------------------------------------------ catalogue

macro_rules! sd {
    ($id:expr, $name:expr, $group:expr, $dir:expr, $status:expr, [$($r:expr),* $(,)?], $notes:expr, $claim:expr) => {
        StrategyDef { id: $id, name: $name, group: $group, session: 0, dir: $dir, status: $status,
            needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules: &[$($r),*], notes: $notes, claim: $claim }
    };
}

fn cl(trades: &'static str, win: &'static str, avg: &'static str, hold: &'static str, period: &'static str) -> Option<Claim> {
    Some(Claim { trades, win, avg, hold, period })
}

const IDX: &str = "Book tested 29 futures + 34 US stocks/indices 1/1/1990-12/31/2001 with $10,000/sigma(100) sizing; run here on one index (instrument swap). India: NIFTY 50 / NIFTY BANK / NIFTY MIDCAP 100 replace the US indices.";
const P90: &str = "SPX 1/1/1990-12/31/2001 (stock table)";

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    const G7: &str = "Ch.7 Popular strategies (pp.139-178)";
    const G8: &str = "Ch.8 New entries, exits, filters (pp.181-245)";
    const G6: &str = "Ch.6 Filtering (pp.117-124)";
    const G9: &str = "Ch.9 Relative-value strategies (pp.247-287)";
    const G10: &str = "Ch.10 Investing in the S&P 500 (pp.289-303)";
    const G12: &str = "Ch.12 New volatility breakout (pp.321-328)";
    let mut v = vec![
        // ---------------- Chapter 7
        sd!("lkqts_cb4020", "40-day entry / 20-day exit channel breakout", G7, "Long/Short", Live,
            ["E|\"enters long when a market's close is the highest close of the past 40 days\" (p.139)",
             "E|\"enters short when a market's close is the lowest close of the past 40 days\" (pp.139-140)",
             "X|\"Longs are exited if today's close is the lowest close of the past 20 days. Shorts are exited if today's close is the highest close of the past 20 days.\" (p.140)",
             "F|Signals at the close, executed at the next day's open (p.139)"],
            IDX, cl("71", "34%", "-$1/share", "53 / 19 bars (win / loss)", P90)),
        sd!("lkqts_ma1040", "10-day / 40-day dual moving average crossover", G7, "Long/Short", Live,
            ["E|\"Long entries are established if the 10-day average crosses above the 40-day average\" (simple averages, p.145)",
             "E|\"Short entries are established if the 10-day average crosses below the 40-day average\" (p.145)",
             "X|\"Positions are exited on an entry signal in the opposite direction\" (p.145)"],
            IDX, cl("97", "30%", "$1/share", "59 / 19 bars", P90)),
        sd!("lkqts_mom20", "20-day momentum", G7, "Long/Short", Live,
            ["E|\"long entries are established if today's close is greater than the close 20 days ago\" (p.150)",
             "E|\"Short entries are established if today's close is less than the close 20 days ago\" (p.150)",
             "X|Reverse on the opposite entry (table: \"exit if today's close < close of 20 days ago\", p.151)"],
            IDX, cl("298", "31%", "$0/share", "20 / 6 bars", P90)),
        sd!("lkqts_mom80", "80-day momentum", G7, "Long/Short", Live,
            ["E|\"long entries are established if today's close is greater than the close 80 days ago\" (p.150)",
             "E|\"Short entries are established if today's close is less than the close 80 days ago\" (p.150)",
             "X|Reverse on the opposite entry (p.157 table)"],
            IDX, cl("149", "31%", "$1/share", "47 / 8 bars", P90)),
        sd!("lkqts_volbo", "Volatility breakout (2 sigma of 100 price changes)", G7, "Long/Short", Live,
            ["E|\"enter long if today's price change is greater than twice the standard deviation of price changes over the past 100 days\" (p.155)",
             "E|\"enter short if today's price change is less than negative two times the standard deviation of price changes over the past 100 days\" (p.155)",
             "X|\"no rules for exiting positions, except for an entry in the other direction\" (p.155)"],
            IDX, cl("89", "30%", "-$3/share", "44 / 28 bars", P90)),
        sd!("lkqts_stoch14", "14-day Slow %K stochastic 20/80", G7, "Long/Short", Live,
            ["E|\"Long entries are established when today's 14-day Slow %K stochastic falls below 20 and then crosses back above 20\" (p.155)",
             "E|\"Short entries are established when today's 14-day Slow %K stochastic rises above 80 and then crosses below 80\" (pp.155, 160)",
             "X|\"no rules for exiting positions, except for an entry in the other direction\" (p.160)",
             "F|Slow %K = 3-day SMA of raw %K = (C - lowest low 14)/(highest high 14 - lowest low 14) (pp.66-67)"],
            IDX, cl("90", "64%", "-$1/share", "18 / 61 bars", P90)),
        sd!("lkqts_rsi14", "14-day RSI 35/65", G7, "Long/Short", Live,
            ["E|\"Long entries are established if today's 14-day RSI falls below 35\" (p.165)",
             "E|\"Short entries are established if today's 14-day RSI rises above 65\" (p.165)",
             "X|\"no rules for exiting positions, except for an entry in the other direction\" (p.165)",
             "F|RSI = 100 - 100/(1+U/D) (p.66); Wilder smoothing (the book cites Wilder 1978)"],
            IDX, cl("36", "69%", "$0/share", "49 / 160 bars", P90)),
        sd!("lkqts_macd", "MACD 12/26/9 crossover", G7, "Long/Short", Live,
            ["F|MACD = 12-day EMA of close - 26-day EMA of close; signal = 9-day EMA of MACD; EMA alpha = 2/(1+days) (pp.57, 68, 170)",
             "E|\"Long entries are established when the MACD rises above the signal line\" (p.170)",
             "E|\"Short entries are established when the MACD falls below the signal line\" (p.170)",
             "X|Reverse on the opposite entry (GE example: \"our short positions are exited and a long position is established\", p.175)"],
            IDX, cl("260", "30%", "-$2/share", "21 / 7 bars", P90)),
        // ---------------- Chapter 8
        sd!("lkqts_kma", "Kestner's moving average system (20 & 80 day)", G8, "Long/Short", Live,
            ["F|Sub-system n (n = 20 and 80): long when the close > n-day SMA of highs and the n-day SMA of closes > its value x days ago; short when the close < n-day SMA of lows and the n-day SMA of closes < its value x days ago (p.183)",
             "F|x = 1: the GE example compares \"the 20-day moving average is greater than the prior day's value\" (p.183); the prose says x is \"typically\" the MA length. x = 1 reproduces the book's SPX/NDX rows (80/52 trades vs 73/52 printed; x = n gives 97/58) [fitted reading, p.187 table]",
             "E|\"if the new moving average strategy using a 20-day moving average is long and the new strategy using an 80-day moving average is also long, we will enter a long position\"; short when both are short (p.183)",
             "X|\"Long positions are exited if today's close falls below the 80-day simple moving average of lows. Short positions are exited if today's close rises above the 80-day simple moving average of highs.\" (p.183)"],
            "x_one=0 gives the prose's 'typically x = MA length' reading (non-default).",
            cl("73", "23%", "-$1/share", "105 / 16 bars", P90)),
        sd!("lkqts_sob", "Second order breakout", G8, "Long/Short", Live,
            ["F|Second Order Breakout statistic = +N if today's close is the highest close of the past N days, -N if the lowest of the past N days, searched up to 80 days (p.183)",
             "E|\"If today's Second Order Breakout statistic is both the highest value over the past 40 days and greater than 20, then long positions are established\" (p.190)",
             "E|\"If today's Second Order Breakout statistic is both the lowest value over the past 40 days and less than -20, then short positions are established\" (p.190)",
             "X|Reverse on the opposite entry (INTC example, p.190)"],
            "When today's close equals yesterday's close the day is both a 2-day high and low; the larger count wins (0 on a tie).",
            cl("93", "31%", "-$3/share", "51 / 19 bars", P90)),
        sd!("lkqts_macdhr", "MACD histogram retracement", G8, "Long/Short", Live,
            ["F|Histogram = MACD(12,26) - 9-day EMA of MACD (p.191)",
             "E|Long: the histogram has been below -0.5 x the 20-day standard deviation of price changes since its last cross below zero, and \"retraces 25 percent of its minimum value since its last cross below zero\" (p.196)",
             "E|Short: the histogram has exceeded +0.5 x sigma(20) since its last cross above zero and retraces 25% of its maximum since that cross (p.196)",
             "X|Reverse on the opposite entry (KO example, p.196)"],
            "Retrace test: histogram > 0.75 x the minimum (long) or < 0.75 x the maximum (short), evaluated each close, once per swing.",
            cl("53", "62%", "$6/share", "46 / 71 bars", P90)),
        sd!("lkqts_dividx", "Divergence Index", G8, "Long/Short", NotTestable("the book gives entries only (DI = mom10 x mom40 / sigma40^2 < -10, direction of mom40) and no exit; its own SPX/NDX tables (36 trades x ~29 bars) show positions were closed by an unstated rule, so stop-and-reverse would be invented"),
            ["F|Divergence Index = 10-day momentum x 40-day momentum / (40-day standard deviation of price changes)^2 (p.197)",
             "E|\"Long entries are established when the Divergence Index is less than -10 and 40-day momentum is greater than zero. Sell signals are generated when the Divergence Index is less than -10 and 40-day momentum is less than zero.\" (p.197)",
             "X|No exit stated"],
            "Book SPX: 36 trades, 31% win, 63 / 14 bars - only ~1/3 of bars held, so not stop-and-reverse.", cl("36", "31%", "$6/share", "63 / 14 bars", P90)),
        sd!("lkqts_macs", "Moving average confluence method (MACS)", G8, "Long/Short", Live,
            ["F|For n = 1..20: add 5 when the n-day SMA > the 4n-day SMA; MACS ranges 0-100 (p.203)",
             "E|\"Long signals are generated when the MACS is 60 or above, and shorts are initiated when the MACS is 40 or below.\" (p.203)",
             "X|Reverse on the opposite entry (S&P 500 example, p.203)"],
            IDX, cl("58", "29%", "$1/share", "107 / 29 bars", P90)),
        sd!("lkqts_nei", "Normalized envelope indicator (NEI)", G8, "Long/Short", Live,
            ["F|NEI = (Close - average of past 50 closes)/standard deviation of past 50 price changes; past NEIs use today's sigma (p.213)",
             "F|Sort the last 50 NEIs: upper band = today's 50-day MA + 10th-ranked NEI, lower band = 50-day MA + 40th-ranked NEI (p.213); the NEI is in sigma units, so it is converted back to price (x today's sigma) - this reading gives SPX 98 trades / 65% win vs the book's 98 / 66% (as printed, unscaled: 157 / 39%) [fitted reading]",
             "E|\"I enter short when prices rise above and then fall below the upper NEI band. Long entries are initiated when prices fall below and then rise above the lower NEI band.\" (p.213)",
             "X|Reverse on the opposite entry (XOM example: \"Profits are taken and shorts entered\", p.213)"],
            "Option scale=0 adds the unitless NEI to the price MA exactly as printed (non-default; does not reproduce the book).",
            cl("98", "66%", "$5/share", "25 / 41 bars", P90)),
        sd!("lkqts_meo", "Multiple Entry Oscillator system", G8, "Long/Short", NotTestable("unit changes are listed only for upward/downward crosses of 55/70/85 and 45/30/15; the book's IP example also covers one short unit on a re-cross of 15, a reduction rule the text never states"),
            ["F|20-day Slow %K stochastic (p.213)",
             "E|crosses above 55/70/85: long 1/2/3 units; crosses below 45/30/15: short 1/2/3 units (p.213)"],
            "Only portfolio equity curves are printed (Figs. 8.21-8.22).", None),
        sd!("lkqts_adjsto", "Adjusted stochastic", G8, "Long/Short", Live,
            ["F|New Stochastic = (14-day Slow %K - 50) x (HH14 - LL14)/(HH100 - LL100) + 50 (p.219)",
             "E|\"We will take countertrend short positions when the adjusted stochastic rises above 65 and then falls back below 65. Countertrend long positions are entered when the adjusted stochastic declines below and then rises back above 35.\" (p.219)",
             "X|Reverse on the opposite entry (TXN example, p.219)"],
            IDX, cl("51", "65%", "-$7/share", "29 / 113 bars", P90)),
        sd!("lkqts_tiar", "Three in a row", G8, "Long/Short", Live,
            ["E|Buy: close > close 5 days ago; close 5 days ago > close 10 days ago; close 10 days ago > close 15 days ago (p.222)",
             "E|Short: the three comparisons reversed (p.227)",
             "X|Reverse on the opposite entry"],
            IDX, cl("78", "33%", "$0/share", "67 / 24 bars", P90)),
        StrategyDef { needs_volume: true, ..sd!("lkqts_volrev", "Volume reversal strategy", G8, "Long/Short", Live,
            ["F|Five-day absolute price change > 100-day standard deviation of price changes (p.227)",
             "F|Five-day average volume < 75% of \"the five-day average volume beginning 10 days prior\" (days t-10..t-6) (p.227)",
             "E|\"We enter long if the most recent five-day price change is negative while we enter short if the five-day price change is positive.\" (p.227)",
             "X|\"All entries are exited on the fifth day of the trade.\" - at the open of the fifth day (entry day = day 1), the book's only execution price (p.139)"],
            "Needs daily volume (index volume on SPX/NIFTY). Option exit_close=1 exits at the fifth day's close instead.",
            cl("26", "69%", "$6/share", "4 / 5 bars", P90)) },
        sd!("lkqts_saitta", "Saitta's support and resistance strategy", G8, "Long/Short", Live,
            ["F|Phase turns positive when the close rises above the 20-day SMA of highs, negative when it falls below the 20-day SMA of lows (p.233)",
             "F|When an uptrend starts, the lowest close of the prior downtrend is a bottom; when a downtrend starts, the highest close of the prior uptrend is a top (p.233)",
             "E|\"Long positions ... are entered when prices rise above a previous market top, while short positions are entered when prices fall below a previous market bottom\" (close vs the latest top/bottom, p.233)",
             "X|Reverse on the opposite entry"],
            "Book: \"lowest closing price (or low)\" - closes by default; option hl=1 uses highs/lows.",
            cl("62", "34%", "$1/share", "85 / 29 bars", P90)),
        sd!("lkqts_cb_atrstop", "Channel breakout with 2-ATR stop loss", G8, "Long/Short", Live,
            ["E|40-day entry / 20-day exit channel breakout (p.236)",
             "X|\"exits longs if prices fall below two times the 20-day average true range from the entry price. Short positions are exited if prices rise more than two times the 20-day average true range above the entry point.\" (p.237) - close vs entry -/+ 2 x ATR20 at the signal day, exit next open"],
            "Book prints portfolio totals only (Fig. 8.34).", None),
        sd!("lkqts_cb_ptake", "Channel breakout with profit taking (Strategy 2)", G8, "Long/Short", Live,
            ["E|40/20 channel breakout, but \"No new long entries are allowed unless five days have passed since the last 40-day high\" (no 40-day high on the 5 previous days); shorts likewise (p.242)",
             "X|20-day opposite extreme, plus \"exit if open profit is greater than three times the average true range of the past 20 days\" (close-based, p.242)"],
            "Portfolio Sharpe/K only (Fig. 8.35).", None),
        sd!("lkqts_cb_pyr", "Channel breakout with pyramiding (Strategy 3)", G8, "Long/Short", Live,
            ["E|40/20 channel breakout (p.242)",
             "A|\"we add another unit long if today is a 40-day high and there have been no new 40-day highs over the five previous days\"; shorts likewise (p.242)",
             "X|20-day opposite extreme"],
            "Each unit = one full position (returns add up per unit, no cap stated).", None),
        sd!("lkqts_cb_trendstat", "Channel breakout with overlap trend statistic", G8, "Long/Short", NotTestable("the overlap statistic's look-back is stated as both \"the past 100 days\" and \"the past 20 days\" (p.243); the book does not resolve which"),
            ["E|40-day breakout only when the trend statistic < 0.15; exits on 20-day extremes (p.244)"], "", None),
        sd!("lkqts_trend_filters", "Other new trend filters (40-day high recency; 10-day range expansion)", G8, "Long/Short", NotTestable("filters only - the book suggests combining them with other systems but specifies no complete system or exit (p.243)"),
            ["F|\"take long entries only if a 40-day high was made more recently than a 40-day low\" (p.243)", "F|range of past 10 days > range of the 10 days beginning 20 days ago (p.243)"], "", None),
        // ---------------- Chapter 6
        sd!("lkqts_cb_afterloss", "Channel breakout - only after a losing trade", G6, "Long/Short", Live,
            ["E|40/20 channel breakout entries taken \"only ... when ... the last trade was a loser\" (p.123)",
             "X|20-day opposite extreme"],
            "Last trade = the last closed trade of the unfiltered breakout (shadow trades); taking only real trades would stop all trading after one winner. Book portfolio: futures $2,971,124 vs $5,971,279 for all trades (Fig. 6.14).", None),
        sd!("lkqts_cb_afterwin", "Channel breakout - only after a winning trade", G6, "Long/Short", Live,
            ["E|40/20 channel breakout entries taken only when the last trade was a winner (p.123)", "X|20-day opposite extreme"],
            "Shadow-trade reading as for lkqts_cb_afterloss.", None),
        sd!("lkqts_regime", "ADX regime switching (channel breakout / RSI)", G6, "Long/Short", NotTestable("the book gives regimes (14-day ADX < 20 or > 30: 40/20 breakout; 20-30: 14-day RSI) but no rule for positions open when the regime changes; its test only bins daily P&L by ADX"),
            ["F|Fig. 6.13 regimes (p.123)"], "", None),
        sd!("lkqts_adx_bins", "Channel breakout with ADX band filter", G6, "Long/Short", NotTestable("a parameter sweep (ADX between x and x+5, x = 15..30, optionally rising) with results only as bar charts; no single rule is recommended"),
            ["F|Take breakout signals only when the 14-day ADX is between x and x+5 (p.118)"], "", None),
        // ---------------- Chapter 9
        sd!("lkqts_d100", "Difference from 100-day moving average", G9, "Long/Short", Live,
            ["F|100-day stat = (Close - 100-day SMA of closes)/100-day standard deviation of price changes (p.277)",
             "E|\"Long entries are entered when the 100-day stat falls below -2.5, while short positions are established when this statistic rises above 2.5\" (p.277)",
             "X|\"Long entries are exited when the 100-day stat rises above zero, and short entries are exited when the 100-day stat falls below zero.\" (p.277)"],
            "Book applied it to 30 relative-value spreads; here on an index (instrument swap).", None),
        sd!("lkqts_d1040", "Difference between 10- and 40-day moving averages", G9, "Long/Short", Live,
            ["F|10/40 stat = (10-day SMA - 40-day SMA)/100-day standard deviation of price changes (p.280)",
             "E|\"Long entries are established when the 10/40 stat falls below -2 ... Short entries are established when the 10/40 stat rises above 2\" (p.280)",
             "X|longs \"are exited if the 10/40 stat rises above zero\"; shorts \"are exited if the 10/40 stat falls below zero\" (p.280)"],
            "Book applied it to relative-value spreads; here on an index.", None),
        sd!("lkqts_rv_markets", "Relative-value markets (yield curve, credit, VIX, index ratios, stock pairs, commodity ratios)", G9, "Long/Short", NeedsData("30 spread series (Treasury 2/5/10/30y yields, 5y swap spreads, Merrill Lynch HY yields, spot commodities, 15 US stock pairs, XAU/XOI) and two-leg spread execution; the engine trades one instrument"),
            ["F|ln(SPX/NDX), ln(SPX/RUT), yield spreads, ... (Fig. 9.17, p.268); position size $10,000/sigma(100)"], "", None),
        // ---------------- Chapter 10
        sd!("lkqts_dom1", "Day-of-month strategy 1 (days 1-5 and 27-31)", G10, "Long", Live,
            ["E|\"We are long the market during Days one through five and Days 27 through 31\" (calendar days, p.297)",
             "X|\"Assets are invested in cash for the remainder of the month\" (p.297)",
             "F|In the market for a day = hold from the previous close to that day's close (book uses daily close-to-close returns)"],
            "", cl("-", "-", "$100 -> $454 (B&H $419)", "<30% of the time", "S&P 500 1/1990-2001")),
        sd!("lkqts_dom2", "Day-of-month strategy 2 (days 1-5, 12-18, 27-31)", G10, "Long", Live,
            ["E|\"We are long the market during Days one through five, 12 through 18, and 27 through 31\" (p.297)",
             "X|In cash the remainder of the month (p.297)"],
            "", cl("-", "-", "$100 -> $758 (B&H $419)", "54% of the time", "S&P 500 1/1990-2001")),
        StrategyDef { needs_vix: true, ..sd!("lkqts_vixscore", "Kestner VIX score strategy", G10, "Long", Live,
            ["F|VIX score = (VIX today - 120-day average VIX)/standard deviation of daily VIX changes over 120 days (p.301)",
             "E|\"Our new VIX strategy buys the market when today's VIX score is greater than +2\" (p.301)",
             "X|\"We stay long until the current day's score falls below -2.\" (p.301)",
             "F|Fills at the signal day's close (Ch.10 measures close-to-close returns)"],
            "US: CBOE VIX; India: India VIX (from 2009; window starts at the first VIX bar).", cl("-", "-", "beats buy & hold", "53% of the time", "S&P 500 1990-2001"))},
        StrategyDef { needs_vix: true, ..sd!("lkqts_gross_vix", "Leon Gross VIX 30/20 (reported by Kestner)", G10, "Long", Live,
            ["E|\"entering long positions in the S&P 500 on VIX crosses above 30\" (p.299)", "X|\"exiting into cash on drops below 20\" (p.299)"],
            "Not Kestner's own rule; reported as a fixed-level method. Fills at the close.", None)},
        sd!("lkqts_headley_vix", "Price Headley VIX Bollinger method (reported)", G10, "Long", NotTestable("the book quotes only the entry (VIX crossed below its upper 21-day 2-sd band five days ago and S&P 100 close > 10-day MA); no exit is given"),
            ["E|p.299"], "", None),
        sd!("lkqts_rate_timing", "Monthly timing with 1-year / 10-year Treasury yields (19 rules of Fig. 10.3)", G10, "Long", NeedsData("monthly 1-year and 10-year US Treasury yields (India: RBI/NSE G-sec yields) and the S&P 500 total-return index with T-bill cash returns"),
            ["E|e.g. \"10 year yields<3 month average\" -> $6,819 from $100 (1970-2001), 50% invested (Fig. 10.3, p.296)"], "", None),
    ];
    for (id, name, rule, claim) in [
        ("lkqts_m_sma12", "Monthly: SPX > 12-month average", "E|Invested when the month-end close > the average of the last 12 month-end closes (Fig. 10.3)", ("$4,123", "76%")),
        ("lkqts_m_sma6", "Monthly: SPX > 6-month average", "E|Invested when the month-end close > the average of the last 6 month-end closes", ("$1,879", "71%")),
        ("lkqts_m_sma3", "Monthly: SPX > 3-month average", "E|Invested when the month-end close > the average of the last 3 month-end closes", ("$1,310", "63%")),
        ("lkqts_m_mom12", "Monthly: SPX > 12 months ago", "E|Invested when the month-end close > the month-end close 12 months earlier", ("$4,211", "81%")),
        ("lkqts_m_mom6", "Monthly: SPX > 6 months ago", "E|Invested when the month-end close > the month-end close 6 months earlier", ("$2,132", "72%")),
        ("lkqts_m_mom3", "Monthly: SPX > 3 months ago", "E|Invested when the month-end close > the month-end close 3 months earlier", ("$2,006", "71%")),
    ] {
        let rules: &'static [&'static str] = Box::leak(vec![rule, "X|Otherwise in cash; decided at each month-end close (p.294-296)", "F|Book used the S&P 500 total-return index and paid T-bill interest in cash; here the price index, no cash interest"].into_boxed_slice());
        v.push(StrategyDef { id, name, group: G10, session: 0, dir: "Long", status: Live, needs_volume: false, needs_vix: false,
            needs_series: &[], book: BOOK, rules, notes: "Fills at the close of the last trading day of the month.",
            claim: cl("-", "-", Box::leak(format!("$100 -> {}", claim.0).into_boxed_str()), Box::leak(format!("{} invested", claim.1).into_boxed_str()), "S&P 500 TR 1970-2001") });
    }
    v.push(sd!("lkqts_nvb_entry", "New volatility breakout - entry only (Version 1)", G12, "Long/Short", Live,
        ["E|For every x from 5 to 20: buy if close - close x days ago > num x sqrt(x) x sigma(100 price changes); sell if < -num x sqrt(x) x sigma (p.322)",
         "F|\"the longer term signal always overrides the shorter term signal\" (p.323); num = 1.0 (p.325)",
         "X|Version #1 \"includes only the entry mechanism\" - reverse on the opposite signal (p.324)"],
        "Book tested 19 trend-prone futures only.", None));
    v.push(sd!("lkqts_nvb_trail", "New volatility breakout with 2-sigma trailing stop (Version 2)", G12, "Long/Short", Live,
        ["E|Version 1 entries, num = 1.0 (p.325)",
         "X|\"If we're long and prices fall from the highest high seen during the trade by twice the 100-day standard deviation of price returns, we'll exit our long position. If we're short and prices rise from the lowest low seen during the trade by twice ... we'll exit our short position.\" (p.324) - close-based, exit next open"],
        "Book (19 futures): $8,741,153, Sharpe 0.74, K 0.42 (Fig. 12.4a).", None));
    v
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    if !id.starts_with("lkqts_") {
        return None;
    }
    Some(match id {
        "lkqts_cb4020" | "lkqts_cb_atrstop" | "lkqts_cb_ptake" | "lkqts_cb_pyr" | "lkqts_cb_trendstat" | "lkqts_cb_afterloss" | "lkqts_cb_afterwin" | "lkqts_adx_bins" | "lkqts_regime" | "lkqts_trend_filters" =>
            ("lkqts_cb", "Kestner 40/20 channel breakout", match id {
                "lkqts_cb4020" => "Plain", "lkqts_cb_atrstop" => "2-ATR stop", "lkqts_cb_ptake" => "Profit taking", "lkqts_cb_pyr" => "Pyramiding",
                "lkqts_cb_afterloss" => "After a loser", "lkqts_cb_afterwin" => "After a winner", "lkqts_cb_trendstat" => "Trend statistic filter",
                "lkqts_adx_bins" => "ADX band filter", "lkqts_regime" => "ADX regime switch", _ => "Trend filters" }),
        "lkqts_mom20" | "lkqts_mom80" => ("lkqts_mom", "Kestner momentum", if id == "lkqts_mom20" { "20-day" } else { "80-day" }),
        "lkqts_d100" | "lkqts_d1040" | "lkqts_rv_markets" => ("lkqts_rv", "Kestner relative-value exhaustion", match id { "lkqts_d100" => "100-day stat", "lkqts_d1040" => "10/40 stat", _ => "Spread markets" }),
        "lkqts_dom1" | "lkqts_dom2" => ("lkqts_dom", "Kestner day of month", if id == "lkqts_dom1" { "Strategy 1" } else { "Strategy 2" }),
        "lkqts_vixscore" | "lkqts_gross_vix" | "lkqts_headley_vix" => ("lkqts_vix", "VIX timing of the S&P 500", match id { "lkqts_vixscore" => "Kestner VIX score", "lkqts_gross_vix" => "Gross 30/20", _ => "Headley bands" }),
        "lkqts_nvb_entry" | "lkqts_nvb_trail" => ("lkqts_nvb", "Kestner new volatility breakout", if id == "lkqts_nvb_entry" { "Entry only" } else { "Trailing stop" }),
        s if s.starts_with("lkqts_m_") || s == "lkqts_rate_timing" => ("lkqts_monthly", "Kestner monthly S&P timing", match s {
            "lkqts_m_sma12" => "> 12-month average", "lkqts_m_sma6" => "> 6-month average", "lkqts_m_sma3" => "> 3-month average",
            "lkqts_m_mom12" => "> 12 months ago", "lkqts_m_mom6" => "> 6 months ago", "lkqts_m_mom3" => "> 3 months ago", _ => "Interest-rate rules" }),
        _ => (id_static(id)?, "Kestner single system", "Book rules"),
    })
}

fn id_static(id: &str) -> Option<&'static str> {
    catalogue().into_iter().find(|d| d.id == id).map(|d| d.id)
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let cb = || vec![p("entry", "Entry channel (closes)", 40., 10., 100., 5.), p("exit", "Exit channel (closes)", 20., 5., 50., 5.)];
    Some(match id {
        "lkqts_cb4020" | "lkqts_cb_afterloss" | "lkqts_cb_afterwin" => cb(),
        "lkqts_cb_atrstop" => { let mut v = cb(); v.push(p("atr_n", "ATR length", 20., 5., 50., 5.)); v.push(p("stop", "Stop, ATR multiples from entry", 2., 0.5, 5., 0.5)); v }
        "lkqts_cb_ptake" => { let mut v = cb(); v.push(p("atr_n", "ATR length", 20., 5., 50., 5.)); v.push(p("target", "Profit exit, ATR multiples", 3., 1., 6., 0.5)); v.push(p("gap", "Days without a 40-day extreme before a new entry", 5., 1., 10., 1.)); v }
        "lkqts_cb_pyr" => { let mut v = cb(); v.push(p("gap", "Days without a 40-day extreme before adding", 5., 1., 10., 1.)); v }
        "lkqts_ma1040" => vec![p("fast", "Fast SMA", 10., 2., 30., 1.), p("slow", "Slow SMA", 40., 20., 120., 5.)],
        "lkqts_mom20" => vec![p("n", "Momentum days", 20., 5., 120., 5.)],
        "lkqts_mom80" => vec![p("n", "Momentum days", 80., 5., 200., 5.)],
        "lkqts_volbo" => vec![p("k", "Sigma multiple", 2., 0.5, 4., 0.25), p("sd_n", "Sigma look-back (price changes)", 100., 20., 250., 10.)],
        "lkqts_stoch14" => vec![p("n", "Stochastic days", 14., 5., 30., 1.), p("lo", "Oversold level", 20., 5., 40., 5.), p("hi", "Overbought level", 80., 60., 95., 5.)],
        "lkqts_rsi14" => vec![p("n", "RSI days", 14., 2., 30., 1.), p("lo", "Long below", 35., 10., 45., 5.), p("hi", "Short above", 65., 55., 90., 5.)],
        "lkqts_macd" => vec![p("fast", "Fast EMA", 12., 5., 20., 1.), p("slow", "Slow EMA", 26., 15., 50., 1.), p("sig", "Signal EMA", 9., 3., 20., 1.)],
        "lkqts_kma" => vec![p("n1", "Short MA length", 20., 5., 50., 5.), p("n2", "Long MA length", 80., 40., 200., 10.), p("x_one", "Filter: compare with yesterday (1, book's GE example and results) or n days ago (0, p.183 'typically')", 1., 0., 1., 1.)],
        "lkqts_sob" => vec![p("search", "Search back (days)", 80., 40., 120., 10.), p("chan", "Statistic channel (days)", 40., 10., 80., 5.), p("min", "Minimum statistic", 20., 5., 40., 5.)],
        "lkqts_macdhr" => vec![p("retrace", "Retracement of the extreme", 0.25, 0.1, 0.5, 0.05), p("arm", "Arming level, x sigma(20)", 0.5, 0.0, 2.0, 0.25), p("sd_n", "Sigma look-back", 20., 10., 50., 5.)],
        "lkqts_macs" => vec![p("long", "Long at MACS >=", 60., 50., 90., 5.), p("short", "Short at MACS <=", 40., 10., 50., 5.)],
        "lkqts_nei" => vec![p("n", "Look-back days", 50., 20., 100., 10.), p("up", "Upper band rank", 10., 1., 25., 1.), p("dn", "Lower band rank", 40., 25., 50., 1.), p("scale", "Band = MA + ranked NEI x sigma (1, reproduces the book's SPX table) or + NEI as printed (0)", 1., 0., 1., 1.)],
        "lkqts_adjsto" => vec![p("n", "Stochastic days", 14., 5., 30., 1.), p("long_n", "Range normaliser days", 100., 50., 200., 10.), p("lo", "Long cross above", 35., 20., 45., 1.), p("hi", "Short cross below", 65., 55., 80., 1.)],
        "lkqts_tiar" => vec![p("step", "Days between closes", 5., 2., 10., 1.)],
        "lkqts_volrev" => vec![p("sd_n", "Sigma look-back", 100., 20., 250., 10.), p("vol_pct", "Volume ratio below", 0.75, 0.5, 1.0, 0.05), p("hold", "Exit on day N of the trade", 5., 2., 10., 1.), p("exit_close", "Exit at that day's close (1) instead of open (0)", 0., 0., 1., 1.)],
        "lkqts_saitta" => vec![p("n", "SMA of highs/lows", 20., 5., 50., 5.), p("hl", "Tops/bottoms from highs/lows (1) instead of closes (0)", 0., 0., 1., 1.)],
        "lkqts_d100" => vec![p("n", "MA / sigma days", 100., 20., 250., 10.), p("k", "Entry threshold", 2.5, 1., 4., 0.25)],
        "lkqts_d1040" => vec![p("fast", "Fast SMA", 10., 5., 20., 1.), p("slow", "Slow SMA", 40., 20., 100., 5.), p("sd_n", "Sigma days", 100., 20., 250., 10.), p("k", "Entry threshold", 2., 1., 4., 0.25)],
        "lkqts_dom1" | "lkqts_dom2" => vec![],
        "lkqts_vixscore" => vec![p("n", "Average / sigma days", 120., 40., 250., 10.), p("hi", "Buy above score", 2., 0.5, 4., 0.25), p("lo", "Exit below score", -2., -4., 0., 0.25)],
        "lkqts_gross_vix" => vec![p("hi", "Buy on VIX cross above", 30., 15., 50., 1.), p("lo", "Exit on VIX drop below", 20., 10., 30., 1.)],
        "lkqts_m_sma12" | "lkqts_m_sma6" | "lkqts_m_sma3" | "lkqts_m_mom12" | "lkqts_m_mom6" | "lkqts_m_mom3" => vec![],
        "lkqts_nvb_entry" => vec![p("num", "num (sigma multiple)", 1.0, 0.5, 3., 0.25), p("x_min", "Shortest change (days)", 5., 2., 10., 1.), p("x_max", "Longest change (days)", 20., 10., 40., 1.)],
        "lkqts_nvb_trail" => vec![p("num", "num (sigma multiple)", 1.0, 0.5, 3., 0.25), p("x_min", "Shortest change (days)", 5., 2., 10., 1.), p("x_max", "Longest change (days)", 20., 10., 40., 1.), p("trail", "Trailing stop, sigma multiples", 2., 0.5, 5., 0.5)],
        _ => return None,
    })
}

// ------------------------------------------------------------------ strategies

fn cross_up(v: &[f64], i: usize, lvl: f64) -> bool {
    i > 0 && v[i - 1] <= lvl && v[i] > lvl
}
fn cross_dn(v: &[f64], i: usize, lvl: f64) -> bool {
    i > 0 && v[i - 1] >= lvl && v[i] < lvl
}

struct CbOpt {
    entry: usize,
    exit: usize,
    atr_n: usize,
    stop: f64,
    target: f64,
    gap: usize,  // 0 = no gap rule
    pyr: usize,  // 0 = no pyramiding, else gap for adds
}

/// Kestner 40/20 channel breakout family (pp.139-140, 236-242).
fn cb(x: &Ctx, from: usize, to: usize, o: &CbOpt) -> Vec<Trade> {
    let c = &x.c;
    let atr = atr_s(x, o.atr_n.max(1));
    let warm = o.entry.max(o.exit);
    let hi = |i: usize, n: usize| i + 1 >= n && c[i] >= hh(c, i, n);
    let lo = |i: usize, n: usize| i + 1 >= n && c[i] <= ll(c, i, n);
    let none_before = |i: usize, g: usize, f: &dyn Fn(usize) -> bool| i >= g && (i - g..i).all(|k| !f(k));
    let mut stop_lvl = f64::NAN;
    run_target(x, from.max(warm), to, Fill::NextOpen, |i, pos, t| {
        let h40 = hi(i, o.entry);
        let l40 = lo(i, o.entry);
        let mut want = pos;
        let mut why = "";
        if pos > 0 {
            let t = t.unwrap();
            let e = t.legs[0].px;
            if lo(i, o.exit) {
                want = 0; why = "20-day lowest close";
            } else if o.stop > 0.0 && c[i] < stop_lvl {
                want = 0; why = "2 x ATR(20) stop below entry";
            } else if o.target > 0.0 && c[i] - e > o.target * atr[i] {
                want = 0; why = "open profit > 3 x ATR(20)";
            }
        } else if pos < 0 {
            let t = t.unwrap();
            let e = t.legs[0].px;
            if hi(i, o.exit) {
                want = 0; why = "20-day highest close";
            } else if o.stop > 0.0 && c[i] > stop_lvl {
                want = 0; why = "2 x ATR(20) stop above entry";
            } else if o.target > 0.0 && e - c[i] > o.target * atr[i] {
                want = 0; why = "open profit > 3 x ATR(20)";
            }
        }
        let gap_ok_l = o.gap == 0 || none_before(i, o.gap, &|k| hi(k, o.entry));
        let gap_ok_s = o.gap == 0 || none_before(i, o.gap, &|k| lo(k, o.entry));
        if want <= 0 && h40 && gap_ok_l {
            want = 1; why = "40-day highest close";
        } else if want >= 0 && l40 && gap_ok_s {
            want = -1; why = "40-day lowest close";
        } else if o.pyr > 0 && want > 0 && h40 && none_before(i, o.pyr, &|k| hi(k, o.entry)) {
            want += 1; why = "add unit: new 40-day high";
        } else if o.pyr > 0 && want < 0 && l40 && none_before(i, o.pyr, &|k| lo(k, o.entry)) {
            want -= 1; why = "add unit: new 40-day low";
        }
        if want != pos && want.signum() != pos.signum() && want != 0 && o.stop > 0.0 {
            // stop distance fixed with the ATR "at the time of entry" (signal day)
            stop_lvl = x.o.get(i + 1).copied().unwrap_or(f64::NAN) - want.signum() as f64 * o.stop * atr[i];
        }
        (want != pos).then_some((want, why))
    })
}

fn stoch_sar(k: &[f64], from: usize, to: usize, x: &Ctx, lo: f64, hi: f64) -> Vec<Trade> {
    let sig: Vec<i8> = (0..k.len()).map(|i| if cross_up(k, i, lo) { 1 } else if cross_dn(k, i, hi) { -1 } else { 0 }).collect();
    sar(x, from, to, &sig, "oscillator crossed back above the low level", "oscillator crossed back below the high level")
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !id.starts_with("lkqts_") {
        return None;
    }
    let defs = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| defs.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let u = |k: &str| g(k).round().max(1.0) as usize;
    let c = &x.c;
    let n = c.len();
    if from >= to || to >= n {
        return Some(vec![]);
    }
    let cbo = |gap, pyr, stop, target| CbOpt { entry: u("entry"), exit: u("exit"), atr_n: if stop > 0.0 || target > 0.0 { u("atr_n") } else { 20 }, stop, target, gap, pyr };
    let t = match id {
        "lkqts_cb4020" => cb(x, from, to, &cbo(0, 0, 0.0, 0.0)),
        "lkqts_cb_atrstop" => cb(x, from, to, &cbo(0, 0, g("stop"), 0.0)),
        "lkqts_cb_ptake" => cb(x, from, to, &cbo(u("gap"), 0, 0.0, g("target"))),
        "lkqts_cb_pyr" => cb(x, from, to, &cbo(0, u("gap"), 0.0, 0.0)),
        "lkqts_cb_afterloss" | "lkqts_cb_afterwin" => {
            let want_win = id == "lkqts_cb_afterwin";
            let shadow = cb(x, 0, to, &cbo(0, 0, 0.0, 0.0));
            let mut out = Vec::new();
            for (k, t) in shadow.iter().enumerate() {
                let sig_i = t.legs[0].i - 1; // entry signal bar (fills are next-open)
                if sig_i < from {
                    continue;
                }
                // last shadow trade already closed at the signal bar (no peeking at a same-open exit fill)
                let prev = shadow[..k].iter().rev().find(|q| !q.open && q.exit_i <= sig_i);
                let Some(prev) = prev else { continue };
                let won = prev.dir * (prev.exit_px - prev.legs[0].px) > 0.0;
                if won == want_win {
                    out.push(t.clone());
                }
            }
            out
        }
        "lkqts_ma1040" => {
            let (f, s) = (x.sma_n(u("fast")), x.sma_n(u("slow")));
            let d: Vec<f64> = (0..n).map(|i| f[i] - s[i]).collect();
            let sig: Vec<i8> = (0..n).map(|i| if cross_up(&d, i, 0.0) { 1 } else if cross_dn(&d, i, 0.0) { -1 } else { 0 }).collect();
            sar(x, from, to, &sig, "10-day SMA crossed above 40-day SMA", "10-day SMA crossed below 40-day SMA")
        }
        "lkqts_mom20" | "lkqts_mom80" => {
            let m = u("n");
            let sig: Vec<i8> = (0..n).map(|i| if i < m { 0 } else if c[i] > c[i - m] { 1 } else if c[i] < c[i - m] { -1 } else { 0 }).collect();
            sar(x, from, to, &sig, "close > close n days ago", "close < close n days ago")
        }
        "lkqts_volbo" => {
            let sd = sd_chg(c, u("sd_n"));
            let k = g("k");
            let sig: Vec<i8> = (0..n).map(|i| {
                if i == 0 || !sd[i].is_finite() { 0 } else if c[i] - c[i - 1] > k * sd[i] { 1 } else if c[i] - c[i - 1] < -k * sd[i] { -1 } else { 0 }
            }).collect();
            sar(x, from, to, &sig, "price change > 2 sigma(100)", "price change < -2 sigma(100)")
        }
        "lkqts_stoch14" => stoch_sar(&slow_k(x, u("n")), from, to, x, g("lo"), g("hi")),
        "lkqts_rsi14" => {
            let r = rsi(c, u("n"));
            let (lo, hi) = (g("lo"), g("hi"));
            let sig: Vec<i8> = (0..n).map(|i| if r[i] < lo { 1 } else if r[i] > hi { -1 } else { 0 }).collect();
            sar(x, from, to, &sig, "RSI(14) below 35", "RSI(14) above 65")
        }
        "lkqts_macd" => {
            let (ef, es) = (ema(c, u("fast")), ema(c, u("slow")));
            let m: Vec<f64> = (0..n).map(|i| ef[i] - es[i]).collect();
            let s = ema(&m, u("sig"));
            let d: Vec<f64> = (0..n).map(|i| m[i] - s[i]).collect();
            let sig: Vec<i8> = (0..n).map(|i| if cross_up(&d, i, 0.0) { 1 } else if cross_dn(&d, i, 0.0) { -1 } else { 0 }).collect();
            sar(x, from, to, &sig, "MACD crossed above signal", "MACD crossed below signal")
        }
        "lkqts_kma" => {
            let (n1, n2) = (u("n1"), u("n2"));
            let one = g("x_one") >= 0.5;
            let hs = |m: usize| (sma(&x.h, m), sma(&x.l, m), sma(c, m));
            let state = |m: usize| -> Vec<i8> {
                let (ah, al, ac) = hs(m);
                let lag = if one { 1 } else { m };
                let mut st = vec![0i8; n];
                let mut s = 0i8;
                for i in 0..n {
                    if i >= lag && ac[i].is_finite() && ac[i - lag].is_finite() {
                        if c[i] > ah[i] && ac[i] > ac[i - lag] { s = 1 } else if c[i] < al[i] && ac[i] < ac[i - lag] { s = -1 }
                    }
                    st[i] = s;
                }
                st
            };
            let (s1, s2) = (state(n1), state(n2));
            let (ah2, al2) = (sma(&x.h, n2), sma(&x.l, n2));
            run_target(x, from, to, Fill::NextOpen, |i, pos, _| {
                let agree = if s1[i] == 1 && s2[i] == 1 { 1 } else if s1[i] == -1 && s2[i] == -1 { -1 } else { 0 };
                if pos > 0 && c[i] < al2[i] {
                    return Some(if agree == -1 { (-1, "both systems short") } else { (0, "close below 80-day SMA of lows") });
                }
                if pos < 0 && c[i] > ah2[i] {
                    return Some(if agree == 1 { (1, "both systems long") } else { (0, "close above 80-day SMA of highs") });
                }
                if agree != 0 && agree as i32 != pos {
                    return Some((agree as i32, if agree > 0 { "20- and 80-day systems both long" } else { "20- and 80-day systems both short" }));
                }
                None
            })
        }
        "lkqts_sob" => {
            let (sb, ch, mn) = (u("search"), u("chan"), g("min"));
            let mut st = vec![f64::NAN; n];
            for i in sb..n {
                let (mut nh, mut nl) = (1usize, 1usize);
                while nh < sb && c[i - nh] <= c[i] { nh += 1 }
                while nl < sb && c[i - nl] >= c[i] { nl += 1 }
                st[i] = if nh > nl { nh as f64 } else if nl > nh { -(nl as f64) } else { 0.0 };
            }
            let sig: Vec<i8> = (0..n).map(|i| {
                if i + 1 < ch || !st[i + 1 - ch].is_finite() { return 0 }
                if st[i] > mn && st[i] >= hh(&st, i, ch) { 1 } else if st[i] < -mn && st[i] <= ll(&st, i, ch) { -1 } else { 0 }
            }).collect();
            sar(x, from, to, &sig, "SOB statistic: 40-day high and > 20", "SOB statistic: 40-day low and < -20")
        }
        "lkqts_macdhr" => {
            let (ef, es) = (ema(c, 12), ema(c, 26));
            let m: Vec<f64> = (0..n).map(|i| ef[i] - es[i]).collect();
            let s = ema(&m, 9);
            let h: Vec<f64> = (0..n).map(|i| m[i] - s[i]).collect();
            let sd = sd_chg(c, u("sd_n"));
            let (rt, arm) = (g("retrace"), g("arm"));
            let mut sig = vec![0i8; n];
            let (mut reg, mut ext, mut armed, mut done) = (0i8, 0.0f64, false, false);
            for i in 1..n {
                if reg < 0 && armed && !done && h[i] > (1.0 - rt) * ext { sig[i] = 1; done = true; }
                if reg > 0 && armed && !done && h[i] < (1.0 - rt) * ext { sig[i] = -1; done = true; }
                if h[i - 1] >= 0.0 && h[i] < 0.0 { reg = -1; ext = h[i]; armed = false; done = false; }
                else if h[i - 1] <= 0.0 && h[i] > 0.0 { reg = 1; ext = h[i]; armed = false; done = false; }
                else if reg < 0 { ext = ext.min(h[i]) } else if reg > 0 { ext = ext.max(h[i]) }
                if reg < 0 && sd[i].is_finite() && h[i] < -arm * sd[i] { armed = true }
                if reg > 0 && sd[i].is_finite() && h[i] > arm * sd[i] { armed = true }
            }
            sar(x, from, to, &sig, "histogram retraced 25% of its trough", "histogram retraced 25% of its peak")
        }
        "lkqts_macs" => {
            let pairs: Vec<(Vec<f64>, Vec<f64>)> = (1..=20).map(|k| (sma(c, k), sma(c, 4 * k))).collect();
            let (lo, hi) = (g("short"), g("long"));
            let sig: Vec<i8> = (0..n).map(|i| {
                if i + 1 < 80 { return 0 }
                let m: f64 = pairs.iter().map(|(a, b)| if a[i] > b[i] { 5.0 } else { 0.0 }).sum();
                if m >= hi { 1 } else if m <= lo { -1 } else { 0 }
            }).collect();
            sar(x, from, to, &sig, "MACS >= 60", "MACS <= 40")
        }
        "lkqts_nei" => {
            let m = u("n");
            let (up, dn) = (u("up").min(m), u("dn").min(m));
            let scale = g("scale") >= 0.5;
            let ma = sma(c, m);
            let sd = sd_chg(c, m);
            let mut ub = vec![f64::NAN; n];
            let mut lb = vec![f64::NAN; n];
            for i in 0..n {
                if i + 1 < 2 * m || !sd[i].is_finite() || sd[i] <= 0.0 { continue }
                let mut v: Vec<f64> = (i + 1 - m..=i).map(|k| (c[k] - ma[k]) / sd[i]).collect();
                v.sort_by(|a, b| b.partial_cmp(a).unwrap());
                let f = if scale { sd[i] } else { 1.0 };
                ub[i] = ma[i] + v[up - 1] * f;
                lb[i] = ma[i] + v[dn - 1] * f;
            }
            let sig: Vec<i8> = (0..n).map(|i| {
                if i == 0 || !ub[i - 1].is_finite() || !ub[i].is_finite() { return 0 }
                if c[i - 1] <= lb[i - 1] && c[i] > lb[i] { 1 } else if c[i - 1] >= ub[i - 1] && c[i] < ub[i] { -1 } else { 0 }
            }).collect();
            sar(x, from, to, &sig, "close back above the lower NEI band", "close back below the upper NEI band")
        }
        "lkqts_adjsto" => {
            let (m, lm) = (u("n"), u("long_n"));
            let k = slow_k(x, m);
            let a: Vec<f64> = (0..n).map(|i| {
                if i + 1 < lm { return f64::NAN }
                let (r1, r2) = (hh(&x.h, i, m) - ll(&x.l, i, m), hh(&x.h, i, lm) - ll(&x.l, i, lm));
                if r2 > 0.0 { (k[i] - 50.0) * r1 / r2 + 50.0 } else { f64::NAN }
            }).collect();
            stoch_sar(&a, from, to, x, g("lo"), g("hi"))
        }
        "lkqts_tiar" => {
            let s = u("step");
            let sig: Vec<i8> = (0..n).map(|i| {
                if i < 3 * s { 0 } else if c[i] > c[i - s] && c[i - s] > c[i - 2 * s] && c[i - 2 * s] > c[i - 3 * s] { 1 }
                else if c[i] < c[i - s] && c[i - s] < c[i - 2 * s] && c[i - 2 * s] < c[i - 3 * s] { -1 } else { 0 }
            }).collect();
            sar(x, from, to, &sig, "three rising 5-day steps", "three falling 5-day steps")
        }
        "lkqts_volrev" => {
            let sd = sd_chg(c, u("sd_n"));
            let vol: Vec<f64> = x.r.iter().map(|r| r.volume.unwrap_or(f64::NAN)).collect();
            let (vp, hold, at_close) = (g("vol_pct"), u("hold"), g("exit_close") >= 0.5);
            let avg = |a: usize, b: usize| (a..=b).map(|k| vol[k]).sum::<f64>() / (b + 1 - a) as f64;
            let mut out = Vec::new();
            let mut i = from.max(10);
            while i < to {
                let ch5 = c[i] - c[i - 5];
                let setup = sd[i].is_finite() && ch5.abs() > sd[i] && avg(i - 4, i) < vp * avg(i - 10, i - 6);
                if setup && ch5 != 0.0 {
                    let e = i + 1;
                    let d = if ch5 < 0.0 { 1.0 } else { -1.0 };
                    let mut t = new_trade(d, e, x.o[e], 1, "open", i);
                    let xi = e + hold - 1;
                    if xi > to {
                        t.exit_i = to; t.exit_px = c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true;
                        out.push(t);
                        break;
                    }
                    t.exit_i = xi;
                    t.exit_px = if at_close { c[xi] } else { x.o[xi] };
                    t.signal_i = xi - 1;
                    t.reason = format!("day {hold} of the trade");
                    out.push(t);
                    i = xi; // the next signal can come on the exit bar's close
                    continue;
                }
                i += 1;
            }
            out
        }
        "lkqts_saitta" => {
            let m = u("n");
            let use_hl = g("hl") >= 0.5;
            let (ah, al) = (sma(&x.h, m), sma(&x.l, m));
            let mut sig = vec![0i8; n];
            let (mut phase, mut start) = (0i8, 0usize);
            let (mut top, mut bot) = (f64::NAN, f64::NAN);
            for i in 0..n {
                if !ah[i].is_finite() { continue }
                if phase <= 0 && c[i] > ah[i] {
                    if phase < 0 { bot = (start..=i).map(|k| if use_hl { x.l[k] } else { c[k] }).fold(f64::INFINITY, f64::min) }
                    phase = 1; start = i;
                } else if phase >= 0 && c[i] < al[i] {
                    if phase > 0 { top = (start..=i).map(|k| if use_hl { x.h[k] } else { c[k] }).fold(f64::NEG_INFINITY, f64::max) }
                    phase = -1; start = i;
                }
                if top.is_finite() && c[i] > top { sig[i] = 1 } else if bot.is_finite() && c[i] < bot { sig[i] = -1 }
            }
            sar(x, from, to, &sig, "close above the last Saitta top", "close below the last Saitta bottom")
        }
        "lkqts_d100" | "lkqts_d1040" => {
            let (st, k): (Vec<f64>, f64) = if id == "lkqts_d100" {
                let m = u("n");
                let (ma, sd) = (sma(c, m), sd_chg(c, m));
                ((0..n).map(|i| (c[i] - ma[i]) / sd[i]).collect(), g("k"))
            } else {
                let (f, s, sd) = (sma(c, u("fast")), sma(c, u("slow")), sd_chg(c, u("sd_n")));
                ((0..n).map(|i| (f[i] - s[i]) / sd[i]).collect(), g("k"))
            };
            run_target(x, from, to, Fill::NextOpen, |i, pos, _| {
                let v = st[i];
                if !v.is_finite() { return None }
                if pos > 0 && v > 0.0 { return Some((if v > k { -1 } else { 0 }, "statistic rose above zero")) }
                if pos < 0 && v < 0.0 { return Some((if v < -k { 1 } else { 0 }, "statistic fell below zero")) }
                if pos <= 0 && v < -k { return Some((1, "statistic below -threshold")) }
                if pos >= 0 && v > k { return Some((-1, "statistic above +threshold")) }
                None
            })
        }
        "lkqts_dom1" | "lkqts_dom2" => {
            let set = |d: u32| (1..=5).contains(&d) || (27..=31).contains(&d) || (id == "lkqts_dom2" && (12..=18).contains(&d));
            let dom = |i: usize| x.r[i].date[8..10].parse::<u32>().unwrap_or(0);
            run_target(x, from, to, Fill::Close, |i, _, _| {
                if i + 1 >= n { return None }
                Some(if set(dom(i + 1)) { (1, "next trading day is in the window") } else { (0, "window over") })
            })
        }
        "lkqts_vixscore" | "lkqts_gross_vix" => {
            let v = x.series("VIX.Close")?;
            if id == "lkqts_vixscore" {
                let m = u("n");
                let ma = sma(v, m);
                let sd = sd_chg(v, m);
                let (hi, lo) = (g("hi"), g("lo"));
                run_target(x, from, to, Fill::Close, |i, pos, _| {
                    let s = (v[i] - ma[i]) / sd[i];
                    if !s.is_finite() { return None }
                    if pos == 0 && s > hi { Some((1, "VIX score > +2")) } else if pos > 0 && s < lo { Some((0, "VIX score < -2")) } else { None }
                })
            } else {
                let (hi, lo) = (g("hi"), g("lo"));
                run_target(x, from, to, Fill::Close, |i, pos, _| {
                    if i == 0 || !v[i].is_finite() || !v[i - 1].is_finite() { return None }
                    if pos == 0 && v[i - 1] <= hi && v[i] > hi { Some((1, "VIX crossed above 30")) }
                    else if pos > 0 && v[i - 1] >= lo && v[i] < lo { Some((0, "VIX dropped below 20")) } else { None }
                })
            }
        }
        s if s.starts_with("lkqts_m_") => {
            let k: usize = if s.ends_with("12") { 12 } else if s.ends_with('6') { 6 } else { 3 };
            let is_ma = s.contains("_sma");
            let month = |i: usize| &x.r[i].date[..7];
            // month-end bars (calendar: the next trading day is in another month)
            let me: Vec<usize> = (0..n.saturating_sub(1)).filter(|&i| month(i) != month(i + 1)).collect();
            let mut want = vec![None::<bool>; n];
            for (j, &i) in me.iter().enumerate() {
                if j < k { continue }
                want[i] = Some(if is_ma {
                    let a = me[j + 1 - k..=j].iter().map(|&q| c[q]).sum::<f64>() / k as f64;
                    c[i] > a
                } else {
                    c[i] > c[me[j - k]]
                });
            }
            run_target(x, from, to, Fill::Close, |i, _, _| want[i].map(|w| if w { (1, "month-end signal on") } else { (0, "month-end signal off") }))
        }
        "lkqts_nvb_entry" | "lkqts_nvb_trail" => {
            let sd = sd_chg(c, 100);
            let (num, x0, x1) = (g("num"), u("x_min"), u("x_max"));
            let sig: Vec<i8> = (0..n).map(|i| {
                if !sd[i].is_finite() || i < x1 { return 0 }
                for xx in (x0..=x1).rev() {
                    let d = c[i] - c[i - xx];
                    let th = num * (xx as f64).sqrt() * sd[i];
                    if d > th { return 1 }
                    if d < -th { return -1 }
                }
                0
            }).collect();
            if id == "lkqts_nvb_entry" {
                sar(x, from, to, &sig, "x-day change > num sqrt(x) sigma", "x-day change < -num sqrt(x) sigma")
            } else {
                let tr = g("trail");
                run_target(x, from, to, Fill::NextOpen, |i, pos, t| {
                    if pos != 0 {
                        let t = t.unwrap();
                        let e = t.legs[0].i;
                        if pos > 0 {
                            let best = (e..=i).map(|k| x.h[k]).fold(f64::NEG_INFINITY, f64::max);
                            if sig[i] == -1 { return Some((-1, "opposite breakout")) }
                            if c[i] < best - tr * sd[i] { return Some((0, "2-sigma trailing stop from the highest high")) }
                        } else {
                            let best = (e..=i).map(|k| x.l[k]).fold(f64::INFINITY, f64::min);
                            if sig[i] == 1 { return Some((1, "opposite breakout")) }
                            if c[i] > best + tr * sd[i] { return Some((0, "2-sigma trailing stop from the lowest low")) }
                        }
                        return None;
                    }
                    match sig[i] { 1 => Some((1, "x-day change > num sqrt(x) sigma")), -1 => Some((-1, "x-day change < -num sqrt(x) sigma")), _ => None }
                })
            }
        }
        _ => return None,
    };
    Some(t)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn rows(cl: &[f64]) -> Vec<Row> {
        cl.iter().enumerate().map(|(i, &c)| Row { ticker: "T".into(), date: format!("2000-{:02}-{:02}", 1 + i / 28, 1 + i % 28), open: c, high: c + 1.0, low: c - 1.0, close: c, volume: Some(1.0) }).collect()
    }
    #[test]
    fn channel_breakout_enters_on_40_day_high_and_exits_on_20_day_low() {
        // 50 flat-ish bars, a 40-day closing high on bar 50, a plateau, then a slide to a 20-day closing low
        let cl: Vec<f64> = (0..80).map(|i| match i { 0..=49 => 150.0 - i as f64, 50 => 200.0, 51..=60 => 199.0, _ => 99.0 - i as f64 }).collect();
        let r = rows(&cl);
        let x = Ctx::new(&r, &r, &[]);
        let all = run("lkqts_cb4020", &x, 0, r.len() - 1, &Params::new()).unwrap();
        let t = all.iter().find(|t| t.legs[0].i == 51).expect("bought at the open after the bar-50 signal");
        assert_eq!(t.dir, 1.0);
        assert_eq!(t.legs[0].px, cl[51]);
        assert_eq!(t.exit_i, 62); // bar 61 is the first close below every close of the past 20 days
        assert_eq!(t.signal_i, 61);
    }
    #[test]
    fn population_sd_of_changes() {
        let s = sd_chg(&[1.0, 2.0, 4.0, 7.0], 3); // changes 1,2,3 -> pop sd sqrt(2/3)
        assert!((s[3] - (2.0f64 / 3.0).sqrt()).abs() < 1e-12);
    }
}
