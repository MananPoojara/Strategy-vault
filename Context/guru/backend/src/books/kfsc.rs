//! Perry J. Kaufman, *A Short Course in Technical Trading* (Wiley 2003, 339 PDF pages, text layer).
//! Page refs: printed page (PDF page = printed + 13). Catalogue Live is not source admission.
//! Only the exact configurations named below have a bounded stock research contract;
//! source choices and execution conventions are separate from book-reported results.
use crate::connors::{sma, true_range, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "kfsc";

/// Exact bounded research configurations; other catalogue entries inherit no review.
pub const REVIEWED_STOCK_IDS: &[&str] = &["kfsc_two_ma","kfsc_pct_band","kfsc_bollinger"];
pub fn reviewed_stock_rule(id: &str) -> bool {REVIEWED_STOCK_IDS.contains(&id)}
/// Additional isolated research candidates. Public deployment still requires
/// independent source/ledger review and a separate normal integration release.
pub const MOMENTUM_STOCK_IDS: &[&str] = &["kfsc_macd","kfsc_dsm"];
pub fn reviewed_momentum_rule(id:&str)->bool {MOMENTUM_STOCK_IDS.contains(&id)}
pub fn validate_momentum(id:&str,p:&Params)->Option<&'static str> {
    if !reviewed_momentum_rule(id) {return None;}
    let schema=params(id)?;
    if p.iter().any(|(key,value)|!value.is_finite() || !schema.iter().any(|s|s.key==key && s.def==*value)) {
        return Some("This bounded Short Course momentum application covers only the exact literal printed MACD constants .0476/.0243/.10 or 10-day momentum double-smoothed with .0476, using the declared full-history seed, turn/crossing and idealized same-close conventions. Changed, unknown, nonfinite or legacy reciprocal settings require separate source and ledger review.");
    }
    None
}
pub fn validate_reviewed(id: &str, p: &Params) -> Option<&'static str> {
    if !reviewed_stock_rule(id) {return None;}
    let schema=params(id)?;
    if p.iter().any(|(key,value)| !value.is_finite() || !schema.iter().any(|s|s.key==key && s.def==*value)) {
        return Some("This bounded Short Course review covers only the exact selected 80/10 SMA, 20/5 divided percentage band or 20/2 literal price-level sample-SD formula, with the fixed declared idealized completed-close execution convention. Changed, unknown or nonfinite settings need separate source and ledger review; they cannot inherit this configuration's review.");
    }
    None
}

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, dir, status, rules, notes| StrategyDef {
        id, name, group, session: 0, dir, status, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None,
    };
    use Status::*;
    vec![
        // ------------------------------------------------ Ch 6 Channels and bands
        d("kfsc_pct_band", "Percentage band countertrend (20-day MA, 5%)", "Ch 6 · Channels and bands", "Long/Short", Live, &[
            "F|\"1. Create a 20-day moving average (MA) ... 2. Multiply the current value of the moving average by 1.05 to get the upper band ... 3. Divide the current value of the moving average by 1.05 to get the lower band.\" (p.97)",
            "E|\"We trade a percentage band the same way as a channel, buying at the bottom, selling at the top, and exiting in the middle. It's a little easier to wait for a penetration of the band\" (p.97); \"As a countertrend trader you would sell when prices penetrated the top band and buy when prices penetrated the bottom band\" (Review answer 25, p.311)",
            "X|\"we wouldn't get out of the trade until the middle of the band is touched\" (p.98): exit at the moving average",
        ], "Source convention: selected 20/5 divided-lower-band interpretation (p.97), strict completed-close penetration, center equality exits, and opposite-band same-close reversal. The author recommends calculating just before close (p.70); using a completed daily close and filling at that close is an idealized research approximation. Actual individual stocks only; short exposure is hypothetical, without certified borrowing, financing or execution. Flat test start, full observed warmup, one position, terminal close entries retained as open. Original AMR figures, population and performance are not reproduced."),
        d("kfsc_bollinger", "Bollinger band countertrend (20-day, 2 SD)", "Ch 6 · Channels and bands", "Long/Short", Live, &[
            "F|\"Upper Bollinger band = average(A81:A100) + 2 × stdev(A81:A100); Lower Bollinger band = average(A81:A100) − 2 × stdev(A81:A100)\" (Excel, p.99)",
            "E|Bands are traded like the percentage band: buy on a penetration of the lower band, sell on a penetration of the upper band (p.97-101; Review answer 25, p.311)",
            "X|Exit at the centre of the band (the 20-day average): \"prices do not return to the center of the band until August\" (p.100-101)",
        ], "Source convention: choose the literal printed Excel STDEV of 20 closing price levels (sample, n-1), despite nearby prose saying closing price changes (p.99). This is a clearly selected formula interpretation, not a claim that the conflict is resolved. Strict completed-close penetration, center equality exit and opposite-band same-close reversal use idealized close fills (p.70 recommends preclose calculation). Actual individual stocks only; shorts are hypothetical. Flat test start, full warmup, one position, terminal close entries remain open. Other periods, including the 200-day example, are separate held configurations; original chart results are not reproduced."),
        // ------------------------------------------------ Ch 8 risk control: two trends
        d("kfsc_two_ma", "Two moving average system (both trends up, long only)", "Ch 8 · Controlling the risk of a trade", "Long", Live, &[
            "F|\"Choose a long calculation period (between 50 and 200 days) and a short calculation period (between 5 and 30 days) for two moving averages.\" (p.123)",
            "E|\"Buy when both trendlines are up\" (p.124)",
            "X|\"and sell when one trendline turns down (the fast one will always turn down before the slow one).\" (p.124)",
        ], "Source convention: select 80/10 from the Amazon example (Fig8.2 p.124), rather than universal mandatory periods. Up/down compares today's full SMA with yesterday's (p.52); equality holds and either falling SMA exits, including slow-first cases. Long/cash only on actual individual stocks. Completed-close same-close fills are an idealized research convention: p.70 recommends preclose calculation. Flat test start, full observed warmup, one position; terminal-close entries remain open. Original Amazon chart, population and performance are not reproduced."),
        // ------------------------------------------------ Ch 14 momentum and MACD
        d("kfsc_macd", "MACD crossover (literal .0476/.0243/.10)", "Ch 14 · Momentum and MACD", "Long/Short", Live, &[
            "F|\"slow = 40 and fast = 20 ... Step 2. Calculate the slow trendline using the smoothing value .0243. Step 3. Calculate the fast trendline using the smoothing value .0476. Step 4. The MACD line ... is calculated as the fast trendline minus the slow trendline. Step 5. The signal line is the 9-day smoothing (using a percentage of .10) of the MACD line.\" (p.204)",
            "E|\"Buy when the MACD line (faster) crosses above the signal line (slower). Sell when the MACD line crosses below the signal line.\" (p.205)",
            "X|Declared research convention: a subsequent opposite crossing closes the current position and reverses; this is not a printed portfolio or capital model.",
        ], "Source convention: choose the literal printed .0476/.0243/.10 (p.204), which do not equal either the stated 2/(N+1) formula or exact 1/(N+1). The book calls the example 20/40/9; these labels do not override its selected literal constants. First actual source close seeds fast/slow; their difference and signal begin at zero. All supplied earlier history is retained, not reset at the test start. First subsequent crossing enters from a flat selected start, equality holds, opposite crossing reverses; terminal-close entries remain open. Completed-close same-close fills are idealized (p.70 recommends preclose calculation). Actual stocks only; shorts are hypothetical without certified borrow/margin/financing. No discretionary divergence/resistance filter, original AOL chart performance, game capital or portfolio is reproduced."),
        // ------------------------------------------------ Ch 15 overbought/oversold
        d("kfsc_stoch", "Slow stochastic timing and regime selection", "Ch 15 · Overbought/oversold", "Long/Short", NotTestable("The source selects buy/sell directions using a sideways/up/down regime but gives no objective regime classifier (p.220,222-223). Raw oscillator timing alone is not the full strategy."), &[
            "F|Raw %K = 100 × (close − lowest low of past N days)/(high − low range of past N days) (p.216); %K-slow (%D) = 3-day average of %K; %D-slow = 3-day average of %K-slow (Fig 15.2, p.217)",
            "E|\"1. After the smoothed stochastic %K-slow has moved above the 80 percent line, sell when the %K-slow line crosses the %D-slow line moving down. 2. After the smoothed stochastic %K-slow has moved below the 20 percent line, buy when the %K-slow line crosses the %D-slow line moving up.\" (p.219)",
            "X|\"In a sideways market use both buy and sell signals\" (p.222): a buy closes a short and goes long, a sell closes a long and goes short",
        ], "Reading/timing component, not a verified backtest. The Nokia table (p.217) uses a 10-observation range including today, conflicting with the explicit preceding-range prose. The first qualifying threshold excursion/crossing also needs a precise re-arm convention. More materially, p.222 permits both directions in a sideways market but restricts directions in trends; the book supplies no mechanical regime classifier. An unfiltered two-way backtest would omit that condition. Legacy illustrative code and parameter alternatives inherit no source admission; no chart results are claimed."),
        d("kfsc_dsm", "Double-smoothed 10-day momentum (Blau)", "Ch 15 · Double smoothing", "Long/Short", Live, &[
            "F|\"1. Calculate the values of the 10-day momentum. 2. Smooth the momentum values using a 20-day percentage smoothing. 3. Smooth the values in step 2 using a 20-day percentage smoothing.\" (p.231); 20-day smoothing = .0476 (p.231)",
            "E|\"It is now possible to use the simple turns in the trendline for buy and sell signals\" (p.231); Fig 15.14: \"Buy when it turns up / Sell when indicator turns down\"",
            "X|Declared research convention: an opposite confirmed turn closes and reverses; a flat selected start waits for a new turn.",
        ], "Source convention: choose the explicit 10-day current-minus-prior momentum and two literal .0476 passes (p.231-232), not exact 1/21 and not the 1-day alternative. First actual 10-day momentum seeds both passes; all source history is retained. A turn changes the previous nonzero slope direction; equality holds, and the first observed slope only initializes direction. A flat selected start waits for the next genuine turn; opposite turns reverse. Terminal close entries remain open. Completed-close same-close fills are idealized (p.70); stock shorts are hypothetical without certified borrow/margin/financing. The generic first-difference sign wording on p.202 conflicts with the current-minus-prior examples on p.201; that sign choice is disclosed. Original Microsoft chart, discretionary combined methods, game risk/capital and portfolio are not reproduced."),
        // ------------------------------------------------ Ch 17 volatility
        d("kfsc_hv_day", "High-volatility day continuation", "Ch 17 · Volatility", "Long/Short", Live, &[
            "F|\"If today's true range (the range including the previous close) is more than twice the average [of the 65-day true range], then we have a high-volatility day.\" (p.250-251)",
            "E|\"1. Buy a high-volatility day if prices close in the upper 90 percent of the range. 2. Sell a high-volatility day if prices close in the lower 10 percent of the range.\" (p.250)",
            "X|\"3. Close out the trade shortly after the open of the next day.\" (p.250)",
            "F|\"4. Do not enter the trade on Friday.\" (p.250)",
        ], "Readings: 'upper 90 percent' is read as the top 10% of the day's high-low range (mirror of 'lower 10 percent'; Fig 17.4 'close near the high or low'); entry at the close of the signal day; exit at the next open. The 65-day average true range includes today (TradeStation AvgTrueRange convention) [book silent]; p.atr_lag=1 uses the previous 65 days."),
        // ------------------------------------------------ not testable
        d("kfsc_trendline", "Trendline penetration", "Ch 2 · Charting the trend", "Long/Short", NotTestable("trendlines are drawn and redrawn by eye (p.23-33)"), &[
            "E|\"Buy when the price closes above the downtrend line (conservative)\" / \"Sell when the price closes below the upward trendline\" (p.28-29)",
        ], "Chart reading. The automatic swing-point spreadsheet (p.32) only marks points; which ones form the line is still judged."),
        d("kfsc_sideways_breakout", "Sideways-range breakout and range trading", "Ch 3 · Breakout trends", "Long/Short", NotTestable("the sideways range and its support/resistance are chosen by eye (p.36-48)"), &[
            "E|\"You buy when prices make a new high above the previous resistance level. You sell when prices make a new low below the previous support level.\" (p.39)",
            "E|\"If the range is sufficiently developed, then buy when prices approach the support line\" (p.48)",
        ], "The rolling N-day version is mechanical and already in the vault (see duplicates in the README)."),
        d("kfsc_channel", "Trend channel trading", "Ch 6 · Channels and bands", "Long/Short", NotTestable("channels are hand-drawn; 'buy in the lower zone' uses 20-25% zones of a drawn channel (p.91-96)"), &[
            "E|\"buy in the lower zone of the channel and sell in the upper zone of the channel\" (p.93)",
        ], ""),
        d("kfsc_regression_band", "Rolling regression bands", "Ch 6 · Channels and bands", "Long/Short", NotTestable("band width ('standard deviation of the price changes') and the exit are not given exactly (p.102-104)"), &[
            "F|\"If you calculate a new 60-day regression every day and look only at the last point on the regression bands\" (p.103)",
        ], ""),
        d("kfsc_stops", "Support-level and volatility stops", "Ch 8/17 · Risk control", "Long/Short", NotTestable("exit rules only (support levels by eye; 2x/3x range targets need an entry the book does not give)"), &[
            "X|\"A stop-loss must not be closer than 1½ times the current volatility\" (p.120); \"Profit target = entry price + (2 × daily range); Stop-loss = entry price − (3 × daily range)\" (p.248-249)",
        ], ""),
        d("kfsc_day_patterns", "Gaps, spikes, island, reversal, run, wide-range and inside days", "Ch 9 · One-day patterns", "Long/Short", NotTestable("definitions only; trades need 'selection' and judgement (p.130-144)"), &[
            "F|\"A day in which there is a new high followed by a lower close is a downward reversal day\" (p.137); run day, thrust day definitions (p.139)",
        ], "The book: 'You cannot expect to profit from a one-day pattern unless you are selective' (p.142)."),
        d("kfsc_chart_patterns", "Continuation, top and bottom formations", "Ch 10/11 · Chart patterns", "Long/Short", NotTestable("triangles, flags, double tops, head-and-shoulders are identified by eye (p.145-166)"), &[
            "X|Profit target = height of the formation projected from the confirmation point (p.159-165)",
        ], ""),
        d("kfsc_retracements", "Retracements, Fibonacci and Gann", "Ch 12 · Retracements", "Long/Short", NotTestable("no entry or exit rule; levels need a judged swing (p.167-179)"), &[
            "F|Most important retracements 100%, 50%, 61.8% (p.171)",
        ], ""),
        d("kfsc_volume", "Volume spikes and volume indicators", "Ch 13 · Volume, breadth, open interest", "Long/Short", NotTestable("indicators and spike definition only; no trading rule (p.180-198)"), &[
            "F|Volume outlier: today's volume 2-3x (stocks) or 1.5-2x (index) the average of days 6 to 25 ago (p.191-192)",
        ], ""),
        d("kfsc_divergence", "MACD and stochastic divergence", "Ch 14/15 · Divergence", "Long/Short", NotTestable("price/indicator peaks are picked by eye; 'clearly rising' peaks (p.206-209, 224-229)"), &[
            "E|\"Once the second price peak has been made in the bearish divergence, we look for the MACD line crossing the signal line heading down. We can now sell.\" (p.208)",
            "X|\"that happens when the value of the MACD reaches zero\" (p.209)",
        ], "Kaufman's mechanical divergence code (TSM Divergence) is coded in kfbpm_divergence."),
        d("kfsc_high_momentum", "High-momentum threshold trading", "Ch 14 · Momentum", "Long/Short", NotTestable("±15 threshold fitted to one AOL chart; exit 'a few minutes ... two or three days' (p.210-211)"), &[
            "E|\"trading in the direction of the trend only when momentum exceeds a high threshold level (for example, +15)\" (p.211)",
        ], ""),
        d("kfsc_dow_theory", "Dow Theory bull/bear signals", "Ch 18 · Dow Theory", "Long/Short", NotTestable("swing size 'perhaps 10 percent' and confirmation by two of DJIA/DJT/DJU (transport and utility averages not in the vault) (p.262-273)"), &[
            "E|\"The bull market signal is at the point where prices confirm the uptrend by moving above the high of the previous rally. The bear market signal occurs on a break below the low of the previous decline\" (p.264-265)",
        ], ""),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "kfsc_pct_band" => ("kfsc_bands", "Countertrend bands (Short Course)", "5% band"),
        "kfsc_bollinger" => ("kfsc_bands", "Countertrend bands (Short Course)", "Bollinger 20/2"),
        "kfsc_macd" | "kfsc_dsm" => ("kfsc_momentum", "Momentum trend (Short Course)", ""),
        s if s.starts_with("kfsc_") => (s_static(s), "Kaufman Short Course", ""),
        _ => return None,
    })
}
fn s_static(s: &str) -> &'static str {
    catalogue().into_iter().find(|d| d.id == s).map(|d| d.id).unwrap_or("kfsc")
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(match id {
        "kfsc_pct_band" => vec![p("period", "Selected source MA period", 20., 20., 20., 1.), p("pct", "Selected band (%): upper MA×1.05, lower MA/1.05", 5., 5., 5., 1.)],
        "kfsc_bollinger" => vec![p("period", "Selected source MA / sample-SD period", 20., 20., 20., 1.), p("nsd", "Selected price-level sample SDs (literal Excel formula)", 2., 2., 2., 1.)],
        "kfsc_two_ma" => vec![p("slow", "Selected example slow SMA", 80., 80., 80., 1.), p("fast", "Selected example fast SMA", 10., 10., 10., 1.)],
        "kfsc_macd" => vec![p("alpha_fast", "Literal printed fast smoothing (formula conflict)", 0.0476, 0.0476, 0.0476, 0.0001), p("alpha_slow", "Literal printed slow smoothing (formula conflict)", 0.0243, 0.0243, 0.0243, 0.0001), p("alpha_signal", "Literal printed MACD signal smoothing", 0.10, 0.10, 0.10, 0.01)],
        "kfsc_stoch" => vec![p("period", "Stochastic days N", 10., 3., 40., 1.), p("prior", "Range: 0 = N days to today (Fig 15.2 numbers), 1 = N days before today (prose)", 0., 0., 1., 1.), p("over", "Overbought line", 80., 50., 95., 1.), p("under", "Oversold line", 20., 5., 50., 1.)],
        "kfsc_dsm" => vec![p("mom", "Selected source momentum days", 10., 10., 10., 1.), p("alpha", "Literal printed smoothing, each pass", 0.0476, 0.0476, 0.0476, 0.0001)],
        "kfsc_hv_day" => vec![p("atr", "Average true range days", 65., 10., 130., 1.), p("factor", "High-volatility factor (x average TR)", 2., 1., 4., 0.1), p("zone", "Close zone (fraction of range from the extreme)", 0.1, 0.05, 0.3, 0.05), p("atr_lag", "0 = average includes today, 1 = previous days", 0., 0., 1., 1.)],
        _ => {
            if catalogue().iter().any(|d| d.id == id) { vec![] } else { return None }
        }
    })
}

