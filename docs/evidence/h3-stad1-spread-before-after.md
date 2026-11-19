# stad1_spread before/after (lane H3, 19 Nov 2026)

Base build = HEAD 21d8c89d; branch build = bd9846b5 (+ gate text). Private US (19793) and India (19794) engines, one at a time, killed by recorded PID. Real basket ids from /api/tickers: basket:us_dow30:AAPL (to 2020-09-04), basket:nifty50:RELIANCE (to 2026-10-16). 7 existing ids x 4 instruments; each pair compared with cmp.

| context | id | base sha256 (12) | branch sha256 (12) | cmp |
|---|---|---|---|---|
| us | arebta_e_br__basket:us_dow30:AAPL | 90db246298d4 | 90db246298d4 | same |
| us | arebta_e_br__SPY | 61116d5b595b | 61116d5b595b | same |
| us | nftm_accutrack__basket:us_dow30:AAPL | 8a5cac86c328 | 8a5cac86c328 | same |
| us | nftm_accutrack__SPY | 2213883e1707 | 2213883e1707 | same |
| us | stad1_maco__basket:us_dow30:AAPL | 2580905d937b | 2580905d937b | same |
| us | stad1_maco__SPY | c465945b93a3 | c465945b93a3 | same |
| us | stad1_spread__basket:us_dow30:AAPL | 423a828e9cff | 2b36af9b6090 | differs (new card) |
| us | stad1_spread__SPY | 423a828e9cff | e73fcb145a27 | differs (new card) |
| us | stad1_sr__basket:us_dow30:AAPL | 7e034f783959 | 7e034f783959 | same |
| us | stad1_sr__SPY | 23ca0a2662a0 | 23ca0a2662a0 | same |
| us | stad1_volex__basket:us_dow30:AAPL | 1ee1951b13cf | 1ee1951b13cf | same |
| us | stad1_volex__SPY | bb8030a99d07 | bb8030a99d07 | same |
| us | stad4_spread_up__basket:us_dow30:AAPL | afc700126c5f | afc700126c5f | same |
| us | stad4_spread_up__SPY | 3de44b9c1c14 | 3de44b9c1c14 | same |
| us | thnhl_ema55__basket:us_dow30:AAPL | 8415a69277d7 | 8415a69277d7 | same |
| us | thnhl_ema55__SPY | 2ce7dbb2d1de | 2ce7dbb2d1de | same |
| in | arebta_e_br_nse__basket:nifty50:RELIANCE | 5701d612478e | 5701d612478e | same |
| in | arebta_e_br_nse__NIFTY_50 | 738334b879cc | 738334b879cc | same |
| in | nftm_accutrack__basket:nifty50:RELIANCE | 91812260f718 | 91812260f718 | same |
| in | nftm_accutrack__NIFTY_50 | 038799a168cb | 038799a168cb | same |
| in | stad1_maco__basket:nifty50:RELIANCE | 6988862d2f5b | 6988862d2f5b | same |
| in | stad1_maco__NIFTY_50 | e86421fac284 | e86421fac284 | same |
| in | stad1_spread__basket:nifty50:RELIANCE | 423a828e9cff | a5ead0820245 | differs (new card) |
| in | stad1_spread__NIFTY_50 | 423a828e9cff | 9b05d040b21f | differs (new card) |
| in | stad1_sr__basket:nifty50:RELIANCE | 2bd590f52ad0 | 2bd590f52ad0 | same |
| in | stad1_sr__NIFTY_50 | 24af87d6aea5 | 24af87d6aea5 | same |
| in | stad1_volex__basket:nifty50:RELIANCE | 208f8d6caf22 | 208f8d6caf22 | same |
| in | stad1_volex__NIFTY_50 | 49e800117b76 | 49e800117b76 | same |
| in | stad4_spread_up__basket:nifty50:RELIANCE | fef5d74141bc | fef5d74141bc | same |
| in | stad4_spread_up__NIFTY_50 | af8bcd5e41a1 | af8bcd5e41a1 | same |
| in | thnhl_ema55_nse__basket:nifty50:RELIANCE | e70c1af6ac6d | e70c1af6ac6d | same |
| in | thnhl_ema55_nse__NIFTY_50 | e869f52fd1b5 | e869f52fd1b5 | same |

28 pairs identical; the 4 that differ are stad1_spread itself (absent on the base build, refused or run on the branch). Real runs in the set include SPY, NIFTY 50 and the AAPL / RELIANCE basket stocks with trades (stad1_sr AAPL 555 KB).
