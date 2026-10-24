//! Intraday phase 1B: real 1-minute bars, read one day at a time.
//!
//! Data: `<dir>/<TICKER>.m1.csv` (Date,Time,Open,High,Low,Close; Time = start of the minute, regular session only) and
//! `<dir>/<TICKER>.m1idx.csv` (Date,offset,bytes), built offline by library/intraday/m1_normalise.py. Padded, no-open
//! (NULL) and stale-open minutes are already removed there, so no such minute can ever produce a fill. Days the builder
//! excluded (outage, late start, >5% minutes dropped, ...) are absent: a rule needing that day's path makes no trade.
//! Memory: only the day index is held (about 6k lines per ticker); each `day()` call seeks and parses one day (~20 KB).
use std::collections::BTreeMap;
use std::io::{Read, Seek, SeekFrom};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bar1 { pub m: u16, pub o: f64, pub h: f64, pub l: f64, pub c: f64 }

pub struct Minutes { idx: BTreeMap<String, (u64, usize)>, path: std::path::PathBuf }

impl Minutes {
    /// None when the ticker has no minute files (every current result then stays exactly as before).
    pub fn load(dir: &str, ticker: &str) -> Option<Minutes> {
        let t = ticker.replace(' ', "_");
        let text = std::fs::read_to_string(format!("{dir}/{t}.m1idx.csv")).ok()?;
        let idx: BTreeMap<String, (u64, usize)> = text.lines().skip(1).filter_map(|l| {
            let mut f = l.split(',');
            Some((f.next()?.to_string(), (f.next()?.parse().ok()?, f.next()?.parse().ok()?)))
        }).collect();
        (!idx.is_empty()).then(|| Minutes { idx, path: format!("{dir}/{t}.m1.csv").into() })
    }
    pub fn first(&self) -> &str { self.idx.keys().next().map_or("", |s| s.as_str()) }
    pub fn last(&self) -> &str { self.idx.keys().next_back().map_or("", |s| s.as_str()) }
    /// The day's minutes in time order, or None (no file, day excluded, or a malformed row).
    pub fn day(&self, date: &str) -> Option<Vec<Bar1>> {
        let &(off, n) = self.idx.get(date)?;
        let mut f = std::fs::File::open(&self.path).ok()?;
        f.seek(SeekFrom::Start(off)).ok()?;
        let mut buf = vec![0u8; n];
        f.read_exact(&mut buf).ok()?;
        std::str::from_utf8(&buf).ok()?.lines().map(|l| {
            let v: Vec<&str> = l.split(',').collect();
            if v.len() < 6 || v[0] != date { return None; }
            let (hh, mm) = v[1].split_once(':')?;
            let num = |s: &str| s.parse::<f64>().ok();
            Some(Bar1 { m: hh.parse::<u16>().ok()? * 60 + mm.parse::<u16>().ok()?, o: num(v[2])?, h: num(v[3])?, l: num(v[4])?, c: num(v[5])? })
        }).collect()
    }
}

/// Regular-session opening minute (start-stamped exchange clock): US 09:30 ET; NSE 09:55 before 2010-01-04,
/// 09:00 until the pre-open call auction began on 2010-10-18, 09:15 since.
pub fn session_open(ticker: &str, date: &str) -> u16 {
    if !ticker.starts_with("NIFTY") { return 9 * 60 + 30; }
    if date < "2010-01-04" { 9 * 60 + 55 } else if date < "2010-10-18" { 9 * 60 } else { 9 * 60 + 15 }
}

/// A resting stop order level: `up` = buy stop (touched when high >= level), else sell stop (low <= level).
#[derive(Clone, Copy, Debug)]
pub struct Level { pub up: bool, pub px: f64 }

