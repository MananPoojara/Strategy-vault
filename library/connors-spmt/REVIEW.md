# S&P Market Timing Course — daily conversion review

Candidate for the isolated `connors-spmt-convert` job, reviewed on 2026-10-13.
This work has not been admitted or published. A different reviewer and the
deterministic publisher must pass the candidate before it becomes live.
The subsequent independent source judgment, declared assumptions and bounded
verification are recorded in [the independent review](INDEPENDENT-REVIEW-2026-10-13.md).

The supplied **S&P Market Timing Manual.pdf** contains 317 scanned pages, despite
the enclosing folder's “350 pgs” label. Page numbers below are PDF page numbers;
the inspected slides use the same printed numbers. The actual source PDF and
rendered rule pages remain outside Git. Their hashes are in the private source
manifest and ledger report.

Page 3 describes tests on E-mini contracts, additional research on SPY, and
entries and exits on the close. This review prepares daily price-signal research
on actual SPY ETF prices and a declared Nifty cash-index timing application.
The latter does not represent an ETF or a tradeable futures contract. Cash-index
short exposure is hypothetical. The E-mini half-point trading cost, contract
dollars, options premiums, dividends and financing are not reproduced here.
Book-reported figures remain separate from these ledgers and are not a condition
for accepting an executable contract.

## Complete daily contracts

All IDs below start with `spmt_`. A long signal requires today's close **strictly
above SMA200** and buys at the close; the exit is **RSI2 strictly above 65**, at
the close. A short signal requires today's close **strictly below SMA200** and
**at least SMA5**; it sells at the close and covers when the close is **strictly
below SMA5**, at the close. All comparisons use unrounded indicator values.

The table supplies every method's additional default predicate. `t-k` means
trading bars ago; day-of-month is the calendar date. RSI caps shown as `<=65`
include equality. Runs end today. A numeric change from these defaults is a
research variant, not a newly printed source rule.

| ID suffix | Rule page | Family | Additional entry predicate |
| --- | ---: | --- | --- |
| `eom_co` | 10 | EOM red bars | Day >=25; last two closes below their opens; RSI2<=65 |
| `eom_rsi` | 14 | EOM RSI | Day >=25; RSI2<25 |
| `eom_cc` | 18 | EOM lower closes | Day >=25; two consecutive lower closes; RSI2<=65 |
| `rsi2_4` | 23 | RSI level | RSI2<4 |
| `rsi4_25` | 27 | RSI level | RSI4<25; RSI2<=65 |
| `rsi_div8` | 31 | RSI divergence | C<C[t-8]; RSI2>RSI2[t-8]; RSI2<15 |
| `rsi_div2` | 35 | RSI divergence | C<C[t-2]; RSI2>RSI2[t-2]; RSI2<30 |
| `rsi_1` | 39 | Cumulative RSI | Sum of last three RSI2 values <50 |
| `rsi_2` | 43 | Cumulative RSI | Sum of last two RSI2 values <40 |
| `rsi_3` | 47 | Cumulative RSI | Sum of last two RSI3 values <45; exit still uses RSI2 |
| `pctb_1` | 75 | %b with RSI | %b(5,1 SD)<0 for at least two bars; RSI2<10 |
| `pctb_2` | 79 | %b with range | %b(5,1 SD)<0.2 for at least three bars; close in bottom25%; RSI2<=65 |
| `price` | 84 | Red bars with range | Three consecutive C<O bars; close in bottom10%; RSI2<=65 |
| `dmi_1` | 114 | DMI | +DI3<5; RSI2<=65 |
| `dmi_2` | 118 | DMI | +DI2<3; RSI2<=65 |
| `dmi_3` | 122 | DMI | +DI2<10 for two bars; RSI2<=65 |
| `dmi_4` | 126 | DMI | +DI2<10 for three bars; RSI2<=65 |
| `dmi_5` | 130 | DMI | +DI2<15 for four bars; RSI2<=65 |
| `short_4up` | 144 | Consecutive up closes | Four consecutive higher closes |
| `short_strong` | 148 | Strong return with RSI | C/C[t-2]-1>3%; RSI2>80 |

These 20 retained source variants correspond to market timing ideas already in
the repository's STC IX catalogue, with the matching ID suffixes. They add **zero
new algorithms**. The SPMT source attribution, study period and claims remain
distinct. There are 12 predicate families within this reviewed subset. Threshold
and lookback alternatives within RSI level, divergence, sums and DMI remain
variants. Price/range, %b/range, %b/RSI and the two short predicates have separate
families. This review does not independently re-review the STC IX source book,
and it does not change `connors.rs` or its execution behavior.

## Explicit research conventions

The book gives the indicator periods and entry/exit thresholds in the table.
It does not print indicator seeds or every account execution convention. Those
assumptions are explicit, rather than being described as printed rules:

- RSI uses Wilder smoothing of positive and negative changes, seeded by the
  arithmetic mean of the first `n` changes. Flat gain and loss give RSI50.
- +DI uses Wilder smoothing of dominant positive directional move divided by
  smoothed true range, starting at bar1. Equal up/down moves give zero +DM.
  SPMT now treats a floating-point subtraction difference within
  `EPSILON*(abs(H[t])+abs(H[t-1])+abs(L[t-1])+abs(L[t]))` as a tie. A real SPY
  equal-move day exposed the issue; this precision treatment is local to SPMT.
- %b uses SMA5 and population SD (divisor5), with the source's1SD bands.
  The divisor is a declared convention. A zero SD cannot produce a %b signal.
