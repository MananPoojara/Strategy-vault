# Short Course: literal MACD and double-smoothed momentum research candidates

This document records a bounded source/implementation candidate. It does not claim public deployment, independent approval or whole-book completion. Exact future source admission and release require a different reviewer and root integration. The earlier three configurations' accepted interpretation remains unchanged.

The source is Perry J. Kaufman, *A Short Course in Technical Trading*, Wiley2003, private339-page PDF SHA256`11dfd5d543d7bd0ab7c43a2e2f4a3593a7be9eee1c9a2fae124f6042601d857e`. Physical pages are printed+13. Direct text and actual figure images were read before changing calculations; the old catalogue/overnight eleven-method work was a locator, not proof.

Exactly two existing IDs are proposed: `kfsc_macd` and `kfsc_dsm`. These are configurations already grouped under the existing momentum entry, not newly discovered algorithms. Other Short Course methods gain no admission. `kfsc_stoch` remains a reading/timing component because its recommended direction selection requires an undefined trend/sideways regime.

## Literal MACD

Printed203–205 defines the fast-smoothed close minus the slow-smoothed close, then a smoothing of that difference. Step1 says2/(N+1), but steps2/3/5 explicitly print slow.0243, fast.0476, signal.10. These also are NOT exact1/41 or1/21. This candidate chooses those literal three decimals; it does not silently recompute either reciprocal formula or claim the conflict is resolved. The20/40/9 example labels do not override the selected constants. A strict upward MACD/signal crossing buys; a strict downward crossing sells; the immediately previous equality can qualify a crossing. Repeated same-direction signals add no position.

First actual source close seeds both price smoothings, so their initial difference and signal are zero. This seed is an explicit computational convention because the chapter supplies no initializer. All supplied earlier source rows are used, without reseeding at the selected test start. A selected flat start waits for the next actual crossing. The fitted±2price thresholds, discretionary resistance and divergence filters discussed on205 are not added or claimed.

## Literal double-smoothed10-day momentum

Printed231–232 explicitly selects10-day momentum for the smoother figure15.14 after discussing a different1-day alternative. This candidate uses close[t]−close[t−10], then two literal.0476 recursive smoothings. Both passes begin at the first actual10-day momentum as a declared seed. Printed201 examples support current-minus-prior; printed202's generic first-difference sentence reverses the wording, which is disclosed rather than inherited.

The figure says buy when the indicator turns up and sell when it turns down. A turn here changes the previous nonzero slope direction. Equality retains the last nonzero direction; the first observable slope only establishes initial direction. This operational tie/initialization convention is explicit. A selected flat start waits for a subsequent genuine turn; it does not enter on an arbitrary already-rising/declining first selected bar. All earlier actual history is retained.

## Execution, accounting and data scope

General printed57–58 supplies the recursive smoothing formula. Printed70 discusses long/short trend positions and recommends calculating just before close. A backtest using the completed daily close to decide and filling at that same close is therefore idealized; it is not proof of executable preclose orders. One position, opposite-signal same-close close/reversal, flat selected start and equality handling are declared operational conventions. A genuine selected-final-close signal may open a position; it stays open, with separately labelled estimated liquidation cost, rather than being silently suppressed or fabricated as a filled exit.

Only actual selected stocks AAPL/RELIANCE are proposed. Full supplied earlier observations remain available. The positive verification request begins2010, with AAPL ending2020-09-11 and RELIANCE2026-10-01. At least a prior seeded MACD state and, for DSM, genuine10-day momentum plus two observable slope comparisons are required. Missing/invalid prior observations are held rather than discarded, filled or reseeded. Source quote accuracy, corporate adjustments and historical membership remain qualified.

Short stock exposure is hypothetical without certified locate/borrow, margin or financing. This is a sequential normalized price-return study, not funded capital, basket sizing, original source population or chart-performance reproduction. Natural opposite-signal exits do not reproduce the broader discretionary combined-method programme or printed232–233 game portfolio, money or risk limits. Additive closed-return sums are distinct from marked compound-equity estimates. Slippage is per side; open final liquidation is an estimate. Losing outcomes are retained.

## Stochastic hold

Printed216–218 says the range excludes today; the actual Nokia spreadsheet on217 clearly uses a10-observation window including today. Printed219 gives threshold/crossing timing;220 specifies the first crossing after an extreme, requiring a precise excursion/re-arm choice. More materially,222–223 permits both directions in sideways conditions and selects direction in trends, without an objective regime classifier. Dropping that requirement would substitute a different unfiltered strategy. The oscillator/table can be explained as reading material; no verified strategy/backtest admission follows. Legacy illustrative execution code is not used as proof and remains held.

Private source pages, fresh chronological reference, full indicator/order/trade/equity comparisons, adverse/prefix/cost tests and exact binary/input manifests remain outside Git. A catalogue entry alone is not verification. This candidate's source/API evidence and separate review must be bound before any live release.
