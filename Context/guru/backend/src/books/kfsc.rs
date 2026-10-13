//! Perry J. Kaufman, *A Short Course in Technical Trading* (Wiley 2003, 339 PDF pages, text layer).
//! Page refs: printed page (PDF page = printed + 13). Only rules the book states exactly are Live; systems already in
//! the vault (mostly kftsm, the same author's TSM 4th ed.) are listed as duplicates in library/kfsc/README.md and not
//! re-coded. Every default is the book's literal number; readings the book leaves open are labelled in `notes`.
use crate::connors::{sma, true_range, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "kfsc";

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
        ], "Reading [book silent on bar mechanics]: penetration and touch are judged on the close and filled at the close (the book's own rule 'Trade on the close, not on the next open', p.70). Long: close < MA/1.05; short: close > MA*1.05; exit when the close reaches the MA. A close beyond the other band exits and reverses. Instrument swap only (the book trades stocks/futures)."),
        d("kfsc_bollinger", "Bollinger band countertrend (20-day, 2 SD)", "Ch 6 · Channels and bands", "Long/Short", Live, &[
            "F|\"Upper Bollinger band = average(A81:A100) + 2 × stdev(A81:A100); Lower Bollinger band = average(A81:A100) − 2 × stdev(A81:A100)\" (Excel, p.99)",
            "E|Bands are traded like the percentage band: buy on a penetration of the lower band, sell on a penetration of the upper band (p.97-101; Review answer 25, p.311)",
            "X|Exit at the centre of the band (the 20-day average): \"prices do not return to the center of the band until August\" (p.100-101)",
        ], "Std dev = Excel STDEV of the 20 closes (sample, n-1) as in the book's formula; the prose says 'closing price changes' but the printed formula uses prices and is followed. Same close-based reading as kfsc_pct_band. p.period=200 = the book's 200-day example (Fig 6.8)."),
        // ------------------------------------------------ Ch 8 risk control: two trends
        d("kfsc_two_ma", "Two moving average system (both trends up, long only)", "Ch 8 · Controlling the risk of a trade", "Long", Live, &[
            "F|\"Choose a long calculation period (between 50 and 200 days) and a short calculation period (between 5 and 30 days) for two moving averages.\" (p.123)",
            "E|\"Buy when both trendlines are up\" (p.124)",
            "X|\"and sell when one trendline turns down (the fast one will always turn down before the slow one).\" (p.124)",
        ], "Defaults 80 / 10 = the book's Amazon example (Fig 8.2, p.124: 'we plot both an 80-day moving average ... and a 10-day moving average'). 'Up' = today's average above yesterday's (the book's trend definition, p.52). Long only: 'you can only be long during this period when both trendlines are moving up' (Fig 8.2). Orders at the close (p.70)."),
        // ------------------------------------------------ Ch 14 momentum and MACD
        d("kfsc_macd", "MACD signal-line crossover (Kaufman 20/40/9)", "Ch 14 · Momentum and MACD", "Long/Short", Live, &[
            "F|\"slow = 40 and fast = 20 ... Step 2. Calculate the slow trendline using the smoothing value .0243. Step 3. Calculate the fast trendline using the smoothing value .0476. Step 4. The MACD line ... is calculated as the fast trendline minus the slow trendline. Step 5. The signal line is the 9-day smoothing (using a percentage of .10) of the MACD line.\" (p.204)",
            "E|\"Buy when the MACD line (faster) crosses above the signal line (slower). Sell when the MACD line crosses below the signal line.\" (p.205)",
            "X|Always in: the opposite crossover reverses (MACD 'as a trend indicator', p.205)",
        ], "Defaults = the printed constants .0476/.0243/.10, i.e. 1/(n+1) for n = 20, 40, 9 (the text's general formula 2/(n+1) would give .0952/.0488/.20; p.kind=1 uses it). Smoothings seeded with the first value. Close-based, orders at the close."),
        // ------------------------------------------------ Ch 15 overbought/oversold
        d("kfsc_stoch", "Slow stochastic 80/20 crossover", "Ch 15 · Overbought/oversold", "Long/Short", Live, &[
            "F|Raw %K = 100 × (close − lowest low of past N days)/(high − low range of past N days) (p.216); %K-slow (%D) = 3-day average of %K; %D-slow = 3-day average of %K-slow (Fig 15.2, p.217)",
            "E|\"1. After the smoothed stochastic %K-slow has moved above the 80 percent line, sell when the %K-slow line crosses the %D-slow line moving down. 2. After the smoothed stochastic %K-slow has moved below the 20 percent line, buy when the %K-slow line crosses the %D-slow line moving up.\" (p.219)",
            "X|\"In a sideways market use both buy and sell signals\" (p.222): a buy closes a short and goes long, a sell closes a long and goes short",
        ], "N = 10: the book's worked 10-day stochastic (Fig 15.2, Nokia). The prose says the range is 'not including today', but every %K printed in Fig 15.2 uses the 10 days ending today (e.g. 18 Jan 1996: high 2.30, low 2.07, close 2.29 -> 95.7); the printed numbers are followed, p.prior=1 gives the prose reading. 'After ... moved above 80' = the first %K-slow down-cross since %K-slow was last above 80 ('the first time after the faster line has been above 80', p.220). The trend-filtered variant (trade only with the trend, p.220-222) has no defined trend and is not coded. Orders at the close."),
        d("kfsc_dsm", "Double-smoothed 10-day momentum (Blau)", "Ch 15 · Double smoothing", "Long/Short", Live, &[
            "F|\"1. Calculate the values of the 10-day momentum. 2. Smooth the momentum values using a 20-day percentage smoothing. 3. Smooth the values in step 2 using a 20-day percentage smoothing.\" (p.231); 20-day smoothing = .0476 (p.231)",
            "E|\"It is now possible to use the simple turns in the trendline for buy and sell signals\" (p.231); Fig 15.14: \"Buy when it turns up / Sell when indicator turns down\"",
            "X|Always in: the opposite turn reverses",
        ], "Smoothing constant .0476 as printed (= 1/(20+1)); p.kind=1 uses 2/(n+1). Smoothings seeded with the first momentum value. Orders at the close."),
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
        "kfsc_pct_band" => vec![p("period", "Moving average period", 20., 2., 250., 1.), p("pct", "Band (%): upper = MA×(1+pct), lower = MA/(1+pct)", 5., 0.5, 20., 0.5)],
        "kfsc_bollinger" => vec![p("period", "Average / std-dev period", 20., 5., 250., 1.), p("nsd", "Band std devs", 2., 0.5, 4., 0.1)],
        "kfsc_two_ma" => vec![p("slow", "Slow moving average", 80., 50., 200., 1.), p("fast", "Fast moving average", 10., 5., 30., 1.)],
        "kfsc_macd" => vec![p("fast", "Fast period", 20., 2., 60., 1.), p("slow", "Slow period", 40., 5., 120., 1.), p("signal", "Signal period", 9., 2., 30., 1.), p("kind", "Constant: 0 = 1/(n+1) as printed, 1 = 2/(n+1)", 0., 0., 1., 1.)],
        "kfsc_stoch" => vec![p("period", "Stochastic days N", 10., 3., 40., 1.), p("prior", "Range: 0 = N days to today (Fig 15.2 numbers), 1 = N days before today (prose)", 0., 0., 1., 1.), p("over", "Overbought line", 80., 50., 95., 1.), p("under", "Oversold line", 20., 5., 50., 1.)],
        "kfsc_dsm" => vec![p("mom", "Momentum days", 10., 1., 40., 1.), p("smooth", "Smoothing period (each pass)", 20., 2., 60., 1.), p("kind", "Constant: 0 = 1/(n+1) (.0476 printed), 1 = 2/(n+1)", 0., 0., 1., 1.)],
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
}