// ---------------------------------------------------------------- order book (one position, full equity)

struct Bk<'a> {
    x: &'a Ctx<'a>,
    from: usize,
    to: usize,
    out: Vec<Trade>,
    cur: Option<Trade>,
    terminal_close_entry: bool,
}

impl<'a> Bk<'a> {
    fn new(x: &'a Ctx<'a>, from: usize, to: usize) -> Self { Bk { x, from, to, out: vec![], cur: None, terminal_close_entry: false } }
    fn source_close(x: &'a Ctx<'a>, from: usize, to: usize) -> Self {
        let mut b=Self::new(x,from,to);b.terminal_close_entry=true;b
    }
    fn pos(&self) -> i8 { self.cur.as_ref().map_or(0, |t| t.dir as i8) }
    fn exit(&mut self, i: usize, px: f64, why: &str) {
        if let Some(mut t) = self.cur.take() {
            t.exit_i = i; t.exit_px = px; t.signal_i = i; t.reason = why.into();
            self.out.push(t);
        }
    }
    /// Target position after closing the old holding. Only source_close permits
    /// an actual terminal-close entry, retained as open with estimated exit costs.
    fn go(&mut self, i: usize, dir: i8, px: f64, kind: &'static str, why: &str) {
        if i < self.from || dir == self.pos() { return; }
        if self.cur.is_some() { self.exit(i, px, why); }
        if dir != 0 && (i < self.to || (self.terminal_close_entry && i == self.to)) {
            self.cur = Some(Trade { dir: dir as f64, legs: vec![Leg { i, px, w: 1. }], exit_i: i, exit_px: px, reason: String::new(), entry_kind: kind, signal_i: i, open: false, exits: vec![] });
        }
    }
    fn at_close(&mut self, i: usize, dir: i8, why: &str) { let c = self.x.c[i]; self.go(i, dir, c, "close", why); }
    fn done(mut self) -> Vec<Trade> {
        if let Some(mut t) = self.cur.take() {
            t.exit_i = self.to; t.exit_px = self.x.c[self.to]; t.signal_i = self.to; t.reason = "open".into(); t.open = true;
            self.out.push(t);
        }
        self.out
    }
}

