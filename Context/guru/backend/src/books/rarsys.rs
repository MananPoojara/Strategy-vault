//! Systems recovered from RAR archives that were blocked until a libarchive (ctypes) reader was written.
//! Sources (research drive, read in place; the TradeStation code was extracted with libarchive into a scratch dir and read as data):
//! - Shay Campbell, Trend Weaver (Timing Charts LLC, 2006): `Code.rar` -> `TradeStation 2000i.txt` / `TradeStation - 8.txt`,
//!   plus `Trend Weaver Trading Rules.pdf` (p.3-9).
//! - Trade-to-Win (tradingsystemsthatwork.com, 2008): Crouching Tiger `Code.rar` -> `Code.txt`, plus `CTS manual.pdf`.
//! - Trade-to-Win: Swing Traders Edge `STE_ELD CODE.rar` -> `STE_ELD CODE.txt`, plus `STEmanual.pdf`.
//! - Weaver-DT (alfaranda.com) `Code.rar` -> `Code.txt` (catalogued, not testable).
//! Book id "rarsys". Notes: library/rarsys/README.md.
use crate::connors::{dmi, hv_ddof, sma, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "rarsys";

const TW: &str = "rarsys_trend_weaver";
const CTS: [&str; 3] = ["rarsys_cts_std", "rarsys_cts_plus", "rarsys_cts_stocks"];
const STE: &str = "rarsys_ste";
const STE_PF: &str = "rarsys_ste_portfolio";
const WDT: &str = "rarsys_weaver_dt";

fn def(id: &'static str, name: &'static str, group: &'static str, dir: &'static str, status: Status, vol: bool,
       rules: &'static [&'static str], notes: &'static str) -> StrategyDef {
    StrategyDef { id, name, group, session: 0, dir, status, needs_volume: vol, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None }
}

