//! AmiBroker databases and their watchlists, as AmiBroker opens them (library/cp-aux/scripts/convert_ami_db.py, mounted
//! read-only at VAULT_AMI_DB_DIR). A watchlist runs as the portfolio universe `ami-<db>-<watchlist>`; every stock keeps its
//! own database's Aux1 / Aux2, so execution.aux1 / aux2 gate its buys per stock as the AFLs' `Buy = Buy AND Aux1 == 1`
//! (series.rs entry_gates). Book strategies and the other universes (baskets.rs, cp.rs) do not read this.
use super::{bars::Bars, rules::LabError};
use crate::Row;
use std::{collections::{BTreeMap, HashMap}, path::{Path, PathBuf}, sync::OnceLock};

pub struct Stock { pub ticker: String, pub path: PathBuf, pub first: String, pub last: String }
pub struct Watchlist { pub db_label: String, pub name: String, pub stocks: Vec<usize> }
pub struct Catalogue { pub stocks: Vec<Stock>, pub lists: BTreeMap<String, Watchlist> }

static AMI: OnceLock<Option<Catalogue>> = OnceLock::new();
/// Read once from VAULT_AMI_DB_DIR; None when the engine has no such data (US, gl, tests).
pub fn catalogue() -> Option<&'static Catalogue> {
    AMI.get_or_init(|| std::env::var("VAULT_AMI_DB_DIR").ok().and_then(|d| match Catalogue::load(Path::new(&d)) {
        Ok(c) => Some(c),
        Err(e) => { eprintln!("AmiBroker database data unavailable: {e}"); None }
    })).as_ref()
}

/// (id, watchlist, database, members) of every watchlist, by id.
pub fn universes() -> Vec<(String, String, String, usize)> {
    catalogue().map_or(vec![], |c| c.lists.iter().map(|(id, w)| (id.clone(), w.name.clone(), w.db_label.clone(), w.stocks.len())).collect())
}

/// The converter writes no quoted fields (it refuses names with a comma or quote).
fn table(root: &Path, name: &str) -> Result<Vec<HashMap<String, String>>, String> {
    let text = std::fs::read_to_string(root.join(name)).map_err(|_| format!("{name} unavailable"))?;
    let mut lines = text.lines();
    let header: Vec<&str> = lines.next().ok_or(format!("{name} is empty"))?.split(',').collect();
    lines.filter(|l| !l.trim().is_empty()).map(|l| {
        let cols: Vec<&str> = l.split(',').collect();
        if cols.len() != header.len() { return Err(format!("{name}: a row has {} fields, the header {}", cols.len(), header.len())); }
        Ok(header.iter().zip(cols).map(|(h, c)| (h.to_string(), c.to_string())).collect())
    }).collect()
}

