//! Urban Jaekle & Emilio Tomasini: Trading Systems - A New Approach to System Development and
//! Portfolio Optimisation, 2nd ed. (Harriman House, 2019). Page refs are PDF pages of
//! Books/Urban Jaekle & Emilio Tomasini/Trading Systems-Amibroker.pdf. Only the systems whose
//! code or rules the book prints in full are Live; WFO / Monte Carlo / money management /
//! equity-curve switching are analysis methods, listed in library/jt/README.md.
//! Conventions: signals on the close of bar i, orders for bar i+1; a stop fills at the level or
//! at the open when the bar gaps through it; exits are not checked on the entry bar; when a stop
//! and a target could both fill on one bar the stop is assumed first. Standard deviations are
//! population (TradeStation StdDev / BollingerBand, AmiBroker StDev / BBandTop).
use crate::connors::{simulate, sma, true_range, Ctx, Entry, Exit, Leg, ParamDef, Params, Spec, Status, StrategyDef, Trade};

const BOOK: &str = "jt";

const LUXOR_30M_GAP: &str = "needs 30-minute GBP/USD FOREX bars (GMT timestamps) for 21/10/2002-4/7/2008; the vault has daily bars only and no FX";
const BB_PORTFOLIO_GAP: &str = "needs point-in-time S&P 500 / Nasdaq 100 / S&P 1500 / ASX constituents incl. delisted stocks (Norgate) and a 20-slot portfolio simulator with momentum ranking; NSE baskets are current-membership snapshots (survivorship bias) so not a substitute";
const BOM_ROT_GAP: &str = "needs daily data for the book's 26 world stock indices (Table 5.6) to rank by 252-day ROC; the vault has 6 (SPX, NDX, DJI, NIFTY 50, NIFTYBANK, NIFTYMIDCAP100)";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, dir, status, rules, notes| StrategyDef {
        id, name, group, session: 0, dir, status, needs_volume: false, needs_vix: false,
        needs_series: &[], book: BOOK, rules, notes, claim: None,
    };
    use Status::*;
    vec![
        d("jt_example_20d_breakout", "Easy example: 20-day breakout with ATR stop", "Ch 1.1 · An easy example of a trading system", "Long/Short", Live, &[
            "E|\"Buy 2 contracts at the highest high in the last 20 days;\" (pdf p.24) = buy stop for the next bar at Highest(High,20) incl. today; fill at the level or the open if gapped above",
            "E|\"Sell short 2 contracts at the lowest low in the last 20 days;\" (pdf p.24) = sell-short stop at Lowest(Low,20); while long it reverses the position",
            "X|\"If marketposition = 1 then sell at last close - avgtruerange(14) stop;\" (pdf p.24); AvgTrueRange = TradeStation simple 14-bar average of true range, recomputed every close",
            "X|\"If marketposition = -1 then buy to cover at last close + avgtruerange(14) stop.\" (pdf p.24)",
            "E|A flat bar that reaches both breakout levels is skipped (order inside the bar unknown). [Book silent]",
        ], "Pseudo code; the book calls it 'a trading system' and notes its risk management is poor. Contracts = one full position here. Run on the index instead of a futures contract."),
        d("jt_luxor_30min", "LUXOR (30-minute GBP/USD, time window, 0.3%/0.8%/1.9% exits)", "Ch 3 · LUXOR step-by-step development", "Long/Short", NeedsData(LUXOR_30M_GAP), &[
            "F|Fast = Average(Close, FastLength); Slow = Average(Close, SlowLength); code inputs FastLength(3), SlowLength(30); developed values FAST=1, SLOW=44 (Text 3.1 pdf p.57-58; Fig 3.9)",
            "E|\"If Fast crosses above Slow then begin BuyStop = High + 1 point; BuyLimit = High + 5 points; ... If GoLong and C < BuyLimit then Buy next bar at BuyStop Stop\"; short symmetric (pdf p.58)",
            "F|Entries only inside the 4-hour window 9.30am-1.30pm GMT (section 3.4)",
            "X|0.3% risk stop loss, 0.8% trailing stop, 1.9% profit target (section 3.5, Table 3.4)",
        ], "30-minute FOREX system; book: 902 trades, $115,000 net, $128 avg (time filter, no exits); with all three exits net profit and avg improve (Table 3.4). Daily version: jt_luxor_daily."),
        d("jt_luxor_daily", "LUXOR on daily bars (7/26, slow-MA stop exit)", "Appendix 3 · Portfolios with the LUXOR system", "Long/Short", Live, &[
            "F|\"fast moving average length=7, slow moving average length=26\" on daily data (App. 3.3, pdf p.375); Fast = Average(Close,7), Slow = Average(Close,26)",
            "E|\"If Fast crosses above Slow then begin BuyStop = High + 1 point; BuyLimit = High + 5 points; end; If GoLong and C < BuyLimit then Buy next bar at BuyStop Stop\" (GoLong = Fast > Slow; Text 3.1, pdf p.57). The crossover bar's levels are kept while Fast > Slow",
            "E|\"If Fast crosses below Slow then begin SellStop = Low - 1 point; SellLimit = Low - 5 points; ... If GoShort and C > SellLimit then Sell Short next bar at SellStop Stop\" (pdf p.58); reverses a long",
            "X|\"Trades are only exited when the price crosses the slower moving average\" (App. 3.2) = the code's exit \"If MP = 1 then Sell next bar at Slow - 1 point Stop; If MP = -1 then Buy to Cover next bar at Slow + 1 point Stop\" (pdf p.58)",
            "F|No intraday time filter on daily bars (section 3.4 filter is for 30-minute FOREX; App. 3 'we do not apply any additional exits'). 1 point = 0.01 (EasyLanguage point = 1/PriceScale; 2-decimal index quotes). [Reading]",
        ], "Book tested 12 bond futures + a 6-market portfolio (Bund, T-Note, Mini S&P, Gold, Crude, Euro) May 1972-April 2006, one contract, $30 S&C; book: 'the Mini S&P produces a negative equity line' (pdf p.377); portfolio 3,168 trades, 37% winners (Table A3.1). Run on the index instead of the futures."),
        d("jt_bb_forex", "Bollinger Band counter-breakout (FOREX version, 1.9% stop / 4.8% target)", "Ch 5.2 · Out-of-sample deterioration", "Long/Short", Live, &[
            "F|\"inputs: LengthUp(18), NumDevsUp(2), LengthDn(20), NumDevsDn(2); Price = (O+H+L+C)/4; UpperBand = BollingerBandFC(Price, LengthUp, NumDevsUp); LowerBand = BollingerBandFC(Price, LengthDn, -NumDevsDn)\" (Fig 5.4B, pdf p.129)",
            "E|\"if CurrentBar > 1 and High crosses under UpperBand then Sell Short next bar at UpperBand stop\" (pdf p.129)",
            "E|\"if CurrentBar > 1 and Low crosses over LowerBand then Buy next bar at LowerBand stop\" (pdf p.129)",
            "X|\"Place an initial stop loss x % below entry price (Opt value: x = 1.9)\" (pdf p.129); short: above",
            "X|\"Place a profit target z % above entry price (Opt value: 4.8)\" (pdf p.129); short: below",
        ], "Book's example of an over-fitted 6-parameter system on GBP/USD daily 2002-2008 (training 69 trades, $78,000; test $379, Table 5.1). Run on the index instead of GBP/USD. Cross = today below, yesterday at/above."),
        d("jt_bom", "Beginning of Month (BOM)", "Ch 5.6 · A simple, robust trading system", "Long", Live, &[
            "E|\"Buy a stock market index every last trading day of each month on close.\" (pdf p.159; AmiBroker isLastOfMonth, pdf p.391)",
            "F|\"do not enter when the market (on the month end) closes below the 40-day moving average\" (pdf p.159); code: C > Average(Close,40)",
            "X|\"Stop Loss: exit when the market drops below the 40-day moving average, exit on close of any day.\" (pdf p.159)",
            "X|\"Time exit: Exit at the end of the fourth trading day, on the close of this day.\" (pdf p.159); code BarsSinceEntry=4 / ApplyStop(stopTypeNBar, 4)",
            "E|Last trading day = the next bar is in a new month; the last bar of the data is never a month end. [Reading of 'last trading day'; the TradeStation calendar shortcut in App. 4.1 is not used]",
        ], "Book: S&P 500 1928-2018 647 trades, 61.3% win, 0.57% avg; CNX Nifty 50 from 1996: 165 trades, 60.6%, 0.79% (Table 5.6, AmiBroker/Norgate, $40 S&C on $100k compounding)."),
        d("jt_bom_rotation", "BOM rotation: strongest of 26 indices", "Ch 5.6 · A simple, robust trading system", "Long", NeedsData(BOM_ROT_GAP), &[
            "F|\"PositionScore = 100 + Ref(ROC(C, 252),-1);\" each month only the strongest index over the last 252 days (pdf p.183)",
            "E|BOM entry/exit rules as jt_bom, one position (pdf p.391)",
        ], "Book: 1995-2018, $80 S&C, CAR > 8%, max DD about 15% (Table 5.7)."),
        d("jt_bb_futures", "Bollinger Band breakout, 60 days / 2 SD, MA stop exit", "Appendix 1 · Bollinger Band System", "Long/Short", Live, &[
            "F|\"Inputs: Length(60), Distance(2); HigherBand = Average(Close, Length) + Distance * StdDev(Close, Length); LowerBand = Average(Close, Length) - Distance * StdDev(Close, Length);\" (App. 1.2, pdf p.358)",
            "E|\"Buy next bar at HigherBand stop;\" every bar (pdf p.358)",
            "E|\"Sell next bar at LowerBand stop;\" = sell short (TradeStation 2000i 'Sell' opens a short; text: 'Enter short: next bar at LowerBand stop')",
            "X|\"ExitLong next bar at Average(Close, Length) stop; ExitShort next bar at Average(Close, Length) stop;\" (pdf p.358)",
            "E|After a stop exit no same-direction re-entry on that bar; the opposite entry may fill on it. [Book silent]",
        ], "Book: 7 futures (Nasdaq-MINI, EuroStoxx 50, SMI, Bund, T-Note, Euro, Swiss Franc) 08/1994-09/2005, portfolio 261 trades, 42.15% win, PF 1.96 (Table A1.1, portfolio only). Run on the index. Similar idea already in vault as cov_tfm_bollinger_75 (close entries, 75/2) - different rules, kept."),
        d("jt_bb_stocks", "Bollinger Band breakout for stocks (single stock, 100 / +3 / -1 SD)", "Ch 9.1-9.2 · Bollinger system on single stocks", "Long", Live, &[
            "F|\"Moving average length of Bollinger Bands: 100 days (SMA100); Entry point (top band): 3 standard deviations above SMA100; Exit point (bottom band): 1 standard deviation below SMA100\" (9.2, pdf p.240)",
            "E|\"buy on the open of the day after the stock closed above the top Bollinger Band\" (Fig 9.2); AmiBroker: C > upperBand AND Ref(C < upperBand, -1), BuyPrice = Open of the next day (pdf p.393)",
            "X|\"Exit on the open of the day after the stock closed below this red line\" (Fig 9.2); AmiBroker LongExit = C < BBandBot (pdf p.393)",
            "F|Long only (9.1). One position per instrument (single-stock test, 5% of equity per position in the book).",
        ], "Book: Dow 30 stocks 1950-2018, e.g. Boeing 32 trades, 18 winners, 173 days avg (Table 9.1). Run on NSE basket stocks (basket:<list>:<SYMBOL>) or the indices; current-membership baskets carry survivorship bias. Portfolio version: jt_bb_stocks_portfolio."),
        d("jt_bb_stocks_portfolio", "Bollinger Band stock portfolio (180 / 3.5 / 0.5, SMA300 index filter, 20 positions)", "Ch 9.4-9.7 · Portfolio of S&P 500 stocks", "Long", NeedsData(BB_PORTFOLIO_GAP), &[
            "F|BollTopPrds = 180; BollingerTopWidth = 3.5; BollBottWdth = 0.5; IndexFilterLength = 300; NumberPositions = 20 (App. 5.2, pdf p.393)",
            "E|Buy next open after C > BBandTop(C,180,3.5) AND Ref(C < upperBand,-1) AND point-in-time index member AND NOT index C < MA(C,300) (pdf p.393)",
            "X|Sell next open after C < BBandBot(C,180,0.5), or delisting, or leaving the index (pdf p.393)",
            "F|Rank simultaneous signals by 100 + Ref(ROC(C,300),-1); 0.25% commission per side (pdf p.393)",
        ], "Book: S&P 500 universe 1989-2018 (Tables 9.3-9.4); the EasyLanguage version App. 4.2 differs (BarsBlw 2, 180/3/1, stop orders)."),
        d("jt_triangle", "Triangle System (low volatility / flat MA breakout)", "Appendix 2 · The Triangle System", "Long/Short", NotTestable("code not disclosed: \"we do not disclose this code but just describe its logic\" (App. 2.2); volatility indicator, 'nearly horizontal' MA, entry distance, target and trailing stop are not quantified; 5-minute futures"), &[
            "F|300-bar volatility at its lowest point and 200-bar SMA nearly horizontal (App. 2.2)",
            "E|Long stop a fixed amount above and short stop the same amount below the market (App. 2.2)",
            "X|Target from the 300-bar high-low range, else a trailing stop (App. 2.2)",
        ], "Not testable: thresholds withheld by the authors."),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "jt_example_20d_breakout" => ("jt_breakout", "Jaekle & Tomasini breakouts", "20-day breakout / ATR stop"),
        "jt_luxor_30min" => ("jt_luxor", "LUXOR", "30-minute GBP/USD (book)"),
        "jt_luxor_daily" => ("jt_luxor", "LUXOR", "Daily 7/26 (Appendix 3)"),
        "jt_bb_forex" => ("jt_bollinger", "Jaekle & Tomasini Bollinger systems", "FOREX 6-parameter (ch 5.2)"),
        "jt_bb_futures" => ("jt_bollinger", "Jaekle & Tomasini Bollinger systems", "Futures 60/2 (App. 1)"),
        "jt_bb_stocks" => ("jt_bollinger", "Jaekle & Tomasini Bollinger systems", "Single stock 100/+3/-1 (ch 9.2)"),
        "jt_bb_stocks_portfolio" => ("jt_bollinger", "Jaekle & Tomasini Bollinger systems", "Stock portfolio (ch 9.4)"),
        "jt_bom" => ("jt_bom", "Beginning of Month", "Single index"),
        "jt_bom_rotation" => ("jt_bom", "Beginning of Month", "Rotation over 26 indices"),
        "jt_triangle" => ("jt_triangle", "Triangle System", ""),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(match id {
        "jt_example_20d_breakout" => vec![p("n", "Breakout lookback (days)", 20., 10., 60., 5.), p("atr_n", "AvgTrueRange days", 14., 5., 30., 1.)],
        "jt_luxor_daily" => vec![
            p("fast", "Fast SMA length", 7., 1., 20., 1.), p("slow", "Slow SMA length", 26., 10., 60., 1.),
            p("stop_pts", "Entry stop: points beyond the crossover bar", 1., 0., 10., 1.), p("limit_pts", "Chase limit: points beyond the crossover bar", 5., 1., 50., 1.),
            p("point", "1 point (price units)", 0.01, 0.01, 1., 0.01),
        ],
        "jt_bb_forex" => vec![
            p("len_up", "Upper band length", 18., 5., 60., 1.), p("dev_up", "Upper band SDs", 2., 0.5, 4., 0.25),
            p("len_dn", "Lower band length", 20., 5., 60., 1.), p("dev_dn", "Lower band SDs", 2., 0.5, 4., 0.25),
            p("stop_pct", "Stop loss % from entry", 1.9, 0.5, 5., 0.1), p("target_pct", "Profit target % from entry", 4.8, 1., 10., 0.1),
        ],
        "jt_bom" => vec![p("ma", "Moving average (filter and stop), days", 40., 10., 300., 10.), p("hold", "Exit after N trading days", 4., 1., 20., 1.)],
        "jt_bb_futures" => vec![p("len", "Length (SMA and StdDev)", 60., 20., 200., 5.), p("dist", "Distance (SDs)", 2., 0.5, 4., 0.25)],
        "jt_bb_stocks" => vec![p("len", "SMA / SD length", 100., 20., 300., 10.), p("top", "Entry band: SDs above SMA", 3., 1., 5., 0.25), p("bot", "Exit band: SDs below SMA (negative = above)", 1., -2., 3., 0.25)],
        _ => return None,
    })
}