- Range location is `(C-L)/(H-L)`; bottom-range equality is accepted. A zero
  range cannot produce a bottom-range signal.
- The strong-short percentage boundary is compared as
  `C > C[t-2]*(1+rise/100)`. This avoids accepting an exact3% move because
  division followed by subtraction rounds just above3%.
- Page10's two closes below their opens are interpreted as consecutive, in
  parallel with page18. The run may start before the25th.
- Indicators retain admitted history before the selected trading start. Nifty
  uses no bars before2000-01-01, including warmup. The existing backend loader
  already applies that floor; no loader or dataset changes are needed.
- Each method starts flat, holds one position, has no pyramiding, and does not
  reopen on the exit close. Portfolio allocation is unspecified and not invented.
- A qualifying final close opens a position marked **open** at that close.
  A remaining position is marked open at the final close rather than liquidated.
- Default contracts are independently checked on SPY and Nifty. Other cash-index
  applications and changed parameters need their own run and evidence; no claim
  of universal market or parameter parity follows from this review. Other ETFs
  and individual stocks cannot silently replace the reviewed SPY signal series.

## Held methods

The other 25 catalogue IDs remain held. The private handoff records each ID,
source context, missing input and reason. In particular:

| Methods | Rule/source context | Hold reason |
| --- | --- | --- |
| Five VIX longs and two VIX shorts | pp.52,56,60,64,69,135,140 | Verified source-exchange VIX close/open/high histories and complete lookbacks are absent from this batch. India VIX is a separate unreviewed adaptation. |
| Five TRIN variants | pp.89,94,98,103,108 | Required exchange-specific Arms-index inputs are unverified. |
| `nyse_adv`, `tick` | pp.153,158 | Actual NYSE advancing issues/TICK are unverified; OHLC or generic auxiliary fields do not supply them. The TICK150 level cannot be silently transplanted to NSE. |
| `multi5`, `multi9`, `mts` | p.6 and pp.164-166 | The complete signal basket is missing auxiliary inputs. Partial counting changes the rules. Signal recurrence, allocation and contract accounting are also unverified. |
| `eom_und`, `eom_puts` | pp.168-169 | The next-open example has RSI76.23 but does not define a general exit threshold. The existing RSI>65 inference remains held. Options additionally require actual traded contracts. |
| `put_sale`, `itm_calls` | Inherited options catalogue context | Outside this daily batch; option prices, contract identity and allocation are missing. |
| Three `dt_*` methods | Inherited intraday catalogue context | Outside the daily batch; actual15-minute prices, execution increments and causal entry/stop ordering are missing. |
| `discretion` | Inherited discretionary catalogue context | No complete mechanical sizing/execution contract. |

The readiness additions name these exact SPMT IDs. Loading a generic auxiliary
column cannot clear a hold, and no partial combined-signal result is published.
All other books' admissions remain unchanged.

## Independent ledger verification

The Python oracle is separately expressed from the source contracts. It uses
40-digit Decimal arithmetic, direct-window means, independently constructed RSI
and +DI, and an independent sequential position ledger. The Rust harness imports
the actual `spmt.rs` and unchanged `connors.rs`; it does not copy their trading
logic. The report hashes those exact compiled source paths, the source PDF and
the original input files.

Run from the isolated checkout, writing evidence outside Git:

```bash
python3 library/connors-spmt/scripts/verify_daily.py --output /home/server/.cache/strategy-vault-webapp/overnight-2026-10-13/jobs/connors-spmt-convert/verification
python3 library/connors-spmt/scripts/verify_policy.py --output /home/server/.cache/strategy-vault-webapp/overnight-2026-10-13/jobs/connors-spmt-convert/verification
```

The standard comparison period is2010-01-01 through2020-09-11, with the first
actual trading day2010-01-04 on both series. SPY retains1993-01-29 onward for
warmup; Nifty retains2000-01-03 onward. Current constituent membership is not
historical membership. The supplied AAPL and RELIANCE files are hashed and
explicitly rejected as the wrong population for these market timing contracts;
they are not used as replacement market series.

Every check compares chronological entry and exit dates, directions, prices,
fill modes, signal indices and open flags. Setup conditions are compared for
every tested bar, including signals ignored while a position is held. Dates and
flags must match exactly; price tolerance is `max(1e-9,abs(expected)*1e-12)`.
A zero-trade run alone cannot pass the method's ledger check. The report contains
per-method contracts, full ledgers, mismatch files, coverage and all actual
commands. Failed first-pass DMI evidence is retained privately.

Focused Rust tests cover strict thresholds, inclusive entry caps, calendar dates,
sequential positions, final-close fills, unavailable warmup, zero-range bars,
directional ties and family separation. Scoped integration assertions import the
actual policy/readiness/gate/baskets modules and check both real market contexts,
all held IDs, stock and ETF substitution rejection, wrong-book/new-ID rejection
and preserved Kaufman admissions. Full-backend `cargo check --tests` passes.
Linking the full backend test executable exceeded the imposed1800MiB address
space limit; that failure is recorded, not represented as a passed test suite.
The focused tests and integration executable fit the approved resource budget.

The required hook installation was attempted, but `.git/config` is read-only in
this managed checkout. No commit, push, deployment, service restart or live API
run is part of this handoff.
That paragraph records the original conversion environment. The host subsequently
prepared this review candidate on the current authoritative base and installed
hooks. The independent reviewer leaves Git metadata untouched; host finalization
remains responsible for validating the exact source hashes and creating a commit.
