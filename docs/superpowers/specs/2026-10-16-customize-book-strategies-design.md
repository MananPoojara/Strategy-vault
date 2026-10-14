# Customize book strategies (Indicators lab) — design

Status: draft for the user's morning review · Branch: `Nandini` (from `Manan`) · Date: 2026-10-16 (written 2026-10-14)
Parent spec: `docs/superpowers/specs/2026-10-14-indicators-lab-design.md` (§1 out of scope "editing book strategies (Phase 2 'Customize')", §4 reserved fields).
Runs after: Phase 5 (`engine-gl`) and Phase 6 (remaining TradingView indicators). Frontend parts are planned for the later frontend phase.

## 1. Goal

From an admitted book method, a researcher presses **Customize** and gets a **user variant**: a lab
strategy pre-filled with a *book template* — a declared adaptation of that method's rules into the lab
rule spec. They can then change it (add a stop-loss, change a threshold, flip a filter, optimize), save it
to their account, and submit it to YT Strategies like any lab strategy.

Customize **never edits a book method**. The book's code, catalogue, readiness, gate verdicts, published
figures and results stay byte-identical. A variant is a lab strategy: it is labelled
"Customized from <book> · <method> · user variant · simulated", never shown as a book result, and never
carries a book claim or gate verdict.

A **fidelity check** (display only) shows how closely the *unmodified* template reproduces the book
engine's trades for the same instrument and period: "template reproduces the book engine's trades: N/N"
or the list of differences. It is not a verification of anything.

What exists today (read for this design, Nandini at `d165472` plus Phase 5's `10f766e`): the lab engine
(`backend/src/lab/`: strict JSON reader, rule spec v1, TradingView-definition indicators, long-only
next-open simulator, `/api/lab/{catalogue,series,run,optimize}`), saved lab strategies with review and
frozen YT snapshots (`webapp/auth/src/lab-strategies.mjs`, `db/002-lab-strategies.sql`), and a book
catalogue of **4,365 methods in 364 books** (Connors, Chande, Ross gurus), of which **164 are admitted**
(catalogue status `live`), 661 `needs_data` and 3,540 `not_testable`.

## 2. Constraints

Carried from `AGENTS.md` and the parent spec §2/§2a; this phase restates the ones it touches.

- **Book results are never touched.** No edit to `connors.rs`, `books/**`, `readiness.rs`, `gate.rs`,
  `instrument_policy.rs`, `baskets.rs`, `accounting.rs`, `trade_validation.rs`, frontend `compat/*`,
  `ResearchPages.tsx`. `main.rs`, gateway `policy.rs` and `custom-pages.ts` additive only. **This phase
  does not change `main.rs` at all**: `/api/lab/*` is already dispatched to `lab::api` by prefix. Book
  functions are only *called* (catalogue, status inputs, instrument policy, readiness, `connors::run`).
  `scripts/check_book_untouched.py --engine-compare` must pass at the phase gate.
- **Keep four things separate** (AGENTS.md): *verified source rules* (the book catalogue's rule text and
  admission, untouched), *declared adaptations* (templates and their notes), *simulated results* (lab
  runs of templates and variants) and *book-reported figures* (catalogue `claim`, never shown with a
  template or variant). A template existing is not a verification; the fidelity check is not one either.
- **No result where rules or data are unresolved.** Templates exist only for methods the vault admits
  (`live`). A method that is `needs_data` or `not_testable` gets "Customize not available — <reason>".
- **Stock methods need real stocks.** A variant keeps its book method's instrument admission
  (`instrument_policy::policy(def).allowed_types`, read-only): a stock method's variant runs on actual
  catalogued stocks only; an index never stands in; ETF methods need ETF prices.
- **Provisional markets never publishable.** Variants on `gl`/`crypto` (if the method's instrument types
  admit them at all) can be saved for research but never submitted (existing Phase 4 rule, unchanged).
- Branch `Nandini`; never `main`; never push (the user pushes); no worktrees; no Co-Authored-By trailer;
  bounded build concurrency (`CARGO_BUILD_JOBS=2`); no data, `.env` or results in Git; no whole-corpus
  backtest at startup (templates and fidelity are computed per request, one instrument).

## 3. Architecture

```
Book method page (later frontend)  ──"Customize"──►  Indicators page, rule builder pre-filled
   │ GET /api/{in|us}/lab/template?method=<id>        (template spec incl. origin, notes, instrument rule)
   │ GET /api/{in|us}/lab/fidelity?method=<id>&ticker=…&from&to   (display-only comparison)
   │ POST /api/app/lab/strategies  {name, spec(origin…), market, instrument}   (Phase 4 storage + origin)
   │ GET /api/{in|us}/lab/run|optimize?spec=…        (variant label, instrument rule enforced)
   ▼
engine lab module (all new code under backend/src/lab/)
   rules.rs      spec v1 fields that were reserved: direction short, origin, execution.fill close /
                 exit_fill, entry.limit, exit.stop_loss / target / trailing; optimize paths for them
   series.rs     Compiled carries the execution settings and ATR series for ATR distances
   simulate.rs   one simulator: market fills at next open or signal close, limit entries, price exits
   templates.rs  43 templates (JSON specs), per-method "not available" reasons, catalogue status mirror
   fidelity.rs   trade-by-trade comparison (pure)
   api.rs        /api/lab/templates, /api/lab/template, /api/lab/fidelity; origin handling in run/optimize
gateway policy.rs   + "lab/templates", "lab/template", "lab/fidelity" (allowlist lines only)
auth contracts.mjs  spec.origin shape check; lab-strategies.mjs stores/keeps origin; db/003-lab-origin.sql
```

Seams with Phases 5 and 6 (both land first):
- Phase 5 rewires `lab/api.rs` (`serve` → `response`, provisional wrapper, gl instrument JSON, `lab_stats`)
  and adds `lab/instruments` to the gateway allowlist. This phase's `api.rs` edits are anchored on
  functions that survive that (`run_json`, `optimize_json`, `route`'s match) and add new functions; the
  gateway lines go after whatever the last `lab/…` allowlist entry is. On `engine-gl` the new routes are
  served too (Phase 5 lets `gl` reach every `lab/*` path); fidelity there answers `needs_data` because no
  book method admits gl instruments.
- Phase 6 adds indicators to `catalogue.rs` and the family files. This phase only *uses* `rsi`, `sma`,
  `bbpct`, `dmi`, `roc`, `lowest`, `atr` and price sources. If Phase 6 changes one of them, the template
  fidelity tests (§10) fail and name the template; the fix is to adjust or drop that template, never to
  change the book.
- `lab/json.rs` is unchanged. `lab/rules.rs`, `lab/series.rs` and `lab/simulate.rs` are changed only in
  the places listed in §5.

## 4. Template catalogue

### 4.1 Rules for a template

1. The method's catalogue status is `live` (computed exactly as the guru catalogue does — gate verdict
   `withheld` without a reviewed cash rule, or `source_review_required`, means not admitted).
2. The method is single-direction (Long or Short), one position at a time, no scale-in, and every rule is
   expressible with this phase's spec (§5): single-instrument price and catalogue-indicator conditions
   with bar offsets, all-of entries, any-of exits, market fills at signal close or next open, limit
   entries, price exits, `bars_held`.
3. **Fidelity admission:** the unmodified template reproduces the book engine's trades *exactly* (same
   direction, entry bar and price, exit bar and price, open flag) on four deterministic synthetic
   fixtures (calm, calm, volatile, very volatile; 1,500 bars each) in a unit test that calls
   `connors::run` read-only. A template that does not reproduce is dropped with a named reason.
