//! Dennis Meyers, intraday papers (Meyers Analytics, 1999-2003; research share "Trading Knowledge III/Dennis Meyers
//! Publications/"). Book id `dmpub`. Lane K2 (23 Nov 2026, R4 #340): four S&P-futures intraday systems coded from the
//! printed rules on the NIFTY 50 / NIFTYBANK futures 5-minute (EPFFT also 1-minute) bars. Same curve-following frame in all:
//! the curve (4th-order least-squares polynomial / fading-memory polynomial / end-point FFT sum) turns up by pctup from its
//! low while short -> buy at the market; turns down by pctdn from its high while long -> sell (reverse) at the market; every
//! position is closed at the day's close. Parameters are the optimum printed in each paper's Figure 1 (one walk-forward test window).
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "dmpub";
const IDS: [&str; 2] = ["dmpub_poly4", "dmpub_fm4"];
const HELD: [&str; 2] = ["dmpub_epfft5", "dmpub_epfft1"];
#[allow(dead_code)]
const HELD_WHY: &str = "Held on Indian data (R4 #377, lead review 23 Nov 2026): the End Point FFT rules use fixed S&P index POINT thresholds (pntup/pntdn 1.0/3.0, pup/pdn 2.75/1.25 points) that mean different things on NIFTY (about 25,000) than on the S&P (about 1,400); rescaling them would be invented. The rule is coded in this file but not published; it needs S&P 500 e-mini minute data.";

const NOTE: &str = "A|Adapted (R4 #340, lane K2 23 Nov 2026), UNVERIFIED: the papers trade S&P 500 / E-mini futures (Sep/99, Mar/00, Sep/02) in dollars and print their results for that contract; the rule runs here on the NIFTY 50 / NIFTYBANK FUTURES 1-minute series (lane DI; nearest-expiry continuous, NOT back-adjusted; days with a blank volume are absent), session 09:15-15:30 IST, bars aligned to the session open (09:15 ...; the 15:30 auction minute dropped). The book's walk-forward results are for a different contract and period and are not reproduced. Parameters = the paper's Figure 1 optimum (a single walk-forward test window; no optimiser is run here); percent parameters carry over unchanged, point parameters (EPFFT pntup/pntdn) are used as printed in index points with no rescaling. Unstated choices, counted, one numbered list per card. dmpub_poly4: (1) a signal on the day's last bar is not taken; (2) at each new day the curve's high and low trackers restart at that bar's curve value; (3) while flat both trackers are followed, so the first move of either size opens a position (long or short); (4) after an entry the tracker of the held side starts at that bar's curve value. Honest total: 4 card-specific choices. dmpub_fm4: (1) a signal on the day's last bar is not taken; (2) at each new day the curve's high and low trackers restart at that bar's curve value; (3) while flat both trackers are followed, so the first move of either size opens a position (long or short); (4) after an entry the tracker of the held side starts at that bar's curve value. Honest total: 4 card-specific choices. Totals: 4 card-specific choices for dmpub_poly4, 4 for dmpub_fm4 (audit count 4 each; the continuous series and the warm-up moved to engine defaults per #404). Engine-wide defaults / declared adaptations (labelled, not counted, R4 #404): one continuous bar series across days for the curve (the papers' windows span days; the nearest-expiry roll gap is inside it); the standard warm-up (no signal before the curve's warm-up: the 60 closes of the polynomial fit, 150 bars for the Fading Memory so the start-up transient 0.91^150 is gone); 'at the market' fills at the next bar's open; 'exit on the close' is the last bar's close, as printed; the lane's nearer-to-open minute-order tie-break; NIFTY 50 / NIFTYBANK futures replace the S&P contract; the 1-minute futures series (nearest-expiry continuous, not back-adjusted, days with a blank volume absent) is aggregated to 5-minute bars aligned to the 09:15 session open with the 15:30 auction minute dropped; no costs. EPFFT zeroes the bins whose magnitude is below thres*max over all 512 bins (DC included). No costs.";
const NOTE_EP: &str = "A|Adapted (R4 #340, lane K2 23 Nov 2026), UNVERIFIED: the papers trade S&P 500 / E-mini futures (Sep/99, Mar/00, Sep/02) in dollars and print their results for that contract; the rule runs here on the NIFTY 50 / NIFTYBANK FUTURES 1-minute series (lane DI; nearest-expiry continuous, NOT back-adjusted; days with a blank volume are absent), session 09:15-15:30 IST, bars aligned to the session open (09:15 ...; the 15:30 auction minute dropped). The book's walk-forward results are for a different contract and period and are not reproduced. Parameters = the paper's Figure 1 optimum (a single walk-forward test window; no optimiser is run here); percent parameters carry over unchanged, the printed point parameters (EPFFT pntup/pntdn, pup/pdn) are S&P index points: they follow ONE stated price-relative scaling rule (R4 #380, re-opened 24 Nov 2026, lane K4): threshold = printed points x the bar's close / ref, ref = the S&P level of the paper's own test window (1,400 for the 5-minute paper, S&P Mar/00, 30 Nov-30 Dec 1999; 1,000 for the 1-minute paper, ES Sep/02, 12 Jun-12 Jul 2002), counted as ONE labelled detail. Readings of what the paper leaves open, 4, all labelled: (1) one continuous bar series across days for the curve (the papers' windows span days; the nearest-expiry roll gap is inside it); (2) 'at the market' = the next bar's open, a signal on the day's last bar is not taken, 'exit on the close' = the last bar's close (the 1-minute paper: one minute before the close = the close of the second-to-last bar); (3) at each new day the curve's high/low trackers restart at that bar and, while flat, both are tracked, so the first move of either size opens a position; after an entry the tracker of the held side starts at that bar's curve value; (4) the point-threshold scaling rule above. Vault convention, not an unstated rule detail (as 'bars before the third are skipped' on other cards): no signal before the curve's warm-up (512 bars EPFFT); EPFFT zeroes the bins whose magnitude is below thres*max over all 512 bins (DC included). No costs.";

