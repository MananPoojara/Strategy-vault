# Indicators Lab Frontend Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship the Indicators page (library, chart workbench, rule builder, backtest, optimization, saved strategies and review, Customize) and the YT Strategies page in the React app, on the existing lab backend, without touching backend, book or protected frontend code.

**Architecture:** New code lives in `src/features/lab/` (types, client, pure models, presentational components with props and callbacks) plus two pages registered through `src/app/custom-pages.ts`. Only the pages and `client.ts` talk to the network; components receive data and loader callbacks, so each one is tested alone with plain props and the tasks can be built in parallel by different agents. Charts use the vendored `window.LightweightCharts` 4.2.0 and `window.VaultReport` (`monthly`, `csv`); no npm dependency is added.

**Tech Stack:** React 19.3, TypeScript 7 (strict), Vite 8, Vitest 5 + jsdom (act/createRoot pattern, `fetch` stubbed), Lightweight Charts 4.2.0 (vendored global), Node 22 for dev scripts, headless `/usr/bin/google-chrome` over the DevTools protocol for the browser check.

**Spec:** `docs/superpowers/specs/2026-10-15-indicators-lab-frontend-design.md` (read it all before any task). Parents: `docs/superpowers/specs/2026-10-14-indicators-lab-design.md` §6–§8, `docs/superpowers/specs/2026-10-16-customize-book-strategies-design.md` §9, §11.

All paths below are relative to `Context/guru/frontend/react/` unless they start with `Context/`, `docs/` or `scripts/check_`. Repository root: `/home/aff34/.cache/strategy-vault-development`.

## Global Constraints

- Branch `Nandini`; never `main`; never push (the user pushes); no force-push; no git worktrees; commits carry **no** Co-Authored-By trailer.
- Backend Phase 7 agents commit to the same branch at the same time. Commit only your task's files with `git add -- <paths> && git commit -m "<type>(lab-ui): …" -- <paths>`; if `.git/index.lock` exists, wait a second and retry. Every commit subject uses the `(lab-ui)` scope (the gate's scope proof depends on it).
- Protected, never edited: `src/compat/*`, `src/pages/ResearchPages.tsx`. Additive only: `src/app/custom-pages.ts`. Never edit any file under `Context/guru/backend`, `Context/guru/webapp`, `src/env.d.ts`, `src/services/*`, `src/state/*`, `src/app/App.tsx`, `src/app/routes.ts`. Shared files touched only by Task 1: `src/app/custom-pages.ts`, `src/components/AppHeader.tsx`, `vite.config.ts`.
- **Only touch the files your task lists.** If you need something from another task's file that does not exist yet, code against the interface written in this plan; never create or edit that file.
- No new npm dependency; `package.json` and `package-lock.json` unchanged.
- Server labels verbatim (they live in `src/features/lab/labels.ts`, Task 1): `User-defined indicator strategy · simulated · not a book method`; `Customized from <book> · <method name> · user variant · simulated`; `Book template · declared adaptation into lab rules · not a verification`; `Template fidelity · display only · not a verification`; `Provisional · TradingView export · internal research only, not publishable to YT Strategies`; prefix `Customize not available — `. Show the `label` a response carries; never show an error body's label as a variant label.
- Never show numbers next to an error (a failed request clears that box's previous result). Never write "verified" or "TradingView-matched" in UI text. Show `needs_data` / `unsupported` / `invalid` / gateway messages exactly as given.
- gl and crypto: every chart, result, optimization and error shows the provisional note + `instrument.label`; Submit/Publish never enabled there.
- Theme: existing classes and tokens only; new CSS uses `var(--card)`, `var(--panel)`, `var(--thead)`, `var(--ink*)`, `var(--rule)`, `var(--steel)` — never `#fff`/`#000`. Lightweight Charts colours come from CSS variables at creation; charts remount on theme change.
- Tests: run only your own files while other agents work (`npx vitest run tests/<your>.test.tsx`); for typecheck, report only errors in your own files (`npx tsc --noEmit 2>&1 | grep -E '<your file names>'`). Never run a whole-corpus backtest; never start Docker; never contact 192.168.4.30; NAS data is read-only.
- Do not change Rust strategy calculations; do not write results, data, `.env` or screenshots into the repository (screenshots go to the session scratchpad).

## Review Focus

1. **A run that fails after a successful one** — expected: the old KPIs/trades disappear and only the error shows. Test: `a failed run clears the previous result` (Task 15).
2. **A saved strategy edited elsewhere (stale version, 409)** — expected: the server message, a reload that keeps the rules on screen, and the next save sends the new version. Test: `409 offers a reload that keeps the rules and saves with the new version` (Task 12).
3. **A Customize variant edited, saved and reopened** — expected: `spec.origin` is byte-for-byte the template's, and every request carries it. Tests: `edits keep origin and unknown keys` (Task 4) and `customize loads the template and runs with its origin` (Task 15).
4. **A spec that is legal for the engine but too long for the gateway's 16 KB query** — expected: a clear client message and no request. Test: `rejects specs that would exceed the gateway query cap` (Task 4).
5. **The shared optimization slot busy (429 Retry-After)** — expected: the message as given, a countdown, a manual retry; no automatic hammering. Test: `429 shows a countdown and re-enables retry` (Task 11).

---

## Existing code this phase relies on (read-only)

| Item | Where | Used for |
|---|---|---|
| `PageRegistration { path; label; component: ComponentType; showInNavigation? }`, `customPages` (empty) | `src/app/custom-pages.ts` | registering both pages (append only) |
| App renders `<Custom/>` (no props) when `resolveRoute(location.hash).path` matches exactly; no ErrorBoundary exists | `src/app/App.tsx` | sub-views go in the query; each page wraps itself in `LabErrorBoundary` |
| `resolveRoute(hash) → {name, path, id?, query}` | `src/app/routes.ts` | reading the page query |
| `apiJson`, `ApiError(message, status)`, `getAccount()` → `{user:{id,name,email,role}, settings}` | `src/services/api.ts` | role; `LabError extends ApiError` |
| `vaultStore.getSnapshot().state.region` | `src/state/store.ts` | region fallback when the hash has none |
| Classes `.box .box-h .metrics .metric .rnote .dbtn(.sm/.primary/.ghost) .join .join-item(.active) .dfield .tbl .tlog .tblwrap .empty .loading .lwc .chip .two .st` | `src/styles.css` | all markup |
| `window.LightweightCharts` (v4: `createChart`, `addCandlestickSeries`, `addLineSeries`, `addAreaSeries`, `series.setMarkers`, `PriceScaleMode.Logarithmic` = 1) | `public/vendor/lightweight-charts-4.2.0.js` via `index.html` | charts |
| `window.VaultReport.monthly({dates, equity}) → {months, annual, years, averages, annualAverage}` (equity must start at 1), `VaultReport.csv(rows)` | `Context/guru/frontend/report-data.js` via `index.html` | monthly table, CSV |
| mocked-chart test pattern | `tests/strategy-indicator-charts.test.tsx` | chart tests |
| act/createRoot pattern; `fetch` stubbing | `tests/library-controls.test.tsx`, `tests/api.test.ts` | component/page tests |
| Lab API contract | spec §3–§6 and `Context/guru/backend/src/lab/api.rs` (read-only) | client and types |

## File structure

```
src/features/lab/types.ts          API + spec types, LabErrorFields                       (Task 1)
src/features/lab/labels.ts         verbatim labels + fixed copy                            (Task 1)
src/features/lab/url-state.ts      page query read/write                                   (Task 1)
src/features/lab/globals.d.ts      window.LightweightCharts / VaultReport typings          (Task 1)
src/pages/IndicatorsPage.tsx       stub (Task 1) → full page (Task 15)
src/pages/YtStrategiesPage.tsx     stub (Task 1) → full page (Task 14)
src/app/custom-pages.ts            + 2 entries                                             (Task 1)
src/components/AppHeader.tsx       drop "YT Strategies soon", active custom link            (Task 1)
vite.config.ts                     coverage include list                                   (Task 1)
tests/fixtures/lab.ts              shared fixtures + fetch router + mount helper           (Task 1)
scripts/lab-dev-proxy.mjs          dev-only engine proxy without auth                      (Task 6)
scripts/lab-browser-check.mjs      dev-only headless Chrome check                          (Task 6)
src/features/lab/client.ts         lab API client                                          (Task 2)
src/features/lab/format.ts         number/percent formatting                               (Task 3)
src/features/lab/ui.tsx, lab.css   boundary, notices, segmented, tiles                     (Task 3)
src/features/lab/spec-model.ts     spec editing, paths, checks, query, reasons             (Task 4)
src/features/lab/charts.ts         createLabChart, useThemeKey                             (Task 5)
src/features/lab/bars.ts           candles, aggregation, line points, markers              (Task 5)
src/features/lab/catalogue.ts      search, groups, availability                            (Task 7)
src/features/lab/instruments.ts    instrument options, default ticker, saved market        (Task 7)
src/features/lab/Toolbar.tsx, LibraryRail.tsx, IndicatorDetail.tsx, library.css           (Task 7)
src/features/lab/Workbench.tsx, workbench.css                                              (Task 8)
src/features/lab/RuleBuilder.tsx, OperandEditor.tsx, OrdersPanel.tsx, rules.css            (Task 9)
src/features/lab/RunResults.tsx, results.css                                               (Task 10)
src/features/lab/OptimizePanel.tsx, optimize.css                                           (Task 11)
src/features/lab/SavedStrategies.tsx, ReviewQueue.tsx, saved.css                           (Task 12)
src/features/lab/CustomizePanel.tsx, customize.css                                         (Task 13)
src/features/lab/lab-state.ts      page reducer                                            (Task 15)
```

### DOM hooks (shared contract; the browser check and page tests use them)

| Attribute | Element | Task |
|---|---|---|
| `data-lab-page="indicators"` / `"yt-strategies"` | page root | 1 (stub), 14, 15 |
| `data-lab-library` | library rail root; items `button[data-lab-item="<id>"]` | 7 |
| `data-lab-chip="<key>"` | each chip | 8 |
| `data-lab-chart` | workbench chart host | 8 |
| `data-lab-action="run"` / `"optimize"` / `"save-new"` / `"save"` / `"fidelity"` | buttons | 9, 11, 12, 13 |
| `data-lab-result` | backtest result root | 10 |
| `data-lab-dim="<path>"` | optimize dimension row (contains the checkbox) | 11 |
| `data-lab-optimize-result` | optimization result root | 11 |
| `data-server-label` | element holding a server label verbatim | 3 |
| `data-lab-provisional` | provisional strip | 3 |
| `data-lab-error` | `ErrorNotice` root (also `role="alert"`) | 3 |

## Before Task 1 (once, by the coordinator)

- [ ] `cd /home/aff34/.cache/strategy-vault-development && git status --short && git log --oneline -1` — note the HEAD as `LABUI_BASE` (the spec/plan commits are its tip).
- [ ] `cd Context/guru/frontend/react && npm ci` (lockfile only; `node_modules` is absent on this checkout). Then `npm run typecheck && npm test` — must pass before any task starts.
- [ ] `scripts/install-hooks.sh` from the repo root if hooks are not installed.

---

### Task 1: Foundation — types, labels, URL state, fixtures, registration, header, coverage list

**Files:**
- Create: `src/features/lab/types.ts`, `src/features/lab/labels.ts`, `src/features/lab/url-state.ts`, `src/features/lab/globals.d.ts`, `src/pages/IndicatorsPage.tsx` (stub), `src/pages/YtStrategiesPage.tsx` (stub), `tests/fixtures/lab.ts`
- Modify: `src/app/custom-pages.ts` (append), `src/components/AppHeader.tsx` (nav block only), `vite.config.ts` (coverage `include` only)
- Test: `tests/lab-foundation.test.tsx`

**Interfaces:**
- Consumes: `resolveRoute` (`src/app/routes.ts`), `ApiError` (`src/services/api.ts`).
- Produces: every type below; `LAB_LABEL`, `TEMPLATE_LABEL`, `FIDELITY_LABEL`, `PROVISIONAL_NOTE`, `variantLabel(book, method)`, `unavailableText(reason)`, `COPY`; `readLabQuery(hash?)`, `labHash(path, patch, hash?)`, `writeLabQuery(patch)`; fixtures `catalogueFixture`, `tickersFixture`, `glInstrumentsFixture`, `seriesFixture(id, outputs, values)`, `runFixture`, `optimizeFixture`, `templatesFixture`, `templateFixture`, `unavailableTemplateFixture`, `fidelityFixture`, `strategyFixture(patch?)`, `publishedFixture`, `jsonResponse(body, status?, headers?)`, `fetchRouter(routes)`, `mount(node)`.

- [ ] **Step 1: Write the failing test** — `tests/lab-foundation.test.tsx`

```tsx
import { afterEach, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { customPages } from "../src/app/custom-pages";
import { AppHeader } from "../src/components/AppHeader";
import { labHash, readLabQuery, writeLabQuery } from "../src/features/lab/url-state";
import { unavailableText, variantLabel, LAB_LABEL, PROVISIONAL_NOTE } from "../src/features/lab/labels";
import type { ViewSnapshot } from "../src/state/store";

afterEach(() => history.replaceState(null, "", "/"));

it("registers both lab pages in navigation", () => {
  expect(customPages.map((p) => [p.path, p.label, p.showInNavigation])).toEqual([
    ["/indicators", "Indicators", true],
    ["/yt-strategies", "YT Strategies", true],
  ]);
});

it("shows one YT Strategies link, no 'soon' placeholder, and marks the open custom page", () => {
  history.replaceState(null, "", "/#/indicators?region=us");
  const snapshot = { route: { name: "home", path: "/", query: new URLSearchParams() }, status: "" } as unknown as ViewSnapshot;
  const html = renderToStaticMarkup(<AppHeader account={null} snapshot={snapshot} />);
  expect(html.match(/YT Strategies/g)).toHaveLength(1);
  expect(html).not.toContain("soon");
  expect(html).toContain('<a href="#/indicators" class="on">Indicators</a>');
  expect(html).not.toMatch(/data-r="home" class="on"/);
});

it("reads the page query with defaults and writes it without losing other keys", () => {
  expect(readLabQuery("#/indicators")).toEqual({ region: "in", tk: "", tf: "D", range: "3Y", customize: "", strategy: "", published: "" });
  expect(readLabQuery("#/indicators?region=gl&tk=CRYPTO%3ABTCUSD&tf=W&range=All").region).toBe("gl");
  expect(readLabQuery("#/indicators?region=xx").region).toBe("in");
  expect(labHash("/indicators", { tk: "NIFTY 50", customize: "" }, "#/indicators?region=us&customize=rsi2_4"))
    .toBe("#/indicators?region=us&tk=NIFTY+50");
  history.replaceState(null, "", "/#/indicators?region=us");
  writeLabQuery({ tf: "M" });
  expect(location.hash).toBe("#/indicators?region=us&tf=M");
});

it("keeps server labels verbatim and adds the unavailable prefix once", () => {
  expect(LAB_LABEL).toBe("User-defined indicator strategy · simulated · not a book method");
  expect(PROVISIONAL_NOTE).toBe("Provisional · TradingView export · internal research only, not publishable to YT Strategies");
  expect(variantLabel("Connors", "RSI 2")).toBe("Customized from Connors · RSI 2 · user variant · simulated");
  expect(unavailableText("the method is not admitted")).toBe("Customize not available — the method is not admitted");
  expect(unavailableText("Customize not available — x")).toBe("Customize not available — x");
});
```

- [ ] **Step 2: Run it to verify it fails**

Run: `npx vitest run tests/lab-foundation.test.tsx`
Expected: FAIL — cannot resolve `../src/features/lab/url-state`.

- [ ] **Step 3: Write `src/features/lab/types.ts`**

```ts
/** Lab API and rule-spec types (spec §5–§6; backend contract in Context/guru/backend/src/lab/api.rs). */
export type Market = "in" | "us" | "gl";
export type SavedMarket = Market | "crypto";
export type Role = "admin" | "researcher" | "viewer";
/** 'D' | 'W' | 'M' today; Phase 7 may add more through catalogue.timeframes. */
export type Timeframe = string;
export type ChartRange = "1Y" | "3Y" | "All";

export interface Provisional { provisional?: true; provisional_note?: string }
export interface IndicatorParam { key: string; label: string; default: number; min: number; max: number; step: number | null; integer: boolean }
export interface Needs { volume: boolean; intraday: boolean; breadth: boolean; second_symbol: boolean; fundamentals: boolean; derivatives: boolean; onchain: boolean }
export interface Indicator {
  id: string; name: string; category: string; placement: "price" | "pane"; needs: Needs; computed: boolean;
  params: IndicatorParam[]; outputs: string[]; first: (number | null)[]; sparse: boolean;
  definition: string; note: string; tv: string; ab: string;
}
export interface Catalogue extends Provisional {
  label: string; price_sources: string[]; operators: string[]; timeframes: Timeframe[];
  max_conditions: number; max_spec_bytes: number; max_combos: number; indicators: Indicator[];
}
export interface Instrument {
  ticker: string; type: string; native: boolean; market: SavedMarket | null; currency: string | null;
  name?: string; timezone?: string; calendar?: string; bars_per_year?: number; price?: string;
  has_volume?: boolean; provisional?: true; label?: string; source?: string; quirks?: string;
}
export interface TickerRow {
  ticker: string; first: string; last: string; has_volume: boolean; series: string[];
  symbol?: string; basket?: string; native?: boolean; asset_type?: string; price_status?: string;
}
export type GlInstrumentRow =
  | { instrument: Instrument; status: "ready"; first: string; last: string; bars: number; tradable_from: string | null }
  | { instrument: Instrument; status: "needs_data"; reason: string };
export interface InstrumentsResponse extends Provisional { label: string; instruments: GlInstrumentRow[] }
export interface SeriesResponse extends Provisional {
  label: string; instrument: Instrument; id: string; tf: Timeframe; params: Record<string, number>;
  outputs: string[]; dates: string[]; series: Record<string, (number | null)[]>; provenance: unknown;
}

// ---- rule spec v1 (Context/guru/backend/src/lab/rules.rs). Unknown keys are kept by every editor.
export interface ValueOperand { value: number }
export interface SeriesOperand { s: string; p?: Record<string, number>; tf?: Timeframe; out?: string; offset?: number }
export type Operand = ValueOperand | SeriesOperand;
export interface Cond { lhs: Operand; op: string; rhs: Operand; [extra: string]: unknown }
export type Dist = { pct: number } | { atr: number; atr_length?: number };
export interface Origin { book: string; method: string; template_version: number }
export interface OptimizeDim { path: string; from: number; to: number; step: number }
export type Fill = "next_open" | "close";
export interface Spec {
  v: 1;
  direction?: "long" | "short";
  origin?: Origin;
  entry: { all: Cond[]; limit?: Dist; [extra: string]: unknown };
  exit: { any: Cond[]; stop_loss?: Dist; target?: Dist; trailing?: Dist; [extra: string]: unknown };
  execution?: { fill?: Fill; exit_fill?: Fill; cost_bps?: number; [extra: string]: unknown };
  optimize?: OptimizeDim[];
  [extra: string]: unknown;
}

// ---- run / optimize
export interface Trade {
  entry: string; exit: string; entry_i: number; exit_i: number; entry_px: number | null; exit_px: number | null;
  ret: number | null; bars: number; open: boolean; signal: string; reason: string; dir: 1 | -1; entry_kind: string;
}
export type Stats = Record<string, number | null>;
export interface Costs {
  fees_bps: number; slippage_bps: number; rollover_bps: number; total_side_bps: number; taxes: null; rolls: null;
  basis: string; rollover_status: string; slippage_status: string; money_status: string; open_trade_cost_basis: string;
}
export interface BookMethod { id: string; name: string; book: string; group: string; dir: string }
export interface ResponseOrigin { method: BookMethod; template_version: number }
export type Start = null | { date: string; reason: string };
export interface RunResult extends Provisional {
  label: string; kind: "lab_simulation"; origin: ResponseOrigin | null; instrument: Instrument;
  from: string; to: string; start: Start; dates: string[]; close: number[]; equity: number[];
  gross_equity: number[]; bh: number[]; trades: Trade[]; stats: Stats; gross_stats: Stats; costs: Costs; provenance: unknown;
}
export interface Cand { row: number; score: number; weakest: number; pct: number[]; vs_leader: (number | null)[]; groups: string[] }
export interface Rank {
  eligible: number; distinct: number; total: number; min_trades: number; per_year: number; stats?: string[];
  groups: { stat: string; label: string; rows: number[] }[]; pool: Cand[]; top3: Cand[];
}
export interface OptimizeResult extends Provisional {
  label: string; kind: "lab_optimization"; origin: ResponseOrigin | null; instrument: Instrument;
  from: string; to: string; start: Start; split: string | null; combos: number; threads: number; ms: number;
  dims: { path: string; values: number[] }[];
  /** dims paths first, then metric names; `rows`/`picked` rows align with cols; `base` aligns with cols after the dims. */
  cols: string[]; base: number[]; ranked_on: "in_sample" | "full"; rank: Rank;
  picked: Record<string, number[]>; rows: number[][]; costs: Costs; provenance: unknown;
}

// ---- Customize
export interface TemplateListItem { method: BookMethod; template_version: number; requirement: string; allowed_types: string[] }
export interface TemplatesResponse { label: string; methods: number; admitted: number; templates: TemplateListItem[] }
export type TemplateResponse =
  | { label: string; method: BookMethod; status: "unavailable"; reason: string }
  | { label: string; method: BookMethod; status: "available"; instrument: { requirement: string; allowed_types: string[] };
      template: { version: number; spec: Spec; notes: string[] } };
export interface FTrade { entry: string; exit: string; entry_px: number | null; exit_px: number | null; dir: number; legs: number; open: boolean }
export interface FidelityResult {
  label: string; kind: "lab_template_fidelity"; method: BookMethod; template_version: number; instrument: Instrument;
  from: string; to: string; book_trades: number; template_trades: number; matched: number; reproduces: boolean;
  differences_total: number; differences: { book: FTrade | null; template: FTrade | null }[]; note: string;
}

// ---- saved strategies (/api/app/lab/strategies)
export type StrategyStatus = "draft" | "submitted" | "published" | "rejected";
export interface LabStrategy {
  id: number; owner: string; ownerName: string; name: string; spec: Spec; origin: Origin | null;
  market: SavedMarket; instrument: string; status: StrategyStatus; version: number;
  submittedAt: string | null; reviewedAt: string | null; reviewNote: string | null;
  createdAt: string; updatedAt: string; publishable: boolean;
}
export interface PublishedStrategy {
  snapshotId: number; id: number | null; version: number; name: string; spec: Spec; origin: Origin | null;
  market: "in" | "us"; instrument: string; ownerName: string | null; publishedAt: string;
}
export interface StrategyDraft { name: string; spec: Spec; market: SavedMarket; instrument: string }

// ---- errors (LabError in client.ts implements this; UI reads it duck-typed)
export type EngineStatus = "invalid" | "unsupported" | "needs_data" | "not_found";
export interface LabErrorFields {
  message: string; status: number; kind: "engine" | "gateway" | "auth";
  engineStatus?: EngineStatus; label?: string; provisionalNote?: string; retryAfter?: number;
}
```

- [ ] **Step 4: Write `src/features/lab/labels.ts`**

```ts
/** Server labels and fixed copy, verbatim (spec §2, §3, §6). Grep here before writing any UI text. */
export const LAB_LABEL = "User-defined indicator strategy · simulated · not a book method";
export const TEMPLATE_LABEL = "Book template · declared adaptation into lab rules · not a verification";
export const FIDELITY_LABEL = "Template fidelity · display only · not a verification";
export const PROVISIONAL_NOTE = "Provisional · TradingView export · internal research only, not publishable to YT Strategies";
export const UNAVAILABLE_PREFIX = "Customize not available — ";
export const variantLabel = (book: string, method: string) => `Customized from ${book} · ${method} · user variant · simulated`;
export const unavailableText = (reason: string) => (reason.startsWith(UNAVAILABLE_PREFIX) ? reason : UNAVAILABLE_PREFIX + reason);

export const COPY = {
  lead: "Pick indicators from the library, chart them on any market, turn them into entry and exit rules, then backtest and optimize. Saved strategies can be submitted to YT Strategies.",
  chartHint: "Click a library item to add it · edit settings in the chips",
  noChips: "No indicators on the chart. Pick one from the library.",
  chartNote: "Indicators are calculated from the full history, then the visible range is shown.",
  inProgress: (tf: string) => `The last ${tf === "W" ? "weekly" : tf === "M" ? "monthly" : "period"} bar is still in progress; its values can change until it closes.`,
  wmRule: "Weekly and monthly values are used only after that bar closes.",
  notChecked: "Not yet checked against TradingView.",
  rulesHint: "values from completed bars only",
  entryTitle: "Entry · all of these",
  exitTitle: "Exit · any of these",
  addCondition: "+ Add condition",
  emptyEntry: "Add at least one entry condition.",
  keptAsIs: "Kept as is: this page cannot edit this rule type yet.",
  startMoved: (date: string) => `Results start on ${date}, not the requested start: earlier opens on this instrument are not reliable enough for next-open fills.`,
  noTrades: "No trades. The entry conditions never lined up on this instrument.",
  researcherOnly: "Researcher access is required for optimization.",
  optimizeLimit: "The engine stops an optimization after 300 s.",
  tickOne: "Tick at least one setting",
  splitLine: (pct: number, date: string) => `Out-of-sample: the last ${pct}% (from ${date}) is held back · settings are ranked on the earlier part only`,
  splitNote: "In-sample and held-back trade counts can differ from the full run: trades that cross the split date are cut at the boundary. A strong ranking score with a weak held-back result usually means curve fitting.",
  noEligible: (min: number) => `No setting had enough trades to rank (at least ${min}). Nothing is ranked.`,
  emptyOptimize: "Run the optimization to rank settings. Results show the ranking period and the held-back period side by side.",
  runFirst: "Run the current rules first; the engine checks the rules only when they run.",
  provisionalSubmit: "Provisional data cannot be published. Rerun on licensed data first.",
  savedEmpty: "Nothing saved yet. Name the current rules above and save them.",
  keptAfterReload: (version: number) => `Your rules on screen are kept. Save again to replace version ${version}.`,
  deleteConfirm: (name: string) => `Delete "${name}"? This cannot be undone.`,
  submitted: "Submitted. An admin reviews it before it appears in YT Strategies.",
  customizeGlOnly: "Customize runs on India and US book methods only.",
  fidelityNothing: "No trades on this window, so there is nothing to compare.",
  ytIntro: "Published lab strategies. Each one is a simulated, user-defined rule set reviewed for presentation — not a book method and not a verification.",
  ytEmpty: "Nothing is published yet.",
  tooLong: "These rules are too long to send. Remove a condition or shorten settings.",
} as const;
```

- [ ] **Step 5: Write `src/features/lab/url-state.ts`**

```ts
import { resolveRoute } from "../../app/routes";
import type { ChartRange, Market } from "./types";

export interface LabQuery { region: Market; tk: string; tf: string; range: ChartRange; customize: string; strategy: string; published: string }
const DEFAULTS: LabQuery = { region: "in", tk: "", tf: "D", range: "3Y", customize: "", strategy: "", published: "" };

export function readLabQuery(hash = location.hash): LabQuery {
  const q = resolveRoute(hash).query;
  const region = q.get("region");
  const range = q.get("range");
  return {
    region: region === "us" || region === "gl" ? region : "in",
    tk: q.get("tk") ?? "",
    tf: q.get("tf") || "D",
    range: range === "1Y" || range === "All" ? range : "3Y",
    customize: q.get("customize") ?? "",
    strategy: q.get("strategy") ?? "",
    published: q.get("published") ?? "",
  };
}

/** Defaults and empty values are omitted; other keys in the current query are kept. */
export function labHash(path: string, patch: Partial<LabQuery>, hash = location.hash): string {
  const q = new URLSearchParams(resolveRoute(hash).query);
  for (const [key, value] of Object.entries(patch)) {
    if (value === "" || value === undefined || value === DEFAULTS[key as keyof LabQuery]) q.delete(key);
    else q.set(key, String(value));
  }
  const search = q.toString();
  return `#${path}${search ? `?${search}` : ""}`;
}

