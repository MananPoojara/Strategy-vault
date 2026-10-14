# Indicators Lab — Phase 6 (every TradingView and AmiBroker indicator) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Put every TradingView built-in indicator and every AmiBroker built-in indicator function into the lab catalogue. Indicators that can be computed from admitted data become rule-usable series. Each one has hand-computed tests, and its TradingView parity fixture stays pending, as in Phase 1. Indicators that need data the lab lacks are still listed, with `computed:false` and a visible note such as "not available yet — needs …". No values are faked, and no book result changes.

**Architecture:** All changes stay inside the indicator layer, `Context/guru/backend/src/lab/`. Twelve new family files follow the Phase 1 pattern of `fn(&Bars, &Params) -> Output` plus a `DEFS` table. `listed.rs` holds every listed-only entry. `sources.rs` maps each catalogue id to its TradingView and AmiBroker names, so the catalogue JSON can expose both. `catalogue::Needs` gains data flags and `IndicatorDef` gains `note`; both changes are additive, so the catalogue JSON only gains fields. Market breadth (advances/declines) is already loaded by the engine from `extra_data/<market>/<MARKET>_ADV_DEC.csv` into `Ctx::extra`. A new `Bars::from_ctx` carries it into the lab, and breadth indicators then compute for India (NSE) and US (NYSE) daily bars.

**Tech Stack:** Rust 2021, std only (no new crates), `cargo test` with `CARGO_BUILD_JOBS=2`; Python 3 std-lib guard scripts; git.

**Spec:** `docs/superpowers/specs/2026-10-14-indicators-lab-design.md` (§5 catalogue, §7 error handling, §8 testing, §9 phase 6). Earlier plans set the format and conventions: `docs/superpowers/plans/2026-10-14-indicators-lab-phase1-engine.md` (families, generic tests, parity harness), `…-phase2-rules-engine.md` (rules, `series_ref`, error contract) and `…-phase5-engine-gl.md` (runs before this phase and owns `lab/api.rs`).

**Planning evidence:** An independent Python reference was written while planning (scratch only, not committed). It reimplements `ta.rs` (TradingView NaN compaction for `sma`/`ema`/`rma`, the restart-after-NaN rule for `highest`/`lowest`, `tr`, `linreg`) and every new indicator below. It produced every expected value in the tests and every `first` (warm-up) index, using two random-walk seeds of 1,000 bars with real dates. The candlestick port was checked against all 44 crafted cases: each case fires its own pattern on its last bar, and a prefix of 15 plain bars fires nothing. The Rust code blocks below were **not** compiled during planning. When a test fails, the implementer must reconcile the code against the formula table, which is the specification; the test values must not be edited to fit the code.

## Global Constraints

- Branch `Nandini`; never `main`; no force-push; no git worktrees; never push (the user pushes). Commits carry no `Co-Authored-By` trailer. If `.git/index.lock` exists, wait and retry; never delete it while another process (Phase 5) is committing.
- Book results never touched. The protected files get no edits: `connors.rs`, `books/**`, `readiness.rs`, `gate.rs`, `instrument_policy.rs`, `baskets.rs`, `accounting.rs`, `trade_validation.rs`, frontend `compat/*` and `ResearchPages.tsx`. `main.rs`, gateway `policy.rs` and `custom-pages.ts` are additive-only and this phase does not touch them at all.
- The scope is the indicator layer only: `Context/guru/backend/src/lab/**` and `docs/indicator-parity-fixtures.md`. `lab/api.rs` is touched in exactly two places (Task 1 `catalogue_json`, Task 2 `Bars::from_ctx`), and it is touched only after Phase 5 has landed on `Nandini`.
- Rust std only, no new crates. `CARGO_BUILD_JOBS=2` always. No whole-corpus backtests. The NAS is read-only.
- NaN means "no value", and every comparison with NaN is false. Lab code never panics on user input; a bad parameter is a typed `invalid` error.
- Formulas follow the TradingView built-in. Where the AmiBroker built-in differs in formula (not only in seeding), a `variant` parameter selects it: `0` = TradingView, `1` = AmiBroker (Decision 9).
- Boolean outputs (patterns, signals) are `1.0`/`0.0`, and they are `0.0` while their inputs are missing, as Pine bools are (Decision 13).
- Test tolerance is `testdata::close_eq` (1e-9 relative). Expected values in tests come from the formula tables; never edit them to match code.
- Raw data, `.env`, generated results and TradingView exports stay outside Git.

## Review Focus

1. **Breadth gaps or a market without breadth published as numbers.** A day with no advance/decline value must give NaN, never 0. The A/D line must continue across the gap rather than restart. A market with no `*_ADV_DEC` file (global/crypto, or a missing extra dir), or any W/M timeframe, must return `needs_data` with the breadth note. Tests: `breadth_gaps_stay_nan_and_cumulative_lines_continue` and `breadth_needs_daily_market_data` (Task 2).
2. **A listed-only entry used in a rule or chart.** It must return typed `needs_data` (missing data) or `unsupported` (look-ahead, drawing, unpublished formula, user code) with its human note in the message, and never a series. Test: `listed_entries_refuse_with_their_note` (Task 1).
3. **Calendar and period boundaries.** A holiday-shortened week, a year change for YTD, a quarter change for anchored VWAP, and a bar dated Sunday (Phase 5 CFDs) must each group into the right period. Pivots must use only the previous completed period. Tests: `pivots_use_the_previous_completed_week_across_a_holiday_week`, `period_keys_split_weeks_months_quarters_years` (Tasks 11–12), and the generic causality tests.
4. **Extreme or contradictory parameters.** `rwi` with min > max, `percentile` at 0 or 100, an unstable `iir2`, or `hamming_ma` length 2 must each give an `invalid` error or finite numbers, never a panic, inf or a silent swap. Tests: `rwi_rejects_min_above_max`, `percentile_bounds_pick_min_and_max`, `iir2_unstable_coefficients_stay_finite_or_nan` (Tasks 10, 6, 3), plus the generic `every_indicator_at_parameter_bounds_never_panics`.
5. **AmiBroker and TradingView silently disagreeing.** Choosing `variant=1` must change exactly the documented formula (StochK/D as a ratio of sums; TRIX as a percent ROC; PercentRank as a strict count), and `variant=0` must leave every existing Batch A value byte-identical. Tests: `stoch_variant_ab_is_ratio_of_sums`, `trix_variant_ab_is_percent_roc`, `variant_zero_is_unchanged` (Task 17), `percentrank_variant_counts_ties_differently` (Task 6).

---

## Decisions (open points resolved without the user; each gives its reason and the cost if wrong)

1. **What counts as "every indicator".** The TradingView sources are:
   - the help-centre "Technical indicators" folder (209 articles, including on-chain, derivatives and fundamentals metrics);
   - the TradingView Charting Library "Indicators List", which includes legacy built-ins absent from the help centre (Accumulative Swing Index, Chaikin Volatility, Majority Rule, Volatility O-H-L-C …);
   - the candlestick-pattern folder (44 patterns plus "All Candlestick Patterns");
   - Pine `ta.*` functions, only where AmiBroker has the same concept as a built-in (`percentrank`, `percentile_*`, `highestbars`/`lowestbars`).

   The AmiBroker sources are the AFL Function Reference categories Indicators, Moving averages/summation, Statistical functions, Basic price pattern detection, Composites and Lowest/Highest, plus every indicator function called by the book AFL files (Bandy QTS 82 files, Alvarez Factor 7 files). AmiBroker's Charts → Indicators folder could not be listed online (no page publishes it, and no AmiBroker install is on the NAS), so it is not enumerated beyond names already present above. *Cost if wrong:* a missing AmiBroker chart formula has no catalogue entry until someone lists that folder from an install; adding one is a one-row change per indicator.
2. **One catalogue entry per concept.** Every other name for a concept is an alias in `sources.rs`. For example, Price Channel maps to `donchian`, Volume Oscillator to `pvo`, Moving Average Adaptive to `kama`, Linear Regression Curve to `lsma`, Volatility Close-to-Close to `hv`, the five MA-cross built-ins to `ma_cross`, and 52 Week High/Low to `highest`/`lowest` with length 252 on D or 52 on W. Average Price, Median Price and Typical Price are the existing rule price operands `ohlc4`, `hl2` and `hlc3`. *Cost:* a user searching for an alias finds it through the `tv`/`ab` fields, not through the id.
3. **Market breadth is computed (class A), not listed.** The engine already loads `NSE_ADV_DEC.csv` (India, 1999→) and `NYSE_ADV_DEC.csv` (US, 1965→) into `Ctx::extra` as `<MARKET>_ADV_DEC.<Column>`, aligned to each ticker's dates with NaN on days that have no value. Book strategies already use these series (`books/apwms.rs:94–98`). Breadth is daily only, and W/M breadth returns `needs_data`, because neither TradingView nor AmiBroker defines a weekly aggregate of advancing issues. *Cost:* if the owner rejects breadth for the lab, delete `breadth.rs` and its five entries.
4. **Breadth columns.** The lab uses Advances, Declines, Unchanged, AdvVolume, DecVolume and UnchVolume (UnchVolume exists for the US only; India has NaN). It finds the market by the key suffix `_ADV_DEC.<Column>`, which is unique per engine instance. India's `UpVolume`/`DownVolume` columns duplicate AdvVolume/DecVolume and are ignored. TRIN is recomputed from its components rather than read from `NSE_TRIN.csv`/`NYSE_TRIN.csv`, so the A/D ratio and TRIN always agree. *Cost:* a rounding difference against the precomputed TRIN files.
5. **Second-symbol indicators (class B) are catalogue-only.** These are Correlation Coefficient, Correlation - Log, Ratio and Spread (AmiBroker Correlation/RelStrength/Foreign). Spec §1 runs one instrument per request. Aligning two instruments across calendars and currencies is an admission question the spec leaves open, and the only loader seam (`lab/api.rs`) belongs to Phase 5 tonight. *Cost:* four concepts show "not available yet — needs a second instrument". A follow-up adds a `vs` operand field and a second `select_context` in `api.rs`.
6. **Look-ahead functions are listed and never computed.** These are AmiBroker `Zig`, `Peak`, `Trough`, `PeakBars` and `TroughBars`, TradingView Zig Zag, and VWAP Auto Anchored (its anchor moves when later bars arrive). The note reads "looks ahead — chart display only, not usable in rules". *Cost:* none for rules. A chart-only zig-zag can be added later behind a display-only flag.
7. **Rob Booker ADX Breakout, Knoxville Divergence, Reversal, Ziv Ghost Pivots and Missed Pivot Points are listed, not computed.** TradingView's help pages for these five describe them only in words. AGENTS.md forbids publishing results whose rules are unresolved. *Cost:* five TradingView indicators stay unavailable until someone copies their Pine source from TradingView's editor into a follow-up task.
8. **On-chain (62), derivatives (13) and fundamentals (3) metrics each get their own listed entry.** The user asked that nothing be hidden. *Cost:* a long list, which the UI groups by `category`.
9. **AmiBroker versus TradingView.** Differences that affect only the seed are not given a variant: AmiBroker RSI seeds the Wilder sums at 0, and AmiBroker DEMA/TEMA/Wilders seed with the first value. These converge geometrically, the lab warms every indicator on the full admitted history (Phase 2 decision 6), and the alias text records the difference. Real formula differences get `variant` (0 = TradingView, 1 = AmiBroker):
   - `stoch`: AmiBroker StochK/StochD use a ratio of sums (`100·Σ(c−LL)/Σ(HH−LL)` over Ksmooth);
   - `trix`: AmiBroker uses a percent ROC of the triple EMA of close, with no log and ×100;
   - `percentrank`: AmiBroker counts strictly lower values, TradingView counts lower-or-equal;
   - `percentile`: variant 1 is the nearest rank, which equals AmiBroker `Median`'s lower median at pct 50.

   *Cost:* if AmiBroker's StochK or TRIX formula differs from the published MetaStock-style definitions, which AmiBroker's own pages do not spell out, `variant=1` is wrong until a parity fixture from AmiBroker corrects it. The definition string says "AmiBroker form per standard definition; unverified".
10. **Arbitrary-array inputs are out of scope:** RSIa or CCIa on non-close arrays, HHV on any array. Lab indicators keep their declared sources. Adding a `source` selector is a later feature (see the AFL utilities table). *Cost:* AFL that applies RSI to, say, a foreign close cannot be expressed yet.
11. **Defaults the TradingView help page does not state are declared in the definition string:** Median 3/14/2, Trend Strength 14, Hamming 10, ADR 14, Majority Rule 14, Standard Error 14, Standard Error Bands 21/2/3, Linear Regression Slope 14, PMO 35/20/10, Chaikin Volatility 10/10, Volatility Index 7/3, Volatility (ZT, OHLC) 10/252, ASI limit 10, A/D Ratio (Bars) 9, Price Oscillator 10/21, Klinger 34/55/13, Chop Zone 30/34. *Cost:* a different TradingView default only changes the starting value a user sees; the parameters stay adjustable.
12. **Declared adaptations, each stated in its definition string with parity pending:**
    - the Hamming-window weights `0.54−0.46·cos(2πk/(n−1))`;
    - Garman–Klass for "Volatility O-H-L-C";
    - Wilder's Volatility System (ARC = mult·ATR) for the charting-library "Volatility Index";
    - Ehlers' Center of Gravity for Bandy's undefined `CGOscillator`;
    - Performance horizons of 5/21/63/126/252 bars, with YTD measured from the last close of the previous calendar year;
    - Moon Phases from Meeus ch. 49 (15 leading terms, UTC), flagged on the first bar dated on or after the event;
    - Technical Ratings with trend = close vs SMA50, Ichimoku as plotted, a group that stays NaN until every component exists, and VWMA dropped when there is no volume;
    - RCI ties given averaged ranks;
    - linear-interpolation percentile at position `(n−1)·p/100`.

    *Cost:* a value mismatch, which the parity fixtures will reveal per indicator.
