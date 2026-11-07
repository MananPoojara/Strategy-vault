# BTST (YT) verification record, 2026-10-23

Strategy: `ytbtst_up_close_ibs_roc` (book `yt_btst`), Nifty front-month futures 5-minute bars on the NIFTY 50 calendar.
Source of rules: the team's final AFL (2022-09-08). Not a book method; no published figures exist. Admission: index/ETF only, and only
the NIFTY 50 ticker has the BTST series.

## Source files (SHA-256)
| file | sha256 |
|---|---|
| final AFL `btst trad in intraday with flt and profit booking with IBS flt and ROC ranking .afl` | b69db4912beefa8f36d8bdd6aa1450b39fcc0b91b2d5a9e8f351eca8bf127a11 |
| combined AFL `BTST_Combined_Master.afl` (not used by v1) | 18ba50d326103136cba50f7b8aabcb17795d5196c0278c81e82a82d09cc3314c |
| older AFL that produced the tradelist `btst trad in intraday with flt and profit booking.afl` | 473e526b4f5ce4bcbd45f9accf435666bf41a704059cf2d201cda510938eea1d |
| tradelist `NIFTY_Tradelist_BTST_Entry 1450_Exit 0925_PB & HSL 0_Without filter.xlsx` | 7d2de309240f5d7f291d88be7ba2898e8b6cc25a4a36f34456ae1081fb751ec3 |
| `BTST Master AFL rules.xlsx` | 2f56a20e22e0a1f747baaf21c4e21d661bb163935142b721ae6d785a1085e14e |
| `BTST AFL list.xlsx` | 130709a680fefe81e5bd49aafad940bfd54899ffca230a957dbdaf9e23d2efda |
| `rolling_futures.parquet` (copy of the share's file) | 46958f8b39724f0b0bd33c9e55b3ec232f0f690ace71320446361bc464e17f47 |
| generated `NIFTY_50__BTST_5M.csv` | cc1b931ad6ac630040802a79f6b9ede1e119e288fcc16028196be15e70681f68 |
| oracle ledger, defaults | 81de2155c83427d03e1fc34f4d5b5ed18abaa33ff8113d652b7fa4e3d6a5542a |
| oracle ledger, no-filter tradelist window | 2d18ef772701a143b5560d6e205cf33f224b42d9ca1417a9c0097e45244d3921 |

## Pipeline steps (VERIFICATION-PIPELINE.txt) as applied to a YT strategy
| step | status |
|---|---|
| 1 Find the source | done: AFL + tradelist + rule sheets, hashes above |
| 2 Read the whole source | done for the final AFL and the 22-variant index; the other 21 variant AFLs were read for the combined AFL only |
| 3 Already coded? | no BTST existed in the catalogue |
| 4 Source contract | `BTST/BTST_VAULT_SPEC.md`; labelled deviations: ROC days >= 2 (AFL default 1 reads a future close), ROC threshold default 1 (AFL default 5 is outside its own range), exit-time step 5, Supertrend period >= 1, futures prices on the cash-index calendar |
| 5 Instrument admission | index/ETF only (YT policy); NIFTY 50 only because only that ticker has the series |
| 6 Code it | done, `books/ytbtst.rs` (23 tests) |
| 7 Independent numbers | done: Python oracle built from the 1-minute data (not from the generated CSV). Engine vs oracle: defaults 15/15 exact, tradelist window 99/100 exact (last trade is the window boundary) |
| 8 Independent review | NOT DONE: the oracle and the Rust code were written by agents from the same spec. A human reviewer who re-reads the AFL has not signed off |
| 9 Comparison with team sheets | done against the NIFTY tradelist on the 2019-02-11..2021-10-26 overlap (97 trades): entry date 94/97, entry price 91/94, exit date 94/94, exit price 91/94. Not covered: profit booking, hard stop, trend filters (the tradelist has them off); unexplained oracle-only trades 2019-03-01, 2019-03-05, 2020-05-11, 2020-12-08 |
| 10 Integrate and test | engine tests pass (ytbtst 23, yt 43, ownership 2, pristine routes 5, readiness 1); the full suite has 1 failure that needs the private idx_data.csv; no frontend change |
| 11 Browser check | done: YT list, strategy page (all six tabs), optimization page, exports present |
| 12 Publish | NOT DONE: nothing committed or pushed |

## Known limits
- The conclusion boxes ("Claude's / GPT's conclusion") analyse the whole history and show "No conclusion" for BTST because the futures data starts 2019-02-11.
- A period outside 2019-02-11..2026-09-29 returns needs_data (no estimates).
- From 2026-08-03 the futures trade until 15:40; the generator drops minutes after 15:30, and the daily close uses the 15:25 bar.
- Profit-booking / hard-stop / filter-on paths are verified by hand-calculated tests and the oracle only.

## Second tradelist check (added 2026-10-23): 3-day lookback, entry 15:15, exit 09:20
File: `BTST_Intra day/BTST_Intraday after 3 consecutive upclose_Without profit booking & SL/btst trad in intraday ver2_updated-Intraday-0.03-BT-NIFTY-BN.xlsb`
(binary .xlsb; converted with LibreOffice to a scratch copy for reading, original untouched). Sheet `NIFTY-TL`: NIFTY_F1, 81 trades 2007-03-13..2021-10-14,
entry 15:15, exit 09:20 (10:00 on some), AFL `btst trad in intraday ver2_updated` (3 rising closes, no profit booking / stop-loss). Sheet `BN-TL`: BANKNIFTY_F1, 87 trades.
Oracle (days 3, entry 1515, exit 920, PB 0, HSL 0, filter / IBS / ROC off) on the overlap 2019-02-11..2021-10-26: team 14 trades, oracle 14 trades,
**12 shared entry dates, and on all 12 the entry price, exit date and exit price match** (tolerance 0.05). Differences: team-only 2019-05-20 and 2021-01-04 (the
31-Dec/1-Jan roll artefact is a known data issue); oracle-only 2019-03-05 and 2019-04-02 (the feed carried the April contract in Feb/Mar 2019). BankNifty cannot be checked (our data starts 2023-09).

## Variants (added 2026-10-23)
BTST is a family of 7 variants (ids `ytbtst_lb2_nofilter`, `_lb2_st_weekly`, `_lb3_1515`, `_lb3_1450`, `_lb3_st_weekly`, `_lb4_nofilter`, `ytbtst_up_close_ibs_roc`), default = `ytbtst_lb2_nofilter`
(team's 456-trade Nifty list setup: lookback 2, entry 14:50, exit 09:25, no profit booking / stop / filter / IBS / ROC; 251 trades on 2019-02-11..2026-09-29).
Each variant was run in the engine with its own defaults (no overrides) and compared with the oracle: `scripts/compare_btst_variants.py` outside the repo (7/7 agree; the only
differences, 1 to 3 trades per variant, are in Aug-Sep 2026 when the futures session runs to 15:40). Only variants A (456-trade list, 90 of 97 trades in the overlap) and C
(81-trade list, 12 of 12 shared trades) have a team trade list to compare against; B, D, E, F are verified against the oracle only. Source of the variant settings: sheets of
`NIFTY & BANKNIFTY.xlsx` ('NIFTY_Without filter' lookback 2/3/4, 'NIFTY_With ST 20,3 weekly', 'NIFTY_Wo flt_PB optim', 'Rolling return').

## Reference AFL and trade-sheet locations (added 2026-10-24)
See `BTST/BTST_Variant_Reference_Sources.md` in the BTST folder: for each Vault variant it gives the reference AFL file, the reference trade sheet (file, sheet, trade count, hash) or, where no trade list exists, the summary sheet with the team's trade counts. Only variants A (456-trade Nifty list) and C (81-trade list, `ver2_updated`) have a trade list; the AFL behind A is inferred, not named by the workbook.
