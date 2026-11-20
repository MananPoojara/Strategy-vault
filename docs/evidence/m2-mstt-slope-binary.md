# mstt_bns_w_slope_plus_binary (+ _2nd_exit) (lane M2, 20 Nov 2026)

Source: Peter's Window on WallStreet systems p.3 (Trading Tech. Systems & Indicators PDF): entry = six slope(...,3)>0 terms summed = 6; exit = cross/close condition AND an eight-term binary score < 4. Printed formulas are run unchanged by the msws engine; long only (buy = formula 1, sell = formula 2), next-open fills. UNVERIFIED (no printed results).

## Built-in definitions (cited; texts saved in authors/lanes-1120/M2w: ms8.txt = MetaStock Pro 8.0 manual, taaz_82/87/65/94.txt)
- SAR(.02,.2): `sar( STEP, MAXIMUM )`, example sar(0.02,0.20) (manual, function list); Wilder, New Concepts in Technical Trading Systems (TAAZ p=87 and manual p.499 defer the calculation to Wilder). Engine start convention = labelled unstated detail.
- OBV(): "adding the day's volume ... when the close is up, subtracting ... down; unchanged if equal" (manual p.492; TAAZ p=82; formula p.310). A constant seed does not change OBV > Mov(OBV,40,S).
- slope(): the manual names `linregslope( DATA ARRAY, PERIODS )`; the source prints "slope": read as the n-bar least-squares slope (labelled unstated detail; same reading as the ~99 live mstt formulas using it).
- OscP(13,55,E,%): TAAZ p=94 text now saved verbatim: "When the Price Oscillator displays the difference ... in percentages, it divides the difference between the averages by the shorter-term moving average" - this CONFIRMS H3d's open point directly from Achelis/MetaStock (previously only via the eSignal quotation).
- cci(13) = mean-deviation CCI (engine, used by many live cards).

## Independent reference (m2_ref.py, pandas/numpy: own EMA, np.polyfit slope, loop SAR, cumsum OBV)
Engine vs reference, long-only trade lists (entry date/price, exit date/price): see ref_us.txt in M2w:
mstt_bns_w_slope_plus_binary SPY/QQQ/DIA/IWM/MDY/SPX 60/50/56/52/68/19 trades, 0 mismatches; _2nd_exit 254/223/239/231/263/89, 0 mismatches. TOTAL 1604 trades, 0 mismatches.
Limit: the SAR start bars are shared by engine and reference (both declared), so the reference checks the implementation of Wilder's recursion, not the start convention.