4. Templates use the book method's **default parameters**. Each declares its adaptations (common notes:
   lab indicators are the TradingView definitions; lab comparisons are exact while some book engines use
   a 1e-9 tie band; orders fill as the template's execution says; no book figures are part of a template).

### 4.2 Result of the survey

Survey method: the full catalogue was read from a scratch build of the engine (`/api/guru/{connors,chande,ross}/strategies`);
every admitted method's rules and the relevant `run` arms were read; every candidate template was then run
against the book engine (planning evidence, outside the repository).

| | Methods |
|---|---|
| Whole catalogue | 4,365 (364 books) |
| Not admitted (`not_testable` 3,540, `needs_data` 661) — no template, reason = the catalogue's own status note | 4,201 |
| Admitted (`live`) | 164 |
| **Admitted with a template** | **43** (stc9 18, spmt 12, ststw 5, hpetf 3, hbqts 3, rsi25 2) |
| Admitted without a template (named reason) | 121 |

Most frequent reasons among the 4,201 not admitted: source-specific rules and instrument not yet
through admission review (1,781); more than half of the gain from same-day trades (191); fixed point or
dollar amounts for an instrument the vault lacks (114); futures history missing (53+); optimisation-only
systems (49); no printed exit (37+); option chains (12).

Reasons among the 121 admitted without a template: two-sided / always-in (35), scale-in (24), calendar
rules (10), VIX as a second instrument (9), indicators not in the lab catalogue (8), book-specific
filters such as the Liquidity Filter (6), sums of an indicator (6), futures (6), external series TRIN (5),
close position within the bar (3), intraday or close-to-next-open holds (2), "k of n days" counts (2),
multi-day pending setup (1), portfolio rules (1), all-of exits (1), and two that fail fidelity because
their book engine uses AmiBroker conventions the lab deliberately does not copy (1 limit filled at the
limit price on gap opens, 1 time exit counted from the latest repeated buy signal).

Planning evidence (not a verification; recorded so the implementer knows what to expect): on the four
synthetic fixtures all 43 templates reproduce exactly; with the real Indian index file (read-only from the
NAS), the 31 index/ETF-type templates reproduce the book engine **N/N on NIFTY 50 and NIFTY BANK**
(62 comparisons, 8,513 book trades, 0 differences), and the 12 stock or ETF-only templates are correctly
refused on an index by the book's own admission ("not admitted on this instrument").

### 4.3 Per-method table (all 164 admitted methods)

"Instrument rule" is the book's `instrument_policy` requirement, which the variant keeps.
"cond." counts entry + exit conditions (limit 12). "offsets" = conditions on earlier bars (e.g. `close[1]`).

