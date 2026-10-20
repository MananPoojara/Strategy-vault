# Kestner stock conversion contract

These three candidates come from Lars Kestner, *Quantitative Trading Strategies* (2003): channel 40/20, printed pp.139–140; simple moving averages10/40, p.145; momentum80, p.150. Printed p.139 specifies completed-close decisions and next-session opening execution. Original dollar-volatility sizing (pp.135,137) and original mixed-instrument study figures remain separate from normalized selected-stock percentage research.

This is a source and calculation contract, **not admission or a completed backtest claim**. The candidates remain excluded from `reviewed_cash_rule`; their instrument policy allows no execution types until actual-stock accounting, API and data checks are accepted. Other Kestner variants inherit neither these conventions nor this review. No homepage/Excel completion count changes from this patch.

## Explicit conventions

- Channels include the current close:40 genuine closes first exist at zero-based index39. Inclusive extrema allow ties with a previous high/low. The prose does not publish indexed code or tie operators; this is a declared interpretation.
- If both40-close directions are simultaneously true, a flat account stays flat. An existing20-close exit is honored; no arbitrary opposing-entry priority is invented. An unambiguous opposite40-close entry closes and reverses at the same next actual open. A20-close exit wins over a same-side entry; same-side re-entry requires a later signal. These collision priorities are declared research conventions, not uniquely established source instructions.
- SMA10/40 needs an observed cross using two valid average comparisons: first possible signal index40. Previous difference≤0/current>0 signals long; previous≥0/current<0 signals short. Current equality alone neither exits nor enters. No position is created from the first average relationship alone.
- Momentum compares `C[i]` with `C[i−80]`, requiring81 observations and first signal index80. Strict greater/less signals long/short. Equality retains an existing holding and creates no order while flat.
- The selected window starts flat; supplied genuine earlier history supplies indicators. No predecessor prices or positions are fabricated. A last-close signal without a following supplied session cannot fill. An actual final-open entry may remain an open holding at the final close.
- Terminal marking is not a closed trade. The ordinary percentage account currently deducts a separately disclosed estimated liquidation-side cost on that mark. Closed-trade counts and arithmetic return sums exclude it.
- A same-open reversal pays the old exit and new entry as separate sides. The account finishes the old position first, allocates the resulting normalized equity to the new position, then marks the new position at that day's close.
- Source default settings are fixed. Unknown, changed, nonfinite, malformed or decimal values that merely round to a default fail before results. Optimization accepts only singleton default grids. No fitting to book performance figures is performed.

## Evidence and remaining gates

The source-event fixtures were hand frozen by a reviewer before reading Rust or reference code. Fifteen executable cases cover boundaries, ties, gaps, reversals, equality and pending final signals; one additional source diagnostic identifies unspecified collision priorities. Separate explicit tests cover our declared collision handling.

The account fixtures cover13 synthetic event schedules at four fee/slippage settings, including nonpositive intermediate/exit accounts. Their chronological exact-rational oracle was frozen before Rust fixture capture; the reviewer had read legacy accounting, so this is **not blind independent account verification**. All fixtures are synthetic arithmetic, never observed prices or reported performance.

Still required before publication: bounded actual qualified US and Indian stock captures through the normal percentage pipeline; source versus fill/mark clocks in actual API output; native identity, dates, full warmup, cost and parameter request guards; index/ETF/derivative/basket negative admission; regression checks; controlled backend release; then refreshed evidence, UI counts and the simple Downloads workbook. Passing these synthetic tests does not satisfy those gates or reproduce the original book's dollar portfolio results.

## Current validation record

The focused synthetic/guard suite passes70 checks. The broader data-independent Rust suite passes788 tests, with four opt-in tests ignored and one legacy whole-catalogue equity test excluded. The initial broad invocation of that legacy test failed before calculation because its private default `idx_data.csv` path was absent. This is recorded as an environment exception, not reported as a passing full-corpus backtest.

An explicit private capture then computed24 candidate runs: three methods on AAPL/MSFT (1990–2001) and RELIANCE/HDFCBANK (2001–2010), at zero costs and fees3bps plus slippage2bps per side. Each capture uses the checked native whitelist loader and ordinary percentage account, with full source history retained. These24 runs are **parameter/context captures, not24 distinct strategies or independently verified market results**. Their data and outputs remain outside Git. Independent trade/account comparison and public API/instrument admission still remain before availability counts can change.
