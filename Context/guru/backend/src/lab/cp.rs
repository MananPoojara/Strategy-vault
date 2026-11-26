//! Dated index membership from the research team's Current & Past (C&P) AmiBroker databases (share DATABASE/Database
//! Directory.xlsx): AUX1 = 1 while a stock is in the index, AUX2 = 1 while it is in the NSE F&O list. A portfolio on one
//! of these universes holds every stock that was ever in the list and buys a stock only on days it is in the list (the
//! AFLs' `Buy = Buy AND Aux1 == 1`); a held stock that leaves keeps its own exit (user, 20-11-2026). `<id>-fo` also needs
//! AUX2 = 1. Data: library/cp-aux/scripts/convert_cp.py, mounted read-only at VAULT_CP_DATA_DIR. Book strategies and the
//! snapshot baskets (baskets.rs) do not read this.
use super::{bars::Bars, rules::LabError};
use crate::Row;
use std::{collections::BTreeMap, path::{Path, PathBuf}, sync::OnceLock};

pub struct Stock { pub ticker: String, pub path: PathBuf, pub first: String, pub last: String, pub bars: usize, pub periods: Vec<(String, String)> }
pub struct List { pub label: String, pub stocks: Vec<Stock> }
pub struct Catalogue { pub lists: BTreeMap<String, List> }

static CP: OnceLock<Option<Catalogue>> = OnceLock::new();
/// The C&P catalogue, read once from VAULT_CP_DATA_DIR; None when the engine has no such data (US, gl, tests).
pub fn catalogue() -> Option<&'static Catalogue> {
    CP.get_or_init(|| std::env::var("VAULT_CP_DATA_DIR").ok().and_then(|d| match Catalogue::load(Path::new(&d)) {
        Ok(c) => Some(c),
        Err(e) => { eprintln!("C&P membership data unavailable: {e}"); None }
    })).as_ref()
}

/// The broad C&P list AmiBroker's Aux1 reads in the team's portfolio databases (memory ami-aux1-is-nse500).
pub const N500: &str = "nse500_cp";
/// `nifty50_cp-n500-fo` -> (`nifty50_cp`, true, true): `-n500` buys the list's stocks on the days they are in NSE 500
/// (the AmiBroker watchlist "Nifty 50 C&P" on the broad database, whose Aux1 is NSE 500 membership), `-fo` only while in F&O.
pub fn split(id: &str) -> (&str, bool, bool) {
    let (rest, fo) = id.strip_suffix("-fo").map_or((id, false), |base| (base, true));
    let (base, n500) = rest.strip_suffix("-n500").map_or((rest, false), |base| (base, true));
    (base, n500, fo)
}

/// Every list the engine offers: each C&P list, and each index list also restricted to F&O stocks.
pub fn universes() -> Vec<(String, String, usize)> {
    let Some(c) = catalogue() else { return vec![] };
    let mut out = vec![];
    for (id, list) in &c.lists {
        out.push((id.clone(), list.label.clone(), list.stocks.len()));
        if id != "fo_cp" && c.lists.contains_key("fo_cp") { out.push((format!("{id}-fo"), format!("{} · also in F&O", list.label), list.stocks.len())); }
        if id != "fo_cp" && id != N500 && c.lists.contains_key(N500) { out.push((format!("{id}-n500"), format!("{} names · bought while in NSE 500 (AmiBroker Aux1)", list.label), list.stocks.len())); }
    }
    out
}

fn field<'a>(header: &[&str], cols: &'a [&str], name: &str) -> Result<&'a str, String> {
    header.iter().position(|h| *h == name).and_then(|i| cols.get(i)).copied().ok_or_else(|| format!("C&P index missing {name}"))
}
fn parse_periods(text: &str) -> Result<Vec<(String, String)>, String> {
    text.split(';').filter(|p| !p.is_empty()).map(|p| p.split_once("..").filter(|(a, b)| crate::baskets::valid_date(a) && crate::baskets::valid_date(b) && a <= b)
        .map(|(a, b)| (a.to_string(), b.to_string())).ok_or_else(|| format!("C&P period {p} is not from..to"))).collect()
}

