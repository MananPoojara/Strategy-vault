# Independent SPMT daily source review

Reviewer `connors-spmt-handoff-v2-review`, separate from conversion
`connors-spmt-convert`, inspected the actual candidate on prepared base
`340b3a6462d76659d4682e043e2960ea574c6828`. The accepted subset is the twenty
exact daily IDs in the [rule table](REVIEW.md#complete-daily-contracts).
Live admission remains pending host finalization and publication gates.

The actual TradingMarkets S&P Market Timing Manual has 317 scanned PDF pages.
The supplied folder also contains an archived TradingMarkets HTML page and its
assets; no companion CD, archive or executable source was present. The review
rendered and visually read the PDF itself, independently of conversion notes.
Attribution is Larry Connors / Paul Sabo, the S&P Market Timing Course for
E-mini and Options Traders, supplied February 2007 edition.

## Source identity and inspected rules

| Actual supplied source | SHA-256 |
| --- | --- |
| `S&P Market Timing Manual.pdf` | `d68b28191ed2289b124b5d0e788f5d973e7b9cc836383558ac7327588ad0ddc9` |
| Archived `Daytrading TradingMarkets Research.htm` | `f55cc2fe363435f9190bf13e0b01860be0ab370359de6281d859629d6e6d5a1a` |

Page 3 supplies close execution and distinguishes E-mini tests from subsequent
SPY research. Daily rule pages inspected were 10, 14, 18, 23, 27, 31, 35, 39,
43, 47, 75, 79, 84, 114, 118, 122, 126, 130, 144 and 148. The periods,
thresholds, comparisons, consecutive-bar requirements and exits in the rule
table agree with these pages. In particular, page 47's entry sums RSI3 while
its exit uses RSI2; page 148's two-day return must be strictly greater than 3%;
the shorts' SMA5 entry accepts equality and the cover does not.

The reviewer also inspected pages 6, 52, 56, 60, 64, 69, 89, 94, 98, 103, 108,
135, 140, 153, 158, 164–166, 168–169, 216, 218–219, 222, 230, 240, 254, 257
and 259–260. These establish exchange-specific VIX/TRIN/breadth dependencies,
combined-signal contract sizing, the incomplete general exit specification for
the EOM options example, actual option-premium requirements, discretionary
allocation and intraday path/tick requirements. Those twenty-five other IDs
remain held. An isolated example at RSI76.23 does not establish an exit
threshold. Generic auxiliary columns cannot certify the exchange inputs.

## Interpretation and scope

The original SPY predicates are complete daily signal contracts. Execution on
actual SPY OHLC is selected price research; Nifty is a declared cash-index timing
application with hypothetical short exposure. Neither application reproduces
E-mini contract returns or options P&L. Individual stocks and other ETFs are
incompatible with this reviewed signal population. Source results, simulated
price ledgers and these declared applications retain separate meanings.

The following remain declared conventions rather than printed rules: Wilder
RSI and +DI seeds from the first complete `n` interbar changes; RSI50 on a flat
seed; population SD for %b; inclusive bottom-range boundaries; undefined %b or
range location produces no setup; page 10's red closes are consecutive; flat
at the selected start; one sequential position per method; no pyramiding or
exit-close reentry. The local +DM roundoff-tie treatment was reviewed against
exact decimal arithmetic. SMA includes the signal close. Full admitted history
warms indicators, including only Nifty history from 2000-01-01 onward. Missing
auxiliary prices are never imputed. Changed numeric parameters are research
variants, with no claim of source-default parity for all combinations.

A close entry has no exposure earlier on its signal bar. All bars while held,
including the exit bar, prevent another entry. There is no additional arbitrary
locked-bar interval after exit. A final-close entry is retained as open, and
an unresolved position is marked at the final close, without inventing an exit.
Daily close fills follow the source's simulation convention; actual auction
execution, financing, dividends, capital sizing and contract accounting are
outside this verification.

## Independent evidence

The reviewer wrote [a second Python oracle](scripts/independent_review.py) from
the visually inspected rules. It imports neither the conversion oracle nor its
results. It uses 48-digit Decimal arithmetic, direct indicator windows and a
separate sequential position loop. A newly compiled Rust harness imports the
actual final candidate `books/spmt.rs` and unchanged `connors.rs`.

The check period is 2010-01-01 through 2020-09-11 on both actual SPY and Nifty.
SPY warmup starts 1993-01-29; Nifty warmup starts 2000-01-03. For every method
and series, the oracle compares the main ledger, the first 199 warmup bars, a
window ending exactly at the first entry, and a flat restart during the first
position. All 160 checks matched, including 2,370 actual trade records and all
tested setup flags. Each method has nonzero standard-window trades on both
series. Row dates, indices, direction, fill mode, signal timing and open flags
match exactly; price tolerance is `max(1e-9, abs(expected)*1e-12)`. The count
includes records from distinct boundary windows, not 2,370 unique positions.
Complete inputs, ledgers, rendered pages and calculation details remain private.

Manual entry, exit and nearby non-signal checks included:

| Method / instrument | Entry and first source exit | Nearby rejected or ignored condition |
| --- | --- | --- |
| EOM close/open / SPY | 2010-01-25 close109.77, SMA200101.01465, RSI2≈25.031; exit 2010-02-02 close110.38, RSI2≈81.531 | 2010-01-22 has a red close and low RSI but fails the calendar gate; 2010-01-26's setup is ignored while held; RSI2≈63.810 on 2010-02-01 does not exit |
| %b/range / Nifty | 2010-01-12 close5210.4; last three %b≈0.100625,0.026829,-0.337544; range location≈0.094927; exit 2010-01-14 close5259.9, RSI2≈77.199 | 2010-01-11 lacks three qualifying %b bars because 2010-01-07 %b≈0.694591; 2010-01-13 RSI2≈53.043 does not exit |
| Strong short / SPY | 2010-06-11 close109.68, two-day return≈3.422914%, RSI2≈86.296, below SMA200111.00415 and above SMA5107.398; cover 2010-06-21 close111.41 below SMA5111.848 | 2010-06-10 has RSI above80 but two-day return≈2.372913%, failing3%; 2010-06-18 close111.73 remains above SMA5111.468 |
| DMI3 / SPY | 2010-01-21 close111.7; last two +DI2≈6.438913,2.547004; exit 2010-02-02 close110.38 | 2010-01-20 lacks two +DI2 values below10 because 2010-01-19 is≈15.568128; repeated setup days do not add positions |

The source inputs were hashed in place without writing to the original shares:

| Input identity | SHA-256 |
| --- | --- |
| `us_data.csv`, actual SPY selected | `edc711af4b2d25bc4bbc6d7cc78dc6a6028a57ae998d7ebea29a6086d279f898` |
| `idx_data.csv`, actual NIFTY 50 selected | `a6a135e8dcdc5d75b0178c292824c4f16845d8531f0c144bdf4e35a30eae874b` |
| Supplied RELIANCE stock file | `6b9b89406825efb7b0fdd87b97f1ace6fd8cc0532603441f79f35041a64b052c` |
| Supplied AAPL stock file | `c220fe04bfd335ae85d6e1ad19464b3aa58026473f102f47012d4618f3bc5319` |

The server's actual native catalogue resolves the stock rejection cases to
`basket:nifty50:RELIANCE` and `basket:us_sp500:AAPL`; both cover the requested
2010-01-01 through 2020-09-11 period. These are actual stock prices, not index
substitutes. Native membership is a snapshot and adjustments are not certified.

Focused Cargo checks importing the final source passed eight
`books::spmt::tests::daily` tests, one `instrument_policy::tests::spmt_review`
test and one `readiness::tests::spmt_partial` test. The isolated harness excludes
unrelated baskets API tests while retaining the exact baskets production code.
Actual-module readiness/policy integration also passed on both price series.
Full-backend `cargo check --tests` passed against the final prepared candidate.
The full backend test build exceeded a 1900MiB address-space limit during LLVM
code generation; this is recorded as a failed build, not a passing full suite.
The host publisher must still run its own full tests and actual API gates.

Compiled baseline/current catalogue-policy comparison preserved all 95 STC IX
and SPMT entries in the scoped harness. All 75 unapproved entries retain their status and allowed instrument
types; exactly the approved twenty policies changed. The 25 other SPMT IDs had
no admitted baseline instrument type. No existing method was removed.

## Counting and intentional corrections

All twenty approved IDs already exist in the catalogue and duplicate the
corresponding STC IX suffixes. They contribute zero new algorithms. The twelve
predicate families describe retained variants and do not establish twelve new
economic ideas. STC IX source or calculation code was not changed or reapproved.

The exact approved IDs authorize the scoped final-close correction, proven by
forty actual-price final-entry ledgers. The five `spmt_dmi_1` through
`spmt_dmi_5` IDs authorize the local decimal-tie correction; `spmt_short_strong`
authorizes the strict3% price-boundary comparison. Family attribution separates
different predicates without counting threshold variants as algorithms. The
private review binds these corrections to the compiled final module hashes and
source-derived ledgers. Host Git finalization and publication remain separate.
