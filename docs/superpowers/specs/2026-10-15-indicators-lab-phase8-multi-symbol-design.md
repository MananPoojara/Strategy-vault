# Indicators lab Phase 8 "Multi-symbol and portfolio rules" (design)

Status: draft for the user's review · Branch: `Nandini` (from `Manan`) · Date: 2026-10-15
Parent specs: `2026-10-14-indicators-lab-design.md` (§1 "portfolio mode later", §4 reserved `portfolio`),
`2026-10-16-customize-book-strategies-design.md` (§5 bar event order) and
`2026-10-15-indicators-lab-phase7-rules-ii-design.md` (Phase 7, executed **before** this phase; §5.1 reserves the
operand field `symbol` for Phase 8, §7 money mode and execution models, §12 item 3 lists this phase).
Plan: `docs/superpowers/plans/2026-10-15-indicators-lab-phase8-multi-symbol.md`.

## 1. Goal

The user asked: *"add features of foreign filter, foreign entry/exit, ranking, position sizing, higher timeframe —
basically we need to replicate AmiBroker, not its codebase: whatever rules the user defines should happen"*.
The user could not be asked questions, so every open point is decided here and listed in §14.

Phase 8 lets a lab rule spec read **other instruments** and trade **many instruments with one shared capital**,
following AmiBroker's *documented* behaviour (guide and AFL reference pages, §15; nothing is decompiled):

1. **Foreign operands** (AmiBroker `Foreign()`, `SetForeign()` / `RestorePriceArrays()`): any series operand may
   read another admitted instrument of the same market, price or indicator, on any timeframe, aligned by date.
   Market filters ("trade only while NIFTY 50 is above its 200-day SMA") are plain conditions on such operands.
   The four listed second-instrument indicators (`correlation`, `correlation_log`, `ratio`, `spread`) become
   computed.
2. **Foreign entry and exit**: signals computed on instrument A, trades executed on instrument B (index signal →
   ETF execution). An index is never the executed instrument in money or portfolio mode (Phase 7 §7.2 rule).
3. **Portfolio backtests** (AmiBroker portfolio backtester, `PositionScore`, `MaxOpenPositions`,
   `SetBacktestMode(backtestRotational)`, `WorstRankHeld`, `SetPositionSize`): a universe of admitted stocks/ETFs,
   one shared money ledger, rank-based entries when signals exceed free slots, rotational mode with periodic
   rebalancing, portfolio-level position sizing and costs.
4. **Higher timeframes across symbols**: a foreign operand may carry Phase 7's `tf` and `expand`; the composition is
   lookahead-free.
5. **Position sizing**: Phase 7 already covers single-instrument sizing (`execution.sizing`, money mode). Phase 8
   applies the same fields at portfolio level against shared equity and cash.

Out of scope (named in §13): portfolio shorts, long/short mixed portfolios (`SeparateLongShortRank`), AmiBroker's
`backtestRegular` signal pre-cleaning across skipped entries, scale-in/scale-out and resting orders inside a
portfolio, rebalancing the size of held positions, universes above 120 members, cross-market references,
historical index membership, intraday bars, futures in portfolios.

## 2. Constraints carried from AGENTS.md and the earlier specs

- **Book results are never touched.** No edits to `connors.rs`, `books/**`, `readiness.rs`, `gate.rs`,
  `instrument_policy.rs`, `baskets.rs`, `accounting.rs`, `trade_validation.rs`, frontend `compat/*`,
  `ResearchPages.tsx`. `main.rs`, gateway `policy.rs` and `custom-pages.ts` are additive-only, and **this phase
  changes none of them**: every engine route is already dispatched by the `/api/lab/` prefix
  (`main.rs`: `p if p.starts_with("/api/lab/") => lab::api::serve(…)`), and the portfolio uses the existing
  `lab/run` and `lab/optimize` paths, which the gateway already allowlists. Book functions are only called
  (`select_context`, `resolve_context`, `stats`, `conclude_rank_with_outcomes`, `Catalogue::tickers_json`,
  `Catalogue::rows` through `SelectedContext`). `scripts/check_book_untouched.py --engine-compare` must pass.
- **Every existing spec keeps its exact meaning.** A spec without `symbol`, `vs`, `fixup` or `portfolio` compiles
  through the unchanged Phase 7 code path; Phase 7's pinned-output hash test (its §8 proof 3) and the 43 Customize
  templates' fidelity test stay green after every task.
- **Keep four things separate.** Documented-example fixtures (§12.4) reproduce AmiBroker *semantics* on crafted
  bars; they verify nothing about a book or a published figure. Portfolio results are simulated; the response
  labels the survivorship bias of the universe (§6.9).
- **No result where rules or data are unresolved.** Missing foreign data is `NaN` (a condition on it is false),
  never forward-filled unless the user asks for `fixup` and the response says so. A universe member without
  data in the window is listed as refused with its reason. Cross-market references, gl data in an India/US
  strategy, futures and portfolio shorts answer `needs_data` / `unsupported`.
- **Stock methods require actual stocks.** A portfolio universe holds actual stocks (basket members) or ETFs with
  their own prices. An index can be a *signal* (foreign operand, benchmark) but never a universe member and never
  the executed instrument in money mode.
- **Provisional markets never publishable.** gl/crypto stay unpublishable (Phase 4). Portfolio strategies are not
  publishable either until dated index membership is admitted (§11).
- **Bounded compute.** Universe ≤ 120 members, explicit lists ≤ 50, ≤ 4 foreign instruments per spec, portfolio
  optimize ≤ 200 combinations and ≤ 12,000 member-combinations, wall-clock limits (§6.10), the gateway's single
  optimize slot. No backtest at startup; no whole-corpus run.
- **Working rules.** Branch `Nandini`; never `main`; never push (the user pushes); no worktrees; no
  `Co-Authored-By` trailer; `CARGO_BUILD_JOBS=2`; std-only Rust (no new crates); the NAS and the basket roots are
  read-only; no data, `.env` or results in Git; every lab body keeps the lab `label`.

## 3. What exists (HEAD `8136de0` at writing; Phase 7 lands first)

### 3.1 Code