const CTS_RULES: &[&str] = &[
    "F|Code.txt: \"ma=average(c,MAlength)\" MAlength 200; \"r=rsi(c, RSIlength)\" RSIlength 2",
    "E|Code.txt: \"Lcount = number of the last NoBars bars with r < RSIentry (40); if Lcount=NoBars and c>ma then buy next bar at open\"",
    "A|CTS manual: \"If at least one day has passed and the first two rules are still valid then enter a second long position at the market open the next day.\" (not in Code.txt)",
    "X|Code.txt: \"if r>RSIexit (70) then sell next bar at open\"",
    "E|Code.txt: \"Scount = bars with r > 100-RSIentry (60); if Scount=NoBars and c<ma then sellshort next bar at open\"",
    "A|CTS manual: second short position on the same terms",
    "X|Code.txt: \"if r<(100-RSIexit) (30) then buytocover next bar at open\"",
];
const CTS_NOTES: &str = "Code.txt (Crouching Tiger Code.rar) is the exact statement of the entry and exit and matches the manual's rules 1, 2 and 4 for all three variants (NoBars 3 / 4 / 5). \
The manual's rule 3 (one second unit when the rules are still valid at least one day after the first signal) is not in Code.txt; it is kept as the default because the manual states it exactly \
(S&P example: 'The RSI closes a fourth day below 40 and a second position is taken'). Each unit = half the allocation; p.second_unit=0 gives the Code.txt-literal single unit. \
Code.txt orders are independent, so an opposite entry reverses at the same open (TradeStation buy/sellshort reverse a position). RSI = Wilder (TradeStation RSI is the same Wilder smoothing). \
Published results are index-point tables (S&P 500 / NASDAQ 100, 1/1/95-1/1/08) printed as images; numbers not in the text layer. India: instrument swap; index shorts hypothetical.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        def(TW, "Trend Weaver (150-day trend, 20-day range stops)", "Trend Weaver (Code.rar + Rules p.3-9)", "Long/Short", Status::Live, false, &[
            "F|\"TradeRange = (Highest(H,20) - Lowest(L,20))/100\"; \"Top_Range = Lowest(Low,150)+(Highest(High,150)-Lowest(Low,150))*.99\"; \"Bottom_Range = ... *.01\"",
            "F|\"If high > Top_Range then TrendDirection = 1; if low < Bottom_Range then TrendDirection = -1\"",
            "E|\"If TrendDirection = 1 then Buy next bar at Long_Entry_Price stop\" (Lowest(L,20)+TradeRange*90)",
            "E|\"If TrendDirection = -1 then Sell short Next Bar at Short_Entry_Price stop\" (Lowest(L,20)+TradeRange*10)",
            "E|\"If TrendDirection = -1 then Buy next bar at Top_Range stop; If TrendDirection = 1 then Sell short Next Bar at Bottom_Range stop\"",
            "X|\"If marketposition >= 1 then Sell next bar at LongExit stop\" (Lowest(L,20)+TradeRange*40)",
            "X|\"If marketposition <= -1 then Buy to Cover Next Bar at ShortExit stop\" (Lowest(L,20)+TradeRange*60)",
        ],
        "Code (TS 2000i and TS 8 files, identical logic) is the rule source; it confirms the Rules PDF (p.3 trend, p.5 stops 1% inside the 150-day range, p.8 90/40/10/60% of the 20-day range). \
Where they differ the code wins: the trend flips when the high exceeds 99% of the 150-day range (Highest/Lowest include today), not only on a new 150-day high; the 1%-inside reversal stops are placed on every bar of the opposite trend, not 'only once'. \
Fills [Assumption, code silent - TradeStation daily-bar path]: stops fill at the level, or at the open when the open is already through it; the bar runs open -> nearer extreme -> other extreme -> close; \
every order placed at the prior close can fill once when the path crosses its level, if it applies to the position at that moment (entries ignored when already in that direction - no pyramiding; an opposite entry reverses), so an exit and a new entry can fill on one bar. \
Built for futures (day-session data); Performance xls 1980-2006 is a futures portfolio in dollars (not comparable). India: instrument swap; index shorts hypothetical."),
        def(CTS[0], "Crouching Tiger Standard (RSI(2) < 40 for 3 days above SMA200)", "Crouching Tiger (Code.rar + manual)", "Long/Short", Status::Live, false, CTS_RULES, CTS_NOTES),
        def(CTS[1], "Crouching Tiger + (4 days)", "Crouching Tiger (Code.rar + manual)", "Long/Short", Status::NotTestable("duplicate of t2w_cts_plus: the vendor code gives identical trades on NIFTY 50 and SPX, so it is listed once"), false, CTS_RULES, CTS_NOTES),
        def(CTS[2], "Crouching Tiger for Stocks (5 days)", "Crouching Tiger (Code.rar + manual)", "Long/Short", Status::NotTestable("duplicate of t2w_cts_stock: the vendor code gives identical trades on NIFTY 50 and SPX, so it is listed once"), false, CTS_RULES, CTS_NOTES),
        def(STE, "Swing Traders Edge (single stock)", "Swing Traders Edge (STE_ELD CODE.rar + manual p.4)", "Long", Status::Live, true, &[
            "F|\"close > AverageFC(close,200) and C<C[1] and C[1]<C[2] and C[2]<C[3] and C[3]<C[4] and C < O and C < AverageFC(C,5)\"",
            "F|\"ADX(10) > 20 and VolatilityStdDev(100) * 100 > 20 and AverageFC(C*V, 20) > 4000000\"",
            "E|\"if MarketPosition = 0 and ... Then buy (\"STE\") next bar at 0.97*C Limit\" (manual: valid for that day only)",
            "X|\"if MarketPosition = 1 and C > AverageFC(C,5) then Sell Next Bar at Market\"",
        ],
        "Contradiction: the manual's rule 8 is 'C >= 5-day SMA'; the code uses 'C >'. Code is the default; p.exit_ge=1 is the manual's reading. \
The manual's portfolio layer (10 slots, ranking by 4-day % drop) is catalogued separately (rarsys_ste_portfolio). The per-stock rules are complete and run on any ticker with volume: the book's market is US stocks, so run it on NSE basket stocks (thresholds literal, money flow in rupees) - on indices the C*V filter is vacuous. \
[Assumption, code silent] VolatilityStdDev(100) = population stdev of ln(C/C[1]) over 100 days x sqrt(252) (TradeStation StdDev is population; p.hv_sample=1 = sample stdev). ADX(10) = Wilder. Long only, no stop (manual 'Stops' section: original has none; 10/25/50% stops are only in result tables)."),
        def(STE_PF, "Swing Traders Edge portfolio (10 slots, 4-day-drop ranking; Aggressive = 2 slots per stock)", "Swing Traders Edge (manual)", "Long",
            Status::NeedsData("The book's universe ('a universe of 10,000 stocks', US 2000-2006) ranked each night into 10 equal slots is not in the vault; the per-stock rules run as rarsys_ste."), true,
            &["F|manual: 'portfolio size factor of 10'; rank candidates by (C[4]-C)/C[4]x100, highest first; a stock fills one slot (Aggressive: up to two)"],
            "Published: monthly returns 2000-2006 (Original, 10/25/50% stop, Aggressive), commission 1 cent/share."),
        def(WDT, "Weaver-DT trendline breakout", "Weaver-DT (Code.rar)", "Long/Short",
            Status::NotTestable("The target (ADD) is the widest swing measured only over bars 'IF W<=MAXBARSBACK', a TradeStation chart setting the code does not state, so the exit price depends on an unstated value."), false,
            &["E|\"Buy Next Bar LINE STOP\" / \"Sell Short Next Bar ULINE STOP\" (trendlines through the last two lower pivot highs / higher pivot lows, STRENGTH 1)", "X|\"Sell Next Bar LONGX LIMIT\" (LONGX = LINE+ADD); \"Buy to Cover Next Bar SHORTX LIMIT\""],
            "No manual in the folder (Code.rar, WEAVER-DT.ELD/.ELS only)."),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        TW => ("rarsys_trend_weaver", "Trend Weaver (Campbell)", "TradeStation code literal"),
        "rarsys_cts_std" => ("rarsys_cts", "Crouching Tiger (Trade-to-Win)", "Standard, 3 days"),
        "rarsys_cts_plus" => ("rarsys_cts", "Crouching Tiger (Trade-to-Win)", "+, 4 days"),
        "rarsys_cts_stocks" => ("rarsys_cts", "Crouching Tiger (Trade-to-Win)", "Stocks, 5 days"),
        STE => ("rarsys_ste", "Swing Traders Edge (Trade-to-Win)", "single stock"),
        STE_PF => ("rarsys_ste", "Swing Traders Edge (Trade-to-Win)", "portfolio"),
        WDT => ("rarsys_weaver_dt", "Weaver-DT (alfaranda)", "code"),
        _ => return None,
    })
}

