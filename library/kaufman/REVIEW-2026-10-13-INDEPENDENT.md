# Independent review of the TSM4 adaptive and daily-pattern contracts

Reviewer job `kaufman-tsm4-handoff-v2-review` inspected the prepared candidate at base `340b3a6462d76659d4682e043e2960ea574c6828`, separately from conversion job `kaufman-tsm4-convert`. This review accepts the ten exact research contracts below for host finalization and publication gates. Live admission remains false. The prior seventeen reviewed methods are excluded from this source review and its checked-trade count.

The actual source is Perry J. Kaufman's *New Trading Systems and Methods*, Fourth Edition (2005), the supplied 942-page PDF and its companion TSM4 CD. Page references are PDF page numbers. The reviewer read the relevant PDF pages and CD programs directly, and wrote a new private Python implementation without importing the conversion's strategy, ledger, loader or comparison functions. The Rust harness imports the actual final candidate `books/kftsm.rs` and `connors.rs`; it does not reimplement their calculations.

## Accepted rules and source boundaries

| Exact ID | Independently inspected source | Accepted interpretation and limitations |
|---|---|---|
| `kftsm_kama` | PDF603–605; Ch17 Adaptive systems and KAMA function | CD selector1, ER over10 close changes, constants .666 and .0645, squared smoothing constant. Zero noise gives ER1. First observed close seeds the recursion; incomplete initial ER history uses ER1. This is declared observed-history initialization, not recovered TradeStation prehistory. |
| `kftsm_adapt_exp` | Ch17 Adaptive systems selector0; PDF603,605 | SmoothedAverage with period10. The absent platform implementation is interpreted as a first10-close mean followed by alpha1/10. The complete driver adds a strict prior-change volatility filter; it is different from the Chapter8 unfiltered benchmark. |
| `kftsm_vidya` | PDF605–606; Ch17 Adaptive systems and VIDYA function | Recent10-close population SD divided by historical50-close population SD; multiply by2/11. Seed at the first complete50-close window, leave earlier values unavailable, and carry when the denominator is0. No volatility-ratio clipping. This warmup/degeneracy adaptation is explicit. |
| `kftsm_adapt_r2` | PDF608; Ch17 Adaptive systems and Adaptive R2 function | CD selector2 squares Pearson correlation between time and10 closes. It does not implement the printed `(r+1)/2` or regression-residual variants. Observed closes seed until a complete correlation window exists; zero price variance carries the trend. |
| `kftsm_adapt_rsi` | PDF608–609; Ch17 Adaptive RSI function and driver introduction | Declared extension that inserts the supplied RSI function into the supplied driver, as the introduction suggests. It is not a delivered fifth selector or a new exact original book algorithm. It inherits adjustable10/.1/0 defaults. Wilder RSI uses10 actual changes; ARSI carries the first close before RSI exists. Flat gains/losses give RSI50; positive gains with zero losses give100. |
| `kftsm_swg` | PDF278,288–289; Ch05 and Ch08 swing systems/functions | Type0, close-scaled distance2.5% of today's close. Extend the active extremum before testing reversal, with one IF/ELSE branch per bar. Breaks of fixed swing levels latch direction; execution is at the close. The repeated Ch05/Ch08 program is one algorithm. Its unusable CD swing0 default is replaced by explicitly adjustable research2.5%, borrowed from TSM Swing. |
| `kftsm_swing` | PDF147,149; Ch05 Swing method | CD type0/swing2.5%. Evaluate old fixed levels before today's update. The long IF precedes the short ELSE, conditioned on the pre-close position. Two sequential update blocks test reversal before extending extrema. This ordering and extremum-scaled threshold distinguish it from `kftsm_swg`; no intraday path is assumed. |
| `kftsm_raschke_first_cross` | PDF276; Ch08 Raschke 1st cross | SMA3 minus SMA10, then SMA16 of that oscillator. A zero crossing resets setup; the first pullback arms a higher-low long or lower-high short at close. A favorable close arms the held-position exit stage; a later adverse close exits. These are Kaufman's suggested exits, not Raschke's own missing exits. The CD's upward short-side crossing resolves the printed typo. Same-close order arbitration is declared as one reversal. |
| `kftsm_outside_day` | PDF125; Ch04 Outside day | Prose baseline1 prior day/3 bars/outside-day extreme stop. Both high and low strictly exceed the prior range, with an outside close. Entry at close; stop starts next day. Strict boundary penetration and boundary/gapped-open fill approximate the unspecified stop offset; no tick size is inferred. Same-direction signals do not reset the clock. CD2/2/prior-range stop and repeated clock resets are different from this prose-based research contract. |
| `kftsm_fc_bb_abx` | Ch08 FC Bollinger ABX; PDF269 band context | SMA21 and population SD21; outer±2SD and inner±1SD. Signals at close fill at the next observed open. Two contracts become equal half-position research weights; this does not establish account capital, contract value or currency P&L. Repeated inner crossings may exit the remainder because the code has no first-exit latch. Opposite entry closes the remainder before the new position. |

