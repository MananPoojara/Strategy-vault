# BTST oracle vs team NIFTY tradelist

Run: `~/.venvs/strategy-vault/bin/python -I verify_against_tradelist.py` (flags: `--days`, `--keep-1530`).
Tradelist: `NIFTY_Tradelist_BTST_Entry 1450_Exit 0925_PB & HSL 0_Without filter.xlsx`, sheet "5 min timeframe",
456 trades 2007-01-22..2021-10-26; only the 97 trades from 2019-02-11 overlap our data.
Oracle settings: entry 14:50, exit 09:25, filter OFF, PB 0, HSL 0, ibs_thr 1, roc_thr -100 (the Mar-2022 AFL has no IBS/ROC/hs
logic, so those are no-ops). Floats; price tolerance 0.05.

## Which lookback
`days=2` (AFL `days` 2 = one higher daily close; the sibling csv is named "..._2day"). days=3 matches 13/97, days=4 2/97.
The full 456 cannot be reproduced (no futures data before 2019-02-11); the nearest check is the 97-trade window.

## Result (days=2, lone 15:30 bar excluded)
| check | match |
|---|---|
| entry date | 94/97 (oracle makes 100 trades; 6 oracle-only) |
| entry price (date-matched) | 91/94 |
| exit date | 94/94 |
| exit price | 91/94 |

With the lone 15:30 bar KEPT (spec section 2 daily close = C1530): entry date 64/97 (76 oracle trades). The feed's 15:30
one-minute print is a stray/auction value (e.g. 2019-05-17: 15:29 close 11440.8, 15:30 print 11690.3), so the team's
database evidently has no such bar and the daily close is the 15:25 bar close. This is a data finding, not a tuned parameter;
it is the single largest effect. **The Rust data contract should not use 1530 as daily close/high/low.**

## Every mismatch
| signal date | what | reason |
|---|---|---|
| 2019-02-25 | entry px (TL 10865.9 vs 10948.7), exit px | feed's rolling series is the April-2019 contract all Feb/Mar 2019 (~80 pts premium); TL is near-month |
| 2019-03-15 | entry + exit px | same |
| 2019-04-01 | exit px 11730.15 vs 11729.8 | 0.35 pt bar-close difference, no structural cause found |
| 2021-01-01 | entry px 14067.95 vs 14098.7 | feed row for 2021-01-01 has expiry 2021-01-01 (roll artefact); TL on Jan-2021 contract |
| 2021-01-04 | TL-only | expiry roll 31-Dec/1-Jan changes daily closes used by up-close/diff |
| 2020-11-17 | TL-only | feed has a full session on Sat 2020-11-14 (Diwali muhurat) as a trading day; TL excludes it |
| 2021-02-25 | TL-only | 2021-02-24 daily close in feed (15112.4) differs from TL's view; unresolved data difference |
Oracle-only: 2020-11-18 (consequence of 2020-11-14), 2021-02-24 (see 2021-02-25); 2019-03-01, 2019-03-05, 2020-05-11,
2020-12-08 unexplained (borderline conditions on slightly different closes; early-2019 ones coincide with the April-vs-near-month contract issue).
All 97 TL exits in the window are 09:25 (the 81 exits at 09:55 and one at 11:15 are all pre-2019, outside our data).

## Old AFL (2022-03-14) vs final AFL
Old lacks: IBS threshold, ROC days/threshold, the `hs` hard-stop array (old uses ApplyStop when hardstop>0), and Sell price
priority for stops. Different ranges: days 1..10, entry step 10, exit 915..945 step 15, pb 0..10 step 1, ST factor 0..20.
Entry/exit/up-close/intradiff/ST-filter/pb code is the same. The 25-May-2022 "Hard SL" file post-dates the tradelist.
