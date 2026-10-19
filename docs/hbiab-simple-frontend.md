# SimpleSystem frontend reporting contract

The fourth reviewed candidate ID is `hbiab_simple_ma25`, book `hbiab`. It uses the existing `bandy_fixed_fraction_fractional_cash_v1` account schema. This frontend addition does not admit a backend strategy, reproduce book performance, or certify native AmiBroker parity.

The source is the SimpleSystem.afl interpretation/commentary teaching example on printed pages 527–528, illustrated with actual SPY. Its raw Buy is strict `Close > MA(Close, 25)`; every true raw Buy resets the exit clock, including while already long. Sell is five observed bars since the latest raw Buy. This is not a five-bar holding period measured from entry.

Daily finalized same-close fills and a 100% fractional current-equity target are declared research conventions. Entry costs reserve cash, so actual deployed notional can be less than the target. The selected account starts flat, indicator history retains the supplied prefix, and a terminal entry may remain open. Open marks pay actual entry costs only; no next-open pending order or invented exit is allowed. The existing cash, quantity, gross/net and reporting conservation checks remain active.

## Public presenter admission

The new method requires matching outer ID/book, initial model unit 1, its exact method-specific account/accounting basis, and an explicit client context containing the catalogue definition, effective parameters and selected instrument. The catalogue must identify the same method, the distinct `bandy_simple_raw_buy_clock_fractional_cash_v1` convention and a single-symbol stock/equity-ETF account. Effective settings must be exactly MA25/hold5. An omitted query uses schema defaults before validation; missing schema/context cannot acquire review.

Native stock selection retains the selected ticker row unchanged and requires its stock metadata. Older runtime ETF rows do not report an asset type; their classification is explicitly based on eligibility in the existing ETF selector. The reporting parser has no independent positive ETF ticker whitelist. An unknown selection outside the actual selector cannot produce this context. Known cash-index identities are rejected as an additional defense. Present selected/body/trade symbols, qualified identities, asset types and provenance hashes must agree.

The older three Bandy methods and protected Elder account do not inherit these new context requirements. Their reporting predicates, wire bodies and model-specific labels remain separate. Unknown or inconsistent Simple responses are withheld, with no generic unit-exposure fallback. Chart tabs regroup the existing presenters; the account notice stays outside those tabs, and display-only closed compounding cannot resize or rewrite the accepted ledger.

## Offline adapter boundary

`parseBandyAdapterContract` is a separate low-level entry point for explicitly bound offline reporting fixtures. Its context records the case and hashes of the input, request and captured body. It can validate an independently labelled synthetic fixture or an adapter body that deliberately omits public API provenance. It never adds provenance to that body.

The public `sourceAccount` path always uses the strict parser and cannot enable this adapter mode. Synthetic context must fail there. Passing recorded adapter bodies is reporting-contract evidence, not actual API/readiness/instrument admission or publication approval. Normal-server metadata, routes, browser rendering and real exports require their own acceptance before release.
