//! Martin J. Pring, "Trading Systems Explained: How to Build Reliable Technical Systems" (pring.com /
//! McGraw-Hill, 2003 printing; scan, 235 pdf pages). Book id `mpts`. Source and checks: library/mpts/.
//! Live: the MA crossover systems with complete entry AND exit rules (Ch.11-12, 15, 20), the MA + price
//! oscillator system (Ch.14) and the MA + CMO partial-profit system (Ch.13, neutral-trend version).
//! Intermarket systems (Ch.17-19) need commercial paper yields, CRB, gold and bond series -> NeedsData.
//! Fills: the signal bar's close (MetaStock System Tester, which the book used, p.1; "Close MONTHLY" tables).
//! Weekly / monthly systems read the week-end / month-end daily close (book: weekly / monthly closes).
use crate::connors::{sma, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "mpts";

const FILL: &str = "Fill at the close of the signal bar (MetaStock System Tester default; the book's tables are 'Close MONTHLY' / weekly closes). No commission or cash interest in the vault default (book: none, except where a table says 1% / 4%).";
const CP: &str = "3-month commercial paper yield (monthly) is not in the vault; the S&P side is available (SPX from 1983)";
const CRB: &str = "CRB Composite and spot gold (monthly / weekly) are not in the vault";
const BONDS: &str = "long-term US government bond prices and the CRB Spot Raw Industrials index are not in the vault";

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    let d = |id, name, group, dir, status, needs_series: &'static [&'static str], rules, notes, claim| StrategyDef {
        id, name, group, session: 0, dir, status, needs_volume: false, needs_vix: false, needs_series, book: BOOK, rules, notes, claim,
    };
    let rs_note = "Book instruments: DJ All Banks index vs DJ Composite (10/26, 1983-1998, Charts 20-2..20-5); funds vs S&P Composite. Vault: RS line = ticker close / STR_DATA2 close (NIFTYBANK/NIFTYIT/NIFTYMIDCAP100 vs NIFTY 50; NDX/DJI/MDY/IWM/QQQ/SMH vs SPX). NIFTY 50 and SPX have no parent index -> data gap. Weekly bars = last trading day of each week. Long only (Appendix p.200: 'This one only trades from the long side'); the book's 4% cash interest variant (Charts 20-5 ff.) is not modelled.";
    vec![
        // ---- Ch.11 The (Almost) Perfect System
        d("mpts_ma2d_long", "2-day MA crossover, long only (Nikkei example)", "Ch.11 · The (Almost) Perfect System", "Long", Live, &[],
          &["E|\"Chart 11-1 shows the Nikkei and the buy and sell signals that resulted from a 2-day MA crossover with no margin\" (p.97): buy on the close when the close crosses above its 2-day simple MA",
            "X|Sell on the close when the close crosses below the 2-day MA; \"222 signals ... were triggered just based on going long\" (p.97)"],
          FILL, crate::connors::claim("222", "80 / 142 (36%)", "net +55.2% (avg win 2.30x loss)", "-", "Nikkei 1994-1998 (Table 11-1)")),
        // ---- Ch.12 Optimizing for the U.S. equity market
        d("mpts_ma10m_ls", "10-month MA crossover, long and short (S&P Composite 1900-1998)", "Ch.12 · Optimizing for the U.S. Equity Market", "Long/Short", Live, &[],
          &["E|\"buy signals were generated as the S&P Composite crossed above its 2-month MA, and sell signals when it crossed below it. Short signals were generated when the Index slipped below its average and covered when it rallied back above it again. Thus, there was always a position, either long or short\" (p.107)",
            "F|\"the result is shown in Chart 12-3, of a 10-month average crossover\" (p.110): monthly closes vs a 10-month simple MA (Tables 12-4/12-5 'S&P Composite Close MONTHLY')",
            "X|Reverse on the opposite crossover (always in the market)"],
          "Monthly bar = last trading day of the month; MA = simple average of the last n month-end closes. Book data 1900-1998 (pre-1926 Cowles); vault SPX from 1983, DJI from 1970, NIFTY 50 from 1990s. Fill: the signal bar's close (MetaStock default); no costs or cash interest by default.",
          crate::connors::claim("124", "57 / 67 (46%)", "net +764,252% ($1 -> $7,643)", "-", "S&P Composite 1900-1998, no costs (Table 12-4)")),
        d("mpts_ma10m_long", "10-month MA crossover, long only", "Ch.12 · Optimizing for the U.S. Equity Market", "Long", Live, &[],
          &["E|\"OPTIMIZE FOR JUST LONG POSITIONS ... only taking long positions and investing the cash during periods when the price is below the moving average\" (p.113): buy on the month-end close above the 10-month MA",
            "X|Sell when the month-end close crosses below the 10-month MA",
            "F|Book variant assumed 1% round-trip commission and 4% annual interest on cash (Tables 12-6/12-7); the vault default has no costs/interest (set the global cost to compare)"],
          "Monthly bars as mpts_ma10m_ls. Long-only variant of the same crossover. Fill: the signal bar's close (MetaStock default); no costs or cash interest by default.",
          crate::connors::claim("62", "33 / 29 (53%)", "net +379,169% (1% comm, 4% cash)", "avg win 20.15 months, loss 4.10", "S&P Composite 1900-1998 (Tables 12-6/12-7)")),
        // ---- Ch.13 MA + oscillator (neutral trend)
        d("mpts_ma_cmo", "25-day MA (advanced 5 days) crossover with 20-day CMO partial profits", "Ch.13 · Combining a Moving Average and an Oscillator", "Long/Short", Live, &[],
          &["F|\"Chart 13-3 just features the 20-day CMO together with a 25-day simple moving average advanced by 5 days\" (p.120)",
            "E|Buy when the price crosses above the (advanced) MA; \"As the final long position was liquidated, at point C, a short position was initiated\" (p.121): short on the cross below",
            "X|Partial profit: \"Partial profits would have been taken in August (B) because that is when the CMO crosses back below its overbought zone\" (p.120), +50 zone (p.121); \"I use the re-crossing of the overbought zones as my signaling point\" (p.120); size: \"to cover half the position\" (p.123). Shorts: CMO crosses back above -50",
            "X|Rest of the position: closed (and reversed) on the opposite MA crossover (p.121-122)"],
          "Neutral-trend version only (Chart 13-3, International Paper). The bull/bear versions (Charts 13-4/13-5) trade only with a 'confirmed' primary trend, which is judgement -> not coded. CMO = Chande Momentum Oscillator 100*(sum up - sum down)/(sum up + sum down) of the last 20 close changes (the book points to pring.com for the formula, p.120). 'Advanced by 5 days' = the MA plotted 5 bars forward, i.e. today's line = SMA25 of 5 bars ago. One partial exit per trade. Fill: the signal bar's close (MetaStock default); no costs or cash interest by default.",
          None),
        // ---- Ch.14 MA + price oscillator
        d("mpts_posc_28_5", "28-day MA crossover with +/-5% price-oscillator profit taking (S&P)", "Ch.14 · Testing a Moving Average and an Oscillator", "Long/Short", Live, &[],
          &["F|\"A price oscillator is calculated by dividing a short-term moving average by a longer term one. In this case, I used a one period moving average; that is, the close as the shorter average\" (p.125); percentage method: +2.5% = \"the price being 2 1/2 percent above ... the 10-day moving average\" (p.125)",
            "E|\"Buy when the price crosses above a moving average\" (p.125); \"the price crosses below the average and this initiates a short signal\" (p.125)",
            "X|\"Then sell either when it crosses below the average, or the price oscillator reaches a designated overbought level\" (p.125); shorts covered at the oversold level or the MA crossover (\"covered ... fairly close to the actual low as the oscillator touches its oversold zone\", p.127)",
            "F|S&P Composite: \"the system using the 28/5/-5 combination ... turning $1 into $1.84\" (p.141, Chart 14-18, 1979-1998)"],
          "'Reaches' = close/SMA - 1 at or beyond the level on a bar's close; after an oscillator exit the system waits for the next MA crossover (Chart 14-2 D-E). Entry and an oscillator exit are not taken on the same bar (oscillator checked from the bar after entry). Other book combos as params: 28/4/-4 (S&P best), 28/2/-2 (T-bonds), 42/6/-6, 20/10/-10. Fill: the signal bar's close (MetaStock default); no costs or cash interest by default.",
          crate::connors::claim("-", "-", "$1 -> $1.84", "-", "S&P Composite 1979-1998 (Chart 14-18)")),
        d("mpts_posc_10_10", "10-day MA crossover with +/-10% price-oscillator profit taking (NASDAQ / Russell 2000)", "Ch.14 · Testing a Moving Average and an Oscillator", "Long/Short", Live, &[],
          &["F|Same rules as mpts_posc_28_5 with a 10-day MA and +10% / -10% levels: \"The best combination was a 10/10/-10\" (Russell 2000, p.139); NASDAQ: \"One of the best combinations was 10/10/-10\" (p.142)",
            "E|Buy on the close crossing above the 10-day SMA; short on the cross below (p.125)",
            "X|Exit longs at the cross below or when close/SMA10 - 1 >= +10%; cover shorts at the cross above or at <= -10% (p.125)"],
          "Book: NASDAQ Composite 1980-1998 (vault NDX from 1990 is a different index); Russell 2000 1987-1998 (vault IWM ETF from 2000). Same oscillator conventions as mpts_posc_28_5. Fill: the signal bar's close (MetaStock default); no costs or cash interest by default.",
          crate::connors::claim("556 (NASDAQ) / 333 (Russell)", "228/328 (41%) / 145/188 (44%)", "+30,000% / +2,687%", "win/loss 3.19 / 8.33 (Table 14-6)", "NASDAQ 1980-98 / Russell 2000 1987-98 (p.139-142)")),
        // ---- Ch.15 Triple indicator system
        d("mpts_triple", "Triple Indicator System (10-week MA, 6- and 13-week ROC)", "Ch.15 · The Triple Indicator System", "Long/Short", Live, &[],
          &["F|\"These are 10-week simple MAs, a 6-week ROC and a 13-week ROC\" (p.145)",
            "E|\"Buy when the price is above the 10-week MA and both ROCs are above zero\" (p.145)",
            "X|\"Sell when all three go negative, that is, the ROCs cross below zero and the price crosses its moving average. Signals cannot be generated unless all three agree\" (p.145); traded \"from both the long and short side\" (p.146, Chart 15-9 'long and short'): reverse to short when all three are negative, back to long when all three are positive"],
          "Weekly bars = last trading day of each week. ROC above zero = close > close n weeks ago. Position held until the opposite three-way agreement (p.148: 'all three indicators were never in agreement' -> no signal). The 300-week MA of the equity line (p.150) is an optional abandonment filter ('could'), not coded. Book markets: British pound, Nikkei, S&P Composite (Chart 15-9, 1977-1998: equity ~$1 -> ~$2), gold, AAA yield, DM. Fill: the signal bar's close (MetaStock default); no costs or cash interest by default.",
          crate::connors::claim("-", "-", "$1 -> about $2 (chart)", "-", "S&P Composite 1977-1998 (Chart 15-9)")),
        d("mpts_ma10w", "10-week MA crossover (Triple Indicator component 1)", "Ch.15 · The Triple Indicator System", "Long/Short", Live, &[],
          &["E|\"starting off with a simple 10-week MA crossover ... Buy signals are once again indicated by the upward pointing arrows and short positions by the downward pointing ones\" (p.145-146): long on the weekly close crossing above the 10-week SMA",
            "X|Reverse to short on the cross below (\"from both the long and short side\", p.146)"],
          "British pound mid-1974 to 1976: 13 signals, +19 cents on $1 (Chart 15-1). Weekly bars as mpts_triple. Fill: the signal bar's close (MetaStock default); no costs or cash interest by default.", None),
        d("mpts_roc13w", "13-week ROC zero crossover (Triple Indicator component 2)", "Ch.15 · The Triple Indicator System", "Long/Short", Live, &[],
          &["E|\"Buy and sell signals are triggered when the 13-week ROC crosses above and below its zero reference line\" (p.146)", "X|Reverse on the opposite zero crossing"],
          "British pound 1974-76: +23 cents, six signals (Chart 15-2). Fill: the signal bar's close (MetaStock default); no costs or cash interest by default.", None),
        d("mpts_roc6w", "6-week ROC zero crossover (Triple Indicator component 3)", "Ch.15 · The Triple Indicator System", "Long/Short", Live, &[],
          &["E|\"A 6-week ROC was chosen ... The result, as shown in Chart 15-3, was an improved 35 cents\" (p.146): signals on the 6-week ROC zero crossings", "X|Reverse on the opposite zero crossing"],
          "British pound 1974-76: +35 cents (Chart 15-3). Fill: the signal bar's close (MetaStock default); no costs or cash interest by default.", None),
        // ---- Ch.17-19 intermarket
        d("mpts_sp_cp_yield", "S&P Composite vs 3-month commercial paper yield (12/12, 2/33)", "Ch.17 · Equities Versus Short-Term Interest Rates", "Long", NeedsData(CP), &[],
          &["E|Bullish when the inverted 3-month commercial paper yield is above its 12-month MA and the S&P Composite is above its 12-month MA (p.164)",
            "X|\"It remains positive until either series moves back below its average\" (p.164); variant 2/33: buy on the 2-month MAs, sell when either crosses below its 33-month MA (p.168)"],
          "Book (Table 17-1, 1960-1998?, monthly): 12/12 25 trades, 17 wins / 8 losses; 2/33 (Table 17-2): 29 trades 22/7.", None),
        d("mpts_crb_gold", "CRB Composite vs spot gold (8/24 monthly, 26/45/65 weekly)", "Ch.18 · CRB Versus Gold", "Long/Short", NeedsData(CRB), &[],
          &["E|\"buy when both gold is above its 24-month average and the CRB is above its 8-month MA and sell when either falls below its respective MA\" (p.174); 4-week MA of Friday closes as the monthly value (p.172-174)",
            "E|Shorts when both are below their averages, covered when one turns positive (p.177); weekly 26/45/65 version (p.179-180)"],
          "Table 18-1: 8/24 17 trades 12/5; Table 18-2 weekly 23/43/65 etc.", None),
        d("mpts_bonds_crb", "Bonds vs commodities (9-month bond MA / 12-month CRB RM MA)", "Ch.19 · Bonds Versus Commodities", "Long/Short", NeedsData(BONDS), &[],
          &["E|\"The system is bullish when the trend of bond prices is up and commodities are down ... a 9-month MA crossover for bonds and a 12-month moving average for commodities. Sell signals are triggered when either series goes negative. Short signals develop when both are negative and covered when either goes positive\" (p.182-184)"],
          "Table 19-1: 9/12 59 trades 30/29 (1940s-1998).", None),
        // ---- Ch.20 relative strength
        d("mpts_rs_10_26", "RS system: 10/26-week MA crossover of price and RS line", "Ch.20 · Systems Using Relative Strength", "Long", Live, &["STR_DATA2"],
          &["F|\"A relative strength line is calculated by dividing a stock or an industry group by a market average such as the S&P Composite\" (p.188)",
            "E|\"Buy signals are generated when the 10-week moving average of the Banks Index crosses above the 26-week simple MA. This is confirmed by a similar signal by the RS line\" (p.190)",
            "X|\"Sell signals are triggered when either of the 26-week averages is recrossed by their 10-week counterparts\" (p.190)"],
          rs_note, None),
        d("mpts_rs_3_10", "More sensitive RS system: 3/10-week MA crossover", "Ch.20 · Systems Using Relative Strength", "Long", Live, &["STR_DATA2"],
          &["E|\"Buy signals are generated when the 3-week moving average of the price and RS crosses above their respective 10-week MAs\" (p.194)",
            "X|\"sell signals occur when either cross below their 10-week MA\" (p.194)"],
          rs_note, None),
        // ---- not testable
        d("mpts_not_testable", "Ch.6-10, 16 illustrations, Ch.13 trend versions, equity-curve filter", "Part I / Ch.13 / Ch.16", "Long/Short",
          NotTestable("Ch.6 stops (no system); Ch.7-10 single-stock / fund MA examples (Merrill Lynch 25/13-day, McDonald's 40-day, Barrick 7-day, CRB 54-day, 21-day RSI 50-cross on GM) do not state long-only vs long/short on index data and are optimisation illustrations; DJIA 45-week MA crossover (p.59, Chart 7-6) gives no direction or result numbers; Ch.13 bull/bear versions need a 'confirmed' primary trend (judgement); 300-week equity-line MA (p.150) and leverage (Ch.16) are money-management suggestions, not entry/exit rules"), &[],
          &["E|See notes"], "Listed so the pipeline knows these pages were read.", None),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "mpts_ma2d_long" => ("mpts_ma", "Pring MA crossover", "2-day, long only"),
        "mpts_ma10m_ls" => ("mpts_ma", "Pring MA crossover", "10-month, long/short"),
        "mpts_ma10m_long" => ("mpts_ma", "Pring MA crossover", "10-month, long only"),
        "mpts_ma_cmo" => ("mpts_osc", "Pring MA + oscillator", "25-day MA + CMO 20"),
        "mpts_posc_28_5" => ("mpts_osc", "Pring MA + oscillator", "28/5/-5 price oscillator"),
        "mpts_posc_10_10" => ("mpts_osc", "Pring MA + oscillator", "10/10/-10 price oscillator"),
        "mpts_triple" => ("mpts_triple", "Pring Triple Indicator", "MA10w + ROC6w + ROC13w"),
        "mpts_ma10w" => ("mpts_triple", "Pring Triple Indicator", "10-week MA only"),
        "mpts_roc13w" => ("mpts_triple", "Pring Triple Indicator", "13-week ROC only"),
        "mpts_roc6w" => ("mpts_triple", "Pring Triple Indicator", "6-week ROC only"),
        "mpts_sp_cp_yield" => ("mpts_inter", "Pring intermarket", "S&P vs CP yield · data missing"),
        "mpts_crb_gold" => ("mpts_inter", "Pring intermarket", "CRB vs gold · data missing"),
        "mpts_bonds_crb" => ("mpts_inter", "Pring intermarket", "Bonds vs commodities · data missing"),
        "mpts_rs_10_26" => ("mpts_rs", "Pring relative strength", "10/26 weeks"),
        "mpts_rs_3_10" => ("mpts_rs", "Pring relative strength", "3/10 weeks"),
        "mpts_not_testable" => ("mpts_nt", "Pring illustrations (not testable)", "-"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(match id {
        "mpts_ma2d_long" => vec![p("n", "MA length (days)", 2., 2., 65., 1.)],
        "mpts_ma10m_ls" | "mpts_ma10m_long" => vec![p("n", "MA length (months)", 10., 2., 36., 1.)],
        "mpts_ma_cmo" => vec![p("ma", "MA length (days)", 25., 5., 60., 1.), p("adv", "MA advanced by (days)", 5., 0., 10., 1.),
                               p("cmo", "CMO length (days)", 20., 5., 40., 1.), p("ob", "CMO overbought (oversold = -)", 50., 20., 80., 5.),
                               p("part", "Fraction taken at the CMO signal", 0.5, 0.0, 1.0, 0.25)],
        "mpts_posc_28_5" => vec![p("n", "MA length (days)", 28., 2., 65., 1.), p("ob", "Overbought % above MA", 5., 1., 20., 1.), p("os", "Oversold % (negative)", -5., -20., -1., 1.)],
        "mpts_posc_10_10" => vec![p("n", "MA length (days)", 10., 2., 65., 1.), p("ob", "Overbought % above MA", 10., 1., 20., 1.), p("os", "Oversold % (negative)", -10., -20., -1., 1.)],
        "mpts_triple" => vec![p("ma", "MA length (weeks)", 10., 4., 40., 1.), p("roc_s", "Short ROC (weeks)", 6., 2., 26., 1.), p("roc_l", "Long ROC (weeks)", 13., 4., 52., 1.)],
        "mpts_ma10w" => vec![p("n", "MA length (weeks)", 10., 2., 52., 1.)],
        "mpts_roc13w" => vec![p("n", "ROC length (weeks)", 13., 2., 52., 1.)],
        "mpts_roc6w" => vec![p("n", "ROC length (weeks)", 6., 2., 52., 1.)],
        "mpts_rs_10_26" => vec![p("fast", "Fast MA (weeks)", 10., 2., 26., 1.), p("slow", "Slow MA (weeks)", 26., 5., 52., 1.)],
        "mpts_rs_3_10" => vec![p("fast", "Fast MA (weeks)", 3., 2., 26., 1.), p("slow", "Slow MA (weeks)", 10., 5., 52., 1.)],
        _ => return None,
    })
}

// ---------------------------------------------------------------- calendar

fn day_number(date: &str) -> i64 {
    let (y, m, d): (i64, i64, i64) = (date[0..4].parse().unwrap(), date[5..7].parse().unwrap(), date[8..10].parse().unwrap());
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (m + if m > 2 { -3 } else { 9 }) + 2) / 5 + d - 1;
    era * 146_097 + (yoe * 365 + yoe / 4 - yoe / 100 + doy) - 719_468
}

