//! F. E. James, Jr., "Monthly Moving Averages - An Effective Investment Tool?", Journal of Financial and
//! Quantitative Analysis 3(3), Sep 1968, pp.315-326. The eight models of Table 2 (p.325): month-end
//! close vs a 7-month simple average (+-2%) or an exponentially smoothed average (alpha .1/.3/.4,
//! +-5%), long-only or long-and-short. The paper tests NYSE stocks 1926-1960 (CRSP); the vault runs
//! the rules on indices / NSE basket stocks (instrument swap). Page refs are the journal pages
//! (PDF page = journal page - 313). Sources: library/fejmma/.
use crate::books::apta::bars;
use crate::connors::{claim, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "fejmma";

const MECH: &str = "E|\"the monthly closing price for each month after the 7th month was compared to the Moving Average value. If the closing price ever rose above the Moving Average value by 2 percent or more (if it was 102 percent of the Moving Average value), this was taken as a buy signal, and if this security was not held in a \"long\" condition at the time, a simulated purchase was made at the current price\" (p.322)";
const MECH2: &str = "X|\"if the closing price ever fell below the Moving Average by 2 percent or more (if it was 98 percent of the Moving Average value), this was taken as a sell signal, and if this security was not held in a \"short\" position at the time, a simulated sale was made\"; \"This position was held until the price movement reversed itself by an amount sufficient to trigger a reverse indicator, at which time the position was closed out, and the funds ... utilized to initiate a position in the opposite direction\" (p.322)";
const SEED: &str = "F|\"an initial period of time was used to set initial values of Moving Averages (six months were used for exponentially smoothed averages, seven months for unweighted averages)\"; month-end close = \"the last price on the last business day of the month\" (p.317, 322)";

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    let d = |id, name, dir, status, rules, notes, claim| StrategyDef {
        id, name, group: "IV · Results (Table 2)", session: 0, dir, status, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim,
    };
    vec![
        d("fejmma_sma7", "7-month unweighted moving average, +-2% (Table 2 lines 1, 8)", "Long/Short", Live,
            &["F|\"The Unweighted Moving Average models (Table 2, Lines 1 and 8) all incorporated MAX values of 1.02 and MIN values of .98\" (p.322); \"all unweighted Moving Averages utilized were constructed for an observational period of 7 months\" (p.320)",
              MECH, MECH2, SEED,
              "F|[Read] Lines 1/8 do not say long-only; the p.322 walk-through for exactly these models sells into a \"short\" position, so the default is long-and-short. p.short=0 = sell to cash (non-default)."],
            "Table 2 reports the mean $ difference (managed - buy and hold, $100 start) across NYSE stocks: line 1 (dividends reinvested) -$771 (n=233, t=-1.815); line 8 (price only) +$136 (t=0.887). Not comparable to one index run.",
            claim("", "", "D = -$771 (with dividends) / +$136 (price only) vs buy & hold per $100", "", "233 NYSE stocks 1926-1960")),
        d("fejmma_es10_ls", "Exponential smoothing a=.1, +-5%, long and short (lines 2, 7)", "Long/Short", Live,
            &["F|\"F(i) = F(i-1) + a(X(i) - F(i-1))\", X = monthly closing price (p.321); a = .1, MAX 1.05, MIN .95, \"LONG AND SHORT\" (Table 2, p.325)",
              "E|Month-end close >= 1.05 x F and not long -> buy (covering any short) at that close (p.322)",
              "X|Month-end close <= .95 x F and not short -> sell and sell short at that close (p.322)", SEED,
              "F|[Read, B] F starts at the first month-end close and is updated through month 6; signals from month 7."],
            "Line 2 (dividends): D = -$1,059 (n=798, t=-5.728); line 7 (price only): +$92 (t=0.906).",
            claim("", "", "D = -$1,059 (dividends) / +$92 (price only) per $100", "", "798 NYSE stocks 1926-1960")),
        d("fejmma_es10_long", "Exponential smoothing a=.1, +-5%, long only (line 3)", "Long", Live,
            &["F|a = .1, MAX 1.05, MIN .95, \"LONG ONLY\" (Table 2 line 3, p.325)", "E|Month-end close >= 1.05 x F and flat -> buy at that close (p.322)",
              "X|Month-end close <= .95 x F while long -> sell to cash (\"an investor who could either take a position in the security, or hold cash\", p.322)", SEED],
            "Line 3: D = -$409 (n=1,376, t=-5.811).", claim("", "", "D = -$409 per $100", "", "1,376 NYSE stocks 1926-1960")),
        d("fejmma_es30_long", "Exponential smoothing a=.3, +-5%, long only (line 4)", "Long", Live,
            &["F|a = .3, MAX 1.05, MIN .95, LONG ONLY (Table 2 line 4, p.325)", "E|Month-end close >= 1.05 x F and flat -> buy", "X|Month-end close <= .95 x F while long -> sell to cash", SEED],
            "Line 4: D = -$422 (n=1,376, t=-4.988).", claim("", "", "D = -$422 per $100", "", "1,376 NYSE stocks 1926-1960")),
        d("fejmma_es40_long", "Exponential smoothing a=.4, +-5%, long only (lines 5, 6)", "Long", Live,
            &["F|a = .4, MAX 1.05, MIN .95, LONG ONLY (Table 2 lines 5-6, p.325)", "E|Month-end close >= 1.05 x F and flat -> buy", "X|Month-end close <= .95 x F while long -> sell to cash", SEED],
            "Line 5 (dividends): D = -$145 (n=232, t=-0.309); line 6 (price only, PR2): +$337 (t=1.788), the best result in the paper.",
            claim("", "", "D = -$145 (dividends) / +$337 (price only) per $100", "", "232 NYSE stocks 1926-1960")),
        d("fejmma_barnes_5pct", "Barnes 5% moving-average penetration", "Long/Short", NotTestable("the moving-average length is not given for this quoted rule"),
            &["F|\"When a stock's price curve penetrates its moving average line on the downside by at least 5 percent, this is a sell signal; when it penetrates its moving average line on the upside by at least 5 percent, this is a buy signal\" (p.319)"],
            "Quoted recommendation (Barnes); the paper's own models above use 7 months / alpha.", None),
        d("fejmma_merritt_12m", "Merritt 12-month Dow average", "Long", NotTestable("a trend definition, not an order: no sell rule is stated (\"The trend of the market is assumed to be up when the Dow-Jones Industrials are above the 12-month moving average and the line is moving upward\")"),
            &["F|\"a 12-month moving average of the Dow-Jones Industrials, plotted monthly\" (p.318)"], "Quoted from Merritt (1957).", None),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "fejmma_sma7" => ("fejmma_mma", "James monthly moving averages", "7-month SMA +-2%"),
        "fejmma_es10_ls" => ("fejmma_mma", "James monthly moving averages", "ES .1 +-5% long/short"),
        "fejmma_es10_long" => ("fejmma_mma", "James monthly moving averages", "ES .1 +-5% long"),
        "fejmma_es30_long" => ("fejmma_mma", "James monthly moving averages", "ES .3 +-5% long"),
        "fejmma_es40_long" => ("fejmma_mma", "James monthly moving averages", "ES .4 +-5% long"),
        "fejmma_barnes_5pct" | "fejmma_merritt_12m" => ("fejmma_quoted", "Rules quoted by James", "not testable"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let es = |a: f64, short: f64| vec![p("alpha", "Smoothing constant", a, 0.05, 0.9, 0.05), p("max", "Buy at close >= MAX x average", 1.05, 1.0, 1.2, 0.01),
        p("min", "Sell at close <= MIN x average", 0.95, 0.8, 1.0, 0.01), p("short", "Sell signals go short (1) or to cash (0)", short, 0.0, 1.0, 1.0)];
    Some(match id {
        "fejmma_sma7" => vec![p("n", "Months in the average", 7.0, 2.0, 24.0, 1.0), p("max", "Buy at close >= MAX x average", 1.02, 1.0, 1.2, 0.01),
            p("min", "Sell at close <= MIN x average", 0.98, 0.8, 1.0, 0.01), p("short", "Sell signals go short (1) or to cash (0)", 1.0, 0.0, 1.0, 1.0)],
        "fejmma_es10_ls" => es(0.1, 1.0),
        "fejmma_es10_long" => es(0.1, 0.0),
        "fejmma_es30_long" => es(0.3, 0.0),
        "fejmma_es40_long" => es(0.4, 0.0),
        _ => return None,
    })
}

fn mk(dir: f64, x: &Ctx, i: usize) -> Trade {
    Trade { dir, legs: vec![Leg { i, px: x.c[i], w: 1.0 }], exit_i: i, exit_px: x.c[i], reason: String::new(),
        entry_kind: "close", signal_i: i, open: false, exits: vec![] }
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    let n = x.r.len();
    if n == 0 || from > to || to >= n {
        return Some(vec![]);
    }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(0.0, |d| d.def));
    let (hi, lo, short) = (g("max"), g("min"), g("short") >= 0.5);
    let me = bars(x, 2);
    let mc: Vec<f64> = me.iter().map(|&i| x.c[i]).collect();
    // average at each month end, and the first month index whose close is compared
    let (avg, first): (Vec<f64>, usize) = if id == "fejmma_sma7" {
        let k = g("n").round().max(1.0) as usize;
        let a = (0..mc.len()).map(|j| if j + 1 >= k { mc[j + 1 - k..=j].iter().sum::<f64>() / k as f64 } else { f64::NAN }).collect();
        (a, k) // "each month after the 7th month"
    } else {
        let al = g("alpha");
        let mut f = f64::NAN;
        let a = mc.iter().map(|&v| { f = if f.is_finite() { f + al * (v - f) } else { v }; f }).collect();
        (a, 6) // six months to set the initial value
    };
    // pos: rule position (+1 / -1 / 0); trades only for positions opened inside [from, to)
    let (mut out, mut cur, mut pos) = (vec![], None::<Trade>, 0i8);
    for j in first..me.len() {
        let i = me[j];
        if i > to {
            break;
        }
        let want = if mc[j] >= hi * avg[j] && pos != 1 {
            1
        } else if mc[j] <= lo * avg[j] && pos == 1 && !short {
            0
        } else if mc[j] <= lo * avg[j] && pos != -1 && short {
            -1
        } else {
            continue;
        };
        if let Some(mut t) = cur.take() {
            (t.exit_i, t.exit_px, t.signal_i) = (i, x.c[i], i);
            t.reason = if want == 1 { format!("month-end close >= {hi} x average") } else { format!("month-end close <= {lo} x average") };
            out.push(t);
        }
        pos = want;
        if want != 0 && i >= from && i < to {
            cur = Some(mk(want as f64, x, i));
        }
    }
    if let Some(mut t) = cur {
        (t.exit_i, t.exit_px, t.signal_i, t.open) = (to, x.c[to], to, true);
        t.reason = "open".into();
        out.push(t);
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;

    #[test]
    fn sma7_two_percent_band_by_hand() {
        // one bar per month; month 8 closes at 103: SMA7 = (6 x 100 + 103) / 7 = 100.43, 103 >= 1.02 x 100.43 -> long.
        // month 10 closes at 97: SMA7 = (4 x 100 + 103 + 101 + 97) / 7 = 100.14, 97 <= .98 x 100.14 = 98.14 -> reverse short.
        let cl = [100., 100., 100., 100., 100., 100., 100., 103., 101., 97., 97., 97.];
        let r: Vec<Row> = cl.iter().enumerate().map(|(k, &c)| Row { ticker: "T".into(), date: format!("2001-{:02}-28", k + 1),
            open: c, high: c, low: c, close: c, volume: None }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let t = run("fejmma_sma7", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert_eq!((t[0].dir, t[0].legs[0].i, t[0].exit_i), (1.0, 7, 9));
        assert_eq!((t[1].dir, t[1].legs[0].i, t[1].open), (-1.0, 9, true));
    }
}
