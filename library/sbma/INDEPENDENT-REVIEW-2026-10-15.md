# Moving Averages 101: two reviewed stock methods

Steve and Holly Burns, *Moving Averages 101* (2015). This review admits exactly `sbma_sma50_long` and `sbma_x_5s20s` at their source defaults as selected-stock research applications. The original Monster-stock population, its selection process, source portfolio and book performance are not reproduced. No numerical backtest table or trade ledger is provided for these methods in the book. Other Burns methods remain outside this review.

## Source contracts

Physical PDF page 10 defines the simple average of completed daily closes. Page 42 describes staying long above the 50-day SMA and exiting below for selected strong stocks. Here, a flat test start above that average may enter without a fresh crossover; equality holds the current state. The stock-selection judgement is not replaced by an invented filter.

Pages 58, 60 and 67 describe the shorter/longer moving-average crossover and list the 5-day SMA / 20-day SMA pair. The reviewed configuration enters long when the fast average is strictly above the slow one after being at or below it; it exits when strictly below after being at or above it. Equality alone does not trade. The reviewed subset is long/cash, without short reversals. Optional RSI, MACD and discretionary extensions are excluded.

Pages 16 and 63 support completed-close decisions and next-day use. `fill=0` remains the platform default but is explicitly an idealized same-close research assumption, not a demonstrated executable closing-price order or a mandatory book default. `fill=1` executes the completed-close signal at the next actual observed open. A terminal next-open order remains unfilled; it is never replaced by a fabricated quote or forced close. Full actual pre-period prices warm the SMAs, while each test starts flat with one sequential position.

The periods are fixed to 50 SMA or 5/20 SMA and long/cash in this bounded admission. Changed lengths, short/always-in settings and fill values other than exactly 0 or 1 do not inherit the review. These relative-price methods have no selected US-dollar price threshold. Indian stocks are a declared market application. Actual individual-stock prices are required: cash indices and ETFs are not substitutes. Current stock snapshots do not prove unbiased historical population membership. Vendor quotes and corporate-action adjustments remain qualified by their data provenance.

## Independent evidence and integration accounting

A prior independent source-based Decimal reference and separately compiled Rust candidate checked the full supplied 2010–2020 AAPL and RELIANCE windows at both fill modes and at 0/5 bps per side: 16 complete cases, 43,376 daily comparison rows, 2,752 order rows and 1,380 trade/mark rows (1,370 closed and 10 open). Repeated configurations are comparison records, not unique trades or additional methods. Actual selected sessions are 2010-01-04 to 2020-09-11 for AAPL and 2010-01-04 to 2020-12-31 for RELIANCE. A current API request explicitly beyond native source coverage is held rather than silently shortened. Four actual-data boundary prefixes were checked separately and are excluded from these totals. Prices were never forward-filled or selected by profitability. All losing trades remain included.

That independent reference charges one entry cost on a terminal open mark. The existing platform instead includes estimated liquidation costs in the final open trade return and final equity mark; this integration preserves that shared convention. Thus the earlier proof binds the unchanged signals, dates, prices and closed-trade arithmetic, while final runtime equity/open estimates require a separate integration compatibility check. It is not correct to claim that all platform open-return values were independently certified by the earlier reference. Estimated liquidation costs are distinct from an actual filled exit order, and no capital amount or basket allocation is implied.

Closed Net Profit sums closed-trade net returns, while the separate chart compounds the sequential normalized equity model. Closed trades incur entry plus exit costs. Slippage is per side; these cash-stock methods have no demonstrated futures rollover events. Gross results use the same fills at zero cost.

## Provenance and limits

Source PDF SHA-256: `5d8fc8a50825df615f3285131ecb30f644609d771fbf90252caa7338a1388d40`.

Original backend SMA/state-machine module SHA-256 before integration: `643554e663ac429767b8fa7233f977084ed74fd19fd1c15fff0920556734dde8`. The reviewed 5/20 pair is registered in that existing runner; its indicator/state-machine calculations and shared accounting are unchanged.

The private independent-review package retains exact source/data hashes, chronological ledgers, commands, comparison tolerances and original candidate source. Raw books, quotes and generated results are excluded from Git. The integration batch separately records its current-base commit, source hashes, focused Rust tests, actual-stock API cases, rejection cases and comparison results. Its author does not independently approve their own integration work; publisher review remains a separate step.
