# DVMTM source-admission correction

This review covers only the Double Volume Market Top Method (`ats_dvmtm`) in
Chapter 13 of Laurence A. Connors's *Connors on Advanced Trading Strategies*,
printed pages 103–108. The inspected primary scan has SHA-256
`9dfb24ab5f426736ee36a213fc7c402a30e0e88ba0fc89d9da6ecadd41341d86`;
these pages occupy physical pages 57–60. No private source or market rows are
included here.

The chapter gives a stock setup, a short stop-entry below the Rule 3 day low,
and an initial protective stop at that day's high. Rule 5 on printed page 104
also calls for trailing stops to protect profits. It supplies no numerical
distance, activation condition, update rule or complete trailing exit. The four
examples describe entries and subsequent price moves; they do not establish a
fixed holding-period exit.

The existing runner instead exits after a selected number of bars, defaulting to
five. That fills a missing strategy rule. It is not a minor indicator-period
default and cannot be certified by matching an entry example. Source review also
distinguishes the printed three-month calendar-day high from the old 63-session
window, the unquantified near-high tolerance and the historical USD one-eighth
entry offset from a generic local stock tick.

The catalogue now withholds this method as `not_testable`. The common admission
check blocks result publication and optimization even with actual US stocks,
complete volume, a changed hold period or a USD-compatible offset. India, index,
ETF and basket choices cannot supply the absent exit. Rules remain readable;
charts and performance are unavailable for a held run.

The old simulator is retained as historical implementation, without changing its
calculations or approving its outputs. Any future mechanical adaptation needs
its own explicit specification, identity, independent review and accounting
checks. This correction does not certify other ATS chapters, reproduce published
performance or change the independently reviewed Kaufman stock rules.