13. **Booleans are 1/0 and 0 during warm-up, as Pine bools are.** This applies to candles, `bar_patterns`, `rsi_divergence` and the `ma_cross` signal (NaN only until both MAs exist). Price-valued sparse outputs (`fractals`) stay NaN between events, like `pivot_high`. *Cost:* a rule `x > 0.5` behaves as on TradingView, and a chart shows 0s rather than gaps.
14. **`sparse: true` marks indicators whose warm-up or gaps depend on the data.** These are `rvi` (TradingView's EMAs count zeros before the stdev exists), `fractals` (event-valued) and `ad_ratio_bars` (a window with no down bar is NaN). The generic warm-up/gap check skips them, and their own tests pin the values. Candles are not sparse: they are 0/1 from bar 0. *Cost:* an unexpected gap in these three would not be caught generically; their value tests cover it.
15. **Calendar and period indicators read the bar's date string.** W/M bars use their last daily date. Weeks run Monday–Sunday (`resample` key); quarters are calendar quarters. `testdata::walk` gets real consecutive dates from 2020-01-01, and the generic warm-up test uses 1,000 bars (the Special K signal first appears at 922).
16. **Sessions on daily bars.** Session VWAP stays listed (it needs intraday bars). VWAP and TWAP anchored to Week/Month/Quarter/Year are computed from daily bars as `vwap_period` and `twap`.
17. **Pivot Points Standard.** The default period is W on daily bars (TradingView "Auto" depends on chart resolution). Types are 0 Traditional, 1 Fibonacci, 2 Woodie, 3 Classic, 4 DM and 5 Camarilla. Levels come from the previous completed period's H/L/C (and open for DM). Woodie uses the current period's first open, which is known at that bar's open. Levels a type does not define are NaN.
18. **The `ma_ribbon` VWMA type needs volume only when chosen.** Type 4 on an instrument with no volume returns `needs_data`; the entry's `needs.volume` stays false.
19. **Volume Profile counts as one concept:** TradingView Fixed Range, Visible Range and Session Volume Profile, plus AmiBroker `PlotVAPOverlayA`. *Cost:* none.
20. **Candlestick formulas come from TradingView's "All Candlestick Patterns" Pine v4 source.** The copy used is github.com/shunjizhan/all-candlestick-pattern-indicators `all-patterns.pine`, which TradingView itself does not publish as a file. Its quirks are kept: Tweezer Bottom uses `C_UpTrend[1]`, as the source does. The trend parameter is `trend` with 0 = SMA50 (default), 1 = SMA50+SMA200 and 2 = none. *Cost:* if that copy drifted from TradingView's current built-in, the parity fixtures will show it.
21. **FIR, IIR and FFT are listed.** AmiBroker FIR/IIR take a coefficient list, which is not expressible as lab parameters, and the lab runs no user code. FFT returns a spectrum, not a bar series. Bandy's fixed-form `IIR2` is computed (three parameters).
22. **The catalogue size is pinned by a test.** `catalogue_matches_the_phase6_master_list` asserts 299 entries (183 computed, 116 listed) and the per-source counts (TradingView only 234, AmiBroker only 23, both 41, lab-only 1). A later batch must update these numbers deliberately.
23. **Parity fixtures for the new indicators stay pending, as in Phase 1.** `docs/indicator-parity-fixtures.md` gains every new id, its parameters and its output order, and nothing is described as TradingView-matched.
24. **Task order follows the dependencies.** The catalogue extension comes first; then breadth (it needs the `api.rs` seam, so it runs after Phase 5); then families in any order; Technical Ratings after the oscillator tasks (it calls `bbp`); then AmiBroker variants; then the gate. Execution is sequential: one implementer per task, then a reviewer.

## Seams with Phase 5 (engine-gl)

| Seam | Phase 5 | Phase 6 | Rule |
|---|---|---|---|
| `lab/api.rs` `catalogue_json` | may wrap bodies with the `provisional` prefix in `serve()`/`response()` | Task 1 edits only the per-indicator `format!` inside `catalogue_json` (adds `needs` keys, `note`, `tv`, `ab`) | Run Task 1 after Phase 5's last commit. Re-read the function before editing; do not touch Phase 5 lines |
| `lab/api.rs` `Frames::new(Bars::from_rows(ctx.r))` (3 sites today: series, run, optimize) | modifies the surrounding functions | Task 2 replaces each with `Frames::new(Bars::from_ctx(ctx))` | `grep -n "Bars::from_rows(ctx" Context/guru/backend/src/lab/api.rs` and replace every hit; the gl engine has no `*_ADV_DEC` extras, so breadth there is `needs_data` |
| `lab/bars.rs` | Phase 5 tests call `Bars::from_rows(&rows)` | Task 2 adds a field with `None` default and a new constructor | `from_rows` signature unchanged |
| `lab/mod.rs` | adds `pub mod markets;` | adds one `pub mod` line per new file | additive lines only |

## Sources (formulas and lists)

- TradingView help centre, Technical indicators folder (the list, and per-indicator articles cited in each task): https://www.tradingview.com/support/folders/43000587405-technical-indicators/
- TradingView Charting Library indicators list (legacy built-ins): https://www.tradingview.com/charting-library-docs/latest/ui_elements/indicators/Indicators-List
- TradingView candlestick patterns folder: https://www.tradingview.com/support/folders/43000570503-learn-more-about-candlestick-patterns/ and the Pine source copy https://github.com/shunjizhan/all-candlestick-pattern-indicators/blob/HEAD/all-patterns.pine
- Technical Ratings rules: https://www.tradingview.com/support/solutions/43000614331-technical-ratings/
- Pring's Special K: https://www.tradingview.com/support/solutions/43000773011-pring-s-special-k/ · PMO: https://www.tradingview.com/support/solutions/43000773010-price-momentum-oscillator-pmo/ · Trend Strength Index: https://www.tradingview.com/support/solutions/43000730926-trend-strength-index/ · Median: https://www.tradingview.com/support/solutions/43000644897-median/ · ADR: https://www.tradingview.com/support/solutions/43000695003-average-daily-range-adr-indicator/ · A/D Ratio (Bars): https://www.tradingview.com/support/solutions/43000644914-advance-decline-ratio-bars/ · Net Volume (uptick/downtick, needs intrabar data): https://www.tradingview.com/support/solutions/43000589192-net-volume/ · Bollinger Bars (chart style only): https://www.tradingview.com/support/solutions/43000742575-bollinger-bars/ · Rob Booker pages (no formulas): …/43000591333, …/43000591336, …/43000599840, …/43000594679, …/43000594678
- AmiBroker AFL Function Reference (categories): https://www.amibroker.com/guide/a_funref.html. Per-function pages used: `afl/{ama,ama2,ema,rsi,stochk,stochd,trix,percentrank,percentile,median,stdev,stderr,tsf,linregslope,linregintercept,correlation,rmi,rwi,rwihi,rwilo,skewness,kurtosis,oscp,oscv,trin,adline,cci,hma,ultimate,wilders,roc,sar,macd,nvi,pvi}.html`
- Book AFL, read-only on the NAS: `Books/Howard B. Bandy/QTS Book Code/*.afl` (`Figure_10_6.afl` APZ/DblSmoothEMA, `Figure_8_1.afl` IIR2, `Figure_A_29.afl` zscore) and `Books/Cesar Alvarez/Alvarez Factor/AmiBroker/*.afl`.
- Standard references for formulas TradingView does not publish: Wilder 1978 (ASI, Volatility System), Garman–Klass 1980, Meeus *Astronomical Algorithms* ch. 49, Ehlers *Cybernetic Analysis* (CG).

## Master list

**299 concepts.** By class: existing Batch A **64 computed + 2 listed** (`vwap`, `opening_range`; notes added); new **class A 119** (114 OHLCV-computed + 5 breadth); **class B 4**; **class C 110**. The class C breakdown is intraday 9, fundamentals 3, derivatives 13, on-chain 62, drawing/display 10, look-ahead 6, unpublished formula 5 and user-code 2; the two existing intraday entries bring the C total to 112. By source over all 299: **TradingView only 234 · AmiBroker only 23 · both 41 · lab-only 1** (`opening_range`). Among the 233 new entries: TradingView only 198 (of them 62 on-chain and 45 candlestick entries), AmiBroker only 23, both 12.

Class legend: `existing` = Batch A (unchanged, alias text added); `A` = computed from daily OHLCV of one instrument; `A (breadth)` = computed from the market's advance/decline series; `B` = needs a second instrument (listed); `C:<reason>` = listed with a note.

| # | id | TradingView name(s) | AmiBroker name(s) | Source | Class | Task | Note |
|---|---|---|---|---|---|---|---|
| 1 | `sma` | Simple Moving Average; Moving Average (charting library); Moving Averages | MA | both | existing | — |  |
| 2 | `ema` | Exponential Moving Average; Moving Average Exponential | EMA | both | existing | — |  |
| 3 | `wma` | Weighted Moving Average; Moving Average Weighted | WMA | both | existing | — |  |
| 4 | `smma` | Smoothed Moving Average | Wilders | both | existing | — |  |
| 5 | `swma` | ta.swma (Pine) | — | TV | existing | — |  |
| 6 | `vwma` | Volume-Weighted Moving Average (VWMA) | — | TV | existing | — |  |
| 7 | `hma` | Hull Moving Average | HMA | both | existing | — |  |
| 8 | `alma` | Arnaud Legoux Moving Average | — | TV | existing | — |  |
| 9 | `lsma` | Least Squares Moving Average; Linear Regression Curve | LinearReg | both | existing | — |  |
| 10 | `dema` | Double Exponential Moving Average (EMA) | DEMA | both | existing | — |  |
| 11 | `tema` | Triple EMA | TEMA | both | existing | — |  |
| 12 | `kama` | Kaufman's Adaptive Moving Average (KAMA); Moving Average Adaptive | — | TV | existing | — |  |
| 13 | `mcginley` | McGinley Dynamic | — | TV | existing | — |  |
| 14 | `rsi` | Relative Strength Index (RSI) | RSI; RSIa (on close) | both | existing | — |  |
| 15 | `macd` | Moving average convergence divergence (MACD) | MACD; Signal | both | existing | — |  |
| 16 | `stoch` | Stochastic (STOCH) | StochK; StochD (variant=1) | both | existing | — |  |
| 17 | `stochrsi` | Stochastic RSI (STOCH RSI) | — | TV | existing | — |  |
| 18 | `cci` | Commodity Channel Index (CCI) | CCI; CCIa (on typical price) | both | existing | — |  |
| 19 | `mom` | Momentum | — | TV | existing | — |  |
| 20 | `roc` | Rate of Change (ROC) | ROC | both | existing | — |  |
| 21 | `willr` | Williams %R (%R) | — | TV | existing | — |  |
| 22 | `mfi` | Money Flow (MFI) | MFI | both | existing | — |  |
| 23 | `tsi` | True Strength Index | — | TV | existing | — |  |
| 24 | `cmo` | Chande Momentum Oscillator (CMO) | — | TV | existing | — |  |
| 25 | `uo` | Ultimate Oscillator (UO) | Ultimate | both | existing | — |  |
| 26 | `ao` | Awesome Oscillator (AO) | — | TV | existing | — |  |
| 27 | `ppo` | Percentage Price Oscillator (PPO) | — | TV | existing | — |  |
| 28 | `trix` | TRIX | TRIX (variant=1) | both | existing | — |  |
| 29 | `kst` | Know Sure Thing (KST) | — | TV | existing | — |  |
| 30 | `dpo` | Detrended Price Oscillator (DPO) | — | TV | existing | — |  |
| 31 | `coppock` | Coppock Curve | — | TV | existing | — |  |
| 32 | `fisher` | Fisher Transform | — | TV | existing | — |  |
| 33 | `crsi` | Connors RSI (CRSI) | — | TV | existing | — |  |
| 34 | `dmi` | Directional Movement (DMI); Average Directional Index (ADX) | ADX; PDI; MDI | both | existing | — |  |
| 35 | `supertrend` | Supertrend | — | TV | existing | — |  |
| 36 | `sar` | Parabolic SAR (SAR) | SAR | both | existing | — |  |
| 37 | `aroon` | Aroon Indicator | — | TV | existing | — |  |
| 38 | `aroon_osc` | Aroon Oscillator | — | TV | existing | — |  |
| 39 | `ichimoku` | Ichimoku Cloud | — | TV | existing | — |  |
| 40 | `vortex` | Vortex Indicator | — | TV | existing | — |  |
| 41 | `chop` | Choppiness Index (CHOP) | — | TV | existing | — |  |
| 42 | `atr` | Average True Range (ATR) | ATR | both | existing | — |  |
| 43 | `bb` | Bollinger Bands (BB) | BBandTop; BBandBot | both | existing | — |  |
| 44 | `bbpct` | Bollinger Bands %b (%b) | — | TV | existing | — |  |
| 45 | `bbw` | Bollinger BandWidth (BBW) | — | TV | existing | — |  |
| 46 | `kc` | Keltner Channels (KC) | — | TV | existing | — |  |
| 47 | `donchian` | Donchian Channels (DC); Price Channel | — | TV | existing | — |  |
| 48 | `stdev` | Standard Deviation | StDev | both | existing | — |  |
| 49 | `hv` | Historical Volatility; Volatility Close-to-Close | — | TV | existing | — |  |
| 50 | `obv` | On Balance Volume (OBV) | OBV | both | existing | — |  |
| 51 | `accdist` | Accumulation Distribution (ADL) | AccDist | both | existing | — |  |
| 52 | `cmf` | Chaikin Money Flow (CMF) | — | TV | existing | — |  |
| 53 | `chaikin_osc` | Chaikin Oscillator | Chaikin | both | existing | — |  |
| 54 | `pvt` | Price Volume Trend (PVT) | — | TV | existing | — |  |
| 55 | `pvo` | Percentage Volume Oscillator (PVO); Volume Oscillator | OscV | both | existing | — |  |
| 56 | `efi` | Elder's Force Index (EFI) | — | TV | existing | — |  |
| 57 | `eom` | Ease of Movement (EOM) | — | TV | existing | — |  |
| 58 | `nvi` | Negative Volume Index (NVI) | NVI | both | existing | — |  |
| 59 | `pvi` | Positive Volume Index (PVI) | PVI | both | existing | — |  |
| 60 | `highest` | ta.highest (Pine); 52 Week High/Low (high side: length 252 on D, 52 on W) | HHV; Highest | both | existing | — |  |
| 61 | `lowest` | ta.lowest (Pine); 52 Week High/Low (low side) | LLV; Lowest | both | existing | — |  |
| 62 | `change` | ta.change (Pine) | — | TV | existing | — |  |
| 63 | `pivot_high` | Pivot Points High Low (high) | — | TV | existing | — |  |
| 64 | `pivot_low` | Pivot Points High Low (low) | — | TV | existing | — |  |
| 65 | `vwap` | Volume Weighted Average Price (VWAP) — session anchor | — | TV | C:intraday | 1 | existing listed entry; note added |
| 66 | `opening_range` | — | — | lab | C:intraday | 1 | existing listed entry (book-driven, not a TV/AB built-in); note added |
| 67 | `breadth` | — | AdvIssues; DecIssues; UncIssues; AdvVolume; DecVolume; UncVolume | AB | A (breadth) | 2 |  |
| 68 | `adl` | Advance/Decline Line | ADLine | both | A (breadth) | 2 |  |
| 69 | `ad_ratio` | Advance/Decline Ratio | — | TV | A (breadth) | 2 |  |
| 70 | `cvi` | Cumulative Volume Index (CVI) | — | TV | A (breadth) | 2 |  |
| 71 | `trin` | — | Trin | AB | A (breadth) | 2 |  |
| 72 | `ma_ribbon` | Moving Average Ribbon; Moving Average Multiple; Moving Average Triple | — | TV | A | 3 |  |
| 73 | `ma_cross` | MA Cross; MovingAvg Cross; MovingAvg2Line Cross; EMA Cross; MA with EMA Cross; Moving Average Double | — | TV | A | 3 |  |
| 74 | `gmma` | Guppy Multiple Moving Average | — | TV | A | 3 |  |
| 75 | `hamming_ma` | Moving Average Hamming | — | TV | A | 3 |  |
| 76 | `ama` | — | AMA | AB | A | 3 |  |
| 77 | `ama2` | — | AMA2 | AB | A | 3 |  |
| 78 | `iir2` | — | IIR2 (Bandy QTS book function) | AB | A | 3 |  |
| 79 | `envelope` | Envelope (ENV); Envelopes | — | TV | A | 4 |  |
| 80 | `ma_channel` | Moving Average Channel | — | TV | A | 4 |  |
| 81 | `apz` | — | Adaptive Price Zone / DblSmoothEMA (Bandy QTS book code) | AB | A | 4 |  |
| 82 | `alligator` | Williams Alligator | — | TV | A | 4 |  |
| 83 | `median` | Median | — | TV | A | 4 |  |
| 84 | `linreg_slope` | Linear Regression Slope | LinRegSlope | both | A | 5 |  |
| 85 | `linreg_intercept` | — | LinRegIntercept | AB | A | 5 |  |
| 86 | `tsf` | — | TSF | AB | A | 5 |  |
| 87 | `stderr` | Standard Error | StdErr | both | A | 5 |  |
| 88 | `se_bands` | Standard Error Bands | — | TV | A | 5 |  |
| 89 | `trend_strength` | Trend Strength Index | — | TV | A | 5 |  |
| 90 | `rci` | Rank Correlation Index (RCI) | — | TV | A | 5 |  |
| 91 | `rci_ribbon` | RCI Ribbon | — | TV | A | 5 |  |
| 92 | `percentrank` | ta.percentrank (Pine) | PercentRank | both | A | 6 |  |
| 93 | `percentile` | ta.percentile_linear_interpolation; ta.percentile_nearest_rank (Pine) | Percentile; Median | both | A | 6 |  |
| 94 | `zscore` | — | zscore / ExampleZScore (Bandy QTS book functions) | AB | A | 6 |  |
| 95 | `skewness` | — | Skewness | AB | A | 6 |  |
| 96 | `kurtosis` | — | Kurtosis | AB | A | 6 |  |
| 97 | `majority_rule` | Majority Rule | — | TV | A | 6 |  |
| 98 | `bop` | Balance of Power (BOP) | — | TV | A | 7 |  |
| 99 | `bbp` | Bull Bear Power | — | TV | A | 7 |  |
| 100 | `accel_osc` | Accelerator Oscillator | — | TV | A | 7 |  |
| 101 | `price_osc` | Price Oscillator | OscP | both | A | 7 |  |
| 102 | `woodies_cci` | Woodies CCI | — | TV | A | 7 |  |
| 103 | `rmi` | — | RMI | AB | A | 7 |  |
| 104 | `cg_osc` | — | CGOscillator (Bandy QTS book code; Ehlers Center of Gravity) | AB | A | 7 |  |
| 105 | `pmo` | Price Momentum Oscillator (PMO) | — | TV | A | 8 |  |
| 106 | `special_k` | Pring's Special K | — | TV | A | 8 |  |
| 107 | `smi_ergodic` | SMI Ergodic Indicator; SMI Ergodic Oscillator | — | TV | A | 8 |  |
| 108 | `smi` | Stochastic Momentum Index (SMI) | — | TV | A | 8 |  |
| 109 | `rvgi` | Relative Vigor Index | — | TV | A | 8 |  |
| 110 | `rvi` | Relative Volatility Index | — | TV | A | 8 |  |
| 111 | `chaikin_vol` | Chaikin Volatility | — | TV | A | 9 |  |
| 112 | `mass_index` | Mass Index | — | TV | A | 9 |  |
| 113 | `ulcer` | Ulcer Index | — | TV | A | 9 |  |
| 114 | `adr` | Average Daily Range (ADR) | — | TV | A | 9 |  |
| 115 | `volatility_zt` | Volatility Zero Trend Close-to-Close | — | TV | A | 9 |  |
| 116 | `volatility_ohlc` | Volatility O-H-L-C | — | TV | A | 9 |  |
| 117 | `bbtrend` | BBTrend | — | TV | A | 9 |  |
| 118 | `chande_kroll` | Chande Kroll Stop | — | TV | A | 10 |  |
| 119 | `chandelier` | Chandelier Exit | — | TV | A | 10 |  |
| 120 | `vstop` | Volatility Stop | — | TV | A | 10 |  |
| 121 | `wilder_vi` | Volatility Index | — | TV | A | 10 |  |
| 122 | `chop_zone` | Chop Zone | — | TV | A | 10 |  |
| 123 | `rwi` | — | RWI; RWIHi; RWILo | AB | A | 10 |  |
| 124 | `asi` | Accumulative Swing Index | — | TV | A | 10 |  |
| 125 | `klinger` | Klinger Oscillator | — | TV | A | 11 |  |
| 126 | `vol` | Volume | — | TV | A | 11 |  |
| 127 | `vwap_period` | VWAP (anchor Week/Month/Quarter/Year) | — | TV | A | 11 |  |
| 128 | `twap` | Time Weighted Average Price | — | TV | A | 11 |  |
| 129 | `pivots_std` | Pivot Points Standard | — | TV | A | 12 |  |
| 130 | `mtp` | Multi-Time Period Charts | — | TV | A | 12 |  |
| 131 | `performance` | Performance | — | TV | A | 12 |  |
| 132 | `calendar` | — | DayOfWeek; Day; Month; Year; DayOfYear; BarIndex | AB | A | 12 |  |
| 133 | `moon_phases` | Moon Phases | — | TV | A | 12 |  |
| 134 | `highest_bars` | ta.highestbars (Pine) | HHVBars; HighestBars | both | A | 12 |  |
| 135 | `lowest_bars` | ta.lowestbars (Pine) | LLVBars; LowestBars | both | A | 12 |  |
| 136 | `fractals` | Williams fractal indicator; Williams Fractal | — | TV | A | 13 |  |
| 137 | `rsi_divergence` | RSI divergence indicator | — | TV | A | 13 |  |
| 138 | `ad_ratio_bars` | Advance/Decline Ratio (Bars); Advance/Decline (charting library) | — | TV | A | 13 |  |
| 139 | `bar_patterns` | — | GapUp; GapDown; Inside; Outside | AB | A | 13 |  |
| 140 | `tech_ratings` | Technical Ratings | — | TV | A | 16 |  |
| 141 | `cdl_abandoned_baby_bear` | Abandoned Baby - Bearish | — | TV | A | 15 |  |
| 142 | `cdl_abandoned_baby_bull` | Abandoned Baby - Bullish | — | TV | A | 15 |  |
| 143 | `cdl_dark_cloud_cover` | Dark Cloud Cover - Bearish | — | TV | A | 14 |  |
| 144 | `cdl_doji` | Doji | — | TV | A | 14 |  |
| 145 | `cdl_doji_star_bear` | Doji Star - Bearish | — | TV | A | 14 |  |
| 146 | `cdl_doji_star_bull` | Doji Star - Bullish | — | TV | A | 14 |  |
| 147 | `cdl_downside_tasuki_gap` | Downside Tasuki Gap - Bearish | — | TV | A | 15 |  |
| 148 | `cdl_dragonfly_doji` | Dragonfly Doji - Bullish | — | TV | A | 14 |  |
| 149 | `cdl_engulfing_bear` | Engulfing - Bearish | — | TV | A | 14 |  |
| 150 | `cdl_engulfing_bull` | Engulfing - Bullish | — | TV | A | 14 |  |
| 151 | `cdl_evening_doji_star` | Evening Doji Star - Bearish | — | TV | A | 15 |  |
| 152 | `cdl_evening_star` | Evening Star - Bearish | — | TV | A | 15 |  |
| 153 | `cdl_falling_three_methods` | Falling Three Methods - Bearish | — | TV | A | 15 |  |
| 154 | `cdl_falling_window` | Falling Window - Bearish | — | TV | A | 14 |  |
| 155 | `cdl_gravestone_doji` | Gravestone Doji - Bearish | — | TV | A | 14 |  |
| 156 | `cdl_hammer` | Hammer - Bullish | — | TV | A | 14 |  |
| 157 | `cdl_hanging_man` | Hanging Man - Bearish | — | TV | A | 14 |  |
| 158 | `cdl_harami_bear` | Harami - Bearish | — | TV | A | 14 |  |
| 159 | `cdl_harami_bull` | Harami - Bullish | — | TV | A | 14 |  |
| 160 | `cdl_harami_cross_bear` | Harami Cross - Bearish | — | TV | A | 14 |  |
| 161 | `cdl_harami_cross_bull` | Harami Cross - Bullish | — | TV | A | 14 |  |
| 162 | `cdl_inverted_hammer` | Inverted Hammer - Bullish | — | TV | A | 14 |  |
| 163 | `cdl_kicking_bear` | Kicking - Bearish | — | TV | A | 14 |  |
| 164 | `cdl_kicking_bull` | Kicking - Bullish | — | TV | A | 14 |  |
| 165 | `cdl_long_lower_shadow` | Long Lower Shadow - Bullish | — | TV | A | 14 |  |
| 166 | `cdl_long_upper_shadow` | Long Upper Shadow - Bearish | — | TV | A | 14 |  |
| 167 | `cdl_marubozu_black` | Marubozu Black - Bearish | — | TV | A | 14 |  |
| 168 | `cdl_marubozu_white` | Marubozu White - Bullish | — | TV | A | 14 |  |
| 169 | `cdl_morning_doji_star` | Morning Doji Star - Bullish | — | TV | A | 15 |  |
| 170 | `cdl_morning_star` | Morning Star - Bullish | — | TV | A | 15 |  |
| 171 | `cdl_on_neck` | On Neck - Bearish | — | TV | A | 14 |  |
| 172 | `cdl_piercing` | Piercing - Bullish | — | TV | A | 14 |  |
| 173 | `cdl_rising_three_methods` | Rising Three Methods - Bullish | — | TV | A | 15 |  |
| 174 | `cdl_rising_window` | Rising Window - Bullish | — | TV | A | 14 |  |
| 175 | `cdl_shooting_star` | Shooting Star - Bearish | — | TV | A | 14 |  |
| 176 | `cdl_spinning_top_black` | Spinning Top Black | — | TV | A | 14 |  |
| 177 | `cdl_spinning_top_white` | Spinning Top White | — | TV | A | 14 |  |
| 178 | `cdl_three_black_crows` | Three Black Crows - Bearish | — | TV | A | 15 |  |
| 179 | `cdl_three_white_soldiers` | Three White Soldiers - Bullish | — | TV | A | 15 |  |
| 180 | `cdl_tri_star_bear` | Tri Star - Bearish | — | TV | A | 15 |  |
| 181 | `cdl_tri_star_bull` | Tri Star - Bullish | — | TV | A | 15 |  |
| 182 | `cdl_tweezer_bottom` | Tweezer Bottom - Bullish | — | TV | A | 14 |  |
| 183 | `cdl_tweezer_top` | Tweezer Top - Bearish | — | TV | A | 14 |  |
| 184 | `cdl_upside_tasuki_gap` | Upside Tasuki Gap - Bullish | — | TV | A | 15 |  |
| 185 | `cdl_all` | All Candlestick Patterns | — | TV | A | 15 |  |
| 186 | `correlation` | Correlation Coefficient (CC) | Correlation (two symbols); Foreign/SetForeign | both | B | 1 | not available yet — needs a second instrument in the same request |
| 187 | `correlation_log` | Correlation - Log | — | TV | B | 1 | not available yet — needs a second instrument in the same request |
| 188 | `ratio` | Ratio | RelStrength | both | B | 1 | not available yet — needs a second instrument in the same request |
| 189 | `spread` | Spread | — | TV | B | 1 | not available yet — needs a second instrument in the same request |
| 190 | `cvd` | Cumulative Volume Delta | — | TV | C:intraday | 1 | not available yet — needs intraday (lower-timeframe/tick) bars |
| 191 | `volume_delta` | Volume Delta | — | TV | C:intraday | 1 | not available yet — needs intraday (lower-timeframe/tick) bars |
| 192 | `up_down_volume` | Up/Down Volume | — | TV | C:intraday | 1 | not available yet — needs intraday (lower-timeframe/tick) bars |
| 193 | `net_volume` | Net Volume | — | TV | C:intraday | 1 | not available yet — needs intraday (lower-timeframe/tick) bars |
| 194 | `rvol_at_time` | Relative Volume at Time | — | TV | C:intraday | 1 | not available yet — needs intraday (lower-timeframe/tick) bars |
| 195 | `trading_sessions` | Trading Sessions | — | TV | C:intraday | 1 | not available yet — needs intraday (lower-timeframe/tick) bars |
| 196 | `volume_24h` | 24-hour Volume | — | TV | C:intraday | 1 | not available yet — needs intraday (lower-timeframe/tick) bars |
| 197 | `volume_profile` | Volume Profile Fixed Range; Volume Profile Visible Range; Session Volume Profile | PlotVAPOverlayA | both | C:intraday | 1 | not available yet — needs intraday (lower-timeframe/tick) bars |
| 198 | `rob_booker_intraday_pivots` | Rob Booker Intraday Pivot Points | — | TV | C:intraday | 1 | not available yet — needs intraday (lower-timeframe/tick) bars |
| 199 | `analyst_forecast` | Analyst price forecast | — | TV | C:fundamentals | 1 | not available yet — needs company fundamentals/estimates data |
| 200 | `price_target` | Price target - indicator | — | TV | C:fundamentals | 1 | not available yet — needs company fundamentals/estimates data |
| 201 | `dividend_yield` | Dividend Yield | — | TV | C:fundamentals | 1 | not available yet — needs company fundamentals/estimates data |
| 202 | `open_interest` | Open Interest; Understanding crypto open interest | — | TV | C:derivatives | 1 | not available yet — needs futures/perpetual-swap market data (open interest, funding, positioning) |
| 203 | `funding_rate` | Funding rate | — | TV | C:derivatives | 1 | not available yet — needs futures/perpetual-swap market data (open interest, funding, positioning) |
| 204 | `liquidations` | Liquidation data | — | TV | C:derivatives | 1 | not available yet — needs futures/perpetual-swap market data (open interest, funding, positioning) |
| 205 | `long_short_ratio` | Long / Short Ratio Accounts | — | TV | C:derivatives | 1 | not available yet — needs futures/perpetual-swap market data (open interest, funding, positioning) |
| 206 | `long_short_accounts` | Long Short Accounts % | — | TV | C:derivatives | 1 | not available yet — needs futures/perpetual-swap market data (open interest, funding, positioning) |
| 207 | `top_trader_accounts` | Top trader long/short accounts | — | TV | C:derivatives | 1 | not available yet — needs futures/perpetual-swap market data (open interest, funding, positioning) |
| 208 | `top_trader_accounts_ratio` | Top trader long/short accounts ratio | — | TV | C:derivatives | 1 | not available yet — needs futures/perpetual-swap market data (open interest, funding, positioning) |
| 209 | `top_trader_positions` | Top trader long/short positions | — | TV | C:derivatives | 1 | not available yet — needs futures/perpetual-swap market data (open interest, funding, positioning) |
| 210 | `top_trader_positions_ratio` | Top trader long/short positions ratio | — | TV | C:derivatives | 1 | not available yet — needs futures/perpetual-swap market data (open interest, funding, positioning) |
| 211 | `basis` | Basis | — | TV | C:derivatives | 1 | not available yet — needs futures/perpetual-swap market data (open interest, funding, positioning) |
| 212 | `premium` | Premium | — | TV | C:derivatives | 1 | not available yet — needs futures/perpetual-swap market data (open interest, funding, positioning) |
| 213 | `index_price` | Index price | — | TV | C:derivatives | 1 | not available yet — needs futures/perpetual-swap market data (open interest, funding, positioning) |
| 214 | `mark_price` | Mark price | — | TV | C:derivatives | 1 | not available yet — needs futures/perpetual-swap market data (open interest, funding, positioning) |
| 215 | `onchain_1_year_active_supply_pct` | 1 year active supply % | — | TV | C:onchain | 1 | not available yet — needs blockchain on-chain data (crypto only) |
| 216 | `onchain_active_addresses_with_contracts` | Active addresses with contracts | — | TV | C:onchain | 1 | not available yet — needs blockchain on-chain data (crypto only) |
| 217 | `onchain_addresses_with_balance_gte_x_pct_of_supply` | Addresses with balance ≥ X (% of supply) | — | TV | C:onchain | 1 | not available yet — needs blockchain on-chain data (crypto only) |
| 218 | `onchain_addresses_with_balance_gte_x_usd` | Addresses with balance ≥ X (USD) | — | TV | C:onchain | 1 | not available yet — needs blockchain on-chain data (crypto only) |
| 219 | `onchain_average_transaction_volume` | Average transaction volume | — | TV | C:onchain | 1 | not available yet — needs blockchain on-chain data (crypto only) |
| 220 | `onchain_block_height` | Block height | — | TV | C:onchain | 1 | not available yet — needs blockchain on-chain data (crypto only) |
| 221 | `onchain_blocks_mined` | Blocks mined | — | TV | C:onchain | 1 | not available yet — needs blockchain on-chain data (crypto only) |
| 222 | `onchain_created_utxos` | Created UTXOs | — | TV | C:onchain | 1 | not available yet — needs blockchain on-chain data (crypto only) |
| 223 | `onchain_difficulty` | Difficulty | — | TV | C:onchain | 1 | not available yet — needs blockchain on-chain data (crypto only) |
| 224 | `onchain_el_salvador_government_balance` | El Salvador Government balance | — | TV | C:onchain | 1 | not available yet — needs blockchain on-chain data (crypto only) |
| 225 | `onchain_etf_balances` | ETF balances | — | TV | C:onchain | 1 | not available yet — needs blockchain on-chain data (crypto only) |
| 226 | `onchain_etf_flows` | ETF flows | — | TV | C:onchain | 1 | not available yet — needs blockchain on-chain data (crypto only) |
| 227 | `onchain_ethereum_new_deposits` | Ethereum new deposits | — | TV | C:onchain | 1 | not available yet — needs blockchain on-chain data (crypto only) |
| 228 | `onchain_ethereum_new_unique_depositors` | Ethereum new unique depositors | — | TV | C:onchain | 1 | not available yet — needs blockchain on-chain data (crypto only) |
| 229 | `onchain_ethereum_new_value_staked` | Ethereum new value staked | — | TV | C:onchain | 1 | not available yet — needs blockchain on-chain data (crypto only) |
| 230 | `onchain_ethereum_total_number_of_deposits` | Ethereum total number of deposits | — | TV | C:onchain | 1 | not available yet — needs blockchain on-chain data (crypto only) |
| 231 | `onchain_ethereum_total_unique_depositors` | Ethereum total unique depositors | — | TV | C:onchain | 1 | not available yet — needs blockchain on-chain data (crypto only) |
| 232 | `onchain_ethereum_total_value_staked` | Ethereum total value staked | — | TV | C:onchain | 1 | not available yet — needs blockchain on-chain data (crypto only) |
| 233 | `onchain_hash_rate` | Hash Rate | — | TV | C:onchain | 1 | not available yet — needs blockchain on-chain data (crypto only) |
| 234 | `onchain_held_tokens_in_addresses_gte_x_pct_of_supply` | Held tokens in addresses ≥ X (% of supply) | — | TV | C:onchain | 1 | not available yet — needs blockchain on-chain data (crypto only) |
| 235 | `onchain_held_tokens_in_addresses_gte_x_tokens` | Held tokens in addresses ≥ X (tokens) | — | TV | C:onchain | 1 | not available yet — needs blockchain on-chain data (crypto only) |
| 236 | `onchain_held_tokens_in_addresses_gte_x_usd` | Held tokens in addresses ≥ X (USD) | — | TV | C:onchain | 1 | not available yet — needs blockchain on-chain data (crypto only) |
| 237 | `onchain_large_transaction_volume` | Large transaction volume | — | TV | C:onchain | 1 | not available yet — needs blockchain on-chain data (crypto only) |
| 238 | `onchain_mean_block_interval` | Mean block interval | — | TV | C:onchain | 1 | not available yet — needs blockchain on-chain data (crypto only) |
| 239 | `onchain_mean_block_size_in_bytes` | Mean block size in bytes | — | TV | C:onchain | 1 | not available yet — needs blockchain on-chain data (crypto only) |
| 240 | `onchain_mean_gas_used` | Mean gas used | — | TV | C:onchain | 1 | not available yet — needs blockchain on-chain data (crypto only) |
| 241 | `onchain_mean_transaction_fees` | Mean transaction fees | — | TV | C:onchain | 1 | not available yet — needs blockchain on-chain data (crypto only) |
| 242 | `onchain_mean_transaction_gas_limit` | Mean transaction gas limit | — | TV | C:onchain | 1 | not available yet — needs blockchain on-chain data (crypto only) |
| 243 | `onchain_mean_transaction_gas_price` | Mean transaction gas price | — | TV | C:onchain | 1 | not available yet — needs blockchain on-chain data (crypto only) |
| 244 | `onchain_mean_transaction_size_in_bytes` | Mean transaction size in bytes | — | TV | C:onchain | 1 | not available yet — needs blockchain on-chain data (crypto only) |
| 245 | `onchain_mean_transfer_volume` | Mean transfer volume | — | TV | C:onchain | 1 | not available yet — needs blockchain on-chain data (crypto only) |
| 246 | `onchain_mean_utxo_value_created` | Mean UTXO value created | — | TV | C:onchain | 1 | not available yet — needs blockchain on-chain data (crypto only) |
| 247 | `onchain_mean_utxo_value_spent` | Mean UTXO value spent | — | TV | C:onchain | 1 | not available yet — needs blockchain on-chain data (crypto only) |
| 248 | `onchain_median_block_interval` | Median block interval | — | TV | C:onchain | 1 | not available yet — needs blockchain on-chain data (crypto only) |
| 249 | `onchain_median_gas_used` | Median gas used | — | TV | C:onchain | 1 | not available yet — needs blockchain on-chain data (crypto only) |
| 250 | `onchain_median_transaction_fees` | Median transaction fees | — | TV | C:onchain | 1 | not available yet — needs blockchain on-chain data (crypto only) |
| 251 | `onchain_median_transaction_gas_limit` | Median transaction gas limit | — | TV | C:onchain | 1 | not available yet — needs blockchain on-chain data (crypto only) |
| 252 | `onchain_median_transaction_gas_price` | Median transaction gas price | — | TV | C:onchain | 1 | not available yet — needs blockchain on-chain data (crypto only) |
| 253 | `onchain_median_transfer_volume` | Median transfer volume | — | TV | C:onchain | 1 | not available yet — needs blockchain on-chain data (crypto only) |
| 254 | `onchain_median_utxo_value_created` | Median UTXO value created | — | TV | C:onchain | 1 | not available yet — needs blockchain on-chain data (crypto only) |
| 255 | `onchain_median_utxo_value_spent` | Median UTXO value spent | — | TV | C:onchain | 1 | not available yet — needs blockchain on-chain data (crypto only) |
| 256 | `onchain_power_law_model` | Power-Law Model | — | TV | C:onchain | 1 | not available yet — needs blockchain on-chain data (crypto only) |
| 257 | `onchain_realized_market_cap` | Realized market cap | — | TV | C:onchain | 1 | not available yet — needs blockchain on-chain data (crypto only) |
| 258 | `onchain_receiving_addresses` | Receiving addresses | — | TV | C:onchain | 1 | not available yet — needs blockchain on-chain data (crypto only) |
| 259 | `onchain_rvt_ratio_90_days` | RVT ratio, 90 days | — | TV | C:onchain | 1 | not available yet — needs blockchain on-chain data (crypto only) |
| 260 | `onchain_sending_addresses` | Sending addresses | — | TV | C:onchain | 1 | not available yet — needs blockchain on-chain data (crypto only) |
| 261 | `onchain_spent_output_profit_ratio_sopr` | Spent Output Profit Ratio (SOPR) | — | TV | C:onchain | 1 | not available yet — needs blockchain on-chain data (crypto only) |
| 262 | `onchain_spent_utxos` | Spent UTXOs | — | TV | C:onchain | 1 | not available yet — needs blockchain on-chain data (crypto only) |
| 263 | `onchain_stock_to_flow_ratio_in_usd` | Stock-to-Flow Ratio in USD | — | TV | C:onchain | 1 | not available yet — needs blockchain on-chain data (crypto only) |
| 264 | `onchain_supply_equality_ratio` | Supply Equality Ratio | — | TV | C:onchain | 1 | not available yet — needs blockchain on-chain data (crypto only) |
| 265 | `onchain_total_block_size_in_bytes` | Total block size in bytes | — | TV | C:onchain | 1 | not available yet — needs blockchain on-chain data (crypto only) |
| 266 | `onchain_total_gas_used` | Total gas used | — | TV | C:onchain | 1 | not available yet — needs blockchain on-chain data (crypto only) |
| 267 | `onchain_total_transactions_size_in_bytes` | Total transactions size in bytes | — | TV | C:onchain | 1 | not available yet — needs blockchain on-chain data (crypto only) |
| 268 | `onchain_total_utxo_value_created` | Total UTXO value created | — | TV | C:onchain | 1 | not available yet — needs blockchain on-chain data (crypto only) |
| 269 | `onchain_total_utxo_value_spent` | Total UTXO value spent | — | TV | C:onchain | 1 | not available yet — needs blockchain on-chain data (crypto only) |
| 270 | `onchain_total_utxos` | Total UTXOs | — | TV | C:onchain | 1 | not available yet — needs blockchain on-chain data (crypto only) |
| 271 | `onchain_transaction_fees` | Transaction fees | — | TV | C:onchain | 1 | not available yet — needs blockchain on-chain data (crypto only) |
| 272 | `onchain_transaction_rate` | Transaction rate | — | TV | C:onchain | 1 | not available yet — needs blockchain on-chain data (crypto only) |
| 273 | `onchain_transfer_count` | Transfer count | — | TV | C:onchain | 1 | not available yet — needs blockchain on-chain data (crypto only) |
| 274 | `onchain_transfer_rate` | Transfer rate | — | TV | C:onchain | 1 | not available yet — needs blockchain on-chain data (crypto only) |
| 275 | `onchain_us_spot_crypto_etf_balances` | US spot crypto ETF balances | — | TV | C:onchain | 1 | not available yet — needs blockchain on-chain data (crypto only) |
| 276 | `onchain_us_spot_crypto_etf_flows` | US spot crypto ETF flows | — | TV | C:onchain | 1 | not available yet — needs blockchain on-chain data (crypto only) |
| 277 | `auto_fib_retracement` | Auto Fib Retracement | — | TV | C:drawing | 1 | chart drawing/display — not a bar-by-bar series; not usable in rules |
| 278 | `auto_fib_extension` | Auto fib extension | — | TV | C:drawing | 1 | chart drawing/display — not a bar-by-bar series; not usable in rules |
| 279 | `auto_key_levels` | Auto key levels | — | TV | C:drawing | 1 | chart drawing/display — not a bar-by-bar series; not usable in rules |
| 280 | `auto_pitchfork` | Auto Pitchfork | — | TV | C:drawing | 1 | chart drawing/display — not a bar-by-bar series; not usable in rules |
| 281 | `auto_trendlines` | Auto trendlines | — | TV | C:drawing | 1 | chart drawing/display — not a bar-by-bar series; not usable in rules |
| 282 | `bollinger_bars` | Bollinger Bars | — | TV | C:drawing | 1 | chart drawing/display — not a bar-by-bar series; not usable in rules |
| 283 | `linreg_channel` | Linear Regression (channel) | — | TV | C:drawing | 1 | chart drawing/display — not a bar-by-bar series; not usable in rules |
| 284 | `seasonality` | Seasonality; Learn using seasonals | — | TV | C:drawing | 1 | chart drawing/display — not a bar-by-bar series; not usable in rules |
| 285 | `visible_avg_price` | Visible Average Price | — | TV | C:drawing | 1 | chart drawing/display — not a bar-by-bar series; not usable in rules |
| 286 | `fft` | — | FFT | AB | C:drawing | 1 | chart drawing/display — not a bar-by-bar series; not usable in rules |
| 287 | `zigzag` | Zigzag indicator; Zig Zag | Zig | both | C:lookahead | 1 | looks ahead — chart display only, not usable in rules |
| 288 | `peak` | — | Peak | AB | C:lookahead | 1 | looks ahead — chart display only, not usable in rules |
| 289 | `trough` | — | Trough | AB | C:lookahead | 1 | looks ahead — chart display only, not usable in rules |
| 290 | `peak_bars` | — | PeakBars | AB | C:lookahead | 1 | looks ahead — chart display only, not usable in rules |
| 291 | `trough_bars` | — | TroughBars | AB | C:lookahead | 1 | looks ahead — chart display only, not usable in rules |
| 292 | `vwap_auto_anchored` | VWAP Auto Anchored | — | TV | C:lookahead | 1 | looks ahead — chart display only, not usable in rules |
| 293 | `rob_booker_adx_breakout` | Rob Booker - ADX Breakout | — | TV | C:rules | 1 | not computed — TradingView does not publish this formula; awaiting a source review |
| 294 | `rob_booker_knoxville` | Rob Booker - Knoxville Divergence | — | TV | C:rules | 1 | not computed — TradingView does not publish this formula; awaiting a source review |
| 295 | `rob_booker_reversal` | Rob Booker Reversal | — | TV | C:rules | 1 | not computed — TradingView does not publish this formula; awaiting a source review |
| 296 | `rob_booker_ziv_ghost` | Rob Booker Ziv Ghost Pivots | — | TV | C:rules | 1 | not computed — TradingView does not publish this formula; awaiting a source review |
| 297 | `rob_booker_missed_pivots` | Rob Booker Missed Pivot Points | — | TV | C:rules | 1 | not computed — TradingView does not publish this formula; awaiting a source review |
| 298 | `fir` | — | FIR | AB | C:usercode | 1 | not computed — needs a user-supplied coefficient list; the lab runs no user code |
| 299 | `iir` | — | IIR | AB | C:usercode | 1 | not computed — needs a user-supplied coefficient list; the lab runs no user code |

## AFL utility functions (not indicators): how each is covered

| AFL function(s) | Used by book AFL | Coverage in the lab |
|---|---|---|
| `Ref(x, -n)` | yes (79) | existing: operand `offset` (n ≥ 0 only; `Ref(x, +n)` looks ahead and is refused) |
| `Cross(a, b)` | yes (66) | existing: operators `crosses_above` / `crosses_below` |
| `HHV`, `LLV`, `Highest`, `Lowest` | yes | existing `highest`/`lowest` on high/low; on other arrays → later `source` selector (Decision 10) |
| `HHVBars`, `LLVBars`, `HighestBars`, `LowestBars` | no | new `highest_bars`/`lowest_bars` (Task 12) |
| `HighestSince`, `LowestSince`, `…SinceBars`, `BarsSince`, `BarsSinceCompare`, `ValueWhen`, `SumSince`, `ProdSince` | yes (`BarsSince` 22, `HighestSince` 1) | not yet: condition-based operands are a rule-builder follow-up (Phase 1 scope note) |
| `IIf`, `Flip`, `ExRem`, `ExRemSpan`, `Hold` | yes | `IIf`: out of scope (form-based rules, no expressions). `ExRem`/`Flip`: simulator semantics already (one position, no pyramiding) |
| `Sum`, `Cum`, `CumProd`, `Prod`, math (`abs`, `log`, `sqrt`, `floor`, `Int`, `Max`, `Min`, `Prec`, `sign`, trig) | yes | out of scope: no arithmetic expressions in the rule spec (spec §1 form-based) |
| `TimeFrameSet`, `TimeFrameExpand`, `TimeFrameCompress`, `TimeFrameRestore`, `TimeFrameGetPrice`, `TimeFrameMode` | yes | existing: operand `tf` D/W/M with expand-after-close; previous-period OHLC also via `mtp` (Task 12) |
| `Day`, `Month`, `Year`, `DayOfWeek`, `DayOfYear`, `BarIndex`, `DateNum` | yes | new `calendar` indicator (Task 12); `Hour`, `Minute` → need intraday (listed reason) |
| `Foreign`, `SetForeign`, `RestorePriceArrays`, `PlotForeign`, `RelStrength`, `GetBaseIndex` | yes (17) | class B, listed (`correlation`, `ratio`); needs a second instrument (Decision 5) |
| `AddToComposite`, `AdvIssues`… | yes (ATC 8) | composites supplied as data: breadth family (Task 2); building composites → out of scope |
| `ApplyStop`, `SetPositionSize`, `SetTradeDelays`, `SetOption`, `EnableRotationalTrading`, `SetBacktestMode`, custom backtester | yes | `SetTradeDelays`: existing fill `next_open`. Stops/size/rotation: reserved spec fields → `unsupported` (spec §4); portfolio/rotation is a later phase |
| `Optimize`, `Param*` | yes | existing: `optimize` paths and catalogue params |
| `Equity`, `Status`, `GetBacktesterObject`, `GetPerformanceStats` | yes | existing: `/lab/run` statistics (book accounting) |
| `Random`, `mtRandom`, `mtRandomA`, book `Rand`, `NormalRandomNumber` | yes | out of scope: non-deterministic |
| `NormDist`, `erf`, book `InitializeNormalCDF`, `ComputeRunZ` | yes | out of scope: statistics of a value or of trades, not a bar series |
| `Plot*`, `AddColumn`, `Filter`, strings, file I/O, GUI | yes | frontend / exploration (basket scan is a later phase) |
| `LastValue`, `SelectedValue`, `BeginValue`, `EndValue`, `*VisibleValue` | yes | out of scope: look-ahead or viewport-dependent |

## File structure

```
Context/guru/backend/src/lab/
  catalogue.rs   Needs flags + NONE + any_data(); IndicatorDef.note; int() param helper; run() note/breadth;
                 all() lists the new DEFS; generic tests (1000 bars, breadth filter, sources, listed, master count)
  sources.rs     NEW  SOURCES: (id, TradingView names, AmiBroker names) for every catalogue id; names(id)
  listed.rs      NEW  all class B/C listed-only entries (114) with notes
  breadth.rs     NEW  Task 2  breadth, adl, ad_ratio, cvi, trin
  ma2.rs         NEW  Task 3  ma_ribbon, ma_cross, gmma, hamming_ma, ama, ama2, iir2
  bands.rs       NEW  Task 4  envelope, ma_channel, apz, alligator, median
  stats.rs       NEW  Tasks 5–6 regression + distribution statistics
  osc2.rs        NEW  Tasks 7–8 oscillators
  vol2.rs        NEW  Task 9  volatility
  stops.rs       NEW  Task 10 stops and trend state
  volume2.rs     NEW  Task 11 klinger, vol, vwap_period, twap
  periods.rs     NEW  Task 12 pivots_std, mtp, performance, calendar, moon_phases, highest_bars, lowest_bars
  patterns.rs    NEW  Task 13 fractals, rsi_divergence, ad_ratio_bars, bar_patterns
  candles.rs     NEW  Tasks 14–15 44 candlestick patterns + cdl_all
  ratings.rs     NEW  Task 16 tech_ratings
  bars.rs        Task 2: Breadth struct, Bars.breadth, Bars::from_ctx
  resample.rs    Task 1: pub civil(); Task 11: pub period_key()
  rules.rs       Task 1: series_ref maps listed entries to needs_data / unsupported with the note
  series.rs      Task 1: tests use resample::civil, reachability + listed tests; Task 2: "needs breadth" mapping
  api.rs         Task 1: catalogue_json fields; Task 2: Bars::from_ctx (seams, see above)
  price.rs       Task 13: pub(super) pivots() helper (pivot_high/low unchanged)
  momentum.rs    Task 17: stoch/trix variant parameter
  ma.rs, trend.rs, volatility.rs, volume.rs  Task 1: Needs literals → Needs::NONE, def helpers add note: ""
  testdata.rs    Task 1: walk real dates, d1(), closes(), go(), compiles(); Task 2: with_breadth(), cut()
  mod.rs         one `pub mod` line per new file
docs/indicator-parity-fixtures.md   every new id: parameters (default) and output order; listed-only section
```

## Conventions for every family task (Tasks 3–16)

Each family file starts like this (Phase 1 style; `div` returns NaN on a zero or NaN operand):

```rust
//! <Family> family (TradingView / AmiBroker built-ins, Phase 6). Formulas: see each definition string.
use super::bars::Bars;
use super::catalogue::{int, len, n, p, real, IndicatorDef, Needs, Output, ParamSpec, Params, Placement};
use super::ta::{self, NA};

const CAT: &str = "<Category>";
fn div(a: f64, b: f64) -> f64 { if b == 0. || a.is_nan() || b.is_nan() { NA } else { a / b } }
#[allow(clippy::too_many_arguments)]
const fn def(id: &'static str, name: &'static str, params: &'static [ParamSpec], outputs: &'static [&'static str], placement: Placement,
             needs: Needs, definition: &'static str, first: &'static [usize], sparse: bool, f: fn(&Bars, &Params) -> Output) -> IndicatorDef {
    IndicatorDef { id, name, category: CAT, params, outputs, placement, needs, definition, first, sparse, compute: Some(f), note: "" }
}
pub const DEFS: &[IndicatorDef] = &[ /* one def(...) per table row */ ];
```

Import only what the file uses: the gate rejects any `src/lab/` compiler warning.

Each family table row gives the id, name, params (`len` = integer 1–2000 unless a range is given; `int` = integer choice; `real` = float `(default, min, max, step)`), outputs in order, placement, needs, `first` (first non-NaN index per output at default parameters on gap-free OHLCV, from the Python reference) and the definition. The definition string to store is the text in the **definition** column, with any "declared" note kept.

Registration for every family task:
1. `lab/mod.rs`: add `pub mod <file>;` (alphabetical place).
2. `catalogue::all()`: append `super::<file>::DEFS` to the `concat` list, once per file (the second task of a shared file only adds rows to `DEFS`).
3. `sources.rs`: append the rows given in the task to `SOURCES`.
4. `docs/indicator-parity-fixtures.md`: append one table row per id: `| id | params (default) | outputs in order |`.

Test helpers (added in Task 1 to `testdata.rs`): `d1()` (8 OHLCV bars below, dates 2024-01-01 (Mon)..2024-01-08), `closes(&[f64])`, `go(id, &bars, &[(key, value)]) -> Vec<Vec<f64>>` (unwraps `catalogue::run`), `compiles(id, out) -> Result<(), LabError>` (parses and compiles a one-condition rule spec on 300 synthetic business days with volume — Task 2 adds breadth to those bars).

```
d1():  O/H/L/C                         V
 0  10    11    9     10.5             100
 1  10.5  12    10    11.5             120
 2  11.5  12.5  11    12                90
 3  12    12.2  10.8  11               150
 4  11    11.5  10    10.2             130
 5  10.2  11    9.5   10.8             110
 6  10.8  12    10.5  11.9             160
 7  11.9  13    11.5  12.6             140
```

Commands (from `Context/guru/backend`): family tests `CARGO_BUILD_JOBS=2 cargo test lab::<file>`; all lab tests `CARGO_BUILD_JOBS=2 cargo test lab::`; guard `python3 ../../../scripts/check_book_untouched.py --base origin/Manan` (from the repo root: `python3 scripts/check_book_untouched.py --base origin/Manan`) → `"problems": []`.

The generic catalogue tests (`every_indicator_length_warmup_and_no_gaps`, `every_indicator_is_causal`, `flat_series_never_inf`, `short_history_never_panics`, `defaults_resolve_for_all`, `ids_unique_and_params_sane`) and the series tests (`signals_never_use_later_bars` on D/W/M, `every_indicator_at_parameter_bounds_never_panics`, `every_computed_indicator_is_reachable_from_a_rule`, `sources_cover_every_indicator_once`) automatically cover every new id once it is registered; a family task is done only when all of them pass.

---
### Task 1: Catalogue extension — data-need flags, notes, sources, all listed-only entries

**Files:**
- Modify: `Context/guru/backend/src/lab/catalogue.rs` (Needs, IndicatorDef.note, `int`, `all`, `run`, generic tests)
- Create: `Context/guru/backend/src/lab/sources.rs`, `Context/guru/backend/src/lab/listed.rs`
- Modify: `lab/mod.rs`, `lab/rules.rs` (`series_ref`, two lines), `lab/api.rs` (`catalogue_json` format only — seam), `lab/series.rs` (tests), `lab/resample.rs` (`civil`), `lab/testdata.rs`
- Modify (mechanical): `lab/ma.rs`, `lab/momentum.rs`, `lab/trend.rs`, `lab/volatility.rs`, `lab/volume.rs`, `lab/price.rs` — every `Needs { volume: false, intraday: false }` → `Needs::NONE`, `Needs { volume: true, intraday: false }` → `Needs { volume: true, ..Needs::NONE }`, every `IndicatorDef { … }` literal gains `note: ""`; `volume.rs` `listed()` gains a `note` argument
- Modify: `docs/indicator-parity-fixtures.md`

**Interfaces:**
- Produces: `catalogue::Needs { volume, intraday, breadth, second_symbol, fundamentals, derivatives, onchain }`, `Needs::NONE`, `Needs::any_data(&self) -> bool`; `IndicatorDef.note: &'static str`; `catalogue::int(key, label, default, min, max) -> ParamSpec`; `sources::SOURCES: &[(&str, &str, &str)]`, `sources::names(id) -> (&'static str, &'static str)`; `listed::DEFS`; `resample::civil(day: i64) -> String`; `testdata::{d1, closes, go, compiles}`; catalogue JSON per indicator gains `needs.{breadth,second_symbol,fundamentals,derivatives,onchain}`, `note`, `tv`, `ab`.
- Consumes: Phase 1/2 catalogue, `rules::{parse_spec, series_ref, LabError}`, `series::{compile, Frames, Cache}`.

- [ ] **Step 1: Write the failing tests**

In `catalogue.rs` tests add a `computable` filter, raise the warm-up test to 1,000 bars, and add:

```rust
    /// Breadth indicators need market breadth on the bars; Task 2 tests them with testdata::with_breadth.
    fn computable(d: &IndicatorDef) -> bool { d.compute.is_some() && !d.needs.breadth }
    #[test] fn listed_entries_carry_a_note() {
        for d in all() {
            if d.compute.is_some() { assert!(d.note.is_empty(), "{}: computed entries have no note", d.id); continue; }
            assert!(!d.note.is_empty(), "{}: listed entry without a note", d.id);
            if d.needs.any_data() { assert!(d.note.starts_with("not available yet — needs"), "{}: {}", d.id, d.note); }
        }
    }
    #[test] fn sources_cover_every_indicator_once() {
        use crate::lab::sources::SOURCES;
        let ids: Vec<&str> = all().iter().map(|d| d.id).collect();
        for (id, tv, ab) in SOURCES {
            assert!(ids.contains(id), "sources row for unknown id {id}");
            assert!(!tv.is_empty() || !ab.is_empty() || *id == "opening_range", "{id}: no TradingView or AmiBroker name");
        }
        for id in &ids { assert_eq!(SOURCES.iter().filter(|r| r.0 == *id).count(), 1, "{id}: exactly one sources row"); }
    }
```
Change `every_indicator_length_warmup_and_no_gaps` to `let bars = walk(1000, 7, true);` and replace `.filter(|d| d.compute.is_some())` with `.filter(|d| computable(d))` in `every_indicator_length_warmup_and_no_gaps`, `every_indicator_is_causal`, `flat_series_never_inf` and `short_history_never_panics`.

In `series.rs` tests (replace the local `civil` with `resample::civil`; delete the local fn) add:

```rust
    #[test] fn every_computed_indicator_is_reachable_from_a_rule() {
        for d in all().iter().filter(|d| d.compute.is_some()) {
            let out = d.outputs[d.outputs.len() - 1];
            if let Err(e) = crate::lab::testdata::compiles(d.id, out) { panic!("{}.{out}: {} {}", d.id, e.status(), e.message()); }
        }
    }
    #[test] fn listed_entries_refuse_with_their_note() {
        for (id, status) in [("vwap", "needs_data"), ("cvd", "needs_data"), ("correlation", "needs_data"), ("dividend_yield", "needs_data"),
            ("open_interest", "needs_data"), ("onchain_hash_rate", "needs_data"), ("zigzag", "unsupported"), ("peak", "unsupported"),
            ("rob_booker_reversal", "unsupported"), ("fir", "unsupported"), ("seasonality", "unsupported")] {
            let d = catalogue::get(id).unwrap();
            let e = crate::lab::testdata::compiles(id, d.outputs[0]).unwrap_err();
            assert_eq!(e.status(), status, "{id}");
            assert!(e.message().contains(d.note), "{id}: {}", e.message());
        }
        for d in all().iter().filter(|d| d.compute.is_none()) { assert!(crate::lab::testdata::compiles(d.id, d.outputs[0]).is_err(), "{}", d.id); }
    }
```

In `api.rs` tests add (the existing `catalogue_lists_every_indicator_with_label` must keep passing unchanged):

```rust
    #[test] fn catalogue_lists_notes_needs_and_sources() {
        let j = catalogue_json();
        assert!(crate::lab::json::parse(&j).is_ok());
        assert!(j.contains("\"id\":\"cvd\"") && j.contains("\"note\":\"not available yet — needs intraday (lower-timeframe/tick) bars\""));
        assert!(j.contains("\"tv\":\"Relative Strength Index (RSI)\"") && j.contains("\"ab\":\"RSI; RSIa on close"));
        assert!(j.contains("\"needs\":{\"volume\":false,\"intraday\":false,\"breadth\":false,\"second_symbol\":false,\"fundamentals\":false,\"derivatives\":false,\"onchain\":false}"));
        assert!(j.contains("\"id\":\"correlation\"") && j.contains("\"second_symbol\":true"));
    }
```

In `resample.rs` tests add `#[test] fn civil_round_trips() { for d in ["2021-01-04", "2024-02-29", "2000-03-01", "2020-01-01"] { assert_eq!(civil(day_number(d)), d); } }` (moved from `series.rs`).

- [ ] **Step 2: Run to verify failure**

Run: `cd Context/guru/backend && CARGO_BUILD_JOBS=2 cargo test lab::`
Expected: compile errors (`sources`, `testdata::compiles`, `note`, `any_data`, `civil` not found).

- [ ] **Step 3: Implement the catalogue types**

`catalogue.rs`:

```rust
#[derive(Clone, Copy, Debug, Default)]
pub struct Needs { pub volume: bool, pub intraday: bool, pub breadth: bool, pub second_symbol: bool, pub fundamentals: bool, pub derivatives: bool, pub onchain: bool }
impl Needs {
    pub const NONE: Needs = Needs { volume: false, intraday: false, breadth: false, second_symbol: false, fundamentals: false, derivatives: false, onchain: false };
    /// Data the lab does not hold for every market (volume is per instrument and checked at run time).
    pub const fn any_data(&self) -> bool { self.intraday || self.breadth || self.second_symbol || self.fundamentals || self.derivatives || self.onchain }
}
```
Add to `IndicatorDef` (last field): `/// Why a listed-only entry is not computed ("not available yet — needs …"); empty for computed entries.` `pub note: &'static str,`.
Add `pub const fn int(key: &'static str, label: &'static str, default: f64, min: f64, max: f64) -> ParamSpec { ParamSpec { key, label, default, min, max, step: 1., integer: true } }`.
`all()`: `[super::ma::DEFS, super::momentum::DEFS, super::trend::DEFS, super::volatility::DEFS, super::volume::DEFS, super::price::DEFS, super::listed::DEFS].concat()`.
`run()`: replace the `compute` line with
`let f = def.compute.ok_or_else(|| if def.note.is_empty() { "computed by a later batch".to_string() } else { def.note.to_string() })?;`

`rules.rs` `series_ref` — replace the two listed-entry lines (intraday and `compute.is_none()`):

```rust
    if def.needs.intraday { return Err(LabError::NeedsData(format!("{at}: {}: {}", def.name, def.note))); }
    if def.compute.is_none() {
        let msg = format!("{at}: {}: {}", def.name, def.note);
        return Err(if def.needs.any_data() { LabError::NeedsData(msg) } else { LabError::Unsupported(msg) });
    }
```

`volume.rs`: `listed(…)` gains a final `note: &'static str` argument stored in `note`; both rows pass `"not available yet — needs intraday bars, which are not admitted"`.

`api.rs` `catalogue_json` (seam — edit only this `format!`): the needs object becomes
`\"needs\":{{\"volume\":{},\"intraday\":{},\"breadth\":{},\"second_symbol\":{},\"fundamentals\":{},\"derivatives\":{},\"onchain\":{}}}` with the matching `d.needs.*` arguments, and after `\"definition\":\"{}\"` append `,\"note\":\"{}\",\"tv\":\"{}\",\"ab\":\"{}\"` with `esc(d.note), esc(tv), esc(ab)` where `let (tv, ab) = super::sources::names(d.id);` is computed at the top of the closure.

`resample.rs`: move `civil` out of the `series.rs` test module as `pub fn civil(z: i64) -> String` (same body; doc: `/// Day number (days since 1970-01-01) → "YYYY-MM-DD" (proleptic Gregorian).`).

`testdata.rs`:

```rust
use super::catalogue::{get, run};
use super::resample::{civil, day_number};
use super::rules::{parse_spec, LabError};
use super::series::{compile, Cache, Frames};
// in walk(): replace the date line with consecutive real calendar days from 2020-01-01
        b.date.push(civil(day_number("2020-01-01") + i as i64));
/// Eight OHLCV bars used by the Phase 6 family tests (2024-01-01 is a Monday; index 7 starts week 2).
pub fn d1() -> Bars {
    let mut b = from_ohlc(&[(10., 11., 9., 10.5), (10.5, 12., 10., 11.5), (11.5, 12.5, 11., 12.), (12., 12.2, 10.8, 11.),
        (11., 11.5, 10., 10.2), (10.2, 11., 9.5, 10.8), (10.8, 12., 10.5, 11.9), (11.9, 13., 11.5, 12.6)]);
    b.volume = Some(vec![100., 120., 90., 150., 130., 110., 160., 140.]);
    b
}
pub fn closes(c: &[f64]) -> Bars { from_ohlc(&c.iter().map(|&x| (x, x, x, x)).collect::<Vec<_>>()) }
pub fn go(id: &str, b: &Bars, kv: &[(&str, f64)]) -> Vec<Vec<f64>> {
    let def = get(id).unwrap_or_else(|| panic!("unknown indicator {id}"));
    run(&def, b, &kv.iter().map(|(k, v)| (k.to_string(), *v)).collect()).unwrap_or_else(|e| panic!("{id}: {e}"))
}
/// A one-condition long rule on `id.out`, compiled on 300 synthetic days with volume: proves a rule can reach the series.
pub fn compiles(id: &str, out: &str) -> Result<(), LabError> {
    let spec = parse_spec(&format!(r#"{{"v":1,"entry":{{"all":[{{"lhs":{{"s":"{id}","out":"{out}"}},"op":">","rhs":{{"value":0}}}}]}},"exit":{{"any":[{{"lhs":{{"s":"bars_held"}},"op":">=","rhs":{{"value":1}}}}]}}}}"#))?;
    compile(&spec, &Frames::new(walk(300, 4, true)), &mut Cache::new()).map(|_| ())
}
```

`mod.rs`: add `pub mod listed;` and `pub mod sources;`.

- [ ] **Step 4: Add `sources.rs`**

```rust
//! TradingView and AmiBroker names (aliases included) for every catalogue id. The catalogue JSON shows them as `tv`/`ab`;
//! the Phase 6 master-list test counts coverage from them. Later batches append rows; every id has exactly one row.
pub const SOURCES: &[(&str, &str, &str)] = &[
    ("sma", "Simple Moving Average; Moving Average (charting library); Moving Averages", "MA"),
    ("ema", "Exponential Moving Average; Moving Average Exponential", "EMA"),
    ("wma", "Weighted Moving Average; Moving Average Weighted", "WMA"),
    ("smma", "Smoothed Moving Average", "Wilders (AmiBroker seeds with the first value — converges, Decision 9)"),
    ("swma", "ta.swma (Pine)", ""),
    ("vwma", "Volume-Weighted Moving Average (VWMA)", ""),
    ("hma", "Hull Moving Average", "HMA"),
    ("alma", "Arnaud Legoux Moving Average", ""),
    ("lsma", "Least Squares Moving Average; Linear Regression Curve", "LinearReg"),
    ("dema", "Double Exponential Moving Average (EMA)", "DEMA (AmiBroker seeds with the first value — converges, Decision 9)"),
    ("tema", "Triple EMA", "TEMA (AmiBroker seeds with the first value — converges, Decision 9)"),
    ("kama", "Kaufman's Adaptive Moving Average (KAMA); Moving Average Adaptive", ""),
    ("mcginley", "McGinley Dynamic", ""),
    ("rsi", "Relative Strength Index (RSI)", "RSI; RSIa on close (AmiBroker seeds the Wilder sums at 0 — converges, Decision 9)"),
    ("macd", "Moving average convergence divergence (MACD)", "MACD; Signal"),
    ("stoch", "Stochastic (STOCH)", "StochK; StochD — use variant=1 (ratio of sums)"),
    ("stochrsi", "Stochastic RSI (STOCH RSI)", ""),
    ("cci", "Commodity Channel Index (CCI)", "CCI; CCIa on typical price"),
    ("mom", "Momentum", ""),
    ("roc", "Rate of Change (ROC)", "ROC (absmode not offered)"),
    ("willr", "Williams %R (%R)", ""),
    ("mfi", "Money Flow (MFI)", "MFI"),
    ("tsi", "True Strength Index", ""),
    ("cmo", "Chande Momentum Oscillator (CMO)", ""),
    ("uo", "Ultimate Oscillator (UO)", "Ultimate"),
    ("ao", "Awesome Oscillator (AO)", ""),
    ("ppo", "Percentage Price Oscillator (PPO)", ""),
    ("trix", "TRIX", "TRIX — use variant=1 (percent ROC, no log)"),
    ("kst", "Know Sure Thing (KST)", ""),
    ("dpo", "Detrended Price Oscillator (DPO)", ""),
    ("coppock", "Coppock Curve", ""),
    ("fisher", "Fisher Transform", ""),
    ("crsi", "Connors RSI (CRSI)", ""),
    ("dmi", "Directional Movement (DMI); Average Directional Index (ADX)", "ADX; PDI; MDI"),
    ("supertrend", "Supertrend", ""),
    ("sar", "Parabolic SAR (SAR)", "SAR"),
    ("aroon", "Aroon Indicator", ""),
    ("aroon_osc", "Aroon Oscillator", ""),
    ("ichimoku", "Ichimoku Cloud", ""),
    ("vortex", "Vortex Indicator", ""),
    ("chop", "Choppiness Index (CHOP)", ""),
    ("atr", "Average True Range (ATR)", "ATR"),
    ("bb", "Bollinger Bands (BB)", "BBandTop; BBandBot"),
    ("bbpct", "Bollinger Bands %b (%b)", ""),
    ("bbw", "Bollinger BandWidth (BBW)", ""),
    ("kc", "Keltner Channels (KC)", ""),
    ("donchian", "Donchian Channels (DC); Price Channel", ""),
    ("stdev", "Standard Deviation", "StDev"),
    ("hv", "Historical Volatility; Volatility Close-to-Close", ""),
    ("obv", "On Balance Volume (OBV)", "OBV"),
    ("accdist", "Accumulation Distribution (ADL)", "AccDist"),
    ("cmf", "Chaikin Money Flow (CMF)", ""),
    ("chaikin_osc", "Chaikin Oscillator", "Chaikin"),
    ("pvt", "Price Volume Trend (PVT)", ""),
    ("pvo", "Percentage Volume Oscillator (PVO); Volume Oscillator", "OscV (EMA volume oscillator)"),
    ("efi", "Elder's Force Index (EFI)", ""),
    ("eom", "Ease of Movement (EOM)", ""),
    ("nvi", "Negative Volume Index (NVI)", "NVI (AmiBroker start value unpublished)"),
    ("pvi", "Positive Volume Index (PVI)", "PVI (AmiBroker start value unpublished)"),
    ("highest", "ta.highest (Pine); 52 Week High/Low (high side: length 252 on D, 52 on W)", "HHV (on high); Highest"),
    ("lowest", "ta.lowest (Pine); 52 Week High/Low (low side)", "LLV (on low); Lowest"),
    ("change", "ta.change (Pine)", ""),
    ("pivot_high", "Pivot Points High Low (high)", ""),
    ("pivot_low", "Pivot Points High Low (low)", ""),
    ("vwap", "Volume Weighted Average Price (VWAP) — session anchor", ""),
    ("opening_range", "", ""),
    ("correlation", "Correlation Coefficient (CC)", "Correlation (two symbols); Foreign/SetForeign"),
    ("correlation_log", "Correlation - Log", ""),
    ("ratio", "Ratio", "RelStrength"),
    ("spread", "Spread", ""),
    ("cvd", "Cumulative Volume Delta", ""),
    ("volume_delta", "Volume Delta", ""),
    ("up_down_volume", "Up/Down Volume", ""),
    ("net_volume", "Net Volume", ""),
    ("rvol_at_time", "Relative Volume at Time", ""),
    ("trading_sessions", "Trading Sessions", ""),
    ("volume_24h", "24-hour Volume", ""),
    ("volume_profile", "Volume Profile Fixed Range; Volume Profile Visible Range; Session Volume Profile", "PlotVAPOverlayA"),
    ("rob_booker_intraday_pivots", "Rob Booker Intraday Pivot Points", ""),
    ("analyst_forecast", "Analyst price forecast", ""),
    ("price_target", "Price target - indicator", ""),
    ("dividend_yield", "Dividend Yield", ""),
    ("open_interest", "Open Interest; Understanding crypto open interest", ""),
    ("funding_rate", "Funding rate", ""),
    ("liquidations", "Liquidation data", ""),
    ("long_short_ratio", "Long / Short Ratio Accounts", ""),
    ("long_short_accounts", "Long Short Accounts %", ""),
    ("top_trader_accounts", "Top trader long/short accounts", ""),
    ("top_trader_accounts_ratio", "Top trader long/short accounts ratio", ""),
    ("top_trader_positions", "Top trader long/short positions", ""),
    ("top_trader_positions_ratio", "Top trader long/short positions ratio", ""),
    ("basis", "Basis", ""),
    ("premium", "Premium", ""),
    ("index_price", "Index price", ""),
    ("mark_price", "Mark price", ""),
    ("onchain_1_year_active_supply_pct", "1 year active supply %", ""),
    ("onchain_active_addresses_with_contracts", "Active addresses with contracts", ""),
    ("onchain_addresses_with_balance_gte_x_pct_of_supply", "Addresses with balance ≥ X (% of supply)", ""),
    ("onchain_addresses_with_balance_gte_x_usd", "Addresses with balance ≥ X (USD)", ""),
    ("onchain_average_transaction_volume", "Average transaction volume", ""),
    ("onchain_block_height", "Block height", ""),
    ("onchain_blocks_mined", "Blocks mined", ""),
    ("onchain_created_utxos", "Created UTXOs", ""),
    ("onchain_difficulty", "Difficulty", ""),
    ("onchain_el_salvador_government_balance", "El Salvador Government balance", ""),
    ("onchain_etf_balances", "ETF balances", ""),
    ("onchain_etf_flows", "ETF flows", ""),
    ("onchain_ethereum_new_deposits", "Ethereum new deposits", ""),
    ("onchain_ethereum_new_unique_depositors", "Ethereum new unique depositors", ""),
    ("onchain_ethereum_new_value_staked", "Ethereum new value staked", ""),
    ("onchain_ethereum_total_number_of_deposits", "Ethereum total number of deposits", ""),
    ("onchain_ethereum_total_unique_depositors", "Ethereum total unique depositors", ""),
    ("onchain_ethereum_total_value_staked", "Ethereum total value staked", ""),
    ("onchain_hash_rate", "Hash Rate", ""),
    ("onchain_held_tokens_in_addresses_gte_x_pct_of_supply", "Held tokens in addresses ≥ X (% of supply)", ""),
    ("onchain_held_tokens_in_addresses_gte_x_tokens", "Held tokens in addresses ≥ X (tokens)", ""),
    ("onchain_held_tokens_in_addresses_gte_x_usd", "Held tokens in addresses ≥ X (USD)", ""),
    ("onchain_large_transaction_volume", "Large transaction volume", ""),
    ("onchain_mean_block_interval", "Mean block interval", ""),
    ("onchain_mean_block_size_in_bytes", "Mean block size in bytes", ""),
    ("onchain_mean_gas_used", "Mean gas used", ""),
    ("onchain_mean_transaction_fees", "Mean transaction fees", ""),
    ("onchain_mean_transaction_gas_limit", "Mean transaction gas limit", ""),
    ("onchain_mean_transaction_gas_price", "Mean transaction gas price", ""),
    ("onchain_mean_transaction_size_in_bytes", "Mean transaction size in bytes", ""),
    ("onchain_mean_transfer_volume", "Mean transfer volume", ""),
    ("onchain_mean_utxo_value_created", "Mean UTXO value created", ""),
    ("onchain_mean_utxo_value_spent", "Mean UTXO value spent", ""),
    ("onchain_median_block_interval", "Median block interval", ""),
    ("onchain_median_gas_used", "Median gas used", ""),
    ("onchain_median_transaction_fees", "Median transaction fees", ""),
    ("onchain_median_transaction_gas_limit", "Median transaction gas limit", ""),
    ("onchain_median_transaction_gas_price", "Median transaction gas price", ""),
    ("onchain_median_transfer_volume", "Median transfer volume", ""),
    ("onchain_median_utxo_value_created", "Median UTXO value created", ""),
    ("onchain_median_utxo_value_spent", "Median UTXO value spent", ""),
    ("onchain_power_law_model", "Power-Law Model", ""),
    ("onchain_realized_market_cap", "Realized market cap", ""),
    ("onchain_receiving_addresses", "Receiving addresses", ""),
    ("onchain_rvt_ratio_90_days", "RVT ratio, 90 days", ""),
    ("onchain_sending_addresses", "Sending addresses", ""),
    ("onchain_spent_output_profit_ratio_sopr", "Spent Output Profit Ratio (SOPR)", ""),
    ("onchain_spent_utxos", "Spent UTXOs", ""),
    ("onchain_stock_to_flow_ratio_in_usd", "Stock-to-Flow Ratio in USD", ""),
    ("onchain_supply_equality_ratio", "Supply Equality Ratio", ""),
    ("onchain_total_block_size_in_bytes", "Total block size in bytes", ""),
    ("onchain_total_gas_used", "Total gas used", ""),
    ("onchain_total_transactions_size_in_bytes", "Total transactions size in bytes", ""),
    ("onchain_total_utxo_value_created", "Total UTXO value created", ""),
    ("onchain_total_utxo_value_spent", "Total UTXO value spent", ""),
    ("onchain_total_utxos", "Total UTXOs", ""),
    ("onchain_transaction_fees", "Transaction fees", ""),
    ("onchain_transaction_rate", "Transaction rate", ""),
    ("onchain_transfer_count", "Transfer count", ""),
    ("onchain_transfer_rate", "Transfer rate", ""),
    ("onchain_us_spot_crypto_etf_balances", "US spot crypto ETF balances", ""),
    ("onchain_us_spot_crypto_etf_flows", "US spot crypto ETF flows", ""),
    ("auto_fib_retracement", "Auto Fib Retracement", ""),
    ("auto_fib_extension", "Auto fib extension", ""),
    ("auto_key_levels", "Auto key levels", ""),
    ("auto_pitchfork", "Auto Pitchfork", ""),
    ("auto_trendlines", "Auto trendlines", ""),
    ("bollinger_bars", "Bollinger Bars", ""),
    ("linreg_channel", "Linear Regression (channel)", ""),
    ("seasonality", "Seasonality; Learn using seasonals", ""),
    ("visible_avg_price", "Visible Average Price", ""),
    ("fft", "", "FFT"),
    ("zigzag", "Zigzag indicator; Zig Zag", "Zig"),
    ("peak", "", "Peak"),
    ("trough", "", "Trough"),
    ("peak_bars", "", "PeakBars"),
    ("trough_bars", "", "TroughBars"),
    ("vwap_auto_anchored", "VWAP Auto Anchored", ""),
    ("rob_booker_adx_breakout", "Rob Booker - ADX Breakout", ""),
    ("rob_booker_knoxville", "Rob Booker - Knoxville Divergence", ""),
    ("rob_booker_reversal", "Rob Booker Reversal", ""),
    ("rob_booker_ziv_ghost", "Rob Booker Ziv Ghost Pivots", ""),
    ("rob_booker_missed_pivots", "Rob Booker Missed Pivot Points", ""),
    ("fir", "", "FIR"),
    ("iir", "", "IIR"),
];
/// (TradingView names, AmiBroker names) of an id; empty strings when the id has no row.
pub fn names(id: &str) -> (&'static str, &'static str) { SOURCES.iter().find(|r| r.0 == id).map_or(("", ""), |r| (r.1, r.2)) }
```

- [ ] **Step 5: Add `listed.rs`**

```rust
//! Listed-only entries: TradingView / AmiBroker indicators the lab cannot compute. Each carries its reason as a note;
//! none is ever computed or filled with placeholder values. Rules and charts get needs_data (missing data) or
//! unsupported (look-ahead, drawing, unpublished formula, user code) with the note (rules::series_ref).
use super::catalogue::{IndicatorDef, Needs, Placement};

const SECOND: Needs = Needs { second_symbol: true, ..Needs::NONE };
const INTRADAY: Needs = Needs { intraday: true, ..Needs::NONE };
const FUNDAMENTALS: Needs = Needs { fundamentals: true, ..Needs::NONE };
const DERIVATIVES: Needs = Needs { derivatives: true, ..Needs::NONE };
const ONCHAIN: Needs = Needs { onchain: true, ..Needs::NONE };
const NONE: Needs = Needs::NONE;
const fn l(id: &'static str, name: &'static str, category: &'static str, needs: Needs, note: &'static str) -> IndicatorDef {
    IndicatorDef { id, name, category, params: &[], outputs: &["value"], placement: Placement::Pane, needs, definition: name,
                   first: &[usize::MAX], sparse: false, compute: None, note }
}

pub const DEFS: &[IndicatorDef] = &[
    l("correlation", "Correlation Coefficient (CC)", "Second instrument", SECOND, "not available yet — needs a second instrument in the same request"),
    l("correlation_log", "Correlation - Log", "Second instrument", SECOND, "not available yet — needs a second instrument in the same request"),
    l("ratio", "Ratio", "Second instrument", SECOND, "not available yet — needs a second instrument in the same request"),
    l("spread", "Spread", "Second instrument", SECOND, "not available yet — needs a second instrument in the same request"),
    l("cvd", "Cumulative Volume Delta", "Intraday & order flow", INTRADAY, "not available yet — needs intraday (lower-timeframe/tick) bars"),
    l("volume_delta", "Volume Delta", "Intraday & order flow", INTRADAY, "not available yet — needs intraday (lower-timeframe/tick) bars"),
    l("up_down_volume", "Up/Down Volume", "Intraday & order flow", INTRADAY, "not available yet — needs intraday (lower-timeframe/tick) bars"),
    l("net_volume", "Net Volume", "Intraday & order flow", INTRADAY, "not available yet — needs intraday (lower-timeframe/tick) bars"),
    l("rvol_at_time", "Relative Volume at Time", "Intraday & order flow", INTRADAY, "not available yet — needs intraday (lower-timeframe/tick) bars"),
    l("trading_sessions", "Trading Sessions", "Intraday & order flow", INTRADAY, "not available yet — needs intraday (lower-timeframe/tick) bars"),
    l("volume_24h", "24-hour Volume", "Intraday & order flow", INTRADAY, "not available yet — needs intraday (lower-timeframe/tick) bars"),
    l("volume_profile", "Volume Profile Fixed Range", "Intraday & order flow", INTRADAY, "not available yet — needs intraday (lower-timeframe/tick) bars"),
    l("rob_booker_intraday_pivots", "Rob Booker Intraday Pivot Points", "Intraday & order flow", INTRADAY, "not available yet — needs intraday (lower-timeframe/tick) bars"),
    l("analyst_forecast", "Analyst price forecast", "Fundamentals", FUNDAMENTALS, "not available yet — needs company fundamentals/estimates data"),
    l("price_target", "Price target - indicator", "Fundamentals", FUNDAMENTALS, "not available yet — needs company fundamentals/estimates data"),
    l("dividend_yield", "Dividend Yield", "Fundamentals", FUNDAMENTALS, "not available yet — needs company fundamentals/estimates data"),
    l("open_interest", "Open Interest", "Derivatives market data", DERIVATIVES, "not available yet — needs futures/perpetual-swap market data (open interest, funding, positioning)"),
    l("funding_rate", "Funding rate", "Derivatives market data", DERIVATIVES, "not available yet — needs futures/perpetual-swap market data (open interest, funding, positioning)"),
    l("liquidations", "Liquidation data", "Derivatives market data", DERIVATIVES, "not available yet — needs futures/perpetual-swap market data (open interest, funding, positioning)"),
    l("long_short_ratio", "Long / Short Ratio Accounts", "Derivatives market data", DERIVATIVES, "not available yet — needs futures/perpetual-swap market data (open interest, funding, positioning)"),
    l("long_short_accounts", "Long Short Accounts %", "Derivatives market data", DERIVATIVES, "not available yet — needs futures/perpetual-swap market data (open interest, funding, positioning)"),
    l("top_trader_accounts", "Top trader long/short accounts", "Derivatives market data", DERIVATIVES, "not available yet — needs futures/perpetual-swap market data (open interest, funding, positioning)"),
    l("top_trader_accounts_ratio", "Top trader long/short accounts ratio", "Derivatives market data", DERIVATIVES, "not available yet — needs futures/perpetual-swap market data (open interest, funding, positioning)"),
    l("top_trader_positions", "Top trader long/short positions", "Derivatives market data", DERIVATIVES, "not available yet — needs futures/perpetual-swap market data (open interest, funding, positioning)"),
    l("top_trader_positions_ratio", "Top trader long/short positions ratio", "Derivatives market data", DERIVATIVES, "not available yet — needs futures/perpetual-swap market data (open interest, funding, positioning)"),
    l("basis", "Basis", "Derivatives market data", DERIVATIVES, "not available yet — needs futures/perpetual-swap market data (open interest, funding, positioning)"),
    l("premium", "Premium", "Derivatives market data", DERIVATIVES, "not available yet — needs futures/perpetual-swap market data (open interest, funding, positioning)"),
    l("index_price", "Index price", "Derivatives market data", DERIVATIVES, "not available yet — needs futures/perpetual-swap market data (open interest, funding, positioning)"),
    l("mark_price", "Mark price", "Derivatives market data", DERIVATIVES, "not available yet — needs futures/perpetual-swap market data (open interest, funding, positioning)"),
    l("onchain_1_year_active_supply_pct", "1 year active supply %", "On-chain (crypto)", ONCHAIN, "not available yet — needs blockchain on-chain data (crypto only)"),
    l("onchain_active_addresses_with_contracts", "Active addresses with contracts", "On-chain (crypto)", ONCHAIN, "not available yet — needs blockchain on-chain data (crypto only)"),
    l("onchain_addresses_with_balance_gte_x_pct_of_supply", "Addresses with balance ≥ X (% of supply)", "On-chain (crypto)", ONCHAIN, "not available yet — needs blockchain on-chain data (crypto only)"),
    l("onchain_addresses_with_balance_gte_x_usd", "Addresses with balance ≥ X (USD)", "On-chain (crypto)", ONCHAIN, "not available yet — needs blockchain on-chain data (crypto only)"),
    l("onchain_average_transaction_volume", "Average transaction volume", "On-chain (crypto)", ONCHAIN, "not available yet — needs blockchain on-chain data (crypto only)"),
    l("onchain_block_height", "Block height", "On-chain (crypto)", ONCHAIN, "not available yet — needs blockchain on-chain data (crypto only)"),
    l("onchain_blocks_mined", "Blocks mined", "On-chain (crypto)", ONCHAIN, "not available yet — needs blockchain on-chain data (crypto only)"),
    l("onchain_created_utxos", "Created UTXOs", "On-chain (crypto)", ONCHAIN, "not available yet — needs blockchain on-chain data (crypto only)"),
    l("onchain_difficulty", "Difficulty", "On-chain (crypto)", ONCHAIN, "not available yet — needs blockchain on-chain data (crypto only)"),
    l("onchain_el_salvador_government_balance", "El Salvador Government balance", "On-chain (crypto)", ONCHAIN, "not available yet — needs blockchain on-chain data (crypto only)"),
    l("onchain_etf_balances", "ETF balances", "On-chain (crypto)", ONCHAIN, "not available yet — needs blockchain on-chain data (crypto only)"),
    l("onchain_etf_flows", "ETF flows", "On-chain (crypto)", ONCHAIN, "not available yet — needs blockchain on-chain data (crypto only)"),
    l("onchain_ethereum_new_deposits", "Ethereum new deposits", "On-chain (crypto)", ONCHAIN, "not available yet — needs blockchain on-chain data (crypto only)"),
    l("onchain_ethereum_new_unique_depositors", "Ethereum new unique depositors", "On-chain (crypto)", ONCHAIN, "not available yet — needs blockchain on-chain data (crypto only)"),
    l("onchain_ethereum_new_value_staked", "Ethereum new value staked", "On-chain (crypto)", ONCHAIN, "not available yet — needs blockchain on-chain data (crypto only)"),
    l("onchain_ethereum_total_number_of_deposits", "Ethereum total number of deposits", "On-chain (crypto)", ONCHAIN, "not available yet — needs blockchain on-chain data (crypto only)"),
    l("onchain_ethereum_total_unique_depositors", "Ethereum total unique depositors", "On-chain (crypto)", ONCHAIN, "not available yet — needs blockchain on-chain data (crypto only)"),
    l("onchain_ethereum_total_value_staked", "Ethereum total value staked", "On-chain (crypto)", ONCHAIN, "not available yet — needs blockchain on-chain data (crypto only)"),
    l("onchain_hash_rate", "Hash Rate", "On-chain (crypto)", ONCHAIN, "not available yet — needs blockchain on-chain data (crypto only)"),
    l("onchain_held_tokens_in_addresses_gte_x_pct_of_supply", "Held tokens in addresses ≥ X (% of supply)", "On-chain (crypto)", ONCHAIN, "not available yet — needs blockchain on-chain data (crypto only)"),
    l("onchain_held_tokens_in_addresses_gte_x_tokens", "Held tokens in addresses ≥ X (tokens)", "On-chain (crypto)", ONCHAIN, "not available yet — needs blockchain on-chain data (crypto only)"),
    l("onchain_held_tokens_in_addresses_gte_x_usd", "Held tokens in addresses ≥ X (USD)", "On-chain (crypto)", ONCHAIN, "not available yet — needs blockchain on-chain data (crypto only)"),
    l("onchain_large_transaction_volume", "Large transaction volume", "On-chain (crypto)", ONCHAIN, "not available yet — needs blockchain on-chain data (crypto only)"),
    l("onchain_mean_block_interval", "Mean block interval", "On-chain (crypto)", ONCHAIN, "not available yet — needs blockchain on-chain data (crypto only)"),
    l("onchain_mean_block_size_in_bytes", "Mean block size in bytes", "On-chain (crypto)", ONCHAIN, "not available yet — needs blockchain on-chain data (crypto only)"),
    l("onchain_mean_gas_used", "Mean gas used", "On-chain (crypto)", ONCHAIN, "not available yet — needs blockchain on-chain data (crypto only)"),
    l("onchain_mean_transaction_fees", "Mean transaction fees", "On-chain (crypto)", ONCHAIN, "not available yet — needs blockchain on-chain data (crypto only)"),
    l("onchain_mean_transaction_gas_limit", "Mean transaction gas limit", "On-chain (crypto)", ONCHAIN, "not available yet — needs blockchain on-chain data (crypto only)"),
    l("onchain_mean_transaction_gas_price", "Mean transaction gas price", "On-chain (crypto)", ONCHAIN, "not available yet — needs blockchain on-chain data (crypto only)"),
    l("onchain_mean_transaction_size_in_bytes", "Mean transaction size in bytes", "On-chain (crypto)", ONCHAIN, "not available yet — needs blockchain on-chain data (crypto only)"),
    l("onchain_mean_transfer_volume", "Mean transfer volume", "On-chain (crypto)", ONCHAIN, "not available yet — needs blockchain on-chain data (crypto only)"),
    l("onchain_mean_utxo_value_created", "Mean UTXO value created", "On-chain (crypto)", ONCHAIN, "not available yet — needs blockchain on-chain data (crypto only)"),
    l("onchain_mean_utxo_value_spent", "Mean UTXO value spent", "On-chain (crypto)", ONCHAIN, "not available yet — needs blockchain on-chain data (crypto only)"),
    l("onchain_median_block_interval", "Median block interval", "On-chain (crypto)", ONCHAIN, "not available yet — needs blockchain on-chain data (crypto only)"),
    l("onchain_median_gas_used", "Median gas used", "On-chain (crypto)", ONCHAIN, "not available yet — needs blockchain on-chain data (crypto only)"),
    l("onchain_median_transaction_fees", "Median transaction fees", "On-chain (crypto)", ONCHAIN, "not available yet — needs blockchain on-chain data (crypto only)"),
    l("onchain_median_transaction_gas_limit", "Median transaction gas limit", "On-chain (crypto)", ONCHAIN, "not available yet — needs blockchain on-chain data (crypto only)"),
    l("onchain_median_transaction_gas_price", "Median transaction gas price", "On-chain (crypto)", ONCHAIN, "not available yet — needs blockchain on-chain data (crypto only)"),
    l("onchain_median_transfer_volume", "Median transfer volume", "On-chain (crypto)", ONCHAIN, "not available yet — needs blockchain on-chain data (crypto only)"),
    l("onchain_median_utxo_value_created", "Median UTXO value created", "On-chain (crypto)", ONCHAIN, "not available yet — needs blockchain on-chain data (crypto only)"),
    l("onchain_median_utxo_value_spent", "Median UTXO value spent", "On-chain (crypto)", ONCHAIN, "not available yet — needs blockchain on-chain data (crypto only)"),
    l("onchain_power_law_model", "Power-Law Model", "On-chain (crypto)", ONCHAIN, "not available yet — needs blockchain on-chain data (crypto only)"),
    l("onchain_realized_market_cap", "Realized market cap", "On-chain (crypto)", ONCHAIN, "not available yet — needs blockchain on-chain data (crypto only)"),
    l("onchain_receiving_addresses", "Receiving addresses", "On-chain (crypto)", ONCHAIN, "not available yet — needs blockchain on-chain data (crypto only)"),
    l("onchain_rvt_ratio_90_days", "RVT ratio, 90 days", "On-chain (crypto)", ONCHAIN, "not available yet — needs blockchain on-chain data (crypto only)"),
    l("onchain_sending_addresses", "Sending addresses", "On-chain (crypto)", ONCHAIN, "not available yet — needs blockchain on-chain data (crypto only)"),
    l("onchain_spent_output_profit_ratio_sopr", "Spent Output Profit Ratio (SOPR)", "On-chain (crypto)", ONCHAIN, "not available yet — needs blockchain on-chain data (crypto only)"),
    l("onchain_spent_utxos", "Spent UTXOs", "On-chain (crypto)", ONCHAIN, "not available yet — needs blockchain on-chain data (crypto only)"),
    l("onchain_stock_to_flow_ratio_in_usd", "Stock-to-Flow Ratio in USD", "On-chain (crypto)", ONCHAIN, "not available yet — needs blockchain on-chain data (crypto only)"),
    l("onchain_supply_equality_ratio", "Supply Equality Ratio", "On-chain (crypto)", ONCHAIN, "not available yet — needs blockchain on-chain data (crypto only)"),
    l("onchain_total_block_size_in_bytes", "Total block size in bytes", "On-chain (crypto)", ONCHAIN, "not available yet — needs blockchain on-chain data (crypto only)"),
    l("onchain_total_gas_used", "Total gas used", "On-chain (crypto)", ONCHAIN, "not available yet — needs blockchain on-chain data (crypto only)"),
    l("onchain_total_transactions_size_in_bytes", "Total transactions size in bytes", "On-chain (crypto)", ONCHAIN, "not available yet — needs blockchain on-chain data (crypto only)"),
    l("onchain_total_utxo_value_created", "Total UTXO value created", "On-chain (crypto)", ONCHAIN, "not available yet — needs blockchain on-chain data (crypto only)"),
    l("onchain_total_utxo_value_spent", "Total UTXO value spent", "On-chain (crypto)", ONCHAIN, "not available yet — needs blockchain on-chain data (crypto only)"),
    l("onchain_total_utxos", "Total UTXOs", "On-chain (crypto)", ONCHAIN, "not available yet — needs blockchain on-chain data (crypto only)"),
    l("onchain_transaction_fees", "Transaction fees", "On-chain (crypto)", ONCHAIN, "not available yet — needs blockchain on-chain data (crypto only)"),
    l("onchain_transaction_rate", "Transaction rate", "On-chain (crypto)", ONCHAIN, "not available yet — needs blockchain on-chain data (crypto only)"),
    l("onchain_transfer_count", "Transfer count", "On-chain (crypto)", ONCHAIN, "not available yet — needs blockchain on-chain data (crypto only)"),
    l("onchain_transfer_rate", "Transfer rate", "On-chain (crypto)", ONCHAIN, "not available yet — needs blockchain on-chain data (crypto only)"),
    l("onchain_us_spot_crypto_etf_balances", "US spot crypto ETF balances", "On-chain (crypto)", ONCHAIN, "not available yet — needs blockchain on-chain data (crypto only)"),
    l("onchain_us_spot_crypto_etf_flows", "US spot crypto ETF flows", "On-chain (crypto)", ONCHAIN, "not available yet — needs blockchain on-chain data (crypto only)"),
    l("auto_fib_retracement", "Auto Fib Retracement", "Drawings & display", NONE, "chart drawing/display — not a bar-by-bar series; not usable in rules"),
    l("auto_fib_extension", "Auto fib extension", "Drawings & display", NONE, "chart drawing/display — not a bar-by-bar series; not usable in rules"),
    l("auto_key_levels", "Auto key levels", "Drawings & display", NONE, "chart drawing/display — not a bar-by-bar series; not usable in rules"),
    l("auto_pitchfork", "Auto Pitchfork", "Drawings & display", NONE, "chart drawing/display — not a bar-by-bar series; not usable in rules"),
    l("auto_trendlines", "Auto trendlines", "Drawings & display", NONE, "chart drawing/display — not a bar-by-bar series; not usable in rules"),
    l("bollinger_bars", "Bollinger Bars", "Drawings & display", NONE, "chart drawing/display — not a bar-by-bar series; not usable in rules"),
    l("linreg_channel", "Linear Regression (channel)", "Drawings & display", NONE, "chart drawing/display — not a bar-by-bar series; not usable in rules"),
    l("seasonality", "Seasonality", "Drawings & display", NONE, "chart drawing/display — not a bar-by-bar series; not usable in rules"),
    l("visible_avg_price", "Visible Average Price", "Drawings & display", NONE, "chart drawing/display — not a bar-by-bar series; not usable in rules"),
    l("fft", "FFT", "Drawings & display", NONE, "chart drawing/display — not a bar-by-bar series; not usable in rules"),
    l("zigzag", "Zigzag indicator", "Look-ahead (display only)", NONE, "looks ahead — chart display only, not usable in rules"),
    l("peak", "Peak", "Look-ahead (display only)", NONE, "looks ahead — chart display only, not usable in rules"),
    l("trough", "Trough", "Look-ahead (display only)", NONE, "looks ahead — chart display only, not usable in rules"),
    l("peak_bars", "PeakBars", "Look-ahead (display only)", NONE, "looks ahead — chart display only, not usable in rules"),
    l("trough_bars", "TroughBars", "Look-ahead (display only)", NONE, "looks ahead — chart display only, not usable in rules"),
    l("vwap_auto_anchored", "VWAP Auto Anchored", "Look-ahead (display only)", NONE, "looks ahead — chart display only, not usable in rules"),
    l("rob_booker_adx_breakout", "Rob Booker - ADX Breakout", "Unpublished formulas", NONE, "not computed — TradingView does not publish this formula; awaiting a source review"),
    l("rob_booker_knoxville", "Rob Booker - Knoxville Divergence", "Unpublished formulas", NONE, "not computed — TradingView does not publish this formula; awaiting a source review"),
    l("rob_booker_reversal", "Rob Booker Reversal", "Unpublished formulas", NONE, "not computed — TradingView does not publish this formula; awaiting a source review"),
    l("rob_booker_ziv_ghost", "Rob Booker Ziv Ghost Pivots", "Unpublished formulas", NONE, "not computed — TradingView does not publish this formula; awaiting a source review"),
    l("rob_booker_missed_pivots", "Rob Booker Missed Pivot Points", "Unpublished formulas", NONE, "not computed — TradingView does not publish this formula; awaiting a source review"),
    l("fir", "FIR", "Custom filters", NONE, "not computed — needs a user-supplied coefficient list; the lab runs no user code"),
    l("iir", "IIR", "Custom filters", NONE, "not computed — needs a user-supplied coefficient list; the lab runs no user code"),
];
```

- [ ] **Step 6: Parity doc**

Append to `docs/indicator-parity-fixtures.md`:

```markdown
## Listed only (no fixture possible)

Entries with `computed:false` in the catalogue (second instrument, intraday/tick, fundamentals, derivatives, on-chain,
drawings, look-ahead functions, unpublished formulas, user coefficient filters) have no fixture. Their catalogue `note`
says why. A rule or chart using one returns `needs_data` or `unsupported` with that note.
```

- [ ] **Step 7: Run tests**

Run: `cd Context/guru/backend && CARGO_BUILD_JOBS=2 cargo test lab::` → all pass (the 64 computed Batch A entries keep their `first` on the 1,000-bar walk because `first` does not depend on dates).
Run (repo root): `python3 scripts/check_book_untouched.py --base origin/Manan` → `"problems": []`.

- [ ] **Step 8: Commit**

```bash
git add Context/guru/backend/src/lab docs/indicator-parity-fixtures.md
git commit -m "Add lab catalogue data needs, notes, sources and listed-only TradingView/AmiBroker entries"
```

### Task 2: Market breadth — `Bars::from_ctx` seam and the breadth family

Run this task only after Phase 5's last commit is on `Nandini` (it edits `lab/api.rs`).

**Files:**
- Modify: `Context/guru/backend/src/lab/bars.rs` (Breadth, `Bars.breadth`, `from_ctx`, test)
- Create: `Context/guru/backend/src/lab/breadth.rs`
- Modify: `lab/catalogue.rs` (`run` breadth check, `all`), `lab/series.rs` ("needs breadth" mapping; two generic tests take breadth bars), `lab/testdata.rs` (`with_breadth`, `cut`, `compiles` uses breadth), `lab/api.rs` (seam: every `Bars::from_rows(ctx.r)` → `Bars::from_ctx(ctx)`), `lab/mod.rs`, `lab/sources.rs`, `docs/indicator-parity-fixtures.md`

**Interfaces:**
- Consumes: `crate::connors::Ctx { r, extra: BTreeMap<String, Vec<f64>> }` (read-only), Task 1 `Needs.breadth`.
- Produces: `bars::Breadth { adv, dec, unch, adv_vol, dec_vol, unch_vol: Vec<f64> }`, `Bars.breadth: Option<Breadth>`, `Bars::from_ctx(&Ctx) -> Bars`; `catalogue::run` returns `Err("needs breadth")` when `needs.breadth` and `bars.breadth` is `None`; `testdata::{with_breadth(Bars) -> Bars, cut(&Bars, usize) -> Bars}`; ids `breadth`, `adl`, `ad_ratio`, `cvi`, `trin`.

| id | name | outputs | first | definition |
|---|---|---|---|---|
| breadth | Market Breadth (raw) | advances, declines, unchanged, adv_volume, dec_volume, unch_volume | [0,0,0,0,0,0] | "the market's advancing/declining/unchanged issues and volumes (NSE for India, NYSE for US) aligned to this instrument's dates; NaN where the source has no value; unch_volume is US only" |
| adl | Advance/Decline Line | adl | [0] | "cumulative (advances − declines); a day without breadth is NaN and the total continues" |
| ad_ratio | Advance/Decline Ratio | ratio | [0] | "advances / declines; zero declines → NaN" |
| cvi | Cumulative Volume Index | cvi | [0] | "cumulative (advancing volume − declining volume); missing days NaN, total continues" |
| trin | TRIN (Arms Index) | trin | [0] | "(advances/declines) / (advancing volume/declining volume); any zero → NaN" |

All five: category "Breadth", placement Pane, `needs: Needs { breadth: true, ..Needs::NONE }`, no params, `sparse: false`.

- [ ] **Step 1: Write the failing tests**

`bars.rs` (add a `#[cfg(test)] mod tests`):

```rust
#[cfg(test)]
mod tests {
    use super::Bars;
    #[test] fn from_ctx_reads_the_markets_adv_dec_columns() {
        let rows: Vec<crate::Row> = (0..3).map(|i| crate::Row { ticker: "NIFTY 50".into(), date: format!("2024-01-0{}", i + 1), open: 1., high: 1., low: 1., close: 1., volume: None }).collect();
        let mut ctx = crate::connors::Ctx::new(&rows, &rows, &[]);
        assert!(Bars::from_ctx(&ctx).breadth.is_none(), "no ADV_DEC series → no breadth");
        ctx.extra.insert("NSE_ADV_DEC.Advances".into(), vec![1., 2., f64::NAN]);
        ctx.extra.insert("NSE_ADV_DEC.Declines".into(), vec![3., 4., 5.]);
        ctx.extra.insert("ADV".into(), vec![9., 9., 9.]); // single-column file: not a breadth source
        let br = Bars::from_ctx(&ctx).breadth.unwrap();
        assert_eq!((br.adv[1], br.dec[2]), (2., 5.));
        assert!(br.adv[2].is_nan() && br.adv_vol.iter().all(|v| v.is_nan()) && br.unch_vol.len() == 3);
    }
}
```

`breadth.rs` tests:

```rust
#[cfg(test)]
mod tests {
    use crate::lab::bars::{Bars, Breadth};
    use crate::lab::catalogue::{get, run, Params};
    use crate::lab::resample::Tf;
    use crate::lab::rules::SeriesRef;
    use crate::lab::series::{series, Cache, Frames};
    use crate::lab::testdata::{assert_series, close_eq, closes, compiles, cut, go, walk, with_breadth};
    const N: f64 = f64::NAN;
    fn bars4() -> Bars {
        let mut b = closes(&[1.; 4]);
        b.breadth = Some(Breadth { adv: vec![100., N, 200., 150.], dec: vec![50., 60., 100., 150.], unch: vec![5.; 4],
            adv_vol: vec![1000., N, 3000., 2000.], dec_vol: vec![500., 600., 1000., 4000.], unch_vol: vec![N; 4] });
        b
    }
    #[test] fn breadth_gaps_stay_nan_and_cumulative_lines_continue() {
        let b = bars4();
        assert_series(&go("adl", &b, &[])[0], &[50., N, 150., 150.]);
        assert_series(&go("ad_ratio", &b, &[])[0], &[2., N, 2., 1.]);
        assert_series(&go("cvi", &b, &[])[0], &[500., N, 2500., 500.]);
        assert_series(&go("trin", &b, &[])[0], &[1., N, 2. / 3., 2.]);
        let raw = go("breadth", &b, &[]);
        assert_eq!(raw.len(), 6);
        assert_series(&raw[0], &[100., N, 200., 150.]);
        assert_series(&raw[5], &[N; 4]);
    }
    #[test] fn breadth_needs_daily_market_data() {
        assert_eq!(run(&get("adl").unwrap(), &closes(&[1., 2.]), &Params::new()).unwrap_err(), "needs breadth");
        let fr = Frames::new(with_breadth(walk(60, 3, true)));
        let r = |tf| SeriesRef { id: "adl".into(), params: Params::new(), tf, out: 0, offset: 0 };
        assert!(series(&fr, &mut Cache::new(), &r(Tf::D)).is_ok());
        for tf in [Tf::W, Tf::M] {
            let e = series(&fr, &mut Cache::new(), &r(tf)).unwrap_err();
            assert_eq!(e.status(), "needs_data");
            assert!(e.message().contains("breadth"), "{}", e.message());
        }
    }
    #[test] fn breadth_family_is_causal_finite_and_reachable() {
        let full = with_breadth(walk(400, 11, true));
        for id in ["breadth", "adl", "ad_ratio", "cvi", "trin"] {
            let d = get(id).unwrap();
            let want = run(&d, &full, &Params::new()).unwrap();
            for c in [150usize, 271, 399] {
                let got = run(&d, &cut(&full, c), &Params::new()).unwrap();
                for (k, s) in got.iter().enumerate() { for i in 0..c { assert!(close_eq(s[i], want[k][i]), "{id}.{k} bar {i}"); } }
            }
            for s in &want { assert!(s.iter().all(|v| v.is_finite()), "{id}: synthetic breadth has no gaps"); }
            compiles(id, d.outputs[d.outputs.len() - 1]).unwrap();
        }
    }
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cd Context/guru/backend && CARGO_BUILD_JOBS=2 cargo test lab::breadth lab::bars`
Expected: compile errors (`Breadth`, `from_ctx`, `with_breadth`, `cut` not found).

- [ ] **Step 3: Implement**

`bars.rs`:

```rust
/// Market breadth of the instrument's market, aligned to its daily bars (NaN = no value that day).
#[derive(Clone, Debug, Default)]
pub struct Breadth { pub adv: Vec<f64>, pub dec: Vec<f64>, pub unch: Vec<f64>, pub adv_vol: Vec<f64>, pub dec_vol: Vec<f64>, pub unch_vol: Vec<f64> }
// in struct Bars, last field:
    /// Daily breadth of the market (from extra_data `<MARKET>_ADV_DEC.csv`); None on W/M bars and markets without it.
    pub breadth: Option<Breadth>,
// in from_rows: `breadth: None,`
    /// Daily bars plus the market's advance/decline series when the engine loaded `<MARKET>_ADV_DEC.csv`.
    /// Each engine instance serves one market, so the `_ADV_DEC.` suffix is unique.
    pub fn from_ctx(ctx: &crate::connors::Ctx) -> Bars {
        let mut b = Bars::from_rows(ctx.r);
        let col = |c: &str| ctx.extra.iter().find(|(k, _)| k.ends_with(&format!("_ADV_DEC.{c}"))).map(|(_, v)| v.clone());
        if let (Some(adv), Some(dec)) = (col("Advances"), col("Declines")) {
            let nan = || vec![f64::NAN; adv.len()];
            b.breadth = Some(Breadth { unch: col("Unchanged").unwrap_or_else(nan), adv_vol: col("AdvVolume").unwrap_or_else(nan),
                dec_vol: col("DecVolume").unwrap_or_else(nan), unch_vol: col("UnchVolume").unwrap_or_else(nan), adv, dec });
        }
        b
    }
```
(`resample` builds W/M bars from `Bars::default()`, so they carry no breadth; `Tf::D` clones and keeps it.)

`catalogue.rs` `run()`: after the volume check add `if def.needs.breadth && bars.breadth.is_none() { return Err("needs breadth".into()); }`; `all()` appends `super::breadth::DEFS`. In the generic tests the `computable` filter stays (breadth is tested here).

`series.rs`: in the error mapping add
`"needs breadth" => LabError::NeedsData(format!("{}: needs market breadth (advances/declines) on daily bars; this market or timeframe has none", def.name)),`
and in the tests `signals_never_use_later_bars` and `every_indicator_at_parameter_bounds_never_panics`: build bars with `with_breadth(business_days(…))`, truncate with `crate::lab::testdata::cut(&full_bars, cut)` instead of the inline truncation, and for `d.needs.breadth && tf != Tf::D` assert `series(…).unwrap_err().status() == "needs_data"` and `continue`.

`testdata.rs`:

```rust
use super::bars::Breadth;
/// Deterministic synthetic breadth (no zeros, no gaps) for any bars.
pub fn with_breadth(mut b: Bars) -> Bars {
    let n = b.len();
    let f = |k: usize, m: f64| -> Vec<f64> { (0..n).map(|i| 400. + ((i * k) % 97) as f64 * m).collect() };
    b.breadth = Some(Breadth { adv: f(37, 5.), dec: f(53, 4.), unch: vec![50.; n], adv_vol: f(41, 1e4), dec_vol: f(59, 1e4), unch_vol: vec![1e3; n] });
    b
}
/// The first `n` bars (all columns, breadth included).
pub fn cut(b: &Bars, n: usize) -> Bars {
    let mut p = b.clone();
    for v in [&mut p.open, &mut p.high, &mut p.low, &mut p.close] { v.truncate(n); }
    p.date.truncate(n); p.end.truncate(n);
    if let Some(v) = p.volume.as_mut() { v.truncate(n); }
    if let Some(r) = p.breadth.as_mut() { for v in [&mut r.adv, &mut r.dec, &mut r.unch, &mut r.adv_vol, &mut r.dec_vol, &mut r.unch_vol] { v.truncate(n); } }
    p
}
```
and in `compiles` use `Frames::new(with_breadth(walk(300, 4, true)))`.

`breadth.rs`:

```rust
//! Market breadth family (TradingView Advance/Decline Line, A/D Ratio, CVI; AmiBroker ADLine, Trin, AdvIssues…):
//! statistics of the instrument's market (NSE for India, NYSE for US), daily bars only.
use super::bars::{Bars, Breadth};
use super::catalogue::{IndicatorDef, Needs, Output, Params, Placement};
use super::ta::NA;

const CAT: &str = "Breadth";
const B: Needs = Needs { breadth: true, ..Needs::NONE };
fn br(b: &Bars) -> &Breadth { b.breadth.as_ref().expect("catalogue::run checks needs.breadth") }
fn div(a: f64, b: f64) -> f64 { if b == 0. || a.is_nan() || b.is_nan() { NA } else { a / b } }
/// Running total; a NaN term outputs NaN and the total continues from the last value.
fn cum(x: impl Iterator<Item = f64>) -> Vec<f64> { let mut s = 0.; x.map(|v| if v.is_nan() { NA } else { s += v; s }).collect() }

fn breadth(b: &Bars, _: &Params) -> Output { let r = br(b); Ok(vec![r.adv.clone(), r.dec.clone(), r.unch.clone(), r.adv_vol.clone(), r.dec_vol.clone(), r.unch_vol.clone()]) }
fn adl(b: &Bars, _: &Params) -> Output { let r = br(b); Ok(vec![cum(r.adv.iter().zip(&r.dec).map(|(a, d)| a - d))]) }
fn ad_ratio(b: &Bars, _: &Params) -> Output { let r = br(b); Ok(vec![r.adv.iter().zip(&r.dec).map(|(a, d)| div(*a, *d)).collect()]) }
fn cvi(b: &Bars, _: &Params) -> Output { let r = br(b); Ok(vec![cum(r.adv_vol.iter().zip(&r.dec_vol).map(|(a, d)| a - d))]) }
fn trin(b: &Bars, _: &Params) -> Output {
    let r = br(b);
    Ok(vec![(0..b.len()).map(|i| div(div(r.adv[i], r.dec[i]), div(r.adv_vol[i], r.dec_vol[i]))).collect()])
}
const fn def(id: &'static str, name: &'static str, outputs: &'static [&'static str], definition: &'static str, first: &'static [usize], f: fn(&Bars, &Params) -> Output) -> IndicatorDef {
    IndicatorDef { id, name, category: CAT, params: &[], outputs, placement: Placement::Pane, needs: B, definition, first, sparse: false, compute: Some(f), note: "" }
}
pub const DEFS: &[IndicatorDef] = &[
    def("breadth", "Market Breadth (raw)", &["advances", "declines", "unchanged", "adv_volume", "dec_volume", "unch_volume"],
        "the market's advancing/declining/unchanged issues and volumes (NSE for India, NYSE for US) aligned to this instrument's dates; NaN where the source has no value; unch_volume is US only", &[0, 0, 0, 0, 0, 0], breadth),
    def("adl", "Advance/Decline Line", &["adl"], "cumulative (advances − declines); a day without breadth is NaN and the total continues", &[0], adl),
    def("ad_ratio", "Advance/Decline Ratio", &["ratio"], "advances / declines; zero declines → NaN", &[0], ad_ratio),
    def("cvi", "Cumulative Volume Index", &["cvi"], "cumulative (advancing volume − declining volume); missing days NaN, total continues", &[0], cvi),
    def("trin", "TRIN (Arms Index)", &["trin"], "(advances/declines) / (advancing volume/declining volume); any zero → NaN", &[0], trin),
];
```

`api.rs` (seam): `grep -n "Bars::from_rows(ctx" Context/guru/backend/src/lab/api.rs` and replace each `Bars::from_rows(ctx.r)` with `Bars::from_ctx(ctx)` (three sites at planning time: series, run, optimize). Do not edit any other line.

`sources.rs` rows:

```rust
    ("breadth", "", "AdvIssues; DecIssues; UncIssues; AdvVolume; DecVolume; UncVolume"),
    ("adl", "Advance/Decline Line", "ADLine"),
    ("ad_ratio", "Advance/Decline Ratio", ""),
    ("cvi", "Cumulative Volume Index (CVI)", ""),
    ("trin", "", "Trin"),
```

Parity doc rows: `breadth | - | advances, declines, unchanged, adv_volume, dec_volume, unch_volume`, `adl | - | adl`, `ad_ratio | - | ratio`, `cvi | - | cvi`, `trin | - | trin`, plus one line: "Breadth fixtures need the market's advance/decline data, not the chart symbol; compare against TradingView `ADVN.NS`/`DECN.NS` style symbols only after confirming they match NSE_ADV_DEC (pending)."

- [ ] **Step 4: Run tests**

Run: `CARGO_BUILD_JOBS=2 cargo test lab::` → all pass. Optional real-data check (read-only NAS): `VAULT_IDX="$NAS/idx_data.csv" CARGO_BUILD_JOBS=2 cargo test lab::api` → pass.
Run (repo root): `python3 scripts/check_book_untouched.py --base origin/Manan` → `"problems": []`.

- [ ] **Step 5: Commit**

```bash
git add Context/guru/backend/src/lab docs/indicator-parity-fixtures.md
git commit -m "Add lab market breadth indicators from the engine's advance/decline data"
```

### Task 3: Moving averages II — `ma2.rs`

**Files:** Create `Context/guru/backend/src/lab/ma2.rs`; modify `lab/mod.rs`, `lab/catalogue.rs` (`all`), `lab/sources.rs`, `docs/indicator-parity-fixtures.md`.

**Interfaces:** Consumes `ta::{sma, ema, rma, wma}`, `catalogue::{int, len, real}`. Produces ids `ma_ribbon`, `ma_cross`, `gmma`, `hamming_ma`, `ama`, `ama2`, `iir2`; `pub(super) fn ma_of(b: &Bars, x: &[f64], n: usize, t: usize) -> Result<Vec<f64>, String>` (types 0 SMA, 1 EMA, 2 SMMA, 3 WMA, 4 VWMA → `Err("needs volume")` without volume) and `pub(super) fn ama_s(x: &[f64], alpha: f64, feedback: f64) -> Vec<f64>` for Task 4.

Category "Moving averages", placement Price, needs `Needs::NONE` for all rows.

| id | name | params (default, range) | outputs | first | definition |
|---|---|---|---|---|---|
| ma_ribbon | Moving Average Ribbon | type int 0 (0–4), len1 20, len2 50, len3 100, len4 200 | ma1, ma2, ma3, ma4 | [19,49,99,199] | "ma_k = MA(close, len_k) of type 0 SMA, 1 EMA, 2 SMMA (RMA), 3 WMA, 4 VWMA (type 4 needs volume)" |
| ma_cross | MA Cross | fast 9, slow 21, fast_type int 0 (0–3), slow_type int 0 (0–3) | fast, slow, cross | [8,20,21] | "fast/slow = MA(close, len) of type 0 SMA, 1 EMA, 2 SMMA, 3 WMA; cross = +1 when fast > slow and fast[1] <= slow[1], −1 when fast < slow and fast[1] >= slow[1], else 0; NaN until both MAs exist on this and the previous bar" |
| gmma | Guppy Multiple Moving Average | — | e3, e5, e8, e10, e12, e15, e30, e35, e40, e45, e50, e60 | [2,4,7,9,11,14,29,34,39,44,49,59] | "ema(close, n) for n = 3, 5, 8, 10, 12, 15 (short group) and 30, 35, 40, 45, 50, 60 (long group)" |
| hamming_ma | Moving Average Hamming | length 10 (2–2000) | hamming | [9] | "Σ w_k·close[n−1−k] / Σ w_k, w_k = 0.54 − 0.46·cos(2πk/(n−1)), k = 0 oldest (declared Hamming window; TradingView formula unpublished)" |
| ama | Adaptive Moving Average (AmiBroker AMA) | alpha real 0.1 (0.001–1, 0.001) | ama | [0] | "AmiBroker AMA(close, alpha): out = alpha·close + (1 − alpha)·out[1]; out[0] = close[0]" |
| ama2 | Adaptive Moving Average 2 (AmiBroker AMA2) | alpha real 0.1 (0.001–1, 0.001), feedback real 0.9 (0–1, 0.001) | ama2 | [0] | "AmiBroker AMA2: out = alpha·close + feedback·out[1]; out[0] = close[0]" |
| iir2 | Second-Order IIR Filter (Bandy) | f0 real 0.2 (−2–2, 0.01), f1 real 1.4 (−2–2, 0.01), f2 real −0.6 (−2–2, 0.01) | iir2 | [0] | "Bandy QTS Fig 8.1: out[0] = close[0], out[1] = close[1], out[i] = f0·close + f1·out[i−1] + f2·out[i−2]; once a value is not finite it and every later value are NaN" |

- [ ] **Step 1: Write the failing tests** (in `ma2.rs`)

```rust
#[cfg(test)]
mod tests {
    use crate::lab::catalogue::{get, run, Params};
    use crate::lab::testdata::{assert_series, closes, compiles, d1, go};
    const N: f64 = f64::NAN;
    #[test] fn ma_ribbon_ema_type_and_vwma_needs_volume() {
        let o = go("ma_ribbon", &d1(), &[("type", 1.), ("len1", 2.), ("len2", 3.), ("len3", 4.), ("len4", 5.)]);
        assert_series(&o[0], &[N, 11.0, 11.666666666667, 11.222222222222, 10.540740740741, 10.713580246914, 11.504526748971, 12.234842249657]);
        assert_series(&o[1], &[N, N, 11.333333333333, 11.166666666667, 10.683333333333, 10.741666666667, 11.320833333333, 11.960416666667]);
        assert_series(&o[3], &[N, N, N, N, 11.04, 10.96, 11.273333333333, 11.715555555556]);
        let p: Params = [("type".to_string(), 4.)].into_iter().collect();
        assert_eq!(run(&get("ma_ribbon").unwrap(), &closes(&[1., 2., 3.]), &p).unwrap_err(), "needs volume");
    }
    #[test] fn ma_cross_sma_over_ema() {
        let o = go("ma_cross", &d1(), &[("fast", 2.), ("slow", 3.), ("fast_type", 0.), ("slow_type", 1.)]);
        assert_series(&o[0], &[N, 11.0, 11.75, 11.5, 10.6, 10.5, 11.35, 12.25]);
        assert_series(&o[1], &[N, N, 11.333333333333, 11.166666666667, 10.683333333333, 10.741666666667, 11.320833333333, 11.960416666667]);
        assert_series(&o[2], &[N, N, N, 0., -1., 0., 1., 0.]);
    }
    #[test] fn gmma_short_lines() {
        let o = go("gmma", &d1(), &[]);
        assert_eq!(o.len(), 12);
        assert_series(&o[0], &[N, N, 11.333333333333, 11.166666666667, 10.683333333333, 10.741666666667, 11.320833333333, 11.960416666667]);
        assert_series(&o[1], &[N, N, N, N, 11.04, 10.96, 11.273333333333, 11.715555555556]);
    }
    #[test] fn hamming_len3() {
        assert_series(&go("hamming_ma", &d1(), &[("length", 3.)])[0], &[N, N, 11.465517241379, 11.896551724138, 11.013793103448, 10.296551724138, 10.834482758621, 11.872413793103]);
    }
    #[test] fn ama_and_ama2_seed_with_the_first_close() {
        assert_series(&go("ama", &d1(), &[("alpha", 0.5)])[0], &[10.5, 11.0, 11.5, 11.25, 10.725, 10.7625, 11.33125, 11.965625]);
        assert_series(&go("ama2", &d1(), &[("alpha", 0.5), ("feedback", 0.25)])[0], &[10.5, 8.375, 8.09375, 7.5234375, 6.980859375, 7.14521484375, 7.736303710938, 8.234075927734]);
    }
    #[test] fn iir2_bandy_defaults() {
        assert_series(&go("iir2", &d1(), &[])[0], &[10.5, 11.5, 12.2, 12.38, 12.052, 11.6048, 11.39552, 11.510848]);
    }
    #[test] fn iir2_unstable_coefficients_stay_finite_or_nan() {
        let o = go("iir2", &closes(&[1.; 3000]), &[("f0", 2.), ("f1", 2.), ("f2", 2.)]);
        assert!(o[0].iter().all(|v| !v.is_infinite()));
        assert!(o[0][2999].is_nan(), "a diverging filter ends as NaN, not inf");
    }
    #[test] fn rules_reach_ma2() { for (id, out) in [("ma_cross", "cross"), ("gmma", "e60"), ("ma_ribbon", "ma4")] { compiles(id, out).unwrap(); } }
}
```

- [ ] **Step 2: Run** `CARGO_BUILD_JOBS=2 cargo test lab::ma2` → FAIL (module missing).

- [ ] **Step 3: Implement** `ma2.rs` with the convention header (category "Moving averages"), the table rows as `def(…)` entries and these compute functions:

```rust
pub(super) fn ma_of(b: &Bars, x: &[f64], n: usize, t: usize) -> Result<Vec<f64>, String> {
    Ok(match t {
        0 => ta::sma(x, n), 1 => ta::ema(x, n), 2 => ta::rma(x, n), 3 => ta::wma(x, n),
        _ => {
            let v = b.volume.as_ref().ok_or("needs volume")?;
            let xv: Vec<f64> = x.iter().zip(v).map(|(a, w)| a * w).collect();
            ta::sma(&xv, n).iter().zip(ta::sma(v, n)).map(|(a, d)| div(*a, d)).collect()
        }
    })
}
pub(super) fn ama_s(x: &[f64], alpha: f64, feedback: f64) -> Vec<f64> {
    let mut prev = NA;
    x.iter().map(|&v| { prev = if prev.is_nan() { v } else { alpha * v + feedback * prev }; prev }).collect()
}
fn ma_ribbon(b: &Bars, prm: &Params) -> Output {
    let t = n(prm, "type");
    ["len1", "len2", "len3", "len4"].iter().map(|k| ma_of(b, &b.close, n(prm, k), t)).collect()
}
fn ma_cross(b: &Bars, prm: &Params) -> Output {
    let f = ma_of(b, &b.close, n(prm, "fast"), n(prm, "fast_type"))?;
    let s = ma_of(b, &b.close, n(prm, "slow"), n(prm, "slow_type"))?;
    let cross = (0..b.len()).map(|i| {
        if i == 0 || [f[i], s[i], f[i - 1], s[i - 1]].iter().any(|v| v.is_nan()) { NA }
        else if f[i] > s[i] && f[i - 1] <= s[i - 1] { 1. } else if f[i] < s[i] && f[i - 1] >= s[i - 1] { -1. } else { 0. }
    }).collect();
    Ok(vec![f, s, cross])
}
fn gmma(b: &Bars, _: &Params) -> Output { Ok([3, 5, 8, 10, 12, 15, 30, 35, 40, 45, 50, 60].iter().map(|&k| ta::ema(&b.close, k)).collect()) }
fn hamming_ma(b: &Bars, prm: &Params) -> Output {
    let k = n(prm, "length");
    let w: Vec<f64> = (0..k).map(|j| 0.54 - 0.46 * (2. * std::f64::consts::PI * j as f64 / (k as f64 - 1.)).cos()).collect();
    let ws: f64 = w.iter().sum();
    Ok(vec![(0..b.len()).map(|i| if i + 1 < k { NA } else { b.close[i + 1 - k..=i].iter().zip(&w).map(|(x, w)| x * w).sum::<f64>() / ws }).collect()])
}
fn ama(b: &Bars, prm: &Params) -> Output { let a = p(prm, "alpha"); Ok(vec![ama_s(&b.close, a, 1. - a)]) }
fn ama2(b: &Bars, prm: &Params) -> Output { Ok(vec![ama_s(&b.close, p(prm, "alpha"), p(prm, "feedback"))]) }
fn iir2(b: &Bars, prm: &Params) -> Output {
    let (f0, f1, f2) = (p(prm, "f0"), p(prm, "f1"), p(prm, "f2"));
    let mut out: Vec<f64> = Vec::with_capacity(b.len());
    for i in 0..b.len() {
        let v = if i < 2 { b.close[i] } else { f0 * b.close[i] + f1 * out[i - 1] + f2 * out[i - 2] };
        out.push(if v.is_finite() { v } else { NA });
    }
    Ok(vec![out])
}
```

`sources.rs` rows:

```rust
    ("ma_ribbon", "Moving Average Ribbon; Moving Average Multiple; Moving Average Triple", ""),
    ("ma_cross", "MA Cross; MovingAvg Cross; MovingAvg2Line Cross; EMA Cross; MA with EMA Cross; Moving Average Double", ""),
    ("gmma", "Guppy Multiple Moving Average", ""),
    ("hamming_ma", "Moving Average Hamming", ""),
    ("ama", "", "AMA"),
    ("ama2", "", "AMA2"),
    ("iir2", "", "IIR2 (Bandy QTS book function)"),
```

- [ ] **Step 4: Register and run** (conventions 1–4). `CARGO_BUILD_JOBS=2 cargo test lab::` → PASS; guard → no problems.

- [ ] **Step 5: Commit** `git add Context/guru/backend/src/lab docs/indicator-parity-fixtures.md && git commit -m "Add lab moving-average II indicators (ribbon, cross, GMMA, Hamming, AMA, AMA2, IIR2)"`

### Task 4: Channels and bands — `bands.rs`

**Files:** Create `lab/bands.rs`; modify `lab/mod.rs`, `catalogue.rs` (`all`), `sources.rs`, parity doc.

**Interfaces:** Consumes `ma2::ama_s` (Task 3), `ta::{sma, ema, rma, tr}`. Produces ids `envelope`, `ma_channel`, `apz`, `alligator`, `median`.

Category "Bands & channels", placement Price, needs NONE.

| id | name | params | outputs | first | definition |
|---|---|---|---|---|---|
| envelope | Envelope | length 20, percent real 10 (0.1–100, 0.1), exponential int 0 (0–1) | basis, upper, lower | [19,19,19] | "basis = sma(close, length) (ema if exponential = 1); upper = basis·(1 + percent/100); lower = basis·(1 − percent/100)" |
| ma_channel | Moving Average Channel | upper_length 20, lower_length 20 | upper, lower | [19,19] | "upper = sma(high, upper_length); lower = sma(low, lower_length) (charting-library offsets: use rule offsets)" |
| apz | Adaptive Price Zone (Bandy) | length 50, mult real 2 (0.1–10, 0.1) | basis, upper, lower | [0,0,0] | "a = 2/(√length + 1); basis = AMA(AMA(close, a), a); band = mult·AMA(AMA(high − low, a), a); upper/lower = basis ± band (Bandy QTS Fig 10.6 DblSmoothEMA; AMA seeded with the first value)" |
| alligator | Williams Alligator | jaw_length 13, teeth_length 8, lips_length 5, jaw_offset int 8 (0–100), teeth_offset int 5 (0–100), lips_offset int 3 (0–100) | jaw, teeth, lips | [20,12,7] | "rma(hl2, length) as plotted: the value shown at bar i is rma[i − offset] (forward plot offset; causal)" |
| median | Median | length 3, atr_length 14, mult real 2 (0.1–10, 0.1) | median, upper, lower, ema | [2,13,13,4] | "median = median of hl2 over length (even length: mean of the two middle values); upper/lower = median ± mult·rma(tr, atr_length); ema = ema(median, length) (declared defaults 3/14/2)" |

- [ ] **Step 1: Write the failing tests**

```rust
#[cfg(test)]
mod tests {
    use crate::lab::testdata::{assert_series, compiles, d1, go};
    const N: f64 = f64::NAN;
    #[test] fn envelope_sma_10pct() {
        let o = go("envelope", &d1(), &[("length", 3.), ("percent", 10.), ("exponential", 0.)]);
        assert_series(&o[0], &[N, N, 11.333333333333, 11.5, 11.066666666667, 10.666666666667, 10.966666666667, 11.766666666667]);
        assert_series(&o[1], &[N, N, 12.466666666667, 12.65, 12.173333333333, 11.733333333333, 12.063333333333, 12.943333333333]);
        assert_series(&o[2], &[N, N, 10.2, 10.35, 9.96, 9.6, 9.87, 10.59]);
    }
    #[test] fn ma_channel_high_low() {
        let o = go("ma_channel", &d1(), &[("upper_length", 2.), ("lower_length", 3.)]);
        assert_series(&o[0], &[N, 11.5, 12.25, 12.35, 11.85, 11.25, 11.5, 12.5]);
        assert_series(&o[1], &[N, N, 10.0, 10.6, 10.6, 10.1, 10.0, 10.5]);
    }
    #[test] fn apz_bandy() {
        let o = go("apz", &d1(), &[("length", 4.), ("mult", 2.)]);
        assert_series(&o[0], &[10.5, 10.944444444444, 11.462962962963, 11.314814814815, 10.802880658436, 10.744718792867, 11.251714677641, 11.907285474775]);
        assert_series(&o[1], &[14.5, 14.944444444444, 15.018518518519, 14.485185185185, 13.854732510288, 13.760356652949, 14.256378600823, 14.908657216888]);
        assert_series(&o[2], &[6.5, 6.944444444444, 7.907407407407, 8.144444444444, 7.751028806584, 7.729080932785, 8.247050754458, 8.905913732663]);
    }
    #[test] fn alligator_as_plotted() {
        let o = go("alligator", &d1(), &[("jaw_length", 3.), ("teeth_length", 2.), ("lips_length", 2.), ("jaw_offset", 2.), ("teeth_offset", 1.), ("lips_offset", 1.)]);
        assert_series(&o[0], &[N, N, N, N, 10.916666666667, 11.111111111111, 10.990740740741, 10.743827160494]);
        assert_series(&o[1], &[N, N, 10.5, 11.125, 11.3125, 11.03125, 10.640625, 10.9453125]);
    }
    #[test] fn median_with_atr_bands() {
        let o = go("median", &d1(), &[("length", 3.), ("atr_length", 2.), ("mult", 1.)]);
        assert_series(&o[0], &[N, N, 11.0, 11.5, 11.5, 10.75, 10.75, 11.25]);
        assert_series(&o[1], &[N, N, 12.75, 13.075, 13.0375, 12.26875, 12.259375, 12.7546875]);
        assert_series(&o[2], &[N, N, 9.25, 9.925, 9.9625, 9.23125, 9.240625, 9.7453125]);
        assert_series(&o[3], &[N, N, N, N, 11.333333333333, 11.041666666667, 10.895833333333, 11.072916666667]);
        assert_series(&go("median", &d1(), &[("length", 2.), ("atr_length", 2.), ("mult", 1.)])[0][..3], &[N, 10.5, 11.375]); // even length: mean of the middle two hl2
    }
    #[test] fn rules_reach_bands() { for (id, out) in [("envelope", "lower"), ("apz", "upper"), ("alligator", "lips"), ("median", "ema")] { compiles(id, out).unwrap(); } }
}
```

- [ ] **Step 2: Run** `CARGO_BUILD_JOBS=2 cargo test lab::bands` → FAIL.

- [ ] **Step 3: Implement** (convention header, category "Bands & channels"):

```rust
fn envelope(b: &Bars, prm: &Params) -> Output {
    let k = n(prm, "length"); let pc = p(prm, "percent") / 100.;
    let ba = if p(prm, "exponential") == 1. { ta::ema(&b.close, k) } else { ta::sma(&b.close, k) };
    Ok(vec![ba.clone(), ba.iter().map(|x| x * (1. + pc)).collect(), ba.iter().map(|x| x * (1. - pc)).collect()])
}
fn ma_channel(b: &Bars, prm: &Params) -> Output { Ok(vec![ta::sma(&b.high, n(prm, "upper_length")), ta::sma(&b.low, n(prm, "lower_length"))]) }
fn apz(b: &Bars, prm: &Params) -> Output {
    let a = 2. / ((n(prm, "length") as f64).sqrt() + 1.);
    let ds = |x: &[f64]| super::ma2::ama_s(&super::ma2::ama_s(x, a, 1. - a), a, 1. - a);
    let (basis, rng) = (ds(&b.close), ds(&(0..b.len()).map(|i| b.high[i] - b.low[i]).collect::<Vec<_>>()));
    let m = p(prm, "mult");
    Ok(vec![basis.clone(), (0..b.len()).map(|i| basis[i] + m * rng[i]).collect(), (0..b.len()).map(|i| basis[i] - m * rng[i]).collect()])
}
fn alligator(b: &Bars, prm: &Params) -> Output {
    let h = b.hl2();
    let line = |l: &str, o: &str| { let (v, s) = (ta::rma(&h, n(prm, l)), n(prm, o)); (0..v.len()).map(|i| if i >= s { v[i - s] } else { NA }).collect::<Vec<_>>() };
    Ok(vec![line("jaw_length", "jaw_offset"), line("teeth_length", "teeth_offset"), line("lips_length", "lips_offset")])
}
fn median(b: &Bars, prm: &Params) -> Output {
    let (k, m) = (n(prm, "length"), p(prm, "mult"));
    let h = b.hl2();
    let md: Vec<f64> = (0..b.len()).map(|i| {
        if i + 1 < k { return NA; }
        let mut w = h[i + 1 - k..=i].to_vec();
        if w.iter().any(|v| v.is_nan()) { return NA; }
        w.sort_by(|a, b| a.total_cmp(b));
        if k % 2 == 1 { w[k / 2] } else { (w[k / 2 - 1] + w[k / 2]) / 2. }
    }).collect();
    let atr = ta::rma(&ta::tr(b, true), n(prm, "atr_length"));
    Ok(vec![md.clone(), (0..b.len()).map(|i| md[i] + m * atr[i]).collect(), (0..b.len()).map(|i| md[i] - m * atr[i]).collect(), ta::ema(&md, k)])
}
```

`sources.rs` rows:

```rust
    ("envelope", "Envelope (ENV); Envelopes", ""),
    ("ma_channel", "Moving Average Channel", ""),
    ("apz", "", "Adaptive Price Zone / DblSmoothEMA (Bandy QTS book code)"),
    ("alligator", "Williams Alligator", ""),
    ("median", "Median", ""),
```

- [ ] **Step 4: Register and run** (conventions 1–4) → `cargo test lab::` PASS; guard clean.

- [ ] **Step 5: Commit** `git commit -m "Add lab envelope, MA channel, adaptive price zone, alligator and median bands"` (after `git add Context/guru/backend/src/lab docs/indicator-parity-fixtures.md`).

### Task 5: Regression statistics — `stats.rs` (part 1)

**Files:** Create `lab/stats.rs`; modify `lab/mod.rs`, `catalogue.rs` (`all`), `sources.rs`, parity doc.

**Interfaces:** Produces ids `linreg_slope`, `linreg_intercept`, `tsf`, `stderr`, `se_bands`, `trend_strength`, `rci`, `rci_ribbon`; private helpers `fit(w) -> (alpha, slope)` (least squares on k = 1..n oldest→newest, `y = alpha + slope·k`) and `rolling(x, n, f)` (bar-counted window; any NaN in the window → NaN), which Task 6 reuses.

Category "Statistics", needs NONE.

| id | name | placement | params | outputs | first | definition |
|---|---|---|---|---|---|---|
| linreg_slope | Linear Regression Slope | Pane | length 14 (2–2000) | slope | [13] | "least-squares slope of close per bar over length bars" |
| linreg_intercept | Linear Regression Intercept | Pane | length 14 (2–2000) | intercept | [13] | "AmiBroker LinRegIntercept: a in a + b·k, k = 1..n oldest→newest (= fitted end value − slope·n)" |
| tsf | Time Series Forecast | Price | length 14 (2–2000) | tsf | [13] | "AmiBroker TSF: fitted end value + slope (forecast of the next bar)" |
| stderr | Standard Error | Pane | length 14 (3–2000) | stderr | [13] | "√(Σ(close − fitted)²/(n − 2)) of the least-squares fit (AmiBroker StdErr; charting-library Standard Error)" |
| se_bands | Standard Error Bands | Price | length 21 (3–2000), mult real 2 (0.1–10, 0.1), smooth 3 | basis, upper, lower | [22,22,22] | "basis = sma(linreg(close, length), smooth); upper/lower = sma(linreg ± mult·stderr, smooth) (declared defaults 21/2/3)" |
| trend_strength | Trend Strength Index | Pane | length 14 (2–2000) | tsi | [13] | "Pearson correlation of close with bar index over length; zero variance → NaN (declared default 14)" |
| rci | Rank Correlation Index | Pane | length 10 (2–2000) | rci | [9] | "Spearman: time rank 1 = newest bar, price rank 1 = highest close (ties share the average rank); 100·(1 − 6Σd²/(n(n²−1)))" |
| rci_ribbon | RCI Ribbon | Pane | short 10 (2–2000), middle 30 (2–2000), long 50 (2–2000) | short, middle, long | [9,29,49] | "rci(close, short), rci(close, middle), rci(close, long)" |

- [ ] **Step 1: Write the failing tests**

```rust
#[cfg(test)]
mod tests {
    use crate::lab::testdata::{assert_series, closes, compiles, d1, go};
    const N: f64 = f64::NAN;
    fn one(id: &str, kv: &[(&str, f64)]) -> Vec<f64> { go(id, &d1(), kv).remove(0) }
    #[test] fn regression_parts_len3() {
        let l = [("length", 3.)];
        assert_series(&one("linreg_slope", &l), &[N, N, 0.75, -0.25, -0.9, -0.1, 0.85, 0.9]);
        assert_series(&one("linreg_intercept", &l), &[N, N, 9.833333333333, 12.0, 12.866666666667, 10.866666666667, 9.266666666667, 9.966666666667]);
        assert_series(&one("tsf", &l), &[N, N, 12.833333333333, 11.0, 9.266666666667, 10.466666666667, 12.666666666667, 13.566666666667]);
        assert_series(&one("stderr", &l), &[N, N, 0.204124145232, 0.612372435696, 0.081649658093, 0.571547606649, 0.204124145232, 0.163299316186]);
        assert_series(&one("trend_strength", &l), &[N, N, 0.981980506062, -0.5, -0.997948715789, -0.240192230708, 0.98588695811, 0.9918697838]);
    }
    #[test] fn se_bands_len3() {
        let o = go("se_bands", &d1(), &[("length", 3.), ("mult", 2.), ("smooth", 2.)]);
        assert_series(&o[0], &[N, N, N, 11.666666666667, 10.708333333333, 10.366666666667, 11.191666666667, 12.241666666667]);
        assert_series(&o[1], &[N, N, N, 12.483163247594, 11.402355427122, 11.019863931409, 11.967338418548, 12.609090128084]);
        assert_series(&o[2], &[N, N, N, 10.850170085739, 10.014311239545, 9.713469401924, 10.415994914785, 11.874243205249]);
    }
    #[test] fn rci_len4_and_ties() {
        assert_series(&one("rci", &[("length", 4.)]), &[N, N, N, 40.0, -80.0, -80.0, 40.0, 100.0]);
        // closes 1,2,2,3: price ranks 4, 2.5, 2.5, 1 vs time ranks 4,3,2,1 → Σd² = 0.5 → 100·(1 − 3/60) = 95
        assert_series(&go("rci", &closes(&[1., 2., 2., 3.]), &[("length", 4.)])[0], &[N, N, N, 95.]);
    }
    #[test] fn flat_window_has_no_trend_strength() { assert!(go("trend_strength", &closes(&[5.; 4]), &[("length", 3.)])[0][3].is_nan()); }
    #[test] fn rules_reach_regression() { for (id, out) in [("linreg_slope", "slope"), ("se_bands", "lower"), ("rci_ribbon", "long")] { compiles(id, out).unwrap(); } }
}
```

- [ ] **Step 2: Run** `CARGO_BUILD_JOBS=2 cargo test lab::stats` → FAIL.

- [ ] **Step 3: Implement** (convention header, `CAT = "Statistics"`):

```rust
/// Least squares over w with k = 1..n (oldest → newest): y = alpha + slope·k.
fn fit(w: &[f64]) -> (f64, f64) {
    let nf = w.len() as f64;
    let (mut sk, mut sy, mut skk, mut sky) = (0., 0., 0., 0.);
    for (j, &y) in w.iter().enumerate() { let k = (j + 1) as f64; sk += k; sy += y; skk += k * k; sky += k * y; }
    let den = nf * skk - sk * sk;
    let slope = if den == 0. { 0. } else { (nf * sky - sk * sy) / den };
    (sy / nf - slope * sk / nf, slope)
}
/// f(window) for every bar with a full window of `n` bars; any NaN in the window → NaN (bar-counted, like ta::linreg).
fn rolling(x: &[f64], n: usize, f: impl Fn(&[f64]) -> f64) -> Vec<f64> {
    (0..x.len()).map(|i| if i + 1 < n { NA } else { let w = &x[i + 1 - n..=i]; if w.iter().any(|v| v.is_nan()) { NA } else { f(w) } }).collect()
}
fn stderr_w(w: &[f64]) -> f64 {
    let (a, s) = fit(w);
    (w.iter().enumerate().map(|(j, y)| (y - (a + s * (j + 1) as f64)).powi(2)).sum::<f64>() / (w.len() as f64 - 2.)).sqrt()
}
fn linreg_slope(b: &Bars, prm: &Params) -> Output { Ok(vec![rolling(&b.close, n(prm, "length"), |w| fit(w).1)]) }
fn linreg_intercept(b: &Bars, prm: &Params) -> Output { Ok(vec![rolling(&b.close, n(prm, "length"), |w| fit(w).0)]) }
fn tsf(b: &Bars, prm: &Params) -> Output { Ok(vec![rolling(&b.close, n(prm, "length"), |w| { let (a, s) = fit(w); a + s * (w.len() + 1) as f64 })]) }
fn stderr(b: &Bars, prm: &Params) -> Output { Ok(vec![rolling(&b.close, n(prm, "length"), stderr_w)]) }
fn se_bands(b: &Bars, prm: &Params) -> Output {
    let (k, m, s) = (n(prm, "length"), p(prm, "mult"), n(prm, "smooth"));
    let (lr, se) = (ta::linreg(&b.close, k, 0), rolling(&b.close, k, stderr_w));
    let up: Vec<f64> = (0..b.len()).map(|i| lr[i] + m * se[i]).collect();
    let dn: Vec<f64> = (0..b.len()).map(|i| lr[i] - m * se[i]).collect();
    Ok(vec![ta::sma(&lr, s), ta::sma(&up, s), ta::sma(&dn, s)])
}
fn corr_w(w: &[f64]) -> f64 {
    let nf = w.len() as f64; let (mk, my) = ((nf + 1.) / 2., w.iter().sum::<f64>() / nf);
    let (mut num, mut vk, mut vy) = (0., 0., 0.);
    for (j, y) in w.iter().enumerate() { let dk = (j + 1) as f64 - mk; let dy = y - my; num += dk * dy; vk += dk * dk; vy += dy * dy; }
    div(num, (vk * vy).sqrt())
}
fn trend_strength(b: &Bars, prm: &Params) -> Output { Ok(vec![rolling(&b.close, n(prm, "length"), corr_w)]) }
/// 1-based ranks, 1 = highest value, ties share their average rank.
fn ranks_desc(w: &[f64]) -> Vec<f64> {
    let mut idx: Vec<usize> = (0..w.len()).collect();
    idx.sort_by(|&a, &b| w[b].total_cmp(&w[a]));
    let mut r = vec![0.; w.len()];
    let mut j = 0;
    while j < idx.len() {
        let mut k = j;
        while k + 1 < idx.len() && w[idx[k + 1]] == w[idx[j]] { k += 1; }
        for t in j..=k { r[idx[t]] = (j + k) as f64 / 2. + 1.; }
        j = k + 1;
    }
    r
}
fn rci_s(x: &[f64], n: usize) -> Vec<f64> {
    let nf = n as f64;
    rolling(x, n, |w| { let pr = ranks_desc(w); let d2: f64 = pr.iter().enumerate().map(|(j, p)| ((n - j) as f64 - p).powi(2)).sum(); 100. * (1. - 6. * d2 / (nf * (nf * nf - 1.))) })
}
fn rci(b: &Bars, prm: &Params) -> Output { Ok(vec![rci_s(&b.close, n(prm, "length"))]) }
fn rci_ribbon(b: &Bars, prm: &Params) -> Output { Ok(["short", "middle", "long"].iter().map(|k| rci_s(&b.close, n(prm, k))).collect()) }
```

`sources.rs` rows:

```rust
    ("linreg_slope", "Linear Regression Slope", "LinRegSlope"),
    ("linreg_intercept", "", "LinRegIntercept"),
    ("tsf", "", "TSF"),
    ("stderr", "Standard Error", "StdErr"),
    ("se_bands", "Standard Error Bands", ""),
    ("trend_strength", "Trend Strength Index", ""),
    ("rci", "Rank Correlation Index (RCI)", ""),
    ("rci_ribbon", "RCI Ribbon", ""),
```

- [ ] **Step 4: Register and run** (conventions) → PASS; guard clean.
- [ ] **Step 5: Commit** `git commit -m "Add lab regression statistics (slope, intercept, TSF, standard error, SE bands, trend strength, RCI)"`

### Task 6: Distribution statistics — `stats.rs` (part 2)

**Files:** Modify `lab/stats.rs` (rows + functions), `sources.rs`, parity doc.

**Interfaces:** Consumes `stats::rolling` (Task 5), `catalogue::int`. Produces ids `percentrank`, `percentile`, `zscore`, `skewness`, `kurtosis`, `majority_rule`.

| id | name | placement | params | outputs | first | definition |
|---|---|---|---|---|---|---|
| percentrank | Percent Rank | Pane | length 20, variant int 0 (0–1) | rank | [20] | "100·(count of the previous length closes ≤ close)/length (TradingView ta.percentrank); variant 1: strictly lower (AmiBroker PercentRank)" |
| percentile | Percentile | Price | length 20, pct real 50 (0–100, 1), variant int 0 (0–1) | percentile | [19] | "sorted window of the last length closes: variant 0 linear interpolation at position (n−1)·pct/100 (TradingView ta.percentile_linear_interpolation, AmiBroker Percentile); variant 1 nearest rank ceil(pct/100·n) (minimum 1) (ta.percentile_nearest_rank; AmiBroker Median = pct 50)" |
| zscore | Z-Score (Bandy) | Pane | length 20 | z | [19] | "(close − sma(close, n))/stdev(close, n) (population); zero stdev → NaN" |
| skewness | Skewness | Pane | length 20 (2–2000) | skew | [19] | "population skewness of close: mean((x − m)³)/σ³ (AmiBroker Skewness, population = True); σ = 0 → NaN" |
| kurtosis | Kurtosis | Pane | length 20 (2–2000) | kurt | [19] | "population excess kurtosis: mean((x − m)⁴)/σ⁴ − 3 (AmiBroker Kurtosis, population = True); σ = 0 → NaN" |
| majority_rule | Majority Rule | Pane | length 14 | pct | [14] | "100·(number of bars with close > close[1] in the last length bars)/length (declared; charting-library formula unpublished)" |

- [ ] **Step 1: Write the failing tests** (append to `stats.rs` tests)

```rust
    #[test] fn percentrank_variant_counts_ties_differently() {
        assert_series(&one("percentrank", &[("length", 3.), ("variant", 0.)]), &[N, N, N, 33.333333333333, 0.0, 33.333333333333, 100.0, 100.0]);
        let t = closes(&[1., 2., 2., 2.]);
        assert_series(&go("percentrank", &t, &[("length", 3.), ("variant", 0.)])[0], &[N, N, N, 100.]);
        assert_series(&go("percentrank", &t, &[("length", 3.), ("variant", 1.)])[0], &[N, N, N, 33.333333333333]);
    }
    #[test] fn percentile_interpolated_and_nearest_rank() {
        assert_series(&one("percentile", &[("length", 4.), ("pct", 50.), ("variant", 0.)]), &[N, N, N, 11.25, 11.25, 10.9, 10.9, 11.35]);
        assert_series(&one("percentile", &[("length", 4.), ("pct", 50.), ("variant", 1.)]), &[N, N, N, 11.0, 11.0, 10.8, 10.8, 10.8]);
        assert_series(&one("percentile", &[("length", 4.), ("pct", 30.), ("variant", 0.)]), &[N, N, N, 10.95, 10.92, 10.74, 10.74, 10.74]);
    }
    #[test] fn percentile_bounds_pick_min_and_max() {
        let t = closes(&[3., 1., 2.]);
        assert_series(&go("percentile", &t, &[("length", 3.), ("pct", 0.), ("variant", 0.)])[0], &[N, N, 1.]);
        assert_series(&go("percentile", &t, &[("length", 3.), ("pct", 0.), ("variant", 1.)])[0], &[N, N, 1.]);
        assert_series(&go("percentile", &t, &[("length", 3.), ("pct", 100.), ("variant", 1.)])[0], &[N, N, 3.]);
        assert_series(&go("percentile", &t, &[("length", 3.), ("pct", 100.), ("variant", 0.)])[0], &[N, N, 3.]);
    }
    #[test] fn moments_and_zscore() {
        assert_series(&one("zscore", &[("length", 3.)]), &[N, N, 1.06904496765, -1.224744871392, -1.176964698553, 0.392232270276, 1.325838426669, 1.124803176665]);
        assert_series(&one("skewness", &[("length", 4.)]), &[N, N, N, 0.0, -0.28629553356, 0.440866714177, 0.353461337519, 0.05239638633]);
        assert_series(&one("kurtosis", &[("length", 4.)]), &[N, N, N, -1.36, -1.234304760038, -1.0, -1.022679189323, -1.571600570393]);
        assert!(go("skewness", &closes(&[2.; 4]), &[("length", 3.)])[0][3].is_nan());
    }
    #[test] fn majority_rule_len3() { assert_series(&one("majority_rule", &[("length", 3.)]), &[N, N, N, 66.666666666667, 33.333333333333, 33.333333333333, 66.666666666667, 100.0]); }
    #[test] fn rules_reach_distribution() { for (id, out) in [("percentrank", "rank"), ("percentile", "percentile"), ("kurtosis", "kurt")] { compiles(id, out).unwrap(); } }
```

- [ ] **Step 2: Run** `CARGO_BUILD_JOBS=2 cargo test lab::stats` → FAIL (unknown ids).

- [ ] **Step 3: Implement**

```rust
fn percentrank(b: &Bars, prm: &Params) -> Output {
    let (k, strict, x) = (n(prm, "length"), p(prm, "variant") == 1., &b.close);
    Ok(vec![(0..x.len()).map(|i| if i < k || x[i].is_nan() { NA } else {
        100. * x[i - k..i].iter().filter(|&&v| if strict { v < x[i] } else { v <= x[i] }).count() as f64 / k as f64
    }).collect()])
}
fn percentile(b: &Bars, prm: &Params) -> Output {
    let (k, pct, nearest) = (n(prm, "length"), p(prm, "pct"), p(prm, "variant") == 1.);
    Ok(vec![rolling(&b.close, k, |w| {
        let mut s = w.to_vec(); s.sort_by(|a, b| a.total_cmp(b));
        if nearest { s[((pct / 100. * k as f64).ceil() as usize).clamp(1, k) - 1] }
        else { let pos = pct / 100. * (k as f64 - 1.); let lo = pos.floor() as usize; let hi = (lo + 1).min(k - 1); s[lo] + (s[hi] - s[lo]) * (pos - lo as f64) }
    })])
}
fn zscore(b: &Bars, prm: &Params) -> Output {
    let k = n(prm, "length"); let (m, s) = (ta::sma(&b.close, k), ta::stdev(&b.close, k));
    Ok(vec![(0..b.len()).map(|i| div(b.close[i] - m[i], s[i])).collect()])
}
fn moment(x: &[f64], k: usize, pow: i32) -> Vec<f64> {
    rolling(x, k, |w| {
        let nf = w.len() as f64; let m = w.iter().sum::<f64>() / nf;
        let sd = (w.iter().map(|v| (v - m).powi(2)).sum::<f64>() / nf).sqrt();
        if sd == 0. { return NA; }
        let r = w.iter().map(|v| (v - m).powi(pow)).sum::<f64>() / nf / sd.powi(pow);
        if pow == 4 { r - 3. } else { r }
    })
}
fn skewness(b: &Bars, prm: &Params) -> Output { Ok(vec![moment(&b.close, n(prm, "length"), 3)]) }
fn kurtosis(b: &Bars, prm: &Params) -> Output { Ok(vec![moment(&b.close, n(prm, "length"), 4)]) }
fn majority_rule(b: &Bars, prm: &Params) -> Output {
    let k = n(prm, "length"); let c = &b.close;
    let up: Vec<f64> = (0..c.len()).map(|i| if i == 0 { NA } else if c[i] > c[i - 1] { 1. } else { 0. }).collect();
    Ok(vec![ta::sum(&up, k).iter().map(|s| 100. * s / k as f64).collect()])
}
```
(A constant window gives σ = 0 → NaN; `sd == 0.` is exact because the deviations are exactly 0.)

`sources.rs` rows:

```rust
    ("percentrank", "ta.percentrank (Pine)", "PercentRank (variant=1)"),
    ("percentile", "ta.percentile_linear_interpolation; ta.percentile_nearest_rank (Pine)", "Percentile; Median (variant=1, pct=50)"),
    ("zscore", "", "zscore / ExampleZScore (Bandy QTS book functions)"),
    ("skewness", "", "Skewness"),
    ("kurtosis", "", "Kurtosis"),
    ("majority_rule", "Majority Rule", ""),
```

- [ ] **Step 4: Run** `cargo test lab::` → PASS; guard clean.
- [ ] **Step 5: Commit** `git commit -m "Add lab percent rank, percentile, z-score, skewness, kurtosis and majority rule"`

### Task 7: Oscillators I — `osc2.rs` (part 1)

**Files:** Create `lab/osc2.rs`; modify `lab/mod.rs`, `catalogue.rs` (`all`), `sources.rs`, parity doc.

**Interfaces:** Produces ids `bop`, `bbp`, `accel_osc`, `price_osc`, `woodies_cci`, `rmi`, `cg_osc` (`bbp` outputs `bull`/`bear` are consumed by Task 16). Private helpers `cci_s(x, n)` and `rsi_like(up, dn, n)`, which Task 8 may reuse.

Category "Momentum", placement Pane, needs NONE.

| id | name | params | outputs | first | definition |
|---|---|---|---|---|---|
| bop | Balance of Power | — | bop | [0] | "(close − open)/(high − low); zero range → NaN" |
| bbp | Bull Bear Power | length 13 | bbp, bull, bear | [12,12,12] | "bull = high − ema(close, n); bear = low − ema(close, n); bbp = bull + bear (the TradingView plot)" |
| accel_osc | Accelerator Oscillator | fast 5, slow 34, smooth 5 | ac | [37] | "ao = sma(hl2, fast) − sma(hl2, slow); ac = ao − sma(ao, smooth)" |
| price_osc | Price Oscillator | fast 10, slow 21, exponential int 0 (0–1) | po | [20] | "100·(ma(close, fast) − ma(close, slow))/ma(close, slow), ma = sma (ema if exponential = 1; AmiBroker OscP = exponential 1)" |
| woodies_cci | Woodies CCI | length 14, turbo 6 | cci, turbo | [13,5] | "cci of close (not hlc3) over length and over turbo: (close − sma)/(0.015·mean absolute deviation); zero deviation → NaN" |
| rmi | Relative Momentum Index | length 20, momentum 5 | rmi | [24] | "up = max(close − close[m], 0), dn = max(close[m] − close, 0); RSI formula on rma(up, n), rma(dn, n) (dn 0 → 100, up 0 → 0) (AmiBroker RMI; rma seeded as TradingView)" |
| cg_osc | Center of Gravity (Ehlers) | length 10 | cg, trigger | [9,10] | "cg = −Σ_{i=0}^{n−1}(i+1)·hl2[t−i] / Σ hl2[t−i]; trigger = cg[1] (declared Ehlers form for Bandy's CGOscillator)" |

- [ ] **Step 1: Write the failing tests**

```rust
#[cfg(test)]
mod tests {
    use crate::lab::testdata::{assert_series, compiles, d1, go};
    const N: f64 = f64::NAN;
    #[test] fn bop_and_bull_bear_power() {
        assert_series(&go("bop", &d1(), &[])[0], &[0.25, 0.5, 0.333333333333, -0.714285714286, -0.533333333333, 0.4, 0.733333333333, 0.466666666667]);
        let o = go("bbp", &d1(), &[("length", 3.)]);
        assert_series(&o[0], &[N, N, 0.833333333333, 0.666666666667, 0.133333333333, -0.983333333333, -0.141666666667, 0.579166666667]);
        assert_series(&o[1], &[N, N, 1.166666666667, 1.033333333333, 0.816666666667, 0.258333333333, 0.679166666667, 1.039583333333]);
        assert_series(&o[2], &[N, N, -0.333333333333, -0.366666666667, -0.683333333333, -1.241666666667, -0.820833333333, -0.460416666667]);
    }
    #[test] fn accelerator_and_price_oscillator() {
        assert_series(&go("accel_osc", &d1(), &[("fast", 2.), ("slow", 3.), ("smooth", 2.)])[0], &[N, N, N, -0.125, -0.208333333333, -0.0625, 0.166666666667, 0.25]);
        assert_series(&go("price_osc", &d1(), &[("fast", 2.), ("slow", 3.), ("exponential", 0.)])[0], &[N, N, 3.676470588235, 0.0, -4.21686746988, -1.5625, 3.495440729483, 4.107648725212]);
    }
    #[test] fn woodies_cci_uses_close() {
        let o = go("woodies_cci", &d1(), &[("length", 3.), ("turbo", 2.)]);
        assert_series(&o[0], &[N, N, 80.0, -100.0, -92.857142857143, 28.571428571429, 100.0, 86.206896551724]);
        assert_series(&o[1], &[N, 66.666666666667, 66.666666666667, -66.666666666667, -66.666666666667, 66.666666666667, 66.666666666667, 66.666666666667]);
    }
    #[test] fn rmi_and_center_of_gravity() {
        assert_series(&go("rmi", &d1(), &[("length", 2.), ("momentum", 2.)])[0], &[N, N, N, 75.0, 26.785714285714, 23.4375, 75.5, 89.959016393443]);
        let o = go("cg_osc", &d1(), &[("length", 3.)]);
        assert_series(&o[0], &[N, N, -1.946564885496, -1.985401459854, -2.029411764706, -2.038461538462, -1.984496124031, -1.940740740741]);
        assert_series(&o[1], &[N, N, N, -1.946564885496, -1.985401459854, -2.029411764706, -2.038461538462, -1.984496124031]);
    }
    #[test] fn rules_reach_osc1() { for (id, out) in [("bbp", "bear"), ("woodies_cci", "turbo"), ("cg_osc", "trigger")] { compiles(id, out).unwrap(); } }
}
```

- [ ] **Step 2: Run** `CARGO_BUILD_JOBS=2 cargo test lab::osc2` → FAIL.

- [ ] **Step 3: Implement** (convention header, `CAT = "Momentum"`):

```rust
fn zip(a: &[f64], b: &[f64], f: impl Fn(f64, f64) -> f64) -> Vec<f64> { a.iter().zip(b).map(|(&x, &y)| f(x, y)).collect() }
pub(super) fn cci_s(x: &[f64], k: usize) -> Vec<f64> {
    let (m, d) = (ta::sma(x, k), ta::dev(x, k));
    (0..x.len()).map(|i| div(x[i] - m[i], 0.015 * d[i])).collect()
}
/// RSI-style ratio of two smoothed series: dn 0 → 100, up 0 → 0.
pub(super) fn rsi_like(up: &[f64], dn: &[f64], k: usize) -> Vec<f64> {
    zip(&ta::rma(up, k), &ta::rma(dn, k), |u, d| if u.is_nan() || d.is_nan() { NA } else if d == 0. { 100. } else if u == 0. { 0. } else { 100. - 100. / (1. + u / d) })
}
fn bop(b: &Bars, _: &Params) -> Output { Ok(vec![(0..b.len()).map(|i| div(b.close[i] - b.open[i], b.high[i] - b.low[i])).collect()]) }
fn bbp(b: &Bars, prm: &Params) -> Output {
    let e = ta::ema(&b.close, n(prm, "length"));
    let bull = zip(&b.high, &e, |h, e| h - e); let bear = zip(&b.low, &e, |l, e| l - e);
    Ok(vec![zip(&bull, &bear, |a, c| a + c), bull, bear])
}
fn accel_osc(b: &Bars, prm: &Params) -> Output {
    let h = b.hl2(); let ao = zip(&ta::sma(&h, n(prm, "fast")), &ta::sma(&h, n(prm, "slow")), |a, c| a - c);
    Ok(vec![zip(&ao, &ta::sma(&ao, n(prm, "smooth")), |a, c| a - c)])
}
fn price_osc(b: &Bars, prm: &Params) -> Output {
    let ma = |k| if p(prm, "exponential") == 1. { ta::ema(&b.close, k) } else { ta::sma(&b.close, k) };
    Ok(vec![zip(&ma(n(prm, "fast")), &ma(n(prm, "slow")), |f, s| div(100. * (f - s), s))])
}
fn woodies_cci(b: &Bars, prm: &Params) -> Output { Ok(vec![cci_s(&b.close, n(prm, "length")), cci_s(&b.close, n(prm, "turbo"))]) }
fn rmi(b: &Bars, prm: &Params) -> Output {
    let ch = ta::change(&b.close, n(prm, "momentum"));
    let up: Vec<f64> = ch.iter().map(|v| if v.is_nan() { NA } else { v.max(0.) }).collect();
    let dn: Vec<f64> = ch.iter().map(|v| if v.is_nan() { NA } else { (-v).max(0.) }).collect();
    Ok(vec![rsi_like(&up, &dn, n(prm, "length"))])
}
fn cg_osc(b: &Bars, prm: &Params) -> Output {
    let (h, k) = (b.hl2(), n(prm, "length"));
    let cg: Vec<f64> = (0..b.len()).map(|t| if t + 1 < k { NA } else {
        let (num, den) = (0..k).fold((0., 0.), |(a, d), i| (a + (i + 1) as f64 * h[t - i], d + h[t - i]));
        div(-num, den)
    }).collect();
    let trig = (0..cg.len()).map(|i| if i == 0 { NA } else { cg[i - 1] }).collect();
    Ok(vec![cg, trig])
}
```

`sources.rs` rows:

```rust
    ("bop", "Balance of Power (BOP)", ""),
    ("bbp", "Bull Bear Power", ""),
    ("accel_osc", "Accelerator Oscillator", ""),
    ("price_osc", "Price Oscillator", "OscP (exponential=1)"),
    ("woodies_cci", "Woodies CCI", ""),
    ("rmi", "", "RMI"),
    ("cg_osc", "", "CGOscillator (Bandy QTS book code; Ehlers Center of Gravity)"),
```

- [ ] **Step 4: Register and run** → PASS; guard clean.
- [ ] **Step 5: Commit** `git commit -m "Add lab oscillators I (balance of power, bull bear power, accelerator, price oscillator, Woodies CCI, RMI, center of gravity)"`

### Task 8: Oscillators II — `osc2.rs` (part 2)

**Files:** Modify `lab/osc2.rs`, `sources.rs`, parity doc.

**Interfaces:** Produces ids `pmo`, `special_k`, `smi_ergodic`, `smi`, `rvgi`, `rvi`.

| id | name | params | outputs | first | sparse | definition |
|---|---|---|---|---|---|---|
| pmo | Price Momentum Oscillator | length1 35, length2 20, signal 10 | pmo, signal | [54,63] | no | "roc1 = 100·(close/close[1] − 1); csf(x, L): seeded with sma(x, L) at its first value, then s += (x − s)·2/L; pmo = csf(10·csf(roc1, length1), length2); signal = ema(pmo, signal) (seed declared)" |
| special_k | Pring's Special K | signal1 100, signal2 100 | sk, signal | [724,922] | no | "Σ w·sma(roc(close, r), m) over (r, m, w) = (10,10,1), (15,10,2), (20,10,3), (30,15,4), (40,50,1), (65,65,2), (75,75,3), (100,100,4), (195,130,1), (265,130,2), (390,130,3), (530,195,4); signal = sma(sma(sk, signal1), signal2)" |
| smi_ergodic | SMI Ergodic | long 20, short 5, signal 5 | smi, signal, osc | [24,28,28] | no | "smi = ema(ema(Δclose, long), short)/ema(ema(\|Δclose\|, long), short) (ta.tsi ratio, not ×100); signal = ema(smi, signal); osc = smi − signal (Indicator + Oscillator)" |
| smi | Stochastic Momentum Index | k 10, d 3, ema 3 | smi, signal | [13,15] | no | "hh/ll = highest(high, k)/lowest(low, k); smi = 200·ema(ema(close − (hh + ll)/2, d), d)/ema(ema(hh − ll, d), d); signal = ema(smi, ema)" |
| rvgi | Relative Vigor Index | length 10 | rvi, signal | [12,15] | no | "rvi = sum(swma(close − open), n)/sum(swma(high − low), n); signal = swma(rvi)" |
| rvi | Relative Volatility Index | length 10, ema 14 | rvi | [MAX] | **yes** | "sd = stdev(close, length); u = change(close) ≤ 0 ? 0 : sd; d = change(close) > 0 ? 0 : sd (bar 0: neither → sd); rvi = 100·ema(u, ema)/(ema(u, ema) + ema(d, ema)); warm-up depends on the data (zeros before stdev exists count, as on TradingView)" |

- [ ] **Step 1: Write the failing tests** (append)

```rust
    use crate::lab::testdata::closes;
    #[test] fn pmo_short() {
        let o = go("pmo", &d1(), &[("length1", 2.), ("length2", 2.), ("signal", 2.)]);
        assert_series(&o[0], &[N, N, N, -6.987577639752, -72.727272727273, 58.823529411765, 101.851851851852, 58.823529411765]);
        assert_series(&o[1], &[N, N, N, N, -39.857425183512, 25.929877880006, 76.54452719457, 64.7305286727]);
    }
    #[test] fn special_k_on_constant_growth() {
        // close_i = 100·1.01^i → every ROC(r) is 100·(1.01^r − 1): sk = Σ w·100·(1.01^r − 1) = 96572.2974214603 from bar 724
        let o = go("special_k", &closes(&(0..1000).map(|i| 100. * 1.01f64.powi(i)).collect::<Vec<_>>()), &[]);
        assert!(o[0][723].is_nan() && o[1][921].is_nan());
        assert!((o[0][724] - 96572.2974214603).abs() < 1e-6 && (o[1][922] - 96572.2974214603).abs() < 1e-6);
    }
    #[test] fn smi_ergodic_and_smi() {
        let o = go("smi_ergodic", &d1(), &[("long", 3.), ("short", 2.), ("signal", 2.)]);
        assert_series(&o[0], &[N, N, N, N, -0.090909090909, 0.092936802974, 0.513043478261, 0.715739868049]);
        assert_series(&o[2], &[N, N, N, N, N, 0.091922946942, 0.170676540743, 0.12445764351]);
        let s = go("smi", &d1(), &[("k", 3.), ("d", 2.), ("ema", 2.)]);
        assert_series(&s[0], &[N, N, N, N, -1.176470588235, -10.690121786198, 31.700423928403, 59.571840263483]);
        assert_series(&s[1], &[N, N, N, N, N, -5.933296187216, 19.15585055653, 46.099843694499]);
    }
    #[test] fn relative_vigor_and_volatility() {
        let v = go("rvgi", &d1(), &[("length", 2.)]);
        assert_series(&v[0], &[N, N, N, N, 0.086294416244, -0.182320441989, -0.158192090395, 0.167597765363]);
        assert_series(&v[1], &[N, N, N, N, N, N, N, -0.07118881386]);
        assert_series(&go("rvi", &d1(), &[("length", 2.), ("ema", 2.)])[0], &[N, N, 100.0, 27.272727272727, 9.933774834437, 62.942779291553, 91.254019292604, 96.442584357834]);
    }
    #[test] fn rules_reach_osc2() { for (id, out) in [("pmo", "signal"), ("smi_ergodic", "osc"), ("rvi", "rvi")] { compiles(id, out).unwrap(); } }
```

- [ ] **Step 2: Run** `CARGO_BUILD_JOBS=2 cargo test lab::osc2` → FAIL.

- [ ] **Step 3: Implement**

```rust
fn roc_s(x: &[f64], k: usize) -> Vec<f64> { (0..x.len()).map(|i| if i >= k { div(100. * (x[i] - x[i - k]), x[i - k]) } else { NA }).collect() }
/// DecisionPoint custom smoothing: SMA seed, then alpha = 2/L (not 2/(L+1)).
fn csf(x: &[f64], k: usize) -> Vec<f64> {
    let seed = ta::sma(x, k); let mut prev = NA;
    x.iter().enumerate().map(|(i, &v)| { if v.is_nan() { return NA; } prev = if prev.is_nan() { seed[i] } else { prev + (v - prev) * 2. / k as f64 }; prev }).collect()
}
fn pmo(b: &Bars, prm: &Params) -> Output {
    let r1: Vec<f64> = csf(&roc_s(&b.close, 1), n(prm, "length1")).iter().map(|v| 10. * v).collect();
    let pm = csf(&r1, n(prm, "length2"));
    let sig = ta::ema(&pm, n(prm, "signal"));
    Ok(vec![pm, sig])
}
const SK: [(usize, usize, f64); 12] = [(10, 10, 1.), (15, 10, 2.), (20, 10, 3.), (30, 15, 4.), (40, 50, 1.), (65, 65, 2.), (75, 75, 3.), (100, 100, 4.), (195, 130, 1.), (265, 130, 2.), (390, 130, 3.), (530, 195, 4.)];
fn special_k(b: &Bars, prm: &Params) -> Output {
    let mut sk = vec![0.; b.len()];
    for (r, m, w) in SK { let s = ta::sma(&roc_s(&b.close, r), m); for i in 0..b.len() { sk[i] += w * s[i]; } }
    let sig = ta::sma(&ta::sma(&sk, n(prm, "signal1")), n(prm, "signal2"));
    Ok(vec![sk, sig])
}
fn smi_ergodic(b: &Bars, prm: &Params) -> Output {
    let (lg, sh) = (n(prm, "long"), n(prm, "short"));
    let d = ta::change(&b.close, 1); let ad: Vec<f64> = d.iter().map(|v| v.abs()).collect();
    let erg = zip(&ta::ema(&ta::ema(&d, lg), sh), &ta::ema(&ta::ema(&ad, lg), sh), div);
    let sig = ta::ema(&erg, n(prm, "signal"));
    let osc = zip(&erg, &sig, |a, c| a - c);
    Ok(vec![erg, sig, osc])
}
fn smi(b: &Bars, prm: &Params) -> Output {
    let (k, d) = (n(prm, "k"), n(prm, "d"));
    let (hh, ll) = (ta::highest(&b.high, k), ta::lowest(&b.low, k));
    let rel: Vec<f64> = (0..b.len()).map(|i| b.close[i] - (hh[i] + ll[i]) / 2.).collect();
    let rng: Vec<f64> = (0..b.len()).map(|i| hh[i] - ll[i]).collect();
    let ee = |x: &[f64]| ta::ema(&ta::ema(x, d), d);
    let s = zip(&ee(&rel), &ee(&rng), |a, c| div(200. * a, c));
    let sig = ta::ema(&s, n(prm, "ema"));
    Ok(vec![s, sig])
}
fn swma(x: &[f64]) -> Vec<f64> { (0..x.len()).map(|i| if i < 3 { NA } else { (x[i - 3] + 2. * x[i - 2] + 2. * x[i - 1] + x[i]) / 6. }).collect() }
fn rvgi(b: &Bars, prm: &Params) -> Output {
    let k = n(prm, "length");
    let num = swma(&(0..b.len()).map(|i| b.close[i] - b.open[i]).collect::<Vec<_>>());
    let den = swma(&(0..b.len()).map(|i| b.high[i] - b.low[i]).collect::<Vec<_>>());
    let r = zip(&ta::sum(&num, k), &ta::sum(&den, k), div);
    let sig = swma(&r);
    Ok(vec![r, sig])
}
fn rvi(b: &Bars, prm: &Params) -> Output {
    let (sd, ch, e) = (ta::stdev(&b.close, n(prm, "length")), ta::change(&b.close, 1), n(prm, "ema"));
    let u: Vec<f64> = (0..b.len()).map(|i| if ch[i] <= 0. { 0. } else { sd[i] }).collect(); // NaN change fails `<= 0` → sd, as in Pine
    let d: Vec<f64> = (0..b.len()).map(|i| if ch[i] > 0. { 0. } else { sd[i] }).collect();
    Ok(vec![zip(&ta::ema(&u, e), &ta::ema(&d, e), |a, c| div(100. * a, a + c))])
}
```

Careful with the SWMA NaN rule: `swma` here is bar-counted (any NaN in the 4 bars → NaN), which the test values assume.

`sources.rs` rows:

```rust
    ("pmo", "Price Momentum Oscillator (PMO)", ""),
    ("special_k", "Pring's Special K", ""),
    ("smi_ergodic", "SMI Ergodic Indicator; SMI Ergodic Oscillator", ""),
    ("smi", "Stochastic Momentum Index (SMI)", ""),
    ("rvgi", "Relative Vigor Index", ""),
    ("rvi", "Relative Volatility Index", ""),
```

- [ ] **Step 4: Run** `cargo test lab::` → PASS (Special K is checked by the 1,000-bar warm-up test: first 724 and 922); guard clean.
- [ ] **Step 5: Commit** `git commit -m "Add lab oscillators II (PMO, Special K, SMI Ergodic, SMI, relative vigor, relative volatility)"`

### Task 9: Volatility II — `vol2.rs`

**Files:** Create `lab/vol2.rs`; modify `lab/mod.rs`, `catalogue.rs` (`all`), `sources.rs`, parity doc.

**Interfaces:** Produces ids `chaikin_vol`, `mass_index`, `ulcer`, `adr`, `volatility_zt`, `volatility_ohlc`, `bbtrend`.

Category "Volatility", placement Pane, needs NONE. `days` is `ParamSpec { max: 366., ..len("days", "Days per year", 252.) }`.

| id | name | params | outputs | first | definition |
|---|---|---|---|---|---|
| chaikin_vol | Chaikin Volatility | length 10, roc 10 | cv | [19] | "e = ema(high − low, length); 100·(e − e[roc])/e[roc]" |
| mass_index | Mass Index | ema 9, length 10 | mi | [25] | "e1 = ema(high − low, ema); e2 = ema(e1, ema); sum(e1/e2, length)" |
| ulcer | Ulcer Index | length 14 | ulcer | [26] | "dd = 100·(close − highest(close, n))/highest(close, n); √sma(dd², n)" |
| adr | Average Daily Range | length 14 | adr | [13] | "sma(high − low, n) (= mean high − mean low; declared default 14)" |
| volatility_zt | Volatility Zero Trend Close-to-Close | length 10, days 252 | vol | [10] | "100·√(days·sma(r², n)), r = ln(close/close[1]); non-positive prices → NaN" |
| volatility_ohlc | Volatility O-H-L-C | length 10, days 252 | vol | [9] | "Garman–Klass (declared): 100·√(days·sma(0.5·ln(h/l)² − (2·ln 2 − 1)·ln(c/o)², n)); a negative mean or non-positive price → NaN" |
| bbtrend | BBTrend | short 20, long 50, mult real 2 (0.1–10, 0.1) | bbtrend | [49] | "100·(\|lower_s − lower_l\| − \|upper_s − upper_l\|)/basis_s with Bollinger(close, short/long, mult) (population stdev)" |

- [ ] **Step 1: Write the failing tests**

```rust
#[cfg(test)]
mod tests {
    use crate::lab::testdata::{assert_series, compiles, d1, go};
    const N: f64 = f64::NAN;
    fn one(id: &str, kv: &[(&str, f64)]) -> Vec<f64> { go(id, &d1(), kv).remove(0) }
    #[test] fn range_based_volatility() {
        assert_series(&one("chaikin_vol", &[("length", 2.), ("roc", 2.)]), &[N, N, N, -25.555555555556, -10.222222222222, 0.663349917081, 0.2200220022, 0.073219842577]);
        assert_series(&one("mass_index", &[("ema", 2.), ("length", 2.)]), &[N, N, N, 1.837497375604, 1.905038215103, 1.969272795197, 1.990359400171, 1.997017668111]);
        assert_series(&one("adr", &[("length", 3.)]), &[N, N, 1.833333333333, 1.633333333333, 1.466666666667, 1.466666666667, 1.5, 1.5]);
    }
    #[test] fn drawdown_and_return_volatility() {
        assert_series(&one("ulcer", &[("length", 3.)]), &[N, N, N, N, 9.906974722293, 9.962433263154, 8.723641921506, 1.049727762163]);
        assert_series(&one("volatility_zt", &[("length", 3.), ("days", 252.)]), &[N, N, N, 121.790223521499, 112.562429044232, 117.869172248916, 124.240969541656, 115.719494008877]);
        assert_series(&one("volatility_ohlc", &[("length", 3.), ("days", 252.)]), &[N, N, 183.553605141884, 146.064378916513, 128.112965729936, 134.551337938428, 136.956224350665, 132.876028180043]);
    }
    #[test] fn bbtrend_short_long() {
        assert_series(&one("bbtrend", &[("short", 2.), ("long", 3.), ("mult", 2.)]), &[N, N, 7.09219858156, 0.0, -8.805031446541, -1.521319399601, 5.425799802576, 7.891156462585]);
    }
    #[test] fn rules_reach_vol2() { for (id, out) in [("mass_index", "mi"), ("bbtrend", "bbtrend"), ("volatility_ohlc", "vol")] { compiles(id, out).unwrap(); } }
}
```

- [ ] **Step 2: Run** `CARGO_BUILD_JOBS=2 cargo test lab::vol2` → FAIL.

- [ ] **Step 3: Implement** (convention header, `CAT = "Volatility"`):

```rust
fn range(b: &Bars) -> Vec<f64> { (0..b.len()).map(|i| b.high[i] - b.low[i]).collect() }
fn chaikin_vol(b: &Bars, prm: &Params) -> Output {
    let (e, r) = (ta::ema(&range(b), n(prm, "length")), n(prm, "roc"));
    Ok(vec![(0..b.len()).map(|i| if i < r { NA } else { div(100. * (e[i] - e[i - r]), e[i - r]) }).collect()])
}
fn mass_index(b: &Bars, prm: &Params) -> Output {
    let k = n(prm, "ema"); let e1 = ta::ema(&range(b), k); let e2 = ta::ema(&e1, k);
    Ok(vec![ta::sum(&(0..b.len()).map(|i| div(e1[i], e2[i])).collect::<Vec<_>>(), n(prm, "length"))])
}
fn ulcer(b: &Bars, prm: &Params) -> Output {
    let k = n(prm, "length"); let hi = ta::highest(&b.close, k);
    let dd2: Vec<f64> = (0..b.len()).map(|i| div(100. * (b.close[i] - hi[i]), hi[i]).powi(2)).collect();
    Ok(vec![ta::sma(&dd2, k).iter().map(|v| v.sqrt()).collect()])
}
fn adr(b: &Bars, prm: &Params) -> Output { Ok(vec![ta::sma(&range(b), n(prm, "length"))]) }
fn annualised(x: &[f64], k: usize, days: f64) -> Vec<f64> { ta::sma(x, k).iter().map(|v| if *v < 0. { NA } else { 100. * (v * days).sqrt() }).collect() }
fn volatility_zt(b: &Bars, prm: &Params) -> Output {
    let c = &b.close;
    let r2: Vec<f64> = (0..c.len()).map(|i| if i > 0 && c[i] > 0. && c[i - 1] > 0. { (c[i] / c[i - 1]).ln().powi(2) } else { NA }).collect();
    Ok(vec![annualised(&r2, n(prm, "length"), p(prm, "days"))])
}
fn volatility_ohlc(b: &Bars, prm: &Params) -> Output {
    let gk: Vec<f64> = (0..b.len()).map(|i| {
        let (o, h, l, c) = (b.open[i], b.high[i], b.low[i], b.close[i]);
        if o <= 0. || h <= 0. || l <= 0. || c <= 0. { NA } else { 0.5 * (h / l).ln().powi(2) - (2. * 2f64.ln() - 1.) * (c / o).ln().powi(2) }
    }).collect();
    Ok(vec![annualised(&gk, n(prm, "length"), p(prm, "days"))])
}
fn bbtrend(b: &Bars, prm: &Params) -> Output {
    let m = p(prm, "mult");
    let bands = |k: usize| { let (ba, sd) = (ta::sma(&b.close, k), ta::stdev(&b.close, k)); (ba.clone(), (0..b.len()).map(|i| ba[i] + m * sd[i]).collect::<Vec<_>>(), (0..b.len()).map(|i| ba[i] - m * sd[i]).collect::<Vec<_>>()) };
    let ((sb, su, sl), (_, lu, ll)) = (bands(n(prm, "short")), bands(n(prm, "long")));
    Ok(vec![(0..b.len()).map(|i| div(100. * ((sl[i] - ll[i]).abs() - (su[i] - lu[i]).abs()), sb[i])).collect()])
}
```

`sources.rs` rows:

```rust
    ("chaikin_vol", "Chaikin Volatility", ""),
    ("mass_index", "Mass Index", ""),
    ("ulcer", "Ulcer Index", ""),
    ("adr", "Average Daily Range (ADR)", ""),
    ("volatility_zt", "Volatility Zero Trend Close-to-Close", ""),
    ("volatility_ohlc", "Volatility O-H-L-C", ""),
    ("bbtrend", "BBTrend", ""),
```

- [ ] **Step 4: Register and run** → PASS; guard clean.
- [ ] **Step 5: Commit** `git commit -m "Add lab volatility II (Chaikin volatility, mass index, ulcer, ADR, zero-trend and OHLC volatility, BBTrend)"`

### Task 10: Stops and trend state — `stops.rs`

**Files:** Create `lab/stops.rs`; modify `lab/mod.rs`, `catalogue.rs` (`all`), `sources.rs`, parity doc.

**Interfaces:** Produces ids `chande_kroll`, `chandelier`, `vstop`, `wilder_vi`, `chop_zone`, `rwi`, `asi`.

Category "Trend", needs NONE.

| id | name | placement | params | outputs | first | definition |
|---|---|---|---|---|---|---|
| chande_kroll | Chande Kroll Stop | Price | p 10, x real 1 (0.1–10, 0.1), q 9 | stop_long, stop_short | [9,9] | "fhs = highest(high, p) − x·atr(p); fls = lowest(low, p) + x·atr(p); stop_short = highest(fhs, q); stop_long = lowest(fls, q) (window restarts after NaN, as ta.highest)" |
| chandelier | Chandelier Exit | Price | length 22, mult real 3 (0.1–20, 0.1), use_close int 1 (0–1) | long_stop, short_stop, direction | [21,21,21] | "long = highest(close or high, n) − mult·atr(n), ratcheted up while close[1] > long[1]; short = lowest(close or low, n) + mult·atr(n), ratcheted down while close[1] < short[1]; direction +1 when close > short[1], −1 when close < long[1], else unchanged (starts +1)" |
| vstop | Volatility Stop | Price | length 20, factor real 2 (0.1–20, 0.1) | stop, direction | [0,0] | "TradingView pine_vstop on close: atrM = atr(n)·factor (tr before the ATR exists); trailing max/min of close since the last flip; stop = max(stop, max − atrM) in an uptrend, min(stop, min + atrM) in a downtrend; flip when close crosses the stop" |
| wilder_vi | Volatility Index (Wilder) | Price | length 7, mult real 3 (0.1–20, 0.1) | sar, direction | [6,6] | "Wilder Volatility System (declared): ARC = mult·atr(length); SIC = extreme close since the trend began; sar = SIC − ARC (long) / SIC + ARC (short); reverse when close crosses the previous sar; starts long" |
| chop_zone | Chop Zone | Pane | length 30, ema_length 34 | angle, zone | [34,34] | "span = 25/(highest(high, length) − lowest(low, length))·lowest(low, length); y = (ema[1] − ema)/hlc3·span; angle = ±round(degrees(acos(1/√(1 + y²)))), negative when y > 0; zone 4…−4 by thresholds 5, 3.57, 2.14, 0.71 (else 0)" |
| rwi | Random Walk Index | Pane | min_length 9, max_length 40 | hi, lo, rwi | [40,40,40] | "for n in min..=max: (high − low[n])/(sma(tr, n)·√n) and (high[n] − low)/(sma(tr, n)·√n); hi/lo = max over n (NaN terms skipped); rwi = hi − lo; min_length > max_length → invalid (AmiBroker RWIHi/RWILo/RWI)" |
| asi | Accumulative Swing Index | Pane | limit real 10 (0.01–1000000, 0.01) | asi | [1] | "Wilder: K = max(\|h − c1\|, \|l − c1\|); R from the largest of \|h − c1\|, \|l − c1\|, h − l (with 0.25·\|c1 − o1\|); SI = 50·(c − c1 + 0.5(c − o) + 0.25(c1 − o1))/R·K/limit (R = 0 → 0); asi = cumulative SI from bar 1" |

- [ ] **Step 1: Write the failing tests**

```rust
#[cfg(test)]
mod tests {
    use crate::lab::catalogue::{get, run, Params};
    use crate::lab::testdata::{assert_series, compiles, d1, go};
    const N: f64 = f64::NAN;
    #[test] fn chande_kroll_short_lengths() {
        let o = go("chande_kroll", &d1(), &[("p", 2.), ("x", 1.), ("q", 2.)]);
        assert_series(&o[0], &[N, 11.0, 11.0, 11.75, 11.5375, 11.01875, 11.009375, 11.009375]);
        assert_series(&o[1], &[N, 10.0, 10.75, 10.925, 10.925, 10.6625, 10.490625, 11.4953125]);
    }
    #[test] fn chandelier_ratchets_and_flips() {
        let o = go("chandelier", &d1(), &[("length", 2.), ("mult", 1.), ("use_close", 1.)]);
        assert_series(&o[0], &[N, 9.5, 10.25, 10.425, 10.425, 9.28125, 10.390625, 11.0953125]);
        assert_series(&o[1], &[N, 12.5, 12.5, 12.5, 11.7375, 11.71875, 11.71875, 13.4046875]);
        assert_series(&o[2], &[N, 1., 1., 1., -1., -1., 1., 1.]);
    }
    #[test] fn volatility_stop_from_bar_zero() {
        let o = go("vstop", &d1(), &[("length", 2.), ("factor", 1.)]);
        assert_series(&o[0], &[10.5, 10.5, 10.5, 10.5, 11.7375, 11.71875, 10.390625, 11.0953125]);
        assert_series(&o[1], &[1., 1., 1., 1., -1., -1., 1., 1.]);
    }
    #[test] fn wilder_volatility_system() {
        let o = go("wilder_vi", &d1(), &[("length", 2.), ("mult", 1.)]);
        assert_series(&o[0], &[N, 9.5, 10.25, 10.425, 11.7375, 11.71875, 10.390625, 11.0953125]);
        assert_series(&o[1], &[N, 1., 1., 1., -1., -1., 1., 1.]);
    }
    #[test] fn chop_zone_angles() {
        let o = go("chop_zone", &d1(), &[("length", 2.), ("ema_length", 2.)]);
        assert_series(&o[0], &[N, N, 80., -81., -82., 63., 81., 81.]);
        assert_series(&o[1], &[N, N, 4., -4., -4., 4., 4., 4.]);
    }
    #[test] fn random_walk_index() {
        let o = go("rwi", &d1(), &[("min_length", 2.), ("max_length", 3.)]);
        assert_series(&o[0], &[N, N, N, 1.131135221269, 0.590471866217, 0.094280904158, 0.942809041582, 1.649915822769]);
        assert_series(&o[1], &[N, N, N, 0.585191818913, 1.219149622735, 1.272792206136, 0.654330305082, 0.0]);
        assert_series(&o[2], &[N, N, N, 0.545943402356, -0.628677756519, -1.178511301978, 0.2884787365, 1.649915822769]);
    }
    #[test] fn rwi_rejects_min_above_max() {
        let p: Params = [("min_length".to_string(), 10.), ("max_length".to_string(), 5.)].into_iter().collect();
        assert!(run(&get("rwi").unwrap(), &d1(), &p).unwrap_err().contains("min_length"));
    }
    #[test] fn accumulative_swing_index() {
        assert_series(&go("asi", &d1(), &[("limit", 10.)])[0], &[N, 5.735294117647, 8.59243697479, 3.182600909216, -0.960256233641, 0.686802589888, 7.232257135343, 11.33789093816]);
    }
    #[test] fn rules_reach_stops() { for (id, out) in [("chandelier", "direction"), ("vstop", "stop"), ("chop_zone", "zone"), ("rwi", "rwi")] { compiles(id, out).unwrap(); } }
}
```

- [ ] **Step 2: Run** `CARGO_BUILD_JOBS=2 cargo test lab::stops` → FAIL.

- [ ] **Step 3: Implement** (convention header, `CAT = "Trend"`):

```rust
fn atr(b: &Bars, k: usize) -> Vec<f64> { ta::rma(&ta::tr(b, true), k) }
fn chande_kroll(b: &Bars, prm: &Params) -> Output {
    let (pp, x, q) = (n(prm, "p"), p(prm, "x"), n(prm, "q"));
    let (a, hh, ll) = (atr(b, pp), ta::highest(&b.high, pp), ta::lowest(&b.low, pp));
    let fhs: Vec<f64> = (0..b.len()).map(|i| hh[i] - x * a[i]).collect();
    let fls: Vec<f64> = (0..b.len()).map(|i| ll[i] + x * a[i]).collect();
    Ok(vec![ta::lowest(&fls, q), ta::highest(&fhs, q)])
}
fn chandelier(b: &Bars, prm: &Params) -> Output {
    let (k, m, uc) = (n(prm, "length"), p(prm, "mult"), p(prm, "use_close") == 1.);
    let a: Vec<f64> = atr(b, k).iter().map(|v| m * v).collect();
    let (hi, lo) = if uc { (ta::highest(&b.close, k), ta::lowest(&b.close, k)) } else { (ta::highest(&b.high, k), ta::lowest(&b.low, k)) };
    let len = b.len();
    let (mut ls, mut ss, mut dir) = (vec![NA; len], vec![NA; len], vec![NA; len]);
    let mut d = 1.;
    for i in 0..len {
        let (l0, s0) = (hi[i] - a[i], lo[i] + a[i]);
        let lp = if i > 0 && !ls[i - 1].is_nan() { ls[i - 1] } else { l0 }; // nz(longStop[1], longStop)
        let sp = if i > 0 && !ss[i - 1].is_nan() { ss[i - 1] } else { s0 };
        ls[i] = if i > 0 && b.close[i - 1] > lp { l0.max(lp) } else { l0 };
        ss[i] = if i > 0 && b.close[i - 1] < sp { s0.min(sp) } else { s0 };
        if b.close[i] > sp { d = 1.; } else if b.close[i] < lp { d = -1.; }
        if !ls[i].is_nan() { dir[i] = d; }
    }
    Ok(vec![ls, ss, dir])
}
fn vstop(b: &Bars, prm: &Params) -> Output {
    let (k, f) = (n(prm, "length"), p(prm, "factor"));
    let (a, tr) = (atr(b, k), ta::tr(b, false));
    let len = b.len();
    let (mut st, mut dir) = (vec![NA; len], vec![NA; len]);
    if len == 0 { return Ok(vec![st, dir]); }
    let (mut mx, mut mn, mut up, mut stop) = (b.close[0], b.close[0], true, NA);
    for i in 0..len {
        let src = b.close[i];
        let m = if a[i].is_nan() { tr[i] } else { a[i] * f }; // nz(atr·factor, tr)
        mx = mx.max(src); mn = mn.min(src);
        // Pine math.max/min return na when an argument is na; nz(…, src) then restarts the stop at src
        let cand = if stop.is_nan() || m.is_nan() { NA } else if up { stop.max(mx - m) } else { stop.min(mn + m) };
        stop = if cand.is_nan() { src } else { cand };
        let was = up;
        up = src - stop >= 0.;
        if up != was { mx = src; mn = src; stop = if up { mx - m } else { mn + m }; }
        st[i] = stop; dir[i] = if up { 1. } else { -1. };
    }
    Ok(vec![st, dir])
}
fn wilder_vi(b: &Bars, prm: &Params) -> Output {
    let (a, m) = (atr(b, n(prm, "length")), p(prm, "mult"));
    let len = b.len();
    let (mut s, mut dir) = (vec![NA; len], vec![NA; len]);
    let (mut started, mut long, mut sic, mut sar) = (false, true, NA, NA);
    for i in 0..len {
        if a[i].is_nan() { continue; }
        let c = b.close[i];
        if !started { started = true; long = true; sic = c; }
        else if long && c < sar { long = false; sic = c; }
        else if !long && c > sar { long = true; sic = c; }
        else { sic = if long { sic.max(c) } else { sic.min(c) }; }
        sar = if long { sic - m * a[i] } else { sic + m * a[i] };
        s[i] = sar; dir[i] = if long { 1. } else { -1. };
    }
    Ok(vec![s, dir])
}
fn chop_zone(b: &Bars, prm: &Params) -> Output {
    let k = n(prm, "length");
    let (avg, hh, ll, e) = (b.hlc3(), ta::highest(&b.high, k), ta::lowest(&b.low, k), ta::ema(&b.close, n(prm, "ema_length")));
    let len = b.len();
    let (mut ang, mut zone) = (vec![NA; len], vec![NA; len]);
    for i in 1..len {
        if [hh[i], ll[i], e[i], e[i - 1]].iter().any(|v| v.is_nan()) || hh[i] == ll[i] || avg[i] == 0. { continue; }
        let span = 25. / (hh[i] - ll[i]) * ll[i];
        let y = (e[i - 1] - e[i]) / avg[i] * span;
        let a = (180. * (1. / (1. + y * y).sqrt()).acos() / std::f64::consts::PI).round();
        let a = if y > 0. { -a } else { a };
        ang[i] = a;
        zone[i] = if a >= 5. { 4. } else if a >= 3.57 { 3. } else if a >= 2.14 { 2. } else if a >= 0.71 { 1. }
                  else if a <= -5. { -4. } else if a <= -3.57 { -3. } else if a <= -2.14 { -2. } else if a <= -0.71 { -1. } else { 0. };
    }
    Ok(vec![ang, zone])
}
fn rwi(b: &Bars, prm: &Params) -> Output {
    let (lo_n, hi_n) = (n(prm, "min_length"), n(prm, "max_length"));
    if lo_n > hi_n { return Err("min_length must not exceed max_length".into()); }
    let t = ta::tr(b, true); let len = b.len();
    let atrs: Vec<Vec<f64>> = (lo_n..=hi_n).map(|k| ta::sma(&t, k)).collect();
    let (mut h, mut l, mut r) = (vec![NA; len], vec![NA; len], vec![NA; len]);
    for i in hi_n..len {
        let (mut bh, mut bl) = (NA, NA);
        for (j, k) in (lo_n..=hi_n).enumerate() {
            let d = atrs[j][i] * (k as f64).sqrt();
            let (x, y) = (div(b.high[i] - b.low[i - k], d), div(b.high[i - k] - b.low[i], d));
            if !x.is_nan() { bh = if bh.is_nan() { x } else { bh.max(x) }; }
            if !y.is_nan() { bl = if bl.is_nan() { y } else { bl.max(y) }; }
        }
        h[i] = bh; l[i] = bl; r[i] = bh - bl;
    }
    Ok(vec![h, l, r])
}
fn asi(b: &Bars, prm: &Params) -> Output {
    let t = p(prm, "limit");
    let (o, h, l, c) = (&b.open, &b.high, &b.low, &b.close);
    let mut s = 0.;
    let mut out = vec![NA; b.len()];
    for i in 1..b.len() {
        let (a1, a2, a3, d) = ((h[i] - c[i - 1]).abs(), (l[i] - c[i - 1]).abs(), h[i] - l[i], (c[i - 1] - o[i - 1]).abs());
        let k = a1.max(a2);
        let r = if a1 >= a2 && a1 >= a3 { a1 - 0.5 * a2 + 0.25 * d } else if a2 >= a3 { a2 - 0.5 * a1 + 0.25 * d } else { a3 + 0.25 * d };
        s += if r == 0. { 0. } else { 50. * (c[i] - c[i - 1] + 0.5 * (c[i] - o[i]) + 0.25 * (c[i - 1] - o[i - 1])) / r * k / t };
        out[i] = s;
    }
    Ok(vec![out])
}
```

`sources.rs` rows:

```rust
    ("chande_kroll", "Chande Kroll Stop", ""),
    ("chandelier", "Chandelier Exit", ""),
    ("vstop", "Volatility Stop", ""),
    ("wilder_vi", "Volatility Index", ""),
    ("chop_zone", "Chop Zone", ""),
    ("rwi", "", "RWI; RWIHi; RWILo"),
    ("asi", "Accumulative Swing Index", ""),
```

- [ ] **Step 4: Register and run** → PASS (generic tests run `rwi` at all-min/all-max bounds, both equal, so no error); guard clean.
- [ ] **Step 5: Commit** `git commit -m "Add lab stops and trend state (Chande Kroll, Chandelier, volatility stop, Wilder volatility system, chop zone, RWI, ASI)"`

### Task 11: Volume II and period anchors — `volume2.rs`, `resample::period_key`, parity dates

**Files:** Create `lab/volume2.rs`; modify `lab/resample.rs` (`period_key`), `lab/parity.rs` (dates from the time column), `lab/mod.rs`, `catalogue.rs` (`all`), `sources.rs`, parity doc.

**Interfaces:** Produces `pub fn resample::period_key(date: &str, anchor: usize) -> i64` (1 week Monday–Sunday, 2 month, 3 quarter, 4 year; Task 12 consumes it); ids `klinger`, `vol`, `vwap_period`, `twap`. The parity harness now gives bars their real dates when the CSV time column is ISO (`YYYY-MM-DD…`) or unix seconds; any other value falls back to `civil(row index)`, which still parses.

Category "Volume", placement as listed. `VOL = Needs { volume: true, ..Needs::NONE }`. `anchor` is `int("anchor", "Anchor (1 week, 2 month, 3 quarter, 4 year)", 1., 1., 4.)`.

| id | name | placement | params | outputs | first | needs | definition |
|---|---|---|---|---|---|---|---|
| klinger | Klinger Oscillator | Pane | fast 34, slow 55, signal 13 | kvo, signal | [54,66] | VOL | "sv = change(hlc3) ≥ 0 ? volume : −volume (bar 0: NaN change → −volume, TradingView quirk); kvo = ema(sv, fast) − ema(sv, slow); signal = ema(kvo, signal)" |
| vol | Volume | Pane | length 20 | volume, ma | [0,19] | VOL | "volume and sma(volume, length)" |
| vwap_period | VWAP (anchored period) | Price | anchor 1, mult real 1 (0.1–10, 0.1) | vwap, upper, lower | [0,0,0] | VOL | "from the first bar of each anchor period: vwap = Σ(hlc3·v)/Σv; sd = √max(Σ(v·hlc3²)/Σv − vwap², 0); upper/lower = vwap ± mult·sd; Σv = 0 → NaN (session VWAP stays listed: it needs intraday bars)" |
| twap | Time Weighted Average Price | Price | anchor 1 | twap | [0] | NONE | "mean of ohlc4 over the bars since the anchor period began" |

- [ ] **Step 1: Write the failing tests**

`resample.rs` tests:

```rust
    #[test] fn period_keys_split_weeks_months_quarters_years() {
        assert_eq!(period_key("2024-01-07", 1), period_key("2024-01-01", 1)); // Sunday closes the Monday week
        assert_ne!(period_key("2024-01-07", 1), period_key("2024-01-08", 1));
        assert_ne!(period_key("2024-01-31", 2), period_key("2024-02-01", 2));
        assert_eq!(period_key("2024-01-15", 3), period_key("2024-03-31", 3));
        assert_ne!(period_key("2024-03-31", 3), period_key("2024-04-01", 3));
        assert_ne!(period_key("2023-12-31", 4), period_key("2024-01-01", 4));
    }
```

`volume2.rs` tests:

```rust
#[cfg(test)]
mod tests {
    use crate::lab::testdata::{assert_series, compiles, d1, go};
    const N: f64 = f64::NAN;
    #[test] fn klinger_short() {
        let o = go("klinger", &d1(), &[("fast", 2.), ("slow", 3.), ("signal", 2.)]);
        assert_series(&o[0], &[N, N, 26.666666666667, -22.222222222222, -19.62962962963, -9.320987654321, 40.504115226337, 31.973593964335]);
        assert_series(&o[1], &[N, N, N, 2.222222222222, -12.345679012346, -10.329218106996, 23.559670781893, 29.168952903521]);
    }
    #[test] fn volume_and_its_average() {
        let o = go("vol", &d1(), &[("length", 2.)]);
        assert_series(&o[0], &[100., 120., 90., 150., 130., 110., 160., 140.]);
        assert_series(&o[1], &[N, 110., 105., 120., 140., 120., 135., 150.]);
    }
    #[test] fn weekly_anchored_vwap_restarts_on_monday() {
        // d1: 2024-01-01 is a Monday; index 7 (2024-01-08) starts the next week
        let o = go("vwap_period", &d1(), &[("anchor", 1.), ("mult", 1.)]);
        assert_series(&o[0], &[10.166666666667, 10.712121212121, 11.037634408602, 11.134057971014, 11.009039548023, 10.918571428571, 11.020542635659, 12.366666666667]);
        assert_series(&o[1], &[10.166666666667, 11.210050809853, 11.697152095707, 11.692934048575, 11.555689351692, 11.462415352094, 11.555548941127, 12.366666666667]);
        assert_series(&o[2], &[10.166666666667, 10.214191614389, 10.378116721497, 10.575181893454, 10.462389744353, 10.374727505049, 10.485536330191, 12.366666666667]);
        assert_series(&go("twap", &d1(), &[("anchor", 1.)])[0], &[10.125, 10.5625, 10.958333333333, 11.09375, 11.01, 10.904166666667, 10.960714285714, 12.25]);
    }
    #[test] fn rules_reach_volume2() { for (id, out) in [("klinger", "signal"), ("vwap_period", "lower"), ("twap", "twap"), ("vol", "ma")] { compiles(id, out).unwrap(); } }
}
```

- [ ] **Step 2: Run** `CARGO_BUILD_JOBS=2 cargo test lab::volume2 lab::resample` → FAIL.

- [ ] **Step 3: Implement**

`resample.rs`:

```rust
/// Calendar period of a "YYYY-MM-DD" date: 1 = week (Monday–Sunday), 2 = month, 3 = quarter, 4 = year.
pub fn period_key(date: &str, anchor: usize) -> i64 {
    let num = |r: std::ops::Range<usize>| date.get(r).and_then(|s| s.parse::<i64>().ok()).unwrap_or(0);
    let (y, m) = (num(0..4), num(5..7));
    match anchor { 1 => key(date, Tf::W), 2 => y * 12 + m, 3 => y * 4 + (m - 1).div_euclid(3), _ => y }
}
```

`volume2.rs` (convention header; `CAT = "Volume"`; `use super::resample::period_key;`):

```rust
fn vol(b: &Bars) -> &[f64] { b.volume.as_deref().unwrap_or(&[]) }
fn klinger(b: &Bars, prm: &Params) -> Output {
    let (v, ch) = (vol(b), ta::change(&b.hlc3(), 1));
    let sv: Vec<f64> = (0..b.len()).map(|i| if ch[i] >= 0. { v[i] } else { -v[i] }).collect();
    let k: Vec<f64> = ta::ema(&sv, n(prm, "fast")).iter().zip(ta::ema(&sv, n(prm, "slow"))).map(|(f, s)| f - s).collect();
    let sig = ta::ema(&k, n(prm, "signal"));
    Ok(vec![k, sig])
}
fn volume(b: &Bars, prm: &Params) -> Output { Ok(vec![vol(b).to_vec(), ta::sma(vol(b), n(prm, "length"))]) }
fn vwap_period(b: &Bars, prm: &Params) -> Output {
    let (a, m, v, src) = (n(prm, "anchor"), p(prm, "mult"), vol(b), b.hlc3());
    let len = b.len();
    let (mut w, mut u, mut l) = (vec![NA; len], vec![NA; len], vec![NA; len]);
    let (mut last, mut sv, mut spv, mut spv2) = (None, 0., 0., 0.);
    for i in 0..len {
        let k = period_key(&b.date[i], a);
        if last != Some(k) { sv = 0.; spv = 0.; spv2 = 0.; last = Some(k); }
        sv += v[i]; spv += src[i] * v[i]; spv2 += src[i] * src[i] * v[i];
        if sv == 0. { continue; }
        let x = spv / sv; let sd = (spv2 / sv - x * x).max(0.).sqrt();
        w[i] = x; u[i] = x + m * sd; l[i] = x - m * sd;
    }
    Ok(vec![w, u, l])
}
fn twap(b: &Bars, prm: &Params) -> Output {
    let (a, src) = (n(prm, "anchor"), b.ohlc4());
    let (mut last, mut s, mut c) = (None, 0., 0.);
    Ok(vec![(0..b.len()).map(|i| {
        let k = period_key(&b.date[i], a);
        if last != Some(k) { s = 0.; c = 0.; last = Some(k); }
        s += src[i]; c += 1.; s / c
    }).collect()])
}
```
(The `vol` entry's compute function is named `volume` to avoid clashing with the `vol` helper.)

`parity.rs`: when building `Bars`, replace `b.date.push(format!("{i}"))` with `b.date.push(date_of(time_column, i))`, where the time column is the first CSV field of the row:

```rust
/// Bar date from the export's time column: ISO date, or unix seconds (UTC day); otherwise a stand-in date.
fn date_of(t: &str, i: usize) -> String {
    let t = t.trim();
    if t.len() >= 10 && t.as_bytes()[4] == b'-' && t.as_bytes()[7] == b'-' { return t[..10].to_string(); }
    match t.parse::<i64>() { Ok(s) => super::resample::civil(s.div_euclid(86_400)), Err(_) => super::resample::civil(i as i64) }
}
```
(Keep the rows' first field: change the row parser to keep `l.split(',').next()` alongside the numeric columns.) Add `#[test] fn parity_dates_come_from_the_time_column() { assert_eq!(date_of("2024-01-05", 0), "2024-01-05"); assert_eq!(date_of("1704412800", 0), "2024-01-05"); assert_eq!(date_of("x", 3), "1970-01-04"); }` (2024-01-05T00:00Z = 1704412800).

`sources.rs` rows:

```rust
    ("klinger", "Klinger Oscillator", ""),
    ("vol", "Volume", ""),
    ("vwap_period", "VWAP (anchor Week/Month/Quarter/Year)", ""),
    ("twap", "Time Weighted Average Price", ""),
```

- [ ] **Step 4: Register and run** `cargo test lab::` → PASS; guard clean.
- [ ] **Step 5: Commit** `git commit -m "Add lab Klinger, volume, anchored VWAP/TWAP and dated parity fixtures"`

### Task 12: Periods and calendar — `periods.rs`

**Files:** Create `lab/periods.rs`; modify `lab/mod.rs`, `catalogue.rs` (`all`), `sources.rs`, parity doc.

**Interfaces:** Consumes `resample::{period_key, day_number}` (Task 11). Produces ids `pivots_std`, `mtp`, `performance`, `calendar`, `moon_phases`, `highest_bars`, `lowest_bars`.

Category "Periods & calendar", needs NONE. `period` is `int("period", "Period (1 week, 2 month, 3 quarter, 4 year)", 1., 1., 4.)`; `type` is `int("type", "Type (0 Traditional, 1 Fibonacci, 2 Woodie, 3 Classic, 4 DM, 5 Camarilla)", 0., 0., 5.)`.

| id | name | placement | params | outputs | first | definition |
|---|---|---|---|---|---|---|
| pivots_std | Pivot Points Standard | Price | type 0, period 1 | p, r1, r2, r3, r4, r5, s1, s2, s3, s4, s5 | [5 × 11] | "levels from the previous completed period's H, L, C (DM: and its open; Woodie: the current period's first open): Traditional P=(H+L+C)/3, R1=2P−L, R2=P+(H−L), R3=H+2(P−L), R4=3P+(H−3L), R5=4P+(H−4L), S mirror; Fibonacci P±0.382/0.618/1.0·(H−L); Woodie P=(H+L+2·O)/4, R3=H+2(P−L), R4=R3+(H−L); Classic P±1/2/3·(H−L) (R1/S1 as Traditional); DM X = C<O ? H+2L+C : C>O ? 2H+L+C : H+L+2C, P=X/4, R1=X/2−L, S1=X/2−H; Camarilla C±1.1(H−L)/12, /6, /4, /2, R5=(H/L)·C, S5=C−(R5−C); undefined levels NaN" |
| mtp | Multi-Time Period Charts | Price | period 1 | open, high, low, prev_high, prev_low, prev_close | [0,0,0,5,5,5] | "current period's first open and high/low so far; previous completed period's high, low, close" |
| performance | Performance | Pane | — | w1, m1, m3, m6, ytd, y1 | [5,21,63,126,366,252] | "roc(close, 5/21/63/126/252 bars) (declared bar horizons); ytd = 100·(close/last close of the previous calendar year − 1)" |
| calendar | Calendar | Pane | — | dow, day, month, year, doy, bar_index | [0,0,0,0,0,0] | "AmiBroker DayOfWeek (0 Sunday … 6 Saturday), Day, Month, Year, DayOfYear (1–366), BarIndex (0-based) of the bar's date" |
| moon_phases | Moon Phases | Price | — | new_moon, full_moon | [0,0] | "1 on the first bar dated on or after a new (full) moon, UTC, Meeus ch. 49 with 15 periodic terms; else 0 (declared)" |
| highest_bars | Bars Since Highest High | Pane | length 20 | bars | [19] | "bars since the highest high of the last length bars (0 = this bar; ties → the most recent) (AmiBroker HHVBars, TradingView ta.highestbars as a positive count)" |
| lowest_bars | Bars Since Lowest Low | Pane | length 20 | bars | [19] | "bars since the lowest low of the last length bars (0 = this bar; ties → the most recent)" |

(`first` values for `pivots_std`/`mtp`/`performance` are for `walk` bars, which start on Wednesday 2020-01-01: the second week starts at index 5 and the first bar of 2021 is index 366.)

- [ ] **Step 1: Write the failing tests**

```rust
#[cfg(test)]
mod tests {
    use crate::lab::bars::Bars;
    use crate::lab::testdata::{assert_series, close_eq, closes, compiles, d1, from_ohlc, go};
    const N: f64 = f64::NAN;
    fn dated(rows: &[(f64, f64, f64, f64)], dates: &[&str]) -> Bars { let mut b = from_ohlc(rows); b.date = dates.iter().map(|d| d.to_string()).collect(); b }
    #[test] fn pivot_types_from_the_previous_week() {
        // week 1 = d1 bars 0..6 (H 12.5, L 9, C 11.9, first open 10); bar 7 opens week 2 at 11.9
        let want: [[f64; 11]; 6] = [
            [11.133333333333, 13.266666666667, 14.633333333333, 16.766666666667, 18.9, 21.033333333333, 9.766666666667, 7.633333333333, 6.266666666667, 4.9, 3.533333333333],
            [11.133333333333, 12.470333333333, 13.296333333333, 14.633333333333, N, N, 9.796333333333, 8.970333333333, 7.633333333333, N, N],
            [11.325, 13.65, 14.825, 17.15, 20.65, N, 10.15, 7.825, 6.65, 3.15, N],
            [11.133333333333, 13.266666666667, 14.633333333333, 18.133333333333, 21.633333333333, N, 9.766666666667, 7.633333333333, 4.133333333333, 0.633333333333, N],
            [11.475, 13.95, N, N, N, N, 10.45, N, N, N, N],
            [11.133333333333, 12.220833333333, 12.541666666667, 12.8625, 13.825, 16.527777777778, 11.579166666667, 11.258333333333, 10.9375, 9.975, 7.272222222222],
        ];
        for (t, w) in want.iter().enumerate() {
            let o = go("pivots_std", &d1(), &[("type", t as f64), ("period", 1.)]);
            for j in 0..11 { assert!(o[j][..7].iter().all(|v| v.is_nan()), "type {t} level {j}"); assert!(close_eq(o[j][7], w[j]), "type {t} level {j}: {}", o[j][7]); }
        }
    }
    #[test] fn pivots_use_the_previous_completed_week_across_a_holiday_week() {
        // week A Tue–Fri plus a Sunday bar (Phase 5 CFDs); week B starts Tuesday after a Monday holiday
        let b = dated(&[(10., 12., 9., 11.), (11., 13., 10., 12.), (12., 12.5, 11., 11.5), (11.5, 12., 10.5, 11.), (11., 11.4, 10.8, 11.1), (11., 11.5, 10.5, 11.2), (11.2, 12., 11., 11.8)],
                      &["2024-01-02", "2024-01-03", "2024-01-04", "2024-01-05", "2024-01-07", "2024-01-09", "2024-01-10"]);
        let o = go("pivots_std", &b, &[("type", 0.), ("period", 1.)]);
        assert_series(&o[0], &[N, N, N, N, N, 11.033333333333, 11.033333333333]);
        assert_series(&o[1], &[N, N, N, N, N, 13.066666666667, 13.066666666667]);
        assert_series(&o[6], &[N, N, N, N, N, 9.066666666667, 9.066666666667]);
    }
    #[test] fn multi_time_period_week() {
        let o = go("mtp", &d1(), &[("period", 1.)]);
        assert_series(&o[0], &[10., 10., 10., 10., 10., 10., 10., 11.9]);
        assert_series(&o[1], &[11., 12., 12.5, 12.5, 12.5, 12.5, 12.5, 13.]);
        assert_series(&o[2], &[9., 9., 9., 9., 9., 9., 9., 11.5]);
        assert_series(&o[3], &[N, N, N, N, N, N, N, 12.5]);
        assert_series(&o[5], &[N, N, N, N, N, N, N, 11.9]);
    }
    #[test] fn performance_horizons_and_ytd() {
        let o = go("performance", &closes(&[1., 2., 3., 4., 5., 6., 7., 8.]), &[]);
        assert_series(&o[0], &[N, N, N, N, N, 500., 250., 166.666666666667]);
        let y = dated(&[(10., 10., 10., 10.), (11., 11., 11., 11.), (12., 12., 12., 12.), (9.9, 9.9, 9.9, 9.9)], &["2023-12-28", "2023-12-29", "2024-01-02", "2024-01-03"]);
        assert_series(&go("performance", &y, &[])[4], &[N, N, 9.090909090909, -10.]);
    }
    #[test] fn calendar_fields() {
        let o = go("calendar", &d1(), &[]);
        assert_eq!((o[0][0], o[1][0], o[2][0], o[3][0], o[4][0], o[5][0]), (1., 1., 1., 2024., 1., 0.)); // Monday 2024-01-01
        assert_eq!((o[0][7], o[1][7], o[4][7], o[5][7]), (1., 8., 8., 7.));
        let leap = go("calendar", &dated(&[(1., 1., 1., 1.)], &["2024-12-31"]), &[]);
        assert_eq!((leap[0][0], leap[4][0]), (2., 366.)); // Tuesday, day 366
    }
    #[test] fn moon_phases_january_2024() {
        let dates: Vec<String> = (1..=31).map(|d| format!("2024-01-{d:02}")).collect();
        let b = dated(&vec![(1., 1., 1., 1.); 31], &dates.iter().map(|s| s.as_str()).collect::<Vec<_>>());
        let o = go("moon_phases", &b, &[]);
        let days = |v: &[f64]| v.iter().enumerate().filter(|(_, x)| **x == 1.).map(|(i, _)| i + 1).collect::<Vec<_>>();
        assert_eq!(days(&o[0]), [11]); // new moon 2024-01-11 11:57 UTC
        assert_eq!(days(&o[1]), [25]); // full moon 2024-01-25 17:54 UTC
        let gap = go("moon_phases", &dated(&vec![(1., 1., 1., 1.); 4], &["2024-01-09", "2024-01-10", "2024-01-12", "2024-01-15"]), &[]);
        assert_series(&gap[0], &[0., 0., 1., 0.]); // no bar on the 11th: flagged on the next bar
    }
    #[test] fn bars_since_extremes() {
        assert_series(&go("highest_bars", &d1(), &[("length", 3.)])[0], &[N, N, 0., 1., 2., 2., 0., 0.]);
        assert_series(&go("lowest_bars", &d1(), &[("length", 3.)])[0], &[N, N, 2., 2., 0., 0., 1., 2.]);
    }
    #[test] fn rules_reach_periods() { for (id, out) in [("pivots_std", "s5"), ("mtp", "prev_close"), ("calendar", "dow"), ("moon_phases", "full_moon"), ("performance", "ytd")] { compiles(id, out).unwrap(); } }
}
```

- [ ] **Step 2: Run** `CARGO_BUILD_JOBS=2 cargo test lab::periods` → FAIL.

- [ ] **Step 3: Implement** (convention header; `CAT = "Periods & calendar"`; `use super::resample::{day_number, period_key};`):

```rust
fn levels(t: usize, h: f64, l: f64, c: f64, o_prev: f64, o_cur: f64) -> [f64; 11] {
    let (p, r) = ((h + l + c) / 3., h - l);
    match t {
        0 => [p, 2. * p - l, p + r, h + 2. * (p - l), 3. * p + (h - 3. * l), 4. * p + (h - 4. * l), 2. * p - h, p - r, l - 2. * (h - p), 3. * p - (3. * h - l), 4. * p - (4. * h - l)],
        1 => [p, p + 0.382 * r, p + 0.618 * r, p + r, NA, NA, p - 0.382 * r, p - 0.618 * r, p - r, NA, NA],
        2 => { let p = (h + l + 2. * o_cur) / 4.; let (r3, s3) = (h + 2. * (p - l), l - 2. * (h - p)); [p, 2. * p - l, p + r, r3, r3 + r, NA, 2. * p - h, p - r, s3, s3 - r, NA] }
        3 => [p, 2. * p - l, p + r, p + 2. * r, p + 3. * r, NA, 2. * p - h, p - r, p - 2. * r, p - 3. * r, NA],
        4 => { let x = if c < o_prev { h + 2. * l + c } else if c > o_prev { 2. * h + l + c } else { h + l + 2. * c }; [x / 4., x / 2. - l, NA, NA, NA, NA, x / 2. - h, NA, NA, NA, NA] }
        _ => { let r5 = div(h, l) * c; [p, c + 1.1 * r / 12., c + 1.1 * r / 6., c + 1.1 * r / 4., c + 1.1 * r / 2., r5, c - 1.1 * r / 12., c - 1.1 * r / 6., c - 1.1 * r / 4., c - 1.1 * r / 2., c - (r5 - c)] }
    }
}
/// Per bar: (current period [o, h, l, c] so far, previous completed period if any).
fn periods(b: &Bars, per: usize) -> Vec<([f64; 4], Option<[f64; 4]>)> {
    let (mut last, mut cur, mut prev) = (None, [NA; 4], None);
    (0..b.len()).map(|i| {
        let k = period_key(&b.date[i], per);
        if last != Some(k) { if last.is_some() { prev = Some(cur); } cur = [b.open[i], b.high[i], b.low[i], b.close[i]]; last = Some(k); }
        else { cur[1] = cur[1].max(b.high[i]); cur[2] = cur[2].min(b.low[i]); cur[3] = b.close[i]; }
        (cur, prev)
    }).collect()
}
fn pivots_std(b: &Bars, prm: &Params) -> Output {
    let t = n(prm, "type");
    let mut out = vec![vec![NA; b.len()]; 11];
    for (i, (cur, prev)) in periods(b, n(prm, "period")).into_iter().enumerate() {
        if let Some(pv) = prev { for (j, v) in levels(t, pv[1], pv[2], pv[3], pv[0], cur[0]).into_iter().enumerate() { out[j][i] = v; } }
    }
    Ok(out)
}
fn mtp(b: &Bars, prm: &Params) -> Output {
    let mut out = vec![vec![NA; b.len()]; 6];
    for (i, (cur, prev)) in periods(b, n(prm, "period")).into_iter().enumerate() {
        out[0][i] = cur[0]; out[1][i] = cur[1]; out[2][i] = cur[2];
        if let Some(pv) = prev { out[3][i] = pv[1]; out[4][i] = pv[2]; out[5][i] = pv[3]; }
    }
    Ok(out)
}
fn performance(b: &Bars, _: &Params) -> Output {
    let c = &b.close;
    let roc = |k: usize| -> Vec<f64> { (0..c.len()).map(|i| if i >= k { div(100. * (c[i] - c[i - k]), c[i - k]) } else { NA }).collect() };
    let (mut base, mut year, mut prev_close) = (NA, None, NA);
    let ytd: Vec<f64> = (0..c.len()).map(|i| {
        let y = period_key(&b.date[i], 4);
        if year.is_some() && year != Some(y) { base = prev_close; }
        year = Some(y); prev_close = c[i];
        div(100. * (c[i] - base), base)
    }).collect();
    Ok(vec![roc(5), roc(21), roc(63), roc(126), ytd, roc(252)])
}
fn calendar(b: &Bars, _: &Params) -> Output {
    let len = b.len();
    let mut out = vec![vec![NA; len]; 6];
    for i in 0..len {
        let d = &b.date[i];
        let dn = day_number(d);
        let num = |r: std::ops::Range<usize>| d.get(r).and_then(|s| s.parse::<f64>().ok()).unwrap_or(f64::NAN);
        out[0][i] = (dn + 4).rem_euclid(7) as f64; // 1970-01-01 was a Thursday (4), Sunday = 0
        out[1][i] = num(8..10); out[2][i] = num(5..7); out[3][i] = num(0..4);
        out[4][i] = (dn - day_number(&format!("{}-01-01", d.get(0..4).unwrap_or("1970"))) + 1) as f64;
        out[5][i] = i as f64;
    }
    Ok(out)
}
/// Meeus, Astronomical Algorithms ch. 49: days since 1970-01-01 UTC of lunation k (k = 0: new moon 2000-01-06); full moon = k + 0.5.
fn phase_day(k: f64, full: bool) -> f64 {
    let k = if full { k + 0.5 } else { k };
    let t = k / 1236.85;
    let jde = 2451550.09766 + 29.530588861 * k + 0.00015437 * t * t - 0.000000150 * t.powi(3) + 0.00000000073 * t.powi(4);
    let r = f64::to_radians;
    let e = 1. - 0.002516 * t - 0.0000074 * t * t;
    let m = r(2.5534 + 29.10535670 * k - 0.0000014 * t * t);
    let mp = r(201.5643 + 385.81693528 * k + 0.0107582 * t * t);
    let f = r(160.7108 + 390.67050284 * k - 0.0016118 * t * t);
    let om = r(124.7746 - 1.56375588 * k + 0.0020672 * t * t);
    let c: [f64; 15] = if full { [-0.40614, 0.17302, 0.01614, 0.01043, 0.00734, -0.00515, 0.00209, -0.00111, -0.00057, 0.00056, -0.00042, 0.00042, 0.00038, -0.00024, -0.00017] }
                       else { [-0.40720, 0.17241, 0.01608, 0.01039, 0.00739, -0.00514, 0.00208, -0.00111, -0.00057, 0.00056, -0.00042, 0.00042, 0.00038, -0.00024, -0.00017] };
    let s: [f64; 15] = [mp.sin(), e * m.sin(), (2. * mp).sin(), (2. * f).sin(), e * (mp - m).sin(), e * (mp + m).sin(), e * e * (2. * m).sin(),
        (mp - 2. * f).sin(), (mp + 2. * f).sin(), e * (2. * mp + m).sin(), (3. * mp).sin(), e * (m + 2. * f).sin(), e * (m - 2. * f).sin(), e * (2. * mp - m).sin(), om.sin()];
    jde + c.iter().zip(&s).map(|(a, b)| a * b).sum::<f64>() - 2440587.5
}
fn moon_phases(b: &Bars, _: &Params) -> Output {
    let days: Vec<i64> = b.date.iter().map(|d| day_number(d)).collect();
    let (mut nm, mut fm) = (vec![0.; b.len()], vec![0.; b.len()]);
    for i in 0..days.len() {
        let (lo, hi) = ((if i == 0 { days[0] } else { days[i - 1] + 1 }) as f64, (days[i] + 1) as f64);
        if hi <= lo { continue; }
        let k0 = ((lo - 10962.6) / 29.530588861).floor() - 1.;
        let span = ((hi - lo) / 29.530588861).ceil() as i64 + 3;
        for j in 0..span {
            let k = k0 + j as f64;
            let t = phase_day(k, false); if lo <= t && t < hi { nm[i] = 1.; }
            let t = phase_day(k, true); if lo <= t && t < hi { fm[i] = 1.; }
        }
    }
    Ok(vec![nm, fm])
}
fn bars_since(x: &[f64], k: usize, high: bool) -> Vec<f64> {
    (0..x.len()).map(|i| {
        if i + 1 < k { return NA; }
        let mut best = 0; // offset from bar i; strict comparison keeps the most recent on ties
        for j in 1..k { if (high && x[i - j] > x[i - best]) || (!high && x[i - j] < x[i - best]) { best = j; } }
        best as f64
    }).collect()
}
fn highest_bars(b: &Bars, prm: &Params) -> Output { Ok(vec![bars_since(&b.high, n(prm, "length"), true)]) }
fn lowest_bars(b: &Bars, prm: &Params) -> Output { Ok(vec![bars_since(&b.low, n(prm, "length"), false)]) }
```

`sources.rs` rows:

```rust
    ("pivots_std", "Pivot Points Standard", ""),
    ("mtp", "Multi-Time Period Charts", ""),
    ("performance", "Performance", ""),
    ("calendar", "", "DayOfWeek; Day; Month; Year; DayOfYear; BarIndex"),
    ("moon_phases", "Moon Phases", ""),
    ("highest_bars", "ta.highestbars (Pine)", "HHVBars; HighestBars"),
    ("lowest_bars", "ta.lowestbars (Pine)", "LLVBars; LowestBars"),
```

- [ ] **Step 4: Register and run** `cargo test lab::` → PASS (the generic tests now run these on 1,000 real dates from 2020-01-01); guard clean.
- [ ] **Step 5: Commit** `git commit -m "Add lab pivot points standard, multi-time period, performance, calendar, moon phases and bars-since-extreme"`

### Task 13: Fractals, divergences and bar patterns — `patterns.rs`

**Files:** Create `lab/patterns.rs`; modify `lab/price.rs` (extract `pub(super) fn pivots`), `lab/mod.rs`, `catalogue.rs` (`all`), `sources.rs`, parity doc.

**Interfaces:** Produces `price::pivots(x: &[f64], left: usize, right: usize, high: bool) -> Vec<f64>`, which `pivot_high`/`pivot_low` call with unchanged results; ids `fractals`, `rsi_divergence`, `ad_ratio_bars`, `bar_patterns`.

Category "Patterns", placement Pane except `fractals` (Price), needs NONE.

| id | name | params | outputs | first | sparse | definition |
|---|---|---|---|---|---|---|
| fractals | Williams Fractal | periods 2 (1–100) | up, down | [MAX, MAX] | yes | "TradingView built-in: up fractal at bar t − n if its high is above the n bars after it (strict) and above the n bars before it (strict), allowing up to 4 equal-or-lower bars immediately before; reported on bar t (confirmed n bars late) with the fractal's high; down mirrors with lows; NaN otherwise" |
| rsi_divergence | RSI Divergence | length 14, lbr 5, lbl 5, range_upper 60, range_lower 5 | bull, hidden_bull, bear, hidden_bear | [0,0,0,0] | no | "TradingView RSI Divergence Indicator: osc = rsi(close, length); pivots of osc (lbl, lbr); on a confirmed pivot low, compared with the previous one (bars since it, range_lower..range_upper): bull = osc higher low and price lower low; hidden_bull = osc lower low and price higher low; bear/hidden_bear mirror on pivot highs; 1 on the confirmation bar, else 0" |
| ad_ratio_bars | Advance/Decline Ratio (Bars) | length 9 | ratio | [MAX] | yes | "count(close > open)/count(close < open) over length bars; no down bar → NaN (declared default 9)" |
| bar_patterns | Gap and Inside/Outside Bars | — | gap_up, gap_down, inside, outside | [0,0,0,0] | no | "AmiBroker GapUp: low > high[1]; GapDown: high < low[1]; Inside: high < high[1] and low > low[1]; Outside: high > high[1] and low < low[1]; 1/0, bar 0 = 0" |

- [ ] **Step 1: Write the failing tests**

```rust
#[cfg(test)]
mod tests {
    use crate::lab::testdata::{assert_series, compiles, d1, from_ohlc, go};
    const N: f64 = f64::NAN;
    fn highs(h: &[f64]) -> crate::lab::bars::Bars { from_ohlc(&h.iter().map(|&x| (x, x, x - 1., x)).collect::<Vec<_>>()) }
    #[test] fn fractals_confirmed_late_with_tie_rules() {
        let o = go("fractals", &highs(&[1., 2., 3., 5., 4., 3., 2., 3., 3., 6., 5., 4.]), &[("periods", 2.)]);
        assert_series(&o[0], &[N, N, N, N, N, 5., N, N, N, N, N, 6.]);
        assert_series(&o[1], &[N, N, N, N, N, N, N, N, 1., N, N, N]);
        assert_series(&go("fractals", &highs(&[1., 2., 5., 5., 3., 2.]), &[("periods", 2.)])[0], &[N, N, N, N, N, 5.]); // equal bar on the left allowed
        assert_series(&go("fractals", &highs(&[1., 2., 5., 3., 5., 2.]), &[("periods", 2.)])[0], &[N; 6]); // equal bar on the right cancels
    }
    #[test] fn rsi_bearish_divergence_on_confirmation_bar() {
        let c = [10., 9., 8., 9., 10., 9., 7.5, 8.5, 9., 8., 9.5, 10.5, 11., 10.2, 10.8, 11.5, 11.2, 12., 11.];
        let b = from_ohlc(&c.iter().map(|&x| (x, x + 0.2, x - 0.2, x)).collect::<Vec<_>>());
        let o = go("rsi_divergence", &b, &[("length", 3.), ("lbr", 1.), ("lbl", 1.), ("range_upper", 10.), ("range_lower", 1.)]);
        let mut bear = vec![0.; 19]; bear[16] = 1.;
        assert_series(&o[2], &bear);
        for k in [0, 1, 3] { assert!(o[k].iter().all(|v| *v == 0.), "output {k}"); }
    }
    #[test] fn advance_decline_bar_ratio() {
        assert_series(&go("ad_ratio_bars", &d1(), &[("length", 3.)])[0], &[N, N, N, 2., 0.5, 0.5, 2., N]); // no down bar in the last window → NaN
    }
    #[test] fn gaps_inside_outside() {
        let b = from_ohlc(&[(10., 12., 9., 11.), (13., 14., 12.5, 13.5), (12., 12., 11., 11.5), (11.6, 11.9, 11.2, 11.5), (11., 12.5, 10.5, 12.)]);
        let o = go("bar_patterns", &b, &[]);
        assert_series(&o[0], &[0., 1., 0., 0., 0.]);
        assert_series(&o[1], &[0., 0., 1., 0., 0.]);
        assert_series(&o[2], &[0., 0., 0., 1., 0.]);
        assert_series(&o[3], &[0., 0., 0., 0., 1.]);
    }
    #[test] fn rules_reach_patterns() { for (id, out) in [("fractals", "down"), ("rsi_divergence", "bear"), ("bar_patterns", "outside")] { compiles(id, out).unwrap(); } }
}
```

- [ ] **Step 2: Run** `CARGO_BUILD_JOBS=2 cargo test lab::patterns` → FAIL.

- [ ] **Step 3: Implement**

`price.rs`: move the body of `pivot` into

```rust
/// Pivot at p = t − right, reported on bar t: ties on the left allowed, ties on the right cancel (ta.pivothigh/low).
pub(super) fn pivots(x: &[f64], left: usize, right: usize, high: bool) -> Vec<f64> { /* the existing body, using left/right */ }
fn pivot(x: &[f64], prm: &Params, high: bool) -> Output { Ok(vec![pivots(x, n(prm, "left"), n(prm, "right"), high)]) }
```
(The existing price tests must pass unchanged.)

`patterns.rs` (convention header; `CAT = "Patterns"`; `use super::catalogue::{get, run};`):

```rust
fn fractal(x: &[f64], k: usize, up: bool) -> Vec<f64> {
    let beats = |a: f64, v: f64| if up { v < a } else { v > a };
    let ties = |a: f64, v: f64| if up { v <= a } else { v >= a };
    (0..x.len()).map(|t| {
        if t < 2 * k { return NA; }
        let p = t - k;
        if !(1..=k).all(|i| beats(x[p], x[p + i])) { return NA; }
        let left = (0..=4usize).any(|e| p >= k + e && (1..=e).all(|j| ties(x[p], x[p - j])) && (1..=k).all(|i| beats(x[p], x[p - e - i])));
        if left { x[p] } else { NA }
    }).collect()
}
fn fractals(b: &Bars, prm: &Params) -> Output { let k = n(prm, "periods"); Ok(vec![fractal(&b.high, k, true), fractal(&b.low, k, false)]) }
fn rsi_divergence(b: &Bars, prm: &Params) -> Output {
    let rp: Params = [("length".to_string(), p(prm, "length"))].into_iter().collect();
    let osc = run(&get("rsi").expect("rsi is in the catalogue"), b, &rp)?.remove(0);
    let (lbr, lbl, hi_r, lo_r) = (n(prm, "lbr"), n(prm, "lbl"), n(prm, "range_upper"), n(prm, "range_lower"));
    let pl: Vec<bool> = super::price::pivots(&osc, lbl, lbr, false).iter().map(|v| !v.is_nan()).collect();
    let ph: Vec<bool> = super::price::pivots(&osc, lbl, lbr, true).iter().map(|v| !v.is_nan()).collect();
    let len = b.len();
    let mut out = vec![vec![0.; len]; 4];
    let (mut last_pl, mut last_ph): (Option<(f64, f64, usize)>, Option<(f64, f64, usize)>) = (None, None); // (osc, price, bar) of the previous pivot
    // _inRange(flag[1]) = barssince(flag[1]) in [range_lower, range_upper]: bars since the previous pivot bar j is i − 1 − j
    let in_range = |i: usize, j: usize| { let bars = i - 1 - j; lo_r <= bars && bars <= hi_r };
    for i in 0..len {
        if pl[i] {
            let (o, pr) = (osc[i - lbr], b.low[i - lbr]);
            if let Some((po, pp, j)) = last_pl { if in_range(i, j) {
                if o > po && pr < pp { out[0][i] = 1.; }
                if o < po && pr > pp { out[1][i] = 1.; }
            } }
            last_pl = Some((o, pr, i));
        }
        if ph[i] {
            let (o, pr) = (osc[i - lbr], b.high[i - lbr]);
            if let Some((po, pp, j)) = last_ph { if in_range(i, j) {
                if o < po && pr > pp { out[2][i] = 1.; }
                if o > po && pr < pp { out[3][i] = 1.; }
            } }
            last_ph = Some((o, pr, i));
        }
    }
    Ok(out)
}
fn ad_ratio_bars(b: &Bars, prm: &Params) -> Output {
    let k = n(prm, "length");
    let up: Vec<f64> = (0..b.len()).map(|i| if b.close[i] > b.open[i] { 1. } else { 0. }).collect();
    let dn: Vec<f64> = (0..b.len()).map(|i| if b.close[i] < b.open[i] { 1. } else { 0. }).collect();
    Ok(vec![ta::sum(&up, k).iter().zip(ta::sum(&dn, k)).map(|(u, d)| div(*u, d)).collect()])
}
fn bar_patterns(b: &Bars, _: &Params) -> Output {
    let (h, l) = (&b.high, &b.low);
    let f = |c: &dyn Fn(usize) -> bool| -> Vec<f64> { (0..b.len()).map(|i| if i > 0 && c(i) { 1. } else { 0. }).collect() };
    Ok(vec![f(&|i| l[i] > h[i - 1]), f(&|i| h[i] < l[i - 1]), f(&|i| h[i] < h[i - 1] && l[i] > l[i - 1]), f(&|i| h[i] > h[i - 1] && l[i] < l[i - 1])])
}
```
`fractals` and `ad_ratio_bars` rows use `sparse: true` and `first: &[usize::MAX, usize::MAX]` / `&[usize::MAX]`.

`sources.rs` rows:

```rust
    ("fractals", "Williams fractal indicator; Williams Fractal", ""),
    ("rsi_divergence", "RSI divergence indicator", ""),
    ("ad_ratio_bars", "Advance/Decline Ratio (Bars); Advance/Decline (charting library)", ""),
    ("bar_patterns", "", "GapUp; GapDown; Inside; Outside"),
```

- [ ] **Step 4: Register and run** `cargo test lab::` → PASS (price tests unchanged); guard clean.
- [ ] **Step 5: Commit** `git commit -m "Add lab Williams fractals, RSI divergence, A/D bar ratio and gap/inside/outside bars"`

### Task 14: Candlestick patterns I — `candles.rs` framework and one/two-bar patterns

**Files:** Create `lab/candles.rs`; modify `lab/mod.rs`, `catalogue.rs` (`all`), `sources.rs`, parity doc.

**Interfaces:** Produces `candles::feats(b, trend) -> F` (TradingView `C_*` variables), the `pattern!` macro, 30 ids `cdl_*` (below), and `const TREND: &[ParamSpec]` (`trend` 0 SMA50 default, 1 SMA50+SMA200, 2 none). Task 15 adds the remaining 14 patterns and `cdl_all` to the same file.

Every pattern: category "Candlestick patterns", placement Pane, needs NONE, outputs `["signal"]`, first `[0]`, `sparse: false`; definition `"<name>: TradingView All Candlestick Patterns rule (Pine v4 source, see candles.rs); 1 on the bar the pattern completes, else 0"`. Params: `TREND` for patterns marked T, none otherwise. Source of the formulas: Decision 20. Shared features (Pine names in brackets):

| feature | definition |
|---|---|
| up / dn [C_UpTrend/C_DownTrend] | trend 0: close > / < sma(close,50); 1: close > sma50 > sma200 / close < sma50 < sma200; 2: always true; NaN SMA → false |
| bh, bl, body, mid | max(o,c), min(o,c), bh − bl, bl + body/2 |
| avg [C_BodyAvg] | ema(body, 14) |
| us, ds, rng | high − bh, bl − low, high − low |
| small / long | body < avg / body > avg (false while avg is NaN) |
| hus / hds | us > 0.05·body / ds > 0.05·body |
| white / black | open < close / open > close |
| dojib [C_IsDojiBody] | rng > 0 and body ≤ 0.05·rng |
| doji [C_Doji] | dojib and (us == ds or (\|us − ds\|/ds·100 < 100 and \|ds − us\|/us·100 < 100)) |

- [ ] **Step 1: Write the failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::DEFS;
    use crate::lab::catalogue::get;
    use crate::lab::testdata::{compiles, from_ohlc, go};
    const PRE: (f64, f64, f64, f64) = (10., 11.2, 9.8, 11.); // 14 identical bars: body 1, so the body EMA is 1
    pub(super) fn case(rows: &[(f64, f64, f64, f64)]) -> crate::lab::bars::Bars { let mut all = vec![PRE; 14]; all.extend_from_slice(rows); from_ohlc(&all) }
    /// The pattern's value on the last bar, with trend detection off (trend = 2) where the pattern has a trend rule.
    pub(super) fn last(id: &str, rows: &[(f64, f64, f64, f64)]) -> f64 {
        let kv: &[(&str, f64)] = if get(id).unwrap().params.is_empty() { &[] } else { &[("trend", 2.)] };
        *go(id, &case(rows), kv)[0].last().unwrap()
    }
    const CASES: &[(&str, &[(f64, f64, f64, f64)])] = &[
        ("cdl_dark_cloud_cover", &[(10.0, 13.1, 9.9, 13.0), (13.2, 13.3, 11.0, 11.2)]),
        ("cdl_doji", &[(11.0, 11.6, 10.4, 11.02)]),
        ("cdl_doji_star_bear", &[(10.0, 13.1, 9.9, 13.0), (13.5, 13.8, 13.2, 13.51)]),
        ("cdl_doji_star_bull", &[(13.0, 13.1, 9.9, 10.0), (9.5, 9.8, 9.2, 9.51)]),
        ("cdl_dragonfly_doji", &[(11.0, 11.01, 10.0, 11.01)]),
        ("cdl_engulfing_bear", &[(10.5, 11.1, 10.4, 11.0), (11.2, 11.3, 9.9, 10.0)]),
        ("cdl_engulfing_bull", &[(11.0, 11.1, 10.4, 10.5), (10.3, 12.1, 10.2, 12.0)]),
        ("cdl_falling_window", &[(11.0, 11.2, 10.8, 11.0), (10.4, 10.6, 10.2, 10.5)]),
        ("cdl_gravestone_doji", &[(11.0, 12.0, 11.0, 11.01)]),
        ("cdl_hammer", &[(11.0, 11.1, 9.6, 11.1)]),
        ("cdl_hanging_man", &[(11.0, 11.1, 9.6, 11.1)]),
        ("cdl_harami_bear", &[(10.0, 13.1, 9.9, 13.0), (12.5, 12.6, 11.6, 12.0)]),
        ("cdl_harami_bull", &[(13.0, 13.1, 9.9, 10.0), (10.5, 11.4, 10.4, 11.0)]),
        ("cdl_harami_cross_bear", &[(10.0, 13.1, 9.9, 13.0), (11.5, 12.0, 11.0, 11.51)]),
        ("cdl_harami_cross_bull", &[(13.0, 13.1, 9.9, 10.0), (11.5, 12.0, 11.0, 11.51)]),
        ("cdl_inverted_hammer", &[(11.0, 12.6, 11.0, 11.1)]),
        ("cdl_kicking_bear", &[(10.0, 13.0, 10.0, 13.0), (9.5, 9.5, 6.5, 6.5)]),
        ("cdl_kicking_bull", &[(13.0, 13.0, 10.0, 10.0), (13.5, 16.5, 13.5, 16.5)]),
        ("cdl_long_lower_shadow", &[(11.0, 11.1, 9.0, 11.05)]),
        ("cdl_long_upper_shadow", &[(11.0, 13.0, 10.95, 10.98)]),
        ("cdl_marubozu_black", &[(13.0, 13.0, 10.0, 10.0)]),
        ("cdl_marubozu_white", &[(10.0, 13.0, 10.0, 13.0)]),
        ("cdl_on_neck", &[(13.0, 13.1, 10.0, 10.1), (9.5, 10.2, 9.4, 10.02)]),
        ("cdl_piercing", &[(13.0, 13.1, 10.0, 10.1), (9.9, 12.0, 9.8, 11.9)]),
        ("cdl_rising_window", &[(11.0, 11.2, 10.8, 11.0), (11.4, 11.6, 11.3, 11.5)]),
        ("cdl_shooting_star", &[(11.0, 12.6, 11.0, 11.1)]),
        ("cdl_spinning_top_black", &[(11.1, 11.8, 10.2, 11.0)]),
        ("cdl_spinning_top_white", &[(11.0, 11.8, 10.2, 11.1)]),
        ("cdl_tweezer_bottom", &[(13.0, 13.1, 9.9, 10.0), (10.5, 12.0, 9.9, 11.8)]),
        ("cdl_tweezer_top", &[(10.0, 13.1, 9.9, 13.0), (12.5, 13.1, 11.0, 11.2)]),
    ];
    #[test] fn each_pattern_fires_on_its_case() { for (id, rows) in CASES { assert_eq!(last(id, rows), 1., "{id}"); } }
    #[test] fn plain_bars_fire_nothing() {
        for d in DEFS.iter().filter(|d| d.id.starts_with("cdl_") && d.id != "cdl_all") {
            let kv: &[(&str, f64)] = if d.params.is_empty() { &[] } else { &[("trend", 2.)] };
            assert!(go(d.id, &case(&[PRE]), kv)[0].iter().all(|v| *v == 0.), "{}", d.id);
        }
    }
    #[test] fn trend_rule_gates_reversal_patterns() {
        // with the default SMA50 trend rule, 15 bars have no trend yet, so a hammer is not reported
        assert_eq!(*go("cdl_hammer", &case(&[(11., 11.1, 9.6, 11.1)]), &[])[0].last().unwrap(), 0.);
    }
    #[test] fn rules_reach_candles() { compiles("cdl_engulfing_bull", "signal").unwrap(); compiles("cdl_doji", "signal").unwrap(); }
}
```

- [ ] **Step 2: Run** `CARGO_BUILD_JOBS=2 cargo test lab::candles` → FAIL.

- [ ] **Step 3: Implement**

```rust
//! Candlestick patterns (TradingView built-ins). Formulas: TradingView "All Candlestick Patterns" (Pine v4), Decision 20.
use super::bars::Bars;
use super::catalogue::{get, int, n, p, run, IndicatorDef, Needs, Output, ParamSpec, Params, Placement};
use super::ta;

