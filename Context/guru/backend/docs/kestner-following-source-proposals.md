# Following Kestner source proposals

This document preserves the source-proposal stage of the methods below. It
does not establish current implementation or measured availability. Later work
is documented separately in [the volatility/RSI/MACD review](kestner-next-stock-candidates.md),
[the SOB review](kestner-sob-stock-research.md), [the MACS review](kestner-macs-raw-stock-research.md)
and [the TIAR review](kestner-tiar-stock-research.md). Those documents distinguish
the checks performed from outstanding release gates. No instrument admission
follows from this historical proposal.

Source: Lars Kestner, *Quantitative Trading Strategies* (2003), PDF SHA-256
`168ff6a95da48f83bb8dcd5d8e00dc3f5e5e9895fdeae1e95dae0e845abd705b`.
Physical PDF page numbers equal printed body page numbers plus 22.

## Twenty-day momentum

Printed 150 defines a strict directional level: long when today's close exceeds
the close 20 observations earlier, short when it is below that close. Printed
151 confirms that this momentum triggers entries and exits. The comparison
requires 21 actual closes: the first possible decision index is 20 and its next
actual opening is index 21. A 19-observation lag or a required previously defined
zero crossing would be a different rule.

Equality supplies no new direction. Under the proposed single-position research
convention, an existing position remains held, repeated same-side levels do not
pyramid, and an opposite level exits and reverses at the same next actual open.
Retain genuine history before a selected period, begin that selected account
flat, and import neither an earlier position nor an earlier pending order. A
final signal with no following in-period open stays unfilled; a terminal close
mark is not a filled exit. These account-window and event conventions are
explicit interpretations, rather than additional quotations from the book.

Seven private, hand-reasoned counterexamples distinguish 20 versus 19 lags,
initial level versus observed crossing, later selected starts, reversals,
equality/resumption, pending terminal signals and terminal marks. They remain
specifications until executed as fixtures and checked against actual prices.
The source-only reviewer had prior implementation-context exposure and did not
reread Rust for this task; this is not a blind implementation review.

Twenty- and eighty-day momentum share the existing `lkqts_mom` family. Reviewing
another horizon does not create a new family or establish measured availability.

## Second Order Breakout

Printed 183 lists the method; the statistic is defined on 185, with entry rules
on 190 and the Intel narrative continuing on 191. It uses closing prices and
the longest signed high/low breakout horizon up to 80 days, followed by a
40-observation channel of that statistic. A simple price channel without this
second channel is a different strategy.

The formal entry paragraph requires the statistic to be strictly greater than
20 at its 40-day high, or strictly below -20 at its 40-day low. The Intel example
instead describes entries at exactly +20/-20. Preserve this contradiction. The
proposed default follows the formal strict rule; an inclusive-threshold reading
would need its own named convention and evidence, rather than a choice fitted
to performance.

The proposed profile is
`sob_close_max80_current_inclusive_channel40_tie_inclusive_strict20`:

- Require 80 actual closes. For each horizon 1 through 80, include the current
  close in its high/low comparison and allow equality. Let `h` and `l` be the
  longest qualifying high and low horizons. The statistic is `h` when `h > l`,
  `-l` when `l > h`, otherwise zero. A completely flat window is neutral.
- Require 40 consecutive defined statistics, including today. Long when today's
  statistic exceeds 20 and equals their maximum; short when it is below -20 and
  equals their minimum. A tied channel extreme can qualify again without adding
  exposure to an already same-side position.
- These choices of inclusion, tie handling and complete-window initialization
  are declared conventions where the source is incomplete. First statistic
  index is 79, first complete predicate index 118, earliest next-open fill 119.
- Preserve the genuine prefix and start each selected account flat. Proposed
  opposite-entry reversal and completed-close/next-open timing are explicit
  research conventions; Chapter 7's clock on printed 139 is not an additional
  SOB-specific execution statement. Do not invent an independent stop or a
  terminal exit fill.

Fifteen private hand-reasoned vectors specify distinguishing cases. They are
unexecuted examples, not backtests. The source review checked text and rendered
pages 185/190 without reading the current Rust method. It does not certify the
existing implementation or the source's reported results.

## Instrument and accounting boundaries

An actual selected stock may support a separately labelled price-return
application after its own source, data, trade and account checks. A cash index,
ETF, futures contract or current basket membership cannot substitute for a stock
or prove the original study population. The book's mixed-instrument,
volatility-scaled dollar sizing is distinct from ordinary normalized percentage
accounting; original quantities and profits are not supplied by these rules.

Keep quote adjustments, historical membership, complete calendars, borrow,
financing, dividends, contract multipliers and rolls qualified. Preserve
nonpositive-account holds without clipping, funding resets or retuned dates.
Only after implementation, independent chronological comparison, API admission
checks and runtime verification can a configuration contribute to availability.
