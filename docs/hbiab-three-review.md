# Three examples from Introduction to AmiBroker

Howard B. Bandy, Blue Owl Press, 2008, ISBN 9780979183812. This bounded review covers only `hbiab_ma5_20`, `hbiab_ma7_4`, and `hbiab_macd_example`. Other registered methods remain outside this review and retain `source_review_required`. Registration does not establish source verification or publication readiness.

The source PDF has 608 physical pages; printed page p maps to physical PDF page p+17. SHA256: `5c2493176135fd1cb19159e5bca4af1632b12e413ecc2c777c925fe907e8ff0c`. The one-page first-printing errata dated November 10, 2008 has SHA256 `b41f8c6b8045ee7dc610ea965eef96835b3472ed55f4e929312ba6e11e1a19a3`. Its printed92 V-to-VV correction concerns a reserved Volume identifier in optimization explanation; it does not change these three signals. Printed568 corrects a word. Source books, images and private numerical ledgers remain outside Git.

## Printed rules and declared conventions

| ID | Printed evidence | Ordered signals and fills | Sizing |
| --- | --- | --- | --- |
| `hbiab_ma5_20` | 78,85,502–503 | Buy when SMA5 crosses above SMA20; sell inverse. Later Example6X explicitly says long, daily, current symbol, Close with delay0. | The formula omits allocation. A 100% target fraction is a declared convention; entry fees reserve cash. |
| `hbiab_ma7_4` | 93–97 | Buy when SMA7 crosses above SMA4; sell inverse. Never reorder the periods. Same-close delay0 is a declared convention borrowed from the later MA example. | Declared 100% target fraction, with entry-fee cash reserve. |
| `hbiab_macd_example` | 224–225,308,317 | MACD12/26 crosses above signal9 to buy; inverse to sell. Source explicitly delays one bar and fills at next actual Open. | Source explicitly uses 25% of current equity per position. |

Printed95–96 chooses the ordered7/4 parameters from an in-sample CAT optimization. Its historical search spans odd1..31 and even2..30, but this admission covers only the chosen fixed7/4 example. The source extends the test end into saved data; no independently restarted out-of-sample performance is invented. Original CAT illustrations and capital figures are book-reported examples, not numerical verification targets for different adjusted data.

Printed287 defines MA as a simple periods average. EMA seeding is not specified. The selected computational convention seeds each price EMA with its full-period SMA, then uses alpha=2/(N+1). MACD first becomes valid at source index25. Signal9 seeds from the first nine valid MACD observations, indices25..33. A crossing needs current and previous valid pairs, making index34 the first eligible MACD cross. This is an explicit reconstruction, not native AmiBroker binary parity.

Cross(a,b) requires a[t]>b[t] and a[t-1]<=b[t-1]; current equality is not a cross. SMA needs full observed history without zero padding. Calculations retain all supplied actual preceding rows. The selected account starts flat with normalized equity1, waits for a selected-window crossing, and does not inherit a pre-window pending order. An all-history run may begin at source index0 and remain flat through unavailable indicator warmup. No missing historical observations or future bars are fabricated.

Same-close fills are idealized historical research. MACD fills use the next supplied actual bar's Open. There is one long position and no pyramiding, source-unprinted stops or ranking overlay. A terminal same-close MA entry is retained as open. A terminal MACD close signal can queue an unfilled next-open order, separately marked as buy or sell. An open holding is marked at final Close and pays no invented exit fee. It is excluded from closed-trade counts, averages and closed-return sums.

## Instrument scope

CAT is an actual cash-stock illustration. Printed502 reopens Example6B and printed503–504 applies that5/20 example to XLU, an actual ETF. Preface printed xv–xvi explicitly uses stock/symbol/issue/ticker generically for tradables including ETFs. Thus actual ordinary equity ETFs are compatible with these indicator rules:5/20 has direct ETF illustration; fixed7/4 and MACD on a selected equity ETF are declared applications. Selected AAPL, RELIANCE, CAT and real SPY observations are independent numerical applications, not original book performance or historical-universe reproduction.

