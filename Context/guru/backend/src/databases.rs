//! The team's databases (jay-local/agent-work/DB_CONTRACT.md, 20-11-2026; names from `Database Directory.xlsx`).
//!
//! A fixed registry: ids, names and kinds never come from a request. Availability is checked at startup (the
//! directory / file exists); symbol lists and first / last dates are built once, lazily (first
//! `GET /api/guru/yt/databases`, warmed in a background thread), from file listings and each native file's first and
//! last record. Prices of a `db:<id>:<SYMBOL>` series are read ON DEMAND per request from the native AmiBroker file
//! (`yt_amibroker_portfolio::read_native`); no database is loaded at startup except the small `idx_old_2022.csv`.
//! Existing tickers (`NIFTY 50`, `NIFTY FUT`, `basket:nse500:SBIN`, ...) are unchanged and mapped to their database
//! by `database_of_ticker`.
use crate::{baskets::{Catalogue, NativeEvidence}, connors::Ctx, esc, Row};
use std::{collections::{BTreeMap, BTreeSet}, io::{Read, Seek, SeekFrom}, path::{Path, PathBuf}, sync::OnceLock};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Source {
    /// Preloaded idx_data.csv tickers plus the other symbols of a native index directory.
    Preloaded { tickers: &'static [&'static str], native: &'static str },
    /// A small Ticker,Date,Open,High,Low,Close CSV under AMIBROKER_DATA_DIR, preloaded at startup.
    Csv(&'static str),
    /// A native AmiBroker directory under AMIBROKER_DATA_DIR.
    Native(&'static str),
    /// The preloaded VAULT_FUTURES_CSV series.
    Futures { tickers: &'static [&'static str] },
    /// A source-qualified basket of VAULT_BASKET_DATA_DIR, plus (when `native`) the database's other symbols.
    Basket { basket: &'static str, native: Option<&'static str> },
    /// Not on this PC / not readable yet: listed with `available: false` and this note.
    Missing(&'static str),
}

pub(crate) struct Spec {
    pub id: &'static str,
    pub name: &'static str,
    /// `index`, `individual_stock` or `futures` (the existing selected types).
    pub kind: &'static str,
    pub intraday: bool,
    pub prices: &'static str,
    pub has_volume: bool,
    pub has_aux: bool,
    /// The database the existing tickers of this kind come from.
    pub default: bool,
    pub source: Source,
}

const fn spec(id: &'static str, name: &'static str, kind: &'static str, has_volume: bool, has_aux: bool, default: bool, source: Source) -> Spec {
    let prices = if matches!(kind.as_bytes(), b"futures") { "Futures" } else { "Cash" };
    Spec { id, name, kind, intraday: false, prices, has_volume, has_aux, default, source }
}

pub(crate) const NEW_SECTOR_DIR: &str = "New Sector Indices_Updated Till  23-10-2026";
pub(crate) const SPECS: [Spec; 16] = [
    spec("idx_new_sector", "New Sector Indices_Updated Till 23-10-2026", "index", false, false, true,
        Source::Preloaded { tickers: &["NIFTY 50", "NIFTYBANK", "NIFTYMIDCAP100"], native: NEW_SECTOR_DIR }),
    spec("idx_old_2022", "Index data to 31-03-2022 (idx_old_2022)", "index", false, false, false, Source::Csv("idx_old_2022.csv")),
    spec("usd_nifty", "USD_NIFTY", "index", false, false, false, Source::Native("USD_NIFTY")),
    spec("futures_bhavcopy", "NSE F&O bhavcopy near-month futures (Nifty, Bank Nifty)", "futures", true, false, true,
        Source::Futures { tickers: &["NIFTY FUT", "BANKNIFTY FUT"] }),
    spec("fo_updated_2024", "NSE FO_UPDATED TILL_07-10-2024", "futures", true, false, false,
        Source::Basket { basket: "allfo", native: Some("fo_updated_2024") }),
    spec("fo_old_2022", "NSE FO_UPDATED TILL_31-03-2022", "futures", true, false, false, Source::Native("fo_old_2022")),
    spec("cp_updated_2026", "C&P DATABASE UPDATED TILL 16-10-2026", "individual_stock", true, true, true,
        Source::Basket { basket: "nse500", native: Some("cp_updated_2026") }),
    spec("cp_old_2022", "C&P DATABASE UPDATED TILL 31-03-2022", "individual_stock", true, true, false, Source::Native("cp_old_2022")),
    spec("nifty50_cp_2026", "NIFTY 50 C&P (With Aux On - Off)_Updated Till 16-10-2026", "individual_stock", true, true, false, Source::Native("nifty50_cp_2026")),
    spec("midcap100_cp_2026", "MIDCAP100 C&P_Updated Till 16-10-2026", "individual_stock", true, true, false, Source::Native("midcap100_cp_2026")),
    spec("midcap100_aux12_2025", "MIDCAP100_withAux1&2_Updated_till_08-07-2025", "individual_stock", true, true, false, Source::Native("midcap100_aux12_2025")),
    spec("midcapselect_cp_2026", "MIDCAP SELECT C&P UPDATED TILL 16-10-2026", "individual_stock", true, true, false, Source::Native("midcapselect_cp_2026")),
    Spec { intraday: true, ..spec("ieod_fo_2023", "IEOD F&O 1-minute (to 07-02-2023)", "futures", true, false, true, Source::Basket { basket: "ieodfo", native: None }) },
    Spec { intraday: true, ..spec("ieod_fo_old", "IEOD F&O 1-minute, old copy", "futures", true, false, false,
        Source::Missing("Only 2 symbols copied; its 1-minute native format is not read yet.")) },
    spec("idx_volume_2026", "Index Data With Volume_08-05-2026", "index", true, false, false, Source::Missing("Not copied to this PC.")),
    spec("fo_ath_2022", "NSE FO + ATH UPDATED TILL_31-03-2022", "futures", true, false, false, Source::Missing("Not copied to this PC.")),
];

pub(crate) fn spec_of(id: &str) -> Option<&'static Spec> { SPECS.iter().find(|s| s.id == id) }
pub(crate) fn name(id: &str) -> Option<&'static str> { spec_of(id).map(|s| s.name) }

/// Registry database of a ticker as the engine resolves it (existing forms and `db:<id>:<SYMBOL>`).
pub(crate) fn database_of_ticker(ticker: &str) -> Option<&'static str> {
    if let Some((id, _)) = ticker.strip_prefix("db:").and_then(|rest| rest.split_once(':')) { return spec_of(id).map(|s| s.id); }
    if let Some((basket, _)) = ticker.strip_prefix("basket:").and_then(|rest| rest.split_once(':')) {
        return SPECS.iter().find(|s| matches!(s.source, Source::Basket { basket: b, .. } if b == basket)).map(|s| s.id);
    }
    SPECS.iter().find(|s| match s.source {
        Source::Preloaded { tickers, .. } | Source::Futures { tickers } => tickers.contains(&ticker),
        _ => false,
    }).map(|s| s.id)
}

/// Native file of `symbol` in `dir` (`<dir>/<first letter|digit, else _>/<SYMBOL>`), confined to `dir`.
fn native_path(dir: &Path, symbol: &str) -> Result<PathBuf, String> {
    if symbol.is_empty() || symbol.len() > 128 || symbol.contains('/') || symbol.contains('\\') || symbol == "." || symbol == ".."
        || symbol.starts_with("~~~") || symbol.chars().any(char::is_control) {
        return Err("Unsafe database symbol".into());
    }
    let first = symbol.chars().next().unwrap().to_ascii_lowercase();
    let sub = if first.is_ascii_alphanumeric() { first } else { '_' };
    let root = dir.canonicalize().map_err(|_| "Database directory unavailable")?;
    let path = root.join(sub.to_string()).join(symbol).canonicalize().map_err(|_| format!("Symbol {symbol} is not in this database"))?;
    if !path.starts_with(&root) || !path.is_file() { return Err(format!("Symbol {symbol} is not in this database")); }
    Ok(path)
}

/// First and last valid record dates of a native file without reading all of it.
fn native_span(path: &Path) -> Option<(String, String)> {
    const HEADER: usize = 1184;
    const REC: u64 = 40;
    let mut f = std::fs::File::open(path).ok()?;
    let mut head = vec![0u8; HEADER + REC as usize];
    f.read_exact(&mut head).ok()?;
    if &head[..8] != b"BROKDAt5" { return None; }
    let count = u32::from_le_bytes(head[HEADER - 4..HEADER].try_into().unwrap()) as u64;
    if count == 0 { return None; }
    let date = |b: &[u8]| {
        let d = u64::from_le_bytes(b[..8].try_into().unwrap());
        let s = format!("{:04}-{:02}-{:02}", d >> 52, (d >> 48) & 15, (d >> 43) & 31);
        crate::baskets::valid_date(&s).then_some(s)
    };
    let first = date(&head[HEADER..]);
    let mut last = [0u8; 8];
    f.seek(SeekFrom::Start(HEADER as u64 + (count - 1) * REC)).ok()?;
    f.read_exact(&mut last).ok()?;
    match (first, date(&last)) {
        (Some(a), Some(b)) if a <= b => Some((a, b)),
        _ => {
            // unsorted or stale records: read the file (bounded) and take the valid extremes
            let bytes = std::fs::read(path).ok().filter(|b| b.len() <= 32 << 20)?;
            let raw = crate::yt_amibroker_portfolio::read_native(&bytes).ok()?;
            let dates: BTreeSet<String> = raw.iter().map(|b| format!("{:04}-{:02}-{:02}", b.date.0, b.date.1, b.date.2)).filter(|d| crate::baskets::valid_date(d)).collect();
            Some((dates.first()?.clone(), dates.last()?.clone()))
        }
    }
}

/// One series of a registry database, read for one request.
pub(crate) struct Series<'a> { pub rows: Vec<Row>, pub evidence: &'a NativeEvidence, pub aux2: Option<Vec<f64>> }