fn rolling_sd(v: &[f64], ma: &[f64], n: usize) -> Vec<f64> {
    (0..v.len()).map(|i| if i + 1 >= n && ma[i].is_finite() {
        (v[i + 1 - n..=i].iter().map(|x| (x - ma[i]).powi(2)).sum::<f64>() / n as f64).sqrt()
    } else { f64::NAN }).collect()
}

fn trade(dir: f64, i: usize, px: f64, kind: &'static str) -> Trade {
    Trade { dir, legs: vec![Leg { i, px, w: 1. }], exit_i: i, exit_px: px, reason: String::new(), entry_kind: kind, signal_i: i, open: false, exits: vec![] }
}

fn close(mut t: Trade, i: usize, px: f64, sig: usize, why: &str) -> Trade {
    t.exit_i = i;
    t.exit_px = px;
    t.signal_i = sig;
    t.reason = why.into();
    t
}

/// Orders placed on a close for the next bar. Stops: le buy-stop entry, se sell-short-stop entry,
/// lx sell-stop exit of a long, sx buy-stop exit of a short. Targets (limits): lt long, st short.
#[derive(Default)]
struct Orders { le: Option<f64>, se: Option<f64>, lx: Option<f64>, sx: Option<f64>, lt: Option<f64>, st: Option<f64> }

