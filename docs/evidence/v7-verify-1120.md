# V7 FINAL (20 Nov 2026, second pass) - verification of the int-1119l..int-1120c releases

**Result: 0 of the listed ids pass R4 #368/#370/#371 in full. None promoted; every gate row stays unverified/withheld. No calculation or live behaviour changed.**
Commit on claude/v7-1120: gate.rs reason text only (20 rows get a "V7 re-check" sentence with the exact comparison). Strings only; no cargo build or test was run on it.

## Method (what was actually done this pass)
1. Printed figures read from the SOURCE, not from gate/FINAL quotes: pdftotext -layout of the share PDFs into authors/lanes-1120/V7w/src (pbwts, mkits, bta, rwmts, kets, capttm) and Piranha.txt; image pages via authors/ocr2 (Zweig pp.65-71 of the scan, Table 1 read from the OCR) and page images by eye (Crabel PDF pp.196-197; Williams Vol.32 No.5 p.3 JPEG).
2. Live binary `snapshot-refresh/bin/guru-int-1120c` as a private engine via authors/heavy (HEAVY_HIGH 1800M / HEAVY_MAX 2G), US data on port 19881 and India data on 19882 (earlier pass), each started with `&`, PID recorded in authors/lanes-1120/V7w/{us,in}.pid, killed by that PID only (port closed afterwards). Runs: /api/guru/connors/run, cost/slippage/rollover 0, params as `p.<key>`. Nothing written to data dirs.
3. Judged per R4 #368/#370/#371 (trades within max(4,10%), win within max(8,100/n), money/percent figures within 10%, every printed row must match).
4. Limit I could not remove: the live binary clamps &SP to 1982-04-21..1995-07-31 (R4 #330) and has no series for several book instruments, so book-window rows for piranha / pbwts use the engine's own trade dumps run past the clamp (authors/held-1119/F2/trades-book.csv, F3/trades-pbwts_*.csv); I recomputed every printed row from those dumps myself.

## Table: id | printed (page) | ours | verdict
| id | printed (page) | ours | verdict |
|---|---|---|---|
| lebsp_piranha | Piranha manual Table 1, 6/15/88-10/29/99, $250/pt, $150/trade: 156 tr (127 L/29 S), 76% (119 W), net 305,037.50, GP 435,850, GL 130,812.50, avg trade 1,955.37, avg W 3,662.61, avg L 3,535.47, max W 15,475, max L 12,425, consec W/L 16/2, PF 3.33; short 29 tr, 83%, 90,700, avg 3,127.59 | book window (engine, past clamp): 167 tr (137 L/30 S), 74.9%, net 288,692 (-5.4%), GP 432,635, GL 143,943 (+10.0%), avg 1,728.69 (**-11.6%**), avg W 3,461, avg L 3,427, max W 14,382, max L 9,225 (**-26%**), consec 20/4, PF 3.01; short 30, 76.7%, 65,175 (**-28%**), avg 2,172.50 (**-31%**). Live published window 109 tr reproduced | FAIL. UNVERIFIED |
| pbwts_intermarket | Pruitt & Hill Table 8.11 p.193, S&P 5/3/96-4/19/02: 29 tr, 11 W (37.93%), net 37,537.50, GP 150,000, GL 112,462.50, avg 1,294.40, avg W 13,636, avg L 6,248, max W 27,750, max L 20,575, consec W/L 4/7, PF 1.33 | engine past clamp, $250/pt: 28 closed + 1 open, 11 W (39.29%), net 20,525 (**-45%**), GP 149,425, GL 128,900 (**+14.6%**), avg 733.04 (**-43%**), avg W 13,584, avg L 7,582 (**+21%**), max W 28,200, max L 21,950, consec 3/5, PF 1.16. Live published 1982-95: 72 tr, 23.6% | FAIL. UNVERIFIED |
| pbwts_cot | Table 8.3 p.175: 68 tr, 64.71% (44 W), net 297,567.50, GP 399,442.50, GL 101,875, avg 4,375.99, PF 3.92, open P/L 93,525; no window | live &SP 1992-10-06..1995-07-31: 1 open trade. Engine to 12/21/2001: 49 closed + 1 open, 71.4%, net 314,670 (+5.7%), GL 86,938, avg 6,422 (**+47%**), PF 4.62 | FAIL (trades 49 vs 68, avg). UNVERIFIED |
| mkits_gold_dis_dxy | Katsanos Table 11.2 p.177 (Disparity DXY), 12/1/95-8/21/07: 31 tr, 74%, +$323,637, 24.1% in market | live GLD in that window: 7 tr, 86% (GLD starts 2004-11-18; &DX stand-in) | FAIL trades. UNVERIFIED |
| mkits_gold_lrs_dxy | Table 11.1 p.176 (LRS DXY): 21 tr, 90%, +$282,257, 20.7% | 2 tr, 100% in window | FAIL trades. UNVERIFIED |
| mkits_gold_rs_dxy | Table 11.3 p.178 (RS DXY): 9 tr, 78%, +$141,363, 20.6% | 2 tr, 50% in window (first entry 2006-12-05) | FAIL trades. UNVERIFIED |
| mkits_gold_z_dxy | Table 11.2 p.177 (Z-score DXY): 24 tr, 71%, +$409,960, 38.4% | 0 tr in window (first signal 2007-11-15) | FAIL trades. UNVERIFIED |
| ch_bta2_cbo_20tick | Chande BTA Tables 5.4/5.5 pp.157-158, 9 markets, window 1/1/75-7/10/95 (chapter text), $1,500 stop, $100/trade. e.g. 5.4 BP 222 tr/41%/$24,588; Coffee 213/39/$175,631; Gold 142/32/$31,610; T-bond 77/38/$29,925 | live runs per market (my $ = pts x contract $/pt - $100): 5.4 BP 156/40%/$25,612; Coffee 147/38%/$59,964; Gold 130/36%/$42,310; T-bond 63/33%/$18,979; 5.5 BP 107/24%/$61,512; Yen 86/34%/$58,688 ... trades ok 1/9 (5.4), 2/9 (5.5); win ok 7/9, 8/9; net ok 1/9 (BP), 0/9 | FAIL (no market passes all three). UNVERIFIED |
| tcdt_tb_orb2 | Crabel Tables E,G-L pp.202-204, Bonds 1978-1987, 56 rows (read on page images; E row1 330 tr/63%/avg W 469/avg L 350/gross 55,321; G1 151/64/459/328/26,814; H1 85/73/468/278/22,680; I1 35/80/619/376/15,468 re-read by me and equal to the C1 transcription) | live &ZB_CCB 1979-87, 56 rows: E1 344/52%/437/274/32,978; G1 170/55/473/261/23,915; H1 72/64/454/267/13,938; I1 42/67/432/161/9,842. Trades ok 44/56, win ok 31/56, gross ok 3/56, **all five columns ok 0/56**; avg loss is 20-35% below print in most rows | FAIL. UNVERIFIED (no gate row exists; the card text carries it) |
| rwmts_2h_cb15_8 | Weissman Table 5.4 p.93: T-bond day session 12/30/98-1/30/04: 89 tr, 7.3 days, 52.81%, $12,450, MDD -12,237; IEURUSD 3/28/00-1/30/04: 95 tr, 6.9 days, 51.58%, $20,025, MDD -7,840; $100 / $75 per trade | live SPY 653 tr/51.2%, SPX 472/55.2%, NIFTY 50 435/50.0%, NIFTYBANK 367/50.5% (equal to gate). Neither book instrument is held | not compared (missing instrument). UNVERIFIED |
| kets_vb_limit_ccy | Katz Test 11 p.101-102: IS (8/1/85-12/31/94) 268 tr, 48%, $3,977, 36.2%/yr; OOS (1/1/95-2/1/99) 102 tr, 43%, $2,106, 17.7%/yr; 6 currencies | live &6B/&6S/&6J in those windows: IS 49+35+53 = 137 tr, OOS 9+13+15 = 37 tr (DM, CD, ED not held) | FAIL / not comparable. UNVERIFIED |
| kets_solar_open/limit/stop | Katz p.199-200: portfolio $/trade IS open -1,631, limit -1,519, stop -52, OOS stop -1,329; S&P 500 stop IS 37 tr, 40%, $4,991 (20.4%/yr), OOS 11 tr, 45%, $5,640 (39.5%/yr) | no S&P futures admitted; SPX in this engine starts 2014, so IS/OOS windows give 0 trades; SPY (ETF) stop OOS 1995-1999: 14 tr, 43% (10 tr, 30% with causal 0 / sameday 1) | not compared (missing instrument). UNVERIFIED |
| kets_ga_{long,short}_{limit,stop} | Katz Ch.12 p.273-274 text: Tests 1-3 IS >$14,000/trade, >=60% wins, ROA >=42%; OOS open/limit >$10,000, stop ROA 11%, $4,246; 43 IS / 17 OOS trades; 36-futures portfolio (Table 12-3 image rows not text-readable) | NIFTY 50 18/72.2%, 10/50.0%, 10/30.0%, 6/66.7%; NIFTYBANK 17/52.9%, 10/50.0%, 36/30.6%, 29/31.0% (live, equal to gate) | not compared (missing instrument). UNVERIFIED |
| capttm_price_ma | Arnold p.81: rule only (200-day, 10/20-day option), no table | SPY 111 tr/24.3%; NIFTY 50 115 tr/17.5% | nothing printed. UNVERIFIED |
| mstt_bns, mstt_bns_by_brook | Peter's Window systems p.3: formulas only; per gate no result printed (source not re-opened this pass) | mstt_bns SPY 322 tr, NIFTY 50 265; by_brook NIFTY 50 95 | nothing printed. UNVERIFIED |
| mzwws_prime_rate | Zweig Tables 1-2 (OCR pp.69, 71 of the scan): 43 dates (22 buys/21 sells), ZUPI $10,000 -> $610,554 (18.7%/yr), 90.9% buys / 61.9% sells right; S&P buys $318,068, 15.5%/yr vs 7.9%, 18 of 22 up (81%) | all 43 printed dates equal the Z1 comparison list: 26 exact, 7 within 1-6 days, 10 not reproduced (FRED prime). Live SPX here starts 2014 -> 1 trade; no series for 1954-96 | FAIL (dates). UNVERIFIED |
| lwnl_sp_hornbrook_im | Williams Commodity Timing Vol.32 No.5 p.3: IS 297 tr, 62%, $74,900, avg $252; OOS 72 tr, 62%, avg $315; windows not printed | live &SP 1982-95: 361 tr, 60.4%, avg $269. First 297 by date: 60.9%, $264, net $78,540 (+4.9%); remaining 64: 57.8%, $289 (64 vs 72 trades, 8 > 7.2) | split is mine, windows unprinted. UNVERIFIED |
| other 97 lwnl_* | newsletter dollar results for S&P futures / bonds / commodity contracts, mostly without a window | only lwnl_sp_hornbrook_im accepts the &SP contract (live endpoint returns 400 for the others); no futures series for the printed windows | not compared (missing instrument/window); NOT re-read page by page (see below) |

## Possible code concern (not changed, for the lead)
tcdt_tb_orb2: engine and the independent pandas reference agree (0 mismatches on 6,305 trades), so the code does what the card states, but ours is systematically lower than the page: average loss 20-35% under print in most rows (E1 274 vs 350) and average win ~10% under, gross far smaller (E1 32,978 vs 55,321), while counts match within tolerance on 44/56 rows. The page header says "1978-1987"; the card runs 1979-01-02..1987-12-31 on the Norgate back-adjusted &ZB_CCB. A magnitude-wide shift like this is worth a look at the entry/exit price definition (open +/- 8 tics fill, close exit) against p.203, but I found no page evidence that the rule differs. No before/after is possible because nothing was changed.
Data note: the engine data dir in authors/data-1117 has SPX from 2014-01-02 only, whereas Z1 reported SPX from 1983; check that the deployed data is the intended one.

## Not done
- The other 97 lwnl_* cards were not each re-read from the Williams JPEG pages (the zip holds ~110 issue pages; each card needs its own page read, and the engine cannot run them on the book instrument anyway). Only Vol.32 No.5 p.3 (Hornbrook) was read.
- mstt_bns p.3 and capttm p.81 reconfirmed only for capttm; Table 12-3 of Katz and the Crabel rows beyond the four re-read ones rely on the C1 transcription.
- No cargo build/test of the gate.rs text edit.

---

# ROUND 2 (20 Nov 2026): the 97 other lwnl_* cards. Commit dedc5636 on claude/v7-1120 (gate text only, 97 rows). **0 of 97 verified; every row stays unverified (or withheld).**

Method (corrected): printed figures are as recorded by lanes O1-O5 from the scans and were NOT re-read in V7 except the pages listed here; reuse of the O1 OCR text in authors/held-1119/O1/text where it was legible, otherwise the page image: Vol.29 No.7 p.3, No.8 p.3, Vol.28 No.5.1 p.2, Vol.34 No.4 p.2, Vol.32 No.5 p.3 read as images; Vol.34 No.12 p.4, Vol.32 No.5 p.2, Vol.33 No.8 p.2/4, Vol.34 No.9 p.3, Vol.28 No.9 p.3-4, Vol.33 No.11 p.3-4, Vol.29 No.9 p.3 confirmed in OCR text); ours from guru-int-1120c (private engine, authors/heavy, PID in V7w/us.pid, killed by PID, port closed). Per-card lines: V7w/single.out, tail.out (assumed-window runs), tab.out (131 table rows), sp.out (S&P cards). Printed windows are given only where the page prints one; otherwise the full history is run and, where the letter states N years, a second run on the last N trades to the issue year (labelled "assumed").