const CAT: &str = "Candlestick patterns";
const TREND: &[ParamSpec] = &[int("trend", "Trend rule (0 SMA50, 1 SMA50+SMA200, 2 none)", 0., 0., 2.)];

/// TradingView C_* candle features per bar; booleans are false where an input is missing, as Pine bools are.
pub(super) struct F {
    up: Vec<bool>, dn: Vec<bool>, bh: Vec<f64>, bl: Vec<f64>, body: Vec<f64>, mid: Vec<f64>, avg: Vec<f64>, us: Vec<f64>, ds: Vec<f64>, rng: Vec<f64>,
    small: Vec<bool>, long: Vec<bool>, hus: Vec<bool>, hds: Vec<bool>, white: Vec<bool>, black: Vec<bool>, dojib: Vec<bool>, doji: Vec<bool>, hl2: Vec<f64>,
}
pub(super) fn feats(b: &Bars, trend: usize) -> F {
    let (o, h, l, c, len) = (&b.open, &b.high, &b.low, &b.close, b.len());
    let (s50, s200) = (ta::sma(c, 50), ta::sma(c, 200));
    let (up, dn): (Vec<bool>, Vec<bool>) = (0..len).map(|i| match trend {
        0 => (c[i] > s50[i], c[i] < s50[i]),
        1 => (c[i] > s50[i] && s50[i] > s200[i], c[i] < s50[i] && s50[i] < s200[i]),
        _ => (true, true),
    }).unzip();
    let bh: Vec<f64> = (0..len).map(|i| o[i].max(c[i])).collect();
    let bl: Vec<f64> = (0..len).map(|i| o[i].min(c[i])).collect();
    let body: Vec<f64> = (0..len).map(|i| bh[i] - bl[i]).collect();
    let avg = ta::ema(&body, 14);
    let us: Vec<f64> = (0..len).map(|i| h[i] - bh[i]).collect();
    let ds: Vec<f64> = (0..len).map(|i| bl[i] - l[i]).collect();
    let rng: Vec<f64> = (0..len).map(|i| h[i] - l[i]).collect();
    let dojib: Vec<bool> = (0..len).map(|i| rng[i] > 0. && body[i] <= rng[i] * 0.05).collect();
    // x/0 is inf or NaN in Rust and na in Pine: both make `< 100` false
    let doji: Vec<bool> = (0..len).map(|i| dojib[i] && (us[i] == ds[i] || ((us[i] - ds[i]).abs() / ds[i] * 100. < 100. && (ds[i] - us[i]).abs() / us[i] * 100. < 100.))).collect();
    F { small: (0..len).map(|i| body[i] < avg[i]).collect(), long: (0..len).map(|i| body[i] > avg[i]).collect(),
        hus: (0..len).map(|i| us[i] > 0.05 * body[i]).collect(), hds: (0..len).map(|i| ds[i] > 0.05 * body[i]).collect(),
        white: (0..len).map(|i| o[i] < c[i]).collect(), black: (0..len).map(|i| o[i] > c[i]).collect(),
        mid: (0..len).map(|i| body[i] / 2. + bl[i]).collect(), hl2: b.hl2(), up, dn, bh, bl, body, avg, us, ds, rng, dojib, doji }
}
fn ham(f: &F, i: usize) -> bool { f.small[i] && f.body[i] > 0. && f.bl[i] > f.hl2[i] && f.ds[i] >= 2. * f.body[i] && !f.hus[i] }
fn inv(f: &F, i: usize) -> bool { f.small[i] && f.body[i] > 0. && f.bh[i] < f.hl2[i] && f.us[i] >= 2. * f.body[i] && !f.hds[i] }
fn maru(f: &F, i: usize) -> bool { f.long[i] && f.us[i] <= 0.05 * f.body[i] && f.ds[i] <= 0.05 * f.body[i] }
fn spin(f: &F, i: usize) -> bool { f.ds[i] >= f.rng[i] * 0.34 && f.us[i] >= f.rng[i] * 0.34 && !f.dojib[i] }
fn drag(f: &F, i: usize) -> bool { f.dojib[i] && f.us[i] <= f.body[i] }
fn grav(f: &F, i: usize) -> bool { f.dojib[i] && f.ds[i] <= f.body[i] }

