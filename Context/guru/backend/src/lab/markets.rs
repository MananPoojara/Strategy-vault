//! Provisional global and crypto instruments served by `engine-gl` (TradingView exports, internal research only).
//! Their metadata lives here because the admission files (instrument_policy.rs, gate.rs) are book code and stay untouched.
use super::json::esc;
use super::rules::LABEL;
use crate::Row;
use std::collections::BTreeMap;

pub const PROVISIONAL: &str = "Provisional · TradingView export · internal research only, not publishable to YT Strategies";
/// The book statistics annualise volatility, Sharpe and Sortino with sqrt(252) (main.rs stats()).
pub const BOOK_BARS_PER_YEAR: f64 = 252.;

pub struct GlInstrument {
    /// TradingView "EXCHANGE:SYMBOL"; the colon keeps these ids apart from every India/US ticker.
    pub ticker: &'static str,
    pub name: &'static str,
    /// index | cfd | crypto
    pub kind: &'static str,
    /// gl | crypto (the saved-strategy market; both are served by /api/gl)
    pub market: &'static str,
    pub currency: &'static str,
    pub timezone: &'static str,
    pub calendar: &'static str,
    pub bars_per_year: f64,
    /// Trades around the clock: an open equal to the previous close is normal, not stale.
    pub continuous: bool,
    pub total_return: bool,
    pub source: &'static str,
    pub quirks: &'static str,
}

const NO_VOLUME: &str = "No volume is staged (TradingView exports carry none, or only CFD tick counts).";
pub const GL: &[GlInstrument] = &[
    GlInstrument { ticker: "BSE_DLY:SENSEX", name: "S&P BSE SENSEX", kind: "index", market: "gl", currency: "INR", timezone: "Asia/Kolkata",
        calendar: "BSE sessions Mon-Fri, plus occasional special weekend sessions", bars_per_year: 252., continuous: false, total_return: false,
        source: "TradingView export BSE_DLY:SENSEX, daily", quirks: "Close-only bars before 1991; many opens equal the previous close in 2009. No volume is staged." },
    GlInstrument { ticker: "CRYPTO:BTCUSD", name: "Bitcoin / US dollar (TradingView CRYPTO index)", kind: "crypto", market: "crypto", currency: "USD", timezone: "UTC",
        calendar: "7 days a week, UTC days", bars_per_year: 365., continuous: true, total_return: false,
        source: "TradingView export CRYPTO:BTCUSD, daily", quirks: "Close-only bars in 2009-2010. No volume is staged." },
    GlInstrument { ticker: "CRYPTO:ETHUSD", name: "Ether / US dollar (TradingView CRYPTO index)", kind: "crypto", market: "crypto", currency: "USD", timezone: "UTC",
        calendar: "7 days a week, UTC days", bars_per_year: 365., continuous: true, total_return: false,
        source: "TradingView export CRYPTO:ETHUSD, daily", quirks: "Erratic first weeks after the August 2015 launch (a 0.15 low). No volume is staged." },
    GlInstrument { ticker: "HSI:HSI", name: "Hang Seng Index", kind: "index", market: "gl", currency: "HKD", timezone: "Asia/Hong_Kong",
        calendar: "HKEX sessions Mon-Fri", bars_per_year: 252., continuous: false, total_return: false,
        source: "TradingView export HSI:HSI, daily", quirks: "Close-only bars 1987-1989; stale opens 1990-1991. No volume is staged." },
    GlInstrument { ticker: "IG:DAX", name: "DAX 40 (IG CFD)", kind: "cfd", market: "gl", currency: "EUR", timezone: "Europe/Berlin",
        calendar: "IG CFD: Mon-Fri plus a short Sunday-evening bar since August 2010", bars_per_year: 311., continuous: false, total_return: true,
        source: "TradingView export IG:DAX, daily", quirks: "About 51 Sunday bars a year count as bars; weeks here run Monday-Sunday, so a weekly close is its Sunday bar. DAX is a total-return index. CFD tick volume is not staged." },
    GlInstrument { ticker: "IG:FTSE", name: "FTSE 100 (IG CFD)", kind: "cfd", market: "gl", currency: "GBP", timezone: "Europe/London",
        calendar: "IG CFD: Mon-Fri plus a short Sunday-evening bar since August 2010", bars_per_year: 311., continuous: false, total_return: false,
        source: "TradingView export IG:FTSE, daily", quirks: "Opens equal the previous close 1999-2010; about 51 Sunday bars a year; weeks here run Monday-Sunday. CFD tick volume is not staged." },
    GlInstrument { ticker: "NASDAQ_DLY:IXIC", name: "NASDAQ Composite", kind: "index", market: "gl", currency: "USD", timezone: "America/New_York",
        calendar: "Nasdaq sessions Mon-Fri", bars_per_year: 252., continuous: false, total_return: false,
        source: "TradingView export NASDAQ_DLY:IXIC, daily", quirks: "Close-only bars 1971-1984; stale opens 1996-1999. No volume is staged." },
    GlInstrument { ticker: "SPREADEX:NIKKEI", name: "Nikkei 225 (Spreadex spread bet)", kind: "cfd", market: "gl", currency: "JPY", timezone: "Asia/Tokyo",
        calendar: "Spreadex quotes Mon-Fri", bars_per_year: 252., continuous: false, total_return: false,
        source: "TradingView export SPREADEX:NIKKEI, daily", quirks: "Starts 2016-05-23; some years have missing sessions. Spread-bet tick volume is not staged." },
    GlInstrument { ticker: "TVC:CAC40", name: "CAC 40", kind: "index", market: "gl", currency: "EUR", timezone: "Europe/Paris",
        calendar: "Euronext Paris sessions Mon-Fri", bars_per_year: 252., continuous: false, total_return: false,
        source: "TradingView export TVC:CAC40, daily", quirks: NO_VOLUME },
    GlInstrument { ticker: "TVC:RUT", name: "Russell 2000", kind: "index", market: "gl", currency: "USD", timezone: "America/New_York",
        calendar: "NYSE sessions Mon-Fri", bars_per_year: 252., continuous: false, total_return: false,
        source: "TradingView export TVC:RUT, daily", quirks: "Close-only bars before November 1992; stale opens 1993-2006. No volume is staged." },
];