#[derive(Default)]
pub(crate) struct Registry {
    root: Option<PathBuf>,
    /// Small CSV databases (idx_old_2022.csv), preloaded.
    csv: BTreeMap<&'static str, BTreeMap<String, Vec<Row>>>,
    evidence: BTreeMap<&'static str, NativeEvidence>,
    json: OnceLock<String>,
}

impl Registry {
    /// `root`: AMIBROKER_DATA_DIR (read-only copies of the team's AmiBroker databases).
    pub(crate) fn new(root: Option<PathBuf>) -> Self {
        let mut csv = BTreeMap::new();
        if let Some(root) = &root {
            for s in &SPECS {
                if let Source::Csv(file) = s.source {
                    match crate::load_csv(&root.join(file).to_string_lossy()) {
                        Ok(rows) => { csv.insert(s.id, rows); }
                        Err(e) => eprintln!("warning: database {}: {e}", s.id),
                    }
                }
            }
        }
        let evidence = SPECS.iter().map(|s| (s.id, NativeEvidence::database(s.kind, s.name))).collect();
        Self { root, csv, evidence, json: OnceLock::new() }
    }
    fn native_dir(&self, s: &Spec) -> Option<PathBuf> {
        let dir = match s.source { Source::Preloaded { native, .. } | Source::Native(native) | Source::Basket { native: Some(native), .. } => native, _ => return None };
        self.root.as_ref().map(|r| r.join(dir)).filter(|p| p.is_dir())
    }
    /// (available, note) of a database.
    fn availability(&self, s: &Spec, ctxs: &BTreeMap<String, Ctx>, baskets: &Catalogue) -> (bool, String) {
        match s.source {
            Source::Missing(note) => (false, note.into()),
            Source::Csv(_) => if self.csv.contains_key(s.id) { (true, String::new()) } else { (false, "Its CSV is not under AMIBROKER_DATA_DIR.".into()) },
            Source::Native(_) => if self.native_dir(s).is_some() { (true, String::new()) } else { (false, "Its AmiBroker copy is not under AMIBROKER_DATA_DIR.".into()) },
            Source::Preloaded { tickers, .. } | Source::Futures { tickers } => {
                if tickers.iter().any(|t| ctxs.contains_key(*t)) { (true, String::new()) } else { (false, "Its series are not loaded in the engine.".into()) }
            }
            Source::Basket { basket, .. } => if !baskets.members(basket).is_empty() { (true, String::new()) } else { (false, format!("Basket {basket} is not loaded in the engine.")) },
        }
    }