pub fn catalogue() -> Vec<StrategyDef> {
    let mk = |id: &'static str, name: &'static str, rules: &'static [&'static str], notes: &'static str| StrategyDef { id, name, group: "Meyers intraday papers", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None };
    let mut v = vec![
        mk(IDS[0], "4th Order Polynomial System (5-min futures)", &["F|Pf = least-squares 4th-degree polynomial through the last T=60 closes, forecast for the next bar (p. 1-2)",
            "E|Buy at the market when Pf has moved up by more than pctup=0.90% from its lowest value recorded while short; sell (reverse) when it has moved down by more than pctdn=0.75% from its highest while long (p. 2)",
            "X|Exit all trades at the close of each day (market on close); no overnight position", NOTE],
            "Paper: '4th Order Polynomial System on S&P500 Futures 5min Bars' (1999), 11 pp. Figure 1 optimum T=60, pctup 0.90%, pctdn 0.75% (S&P Sep/99, 7 Jul - 5 Aug 1999). Printed results are S&P dollar results for that contract and are not compared."),
        mk(IDS[1], "Fading Memory 4th Order Polynomial System (5-min futures)", &["F|X4est(t+1) = fading-memory 4th-degree polynomial one bar ahead, beta=0.91 (Meixner solution, recursion b0=(1-a)b0[1]+x, bk=(1-a)bk[1]+b(k-1)-b(k-1)[1], X4est=a*sum bk, a=1-beta), pp. 1-3",
            "E|Buy at the market when X4est has moved up by more than pctup=1.10% from its lowest while short; sell (reverse) when it has moved down by more than pctdn=0.60% from its highest while long",
            "X|Exit all trades at the close of each day", NOTE],
            "Paper: 'The Fading Memory 4th Order Polynomial' (2000), 10 pp. Figure 1 optimum beta 0.910, pctup 1.10%, pctdn 0.60% (S&P Mar/00, 11 Jan - 11 Feb 2000)."),
        mk(HELD[0], "End Point FFT System (5-min futures)", &["F|Each bar: take ln of the last 512 closes, end-flatten (a=x1, b=(xn-x1)/(n-1)), FFT, zero the bins below 0.15*max magnitude, inverse FFT, add back the trend and exp; y = last point, y1 = the point before; dsumEP += y-y1 (pp. 2-3)",
            "E|Buy at the market when dsumEP has moved up by more than pntup=1.0 points from its lowest while short; sell (reverse) when down by more than pntdn=3.0 points from its highest while long",
            "X|Exit on the close", NOTE_EP],
            "Paper: 'The End Point Fast Fourier Transform' (2000), 9 pp. Figure 1 optimum pntup 1.0, pntdn 3.0 (S&P Mar/00 5-min, 30 Nov - 30 Dec 1999). Points are scaled by close / 1,400 (R4 #380)."),
        mk(HELD[1], "End Point FFT System (1-min futures, E-mini paper)", &["F|Same EPFFT as the 5-minute paper on 1-minute bars: 512 bars, thres=0.13 of the maximum magnitude; sumEP += ep-ep1",
            "E|Buy at the market when sumEP has moved up by more than pup=2.75 points from its lowest while short; sell (reverse) when down by more than pdn=1.25 points from its highest while long",
            "X|Close the position 1 minute before the close (no overnight)", NOTE_EP],
            "Paper: 'Trading The E-Mini Using The End Point Fast Fourier Transform' (2003), 16 pp. Figure 4 optimum thres 0.13, pntup 2.75, pntdn 1.25 (ES Sep/02 1-min, 12 Jun - 12 Jul 2002)."),
    ];
    v
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (IDS.contains(&id) || HELD.contains(&id)).then_some(("dmpub", "Meyers intraday curve systems", match id { "dmpub_poly4" => "4th-order polynomial", "dmpub_fm4" => "Fading memory 4th order", "dmpub_epfft5" => "End point FFT 5-min", _ => "End point FFT 1-min" }))
}
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    match id {
        "dmpub_poly4" => Some(vec![p("bar", "Bar size (minutes; 5 per the paper)", 5., 1., 30., 1.), p("t", "T, closes in the least-squares fit", 60., 10., 200., 1.), p("pctup", "pctup, % rise of Pf from its low", 0.90, 0.05, 3., 0.05), p("pctdn", "pctdn, % fall of Pf from its high", 0.75, 0.05, 3., 0.05)]),
        "dmpub_fm4" => Some(vec![p("bar", "Bar size (minutes; 5 per the paper)", 5., 1., 30., 1.), p("beta", "beta (memory)", 0.91, 0.5, 0.99, 0.01), p("pctup", "pctup, % rise of X4est from its low", 1.10, 0.05, 3., 0.05), p("pctdn", "pctdn, % fall of X4est from its high", 0.60, 0.05, 3., 0.05)]),
        "dmpub_epfft5" => Some(vec![p("bar", "Bar size (minutes; 5 per the paper)", 5., 1., 30., 1.), p("thres", "thres (fraction of the maximum magnitude)", 0.15, 0.01, 0.9, 0.01), p("pup", "pntup (S&P points at ref, scaled by close / ref)", 1.0, 0.1, 20., 0.05), p("pdn", "pntdn (S&P points at ref, scaled by close / ref)", 3.0, 0.1, 20., 0.05), p("ref", "S&P level of the paper's test window (Dec 1999)", 1400., 100., 5000., 100.)]),
        "dmpub_epfft1" => Some(vec![p("bar", "Bar size (minutes; 1 per the paper)", 1., 1., 30., 1.), p("thres", "thres (fraction of the maximum magnitude)", 0.13, 0.01, 0.9, 0.01), p("pup", "pup (S&P points at ref, scaled by close / ref)", 2.75, 0.1, 20., 0.05), p("pdn", "pdn (S&P points at ref, scaled by close / ref)", 1.25, 0.1, 20., 0.05), p("ref", "S&P level of the paper's test window (Jun-Jul 2002)", 1000., 100., 5000., 100.)]),
        _ => None,
    }
}
pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let defs = params(id)?;
    if to >= x.c.len() || from > to { return None; }
    let g = |k: &str| *p.get(k).unwrap_or(&defs.iter().find(|d| d.key == k).unwrap().def);
    let n = g("bar").round().max(1.0) as u16;
    let (bars, one_before) = bars(x, from, to, n, id == "dmpub_epfft1");
    Some(match id {
        "dmpub_poly4" => {
            let t = g("t").round() as usize;
            let w = ls_weights(t, 4);
            let f: Vec<f64> = (0..bars.len()).map(|k| if k + 1 >= t { (0..t).map(|j| w[j] * bars[k + 1 - t + j].c).sum() } else { f64::NAN }).collect();
            curve_system(&bars, &f, t - 1, Thr::Pct(g("pctup"), g("pctdn")), one_before)
        }
        "dmpub_fm4" => {
            let a = 1.0 - g("beta");
            let mut b = [0f64; 5];
            let mut f = vec![f64::NAN; bars.len()];
            for k in 0..bars.len() {
                let x0 = bars[k].c;
                if k == 0 { b = [0.0; 5]; b[0] = x0 / a; } else {
                    let bp = b;
                    b[0] = (1.0 - a) * bp[0] + x0;
                    for j in 1..5 { b[j] = (1.0 - a) * bp[j] + b[j - 1] - bp[j - 1]; }
                }
                f[k] = a * b.iter().sum::<f64>();
            }
            curve_system(&bars, &f, 149, Thr::Pct(g("pctup"), g("pctdn")), one_before)
        }
        _ => {
            let thres = g("thres");
            let mut s = 0.0;
            let mut f = vec![f64::NAN; bars.len()];
            for k in 511..bars.len() {
                let (y, y1) = epfft(&bars[k - 511..=k], thres);
                s += y - y1;
                f[k] = s; // sumEP starts at 0 on the first full window
            }
            curve_system(&bars, &f, 511, Thr::Pts(g("pup"), g("pdn"), g("ref")), one_before)
        }
    })
}