- **Rule spec** (`lab/rules.rs`): v1 + Customize fields; Phase 7 adds nodes, expressions, `tf` Q/Y/`nD`, `expand`
  modes, two-sided specs, orders, money mode (`execution.capital`, `execution.sizing`, `execution.costs`) and an
  execution model per instrument kind. Phase 7 reserves the operand field **`symbol`** ("operands from another
  symbol … arrive in Phase 8", answered `unsupported`) and keeps `spec.portfolio` reserved (`unsupported`).
- **Series resolution** (`lab/series.rs`): every operand goes through one function, `series::series(fr, cache, s)`,
  which reads only the run instrument's `Frames` (daily bars + W/M, and Phase 7's Q/Y/`nD`). Higher-timeframe
  values reach daily bars through `resample::expand` (AmiBroker `expandLast`). The indicator cache key already
  fingerprints the instrument (length, first/last date, first/last close).
- **Simulator** (`lab/simulate.rs`): one position at a time; book-format `connors::Trade`. Phase 7 extends it
  (orders, intrabar modes, money ledger in `lab/ledger.rs`).
- **API** (`lab/api.rs`): `/api/lab/{catalogue,series,run,optimize,instruments,templates,template,fidelity}`;
  `select_context` loads one instrument per request; baskets are read lazily per symbol
  (`baskets::Catalogue::rows`, SHA-256 checked on every read); each engine instance serves **one market**
  (`engine-in`, `engine-us`, `engine-gl`).
- **Catalogue**: 299 entries; `correlation`, `correlation_log`, `ratio`, `spread` are listed with
  `needs.second_symbol` and the note "not available yet — needs a second instrument in the same request".
- **Gateway** (`webapp/gateway/src/policy.rs`, `main.rs`): engine routes GET-only; query ≤ 16,384 bytes; lab spec
  ≤ 8,192 bytes and ≤ 12 top-level conditions; `lab/optimize` single slot (research roles), 600 s; other engine
  requests 120 s and 4 concurrent reads.
- **Book engine baskets (read-only)**: `baskets.rs` admits basket members by a whitelist `index.csv` with typed
  evidence (`trusted_market`, `asset_type=individual_stock`, `membership_kind` = `current_snapshot` or
  `current_plus_past_snapshot`, `historical_membership_verified:false`). Qualified keys are
  `basket:<basket>:<symbol>`. The book engine never runs a portfolio; it runs one basket member per request.
  Extra series (`extra_data/<market>/…`) are aligned **by date with NaN on missing dates** (`main.rs` `ctxs`
  construction) — the same no-fill convention this phase adopts.

### 3.2 Data found (read-only, `ls`/`head`, 2026-10-15)

| Universe | Where | Members | Membership kind | Quotes | Usable in Phase 8 |
|---|---|---|---|---|---|
| Nifty 50 | India basket cache (`VAULT_BASKET_DATA_ROOT`, default `/home/server/.cache/strategy-vault/market-data`; documented in NAS `library/market-data/README.md`) | 50 | current snapshot (`Current Nifty.tls`) | to 2026-10-01 | yes |
| Midcap 100 | same | 100 | official current CSV (2026-10-08), not dated | to 2026-10-01 | yes |
| All F&O cash stocks | same | 224 | user-selected current watchlist | to 2026-10-01 | as an explicit list ≤ 50 only (cap §6.10) |
| NSE 500 | same | 499 | current watchlist | to 2026-10-01 | as an explicit list ≤ 50 only |
| S&P 500 current / current+past | US basket cache (`VAULT_US_BASKET_DATA_ROOT`; NAS `library/us-baskets/README.md`) | 505 / 1,802 | current / union of current and past, **not dated** | to 2020-09-11 | explicit list ≤ 50 only |
| NASDAQ 100 current / current+past | same | 103 / 395 | as above | to 2020-09-04 | current: yes; past: list only |
| Dow 30 current / current+past | same | 30 / 72 | as above | to 2020-09-04 | yes |
| Russell 2000 current / current+past | same | 1,994 / 10,002 | as above | to 2020-09-04 | explicit list ≤ 50 only |
| US ETFs | engine `ctxs` (SPY, QQQ, IWM, MDY, SMH, DIA) | 6 | — | engine data | yes (members, foreign-entry targets) |
| Indices | India `idx_data.csv` (NIFTY 50, NIFTYBANK, NIFTYIT, NIFTYMIDCAP100, NIFTYMIDCAPSELECT); US SPX/NDX/DJI | — | — | — | foreign signals and benchmark only |
| Volatility | `india_vix/` (INDIA VIX), CBOE VIX (US engine `st.vix`) | — | — | — | foreign signals only |
| gl / crypto | `Global Bhavcopy/` (Phase 5, provisional) | 10 | — | — | foreign signals inside `engine-gl` only |

**No dated (point-in-time) index membership exists anywhere** (both READMEs say so; the engine refuses a
`point_in_time` membership claim). Every portfolio result therefore carries survivor-selection bias (§6.9). The
basket caches exist on the live server only, not on this workstation; tests use synthetic baskets
(`baskets.rs`'s own fixture pattern) and an optional real-data test runs when `VAULT_BASKET_DATA_DIR` is set.

## 4. Foreign operands (AmiBroker `Foreign` / `SetForeign`)

### 4.1 Grammar (additive; Phase 7's reserved field made real)

Any series operand — top-level, inside a Phase 7 node or expression, or inside `compress` — accepts:

| Field | Meaning | AmiBroker / TradingView |
|---|---|---|
| `"symbol": "<ticker>"` | compute this operand on another instrument of the same market and align it to the run instrument's dates (§4.3) | `Foreign(ticker, field)`; `SetForeign(ticker)` … `RestorePriceArrays()`; Pine `request.security(ticker, …)` |
| `"fixup": "none" \| "previous"` | `previous` fills a missing date from the latest earlier bar of that instrument (only with `symbol`; default `none`) | `Foreign(…, fixup)`: 0 = holes stay Null, 1 (AmiBroker's default) = fill from the previous bar |
| `"vs": "<ticker>"` | the second instrument of a pair indicator (`correlation`, `correlation_log`, `ratio`, `spread`; required there, refused elsewhere; tf D only) | TradingView "Symbol" input of those indicators; AmiBroker `Correl(C, Foreign(t,"C"), n)`, `RelStrength(t)` |

Examples:

```json
{"lhs":{"s":"close","symbol":"NIFTY 50"},"op":">","rhs":{"s":"sma","p":{"length":200},"symbol":"NIFTY 50"}}
{"lhs":{"s":"close","symbol":"INDIA VIX"},"op":"<","rhs":{"value":20}}
{"lhs":{"s":"rsi","p":{"length":2},"symbol":"SPX","tf":"W","offset":1,"expand":"first"},"op":"<","rhs":{"value":10}}
{"lhs":{"s":"ratio","vs":"NIFTY 50"},"op":"crosses_above","rhs":{"s":"sma","p":{"length":50},"symbol":"NIFTY 50"}}
{"lhs":{"s":"correlation","p":{"length":60},"vs":"SPX"},"op":"<","rhs":{"value":0.3}}
```

Rules:

- `symbol` names any instrument this market's engine can select: preloaded indices/ETFs (`NIFTY 50`, `SPX`, `SPY`
  …), basket members (`basket:nifty50:TCS`), and the engine's volatility index (`INDIA VIX` on `engine-in`,
  `CBOE VIX` on `engine-us`). On `engine-gl`, the ten gl ids.
