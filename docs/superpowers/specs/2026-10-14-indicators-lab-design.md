# Indicators tab (Strategy Lab) — design

Status: draft for review · Branch: `Nandini` (from `Manan`) · Date: 2026-10-14

## 1. Goal

Add an **Indicators** tab where a researcher picks indicators, charts them on any admitted market,
combines them into entry/exit rules across timeframes, backtests and optimizes the rules, saves the
strategy to their account and submits it for **YT Strategies** (the company's own strategies).

Agreed decisions:

| Topic | Decision |
|---|---|
| Data in v1 | Only admitted data: daily bars, weekly/monthly resampled from daily. Futures and intraday are designed for and shown as "needs data". |
| Markets | India, US, Global, Crypto. Global and crypto files are TradingView exports: provisional, internal research only, not publishable to YT until licensed data replaces them. |
| Rule input | Form-based rule builder producing a JSON rule spec evaluated by the Rust engine. No user code runs on the server. |
| Run scope | One instrument per run. Basket scan and portfolio mode (max open positions, ranking score) are later phases. |
| Saving | Per user, in PostgreSQL via the auth/app service. |
| Indicator set | Target: TradingView's built-in list. Delivered in batches; each indicator ships only with a parity test. |
| Formula reference | TradingView `ta.*` / built-in definitions. |
| Tab shape | A workbench (library rail + chart + rule builder), not a Gurus-style catalogue. Results and optimization reuse the existing strategy and optimization presenters. |

Out of scope: live trading or orders, editing book strategies (Phase 2 "Customize"), portfolio mode,
futures and intraday until real data is admitted.

## 2. Constraints carried from AGENTS.md

- Work on `Nandini`; never commit or push to `main`; no force-push. Run `scripts/install-hooks.sh`.
- Pages in JSX/TSX under `Context/guru/frontend/react`, registered in `src/app/custom-pages.ts`.
- Existing book strategy calculations (`connors.rs`, `books/*`, admission files) are **not changed**.
  The lab gets its own indicator implementations; an indicator never alters a book result.
- Stock rules run on actual stocks from the basket catalogue; ETF rules on actual ETF prices; no index
  substitution. Futures stay blocked until contract data exists.
- Lab results are labelled "User-defined indicator strategy · simulated · not a book method"; never mixed
  with book verification, `claim` figures or gate verdicts.
- Data, `.env`, generated results stay outside Git; runtime data mounts read-only; bounded build/test
  concurrency (`CARGO_BUILD_JOBS=2`).
- Frontend changes: `npm run build` (typecheck), `npm run test:coverage`, real-browser desktop + mobile
  check on all themes (`data-theme` aqua, lagoon, glacier, … and the default).

### 2a. Book results are never touched (hard rule)

Every book strategy's catalogue, readiness, trades, statistics and optimization output must stay
**byte-identical** on `Nandini`. Enforced three ways:

1. **Protected files — no edits at all:** `backend/src/connors.rs`, `backend/src/books/**`,
   `readiness.rs`, `gate.rs`, `instrument_policy.rs`, `baskets.rs`, `accounting.rs`, `trade_validation.rs`;
   frontend `src/compat/runtime.js`, `src/compat/catalogue.js`, `src/pages/ResearchPages.tsx` and the existing
   strategy/optimization presenters. The lab calls shared functions (costs, validation, stats) read-only;
   if one needs a change, the lab gets its own copy instead.
2. **Additive-only shared files:** `main.rs` gains new `/api/lab/*` match arms and new functions only;
   no existing function, route or JSON field changes. Same for `gateway/src/policy.rs` (new allowlist
   entries only) and `custom-pages.ts` (new registration only).
3. **Automatic guard on every phase:** a check fails the phase if (a) the diff against `Manan` touches a
   protected path, or (b) the book regression hash differs: every enabled book strategy's
   summary/run/optimize JSON on the admitted data, hashed before and after (the method
   `publish_reviewed_batch.py` already uses). A phase cannot be proposed for merge with a failing guard.

Lab indicators are separate implementations even where a book uses the same indicator name (e.g. RSI),
so a TradingView-definition change can never shift a book result.

## 3. Architecture

```
Indicators tab (React)  src/pages/indicators/*  registered in custom-pages.ts (showInNavigation)
  │ apiJson
  ├─ /api/app/lab/strategies[...]            → auth/app service → PostgreSQL (saved specs)
  └─ /api/{in|us|gl}/lab/{catalogue,series,run,optimize}
        → gateway: session, allowlist, spec size + number bounds, optimize role + single slot
        → engine `lab` module:
             resample (D→W/M, exchange calendar) → indicators (TV definitions)
             → rules (JSON spec) → simulate (signal at close, fill next open, exits before entries)
             → trade_validation → accounting costs → stats / optimizer ranking (existing)
```

- **Engine:** new module `backend/src/lab/` (`mod.rs`, `resample.rs`, `indicators/*.rs`, `rules.rs`,
  `simulate.rs`). Reuses `Ctx` data loading, baskets, `trade_validation`, `accounting::Costs`, `stats()` and
  the optimizer's grid cap and percentile ranking. Routes added to `handle()` in `main.rs`.
- **Global and crypto:** a third engine instance `engine-gl` (same image) with the global/crypto files mounted
  read-only; gateway market list `in|us|gl`, crypto served by `engine-gl`. Staged by `setup.py`.
- **Instrument metadata** per series: market, currency, timezone, calendar, bars/year, type
  (index / etf / stock / cfd / composite / crypto / futures), feed provenance, total-return vs price,
  has-volume, provisional flag, tradable-from date.
- **Gateway:** add `lab/*` paths to `allowed_engine_path`; `lab/optimize` to `is_optimization_path`;
  bound the spec (max 12 conditions, numeric limits, max combinations 50,000).
- **Auth/app:** table `lab_strategy(id, owner, name, spec jsonb, market, instrument, status, version,
  created_at, updated_at)`; statuses `draft → submitted → published | rejected`; publish is admin-only;
  published versions are frozen. Routes under `/api/app/lab/strategies`.

## 4. Rule spec (JSON, versioned)

```json
{ "v": 1, "direction": "long",
  "entry": { "all": [
    { "lhs": {"s":"rsi","p":{"len":2},"tf":"D"}, "op":"crosses_below", "rhs": {"value":10} },
    { "lhs": {"s":"close","tf":"D"}, "op":">", "rhs": {"s":"sma","p":{"len":40},"tf":"W"} } ] },
  "exit": { "any": [
    { "lhs": {"s":"rsi","p":{"len":2},"tf":"D"}, "op":"crosses_above", "rhs": {"value":70} },
    { "lhs": {"s":"bars_held"}, "op":">=", "rhs": {"value":10} } ] },
  "execution": { "fill": "next_open", "cost_bps": 12 },
  "optimize": [ {"path":"entry.all[0].lhs.p.len","from":2,"to":6,"step":1} ] }
```

- Operators: `crosses_above` (a>b and a[1]<=b[1]), `crosses_below`, `>`, `<`, `>=`, `<=`.
- Operand: series id, params, timeframe (`D|W|M`), optional bar offset `[n]` (n ≥ 0 only — no future).
- Higher-timeframe values become usable on the last daily bar of that week/month, after its close
  (AmiBroker expandLast; TradingView `expr[1]` + lookahead). A test proves no lookahead.
- Designed-for fields (rejected as `needs_data` / `unsupported` in v1): `short`, limit/stop entries,
  stop-loss / target / trailing exits, scale-in legs, portfolio settings. Book "Customize" (Phase 2)
  needs these, so the spec reserves them now.

## 5. Indicator catalogue

Every indicator has: id, name, category, params with bounds, outputs, placement (price/pane), warm-up
rule, data needs (volume / intraday), TradingView definition notes, and a parity fixture.

- **Batch A (v1, ~60):** SMA, EMA, WMA, RMA, VWMA, HMA, ALMA, DEMA, TEMA, SWMA, LinReg, KAMA; RSI, MACD,
  Stochastic, Stoch RSI, CCI, Momentum, ROC, Williams %R, MFI, TSI, CMO, Ultimate Osc, Awesome Osc, PPO,
  TRIX, KST, DPO, Coppock, Fisher, Connors RSI; ADX/DMI, Supertrend, Parabolic SAR, Aroon, Ichimoku, Vortex,
  Choppiness; ATR, Bollinger (+%B, width), Keltner (+width), Donchian, StdDev, Historical volatility;
  OBV, Acc/Dist, CMF, Chaikin Osc, PVT, Volume Osc, Elder Force, Ease of Movement; Highest, Lowest, Change,
  Pivots (confirmed late), Bars since.
- **Batch B:** remaining TradingView built-ins computable from daily OHLCV (e.g. McGinley, Chande Kroll stop,
  Klinger, Mass index, RVI, SMI, Balance of Power, Price channel, Envelope, Zig Zag, Williams Alligator /
  Fractals, Pivot points standard, candlestick patterns).
- **Listed as `needs data`:** VWAP / anchored VWAP, volume profile, opening range (OPEN-n high/low), anything
  intraday, breadth, fundamentals.

Verified on TradingView itself on 2026-10-14 (Pine v6 probe, NSE:SWIGGY daily from its first listed bar,
475 bars): `ta.ema` seeds with the SMA of the first n values (0 differences vs SMA seed, 42 vs first-value
seed); first non-na bar: EMA5 4, RSI14 14, ATR14 13, DI+ 14, ADX 27, MFI14 13, CMO9 9, OBV 1, HMA9 10,
MACD 25 / signal 33, Stoch RSI %K 17 (partial-window rule), SAR 1, Supertrend(10,3) 9 (bar 0 shows a 0
artefact). Parabolic SAR's initial direction follows **highs** (high[1] > high[0] → long, SAR = low[0]),
not closes as the reference example states: bars O/H/L/C 420/465.8/391/456, 473.15/489.4/418.6/430.7,
416.1/438.7/410.4/420.1, 424/430.9/408.85/412.45 give SAR na, 391, 391, 392.968 (clip to the last two lows).
Still to settle by parity fixtures: chart vs `ta.*` variants (Keltner, TSI ×100, TRIX, DPO), na-window
rules on derived series, highest/lowest tie rules, Heikin Ashi seed.

## 6. UI (match the current theme)

- Uses only existing tokens (`--bg --panel --card --ink --ink2 --ink3 --steel --rule --blue --green --red`,
  `--chart`, `--bh`) and existing classes (`.seg`, `.sel`, `.chip`, `.card`, `.tbl`, `.lab`); square steel
  cards, Archivo + IBM Plex Mono. Works under every `data-theme` and the theme switcher.
- Layout: library rail (search, groups, tags chart / engine / needs volume / needs intraday) | toolbar
  (market, instrument with type badge, D/W/M, range) | chart (Lightweight Charts already vendored; overlays
  and panes; trade shading) | rule builder | results.
- Results page and optimization page reuse the existing strategy/optimization presenters: KPI strip,
  equity, drawdown, calendar/monthly returns, tradesheet, Excel export, top-3 percentile ranking. The
  "before/after book" split becomes a user-chosen in-sample / out-of-sample split; a "variants tried"
  counter is shown.
- Bonus: indicator overlays on existing strategy charts ("overlays awaiting implementation") use the same
  library — separate follow-up, display only.
- Reference mockup: the published Indicator Lab artifact (sample data only).

## 7. Error handling

`needs_data` (missing series, volume, intraday, futures, outside coverage), `unsupported` (reserved spec
fields), `invalid` (bounds) are returned as typed results; the UI shows the reason and no numbers. Engine
never panics on a spec: unknown ids or params → 400 JSON error. Optimizer combination limits enforced in
gateway and engine.

## 8. Testing and verification

- **Rust:** unit tests per indicator against TradingView parity fixtures (CSV exported from a TradingView
  chart on the same OHLC, compared at 1e-9 relative tolerance after warm-up); resample tests (week = exchange
  calendar, partial last bar); no-lookahead test for W/M operands; rule operator tests; simulate tests
  (next-open fills, exits before entries, gap behaviour); spec validation tests. Run with `CARGO_BUILD_JOBS=2`.
- **Gateway/auth:** policy tests for new paths and bounds; contract tests for lab strategy routes and roles.
- **React:** vitest for rule-spec builder, catalogue rendering, needs-data states; typecheck; real-browser
  check desktop + mobile across themes.
- Existing book strategy results must be byte-identical before/after (regression hash, as the publish
  script already does).

## 9. Delivery flow

| Phase | Content | Exit criteria |
|---|---|---|
| 0 | Branch `Nandini`, hooks, this spec reviewed | Spec approved |
| 1 | Engine: resample + Batch A indicators + parity tests | All Batch A pass parity; no book output change |
| 2 | Engine: rules + simulate + `lab/*` routes + gateway policy | Spec round-trip tests, no-lookahead test |
| 3 | Frontend: Indicators page (library, chart, rule builder) on India/US | build + tests + browser check |
| 4 | Results/optimize reuse + saved strategies (auth/app + DB migration) + submit to YT | role tests |
| 5 | `engine-gl` for global/crypto with provisional labels | setup + compose verified on an isolated stack |
| 6 | Batch B indicators | parity per indicator |
| later | Customize book strategies; basket scan; portfolio mode; futures; intraday | own specs |

Each phase is one reviewed pull request from `Nandini` into `Manan` (never into `main`). Multi-agent
implementation per phase: one agent per independent unit (e.g. an indicator family), each writing its parity
test first, then an independent reviewer agent, then the full checks.

## 10. Open questions

- TradingView export terms for parity fixtures and global data (owner/legal).
- Who approves YT submissions (admin only assumed).
- Where the intraday 1-minute data and `build_series.py` live (needed later for opening-range indicators).