// ---------------------------------------------------------------- helpers

/// Exponential smoothing e = e[1] + a*(v - e[1]), seeded with the first finite value.
fn expo(v: &[f64], a: f64) -> Vec<f64> {
    let mut out = vec![f64::NAN; v.len()];
    let mut prev = f64::NAN;
    for i in 0..v.len() {
        if !v[i].is_finite() { continue; }
        prev = if prev.is_finite() { prev + a * (v[i] - prev) } else { v[i] };
        out[i] = prev;
    }
    out
}
struct MacdStates {fast:Vec<f64>,slow:Vec<f64>,macd:Vec<f64>,signal:Vec<f64>,histogram:Vec<f64>,events:Vec<i8>}
/// Literal step2/3/5 constants on printed204, deliberately not either reciprocal
/// interpretation of the inconsistent step1 period formula. First close seeds
/// both price smoothings; the resulting difference/signal begin at zero.
fn literal_macd_states(c:&[f64])->MacdStates {
    let fast=expo(c,0.0476);let slow=expo(c,0.0243);
    let macd:Vec<f64>=fast.iter().zip(&slow).map(|(f,s)|f-s).collect();
    let signal=expo(&macd,0.10);let histogram:Vec<f64>=macd.iter().zip(&signal).map(|(m,s)|m-s).collect();
    let mut events=vec![0;c.len()];
    for i in 1..c.len() {
        if histogram[i]>0. && histogram[i-1]<=0. {events[i]=1;}
        else if histogram[i]<0. && histogram[i-1]>=0. {events[i]=-1;}
    }
    MacdStates {fast,slow,macd,signal,histogram,events}
}
struct DsmStates {momentum:Vec<f64>,first:Vec<f64>,second:Vec<f64>,events:Vec<i8>}
/// Printed231/232 selected10-day momentum, double literal .0476. A turn changes
/// the last nonzero slope. Ties retain that direction; the first observed slope
/// initializes direction, rather than inventing a prior turn at a test start.
fn literal_dsm_states(c:&[f64])->DsmStates {
    let momentum:Vec<f64>=(0..c.len()).map(|i|if i>=10 {c[i]-c[i-10]} else {f64::NAN}).collect();
    let first=expo(&momentum,0.0476);let second=expo(&first,0.0476);
    let mut events=vec![0;c.len()];let mut direction=0;
    for i in 11..c.len() {
        if !second[i].is_finite() || !second[i-1].is_finite() {continue;}
        let next=if second[i]>second[i-1] {1} else if second[i]<second[i-1] {-1} else {0};
        if next!=0 {if direction!=0 && next!=direction {events[i]=next;}direction=next;}
    }
    DsmStates {momentum,first,second,events}
}
/// Excel STDEV (sample, n-1) of v[i+1-n..=i].
fn sd_sample(v: &[f64], n: usize, i: usize) -> f64 {
    if i + 1 < n || n < 2 { return f64::NAN; }
    let w = &v[i + 1 - n..=i];
    let m = w.iter().sum::<f64>() / n as f64;
    (w.iter().map(|x| (x - m) * (x - m)).sum::<f64>() / (n as f64 - 1.)).sqrt()
}
fn days(d: &str) -> i64 {
    let y: i64 = d[0..4].parse().unwrap_or(1970);
    let m: i64 = d[5..7].parse().unwrap_or(1);
    let dd: i64 = d[8..10].parse().unwrap_or(1);
    let y2 = if m <= 2 { y - 1 } else { y };
    let era = y2.div_euclid(400);
    let yoe = y2 - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + dd - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}