#[derive(Clone, Copy)]
struct B { i: usize, o: f64, c: f64, last: bool, pen: bool }
#[derive(Clone, Copy)]
enum Thr { Pct(f64, f64), Pts(f64, f64, f64) }

/// Continuous bar series over rows from..=to; `pen` marks the second-to-last bar of a day.
fn bars(x: &Ctx, from: usize, to: usize, n: u16, _one: bool) -> (Vec<B>, bool) {
    let mut b: Vec<B> = vec![];
    if let Some(f) = x.fut {
        for i in from..=to {
            let r = &x.r[i];
            let Some((m, _)) = f.fut_day(&r.date) else { continue };
            let (open, close) = (crate::intraday::session_open(&r.ticker, &r.date), crate::intraday_ep::session_close(&r.ticker));
            let day = crate::intraday_ep::nbars(&m, open, n, close);
            let len = day.len();
            for (k, z) in day.iter().enumerate() { b.push(B { i, o: z.o, c: z.c, last: k + 1 == len, pen: k + 2 == len }); }
        }
    }
    (b, _one)
}

/// Curve-following stop-and-reverse (see NOTE readings 2-4). `warm` = first bar index allowed to signal.
fn curve_system(b: &[B], f: &[f64], warm: usize, th: Thr, exit_pen: bool) -> Vec<Trade> {
    let mut out = vec![];
    let (mut pos, mut ep, mut ei) = (0.0f64, 0.0f64, 0usize);
    let (mut lo, mut hi) = (f64::NAN, f64::NAN);
    let up = |lo: f64, c: f64, px: f64| match th { Thr::Pct(u, _) => c > lo * (1.0 + u / 100.0), Thr::Pts(u, _, rf) => c > lo + u * px / rf };
    let dn = |hi: f64, c: f64, px: f64| match th { Thr::Pct(_, d) => c < hi * (1.0 - d / 100.0), Thr::Pts(_, d, rf) => c < hi - d * px / rf };
    let mut pend: Option<f64> = None; // order for the next bar's open: +1 buy, -1 sell
    for k in 0..b.len() {
        let z = &b[k];
        let new_day = k == 0 || b[k - 1].i != z.i;
        if let Some(d) = pend.take() {
            if !new_day {
                if pos != 0.0 { out.push(crate::intraday::day_trade(pos, ei, ep, z.i, z.o, "reversed by the opposite signal", "limit", false)); }
                pos = d; ep = z.o; ei = z.i;
            }
        }
        if new_day { lo = f64::NAN; hi = f64::NAN; if pos != 0.0 { pos = 0.0; } }
        // exit on the close of the day (the 1-minute paper: the second-to-last bar's close)
        if pos != 0.0 && ((exit_pen && z.pen) || (!exit_pen && z.last)) { out.push(crate::intraday::day_trade(pos, ei, ep, z.i, z.c, "exit on the close", "limit", false)); pos = 0.0; }
        let c = f[k];
        if !c.is_finite() || k < warm { continue; }
        if lo.is_nan() { lo = c; hi = c; if pos > 0.0 { lo = f64::INFINITY; } if pos < 0.0 { hi = f64::NEG_INFINITY; } }
        if z.last || (exit_pen && z.pen) { continue; }
        if pos <= 0.0 { lo = lo.min(c); }
        if pos >= 0.0 { hi = hi.max(c); }
        if pos <= 0.0 && up(lo, c, z.c) { pend = Some(1.0); lo = f64::INFINITY; hi = c; }
        else if pos >= 0.0 && dn(hi, c, z.c) { pend = Some(-1.0); hi = f64::NEG_INFINITY; lo = c; }
    }
    out
}