impl Catalogue {
    pub fn load(dir: &Path) -> Result<Self, String> {
        let root = dir.canonicalize().map_err(|_| "C&P data directory unavailable")?;
        let text = std::fs::read_to_string(root.join("index.csv")).map_err(|_| "C&P index.csv unavailable")?;
        let mut lines = text.lines();
        let header: Vec<&str> = lines.next().ok_or("C&P index.csv is empty")?.split(',').collect();
        let mut lists: BTreeMap<String, List> = BTreeMap::new();
        for line in lines.filter(|l| !l.trim().is_empty()) {
            let cols: Vec<&str> = line.split(',').collect(); // the converter writes no quoted fields
            let f = |n: &str| field(&header, &cols, n);
            let (id, ticker, rel) = (f("universe")?, f("ticker")?, f("path")?);
            if !ticker.bytes().all(|c| c.is_ascii_alphanumeric() || b"&.-_".contains(&c)) || ticker.is_empty() { return Err(format!("C&P ticker {ticker} is not a plain symbol")); }
            let path = root.join(rel).canonicalize().map_err(|_| format!("C&P prices for {ticker} unavailable"))?;
            if !path.starts_with(&root) { return Err(format!("C&P prices for {ticker} escape the data directory")); }
            let periods = parse_periods(f("periods")?)?;
            if periods.is_empty() { return Err(format!("C&P {id} {ticker} has no member period")); }
            let list = lists.entry(id.to_string()).or_insert_with(|| List { label: String::new(), stocks: vec![] });
            list.label = f("label")?.to_string();
            list.stocks.push(Stock { ticker: ticker.into(), path, first: f("first")?.into(), last: f("last")?.into(), bars: f("bars")?.parse().map_err(|_| "C&P bars is not a count")?, periods });
        }
        if lists.is_empty() { return Err("C&P index.csv lists no stock".into()); }
        Ok(Self { lists })
    }
}

impl Stock {
    /// True when a member period overlaps from..=to (the stocks a window can buy).
    pub fn member_within(&self, from: &str, to: &str) -> bool { within(&self.periods, from, to) }
    /// Daily bars and the F&O flag per bar (AUX2), from the converted CSV.
    pub fn read(&self) -> Result<(Bars, Vec<bool>), LabError> {
        let text = std::fs::read_to_string(&self.path).map_err(|_| LabError::NeedsData(format!("{}: C&P prices unavailable", self.ticker)))?;
        let rows: Vec<Row> = crate::baskets::parse_rows(&text, &self.ticker).map_err(|e| LabError::NeedsData(format!("{}: {e}", self.ticker)))?;
        let mut lines = text.lines();
        let aux2 = lines.next().and_then(|h| h.split(',').position(|c| c == "aux2")).ok_or_else(|| LabError::NeedsData(format!("{}: C&P prices lack aux2", self.ticker)))?;
        let fo: Vec<bool> = lines.filter(|l| !l.trim().is_empty()).map(|l| l.split(',').nth(aux2).and_then(|v| v.parse::<f64>().ok()) == Some(1.)).collect();
        if fo.len() != rows.len() { return Err(LabError::NeedsData(format!("{}: C&P aux2 does not line up with the prices", self.ticker))); }
        Ok((Bars::from_rows(&rows), fo))
    }
    /// Per bar of `dates` (the stock's own sessions): in the list that day, and in F&O too when `fo` is given.
    pub fn eligible(&self, dates: &[String], fo: Option<&[bool]>) -> Vec<bool> { eligible(&self.periods, dates, fo) }
}
/// True when a period overlaps from..=to.
pub fn within(periods: &[(String, String)], from: &str, to: &str) -> bool { periods.iter().any(|(a, b)| a.as_str() <= to && b.as_str() >= from) }
/// Per bar of `dates`: inside one of `periods`, and in F&O too when `fo` is given.
pub fn eligible(periods: &[(String, String)], dates: &[String], fo: Option<&[bool]>) -> Vec<bool> {
    dates.iter().enumerate().map(|(k, d)| periods.iter().any(|(a, b)| a <= d && d <= b) && fo.is_none_or(|f| f[k])).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn stock(periods: &str) -> Stock { Stock { ticker: "YESBANK".into(), path: PathBuf::new(), first: String::new(), last: String::new(), bars: 0, periods: parse_periods(periods).unwrap() } }
    #[test]
    fn a_stock_is_eligible_only_inside_its_member_periods_and_fo_narrows_it() {
        // YESBANK in the NIFTY 50 C&P database: AUX1 = 1 from 2015-03-27 to 2020-03-18
        let s = stock("2015-03-27..2020-03-18");
        let d: Vec<String> = ["2015-03-26", "2015-03-27", "2020-03-18", "2020-03-19"].map(String::from).into();
        assert_eq!(s.eligible(&d, None), [false, true, true, false]);
        assert_eq!(s.eligible(&d, Some(&[true, false, true, true])), [false, false, true, false]);
        assert!(s.member_within("2019-01-01", "2026-10-16") && !s.member_within("2020-03-19", "2026-10-16"));
        assert_eq!(stock("2001-01-01..2002-01-01;2010-01-01..2011-01-01").periods.len(), 2);
        assert!(parse_periods("2020-01-02..2020-01-01").is_err());
        assert_eq!(split("midcap100_cp-fo"), ("midcap100_cp", false, true));
        assert_eq!((split("nifty50_cp-n500"), split("nifty50_cp-n500-fo"), split("nifty50_cp")), (("nifty50_cp", true, false), ("nifty50_cp", true, true), ("nifty50_cp", false, false)));
    }
}
