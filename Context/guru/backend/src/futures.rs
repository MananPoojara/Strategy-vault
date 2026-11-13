//! Norgate continuous futures (R4 #293, lane F1, 13 Nov 2026).
//!
//! Data: the user's share DATABASE/GLOBAL DATABASE/norgate data 8SEP2020/Continuous Futures, mirrored read-only
//! (SOURCES.csv norgate-continuous-futures-20200908, 222/222 sha256); the engine reads the copy `futures/norgate-20200908`
//! next to its price file (us_data.csv), one `&SYM.csv` (unadjusted, spliced at the roll) and one `&SYM_CCB.csv`
//! (back-adjusted) per market. Only the markets in `MARKETS` are loaded, and only by the US engine.
//!
//! #293 (2): a futures context's bars are the BACK-ADJUSTED series (signals, indicators and point P&L: point differences
//! survive Norgate's additive roll adjustment); the unadjusted series is attached as RAW.Open/High/Low/Close for rules that
//! compare absolute price levels. Days without an unadjusted bar (6J _CCB 1976-08..1979-04) are not loaded.
//! Returns: each trade is priced in the contract held at entry (back-adjusted price + that day's roll offset, i.e. the
//! unadjusted price), so its % = point P&L / unadjusted entry price (= the raw close on the entry date for close entries).
//! Labelled on every card: "Norgate continuous futures, ends 2020-09-11; roll method not stated by the vendor" (#293 (5)).
use crate::{connors::{Ctx, Trade}, Row};
use std::collections::BTreeMap;

/// One continuous-futures market. `usd` = (first date in force, US$ per 1.00 of the quoted price), oldest first.
/// `tick` = the minimum price move in the quote (TradeStation "points" in entry/stop offsets are ticks, #57).
pub struct Market { pub sym: &'static str, pub name: &'static str, pub tick: f64, pub usd: &'static [(&'static str, f64)], pub source: &'static str }

/// Contract multipliers (#293 (3)): exchange contract specifications (reference data, not market data). Norgate quotes
/// currencies in US$ per unit (yen per 100 yen), metals/energy in the exchange's quote unit, T-bonds in points of par.
pub const MARKETS: &[Market] = &[
    Market { sym: "&6J", name: "Japanese Yen", tick: 0.0001, usd: &[("1972-05-16", 125_000.)],
        source: "CME Japanese yen futures: 12,500,000 yen per contract; Norgate quotes US$ per 100 yen, so 1.00 = US$125,000 (0.0001 = US$12.50, the tick of the book era)" },
    Market { sym: "&6A", name: "Australian Dollar", tick: 0.0001, usd: &[("1987-01-13", 100_000.)],
        source: "CME Australian dollar futures: 100,000 Australian dollars per contract (1.00 = US$100,000; 0.0001 = US$10)" },
    Market { sym: "&6B", name: "British Pound", tick: 0.0002, usd: &[("1975-01-02", 62_500.)],
        source: "CME British pound futures: 62,500 pounds per contract (1.00 = US$62,500; 0.0002 = US$12.50, the tick of the book era); no earlier contract size was found, so 62,500 applies to the whole series (labelled)" },
    Market { sym: "&GC", name: "Gold", tick: 0.1, usd: &[("1974-12-31", 100.)],
        source: "COMEX gold futures: 100 troy ounces per contract, quoted in US$ per ounce (US$1.00 = US$100; 0.10 = US$10)" },
    Market { sym: "&SP", name: "S&P 500", tick: 0.05, usd: &[("1982-04-21", 500.), ("1997-11-03", 250.)],
        source: "CME S&P 500 futures: US$500 x the index to 1997-11-02, US$250 x the index from 1997-11-03 (R4 #293 (3)); tick 0.05 of the book era" },
    Market { sym: "&6S", name: "Swiss Franc", tick: 0.0001, usd: &[("1972-05-16", 125_000.)],
        source: "CME Swiss franc futures: 125,000 francs per contract (1.00 = US$125,000; 0.0001 = US$12.50)" },
    Market { sym: "&CL", name: "Crude Oil", tick: 0.01, usd: &[("1983-03-30", 1_000.)],
        source: "NYMEX light sweet crude oil futures: 1,000 barrels per contract (US$1.00 = US$1,000; 0.01 = US$10)" },
    Market { sym: "&ZB", name: "US T-Bond", tick: 1. / 32., usd: &[("1977-08-22", 1_000.)],
        source: "CBOT U.S. Treasury bond futures: US$100,000 face value, quoted in points of par (1 point = US$1,000; 1/32 = US$31.25)" },
    // lane F3 (R4 #294 (1)): Harris's COMEX silver patterns
    Market { sym: "&SI", name: "Silver", tick: 0.5, usd: &[("1978-04-27", 50.)],
        source: "COMEX silver futures: 5,000 troy ounces per contract; Norgate quotes US cents per ounce, so 1.00 (one cent) = US$50; tick 0.5 cent = US$25 per the current CME contract specification (not used by the Harris patterns)" },
    // lane P1 (vvbats_spread, Van Vliet BATS ch.46): the E-mini legs of the ES/NQ ratio spread
    Market { sym: "&ES", name: "E-mini S&P 500", tick: 0.25, usd: &[("1997-09-09", 50.)],
        source: "CME E-mini S&P 500 futures: US$50 x the S&P 500 index since launch (9 Sep 1997); tick 0.25 index point = US$12.50" },
    Market { sym: "&NQ", name: "E-mini Nasdaq-100", tick: 0.25, usd: &[("1999-06-21", 20.)],
        source: "CME E-mini Nasdaq-100 futures: US$20 x the Nasdaq-100 index since launch (21 Jun 1999); tick 0.25 index point = US$5" },
    // lane PB (Pee TDI, S&C V19:10 Figure 2 basket): contract sizes from the exchange specifications (reference data); the TDI
    // reads closes and fills at the open, so these multipliers only label the contract
    Market { sym: "&KC", name: "Coffee", tick: 0.05, usd: &[("1978-12-14", 375.)],
        source: "ICE Coffee C futures: 37,500 lb per contract, quoted in US cents per lb (1.00 = US$375; 0.05 = US$18.75)" },
    Market { sym: "&HG", name: "Copper", tick: 0.05, usd: &[("1978-04-27", 250.)],
        source: "COMEX copper futures: 25,000 lb per contract; Norgate quotes US cents per lb (1.00 = US$250; 0.05 = US$12.50)" },
    Market { sym: "&CT", name: "Cotton", tick: 0.01, usd: &[("1978-09-19", 500.)],
        source: "ICE Cotton No. 2 futures: 50,000 lb per contract, quoted in US cents per lb (1.00 = US$500; 0.01 = US$5)" },
    Market { sym: "&GE", name: "Eurodollar", tick: 0.01, usd: &[("1982-02-01", 2_500.)],
        source: "CME Eurodollar futures: US$1,000,000 three-month deposit, quoted as an IMM index (1.00 = 100 basis points = US$2,500; 0.01 = US$25, the tick of the book era)" },
    Market { sym: "&HO", name: "Heating Oil", tick: 0.0001, usd: &[("1978-11-15", 42_000.)],
        source: "NYMEX NY Harbor heating oil (ULSD) futures: 42,000 gallons per contract, quoted in US$ per gallon (1.00 = US$42,000; 0.0001 = US$4.20)" },
    Market { sym: "&ZO", name: "Oats", tick: 0.25, usd: &[("1978-04-26", 50.)],
        source: "CBOT oats futures: 5,000 bushels per contract, quoted in US cents per bushel (1.00 = US$50; 0.25 = US$12.50)" },
    Market { sym: "&OJ", name: "Orange Juice", tick: 0.05, usd: &[("1978-08-08", 150.)],
        source: "ICE frozen concentrated orange juice futures: 15,000 lb per contract, quoted in US cents per lb (1.00 = US$150; 0.05 = US$7.50)" },
    // lane PA1 (Downs, S&C V.17:1 soybean gaps and equilateral quads)
    Market { sym: "&ZS", name: "Soybeans", tick: 0.25, usd: &[("1977-11-21", 50.)],
        source: "CBOT soybean futures: 5,000 bushels per contract; Norgate quotes US cents per bushel, so 1.00 (one cent) = US$50 (Downs V.17:1 Figure 2: +7 cents on 5 contracts = $1,750 before costs); tick 0.25 cent = US$12.50" },
    Market { sym: "&DX", name: "US Dollar Index", tick: 0.01, usd: &[("1985-11-20", 500.), ("1996-08-01", f64::NAN)],
        source: "FINEX/ICE US Dollar Index futures: Downs's printed P&L (V.17:1 Figure 15: +0.50 index points on 5 contracts = $875 after $375 costs) implies US$500 per index point in 1987-96 (R4 #323 (2)); the later US$1,000 contract's start date is not established, so from 1996-08-01 (after the article's last printed month) no multiplier is given and US$ amounts are refused (R4 #324 M1); tick 0.01 of the book era" },
    // lane PF (Abel, S&C V21:10 Figure 3 portfolio): contract sizes from the exchange specifications (reference data); the Abel
    // system's stops are standard-deviation distances, so these multipliers only label the contract
    Market { sym: "&NG", name: "Natural Gas", tick: 0.001, usd: &[("1990-04-03", 10_000.)],
        source: "NYMEX Henry Hub natural gas futures: 10,000 MMBtu per contract, quoted in US$ per MMBtu (1.00 = US$10,000; 0.001 = US$10)" },
    Market { sym: "&ZC", name: "Corn", tick: 0.25, usd: &[("1978-01-04", 50.)],
        source: "CBOT corn futures: 5,000 bushels per contract, quoted in US cents per bushel (1.00 = US$50; 0.25 = US$12.50)" },
    Market { sym: "&ZW", name: "Wheat", tick: 0.25, usd: &[("1978-01-05", 50.)],
        source: "CBOT Chicago SRW wheat futures: 5,000 bushels per contract, quoted in US cents per bushel (1.00 = US$50; 0.25 = US$12.50)" },
    Market { sym: "&CC", name: "Cocoa", tick: 1., usd: &[("1979-12-19", 10.)],
        source: "ICE cocoa futures: 10 metric tonnes per contract, quoted in US$ per tonne (1.00 = US$10)" },
    Market { sym: "&SB", name: "Sugar No. 11", tick: 0.01, usd: &[("1978-10-17", 1_120.)],
        source: "ICE Sugar No. 11 futures: 112,000 lb per contract, quoted in US cents per lb (1.00 = US$1,120; 0.01 = US$11.20)" },
    Market { sym: "&ZN", name: "10-Year T-Note", tick: 1. / 64., usd: &[("1982-05-03", 1_000.)],
        source: "CBOT 10-year U.S. Treasury note futures: US$100,000 face value, quoted in points of par (1 point = US$1,000; 1/64 = US$15.625)" },
];

