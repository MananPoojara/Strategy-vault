//! Jay Kaeppel, Seasonal Stock Market Trends (Wiley, 2009). Book tests every rule on the Dow (DJIA).
//! Every rule is a calendar mask: on trading day t the position (long x1, long x2, short, or cash) earns
//! the close(t-1) -> close(t) change, exactly how the book measures "invested in the Dow only on day X".
//! Consecutive days with the same position form one trade (enter at the close before the first day,
//! exit at the close of the last day). Cash interest (1% a year in UJBS/UHTS/UMDS/UTCM/UECS/KTI models)
//! is not modelled. Holiday rules run on US instruments only (2026-10-24 review); elections are calendar dates (India =
//! instrument swap only). Page refs are printed pages. Sources and checks: library/kpss/.
use crate::connors::{claim, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "kpss";
// Admitted 2026-10-24 (authors/kaeppel/admission/SOURCE-CONTRACT.md): complete calendar rules with independent US/India ledgers.
pub const REVIEWED: &[&str] = &["kpss_first5", "kpss_last5", "kpss_janbar", "kpss_jbs", "kpss_jaynewary", "kpss_ujbs", "kpss_hol_d1", "kpss_hol_d2", "kpss_hol_3x3", "kpss_hol_top24", "kpss_uhts", "kpss_xmas_week", "kpss_td2", "kpss_td1_5", "kpss_mid4", "kpss_worst14", "kpss_nonworst", "kpss_six", "kpss_monthly10", "kpss_santa", "kpss_m10_santa", "kpss_umds", "kpss_uumds", "kpss_dec_b", "kpss_dec_c", "kpss_middecade", "kpss_latedecade", "kpss_20yr", "kpss_intradecade", "kpss_c212", "kpss_c40w", "kpss_c53d", "kpss_ctcm1", "kpss_ctcm2", "kpss_utcm", "kpss_yr_post", "kpss_yr_mid", "kpss_yr_pre", "kpss_yr_elec", "kpss_yr_post_mid", "kpss_yr_pre_elec", "kpss_stoken15", "kpss_fav14", "kpss_fav21", "kpss_midterm", "kpss_pre_mar_jul", "kpss_uecs", "kpss_nov_may", "kpss_ndx_nov_jun", "kpss_jja", "kpss_sept", "kpss_ex_sept", "kpss_oct7", "kpss_kti_long", "kpss_lopl", "kpss_jusb"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }
/// Rules whose days are defined by US exchange holidays (closures): US instruments only.
pub const US_ONLY: &[&str] = &["kpss_xmas_week", "kpss_hol_d1", "kpss_hol_d2", "kpss_hol_3x3", "kpss_hol_top24", "kpss_uhts", "kpss_santa", "kpss_m10_santa", "kpss_umds", "kpss_uumds", "kpss_kti_long", "kpss_lopl", "kpss_jusb"];

// ------------------------------------------------------------------ calendar helpers

fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m as i64 + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d as i64 - 1;
    era * 146097 + yoe * 365 + yoe / 4 - yoe / 100 + doy - 719468
}
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (yoe + era * 400 + if m <= 2 { 1 } else { 0 }, m, d)
}
/// 0 = Monday (1970-01-01 was a Thursday)
fn weekday(z: i64) -> i64 {
    (z + 3).rem_euclid(7)
}
fn month_end(y: i64, m: u32) -> i64 {
    if m == 12 { days_from_civil(y + 1, 1, 1) - 1 } else { days_from_civil(y, m + 1, 1) - 1 }
}
/// n-th given weekday (0=Mon) of a month (n>=1)
fn nth_wd(y: i64, m: u32, wd: i64, n: i64) -> i64 {
    let f = days_from_civil(y, m, 1);
    f + (wd - weekday(f)).rem_euclid(7) + 7 * (n - 1)
}
fn last_wd(y: i64, m: u32, wd: i64) -> i64 {
    let e = month_end(y, m);
    e - (weekday(e) - wd).rem_euclid(7)
}
/// Easter Sunday (anonymous Gregorian algorithm)
fn easter(y: i64) -> i64 {
    let (a, b, c) = (y % 19, y / 100, y % 100);
    let (d, e) = (b / 4, b % 4);
    let f = (b + 8) / 25;
    let g = (b - f + 1) / 3;
    let h = (19 * a + b - d - g + 15) % 30;
    let (i, k) = (c / 4, c % 4);
    let l = (32 + 2 * e + 2 * i - h - k) % 7;
    let m = (a + 11 * h + 22 * l) / 451;
    let month = (h + l - 7 * m + 114) / 31;
    let day = (h + l - 7 * m + 114) % 31 + 1;
    days_from_civil(y, month as u32, day as u32)
}
/// Saturday -> Friday, Sunday -> Monday
fn observed(z: i64) -> i64 {
    match weekday(z) {
        5 => z - 1,
        6 => z + 1,
        _ => z,
    }
}
/// The book's eight major US market holidays (Table 3.1, p.50) of year y, in calendar order:
/// 0 Presidents', 1 Good Friday, 2 Memorial, 3 Independence, 4 Labor, 5 Thanksgiving, 6 Christmas,
/// 7 New Year's (of y+1, i.e. the holiday that closes year y).
fn holidays(y: i64) -> [i64; 8] {
    let ny = days_from_civil(y + 1, 1, 1);
    let ny = if weekday(ny) == 6 { ny + 1 } else { ny }; // Sunday -> Monday; Saturday: no weekday closure
    [
        if y >= 1971 { nth_wd(y, 2, 0, 3) } else { observed(days_from_civil(y, 2, 22)) },
        easter(y) - 2,
        if y >= 1971 { last_wd(y, 5, 0) } else { observed(days_from_civil(y, 5, 30)) },
        observed(days_from_civil(y, 7, 4)),
        nth_wd(y, 9, 0, 1),
        nth_wd(y, 11, 3, 4),
        observed(days_from_civil(y, 12, 25)),
        ny,
    ]
}
/// US midterm / presidential election day: the Tuesday after the first Monday of November
/// (p.206 says "first Tuesday of November"; option literal=1 uses that wording).
fn election_day(y: i64, literal: bool) -> i64 {
    if literal { nth_wd(y, 11, 1, 1) } else { nth_wd(y, 11, 0, 1) + 1 }
}
/// date six calendar months later (day clamped to month end)
fn plus_months(z: i64, k: i64) -> i64 {
    let (y, m, d) = civil_from_days(z);
    let t = y * 12 + m as i64 - 1 + k;
    let (ny, nm) = (t.div_euclid(12), (t.rem_euclid(12) + 1) as u32);
    let e = month_end(ny, nm);
    (days_from_civil(ny, nm, 1) + d as i64 - 1).min(e)
}

/// Per-bar calendar facts (from the bar dates only).
struct Cal {
    y: Vec<i64>,
    m: Vec<u32>,
    dn: Vec<i64>,
    /// k-th trading day of the month (1-based)
    tdf: Vec<i64>,
    /// -k = k-th trading day from the month end (-1 = last); 0 when the month is not complete in the data
    tdb: Vec<i64>,
    /// (holiday id 0..7, offset -3..-1 / 1..3) for each bar
    hol: Vec<Vec<(usize, i64)>>,
    /// bar is in the Santa Claus window (two days before Thanksgiving .. 5th trading day of January)
    santa: Vec<bool>,
    /// bar is after Christmas and on/before Dec 31 (p.63)
    xmas_week: Vec<bool>,
}

impl Cal {
    fn new(x: &Ctx) -> Cal {
        let n = x.r.len();
        let ymd = |s: &str| -> (i64, u32, u32) { (s[0..4].parse().unwrap_or(0), s[5..7].parse().unwrap_or(0), s[8..10].parse().unwrap_or(0)) };
        let mut c = Cal { y: vec![0; n], m: vec![0; n], dn: vec![0; n], tdf: vec![0; n], tdb: vec![0; n], hol: vec![vec![]; n], santa: vec![false; n], xmas_week: vec![false; n] };
        for i in 0..n {
            let (y, m, d) = ymd(&x.r[i].date);
            c.y[i] = y;
            c.m[i] = m;
            c.dn[i] = days_from_civil(y, m, d);
            c.tdf[i] = if i > 0 && (c.y[i - 1], c.m[i - 1]) == (y, m) { c.tdf[i - 1] + 1 } else { 1 };
        }
        // backward count only for months that are closed in the data (a later bar exists)
        let mut k = 0i64;
        for i in (0..n).rev() {
            if i + 1 < n && (c.y[i + 1], c.m[i + 1]) == (c.y[i], c.m[i]) {
                k = if k == 0 { 0 } else { k - 1 };
            } else {
                k = if i + 1 < n { -1 } else { 0 };
            }
            c.tdb[i] = k;
        }
        if n == 0 {
            return c;
        }
        let first = |day: i64| c.dn.partition_point(|&z| z < day); // first bar on/after day
        let after = |day: i64| c.dn.partition_point(|&z| z <= day); // first bar after day
        for y in c.y[0] - 1..=c.y[n - 1] {
            let h = holidays(y);
            for (hid, &hd) in h.iter().enumerate() {
                // A weekend holiday whose weekday stand-in has a bar was not closed (e.g. Memorial Day on Saturday
                // 30 May 1970, NYSE open on Friday 29 May): count the trading days around the weekend date itself.
                let act = match hid {
                    0 if y < 1971 => Some(days_from_civil(y, 2, 22)),
                    2 if y < 1971 => Some(days_from_civil(y, 5, 30)),
                    3 => Some(days_from_civil(y, 7, 4)),
                    6 => Some(days_from_civil(y, 12, 25)),
                    7 => Some(days_from_civil(y + 1, 1, 1)),
                    _ => None,
                };
                let day = match act { Some(a) if a != hd && after(hd) > first(hd) => a, _ => hd };
                let (p, q) = (first(day), after(day));
                // A holiday before the first bar or after the last bar has no known trading days around it.
                if p == 0 || p >= n {
                    continue;
                }
                for k in 1..=3usize {
                    if p >= k {
                        c.hol[p - k].push((hid, -(k as i64)));
                    }
                    if q + k - 1 < n {
                        c.hol[q + k - 1].push((hid, k as i64));
                    }
                }
            }
            // Santa: from bar -2 before Thanksgiving through the 5th trading day of January y+1
            let p = first(h[5]);
            let jan = first(days_from_civil(y + 1, 1, 1));
            let j5 = jan + 4;
            if p >= 2 && j5 < n && c.y[j5] == y + 1 && c.m[j5] == 1 {
                for b in p - 2..=j5 {
                    c.santa[b] = true;
                }
            }
            // Christmas -> New Year's Eve
            for b in after(h[6])..first(days_from_civil(y + 1, 1, 1)) {
                c.xmas_week[b] = true;
            }
        }
        c
    }
    /// calendar interval (a, b]: bar dates after a and on/before b
    fn mark(&self, w: &mut [f64], a: i64, b: i64, v: f64) {
        let s = self.dn.partition_point(|&z| z <= a);
        let e = self.dn.partition_point(|&z| z <= b);
        for k in s..e {
            w[k] = v;
        }
    }
    fn years(&self) -> std::ops::RangeInclusive<i64> {
        // from four years before the first bar: a decade or election window that began earlier still holds from the first bar
        if self.y.is_empty() { 1..=0 } else { self.y[0] - 4..=self.y[self.y.len() - 1] }
    }
}

