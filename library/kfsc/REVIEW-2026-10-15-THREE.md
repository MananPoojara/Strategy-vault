# Short Course: three selected stock configurations

This is a bounded research contract for three existing method IDs, across two existing family keys. It is not a whole-book review, a new algorithm count, a reproduction of the author's charts or performance, or approval of other Kaufman methods. Candidate host integration requires a different review before publication.

Source: Perry J. Kaufman, *A Short Course in Technical Trading* (Wiley, 2003), supplied 339-page PDF, SHA256 `11dfd5d543d7bd0ab7c43a2e2f4a3593a7be9eee1c9a2fae124f6042601d857e`. Printed pages are PDF page minus13. Prior separate private source/ledger review is identified by receipt SHA256 `742e75ac54f6c1da89405e6d9a9d2476ee5931086057abed8497fbc88b7c3e5f`; its broader eleven-method approval is not inherited here.

| Exact method | Selected source configuration | Source pages, printed (PDF) |
|---|---|---|
| `kfsc_two_ma` | SMA80/SMA10 slope agreement, long/cash |52,70,123–124 (65,83,136–137)|
| `kfsc_pct_band` | SMA20 center, upper×1.05, lower÷1.05 |97–98,311 (110–111,324)|
| `kfsc_bollinger` | SMA20 center ±2 sample standard deviations of20 closing price levels |99–102,311 (112–115,324)|

The SMA definition and slope direction are on printed52. Two-MA entry requires both full moving averages to rise; either falling average exits, even if the slow one falls first. Equality holds. Periods80/10 are the Amazon example, not universally required periods. This is slope agreement, not a fast/slow crossover.

Percentage-band entry requires a strict outer-band close penetration: long below MA/1.05, short above MA×1.05. A long exits at close≥MA, a short at close≤MA. The explicitly printed divided lower band is selected instead of the alternative MA×0.95 mentioned on the same page.

Bollinger uses the literal Excel `STDEV` formula on closing **price levels**, with sample divisor n−1. Nearby prose says closing price changes; this conflict remains disclosed. This candidate chooses the printed price-level formula, without claiming the prose is corrected. The200-day example and a changes-based formula are separate held configurations.

Both band methods apply the chapter's countertrend sell/buy symmetry, confirmed by review answer25 on printed311. Short positions are a declared hypothetical interpretation of the sell signal; source material does not establish actual borrow, margin, financing or execution availability. An opposite-band close first exits the existing position at the center condition, then may reverse at the same close. This order, strict penetration and center-equality handling are explicit mechanical conventions. No pair-trading or simultaneous long/short portfolio is inferred.

The fixed `source_convention` metadata and review notice identify the selected research convention; there is no unused numeric convention control and it is not called a book default. The author recommends preclose calculations and close orders on printed70. Using a final completed daily close to decide an order filled at that same close is an idealized research approximation. Actual auction feasibility is not certified. New terminal-close orders are filled under that convention and retained as open holdings, with platform estimated liquidation costs kept separate from filled exit orders. Other KFSC runners retain their earlier terminal behavior.

These are selected standalone chapter rule applications. Chapter8 printed119–120 explicitly discusses whether natural method exits provide adequate risk control and the conflict between an added stop and trend following; printed123–124 provides the two-trend alternative. The discretionary support/trendline stop process is not silently converted into an extra mandatory mechanical stop or represented as reproduced here. No claim is made to reproduce the book's whole trading plan, position sizing or risk program.

Only actual individual-stock prices are eligible. AAPL and RELIANCE are selected compatible research instruments, not the original AMR/Amazon study or a historical universe. No index/ETF substitution or basket portfolio admission is inherited. Supplied adjustment, historical membership, dividend, borrowing and quote-authenticity limitations remain explicit. No monetary/capital statistics are inferred.

All three IDs accept only their exact selected parameters; the fixed source convention is metadata, not a configurable parameter; nonfinite, malformed, unknown, changed or rounded-near-default parameters fail closed. Optimization can evaluate only an unchanged singleton. Full actual historical warmup remains loaded. Current/previous SMA80 requires at least80 preceding observations; the20-period bands require at least19. No earlier prices are generated, discarded or reseeded to force availability.

The earlier eleven-method review reported42 compatible cases,5,527 closed records and31 open marks. These three default configurations alone covered six AAPL/RELIANCE cases,632 closed records and four open marks over2010-01-04 through2020-09-11. Those are prior source-ledger checks, not a claim that the present financial/API integration is independently accepted. Its previous publisher stopped with an LLVM allocation failure during compilation, not a numerical test assertion. Current source, compiled output, input hashes, costs, open marks, adverse results, guards and regression evidence must bind to the final candidate and be checked by another reviewer.
