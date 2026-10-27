//! Order-flow footprint data for x_orderflow.rs: per session date and price level, the volume traded at the bid (aggressive
//! sells) and at the ask (aggressive buys). One file per ticker, `$VAULT_ORDERFLOW_DIR/<TICKER, spaces as _>__FOOTPRINT.csv`
//! with columns `date,price,bid_volume,ask_volume` (any order; library/orderflow/footprint_from_trades.py writes it from trades).
//! Price levels map to one grid per file: step = the smallest gap between distinct prices, k = round((p - p_min) / step).
//! Duplicate (date, level) rows are summed; malformed rows are skipped and counted; a negative or non-finite volume or a
//! non-positive price rejects the whole file (nothing is clamped). Files are read once per engine start.
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

/// One session: grid rows sorted by k, no duplicates.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Day { pub k: Vec<i64>, pub b: Vec<f64>, pub a: Vec<f64> }
#[derive(Debug)]
pub struct File { pub step: f64, pub p_min: f64, days: Vec<Day>, by_date: HashMap<String, usize>, level_price: HashMap<i64, f64> }
/// A file aligned to a series of daily bars (None = no footprint that day).
#[derive(Clone, Debug)]
pub struct Footprint { file: Arc<File>, idx: Vec<Option<usize>> }

impl Footprint {
    pub fn align(file: Arc<File>, dates: &[String]) -> Footprint {
        let idx = dates.iter().map(|d| file.by_date.get(d).copied()).collect();
        Footprint { file, idx }
    }
    pub fn day(&self, i: usize) -> Option<&Day> { self.idx.get(i).copied().flatten().map(|j| &self.file.days[j]) }
    /// The price written in the file for row k (rows inside a session's range without a file row: the grid price).
    pub fn price(&self, k: i64) -> f64 { self.file.level_price.get(&k).copied().unwrap_or(self.file.p_min + k as f64 * self.file.step) }
    /// The first `n` bars only (causality tests cut the bars and their footprint together).
    pub fn truncate(&mut self, n: usize) { self.idx.truncate(n); }
}

impl File {
    /// (date YYYY-MM-DD, price, bid volume, ask volume) rows.
    pub fn from_rows(rows: &[(String, f64, f64, f64)]) -> Result<File, String> {
        if let Some(r) = rows.iter().find(|r| !(r.1.is_finite() && r.1 > 0.)) { return Err(format!("price {} on {} is not a positive number", r.1, r.0)); }
        if let Some(r) = rows.iter().find(|r| !(r.2.is_finite() && r.3.is_finite() && r.2 >= 0. && r.3 >= 0.)) {
            return Err(format!("volume on {} at {} is negative or not a number", r.0, r.1));
        }
        let mut prices: Vec<f64> = rows.iter().map(|r| r.1).collect();
        prices.sort_by(f64::total_cmp);
        let p_min = prices.first().copied().unwrap_or(0.);
        let step = prices.windows(2).map(|w| w[1] - w[0]).filter(|g| *g > 1e-9 * p_min.abs().max(1.)).fold(f64::INFINITY, f64::min);
        let step = if step.is_finite() { step } else { 1. };
        let mut map: HashMap<String, std::collections::BTreeMap<i64, (f64, f64)>> = HashMap::new();
        let mut level_price = HashMap::new();
        for (d, p, b, a) in rows {
            let k = ((p - p_min) / step).round() as i64;
            level_price.entry(k).or_insert(*p);
            let e = map.entry(d.clone()).or_default().entry(k).or_insert((0., 0.));
            e.0 += b; e.1 += a;
        }
        let (mut days, mut by_date) = (Vec::new(), HashMap::new());
        for (d, levels) in map {
            by_date.insert(d, days.len());
            days.push(Day { k: levels.keys().copied().collect(), b: levels.values().map(|v| v.0).collect(), a: levels.values().map(|v| v.1).collect() });
        }
        Ok(File { step, p_min, days, by_date, level_price })
    }
    /// CSV text → file and the number of malformed rows skipped.
    pub fn parse(text: &str) -> Result<(File, usize), String> {
        let mut lines = text.lines();
        let head: Vec<String> = lines.next().ok_or("empty file")?.split(',').map(|h| h.trim().to_ascii_lowercase()).collect();
        let col = |n: &str| head.iter().position(|h| h == n).ok_or_else(|| format!("missing column {n}"));
        let (di, pi, bi, ai) = (col("date")?, col("price")?, col("bid_volume")?, col("ask_volume")?);
        let (mut rows, mut skipped) = (Vec::new(), 0);
        for line in lines.filter(|l| !l.trim().is_empty()) {
            let f: Vec<&str> = line.split(',').map(str::trim).collect();
            let num = |i: usize| f.get(i).and_then(|v| v.parse::<f64>().ok());
            match (f.get(di).and_then(|d| crate::normalize_date(d)), num(pi), num(bi), num(ai)) {
                (Some(d), Some(p), Some(b), Some(a)) => rows.push((d, p, b, a)),
                _ => skipped += 1,
            }
        }
        Ok((File::from_rows(&rows)?, skipped))
    }
}