/// One compute fn per pattern: 1 where `$e` holds on bar `$i`, else 0. `trend` patterns read the `trend` parameter.
macro_rules! pattern {
    ($name:ident, trend, |$f:ident, $b:ident, $i:ident| $e:expr) => {
        #[allow(unused_variables)]
        fn $name(b: &Bars, prm: &Params) -> Output { let $f = feats(b, n(prm, "trend")); let $b = b; Ok(vec![(0..b.len()).map(|$i| if $e { 1. } else { 0. }).collect()]) }
    };
    ($name:ident, |$f:ident, $b:ident, $i:ident| $e:expr) => {
        #[allow(unused_variables)]
        fn $name(b: &Bars, _: &Params) -> Output { let $f = feats(b, 2); let $b = b; Ok(vec![(0..b.len()).map(|$i| if $e { 1. } else { 0. }).collect()]) }
    };
}
pattern!(dark_cloud_cover, trend, |f, b, i| i >= 1 && f.up[i - 1] && f.white[i - 1] && f.long[i - 1] && f.black[i] && b.open[i] >= b.high[i - 1] && b.close[i] < f.mid[i - 1] && b.close[i] > b.open[i - 1]);
pattern!(doji, |f, b, i| f.doji[i] && !drag(&f, i) && !grav(&f, i));
pattern!(doji_star_bear, trend, |f, b, i| i >= 1 && f.up[i] && f.white[i - 1] && f.long[i - 1] && f.dojib[i] && f.bl[i] > f.bh[i - 1]);
pattern!(doji_star_bull, trend, |f, b, i| i >= 1 && f.dn[i] && f.black[i - 1] && f.long[i - 1] && f.dojib[i] && f.bh[i] < f.bl[i - 1]);
pattern!(dragonfly_doji, |f, b, i| drag(&f, i));
pattern!(engulfing_bear, trend, |f, b, i| i >= 1 && f.up[i] && f.black[i] && f.long[i] && f.white[i - 1] && f.small[i - 1] && b.close[i] <= b.open[i - 1] && b.open[i] >= b.close[i - 1] && (b.close[i] < b.open[i - 1] || b.open[i] > b.close[i - 1]));
pattern!(engulfing_bull, trend, |f, b, i| i >= 1 && f.dn[i] && f.white[i] && f.long[i] && f.black[i - 1] && f.small[i - 1] && b.close[i] >= b.open[i - 1] && b.open[i] <= b.close[i - 1] && (b.close[i] > b.open[i - 1] || b.open[i] < b.close[i - 1]));
pattern!(falling_window, trend, |f, b, i| i >= 1 && f.dn[i - 1] && f.rng[i] != 0. && f.rng[i - 1] != 0. && b.high[i] < b.low[i - 1]);
pattern!(gravestone_doji, |f, b, i| grav(&f, i));
pattern!(hammer, trend, |f, b, i| ham(&f, i) && f.dn[i]);
pattern!(hanging_man, trend, |f, b, i| ham(&f, i) && f.up[i]);
pattern!(harami_bear, trend, |f, b, i| i >= 1 && f.long[i - 1] && f.white[i - 1] && f.up[i - 1] && f.black[i] && f.small[i] && b.high[i] <= f.bh[i - 1] && b.low[i] >= f.bl[i - 1]);
pattern!(harami_bull, trend, |f, b, i| i >= 1 && f.long[i - 1] && f.black[i - 1] && f.dn[i - 1] && f.white[i] && f.small[i] && b.high[i] <= f.bh[i - 1] && b.low[i] >= f.bl[i - 1]);
pattern!(harami_cross_bear, trend, |f, b, i| i >= 1 && f.long[i - 1] && f.white[i - 1] && f.up[i - 1] && f.dojib[i] && b.high[i] <= f.bh[i - 1] && b.low[i] >= f.bl[i - 1]);
pattern!(harami_cross_bull, trend, |f, b, i| i >= 1 && f.long[i - 1] && f.black[i - 1] && f.dn[i - 1] && f.dojib[i] && b.high[i] <= f.bh[i - 1] && b.low[i] >= f.bl[i - 1]);
pattern!(inverted_hammer, trend, |f, b, i| inv(&f, i) && f.dn[i]);
pattern!(kicking_bear, |f, b, i| i >= 1 && maru(&f, i - 1) && f.white[i - 1] && maru(&f, i) && f.black[i] && b.low[i - 1] > b.high[i]);
pattern!(kicking_bull, |f, b, i| i >= 1 && maru(&f, i - 1) && f.black[i - 1] && maru(&f, i) && f.white[i] && b.high[i - 1] < b.low[i]);
pattern!(long_lower_shadow, |f, b, i| f.ds[i] > f.rng[i] * 0.75);
pattern!(long_upper_shadow, |f, b, i| f.us[i] > f.rng[i] * 0.75);
pattern!(marubozu_black, |f, b, i| maru(&f, i) && f.black[i]);
pattern!(marubozu_white, |f, b, i| maru(&f, i) && f.white[i]);
pattern!(on_neck, trend, |f, b, i| i >= 1 && f.dn[i] && f.black[i - 1] && f.long[i - 1] && f.white[i] && b.open[i] < b.close[i - 1] && f.small[i] && f.rng[i] != 0. && (b.close[i] - b.low[i - 1]).abs() <= f.avg[i] * 0.05);
pattern!(piercing, trend, |f, b, i| i >= 1 && f.dn[i - 1] && f.black[i - 1] && f.long[i - 1] && f.white[i] && b.open[i] <= b.low[i - 1] && b.close[i] > f.mid[i - 1] && b.close[i] < b.open[i - 1]);
pattern!(rising_window, trend, |f, b, i| i >= 1 && f.up[i - 1] && f.rng[i] != 0. && f.rng[i - 1] != 0. && b.low[i] > b.high[i - 1]);
pattern!(shooting_star, trend, |f, b, i| inv(&f, i) && f.up[i]);
pattern!(spinning_top_black, |f, b, i| spin(&f, i) && f.black[i]);
pattern!(spinning_top_white, |f, b, i| spin(&f, i) && f.white[i]);
// TradingView's source tests C_UpTrend[1] for Tweezer Bottom too; kept as published (Decision 20).
pattern!(tweezer_bottom, trend, |f, b, i| i >= 1 && f.up[i - 1] && (!f.dojib[i] || (f.hus[i] && f.hds[i])) && (b.low[i] - b.low[i - 1]).abs() <= f.avg[i] * 0.05 && f.black[i - 1] && f.white[i] && f.long[i - 1]);
pattern!(tweezer_top, trend, |f, b, i| i >= 1 && f.up[i - 1] && (!f.dojib[i] || (f.hus[i] && f.hds[i])) && (b.high[i] - b.high[i - 1]).abs() <= f.avg[i] * 0.05 && f.white[i - 1] && f.black[i] && f.long[i - 1]);

