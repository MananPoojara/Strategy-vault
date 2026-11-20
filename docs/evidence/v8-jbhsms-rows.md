# Bernstein, Hot Stock Market Strategies - Figures 7.12-7.18: every printed trade row compared (lane V8, 20 Nov 2026)

Printed rows: OCR (authors/ocr2) of the scan, PDF pages 144-152 (printed p.121-128); all 189 rows of Figures 7.11-7.18 parsed (V8w/printed-jbhsms-figures.tsv). Ours: live release int-1120d private engine, same ids, windows = the printed entry years, tickers basket:us_dow30:KO / MSFT / PG, basket:us_sp500:GE, basket:us_nasdaq100:TXN (V8w/rowcompare-jbhsms.tsv: one line per printed row with printed and our entry/exit dates, % result, prices and price ratio).
Per row: entry date, exit date, sign of the result, % result.
| card | printed rows | entry date same | exit date same | sign same | rows with % differing by > 1.5 points |
|---|---|---|---|---|---|
| jbhsms_stk_ko (Fig 7.12) | 35 | 35 | 35 | 35 | 0 |
| jbhsms_stk_ge (Fig 7.13) | 35 | 35 | 35 | 35 | 1: 1983 printed +5.40% vs ours +3.65% |
| jbhsms_stk_msft (Fig 7.14) | 18 | 18 | 18 | 18 | 2: 1986 +177.77% vs +163.07%, 1988 +20.58% vs +18.50% |
| jbhsms_stk_pg (Fig 7.16) | 34 | 34 | 34 | 34 | 0 |
| jbhsms_stk_txn (Fig 7.18) | 34 | 34 | 34 | 34 | 1: 1974 +59.42% vs +62.57% |
All 156 printed rows have the same entry date, the same exit date (including GE's stop exit on 3 Jun 2002) and the same win/loss result. The four rows above differ by more than 1.5 points in the % result; the printed prices are rounded to cents on low-priced early quotes and Norgate's series is split-adjusted (KO's price ratio printed/ours is 1.93-2.00, the 2-for-1 split plus rounding), so these are data differences, not rule differences. No printed row was left uncompared. Alcoa (Fig 7.11, 33 rows) has no held series and was not compared (stays UNVERIFIED).

# IBM gap trade (Tables 6.1 / 6.2), trace of a possible code error
The book prints summary reports only (no trade list): V8w/printed-jbhsms-gap-tables.txt. The card's parameter mapping matches the Table 6.1 caption (gap 20c above the prior high / 10c below the prior low; penetration 35c into the high / 50c into the low; 3.9% stop; first profitable opening). Independent pandas reading (V8w/gapref.py) and variants, printed 119 trades / 84.0% / $442 (Table 6.1) and 56 / 92.9% / $741 (Table 6.2):
| reading | Table 6.1 trades / win / avg | Table 6.2 trades / win / avg |
|---|---|---|
| as coded (engine) | 113 closed + 1 open / 78.8% / $228 | 45 / 77.8% / $307 |
| as coded (pandas) | 115 / 74.8% / $178 | 45 / 77.8% / $310 |
| re-entry allowed on the exit day | 125 / 75.2% / $158 | 47 / 76.6% / $267 |
| gap measured from the prior close | 347 / 80.4% / $420 | 224 / 83.0% / $531 |
| stop on the close | 106 / 83.0% / $225 | 44 / 86.4% / $425 |
| no stop | 38 / 94.7% / $786 | 31 / 100% / $739 |
| exit on the entry-day close if profitable | 118 / 80.5% / $164 | 45 / 82.2% / $264 |
No reading reproduces the printed count and win rate together; the card as coded is the page's own text. Live behaviour unchanged. Candidates not testable here: TradeStation's own order handling (stop/limit fill rules), the book's data vendor prices.
