# Strategy review status — 2026-10-13

The inventory and implementation catalogue are not a completed convertibility audit.

| Actual selected instrument | Families with closed-trade results | Variants with closed-trade results |
|---|---:|---:|
| Nifty50 cash index |15|35|
| SPY actual US ETF |22|49|
| RELIANCE actual cash stock |4|10|
| AAPL actual cash stock |4|10|

These are live full-history availability snapshots, not original-book study certification. Families and variants overlap across instruments; do not add the rows. Single-stock results do not establish a sized basket portfolio backtest. Parameter/direction variants are not independent methods.

The live catalogue has 41 enabled book-specific strategy families, containing 100 variants across 15 book/course entries. These are implementation/admission counts, not completed source certification. Its larger set includes reference methods awaiting source review and methods with missing inputs. The earlier roughly 700 converted claim is not a verified-strategy count. The final number of convertible methods in the research share is unknown.

Indian and US stock baskets and actual ETF series have already been supplied. Use stocks for stock methods and actual ETF prices for ETF methods; do not route them to a cash index just to obtain a result. For many daily OHLC methods the outstanding task is source-rule review and implementation, rather than obtaining those datasets again. Check each method's required fields, coverage and execution assumptions individually. Current/past constituent collections without verified membership dates cannot certify a point-in-time historical universe.

Completed bounded work includes VanTharp source-attribution/true-range EMA repairs, original research-library reconciliation, and independent selected Connors trade-ledger checks on actual Indian/US stocks. This does not certify every strategy in those books or every source file. The React migration does not add new verified strategies or lift instrument/data restrictions.

Next work: review actual source entries, record page-specific entry/exit/management/execution rules, distinguish complete rules from partial components, map each method to the correct instrument and available data, implement a bounded candidate, independently check its ledger, and only then expand admission. Psychology and sizing material remain readable without fabricated P&L.

The research inventory reports 41,895 nonempty direct document files, including 38,297 PDFs; these physical-file totals are not distinct-book counts. The bounded inventory review covered 29 candidate units; the historical 11,812 title-candidate list still contains 11,783 unreviewed entries. Newly recovered filenames remain content-review candidates. Full-corpus classification and conversion remain unfinished. The frontend's 35,712 metadata entries use a different denominator and do not establish a minimum number of convertible strategies.