impl Catalogue {
    pub fn load(dir: &Path) -> Result<Self, String> {
        let root = dir.canonicalize().map_err(|_| "AmiBroker database directory unavailable")?;
        let (mut stocks, mut at) = (vec![], HashMap::new());
        for r in table(&root, "symbols.csv")? {
            let ticker = &r["ticker"];
            if ticker.is_empty() || !ticker.bytes().all(|c| c.is_ascii_alphanumeric() || b"&.-_".contains(&c)) { return Err(format!("ticker {ticker} is not a plain symbol")); }
            let path = root.join(&r["path"]).canonicalize().map_err(|_| format!("prices for {ticker} unavailable"))?;
            if !path.starts_with(&root) { return Err(format!("prices for {ticker} escape the data directory")); }
            at.insert((r["db"].clone(), ticker.clone()), stocks.len());
            stocks.push(Stock { ticker: ticker.clone(), path, first: r["first"].clone(), last: r["last"].clone() });
        }
        let mut lists: BTreeMap<String, Watchlist> = BTreeMap::new();
        for r in table(&root, "watchlists.csv")? {
            let id = &r["id"];
            if !(id.len() <= 40 && id.starts_with("ami-") && id.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')) { return Err(format!("watchlist id {id} is not a basket id")); }
            let k = *at.get(&(r["db"].clone(), r["ticker"].clone())).ok_or_else(|| format!("{id}: {} has no prices", r["ticker"]))?;
            lists.entry(id.clone()).or_insert_with(|| Watchlist { db_label: r["db_label"].clone(), name: r["watchlist"].clone(), stocks: vec![] }).stocks.push(k);
        }
        if lists.is_empty() { return Err("watchlists.csv lists no watchlist".into()); }
        Ok(Self { stocks, lists })
    }
}

impl Stock {
    /// Daily bars with this database's Aux1 / Aux2 per bar.
    pub fn read(&self) -> Result<Bars, LabError> {
        let need = |why: &str| LabError::NeedsData(format!("{}: {why}", self.ticker));
        let text = std::fs::read_to_string(&self.path).map_err(|_| need("prices unavailable"))?;
        let rows: Vec<Row> = crate::baskets::parse_rows(&text, &self.ticker).map_err(|e| need(&e.to_string()))?;
        let mut lines = text.lines();
        let header: Vec<&str> = lines.next().unwrap_or("").split(',').collect();
        let col = |name: &str| header.iter().position(|c| *c == name).ok_or_else(|| need(&format!("prices lack {name}")));
        let (a1, a2) = (col("aux1")?, col("aux2")?);
        let body: Vec<Vec<&str>> = lines.filter(|l| !l.trim().is_empty()).map(|l| l.split(',').collect()).collect();
        if body.len() != rows.len() { return Err(need("Aux columns do not line up with the prices")); }
        let aux = |i: usize| body.iter().map(|c| c.get(i).and_then(|v| v.parse::<f64>().ok()).unwrap_or(f64::NAN)).collect::<Vec<f64>>();
        let mut bars = Bars::from_rows(&rows);
        bars.aux = [Some(aux(a1)), Some(aux(a2))];
        Ok(bars)
    }
}

#[cfg(test)]
mod tests {
    use super::Catalogue;
    #[test] fn a_watchlist_loads_its_stocks_with_their_own_aux_columns() {
        let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("target/lab-ami-fixtures/{}", std::process::id()));
        std::fs::create_dir_all(dir.join("prices")).unwrap();
        std::fs::write(dir.join("prices/a.csv"), "date,open,high,low,close,volume,open_interest,aux1,aux2\n2024-01-01,10.0,11.0,9.0,10.5,100.0,0.0,1.0,0.0\n2024-01-02,10.5,12.0,10.0,11.0,100.0,0.0,0.0,1.0\n").unwrap();
        std::fs::write(dir.join("symbols.csv"), "db,db_label,symbol,ticker,path,first,last,bars,supplemented,source_sha256\ncp,C&P DATABASE,ABB,ABB,prices/a.csv,2024-01-01,2024-01-02,2,0,x\n").unwrap();
        std::fs::write(dir.join("watchlists.csv"), "id,db,db_label,watchlist,ticker\nami-cp-n50,cp,C&P DATABASE,Nifty 50 C&P 141,ABB\n").unwrap();
        let c = Catalogue::load(&dir).unwrap();
        let w = &c.lists["ami-cp-n50"];
        assert_eq!((w.name.as_str(), w.db_label.as_str(), w.stocks.len()), ("Nifty 50 C&P 141", "C&P DATABASE", 1));
        let b = c.stocks[w.stocks[0]].read().unwrap();
        assert_eq!((b.aux[0].clone().unwrap(), b.aux[1].clone().unwrap()), (vec![1., 0.], vec![0., 1.]));
        std::fs::write(dir.join("watchlists.csv"), "id,db,db_label,watchlist,ticker\nami-cp-n50,cp,C&P DATABASE,X,NOPE\n").unwrap();
        assert!(Catalogue::load(&dir).err().unwrap().contains("NOPE has no prices"));
    }
}