- At most **4** distinct other instruments per spec (`symbol` and `vs` together). Each is loaded once per
  request (and shared read-only by optimizer threads).
- `symbol` equal to the run instrument is allowed and means the same series.
- `bars_held` takes none of these fields. A `symbol` operand inside a portfolio `score` is refused (a score must
  differ by member); `vs` is allowed there (relative strength ranking).
- Unknown ticker → `invalid` naming the field ("entry.all[0].lhs.symbol: unknown ticker X on this market").
  A gl id on `engine-in`/`engine-us`, or an India/US ticker on `engine-gl` → `needs_data` ("X is served by
  /api/gl; one strategy reads one market's engine, and provisional gl data is never mixed into an India/US
  strategy"). A ticker of the *other* India/US market → `needs_data` ("cross-market references are not
  available").
- The operand's `p`, `out`, `tf`, `offset`, `expand` (Phase 7) keep their meaning **on the other instrument's own
  bars**: indicators run on its own calendar, `offset` counts its own bars of the operand's timeframe, and
  `tf`/`expand` resample and expand on its own daily calendar. The result is then aligned (§4.3).
- Phase 7's `compress` expression applied to a foreign daily operand compresses the **aligned** array on the run
  instrument's calendar — AmiBroker's `TimeFrameCompress(Foreign(…))`. Both forms are lookahead-free (§4.4); the
  catalogue notes the difference.

### 4.2 Execution stays on the run instrument (foreign entry and exit)

`SetForeign(A)` … signals … `RestorePriceArrays()` in AmiBroker computes signals on A and trades the current
symbol. In the lab the **run instrument** (`ticker`, or each portfolio member) is always the traded one, and
every operand with `symbol: A` is a signal input. So "signal on A, execute on B" is a run on B whose conditions
read A. Fills, stops, targets, trailing levels, limit/stop entry levels and ATR distances all use **B's own
prices** (AmiBroker's `ApplyStop` and `BuyPrice` act on the current symbol after `RestorePriceArrays`).

Execution admission follows Phase 7 §7: the executed instrument's model decides. An index or a CFD run stays a
unit-mode **study** (`executable:false`, Phase 7 label); money mode, portfolio mode and futures on an index answer
`needs_data`. So an index is never the executed instrument of an executable result; it may be the signal source.
The run response lists every foreign instrument (`foreign`, §10).

### 4.3 Date alignment

AmiBroker `Foreign()` "synchronizes the data file you are referencing with the currently selected symbol" and
removes bars that exist only in the foreign symbol; with `fixup=0` holes stay Null. The lab does the same, with
one extra rule for instruments that close at different times:

| Pair | Rule | Value on run bar *t* |
|---|---|---|
| same session: every India pair on `engine-in` (NSE stocks, indices, INDIA VIX); every US pair on `engine-us` except CBOE VIX; gl pairs with the same timezone and calendar | `same_date` | the other instrument's value dated *t*; **NaN** if it has no bar on *t* |
| the other instrument closes later than the run instrument on the same date: CBOE VIX (settles ~16:15 ET vs NYSE 16:00) when any order of the spec fills at the signal bar's close; every gl pair with a different timezone or calendar (HSI vs DAX CFD, crypto UTC days vs session markets) | `prior_date` | the other instrument's latest value dated **before** *t* and **on or after** the run instrument's previous bar date; NaN otherwise (and on the first run bar) |
| CBOE VIX when every fill is at a later open (next-open market orders, resting orders for the next bar) | `same_date` | the VIX close on *t* is known before the next open |

`fixup: previous` drops the lower bound: the latest value dated on/before *t* (`same_date`) or before *t*
(`prior_date`). It only fills **missing dates**; a NaN the indicator itself produces (warm-up, sparse outputs such
as fractals) is never filled. The response marks each foreign instrument's `alignment` and `fixup`.

Values never come from a bar dated after *t*, so alignment cannot look ahead (§12.1).

### 4.4 Higher timeframe across symbols (no lookahead)

A foreign W/M/Q/Y/`nD` operand is resampled from the other instrument's daily bars, computed there, expanded onto
**its** daily calendar with Phase 7's `expand` mode (last, point, or lookahead-free first), shifted by its own
`offset`, and only then aligned to the run instrument by §4.3. Each step uses data dated on or before the run
bar (`same_date`) or strictly before it (`prior_date`), so the composition is causal. Phase 7's refusal of
look-ahead `expandFirst` applies unchanged. Example: `{"s":"close","symbol":"NIFTY 50","tf":"W"}` shows the
index's weekly close from the last NSE session of the week; a stock that trades that Friday sees it at its Friday
close, as AmiBroker's `TimeFrameSet(inWeekly)` + `SetForeign` + `TimeFrameExpand(…, expandLast)` would.

### 4.5 Second-instrument indicators (now computed)

`Bars` gains `second: Option<Vec<f64>>` — the `vs` instrument's close aligned to the base instrument's daily bars
(§4.3, `fixup` none). Definitions (declared; TradingView parity fixtures pending as for every Phase 6 indicator):

| id | Output | Definition | first (default) |
|---|---|---|---|
| `ratio` | value | close ÷ vs close (vs close 0 → NaN) | 0 |
| `spread` | value | close − vs close | 0 |
| `correlation` | value | Pearson correlation of close and vs close over `length` bars (default 20), bar-counted: a NaN anywhere in the window → NaN; zero variance → NaN | 19 |
| `correlation_log` | value | the same over log returns ln(c/c[1]) of both series (non-positive price → NaN) | 20 |

They run on daily bars only (`tf` D); the base instrument is the run instrument, or the operand's `symbol`.
The catalogue moves them from "listed" to "computed" (299 entries: 187 computed + 112 listed) and their entries
keep `needs.second_symbol: true`, which tells the UI to ask for `vs`.

## 5. Universe and calendar of a portfolio

### 5.1 Universe

```json
"portfolio": { "universe": { "basket": "nifty50" } | { "tickers": ["basket:nifty50:TCS", "basket:nifty50:INFY", "…"] }, … }
```

- `basket` takes a basket id of this engine's catalogue (`nifty50`, `midcap100`, `nse500`, `allfo`, `us_sp500`,
  `us_sp500_past`, `us_nasdaq100`, `us_nasdaq100_past`, `us_dow30`, `us_dow30_past`, `us_russell2000`,
  `us_russell2000_past`). Members are every whitelist row of that basket, read through the public
  `Catalogue::tickers_json()` (no edit to `baskets.rs`).
- `tickers` takes 2–50 distinct qualified basket members and/or ETFs of this market. Indices, VIX, gl ids and
  unknown tickers are refused by name.
- Universe size after expansion: **≤ 120** (`nifty50`, `midcap100`, `us_nasdaq100`, `us_dow30`, `us_dow30_past`, ETF
  lists). A larger basket answers `invalid` "<basket> has N members; the portfolio limit is 120 — choose a smaller
  basket or a list of at most 50 members". (Reason and cost: §14 item 12.)
- Member admission (per member, never fatal for the whole run): asset type `individual_stock` or `etf`;
  non-empty overlap with the window; no invalid bar inside the window (the lab's `bars_clean` rule);
  indicator data present (e.g. volume for volume indicators). A refused member is listed in
  `universe.refused: [{ticker, reason}]`. Zero admitted members → `needs_data`.

### 5.2 Calendar

The portfolio steps through the **reference calendar** of the market — the dates of `NIFTY 50` on `engine-in` and
`SPX` on `engine-us` (AmiBroker's "Pad and align all data to reference symbol", which it documents as an option;
the lab makes the reference explicit). A member bar on a date missing from the reference calendar is not used.
On a reference date where a member has no bar (not yet listed, suspended, delisted, data ended), that member
generates no signal and no fill; an open position is marked at its last known close.

Default window: the reference coverage intersected with [earliest member first bar, latest member last bar], so
a US run does not run years past the 2020 data end. `from`/`to` must lie inside the reference coverage
(`needs_data` otherwise). The index's stale-open cut (`gate::real_open_from`) does not apply: fills use member
prices.

### 5.3 Data end

When a member's data ends inside the window while a position is open, the position is closed at that member's
last close, reason `data_end` ("coverage ends — delisting or data cut; exit at the last available close"). This is
an honest stand-in for an unknown delisting outcome and is counted in `events`.

## 6. Portfolio mode (AmiBroker portfolio backtester)

### 6.1 Spec

```json
{ "v": 1, "direction": "long",
  "entry": { "all": [nodes] }, "exit": { "any": [nodes], "stop_loss": D, "target": D, "trailing": … },
  "execution": { "fill": "next_open | close", "exit_fill": "…", "capital": 1000000,
                 "sizing": { "pct_equity": 10 } | { "risk_pct": 1 } | { "units": 50 } | { "value": 100000 },
                 "costs": { … Phase 7 §7.4 … } },
  "portfolio": {
    "universe": { "basket": "nifty50" },
    "mode": "signals | rotation",
    "score": operand,                      // PositionScore; required for rotation, optional for signals
    "rank": "highest | lowest",            // default highest
    "max_open": 10,                        // MaxOpenPositions, 1..50
    "worst_rank_held": 15,                 // rotation only; max_open..120; default = max_open
    "rebalance": "bar | W | M | Q | Y"     // rotation only; default "bar"
  } }
```

| Field | AmiBroker documented equivalent |
|---|---|
| `mode: signals` | regular, signal-based portfolio backtest; buy/sell signals per symbol |
| `mode: rotation` | `SetBacktestMode(backtestRotational)` (formerly `EnableRotationalTrading()`): "uses only position score"; buy/sell signals are ignored |
| `score` | `PositionScore` |
| `rank: lowest` | the AFL idiom `PositionScore = 100 - RSI(…)` ("the stock with lower RSI will be preferred") without the sign change that means "short" in rotational mode |
| `max_open` | `SetOption("MaxOpenPositions", n)` |
| `worst_rank_held` | `SetOption("WorstRankHeld", n)` |
| `rebalance: W/M/Q/Y` | `PositionScore = scoreNoRotate` on every bar except the period's last bar ("already open trades should be kept and no new trades entered") |
| `execution.capital` | `SetOption("InitialEquity", x)` (money mode is mandatory in a portfolio) |
| `execution.sizing` | `SetPositionSize(size, spsPercentOfEquity / spsValue / spsShares)`; `risk_pct` = the KB risk-based sizing formula |
| `execution.fill` | Settings "Trades" prices and delays (`BuyPrice`/`SellPrice`, `SetTradeDelays`): close = delay 0 at close, next_open = delay 1 at open |

Allowed with `portfolio` (everything else answers `unsupported`, naming the field): `direction: long`; market
entries (`execution.fill` next_open | close, Phase 7 `delay_bars` 1); Phase 7 `entry_on` level | edge; rule exits
(`exit.any`, nodes, `bars_held`, `min_bars`); price exits (`stop_loss`, `target`, `trailing` incl. `activate`,
`breakeven`) with Phase 7's `intrabar` mode; Phase 7 money sizing and costs. Refused in a portfolio: `short` /
`both`, `entry.limit`, `entry.stop`, `scale_in`, `take_profits`, `cooldown`, `max_trades`, `risk`, futures,
`delay_bars` ≠ 1, the bps `cost` query (Phase 7 money rule), a `ticker` query (the members come from the
universe), and `split` on `lab/run`.

`mode: rotation` additionally requires `exit.any: []` and no price exit (AmiBroker rotational exits are rank
exits); `entry.all` may be empty and acts as an **eligibility filter** (the AFL idiom
`PositionScore = IIf(filter, score, 0)`, where AmiBroker treats a score of 0 as "no trade" and exits a held
position; in the lab, ineligibility plays that role, §14 item 17).

### 6.2 Bar event order (portfolio)

On reference bar *i* (members without a bar on *i* skip every step that needs a price):

1. **Open.**
   1. Pending next-open exits fill at each member's open (exits before entries, as in every earlier lab phase).
   2. Pending next-open entries fill **in rank order** at each member's open, sized as in §6.4, while slots and
      cash allow; an entry whose member has no bar on *i* is cancelled (`skipped.no_bar`).
2. **Inside the bar** (signals mode): each held member's price exits, by Phase 7's per-position rules and
   `intrabar` mode (gap through a level fills at the open; stop before target; levels from closed bars).
3. **Close.**
   1. Data-end exits (§5.3).
   2. Rule exits (signals mode) or rank exits (rotation, on rebalance bars) fill at this close or set a next-open
      exit.
   3. Candidate entries are ranked (§6.3) and either fill at this close in rank order (close fills) or become
      pending next-open entries (all candidates are queued in rank order; slots and cash are checked at the fill).
   4. Equity is marked: cash + Σ member positions at their close (last known close if no bar today).

Freed slots and cash from exits on a bar are usable by entries on the same bar (exits first). For **one member**,
re-entry after an exit follows the single-instrument rule (Customize §5.1, Phase 7 §6.2): next-open market mode may
share the open; with close fills there is no new entry on a bar where that member exited after its open.

### 6.3 Ranking and ties (deterministic)

Candidates on a bar are flat members with a bar on *i*, whose entry rules are true (signals mode) or which are
eligible and not held (rotation), and — when `score` is given — whose score is a finite number. Order:

1. effective score descending (`rank: highest` → score; `lowest` → −score). In signals mode without `score`, all
   tie;
2. requested position value descending (AmiBroker KB 2015-09-28: "transaction with greater PositionSize is
   preferred" — the sizing rule's own measure: equity percent, value or units);
3. ticker ascending (byte order; AmiBroker: "alphabetical order");
4. (long before short — not reachable; portfolios are long-only.)

The universe's listed order never matters; the same request always gives the same trades (§12.3).

**Signals mode.** Free slots = `max_open` − open positions + positions with a pending next-open exit. Candidates
are taken in order; one that cannot be funded with at least one unit is skipped (`skipped.no_cash`) and the next
is tried; once slots are full the rest are counted (`skipped.no_slot`). A skipped signal is not queued for
retry; with `entry_on: level` the member is a candidate again on the next bar where its rules are still true
(AmiBroker `backtestRegularRaw` behaviour; §14 item 9).

**Rotation mode** (on rebalance bars only). All eligible members with a finite score are ranked 1..n by the
order above. A held member whose rank is worse than `worst_rank_held`, or which is ineligible or has no finite
score, exits ("rank exit"). Then the free slots are filled from the best-ranked members not held, in order.
A held member without a bar on the rebalance date is kept (no rank, no price) until its next bar. Between
rebalance bars nothing rotates. `rebalance: bar` (the default, as AmiBroker) rotates on every bar;
W/M/Q/Y rotate on the last reference bar of each calendar period (`resample::period_key`), which uses the same
calendar knowledge as the lab's W/M operands; the final, unfinished period of the window does not rebalance.

### 6.4 Sizing and the shared ledger (money mode)

Phase 7 money-mode rules apply per position, with the portfolio's shared equity and cash:

- Size is computed at the **signal close** from portfolio equity at that close (cash + open positions marked at
  the close; AmiBroker defines "portfolio equity is equal to available cash plus sum of all simultaneously open
  positions") and the member's signal close price: `pct_equity` p → p% of equity; `value` v → v currency;
  `units` n → n shares; `risk_pct` r → r% of equity ÷ stop distance (needs `exit.stop_loss`). Default when
  `execution.sizing` is absent: `pct_equity` = 100 / `max_open` (declared, §14 item 15).
- Whole shares, rounded down. At the fill the quantity shrinks to the **available cash** after this bar's earlier
  fills and their costs (AmiBroker "Allow position size shrinking"; Phase 7 rule); zero shares → skipped
  (`skipped.no_cash`). No leverage: cash never goes below zero (AmiBroker account margin 100).
- Costs are Phase 7's user-declared money costs per fill, paid from cash.
- No interest on idle cash (AmiBroker "Annual interest rate" 0, declared). Held positions are not re-sized.

Accounting identity on every bar: **equity = cash + Σ qty × mark**, cash ≥ 0, open positions ≤ `max_open`,
Σ qty × mark ≤ equity (exposure ≤ 100 %). These are test invariants (§12.2).

### 6.5 Trades, statistics and benchmark

Each trade is reported in money terms (member, qty, entry/exit fills, notional, costs, rank and score at the
signal, reason) and is also mapped to the reference calendar as a book-format `Trade` with Phase 7 effective
prices and weight = net entry notional ÷ portfolio equity at the signal close, so `Trade::ret(0)` is the trade's
contribution to portfolio return. The portfolio equity curve is the ledger itself (normalized to 1 at `from`).
Statistics come from the book `stats()` (read-only) on that equity curve and trade list, with the reference
instrument's dates for years and holding days; `exposure` there means "share of bars with at least one open
position", so the response adds `avg_exposure` (mean invested ÷ equity) and `max_positions`. The benchmark (`bh`)
is the reference index, normalized — an index used as a benchmark is never traded.

### 6.6 Optimization

`lab/optimize` with `spec.portfolio` (one gateway slot, research roles). Optimize paths: every Phase 7 path,
plus `portfolio.max_open`, `portfolio.worst_rank_held`, `portfolio.score.p.<key>` and Phase 7's
`execution.sizing.<field>`. Limits: ≤ 200 combinations and members × combinations ≤ 12,000; `split` is refused
for portfolios in this phase; ranking reuses the book optimizer's `conclude_rank_with_outcomes` on the portfolio
statistics (trade-rate floor, five rank statistics, base preferred on ties). Member bars are loaded once per
request and shared read-only by the worker threads; each thread keeps one indicator cache per member.

### 6.7 What a portfolio never does

It never substitutes an index for a member, never fills a missing member price, never trades a member before
its first bar or after its last, never sizes from future equity, never opens a short, and never claims dated index
membership.

### 6.8 Labels

Every body keeps `label: "User-defined indicator strategy · simulated · not a book method"`. A portfolio body adds
`kind: "lab_portfolio_simulation"` / `"lab_portfolio_optimization"`, `execution_model` (Phase 7) and
`membership` (§6.9). Foreign runs add `foreign` (§10).

### 6.9 Survivorship (declared, never hidden)

`membership: {kind, historical_membership_verified: false, note}` on every portfolio body:

- `current_snapshot` (India baskets, US "current"): "Members are today's constituents tested over past dates:
  stocks that left the index (often after falling) are missing, which biases results upward."
- `current_plus_past_snapshot` (US "_past"): "Members include past constituents but without the dates they were
  in the index, so a stock can be traded years before it joined."
- `user_list` (explicit tickers): "A list chosen today carries hindsight selection bias."

### 6.10 Compute bounds

| Bound | Value | Reason |
|---|---|---|
| universe members | ≤ 120 | memory: member bars are kept for the request (≈ 100 × 7,700 India bars or 100 × 17,000 US bars ≈ 50–140 MB); covers Nifty 50, Midcap 100, NASDAQ 100, Dow lists |
| explicit `tickers` | ≤ 50 | spec bytes: 50 qualified tickers ≈ 1.5 KB of the 8 KB spec |
| other instruments (`symbol` + `vs`) | ≤ 4 per spec | one load each |
| portfolio run wall clock | 90 s (gateway allows 120 s) | refused as `invalid` "portfolio run stopped after 90 s; use fewer members or a shorter window" — no partial result |
| portfolio optimize | ≤ 200 combos, members × combos ≤ 12,000, 300 s | single slot; no partial ranking |
| member loading | sequential; raw rows released after conversion while member Frames remain for the request | source-bar admission checks prevent accumulating an unbounded cohort; one loader operation is cooperative rather than preemptible |
| member history | ≤ 50,000 source bars per member and ≤ 500,000 across members, reference and loaded foreign sources | initial bounds for the 1 GiB engine; no source is truncated and no member is silently removed to meet a resource limit |
| compiled portfolio work | ≤ 8,000,000 compiled cells; at most 2 optimize workers, reduced by the working-set estimate | raw indicator caches are cleared between members; requests over the budget fail rather than publishing a narrowed universe |

## 7. Markets

- **India (`engine-in`) and US (`engine-us`)**: foreign operands and portfolios. A foreign reference to the other
  India/US market or to a gl id answers `needs_data` (each engine holds one market's data; nothing is fetched
  across engines).
- **gl/crypto (`engine-gl`)**: foreign operands between gl instruments are allowed (every body keeps Phase 5's
  `provisional: true`; alignment is usually `prior_date`). Portfolio mode on `engine-gl` answers `unsupported`
  ("portfolio mode runs on India/US stocks and ETFs; gl instruments are provisional indices, CFDs and crypto").
  Mixing gl into an India/US strategy is refused as above, so there is nothing to label.

## 8. Transport (GET, gateway unchanged)

Engine routes stay GET-only and the gateway's 16 KB query / 8 KB spec caps stay. A portfolio spec references its
universe **by basket id** (a few bytes) or carries at most 50 tickers inline, so it fits the existing caps.
Alternatives rejected: *spec by reference to a saved strategy id* would need the engine (which has no database
access) or the gateway to fetch from the auth service and would make unsaved runs impossible; *compressed specs*
need a decompressor (a new crate or hand-written inflate) and defeat the gateway's spec checks. The catalogue
already publishes `max_spec_bytes` / `max_query_bytes` (Phase 7) and now `max_universe`, `max_universe_list`,
`max_foreign`, so the UI can warn before sending.

## 9. Higher timeframe and position sizing (items 4 and 5 of the request)

Covered by §4.4 (Foreign + TimeFrame) and §6.4 (portfolio sizing). Single-instrument sizing is Phase 7's
`execution.sizing`; Phase 8 adds no single-instrument sizing field.

## 10. API (engine, `lab/api.rs`; no `main.rs` or gateway change)

- `GET lab/catalogue`: the four pair indicators appear as computed; adds `max_universe` (120),
  `max_universe_list` (50), `max_foreign` (4), `portfolio_modes`, `rebalance` values, `foreign_volatility`
  (this engine's VIX name or null), and `baskets: [{id, members, membership_kind}]` of this engine.
- `GET lab/series`: accepts `vs=<ticker>` for the pair indicators.
- `GET lab/run`, `lab/optimize` (single instrument): unchanged contract; when the spec reads other instruments the
  body adds, after `instrument`, `"foreign":[{"ticker","type","alignment","fixup","first","last"}]` (absent
  otherwise, so existing bodies stay byte-identical).
- `GET lab/run` with `spec.portfolio` (no `ticker`): `{label, kind:"lab_portfolio_simulation", origin:null,
  portfolio:{mode, max_open, worst_rank_held, rebalance, rank, universe:{basket|null, members, admitted,
  refused:[…]}}, membership, execution_model, reference:{ticker, type:"index"}, foreign, from, to, dates,
  equity, gross_equity, cash, exposure, positions, bh, trades:[{ticker, entry, exit, entry_i, exit_i, entry_px,
  exit_px, qty, notional, costs, ret, bars, open, signal, reason, rank, score, dir, entry_kind}], skipped:{no_slot,
  no_cash, no_bar}, events, stats, gross_stats, money:{capital, final_equity, costs_total, currency}, provenance:[…
  per admitted member]}`.
- `GET lab/optimize` with `spec.portfolio`: the Phase 2 optimize body shape with `kind:"lab_portfolio_optimization"`
  and the portfolio `cols`; `split` refused.
- Errors: unchanged contract (`invalid`, `unsupported`, `needs_data`, each with the lab label).

## 11. Saved strategies (auth service)

No schema change. Portfolio specs save like any lab strategy (the UI stores `instrument` as `portfolio:<basket>` or
`portfolio:list`). **Submit/publish refuses a spec with `portfolio`** ("Portfolio strategies use today's basket
lists, not dated index membership (survivorship bias); they stay research-only until historical membership is
admitted."), and `publishable` is false for them — one helper in `contracts.mjs` and its use in
`lab-strategies.mjs`. Foreign single-instrument strategies stay publishable on in/us.

## 12. Testing

### 12.1 No lookahead across symbols

- Alignment property: for random calendars, every aligned value comes from a foreign date ≤ *t* (`same_date`) or
  < *t* (`prior_date`); with `fixup` too.
- Series property: every computed indicator as a foreign operand on D/W/M/Q (and one `nD`), truncating **either**
  instrument's data after bar *k* never changes an earlier aligned value.
- Trade property (single instrument): perturbing every bar after *k* of the run instrument **and** of each foreign
  instrument never changes trades closed by *k*.
- Portfolio property: perturbing every member's and every foreign instrument's bars after *k* never changes
  equity, cash, positions or trades entered and closed by *k*, for signals and rotation specs with next-open and
  close fills.

### 12.2 Alignment with mismatched holidays; accounting invariants

- India stock vs NIFTY 50 with a stock-only suspension day and an index-only date; NYSE vs crypto (7-day) with a
  weekend; HSI vs IG:DAX with a Hong Kong holiday on the previous date; CBOE VIX `same_date` vs `prior_date` by fill
  mode; `fixup` filling only missing dates, not indicator NaNs.
- Every bar of every portfolio test: equity = cash + Σ qty × mark (1e-9 relative), cash ≥ 0, open ≤ `max_open`,
  invested ÷ equity ≤ 1 (`exposure` on the wire is invested currency, not a fraction), quantities whole, every fill price inside its bar's low–high.

### 12.3 Determinism

Shuffled universe order, repeated runs and 1 vs N optimizer threads give identical bodies (except `ms`, `threads`).

### 12.4 AmiBroker documented-example fixtures (hand-computed from the documented semantics)

| # | Source | Behaviour reproduced |
|---|---|---|
| P1 | Portfolio-level backtesting guide (h_portfolio.html): `PositionScore = 50 - RSI(14)` example, "the stock with lower RSI will be preferred", `PositionSize = -25` ("invest 25% of portfolio equity in single trade") | four members signal on one bar, `max_open` 2: the two lowest-RSI members are bought, each with 25 % of equity; the others count as `no_slot` |
| P2 | KB 2015-09-28 "Symbol selection when PositionScore is not defined" | equal scores → greater position size first, then alphabetical order |
| P3 | `EnableRotationalTrading` / `backtestRotational` reference ("sorted according to absolute value of PositionScore", "enters the trades starting from highest ranked security", held "until security's rank drops below WorstRankHeld", score 0 = no trade and exit) | `max_open` 2, `worst_rank_held` 3: a holding at rank 3 is kept, at rank 4 replaced by the best non-held member; an ineligible holding exits |
| P4 | `Foreign()` reference: "remove bars that exist in Foreign symbol but do not exist in currently selected symbol"; fixup 0 holes stay Null; fixup 1 fills from the previous bar | alignment fixture with holes on both sides |
| P5 | `SetForeign()` / `RestorePriceArrays()` reference (signals on the foreign symbol, trade prices on the current one) | index-signal, ETF-executed run: fills and stops on the ETF's prices |

### 12.5 Gate

Full engine suite; Phase 7 pinned-output hash and Customize template fidelity unchanged; book guard with engine
compare; Phase 5 gl checks (gl foreign run carries `provisional`, gl portfolio `unsupported`, gl id on in/us
`needs_data`); auth node tests; `check_upload.py`; release build without lab warnings; scope proof (only
`lab/**`, two auth files and their tests, docs); optional real-data smoke when the basket cache is present.

## 13. Out of scope, named follow-ups

1. Dated index membership (point-in-time constituents) admission — then publishable portfolios and per-date
   universes.
2. Universes above 120 (NSE 500, S&P 500, Russell): streaming member evaluation with compact per-member signal
   arrays.
3. Portfolio shorts and long/short books (`MaxOpenLong`/`MaxOpenShort`, `SeparateLongShortRank`, a short-margin
   model; India cash shorts stay refused by Phase 7).
4. AmiBroker `backtestRegular` pre-cleaning (`ExRem(Buy, Sell)` before the portfolio pass) as an option.
5. Resting limit/stop entries, scale-in/out and per-position rebalancing (AmiBroker KB "re-balancing open
   positions") inside a portfolio; cash interest; `HoldMinBars` for rank exits.
6. Cross-market references (an engine-to-engine data service) and extra series (`extra_data` TRIN, ADV) as
   foreign symbols.
7. New Customize templates that need a second instrument (the 9 VIX and 5 TRIN methods): each needs its own
   fidelity admission.

## 14. Assumptions for the user's review

Decided without the user (unavailable). Each: decision — reason — cost if wrong.

1. **Operand field `symbol`** (Phase 7's reserved name), plus `fixup` and `vs`. — One grammar for every operand,
   node and expression. — None; renaming is a parser change.
2. **Foreign series are computed on the other instrument's own bars, then aligned.** AmiBroker aligns first
   (`SetForeign` replaces the price arrays aligned to the current symbol, then indicators run). — The value equals
   the other instrument's own chart, and no aligned hole enters an indicator. — On dates where the two calendars
   differ, an indicator value can differ from AmiBroker's; same-exchange pairs (the common case) agree.
3. **Default `fixup: none`** (AmiBroker's default is fixup 1). — AGENTS.md forbids filled data unless declared;
   `previous` is one field away and labelled. — Ported AFL relying on fixup 1 needs `fixup:"previous"` per
   operand.
4. **`offset` on a foreign operand counts the other instrument's bars.** AmiBroker `Ref(Foreign(…), -1)` counts the
   current symbol's bars. — Consistent with Phase 7 ("offset counts bars of the operand's own timeframe"). —
   Differs only around holidays the two calendars do not share.
5. **`prior_date` for instruments that close later** (CBOE VIX with close fills; gl pairs with different calendar
   or timezone), `same_date` otherwise. — Never uses information that did not exist at the decision time. —
   Connors-style VIX rules with *close* fills see the previous day's VIX (with next-open fills they see the
   same day's); a gl pair that actually closes earlier is lagged one day (conservative).
6. **Index runs that read other instruments stay Phase 7 unit studies** (`executable:false`); money/portfolio
   on an index answer `needs_data`. — Keeps Phase 7's single rule for indices; an index is never the executed
   instrument of an executable result. — Users wanting an executable index strategy need the ETF's data (India
   has no ETF series yet).
7. **Pair indicators: TradingView-style definitions declared here, `vs` aligned without fixup, D only.** — No
   parity fixture exists yet (as for Phase 6). — W/M correlation needs a later change.
8. **Portfolio is money mode only** (`execution.capital` required). — Shared cash and slots are money concepts;
   unit weights cannot express them. — One extra field for the user.
9. **Signals mode re-tries a skipped entry on the next true bar** (`backtestRegularRaw`); AmiBroker's default
   `backtestRegular` would ignore it until a sell. — Phase 8 exits can depend on the position (`bars_held`,
   stops), which AmiBroker's signal-level pre-cleaning cannot evaluate; Phase 7 also does not queue skipped
   entries. — Ported AFL in regular mode can show more entries after skipped signals (follow-up 4).
10. **Ties: score, then requested size, then ticker** (AmiBroker KB order). — Documented and deterministic. — None.
11. **Long-only portfolios.** — No short-margin model is declared; India cash shorts are refused by Phase 7. —
    Rotational long/short (negative `PositionScore`) is not available (follow-up 3).
12. **Universe ≤ 120 and lists ≤ 50.** — Bounded memory and time in one request; fits the gateway caps. — NSE 500,
    F&O, S&P 500 and Russell universes need a hand-picked list of ≤ 50 until follow-up 2.
13. **Reference calendar = NIFTY 50 / SPX dates** (AmiBroker "pad and align to reference symbol"). — One
    deterministic clock for shared equity. — A member date missing from the index file is unused (rare).
14. **Positions on members without a bar keep their last close; data end exits at the last close.** — No invented
    prices; delisting proceeds are unknown. — Delisted US members' true outcomes (buyouts, bankruptcies) are not
    reproduced; `events` counts these exits.
15. **Default size = 100 % / `max_open` of equity at the signal close; shrink to cash; whole shares; no interest;
    no re-sizing of held positions.** — Phase 7 money rules applied to shared cash. — AmiBroker users with other
    defaults (e.g. intraday-equity sizing) see slightly different quantities.
16. **Rotation default rebalances every bar; W/M/Q/Y use the last bar of the period** (same calendar knowledge as
    W/M operands). — Mirrors AmiBroker's default and its `scoreNoRotate` idiom. — An AFL system that rebalances on
    the first bar of a month is one bar later/earlier.
17. **Rotation eligibility = `entry.all`; a held member that becomes ineligible exits.** — The AFL idiom
    `IIf(filter, score, 0)`. — A literal score of exactly 0 is not special (only eligibility is).
18. **Survivorship declared on every body; portfolio specs not publishable.** — No dated membership exists;
    AGENTS.md forbids publishing unresolved results. — YT Strategies cannot show portfolios until follow-up 1.
19. **Universe by basket id or inline list; GET and the gateway unchanged.** — See §8. — A future universe beyond
    50 hand-picked names needs a saved-universe feature.
20. **Same-market only; gl never mixed into India/US; gl portfolios unsupported.** — Each engine holds one
    market; gl is provisional. — Cross-market filters (e.g. SPX filter on Indian stocks) wait for follow-up 6.
21. **No `main.rs`, gateway or book change.** — The `/api/lab/` prefix and the `run`/`optimize` paths already
    exist. — None.
22. **Documented-example fixtures are hand-computed** from the AmiBroker pages cited in §15, not exported from
    AmiBroker (no install here). — No decompilation, no licence question. — If AmiBroker's undocumented behaviour
    differs on a fixture, the fixture documents the lab's choice, not AmiBroker's output.

23. **Initial deployment memory bounds also limit admitted history.** The member-count and combination caps alone do not bound a full-history rule tree's memory. The 1 GiB engine therefore additionally admits at most 50,000 bars per member, 500,000 source bars including the cohort, reference and loaded foreign sources, and 8,000,000 compiled cells, with at most two optimization workers. A refusal asks for fewer members or shorter admitted histories; changing only the visible From/To window does not remove indicator warm-up history. These are conservative admission limits, not a completed peak-memory benchmark.
24. **Final-bar entries and coverage-end knowledge.** New portfolio entries do not fill on the final reference bar or a member's known terminal quote; realized exits still process. Daily rotation can take final-close rank exits, while an unfinished W/M/Q/Y period does not rebalance. Terminal `data_end` liquidation is a disclosed historical adaptation, not live-causal knowledge of a delisting. `entry_on: edge` applies to new rotation entries on that rebalance bar without latching; held eligibility and ranks continue to use the current level.

## 15. Sources (AmiBroker documentation only; nothing decompiled)

- Foreign: https://www.amibroker.com/guide/afl/foreign.html — signature `Foreign(TICKER, DATAFIELD, fixup = 1)`;
  fixup 0 "the holes are not fixed", 1 "missing data bar OHLC fields are all filled using previous bar Close and
  volume is set to zero", 2 pre-4.90 behaviour; "synchronizes the data file you are referencing with the currently
  selected symbol"; removes bars that exist only in the foreign symbol.
- SetForeign / RestorePriceArrays: https://www.amibroker.com/guide/afl/setforeign.html,
  https://www.amibroker.com/guide/afl/restorepricearrays.html — `SetForeign(ticker, fixup = True, tradeprices = False)`.
- Portfolio-level backtesting: https://www.amibroker.com/guide/h_portfolio.html — "AmiBroker will use the absolute
  value of PositionScore variable to decide which trades are preferred"; `PositionSize = -25` example; "Portfolio
  equity is equal to available cash plus sum of all simultaneously open positions"; `MaxOpenPositions`;
  same-bar exit/entry options.
- Ranking ties: https://www.amibroker.com/kb/2015/09/28/symbol-selection-when-positionscore-is-not-defined/
- Rotational mode: https://www.amibroker.com/guide/afl/enablerotationaltrading.html (obsolete name; "Use
  SetBacktestMode( backtestRotational )"; scoreNoRotate; score 0; WorstRankHeld), and
  https://www.amibroker.com/guide/afl/setbacktestmode.html (backtestRegular, backtestRegularRaw, …).
- SetPositionSize: https://www.amibroker.com/guide/afl/setpositionsize.html (spsValue, spsPercentOfEquity, spsShares,
  spsPercentOfPosition, spsNoChange).
- SetOption: https://www.amibroker.com/guide/afl/setoption.html (MaxOpenPositions, WorstRankHeld, AccountMargin,
  MinShares, SeparateLongShortRank, MaxOpenLong/Short, UsePrevBarEquityForPosSizing …).
- Settings window: https://www.amibroker.com/guide/w_settings.html (Initial equity, Allow position size shrinking,
  Round lot size, Pad and align to reference symbol, Account margin, Annual interest rate, Max. open positions).
- Multiple time frames: https://www.amibroker.com/guide/h_timeframe.html (expandLast / expandFirst / expandPoint;
  "expandFirst used on prices different from open may look into the future").