fn cts_bars(id: &str) -> Option<f64> {
    CTS.iter().position(|&c| c == id).map(|k| 3. + k as f64)
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    if id == TW {
        return Some(vec![p("trend_len", "TrendLength", 150., 50., 300., 10.), p("trade_len", "TradeLength", 20., 5., 60., 1.),
            p("long_pct", "LongPercent (buy stop, % of range)", 90., 50., 100., 5.), p("short_pct", "ShortPercent (sell stop)", 10., 0., 50., 5.),
            p("exitlong_pct", "ExitLong_Percent", 40., 0., 90., 5.), p("exitshort_pct", "ExitShort_Percent", 60., 10., 100., 5.)]);
    }
    if let Some(nb) = cts_bars(id) {
        return Some(vec![p("nobars", "NoBars (days RSI beyond entry level)", nb, 1., 8., 1.), p("ma_len", "MAlength", 200., 50., 300., 10.),
            p("rsi_len", "RSIlength", 2., 2., 14., 1.), p("rsi_entry", "RSIentry (long below; short above 100-x)", 40., 10., 50., 5.),
            p("rsi_exit", "RSIexit (long above; short below 100-x)", 70., 50., 90., 5.),
            p("second_unit", "Second unit (manual rule 3; 0 = Code.txt single unit)", 1., 0., 1., 1.)]);
    }
    (id == STE).then(|| vec![p("limit_pct", "Limit below close (%)", 3., 0., 10., 0.5), p("adx_min", "ADX(10) above", 20., 0., 40., 1.),
        p("hv_min", "HV(100) above", 20., 0., 60., 1.), p("mf_min", "20-day avg C*V above", 4_000_000., 0., 100_000_000., 1_000_000.),
        p("exit_ge", "Exit on C >= SMA5 (manual) instead of C > SMA5 (code)", 0., 0., 1., 1.), p("hv_sample", "HV with sample stdev", 0., 0., 1., 1.)])
}

