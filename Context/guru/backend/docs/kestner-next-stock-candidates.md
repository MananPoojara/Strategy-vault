# Kestner next stock conversion candidates

These are unadmitted source-conversion candidates. Registration, synthetic fixtures and this note do not establish measured availability, numerical acceptance or original book performance. The earlier channel40/20, SMA10/40 and momentum80 batch has its own contract; its accepted behavior and admission guards are separate.

## Source and scope

Lars Kestner, Quantitative Trading Strategies (2003), source PDF SHA-256 `168ff6a95da48f83bb8dcd5d8e00dc3f5e5e9895fdeae1e95dae0e845abd705b`. Printed body page plus22 identifies the one-based physical PDF page. Printed41 defines population variance; printed57 gives the general EMA coefficient; printed66 defines RSI through average up/down moves; printed68 gives MACD-specific coefficients; printed139 establishes close signals and next-session-open execution. The concrete strategy rules are on printed155 (volatility breakout),165/170 (RSI) and170/175 (MACD).

The intended next validation uses actual selected US and Indian individual stocks. A stock, cash index, ETF and futures contract retain distinct instrument identities. No stock universe or derivative can be supplied by an index substitution. Source stock/index tables and dollar-volatility position sizing are separate from selected-stock normalized percentage research. Present constituent membership, quote adjustments, borrow, financing, margin, dividends and original study performance are not certified here.

The existing three IDs below remain outside the admitted stock-ID list. The new literal-alpha MACD ID is explicitly `NotTestable` while independent acceptance is pending. No policy, API, readiness, gateway or accuracy-gate permission is changed by this candidate. Its direct runner supports bounded independent verification; that is not public market admission.

## Fixed configurations

| ID | Exact settings | Source rule and declared indicator convention |
| --- | --- | --- |
| `lkqts_volbo` | `sd_n=100`, `k=2` | Close-to-close price-unit changes, current-inclusive100-change population variance, strict positive/negative2-sigma entry. Window indexing is declared. |
| `lkqts_rsi14` | `n=14`, `lo=35`, `hi=65` | Declared Wilder RMA14, arithmetic first14-change seed, zero-filled gains/losses, neutral50 for a flat seed. Strict level below35 enters long, above65 enters short. |
| `lkqts_macd` | `fast=12`, `slow=26`, `sig=9` | Existing period-derived coefficients2/13,2/27,1/5. First genuine close seeds both price EMAs; MACD and signal start0. |
| `lkqts_macd_literal_alpha` | Same12/26/9 labels | Printed MACD-specific coefficients .15/.075 literally; declared signal .2 from the general9-day EMA rule; same seed/cross convention. |

The MACD IDs share the existing MACD family. Coefficient interpretations are configurations, not additional algorithm families. The period interpretation is retained instead of silently changing its default. The literal interpretation is explicit and has no inherited source-performance claim. No choice is based on market counts, profits or matching printed tables.

RSI source prose does not specify the recurrence, seed, flat handling or startup operator. Naming Wilder does not certify those missing details. The local candidate implements `(13*previous + current)/14`, initialized from changes1..14, giving its first defined state at index14. A flat selected start can enter on its first qualifying level; it need not observe a fresh threshold crossing. Equality35/65 supplies no entry, and the middle region holds the current position. One-sided gain/loss states yield RSI100/0; both zero give the declared50. The local threshold checks use the algebraically equivalent strict13U<7D and7U>13D comparisons to avoid dividing at a zero denominator.

Alternative RSI arithmetic-window averaging, conditional nonzero-day averages and observed threshold-cross startup are distinguishable interpretations, not this configuration. The principal threshold paragraph and the annotated figure determine long/short direction despite contradictory illustrative prose and copied stochastic labels elsewhere. No global RSI helper is changed.