| Book | Method | Name | Dir | Instrument rule | Template | Lab features used / reason |
|---|---|---|---|---|---|---|
| hbqts | `hbqts_ma1_36` | MA crossover 1/36 (close vs 36-day MA) (Fig 13.2) | Long | individual_stock | **yes** (v1) | close fill, 2 cond. |
| hbqts | `hbqts_ma5_25` | Enter at market: 5/25 MA crossover on the close (Fig 7.1) | Long | individual_stock | **yes** (v1) | close fill, 2 cond. |
| hbqts | `hbqts_profit_target` | 10% profit target, no other exit (Fig 7.9) | Long | individual_stock | **yes** (v1) | close fill, target 10%, 1 cond. |
| hpetf | `hpetf_3dhl_long` | 3-Day High/Low · Long | Long | etf | **yes** (v1) | close fill, offsets, 9 cond. |
| hpetf | `hpetf_r3_long` | R3 · Long | Long | etf | **yes** (v1) | close fill, offsets, 7 cond. |
| hpetf | `hpetf_rsi75_short` | RSI 75 · Short | Short | etf | **yes** (v1) | short, close fill, 3 cond. |
| rsi25 | `r25_rsi25_basic_no_filter` | RSI 25 Explosions (no trend filter) | Long | index | **yes** (v1) | close fill, 2 cond. |
| rsi25 | `r25_rsi25_official_sma200` | RSI 25 Explosions (Official, 200-day SMA) | Long | index | **yes** (v1) | close fill, 3 cond. |
| spmt | `spmt_dmi_1` | DMI(1) · +DMI(3) < 5 | Long | declared_index_timing | **yes** (v1) | close fill, 4 cond. |
| spmt | `spmt_dmi_2` | DMI(2) · +DMI(2) < 3 | Long | declared_index_timing | **yes** (v1) | close fill, 4 cond. |
| spmt | `spmt_dmi_3` | DMI(3) · +DMI(2) < 10, 2 days | Long | declared_index_timing | **yes** (v1) | close fill, offsets, 5 cond. |
| spmt | `spmt_dmi_4` | DMI(4) · +DMI(2) < 10, 3 days | Long | declared_index_timing | **yes** (v1) | close fill, offsets, 6 cond. |
| spmt | `spmt_dmi_5` | DMI(5) · +DMI(2) < 15, 4 days | Long | declared_index_timing | **yes** (v1) | close fill, offsets, 7 cond. |
| spmt | `spmt_pctb_1` | %b < 0, 2 days + RSI | Long | declared_index_timing | **yes** (v1) | close fill, offsets, 5 cond. |
| spmt | `spmt_rsi2_4` | RSI(2) < 4 | Long | declared_index_timing | **yes** (v1) | close fill, 3 cond. |
| spmt | `spmt_rsi4_25` | RSI(4) < 25 | Long | declared_index_timing | **yes** (v1) | close fill, 4 cond. |
| spmt | `spmt_rsi_div2` | RSI Div 2-day | Long | declared_index_timing | **yes** (v1) | close fill, offsets, 5 cond. |
| spmt | `spmt_rsi_div8` | RSI Div 8-day | Long | declared_index_timing | **yes** (v1) | close fill, offsets, 5 cond. |
| spmt | `spmt_short_4up` | Short · 4 Up Days | Short | declared_index_timing | **yes** (v1) | short, close fill, offsets, 7 cond. |
| spmt | `spmt_short_strong` | Short · Strong 2-day | Short | declared_index_timing | **yes** (v1) | short, close fill, 5 cond. |
| stc9 | `dmi_1` | DMI(1) · +DI(3) < 5 | Long | index_or_etf | **yes** (v1) | close fill, 4 cond. |
| stc9 | `dmi_2` | DMI(2) · +DI(2) < 3 | Long | index_or_etf | **yes** (v1) | close fill, 4 cond. |
| stc9 | `dmi_3` | DMI(3) · +DI(2) < 10, 2 days | Long | index_or_etf | **yes** (v1) | close fill, offsets, 5 cond. |
| stc9 | `dmi_4` | DMI(4) · +DI(2) < 10, 3 days | Long | index_or_etf | **yes** (v1) | close fill, offsets, 6 cond. |
| stc9 | `dmi_5` | DMI(5) · +DI(2) < 15, 4 days | Long | index_or_etf | **yes** (v1) | close fill, offsets, 7 cond. |
| stc9 | `f1a` | Test 1a · Above 200-day MA | Long | individual_stock | **yes** (v1) | close fill, 2 cond. |
| stc9 | `f3a` | Test 3a · Pullback to 10-day low close | Long | individual_stock | **yes** (v1) | close fill, offsets, 11 cond. |
| stc9 | `f4a` | Test 4a · 10-day low + 3% limit | Long | individual_stock | **yes** (v1) | limit 3%, close exit, 3 cond. |
| stc9 | `f4c` | Test 4c · 10-day low + 7% limit | Long | individual_stock | **yes** (v1) | limit 7%, close exit, 3 cond. |
| stc9 | `f4d` | Test 4d · 7% limit, exit > 5MA | Long | individual_stock | **yes** (v1) | limit 7%, close exit, 3 cond. |
| stc9 | `pctb_1` | %b < 0 two days + RSI | Long | index_or_etf | **yes** (v1) | close fill, offsets, 5 cond. |
| stc9 | `rsi2_4` | RSI(2) < 4 | Long | index_or_etf | **yes** (v1) | close fill, 3 cond. |
| stc9 | `rsi2_5d` | Test 5d · RSI(2) < 2 | Long | individual_stock | **yes** (v1) | close fill, 3 cond. |
| stc9 | `rsi4_25` | RSI(4) < 25 | Long | index_or_etf | **yes** (v1) | close fill, 4 cond. |
| stc9 | `rsi_div2` | RSI Divergence 2-day | Long | index_or_etf | **yes** (v1) | close fill, offsets, 5 cond. |
| stc9 | `rsi_div8` | RSI Divergence 8-day | Long | index_or_etf | **yes** (v1) | close fill, offsets, 5 cond. |
| stc9 | `short_4up` | Short · 4 Up Days | Short | index_or_etf | **yes** (v1) | short, close fill, offsets, 7 cond. |
| stc9 | `short_strong` | Short · Strong 2-day | Short | index_or_etf | **yes** (v1) | short, close fill, 5 cond. |
| ststw | `ststw_10d_high_buy` | Buy 10-day highs, exit below 10-day MA | Long | index | **yes** (v1) | close fill, offsets, 10 cond. |
| ststw | `ststw_10d_high_short` | Short 10-day highs, cover below 10-day MA | Short | index | **yes** (v1) | short, close fill, offsets, 10 cond. |
| ststw | `ststw_10d_low_buy` | Buy 10-day lows, exit above 10-day MA | Long | index | **yes** (v1) | close fill, offsets, 10 cond. |
| ststw | `ststw_rsi2_under5` | 2-period RSI under 5 on the S&P 500 | Long | index | **yes** (v1) | close fill, 3 cond. |
| ststw | `ststw_sp_short` | The S&P Short Strategy | Short | index_or_etf | **yes** (v1) | short, close fill, offsets, 6 cond. |
| afactor | `afactor_baseline` | The Alvarez Factor (baseline) | Long | individual_stock | no | today's-open / intraday entry, or close-to-next-open hold |
| alpha | `alpha_etf_avalanches_single` | ETF Avalanches · single instrument | Short | etf | no | portfolio caps, ranking, cash ETF |
| alpha | `alpha_spy_rsi4_mr` | SPY Mean Reversion (RSI 4: buy < 25, add < 20, sell > 55) | Long | etf | no | adds units in the trade (scale-in) |
| ats | `ats_dvmtm` | Double Volume Market Top Method (DVMTM) | Short | individual_stock | no | multi-day pending setup with stop entry |
| ch_bta | `ch_65sma3cc` | 65sma-3cc reversal · OHLC adaptation | Long/Short | declared_ohlc_adaptation | no | long and short in one strategy (always in / reversing) |
| ch_bta | `ch_cbpb_time` | CB-PB fixed-horizon · OHLC adaptation | Long | declared_ohlc_adaptation | no | indicator not in the lab catalogue |
| chbta2 | `ch_bta2_65sma_ravi` | 65sma-3cc with RAVI filter | Long/Short | declared_ohlc_adaptation | no | long and short in one strategy (always in / reversing) |
| chbta2 | `ch_bta2_ma_stack` | Multiple short SMAs versus 65-day SMA | Long/Short | declared_ohlc_adaptation | no | long and short in one strategy (always in / reversing) |
| ehmd | `ehmd_mama` | MAMA / FAMA crossover (long only) | Long | individual_stock | no | indicator not in the lab catalogue |
| ehrs | `ehrs_sinetrend` | SineTrend Automatic System (Fig. 12.1) | Long/Short | declared_ohlc_adaptation | no | long and short in one strategy (always in / reversing) |
| etfss | `ss_both` | ETF Sweet Spot · Long & Short | Long/Short | etf | no | long and short in one strategy (always in / reversing) |
| etfss | `ss_long` | ETF Sweet Spot · Long | Long | etf | no | adds units in the trade (scale-in) |
| etfss | `ss_short` | ETF Sweet Spot · Short | Short | etf | no | adds units in the trade (scale-in) |
| hbqts | `hbqts_bb_breakout` | Bollinger band breakout 12 / 2.8 (Fig 9.13) | Long/Short | individual_stock | no | long and short in one strategy (always in / reversing) |
| hbqts | `hbqts_cgo` | Center of Gravity oscillator crossover, 6-day max hold (Fig 10.1) | Long | individual_stock | no | indicator not in the lab catalogue |
| hbqts | `hbqts_cmo` | CMO oscillator crossover with 6-day exit (Fig 20.2) | Long | etf | no | indicator not in the lab catalogue |
| hbqts | `hbqts_dom` | Seasonality: day of the month (Fig 11.3) | Long/Short | individual_stock | no | long and short in one strategy (always in / reversing) |
| hbqts | `hbqts_dpo` | Detrended price oscillator crossover (Fig 10.8) | Long | individual_stock | no | indicator not in the lab catalogue |
| hbqts | `hbqts_entry_id1` | Entry identification I: 3/16 MA cross, hold 6 days (Fig 7.14) | Long | individual_stock | no | time exit counts from the latest repeated buy signal (AmiBroker BarsSince); fidelity fails on fixtures |
| hbqts | `hbqts_fixed_interval` | Enter every 6th bar, hold 3 (Fig 7.6) | Long | individual_stock | no | indicator not in the lab catalogue |
| hbqts | `hbqts_limit_entry` | Enter on a limit 1% below the signal close (Fig 7.2) | Long | individual_stock | no | book fills gapped limits at the limit price (AmiBroker); fidelity fails on fixtures |
| hbqts | `hbqts_macd_daily` | MACD-slope permission filter on daily data + 8/7 MA (Fig 17.6) | Long | individual_stock | no | indicator not in the lab catalogue |
| hbqts | `hbqts_stoch_rsi` | Stochastic of the RSI (Fig 10.3) | Long | individual_stock | no | indicator not in the lab catalogue |
| hpetf | `hpetf_3dhl_long_aggr` | 3-Day High/Low · Long aggressive | Long | etf | no | adds units in the trade (scale-in) |
| hpetf | `hpetf_mdd_long` | Multiple Days Down (MDD) · Long | Long | etf | no | "k of the last n days" count |
| hpetf | `hpetf_mdd_long_aggr` | Multiple Days Down (MDD) · Long aggressive | Long | etf | no | adds units in the trade (scale-in) |
| hpetf | `hpetf_mdu_short` | Multiple Days Up (MDU) · Short | Short | etf | no | "k of the last n days" count |
| hpetf | `hpetf_mdu_short_aggr` | Multiple Days Up (MDU) · Short aggressive | Short | etf | no | adds units in the trade (scale-in) |
| hpetf | `hpetf_pctb_long_aggr` | %b · Long aggressive | Long | etf | no | adds units in the trade (scale-in) |
| hpetf | `hpetf_pctb_short_aggr` | %b · Short aggressive | Short | etf | no | adds units in the trade (scale-in) |
| hpetf | `hpetf_r3_long_aggr` | R3 · Long aggressive | Long | etf | no | adds units in the trade (scale-in) |
| hpetf | `hpetf_rsi10_6_long` | RSI 10/6 · Long | Long | etf | no | adds units in the trade (scale-in) |
| hpetf | `hpetf_rsi25_long_aggr` | RSI 25 · Long aggressive | Long | etf | no | adds units in the trade (scale-in) |
| hpetf | `hpetf_rsi90_94_short` | RSI 90/94 · Short | Short | etf | no | adds units in the trade (scale-in) |
| hpetf | `hpetf_tps_long` | TPS 1-2-3-4 · Long | Long | etf | no | adds units in the trade (scale-in) |
| hpetf | `hpetf_tps_short` | TPS 1-2-3-4 · Short | Short | etf | no | adds units in the trade (scale-in) |
| kftsm | `kftsm_4_9_18` | 4-9-18 crossover | Long/Short | declared_ohlc_adaptation | no | long and short in one strategy (always in / reversing) |
| kftsm | `kftsm_adapt_exp` | Adaptive driver: exponential benchmark | Long/Short | declared_ohlc_adaptation | no | long and short in one strategy (always in / reversing) |
| kftsm | `kftsm_adapt_r2` | Adaptive driver: correlation R2 trend | Long/Short | declared_ohlc_adaptation | no | long and short in one strategy (always in / reversing) |
| kftsm | `kftsm_adapt_rsi` | Adaptive driver extension: Wilder RSI trend | Long/Short | declared_ohlc_adaptation | no | long and short in one strategy (always in / reversing) |
| kftsm | `kftsm_bb_ma` | Moving average with Bollinger bands | Long/Short | declared_ohlc_adaptation | no | long and short in one strategy (always in / reversing) |
| kftsm | `kftsm_exp` | Exponential smoothing trendline direction | Long/Short | declared_ohlc_adaptation | no | long and short in one strategy (always in / reversing) |
| kftsm | `kftsm_fc_bb_abx` | FC Bollinger ABX (two bands, profit-taking) | Long/Short | declared_ohlc_adaptation | no | long and short in one strategy (always in / reversing) |
| kftsm | `kftsm_hh_channel` | Hochheimer closing-price channel (N-day breakout) | Long/Short | declared_ohlc_adaptation | no | long and short in one strategy (always in / reversing) |
| kftsm | `kftsm_hh_cross` | Hochheimer two moving average crossover | Long/Short | declared_ohlc_adaptation | no | long and short in one strategy (always in / reversing) |
| kftsm | `kftsm_hh_ma` | Hochheimer single moving average (close penetration) | Long/Short | declared_ohlc_adaptation | no | long and short in one strategy (always in / reversing) |
| kftsm | `kftsm_hirsch` | Hirsch November-April | Long | declared_ohlc_adaptation | no | calendar rule (day of month, month end, named months) |
| kftsm | `kftsm_kama` | KAMA adaptive moving average system | Long/Short | declared_ohlc_adaptation | no | long and short in one strategy (always in / reversing) |
| kftsm | `kftsm_keltner` | Keltner 10-day moving average rule | Long/Short | declared_ohlc_adaptation | no | long and short in one strategy (always in / reversing) |
| kftsm | `kftsm_lrs` | Linear regression slope | Long/Short | declared_ohlc_adaptation | no | long and short in one strategy (always in / reversing) |
| kftsm | `kftsm_ma` | Moving average trendline direction | Long/Short | declared_ohlc_adaptation | no | long and short in one strategy (always in / reversing) |
| kftsm | `kftsm_ma_pct_band` | Moving average with percent bands | Long/Short | declared_ohlc_adaptation | no | long and short in one strategy (always in / reversing) |
| kftsm | `kftsm_ma_tr_band` | Moving average with true-range bands | Long/Short | declared_ohlc_adaptation | no | long and short in one strategy (always in / reversing) |
| kftsm | `kftsm_mod_3ma` | Modified 3 moving average crossover | Long/Short | declared_ohlc_adaptation | no | long and short in one strategy (always in / reversing) |
| kftsm | `kftsm_month_end` | Month-end effect | Long | declared_ohlc_adaptation | no | calendar rule (day of month, month end, named months) |
| kftsm | `kftsm_ndb` | N-day breakout (high/low with close confirmation) | Long/Short | declared_ohlc_adaptation | no | long and short in one strategy (always in / reversing) |
| kftsm | `kftsm_outside_day` | Outside day with an outside close | Long/Short | declared_ohlc_adaptation | no | long and short in one strategy (always in / reversing) |
| kftsm | `kftsm_raschke_first_cross` | Raschke First Cross | Long/Short | declared_ohlc_adaptation | no | long and short in one strategy (always in / reversing) |
| kftsm | `kftsm_swg` | Swing breakout (KSWG) | Long/Short | declared_ohlc_adaptation | no | long and short in one strategy (always in / reversing) |
| kftsm | `kftsm_swing` | Percent swing method (TSM Swing) | Long/Short | declared_ohlc_adaptation | no | long and short in one strategy (always in / reversing) |
| kftsm | `kftsm_triple_smooth` | Triple exponential smoothing | Long/Short | declared_ohlc_adaptation | no | long and short in one strategy (always in / reversing) |
| kftsm | `kftsm_vidya` | Adaptive driver: Chande VIDYA | Long/Short | declared_ohlc_adaptation | no | long and short in one strategy (always in / reversing) |
| kftsm | `kftsm_vol_breakout` | Volatility system (Bookstaber) | Long/Short | declared_ohlc_adaptation | no | long and short in one strategy (always in / reversing) |
| lwdg2 | `lwdg2_vol_dn_dn` | Volume down, close down: buy next open (Table 4) | Long | futures | no | futures contract prices (not admitted) |
| lwdg2 | `lwdg2_vol_dn_up` | Volume down, close up: buy next open (Table 2) | Long | futures | no | futures contract prices (not admitted) |
| lwdg2 | `lwdg2_vol_up_dn` | Volume up, close down: buy next open (Table 3) | Long | futures | no | futures contract prices (not admitted) |
| lwdg2 | `lwdg2_vol_up_up` | Volume up, close up: buy next open (Table 1) | Long | futures | no | futures contract prices (not admitted) |
| lwdg2 | `lwdg2_weekly_entry` | Weekly entry signal | Long/Short | futures | no | futures contract prices (not admitted) |
| lwrsr | `lwrsr_apr_oct` | Buy the Dow the last day of April, sell the last trading day of October | Long | index | no | calendar rule (day of month, month end, named months) |
| lwrsr | `lwrsr_oct_apr` | Buy the Dow the last day of October, sell the last trading day of April | Long | index | no | calendar rule (day of month, month end, named months) |
| mmft | `mmft_nasdaq79` | Nasdaq 7.9% momentum model (Model #1) | Long/Short | index | no | long and short in one strategy (always in / reversing) |
| pctb | `pbs_percent_b_plus_one` | %b Plus One Swing | Long/Short | index_etf_or_stock | no | long and short in one strategy (always in / reversing) |
| pctb | `pbs_percent_b_swing` | Bollinger Bands %b Swing (standard) | Long/Short | index_etf_or_stock | no | long and short in one strategy (always in / reversing) |
| r3r4 | `r3r4_base` | R3/R4 Base Rules | Long | individual_stock | no | Liquidity Filter, book HV/ADX or HV-scaled stretch |
| r3r4 | `r3r4_base_scale` | R3/R4 Base Rules · Aggressive (scale-in) | Long | individual_stock | no | adds units in the trade (scale-in) |
| r3r4 | `r3r4_bigcap` | R3/R4 Big Cap Rules (Guide) | Long | individual_stock | no | Liquidity Filter, book HV/ADX or HV-scaled stretch |
| r3r4 | `r3r4_bigcap_deck` | R3/R4 Big Cap Rules (class deck) | Long | individual_stock | no | Liquidity Filter, book HV/ADX or HV-scaled stretch |
| r3r4 | `r3r4_bigcap_deck_scale` | R3/R4 Big Cap Rules (class deck) · Aggressive | Long | individual_stock | no | adds units in the trade (scale-in) |
| r3r4 | `r3r4_bigcap_scale` | R3/R4 Big Cap Rules (Guide) · Aggressive | Long | individual_stock | no | adds units in the trade (scale-in) |
| r3r4 | `r3r4_midcap` | R3/R4 Mid Cap Rules (class deck) | Long | individual_stock | no | Liquidity Filter, book HV/ADX or HV-scaled stretch |
| r3r4 | `r3r4_midcap_scale` | R3/R4 Mid Cap Rules (class deck) · Aggressive | Long | individual_stock | no | adds units in the trade (scale-in) |
| radge | `radge_ma220` | 2/20 Moving Average Breakout | Long/Short | futures | no | futures contract prices (not admitted) |
| rsi25 | `r25_rsi25_15` | RSI 25/15 (aggressive add-on) | Long | index | no | adds units in the trade (scale-in) |
| rsi25 | `r25_rsi25_15_vix` | RSI 25/15 with the VIX | Long | index | no | reads the VIX (second instrument) |
| rsi25 | `r25_rsi25_vix` | RSI 25 Explosions with the VIX | Long | index | no | reads the VIX (second instrument) |
| spmt | `spmt_eom_cc` | EOM · Close < Close | Long | declared_index_timing | no | calendar rule (day of month, month end, named months) |
| spmt | `spmt_eom_co` | EOM · Close < Open | Long | declared_index_timing | no | calendar rule (day of month, month end, named months) |
| spmt | `spmt_eom_rsi` | EOM · RSI | Long | declared_index_timing | no | calendar rule (day of month, month end, named months) |
| spmt | `spmt_pctb_2` | %b < 0.2 + Bottom | Long | declared_index_timing | no | close position within the bar range |
| spmt | `spmt_price` | Close < Open 3 days, bottom 10% of range | Long | declared_index_timing | no | close position within the bar range |
| spmt | `spmt_rsi_1` | RSI-1 · 3-day RSI(2) sum < 50 | Long | declared_index_timing | no | sum of an indicator over days |
| spmt | `spmt_rsi_2` | RSI-2 · 2-day RSI(2) sum < 40 | Long | declared_index_timing | no | sum of an indicator over days |
| spmt | `spmt_rsi_3` | RSI-3 · 2-day RSI(3) sum < 45 | Long | declared_index_timing | no | sum of an indicator over days |
| stc9 | `eom_cc` | EOM · Close < Close | Long | index_or_etf | no | calendar rule (day of month, month end, named months) |
| stc9 | `eom_co` | EOM · Close < Open | Long | index_or_etf | no | calendar rule (day of month, month end, named months) |
| stc9 | `eom_rsi` | EOM · RSI | Long | index_or_etf | no | calendar rule (day of month, month end, named months) |
| stc9 | `h10` | STC Short Selling (H10) | Short | individual_stock | no | Liquidity Filter, book HV/ADX or HV-scaled stretch |
| stc9 | `pctb_2` | %b < 0.2 three days + bottom 25% | Long | index_or_etf | no | close position within the bar range |
| stc9 | `r3` | R3 Strategy | Long | index | no | adds units in the trade (scale-in) |
| stc9 | `rsi_1` | RSI-1 · 3-day RSI(2) sum < 50 | Long | index_or_etf | no | sum of an indicator over days |
| stc9 | `rsi_2` | RSI-2 · 2-day RSI(2) sum < 40 | Long | index_or_etf | no | sum of an indicator over days |
| stc9 | `rsi_3` | RSI-3 · 2-day RSI(3) sum < 45 | Long | index_or_etf | no | sum of an indicator over days |
| stc9 | `s12345` | 1-2-3-4-5 Scale-In | Long | leveraged_etf | no | adds units in the trade (scale-in) |
| stc9 | `tps_long` | TPS 2-3-5 Long | Long | etf | no | adds units in the trade (scale-in) |
| stc9 | `tps_short` | TPS 2-3-5 Short | Short | etf | no | adds units in the trade (scale-in) |
| stc9 | `tps_stocks` | TPS for Stocks | Long | individual_stock | no | adds units in the trade (scale-in) |
| stc9 | `trin2_11` | TRIN 2 Days > 1.1 | Long | index_or_etf | no | external series (TRIN) |
| stc9 | `trin3_10` | TRIN 3 Days > 1.0 | Long | index_or_etf | no | external series (TRIN) |
| stc9 | `trin3_11` | TRIN 3 Days > 1.1 | Long | index_or_etf | no | external series (TRIN) |
| stc9 | `trin4` | TRIN 4 Days > 1.0 | Long | index_or_etf | no | external series (TRIN) |
| stc9 | `trin4_12` | TRIN 4 Days > 1.2 | Long | index_or_etf | no | external series (TRIN) |
| stc9 | `vix101_short` | Short · VIX 10.1 | Short | index_or_etf | no | reads the VIX (second instrument) |
| stc9 | `vix2` | VIX (2) · VIX RSI(2) > 90 + gap | Long | index_or_etf | no | reads the VIX (second instrument) |
| stc9 | `vix200` | VIX 200 · stretched 5% for 3 days | Long | index_or_etf | no | reads the VIX (second instrument) |
| stc9 | `vix4` | VIX (4) · VIX up 5 days | Long | index_or_etf | no | reads the VIX (second instrument) |
| stc9 | `vix53_short` | Short · VIX 5.3 | Short | index_or_etf | no | reads the VIX (second instrument) |
| stc9 | `vix7` | VIX (7) · up 4 days + gap over high | Long | index_or_etf | no | reads the VIX (second instrument) |
| stc9 | `vix8` | VIX (8) · stretched 10% for 2 days | Long | index_or_etf | no | reads the VIX (second instrument) |
| stc9 | `x555` | 5x5x5 Version 2.0 | Long | individual_stock | no | Liquidity Filter, book HV/ADX or HV-scaled stretch |
| ststw | `ststw_double7` | Double 7's | Long | etf | no | exit needs all-of conditions |
| ststw | `ststw_overnight` | Overnight: buy the close, sell the next open | Long | etf | no | today's-open / intraday entry, or close-to-next-open hold |

## 5. Engine additions (reserved spec fields made real)

The spec version stays **`"v": 1`**: these fields were reserved in v1 and refused as `unsupported`; now
they are accepted. Every existing saved spec means exactly what it meant before (defaults reproduce the
v1 simulator; the existing simulator tests stay unchanged and pass).

```json
{ "v": 1, "direction": "long | short",
  "origin": { "book": "stc9", "method": "rsi2_4", "template_version": 1 },
  "entry": { "all": [ …conditions… ],
             "limit": { "pct": 3 }  |  { "atr": 0.4, "atr_length": 10 } },
  "exit":  { "any": [ …conditions… ],                       // may be [] when a price exit exists
             "stop_loss": { "pct": 5 } | { "atr": 2, "atr_length": 14 },
             "target":    { "pct": 10 } | { "atr": 3 },
             "trailing":  { "pct": 8 }  | { "atr": 3, "atr_length": 20 } },
  "execution": { "fill": "next_open | close", "exit_fill": "next_open | close", "cost_bps": 12 },
  "optimize": [ {"path": "exit.stop_loss.pct", "from": 2, "to": 8, "step": 1}, … ] }
```

| Field | Meaning | Bounds / errors |
|---|---|---|
| `direction` | `short` simulates the mirror position (`Trade.dir = -1`; book accounting already handles shorts) | `long`/`short`, else invalid |
| `origin` | lineage of a customized template (§7); engine checks the template exists (same book, version ≤ current) | ids `[A-Za-z0-9_]{1,80}`, version 1–1000 |
| `execution.fill` | market entries (and, by default, signal exits) fill at the **next open** (v1) or the **signal bar's close** (the books' "buy on the close") | `limit`/`stop` stay reserved (`unsupported`) |
| `execution.exit_fill` | overrides where signal exits fill | `next_open`/`close`; default = `fill` |
| `entry.limit` | after an entry signal, a limit order for the **next bar only** at close ∓ distance (below for long, above for short); fills if the bar's low (high) touches it, at the better of open and limit; otherwise expires | pct 0–50, atr mult (0, 50], atr_length 1–500 (default 14); `execution.fill` must be absent |
| `exit.stop_loss` | fixed from the entry price: entry ∓ distance (ATR read at the signal bar) | pct (0, 99] |
| `exit.target` | entry ± distance | pct (0, 1000] |
| `exit.trailing` | best price since entry (through the previous bar) ∓ distance (ATR at the previous close) | pct (0, 99] |
| optimize paths | `entry.limit.pct|atr`, `exit.stop_loss.pct|atr`, `exit.target.pct|atr`, `exit.trailing.pct|atr` (must match the form used) | same grid caps as v1 (≤ 1,000 values per range, ≤ 50,000 combinations) |