const fn cdl(id: &'static str, name: &'static str, params: &'static [ParamSpec], f: fn(&Bars, &Params) -> Output) -> IndicatorDef {
    IndicatorDef { id, name, category: CAT, params, outputs: &["signal"], placement: Placement::Pane, needs: Needs::NONE,
        definition: "TradingView All Candlestick Patterns rule (Pine v4 source, see lab/candles.rs); 1 on the bar the pattern completes, else 0",
        first: &[0], sparse: false, compute: Some(f), note: "" }
}
pub const DEFS: &[IndicatorDef] = &[
    cdl("cdl_dark_cloud_cover", "Dark Cloud Cover - Bearish", TREND, dark_cloud_cover),
    cdl("cdl_doji", "Doji", &[], doji),
    cdl("cdl_doji_star_bear", "Doji Star - Bearish", TREND, doji_star_bear),
    cdl("cdl_doji_star_bull", "Doji Star - Bullish", TREND, doji_star_bull),
    cdl("cdl_dragonfly_doji", "Dragonfly Doji - Bullish", &[], dragonfly_doji),
    cdl("cdl_engulfing_bear", "Engulfing - Bearish", TREND, engulfing_bear),
    cdl("cdl_engulfing_bull", "Engulfing - Bullish", TREND, engulfing_bull),
    cdl("cdl_falling_window", "Falling Window - Bearish", TREND, falling_window),
    cdl("cdl_gravestone_doji", "Gravestone Doji - Bearish", &[], gravestone_doji),
    cdl("cdl_hammer", "Hammer - Bullish", TREND, hammer),
    cdl("cdl_hanging_man", "Hanging Man - Bearish", TREND, hanging_man),
    cdl("cdl_harami_bear", "Harami - Bearish", TREND, harami_bear),
    cdl("cdl_harami_bull", "Harami - Bullish", TREND, harami_bull),
    cdl("cdl_harami_cross_bear", "Harami Cross - Bearish", TREND, harami_cross_bear),
    cdl("cdl_harami_cross_bull", "Harami Cross - Bullish", TREND, harami_cross_bull),
    cdl("cdl_inverted_hammer", "Inverted Hammer - Bullish", TREND, inverted_hammer),
    cdl("cdl_kicking_bear", "Kicking - Bearish", &[], kicking_bear),
    cdl("cdl_kicking_bull", "Kicking - Bullish", &[], kicking_bull),
    cdl("cdl_long_lower_shadow", "Long Lower Shadow - Bullish", &[], long_lower_shadow),
    cdl("cdl_long_upper_shadow", "Long Upper Shadow - Bearish", &[], long_upper_shadow),
    cdl("cdl_marubozu_black", "Marubozu Black - Bearish", &[], marubozu_black),
    cdl("cdl_marubozu_white", "Marubozu White - Bullish", &[], marubozu_white),
    cdl("cdl_on_neck", "On Neck - Bearish", TREND, on_neck),
    cdl("cdl_piercing", "Piercing - Bullish", TREND, piercing),
    cdl("cdl_rising_window", "Rising Window - Bullish", TREND, rising_window),
    cdl("cdl_shooting_star", "Shooting Star - Bearish", TREND, shooting_star),
    cdl("cdl_spinning_top_black", "Spinning Top Black", &[], spinning_top_black),
    cdl("cdl_spinning_top_white", "Spinning Top White", &[], spinning_top_white),
    cdl("cdl_tweezer_bottom", "Tweezer Bottom - Bullish", TREND, tweezer_bottom),
    cdl("cdl_tweezer_top", "Tweezer Top - Bearish", TREND, tweezer_top),
];
```
(`get`, `p` and `run` are used by Task 15's `cdl_all`; add `#[allow(unused_imports)]` on that `use` line until then, or import them in Task 15.)

