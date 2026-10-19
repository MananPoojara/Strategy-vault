# Visual strategy rules

The information button beside **Rules** opens a React visual guide on every
strategy route. It keeps the complete registered source rules and implementation
notes visible, including when a selected backtest is withheld. Held selections
never receive an illustrative trade or fabricated chart.

For an admitted run, the guide uses that run's recorded entry legs, partial exits,
final fills and terminal open marks. A long position buys and then sells; a short
position sells short and then buys back. An open mark is not an exit. The exit
signal date, when supplied, is separate from the fill date. An entry signal date
is not inferred from the exit signal field or from generic indicator snapshots.

Choose a trade, hover a daily candle to inspect it, or click to pin the values.
The tooltip identifies recorded fills on that date. The inspector shows actual
OHLC, available condition inputs and the precise comparison used by a supported
provider. Zooming changes only the view. Position selection remains bounded to at
most 41 nearby dropdown options, with a numeric jump to every actual position.

## What the guide proves

There are three distinct calculation scopes:

- **Method-specific calculations** reproduce the declared engine arithmetic for
  the selected parameters. An individual price or indicator comparison can be
  marked met. This is not a claim that every execution, position-state, sizing,
  source-interpretation or data-quality requirement has been verified.
- **Partial indicator inputs** show the existing Connors/High Probability ETF
  indicator calculations. Price patterns, auxiliary series, liquidity, scale-in
  and position-management rules may remain outside this view. No indicator value
  is promoted to proof that all rules matched.
- **Rule trace unavailable** keeps actual trade fills and all registered rules
  readable, but does not manufacture a source-specific condition calculation.

An arrow always comes from an actual accepted trade. It never comes from a
frontend recomputation of a signal or a hypothetical example. Existing source
review, instrument-admission and data-gap notices continue to govern the run.
For scale-outs, each piece retains its own price; the aggregate weighted exit
price is not presented as a final fill. An open position's terminal tuple is a
remaining-position mark, and ambiguous scale-outs on that terminal date remain
readable without inferring a terminal exit arrow.

As of the 2026-10-19 catalogue snapshot, all 4,321 registered definition routes
use the common Rules presenter. Among the 183 definitions with live status and a
compatible instrument policy, 30 have method-specific arithmetic providers, 63
have partial indicator inputs, and 90 await a condition provider. These are
frontend coverage counts, not verified-book counts or assurances that the
required data exists for every selection. Detailed condition coverage remains
unfinished; the common popup does not certify all strategies.

## Method-specific providers

- `kftsm_exp`: all three declared smoothing kinds. Kind 0 is mean-seeded
  SmoothedAverage, followed by `(previous × (n−1) + close) / n`; it is not an
  ordinary EMA. Kinds 1 and 2 seed at the first actual close and use `2/(n+1)` and
  `1/(n+1)`. Normalized trend change uses the previous close, applies the selected
  lag, compares strictly with the signed filter, and carries direction through
  equality or the neutral band. An initial selected entry may use an earlier
  latched direction without a fresh crossing on that candle.
- `kftsm_ma`: the engine's full-history running-sum SMA, with the same normalized
  change, lag, filter and direction latch.
- `kfsc_two_ma`: both SMA directions use the exact close-versus-N-bars-earlier
  comparison. This is not an average crossover.
- `kfsc_pct_band`: fresh-window mean; upper `MA × 1.05`, lower `MA / 1.05`.
- `kfsc_bollinger`: fresh-window mean and closing-price-level sample deviation
  with denominator `N−1`, preserving the documented source-formula conflict.
- `kfsc_macd`: literal `.0476/.0243/.10` smoothing, first-close seeds, and actual
  zero-crossings of MACD minus signal. No standard-period substitution.
- `kfsc_dsm`: current close minus the close 10 bars earlier, two literal `.0476`
  passes, and a genuine change in the last nonzero slope. The first observed
  slope is not invented as a turn.
- Burns pairs `sbma_x_5s20s`, `sbma_x_5e50s`, `sbma_x_10e20e`,
  `sbma_x_10e50e`, `sbma_x_15e30e`, `sbma_x_15e150e`,
  `sbma_x_10s200s`, `sbma_x_50s200s`: fresh-window SMA, mean-seeded EMA,
  strict crossings with previous-value comparisons, and selected fill and
  long/cash-or-short settings. The frontend does not create fills from them.

Indicator arithmetic retains the engine's complete source history. Only actual
dates in the admitted run are painted as candles and indicator points; earlier
warmup bars are not presented as admitted OHLC history. Missing trading dates are
not interpolated, and sliced histories cannot establish exact smoothed values.

## Connors predicate expansion (2026-10-19)