pub fn ticker(m: &Market) -> String { format!("{} (Norgate {})", m.name, m.sym) }
/// The market of a futures context ticker ("US T-Bond (Norgate &ZB)").
pub fn of_ticker(t: &str) -> Option<&'static Market> {
    let sym = t.strip_suffix(')')?.rsplit_once(" (Norgate ")?.1;
    MARKETS.iter().find(|m| m.sym == sym)
}
pub fn is_futures(t: &str) -> bool { of_ticker(t).is_some() }
pub fn ctx_market(x: &Ctx) -> Option<&'static Market> { x.r.first().and_then(|r| of_ticker(&r.ticker)) }
/// US$ per 1.00 of the quote on `date` (the multiplier in force on the trade date, #293 (3)).
/// A NaN entry ends a known period: from that date the multiplier is unknown (None), so US$ rules fail closed (R4 #324 M1).
pub fn usd_per_point(m: &Market, date: &str) -> Option<f64> { m.usd.iter().rev().find(|(d, _)| *d <= date).map(|(_, v)| *v).filter(|v| v.is_finite()) }
/// A fixed US$ amount per contract in quote points on bar `i` of a futures context (None elsewhere).
pub fn usd_points(x: &Ctx, i: usize, usd: f64) -> Option<f64> { usd_per_point(ctx_market(x)?, &x.r[i].date).map(|v| usd / v) }

/// Loads the back-adjusted bars of every market in MARKETS from `dir` into `data` (key = ticker()) and returns the
/// unadjusted series as extra_data entries (ticker, RAW.Open/High/Low/Close). A missing folder loads nothing.
pub fn load(dir: &std::path::Path, data: &mut BTreeMap<String, Vec<Row>>) -> BTreeMap<(String, String), BTreeMap<String, f64>> {
    let mut extra = BTreeMap::new();
    for m in MARKETS {
        let (Some(adj), Some(raw)) = (read(&dir.join(format!("{}_CCB.csv", m.sym))), read(&dir.join(format!("{}.csv", m.sym)))) else {
            eprintln!("warning: futures {} not loaded from {}", m.sym, dir.display()); continue };
        let t = ticker(m);
        let raw: BTreeMap<String, Row> = raw.into_iter().map(|r| (r.date.clone(), r)).collect();
        let rows: Vec<Row> = adj.into_iter().filter(|r| raw.contains_key(&r.date)).map(|r| Row { ticker: t.clone(), ..r }).collect();
        // lead review of F3: a file that parsed to no common bars is not served (an empty context would break /api/tickers)
        if rows.is_empty() { eprintln!("warning: futures {} not loaded: 0 bars in both {}_CCB.csv and {}.csv", m.sym, m.sym, m.sym); continue; }
        for (k, f) in [("Open", 0), ("High", 1), ("Low", 2), ("Close", 3)] {
            extra.insert((t.clone(), format!("RAW.{k}")), rows.iter().map(|r| { let x = &raw[&r.date]; (r.date.clone(), [x.open, x.high, x.low, x.close][f]) }).collect());
        }
        println!("futures {t}: {} bars {} -> {}", rows.len(), rows.first().map_or("", |r| &r.date), rows.last().map_or("", |r| &r.date));
        data.insert(t, rows);
    }
    extra
}
fn read(path: &std::path::Path) -> Option<Vec<Row>> {
    let text = std::fs::read_to_string(path).ok()?;
    let mut lines = text.lines();
    let head = crate::parse_csv_fields(lines.next()?);
    let col = |n: &str| head.iter().position(|h| h == n);
    let (di, oi, hi, li, ci, vi) = (col("Date")?, col("Open")?, col("High")?, col("Low")?, col("Close")?, col("Volume"));
    let mut out: Vec<Row> = lines.filter(|l| !l.trim().is_empty()).filter_map(|l| {
        let f = crate::parse_csv_fields(l);
        let num = |i: usize| f.get(i).and_then(|v| crate::parse_csv_number(v).ok()).filter(|v| v.is_finite());
        Some(Row { ticker: String::new(), date: crate::normalize_date(f.get(di)?).or_else(|| compact_date(f.get(di)?))?, open: num(oi)?, high: num(hi)?, low: num(li)?, close: num(ci)?, volume: vi.and_then(num).filter(|v| *v > 0.) })
    }).collect();
    out.sort_by(|a, b| a.date.cmp(&b.date));
    out.dedup_by(|a, b| a.date == b.date);
    Some(out)
}