pub fn gl(ticker: &str) -> Option<&'static GlInstrument> { GL.iter().find(|m| m.ticker == ticker) }

impl GlInstrument {
    pub fn label(&self) -> String { format!("Provisional · {}", self.source) }
    /// Same leading keys as the India/US instrument JSON (lab/api.rs instrument()), then the provisional metadata.
    pub fn json(&self) -> String {
        format!(
            "{{\"ticker\":\"{}\",\"type\":\"{}\",\"native\":false,\"market\":\"{}\",\"currency\":\"{}\",\"name\":\"{}\",\"timezone\":\"{}\",\"calendar\":\"{}\",\"bars_per_year\":{},\"price\":\"{}\",\"has_volume\":false,\"provisional\":true,\"label\":\"{}\",\"source\":\"{}\",\"quirks\":\"{}\"}}",
            esc(self.ticker), self.kind, self.market, self.currency, esc(self.name), self.timezone, esc(self.calendar), self.bars_per_year,
            if self.total_return { "total_return" } else { "price" }, esc(&self.label()), esc(self.source), esc(self.quirks))
    }
}

/// First date from which next-open fills use real opens: 1 January after the last calendar year in which more than 20% of
/// bars are close-only (O=H=L=C) or, on session markets, open exactly at the previous close. None: every year is usable.
pub fn tradable_from(rows: &[Row], continuous: bool) -> Option<String> {
    let mut years: BTreeMap<&str, (usize, usize)> = BTreeMap::new();
    for (i, r) in rows.iter().enumerate() {
        let close_only = r.open == r.high && r.high == r.low && r.low == r.close;
        let stale = !continuous && i > 0 && r.open == rows[i - 1].close;
        let y = years.entry(&r.date[..4]).or_default();
        y.0 += 1;
        if close_only || stale { y.1 += 1; }
    }
    years.iter().rev().find(|(_, (n, bad))| bad * 5 > *n).map(|(y, _)| format!("{}-01-01", y.parse::<i32>().unwrap_or(0) + 1))
}

pub fn bars_per_year(ticker: &str) -> f64 { gl(ticker).map_or(BOOK_BARS_PER_YEAR, |m| m.bars_per_year) }
/// Rescales the sqrt(252) ratios of the book statistics to the instrument's own bar frequency (crypto: 365 bars a year).
/// CAGR, years and trade rates already use calendar days, so nothing else changes.
pub fn annualise(s: &mut BTreeMap<&'static str, f64>, bars_per_year: f64) {
    if bars_per_year == BOOK_BARS_PER_YEAR { return; }
    let f = (bars_per_year / BOOK_BARS_PER_YEAR).sqrt();
    for k in ["vol", "sharpe", "sortino", "gross_sharpe"] {
        if let Some(v) = s.get_mut(k) { *v *= f; }
    }
}

