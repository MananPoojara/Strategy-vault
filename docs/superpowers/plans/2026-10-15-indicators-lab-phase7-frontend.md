# Indicators Lab Phase 7 "Rules II" — Frontend Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking. Tasks inside one wave own disjoint files and run in parallel.

**Goal:** Give the Indicators page editors and result views for everything Phase 7 adds to the lab rule spec: a rule tree (groups, not, ranges, trends, at-least-k-of-m, sequences), expression and state operands, higher timeframes with AmiBroker expand modes, calendar operands, two-sided (long/short/both) strategies, entry orders and the new exits, sizing / re-entry / risk / money mode, the per-instrument execution model, scale-in legs and scale-out fills, the `adaptations` note, and optimization over every new path. Nothing on the backend, the book engine or protected frontend files changes.

**Architecture:** Three new pure modules hold the logic, so components stay thin and testable alone:
- `rule-tree.ts` (Task 2): node and operand kinds, value-based tree edits, one walker that follows the engine's optimize-path grammar, the engine's comparison/node budget, location labels and tree pre-checks;
- `execution-model.ts` (Task 3): direction and sides, entry orders, level distances, exits, scale-in, sizing, money mode, costs, re-entry and risk fields, instrument context and pre-run instrument notes, execution pre-checks;
- `optimize-paths.ts` (Task 10): optimizable settings built on the walker.

Components: `OperandEditor.tsx` v2 (Task 5), new `NodeEditor.tsx` (Task 6), `OrdersPanel.tsx` v2 with `ExecutionPanel` (Task 7), new `RiskPanel.tsx` (Task 8), `RunResults.tsx` (Task 9), `OptimizePanel.tsx` (Task 10). `RuleBuilder.tsx` and `IndicatorsPage.tsx` assemble them (Task 11). The engine stays the authority: every client check is a UX pre-check, and server messages are shown as given.

**Tech Stack:** React 19.3, TypeScript 7 (strict), Vite 8, Vitest 5 + jsdom (act/createRoot, `tests/fixtures/lab.ts` helpers), vendored Lightweight Charts 4.2.0, headless `/usr/bin/google-chrome` over DevTools for `scripts/lab-browser-check.mjs`. No new npm dependency.

