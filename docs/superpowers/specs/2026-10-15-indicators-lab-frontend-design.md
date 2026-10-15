# Indicators lab — frontend (React) — design

Status: draft for the user's review · Branch `Nandini` (PRs into `Manan`, never `main`) · Parents:
`2026-10-14-indicators-lab-design.md` §6–§8 and `2026-10-16-customize-book-strategies-design.md` §9 and §11.

## 1. Goal

Give researchers a working **Indicators** page and a **YT Strategies** page in the React app
(`Context/guru/frontend/react`), on top of the lab backend that already exists (Phases 1, 2, 4, 5, 6 and
Customize). One page, top to bottom:

1. pick a market (India, US, Global & crypto — provisional) and an instrument;
2. browse all 299 catalogue indicators, read any of them, and chart the computed ones (price overlays and
   panes, editable settings, D/W/M);
3. build entry/exit rules, price orders and execution settings, run a backtest, and read the result;
4. optimize chosen settings with an optional in-sample / out-of-sample split;
5. save, edit, duplicate, delete, submit and withdraw strategies; admins publish or reject;
6. start a variant from a book method template ("Customize"), see the template's adaptation notes and a
   display-only fidelity comparison.

YT Strategies lists the published snapshots read-only and opens any of them in the lab.

Not in scope: backend changes (Phase 7 is being built in parallel and only extends the catalogue and the
spec grammar), a book-page "Customize" button (`ResearchPages.tsx` is protected), Excel export, a full
optimization parameter heat map, basket scans, portfolio mode, intraday.

## 2. Constraints

- **Protected, never edited:** `src/compat/*`, `src/pages/ResearchPages.tsx`. **Additive only:**
  `src/app/custom-pages.ts`. Other shared files touched by this phase: `src/components/AppHeader.tsx`
  (remove the hard-coded "YT Strategies soon" item, give custom links the `on` state) and
  `vite.config.ts` (coverage `include` list only). No backend, gateway, auth or Rust file changes.
- **No new npm dependency.** Charts use the vendored Lightweight Charts 4.2.0 (`window.LightweightCharts`)
  and the vendored `window.VaultReport` (`monthly`, `csv`), both already loaded by `index.html`. Typed
  through a lab-local `globals.d.ts`; `src/env.d.ts` stays unchanged. W/M (and any later Q/Y) candles
  are aggregated by a small lab function (Monday-start weeks, calendar months, the same keys as
  `VaultReport.aggregate`) so new timeframes need no vendored change.
- **Theme:** existing tokens and classes only (`.box`, `.box-h`, `.metrics/.metric`, `.rnote`, `.dbtn`,
  `.join/.join-item`, `.dfield`, `.tbl`, `.tlog`, `.empty`, `.loading`, `.lwc`, `.chip`). New CSS uses
  `var(--card)`, `var(--panel)`, `var(--thead)` — never `#fff` — so Midnight works. Archivo + IBM Plex Mono.
  Lightweight Charts colours are read from CSS variables when a chart is created; a theme change remounts
  charts.
- **AGENTS.md:** verified source rules, declared adaptations, simulated results and book-reported figures
  stay separate. Lab output never carries a claim, gate, readiness or "verified" wording. A catalogue entry
  is not a verification. Stock methods need actual stocks; an index never substitutes.
- **Session:** all calls go through `/api/...` with the HttpOnly cookie (`credentials: 'same-origin'`).
  Never store tokens. `localStorage` only for per-browser conveniences (none needed here).
- **Routing:** the shell matches the hash path exactly, so every sub-view lives in the query:
  `#/indicators?region=in&tk=NIFTY%2050&tf=D&customize=rsi2_4&strategy=12&published=7`. `region` is
  `in | us | gl`. The page writes its own query with `history.replaceState`, so `lastView` restores it.
