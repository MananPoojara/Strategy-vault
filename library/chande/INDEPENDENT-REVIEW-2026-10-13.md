# Independent Chande source review, 13 October 2026

The accepted subset is `ch_bta2_ma_stack` and `ch_bta2_65sma_ravi` from
Tushar Chande's **Beyond Technical Analysis** (Wiley, 1997). These are declared
single-symbol price-return research applications. They do not reproduce the
book's futures P&L or reported performance. Source correctness, executable
rule verification and live publication are separate: final host and publisher
gates remain pending.

The independent reviewer read the actual complete PDF, printed pp.20–21,
77–80 and 93–99, and visually inspected the supplied scanned **The New
Technical Trader** PDF, printed pp.156–157 and 180–183. No companion CD was
supplied or found in the immediate source directory. Source performance
tables and historical trades were not prerequisites for these concrete rules.

## Rules and declared interpretations

- **SMA agreement**, printed pp.20–21 (PDF pp.29–30): arithmetic SMA3 and
  SMA4 must both exceed SMA65 for a long at the current close, or both be below
  for a short. Full opposing agreement reverses. Symmetric reversal is the
  disclosed interpretation of the example's two rules each for long and
  short and its discussion of delayed exits. Mixed or equal agreement holds.
  The example explicitly has no initial stop. SMA3 through SMA7 is a printed
  variant. Other exposed lookbacks are declared research parameters.
- **RAVI reversal**, printed pp.77,80,94–97,99 (PDF pp.85,88,102–105,107):
  three consecutive closes above/below their respective completed SMA65,
  plus `abs(100*(SMA7-SMA65)/SMA65) > 1`, enter or reverse at the next
  observed open. Equality breaks confirmation. Weak RAVI alone does not exit.
  The stop-free, always-invested model in Figures 4.17–4.18 is distinct from
  Table 4.4's dollar-stop experiment. A 1.5% barrier is printed; 0.5% without
  that stop is a research variant. Any nonzero/nonfinite legacy `stop_pts`
  request is rejected, rather than interpreting futures dollars as cash points.

Both methods use complete windows without recursive seeds. Indicators retain
available prestart observations; NIFTY history starts in January 2000 before
warmup. Each test starts flat, with no prestart pending order. One unit and one
position per actual symbol are used, with no pyramiding or portfolio sizing.
Opposite reversals share an exit/entry fill; no additional exit-bar lock or
cooldown is introduced. Same-direction signals are ignored. Last-bar close
orders can fill; next-open orders require a following observed bar. An existing
terminal position is marked at the last close with `open=true`.

Selected actual index, ETF and individual-stock prices are declared instrument
adaptations of these scale-independent rules. NIFTY short exposure is
hypothetical. Supplied adjustment bases are preserved; dividend, borrowing,
financing, contract multipliers, rolls, tick values and dollar trading costs
are not inferred. RELIANCE and AAPL are actual stocks, SPY is an actual ETF,
and none is replaced with an index. Present basket selection does not establish
historical membership. No original pooled universe is reconstructed.

## Independent verification and identity

A fresh review-side Python oracle used exact rational window means and a
sequence of executable direction changes. It imported no conversion oracle.
The review freshly compiled the actual final `chbta2.rs`, `chntt.rs` and
unchanged `connors.rs` through the diagnostic Rust driver. Across four actual
series and three parameter variants for each method, **1,554 trade records in
24 checks matched with zero row mismatches**. Prices were compared within
1e-8; dates/indices, direction, fill mode, unit weight, exit signal, reason and
terminal open flag were checked. Eight causal prefixes and four real unfilled
last-signal checks passed and are excluded from the record total.

The reviewer manually inspected three completed trades per method on both
NIFTY and SPY, including nearby mixed agreement, insufficient confirmation,
weak RAVI and suppressed same-direction signals. Private evidence retains
the observed windows and calculations. The 38 focused Cargo tests passed,
covering strict thresholds/equality, warmup, final-close reversal, sample-end
orders, prestart orders, rejected cash stops and excluded-method regressions.

The common trade window is 2010-01-01 through 2020-09-11. Actual indicator
history begins 2000-01-03 for NIFTY and RELIANCE, 1993-01-29 for SPY, and
1980-12-12 for AAPL. No unsupported later US-stock date is used. Input hashes
identify the full supplied files; the index/ETF files are selected by ticker.
Raw books, text, prices, window calculations and ledger/result JSON remain
private and outside Git.

| Actual source | SHA-256 |
| --- | --- |
| Beyond Technical Analysis – Complete PDF | `c94896d277a2477bef43027068fc1f114ea62d317bbbc87c856e051ab099ee6a` |
| Chande Kroll – The New Technical Trader PDF | `28453810cfa84ecc256fa0f5af4fb2d65132a18bba69b0b8e2fc7a6257d8f9b1` |

| Actual input file / selection | SHA-256 |
| --- | --- |
| `idx_data.csv` / NIFTY 50 | `a6a135e8dcdc5d75b0178c292824c4f16845d8531f0c144bdf4e35a30eae874b` |
| `us_data.csv` / SPY | `edc711af4b2d25bc4bbc6d7cc78dc6a6028a57ae998d7ebea29a6086d279f898` |
| `9e95498193cc63fcd447.csv` / actual RELIANCE | `6b9b89406825efb7b0fdd87b97f1ace6fd8cc0532603441f79f35041a64b052c` |
| `c220fe04bfd335ae85d6e1ad19464b3aa58026473f102f47012d4618f3bc5319.csv` / actual AAPL | `c220fe04bfd335ae85d6e1ad19464b3aa58026473f102f47012d4618f3bc5319` |

## Exclusions, preservation and method count

SMA agreement adds one catalogue method. RAVI is an existing ID: the reviewed
cash/ETF/stock policy and rejected cash-point stop requests add zero new
methods. Threshold, lookback and instrument variants add zero algorithms.
`ch_65sma3cc` and `ch_cbpb_time` remain unchanged and excluded. All other
methods are outside this accepted subset; this review makes no entire-book
completion claim.

NTT's CMO-driven VIDYA requires actual contract ticks and a $1,500 stop
(printed pp.181–183). The closing-basis suggestion does not establish a
cash-unit replacement stop. Its thrust buy table supplies forward observation
horizons, not an executable exit (printed p.157). Actual breadth is also
unavailable in the supplied price-only series. Neither is accepted here.

The independent reviewer restored the three NTT pre-existing live catalogue
statuses that the conversion had changed. Their unchanged source-review
instrument policies continue to block performance; no unapproved instrument
type or calculation changes. The exact reviewed policy additions name only
the two accepted BTA IDs. No blanket book-prefix admission is used.

All three supported API instrument classes are allowed for these declared
pure-price adaptations. Negative API cases test incompatible futures-stop
requests on actual cash-index and ETF series; they must reject backtests.
Options and futures are outside the allowed-type policy, and underlying price
returns cannot stand in for option P&L. Final source/data/ledger hashes and
the exact API cases are in the private independent review contract. No stage,
commit, push or deployment was performed by this reviewer.