fn weekday(d: &str) -> i64 { (days(d) + 3).rem_euclid(7) } // 0 = Monday

/// Raw stochastic %K over n days. `prior` = false: the n days ending today (the book's worked Fig 15.2 numbers);
/// true: the n days before today, as the prose says ('not including today'), clamped to 0..100 (p.216-217).
pub fn raw_k(x: &Ctx, n: usize, prior: bool) -> Vec<f64> {
    let len = x.c.len();
    let mut k = vec![f64::NAN; len];
    for i in 0..len {
        if i + 1 < n + prior as usize { continue; }
        let (a, b) = if prior { (i - n, i) } else { (i + 1 - n, i + 1) };
        let hh = x.h[a..b].iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let ll = x.l[a..b].iter().copied().fold(f64::INFINITY, f64::min);
        if hh > ll { k[i] = ((x.c[i] - ll) * 100. / (hh - ll)).clamp(0., 100.); }
    }
    k
}

/// N-day average summed afresh each bar, oldest to newest (like a spreadsheet AVERAGE; no running-sum drift).
fn avg_n(v: &[f64], n: usize) -> Vec<f64> {
    (0..v.len()).map(|i| if i + 1 >= n { v[i + 1 - n..=i].iter().fold(0., |a, b| a + b) / n as f64 } else { f64::NAN }).collect()
}
/// 3-day average summed directly (a+b+c)/3, so equal inputs give exactly equal averages (ties matter for crossings).
fn avg3(v: &[f64]) -> Vec<f64> { (0..v.len()).map(|i| if i >= 2 { (v[i - 2] + v[i - 1] + v[i]) / 3. } else { f64::NAN }).collect() }