## A. Single-system futures cards (live on own contract) - 21
| id | printed (page) | ours (live) | verdict |
|---|---|---|---|
| bond_dec_td14_buy | V34 N12 p.4: 14 of 19 (73%), $3,250 | &ZB full 42 tr, 81%, $7,820; assumed last 19 (1978-96): 19 tr, 89%, $6,226 | FAIL (net) |
| bond_dec_td22_buy | 9 of 10 (90%), $4,593 | full 15 tr, 80%, $2,550; assumed: 5 tr, 100%, $3,813 | FAIL |
| bond_dowm_buy | V32 N5 p.2: 190 tr, 60%, $48,031.25, avg $252.80 (1988-May 96) | 144 tr, 57%, $37,028, avg $257 | FAIL (trades 144 vs 190, net -23%) |
| bond_jul_td12_sell | V34 N7 p.3: 94% accuracy, count/window not legible | full 43 tr, 74% | FAIL (win rate) |
| bond_labor_buy | V33 N8 p.2: 13 yrs, 83%, avg $700 | assumed 13 tr, 85%, avg $329 | FAIL (average) |
| bond_memorial_buy | V29 N5 p.4: 10 of 12, avg $511, $6,140 | assumed 12 tr, 100%, $4,627, avg $386 | FAIL |
| bond_newyear_sell / bond_xmas_buy | V34 N12 p.4: "a big winner", no figures | 42 tr, 55% / 62% | nothing printed |
| bond_sep_td19_buy | V33 N8 p.4: 9 of 16 yrs, $13,356 | assumed last 16 to 1995: 16 tr, 56%, $13,812 (+3.4%) | headline within tolerance on an assumed window; unstated details remain, so UNVERIFIED (full history 41 tr fails) |
| bond_sep_td20_buy | V34 N9 p.3: 16 of 16, $6,737 | assumed 16 tr, 75%, -$379 | FAIL |
| bond_sep_td4_sell | 17 of 18, $6,056 | assumed 18 tr, 78%, $494 | FAIL |
| bond_sep_td9_sell | V33 N8 p.4: 12 of 16, $11,293 | assumed 16 tr, 62%, $8,332 | FAIL |
| cattle_jul_td18 | V29 N7 p.3 Test 5: 15 tr, 86%, $6,861, avg $457.40, GP 8,051, GL 1,190, avg win 619.31 | &LE full 30 tr, 63%, $9,512; last 15 to 1994: 13 tr, 100%, $9,558 | FAIL |
| corn_jul_td2_short | V29 N7 p.3 Test 24: 18 tr, 83%, $16,190, avg $899.44, GP 19,418.75, GL 3,228.75 | &ZC full 36 tr, 56%, $13,086; last 18: 13 tr, 69%, $11,045 | FAIL |
| coffee_aug_td12_open | V29 N8 p.3 clip 1 (1974-94): 20 tr, 60%, $29,411.25, avg 1,470.56 | &KC 1974-94: 16 tr, 69%, $18,188, avg $1,137 | FAIL (net -38%) |
| coffee_aug_td12_limit | clip 2: 15 tr, 73%, $30,615, avg $2,041 | 9 tr, 56%, $4,440, avg $493 | FAIL |
| gold_13x13 | V28 N5.1 p.2 printout: 175 tr, 40%, $112,910, GP 202,030, GL 89,120, avg 645.20, avg win 2,886.14, PF 2.27, consec 3/7 | &GC to 1993-06, $50: 157 tr (18 apart, limit 17.5), 38.9%, $110,600 (-2.0%), avg $704 (+9.1%), avg win 3,225 (+11.7%), PF 2.28, consec 3/6 | FAIL by a hair (trades, avg win). Closest match in the set |
| gold_nov_td16 | V33 N11 p.4: 19 tr, 100%, $16,560, avg $871.58 (5/28/75-10/29/96, $50) | 17 tr, 94%, $18,790, avg $1,105 | FAIL (net +13%, avg +27%) |
| gold_tdom1_4d | V29 N3 p.4: 84 tr, 53%, $41,850, avg $498.21 | 90 tr, 52%, $47,270, avg $525 | FAIL (net +13%) |
| wheat_sep_td1_5_buy | printout p.3 (V25 N8/V33 N8 p.3 text): 25 tr, 80%, $14,675, avg $587 | &ZW full 41 tr, 54%, $8,138; last 25: 18 tr, 83%, $10,852 | FAIL |
| yen_apr_td4 | V34 N3 p.3: 20 tr, 60%, $20,487.50, avg $1,024.37 (11/24/76-2/21/97, $50) | &6J 17 tr, 65%, $18,175 (-11.3%), avg $1,069 | FAIL (net just outside 10%) |
| yen_nov_td10 | V33 N11 p.3: 17 tr, 88%, $8,862.50, avg $521.32 | 17 tr, 76%, $4,950, avg $291 | FAIL (win, net) |

