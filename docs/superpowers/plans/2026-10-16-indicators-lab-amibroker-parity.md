# Indicators lab — AmiBroker parity implementation plan

**Gap list (spec):** `docs/superpowers/specs/2026-10-16-indicators-lab-amibroker-parity-gaps.md`

Done on 2026-10-16 (working tree, not committed): Parameters window (Task 1), AmiBroker report + MAE/MFE + tradesheet columns (Task 2).
Tasks 3–7 done (2026-10-17, working tree, not committed): trade prices / delays / trailing % of profit / re-entry per stop (3),
Monte Carlo (4), settings tabs + scenario presets (5), walk-forward (6), sizing limits (7).
Correction to the gap list: stop distances in points (`Dist::Points`) and ticks already exist in engine and UI.

## Global constraints

- Book results never change: no edits to `connors.rs`, `books/**`, `readiness.rs`, `gate.rs`, `instrument_policy.rs`, `baskets.rs`,
  `accounting.rs`, `trade_validation.rs`, `src/compat/*`, `ResearchPages.tsx`; `main.rs` additive-only. Guard:
  `python3 scripts/check_book_untouched.py --base origin/Manan` must report no problems.
- Every existing lab spec keeps producing identical trades (new fields default to today's behaviour). Existing tests stay green:
  engine `VAULT_IDX=<NAS>/idx_data.csv CARGO_BUILD_JOBS=8 cargo test --release` in `Context/guru/backend`; frontend
  `npx tsc --noEmit -p . && npx vitest run` in `Context/guru/frontend/react`.
- No look-ahead: a value used on bar i is known at bar i's close; a fill on bar j > i may use bar j's prices.
- Labels: results stay "simulated · not a book method"; never say "verified".
- No git write commands. No worktrees. Backend tasks run one at a time (shared build tree).

## Task 3 — Engine: AmiBroker trade prices, delays per side, trailing % of profit, re-entry delay per stop

Files: `Context/guru/backend/src/lab/rules.rs` (parse + `Exec`), `simulate.rs` (fills), any other `Fill` match sites
(`grep -rn "Fill::" src/lab`), tests beside them. Frontend types only if a test needs them (UI is Task 5).

1. **Price fields.** New optional `execution.prices` = `{ "buy": P, "sell": P, "short": P, "cover": P }` with
   P ∈ `"open" | "high" | "low" | "close" | "average"` (average = (O+H+L+C)/4), and optional `execution.delays` =
   `{ "buy": n, "sell": n, "short": n, "cover": n }`, n ∈ 0..=20. Semantics as AmiBroker Analysis settings › Trades: the order
   fills on bar `signal + delay` at that bar's chosen field. Delay 0 allows only `close` (a signal is known at the close) —
   any other field with delay 0 is refused `invalid` "… with delay 0 would use prices before the signal". When `prices`/`delays`
   are absent, behaviour is exactly today's `fill`/`exit_fill`/`delay_bars`; giving both the old and the new form is refused.
   Long entries use `buy`, long exits `sell`, short entries `short`, short exits `cover`. Stops/targets/limit orders keep their
   own fill rules (unchanged); the fields apply to market (rule-signal) orders only.
2. **Trailing as percent of profit.** `exit.trailing` accepts `{ "profit_pct": x }` (0 < x < 100): once the trade has open profit,
   the stop sits at `peak price − x % × (peak − entry)` for longs (mirror for shorts); no stop while there is no profit.
   Optimizable path `exit.trailing.profit_pct`.
3. **Re-entry delay per stop.** `execution.reentry` = `{ "stop_loss": n, "target": n, "trailing": n, "bars": n }` (bars after an exit
   of that kind before a new entry on the same side, AmiBroker "Re-entry delay"); combines with `cooldown` (the larger wait wins).
4. Tests: one per field and side (long and short), the delay-0 refusal, average price arithmetic, trailing-%-of-profit
   with no-profit and profit cases, re-entry per stop, and a regression test that a spec without the new fields gives the same
   trades as before (use existing fixtures).

## Task 4 — Frontend: Monte Carlo (from the trade list)

New `src/features/lab/monte-carlo.ts` (pure) + `MonteCarlo.tsx` + `monte-carlo.css` + tests; one line in `IndicatorsPage.tsx`
to render the panel after the Report for single-instrument runs. AmiBroker Monte Carlo tab and report:

- Inputs: number of runs (default 1000, max 5000), position sizing: don't change (compound trade returns as-is),
  fixed fraction % of equity (default 5 %… applies the trade % to that fraction), seed (default 1, deterministic PRNG, e.g. mulberry32).
- Each run samples the trade list with replacement (same count as the backtest) and builds an equity path from 1.0.
- Output table at percentiles 1, 5, 10, 25, 50, 75, 90, 95, 99: final equity, annual return (years = backtest years),
  max drawdown %, lowest equity. Charts: MC equity min/median/max envelope by trade number; CDF of max drawdown %; CDF of
  final equity (SVG, theme tokens only).
- Label: "Monte Carlo · resampled simulated trades · not a forecast". Runs in the browser; keep it under ~200 ms for 1000×1000.

## Task 5 — Frontend: AmiBroker-style settings tabs and scenario presets

Regroup the existing rule-builder execution controls (fills, delays, orders, stops, sizing, costs, re-entry) into tabs named after
AmiBroker Analysis settings: **General** (direction, periodicity note, initial equity / money mode, commissions), **Trades**
(scenario preset + buy/sell/short/cover price and delay from Task 3), **Stops** (max loss, profit target, trailing incl. % of
profit, N-bar via bars-held, re-entry delay per stop), **Portfolio** (existing portfolio controls). Scenario preset dropdown:
"Trade next bar on open, stops intraday", "Trade next bar on open, stops on next bar open", "Trade current bar on close, stops
intraday", "Trade current bar on close, stops on close", "Custom". The rule tree (entry/exit) stays as it is.

## Task 6 — Engine + UI: walk-forward (easy mode)

New `lab/walkforward.rs` and route `/api/{in|us|gl}/lab/walkforward` (gateway allow-list additive): in-sample length, out-of-sample
length, step (years or months), anchored flag, target metric (an optimize rank column). For each window: optimize on IS (existing
optimizer, same caps), take the best row, run it on OOS; return the window table (mode IS/OOS, start, end, params, metrics) and the
stitched OOS equity. Refuse when windows exceed the combo/time caps. UI: Walk-Forward tab in Optimization.

## Task 7 — Engine: sizing limits

`execution.limits` = min shares, round lot, min / max position value, max % of bar volume, interest on cash (% a year),
use previous bar equity. Money mode only; refuse in unit mode.