- **Phase 7 compatibility:** operators, timeframes, price sources and limits come from `/lab/catalogue`;
  special operands (`bars_held`) and price orders (limit, stop-loss, target, trailing) come from two small
  registries in `spec-model.ts`; the editor edits the spec object in place and keeps every key it does not
  understand. A new timeframe (Q/Y), operator or operand type therefore appears or survives without a
  rewrite; an operand shape the editor cannot edit is shown read-only ("kept as is") and can be removed.

## 3. Screens

### 3.1 Indicators (`#/indicators`)

Desktop: two columns (library rail 300px | work column). Below 900px the rail collapses into a
"Library" disclosure above the work column; at 400px every box is single-column with no horizontal scroll
(tables scroll inside `.tlog`/`.tblwrap`).

1. **Header** — `h1.st` "Indicators", lead copy "Pick indicators from the library, chart them on any
   market, turn them into entry and exit rules, then backtest and optimize. Saved strategies can be
   submitted to YT Strategies.", and the server `label` of the catalogue verbatim
   ("User-defined indicator strategy · simulated · not a book method").
2. **Toolbar** — Market `.join` (India | US | Global & crypto · provisional); Instrument select;
   Chart D | W | M (plus any timeframe the catalogue adds); Show 1Y | 3Y | All. Instrument options:
   - in/us: `GET /api/{m}/tickers` (indices and catalogued basket stocks; a stock shows its basket).
   - gl: `GET /api/gl/lab/instruments`; `ready` rows show `instrument.name`; `needs_data` rows are
     disabled with their `reason`.
   - With a Customize template loaded, rows whose known type is outside `allowed_types` are disabled
     with "Not admitted for this method (needs <types>)". Native rows carry `asset_type`; index rows have
     no client-side type, so they stay selectable and the engine's `needs_data` answer is shown as given.
   - On gl a provisional strip is always visible: `provisional_note` + `instrument.label`.
3. **Library rail** — search box ("Search name, id, TradingView or AmiBroker name") matching `name`,
   `id`, `tv`, `ab`; 22 category groups with counts; each item shows name and a tag (`price`, `pane`,
   `needs data`). Listed (`computed:false`) items, volume indicators on an instrument without volume
   (gl, or `has_volume:false`) and breadth items on gl or on W/M are disabled and show the server `note`
   or a fixed reason. Clicking a computed item adds a chip; clicking the name text opens the reference
   panel.
4. **Reference panel** (collapsible) — Definition, facts (placement, needs, warm-up `first`, sparse,
   timeframes, engine computed/listed), Settings table (key, default, min, max, step), Outputs, "Using it
   in rules". No TradingView parity badge: zero fixtures exist, so the panel says "Not yet checked
   against TradingView." for every indicator.
5. **Chart workbench** (`.box`) — Lightweight Charts candles (from `series` calls for open/high/low/
   close, cached per market+ticker) with up to 4 price overlays and 3 panes (oldest dropped). Chips show
   colour, name and editable settings (number inputs with min/max/step; the `variant` parameter is a
   select TradingView | AmiBroker); × removes. Sparse outputs draw gaps (null → whitespace point),
   boolean outputs (0/1) draw as markers on the price chart, not lines. W/M: candles aggregated from
   the daily OHLC, indicator values sampled at each period's last daily bar; the last bar is
   marked "in progress" and the chart note says "The last weekly bar is still in progress; its values
   can change until the week closes." Trade spans from the latest backtest show as entry/exit markers.
   Note text: "Indicators are calculated from the full history, then the visible range is shown."