pub fn file_name(ticker: &str) -> String { format!("{}__FOOTPRINT.csv", ticker.replace(' ', "_")) }

static CACHE: OnceLock<Mutex<HashMap<PathBuf, Option<Arc<File>>>>> = OnceLock::new();
fn read(path: &Path) -> Option<Arc<File>> {
    let text = std::fs::read_to_string(path).ok()?;
    match File::parse(&text) {
        Ok((file, skipped)) => {
            if skipped > 0 { eprintln!("order-flow: {}: skipped {skipped} malformed rows", path.display()); }
            Some(Arc::new(file))
        }
        Err(e) => { eprintln!("order-flow: {} rejected: {e}", path.display()); None }
    }
}
/// The ticker's footprint aligned to `dates`, or None (no VAULT_ORDERFLOW_DIR, no file, or a rejected file).
pub fn load(ticker: &str, dates: &[String]) -> Option<Footprint> {
    if ticker.is_empty() || ticker.contains('/') || ticker.contains('\\') || ticker.contains("..") { return None; }
    let dir = std::env::var_os("VAULT_ORDERFLOW_DIR")?;
    let path = Path::new(&dir).join(file_name(ticker));
    let file = CACHE.get_or_init(Default::default).lock().ok()?.entry(path.clone()).or_insert_with(|| read(&path)).clone()?;
    Some(Footprint::align(file, dates))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn parses_sums_duplicates_and_builds_one_grid() {
        let text = "date,price,bid_volume,ask_volume\n2024-01-02,100.10,5,1\n2024-01-02,100.00,2,3\n2024-01-02,100.10,1,1\nbad,row,,\n02-01-2024,100.20,0,4\n2024-01-03,100.05,1,0\n";
        let (f, skipped) = File::parse(text).unwrap();
        assert_eq!(skipped, 1);
        assert!((f.step - 0.05).abs() < 1e-9 && f.p_min == 100.);
        let fp = Footprint::align(Arc::new(f), &["2024-01-01".into(), "2024-01-02".into(), "2024-01-03".into()]);
        assert!(fp.day(0).is_none());
        // 02-01-2024 is dd-mm-yyyy → 2024-01-02, summed into the same session
        assert_eq!(fp.day(1).unwrap(), &Day { k: vec![0, 2, 4], b: vec![2., 6., 0.], a: vec![3., 2., 4.] });
        assert_eq!(fp.day(2).unwrap().k, vec![1]);
        assert_eq!(fp.price(4), 100.2, "the file's own price, not p_min + k × step");
    }
    #[test] fn bad_volumes_reject_the_file() {
        assert!(File::parse("date,price,bid_volume,ask_volume\n2024-01-02,100,-1,1\n").is_err());
        assert!(File::parse("date,price,bid_volume,ask_volume\n2024-01-02,0,1,1\n").is_err());
        assert!(File::parse("date,price,bid\n").is_err());
    }
    #[test] fn unsafe_ticker_names_never_reach_the_filesystem() {
        assert!(load("../etc", &[]).is_none() && load("a/b", &[]).is_none());
        assert_eq!(file_name("NIFTY 50"), "NIFTY_50__FOOTPRINT.csv");
    }
}
