# Strategy settings and basket execution

The active React strategy page separates individual Stock selection from a true
Basket calculation. Stock selection follows the Index controls, retains the
source-qualified native key and does not require selecting a basket first.
Like, Share, PDF download, Save and Settings use accessible line-icon buttons.

Settings changes the individual-instrument Net Profit report only. The user’s
“Fixed quantity” label means additive closed-trade percentage returns; no share
quantity or monetary capital has been supplied. Compounding multiplies the
closed-trade net return factors. Both exclude open marks. Trade execution,
other statistics, the equity chart and source rules remain unchanged. Current
engine reports provide a full-precision net return alongside the legacy rounded
field. Exports capture the selected display and its formula. Missing or invalid
return factors remain unavailable rather than falling back to another formula.

Basket selection requests the separate authenticated `basket-run` endpoint. The
user selected at most 10, 20 or a custom 1–100 simultaneous positions, A–Z order
for competing entries, and equal initial-equity slots. Unused slots stay in cash.
The adapter replays unchanged, admitted independent-stock candidate ledgers in
regular mode: rejected entries are not deferred or retried. This is an explicit
portfolio adaptation, not reproduction of the author’s rankings or portfolio.
Source execution contracts that cannot establish cross-stock fill order remain
withheld; legs, costs, missing held-position quotes and failed members are never
silently dropped. There is no margin funding, short-proceeds reuse, capital
statistic or fabricated single-symbol price chart for a basket.

Basket Net Profit measures allocated closed contributions relative to initial
normalized equity. Avg net pnl remains the average source-unit stock-trade net
return. Daily equity and risk measure chronological cash plus simultaneous actual
position marks. Overlapping stock returns are not multiplied in closing order.
Gross accounts for the same accepted fills without costs. Constituent coverage,
allocation and method are included in the Excel export. Basket optimization and
report-only sequential compounding are not admitted as equivalent calculations.

Current/Past is a basket membership choice. A current snapshot has retrospective
survivor-selection limitations. Past results require a reviewed dated-membership
contract and the necessary former-stock quotes; static Current & Past unions
and unnamed Aux fields do not establish such a contract. The UI preserves source
information and controls when it withholds a result. Aux1/Aux2 are source-specific
arrays, not universal alive/dead flags.

The Nifty current snapshot was checked against the full official constituent CSV
retrieved on 2026-10-14. The older supplied list retained HEROMOTOCO, INDUSINDBK
and WIPRO; the matched current list instead includes INDIGO, MAXHEALTH and BSE.
This verifies the retrieved current list, not every historical membership date.
Private import receipts, original quote hashes, official CSV and validation
remain outside Git; original research and market-data shares are read-only.

Basket work is user-triggered, one request at a time per market engine, processes
stocks sequentially, and has bounded candidates, marks and a time budget. No
whole-corpus calculation runs at application startup. Validation covers React
settings and sanitization, actual Chromium navigation and exports, hand-computed
portfolio accounting, source-admission holds and existing Rust strategy parity.
Synthetic browser fixtures are UI contracts, not book-performance verification.