/// Weights w[j] with forecast(T+1) = sum w[j]*x[t=j+1], least-squares polynomial of degree d through t=1..T.
fn ls_weights(t: usize, d: usize) -> Vec<f64> {
    let m = d + 1;
    let u = |k: f64| (k - (t as f64 + 1.0) / 2.0) / (t as f64 / 2.0);
    let mut a = vec![vec![0f64; m]; m];
    for i in 0..m { for j in 0..m { a[i][j] = (1..=t).map(|k| u(k as f64).powi((i + j) as i32)).sum(); } }
    // solve A s = e where e_j = u(T+1)^j, then w_k = sum_j s_j u(k)^j
    let e: Vec<f64> = (0..m).map(|j| u(t as f64 + 1.0).powi(j as i32)).collect();
    let s = solve(a, e);
    (1..=t).map(|k| (0..m).map(|j| s[j] * u(k as f64).powi(j as i32)).sum()).collect()
}
fn solve(mut a: Vec<Vec<f64>>, mut b: Vec<f64>) -> Vec<f64> {
    let n = b.len();
    for c in 0..n {
        let p = (c..n).max_by(|&i, &j| a[i][c].abs().partial_cmp(&a[j][c].abs()).unwrap()).unwrap();
        a.swap(c, p); b.swap(c, p);
        for r in c + 1..n { let f = a[r][c] / a[c][c]; for k in c..n { a[r][k] -= f * a[c][k]; } b[r] -= f * b[c]; }
    }
    let mut s = vec![0.0; n];
    for r in (0..n).rev() { s[r] = (b[r] - (r + 1..n).map(|k| a[r][k] * s[k]).sum::<f64>()) / a[r][r]; }
    s
}