fn roll(v: &[f64], n: usize, hi: bool) -> Vec<f64> {
    (0..v.len()).map(|i| if i + 1 < n { f64::NAN } else {
        let w = &v[i + 1 - n..=i];
        if hi { w.iter().copied().fold(f64::MIN, f64::max) } else { w.iter().copied().fold(f64::MAX, f64::min) }
    }).collect()
}

fn trade(dir: f64, i: usize, px: f64, w: f64, kind: &'static str) -> Trade {
    Trade { dir, legs: vec![Leg { i, px, w }], exit_i: i, exit_px: px, reason: String::new(), entry_kind: kind, signal_i: i, open: false, exits: vec![] }
}

fn close(out: &mut Vec<Trade>, cur: &mut Option<Trade>, i: usize, px: f64, sig: usize, reason: &str) {
    if let Some(mut t) = cur.take() { t.exit_i = i; t.exit_px = px; t.signal_i = sig; t.reason = reason.into(); out.push(t); }
}

fn mark_open(out: &mut Vec<Trade>, cur: Option<Trade>, x: &Ctx, to: usize) {
    if let Some(mut t) = cur { t.exit_i = to; t.exit_px = x.c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true; out.push(t); }
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Ord { BuyE, SellE, XL, XS }

/// One daily bar of stop orders, TradeStation path O -> nearer extreme -> far extreme -> C.
/// Returns the fills (price, order) in path order and updates `pos`.
fn stop_bar(o: f64, h: f64, l: f64, c: f64, orders: &[(f64, Ord, &'static str)], pos: &mut i8) -> Vec<(f64, Ord, &'static str)> {
    let up = |k: Ord| matches!(k, Ord::BuyE | Ord::XS);
    let applies = |k: Ord, pos: i8| match k { Ord::BuyE => pos <= 0, Ord::SellE => pos >= 0, Ord::XL => pos == 1, Ord::XS => pos == -1 };
    let mut fills = vec![];
    let apply = |px: f64, k: Ord, name: &'static str, pos: &mut i8, fills: &mut Vec<_>| {
        if applies(k, *pos) {
            *pos = match k { Ord::BuyE => 1, Ord::SellE => -1, _ => 0 };
            fills.push((px, k, name));
        }
    };
    // gap through at the open: exits first, then entries
    for pass in [true, false] {
        for &(lv, k, nm) in orders {
            let is_exit = matches!(k, Ord::XL | Ord::XS);
            if is_exit == pass && if up(k) { lv <= o } else { lv >= o } { apply(o, k, nm, pos, &mut fills); }
        }
    }
    let hi_first = h - o <= o - l;
    let path = if hi_first { [o, h, l, c] } else { [o, l, h, c] };
    for s in path.windows(2) {
        let (a, b) = (s[0], s[1]);
        let mut cand: Vec<_> = orders.iter().copied().filter(|&(lv, k, _)| if b > a { up(k) && lv > a && lv <= b } else if b < a { !up(k) && lv < a && lv >= b } else { false }).collect();
        if b > a { cand.sort_by(|p, q| p.0.total_cmp(&q.0)) } else { cand.sort_by(|p, q| q.0.total_cmp(&p.0)) }
        for (lv, k, nm) in cand { apply(lv, k, nm, pos, &mut fills); }
    }
    fills
}

fn run_tw(x: &Ctx, from: usize, to: usize, p: &Params) -> Vec<Trade> {
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d);
    let (tl, dl) = (g("trend_len", 150.).max(2.) as usize, g("trade_len", 20.).max(1.) as usize);
    let (lp, sp, xlp, xsp) = (g("long_pct", 90.), g("short_pct", 10.), g("exitlong_pct", 40.), g("exitshort_pct", 60.));
    let (hh, ll, h20, l20) = (roll(&x.h, tl, true), roll(&x.l, tl, false), roll(&x.h, dl, true), roll(&x.l, dl, false));
    let n = x.c.len();
    // TrendDirection is a pure function of the data (computed from the first bar).
    let mut td = vec![0i8; n];
    let mut t = 0i8;
    for i in 0..n {
        if !hh[i].is_nan() {
            if x.h[i] > ll[i] + (hh[i] - ll[i]) * 0.99 { t = 1; }
            if x.l[i] < ll[i] + (hh[i] - ll[i]) * 0.01 { t = -1; }
        }
        td[i] = t;
    }
    let mut out = vec![];
    let mut cur: Option<Trade> = None;
    let mut pos = 0i8;
    for i in from..to {
        // orders from the close of bar i, filled on bar i+1
        if hh[i].is_nan() || h20[i].is_nan() { continue; }
        let tr = (h20[i] - l20[i]) / 100.;
        let (top, bot) = (ll[i] + (hh[i] - ll[i]) * 0.99, ll[i] + (hh[i] - ll[i]) * 0.01);
        let mut ords = vec![];
        if td[i] == 1 { ords.push((l20[i] + tr * lp, Ord::BuyE, "LongEntry")); ords.push((bot, Ord::SellE, "ShortEnt")); }
        if td[i] == -1 { ords.push((l20[i] + tr * sp, Ord::SellE, "ShortEntry")); ords.push((top, Ord::BuyE, "LongEnt")); }
        if pos >= 1 { ords.push((l20[i] + tr * xlp, Ord::XL, "LongExit")); }
        if pos <= -1 { ords.push((l20[i] + tr * xsp, Ord::XS, "ShortExit")); }
        let j = i + 1;
        for (px, k, nm) in stop_bar(x.o[j], x.h[j], x.l[j], x.c[j], &ords, &mut pos) {
            match k {
                Ord::XL | Ord::XS => close(&mut out, &mut cur, j, px, j, &format!("{nm} stop")),
                Ord::BuyE | Ord::SellE => {
                    close(&mut out, &mut cur, j, px, j, &format!("reversed by {nm} stop"));
                    cur = Some(trade(if k == Ord::BuyE { 1. } else { -1. }, j, px, 1., "stop"));
                }
            }
        }
    }
    mark_open(&mut out, cur, x, to);
    out
}

fn run_cts(x: &Ctx, from: usize, to: usize, p: &Params, nb_def: f64) -> Vec<Trade> {
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d);
    let nb = g("nobars", nb_def).max(1.) as usize;
    let (ent, ext) = (g("rsi_entry", 40.), g("rsi_exit", 70.));
    let two = g("second_unit", 1.) >= 0.5;
    let ma = sma(&x.c, g("ma_len", 200.).max(1.) as usize);
    let r = x.rsi_n(g("rsi_len", 2.).max(1.) as usize);
    let cnt = |i: usize, f: &dyn Fn(f64) -> bool| i + 1 >= nb && (0..nb).all(|k| f(r[i - k]));
    let cl = |i: usize| x.c[i] > ma[i] && cnt(i, &|v| v < ent);
    let cs = |i: usize| x.c[i] < ma[i] && cnt(i, &|v| v > 100. - ent);
    let w = if two { 0.5 } else { 1. };
    let mut out = vec![];
    let mut cur: Option<Trade> = None;
    let mut first_sig = 0usize;
    for i in from..to {
        let j = i + 1;
        let d = cur.as_ref().map_or(0., |t| t.dir);
        if d > 0. && r[i] > ext { close(&mut out, &mut cur, j, x.o[j], i, "RSI > exit level · next open"); }
        else if d < 0. && r[i] < 100. - ext { close(&mut out, &mut cur, j, x.o[j], i, "RSI < 100-exit level · next open"); }
        else if two && d != 0. && i > first_sig && (if d > 0. { cl(i) } else { cs(i) }) {
            if let Some(t) = cur.as_mut() { if t.legs.len() == 1 { t.legs.push(Leg { i: j, px: x.o[j], w }); } }
        }
        let d = cur.as_ref().map_or(0., |t| t.dir);
        let want = if cl(i) { 1. } else if cs(i) { -1. } else { 0. };
        if want != 0. && want != d {
            close(&mut out, &mut cur, j, x.o[j], i, "reversed · next open");
            cur = Some(trade(want, j, x.o[j], w, "open"));
            if let Some(t) = cur.as_mut() { t.signal_i = i; }
            first_sig = i;
        }
    }
    mark_open(&mut out, cur, x, to);
    out
}