/// Period-end bars: the next bar is in another week (Monday-based) / month. The last bar of the data is
/// a week end only if it is a Friday, never a month end (the period may be incomplete).
fn period_ends(x: &Ctx, monthly: bool) -> Vec<bool> {
    let n = x.r.len();
    (0..n).map(|i| match x.r.get(i + 1) {
        Some(nx) if monthly => nx.date[..7] != x.r[i].date[..7],
        Some(nx) => (day_number(&nx.date) + 3).div_euclid(7) != (day_number(&x.r[i].date) + 3).div_euclid(7),
        None => !monthly && (day_number(&x.r[i].date) + 3).rem_euclid(7) == 4,
    }).collect()
}

/// Indicator on the period-end closes, written back on period-end bars (NaN elsewhere).
fn on_periods(ends: &[bool], v: &[f64], f: impl Fn(&[f64]) -> Vec<f64>) -> Vec<f64> {
    let idx: Vec<usize> = (0..ends.len()).filter(|&i| ends[i]).collect();
    let pv: Vec<f64> = idx.iter().map(|&i| v[i]).collect();
    let r = f(&pv);
    let mut out = vec![f64::NAN; ends.len()];
    for (k, &i) in idx.iter().enumerate() { out[i] = r[k]; }
    out
}

fn roc_pos(v: &[f64], n: usize) -> Vec<f64> {
    (0..v.len()).map(|i| if i >= n { v[i] / v[i - n] - 1.0 } else { f64::NAN }).collect()
}

