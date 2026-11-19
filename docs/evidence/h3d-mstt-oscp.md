# MetaStock OscP cards (lane H3d, 20 Nov 2026)

New engine function: msws `oscp` = MetaStock Price Oscillator OscP(short, long, method, %): the difference of two moving averages of the close; in percent divided by the SHORTER average. Source of the definition: Achelis, Technical Analysis from A to Z (Price Oscillator), hosted by MetaStock https://www.metastock.com/customer/resources/taaz/?p=94, quoted in the eSignal Technical Analysis Dictionary 'Price Oscillator' (the page body could not be read directly in this lane; the sentence was confirmed through the eSignal entry; other platforms' PPO divides by the longer average).
Newly Live (unverified): mstt_bns (p.3 BNS) and mstt_bns_by_brook (p.3): the printed formulas are the buy and the sell of one LONG-ONLY system (no short side printed). mstt_bns_w_slope_plus_binary (+_2nd_exit) stay held: they also need SAR(.02,.2), OBV(), slope() and volume, each needing its own reference.

## Independent reference (h3d_ref.py, pandas, own structure; Std = population, EMA seeded with the first value, Stoch(k,s) = 100 x sum(C - LLV(L,k), s) / sum(HHV(H,k) - LLV(L,k), s), as the msws engine documents)
```
mstt_bns SPY 1993-01-29 2026-11-18 engine 322 ref 322 mismatches 0 []
mstt_bns QQQ 1999-03-10 2026-11-18 engine 246 ref 246 mismatches 0 []
mstt_bns DIA 1998-01-20 2026-11-18 engine 277 ref 277 mismatches 0 []
mstt_bns IWM 2000-05-26 2026-11-18 engine 268 ref 268 mismatches 0 []
mstt_bns MDY 1995-05-04 2026-11-18 engine 321 ref 321 mismatches 0 []
mstt_bns SPX 2014-01-02 2026-11-18 engine 121 ref 121 mismatches 0 []
mstt_bns_by_brook SPY 1993-01-29 2026-11-18 engine 123 ref 123 mismatches 0 []
mstt_bns_by_brook QQQ 1999-03-10 2026-11-18 engine 97 ref 97 mismatches 0 []
mstt_bns_by_brook DIA 1998-01-20 2026-11-18 engine 93 ref 93 mismatches 0 []
mstt_bns_by_brook IWM 2000-05-26 2026-11-18 engine 88 ref 88 mismatches 0 []
mstt_bns_by_brook MDY 1995-05-04 2026-11-18 engine 108 ref 108 mismatches 0 []
mstt_bns_by_brook SPX 2014-01-02 2026-11-18 engine 49 ref 49 mismatches 0 []
TOTAL engine trades 2113 mismatches 0
```

## Before/after (HEAD at the start of the lane vs this change; SPY, basket:us_dow30:AAPL, NIFTY 50, basket:nifty50:RELIANCE)
7 existing ids (mstt_slope_mo_plus_sto_filter, mstt_ob_os, mstt_four_bar_fear, msws_double_mo_system, kets_solar_open, arebta_e_br, stad1_sr): 28 of 28 dumps byte-identical (0 changed, 0 lost). The 12 dumps that differ are mstt_bns, mstt_bns_by_brook (now run) and mstt_bns_w_slope_plus_binary (only its refusal text changed).