/** location.replace fires hashchange (so App saves lastView) without adding a history entry. */
export function writeLabQuery(patch: Partial<LabQuery>, path = resolveRoute(location.hash).path) {
  const next = labHash(path, patch);
  if (next !== location.hash) location.replace(next);
}
```

Note: `region=in` is the default, so `labHash` drops it; `readLabQuery` restores it.

- [ ] **Step 6: Write `src/features/lab/globals.d.ts`**

```ts
/** Minimal typings for vendored globals loaded by index.html (no npm dependency). */
export interface LabSeriesApi {
  setData(points: unknown[]): void;
  setMarkers?(markers: unknown[]): void;
  applyOptions(options: Record<string, unknown>): void;
}
export interface LabChartApi {
  addCandlestickSeries(options: Record<string, unknown>): LabSeriesApi;
  addLineSeries(options: Record<string, unknown>): LabSeriesApi;
  addAreaSeries(options: Record<string, unknown>): LabSeriesApi;
  timeScale(): { fitContent(): void; setVisibleRange(range: { from: string; to: string }): void };
  applyOptions(options: Record<string, unknown>): void;
  resize(width: number, height: number): void;
  remove(): void;
}
export interface LabChartLibrary { createChart(element: HTMLElement, options: Record<string, unknown>): LabChartApi }
export interface VaultReportMonthly {
  months: Record<string, number | null>; annual: Record<string, number | null>; years: string[];
  averages: (number | null)[]; annualAverage: number | null;
}
export interface VaultReportApi {
  monthly(d: { dates: string[]; equity: number[] }): VaultReportMonthly;
  csv(rows: unknown[][]): string;
}
declare global {
  interface Window { LightweightCharts?: LabChartLibrary; VaultReport?: VaultReportApi }
}
```

- [ ] **Step 7: Write the page stubs**

`src/pages/IndicatorsPage.tsx`:
```tsx
/** Stub; Task 15 replaces the body. */
export function IndicatorsPage() {
  return <div className="lab-page" data-lab-page="indicators"><div className="loading" role="status">Loading the Indicators lab…</div></div>;
}
```
`src/pages/YtStrategiesPage.tsx`:
```tsx
/** Stub; Task 14 replaces the body. */
export function YtStrategiesPage() {
  return <div className="lab-page" data-lab-page="yt-strategies"><div className="loading" role="status">Loading YT Strategies…</div></div>;
}
```

- [ ] **Step 8: Append the registrations** — `src/app/custom-pages.ts` (append only; keep every existing line)

Add after the existing `import type { ComponentType } from "react";` line:
```ts
import { IndicatorsPage } from "../pages/IndicatorsPage";
import { YtStrategiesPage } from "../pages/YtStrategiesPage";
```
and after `export const customPages: PageRegistration[] = [];`:
```ts
customPages.push(
  { path: "/indicators", label: "Indicators", component: IndicatorsPage, showInNavigation: true },
  { path: "/yt-strategies", label: "YT Strategies", component: YtStrategiesPage, showInNavigation: true },
);
```
(`push` keeps the edit purely additive for `check_book_untouched.py`.)

- [ ] **Step 9: Edit the nav in `src/components/AppHeader.tsx`**

Add `import { resolveRoute } from "../app/routes";`. Replace `const active = snapshot.route?.name;` with:
```tsx
  const customPath = resolveRoute(location.hash).path;
  const onCustom = customPages.some((page) => page.path === customPath);
  // The runtime maps unknown paths to 'home'; on a custom page no research link is active.
  const active = onCustom ? undefined : snapshot.route?.name;
```
Replace the custom-link map and delete the placeholder:
```tsx
          {customPages
            .filter((page) => page.showInNavigation)
            .map((page) => (
              <a href={`#${page.path}`} key={page.path} className={customPath === page.path ? "on" : ""}>
                {page.label}
              </a>
            ))}
```
(the three lines `<a className="off">`, `YT Strategies<em>soon</em>`, `</a>` are removed).

- [ ] **Step 10: Add the coverage entries** — in `vite.config.ts` `test.coverage.include`, append after `"src/components/ResearchMarkup.tsx",`:
```ts
        "src/features/lab/url-state.ts",
        "src/features/lab/labels.ts",
        "src/features/lab/client.ts",
        "src/features/lab/format.ts",
        "src/features/lab/spec-model.ts",
        "src/features/lab/charts.ts",
        "src/features/lab/bars.ts",
        "src/features/lab/catalogue.ts",
        "src/features/lab/instruments.ts",
        "src/features/lab/lab-state.ts",
```

- [ ] **Step 11: Write the shared fixtures** — `tests/fixtures/lab.ts`

```ts
import { act, type ReactNode } from "react";
import { createRoot, type Root } from "react-dom/client";
import type {
  Catalogue, FidelityResult, Indicator, InstrumentsResponse, LabStrategy, OptimizeResult, PublishedStrategy,
  RunResult, SeriesResponse, TemplateResponse, TemplatesResponse, TickerRow,
} from "../../src/features/lab/types";

const needs = { volume: false, intraday: false, breadth: false, second_symbol: false, fundamentals: false, derivatives: false, onchain: false };
const ind = (patch: Partial<Indicator> & Pick<Indicator, "id" | "name" | "category">): Indicator => ({
  placement: "price", needs, computed: true, params: [], outputs: [patch.id], first: [0], sparse: false,
  definition: `${patch.id}(close)`, note: "", tv: "", ab: "", ...patch,
});
const len = (def: number) => ({ key: "length", label: "Length", default: def, min: 1, max: 5000, step: 1, integer: true });

export const catalogueFixture: Catalogue = {
  label: "User-defined indicator strategy · simulated · not a book method",
  price_sources: ["open", "high", "low", "close", "hl2", "hlc3", "ohlc4", "volume"],
  operators: ["crosses_above", "crosses_below", ">", "<", ">=", "<="],
  timeframes: ["D", "W", "M"], max_conditions: 12, max_spec_bytes: 8192, max_combos: 50000,
  indicators: [
    ind({ id: "ema", name: "Exponential Moving Average", category: "Moving averages", params: [len(9)], tv: "ta.ema", ab: "EMA" }),
    ind({ id: "rsi", name: "Relative Strength Index", category: "Oscillators", placement: "pane", params: [len(14)], tv: "ta.rsi", ab: "RSI" }),
    ind({ id: "macd", name: "MACD", category: "Oscillators", placement: "pane", outputs: ["macd", "signal", "hist"], first: [25, 33, 33],
      params: [{ key: "fast", label: "Fast", default: 12, min: 1, max: 500, step: 1, integer: true },
               { key: "slow", label: "Slow", default: 26, min: 1, max: 500, step: 1, integer: true },
               { key: "signal", label: "Signal", default: 9, min: 1, max: 500, step: 1, integer: true }] }),
    ind({ id: "stoch", name: "Stochastic", category: "Oscillators", placement: "pane", outputs: ["k", "d"],
      params: [len(14), { key: "variant", label: "Variant", default: 0, min: 0, max: 1, step: 1, integer: true }] }),
    ind({ id: "obv", name: "On-Balance Volume", category: "Volume", placement: "pane", needs: { ...needs, volume: true } }),
    ind({ id: "ad_line", name: "Advance/Decline Line", category: "Breadth", placement: "pane", needs: { ...needs, breadth: true } }),
    ind({ id: "fractals", name: "Williams Fractals", category: "Patterns", outputs: ["up", "down"], sparse: true }),
    ind({ id: "doji", name: "Doji", category: "Patterns", outputs: ["doji"] }),
    ind({ id: "or_range", name: "Opening Range", category: "Opening range", computed: false, first: [null],
      needs: { ...needs, intraday: true }, note: "Needs intraday (minute) bars; not available yet." }),
  ],
};

export const tickersFixture: TickerRow[] = [
  { ticker: "NIFTY 50", first: "2000-01-03", last: "2026-10-14", has_volume: false, series: [] },
  { ticker: "NIFTYBANK", first: "2000-01-03", last: "2026-10-14", has_volume: false, series: [] },
  { ticker: "NSE:RELIANCE", symbol: "RELIANCE", basket: "NIFTY 50", first: "2005-01-03", last: "2026-10-14",
    has_volume: true, series: [], native: true, asset_type: "individual_stock", price_status: "ready" },
];

export const glInstrumentsFixture: InstrumentsResponse = {
  provisional: true, provisional_note: "Provisional · TradingView export · internal research only, not publishable to YT Strategies",
  label: "User-defined indicator strategy · simulated · not a book method",
  instruments: [
    { status: "ready", first: "2014-01-02", last: "2026-10-10", bars: 4300, tradable_from: "2014-02-01",
      instrument: { ticker: "CRYPTO:BTCUSD", type: "crypto", native: false, market: "crypto", currency: "USD", name: "Bitcoin / US dollar",
        has_volume: false, provisional: true, label: "Provisional · TradingView export CRYPTO:BTCUSD, daily" } },
    { status: "needs_data", reason: "IG:DAX is not staged on this server",
      instrument: { ticker: "IG:DAX", type: "cfd", native: false, market: "gl", currency: "EUR", name: "DAX", has_volume: false, provisional: true,
        label: "Provisional · TradingView export IG:DAX, daily" } },
  ],
};

const dates = ["2024-01-05", "2024-01-08", "2024-01-09", "2024-01-15", "2024-02-01", "2024-02-02"];
export const seriesFixture = (id: string, outputs: string[], values: Record<string, (number | null)[]>): SeriesResponse => ({
  label: catalogueFixture.label, id, tf: "D", params: {}, outputs, dates, series: values, provenance: null,
  instrument: { ticker: "NIFTY 50", type: "index", native: false, market: "in", currency: "INR" },
});

export const runFixture: RunResult = {
  label: "User-defined indicator strategy · simulated · not a book method", kind: "lab_simulation", origin: null,
  instrument: { ticker: "NIFTY 50", type: "index", native: false, market: "in", currency: "INR" },
  from: dates[0], to: dates[5], start: { date: "2024-01-08", reason: "stale opens before 2024-01-08" }, dates,
  close: [100, 101, 103, 102, 105, 106], equity: [1, 1, 1.02, 1.01, 1.04, 1.05], gross_equity: [1, 1, 1.021, 1.011, 1.041, 1.051],
  bh: [1, 1.01, 1.03, 1.02, 1.05, 1.06],
  trades: [
    { entry: "2024-01-08", exit: "2024-01-15", entry_i: 1, exit_i: 3, entry_px: 101, exit_px: 102, ret: 0.0099, bars: 2, open: false,
      signal: "2024-01-09", reason: "exit.any[0]: bars_held >= 2", dir: 1, entry_kind: "next_open" },
    { entry: "2024-02-01", exit: "2024-02-02", entry_i: 4, exit_i: 5, entry_px: 105, exit_px: 106, ret: -0.0095, bars: 1, open: true,
      signal: "", reason: "open", dir: -1, entry_kind: "close" },
  ],
  stats: { total_return: 0.05, cagr: 0.31, max_dd: -0.0098, sharpe: 1.42, trades: 2, win_rate: 0.5, avg_trade: 0.0002, bh_cagr: 0.4,
    exposure: 0.6, gross_total_return: 0.051 },
  gross_stats: { total_return: 0.051 },
  costs: { fees_bps: 5, slippage_bps: 2, rollover_bps: 0, total_side_bps: 7, taxes: null, rolls: null, basis: "per side",
    rollover_status: "none", slippage_status: "applied", money_status: "normalized", open_trade_cost_basis: "entry only" },
  provenance: null,
};

const P = "entry.all[0].rhs.p.length";
export const optimizeFixture: OptimizeResult = {
  label: runFixture.label, kind: "lab_optimization", origin: null, instrument: runFixture.instrument,
  from: "2010-01-04", to: "2026-10-14", start: null, split: "2022-01-03", combos: 3, threads: 2, ms: 900,
  dims: [{ path: P, values: [10, 20, 30] }],
  cols: [P, "total_return", "cagr", "sharpe", "max_dd", "trades", "monthly_thickness", "expectancy_ratio", "mar",
    "is_total_return", "is_cagr", "is_sharpe", "is_trades", "oos_total_return", "oos_cagr", "oos_sharpe", "oos_trades"],
  base: [0.5, 0.05, 0.6, -0.2, 40, 0.3, 0.1, 0.25, 0.4, 0.05, 0.6, 30, 0.1, 0.04, 0.5, 10],
  ranked_on: "in_sample",
  rank: { eligible: 3, distinct: 3, total: 3, min_trades: 10, per_year: 1, stats: ["cagr", "sharpe", "monthly_thickness", "expectancy_ratio", "mar"],
    groups: [], pool: [],
    top3: [{ row: 1, score: 0.9, weakest: 0.8, pct: [90, 85, 80, 95, 88], vs_leader: [null, null, null, null, null], groups: [] },
           { row: 0, score: 0.7, weakest: 0.6, pct: [70, 65, 60, 75, 68], vs_leader: [-0.01, -0.1, -0.05, -0.02, -0.03], groups: [] }] },
  picked: {
    "1": [20, 0.7, 0.07, 0.8, -0.18, 35, 0.35, 0.12, 0.39, 0.6, 0.07, 0.8, 28, 0.08, 0.03, 0.4, 7],
    "0": [10, 0.6, 0.06, 0.7, -0.19, 45, 0.31, 0.11, 0.32, 0.5, 0.06, 0.7, 33, 0.09, 0.035, 0.45, 12],
  },
  rows: [], costs: runFixture.costs, provenance: null,
};

const method = { id: "rsi2_4", name: "RSI 2 · 4", book: "Short Term Trading Strategies That Work", group: "RSI", dir: "long" };
export const templatesFixture: TemplatesResponse = {
  label: "Book template · declared adaptation into lab rules · not a verification", methods: 4365, admitted: 164,
  templates: [
    { method, template_version: 1, requirement: "index or ETF", allowed_types: ["index", "etf"] },
    { method: { ...method, id: "hpetf_rsi", name: "High-probability ETF RSI", book: "High Probability ETF Trading" }, template_version: 1,
      requirement: "actual ETF prices", allowed_types: ["etf"] },
  ],
};
export const templateFixture: TemplateResponse = {
  label: templatesFixture.label, method, status: "available",
  instrument: { requirement: "index or ETF", allowed_types: ["index", "etf"] },
  template: { version: 1, notes: ["Entry at the close, as in the book."],
    spec: { v: 1, origin: { book: "ststw", method: "rsi2_4", template_version: 1 }, execution: { fill: "close" },
      entry: { all: [{ lhs: { s: "rsi", p: { length: 2 } }, op: "<", rhs: { value: 4 } }] },
      exit: { any: [{ lhs: { s: "close" }, op: ">", rhs: { s: "sma", p: { length: 5 } } }] } } },
};
export const unavailableTemplateFixture: TemplateResponse = { label: templatesFixture.label, method, status: "unavailable", reason: "the book method is not admitted" };
export const fidelityFixture: FidelityResult = {
  label: "Template fidelity · display only · not a verification", kind: "lab_template_fidelity", method, template_version: 1,
  instrument: runFixture.instrument, from: "2000-01-03", to: "2026-10-14", book_trades: 12, template_trades: 12, matched: 12,
  reproduces: true, differences_total: 0, differences: [],
  note: "Compares the template's simulated trades with the book engine's simulated trades on the same data, window and default parameters, before costs. It does not verify the book method, its published figures or either engine.",
};
export const strategyFixture = (patch: Partial<LabStrategy> = {}): LabStrategy => ({
  id: 7, owner: "u1", ownerName: "Nandini", name: "EMA pullback", spec: { v: 1,
    entry: { all: [{ lhs: { s: "close" }, op: ">", rhs: { s: "ema", p: { length: 50 } } }] },
    exit: { any: [{ lhs: { s: "bars_held" }, op: ">=", rhs: { value: 5 } }] } },
  origin: null, market: "in", instrument: "NIFTY 50", status: "draft", version: 3, submittedAt: null, reviewedAt: null,
  reviewNote: null, createdAt: "2026-10-10T10:00:00Z", updatedAt: "2026-10-12T10:00:00Z", publishable: true, ...patch,
});
export const publishedFixture: PublishedStrategy[] = [
  { snapshotId: 11, id: 7, version: 3, name: "EMA pullback", spec: strategyFixture().spec, origin: null, market: "in",
    instrument: "NIFTY 50", ownerName: "Nandini", publishedAt: "2026-10-13T09:00:00Z" },
  { snapshotId: 12, id: null, version: 1, name: "RSI 2 variant", spec: templateFixture.status === "available" ? templateFixture.template.spec : strategyFixture().spec,
    origin: { book: "ststw", method: "rsi2_4", template_version: 1 }, market: "us", instrument: "SPY", ownerName: null,
    publishedAt: "2026-10-14T09:00:00Z" },
];

export const jsonResponse = (body: unknown, status = 200, headers: Record<string, string> = {}) =>
  new Response(body === null ? null : JSON.stringify(body), { status, headers: { "Content-Type": "application/json", ...headers } });

/** fetch stub: first route whose pattern matches the URL (string = prefix) answers; unmatched → 404. Records calls. */
export function fetchRouter(routes: [RegExp | string, (url: string, init?: RequestInit) => Response | Promise<Response>][]) {
  const calls: { url: string; init?: RequestInit }[] = [];
  const fn = async (url: string, init?: RequestInit) => {
    calls.push({ url, init });
    const hit = routes.find(([p]) => (typeof p === "string" ? url.startsWith(p) : p.test(url)));
    return hit ? hit[1](url, init) : jsonResponse({ error: "Unknown endpoint." }, 404);
  };
  return Object.assign(fn, { calls });
}

let root: Root | undefined;
let host: HTMLDivElement | undefined;
/** act/createRoot mount (pattern of tests/library-controls.test.tsx). Call unmount() in afterEach. */
export async function mount(node: ReactNode) {
  Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
  host = document.createElement("div");
  document.body.append(host);
  root = createRoot(host);
  await act(async () => root!.render(node));
  return host;
}
export async function unmount() {
  if (root) await act(async () => root!.unmount());
  host?.remove();
  root = undefined;
  host = undefined;
}
/** Let pending promises and effects settle inside act. */
export const flush = () => act(async () => { for (let i = 0; i < 5; i++) await Promise.resolve(); await new Promise((r) => setTimeout(r, 0)); });
```

- [ ] **Step 12: Run the test and the existing suite**

Run: `npx vitest run tests/lab-foundation.test.tsx && npm test && npm run typecheck`
Expected: PASS (all existing tests stay green).

- [ ] **Step 13: Commit**

```bash
cd /home/aff34/.cache/strategy-vault-development
F=Context/guru/frontend/react
P="$F/src/features/lab/types.ts $F/src/features/lab/labels.ts $F/src/features/lab/url-state.ts $F/src/features/lab/globals.d.ts $F/src/pages/IndicatorsPage.tsx $F/src/pages/YtStrategiesPage.tsx $F/src/app/custom-pages.ts $F/src/components/AppHeader.tsx $F/vite.config.ts $F/tests/fixtures/lab.ts $F/tests/lab-foundation.test.tsx"
git add -- $P && git commit -m "feat(lab-ui): lab foundation, page registration and header link" -- $P
```

---

### Task 2: Lab API client (`client.ts`)

**Files:**
- Create: `src/features/lab/client.ts`
- Test: `tests/lab-client.test.ts`

**Interfaces:**
- Consumes: `ApiError` (`src/services/api.ts`); types from Task 1.
- Produces:
  - `class LabError extends ApiError implements LabErrorFields` — `kind`, `engineStatus?`, `label?`, `provisionalNote?`, `retryAfter?`.
  - `engineBase(m: SavedMarket): "/api/in" | "/api/us" | "/api/gl"`
  - `labRequest<T>(path: string, init?: RequestInit): Promise<T>` (204 → `null`)
  - `catalogue(m)`, `series(m, {id, ticker, tf?, p?})`, `run(m, spec, q: RunQuery)`, `optimize(m, spec, q: RunQuery & {split?: string | null})`, `tickers(m: "in" | "us")`, `labInstruments()`, `templates(m)`, `template(m, method)`, `fidelity(m, method, ticker, from?, to?)`, `strategies.{list, published, review, get, create, update, remove, duplicate, action}`, `clearLabCache()`.
  - `RunQuery = { ticker: string; from?: string; to?: string; cost?: number; slippage?: number }`.

- [ ] **Step 1: Write the failing test** — `tests/lab-client.test.ts`

```ts
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import * as lab from "../src/features/lab/client";
import { ApiError } from "../src/services/api";
import { catalogueFixture, jsonResponse, fetchRouter, runFixture, strategyFixture } from "./fixtures/lab";

beforeEach(() => lab.clearLabCache());
afterEach(() => vi.unstubAllGlobals());

it("routes markets to engine bases; crypto runs on gl", () => {
  expect(["in", "us", "gl", "crypto"].map((m) => lab.engineBase(m as never))).toEqual(["/api/in", "/api/us", "/api/gl", "/api/gl"]);
});

it("encodes spec, ticker and p.* settings and always sends the ticker", async () => {
  const f = fetchRouter([["/api/", () => jsonResponse(runFixture)]]);
  vi.stubGlobal("fetch", f);
  await lab.run("in", { v: 1, entry: { all: [] }, exit: { any: [] } }, { ticker: "NIFTY 50", cost: 5 });
  await lab.series("gl", { id: "ema", ticker: "CRYPTO:BTCUSD", tf: "W", p: { length: 20 } });
  const [runUrl, seriesUrl] = f.calls.map((c) => new URL(c.url, "http://x"));
  expect(runUrl.pathname).toBe("/api/in/lab/run");
  expect(runUrl.searchParams.get("ticker")).toBe("NIFTY 50");
  expect(JSON.parse(runUrl.searchParams.get("spec")!).v).toBe(1);
  expect(runUrl.searchParams.get("cost")).toBe("5");
  expect(runUrl.searchParams.has("slippage")).toBe(false);
  expect(seriesUrl.pathname).toBe("/api/gl/lab/series");
  expect(seriesUrl.searchParams.get("p.length")).toBe("20");
  expect(f.calls[0].init?.credentials).toBe("same-origin");
});

it("optimize always asks for slim rows and sends split only when given", async () => {
  const f = fetchRouter([["/api/", () => jsonResponse({})]]);
  vi.stubGlobal("fetch", f);
  await lab.optimize("us", { v: 1, entry: { all: [] }, exit: { any: [] } }, { ticker: "SPY", split: "2020-01-02" });
  await lab.optimize("us", { v: 1, entry: { all: [] }, exit: { any: [] } }, { ticker: "SPY", split: null });
  const [a, b] = f.calls.map((c) => new URL(c.url, "http://x").searchParams);
  expect(a.get("slim")).toBe("1");
  expect(a.get("split")).toBe("2020-01-02");
  expect(b.has("split")).toBe(false);
});

it("keeps engine error fields and never returns numbers", async () => {
  vi.stubGlobal("fetch", fetchRouter([["/api/gl/lab/run", () => jsonResponse({ provisional: true, provisional_note: "Provisional · x",
    status: "needs_data", error: "window starts before the first admitted bar", label: "User-defined indicator strategy · simulated · not a book method" }, 400)]]));
  const e = await lab.run("gl", { v: 1, entry: { all: [] }, exit: { any: [] } }, { ticker: "CRYPTO:BTCUSD" }).catch((x) => x);
  expect(e).toBeInstanceOf(lab.LabError);
  expect(e).toBeInstanceOf(ApiError);
  expect([e.kind, e.status, e.engineStatus, e.provisionalNote, e.message]).toEqual(
    ["engine", 400, "needs_data", "Provisional · x", "window starts before the first admitted bar"]);
});

it("reads gateway errors, Retry-After and non-JSON bodies", async () => {
  vi.stubGlobal("fetch", fetchRouter([
    ["/api/in/lab/optimize", () => jsonResponse({ error: "An optimization is already running. Please retry when it finishes." }, 429, { "Retry-After": "7" })],
    ["/api/us/lab/run", () => new Response("<html>bad gateway</html>", { status: 502 })],
    ["/api/in/lab/run", () => jsonResponse({ error: "Busy" }, 429)],
  ]));
  const spec = { v: 1 as const, entry: { all: [] }, exit: { any: [] } };
  const busy = await lab.optimize("in", spec, { ticker: "NIFTY 50" }).catch((x) => x);
  expect([busy.kind, busy.status, busy.retryAfter]).toEqual(["gateway", 429, 7]);
  expect((await lab.run("in", spec, { ticker: "NIFTY 50" }).catch((x) => x)).retryAfter).toBe(5);
  const bad = await lab.run("us", spec, { ticker: "SPY" }).catch((x) => x);
  expect([bad.kind, bad.message]).toEqual(["gateway", "Request failed (502)."]);
});

it("caches the catalogue per market and drops a failed load", async () => {
  let n = 0;
  const f = fetchRouter([["/api/in/lab/catalogue", () => (++n === 1 ? jsonResponse({ error: "x" }, 503) : jsonResponse(catalogueFixture))]]);
  vi.stubGlobal("fetch", f);
  await expect(lab.catalogue("in")).rejects.toThrow("x");
  await lab.catalogue("in");
  await lab.catalogue("in");
  expect(f.calls).toHaveLength(2);
});

it("limits engine reads to four in flight", async () => {
  let active = 0, peak = 0;
  vi.stubGlobal("fetch", async () => { active++; peak = Math.max(peak, active); await new Promise((r) => setTimeout(r, 5)); active--; return jsonResponse({}); });
  await Promise.all(Array.from({ length: 7 }, (_, i) => lab.series("in", { id: `x${i}`, ticker: "NIFTY 50" })));
  expect(peak).toBe(4);
});

it("saved strategies: version on every mutation, 204 delete, auth errors", async () => {
  const f = fetchRouter([
    [/\/7\?version=3$/, () => new Response(null, { status: 204 })],
    [/\/7\/submit$/, () => jsonResponse({ error: "This strategy changed since you opened it (now version 4). Reload before saving." }, 409)],
    ["/api/app/lab/strategies", () => jsonResponse(strategyFixture(), 201)],
  ]);
  vi.stubGlobal("fetch", f);
  const draft = { name: "x", spec: strategyFixture().spec, market: "crypto" as const, instrument: "CRYPTO:BTCUSD" };
  await lab.strategies.create(draft);
  await lab.strategies.update(7, 3, draft);
  expect(await lab.strategies.remove(7, 3)).toBeNull();
  const conflict = await lab.strategies.action(7, "submit", 3).catch((x) => x);
  await lab.strategies.action(7, "reject", 3, "Needs a stop").catch(() => null);
  expect(JSON.parse(String(f.calls[0].init?.body))).toMatchObject({ market: "crypto" });
  expect(f.calls[1].init?.method).toBe("PUT");
  expect(JSON.parse(String(f.calls[1].init?.body)).version).toBe(3);
  expect([conflict.kind, conflict.status]).toEqual(["auth", 409]);
  expect(JSON.parse(String(f.calls[4].init?.body))).toEqual({ version: 3, note: "Needs a stop" });
  expect(f.calls[0].init?.headers).toMatchObject({ "Content-Type": "application/json" });
});
```

- [ ] **Step 2: Run it to verify it fails** — `npx vitest run tests/lab-client.test.ts` → FAIL (module not found).

- [ ] **Step 3: Implement `src/features/lab/client.ts`**

```ts
import { ApiError } from "../../services/api";
import type {
  Catalogue, EngineStatus, FidelityResult, InstrumentsResponse, LabErrorFields, LabStrategy, OptimizeResult,
  PublishedStrategy, RunResult, SavedMarket, SeriesResponse, Spec, StrategyDraft, StrategyStatus, TemplateResponse,
  TemplatesResponse, TickerRow,
} from "./types";

export class LabError extends ApiError implements LabErrorFields {
  constructor(
    message: string, status: number,
    readonly kind: LabErrorFields["kind"],
    readonly engineStatus?: EngineStatus, readonly label?: string,
    readonly provisionalNote?: string, readonly retryAfter?: number,
  ) {
    super(message, status);
    this.name = "LabError";
  }
}

export const engineBase = (m: SavedMarket) => (m === "in" ? "/api/in" : m === "us" ? "/api/us" : "/api/gl");