const NAMES: [&str; 3] = ["stop exit", "reversal stop", "profit target"];

/// One-position stop-order simulator (stop-and-reverse capable). `orders(i, pos)` runs on bar i's
/// close with the position held then; the orders apply to bar i+1.
fn stop_sim(x: &Ctx, from: usize, to: usize, start: usize, orders: &dyn Fn(usize, Option<&Trade>) -> Orders, why: [&str; 2]) -> Vec<Trade> {
    let mut out = vec![];
    let mut cur: Option<Trade> = None;
    let mut o = Orders::default();
    for j in from.max(start)..=to {
        let (op, hi, lo) = (x.o[j], x.h[j], x.l[j]);
        if let Some(t) = cur.take() {
            let d = t.dir;
            // stops on the adverse side, in the order price reaches them (nearest first)
            let (stop, rev) = if d > 0. { (o.lx, o.se) } else { (o.sx, o.le) };
            let hit = |lvl: Option<f64>| lvl.filter(|&l| if d > 0. { lo <= l } else { hi >= l });
            let (s, r) = (hit(stop), hit(rev));
            let first = match (s, r) {
                (Some(a), Some(b)) => Some(if d > 0. { if b >= a { (b, 1) } else { (a, 0) } } else if b <= a { (b, 1) } else { (a, 0) }),
                (Some(a), None) => Some((a, 0)),
                (None, Some(b)) => Some((b, 1)),
                _ => None,
            };
            let tgt = if d > 0. { o.lt.filter(|&l| hi >= l) } else { o.st.filter(|&l| lo <= l) };
            let fill = |l: f64| if d > 0. { op.min(l) } else { op.max(l) };
            if let Some((lvl, k)) = first {
                let px = fill(lvl);
                out.push(close(t, j, px, j - 1, if k == 1 { why[1] } else { why[0] }));
                if let Some(b) = r { // opposite entry also reached on this bar
                    let px2 = if d > 0. { op.min(b) } else { op.max(b) };
                    cur = Some(trade(-d, j, px2, "stop"));
                }
            } else if let Some(l) = tgt {
                out.push(close(t, j, if d > 0. { op.max(l) } else { op.min(l) }, j - 1, NAMES[2]));
            } else {
                cur = Some(t);
            }
        } else if j > from.max(start) {
            let up = o.le.filter(|&l| hi >= l);
            let dn = o.se.filter(|&l| lo <= l);
            match (up, dn) {
                (Some(l), None) => cur = Some(trade(1., j, op.max(l), "stop")),
                (None, Some(l)) => cur = Some(trade(-1., j, op.min(l), "stop")),
                _ => {}
            }
        }
        if j < to { o = orders(j, cur.as_ref()); }
    }
    if let Some(t) = cur {
        let mut t = close(t, to, x.c[to], to, "open");
        t.open = true;
        out.push(t);
    }
    out
}