fn dec31(y: i64) -> i64 {
    days_from_civil(y, 12, 31)
}

// ------------------------------------------------------------------ weights -> trades

/// w[t] = position held from close(t-1) to close(t): 1 long, 2 long with 2:1 leverage, -1 short, 0 cash.
fn trades(x: &Ctx, w: &[f64], from: usize, to: usize) -> Vec<Trade> {
    let n = x.r.len();
    let mut out = vec![];
    let mut t = from + 1;
    while t <= to {
        let v = w[t];
        if v == 0.0 {
            t += 1;
            continue;
        }
        let s = t;
        // [Fitted reading] 2:1 and short positions are daily-reset, as the leveraged / inverse index funds
        // the book trades (Table 1.1, p.11): one trade per day. Reproduces Table 2.4 year by year.
        while v == 1.0 && t < to && w[t + 1] == v {
            t += 1;
        }
        let e = s - 1;
        let open = v == 1.0 && t == to && (to + 1 >= n || w[to + 1] == v);
        let reason = if open { "open" } else if v != 1.0 { "daily-reset leveraged / inverse fund day · close" } else if w.get(t + 1).copied().unwrap_or(0.0) == 0.0 { "seasonal window ends · close" } else { "position size changes · close" }.to_string();
        out.push(Trade {
            dir: v.signum(),
            legs: vec![Leg { i: e, px: x.c[e], w: v.abs() }],
            exit_i: t,
            exit_px: x.c[t],
            reason,
            entry_kind: "close",
            signal_i: t,
            open,
            exits: vec![],
        });
        t += 1;
    }
    out
}

// ------------------------------------------------------------------ the book's building blocks

struct Ix<'a> {
    c: &'a Cal,
    close: &'a [f64],
    n: usize,
}

impl<'a> Ix<'a> {
    fn zeros(&self) -> Vec<f64> {
        vec![0.0; self.n]
    }
    fn tdm(&self, f: &[i64], b: &[i64]) -> Vec<f64> {
        (0..self.n).map(|i| if f.contains(&self.c.tdf[i]) || (self.c.tdb[i] != 0 && b.contains(&self.c.tdb[i])) { 1.0 } else { 0.0 }).collect()
    }
    fn hol(&self, offs: &[i64]) -> Vec<f64> {
        (0..self.n).map(|i| if self.c.hol[i].iter().any(|(_, o)| offs.contains(o)) { 1.0 } else { 0.0 }).collect()
    }
    /// January indicators of year y: (first five days up, last five days up, January up); None if incomplete
    fn january(&self, y: i64) -> Option<(bool, bool, bool)> {
        let c = self.c;
        let s = c.dn.partition_point(|&z| z < days_from_civil(y, 1, 1));
        let e = c.dn.partition_point(|&z| z <= days_from_civil(y, 1, 31));
        if s == 0 || e <= s + 5 || e >= self.n {
            return None; // need the prior December close, 6+ January bars and a February bar
        }
        let (prev, last) = (s - 1, e - 1);
        Some((self.close[s + 4] > self.close[prev], self.close[last] > self.close[last - 5], self.close[last] > self.close[prev]))
    }
    /// Feb 1 - Dec 31 interval of year y
    fn feb_dec(&self, w: &mut [f64], y: i64, v: f64) {
        self.c.mark(w, days_from_civil(y, 1, 31), dec31(y), v);
    }
    fn jan(&self, w: &mut [f64], y: i64, v: f64) {
        self.c.mark(w, dec31(y - 1), days_from_civil(y, 1, 31), v);
    }
    /// cycle bullish phases: anchor + k*period, bullish (start, start+bull] (days) or 6 calendar months
    fn cycle(&self, anchor: i64, period: i64, bull: i64, months: bool) -> Vec<f64> {
        let mut w = self.zeros();
        if self.n == 0 || period <= 0 {
            return w;
        }
        let (lo, hi) = (self.c.dn[0], self.c.dn[self.n - 1]);
        let k0 = (lo - anchor).div_euclid(period) - 1;
        let k1 = (hi - anchor).div_euclid(period) + 1;
        for k in k0..=k1 {
            let s = anchor + k * period;
            let e = if months { plus_months(s, bull) } else { s + bull };
            self.c.mark(&mut w, s, e, 1.0);
        }
        w
    }
    fn years(&self, keep: impl Fn(i64) -> bool) -> Vec<f64> {
        let mut w = self.zeros();
        for y in self.c.years() {
            if keep(y) {
                self.c.mark(&mut w, dec31(y - 1), dec31(y), 1.0);
            }
        }
        w
    }
    fn midterm(&self, before: usize, after: usize, literal: bool) -> Vec<f64> {
        let mut w = self.zeros();
        for y in self.c.years() {
            if y.rem_euclid(4) != 2 {
                continue;
            }
            let e = election_day(y, literal);
            let p = self.c.dn.partition_point(|&z| z < e);
            let q = self.c.dn.partition_point(|&z| z <= e);
            let s = p.saturating_sub(before);
            let t = (q + after).min(self.n);
            if p >= before && q + after <= self.n {
                for k in s..t {
                    w[k] = 1.0;
                }
            }
        }
        w
    }
}

// ------------------------------------------------------------------ catalogue