impl<'a> Bk<'a> {
    fn new(x: &'a Ctx<'a>, from: usize, to: usize) -> Self { Bk { x, from, to, out: vec![], cur: None } }
    fn pos(&self) -> i8 { self.cur.as_ref().map_or(0, |t| t.dir as i8) }
    fn exit(&mut self, i: usize, px: f64, why: &str) {
        if let Some(mut t) = self.cur.take() {
            t.exit_i = i; t.exit_px = px; t.signal_i = i; t.reason = why.into();
            self.out.push(t);
        }
    }
    /// Target position `dir` at price px on bar i (close-out then entry). Entries only inside [from, to).
    fn go(&mut self, i: usize, dir: i8, px: f64, kind: &'static str, why: &str) {
        if i < self.from || dir == self.pos() { return; }
        if self.cur.is_some() { self.exit(i, px, why); }
        if dir != 0 && i < self.to {
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
    let mut b = Bk::new(x, from, to);
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
    if from > to || to >= x.r.len() { return None; }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(0., |d| d.def));
    let u = |k: &str| g(k).round().max(0.) as usize;
    let n = x.r.len();
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let k_of = |nn: usize| if u("kind") == 1 { 2. / (nn as f64 + 1.) } else { 1. / (nn as f64 + 1.) };
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
            let mut b = Bk::new(x, from, to);
            for i in from.max(sn.max(fnn))..=to {
                let (su, sd_) = (c[i] > c[i - sn], c[i] < c[i - sn]);
                let (fu, fd) = (c[i] > c[i - fnn], c[i] < c[i - fnn]);
                if b.pos() == 1 && (fd || sd_) { b.at_close(i, 0, "a trendline turned down"); }
                else if b.pos() == 0 && su && fu { b.at_close(i, 1, "both trendlines up"); }
            }
            b.done()
        }
        "kfsc_macd" => {
            let (fe, se) = (expo(c, k_of(u("fast").max(1))), expo(c, k_of(u("slow").max(1))));
            let m: Vec<f64> = (0..n).map(|i| fe[i] - se[i]).collect();
            let s = expo(&m, k_of(u("signal").max(1)));
            let mut b = Bk::new(x, from, to);
            for i in from.max(1)..=to {
                if m[i] > s[i] && m[i - 1] <= s[i - 1] { b.at_close(i, 1, "MACD crossed above signal"); }
                else if m[i] < s[i] && m[i - 1] >= s[i - 1] { b.at_close(i, -1, "MACD crossed below signal"); }
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
            let mn = u("mom").max(1);
            let a = k_of(u("smooth").max(1));
            let m: Vec<f64> = (0..n).map(|i| if i >= mn { c[i] - c[i - mn] } else { f64::NAN }).collect();
            let e2 = expo(&expo(&m, a), a);
            let mut b = Bk::new(x, from, to);
            for i in from.max(1)..=to {
                if !(e2[i].is_finite() && e2[i - 1].is_finite()) { continue; }
                if e2[i] > e2[i - 1] { b.at_close(i, 1, "double-smoothed momentum turned up"); }
                else if e2[i] < e2[i - 1] { b.at_close(i, -1, "double-smoothed momentum turned down"); }
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