#[derive(Debug, PartialEq)]
pub enum Touch {
    /// Level `k` was touched first, in bar `bar`; fill = the level, or the bar's open when the bar opened through it
    /// (fills price at the minute's open, never its close: the store README's rule).
    Hit { k: usize, bar: usize, px: f64 },
    /// One minute reached two levels and did not open through either, so their order is unknown from 1-minute OHLC.
    /// The caller applies its declared convention and the case is listed in the verification report.
    Ambiguous { bar: usize },
    None,
}

pub fn touched(b: &Bar1, lv: Level) -> bool { if lv.up { b.h >= lv.px } else { b.l <= lv.px } }
pub fn fill_px(b: &Bar1, lv: Level) -> f64 { if lv.up { b.o.max(lv.px) } else { b.o.min(lv.px) } }

/// First level touched in bars[from..].
pub fn first_touch(bars: &[Bar1], from: usize, levels: &[Level]) -> Touch {
    for (bar, b) in bars.iter().enumerate().skip(from) {
        let hit: Vec<usize> = (0..levels.len()).filter(|&k| touched(b, levels[k])).collect();
        match hit.len() {
            0 => continue,
            1 => return Touch::Hit { k: hit[0], bar, px: fill_px(b, levels[hit[0]]) },
            _ => {
                let at_open: Vec<usize> = hit.iter().copied().filter(|&k| if levels[k].up { b.o >= levels[k].px } else { b.o <= levels[k].px }).collect();
                return if at_open.len() == 1 { Touch::Hit { k: at_open[0], bar, px: b.o } } else { Touch::Ambiguous { bar } };
            }
        }
    }
    Touch::None
}