/// Lane F3 (R4 #294 (2)): Norgate US indices and Nasdaq breadth, copied read-only to futures/norgate-us-indices-20200908 next to
/// us_data.csv (SOURCES.csv norgate-us-indices-20200908-f3). (file, series): each file's close becomes a series every US context
/// reads by date (NaN where the file has no bar). Other lanes add their US Indices files here.
pub const INDEX_SERIES: &[(&str, &str)] = &[("$COMP", "NASDAQ_COMP"), ("$NYA", "NYSE_COMP"), ("#NASHI", "NASDAQ_NEW_HIGHS"), ("#NASLO", "NASDAQ_NEW_LOWS"),
    // lane PB: NYSE 52-week new highs / lows 1932-2020 (SOURCES norgate-us-indices-20200908-pb)
    ("#NYSEHI", "NYSE_NEW_HIGHS"), ("#NYSELO", "NYSE_NEW_LOWS"), ("$DJU", "DJ_UTILITY"),
    // lane PF: the DJIA close for Meyers' DJstr/DJCstr (SOURCES norgate-us-indices-20200908-pf)
    ("$DJI", "DJ_INDUSTRIAL")];
pub fn load_indices(dir: &std::path::Path) -> BTreeMap<(String, String), BTreeMap<String, f64>> {
    let mut out = BTreeMap::new();
    for (file, name) in INDEX_SERIES {
        let Ok(text) = std::fs::read_to_string(dir.join(format!("{file}.csv"))) else { eprintln!("warning: index {file} not loaded from {}", dir.display()); continue };
        let mut lines = text.lines();
        let head = crate::parse_csv_fields(lines.next().unwrap_or(""));
        let (Some(di), Some(ci)) = (head.iter().position(|h| h == "Date"), head.iter().position(|h| h == "Close")) else { continue };
        let m: BTreeMap<String, f64> = lines.filter_map(|l| {
            let f = crate::parse_csv_fields(l);
            let d = f.get(di)?; // Norgate compact yyyymmdd
            let date = (d.len() == 8 && d.bytes().all(|b| b.is_ascii_digit())).then(|| format!("{}-{}-{}", &d[..4], &d[4..6], &d[6..]))?;
            Some((date, crate::parse_csv_number(f.get(ci)?).ok().filter(|v| v.is_finite())?))
        }).collect();
        println!("index {file} -> {name}: {} days", m.len());
        out.insert(("*".to_string(), name.to_string()), m);
    }
    out
}

/// Lane PF (Meyers, S&C V15:11): the CBOT T-bond continuous future, back-adjusted close (Norgate &ZB_CCB in futures/norgate-20200908,
/// SOURCES norgate-continuous-futures-20200908; R4 #293 (2): indicators on the back-adjusted series) as a series every US context reads
/// by date. A missing file loads nothing.
pub const TBOND_SERIES: &str = "TBOND_CCB";
pub fn load_tbond(dir: &std::path::Path) -> Option<BTreeMap<String, f64>> {
    let rows = read(&dir.join("&ZB_CCB.csv")).filter(|r| !r.is_empty());
    if rows.is_none() { eprintln!("warning: {TBOND_SERIES} not loaded from {}", dir.display()); }
    Some(rows?.into_iter().map(|r| (r.date, r.close)).collect())
}

/// Norgate's compact yyyymmdd date (the US Indices files) as yyyy-mm-dd.
fn compact_date(d: &str) -> Option<String> { (d.len() == 8 && d.bytes().all(|b| b.is_ascii_digit())).then(|| format!("{}-{}-{}", &d[..4], &d[4..6], &d[6..])) }

/// Lane PE (R4 #319): the NYSE Composite as its own context, from Norgate $NYA (futures/norgate-us-indices-20200908/$NYA.csv,
/// SOURCES.csv norgate-us-indices-20200908-f3), so Holt's NYSE Composite systems run on the index they were tested on. Close and
/// volume are real from 1970-01-05 (volume 0 before = no volume); Norgate's open/high/low equal the close before 1988-12-07, so
/// only the close(-and-volume) cards in NYA_CARDS are admitted (instrument_policy::check). A missing file loads nothing.
pub const NYA_TICKER: &str = "NYSE Composite (Norgate $NYA)";
pub const NYA_CARDS: &[&str] = &["scholt_obvdi", "scholt_nvdi",
    // lane PF: Appel & Hitschler's close-only NYSE Index systems (Stock Market Trading Systems, Tables 4-5, 4-6, 5-1, 6-2, 9-1)
    "apsmts_confirm", "apsmts_shift", "apsmts_mmacd", "apsmts_lead", "apsmts_channel"];
pub fn load_nya(dir: &std::path::Path, data: &mut BTreeMap<String, Vec<Row>>) {
    let Some(rows) = read(&dir.join("$NYA.csv")).filter(|r| !r.is_empty()) else { eprintln!("warning: index $NYA context not loaded from {}", dir.display()); return };
    println!("context {NYA_TICKER}: {} bars {} -> {}", rows.len(), rows[0].date, rows[rows.len() - 1].date);
    data.insert(NYA_TICKER.into(), rows.into_iter().map(|r| Row { ticker: NYA_TICKER.into(), ..r }).collect());
}

/// The unadjusted close of a futures context (RAW.Close), aligned to its bars.
pub fn raw_close<'a>(x: &'a Ctx) -> Option<&'a [f64]> { ctx_market(x)?; x.series("RAW.Close") }