fn crossed_above(a: &[f64], b: &[f64], i: usize) -> bool { i > 0 && a[i] > b[i] && a[i - 1] <= b[i - 1] }
fn crossed_below(a: &[f64], b: &[f64], i: usize) -> bool { i > 0 && a[i] < b[i] && a[i - 1] >= b[i - 1] }

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if from > to || to >= x.r.len() {
        return None;
    }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap().def);
    let u = |k: &str| g(k).round().max(1.) as usize;
    let n = x.c.len();
    Some(match id {
        "jt_example_20d_breakout" => {
            let (k, an) = (u("n"), u("atr_n"));
            let atr = sma(&true_range(x.r), an);
            let hh = |i: usize| x.h[i + 1 - k..=i].iter().copied().fold(f64::NEG_INFINITY, f64::max);
            let ll = |i: usize| x.l[i + 1 - k..=i].iter().copied().fold(f64::INFINITY, f64::min);
            let orders = |i: usize, t: Option<&Trade>| {
                if i + 1 < k { return Orders::default(); }
                let mut o = Orders { le: Some(hh(i)), se: Some(ll(i)), ..Default::default() };
                if atr[i].is_finite() {
                    match t.map(|t| t.dir) {
                        Some(d) if d > 0. => o.lx = Some(x.c[i] - atr[i]),
                        Some(_) => o.sx = Some(x.c[i] + atr[i]),
                        None => {}
                    }
                }
                o
            };
            stop_sim(x, from, to, 0, &orders, ["ATR stop (last close -/+ AvgTrueRange)", "opposite 20-day breakout (reverse)"])
        }
        "jt_luxor_daily" => {
            let (fast, slow) = (sma(&x.c, u("fast")), sma(&x.c, u("slow")));
            let pt = g("point");
            let (sp, lp) = (g("stop_pts") * pt, g("limit_pts") * pt);
            // TradeStation variables keep their value bar to bar: levels set on the crossover bar
            let mut lv = vec![(0f64, 0f64, 0f64, 0f64); n]; // (BuyStop, BuyLimit, SellStop, SellLimit)
            let mut cur = (0., 0., 0., 0.);
            for i in 0..n {
                if crossed_above(&fast, &slow, i) { cur.0 = x.h[i] + sp; cur.1 = x.h[i] + lp; }
                if crossed_below(&fast, &slow, i) { cur.2 = x.l[i] - sp; cur.3 = x.l[i] - lp; }
                lv[i] = cur;
            }
            let orders = |i: usize, t: Option<&Trade>| {
                let mut o = Orders::default();
                let (bs, bl, ss, sl) = lv[i];
                if fast[i] > slow[i] && x.c[i] < bl { o.le = Some(bs); }
                if fast[i] < slow[i] && x.c[i] > sl { o.se = Some(ss); }
                if slow[i].is_finite() {
                    match t.map(|t| t.dir) {
                        Some(d) if d > 0. => o.lx = Some(slow[i] - pt),
                        Some(_) => o.sx = Some(slow[i] + pt),
                        None => {}
                    }
                }
                o
            };
            stop_sim(x, from, to, 0, &orders, ["slow MA stop", "opposite LUXOR entry (reverse)"])
        }
        "jt_bb_forex" => {
            let price: Vec<f64> = (0..n).map(|i| (x.o[i] + x.h[i] + x.l[i] + x.c[i]) / 4.).collect();
            let (lu, ld) = (u("len_up"), u("len_dn"));
            let (mu, md) = (sma(&price, lu), sma(&price, ld));
            let (su, sd) = (rolling_sd(&price, &mu, lu), rolling_sd(&price, &md, ld));
            let ub: Vec<f64> = (0..n).map(|i| mu[i] + g("dev_up") * su[i]).collect();
            let lb: Vec<f64> = (0..n).map(|i| md[i] - g("dev_dn") * sd[i]).collect();
            let (sl, tp) = (g("stop_pct") / 100., g("target_pct") / 100.);
            let orders = |i: usize, t: Option<&Trade>| {
                let mut o = Orders::default();
                if crossed_below(&x.h, &ub, i) { o.se = Some(ub[i]); }
                if crossed_above(&x.l, &lb, i) { o.le = Some(lb[i]); }
                if let Some(t) = t {
                    let e = t.legs[0].px;
                    if t.dir > 0. { o.lx = Some(e * (1. - sl)); o.lt = Some(e * (1. + tp)); } else { o.sx = Some(e * (1. + sl)); o.st = Some(e * (1. - tp)); }
                }
                o
            };
            stop_sim(x, from, to, 0, &orders, ["stop loss %", "opposite band entry (reverse)"])
        }
        "jt_bb_futures" => {
            let k = u("len");
            let ma = sma(&x.c, k);
            let sd = rolling_sd(&x.c, &ma, k);
            let dist = g("dist");
            let orders = |i: usize, t: Option<&Trade>| {
                if !ma[i].is_finite() { return Orders::default(); }
                let mut o = Orders { le: Some(ma[i] + dist * sd[i]), se: Some(ma[i] - dist * sd[i]), ..Default::default() };
                match t.map(|t| t.dir) {
                    Some(d) if d > 0. => o.lx = Some(ma[i]),
                    Some(_) => o.sx = Some(ma[i]),
                    None => {}
                }
                o
            };
            stop_sim(x, from, to, 0, &orders, ["moving average stop", "opposite band (reverse)"])
        }
        "jt_bom" => {
            let ma = sma(&x.c, u("ma"));
            let hold = u("hold");
            let month = |i: usize| &x.r[i].date[0..7]; // YYYY-MM-DD -> YYYY-MM
            let last_of_month = |i: usize| i + 1 < n && month(i + 1) != month(i);
            let entry = |i: usize| (last_of_month(i) && x.c[i] > ma[i]).then_some(Entry::Close);
            let exit = |i: usize, t: &Trade| {
                if x.c[i] < ma[i] { Some(Exit::Close("close below the moving average".into())) }
                else if i - t.legs[0].i >= hold { Some(Exit::Close(format!("time exit: close of day {hold}"))) }
                else { None }
            };
            let add = |_: usize, _: &Trade| false;
            simulate(x, &Spec { dir: 1., weights: &[1.], entry: &entry, add: &add, exit: &exit }, from, to)
        }
        "jt_bb_stocks" => {
            let k = u("len");
            let ma = sma(&x.c, k);
            let sd = rolling_sd(&x.c, &ma, k);
            let top: Vec<f64> = (0..n).map(|i| ma[i] + g("top") * sd[i]).collect();
            let bot: Vec<f64> = (0..n).map(|i| ma[i] - g("bot") * sd[i]).collect();
            let mut out = vec![];
            let mut cur: Option<Trade> = None;
            let mut pend_in = false;
            let mut pend_out: Option<usize> = None;
            for i in from..=to {
                if let Some(s) = pend_out.take() {
                    out.push(close(cur.take().unwrap(), i, x.o[i], s, "close below bottom band · next open"));
                }
                if pend_in {
                    pend_in = false;
                    cur = Some(trade(1., i, x.o[i], "open"));
                }
                if i == to { break; }
                if cur.is_some() {
                    if x.c[i] < bot[i] { pend_out = Some(i); }
                } else if x.c[i] > top[i] && i > 0 && x.c[i - 1] < top[i - 1] {
                    pend_in = true;
                }
            }
            if let Some(t) = cur {
                let mut t = close(t, to, x.c[to], to, "open");
                t.open = true;
                out.push(t);
            }
            out
        }
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn rows(dates: &[&str], c: &[f64]) -> Vec<Row> {
        dates.iter().zip(c).map(|(d, &c)| Row { ticker: "T".into(), date: d.to_string(), open: c, high: c + 1., low: c - 1., close: c, volume: None }).collect()
    }
    #[test]
    fn bom_buys_last_day_of_month_and_exits_after_four_days() {
        // 45 rising days in March, then April: last March bar is a month end above the rising 40-day MA
        let mut d: Vec<String> = (1..=45).map(|i| format!("2020-03-{:02}", (i % 28) + 1)).collect();
        d.extend((1..=10).map(|i| format!("2020-04-{:02}", i)));
        let ds: Vec<&str> = d.iter().map(String::as_str).collect();
        let c: Vec<f64> = (0..55).map(|i| 100. + i as f64).collect();
        let r = rows(&ds, &c);
        let x = Ctx::new(&r, &r, &[]);
        let ts = run("jt_bom", &x, 0, 54, &Params::new()).unwrap();
        assert_eq!(ts.len(), 1);
        assert_eq!(ts[0].legs[0].i, 44);
        assert_eq!(ts[0].exit_i, 48);
        assert_eq!(ts[0].exit_px, c[48]);
    }
    #[test]
    fn stop_entry_gap_fills_at_open() {
        let mut c: Vec<f64> = vec![100.; 30];
        c[25] = 110.; // breakout bar opens above the 20-day high of 101
        let d: Vec<String> = (0..30).map(|i| format!("2020-01-{:02}", i)).collect();
        let ds: Vec<&str> = d.iter().map(String::as_str).collect();
        let r = rows(&ds, &c);
        let x = Ctx::new(&r, &r, &[]);
        let ts = run("jt_example_20d_breakout", &x, 0, 29, &Params::new()).unwrap();
        assert_eq!(ts[0].legs[0].i, 25);
        assert_eq!(ts[0].legs[0].px, 110.);
        for t in &ts { assert!(t.exit_px >= x.l[t.exit_i] - 1e-9 && t.exit_px <= x.h[t.exit_i] + 1e-9); }
    }
}
