
## 2026-11-04 — Optimization result list: heading row stays fixed
- Bug: the heading row of the "Optimization result list" scrolled away with the rows. `.tbl { overflow: hidden }` made the table itself the sticky container, so `position: sticky` on the headings never took effect.
- Fix (frontend/react/src/styles.css): `.tbl.lab-opt-all-table { overflow: visible; }`; the headings get `z-index: 1` and a thin bottom line. Only the rows scroll inside the 480px box. Applies to the ADR, YT and ATH optimizers (they share listHTML).
- Checked in headless Chrome with the deployed CSS: heading offset after scrolling 300px went from -299.5px to 0px. Gateway rebuilt and redeployed.

## 2026-11-04 — ATH optimizer "Tradesheet" button: the team's Excel tradesheet (same as ADR "Download TS")
- Request: the single tradesheet from the ATH Optimization result list should have the same format as the team file (TS_NF_En_RSI 14_EX_ATR 5-2,7-4.xlsx), exactly as ADR's.
- Engine (backend/src/ath_run.rs): new optional `bars=1` on ath-run; each trade then carries its own daily bars `[date,o,h,l,c]`, entry to exit, prices in their stored form. Without it the answer is unchanged.
- frontend/react/src/compat/ts-workbook.js: buildTsWorkbook takes a symbol per trade (`sym`) and per bar (`bars.s`) for a stock basket, and optional `mae` (results worked out by the new `maeOf`, so thousands of MAE array formulas are not evaluated in the browser; the formulas are still written). ts-eval.js uses such a stored result (`pre`).
- frontend/react/src/compat/runtime.js: `athLabTradesheet` reruns the combination with `bars=1` and saves `TS_<combination>.xlsx`: the same 5 sheets, formulas, styles and costs (fees + slippage per side, both sides, in C1; rollover 0) as ADR. Trades are sorted by entry date; All Index Explore holds each trade's bars. "All tradesheets" is unchanged (flat, as ADR's).
- Checked: Rust ath_run tests 8/8, tsc clean, frontend 450/450 (2 new basket tests). Real run (ytath_chk_pctswing_turnover_running_buy, hardstop 5 · max_positions 50 · tstart 1, slippage 65): 3,262 trades, 137,896 bars, built in ~5 s, 42 MB. LibreOffice recalculation gives the same values in every cell of the trade sheet; MAE matched on all 3,262 trades even when the stored values were deliberately wrong.
- As in the template: CAGR/DD chain trade % one after another (meant for one position at a time), and a trade entered and exited on the same day shows MAE −100 % (no bar after entry; 3 trades in this run).

## 2026-11-04 — Team tradesheet (Download TS / ATH Tradesheet button): 45 MB -> 1.2 MB
- Problem (user): the ATH team tradesheet was 45 MB. Causes: (1) the workbook was written uncompressed (every part "Stored 0%"; ADR "Download TS" had the same problem); (2) All Index Explore held every bar of every trade (137,896 rows for 3,262 trades).
- Fix 1 (optimize-sheet.js `zipDeflate`, ts-workbook.js `tsBytes`): SheetJS writes the parts, which are re-zipped with the browser's native deflate (CompressionStream 'deflate-raw'), as Excel compresses its own files. SheetJS's own compression was ~3x larger. Used by ADR "Download TS" and the ATH Tradesheet button.
- Fix 2 (ts-workbook.js `maeBars`, ATH only): All Index Explore keeps, per trade, the one bar its MAE formula picks (lowest Low after entry up to exit; highest High for Short). The MAE formulas stay live and give the same results. ADR keeps the index's full history.
- Checked on the real run (ytath_chk_pctswing_turnover_running_buy, hardstop 5 · max_positions 50 · tstart 1, slippage 65, 3,262 trades): 45.2 MB -> 1.19 MB; every cell of the trade sheet equals the full-bars file of the same run; LibreOffice recalculation gives the same values (0 cells differ). tsc clean; frontend 452/452 (new: maeBars equivalence, zipDeflate round trip). Gateway redeployed.