/// #293 (2) return basis. Each trade is shifted by the roll offset of its entry bar (unadjusted - back-adjusted close),
/// so its prices are those of the contract held at entry and point P&L is unchanged. Returns the shifted trades and the
/// close series the equity curve marks open trades with (back-adjusted close + the trade's offset, per trade window).
pub fn raw_basis(x: &Ctx, trades: Vec<Trade>) -> Result<(Vec<Trade>, Vec<f64>), String> {
    let raw = raw_close(x).ok_or("needs_data: the unadjusted futures series (RAW.Close) is not loaded")?;
    let mut close = x.c.clone();
    let out = trades.into_iter().map(|mut t| {
        let e = t.legs[0].i;
        let off = raw[e] - x.c[e]; // R4 #301: the % base is the unadjusted entry fill (the entry day's roll offset applied)
        if !off.is_finite() { return Err(format!("needs_data: no unadjusted futures close on {}", x.r[e].date)); }
        for l in &mut t.legs { l.px += off; }
        t.exit_px += off;
        for p in &mut t.exits { p.1 += off; }
        for k in e..=t.exit_i { close[k] = x.c[k] + off; }
        if t.legs.iter().any(|l| l.px <= 0.) { return Err(format!("calculation_unavailable: the unadjusted entry price on {} is not positive (the % return has no base); no result is published", x.r[e].date)); }
        Ok(t)
    }).collect::<Result<Vec<_>, String>>()?;
    Ok((out, close))
}
/// Lane F3 (lead review, shared-loader blocker): the roll offset a futures trade was shifted by (unadjusted - back-adjusted
/// close of its entry bar; raw_basis), so code that reads the context's back-adjusted bars against a shifted trade can put
/// them on the trade's prices (MAE/MFE path, entry-type checks). 0 elsewhere.
pub fn trade_offset(x: &Ctx, t: &Trade) -> f64 {
    let e = t.legs[0].i;
    raw_close(x).and_then(|r| r.get(e)).map_or(0., |r| r - x.c[e])
}
/// The close series an equity curve marks shifted futures trades with (back-adjusted close + each trade's offset over its
/// window), None on a non-futures context. Gross equity / gross statistics use it like normalized_run does.
pub fn marks(x: &Ctx, trades: &[Trade]) -> Option<Vec<f64>> {
    raw_close(x)?;
    let mut close = x.c.clone();
    for t in trades { let off = trade_offset(x, t); for k in t.legs[0].i..=t.exit_i.min(close.len() - 1) { close[k] = x.c[k] + off; } }
    Some(close)
}
/// Buy and hold on a futures context: point change of the back-adjusted close over the unadjusted close at the start.
pub fn buy_hold(x: &Ctx, from: usize, to: usize) -> Option<Vec<f64>> {
    let raw = raw_close(x)?;
    Some((from..=to).map(|k| 1. + (x.c[k] - x.c[from]) / raw[from]).collect())
}
/// Readiness: OHLC must be finite and ordered on the back-adjusted bars (which may be negative far back) and finite on
/// the unadjusted ones. false = valid.
pub fn invalid_bar(x: &Ctx, i: usize) -> bool {
    let r = &x.r[i];
    let ok = [r.open, r.high, r.low, r.close].iter().all(|v| v.is_finite()) && r.low <= r.high && r.open >= r.low && r.open <= r.high && r.close >= r.low && r.close <= r.high;
    // an unadjusted price may be negative (NYMEX crude settled at -37.63 on 2020-04-20); a trade entered on such a day has no
    // % base and is refused by raw_basis
    let raw_ok = ["RAW.Open", "RAW.High", "RAW.Low", "RAW.Close"].iter().all(|k| x.series(k).and_then(|s| s.get(i)).is_some_and(|v| v.is_finite()));
    !(ok && raw_ok)
}

/// R4 #316 (lane X4): cards whose published run ends the day before the first non-positive unadjusted price (NYMEX crude
/// settled at -37.63 on 2020-04-20, where a % return has no base); readiness::auto_end / check apply it. The series itself
/// stays whole for every other card (raw_basis refuses a trade whose unadjusted entry price is not positive).
const POSITIVE_RAW_ONLY: &[&str] = &["jkbwts_ehrlich17"];
/// The first bar whose unadjusted open/high/low/close is <= 0, for a POSITIVE_RAW_ONLY card on a futures context.
pub fn first_nonpositive(id: &str, x: &Ctx) -> Option<usize> {
    if !POSITIVE_RAW_ONLY.contains(&id) { return None; }
    let s = ["RAW.Open", "RAW.High", "RAW.Low", "RAW.Close"].map(|k| x.series(k));
    if s.iter().any(|v| v.is_none()) { return None; }
    (0..x.c.len()).find(|&i| s.iter().flatten().any(|v| v.get(i).is_some_and(|p| *p <= 0.)))
}

