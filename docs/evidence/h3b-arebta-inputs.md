# Aronson Table 8.2 other-input cards (lane H3b, 19 Nov 2026)

New live ids (6, one per input): arebta_d_djta (input 4, D-8-4-10-60), arebta_d_hlr10 (31, D-8-31-10-30), arebta_d_hlr30 (32, D-8-32-10-15), arebta_d_pt10y (36, D-7-36-10-30), arebta_e_durspd (39, E-11-39-10-60), arebta_tt_pt10y (36, TT-36-5). Defaults are the rule IDs printed in Table 9.1 (p.445-446); inputs defined on p.414-417.

Engine additions: Norgate $DJT close as series DJ_TRANSPORT (futures.rs INDEX_SERIES); the six ids join readiness GAP_FIXED_WINDOW (fixed-window arithmetic that skips days without a print); FOLLOWER_CLOSE required for the D cards.

## Independent reference (authors/held-1119/H3b/h3b_ref.py, pandas, built from the raw files; engine window taken from the API dump)
```
arebta_d_djta SPY engine 111 ref 111 mismatches 0 []
arebta_d_djta QQQ engine 71 ref 71 mismatches 0 []
arebta_d_djta SPX engine 142 ref 142 mismatches 0 []
arebta_d_hlr10 SPY engine 324 ref 324 mismatches 0 []
arebta_d_hlr10 QQQ engine 240 ref 240 mismatches 0 []
arebta_d_hlr10 SPX engine 387 ref 387 mismatches 0 []
arebta_d_hlr30 SPY engine 258 ref 258 mismatches 0 []
arebta_d_hlr30 QQQ engine 195 ref 195 mismatches 0 []
arebta_d_hlr30 SPX engine 300 ref 300 mismatches 0 []
arebta_d_pt10y SPY engine 935 ref 935 mismatches 0 []
arebta_d_pt10y QQQ engine 750 ref 750 mismatches 0 []
arebta_d_pt10y SPX engine 1056 ref 1056 mismatches 0 []
arebta_e_durspd SPY engine 174 ref 174 mismatches 0 []
arebta_e_durspd QQQ engine 137 ref 137 mismatches 0 []
arebta_e_durspd SPX engine 184 ref 184 mismatches 0 []
arebta_tt_pt10y SPY engine 958 ref 958 mismatches 0 []
arebta_tt_pt10y QQQ engine 793 ref 793 mismatches 0 []
arebta_tt_pt10y SPX engine 1051 ref 1051 mismatches 0 []
TOTAL engine trades 8066 mismatches 0
```

Indian instruments (NIFTY 50, basket:nifty50:RELIANCE): the engine refuses with the missing-series message (the inputs are US series). basket:us_dow30:AAPL runs and returns trades for all six cards (dumps in H3b/w/out_us); it is NOT compared with the reference, which reads the vault's us_data.csv where the basket stocks are not held (the comparison covers SPY, QQQ, SPX).

## Before/after (Manan build b8e5d73a vs this branch), 8 existing ids x SPY, basket:us_dow30:AAPL, NIFTY 50, basket:nifty50:RELIANCE
ids: arebta_tt, arebta_e_br, arebta_d_br, stad1_sr, nftm_accutrack, thnhl_ema55, dmutil_ud, kdta_nh_nl. 32 of 32 dumps byte-identical (cmp), 17 of them with trade data: 0 changed, 0 lost.