Still reserved (`unsupported`): `entry.stop` (stop entries), `scale_in`, `portfolio`, `execution.fill`
`limit`/`stop`. Price exits do not count toward the 12-condition limit.

### 5.1 Bar event order (one position at a time)

On bar *i*:
1. **At the open:** a pending next-open exit fills; then a pending next-open market entry; then a resting
   limit entry (touch test on low/high, fill at the better of open and limit).
2. **Inside the bar:** stop-loss / trailing / target. A gap through a level fills **at the open**. If a
   stop (or trailing) level and the target are both inside one bar, the **stop** fills (conservative —
   daily bars do not show which came first). Stop-loss and trailing combine to the tighter level.
   On a **limit-fill bar** the stop is assumed reachable after the fill but the target is not checked
   (the high may have preceded the fill). A close-fill entry is never price-checked on its own bar.
3. **At the close:** exit conditions (any) fill at this close, or set a next-open exit. Then entry
   conditions (all) fill at this close, set a next-open market order, or place a limit for bar *i+1*.

Re-entry: with next-open market fills the v1 rule is kept (an exit and a new entry may share one open).
With close fills or limits the books' rule applies: no new order on a bar that held a position at its
close or exited after its open (a next-open exit at bar *i*'s open still allows an entry at *i*'s close).
No signal on the last bar of the window; a position still open there is marked `open` at the last close.