fn sign(a: f64, b: f64) -> Option<i8> {
    if !(a.is_finite() && b.is_finite()) { None } else if a > b { Some(1) } else if a < b { Some(-1) } else { None }
}

/// Chande Momentum Oscillator over the last n close changes.
pub fn cmo(c: &[f64], n: usize) -> Vec<f64> {
    (0..c.len()).map(|i| {
        if i < n { return f64::NAN; }
        let (mut up, mut dn) = (0.0, 0.0);
        for k in i + 1 - n..=i { let d = c[k] - c[k - 1]; if d > 0. { up += d } else { dn -= d } }
        if up + dn > 0. { 100. * (up - dn) / (up + dn) } else { 0. }
    }).collect()
}

// ---------------------------------------------------------------- simulator

fn open_trade(dir: f64, i: usize, px: f64) -> Trade {
    Trade { dir, legs: vec![Leg { i, px, w: 1. }], exit_i: i, exit_px: px, reason: String::new(), entry_kind: "close", signal_i: i, open: false, exits: vec![] }
}

fn close_trade(mut t: Trade, i: usize, px: f64, why: &str) -> Trade {
    if !t.exits.is_empty() {
        let done: f64 = t.exits.iter().map(|e| e.2).sum();
        t.exits.push((i, px, 1. - done));
        t.exit_px = t.exits.iter().map(|e| e.1 * e.2).sum();
    } else { t.exit_px = px; }
    t.exit_i = i; t.signal_i = i; t.reason = why.into();
    t
}