/** Same rules as apiJson (same-origin cookie, /api/ only) plus engine error fields, Retry-After and 204. */
export async function labRequest<T>(path: string, init: RequestInit = {}): Promise<T> {
  if (!path.startsWith("/api/")) throw new Error("Only same-origin application API requests are supported.");
  const response = await fetch(path, { ...init, credentials: "same-origin", headers: { Accept: "application/json", ...init.headers } });
  if (response.status === 204) return null as T;
  let body: Record<string, unknown> | null = null;
  try { body = (await response.json()) as Record<string, unknown>; } catch { /* non-JSON: keep the status */ }
  if (!response.ok) {
    const engineStatus = typeof body?.status === "string" ? (body.status as EngineStatus) : undefined;
    const header = Number(response.headers.get("Retry-After"));
    const retryAfter = header > 0 ? header : response.status === 429 ? 5 : undefined;
    throw new LabError(
      typeof body?.error === "string" ? body.error : `Request failed (${response.status}).`, response.status,
      path.startsWith("/api/app/") ? "auth" : engineStatus ? "engine" : "gateway",
      engineStatus, typeof body?.label === "string" ? body.label : undefined,
      typeof body?.provisional_note === "string" ? body.provisional_note : undefined, retryAfter,
    );
  }
  if (body === null) throw new LabError("The server returned an unreadable response.", response.status, "gateway");
  return body as T;
}

// The gateway shares 4 read slots; never queue more than that from one page.
let active = 0;
const waiting: (() => void)[] = [];
async function slot<T>(task: () => Promise<T>): Promise<T> {
  while (active >= 4) await new Promise<void>((resolve) => waiting.push(resolve));
  active++;
  try { return await task(); } finally { active--; waiting.shift()?.(); }
}

type QueryValue = string | number | null | undefined;
const qs = (params: Record<string, QueryValue>) =>
  new URLSearchParams(Object.entries(params).filter(([, v]) => v !== undefined && v !== null && v !== "").map(([k, v]) => [k, String(v)])).toString();
const engine = <T>(m: SavedMarket, path: string, params: Record<string, QueryValue> = {}) => {
  const query = qs(params);
  return slot(() => labRequest<T>(`${engineBase(m)}/${path}${query ? `?${query}` : ""}`));
};

const cache = new Map<string, Promise<unknown>>();
function cached<T>(key: string, load: () => Promise<T>): Promise<T> {
  if (!cache.has(key)) cache.set(key, load().catch((error) => { cache.delete(key); throw error; }));
  return cache.get(key) as Promise<T>;
}
export const clearLabCache = () => cache.clear();

export interface RunQuery { ticker: string; from?: string; to?: string; cost?: number; slippage?: number }

export const catalogue = (m: SavedMarket) => cached(`cat:${engineBase(m)}`, () => engine<Catalogue>(m, "lab/catalogue"));
export function series(m: SavedMarket, q: { id: string; ticker: string; tf?: string; p?: Record<string, number> }) {
  const params: Record<string, QueryValue> = { id: q.id, ticker: q.ticker, tf: q.tf };
  for (const [k, v] of Object.entries(q.p ?? {})) params[`p.${k}`] = v;
  return engine<SeriesResponse>(m, "lab/series", params);
}
export const run = (m: SavedMarket, spec: Spec, q: RunQuery) => engine<RunResult>(m, "lab/run", { spec: JSON.stringify(spec), ...q });
/** slim=1 always: the page reads `picked`, never `rows` (full rows can reach the 64 MB gateway cap). Not slot-limited: it has its own server slot. */
export const optimize = (m: SavedMarket, spec: Spec, q: RunQuery & { split?: string | null }) =>
  labRequest<OptimizeResult>(`${engineBase(m)}/lab/optimize?${qs({ spec: JSON.stringify(spec), ...q, slim: "1" })}`);
export const tickers = (m: "in" | "us") => cached(`tickers:${m}`, () => engine<TickerRow[]>(m, "tickers"));
export const labInstruments = () => cached("gl:instruments", () => engine<InstrumentsResponse>("gl", "lab/instruments"));
export const templates = (m: SavedMarket) => cached(`templates:${engineBase(m)}`, () => engine<TemplatesResponse>(m, "lab/templates"));
export const template = (m: SavedMarket, method: string) => engine<TemplateResponse>(m, "lab/template", { method });
export const fidelity = (m: SavedMarket, method: string, ticker: string, from?: string, to?: string) =>
  engine<FidelityResult>(m, "lab/fidelity", { method, ticker, from, to });

const BASE = "/api/app/lab/strategies";
const send = (method: string, body?: unknown): RequestInit =>
  body === undefined ? { method } : { method, headers: { "Content-Type": "application/json" }, body: JSON.stringify(body) };
export const strategies = {
  list: (status?: StrategyStatus) => labRequest<LabStrategy[]>(BASE + (status ? `?status=${status}` : "")),
  published: () => labRequest<PublishedStrategy[]>(`${BASE}/published`),
  review: () => labRequest<LabStrategy[]>(`${BASE}/review`),
  get: (id: number) => labRequest<LabStrategy>(`${BASE}/${id}`),
  create: (draft: StrategyDraft) => labRequest<LabStrategy>(BASE, send("POST", draft)),
  update: (id: number, version: number, draft: StrategyDraft) => labRequest<LabStrategy>(`${BASE}/${id}`, send("PUT", { ...draft, version })),
  remove: (id: number, version: number) => labRequest<null>(`${BASE}/${id}?version=${version}`, send("DELETE")),
  duplicate: (id: number) => labRequest<LabStrategy>(`${BASE}/${id}/duplicate`, send("POST")),
  action: (id: number, action: "submit" | "withdraw" | "publish" | "reject", version: number, note?: string) =>
    labRequest<LabStrategy>(`${BASE}/${id}/${action}`, send("POST", note === undefined ? { version } : { version, note })),
};
export type StrategiesApi = typeof strategies;
```

- [ ] **Step 4: Run** — `npx vitest run tests/lab-client.test.ts` → PASS; `npx tsc --noEmit 2>&1 | grep -E 'lab/client|lab-client'` → no output.

- [ ] **Step 5: Commit**

```bash
cd /home/aff34/.cache/strategy-vault-development; F=Context/guru/frontend/react
P="$F/src/features/lab/client.ts $F/tests/lab-client.test.ts"
git add -- $P && git commit -m "feat(lab-ui): lab API client with engine errors, Retry-After and 204" -- $P
```

---

### Task 3: UI primitives and formatting (`ui.tsx`, `format.ts`, `lab.css`)

**Files:**
- Create: `src/features/lab/ui.tsx`, `src/features/lab/format.ts`, `src/features/lab/lab.css`
- Test: `tests/lab-ui.test.tsx`

**Interfaces:**
- Consumes: `LabErrorFields`, `SavedMarket` (Task 1), `PROVISIONAL_NOTE` (Task 1).
- Produces: `pct(v, digits=1)`, `num(v, digits=2)`, `int(v)`, `DASH`; `LabErrorBoundary`, `ErrorNotice({error, market, instrumentLabel?, onRetry?})`, `Loading({text})`, `Segmented<T>({label, value, options, onChange})`, `ServerLabel({text})`, `ProvisionalStrip({note?, instrumentLabel?})`, `MetricTiles({items})`, `isProvisional(m)`.

- [ ] **Step 1: Write the failing test** — `tests/lab-ui.test.tsx`

```tsx
import { afterEach, expect, it, vi } from "vitest";
import { act } from "react";
import { ErrorNotice, LabErrorBoundary, MetricTiles, ProvisionalStrip, Segmented, ServerLabel, isProvisional } from "../src/features/lab/ui";
import { int, num, pct } from "../src/features/lab/format";
import { PROVISIONAL_NOTE } from "../src/features/lab/labels";
import { mount, unmount } from "./fixtures/lab";

afterEach(async () => { await unmount(); vi.useRealTimers(); });
const err = (fields: Record<string, unknown>) => Object.assign(new Error(String(fields.message)), fields);

it("formats numbers with a dash for missing values", () => {
  expect([pct(0.1234), pct(null), pct(Number.NaN), num(1234.5), int(12.6)]).toEqual(["12.3%", "—", "—", "1,234.50", "13"]);
});

it("shows engine errors as given with a status heading and no provisional note on India", async () => {
  const host = await mount(<ErrorNotice market="in" error={err({ message: "NIFTY 50: window starts before 2001", status: 400, kind: "engine", engineStatus: "needs_data" })} />);
  expect(host.querySelector("[data-lab-error]")?.getAttribute("role")).toBe("alert");
  expect(host.textContent).toContain("Data needed");
  expect(host.textContent).toContain("NIFTY 50: window starts before 2001");
  expect(host.querySelector("[data-lab-provisional]")).toBeNull();
});

it("labels gl errors as provisional even when a gateway error has no flag", async () => {
  const host = await mount(<ErrorNotice market="crypto" instrumentLabel="Provisional · TradingView export CRYPTO:BTCUSD, daily"
    error={err({ message: "The server is busy. Please retry shortly.", status: 429, kind: "gateway" })} />);
  expect(host.querySelector("[data-lab-provisional]")?.textContent).toContain(PROVISIONAL_NOTE);
  expect(host.textContent).toContain("CRYPTO:BTCUSD, daily");
  expect(isProvisional("gl") && isProvisional("crypto") && !isProvisional("us")).toBe(true);
});

it("429 shows a countdown, then a working retry", async () => {
  vi.useFakeTimers();
  const onRetry = vi.fn();
  const host = await mount(<ErrorNotice market="in" onRetry={onRetry} error={err({ message: "An optimization is already running.", status: 429, kind: "gateway", retryAfter: 2 })} />);
  const button = () => host.querySelector("button")!;
  expect(button().disabled).toBe(true);
  expect(button().textContent).toBe("Try again in 2 s");
  await act(async () => { vi.advanceTimersByTime(2000); });
  expect(button().disabled).toBe(false);
  await act(async () => button().click());
  expect(onRetry).toHaveBeenCalledOnce();
});

it("the boundary keeps a crashing part from blanking the page", async () => {
  const Boom = () => { throw new Error("bad series"); };
  vi.spyOn(console, "error").mockImplementation(() => {});
  const host = await mount(<div><h1>Header</h1><LabErrorBoundary><Boom /></LabErrorBoundary></div>);
  expect(host.textContent).toContain("Header");
  expect(host.querySelector("[role=alert]")?.textContent).toContain("bad series");
});

it("segmented control, labels, strip and tiles", async () => {
  const onChange = vi.fn();
  const host = await mount(<>
    <Segmented label="Market" value="in" onChange={onChange}
      options={[{ value: "in", label: "India" }, { value: "gl", label: "Global", disabled: true, title: "No data" }]} />
    <ServerLabel text="User-defined indicator strategy · simulated · not a book method" />
    <ProvisionalStrip instrumentLabel="Provisional · TradingView export IG:DAX, daily" />
    <MetricTiles items={[{ label: "CAGR", value: "12.0%", note: "net" }]} />
  </>);
  const [india, global] = host.querySelectorAll<HTMLButtonElement>(".join-item");
  expect(india.className).toContain("active");
  expect(india.getAttribute("aria-pressed")).toBe("true");
  expect(global.disabled).toBe(true);
  await act(async () => india.click());
  expect(onChange).toHaveBeenCalledWith("in");
  expect(host.querySelector("[data-server-label]")?.textContent).toBe("User-defined indicator strategy · simulated · not a book method");
  expect(host.querySelector("[data-lab-provisional]")?.textContent).toContain(PROVISIONAL_NOTE);
  expect(host.querySelector(".metric b")?.textContent).toBe("12.0%");
});
```

- [ ] **Step 2: Run** — `npx vitest run tests/lab-ui.test.tsx` → FAIL (modules missing).

- [ ] **Step 3: Implement `src/features/lab/format.ts`**

```ts
export const DASH = "—";
const finite = (v: unknown): v is number => typeof v === "number" && Number.isFinite(v);
/** Ratios from the engine (0.12 → "12.0%"), the same units as the book presenters. */
export const pct = (v: unknown, digits = 1) => (finite(v) ? `${(v * 100).toFixed(digits)}%` : DASH);
export const num = (v: unknown, digits = 2) =>
  finite(v) ? v.toLocaleString("en-US", { minimumFractionDigits: digits, maximumFractionDigits: digits }) : DASH;
export const int = (v: unknown) => (finite(v) ? Math.round(v).toLocaleString("en-US") : DASH);
```

- [ ] **Step 4: Implement `src/features/lab/ui.tsx`**

```tsx
import { Component, useEffect, useState, type ReactNode } from "react";
import { PROVISIONAL_NOTE } from "./labels";
import type { LabErrorFields, SavedMarket } from "./types";
import "./lab.css";

export const isProvisional = (m: SavedMarket) => m === "gl" || m === "crypto";

export class LabErrorBoundary extends Component<{ children: ReactNode }, { error: Error | null }> {
  state = { error: null as Error | null };
  static getDerivedStateFromError(error: Error) { return { error }; }
  render() {
    if (!this.state.error) return this.props.children;
    return (
      <div className="empty" role="alert">
        <b>This part of the page could not be shown</b>
        {this.state.error.message}{" "}
        <button type="button" className="dbtn sm" onClick={() => this.setState({ error: null })}>Try again</button>
      </div>
    );
  }
}

const HEADINGS: Record<string, string> = { needs_data: "Data needed", unsupported: "Not supported yet", invalid: "Rule problem", not_found: "Unknown request" };

export function ErrorNotice({ error, market, instrumentLabel, onRetry }: {
  error: unknown; market: SavedMarket; instrumentLabel?: string; onRetry?: () => void;
}) {
  const e = (error ?? {}) as Partial<LabErrorFields>;
  const message = error instanceof Error ? error.message : String(error);
  // A deadline plus a ticking interval stays correct when several seconds pass in one batch (and under fake timers).
  const deadline = () => (e.retryAfter ? Date.now() + e.retryAfter * 1000 : 0);
  const [until, setUntil] = useState(deadline);
  const [, setTick] = useState(0);
  useEffect(() => setUntil(deadline()), [error]); // eslint-disable-line react-hooks/exhaustive-deps
  useEffect(() => {
    if (!until) return;
    const id = setInterval(() => { setTick((t) => t + 1); if (Date.now() >= until) clearInterval(id); }, 250);
    return () => clearInterval(id);
  }, [until]);
  const wait = until ? Math.max(0, Math.ceil((until - Date.now()) / 1000)) : 0;
  const heading = (e.engineStatus && HEADINGS[e.engineStatus]) || (e.kind === "auth" ? "Not saved" : "Request refused");
  return (
    <div className="empty lab-error" role="alert" data-lab-error>
      <b>{heading}</b>
      <span>{message}</span>
      {isProvisional(market) && <ProvisionalStrip note={e.provisionalNote} instrumentLabel={instrumentLabel} />}
      {onRetry && e.retryAfter !== undefined && (
        <button type="button" className="dbtn sm" disabled={wait > 0} onClick={onRetry}>
          {wait > 0 ? `Try again in ${wait} s` : "Try again"}
        </button>
      )}
    </div>
  );
}

export const Loading = ({ text }: { text: string }) => <div className="loading" role="status">{text}</div>;

export interface SegmentOption<T extends string> { value: T; label: string; disabled?: boolean; title?: string }
export function Segmented<T extends string>({ label, value, options, onChange }: {
  label: string; value: T; options: SegmentOption<T>[]; onChange(value: T): void;
}) {
  return (
    <div className="join sm" role="group" aria-label={label}>
      {options.map((o) => (
        <button type="button" key={o.value} className={`join-item${o.value === value ? " active" : ""}`}
          aria-pressed={o.value === value} disabled={o.disabled} title={o.title} onClick={() => onChange(o.value)}>
          {o.label}
        </button>
      ))}
    </div>
  );
}

export const ServerLabel = ({ text }: { text: string }) => <p className="lab-label" data-server-label>{text}</p>;

export const ProvisionalStrip = ({ note, instrumentLabel }: { note?: string; instrumentLabel?: string }) => (
  <div className="rnote lab-provisional" role="note" data-lab-provisional>
    {note || PROVISIONAL_NOTE}{instrumentLabel ? ` · ${instrumentLabel}` : ""}
  </div>
);

export const MetricTiles = ({ items }: { items: { label: string; value: string; note?: string }[] }) => (
  <div className="metrics">
    {items.map((item) => (
      <div className="metric" key={item.label}><span>{item.label}</span><b>{item.value}</b>{item.note && <i>{item.note}</i>}</div>
    ))}
  </div>
);
```

- [ ] **Step 5: Write `src/features/lab/lab.css`** (shared lab layout; tokens only)

```css
.lab-page { display: grid; gap: 18px; }
.lab-grid { display: grid; grid-template-columns: 300px minmax(0, 1fr); gap: 18px; align-items: start; }
.lab-main { display: grid; gap: 18px; min-width: 0; }
.lab-label { margin: 6px 0 0; font: 500 12.5px var(--font); color: var(--ink2); }
.lab-provisional { border-left-color: var(--red); background: var(--card); color: var(--ink); }
.lab-error { display: grid; gap: 8px; justify-items: center; padding: 24px; }
.lab-error .lab-provisional { justify-self: stretch; text-align: left; }
.lab-row { display: flex; flex-wrap: wrap; gap: 10px; align-items: center; }
.lab-kept code { font: 400 11.5px "IBM Plex Mono", monospace; color: var(--ink2); background: var(--panel); padding: 2px 4px; }
@media (max-width: 900px) { .lab-grid { grid-template-columns: minmax(0, 1fr); } }
@media (max-width: 600px) { .lab-page .box { padding-left: 12px; padding-right: 12px; } .lab-row { gap: 6px; } }
```

- [ ] **Step 6: Run** — `npx vitest run tests/lab-ui.test.tsx` → PASS; typecheck own files clean.

- [ ] **Step 7: Commit**

```bash
cd /home/aff34/.cache/strategy-vault-development; F=Context/guru/frontend/react
P="$F/src/features/lab/ui.tsx $F/src/features/lab/format.ts $F/src/features/lab/lab.css $F/tests/lab-ui.test.tsx"
git add -- $P && git commit -m "feat(lab-ui): lab notices, error boundary, segmented control and formatting" -- $P
```

---

### Task 4: Rule spec model (`spec-model.ts`)

**Files:**
- Create: `src/features/lab/spec-model.ts`
- Test: `tests/lab-spec-model.test.ts`

**Interfaces:**
- Consumes: types (Task 1), `COPY.tooLong` (Task 1).
- Produces (exact names; Tasks 9–15 use them):
  - `type Side = "entry" | "exit"`; `SPECIAL_OPERANDS: {s: string; label: string; sides: Side[]}[]`; `PRICE_ORDERS: PriceOrder[]` with `PriceOrder = {key: "limit" | "stop_loss" | "target" | "trailing"; side: Side; label: string; pct: [number, number]; zeroOk: boolean}`; `OPERATOR_LABELS: Record<string, string>`.
  - `defaultSpec(): Spec`; `isValue(o)`, `isSeries(o)`; `conds(spec, side): Cond[]`; `setCond(spec, side, i, cond)`, `addCond(spec, side, cond)`, `removeCond(spec, side, i)`; `orderOf(spec, key): Dist | undefined`; `setOrder(spec, key, dist | undefined)`; `setExecution(spec, patch)`; `setDirection(spec, dir)`.
  - `getAt(obj, path)`, `setAt(obj, path, value)`.
  - `optimizablePaths(spec, cat): OptimizableSetting[]` with `OptimizableSetting = {path; label; current; min; max; step; integer}`; `valuesCount(dim)`, `comboCount(dims)`, `withOptimize(spec, dims)`.
  - `specProblems(spec, cat): string[]`; `specBytes(spec)`; `runQuery(spec, fees, slippage): {cost?: number; slippage?: number}`; `splitDate(dates, heldBackPct)`; `friendlyReason(reason)`; `canonicalJson(value)`; `operandLabel(o, cat)`; `conditionCount(spec)`.

- [ ] **Step 1: Write the failing test** — `tests/lab-spec-model.test.ts`

```ts
import { expect, it } from "vitest";
import * as m from "../src/features/lab/spec-model";
import type { Spec } from "../src/features/lab/types";
import { catalogueFixture as cat, templateFixture } from "./fixtures/lab";

const variant = (): Spec => (templateFixture.status === "available" ? structuredClone(templateFixture.template.spec) : m.defaultSpec());

it("default spec is close > EMA(50) entry and a 5-bar time exit", () => {
  const s = m.defaultSpec();
  expect(s.entry.all[0]).toEqual({ lhs: { s: "close" }, op: ">", rhs: { s: "ema", p: { length: 50 } } });
  expect(s.exit.any[0]).toEqual({ lhs: { s: "bars_held" }, op: ">=", rhs: { value: 5 } });
  expect(m.specProblems(s, cat)).toEqual([]);
});

it("edits keep origin and unknown keys and never mutate the input", () => {
  const s = { ...variant(), future_rule: { q: 1 } } as Spec;
  const before = JSON.stringify(s);
  let t = m.setCond(s, "entry", 0, { lhs: { s: "rsi", p: { length: 3 } }, op: "<", rhs: { value: 10 }, tag: "x" });
  t = m.addCond(t, "exit", { lhs: { s: "bars_held" }, op: ">=", rhs: { value: 3 } });
  t = m.setOrder(t, "stop_loss", { atr: 2, atr_length: 14 });
  t = m.setDirection(t, "short");
  t = m.setAt(t, "entry.all[0].rhs.value", 12);
  expect(JSON.stringify(s)).toBe(before);
  expect(t.origin).toEqual(s.origin);
  expect(t.future_rule).toEqual({ q: 1 });
  expect(t.entry.all[0]).toMatchObject({ tag: "x", rhs: { value: 12 } });
  expect(JSON.parse(JSON.stringify(t))).toEqual(t);
  expect(m.setDirection(t, "long").direction).toBeUndefined();
  expect(m.removeCond(t, "exit", 1).exit.any).toHaveLength(1);
});

it("a limit entry removes execution.fill; removing an order deletes the key", () => {
  const s = m.setExecution(m.defaultSpec(), { fill: "close", exit_fill: "close" });
  const limited = m.setOrder(s, "limit", { pct: 1 });
  expect(limited.entry.limit).toEqual({ pct: 1 });
  expect(limited.execution).toEqual({ exit_fill: "close" });
  expect("limit" in m.setOrder(limited, "limit", undefined).entry).toBe(false);
  expect(m.setExecution(m.defaultSpec(), { fill: undefined }).execution).toBeUndefined();
});

it("getAt/setAt follow the engine's optimize path grammar", () => {
  const s = m.defaultSpec();
  expect(m.getAt(s, "entry.all[0].rhs.p.length")).toBe(50);
  expect(m.getAt(m.setAt(s, "exit.stop_loss.pct", 3), "exit.stop_loss.pct")).toBe(3);
  expect(() => m.setAt(s, "entry.all[x]", 1)).toThrow();
});

it("lists optimizable settings for params, numbers and price orders", () => {
  const s = m.setOrder(m.defaultSpec(), "target", { pct: 5 });
  const paths = m.optimizablePaths(s, cat);
  expect(paths.map((p) => p.path)).toEqual(["entry.all[0].rhs.p.length", "exit.any[0].rhs.value", "exit.target.pct"]);
  expect(paths[0]).toMatchObject({ label: "Entry 1 · Exponential Moving Average Length", current: 50, min: 1, max: 5000, step: 1, integer: true });
  expect(paths[2]).toMatchObject({ min: 0, max: 1000 });
});

it("counts combinations like the engine", () => {
  expect(m.valuesCount({ path: "a", from: 10, to: 30, step: 10 })).toBe(3);
  expect(m.valuesCount({ path: "a", from: 0.1, to: 0.3, step: 0.1 })).toBe(3);
  expect(m.valuesCount({ path: "a", from: 5, to: 1, step: 1 })).toBe(0);
  expect(m.comboCount([{ path: "a", from: 1, to: 3, step: 1 }, { path: "b", from: 1, to: 2, step: 1 }])).toBe(6);
  expect(m.withOptimize(m.defaultSpec(), []).optimize).toBeUndefined();
});

it("pre-checks: two numbers, empty exit without a price exit, listed indicators, bounds, condition cap", () => {
  const s = m.defaultSpec();
  expect(m.specProblems(m.setCond(s, "entry", 0, { lhs: { value: 1 }, op: ">", rhs: { value: 2 } }), cat))
    .toContain("Entry 1: a condition cannot compare two numbers; one side must be a series.");
  const noExit = m.removeCond(s, "exit", 0);
  expect(m.specProblems(noExit, cat)).toContain("exit.any needs at least one condition (or a stop_loss, target or trailing exit)");
  expect(m.specProblems(m.setOrder(noExit, "trailing", { pct: 5 }), cat)).toEqual([]);
  expect(m.specProblems(m.setCond(s, "entry", 0, { lhs: { s: "or_range" }, op: ">", rhs: { value: 1 } }), cat)[0])
    .toBe("Entry 1: Opening Range is listed only and cannot be used in rules. Needs intraday (minute) bars; not available yet.");
  expect(m.specProblems(m.setOrder(s, "stop_loss", { pct: 0 }), cat)).toContain("Stop-loss: % must be above 0 and at most 99.");
  expect(m.specProblems(m.setOrder(s, "limit", { pct: 0 }), cat)).toEqual([]);
  expect(m.specProblems(m.setOrder(s, "target", { atr: 2, atr_length: 600 }), cat)).toContain("Target: ATR length must be a whole number from 1 to 500.");
  let many = s;
  for (let i = 0; i < 11; i++) many = m.addCond(many, "entry", s.entry.all[0]);
  expect(m.specProblems(many, cat)).toContain("Use at most 12 conditions in total.");
  expect(m.specProblems(m.setCond(s, "entry", 0, { lhs: { s: "phase7_thing" }, op: ">", rhs: { value: 1 } }), cat)).toEqual([]);
});

it("rejects specs that would exceed the gateway query cap", () => {
  let s = m.defaultSpec();
  s = { ...s, note_padding: "x".repeat(7600) } as Spec;
  expect(m.specProblems(s, cat)).toContain("These rules are too long to send. Remove a condition or shorten settings.");
  const encoded = { ...m.defaultSpec(), pad: "{}".repeat(3500) } as Spec; // 7 KB raw, ~21 KB URL-encoded
  expect(m.specBytes(encoded)).toBeLessThan(7500);
  expect(m.specProblems(encoded, cat)).toContain("These rules are too long to send. Remove a condition or shorten settings.");
});

it("sends cost from the fees box only when the spec has no cost_bps", () => {
  expect(m.runQuery(m.defaultSpec(), "5", "2")).toEqual({ cost: 5, slippage: 2 });
  expect(m.runQuery(m.defaultSpec(), "", "")).toEqual({});
  expect(m.runQuery(m.setExecution(m.defaultSpec(), { cost_bps: 10 }), "5", "")).toEqual({});
});

it("split date leaves more than 20 bars each side", () => {
  const dates = Array.from({ length: 200 }, (_, i) => `d${String(i).padStart(3, "0")}`);
  expect(m.splitDate(dates, 30)).toBe("d140");
  expect(m.splitDate(dates.slice(0, 60), 30)).toBeNull();
  expect(m.splitDate(dates, 0)).toBeNull();
});

it("maps exit reasons to friendly text and keeps unknown text", () => {
  expect(["open", "exit.stop_loss", "exit.target (gap at the open)", "exit.trailing", "exit.any[1]: rsi > 70", "something new"].map(m.friendlyReason))
    .toEqual(["Still open at the end", "Stop-loss", "Target (gap at the open)", "Trailing stop", "Exit rule 2: rsi > 70", "something new"]);
});

it("canonical JSON ignores key order (jsonb does not keep it)", () => {
  expect(m.canonicalJson({ b: 1, a: { d: 2, c: [3, { z: 1, y: 2 }] } })).toBe(m.canonicalJson({ a: { c: [3, { y: 2, z: 1 }], d: 2 }, b: 1 }));
});