The volatility calculation includes changes ending at the current decision close. Its first complete100-change window needs101 real closes, earliest signal index100 and possible open fill101. Strict comparison uses the sign of the current change and its square against four times population variance; no square root or comparison epsilon is introduced. This is finite IEEE754 arithmetic, not exact arbitrary-precision decimal certification. At zero variance, signed constant changes can qualify, while all-zero changes do not. Original share quantity10000/sigma is undefined at zero variance: no quantity, cap or original dollar return is fabricated. Prior-only100-change windows and sample variance produce different signals and are not silently substituted.

For MACD, crossing means previous histogram<=0 and current>0 for long, previous>=0 and current<0 for short. Current equality alone supplies neither entry nor exit. These are signal-line crossings, not MACD-zero crossings. The first-close-seeded profile can have its first observed transition at index1 and next-open fill2; it does not invent an author-prescribed26/34-bar suppression. A flat selected window waits for a transition at or after its starting decision close using the retained full prefix. A named SMA-seeded alternative would require separate declared eligibility. No shared EMA helper is changed.

The local machine-operation convention for both MACD profiles is `state += alpha * (value - state)`, algebraically equivalent to the printed recurrence. It exactly preserves a seeded constant input such as100.1 or1001.13 instead of creating tiny unequal EMA states through separate weighted-product rounding. This declaration is fixed before market comparison; no histogram epsilon or performance tuning is used. Both profiles have explicit flat noninteger-prefix/no-order fixtures. Other strategy EMA helpers and the earlier accepted three methods are untouched.

## History, event clocks and numerical domain

The candidate validates all supplied source observations through the selected end: positive finite internally consistent OHLC, strictly increasing valid dates and one identity. Full earlier history is retained. No missing prefix is filled, invalid observation dropped or recurrence reseeded at the requested start. Early selected observations remain flat until the declared indicator exists; the requested date range is not silently shortened. Nonfinite intermediate variance, squared threshold, RSI state or MACD state makes the direct candidate unavailable instead of clipping a signal. Invalid/nonpositive percentage equity remains the ordinary account validator's responsibility; this module supplies no funding or bankruptcy reset.

Each signal uses a completed close at source index i and fills only at actual supplied session open i+1 within the selected window. Repeated same-side signals do not pyramid. Opposite signals close the previous direction and open the new direction at that same next open. A final-close signal has no invented fill. A final actual-open entry remains open and can be marked at the final close.

Legacy `Trade.signal_i` holds the exit-signal index for a closed trade and the terminal-mark index for an open one. Entry signal may be reconstructed as entry index minus one only after verifying actual next-open execution. These clocks must not be conflated in account/API evidence. A mark is not a filled exit.

## Acceptance work still required

The local schemas contain exact singleton defaults and reject unknown, changed, fractional or nonfinite numeric settings instead of rounding. Admission still needs raw request guards so malformed or near-default textual values cannot be erased before the module sees them, including optimization grids and all response routes. These source changes do not supply those guards or enable policy.

Data-independent fixtures cover the source population arithmetic, positive/negative strict volatility ties, population versus sample and current versus prior windows, zero variance, RSI exact threshold states and seed arithmetic, level-start behavior, reversals, signal-line versus zero-line MACD decisions, coefficient divergence, seed distinction, prefix retention, final pending signals, open marks and invalid observations/defaults. The12 focused Kestner tests passed, followed by800 passing tests in the broader data-independent suite. Five opt-in private-input tests were ignored; the same legacy whole-catalogue private-index test excluded in the earlier batch remains excluded. These checks establish synthetic implementation behavior, not actual-price/account/API acceptance or additional live availability.

Next validation must independently compare indicator states and raw signals/orders/trades on bounded actual US and Indian stock inputs, then compare the normal percentage-account pipeline separately at zero and one nonzero fee/slippage setting. One-share P&L is not that compounded account. Preserve actual long/short clocks, nonpositive-equity rejection, displayed costs and terminal open-liquidation estimates. Do not route shorts through a long-only Pristine Span or change DEMA/Pristine arithmetic. Keep actual source prices, extracted text, generated ledgers and historical results outside Git. Original source figures, declared conventions and fresh simulations remain separate.