/// State machine, all fills at the signal bar's close.
/// `state[i]`: Some(+1/-1) = the rule's desired side on bar i, None = no information (hold).
/// A signal is a CHANGE of the desired side versus the last known one (history before `from` counts,
/// trading starts at `from`). Long-only: -1 means flat. `osc` = optional profit-taking oscillator
/// (value, overbought, oversold): a long is closed when osc >= ob, a short when osc <= os.
fn run_states(x: &Ctx, from: usize, to: usize, state: &[Option<i8>], long_only: bool, osc: Option<(&[f64], f64, f64)>, why: [&str; 2]) -> Vec<Trade> {
    let mut out = vec![];
    let mut cur: Option<Trade> = None;
    let mut last: Option<i8> = None;
    for i in 0..=to {
        let s = state[i];
        let signal = match (s, last) { (Some(a), Some(b)) if a != b => Some(a), _ => None };
        if s.is_some() { last = s; }
        if i < from { continue; }
        let c = x.c[i];
        if let Some(sd) = signal {
            if let Some(t) = cur.take() { out.push(close_trade(t, i, c, why[1])); }
            if sd > 0 || !long_only { cur = Some(open_trade(sd as f64, i, c)); }
            continue;
        }
        if let (Some(t), Some((o, ob, os))) = (cur.as_ref(), osc) {
            let hit = if t.dir > 0. { o[i] >= ob } else { o[i] <= os };
            if hit && i > t.legs[0].i {
                let t = cur.take().unwrap();
                let long = t.dir > 0.;
                out.push(close_trade(t, i, c, if long { "oscillator reached overbought" } else { "oscillator reached oversold" }));
            }
        }
    }
    if let Some(mut t) = cur { t = close_trade(t, to, x.c[to], "open"); t.open = true; out.push(t); }
    let _ = why[0];
    out
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if from > to || to >= x.r.len() { return None; }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap().def);
    let u = |k: &str| g(k).round().max(1.) as usize;
    let n = x.c.len();
    let cross = |ma: &[f64]| -> Vec<Option<i8>> { (0..n).map(|i| sign(x.c[i], ma[i])).collect() };
    let why = ["", "opposite signal"];
    Some(match id {
        "mpts_ma2d_long" => run_states(x, from, to, &cross(&sma(&x.c, u("n"))), true, None, why),
        "mpts_posc_28_5" | "mpts_posc_10_10" => {
            let ma = sma(&x.c, u("n"));
            let o: Vec<f64> = (0..n).map(|i| 100. * (x.c[i] / ma[i] - 1.)).collect();
            run_states(x, from, to, &cross(&ma), false, Some((&o, g("ob"), g("os"))), why)
        }
        "mpts_ma10m_ls" | "mpts_ma10m_long" | "mpts_ma10w" => {
            let monthly = id != "mpts_ma10w";
            let ends = period_ends(x, monthly);
            let ma = on_periods(&ends, &x.c, |v| sma(v, u("n")));
            run_states(x, from, to, &cross(&ma), id == "mpts_ma10m_long", None, why)
        }
        "mpts_roc13w" | "mpts_roc6w" => {
            let ends = period_ends(x, false);
            let r = on_periods(&ends, &x.c, |v| roc_pos(v, u("n")));
            let st: Vec<Option<i8>> = r.iter().map(|&v| sign(v, 0.)).collect();
            run_states(x, from, to, &st, false, None, why)
        }
        "mpts_triple" => {
            let ends = period_ends(x, false);
            let ma = on_periods(&ends, &x.c, |v| sma(v, u("ma")));
            let rs = on_periods(&ends, &x.c, |v| roc_pos(v, u("roc_s")));
            let rl = on_periods(&ends, &x.c, |v| roc_pos(v, u("roc_l")));
            let st: Vec<Option<i8>> = (0..n).map(|i| {
                let (a, b, c) = (sign(x.c[i], ma[i]), sign(rs[i], 0.), sign(rl[i], 0.));
                match (a, b, c) { (Some(a), Some(b), Some(c)) if a == b && b == c => Some(a), _ => None }
            }).collect();
            run_states(x, from, to, &st, false, None, ["", "all three indicators reversed"])
        }
        "mpts_rs_10_26" | "mpts_rs_3_10" => {
            let Some(b) = x.series("STR_DATA2") else { return Some(vec![]) };
            let ends = period_ends(x, false);
            let ends: Vec<bool> = (0..n).map(|i| ends[i] && b[i].is_finite() && b[i] > 0.).collect();
            let rsl: Vec<f64> = (0..n).map(|i| x.c[i] / b[i]).collect();
            let (f, s) = (u("fast"), u("slow"));
            let (pf, ps) = (on_periods(&ends, &x.c, |v| sma(v, f)), on_periods(&ends, &x.c, |v| sma(v, s)));
            let (rf, rsl2) = (on_periods(&ends, &rsl, |v| sma(v, f)), on_periods(&ends, &rsl, |v| sma(v, s)));
            let st: Vec<Option<i8>> = (0..n).map(|i| match (sign(pf[i], ps[i]), sign(rf[i], rsl2[i])) {
                (Some(1), Some(1)) => Some(1),
                (Some(-1), _) | (_, Some(-1)) => Some(-1),
                _ => None,
            }).collect();
            run_states(x, from, to, &st, true, None, ["", "a 10-week (fast) average recrossed its slow average"])
        }
        "mpts_ma_cmo" => ma_cmo(x, from, to, u("ma"), g("adv").round().max(0.) as usize, u("cmo"), g("ob"), g("part").clamp(0., 1.)),
        _ => return None,
    })
}