/// Cards that run on one named continuous contract (#293 (1)): (strategy id, market symbol, instrument_policy basis).
pub const CARDS: &[(&str, &str, &str)] = &[
    ("ataa_moon", "&6A", "ATAA Journal Dec 1990 p.11-12 (Krastins, Sheer Lunacy): the moon-phase stop and reverse tested on AUD/USD IMM futures, run on the Norgate Australian dollar continuous contract (R4 #293 (1)); close fills, no amounts."),
    ("jkbwts_neeley31", "&6J", "Krutsinger, Building Winning Trading Systems (seminar manual, Dec 1997) p.22-23 Neeley31, Japanese Yen CME weekly: the printed Quick System sheet on weekly bars built from the Norgate continuous contract's daily bars (R4 #293 (1)); $ amounts at the contract's $ per point (futures::MARKETS)."),
    ("jkbwts_wagner31", "&GC", "Krutsinger, Building Winning Trading Systems (seminar manual, Dec 1997) p.30-31 Wagner31, COMEX gold weekly: the printed Quick System sheet on weekly bars built from the Norgate continuous contract's daily bars (R4 #293 (1)); $ amounts at the contract's $ per point (futures::MARKETS)."),
    ("stad10_mwt", "&SP", "STAD Vol 10 p.11-17 (Joe Krutsinger, JK Monday We Trade): the printed code with SetStopLoss $1,500 per position, run on the Norgate S&P 500 continuous contract; $1,500 at the multiplier in force on the trade date ($500 per point to 1997-11-02, $250 after, R4 #293 (3))."),
    ("ch_bta2_vol2x", "&6S", "Chande, Beyond Technical Analysis (1997) p.236-239, Table 8.4: the 2x-range volatility system tested on Swiss franc continuous futures 6/30/89-6/30/95 with a $3,000 initial stop, run on the Norgate Swiss franc continuous contract (R4 #293 (1)); the $3,000 at $125,000 per 1.00."),
    ("jkbwts_bwilliams19", "&6J", "Krutsinger, Building Winning Trading Systems (seminar manual, Dec 1997) p.32-33 B Williams19, Japanese Yen CME daily: the printed TradeStation code / Quick System sheet run on the Norgate continuous contract of the tested market (R4 #293 (1)); $ amounts at the contract's $ per point (futures::MARKETS)."),
    ("jkbwts_detrend", "&6S", "Krutsinger, Building Winning Trading Systems (seminar manual, Dec 1997) p.24-25 DeTrend (Jeff Roy DeTrend Oscillator), Swiss Franc daily: the printed TradeStation code run on the Norgate continuous contract of the tested market (R4 #293 (1)); the 0.01 threshold is in the contract's own quote."),
    ("jkbwts_ehrlich17", "&CL", "Krutsinger, Building Winning Trading Systems (seminar manual, Dec 1997) p.6-7 Ehrlich17, NY light crude oil daily: the printed TradeStation code run on the Norgate continuous contract of the tested market (R4 #293 (1)); $ amounts at the contract's $ per point (futures::MARKETS)."),
    ("jkbwts_conner19", "&6J", "Krutsinger, Building Winning Trading Systems (seminar manual, Dec 1997) p.2-3 Conner 19, Japanese Yen CME daily: the printed TradeStation code / Quick System sheet run on the Norgate continuous contract of the tested market (R4 #293 (1)); $ amounts at the contract's $ per point (futures::MARKETS)."),
    ("jkbwts_ez_bonder", "&ZB", "Krutsinger, Building Winning Trading Systems (seminar manual, Dec 1997) p.20-21 EZ Bonder, Day T-Bonds daily: the printed TradeStation code / Quick System sheet run on the Norgate continuous contract of the tested market (R4 #293 (1)); $ amounts at the contract's $ per point (futures::MARKETS)."),
    ("jkbwts_fox31", "&6J", "Krutsinger, Building Winning Trading Systems (seminar manual, Dec 1997) p.8-9 Fox31, Japanese Yen CME daily: the printed TradeStation code / Quick System sheet run on the Norgate continuous contract of the tested market (R4 #293 (1)); $ amounts at the contract's $ per point (futures::MARKETS)."),
    ("jkbwts_kase30", "&6B", "Krutsinger, Building Winning Trading Systems (seminar manual, Dec 1997) p.14-15 Kase30, British Pound daily: the printed TradeStation code / Quick System sheet run on the Norgate continuous contract of the tested market (R4 #293 (1)); $ amounts at the contract's $ per point (futures::MARKETS)."),
    ("jkbwts_one_night_stand", "&6J", "Krutsinger, Building Winning Trading Systems (seminar manual, Dec 1997) p.16-17 One Night Stand, Japanese Yen CME daily: the printed TradeStation code / Quick System sheet run on the Norgate continuous contract of the tested market (R4 #293 (1)); $ amounts at the contract's $ per point (futures::MARKETS)."),
    ("jkbwts_stuckey19", "&6J", "Krutsinger, Building Winning Trading Systems (seminar manual, Dec 1997) p.28-29 Stuckey19, Japanese Yen CME daily: the printed TradeStation code / Quick System sheet run on the Norgate continuous contract of the tested market (R4 #293 (1)); $ amounts at the contract's $ per point (futures::MARKETS)."),
    ("jkbwts_williams19", "&6J", "Krutsinger, Building Winning Trading Systems (seminar manual, Dec 1997) p.34-35 Williams 19, Japanese Yen CME daily: the printed TradeStation code / Quick System sheet run on the Norgate continuous contract of the tested market (R4 #293 (1)); $ amounts at the contract's $ per point (futures::MARKETS)."),
];
/// Lane F3 (R4 #294): Harris's single-market patterns (books::mhstp), each on the contract its Pattern Library page names.
const CARDS_F3: &[(&str, &str, &str)] = &[
    ("mhs_ua830406_1", "&ZB", MHS_TB), ("mhs_ua830407_1", "&ZB", MHS_TB), ("mhs_ua830826_1", "&ZB", MHS_TB), ("mhs_ua830131_1", "&ZB", MHS_TB),
    ("mhs_ua830304_1", "&ZB", MHS_TB), ("mhs_ua830125_1", "&ZB", MHS_TB), ("mhs_ua830418_1", "&ZB", MHS_TB), ("mhs_us800310_1", "&ZB", MHS_TB),
    ("mhs_us801701_1", "&ZB", MHS_TB), ("mhs_us810220_1", "&ZB", MHS_TB), ("mhs_us800111_1", "&ZB", MHS_TB), ("mhs_hd_rt_long", "&ZB", MHS_TB),
    ("mhs_cl840221_1", "&CL", MHS_CL), ("mhs_cl850210_1", "&CL", MHS_CL), ("mhs_cl850919_1", "&CL", MHS_CL), ("mhs_cl841206_1", "&CL", MHS_CL),
    ("mhs_sf830520_1", "&6S", MHS_SF), ("mhs_sf831011_3", "&6S", MHS_SF), ("mhs_sf830519_1", "&6S", MHS_SF),
    ("mhs_si830520_1", "&SI", MHS_SI), ("mhs_si830630_1", "&SI", MHS_SI),
    // inter-market: the card runs on the TRADED contract; the pattern is read from the other market (books::mhstp::INTER)
    ("mhs_sp830128_2", "&ZB", MHS_INTER), ("mhs_sp830107_2", "&ZB", MHS_INTER), ("mhs_sp830825_2", "&ZB", MHS_INTER), ("mhs_sp830405_2", "&ZB", MHS_INTER),
    ("mhs_us881011_2", "&SP", MHS_INTER), ("mhs_sf850419_2", "&SI", MHS_INTER),
    ("rgc_im_sp_tbond", "&SP", "Ruggiero, Cybernetic Trading Strategies (Wiley 1997) p.30-31 Table 1.4 (S&P500/T-Bond divergence, back-adjusted continuous contracts 4/21/82-2/7/96, $50 costs) and Table 7.5 p.117 (the rule as EasyLanguage, run without its MEM filter): run on the Norgate S&P 500 continuous contract with the Norgate T-bond contract as Data2 (R4 #293 (1), #294 (1)), lane F3."),
];
const MHS_INTER: &str = "Michael Harris, Short-Term Trading with Price Patterns (Traders Press 1998/2000) p.194-205 (inter-market patterns): the printed pattern logic on one contract (S&P 500, T-bond or Swiss franc futures), the position in another (T-bonds, S&P 500 or COMEX silver) at the Open of Tomorrow, fixed profit target / protective stop in the traded contract's points, good-til-cancelled (p.63); both Norgate continuous contracts, back-adjusted, joined by exact date (R4 #293 (1), #294 (1)), lane F3.";
const MHS_TB: &str = "Michael Harris, Short-Term Trading with Price Patterns (Traders Press 1998/2000) p.131-160 (T-Bond patterns), p.64-66 (H&D RT LONG): the printed pattern logic, entry (close or open of tomorrow) and fixed profit target / protective stop in T-bond points (1 point = 1.00 of par = $1,000), good-til-cancelled (p.63), one position per pattern; run on the Norgate continuous T-bond contract (R4 #293 (1), #294 (1)), lane F3. Norgate &ZB session coverage not stated.";
const MHS_CL: &str = "Michael Harris, Short-Term Trading with Price Patterns (Traders Press 1998/2000) p.162-170 (NYMEX crude oil patterns): the printed pattern logic, entry and fixed profit target / protective stop in crude points (1 point = $1.00 a barrel = $1,000), good-til-cancelled (p.63), one position per pattern; run on the Norgate continuous crude oil contract (R4 #293 (1), #294 (1)), lane F3.";
const MHS_SF: &str = "Michael Harris, Short-Term Trading with Price Patterns (Traders Press 1998/2000) p.172-178 (IMM Swiss franc patterns): the printed pattern logic, entry and fixed profit target / protective stop in franc points (1 point = 0.01 US$ per franc = $1,250), good-til-cancelled (p.63), one position per pattern; run on the Norgate continuous Swiss franc contract (R4 #293 (1), #294 (1)), lane F3.";
const MHS_SI: &str = "Michael Harris, Short-Term Trading with Price Patterns (Traders Press 1998/2000) p.188-190 (COMEX silver patterns): the printed pattern logic, entry and fixed profit target / protective stop in silver points (1 point = 1 cent an ounce = $50), good-til-cancelled (p.63), one position per pattern; run on the Norgate continuous silver contract (R4 #293 (1), #294 (1)), lane F3.";
pub const LABEL: &str = "Norgate continuous futures, ends 2020-09-11; roll method not stated by the vendor.";
/// Lane P1: two-leg spreads on two continuous contracts, run in the first contract's context.
const CARDS_P1: &[(&str, &str, &str)] = &[("vvbats_spread", "&ES", "Benjamin Van Vliet, Building Automated Trading Systems (Academic Press 2007) ch.46 p.290-301 (Spreader_Example: spread = -1 x ES + 2 x NQ, z of the last 30 spread prices, |z| > 2 entry, target z = 0, stop 4 ES ticks): run on the Norgate E-mini S&P 500 continuous contract with the Norgate E-mini Nasdaq-100 contract as the second leg, joined by exact date, daily closes instead of bid/ask quotes (R4 #293 (1), #297), lane P1.")];
/// (traded contract, second leg): the second leg's back-adjusted and unadjusted closes are attached to the traded contract's
/// context as `<sym>.Close` / `<sym>.RAW.Close` (extra series: exact date, NaN where it has no bar).
pub const LEG2: &[(&str, &str)] = &[("&ES", "&NQ")];
pub fn leg2_series(data: &BTreeMap<String, Vec<Row>>, extra: &mut BTreeMap<(String, String), BTreeMap<String, f64>>) {
    for (a, b) in LEG2 {
        let (Some(ma), Some(mb)) = (MARKETS.iter().find(|m| m.sym == *a), MARKETS.iter().find(|m| m.sym == *b)) else { continue };
        let (ta, tb) = (ticker(ma), ticker(mb));
        let Some(rows) = data.get(&tb) else { continue };
        extra.insert((ta.clone(), format!("{b}.Close")), rows.iter().map(|r| (r.date.clone(), r.close)).collect());
        if let Some(raw) = extra.get(&(tb, "RAW.Close".to_string())).cloned() { extra.insert((ta, format!("{b}.RAW.Close")), raw); }
    }
}
/// Lane PF: Saitta (S&C V14:3) on the T-bond; Abel (S&C V21:10) on each Figure 3 contract held (the regular Nasdaq-100 contract is
/// not held: its E-mini, which the article names, stands in).
const ABEL: &str = "Phil Abel, Diversification And Your Portfolio, S&C V21:10 p.20-31 (Figure 3: 16 US futures, 1995-2002): the printed SMA 25/50/100 system with the 4-standard-deviation hard stop on this Norgate continuous contract (R4 #293 (1)), lane PF.";
const CARDS_PF: &[(&str, &str, &str)] = &[
    ("scbrower_long", "&SP", "William Brower, The S&P 500 Seasonal Day Trade, S&C V14:8 p.338-342: the buy patterns with their DoWinMo filters on the Norgate continuous S&P 500 contract, back-adjusted daily bars (R4 #293 (1)), lane PF."),
    ("scbrower_short", "&SP", "William Brower, The S&P 500 Seasonal Day Trade, S&C V14:8 p.338-342: the sell patterns with their DoWinMo filters on the Norgate continuous S&P 500 contract, back-adjusted daily bars (R4 #293 (1)), lane PF."),
    ("scsaitta_volma", "&ZB", "Alex Saitta, A Price- And Volume-Based System, S&C V14:3 p.103-105: the T-bond 20-day moving average with the volume condition on the Norgate continuous T-bond contract, back-adjusted daily bars (R4 #293 (1)), lane PF."),
    ("scabeldiv_sma", "&6J", ABEL),
    ("scabeldiv_sma", "&6S", ABEL),
    ("scabeldiv_sma", "&GC", ABEL),
    ("scabeldiv_sma", "&SI", ABEL),
    ("scabeldiv_sma", "&SP", ABEL),
    ("scabeldiv_sma", "&NQ", ABEL),
    // ("scabeldiv_sma", "&CL"): not run: a crude trade held through 2020-04-20 (the short of 2020-02-27 at 47.09) is priced below zero in the contract held at entry (its mark is -38.32 on 2020-04-20, exit -15.84), so the engine's trade validation refuses the whole run
    ("scabeldiv_sma", "&NG", ABEL),
    ("scabeldiv_sma", "&ZC", ABEL),
    ("scabeldiv_sma", "&ZW", ABEL),
    ("scabeldiv_sma", "&CC", ABEL),
    ("scabeldiv_sma", "&SB", ABEL),
    ("scabeldiv_sma", "&CT", ABEL),
    ("scabeldiv_sma", "&KC", ABEL),
    ("scabeldiv_sma", "&ZN", ABEL),
    ("scabeldiv_sma", "&GE", ABEL),
];
/// Lane PB: Konstenius wave 1 (S&C V14:12), each printed parameter set on its own contract.
const CARDS_PB: &[(&str, &str, &str)] = &[
    ("mhptdi_tdi", "&ZB", "M.H. Pee, Trend Detection Index, S&C V19:10 p.54-61 and Figure 2 (15 futures, 16 years): the sidebar spreadsheet rule on this Norgate continuous contract (one of the 14 Figure 2 markets the vault holds; no Deutschemark), back-adjusted (R4 #293 (1)), lane PB."),
    ("mhptdi_tdi", "&6B", "M.H. Pee, Trend Detection Index, S&C V19:10 p.54-61 and Figure 2 (15 futures, 16 years): the sidebar spreadsheet rule on this Norgate continuous contract (one of the 14 Figure 2 markets the vault holds; no Deutschemark), back-adjusted (R4 #293 (1)), lane PB."),
    ("mhptdi_tdi", "&KC", "M.H. Pee, Trend Detection Index, S&C V19:10 p.54-61 and Figure 2 (15 futures, 16 years): the sidebar spreadsheet rule on this Norgate continuous contract (one of the 14 Figure 2 markets the vault holds; no Deutschemark), back-adjusted (R4 #293 (1)), lane PB."),
    // ("mhptdi_tdi", "&CL"): not run: the 2020-04-20 negative crude settlement leaves a TDI trade without a % base (raw_basis refuses the whole run)
    ("mhptdi_tdi", "&HG", "M.H. Pee, Trend Detection Index, S&C V19:10 p.54-61 and Figure 2 (15 futures, 16 years): the sidebar spreadsheet rule on this Norgate continuous contract (one of the 14 Figure 2 markets the vault holds; no Deutschemark), back-adjusted (R4 #293 (1)), lane PB."),
    ("mhptdi_tdi", "&CT", "M.H. Pee, Trend Detection Index, S&C V19:10 p.54-61 and Figure 2 (15 futures, 16 years): the sidebar spreadsheet rule on this Norgate continuous contract (one of the 14 Figure 2 markets the vault holds; no Deutschemark), back-adjusted (R4 #293 (1)), lane PB."),
    ("mhptdi_tdi", "&GE", "M.H. Pee, Trend Detection Index, S&C V19:10 p.54-61 and Figure 2 (15 futures, 16 years): the sidebar spreadsheet rule on this Norgate continuous contract (one of the 14 Figure 2 markets the vault holds; no Deutschemark), back-adjusted (R4 #293 (1)), lane PB."),
    ("mhptdi_tdi", "&GC", "M.H. Pee, Trend Detection Index, S&C V19:10 p.54-61 and Figure 2 (15 futures, 16 years): the sidebar spreadsheet rule on this Norgate continuous contract (one of the 14 Figure 2 markets the vault holds; no Deutschemark), back-adjusted (R4 #293 (1)), lane PB."),
    ("mhptdi_tdi", "&HO", "M.H. Pee, Trend Detection Index, S&C V19:10 p.54-61 and Figure 2 (15 futures, 16 years): the sidebar spreadsheet rule on this Norgate continuous contract (one of the 14 Figure 2 markets the vault holds; no Deutschemark), back-adjusted (R4 #293 (1)), lane PB."),
    ("mhptdi_tdi", "&6J", "M.H. Pee, Trend Detection Index, S&C V19:10 p.54-61 and Figure 2 (15 futures, 16 years): the sidebar spreadsheet rule on this Norgate continuous contract (one of the 14 Figure 2 markets the vault holds; no Deutschemark), back-adjusted (R4 #293 (1)), lane PB."),
    ("mhptdi_tdi", "&ZO", "M.H. Pee, Trend Detection Index, S&C V19:10 p.54-61 and Figure 2 (15 futures, 16 years): the sidebar spreadsheet rule on this Norgate continuous contract (one of the 14 Figure 2 markets the vault holds; no Deutschemark), back-adjusted (R4 #293 (1)), lane PB."),
    ("mhptdi_tdi", "&OJ", "M.H. Pee, Trend Detection Index, S&C V19:10 p.54-61 and Figure 2 (15 futures, 16 years): the sidebar spreadsheet rule on this Norgate continuous contract (one of the 14 Figure 2 markets the vault holds; no Deutschemark), back-adjusted (R4 #293 (1)), lane PB."),
    ("mhptdi_tdi", "&6S", "M.H. Pee, Trend Detection Index, S&C V19:10 p.54-61 and Figure 2 (15 futures, 16 years): the sidebar spreadsheet rule on this Norgate continuous contract (one of the 14 Figure 2 markets the vault holds; no Deutschemark), back-adjusted (R4 #293 (1)), lane PB."),
    ("mhptdi_tdi", "&SI", "M.H. Pee, Trend Detection Index, S&C V19:10 p.54-61 and Figure 2 (15 futures, 16 years): the sidebar spreadsheet rule on this Norgate continuous contract (one of the 14 Figure 2 markets the vault holds; no Deutschemark), back-adjusted (R4 #293 (1)), lane PB."),
    ("konwv_yen", "&6J", "Jeremy G. Konstenius, Combining Trend And Oscillator Signals, S&C V14:12 p.520-525: the wave 1 model with the yen parameters (30/5/50 weeks) on the Norgate continuous Japanese yen contract, back-adjusted weekly bars (R4 #293 (1)), lane PB."),
    ("konwv_bond", "&ZB", "Jeremy G. Konstenius, Combining Trend And Oscillator Signals, S&C V14:12 p.520-525: the wave 1 model with the T-bond parameters (20/10/30 weeks) on the Norgate continuous T-bond contract, back-adjusted weekly bars (R4 #293 (1)), lane PB."),
];
/// Lane PA1: Downs's tactical patterns (books::scdtac), each on the contract the article tests (S&C V.17:1).
const DOWNS: &str = "Walter T. Downs, The Tactical Trader, Technical Analysis of Stocks & Commodities V.17:1 (Jan 1999) p.27-37: weekly soybean gaps and monthly equilateral quads, each tested by the author on one futures market (Omega back-adjusted continuous data); run on the Norgate continuous contract of that market (R4 #293 (1)), lane PA1.";
const CARDS_PA1: &[(&str, &str, &str)] = &[("scdtac_gaps_zs", "&ZS", DOWNS)]; // the quads ids are duplicates of wtdtt_m_equi_quads (R4 #324 B1)
/// Lane PA2 (13 Nov 2026): Barrie's CBOT T-bond tests (books::scsbqs), on the T-bond continuous contract the article names.
const SBQS: &str = "Scott Barrie, 'Quasi-Seasonal Tendencies in Bond Futures', Technical Analysis of Stocks & Commodities V.14:1 (1996) p.19-25 (Figures 4, 7, 10: CBOT long bond futures 1979-1994, close-to-close, no costs): run on the Norgate T-bond continuous contract only (R4 #293 (1))";
const JMCT: &str = "John L. Momsen, 'Trade Crude Now', Technical Analysis of Stocks & Commodities V.19:8 (2001) p.53-56, sidebar 'Crude oil seasonal trade #2' (TradeStation code; November crude oil futures 1983-1999): run on the Norgate crude-oil continuous contract only (R4 #293 (1))";
const CARDS_PA2: &[(&str, &str, &str)] = &[("scsbqs_tuesday", "&ZB", SBQS), ("scsbqs_month_end", "&ZB", SBQS), ("scsbqs_april15", "&ZB", SBQS), ("scjmct_crude2", "&CL", JMCT), ("scasrb_breakout", "&ZB", ASRB)];
const ASRB: &str = "Alex Saitta, 'Range Breakout Trading in Treasury Bonds', Technical Analysis of Stocks & Commodities V.13:4 (1995) p.149-152 (Figure 4: one T-bond contract 1984-1994, closes, no costs; Morris's MetaStock transcription msws_gm9_range_breakout is held as NotTestable): run on the Norgate T-bond continuous contract only (R4 #293 (1))";
/// Every lane's card list; card() and home() both read this, so a new list is added once here.
fn all_cards() -> impl Iterator<Item = &'static (&'static str, &'static str, &'static str)> { CARDS.iter().chain(CARDS_F3).chain(CARDS_P1).chain(CARDS_PB).chain(CARDS_PA1).chain(CARDS_PA2).chain(CARDS_PF) }
fn card(id: &str) -> Option<&'static (&'static str, &'static str, &'static str)> { all_cards().find(|c| c.0 == id) }
pub fn card_basis(def: &crate::connors::StrategyDef) -> Option<&'static str> { card(def.id).map(|c| c.2) }
/// Readiness: a futures card runs only on the continuous contract its source names (None = this is that contract, or not a futures card).
pub fn home(id: &str, t: &str) -> Option<String> {
    let c = card(id)?;
    // lane PB: a card may list several contracts (one tuple each); any of them is home
    let homes: Vec<&Market> = all_cards().filter(|k| k.0 == id).filter_map(|k| MARKETS.iter().find(|m| m.sym == k.1)).collect();
    let m = MARKETS.iter().find(|m| m.sym == c.1)?;
    let sym = of_ticker(t).map(|m| m.sym);
    if homes.iter().any(|h| Some(h.sym) == sym) { return None; }
    Some(if homes.len() > 1 { format!("Not published on {t}: the source tests futures; this card runs only on the Norgate continuous contracts {} ({LABEL})", homes.iter().map(|h| ticker(h)).collect::<Vec<_>>().join(", ")) }
        else { format!("Not published on {t}: the source trades {} futures; this card runs only on {} ({LABEL})", m.name, ticker(m)) })
}