it("labels operands for summaries", () => {
  expect(m.operandLabel({ s: "ema", p: { length: 50 }, tf: "W", offset: 1 }, cat)).toBe("Exponential Moving Average(50) · W · 1 bar ago");
  expect(m.operandLabel({ s: "macd", out: "signal" }, cat)).toBe("MACD.signal");
  expect(m.operandLabel({ value: 30 }, cat)).toBe("30");
  expect(m.operandLabel({ s: "bars_held" }, cat)).toBe("Bars held");
  expect(m.conditionCount(m.defaultSpec())).toBe(2);
});
```

- [ ] **Step 2: Run** — `npx vitest run tests/lab-spec-model.test.ts` → FAIL.

- [ ] **Step 3: Implement `src/features/lab/spec-model.ts`**

```ts
import { COPY } from "./labels";
import type { Catalogue, Cond, Dist, Operand, OptimizeDim, SeriesOperand, Spec, ValueOperand } from "./types";

export type Side = "entry" | "exit";
/** Operands that are not catalogue series. Phase 7 adds entries here (or the catalogue starts listing them). */
export const SPECIAL_OPERANDS: { s: string; label: string; sides: Side[] }[] = [{ s: "bars_held", label: "Bars held", sides: ["exit"] }];
export interface PriceOrder { key: "limit" | "stop_loss" | "target" | "trailing"; side: Side; label: string; pct: [number, number]; zeroOk: boolean }
/** Price orders the engine accepts (rules.rs bounds). Phase 7 order types are added here. */
export const PRICE_ORDERS: PriceOrder[] = [
  { key: "limit", side: "entry", label: "Limit entry", pct: [0, 50], zeroOk: true },
  { key: "stop_loss", side: "exit", label: "Stop-loss", pct: [0, 99], zeroOk: false },
  { key: "target", side: "exit", label: "Target", pct: [0, 1000], zeroOk: false },
  { key: "trailing", side: "exit", label: "Trailing stop", pct: [0, 99], zeroOk: false },
];
export const OPERATOR_LABELS: Record<string, string> = {
  crosses_above: "crosses above", crosses_below: "crosses below", ">": "is above", "<": "is below", ">=": "is at least", "<=": "is at most",
};

export const defaultSpec = (): Spec => ({
  v: 1,
  entry: { all: [{ lhs: { s: "close" }, op: ">", rhs: { s: "ema", p: { length: 50 } } }] },
  exit: { any: [{ lhs: { s: "bars_held" }, op: ">=", rhs: { value: 5 } }] },
});

export const isValue = (o: unknown): o is ValueOperand =>
  typeof o === "object" && o !== null && Object.keys(o).length === 1 && typeof (o as ValueOperand).value === "number";
export const isSeries = (o: unknown): o is SeriesOperand => typeof o === "object" && o !== null && typeof (o as SeriesOperand).s === "string";

export const conds = (spec: Spec, side: Side): Cond[] => (side === "entry" ? spec.entry.all : spec.exit.any);
const withConds = (spec: Spec, side: Side, list: Cond[]): Spec =>
  side === "entry" ? { ...spec, entry: { ...spec.entry, all: list } } : { ...spec, exit: { ...spec.exit, any: list } };
export const setCond = (spec: Spec, side: Side, i: number, cond: Cond) => withConds(spec, side, conds(spec, side).map((c, j) => (j === i ? cond : c)));
export const addCond = (spec: Spec, side: Side, cond: Cond) => withConds(spec, side, [...conds(spec, side), cond]);
export const removeCond = (spec: Spec, side: Side, i: number) => withConds(spec, side, conds(spec, side).filter((_, j) => j !== i));
export const conditionCount = (spec: Spec) => spec.entry.all.length + spec.exit.any.length;

const without = <T extends object>(obj: T, key: string): T => {
  const copy = { ...obj } as Record<string, unknown>;
  delete copy[key];
  return copy as T;
};
export const orderOf = (spec: Spec, key: PriceOrder["key"]): Dist | undefined => (key === "limit" ? spec.entry.limit : spec.exit[key]);

export function setExecution(spec: Spec, patch: Partial<NonNullable<Spec["execution"]>>): Spec {
  let execution: Record<string, unknown> = { ...(spec.execution ?? {}) };
  for (const [k, v] of Object.entries(patch)) execution = v === undefined ? without(execution, k) : { ...execution, [k]: v };
  return Object.keys(execution).length ? { ...spec, execution } : without(spec, "execution");
}

export function setOrder(spec: Spec, key: PriceOrder["key"], dist: Dist | undefined): Spec {
  if (key === "limit") {
    const entry = dist ? { ...spec.entry, limit: dist } : without(spec.entry, "limit");
    // The engine refuses execution.fill together with a limit entry.
    return dist ? setExecution({ ...spec, entry }, { fill: undefined }) : { ...spec, entry };
  }
  return { ...spec, exit: dist ? { ...spec.exit, [key]: dist } : without(spec.exit, key) };
}
export const setDirection = (spec: Spec, dir: "long" | "short"): Spec => (dir === "short" ? { ...spec, direction: "short" } : without(spec, "direction"));

function parsePath(path: string): (string | number)[] {
  return path.split(".").flatMap((part) => {
    const match = /^([^[\]]+)((?:\[\d+\])*)$/.exec(part);
    if (!match) throw new Error(`Unreadable setting path ${path}`);
    return [match[1], ...[...match[2].matchAll(/\[(\d+)\]/g)].map((x) => Number(x[1]))];
  });
}
export const getAt = (obj: unknown, path: string): unknown =>
  parsePath(path).reduce<unknown>((o, k) => (o == null ? undefined : (o as Record<string | number, unknown>)[k]), obj);
export function setAt<T>(obj: T, path: string, value: unknown): T {
  const keys = parsePath(path);
  const go = (o: unknown, i: number): unknown => {
    if (i === keys.length) return value;
    const copy: Record<string | number, unknown> = Array.isArray(o) ? [...o] : { ...((o as object) ?? {}) };
    copy[keys[i]] = go((o as Record<string | number, unknown> | undefined)?.[keys[i]], i + 1);
    return copy;
  };
  return go(obj, 0) as T;
}

export interface OptimizableSetting { path: string; label: string; current: number; min: number; max: number; step: number; integer: boolean }
const orderPath = (key: PriceOrder["key"]) => (key === "limit" ? "entry.limit" : `exit.${key}`);
export function optimizablePaths(spec: Spec, cat: Catalogue): OptimizableSetting[] {
  const out: OptimizableSetting[] = [];
  for (const side of ["entry", "exit"] as const)
    conds(spec, side).forEach((cond, i) => {
      for (const slot of ["lhs", "rhs"] as const) {
        const o = cond[slot];
        const base = `${side === "entry" ? "entry.all" : "exit.any"}[${i}].${slot}`;
        const tag = `${side === "entry" ? "Entry" : "Exit"} ${i + 1}`;
        if (isValue(o)) {
          const span = Math.max(Math.abs(o.value) * 0.5, 1);
          out.push({ path: `${base}.value`, label: `${tag} · number`, current: o.value, min: o.value - span, max: o.value + span, step: 1, integer: Number.isInteger(o.value) });
        } else if (isSeries(o)) {
          const ind = cat.indicators.find((x) => x.id === o.s);
          for (const p of ind?.params ?? [])
            out.push({ path: `${base}.p.${p.key}`, label: `${tag} · ${ind!.name} ${p.label}`, current: o.p?.[p.key] ?? p.default,
              min: p.min, max: p.max, step: p.step ?? (p.integer ? 1 : (p.max - p.min) / 100), integer: p.integer });
        }
      }
    });
  for (const order of PRICE_ORDERS) {
    const d = orderOf(spec, order.key);
    if (!d) continue;
    const isPct = "pct" in d;
    out.push({ path: `${orderPath(order.key)}.${isPct ? "pct" : "atr"}`, label: `${order.label} · ${isPct ? "%" : "ATR ×"}`,
      current: isPct ? d.pct : d.atr, min: isPct ? order.pct[0] : 0, max: isPct ? order.pct[1] : 50, step: isPct ? 0.5 : 0.25, integer: false });
  }
  return out;
}
export const valuesCount = (d: OptimizeDim) => (d.step > 0 && d.to >= d.from ? Math.floor((d.to - d.from) / d.step + 1e-9) + 1 : 0);
export const comboCount = (dims: OptimizeDim[]) => dims.reduce((n, d) => n * valuesCount(d), dims.length ? 1 : 0);
export const withOptimize = (spec: Spec, dims: OptimizeDim[]): Spec => (dims.length ? { ...spec, optimize: dims } : without(spec, "optimize"));

export const specBytes = (spec: Spec) => new TextEncoder().encode(JSON.stringify(spec)).length;
// Engine cap 8192 bytes; the gateway caps the URL-encoded query at 16384, so keep margins for ticker/dates.
const MAX_BYTES = 7500, MAX_ENCODED = 15000;

function distProblems(label: string, d: Dist, [lo, hi]: [number, number], zeroOk: boolean): string[] {
  if ("pct" in d)
    return (zeroOk ? d.pct >= lo : d.pct > lo) && d.pct <= hi ? [] : [`${label}: % must be ${zeroOk ? `from ${lo}` : `above ${lo}`} and at most ${hi}.`];
  const out: string[] = [];
  if (!(d.atr > 0 && d.atr <= 50)) out.push(`${label}: ATR multiplier must be above 0 and at most 50.`);
  if (d.atr_length !== undefined && !(Number.isInteger(d.atr_length) && d.atr_length >= 1 && d.atr_length <= 500))
    out.push(`${label}: ATR length must be a whole number from 1 to 500.`);
  return out;
}

/** UX pre-checks only; the engine stays the authority (unknown series ids are left to it). */
export function specProblems(spec: Spec, cat: Catalogue): string[] {
  const out: string[] = [];
  if (!spec.entry.all.length) out.push(COPY.emptyEntry);
  const priceExit = ["stop_loss", "target", "trailing"].some((k) => spec.exit[k] !== undefined);
  if (!spec.exit.any.length && !priceExit) out.push("exit.any needs at least one condition (or a stop_loss, target or trailing exit)");
  if (conditionCount(spec) > cat.max_conditions) out.push(`Use at most ${cat.max_conditions} conditions in total.`);
  for (const side of ["entry", "exit"] as const)
    conds(spec, side).forEach((c, i) => {
      const tag = `${side === "entry" ? "Entry" : "Exit"} ${i + 1}`;
      if (isValue(c.lhs) && isValue(c.rhs)) out.push(`${tag}: a condition cannot compare two numbers; one side must be a series.`);
      for (const o of [c.lhs, c.rhs]) {
        const ind = isSeries(o) ? cat.indicators.find((x) => x.id === o.s && !x.computed) : undefined;
        if (ind) out.push(`${tag}: ${ind.name} is listed only and cannot be used in rules. ${ind.note}`.trim());
      }
    });
  for (const order of PRICE_ORDERS) {
    const d = orderOf(spec, order.key);
    if (d) out.push(...distProblems(order.label, d, order.pct, order.zeroOk));
  }
  const json = JSON.stringify(spec);
  if (specBytes(spec) > MAX_BYTES || encodeURIComponent(json).length > MAX_ENCODED) out.push(COPY.tooLong);
  return out;
}

/** The engine refuses a cost query that differs from execution.cost_bps, so only one is ever sent. */
export function runQuery(spec: Spec, fees: string, slippage: string): { cost?: number; slippage?: number } {
  const q: { cost?: number; slippage?: number } = {};
  if (spec.execution?.cost_bps === undefined && fees.trim() !== "" && Number.isFinite(Number(fees))) q.cost = Number(fees);
  if (slippage.trim() !== "" && Number.isFinite(Number(slippage))) q.slippage = Number(slippage);
  return q;
}

/** The engine needs more than 20 bars on each side of the split. */
export function splitDate(dates: string[], heldBackPct: number): string | null {
  if (heldBackPct <= 0) return null;
  const i = Math.floor(dates.length * (1 - heldBackPct / 100));
  return i > 20 && dates.length - i > 20 ? dates[i] : null;
}

export function friendlyReason(reason: string): string {
  if (reason === "open") return "Still open at the end";
  const order = /^exit\.(stop_loss|target|trailing)( \(gap at the open\))?$/.exec(reason);
  if (order) return ({ stop_loss: "Stop-loss", target: "Target", trailing: "Trailing stop" } as const)[order[1] as "stop_loss"] + (order[2] ?? "");
  const rule = /^exit\.any\[(\d+)\]: (.*)$/.exec(reason);
  return rule ? `Exit rule ${Number(rule[1]) + 1}: ${rule[2]}` : reason;
}

export const canonicalJson = (value: unknown): string =>
  JSON.stringify(value, (_key, x) =>
    x && typeof x === "object" && !Array.isArray(x) ? Object.fromEntries(Object.entries(x).sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0))) : x);

export function operandLabel(o: Operand, cat: Catalogue): string {
  if (isValue(o)) return String(o.value);
  if (!isSeries(o)) return "kept as is";
  const special = SPECIAL_OPERANDS.find((x) => x.s === o.s);
  if (special) return special.label;
  const ind = cat.indicators.find((x) => x.id === o.s);
  const params = ind && o.p ? `(${ind.params.filter((p) => o.p![p.key] !== undefined).map((p) => o.p![p.key]).join(", ")})` : "";
  return [`${ind?.name ?? o.s}${params}${o.out ? `.${o.out}` : ""}`, o.tf && o.tf !== "D" ? o.tf : "",
    o.offset ? `${o.offset} bar${o.offset === 1 ? "" : "s"} ago` : ""].filter(Boolean).join(" · ");
}
```

- [ ] **Step 4: Run** — `npx vitest run tests/lab-spec-model.test.ts` → PASS; own-file typecheck clean.

- [ ] **Step 5: Commit**

```bash
cd /home/aff34/.cache/strategy-vault-development; F=Context/guru/frontend/react
P="$F/src/features/lab/spec-model.ts $F/tests/lab-spec-model.test.ts"
git add -- $P && git commit -m "feat(lab-ui): rule spec model with paths, pre-checks and query helpers" -- $P
```

---

### Task 5: Chart kit (`charts.ts`, `bars.ts`)

**Files:**
- Create: `src/features/lab/charts.ts`, `src/features/lab/bars.ts`
- Test: `tests/lab-charts.test.tsx`

**Interfaces:**
- Consumes: `LabChartApi`, `LabChartLibrary` (`globals.d.ts`, Task 1); `SeriesResponse` (Task 1).
- Produces:
  - `themeColors(el): ChartColors` (`ink, ink3, rule, steel, panel, chart, bh, green, red, blue`); `PALETTE: (keyof ChartColors)[]`.
  - `createLabChart(el, height, options?, library?) → {chart, colors, dispose()} | null`.
  - `useThemeKey(): string` (changes when `<html data-theme>` changes; use as an effect dependency to remount charts).
  - `Ohlc = {d: string[]; o: number[]; h: number[]; l: number[]; c: number[]}`; `ohlcFromSeries({open, high, low, close}: Record<string, SeriesResponse>) → Ohlc`; `periodKey(date, tf)`; `aggregate(px, tf) → Ohlc` (label = last date in the period); `candles(px) → {time, open, high, low, close}[]`; `linePoints(dates, values, keep?: Set<string>) → ({time, value} | {time})[]`; `isBooleanOutput(values)`; `booleanMarkers(dates, values, text, color, keep?)`; `rangeStart(dates, range)`.

- [ ] **Step 1: Write the failing test** — `tests/lab-charts.test.tsx`

```tsx
import { afterEach, expect, it, vi } from "vitest";
import { act } from "react";
import { aggregate, booleanMarkers, candles, isBooleanOutput, linePoints, ohlcFromSeries, periodKey, rangeStart } from "../src/features/lab/bars";
import { createLabChart, useThemeKey } from "../src/features/lab/charts";
import { mount, seriesFixture, unmount } from "./fixtures/lab";

afterEach(async () => { await unmount(); vi.unstubAllGlobals(); document.documentElement.removeAttribute("data-theme"); });

const px = { d: ["2024-01-05", "2024-01-08", "2024-01-09", "2024-01-15", "2024-02-01"], o: [1, 2, 3, 4, 5], h: [2, 6, 4, 5, 9], l: [0.5, 1, 2, 3, 4], c: [1.5, 3, 3.5, 4.5, 8] };

it("aggregates Monday-start weeks, months, quarters and years, labelled by the last daily date", () => {
  expect(periodKey("2024-01-07", "W")).toBe("2024-01-01");
  expect(periodKey("2024-05-20", "Q")).toBe("2024-Q2");
  const w = aggregate(px, "W");
  expect(w.d).toEqual(["2024-01-05", "2024-01-09", "2024-01-15", "2024-02-01"]);
  expect([w.o[1], w.h[1], w.l[1], w.c[1]]).toEqual([2, 6, 1, 3.5]);
  expect(aggregate(px, "M").d).toEqual(["2024-01-15", "2024-02-01"]);
  expect(aggregate(px, "Y").d).toEqual(["2024-02-01"]);
  expect(aggregate(px, "D")).toBe(px);
});

it("builds OHLC from series responses, dropping incomplete bars", () => {
  const s = (id: string, v: (number | null)[]) => seriesFixture(id, [id], { [id]: v });
  const out = ohlcFromSeries({ open: s("open", [1, 2, null, 4, 5, 6]), high: s("high", [2, 3, 4, 5, 6, 7]), low: s("low", [0, 1, 2, 3, 4, 5]), close: s("close", [1, 2, 3, 4, 5, 6]) });
  expect(out.d).toHaveLength(5);
  expect(candles(out)[0]).toEqual({ time: "2024-01-05", open: 1, high: 2, low: 0, close: 1 });
});

it("draws gaps for nulls, samples period ends, and turns 0/1 outputs into markers", () => {
  expect(linePoints(["a", "b", "c"], [1, null, 3])).toEqual([{ time: "a", value: 1 }, { time: "b" }, { time: "c", value: 3 }]);
  expect(linePoints(["a", "b", "c"], [1, 2, 3], new Set(["c"]))).toEqual([{ time: "c", value: 3 }]);
  expect(isBooleanOutput([0, 1, null, 0])).toBe(true);
  expect(isBooleanOutput([0, 1, 2])).toBe(false);
  expect(booleanMarkers(["a", "b", "c"], [0, 1, 1], "Doji", "#f00")).toEqual([
    { time: "b", position: "aboveBar", shape: "circle", color: "#f00", text: "Doji" },
    { time: "c", position: "aboveBar", shape: "circle", color: "#f00", text: "Doji" }]);
  expect(rangeStart(["2020-01-01", "2023-06-01", "2024-06-01", "2025-06-01"], "1Y")).toBe("2024-06-01");
  expect(rangeStart(["2020-01-01"], "All")).toBeNull();
});

it("creates a themed chart and disposes it; no library means no chart", () => {
  const remove = vi.fn(), disconnect = vi.fn(), observe = vi.fn();
  vi.stubGlobal("ResizeObserver", class { observe = observe; disconnect = disconnect; });
  const createChart = vi.fn(() => ({ remove, resize: vi.fn() }) as never);
  const el = document.createElement("div");
  document.body.append(el);
  el.style.setProperty("--panel", "#123456"); // jsdom does not inherit custom properties; set them on the host
  const made = createLabChart(el, 300, {}, { createChart });
  expect((createChart.mock.calls[0] as unknown[])[1]).toMatchObject({ height: 300, layout: { background: { color: "#123456" } } });
  made!.dispose();
  expect(remove).toHaveBeenCalled();
  expect(disconnect).toHaveBeenCalled();
  expect(createLabChart(el, 300, {}, undefined)).toBeNull();
  el.remove();
});

it("useThemeKey changes when the theme attribute changes", async () => {
  const Probe = () => <span>{useThemeKey() || "default"}</span>;
  const host = await mount(<Probe />);
  expect(host.textContent).toBe("default");
  await act(async () => { document.documentElement.setAttribute("data-theme", "midnight"); await new Promise((r) => setTimeout(r, 0)); });
  expect(host.textContent).toBe("midnight");
});
```

- [ ] **Step 2: Run** — `npx vitest run tests/lab-charts.test.tsx` → FAIL.

- [ ] **Step 3: Implement `src/features/lab/bars.ts`**

```ts
import type { ChartRange, SeriesResponse } from "./types";

export interface Ohlc { d: string[]; o: number[]; h: number[]; l: number[]; c: number[] }

export function ohlcFromSeries(parts: Record<"open" | "high" | "low" | "close", SeriesResponse>): Ohlc {
  const v = (k: keyof typeof parts) => parts[k].series[parts[k].outputs[0]] ?? [];
  const [o, h, l, c] = [v("open"), v("high"), v("low"), v("close")];
  const out: Ohlc = { d: [], o: [], h: [], l: [], c: [] };
  parts.close.dates.forEach((date, i) => {
    if ([o[i], h[i], l[i], c[i]].some((x) => typeof x !== "number")) return;
    out.d.push(date); out.o.push(o[i]!); out.h.push(h[i]!); out.l.push(l[i]!); out.c.push(c[i]!);
  });
  return out;
}

/** Same keys as VaultReport.aggregate (Monday-start weeks, calendar months) plus Q and Y for Phase 7. */
export function periodKey(date: string, tf: string): string {
  if (tf === "M") return date.slice(0, 7);
  if (tf === "Y") return date.slice(0, 4);
  if (tf === "Q") return `${date.slice(0, 4)}-Q${Math.floor((Number(date.slice(5, 7)) - 1) / 3) + 1}`;
  if (tf === "W") {
    const day = new Date(`${date}T00:00:00Z`);
    day.setUTCDate(day.getUTCDate() - ((day.getUTCDay() + 6) % 7));
    return day.toISOString().slice(0, 10);
  }
  return date;
}

export function aggregate(px: Ohlc, tf: string): Ohlc {
  if (tf === "D") return px;
  const out: Ohlc = { d: [], o: [], h: [], l: [], c: [] };
  let previous = "";
  px.d.forEach((date, i) => {
    const key = periodKey(date, tf);
    if (key !== previous) {
      out.d.push(date); out.o.push(px.o[i]); out.h.push(px.h[i]); out.l.push(px.l[i]); out.c.push(px.c[i]);
      previous = key;
    } else {
      const j = out.d.length - 1;
      out.d[j] = date; out.h[j] = Math.max(out.h[j], px.h[i]); out.l[j] = Math.min(out.l[j], px.l[i]); out.c[j] = px.c[i];
    }
  });
  return out;
}

export const candles = (px: Ohlc) => px.d.map((time, i) => ({ time, open: px.o[i], high: px.h[i], low: px.l[i], close: px.c[i] }));

/** Whitespace points keep gaps (sparse outputs, warm-up) and keep logical ranges aligned. */
export const linePoints = (dates: string[], values: (number | null)[], keep?: Set<string>) =>
  dates.flatMap((time, i) => (keep && !keep.has(time) ? [] : [typeof values[i] === "number" ? { time, value: values[i]! } : { time }]));

export const isBooleanOutput = (values: (number | null)[]) => values.some((v) => v === 1) && values.every((v) => v === null || v === 0 || v === 1);

export const booleanMarkers = (dates: string[], values: (number | null)[], text: string, color: string, keep?: Set<string>) =>
  dates.flatMap((time, i) => (values[i] === 1 && (!keep || keep.has(time)) ? [{ time, position: "aboveBar", shape: "circle", color, text }] : []));

export function rangeStart(dates: string[], range: ChartRange): string | null {
  if (range === "All" || !dates.length) return null;
  const last = new Date(`${dates[dates.length - 1]}T00:00:00Z`);
  last.setUTCFullYear(last.getUTCFullYear() - (range === "1Y" ? 1 : 3));
  const from = last.toISOString().slice(0, 10);
  return dates.find((d) => d >= from) ?? null;
}
```

- [ ] **Step 4: Implement `src/features/lab/charts.ts`**

```ts
import { useSyncExternalStore } from "react";
import type { LabChartApi, LabChartLibrary } from "./globals";

export interface ChartColors { ink: string; ink3: string; rule: string; steel: string; panel: string; chart: string; bh: string; green: string; red: string; blue: string }
export const PALETTE: (keyof ChartColors)[] = ["blue", "bh", "green", "red", "chart", "ink3"];

export function themeColors(el: HTMLElement): ChartColors {
  const css = getComputedStyle(el);
  const c = (name: string, fallback: string) => css.getPropertyValue(name).trim() || fallback;
  return {
    ink: c("--ink", "#1d1f23"), ink3: c("--ink3", "#8b8d93"), rule: c("--rule", "#e3e3e4"), steel: c("--steel", "#b9b9ba"),
    panel: c("--panel", "#f3f3f3"), chart: c("--chart", "#2f5a8f"), bh: c("--bh", "#8b738f"), green: c("--green", "#2e7d4f"),
    red: c("--red", "#c43c3c"), blue: c("--blue", "#2f5a8f"),
  };
}

/** Lab pages bypass VaultRuntime.cleanup, so every chart owns its resize observer and removal. */
export function createLabChart(el: HTMLElement, height: number, options: Record<string, unknown> = {}, library: LabChartLibrary | undefined = window.LightweightCharts) {
  if (!library) return null;
  const colors = themeColors(el);
  const chart: LabChartApi = library.createChart(el, {
    width: el.clientWidth || 600, height,
    layout: { background: { color: colors.panel }, textColor: colors.ink, fontFamily: "IBM Plex Mono, monospace" },
    grid: { vertLines: { color: colors.rule }, horzLines: { color: colors.rule } },
    rightPriceScale: { borderColor: colors.steel }, timeScale: { borderColor: colors.steel },
    ...options,
  });
  const observer = typeof ResizeObserver === "undefined" ? null : new ResizeObserver(() => chart.resize(el.clientWidth, height));
  observer?.observe(el);
  return { chart, colors, dispose() { observer?.disconnect(); chart.remove(); } };
}

function subscribe(onChange: () => void) {
  const observer = new MutationObserver(onChange);
  observer.observe(document.documentElement, { attributes: true, attributeFilter: ["data-theme"] });
  return () => observer.disconnect();
}
/** Chart colours are read at creation; effects that depend on this key remount on a theme change. */
export const useThemeKey = () => useSyncExternalStore(subscribe, () => document.documentElement.getAttribute("data-theme") ?? "");
```

- [ ] **Step 5: Run** — `npx vitest run tests/lab-charts.test.tsx` → PASS; own-file typecheck clean.

- [ ] **Step 6: Commit**

```bash
cd /home/aff34/.cache/strategy-vault-development; F=Context/guru/frontend/react
P="$F/src/features/lab/charts.ts $F/src/features/lab/bars.ts $F/tests/lab-charts.test.tsx"
git add -- $P && git commit -m "feat(lab-ui): chart kit with themed charts, period bars, gaps and markers" -- $P
```

---

### Task 6: Dev tooling — local engine proxy and headless browser check

**Files:**
- Create: `scripts/lab-dev-proxy.mjs`, `scripts/lab-browser-check.mjs`
- Test: `tests/lab-dev-proxy.test.ts`

**Interfaces:**
- Consumes: nothing from other tasks (DOM hooks table above for the browser check).
- Produces: `route(method, url, env) → {kind: "json", status, body} | {kind: "engine", url}`; CLI `node scripts/lab-dev-proxy.mjs` (port `LAB_DEV_PROXY_PORT`, default 8188); CLI `node scripts/lab-browser-check.mjs <baseUrl> <outDir>`.

- [ ] **Step 1: Write the failing test** — `tests/lab-dev-proxy.test.ts`

```ts
import { expect, it } from "vitest";
import { route } from "../scripts/lab-dev-proxy.mjs"; // allowJs resolves it; checkJs is off