macro_rules! note { () => { "Book index: Dow Jones Industrial Average (vault ticker DJI, 1970-; the book starts 1900-1967 depending on the rule). Runs on other indices, ETFs and stocks as a declared application; rules set by US market holidays (the days around a closure, Christmas week, Santa Claus, UMDS/UUMDS, the KTI models) run on US instruments only, election dates are applied as calendar dates. Position on day t earns close(t-1)->close(t), as the book measures it. 2:1 and short positions are daily-reset like the leveraged / inverse index funds the book names (Table 1.1, p.11: a 2-to-1 fund \"will gain roughly 2 percent if the Dow rises 1 percent\"): one trade per day [fitted reading: reproduces Table 2.4 year by year]." }; }
macro_rules! cash { () => { " The book adds 1% a year interest while in cash; the vault does not model cash interest (years in cash show 0.0% here, +1.0% in the book)." }; }

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    let d = |id, name, group, dir, status, rules, notes, claim| StrategyDef {
        id, name, group, session: 0, dir, status, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim,
    };
    vec![
        // ---- Chapter 2: January
        d("kpss_first5", "First five days of January -> long Feb-Dec", "Ch 2 · The month of January (p.22-24)", "Long", Live,
            &["F|\"A bullish signal occurs when the first five trading days of January show a net gain.\" (p.24): close of the 5th January trading day above the last December close",
              "E|\"A bullish period extends from the close on January 31 following a bullish signal\" (p.24): buy at the close of the last January trading day",
              "X|\"through December 31 of the same year\" (p.24): sell at the close of the last trading day of the year"],
            note!(), claim("45 bullish years", "82.2% (37 up / 8 down)", "10.1% annualized in bullish periods", "11 months", "Dow 1937-2007 ($1,000 -> $38,581)")),
        d("kpss_last5", "Last five days of January -> long Feb-Dec", "Ch 2 · The month of January (p.25-27)", "Long", Live,
            &["F|\"A bullish signal occurs when the last five trading days of January show a net gain.\" (p.25): last January close above the close five trading days earlier",
              "E|Buy at the close on January 31 (last January trading day) after a bullish signal (p.25)",
              "X|Sell at the close on December 31 (p.25)"],
            note!(), claim("42 bullish years", "83.3% (35 up / 7 down)", "11.3% annualized in bullish periods", "11 months", "Dow 1937-2007 ($1,000 -> $49,138)")),
        d("kpss_janbar", "January Barometer -> long Feb-Dec", "Ch 2 · The January barometer (p.27-29)", "Long", Live,
            &["F|\"A bullish signal occurs when the month of January as a whole shows a net gain.\" (p.27)",
              "E|\"A bullish period extends from the close on January 31 following a bullish signal\" (p.28)",
              "X|\"through December 31 of the same year\" (p.28)"],
            note!(), claim("47 bullish years", "91.4% (43 up / 4 down)", "11.8% annualized in bullish periods", "11 months", "Dow 1937-2007 ($1,000 -> $89,570)")),
        d("kpss_jbs", "January Barometer as a stand-alone system", "Ch 2 · The January barometer as a stand-alone system (p.29-31)", "Long", Live,
            &["E|\"We will be long the Dow during the month of January every year.\" (p.29)",
              "X|\"If January shows a net gain, we will continue to hold the Dow through the end of the year.\" (p.29)",
              "X|\"If January shows a net loss, we will exit the market and hold cash through the end of the year.\" (p.29)"],
            concat!(note!(), cash!()), claim("70 years", "", "", "1 or 12 months", "Dow 1937-2007 ($1,000 -> $215,338 vs $109,763 buy and hold)")),
        d("kpss_jaynewary", "JayNewary barometer reading -> long Feb-Dec", "Ch 2 · The JayNewary barometer (p.31-38)", "Long", Live,
            &["F|\"If the first five trading days of January register a net gain, add one point ... If the last five trading days of January register a net gain, add one point ... If the month of January as a whole registers a net gain, add one point\" (p.31)",
              "E|Long the Dow Feb 1-Dec 31 only in years whose reading equals p.reading (Table 2.1, p.32 tests readings 0, 1, 2 and 3; default 3: \"no investor should ignore a JayNewary barometer reading of 3\", p.36)",
              "X|Sell at the close on December 31"],
            note!(), claim("24 (reading 3)", "91.7% (22 up / 2 down)", "+13.3% avg Feb-Dec", "11 months", "Dow 1938-2007 (reading 3: $1,000 -> $17,408; 0: 1 of 6 up, -7.5%)")),
        d("kpss_ujbs", "Ultimate January Barometer System (UJBS)", "Ch 2 · The ultimate January barometer system (p.38-42)", "Long/Short", Live,
            &["E|\"We will hold a long position in the Dow each year during the month of January.\" (p.39)",
              "E|\"If the JayNewary barometer registers a reading of 3, we will go long the Dow using two-to-one leverage.\" (p.39)",
              "E|\"If the JayNewary barometer registers a reading of 2 and/or if the month of January as a whole registers a gain, we will buy the Dow (without leverage).\" (p.39)",
              "F|\"If the JayNewary barometer registers a reading of 1 and the month of January as a whole registers a loss, we will sit out the stock market and hold cash until the end of the year.\" (p.39)",
              "E|\"If the JayNewary barometer registers a reading of 0, we will sell short the Dow and hold that short position until the end of the year.\" (p.39)",
              "X|Every position ends at the close on December 31 (Table 2.3, p.39)"],
            concat!(note!(), cash!()), claim("70 years", "50 up / 20 down", "+14.9% avg year (buy and hold 8.1%)", "1-12 months", "Dow 1938-2007 ($1,000 -> $5,365,964), Table 2.4 year by year")),
        // ---- Chapter 3: holidays
        d("kpss_hol_d1", "Trading day before each major holiday", "Ch 3 · The trading day before each holiday (p.51-52)", "Long", Live,
            &["F|Holidays (Table 3.1, p.50): Presidents' Day, Good Friday, Memorial Day, Independence Day, Labor Day, Thanksgiving, Christmas, New Year's (Martin Luther King Day omitted)",
              "E|\"invested in the Dow only during the single trading day prior to each major stock market holiday\" (p.51): buy at the close two trading days before the holiday",
              "X|Sell at the close of the last trading day before the holiday"],
            note!(), claim("8 a year", "62.2% of days up", "0.1923% avg day (62.3% annualized)", "1 day", "Dow 1933-2007 ($1,000 -> $3,147)")),
        d("kpss_hol_d2", "Two trading days before each major holiday", "Ch 3 · Two days before each holiday (p.52-54)", "Long", Live,
            &["E|\"invested in the Dow only during the two trading days prior to each market holiday\" (p.52): buy at the close three trading days before the holiday",
              "X|Sell at the close of the last trading day before the holiday"],
            note!(), claim("8 a year", "", "0.1263% avg day (37.5% annualized)", "2 days", "Dow 1933-2007 ($1,000 -> $4,552)")),
        d("kpss_hol_3x3", "Three days before and after each major holiday", "Ch 3 · Three days before and after each holiday (p.54-55)", "Long", Live,
            &["E|\"we will look at the three trading days prior to each market holiday and the three trading days directly following each market holiday\" (p.54): buy at the close four trading days before the holiday",
              "X|Sell at the close of the third trading day after the holiday (Christmas/New Year's windows overlap and merge)"],
            note!(), claim("8 a year", "", "0.0810% avg day (22.6% annualized)", "6 days", "Dow 1933-2007 ($1,000 -> $15,247)")),
        d("kpss_hol_top24", "Top 24 holiday trading days", "Ch 3 · The best holiday trading days (p.56-61)", "Long", Live,
            &["E|\"assume that we are long the Dow only during the first 24 pre- and postholiday trading days listed at the top of Table 3.3\" (p.58), Table 3.4 (p.60):",
              "F|New Year's -3,-2,-1,+1,+2,+3; Presidents' Day -2,+3; Good Friday -2,-1,+2; Memorial Day -1; Independence Day -2,-1,+1,+2,+3; Labor Day -1; Thanksgiving -2,-1,+1; Christmas -1,+1,+3",
              "X|Each listed day: buy at the prior close, sell at that day's close (adjacent days merge)"],
            concat!(note!(), " Cherry-picked in hindsight from 1933-2007 data; the book warns these results \"must be viewed with a grain of salt\" (p.60)."),
            claim("24 a year", "", "0.189% avg day (61% annualized)", "1-6 days", "Dow 1933-2007 ($1,000 -> $20,876)")),
        d("kpss_uhts", "Ultimate Holiday Trading System (UHTS)", "Ch 3 · The ultimate holiday system (p.61-63)", "Long", Live,
            &["E|\"On the single trading day just prior to each of the eight major stock market holidays ... we will hold the Dow using two-to-one leverage.\" (p.61)",
              "E|\"On the two previous trading days (i.e., the second and third trading days prior to the holiday), we will hold the Dow using no leverage.\" (p.61)",
              "E|\"Likewise, on the three trading days after each holiday, we will hold the Dow using no leverage.\" (p.61)",
              "X|\"on all other trading days, we will assume that we are in cash\" (p.61)",
              "F|[Book silent] A day that is both after Christmas and before New Year's takes the larger position."],
            concat!(note!(), cash!()), claim("8 a year", "", "", "6 days", "Dow 1933-2007 ($1,000 -> $100,708 vs $132,781 buy and hold; in market ~18% of the time)")),
        d("kpss_xmas_week", "Day after Christmas through New Year's Eve", "Ch 3 · The most wonderful week of the year (p.63-65)", "Long", Live,
            &["E|\"the performance of the Dow between the first trading day after Christmas through—and including—New Year's Eve\" (p.63): buy at the close of the last trading day before Christmas",
              "X|Sell at the close of the last trading day of the year"],
            note!(), claim("108", "78.7% (85 up / 23 down)", "+1.02% avg", "3-5 days", "Dow 1900-2007 (last 19 years: 58% up, +0.36% avg)")),
        // ---- Chapter 4: days of the month
        d("kpss_td2", "Second trading day of the month", "Ch 4 · The best day of the month (p.68-70)", "Long", Live,
            &["E|\"invested in the Dow only on the second trading day of each month\" (p.69): buy at the close of trading day 1", "X|Sell at the close of trading day 2"],
            note!(), claim("12 a year", "", "0.1512% avg day (46.3% annualized)", "1 day", "Dow 1901-2007")),
        d("kpss_td1_5", "Trading days 1-5 of the month", "Ch 4 · Monthly wave 1 (p.72-73)", "Long", Live,
            &["E|\"$1,000 invested only during the first five trading days of each trading month\" (p.72): buy at the close of the last trading day of the month", "X|Sell at the close of trading day 5"],
            note!(), claim("12 a year", "", "Table 4.1: days 1-5 cumulative +223/+524/+303/+65/+31%", "5 days", "Dow 1900-2007")),
        d("kpss_mid4", "Trading days 9-12 of the month", "Ch 4 · Monthly wave 3 / The midmonth pattern (p.73-87)", "Long", Live,
            &["E|\"invested in the Dow only during trading days 9, 10, 11, and 12 each month\" (p.85): buy at the close of trading day 8", "X|Sell at the close of trading day 12"],
            note!(), claim("12 a year", "", "1933-1979: $1,000 -> $874; 1980-2007: strong gains (Fig 4.13)", "4 days", "Dow 1933-2007")),
        d("kpss_worst14", "The 14 worst days of the month", "Ch 4 · The worst trading days of the month (p.80-81)", "Long", Live,
            &["E|\"assume that an investor bought and held the Dow only during the following periods each month: Trading days 6, 7, and 8. Trading days 13, 14, 15, 16, 17, 18, 19, and 20. Trading days −7, −6, and −5\" (p.80)",
              "X|Out on all other days"],
            concat!(note!(), " The book's negative control (what to avoid)."), claim("", "", "$1,000 -> $92 (-90.8%)", "", "Dow 1900-2007")),
        d("kpss_nonworst", "All but the 14 worst days of the month", "Ch 4 · The best trading days of the month, part 1 (p.81-83)", "Long", Live,
            &["E|\"We will hold a long position in the Dow on all trading days each month except for trading days 6 through 8, 13 through 20, and −7, −6, and −5.\" (p.82)",
              "X|Out during those days"],
            note!(), claim("", "", "$1,000 -> $2,029,812 (Table 4.7 by decade)", "", "Dow 1900-2007")),
        d("kpss_six", "Last two + first four trading days of the month", "Ch 4 · The month-end/new-month pattern (p.83-85)", "Long", Live,
            &["E|\"an investor is long the Dow only during these six days each month\" (p.84): the last two and the first four trading days; buy at the close of trading day -3",
              "X|Sell at the close of trading day 4"],
            note!(), claim("12 a year", "", "0.1037% avg day (29.9% annualized)", "6 days", "Dow 1933-2007 ($1,000 -> $202,335)")),
        d("kpss_monthly10", "Monthly 10", "Ch 4 · The monthly 10 (p.87-89)", "Long", Live,
            &["E|\"Starting December 31, 1933, we will hold a long position in the Dow during the first four and the last two trading days of each month.\" (p.88)",
              "E|\"Starting December 31, 1979, we will also hold a long position in the Dow during trading days 9, 10, 11 and 12 each month.\" (p.88)",
              "X|Out (in cash) on every other trading day: sell at the close of the last day of each held stretch"],
            note!(), claim("", "", "0.0993% avg day (28.4% annualized)", "", "Dow 1933-2007 ($1,000 -> $562,500)")),
        d("kpss_santa", "Santa Claus rally (Thanksgiving to 5th day of January)", "Ch 4 · The most wonderful time of the year (p.89-91)", "Long", Live,
            &["E|\"we will be long the Dow for the two days before Thanksgiving through the fifth trading day of the following January\" (p.89); \"entering the market at the close on the Monday of Thanksgiving week\" (p.91)",
              "X|Sell at the close of the fifth trading day of January"],
            note!(), claim("75", "78.7% (59 up / 15 down)", "+2.92% avg", "~1.3 months", "Dow 1933-2007")),
        d("kpss_m10_santa", "Monthly 10 + Santa Claus rally", "Ch 4 · Monthly 10 combined with the Santa Claus rally (p.91-93)", "Long", Live,
            &["E|\"The first four trading days of every month. The last two trading days of every month. Trading days 9, 10, 11, and 12 of every month. All days between and including the two trading days prior to Thanksgiving through the fifth trading day of the next year.\" (p.91)",
              "F|Days 9-12 from December 31, 1979 (as in Monthly 10)",
              "X|Out (in cash) on every other trading day: sell at the close of the last day of each held stretch"],
            note!(), claim("", "", "0.092% avg day (26.1% annualized)", "", "Dow 1933-2007 ($1,000 -> $1,200,209)")),
        d("kpss_umds", "Ultimate Monthly Days System (UMDS)", "Ch 4 · The ultimate monthly days system (p.93-95)", "Long", Live,
            &["E|\"The last two trading days of each month. The first four trading days of each month. Trading days 9, 10, 11, and 12 of each month. The period starting two days prior to Thanksgiving and extending through the fifth trading day of the following January. The trading day prior to each major stock market holiday.\" (p.93)",
              "X|\"On all other trading days, be in cash\" (p.93)",
              "F|Days 9-12 from December 31, 1979 (as in Monthly 10)"],
            concat!(note!(), cash!()), claim("", "", "0.0925% avg day (26.2% annualized)", "", "Dow 1933-Oct 2007 ($1,000 -> $3,027,390)")),
        d("kpss_uumds", "Ultimate, Ultimate Monthly Days System (UUMDS)", "Ch 4 · One last tweak (p.96-100)", "Long", Live,
            &["E|UMDS rules (p.93) plus: \"On the trading day prior to each major stock market holiday, we will hold the Dow using two-to-one leverage.\" (p.96)",
              "F|Days 9-12 from December 31, 1979 (as in Monthly 10)",
              "X|Out (in cash) on every other trading day: sell at the close of the last day of each held stretch"],
            concat!(note!(), cash!()), claim("74 years", "67 up / 7 down", "+13.8% avg year (buy and hold 8.1%)", "", "Dow 1934-2007 ($1,000 -> $9,506,979), Table 4.8 year by year")),
        // ---- Chapter 5: yearly
        d("kpss_dec_b", "Decennial strategy B (skip years 0, 1 and 7)", "Ch 5 · Decennial patterns part 2 (p.106-110)", "Long", Live,
            &["E|\"Hold the Dow only during years 2, 3, 4, 5, 6, 8, and 9 of every decade.\" (Table 5.2, p.106): buy at the last close of years ending 1, 7", "X|Sell at the last close of years ending 9, 6"],
            note!(), claim("", "", "$1,000 -> $1,764,664 vs $199,142 buy and hold", "years", "Dow 1900-2007 (1942-2007: $62,521 vs $119,546)")),
        d("kpss_dec_c", "Decennial strategy C (only years 0, 1 and 7)", "Ch 5 · Decennial patterns part 2 (p.106-110)", "Long", Live,
            &["E|\"Hold the Dow only during years 0, 1, and 7 of every decade.\" (Table 5.2, p.106)",
              "X|Out (in cash) on every other trading day: sell at the close of the last day of each held stretch"],
            note!(), claim("", "", "$1,000 -> $113 (1942-2007: $1,912)", "years", "Dow 1900-2007")),
        d("kpss_middecade", "Mid-decade cycle (Sep 30 year 4 - Mar 31 year 6)", "Ch 5 · Intradecade trend 1 (p.123-126)", "Long", Live,
            &["E|\"Starting at the close of trading on September 30 of year 4\" (p.131): buy at the last close on or before Sep 30 of years ending in 4", "X|\"through the close of trading on March 31 of year 6\" (p.131)"],
            note!(), claim("11", "10 up / 1 down", "+41.7% avg (Table 5.8)", "18 months", "Dow 1904-2006 ($1,000 -> $40,948)")),
        d("kpss_latedecade", "Late-decade cycle (Feb year 8 - Sep 30 year 9)", "Ch 5 · Intradecade trend 2 (p.127-128)", "Long", Live,
            &["E|\"Starting at the close of trading on February 28 of year 8\" (p.131): buy at the last February close of years ending in 8", "X|\"through the close of trading on September 30 of year 9\" (p.131)"],
            note!(), claim("10", "9 up / 1 down", "+31.6% avg (Table 5.9)", "19 months", "Dow 1908-1999 ($1,000 -> $13,338)")),
        d("kpss_20yr", "20-year cycle (Sep 30 year 2 - Dec 31 year 5, even decades)", "Ch 5 · Intradecade trend 3 (p.128-130)", "Long", Live,
            &["E|\"Starting at the close of trading on September 30, 1902 (and 1922, 1942, 1962, 1982, 2002)\" (p.131)", "X|\"through the close of trading on December 31 of the next year 5\" (p.131)"],
            note!(), claim("6", "6 up", "+61.0% avg (Table 5.10)", "39 months", "Dow 1902-2005 ($1,000 -> $17,063)")),
        d("kpss_intradecade", "Intradecade cycles combined", "Ch 5 · Intradecade cycles combined (p.130-136)", "Long", Live,
            &["E|Long during the mid-decade, late-decade and 20-year cycle windows (Table 5.11, p.132): \"An investor could have simply bought on the dates in the left-hand column and sold on the dates in the column to the right.\" (p.132)",
              "F|Overlapping 20-year and mid-decade windows form one holding (Sep 30 year 2 - Mar 31 year 6)",
              "X|Out (in cash) on every other trading day: sell at the close of the last day of each held stretch"],
            note!(), claim("21", "20 up / 1 down", "Table 5.11", "", "Dow 1902-2007 ($1,000 -> $1,851,398 vs $187,595)")),
        // ---- Chapter 6: cycles
        d("kpss_c212", "212-week cycle: first six months", "Ch 6 · The 212-week cycle (p.140-146)", "Long", Live,
            &["F|\"Our first cycle will begin at the close of trading on May 16, 1938. From this point, we will count forward exactly 1,484 calendar days\" (p.141)",
              "E|Buy at the close on each cycle start date (last trading day on or before it)",
              "X|Sell six months later (\"6-Month Exit Dates\", Table 6.2, p.146): at the close on the same day of the month six months later"],
            note!(), claim("18", "88.9% (16 up / 2 down)", "+14.5% avg (median 16.1%)", "6 months", "Dow 1938-2007 ($1,000 -> $11,104), Table 6.1")),
        d("kpss_c40w", "40-week cycle: bullish 20 weeks", "Ch 6 · The 40-week cycle (p.146-152)", "Long", Live,
            &["F|\"we will start at the close of trading on April 21, 1967. ... the first 20 weeks (measured as 140 calendar days) is the bullish phase of the cycle and ... the next 20 weeks (another 140 calendar days) is the bearish phase\" (p.147)",
              "E|Buy at the close on each cycle start (every 280 days)", "X|Sell at the close 140 days later",
              "F|Dates on a holiday or weekend use the prior trading day (Table 6.3: 4/13/95, 12/24/92)"],
            note!(), claim("53", "75.5% (40 up / 13 down)", "17.7% annualized", "20 weeks", "Dow 1967-2007 ($1,000 -> $21,308), Table 6.3 with Dow prices")),
        d("kpss_c53d", "53-day cycle: bullish 27 days", "Ch 6 · The 53-day cycle (p.153-163)", "Long", Live,
            &["F|\"Every 53 calendar days a new 53-day cycle begins ... the first 27 calendar days of each new cycle as the bullish phase\" (p.153); \"Our first cycle begins at the close on March 22, 1967.\" (p.153)",
              "E|Buy at the close on each cycle start", "X|Sell at the close 27 days later"],
            note!(), claim("", "", "$1,000 -> $6,239 (bearish phase $2,402)", "27 days", "Dow 1967-2007, dates in Table 6.4")),
        d("kpss_ctcm1", "Comprehensive time cycles model > 0", "Ch 6 · Combining cycles (p.163-167)", "Long", Live,
            &["F|\"If the 212-week cycle is bullish, then one point is added ... If the 40-week cycle is bullish, then one point ... If the 53-day cycle is bullish, then one point\" (p.164)",
              "E|\"The simplest strategy is to be in the market whenever the CTCM is greater than 0.\" (p.166)",
              "X|Out (in cash) on every other trading day: sell at the close of the last day of each held stretch"],
            note!(), claim("", "", "$1,000 -> $33,005 vs $15,019 buy and hold", "", "Dow 1967-2007")),
        d("kpss_ctcm2", "Comprehensive time cycles model >= 2", "Ch 6 · Combining cycles (p.167-168)", "Long", Live,
            &["E|\"invested in the Dow only during times when at least two or more of the three cycles are in a bullish phase\" (p.167)",
              "X|Out (in cash) on every other trading day: sell at the close of the last day of each held stretch"],
            note!(), claim("", "", "$1,000 -> $9,119 (20.0% annualized)", "", "Dow 1967-2007")),
        d("kpss_utcm", "Ultimate Time Cycles Model (UTCM)", "Ch 6 · The ultimate time cycles model (p.168-171)", "Long", Live,
            &["E|\"Rule 1: If any two cycles are bullish at the same time, then we will ... buy the Dow using two-to-one leverage.\" (p.168)",
              "E|\"Rule 2: If only the 212-week or the 40-week cycle is bullish, then we will buy the Dow without any leverage.\" (p.168)",
              "X|\"Rule 3: If the 53-day cycle is the only cycle that is bullish, or if no cycle is bullish, then we will hold cash\" (p.168)"],
            concat!(note!(), cash!()), claim("41 years", "35 up / 6 down", "+17.0% avg year (Dow 8.3%)", "", "Dow 1967-2007 ($1,000 -> $356,452), Table 6.7 year by year")),
        // ---- Chapter 7: election cycle
        d("kpss_yr_post", "Postelection years only", "Ch 7 · The postelection year (p.176-178)", "Long", Live,
            &["E|\"invested in the Dow only during each postelection year\" (p.176): buy at the last close of the election year", "X|Sell at the last close of the postelection year"],
            note!(), claim("18", "8 up / 10 down", "+1.6% avg (median -2.2%)", "1 year", "Dow 1937-2005, Table 7.2")),
        d("kpss_yr_mid", "Midterm years only", "Ch 7 · The midterm year (p.178-180)", "Long", Live,
            &["E|Long the Dow only during each midterm year (calendar year)",
              "X|Out (in cash) on every other trading day: sell at the close of the last day of each held stretch"],
            note!(), claim("19", "12 up / 7 down", "+6.7% avg (median 4.8%)", "1 year", "Dow 1934-2006, Table 7.3")),
        d("kpss_yr_pre", "Preelection years only", "Ch 7 · The preelection year (p.181-182)", "Long", Live,
            &["E|\"investing in the Dow only during each preelection year\" (p.181)",
              "X|Out (in cash) on every other trading day: sell at the close of the last day of each held stretch"],
            note!(), claim("19", "18 up / 1 down", "+15.1% avg (median 15.2%)", "1 year", "Dow 1935-2007, Table 7.4")),
        d("kpss_yr_elec", "Election years only", "Ch 7 · The election year (p.182-185)", "Long", Live,
            &["E|\"investing in the Dow only during each election year\" (p.183)",
              "X|Out (in cash) on every other trading day: sell at the close of the last day of each held stretch"],
            note!(), claim("18", "13 up / 5 down", "+6.9% avg (median 6.3%)", "1 year", "Dow 1936-2004, Table 7.5")),
        d("kpss_yr_post_mid", "Postelection + midterm years", "Ch 7 · Postelection and midterm years combined (p.185-186)", "Long", Live,
            &["E|\"investing in the Dow only during these two years of every four years\" (p.185)",
              "X|Out (in cash) on every other trading day: sell at the close of the last day of each held stretch"],
            note!(), claim("18", "11 up / 7 down", "+12.7% avg per 2 years", "2 years", "Dow 1934-2007 ($1,000 -> $2,446), Table 7.6")),
        d("kpss_yr_pre_elec", "Preelection + election years", "Ch 7 · Preelection and election years combined (p.186-187)", "Long", Live,
            &["E|\"$1,000 invested during the preelection and election years\" (p.186)",
              "X|Out (in cash) on every other trading day: sell at the close of the last day of each held stretch"],
            note!(), claim("18", "17 up / 1 down", "+28.4% avg per 2 years", "2 years", "Dow 1934-2007 ($1,000 -> $52,115), Table 7.6")),
        d("kpss_stoken15", "Stoken's 15 most bullish months", "Ch 7 · Zeroing in on the 15 most bullish months (p.189-192)", "Long", Live,
            &["E|\"begins on October 1 of the midterm election year ... (we start at the close of the last trading day of September of each midterm election year\" (p.189)",
              "X|\"and extends through December 31 of the preelection year\" (p.189)"],
            note!(), claim("19", "18 up / 1 down", "+25.6% avg (Table 7.8)", "15 months", "Dow 1934-2007 ($1,000 -> $68,189)")),
        d("kpss_fav14", "14 most bullish months of the election cycle", "Ch 7 · Narrowing down the 14 most-bullish months (p.192-196)", "Long", Live,
            &["E|\"Starting at close of last trading day during September of the midterm year and extending through the close of the last trading day during September of the preelection year.\" (Table 7.10, p.194)",
              "E|\"Starting at close of last trading day of October during the preelection year and extending through close of last trading day of December during the preelection year.\" (Table 7.10, p.194)",
              "X|Out (in cash) on every other trading day: sell at the close of the last day of each held stretch"],
            note!(), claim("19", "19 up / 0 down", "+27.1% avg (Table 7.11)", "14 months", "Dow 1934-2007 ($1,000 -> $83,908)")),
        d("kpss_fav21", "21 favorable months of the election cycle", "Ch 7 · 21 bullish months combined (p.201-205)", "Long", Live,
            &["E|The 14-month period (Table 7.10) plus \"the action of the Dow between the close on May 31 of each presidential election year and December 31 of that same year\" (p.198); Table 7.13 (p.202-203)",
              "X|Out (in cash) on every other trading day: sell at the close of the last day of each held stretch"],
            note!(), claim("18 cycles", "94.4% (17 up / 1 down)", "+38.5% avg per cycle (Table 7.14)", "21 of 48 months", "Dow 1933-2007 ($1,000 -> $344,142)")),
        d("kpss_midterm", "Midterm election short-term trade", "Ch 7 · Midterm election short-term trade (p.206-208)", "Long", Live,
            &["E|\"being in the market during the five trading days prior to the midterm election, the three trading days after the midterm election, and the day of the midterm election itself if the market is open\" (p.206): buy at the close six trading days before election day",
              "X|Sell at the close of the third trading day after election day",
              "F|\"The midterm election takes place on the first Tuesday of November\" (p.206): used as written (Table 7.15 confirms it: 1994 = -1.2% only with Tuesday November 1). p.literal=0 uses the legal date (Tuesday after the first Monday) instead"],
            note!(), claim("19", "89.5% (17 up / 2 down)", "+3.0% avg (Table 7.15)", "8-9 days", "Dow 1934-2006")),
        d("kpss_pre_mar_jul", "March through July of preelection years", "Ch 7 · March through July of a preelection year (p.208-210)", "Long", Live,
            &["E|\"between March 1 and the end of July during preelection years\" (p.208); Table 7.16: \"Between the End of February and the End of July\": buy at the last February close", "X|Sell at the last July close"],
            note!(), claim("19", "89.5% (17 up / 2 down)", "+8.8% avg (Table 7.16)", "5 months", "Dow 1935-2007")),
        d("kpss_uecs", "Ultimate Election Cycle System (UECS)", "Ch 7 · The ultimate election cycle system (p.210-214)", "Long", Live,
            &["E|\"If we are within five trading days before or three trading days after a midterm election, then we will buy the Dow using two-to-one leverage.\" (p.210)",
              "E|\"If we are between the first trading day of March and the last trading day of July in a preelection year, then we will buy the Dow using two-to-one leverage.\" (p.210)",
              "E|\"Otherwise, we will be long the Dow with no leverage between the following: October 1 of the midterm year and September 30 of the preelection year. November 1 of the preelection year and December 31 of the preelection year. June 1 and December 31 of the election year. January 1 and December 31 of a postelection year that ends in 5\" (p.211)",
              "X|\"Under any other circumstance, we will be in cash.\" (p.211)"],
            concat!(note!(), cash!()), claim("74 years", "67 up / 7 down", "Table 7.17 year by year", "", "Dow 1934-2007 ($1,000 -> $4,590,746)")),
        // ---- Chapter 8: sell in May
        d("kpss_nov_may", "November to the third trading day of May", "Ch 8 · November to May (p.220-224)", "Long", Live,
            &["E|\"Buy the Dow at the close of trading on the last trading day of October.\" (p.220)",
              "X|\"Sell the Dow and move to cash at the close of trading on the third trading day in the month of May in the following year.\" (p.220)"],
            note!(), claim("58", "81% (47 up / 11 down)", "17.1% annualized", "6 months + 3 days", "Dow 1949-2007 ($1,000 -> $82,666), Table 8.1 with Dow prices")),
        d("kpss_ndx_nov_jun", "Nasdaq November through June", "Ch 8 · October through June for the Nasdaq (p.238-241)", "Long", Live,
            &["E|\"investing in the Nasdaq from November through June\" (p.241); Table 8.4: buy at the last October close",
              "X|Sell at the last June close"],
            "Book index: OTC Composite 1971-1990, then Nasdaq 100 (p.238). Run on NDX (1990-) or QQQ; the OTC Composite is not in the vault.",
            claim("36", "78% (28 up / 8 down)", "19.4% annualized", "8 months", "OTC/NDX 1971-2007 ($1,000 -> $37,689), Table 8.4")),
        d("kpss_jja", "June, July and August only", "Ch 8 · The months of June, July, and August (p.249-252)", "Long", Live,
            &["E|\"had investors bought the Dow on May 31 of each year and then sold the Dow and returned to cash on August 31 of the same year\" (p.249)",
              "X|Out (in cash) on every other trading day: sell at the close of the last day of each held stretch"],
            note!(), claim("43", "25 up / 18 down", "+0.3% avg (Table 8.6)", "3 months", "Dow 1965-2007 ($1,000 -> $996)")),
        d("kpss_sept", "September only", "Ch 8 · The dreaded month of September (p.254-259)", "Long", Live,
            &["E|\"$1,000 invested in the Dow only during September every year since 1900\" (p.258)",
              "X|Out (in cash) on every other trading day: sell at the close of the last day of each held stretch"],
            concat!(note!(), " The book's dead zone (what to avoid)."), claim("108", "42% (45 up)", "-11.7% annualized", "1 month", "Dow 1900-2007 ($1,000 -> $251)")),
        d("kpss_ex_sept", "Buy and hold except September", "Ch 8 · The easiest market-beating strategy in the world (p.260-261)", "Long", Live,
            &["E|\"what if all we did was sit out the stock market during September every year?\" (p.260); \"the willingness to act on August 31 and September 30 of every year\" (p.261)",
              "X|Sell at the last August close, buy back at the last September close"],
            note!(), claim("", "", "$1,000 -> $747,079 vs $199,142 buy and hold", "11 months", "Dow 1900-2007")),
        d("kpss_oct7", "October of years ending in 7", "Ch 8 · October of year 7 (p.261-264)", "Long", Live,
            &["E|\"invested in the Dow only during the month of October during years ending in 7\" (p.262): buy at the Sep 30 close", "X|Sell at the Oct 31 close"],
            concat!(note!(), " The book's dead zone (what to avoid)."), claim("11", "2 up / 9 down", "-8.3% avg (Table 8.9 with Dow closes)", "1 month", "Dow 1907-2007 ($1,000 -> $408)")),
        // ---- Chapter 9: KTI models
        d("kpss_kti_long", "Model 1: Long-Only Method (KTI >= 3)", "Ch 9 · Model 1 (p.276-278)", "Long", Live,
            &["F|Known trends index (Table 9.1, p.270): +1 trading day 1-4, 9-12, last or next-to-last; +1 Nov 1 to 3rd trading day of May; +1 last 3 June / first 9 July days; -1 September; +1 Oct 1 midterm-Sep 30 preelection, Nov 1-Dec 31 preelection, Jun 1-Dec 31 election year; +1 Mar 1-Jul 31 preelection; +1 midterm election window (5 before, day, 3 after); +1 40-week bullish phase; +1 six months after a 212-week cycle date; +1 Oct year 4-Mar year 6; +1 Mar year 8-Sep year 9; +1 Oct year 2-Dec year 5 of even decades; +1 within 3 trading days before/after a major holiday",
              "E|\"we will hold the Dow during any trading day for which the KTI registers a reading of 3 or more\" (p.276)",
              "X|\"We will not use any leverage, we will not sell short\" (p.276)"],
            concat!(note!(), cash!()), claim("74 years", "", "11.8% average annual", "", "Dow 1933-2007 ($1,000 -> $2,792,896), Table 9.6")),
        d("kpss_lopl", "Model 2: Long-Only Plus Leverage (LOPL)", "Ch 9 · Model 2 (p.278-281)", "Long", Live,
            &["E|\"When the KTI is at 5 or more, we will hold the Dow using two-to-one leverage\" (p.279)",
              "E|\"When the KTI is at 3 or 4, we will hold the Dow using no leverage.\" (p.279)",
              "X|\"When the KTI is at any level below 3, we will hold cash.\" (p.279)"],
            concat!(note!(), cash!()), claim("74 years", "65 up / 9 down", "18.4% average annual", "", "Dow 1933-2007 ($1,000 -> $116,296,163), Table 9.6")),
        d("kpss_jusb", "Model 3: Jay's Ultimate Seasonal Barometer (JUSB)", "Ch 9 · Model 3 (p.281-284)", "Long/Short", Live,
            &["E|\"When the KTI is at 5 or more, we will hold the Dow using two-to-one leverage\" (p.282)",
              "E|\"When the KTI is at 3 or 4, we will hold the Dow using no leverage.\" (p.282)",
              "F|\"When the KTI is at 2, we will simply hold cash\" (p.282)",
              "E|\"When the KTI is at any level below 2 (i.e., –1, 0, or 1), we will hold a short position in the Dow.\" (p.282)",
              "X|Out (in cash) on every other trading day: sell at the close of the last day of each held stretch"],
            concat!(note!(), cash!()), claim("74 years", "70 up / 4 down", "22.7% average annual", "", "Dow 1933-2007 ($1,000 -> $1,535,650,131), Table 9.6")),
        // ---- not testable / duplicates are listed in the README; statistics only:
        d("kpss_road_map", "Decennial road map and four decade tendencies", "Ch 5 · Decennial patterns part 3 (p.110-123)", "Long", NotTestable("average-decade path and judged tendencies (early lull, mid-decade rally, 7/8 sell-off, late rally); no entry or exit dates — the exact windows are kpss_middecade / kpss_latedecade / kpss_20yr"),
            &["F|\"No one should get the idea that every decade will follow the road map\" (p.112)"], "Statistic and judgement only.", None),
        d("kpss_rolling", "Rolling-return and drawdown studies", "Ch 2, 4, 8 (Fig 4.8, Table 8.3)", "Long", NotTestable("statistics of the rules above, not separate rules"),
            &["F|Five- and ten-year rolling returns of the bullish versus bearish periods (p.43, p.80, p.236)"], "Statistic only.", None),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    if !id.starts_with("kpss_") {
        return None;
    }
    Some(match id {
        "kpss_first5" | "kpss_last5" | "kpss_janbar" | "kpss_jbs" | "kpss_jaynewary" | "kpss_ujbs" => ("kpss_jan", "January indicators", "January"),
        "kpss_hol_d1" | "kpss_hol_d2" | "kpss_hol_3x3" | "kpss_hol_top24" | "kpss_uhts" | "kpss_xmas_week" => ("kpss_hol", "Holiday trading days", "Holiday"),
        "kpss_td2" | "kpss_td1_5" | "kpss_mid4" | "kpss_worst14" | "kpss_nonworst" | "kpss_six" | "kpss_monthly10" | "kpss_santa" | "kpss_m10_santa" | "kpss_umds" | "kpss_uumds" => ("kpss_month", "Days of the month", "Monthly days"),
        "kpss_dec_b" | "kpss_dec_c" | "kpss_middecade" | "kpss_latedecade" | "kpss_20yr" | "kpss_intradecade" | "kpss_road_map" => ("kpss_decade", "Decennial cycle", "Decade"),
        "kpss_c212" | "kpss_c40w" | "kpss_c53d" | "kpss_ctcm1" | "kpss_ctcm2" | "kpss_utcm" => ("kpss_cycles", "Time cycles (212-week, 40-week, 53-day)", "Cycles"),
        "kpss_nov_may" | "kpss_ndx_nov_jun" | "kpss_jja" | "kpss_sept" | "kpss_ex_sept" | "kpss_oct7" => ("kpss_may", "Sell in May and dead zones", "Season"),
        "kpss_kti_long" | "kpss_lopl" | "kpss_jusb" => ("kpss_kti", "Known trends index models", "KTI"),
        _ if id.starts_with("kpss_yr_") || matches!(id, "kpss_stoken15" | "kpss_fav14" | "kpss_fav21" | "kpss_midterm" | "kpss_pre_mar_jul" | "kpss_uecs") => ("kpss_elec", "Election cycle", "Election"),
        _ => ("kpss_other", "Kaeppel statistics", ""),
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let lev = || p("lev", "Leverage of the 2:1 positions", 2., 1., 3., 0.5);
    let mid = || p("mid_from", "Days 9-12 added from year (Dec 31 of the prior year)", 1980., 1900., 2010., 1.);
    let elec = || p("literal", "Election day = first Tuesday of November (1, book) / Tuesday after first Monday (0)", 1., 0., 1., 1.);
    Some(match id {
        "kpss_first5" | "kpss_last5" | "kpss_janbar" | "kpss_jbs" => vec![],
        "kpss_jaynewary" => vec![p("reading", "JayNewary reading that puts you long Feb-Dec", 3., 0., 3., 1.)],
        "kpss_ujbs" => vec![lev()],
        "kpss_hol_d1" | "kpss_hol_d2" | "kpss_hol_top24" | "kpss_xmas_week" => vec![],
        "kpss_hol_3x3" => vec![p("before", "Trading days before each holiday", 3., 1., 5., 1.), p("after", "Trading days after each holiday", 3., 0., 5., 1.)],
        "kpss_uhts" => vec![lev()],
        "kpss_td2" | "kpss_td1_5" | "kpss_mid4" | "kpss_worst14" | "kpss_nonworst" | "kpss_six" | "kpss_santa" => vec![],
        "kpss_monthly10" | "kpss_m10_santa" | "kpss_umds" => vec![mid()],
        "kpss_uumds" => vec![mid(), lev()],
        "kpss_dec_b" | "kpss_dec_c" | "kpss_middecade" | "kpss_latedecade" | "kpss_20yr" | "kpss_intradecade" => vec![],
        "kpss_c212" => vec![p("period", "Cycle length (calendar days)", 1484., 1400., 1600., 7.), p("months", "Bullish months after each start", 6., 3., 12., 1.)],
        "kpss_c40w" => vec![p("period", "Cycle length (calendar days)", 280., 210., 350., 7.), p("bull", "Bullish phase (calendar days)", 140., 70., 210., 7.)],
        "kpss_c53d" => vec![p("period", "Cycle length (calendar days)", 53., 40., 70., 1.), p("bull", "Bullish phase (calendar days)", 27., 15., 40., 1.)],
        "kpss_ctcm1" | "kpss_ctcm2" => vec![],
        "kpss_utcm" => vec![lev()],
        "kpss_yr_post" | "kpss_yr_mid" | "kpss_yr_pre" | "kpss_yr_elec" | "kpss_yr_post_mid" | "kpss_yr_pre_elec" | "kpss_stoken15" | "kpss_fav14" | "kpss_fav21" | "kpss_pre_mar_jul" => vec![],
        "kpss_midterm" => vec![p("before", "Trading days before the midterm election", 5., 1., 10., 1.), p("after", "Trading days after", 3., 1., 10., 1.), elec()],
        "kpss_uecs" => vec![lev(), elec()],
        "kpss_nov_may" => vec![p("may_day", "Sell at the close of May trading day", 3., 1., 10., 1.)],
        "kpss_ndx_nov_jun" | "kpss_jja" | "kpss_sept" | "kpss_ex_sept" | "kpss_oct7" => vec![],
        "kpss_kti_long" => vec![p("min_kti", "Hold when KTI >= ", 3., 1., 6., 1.), elec()],
        "kpss_lopl" => vec![p("min_kti", "Hold when KTI >= ", 3., 1., 6., 1.), p("lev_kti", "2:1 leverage when KTI >= ", 5., 3., 8., 1.), lev(), elec()],
        "kpss_jusb" => vec![p("min_kti", "Hold when KTI >= ", 3., 1., 6., 1.), p("lev_kti", "2:1 leverage when KTI >= ", 5., 3., 8., 1.), p("short_kti", "Short when KTI < ", 2., -1., 4., 1.), lev(), elec()],
        _ => return None,
    })
}

const TOP24: [(usize, i64); 24] = [
    (7, -3), (7, -2), (7, -1), (7, 1), (7, 2), (7, 3),
    (0, -2), (0, 3),
    (1, -2), (1, -1), (1, 2),
    (2, -1),
    (3, -2), (3, -1), (3, 1), (3, 2), (3, 3),
    (4, -1),
    (5, -2), (5, -1), (5, 1),
    (6, -1), (6, 1), (6, 3),
];

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    let n = x.r.len();
    if n == 0 || from > to || to >= n {
        return Some(vec![]);
    }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(0.0, |d| d.def));
    let u = |k: &str| g(k).round().max(0.0) as usize;
    let cal = Cal::new(x);
    let ix = Ix { c: &cal, close: &x.c, n };
    let lev = g("lev");
    let literal = g("literal") >= 0.5;
    let ymd = |y: i64, m: u32, d: u32| days_from_civil(y, m, d);
    let w: Vec<f64> = match id {
        "kpss_first5" | "kpss_last5" | "kpss_janbar" | "kpss_jaynewary" | "kpss_jbs" | "kpss_ujbs" => {
            let mut w = ix.zeros();
            for y in cal.years() {
                let jb = ix.january(y);
                if id == "kpss_jbs" || id == "kpss_ujbs" {
                    // January is always held (as soon as the prior December close exists)
                    ix.jan(&mut w, y, 1.0);
                }
                let Some((f5, l5, jan)) = jb else { continue };
                let r = f5 as i64 + l5 as i64 + jan as i64;
                let v = match id {
                    "kpss_first5" => f5 as i64 as f64,
                    "kpss_last5" => l5 as i64 as f64,
                    "kpss_janbar" | "kpss_jbs" => jan as i64 as f64,
                    "kpss_jaynewary" => (r == u("reading") as i64) as i64 as f64,
                    _ => {
                        if r == 3 { lev } else if r == 2 || jan { 1.0 } else if r == 0 { -1.0 } else { 0.0 }
                    }
                };
                if v != 0.0 {
                    ix.feb_dec(&mut w, y, v);
                }
            }
            w
        }
        "kpss_hol_d1" => ix.hol(&[-1]),
        "kpss_hol_d2" => ix.hol(&[-2, -1]),
        "kpss_hol_3x3" => {
            let (b, a) = (u("before").min(3) as i64, u("after").min(3) as i64);
            let offs: Vec<i64> = (1..=b).map(|k| -k).chain(1..=a).collect();
            ix.hol(&offs)
        }
        "kpss_hol_top24" => (0..n).map(|i| if cal.hol[i].iter().any(|h| TOP24.contains(h)) { 1.0 } else { 0.0 }).collect(),
        "kpss_uhts" => (0..n)
            .map(|i| cal.hol[i].iter().map(|&(_, o)| if o == -1 { lev } else { 1.0 }).fold(0.0, f64::max))
            .collect(),
        "kpss_xmas_week" => cal.xmas_week.iter().map(|&b| b as i64 as f64).collect(),
        "kpss_td2" => ix.tdm(&[2], &[]),
        "kpss_td1_5" => ix.tdm(&[1, 2, 3, 4, 5], &[]),
        "kpss_mid4" => ix.tdm(&[9, 10, 11, 12], &[]),
        "kpss_six" => ix.tdm(&[1, 2, 3, 4], &[-2, -1]),
        "kpss_worst14" | "kpss_nonworst" => {
            let worst = ix.tdm(&[6, 7, 8, 13, 14, 15, 16, 17, 18, 19, 20], &[-7, -6, -5]);
            if id == "kpss_worst14" { worst } else { worst.iter().map(|v| 1.0 - v).collect() }
        }
        "kpss_santa" => cal.santa.iter().map(|&b| b as i64 as f64).collect(),
        "kpss_monthly10" | "kpss_m10_santa" | "kpss_umds" | "kpss_uumds" => {
            let six = ix.tdm(&[1, 2, 3, 4], &[-2, -1]);
            let mid4 = ix.tdm(&[9, 10, 11, 12], &[]);
            let start = ymd(g("mid_from").round() as i64 - 1, 12, 31);
            let mut w: Vec<f64> = (0..n).map(|i| if six[i] > 0.0 || (mid4[i] > 0.0 && cal.dn[i] > start) { 1.0 } else { 0.0 }).collect();
            if id != "kpss_monthly10" {
                for i in 0..n {
                    if cal.santa[i] {
                        w[i] = 1.0;
                    }
                }
            }
            if id == "kpss_umds" || id == "kpss_uumds" {
                let v = if id == "kpss_uumds" { lev } else { 1.0 };
                for i in 0..n {
                    if cal.hol[i].iter().any(|&(_, o)| o == -1) {
                        w[i] = v;
                    }
                }
            }
            w
        }
        "kpss_dec_b" => ix.years(|y| ![0, 1, 7].contains(&y.rem_euclid(10))),
        "kpss_dec_c" => ix.years(|y| [0, 1, 7].contains(&y.rem_euclid(10))),
        "kpss_middecade" | "kpss_latedecade" | "kpss_20yr" | "kpss_intradecade" => {
            let mut w = ix.zeros();
            for y in cal.years() {
                let (dy, dec) = (y.rem_euclid(10), y.div_euclid(10));
                if dy == 4 && (id == "kpss_middecade" || id == "kpss_intradecade") {
                    cal.mark(&mut w, ymd(y, 9, 30), ymd(y + 2, 3, 31), 1.0);
                }
                if dy == 8 && (id == "kpss_latedecade" || id == "kpss_intradecade") {
                    cal.mark(&mut w, month_end(y, 2), ymd(y + 1, 9, 30), 1.0);
                }
                if dy == 2 && dec.rem_euclid(2) == 0 && (id == "kpss_20yr" || id == "kpss_intradecade") {
                    cal.mark(&mut w, ymd(y, 9, 30), dec31(y + 3), 1.0);
                }
            }
            w
        }
        "kpss_c212" => ix.cycle(ymd(1938, 5, 16), u("period").max(1) as i64, u("months") as i64, true),
        "kpss_c40w" => ix.cycle(ymd(1967, 4, 21), u("period").max(1) as i64, u("bull") as i64, false),
        "kpss_c53d" => ix.cycle(ymd(1967, 3, 22), u("period").max(1) as i64, u("bull") as i64, false),
        "kpss_ctcm1" | "kpss_ctcm2" | "kpss_utcm" => {
            let a = ix.cycle(ymd(1938, 5, 16), 1484, 6, true);
            let b = ix.cycle(ymd(1967, 4, 21), 280, 140, false);
            let c = ix.cycle(ymd(1967, 3, 22), 53, 27, false);
            (0..n)
                .map(|i| {
                    let k = a[i] + b[i] + c[i];
                    match id {
                        "kpss_ctcm1" => (k >= 1.0) as i64 as f64,
                        "kpss_ctcm2" => (k >= 2.0) as i64 as f64,
                        _ => {
                            if k >= 2.0 { lev } else if a[i] + b[i] >= 1.0 { 1.0 } else { 0.0 }
                        }
                    }
                })
                .collect()
        }
        "kpss_yr_post" => ix.years(|y| y.rem_euclid(4) == 1),
        "kpss_yr_mid" => ix.years(|y| y.rem_euclid(4) == 2),
        "kpss_yr_pre" => ix.years(|y| y.rem_euclid(4) == 3),
        "kpss_yr_elec" => ix.years(|y| y.rem_euclid(4) == 0),
        "kpss_yr_post_mid" => ix.years(|y| matches!(y.rem_euclid(4), 1 | 2)),
        "kpss_yr_pre_elec" => ix.years(|y| matches!(y.rem_euclid(4), 3 | 0)),
        "kpss_stoken15" | "kpss_fav14" | "kpss_fav21" | "kpss_pre_mar_jul" => {
            let mut w = ix.zeros();
            for y in cal.years() {
                match y.rem_euclid(4) {
                    2 if id == "kpss_stoken15" => cal.mark(&mut w, ymd(y, 9, 30), dec31(y + 1), 1.0),
                    2 if id == "kpss_fav14" || id == "kpss_fav21" => {
                        cal.mark(&mut w, ymd(y, 9, 30), ymd(y + 1, 9, 30), 1.0);
                        cal.mark(&mut w, ymd(y + 1, 10, 31), dec31(y + 1), 1.0);
                    }
                    0 if id == "kpss_fav21" => cal.mark(&mut w, ymd(y, 5, 31), dec31(y), 1.0),
                    3 if id == "kpss_pre_mar_jul" => cal.mark(&mut w, month_end(y, 2), ymd(y, 7, 31), 1.0),
                    _ => {}
                }
            }
            w
        }
        "kpss_midterm" => ix.midterm(u("before"), u("after"), literal),
        "kpss_uecs" => {
            let mut w = ix.zeros();
            for y in cal.years() {
                match y.rem_euclid(4) {
                    2 => {
                        cal.mark(&mut w, ymd(y, 9, 30), ymd(y + 1, 9, 30), 1.0);
                        cal.mark(&mut w, ymd(y + 1, 10, 31), dec31(y + 1), 1.0);
                    }
                    0 => cal.mark(&mut w, ymd(y, 5, 31), dec31(y), 1.0),
                    1 if y.rem_euclid(10) == 5 => cal.mark(&mut w, dec31(y - 1), dec31(y), 1.0),
                    _ => {}
                }
            }
            for y in cal.years() {
                if y.rem_euclid(4) == 3 {
                    cal.mark(&mut w, month_end(y, 2), ymd(y, 7, 31), lev);
                }
            }
            let mt = ix.midterm(5, 3, literal);
            for i in 0..n {
                if mt[i] > 0.0 {
                    w[i] = lev;
                }
            }
            w
        }
        "kpss_nov_may" => {
            let md = u("may_day").max(1) as i64;
            (0..n).map(|i| if matches!(cal.m[i], 11 | 12 | 1 | 2 | 3 | 4) || (cal.m[i] == 5 && cal.tdf[i] <= md) { 1.0 } else { 0.0 }).collect()
        }
        "kpss_ndx_nov_jun" => (0..n).map(|i| if !matches!(cal.m[i], 7..=10) { 1.0 } else { 0.0 }).collect(),
        "kpss_jja" => (0..n).map(|i| matches!(cal.m[i], 6..=8) as i64 as f64).collect(),
        "kpss_sept" => (0..n).map(|i| (cal.m[i] == 9) as i64 as f64).collect(),
        "kpss_ex_sept" => (0..n).map(|i| (cal.m[i] != 9) as i64 as f64).collect(),
        "kpss_oct7" => (0..n).map(|i| (cal.m[i] == 10 && cal.y[i].rem_euclid(10) == 7) as i64 as f64).collect(),
        "kpss_kti_long" | "kpss_lopl" | "kpss_jusb" => {
            let k = kti(&cal, &ix, literal);
            let (lo, hi) = (g("min_kti"), g("lev_kti"));
            k.iter()
                .map(|&v| match id {
                    "kpss_kti_long" => (v >= lo) as i64 as f64,
                    "kpss_lopl" => if v >= hi { lev } else if v >= lo { 1.0 } else { 0.0 },
                    _ => if v >= hi { lev } else if v >= lo { 1.0 } else if v < g("short_kti") { -1.0 } else { 0.0 },
                })
                .collect()
        }
        _ => return None,
    };
    Some(trades(x, &w, from, to))
}