### 5.2 No lookahead

Signals use values at the signal bar's close or earlier. Fills happen at that close (`close` mode) or
later. Stop and target levels use the entry price and ATR at the signal bar; trailing levels use the
best price through the previous bar and ATR at the previous close; limit prices use the signal close.
A property test perturbs every bar after *k* and requires every trade closed by *k* to be unchanged, for
four specs that together cover next-open and close fills, short, limits, stop, target and trailing.

## 6. Fidelity check (display only)

`GET /api/{in|us}/lab/fidelity?method=<id>&ticker=…[&from&to][basket params]` runs, on the same data:
- the **book engine**, exactly as `/api/guru/*/run` would: `select_context` → `resolve_context` →
  `readiness::check` + `native_admission` (refuse with `needs_data` "the book method is not admitted on
  this instrument and window (…)") → `window_start` → `connors::run(id, ctx, from, to, default params)`;
- the **unmodified template** through the lab compiler and simulator on the same `from..to`.

Trades are paired by entry bar; a pair matches when direction, entry bar and price, exit bar and price
(1e-9 relative) and the open flag agree; multi-leg book trades never match. Response:
`{label, kind:"lab_template_fidelity", method, template_version, instrument, from, to, book_trades,
template_trades, matched, reproduces, differences_total, differences:[{book, template}] (first 50), note}`.
No costs, no statistics, no equity, nothing stored. `p.*` parameters are refused (defaults only).
Book code is called, never changed. The UI shows "Template reproduces the book engine's trades: N/N" or
the differences — never "verified".

## 7. Storage and migration

- `spec.origin` is part of the saved spec, so drafts, edits and **published snapshots** carry the lineage
  without touching `lab_strategy_version`.
- `db/003-lab-origin.sql` (additive, idempotent, applied by one added `bootstrap.mjs` line after 002):
  `ALTER TABLE lab_strategy ADD COLUMN IF NOT EXISTS origin jsonb CHECK (origin IS NULL OR (jsonb_typeof(origin)='object' AND origin ? 'book' AND origin ? 'method' AND origin ? 'template_version'))`.
- The auth service sets `origin` once, at creation, from `spec.origin` (validated shape:
  `{book, method, template_version}`); an edit whose `spec.origin` differs from the row's origin
  (removed, changed or newly added) is `400 "A strategy keeps the book template it was customized from
  (spec.origin); start a new customization instead."`. Duplicate copies the origin. Lists and reads
  return `origin`; the YT Strategies list returns `spec->'origin' AS origin` for each snapshot.
- Submit/publish rules are unchanged: in/us only, admin publishes, snapshots frozen.
- No parent/duplicate-lineage column (a copy keeps the template origin; that is the lineage that matters).

## 8. API

Engine (through the gateway as `/api/{in|us}/lab/…`; all signed-in roles, like book runs):

| Route | Answer |
|---|---|
| `GET lab/templates` | `{label, methods: 4365, admitted: 164, templates:[{method:{id,name,book,group,dir}, template_version, requirement, allowed_types}]}` |
| `GET lab/template?method=<id>` | available: `{label, method, status:"available", instrument:{requirement, allowed_types}, template:{version, spec, notes}}`; otherwise `{label, method, status:"unavailable", reason}` (200); unknown id → 400 invalid |
| `GET lab/fidelity?method=<id>&ticker=…` | §6; no template → 400 `unsupported` "Customize not available — <reason>"; book not admitted → 400 `needs_data` |
| `GET lab/run`, `lab/optimize` | unchanged contract plus: `"origin": null | {method:{…}, template_version}` after `kind`; variant `label`; each run trade gains `"dir"` and `"entry_kind"`; a spec with an unknown origin → invalid; a variant on an instrument type its method does not admit → `needs_data` "SPX (index): variants of stc9 · Test 1a … run on individual_stock only (no index or other substitute)" |

Gateway: three allowlist lines (`lab/templates`, `lab/template`, `lab/fidelity`); none is an
optimization path; no spec is sent to them. Auth (`/api/app/lab/strategies`): unchanged routes; bodies
may carry `spec.origin`; responses carry `origin`.

## 9. Labels and copy

| Where | Text |
|---|---|
| Plain lab run (unchanged) | User-defined indicator strategy · simulated · not a book method |
| Variant run / optimization | Customized from <book> · <method name> · user variant · simulated |
| Template | Book template · declared adaptation into lab rules · not a verification |
| Fidelity | Template fidelity · display only · not a verification — "Compares the template's simulated trades with the book engine's simulated trades on the same data, window and default parameters, before costs. It does not verify the book method, its published figures or either engine." |
| No template | Customize not available — <reason> |
| Variant on a wrong instrument type | <ticker> (<type>): variants of <book> · <method> run on <types> only (no index or other substitute) |

Templates, variants and fidelity responses never contain `claim`, `gate` or `readiness` fields.

## 10. Testing

- **Rust (`cargo test`, `CARGO_BUILD_JOBS=2`):** rules — new fields parse, bounds name the field, reserved
  fields still `unsupported`, optimize reaches price orders; simulator — close fills skip the entry bar's
  exits and block same-close re-entry, next-open exits allow re-entry at the exit bar's close, last-bar
  close exit, short mirror, limit touch / gap / expiry / short limit, limit-bar exit and order block, stop
  and target levels and gaps, stop wins inside one bar, limit bar assumes stop but not target, trailing
  from closed bars, ATR from the signal bar (NaN ATR = no level), future-bars property test; fidelity —
  pairing and difference kinds; templates — every template parses, names its live method and direction,
  reproduces the book engine on four fixtures (non-vacuous), catalogue status mirror equals the guru
  catalogue for all 4,365 methods, every admitted method has a template or a named reason; API —
  template list/detail/unavailable, variant label and instrument rule, origin errors, fidelity N/N on a
  synthetic index, fidelity refusals. Planning count: 690 → 719 engine tests on `d165472` (29 new).
- **Gateway:** allowlist test for the three routes (6 → 7 tests).
- **Auth (`node --test`):** origin shape and `sameLabOrigin`; 003 migration additive and ordered after
  002; routes store origin, refuse an origin change on edit, keep it on duplicate, list it for snapshots
  (39 → 43 in-process tests).
- **Manual isolated-stack gate (the user):** `lab-strategies.integration.test.mjs` gains one subtest
  (create a variant, edit cannot drop its origin, duplicate keeps it, publish shows it on YT Strategies)
  and the migration re-run on restart.
- **Book guard:** `check_book_untouched.py --base origin/Manan --engine-compare …` at the gate; scope proof
  shows no protected path, no `main.rs` change, no frontend change.

## 11. Delivery

One backend PR from `Nandini` into `Manan` (never `main`), after Phases 5 and 6 are merged into `Nandini`,
implemented task by task (subagent-driven, TDD) from
`docs/superpowers/plans/2026-10-16-customize-book-strategies.md`. The user pushes.

**Frontend, planned for the later frontend phase (not in this phase's plan):**
1. "Customize" button on admitted book method pages. `ResearchPages.tsx` is protected, so the entry point
   must be additive (a new component registered through `custom-pages.ts` or an existing extension slot);
   if no additive slot exists, the entry point is a "Customize a book method" picker on the Indicators
   page (method search over `/lab/templates`), and the book-page button waits for Manan's approval.
2. Indicators page `?customize=<method>` route: load `/lab/template`, pre-fill the rule builder, show the
   template label, the instrument rule and the adaptation notes; "Customize not available — <reason>"
   when unavailable.
3. Rule-builder controls for the new fields: direction, fill / exit fill, limit entry, stop-loss, target,
   trailing (pct or ATR), and optimize ranges for them.
4. Fidelity panel ("Template reproduces the book engine's trades: N/N" or the difference list), clearly
   display-only, on the template's first run.
5. Variant badges: "Customized from <book> · <method> · user variant · simulated" on results, saved
   lists and YT Strategies; group "My variants of <method>" client-side from `origin`.
6. Instrument picker limited to the method's admitted instrument types for variants.
7. Vitest for the builder mapping and copy; typecheck; real-browser check desktop + mobile across themes.

## 12. Assumptions for the user's morning review

Decided without asking (the user authorised overnight work and is asleep). Each can be reversed cheaply.

1. **Templates only for admitted (`live`) methods.** Expressible but unadmitted methods (e.g. many stock
   tests) get "Customize not available — the book method is not admitted …". Customizing an unadmitted
   method would publish results on unresolved rules, which AGENTS.md forbids.
2. **Template admission = exact reproduction on four synthetic fixtures** (a unit test). Real-data
   fidelity is shown on demand and may differ on edge values; it never blocks a template and never
   claims verification.
3. **43 templates.** Dropped: `hbqts_limit_entry` (AmiBroker fills a gapped limit at the limit price; the
   lab fills at the open) and `hbqts_entry_id1` (AmiBroker `BarsSince(Buy)` restarts on repeated buy
   signals; lab `bars_held` counts from the fill). The lab does not copy these conventions.
4. **Close fills are an explicit option**, the default stays next-open. The books' "buy on the close" is
   the reproduction target for templates; users can switch a variant to next-open.
5. **Conservative intrabar rules:** stop before target inside one bar; gaps fill at the open; limit-fill
   bars check the stop but not the target; trailing uses only closed bars.
6. **ATR is the lab's TradingView ATR** (RMA of true range), read at the signal bar (stops, targets,
   limits) or the previous close (trailing). While ATR warms up the level is inactive (no order/stop).
7. **Re-entry follows the books for close fills and limits**; next-open market mode keeps the v1 rule.
8. **Spec stays `v: 1`**; no saved spec changes meaning; the auth service's `v === 1` check is unchanged.
9. **Origin lives in the spec and in a DB column set once.** Edits cannot add, change or remove it; a
   variant whose author wants no lineage starts a fresh lab strategy. No parent-id column.
10. **Variants keep the method's instrument admission** (`allowed_types`), enforced by the engine on run
    and optimize, so a stock method's variant never runs on an index.
11. **Variant label uses the book id** (e.g. `stc9`) and the method name; the frontend may show the full
    book title next to it.
12. **Fidelity uses defaults, the book's own window and admission, no costs**, and is open to every
    signed-in role (a read, like book runs). It is not an optimization path.
13. **Out of scope** (each listed as a reason): scale-in, two-sided/always-in methods, stop entries,
    calendar operands, sums/ratios of series, second instruments (VIX, TRIN), portfolio rules,
    multi-day pending setups, futures, intraday. Later specs can add them and grow the template list.
14. **A completeness test** fails when an admitted method has neither a template nor a named reason.
    After a future merge from `Manan` adds admitted methods, the fix is one reason line (or a template).
15. **Frontend entry point** respects the protected `ResearchPages.tsx` (§11 item 1).
16. **Phases 5 and 6 land first.** Code in the plan was proven on `d165472`; anchors in `lab/api.rs` and the
    gateway allowlist are described by function so they survive Phase 5's edits. If Phase 6 changes an
    indicator a template uses, the fidelity test names the template.
17. **No `main.rs` change**, which keeps the additive-only file untouched in this phase.
