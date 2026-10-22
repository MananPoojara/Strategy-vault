# Bandy SimpleSystem commentary example

This review covers only `hbiab_simple_ma25`, from Howard B. Bandy,
*Introduction to AmiBroker* (Blue Owl Press, 2008), printed pages 527–528.
It extends the separately reviewed three-example batch without changing its
source rules, accounting conventions or labels. Other book entries remain held.

The displayed SimpleSystem.afl teaches interpretation-window commentary. Its
chart uses actual SPY. It is not a printed trading-performance study. The exact
signal arrays are `MA25 = MA(C,25)`, `Buy = Close > MA25`, and
`Sell = BarsSince(Buy) >= 5`.

Buy is a repeated raw state. Every true bar resets the exit clock, including
while a position is held. The exit occurs after five observed bars without a
new raw Buy, rather than five bars after entry. Equality with MA25 is false.
The reconstruction requires 25 genuine observations for MA25 and claims no
native BarsSince sentinel before the first true Buy. Before that first Buy no
long exit is actionable. It uses the exact shortest round-trip decimal values
of the loaded Rust prices to compare the current close with the MA sum; it
does not claim identity with undisclosed original vendor decimal tokens.

The displayed formula supplies no explicit fill, sizing, timeframe or funded
capital settings. This daily research application declares one long position,
no pyramiding, an idealized finalized signal-close fill with zero delay, and a
100% fractional current-equity target. The normalized account starts at 1 and
reserves entry fees and slippage, so actual deployed allocation can be smaller
than the target. This is not certified broker execution or native AmiBroker
binary parity. No book return, funded capital or profitability is claimed.

All supplied genuine prior history drives the signal state. The selected
account starts flat and does not inherit an earlier position. A true Buy on
the selected first bar may enter even if the raw state was already true before
the window. A final-close Buy is retained as an open position. Open positions
are marked at the last actual close and pay only actual entry costs;
hypothetical liquidation cost is disclosed separately. Closed trades charge
actual entry and exit notionals. Gross equity retains the accepted quantities
and removes actual paid costs rather than resizing a second zero-cost run.

Actual ordinary equity ETFs include the source-shown SPY. Actual individual
stocks and other ordinary equity ETFs are declared research applications.
Cash indices, baskets treated as independent symbol totals, derivatives and
synthetic ETF prices are not substitutes. There is no absolute USD threshold
or tick rule in this signal formula. Quotes, adjustment history and local
data provenance remain distinct from source-rule fidelity.

Configuration is fixed at MA25 and five bars since the latest raw Buy.
Unknown or changed parameters and non-singleton grids do not inherit review.
Rollover is unsupported for these cash instruments. Exposure counts observed
sessions inclusively from entry through exit; drawdown duration counts observed
sessions. Trade-sheet Days Held counts calendar date differences. Exactly zero
closed returns count as non-wins for loss and streak reporting.

The candidate requires independently frozen actual-stock and ETF ledger,
reporting, API and presentation acceptance before publication. Source signal
review and synthetic checks alone are not financial acceptance.
