# Katz kets_ma_* and kets_*_limit/stop cards (lane H3, 19 Nov 2026)

New live ids (12): kets_ma_xo_open/limit/stop (Ch.6 Tests 1-3), kets_ma_slope_open/limit/stop (Tests 13-15), kets_macd_sig_/kets_rsi_div_/kets_macd_div_ limit and stop (Tests 11,12,17,18,20,21). Rules read from the book text (pp.116-121, 90, 119, 148-150).

## Independent reference (kets_h3_ref.py = the D2 Python reference from the book's C++ / CD TRDSIM2, extended with these signals)
196 comparisons (12 ids x 2 parameter sets x 2 windows x SPY, QQQ, SPX, NIFTY 50): US 11,892 trades, India 8,034 trades, 0 mismatches. Full output below.

```
QQQ kets_ma_slope_open {'len': 10} default 1999-03-10 2026-11-17 engine 929 ref 929 mismatches 0
QQQ kets_ma_slope_open {'len': 10} {'from': '2008-01-02', 'to': '2012-12-31'} 2008-01-02 2012-12-31 engine 177 ref 177 mismatches 0
QQQ kets_ma_slope_limit {} default 1999-03-10 2026-11-17 engine 367 ref 367 mismatches 0
QQQ kets_ma_slope_limit {} {'from': '2008-01-02', 'to': '2012-12-31'} 2008-01-02 2012-12-31 engine 65 ref 65 mismatches 0
QQQ kets_ma_slope_limit {'len': 10} default 1999-03-10 2026-11-17 engine 584 ref 584 mismatches 0
QQQ kets_ma_slope_limit {'len': 10} {'from': '2008-01-02', 'to': '2012-12-31'} 2008-01-02 2012-12-31 engine 108 ref 108 mismatches 0
QQQ kets_ma_slope_stop {} default 1999-03-10 2026-11-17 engine 240 ref 240 mismatches 0
QQQ kets_ma_slope_stop {} {'from': '2008-01-02', 'to': '2012-12-31'} 2008-01-02 2012-12-31 engine 36 ref 36 mismatches 0
QQQ kets_ma_slope_stop {'len': 10} default 1999-03-10 2026-11-17 engine 343 ref 343 mismatches 0
QQQ kets_ma_slope_stop {'len': 10} {'from': '2008-01-02', 'to': '2012-12-31'} 2008-01-02 2012-12-31 engine 55 ref 55 mismatches 0
TOTAL ref trades 7846 mismatches 0
SPX kets_macd_sig_limit {} default 1983-01-03 2026-11-17 engine 521 ref 521 mismatches 0
SPX kets_macd_sig_limit {} {'from': '2008-01-02', 'to': '2012-12-31'} 2008-01-02 2012-12-31 engine 53 ref 53 mismatches 0
SPX kets_macd_sig_limit {'fast': 8} default 1983-01-03 2026-11-17 engine 589 ref 589 mismatches 0
SPX kets_macd_sig_limit {'fast': 8} {'from': '2008-01-02', 'to': '2012-12-31'} 2008-01-02 2012-12-31 engine 63 ref 63 mismatches 0
SPX kets_macd_sig_stop {} default 1983-01-03 2026-11-17 engine 382 ref 382 mismatches 0
SPX kets_macd_sig_stop {} {'from': '2008-01-02', 'to': '2012-12-31'} 2008-01-02 2012-12-31 engine 37 ref 37 mismatches 0
SPX kets_macd_sig_stop {'fast': 8} default 1983-01-03 2026-11-17 engine 449 ref 449 mismatches 0
SPX kets_macd_sig_stop {'fast': 8} {'from': '2008-01-02', 'to': '2012-12-31'} 2008-01-02 2012-12-31 engine 43 ref 43 mismatches 0
SPX kets_rsi_div_limit {} default 1983-01-03 2026-11-17 engine 242 ref 242 mismatches 0
SPX kets_rsi_div_limit {} {'from': '2008-01-02', 'to': '2012-12-31'} 2008-01-02 2012-12-31 engine 26 ref 26 mismatches 0
SPX kets_rsi_div_limit {'lb': 15} default 1983-01-03 2026-11-17 engine 203 ref 203 mismatches 0
SPX kets_rsi_div_limit {'lb': 15} {'from': '2008-01-02', 'to': '2012-12-31'} 2008-01-02 2012-12-31 engine 22 ref 22 mismatches 0
SPX kets_rsi_div_stop {} default 1983-01-03 2026-11-17 engine 191 ref 191 mismatches 0
SPX kets_rsi_div_stop {} {'from': '2008-01-02', 'to': '2012-12-31'} 2008-01-02 2012-12-31 engine 22 ref 22 mismatches 0
SPX kets_rsi_div_stop {'lb': 15} default 1983-01-03 2026-11-17 engine 151 ref 151 mismatches 0
SPX kets_rsi_div_stop {'lb': 15} {'from': '2008-01-02', 'to': '2012-12-31'} 2008-01-02 2012-12-31 engine 19 ref 19 mismatches 0
SPX kets_macd_div_limit {} default 1983-01-03 2026-11-17 engine 109 ref 109 mismatches 0
SPX kets_macd_div_limit {} {'from': '2008-01-02', 'to': '2012-12-31'} 2008-01-02 2012-12-31 engine 10 ref 10 mismatches 0
SPX kets_macd_div_limit {'lb': 15} default 1983-01-03 2026-11-17 engine 87 ref 87 mismatches 0
SPX kets_macd_div_limit {'lb': 15} {'from': '2008-01-02', 'to': '2012-12-31'} 2008-01-02 2012-12-31 engine 10 ref 10 mismatches 0
SPX kets_macd_div_stop {} default 1983-01-03 2026-11-17 engine 82 ref 82 mismatches 0
SPX kets_macd_div_stop {} {'from': '2008-01-02', 'to': '2012-12-31'} 2008-01-02 2012-12-31 engine 8 ref 8 mismatches 0
SPX kets_macd_div_stop {'lb': 15} default 1983-01-03 2026-11-17 engine 54 ref 54 mismatches 0
SPX kets_macd_div_stop {'lb': 15} {'from': '2008-01-02', 'to': '2012-12-31'} 2008-01-02 2012-12-31 engine 8 ref 8 mismatches 0
SPX kets_ma_xo_open {} default 1983-01-03 2026-11-17 engine 671 ref 671 mismatches 0
SPX kets_ma_xo_open {} {'from': '2008-01-02', 'to': '2012-12-31'} 2008-01-02 2012-12-31 engine 83 ref 83 mismatches 0
SPX kets_ma_xo_open {'fast': 3, 'slow': 30} default 1983-01-03 2026-11-17 engine 619 ref 619 mismatches 0
SPX kets_ma_xo_open {'fast': 3, 'slow': 30} {'from': '2008-01-02', 'to': '2012-12-31'} 2008-01-02 2012-12-31 engine 66 ref 66 mismatches 0
SPX kets_ma_xo_limit {} default 1983-01-03 2026-11-17 engine 445 ref 445 mismatches 0
SPX kets_ma_xo_limit {} {'from': '2008-01-02', 'to': '2012-12-31'} 2008-01-02 2012-12-31 engine 54 ref 54 mismatches 0
SPX kets_ma_xo_limit {'fast': 3, 'slow': 30} default 1983-01-03 2026-11-17 engine 393 ref 393 mismatches 0
SPX kets_ma_xo_limit {'fast': 3, 'slow': 30} {'from': '2008-01-02', 'to': '2012-12-31'} 2008-01-02 2012-12-31 engine 44 ref 44 mismatches 0
SPX kets_ma_xo_stop {} default 1983-01-03 2026-11-17 engine 270 ref 270 mismatches 0
SPX kets_ma_xo_stop {} {'from': '2008-01-02', 'to': '2012-12-31'} 2008-01-02 2012-12-31 engine 28 ref 28 mismatches 0
SPX kets_ma_xo_stop {'fast': 3, 'slow': 30} default 1983-01-03 2026-11-17 engine 257 ref 257 mismatches 0
SPX kets_ma_xo_stop {'fast': 3, 'slow': 30} {'from': '2008-01-02', 'to': '2012-12-31'} 2008-01-02 2012-12-31 engine 19 ref 19 mismatches 0
SPX kets_ma_slope_open {} default 1983-01-03 2026-11-17 engine 1047 ref 1047 mismatches 0
SPX kets_ma_slope_open {} {'from': '2008-01-02', 'to': '2012-12-31'} 2008-01-02 2012-12-31 engine 126 ref 126 mismatches 0
SPX kets_ma_slope_open {'len': 10} default 1983-01-03 2026-11-17 engine 1529 ref 1529 mismatches 0
SPX kets_ma_slope_open {'len': 10} {'from': '2008-01-02', 'to': '2012-12-31'} 2008-01-02 2012-12-31 engine 183 ref 183 mismatches 0
SPX kets_ma_slope_limit {} default 1983-01-03 2026-11-17 engine 560 ref 560 mismatches 0
SPX kets_ma_slope_limit {} {'from': '2008-01-02', 'to': '2012-12-31'} 2008-01-02 2012-12-31 engine 72 ref 72 mismatches 0
SPX kets_ma_slope_limit {'len': 10} default 1983-01-03 2026-11-17 engine 826 ref 826 mismatches 0
SPX kets_ma_slope_limit {'len': 10} {'from': '2008-01-02', 'to': '2012-12-31'} 2008-01-02 2012-12-31 engine 95 ref 95 mismatches 0
SPX kets_ma_slope_stop {} default 1983-01-03 2026-11-17 engine 426 ref 426 mismatches 0
SPX kets_ma_slope_stop {} {'from': '2008-01-02', 'to': '2012-12-31'} 2008-01-02 2012-12-31 engine 34 ref 34 mismatches 0
SPX kets_ma_slope_stop {'len': 10} default 1983-01-03 2026-11-17 engine 601 ref 601 mismatches 0
SPX kets_ma_slope_stop {'len': 10} {'from': '2008-01-02', 'to': '2012-12-31'} 2008-01-02 2012-12-31 engine 63 ref 63 mismatches 0
TOTAL ref trades 11892 mismatches 0
NIFTY 50 kets_ma_xo_limit {} {'from': '2008-01-02', 'to': '2012-12-31'} 2008-01-02 2012-12-31 engine 59 ref 59 mismatches 0
NIFTY 50 kets_ma_xo_limit {'fast': 3, 'slow': 30} default 1995-11-03 2026-11-18 engine 253 ref 253 mismatches 0
NIFTY 50 kets_ma_xo_limit {'fast': 3, 'slow': 30} {'from': '2008-01-02', 'to': '2012-12-31'} 2008-01-02 2012-12-31 engine 37 ref 37 mismatches 0
NIFTY 50 kets_ma_xo_stop {} default 1995-11-03 2026-11-18 engine 203 ref 203 mismatches 0
NIFTY 50 kets_ma_xo_stop {} {'from': '2008-01-02', 'to': '2012-12-31'} 2008-01-02 2012-12-31 engine 34 ref 34 mismatches 0
NIFTY 50 kets_ma_xo_stop {'fast': 3, 'slow': 30} default 1995-11-03 2026-11-18 engine 186 ref 186 mismatches 0
NIFTY 50 kets_ma_xo_stop {'fast': 3, 'slow': 30} {'from': '2008-01-02', 'to': '2012-12-31'} 2008-01-02 2012-12-31 engine 31 ref 31 mismatches 0
NIFTY 50 kets_ma_slope_open {} default 1995-11-03 2026-11-18 engine 650 ref 650 mismatches 0
NIFTY 50 kets_ma_slope_open {} {'from': '2008-01-02', 'to': '2012-12-31'} 2008-01-02 2012-12-31 engine 126 ref 126 mismatches 0
NIFTY 50 kets_ma_slope_open {'len': 10} default 1995-11-03 2026-11-18 engine 896 ref 896 mismatches 0
NIFTY 50 kets_ma_slope_open {'len': 10} {'from': '2008-01-02', 'to': '2012-12-31'} 2008-01-02 2012-12-31 engine 162 ref 162 mismatches 0
NIFTY 50 kets_ma_slope_limit {} default 1995-11-03 2026-11-18 engine 339 ref 339 mismatches 0
NIFTY 50 kets_ma_slope_limit {} {'from': '2008-01-02', 'to': '2012-12-31'} 2008-01-02 2012-12-31 engine 73 ref 73 mismatches 0
NIFTY 50 kets_ma_slope_limit {'len': 10} default 1995-11-03 2026-11-18 engine 481 ref 481 mismatches 0
NIFTY 50 kets_ma_slope_limit {'len': 10} {'from': '2008-01-02', 'to': '2012-12-31'} 2008-01-02 2012-12-31 engine 90 ref 90 mismatches 0
NIFTY 50 kets_ma_slope_stop {} default 1995-11-03 2026-11-18 engine 293 ref 293 mismatches 0
NIFTY 50 kets_ma_slope_stop {} {'from': '2008-01-02', 'to': '2012-12-31'} 2008-01-02 2012-12-31 engine 56 ref 56 mismatches 0
NIFTY 50 kets_ma_slope_stop {'len': 10} default 1995-11-03 2026-11-18 engine 402 ref 402 mismatches 0
NIFTY 50 kets_ma_slope_stop {'len': 10} {'from': '2008-01-02', 'to': '2012-12-31'} 2008-01-02 2012-12-31 engine 62 ref 62 mismatches 0
TOTAL ref trades 8034 mismatches 0
```

## Before/after (existing results unchanged)
Branch before (bd9846b5 build) vs after (this change), 6 existing kets ids (macd_sig_open, rsi_div_open, macd_div_open, stoch_sig_open, cb_open, vb_limit) on SPY, basket:us_dow30:AAPL, NIFTY 50, basket:nifty50:RELIANCE: 24 of 24 dumps byte-identical (cmp), each with trade data (>5 KB).