it("maps market engine paths, stubs the account and refuses everything else", () => {
  const env = { LAB_ENGINE_IN: "http://127.0.0.1:18090" };
  expect(route("GET", "/api/in/lab/run?spec=%7B%7D&ticker=NIFTY+50", env)).toEqual({ kind: "engine", url: "http://127.0.0.1:18090/api/lab/run?spec=%7B%7D&ticker=NIFTY+50" });
  expect(route("GET", "/api/app/me", { LAB_DEV_ROLE: "viewer" })).toMatchObject({ kind: "json", status: 200, body: { user: { role: "viewer" } } });
  expect(route("GET", "/api/app/me", {})).toMatchObject({ body: { user: { role: "researcher" } } });
  expect(route("PUT", "/api/app/settings", {})).toMatchObject({ status: 200 });
  expect(route("GET", "/api/app/lab/strategies", {})).toMatchObject({ status: 503 });
  expect(route("POST", "/api/in/lab/run", env)).toMatchObject({ status: 405 });
  expect(route("GET", "/api/gl/tickers", { LAB_ENGINE_GL: "http://x" })).toMatchObject({ status: 400 });
  expect(route("GET", "/api/us/lab/catalogue", {})).toMatchObject({ status: 503 });
  expect(route("GET", "/api/xx/lab/catalogue", env)).toMatchObject({ status: 404 });
});
```

- [ ] **Step 2: Run** — `npx vitest run tests/lab-dev-proxy.test.ts` → FAIL.

- [ ] **Step 3: Implement `scripts/lab-dev-proxy.mjs`**

```js
#!/usr/bin/env node
// Dev only: stands in for the gateway so `npm run dev` can drive the lab pages against a locally started engine.
// No auth: /api/app/me is a fixed local account; saved strategies need the real auth service and answer 503.
import http from "node:http";
import { pathToFileURL } from "node:url";

const json = (status, body) => ({ kind: "json", status, body });

export function route(method, url, env = process.env) {
  const { pathname, search } = new URL(url, "http://local");
  if (pathname === "/api/app/me")
    return json(200, { user: { id: "dev", name: "Local dev", email: "dev@localhost", role: env.LAB_DEV_ROLE || "researcher" }, settings: { theme: "aqua" } });
  if (pathname === "/api/app/settings") return json(200, {});
  if (pathname.startsWith("/api/app/")) return json(503, { error: "Saved strategies need the auth service; this local dev proxy has none." });
  const match = /^\/api\/(in|us|gl)\/(.+)$/.exec(pathname);
  if (!match) return json(404, { error: "Unknown endpoint." });
  const [, market, path] = match;
  if (method !== "GET") return json(405, { error: "GET required." });
  if (market === "gl" && !path.startsWith("lab/")) return json(400, { error: "Invalid market or query." });
  const name = `LAB_ENGINE_${market.toUpperCase()}`;
  const base = env[name] || (market === "in" ? "http://127.0.0.1:18090" : "");
  if (!base) return json(503, { error: `No local ${market} engine is configured (${name}).` });
  return { kind: "engine", url: `${base}/api/${path}${search}` };
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const port = Number(process.env.LAB_DEV_PROXY_PORT || 8188);
  http.createServer(async (req, res) => {
    const target = route(req.method || "GET", req.url || "/");
    if (target.kind === "json") {
      res.writeHead(target.status, { "Content-Type": "application/json" });
      return res.end(JSON.stringify(target.body));
    }
    try {
      const upstream = await fetch(target.url);
      const headers = { "Content-Type": upstream.headers.get("content-type") || "application/json" };
      const retry = upstream.headers.get("retry-after");
      if (retry) headers["Retry-After"] = retry;
      res.writeHead(upstream.status, headers);
      res.end(Buffer.from(await upstream.arrayBuffer()));
    } catch {
      res.writeHead(502, { "Content-Type": "application/json" });
      res.end(JSON.stringify({ error: "The requested service did not respond. Please try again." }));
    }
  }).listen(port, "127.0.0.1", () => console.log(`lab dev proxy on http://127.0.0.1:${port}`));
}
```

- [ ] **Step 4: Implement `scripts/lab-browser-check.mjs`** (not unit-tested; exercised in Task 16)

```js
#!/usr/bin/env node
// Dev only: headless Chrome check of the lab pages over the DevTools protocol (no Playwright needed).
// Usage: node scripts/lab-browser-check.mjs http://127.0.0.1:5174 <out-dir>
import { spawn } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const [base = "http://127.0.0.1:5174", out = "."] = process.argv.slice(2);
mkdirSync(out, { recursive: true });
const profile = mkdtempSync(join(tmpdir(), "lab-chrome-"));
const chrome = spawn("/usr/bin/google-chrome", ["--headless=new", "--disable-gpu", "--disable-dev-shm-usage", "--no-first-run",
  `--user-data-dir=${profile}`, "--remote-debugging-port=0", "about:blank"], { stdio: "ignore" });
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
let port;
for (let i = 0; i < 100 && !port; i++) {
  await sleep(100);
  const file = join(profile, "DevToolsActivePort");
  if (existsSync(file)) port = readFileSync(file, "utf8").split("\n")[0];
}
if (!port) { chrome.kill(); throw new Error("Chrome did not start"); }
const target = await (await fetch(`http://127.0.0.1:${port}/json/new?about:blank`, { method: "PUT" })).json();
const ws = new WebSocket(target.webSocketDebuggerUrl);
await new Promise((resolve) => (ws.onopen = resolve));
let id = 0;
const pending = new Map(), errors = [];
ws.onmessage = ({ data }) => {
  const m = JSON.parse(data);
  if (m.id) { pending.get(m.id)?.(m); pending.delete(m.id); }
  else if (m.method === "Runtime.exceptionThrown") errors.push(m.params.exceptionDetails.exception?.description ?? m.params.exceptionDetails.text);
};
const send = (method, params = {}) => new Promise((resolve) => { pending.set(++id, resolve); ws.send(JSON.stringify({ id, method, params })); });
const evaluate = async (expression) => (await send("Runtime.evaluate", { expression, awaitPromise: true, returnByValue: true })).result?.result?.value;
const waitFor = async (expression, ms = 60000) => { for (let t = 0; t < ms; t += 250) { if (await evaluate(expression)) return true; await sleep(250); } return false; };
const click = (selector) => evaluate(`(() => { const el = document.querySelector(${JSON.stringify(selector)}); if (el && !el.disabled) { el.click(); return true; } return false; })()`);
await send("Runtime.enable");
await send("Page.enable");

const report = [];
const views = [["desktop-steel", 1440, ""], ["desktop-aqua", 1440, "aqua"], ["desktop-midnight", 1440, "midnight"], ["mobile-aqua", 400, "aqua"]];
for (const [name, width, theme] of views) {
  const before = errors.length, checks = {};
  await send("Emulation.setDeviceMetricsOverride", { width, height: 1000, deviceScaleFactor: 1, mobile: width < 600 });
  await send("Page.navigate", { url: `${base}/#/indicators?tk=NIFTY%2050` });
  checks.library = await waitFor(`!!document.querySelector('[data-lab-library] [data-lab-item]')`);
  await evaluate(theme ? `document.documentElement.setAttribute('data-theme', ${JSON.stringify(theme)})` : `document.documentElement.removeAttribute('data-theme')`);
  checks.label = await evaluate(`[...document.querySelectorAll('[data-server-label]')].some(e => e.textContent === 'User-defined indicator strategy · simulated · not a book method')`);
  if (name === "desktop-steel") {
    checks.chip = (await click(`[data-lab-item="rsi"]`)) && (await waitFor(`!!document.querySelector('[data-lab-chip]')`));
    checks.run = (await click(`[data-lab-action="run"]`)) && (await waitFor(`!!document.querySelector('[data-lab-result], [data-lab-error]')`, 120000));
    checks.runOk = await evaluate(`!!document.querySelector('[data-lab-result]')`);
    checks.optimize = (await click(`[data-lab-dim] input[type=checkbox]`)) && (await click(`[data-lab-action="optimize"]`))
      && (await waitFor(`!!document.querySelector('[data-lab-optimize-result], [data-lab-optimize] [data-lab-error]')`, 300000));
    checks.optimizeOk = await evaluate(`!!document.querySelector('[data-lab-optimize-result]')`);
  }
  checks.noHorizontalScroll = await evaluate(`document.documentElement.scrollWidth <= innerWidth + 1`);
  checks.noVerified = await evaluate(`!/\\bverified\\b|TradingView-matched/i.test(document.body.innerText)`);
  const shot = await send("Page.captureScreenshot", { format: "png", captureBeyondViewport: true });
  writeFileSync(join(out, `lab-${name}.png`), Buffer.from(shot.result.data, "base64"));
  report.push({ name, checks, pageErrors: errors.slice(before) });
}
await send("Page.navigate", { url: `${base}/#/yt-strategies` });
report.push({ name: "yt-strategies", checks: { rendered: await waitFor(`!!document.querySelector('[data-lab-page="yt-strategies"] [data-lab-error], [data-lab-page="yt-strategies"] .lab-card, [data-lab-page="yt-strategies"] .empty')`) }, pageErrors: [] });
ws.close(); chrome.kill(); rmSync(profile, { recursive: true, force: true });
console.log(JSON.stringify(report, null, 2));
process.exit(report.every((r) => !r.pageErrors.length && Object.values(r.checks).every(Boolean)) ? 0 : 1);
```

- [ ] **Step 5: Run** — `npx vitest run tests/lab-dev-proxy.test.ts` → PASS; `node --check scripts/lab-browser-check.mjs && node --check scripts/lab-dev-proxy.mjs` → no output.

- [ ] **Step 6: Commit**

```bash
cd /home/aff34/.cache/strategy-vault-development; F=Context/guru/frontend/react
P="$F/scripts/lab-dev-proxy.mjs $F/scripts/lab-browser-check.mjs $F/tests/lab-dev-proxy.test.ts"
git add -- $P && git commit -m "test(lab-ui): dev-only engine proxy and headless browser check" -- $P
```

---

### Task 7: Library rail, reference panel and toolbar

**Files:**
- Create: `src/features/lab/catalogue.ts`, `src/features/lab/instruments.ts`, `src/features/lab/Toolbar.tsx`, `src/features/lab/LibraryRail.tsx`, `src/features/lab/IndicatorDetail.tsx`, `src/features/lab/library.css`
- Test: `tests/lab-library.test.tsx`

**Interfaces:**
- Consumes: types, `COPY` (Task 1); `Segmented`, `ProvisionalStrip`, `isProvisional` (Task 3).
- Produces:
  - `catalogue.ts`: `searchIndicators(list, query)`, `groupByCategory(list) → [string, Indicator[]][]` (catalogue order), `availability(ind, {market, hasVolume, tf}) → {ok: boolean; reason?: string}`, `MAX_PRICE_OVERLAYS = 4`, `MAX_PANES = 3`.
  - `instruments.ts`: `InstrumentOption = {ticker; label; disabled; reason?; type?; hasVolume; market?: SavedMarket; instrumentLabel?; first?; last?}`; `optionsFromTickers(rows, allowedTypes?)`; `optionsFromGl(res, allowedTypes?)`; `defaultTicker(market, options)`; `savedMarket(market, option?) → SavedMarket`.
  - `Toolbar({market, onMarket, options, ticker, onTicker, tf, timeframes, onTf, range, onRange, provisionalNote?})`.
  - `LibraryRail({catalogue, market, hasVolume, tf, onAdd(ind), onRead(ind)})` root `data-lab-library`, items `button[data-lab-item=<id>]`.
  - `IndicatorDetail({indicator, catalogue, onClose})`.

Availability rules (in this order; the first match wins):
1. `!computed` → `ind.note || "Listed only; the engine does not compute it yet."`
2. `needs.volume && !hasVolume` → `"Needs volume; this instrument has no volume in its data."`
3. `needs.breadth && (market === "gl" || tf !== "D")` → `"Breadth data exists only for India and US daily bars."`
4. any of `intraday, second_symbol, fundamentals, derivatives, onchain` → `ind.note || "Needs data Strategy Vault does not have yet."`

Instrument options: in/us rows → `label` is `ticker` for indices and `${symbol} · ${basket}` for native stocks; `type = asset_type` for native rows (confirm the vocabulary in `Context/guru/backend/src/baskets.rs` `asset_type()` read-only; it must match `allowed_types` words such as `individual_stock`, `etf`); disabled with `Not admitted for this method (needs <types joined by ", ">)` when `allowedTypes` is given and the type is known and not in it. gl rows → label `instrument.name ?? ticker`, `needs_data` rows disabled with their `reason`, `market = instrument.market`, `instrumentLabel = instrument.label`, `hasVolume = false`. `defaultTicker`: `in` → `"NIFTY 50"` when present, otherwise the first enabled option; `savedMarket("gl", option)` → `"crypto"` when `option.market === "crypto"`, else `"gl"`.

Toolbar markup: `.box.lab-toolbar` > `.lab-row` with `Segmented` Market (`India`, `US`, `Global & crypto · provisional`), a `.dfield` select for the instrument (disabled options keep their `title` = reason; label "Instrument"), `Segmented` Chart (one option per `timeframes`, labels `D → Daily`, `W → Weekly`, `M → Monthly`, others as given) and `Segmented` Show (`1Y`, `3Y`, `All`). On gl, a `ProvisionalStrip` with `provisionalNote` and the selected option's `instrumentLabel` sits under the row.

LibraryRail markup: `aside.box.lab-library[data-lab-library]` with `h2.box-h` "Library" + `<small>{n} indicators</small>`, a search `input` (placeholder `Search name, id, TradingView or AmiBroker name`, `aria-label="Search indicators"`), then per group `<details open>` with `<summary>{category} <small>{count}</small></summary>` and items `<li><button data-lab-item={id} disabled={!ok} title={reason}>name <span className="tag">{price|pane|needs data}</span></button> <button className="dbtn ghost sm" aria-label={`About ${name}`}>i</button></li>`; a disabled item shows its reason under the name in `<small>`. Below 900px the whole rail is wrapped in `<details>` with summary "Library" (closed by default on mobile via `matchMedia('(max-width: 900px)')`).

IndicatorDetail markup: `.box` with `h2` name + `<small>id {id} · {category}</small>`, Definition `<pre>`, a facts `<dl>` (Placement `On price`/`Own pane`; Needs: `Price only` or the list of true `needs` keys in plain words; Warm-up bars `first` joined by ", " with ` + confirmation` when `sparse`; Timeframes `catalogue.timeframes.join(" · ")`; Engine `Computed`/`Listed only`), Settings table (Setting, Key, Default, Min, Max, Step), Outputs list, the line `COPY.notChecked`, and "Using it in rules": `Compare any output with a number, a price (${price_sources.join(", ")}) or another indicator, on ${timeframes.join(", ")} bars.` + `COPY.wmRule`. For listed items: the server `note` and "Rules cannot reference it until that data exists."

- [ ] **Step 1: Write the failing test** — `tests/lab-library.test.tsx`

```tsx
import { afterEach, expect, it, vi } from "vitest";
import { act } from "react";
import { availability, groupByCategory, searchIndicators } from "../src/features/lab/catalogue";
import { defaultTicker, optionsFromGl, optionsFromTickers, savedMarket } from "../src/features/lab/instruments";
import { LibraryRail } from "../src/features/lab/LibraryRail";
import { IndicatorDetail } from "../src/features/lab/IndicatorDetail";
import { Toolbar } from "../src/features/lab/Toolbar";
import { catalogueFixture as cat, glInstrumentsFixture, mount, tickersFixture, unmount } from "./fixtures/lab";

afterEach(unmount);
const byId = (id: string) => cat.indicators.find((i) => i.id === id)!;

it("searches name, id, TradingView and AmiBroker names", () => {
  expect(searchIndicators(cat.indicators, "ta.rsi").map((i) => i.id)).toEqual(["rsi"]);
  expect(searchIndicators(cat.indicators, "ema").map((i) => i.id)).toEqual(["ema"]);
  expect(searchIndicators(cat.indicators, "  ").length).toBe(cat.indicators.length);
  expect(groupByCategory(cat.indicators).map(([c, l]) => [c, l.length])[1]).toEqual(["Oscillators", 3]);
});

it("availability: listed, volume, breadth and other data needs", () => {
  const ctx = { market: "in" as const, hasVolume: false, tf: "D" };
  expect(availability(byId("or_range"), ctx)).toEqual({ ok: false, reason: "Needs intraday (minute) bars; not available yet." });
  expect(availability(byId("obv"), ctx).reason).toBe("Needs volume; this instrument has no volume in its data.");
  expect(availability(byId("obv"), { ...ctx, hasVolume: true }).ok).toBe(true);
  expect(availability(byId("ad_line"), { ...ctx, tf: "W" }).reason).toBe("Breadth data exists only for India and US daily bars.");
  expect(availability(byId("ad_line"), ctx).ok).toBe(true);
});

it("instrument options per market and allowed types", () => {
  const inOpts = optionsFromTickers(tickersFixture, ["index", "etf"]);
  expect(inOpts.map((o) => [o.label, o.disabled])).toEqual([["NIFTY 50", false], ["NIFTYBANK", false], ["RELIANCE · NIFTY 50", true]]);
  expect(inOpts[2].reason).toBe("Not admitted for this method (needs index, etf)");
  const gl = optionsFromGl(glInstrumentsFixture);
  expect(gl.map((o) => [o.label, o.disabled, o.reason])).toEqual([["Bitcoin / US dollar", false, undefined], ["DAX", true, "IG:DAX is not staged on this server"]]);
  expect(defaultTicker("in", inOpts)).toBe("NIFTY 50");
  expect(defaultTicker("gl", gl)).toBe("CRYPTO:BTCUSD");
  expect([savedMarket("gl", gl[0]), savedMarket("gl", gl[1]), savedMarket("us")]).toEqual(["crypto", "gl", "us"]);
});

it("rail lists every entry, disables listed ones with their note and adds computed ones", async () => {
  const onAdd = vi.fn(), onRead = vi.fn();
  const host = await mount(<LibraryRail catalogue={cat} market="in" hasVolume={false} tf="D" onAdd={onAdd} onRead={onRead} />);
  expect(host.querySelectorAll("[data-lab-item]")).toHaveLength(cat.indicators.length);
  const listed = host.querySelector<HTMLButtonElement>('[data-lab-item="or_range"]')!;
  expect(listed.disabled).toBe(true);
  expect(listed.closest("li")!.textContent).toContain("Needs intraday (minute) bars; not available yet.");
  await act(async () => host.querySelector<HTMLButtonElement>('[data-lab-item="ema"]')!.click());
  expect(onAdd).toHaveBeenCalledWith(byId("ema"));
  const search = host.querySelector<HTMLInputElement>('input[aria-label="Search indicators"]')!;
  await act(async () => {
    Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(search, "ta.rsi");
    search.dispatchEvent(new Event("input", { bubbles: true }));
  });
  expect([...host.querySelectorAll<HTMLElement>("[data-lab-item]")].map((b) => b.dataset.labItem)).toEqual(["rsi"]);
});

it("reference panel shows facts and never claims a TradingView match", async () => {
  const host = await mount(<IndicatorDetail indicator={byId("macd")} catalogue={cat} onClose={() => {}} />);
  expect(host.textContent).toContain("Own pane");
  expect(host.textContent).toContain("25, 33, 33");
  expect(host.textContent).toContain("Not yet checked against TradingView.");
  expect(host.textContent).not.toMatch(/\bverified\b|TradingView-matched/i);
});

it("toolbar shows the provisional strip on gl and reports choices", async () => {
  const onMarket = vi.fn();
  const host = await mount(<Toolbar market="gl" onMarket={onMarket} options={optionsFromGl(glInstrumentsFixture)} ticker="CRYPTO:BTCUSD" onTicker={() => {}}
    tf="D" timeframes={["D", "W", "M", "Q"]} onTf={() => {}} range="3Y" onRange={() => {}} provisionalNote={glInstrumentsFixture.provisional_note} />);
  expect(host.querySelector("[data-lab-provisional]")?.textContent).toContain("CRYPTO:BTCUSD, daily");
  expect(host.textContent).toContain("Q");
  await act(async () => [...host.querySelectorAll<HTMLButtonElement>(".join-item")].find((b) => b.textContent === "US")!.click());
  expect(onMarket).toHaveBeenCalledWith("us");
});
```

(React-controlled inputs are driven in tests through the native value setter plus an `input` event; implement the search box as a controlled input with `onChange`.)

- [ ] **Step 2: Run** — `npx vitest run tests/lab-library.test.tsx` → FAIL.
- [ ] **Step 3: Implement `catalogue.ts` and `instruments.ts`** exactly to the rules above (pure functions; `searchIndicators` lower-cases and trims the query, an empty query returns the list unchanged; `groupByCategory` keeps first-appearance order with a `Map`).
- [ ] **Step 4: Implement `Toolbar.tsx`, `LibraryRail.tsx`, `IndicatorDetail.tsx`, `library.css`** to the markup above. CSS: `.lab-library { position: sticky; top: 12px; max-height: calc(100vh - 24px); overflow: auto; }`, `.lab-library button[disabled] { color: var(--ink3); cursor: not-allowed; }`, `.lab-library li { list-style: none; }`, `.lab-toolbar .dfield select { max-width: 100%; }`, and `@media (max-width: 900px) { .lab-library { position: static; max-height: none; } }`. Tokens only.
- [ ] **Step 5: Run** — `npx vitest run tests/lab-library.test.tsx` → PASS; own-file typecheck clean.
- [ ] **Step 6: Commit**

```bash
cd /home/aff34/.cache/strategy-vault-development; F=Context/guru/frontend/react; L=$F/src/features/lab
P="$L/catalogue.ts $L/instruments.ts $L/Toolbar.tsx $L/LibraryRail.tsx $L/IndicatorDetail.tsx $L/library.css $F/tests/lab-library.test.tsx"
git add -- $P && git commit -m "feat(lab-ui): indicator library, reference panel and market toolbar" -- $P
```

---

### Task 8: Chart workbench

**Files:**
- Create: `src/features/lab/Workbench.tsx`, `src/features/lab/workbench.css`
- Test: `tests/lab-workbench.test.tsx`

**Interfaces:**
- Consumes: types, `COPY` (Task 1); `ErrorNotice`, `ProvisionalStrip`, `isProvisional` (Task 3); `createLabChart`, `useThemeKey`, `PALETTE` (Task 5); `ohlcFromSeries`, `aggregate`, `candles`, `linePoints`, `isBooleanOutput`, `booleanMarkers`, `rangeStart` (Task 5).
- Produces: `interface Chip { key: string; id: string; params: Record<string, number> }`; `Workbench(props: WorkbenchProps)` with

```ts
export interface WorkbenchProps {
  market: SavedMarket; ticker: string; tf: string; range: ChartRange; catalogue: Catalogue; chips: Chip[];
  trades?: Trade[];
  loadSeries(q: { id: string; tf: string; p?: Record<string, number> }): Promise<SeriesResponse>;
  onChipChange(key: string, params: Record<string, number>): void;
  onChipRemove(key: string): void;
  instrumentLabel?: string;
}
```

Behaviour:
- One effect keyed on `[market, ticker, tf, range, chips (canonical JSON), trades, themeKey]`: load OHLC via four `loadSeries({id: "open"|"high"|"low"|"close", tf: "D"})` calls and each chip via `loadSeries({id, tf, p: chip.params})` (chips whose indicator is missing or `!computed` are skipped). An abort flag set in the cleanup prevents updates after unmount/rerender. Cleanup disposes every chart.
- Price chart: `createLabChart(host, 420)` (`.lwc.px` host with `data-lab-chart`), `addCandlestickSeries` with `upColor/borderUpColor/wickUpColor = colors.green`, down = `colors.red`; data `candles(aggregate(ohlc, tf))`. Period ends `keep = new Set(aggregated.d)` when `tf !== "D"`.
- For each price chip output: if `isBooleanOutput(values)` → collect `booleanMarkers(dates, values, `${name}${outputs.length > 1 ? "." + out : ""}`, color, keep)`; otherwise `addLineSeries({color, lineWidth: 2, priceLineVisible: false, lastValueVisible: false})` with `linePoints(dates, values, keep)`. Pane chips: their own `createLabChart(paneHost, 150)` under the price chart in `.lab-pane[data-lab-pane=<key>]`, same series rules. Colours cycle through `PALETTE` by chip index.
- Trades: entry markers `arrowUp` below bar (`colors.green`, text `Long`/`Short`), exit markers `arrowDown` above bar (`colors.red`); markers on dates not in `keep` move to the next period end. All markers on the candle series are sorted by time before `setMarkers`.
- `rangeStart(aggregated.d, range)` → `timeScale().setVisibleRange({from, to: last})`, else `fitContent()`.
- Chips row: each `span.chip[data-lab-chip=<key>]` with a colour dot, the indicator name, one input per param (`type=number`, `min/max/step`, `aria-label={`${name} ${p.label}`}`; integer params round), except `key === "variant"` which is a `select` with `0 → TradingView`, `1 → AmiBroker`; and `button aria-label={`Remove ${name}`}` "×". A chip whose series failed shows a compact `ErrorNotice` inside the chip; the rest of the chart still renders. Changing an input calls `onChipChange(key, {...params, [k]: value})` only for finite values inside min/max.
- Header: `.box-h` "`{ticker} · {Daily|Weekly|Monthly|tf} bars`" + `<small>{COPY.chartHint}</small>`; empty chips → `COPY.noChips` in `.cap`.
- Notes under the chart (`.cap`): `COPY.chartNote`; when `tf !== "D"`: `COPY.inProgress(tf)` and a marker `text: "in progress"` on the last candle; when trades exist: "Shaded markers are trades from the backtest below." + (`tf !== "D"`: " Trades run on daily bars and are mapped onto this timeframe."). On gl: `ProvisionalStrip` with `instrumentLabel`.
- OHLC load failure → `ErrorNotice` replaces the chart (no candles).
- Without `window.LightweightCharts` (tests that do not mock it) the component still renders chips and notes.

- [ ] **Step 1: Write the failing test** — `tests/lab-workbench.test.tsx`

```tsx
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { act } from "react";
import { Workbench, type Chip } from "../src/features/lab/Workbench";
import { catalogueFixture as cat, flush, mount, runFixture, seriesFixture, unmount } from "./fixtures/lab";

type Spy = { setData: ReturnType<typeof vi.fn>; setMarkers: ReturnType<typeof vi.fn>; applyOptions: ReturnType<typeof vi.fn> };
let charts: { remove: ReturnType<typeof vi.fn>; series: Spy[]; kinds: string[] }[] = [];
beforeEach(() => {
  charts = [];
  const make = () => {
    const c = { remove: vi.fn(), series: [] as Spy[], kinds: [] as string[] };
    charts.push(c);
    const add = (kind: string) => () => { const s = { setData: vi.fn(), setMarkers: vi.fn(), applyOptions: vi.fn() }; c.series.push(s); c.kinds.push(kind); return s; };
    return { addCandlestickSeries: add("candle"), addLineSeries: add("line"), addAreaSeries: add("area"), remove: c.remove, resize: vi.fn(),
      applyOptions: vi.fn(), timeScale: () => ({ fitContent: vi.fn(), setVisibleRange: vi.fn() }) };
  };
  window.LightweightCharts = { createChart: vi.fn(make) } as never;
});
afterEach(async () => { await unmount(); delete window.LightweightCharts; });

const ohlc = (id: string) => seriesFixture(id, [id], { [id]: [1, 2, 3, 4, 5, 6] });
const load = vi.fn(async (q: { id: string }) => {
  if (["open", "high", "low", "close"].includes(q.id)) return ohlc(q.id);
  if (q.id === "fractals") return seriesFixture("fractals", ["up", "down"], { up: [null, 3, null, null, 5, null], down: [null, null, null, null, null, null] });
  if (q.id === "doji") return seriesFixture("doji", ["doji"], { doji: [0, 1, 0, 0, 1, 0] });
  if (q.id === "rsi") throw Object.assign(new Error("rsi: needs more history"), { status: 400, kind: "engine", engineStatus: "needs_data" });
  return seriesFixture(q.id, [q.id], { [q.id]: [null, 1, 2, 3, 4, 5] });
});
const chips: Chip[] = [
  { key: "c1", id: "ema", params: { length: 20 } }, { key: "c2", id: "fractals", params: {} },
  { key: "c3", id: "doji", params: {} }, { key: "c4", id: "stoch", params: { length: 14, variant: 1 } }, { key: "c5", id: "rsi", params: { length: 14 } },
];
const props = { market: "in" as const, ticker: "NIFTY 50", tf: "D", range: "All" as const, catalogue: cat, chips, loadSeries: load, onChipChange: vi.fn(), onChipRemove: vi.fn() };