The following 15 existing definitions now add method-specific entry, exit and
position-gate tests. This is visualization coverage, not 15 newly converted
strategies or an independent re-review of all source books:

- `dmi_1` through `dmi_5`, and `spmt_dmi_1` through `spmt_dmi_5`:
  trend close strictly above the selected SMA; current +DI and its complete
  consecutive-bar window strictly below the selected threshold; RSI entry cap
  inclusive of equality; strict RSI exit. +DI is not ADX. Both use the declared
  Wilder seeds. SPMT alone uses its reviewed subtraction-roundoff tie budget;
  the unchanged STC IX comparison is retained for the course methods.
- `rsi2_4` and `spmt_rsi2_4`: selected trend and strict RSI-level entry;
  the separate RSI exit threshold. An extra RSI entry cap is not invented.
- `rsi4_25` and `spmt_rsi4_25`: the slower selected RSI supplies the entry
  oversold test; the separately selected fast RSI supplies the inclusive entry
  cap and strict exit. Their default names do not override custom periods.
- `r3`: course R3, two half-position units, selected falling-RSI streak,
  rolling first-falling-day threshold and current RSI entry threshold. Its
  first-day index is `today + 1 - fall`, while HPETF R3 exposes a selectable prior-value
  rule. A later held bar may add the second half; exits take priority over
  adding. The add predicate does not repeat all original entry conditions.

These source-specific providers require the actual matching `stc9` / `spmt`
book identity and resolved selected parameters. Missing or mismatched source
identity is unavailable. Raw ledger dates and every leg/exit tuple are checked
before display sanitization; dropped malformed pieces cannot establish position
permission. These fifteen close-only long providers additionally withhold state
checks for overlapping source holdings, missing source dates, or an incompatible
leg allocation. Price-input tests remain separate from unknown position state.
Falling-streak checks stay unknown until all required RSI comparisons exist.

Position gates use the selected run's recorded ledger, including the held exit
close. Entry price tests can pass while that position gate prevents a new trade.
For a basket chart, another constituent's holding cannot supply the selected
stock's position state. A basket with no accepted holding cannot prove that the
underlying source runner is flat: a capacity-rejected source position may still
be open. Accepted exposure is labelled separately and source-flat permission
stays unknown in that case. Missing or malformed ledgers also leave this gate
unresolved.
A final-close entry is supported by the reviewed SPMT close runner; the shared
STC IX runner does not open a new position on that final selected close.

The existing public [SPMT independent review](../library/connors-spmt/INDEPENDENT-REVIEW-2026-10-13.md)
provides the reviewed daily source scope and declared conventions. This frontend
change does not transfer that acceptance to the STC IX course or HPETF variants,
change admissions, or certify printed performance. HPETF R3 and auxiliary-data
methods retain their existing partial or unavailable condition status.

## Basket handling and resource limits

A basket chart follows the chosen accepted stock position, never its benchmark.
Opening a basket guide can fetch **one** constituent's full read-only price
history, using its qualified ticker and the native catalogue source and CSV
hashes. The response must match those identities. The current basket response
does not embed historical source hashes, so this matches the engine catalogue
identity rather than proving a historical dataset-version contract.

Changing stock cancels the previous request. Requests have a 30-second bound,
closing or leaving the route aborts them, and at most four matched histories are
cached. There is no new backtest, universe-wide fetch or corpus calculation on
opening the popup. Missing source identity or prices leaves a clear unavailable
state with the actual recorded fills readable.

Each open guide owns its chart and companion panes. Close, route replacement and
unmount remove charts, observers and listeners. Escape, focus containment, focus
restoration, body-scroll restoration and reduced-motion preferences are handled
by the React component. All styling inherits the page's existing theme and font.

## Adding a condition provider

Use `src/services/rule-evidence.ts` for typed arithmetic and evidence, with
`RulesVisualGuide.tsx` as the common presentation. New providers must:

1. Identify the exact method IDs and resolved parameters; fail closed for missing
   or unsupported settings.
2. Match source-reviewed engine arithmetic, seed, warmup, inequalities, equality,
   lag and state conventions. Do not substitute an indicator of the same name.
3. Keep computation on full source history and map values by actual date.
4. Distinguish an individual predicate from the full entry/exit/state machine.
   Missing conditions remain visibly partial; generic RSI/SMA context is not a
   source-rule proof.
5. Preserve recorded trades as the only authority for arrows, execution prices,
   scale-ins, partial exits and terminal open positions.
6. Add independent arithmetic and event checks for non-default parameters,
   selected-window starts, warmup, ties, reversals and terminal bars, plus modal
   integration and real-browser verification. Keep private market rows, raw
   outputs, source books and review artifacts outside Git.

No Rust strategy calculation or authentication behavior is changed by this
presentation feature.
