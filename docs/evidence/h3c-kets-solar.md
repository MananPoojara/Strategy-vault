# Katz Ch.9 solar (sunspot) Tests 1-3 (lane H3c, 19 Nov 2026)

New live ids: kets_solar_open (Test 1, lb1/lb2/disp 35/15/2), kets_solar_limit (Test 2, 40/15/0), kets_solar_stop (Test 3, 35/15/2); the printed best in-sample values (p.199). Rule: printed prose pp.197-199 + CD TRMODS/CH13 LUNAR/X13MOD02.C (SolarSignals, Model). Engine change: kets.rs (solar, solar_signals), main.rs sets kets::EXTRA_DIR so the card can read the calendar series extra_data/us/SUNSPOTS.csv (weekends included); the same file is also loaded as the aligned series SUNSPOTS, so the card refuses with the missing-series message when the file is absent.

## Data (not installed; staged for the lead)
authors/data-1119/sunspots/: MANIFEST.md, SOURCES-ENTRY.md, raw/ (SILSO daily V2.0 sha256 28525cc0..., monthly, NOAA SWPC monthly), staged/SUNSPOTS.csv (sha256 4a76b261..., 73,028 rows 1818-01-08..2026-10-31), build_sunspots.py. Licence CC BY-NC, credit WDC-SILSO. Verification: monthly means of the staged daily series vs SILSO monthly and NOAA SWPC monthly ssn: 2,505 months, max abs diff 0.1 (rounding); 1985-1999 complete (5,478 days, none missing). The mirror dir internet/ and SOURCES.csv are read-only to this lane, so the SOURCES entry is staged as SOURCES-ENTRY.md.

## Independent reference (solar_ref.py, pandas / numpy, own structure; trading by the kets_ref.py TRDSIM2 order rules; the engine's window and its NYSE-closure phantom-bar data rule are inputs)
```
SPY kets_solar_open default 1993-01-29 2026-10-30 engine 302 ref 302 mismatches 0
SPY kets_solar_open {'lb1': 25, 'lb2': 10, 'disp': 1} 1993-01-29 2026-10-30 engine 390 ref 390 mismatches 0
SPY kets_solar_limit default 1993-01-29 2026-10-30 engine 202 ref 202 mismatches 0
SPY kets_solar_limit {'lb1': 25, 'lb2': 10, 'disp': 1} 1993-01-29 2026-10-30 engine 295 ref 295 mismatches 0
SPY kets_solar_stop default 1993-01-29 2026-10-30 engine 115 ref 115 mismatches 0
SPY kets_solar_stop {'lb1': 25, 'lb2': 10, 'disp': 1} 1993-01-29 2026-10-30 engine 173 ref 173 mismatches 0
QQQ kets_solar_open default 1999-03-10 2026-10-30 engine 236 ref 236 mismatches 0
QQQ kets_solar_open {'lb1': 25, 'lb2': 10, 'disp': 1} 1999-03-10 2026-10-30 engine 334 ref 334 mismatches 0
QQQ kets_solar_limit default 1999-03-10 2026-10-30 engine 178 ref 178 mismatches 0
QQQ kets_solar_limit {'lb1': 25, 'lb2': 10, 'disp': 1} 1999-03-10 2026-10-30 engine 266 ref 266 mismatches 0
QQQ kets_solar_stop default 1999-03-10 2026-10-30 engine 100 ref 100 mismatches 0
QQQ kets_solar_stop {'lb1': 25, 'lb2': 10, 'disp': 1} 1999-03-10 2026-10-30 engine 137 ref 137 mismatches 0
SPX kets_solar_open default 1983-01-03 2026-10-30 engine 392 ref 392 mismatches 0
SPX kets_solar_open {'lb1': 25, 'lb2': 10, 'disp': 1} 1983-01-03 2026-10-30 engine 531 ref 531 mismatches 0
SPX kets_solar_limit default 1983-01-03 2026-10-30 engine 285 ref 285 mismatches 0
SPX kets_solar_limit {'lb1': 25, 'lb2': 10, 'disp': 1} 1983-01-03 2026-10-30 engine 403 ref 403 mismatches 0
SPX kets_solar_stop default 1983-01-03 2026-10-30 engine 147 ref 147 mismatches 0
SPX kets_solar_stop {'lb1': 25, 'lb2': 10, 'disp': 1} 1983-01-03 2026-10-30 engine 215 ref 215 mismatches 0
TOTAL ref trades 4701 mismatches 0
```

## Before/after (Manan c63f352f vs this branch, live data dirs)
9 existing ids x SPY, basket:us_dow30:AAPL, NIFTY 50, basket:nifty50:RELIANCE: 36 of 36 byte-identical (0 changed, 0 lost); with the SUNSPOTS file present (private extra dir) the existing ids on the new binary are identical again (20 of 20). The new solar cards on the live dirs refuse with 'needs_data: Required market data is missing: SUNSPOTS' (US and India); the other dumps that differ are the new ids.