## B. Futures cards the engine refuses (withheld by the accuracy check #240/#297) - 6
bond_dec_td7_sell, bond_dec_td8_sell (V34 N12 p.4: 15 of 16, $7,050; 15 of 18, $7,493), bond_projected (V29 N9 p.3: 352 tr, 257 W, ~$71,500; OCR confirms 71,472.50 / 257), cti_bonds (Jun 1992 p.3: 98 tr, 41%, $17,756.25), bond_sp_9ma_1down / 2down (V28 N9 p.3-4: 308 tr 59% $116,900 avg $379.55; 145 tr 58% $43,875 avg $302.59; OCR confirms all). The live endpoint returns not_testable, so no run on the own contract is possible; the gate's recorded reference does not match counts (projected 501 vs 352, cti 175 vs 98, 9ma 396 vs 308 and 187 vs 145; dec td8 win 95% vs 83%). Stay withheld.

## C. Table cards (R4 #375, every printed row run on its own contract, full history because no window is printed) - 9 cards, 131 rows
| card | printed (page) | rows within tolerance on every printed field | verdict |
|---|---|---|---|
| jan97_list (8 rows) | V34 N1 p.3-6 | 1/8 (row 6 Swiss franc: 7 tr, 86%, $4,375 vs ours 7 tr, 86%, $4,110) | UNVERIFIED |
| jan97_bonds (2) | V34 N1 p.8 | 0/2 | UNVERIFIED |
| apr96_list (9) | V31 N3 p.4 (text: win % and $/trade only) | 0/9 (win ok 5/9, average ok 0/9) | UNVERIFIED |
| apr97_7day (25) | V34 N4 p.2 (3 rows re-read on the page image and equal to the card) | 0/25 (trades ok 0/25, win ok 3, average ok 1) | UNVERIFIED |
| jul97_7day (17) | V34 N7 p.2-3 | 0/17 | UNVERIFIED |
| may97 (52) | V34 N5 p.3-4 | 0/52 (win ok 17, average ok 2) | UNVERIFIED |
| sep97_7day (14) | V34 N9 p.2-3 | 0/14 | UNVERIFIED |
| bonds_oct (2) | V33 N9 p.3-4 | 0/2 | UNVERIFIED |
| ho_mar_buy (2) | V34 N3 p.3 | 0/2 | UNVERIFIED |
Systematic: our full-history trade counts are roughly 1.5-2x the printed counts (the printed windows are shorter and unprinted), which alone fails #368 on trades for nearly every row.