it("draws candles, overlays with gaps, boolean markers and a pane; a failing chip stays local", async () => {
  const host = await mount(<Workbench {...props} />);
  await flush();
  const price = charts[0];
  expect(price.kinds[0]).toBe("candle");
  expect(price.series[0].setData.mock.calls[0][0]).toHaveLength(6);
  const lines = price.series.slice(1).map((s) => s.setData.mock.calls[0][0]);
  expect(lines.some((points: { value?: number }[]) => points[0].value === undefined)).toBe(true);
  expect(price.series[0].setMarkers.mock.calls.at(-1)![0].filter((m: { text: string }) => m.text === "Doji")).toHaveLength(2);
  expect(charts.length).toBeGreaterThanOrEqual(2);
  expect(host.querySelector('[data-lab-chip="c5"] [data-lab-error]')?.textContent).toContain("rsi: needs more history");
  expect(host.querySelector('[data-lab-chip="c4"] select')?.textContent).toContain("AmiBroker");
});

it("edits and removes chips through callbacks", async () => {
  const onChipChange = vi.fn(), onChipRemove = vi.fn();
  const host = await mount(<Workbench {...props} onChipChange={onChipChange} onChipRemove={onChipRemove} />);
  const input = host.querySelector<HTMLInputElement>('[data-lab-chip="c1"] input')!;
  await act(async () => {
    const setter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!;
    setter.call(input, "30");
    input.dispatchEvent(new Event("input", { bubbles: true }));
  });
  expect(onChipChange).toHaveBeenCalledWith("c1", { length: 30 });
  await act(async () => host.querySelector<HTMLButtonElement>('[aria-label="Remove Exponential Moving Average"]')!.click());
  expect(onChipRemove).toHaveBeenCalledWith("c1");
});

it("weekly view marks the last bar in progress and maps trades", async () => {
  const host = await mount(<Workbench {...props} tf="W" chips={[]} trades={runFixture.trades} />);
  await flush();
  expect(host.textContent).toContain("The last weekly bar is still in progress");
  expect(host.textContent).toContain("Trades run on daily bars and are mapped onto this timeframe.");
  const markers = charts[0].series[0].setMarkers.mock.calls.at(-1)![0];
  expect(markers.some((m: { text: string }) => m.text === "in progress")).toBe(true);
  expect(markers.some((m: { text: string }) => m.text === "Short")).toBe(true);
});

it("removes charts on unmount", async () => {
  await mount(<Workbench {...props} />);
  await flush();
  await unmount();
  expect(charts.every((c) => c.remove.mock.calls.length === 1)).toBe(true);
});
```

- [ ] **Step 2: Run** — `npx vitest run tests/lab-workbench.test.tsx` → FAIL.
- [ ] **Step 3: Implement `Workbench.tsx` and `workbench.css`** to the behaviour list above. CSS: `.lab-chips { display: flex; flex-wrap: wrap; gap: 6px; }`, `.lab-chips .chip input { width: 64px; }`, `.lab-dot { width: 9px; height: 9px; border-radius: 50%; display: inline-block; }`, `.lab-pane { height: 150px; margin-top: 6px; }`; tokens only.
- [ ] **Step 4: Run** — PASS; own-file typecheck clean.
- [ ] **Step 5: Commit**

```bash
cd /home/aff34/.cache/strategy-vault-development; F=Context/guru/frontend/react; L=$F/src/features/lab
P="$L/Workbench.tsx $L/workbench.css $F/tests/lab-workbench.test.tsx"
git add -- $P && git commit -m "feat(lab-ui): chart workbench with chips, panes, gaps and markers" -- $P
```

---

### Task 9: Rule builder, operand editor and orders panel

**Files:**
- Create: `src/features/lab/RuleBuilder.tsx`, `src/features/lab/OperandEditor.tsx`, `src/features/lab/OrdersPanel.tsx`, `src/features/lab/rules.css`
- Test: `tests/lab-rule-builder.test.tsx`

**Interfaces:**
- Consumes: types, `COPY` (Task 1); `Segmented` (Task 3); from Task 4: `SPECIAL_OPERANDS`, `PRICE_ORDERS`, `OPERATOR_LABELS`, `isValue`, `isSeries`, `conds`, `setCond`, `addCond`, `removeCond`, `conditionCount`, `orderOf`, `setOrder`, `setExecution`, `setDirection`, `Side`.
- Produces:

```ts
export interface RunSettings { from: string; to: string; fees: string; slippage: string }
export function RuleBuilder(props: {
  spec: Spec; catalogue: Catalogue; onChange(spec: Spec): void;
  settings: RunSettings; onSettings(patch: Partial<RunSettings>): void;
  problems: string[]; running: boolean; onRun(): void;
}): JSX.Element;
export function OperandEditor(props: { operand: Operand; side: Side; catalogue: Catalogue; onChange(o: Operand): void; label: string }): JSX.Element;
export function OrdersPanel(props: { spec: Spec; onChange(spec: Spec): void }): JSX.Element;
```

Behaviour:
- `section.box.lab-rules`: `h2.box-h` "Strategy rules" + `<small>{COPY.rulesHint}</small>`; `OrdersPanel` (direction + execution + price orders, rows `.lab-order` starting with the order label); two `div.lab-side` blocks with `h3` `COPY.entryTitle` / `COPY.exitTitle`; each condition row `.lab-cond` = `OperandEditor(lhs)` · operator `select` with `aria-label="Operator"` (options from `catalogue.operators`, text `OPERATOR_LABELS[op] ?? op`; a spec op missing from the list is still shown as its own option) · `OperandEditor(rhs)` · `button aria-label="Remove condition"` ×. A condition with extra keys keeps them (edits go through `setCond` with `{...cond, lhs|op|rhs}`).
- `+ Add condition` per side (`COPY.addCondition`) is disabled when `conditionCount(spec) >= catalogue.max_conditions`. New entry condition: `{lhs: {s: "close"}, op: ">", rhs: {s: "ema", p: {length: 50}}}`; new exit condition: `{lhs: {s: "bars_held"}, op: ">=", rhs: {value: 5}}`.
- When any operand has `tf` other than `D`: `.cap` `COPY.wmRule`.
- Run settings row `.lab-row`: From and To (`type=date`, empty = full history), "Fees (bps / side)" — when `spec.execution?.cost_bps` is set this input edits `cost_bps` through `setExecution` instead of `settings.fees` —, "Slippage (bps / side)", and `button.dbtn.primary[data-lab-action="run"]` "Run backtest" (text "Running…" and disabled while `running`; disabled when `problems.length`). `problems` render as `ul.rnote` above the button. Exec line `.cap`: "Signal at the bar's close · fill at the " + (`limit` ? "limit price" : fill === "close" ? "same close" : "next open").
- OperandEditor: kind `select` with `Price`, `Indicator`, `Number`, plus each `SPECIAL_OPERANDS` entry whose `sides` include `side`. Unknown shape (neither `isValue` nor `isSeries`, or `s` that is neither a price source, a catalogue indicator nor a special operand) → `span.lab-kept` with `COPY.keptAsIs` and `<code>{JSON.stringify(operand)}</code>`, no inputs. Price: select over `catalogue.price_sources` (sets `{s}` keeping `tf`/`offset`, dropping `p`/`out`). Indicator: select of computed indicators grouped in `<optgroup label={category}>` (listed ones are never options); param inputs (`p[key] ?? default`, writes `p`), output select when `outputs.length > 1` (`out`), timeframe select over `catalogue.timeframes` (`D` deletes `tf`), offset number 0–1000 labelled "bars ago" (0 deletes `offset`). Number: one numeric input (`{value}`). Special: `{s}` only.
- OrdersPanel: Direction `Segmented` Long | Short (`setDirection`); "Entry fill" select Next open | Same close (hidden when a limit entry is set; `setExecution({fill})`, next open deletes the key); "Exit fill" select (`exit_fill`); for each `PRICE_ORDERS` entry a row: label, `Segmented` Off | % | ATR, then a number input for `pct`, or `atr` multiplier + `atr_length` (default 14) inputs; Off → `setOrder(key, undefined)`.

- [ ] **Step 1: Write the failing test** — `tests/lab-rule-builder.test.tsx`

```tsx
import { afterEach, expect, it, vi } from "vitest";
import { act } from "react";
import { RuleBuilder } from "../src/features/lab/RuleBuilder";
import { defaultSpec, addCond } from "../src/features/lab/spec-model";
import type { Spec } from "../src/features/lab/types";
import { catalogueFixture as cat, mount, templateFixture, unmount } from "./fixtures/lab";

afterEach(unmount);
const settings = { from: "", to: "", fees: "", slippage: "" };
const setup = async (spec: Spec, extra: Partial<Parameters<typeof RuleBuilder>[0]> = {}) => {
  const onChange = vi.fn(), onRun = vi.fn(), onSettings = vi.fn();
  const host = await mount(<RuleBuilder spec={spec} catalogue={{ ...cat, timeframes: ["D", "W", "M", "Q"] }} onChange={onChange}
    settings={settings} onSettings={onSettings} problems={[]} running={false} onRun={onRun} {...extra} />);
  return { host, onChange, onRun, onSettings, last: () => onChange.mock.calls.at(-1)![0] as Spec };
};
const choose = async (el: HTMLSelectElement, value: string) => act(async () => { el.value = value; el.dispatchEvent(new Event("change", { bubbles: true })); });

it("shows both sides, catalogue operators incl. 'is at most' and catalogue timeframes", async () => {
  const { host } = await setup(defaultSpec());
  expect(host.textContent).toContain("Entry · all of these");
  expect(host.textContent).toContain("Exit · any of these");
  expect([...host.querySelectorAll("option")].map((o) => o.textContent)).toEqual(expect.arrayContaining(["is at most", "crosses above"]));
  expect([...host.querySelectorAll("option")].some((o) => o.value === "Q")).toBe(true);
});

it("bars held only on the exit side; listed indicators are never offered", async () => {
  const { host } = await setup(defaultSpec());
  const [entry, exit] = host.querySelectorAll<HTMLElement>(".lab-side");
  expect(entry.textContent).not.toContain("Bars held");
  expect(exit.textContent).toContain("Bars held");
  expect(host.querySelector('option[value="or_range"]')).toBeNull();
});

it("editing keeps origin and unknown keys", async () => {
  const spec = { ...(templateFixture.status === "available" ? templateFixture.template.spec : defaultSpec()), future_rule: 1 } as Spec;
  const { host, last } = await setup(spec);
  await choose(host.querySelector<HTMLSelectElement>(".lab-cond select[aria-label='Operator']")!, "<=");
  expect(last().entry.all[0].op).toBe("<=");
  expect(last().origin).toEqual(spec.origin);
  expect(last().future_rule).toBe(1);
});

it("unknown operand shapes are kept and shown read-only", async () => {
  const spec = { ...defaultSpec(), entry: { all: [{ lhs: { s: "close" }, op: ">", rhs: { fancy: { k: 1 } } as never }] } } as Spec;
  const { host } = await setup(spec);
  expect(host.querySelector(".lab-kept")?.textContent).toContain('{"fancy":{"k":1}}');
});

it("add condition stops at the catalogue limit; problems block the run", async () => {
  let spec = defaultSpec();
  for (let i = 0; i < 10; i++) spec = addCond(spec, "entry", spec.entry.all[0]);
  const { host } = await setup(spec, { problems: ["Entry 1: a condition cannot compare two numbers; one side must be a series."] });
  expect([...host.querySelectorAll<HTMLButtonElement>("button")].filter((b) => b.textContent === "+ Add condition").every((b) => b.disabled)).toBe(true);
  expect(host.querySelector<HTMLButtonElement>('[data-lab-action="run"]')!.disabled).toBe(true);
  expect(host.textContent).toContain("one side must be a series");
});

it("orders: a limit hides the entry fill and removes it; ATR stops; short direction", async () => {
  const { host, last } = await setup({ ...defaultSpec(), execution: { fill: "close" } });
  const row = (label: string) => [...host.querySelectorAll<HTMLElement>(".lab-order")].find((r) => r.textContent!.startsWith(label))!;
  await act(async () => [...row("Limit entry").querySelectorAll<HTMLButtonElement>(".join-item")].find((b) => b.textContent === "%")!.click());
  expect(last().entry.limit).toBeDefined();
  expect(last().execution).toBeUndefined();
  await act(async () => [...row("Stop-loss").querySelectorAll<HTMLButtonElement>(".join-item")].find((b) => b.textContent === "ATR")!.click());
  expect(last().exit.stop_loss).toEqual({ atr: 2, atr_length: 14 });
  await act(async () => [...host.querySelectorAll<HTMLButtonElement>(".join-item")].find((b) => b.textContent === "Short")!.click());
  expect(last().direction).toBe("short");
});

