# Kestner next stock conversion candidates

These source-conversion candidates have bounded production-handler acceptance, described below. Registration, synthetic fixtures and this note do not establish measured availability or original book performance. The earlier channel40/20, SMA10/40 and momentum80 batch has its own contract; its accepted behavior and admission guards are separate. The deployed engine and measured availability remain unchanged until a coordinated release passes its remaining checks.

## Source and scope

Lars Kestner, Quantitative Trading Strategies (2003), source PDF SHA-256 `168ff6a95da48f83bb8dcd5d8e00dc3f5e5e9895fdeae1e95dae0e845abd705b`. Printed body page plus22 identifies the one-based physical PDF page. Printed41 defines population variance; printed57 gives the general EMA coefficient; printed66 defines RSI through average up/down moves; printed68 gives MACD-specific coefficients; printed139 establishes close signals and next-session-open execution. The concrete strategy rules are on printed155 (volatility breakout),165/170 (RSI) and170/175 (MACD).

The intended next validation uses actual selected US and Indian individual stocks. A stock, cash index, ETF and futures contract retain distinct instrument identities. No stock universe or derivative can be supplied by an index substitution. Source stock/index tables and dollar-volatility position sizing are separate from selected-stock normalized percentage research. Present constituent membership, quote adjustments, borrow, financing, margin, dividends and original study performance are not certified here.

The original exact-three stock-ID list remains unchanged. A separate four-ID list identifies the configurations below. The source candidate now registers all four and supplies stock-only policy, request/history guards and causal diagnostics so bounded production-handler verification can exercise them. This source configuration does not establish deployment, measured availability, account viability for every requested window or original book-performance reproduction. Index, ETF, derivative and basket applications remain outside this reviewed scope.

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

The candidate validates all supplied source observations through the selected end: positive finite internally consistent OHLC, strictly increasing valid dates and one identity. Full earlier history is retained. No missing prefix is filled, invalid observation dropped or recurrence reseeded at the requested start. Early selected observations remain flat until the declared indicator exists; the requested date range is not silently shortened. Nonfinite intermediate variance, squared threshold, RSI state or MACD state makes the direct candidate unavailable instead of clipping a signal. Detectable squared-change/variance underflow and nonzero RSI numerators disappearing during averaging also hold; genuine zero states remain supported. Invalid/nonpositive percentage equity remains the ordinary account validator's responsibility; this module supplies no funding or bankruptcy reset.

Each signal uses a completed close at source index i and fills only at actual supplied session open i+1 within the selected window. Repeated same-side signals do not pyramid. Opposite signals close the previous direction and open the new direction at that same next open. A final-close signal has no invented fill. A final actual-open entry remains open and can be marked at the final close.

Legacy `Trade.signal_i` holds the exit-signal index for a closed trade and the terminal-mark index for an open one. Entry signal may be reconstructed as entry index minus one only after verifying actual next-open execution. These clocks must not be conflated in account/API evidence. A mark is not a filled exit.

## Acceptance work still required

The local schemas contain exact singleton defaults and reject unknown, changed, fractional or nonfinite numeric settings instead of rounding. The production-handler candidate additionally checks raw numeric text and singleton grids before legacy rounding/default handling. Duplicate, bare or encoded query keys in a request naming any protected Kestner method are rejected before map collapse can erase ambiguity. The bounded positive/negative production-handler checks below cover these guards and the exact stock-only policy. Other instrument types, settings and source/account interpretations gain no admission from those checks.

Data-independent fixtures cover the source population arithmetic, positive/negative strict volatility ties, population versus sample and current versus prior windows, zero variance, RSI exact threshold states and seed arithmetic, level-start behavior, reversals, signal-line versus zero-line MACD decisions, coefficient divergence, seed distinction, prefix retention, final pending signals, open marks and invalid observations/defaults. The final focused and broader test counts are recorded below. Synthetic checks alone do not establish actual-price/account/API acceptance or additional live availability.

