//! TradingView parity harness: compares lab indicators to "Export chart data" CSVs kept outside Git.
//! See docs/indicator-parity-fixtures.md.
use super::bars::Bars;
use super::catalogue::{get, run, Params};
use std::path::Path;

/// Checks every `*.csv` in `dir`; returns the number of fixtures checked or the first mismatch.
fn check_dir(dir: &Path) -> Result<usize, String> {
    let mut checked = 0;
    for entry in std::fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))? {
        let path = entry.map_err(|e| e.to_string())?.path();
        if path.extension().and_then(|e| e.to_str()) != Some("csv") { continue; }
        let at = |m: &str| format!("{}: {m}", path.display());
        let stem = path.file_stem().and_then(|s| s.to_str()).ok_or_else(|| at("bad file name"))?.to_string();
        let (id, args) = stem.split_once("__").unwrap_or((&stem, ""));
        let def = get(id).ok_or_else(|| at(&format!("unknown indicator {id}")))?;
        let mut prm = Params::new();
        for kv in args.split(',').filter(|s| !s.is_empty()) {
            let (k, v) = kv.split_once('=').ok_or_else(|| at(&format!("bad param {kv}")))?;
            prm.insert(k.into(), v.parse().map_err(|_| at(&format!("bad value in {kv}")))?);
        }
        let text = std::fs::read_to_string(&path).map_err(|e| at(&e.to_string()))?;
        let mut lines = text.lines();
        let header: Vec<&str> = lines.next().ok_or_else(|| at("empty file"))?.split(',').collect();
        let vol_col = header.iter().position(|h| h.eq_ignore_ascii_case("volume"));
        let first_out = 5 + vol_col.is_some() as usize;
        let rows: Vec<Vec<f64>> = lines.map(|l| l.split(',').skip(1).map(|x| x.trim().parse().unwrap_or(f64::NAN)).collect()).collect();
        if rows.iter().any(|r| r.len() < first_out - 1 + def.outputs.len()) { return Err(at("row has too few columns")); }
        if rows.is_empty() { return Err(at("no data rows")); }
        let mut b = Bars::default();
        for (i, r) in rows.iter().enumerate() { b.date.push(format!("{i}")); b.open.push(r[0]); b.high.push(r[1]); b.low.push(r[2]); b.close.push(r[3]); b.end.push(i); }
        if let Some(vc) = vol_col { b.volume = Some(rows.iter().map(|r| r[vc - 1]).collect()); }
        let got = run(&def, &b, &prm).map_err(|e| at(&format!("{e:?}")))?;
        for (k, series) in got.iter().enumerate() {
            if rows.iter().all(|r| r[first_out - 1 + k].is_nan()) { return Err(at(&format!("output {} has no non-NaN expected value", def.outputs[k]))); }
            for (i, r) in rows.iter().enumerate() {
                let want = r[first_out - 1 + k];
                // 1e-6 relative: exported values are rounded to display precision
                let ok = (want.is_nan() && series[i].is_nan()) || (want - series[i]).abs() <= 1e-6 * want.abs().max(1.);
                if !ok { return Err(at(&format!("output {} bar {i}: TradingView {want}, lab {}", def.outputs[k], series[i]))); }
            }
        }
        checked += 1;
    }
    Ok(checked)
}

#[test]
fn tradingview_exports_match_when_supplied() {
    let Ok(dir) = std::env::var("LAB_TV_FIXTURES") else { eprintln!("LAB_TV_FIXTURES not set; parity check skipped"); return; };
    let n = check_dir(Path::new(&dir)).unwrap_or_else(|e| panic!("{e}"));
    eprintln!("checked {n} TradingView fixtures");
}

#[test]
fn parity_harness_detects_mismatch_and_match() {
    let base = std::env::temp_dir().join(format!("lab_parity_{}", std::process::id()));
    let (good, bad) = (base.join("good"), base.join("bad"));
    std::fs::create_dir_all(&good).unwrap();
    std::fs::create_dir_all(&bad).unwrap();
    // SMA(2) of closes 1..4: NaN, 1.5, 2.5, 3.5 (blank = no value)
    let csv = |v: &str| format!("time,open,high,low,close,SMA\n2024-01-01,1,1,1,1,\n2024-01-02,2,2,2,2,1.5\n2024-01-03,3,3,3,3,2.5\n2024-01-04,4,4,4,4,{v}\n");
    std::fs::write(good.join("sma__length=2.csv"), csv("3.5")).unwrap();
    std::fs::write(bad.join("sma__length=2.csv"), csv("3.6")).unwrap();
    let ok = check_dir(&good);
    let err = check_dir(&bad);
    std::fs::remove_dir_all(&base).ok();
    assert_eq!(ok, Ok(1));
    assert!(err.is_err_and(|e| e.contains("bar 3")));
}

#[test]
fn parity_harness_rejects_empty_and_all_blank_fixtures() {
    let base = std::env::temp_dir().join(format!("lab_parity_vacuous_{}", std::process::id()));
    let (empty, blank) = (base.join("empty"), base.join("blank"));
    std::fs::create_dir_all(&empty).unwrap();
    std::fs::create_dir_all(&blank).unwrap();
    std::fs::write(empty.join("sma__length=2.csv"), "time,open,high,low,close,SMA\n").unwrap();
    std::fs::write(blank.join("sma__length=2.csv"), "time,open,high,low,close,SMA\n2024-01-01,1,1,1,1,\n2024-01-02,2,2,2,2,\n").unwrap();
    let (e, b) = (check_dir(&empty), check_dir(&blank));
    std::fs::remove_dir_all(&base).ok();
    assert!(e.is_err_and(|m| m.contains("sma__length=2.csv") && m.contains("no data rows")));
    assert!(b.is_err_and(|m| m.contains("sma__length=2.csv") && m.contains("sma")));
}