The five adaptive variants share the actual CD driver's conjunction: current one-based bar exceeds period+5, absolute trend change strictly exceeds .1 times the population SD of the **previous20** trend changes, and also strictly exceeds mfilter0. Today's change is excluded from that SD window. Earlier actual observations warm indicators, but the selected test starts flat. There is one position per symbol and no pyramiding. At default periods the first eligible observed indices are at least21 for KAMA/R2/RSI,30 for SmoothedAverage and70 for VIDYA; unavailable changes cannot satisfy the filter.

Both swing contracts leave unestablished fixed levels unavailable instead of trading against the source variables' initial zero. Nifty is a declared pure-price cash-index research application. SPY is an actual ETF; RELIANCE and AAPL are actual individual stocks, not an index substituted for stocks or a historical population. Financing, borrow, dividends, portfolio selection, capital sizing, options premiums, futures contracts, delivery/roll events, point values and currency conversion are outside the verified interpretation. Nonzero mfilter and altered periods, thresholds or stop references remain research variants.

## Scoped correction and independent evidence

The reviewer corrected `kftsm_outside_day` so an opposite qualifying final close liquidates the existing position while the shared order builder suppresses a new entry beyond the test window. The previous branch suppressed both actions. A focused Rust regression and actual final-opposite-close cases on all four series verify the correction. Its existing catalogue identity adds zero new algorithms. The reviewed VIDYA history correction and ABX signal-index correction are also changes to exact existing IDs, not additions.

Fresh compiled Rust and the independent45-digit Decimal source Python agree on **10,015 chronological records across40 nonempty method/series cases**, January2010 through September11,2020. Every case has positive trade records. Comparisons cover bar indices, prices within max(1e-8 absolute,1e-12 relative), direction, entry/exit modes, exit signal indices, terminal marks and partial fractions. There are102 additional endpoint, actual next-open, opposite-final-close and future-truncation checks. These are excluded from checked-trade counts. The reviewer manually examined three real Nifty trades and nearby non-signals for each ID, including filter rejection, same-direction suppression, fixed-level ordering, staged exits, timed/stopped exits and weighted ABX fills.

Partial ABX exits retain their actual bar, price and weight. A final remainder is a mark with `open=true`; it is not an executed liquidation or a closed trade. Close-based final entries are suppressed, whereas an earlier ABX close signal can fill at the selected final open. Sequential-position and half-weight totals were checked. The selected real windows contain no high=low bars. The ten sources do not specify Hochheimer's locked-bar exclusion, so it is not inserted into these algorithms; exchange limit-day execution or liquidity is not certified. A bounded preservation regression compared the prepared-base and final Rust outputs for the excluded seventeen methods across68 cases, with exact output equality and zero contribution to the reviewed-trade count. Their calculations and instrument types remain outside this approved change set.

Focused Cargo filters are `books::kftsm::tests` and `instrument_policy::tests`, with14 and13 passing tests respectively. Contract/premium tickers and an unclassified index identity remain rejected. All three supported price classes are compatible with these explicitly declared research applications; they are not made artificially incompatible. The smoke cases therefore include actual ready stocks/ETF/index, an actual loaded unclassified Indian index rejected by policy, and separate policy tests rejecting futures/options identities. The host must run the full suite and temporary engine/API/public-upload gates before publication.

## Held systems and counting

Donchian Four-Week remains held. PDF177 requires four **full calendar weeks** and price-triggered reversal; a daily bar crossing both bounds cannot establish which trade occurred first. The old skip-both/one-reversal convention does not resolve that missing path, and cash prices cannot supply futures rolls. Related calendar-week and20-trading-day registrations are not new algorithms.