`sources.rs` rows (one per id, TradingView name only):

```rust
    ("cdl_dark_cloud_cover", "Dark Cloud Cover - Bearish", ""), ("cdl_doji", "Doji", ""), ("cdl_doji_star_bear", "Doji Star - Bearish", ""),
    ("cdl_doji_star_bull", "Doji Star - Bullish", ""), ("cdl_dragonfly_doji", "Dragonfly Doji - Bullish", ""), ("cdl_engulfing_bear", "Engulfing - Bearish", ""),
    ("cdl_engulfing_bull", "Engulfing - Bullish", ""), ("cdl_falling_window", "Falling Window - Bearish", ""), ("cdl_gravestone_doji", "Gravestone Doji - Bearish", ""),
    ("cdl_hammer", "Hammer - Bullish", ""), ("cdl_hanging_man", "Hanging Man - Bearish", ""), ("cdl_harami_bear", "Harami - Bearish", ""),
    ("cdl_harami_bull", "Harami - Bullish", ""), ("cdl_harami_cross_bear", "Harami Cross - Bearish", ""), ("cdl_harami_cross_bull", "Harami Cross - Bullish", ""),
    ("cdl_inverted_hammer", "Inverted Hammer - Bullish", ""), ("cdl_kicking_bear", "Kicking - Bearish", ""), ("cdl_kicking_bull", "Kicking - Bullish", ""),
    ("cdl_long_lower_shadow", "Long Lower Shadow - Bullish", ""), ("cdl_long_upper_shadow", "Long Upper Shadow - Bearish", ""), ("cdl_marubozu_black", "Marubozu Black - Bearish", ""),
    ("cdl_marubozu_white", "Marubozu White - Bullish", ""), ("cdl_on_neck", "On Neck - Bearish", ""), ("cdl_piercing", "Piercing - Bullish", ""),
    ("cdl_rising_window", "Rising Window - Bullish", ""), ("cdl_shooting_star", "Shooting Star - Bearish", ""), ("cdl_spinning_top_black", "Spinning Top Black", ""),
    ("cdl_spinning_top_white", "Spinning Top White", ""), ("cdl_tweezer_bottom", "Tweezer Bottom - Bullish", ""), ("cdl_tweezer_top", "Tweezer Top - Bearish", ""),
```

