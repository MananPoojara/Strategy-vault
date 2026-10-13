//! Richard L. Weissman, *Trade Like a Casino: Find Your Edge, Manage Risk, and Win Like the House* (Wiley, 2011).
//! Book id `rwtlc`. Source copy: Books/Richard L. Weissman/Richard L. Weissman - Trade Like a Casino (Wiley 2011).pdf.
//! Transcription, testability table and book-vs-ours checks: library/rwtlc/README.md.
//!
//! Conventions (same CQG code style as the author's 2004 book, module `rwmts`): every `[-1]` trigger is evaluated on
//! the CLOSE of bar s = t-1 and filled at the OPEN of bar t; an opposite entry signal reverses. Exits written as a
//! price (`LoLevel(@,3)[-1]`, `MA(@,Sim,20)[-1]`, the dollar stop) are price orders working during bar t (including
//! the entry bar): touch = fill at the level, gap through = fill at the open; stop before target on the same bar.
//! Book results are one-lot futures 2000-2009 (Tables 8.2-8.9, $10 round turn); the ES row is the only one the vault
//! can approach (SPX cash index, instrument swap). The single dollar amount (RSI Extremes $7,500 stop) is converted
//! for ES only (cash S&P 500 x $50 = 150 points) and kept as a labelled point parameter elsewhere (instrument swap).
//! Duplicates of `rwmts` (same code in both books) are not repeated here: Bollinger Band breakout (p.57, p.157 =
//! rwmts_bb_break), MACD crossover (p.56-57 = rwmts_macd), 9/26 SMA crossover (p.31, p.39 = rwmts_ma2x).
use crate::connors::{rsi, sma, true_range, wilder, Claim, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "rwtlc";

// ------------------------------------------------------------------ indicators

/// Population standard deviation over n bars ending at i (CQG Bollinger, as in rwmts).
fn sd(v: &[f64], n: usize) -> Vec<f64> {
    let mut out = vec![f64::NAN; v.len()];
    for i in n.saturating_sub(1)..v.len() {
        let w = &v[i + 1 - n..=i];
        let m = w.iter().sum::<f64>() / n as f64;
        out[i] = (w.iter().map(|x| (x - m).powi(2)).sum::<f64>() / n as f64).sqrt();
    }
    out
}
/// ADX as printed on p.85-86: +DI/-DI = smoothed directional movement / smoothed true range x 100 (Wilder sums over
/// n - the book only says "over a specified lookback period"), DX = |+DI - -DI| / (+DI + -DI) x 100, and
/// "ADX = SUM[DX, N] / N" (simple average). `wilder_adx` = true smooths DX with Wilder's average instead (engine's dmi).
fn adx(x: &Ctx, n: usize, wilder_adx: bool) -> Vec<f64> {
    let len = x.r.len();
    let (mut p, mut m, mut tr) = (vec![f64::NAN; len], vec![f64::NAN; len], vec![f64::NAN; len]);
    let trs = true_range(x.r);
    for i in 1..len {
        let up = x.h[i] - x.h[i - 1];
        let dn = x.l[i - 1] - x.l[i];
        p[i] = if up > dn && up > 0.0 { up } else { 0.0 };
        m[i] = if dn > up && dn > 0.0 { dn } else { 0.0 };
        tr[i] = trs[i];
    }
    let (sp, sm, st) = (wilder(&p, n), wilder(&m, n), wilder(&tr, n));
    let dx: Vec<f64> = (0..len).map(|i| {
        if !(st[i] > 0.0) { return f64::NAN; }
        let (a, b) = (100.0 * sp[i] / st[i], 100.0 * sm[i] / st[i]);
        if a + b > 0.0 { 100.0 * (a - b).abs() / (a + b) } else { 0.0 }
    }).collect();
    if wilder_adx { wilder(&dx, n) } else { sma(&dx, n) }
}
/// Midpoint of the n-bar high/low range ending at i (Ichimoku Tenkan / Kijun / Senkou B before displacement).
fn mid(x: &Ctx, n: usize) -> Vec<f64> {
    (0..x.c.len()).map(|i| if i + 1 < n { f64::NAN } else { (hi_n(x, i, n) + lo_n(x, i, n)) / 2.0 }).collect()
}
fn hi_n(x: &Ctx, i: usize, n: usize) -> f64 { x.h[i + 1 - n..=i].iter().cloned().fold(f64::MIN, f64::max) }
fn lo_n(x: &Ctx, i: usize, n: usize) -> f64 { x.l[i + 1 - n..=i].iter().cloned().fold(f64::MAX, f64::min) }

// ------------------------------------------------------------------ simulator (same engine as rwmts)

#[derive(Clone, Copy)]
struct Pos { dir: f64, ei: usize, epx: f64 }

struct Book { out: Vec<Trade>, pos: Option<Pos> }
impl Book {
    fn new() -> Self { Book { out: vec![], pos: None } }
    fn enter(&mut self, dir: f64, i: usize, px: f64) { self.pos = Some(Pos { dir, ei: i, epx: px }); }
    fn exit(&mut self, i: usize, px: f64, reason: &str, sig: usize) {
        if let Some(p) = self.pos.take() {
            self.out.push(Trade { dir: p.dir, legs: vec![Leg { i: p.ei, px: p.epx, w: 1.0 }], exit_i: i, exit_px: px,
                reason: reason.into(), entry_kind: "open", signal_i: sig, open: false, exits: vec![] });
        }
    }
    fn finish(mut self, x: &Ctx, to: usize) -> Vec<Trade> {
        if self.pos.is_some() { self.exit(to, x.c[to], "open", to); self.out.last_mut().unwrap().open = true; }
        self.out
    }
}

/// Stop / target fill on bar t: gap through first (at the open), then stop before target (conservative).
fn level_fill(x: &Ctx, t: usize, dir: f64, stop: Option<f64>, tgt: Option<f64>) -> Option<(f64, &'static str)> {
    let (o, h, l) = (x.o[t], x.h[t], x.l[t]);
    let s_hit = |s: f64| if dir > 0.0 { l <= s } else { h >= s };
    let t_hit = |g: f64| if dir > 0.0 { h >= g } else { l <= g };
    let s_gap = |s: f64| if dir > 0.0 { o <= s } else { o >= s };
    let t_gap = |g: f64| if dir > 0.0 { o >= g } else { o <= g };
    if let Some(s) = stop.filter(|s| s.is_finite()) { if s_gap(s) { return Some((o, "stop (gap at open)")); } }
    if let Some(g) = tgt.filter(|g| g.is_finite()) { if t_gap(g) { return Some((o, "target (gap at open)")); } }
    if let Some(s) = stop.filter(|s| s.is_finite()) { if s_hit(s) { return Some((s, "stop")); } }
    if let Some(g) = tgt.filter(|g| g.is_finite()) { if t_hit(g) { return Some((g, "target")); } }
    None
}

/// CQG-style system: `ent(s)` signal on the close of s (+1/-1/0) filled at the next open (opposite = reverse);
/// `cx(t, p)` = condition exit known before the open of t, filled at that open; `lv(t, p)` = (stop, target) on bar t.
fn sim(x: &Ctx, from: usize, to: usize, ent: &dyn Fn(usize) -> i8, cx: &dyn Fn(usize, &Pos) -> Option<&'static str>,
       lv: &dyn Fn(usize, &Pos) -> (Option<f64>, Option<f64>)) -> Vec<Trade> {
    let mut b = Book::new();
    for t in from + 1..=to {
        let s = t - 1;
        if let Some(p) = b.pos { if let Some(r) = cx(t, &p) { b.exit(t, x.o[t], r, s); } }
        let e = ent(s);
        if e != 0 {
            let d = e as f64;
            match b.pos {
                Some(p) if p.dir == d => {}
                Some(_) => { b.exit(t, x.o[t], "opposite signal (reverse at open)", s); b.enter(d, t, x.o[t]); }
                None => b.enter(d, t, x.o[t]),
            }
        }
        if let Some(p) = b.pos {
            let (st, tg) = lv(t, &p);
            if let Some((px, why)) = level_fill(x, t, p.dir, st, tg) { b.exit(t, px, why, t); }
        }
    }
    b.finish(x, to)
}
fn no_cx(_: usize, _: &Pos) -> Option<&'static str> { None }

// ------------------------------------------------------------------ catalogue

const FUT: &str = "Book: one-lot futures, equalized active daily continuation, 1/1/2000-12/31/2009, $10 round turn (Tables 8.2-8.9 note). The vault runs the same code on the selected index (instrument swap only); shorting a cash index is hypothetical.";
const C4: &str = "Ch.4 · Volatility models";
const C3: &str = "Ch.3/5/8 · RSI Trend";
const C8: &str = "Ch.8 · Trend and mean-reversion systems";
const CD: &str = "Not testable · discretionary / intraday";

macro_rules! def {
    ($id:expr, $name:expr, $g:expr, $dir:expr, $st:expr, $vol:expr, $rules:expr, $notes:expr, $claim:expr) => {
        StrategyDef { id: $id, name: $name, group: $g, session: 0, dir: $dir, status: $st, needs_volume: $vol, needs_vix: false,
            needs_series: &[], book: BOOK, rules: $rules, notes: $notes, claim: $claim }
    };
}
fn es(t: &'static str, w: &'static str, p: &'static str, d: &'static str, tbl: &'static str) -> Option<Claim> {
    Some(Claim { trades: t, win: w, avg: p, hold: d, period: tbl })
}

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    vec![
        def!("rwtlc_adx_lowvol_bb", "ADX low-volatility Bollinger breakout, 3-day trailing stop", C4, "Long/Short", Live, false,
            &["F|\"we will also define low volatility as times during which the 10-period ADX gives a reading below 20\" (p.90)",
              "E|Long Entry: Close(@)[-1] > BHI(@,Sim,20,2.00)[-1] AND ADX(@,10)[-2] < 20 (p.90)",
              "E|Short Entry: Close(@)[-1] < BLO(@,Sim,20,2.00)[-1] AND ADX(@,10)[-2] < 20 (p.90)",
              "X|Long Exit, set \"Price\" field to: LoLevel(@,3)[-1]; Short Exit: HiLevel(@,3)[-1] (p.90) - \"a trailing stop ... at the lowest low of the previous three trading days\"",
              "X|[Book silent, neutral] ADX = simple N-bar average of DX as printed p.86 (DI smoothing over the same 10 bars, Wilder); param adx_wilder=1 uses Wilder's smoothing of DX instead."],
            "Book result only as a CQG chart (Fig 4.20, ICE sugar continuation 2000-2009, $10 round turn) - an image of a futures result the vault has no data for; not reproduced. Indicator triggers on the prior close, orders at the next open; the trailing stop works from the entry bar.",
            None),
        def!("rwtlc_adx_hivol_ct", "ADX high-volatility countertrend (RSI 9, prior-bar stop, 20-day MA target)", C4, "Long/Short", Live, false,
            &["E|Long Entry: ADX(@,10)[-1] > 50 AND High(@)[-1] > High(@)[-2] AND RSI(@,9)[-1] < 35 (p.92)",
              "X|Long Exit: LoLevel(@,1)[-1] OR MA(@,Sim,20)[-1] (p.92) - \"our stop loss exit is set to the previous bar's low ... exits with profits at the previous bar's 20-period simple moving average\" (p.91)",
              "E|Short Entry: ADX(@,10)[-1] > 50 AND Low(@)[-1] < Low(@)[-2] AND RSI(@,9)[-1] > 65 (p.92)",
              "X|Short Exit: HiLevel(@,1)[-1] OR MA(@,Sim,20)[-1] (p.92)",
              "X|[Book silent, neutral] Both exits are price orders on the bar (stop at prior low, limit at prior SMA20); same-bar touch of both = stop first."],
            "Book result only as a CQG chart (Fig 4.21, cash USD/CHF 2000-2009) - not reproducible. ADX as in rwtlc_adx_lowvol_bb.",
            None),
        def!("rwtlc_rsi_trend3", "RSI Trend, 3-day trailing stop", C3, "Long/Short", Live, false,
            &["E|Long Entry: RSI(@,9)[-1] > 65 (p.58, p.147)", "E|Short Entry: RSI(@,9)[-1] < 35 (p.58, p.147)",
              "X|Long Exit: LoLevel(@,3)[-1]; Short Exit: HiLevel(@,3)[-1] (p.58, p.147) - \"stops at previous three-bar low for long positions and previous three-bar high for short positions\" (p.57)",
              "X|Entries are levels: while flat the system re-enters at the next open whenever RSI is still beyond 65/35 (code as printed)."],
            FUT, es("193", "25.91%", "-$55,105 total (ES, $50/pt)", "6 days", "ES row of Table 8.2, 2000-2009")),
        def!("rwtlc_rsi_trend1", "Short-term RSI Trend, 1-day trailing stop", C3, "Long/Short", Live, false,
            &["E|Long Entry: RSI(@,9)[-1] > 65; Short Entry: RSI(@,9)[-1] < 35 (p.59)",
              "X|Long Exit: LoLevel(@,1)[-1]; Short Exit: HiLevel(@,1)[-1] (p.59) - \"tightening the exit criterion ... to the previous bars' low (or high)\""],
            "Book result: Brent crude only (Table 3.1: P:MD 1.77, 40.55% win, avg 3 days). Not an index; not reproduced.", None),
        def!("rwtlc_rsi_trend10", "RSI Trend, 10-day trailing stop", C3, "Long/Short", Live, false,
            &["E|Long Entry: RSI(@,9)[-1] > 65; Short Entry: RSI(@,9)[-1] < 35 (p.148)",
              "X|Long Exit: LoLevel(@,10)[-1]; Short Exit: HiLevel(@,10)[-1] (p.148)"],
            FUT, es("105", "34.29%", "-$29,800 total (ES, $50/pt)", "17 days", "ES row of Table 8.3, 2000-2009")),
        def!("rwtlc_rsi_trend_ext", "RSI Trend with extreme readings exit", C3, "Long/Short", Live, false,
            &["E|Long Entry: RSI(@,9)[-1] > 65; Short Entry: RSI(@,9)[-1] < 35 (p.150)",
              "X|Long Exit: LoLevel(@,10)[-1] OR RSI(@,9)[-1] > 80 (p.150)", "X|Short Exit: HiLevel(@,10)[-1] OR RSI(@,9)[-1] < 20 (p.150)",
              "X|Code as printed: the entry level (RSI > 65) is still true when the RSI > 80 exit fires, so the system exits and re-enters at the same open (a new trade); the book's higher trade count (ES 117 vs 105) is consistent with that."],
            FUT, es("117", "33.33%", "-$27,382 total (ES, $50/pt)", "14 days", "ES row of Table 8.4, 2000-2009")),
        def!("rwtlc_rsi_trend3_time", "RSI Trend, 3-day stop with time exit", C3, "Long/Short", Live, false,
            &["E|RSI Trend entries: RSI(@,9)[-1] > 65 long, < 35 short (p.102, Fig 5.3-5.4)",
              "X|Long Exit: BarsSinceEntry(@,0,All,ThisTradeOnly) > 1 OR Price field set to: \"LoLevel(@,3)[-1]\" (p.103)",
              "X|Short Exit: BarsSinceEntry(@,0,All,ThisTradeOnly) > 1 OR Price field set to: \"HiLevel(@,3)[-1]\" (p.103) - \"forces an exit after the position has been held for more than one trading day\"",
              "X|Read: BarsSinceEntry carries no [-1], so it is the count at the bar of the order: exit at the open of the first bar with bars-since-entry > 1 (entry bar = 0, so the open of entry+2). Param hold."],
            "Book result: soybean futures only (Fig 5.4: % winners 40.62 -> 56.99, avg win/loss 1.88 -> 1.10). Not reproduced (no soybean data).", None),
        def!("rwtlc_ichi_x", "Ichimoku Tenkan/Kijun crossover", C8, "Long/Short", Live, false,
            &["F|Tenkan = (highest high + lowest low)/2 over 9 bars; Kijun = same over 26 bars (p.150)",
              "E|Long Entry: Imoku1(@,9)[-1] > Imoku2(@,26)[-1] (p.153)", "X|Long Exit: Imoku1(@,9)[-1] < Imoku2(@,26)[-1] (p.153)",
              "E|Short Entry: Imoku1(@,9)[-1] < Imoku2(@,26)[-1] (p.153)", "X|Short Exit: Imoku1(@,9)[-1] > Imoku2(@,26)[-1] (p.153)"],
            FUT, es("105", "33.33%", "-$10,200 total (ES, $50/pt)", "25 days", "ES row of Table 8.5, 2000-2009")),
        def!("rwtlc_ichi_cloud", "Ichimoku clouds crossover", C8, "Long/Short", Live, false,
            &["F|Senkou Span B = (HH + LL)/2 over 52 bars sent 26 ahead; Senkou Span A = (Tenkan + Kijun)/2 sent 26 ahead (p.150)",
              "E|Long Entry: Imoku1(@,9)[-1] > Imoku2(@,26)[-1] AND Close(@)[-1] > Imoku3(@,52)[-27] AND Close(@)[-1] > Imoku4(@,9,26,Sim,1)[-27] (p.154)",
              "X|Long Exit: Imoku1(@,9)[-1] < Imoku2(@,26)[-1] OR Close(@)[-1] < Imoku3(@,52)[-27] OR Close(@)[-1] < Imoku4(@,9,26,Sim,1)[-27] (p.154)",
              "E|Short Entry / Short Exit: the mirror (p.154)",
              "X|Read: [-27] = the span computed 26 bars before the signal bar, i.e. the cloud plotted under that bar. Code (level conditions) wins over the prose's \"crossovers to occur above the clouds\"."],
            FUT, es("106", "28.30%", "$1,302 total (ES, $50/pt)", "19 days", "ES row of Table 8.6, 2000-2009")),
        def!("rwtlc_rsix", "RSI Extremes with 200-day MA, $7,500 stop", C8, "Long/Short", Live, false,
            &["E|Long Entry: RSI(@,9)[-1] < 35 AND Close(@)[-1] > MA(@,Sim,200)[-1] (p.159)",
              "X|Long Exit: RSI(@,9)[-1] > 65 OR OpenPositionAverageEntryPrice(@,ThisTradeOnly) - Dollar2Price(@,7500) / OpenPositionSize(@,ThisTradeOnly) (p.159)",
              "E|Short Entry: RSI(@,9)[-1] > 65 AND Close(@)[-1] < MA(@,Sim,200)[-1] (p.159)",
              "X|Short Exit: RSI(@,9)[-1] < 35 OR the same $7,500 stop (p.159) - \"a fail-safe stop loss exit of $7,500 per contract\" (p.157)",
              "X|$7,500 on one ES contract ($50 per S&P point) = 150 index points (param pts; instrument swap keeps the points)."],
            FUT, es("32", "81.25%", "$25,980 total (ES, $50/pt)", "31 days", "ES row of Table 8.8, 2000-2009")),
        def!("rwtlc_rsix_vol", "RSI Extremes with volume filter", C8, "Long/Short", Live, true,
            &["E|Long Entry: RSI(@,9)[-1] < 35 AND Close(@)[-1] > MA(@,Sim,200)[-1] AND Vol(@)[-2] > Vol(@)[-1] (p.159)",
              "E|Short Entry: RSI(@,9)[-1] > 65 AND Close(@)[-1] < MA(@,Sim,200)[-1] AND Vol(@)[-2] > Vol(@)[-1] (p.160)",
              "X|Exits as rwtlc_rsix: RSI(@,9)[-1] > 65 (long) / < 35 (short) OR the $7,500 stop (p.159-160)"],
            "Needs volume (NIFTY 50 cash index has none in the vault; SPX has). Book: futures with exchange volume.",
            es("20", "70%", "$10,100 total (ES, $50/pt)", "32 days", "ES row of Table 8.9, 2000-2009")),
        // ---- not testable
        def!("rwtlc_bb_rsi_ct", "Bollinger Band + RSI countertrend, $1,000 stop (Fig 2.10)", CD, "Long/Short",
            NotTestable("entry thresholds not stated: \"enters when both Bollinger Bands and a relative strength index signal extreme overbought or oversold levels\" gives no band width, RSI period or RSI levels (p.32-33); the chart (QQQ, 1,000 shares) is the only source"), false,
            &["E|\"a countertrend system that enters when both Bollinger Bands and a relative strength index signal extreme overbought or oversold levels\" (p.32)",
              "X|\"Exit occurs with profit if the market returns to the previous day's 20-day simple moving average or a stop loss order is triggered when the asset violates a $1,000 monetary stop loss level\" (p.33)"], "", None),
        def!("rwtlc_rsix_fx", "RSI Extremes, cash GBP/USD with $350 stop (Fig 2.1, 2.11, 3.1)", CD, "Long/Short",
            NotTestable("parameters of this version are not printed (only its $350 stop and results on spot GBP/USD, p.25-36, 54); the coded RSI Extremes is rwtlc_rsix"), false,
            &["E|\"RSI extremes trading system for cash foreign exchange British pound-U.S. dollar used a $350 stop loss\" (p.35)"], "", None),
        def!("rwtlc_regret_min", "Regret minimization: half off at 50% of 10-day ATR, rest to breakeven (Ch.6)", CD, "Long/Short",
            NotTestable("entries are buy/sell stops at \"classical technical horizontal support and resistance levels\" chosen by eye in a low-volatility ATR environment with no ATR threshold (p.107-112); the trailing stop is chosen \"based on our psychological temperament\""), false,
            &["F|low volatility \"as defined by its 10-day average true range\" - no threshold (p.107)", "E|buy stop / sell stop at horizontal resistance / support (p.107)",
              "X|\"significant unrealized profit level ... 50 percent of the market's 10-day average true range\": take 50% at the next open, stop on the rest to breakeven; trail with 1-, 3- or 5-day stop (p.107-108)"], "", None),
        def!("rwtlc_pyramid", "Pyramiding / staggered Fibonacci limit entries (Ch.6)", CD, "Long/Short",
            NotTestable("resistance levels, cycle lows / highs and add-on prices are chart judgement (wheat, T-note, IBM, natural gas examples p.116-123)"), false,
            &["E|buy stop at resistance, adds at higher prices with stops raised to prior entry (p.116-117)", "E|limit entries at 38.2/50/61.8% retracements of a cycle swing (p.120-122)"], "", None),
        def!("rwtlc_timeframe", "Timeframe confirmation / divergence (Ch.7)", CD, "Long/Short",
            NotTestable("intraday bars (2-, 5-, 30-, 60-minute) for the short-term versions; the daily crude-oil version places its stop \"below recent cycle lows according to the longer (weekly) timeframe\" - a judgement level (p.131-141)"), false,
            &["E|daily RSI(9) < 35 while weekly and monthly RSI(9) are bullish (p.139-141)", "X|stop below weekly cycle lows; profit at the high of the bar before the oversold bar (p.141)"], "", None),
        def!("rwtlc_classical", "Fibonacci, divergence and trendline models (Ch.8 nonmechanical)", CD, "Long/Short",
            NotTestable("cycle highs/lows, divergences and trendlines are drawn by eye; the book calls them nonmechanical models (p.162-170)"), false,
            &["E|sell at 38.2/50/61.8% retracement, stop above 61.8%, exit at prior cycle lows (p.163-164)", "E|RSI (and volume) divergence: sell next open, stop at the prior high, 50% at cycle lows, 1-day trailing stop (p.165-167)"], "", None),
        def!("rwtlc_orb_gap", "Opening range breakout / gap trading (equities)", CD, "Long/Short",
            NotTestable("intraday bars (first 15 minutes, 15-minute bars) and an opening range length left to \"each individual trader's personality\" (p.171-174)"), false,
            &["E|buy stop one tick above the 15-minute opening range high, stop one tick below its low (p.172)", "X|limit for half one tick above the breakout bar's high, previous-bar low trailing stop on the rest (p.172-173)"], "", None),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let d = catalogue().into_iter().find(|d| d.id == id)?;
    let fam = match id {
        "rwtlc_adx_lowvol_bb" | "rwtlc_adx_hivol_ct" => ("rwtlc_vol", "Weissman · volatility-cycle models"),
        "rwtlc_rsi_trend3" | "rwtlc_rsi_trend1" | "rwtlc_rsi_trend10" | "rwtlc_rsi_trend_ext" | "rwtlc_rsi_trend3_time" => ("rwtlc_rsitrend", "Weissman · RSI Trend"),
        "rwtlc_ichi_x" | "rwtlc_ichi_cloud" => ("rwtlc_ichi", "Weissman · Ichimoku"),
        "rwtlc_rsix" | "rwtlc_rsix_vol" | "rwtlc_rsix_fx" | "rwtlc_bb_rsi_ct" => ("rwtlc_rsix", "Weissman · RSI Extremes (TLAC)"),
        _ => ("rwtlc_disc", "Weissman · discretionary models (TLAC)"),
    };
    Some((fam.0, fam.1, d.name))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let rsi_t = |trail: f64| vec![p("rsi_n", "RSI period", 9., 2., 30., 1.), p("hi", "Long entry RSI above", 65., 50., 90., 1.),
        p("lo", "Short entry RSI below", 35., 10., 50., 1.), p("trail", "Trailing stop bars (LoLevel/HiLevel)", trail, 1., 30., 1.)];
    Some(match id {
        "rwtlc_adx_lowvol_bb" => vec![p("bb_n", "Bollinger days", 20., 10., 50., 1.), p("k", "Standard deviations", 2., 1., 3., 0.25),
            p("adx_n", "ADX period", 10., 5., 30., 1.), p("adx_lt", "ADX below (2 bars back)", 20., 10., 40., 1.),
            p("trail", "Trailing stop bars", 3., 1., 20., 1.), p("adx_wilder", "ADX: 0 book average, 1 Wilder smoothing", 0., 0., 1., 1.)],
        "rwtlc_adx_hivol_ct" => vec![p("adx_n", "ADX period", 10., 5., 30., 1.), p("adx_gt", "ADX above", 50., 30., 70., 1.),
            p("rsi_n", "RSI period", 9., 2., 30., 1.), p("lo", "Long entry RSI below", 35., 10., 50., 1.), p("hi", "Short entry RSI above", 65., 50., 90., 1.),
            p("ma", "Profit target MA days", 20., 5., 50., 1.), p("adx_wilder", "ADX: 0 book average, 1 Wilder smoothing", 0., 0., 1., 1.)],
        "rwtlc_rsi_trend3" => rsi_t(3.),
        "rwtlc_rsi_trend1" => rsi_t(1.),
        "rwtlc_rsi_trend10" => rsi_t(10.),
        "rwtlc_rsi_trend_ext" => { let mut v = rsi_t(10.); v.push(p("xl", "Long exit RSI above", 80., 60., 95., 1.)); v.push(p("xs", "Short exit RSI below", 20., 5., 40., 1.)); v }
        "rwtlc_rsi_trend3_time" => { let mut v = rsi_t(3.); v.push(p("hold", "Exit when bars since entry exceed", 1., 1., 10., 1.)); v }
        "rwtlc_ichi_x" => vec![p("tenkan", "Tenkan bars", 9., 3., 20., 1.), p("kijun", "Kijun bars", 26., 10., 60., 1.)],
        "rwtlc_ichi_cloud" => vec![p("tenkan", "Tenkan bars", 9., 3., 20., 1.), p("kijun", "Kijun bars", 26., 10., 60., 1.),
            p("senkou", "Senkou B bars", 52., 26., 104., 1.), p("shift", "Cloud displacement bars", 26., 10., 52., 1.)],
        "rwtlc_rsix" | "rwtlc_rsix_vol" => vec![p("rsi_n", "RSI period", 9., 2., 30., 1.), p("lo", "Long entry RSI below", 35., 10., 50., 1.),
            p("hi", "Short entry RSI above / long exit", 65., 50., 90., 1.), p("ma", "Trend MA days", 200., 50., 300., 10.),
            p("pts", "Fail-safe stop, index points ($7,500 / $50 ES)", 150., 10., 500., 10.)],
        _ => return None,
    })
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !id.starts_with("rwtlc_") { return None; }
    let schema = params(id)?;
    if from >= to || to >= x.r.len() { return Some(vec![]); }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let u = |k: &str| g(k).round().max(1.0) as usize;
    let c = &x.c;
    let trail = |n: usize| move |t: usize, q: &Pos| {
        if t < n { return (None, None); }
        (Some(if q.dir > 0.0 { lo_n(x, t - 1, n) } else { hi_n(x, t - 1, n) }), None)
    };
    Some(match id {
        "rwtlc_adx_lowvol_bb" => {
            let (n, k, lt) = (u("bb_n"), g("k"), g("adx_lt"));
            let (m, s, a) = (sma(c, n), sd(c, n), adx(x, u("adx_n"), g("adx_wilder") >= 0.5));
            sim(x, from, to, &|i| {
                if i < 1 || !(a[i - 1] < lt) { return 0; }
                if c[i] > m[i] + k * s[i] { 1 } else if c[i] < m[i] - k * s[i] { -1 } else { 0 }
            }, &no_cx, &trail(u("trail")))
        }
        "rwtlc_adx_hivol_ct" => {
            let (a, r, m) = (adx(x, u("adx_n"), g("adx_wilder") >= 0.5), rsi(c, u("rsi_n")), sma(c, u("ma")));
            let (gt, lo, hi) = (g("adx_gt"), g("lo"), g("hi"));
            sim(x, from, to, &|i| {
                if i < 1 || !(a[i] > gt) { return 0; }
                if x.h[i] > x.h[i - 1] && r[i] < lo { 1 } else if x.l[i] < x.l[i - 1] && r[i] > hi { -1 } else { 0 }
            }, &no_cx, &|t, q| (Some(if q.dir > 0.0 { x.l[t - 1] } else { x.h[t - 1] }), Some(m[t - 1])))
        }
        "rwtlc_rsi_trend3" | "rwtlc_rsi_trend1" | "rwtlc_rsi_trend10" | "rwtlc_rsi_trend_ext" | "rwtlc_rsi_trend3_time" => {
            let r = rsi(c, u("rsi_n"));
            let (hi, lo) = (g("hi"), g("lo"));
            let ent = |i: usize| if r[i] > hi { 1 } else if r[i] < lo { -1 } else { 0 };
            match id {
                "rwtlc_rsi_trend_ext" => {
                    let (xl, xs) = (g("xl"), g("xs"));
                    sim(x, from, to, &ent, &|t, q| {
                        let s = t - 1;
                        if (q.dir > 0.0 && r[s] > xl) || (q.dir < 0.0 && r[s] < xs) { Some("RSI extreme reading exit") } else { None }
                    }, &trail(u("trail")))
                }
                "rwtlc_rsi_trend3_time" => {
                    let hold = u("hold");
                    sim(x, from, to, &ent, &|t, q| if t - q.ei > hold { Some("time exit (bars since entry)") } else { None }, &trail(u("trail")))
                }
                _ => sim(x, from, to, &ent, &no_cx, &trail(u("trail"))),
            }
        }
        "rwtlc_ichi_x" => {
            let (tk, kj) = (mid(x, u("tenkan")), mid(x, u("kijun")));
            sim(x, from, to, &|i| if tk[i] > kj[i] { 1 } else if tk[i] < kj[i] { -1 } else { 0 }, &|t, q| {
                let s = t - 1;
                if (q.dir > 0.0 && tk[s] < kj[s]) || (q.dir < 0.0 && tk[s] > kj[s]) { Some("Tenkan/Kijun cross back") } else { None }
            }, &|_, _| (None, None))
        }
        "rwtlc_ichi_cloud" => {
            let (tk, kj, sb) = (mid(x, u("tenkan")), mid(x, u("kijun")), mid(x, u("senkou")));
            let sh = u("shift");
            let sa: Vec<f64> = tk.iter().zip(&kj).map(|(a, b)| (a + b) / 2.0).collect();
            // cloud under signal bar s = spans computed at s - shift (CQG [-27] with signals on [-1])
            let above = |s: usize| s >= sh && c[s] > sb[s - sh] && c[s] > sa[s - sh];
            let below = |s: usize| s >= sh && c[s] < sb[s - sh] && c[s] < sa[s - sh];
            sim(x, from, to, &|i| if tk[i] > kj[i] && above(i) { 1 } else if tk[i] < kj[i] && below(i) { -1 } else { 0 }, &|t, q| {
                let s = t - 1;
                if s < sh { return None; }
                let out = if q.dir > 0.0 { tk[s] < kj[s] || c[s] < sb[s - sh] || c[s] < sa[s - sh] }
                          else { tk[s] > kj[s] || c[s] > sb[s - sh] || c[s] > sa[s - sh] };
                if out { Some("crossover or cloud condition lost") } else { None }
            }, &|_, _| (None, None))
        }
        "rwtlc_rsix" | "rwtlc_rsix_vol" => {
            let (r, m) = (rsi(c, u("rsi_n")), sma(c, u("ma")));
            let (lo, hi, pts) = (g("lo"), g("hi"), g("pts"));
            let vf = id == "rwtlc_rsix_vol";
            let vol = |i: usize| x.r[i].volume.unwrap_or(f64::NAN);
            sim(x, from, to, &|i| {
                if vf && !(i >= 1 && vol(i - 1) > vol(i)) { return 0; }
                if r[i] < lo && c[i] > m[i] { 1 } else if r[i] > hi && c[i] < m[i] { -1 } else { 0 }
            }, &|t, q| {
                let s = t - 1;
                if (q.dir > 0.0 && r[s] > hi) || (q.dir < 0.0 && r[s] < lo) { Some("RSI exit level") } else { None }
            }, &|_, q| (Some(q.epx - q.dir * pts), None))
        }
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn ctx_rows(close: &[f64]) -> Vec<Row> {
        close.iter().enumerate().map(|(i, &c)| Row { ticker: "T".into(), date: format!("2000-01-{:02}", i % 28 + 1),
            open: c, high: c + 1.0, low: c - 1.0, close: c, volume: Some(1.0) }).collect()
    }
    #[test]
    fn rsi_trend3_enters_next_open_and_stops_at_prior_three_bar_low() {
        // 30 rising closes push RSI(9) to 100 (> 65): long at the open after the first signal bar.
        let mut c: Vec<f64> = (0..30).map(|i| 100.0 + i as f64).collect();
        c.extend([110.0, 109.0]); // sharp drop: bar 30 opens/lows below the prior three-bar low
        let rows = ctx_rows(&c);
        let x = Ctx::new(&rows, &rows, &[]);
        let t = run("rwtlc_rsi_trend3", &x, 0, rows.len() - 1, &Params::new()).unwrap();
        assert!(!t.is_empty());
        let first = &t[0];
        assert_eq!(first.dir, 1.0);
        assert_eq!(first.legs[0].px, x.o[first.legs[0].i]);
        // bar 30 opens at 110 <= lowest low of bars 27..29 (126) -> gap stop at the open
        assert_eq!(first.exit_i, 30);
        assert_eq!(first.exit_px, 110.0);
        assert!(first.reason.starts_with("stop"));
    }
}