/// One day of the 15-Minute ADX Breakout (ATS Ch 20 / PTJ Day Trading) in one direction on 1-minute bars.
/// Window = the first 15 minutes from the session's opening clock. Entry stop one tick beyond the window extreme,
/// protective stop at the opposite extreme -/+ `stop_tick`; after a stop-out the same entry stop is live again from the
/// next minute ("it is correct to re-enter"), at most `cap` entries a day. A minute that both fills and reaches the stop:
/// the extreme nearer the minute's open came first [convention of the ptjdt module, fitted to the book's Examples 5 and
/// 11]; a stop-level touch while flat is ignored. Returns (fill, Some(stop exit) | None = held to the close, ambiguous).
pub fn adx15_day(b: &[Bar1], open: u16, up: bool, tick: f64, stop_tick: f64, cap: usize) -> Vec<(f64, Option<f64>, bool)> {
    let k0 = b.iter().position(|z| z.m >= open + 15).unwrap_or(b.len());
    if k0 == 0 || k0 == b.len() { return vec![]; }
    let h15 = b[..k0].iter().map(|z| z.h).fold(f64::NEG_INFINITY, f64::max);
    let l15 = b[..k0].iter().map(|z| z.l).fold(f64::INFINITY, f64::min);
    let entry = Level { up, px: if up { h15 + tick } else { l15 - tick } };
    let stop = Level { up: !up, px: if up { l15 - stop_tick } else { h15 + stop_tick } };
    let mut out: Vec<(f64, Option<f64>, bool)> = vec![];
    let mut pos: Option<(f64, bool)> = None;
    for z in &b[k0..] {
        if let Some((px, amb)) = pos {
            if touched(z, stop) {
                out.push((px, Some(fill_px(z, stop)), amb));
                pos = None;
            }
            continue; // a minute that stops a position never re-enters
        }
        if out.len() >= cap || !touched(z, entry) { continue; }
        let px = fill_px(z, entry);
        if touched(z, stop) {
            let stop_first = if up { z.o - z.l < z.h - z.o } else { z.h - z.o < z.o - z.l };
            if stop_first { pos = Some((px, true)); } else { out.push((px, Some(stop.px), true)); }
        } else {
            pos = Some((px, false));
        }
    }
    if let Some((px, amb)) = pos { out.push((px, None, amb)); }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    fn b(m: u16, o: f64, h: f64, l: f64, c: f64) -> Bar1 { Bar1 { m, o, h, l, c } }
    #[test]
    fn first_touch_orders_levels_and_fills_at_open_on_gaps() {
        let up = Level { up: true, px: 101.0 };
        let dn = Level { up: false, px: 99.0 };
        let bars = [b(0, 100., 100.5, 99.5, 100.), b(1, 100., 101.2, 99.8, 101.), b(2, 98., 98.5, 97., 98.)];
        assert_eq!(first_touch(&bars, 0, &[up, dn]), Touch::Hit { k: 0, bar: 1, px: 101.0 });
        assert_eq!(first_touch(&bars, 2, &[up, dn]), Touch::Hit { k: 1, bar: 2, px: 98.0 }); // gapped through: the open
        let both = [b(0, 100., 101.5, 98.5, 100.)];
        assert_eq!(first_touch(&both, 0, &[up, dn]), Touch::Ambiguous { bar: 0 });
        let gap = [b(0, 101.5, 101.6, 98.5, 100.)];
        assert_eq!(first_touch(&gap, 0, &[up, dn]), Touch::Hit { k: 0, bar: 0, px: 101.5 });
        assert_eq!(first_touch(&bars[..1], 0, &[up, dn]), Touch::None);
    }
    #[test]
    fn adx15_reenters_after_a_stop_and_caps_entries() {
        // window 09:30-09:44 high 101 low 99; tick 0.1 -> buy 101.1, stop 98.9
        let mut v: Vec<Bar1> = (0..15).map(|k| b(570 + k, 100., 101., 99., 100.)).collect();
        v.push(b(585, 100., 101.2, 100., 101.)); // fill 101.1
        v.push(b(586, 101., 101., 98.5, 98.6)); // stopped 98.9
        v.push(b(587, 99.5, 101.5, 99.5, 101.4)); // re-entry 101.1
        v.push(b(588, 101.4, 102., 101., 101.8)); // held to the close
        assert_eq!(adx15_day(&v, 570, true, 0.1, 0.1, 2), vec![(101.1, Some(98.9), false), (101.1, None, false)]);
        assert_eq!(adx15_day(&v, 570, true, 0.1, 0.1, 1), vec![(101.1, Some(98.9), false)]);
        // same-minute fill and stop, open nearer the high: fill then stop
        let mut w: Vec<Bar1> = (0..15).map(|k| b(570 + k, 100., 101., 99., 100.)).collect();
        w.push(b(585, 101., 101.5, 98.0, 98.2));
        assert_eq!(adx15_day(&w, 570, true, 0.1, 0.1, 1), vec![(101.1, Some(98.9), true)]);
    }
    #[test]
    fn reads_one_day_by_byte_range() {
        let dir = std::env::temp_dir().join(format!("m1test{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let body = "Date,Time,Open,High,Low,Close\n2020-01-02,09:30,1,2,0.5,1.5\n2020-01-02,09:31,1.5,2,1,1.75\n2020-01-03,09:30,3,3,3,3\n";
        std::fs::write(dir.join("NIFTY_50.m1.csv"), body).unwrap();
        std::fs::write(dir.join("NIFTY_50.m1idx.csv"), "Date,offset,bytes\n2020-01-02,30,59\n2020-01-03,89,25\n").unwrap();
        let m = Minutes::load(dir.to_str().unwrap(), "NIFTY 50").unwrap();
        assert_eq!(m.day("2020-01-02").unwrap(), vec![b(570, 1., 2., 0.5, 1.5), b(571, 1.5, 2., 1., 1.75)]);
        assert_eq!(m.day("2020-01-03").unwrap(), vec![b(570, 3., 3., 3., 3.)]);
        assert!(m.day("2020-01-06").is_none());
        assert_eq!((m.first(), m.last()), ("2020-01-02", "2020-01-03"));
        std::fs::remove_dir_all(dir).unwrap();
    }
}