6. **Strategy rules** (`.box`) — subtitle "values from completed bars only". Direction Long | Short.
   "Entry · all of these" and "Exit · any of these" lists of conditions; each condition =
   operand · operator · operand. Operand editor: kind (price source | indicator | number | bars held on
   the exit side), indicator settings, output (multi-output indicators), timeframe, offset ("n bars
   ago", 0–1000). "+ Add condition" per side, disabled at `max_conditions` total. Price orders:
   limit entry, stop-loss, target, trailing, each off | % | ATR (multiplier + ATR length). Execution:
   fill (next open | close, hidden when a limit entry is set), exit fill (next open | close). Run
   settings: From / To (optional dates), fees and slippage (bps per side). Client pre-checks (§6.4)
   show inline; "Run backtest" is the primary button.
7. **Backtest result** (`.box`) — server `label` verbatim (plain or variant), `start` disclosure,
   KPI strip, equity vs buy & hold (log scale), drawdown, monthly returns, tradesheet, CSV download.
8. **Optimization** (`.box`) — role-gated, §3.4.
9. **Customize a book method** (`.box`, in/us only) — §3.5.
10. **My indicator strategies** (`.box`) and, for admins, **Review queue** — §3.6.

### 3.2 Result details

- KPIs (`.metrics`): Net return `total_return`, CAGR `cagr`, Max drawdown `max_dd`, Sharpe `sharpe`,
  Trades `trades`, Win rate `win_rate`, Avg trade `avg_trade`, Buy & hold CAGR `bh_cagr`. `null` → "—".
  Under the strip: "Exposure N% · before costs: gross return X" from `exposure`, `gross_total_return`.
- `start` not null → `.rnote`: "Results start on <date>, not the requested start: earlier opens on this
  instrument are not reliable enough for next-open fills." (own copy; the server reason always says
  "stale").
- Costs line from `costs`: "Fees F bps + slippage S bps per side · taxes not modelled" (`taxes:null`).
- Tradesheet (`.tlog > table.tbl.tl`, newest first, first 200 rows; "N older trades are in the CSV"):
  Entry date, Side (Long/Short from `dir`), Fill (`entry_kind`: Next open / Close / Limit), Entry,
  Exit date, Exit, Return, Bars, Exit reason. Open trades show "Open" in exit date and "Still open at
  the end" as reason. `reason` maps to friendly text (§6.5); unknown text is shown as given.
- Monthly table from `VaultReport.monthly({dates, equity: equity/equity[0]})`.
- CSV via `VaultReport.csv` (header + rows of the tradesheet columns), file
  `lab-<ticker>-<from>-<to>.csv`.
- A failed run clears the previous result: numbers never stay on screen next to an error.

### 3.3 Variants and labels

The result `label` is shown exactly as returned (it is the variant label when `origin` is set).
Error bodies carry the plain label; the page never shows an error's label as a variant label.
Saved lists and YT Strategies show "Customized from <book> · <method>" as a badge when the row's
`origin` is set (book/method ids from the spec origin; the method name when the template list is
loaded), and saved rows group under "My variants of <method>".

### 3.4 Optimization

- Viewers: the box shows "Researcher access is required for optimization." and no controls.
- Dimension table: one row per optimizable path of the current spec (§6.3) — checkbox, label (e.g.
  "Entry 1 · EMA length"), From, To, Step, Values; footer "<n> combinations" and the limits
  (≤ 1,000 values per setting, ≤ `max_combos` combinations). "Tick at least one setting" when empty.