#[cfg(test)]
mod tests {
    #[test] fn market_symbols_unique() { let mut v: Vec<&str> = super::MARKETS.iter().map(|m| m.sym).collect(); let n = v.len(); v.sort(); v.dedup(); assert_eq!(v.len(), n, "duplicate MARKETS symbol (lane merge)"); } // lane PF (review PF2 M4)
    use super::*;
    /// Lane PE (R4 #319): $NYA loads as its own context from Norgate's compact dates (volume 0 = none); only Holt's cards run on it.
    #[test]
    fn nya_context_loads_compact_dates_and_admits_only_holt() {
        let dir = std::env::temp_dir().join(format!("pe-nya-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("$NYA.csv"), "\"Date\",\"Symbol\",\"Security Name\",\"Open\",\"High\",\"Low\",\"Close\",\"Volume\"\n\"19800102\",\"$NYA\",\"NYSE Composite Index\",\"61.6\",\"61.6\",\"61.6\",\"61.6\",\"0\"\n\"19800103\",\"$NYA\",\"NYSE Composite Index\",\"61.2\",\"61.2\",\"61.2\",\"61.2\",\"50480000\"\n").unwrap();
        let mut data = BTreeMap::new();
        load_nya(&dir, &mut data);
        let r = &data[NYA_TICKER];
        assert_eq!((r.len(), r[0].date.as_str(), r[0].volume, r[1].volume), (2, "1980-01-02", None, Some(50480000.)));
        let _ = std::fs::remove_dir_all(&dir);
        let cat = crate::books::scholt::catalogue();
        assert!(crate::instrument_policy::check(&cat[0], NYA_TICKER, false).is_none());
        let other = crate::books::scweiss::catalogue();
        assert!(crate::instrument_policy::check(&other[0], NYA_TICKER, false).is_some_and(|i| i.note.contains("Holt")));
    }
    #[test]
    fn f3_excursion_and_gross_marks_on_the_unadjusted_contract() {
        // back-adjusted bars around -11, unadjusted close = back-adjusted + 91.5 (one roll offset)
        let mk = |d: &str, o: f64, h: f64, l: f64, c: f64| Row { ticker: "US T-Bond (Norgate &ZB)".into(), date: d.into(), open: o, high: h, low: l, close: c, volume: None };
        let r = vec![mk("1990-01-02", -11.7, -11.0, -12.7, -11.2), mk("1990-01-03", -11.2, -10.0, -11.5, -10.5)];
        let mut x = Ctx::new(&r, &[], &[]);
        x.extra.insert("RAW.Close".into(), vec![80.3, 81.0]);
        // long at the open of bar 0, exit at bar 1's close
        let t = Trade { dir: 1., legs: vec![Leg { i: 0, px: -11.7, w: 1. }], exit_i: 1, exit_px: -10.5, reason: "x".into(), entry_kind: "open", signal_i: 1, open: false, exits: vec![] };
        let (out, close) = raw_basis(&x, vec![t]).unwrap();
        assert!((trade_offset(&x, &out[0]) - 91.5).abs() < 1e-9 && (out[0].legs[0].px - 79.8).abs() < 1e-9);
        let (mae, mfe, _) = crate::accounting::excursion(&x, &out[0]).unwrap();
        assert!((mae - (78.8 / 79.8 - 1.)).abs() < 1e-12 && (mfe - (81.5 / 79.8 - 1.)).abs() < 1e-12, "{mae} {mfe}");
        assert_eq!(marks(&x, &out).unwrap(), close);
        let plain = Ctx::new(&r, &[], &[]); // no RAW series: offset 0, no marks
        assert!(trade_offset(&plain, &out[0]) == 0. && marks(&plain, &out).is_none());
    }
    #[test]
    fn f3_index_series_from_norgate_compact_dates() {
        let dir = std::env::temp_dir().join(format!("f3idx{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("$COMP.csv"), "\"Date\",\"Symbol\",\"Security Name\",\"Open\",\"High\",\"Low\",\"Close\",\"Volume\"\n\"19710205\",\"$COMP\",\"Nasdaq Composite Index\",\"100\",\"100\",\"100\",\"100.5\",\"0\"\n").unwrap();
        let m = load_indices(&dir);
        assert_eq!(m.get(&("*".to_string(), "NASDAQ_COMP".to_string())).and_then(|v| v.get("1971-02-05")), Some(&100.5));
        assert!(!m.contains_key(&("*".to_string(), "NYSE_COMP".to_string()))); // a missing file loads nothing
        let _ = std::fs::remove_dir_all(&dir);
    }
    #[test]
    fn x4_ehrlich17_run_ends_before_a_nonpositive_unadjusted_price() {
        // R4 #316: the &CL series stays whole; only jkbwts_ehrlich17's run is capped (auto_end) / refused (check)
        let r: Vec<Row> = (0..10).map(|i| Row { ticker: "Crude Oil (Norgate &CL)".into(), date: format!("2020-04-{:02}", i + 10), open: 20., high: 20., low: 20., close: 20., volume: None }).collect();
        let mut x = Ctx::new(&r, &[], &[]);
        for k in ["RAW.Open", "RAW.High", "RAW.Low"] { x.extra.insert(k.into(), vec![30.; 10]); }
        let mut c = vec![30.; 10]; c[6] = -37.63; x.extra.insert("RAW.Close".into(), c);
        assert_eq!(first_nonpositive("jkbwts_ehrlich17", &x), Some(6));
        assert_eq!(first_nonpositive("mhs_cl850210_1", &x), None);
        let d = crate::books::jkbwts::catalogue().into_iter().find(|d| d.id == "jkbwts_ehrlich17").unwrap();
        let p = crate::connors::Params::new();
        let (end, note) = crate::readiness::auto_end(&x, &d, 0, 9, &p);
        assert!(end == 5 && note.is_some_and(|n| n.contains("2020-04-16") && n.contains("R4 #316")));
        assert!(crate::readiness::check(&x, &d, 0, 5, &p).map_or(true, |i| !i.note.contains("R4 #316")));
        assert!(crate::readiness::check(&x, &d, 6, 9, &p).is_some_and(|i| i.note.contains("R4 #316")));
    }
    use crate::connors::Leg;
    fn row(d: &str, c: f64) -> Row { Row { ticker: "US T-Bond (Norgate &ZB)".into(), date: d.into(), open: c, high: c, low: c, close: c, volume: None } }
    #[test]
    fn ticker_names_and_multipliers() {
        let zb = of_ticker("US T-Bond (Norgate &ZB)").unwrap();
        assert_eq!(zb.sym, "&ZB");
        assert!(of_ticker("SPX").is_none() && of_ticker("US T-Bond (Norgate &XX)").is_none());
        assert_eq!(usd_per_point(zb, "1990-01-02"), Some(1000.));
        assert_eq!(usd_per_point(zb, "1970-01-02"), None);
        assert_eq!(ticker(zb), "US T-Bond (Norgate &ZB)");
    }
    #[test]
    fn raw_basis_keeps_points_and_prices_in_the_held_contract() {
        // back-adjusted -11.7 .. -9.7 with a roll offset of +91.5: a long from -11.7 to -9.7 = +2 points on a 79.8 contract
        let r = vec![row("1978-03-10", -11.7), row("1978-03-13", -10.7), row("1978-03-14", -9.7)];
        let mut x = Ctx::new(&r, &[], &[]);
        x.extra.insert("RAW.Close".into(), vec![79.8, 80.8, 81.8]);
        let t = Trade { dir: 1., legs: vec![Leg { i: 0, px: -11.7, w: 1. }], exit_i: 2, exit_px: -9.7, reason: "x".into(), entry_kind: "close", signal_i: 2, open: false, exits: vec![] };
        let (out, close) = raw_basis(&x, vec![t]).unwrap();
        assert!((out[0].legs[0].px - 79.8).abs() < 1e-9 && (out[0].exit_px - 81.8).abs() < 1e-9);
        assert!((out[0].ret(0.) - 2. / 79.8).abs() < 1e-12);
        assert!((close[1] - 80.8).abs() < 1e-9);
        assert!((buy_hold(&x, 0, 2).unwrap()[2] - (1. + 2. / 79.8)).abs() < 1e-12);
        assert!(!invalid_bar(&x, 0) || x.series("RAW.Open").is_none());
    }
}