it("the fees box edits cost_bps when the spec carries it", async () => {
  const { host, last, onSettings } = await setup({ ...defaultSpec(), execution: { cost_bps: 10 } });
  const fees = host.querySelector<HTMLInputElement>('input[aria-label="Fees (bps / side)"]')!;
  expect(fees.value).toBe("10");
  await act(async () => {
    Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(fees, "12");
    fees.dispatchEvent(new Event("input", { bubbles: true }));
  });
  expect(last().execution?.cost_bps).toBe(12);
  expect(onSettings).not.toHaveBeenCalledWith(expect.objectContaining({ fees: "12" }));
});
```

Default ATR form: multiplier 2, `atr_length` 14; default % form: limit 1, stop-loss 5, target 10, trailing 5.

- [ ] **Step 2: Run** — FAIL.
- [ ] **Step 3: Implement** the three components and `rules.css` (`.lab-cond { display: flex; flex-wrap: wrap; gap: 6px; align-items: center; padding: 8px 0; border-bottom: 1px solid var(--rule); }`, `.lab-side + .lab-side { margin-top: 14px; }`, `.lab-order { display: flex; flex-wrap: wrap; gap: 8px; align-items: center; }`, inputs `width: 76px`; tokens only).
- [ ] **Step 4: Run** — PASS; own-file typecheck clean.
- [ ] **Step 5: Commit**

```bash
cd /home/aff34/.cache/strategy-vault-development; F=Context/guru/frontend/react; L=$F/src/features/lab
P="$L/RuleBuilder.tsx $L/OperandEditor.tsx $L/OrdersPanel.tsx $L/rules.css $F/tests/lab-rule-builder.test.tsx"
git add -- $P && git commit -m "feat(lab-ui): rule builder with operands, price orders and execution" -- $P
```

---

### Task 10: Backtest results

**Files:**
- Create: `src/features/lab/RunResults.tsx`, `src/features/lab/results.css`
- Test: `tests/lab-results.test.tsx`

**Interfaces:**
- Consumes: types, `COPY` (Task 1); `ServerLabel`, `MetricTiles`, `ProvisionalStrip`, `isProvisional` (Task 3); `pct`, `num`, `int` (Task 3); `friendlyReason` (Task 4); `createLabChart`, `useThemeKey` (Task 5); `window.VaultReport` (`globals.d.ts`).
- Produces: `RunResults({result, market}: {result: RunResult; market: SavedMarket})`, root `section.box[data-lab-result]`; exported pure helpers `kpiItems(stats)`, `tradeRows(trades)`, `drawdown(equity)`.

Units: confirm against `function label(` in `src/compat/runtime.js` (read-only) that `total_return`, `cagr`, `max_dd`, `win_rate`, `avg_trade`, `exposure`, `bh_cagr` are ratios (the book presenters format them as percent); `sharpe` is a ratio number; `trades` an integer.

Content, in order:
1. `h2.box-h` "Backtest" + `<small>{from} → {to} · {instrument.name ?? instrument.ticker}</small>`; `ServerLabel(result.label)`; on gl `ProvisionalStrip(result.provisional_note, instrument.label)`.
2. `start` → `.rnote` `COPY.startMoved(start.date)` (never the server `reason`).
3. `MetricTiles(kpiItems(stats))`: Net return `pct(total_return)`, CAGR `pct(cagr)`, Max drawdown `pct(max_dd)`, Sharpe `num(sharpe)`, Trades `int(trades)`, Win rate `pct(win_rate)`, Avg trade `pct(avg_trade, 2)`, Buy & hold CAGR `pct(bh_cagr)`. `.cap`: `Exposure ${pct(exposure)} · before costs: ${pct(gross_total_return)}` and `Fees ${costs.fees_bps} bps + slippage ${costs.slippage_bps} bps per side · taxes not modelled`.
4. Equity chart `.lwc[data-lab-equity]` via `createLabChart(el, 320, {rightPriceScale: {mode: 1}})` (1 = logarithmic): `addAreaSeries({lineColor: colors.chart, topColor: colors.chart + "33", bottomColor: "transparent"})` with `equity`, `addLineSeries({color: colors.bh, lineWidth: 1})` with `bh`; legend `.legend` "Strategy" / "Buy & hold". Drawdown chart `.lwc[data-lab-drawdown]` height 160, area of `drawdown(equity)` (`equity[i] / runningPeak - 1`) in `colors.red`. Both disposed in the effect cleanup; effect deps `[result, themeKey]`.
5. Monthly returns: when `window.VaultReport` exists, `VaultReport.monthly({dates, equity: equity.map(v => v / equity[0])})` → `.tblwrap > table.tbl` Year, Jan…Dec, Total (`pct`, `td.n`, `.pos/.neg`); otherwise omitted.
6. Tradesheet `section` "Tradesheet" + `<small>{n} trades · newest first</small>` + "Download CSV" `button.dbtn.sm`; `.tlog > table.tbl.tl` with columns Entry date, Side, Fill, Entry, Exit date, Exit, Return, Bars, Exit reason; rows `tradeRows(trades)` newest first, first 200; footer `.cap` "{n − 200} older trades are in the CSV." when more. Side `Long`/`Short`; Fill `{next_open: "Next open", close: "Close", limit: "Limit"}[k] ?? k`; Entry/Exit `num(px)`; Return `pct(ret, 2)`; Bars `int(bars)`; Exit reason `friendlyReason(reason)`; open trade → exit date "Open" (its exit price is the last mark, still shown). No trades → `COPY.noTrades`.
7. CSV: `VaultReport.csv([header, ...allRows])` → `Blob` (`text/csv;charset=utf-8`) → `URL.createObjectURL` → temporary `a[download=lab-<ticker>-<from>-<to>.csv]` click → `URL.revokeObjectURL`. Button hidden when `VaultReport` is missing.

- [ ] **Step 1: Write the failing test** — `tests/lab-results.test.tsx`

```tsx
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { act } from "react";
import { RunResults, drawdown, kpiItems, tradeRows } from "../src/features/lab/RunResults";
import { PROVISIONAL_NOTE } from "../src/features/lab/labels";
import { mount, runFixture, unmount } from "./fixtures/lab";

beforeEach(() => {
  window.VaultReport = { monthly: vi.fn(() => ({ months: { "2024-01": 0.02 }, annual: { "2024": 0.05 }, years: ["2024"], averages: [], annualAverage: 0.05 })),
    csv: vi.fn(() => "a,b") };
});
afterEach(async () => { await unmount(); delete window.VaultReport; vi.unstubAllGlobals(); });

it("label verbatim, own start copy, KPIs and costs", async () => {
  const host = await mount(<RunResults result={runFixture} market="in" />);
  expect(host.querySelector("[data-server-label]")?.textContent).toBe(runFixture.label);
  expect(host.textContent).toContain("Results start on 2024-01-08, not the requested start");
  expect(host.textContent).not.toContain("stale");
  expect(kpiItems(runFixture.stats).map((k) => [k.label, k.value])).toEqual([
    ["Net return", "5.0%"], ["CAGR", "31.0%"], ["Max drawdown", "-1.0%"], ["Sharpe", "1.42"],
    ["Trades", "2"], ["Win rate", "50.0%"], ["Avg trade", "0.02%"], ["Buy & hold CAGR", "40.0%"]]);
  expect(host.textContent).toContain("Fees 5 bps + slippage 2 bps per side · taxes not modelled");
  expect(host.textContent).not.toMatch(/\bverified\b/i);
});

it("tradesheet: newest first, side, fill, open trade and friendly reasons", () => {
  const rows = tradeRows(runFixture.trades);
  expect(rows[0]).toEqual(["2024-02-01", "Short", "Close", "105.00", "Open", "106.00", "-0.95%", "1", "Still open at the end"]);
  expect(rows[1][8]).toBe("Exit rule 1: bars_held >= 2");
  expect(rows[1][2]).toBe("Next open");
});

it("caps the table at 200 rows and points to the CSV", async () => {
  const many = { ...runFixture, trades: Array.from({ length: 205 }, () => runFixture.trades[0]) };
  const host = await mount(<RunResults result={many} market="in" />);
  expect(host.querySelectorAll("table.tl tbody tr")).toHaveLength(200);
  expect(host.textContent).toContain("5 older trades are in the CSV.");
});

it("CSV download uses VaultReport.csv", async () => {
  Object.assign(URL, { createObjectURL: vi.fn(() => "blob:x"), revokeObjectURL: vi.fn() }); // jsdom has neither
  const host = await mount(<RunResults result={runFixture} market="in" />);
  await act(async () => [...host.querySelectorAll<HTMLButtonElement>("button")].find((b) => b.textContent === "Download CSV")!.click());
  expect(window.VaultReport!.csv).toHaveBeenCalled();
  expect((window.VaultReport!.csv as ReturnType<typeof vi.fn>).mock.calls[0][0][0][0]).toBe("Entry date");
});

it("provisional strip on gl and drawdown math", async () => {
  const gl = { ...runFixture, provisional: true as const, instrument: { ...runFixture.instrument, label: "Provisional · TradingView export HSI:HSI, daily" } };
  const host = await mount(<RunResults result={gl} market="gl" />);
  expect(host.querySelector("[data-lab-provisional]")?.textContent).toContain(PROVISIONAL_NOTE);
  expect(host.querySelector("[data-lab-provisional]")?.textContent).toContain("HSI:HSI");
  expect(drawdown([1, 2, 1, 3]).map((v) => Number(v.toFixed(2)))).toEqual([0, 0, -0.5, 0]);
});
```

- [ ] **Step 2: Run** — FAIL.
- [ ] **Step 3: Implement** `RunResults.tsx` and `results.css` (`.lab-result .lwc { height: auto; }`, legend swatches via existing `.legend .sw-line`; tokens only).
- [ ] **Step 4: Run** — PASS; own-file typecheck clean.
- [ ] **Step 5: Commit**

```bash
cd /home/aff34/.cache/strategy-vault-development; F=Context/guru/frontend/react; L=$F/src/features/lab
P="$L/RunResults.tsx $L/results.css $F/tests/lab-results.test.tsx"
git add -- $P && git commit -m "feat(lab-ui): backtest result with KPIs, equity, drawdown, monthly and tradesheet" -- $P
```

---

### Task 11: Optimization panel

**Files:**
- Create: `src/features/lab/OptimizePanel.tsx`, `src/features/lab/optimize.css`
- Test: `tests/lab-optimize.test.tsx`

**Interfaces:**
- Consumes: types, `COPY` (Task 1); `ErrorNotice`, `ServerLabel`, `ProvisionalStrip`, `isProvisional`, `Loading` (Task 3); `pct`, `num`, `int` (Task 3); `optimizablePaths`, `valuesCount`, `comboCount`, `withOptimize`, `setAt`, `splitDate` (Task 4).
- Produces:

```ts
export function OptimizePanel(props: {
  spec: Spec; catalogue: Catalogue; market: SavedMarket; role: Role | null; dates: string[];
  onOptimize(spec: Spec, split: string | null): Promise<OptimizeResult>;
  onApply(spec: Spec): void; onTried(n: number): void; tried: number; instrumentLabel?: string;
}): JSX.Element;
export function colValue(result: OptimizeResult, values: number[], name: string): number | null; // row (full cols) or base (cols after dims)
```

Behaviour:
- Root `section.box[data-lab-optimize]`; `h2.box-h` "Optimization" + `<small>same ranking as the Guru optimizer · tick what to vary</small>`.
- `role === "viewer"` (or `null` while loading): only `.rnote` `COPY.researcherOnly`.
- Dimension table: one `tr[data-lab-dim=<path>]` per `optimizablePaths(spec, catalogue)` — checkbox, label, From, To, Step inputs, Values `valuesCount`. Ticking initialises `{from: current, to: min(max, current + 4·step), step}`. Selections whose path disappears from the spec are dropped. Footer: `{comboCount} combinations` (or `COPY.tickOne`), and a warning when any dim has > 1,000 values or combos > `catalogue.max_combos` (button disabled).
- Split `select` aria-label "Held back": None | last 20% | last 30% | last 40%; with a choice the line `COPY.splitLine(pct, date)` using `splitDate(dates, pct)`; when that returns `null`, "Not enough history for this split." and the button is disabled.
- `button.dbtn.primary[data-lab-action="optimize"]` "Run optimization" → `onOptimize(withOptimize(spec, dims), split)`. While running: `Loading` "Optimizing… {s} s" updated every second, plus `COPY.optimizeLimit`. A new run clears the old result first.
- Error → `ErrorNotice` (with `onRetry` = rerun) — never the old table.
- Result `div[data-lab-optimize-result]`: `ServerLabel(result.label)`; gl strip; "Ranked on: in-sample only" / "Ranked on: full period"; `.cap` `{combos} settings tested · {eligible} eligible (at least {min_trades} trades) · {distinct} distinct`; `eligible === 0` → `COPY.noEligible(min_trades)` and no table. Table `.tblwrap > table.tbl.ocmp`: header = "", each dim label, then the five `rank.stats` columns (`cagr → CAGR`, `sharpe → Sharpe`, `monthly_thickness → Monthly thickness`, `expectancy_ratio → Expectancy ratio`, `mar → CAR/MDD`) with value and percentile (`{value} · p{pct}`), then with a split: `is_total_return`, `is_cagr`, `is_sharpe`, `is_trades` under a group header "Ranking period" and `oos_*` under "Held back · display only"; last column Apply. Rows: "Now" from `base` (no Apply), then `#1..#3` from `rank.top3` reading `picked[String(row)]`. With a split: `.rnote` `COPY.splitNote`.
- Apply: `onApply(dims.reduce((s, d, i) => setAt(s, d.path, values[i]), spec))` (the spec without `optimize`).
- `onTried(result.combos)` once per finished optimization; the panel shows `Variants tried this session: {tried}`.

- [ ] **Step 1: Write the failing test** — `tests/lab-optimize.test.tsx`

```tsx
import { afterEach, expect, it, vi } from "vitest";
import { act } from "react";
import { OptimizePanel, colValue } from "../src/features/lab/OptimizePanel";
import { defaultSpec } from "../src/features/lab/spec-model";
import type { Spec } from "../src/features/lab/types";
import { catalogueFixture as cat, flush, mount, optimizeFixture, unmount } from "./fixtures/lab";

afterEach(async () => { await unmount(); vi.useRealTimers(); });
const dates = Array.from({ length: 300 }, (_, i) => new Date(Date.UTC(2020, 0, 1 + i)).toISOString().slice(0, 10));
const base = { spec: defaultSpec(), catalogue: cat, market: "in" as const, role: "researcher" as const, dates, onApply: vi.fn(), onTried: vi.fn(), tried: 0 };
const tick = async (host: HTMLElement, path: string) => act(async () => host.querySelector<HTMLInputElement>(`[data-lab-dim="${path}"] input[type=checkbox]`)!.click());

it("viewers see the researcher notice and no controls", async () => {
  const host = await mount(<OptimizePanel {...base} role="viewer" onOptimize={vi.fn()} />);
  expect(host.textContent).toContain("Researcher access is required for optimization.");
  expect(host.querySelector('[data-lab-action="optimize"]')).toBeNull();
});

it("ticking a setting fills a small range; split sends a date; optimize carries the dims", async () => {
  const onOptimize = vi.fn().mockResolvedValue(optimizeFixture);
  const host = await mount(<OptimizePanel {...base} onOptimize={onOptimize} />);
  await tick(host, "entry.all[0].rhs.p.length");
  expect(host.textContent).toContain("5 combinations");
  const split = host.querySelector<HTMLSelectElement>('select[aria-label="Held back"]')!;
  await act(async () => { split.value = "30"; split.dispatchEvent(new Event("change", { bubbles: true })); });
  await act(async () => host.querySelector<HTMLButtonElement>('[data-lab-action="optimize"]')!.click());
  await flush();
  const [spec, splitDate] = onOptimize.mock.calls[0] as [Spec, string];
  expect(spec.optimize).toEqual([{ path: "entry.all[0].rhs.p.length", from: 50, to: 54, step: 1 }]);
  expect(splitDate).toBe(dates[210]);
  expect(base.onTried).toHaveBeenCalledWith(3);
});

it("result: ranked-on text, Now and top 3, held-back columns, Apply writes the setting", async () => {
  const onApply = vi.fn();
  const host = await mount(<OptimizePanel {...base} onApply={onApply} onOptimize={vi.fn().mockResolvedValue(optimizeFixture)} />);
  await tick(host, "entry.all[0].rhs.p.length");
  await act(async () => host.querySelector<HTMLButtonElement>('[data-lab-action="optimize"]')!.click());
  await flush();
  const result = host.querySelector("[data-lab-optimize-result]")!;
  expect(result.textContent).toContain("Ranked on: in-sample only");
  expect(result.textContent).toContain("Held back · display only");
  expect(result.textContent).toContain("trades that cross the split date are cut at the boundary");
  expect([...result.querySelectorAll("tbody tr")].map((r) => r.firstElementChild!.textContent)).toEqual(["Now", "#1", "#2"]);
  await act(async () => result.querySelector<HTMLButtonElement>("tbody tr:nth-child(2) button")!.click());
  expect((onApply.mock.calls[0][0] as Spec).entry.all[0].rhs).toEqual({ s: "ema", p: { length: 20 } });
  expect((onApply.mock.calls[0][0] as Spec).optimize).toBeUndefined();
  expect(colValue(optimizeFixture, optimizeFixture.base, "cagr")).toBe(0.05);
  expect(colValue(optimizeFixture, optimizeFixture.picked["1"], "oos_trades")).toBe(7);
});

it("429 shows a countdown and re-enables retry", async () => {
  vi.useFakeTimers();
  const busy = Object.assign(new Error("An optimization is already running. Please retry when it finishes."), { status: 429, kind: "gateway", retryAfter: 5 });
  const onOptimize = vi.fn().mockRejectedValueOnce(busy).mockResolvedValueOnce(optimizeFixture);
  const host = await mount(<OptimizePanel {...base} onOptimize={onOptimize} />);
  await tick(host, "entry.all[0].rhs.p.length");
  await act(async () => host.querySelector<HTMLButtonElement>('[data-lab-action="optimize"]')!.click());
  await act(async () => { await Promise.resolve(); });
  const retry = () => host.querySelector<HTMLButtonElement>("[data-lab-error] button")!;
  expect(retry().textContent).toBe("Try again in 5 s");
  expect(host.querySelector("[data-lab-optimize-result]")).toBeNull();
  await act(async () => { vi.advanceTimersByTime(5000); });
  await act(async () => retry().click());
  await act(async () => { await Promise.resolve(); await Promise.resolve(); });
  expect(onOptimize).toHaveBeenCalledTimes(2);
});

it("the engine timeout text is shown as given, with no numbers", async () => {
  const timeout = Object.assign(new Error("optimization stopped after 300 s; narrow the grid. No partial ranking is published."), { status: 400, kind: "engine", engineStatus: "invalid" });
  const host = await mount(<OptimizePanel {...base} onOptimize={vi.fn().mockRejectedValue(timeout)} />);
  await tick(host, "entry.all[0].rhs.p.length");
  await act(async () => host.querySelector<HTMLButtonElement>('[data-lab-action="optimize"]')!.click());
  await flush();
  expect(host.querySelector("[data-lab-error]")?.textContent).toContain("No partial ranking is published.");
  expect(host.querySelector("table.ocmp")).toBeNull();
});

it("no eligible settings: nothing is ranked", async () => {
  const none = { ...optimizeFixture, rank: { ...optimizeFixture.rank, eligible: 0, distinct: 0, top3: [], stats: undefined } };
  const host = await mount(<OptimizePanel {...base} onOptimize={vi.fn().mockResolvedValue(none)} />);
  await tick(host, "entry.all[0].rhs.p.length");
  await act(async () => host.querySelector<HTMLButtonElement>('[data-lab-action="optimize"]')!.click());
  await flush();
  expect(host.textContent).toContain("No setting had enough trades to rank (at least 10). Nothing is ranked.");
});
```

- [ ] **Step 2: Run** — FAIL.
- [ ] **Step 3: Implement** `OptimizePanel.tsx` (`colValue`: `const i = result.cols.indexOf(name); if (i < 0) return null; const j = values.length === result.cols.length ? i : i - result.dims.length; return typeof values[j] === "number" ? values[j] : null;`) and `optimize.css` (dim inputs `width: 80px`; tokens only).
- [ ] **Step 4: Run** — PASS; own-file typecheck clean.
- [ ] **Step 5: Commit**

```bash
cd /home/aff34/.cache/strategy-vault-development; F=Context/guru/frontend/react; L=$F/src/features/lab
P="$L/OptimizePanel.tsx $L/optimize.css $F/tests/lab-optimize.test.tsx"
git add -- $P && git commit -m "feat(lab-ui): optimization panel with split, ranking table and busy handling" -- $P
```

---

### Task 12: Saved strategies and admin review queue

**Files:**
- Create: `src/features/lab/SavedStrategies.tsx`, `src/features/lab/ReviewQueue.tsx`, `src/features/lab/saved.css`
- Test: `tests/lab-saved.test.tsx`

**Interfaces:**
- Consumes: types, `COPY`, `variantLabel` (Task 1); `ErrorNotice`, `Loading`, `isProvisional` (Task 3); `StrategiesApi` type and `strategies` (Task 2, import type only + default prop).
- Produces:

```ts
export interface EditorState { spec: Spec; market: SavedMarket; ticker: string; row: LabStrategy | null; unchanged: boolean; ranFresh: boolean }
export function SavedStrategies(props: {
  role: Role | null; api: Pick<StrategiesApi, "list" | "get" | "create" | "update" | "remove" | "duplicate" | "action">;
  editor: EditorState; methodNames?: Record<string, string>;
  onOpen(row: LabStrategy): void; onSaved(row: LabStrategy): void;
}): JSX.Element;
export function ReviewQueue(props: { api: Pick<StrategiesApi, "review" | "action">; onOpen(row: LabStrategy): void }): JSX.Element;
```

Behaviour (SavedStrategies, root `section.box[data-lab-saved]`, `h2.box-h` "My indicator strategies"):
- Loads `api.list()` on mount; failure → `ErrorNotice` (market `"in"`; e.g. the dev proxy's 503) and the save controls still render. An empty list (after load or after the last delete) → `.empty` with `COPY.savedEmpty`.
- Viewer: list only, no name field, no Save buttons, no Submit/Delete/Duplicate.
- Save row: name `input` (placeholder "Strategy name", 1–120 chars, prefilled with `editor.row?.name`), `button[data-lab-action="save-new"]` "Save as new" → `api.create({name, spec, market: editor.market, instrument: editor.ticker})`; when `editor.row` is own and status draft/rejected: `button[data-lab-action="save"]` "Save changes" → `api.update(row.id, row.version, draft)`. Success → replace/insert the row locally, call `onSaved(row)`, status text "Saved "{name}"".
- List grouping: rows with `origin` grouped under `h3` `My variants of {methodNames[origin.method] ?? origin.method}`; others under "Lab strategies". Each row: name, variant badge `span.tag` (`variantLabel(origin.book, methodName)` shortened to "Customized from {book} · {method}"), meta `{instrument} · {market} · {entry.all.length} entry / {exit.any.length} exit · updated {updatedAt.slice(0,10)}`, status chip (`Draft`, `Submitted`, `Published`, `Rejected`), `reviewNote` only when status is rejected or published (`Review note: …`). Buttons: Open (all) → `onOpen(row)`; Duplicate → `api.duplicate(id)` inserted at the top; Delete (draft/rejected) → `confirm(COPY.deleteConfirm(name))` then `api.remove(id, version)`, row removed; Submit (draft/rejected) → `api.action(id, "submit", version)`; Withdraw (submitted) → `api.action(id, "withdraw", version)`.
- Submit enabled only when `row.publishable && editor.row?.id === row.id && editor.unchanged && editor.ranFresh`; otherwise disabled with `title` `COPY.provisionalSubmit` (not publishable) or `COPY.runFirst`. Success → status text `COPY.submitted`.
- Errors from any mutation show in an `ErrorNotice` under the row (message as given). For 409 add `button` "Reload this strategy" → `api.get(id)`, replace the row and show `COPY.keptAfterReload(row.version)`; when that row is open in the editor also call `onOpen`? **No** — reloading must keep the rules on screen; call `onSaved(fresh)` so the page updates its `row` (version) without touching the spec.

ReviewQueue (`section.box[data-lab-review]`, `h2.box-h` "Review queue"): loads `api.review()`; empty → "No strategies are waiting for review."; each row: name, owner, market, instrument, submitted date, Open in lab, a note `textarea` (aria-label "Review note", max 1000), Publish (note optional) → `api.action(id, "publish", version, note || undefined)`, Reject (disabled until the trimmed note has 1–1000 chars) → `api.action(id, "reject", version, note)`; success removes the row; errors as given.

- [ ] **Step 1: Write the failing test** — `tests/lab-saved.test.tsx`

```tsx
import { afterEach, expect, it, vi } from "vitest";
import { act } from "react";
import { SavedStrategies, type EditorState } from "../src/features/lab/SavedStrategies";
import { ReviewQueue } from "../src/features/lab/ReviewQueue";
import { flush, mount, strategyFixture, unmount } from "./fixtures/lab";
import type { LabStrategy } from "../src/features/lab/types";

afterEach(async () => { await unmount(); vi.restoreAllMocks(); });
const conflict = () => Object.assign(new Error("This strategy changed since you opened it (now version 4). Reload before saving."), { status: 409, kind: "auth" });
const api = (rows: LabStrategy[], patch: Record<string, unknown> = {}) => ({
  list: vi.fn().mockResolvedValue(rows), get: vi.fn(), create: vi.fn(), update: vi.fn(), remove: vi.fn().mockResolvedValue(null),
  duplicate: vi.fn(), action: vi.fn(), ...patch,
}) as never;
const editor = (row: LabStrategy | null, extra: Partial<EditorState> = {}): EditorState =>
  ({ spec: strategyFixture().spec, market: "in", ticker: "NIFTY 50", row, unchanged: true, ranFresh: true, ...extra });
const button = (host: HTMLElement, text: string) => [...host.querySelectorAll<HTMLButtonElement>("button")].find((b) => b.textContent === text)!;

it("lists rows with status and shows review notes only when rejected or published", async () => {
  const rows = [strategyFixture({ id: 1, status: "rejected", reviewNote: "Add a stop" }), strategyFixture({ id: 2, status: "submitted", reviewNote: "hidden" })];
  const host = await mount(<SavedStrategies role="researcher" api={api(rows)} editor={editor(null)} onOpen={vi.fn()} onSaved={vi.fn()} />);
  await flush();
  expect(host.textContent).toContain("Rejected");
  expect(host.textContent).toContain("Review note: Add a stop");
  expect(host.textContent).not.toContain("hidden");
  expect(button(host, "Withdraw")).toBeDefined();
});

it("save as new maps the market and save changes sends the version", async () => {
  const a = api([strategyFixture()], { create: vi.fn().mockResolvedValue(strategyFixture({ id: 9 })), update: vi.fn().mockResolvedValue(strategyFixture({ version: 4 })) });
  const host = await mount(<SavedStrategies role="researcher" api={a} editor={{ ...editor(strategyFixture()), market: "crypto", ticker: "CRYPTO:BTCUSD" }} onOpen={vi.fn()} onSaved={vi.fn()} />);
  await flush();
  await act(async () => host.querySelector<HTMLButtonElement>('[data-lab-action="save-new"]')!.click());
  await act(async () => host.querySelector<HTMLButtonElement>('[data-lab-action="save"]')!.click());
  expect((a as { create: ReturnType<typeof vi.fn> }).create.mock.calls[0][0]).toMatchObject({ market: "crypto", instrument: "CRYPTO:BTCUSD", name: "EMA pullback" });
  expect((a as { update: ReturnType<typeof vi.fn> }).update.mock.calls[0].slice(0, 2)).toEqual([7, 3]);
});

it("409 offers a reload that keeps the rules and saves with the new version", async () => {
  const onSaved = vi.fn();
  const a = api([strategyFixture()], { update: vi.fn().mockRejectedValueOnce(conflict()).mockResolvedValueOnce(strategyFixture({ version: 5 })),
    get: vi.fn().mockResolvedValue(strategyFixture({ version: 4 })) });
  const host = await mount(<SavedStrategies role="researcher" api={a} editor={editor(strategyFixture())} onOpen={vi.fn()} onSaved={onSaved} />);
  await flush();
  await act(async () => host.querySelector<HTMLButtonElement>('[data-lab-action="save"]')!.click());
  await flush();
  expect(host.textContent).toContain("now version 4");
  await act(async () => button(host, "Reload this strategy").click());
  await flush();
  expect(host.textContent).toContain("Your rules on screen are kept. Save again to replace version 4.");
  expect(onSaved).toHaveBeenCalledWith(expect.objectContaining({ version: 4 }));
});

it("delete asks first and sends the version", async () => {
  vi.spyOn(window, "confirm").mockReturnValue(true);
  const a = api([strategyFixture()]);
  const host = await mount(<SavedStrategies role="researcher" api={a} editor={editor(null)} onOpen={vi.fn()} onSaved={vi.fn()} />);
  await flush();
  await act(async () => button(host, "Delete").click());
  await flush();
  expect(window.confirm).toHaveBeenCalledWith('Delete "EMA pullback"? This cannot be undone.');
  expect((a as { remove: ReturnType<typeof vi.fn> }).remove).toHaveBeenCalledWith(7, 3);
  expect(host.textContent).toContain("Nothing saved yet. Name the current rules above and save them.");
});

it("submit needs a fresh run of the unchanged row and a publishable market", async () => {
  const row = strategyFixture();
  const stale = await mount(<SavedStrategies role="researcher" api={api([row])} editor={editor(row, { ranFresh: false })} onOpen={vi.fn()} onSaved={vi.fn()} />);
  await flush();
  expect(button(stale, "Submit").disabled).toBe(true);
  expect(button(stale, "Submit").title).toBe("Run the current rules first; the engine checks the rules only when they run.");
  await unmount();
  const gl = strategyFixture({ market: "gl", publishable: false });
  const host = await mount(<SavedStrategies role="researcher" api={api([gl])} editor={editor(gl)} onOpen={vi.fn()} onSaved={vi.fn()} />);
  await flush();
  expect(button(host, "Submit").title).toBe("Provisional data cannot be published. Rerun on licensed data first.");
});

it("viewers get no save controls; variants are grouped by method", async () => {
  const v = strategyFixture({ id: 3, origin: { book: "ststw", method: "rsi2_4", template_version: 1 } });
  const host = await mount(<SavedStrategies role="viewer" api={api([v])} editor={editor(null)} methodNames={{ rsi2_4: "RSI 2 · 4" }} onOpen={vi.fn()} onSaved={vi.fn()} />);
  await flush();
  expect(host.querySelector('[data-lab-action="save-new"]')).toBeNull();
  expect(host.textContent).toContain("My variants of RSI 2 · 4");
});

it("review queue: reject needs a note; publish sends the version", async () => {
  const row = strategyFixture({ status: "submitted" });
  const action = vi.fn().mockResolvedValue({ ...row, status: "published" });
  const host = await mount(<ReviewQueue api={{ review: vi.fn().mockResolvedValue([row]), action } as never} onOpen={vi.fn()} />);
  await flush();
  expect(button(host, "Reject").disabled).toBe(true);
  await act(async () => button(host, "Publish").click());
  expect(action).toHaveBeenCalledWith(7, "publish", 3, undefined);
});
```

- [ ] **Step 2: Run** — FAIL.
- [ ] **Step 3: Implement** both components and `saved.css` (`.lab-saved-row { display: flex; flex-wrap: wrap; gap: 8px; align-items: center; padding: 10px 0; border-bottom: 1px solid var(--rule); }`, status chips via `.tag`; tokens only).
- [ ] **Step 4: Run** — PASS; own-file typecheck clean.
- [ ] **Step 5: Commit**

```bash
cd /home/aff34/.cache/strategy-vault-development; F=Context/guru/frontend/react; L=$F/src/features/lab
P="$L/SavedStrategies.tsx $L/ReviewQueue.tsx $L/saved.css $F/tests/lab-saved.test.tsx"
git add -- $P && git commit -m "feat(lab-ui): saved strategies with version handling and admin review" -- $P
```

---

### Task 13: Customize panel (templates, template summary, fidelity)

**Files:**
- Create: `src/features/lab/CustomizePanel.tsx`, `src/features/lab/customize.css`
- Test: `tests/lab-customize.test.tsx`

**Interfaces:**
- Consumes: types, `COPY`, `TEMPLATE_LABEL`, `FIDELITY_LABEL`, `unavailableText` (Task 1); `ErrorNotice`, `ServerLabel`, `Loading` (Task 3).
- Produces:

```ts
export function CustomizePanel(props: {
  market: SavedMarket; ticker: string; selected: string;
  templates: TemplatesResponse | null; templatesError?: unknown;
  template: TemplateResponse | null; templateError?: unknown;
  onSelect(methodId: string): void; onClear(): void;
  loadFidelity(methodId: string, ticker: string): Promise<FidelityResult>;
}): JSX.Element;
```

Behaviour (root `section.box[data-lab-customize]`, `h2.box-h` "Customize a book method"):
- `market` is gl/crypto → only `.rnote` `COPY.customizeGlOnly`.
- Picker: `ServerLabel(templates.label)`; `.cap` `{templates.length} templates · {admitted} admitted methods · {methods} catalogued`; search input (aria-label "Search book methods") over method `name`, `id`, `book`; list of buttons `{name} · {book}` + `<small>{requirement}</small>` → `onSelect(id)`. `templatesError` → `ErrorNotice`.
- Selected + `template.status === "unavailable"` → `.rnote` `unavailableText(template.reason)` and a "Clear" button (`onClear`).
- Selected + available → summary: `ServerLabel(template.label)`, `h3` `{method.name} · {method.book}`, "Instrument: {requirement} ({allowed_types.join(", ")})", notes `ul`, "Template version {template.version}", "Clear" button. Then the fidelity block.
- `templateError` → `ErrorNotice`.
- Fidelity block: `button[data-lab-action="fidelity"]` "Compare with the book engine (display only)" → `loadFidelity(method.id, ticker)`; while loading `Loading`; result: `ServerLabel(result.label)`, `.rnote` `result.note` verbatim; headline: `reproduces` → `Template reproduces the book engine's trades: {matched}/{book_trades}`; `book_trades === 0 && template_trades === 0` → `COPY.fidelityNothing`; else `Matched {matched} of {book_trades} book trades and {template_trades} template trades`, then a `.tlog > table.tbl` of differences (Book entry, Book exit, Template entry, Template exit; `—` for null) and `.cap` `{differences_total} differences in all` when more than shown. Error → `ErrorNotice` (text as given; the 400 already carries the prefix). Changing `selected` or `ticker` clears the fidelity result.

- [ ] **Step 1: Write the failing test** — `tests/lab-customize.test.tsx`

```tsx
import { afterEach, expect, it, vi } from "vitest";
import { act } from "react";
import { CustomizePanel } from "../src/features/lab/CustomizePanel";
import { fidelityFixture, flush, mount, templateFixture, templatesFixture, unavailableTemplateFixture, unmount } from "./fixtures/lab";

afterEach(unmount);
const base = { market: "in" as const, ticker: "NIFTY 50", selected: "", templates: templatesFixture, template: null, onSelect: vi.fn(), onClear: vi.fn(), loadFidelity: vi.fn() };
const click = (host: HTMLElement, sel: string) => act(async () => host.querySelector<HTMLButtonElement>(sel)!.click());

it("gl has no Customize", async () => {
  const host = await mount(<CustomizePanel {...base} market="gl" />);
  expect(host.textContent).toContain("Customize runs on India and US book methods only.");
});

it("lists templates with counts and searches by book", async () => {
  const host = await mount(<CustomizePanel {...base} />);
  expect(host.textContent).toContain("2 templates · 164 admitted methods · 4365 catalogued");
  const search = host.querySelector<HTMLInputElement>('input[aria-label="Search book methods"]')!;
  await act(async () => {
    Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(search, "etf trading");
    search.dispatchEvent(new Event("input", { bubbles: true }));
  });
  const buttons = [...host.querySelectorAll<HTMLButtonElement>("[data-lab-method]")];
  expect(buttons.map((b) => b.dataset.labMethod)).toEqual(["hpetf_rsi"]);
  await act(async () => buttons[0].click());
  expect(base.onSelect).toHaveBeenCalledWith("hpetf_rsi");
});

it("unavailable methods get the prefix once", async () => {
  const host = await mount(<CustomizePanel {...base} selected="rsi2_4" template={unavailableTemplateFixture} />);
  expect(host.textContent).toContain("Customize not available — the book method is not admitted");
  expect(host.textContent).not.toContain("Customize not available — Customize not available");
});

it("available template: label, requirement, notes; fidelity is display only", async () => {
  const loadFidelity = vi.fn().mockResolvedValue(fidelityFixture);
  const host = await mount(<CustomizePanel {...base} selected="rsi2_4" template={templateFixture} loadFidelity={loadFidelity} />);
  expect(host.textContent).toContain("Book template · declared adaptation into lab rules · not a verification");
  expect(host.textContent).toContain("index or ETF (index, etf)");
  expect(host.textContent).toContain("Entry at the close, as in the book.");
  await click(host, '[data-lab-action="fidelity"]');
  await flush();
  expect(loadFidelity).toHaveBeenCalledWith("rsi2_4", "NIFTY 50");
  expect(host.textContent).toContain("Template fidelity · display only · not a verification");
  expect(host.textContent).toContain("Template reproduces the book engine's trades: 12/12");
  expect(host.textContent).toContain("It does not verify the book method");
  expect(host.textContent!.replace(/does not verify/g, "")).not.toMatch(/\bverified\b/i);
});

it("0/0 is not a reproduction; differences are listed; refusals are shown as given", async () => {
  const empty = { ...fidelityFixture, book_trades: 0, template_trades: 0, matched: 0, reproduces: false };
  const host = await mount(<CustomizePanel {...base} selected="rsi2_4" template={templateFixture} loadFidelity={vi.fn().mockResolvedValue(empty)} />);
  await click(host, '[data-lab-action="fidelity"]');
  await flush();
  expect(host.textContent).toContain("No trades on this window, so there is nothing to compare.");
  expect(host.textContent).not.toContain("reproduces");
  await unmount();
  const diff = { ...fidelityFixture, reproduces: false, matched: 10, template_trades: 11, differences_total: 3,
    differences: [{ book: { entry: "2020-01-02", exit: "2020-01-05", entry_px: 1, exit_px: 2, dir: 1, legs: 1, open: false }, template: null }] };
  const host2 = await mount(<CustomizePanel {...base} selected="rsi2_4" template={templateFixture} loadFidelity={vi.fn().mockResolvedValue(diff)} />);
  await click(host2, '[data-lab-action="fidelity"]');
  await flush();
  expect(host2.textContent).toContain("Matched 10 of 12 book trades and 11 template trades");
  expect(host2.textContent).toContain("3 differences in all");
  await unmount();
  const refusal = Object.assign(new Error("Customize not available — the method is not admitted on NIFTY 50"), { status: 400, kind: "engine", engineStatus: "unsupported" });
  const host3 = await mount(<CustomizePanel {...base} selected="rsi2_4" template={templateFixture} loadFidelity={vi.fn().mockRejectedValue(refusal)} />);
  await click(host3, '[data-lab-action="fidelity"]');
  await flush();
  expect(host3.querySelector("[data-lab-error]")?.textContent).toContain("Customize not available — the method is not admitted on NIFTY 50");
});
```

Method buttons carry `data-lab-method={id}`.

- [ ] **Step 2: Run** — FAIL.
- [ ] **Step 3: Implement** `CustomizePanel.tsx` and `customize.css` (method list `max-height: 320px; overflow: auto;`; tokens only).
- [ ] **Step 4: Run** — PASS; own-file typecheck clean.
- [ ] **Step 5: Commit**

```bash
cd /home/aff34/.cache/strategy-vault-development; F=Context/guru/frontend/react; L=$F/src/features/lab
P="$L/CustomizePanel.tsx $L/customize.css $F/tests/lab-customize.test.tsx"
git add -- $P && git commit -m "feat(lab-ui): Customize picker, template summary and display-only fidelity" -- $P
```

---

### Task 14: YT Strategies page

**Files:**
- Modify (replace the Task 1 stub): `src/pages/YtStrategiesPage.tsx`
- Test: `tests/yt-strategies-page.test.tsx`

**Interfaces:**
- Consumes: `strategies.published` (Task 2); `PublishedStrategy` (Task 1); `COPY`, `labHash` (Task 1); `LabErrorBoundary`, `ErrorNotice`, `Loading` (Task 3). Does not import `spec-model.ts` (counts come straight from `spec.entry.all.length`).
- Produces: `YtStrategiesPage()` (no props), root `div.lab-page[data-lab-page="yt-strategies"]`.

Behaviour: `h1.st` "YT Strategies"; `.rnote` `COPY.ytIntro`; loads `strategies.published()`; loading → `Loading`; error → `ErrorNotice` (market `"in"`); empty → `.empty` `<b>{COPY.ytEmpty}</b>`; otherwise a grid of `article.box.lab-card` (newest `publishedAt` first): `h2` name, variant badge `span.tag` "Customized from {origin.book} · {origin.method}" when `origin`, meta "{ownerName ?? "Former member"} · {market === "in" ? "India" : "US"} · {instrument}", "{entry.all.length} entry / {exit.any.length} exit · published {publishedAt.slice(0, 10)}", and `a.dbtn.sm` "Open in lab" with `href = labHash("/indicators", {region: market, tk: instrument, published: String(snapshotId)}, "#/indicators")`. The whole page sits inside `LabErrorBoundary`.

- [ ] **Step 1: Write the failing test** — `tests/yt-strategies-page.test.tsx`

```tsx
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { YtStrategiesPage } from "../src/pages/YtStrategiesPage";
import { clearLabCache } from "../src/features/lab/client";
import { fetchRouter, flush, jsonResponse, mount, publishedFixture, unmount } from "./fixtures/lab";

beforeEach(clearLabCache);
afterEach(async () => { await unmount(); vi.unstubAllGlobals(); });

it("lists published strategies newest first with open-in-lab links and variant badges", async () => {
  vi.stubGlobal("fetch", fetchRouter([["/api/app/lab/strategies/published", () => jsonResponse(publishedFixture)]]));
  const host = await mount(<YtStrategiesPage />);
  await flush();
  const cards = [...host.querySelectorAll(".lab-card")];
  expect(cards.map((c) => c.querySelector("h2")!.textContent)).toEqual(["RSI 2 variant", "EMA pullback"]);
  expect(cards[0].textContent).toContain("Customized from ststw · rsi2_4");
  expect(cards[0].textContent).toContain("Former member");
  expect(cards[0].querySelector("a")!.getAttribute("href")).toBe("#/indicators?region=us&tk=SPY&published=12");
  expect(cards[1].querySelector("a")!.getAttribute("href")).toBe("#/indicators?tk=NIFTY+50&published=11");
  expect(host.textContent).toContain("not a book method and not a verification");
});

it("empty and error states", async () => {
  vi.stubGlobal("fetch", fetchRouter([["/api/app/lab/strategies/published", () => jsonResponse([])]]));
  const host = await mount(<YtStrategiesPage />);
  await flush();
  expect(host.textContent).toContain("Nothing is published yet.");
  await unmount();
  vi.stubGlobal("fetch", fetchRouter([["/api/app/lab/strategies/published", () => jsonResponse({ error: "Please sign in." }, 401)]]));
  const host2 = await mount(<YtStrategiesPage />);
  await flush();
  expect(host2.querySelector("[data-lab-error]")?.textContent).toContain("Please sign in.");
});
```

- [ ] **Step 2: Run** — FAIL (stub has no cards).
- [ ] **Step 3: Implement** the page (CSS from `lab.css`; add no new CSS file — `.lab-card` uses `.box`; if a grid is needed use inline `style={{ display: "grid", gap: 14, gridTemplateColumns: "repeat(auto-fill, minmax(min(100%, 320px), 1fr))" }}`).
- [ ] **Step 4: Run** — PASS; own-file typecheck clean.
- [ ] **Step 5: Commit**

```bash
cd /home/aff34/.cache/strategy-vault-development; F=Context/guru/frontend/react
P="$F/src/pages/YtStrategiesPage.tsx $F/tests/yt-strategies-page.test.tsx"
git add -- $P && git commit -m "feat(lab-ui): YT Strategies page for published lab strategies" -- $P
```

---

### Task 15: Indicators page assembly and page state

**Files:**
- Create: `src/features/lab/lab-state.ts`
- Modify (replace the Task 1 stub): `src/pages/IndicatorsPage.tsx`
- Test: `tests/lab-state.test.ts`, `tests/indicators-page.test.tsx`

**Interfaces:**
- Consumes: everything from Tasks 1–13 by the names given there; `getAccount` (`src/services/api.ts`); `vaultStore` (`src/state/store.ts`) for the region fallback.
- Produces: `IndicatorsPage()`; `lab-state.ts`: `LabState`, `LabAction`, `initialLabState(q: LabQuery, regionFallback?: string)`, `labReducer`, `runKey(spec, market, ticker)`, `editorState(state, savedMarket) → EditorState`.

- [ ] **Step 1: Write the failing state test** — `tests/lab-state.test.ts`

```ts
import { expect, it } from "vitest";
import { editorState, initialLabState, labReducer, runKey } from "../src/features/lab/lab-state";
import { defaultSpec, setAt } from "../src/features/lab/spec-model";
import { catalogueFixture as cat, strategyFixture } from "./fixtures/lab";
import { readLabQuery } from "../src/features/lab/url-state";

const ind = (id: string) => cat.indicators.find((i) => i.id === id)!;
const start = () => initialLabState(readLabQuery("#/indicators?tk=NIFTY%2050"));

it("starts from the query; a missing region falls back to the research region", () => {
  expect(start()).toMatchObject({ market: "in", ticker: "NIFTY 50", tf: "D", range: "3Y", chips: [], row: null, tried: 0 });
  expect(initialLabState(readLabQuery("#/indicators"), "us").market).toBe("us");
  expect(initialLabState(readLabQuery("#/indicators?region=gl")).market).toBe("gl");
});

it("caps price overlays at 4 and panes at 3, dropping the oldest", () => {
  let s = start();
  for (let i = 0; i < 5; i++) s = labReducer(s, { type: "chip-add", indicator: ind("ema") });
  for (let i = 0; i < 4; i++) s = labReducer(s, { type: "chip-add", indicator: ind("rsi") });
  expect(s.chips.filter((c) => c.id === "ema")).toHaveLength(4);
  expect(s.chips.filter((c) => c.id === "rsi")).toHaveLength(3);
  expect(s.chips[0].params).toEqual({ length: 9 });
  const key = s.chips[0].key;
  s = labReducer(s, { type: "chip-set", key, params: { length: 30 } });
  expect(s.chips.find((c) => c.key === key)!.params).toEqual({ length: 30 });
  expect(labReducer(s, { type: "chip-remove", key }).chips.some((c) => c.key === key)).toBe(false);
});

it("market change resets instrument, chips and the open row but keeps the rules", () => {
  let s = labReducer(start(), { type: "chip-add", indicator: ind("ema") });
  s = labReducer(s, { type: "open", row: strategyFixture(), spec: strategyFixture().spec });
  s = labReducer(s, { type: "spec", spec: setAt(s.spec, "exit.any[0].rhs.value", 9) });
  const next = labReducer(s, { type: "market", market: "us" });
  expect(next).toMatchObject({ market: "us", ticker: "", chips: [], row: null });
  expect(next.spec).toEqual(s.spec);
});

it("submit readiness: unchanged row, same instrument, fresh successful run", () => {
  const row = strategyFixture();
  let s = labReducer(start(), { type: "open", row, spec: row.spec });
  expect(editorState(s, "in")).toMatchObject({ unchanged: true, ranFresh: false });
  s = labReducer(s, { type: "ran", key: runKey(s.spec, s.market, s.ticker), combos: 0 });
  expect(editorState(s, "in")).toMatchObject({ unchanged: true, ranFresh: true });
  expect(s.tried).toBe(1);
  const edited = labReducer(s, { type: "spec", spec: setAt(s.spec, "exit.any[0].rhs.value", 6) });
  expect(editorState(edited, "in")).toMatchObject({ unchanged: false, ranFresh: false });
  const reordered = labReducer(s, { type: "row", row: { ...row, version: 4, spec: JSON.parse(JSON.stringify(row.spec, ["v", "exit", "any", "lhs", "s", "op", "rhs", "value", "entry", "all", "p", "length"])) } });
  expect(editorState(reordered, "in").unchanged).toBe(true);
  expect(labReducer(s, { type: "tried", n: 3 }).tried).toBe(4);
  expect(initialLabState(readLabQuery("#/indicators")).spec).toEqual(defaultSpec());
});
```

- [ ] **Step 2: Implement `src/features/lab/lab-state.ts`**

```ts
import { MAX_PANES, MAX_PRICE_OVERLAYS } from "./catalogue";
import { canonicalJson, defaultSpec } from "./spec-model";
import type { EditorState } from "./SavedStrategies";
import type { LabQuery } from "./url-state";
import type { Chip } from "./Workbench";
import type { ChartRange, Indicator, LabStrategy, Market, SavedMarket, Spec } from "./types";

export interface LabState {
  market: Market; ticker: string; tf: string; range: ChartRange; spec: Spec; chips: Chip[];
  placements: Record<string, "price" | "pane">; nextChip: number; row: LabStrategy | null;
  lastRun: string | null; tried: number;
}
export type LabAction =
  | { type: "market"; market: Market } | { type: "ticker"; ticker: string } | { type: "tf"; tf: string } | { type: "range"; range: ChartRange }
  | { type: "spec"; spec: Spec } | { type: "open"; row: LabStrategy | null; spec: Spec } | { type: "row"; row: LabStrategy }
  | { type: "chip-add"; indicator: Indicator } | { type: "chip-set"; key: string; params: Record<string, number> } | { type: "chip-remove"; key: string }
  | { type: "ran"; key: string; combos: number } | { type: "tried"; n: number };

export const runKey = (spec: Spec, market: SavedMarket, ticker: string) => canonicalJson([spec, market, ticker]);

/** The page passes regionFallback only when the hash has no `region` (then the research pages' region wins). */
export function initialLabState(q: LabQuery, regionFallback?: string): LabState {
  const market: Market = regionFallback === "us" ? "us" : q.region;
  return { market, ticker: q.tk, tf: q.tf, range: q.range, spec: defaultSpec(), chips: [], placements: {}, nextChip: 1, row: null, lastRun: null, tried: 0 };
}

export function labReducer(state: LabState, action: LabAction): LabState {
  switch (action.type) {
    case "market":
      return action.market === state.market ? state : { ...state, market: action.market, ticker: "", chips: [], placements: {}, row: null, lastRun: null };
    case "ticker": return { ...state, ticker: action.ticker, lastRun: null };
    case "tf": return { ...state, tf: action.tf };
    case "range": return { ...state, range: action.range };
    case "spec": return { ...state, spec: action.spec };
    case "open": return { ...state, row: action.row, spec: action.spec, lastRun: null };
    case "row": return { ...state, row: action.row };
    case "chip-add": {
      const { indicator } = action;
      const key = `c${state.nextChip}`;
      const params = Object.fromEntries(indicator.params.map((p) => [p.key, p.default]));
      let chips = [...state.chips, { key, id: indicator.id, params }];
      const placements = { ...state.placements, [key]: indicator.placement };
      const cap = indicator.placement === "price" ? MAX_PRICE_OVERLAYS : MAX_PANES;
      const same = chips.filter((c) => placements[c.key] === indicator.placement);
      if (same.length > cap) chips = chips.filter((c) => c.key !== same[0].key);
      return { ...state, chips, placements, nextChip: state.nextChip + 1 };
    }
    case "chip-set": return { ...state, chips: state.chips.map((c) => (c.key === action.key ? { ...c, params: action.params } : c)) };
    case "chip-remove": return { ...state, chips: state.chips.filter((c) => c.key !== action.key) };
    case "ran": return { ...state, lastRun: action.key, tried: state.tried + 1 };
    case "tried": return { ...state, tried: state.tried + action.n };
  }
}

export function editorState(state: LabState, market: SavedMarket): EditorState {
  const unchanged = !!state.row && state.row.instrument === state.ticker && canonicalJson(state.row.spec) === canonicalJson(state.spec);
  return { spec: state.spec, market, ticker: state.ticker, row: state.row, unchanged,
    ranFresh: state.lastRun === runKey(state.spec, market, state.ticker) };
}
```

(`ran` counts one variant per successful backtest; `tried` adds an optimization's `combos`. The test's `combos: 0` is ignored by `ran`.)

- [ ] **Step 3: Run** — `npx vitest run tests/lab-state.test.ts` → PASS.

- [ ] **Step 4: Write the failing page test** — `tests/indicators-page.test.tsx`

```tsx
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { act } from "react";
import { IndicatorsPage } from "../src/pages/IndicatorsPage";
import { clearLabCache } from "../src/features/lab/client";
import {
  catalogueFixture, fetchRouter, flush, glInstrumentsFixture, jsonResponse, mount, publishedFixture, runFixture,
  seriesFixture, templateFixture, templatesFixture, tickersFixture, unmount,
} from "./fixtures/lab";

const me = (role: string) => jsonResponse({ user: { id: "u1", name: "N", email: "n@x", role }, settings: { theme: "aqua" } });
const series = (url: string) => { const id = new URL(url, "http://x").searchParams.get("id")!; return jsonResponse(seriesFixture(id, [id], { [id]: [1, 2, 3, 4, 5, 6] })); };
let runAnswer: () => Response;
const routes = (role = "researcher") => fetchRouter([
  ["/api/app/me", () => me(role)],
  ["/api/app/lab/strategies/published", () => jsonResponse(publishedFixture)],
  ["/api/app/lab/strategies/review", () => jsonResponse([])],
  ["/api/app/lab/strategies", () => jsonResponse({ error: "Saved strategies need the auth service; this local dev proxy has none." }, 503)],
  [/\/api\/(in|us|gl)\/lab\/catalogue/, () => jsonResponse(catalogueFixture)],
  [/\/api\/(in|us)\/tickers/, () => jsonResponse(tickersFixture)],
  ["/api/gl/lab/instruments", () => jsonResponse(glInstrumentsFixture)],
  [/\/lab\/series/, (url) => series(url)],
  [/\/lab\/templates/, () => jsonResponse(templatesFixture)],
  [/\/lab\/template\?/, () => jsonResponse(templateFixture)],
  [/\/lab\/run/, () => runAnswer()],
]);
beforeEach(() => { clearLabCache(); runAnswer = () => jsonResponse(runFixture); });
afterEach(async () => { await unmount(); vi.unstubAllGlobals(); history.replaceState(null, "", "/"); });
const go = async (hash: string, role = "researcher") => {
  history.replaceState(null, "", `/${hash}`);
  const f = routes(role);
  vi.stubGlobal("fetch", f);
  const host = await mount(<IndicatorsPage />);
  await flush(); await flush();
  return { host, f };
};
const run = async (host: HTMLElement) => { await act(async () => host.querySelector<HTMLButtonElement>('[data-lab-action="run"]')!.click()); await flush(); };

it("renders the catalogue label, library and default instrument; saved-strategy errors stay local", async () => {
  const { host } = await go("#/indicators");
  expect(host.querySelector('[data-lab-page="indicators"]')).not.toBeNull();
  expect(host.querySelector("[data-server-label]")?.textContent).toBe(catalogueFixture.label);
  expect(host.querySelectorAll("[data-lab-item]")).toHaveLength(catalogueFixture.indicators.length);
  expect(host.querySelector<HTMLSelectElement>(".lab-toolbar select")!.value).toBe("NIFTY 50");
  expect(host.querySelector("[data-lab-saved] [data-lab-error]")?.textContent).toContain("auth service");
});

it("a failed run clears the previous result", async () => {
  const { host } = await go("#/indicators?tk=NIFTY%2050");
  await run(host);
  expect(host.querySelector("[data-lab-result]")).not.toBeNull();
  runAnswer = () => jsonResponse({ status: "needs_data", error: "NIFTY 50: invalid OHLC bar on 2001-03-02", label: catalogueFixture.label }, 400);
  await run(host);
  expect(host.querySelector("[data-lab-result]")).toBeNull();
  expect(host.textContent).toContain("NIFTY 50: invalid OHLC bar on 2001-03-02");
});

it("gl shows the provisional strip and hides Customize", async () => {
  const { host } = await go("#/indicators?region=gl");
  expect(host.querySelector("[data-lab-provisional]")?.textContent).toContain("Provisional · TradingView export CRYPTO:BTCUSD, daily");
  expect(host.textContent).toContain("Customize runs on India and US book methods only.");
});

it("viewers cannot optimize", async () => {
  const { host } = await go("#/indicators", "viewer");
  expect(host.textContent).toContain("Researcher access is required for optimization.");
  expect(host.querySelector('[data-lab-action="optimize"]')).toBeNull();
});

it("customize loads the template and runs with its origin", async () => {
  const { host, f } = await go("#/indicators?customize=rsi2_4&tk=NIFTY%2050");
  expect(host.textContent).toContain("Book template · declared adaptation into lab rules · not a verification");
  await run(host);
  const runCall = f.calls.find((c) => c.url.includes("/lab/run"))!;
  const spec = JSON.parse(new URL(runCall.url, "http://x").searchParams.get("spec")!);
  expect(spec.origin).toEqual({ book: "ststw", method: "rsi2_4", template_version: 1 });
});

it("a published snapshot opens as an unsaved draft on its market", async () => {
  const { host, f } = await go("#/indicators?region=us&tk=SPY&published=12");
  await run(host);
  const url = new URL(f.calls.find((c) => c.url.includes("/lab/run"))!.url, "http://x");
  expect(url.pathname).toBe("/api/us/lab/run");
  expect(JSON.parse(url.searchParams.get("spec")!).origin.method).toBe("rsi2_4");
  expect(host.querySelector('[data-lab-action="save"]')).toBeNull();
});
```

- [ ] **Step 5: Run** — `npx vitest run tests/indicators-page.test.tsx` → FAIL (stub).

- [ ] **Step 6: Implement `src/pages/IndicatorsPage.tsx`**

Structure (keep the page a wiring layer; no business logic beyond what is listed):

```tsx
import { useCallback, useEffect, useMemo, useReducer, useState } from "react";
import { getAccount } from "../services/api";
import { vaultStore } from "../state/store";
import * as lab from "../features/lab/client";
import { COPY } from "../features/lab/labels";
import { readLabQuery, writeLabQuery } from "../features/lab/url-state";
import { editorState, initialLabState, labReducer, runKey } from "../features/lab/lab-state";
import { specProblems, runQuery } from "../features/lab/spec-model";
import { availability } from "../features/lab/catalogue";
import { defaultTicker, optionsFromGl, optionsFromTickers, savedMarket, type InstrumentOption } from "../features/lab/instruments";
import { ErrorNotice, LabErrorBoundary, Loading, ServerLabel } from "../features/lab/ui";
import { Toolbar } from "../features/lab/Toolbar";
import { LibraryRail } from "../features/lab/LibraryRail";
import { IndicatorDetail } from "../features/lab/IndicatorDetail";
import { Workbench } from "../features/lab/Workbench";
import { RuleBuilder, type RunSettings } from "../features/lab/RuleBuilder";
import { RunResults } from "../features/lab/RunResults";
import { OptimizePanel } from "../features/lab/OptimizePanel";
import { CustomizePanel } from "../features/lab/CustomizePanel";
import { SavedStrategies } from "../features/lab/SavedStrategies";
import { ReviewQueue } from "../features/lab/ReviewQueue";
import type { Catalogue, Indicator, RunResult, Role, TemplateResponse, TemplatesResponse } from "../features/lab/types";

export function IndicatorsPage() {
  return <LabErrorBoundary><IndicatorsLab /></LabErrorBoundary>;
}

function IndicatorsLab() {
  const query = useMemo(() => readLabQuery(), []);
  const [state, dispatch] = useReducer(labReducer, undefined, () =>
    initialLabState(query, /[?&]region=/.test(location.hash) ? undefined : vaultStore.getSnapshot().state?.region));
  // role, catalogue, options, result, runError, running, settings, reading, templates, template, errors: local useState.
  ...
}
```

Wiring rules (each is covered by the tests above or by the component tests):
1. Role: `getAccount().then(a => setRole(a.user.role))`, errors ignored (the App shell handles sign-in); `role` stays `null` until then (OptimizePanel treats `null` like viewer, SavedStrategies hides save controls).
2. Market effect `[state.market]`: load `lab.catalogue(market)` and the options (`in`/`us` → `optionsFromTickers(await lab.tickers(m), allowedTypes)`; `gl` → `optionsFromGl(await lab.labInstruments(), allowedTypes)`), keep the gl `provisional_note` from the instruments response; if `state.ticker` is empty or not among the options, dispatch `ticker` with `defaultTicker`. Errors → page-level `ErrorNotice` in place of the work column (`Loading` while pending). Each async effect uses a `cancelled` flag in its cleanup.
3. `allowedTypes` = the available template's `instrument.allowed_types`, else `undefined`; options recompute when it changes.
4. URL sync effect: `writeLabQuery({region: state.market, tk: state.ticker, tf: state.tf, range: state.range})` whenever those change (`customize`, `strategy`, `published` are written by their own handlers: selecting a template writes `customize`, Clear writes `customize: ""`, opening a saved row writes `strategy: String(row.id)` and clears `published`/`customize`).
5. `?customize=<id>` (in/us only): `lab.templates(market)` for the list (always loaded on in/us, for the picker and `methodNames`), `lab.template(market, id)` for the selection; `available` → `dispatch({type: "open", row: null, spec: template.template.spec})`; errors → `templateError`.
6. `?strategy=<id>` → `lab.strategies.get(Number(id))` → `dispatch({type: "open", row, spec: row.spec})`, market from `row.market` (`crypto` → `gl`) and ticker from `row.instrument`; `?published=<snapshotId>` → `lab.strategies.published()` → find by `snapshotId` → `dispatch({type: "open", row: null, spec})` (market/ticker already in the query). Errors → `ErrorNotice` in the rules area.
7. Library: `onAdd(ind)` dispatches `chip-add` only when `availability(ind, {market, hasVolume: option.hasVolume, tf}).ok`; `onRead(ind)` toggles `IndicatorDetail`.
8. Workbench `loadSeries = q => lab.series(state.market, {...q, ticker: state.ticker})`; `trades = result?.trades`; `instrumentLabel` = selected option's `instrumentLabel`.
9. Problems: `specProblems(state.spec, catalogue)`.
10. Run: `setResult(null); setRunError(null); setRunning(true)`; `lab.run(state.market, state.spec, {ticker, from: settings.from || undefined, to: settings.to || undefined, ...runQuery(state.spec, settings.fees, settings.slippage)})` → `setResult(r)` and `dispatch({type: "ran", key: runKey(state.spec, savedMarketValue, state.ticker), combos: 0})`; catch → `setRunError(e)`. (`savedMarketValue = savedMarket(state.market, selectedOption)`.)
11. Results area: `runError` → `ErrorNotice` (with gl strip); `result` → `RunResults`; neither → nothing.
12. OptimizePanel: `dates = result?.dates ?? []`; `onOptimize = (spec, split) => lab.optimize(state.market, spec, {ticker, from, to, ...runQuery(...), split})`; `onApply = spec => { dispatch({type: "spec", spec}); then run with that spec }` (run accepts an optional spec argument); `onTried = n => dispatch({type: "tried", n})`; `tried = state.tried`.
13. Customize (always rendered; the panel itself shows the gl message): `onSelect(id)` → `writeLabQuery({customize: id})` and load; `onClear` → `writeLabQuery({customize: ""})`, clear template state, keep the rules; `loadFidelity = (id, tk) => lab.fidelity(state.market, id, tk, settings.from || undefined, settings.to || undefined)`.
14. Saved: `<SavedStrategies role api={lab.strategies} editor={editorState(state, savedMarketValue)} methodNames={Object.fromEntries(templates?.templates.map(t => [t.method.id, t.method.name]) ?? [])} onOpen={row => openRow(row)} onSaved={row => dispatch({type: "row", row})} />`; `role === "admin"` adds `<ReviewQueue api={lab.strategies} onOpen={openRow} />`.
15. Layout: `div.lab-page[data-lab-page="indicators"]` > header (`h1.st` "Indicators", `p` `COPY.lead`, `ServerLabel(catalogue.label)`) > `Toolbar` > `div.lab-grid` (`LibraryRail` | `div.lab-main`: optional `IndicatorDetail`, `Workbench`, `div.two` [`RuleBuilder` | results], `OptimizePanel`, `CustomizePanel`, `SavedStrategies`, `ReviewQueue`). Each major box is wrapped in its own `LabErrorBoundary` so one failing part never blanks the page.

- [ ] **Step 7: Run** — `npx vitest run tests/lab-state.test.ts tests/indicators-page.test.tsx` → PASS; then the whole suite `npm test` and `npm run typecheck` (all tasks are in by now) → PASS.

- [ ] **Step 8: Commit**

```bash
cd /home/aff34/.cache/strategy-vault-development; F=Context/guru/frontend/react
P="$F/src/features/lab/lab-state.ts $F/src/pages/IndicatorsPage.tsx $F/tests/lab-state.test.ts $F/tests/indicators-page.test.tsx"
git add -- $P && git commit -m "feat(lab-ui): Indicators page wiring and page state" -- $P
```

---

### Task 16: Gate — checks, real-browser run, guards, scope proof

**Files:** none created in the repository (screenshots and logs go to the session scratchpad `$SCRATCH`). Fixes found here go back to the owning task's files, committed with that task's file list.

- [ ] **Step 1: Static checks** (from `Context/guru/frontend/react`)

Run: `npm run typecheck && npm test && npm run test:coverage && npm run build`
Expected: all pass; coverage ≥ 80% lines/functions/branches/statements over the include list (the lab files included). A coverage miss is fixed by adding tests in the owning task's test file.

- [ ] **Step 2: Start the India engine on NAS data (read-only), the dev proxy and Vite**

```bash
NAS="/run/user/1000/gvfs/smb-share:server=192.168.4.50,share=share/Manan Pujara/Book-to-rust"
REPO=/home/aff34/.cache/strategy-vault-development
ls -l --time-style=+%F_%T "$REPO/Context/guru/backend/target/release/guru"   # record the binary date; do not rebuild while Phase 7 is in flight
"$REPO/Context/guru/backend/target/release/guru" "$NAS/idx_data.csv" 18090 "$NAS/india_vix" "$NAS/extra_data/india"
```
(run in the background; wait until `curl -s http://127.0.0.1:18090/api/lab/catalogue | head -c 80` answers). Then, in `Context/guru/frontend/react`, in the background:
```bash
LAB_ENGINE_IN=http://127.0.0.1:18090 node scripts/lab-dev-proxy.mjs
VAULT_DEV_PROXY=http://127.0.0.1:8188 npm run dev -- --port 5174 --strictPort
```
Expected: `curl -s http://127.0.0.1:5174/api/in/lab/catalogue | head -c 80` returns the catalogue JSON with the lab label.

- [ ] **Step 3: Browser check**

Run: `node scripts/lab-browser-check.mjs http://127.0.0.1:5174 "$SCRATCH/lab-browser"`
Expected: exit 0; JSON report with every check `true` and no page errors for desktop-steel, desktop-aqua, desktop-midnight, mobile-aqua (400px) and yt-strategies (which shows the dev proxy's 503 as an error notice — that counts as rendered). Open each PNG with the Read tool and check: theme colours applied (no white boxes in Midnight), library/chart/rules/results visible, mobile single column, labels verbatim. Record the run's KPIs once and compare a single number (e.g. trades) against `curl` of the same `/api/in/lab/run` query through the proxy — the UI must show what the engine returned.

- [ ] **Step 4: Viewer and gl spot checks**

`LAB_DEV_ROLE=viewer` restart of the proxy → rerun the browser check's desktop view manually (or `node scripts/lab-browser-check.mjs …` and expect only `optimize` to fail) and confirm "Researcher access is required for optimization." in the screenshot. gl has no local data on the NAS: confirm `#/indicators?region=gl` shows the dev proxy's 503 message ("No local gl engine is configured (LAB_ENGINE_GL).") inside an error notice with the provisional strip, and no numbers.

- [ ] **Step 5: Stop the processes** (engine, proxy, Vite) and delete nothing else.

- [ ] **Step 6: Guards** (from the repo root)

```bash
python3 scripts/check_book_untouched.py --base origin/Manan
python3 scripts/check_upload.py
```
Expected: both exit 0 (`custom-pages.ts` changes are additions only; no protected path).

- [ ] **Step 7: Scope proof**

```bash
cd /home/aff34/.cache/strategy-vault-development
for c in $(git log --format=%H "$LABUI_BASE"..HEAD --grep='(lab-ui)'); do git show --name-only --format= "$c"; done | sort -u \
  | grep -Ev '^(Context/guru/frontend/react/|docs/)' && echo "OUT OF SCOPE" || echo "scope ok"
git log --format=%H "$LABUI_BASE"..HEAD --invert-grep --grep='(lab-ui)' | while read c; do git show --name-only --format= "$c"; done \
  | grep -E '^Context/guru/frontend/react/' && echo "FRONTEND TOUCHED OUTSIDE lab-ui" || echo "no foreign frontend edits"
git diff --name-only "$LABUI_BASE"..HEAD -- Context/guru/frontend/react/src/compat Context/guru/frontend/react/src/pages/ResearchPages.tsx
```
Expected: `scope ok`, `no foreign frontend edits`, and an empty last command.

- [ ] **Step 8: Report** — list commits (`git log --oneline "$LABUI_BASE"..HEAD --grep='(lab-ui)'`), test counts before/after, coverage numbers, the browser report JSON, screenshot paths, the engine binary date, and the open items for the user (assumptions in spec §9; DB-backed saved-strategy flow and review queue are the user's manual gate on an isolated `strategy-vault-dev-<name>` stack with the Vite origin in `VAULT_ALLOWED_ORIGINS`). Do not push.

---

## Waves (parallel execution)

| Wave | Tasks | Notes |
|---|---|---|
| 1 | Task 1, Task 6 | Task 6 shares no file or interface with Task 1 |
| 2 | Tasks 2, 3, 4, 5 | all depend on Task 1 only |
| 3 | Tasks 7, 8, 9, 10, 11, 12, 13, 14 | disjoint files; interfaces from waves 1–2 |
| 4 | Task 15 | imports every component |
| 5 | Task 16 | gate |

Dependencies: T2←T1; T3←T1; T4←T1; T5←T1; T6←∅; T7←T1,T3; T8←T1,T2,T3,T5; T9←T1,T3,T4; T10←T1,T3,T4,T5; T11←T1,T2,T3,T4; T12←T1,T2,T3; T13←T1,T2,T3; T14←T1,T2,T3; T15←T1–T14; T16←all.

## Self-review notes

- Spec coverage: screens §3.1 (T7, T8, T9, T15), §3.2 (T10), §3.3 (T10, T12, T14), §3.4 (T11), §3.5 (T13, T15), §3.6 (T12), §3.7 (T14); components §4 (file table); data flow §5 (T2, T15); honesty §6 (T3, T4, T10–T15 tests incl. no-"verified" checks); testing §7 (every task + T16); waves §8 (table above).
- Phase 7: operators/timeframes/price sources from the catalogue (T9 test with `Q`), `SPECIAL_OPERANDS`/`PRICE_ORDERS` registries (T4), unknown keys kept (T4, T9), unknown operands read-only (T9), Q/Y aggregation (T5).
- Names used across tasks: `LabError`, `strategies`, `StrategiesApi` (T2); `ErrorNotice`, `LabErrorBoundary`, `Segmented`, `ServerLabel`, `ProvisionalStrip`, `MetricTiles`, `isProvisional`, `pct/num/int` (T3); spec-model names (T4); `createLabChart`, `useThemeKey`, `PALETTE`, bars helpers (T5); `availability`, `MAX_*`, instrument helpers (T7); `Chip` (T8); `RunSettings` (T9); `EditorState` (T12) — each defined once in its owning task.