Parabolic remains held. PDF274–275,609 gives a trade-extreme recursion, while the inspected CD function continues with the prior bar's high/low and the delivered wrapper is an indicator. Default AFmax also differs between the CD indicator and prose. This review does not certify the current order wrapper or its reversal fills. The existing Wilder SAR family is not counted again. Fixed-box point-and-figure and delivery-period variants retain their earlier holds; no unsupported instrument policy is widened.

Nine accepted IDs already existed in the prepared base; the RSI ID is a declared driver extension. **New original book algorithm count is0.** This is an independent verification/adaptation review, not a claim that ten registrations create ten new algorithms. Original source performance, optimal parameters, source-study statistics and vendor quote/corporate-action accuracy are not reproduced or certified.

## Source and actual input identity

The private handoff records absolute actual source/input paths and exact candidate/module/ledger digests. Public source identity is recorded here without copying source text, prices, PDFs or generated ledgers into Git.

| Actual source | SHA-256 |
|---|---|
| Supplied TSM4 PDF | `aa77db8f22dd277a94b8988e5f5050f3461897d20d6c4a2049094420e7885dae` |
| Ch17 Adaptive systems | `3560036e7d23289992030781b6d41870248d191bf8372a622e86958fc6d72263` |
| Ch17 KAMA function | `0930a9f3540ce5bb3fa3d34343b09bc139a34f00a8d658bbc9174f0b5a88cbbe` |
| Ch17 VIDYA function | `59a24ca28a3136705bd48608d9c29c4fcd26ef2d5a8f2c0f99c6413bb858b089` |
| Ch17 Adaptive R2 function | `92a22547b81f391464a95573a4075dd75ed3748dd1a014a0f1577ff4b85b5612` |
| Ch17 Adaptive RSI function | `0a09cb79dbfe046c44daf4fcdbfddbe18f4b223e7493ad8788b20fc0e7be584a` |
| Ch05 Swing System | `bc77b086c6d9102bb2b50649de45b26c8762b163ca82f8f9c465e3bed9931b04` |
| Ch05 Swing signal | `ff68eff625da490c31c7166c999271bf0dd3af787717dfad50a687fc705d73ca` |
| Ch05 Swing method | `f91c254c35a7fdf91b04e46a50433b64252b01198690b9b2d7fdb379e0d4254e` |
| Ch08 Swing breakout | `136f046ca4af22885cab16352f464ad4a08ef124b8d17e041232be9e9c0563ea` |
| Ch08 Swing signal | `d0f7165d8fc8156adbca38d1a24c54aea1ed43a971cbd96d471a131ec408e948` |
| Ch08 Raschke 1st cross | `9a8e82b8a7373cb9bb877b62c363fd67ec370011af2227399fd96a937a56b2f6` |
| Ch04 Outside day | `bd8e7ed3292bee533988359aba78bdd1a295b19d13527046affee9dcd5282c3a` |
| Ch08 FC Bollinger ABX | `839ec82c3af379f8c0312bda28fee81c9c66347eea7e76d9b6e9b67b51a0c41f` |
| Ch17 Parabolic function, held | `f53afdf9336f10caa418d955ffa53a1e68b7d0136fd4a2d01d84b332302c13d9` |
| Ch17 Parabolic indicator, held | `da2336a908db723dd14e634e34f565d6f9347c67428fe3e42c0103edfbc816f7` |

| Actual input file | Read-only warmup coverage | SHA-256 |
|---|---|---|
| Index CSV, selected NIFTY50 | January3,2000 onward; older observations excluded | `a6a135e8dcdc5d75b0178c292824c4f16845d8531f0c144bdf4e35a30eae874b` |
| US CSV, selected SPY | January29,1993 onward | `edc711af4b2d25bc4bbc6d7cc78dc6a6028a57ae998d7ebea29a6086d279f898` |
| Actual RELIANCE native CSV | January3,2000 onward | `6b9b89406825efb7b0fdd87b97f1ace6fd8cc0532603441f79f35041a64b052c` |
| Actual AAPL S&P500 native CSV | December12,1980 through September11,2020 | `c220fe04bfd335ae85d6e1ad19464b3aa58026473f102f47012d4618f3bc5319` |

Native smoke tickers are derived from the actual server manifests: `basket:nifty50:RELIANCE` and `basket:us_sp500:AAPL`. Other AAPL basket copies end September4,2020 and are not substituted. Current constituents and current/past filename unions do not establish historical membership. Actual data, manual traces, full ledger JSON, source extracts, harnesses and command logs remain private. No reviewer commit, push, deployment or original-share write is authorized by this document.