**Specs and contract:**
- Backend spec `docs/superpowers/specs/2026-10-15-indicators-lab-phase7-rules-ii-design.md` (§4 grammar, §5 timeframes/calendar, §6 orders, §7 instruments, §8 optimize paths, §9 API, §13 assumptions).
- Backend plan `docs/superpowers/plans/2026-10-15-indicators-lab-phase7-rules-ii.md` (Tasks 10–15 not built yet; their JSON is taken from the plan's patches, quoted in "Contract" below).
- Backend code as it is today: `Context/guru/backend/src/lab/{rules.rs,series.rs,simulate.rs,api.rs,resample.rs,periods.rs,catalogue.rs}` (batches 1–2 committed, batch 3 = Tasks 7–9 finishing in the working tree).
- Frontend spec/plan `docs/superpowers/{specs,plans}/2026-10-15-indicators-lab-frontend*.md` (conventions, labels, honesty rules).

All paths are relative to `Context/guru/frontend/react/` unless they start with `Context/`, `docs/` or `scripts/check_`. Repository root: `/home/aff34/.cache/strategy-vault-development` (branch `Nandini`).

---

## Decisions and assumptions (made without the user; each is cheap to reverse)

1. **Value-based tree editing.** A node editor receives a node and `onChange(node)`; a list editor receives a list and `onChange(list)`. Spec paths are used only for the budget, labels, pre-checks and optimize paths. *Why:* no path bookkeeping in components; recursion is trivial. *Cost if wrong:* none visible.
2. **The operator select is the leaf-type switch.** "is between / is outside" turn a comparison into a range node, "is rising for / is falling for" into a trend node, and back. The "+ Add…" menu only offers Condition, All of, Any of, Not, At least K of M, Sequence. *Why:* one control instead of three add buttons.
3. **Progressive disclosure:** one "+ Add…" select per list, one "⋯" select per node (wrap / negate / unwrap / move), operand timeframe/expand/offset behind a "⋯" timing toggle that opens by itself (and shows a summary label) when not default, exits added from an "+ Add exit…" select (only set exits are shown), execution timing options behind a toggle, sizing/re-entry/risk/money in one collapsed `<details>`. Groups are open by default; each group can collapse to a one-line summary.
4. **Formula switching keeps the inner value.** Choosing a formula kind on a plain operand wraps it (`close` → `close, 1 bar ago`); switching between formula kinds keeps the inner operand (`ref(close)` → `avg(close, 10)`); a "Remove formula" button unwraps. Unary/window/state formulas over a number start on `close` (the engine refuses them over a number).
5. **Calendar kind.** A "Calendar" operand kind lists `calendar`, `session` and `sessions_left` outputs in one select with plain labels; `sessions_left` options say "future dates". `expiry` is not offered (futures only; `needs_data`).
6. **Two-sided layout.** With Both, the rules box shows a "Long side" panel and a "Short side" panel (each: entry list, exit list, entry order, exits). Execution settings (fills, opposite, delay, edge, intrabar) are shared and shown once. Switching away from Both keeps the short side in memory for this session, so switching back restores it. A new short side starts as `close < EMA(50)` / `bars held >= 5` (never a mirrored copy: mirroring thresholds is not well defined).
7. **Distances use a unit select** (% / ATR / Points / At) instead of segmented buttons: fewer pixels per exit. `ticks` is never offered (futures only, and futures data is not admitted); a spec that has ticks is flagged, not rewritten.
8. **Futures:** no editor. A cap line in the risk panel says the contract model exists but no contract data is admitted, so futures runs answer "Data needed". A spec carrying `execution.instrument` is kept as is and blocked before sending with the same note (the engine would refuse it anyway).
9. **Money mode gating before the run** (pre-check only, the server decides): an index, crypto or CFD instrument, or an India instrument with shorts, disables "Money" with the reason shown under the control. US shorts in money mode require a borrow % field. Money mode removes `execution.cost_bps`, hides the Fees/Slippage boxes and never sends the `cost`/`slippage` query (the engine refuses both).
10. **Feature gating on the catalogue.** The sizing / re-entry / risk / money panel and the scale-in editor appear only when `/lab/catalogue` has the `execution` key (backend Task 12). Limits come from the catalogue with spec fallbacks (40 comparisons, 120 parts, 12 top-level per side, 16,384 query bytes). *Why:* the frontend can merge before or after backend batches 4–5 without offering fields the engine would refuse as unknown.
11. **Pre-run instrument notes mirror `model.rs` verbatim** (index study, India short hypothetical, US short borrow, crypto, CFD) as cap lines just above the run row; after a run the response's `execution_model.notes` replace them. Notes, events and `adaptations` are always shown as given.
12. **Ledger join:** in money mode the tradesheet gains Qty / Value / Costs / P&L columns only when `ledger.length === trades.length` and every row's `entry` date matches its trade; otherwise the ledger is shown as its own table. *Why:* the backend plan does not state whether money-mode trade prices are fills or effective prices, or whether skipped trades are omitted; the join never mislabels money.
13. **No empty columns:** Legs, Scale-outs and money columns appear only when some trade has them. Legs and scale-outs are a `<details>` inside the cell (count in the summary, dates/prices/weights inside).
14. **Optimize settings are grouped** (Entry, Exit or Long/Short entry/exit, Entry order, Exits, Position size) with a group header row; labels come from `pathLabel` ("Entry 3 › any 1 › right › number"). Take-profit sizes are not optimizable (backend decision 6).
15. **Error contract unchanged:** no numbers next to an error; `needs_data` / `unsupported` / `invalid` shown as given; nothing says "verified".
16. **Caps shown live:** one line in the rules header: `Top-level 3/12 · short 2/12 · comparisons 7/40 · rule parts 12/120 · 2.1 of 7.5 KB`; it turns red at ≥ 90%; add/wrap/formula controls are disabled at a cap; the 16-level JSON nesting limit is pre-checked.
17. **Always-in specs** (Both + Reverse + empty exits) pass the client pre-check once backend Task 15 lands; until then the engine answers with its own message.
18. **Out of scope / skipped:** a raw JSON editor, futures roll/adjust editors, ticks, a YT Strategies change for two-sided cards (it still shows the long counts), new templates, intraday.

---

## Global Constraints

- Branch `Nandini`; never `main`; never push; no force-push; no worktrees. **No git write without the user's explicit permission in the session** (no add/commit/stash/reset/checkout/clean). Each task's last step commits only with that permission: `git add -- <task paths> && git commit -m "(lab-ui) <subject>" -- <task paths>`; subjects use the `(lab-ui)` scope; **no `Co-Authored-By` trailer**. Without permission: stop, leave the changes in the working tree, report `git status --short -- <task paths>` and `git diff --stat -- <task paths>`.
- The working tree holds uncommitted lab-ui fixes (CSS, `LibraryRail`, `OptimizePanel`, `OrdersPanel`, `RuleBuilder`, `IndicatorsPage`, tests, `scripts/lab-browser-check.mjs`) and uncommitted backend batch-3 work (`rules.rs`, `series.rs`, `simulate.rs`). Both are the starting point. Never revert, stash or "clean" them; never edit any file under `Context/guru/backend`, `Context/guru/webapp`, `src/compat/*`, `src/pages/ResearchPages.tsx`, `src/services/*`, `src/state/*`, `src/app/*`, `src/env.d.ts`.
- **Only touch the files your task lists.** If you need something from a later task, code against the interface written here; never create or edit that task's file.
- Server labels verbatim (from `labels.ts`). Keep verified source rules, declared adaptations (`adaptations`, `sessions_left`), simulated results and book figures separate. Never write "verified" or "TradingView-matched".
- Theme: house classes and tokens only (`var(--card) --panel --thead --ink --ink2 --ink3 --rule --steel --red --green --r-field --font`); never `#fff`/`#000`. Section rhythm: one 14 px step between sections (`.lab-page`/`.lab-main` grid gap); inside the rules box sub-sections are ≤ 14 px apart, side-panel rows ≤ 10 px; controls ≤ 32 px tall; nested group indent ≤ 16 px (8 px at ≤ 600 px). Works at 400 px with no horizontal page scroll. Library groups stay collapsed by default; rules stay full width before a run.
- Tests: while other agents work, run only your files (`npx vitest run tests/<file>`); typecheck with `npx tsc --noEmit` and fix every error in your own files (the project must still typecheck after each task). In wave 3, `tests/lab-rule-builder.test.tsx` and `tests/indicators-page.test.tsx` belong to Task 11 and may fail until wave 4.
- Running servers belong to the user: Vite 5173, dev proxy 8188, engines 18090 (in) and 18091 (gl). Never restart, kill or reconfigure them. Per-task browser checks may *read* through Vite 5173 if it is up; otherwise start a private stack (Task 12 Step 2 commands, ports 5175/8189/18092) and stop only that. Screenshots and logs go to the session scratchpad (`$SCRATCH`), never into the repository.
- `CARGO_BUILD_JOBS=2`; no whole-corpus backtest; NAS read-only; no data, `.env` or results in Git; no Docker.

## Pre-flight (once, before wave 1)

- `git -C /home/aff34/.cache/strategy-vault-development branch --show-current` → `Nandini`. Record `LABUI7_BASE=$(git rev-parse HEAD)` in the task log.
- From `Context/guru/frontend/react`: `npx tsc --noEmit` and `npx vitest run` — record the pass counts as the baseline. If anything fails at baseline, report it and do not start.

## Review Focus

1. **An existing spec changes meaning through the editor** (v1 spec, Customize variant, saved strategy): unknown keys, `origin` and unknown node/operand shapes must survive every edit. Tests: `nested edits keep origin and unknown keys` (Task 11), `unknown nodes are kept read-only` (Task 6), `edits keep origin and unknown keys` (Task 1).
2. **A lookahead combination offered as valid** (expand first at offset 0 on a non-open series, compress first-expand without mode first): disabled with the server's reason. Tests: `expand first is offered only where the engine accepts it` (Task 5), `timing problems mirror rules.rs` (Task 2).
3. **A number that claims something cannot be executed** (money mode on an index, India short money, US short without borrow, futures): blocked before sending with the reason; after a run the server's `execution_model` notes are always visible. Tests: `money mode is blocked with the reason` (Task 8), `execution model notes, events and adaptations are shown as given` (Task 9), `money runs send no cost or slippage query` (Task 11).
4. **Budget drift from the engine** (client says 39 comparisons, engine says 41): one walker mirrors `rules.rs` `Budget::take`. Test: `budget counts like the engine` (Task 2).
5. **Optimize writes an impossible or wrong path** (nested node, short side, `.bars` of a window): paths are JSON paths identical to `rules.rs` `walk`. Tests: `walker yields the engine's optimize paths in engine order` (Task 2), `optimizable paths reach every Phase 7 number` (Task 10).

---

## Contract consumed (JSON the frontend reads or writes)

Rule spec (`"v": 1`, additive; `rules.rs` today plus backend plan Tasks 10–15):

| Area | JSON |
|---|---|
| Nodes | `{lhs,op,rhs}` op ∈ `crosses_above crosses_below crosses > < >= <= = !=`; `{lhs,op:"between"\|"outside",lo,hi}`; `{lhs,op:"rising"\|"falling",bars}`; `{all:[…]}`; `{any:[…]}`; `{not:node}`; `{at_least:k,of:m,when:node}`; `{then:[2..4 nodes],within:N}` |
| Operands | `{value}`; `{s,p?,tf?,out?,offset?,expand?}` (tf `D W M Q Y nD` 2–63; expand `last first point`, not on D; `first` only with offset ≥ 1 or `s:"open"`); `{add\|sub\|mul\|div\|min\|max\|pct:[A,B]}`; `{abs:A}`; `{ref:A,bars:0..1000}`; `{sum\|avg\|highest\|lowest\|change:A,bars:1..1000}`; `{bars_since:node}`; `{value_when:node,of:A,nth:1..20}`; `{highest_since\|lowest_since:node,of:A}`; `{count:node,bars}`; `{compress:A,tf:(not D),mode:last\|first\|high\|low\|sum,offset?,expand?}` (`first` expand only with offset ≥ 1 or mode first); `bars_held` only in a top-level exit comparison |
| Limits | 12 top-level items per side (entry+exit), 40 leaf comparisons and 120 comparisons+groups+expressions in the whole spec (both sides and `at` levels), 8,192 bytes, JSON nesting 16 |
| Orders (per side) | `entry.limit`, `entry.stop` = D or `{at:operand}`; both = stop-limit; `entry.valid_bars` 1..20 (orders only); `entry.scale_in {legs[2..5], min_bars, price any\|better\|worse}` (Task 10; not with take-profits) |
| Exits (per side) | `stop_loss`, `target`, `trailing {D…, activate?: D}`, `breakeven {after: D, plus?: D}` (plus < after in the same units; no `at`), `take_profits [{D…, size}]` ≤ 3, sizes in (0,1) summing < 1, `min_bars` 0..1000; exit list may be empty when a price exit exists, or (Task 15) Both + Reverse |
| D | `{pct}` · `{atr, atr_length?}` · `{points}` · (`{ticks}` futures only) |
| Execution (shared) | `fill`, `exit_fill` next_open\|close (no `fill` with an order on either side); `opposite` reverse\|exit\|ignore (Both only); `delay_bars` 1..10 (next-open fills only); `entry_on` level\|edge; `intrabar` stop_first\|ohlc_path; `sizing` one of `pct_equity` (0,100] / `risk_pct` (needs a stop-loss on every side) / `units` / `value` (money only) (Task 10); `cooldown {bars, after any\|loss\|win}`, `max_trades {count, per W\|M\|Q\|Y}`, `risk {max_consecutive_losses + pause_bars, max_drawdown_pct (0,100)}` (Task 11); `capital`, `costs {commission{per_order,per_unit,pct 0..10}, slippage{bps}, taxes[≤6 {name,pct,side buy\|sell\|both}], borrow_pct_year}` (Task 13; refuses `cost_bps` and the cost/slippage query); `instrument {kind:"futures",…}` (Task 14, always `needs_data`) |
| Direction | `direction: long\|short\|both`; `short_side {entry, exit}` with Both only |

Responses:

| Route | Added keys |
|---|---|
| `GET lab/catalogue` | today: `operators` (13), `timeframes [D,W,M,Q,Y]`, `custom_days [2,63]`, `expand_modes`; Task 12: `max_leaves`, `max_nodes`, `node_kinds`, `expressions`, `max_query_bytes`, `execution {direction, fill, opposite, entry_on, intrabar, sizing, distances, max_take_profits, max_legs, max_valid_bars, max_delay_bars}` |
| `GET lab/run` | Task 12: `execution_model {kind, mode:"unit"\|"money", executable, notes[], events[]}`; trades gain `legs` / `fills` = `[[date, price, weight], …]` only when more than one; `entry_kind` ∈ next_open close limit stop stop_limit reverse roll. Task 13: `money {capital, final_equity, costs_total, skipped, currency, basis}`, `ledger [{entry, qty, notional, costs, pnl}]`. Today: `adaptations [text]` only when the spec reads `sessions_left` |
| `GET lab/optimize` | `execution_model` (Task 12), `adaptations` (today) |
| `GET lab/series` | `adaptations` for `sessions_left` (today) |
| Trade reasons | `exit.stop_loss`, `exit.target`, `exit.trailing`, `exit.breakeven` (+ ` (gap at the open)`), `exit.any[k]: …`, `short_side.exit.any[k]: …`, `reverse to long\|short`, `opposite entry`, `open`; futures (fixtures only): `expiry`, `margin_call`, `roll` |

## File structure and ownership

```
src/features/lab/types.ts            Phase 7 spec + response types                              (Task 1)
src/features/lab/labels.ts           new copy, server-mirrored notes, calendar/operand labels    (Task 1)
src/features/lab/spec-model.ts       isCond, removeAt/putAt, runQuery, reasons, slim checks     (Task 1) → dead-code cleanup (Task 11)
tests/fixtures/lab.ts                catalogue7Fixture, run7Fixture, money7Fixture              (Task 1)
vite.config.ts                       coverage include list                                      (Task 1)
scripts/lab-browser-check.mjs        phase7 scenario, spacing/compactness/theme checks          (Task 4)
src/features/lab/rule-tree.ts        NEW tree model, walker, budget, labels, tree checks        (Task 2)
src/features/lab/execution-model.ts  NEW sides, orders, exits, sizing/money/risk, instrument    (Task 3)
src/features/lab/RunResults.tsx, results.css                                                    (Task 9)
src/features/lab/OperandEditor.tsx, operand.css (NEW)                                           (Task 5)
src/features/lab/NodeEditor.tsx (NEW), tree.css (NEW)                                           (Task 6)
src/features/lab/OrdersPanel.tsx, orders.css (NEW)                                              (Task 7)
src/features/lab/RiskPanel.tsx (NEW), risk.css (NEW)                                            (Task 8)
src/features/lab/optimize-paths.ts (NEW), OptimizePanel.tsx, optimize.css                       (Task 10)
src/features/lab/RuleBuilder.tsx, rules.css, Workbench.tsx, src/pages/IndicatorsPage.tsx        (Task 11)
tests: lab-spec-model (T1, T11), lab-rule-tree (T2), lab-execution-model (T3), lab-results (T9), lab-operand-editor (T5),
       lab-node-editor (T6), lab-orders (T7), lab-risk (T8), lab-optimize-paths + lab-optimize (T10),
       lab-rule-builder + indicators-page + lab-workbench (T11)
```

## Selector contract (tests, browser check and components agree on these)

| Selector | Owner | Meaning |
|---|---|---|
| `[data-lab-caps]` (+ class `lab-caps-warn` at ≥ 90%) | T11 | live caps line |
| `[data-lab-side="long"\|"short"]` | T11 | side panel |
| `[data-lab-list="entry.all"\|"exit.any"\|"short_side.entry.all"\|"short_side.exit.any"]` | T6 | rule list root |
| `[data-lab-node][data-lab-kind=cmp\|range\|trend\|all\|any\|not\|at_least\|then\|unknown]` | T6 | node row / group |
| `select[data-lab-add]` values `cond all any not at_least then` | T6 | add menu |
| `select[data-lab-node-menu]` values `wrap_all wrap_any wrap_not unwrap up down` | T6 | node actions |
| `.lab-group-body` | T6 | indented children |
| `.lab-operand[data-lab-operand-kind=<OperandKind>]`, `select[aria-label="Operand type"]`, `button[data-lab-timing]`, `select[aria-label="Timeframe"]`, `select[aria-label="Use the period value"]`, `select[aria-label="Calendar value"]` | T5 | operand editor |
| `[role=group][aria-label="Direction"]`, `[role=group][aria-label="When the other side signals"]`, `button[data-lab-timing-options]` | T7 | execution panel |
| `[data-lab-orders="long"\|"short"]`, `[role=group][aria-label="Long entry order"\|"Short entry order"]`, `select[aria-label="Add exit (long)"\|"Add exit (short)"]`, `[data-lab-exit=stop_loss\|target\|trailing\|breakeven\|take_profits\|min_bars]` | T7 | orders panel |
| `details[data-lab-risk]`, `[role=group][aria-label="Results in"]`, `[data-lab-money-blocked]` | T8 | risk panel |
| `[data-lab-execution-notes]`, `[data-lab-events]`, `[data-lab-adaptations]`, `[data-lab-money]`, `table[data-lab-trades]`, `table[data-lab-ledger]` | T9 | results |
| `tr[data-lab-dim=<path>]`, `tr.lab-dim-group` | T10 | optimize table |

## Waves and dependencies

| Wave | Tasks (parallel, disjoint files) | Needs |
|---|---|---|
| 1 | Task 1 (foundation), Task 4 (browser check script) | — |
| 2 | Task 2 (rule-tree), Task 3 (execution-model), Task 9 (results) | Task 1 |
| 3 | Task 5 (operand), Task 6 (tree editor), Task 7 (execution + orders), Task 8 (risk), Task 10 (optimize) | T1 + T2 (T5, T6, T10), T1 + T3 (T7, T8, T10) |
| 4 | Task 11 (assembly) | Tasks 1–10 |
| 5 | Task 12 (gate) | everything + backend Tasks 10–16 |

Dependencies: T2←T1; T3←T1; T4←∅; T9←T1; T5←T1,T2; T6←T1,T2 (renders today's `OperandEditor`; the `renderNode` prop arrives with T5); T7←T1,T3; T8←T1,T3; T10←T1,T2,T3; T11←T1–T10; T12←all.

**Backend dependency** (unit tests never need the engine; these need it for *live* browser verification, and T12 needs all of it):

| Task | Backend tasks that must have landed for the live check |
|---|---|
| T1, T2, T3, T4, T5, T6 | none beyond batches 1–2 (T3 encodes the Tasks 10–15 contract but runs without them) |
| T7 | Tasks 7–9 (batch 3, finishing); scale-in editor: Task 10 |
| T8 | Tasks 10, 11, 12 (catalogue `execution` gate), 13 (money), 14 (futures note wording) |
| T9 | Task 12 (`execution_model`, `legs`, `fills`), Task 13 (`money`, `ledger`) |
| T10 | Task 10 (sizing and scale-in paths), Task 12 (`execution_model` on optimize) |
| T11 | Task 12 (catalogue limits/`execution`), Task 13 (money query refusal), Task 15 (always-in empty exits) |
| T12 | Tasks 10–15 and the backend gate Task 16 (release binary built from that tree) |

---

### Task 1: Foundation — Phase 7 types, labels, fixtures, spec-model updates, coverage list

**Files:** Modify `src/features/lab/types.ts`, `src/features/lab/labels.ts`, `src/features/lab/spec-model.ts`, `src/features/lab/RuleBuilder.tsx` (type fallout only), `tests/fixtures/lab.ts`, `tests/lab-spec-model.test.ts`, `vite.config.ts`.
**Depends on:** nothing. **Backend:** contract only.
**Produces:** the types below; `isCond`, `removeAt`, `putAt`, `SPEC_BYTE_BUDGET` in spec-model; copy constants; `catalogue7Fixture`, `run7Fixture`, `money7Fixture`.

- [ ] **Step 1: Write the failing tests** — replace the "pre-checks", "sends cost", "maps exit reasons" tests in `tests/lab-spec-model.test.ts` and add the new ones:

```ts
it("isCond tells v1 comparisons from Phase 7 nodes", () => {
  expect(m.isCond({ lhs: { s: "close" }, op: ">", rhs: { value: 1 } })).toBe(true);
  expect(m.isCond({ lhs: { s: "close" }, op: "between", lo: { value: 1 }, hi: { value: 2 } })).toBe(false);
  expect(m.isCond({ lhs: { s: "close" }, op: "rising", bars: 3 })).toBe(false);
  expect(m.isCond({ all: [] })).toBe(false);
});

it("removeAt and putAt edit by engine path and never mutate", () => {
  const s = m.setAt(m.defaultSpec(), "exit.breakeven.after.pct", 2);
  const before = JSON.stringify(s);
  expect(m.removeAt(s, "exit.breakeven").exit).not.toHaveProperty("breakeven");
  expect(m.putAt(s, "exit.min_bars", 3).exit.min_bars).toBe(3);
  expect(m.putAt(s, "exit.min_bars", undefined).exit).not.toHaveProperty("min_bars");
  expect(m.removeAt(s, "exit.nothing.here")).toEqual(s);
  expect(JSON.stringify(s)).toBe(before);
});

it("pre-checks: entry, exit (price exits incl. breakeven and take-profits; always-in), per-side top-level cap, size", () => {
  const s = m.defaultSpec();
  const noExit = m.removeCond(s, "exit", 0);
  expect(m.specProblems(noExit, cat)).toContain("exit.any needs at least one condition (or a stop_loss, target, trailing, breakeven or take-profit exit)");
  expect(m.specProblems(m.setAt(noExit, "exit.breakeven.after.pct", 2), cat)).toEqual([]);
  expect(m.specProblems(m.setAt(noExit, "exit.take_profits", [{ pct: 2, size: 0.5 }]), cat)).toEqual([]);
  const always = { ...noExit, direction: "both", short_side: { entry: { all: [s.entry.all[0]] }, exit: { any: [] } } } as Spec;
  expect(m.specProblems(always, cat)).toEqual([]);
  expect(m.specProblems(m.setExecution(always, { opposite: "exit" }), cat)).toContain("exit.any needs at least one condition (or a stop_loss, target, trailing, breakeven or take-profit exit)");
  let many = s;
  for (let i = 0; i < 11; i++) many = m.addCond(many, "entry", s.entry.all[0]);
  expect(m.specProblems(many, cat)).toContain("Use at most 12 conditions in total.");
  const shortMany = { ...s, direction: "both", short_side: { entry: { all: Array(12).fill(s.entry.all[0]) }, exit: { any: [s.exit.any[0]] } } } as Spec;
  expect(m.specProblems(shortMany, cat)).toContain("Short side: use at most 12 conditions in total.");
  expect(m.specProblems({ ...s, entry: { all: [] } }, cat)).toContain("Add at least one entry condition.");
});

it("money mode sends no cost or slippage query; otherwise as before", () => {
  expect(m.runQuery(m.defaultSpec(), "5", "2")).toEqual({ cost: 5, slippage: 2 });
  expect(m.runQuery(m.defaultSpec(), "", "")).toEqual({});
  expect(m.runQuery(m.setExecution(m.defaultSpec(), { cost_bps: 10 }), "5", "")).toEqual({});
  expect(m.runQuery(m.setExecution(m.defaultSpec(), { capital: 100000 }), "5", "2")).toEqual({});
});

it("maps Phase 7 exit reasons to friendly text and keeps unknown text", () => {
  expect(["open", "exit.stop_loss", "exit.target (gap at the open)", "exit.trailing", "exit.breakeven", "exit.breakeven (gap at the open)",
    "exit.any[1]: rsi > 70", "short_side.exit.any[0]: rsi < 30", "reverse to short", "reverse to long", "opposite entry", "expiry", "margin_call", "roll", "something new"]
    .map(m.friendlyReason)).toEqual(["Still open at the end", "Stop-loss", "Target (gap at the open)", "Trailing stop", "Breakeven stop",
    "Breakeven stop (gap at the open)", "Exit rule 2: rsi > 70", "Short exit rule 1: rsi < 30", "Reversed to short", "Reversed to long",
    "Opposite signal", "Contract expiry", "Margin call", "Contract roll", "something new"]);
});

it("operator labels cover the Phase 7 operators", () => {
  expect(cat7.operators.every((op) => m.OPERATOR_LABELS[op])).toBe(true);
});
```
(Import `catalogue7Fixture as cat7` from `./fixtures/lab`. Keep "rejects specs that would exceed the gateway query cap" and the remaining tests unchanged. The per-condition and distance assertions removed here reappear in Task 2 and Task 3 tests.)

- [ ] **Step 2: Run** `npx vitest run tests/lab-spec-model.test.ts` — expected failures: `isCond`, `removeAt`, `putAt` missing; new messages/reasons differ; `catalogue7Fixture` missing.

- [ ] **Step 3: Implement.**

`types.ts` — replace the rule-spec section and extend responses (keep every existing export name):

```ts
// ---- rule spec (Context/guru/backend/src/lab/rules.rs; Phase 7 spec §4–§7). Unknown keys are kept by every editor.
export type ExpandMode = "last" | "first" | "point";
export interface ValueOperand { value: number }
export interface SeriesOperand { s: string; p?: Record<string, number>; tf?: Timeframe; out?: string; offset?: number; expand?: ExpandMode }
/** An expression or state operand: one key of rule-tree EXPR_KEYS plus its fields; read it through rule-tree.ts. */
export type ExprOperand = { [key: string]: unknown };
export type Operand = ValueOperand | SeriesOperand | ExprOperand;
export interface Cond { lhs: Operand; op: string; rhs: Operand; [extra: string]: unknown }
export interface RangeNode { lhs: Operand; op: "between" | "outside"; lo: Operand; hi: Operand; [extra: string]: unknown }
export interface TrendNode { lhs: Operand; op: "rising" | "falling"; bars: number; [extra: string]: unknown }
export interface AllNode { all: RuleNode[] }
export interface AnyNode { any: RuleNode[] }
export interface NotNode { not: RuleNode }
export interface AtLeastNode { at_least: number; of: number; when: RuleNode }
export interface ThenNode { then: RuleNode[]; within: number }
export type RuleNode = Cond | RangeNode | TrendNode | AllNode | AnyNode | NotNode | AtLeastNode | ThenNode;
export type Dist = { pct: number } | { atr: number; atr_length?: number } | { points: number };
export type EntryLevel = Dist | { at: Operand };
export type TakeProfit = Dist & { size: number };
export interface ScaleIn { legs: number[]; min_bars?: number; price?: "any" | "better" | "worse" }
export interface EntrySpec { all: RuleNode[]; limit?: EntryLevel; stop?: EntryLevel; valid_bars?: number; scale_in?: ScaleIn; [extra: string]: unknown }
export interface ExitSpec {
  any: RuleNode[]; stop_loss?: Dist; target?: Dist; trailing?: Dist & { activate?: Dist }; breakeven?: { after: Dist; plus?: Dist };
  take_profits?: TakeProfit[]; min_bars?: number; [extra: string]: unknown;
}
export interface ShortSide { entry: EntrySpec; exit: ExitSpec; [extra: string]: unknown }
export type Sizing = { pct_equity: number } | { risk_pct: number } | { units: number } | { value: number };
export interface MoneyCosts {
  commission?: { per_order?: number; per_unit?: number; pct?: number }; slippage?: { bps: number };
  taxes?: { name: string; pct: number; side: "buy" | "sell" | "both" }[]; borrow_pct_year?: number;
}
export interface Execution {
  fill?: Fill; exit_fill?: Fill; cost_bps?: number; opposite?: "reverse" | "exit" | "ignore"; delay_bars?: number;
  entry_on?: "level" | "edge"; intrabar?: "stop_first" | "ohlc_path"; sizing?: Sizing; capital?: number; costs?: MoneyCosts;
  cooldown?: { bars: number; after?: "any" | "loss" | "win" }; max_trades?: { count: number; per: "W" | "M" | "Q" | "Y" };
  risk?: { max_consecutive_losses?: number; pause_bars?: number; max_drawdown_pct?: number };
  [extra: string]: unknown;
}
export interface Origin { book: string; method: string; template_version: number }
export interface OptimizeDim { path: string; from: number; to: number; step: number }
export type Fill = "next_open" | "close";
export interface Spec {
  v: 1; direction?: "long" | "short" | "both"; origin?: Origin; entry: EntrySpec; exit: ExitSpec; short_side?: ShortSide;
  execution?: Execution; optimize?: OptimizeDim[]; [extra: string]: unknown;
}
```
Catalogue: add optional `custom_days?: [number, number]; expand_modes?: string[]; max_leaves?: number; max_nodes?: number; node_kinds?: string[]; expressions?: string[]; max_query_bytes?: number; execution?: CatalogueExecution;` with
`export interface CatalogueExecution { direction: string[]; fill: string[]; opposite: string[]; entry_on: string[]; intrabar: string[]; sizing: string[]; distances: string[]; max_take_profits: number; max_legs: number; max_valid_bars: number; max_delay_bars: number }`.
Results: `export type Piece = [date: string, price: number, weight: number];` Trade gains `legs?: Piece[]; fills?: Piece[]` (entry_kind stays `string`); add
`export interface ExecutionModel { kind: string; mode: "unit" | "money"; executable: boolean; notes: string[]; events: string[] }`,
`export interface MoneySummary { capital: number; final_equity: number; costs_total: number; skipped: number; currency: string | null; basis: string }`,
`export interface LedgerRow { entry: string; qty: number; notional: number; costs: number; pnl: number }`;
RunResult gains `execution_model?: ExecutionModel; money?: MoneySummary; ledger?: LedgerRow[]; adaptations?: string[]`; OptimizeResult gains `execution_model?: ExecutionModel; adaptations?: string[]`; SeriesResponse gains `adaptations?: string[]`. Update the `Timeframe` comment: `'D' | 'W' | 'M' | 'Q' | 'Y' | '<n>D' (2–63).`

`labels.ts` — append (verbatim; the model texts are `lab/model.rs` constants, the refusal text is `rules.rs` `expand_of`):

```ts
export const EXPAND_FIRST_REFUSED = "expandFirst would show the period's final value from its first day (lookahead); use offset >= 1, the period open, or expand last";
/** Pre-run mirrors of lab/model.rs notes; after a run the response's execution_model.notes are shown instead. */
export const MODEL_NOTE = {
  index: "An index cannot be traded; returns are a study of index levels, not of an ETF or futures contract.",
  indiaShort: "Hypothetical — overnight short positions are not executable in the Indian cash segment (they need F&O or SLB borrowing).",
  usShortUnit: "Short positions: borrow cost is not modelled in unit mode.",
  crypto: "CRYPTO:* is a TradingView index composite, not an exchange order book.",
  cryptoShort: "Hypothetical — shorting spot needs a margin borrow or perpetual data with funding, which is not staged.",
  cfd: "Provisional CFD quotes; CFD contract terms (size, financing) are not admitted.",
} as const;
export const MONEY_BLOCKED = {
  index: "Money mode needs a tradable quantity: an index is never priced as a stand-in for an ETF or futures contract.",
  crypto: "Money mode is not available: the crypto series is a TradingView index composite, not an exchange order book.",
  cfd: "Money mode is not available: CFD contract terms are not admitted.",
  indiaShort: "Money mode is not available for short positions on India cash instruments: overnight shorts need an F&O contract or an SLB borrow model, and neither is admitted.",
} as const;
export const FUTURES_NOTE = "Futures: the contract model exists, but no futures contract data (contract chain with expiries, lot sizes and tick size) is admitted, so futures runs answer “Data needed”. The index or spot series is never used as a stand-in.";
export const MONEY_BASIS_HINT = "User-declared costs; not a verified statutory schedule.";
export const OPERAND_KIND_LABELS: Record<string, string> = {
  price: "Price", indicator: "Indicator", calendar: "Calendar", number: "Number", bars_held: "Bars held",
  arith: "Arithmetic", pct: "% difference", abs: "Absolute value", ref: "N bars ago", window: "Rolling window", change: "Change over N bars",
  compress: "Higher-timeframe aggregate", bars_since: "Bars since", value_when: "Value when", since: "Highest / lowest since", count: "Count of bars",
};
export const CALENDAR_OUTPUTS: { id: string; out: string; label: string }[] = [
  { id: "session", out: "tdom", label: "Trading day of the month" }, { id: "session", out: "tdow", label: "Trading day of the week" },
  { id: "session", out: "tdoq", label: "Trading day of the quarter" }, { id: "session", out: "tdoy", label: "Trading day of the year" },
  { id: "session", out: "week_of_month", label: "Week of the month" }, { id: "session", out: "quarter", label: "Quarter (1–4)" },
  { id: "sessions_left", out: "m", label: "Sessions left in the month · future dates" },
  { id: "sessions_left", out: "q", label: "Sessions left in the quarter · future dates" },
  { id: "sessions_left", out: "y", label: "Sessions left in the year · future dates" },
  { id: "session", out: "weekdays_left_m", label: "Weekdays left in the month" }, { id: "session", out: "weekdays_left_q", label: "Weekdays left in the quarter" },
  { id: "session", out: "weekdays_left_y", label: "Weekdays left in the year" },
  { id: "calendar", out: "dow", label: "Day of the week (0 Sun … 6 Sat)" }, { id: "calendar", out: "day", label: "Day of the month" },
  { id: "calendar", out: "month", label: "Month (1–12)" }, { id: "calendar", out: "year", label: "Year" }, { id: "calendar", out: "doy", label: "Day of the year" },
];
export const TF_LABELS: Record<string, string> = { D: "Daily", W: "Weekly", M: "Monthly", Q: "Quarterly", Y: "Yearly" };
export const ENTRY_KIND_LABELS: Record<string, string> = {
  next_open: "Next open", close: "Close", limit: "Limit", stop: "Stop", stop_limit: "Stop-limit", reverse: "Reversal", roll: "Contract roll",
};
```
and extend `COPY` with:

```ts
  longSide: "Long side", shortSide: "Short side",
  longEntryTitle: "Long entry · all of these", longExitTitle: "Long exit · any of these",
  shortEntryTitle: "Short entry · all of these", shortExitTitle: "Short exit · any of these",
  addMenu: "+ Add…",
  addOptions: { cond: "Condition", all: "All of (group)", any: "Any of (group)", not: "Not (negate)", at_least: "At least K of the last M bars", then: "Sequence (A then B within N bars)" },
  nodeMenu: { wrap_all: "Wrap in All of", wrap_any: "Wrap in Any of", wrap_not: "Negate (Not)", unwrap: "Unwrap", up: "Move up", down: "Move down" },
  heldTopOnly: "Bars held works only in a top-level exit condition (not inside a group, range, trend or formula).",
  tooDeep: "These rules are nested too deeply (the engine reads at most 16 levels). Unwrap a group or a formula.",
  caps: (top: string, leaves: number, maxLeaves: number, nodes: number, maxNodes: number, kb: string) =>
    `${top} · comparisons ${leaves}/${maxLeaves} · rule parts ${nodes}/${maxNodes} · ${kb} of 7.5 KB`,
  sessionDatesHint: "These rules read Sessions left, which uses this instrument's future session dates (never prices). Results carry an adaptation note.",
  oppositeHelp: {
    reverse: "Close the position and open the other side in one fill (the AmiBroker and TradingView default).",
    exit: "Only close the position; a new trade needs the next signal.",
    ignore: "Keep the position; the other side's signal does nothing until the position is closed.",
  },
  bothFlat: "If both sides signal on the same bar while flat, no trade is taken.",
  intrabarHelp: {
    stop_first: "When a stop and a target both sit inside one daily bar, the stop is assumed to fill first (AmiBroker's priority; the cautious choice).",
    ohlc_path: "TradingView's path: open, then the nearer of high and low, then the other, then close; the first level the path reaches fills first.",
  },
  delayOnlyNextOpen: "Fill delay applies to next-open fills only.",
  stopLimitHint: "A limit set as a distance sits below the signal close for a long (above for a short), so this order fills only after a gap through the stop. For the usual stop-limit, set the limit with At (at or above the stop).",
  minBarsHint: "Stops and targets stay active.",
  moneyCostsHere: "Money mode: fees, slippage and taxes come from the money settings.",
  needsMoney: "Shares and position-value sizing need money mode.",
  riskNeedsStop: "Risk per trade needs a stop-loss on every side.",
  borrowRequired: "US short positions in money mode need a declared borrow cost (0 is allowed).",
  scaleInWithTp: "Scale-in cannot be combined with take-profits.",
  studyOnly: "Study only · not executable", unitStudy: "Unit study",
  riskEvents: "Risk events",
```
Update `COPY.wmRule` to `"Higher-timeframe values are used only after that period closes (unless a timing option says otherwise)."`.

`spec-model.ts`:
- `export const isCond = (n: unknown): n is Cond => typeof n === "object" && n !== null && "lhs" in n && "rhs" in n && typeof (n as Cond).op === "string" && !["between", "outside", "rising", "falling"].includes((n as Cond).op);`
- `conds()` returns `RuleNode[]`; `withConds`, `setCond`, `addCond` take `RuleNode`.
- `export function removeAt<T>(obj: T, path: string): T` — copies along the path and deletes the last key (array parents: splice); returns `obj` unchanged when the path does not exist. `export const putAt = <T,>(obj: T, path: string, value: unknown): T => (value === undefined ? removeAt(obj, path) : setAt(obj, path, value));`
- `OPERATOR_LABELS` gains `"=": "equals", "!=": "is not", crosses: "crosses", between: "is between", outside: "is outside", rising: "is rising for", falling: "is falling for"`.
- `orderOf`: `const d = key === "limit" ? spec.entry.limit : spec.exit[key]; return d && !("at" in d) ? d : undefined;` (the v1 orders panel lives until Task 7).
- `optimizablePaths`: skip `!isCond(cond)` nodes; for orders skip distances that are neither pct nor atr (replaced in Task 10).
- Rename `MAX_BYTES` to exported `SPEC_BYTE_BUDGET = 7500`.
- `specProblems(spec, cat)` keeps only: empty entry (`COPY.emptyEntry`); the exit rule `"exit.any needs at least one condition (or a stop_loss, target, trailing, breakeven or take-profit exit)"` for each side (`"short_side.exit.any needs …"` for the short side), skipped when `direction === "both"` and `(execution.opposite ?? "reverse") === "reverse"` and *both* exit lists are empty (always in); `"Use at most ${max} conditions in total."` for the long/only side and `"Short side: use at most ${max} conditions in total."` (top-level entry+exit per side); `COPY.tooLong`. Delete `distProblems` and the per-condition loop (moved to Tasks 2 and 3).
- `runQuery`: `if (spec.execution?.capital !== undefined) return {};` first.
- `friendlyReason`: order regex `/^exit\.(stop_loss|target|trailing|breakeven)( \(gap at the open\))?$/` with `breakeven: "Breakeven stop"`; `/^(short_side\.)?exit\.any\[(\d+)\]: (.*)$/` → `Short exit rule n: …` / `Exit rule n: …`; fixed map `{ "reverse to long": "Reversed to long", "reverse to short": "Reversed to short", "opposite entry": "Opposite signal", expiry: "Contract expiry", margin_call: "Margin call", roll: "Contract roll" }`.

`RuleBuilder.tsx` (fallout only): import `isCond`; `weeklyOrMonthly` reads `all.filter(isCond)`; in the list map, a node that is not `isCond` renders `<div className="lab-cond" key={i}><span className="lab-kept">{COPY.keptAsIs} <code>{JSON.stringify(c)}</code></span>` plus the existing remove button.

`tests/fixtures/lab.ts`: add `catalogue7Fixture` (= `catalogueFixture` plus `operators` = the 13 `rules.rs` OPERATORS, `timeframes: ["D","W","M","Q","Y"]`, `custom_days: [2, 63]`, `expand_modes: ["last","first","point"]`, `max_leaves: 40`, `max_nodes: 120`, `node_kinds`, `expressions` = the 20 `rules.rs` EXPRESSIONS, `max_query_bytes: 16384`, `execution` = the Task 12 object with `max_take_profits: 3, max_legs: 5, max_valid_bars: 20, max_delay_bars: 10`, and three more indicators in category "Periods & calendar": `calendar` (outputs dow, day, month, year, doy, bar_index), `session` (tdom, tdoq, tdoy, tdow, week_of_month, quarter, weekdays_left_m, weekdays_left_q, weekdays_left_y), `sessions_left` (m, q, y)); `run7Fixture` (= `runFixture` plus `execution_model: { kind: "index", mode: "unit", executable: false, notes: [MODEL_NOTE.index], events: ["risk pause after 2 losing trades on 2024-01-15"] }`, `adaptations: ["sessions_left reads this instrument's future session dates"]`, trade 0 with `legs: [["2024-01-08", 101, 0.5], ["2024-01-09", 101.5, 0.5]]` and `fills: [["2024-01-10", 103, 0.5], ["2024-01-15", 102, 0.5]]`, trade 1 with `entry_kind: "reverse"`); `money7Fixture` (= `runFixture` with instrument SPY `{ type: "etf", native: true, market: "us", currency: "USD" }`, `execution_model: { kind: "etf", mode: "money", executable: true, notes: [], events: [] }`, `money: { capital: 100000, final_equity: 101234.5, costs_total: 42, skipped: 1, currency: "USD", basis: MONEY_BASIS_HINT + " Quantities are computed on the admitted price series." }`, `ledger` with one row per trade: `{ entry: "2024-01-08", qty: 10, notional: 1010, costs: 21, pnl: 9.4 }`, `{ entry: "2024-02-01", qty: 10, notional: 1050, costs: 21, pnl: -10.5 }`).

`vite.config.ts`: add `"src/features/lab/rule-tree.ts"`, `"src/features/lab/execution-model.ts"`, `"src/features/lab/optimize-paths.ts"` to `coverage.include`.

- [ ] **Step 4: Run** `npx vitest run tests/lab-spec-model.test.ts tests/lab-rule-builder.test.tsx tests/lab-optimize.test.tsx tests/lab-results.test.tsx` and `npx tsc --noEmit` — all pass, no type errors anywhere.
- [ ] **Step 5: Browser acceptance:** n/a for new UI (types and copy). Sanity: open `http://127.0.0.1:5173/#/indicators?tk=NIFTY%2050` (if up) — the page renders as before.
- [ ] **Step 6: Commit (permission only):** `(lab-ui) Phase 7 foundation: types, labels, fixtures, spec-model updates` with the seven paths.

---

### Task 2: Rule tree model (`rule-tree.ts`)

**Files:** Create `src/features/lab/rule-tree.ts`, `tests/lab-rule-tree.test.ts`.
**Depends on:** Task 1. **Backend:** none (mirrors `rules.rs` as committed).
**Produces (exact names):** `ARITH`, `WINDOWS`, `EXPR_KEYS`, `ExprKey`, `MAX_WHOLE`, `MAX_NTH`, `MAX_STEPS`, `MAX_DEPTH`, `NodeKind`, `nodeKind`, `isLeaf`, `exprKey`, `OperandKind`, `operandKind`, `CALENDAR_IDS`, `DEFAULT_COND`, `DEFAULT_STATE_COND`, `newNode`, `withOp`, `wrap`, `unwrap`, `move`, `convertOperand`, `innerOperand`, `ListPath`, `LIST_PATHS`, `WalkItem`, `walkSpec`, `Budget`, `budget`, `limits`, `pathLabel`, `firstAllowed`, `tfOk`, `timingProblem`, `treeProblems`, `readsSessionDates`, `usesHigherTimeframes`, `jsonDepth`, `describeOperand`, `describeNode`.

- [ ] **Step 1: Write the failing tests** (`tests/lab-rule-tree.test.ts`):

```ts
import { expect, it } from "vitest";
import * as t from "../src/features/lab/rule-tree";
import { EXPAND_FIRST_REFUSED } from "../src/features/lab/labels";
import { getAt } from "../src/features/lab/spec-model";
import type { Cond, Operand, RuleNode, Spec } from "../src/features/lab/types";
import { catalogueFixture as cat, catalogue7Fixture as cat7 } from "./fixtures/lab";

const C: Cond = { lhs: { s: "close" }, op: ">", rhs: { s: "ema", p: { length: 50 } } };
const spec = (patch: Partial<Spec>): Spec => ({ v: 1, entry: { all: [C] }, exit: { any: [{ lhs: { s: "bars_held" }, op: ">=", rhs: { value: 5 } }] }, ...patch });

it("classifies nodes and operands", () => {
  expect(t.nodeKind(C)).toBe("cmp");
  expect(t.nodeKind({ lhs: { s: "rsi" }, op: "between", lo: { value: 30 }, hi: { value: 70 } })).toBe("range");
  expect(t.nodeKind({ lhs: { s: "close" }, op: "falling", bars: 3 })).toBe("trend");
  expect((["all", "any", "not", "at_least", "then"] as const).map((k) => t.nodeKind(t.newNode(k)))).toEqual(["all", "any", "not", "at_least", "then"]);
  expect(t.nodeKind({ fancy: 1 } as never)).toBe("unknown");
  const k = (o: Operand, c = cat7) => t.operandKind(o, c);
  expect([k({ value: 3 }), k({ s: "close" }), k({ s: "rsi" }), k({ s: "bars_held" }), k({ s: "session", out: "tdom" }), k({ s: "nope" })])
    .toEqual(["number", "price", "indicator", "bars_held", "calendar", "unknown"]);
  expect([k({ sub: [{ s: "close" }, { value: 1 }] }), k({ pct: [{ s: "close" }, { s: "open" }] }), k({ avg: { s: "close" }, bars: 5 }),
    k({ change: { s: "close" }, bars: 1 }), k({ ref: { s: "close" }, bars: 1 }), k({ highest_since: C, of: { s: "high" } }),
    k({ count: C, bars: 5 }), k({ compress: { s: "close" }, tf: "W" })]).toEqual(["arith", "pct", "window", "change", "ref", "since", "count", "compress"]);
});

it("budget counts like the engine (comparisons, groups, expressions; both sides; at levels)", () => {
  const s: Spec = { v: 1, direction: "both",
    entry: { all: [{ any: [C, { not: C }] }, { lhs: { ref: { s: "close" }, bars: 1 }, op: "<", rhs: { value: 5 } }], stop: { at: { highest: { s: "high" }, bars: 5 } } },
    exit: { any: [{ lhs: { s: "bars_held" }, op: ">=", rhs: { value: 3 } }] },
    short_side: { entry: { all: [{ at_least: 2, of: 3, when: C }] }, exit: { any: [{ lhs: { bars_since: C }, op: ">", rhs: { value: 4 } }] } } };
  expect(t.budget(s)).toMatchObject({ leaves: 7, nodes: 13, top: { long: 3, short: 2 } });
  expect(t.budget(spec({ entry: { all: [{ lhs: { s: "close", tf: "W", expand: "point" }, op: ">", rhs: { value: 1 } }] } }))).toMatchObject({ leaves: 2, nodes: 2 });
  expect(t.limits(cat)).toEqual({ leaves: 40, nodes: 120, top: 12 });
});

it("walker yields the engine's optimize paths in engine order, as JSON paths", () => {
  const s = spec({
    entry: { all: [
      { lhs: { s: "close" }, op: "rising", bars: 3 },
      { at_least: 2, of: 3, when: { lhs: { avg: { s: "close" }, bars: 10 }, op: ">", rhs: { value: 1 } } },
      { then: [C, { lhs: { s: "rsi", p: { length: 2 } }, op: "<", rhs: { ref: { s: "rsi" }, bars: 0 } }], within: 5 },
    ] },
    exit: { any: [{ lhs: { value_when: C, of: { s: "close" }, nth: 2 }, op: "<", rhs: { s: "close" } }] },
  });
  const knobs: t.WalkItem[] = [];
  t.walkSpec(s, (x) => x.t === "knob" && knobs.push(x));
  expect(knobs.map((x) => x.path)).toEqual(["entry.all[0].bars", "entry.all[1].at_least", "entry.all[1].of", "entry.all[1].when.lhs.bars",
    "entry.all[1].when.rhs.value", "entry.all[2].then[1].rhs.bars", "entry.all[2].within", "exit.any[0].lhs.nth"]);
  for (const x of knobs) if (x.t === "knob") expect(getAt(s, x.path)).toBe(x.value);
  expect(knobs.find((x) => x.path.endsWith("rhs.bars"))).toMatchObject({ min: 0, max: 1000, whole: true });
  expect(knobs.find((x) => x.path.endsWith(".nth"))).toMatchObject({ min: 1, max: 20 });
  const held: string[] = [];
  t.walkSpec(spec({ exit: { any: [{ lhs: { s: "bars_held" }, op: ">=", rhs: { value: 5 } }, { not: { lhs: { s: "bars_held" }, op: ">", rhs: { value: 1 } } }] } }),
    (x) => x.t === "operand" && x.held && held.push(x.path));
  expect(held).toEqual(["exit.any[0].lhs", "exit.any[0].rhs"]);
});

it("labels engine paths in plain words", () => {
  expect(t.pathLabel("entry.all[0]", false)).toBe("Entry 1");
  expect(t.pathLabel("entry.all[2].any[0].rhs.value", false)).toBe("Entry 3 › any 1 › right › number");
  expect(t.pathLabel("exit.any[0]", true)).toBe("Long exit 1");
  expect(t.pathLabel("short_side.exit.any[1].lhs.bars_since", true)).toBe("Short exit 2 › left › bars since");
  expect(t.pathLabel("entry.stop.at.bars", false)).toBe("Stop level › bars");
  expect(t.pathLabel("entry.all[1].when.lhs.bars", false)).toBe("Entry 2 › when › left › bars");
  expect(t.pathLabel("entry.all[0].lhs.p.length", false)).toBe("Entry 1 › left › length");
});

it("tree problems mirror rules.rs in plain words", () => {
  expect(t.treeProblems(spec({}), cat7)).toEqual([]);
  const p = (s: Spec) => t.treeProblems(s, cat7);
  expect(p(spec({ entry: { all: [{ any: [{ lhs: { value: 1 }, op: ">", rhs: { value: 2 } }] }] } })))
    .toContain("Entry 1 › any 1: a condition cannot compare two numbers; one side must be a series.");
  expect(p(spec({ entry: { all: [{ at_least: 4, of: 3, when: C }] } }))).toContain('Entry 1: "at least" (4) cannot exceed the number of bars (3).');
  expect(p(spec({ entry: { all: [{ then: [C], within: 5 }] } }))).toContain("Entry 1: a sequence needs 2 to 4 steps.");
  expect(p(spec({ entry: { all: [{ then: [C, C, C], within: 1 }] } }))).toContain("Entry 1: within (1) is shorter than the 3 steps need (2).");
  expect(p(spec({ entry: { all: [{ all: [] }] } }))).toContain("Entry 1: add at least one condition to this group.");
  expect(p(spec({ entry: { all: [{ lhs: { s: "close" }, op: "between", lo: { value: 9 }, hi: { value: 1 } }] } }))).toContain("Entry 1: the low bound is above the high bound.");
  expect(p(spec({ entry: { all: [{ lhs: { s: "bars_held" }, op: ">", rhs: { value: 1 } }] } })))
    .toContain("Entry 1 › left: Bars held works only in a top-level exit condition (not inside a group, range, trend or formula).");
  expect(p(spec({ exit: { any: [{ not: { lhs: { s: "bars_held" }, op: ">", rhs: { value: 1 } } }] } })))
    .toContain("Exit 1 › not › left: Bars held works only in a top-level exit condition (not inside a group, range, trend or formula).");
  expect(p(spec({ entry: { all: [{ lhs: { s: "or_range" }, op: ">", rhs: { value: 1 } }] } })))
    .toContain("Entry 1 › left: Opening Range is listed only and cannot be used in rules. Needs intraday (minute) bars; not available yet.");
  expect(p(spec({ entry: { all: [{ lhs: { s: "close", tf: "W", expand: "first" }, op: ">", rhs: { value: 1 } }] } }))).toContain(`Entry 1 › left: ${EXPAND_FIRST_REFUSED}`);
  expect(p(spec({ entry: { all: [{ lhs: { s: "open", tf: "W", expand: "first" }, op: ">", rhs: { value: 1 } }] } }))).toEqual([]);
  expect(p(spec({ entry: { all: [{ lhs: { compress: { s: "high" }, tf: "M", mode: "first", expand: "first" }, op: ">", rhs: { value: 1 } }] } }))).toEqual([]);
  expect(p(spec({ entry: { all: [{ lhs: { s: "close", expand: "point" }, op: ">", rhs: { value: 1 } }] } })))
    .toContain("Entry 1 › left: “Use the period value” applies to W, M, Q, Y and N-session values only.");
  expect(p(spec({ entry: { all: [{ lhs: { s: "close", tf: "70D" }, op: ">", rhs: { value: 1 } }] } })))
    .toContain("Entry 1 › left: timeframe must be D, W, M, Q, Y or N sessions (2 to 63).");
  expect(p(spec({ entry: { all: [{ lhs: { ref: { value: 3 }, bars: 1 }, op: ">", rhs: { s: "close" } }] } }))).toContain("Entry 1 › left: N bars ago needs a series, not a number.");
  expect(p(spec({ entry: { all: [{ lhs: { add: [{ value: 1 }, { value: 2 }] }, op: ">", rhs: { s: "close" } }] } })))
    .toContain("Entry 1 › left: a formula cannot combine two numbers; use at least one series.");
  expect(p(spec({ entry: { all: [{ lhs: { avg: { s: "close" }, bars: 0 }, op: ">", rhs: { s: "close" } }] } }))).toContain("Entry 1 › left › bars must be a whole number from 1 to 1000.");
  const big = spec({ entry: { all: Array.from({ length: 12 }, () => ({ all: [C, C, C, C] })) } });
  expect(p(big)).toContain("More than 40 comparisons in the rules; the limit is 40.");
  let deep: RuleNode = C;
  for (let i = 0; i < 12; i++) deep = { not: deep };
  expect(p(spec({ entry: { all: [deep] } }))).toContain("These rules are nested too deeply (the engine reads at most 16 levels). Unwrap a group or a formula.");
});

it("converts operands: wraps, swaps formulas keeping the inner value, replaces plain kinds", () => {
  const rsi = { s: "rsi", p: { length: 2 } };
  expect(t.convertOperand(rsi, "ref", cat7)).toEqual({ ref: rsi, bars: 1 });
  expect(t.convertOperand({ ref: rsi, bars: 1 }, "window", cat7)).toEqual({ avg: rsi, bars: 10 });
  expect(t.convertOperand({ value: 5 }, "abs", cat7)).toEqual({ abs: { s: "close" } });
  expect(t.convertOperand({ value: 5 }, "arith", cat7)).toEqual({ sub: [{ s: "close" }, { value: 5 }] });
  expect(t.convertOperand(rsi, "arith", cat7)).toEqual({ sub: [rsi, { value: 0 }] });
  expect(t.convertOperand(rsi, "pct", cat7)).toEqual({ pct: [rsi, { ref: rsi, bars: 1 }] });
  expect(t.convertOperand(rsi, "bars_since", cat7)).toEqual({ bars_since: t.DEFAULT_STATE_COND });
  expect(t.convertOperand(rsi, "value_when", cat7)).toEqual({ value_when: t.DEFAULT_STATE_COND, of: rsi, nth: 1 });
  expect(t.convertOperand(rsi, "since", cat7)).toEqual({ highest_since: t.DEFAULT_STATE_COND, of: rsi });
  expect(t.convertOperand(rsi, "count", cat7)).toEqual({ count: t.DEFAULT_STATE_COND, bars: 20 });
  expect(t.convertOperand(rsi, "compress", cat7)).toEqual({ compress: rsi, tf: "W", mode: "last" });
  expect(t.convertOperand({ s: "ema", p: { length: 9 }, tf: "W", offset: 1 }, "price", cat7)).toEqual({ s: "close", tf: "W", offset: 1 });
  expect(t.convertOperand(rsi, "calendar", cat7)).toEqual({ s: "session", out: "tdom" });
  expect(t.convertOperand(rsi, "number", cat7)).toEqual({ value: 0 });
  expect(t.innerOperand({ sub: [rsi, { value: 1 }] })).toEqual(rsi);
  expect(t.innerOperand({ value_when: C, of: rsi })).toEqual(rsi);
  expect(t.innerOperand({ bars_since: C })).toBeUndefined();
});

it("tree transforms keep unknown keys", () => {
  const tagged = { ...C, tag: "x" } as Cond;
  expect(t.withOp(tagged, "between")).toEqual({ lhs: C.lhs, op: "between", lo: C.rhs, hi: { value: 100 }, tag: "x" });
  expect(t.withOp({ lhs: { s: "rsi" }, op: "between", lo: { value: 30 }, hi: { value: 70 } } as RuleNode, "rising")).toEqual({ lhs: { s: "rsi" }, op: "rising", bars: 3 });
  expect(t.withOp({ lhs: { s: "rsi" }, op: "between", lo: { value: 30 }, hi: { value: 70 } } as RuleNode, "outside")).toMatchObject({ lo: { value: 30 }, hi: { value: 70 } });
  expect(t.withOp({ lhs: { s: "close" }, op: "rising", bars: 3 } as RuleNode, ">")).toEqual({ lhs: { s: "close" }, op: ">", rhs: { value: 0 } });
  expect(t.withOp({ lhs: { s: "rsi" }, op: "<", rhs: { value: 30 } }, "between")).toMatchObject({ lo: { value: 30 }, hi: { value: 40 } });
  expect(t.wrap(C, "not")).toEqual({ not: C });
  expect(t.unwrap({ all: [C] })).toEqual(C);
  expect(t.unwrap({ all: [C, C] })).toBeUndefined();
  expect(t.unwrap({ at_least: 2, of: 3, when: C })).toEqual(C);
  expect(t.move(["a", "b", "c"], 0, 1)).toEqual(["b", "a", "c"]);
  expect(t.move(["a", "b", "c"], 2, 1)).toEqual(["a", "b", "c"]);
});

it("detects session-date readers, higher timeframes and describes nodes", () => {
  expect(t.readsSessionDates(spec({ exit: { any: [{ lhs: { s: "sessions_left", out: "m" }, op: "<", rhs: { value: 2 } }] } }))).toBe(true);
  expect(t.readsSessionDates(spec({}))).toBe(false);
  expect(t.usesHigherTimeframes(spec({ entry: { all: [{ lhs: { compress: { s: "close" }, tf: "Q" }, op: ">", rhs: { value: 1 } }] } }))).toBe(true);
  expect(t.firstAllowed({ s: "close", tf: "W" })).toBe(false);
  expect(t.firstAllowed({ s: "close", tf: "W", offset: 1 })).toBe(true);
  expect(t.firstAllowed({ s: "open", tf: "W" })).toBe(true);
  expect(t.describeNode({ any: [C, { lhs: { ref: { s: "close" }, bars: 1 }, op: "<", rhs: { value: 5 } }] }, cat))
    .toBe("Any of 2: close is above Exponential Moving Average(50); close · 1 bar ago is below 5");
  expect(t.describeNode({ at_least: 2, of: 3, when: C }, cat)).toBe("At least 2 of 3 bars: close is above Exponential Moving Average(50)");
  expect(t.describeOperand({ avg: { s: "close" }, bars: 10 }, cat)).toBe("average(close, 10)");
});
```

- [ ] **Step 2: Run** `npx vitest run tests/lab-rule-tree.test.ts` — fails: module missing.

- [ ] **Step 3: Implement** `src/features/lab/rule-tree.ts`:

```ts
/** Phase 7 rule tree (rules.rs grammar): kinds, value-based edits, the walker that follows the engine's optimize paths,
 *  the engine's comparison/node budget, location labels and UX pre-checks. The engine stays the authority. */
import { COPY, EXPAND_FIRST_REFUSED, OPERAND_KIND_LABELS } from "./labels";
import { getAt, isCond, isSeries, isValue, OPERATOR_LABELS, operandLabel } from "./spec-model";
import type { Catalogue, Cond, Operand, RuleNode, Spec } from "./types";

export const ARITH = ["add", "sub", "mul", "div", "min", "max"] as const;
export const WINDOWS = ["sum", "avg", "highest", "lowest"] as const;
/** rules.rs EXPRESSIONS, same order. */
export const EXPR_KEYS = [...ARITH, "pct", "abs", "ref", ...WINDOWS, "change", "bars_since", "value_when", "highest_since", "lowest_since", "count", "compress"] as const;
export type ExprKey = (typeof EXPR_KEYS)[number];
export const MAX_WHOLE = 1000, MAX_NTH = 20, MAX_STEPS = 4, MAX_DEPTH = 16;
const RANGE_OPS = ["between", "outside"], TREND_OPS = ["rising", "falling"];
type Obj = Record<string, unknown>;
const isObj = (v: unknown): v is Obj => typeof v === "object" && v !== null && !Array.isArray(v);

export type NodeKind = "cmp" | "range" | "trend" | "all" | "any" | "not" | "at_least" | "then" | "unknown";
export function nodeKind(n: unknown): NodeKind {
  if (!isObj(n)) return "unknown";
  if ("lhs" in n) return RANGE_OPS.includes(n.op as string) ? "range" : TREND_OPS.includes(n.op as string) ? "trend" : isCond(n) ? "cmp" : "unknown";
  if (Array.isArray(n.all)) return "all";
  if (Array.isArray(n.any)) return "any";
  if ("not" in n) return "not";
  if ("at_least" in n) return "at_least";
  return Array.isArray(n.then) ? "then" : "unknown";
}
export const isLeaf = (k: NodeKind) => k === "cmp" || k === "range" || k === "trend";
/** The first expression key, as rules.rs `expr()` finds it. */
export const exprKey = (o: unknown): ExprKey | undefined => (isObj(o) ? (Object.keys(o).find((k) => (EXPR_KEYS as readonly string[]).includes(k)) as ExprKey | undefined) : undefined);

export type OperandKind = "number" | "price" | "indicator" | "calendar" | "bars_held" | "arith" | "pct" | "abs" | "ref" | "window" | "change"
  | "compress" | "bars_since" | "value_when" | "since" | "count" | "unknown";
export const CALENDAR_IDS = ["calendar", "session", "sessions_left"];
const kindOfKey = (k: ExprKey): OperandKind =>
  (ARITH as readonly string[]).includes(k) ? "arith" : (WINDOWS as readonly string[]).includes(k) ? "window" : k === "highest_since" || k === "lowest_since" ? "since" : (k as OperandKind);
export function operandKind(o: Operand, cat: Catalogue): OperandKind {
  if (isValue(o)) return "number";
  const k = exprKey(o);
  if (k) return kindOfKey(k);
  if (!isSeries(o)) return "unknown";
  if (o.s === "bars_held") return "bars_held";
  if (cat.price_sources.includes(o.s)) return "price";
  if (CALENDAR_IDS.includes(o.s)) return "calendar";
  return cat.indicators.some((i) => i.id === o.s) ? "indicator" : "unknown";
}
```
Then:
- `DEFAULT_COND: Cond = { lhs: { s: "close" }, op: ">", rhs: { s: "ema", p: { length: 50 } } }`; `DEFAULT_STATE_COND: Cond = { lhs: { s: "close" }, op: "crosses_above", rhs: { s: "ema", p: { length: 50 } } }`.
- `newNode(kind: "cond" | "all" | "any" | "not" | "at_least" | "then"): RuleNode` → `cond`: clone of `DEFAULT_COND`; `all`/`any`: `{ [kind]: [cond] }`; `not`: `{ not: cond }`; `at_least`: `{ at_least: 2, of: 3, when: { lhs: { s: "close" }, op: ">", rhs: { ref: { s: "close" }, bars: 1 } } }`; `then`: `{ then: [{ lhs: { s: "close" }, op: "crosses_above", rhs: { s: "ema", p: { length: 20 } } }, cond], within: 5 }`. Always `structuredClone` the defaults.
- `withOp(n, op)`: keeps every key except `lhs op rhs lo hi bars`; range: `lo = n.lo ?? n.rhs ?? {value: 0}`, `hi = n.hi ?? { value: isValue(lo) ? lo.value + 10 : 100 }`; trend: `bars = n.bars ?? 3`; comparison: `rhs = n.rhs ?? n.lo ?? { value: 0 }`.
- `wrap(n, k)`; `unwrap(n)` → `not` child, single child of `all`/`any`, `at_least.when`, else `undefined`; `move(list, i, d)` swaps or returns the list unchanged at the ends.
- `innerOperand(o)` → `[A,B]` keys: A; `abs ref sum avg highest lowest change compress`: that key's operand; `value_when`/`*_since`: `of`; else `undefined`.
- `convertOperand(o, to, cat)`: plain kinds replace (`number` → `{value: 0}`; `price` → `{ s: "close", ...timing }`; `indicator` → `{ s: <first computed indicator not in CALENDAR_IDS>, ...timing }`; `calendar` → `{ s: "session", out: "tdom" }` when `session` is computed, else `{ s: "calendar", out: "dow" }`; `bars_held` → `{ s: "bars_held" }`), where `timing` keeps `tf`, `offset`, `expand` of a series. Formula kinds use `base = innerOperand(o) ?? o`, with `series = isValue(base) ? { s: "close" } : base`: `arith` → `{ sub: isValue(base) ? [{ s: "close" }, base] : [base, { value: 0 }] }`; `pct` → `{ pct: [series, { ref: series, bars: 1 }] }`; `abs` → `{ abs: series }`; `ref` → `{ ref: series, bars: 1 }`; `window` → `{ avg: series, bars: 10 }`; `change` → `{ change: series, bars: 1 }`; `bars_since` → `{ bars_since: DEFAULT_STATE_COND }`; `value_when` → `{ value_when: DEFAULT_STATE_COND, of: series, nth: 1 }`; `since` → `{ highest_since: DEFAULT_STATE_COND, of: series }`; `count` → `{ count: DEFAULT_STATE_COND, bars: 20 }`; `compress` → `{ compress: series, tf: "W", mode: "last" }`. Converting to the current kind returns `o`.

Walker (the core; mirrors `rules.rs` `walk`, `walk_node`, `walk_operand` and `Budget::take`):

```ts
export const LIST_PATHS = ["entry.all", "exit.any", "short_side.entry.all", "short_side.exit.any"] as const;
export type ListPath = (typeof LIST_PATHS)[number];
const LEVEL_ROOTS = ["entry.limit.at", "entry.stop.at", "short_side.entry.limit.at", "short_side.entry.stop.at"];
export type WalkItem =
  | { t: "node"; path: string; root: string; node: RuleNode; kind: NodeKind; top: boolean }
  | { t: "operand"; path: string; root: string; operand: Operand; held: boolean }
  | { t: "knob"; path: string; root: string; value: number; whole: boolean; min: number; max: number };
type Emit = (x: WalkItem) => void;

export function walkSpec(spec: Spec, f: Emit): void {
  for (const root of LIST_PATHS) {
    const list = getAt(spec, root);
    if (Array.isArray(list)) list.forEach((n, i) => walkNode(n as RuleNode, `${root}[${i}]`, root, true, f));
  }
  for (const root of LEVEL_ROOTS) { const o = getAt(spec, root); if (o !== undefined) walkOperand(o as Operand, root, root, false, f); }
}
const knob = (f: Emit, path: string, root: string, v: unknown, min: number, max = MAX_WHOLE) =>
  f({ t: "knob", path, root, value: typeof v === "number" ? v : NaN, whole: true, min, max });
function walkNode(n: RuleNode, path: string, root: string, top: boolean, f: Emit) {
  const kind = nodeKind(n);
  f({ t: "node", path, root, node: n, kind, top });
  const x = n as unknown as Obj, kids = (k: string) => (Array.isArray(x[k]) ? (x[k] as RuleNode[]) : []);
  switch (kind) {
    case "cmp": { const held = top && root.endsWith("exit.any"); walkOperand(x.lhs as Operand, `${path}.lhs`, root, held, f); walkOperand(x.rhs as Operand, `${path}.rhs`, root, held, f); break; }
    case "range": for (const k of ["lhs", "lo", "hi"]) walkOperand(x[k] as Operand, `${path}.${k}`, root, false, f); break;
    case "trend": walkOperand(x.lhs as Operand, `${path}.lhs`, root, false, f); knob(f, `${path}.bars`, root, x.bars, 1); break;
    case "all": case "any": kids(kind).forEach((c, i) => walkNode(c, `${path}.${kind}[${i}]`, root, false, f)); break;
    case "not": walkNode(x.not as RuleNode, `${path}.not`, root, false, f); break;
    case "at_least": knob(f, `${path}.at_least`, root, x.at_least, 1); knob(f, `${path}.of`, root, x.of, 1); walkNode(x.when as RuleNode, `${path}.when`, root, false, f); break;
    case "then": kids("then").forEach((c, i) => walkNode(c, `${path}.then[${i}]`, root, false, f)); knob(f, `${path}.within`, root, x.within, 1); break;
  }
}
function walkOperand(o: Operand | undefined, path: string, root: string, held: boolean, f: Emit) {
  if (o === undefined) return;
  f({ t: "operand", path, root, operand: o, held });
  if (isValue(o)) { f({ t: "knob", path: `${path}.value`, root, value: o.value, whole: false, min: -Infinity, max: Infinity }); return; }
  const k = exprKey(o);
  if (!k) return;
  const x = o as Obj;
  const sub = (key: string) => walkOperand(x[key] as Operand, `${path}.${key}`, root, false, f);
  const cond = (key: string) => walkNode(x[key] as RuleNode, `${path}.${key}`, root, false, f);
  if ((ARITH as readonly string[]).includes(k) || k === "pct") {
    if (Array.isArray(x[k])) (x[k] as Operand[]).forEach((c, i) => walkOperand(c, `${path}.${k}[${i}]`, root, false, f));
    return;
  }
  switch (k) {
    case "abs": sub("abs"); break;
    case "ref": sub("ref"); knob(f, `${path}.bars`, root, x.bars, 0); break;
    case "sum": case "avg": case "highest": case "lowest": case "change": sub(k); knob(f, `${path}.bars`, root, x.bars, 1); break;
    case "bars_since": cond("bars_since"); break;
    case "count": cond("count"); knob(f, `${path}.bars`, root, x.bars, 1); break;
    case "value_when": cond("value_when"); sub("of"); knob(f, `${path}.nth`, root, x.nth ?? 1, 1, MAX_NTH); break;
    case "highest_since": case "lowest_since": cond(k); sub("of"); break;
    case "compress": sub("compress"); break;
  }
}

export interface Budget { leaves: number; nodes: number; top: { long: number; short: number } }
/** rules.rs Budget::take: every leaf (comparison, range, trend) and every group and expression counts once, over both sides and `at` levels. */
export function budget(spec: Spec): Budget {
  let leaves = 0, nodes = 0;
  walkSpec(spec, (x) => {
    if (x.t === "node") { nodes++; if (isLeaf(x.kind)) leaves++; }
    else if (x.t === "operand" && exprKey(x.operand)) nodes++;
  });
  const side = (e?: { all: unknown[] }, x?: { any: unknown[] }) => (e?.all.length ?? 0) + (x?.any.length ?? 0);
  return { leaves, nodes, top: { long: side(spec.entry, spec.exit), short: side(spec.short_side?.entry, spec.short_side?.exit) } };
}
export const limits = (cat: Catalogue) => ({ leaves: cat.max_leaves ?? 40, nodes: cat.max_nodes ?? 120, top: cat.max_conditions });
```
- `pathLabel(path, both)`: regex `/^(short_side\.)?(entry\.all|exit\.any|entry\.limit\.at|entry\.stop\.at)(?:\[(\d+)\])?(.*)$/`; base word `entry | exit | limit level | stop level`; prefix `Short ` (short_side), else `Long ` when `both`, else capitalise; append `n+1` for the list index; then each `.key` or `.key[i]` segment maps through `SEGMENT = { lhs: "left", rhs: "right", lo: "low", hi: "high", all: "all", any: "any", not: "not", when: "when", then: "step", within: "within", at_least: "at least", of: "of", bars: "bars", nth: "occurrence", value: "number", p: "", ref: "bars ago", abs: "absolute", pct: "% difference", add: "plus", sub: "minus", mul: "times", div: "divided by", min: "min", max: "max", sum: "sum", avg: "average", highest: "highest", lowest: "lowest", change: "change", bars_since: "bars since", value_when: "value when", highest_since: "highest since", lowest_since: "lowest since", count: "count", compress: "aggregate" }` (unknown keys: underscores to spaces; indexed segments get ` i+1`), empty names dropped, joined with ` › `. Unmatched paths return the path.
- `tfOk(tf, cat)`: `D`, any of `cat.timeframes`, or `/^\d+D$/` with n in `2..(cat.custom_days?.[1] ?? 63)`.
- `firstAllowed(o, compress = false)`: `(o.offset ?? 0) >= 1 || (compress ? (o.mode ?? "last") === "first" : o.s === "open")`.
- `timingProblem(o, cat, compress = false)`: bad tf → `"timeframe must be D, W, M, Q, Y or N sessions (2 to 63)."` (compress with D or a bad tf → `"the aggregate needs a timeframe of W, M, Q, Y or N sessions (2 to 63)."`); `expand` other than last on D → `"“Use the period value” applies to W, M, Q, Y and N-session values only."`; `first` not allowed → `EXPAND_FIRST_REFUSED`; else `undefined`.
- `jsonDepth(v)`: containers `1 + max(children, 0)`, scalars `1` (matches `json.rs`: 16 empty levels pass, 16 levels holding a scalar fail).
- `treeProblems(spec, cat)`: caps (`More than ${max} comparisons in the rules; the limit is ${max}.`, `More than ${max} comparisons, groups and formulas in the rules; the limit is ${max}.`), `jsonDepth(spec) > MAX_DEPTH` → `COPY.tooDeep`, then one `walkSpec` pass with the node, operand and knob messages exactly as in the test (location = `pathLabel(path, spec.direction === "both")`; "needs a series" names `OPERAND_KIND_LABELS[kindOfKey(k)]`; knob message `${label} must be a whole number from ${min} to ${max}.`; `unknown` nodes produce nothing).
- `readsSessionDates(spec)`, `usesHigherTimeframes(spec)` (a series with `tf` other than D, or a `compress`) via `walkSpec`.
- `describeOperand(o, cat)`: values/series via `operandLabel`; `ref` → `${inner} · ${n} bar(s) ago`; windows → `${sum|average|highest|lowest}(${inner}, ${n})`; arith → `(${a} ${+ − × ÷ min max} ${b})`; `pct` → `% diff(${a}, ${b})`; `abs` → `|${a}|`; `change` → `change(${a}, ${n})`; `bars_since` → `bars since (${node})`; `value_when` → `${of} when (${node})`; since → `highest|lowest of ${of} since (${node})`; `count` → `count(${node}, ${n})`; `compress` → `${inner} as ${tf} ${mode}`; unknown → `kept as is`.
- `describeNode(n, cat)`: cmp → `${l} ${OPERATOR_LABELS[op] ?? op} ${r}`; range → `${l} ${is between|is outside} ${lo} and ${hi}`; trend → `${l} ${is rising for|is falling for} ${n} bars`; `all`/`any` → `${All of|Any of} ${count}: ${children joined by "; "}`; `not` → `not (${child})`; `at_least` → `At least ${k} of ${m} bars: ${child}`; `then` → `${steps joined by " then "} within ${N} bars`; unknown → `kept as is`.

- [ ] **Step 4: Run** `npx vitest run tests/lab-rule-tree.test.ts` and `npx tsc --noEmit` — pass.
- [ ] **Step 5: Browser acceptance:** n/a (no UI).
- [ ] **Step 6: Commit (permission only):** `(lab-ui) Phase 7 rule tree model: kinds, walker, budget, labels, pre-checks`.

---

### Task 3: Execution model (`execution-model.ts`)

**Files:** Create `src/features/lab/execution-model.ts`, `tests/lab-execution-model.test.ts`.
**Depends on:** Task 1. **Backend:** contract of Tasks 7–15 (no runtime need).
**Produces (exact names):** `Direction`, `SidePre`, `directionOf`, `hasShorts`, `DEFAULT_SHORT_SIDE`, `withDirection`, `sideOf`, `EntryOrderType`, `entryOrderType`, `withEntryOrder`, `LevelSlot`, `LEVEL_SLOTS`, `LevelForm`, `levelForm`, `LEVELS`, `TP_BOUNDS`, `defaultLevel`, `levelAt`, `withLevel`, `takeProfits`, `withTakeProfits`, `ResultMode`, `modeOf`, `DEFAULT_CAPITAL`, `withMode`, `SizingKind`, `sizingKind`, `withSizingKind`, `InstrumentContext`, `instrumentContext`, `moneyBlocked`, `instrumentNotes`, `executionProblems`.

- [ ] **Step 1: Write the failing tests** (`tests/lab-execution-model.test.ts`):

```ts
import { expect, it } from "vitest";
import * as x from "../src/features/lab/execution-model";
import { COPY, FUTURES_NOTE, MODEL_NOTE, MONEY_BLOCKED } from "../src/features/lab/labels";
import { defaultSpec, setAt, setExecution } from "../src/features/lab/spec-model";
import type { Spec } from "../src/features/lab/types";
import { catalogue7Fixture as cat } from "./fixtures/lab";

const idx: x.InstrumentContext = { kind: "index", market: "in" };
const inStock: x.InstrumentContext = { kind: "individual_stock", market: "in" };
const usEtf: x.InstrumentContext = { kind: "etf", market: "us" };

it("direction: both adds a short side and keeps it across a round trip; long removes short_side and opposite", () => {
  const s = setExecution(defaultSpec(), { opposite: "exit" });
  const both = x.withDirection(s, "both");
  expect(both.direction).toBe("both");
  expect(both.short_side).toEqual(x.DEFAULT_SHORT_SIDE);
  const edited = setAt(both, "short_side.entry.all[0].rhs.p.length", 20);
  const long = x.withDirection(edited, "long");
  expect(long).not.toHaveProperty("short_side");
  expect(long).not.toHaveProperty("direction");
  expect(long.execution).toBeUndefined();
  expect(x.withDirection(long, "both", edited.short_side).short_side?.entry.all[0]).toMatchObject({ rhs: { p: { length: 20 } } });
  expect(x.withDirection(defaultSpec(), "short").direction).toBe("short");
  expect(x.hasShorts(both) && x.hasShorts(x.withDirection(s, "short")) && !x.hasShorts(s)).toBe(true);
});

it("entry orders: types, default levels, fill moved to exit_fill, per side", () => {
  const s = setExecution(defaultSpec(), { fill: "close" });
  const stop = x.withEntryOrder(s, "", "stop");
  expect(stop.entry.stop).toEqual({ pct: 1 });
  expect(stop.execution).toEqual({ exit_fill: "close" });
  expect(x.entryOrderType(stop.entry)).toBe("stop");
  const sl = x.withEntryOrder(stop, "", "stop_limit");
  expect([sl.entry.limit, sl.entry.stop, x.entryOrderType(sl.entry)]).toEqual([{ pct: 1 }, { pct: 1 }, "stop_limit"]);
  const market = x.withEntryOrder(setAt(sl, "entry.valid_bars", 3), "", "market");
  expect(market.entry).not.toHaveProperty("limit");
  expect(market.entry).not.toHaveProperty("valid_bars");
  const both = x.withDirection(defaultSpec(), "both");
  expect(x.withEntryOrder(both, "short_side.", "limit").short_side?.entry.limit).toEqual({ pct: 1 });
  expect(x.defaultLevel("entry.stop", "at", false)).toEqual({ at: { highest: { s: "high" }, bars: 5 } });
  expect(x.defaultLevel("entry.stop", "at", true)).toEqual({ at: { lowest: { s: "low" }, bars: 5 } });
  expect(x.defaultLevel("entry.limit", "at", false)).toEqual({ at: { lowest: { s: "low" }, bars: 5 } });
  expect(x.defaultLevel("exit.stop_loss", "atr", false)).toEqual({ atr: 2, atr_length: 14 });
  expect(x.defaultLevel("exit.target", "points", false)).toEqual({ points: 10 });
});

it("levels: trailing keeps its activation; breakeven after removal drops breakeven; take-profits list", () => {
  let s = x.withLevel(defaultSpec(), "", "exit.trailing", { pct: 5 });
  s = x.withLevel(s, "", "exit.trailing.activate", { pct: 2 });
  s = x.withLevel(s, "", "exit.trailing", { atr: 3, atr_length: 10 });
  expect(s.exit.trailing).toEqual({ atr: 3, atr_length: 10, activate: { pct: 2 } });
  expect(x.levelAt(s, "", "exit.trailing")).toEqual({ atr: 3, atr_length: 10 });
  expect(x.withLevel(s, "", "exit.trailing.activate", undefined).exit.trailing).toEqual({ atr: 3, atr_length: 10 });
  s = x.withLevel(s, "", "exit.breakeven.after", { pct: 2 });
  s = x.withLevel(s, "", "exit.breakeven.plus", { pct: 0.1 });
  expect(s.exit.breakeven).toEqual({ after: { pct: 2 }, plus: { pct: 0.1 } });
  expect(x.withLevel(s, "", "exit.breakeven.after", undefined).exit).not.toHaveProperty("breakeven");
  s = x.withTakeProfits(s, "", [{ pct: 3, size: 0.5 }]);
  expect(x.takeProfits(s, "")).toEqual([{ pct: 3, size: 0.5 }]);
  expect(x.withTakeProfits(s, "", []).exit).not.toHaveProperty("take_profits");
});

it("money mode: capital default, removes cost_bps; unit mode drops money-only sizing and costs", () => {
  const s = setExecution(defaultSpec(), { cost_bps: 10, sizing: { units: 5 } });
  const money = x.withMode(s, "money");
  expect(money.execution).toEqual({ capital: 1_000_000, sizing: { units: 5 } });
  expect(x.modeOf(money)).toBe("money");
  const unit = x.withMode(setExecution(money, { costs: { slippage: { bps: 5 } } }), "unit");
  expect(unit.execution).toBeUndefined();
  expect(x.withMode(setExecution(money, { sizing: { risk_pct: 1 } }), "unit").execution).toEqual({ sizing: { risk_pct: 1 } });
  expect(x.sizingKind(defaultSpec())).toBe("full");
  expect(x.withSizingKind(defaultSpec(), "risk_pct").execution).toEqual({ sizing: { risk_pct: 1 } });
  expect(x.withSizingKind(x.withSizingKind(defaultSpec(), "pct_equity"), "full").execution).toBeUndefined();
});

it("instrument context, money gating and pre-run notes mirror model.rs", () => {
  expect(x.instrumentContext("in", undefined)).toEqual({ kind: "index", market: "in" });
  expect(x.instrumentContext("us", { ticker: "SPY", label: "SPY", disabled: false, type: "etf", hasVolume: true })).toEqual({ kind: "etf", market: "us" });
  expect(x.instrumentContext("gl", { ticker: "IG:DAX", label: "DAX", disabled: false, type: "cfd", hasVolume: false })).toEqual({ kind: "cfd", market: "gl" });
  const short = x.withDirection(defaultSpec(), "short");
  expect(x.moneyBlocked(idx, defaultSpec())).toBe(MONEY_BLOCKED.index);
  expect(x.moneyBlocked({ kind: "crypto", market: "gl" }, defaultSpec())).toBe(MONEY_BLOCKED.crypto);
  expect(x.moneyBlocked({ kind: "cfd", market: "gl" }, defaultSpec())).toBe(MONEY_BLOCKED.cfd);
  expect(x.moneyBlocked(inStock, short)).toBe(MONEY_BLOCKED.indiaShort);
  expect(x.moneyBlocked(inStock, defaultSpec())).toBeNull();
  expect(x.moneyBlocked(usEtf, short)).toBeNull();
  expect(x.instrumentNotes(idx, short)).toEqual([MODEL_NOTE.index]);
  expect(x.instrumentNotes(inStock, short)).toEqual([MODEL_NOTE.indiaShort]);
  expect(x.instrumentNotes(usEtf, short)).toEqual([MODEL_NOTE.usShortUnit]);
  expect(x.instrumentNotes(usEtf, x.withMode(short, "money"))).toEqual([]);
  expect(x.instrumentNotes({ kind: "crypto", market: "gl" }, short)).toEqual([MODEL_NOTE.crypto, MODEL_NOTE.cryptoShort]);
  expect(x.instrumentNotes(usEtf, defaultSpec())).toEqual([]);
});

it("execution problems: distances, take-profits, breakeven, orders, scale-in, sizing, money, re-entry, futures", () => {
  const p = (s: Spec, ctx = usEtf) => x.executionProblems(s, cat, ctx);
  const s = defaultSpec();
  expect(p(s)).toEqual([]);
  expect(p(x.withLevel(s, "", "exit.stop_loss", { pct: 0 }))).toContain("Stop-loss: % must be above 0 and at most 99.");
  expect(p(x.withLevel(s, "", "entry.limit", { pct: 0 }))).toEqual([]);
  expect(p(x.withLevel(s, "", "exit.target", { atr: 51 }))).toContain("Target: ATR multiplier must be above 0 and at most 50.");
  expect(p(x.withLevel(s, "", "exit.target", { atr: 2, atr_length: 600 }))).toContain("Target: ATR length must be a whole number from 1 to 500.");
  expect(p(setAt(s, "exit.stop_loss", { ticks: 20 }))).toContain("Stop-loss: ticks need a futures contract, and futures contract data is not admitted.");
  expect(p(setAt(s, "exit.target", { at: { s: "high" } }))).toContain("Target: an At level works only for entry orders.");
  const both = x.withDirection(s, "both");
  expect(p(x.withLevel(both, "short_side.", "exit.stop_loss", { pct: 120 }))).toContain("Short Stop-loss: % must be above 0 and at most 99.");
  expect(p(x.withTakeProfits(s, "", [{ pct: 2, size: 0.6 }, { pct: 4, size: 0.5 }])))
    .toContain("Take-profits: sizes must add up to less than 100%; the rest exits by the other rules.");
  expect(p(x.withTakeProfits(s, "", [{ pct: 2, size: 1 }]))).toContain("Take-profit 1: size must be above 0% and below 100% of the initial position.");
  expect(p(setAt(s, "exit.breakeven", { after: { pct: 2 }, plus: { pct: 2 } }))).toContain('Breakeven: "plus" must be below "after".');
  expect(p(setAt(s, "entry.valid_bars", 3))).toContain("Order duration applies to limit and stop orders only.");
  expect(p(setAt(x.withEntryOrder(s, "", "stop"), "entry.valid_bars", 30))).toContain("Order duration must be a whole number of bars from 1 to 20.");
  expect(p(setExecution(x.withEntryOrder(s, "", "limit"), { fill: "close" }))).toContain("Remove the entry fill: limit and stop orders fill on the bars after the signal.");
  expect(p(setExecution(s, { fill: "close", delay_bars: 2 }))).toContain(COPY.delayOnlyNextOpen);
  const si = setAt(s, "entry.scale_in", { legs: [0.6, 0.6] });
  expect(p(si)).toContain("Scale-in: use 2 to 5 legs above 0% that add up to at most 100%.");
  expect(p(x.withTakeProfits(setAt(s, "entry.scale_in", { legs: [0.5, 0.5] }), "", [{ pct: 2, size: 0.5 }]))).toContain(COPY.scaleInWithTp);
  expect(p(x.withSizingKind(s, "risk_pct"))).toContain(COPY.riskNeedsStop);
  expect(p(x.withLevel(x.withSizingKind(s, "risk_pct"), "", "exit.stop_loss", { pct: 3 }))).toEqual([]);
  expect(p(setExecution(s, { sizing: { units: 10 } }))).toContain(COPY.needsMoney);
  expect(p(setExecution(s, { sizing: { pct_equity: 150 } }))).toContain("% of equity must be above 0 and at most 100.");
  const money = x.withMode(s, "money");
  expect(p(money, idx)).toContain(MONEY_BLOCKED.index);
  expect(p(x.withDirection(money, "short"))).toContain(COPY.borrowRequired);
  expect(p(setAt(x.withDirection(money, "short"), "execution.costs.borrow_pct_year", 0))).toEqual([]);
  expect(p(setExecution(money, { cost_bps: 5 }))).toContain("Money mode takes its costs from the money settings; remove the spec's fee (cost_bps).");
  expect(p(setExecution(s, { costs: { slippage: { bps: 5 } } }))).toContain("Money costs apply in money mode only.");
  expect(p(setAt(money, "execution.costs.taxes", Array(7).fill({ name: "STT", pct: 0.1, side: "both" })))).toContain("Use at most 6 taxes.");
  expect(p(setAt(money, "execution.costs.taxes", [{ name: "STT; DROP", pct: 0.1, side: "both" }]))).toContain("Tax 1: the name must be 1 to 40 letters, digits or spaces.");
  expect(p(setAt(money, "execution.costs.commission", { pct: 11 }))).toContain("Commission %: must be from 0 to 10.");
  expect(p(setExecution(s, { cooldown: { bars: 0 } }))).toContain("Cooldown must be a whole number of bars from 1 to 1000.");
  expect(p(setExecution(s, { max_trades: { count: 2, per: "D" as never } }))).toContain("Max entries: choose week, month, quarter or year.");
  expect(p(setExecution(s, { risk: { max_consecutive_losses: 3 } }))).toContain("Loss-streak pause needs both the number of losses and the pause in bars.");
  expect(p(setExecution(s, { risk: { max_drawdown_pct: 100 } }))).toContain("Drawdown stop must be above 0% and below 100%.");
  expect(p(setExecution(s, { instrument: { kind: "futures" } }))).toContain(FUTURES_NOTE);
});
```

- [ ] **Step 2: Run** `npx vitest run tests/lab-execution-model.test.ts` — fails: module missing.

- [ ] **Step 3: Implement** `src/features/lab/execution-model.ts` (imports `getAt`, `putAt`, `removeAt`, `setAt`, `setExecution` from spec-model; copy from labels; `InstrumentOption` type from `./instruments`):

```ts
export type Direction = "long" | "short" | "both";
export type SidePre = "" | "short_side.";
export const directionOf = (s: Spec): Direction => s.direction ?? "long";
export const hasShorts = (s: Spec) => directionOf(s) !== "long";
export const DEFAULT_SHORT_SIDE: ShortSide = {
  entry: { all: [{ lhs: { s: "close" }, op: "<", rhs: { s: "ema", p: { length: 50 } } }] },
  exit: { any: [{ lhs: { s: "bars_held" }, op: ">=", rhs: { value: 5 } }] },
};
const omit = <T extends object>(o: T, ...keys: string[]): T => { const c = { ...o } as Record<string, unknown>; for (const k of keys) delete c[k]; return c as T; };
export function withDirection(spec: Spec, dir: Direction, kept?: ShortSide): Spec {
  let s = omit(spec, "direction", "short_side");
  if (dir !== "both") s = setExecution(s, { opposite: undefined });
  if (dir === "short") return { ...s, direction: "short" };
  if (dir === "both") return { ...s, direction: "both", short_side: structuredClone(spec.short_side ?? kept ?? DEFAULT_SHORT_SIDE) };
  return s;
}
export const sideOf = (s: Spec, pre: SidePre): { entry: EntrySpec; exit: ExitSpec } | undefined => (pre ? s.short_side : s);
export type EntryOrderType = "market" | "limit" | "stop" | "stop_limit";
export const entryOrderType = (e: EntrySpec): EntryOrderType => (e.limit && e.stop ? "stop_limit" : e.stop ? "stop" : e.limit ? "limit" : "market");
export function withEntryOrder(spec: Spec, pre: SidePre, type: EntryOrderType): Spec {
  const e = sideOf(spec, pre)!.entry;
  let entry = omit(e, "limit", "stop", "valid_bars");
  if (type === "limit" || type === "stop_limit") entry = { ...entry, limit: e.limit ?? { pct: 1 } };
  if (type === "stop" || type === "stop_limit") entry = { ...entry, stop: e.stop ?? { pct: 1 } };
  if (type !== "market" && e.valid_bars !== undefined) entry = { ...entry, valid_bars: e.valid_bars };
  let s = setAt(spec, `${pre}entry`, entry);
  if (type !== "market") { // the engine refuses execution.fill with an order; fill also set exits, so keep it as exit_fill
    const ex = s.execution;
    s = setExecution(s, { fill: undefined, ...(ex?.fill !== undefined && ex.exit_fill === undefined ? { exit_fill: ex.fill } : {}) });
  }
  return s;
}
export type LevelSlot = "entry.limit" | "entry.stop" | "exit.stop_loss" | "exit.target" | "exit.trailing" | "exit.trailing.activate" | "exit.breakeven.after" | "exit.breakeven.plus";
export const LEVEL_SLOTS: LevelSlot[] = ["entry.limit", "entry.stop", "exit.stop_loss", "exit.target", "exit.trailing", "exit.trailing.activate", "exit.breakeven.after", "exit.breakeven.plus"];
export interface LevelBounds { label: string; maxPct: number; zeroOk: boolean; at: boolean; pct: number }
/** rules.rs SLOTS and dist() bounds: entry orders may sit at the signal close (0); exits need > 0. */
export const LEVELS: Record<LevelSlot, LevelBounds> = {
  "entry.limit": { label: "Limit", maxPct: 50, zeroOk: true, at: true, pct: 1 },
  "entry.stop": { label: "Stop", maxPct: 50, zeroOk: true, at: true, pct: 1 },
  "exit.stop_loss": { label: "Stop-loss", maxPct: 99, zeroOk: false, at: false, pct: 5 },
  "exit.target": { label: "Target", maxPct: 1000, zeroOk: false, at: false, pct: 10 },
  "exit.trailing": { label: "Trailing stop", maxPct: 99, zeroOk: false, at: false, pct: 5 },
  "exit.trailing.activate": { label: "Trailing activation", maxPct: 1000, zeroOk: false, at: false, pct: 2 },
  "exit.breakeven.after": { label: "Breakeven after", maxPct: 1000, zeroOk: false, at: false, pct: 2 },
  "exit.breakeven.plus": { label: "Breakeven plus", maxPct: 1000, zeroOk: false, at: false, pct: 0.1 },
};
export const TP_BOUNDS: LevelBounds = { label: "Take-profit", maxPct: 1000, zeroOk: false, at: false, pct: 5 };
export type LevelForm = "pct" | "atr" | "points" | "at";
export const levelForm = (d: unknown): LevelForm | undefined =>
  typeof d === "object" && d !== null ? (["at", "pct", "atr", "points"] as const).find((k) => k in d) : undefined;
export function defaultLevel(slot: LevelSlot | "take_profit", form: LevelForm, short: boolean): EntryLevel {
  if (form === "atr") return { atr: 2, atr_length: 14 };
  if (form === "points") return { points: 10 };
  if (form === "at") return (slot === "entry.stop") !== short ? { at: { highest: { s: "high" }, bars: 5 } } : { at: { lowest: { s: "low" }, bars: 5 } };
  return { pct: slot === "take_profit" ? TP_BOUNDS.pct : LEVELS[slot].pct };
}
/** The level at a slot; the trailing distance is returned without its activation. */
export function levelAt(s: Spec, pre: SidePre, slot: LevelSlot): EntryLevel | undefined {
  const v = getAt(s, pre + slot) as Record<string, unknown> | undefined;
  return (slot === "exit.trailing" && v ? omit(v, "activate") : v) as EntryLevel | undefined;
}
export function withLevel(s: Spec, pre: SidePre, slot: LevelSlot, d: EntryLevel | undefined): Spec {
  const path = pre + slot;
  if (slot === "exit.trailing" && d) { const act = getAt(s, `${path}.activate`); return setAt(s, path, act ? { ...d, activate: act } : d); }
  if (slot === "exit.breakeven.after" && !d) return removeAt(s, `${pre}exit.breakeven`);
  return putAt(s, path, d);
}
export const takeProfits = (s: Spec, pre: SidePre): TakeProfit[] => (getAt(s, `${pre}exit.take_profits`) as TakeProfit[] | undefined) ?? [];
export const withTakeProfits = (s: Spec, pre: SidePre, list: TakeProfit[]) => putAt(s, `${pre}exit.take_profits`, list.length ? list : undefined);
export type ResultMode = "unit" | "money";
export const modeOf = (s: Spec): ResultMode => (s.execution?.capital !== undefined ? "money" : "unit");
export const DEFAULT_CAPITAL = 1_000_000;
export function withMode(s: Spec, mode: ResultMode): Spec {
  if (mode === "money") return setExecution(s, { capital: s.execution?.capital ?? DEFAULT_CAPITAL, cost_bps: undefined });
  const z = s.execution?.sizing;
  return setExecution(s, { capital: undefined, costs: undefined, sizing: z && ("units" in z || "value" in z) ? undefined : z });
}
export type SizingKind = "full" | "pct_equity" | "risk_pct" | "units" | "value";
export const sizingKind = (s: Spec): SizingKind => { const z = s.execution?.sizing; return z ? ((["pct_equity", "risk_pct", "units", "value"] as const).find((k) => k in z) ?? "full") : "full"; };
const SIZING_DEFAULT: Record<Exclude<SizingKind, "full">, Sizing> = { pct_equity: { pct_equity: 100 }, risk_pct: { risk_pct: 1 }, units: { units: 100 }, value: { value: 100_000 } };
export const withSizingKind = (s: Spec, k: SizingKind) => setExecution(s, { sizing: k === "full" ? undefined : SIZING_DEFAULT[k] });
export interface InstrumentContext { kind: string; market: Market }
/** in/us rows without a native asset type are indices (instruments.ts); gl rows carry their kind (index | cfd | crypto). */
export const instrumentContext = (market: Market, option?: InstrumentOption): InstrumentContext => ({ market, kind: option?.type ?? (market === "gl" ? "unknown" : "index") });
export function moneyBlocked(ctx: InstrumentContext, spec: Spec): string | null {
  if (ctx.kind === "index") return MONEY_BLOCKED.index;
  if (ctx.kind === "crypto") return MONEY_BLOCKED.crypto;
  if (ctx.kind === "cfd") return MONEY_BLOCKED.cfd;
  return ctx.market === "in" && hasShorts(spec) ? MONEY_BLOCKED.indiaShort : null;
}
/** Pre-run mirror of lab/model.rs model(); the response's execution_model replaces it after a run. */
export function instrumentNotes(ctx: InstrumentContext, spec: Spec): string[] {
  const shorts = hasShorts(spec);
  if (ctx.kind === "index") return [MODEL_NOTE.index];
  if (ctx.kind === "crypto") return shorts ? [MODEL_NOTE.crypto, MODEL_NOTE.cryptoShort] : [MODEL_NOTE.crypto];
  if (ctx.kind === "cfd") return [MODEL_NOTE.cfd];
  if (shorts && ctx.market === "in") return [MODEL_NOTE.indiaShort];
  return shorts && modeOf(spec) === "unit" ? [MODEL_NOTE.usShortUnit] : [];
}
```
`executionProblems(spec, cat, ctx)` — one function, messages exactly as in the test, in this order: per side (`""`, and `"short_side."` when Both; label prefix `"Short "` for the short side, `"Long "` for the long side when Both, else none): each `LEVEL_SLOTS` level through `levelProblems(label, d, bounds)` (pct → `"${label}: % must be ${zeroOk ? "from 0" : "above 0"} and at most ${maxPct}."`; points → `"${label}: points must be ${…} and at most 1,000,000,000."`; atr → the two ATR messages; `at` on a non-entry slot → `"${label}: an At level works only for entry orders."`; `ticks` → `"${label}: ticks need a futures contract, and futures contract data is not admitted."`; none of these → `"${label}: choose %, ATR or points."`); take-profits (count `"Take-profits: use at most ${max} levels."`, sizes, sum, each level with `TP_BOUNDS` and label `Take-profit k`); breakeven plus vs after (same units: pct/pct, points/points, atr/atr with the same `atr_length ?? 14`); `valid_bars`; `min_bars` (`"Minimum bars must be a whole number from 0 to 1000."`); scale-in (legs rule, `min_bars` 1..1000 → `"Scale-in: bars between legs must be a whole number from 1 to 1000."`, `COPY.scaleInWithTp`). Then shared: fill-with-order, delay rule (`COPY.delayOnlyNextOpen`, or `"Fill delay must be a whole number of bars from 1 to ${max}."`), sizing (`pct_equity`/`risk_pct` bounds `"% of equity must be above 0 and at most 100."` / `"Risk per trade must be above 0% and at most 100%."`, `COPY.riskNeedsStop` when any side lacks `exit.stop_loss`, `COPY.needsMoney`, `"Shares must be a whole number of at least 1."`, `"Position value must be above 0."`), money (`moneyBlocked`, capital `"Capital must be above 0 and at most 1,000,000,000,000."`, cost_bps message, commission `"Commission per order: must be from 0 to 1,000,000,000."` / `"Commission per share: must be from 0 to 1,000,000."` / `"Commission %: must be from 0 to 10."`, slippage `"Slippage: bps must be from 0 to 1000."`, `"Use at most 6 taxes."`, per tax name `/^[A-Za-z0-9 ]{1,40}$/` with at least one letter → `"Tax ${k}: the name must be 1 to 40 letters, digits or spaces."`, pct 0..100 → `"Tax ${k}: % must be from 0 to 100."`, `COPY.borrowRequired` for US shorts, borrow 0..100 → `"Borrow cost must be from 0% to 100% a year."`), or `"Money costs apply in money mode only."` in unit mode; re-entry (`"Cooldown must be a whole number of bars from 1 to 1000."`, after ∉ any/loss/win → `"Cooldown: choose after any trade, a loss or a win."`, `"Max entries must be a whole number from 1 to 1000."`, `"Max entries: choose week, month, quarter or year."`); risk (`"Loss-streak pause needs both the number of losses and the pause in bars."`, losses 1..1000 / pause 1..100000 → `"Loss-streak pause: losses 1 to 1000, pause 1 to 100000 bars."`, `"Drawdown stop must be above 0% and below 100%."`); `getAt(spec, "execution.instrument.kind") === "futures"` → `FUTURES_NOTE`. Limits come from `cat.execution` with fallbacks 3/5/20/10.

- [ ] **Step 4: Run** `npx vitest run tests/lab-execution-model.test.ts` and `npx tsc --noEmit` — pass.
- [ ] **Step 5: Browser acceptance:** n/a (no UI).
- [ ] **Step 6: Commit (permission only):** `(lab-ui) Phase 7 execution model: sides, orders, exits, sizing, money, instrument notes`.

---

### Task 4: Browser check — Phase 7 scenario, spacing, compactness and theme checks

**Files:** Modify `scripts/lab-browser-check.mjs`.
**Depends on:** nothing (codes against the selector contract). **Backend:** the live run checks need backend Tasks 7–12 (see check list).

- [ ] **Step 1: Arguments.** `node scripts/lab-browser-check.mjs <base> <out-dir> [all|legacy|phase7]` (default `all` = legacy + phase7). Keep every legacy check. Fix `optimizeCompact` to read `.lab-dims tbody tr[data-lab-dim]` (Task 10 adds group header rows).

- [ ] **Step 2: Helpers** (add next to `click`):

```js
// `el` is a JS expression evaluated in the page that yields the element (so fallbacks and scoped lookups work).
const q = (sel) => `document.querySelector(${JSON.stringify(sel)})`;
const firstOf = (...sels) => `(${JSON.stringify(sels)}).map((s) => document.querySelector(s)).find(Boolean)`;
const chooseEl = (el, value) => evaluate(`(() => { const e = ${el}; if (!e || e.disabled) return false;
  Object.getOwnPropertyDescriptor(HTMLSelectElement.prototype, 'value').set.call(e, ${JSON.stringify(value)});
  e.dispatchEvent(new Event('change', { bubbles: true })); return true; })()`);
const choose = (sel, value) => chooseEl(q(sel), value);
const typeInto = (sel, value) => evaluate(`(() => { const e = ${q(sel)}; if (!e) return false;
  Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set.call(e, ${JSON.stringify(String(value))});
  e.dispatchEvent(new Event('input', { bubbles: true })); return true; })()`);
const clickText = (sel, text) => evaluate(`(() => { const b = [...document.querySelectorAll(${JSON.stringify(sel)})].find((e) => e.textContent.trim() === ${JSON.stringify(text)});
  if (!b || b.disabled) return false; b.click(); return true; })()`);
```
Step 4's "first operand type select" is `chooseEl(\`(${firstOf('[data-lab-list="entry.all"]', '.lab-cond')})?.querySelector('select[aria-label="Operand type"]')\`, 'ref')`.

- [ ] **Step 3: Measurements** (strings evaluated in the page):

```js
const gapsWithin = (sel, max) => `(() => { const box = document.querySelector(${JSON.stringify(sel)}); if (!box) return false;
  const k = [...box.children].filter((e) => e.offsetHeight > 0);
  return k.slice(1).every((e, i) => { const g = e.getBoundingClientRect().top - k[i].getBoundingClientRect().bottom; return g >= -1 && g <= ${max}; }); })()`;
const compactControls = `[...document.querySelectorAll('.lab-rules select, .lab-rules input:not([type=checkbox]):not([type=radio])')].every((e) => e.offsetHeight === 0 || e.offsetHeight <= 32)`;
const indentOk = `[...document.querySelectorAll('.lab-group-body')].every((b) => { const s = getComputedStyle(b); return parseFloat(s.paddingLeft) + parseFloat(s.marginLeft) <= 16; })`;
const leafTwoLines = `[...document.querySelectorAll('[data-lab-node][data-lab-kind=cmp]')].every((c) => c.getBoundingClientRect().height < 90)`;
const noWhite = `[...document.querySelectorAll('.lab-page .box, .lab-page select, .lab-page input, .lab-page details')].every((e) => getComputedStyle(e).backgroundColor !== 'rgb(255, 255, 255)')`;
const noEmptyColumns = `[...document.querySelectorAll('.lab-page table')].filter((t) => t.offsetHeight > 0).every((t) => {
  const rows = [...t.tBodies].flatMap((b) => [...b.rows]).filter((r) => !r.classList.contains('lab-dim-group')); if (!rows.length) return true;
  const n = Math.max(...rows.map((r) => r.cells.length));
  for (let c = 0; c < n; c++) if (rows.every((r) => !r.cells[c] || (['', '—'].includes(r.cells[c].textContent.trim()) && !r.cells[c].querySelector('input,button,select,details')))) return false;
  return true; })`;
```

- [ ] **Step 4: Phase 7 scenario** over `layoutViews` (1920/1425/400 × steel/midnight). Per view: navigate to `#/indicators?tk=NIFTY%2050`, reload, wait for library + `.lab-rules`, apply theme, then record each check (a failed step never aborts the view; it records `false` and continues):
  1. `p7BothPanel`: `clickText('[role=group][aria-label="Direction"] button', 'Both')` and `[data-lab-side="short"]` exists.
  2. `p7GroupAdded`: `choose('[data-lab-list="entry.all"] select[data-lab-add]', 'any')` and `[data-lab-list="entry.all"] [data-lab-kind="any"]` exists.
  3. `p7RangeOp`: choose `'between'` on `select[aria-label="Operator"]` inside the first `[data-lab-list="entry.all"] [data-lab-kind=cmp]`; then `[data-lab-kind="range"]` exists.
  4. `p7Formula`: choose `'ref'` on the first `select[aria-label="Operand type"]` of `firstOf('[data-lab-list="entry.all"]', '.lab-cond')`; then `.lab-operand[data-lab-operand-kind="ref"]` exists.
  5. `p7StopOrder`: `clickText('[role=group][aria-label="Long entry order"] button', 'Stop')`; then `[data-lab-orders="long"]` shows a select with value `pct` for the stop level.
  6. `p7TakeProfit`: `choose('select[aria-label="Add exit (long)"]', 'take_profits')` and `[data-lab-orders="long"] [data-lab-exit="take_profits"]` exists.
  7. `p7RiskOpen`: click `details[data-lab-risk] > summary`; `details[data-lab-risk]` has `open`.
  8. `p7MoneyBlockedOnIndex`: the `Money` button in `[role=group][aria-label="Results in"]` is disabled and `[data-lab-money-blocked]` contains "an index is never priced".
  9. `p7Caps`: `[data-lab-caps]` text matches `/comparisons \d+\/40/`.
  10. Layout: `p7RulesFullWidth` (rules/main width ratio ≥ 0.9), `p7NoOverflow` (existing `bad`), `p7PageGaps` (existing `gapsOk`), `p7RulesRhythm` (`gapsWithin('.lab-rules', 15)`), `p7SideRhythm` (`gapsWithin('[data-lab-side="long"]', 11)`), `p7CompactControls`, `p7Indent`, `p7LeafTwoLines` (only width ≥ 1400), `p7NoWhiteInMidnight` (only midnight views), `p7LibraryClosed` (existing check). Screenshot `p7/p7-prerun-<view>.png`.
  11. Run: click `[data-lab-action="run"]`, wait ≤ 120 s for `[data-lab-result], .lab-two [data-lab-error]`. Checks: `p7Run` (`[data-lab-result]`), `p7IndexNote` (`[data-lab-execution-notes]` contains "An index cannot be traded"), `p7ShortTrades` (`table[data-lab-trades]` has a cell "Short"), `p7NoEmptyColumns`, `p7PostNoOverflow`, `p7PostGaps`. When `p7Run` is false, store `document.querySelector('.lab-rules .rnote')?.innerText` and the error text in the report. Screenshot `p7/p7-postrun-<view>.png`.
  12. Only on `1920-steel`: fresh load; set the first entry lhs to Calendar `sessions_left.m` (`choose` Operand type `calendar`, then `select[aria-label="Calendar value"]` `sessions_left.m`), rhs to Number 5, operator `<`; run; `p7Adaptations` = `[data-lab-adaptations]` contains "sessions_left". Then add an Any group, tick `tr[data-lab-dim="entry.all[1].any[0].rhs.p.length"] input[type=checkbox]`, run optimization (wait ≤ 300 s), `p7OptimizeNested` = `[data-lab-optimize-result]` exists. Screenshots `p7/p7-adaptations-1920-steel.png`, `p7/p7-optimize-1920-steel.png`.

- [ ] **Step 5: Verify** `node --check scripts/lab-browser-check.mjs`; then, if Vite 5173 is up, `node scripts/lab-browser-check.mjs http://127.0.0.1:5173 "$SCRATCH/t4" phase7` must finish without throwing (most p7 checks are `false` before waves 3–4; the report must list them, not crash). Legacy: `… legacy` exits as before.
- [ ] **Step 6: Commit (permission only):** `(lab-ui) browser check: Phase 7 scenario, spacing, compactness and theme checks`.

---

### Task 5: Operand editor v2 (expressions, state operands, timeframes, calendar)

**Files:** Modify `src/features/lab/OperandEditor.tsx`; create `src/features/lab/operand.css`, `tests/lab-operand-editor.test.tsx`.
**Depends on:** Tasks 1, 2. **Backend:** none (batches 1–2).
**Interface (additive; existing props unchanged, `NumField` unchanged):**

```ts
export function OperandEditor(props: {
  operand: Operand; side: Side; catalogue: Catalogue; onChange(o: Operand): void; label: string;
  /** bars_held is offered only here (top-level exit comparison). Default: side === "exit" (v1 behaviour). */
  heldOk?: boolean;
  /** Renders the condition of a state operand; without it the condition is shown read-only (describeNode). */
  renderNode?: (node: RuleNode, onChange: (n: RuleNode) => void, label: string) => ReactNode;
  /** At the rule-parts cap: formula kinds are disabled (the current kind stays selectable). */
  nodesFull?: boolean;
})
```

- [ ] **Step 1: Write the failing tests** (`tests/lab-operand-editor.test.tsx`; mount helpers from fixtures, `catalogue7Fixture`; a `choose(select, value)` helper like the rule-builder test):

```tsx
it("offers grouped kinds; bars held only when allowed; formula kinds disabled at the cap", async () => {
  const { host } = await setup({ s: "close" }, { side: "entry" });
  const kind = host.querySelector<HTMLSelectElement>('select[aria-label="Operand type"]')!;
  expect([...kind.querySelectorAll("optgroup")].map((g) => g.label)).toEqual(["Formula", "Since a condition"]);
  expect(kind.querySelector('option[value="bars_held"]')).toBeNull();
  const held = await setup({ s: "close" }, { side: "exit", heldOk: true });
  expect(held.host.querySelector('option[value="bars_held"]')).not.toBeNull();
  const full = await setup({ s: "close" }, { nodesFull: true });
  expect(full.host.querySelector<HTMLOptionElement>('option[value="ref"]')!.disabled).toBe(true);
});

it("wraps into N bars ago, swaps to a rolling window keeping the inner value, edits bars, and unwraps", async () => {
  const { host, last } = await setup({ s: "rsi", p: { length: 2 } });
  await choose(host.querySelector('select[aria-label="Operand type"]')!, "ref");
  expect(last()).toEqual({ ref: { s: "rsi", p: { length: 2 } }, bars: 1 });
  const ref = await setup({ ref: { s: "rsi", p: { length: 2 } }, bars: 1 });
  expect(ref.host.querySelector(".lab-operand")!.getAttribute("data-lab-operand-kind")).toBe("ref");
  await choose(ref.host.querySelector('select[aria-label="Operand type"]')!, "window");
  expect(ref.last()).toEqual({ avg: { s: "rsi", p: { length: 2 } }, bars: 10 });
  const win = await setup({ avg: { s: "close" }, bars: 10 });
  await choose(win.host.querySelector('select[aria-label="Window"]')!, "highest");
  expect(win.last()).toEqual({ highest: { s: "close" }, bars: 10 });
  await type(win.host.querySelector('input[aria-label="bars"]')!, "20");
  expect(win.last()).toEqual({ avg: { s: "close" }, bars: 20 });
  await click(win.host.querySelector('button[aria-label="Remove formula"]')!);
  expect(win.last()).toEqual({ s: "close" });
});

it("arithmetic: operator select and two nested operands", async () => {
  const { host, last } = await setup({ sub: [{ s: "close" }, { value: 1 }] });
  await choose(host.querySelector('select[aria-label="Arithmetic"]')!, "div");
  expect(last()).toEqual({ div: [{ s: "close" }, { value: 1 }] });
  expect(host.querySelectorAll(".lab-operand .lab-operand")).toHaveLength(2);
});

it("state operands render their condition through renderNode, or read-only without it", async () => {
  const renderNode = vi.fn(() => <span data-testid="node" />);
  const { host } = await setup({ bars_since: DEFAULT_STATE_COND }, { renderNode });
  expect(renderNode).toHaveBeenCalledWith(DEFAULT_STATE_COND, expect.any(Function), expect.stringContaining("condition"));
  expect(host.querySelector('[data-testid="node"]')).not.toBeNull();
  const plain = await setup({ bars_since: DEFAULT_STATE_COND });
  expect(plain.host.textContent).toContain("close crosses above Exponential Moving Average(50)");
});

it("timing stays folded until needed; N sessions; expand first is offered only where the engine accepts it", async () => {
  const { host, last } = await setup({ s: "close" });
  expect(host.querySelector('select[aria-label="Timeframe"]')).toBeNull();
  await click(host.querySelector("button[data-lab-timing]")!);
  await choose(host.querySelector('select[aria-label="Timeframe"]')!, "nD");
  expect(last()).toEqual({ s: "close", tf: "5D" });
  const w = await setup({ s: "close", tf: "W" });
  expect(w.host.querySelector("button[data-lab-timing]")!.textContent).toContain("Weekly");
  await click(w.host.querySelector("button[data-lab-timing]")!);
  const expand = w.host.querySelector<HTMLSelectElement>('select[aria-label="Use the period value"]')!;
  expect(expand.querySelector<HTMLOptionElement>('option[value="first"]')!.disabled).toBe(true);
  expect(w.host.textContent).toContain(EXPAND_FIRST_REFUSED);
  const open = await setup({ s: "open", tf: "W" });
  await click(open.host.querySelector("button[data-lab-timing]")!);
  expect(open.host.querySelector<HTMLOptionElement>('option[value="first"]')!.disabled).toBe(false);
  const shifted = await setup({ s: "close", tf: "W", offset: 1, expand: "first" });
  await click(shifted.host.querySelector("button[data-lab-timing]")!);
  await type(shifted.host.querySelector('input[aria-label="periods ago"]')!, "0");
  expect(shifted.last()).toEqual({ s: "close", tf: "W" }); // first is not valid at offset 0, so it is dropped
});

it("calendar: one select with plain labels writes id and output", async () => {
  const { host, last } = await setup({ s: "close" });
  await choose(host.querySelector('select[aria-label="Operand type"]')!, "calendar");
  expect(last()).toEqual({ s: "session", out: "tdom" });
  const cal = await setup({ s: "session", out: "tdom" });
  const sel = cal.host.querySelector<HTMLSelectElement>('select[aria-label="Calendar value"]')!;
  expect([...sel.options].map((o) => o.textContent)).toContain("Sessions left in the month · future dates");
  await choose(sel, "sessions_left.m");
  expect(cal.last()).toEqual({ s: "sessions_left", out: "m" });
});

it("compress: timeframe without D, mode, and first-expand only with mode first or offset", async () => {
  const { host, last } = await setup({ compress: { s: "high" }, tf: "W", mode: "last" });
  const tf = host.querySelector<HTMLSelectElement>('select[aria-label="Aggregate timeframe"]')!;
  expect([...tf.options].some((o) => o.value === "D")).toBe(false);
  await choose(host.querySelector('select[aria-label="Aggregate"]')!, "first");
  expect(last()).toEqual({ compress: { s: "high" }, tf: "W", mode: "first" });
});

it("unknown operand shapes stay read-only", async () => {
  const { host } = await setup({ fancy: { k: 1 } } as never);
  expect(host.querySelector(".lab-kept")?.textContent).toContain('{"fancy":{"k":1}}');
});
```
(`setup(operand, extra)` mounts `<OperandEditor operand side="entry" catalogue={catalogue7Fixture} onChange label="Entry 1 left" {...extra}/>` and exposes `last()`; `type` sets an input value through the native setter and dispatches `input`; `click` wraps `.click()` in `act`.)

- [ ] **Step 2: Run** `npx vitest run tests/lab-operand-editor.test.tsx` — fails.

- [ ] **Step 3: Implement.** Keep `NumField`, `IndicatorFields` and the price/indicator/number paths. New structure:
  - `kind = operandKind(operand, catalogue)`; `unknown` → the existing read-only "kept as is" span.
  - Root `<span className="lab-operand" data-lab-operand-kind={kind} role="group" aria-label={label}>`; formula and state kinds add class `lab-expr`.
  - Kind select (`aria-label="Operand type"`): options price, indicator, calendar (disabled when no `CALENDAR_IDS` indicator is computed), number, bars_held (only when `heldOk ?? side === "exit"`, or when it is the current kind); `<optgroup label="Formula">` arith, pct, abs, ref, window, change, compress; `<optgroup label="Since a condition">` bars_since, value_when, since, count. Labels from `OPERAND_KIND_LABELS`. Formula/state options are `disabled` when `nodesFull` and not current. Change → `onChange(convertOperand(operand, k, catalogue))`.
  - Per kind (connector words are `<span className="lab-word">`): `arith` → child A, `select aria-label="Arithmetic"` (plus, minus, times, divided by, min, max → key swap keeping the pair), child B; `pct` → "% difference of" A "vs" B; `abs` → "|" A "|"; `ref` → A "," `NumField label="bars"` (0..1000, whole) "bars ago"; `window` → `select aria-label="Window"` (sum, average, highest, lowest) "of" A "over" `NumField label="bars"` (1..1000); `change` → "change of" A "over" `NumField label="bars"`; `bars_since` → "bars since" node; `value_when` → A "when" node "occurrence" `NumField label="occurrence"` (1..20); `since` → `select aria-label="Highest or lowest"` (highest_since / lowest_since key swap) "of" A "since" node; `count` → "bars in the last" `NumField label="bars"` "where" node; `compress` → A "as" `select aria-label="Aggregate timeframe"` (`catalogue.timeframes` minus D, plus "Every N sessions" = `nD` revealing `NumField label="sessions"` 2..`custom_days[1]`) `select aria-label="Aggregate"` (last, first, high, low, sum) + the timing toggle for its `offset`/`expand`.
  - Children are recursive `<OperandEditor>` with `label={`${label} › …`}`, `heldOk={false}`, the same `renderNode` and `nodesFull`. Nodes: `renderNode ? renderNode(node, set, `${label} › condition`) : <span className="lab-kept">{describeNode(node, catalogue)}</span>`.
  - Every formula/state kind ends with `<button type="button" className="dbtn sm ghost" aria-label="Remove formula" title="Remove formula, keep the inner value">×</button>` → `onChange(innerOperand(operand) ?? { s: "close" })`.
  - Calendar: one `select aria-label="Calendar value"` listing `CALENDAR_OUTPUTS` whose `id` is computed in the catalogue, `value="${id}.${out}"`, change → `{ ...timing, s: id, out }`.
  - Timing for price, indicator, calendar and compress: `<button type="button" className="dbtn sm ghost" data-lab-timing aria-expanded aria-label="Timeframe and offset">` showing `⋯` when default, else a summary (`TF_LABELS[tf] ?? "Every N sessions"`, `· n bars|periods ago`, `· period's last day` / `· from the period's first day`). Open state: `useState(false)`. Opened: `select aria-label="Timeframe"` (D, `catalogue.timeframes`, plus `nD` "Every N sessions…" → default `5D`, revealing `NumField label="sessions"` 2..63), `select aria-label="Use the period value"` only when tf ≠ D (`last` "after the period closes", `point` "on the period's last day only", `first` "from the period's first day", the latter `disabled` unless `firstAllowed`; when disabled, a `<p className="cap">` shows `EXPAND_FIRST_REFUSED`), `NumField` labelled "bars ago" (D) or "periods ago" (other tf), 0..1000 whole; setting 0 when `expand === "first"` and not `firstAllowed` drops `expand`. Defaults remove keys (tf D, offset 0, expand last).
  - `operand.css`: `.lab-expr { display: inline-flex; flex-wrap: wrap; gap: 4px 6px; align-items: center; padding: 2px 6px; border: 1px solid var(--rule); border-radius: var(--r-field); background: var(--panel); max-width: 100%; }` `.lab-word { font-size: 12px; color: var(--ink2); }` `.lab-timing { display: inline-flex; flex-wrap: wrap; gap: 4px 6px; align-items: center; }` `.lab-operand .lab-kept { font-size: 12px; }` `@media (max-width: 600px) { .lab-expr { padding: 2px 4px; } }`.

- [ ] **Step 4: Run** `npx vitest run tests/lab-operand-editor.test.tsx` and `npx tsc --noEmit` — pass. Existing `tests/lab-rule-builder.test.tsx` cases for operands still pass (the timeframe select now sits behind the timing toggle: the test "shows … catalogue timeframes" is Task 11's to update; report it if it fails).
- [ ] **Step 5: Browser acceptance** (operands are on the page through today's rule builder): run `node scripts/lab-browser-check.mjs <base> "$SCRATCH/t5" phase7`. Required: `p7Formula`, `p7NoOverflow`, `p7CompactControls`, `p7NoWhiteInMidnight` on all six views. Open `p7-prerun-{1920,1425,400}-{steel,midnight}.png`: the `ref` formula sits inline in the first condition as a bordered group in theme colours, no white boxes in Midnight, at 400 px it wraps inside the box.
- [ ] **Step 6: Commit (permission only):** `(lab-ui) operand editor: formulas, state operands, higher timeframes, calendar`.

---

### Task 6: Rule tree editor (`NodeEditor.tsx`)

**Files:** Create `src/features/lab/NodeEditor.tsx`, `src/features/lab/tree.css`, `tests/lab-node-editor.test.tsx`.
**Depends on:** Tasks 1, 2 (renders `OperandEditor`; `renderNode` is passed and takes effect once Task 5 lands). **Backend:** none.
**Interface:**

```ts
export interface TreeCaps { leaves: boolean; nodes: boolean; top: boolean } // true = at that cap
export function RuleList(props: {
  list: RuleNode[]; onChange(list: RuleNode[]): void; listPath: ListPath; title: string; tag: string; // tag: "Entry", "Short exit", …
  catalogue: Catalogue; caps: TreeCaps;
}): JSX.Element
export function NodeEditor(props: {
  node: RuleNode; onChange(n: RuleNode): void; onRemove?(): void; onMove?(d: -1 | 1): void; canUp?: boolean; canDown?: boolean;
  catalogue: Catalogue; side: Side; top: boolean; caps: TreeCaps; label: string;
}): JSX.Element
```

- [ ] **Step 1: Write the failing tests** (`tests/lab-node-editor.test.tsx`; mount `RuleList` with `catalogue7Fixture`, caps all false, `listPath` `entry.all` unless stated):

```tsx
it("renders every node kind with its kind attribute", async () => {
  const list: RuleNode[] = [C, { lhs: { s: "rsi" }, op: "between", lo: { value: 30 }, hi: { value: 70 } }, { lhs: { s: "close" }, op: "rising", bars: 3 },
    { all: [C] }, { any: [C] }, { not: C }, { at_least: 2, of: 3, when: C }, { then: [C, C], within: 5 }];
  const { host } = await setup(list);
  expect([...host.querySelectorAll("[data-lab-list='entry.all'] > [data-lab-node]")].map((n) => n.getAttribute("data-lab-kind")))
    .toEqual(["cmp", "range", "trend", "all", "any", "not", "at_least", "then"]);
});

it("the add menu appends each kind and resets", async () => {
  const { host, onChange } = await setup([C]);
  const add = host.querySelector<HTMLSelectElement>("[data-lab-list='entry.all'] > select[data-lab-add]")!;
  for (const k of ["cond", "all", "any", "not", "at_least", "then"]) await choose(add, k);
  // the list prop is static (onChange is a spy), so every call appends to [C]
  expect(onChange.mock.calls.map(([l]) => (l as RuleNode[]).map(nodeKind)[1])).toEqual(["cmp", "all", "any", "not", "at_least", "then"]);
  expect(add.value).toBe("");
});

it("the operator select switches comparison, range and trend", async () => {
  const { host, last } = await setup([C]);
  await choose(host.querySelector('select[aria-label="Operator"]')!, "between");
  expect(nodeKind(last()[0])).toBe("range");
  const r = await setup([{ lhs: { s: "rsi" }, op: "between", lo: { value: 30 }, hi: { value: 70 } }]);
  await choose(r.host.querySelector('select[aria-label="Operator"]')!, "rising");
  expect(r.last()[0]).toEqual({ lhs: { s: "rsi" }, op: "rising", bars: 3 });
});

it("node menu wraps, negates, unwraps and moves; remove deletes", async () => {
  const { host, last } = await setup([C, { lhs: { s: "close" }, op: "<", rhs: { value: 5 } }]);
  const menu = () => host.querySelectorAll<HTMLSelectElement>("select[data-lab-node-menu]");
  await choose(menu()[0], "wrap_any");
  expect(last()[0]).toEqual({ any: [C] });
  await choose(menu()[0], "wrap_not");
  expect(last()[0]).toEqual({ not: C });
  await choose(menu()[1], "up");
  expect(last()[0]).toMatchObject({ rhs: { value: 5 } });
  expect([...menu()[0].options].find((o) => o.value === "up")!.disabled).toBe(true);
  await click(host.querySelector('button[aria-label="Remove Entry 1"]')!);
  expect(last()).toHaveLength(1);
  const g = await setup([{ all: [C] }]);
  await choose(g.host.querySelector("select[data-lab-node-menu]")!, "unwrap");
  expect(g.last()[0]).toEqual(C);
});

it("groups edit children, collapse to a summary, and at_least / then edit their numbers", async () => {
  const { host, last } = await setup([{ any: [C] }, { at_least: 2, of: 3, when: C }, { then: [C, C], within: 5 }]);
  const group = host.querySelector("[data-lab-kind='any']")!;
  await choose(group.querySelector<HTMLSelectElement>(".lab-group-body > select[data-lab-add]")!, "cond");
  expect((last()[0] as { any: RuleNode[] }).any).toHaveLength(2);
  await click(group.querySelector<HTMLButtonElement>("button[aria-expanded]")!);
  expect(group.querySelector(".lab-group-body")).toBeNull();
  expect(group.textContent).toContain("Any of 1: close is above Exponential Moving Average(50)");
  await type(host.querySelector('input[aria-label="at least"]')!, "3");
  expect(last()[1]).toMatchObject({ at_least: 3, of: 3 });
  await type(host.querySelector('input[aria-label="within (bars)"]')!, "8");
  expect(last()[2]).toMatchObject({ within: 8 });
  const steps = host.querySelector("[data-lab-kind='then'] .lab-group-body > select[data-lab-add]") as HTMLSelectElement;
  expect(steps).not.toBeNull();
});

it("caps disable adding and wrapping", async () => {
  const { host } = await setup([C], { caps: { leaves: true, nodes: false, top: false } });
  expect(host.querySelector<HTMLOptionElement>("select[data-lab-add] option[value='cond']")!.disabled).toBe(true);
  const n = await setup([C], { caps: { leaves: false, nodes: true, top: false } });
  expect(n.host.querySelector<HTMLOptionElement>("select[data-lab-node-menu] option[value='wrap_all']")!.disabled).toBe(true);
  const top = await setup([C], { caps: { leaves: false, nodes: false, top: true } });
  expect(top.host.querySelector<HTMLOptionElement>("[data-lab-list] > select[data-lab-add] option[value='any']")!.disabled).toBe(true);
});

it("unknown nodes are kept read-only and can be removed", async () => {
  const { host, last } = await setup([{ fancy: 1 } as never, C]);
  expect(host.querySelector("[data-lab-kind='unknown'] .lab-kept")!.textContent).toContain('{"fancy":1}');
  await click(host.querySelector('button[aria-label="Remove Entry 1"]')!);
  expect(last()).toEqual([C]);
});

it("bars held is offered only in top-level exit comparisons", async () => {
  const { host } = await setup([C, { not: C }], { listPath: "exit.any", tag: "Exit" });
  const kinds = [...host.querySelectorAll<HTMLSelectElement>('select[aria-label="Operand type"]')].map((s) => !!s.querySelector('option[value="bars_held"]'));
  expect(kinds).toEqual([true, true, false, false]);
});
```
(The last test needs Task 5's `heldOk`; before Task 5 lands, today's editor offers bars held on every exit operand and the assertion fails — that is expected; it must pass at the wave-3 merge.)

- [ ] **Step 2: Run** `npx vitest run tests/lab-node-editor.test.tsx` — fails: module missing.

- [ ] **Step 3: Implement.**
  - `RuleList`: `<div className="lab-list" data-lab-list={listPath}>` → `<h3>{title}</h3>`, then each node as `<NodeEditor top label={`${tag} ${i + 1}`} … />` with `onChange` replacing index i, `onRemove` filtering, `onMove` via `move(list, i, d)`, `canUp={i > 0}`, `canDown={i < list.length - 1}`; then `<AddMenu caps top />`.
  - `AddMenu` (internal): `<select data-lab-add aria-label={`Add to ${title}`} value="">` with placeholder `COPY.addMenu` and `COPY.addOptions`; every option (a condition, or a group with its default condition) is disabled when `caps.leaves || caps.nodes || (top && caps.top)`; choosing calls `onAdd(newNode(kind))` and the select stays at `""` (controlled value).
  - `NodeEditor`: `kind = nodeKind(node)`; `side` is `"exit"` for exit lists; `heldOk = top && side === "exit"` for comparison operands, `false` elsewhere; `renderNode = (n, set, label) => <NodeEditor node={n} onChange={set} catalogue caps side top={false} label={label} />`; every `OperandEditor` gets `nodesFull={caps.nodes}`.
    - Leaf row: `<div className="lab-cond lab-node" data-lab-node data-lab-kind={kind}>` → lhs editor; `select aria-label="Operator"` (catalogue operators plus the current op; labels `OPERATOR_LABELS`; change → `onChange(withOp(node, op))`); cmp → rhs editor; range → lo editor, `<span className="lab-word">and</span>`, hi editor; trend → `NumField label="bars"` (1..1000) `<span className="lab-word">bars</span>`; then `NodeMenu` and the remove button `aria-label={`Remove ${label}`}` (`×`, `dbtn sm ghost`).
    - Group (`all`, `any`, `not`, `at_least`, `then`): `<div className="lab-group lab-node" data-lab-node data-lab-kind={kind}>` with a header row `.lab-group-h`: a toggle `<button type="button" className="dbtn sm ghost" aria-expanded={open}>` labelled `All of` / `Any of` / `Not` / `At least` / `Sequence`; for `all`/`any` a compact `select aria-label="Group type"` (All of / Any of, swaps the key); `at_least` → `NumField label="at least"` "of the last" `NumField label="of (bars)"` "bars"; `then` → "within" `NumField label="within (bars)"` "bars"; then `NodeMenu` and remove. Body `<div className="lab-group-body">` (only when open): children as `NodeEditor` (top false) with their own remove/move callbacks inside the group (a `not`/`at_least` child gets no remove), `then` steps separated by `<span className="lab-word">then</span>`, and an `AddMenu` (for `then`: only `cond`, `all`, `any`, `not`, disabled at 4 steps). Collapsed: header plus `<span className="cap">{describeNode(node, catalogue)}</span>`. `useState(true)` per node.
    - Unknown: `<div className="lab-cond lab-node" data-lab-node data-lab-kind="unknown"><span className="lab-kept">{COPY.keptAsIs} <code>{JSON.stringify(node)}</code></span>` + remove.
  - `NodeMenu`: `<select data-lab-node-menu aria-label={`Actions for ${label}`} value="">` with placeholder `⋯` and `COPY.nodeMenu` entries: wrap_all / wrap_any / wrap_not (disabled when `caps.nodes`), unwrap (only when `unwrap(node)` is defined), up / down (only when `onMove`; disabled by `canUp`/`canDown`).
  - `tree.css`: `.lab-list { display: grid; gap: 4px; align-content: start; }` `.lab-list > h3 { margin: 0 0 2px; }` `.lab-group { padding: 4px 0; border-bottom: 1px solid var(--rule); }` `.lab-group-h { display: flex; flex-wrap: wrap; gap: 4px 8px; align-items: center; }` `.lab-group-body { display: grid; gap: 2px; margin: 4px 0 0 4px; padding-left: 10px; border-left: 2px solid var(--rule); }` `.lab-group-body > .lab-cond:last-of-type { border-bottom: 0; }` `.lab-group-body > select[data-lab-add] { justify-self: start; }` `.lab-list > select[data-lab-add] { justify-self: start; margin-top: 4px; }` `@media (max-width: 600px) { .lab-group-body { margin-left: 2px; padding-left: 6px; } }` (indent = 4 + 10 + 2 px border ≤ 16; 8 at ≤ 600 px).

- [ ] **Step 4: Run** `npx vitest run tests/lab-node-editor.test.tsx` and `npx tsc --noEmit` — pass (the bars-held test passes once Task 5 is merged).
- [ ] **Step 5: Browser acceptance:** the tree editor is mounted by Task 11; its visual checks (`p7GroupAdded`, `p7RangeOp`, `p7Indent`, `p7LeafTwoLines`, `p7RulesRhythm`) run in Task 11 Step 5 on all six views.
- [ ] **Step 6: Commit (permission only):** `(lab-ui) rule tree editor: groups, not, ranges, trends, at-least, sequences`.

---

### Task 7: Execution panel and orders panel (direction, opposite, fills, timing; entry orders, exits, scale-in)

**Files:** Modify `src/features/lab/OrdersPanel.tsx`; create `src/features/lab/orders.css`, `tests/lab-orders.test.tsx`.
**Depends on:** Tasks 1, 3 (and `OperandEditor` for At levels). **Backend:** Tasks 7–9 for live runs; scale-in: Task 10.
**Interface:**

```ts
/** Shared execution settings: direction (+ opposite with Both), entry/exit fill, and folded timing options (delay, entry on, same-bar order). */
export function ExecutionPanel(props: { spec: Spec; catalogue: Catalogue; onChange(spec: Spec): void }): JSX.Element
/** One side's entry order and exits. pre "" = the long (or only) side, "short_side." = the short side. */
export function OrdersPanel(props: { spec: Spec; onChange(spec: Spec): void; pre?: SidePre; catalogue?: Catalogue }): JSX.Element
```
(`catalogue` is optional so today's `RuleBuilder` still compiles; without it the At form is hidden.)

- [ ] **Step 1: Write the failing tests** (`tests/lab-orders.test.tsx`):

```tsx
it("Both adds the short side and the opposite setting with help; Long removes them; Both again restores the short side", async () => {
  const { host, last, rerender } = await exec(defaultSpec());
  await clickIn(host, '[aria-label="Direction"]', "Both");
  expect(last().short_side).toEqual(DEFAULT_SHORT_SIDE);
  await rerender(setAt(last(), "short_side.entry.all[0].rhs.p.length", 20));
  expect(host.querySelector('[aria-label="When the other side signals"]')).not.toBeNull();
  expect(host.textContent).toContain(COPY.oppositeHelp.reverse);
  expect(host.textContent).toContain(COPY.bothFlat);
  await clickIn(host, '[aria-label="When the other side signals"]', "Exit only");
  expect(last().execution).toEqual({ opposite: "exit" });
  await clickIn(host, '[aria-label="Direction"]', "Long");
  expect(last()).not.toHaveProperty("short_side");
  await rerender(last());
  await clickIn(host, '[aria-label="Direction"]', "Both");
  expect(last().short_side?.entry.all[0]).toMatchObject({ rhs: { p: { length: 20 } } });
});

it("fills: entry fill hidden while any side has an order; same close drops the fill delay", async () => {
  const ordered = await exec(withEntryOrder(withDirection(defaultSpec(), "both"), "short_side.", "stop"));
  expect(ordered.host.querySelector('select[aria-label="Entry fill"]')).toBeNull();
  const { host, last } = await exec(setExecution(defaultSpec(), { delay_bars: 2 }));
  await choose(host.querySelector('select[aria-label="Exit fill"]')!, "close");
  expect(last().execution).toEqual({ exit_fill: "close" });
});

it("timing options fold away until set; OHLC path shows its explanation; edge entries", async () => {
  const { host, last } = await exec(defaultSpec());
  expect(host.querySelector('[aria-label="Same-bar order"]')).toBeNull();
  await click(host.querySelector("button[data-lab-timing-options]")!);
  await clickIn(host, '[aria-label="Same-bar order"]', "OHLC path");
  expect(last().execution).toEqual({ intrabar: "ohlc_path" });
  const set = await exec(setExecution(defaultSpec(), { intrabar: "ohlc_path" }));
  expect(set.host.textContent).toContain(COPY.intrabarHelp.ohlc_path);
  await clickIn(set.host, '[aria-label="Enter on"]', "Only the bar they turn true");
  expect(set.last().execution).toMatchObject({ entry_on: "edge" });
});

it("entry orders: stop, At level with an operand editor, stop-limit hint, order duration", async () => {
  const { host, last, rerender } = await orders(setExecution(defaultSpec(), { fill: "close" }));
  await clickIn(host, '[aria-label="Long entry order"]', "Stop");
  expect(last().entry.stop).toEqual({ pct: 1 });
  expect(last().execution).toEqual({ exit_fill: "close" });
  await rerender(last());
  await choose(host.querySelector('select[aria-label="Stop unit"]')!, "at");
  expect(last().entry.stop).toEqual({ at: { highest: { s: "high" }, bars: 5 } });
  await rerender(withEntryOrder(last(), "", "stop_limit"));
  expect(host.textContent).toContain(COPY.stopLimitHint);
  await type(host.querySelector('input[aria-label="Order stays (bars)"]')!, "3");
  expect(last().entry.valid_bars).toBe(3);
});

it("exits: add menu lists only unset exits; trailing activation; breakeven; take-profits in % of the position; min bars", async () => {
  const { host, last, rerender } = await orders(defaultSpec());
  const add = () => host.querySelector<HTMLSelectElement>('select[aria-label="Add exit (long)"]')!;
  await choose(add(), "trailing");
  await rerender(last());
  expect([...add().options].map((o) => o.value)).not.toContain("trailing");
  await choose(host.querySelector('[data-lab-exit="trailing"] select[aria-label="Trailing activation unit"]')!, "pct");
  expect(last().exit.trailing).toEqual({ pct: 5, activate: { pct: 2 } });
  await choose(add(), "breakeven");
  expect(last().exit.breakeven).toEqual({ after: { pct: 2 } });
  await rerender(withTakeProfits(defaultSpec(), "", [{ pct: 5, size: 0.5 }]));
  expect(host.querySelector<HTMLInputElement>('[data-lab-exit="take_profits"] input[aria-label="Take-profit 1 size (%)"]')!.value).toBe("50");
  await type(host.querySelector('[data-lab-exit="take_profits"] input[aria-label="Take-profit 1 size (%)"]')!, "30");
  expect(last().exit.take_profits).toEqual([{ pct: 5, size: 0.3 }]);
  await rerender(withTakeProfits(defaultSpec(), "", [{ pct: 1, size: 0.2 }, { pct: 2, size: 0.2 }, { pct: 3, size: 0.2 }]));
  expect(host.querySelector('[data-lab-exit="take_profits"] button[aria-label="Add take-profit"]')).toBeNull();
  await choose(add(), "min_bars");
  expect(last().exit.min_bars).toBe(1);
  await rerender(last());
  expect(host.textContent).toContain(COPY.minBarsHint);
  await click(host.querySelector('[data-lab-exit="min_bars"] button[aria-label="Remove Minimum bars"]')!);
  expect(last().exit).not.toHaveProperty("min_bars");
});

it("the short side writes to short_side paths and labels itself", async () => {
  const both = withDirection(defaultSpec(), "both");
  const { host, last } = await orders(both, "short_side.");
  expect(host.querySelector('[data-lab-orders="short"]')).not.toBeNull();
  await choose(host.querySelector('select[aria-label="Add exit (short)"]')!, "stop_loss");
  expect(last().short_side?.exit.stop_loss).toEqual({ pct: 5 });
  expect(last().exit).not.toHaveProperty("stop_loss");
});

it("scale-in shows only with the engine's execution options and refuses take-profits", async () => {
  const none = await orders(defaultSpec(), "", catalogueFixture);
  expect(none.host.querySelector('input[aria-label="Scale-in legs (% of full size)"]')).toBeNull();
  const { host, last, rerender } = await orders(defaultSpec());
  await click(host.querySelector('input[type=checkbox][aria-label="Scale in"]')!);
  expect(last().entry.scale_in).toEqual({ legs: [0.5, 0.5] });
  await rerender(last());
  await type(host.querySelector('input[aria-label="Scale-in legs (% of full size)"]')!, "50, 25, 25");
  expect(last().entry.scale_in).toEqual({ legs: [0.5, 0.25, 0.25] });
  const tp = await orders(withTakeProfits(defaultSpec(), "", [{ pct: 5, size: 0.5 }]));
  expect(tp.host.querySelector<HTMLInputElement>('input[type=checkbox][aria-label="Scale in"]')!.disabled).toBe(true);
  expect(tp.host.textContent).toContain(COPY.scaleInWithTp);
});
```
(`exec(spec)` mounts `ExecutionPanel` with `catalogue7Fixture`; `orders(spec, pre = "", cat = catalogue7Fixture)` mounts `OrdersPanel`; `rerender(spec)` re-renders the same root with a new spec; `clickIn(host, groupSel, text)` clicks the button with that text inside the group.)

- [ ] **Step 2: Run** `npx vitest run tests/lab-orders.test.tsx` — fails.

- [ ] **Step 3: Implement** (all edits through `execution-model.ts` and `setExecution`/`putAt`):
  - `ExecutionPanel`: `<div className="lab-exec" data-lab-exec>`; row 1 (`.lab-row`): `<span className="dfield">Direction</span>` + `Segmented label="Direction"` Long / Short / Both → `withDirection(spec, d, kept.current)` where `const kept = useRef<ShortSide>()` stores `spec.short_side` before leaving Both; with Both, `Segmented label="When the other side signals"` Reverse / Exit only / Ignore (`opposite` removed when Reverse = default); `select aria-label="Entry fill"` (hidden when any side's `entryOrderType` ≠ market) and `select aria-label="Exit fill"` (existing semantics: no `exit_fill` while equal to fill; choosing `close` for either also removes `delay_bars`); `<button type="button" className="dbtn sm ghost" data-lab-timing-options aria-expanded>Timing options</button>` (open by default when `delay_bars`, `entry_on` or `intrabar` is set). With Both: `<p className="cap">{COPY.oppositeHelp[opposite]} {COPY.bothFlat}</p>`. Timing row (when open): `NumField label="Fill delay (bars)"` 1..`max_delay_bars` (shown disabled with `title={COPY.delayOnlyNextOpen}` unless both fills are next open; 1 removes the key), `Segmented label="Enter on"` "Every bar the rules are true" / "Only the bar they turn true" (`entry_on` edge), `Segmented label="Same-bar order"` "Stop first" / "OHLC path", then `<p className="cap">{COPY.intrabarHelp[intrabar]}</p>`.
  - `OrdersPanel`: `sideName = pre ? "Short" : spec.direction === "short" ? "Short" : "Long"`; `<div className="lab-orders" data-lab-orders={pre ? "short" : "long"}>`.
    - Entry row: `<span className="dfield">Entry order</span>` + `Segmented label={`${sideName} entry order`}` Market / Limit / Stop / Stop-limit → `withEntryOrder`; for each set order level (limit, stop) a `LevelEditor` (label "Limit" / "Stop", `aria-label="Limit unit"` / `"Stop unit"`, forms % / ATR / Points / At (At only with `catalogue`)); not market → `NumField label="Order stays (bars)"` 1..`max_valid_bars` (1 removes the key); stop-limit whose limit is not At → `<p className="cap">{COPY.stopLimitHint}</p>`.
    - Scale-in (only when `catalogue?.execution`): checkbox `aria-label="Scale in"` (disabled with the reason cap when take-profits exist) → `{ legs: [0.5, 0.5] }` / removed; when set: text input `aria-label="Scale-in legs (% of full size)"` keeping typed text locally, parsed as comma/space separated percentages → fractions (only valid parses reach the spec), `NumField label="Bars between legs"` (`min_bars`, 1 = default removes the key), `select aria-label="Leg price"` any / better / worse (`any` removes the key).
    - Exits row: one chip per set exit, each `<div className="lab-order" data-lab-exit={key} role="group" aria-label={label}>` with the label, its editor and `<button aria-label={`Remove ${label}`}>×</button>`: `stop_loss` "Stop-loss", `target` "Target" (`LevelEditor`), `trailing` "Trailing stop" (`LevelEditor` + `LevelEditor` "activate after" with an Off option, `aria-label="Trailing activation unit"`), `breakeven` "Breakeven" (after `LevelEditor` + "plus" `LevelEditor` with Off, no At), `take_profits` "Take-profits" (rows: `NumField label={`Take-profit ${k+1} size (%)`}` showing `size*100`, writing `size/100`; "at"; `LevelEditor`; remove row; `button aria-label="Add take-profit"` while fewer than `max_take_profits`, adding `{ pct: 5, size: 0.25 }`), `min_bars` "Minimum bars" (`NumField label="Rule exits wait (bars)"` 0..1000 + `<span className="cap">{COPY.minBarsHint}</span>`). Then `select aria-label={`Add exit (${sideName.toLowerCase()})`}` with placeholder "+ Add exit…" listing only unset exits: stop_loss → `{pct: 5}`, target → `{pct: 10}`, trailing → `{pct: 5}`, breakeven → `{ after: { pct: 2 } }`, take_profits → `[{ pct: 5, size: 0.5 }]`, min_bars → `1`.
  - `LevelEditor` (internal): `{ value, slot, short, onChange, catalogue?, allowOff?, label }` → `select aria-label={`${label} unit`}` (Off when `allowOff`, %, ATR, Points, At when `LEVELS[slot].at && catalogue`) + fields: `NumField label="%"` (bounds from `LEVELS`), `NumField label="ATR ×"` + `NumField label="ATR length"`, `NumField label="Points"`, or `<OperandEditor side="entry" heldOk={false} catalogue … label={`${label} level`}/>`. Switching form → `defaultLevel(slot, form, short)`.
  - `orders.css`: `.lab-exec { display: grid; gap: 6px; }` `.lab-orders { display: grid; gap: 6px; }` `.lab-exits { display: flex; flex-wrap: wrap; gap: 6px 10px; align-items: center; }` `.lab-exits .lab-order { padding: 4px 8px; border: 1px solid var(--rule); border-radius: var(--r-field); }` `.lab-tp-row { display: flex; flex-wrap: wrap; gap: 4px 6px; align-items: center; }` `.lab-scale-legs { width: 110px; padding: 5px 8px; border: 1px solid var(--steel); border-radius: var(--r-field); background: var(--card); color: var(--ink); font: 500 13px var(--font); }`.

- [ ] **Step 4: Run** `npx vitest run tests/lab-orders.test.tsx` and `npx tsc --noEmit` — pass.
- [ ] **Step 5: Browser acceptance** (the orders panel is on the page through today's `RuleBuilder`; the execution panel is mounted by Task 11): `phase7` run; required on all six views: `p7StopOrder`, `p7TakeProfit`, `p7NoOverflow`, `p7CompactControls`, `p7NoWhiteInMidnight`. Screenshots: the exits sit on one wrapped row of chips at 1920/1425 and wrap cleanly at 400 px; the take-profit chip shows size and level on one line at ≥ 1425 px. `p7BothPanel` is checked in Task 11.
- [ ] **Step 6: Commit (permission only):** `(lab-ui) execution and orders panels: both sides, entry orders, exits, scale-in`.

---

### Task 8: Risk panel (sizing, money mode and costs, re-entry, risk, instrument note)

**Files:** Create `src/features/lab/RiskPanel.tsx`, `src/features/lab/risk.css`, `tests/lab-risk.test.tsx`.
**Depends on:** Tasks 1, 3. **Backend:** Tasks 10, 11, 12, 13 (live); 14 (futures note wording).
**Interface:** `export function RiskPanel(props: { spec: Spec; catalogue: Catalogue; instrument: InstrumentContext; onChange(spec: Spec): void }): JSX.Element | null` — `null` when `!catalogue.execution`.

- [ ] **Step 1: Write the failing tests** (`tests/lab-risk.test.tsx`):

```tsx
it("is absent until the engine lists execution options; folded unless something is set", async () => {
  expect((await risk(defaultSpec(), usEtf, catalogueFixture)).host.innerHTML).toBe("");
  const { host } = await risk(defaultSpec(), usEtf);
  expect(host.querySelector<HTMLDetailsElement>("details[data-lab-risk]")!.open).toBe(false);
  expect(host.querySelector("summary")!.textContent).toContain("Position size, re-entry and risk");
  const set = await risk(setExecution(defaultSpec(), { cooldown: { bars: 3 } }), usEtf);
  expect(set.host.querySelector<HTMLDetailsElement>("details[data-lab-risk]")!.open).toBe(true);
  expect(set.host.querySelector("summary")!.textContent).toContain("cooldown 3 bars");
});

it("money mode is blocked with the reason on an index and for India shorts", async () => {
  const { host } = await risk(defaultSpec(), { kind: "index", market: "in" });
  const money = [...host.querySelectorAll<HTMLButtonElement>('[aria-label="Results in"] button')].find((b) => b.textContent === "Money")!;
  expect(money.disabled).toBe(true);
  expect(host.querySelector("[data-lab-money-blocked]")!.textContent).toBe(MONEY_BLOCKED.index);
  const short = await risk(withDirection(defaultSpec(), "short"), { kind: "individual_stock", market: "in" });
  expect(short.host.querySelector("[data-lab-money-blocked]")!.textContent).toBe(MONEY_BLOCKED.indiaShort);
});

it("money mode on a US ETF: capital, sizing in shares, costs, taxes, borrow for shorts", async () => {
  const { host, last, rerender } = await risk(setExecution(defaultSpec(), { cost_bps: 10 }), usEtf);
  await clickIn(host, '[aria-label="Results in"]', "Money");
  expect(last().execution).toEqual({ capital: 1_000_000 });
  await rerender(last());
  expect(host.textContent).toContain(MONEY_BASIS_HINT);
  await choose(host.querySelector('select[aria-label="Size"]')!, "units");
  expect(last().execution?.sizing).toEqual({ units: 100 });
  await type(host.querySelector('input[aria-label="Commission per order"]')!, "20");
  expect(last().execution?.costs).toEqual({ commission: { per_order: 20 } });
  await click(host.querySelector('button[aria-label="Add tax"]')!);
  expect(last().execution?.costs?.taxes).toEqual([{ name: "Tax", pct: 0.1, side: "both" }]);
  expect(host.querySelector('input[aria-label="Borrow % per year"]')).toBeNull();
  const short = await risk(withMode(withDirection(defaultSpec(), "short"), "money"), usEtf);
  expect(short.host.querySelector('input[aria-label="Borrow % per year"]')).not.toBeNull();
});

it("unit mode offers full, % of equity and risk %; shares and value need money mode", async () => {
  const { host, last } = await risk(defaultSpec(), usEtf);
  const size = host.querySelector<HTMLSelectElement>('select[aria-label="Size"]')!;
  expect([...size.options].filter((o) => o.disabled).map((o) => o.value)).toEqual(["units", "value"]);
  await choose(size, "risk_pct");
  expect(last().execution).toEqual({ sizing: { risk_pct: 1 } });
});

it("re-entry and risk fields: empty means off", async () => {
  const { host, last, rerender } = await risk(defaultSpec(), usEtf);
  await type(host.querySelector('input[aria-label="Cooldown (bars)"]')!, "3");
  expect(last().execution).toEqual({ cooldown: { bars: 3 } });
  await rerender(last());
  await choose(host.querySelector('select[aria-label="Cooldown after"]')!, "loss");
  expect(last().execution?.cooldown).toEqual({ bars: 3, after: "loss" });
  await type(host.querySelector('input[aria-label="Max entries"]')!, "2");
  expect(last().execution?.max_trades).toEqual({ count: 2, per: "M" });
  await type(host.querySelector('input[aria-label="Losses in a row"]')!, "4");
  expect(last().execution?.risk).toEqual({ max_consecutive_losses: 4, pause_bars: 20 });
  await type(host.querySelector('input[aria-label="Stop new entries at drawdown (%)"]')!, "25");
  expect(last().execution?.risk).toMatchObject({ max_drawdown_pct: 25 });
  await type(host.querySelector('input[aria-label="Cooldown (bars)"]')!, "");
  expect(last().execution).not.toHaveProperty("cooldown");
});

it("shows the futures note", async () => {
  expect((await risk(defaultSpec(), usEtf)).host.textContent).toContain(FUTURES_NOTE);
});
```

- [ ] **Step 2: Run** `npx vitest run tests/lab-risk.test.tsx` — fails.

- [ ] **Step 3: Implement.** `<details className="lab-risk" data-lab-risk open={open} onToggle={(e) => setOpen(e.currentTarget.open)}>` with `const [open, setOpen] = useState(active)` (`active` = any of sizing, capital, cooldown, max_trades, risk set). `<summary>Position size, re-entry and risk <small>{summary}</small></summary>` where summary joins set items (`money · 1,000,000`, `risk 1% per trade`, `50% of equity`, `100 shares`, `value 100,000`, `cooldown 3 bars`, `max 2 per month`, `pause after 4 losses`, `stop at 25% drawdown`) or `off`. Body rows (`.lab-row`):
  - "Results in": `Segmented label="Results in"` Unit study / Money (`disabled` + `title` = `moneyBlocked(instrument, spec)`); when blocked `<p className="cap" data-lab-money-blocked>{reason}</p>`; Money → `withMode(spec, "money")`.
  - Size: `select aria-label="Size"` full "All equity (default)", pct_equity "% of equity", risk_pct "Risk % per trade", units "Shares", value "Position value" (units/value `disabled` in unit mode, `title={COPY.needsMoney}`) → `withSizingKind`; value field `NumField` labelled "% of equity" (0..100), "Risk %" (0..100), "Shares" (whole ≥ 1), "Value" (> 0). Money mode adds `NumField label="Capital"` (> 0) first.
  - Money costs (money only): `NumField label="Commission per order"`, `"Commission per share"`, `"Commission %"`, `"Slippage (bps)"` (empty removes the key; empty objects removed); taxes: rows of a text input `aria-label={`Tax ${k+1} name`}`, `NumField label={`Tax ${k+1} %`}`, `select aria-label={`Tax ${k+1} side`}` (Buy / Sell / Both), remove; `button aria-label="Add tax"` (≤ 6) adding `{ name: "Tax", pct: 0.1, side: "both" }`; `NumField label="Borrow % per year"` only when `hasShorts(spec)` and `instrument.market === "us"`; `<p className="cap">{MONEY_BASIS_HINT}</p>`.
  - Re-entry: `NumField label="Cooldown (bars)"` + `select aria-label="Cooldown after"` (any trade / a loss / a win; any removes `after`); `NumField label="Max entries"` + `select aria-label="Max entries per"` (week / month / quarter / year; default M).
  - Risk: `NumField label="Losses in a row"` + `NumField label="Pause (bars)"` (default 20 when losses first set; both removed together when losses is emptied); `NumField label="Stop new entries at drawdown (%)"`.
  - `<p className="cap">{FUTURES_NOTE}</p>`.
  - Execution fields go through `setExecution`; nested objects are rebuilt and empty objects removed.
  - `risk.css`: `.lab-risk > summary { cursor: pointer; font-weight: 600; color: var(--ink); padding: 2px 0; }` `.lab-risk > summary small { margin-left: 8px; font-weight: 500; color: var(--ink2); }` `.lab-risk[open] > summary { margin-bottom: 6px; }` `.lab-risk { display: grid; gap: 6px; }` `.lab-risk .lab-row + .lab-row { margin-top: 6px; }` `.lab-risk input[type=text] { width: 90px; padding: 5px 8px; border: 1px solid var(--steel); border-radius: var(--r-field); background: var(--card); color: var(--ink); font: 500 13px var(--font); }`.

- [ ] **Step 4: Run** `npx vitest run tests/lab-risk.test.tsx` and `npx tsc --noEmit` — pass.
- [ ] **Step 5: Browser acceptance:** mounted by Task 11; Task 11 Step 5 checks `p7RiskOpen`, `p7MoneyBlockedOnIndex`, `p7CompactControls`, `p7NoWhiteInMidnight` on all six views, and the screenshots must show the open panel in two to four compact rows at 1920 px.
- [ ] **Step 6: Commit (permission only):** `(lab-ui) risk panel: sizing, money mode and costs, re-entry, risk`.

---

### Task 9: Results — execution model, adaptations, money, legs and scale-outs, side and order columns

**Files:** Modify `src/features/lab/RunResults.tsx`, `src/features/lab/results.css`, `tests/lab-results.test.tsx`.
**Depends on:** Task 1. **Backend:** Task 12 (`execution_model`, `legs`, `fills`), Task 13 (`money`, `ledger`) for live checks; `adaptations` exists today.
**Interface:** unchanged exports `RunResults`, `RunTables`, `kpiItems`, `drawdown`; `tradeRows` is replaced by:

```ts
export interface TradeTable { header: string[]; rows: (string | ReactNode)[][]; csv: string[][]; numeric: number[]; ret: number }
/** Columns appear only when some trade has them (no empty columns); the ledger joins only when aligned. */
export function tradeTable(result: Pick<RunResult, "trades" | "ledger">): TradeTable
export function ExecutionStrip(props: { model?: ExecutionModel; money?: MoneySummary; adaptations?: string[] }): JSX.Element | null
```

- [ ] **Step 1: Write the failing tests** (add to `tests/lab-results.test.tsx`; replace the `tradeRows` assertions with `tradeTable`):

```tsx
it("execution model notes, events and adaptations are shown as given", async () => {
  const host = await mount(<RunResults result={run7Fixture} market="in" />);
  expect(host.querySelector("[data-lab-execution-notes]")!.textContent).toContain(MODEL_NOTE.index);
  expect(host.textContent).toContain(COPY.studyOnly);
  expect(host.querySelector("[data-lab-events]")!.textContent).toContain("risk pause after 2 losing trades on 2024-01-15");
  expect(host.querySelector("[data-lab-adaptations]")!.textContent).toBe(run7Fixture.adaptations![0]);
});

it("a response without Phase 7 keys renders as before", async () => {
  const host = await mount(<RunResults result={runFixture} market="in" />);
  expect(host.querySelector("[data-lab-execution-notes], [data-lab-adaptations], [data-lab-money]")).toBeNull();
  expect(host.textContent).toContain("Fees 5 bps + slippage 2 bps per side");
});

it("money mode: tiles, basis, no bps line", async () => {
  const host = await mount(<RunResults result={money7Fixture} market="us" />);
  const money = host.querySelector("[data-lab-money]")!;
  expect(money.textContent).toContain("Money · USD 100,000");
  expect([...host.querySelectorAll(".metric span")].map((s) => s.textContent)).toEqual(expect.arrayContaining(["Final equity", "Costs paid"]));
  expect(host.textContent).toContain("1 trade skipped (size rounded to 0)");
  expect(host.textContent).toContain(money7Fixture.money!.basis);
  expect(host.textContent).not.toContain("bps + slippage");
});

it("trade table: side and order columns always; legs, scale-outs and money only when present", () => {
  const plain = tradeTable(runFixture);
  expect(plain.header).toEqual(["Entry date", "Side", "Order", "Entry", "Exit date", "Exit", "Return", "Bars", "Exit reason"]);
  const p7 = tradeTable(run7Fixture);
  expect(p7.header).toEqual(["Entry date", "Side", "Order", "Entry", "Legs", "Exit date", "Exit", "Scale-outs", "Return", "Bars", "Exit reason"]);
  expect(p7.csv[1]).toEqual(expect.arrayContaining(["Short", "Reversal"])); // newest first
  expect(p7.csv[2][4]).toBe("2024-01-08 @ 101.00 × 50%; 2024-01-09 @ 101.50 × 50%");
  const money = tradeTable(money7Fixture);
  expect(money.header.slice(-4)).toEqual(["Qty", "Value", "Costs", "P&L"]);
  const misaligned = tradeTable({ ...money7Fixture, ledger: money7Fixture.ledger!.slice(1) });
  expect(misaligned.header).not.toContain("Qty");
});

it("the tradesheet shows legs in a details cell and the ledger table when it cannot join", async () => {
  const host = await mount(<RunTables result={run7Fixture} />);
  expect(host.querySelector("table[data-lab-trades] details summary")!.textContent).toBe("2 legs");
  const m = await mount(<RunTables result={{ ...money7Fixture, ledger: money7Fixture.ledger!.slice(1) }} />);
  expect(m.querySelector("table[data-lab-ledger]")).not.toBeNull();
});
```

- [ ] **Step 2: Run** `npx vitest run tests/lab-results.test.tsx` — fails.

- [ ] **Step 3: Implement.**
  - `ExecutionStrip`: `null` when nothing to show. Otherwise `<div className="lab-exec-strip">`: a chip row (`<span className="chip">`): mode `COPY.unitStudy` or (in `<span data-lab-money>`) `Money · ${currency ?? ""} ${capital.toLocaleString("en-US")}`; `COPY.studyOnly` when `executable === false`. Notes `<ul className="rnote" data-lab-execution-notes>` (one `<li>` per note, verbatim). Events `<p className="cap" data-lab-events>{COPY.riskEvents}: {events.join(" · ")}</p>` when any. Adaptations `<p className="rnote" role="note" data-lab-adaptations>` per text, verbatim.
  - `RunResults`: after `ServerLabel` / provisional strip / start note: `<ExecutionStrip model={result.execution_model} money={result.money} adaptations={result.adaptations} />`. KPI items: in money mode prepend `{ label: "Final equity", value: money.final_equity.toLocaleString("en-US", { maximumFractionDigits: 0 }) }` and `{ label: "Costs paid", value: … }`. Cost line: money → `<p className="cap">{money.basis}</p>` plus, when `skipped > 0`, `` `${n} trade${n === 1 ? "" : "s"} skipped (size rounded to 0)` ``; unit → the existing fees/slippage line.
  - `tradeTable`: newest first. Columns: Entry date, Side (Long/Short), Order (`ENTRY_KIND_LABELS[k] ?? k`), Entry, [Legs], Exit date ("Open" when open), Exit, [Scale-outs], Return, Bars, Exit reason (`friendlyReason`), [Qty, Value, Costs, P&L]. Legs present when any trade has `legs`; Scale-outs when any has `fills`; a trade without them shows `1` (legs) or `—` (scale-outs). Cell: `<details><summary>{n} legs|scale-outs</summary>{pieces}</details>`; CSV text: `` `${date} @ ${price.toFixed(2)} × ${Math.round(w * 100)}%` `` joined by `; `. Money columns only when `ledger.length === trades.length && ledger.every((r, k) => r.entry === trades[k].entry)` (Qty integer, Value/Costs/P&L to 2 decimals; P&L coloured like Return). `numeric` lists the numeric column indices; `ret` the Return index.
  - `RunTables`: table gets `data-lab-trades`; header classes from `numeric`; Return colouring from `ret`; CSV from `csv`; when money rows exist but cannot join, render `<table className="tbl" data-lab-ledger>` (Entry date, Qty, Value, Costs, P&L) under the tradesheet with the caption cap "Money ledger".
  - `results.css`: `.lab-exec-strip { display: grid; gap: 6px; margin-top: 8px; }` `.lab-exec-strip .chip { font-size: 12px; }` `.lab-exec-strip ul.rnote { margin: 0; padding-left: 18px; }` `.lab-trades td details summary { cursor: pointer; color: var(--ink2); }` `.lab-trades td details { font-size: 12px; }`.

- [ ] **Step 4: Run** `npx vitest run tests/lab-results.test.tsx` and `npx tsc --noEmit` — pass.
- [ ] **Step 5: Browser acceptance** (results are on the page today): `phase7` run; required on all six views: `p7Run`, `p7NoEmptyColumns`, `p7PostNoOverflow`, `p7PostGaps`; with backend Task 12 landed also `p7IndexNote`. Screenshots `p7-postrun-*`: the notes sit directly under the label, one line each; the tradesheet shows Side and Order and no empty column; Midnight has no white cells; at 400 px the tradesheet scrolls inside its own container.
- [ ] **Step 6: Commit (permission only):** `(lab-ui) results: execution model, adaptations, money ledger, legs and scale-outs`.

---

### Task 10: Optimize — every Phase 7 path, grouped, with result notes

**Files:** Create `src/features/lab/optimize-paths.ts`, `tests/lab-optimize-paths.test.ts`; modify `src/features/lab/OptimizePanel.tsx`, `src/features/lab/optimize.css`, `tests/lab-optimize.test.tsx`.
**Depends on:** Tasks 1, 2, 3. **Backend:** Task 10 (sizing and scale-in paths), Task 12 (`execution_model` on optimize) for live checks.
**Interface:** `export interface OptimizableSetting { path: string; label: string; group: string; current: number; min: number; max: number; step: number; integer: boolean }`; `export function optimizablePaths(spec: Spec, cat: Catalogue): OptimizableSetting[]`. (The old `optimizablePaths` in spec-model stays until Task 11 deletes it; `OptimizePanel` imports from `optimize-paths.ts` from now on.)

- [ ] **Step 1: Write the failing tests.** `tests/lab-optimize-paths.test.ts`:

```ts
it("optimizable paths reach every Phase 7 number with engine paths, labels, groups and bounds", () => {
  let s: Spec = { v: 1, direction: "both",
    entry: { all: [{ lhs: { s: "close" }, op: ">", rhs: { s: "ema", p: { length: 50 } } }, { any: [{ lhs: { avg: { s: "close" }, bars: 10 }, op: ">", rhs: { value: 1 } }] }],
             stop: { at: { highest: { s: "high" }, bars: 5 } } },
    exit: { any: [{ lhs: { s: "bars_held" }, op: ">=", rhs: { value: 5 } }], take_profits: [{ pct: 2, size: 0.5 }, { atr: 2, size: 0.25 }],
            trailing: { pct: 5, activate: { points: 4 } }, breakeven: { after: { pct: 2 } } },
    short_side: { entry: { all: [{ lhs: { s: "rsi", p: { length: 2 } }, op: ">", rhs: { value: 90 } }], scale_in: { legs: [0.5, 0.5], min_bars: 2 } }, exit: { any: [], stop_loss: { pct: 3 } } },
    execution: { sizing: { risk_pct: 1 } } };
  const paths = optimizablePaths(s, cat7);
  const by = (p: string) => paths.find((x) => x.path === p)!;
  expect(paths.map((p) => p.path)).toEqual([
    "entry.all[0].rhs.p.length", "entry.all[1].any[0].lhs.bars", "entry.all[1].any[0].rhs.value", "exit.any[0].rhs.value",
    "short_side.entry.all[0].lhs.p.length", "short_side.entry.all[0].rhs.value", "entry.stop.at.bars",
    "exit.trailing.pct", "exit.trailing.activate.points", "exit.breakeven.after.pct", "exit.take_profits[0].pct", "exit.take_profits[1].atr",
    "short_side.exit.stop_loss.pct", "short_side.entry.scale_in.min_bars", "execution.sizing.risk_pct",
  ]);
  expect(by("entry.all[0].rhs.p.length")).toMatchObject({ label: "Long entry 1 › right · Exponential Moving Average Length", group: "Long entry", current: 50, integer: true });
  expect(by("entry.all[1].any[0].lhs.bars")).toMatchObject({ label: "Long entry 2 › any 1 › left › bars", min: 1, max: 1000, step: 1, integer: true });
  expect(by("entry.stop.at.bars")).toMatchObject({ group: "Long entry order" });
  expect(by("exit.take_profits[1].atr")).toMatchObject({ label: "Long Take-profit 2 · ATR ×", group: "Long exits", min: 0.25, max: 50 });
  expect(by("exit.trailing.activate.points")).toMatchObject({ label: "Long Trailing activation · points", min: 1, step: 1 });
  expect(by("short_side.exit.stop_loss.pct")).toMatchObject({ label: "Short Stop-loss · %", group: "Short exits", min: 0.5, max: 99, step: 0.5 });
  expect(by("short_side.entry.scale_in.min_bars")).toMatchObject({ label: "Short Scale-in · bars between legs", group: "Short entry order", min: 1, max: 1000 });
  expect(by("execution.sizing.risk_pct")).toMatchObject({ label: "Risk per trade · %", group: "Position size", min: 0.25, max: 100, step: 0.25 });
  s = { ...s, direction: undefined, short_side: undefined } as Spec;
  expect(optimizablePaths(s, cat7)[0]).toMatchObject({ label: "Entry 1 › right · Exponential Moving Average Length", group: "Entry" });
  expect(optimizablePaths(setAt(defaultSpec(), "entry.limit", { pct: 1 }), cat7).find((p) => p.path === "entry.limit.pct")).toMatchObject({ min: 0, max: 50 });
});
```
`tests/lab-optimize.test.tsx` additions:

```tsx
it("settings are grouped with header rows and nested paths can be varied", async () => {
  const spec = { ...defaultSpec(), entry: { all: [defaultSpec().entry.all[0], { any: [{ lhs: { s: "close" }, op: ">", rhs: { s: "ema", p: { length: 20 } } }] }] } } as Spec;
  const onOptimize = vi.fn(async () => optimizeFixture);
  const host = await mountPanel({ spec, onOptimize, catalogue: catalogue7Fixture });
  expect([...host.querySelectorAll("tr.lab-dim-group")].map((r) => r.textContent)).toEqual(["Entry", "Exit"]);
  await click(host.querySelector('tr[data-lab-dim="entry.all[1].any[0].rhs.p.length"] input[type=checkbox]')!);
  await click(host.querySelector('[data-lab-action="optimize"]')!);
  expect(onOptimize.mock.calls[0][0].optimize[0].path).toBe("entry.all[1].any[0].rhs.p.length");
});

it("the result shows the execution model notes and adaptations as given", async () => {
  const result = { ...optimizeFixture, execution_model: run7Fixture.execution_model, adaptations: run7Fixture.adaptations };
  const host = await mountPanel({ onOptimize: async () => result });
  await click(host.querySelector('[data-lab-dim] input[type=checkbox]')!);
  await click(host.querySelector('[data-lab-action="optimize"]')!);
  await flush();
  expect(host.querySelector("[data-lab-optimize-result] [data-lab-execution-notes]")!.textContent).toContain(MODEL_NOTE.index);
  expect(host.querySelector("[data-lab-optimize-result] [data-lab-adaptations]")).not.toBeNull();
});
```
(`mountPanel` = the file's existing mount helper; extend it with `catalogue` if it lacks one.)

- [ ] **Step 2: Run** both files — fail.

- [ ] **Step 3: Implement.**
  - `optimize-paths.ts`: `const both = spec.direction === "both"`; group of a root: `entry.all` → `both ? "Long entry" : "Entry"`, `exit.any` → `"Long exit"`/`"Exit"`, `short_side.entry.all` → `"Short entry"`, `short_side.exit.any` → `"Short exit"`, `*.entry.limit.at` / `*.entry.stop.at` → `"Entry order"` with the side prefix (`"Long entry order"` / `"Short entry order"` / `"Entry order"`). One `walkSpec` pass: knobs → whole: `{ label: pathLabel(path, both), current: value, min, max, step: 1, integer: true }`; value knobs → `span = max(|v|·0.5, 1)`, `min v−span`, `max v+span`, `step 1`, `integer = Number.isInteger(v)`; series operands → one setting per catalogue param (label `${pathLabel(operandPath, both)} · ${ind.name} ${p.label}`, bounds and step as the old function). Then per side (`""`, plus `"short_side."` with Both; label prefix `"Short "`, `"Long "` with Both, else none; group `"<prefix>exits"` capitalised for exit slots, `"<prefix>entry order"` for entry slots): every `LEVEL_SLOTS` slot except `exit.breakeven.plus`, then `take_profits[k]` with `TP_BOUNDS`, each by its form: pct (`min = zeroOk ? 0 : 0.5`, `max = maxPct`, `step 0.5`), atr (`0.25..50`, `step 0.25`), points (`min = zeroOk ? 0 : 1`, `max 1e9`, `step 1`, not integer); At levels and ticks produce nothing (their inner numbers come from the walker). Labels: `${prefix}${LEVELS[slot].label} · %|ATR ×|points`, take-profits `${prefix}Take-profit ${k+1} · …`. Then `${pre}entry.scale_in.min_bars` (label `${prefix}Scale-in · bars between legs`, 1..1000, current `min_bars ?? 1`) when scale-in is set. Last: `execution.sizing.pct_equity` (label "Position size · % of equity", 5..100 step 5) or `execution.sizing.risk_pct` ("Risk per trade · %", 0.25..100 step 0.25), group "Position size".
  - `OptimizePanel.tsx`: import `optimizablePaths` and `OptimizableSetting` from `./optimize-paths`; render `tbody` rows grouped in order of first appearance: `<tr className="lab-dim-group"><th colSpan={6}>{group}</th></tr>` before each group's rows (rows keep `data-lab-dim`). In `OptimizeResultView`, after `ServerLabel`/provisional strip, render `<ExecutionStrip model={result.execution_model} adaptations={result.adaptations} />` (imported from `./RunResults`). Column headers for dims use the setting label (unchanged logic).
  - `optimize.css`: `.lab-page .lab-dims tr.lab-dim-group th { padding: 8px 10px 2px; text-align: left; font-size: 12px; color: var(--ink2); background: transparent; border-bottom: 0; }`; the existing `:first-child` width rules must not apply to the group row: scope them to `tr[data-lab-dim]`.

- [ ] **Step 4: Run** both files and `npx tsc --noEmit` — pass.
- [ ] **Step 5: Browser acceptance:** legacy `preOptimizeColumnsCompact` (now reading `tr[data-lab-dim]`) on all six views; `p7OptimizeNested` on 1920-steel once Task 11 has mounted the tree editor. Screenshots: group headers are small grey captions, no extra column, numeric columns stay beside the label at 1920 and 1425, and the table scrolls inside its box at 400 px.
- [ ] **Step 6: Commit (permission only):** `(lab-ui) optimize: every Phase 7 path, grouped settings, result notes`.

---

### Task 11: Assembly — rules box, page wiring, chart markers, spec-model cleanup

**Files:** Modify `src/features/lab/RuleBuilder.tsx`, `src/features/lab/rules.css`, `src/pages/IndicatorsPage.tsx`, `src/features/lab/Workbench.tsx`, `src/features/lab/spec-model.ts`, `tests/lab-rule-builder.test.tsx`, `tests/indicators-page.test.tsx`, `tests/lab-spec-model.test.ts`, `tests/lab-workbench.test.tsx`.
**Depends on:** Tasks 1–10. **Backend:** Task 12 (catalogue limits and `execution`), Task 13 (money query refusal), Task 15 (always-in) for live checks.
**Interface:** `RuleBuilder` gains a required prop `instrument: InstrumentContext`; everything else unchanged.

- [ ] **Step 1: Write the failing tests.** Rewrite `tests/lab-rule-builder.test.tsx` (mount with `catalogue7Fixture` and `instrument = { kind: "index", market: "in" }` unless stated):

```tsx
it("one side by default; Both shows long and short panels with their own lists and orders", async () => {
  const { host } = await setup(defaultSpec());
  expect(host.querySelectorAll("[data-lab-side]")).toHaveLength(1);
  expect(host.querySelector("[data-lab-list='entry.all'] h3")!.textContent).toBe(COPY.entryTitle);
  const both = await setup(withDirection(defaultSpec(), "both"));
  expect([...both.host.querySelectorAll("[data-lab-side]")].map((e) => e.getAttribute("data-lab-side"))).toEqual(["long", "short"]);
  expect(both.host.querySelector("[data-lab-list='entry.all'] h3")!.textContent).toBe(COPY.longEntryTitle);
  expect(both.host.querySelector("[data-lab-side='short'] [data-lab-list='short_side.exit.any']")).not.toBeNull();
  expect(both.host.querySelector("[data-lab-side='short'] [data-lab-orders='short']")).not.toBeNull();
});

it("short-side list edits write to short_side", async () => {
  const { host, last } = await setup(withDirection(defaultSpec(), "both"));
  await choose(host.querySelector("[data-lab-list='short_side.entry.all'] > select[data-lab-add]")!, "any");
  expect(last().short_side!.entry.all).toHaveLength(2);
  expect(last().entry.all).toHaveLength(1);
});

it("caps line counts like the engine and warns near a cap; adding stops at the cap", async () => {
  const { host } = await setup(defaultSpec());
  expect(host.querySelector("[data-lab-caps]")!.textContent).toBe("Top-level 2/12 · comparisons 2/40 · rule parts 2/120 · 0.2 of 7.5 KB");
  let s = defaultSpec();
  for (let i = 0; i < 10; i++) s = addCond(s, "entry", { all: [DEFAULT_COND, DEFAULT_COND, DEFAULT_COND, DEFAULT_COND] }); // 12 top-level, 42 comparisons
  const full = await setup(s);
  expect(full.host.querySelector("[data-lab-caps]")!.classList.contains("lab-caps-warn")).toBe(true);
  expect(full.host.querySelector<HTMLOptionElement>("[data-lab-list='entry.all'] > select[data-lab-add] option[value='cond']")!.disabled).toBe(true);
});

it("nested edits keep origin and unknown keys", async () => {
  const base = { ...(templateFixture.status === "available" ? templateFixture.template.spec : defaultSpec()), future_rule: 1 } as Spec;
  const spec = { ...base, entry: { ...base.entry, all: [{ any: [{ ...DEFAULT_COND, tag: "x" }] }, ...base.entry.all] } } as Spec;
  const { host, last } = await setup(spec);
  await choose(host.querySelector("[data-lab-kind='any'] select[aria-label='Operator']")!, "<=");
  expect(last().entry.all[0]).toEqual({ any: [{ ...DEFAULT_COND, op: "<=", tag: "x" }] });
  expect(last().origin).toEqual(base.origin);
  expect(last().future_rule).toBe(1);
});

it("state operands edit their condition through the tree editor", async () => {
  const spec = setAt(defaultSpec(), "exit.any[0]", { lhs: { bars_since: DEFAULT_STATE_COND }, op: ">", rhs: { value: 3 } });
  const { host, last } = await setup(spec);
  await choose(host.querySelector("[data-lab-list='exit.any'] .lab-expr [data-lab-node] select[aria-label='Operator']")!, "crosses_below");
  expect(getAt(last(), "exit.any[0].lhs.bars_since.op")).toBe("crosses_below");
});

it("money mode hides fees and slippage; instrument notes, session-date hint and problems show", async () => {
  const money = await setup(withMode(defaultSpec(), "money"), { instrument: { kind: "etf", market: "us" } });
  expect(money.host.querySelector('input[aria-label="Fees (bps / side)"]')).toBeNull();
  expect(money.host.textContent).toContain(COPY.moneyCostsHere);
  const idx = await setup(withDirection(defaultSpec(), "both"));
  expect(idx.host.textContent).toContain(MODEL_NOTE.index);
  const sl = await setup(setAt(defaultSpec(), "exit.any[0]", { lhs: { s: "sessions_left", out: "m" }, op: "<", rhs: { value: 2 } }));
  expect(sl.host.textContent).toContain(COPY.sessionDatesHint);
  const bad = await setup(defaultSpec(), { problems: ["Entry 1 › any 1: a condition cannot compare two numbers; one side must be a series."] });
  expect(bad.host.querySelector<HTMLButtonElement>('[data-lab-action="run"]')!.disabled).toBe(true);
});

it("the risk panel and execution panel are mounted", async () => {
  const { host } = await setup(defaultSpec());
  expect(host.querySelector("details[data-lab-risk]")).not.toBeNull();
  expect(host.querySelector('[aria-label="Direction"]')).not.toBeNull();
});
```
`tests/indicators-page.test.tsx`: add `let catalogueAnswer: Catalogue = catalogueFixture;` (reset in `beforeEach`), make the catalogue route answer `jsonResponse(catalogueAnswer)`, add two helpers, then three tests:

```tsx
const typeIn = async (host: HTMLElement, sel: string, v: string) => act(async () => {
  const el = host.querySelector<HTMLInputElement>(sel)!;
  Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(el, v);
  el.dispatchEvent(new Event("input", { bubbles: true }));
});
const pressIn = async (host: HTMLElement, group: string, text: string) => act(async () =>
  [...host.querySelectorAll<HTMLButtonElement>(`[aria-label="${group}"] button`)].find((b) => b.textContent === text)!.click());
const spyEtf = { ...spy, native: true, asset_type: "etf" };

it("money runs send no cost or slippage query", async () => {
  catalogueAnswer = catalogue7Fixture;
  usTickers = [...tickersFixture, spyEtf];
  const { host, f } = await go("#/indicators?region=us&tk=SPY");
  await typeIn(host, 'input[aria-label="Fees (bps / side)"]', "5");
  await typeIn(host, 'input[aria-label="Slippage (bps / side)"]', "2");
  await act(async () => host.querySelector<HTMLElement>("details[data-lab-risk] > summary")!.click());
  await pressIn(host, "Results in", "Money");
  await flush();
  expect(host.querySelector('input[aria-label="Fees (bps / side)"]')).toBeNull();
  await run(host);
  const url = new URL(f.calls.find((c) => c.url.includes("/lab/run"))!.url, "http://x");
  expect(url.searchParams.has("cost")).toBe(false);
  expect(url.searchParams.has("slippage")).toBe(false);
  expect(JSON.parse(url.searchParams.get("spec")!).execution.capital).toBe(1_000_000);
});

it("nested problems from the tree checks block the run", async () => {
  catalogueAnswer = catalogue7Fixture;
  const t = templateFixture as Extract<TemplateResponse, { status: "available" }>;
  templateAnswer = { ...t, template: { ...t.template, spec: { ...t.template.spec, entry: { all: [{ any: [{ lhs: { value: 1 }, op: ">", rhs: { value: 2 } }] }] } } } };
  const { host, f } = await go("#/indicators?tk=NIFTY%2050&customize=rsi2_4");
  await flush();
  expect(host.textContent).toContain("Entry 1 › any 1: a condition cannot compare two numbers; one side must be a series.");
  expect(host.querySelector<HTMLButtonElement>('[data-lab-action="run"]')!.disabled).toBe(true);
  expect(f.calls.some((c) => c.url.includes("/lab/run"))).toBe(false);
});

it("an index instrument disables money mode before any request", async () => {
  catalogueAnswer = catalogue7Fixture;
  const { host, f } = await go("#/indicators?tk=NIFTY%2050");
  await act(async () => host.querySelector<HTMLElement>("details[data-lab-risk] > summary")!.click());
  const money = [...host.querySelectorAll<HTMLButtonElement>('[aria-label="Results in"] button')].find((b) => b.textContent === "Money")!;
  expect(money.disabled).toBe(true);
  expect(host.querySelector("[data-lab-money-blocked]")!.textContent).toBe(MONEY_BLOCKED.index);
  expect(f.calls.some((c) => c.url.includes("/lab/run"))).toBe(false);
});
```
(Imports added: `catalogue7Fixture` from the fixtures, `MONEY_BLOCKED` from labels, `Catalogue` and `TemplateResponse` types. If `templateFixture.template.spec.exit` is empty in the fixture, the second test still holds: its message list is checked with `toContain`.)
`tests/lab-workbench.test.tsx` addition: `short trades are marked above the bar in red; long below in green` (assert the `setMarkers` mock receives `{ position: "aboveBar", shape: "arrowDown", text: "Short" }` for a `dir: -1` trade and `{ position: "belowBar", shape: "arrowUp", text: "Exit" }` for its exit). `tests/lab-spec-model.test.ts`: delete the tests for the removed functions (`setOrder`, `setDirection`, `optimizablePaths`; their behaviour is covered in `lab-execution-model` and `lab-optimize-paths`), and replace `m.setOrder(...)` in "edits keep origin and unknown keys" with `setAt`.

- [ ] **Step 2: Run** the four files — fail.

- [ ] **Step 3: Implement.**
  - `RuleBuilder.tsx`:

```tsx
export function RuleBuilder({ spec, catalogue, instrument, onChange, settings, onSettings, problems, running, onRun }: {
  spec: Spec; catalogue: Catalogue; instrument: InstrumentContext; onChange(spec: Spec): void;
  settings: RunSettings; onSettings(patch: Partial<RunSettings>): void; problems: string[]; running: boolean; onRun(): void;
}) {
  const both = spec.direction === "both";
  const b = budget(spec), lim = limits(catalogue), bytes = specBytes(spec);
  const caps = (top: number): TreeCaps => ({ leaves: b.leaves >= lim.leaves, nodes: b.nodes >= lim.nodes, top: top >= lim.top });
  const warn = b.leaves >= 0.9 * lim.leaves || b.nodes >= 0.9 * lim.nodes || bytes >= 0.9 * SPEC_BYTE_BUDGET || b.top.long >= lim.top || b.top.short >= lim.top;
  const top = `Top-level ${b.top.long}/${lim.top}${both ? ` · short ${b.top.short}/${lim.top}` : ""}`;
  const list = (path: ListPath, title: string, tag: string, n: number) => (
    <RuleList listPath={path} title={title} tag={tag} catalogue={catalogue} caps={caps(n)}
      list={(getAt(spec, path) as RuleNode[]) ?? []} onChange={(l) => onChange(setAt(spec, path, l))} />
  );
  const money = modeOf(spec) === "money";
  const notes = instrumentNotes(instrument, spec);
  …
}
```
  Markup: `<section className="box lab-rules">` → `<h2 className="box-h">Strategy rules <small>{COPY.rulesHint}</small> <span className={`cap lab-caps${warn ? " lab-caps-warn" : ""}`} data-lab-caps>{COPY.caps(top, b.leaves, lim.leaves, b.nodes, lim.nodes, (bytes / 1024).toFixed(1))}</span></h2>` → `<ExecutionPanel spec catalogue onChange />` → long panel `<div className="lab-side-panel" data-lab-side="long">` (`{both && <h3 className="lab-side-h">{COPY.longSide}</h3>}`, `<div className="lab-sides">` with the entry list (title `both ? COPY.longEntryTitle : COPY.entryTitle`, tag `both ? "Long entry" : "Entry"`) and the exit list (`COPY.longExitTitle` / `COPY.exitTitle`), then `<OrdersPanel spec pre="" catalogue onChange />`) → with Both the same for `short_side.` (`data-lab-side="short"`, `COPY.shortSide`, `COPY.shortEntryTitle`, `COPY.shortExitTitle`, tags "Short entry"/"Short exit", `pre="short_side."`) → `<RiskPanel spec catalogue instrument onChange />` → caps (`.cap`): each instrument note, `COPY.sessionDatesHint` when `readsSessionDates(spec)`, `COPY.wmRule` when `usesHigherTimeframes(spec)` → the run row (unchanged, except money mode replaces the Fees and Slippage fields with `<span className="cap">{COPY.moneyCostsHere}</span>`; the fill caption becomes "fill at the order level" when any side has an order) → problems list. Delete the old condition rendering, `NEW_COND`, `SIDES`, `conditionCount` use.
  - `rules.css`: keep the existing rules that still apply (`.lab-rules h3/small`, `.lab-rules select`, `.lab-rules .dfield input`, `.lab-cond`, `.lab-operand`, `.lab-settings`, `.lab-two`, container query); add `.lab-rules { display: grid; gap: 12px; }` `.lab-rules > .box-h { margin-bottom: 0; }` `.lab-side-panel { display: grid; gap: 8px; }` `.lab-side-panel + .lab-side-panel { padding-top: 10px; border-top: 1px solid var(--rule); }` `.lab-side-h { margin: 0; font-size: 13px; }` `.lab-caps { margin: 0 0 0 8px; font-weight: 500; }` `.lab-caps-warn { color: var(--red); }` `.lab-settings { margin-top: 0; }`; delete `.lab-orders { margin: 0 0 10px }`, `.lab-price-orders` rules and `.lab-side > .dbtn` (superseded). Every gap inside the box stays ≤ 14 px.
  - `IndicatorsPage.tsx`: `const instrument = useMemo(() => instrumentContext(market, option), [market, option]);` problems: `[...specProblems(spec, cat), ...treeProblems(spec, cat), ...executionProblems(spec, cat, instrument)]`; pass `instrument` to `RuleBuilder`. `runQuery` already returns `{}` in money mode (Task 1) — keep using it for run and optimize.
  - `Workbench.tsx`: trade markers — long entry `belowBar arrowUp green "Long"`, short entry `aboveBar arrowDown red "Short"`, long exit `aboveBar arrowDown red "Exit"`, short exit `belowBar arrowUp green "Exit"`; when any loaded chip series has `adaptations`, render each text once as `<p className="rnote" role="note" data-lab-adaptations>` under the chart note.
  - `spec-model.ts`: delete `PRICE_ORDERS`, `PriceOrder`, `orderOf`, `setOrder`, `setDirection`, `optimizablePaths`, `OptimizableSetting`, `conditionCount` after `grep -rn` shows no importer outside tests you update in this task; keep `SPECIAL_OPERANDS` only if `OperandEditor` still imports it, otherwise delete it too.

- [ ] **Step 4: Run** `npx tsc --noEmit && npx vitest run` — everything passes (the full suite is green again at the end of wave 4).
- [ ] **Step 5: Browser acceptance** (full scenario): `node scripts/lab-browser-check.mjs <base> "$SCRATCH/t11" all`. Required true on all six layout views: `p7BothPanel`, `p7GroupAdded`, `p7RangeOp`, `p7Formula`, `p7StopOrder`, `p7TakeProfit`, `p7RiskOpen`, `p7MoneyBlockedOnIndex`, `p7Caps`, `p7RulesFullWidth`, `p7NoOverflow`, `p7PageGaps`, `p7RulesRhythm`, `p7SideRhythm`, `p7CompactControls`, `p7Indent`, `p7LeafTwoLines` (≥ 1400 px), `p7NoWhiteInMidnight` (midnight), `p7LibraryClosed`, and every legacy check. Live checks that need the engine: `p7Run`, `p7ShortTrades`, `p7NoEmptyColumns`, `p7PostNoOverflow`, `p7PostGaps` (batch 3), `p7IndexNote` (backend Task 12), `p7Adaptations`, `p7OptimizeNested`. Open every `p7-prerun-*` and `p7-postrun-*` PNG with the Read tool and confirm: rules full width before the run and beside the backtest after it at 1920/1425; long and short panels visibly separated by one rule line; the caps line on the header row; no white controls in Midnight; at 400 px every control wraps inside the box, groups indent ≤ 8 px, and nothing scrolls sideways.
- [ ] **Step 6: Commit (permission only):** `(lab-ui) Phase 7 rules box assembly, page wiring, markers, spec-model cleanup`.

---

### Task 12: Gate — static checks, contract checks against a fresh engine, real-browser run, guards, scope proof

**Files:** none in the repository (fixes go back to the owning task's files and tests). Screenshots and logs in `$SCRATCH/p7-gate/`.
**Depends on:** Tasks 1–11 and backend Tasks 10–15 merged in the working tree, with the backend gate (Task 16) passed.

- [ ] **Step 1: Static checks** (from `Context/guru/frontend/react`): `npm run typecheck && npx vitest run && npx vitest run --coverage && npx vite build --outDir "$SCRATCH/p7-gate/dist"` — all pass; coverage ≥ 80% on the include list (the three new pure modules included); a miss is fixed with tests in the owning task's file. (`--outDir` keeps `dist/` out of the tree.)

- [ ] **Step 2: Private stack** (never touch 5173 / 8188 / 18090 / 18091):

```bash
REPO=/home/aff34/.cache/strategy-vault-development
NAS="/run/user/1000/gvfs/smb-share:server=192.168.4.50,share=share/Manan Pujara/Book-to-rust"
cd "$REPO/Context/guru/backend" && CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$SCRATCH/p7-target" cargo build --release
cp "$SCRATCH/p7-target/release/guru" "$SCRATCH/guru-p7"
"$SCRATCH/guru-p7" "$NAS/idx_data.csv" 18092 "$NAS/india_vix" "$NAS/extra_data/india"                     # background
# optional gl engine when the staged gl file exists:
GL=/tmp/claude-1000/stage-gl-check/gl_data.csv; mkdir -p "$SCRATCH/gl-empty/no-vix"
[ -f "$GL" ] && "$SCRATCH/guru-p7" "$GL" 18093 "$SCRATCH/gl-empty/no-vix" "$SCRATCH/gl-empty/"             # background
cd "$REPO/Context/guru/frontend/react"
LAB_DEV_PROXY_PORT=8189 LAB_ENGINE_IN=http://127.0.0.1:18092 LAB_ENGINE_GL=http://127.0.0.1:18093 node scripts/lab-dev-proxy.mjs   # background
VAULT_DEV_PROXY=http://127.0.0.1:8189 npx vite --host 127.0.0.1 --port 5175 --strictPort                   # background
```
Wait until `curl -s http://127.0.0.1:5175/api/in/lab/catalogue | head -c 80` returns the lab label.

- [ ] **Step 3: Contract checks** — write `$SCRATCH/p7-gate/contract.py` (std-lib only) that GETs through `http://127.0.0.1:8189/api/in/lab/…` with `ticker=NIFTY 50` and asserts:
  1. catalogue: `max_leaves == 40`, `max_nodes == 120`, `max_query_bytes == 16384`, `execution.max_take_profits == 3`, `custom_days == [2, 63]`;
  2. run of a Both spec (long `close crosses_above sma 5`, short `close crosses_below sma 5`, long exit stop-loss 3% + take-profit `{pct:1,size:0.5}`): `execution_model == {kind:"index", mode:"unit", executable:false, notes:[<MODEL_NOTE.index>], events:[]}`, trades with `dir` −1 and 1, at least one trade with `fills`;
  3. the same spec with `execution.capital` (no cost query): HTTP 400 `needs_data`;
  4. a v1 spec with `execution.capital` and `cost=5`: 400 `invalid` containing "remove the cost and slippage query";
  5. a spec reading `{"s":"sessions_left","out":"m"}`: `adaptations` present; without it: key absent;
  6. `execution.instrument {kind:"futures"}`: 400 `needs_data`;
  7. an always-in spec (Both, empty exit lists): 200;
  8. a spec with 41 comparisons: 400 `invalid` mentioning 40 — and `treeProblems` reports the same spec (compare with a one-line `node -e` import is not possible for TS; instead record both messages side by side in the report).
  Record every response's status and first 300 bytes in the report. Money success (whole shares, ledger alignment) has no local data (no US ETF or India stock basket on this workstation): record it as an open item for the live server.

- [ ] **Step 4: Browser check:** `node scripts/lab-browser-check.mjs http://127.0.0.1:5175 "$SCRATCH/p7-gate/browser" all` — exit 0; every check true, no page errors. Open every PNG with the Read tool and check the items listed in Task 11 Step 5 plus: the `adaptations` note sits under the result label in plain text; the execution-model chip says "Study only · not executable" on NIFTY 50; the optimization result shows the same notes. Compare one number (trade count) between the UI and `curl` of the same `lab/run` query.

- [ ] **Step 5: Spot checks:** restart only the private proxy with `LAB_DEV_ROLE=viewer` → optimization shows "Researcher access is required for optimization."; with the gl engine up, `#/indicators?region=gl` on `IG:DAX` shows the provisional strip, Money disabled with the CFD reason, and a unit run's notes contain the CFD note.

- [ ] **Step 6: Stop the private stack** (proxy 8189, Vite 5175, engines 18092/18093) by PID; delete nothing else.

- [ ] **Step 7: Guards** (repo root): `python3 scripts/check_book_untouched.py --base origin/Manan` and `python3 scripts/check_upload.py` — exit 0.

- [ ] **Step 8: Scope proof:**

```bash
cd /home/aff34/.cache/strategy-vault-development
git diff --name-only "$LABUI7_BASE" -- Context/guru/frontend/react docs | sort -u   # expected: only files listed in this plan
for c in $(git log --format=%H "$LABUI7_BASE"..HEAD --grep='(lab-ui)'); do git show --name-only --format= "$c"; done | sort -u \
  | grep -Ev '^(Context/guru/frontend/react/|docs/)' && echo "OUT OF SCOPE" || echo "scope ok"
git diff --name-only "$LABUI7_BASE" -- Context/guru/frontend/react/src/compat Context/guru/frontend/react/src/pages/ResearchPages.tsx Context/guru/frontend/react/src/app
```
Expected: only plan files, `scope ok`, empty last output.

- [ ] **Step 9: Report** (no push): commits (if permitted) or `git status --short`, test counts before/after, coverage, the contract-check output, the browser report JSON, screenshot paths, the engine build's commit, and open items: money success path and ledger join on the live server (US ETF), the always-in and money behaviour if any backend task changed its contract, decisions 1–18 for the user's review.

---

## Self-review

**Phase 7 feature coverage** (spec section → task):
- §4.1 nodes: between/outside/rising/falling (T2 `withOp`, T6 operator switch), all/any/not (T2, T6), at_least (T2, T6), then (T2, T6); operators `= != crosses` (T1 labels, T6 select).
- §4.1 expressions: add/sub/mul/div/min/max, pct, abs, ref, sum/avg/highest/lowest, change (T2 convert/walk, T5 editors); state operands bars_since, value_when, highest/lowest_since, count (T2, T5 + T6 `renderNode`, T11 test); compress (T2, T5).
- §4.2 three-valued logic: engine-only; no UI change needed (the UI never evaluates rules).
- §4.3 limits: 40 leaves / 120 nodes / 12 per side / 8 KB / nesting 16 / window bounds / nth / steps (T2 `budget`, `treeProblems`, T11 caps line, T6 disabled adds).
- §4.4 conventions: `rising` meaning shown via operator label; Pine form expressible with formulas (T5).
- §5.1 timeframes D/W/M/Q/Y/nD, expand last/first/point with lookahead refusals, compress (T2 `timingProblem`, T5 timing toggle); `symbol` seam: unknown key kept, engine refuses (decision 18).
- §5.2 calendar operands and `sessions_left` adaptations (T1 labels, T5 calendar kind, T11 hint, T9/T10 result notes, T11 Workbench series note).
- §6.1 direction both + short_side + opposite (T3, T7, T11); entry stop / stop-limit / at / points / valid_bars / delay_bars / edge (T3, T7); trailing activate, breakeven, take_profits, min_bars, intrabar with explanation (T3, T7); cooldown, max_trades, risk (T3, T8); scale_in (T3, T7); sizing (T3, T8).
- §6.2–6.5 bar event order, same-bar rules, equity-level state: engine-only; the UI explains opposite, both-flat and intrabar (T1 copy, T7).
- §7 execution model per kind: pre-run notes and money gating (T3, T8, T11), result notes/events (T9, T10); India short hypothetical/refused, US borrow, index study, crypto, CFD (T1 `MODEL_NOTE`, `MONEY_BLOCKED`); futures note and refusal (T1, T3, T8).
- §7.4 sizing and costs: capital, commission, slippage bps, taxes ≤ 6, borrow; money refuses cost_bps and the query (T3, T8, T1 `runQuery`).
- §8 optimize paths: nested values, whole numbers, take-profits, activation, breakeven, short side, sizing, scale-in (T2 walker, T10).
- §9 API: catalogue keys (T1 types, T11 gating), `execution_model`, `money`, `ledger`, `legs`, `fills`, new `entry_kind` values, `adaptations` on run/optimize/series (T1, T9, T10, T11).

**Placeholder scan:** every task names exact files, functions, props, selectors, copy strings and test assertions. Component implementations are specified by structure, props, labels and CSS rather than full code; the pure modules (Tasks 2, 3, 10) carry their core code because the engine grammar lives there. Test expectations were traced by hand against the implementation rules (budget 7/13, knob order, label strings, caps line `0.2 of 7.5 KB`, optimize path order).

**Name consistency:** `RuleNode`, `EntryLevel`, `TakeProfit`, `ShortSide`, `Execution`, `ExecutionModel`, `MoneySummary`, `LedgerRow`, `Piece` (T1); `walkSpec`, `WalkItem`, `budget`, `limits`, `pathLabel`, `treeProblems`, `convertOperand`, `innerOperand`, `withOp`, `DEFAULT_COND`, `DEFAULT_STATE_COND`, `ListPath` (T2); `withDirection`, `withEntryOrder`, `levelAt`, `withLevel`, `LEVELS`, `TP_BOUNDS`, `withMode`, `modeOf`, `withSizingKind`, `instrumentContext`, `moneyBlocked`, `instrumentNotes`, `executionProblems`, `SidePre` (T3); `OperandEditor` props `heldOk`/`renderNode`/`nodesFull` (T5); `RuleList`, `NodeEditor`, `TreeCaps` (T6); `ExecutionPanel`, `OrdersPanel` (T7); `RiskPanel` (T8); `tradeTable`, `ExecutionStrip` (T9); `optimizablePaths`, `OptimizableSetting` in `optimize-paths.ts` (T10). Each is defined once, in its owning task, and used only by later waves.