/// Countertrend band book: long below `lo`, short above `up`, exit when the close reaches `mid` (all at the close).
fn band_ct(x: &Ctx, from: usize, to: usize, lo: &[f64], mid: &[f64], up: &[f64]) -> Vec<Trade> {
    let c = &x.c;
    let mut b = Bk::source_close(x, from, to);
    for i in from..=to {
        if !(mid[i].is_finite() && lo[i].is_finite() && up[i].is_finite()) { continue; }
        match b.pos() {
            1 if c[i] >= mid[i] => b.at_close(i, 0, "close reached the average"),
            -1 if c[i] <= mid[i] => b.at_close(i, 0, "close reached the average"),
            _ => {}
        }
        if b.pos() == 0 {
            if c[i] < lo[i] { b.at_close(i, 1, "close below lower band"); } else if c[i] > up[i] { b.at_close(i, -1, "close above upper band"); }
        }
    }
    b.done()
}

// ---------------------------------------------------------------- run

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if validate_reviewed(id,p).is_some() {return None;}
    if validate_momentum(id,p).is_some() {return None;}
    if from > to || to >= x.r.len() { return None; }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(0., |d| d.def));
    let u = |k: &str| g(k).round().max(0.) as usize;
    let n = x.r.len();
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    Some(match id {
        "kfsc_pct_band" => {
            let ma = avg_n(c, u("period").max(1));
            let f = 1. + g("pct") / 100.;
            let up: Vec<f64> = ma.iter().map(|m| m * f).collect();
            let lo: Vec<f64> = ma.iter().map(|m| m / f).collect();
            band_ct(x, from, to, &lo, &ma, &up)
        }
        "kfsc_bollinger" => {
            let pn = u("period").max(2);
            let ma = avg_n(c, pn);
            let k = g("nsd");
            let up: Vec<f64> = (0..n).map(|i| ma[i] + k * sd_sample(c, pn, i)).collect();
            let lo: Vec<f64> = (0..n).map(|i| ma[i] - k * sd_sample(c, pn, i)).collect();
            band_ct(x, from, to, &lo, &ma, &up)
        }
        "kfsc_two_ma" => {
            // direction of an N-day average = sign of c[i] - c[i-N] (exact; avoids running-sum noise on flat days)
            let (sn, fnn) = (u("slow").max(1), u("fast").max(1));
            let mut b = Bk::source_close(x, from, to);
            for i in from.max(sn.max(fnn))..=to {
                let (su, sd_) = (c[i] > c[i - sn], c[i] < c[i - sn]);
                let (fu, fd) = (c[i] > c[i - fnn], c[i] < c[i - fnn]);
                if b.pos() == 1 && (fd || sd_) { b.at_close(i, 0, "a trendline turned down"); }
                else if b.pos() == 0 && su && fu { b.at_close(i, 1, "both trendlines up"); }
            }
            b.done()
        }
        "kfsc_macd" => {
            let state=literal_macd_states(c);let mut b=Bk::source_close(x,from,to);
            for i in from.max(1)..=to {
                if state.events[i]==1 {b.at_close(i,1,"MACD crossed above signal");}
                else if state.events[i]==-1 {b.at_close(i,-1,"MACD crossed below signal");}
            }
            b.done()
        }
        "kfsc_stoch" => {
            let k = raw_k(x, u("period").max(1), u("prior") == 1);
            let ks = avg3(&k);
            let ds = avg3(&ks);
            let (ob, os) = (g("over"), g("under"));
            let mut b = Bk::new(x, from, to);
            let (mut arm_s, mut arm_b) = (false, false);
            for i in 1..=to {
                if !(ks[i].is_finite() && ds[i].is_finite() && ks[i - 1].is_finite() && ds[i - 1].is_finite()) { continue; }
                if ks[i] > ob { arm_s = true; }
                if ks[i] < os { arm_b = true; }
                if arm_s && ks[i] < ds[i] && ks[i - 1] >= ds[i - 1] { arm_s = false; b.at_close(i, -1, "%K-slow crossed down after overbought"); }
                else if arm_b && ks[i] > ds[i] && ks[i - 1] <= ds[i - 1] { arm_b = false; b.at_close(i, 1, "%K-slow crossed up after oversold"); }
            }
            b.done()
        }
        "kfsc_dsm" => {
            let state=literal_dsm_states(c);let mut b=Bk::source_close(x,from,to);
            for i in from.max(12)..=to {
                if state.events[i]==1 {b.at_close(i,1,"double-smoothed momentum turned up");}
                else if state.events[i]==-1 {b.at_close(i,-1,"double-smoothed momentum turned down");}
            }
            b.done()
        }
        "kfsc_hv_day" => {
            let tr = true_range(x.r);
            let at = sma(&tr, u("atr").max(1));
            let (k, z, lag) = (g("factor"), g("zone"), u("atr_lag").min(1));
            let mut b = Bk::new(x, from, to);
            for i in from.max(1)..=to {
                if b.pos() != 0 { b.exit(i, o[i], "next open"); }
                let a = at[i - lag];
                let rg = h[i] - l[i];
                if !a.is_finite() || rg <= 0. || tr[i] <= k * a || weekday(&x.r[i].date) == 4 { continue; }
                if c[i] >= h[i] - z * rg { b.at_close(i, 1, ""); } else if c[i] <= l[i] + z * rg { b.at_close(i, -1, ""); }
            }
            b.done()
        }
        _ => return None, // not-testable entries
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn mk(hlc: &[(f64, f64, f64)]) -> Vec<Row> {
        hlc.iter().enumerate().map(|(i, &(h, l, c))| Row { ticker: "T".into(), date: format!("1996-01-{:02}", i + 1), open: c, high: h, low: l, close: c, volume: None }).collect()
    }
    #[test]
    fn literal_momentum_constants_and_first_two_steps_match_printed_choices() {
        let s=literal_macd_states(&[100.,110.,90.]);
        for (got,want) in [(s.fast[1],100.476),(s.slow[1],100.243),(s.signal[1],0.0233),
            (s.fast[2],99.9773424),(s.slow[2],99.9940951),(s.signal[2],0.01929473)] {
            assert!((got-want).abs()<1e-12,"{got} vs {want}");
        }
        assert_eq!(s.events,vec![0,1,-1]);
        assert!((s.fast[1]-(100.+10./21.)).abs()>1e-5);
    }
    #[test]
    fn dsm_uses_actual_turns_not_every_rising_bar_and_preserves_terminal_entry() {
        let mut values=vec![100.;10];values.extend([110.,90.,130.,60.]);
        let s=literal_dsm_states(&values);
        assert!((s.first[11]-9.048).abs()<1e-12);
        assert!((s.second[11]-9.9546848).abs()<1e-12);
        assert!((s.second[12]-9.95899880704).abs()<1e-12);
        assert_eq!(&s.events[10..],&[0,0,1,-1]);
        let r=mk(&values.iter().map(|&v|(v,v,v)).collect::<Vec<_>>());let x=Ctx::new(&r,&r,&[]);
        let t=run("kfsc_dsm",&x,11,13,&Params::new()).unwrap();assert_eq!(t.len(),2);
        assert_eq!((t[0].legs[0].i,t[0].exit_i,t[0].dir,t[0].open),(12,13,1.,false));
        assert_eq!((t[1].legs[0].i,t[1].exit_i,t[1].dir,t[1].open),(13,13,-1.,true));
        let s=literal_dsm_states(&[100.;30]);assert!(s.events.iter().all(|&e|e==0));
    }
    #[test]
    fn macd_reverses_at_terminal_close_without_a_synthetic_exit() {
        let r=mk(&[(100.,100.,100.),(110.,110.,110.),(90.,90.,90.)]);let x=Ctx::new(&r,&r,&[]);
        let t=run("kfsc_macd",&x,1,2,&Params::new()).unwrap();assert_eq!(t.len(),2);
        assert_eq!((t[0].legs[0].i,t[0].exit_i,t[0].dir,t[0].open),(1,2,1.,false));
        assert_eq!((t[1].legs[0].i,t[1].exit_i,t[1].dir,t[1].open),(2,2,-1.,true));
    }
    #[test]
    fn momentum_contract_is_exact_and_stochastic_remains_a_reading_component() {
        for &id in MOMENTUM_STOCK_IDS {
            assert!(validate_momentum(id,&Params::new()).is_none());
            for s in params(id).unwrap() {
                assert_eq!((s.min,s.max),(s.def,s.def));
                assert!(validate_momentum(id,&Params::from([(s.key.into(),s.def)])).is_none());
                for value in [s.def+1e-10,s.def+1.,f64::NAN,f64::INFINITY] {
                    assert!(validate_momentum(id,&Params::from([(s.key.into(),value)])).is_some());
                }
            }
            for key in ["kind","fill","unknown","fast","smooth"] {
                assert!(validate_momentum(id,&Params::from([(key.into(),0.)])).is_some());
            }
        }
        let stoch=catalogue().into_iter().find(|d|d.id=="kfsc_stoch").unwrap();
        assert!(matches!(stoch.status,Status::NotTestable(note) if note.contains("regime classifier")));
        assert!(!reviewed_momentum_rule(stoch.id));assert!(!reviewed_stock_rule(stoch.id));
    }
    #[test]
    fn momentum_prefix_states_are_causal_without_resetting_full_history() {
        let c:Vec<f64>=(0..100).map(|i|100.+((i*37)%29) as f64).collect();
        let macd=literal_macd_states(&c);let dsm=literal_dsm_states(&c);
        for end in 14..c.len() {
            let m=literal_macd_states(&c[..=end]);let d=literal_dsm_states(&c[..=end]);
            assert_eq!(m.events,macd.events[..=end]);assert_eq!(m.histogram,macd.histogram[..=end]);
            assert_eq!(d.events,dsm.events[..=end]);assert_eq!(d.second[10..],dsm.second[10..=end]);
        }
    }
    #[test]
    #[ignore = "Private bounded actual-stock state audit: explicit CSV/OUT paths required"]
    fn dump_actual_momentum_source_states() {
        use std::io::Write;
        let input=std::env::var("KFSC_MOM_AUDIT_CSV").expect("explicit private input CSV required");
        let output=std::env::var("KFSC_MOM_AUDIT_OUT").expect("explicit private output CSV required");
        assert_ne!(input,output);let text=std::fs::read_to_string(input).unwrap();let mut lines=text.lines();
        let header=crate::parse_csv_fields(lines.next().unwrap());
        let date=header.iter().position(|s|s=="date").unwrap();let close=header.iter().position(|s|s=="close").unwrap();
        let rows:Vec<Vec<String>>=lines.filter(|s|!s.trim().is_empty()).map(crate::parse_csv_fields).collect();
        let c:Vec<f64>=rows.iter().map(|r|r[close].parse().unwrap()).collect();assert!(c.iter().all(|v|v.is_finite() && *v>0.));
        let m=literal_macd_states(&c);let d=literal_dsm_states(&c);
        let mut f=std::io::BufWriter::new(std::fs::File::create(output).unwrap());
        writeln!(f,"date,close,fast,slow,macd,signal,histogram,macd_event,momentum,first,second,dsm_event").unwrap();
        for i in 0..c.len() {
            writeln!(f,"{},{:.17},{:.17},{:.17},{:.17},{:.17},{:.17},{},{:.17},{:.17},{:.17},{}",rows[i][date],c[i],m.fast[i],m.slow[i],m.macd[i],m.signal[i],m.histogram[i],m.events[i],d.momentum[i],d.first[i],d.second[i],d.events[i]).unwrap();
        }
    }
    #[test]
    fn reviewed_short_course_accepts_only_exact_finite_source_choices() {
        for &id in REVIEWED_STOCK_IDS {
            assert!(validate_reviewed(id,&Params::new()).is_none());
            for s in params(id).unwrap() {
                assert!(validate_reviewed(id,&Params::from([(s.key.into(),s.def)])).is_none());
                for v in [s.def+1.,s.def+1e-10,f64::NAN,f64::INFINITY] {
                    assert!(validate_reviewed(id,&Params::from([(s.key.into(),v)])).is_some());
                }
            }
            assert!(validate_reviewed(id,&Params::from([("unknown".into(),0.)])).is_some());
        }
        assert!(!reviewed_stock_rule("kfsc_regression_band"));
        assert!(!reviewed_stock_rule("kfsc_macd"));
    }
    #[test]
    fn reviewed_bands_fill_terminal_close_and_retain_open_holding() {
        let mut prices=vec![(100.,100.,100.);21];prices[20]=(80.,80.,80.);
        let r=mk(&prices);let x=Ctx::new(&r,&r,&[]);
        for id in ["kfsc_pct_band","kfsc_bollinger"] {
            let t=run(id,&x,19,20,&Params::new()).unwrap();assert_eq!(t.len(),1);
            assert_eq!((t[0].legs[0].i,t[0].exit_i,t[0].open),(20,20,true));
        }
        let mut legacy=Bk::new(&x,19,20);legacy.at_close(20,1,"legacy");assert!(legacy.done().is_empty());
    }
    #[test]
    fn reviewed_percentage_reverses_after_center_exit_without_overlap() {
        let mut prices=vec![(100.,100.,100.);20];
        prices.extend([(80.,80.,80.),(80.,80.,80.),(120.,120.,120.),(98.,98.,98.)]);
        let r=mk(&prices);let x=Ctx::new(&r,&r,&[]);
        let t=run("kfsc_pct_band",&x,19,23,&Params::new()).unwrap();assert_eq!(t.len(),2);
        assert_eq!((t[0].dir,t[0].legs[0].i,t[0].exit_i,t[0].open),(1.,20,22,false));
        assert_eq!((t[1].dir,t[1].legs[0].i,t[1].exit_i,t[1].open),(-1.,22,23,false));
    }
    #[test]
    fn reviewed_two_ma_retains_terminal_entry_and_slow_first_exit() {
        let mut prices=vec![(100.,100.,100.);82];
        prices[0]=(1000.,1000.,1000.);prices[1]=(1000.,1000.,1000.);
        prices[80]=(1100.,1100.,1100.);prices[81]=(150.,150.,150.);
        let r=mk(&prices);let x=Ctx::new(&r,&r,&[]);
        let t=run("kfsc_two_ma",&x,80,80,&Params::new()).unwrap();assert_eq!(t.len(),1);assert!(t[0].open);
        let t=run("kfsc_two_ma",&x,80,81,&Params::new()).unwrap();assert_eq!(t.len(),1);
        assert_eq!((t[0].legs[0].i,t[0].exit_i,t[0].open),(80,81,false));
        assert!(x.c[81]>x.c[71]);assert!(x.c[81]<x.c[1]);
    }
    #[test]
    fn reviewed_percentage_center_equality_exits_and_band_equality_is_no_entry() {
        let r=mk(&[(100.,100.,100.);3]);let x=Ctx::new(&r,&r,&[]);
        // Use explicit levels to isolate touch/strictness from MA arithmetic.
        let t=band_ct(&x,0,2,&[100.,101.,99.],&[100.,102.,100.],&[100.,103.,101.]);
        assert_eq!(t.len(),1);assert_eq!((t[0].legs[0].i,t[0].exit_i,t[0].open),(1,2,false));
    }
    #[test]
    fn raw_stochastic_matches_fig_15_2() {
        // Nokia, 2 Jan - 8 Feb 1996 (book Fig 15.2, p.217): %K column
        let hlc = [(2.50, 2.45, 2.50), (2.55, 2.41, 2.44), (2.39, 2.29, 2.34), (2.30, 2.19, 2.21), (2.23, 2.19, 2.19), (2.20, 2.14, 2.19), (2.20, 2.07, 2.16),
            (2.27, 2.15, 2.27), (2.26, 2.18, 2.19), (2.21, 2.10, 2.12), (2.25, 2.12, 2.25), (2.30, 2.18, 2.20), (2.30, 2.23, 2.29), (2.33, 2.25, 2.30), (2.37, 2.27, 2.37)];
        let r = mk(&hlc);
        let x = Ctx::new(&r, &r, &[]);
        let k = raw_k(&x, 10, false);
        let want = [10.4, 37.5, 40.6, 95.7, 88.5, 100.0];
        for (j, w) in want.iter().enumerate() { assert!((k[9 + j] - w).abs() < 0.06, "K[{}] = {} vs {}", 9 + j, k[9 + j], w); }
    }
    #[test]
    fn hv_day_buys_close_exits_next_open() {
        let mut hlc = vec![(101., 99., 100.); 80];
        hlc[70] = (110., 99., 109.6); // TR 11 > 2x avg, close in the top 10%
        let mut r = mk(&hlc);
        for (i, row) in r.iter_mut().enumerate() { row.date = format!("{}", ["2024-01-01", "2024-01-02", "2024-01-03"][i % 3]); } // never a Friday
        r[71].open = 105.;
        let x = Ctx::new(&r, &r, &[]);
        let ts = run("kfsc_hv_day", &x, 1, 79, &Params::new()).unwrap();
        assert_eq!(ts.len(), 1);
        assert_eq!((ts[0].legs[0].i, ts[0].legs[0].px, ts[0].exit_i, ts[0].exit_px), (70, 109.6, 71, 105.));
    }
}