## 2026-11-04 — ATH optimization sheet (Optims xlsx): Rules sheet like the lab's
- Compared "NSE 500 C&P Long Daily Optims (1).xlsx" (ours, ATH) with "NIFTY 50 Long Daily Optims (4).xlsx" (Indicators lab): same sheets, columns, formulas and styles; differences were in the Rules sheet.
- Fix 1: Entry / Exit / Filter / Setup now give the rule text, then " · " and the optimized ranges (as the lab: "RSI(14) crosses above 30 · RSI Period - 14,18,1"); optimized settings show at their From value. Before, Entry held only ranges.
- Fix 2: portfolio / backtest settings (max_positions, tstart, pct, equity, sizing, alloc, deadstock, ranking, …) are grouped under Setup (athGroup 'S'), not Entry.
- Brokerage: the value the optimization ran with (stored on the result when it is fetched), so a later change in the box does not alter the sheet. Strategy line unchanged (user's choice).
- File: frontend/react/src/compat/runtime.js (athGroup, athLabSheet, ATH optimize fetch). tsc clean; frontend tests pass; gateway redeployed.

## 2026-11-05 — ATH optimization grid: only the settings that decide trades
- User's choice: the Optimization grid of every ATH sub-variant no longer offers the money / data settings: FIXED EQUITY / % OF Equity (sizing, all 233), EQUITY ALLOCATION (alloc, all 233), DEAD STOCK (deadstock 225, deadticker 1); Equity and brokerage were already left out. They run at the backtest page's values.
- Also left out: settings whose engine label says the AFL's trades never read them ("not used", "plot only", "Explore filter only: no effect on the backtest", …), e.g. TRADING START BAR in 16 variants; varying them only repeated the same run.
- Kept by request: MAXIMUM POSITION and POS. SIZE %.
- File: frontend/react/src/services/ath-optimize.ts (NOT_VARIED, UNUSED); test in ath-optimize.test.ts. tsc clean; 452 frontend tests pass; gateway redeployed.

## 2026-11-05 — YT main-variant cards: stats row and "Best on" line removed
- User's choice (after the sample page https://claude.ai/artifact/N86Jof2oATcRji2AtyiyDp): the Main variant cards of a YT family page no longer show Backtested / Unavailable / Best Sharpe, nor the "Best on NSE 500: … · N trades" line.
- Kept: green dot, Main variant N, sub-variant count, name, family line, description, View sub-variants button.
- Even cards: the button row (thin rule above it) is pinned to the bottom of the card (margin-top: auto), so it lines up across a row whatever the description length.
- Files: frontend/react/src/compat/runtime.js (ytGroupCards no longer computes the per-group results), src/styles.css (.card.yt-main rules). tsc clean; frontend tests pass; gateway redeployed.

## 2026-11-05 — ATH optimization speed (fixes 1, 2, 3)
- Measured first: one ATH run = read ~1,300 stocks (0.04 s), buy/sell signals (0.2–0.75 s, the main cost), net backtest (0.06–0.5 s), a second gross backtest (same again), JSON (<0.01 s). The optimization repeated all of it for every grid point. (ADR/BTST index optimizer already fast: 600 combos in 0.2 s; not changed.)
- Fix 1: the optimization no longer runs the gross (brokerage 0) backtest; its rows and Optims sheet read only the net run.
- Fix 2: stocks, index series, period and symbol order are read once per optimization; the run permit is held for the whole grid (before, each grid point took it in turn).
- Fix 3: buy/sell signals are reused while the settings the variant's signal code actually read keep their values (recorded per run, so ytath_entry_and_pctswing_exit_with_scale_in, whose signals read MAXIMUM POSITION, still gets new signals per value). Grid points are run grouped by those settings; rows keep the grid order.
- Single ath-run unchanged in behaviour; run_json split into variant_of / Knobs / setup / signals_of / portfolio_of so the optimization reuses the steps.
- Checked: 6 optimizations (sheet=1, slippage, from/to, 1–3 ticked settings, reshuffle and scale-in variants) and 2 single runs give identical JSON to the old engine (only the timing field differs).
- Speed, 60 combinations: large_mid_cap MAXIMUM POSITION × POS. SIZE 54.0 s → 5.5 s; EXIT % × MAXIMUM POSITION 52.4 s → 18.8 s; reshuffling MAXIMUM POSITION × POS. SIZE 87.6 s → 31.3 s.
- File: backend/src/ath_run.rs (backup in the session scratchpad: ath_run.rs.bak-optspeed). ath_run tests pass; engine-in redeployed.

## 2026-11-05 — ATH optimization grid: POS. SIZE % out, ATH grouping, entry rule shown
- User's choice: POS. SIZE % (pct) no longer offered for variation (runs at the backtest page's value); only MAXIMUM POSITION stays of the money settings.
- Also hidden: settings whose engine label says they have no effect ("plotted only", "overwritten by the second", "no effect"), e.g. ST ENTRY Period/Factor (plotted only).
- The ATH grid is grouped Entry / Filters / Exit / Portfolio / Engine options instead of the BTST sections (which put EXIT %, filters and POS. SIZE under "Rule settings"). Entry is always shown and starts with the variant's entry rule as text ("fixed rule, no setting to vary" when the AFL has none).
- athGroup (shared with the Optims sheet's Rules) sharpened: labels with EXIT / ENTRY decide (ST EXIT Period -> Exit), stops / trailing SL / profit booking / holding days -> Exit, index / circuit / VIX filters -> Filters, ranking settings and engine options -> Setup (grid: own "Engine options" heading).
- Example ytath_chk_pctswing_turnover_running_buy: Entry (rule text) · Filters: Penny, Turnover, Aux1, Aux2 · Exit: Hard Stoploss, EXIT % · Portfolio: MAXIMUM POSITION.
- Files: frontend/react/src/compat/runtime.js (athEngineOpt, athGroup, athSections, athEntryRow, optGrid), src/styles.css (.ogrid tr.orule), src/services/ath-optimize.ts (+ test). tsc clean; 452 tests pass; gateway redeployed.

## 2026-11-05 — ATH optimization grid: entry text row removed
- User's choice: the entry-rule text row at the top of the grid is removed; the Entry heading now shows only when the variant has entry settings (headings without settings are left out). Grouping, POS. SIZE % removal and the hidden settings stay.
- Files: frontend/react/src/compat/runtime.js (athSections, optGrid; athEntryRow removed), src/styles.css (.ogrid tr.orule removed). 452 tests pass; gateway redeployed.

## 2026-11-07 — Supertrend long only: matches the AmiBroker sheet (215/215 trades, shares and profit)
- Reference: Yash/…/ytath_supertrend_long_only_buy_st_5_2_up-NSE-500-C-P-2020-01-01-2021-12-31-trades.xlsx, Sheet2 (AmiBroker). Highlighted rows 197-216 = 20 positions open on 31-12-2021 in BOTH runs; our export wrote "Open" and no exit price, so the sheet's date formulas showed #VALUE!.
- Fix 1 (export): open trades are written like AmiBroker's "Open Long": exit date = the bar they are marked at, exit price = that close, exit reason "open" (services/btst-universe.ts universeTradeRows; test updated).
- Fix 2 (engine): this AFL does not set UsePrevBarEquityForPosSizing, so AmiBroker sizes each entry from the equity at that moment (falls ~0.65% x 2 of each position with every entry of the bar; positions held from earlier bars valued at the bar's Open, new ones at their entry price, less the commission to close). New PortfolioParams.current_equity_sizing, opt-in list ath_run::CURRENT_EQUITY_SIZING (this variant only). Before: 41/215 share counts equal; after: 215/215, total profit 11,12,694.30 vs AmiBroker 11,12,694.33.
- Files: backend/src/ath_core.rs, ath_run.rs (+ struct literals in ath_pctswing.rs, ath_v2_d1.rs, ath_v2_d2.rs tests), frontend services/btst-universe.ts + test. 452 frontend tests pass; ATH engine tests pass except books::ytath every_rule_placeholder_is_a_parameter (pre-existing: Codex's EXIT % rule text for ytath_close_entry_on_daily_chart_exit_below). engine-in + gateway redeployed.

## 2026-11-07 — ATH catalogue test fixed (EXIT % of the weekly %swing variant)
- Codex had added EXIT % (exitpct, default 10, 5-35) to ytath_close_entry_on_daily_chart_exit_below in the engine and its rule text, but not to the variant's parameter set in the ATH book, so books::ytath every_rule_placeholder_is_a_parameter failed.
- Added exitpct to that variant's parameter set (books/ytath.rs PSETS, used only by this variant) and to its params in ATH/ath_catalog.json (fn Optimize, so a regeneration keeps it). Nothing else changed.
- 207 ATH engine tests pass. engine-in redeployed.

## 2026-11-07 — Stock-universe tradesheet: AmiBroker "Trade" column; open trades counted in No. of trades
- User's request (Supertrend long only sheet, Sheet2 column B): every trade shows Long / Short, and "Open Long" / "Open Short" while still open, as AmiBroker's trade list does. New column "Trade" right after Symbol in the page tradesheet, the Excel and the CSV (services/btst-universe.ts UNIVERSE_TRADE_HEADERS / universeTradeRows; column indices shifted in runtime.js universeTradeSheet, UNIVERSE_PERCENT_COLUMNS 9/11, tradePage reads the entry date from column 2).
- Engine: each ATH trade carries "short" (ath_run.rs trade JSON) so a short leg is written Short; BTST universe trades have no short and show Long.
- The top tile "No. of trades" now counts all trades including those still open (e.g. Supertrend long only 2020-2021: 215 = 195 closed + 20 open), with "N still open" under it. Win rate and the engine's statistics (optimizer, Optims sheet) still use closed trades.
- Tests updated (btst-universe.test.ts). 452 frontend tests, 217 ATH engine tests pass. engine-in + gateway redeployed.

## 2026-11-07 — Excel downloads: dates as real Excel dates
- User's request: Entry date / Exit date came out as text. The shared sheet builder (runtime.js styledSheet) now writes every plain date cell (YYYY-MM-DD) as an Excel date with format dd-mm-yyyy, right-aligned, so it sorts, filters and works in date formulas. Applies to the tradesheet download and the other Excel sheets built the same way (equity, metadata From/To, DetailLog dates). Times and CSV files are unchanged (CSV has no cell types).
- Checked by writing and re-reading a sheet with the app's Excel library: the cell reads back as a date, shown 01-01-2020. 452 frontend tests pass; gateway redeployed.

## 2026-11-09 — Pack script and Windows setup guide (outside the repo)
- User's request: copy the project to another PC. New files next to the repo, not inside it: `Strategy Vault/pack-strategy-vault.sh` (one .tar.gz with the project + git history, ~/.cache/strategy-vault-webapp data/settings/secrets, a database dump, optional `--with-images`; leaves out target/, node_modules/, dist/, scratch files, idx_data.csv.bak, webapp/.env; writes a .sha256) and `Strategy Vault/RUN-ON-WINDOWS.md` (WSL 2 + Docker Desktop steps, .env fixes, build/start without setup.py, database restore, daily commands, problems/fixes), copied into the archive.
- Checked: script syntax, dry-run file list (11,865 entries, no excluded folders), database dump step. No project code changed.

## 2026-11-16 — ATH: brokerage 0 is used as 0 inside the AFL's own rules
- Cause found from Tulsi Ma'am's sheet "Verification - ATHC with 19%swing sl with probability- WIth Aux- no brokerage.xlsx" (ytath_chk_19pct_swing_sl_probability, brokerage 0): with page brokerage 0 the engine fell back to the AFL default 0.65% for the AFL's own BROKERAGE (virtual-trade loss test), while real trades used 0. First effect BANARISUG 2000-09-12: last virtual trade 112.50 -> 116.00 is +3.11% at 0% (not a loss, no buy, as AmiBroker) but +1.79% at 0.65% (loss, buy).
- Fix (backend/src/ath_run.rs Knobs): comm_for_afl is now Option; the page's brokerage is used as entered (clamped into the AFL's Param range, as AmiBroker does); the AFL default only when the request gives no cost/slippage at all. Runs at 0.65% are unchanged. Affects only ATH variants whose AFL reads BROKERAGE in its rules.
- Result vs the sheet: 5 open 204/204 trades same (was 196, 8+8 different), same exits, total P&L Rs 9,583,975,681 vs AmiBroker 9,583,963,519; 50 open 1,347 shared (was 1,297), 10 AmiBroker-only / 13 ours-only left from 2015 on. The remaining gaps are AmiBroker float32 share rounding (MOTHERSUMI 4,800,000 vs 4,799,999), not signals; not changed.
- 213 ATH engine tests pass. engine-in redeployed.

## 2026-11-16 — AmiBroker full set: shares of a split-adjusted price sized with the stored price
- Same sheet (19% swing probability, brokerage 0). With the brokerage fix the first remaining cash difference was day 1: MOTHERSUMI 03-01-2000 at 0.41666666 (5/12) got 4,799,999 shares, AmiBroker 4,800,000 (5 open; 480,000 vs 479,999 in 50 open), and MOTHERSUMI 24-01-2003 (89/120) 260,090 vs 260,091.
- Fix (backend/src/ath_core.rs ab_shares, full-set variants only): the size divides by the 2-decimal price when the stored float32 is a 2-decimal price (MARICO rule kept), else by the stored float32 itself instead of its 7-digit decimal. Unit test added.
- Result: 50 open cash now equal to AmiBroker to the paisa until 22-01-2008 (was from day 1). 5 open still 204/204.
- Still open (not changed): two split-adjusted exits whose AmiBroker profit no rule found fits (42 of 44 split-adjusted trades fit the 7-digit rule): MOTHERSUMI 15-12-2006 -> 22-01-2008 (Rs 295,066.19 vs ours 295,067.65) and HINDZINC 17-11-2009 -> 05-02-2010 (144,662.56 vs 144,664.49). Their Rs 3.39 more cash gives ORIENTPPR 10-04-2014 one more share (cash-limited 45 vs 44) and the 50-open run cascades from 2015 (10 AmiBroker-only / 13 ours-only).
- Other full-set variants: ytath_entry_and_pctswing_exit_with_pct_hard_2 and ytath_buy_only_if_within_10pct_range_of_st unchanged; ytath_running_pctswing_after_2_pctswing changes only MOTHERSUMI 2000-01-03 (its 2008+ Tulsi equity sheets give the same result before and after); ytath_chk_all_st_variations_all_ranking changes from MOTHERSUMI 2000-01-03 on (no team sheet). 213 ATH engine tests pass; engine-in redeployed.

## 2026-11-16 — 6-decimal check copy of the 19% swing probability AFL (for Tulsi Ma'am)
- New file `AFL ATH checking/6 DECIMAL CHECK/ATHC with 19%swing sl with probability - 6 DECIMAL CHECK.afl` (original AFL unchanged): BROKERAGE minimum .01 -> 0 (as in her no-brokerage run) and the same "X ..." 6-decimal trade-list columns as the AFL Default Equity Solving copy (equity prev bar / entry bar, entry/exit price, shares, entry/exit value, commission, profit). Purpose: AmiBroker's own numbers for MOTHERSUMI 15-12-2006 -> 22-01-2008 and HINDZINC 17-11-2009 -> 05-02-2010. Copied to the share.