/// Known trends index (Table 9.1, p.270) for every bar.
fn kti(cal: &Cal, ix: &Ix, literal: bool) -> Vec<f64> {
    let n = ix.n;
    let ymd = days_from_civil;
    let mut k = ix.tdm(&[1, 2, 3, 4, 9, 10, 11, 12], &[-2, -1]);
    let add = |k: &mut Vec<f64>, w: &[f64]| {
        for i in 0..n {
            k[i] += w[i];
        }
    };
    let nov_may: Vec<f64> = (0..n).map(|i| (matches!(cal.m[i], 11 | 12 | 1 | 2 | 3 | 4) || (cal.m[i] == 5 && cal.tdf[i] <= 3)) as i64 as f64).collect();
    add(&mut k, &nov_may);
    let summer: Vec<f64> = (0..n).map(|i| ((cal.m[i] == 6 && cal.tdb[i] != 0 && cal.tdb[i] >= -3) || (cal.m[i] == 7 && cal.tdf[i] <= 9)) as i64 as f64).collect();
    add(&mut k, &summer);
    let sept: Vec<f64> = (0..n).map(|i| -((cal.m[i] == 9) as i64 as f64)).collect();
    add(&mut k, &sept);
    let (mut elec, mut marjul, mut mid, mut late, mut twenty) = (ix.zeros(), ix.zeros(), ix.zeros(), ix.zeros(), ix.zeros());
    for y in cal.years() {
        match y.rem_euclid(4) {
            2 => {
                cal.mark(&mut elec, ymd(y, 9, 30), ymd(y + 1, 9, 30), 1.0);
                cal.mark(&mut elec, ymd(y + 1, 10, 31), dec31(y + 1), 1.0);
            }
            0 => cal.mark(&mut elec, ymd(y, 5, 31), dec31(y), 1.0),
            3 => cal.mark(&mut marjul, month_end(y, 2), ymd(y, 7, 31), 1.0),
            _ => {}
        }
        let (dy, dec) = (y.rem_euclid(10), y.div_euclid(10));
        if dy == 4 {
            cal.mark(&mut mid, ymd(y, 9, 30), ymd(y + 2, 3, 31), 1.0);
        }
        if dy == 8 {
            cal.mark(&mut late, month_end(y, 2), ymd(y + 1, 9, 30), 1.0);
        }
        if dy == 2 && dec.rem_euclid(2) == 0 {
            cal.mark(&mut twenty, ymd(y, 9, 30), dec31(y + 3), 1.0);
        }
    }
    for w in [&elec, &marjul, &mid, &late, &twenty] {
        add(&mut k, w);
    }
    add(&mut k, &ix.midterm(5, 3, literal));
    add(&mut k, &ix.cycle(ymd(1967, 4, 21), 280, 140, false));
    add(&mut k, &ix.cycle(ymd(1938, 5, 16), 1484, 6, true));
    add(&mut k, &ix.hol(&[-3, -2, -1, 1, 2, 3]));
    k
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;

    #[test]
    fn calendar_dates_match_the_book() {
        let iso = |z: i64| {
            let (y, m, d) = civil_from_days(z);
            format!("{y:04}-{m:02}-{d:02}")
        };
        // Good Friday 1995 = Apr 14 (the 40-week table's 4/13/95 buy is the prior trading day)
        assert_eq!(iso(easter(1995) - 2), "1995-04-14");
        assert_eq!(iso(holidays(2007)[5]), "2007-11-22");
        assert_eq!(iso(holidays(2010)[6]), "2010-12-24"); // Christmas Saturday -> Friday
        // 212-week cycle: 5/16/1938 + 1484 days = 6/8/1942 (Table 6.1); exit six months after 7/4/2011 = 1/4/2012 (Table 6.2)
        assert_eq!(iso(days_from_civil(1938, 5, 16) + 1484), "1942-06-08");
        assert_eq!(iso(plus_months(days_from_civil(2011, 7, 4), 6)), "2012-01-04");
        // 40-week: 4/21/67 + 280 days = 1/26/68 (Table 6.3); 53-day: 3/22/67 + 27 = 4/18/67 (Table 6.4)
        assert_eq!(iso(days_from_civil(1967, 4, 21) + 280), "1968-01-26");
        assert_eq!(iso(days_from_civil(1967, 3, 22) + 27), "1967-04-18");
        assert_eq!(iso(election_day(1994, false)), "1994-11-08");
        assert_eq!(iso(election_day(1994, true)), "1994-11-01");
    }

    #[test]
    fn second_trading_day_trade_is_close_of_day1_to_close_of_day2() {
        let d = ["2007-01-29", "2007-01-30", "2007-01-31", "2007-02-01", "2007-02-02", "2007-02-05", "2007-02-06"];
        let r: Vec<Row> = d.iter().enumerate().map(|(k, d)| Row { ticker: "T".into(), date: d.to_string(), open: 100. + k as f64, high: 101. + k as f64, low: 99. + k as f64, close: 100. + k as f64, volume: None }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let t = run("kpss_td2", &x, 0, 6, &Params::new()).unwrap();
        assert_eq!(t.len(), 2); // Jan 30 (2nd bar of the data's January) and Feb 2
        assert_eq!((t[1].legs[0].i, t[1].exit_i, t[1].open), (3, 4, false)); // Feb 1 close -> Feb 2 close
    }
}