- Split select: None | last 20% | 30% | 40% held back; the percentage converts to a `split` date from
  the dates of the last run (or the chart's dates). Line: "Out-of-sample: the last N% (from <date>) is
  held back · settings are ranked on the earlier part only".
- Request always uses `slim=1` (the page reads `picked`, never `rows`) — avoids the 64 MB cap.
- While running: elapsed seconds and "The engine stops an optimization after 300 s."
- Result: server label verbatim; "Ranked on: in-sample only" / "full period" from `ranked_on`;
  "<combos> settings tested · <eligible> eligible (at least <min_trades> trades) · <distinct> distinct";
  table rows "Now" (`base`) and #1–#3 (`rank.top3`) with the dims values, the five rank stats
  (`rank.stats`) with percentiles, and with a split `is_*` (labelled "ranking period") and `oos_*`
  ("held back · display only") total return, CAGR, Sharpe, trades. "Apply" writes the dims values into
  the spec (`setAt`) and reruns the backtest.
- With a split, a fixed note: "In-sample and held-back trade counts can differ from the full run:
  trades that cross the split date are cut at the boundary. A strong ranking score with a weak
  held-back result usually means curve fitting."
- `eligible: 0` → "No setting had enough trades to rank (at least <min_trades>). Nothing is ranked."
- 429: message as given plus "Try again in N s" (from `Retry-After`, default 5) — the button re-enables
  after the countdown; no automatic retry. The one slot is shared by every market and the book
  optimizers, so the copy does not blame the user.
- "Variants tried this session: N" = sum of `combos` of finished optimizations + backtest runs.

### 3.5 Customize

- Picker (in/us only; on gl: "Customize runs on India and US book methods only."): search over the
  `/lab/templates` list (43) by method name, id and book; header shows the server label and
  "<templates> templates · <admitted> admitted methods · <methods> catalogued".
- Choosing one sets `?customize=<id>` → `GET /lab/template?method=<id>`:
  - `available`: the builder is replaced by `template.spec` (which carries `origin`), the panel shows the
    template label verbatim, the instrument requirement and allowed types, and the adaptation notes; the
    instrument picker is filtered (§3.1).
  - `unavailable`: "Customize not available — <reason>" (the UI adds the prefix), builder unchanged.
  - Methods without a template are not listed by the backend; a link with their id (the future
    book-page button) still lands on the reason.
- Fidelity panel (display only, appears with a loaded template): button "Compare with the book engine"
  → `GET /lab/fidelity?method&ticker` (no `p.*`). Shows label "Template fidelity · display only · not
  a verification" and the server `note` verbatim; headline "Template reproduces the book engine's trades:
  N/N" only when `reproduces`; 0/0 → "No trades on this window, so there is nothing to compare."; else
  "Matched M of B book trades and T template trades" + the differences table (≤50 rows; "<total> in
  all"). Refusal text shown as given.
- Changing the template's settings keeps `origin`; there is no control that edits or removes it.

### 3.6 Saved strategies and review

- List (`GET /api/app/lab/strategies`, own rows): name, variant badge, `<instrument> · <market> ·
  n entry / m exit · updated <date>`, status chip (Draft / Submitted / Published / Rejected),
  `reviewNote` only when rejected or published. Actions per status: Open (all), Duplicate (all),
  Delete (draft, rejected), Submit (draft, rejected, `publishable`), Withdraw (submitted).
- Save: name field + "Save as new" (POST) and, when the editor holds an own draft/rejected row,
  "Save changes" (PUT with `version`). Market saved as `in | us`, or for gl `crypto` when the
  instrument's market is `crypto`, else `gl`.
- Submit is enabled only when the editor shows that row unchanged and it ran successfully on the same
  market and instrument since it was opened; otherwise its title says "Run the current rules first; the
  engine checks the rules only when they run." Not publishable (gl/crypto) → disabled with "Provisional
  data cannot be published. Rerun on licensed data first."
- 409: the server message is shown as given with "Reload this strategy" (refetches the row and keeps
  the rules on screen; "Your rules on screen are kept. Save again to replace version N."). 403/404/400:
  message as given.
- Delete asks `confirm("Delete \"<name>\"? This cannot be undone.")`, sends `?version=N`, expects 204.
- Admin review queue (`GET .../review`): each submitted row with owner, market, instrument, "Open in lab",
  "Publish" (optional note), "Reject" (note required, 1–1000 chars; the button stays disabled until
  then). Errors as given.
- Viewers see their (empty) list read-only and no Save controls.

### 3.7 YT Strategies (`#/yt-strategies`)

`GET /api/app/lab/strategies/published`: cards with name, variant badge, owner, market, instrument,
`n entry / m exit`, published date, a short rule summary, and "Open in lab" →
`#/indicators?region=<in|us>&tk=<instrument>&published=<snapshotId>`. The Indicators page loads that
snapshot's spec into the builder as an unsaved draft (Save creates the user's own copy, origin kept).
Copy: "Published lab strategies. Each one is a simulated, user-defined rule set reviewed for
presentation — not a book method and not a verification." Empty: "Nothing is published yet."

## 4. Components and files

All new code lives under `src/features/lab/` plus two page files; each file has one job.

| File | Job |
|---|---|
| `types.ts` | API and spec types (catalogue, series, run, optimize, templates, fidelity, saved rows, errors) |
| `labels.ts` | Verbatim server labels and fixed copy (one place to grep) |
| `url-state.ts` | read/write the page query (`region`, `tk`, `tf`, `range`, `customize`, `strategy`, `published`) |
| `globals.d.ts` | minimal typings for `window.LightweightCharts` and `window.VaultReport` |
| `client.ts` | lab API client: engine base per market, `LabError`, Retry-After, 204, catalogue cache |
| `format.ts` | percent/number/date formatting, `—` for null |
| `ui.tsx` + `lab.css` | `LabErrorBoundary`, `ErrorNotice`, `Loading`, `Segmented`, `ServerLabel`, `ProvisionalStrip`, `MetricTiles` |
| `spec-model.ts` | default spec, immutable edits, `setAt`, optimizable paths, combos, client pre-checks, size checks, run query, split date, friendly reasons, registries |
| `charts.ts` | `createLabChart` (theme colours, ResizeObserver, dispose) |
| `bars.ts` | candles from OHLC series, W/M aggregation with in-progress flag, indicator sampling, gaps, boolean markers |
| `catalogue.ts` | search, grouping, availability of an indicator on a market/instrument/timeframe |
| `instruments.ts` | instrument options per market, allowed-types filter |
| `Toolbar.tsx`, `LibraryRail.tsx`, `IndicatorDetail.tsx` (+ `library.css`) | selection UI |
| `Workbench.tsx` (+ `workbench.css`) | chart + chips |
| `RuleBuilder.tsx`, `OperandEditor.tsx`, `OrdersPanel.tsx` (+ `rules.css`) | rule editing |
| `RunResults.tsx` (+ `results.css`) | backtest result |
| `OptimizePanel.tsx` (+ `optimize.css`) | optimization |
| `SavedStrategies.tsx`, `ReviewQueue.tsx` (+ `saved.css`) | saved rows and admin review |
| `CustomizePanel.tsx` (+ `customize.css`) | template picker, template summary, fidelity |
| `lab-state.ts` | page reducer: market, ticker, spec, dirty/last-run tracking, open row, variants counter |
| `src/pages/IndicatorsPage.tsx` | wires everything; registered at `/indicators` |
| `src/pages/YtStrategiesPage.tsx` | published list; registered at `/yt-strategies` |
| `scripts/lab-dev-proxy.mjs`, `scripts/lab-browser-check.mjs` | dev-only: local engine proxy without auth; headless Chrome check |

Components are presentational: they receive data and callbacks as props; only the two pages and
`client.ts` talk to the network. That keeps every component testable with plain props and lets several
people build them in parallel.

## 5. Data flow

- Page mount: `getAccount()` (role) and `client.catalogue(market)` (cached per market) in parallel;
  instruments per market; `region`/`tk` from the query (defaults: `in`, `NIFTY 50`; on gl the first
  `ready` instrument; on us the first ticker).
- Market change: resets instrument, chips, the open saved row, Customize state and optimization; keeps
  the rules (operands that the new market cannot serve are reported by the pre-checks or by the engine's
  `needs_data` answer, as given).
- Chart: per chip `client.series(market, {id, ticker, tf, p})`; OHLC via four `series` calls for the
  price sources, cached per market+ticker; at most 4 requests in flight (the gateway has 4 read slots).
- Run: `client.run(market, spec, {ticker, from, to, cost, slippage})` → result or `LabError`.
  `lab-state` records `lastRun = {specJson, market, ticker}` on success; any spec edit marks dirty.
- Optimize: `client.optimize(market, specWithOptimize, {..., split, slim:'1'})`.
- Saved rows: `client.strategies.*`; after every mutation the returned row replaces the local one (its
  `version` is the one to send next).
- Engine base: `in → /api/in`, `us → /api/us`, `gl | crypto → /api/gl`.

## 6. Error and honesty rules

### 6.1 Error shapes
`LabError extends ApiError` with `kind: 'engine' | 'gateway' | 'auth'`, `status` (HTTP),
`engineStatus?: 'invalid' | 'unsupported' | 'needs_data' | 'not_found'`, `label?`, `provisionalNote?`,
`retryAfter?` (seconds). Engine errors are 400/404 bodies with `status`; everything else is `{error}`.
A non-JSON body keeps "Request failed (N)." `ErrorNotice` renders the message only (plus the
provisional strip on gl, plus the countdown on 429). Headings by `engineStatus`: needs_data
"Data needed", unsupported "Not supported yet", invalid "Rule problem", gateway "Request refused".

### 6.2 Honesty
1. No numbers next to an error: a failed run/optimize/fidelity clears that box's previous result.
2. Every lab result, optimization and chart shows the server `label` verbatim.
3. On gl/crypto every chart, result, optimization and error shows `provisional_note` (from the body, or
   the constant in `labels.ts` when a gateway error has none) and `instrument.label`; Submit and Publish
   never appear enabled; `publishable:false` rows cannot be submitted.
4. The UI never says "verified", "TradingView-matched" or "reproduces" except the fidelity headline when
   the server says `reproduces:true`. A test renders the components and rejects `/\bverified\b/i` and
   `/TradingView-matched/i`.
5. Listed indicators are never charted and never offered as operands; their `note` is shown.
6. `needs_data` / `unsupported` / `invalid` messages are shown exactly as given.
7. W/M last bar is labelled in progress; operands on W/M carry the note "Weekly and monthly values are
   used only after that bar closes."
8. Book figures, claims and gates never appear on these pages.

### 6.3 Optimizable paths
From the spec: `entry.all[i].(lhs|rhs).value`, `entry.all[i].(lhs|rhs).p.<key>` (from the catalogue
param), the same under `exit.any[i]`, and `<order>.(pct|atr)` for each price order in use. Bounds: a
param's min/max/step; values: pct bounds from §6.4; numbers: ±50% of the current value, step 1.

### 6.4 Client pre-checks (UX only; the engine stays the authority)
Total conditions ≤ `max_conditions`; entry has ≥ 1 condition; exit has ≥ 1 condition unless a
stop-loss, target or trailing exists ("exit.any needs at least one condition (or a stop_loss, target or
trailing exit)"); no condition compares two numbers; no listed indicator; with a limit entry
`execution.fill` is removed; `JSON.stringify(spec)` ≤ 7,500 bytes and the encoded query ≤ 15,000
characters ("These rules are too long to send. Remove a condition or shorten settings."); pct bounds
(limit 0–50, stop 0–99, target 0–1000, trailing 0–99, ATR multiplier 0–50, ATR length 1–500).
`execution.cost_bps` present → the fees input edits it and the `cost` query is not sent.

### 6.5 Friendly exit reasons
`open` → "Still open at the end"; `exit.stop_loss` → "Stop-loss"; `exit.stop_loss (gap at the open)` →
"Stop-loss (gap at the open)"; `exit.target` → "Target" (and gap form); `exit.trailing` → "Trailing
stop"; `exit.any[i]: <text>` → "Exit rule i+1: <text>"; anything else as given.

## 7. Testing

- **Vitest per unit** (jsdom, act/createRoot, `fetch` stubbed, `window.LightweightCharts` and
  `window.VaultReport` mocked): client (paths per market, query encoding, `LabError` fields for engine
  and gateway bodies, Retry-After, 204 → null, non-JSON errors, catalogue cache); spec-model (default
  spec, edit helpers keep `origin` and unknown keys, JSON round-trip, `setAt`, optimizable paths and
  combos, pre-checks, size checks, run query with/without `cost_bps`, split date, friendly reasons);
  bars (W/M aggregation, in-progress flag, gaps, boolean markers); catalogue (search by tv/ab, listed
  disabled with note, volume on gl, breadth); each component (renders labels verbatim, disabled states,
  callbacks); saved strategies (version sent, 409 reload flow, delete 204, submit gating, publishable);
  review (reject note required); customize (prefix added, fidelity 0/0, no "verified"); YT page; the
  Indicators page end-to-end with stubbed fetch (role gating, error clears result, provisional strip).
- New `src/features/lab/*.ts` service files join the coverage `include` list (80% thresholds).
- `npm run typecheck`, `npm test`, `npm run test:coverage`, `npm run build`.
- **Real browser:** the existing release engine on the NAS India data (read-only) behind
  `scripts/lab-dev-proxy.mjs` (maps `/api/{in|us|gl}/*` → engine `/api/*`, stubs `/api/app/me`, answers
  `/api/app/lab/*` with 503), Vite pointed at it with `VAULT_DEV_PROXY`, and
  `scripts/lab-browser-check.mjs` (headless `/usr/bin/google-chrome` over the DevTools protocol):
  `#/indicators` at 1440px and 400px in Steel, Aqua and Midnight — no page errors, no horizontal
  scroll, catalogue rendered, a chip charted, a backtest run, an optimize run (small grid);
  screenshots saved to the scratchpad. US and gl have no local data on the NAS, so they are covered by
  the stubbed-fetch tests; saved strategies and review are DB-backed and stay the user's manual gate.
- **Guards:** `python3 scripts/check_book_untouched.py --base origin/Manan`,
  `python3 scripts/check_upload.py`; scope proof: every `lab-ui` commit touches only
  `Context/guru/frontend/react/**` or `docs/**`, and no protected path.

## 8. Delivery and waves

Implemented by several agents in parallel without worktrees, each owning disjoint files (the plan
lists them). Commits use the `(lab-ui)` scope and `git commit -- <paths>` so concurrent backend Phase 7
commits never mix in.

| Wave | Tasks (parallel inside a wave) |
|---|---|
| 1 | Foundation: types, labels, url-state, globals, page stubs, registration, header, coverage list |
| 2 | API client · UI primitives + format · spec model · chart kit (charts, bars) · dev tooling |
| 3 | Library + toolbar · workbench · rule builder · results · optimize · saved + review · customize · YT page |
| 4 | Indicators page assembly (state reducer + wiring) |
| 5 | Gate: typecheck, tests, coverage, build, browser check, guards, scope proof |

One PR from `Nandini` into `Manan`; the user pushes.

## 9. Assumptions for the user's review

1. **Two nav entries:** "Indicators" and "YT Strategies" replace the "YT Strategies soon" placeholder.
2. **Customize entry is the picker on the Indicators page.** Methods without a template are not listed
   (the backend lists only the 43 templates); a direct `?customize=<id>` still shows the server reason.
3. **No TradingView parity badges** anywhere until fixtures exist.
4. **CSV instead of Excel** for the lab tradesheet; Excel and the parameter heat map are follow-ups.
5. **Optimizations always use `slim=1`** and show the top 3 (the backend ranking), not a top 5.
6. **The split is chosen as a percentage** and sent as the matching date.
7. **Submit needs a fresh successful run** of the exact saved rules on the same instrument in this
   session ("run before submit").
8. **Index types are not known client-side** for in/us; the engine's answer is shown for a variant on an
   index it does not admit.
9. **Browser check uses India data only** behind a no-auth dev proxy; US/gl and DB-backed flows are
   covered by tests with stubbed fetch and by the user's manual gate on an isolated stack.
10. **Variants-tried counter is per page session** (not stored).
