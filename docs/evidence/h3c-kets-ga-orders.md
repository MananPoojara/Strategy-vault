# Katz Ch.12 limit / stop entries (lane H3c, 19 Nov 2026)

New live ids (4): kets_ga_long_limit (Test 2), kets_ga_long_stop (Test 3), kets_ga_short_limit (Test 5), kets_ga_short_stop (Test 6).
Source: Encyclopedia p.270 ('The same solution was also tested and evaluated with entry on stop and on limit'; the optimal solution per order was NOT selected separately) and the CD scripts TRMODS/CHAP16/X16SCR02.DAT and X16SCR03.DAT, which load the one solution (y16b01 generation 2306 for longs, y16b04 generation 1671 for shorts) and run it with p15 = 1, 2, 3 (open, limit, stop). Orders: limit = (high + low) / 2 of the signal bar, stop = close + 0.5 x signal x ATR(50) (x16mod01.c lines 229-245). The chromosomes are the printed p.276-278 ones already live as kets_ga_long / kets_ga_short; no new parameter, so 0 new details beyond those cards' labelled Indian application (NSE futures open interest, prices = the NSE index).

## Independent reference (authors/held-1119/H3c/w/h3c_ref.py = the lane I1 reference, from the book pages and the bhavcopy extracts, extended with the order types; engine window = the API dump)
```
kets_ga_long	NIFTY 50	2000-10-10..2026-11-12	engine 20	ref 20	mismatches 0	ref 20 closed tr, win 65.00%, avg +1.510%, comp +33.9%	engine {"trades": 20, "win_rate": 0.65, "avg_
kets_ga_short	NIFTY 50	2000-10-10..2026-11-12	engine 12	ref 12	mismatches 0	ref 12 closed tr, win 41.67%, avg +1.597%, comp +17.5%	engine {"trades": 12, "win_rate": 0.416667, 
kets_ga_long_limit	NIFTY 50	2000-10-10..2026-11-12	engine 18	ref 18	mismatches 0	ref 18 closed tr, win 72.22%, avg +1.811%, comp +36.8%	engine {"trades": 18, "win_rate": 0.722
kets_ga_long_stop	NIFTY 50	2000-10-10..2026-11-12	engine 10	ref 10	mismatches 0	ref 10 closed tr, win 50.00%, avg +0.139%, comp +1.1%	engine {"trades": 10, "win_rate": 0.5, "a
kets_ga_short_limit	NIFTY 50	2000-10-10..2026-11-12	engine 10	ref 10	mismatches 0	ref 10 closed tr, win 30.00%, avg +1.597%, comp +14.2%	engine {"trades": 10, "win_rate": 0.3,
kets_ga_short_stop	NIFTY 50	2000-10-10..2026-11-12	engine 6	ref 6	mismatches 0	ref 6 closed tr, win 66.67%, avg +3.508%, comp +21.5%	engine {"trades": 6, "win_rate": 0.666667,
kets_ga_long	NIFTYBANK	2005-06-13..2026-11-12	engine 20	ref 20	mismatches 0	ref 20 closed tr, win 50.00%, avg +0.448%, comp +8.3%	engine {"trades": 20, "win_rate": 0.5, "avg_t
kets_ga_short	NIFTYBANK	2005-06-13..2026-11-12	engine 38	ref 38	mismatches 0	ref 38 closed tr, win 31.58%, avg -0.969%, comp -33.4%	engine {"trades": 38, "win_rate": 0.315789,
kets_ga_long_limit	NIFTYBANK	2005-06-13..2026-11-12	engine 17	ref 17	mismatches 0	ref 17 closed tr, win 52.94%, avg +0.487%, comp +7.8%	engine {"trades": 17, "win_rate": 0.529
kets_ga_long_stop	NIFTYBANK	2005-06-13..2026-11-12	engine 10	ref 10	mismatches 0	ref 10 closed tr, win 50.00%, avg +0.820%, comp +8.1%	engine {"trades": 10, "win_rate": 0.5, "
kets_ga_short_limit	NIFTYBANK	2005-06-13..2026-11-12	engine 36	ref 36	mismatches 0	ref 36 closed tr, win 30.56%, avg -1.542%, comp -44.9%	engine {"trades": 36, "win_rate": 0.3
kets_ga_short_stop	NIFTYBANK	2005-06-13..2026-11-12	engine 29	ref 29	mismatches 0	ref 29 closed tr, win 31.03%, avg -1.113%, comp -29.8%	engine {"trades": 29, "win_rate": 0.31
TOTAL ref trades 226 mismatches 0
```

## Before/after (Manan c63f352f vs this branch), 10 existing ids (kets_ga_long, kets_ga_short, kets_macd_sig_open, kets_cb_open, kets_seas_xo_open, kets_lunar_xo_open, kets_cycle_open, kets_neural, kets_rand_ses_open, arebta_e_br) x SPY, basket:us_dow30:AAPL, NIFTY 50, basket:nifty50:RELIANCE
40 of 40 dumps byte-identical (cmp): 0 changed, 0 lost. The 20 other dumps are the four new ids, absent on the base build.