Admission is exact-book plus exact-ID and accepts actual individual stocks or admitted ordinary equity ETFs. A cash-index level cannot substitute. Basket/portfolio selection does not inherit this single-symbol account. The separate Example7A/B shared-cash watchlist portfolio and adjacent rotational example are outside scope. No option, futures, short borrowing, statutory taxes, dividends or original broker capital are supplied. Existing local SPY data predates this conversion; no new research-share provenance is claimed.

## Account and report contract

`books/hbiab_reviewed.rs` owns states and the `bandy_fixed_fraction_fractional_cash_v1` model. `Run.bandy_account` is separate from the existing Elder account, and its report serializer applies only to these three exact IDs.

At entry, target notional is f times current net equity. Actual fractional quantity is the lesser of target notional/price and free cash/[price*(1+side cost)]. For MA f=1 this reserves entry costs; for MACD f=.25 the source25% target is preserved. Paid entry and exit fees/slippage use their respective actual notionals. Cash plus marked holdings equals net equity. Gross equity removes actual paid costs from the same accepted quantities and fills; it never reruns sizing at zero costs.

Per-trade audits distinguish entry/exit signal dates, fill/mark phases, entry equity, fractional quantity, actual entry/exit notionals, actual costs, gross/net account contributions and normalized entry-equity fractions. Open exit notional and exit signal date are null, and paid exit cost is zero. The hypothetical open liquidation estimate is separate from paid costs, equity and statistics. Money/tax/excursion fields remain honestly unavailable rather than showing normalized units as currency.

New account/day/trade values, outer prices/weights/equity and `ret_full_precision` serialize round-trip f64. The general stats interface retains six-decimal serialization. Fees, slippage and total-side bps also retain round-trip values for this model. Nonzero rollover remains unsupported without actual contracts and roll evidence.

Source parameter values are strict, including raw decimal text before float rounding/underflow. Unknown keys, changed defaults, malformed/nonfinite values and non-singleton grids are rejected. Equivalent exact scientific notation is allowed. The optimizer evaluates the one reviewed configuration only; this does not repeat the source's historical optimization or give arbitrary settings inherited verification.

## Verification boundaries

Tests cover ordered signals, full-period seeds, strict ties, all-history warmup, terminal entries/pending orders, next-open signal metadata,25% current-equity allocation, cash conservation, actual exit-notional fees, open entry-only costs, unknown settings and exact instrument scope. A cfg(test) environment-driven adapter invokes the production backtest and serializer on independently supplied private CSV cases; it does not add a runtime endpoint or embed data.

Independent frozen references cover actual stocks and SPY, full preceding history, zero and explicit modeled costs, source-date and extended windows, partial-history/prefix and terminal-boundary cases. Publication requires the final source/binary binding, independent account/indicator/report comparisons and appropriate API/browser integration review. No whole-book completion or certification of unread sections is claimed.

Reporting retains the platform conventions: exposure counts observed sessions inclusively from entry through exit, rather than intraday duration. `max_dd_days` counts observed sessions, not calendar days. A zero-return closed trade counts among non-wins for loss and streak reporting. These conventions do not change source signals or account transactions.

Arithmetic qualification: prices and individual fee/slippage bps are interpreted as the finite decimals represented by their shortest round-trip loaded Rust f64 values. This does not certify the original vendor text's discarded parsing precision. SMA crossing compares exact decimal ordered means; floating SMA states are diagnostic display values. EMA full-period seeds use an anchored mean and the recurrence prev+alpha*(value-prev), preserving a constant series. Exact net price/cost expressions include accepted quantity before conversion, so exact break-even remains zero and intermediate underflow does not erase a representable account loss. Fee and slippage bps remain separate until the exact rate sum. No epsilon, instrument exception or fitted signal tolerance is used. Cash/NAV and same-quantity gross marks accumulate trade contributions, keeping a no-movement gross account flat. Unsupported fractional-quantity arithmetic returns an explicit calculation error. A genuinely covered one-session account is supported for these exact three methods.