## D. S&P-letter and other index cards - 61
These cards are admitted on the cash S&P 500 only ("a declared application of the futures rule"): on &SP the engine returns needs_data (instrument refused), on SPX the engine serves them from 2014-01-02 although the data file holds SPX from 1983, and a number are refused outright (not_testable: Held by decisions #243/#177b/#246, or "Backtest unavailable" for od_*). The printed windows are 1982-1997, so there is **no overlap** with any run and no printed row can be compared. Live SPX runs (2014-2026) for the record are in V7w/sp.out (e.g. tg_open8 12 tr 75%, july4_buy 13 tr 100%, aug97_td12_buy_bail 10 tr 10%, proj_limit 1,129 tr 49%). Refused outright: aug97_td2_sell_7d, aug97_td7_sell_bail, jan_rally, jul97_td17_buy, jul97_td4_buy, july4_sell, keyrev_sell, nov97_td17_buy_7d, nov97_td18_buy_7d, od_buy, od_buy3, od_fade_buy, od_fade_sell, od_sell, ndx_vb_oops (SPX needs data). All: **not compared (window / instrument not available), UNVERIFIED / withheld unchanged.**

## For the lead
- The S&P-letter cards (about 35 with printed S&P windows ending 1993-1997, e.g. keyrev 1982-93, tg_open8 to 03/94, xmas_bail to 03/94, tdw_oc 1988-93, jan_rally/nov_td17/oct 1982-96, proj_contra 1982-94, pullback to 06/92, tg_close 1982-91) could be compared if SPX were served from its 1983 start for them, or if &SP (clamped 1982-04-21..1995-07-31) were admitted; both are admission/policy decisions I did not change.
- Not done: the per-row printed figures of the 131 table rows were not each re-read from the images (3 apr97 rows and the single-system printouts were); the S&P cards' printed figures are the ones recorded in the book field (OCR-confirmed for none of them this pass).

## Round 2 correction and continued re-reading (20 Nov 2026, after lead review)
- The gate rows and the Method paragraph above wrongly said all printed figures were read from the JPEGs. Corrected (commit d2700168): they are the figures recorded by O1-O5, re-read in V7 only for: Vol.29 No.7 p.3, Vol.29 No.8 p.3, Vol.28 No.5.1 p.2, Vol.34 No.4 p.2 (rows 1-3), Vol.34 No.9 p.2 (rows 1-3: cocoa 7 5,555 10/15 66% 370; cotton 6 10,025 7/8 87% 1,253; juice 13 7,305 14/17 82% 429, all equal to the card), Vol.32 No.5 p.3; OCR-text confirmations as listed.
- New fact from Vol.34 No.9 p.2: the printed sep97 table covers "the last 15 plus years" (stop $1,200, entry open + 7%), so the printed window is about 1982-1997, not the full history. Our full-history runs (1980s-2020) therefore overcount trades; a re-run of the 7-day tables over the last ~15 years would test that and could change the trade-count and net failures. Not run (no engine slot yet; stopped at the context budget). Same re-run applies to apr97/jul97/may97 if their pages say "last 15 years".
- Still not re-read from the images: the other ~125 table rows and all S&P-letter printed figures.

## Round 2b (20 Nov 2026): table cards re-run over the stated printed windows
Windows from the pages: sep97 "last 15 plus years" (Vol.34 No.9 p.2 JPEG), jan97 "last 18 years" (Vol.34 No.1 p.3 OCR), apr96 "last 16 years" (Vol.31 No.3 p.4 OCR); apr97/jul97/may97 assumed the same 15 years (not stated on the OCR text of those pages). Entries 1982-1996 (jan97 1979-1996, apr96 1980-1995). Result (tab2.out): jan97_list 1/8 rows pass, apr96_list 1/9, apr97_7day 1/25, jul97_7day 0/17, may97 1/52, sep97_7day 0/14; trade counts now within tolerance on about half the rows (apr97 13/25, may97 25/52, jul97 7/17, sep97 6/14, jan97 6/7) against ~0 before, win rate on about half, average on very few (the printed "average" often disagrees with net/trades, as the cards already note). Passing rows: jan97_list r6 (&6S), apr96_list r8 (&ZB), apr97_7day r2 (&CC), may97 r10 (&OJ). No card passes in full: all UNVERIFIED. Gate rows for these 6 cards carry a "V7 round 2b" sentence.
Still open from the lead's brief: re-reading the printed figures of the remaining ~120 table rows and all S&P-letter cards from the JPEGs; windowed re-runs of the 21 single-system cards with a letter-stated window are done only for those with a printed window (coffee, gold x3, yen x2, dowm) and the assumed-years ones.

## Round 3 progress: page re-reads (JPEG images, V7)
Each row of the card's recorded printed figures compared to the page image: apr97_7day Vol.34 No.4 p.2: 25/25 rows equal (incl. bonds sells with the "6 DAY EXIT" / "$1600 STOP" annotations); may97 Vol.34 No.5 p.3-4: 52/52 equal (printed quirks reproduced: cotton copper 49375, cattle 53,280, coffee 250550, juice 9%); jul97_7day Vol.34 No.7 p.2-3: 17/17 equal (rule text: buy open + 7% of the previous range, $1,500 stop or close 7 sessions later; p.3 also prints bonds sell TD12 94% and S&P TD14 86%); sep97_7day Vol.34 No.9 p.2: 12/14 equal (beans / pound sells rows 13-14 are below the part of the page read). The O4/O5 transcription is therefore confirmed on 106 rows with no disagreement.
Round 3 batch 1 (re-read images, commit follows): jan97_list 8/8 (Vol.34 No.1 p.4-6), jan97_bonds 2/2 (p.8), bonds_oct (Vol.33 No.9 p.4), oct_td9_buy / oct_td17_sell (same page, S&P printout), aug97 x4 (Vol.34 No.8 p.2-3: td12 bail $7,120 11/14 78% 508; td18 bail $7,400 10/13 76% 569; td7 sell bail $6,325 9/12 75% 527; td18 7d $12,470 61% 959), nov97 x4 (Vol.34 No.11 p.3: td17 7d $26,825 60% 1,788; td18 7d $25,175 64% 1,798; td6 30d $32,450 46% 2,163; td15 30d $21,825 46% 1,680), dec97 x3 and bond dec x4 (Vol.34 No.12 p.3; td8 prints 13/18 at 83%, a misprint for 15/18), bond_jul_td12_sell (Vol.34 No.7 p.3 text). All equal the cards' recorded figures; no printed disagreement found anywhere. Verdicts unchanged: every one still fails/cannot be compared on the live side.
Not yet re-read: S&P cards from Jan 1997 p.3/p.7, Jun/May/Oct/Nov 1994-95, Sep 1995, 1991-93 letters (jan_rally, keyrev, krut, gap_fail, engulf, od_*, tdw, tg_*, truth1, pullback, mar_short, proj_*, ndx_vb_oops, xmas, yearend, july4, memorial), gold_nov_td16, gold_tdom1_4d, yen x2, wheat, cti, projected, 9ma (OCR-confirmed only), jan97_td16/td2, aug97_td2_sell_7d, dec97 p.4 variants, and sep97 rows 13-14.
Round 3 batch 2: yen_apr_td4 (Vol.34 No.3 p.3: full printout; live net -11.3%), gold_tdom1_4d (Vol.29 No.3 p.4: full printout; live net +13%), ho_mar_buy (rule text + Test 3 printout), tg_oc and tg_open3_bail (Vol.29 No.10 p.3). **Citation error found:** the tg_* card rows cite "letter Oct 1995 p.3", but Vol.29 No.10 is the November 1994 letter (text: "this year one would buy on the opening of Monday, the 20th"); figures equal the card. gold_nov_td16 / yen_nov_td10 printed figures confirmed in OCR text only.
Round 3 batch 3 (commit cf4c9f42): printed figures confirmed in the O1 OCR text of the cited page for 25 more cards: engulf x6 (Vol.25 No.2 p.3), july4 x2 (Vol.29 No.6 p.4), yearend_sell (Vol.30 No.12 p.3), jan_rally (Vol.31 No.1 p.3), jan97_td16/td2 (Vol.34 No.1 p.7), proj_contra / proj_limit (Vol.29 No.9 p.3 / p.2), mar_short (Vol.31 No.2 p.4), tdw_oc x4 (Vol.28 No.7 p.3), nov_td17_db7 (Vol.33 No.11 p.3-4), od_buy / od_buy3 / od_fade_buy (Vol.28 No.4 p.3), jul97_td14_sell (Vol.34 No.7 p.4), ndx_vb_oops (average only). Page images were not re-read for these.
Attempted and not located: Vol.29 No.11 p.3 (a Dec 1995 "Christmas comes early" sheet; it is not the xmas_bail source). Still without any re-read or OCR confirmation of their printed figures: keyrev_buy/sell, krut_buy/sell, gap_fail_300/525, memorial_bail/6, xmas_bail/9, pullback, truth1_closehigh, tg_open8, tg_close, od_sell, od_fade_sell, aug97_td2_sell_7d, jul97_td17_buy, jul97_td4_buy, jul97_td16_buy_30d, dec97 p.4 variants, cti_bonds (OCR-only earlier), sep97 rows 13-14 (about 22 cards). Their cited issues (1991-1994 letters) are either not in the share (April 1991: lead decision 197) or the OCR index does not carry the cited figures, so locating the page needs a manual page-by-page search of Vol.25-28 first pages.
Totals after round 3: 97 cards, printed figures re-read from a page image in V7 for 47 (tables 131 rows incl. 3 caveats, single printouts, Aug/Nov/Dec 1997, Oct S&P, Thanksgiving, yen/gold), confirmed in OCR text for 28 more, still open for ~22. No verdict changed; none verified.
tg_close re-read (Vol.27 No.11 p.3 JPEG): figures equal; citation corrected from Nov 1992 to Vol.27 No.11 (Nov 1993); window is 'since 1982', not 1982-1991. tg_* Oct 1995 citations corrected to Vol.29 No.10 (Nov 1994) in lwnl.rs/gate.rs (commit eb93c842).

## Round 4 (V9, 20 Nov 2026): the 21 remaining lwnl cards (+ sep97 rows 13-14), gate text only
Pages were found by name, not search: the issue date is printed on page 1 of each scan folder (folder names are unreliable: e.g. folder "Volume 29 - Issue 6" prints JUNE 1993 VOLUME 28 ISSUE 6). Pages read as JPEG images where marked "img"; otherwise the O1 OCR text (clear). Live side: every S&P card is admitted on cash SPX (data from 2014) or refused (&SP needs_data / not_testable held), so no printed 1982-1997 window overlaps any run: all "not compared (no data overlap)", UNVERIFIED. Commit follows; gate.rs text only (23 rows get a "V9 round 4" sentence).

| id | printed (page) | ours / card | verdict |
|---|---|---|---|
| keyrev_buy | Vol.28 No.10 (Oct 1993) p.3 img: S&P buys +13,325 46/59 77% (look-back 15) | card 59 tr 77% $226 ($13,325) equal; live SPX 2014+ 94 tr | printed equal; not compared. NOTE: the sells table prints the identical S&P row |
| keyrev_sell | same page, sells table: S&P +13,325 46/59 77% | equal; live refused (held) | printed equal; not compared |
| krut_buy | Vol.25 No.7 (Jul 1991) p.2-3 img: BUYS $35,325 32/53 60%, AVG $498 (prose: $666), DD 6,400; exit 6th close, $3,000 stop | card 53 tr 60% $666 equal ($35,325/53=666); table avg $498 is a misprint (same as the NIFE row) | printed equal; not compared |
| krut_sell | SELLS $49,825 42/68 61% AVG $732, DD 6,200; 3rd close, $2,000 stop | equal | printed equal; not compared |
| gap_fail_300 | Vol.25 No.1 (Jan 1991) p.3 img (card says p.2): 22/26 84% $35,000 AVTRD $1,346; 10/11 90% $35,850 AVTRD $3,259; exit "not fully disclosable" | equal; coded exit = close; live SPX 886 tr | printed equal; not compared |
| gap_fail_525 | same page: 18/18 100% $72,050, largest win $16,325 | equal ($4,003 avg) | printed equal; not compared |
| memorial_bail | Vol.29 No.5 (May 1995) p.4 OCR: 9 of 12, $8,953, avg $746 | equal | printed equal; not compared |
| memorial_bail6 | same: 12 of 12, $30,753 | equal | printed equal; not compared |
| xmas_bail | Vol.28 No.11 (Nov 1994) p.4 img (card says p.3): S&P 03/94 12 tr 100% $11,803 avg $983.58 | equal | printed equal; not compared |
| xmas_bail9 | same page: 12 of 12, $40,753 | equal | printed equal; not compared |
| tg_open8 | Vol.28 No.10 (Oct 1994) p.3 rule + p.4 img printout (card says p.3): 12 tr 91% (11 w) $25,325 avg $2,110.42 | equal; live SPX 12 tr 75% (2014+) | printed equal; not compared |
| pullback | Vol.28 No.5 (May 1993) p.2 rules, p.3 OCR printout (card says p.2): 70 tr 51% net $30,675 (GP 78,925 / GL 48,250) avg $438.21 | equal | printed equal; not compared |
| truth1_closehigh | Vol.27 No.6 (Jun 1992) p.2-3 OCR: S&P 500 6,675, 5 trades, exit 5, stop 500 | equal | printed equal; not compared |
| cti_bonds | OCR of folder "Volume 29 - Issue 6" p.3 (heading JUNE 1993, Vol.28 No.6): TREASURY BONDS 06/92 98 tr (41 won) 41% $17,756.25 avg $181.19 | equal; engine refuses (not_testable) | printed equal; **citation wrong: "Jun 1992 p.3" should be Vol.28 No.6 (Jun 1993) p.3** |
| od_sell | Apr 1991 (Vol.25 No.4) is not in the scan set | n/a | cannot be re-read; withheld |
| od_fade_sell | same | n/a (no results printed) | cannot be re-read; withheld |
| jul97_td4_buy | Vol.34 No.7 p.3 OCR: S&P day 4 20445 9/14 64% avg 1460 | equal | printed equal; not compared (held) |
| jul97_td17_buy | p.3 OCR: day 17 43025 8/13 61% avg 3309 | equal | printed equal; not compared (held) |
| jul97_td16_buy_30d | p.2 OCR: 30-day buys day 16 34525 7/13 53% avg 2655 | equal | printed equal; not compared |
| aug97_td2_sell_7d | Vol.34 No.8 p.4 img (card says p.3): S&P500 day 2 13845 7/14 50% avg 988 | equal | printed equal; not compared (held) |
| dec97_td2/td14/td9 | Vol.34 No.12 p.3 OCR and p.4 img: td2 8525 12/15 80% 568 and $11,175 14/15 93%; td14 11300 11/14 78% 807 and $14,250 13/14 92%; td9 6225 9/14 64% 444 (both) | equal to both tables | printed equal; the two tables disagree with each other; not compared |
| sep97_7day rows 13-14 | Vol.34 No.9 p.3 img: Beans sell day 15 17056 19/26 73% avg 656; BritPound sell day 3 23268 13/20 65% avg 1163 | card rows equal; engine row13 19 tr (vs 26) 79% $15,812; row14 16 tr (vs 20) 69% $18,152 (-22%) | printed equal; live rows fail (row 13 trades, row 14 net) |

Result: all 21 cards' recorded printed figures are confirmed against the page (13 on page images, 8 on clear OCR); none disagrees. No card can be promoted: the printed windows (1982-1995) have no overlap with the data the cards can run on. Citation corrections found (text in gate rows only, lwnl.rs untouched): cti_bonds Jun 1992 -> Jun 1993 (Vol.28 No.6); page numbers off by one for gap_fail (p.3), xmas (p.4), tg_open8 printout (p.4), pullback printout (p.3), aug97_td2 (p.4). No code error found (no rule differs from its page). Not re-read: od_* (issue absent), the OCR-only rows above on the image.
Totals: 97 cards: printed figures re-read from the page for all but od_sell/od_fade_sell/od_buy/od_buy3/od_fade_buy (April 1991 issue absent from the scans). No verdict changed; none verified.

## Round 5 (V9, 20 Nov 2026): help lane M2, books from the bottom of V8-list.tsv (claims written to V8-claims.tsv: wmmts, ststw, sglr, stwds). Engine guru-int-1120e via authors/heavy on port 19891 (PID recorded, killed by PID). Commit on claude/v7-1120.
First: lwnl citation fixes done (page numbers, cti_bonds Jun 1993) in lwnl.rs/gate.rs text only; `authors/heavy cargo check` ok (0 errors).

| id | printed (source page) | ours (live) | verdict |
|---|---|---|---|
| wmmts_market_turn | MarketTurnStrategy.pdf p.15-23, 3 examples: GOOG short 1/11/06 -> 1/23/06 +$64.07 (entry avg of open $471.27 and close $471.63 = 471.45, exit open $407.38); SBUX long 12/27/05 open $30.64 -> 2/9/06 +$4.46; DIA long 10/13/05 $102.03 -> 11/11/05 $106.56 +$4.53 | DIA +4.53 exact (DIA is now held; gate text said not held); SBUX +2.23 x2 split = +4.46 exact; GOOGL 1/11/06 471.27 -> default: NO exit on 1/23 (calc 0.35034 on 1/20), runs to 4/28 +53.33; with p.round=1 exits 1/23/06 407.38 = +63.89 (-0.3% vs 64.07, the gap is the averaged entry) | UNVERIFIED on the default; all three pass #368 only with the labelled option p.round=1. **Possible default question for the lead**: the book's '.35' is the 2-decimal value the spreadsheet shows; before (round 0): GOOG example fails; after (round 1): passes. Not changed. |
| ststw_vix_rsi | Ch.12 p.97: 92 trades, 79.35% correct, 879.46 SPX points, hold under five days | SPX 1995-2007: 97 closed, 82.47%, 817.41 pts (+5 trades, +3.1 pts, -7.1%); avg hold 3.9 bars | **PASS #368 -> VERIFIED** (gate promoted, lead may revert). Window is the chapter's 1995-2007 (not printed beside this section); the section's example is an undated chart |
| sglr_nasdaq_ma200_band | p.294 / Table 17-1: Nasdaq Composite 1990-2001 34 switches, 72.4% in, 17.52%/yr (incl. dividends, T-bills); 1972-2001 77 switches, 16.67%/yr; dated exit 9/11/2000, re-entry 12/5/2001 | Composite refused by the engine for this card; NDX 1990-04..2001: 24 trades = 48 switches, 73.3% in, CAGR 16.28% price-only; dates 2000-09-11 and 2001-12-05 equal; the 11/2/1998 entry does not | FAIL (switches); UNVERIFIED |
| stwds_obv30_75 | Table VI S&P futures 1984-96: 43 trades, 22/43, 3.00%/trade (as in the book field; paper page not re-read) | &SP refused; SPX volume starts 1991-05-28: 17 closed, 35.29% | not compared (data window); UNVERIFIED |
Not reached (context budget): wcdtf_ath_atr (24,000-stock universe), upg18_traders_dream (@SP 2001-02), tbabc_stampede_long (SUNW) and the rest of the list; they are unclaimed except the four above.

## Round 6 (V9): lwnl tg_close exit text made to match the entry (Vol. 27 No. 11 = Nov 1993). Claimed wcdtf, upg18, tbabc.
| id | printed | ours | verdict |
|---|---|---|---|
| tbabc_stampede_long | ABCs of Trading slide p.90 (image): 21 trades, 71.43%, net $24,180.40, avg $1,151.45, SUNW 1,500 bars to 08/31/00 | SUNW not in the vault | printed equal to card; not compared (instrument absent) |
| upg18_traders_dream | 90 trades, 45.56%, +$486.67 (as recorded; slide not located) | live card needs 1-min bars (2014+); old daily reading 89 tr, 50.56% | not compared (window); UNVERIFIED |
| wcdtf_ath_atr | 18,000+ trades, 49.3%, ~15.2% (as recorded; paper not in the share) | 24,000-stock universe absent | not compared; UNVERIFIED |
Remaining books in V8-list.tsv above these are unclaimed by V9; stopping here for context.

## Round 7 (V9): claimed rinadz, lydetf, mkfve, mrina. All four not compared (data/instrument); printed figures as recorded, source pages not re-read.
| id | printed | ours | verdict |
|---|---|---|---|
| mrina_xstoch14 | T-bond 1986-96: 37 tr, 84%, $203.38 | &ZB refused for this card (needs_data) | not compared |
| rinadz_rsi9_fixed | S&P 1970-96: 133 tr, 73.68% | SPX refused (opens); SPY 1993-96: 16 tr, 81.25% | not compared |
| lydetf_ema200 | S&P 1970-2008: 7.86%/yr, DD -18.8% | SPX 1983-2008: 90 tr, CAGR 8.20% price-only | not compared (window, dividends) |
| mkfve_div | XMSR 4 tr 100% | XMSR absent | not compared |
Stopping here (context budget); remaining books in V8-list.tsv unclaimed by V9.