/// `VAULT_LAB_MARKET=gl` marks the engine-gl container (compose.yaml); the India and US engines leave it unset.
pub fn engine_market() -> Option<String> { std::env::var("VAULT_LAB_MARKET").ok() }
/// engine-gl answers indicator-lab routes only: book strategies have no admission rules for provisional data.
pub fn engine_route_allowed(market: Option<&str>, path: &str) -> bool { market != Some("gl") || path.starts_with("/api/lab/") }
pub fn refused_json(path: &str) -> String {
    mark_provisional(&format!("{{\"status\":\"not_found\",\"error\":\"engine-gl serves indicator-lab routes only; {} is not available on provisional TradingView data\",\"label\":\"{LABEL}\"}}", esc(path)))
}
/// Every engine-gl lab body (results and errors) starts with the provisional flag. Lab bodies are JSON objects.
pub fn mark_provisional(body: &str) -> String {
    format!("{{\"provisional\":true,\"provisional_note\":\"{PROVISIONAL}\",{}", body.strip_prefix('{').unwrap_or(body))
}
pub fn not_staged(ticker: &str) -> String {
    format!("{ticker}: the TradingView export is not staged on this engine (served by /api/gl once setup.py stages it); no result is published")
}
/// /api/lab/instruments: every provisional instrument and whether this engine holds its data.
pub fn instruments_json(st: &crate::State) -> String {
    let opt = |s: Option<String>| s.map_or("null".into(), |s| format!("\"{s}\""));
    let rows: Vec<String> = GL.iter().map(|m| match st.ctxs.get(m.ticker).filter(|c| !c.r.is_empty()) {
        Some(c) => format!("{{\"instrument\":{},\"status\":\"ready\",\"first\":\"{}\",\"last\":\"{}\",\"bars\":{},\"tradable_from\":{}}}",
            m.json(), c.r[0].date, c.r[c.r.len() - 1].date, c.r.len(), opt(tradable_from(c.r, m.continuous))),
        None => format!("{{\"instrument\":{},\"status\":\"needs_data\",\"reason\":\"{}\"}}", m.json(), esc(&not_staged(m.ticker))),
    }).collect();
    format!("{{\"label\":\"{LABEL}\",\"instruments\":[{}]}}", rows.join(","))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lab::json::parse;
    fn row(date: &str, o: f64, h: f64, l: f64, c: f64) -> Row {
        Row { ticker: "T:X".into(), date: date.into(), open: o, high: h, low: l, close: c, volume: None }
    }

    #[test] fn every_gl_instrument_is_provisional_and_declares_its_calendar() {
        let mut seen = std::collections::BTreeSet::new();
        for m in GL {
            assert!(seen.insert(m.ticker) && m.ticker.contains(':'), "{}", m.ticker);
            assert!(matches!(m.kind, "index" | "cfd" | "crypto") && matches!(m.market, "gl" | "crypto"), "{}", m.ticker);
            assert_eq!(m.kind == "crypto", m.market == "crypto");
            if m.kind == "crypto" { assert!(m.continuous && m.bars_per_year == 365. && m.timezone == "UTC"); }
            else { assert!(!m.continuous && m.bars_per_year >= 252.); }
            let j = parse(&m.json()).unwrap();
            assert_eq!(j.get("ticker").unwrap().as_str(), Some(m.ticker));
            assert!(matches!(j.get("provisional"), Some(crate::lab::json::Json::Bool(true))));
            assert!(j.get("label").unwrap().as_str().unwrap().starts_with("Provisional · TradingView export "));
        }
        assert_eq!(gl("IG:DAX").unwrap().bars_per_year, 311.);
        assert!(gl("SPX").is_none() && gl("NIFTY 50").is_none() && gl("basket:us_sp500:AAPL").is_none());
    }
    #[test] fn tradable_from_skips_close_only_and_stale_open_years() {
        let mut rows = vec![row("2001-01-02", 10., 10., 10., 10.), row("2001-01-03", 11., 11., 11., 11.)]; // close-only year
        rows.extend([row("2002-01-02", 11., 12., 10., 11.5), row("2002-01-03", 11.5, 12., 11., 11.8)]); // opens = previous close
        rows.extend((0..9).map(|d| row(&format!("2003-01-{:02}", d + 2), 12. + d as f64, 13. + d as f64, 11. + d as f64, 12.5 + d as f64)));
        rows.push(row("2003-02-01", 20.5, 22., 20., 21.5)); // opens at the previous close: 1 stale bar in 10 is under 20%
        assert_eq!(tradable_from(&rows, false).as_deref(), Some("2003-01-01"));
        assert_eq!(tradable_from(&rows, true).as_deref(), Some("2002-01-01"), "around the clock an open at the previous close is normal");
        assert_eq!(tradable_from(&rows[4..], false), None);
        assert_eq!(tradable_from(&[], false), None);
    }
    #[test] fn annualisation_uses_the_instruments_bar_frequency() {
        let base: BTreeMap<&'static str, f64> = [("vol", 0.2), ("sharpe", 1.0), ("sortino", 1.5), ("gross_sharpe", 1.2), ("cagr", 0.1), ("trades_per_year", 4.)].into();
        let mut s = base.clone();
        annualise(&mut s, bars_per_year("NIFTY 50"));
        assert_eq!(s, base, "India/US results keep the book's sqrt(252)");
        annualise(&mut s, bars_per_year("CRYPTO:BTCUSD"));
        let f = (365f64 / 252.).sqrt();
        for k in ["vol", "sharpe", "sortino", "gross_sharpe"] { assert!((s[k] - base[k] * f).abs() < 1e-12, "{k}"); }
        assert_eq!((s["cagr"], s["trades_per_year"]), (0.1, 4.), "calendar-time statistics are unchanged");
    }
    #[test] fn engine_gl_serves_lab_routes_only_and_marks_every_body() {
        assert!(engine_route_allowed(None, "/api/guru/connors/run"));
        assert!(engine_route_allowed(Some("in"), "/api/tickers"));
        assert!(engine_route_allowed(Some("gl"), "/api/lab/run"));
        for p in ["/api/guru/connors/run", "/api/tickers", "/api/ohlc", "/api/notes", "/api/gurus", "/api/labx"] { assert!(!engine_route_allowed(Some("gl"), p), "{p}"); }
        for body in [refused_json("/api/guru/connors/run"), mark_provisional("{\"status\":\"invalid\",\"error\":\"x\",\"label\":\"l\"}")] {
            assert!(body.starts_with(&format!("{{\"provisional\":true,\"provisional_note\":\"{PROVISIONAL}\",")), "{body}");
            parse(&body).unwrap();
        }
    }
    #[test] fn a_cfd_sunday_bar_closes_its_monday_to_sunday_week() {
        use crate::lab::resample::{expand, resample, Tf};
        let rows = [row("2026-09-18", 10., 11., 9., 10.), row("2026-09-20", 10., 10.5, 9.5, 10.2), row("2026-09-21", 10.2, 12., 10., 11.)]; // Fri, Sun, Mon
        let d = crate::lab::bars::Bars::from_rows(&rows);
        let w = resample(&d, Tf::W);
        assert_eq!(w.date, ["2026-09-20", "2026-09-21"]);
        assert_eq!(w.close, [10.2, 11.]);
        assert_eq!(w.end, [1, 2]);
        let x = expand(&w.close, &w, 3);
        assert!(x[0].is_nan() && x[1] == 10.2, "the week ending on its Sunday bar is usable from that Sunday close, never on Friday");
    }
    /// Real-data check of the declared cuts: VAULT_GL_CSV = a gl_data.csv staged by Context/guru/webapp/scripts/stage_gl.py.
    #[test] fn staged_gl_data_matches_the_declared_cuts() {
        let Ok(path) = std::env::var("VAULT_GL_CSV") else { eprintln!("VAULT_GL_CSV not set; staged-data check skipped"); return };
        let data = crate::load_csv(&path).unwrap();
        let expected = [("BSE_DLY:SENSEX", Some("2010-01-01")), ("CRYPTO:BTCUSD", Some("2011-01-01")), ("CRYPTO:ETHUSD", None), ("HSI:HSI", Some("1992-01-01")),
            ("IG:DAX", None), ("IG:FTSE", Some("2011-01-01")), ("NASDAQ_DLY:IXIC", Some("2000-01-01")), ("SPREADEX:NIKKEI", None), ("TVC:CAC40", None), ("TVC:RUT", Some("2007-01-01"))];
        for (ticker, cut) in expected {
            let Some(rows) = data.get(ticker) else { eprintln!("{ticker} not staged"); continue };
            assert!(rows.iter().all(|r| r.volume.is_none()), "{ticker}: volume must not be staged");
            assert_eq!(tradable_from(rows, gl(ticker).unwrap().continuous).as_deref(), cut, "{ticker}");
        }
        assert!(data.keys().all(|t| gl(t).is_some()), "only declared instruments are staged: {:?}", data.keys().collect::<Vec<_>>());
    }
}