/// Ch.13 neutral-trend system: reversal on crosses of the advanced SMA, one partial exit on the CMO re-cross.
fn ma_cmo(x: &Ctx, from: usize, to: usize, man: usize, adv: usize, cn: usize, ob: f64, part: f64) -> Vec<Trade> {
    let n = x.c.len();
    let base = sma(&x.c, man);
    let ma: Vec<f64> = (0..n).map(|i| if i >= adv { base[i - adv] } else { f64::NAN }).collect();
    let cm = cmo(&x.c, cn);
    let mut out = vec![];
    let mut cur: Option<Trade> = None;
    let mut last: Option<i8> = None;
    for i in 0..=to {
        let s = sign(x.c[i], ma[i]);
        let signal = match (s, last) { (Some(a), Some(b)) if a != b => Some(a), _ => None };
        if s.is_some() { last = s; }
        if i < from { continue; }
        let c = x.c[i];
        if let Some(sd) = signal {
            if let Some(t) = cur.take() { out.push(close_trade(t, i, c, "opposite MA crossover")); }
            cur = Some(open_trade(sd as f64, i, c));
            continue;
        }
        if let Some(t) = cur.as_mut() {
            if t.exits.is_empty() && part > 0. && i > t.legs[0].i && i > 0 {
                let hit = if t.dir > 0. { cm[i - 1] >= ob && cm[i] < ob } else { cm[i - 1] <= -ob && cm[i] > -ob };
                if hit {
                    if part >= 1. { let t = cur.take().unwrap(); out.push(close_trade(t, i, c, "CMO re-crossed its zone (full)")); }
                    else { t.exits.push((i, c, part)); }
                }
            }
        }
    }
    if let Some(mut t) = cur { t = close_trade(t, to, x.c[to], "open"); t.open = true; out.push(t); }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cmo_by_hand() {
        let c = [10., 11., 10.5, 12., 12.];
        let v = cmo(&c, 3); // changes +1, -0.5, +1.5 -> (2.5-0.5)/(3)=66.67 ; next: -0.5,+1.5,0 -> (1.5-.5)/2=50
        assert!(v[2].is_nan());
        assert!((v[3] - 200. / 3.).abs() < 1e-9 && (v[4] - 50.).abs() < 1e-9);
    }
    #[test]
    fn sign_rules() {
        assert_eq!(sign(2., 1.), Some(1));
        assert_eq!(sign(1., 1.), None);
        assert_eq!(sign(f64::NAN, 1.), None);
    }
}