A separate opt-in capture subsequently checked the ordinary pipeline on actual AAPL prices for1990–2001 and RELIANCE for2001–2010. Four configurations at zero costs and fee3bps/slippage2bps per side give16 context/cost runs, not16 methods. All source trade directions, signal/fill dates, observed fill prices and terminal-open states matched the frozen exact-rational/Decimal100 reference. For volatility and both MACD profiles, all12 normalized-account cases also matched the independent account formulation and checked statistics. RSI's four cost/instrument cases remained withheld: both the reference and ordinary engine reject a nonpositive normalized account. Source-event agreement does not make that account financially supported. No losses were clipped, accounts reset, positions resized or periods shortened to create an available result. The account-hold comparison establishes withholding, not a certified bankruptcy date or margin process.

Public request guards, indicator/clock serialization and instrument admission are implemented in the candidate and passed the bounded production-handler verification below. Runtime deployment and measured availability are separate pending gates. These next profiles have not been deployed or added to the live availability count. The separate source-trade artifacts are retained even when their normalized account is withheld; they are not published performance results.

The reference author did not read candidate Rust or candidate results, but did read the requested source contract's appendix describing legacy implementation agreement. This is disclosed source-based corroboration, not complete blindness to implementation descriptions. The account oracle's author separately disclosed prior exposure to legacy accounting. RSI/MACD reference recurrences use finite Decimal100 arithmetic, rather than exact real arithmetic or an emulator of Rust binary64 operations. Dates, predicates expressed as executed events and trade identity require exact agreement; finite account numbers retain the earlier field-specific comparison bounds.

## Bounded production-handler acceptance

The final focused guard/diagnostic suite passed 13 tests. The broader scoped suite
passed 813 tests, with seven opt-in tests ignored and one legacy private-index
test excluded. A separately invoked ignored test then captured 480 production
handler responses: 64 positive requests over the 16 fixed context/cost cases,
plus 416 negative requests. Each raw request passed the actual query parser and
owner restriction before run, readiness, selected summary or singleton
optimization. This is function/serialization evidence, not HTTP transport,
authentication or deployment evidence.

The frozen independent source/account comparison passed all 16 cases, including
the four expected RSI account holds. It checked 33,156 selected causal states,
286,944 numeric rule fields, 33,156 account dates and 1,956 repeated-context trade
spans including terminal marks. These are verification-case totals, not numbers
of strategies or distinct economic trades. Predicates, dates, direction and
open/closed/fill states require exact agreement; numeric serialization retains
the previously frozen bounds. Readiness describes input eligibility and does
not certify account viability. Held RSI run/optimizer requests remain errors;
selected summaries carry a calculation-unavailable reason without performance.

Two additional full-history chart responses were bound to their original source
CSVs and selected absolute indices. The first comparison retained one failure:
an AAPL source volume of zero on 1981-08-10 was exported as null by the unchanged
native loader. A disclosed post-observation clarification checks that existing
zero-to-unavailable mapping and records that one pre-window observation. The
failed report remains retained. No price, source predicate, trade, account,
numeric tolerance or raw source file was changed to resolve this auxiliary
volume representation difference. Volume correctness is not certified by that
mapping.

The frontend candidate consumes those backend causal states instead of
recalculating indicators. It validates profile, fixed settings, native source
identity, full-precision selected prices, dates and source indices. Its focused
15 tests, complete 326-test React suite, type check and production build passed.
Missing, held or mismatched reports cannot supply indicator values. Actual
browser checks passed 16 captured-report/component scenarios, including real
visible indicator lines, desktop/mobile rendering, daily-to-weekly/monthly
sampling, collapsed controls and suppression of held or mismatched reports.
Screenshot inspection exposed an earlier blank-chart viewport outside the
selected report; the scoped viewport fix now ends at the selected report's
last actual observation. Full source history remains available. This browser
check uses actual captured backend reports and production chart modules; it
does not certify authenticated full-page routes or deployed HTTP behavior.
Staged/live verification remains pending.

Next validation must exercise a release binary over real HTTP and preserve
existing accepted methods, then coordinate frontend, measured availability and
runtime checks before deployment. One-share P&L is not the compounded account
already checked. Preserve actual long/short clocks, nonpositive-equity
rejection, displayed costs and terminal open-liquidation estimates. Do not route
shorts through a long-only Pristine Span or change DEMA/Pristine arithmetic.
Keep actual source prices, extracted text, generated ledgers and historical
results outside Git. Original source figures, declared conventions and fresh
simulations remain separate.