    /// Prices (and, for the C&P databases, dated AUX2) of a `db:<id>:<SYMBOL>` ticker. `None`: not a `db:` ticker.
    pub(crate) fn series(&self, ticker: &str, with_aux2: bool) -> Option<Result<Series<'_>, String>> {
        let rest = ticker.strip_prefix("db:")?;
        Some((|| {
            let (id, symbol) = rest.split_once(':').ok_or("A database ticker is db:<database>:<SYMBOL>")?;
            let s = spec_of(id).ok_or_else(|| format!("unknown database {id}"))?;
            let evidence = &self.evidence[s.id];
            let rows_aux = match s.source {
                Source::Missing(note) => return Err(format!("needs_data: {} is not available: {note}", s.name)),
                Source::Futures { .. } => return Err(format!("{} series use their existing tickers (NIFTY FUT, BANKNIFTY FUT).", s.name)),
                Source::Basket { native: None, basket } => return Err(format!("{} series use their existing tickers (basket:{basket}:<SYMBOL>).", s.name)),
                Source::Csv(_) => {
                    let rows = self.csv.get(s.id).and_then(|m| m.get(symbol)).ok_or_else(|| format!("Symbol {symbol} is not in {}", s.name))?;
                    if rows.len() < 2 { return Err(format!("needs_data: {symbol} has fewer than two bars in {}", s.name)); }
                    (rows.clone(), None)
                }
                _ => {
                    let dir = self.native_dir(s).ok_or_else(|| format!("needs_data: {} is not available on this server", s.name))?;
                    let path = native_path(&dir, symbol)?;
                    if std::fs::metadata(&path).map_err(|_| "Symbol file unavailable")?.len() > 32 << 20 { return Err("Symbol file exceeds the size limit".into()); }
                    let bytes = std::fs::read(&path).map_err(|_| "Symbol file unavailable")?;
                    let raw = crate::yt_amibroker_portfolio::read_native(&bytes)?;
                    let (mut rows, aux) = crate::yt_amibroker_portfolio::clean(symbol, &raw);
                    // no volume recorded (indices) is absent volume, as the basket CSVs read it
                    for r in &mut rows { r.volume = r.volume.filter(|v| *v > 0.); }
                    if rows.len() < 2 { return Err(format!("needs_data: {symbol} has fewer than two usable bars in {}", s.name)); }
                    // AUX2 is dated F&O membership only in the two C&P cash databases (yt_aux::AUX2_FO_DATABASES).
                    let aux2 = (with_aux2 && crate::yt_aux::AUX2_FO_DATABASES.contains(&s.id)).then(|| aux.iter().map(|a| a.aux2 as f64).collect());
                    (rows, aux2)
                }
            };
            Ok(Series { rows: rows_aux.0, evidence, aux2: rows_aux.1 })
        })())
    }

    pub(crate) fn provenance_json(&self, ticker: &str) -> Option<String> {
        let (id, symbol) = ticker.strip_prefix("db:")?.split_once(':')?;
        let s = spec_of(id)?;
        Some(format!("{{\"qualified_ticker\":\"{}\",\"symbol\":\"{}\",\"database\":\"{}\",\"database_name\":\"{}\",\"source\":\"Team AmiBroker database {}\",\"quote_accuracy_independently_verified\":false,\"basis\":\"Actual native daily OHLC and volume of the team's database, read from its AmiBroker file for this request (bars with a non-positive price or a duplicate date dropped, high/low widened to cover open/close). Current database membership is a snapshot, not historical membership.\",{}}}",
            esc(ticker), esc(symbol), s.id, esc(s.name), esc(s.name), self.evidence[s.id].json()))
    }

    /// Body of `GET /api/guru/yt/databases`, built once.
    pub(crate) fn json(&self, ctxs: &BTreeMap<String, Ctx>, baskets: &Catalogue) -> &str {
        self.json.get_or_init(|| self.build_json(ctxs, baskets))
    }
    fn build_json(&self, ctxs: &BTreeMap<String, Ctx>, baskets: &Catalogue) -> String {
        let mut items = Vec::new();
        for s in &SPECS {
            let (available, note) = self.availability(s, ctxs, baskets);
            // (ticker, symbol) and the database's first / last date
            let mut tickers: Vec<(String, String)> = Vec::new();
            let mut span: Option<(String, String)> = None;
            let mut widen = |a: &str, b: &str| { span = Some(match span.take() { None => (a.into(), b.into()), Some((x, y)) => (x.min(a.into()), y.max(b.into())) }); };
            let mut has_volume = s.has_volume;
            if available {
                let mut taken = BTreeSet::new();
                match s.source {
                    Source::Preloaded { tickers: t, .. } | Source::Futures { tickers: t } => for ticker in t {
                        if let Some(c) = ctxs.get(*ticker).filter(|c| !c.r.is_empty()) {
                            tickers.push((ticker.to_string(), ticker.to_string()));
                            taken.insert(ticker.to_string());
                            widen(&c.r[0].date, &c.r[c.r.len() - 1].date);
                            if matches!(s.source, Source::Futures { .. }) { has_volume = c.has_volume; }
                        }
                    },
                    Source::Basket { basket, .. } => for m in baskets.members(basket) {
                        widen(&m.first, &m.last);
                        taken.insert(m.symbol.clone());
                        tickers.push((m.key, m.symbol));
                    },
                    Source::Csv(_) => if let Some(m) = self.csv.get(s.id) {
                        for (symbol, rows) in m.iter().filter(|(_, r)| r.len() >= 2) {
                            widen(&rows[0].date, &rows[rows.len() - 1].date);
                            tickers.push((format!("db:{}:{symbol}", s.id), symbol.clone()));
                        }
                    },
                    _ => {}
                }
                if let Some(dir) = self.native_dir(s) {
                    for (symbol, path) in crate::yt_amibroker_portfolio::universe(&dir).unwrap_or_default() {
                        if taken.contains(&symbol) { continue; }
                        let Some((a, b)) = native_span(&path) else { continue };
                        widen(&a, &b);
                        tickers.push((format!("db:{}:{symbol}", s.id), symbol));
                    }
                }
            }
            let date = |d: Option<&String>| d.map_or("null".into(), |d| format!("\"{d}\""));
            let list: Vec<String> = tickers.iter().map(|(t, sym)| format!("{{\"ticker\":\"{}\",\"symbol\":\"{}\"}}", esc(t), esc(sym))).collect();
            items.push(format!("{{\"id\":\"{}\",\"name\":\"{}\",\"kind\":\"{}\",\"intraday\":{},\"prices\":\"{}\",\"first\":{},\"last\":{},\"has_volume\":{},\"has_aux\":{},\"available\":{},\"note\":\"{}\",\"default\":{},\"tickers\":[{}]}}",
                s.id, esc(s.name), s.kind, s.intraday, s.prices, date(span.as_ref().map(|x| &x.0)), date(span.as_ref().map(|x| &x.1)), has_volume, s.has_aux, available, esc(&note), s.default, list.join(",")));
        }
        format!("{{\"databases\":[{}]}}", items.join(","))
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    static NEXT: AtomicUsize = AtomicUsize::new(0);

    /// A native AmiBroker format-5 file: (date, open, high, low, close, volume, aux2).
    pub(crate) fn native_file(bars: &[((u64, u64, u64), f32, f32, f32, f32, f32, f32)]) -> Vec<u8> {
        let mut b = vec![0u8; 1184 + 40 * bars.len()];
        b[..8].copy_from_slice(b"BROKDAt5");
        b[1180..1184].copy_from_slice(&(bars.len() as u32).to_le_bytes());
        for (i, &((y, m, d), o, h, l, c, v, a2)) in bars.iter().enumerate() {
            let k = 1184 + 40 * i;
            b[k..k + 8].copy_from_slice(&((y << 52) | (m << 48) | (d << 43)).to_le_bytes());
            for (off, val) in [(8, c), (12, o), (16, h), (20, l), (24, v), (28, 0.), (32, 1.), (36, a2)] { b[k + off..k + off + 4].copy_from_slice(&val.to_le_bytes()); }
        }
        b
    }
    /// Fixture AMIBROKER_DATA_DIR in a temp dir: cp_old_2022 (stock with volume and AUX2), nifty50_cp_2026,
    /// the New Sector index directory (no volume) and idx_old_2022.csv.
    pub(crate) fn fixture() -> PathBuf {
        let root = std::env::temp_dir().join(format!("sv-databases-{}-{}", std::process::id(), NEXT.fetch_add(1, Ordering::SeqCst)));
        let _ = std::fs::remove_dir_all(&root);
        let bars = |vol: f32| -> Vec<_> { (1..=60u64).map(|d| ((2021, 1 + (d - 1) / 28, 1 + (d - 1) % 28), 100. + d as f32, 102. + d as f32, 99. + d as f32, 101. + d as f32, vol, (d % 2) as f32)).collect() };
        for (dir, sym, vol) in [("cp_old_2022", "RELIANCE", 1000.), ("nifty50_cp_2026", "RELIANCE", 1000.), (NEW_SECTOR_DIR, "NIFTYAUTO", 0.), (NEW_SECTOR_DIR, "NIFTY 50", 0.), ("USD_NIFTY", "USDINR", 0.)] {
            let sub = root.join(dir).join(sym.chars().next().unwrap().to_ascii_lowercase().to_string());
            std::fs::create_dir_all(&sub).unwrap();
            std::fs::write(sub.join(sym), native_file(&bars(vol))).unwrap();
        }
        std::fs::create_dir_all(root.join("cp_old_2022/WatchLists")).unwrap();
        std::fs::write(root.join("cp_old_2022/WatchLists/List 0.tls"), "RELIANCE").unwrap();
        let mut csv = String::from("Ticker,Date,Open,High,Low,Close\n");
        for d in 1..=28 { csv.push_str(&format!("NIFTYIT,2021-02-{d:02},10,11,9,10.5\n")); }
        std::fs::write(root.join("idx_old_2022.csv"), csv).unwrap();
        root
    }

    #[test]
    fn registry_has_the_contract_ids_names_and_kinds() {
        let ids: Vec<_> = SPECS.iter().map(|s| s.id).collect();
        assert_eq!(ids, ["idx_new_sector", "idx_old_2022", "usd_nifty", "futures_bhavcopy", "fo_updated_2024", "fo_old_2022", "cp_updated_2026", "cp_old_2022", "nifty50_cp_2026", "midcap100_cp_2026", "midcap100_aux12_2025", "midcapselect_cp_2026", "ieod_fo_2023", "ieod_fo_old", "idx_volume_2026", "fo_ath_2022"]);
        assert_eq!(SPECS.iter().filter(|s| s.default).map(|s| s.id).collect::<Vec<_>>(), ["idx_new_sector", "futures_bhavcopy", "cp_updated_2026", "ieod_fo_2023"]);
        assert_eq!(spec_of("fo_updated_2024").unwrap().kind, "futures", "allfo is futures prices");
        assert!(spec_of("ieod_fo_2023").unwrap().intraday && spec_of("ieod_fo_2023").unwrap().kind == "futures");
        assert_eq!(name("cp_old_2022"), Some("C&P DATABASE UPDATED TILL 31-03-2022"));
        for (ticker, db) in [("NIFTY 50", Some("idx_new_sector")), ("NIFTY FUT", Some("futures_bhavcopy")), ("basket:nse500:SBIN", Some("cp_updated_2026")),
            ("basket:allfo:SBIN", Some("fo_updated_2024")), ("basket:ieodfo:NIFTY", Some("ieod_fo_2023")), ("db:cp_old_2022:SBIN", Some("cp_old_2022")),
            ("db:nope:SBIN", None), ("SPY", None)] {
            assert_eq!(database_of_ticker(ticker), db, "{ticker}");
        }
    }

    #[test]
    fn registry_json_lists_available_databases_tickers_and_dates_from_fixture_files() {
        let root = fixture();
        let reg = Registry::new(Some(root.clone()));
        let json = reg.json(&BTreeMap::new(), &Catalogue::default()).to_string();
        let item = |id: &str| { let k = json.find(&format!("{{\"id\":\"{id}\"")).unwrap(); let e = json[k..].find("]}").unwrap(); json[k..k + e].to_string() };
        let cp = item("cp_old_2022");
        assert!(cp.contains("\"available\":true") && cp.contains("\"kind\":\"individual_stock\"") && cp.contains("\"first\":\"2021-01-01\"") && cp.contains("\"last\":\"2021-03-04\""), "{cp}");
        assert!(cp.contains("{\"ticker\":\"db:cp_old_2022:RELIANCE\",\"symbol\":\"RELIANCE\"}") && !cp.contains("List 0"), "{cp}");
        assert!(item("idx_old_2022").contains("{\"ticker\":\"db:idx_old_2022:NIFTYIT\",\"symbol\":\"NIFTYIT\"}"));
        // nothing preloaded in this state: the New Sector database lists its native symbols as db: tickers
        assert!(item("idx_new_sector").contains("\"available\":false"));
        let missing = item("idx_volume_2026");
        assert!(missing.contains("\"available\":false") && missing.contains("Not copied to this PC") && missing.contains("\"first\":null"), "{missing}");
        assert!(item("fo_old_2022").contains("\"available\":false"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn db_tickers_read_native_files_on_demand_with_type_volume_and_aux2() {
        let root = fixture();
        let reg = Registry::new(Some(root.clone()));
        assert!(reg.series("NIFTY 50", false).is_none(), "existing tickers are not db: tickers");
        let s = reg.series("db:cp_old_2022:RELIANCE", true).unwrap().unwrap();
        assert_eq!((s.rows.len(), s.rows[0].ticker.as_str(), s.rows[0].date.as_str(), s.evidence.asset_type()), (60, "RELIANCE", "2021-01-01", "individual_stock"));
        assert_eq!(s.rows[0].volume, Some(1000.));
        assert_eq!(s.aux2.as_ref().unwrap()[..3], [1., 0., 1.]);
        // AUX2 is F&O membership only in the C&P databases: never loaded from another database
        assert!(reg.series("db:nifty50_cp_2026:RELIANCE", true).unwrap().unwrap().aux2.is_none());
        let idx = reg.series(&format!("db:idx_new_sector:NIFTYAUTO"), false).unwrap().unwrap();
        assert_eq!((idx.evidence.asset_type(), idx.rows[0].volume), ("index", None));
        let csv = reg.series("db:idx_old_2022:NIFTYIT", false).unwrap().unwrap();
        assert_eq!((csv.rows.len(), csv.evidence.asset_type()), (28, "index"));
        for bad in ["db:cp_old_2022:../cp_old_2022/r/RELIANCE", "db:cp_old_2022:SBIN", "db:nope:SBIN", "db:cp_old_2022", "db:idx_volume_2026:NIFTY", "db:futures_bhavcopy:NIFTY FUT", "db:cp_old_2022:~~~EQUITY"] {
            assert!(reg.series(bad, false).unwrap().is_err(), "{bad}");
        }
        assert!(reg.provenance_json("db:cp_old_2022:RELIANCE").unwrap().contains("\"asset_type\":\"individual_stock\""));
        let _ = std::fs::remove_dir_all(root);
    }
}