fn run_ste(x: &Ctx, from: usize, to: usize, p: &Params) -> Vec<Trade> {
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d);
    let (lim, adx_min, hv_min, mf_min) = (g("limit_pct", 3.), g("adx_min", 20.), g("hv_min", 20.), g("mf_min", 4_000_000.));
    let ge = g("exit_ge", 0.) >= 0.5;
    let (ma200, ma5) = (sma(&x.c, 200), sma(&x.c, 5));
    let adx = dmi(x.r, 10).1;
    let hv = hv_ddof(&x.c, 100, if g("hv_sample", 0.) >= 0.5 { 1. } else { 0. });
    let cv: Vec<f64> = x.r.iter().map(|r| r.volume.map_or(f64::NAN, |v| v * r.close)).collect();
    let mf = sma(&cv, 20);
    let c = &x.c;
    let sig = |i: usize| i >= 4 && c[i] > ma200[i] && c[i] < c[i - 1] && c[i - 1] < c[i - 2] && c[i - 2] < c[i - 3] && c[i - 3] < c[i - 4]
        && c[i] < x.o[i] && c[i] < ma5[i] && adx[i] > adx_min && hv[i] > hv_min && mf[i] > mf_min;
    let mut out = vec![];
    let mut cur: Option<Trade> = None;
    for i in from..to {
        let j = i + 1;
        if cur.is_some() {
            if if ge { c[i] >= ma5[i] } else { c[i] > ma5[i] } { close(&mut out, &mut cur, j, x.o[j], i, "C > SMA5 · next open"); }
        } else if sig(i) {
            let px = (1. - lim / 100.) * c[i];
            if x.l[j] <= px { let mut t = trade(1., j, x.o[j].min(px), 1., "limit"); t.signal_i = i; cur = Some(t); }
        }
    }
    mark_open(&mut out, cur, x, to);
    out
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if from >= to || to >= x.r.len() { return None; }
    if id == TW { return Some(run_tw(x, from, to, p)); }
    if let Some(nb) = cts_bars(id) { return Some(run_cts(x, from, to, p, nb)); }
    (id == STE).then(|| run_ste(x, from, to, p))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stop_bar_exit_then_reverse_on_one_bar() {
        // short; bar opens 10, high first to 14: ShortExit at 12 then LongEntry at 13
        let mut pos = -1i8;
        let f = stop_bar(10., 14., 9.9, 13.5, &[(13., Ord::BuyE, "LE"), (12., Ord::XS, "SX"), (5., Ord::SellE, "SE")], &mut pos);
        assert_eq!(f.iter().map(|x| x.2).collect::<Vec<_>>(), vec!["SX", "LE"]);
        assert_eq!(pos, 1);
        // gap through a sell stop fills at the open
        let mut pos = 1i8;
        let f = stop_bar(8., 8.5, 7., 7.5, &[(9., Ord::XL, "LX")], &mut pos);
        assert_eq!((f[0].0, pos), (8., 0));
    }
}