Parity doc: one row per id (`cdl_* | trend=0 (where present) | signal`) and the note "export with a Pine script plotting `pattern ? 1 : 0` per pattern; TradingView draws labels, not a series".

- [ ] **Step 4: Register and run** `cargo test lab::` → PASS (generic tests: every pattern 0/1 from bar 0, causal, no inf); guard clean.
- [ ] **Step 5: Commit** `git commit -m "Add lab candlestick pattern framework and one/two-bar TradingView patterns"`

### Task 15: Candlestick patterns II — three-to-five-bar patterns and `cdl_all`

**Files:** Modify `lab/candles.rs`, `sources.rs`, parity doc.

**Interfaces:** Consumes Task 14 `F`, `feats`, `pattern!`, `cdl`, `TREND`, the test helpers `case`/`last`. Produces 14 ids and `cdl_all` (outputs bullish, bearish, neutral = number of patterns completing on the bar, by TradingView's classification).

- [ ] **Step 1: Write the failing tests** (append to the `candles.rs` tests)

```rust
    const CASES_II: &[(&str, &[(f64, f64, f64, f64)])] = &[
        ("cdl_abandoned_baby_bear", &[(10.0, 13.2, 9.9, 13.0), (13.5, 13.6, 13.4, 13.5), (13.0, 13.1, 10.0, 10.5)]),
        ("cdl_abandoned_baby_bull", &[(13.0, 13.1, 9.9, 10.0), (9.5, 9.6, 9.4, 9.5), (10.0, 13.0, 9.9, 12.5)]),
        ("cdl_downside_tasuki_gap", &[(13.0, 13.1, 10.9, 11.0), (10.5, 10.6, 9.9, 10.0), (10.2, 10.8, 10.1, 10.7)]),
        ("cdl_evening_doji_star", &[(10.0, 13.1, 9.9, 13.0), (13.5, 13.7, 13.3, 13.5), (13.2, 13.25, 11.2, 11.4)]),
        ("cdl_evening_star", &[(10.0, 13.1, 9.9, 13.0), (13.4, 13.8, 13.3, 13.6), (13.2, 13.25, 11.2, 11.4)]),
        ("cdl_falling_three_methods", &[(14.0, 14.1, 11.0, 11.2), (11.3, 11.9, 11.2, 11.8), (11.8, 12.4, 11.7, 12.3), (12.3, 12.9, 12.2, 12.8), (12.8, 12.9, 10.4, 10.5)]),
        ("cdl_morning_doji_star", &[(13.0, 13.1, 9.9, 10.0), (9.5, 9.7, 9.3, 9.5), (9.8, 12.0, 9.75, 11.9)]),
        ("cdl_morning_star", &[(13.0, 13.1, 9.9, 10.0), (9.6, 9.8, 9.2, 9.4), (9.8, 12.0, 9.75, 11.9)]),
        ("cdl_rising_three_methods", &[(11.0, 14.1, 10.9, 14.0), (13.9, 14.0, 13.2, 13.3), (13.3, 13.4, 12.6, 12.7), (12.7, 12.8, 12.1, 12.2), (12.3, 15.1, 12.2, 15.0)]),
        ("cdl_three_black_crows", &[(14.0, 14.01, 11.99, 12.0), (12.5, 12.51, 10.49, 10.5), (11.0, 11.01, 8.99, 9.0)]),
        ("cdl_three_white_soldiers", &[(9.0, 11.01, 8.99, 11.0), (10.5, 12.51, 10.49, 12.5), (12.0, 14.01, 11.99, 14.0)]),
        ("cdl_tri_star_bear", &[(11.0, 11.6, 10.4, 11.01), (12.0, 12.6, 11.4, 12.01), (11.0, 11.6, 10.4, 11.01)]),
        ("cdl_tri_star_bull", &[(11.0, 11.6, 10.4, 11.01), (10.0, 10.6, 9.4, 10.01), (11.0, 11.6, 10.4, 11.01)]),
        ("cdl_upside_tasuki_gap", &[(10.0, 12.1, 9.9, 12.0), (12.5, 13.1, 12.4, 13.0), (12.8, 12.9, 12.2, 12.3)]),
    ];
    #[test] fn each_multi_bar_pattern_fires_on_its_case() { for (id, rows) in CASES_II { assert_eq!(last(id, rows), 1., "{id}"); } }
    #[test] fn all_patterns_counts_by_kind() {
        let count = |rows: &[(f64, f64, f64, f64)]| { let o = go("cdl_all", &case(rows), &[("trend", 2.)]); (o[0][o[0].len() - 1], o[1][o[1].len() - 1], o[2][o[2].len() - 1]) };
        assert_eq!(count(CASES_II[0].1), (0., 4., 0.)); // abandoned baby bear + evening doji star + evening star + falling window
        assert_eq!(count(&[(11., 11.1, 9.6, 11.1)]), (2., 1., 0.)); // hammer + long lower shadow; hanging man (no trend rule)
        assert_eq!(count(&[(11., 11.6, 10.4, 11.02)]), (0., 0., 1.)); // doji
    }
```

- [ ] **Step 2: Run** `CARGO_BUILD_JOBS=2 cargo test lab::candles` → FAIL.

- [ ] **Step 3: Implement**

```rust
pattern!(abandoned_baby_bear, trend, |f, b, i| i >= 2 && f.up[i - 2] && f.white[i - 2] && f.dojib[i - 1] && b.high[i - 2] < b.low[i - 1] && f.black[i] && b.low[i - 1] > b.high[i]);
pattern!(abandoned_baby_bull, trend, |f, b, i| i >= 2 && f.dn[i - 2] && f.black[i - 2] && f.dojib[i - 1] && b.low[i - 2] > b.high[i - 1] && f.white[i] && b.high[i - 1] < b.low[i]);
pattern!(downside_tasuki_gap, trend, |f, b, i| i >= 2 && f.long[i - 2] && f.small[i - 1] && f.dn[i] && f.black[i - 2] && f.bh[i - 1] < f.bl[i - 2] && f.black[i - 1] && f.white[i] && f.bh[i] <= f.bl[i - 2] && f.bh[i] >= f.bh[i - 1]);
pattern!(upside_tasuki_gap, trend, |f, b, i| i >= 2 && f.long[i - 2] && f.small[i - 1] && f.up[i] && f.white[i - 2] && f.bl[i - 1] > f.bh[i - 2] && f.white[i - 1] && f.black[i] && f.bl[i] >= f.bh[i - 2] && f.bl[i] <= f.bl[i - 1]);
pattern!(evening_doji_star, trend, |f, b, i| i >= 2 && f.long[i - 2] && f.dojib[i - 1] && f.long[i] && f.up[i] && f.white[i - 2] && f.bl[i - 1] > f.bh[i - 2] && f.black[i] && f.bl[i] <= f.mid[i - 2] && f.bl[i] > f.bl[i - 2] && f.bl[i - 1] > f.bh[i]);
pattern!(evening_star, trend, |f, b, i| i >= 2 && f.long[i - 2] && f.small[i - 1] && f.long[i] && f.up[i] && f.white[i - 2] && f.bl[i - 1] > f.bh[i - 2] && f.black[i] && f.bl[i] <= f.mid[i - 2] && f.bl[i] > f.bl[i - 2] && f.bl[i - 1] > f.bh[i]);
pattern!(morning_doji_star, trend, |f, b, i| i >= 2 && f.long[i - 2] && f.dojib[i - 1] && f.long[i] && f.dn[i] && f.black[i - 2] && f.bh[i - 1] < f.bl[i - 2] && f.white[i] && f.bh[i] >= f.mid[i - 2] && f.bh[i] < f.bh[i - 2] && f.bh[i - 1] < f.bl[i]);
pattern!(morning_star, trend, |f, b, i| i >= 2 && f.long[i - 2] && f.small[i - 1] && f.long[i] && f.dn[i] && f.black[i - 2] && f.bh[i - 1] < f.bl[i - 2] && f.white[i] && f.bh[i] >= f.mid[i - 2] && f.bh[i] < f.bh[i - 2] && f.bh[i - 1] < f.bl[i]);
pattern!(falling_three_methods, trend, |f, b, i| i >= 4 && f.dn[i - 4] && f.long[i - 4] && f.black[i - 4]
    && (1..=3).all(|j| f.small[i - j] && f.white[i - j] && b.open[i - j] > b.low[i - 4] && b.close[i - j] < b.high[i - 4]) && f.long[i] && f.black[i] && b.close[i] < b.close[i - 4]);
pattern!(rising_three_methods, trend, |f, b, i| i >= 4 && f.up[i - 4] && f.long[i - 4] && f.white[i - 4]
    && (1..=3).all(|j| f.small[i - j] && f.black[i - j] && b.open[i - j] < b.high[i - 4] && b.close[i - j] > b.low[i - 4]) && f.long[i] && f.white[i] && b.close[i] > b.close[i - 4]);
pattern!(three_black_crows, |f, b, i| i >= 2 && (0..3).all(|j| f.long[i - j] && f.black[i - j] && f.rng[i - j] * 0.05 > f.ds[i - j])
    && b.close[i] < b.close[i - 1] && b.close[i - 1] < b.close[i - 2] && b.open[i] > b.close[i - 1] && b.open[i] < b.open[i - 1] && b.open[i - 1] > b.close[i - 2] && b.open[i - 1] < b.open[i - 2]);
pattern!(three_white_soldiers, |f, b, i| i >= 2 && (0..3).all(|j| f.long[i - j] && f.white[i - j] && f.rng[i - j] * 0.05 > f.us[i - j])
    && b.close[i] > b.close[i - 1] && b.close[i - 1] > b.close[i - 2] && b.open[i] < b.close[i - 1] && b.open[i] > b.open[i - 1] && b.open[i - 1] < b.close[i - 2] && b.open[i - 1] > b.open[i - 2]);
pattern!(tri_star_bear, trend, |f, b, i| i >= 2 && f.doji[i - 2] && f.doji[i - 1] && f.doji[i] && f.up[i - 2] && f.bh[i - 2] < f.bl[i - 1] && f.bl[i - 1] > f.bh[i]);
pattern!(tri_star_bull, trend, |f, b, i| i >= 2 && f.doji[i - 2] && f.doji[i - 1] && f.doji[i] && f.dn[i - 2] && f.bl[i - 2] > f.bh[i - 1] && f.bh[i - 1] < f.bl[i]);

/// TradingView classification used by "All Candlestick Patterns": +1 bullish, −1 bearish, 0 neutral.
const KIND: &[(&str, i8)] = &[
    ("cdl_abandoned_baby_bull", 1), ("cdl_doji_star_bull", 1), ("cdl_dragonfly_doji", 1), ("cdl_engulfing_bull", 1), ("cdl_hammer", 1), ("cdl_harami_bull", 1),
    ("cdl_harami_cross_bull", 1), ("cdl_inverted_hammer", 1), ("cdl_kicking_bull", 1), ("cdl_long_lower_shadow", 1), ("cdl_marubozu_white", 1), ("cdl_morning_doji_star", 1),
    ("cdl_morning_star", 1), ("cdl_piercing", 1), ("cdl_rising_three_methods", 1), ("cdl_rising_window", 1), ("cdl_three_white_soldiers", 1), ("cdl_tri_star_bull", 1),
    ("cdl_tweezer_bottom", 1), ("cdl_upside_tasuki_gap", 1),
    ("cdl_abandoned_baby_bear", -1), ("cdl_dark_cloud_cover", -1), ("cdl_doji_star_bear", -1), ("cdl_downside_tasuki_gap", -1), ("cdl_engulfing_bear", -1),
    ("cdl_evening_doji_star", -1), ("cdl_evening_star", -1), ("cdl_falling_three_methods", -1), ("cdl_falling_window", -1), ("cdl_gravestone_doji", -1),
    ("cdl_hanging_man", -1), ("cdl_harami_bear", -1), ("cdl_harami_cross_bear", -1), ("cdl_kicking_bear", -1), ("cdl_long_upper_shadow", -1),
    ("cdl_marubozu_black", -1), ("cdl_on_neck", -1), ("cdl_shooting_star", -1), ("cdl_three_black_crows", -1), ("cdl_tri_star_bear", -1), ("cdl_tweezer_top", -1),
    ("cdl_doji", 0), ("cdl_spinning_top_black", 0), ("cdl_spinning_top_white", 0),
];
fn all_patterns(b: &Bars, prm: &Params) -> Output {
    let (tp, none): (Params, Params) = ([("trend".to_string(), p(prm, "trend"))].into_iter().collect(), Params::new());
    let mut out = vec![vec![0.; b.len()]; 3];
    for (id, kind) in KIND {
        let d = get(id).expect("pattern id is in the catalogue");
        let s = run(&d, b, if d.params.is_empty() { &none } else { &tp })?.remove(0);
        let k = match kind { 1 => 0, -1 => 1, _ => 2 };
        for i in 0..b.len() { out[k][i] += s[i]; }
    }
    Ok(out)
}
```
DEFS rows (14 `cdl(…)` rows, TREND except the two three-bar soldier/crow patterns, which take `&[]`), plus

```rust
    IndicatorDef { id: "cdl_all", name: "All Candlestick Patterns", category: CAT, params: TREND, outputs: &["bullish", "bearish", "neutral"],
        placement: Placement::Pane, needs: Needs::NONE, definition: "number of the 44 TradingView patterns completing on this bar, by kind (bullish, bearish, neutral)",
        first: &[0, 0, 0], sparse: false, compute: Some(all_patterns), note: "" },
```
The 14 names: Abandoned Baby - Bearish/Bullish, Downside Tasuki Gap - Bearish, Upside Tasuki Gap - Bullish, Evening Doji Star - Bearish, Evening Star - Bearish, Morning Doji Star - Bullish, Morning Star - Bullish, Falling Three Methods - Bearish, Rising Three Methods - Bullish, Three Black Crows - Bearish, Three White Soldiers - Bullish, Tri Star - Bearish, Tri Star - Bullish (ids `cdl_` + snake case as in `KIND`).

`sources.rs` rows: the same 14 `("cdl_…", "<name>", "")` and `("cdl_all", "All Candlestick Patterns", "")`.

- [ ] **Step 4: Run** `cargo test lab::` → PASS; guard clean.
- [ ] **Step 5: Commit** `git commit -m "Add lab multi-bar candlestick patterns and the all-patterns count"`

### Task 16: Technical Ratings — `ratings.rs`

**Files:** Create `lab/ratings.rs`; modify `lab/mod.rs`, `catalogue.rs` (`all`), `sources.rs`, parity doc.

**Interfaces:** Consumes the catalogue through `catalogue::{get, run}` (sma, ema, hma, vwma, ichimoku, rsi, stoch, cci, dmi, ao, mom, macd, stochrsi, willr, uo, and Task 7 `bbp`). Produces id `tech_ratings`.

| id | name | placement | params | outputs | first | definition |
|---|---|---|---|---|---|---|
| tech_ratings | Technical Ratings | Pane | — | ma, osc, all | [199, 49, 199] | "TradingView Technical Ratings: each component votes +1/−1/0; ma = mean of SMA & EMA 10/20/30/50/100/200, HMA 9, VWMA 20 (dropped without volume) and Ichimoku (as plotted); osc = mean of RSI 14, Stoch 14/3/3, CCI 20, ADX 14/14, AO, Momentum 10, MACD 12/26/9, Stoch RSI 3/3/14/14, Williams %R 14, Bull Bear Power 13, UO 7/14/28 (rules per TradingView help); trend = close vs sma(close, 50) (declared); a group is NaN until all its components exist; all = (ma + osc)/2" |

Category "Ratings", needs NONE.

- [ ] **Step 1: Write the failing tests**

```rust
#[cfg(test)]
mod tests {
    use crate::lab::testdata::{close_eq, compiles, from_ohlc, go};
    fn series(growth: f64, volume: bool) -> crate::lab::bars::Bars {
        let mut b = from_ohlc(&(0..300).map(|i| { let c = 100. * growth.powi(i); (c, c * 1.005, c * 0.995, c) }).collect::<Vec<_>>());
        if volume { b.volume = Some(vec![1000.; 300]); }
        b
    }
    #[test] fn moving_average_rating_on_steady_trends() {
        for volume in [true, false] {
            let up = go("tech_ratings", &series(1.01, volume), &[]);
            assert!(up[0][198].is_nan() && !up[0][199].is_nan());
            assert_eq!(up[0][299], 1.0, "every MA below price in a steady rise (volume {volume})");
        }
        let down = go("tech_ratings", &series(0.99, true), &[]);
        assert!((down[0][299] + 13. / 15.).abs() < 1e-12, "HMA overshoots in a steady fall: 13 sells, 1 buy of 15 → {}", down[0][299]);
    }
    #[test] fn overall_is_the_mean_of_the_groups() {
        let o = go("tech_ratings", &crate::lab::testdata::walk(400, 3, true), &[]);
        for i in 199..400 { assert!(close_eq(o[2][i], (o[0][i] + o[1][i]) / 2.) && !(o[1][i].abs() > 1.), "bar {i}"); }
    }
    #[test] fn rules_reach_ratings() { compiles("tech_ratings", "all").unwrap(); }
}
```

- [ ] **Step 2: Run** `CARGO_BUILD_JOBS=2 cargo test lab::ratings` → FAIL.

- [ ] **Step 3: Implement**

```rust
//! TradingView Technical Ratings from lab catalogue components (rules: TradingView help 43000614331).
use super::bars::Bars;
use super::catalogue::{get, run, IndicatorDef, Needs, Output, Params, Placement};
use super::ta::NA;

fn vote(buy: bool, sell: bool, inputs: &[f64]) -> f64 { if inputs.iter().any(|v| v.is_nan()) { NA } else if buy { 1. } else if sell { -1. } else { 0. } }
fn mean(v: &[f64]) -> f64 { if v.iter().any(|x| x.is_nan()) { NA } else { v.iter().sum::<f64>() / v.len() as f64 } }
fn tech_ratings(b: &Bars, _: &Params) -> Output {
    let r = |id: &str, kv: &[(&str, f64)]| run(&get(id).expect("component in catalogue"), b, &kv.iter().map(|(k, v)| (k.to_string(), *v)).collect());
    let c = &b.close;
    let mut mas: Vec<Vec<f64>> = Vec::new();
    for k in [10., 20., 30., 50., 100., 200.] { mas.push(r("sma", &[("length", k)])?.remove(0)); mas.push(r("ema", &[("length", k)])?.remove(0)); }
    mas.push(r("hma", &[("length", 9.)])?.remove(0));
    if b.volume.is_some() { mas.push(r("vwma", &[("length", 20.)])?.remove(0)); }
    let ich = r("ichimoku", &[])?;
    let trend = r("sma", &[("length", 50.)])?.remove(0);
    let rsi = r("rsi", &[("length", 14.)])?.remove(0);
    let st = r("stoch", &[("k", 14.), ("smooth_k", 3.), ("d", 3.)])?;
    let cci = r("cci", &[("length", 20.)])?.remove(0);
    let dmi = r("dmi", &[])?;
    let ao = r("ao", &[])?.remove(0);
    let mom = r("mom", &[("length", 10.)])?.remove(0);
    let macd = r("macd", &[])?;
    let srsi = r("stochrsi", &[])?;
    let wr = r("willr", &[("length", 14.)])?.remove(0);
    let bbp = r("bbp", &[("length", 13.)])?;
    let uo = r("uo", &[])?.remove(0);
    let back = |v: &[f64], i: usize, k: usize| if i >= k { v[i - k] } else { NA };
    let len = b.len();
    let (mut ma, mut osc, mut all) = (vec![NA; len], vec![NA; len], vec![NA; len]);
    for i in 0..len {
        let mut m: Vec<f64> = mas.iter().map(|s| vote(s[i] < c[i], s[i] > c[i], &[s[i]])).collect();
        let (t, k, sa, sb) = (ich[0][i], ich[1][i], ich[2][i], ich[3][i]);
        m.push(vote(sa > sb && k > sa && t > k && c[i] > t, sa < sb && k < sa && t < k && c[i] < t, &[t, k, sa, sb]));
        let (up, dn) = (c[i] > trend[i], c[i] < trend[i]);
        let (r1, c1, a1, m1, w1) = (back(&rsi, i, 1), back(&cci, i, 1), back(&dmi[2], i, 1), back(&mom, i, 1), back(&wr, i, 1));
        let (ao1, ao2, bull1, bear1) = (back(&ao, i, 1), back(&ao, i, 2), back(&bbp[1], i, 1), back(&bbp[2], i, 1));
        let o = [
            vote(rsi[i] < 30. && rsi[i] > r1, rsi[i] > 70. && rsi[i] < r1, &[rsi[i], r1]),
            vote(st[0][i] < 20. && st[1][i] < 20. && st[0][i] > st[1][i], st[0][i] > 80. && st[1][i] > 80. && st[0][i] < st[1][i], &[st[0][i], st[1][i]]),
            vote(cci[i] < -100. && cci[i] > c1, cci[i] > 100. && cci[i] < c1, &[cci[i], c1]),
            vote(dmi[0][i] > dmi[1][i] && dmi[2][i] > 20. && dmi[2][i] > a1, dmi[0][i] < dmi[1][i] && dmi[2][i] > 20. && dmi[2][i] < a1, &[dmi[0][i], dmi[1][i], dmi[2][i], a1]),
            vote((ao[i] > 0. && ao1 <= 0.) || (ao[i] > 0. && ao1 > 0. && ao[i] > ao1 && ao1 < ao2),
                 (ao[i] < 0. && ao1 >= 0.) || (ao[i] < 0. && ao1 < 0. && ao[i] < ao1 && ao1 > ao2), &[ao[i], ao1, ao2]),
            vote(mom[i] > m1, mom[i] < m1, &[mom[i], m1]),
            vote(macd[0][i] > macd[1][i], macd[0][i] < macd[1][i], &[macd[0][i], macd[1][i]]),
            vote(dn && srsi[0][i] < 20. && srsi[1][i] < 20. && srsi[0][i] > srsi[1][i], up && srsi[0][i] > 80. && srsi[1][i] > 80. && srsi[0][i] < srsi[1][i], &[srsi[0][i], srsi[1][i], trend[i]]),
            vote(wr[i] < -80. && wr[i] > w1, wr[i] > -20. && wr[i] < w1, &[wr[i], w1]),
            vote(up && bbp[2][i] < 0. && bbp[2][i] > bear1, dn && bbp[1][i] > 0. && bbp[1][i] < bull1, &[bbp[1][i], bbp[2][i], bull1, bear1, trend[i]]),
            vote(uo[i] > 70., uo[i] < 30., &[uo[i]]),
        ];
        ma[i] = mean(&m); osc[i] = mean(&o); all[i] = (ma[i] + osc[i]) / 2.;
    }
    Ok(vec![ma, osc, all])
}
pub const DEFS: &[IndicatorDef] = &[IndicatorDef { id: "tech_ratings", name: "Technical Ratings", category: "Ratings", params: &[], outputs: &["ma", "osc", "all"],
    placement: Placement::Pane, needs: Needs::NONE, definition: "TradingView Technical Ratings: each component votes +1/−1/0; ma = mean of SMA & EMA 10/20/30/50/100/200, HMA 9, VWMA 20 (dropped without volume) and Ichimoku (as plotted); osc = mean of RSI 14, Stoch 14/3/3, CCI 20, ADX 14/14, AO, Momentum 10, MACD 12/26/9, Stoch RSI 3/3/14/14, Williams %R 14, Bull Bear Power 13, UO 7/14/28 (TradingView help rules); trend = close vs sma(close, 50) (declared); a group is NaN until all its components exist; all = (ma + osc)/2", first: &[199, 49, 199], sparse: false, compute: Some(tech_ratings), note: "" }];
```

`sources.rs` row: `("tech_ratings", "Technical Ratings", ""),`.

- [ ] **Step 4: Register and run** `cargo test lab::` → PASS; guard clean.
- [ ] **Step 5: Commit** `git commit -m "Add lab TradingView technical ratings"`

### Task 17: AmiBroker variants on Batch A entries — `stoch`, `trix`

**Files:** Modify `lab/momentum.rs` (params, `stoch`, `trix`, tests), parity doc (stoch/trix rows gain `variant=0`).

**Interfaces:** `stoch` and `trix` gain the last parameter `variant` (`int`, default 0, range 0–1). Default results are unchanged. Rule labels (`series::describe`) now show `variant=0` among the parameters, e.g. `stoch(d=3,k=14,smooth_k=1,variant=0)`. That is cosmetic, but it changes exit-reason text for new runs. Saved specs keep working because a missing `variant` resolves to 0.

| id | variant 1 (AmiBroker) definition appended to the definition string |
|---|---|
| stoch | "variant 1 (AmiBroker StochK/StochD, standard ratio-of-sums form; unverified): k = 100·sum(close − lowest(low, k), smooth_k)/sum(highest(high, k) − lowest(low, k), smooth_k); d = sma(k, d)" |
| trix | "variant 1 (AmiBroker TRIX, standard form; unverified): 100·(e − e[1])/e[1], e = ema(ema(ema(close, n), n), n) (no log)" |

- [ ] **Step 1: Write the failing tests** (in `momentum.rs` tests; import `crate::lab::testdata::{d1, go, walk}`)

```rust
    #[test] fn stoch_variant_ab_is_ratio_of_sums() {
        let o = go("stoch", &d1(), &[("k", 3.), ("smooth_k", 2.), ("d", 2.), ("variant", 1.)]);
        assert_series(&o[0], &[N, N, N, 66.666666666667, 24.0, 28.846153846154, 71.153846153846, 91.666666666667]);
        assert_series(&o[1], &[N, N, N, N, 45.333333333333, 26.423076923077, 50.0, 81.410256410256]);
    }
    #[test] fn trix_variant_ab_is_percent_roc() {
        assert_series(&go("trix", &d1(), &[("length", 2.), ("variant", 1.)])[0], &[N, N, N, N, -3.045537340619, -1.317602264472, 2.716080720904, 4.83106113651]);
    }
    #[test] fn variant_zero_is_unchanged() {
        let b = walk(300, 5, true);
        for id in ["stoch", "trix"] {
            let (a, z) = (go(id, &b, &[]), go(id, &b, &[("variant", 0.)]));
            for (x, y) in a.iter().zip(&z) { assert_series(x, y); }
        }
    }
```
The existing `stoch_k_and_d` test must keep passing unchanged.

- [ ] **Step 2: Run** `CARGO_BUILD_JOBS=2 cargo test lab::momentum` → FAIL (unknown parameter `variant`).

- [ ] **Step 3: Implement**

```rust
fn stoch(b: &Bars, prm: &Params) -> Output {
    let k = n(prm, "k");
    let (hh, ll) = (ta::highest(&b.high, k), ta::lowest(&b.low, k));
    let kk = if p(prm, "variant") == 1. {
        let sk = n(prm, "smooth_k");
        zip(&ta::sum(&zip(&b.close, &ll, |c, l| c - l), sk), &ta::sum(&zip(&hh, &ll, |h, l| h - l), sk), |a, d| div(100. * a, d))
    } else { ta::sma(&stoch_s(&b.close, &hh, &ll), n(prm, "smooth_k")) };
    let d = ta::sma(&kk, n(prm, "d"));
    Ok(vec![kk, d])
}
fn trix(b: &Bars, prm: &Params) -> Output {
    let l = n(prm, "length");
    if p(prm, "variant") == 1. {
        let e = ta::ema(&ta::ema(&ta::ema(&b.close, l), l), l);
        return Ok(vec![(0..e.len()).map(|i| if i == 0 { NA } else { div(100. * (e[i] - e[i - 1]), e[i - 1]) }).collect()]);
    }
    let ln: Vec<f64> = b.close.iter().map(|&c| if c > 0. { c.ln() } else { NA }).collect();
    let e = ta::ema(&ta::ema(&ta::ema(&ln, l), l), l);
    Ok(vec![ta::change(&e, 1).iter().map(|d| 10000. * d).collect()])
}
```
Add `int("variant", "Variant (0 TradingView, 1 AmiBroker ratio of sums)", 0., 0., 1.)` as the last `stoch` param and `int("variant", "Variant (0 TradingView log ×10000, 1 AmiBroker percent ROC)", 0., 0., 1.)` as the last `trix` param (import `int` and `p`). Append the variant text above to each definition string.

- [ ] **Step 4: Run** `cargo test lab::` → PASS (all generic tests, including `every_indicator_at_parameter_bounds_never_panics`, which now runs variant 0 and 1); guard clean.
- [ ] **Step 5: Commit** `git commit -m "Add AmiBroker variants to lab Stochastic and TRIX"`

### Task 18: Phase gate

**Files:** Modify `lab/catalogue.rs` (master-list test). No other code changes, except fixes the gate finds.

- [ ] **Step 1: Pin the master list**

```rust
    #[test] fn catalogue_matches_the_phase6_master_list() {
        let defs = all();
        assert_eq!(defs.len(), 299, "Phase 6 master list: 66 Batch A + 233 new");
        assert_eq!(defs.iter().filter(|d| d.compute.is_some()).count(), 183);
        assert_eq!(defs.iter().filter(|d| d.compute.is_none()).count(), 116);
        let mut c = std::collections::BTreeMap::new();
        for (_, tv, ab) in crate::lab::sources::SOURCES {
            *c.entry(match (tv.is_empty(), ab.is_empty()) { (false, false) => "both", (false, true) => "tv", (true, false) => "ab", _ => "lab" }).or_insert(0) += 1;
        }
        assert_eq!((c["tv"], c["ab"], c["both"], c["lab"]), (234, 23, 41, 1));
    }
```
Run `CARGO_BUILD_JOBS=2 cargo test lab::catalogue` → PASS. If a count differs, compare `catalogue::all()` with the Master list table, fix the missing or extra entry, and never edit the numbers to match.

- [ ] **Step 2: Engine suite.** `cd Context/guru/backend && VAULT_IDX="/run/user/1000/gvfs/smb-share:server=192.168.4.50,share=share/Manan Pujara/Book-to-rust/idx_data.csv" CARGO_BUILD_JOBS=2 cargo test`. Expect all to pass. Record the test count and the count at `P6_BASE` (the commit before Task 1; find it with `git log --oneline` as the parent of Task 1's commit).
- [ ] **Step 3: Scripts.** `python3 -m unittest discover -s scripts -p 'test_*.py'` and `python3 scripts/check_upload.py` → both pass.
- [ ] **Step 4: Book guard with engine compare.** `NAS="/run/user/1000/gvfs/smb-share:server=192.168.4.50,share=share/Manan Pujara/Book-to-rust"; python3 scripts/check_book_untouched.py --base origin/Manan --engine-compare --csv "$NAS/idx_data.csv" --vix "$NAS/india_vix" --extra "$NAS/extra_data"` → `"problems": []`. Copy the `notes` (hash count, basket note) into the report.
- [ ] **Step 5: Release build without lab warnings.** `cd Context/guru/backend && touch src/main.rs && CARGO_BUILD_JOBS=2 cargo build --release 2>&1 | grep -A3 '^warning' | grep -- '--> src/lab/'` → no output.
- [ ] **Step 6: Real-data smoke (bounded, read-only NAS, local port).**

```bash
cd Context/guru/backend
NAS="/run/user/1000/gvfs/smb-share:server=192.168.4.50,share=share/Manan Pujara/Book-to-rust"
PORT=$(python3 -c 'import socket;s=socket.socket();s.bind(("127.0.0.1",0));print(s.getsockname()[1])')
./target/release/guru "$NAS/idx_data.csv" "$PORT" "$NAS/india_vix" "$NAS/extra_data/india" >/dev/null 2>&1 & PID=$!
until curl -s -o /dev/null "http://127.0.0.1:$PORT/api/gurus"; do sleep 0.5; done
A="http://127.0.0.1:$PORT/api/lab"
curl -s "$A/catalogue" | python3 -c 'import sys,json;d=json.load(sys.stdin);i=d["indicators"];print(len(i),sum(x["computed"] for x in i),sum(not x["computed"] for x in i))'
curl -s "$A/series?ticker=NIFTY%2050&id=adl" | python3 -c 'import sys,json;d=json.load(sys.stdin);s=d["series"]["adl"];print(next(d["dates"][k] for k,v in enumerate(s) if v is not None), s[-1])'
curl -s -w ' %{http_code}\n' "$A/series?ticker=NIFTY%2050&id=adl&tf=W"
curl -s -w ' %{http_code}\n' "$A/series?ticker=NIFTY%2050&id=correlation"
curl -s -w ' %{http_code}\n' "$A/series?ticker=NIFTY%2050&id=zigzag"
curl -s "$A/series?ticker=NIFTY%2050&id=tech_ratings" | python3 -c 'import sys,json;d=json.load(sys.stdin);print(d["series"]["all"][-1])'
kill $PID
```
Expected:
- `299 183 116`;
- the A/D line's first date is in 1999 (the start of NSE_ADV_DEC) and its last value is a number;
- the weekly A/D line returns 400 `needs_data` "…needs market breadth…";
- `correlation` returns 400 `needs_data` "…not available yet — needs a second instrument…";
- `zigzag` returns 400 `unsupported` "…looks ahead — chart display only…";
- the last overall technical rating is a number in [−1, 1].

Repeat the first two curls with the US data (`"$NAS/extra_data/us"` and `ticker=SPX`) when US data is in `idx_data.csv`; the A/D line should start in 1965 or at the first SPX bar.
- [ ] **Step 7: Scope proof.**
  - `git diff P6_BASE --stat` lists only `Context/guru/backend/src/lab/*`, `docs/indicator-parity-fixtures.md` and (if not committed earlier) this plan.
  - `git diff P6_BASE -- Context/guru/backend/src/lab/api.rs` shows only the `catalogue_json` format hunk (Task 1) and the `Bars::from_ctx` substitutions (Task 2).
  - `git diff P6_BASE -U0 -- Context/guru/backend/src/main.rs Context/guru/webapp` → no output.
  - `git diff origin/Manan --stat -- Context/guru/backend/src/connors.rs Context/guru/backend/src/books` → no output.
- [ ] **Step 8: Report.** Commit any gate fixes (`git commit -m "Fix Phase 6 gate findings"`). Then draft the PR description: test counts before and after, the master-list counts (299 = 64 + 2 existing, 119 A, 4 B, 110 C; TradingView 234 / AmiBroker 23 / both 41 / lab 1), guard notes, smoke output, Decisions 1–24, and the open items:
  - parity fixtures pending for every new indicator;
  - Rob Booker formulas awaiting a source review;
  - AmiBroker variant definitions unverified;
  - second-instrument (B) follow-up after Phase 5;
  - session VWAP/intraday still listed.

  Do not push; the user pushes.