fn fft(re: &mut [f64], im: &mut [f64], inv: bool) {
    let n = re.len();
    let mut j = 0;
    for i in 1..n { let mut bit = n >> 1; while j & bit != 0 { j ^= bit; bit >>= 1; } j ^= bit; if i < j { re.swap(i, j); im.swap(i, j); } }
    let mut len = 2;
    while len <= n {
        let ang = 2.0 * std::f64::consts::PI / len as f64 * if inv { 1.0 } else { -1.0 };
        for s in (0..n).step_by(len) {
            for k in 0..len / 2 {
                let (wr, wi) = ((ang * k as f64).cos(), (ang * k as f64).sin());
                let (a, b) = (s + k, s + k + len / 2);
                let (tr, ti) = (re[b] * wr - im[b] * wi, re[b] * wi + im[b] * wr);
                re[b] = re[a] - tr; im[b] = im[a] - ti; re[a] += tr; im[a] += ti;
            }
        }
        len <<= 1;
    }
    if inv { for v in re.iter_mut() { *v /= n as f64; } for v in im.iter_mut() { *v /= n as f64; } }
}
/// End-point filtered value of the last bar and of the bar before it (512 closes).
fn epfft(w: &[B], thres: f64) -> (f64, f64) {
    let n = w.len();
    let lx: Vec<f64> = w.iter().map(|z| z.c.ln()).collect();
    let (a, bb) = (lx[0], (lx[n - 1] - lx[0]) / (n as f64 - 1.0));
    let mut re: Vec<f64> = (0..n).map(|i| lx[i] - (a + bb * i as f64)).collect();
    let mut im = vec![0.0; n];
    fft(&mut re, &mut im, false);
    let mx = (0..n).map(|i| re[i].hypot(im[i])).fold(0.0, f64::max);
    for i in 0..n { if re[i].hypot(im[i]) < thres * mx { re[i] = 0.0; im[i] = 0.0; } }
    fft(&mut re, &mut im, true);
    let back = |i: usize| (re[i] + a + bb * i as f64).exp();
    (back(n - 1), back(n - 2))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn defaults_are_the_printed_figures() {
        let d = |id: &str| params(id).unwrap().iter().map(|p| p.def).collect::<Vec<_>>();
        assert_eq!(d("dmpub_poly4"), vec![5., 60., 0.90, 0.75]);
        assert_eq!(d("dmpub_fm4"), vec![5., 0.91, 1.10, 0.60]);
        assert_eq!(d("dmpub_epfft5"), vec![5., 0.15, 1.0, 3.0, 1400.]);
        assert_eq!(d("dmpub_epfft1"), vec![1., 0.13, 2.75, 1.25, 1000.]);
    }
    #[test]
    fn ls_weights_forecast_a_cubic_exactly() {
        let w = ls_weights(30, 4);
        let y: Vec<f64> = (1..=30).map(|t| { let t = t as f64; 5.0 + 0.3 * t - 0.02 * t * t + 0.001 * t * t * t }).collect();
        let f: f64 = w.iter().zip(&y).map(|(a, b)| a * b).sum();
        let t = 31.0f64;
        assert!((f - (5.0 + 0.3 * t - 0.02 * t * t + 0.001 * t * t * t)).abs() < 1e-6);
    }
    #[test]
    fn fft_roundtrip() {
        let mut re: Vec<f64> = (0..16).map(|i| (i as f64).sin()).collect();
        let o = re.clone();
        let mut im = vec![0.0; 16];
        fft(&mut re, &mut im, false); fft(&mut re, &mut im, true);
        assert!(re.iter().zip(&o).all(|(a, b)| (a - b).abs() < 1e-12));
    }
}
