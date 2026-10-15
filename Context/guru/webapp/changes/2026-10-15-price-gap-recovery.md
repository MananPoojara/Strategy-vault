# Missing-price repair and explicit period recovery

A basket needs every required constituent quotation. The supplied Indian source
omitted 16 December 2025 across its current Nifty stock files. Actual NSE cash
bhavcopy observations were downloaded and checked against both adjacent source
sessions. The repair preserves original data, raw exchange files, hashes and
per-stock source receipts outside Git. Later split/bonus adjustments are stated
separately where needed to preserve the existing quote basis. Unknown auxiliary
membership and open-interest values are left unknown.

The same official session was reconciled with the other supplied Indian stock
sources. The first promoted snapshot repairs 632 distinct cache files covering
486 stock symbols; separate copies used by different baskets are counted as
files, not additional stocks. Eighteen files failed identity, quote-basis or
corporate-action qualification and remain unchanged. Existing same-company
history from the broader source also restores the previously omitted BEL and
TRENT prefixes without replacing any existing Nifty-source observation.

The runtime dataset is a new read-only source snapshot. Composite source receipts
retain original native-file identities and counts, the official supplement, and
any restored same-company price prefix. This does not certify every historical
price, adjustment or membership date. No strategy calculation changes are part
of the repair.

Older history also contains genuine periods without quotations. An inspected
official May 2009 exchange file omits the affected Bajaj securities; L&T's May–June
2004 suspension is separately documented. Such dates are not invented as candles.
When a selected Current basket period has unavailable prices, the page can offer
an explicitly chosen covered price window. The existing dates remain selected
until the user clicks **Run available price period**. Metadata binds the window
to the staged source-index and benchmark hashes; a price window cannot clear a
Past membership hold or promise source-rule readiness.

Validation includes missing/invalid metadata, date intersection, preserving
requests, synthetic browser recovery and an actual selected-stock basket run.
Private quote and result evidence remains outside the public repository.

The live Current Nifty basket was also checked against an independent daily cash,
position and actual-price ledger. An authenticated LAN browser confirmed that the
original requested dates remain selected until recovery is clicked, that the
covered result loads, and that Past still withholds performance without the
recovery action. The US engine, authentication, database and proxy containers were
not recreated. Historical temporary constituents and their actual listing quotes
are separate evidence; fetching them alone does not admit a Past backtest.
