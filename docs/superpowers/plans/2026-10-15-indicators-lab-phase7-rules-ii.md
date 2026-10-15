# Indicators Lab — Phase 7 "Rules II" (every entry/exit condition, orders, instruments) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the lab rule spec express every entry and exit condition, order type, exit type, re-entry rule and risk rule that AmiBroker AFL or a TradingView Pine strategy can express on daily bars of one instrument. This covers AmiBroker's higher-timeframe TimeFrameSet / Compress / Expand semantics, without lookahead. It also adds an explicit execution model for cash equity, index/spot and futures. Every existing spec keeps its exact meaning, and book results are never touched.

**Architecture:** All engine changes are inside `Context/guru/backend/src/lab/`.
- `rules.rs` grows from a flat list of v1 comparisons into a node tree: groups, `not`, `at_least`, `then`, ranges and trends, plus expression and state operands. One generic walker serves optimize paths.
- `series.rs` compiles every new node into a three-valued 1/0/NaN array. A top-level v1 comparison keeps its unchanged code path.
- `resample.rs` gains Q, Y and n-session timeframes and the AmiBroker expand modes.
- `simulate.rs` becomes one state machine for one or two sides. It handles reversal, resting orders per side (limit, stop, stop-limit, `at` levels, valid bars), delay and edge entries, the new exits (trailing activation, breakeven, take-profits, min bars), the intrabar path option, scale-in, unit sizing and equity-level re-entry rules.
- Four new files:
  - `model.rs`: the per-instrument execution model;
  - `ledger.rs`: money mode, which turns quantities and costs into effective prices so the book's equity and statistics are reused read-only;
  - `futures.rs`: contract chain, rolls, causal continuous series, actual-price fills, margin calls and the expiry operand;
  - `examples.rs`: test-only documented examples and the trade-level no-lookahead test.
- `api.rs` adds the execution model, money mode and the futures `needs_data` refusal.

`main.rs`, the gateway, the auth service and the frontend are not touched.

**Tech Stack:** Rust 2021, std only (no new crates), `cargo test` with `CARGO_BUILD_JOBS=2`; Python 3 std-lib guard scripts; `git apply` for the task patches.

**Spec:** `docs/superpowers/specs/2026-10-15-indicators-lab-phase7-rules-ii-design.md`. Read §4–§10 and §13 (Assumptions for the user's review) before Task 1. Earlier plans set the conventions: `…-phase2-rules-engine.md` (rules, error contract), `2026-10-16-customize-book-strategies.md` (Customize fields, template fidelity, bar event order) and `…-phase6-all-tv-indicators.md` (catalogue, generic causality tests).

**Planning evidence:** Every patch below was produced by replaying the task edits in order on a scratch copy of HEAD, exported with `git archive` outside the repository.
- After each task, `cargo test lab::` passed. Lab test counts after Tasks 1–15: 277, 284, 287, 291, 297, 299, 306, 315, 321, 324, 328, 330, 335, 341, 349 (276 at HEAD).
- The 30 patches were then applied with `git apply`, in order, to a fresh HEAD extraction. The result is byte-identical to the verified files.
- The full engine suite with real NIFTY data (`VAULT_IDX`) passed: 920 passed / 1 ignored (848 / 1 at HEAD). A debug build showed no `src/lab/` warnings.
- Not run during planning: the book guard's engine compare, the release build, the gl smoke and the real-data smoke. Task 16 runs them.

---

## Pre-flight (read before Task 1)

- **The plan targets HEAD after Phase 6:** Phase 6's last commit `53d7308`, plus the docs-only spec commit `8136de0`. Confirm with `git log --oneline -3` and `git merge-base --is-ancestor 53d7308 HEAD && echo ok`.
- Frontend-only commits after `8136de0`, such as the `(lab-ui)` series, do not touch the backend; the patches were verified against a HEAD that already included them. If later commits touched `Context/guru/backend/src/lab/`, the patches may not apply. In that case, read each failing hunk's intent and reconcile by hand. Never weaken a test to make a patch fit.
- **Baselines.** Record them first:
  - `BASE=$(git rev-parse HEAD)`;
  - `cd Context/guru/backend && CARGO_BUILD_JOBS=2 cargo test lab:: 2>&1 | grep 'test result'` → 276 passed at planning time.
- **Applying a patch.** Every code step is a patch in a `bash` block (`git apply --whitespace=nowarn <<'PATCH' … PATCH`), run from the repository root.
  - Patches go in task order. Each test patch goes before its implementation patch.
  - If `git apply` refuses with a whitespace or context error, use `git apply --3way` or apply the hunk by hand. The hunk text is the specification.
- **Git writes need the user's explicit permission.** The user's rule from 2026-10-15: no `git commit`, `add`, `push`, `stash`, `reset`, `checkout` or other git write without explicit permission in the session. Each task's "Commit" step therefore says: commit only with that permission; otherwise stop at the end of the task, leave the changes in the working tree, and report the diff. When permission is given, commit on `Nandini` with no `Co-Authored-By` trailer and never push.

## Global Constraints

- Branch `Nandini`; never `main`; no force-push; no worktrees; never push (the user pushes). There is no git write without the user's explicit permission (Pre-flight). Commits carry no `Co-Authored-By` trailer.
- Book results are never touched.
  - Protected files get no edits: `connors.rs`, `books/**`, `readiness.rs`, `gate.rs`, `instrument_policy.rs`, `baskets.rs`, `accounting.rs`, `trade_validation.rs`, frontend `compat/*` and `ResearchPages.tsx`.
  - `main.rs`, gateway `policy.rs` and `custom-pages.ts` are not touched in this phase.
  - Book functions are only *called*: `trade_validation::check`, `equity_curve`, `stats`, `connors::run` in the fidelity tests.
- Scope: `Context/guru/backend/src/lab/**` only, plus the spec and this plan.
- Every existing spec keeps its exact meaning. `lab::pinned::existing_specs_keep_their_exact_trades` (Task 1) and `lab::templates::templates_reproduce_the_book_engine_on_fixtures` must pass after every task.
- No lookahead. A signal reads values known at the signal bar's close; higher-timeframe values become usable only after their period closes; `expand: first` is allowed only where the value is known at the period's first bar. Backward futures adjustment is refused. The only declared reader of future *dates* (never prices) is `sessions_left`.
- No result where the rules or data are unresolved:
  - futures answer `needs_data` on every real request;
  - money mode on an index, crypto composite or CFD answers `needs_data`;
  - an India cash short in money mode answers `needs_data`;
  - unit-mode India cash shorts are labelled hypothetical.
- Costs are user-declared. No statutory rate is hard-coded.
- Rust std only, `CARGO_BUILD_JOBS=2`, no whole-corpus backtest, NAS read-only, no data, `.env` or results in Git.
- NaN means no value. A comparison with NaN is false at the top of a rule list and unknown inside nodes (spec §4.2). Lab code never panics on user input; a bad field is a typed `invalid` error naming its path.

## Review Focus

The five failure modes most likely to bite a user, and the test that pins each one:

1. **An existing spec changes meaning.** This covers a v1 strategy, a Customize template or a saved spec trading differently after an engine change. Pinned by `lab::pinned::existing_specs_keep_their_exact_trades` (Task 1; hash of 4,422 template trades and 1,366 plain-spec trades on four fixtures) and the template fidelity test, both run in every task.
2. **Lookahead through a new operand.** Examples are expandFirst of a high or close, a Q/Y value used inside its quarter, a sequence or `bars_since` peeking ahead, a back-adjusted futures series, or an order level taken after the signal bar. Pinned by:
   - `nodes_never_use_later_bars` (Task 2), `expressions_never_use_later_bars` (Task 3), `state_operands_never_use_later_bars` (Task 4);
   - `each_expand_mode_is_causal_and_placed_like_amibroker`, `lookahead_expand_first_is_refused_with_a_note`, `expand_modes_and_compress_never_use_later_bars`, and the generic causality tests over Q, Y and 5D (Task 5);
   - `later_bars_never_change_earlier_series_or_trades` (Task 14) and `phase7_features_never_use_later_bars` (Task 15).
3. **A number that claims something cannot be executed.** Money sizing on an index, a futures result from index prices, an overnight India cash short in money mode, or a US short without a borrow assumption. Pinned by `each_kind_says_what_its_result_means` (Task 12) and `money_mode_books_whole_shares_on_an_etf_and_refuses_an_index` (Tasks 13–14: index money, US short borrow, futures refusal on run and optimize).
4. **Ambiguous same-bar events resolved optimistically.** Stop and target in one bar, two resting orders touched in one bar, a reversal and a stop in one bar, a stop-limit triggered inside the bar. Pinned by:
   - `stop_limit_fills_only_where_the_bar_proves_it`, `both_sides_rest_an_order_while_flat_and_the_touched_one_fills`, `a_resting_reversal_order_flips_the_position_at_its_level` (Task 8);
   - `ohlc_path_takes_the_nearer_extreme_first` (Task 9);
   - the existing `stop_and_target_inside_one_bar_take_the_stop`.
5. **The optimizer writes an impossible spec.** Examples are `at_least` above `of`, a window of 0, a sizing percentage above 100, or a path into the wrong side. Pinned by `optimize_reaches_numbers_inside_nodes` (Task 2), `expressions_parse_validate_and_reach_optimize` (Task 3), `optimize_reaches_the_short_side` (Task 7), `entry_orders_parse_and_validate` (Task 8), `exits_parse_validate_and_reach_optimize` (Task 9) and `scale_in_and_sizing_parse_and_validate` (Task 10).

## Decisions made while planning (in addition to spec §13)

These were found while the code was compile-checked. Each is already reflected in the spec's current text.

1. **Futures adjustment is forward (causal) by default, and `back` is refused.** Backward adjustment rewrites earlier bars at every roll; the futures causality test failed with it. *Cost:* historical levels differ from TradingView B-ADJ charts.
2. **One resting order per side.** Two orders touched on one bar fill nothing. This is what TradingView's Channel BreakOut (E4) needs, with a long stop and a short stop resting together. *Cost:* the rare bar touching both differs from TradingView's path.
3. **A delayed exit is not re-placed while it is pending.** Without this guard, `delay_bars > 1` kept postponing exits. *Cost:* none.
4. **Money sizing uses equity at the trade's start and the entry fill price.** Trades do not carry their entry signal bar. *Cost:* a small difference from TradingView's signal-close sizing.
5. **`expiry.days_to_expiry` counts Monday–Friday dates to the contract's published expiry date (causal), not data sessions.** *Cost:* an exchange holiday before expiry is counted.
6. **Optimize paths reach `execution.sizing.pct_equity|risk_pct` and `entry.scale_in.min_bars`; take-profit sizes are not optimizable.** *Cost:* sizes are edited by hand.

## File structure

```
Context/guru/backend/src/lab/
  pinned.rs    NEW (test-only) compatibility pin: trade hashes of every template and four v1 specs     (Task 1)
  rules.rs     node tree, operators, expressions, state operands, expand modes, two sides, orders, exits,
               scale-in, sizing, re-entry/risk, money costs, futures instrument, generic optimize walker   (Tasks 2–5, 7–11, 13–15)
  series.rs    three-valued node masks, expression/state evaluation, Q/Y/nD frames, expand modes, compress,
               Compiled.other/cost/limit_at/stop_at/tick, short-side compile                              (Tasks 2–10, 14)
  resample.rs  Tf::Q/Y/Days(n), Tf::name, Expand, expand_mode                                              (Task 5)
  periods.rs   `session` and `sessions_left` indicators                                                  (Task 6)
  catalogue.rs / sources.rs  master list 299 → 302, generic-test exemptions                              (Tasks 6, 14)
  simulate.rs  two sides, reversal, per-side resting orders, delay, edge, new exits, scale-in, sizing,
               cooldown / max trades / risk, simulate_events                                              (Tasks 7–11, 14)
  model.rs     NEW execution model per instrument kind                                                    (Task 12)
  ledger.rs    NEW money mode: quantities, declared costs, effective prices                               (Task 13)
  futures.rs   NEW contract chain, rolls, continuous series, actual-price trades, margin calls, `expiry`  (Task 14)
  bars.rs      Bars.expiry (futures runs only)                                                            (Task 14)
  examples.rs  NEW (test-only) documented AmiBroker/TradingView examples, trade-level no-lookahead test   (Task 15)
  api.rs       tf names, execution_model, legs/fills JSON, catalogue limits, money mode, futures refusal  (Tasks 5, 12–14)
  mod.rs       module lines                                                                                (Tasks 1, 12–15)
```

## How each task is written

Each task lists its files, the interfaces it consumes and produces, then five steps:
1. the test patch, which adds the new tests and, where noted, edits existing tests mechanically;
2. a run that shows the failure: compile errors naming the missing items, or failing assertions;
3. the implementation patch;
4. a run that shows every lab test passing, with the expected count;
5. the commit, with permission only.

New files arrive whole in the implementation patch. Their `#[cfg(test)] mod tests` at the end is the test-first part: read it first.

Run lab tests with `cd Context/guru/backend && CARGO_BUILD_JOBS=2 cargo test lab::` (about 5 s after the build).

### Task 1: Compatibility pin — every template and four v1 specs keep their exact trades

Spec §8. Before any engine change, this hashes the trade lists of all 43 Customize templates and four plain v1 specs (next-open, close fill, stop/target, trailing ATR, short with a monthly operand and an ATR limit) on four seeded 1,500-bar fixtures. Every later task must keep this test green.

**Files:**
- Modify: `Context/guru/backend/src/lab/mod.rs`
- Create: `Context/guru/backend/src/lab/pinned.rs`

**Interfaces:**
- Consumes: `templates::{TEMPLATES, parsed}`, `rules::parse_spec`, `series::{compile, Cache, Frames}`, `simulate::simulate`, `testdata::walk` (all existing).
- Produces: `lab::pinned` (test-only module) with `existing_specs_keep_their_exact_trades`, pinned at ((8551371247103554118, 4422), (14900454072103790734, 1366)).

- [ ] **Step 1: Write the failing tests** (apply from the repository root)

```bash
git apply --whitespace=nowarn <<'PATCH'
diff --git a/Context/guru/backend/src/lab/mod.rs b/Context/guru/backend/src/lab/mod.rs
--- a/Context/guru/backend/src/lab/mod.rs
+++ b/Context/guru/backend/src/lab/mod.rs
@@ -39,3 +39,5 @@
 pub mod testdata;
 #[cfg(test)]
 pub mod parity;
+#[cfg(test)]
+pub mod pinned;
diff --git a/Context/guru/backend/src/lab/pinned.rs b/Context/guru/backend/src/lab/pinned.rs
new file mode 100644
--- /dev/null
+++ b/Context/guru/backend/src/lab/pinned.rs
@@ -0,0 +1,40 @@
+//! Compatibility pin (Phase 7 spec §8): the trade lists of every Customize template and of four plain v1 specs, hashed on
+//! fixed synthetic data at Phase 6 HEAD. Any change to what an existing spec means changes the hash.
+use super::bars::Bars;
+use super::rules::parse_spec;
+use super::series::{compile, Cache, Frames};
+use super::simulate::simulate;
+use super::templates::{parsed, TEMPLATES};
+use super::testdata::walk;
+
+/// FNV-1a over every field of every trade.
+fn fold(h: &mut u64, bytes: &[u8]) { for b in bytes { *h ^= u64::from(*b); *h = h.wrapping_mul(0x100_0000_01b3); } }
+fn digest(specs: &[super::rules::Spec]) -> (u64, usize) {
+    let (mut h, mut n) = (0xcbf2_9ce4_8422_2325u64, 0);
+    for seed in 1..=4u64 {
+        let b: Bars = walk(1500, seed, true);
+        let fr = Frames::new(b.clone());
+        for s in specs {
+            for t in simulate(&compile(s, &fr, &mut Cache::new()).unwrap(), &b, 0, b.len() - 1) {
+                for l in &t.legs { fold(&mut h, &(l.i as u64).to_le_bytes()); fold(&mut h, &l.px.to_bits().to_le_bytes()); fold(&mut h, &l.w.to_bits().to_le_bytes()); }
+                fold(&mut h, &(t.exit_i as u64).to_le_bytes()); fold(&mut h, &t.exit_px.to_bits().to_le_bytes());
+                fold(&mut h, &t.dir.to_bits().to_le_bytes()); fold(&mut h, &[t.open as u8]); fold(&mut h, t.reason.as_bytes());
+                fold(&mut h, t.entry_kind.as_bytes()); fold(&mut h, &(t.signal_i as u64).to_le_bytes());
+                n += 1;
+            }
+        }
+    }
+    (h, n)
+}
+pub const PLAIN: [&str; 4] = [
+    r#"{"v":1,"entry":{"all":[{"lhs":{"s":"rsi","p":{"length":2}},"op":"crosses_below","rhs":{"value":10}},{"lhs":{"s":"close"},"op":">","rhs":{"s":"sma","p":{"length":40},"tf":"W"}}]},"exit":{"any":[{"lhs":{"s":"rsi","p":{"length":2}},"op":"crosses_above","rhs":{"value":70}},{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":10}}]}}"#,
+    r#"{"v":1,"entry":{"all":[{"lhs":{"s":"rsi","p":{"length":2}},"op":"<","rhs":{"value":20}}]},"exit":{"any":[],"stop_loss":{"pct":2},"target":{"pct":3}},"execution":{"fill":"close"}}"#,
+    r#"{"v":1,"entry":{"all":[{"lhs":{"s":"close"},"op":">","rhs":{"s":"sma","p":{"length":5}}}]},"exit":{"any":[{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":8}}],"trailing":{"atr":2,"atr_length":10}}}"#,
+    r#"{"v":1,"direction":"short","entry":{"all":[{"lhs":{"s":"macd","out":"signal","tf":"M","offset":1},"op":"<","rhs":{"s":"macd","tf":"M"}},{"lhs":{"s":"rsi","p":{"length":2}},"op":">","rhs":{"value":80}}],"limit":{"atr":0.4,"atr_length":10}},"exit":{"any":[{"lhs":{"s":"rsi","p":{"length":2}},"op":"<","rhs":{"value":40}}],"stop_loss":{"atr":3}},"execution":{"exit_fill":"close"}}"#,
+];
+
+#[test] fn existing_specs_keep_their_exact_trades() {
+    let templates: Vec<_> = TEMPLATES.iter().map(parsed).collect();
+    let plain: Vec<_> = PLAIN.iter().map(|t| parse_spec(t).unwrap()).collect();
+    assert_eq!((digest(&templates), digest(&plain)), ((8551371247103554118, 4422), (14900454072103790734, 1366)), "pinned at Phase 6 HEAD 53d7308; a change here means an existing spec changed meaning");
+}
PATCH
```

- [ ] **Step 2: Run the pin at HEAD (it must pass)**

Run: `cd Context/guru/backend && CARGO_BUILD_JOBS=2 cargo test lab::pinned 2>&1 | tail -20`
Expected: This task only adds a test. Run it at once: it must **pass** at HEAD. If the hash differs, HEAD is not the Phase 6 tree this plan was made on. Stop and report rather than re-pinning.

- [ ] **Step 3: Run all lab tests**

Run: `cd Context/guru/backend && CARGO_BUILD_JOBS=2 cargo test lab:: 2>&1 | grep 'test result'`
Expected: `ok. 277 passed; 0 failed` (276 at HEAD plus this phase's tests so far; a different count means a test is missing or a patch was not fully applied). `lab::pinned::existing_specs_keep_their_exact_trades` and `lab::templates::templates_reproduce_the_book_engine_on_fixtures` must be among the passing tests.

- [ ] **Step 4: Commit (only with the user's explicit permission for git writes)**

With permission: `git add Context/guru/backend/src/lab && git commit -m "test(lab): pin the trades of every template and four v1 specs before Rules II"` (no trailer, no push). Without it: stop here and report `git status --short` and `git diff --stat`.

---
### Task 2: Rule nodes — groups, not, ranges, trends, at-least-k-of-m, sequences, three-valued logic, nested optimize paths

Spec §4.1–§4.3. `Spec.entry` and `Spec.exit` become `Vec<Node>`. `Node::Cmp(Cond)` is the v1 condition and compiles exactly as before. Every other node compiles to a 1/0/NaN mask tested with `> 0.5` at the top. A whole-spec budget caps comparisons at 40 and nodes plus expressions at 120. One generic walker replaces the old optimize `Target` fields, so paths reach any number inside nodes.

**Files:**
- Modify: `Context/guru/backend/src/lab/rules.rs`
- Modify: `Context/guru/backend/src/lab/series.rs`

**Interfaces:**
- Consumes: Task 1 (the pin).
- Produces: `rules::{Node, Op::{Eq, Ne, Crosses}, NODE_KINDS, MAX_LEAVES, MAX_NODES, MAX_STEPS, Target { path, slot }}`; `Node::cmp()`; `series::{mask, describe_node, operand_text, op_text}`; private `rules::{Budget, Knob, walk, walk_node, walk_operand, consistent, whole}`.

> Note: The test patch also edits four existing assertions in `rules.rs` mechanically: `s.entry[k].rhs` → `s.entry[k].cmp().unwrap().rhs` (three lines) and `s.exit[1].lhs` → `s.exit[1].cmp().unwrap().lhs`. Their meaning is unchanged.

> Note: The optimize error for a path that names no number now reads "names no number in the spec …". No existing test asserted the old text.

- [ ] **Step 1: Write the failing tests** (apply from the repository root)

```bash
git apply --whitespace=nowarn <<'PATCH'
diff --git a/Context/guru/backend/src/lab/rules.rs b/Context/guru/backend/src/lab/rules.rs
--- a/Context/guru/backend/src/lab/rules.rs
+++ b/Context/guru/backend/src/lab/rules.rs
@@ -359,18 +492,18 @@
     #[test] fn spec_example_round_trips() {
         let s = parse_spec(EXAMPLE).unwrap();
         assert_eq!((s.entry.len(), s.exit.len(), s.cost_bps), (2, 2, Some(12.)));
-        let Operand::Series(r) = &s.entry[1].rhs else { panic!() };
+        let Operand::Series(r) = &s.entry[1].cmp().unwrap().rhs else { panic!() };
         assert_eq!((r.id.as_str(), r.tf, r.params["length"]), ("sma", Tf::W, 40.));
-        assert_eq!(s.exit[1].lhs, Operand::BarsHeld);
+        assert_eq!(s.exit[1].cmp().unwrap().lhs, Operand::BarsHeld);
         assert_eq!(s.optimize[0].values, vec![2., 3., 4., 5., 6.]);
         assert_eq!(current(&s, &s.optimize[0].target), 2.);
         let t = apply(&s, &s.optimize, &[5.]);
-        let Operand::Series(r) = &t.entry[0].lhs else { panic!() };
+        let Operand::Series(r) = &t.entry[0].cmp().unwrap().lhs else { panic!() };
         assert_eq!(r.params["length"], 5.);
     }
     #[test] fn defaults_fill_params_tf_and_output() {
         let s = parse_spec(&with(r#"{"s":"close","tf":"D"}"#, r#"{"s":"macd","out":"signal"}"#)).unwrap();
-        let Operand::Series(r) = &s.entry[1].lhs else { panic!() };
+        let Operand::Series(r) = &s.entry[1].cmp().unwrap().lhs else { panic!() };
         assert_eq!((r.tf, r.out, r.params["fast"], r.params["slow"]), (Tf::D, 1, 12., 26.));
     }
     #[test] fn malformed_input_is_invalid_and_names_the_field() {
@@ -498,4 +631,73 @@
         let j = LabError::Invalid("bad \"x\"\n".into()).json();
         assert!(j.starts_with("{\"status\":\"invalid\",\"error\":\"bad \\\"x\\\"\\u000a\",\"label\":\"User-defined indicator strategy"), "{j}");
     }
-}
+
+    // ---- Phase 7: nodes (spec §4.1–§4.3)
+    const ONE_EXIT: &str = r#"{"v":1,"entry":{"all":[NODE]},"exit":{"any":[{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":5}}]}}"#;
+    #[test] fn nodes_parse_and_name_their_fields() {
+        let s = parse_spec(r#"{"v":1,"entry":{"all":[
+            {"any":[{"lhs":{"s":"close"},"op":"=","rhs":{"value":1}},{"not":{"lhs":{"s":"close"},"op":"crosses","rhs":{"s":"open"}}}]},
+            {"lhs":{"s":"rsi"},"op":"between","lo":{"value":20},"hi":{"value":40}},
+            {"lhs":{"s":"close"},"op":"rising","bars":3},
+            {"at_least":3,"of":5,"when":{"lhs":{"s":"close"},"op":">","rhs":{"s":"open"}}},
+            {"then":[{"lhs":{"s":"rsi"},"op":"<","rhs":{"value":30}},{"lhs":{"s":"rsi"},"op":">","rhs":{"value":50}}],"within":10}]},
+            "exit":{"any":[{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":5}}]}}"#).unwrap();
+        assert!(matches!(&s.entry[0], Node::Any(v) if matches!(&v[1], Node::Not(_))));
+        assert!(matches!(&s.entry[1], Node::Range { outside: false, .. }));
+        assert!(matches!(&s.entry[2], Node::Trend { bars: 3, rising: true, .. }));
+        assert!(matches!(&s.entry[3], Node::AtLeast { k: 3, of: 5, .. }));
+        assert!(matches!(&s.entry[4], Node::Then { within: 10, .. }));
+        for (node, needle) in [
+            (r#"{"all":[]}"#, "entry.all[0].all needs at least one entry"),
+            (r#"{"at_least":4,"of":3,"when":{"lhs":{"s":"close"},"op":">","rhs":{"value":1}}}"#, "at_least (4) cannot exceed of (3)"),
+            (r#"{"then":[{"lhs":{"s":"close"},"op":">","rhs":{"value":1}}],"within":3}"#, "then needs 2 to 4 steps"),
+            (r#"{"lhs":{"s":"close"},"op":"rising","bars":0}"#, "entry.all[0].bars must be a whole number from 1"),
+            (r#"{"lhs":{"s":"close"},"op":"between","lo":{"value":1}}"#, "entry.all[0].hi is required"),
+            (r#"{"lhs":{"s":"close"},"op":"between","lo":{"value":1},"hi":{"value":2},"rhs":{"value":3}}"#, "unknown field 'rhs' in entry.all[0]"),
+            (r#"{"not":{"lhs":{"s":"bars_held"},"op":">","rhs":{"value":1}}}"#, "only available in exit"),
+            (r#"{"maybe":[]}"#, "must be a comparison (lhs, op, rhs) or a group (all, any, not, at_least, then)"),
+        ] {
+            let e = err(&ONE_EXIT.replace("NODE", node));
+            assert_eq!(e.status(), "invalid", "{node}");
+            assert!(e.message().contains(needle), "{needle:?} not in {:?}", e.message());
+        }
+        // bars_held stays a top-level exit operand: inside an exit group it is refused
+        let grouped = ONE_EXIT.replace("NODE", r#"{"lhs":{"s":"close"},"op":">","rhs":{"value":1}}"#)
+            .replace(r#""any":[{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":5}}]"#, r#""any":[{"any":[{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":5}}]}]"#);
+        assert!(err(&grouped).message().contains("only available in exit"));
+    }
+    #[test] fn whole_spec_bounds_count_nested_comparisons_and_nodes() {
+        let c = r#"{"lhs":{"s":"close"},"op":">","rhs":{"value":1}}"#;
+        let group = |n: usize| format!(r#"{{"all":[{}]}}"#, vec![c; n].join(","));
+        let spec = |items: Vec<String>| format!(r#"{{"v":1,"entry":{{"all":[{}]}},"exit":{{"any":[{c}]}}}}"#, items.join(","));
+        assert!(parse_spec(&spec(vec![group(13), group(13), group(13)])).is_ok()); // 39 nested + 1 exit = 40 comparisons
+        assert!(err(&spec(vec![group(13), group(13), group(14)])).message().contains("more than 40 comparisons"));
+        let nots = format!("{}{c}{}", r#"{"not":"#.repeat(10), "}".repeat(10)); // 11 nodes, 1 comparison
+        assert!(err(&spec(vec![nots; 11])).message().contains("more than 120 comparisons, groups and expressions"));
+    }
+    #[test] fn optimize_reaches_numbers_inside_nodes() {
+        let text = r#"{"v":1,"entry":{"all":[{"any":[{"lhs":{"s":"rsi","p":{"length":2}},"op":"<","rhs":{"value":10}}]},{"at_least":2,"of":3,"when":{"lhs":{"s":"close"},"op":"rising","bars":2}}]},
+            "exit":{"any":[{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":5}}]},
+            "optimize":[{"path":"entry.all[0].any[0].rhs.value","from":5,"to":15,"step":5},{"path":"entry.all[0].any[0].lhs.p.length","from":2,"to":3,"step":1},{"path":"entry.all[1].when.bars","from":1,"to":3,"step":1},{"path":"entry.all[1].at_least","from":1,"to":3,"step":1}]}"#;
+        let s = parse_spec(text).unwrap();
+        assert_eq!(s.optimize.iter().map(|d| current(&s, &d.target)).collect::<Vec<_>>(), vec![10., 2., 2., 2.]);
+        let t = apply(&s, &s.optimize, &[15., 3., 3., 1.]);
+        let Node::Any(v) = &t.entry[0] else { panic!() };
+        let c = v[0].cmp().unwrap();
+        let Operand::Series(r) = &c.lhs else { panic!() };
+        assert_eq!((&c.rhs, r.params["length"]), (&Operand::Value(15.), 3.));
+        assert!(matches!(&t.entry[1], Node::AtLeast { k: 1, of: 3, when } if matches!(**when, Node::Trend { bars: 3, .. })));
+        let one = |path: &str, from: f64, to: f64| text.split(r#""optimize":"#).next().unwrap().to_string()
+            + &format!(r#""optimize":[{{"path":"{path}","from":{from},"to":{to},"step":1}}]}}"#);
+        for (path, from, to, needle) in [
+            ("entry.all[1].at_least", 1., 4., "at_least (4) cannot exceed of (3)"),
+            ("entry.all[1].when.bars", 0., 2., "entry.all[1].when.bars must be a whole number from 1"),
+            ("entry.all[0].any[3].rhs.value", 1., 2., "names no number in the spec"),
+            ("entry.all[0].any[0].lhs.p.len", 1., 2., "rsi has no parameter 'len'"),
+            ("entry.all[2].lhs.p.length", 1., 2., "no condition at that index"),
+        ] {
+            let e = err(&one(path, from, to));
+            assert!(e.message().contains(needle), "{path}: {needle:?} not in {:?}", e.message());
+        }
+    }
+}
diff --git a/Context/guru/backend/src/lab/series.rs b/Context/guru/backend/src/lab/series.rs
--- a/Context/guru/backend/src/lab/series.rs
+++ b/Context/guru/backend/src/lab/series.rs
@@ -274,4 +377,73 @@
         assert_eq!(c.entry[0].label, "entry.all[0]: macd(fast=12,signal=9,slow=26).signal@W[1] < 0");
         assert_eq!(c.exit[0].label, "exit.any[0]: bars_held >= 3");
     }
-}
+
+    // ---- Phase 7: nodes (spec §4.2)
+    fn node(t: &str) -> Node {
+        parse_spec(&format!(r#"{{"v":1,"entry":{{"all":[{t}]}},"exit":{{"any":[{{"lhs":{{"s":"bars_held"}},"op":">=","rhs":{{"value":1}}}}]}}}}"#)).unwrap().entry.remove(0)
+    }
+    #[test] fn nodes_use_three_valued_logic() {
+        let (fr, x) = (Frames::new(crate::lab::testdata::closes(&[1., 2., 3., 2., 4., 5., 6.])), f64::NAN);
+        let m = |t: &str| mask(&node(t), &fr, &mut Cache::new()).unwrap();
+        let up = r#"{"lhs":{"s":"close"},"op":">","rhs":{"s":"close","offset":1}}"#;
+        let big = r#"{"lhs":{"s":"close"},"op":">","rhs":{"value":3}}"#;
+        let small = r#"{"lhs":{"s":"close"},"op":"<","rhs":{"value":2}}"#;
+        assert_series(&m(up), &[x, 1., 1., 0., 1., 1., 1.]);
+        assert_series(&m(&format!(r#"{{"not":{up}}}"#)), &[x, 0., 0., 1., 0., 0., 0.]);
+        assert_series(&m(&format!(r#"{{"all":[{up},{big}]}}"#)), &[0., 0., 0., 0., 1., 1., 1.]); // a false child decides
+        assert_series(&m(&format!(r#"{{"any":[{up},{small}]}}"#)), &[1., 1., 1., 0., 1., 1., 1.]); // a true child decides
+        assert_series(&m(&format!(r#"{{"any":[{up},{big}]}}"#)), &[x, 1., 1., 0., 1., 1., 1.]); // otherwise unknown stays unknown
+        assert_series(&m(r#"{"lhs":{"s":"close"},"op":"between","lo":{"value":2},"hi":{"value":4}}"#), &[0., 1., 1., 1., 1., 0., 0.]);
+        assert_series(&m(r#"{"lhs":{"s":"close"},"op":"outside","lo":{"value":2},"hi":{"value":4}}"#), &[1., 0., 0., 0., 0., 1., 1.]);
+        assert_series(&m(r#"{"lhs":{"s":"close"},"op":"rising","bars":2}"#), &[x, x, 1., 0., 0., 1., 1.]);
+        assert_series(&m(r#"{"lhs":{"s":"close"},"op":"falling","bars":1}"#), &[x, 0., 0., 1., 0., 0., 0.]);
+        assert_series(&m(&format!(r#"{{"at_least":2,"of":3,"when":{up}}}"#)), &[x, x, 1., 1., 1., 1., 1.]);
+        assert_series(&m(&format!(r#"{{"at_least":3,"of":3,"when":{up}}}"#)), &[x, x, 0., 0., 0., 0., 1.]);
+        assert_series(&m(r#"{"lhs":{"s":"close"},"op":"=","rhs":{"value":2}}"#), &[0., 1., 0., 1., 0., 0., 0.]);
+        assert_series(&m(r#"{"lhs":{"s":"close"},"op":"!=","rhs":{"value":2}}"#), &[1., 0., 1., 0., 1., 1., 1.]);
+        assert_series(&m(r#"{"lhs":{"s":"close"},"op":"crosses","rhs":{"value":2.5}}"#), &[x, 0., 1., 1., 1., 0., 0.]);
+        // the same comparison at the top of a rule list (v1 path) treats unknown as false
+        let top = RCond { lhs: Val::Arr(m(up)), op: Op::Gt, rhs: Val::Const(0.5), label: String::new() };
+        assert!(!top.holds(0, None) && top.holds(1, None));
+    }
+    #[test] fn sequences_need_steps_in_order_within_the_span() {
+        let fr = Frames::new(crate::lab::testdata::closes(&[1., 3., 3., 3., 5., 1., 5., 3., 5.]));
+        let (a, b) = (r#"{"lhs":{"s":"close"},"op":"<","rhs":{"value":2}}"#, r#"{"lhs":{"s":"close"},"op":">","rhs":{"value":4}}"#);
+        let m = |steps: &str, within: usize| mask(&node(&format!(r#"{{"then":[{steps}],"within":{within}}}"#)), &fr, &mut Cache::new()).unwrap();
+        // A on bars 0 and 5, B on 4, 6 and 8: B counts when an A came strictly earlier and at most `within` bars before
+        assert_series(&m(&format!("{a},{b}"), 4), &[0., 0., 0., 0., 1., 0., 1., 0., 1.]);
+        assert_series(&m(&format!("{a},{b}"), 3), &[0., 0., 0., 0., 0., 0., 1., 0., 1.]);
+        assert_series(&m(&format!("{a},{b},{a}"), 6), &[0., 0., 0., 0., 0., 1., 0., 0., 0.]);
+        // one bar is never two steps
+        let same = Frames::new(crate::lab::testdata::closes(&[5., 5.]));
+        assert_series(&mask(&node(&format!(r#"{{"then":[{b},{b}],"within":5}}"#)), &same, &mut Cache::new()).unwrap(), &[0., 1.]);
+    }
+    #[test] fn nodes_never_use_later_bars() {
+        let full = business_days(420, true);
+        let spec = parse_spec(r#"{"v":1,"entry":{"all":[
+            {"any":[{"not":{"lhs":{"s":"close"},"op":"crosses","rhs":{"s":"sma","p":{"length":10},"tf":"W"}}},{"lhs":{"s":"rsi"},"op":"=","rhs":{"value":50}}]},
+            {"lhs":{"s":"close"},"op":"between","lo":{"s":"bb","out":"lower"},"hi":{"s":"bb","out":"upper"}},
+            {"lhs":{"s":"close"},"op":"rising","bars":3},
+            {"at_least":3,"of":7,"when":{"lhs":{"s":"close"},"op":">","rhs":{"s":"open"}}},
+            {"then":[{"lhs":{"s":"rsi"},"op":"<","rhs":{"value":40}},{"lhs":{"s":"rsi"},"op":">","rhs":{"value":60}}],"within":12}]},
+            "exit":{"any":[{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":5}}]}}"#).unwrap();
+        for n in &spec.entry {
+            let want = mask(n, &Frames::new(full.clone()), &mut Cache::new()).unwrap();
+            assert!(want.iter().any(|v| *v == 1.) && want.iter().any(|v| *v == 0.), "{}: vacuous", describe_node(n));
+            for cut in [97usize, 203, 311] {
+                let got = mask(n, &Frames::new(crate::lab::testdata::cut(&full, cut)), &mut Cache::new()).unwrap();
+                for i in 0..cut - 1 { assert!(close_eq(got[i], want[i]), "{} bar {i} changes when bars after {cut} exist", describe_node(n)); }
+            }
+        }
+    }
+    #[test] fn groups_compile_with_readable_labels_and_v1_comparisons_keep_theirs() {
+        let spec = parse_spec(r#"{"v":1,"entry":{"all":[{"lhs":{"s":"close"},"op":">","rhs":{"value":1}},{"any":[{"lhs":{"s":"close"},"op":"<","rhs":{"value":5}},{"not":{"lhs":{"s":"close"},"op":"crosses","rhs":{"s":"open"}}}]}]},
+            "exit":{"any":[{"at_least":2,"of":3,"when":{"lhs":{"s":"close"},"op":"falling","bars":1}},{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":3}}]}}"#).unwrap();
+        let c = compile(&spec, &Frames::new(business_days(60, false)), &mut Cache::new()).unwrap();
+        assert_eq!(c.entry[0].label, "entry.all[0]: close > 1");
+        assert!(matches!(c.entry[0].lhs, Val::Arr(_)) && c.entry[0].op == Op::Gt);
+        assert_eq!(c.entry[1].label, "entry.all[1]: any(close < 5, not(close crosses open))");
+        assert_eq!(c.exit[0].label, "exit.any[0]: at_least 2 of 3(close falling 1 bars)");
+        assert!(matches!(c.exit[1].lhs, Val::Held));
+    }
+}
PATCH
```

- [ ] **Step 2: Run the tests and confirm the failure**

Run: `cd Context/guru/backend && CARGO_BUILD_JOBS=2 cargo test lab:: 2>&1 | tail -20`
Expected: Compile errors: `Node`, `mask` and `describe_node` are not defined, and `cmp()` is not found.

- [ ] **Step 3: Implement** (apply from the repository root)

```bash
git apply --whitespace=nowarn <<'PATCH'
diff --git a/Context/guru/backend/src/lab/rules.rs b/Context/guru/backend/src/lab/rules.rs
--- a/Context/guru/backend/src/lab/rules.rs
+++ b/Context/guru/backend/src/lab/rules.rs
@@ -9,7 +9,13 @@
 pub const MAX_SPEC_BYTES: usize = 8192;
 pub const MAX_OFFSET: usize = 1000;
 pub const PRICE_SOURCES: &[&str] = &["open", "high", "low", "close", "hl2", "hlc3", "ohlc4", "volume"];
-pub const OPERATORS: &[&str] = &["crosses_above", "crosses_below", ">", "<", ">=", "<="];
+pub const OPERATORS: &[&str] = &["crosses_above", "crosses_below", ">", "<", ">=", "<=", "=", "!=", "crosses", "between", "outside", "rising", "falling"];
+/// Group kinds a rule list may hold besides comparisons (spec §4.1).
+pub const NODE_KINDS: &[&str] = &["all", "any", "not", "at_least", "then"];
+/// Whole-spec bounds (spec §4.3): comparisons, and comparisons + groups + expressions, over every rule list.
+pub const MAX_LEAVES: usize = 40;
+pub const MAX_NODES: usize = 120;
+pub const MAX_STEPS: usize = 4;
 
 #[derive(Clone, Debug, PartialEq)]
 pub enum LabError { Invalid(String), Unsupported(String), NeedsData(String) }
@@ -31,16 +37,32 @@
 fn inv(m: impl Into<String>) -> LabError { LabError::Invalid(m.into()) }
 
 #[derive(Clone, Copy, Debug, PartialEq, Eq)]
-pub enum Op { CrossesAbove, CrossesBelow, Gt, Lt, Ge, Le }
+pub enum Op { CrossesAbove, CrossesBelow, Gt, Lt, Ge, Le, Eq, Ne, Crosses }
 #[derive(Clone, Debug, PartialEq)]
 pub struct SeriesRef { pub id: String, pub params: Params, pub tf: Tf, pub out: usize, pub offset: usize }
 #[derive(Clone, Debug, PartialEq)]
 pub enum Operand { Series(SeriesRef), Value(f64), BarsHeld }
 #[derive(Clone, Debug, PartialEq)]
 pub struct Cond { pub lhs: Operand, pub op: Op, pub rhs: Operand }
-/// Where an optimize path points: entry/exit list, condition index, side, and parameter key (None = `value`).
-#[derive(Clone, Debug, PartialEq)]
-pub struct Target { pub exit: bool, pub cond: usize, pub rhs: bool, pub param: Option<String>, pub slot: Option<Slot> }
+/// One entry of a rule list (spec §4.1). `Cmp` is the v1 condition; the other kinds are Phase 7.
+#[derive(Clone, Debug, PartialEq)]
+pub enum Node {
+    Cmp(Cond),
+    Range { x: Operand, lo: Operand, hi: Operand, outside: bool },
+    Trend { x: Operand, bars: usize, rising: bool },
+    All(Vec<Node>),
+    Any(Vec<Node>),
+    Not(Box<Node>),
+    AtLeast { k: usize, of: usize, when: Box<Node> },
+    Then { steps: Vec<Node>, within: usize },
+}
+impl Node {
+    /// The v1 comparison, when this node is one.
+    pub fn cmp(&self) -> Option<&Cond> { if let Node::Cmp(c) = self { Some(c) } else { None } }
+}
+/// Where an optimize path points: a price-order slot, or a number inside a rule list addressed by its spec path.
+#[derive(Clone, Debug, PartialEq)]
+pub struct Target { pub path: String, pub slot: Option<Slot> }
 /// Market orders fill at the next bar's open (default) or at the signal bar's own close (the book convention).
 #[derive(Clone, Copy, Debug, PartialEq, Eq)]
 pub enum Fill { NextOpen, Close }
@@ -69,7 +91,7 @@
 #[derive(Clone, Debug, PartialEq)]
 pub struct Dim { pub path: String, pub target: Target, pub values: Vec<f64> }
 #[derive(Clone, Debug, PartialEq)]
-pub struct Spec { pub entry: Vec<Cond>, pub exit: Vec<Cond>, pub cost_bps: Option<f64>, pub optimize: Vec<Dim>, pub exec: Exec, pub origin: Option<Origin> }
+pub struct Spec { pub entry: Vec<Node>, pub exit: Vec<Node>, pub cost_bps: Option<f64>, pub optimize: Vec<Dim>, pub exec: Exec, pub origin: Option<Origin> }
 
 pub fn parse_spec(text: &str) -> Result<Spec, LabError> {
     if text.len() > MAX_SPEC_BYTES { return Err(inv(format!("spec is {} bytes; the limit is {MAX_SPEC_BYTES}", text.len()))); }
@@ -93,8 +115,9 @@
         if let Some(v) = obj.get(at.split_once('.').unwrap().1) { *exec.slot_mut(slot) = Some(dist(v, at, hi)?); }
     }
     let price_exit = exec.stop_loss.is_some() || exec.target.is_some() || exec.trailing.is_some();
-    let entry = conds(entry_obj.get("all"), "entry.all", false, false)?;
-    let exit = conds(exit_obj.get("any"), "exit.any", true, price_exit)?;
+    let mut budget = Budget::default();
+    let entry = conds(entry_obj.get("all"), "entry.all", false, false, &mut budget)?;
+    let exit = conds(exit_obj.get("any"), "exit.any", true, price_exit, &mut budget)?;
     if entry.len() + exit.len() > MAX_CONDITIONS {
         return Err(inv(format!("{} conditions; the limit is {MAX_CONDITIONS}", entry.len() + exit.len())));
     }
@@ -163,31 +186,97 @@
     Ok(Origin { book, method, version })
 }
 
-fn conds(v: Option<&Json>, at: &str, exit: bool, may_be_empty: bool) -> Result<Vec<Cond>, LabError> {
+/// Counts comparisons and nodes while a spec is parsed (spec §4.3).
+#[derive(Default)]
+struct Budget { leaves: usize, nodes: usize }
+impl Budget {
+    fn take(&mut self, leaf: bool) -> Result<(), LabError> {
+        self.nodes += 1;
+        if leaf { self.leaves += 1; }
+        if self.leaves > MAX_LEAVES { return Err(inv(format!("more than {MAX_LEAVES} comparisons in the spec; the limit is {MAX_LEAVES}"))); }
+        if self.nodes > MAX_NODES { return Err(inv(format!("more than {MAX_NODES} comparisons, groups and expressions in the spec; the limit is {MAX_NODES}"))); }
+        Ok(())
+    }
+}
+
+fn conds(v: Option<&Json>, at: &str, exit: bool, may_be_empty: bool, b: &mut Budget) -> Result<Vec<Node>, LabError> {
     let Some(Json::Arr(items)) = v else { return Err(inv(format!("{at} must be a list of conditions"))) };
     if items.is_empty() && !may_be_empty { return Err(inv(format!("{at} needs at least one condition{}", if exit { " (or a stop_loss, target or trailing exit)" } else { "" }))); }
     if items.len() > MAX_CONDITIONS { return Err(inv(format!("{at} has {} conditions; the limit is {MAX_CONDITIONS}", items.len()))); }
-    items.iter().enumerate().map(|(i, c)| cond(c, &format!("{at}[{i}]"), exit)).collect()
-}
-
-fn cond(v: &Json, at: &str, exit: bool) -> Result<Cond, LabError> {
+    items.iter().enumerate().map(|(i, c)| node(c, &format!("{at}[{i}]"), exit, b)).collect()
+}
+
+/// One rule-list entry. `held` allows bars_held: top-level exit comparisons only, because every other node is a precomputed array.
+fn node(v: &Json, at: &str, held: bool, b: &mut Budget) -> Result<Node, LabError> {
+    if v.get("lhs").is_some() { return leaf(v, at, held, b); }
+    b.take(false)?;
+    let list = |key: &str, b: &mut Budget| -> Result<Vec<Node>, LabError> {
+        let Some(Json::Arr(items)) = v.get(key) else { return Err(inv(format!("{at}.{key} must be a list"))) };
+        if items.is_empty() { return Err(inv(format!("{at}.{key} needs at least one entry"))); }
+        items.iter().enumerate().map(|(i, c)| node(c, &format!("{at}.{key}[{i}]"), false, b)).collect()
+    };
+    if v.get("all").is_some() { fields(v, at, &["all"], &[])?; return Ok(Node::All(list("all", b)?)); }
+    if v.get("any").is_some() { fields(v, at, &["any"], &[])?; return Ok(Node::Any(list("any", b)?)); }
+    if let Some(n) = v.get("not") { fields(v, at, &["not"], &[])?; return Ok(Node::Not(Box::new(node(n, &format!("{at}.not"), false, b)?))); }
+    if v.get("at_least").is_some() {
+        fields(v, at, &["at_least", "of", "when"], &[])?;
+        let (k, of) = (whole(v.get("at_least"), &format!("{at}.at_least"), 1)?, whole(v.get("of"), &format!("{at}.of"), 1)?);
+        if k > of { return Err(inv(format!("{at}: at_least ({k}) cannot exceed of ({of})"))); }
+        let when = v.get("when").ok_or_else(|| inv(format!("{at}.when is required")))?;
+        return Ok(Node::AtLeast { k, of, when: Box::new(node(when, &format!("{at}.when"), false, b)?) });
+    }
+    if v.get("then").is_some() {
+        fields(v, at, &["then", "within"], &[])?;
+        let steps = list("then", b)?;
+        if !(2..=MAX_STEPS).contains(&steps.len()) { return Err(inv(format!("{at}.then needs 2 to {MAX_STEPS} steps"))); }
+        return Ok(Node::Then { steps, within: whole(v.get("within"), &format!("{at}.within"), 1)? });
+    }
+    Err(inv(format!("{at} must be a comparison (lhs, op, rhs) or a group ({})", NODE_KINDS.join(", "))))
+}
+
+/// A whole number from `min` to MAX_OFFSET (window lengths, counts, sequence spans).
+fn whole(v: Option<&Json>, at: &str, min: usize) -> Result<usize, LabError> {
+    v.and_then(Json::as_f64).filter(|n| n.fract() == 0. && *n >= min as f64 && *n <= MAX_OFFSET as f64).map(|n| n as usize)
+        .ok_or_else(|| inv(format!("{at} must be a whole number from {min} to {MAX_OFFSET}")))
+}
+
+fn leaf(v: &Json, at: &str, held: bool, b: &mut Budget) -> Result<Node, LabError> {
+    b.take(true)?;
+    let side = |k: &str, b: &mut Budget| operand(v.get(k), &format!("{at}.{k}"), false, b);
+    match v.get("op").and_then(Json::as_str) {
+        Some(op @ ("between" | "outside")) => {
+            fields(v, at, &["lhs", "op", "lo", "hi"], &[])?;
+            Ok(Node::Range { x: side("lhs", b)?, lo: side("lo", b)?, hi: side("hi", b)?, outside: op == "outside" })
+        }
+        Some(op @ ("rising" | "falling")) => {
+            fields(v, at, &["lhs", "op", "bars"], &[])?;
+            Ok(Node::Trend { x: side("lhs", b)?, bars: whole(v.get("bars"), &format!("{at}.bars"), 1)?, rising: op == "rising" })
+        }
+        _ => cond(v, at, held, b).map(Node::Cmp),
+    }
+}
+
+fn cond(v: &Json, at: &str, held: bool, b: &mut Budget) -> Result<Cond, LabError> {
     fields(v, at, &["lhs", "op", "rhs"], &[])?;
     let op = match v.get("op").and_then(Json::as_str) {
         Some("crosses_above") => Op::CrossesAbove,
         Some("crosses_below") => Op::CrossesBelow,
+        Some("crosses") => Op::Crosses,
         Some(">") => Op::Gt,
         Some("<") => Op::Lt,
         Some(">=") => Op::Ge,
         Some("<=") => Op::Le,
+        Some("=") => Op::Eq,
+        Some("!=") => Op::Ne,
         _ => return Err(inv(format!("{at}.op must be one of {}", OPERATORS.join(", ")))),
     };
-    let lhs = operand(v.get("lhs"), &format!("{at}.lhs"), exit)?;
-    let rhs = operand(v.get("rhs"), &format!("{at}.rhs"), exit)?;
+    let lhs = operand(v.get("lhs"), &format!("{at}.lhs"), held, b)?;
+    let rhs = operand(v.get("rhs"), &format!("{at}.rhs"), held, b)?;
     if matches!((&lhs, &rhs), (Operand::Value(_), Operand::Value(_))) { return Err(inv(format!("{at} compares two numbers; use at least one series"))); }
     Ok(Cond { lhs, op, rhs })
 }
 
-fn operand(v: Option<&Json>, at: &str, exit: bool) -> Result<Operand, LabError> {
+fn operand(v: Option<&Json>, at: &str, held: bool, _b: &mut Budget) -> Result<Operand, LabError> {
     let v = v.ok_or_else(|| inv(format!("{at} is required")))?;
     if v.get("value").is_some() {
         fields(v, at, &["value"], &[])?;
@@ -196,7 +285,7 @@
     fields(v, at, &["s", "p", "tf", "out", "offset"], &[])?;
     let id = v.get("s").and_then(Json::as_str).ok_or_else(|| inv(format!("{at}.s must name a series (or give a value)")))?;
     if id == "bars_held" {
-        if !exit { return Err(inv(format!("{at}: bars_held is only available in exit conditions"))); }
+        if !held { return Err(inv(format!("{at}: bars_held is only available in exit conditions (top level, not inside a group)"))); }
         if ["p", "tf", "out", "offset"].iter().any(|k| v.get(k).is_some()) { return Err(inv(format!("{at}: bars_held takes no p, tf, out or offset"))); }
         return Ok(Operand::BarsHeld);
     }
@@ -266,50 +355,84 @@
     if let Some((slot, at, _)) = SLOTS.iter().find(|(_, at, _)| path.starts_with(&format!("{at}."))) {
         let field = &path[at.len() + 1..];
         return match (spec.exec.slot(*slot), field) {
-            (Some(Dist::Pct(_)), "pct") | (Some(Dist::Atr { .. }), "atr") => Ok(Target { exit: false, cond: 0, rhs: false, param: None, slot: Some(*slot) }),
+            (Some(Dist::Pct(_)), "pct") | (Some(Dist::Atr { .. }), "atr") => Ok(Target { path: path.into(), slot: Some(*slot) }),
             (None, _) => Err(format!("the spec has no {at}")),
             _ => Err(format!("{at} is set by {}; optimize that field", if matches!(spec.exec.slot(*slot), Some(Dist::Pct(_))) { "pct" } else { "atr" })),
         };
     }
-    let (exit, rest) = match (path.strip_prefix("entry.all["), path.strip_prefix("exit.any[")) {
-        (Some(r), _) => (false, r),
-        (_, Some(r)) => (true, r),
-        _ => return Err("must start with entry.all[i], exit.any[i], entry.limit, exit.stop_loss, exit.target or exit.trailing".into()),
-    };
-    let (idx, rest) = rest.split_once("].").ok_or("expected ].lhs or ].rhs")?;
-    let cond: usize = idx.parse().map_err(|_| "condition index must be a whole number")?;
-    let c = (if exit { &spec.exit } else { &spec.entry }).get(cond).ok_or("no condition at that index")?;
-    let (rhs, field) = match (rest.strip_prefix("lhs."), rest.strip_prefix("rhs.")) {
-        (Some(f), _) => (false, f),
-        (_, Some(f)) => (true, f),
-        _ => return Err("expected lhs or rhs".into()),
-    };
-    match (if rhs { &c.rhs } else { &c.lhs }, field) {
-        (Operand::Value(_), "value") => Ok(Target { exit, cond, rhs, param: None, slot: None }),
-        (Operand::Series(s), f) if f.starts_with("p.") => {
-            let def = catalogue::get(&s.id).ok_or("price sources have no parameters")?;
-            let key = &f[2..];
-            if !def.params.iter().any(|p| p.key == key) { return Err(format!("{} has no parameter '{key}'", def.id)); }
-            Ok(Target { exit, cond, rhs, param: Some(key.into()), slot: None })
-        }
-        _ => Err("must end in .value (a number operand) or .p.<key> (an indicator parameter)".into()),
-    }
-}
-
-fn operand_mut<'a>(s: &'a mut Spec, t: &Target) -> &'a mut Operand {
-    let c = if t.exit { &mut s.exit[t.cond] } else { &mut s.entry[t.cond] };
-    if t.rhs { &mut c.rhs } else { &mut c.lhs }
-}
+    if !path.starts_with("entry.all[") && !path.starts_with("exit.any[") {
+        return Err("must start with entry.all[i], exit.any[i], entry.limit, exit.stop_loss, exit.target or exit.trailing".into());
+    }
+    let (mut found, mut near) = (false, None::<String>);
+    walk(&mut spec.clone(), &mut |p, k| match k {
+        Knob::Series(r) => if let Some(key) = path.strip_prefix(&format!("{p}.p.")) {
+            near = Some(catalogue::get(&r.id).map_or("price sources have no parameters".into(), |d| format!("{} has no parameter '{key}'", d.id)));
+        },
+        _ => found |= p == path,
+    });
+    if found { return Ok(Target { path: path.into(), slot: None }); }
+    if let Some(m) = near { return Err(m); }
+    let beyond = |list: &str, n: usize| path.strip_prefix(list).and_then(|r| r.split_once(']')).and_then(|(i, _)| i.parse::<usize>().ok()).is_some_and(|i| i >= n);
+    if beyond("entry.all[", spec.entry.len()) || beyond("exit.any[", spec.exit.len()) { return Err("no condition at that index".into()); }
+    Err("names no number in the spec (paths end in .value, .p.<key> or a whole-number field: bars, within, of, at_least)".into())
+}
+
+/// A number an optimize path can reach inside a rule list; `Series` is reported for error messages only.
+enum Knob<'a> { Value(&'a mut f64), Param(&'a mut SeriesRef, String), Whole(&'a mut usize), Series(&'a SeriesRef) }
+/// Visits every optimizable number in the rule lists with its spec path (`entry.all[0].lhs.p.length`, `exit.any[1].rhs.value`, `….bars`).
+fn walk(s: &mut Spec, f: &mut dyn FnMut(String, Knob)) {
+    for (k, n) in s.entry.iter_mut().enumerate() { walk_node(n, &format!("entry.all[{k}]"), f); }
+    for (k, n) in s.exit.iter_mut().enumerate() { walk_node(n, &format!("exit.any[{k}]"), f); }
+}
+fn walk_node(n: &mut Node, at: &str, f: &mut dyn FnMut(String, Knob)) {
+    match n {
+        Node::Cmp(c) => { walk_operand(&mut c.lhs, &format!("{at}.lhs"), f); walk_operand(&mut c.rhs, &format!("{at}.rhs"), f); }
+        Node::Range { x, lo, hi, .. } => { walk_operand(x, &format!("{at}.lhs"), f); walk_operand(lo, &format!("{at}.lo"), f); walk_operand(hi, &format!("{at}.hi"), f); }
+        Node::Trend { x, bars, .. } => { walk_operand(x, &format!("{at}.lhs"), f); f(format!("{at}.bars"), Knob::Whole(bars)); }
+        Node::All(v) => for (k, c) in v.iter_mut().enumerate() { walk_node(c, &format!("{at}.all[{k}]"), f) },
+        Node::Any(v) => for (k, c) in v.iter_mut().enumerate() { walk_node(c, &format!("{at}.any[{k}]"), f) },
+        Node::Not(c) => walk_node(c, &format!("{at}.not"), f),
+        Node::AtLeast { k, of, when } => { f(format!("{at}.at_least"), Knob::Whole(k)); f(format!("{at}.of"), Knob::Whole(of)); walk_node(when, &format!("{at}.when"), f); }
+        Node::Then { steps, within } => {
+            for (k, c) in steps.iter_mut().enumerate() { walk_node(c, &format!("{at}.then[{k}]"), f); }
+            f(format!("{at}.within"), Knob::Whole(within));
+        }
+    }
+}
+fn walk_operand(o: &mut Operand, at: &str, f: &mut dyn FnMut(String, Knob)) {
+    match o {
+        Operand::Value(x) => f(format!("{at}.value"), Knob::Value(x)),
+        Operand::Series(r) => {
+            f(at.to_string(), Knob::Series(r));
+            let keys: Vec<String> = r.params.keys().cloned().collect();
+            for k in keys { f(format!("{at}.p.{k}"), Knob::Param(&mut *r, k)); }
+        }
+        Operand::BarsHeld => {}
+    }
+}
+/// `at_least` never exceeds `of` (also after an optimize value is applied).
+fn consistent(n: &Node) -> Result<(), String> {
+    match n {
+        Node::AtLeast { k, of, .. } if k > of => Err(format!("at_least ({k}) cannot exceed of ({of})")),
+        Node::AtLeast { when, .. } | Node::Not(when) => consistent(when),
+        Node::All(v) | Node::Any(v) | Node::Then { steps: v, .. } => v.iter().try_for_each(consistent),
+        _ => Ok(()),
+    }
+}
+
 fn set(s: &mut Spec, t: &Target, x: f64) {
     if let Some(slot) = t.slot {
         match s.exec.slot_mut(slot) { Some(Dist::Pct(p)) => *p = x, Some(Dist::Atr { mult, .. }) => *mult = x, None => {} }
         return;
     }
-    match (operand_mut(s, t), &t.param) {
-        (Operand::Value(v), None) => *v = x,
-        (Operand::Series(r), Some(k)) => { r.params.insert(k.clone(), x); }
-        _ => {}
-    }
+    walk(s, &mut |p, k| if p == t.path {
+        match k {
+            Knob::Value(v) => *v = x,
+            Knob::Param(r, key) => { r.params.insert(key, x); }
+            Knob::Whole(n) => *n = x as usize,
+            Knob::Series(_) => {}
+        }
+    });
 }
 fn check_value(spec: &Spec, t: &Target, x: f64) -> Result<(), String> {
     if let Some(slot) = t.slot {
@@ -319,20 +442,30 @@
     }
     let mut s = spec.clone();
     set(&mut s, t, x);
-    match operand_mut(&mut s, t) {
-        Operand::Series(r) => catalogue::resolve_params(&catalogue::get(&r.id).ok_or("unknown series")?, &r.params).map(|_| ()),
-        _ => Ok(()),
-    }
+    let mut res = Ok(());
+    walk(&mut s, &mut |p, k| if p == t.path {
+        res = match k {
+            Knob::Param(r, _) => catalogue::get(&r.id).ok_or_else(|| "unknown series".to_string()).and_then(|d| catalogue::resolve_params(&d, &r.params).map(|_| ())),
+            Knob::Whole(_) if x.fract() != 0. || x < 1. || x > MAX_OFFSET as f64 => Err(format!("{} must be a whole number from 1 to {MAX_OFFSET}, not {x}", t.path)),
+            _ => Ok(()),
+        };
+    });
+    res?;
+    s.entry.iter().chain(&s.exit).try_for_each(consistent)
 }
 /// The spec's own value at an optimize target (the "base" setting the ranking prefers on ties).
 pub fn current(spec: &Spec, t: &Target) -> f64 {
     if let Some(slot) = t.slot { return match spec.exec.slot(slot) { Some(Dist::Pct(p)) => p, Some(Dist::Atr { mult, .. }) => mult, None => f64::NAN }; }
-    let mut s = spec.clone();
-    match (operand_mut(&mut s, t), &t.param) {
-        (Operand::Value(v), None) => *v,
-        (Operand::Series(r), Some(k)) => r.params.get(k).copied().unwrap_or(f64::NAN),
-        _ => f64::NAN,
-    }
+    let mut v = f64::NAN;
+    walk(&mut spec.clone(), &mut |p, k| if p == t.path {
+        match k {
+            Knob::Value(x) => v = *x,
+            Knob::Param(r, key) => v = r.params.get(&key).copied().unwrap_or(f64::NAN),
+            Knob::Whole(n) => v = *n as f64,
+            Knob::Series(_) => {}
+        }
+    });
+    v
 }
 /// The spec with one grid combination applied (values in `dims` order; validated when the spec was parsed).
 pub fn apply(spec: &Spec, dims: &[Dim], values: &[f64]) -> Spec {
diff --git a/Context/guru/backend/src/lab/series.rs b/Context/guru/backend/src/lab/series.rs
--- a/Context/guru/backend/src/lab/series.rs
+++ b/Context/guru/backend/src/lab/series.rs
@@ -3,7 +3,7 @@
 use super::bars::Bars;
 use super::catalogue;
 use super::resample::{self, Tf};
-use super::rules::{Cond, Dist, Exec, LabError, Op, Operand, SeriesRef, Spec};
+use super::rules::{Cond, Dist, Exec, LabError, Node, Op, Operand, SeriesRef, Spec};
 use std::collections::BTreeMap;
 use std::rc::Rc;
 
@@ -88,6 +88,9 @@
             Op::Le => a <= b,
             Op::CrossesAbove => { let (a1, b1) = prev(); a > b && a1 <= b1 }
             Op::CrossesBelow => { let (a1, b1) = prev(); a < b && a1 >= b1 }
+            Op::Eq => a == b,
+            Op::Ne => !a.is_nan() && !b.is_nan() && a != b,
+            Op::Crosses => { let (a1, b1) = prev(); (a > b && a1 <= b1) || (a < b && a1 >= b1) }
         }
     }
 }
@@ -104,8 +107,90 @@
 fn val(o: &Operand, fr: &Frames, cache: &mut Cache) -> Result<Val, LabError> {
     Ok(match o { Operand::Series(s) => Val::Arr(series(fr, cache, s)?), Operand::Value(x) => Val::Const(*x), Operand::BarsHeld => Val::Held })
 }
-fn list(cs: &[Cond], at: &str, fr: &Frames, cache: &mut Cache) -> Result<Vec<RCond>, LabError> {
-    cs.iter().enumerate().map(|(k, c)| Ok(RCond { lhs: val(&c.lhs, fr, cache)?, op: c.op, rhs: val(&c.rhs, fr, cache)?, label: format!("{at}[{k}]: {}", describe(c)) })).collect()
+/// A v1 comparison compiles exactly as before; every other node becomes a precomputed 1/0/NaN array tested with `> 0.5`.
+fn list(ns: &[Node], at: &str, fr: &Frames, cache: &mut Cache) -> Result<Vec<RCond>, LabError> {
+    ns.iter().enumerate().map(|(k, n)| Ok(match n {
+        Node::Cmp(c) => RCond { lhs: val(&c.lhs, fr, cache)?, op: c.op, rhs: val(&c.rhs, fr, cache)?, label: format!("{at}[{k}]: {}", describe(c)) },
+        _ => RCond { lhs: Val::Arr(mask(n, fr, cache)?), op: Op::Gt, rhs: Val::Const(0.5), label: format!("{at}[{k}]: {}", describe_node(n)) },
+    })).collect()
+}
+/// An operand as a daily array (constants repeated). bars_held never reaches here: the parser keeps it top-level.
+fn arr(o: &Operand, fr: &Frames, cache: &mut Cache) -> Result<Vec<f64>, LabError> {
+    match val(o, fr, cache)? {
+        Val::Arr(v) => Ok(v),
+        Val::Const(x) => Ok(vec![x; fr.d.len()]),
+        Val::Held => Err(LabError::Invalid("bars_held is only available in top-level exit conditions".into())),
+    }
+}
+fn truth(t: bool) -> f64 { if t { 1. } else { 0. } }
+/// One comparison as 1 (true), 0 (false) or NaN (unknown: an input, or the previous bar for crosses, is missing).
+fn cmp_at(op: Op, a: &[f64], b: &[f64], i: usize) -> f64 {
+    let (x, y) = (a[i], b[i]);
+    let (x1, y1) = if i == 0 { (f64::NAN, f64::NAN) } else { (a[i - 1], b[i - 1]) };
+    let crossing = matches!(op, Op::CrossesAbove | Op::CrossesBelow | Op::Crosses);
+    if x.is_nan() || y.is_nan() || (crossing && (x1.is_nan() || y1.is_nan())) { return f64::NAN; }
+    truth(match op {
+        Op::Gt => x > y,
+        Op::Lt => x < y,
+        Op::Ge => x >= y,
+        Op::Le => x <= y,
+        Op::Eq => x == y,
+        Op::Ne => x != y,
+        Op::CrossesAbove => x > y && x1 <= y1,
+        Op::CrossesBelow => x < y && x1 >= y1,
+        Op::Crosses => (x > y && x1 <= y1) || (x < y && x1 >= y1),
+    })
+}
+/// Three-valued truth of a node on every daily bar (spec §4.2): 1 true, 0 false, NaN unknown.
+pub fn mask(n: &Node, fr: &Frames, cache: &mut Cache) -> Result<Vec<f64>, LabError> {
+    let len = fr.d.len();
+    Ok(match n {
+        Node::Cmp(c) => { let (a, b) = (arr(&c.lhs, fr, cache)?, arr(&c.rhs, fr, cache)?); (0..len).map(|i| cmp_at(c.op, &a, &b, i)).collect() }
+        Node::Range { x, lo, hi, outside } => {
+            let (x, lo, hi) = (arr(x, fr, cache)?, arr(lo, fr, cache)?, arr(hi, fr, cache)?);
+            (0..len).map(|i| if x[i].is_nan() || lo[i].is_nan() || hi[i].is_nan() { f64::NAN } else { truth((lo[i] <= x[i] && x[i] <= hi[i]) != *outside) }).collect()
+        }
+        Node::Trend { x, bars, rising } => {
+            let x = arr(x, fr, cache)?;
+            (0..len).map(|i| {
+                if i < *bars || x[i - bars..=i].iter().any(|v| v.is_nan()) { return f64::NAN; }
+                truth(x[i - bars..=i].windows(2).all(|w| if *rising { w[1] > w[0] } else { w[1] < w[0] }))
+            }).collect()
+        }
+        Node::All(v) | Node::Any(v) => {
+            let all = matches!(n, Node::All(_));
+            let kids = v.iter().map(|c| mask(c, fr, cache)).collect::<Result<Vec<_>, _>>()?;
+            (0..len).map(|i| {
+                let (decisive, other) = if all { (0., 1.) } else { (1., 0.) };
+                if kids.iter().any(|k| k[i] == decisive) { decisive } else if kids.iter().any(|k| k[i].is_nan()) { f64::NAN } else { other }
+            }).collect()
+        }
+        Node::Not(c) => mask(c, fr, cache)?.into_iter().map(|v| if v.is_nan() { v } else { 1. - v }).collect(),
+        Node::AtLeast { k, of, when } => {
+            let m = mask(when, fr, cache)?;
+            let mut count = 0usize;
+            (0..len).map(|i| {
+                count += usize::from(m[i] == 1.);
+                if i >= *of { count -= usize::from(m[i - of] == 1.); }
+                if i + 1 < *of { f64::NAN } else { truth(count >= *k) }
+            }).collect()
+        }
+        Node::Then { steps, within } => {
+            let ms = steps.iter().map(|c| mask(c, fr, cache)).collect::<Result<Vec<_>, _>>()?;
+            // start[i]: the latest first-step bar of a chain whose current step is true on bar i (-1 = none).
+            let mut start: Vec<i64> = ms[0].iter().enumerate().map(|(i, v)| if *v == 1. { i as i64 } else { -1 }).collect();
+            for m in &ms[1..] {
+                let mut best = -1i64; // latest start among chains ending strictly before bar i
+                start = (0..len).map(|i| {
+                    let here = if m[i] == 1. { best } else { -1 };
+                    best = best.max(start[i]);
+                    here
+                }).collect();
+            }
+            let last = &ms[ms.len() - 1];
+            (0..len).map(|i| if last[i].is_nan() { f64::NAN } else { truth(start[i] >= 0 && i as i64 - start[i] <= *within as i64) }).collect()
+        }
+    })
 }
 pub fn compile(spec: &Spec, fr: &Frames, cache: &mut Cache) -> Result<Compiled, LabError> {
     let mut c = Compiled { entry: list(&spec.entry, "entry.all", fr, cache)?, exit: list(&spec.exit, "exit.any", fr, cache)?, exec: spec.exec.clone(), atr: BTreeMap::new() };
@@ -122,7 +207,10 @@
 
 /// Human-readable condition, recorded as a trade's exit reason, e.g. `rsi(length=2) crosses_above 70`.
 pub fn describe(c: &Cond) -> String {
-    let side = |o: &Operand| match o {
+    format!("{} {} {}", operand_text(&c.lhs), op_text(c.op), operand_text(&c.rhs))
+}
+pub fn operand_text(o: &Operand) -> String {
+    match o {
         Operand::Value(x) => format!("{x}"),
         Operand::BarsHeld => "bars_held".into(),
         Operand::Series(s) => {
@@ -133,9 +221,24 @@
             if s.offset > 0 { t = format!("{t}[{}]", s.offset); }
             t
         }
-    };
-    let op = match c.op { Op::CrossesAbove => "crosses_above", Op::CrossesBelow => "crosses_below", Op::Gt => ">", Op::Lt => "<", Op::Ge => ">=", Op::Le => "<=" };
-    format!("{} {op} {}", side(&c.lhs), side(&c.rhs))
+    }
+}
+fn op_text(op: Op) -> &'static str {
+    match op { Op::CrossesAbove => "crosses_above", Op::CrossesBelow => "crosses_below", Op::Gt => ">", Op::Lt => "<", Op::Ge => ">=", Op::Le => "<=", Op::Eq => "=", Op::Ne => "!=", Op::Crosses => "crosses" }
+}
+/// Human-readable node, recorded as an exit reason, e.g. `all(rsi(length=2) < 10, close > 5)`.
+pub fn describe_node(n: &Node) -> String {
+    let join = |v: &[Node]| v.iter().map(describe_node).collect::<Vec<_>>().join(", ");
+    match n {
+        Node::Cmp(c) => describe(c),
+        Node::Range { x, lo, hi, outside } => format!("{} {} {} and {}", operand_text(x), if *outside { "outside" } else { "between" }, operand_text(lo), operand_text(hi)),
+        Node::Trend { x, bars, rising } => format!("{} {} {bars} bars", operand_text(x), if *rising { "rising" } else { "falling" }),
+        Node::All(v) => format!("all({})", join(v)),
+        Node::Any(v) => format!("any({})", join(v)),
+        Node::Not(c) => format!("not({})", describe_node(c)),
+        Node::AtLeast { k, of, when } => format!("at_least {k} of {of}({})", describe_node(when)),
+        Node::Then { steps, within } => format!("then({}) within {within}", join(steps)),
+    }
 }
 
 #[cfg(test)]
PATCH
```

- [ ] **Step 4: Run all lab tests**

Run: `cd Context/guru/backend && CARGO_BUILD_JOBS=2 cargo test lab:: 2>&1 | grep 'test result'`
Expected: `ok. 284 passed; 0 failed` (276 at HEAD plus this phase's tests so far; a different count means a test is missing or a patch was not fully applied). `lab::pinned::existing_specs_keep_their_exact_trades` and `lab::templates::templates_reproduce_the_book_engine_on_fixtures` must be among the passing tests.

- [ ] **Step 5: Commit (only with the user's explicit permission for git writes)**

With permission: `git add Context/guru/backend/src/lab && git commit -m "feat(lab): rule nodes — groups, not, ranges, trends, at-least, sequences with three-valued logic"` (no trailer, no push). Without it: stop here and report `git status --short` and `git diff --stat`.

---
### Task 3: Expression operands — arithmetic, abs, percent difference, ref, windows, change; the Phase 8 `symbol` seam

Spec §4.1. Implements `Operand::Expr(Box<Expr>)` with `add/sub/mul/div/min/max/pct`, `abs`, `ref`, and `sum/avg/highest/lowest/change` over n daily bars. NaN in gives NaN out, and division by zero gives NaN. A series operand with `symbol` answers `unsupported` ("arrive in Phase 8").

**Files:**
- Modify: `Context/guru/backend/src/lab/rules.rs`
- Modify: `Context/guru/backend/src/lab/series.rs`

**Interfaces:**
- Consumes: Task 2 (`node`, `whole`, `Budget`, `walk_operand`, `arr`).
- Produces: `rules::{Expr, Bin, Win, EXPRESSIONS}` (`Bin::key`, `Win::key`); `series::eval` (private); `Operand::Expr` in `operand_text`.

- [ ] **Step 1: Write the failing tests** (apply from the repository root)

```bash
git apply --whitespace=nowarn <<'PATCH'
diff --git a/Context/guru/backend/src/lab/rules.rs b/Context/guru/backend/src/lab/rules.rs
--- a/Context/guru/backend/src/lab/rules.rs
+++ b/Context/guru/backend/src/lab/rules.rs
@@ -700,4 +751,30 @@
             assert!(e.message().contains(needle), "{path}: {needle:?} not in {:?}", e.message());
         }
     }
-}
+
+    // ---- Phase 7: expressions (spec §4.1)
+    #[test] fn expressions_parse_validate_and_reach_optimize() {
+        let s = parse_spec(&ONE_EXIT.replace("NODE", r#"{"lhs":{"sum":{"s":"rsi","p":{"length":2}},"bars":3},"op":"<","rhs":{"mul":[{"s":"atr"},{"value":2}]}}"#)).unwrap();
+        let c = s.entry[0].cmp().unwrap();
+        assert!(matches!(&c.lhs, Operand::Expr(e) if matches!(**e, Expr::Win(Win::Sum, Operand::Series(_), 3))));
+        assert!(matches!(&c.rhs, Operand::Expr(e) if matches!(**e, Expr::Bin(Bin::Mul, Operand::Series(_), Operand::Value(_)))));
+        for (lhs, needle, status) in [
+            (r#"{"add":[{"value":1}]}"#, "entry.all[0].lhs.add must be a list of two operands", "invalid"),
+            (r#"{"sum":{"s":"close"}}"#, "entry.all[0].lhs.bars must be a whole number from 1", "invalid"),
+            (r#"{"sum":{"s":"close"},"bars":3,"x":1}"#, "unknown field 'x' in entry.all[0].lhs", "invalid"),
+            (r#"{"ref":{"s":"close"},"bars":-1}"#, "entry.all[0].lhs.bars must be a whole number from 0", "invalid"),
+            (r#"{"abs":{"s":"bars_held"}}"#, "only available in exit", "invalid"),
+            (r#"{"s":"close","symbol":"NIFTY BANK"}"#, "arrive in Phase 8", "unsupported"),
+        ] {
+            let e = err(&ONE_EXIT.replace("NODE", &format!(r#"{{"lhs":{lhs},"op":">","rhs":{{"value":1}}}}"#)));
+            assert_eq!(e.status(), status, "{lhs}");
+            assert!(e.message().contains(needle), "{needle:?} not in {:?}", e.message());
+        }
+        let base = ONE_EXIT.replace("NODE", r#"{"lhs":{"sum":{"s":"rsi","p":{"length":2}},"bars":3},"op":"<","rhs":{"add":[{"s":"close"},{"value":5}]}}"#);
+        let text = format!(r#"{},"optimize":[{{"path":"entry.all[0].lhs.bars","from":2,"to":4,"step":1}},{{"path":"entry.all[0].lhs.sum.p.length","from":2,"to":3,"step":1}},{{"path":"entry.all[0].rhs.add[1].value","from":1,"to":9,"step":4}}]}}"#, &base[..base.len() - 1]);
+        let s = parse_spec(&text).unwrap();
+        assert_eq!(s.optimize.iter().map(|d| current(&s, &d.target)).collect::<Vec<_>>(), vec![3., 2., 5.]);
+        let t = apply(&s, &s.optimize, &[4., 3., 9.]);
+        assert_eq!(crate::lab::series::describe(t.entry[0].cmp().unwrap()), "sum(rsi(length=3), 4) < add(close, 9)");
+    }
+}
diff --git a/Context/guru/backend/src/lab/series.rs b/Context/guru/backend/src/lab/series.rs
--- a/Context/guru/backend/src/lab/series.rs
+++ b/Context/guru/backend/src/lab/series.rs
@@ -446,4 +491,38 @@
         assert_eq!(c.exit[0].label, "exit.any[0]: at_least 2 of 3(close falling 1 bars)");
         assert!(matches!(c.exit[1].lhs, Val::Held));
     }
-}
+
+    // ---- Phase 7: expressions (spec §4.1)
+    #[test] fn expressions_compute_on_daily_bars() {
+        let fr = Frames::new(crate::lab::testdata::closes(&[1., 2., 3., 2., 4.]));
+        let x = f64::NAN;
+        let e = |t: &str| { let Node::Cmp(c) = node(&format!(r#"{{"lhs":{t},"op":">","rhs":{{"value":0}}}}"#)) else { panic!() }; arr(&c.lhs, &fr, &mut Cache::new()).unwrap() };
+        assert_series(&e(r#"{"add":[{"s":"close"},{"value":1}]}"#), &[2., 3., 4., 3., 5.]);
+        assert_series(&e(r#"{"mul":[{"s":"close"},{"s":"close"}]}"#), &[1., 4., 9., 4., 16.]);
+        assert_series(&e(r#"{"div":[{"s":"close"},{"sub":[{"s":"close"},{"value":2}]}]}"#), &[-1., x, 3., x, 2.]); // division by zero is no value
+        assert_series(&e(r#"{"pct":[{"s":"close"},{"ref":{"s":"close"},"bars":1}]}"#), &[x, 100., 50., -100. / 3., 100.]);
+        assert_series(&e(r#"{"abs":{"sub":[{"s":"close"},{"value":3}]}}"#), &[2., 1., 0., 1., 1.]);
+        assert_series(&e(r#"{"min":[{"s":"close"},{"value":2.5}]}"#), &[1., 2., 2.5, 2., 2.5]);
+        assert_series(&e(r#"{"max":[{"s":"close"},{"value":2.5}]}"#), &[2.5, 2.5, 3., 2.5, 4.]);
+        assert_series(&e(r#"{"sum":{"s":"close"},"bars":3}"#), &[x, x, 6., 7., 9.]);
+        assert_series(&e(r#"{"avg":{"s":"close"},"bars":3}"#), &[x, x, 2., 7. / 3., 3.]);
+        assert_series(&e(r#"{"highest":{"s":"close"},"bars":2}"#), &[x, 2., 3., 3., 4.]);
+        assert_series(&e(r#"{"lowest":{"s":"close"},"bars":2}"#), &[x, 1., 2., 2., 2.]);
+        assert_series(&e(r#"{"change":{"s":"close"},"bars":2}"#), &[x, x, 2., 0., 1.]);
+        assert_series(&e(r#"{"sum":{"ref":{"s":"close"},"bars":1},"bars":2}"#), &[x, x, 3., 5., 5.]); // a NaN inside the window gives NaN
+    }
+    #[test] fn expressions_never_use_later_bars() {
+        let full = business_days(420, true);
+        for t in [r#"{"pct":[{"s":"close"},{"s":"sma","p":{"length":4},"tf":"W"}]}"#, r#"{"sum":{"s":"rsi","p":{"length":2}},"bars":3}"#,
+                  r#"{"highest":{"sub":[{"s":"high"},{"s":"low"}]},"bars":10}"#, r#"{"change":{"s":"close","tf":"M"},"bars":1}"#,
+                  r#"{"div":[{"abs":{"change":{"s":"close"},"bars":5}},{"ref":{"s":"atr"},"bars":1}]}"#] {
+            let Node::Cmp(c) = node(&format!(r#"{{"lhs":{t},"op":">","rhs":{{"value":0}}}}"#)) else { panic!() };
+            let want = arr(&c.lhs, &Frames::new(full.clone()), &mut Cache::new()).unwrap();
+            assert!(want.iter().filter(|v| v.is_finite()).count() > 300, "{t}: vacuous");
+            for cut in [97usize, 203, 311] {
+                let got = arr(&c.lhs, &Frames::new(crate::lab::testdata::cut(&full, cut)), &mut Cache::new()).unwrap();
+                for i in 0..cut - 1 { assert!(close_eq(got[i], want[i]), "{t} bar {i} changes when bars after {cut} exist"); }
+            }
+        }
+    }
+}
PATCH
```

- [ ] **Step 2: Run the tests and confirm the failure**

Run: `cd Context/guru/backend && CARGO_BUILD_JOBS=2 cargo test lab:: 2>&1 | tail -20`
Expected: Compile errors: `Expr`, `Bin` and `Win` are not defined.

- [ ] **Step 3: Implement** (apply from the repository root)

```bash
git apply --whitespace=nowarn <<'PATCH'
diff --git a/Context/guru/backend/src/lab/rules.rs b/Context/guru/backend/src/lab/rules.rs
--- a/Context/guru/backend/src/lab/rules.rs
+++ b/Context/guru/backend/src/lab/rules.rs
@@ -41,7 +41,22 @@
 #[derive(Clone, Debug, PartialEq)]
 pub struct SeriesRef { pub id: String, pub params: Params, pub tf: Tf, pub out: usize, pub offset: usize }
 #[derive(Clone, Debug, PartialEq)]
-pub enum Operand { Series(SeriesRef), Value(f64), BarsHeld }
+pub enum Operand { Series(SeriesRef), Value(f64), BarsHeld, Expr(Box<Expr>) }
+/// Expression operands (spec §4.1), evaluated on the daily bars.
+pub const EXPRESSIONS: &[&str] = &["add", "sub", "mul", "div", "min", "max", "pct", "abs", "ref", "sum", "avg", "highest", "lowest", "change"];
+#[derive(Clone, Copy, Debug, PartialEq, Eq)]
+pub enum Bin { Add, Sub, Mul, Div, Min, Max, Pct }
+#[derive(Clone, Copy, Debug, PartialEq, Eq)]
+pub enum Win { Sum, Avg, Highest, Lowest, Change }
+impl Bin { pub fn key(self) -> &'static str { EXPRESSIONS[self as usize] } }
+impl Win { pub fn key(self) -> &'static str { EXPRESSIONS[9 + self as usize] } }
+#[derive(Clone, Debug, PartialEq)]
+pub enum Expr {
+    Bin(Bin, Operand, Operand),
+    Abs(Operand),
+    Ref(Operand, usize),
+    Win(Win, Operand, usize),
+}
 #[derive(Clone, Debug, PartialEq)]
 pub struct Cond { pub lhs: Operand, pub op: Op, pub rhs: Operand }
 /// One entry of a rule list (spec §4.1). `Cmp` is the v1 condition; the other kinds are Phase 7.
@@ -276,11 +291,15 @@
     Ok(Cond { lhs, op, rhs })
 }
 
-fn operand(v: Option<&Json>, at: &str, held: bool, _b: &mut Budget) -> Result<Operand, LabError> {
+fn operand(v: Option<&Json>, at: &str, held: bool, b: &mut Budget) -> Result<Operand, LabError> {
     let v = v.ok_or_else(|| inv(format!("{at} is required")))?;
     if v.get("value").is_some() {
         fields(v, at, &["value"], &[])?;
         return v.get("value").and_then(Json::as_f64).map(Operand::Value).ok_or_else(|| inv(format!("{at}.value must be a number")));
+    }
+    if let Some(e) = expr(v, at, b)? { return Ok(Operand::Expr(Box::new(e))); }
+    if v.get("symbol").is_some() {
+        return Err(LabError::Unsupported(format!("{at}.symbol: operands from another symbol (AmiBroker Foreign, TradingView request.security on another ticker) arrive in Phase 8")));
     }
     fields(v, at, &["s", "p", "tf", "out", "offset"], &[])?;
     let id = v.get("s").and_then(Json::as_str).ok_or_else(|| inv(format!("{at}.s must name a series (or give a value)")))?;
@@ -302,6 +321,32 @@
     }
     let out = match v.get("out") { None => None, Some(o) => Some(o.as_str().ok_or_else(|| inv(format!("{at}.out must be a string")))?) };
     series_ref(at, id, &given, tf, out, offset).map(Operand::Series)
+}
+
+/// An expression operand (spec §4.1), or None when `v` is a series operand.
+fn expr(v: &Json, at: &str, b: &mut Budget) -> Result<Option<Expr>, LabError> {
+    let Json::Obj(kv) = v else { return Ok(None) };
+    let Some(key) = kv.iter().map(|(k, _)| k.as_str()).find(|k| EXPRESSIONS.contains(k)) else { return Ok(None) };
+    b.take(false)?;
+    let sub = |k: &str, b: &mut Budget| operand(v.get(k), &format!("{at}.{k}"), false, b);
+    Ok(Some(match key {
+        "add" | "sub" | "mul" | "div" | "min" | "max" | "pct" => {
+            fields(v, at, &[key], &[])?;
+            let pair = || inv(format!("{at}.{key} must be a list of two operands"));
+            let Some(Json::Arr(xs)) = v.get(key) else { return Err(pair()) };
+            if xs.len() != 2 { return Err(pair()); }
+            let (l, r) = (operand(Some(&xs[0]), &format!("{at}.{key}[0]"), false, b)?, operand(Some(&xs[1]), &format!("{at}.{key}[1]"), false, b)?);
+            let op = [Bin::Add, Bin::Sub, Bin::Mul, Bin::Div, Bin::Min, Bin::Max, Bin::Pct].into_iter().find(|o| o.key() == key).unwrap();
+            Expr::Bin(op, l, r)
+        }
+        "abs" => { fields(v, at, &["abs"], &[])?; Expr::Abs(sub("abs", b)?) }
+        "ref" => { fields(v, at, &["ref", "bars"], &[])?; Expr::Ref(sub("ref", b)?, whole(v.get("bars"), &format!("{at}.bars"), 0)?) }
+        _ => {
+            fields(v, at, &[key, "bars"], &[])?;
+            let w = [Win::Sum, Win::Avg, Win::Highest, Win::Lowest, Win::Change].into_iter().find(|w| w.key() == key).unwrap();
+            Expr::Win(w, sub(key, b)?, whole(v.get("bars"), &format!("{at}.bars"), 1)?)
+        }
+    }))
 }
 
 /// Validates one series operand against the catalogue (shared by rule specs and /api/lab/series).
@@ -408,6 +453,12 @@
             for k in keys { f(format!("{at}.p.{k}"), Knob::Param(&mut *r, k)); }
         }
         Operand::BarsHeld => {}
+        Operand::Expr(e) => match &mut **e {
+            Expr::Bin(op, l, r) => { walk_operand(l, &format!("{at}.{}[0]", op.key()), f); walk_operand(r, &format!("{at}.{}[1]", op.key()), f); }
+            Expr::Abs(x) => walk_operand(x, &format!("{at}.abs"), f),
+            Expr::Ref(x, n) => { walk_operand(x, &format!("{at}.ref"), f); f(format!("{at}.bars"), Knob::Whole(n)); }
+            Expr::Win(w, x, n) => { walk_operand(x, &format!("{at}.{}", w.key()), f); f(format!("{at}.bars"), Knob::Whole(n)); }
+        },
     }
 }
 /// `at_least` never exceeds `of` (also after an optimize value is applied).
diff --git a/Context/guru/backend/src/lab/series.rs b/Context/guru/backend/src/lab/series.rs
--- a/Context/guru/backend/src/lab/series.rs
+++ b/Context/guru/backend/src/lab/series.rs
@@ -3,7 +3,7 @@
 use super::bars::Bars;
 use super::catalogue;
 use super::resample::{self, Tf};
-use super::rules::{Cond, Dist, Exec, LabError, Node, Op, Operand, SeriesRef, Spec};
+use super::rules::{Bin, Cond, Dist, Exec, Expr, LabError, Node, Op, Operand, SeriesRef, Spec, Win};
 use std::collections::BTreeMap;
 use std::rc::Rc;
 
@@ -105,7 +105,7 @@
 }
 
 fn val(o: &Operand, fr: &Frames, cache: &mut Cache) -> Result<Val, LabError> {
-    Ok(match o { Operand::Series(s) => Val::Arr(series(fr, cache, s)?), Operand::Value(x) => Val::Const(*x), Operand::BarsHeld => Val::Held })
+    Ok(match o { Operand::Series(s) => Val::Arr(series(fr, cache, s)?), Operand::Value(x) => Val::Const(*x), Operand::BarsHeld => Val::Held, Operand::Expr(e) => Val::Arr(eval(e, fr, cache)?) })
 }
 /// A v1 comparison compiles exactly as before; every other node becomes a precomputed 1/0/NaN array tested with `> 0.5`.
 fn list(ns: &[Node], at: &str, fr: &Frames, cache: &mut Cache) -> Result<Vec<RCond>, LabError> {
@@ -121,6 +121,45 @@
         Val::Const(x) => Ok(vec![x; fr.d.len()]),
         Val::Held => Err(LabError::Invalid("bars_held is only available in top-level exit conditions".into())),
     }
+}
+/// An expression on the daily bars (spec §4.1–§4.2): a NaN input gives NaN, division by zero gives NaN, windows are NaN until full.
+fn eval(e: &Expr, fr: &Frames, cache: &mut Cache) -> Result<Vec<f64>, LabError> {
+    let len = fr.d.len();
+    let nan = f64::NAN;
+    Ok(match e {
+        Expr::Bin(op, l, r) => {
+            let (a, b) = (arr(l, fr, cache)?, arr(r, fr, cache)?);
+            a.iter().zip(&b).map(|(&x, &y)| if x.is_nan() || y.is_nan() { nan } else {
+                match op {
+                    Bin::Add => x + y,
+                    Bin::Sub => x - y,
+                    Bin::Mul => x * y,
+                    Bin::Div => if y == 0. { nan } else { x / y },
+                    Bin::Min => x.min(y),
+                    Bin::Max => x.max(y),
+                    Bin::Pct => if y == 0. { nan } else { 100. * (x - y) / y },
+                }
+            }).collect()
+        }
+        Expr::Abs(x) => arr(x, fr, cache)?.into_iter().map(f64::abs).collect(),
+        Expr::Ref(x, n) => { let a = arr(x, fr, cache)?; (0..len).map(|i| if i >= *n { a[i - n] } else { nan }).collect() }
+        Expr::Win(Win::Change, x, n) => { let a = arr(x, fr, cache)?; (0..len).map(|i| if i >= *n { a[i] - a[i - n] } else { nan }).collect() }
+        Expr::Win(w, x, n) => {
+            let a = arr(x, fr, cache)?;
+            // shortcut: O(bars × window); a running sum / monotone deque is the upgrade if optimizer profiles show it.
+            (0..len).map(|i| {
+                if i + 1 < *n { return nan; }
+                let s = &a[i + 1 - n..=i];
+                if s.iter().any(|v| v.is_nan()) { return nan; }
+                match w {
+                    Win::Sum => s.iter().sum(),
+                    Win::Avg => s.iter().sum::<f64>() / *n as f64,
+                    Win::Highest => s.iter().copied().fold(f64::MIN, f64::max),
+                    _ => s.iter().copied().fold(f64::MAX, f64::min),
+                }
+            }).collect()
+        }
+    })
 }
 fn truth(t: bool) -> f64 { if t { 1. } else { 0. } }
 /// One comparison as 1 (true), 0 (false) or NaN (unknown: an input, or the previous bar for crosses, is missing).
@@ -213,6 +252,12 @@
     match o {
         Operand::Value(x) => format!("{x}"),
         Operand::BarsHeld => "bars_held".into(),
+        Operand::Expr(e) => match &**e {
+            Expr::Bin(op, l, r) => format!("{}({}, {})", op.key(), operand_text(l), operand_text(r)),
+            Expr::Abs(x) => format!("abs({})", operand_text(x)),
+            Expr::Ref(x, n) => format!("ref({}, {n})", operand_text(x)),
+            Expr::Win(w, x, n) => format!("{}({}, {n})", w.key(), operand_text(x)),
+        },
         Operand::Series(s) => {
             let p: Vec<String> = s.params.iter().map(|(k, v)| format!("{k}={v}")).collect();
             let mut t = if p.is_empty() { s.id.clone() } else { format!("{}({})", s.id, p.join(",")) };
PATCH
```

- [ ] **Step 4: Run all lab tests**

Run: `cd Context/guru/backend && CARGO_BUILD_JOBS=2 cargo test lab:: 2>&1 | grep 'test result'`
Expected: `ok. 287 passed; 0 failed` (276 at HEAD plus this phase's tests so far; a different count means a test is missing or a patch was not fully applied). `lab::pinned::existing_specs_keep_their_exact_trades` and `lab::templates::templates_reproduce_the_book_engine_on_fixtures` must be among the passing tests.

- [ ] **Step 5: Commit (only with the user's explicit permission for git writes)**

With permission: `git add Context/guru/backend/src/lab && git commit -m "feat(lab): expression operands and the reserved symbol seam for Phase 8"` (no trailer, no push). Without it: stop here and report `git status --short` and `git diff --stat`.

---
### Task 4: State operands — bars_since, value_when, highest/lowest_since, count

Spec §4.1. Implements the AmiBroker/Pine state functions as expressions over a node: `bars_since` (0 on the true bar), `value_when` (nth 1..20), `highest_since` and `lowest_since` (from the latest true bar, inclusive, skipping NaN), and `count` over n bars.

> **Rule (fix round 1, spec §4.2):** `count`, `bars_since`, `value_when` and `highest/lowest_since` must follow the same Kleene rule as `at_least` and `then`: when an unknown bar in the lookback could have changed the answer, the result is NaN (unknown), never 0 or "not found". A `not` over such an operand must stay unknown through warm-up. Add the matching warm-up test (sma 200 over 300 bars) for each.

**Files:**
- Modify: `Context/guru/backend/src/lab/rules.rs`
- Modify: `Context/guru/backend/src/lab/series.rs`

**Interfaces:**
- Consumes: Tasks 2–3.
- Produces: `Expr::{BarsSince, ValueWhen, Since, Count}`, `rules::MAX_NTH`.

- [ ] **Step 1: Write the failing tests** (apply from the repository root)

```bash
git apply --whitespace=nowarn <<'PATCH'
diff --git a/Context/guru/backend/src/lab/rules.rs b/Context/guru/backend/src/lab/rules.rs
--- a/Context/guru/backend/src/lab/rules.rs
+++ b/Context/guru/backend/src/lab/rules.rs
@@ -777,4 +803,20 @@
         let t = apply(&s, &s.optimize, &[4., 3., 9.]);
         assert_eq!(crate::lab::series::describe(t.entry[0].cmp().unwrap()), "sum(rsi(length=3), 4) < add(close, 9)");
     }
-}
+
+    #[test] fn state_operands_parse_and_reach_optimize() {
+        let base = ONE_EXIT.replace("NODE", r#"{"lhs":{"value_when":{"lhs":{"s":"rsi"},"op":"<","rhs":{"value":30}},"of":{"s":"low"},"nth":2},"op":"<","rhs":{"s":"close"}}"#);
+        let text = format!(r#"{},"optimize":[{{"path":"entry.all[0].lhs.value_when.rhs.value","from":20,"to":30,"step":10}},{{"path":"entry.all[0].lhs.nth","from":1,"to":3,"step":1}}]}}"#, &base[..base.len() - 1]);
+        let s = parse_spec(&text).unwrap();
+        assert_eq!(s.optimize.iter().map(|d| current(&s, &d.target)).collect::<Vec<_>>(), vec![30., 2.]);
+        for (lhs, needle) in [
+            (r#"{"value_when":{"lhs":{"s":"rsi"},"op":"<","rhs":{"value":30}},"of":{"s":"low"},"nth":21}"#, "entry.all[0].lhs.nth must be a whole number from 1 to 20"),
+            (r#"{"count":{"lhs":{"s":"rsi"},"op":"<","rhs":{"value":30}}}"#, "entry.all[0].lhs.bars must be a whole number from 1"),
+            (r#"{"bars_since":{"value":1}}"#, "entry.all[0].lhs.bars_since must be a comparison"),
+            (r#"{"highest_since":{"lhs":{"s":"close"},"op":">","rhs":{"value":1}}}"#, "entry.all[0].lhs.of is required"),
+        ] {
+            let e = err(&ONE_EXIT.replace("NODE", &format!(r#"{{"lhs":{lhs},"op":">","rhs":{{"value":1}}}}"#)));
+            assert!(e.message().contains(needle), "{needle:?} not in {:?}", e.message());
+        }
+    }
+}
diff --git a/Context/guru/backend/src/lab/series.rs b/Context/guru/backend/src/lab/series.rs
--- a/Context/guru/backend/src/lab/series.rs
+++ b/Context/guru/backend/src/lab/series.rs
@@ -525,4 +557,40 @@
             }
         }
     }
-}
+
+    // ---- Phase 7: state operands (spec §4.1)
+    #[test] fn state_operands_follow_amibroker_definitions() {
+        let fr = Frames::new(crate::lab::testdata::closes(&[1., 3., 2., 5., 4., 6., 1.]));
+        let x = f64::NAN;
+        let up = r#"{"lhs":{"s":"close"},"op":">","rhs":{"s":"close","offset":1}}"#; // unknown, 1, 0, 1, 0, 1, 0
+        let e = |t: &str| { let Node::Cmp(c) = node(&format!(r#"{{"lhs":{t},"op":">","rhs":{{"value":0}}}}"#)) else { panic!() }; arr(&c.lhs, &fr, &mut Cache::new()).unwrap() };
+        assert_series(&e(&format!(r#"{{"bars_since":{up}}}"#)), &[x, 0., 1., 0., 1., 0., 1.]);
+        assert_series(&e(&format!(r#"{{"value_when":{up},"of":{{"s":"close"}}}}"#)), &[x, 3., 3., 5., 5., 6., 6.]);
+        assert_series(&e(&format!(r#"{{"value_when":{up},"of":{{"s":"close"}},"nth":2}}"#)), &[x, x, x, 3., 3., 5., 5.]);
+        assert_series(&e(&format!(r#"{{"highest_since":{up},"of":{{"s":"close"}}}}"#)), &[x, 3., 3., 5., 5., 6., 6.]);
+        assert_series(&e(&format!(r#"{{"lowest_since":{up},"of":{{"s":"close"}}}}"#)), &[x, 3., 2., 5., 4., 6., 1.]);
+        assert_series(&e(&format!(r#"{{"count":{up},"bars":3}}"#)), &[x, x, 1., 2., 1., 2., 1.]);
+    }
+    #[test] fn state_operands_never_use_later_bars() {
+        let full = business_days(420, true);
+        let rsi_low = r#"{"lhs":{"s":"rsi","p":{"length":2}},"op":"<","rhs":{"value":20}}"#;
+        for t in [format!(r#"{{"bars_since":{rsi_low}}}"#), format!(r#"{{"value_when":{rsi_low},"of":{{"s":"low"}},"nth":2}}"#),
+                  format!(r#"{{"highest_since":{rsi_low},"of":{{"s":"high"}}}}"#), format!(r#"{{"lowest_since":{{"lhs":{{"s":"close"}},"op":">","rhs":{{"s":"sma","p":{{"length":4}},"tf":"W"}}}},"of":{{"s":"close"}}}}"#),
+                  format!(r#"{{"count":{rsi_low},"bars":10}}"#)] {
+            let Node::Cmp(c) = node(&format!(r#"{{"lhs":{t},"op":">","rhs":{{"value":0}}}}"#)) else { panic!() };
+            let want = arr(&c.lhs, &Frames::new(full.clone()), &mut Cache::new()).unwrap();
+            assert!(want.iter().filter(|v| v.is_finite()).count() > 300, "{t}: vacuous");
+            for cut in [97usize, 203, 311] {
+                let got = arr(&c.lhs, &Frames::new(crate::lab::testdata::cut(&full, cut)), &mut Cache::new()).unwrap();
+                for i in 0..cut - 1 { assert!(close_eq(got[i], want[i]), "{t} bar {i} changes when bars after {cut} exist"); }
+            }
+        }
+    }
+    #[test] fn the_amibroker_bars_since_time_exit_is_expressible() {
+        // hbqts_entry_id1's book exit counts bars since the latest (repeating) buy signal; the lab can now say so.
+        let spec = parse_spec(r#"{"v":1,"entry":{"all":[{"lhs":{"s":"sma","p":{"length":3}},"op":">","rhs":{"s":"sma","p":{"length":16}}}]},
+            "exit":{"any":[{"lhs":{"bars_since":{"lhs":{"s":"sma","p":{"length":3}},"op":">","rhs":{"s":"sma","p":{"length":16}}}},"op":">=","rhs":{"value":6}}]},"execution":{"fill":"close"}}"#).unwrap();
+        let c = compile(&spec, &Frames::new(business_days(200, false)), &mut Cache::new()).unwrap();
+        assert_eq!(c.exit[0].label, "exit.any[0]: bars_since(sma(length=3) > sma(length=16)) >= 6");
+    }
+}
PATCH
```

- [ ] **Step 2: Run the tests and confirm the failure**

Run: `cd Context/guru/backend && CARGO_BUILD_JOBS=2 cargo test lab:: 2>&1 | tail -20`
Expected: Compile errors: the `Expr` variants are missing.

- [ ] **Step 3: Implement** (apply from the repository root)

```bash
git apply --whitespace=nowarn <<'PATCH'
diff --git a/Context/guru/backend/src/lab/rules.rs b/Context/guru/backend/src/lab/rules.rs
--- a/Context/guru/backend/src/lab/rules.rs
+++ b/Context/guru/backend/src/lab/rules.rs
@@ -43,7 +43,9 @@
 #[derive(Clone, Debug, PartialEq)]
 pub enum Operand { Series(SeriesRef), Value(f64), BarsHeld, Expr(Box<Expr>) }
 /// Expression operands (spec §4.1), evaluated on the daily bars.
-pub const EXPRESSIONS: &[&str] = &["add", "sub", "mul", "div", "min", "max", "pct", "abs", "ref", "sum", "avg", "highest", "lowest", "change"];
+pub const EXPRESSIONS: &[&str] = &["add", "sub", "mul", "div", "min", "max", "pct", "abs", "ref", "sum", "avg", "highest", "lowest", "change",
+    "bars_since", "value_when", "highest_since", "lowest_since", "count"];
+pub const MAX_NTH: usize = 20;
 #[derive(Clone, Copy, Debug, PartialEq, Eq)]
 pub enum Bin { Add, Sub, Mul, Div, Min, Max, Pct }
 #[derive(Clone, Copy, Debug, PartialEq, Eq)]
@@ -56,6 +58,10 @@
     Abs(Operand),
     Ref(Operand, usize),
     Win(Win, Operand, usize),
+    BarsSince(Node),
+    ValueWhen { when: Node, of: Operand, nth: usize },
+    Since { when: Node, of: Operand, highest: bool },
+    Count(Node, usize),
 }
 #[derive(Clone, Debug, PartialEq)]
 pub struct Cond { pub lhs: Operand, pub op: Op, pub rhs: Operand }
@@ -341,12 +347,28 @@
         }
         "abs" => { fields(v, at, &["abs"], &[])?; Expr::Abs(sub("abs", b)?) }
         "ref" => { fields(v, at, &["ref", "bars"], &[])?; Expr::Ref(sub("ref", b)?, whole(v.get("bars"), &format!("{at}.bars"), 0)?) }
+        "bars_since" => { fields(v, at, &[key], &[])?; Expr::BarsSince(cond_of(v, key, at, b)?) }
+        "count" => { fields(v, at, &[key, "bars"], &[])?; Expr::Count(cond_of(v, key, at, b)?, whole(v.get("bars"), &format!("{at}.bars"), 1)?) }
+        "value_when" => {
+            fields(v, at, &[key, "of", "nth"], &[])?;
+            let nth = match v.get("nth") {
+                None => 1,
+                Some(n) => n.as_f64().filter(|n| n.fract() == 0. && (1. ..=MAX_NTH as f64).contains(n)).ok_or_else(|| inv(format!("{at}.nth must be a whole number from 1 to {MAX_NTH}")))? as usize,
+            };
+            Expr::ValueWhen { when: cond_of(v, key, at, b)?, of: sub("of", b)?, nth }
+        }
+        "highest_since" | "lowest_since" => { fields(v, at, &[key, "of"], &[])?; Expr::Since { when: cond_of(v, key, at, b)?, of: sub("of", b)?, highest: key == "highest_since" } }
         _ => {
             fields(v, at, &[key, "bars"], &[])?;
             let w = [Win::Sum, Win::Avg, Win::Highest, Win::Lowest, Win::Change].into_iter().find(|w| w.key() == key).unwrap();
             Expr::Win(w, sub(key, b)?, whole(v.get("bars"), &format!("{at}.bars"), 1)?)
         }
     }))
+}
+
+/// The condition a state operand watches (`{"bars_since": node}` …).
+fn cond_of(v: &Json, key: &str, at: &str, b: &mut Budget) -> Result<Node, LabError> {
+    node(v.get(key).ok_or_else(|| inv(format!("{at}.{key} is required")))?, &format!("{at}.{key}"), false, b)
 }
 
 /// Validates one series operand against the catalogue (shared by rule specs and /api/lab/series).
@@ -458,6 +480,10 @@
             Expr::Abs(x) => walk_operand(x, &format!("{at}.abs"), f),
             Expr::Ref(x, n) => { walk_operand(x, &format!("{at}.ref"), f); f(format!("{at}.bars"), Knob::Whole(n)); }
             Expr::Win(w, x, n) => { walk_operand(x, &format!("{at}.{}", w.key()), f); f(format!("{at}.bars"), Knob::Whole(n)); }
+            Expr::BarsSince(c) => walk_node(c, &format!("{at}.bars_since"), f),
+            Expr::Count(c, n) => { walk_node(c, &format!("{at}.count"), f); f(format!("{at}.bars"), Knob::Whole(n)); }
+            Expr::ValueWhen { when, of, nth } => { walk_node(when, &format!("{at}.value_when"), f); walk_operand(of, &format!("{at}.of"), f); f(format!("{at}.nth"), Knob::Whole(nth)); }
+            Expr::Since { when, of, highest } => { walk_node(when, &format!("{at}.{}", if *highest { "highest_since" } else { "lowest_since" }), f); walk_operand(of, &format!("{at}.of"), f); }
         },
     }
 }
diff --git a/Context/guru/backend/src/lab/series.rs b/Context/guru/backend/src/lab/series.rs
--- a/Context/guru/backend/src/lab/series.rs
+++ b/Context/guru/backend/src/lab/series.rs
@@ -144,6 +144,34 @@
         Expr::Abs(x) => arr(x, fr, cache)?.into_iter().map(f64::abs).collect(),
         Expr::Ref(x, n) => { let a = arr(x, fr, cache)?; (0..len).map(|i| if i >= *n { a[i - n] } else { nan }).collect() }
         Expr::Win(Win::Change, x, n) => { let a = arr(x, fr, cache)?; (0..len).map(|i| if i >= *n { a[i] - a[i - n] } else { nan }).collect() }
+        Expr::BarsSince(c) => {
+            let mut last = None;
+            mask(c, fr, cache)?.iter().enumerate().map(|(i, m)| { if *m == 1. { last = Some(i); } last.map_or(nan, |l| (i - l) as f64) }).collect()
+        }
+        Expr::Count(c, n) => {
+            let m = mask(c, fr, cache)?;
+            let mut count = 0usize;
+            (0..len).map(|i| {
+                count += usize::from(m[i] == 1.);
+                if i >= *n { count -= usize::from(m[i - n] == 1.); }
+                if i + 1 < *n { nan } else { count as f64 }
+            }).collect()
+        }
+        Expr::ValueWhen { when, of, nth } => {
+            let (m, a) = (mask(when, fr, cache)?, arr(of, fr, cache)?);
+            let mut hits: Vec<usize> = Vec::new();
+            (0..len).map(|i| { if m[i] == 1. { hits.push(i); } if hits.len() >= *nth { a[hits[hits.len() - nth]] } else { nan } }).collect()
+        }
+        Expr::Since { when, of, highest } => {
+            // AmiBroker HighestSince/LowestSince: from the latest bar where `when` held (inclusive); NaN values of `of` are skipped.
+            let (m, a) = (mask(when, fr, cache)?, arr(of, fr, cache)?);
+            let (mut started, mut ext) = (false, nan);
+            (0..len).map(|i| {
+                if m[i] == 1. { started = true; ext = a[i]; }
+                else if started && !a[i].is_nan() { ext = if ext.is_nan() { a[i] } else if *highest { ext.max(a[i]) } else { ext.min(a[i]) }; }
+                if started { ext } else { nan }
+            }).collect()
+        }
         Expr::Win(w, x, n) => {
             let a = arr(x, fr, cache)?;
             // shortcut: O(bars × window); a running sum / monotone deque is the upgrade if optimizer profiles show it.
@@ -257,6 +285,10 @@
             Expr::Abs(x) => format!("abs({})", operand_text(x)),
             Expr::Ref(x, n) => format!("ref({}, {n})", operand_text(x)),
             Expr::Win(w, x, n) => format!("{}({}, {n})", w.key(), operand_text(x)),
+            Expr::BarsSince(c) => format!("bars_since({})", describe_node(c)),
+            Expr::Count(c, n) => format!("count({}, {n})", describe_node(c)),
+            Expr::ValueWhen { when, of, nth } => format!("value_when({}, {}, {nth})", describe_node(when), operand_text(of)),
+            Expr::Since { when, of, highest } => format!("{}({}, {})", if *highest { "highest_since" } else { "lowest_since" }, describe_node(when), operand_text(of)),
         },
         Operand::Series(s) => {
             let p: Vec<String> = s.params.iter().map(|(k, v)| format!("{k}={v}")).collect();
PATCH
```

- [ ] **Step 4: Run all lab tests**

Run: `cd Context/guru/backend && CARGO_BUILD_JOBS=2 cargo test lab:: 2>&1 | grep 'test result'`
Expected: `ok. 291 passed; 0 failed` (276 at HEAD plus this phase's tests so far; a different count means a test is missing or a patch was not fully applied). `lab::pinned::existing_specs_keep_their_exact_trades` and `lab::templates::templates_reproduce_the_book_engine_on_fixtures` must be among the passing tests.

- [ ] **Step 5: Commit (only with the user's explicit permission for git writes)**

With permission: `git add Context/guru/backend/src/lab && git commit -m "feat(lab): state operands bars_since, value_when, highest/lowest_since, count"` (no trailer, no push). Without it: stop here and report `git status --short` and `git diff --stat`.

---
### Task 5: Higher timeframes with AmiBroker TimeFrame semantics — Q, Y, n-session blocks, expand last/first/point, compress

Spec §5.1, user request of 2026-10-15. This adds `Tf::Q`, `Tf::Y` and `Tf::Days(2..=63)`; `Tf::name`; `Expand` with `resample::expand_mode`; `Frames` with `q` and `y` (n-session blocks are built on use, through `Cow`); operand `expand`; and the `compress` expression. `expand: first` is accepted only for offset ≥ 1, the period open, or compress mode `first`; anything else is `unsupported` with the lookahead note. The generic causality and parameter-bound tests now cover Q, Y and 5D.

**Files:**
- Modify: `Context/guru/backend/src/lab/api.rs`
- Modify: `Context/guru/backend/src/lab/resample.rs`
- Modify: `Context/guru/backend/src/lab/rules.rs`
- Modify: `Context/guru/backend/src/lab/series.rs`

**Interfaces:**
- Consumes: Tasks 2–4.
- Produces: `resample::{Tf::{Q, Y, Days}, Tf::name, MAX_DAYS, TF_HELP, Expand, expand_mode}`; `rules::{Agg, AGGREGATES, Expr::{Expand, Compress}}`; `Frames { d, w, m, q, y }` with `Frames::bars(tf) -> Cow<Bars>`; catalogue JSON `timeframes` [D, W, M, Q, Y], `custom_days` and `expand_modes`.

> Note: The test patch edits one existing `api.rs` assertion: `tf=Q` is now valid. The invalid example becomes `H`, with the new message `tf must be D, W, M, Q, Y or nD (2D to 63D)`.

- [ ] **Step 1: Write the failing tests** (apply from the repository root)

```bash
git apply --whitespace=nowarn <<'PATCH'
diff --git a/Context/guru/backend/src/lab/series.rs b/Context/guru/backend/src/lab/series.rs
--- a/Context/guru/backend/src/lab/series.rs
+++ b/Context/guru/backend/src/lab/series.rs
@@ -380,7 +413,7 @@
         let full_bars = with_breadth(business_days(420, true));
         let full = Frames::new(full_bars.clone());
         for d in all().iter().filter(|d| d.compute.is_some()) {
-            for tf in [Tf::D, Tf::W, Tf::M] {
+            for tf in [Tf::D, Tf::W, Tf::M, Tf::Q, Tf::Y, Tf::Days(5)] {
                 let r = sref(d.id, tf, d.outputs.len() - 1, 0);
                 if d.needs.breadth && tf != Tf::D { assert_eq!(series(&full, &mut Cache::new(), &r).unwrap_err().status(), "needs_data"); continue; }
                 let want = series(&full, &mut Cache::new(), &r).unwrap();
@@ -425,7 +458,7 @@
                 let picks: [fn(&catalogue::ParamSpec) -> f64; 2] = [|p| p.min, |p| p.max];
                 for pick in picks {
                     let params: Params = d.params.iter().map(|p| (p.key.to_string(), pick(p))).collect();
-                    for tf in [Tf::D, Tf::W, Tf::M] {
+                    for tf in [Tf::D, Tf::W, Tf::M, Tf::Q, Tf::Y, Tf::Days(5)] {
                         let r = SeriesRef { id: d.id.into(), params: params.clone(), tf, out: 0, offset: 0 };
                         if d.needs.breadth && tf != Tf::D { assert_eq!(series(&fr, &mut Cache::new(), &r).unwrap_err().status(), "needs_data"); continue; }
                         assert_eq!(series(&fr, &mut Cache::new(), &r).unwrap().len(), fr.d.len(), "{} {tf:?}", d.id);
@@ -593,4 +626,61 @@
         let c = compile(&spec, &Frames::new(business_days(200, false)), &mut Cache::new()).unwrap();
         assert_eq!(c.exit[0].label, "exit.any[0]: bars_since(sma(length=3) > sma(length=16)) >= 6");
     }
-}
+
+    // ---- Phase 7: AmiBroker TimeFrame semantics (spec §5.1)
+    fn operand_of(t: &str) -> Operand { let Node::Cmp(c) = node(&format!(r#"{{"lhs":{t},"op":">","rhs":{{"value":0}}}}"#)) else { panic!() }; c.lhs }
+    #[test] fn each_expand_mode_is_causal_and_placed_like_amibroker() {
+        let fr = Frames::new(business_days(30, false)); // 2021-01-04 is a Monday: weeks are daily 0-4, 5-9, …
+        let x = f64::NAN;
+        let v = |t: &str| arr(&operand_of(t), &fr, &mut Cache::new()).unwrap();
+        let (c, o) = (&fr.d.close, &fr.d.open);
+        // last (the default, = v1): week 1's close from its Friday (4) until week 2's Friday (9)
+        assert_series(&v(r#"{"s":"close","tf":"W"}"#)[..10], &[x, x, x, x, c[4], c[4], c[4], c[4], c[4], c[9]]);
+        // point: only on each week's last day
+        assert_series(&v(r#"{"s":"close","tf":"W","expand":"point"}"#)[..10], &[x, x, x, x, c[4], x, x, x, x, c[9]]);
+        // first + offset 1: the previous completed week from Monday (TradingView expr[1] with lookahead_on)
+        assert_series(&v(r#"{"s":"close","tf":"W","offset":1,"expand":"first"}"#)[..10], &[x, x, x, x, x, c[4], c[4], c[4], c[4], c[4]]);
+        // first on the open: the week's own open from Monday
+        assert_series(&v(r#"{"s":"open","tf":"W","expand":"first"}"#)[..7], &[o[0], o[0], o[0], o[0], o[0], o[5], o[5]]);
+        // compress: the week's highest close at its Friday (last), and the week's first close from Monday (first)
+        let hi = c[..5].iter().copied().fold(f64::MIN, f64::max);
+        assert_series(&v(r#"{"compress":{"s":"close"},"tf":"W","mode":"high"}"#)[..6], &[x, x, x, x, hi, hi]);
+        assert_series(&v(r#"{"compress":{"s":"close"},"tf":"W","mode":"first","expand":"first"}"#)[..6], &[c[0], c[0], c[0], c[0], c[0], c[5]]);
+    }
+    #[test] fn lookahead_expand_first_is_refused_with_a_note() {
+        for (t, status, needle) in [
+            (r#"{"s":"close","tf":"W","expand":"first"}"#, "unsupported", "expandFirst would show the period's final value"),
+            (r#"{"s":"high","tf":"M","expand":"first"}"#, "unsupported", "lookahead"),
+            (r#"{"compress":{"s":"close"},"tf":"W","mode":"high","expand":"first"}"#, "unsupported", "lookahead"),
+            (r#"{"s":"close","expand":"point"}"#, "invalid", "expand applies to W, M, Q, Y and nD"),
+            (r#"{"s":"close","tf":"W","expand":"later"}"#, "invalid", "expand must be last, first or point"),
+            (r#"{"compress":{"s":"close"},"tf":"D"}"#, "invalid", "tf must be one of W, M, Q, Y or nD"),
+            (r#"{"s":"close","tf":"1D"}"#, "invalid", "tf must be D, W, M, Q, Y or nD (2D to 63D)"),
+        ] {
+            let e = parse_spec(&format!(r#"{{"v":1,"entry":{{"all":[{{"lhs":{t},"op":">","rhs":{{"value":0}}}}]}},"exit":{{"any":[{{"lhs":{{"s":"bars_held"}},"op":">=","rhs":{{"value":1}}}}]}}}}"#)).unwrap_err();
+            assert_eq!(e.status(), status, "{t}");
+            assert!(e.message().contains(needle), "{needle:?} not in {:?}", e.message());
+        }
+    }
+    #[test] fn a_quarter_close_is_invisible_until_the_quarters_last_session() {
+        let fr = Frames::new(business_days(200, false)); // from 2021-01-04; Q1 ends 2021-03-31
+        let q = series(&fr, &mut Cache::new(), &sref("close", Tf::Q, 0, 0)).unwrap();
+        let last_q1 = fr.d.date.iter().rposition(|d| d.as_str() <= "2021-03-31").unwrap();
+        assert!(q[..last_q1].iter().all(|v| v.is_nan()));
+        assert!(q[last_q1..last_q1 + 20].iter().all(|v| *v == fr.d.close[last_q1]));
+    }
+    #[test] fn expand_modes_and_compress_never_use_later_bars() {
+        let full = business_days(420, true);
+        for t in [r#"{"s":"close","tf":"W","expand":"point"}"#, r#"{"s":"rsi","tf":"M","offset":1,"expand":"first"}"#, r#"{"s":"open","tf":"Q","expand":"first"}"#,
+                  r#"{"s":"sma","p":{"length":3},"tf":"5D"}"#, r#"{"compress":{"s":"rsi","p":{"length":2}},"tf":"W","mode":"low"}"#,
+                  r#"{"compress":{"s":"volume"},"tf":"M","mode":"sum","offset":1,"expand":"first"}"#, r#"{"s":"close","tf":"Y","offset":1,"expand":"first"}"#] {
+            let o = operand_of(t);
+            let want = arr(&o, &Frames::new(full.clone()), &mut Cache::new()).unwrap();
+            assert!(want.iter().any(|v| v.is_finite()), "{t}: vacuous");
+            for cut in [97usize, 203, 311] {
+                let got = arr(&o, &Frames::new(crate::lab::testdata::cut(&full, cut)), &mut Cache::new()).unwrap();
+                for i in 0..cut - 1 { assert!(close_eq(got[i], want[i]), "{t} bar {i} changes when bars after {cut} exist"); }
+            }
+        }
+    }
+}
diff --git a/Context/guru/backend/src/lab/resample.rs b/Context/guru/backend/src/lab/resample.rs
--- a/Context/guru/backend/src/lab/resample.rs
+++ b/Context/guru/backend/src/lab/resample.rs
@@ -110,6 +142,9 @@
         let r = resample(&d, Tf::D);
         assert_eq!(r.close, d.close); assert_eq!(r.end, vec![0, 1]);
         assert_eq!((Tf::parse("D"), Tf::parse("W"), Tf::parse("M"), Tf::parse("X")), (Some(Tf::D), Some(Tf::W), Some(Tf::M), None));
+        assert_eq!((Tf::parse("Q"), Tf::parse("Y"), Tf::parse("5D"), Tf::parse("63D")), (Some(Tf::Q), Some(Tf::Y), Some(Tf::Days(5)), Some(Tf::Days(63))));
+        for bad in ["1D", "64D", "+5D", "D5", "5d", ""] { assert_eq!(Tf::parse(bad), None, "{bad}"); }
+        assert_eq!((Tf::Days(5).name(), Tf::Q.name()), ("5D".to_string(), "Q".to_string()));
     }
     #[test] fn monthly_groups() {
         let d = daily(&["2024-01-30", "2024-01-31", "2024-02-01"], &[1., 2., 3.]);
@@ -127,4 +162,21 @@
         d.volume = Some(vec![10., 5.]);
         assert_eq!(resample(&d, Tf::W).volume, Some(vec![15.]));
     }
+
+    // ---- Phase 7: AmiBroker TimeFrame semantics (spec §5.1)
+    #[test] fn quarters_years_and_session_blocks_group_correctly() {
+        let d = daily(&["2023-12-29", "2024-01-02", "2024-03-28", "2024-04-01", "2024-12-31", "2025-01-02"], &[1., 2., 3., 4., 5., 6.]);
+        assert_eq!(resample(&d, Tf::Q).end, vec![0, 2, 3, 4, 5]);
+        assert_eq!(resample(&d, Tf::Y).end, vec![0, 4, 5]);
+        let b = resample(&d, Tf::Days(4)); // sessions 0-3 and 4-5, whatever their dates
+        assert_eq!((b.end.clone(), b.open.clone(), b.close.clone(), b.high.clone()), (vec![3, 5], vec![1., 5.], vec![4., 6.], vec![5., 7.]));
+    }
+    #[test] fn expand_modes_place_values_where_amibroker_does() {
+        let d = daily(&["2024-01-05", "2024-01-08", "2024-01-09", "2024-01-12", "2024-01-15"], &[1., 2., 3., 4., 5.]);
+        let w = resample(&d, Tf::W); // weeks end at daily 0, 3, 4
+        let x = f64::NAN;
+        assert_series(&expand_mode(&w.close, &w, 5, Expand::Last), &[1., 1., 1., 4., 5.]);
+        assert_series(&expand_mode(&w.close, &w, 5, Expand::Point), &[1., x, x, 4., 5.]);
+        assert_series(&expand_mode(&w.open, &w, 5, Expand::First), &[1., 2., 2., 2., 5.]); // the week's open from its first day
+    }
 }
diff --git a/Context/guru/backend/src/lab/api.rs b/Context/guru/backend/src/lab/api.rs
--- a/Context/guru/backend/src/lab/api.rs
+++ b/Context/guru/backend/src/lab/api.rs
@@ -501,7 +501,8 @@
         assert!(series_json(&st, &query).unwrap_err().message().contains("p.fast"));
         query.insert("p.fast".into(), "0".into());
         assert!(series_json(&st, &query).unwrap_err().message().contains("fast must be between"));
-        assert_eq!(series_json(&st, &q(&[("id", "close"), ("tf", "Q")])).unwrap_err().message(), "tf must be D, W or M");
+        assert_eq!(series_json(&st, &q(&[("id", "close"), ("tf", "H")])).unwrap_err().message(), "tf must be D, W, M, Q, Y or nD (2D to 63D)");
+        assert!(series_json(&st, &q(&[("id", "close"), ("tf", "Q")])).unwrap().contains("\"tf\":\"Q\""));
     }
     #[test] fn run_is_labelled_simulated_and_reuses_book_accounting() {
         let st = state(400, "SPX", false);
PATCH
```

- [ ] **Step 2: Run the tests and confirm the failure**

Run: `cd Context/guru/backend && CARGO_BUILD_JOBS=2 cargo test lab:: 2>&1 | tail -20`
Expected: Compile errors: `Tf::Q`, `Expand` and `expand_mode` are missing.

- [ ] **Step 3: Implement** (apply from the repository root)

```bash
git apply --whitespace=nowarn <<'PATCH'
diff --git a/Context/guru/backend/src/lab/rules.rs b/Context/guru/backend/src/lab/rules.rs
--- a/Context/guru/backend/src/lab/rules.rs
+++ b/Context/guru/backend/src/lab/rules.rs
@@ -2,7 +2,7 @@
 //! Reserved fields return `Unsupported`; malformed input returns `Invalid` naming the field.
 use super::catalogue::{self, Params};
 use super::json::{self, Json};
-use super::resample::Tf;
+use super::resample::{Expand, Tf, TF_HELP};
 
 pub const LABEL: &str = "User-defined indicator strategy · simulated · not a book method";
 pub const MAX_CONDITIONS: usize = 12;
@@ -44,7 +44,12 @@
 pub enum Operand { Series(SeriesRef), Value(f64), BarsHeld, Expr(Box<Expr>) }
 /// Expression operands (spec §4.1), evaluated on the daily bars.
 pub const EXPRESSIONS: &[&str] = &["add", "sub", "mul", "div", "min", "max", "pct", "abs", "ref", "sum", "avg", "highest", "lowest", "change",
-    "bars_since", "value_when", "highest_since", "lowest_since", "count"];
+    "bars_since", "value_when", "highest_since", "lowest_since", "count", "compress"];
+/// TimeFrameCompress modes (spec §5.1).
+pub const AGGREGATES: &[&str] = &["last", "first", "high", "low", "sum"];
+#[derive(Clone, Copy, Debug, PartialEq, Eq)]
+pub enum Agg { Last, First, High, Low, Sum }
+impl Agg { pub fn key(self) -> &'static str { AGGREGATES[self as usize] } }
 pub const MAX_NTH: usize = 20;
 #[derive(Clone, Copy, Debug, PartialEq, Eq)]
 pub enum Bin { Add, Sub, Mul, Div, Min, Max, Pct }
@@ -62,6 +67,9 @@
     ValueWhen { when: Node, of: Operand, nth: usize },
     Since { when: Node, of: Operand, highest: bool },
     Count(Node, usize),
+    /// A series operand expanded with `expand: first|point` (`last` stays a plain series operand, the v1 path).
+    Expand(SeriesRef, Expand),
+    Compress { of: Operand, tf: Tf, agg: Agg, offset: usize, expand: Expand },
 }
 #[derive(Clone, Debug, PartialEq)]
 pub struct Cond { pub lhs: Operand, pub op: Op, pub rhs: Operand }
@@ -307,14 +315,14 @@
     if v.get("symbol").is_some() {
         return Err(LabError::Unsupported(format!("{at}.symbol: operands from another symbol (AmiBroker Foreign, TradingView request.security on another ticker) arrive in Phase 8")));
     }
-    fields(v, at, &["s", "p", "tf", "out", "offset"], &[])?;
+    fields(v, at, &["s", "p", "tf", "out", "offset", "expand"], &[])?;
     let id = v.get("s").and_then(Json::as_str).ok_or_else(|| inv(format!("{at}.s must name a series (or give a value)")))?;
     if id == "bars_held" {
         if !held { return Err(inv(format!("{at}: bars_held is only available in exit conditions (top level, not inside a group)"))); }
         if ["p", "tf", "out", "offset"].iter().any(|k| v.get(k).is_some()) { return Err(inv(format!("{at}: bars_held takes no p, tf, out or offset"))); }
         return Ok(Operand::BarsHeld);
     }
-    let tf = match v.get("tf") { None => Tf::D, Some(t) => t.as_str().and_then(Tf::parse).ok_or_else(|| inv(format!("{at}.tf must be D, W or M")))? };
+    let tf = match v.get("tf") { None => Tf::D, Some(t) => t.as_str().and_then(Tf::parse).ok_or_else(|| inv(format!("{at}.tf must be {TF_HELP}")))? };
     let offset = match v.get("offset") {
         None => 0,
         Some(o) => o.as_f64().filter(|n| n.fract() == 0. && *n >= 0. && *n <= MAX_OFFSET as f64)
@@ -326,7 +334,21 @@
         for (k, x) in kv { given.insert(k.clone(), x.as_f64().ok_or_else(|| inv(format!("{at}.p.{k} must be a number")))?); }
     }
     let out = match v.get("out") { None => None, Some(o) => Some(o.as_str().ok_or_else(|| inv(format!("{at}.out must be a string")))?) };
-    series_ref(at, id, &given, tf, out, offset).map(Operand::Series)
+    let expand = expand_of(v, at, tf, offset, id == "open")?;
+    let r = series_ref(at, id, &given, tf, out, offset)?;
+    Ok(if expand == Expand::Last { Operand::Series(r) } else { Operand::Expr(Box::new(Expr::Expand(r, expand))) })
+}
+/// `expand` (default last). `first` is accepted only where the value is known by the close of the period's first daily bar:
+/// a completed earlier period (offset >= 1) or the period's own first value (`known_at_start`). AmiBroker's docs: expandFirst
+/// looks into the future for anything else.
+fn expand_of(v: &Json, at: &str, tf: Tf, offset: usize, known_at_start: bool) -> Result<Expand, LabError> {
+    let Some(e) = v.get("expand") else { return Ok(Expand::Last) };
+    let e = e.as_str().and_then(Expand::parse).ok_or_else(|| inv(format!("{at}.expand must be last, first or point")))?;
+    if tf == Tf::D && e != Expand::Last { return Err(inv(format!("{at}.expand applies to W, M, Q, Y and nD operands"))); }
+    if e == Expand::First && offset == 0 && !known_at_start {
+        return Err(LabError::Unsupported(format!("{at}: expandFirst would show the period's final value from its first day (lookahead); use offset >= 1, the period open, or expand last")));
+    }
+    Ok(e)
 }
 
 /// An expression operand (spec §4.1), or None when `v` is a series operand.
@@ -356,6 +378,19 @@
                 Some(n) => n.as_f64().filter(|n| n.fract() == 0. && (1. ..=MAX_NTH as f64).contains(n)).ok_or_else(|| inv(format!("{at}.nth must be a whole number from 1 to {MAX_NTH}")))? as usize,
             };
             Expr::ValueWhen { when: cond_of(v, key, at, b)?, of: sub("of", b)?, nth }
+        }
+        "compress" => {
+            fields(v, at, &[key, "tf", "mode", "offset", "expand"], &[])?;
+            let tf = v.get("tf").and_then(Json::as_str).and_then(Tf::parse).filter(|t| *t != Tf::D)
+                .ok_or_else(|| inv(format!("{at}.tf must be one of W, M, Q, Y or nD (2D to 63D)")))?;
+            let agg = match v.get("mode") {
+                None => Agg::Last,
+                Some(m) => [Agg::Last, Agg::First, Agg::High, Agg::Low, Agg::Sum].into_iter().find(|a| Some(a.key()) == m.as_str())
+                    .ok_or_else(|| inv(format!("{at}.mode must be one of {}", AGGREGATES.join(", "))))?,
+            };
+            let offset = if v.get("offset").is_some() { whole(v.get("offset"), &format!("{at}.offset"), 0)? } else { 0 };
+            let expand = expand_of(v, at, tf, offset, agg == Agg::First)?;
+            Expr::Compress { of: sub(key, b)?, tf, agg, offset, expand }
         }
         "highest_since" | "lowest_since" => { fields(v, at, &[key, "of"], &[])?; Expr::Since { when: cond_of(v, key, at, b)?, of: sub("of", b)?, highest: key == "highest_since" } }
         _ => {
@@ -469,11 +504,7 @@
 fn walk_operand(o: &mut Operand, at: &str, f: &mut dyn FnMut(String, Knob)) {
     match o {
         Operand::Value(x) => f(format!("{at}.value"), Knob::Value(x)),
-        Operand::Series(r) => {
-            f(at.to_string(), Knob::Series(r));
-            let keys: Vec<String> = r.params.keys().cloned().collect();
-            for k in keys { f(format!("{at}.p.{k}"), Knob::Param(&mut *r, k)); }
-        }
+        Operand::Series(r) => walk_series(r, at, f),
         Operand::BarsHeld => {}
         Operand::Expr(e) => match &mut **e {
             Expr::Bin(op, l, r) => { walk_operand(l, &format!("{at}.{}[0]", op.key()), f); walk_operand(r, &format!("{at}.{}[1]", op.key()), f); }
@@ -484,8 +515,15 @@
             Expr::Count(c, n) => { walk_node(c, &format!("{at}.count"), f); f(format!("{at}.bars"), Knob::Whole(n)); }
             Expr::ValueWhen { when, of, nth } => { walk_node(when, &format!("{at}.value_when"), f); walk_operand(of, &format!("{at}.of"), f); f(format!("{at}.nth"), Knob::Whole(nth)); }
             Expr::Since { when, of, highest } => { walk_node(when, &format!("{at}.{}", if *highest { "highest_since" } else { "lowest_since" }), f); walk_operand(of, &format!("{at}.of"), f); }
+            Expr::Expand(r, _) => walk_series(r, at, f),
+            Expr::Compress { of, .. } => walk_operand(of, &format!("{at}.compress"), f),
         },
     }
+}
+fn walk_series(r: &mut SeriesRef, at: &str, f: &mut dyn FnMut(String, Knob)) {
+    f(at.to_string(), Knob::Series(r));
+    let keys: Vec<String> = r.params.keys().cloned().collect();
+    for k in keys { f(format!("{at}.p.{k}"), Knob::Param(&mut *r, k)); }
 }
 /// `at_least` never exceeds `of` (also after an optimize value is applied).
 fn consistent(n: &Node) -> Result<(), String> {
diff --git a/Context/guru/backend/src/lab/series.rs b/Context/guru/backend/src/lab/series.rs
--- a/Context/guru/backend/src/lab/series.rs
+++ b/Context/guru/backend/src/lab/series.rs
@@ -2,16 +2,26 @@
 //! W/M values reach daily bars through `resample::expand` (usable after the period's last daily close).
 use super::bars::Bars;
 use super::catalogue;
-use super::resample::{self, Tf};
-use super::rules::{Bin, Cond, Dist, Exec, Expr, LabError, Node, Op, Operand, SeriesRef, Spec, Win};
+use super::resample::{self, Expand, Tf};
+use super::rules::{Agg, Bin, Cond, Dist, Exec, Expr, LabError, Node, Op, Operand, SeriesRef, Spec, Win};
+use std::borrow::Cow;
 use std::collections::BTreeMap;
 use std::rc::Rc;
 
-/// Daily bars of one instrument plus their weekly and monthly resamples (built once per request).
-pub struct Frames { pub d: Bars, pub w: Bars, pub m: Bars }
+/// Daily bars of one instrument plus their calendar resamples (built once per request); n-session blocks are built on use.
+pub struct Frames { pub d: Bars, pub w: Bars, pub m: Bars, pub q: Bars, pub y: Bars }
 impl Frames {
-    pub fn new(d: Bars) -> Frames { let (w, m) = (resample::resample(&d, Tf::W), resample::resample(&d, Tf::M)); Frames { d, w, m } }
-    pub fn bars(&self, tf: Tf) -> &Bars { match tf { Tf::D => &self.d, Tf::W => &self.w, Tf::M => &self.m } }
+    pub fn new(d: Bars) -> Frames {
+        let r = |tf| resample::resample(&d, tf);
+        let (w, m, q, y) = (r(Tf::W), r(Tf::M), r(Tf::Q), r(Tf::Y));
+        Frames { d, w, m, q, y }
+    }
+    pub fn bars(&self, tf: Tf) -> Cow<'_, Bars> {
+        match tf {
+            Tf::D => Cow::Borrowed(&self.d), Tf::W => Cow::Borrowed(&self.w), Tf::M => Cow::Borrowed(&self.m),
+            Tf::Q => Cow::Borrowed(&self.q), Tf::Y => Cow::Borrowed(&self.y), Tf::Days(_) => Cow::Owned(resample::resample(&self.d, tf)),
+        }
+    }
 }
 /// Indicator outputs on their own timeframe, keyed by instrument fingerprint, id, tf and resolved params. One per request (per instrument) / optimizer thread.
 pub type Cache = BTreeMap<String, Rc<Vec<Vec<f64>>>>;
@@ -32,8 +42,11 @@
 }
 
 /// One operand as a series aligned to the daily bars (NaN = no value).
-pub fn series(fr: &Frames, cache: &mut Cache, s: &SeriesRef) -> Result<Vec<f64>, LabError> {
-    let b = fr.bars(s.tf);
+pub fn series(fr: &Frames, cache: &mut Cache, s: &SeriesRef) -> Result<Vec<f64>, LabError> { series_mode(fr, cache, s, Expand::Last) }
+/// One operand on daily bars with an AmiBroker expand mode (spec §5.1).
+fn series_mode(fr: &Frames, cache: &mut Cache, s: &SeriesRef, mode: Expand) -> Result<Vec<f64>, LabError> {
+    let held = fr.bars(s.tf);
+    let b: &Bars = &held;
     let raw = match price(b, &s.id) {
         Some(r) => r?,
         None => {
@@ -61,7 +74,7 @@
     };
     // Offset counts bars of the operand's own timeframe (W offset 1 = the previous completed week).
     let shifted: Vec<f64> = (0..raw.len()).map(|i| if i >= s.offset { raw[i - s.offset] } else { f64::NAN }).collect();
-    Ok(if s.tf == Tf::D { shifted } else { resample::expand(&shifted, b, fr.d.len()) })
+    Ok(if s.tf == Tf::D { shifted } else { resample::expand_mode(&shifted, b, fr.d.len(), mode) })
 }
 
 pub enum Val { Arr(Vec<f64>), Const(f64), Held }
@@ -172,6 +185,24 @@
                 if started { ext } else { nan }
             }).collect()
         }
+        Expr::Expand(r, mode) => series_mode(fr, cache, r, *mode)?,
+        Expr::Compress { of, tf, agg, offset, expand } => {
+            let (a, held) = (arr(of, fr, cache)?, fr.bars(*tf));
+            let b: &Bars = &held;
+            let c: Vec<f64> = (0..b.len()).map(|w| {
+                let g = &a[if w == 0 { 0 } else { b.end[w - 1] + 1 }..=b.end[w]];
+                match agg {
+                    Agg::Last => g[g.len() - 1],
+                    Agg::First => g[0],
+                    _ if g.iter().any(|v| v.is_nan()) => nan,
+                    Agg::High => g.iter().copied().fold(f64::MIN, f64::max),
+                    Agg::Low => g.iter().copied().fold(f64::MAX, f64::min),
+                    Agg::Sum => g.iter().sum(),
+                }
+            }).collect();
+            let shifted: Vec<f64> = (0..c.len()).map(|w| if w >= *offset { c[w - offset] } else { nan }).collect();
+            resample::expand_mode(&shifted, b, len, *expand)
+        }
         Expr::Win(w, x, n) => {
             let a = arr(x, fr, cache)?;
             // shortcut: O(bars × window); a running sum / monotone deque is the upgrade if optimizer profiles show it.
@@ -289,12 +320,14 @@
             Expr::Count(c, n) => format!("count({}, {n})", describe_node(c)),
             Expr::ValueWhen { when, of, nth } => format!("value_when({}, {}, {nth})", describe_node(when), operand_text(of)),
             Expr::Since { when, of, highest } => format!("{}({}, {})", if *highest { "highest_since" } else { "lowest_since" }, describe_node(when), operand_text(of)),
+            Expr::Expand(r, mode) => format!("{} expand {}", operand_text(&Operand::Series(r.clone())), mode.key()),
+            Expr::Compress { of, tf, agg, offset, expand } => format!("compress({}, {}, {}){} expand {}", operand_text(of), tf.name(), agg.key(), if *offset > 0 { format!("[{offset}]") } else { String::new() }, expand.key()),
         },
         Operand::Series(s) => {
             let p: Vec<String> = s.params.iter().map(|(k, v)| format!("{k}={v}")).collect();
             let mut t = if p.is_empty() { s.id.clone() } else { format!("{}({})", s.id, p.join(",")) };
             if s.out > 0 { if let Some(d) = catalogue::get(&s.id) { t = format!("{t}.{}", d.outputs[s.out]); } }
-            if s.tf != Tf::D { t = format!("{t}@{:?}", s.tf); }
+            if s.tf != Tf::D { t = format!("{t}@{}", s.tf.name()); }
             if s.offset > 0 { t = format!("{t}[{}]", s.offset); }
             t
         }
diff --git a/Context/guru/backend/src/lab/resample.rs b/Context/guru/backend/src/lab/resample.rs
--- a/Context/guru/backend/src/lab/resample.rs
+++ b/Context/guru/backend/src/lab/resample.rs
@@ -2,10 +2,27 @@
 //! last daily bar of its period, after that bar's close (AmiBroker expandLast, TradingView expr[1]+lookahead).
 use super::bars::Bars;
 
+/// Daily, calendar week (Mon–Sun), month, quarter, year, or a block of n sessions counted from the first bar (AmiBroker n*inDaily).
 #[derive(Clone, Copy, Debug, PartialEq, Eq)]
-pub enum Tf { D, W, M }
+pub enum Tf { D, W, M, Q, Y, Days(u16) }
+pub const MAX_DAYS: u16 = 63;
+pub const TF_HELP: &str = "D, W, M, Q, Y or nD (2D to 63D)";
 impl Tf {
-    pub fn parse(s: &str) -> Option<Tf> { match s { "D" => Some(Tf::D), "W" => Some(Tf::W), "M" => Some(Tf::M), _ => None } }
+    pub fn parse(s: &str) -> Option<Tf> {
+        match s {
+            "D" => Some(Tf::D), "W" => Some(Tf::W), "M" => Some(Tf::M), "Q" => Some(Tf::Q), "Y" => Some(Tf::Y),
+            _ => s.strip_suffix('D').filter(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit())).and_then(|n| n.parse::<u16>().ok()).filter(|n| (2..=MAX_DAYS).contains(n)).map(Tf::Days),
+        }
+    }
+    /// The spec spelling ("W", "5D").
+    pub fn name(self) -> String { match self { Tf::Days(n) => format!("{n}D"), t => format!("{t:?}") } }
+}
+/// How a higher-timeframe value reaches daily bars (AmiBroker TimeFrameExpand; spec §5.1).
+#[derive(Clone, Copy, Debug, PartialEq, Eq)]
+pub enum Expand { Last, First, Point }
+impl Expand {
+    pub fn parse(s: &str) -> Option<Expand> { match s { "last" => Some(Expand::Last), "first" => Some(Expand::First), "point" => Some(Expand::Point), _ => None } }
+    pub fn key(self) -> &'static str { match self { Expand::Last => "last", Expand::First => "first", Expand::Point => "point" } }
 }
 /// Day number (days since 1970-01-01) → "YYYY-MM-DD" (proleptic Gregorian).
 pub fn civil(z: i64) -> String {
@@ -29,7 +46,9 @@
 }
 fn key(date: &str, tf: Tf) -> i64 {
     match tf {
-        Tf::D => day_number(date),
+        Tf::D | Tf::Days(_) => day_number(date),
+        Tf::Q => period_key(date, 3),
+        Tf::Y => period_key(date, 4),
         Tf::W => { let n = day_number(date); n - (n + 3).rem_euclid(7) } // 1970-01-01 was a Thursday
         Tf::M => date[0..4].parse::<i64>().unwrap_or(0) * 12 + date[5..7].parse::<i64>().unwrap_or(0),
     }
@@ -46,7 +65,7 @@
     let mut vol: Vec<f64> = Vec::new();
     let mut last: Option<i64> = None;
     for i in 0..d.len() {
-        let k = key(&d.date[i], tf);
+        let k = if let Tf::Days(n) = tf { (i / usize::from(n)) as i64 } else { key(&d.date[i], tf) };
         let v = d.volume.as_ref().map(|v| v[i]).unwrap_or(0.);
         if last != Some(k) {
             out.date.push(d.date[i].clone()); out.open.push(d.open[i]); out.high.push(d.high[i]);
@@ -67,6 +86,19 @@
     for w in 0..htf.len() {
         let to = if w + 1 < htf.len() { htf.end[w + 1] } else { n };
         for slot in out.iter_mut().take(to).skip(htf.end[w]) { *slot = values[w]; }
+    }
+    out
+}
+
+/// `expand` with a mode: First shows period p's value on every daily bar of p; Point only on p's last daily bar.
+/// First is causal only for values known by the close of p's first bar; the rule parser enforces that.
+pub fn expand_mode(values: &[f64], htf: &Bars, n: usize, mode: Expand) -> Vec<f64> {
+    if mode == Expand::Last { return expand(values, htf, n); }
+    let mut out = vec![f64::NAN; n];
+    for w in 0..htf.len() {
+        let end = htf.end[w];
+        let start = if mode == Expand::Point { end } else if w == 0 { 0 } else { htf.end[w - 1] + 1 };
+        for slot in &mut out[start..=end] { *slot = values[w]; }
     }
     out
 }
diff --git a/Context/guru/backend/src/lab/api.rs b/Context/guru/backend/src/lab/api.rs
--- a/Context/guru/backend/src/lab/api.rs
+++ b/Context/guru/backend/src/lab/api.rs
@@ -75,8 +75,8 @@
             d.compute.is_some(), quoted(d.outputs.iter().copied()), arr(d.first.iter().map(|&f| if f == usize::MAX { "null".into() } else { f.to_string() })), d.sparse, esc(d.definition), esc(d.note), esc(tv), esc(ab))
     });
     format!(
-        "{{\"label\":\"{LABEL}\",\"price_sources\":{},\"operators\":{},\"timeframes\":[\"D\",\"W\",\"M\"],\"max_conditions\":{},\"max_spec_bytes\":{},\"max_combos\":{},\"indicators\":{}}}",
-        quoted(rules::PRICE_SOURCES.iter().copied()), quoted(rules::OPERATORS.iter().copied()), rules::MAX_CONDITIONS, rules::MAX_SPEC_BYTES, crate::MAX_COMBOS, arr(defs))
+        "{{\"label\":\"{LABEL}\",\"price_sources\":{},\"operators\":{},\"timeframes\":[\"D\",\"W\",\"M\",\"Q\",\"Y\"],\"custom_days\":[2,{}],\"expand_modes\":[\"last\",\"first\",\"point\"],\"max_conditions\":{},\"max_spec_bytes\":{},\"max_combos\":{},\"indicators\":{}}}",
+        quoted(rules::PRICE_SOURCES.iter().copied()), quoted(rules::OPERATORS.iter().copied()), super::resample::MAX_DAYS, rules::MAX_CONDITIONS, rules::MAX_SPEC_BYTES, crate::MAX_COMBOS, arr(defs))
 }
 
 /// Instrument identity from the server's own data (never from the query): type, native flag, market, currency.
@@ -99,7 +99,7 @@
 }
 pub fn series_json(st: &State, q: &Query) -> Result<String, LabError> {
     let id = q.get("id").ok_or_else(|| LabError::Invalid("id is required".into()))?;
-    let tf = q.get("tf").map_or(Some(Tf::D), |t| Tf::parse(t)).ok_or_else(|| LabError::Invalid("tf must be D, W or M".into()))?;
+    let tf = q.get("tf").map_or(Some(Tf::D), |t| Tf::parse(t)).ok_or_else(|| LabError::Invalid(format!("tf must be {}", super::resample::TF_HELP)))?;
     if let Some((k, _)) = q.iter().find(|(k, v)| k.starts_with("p.") && !v.parse::<f64>().is_ok_and(f64::is_finite)) {
         return Err(LabError::Invalid(format!("{k} must be a finite number")));
     }
@@ -117,8 +117,8 @@
         values.push(format!("\"{}\":{}", esc(name), arr(s.into_iter().map(full))));
     }
     let params = r.params.iter().map(|(k, v)| format!("\"{}\":{}", esc(k), full(*v))).collect::<Vec<_>>().join(",");
-    Ok(format!("{{\"label\":\"{LABEL}\",\"instrument\":{instrument},\"id\":\"{}\",\"tf\":\"{:?}\",\"params\":{{{params}}},\"outputs\":{},\"dates\":{},\"series\":{{{}}},\"provenance\":{}}}",
-        esc(id), tf, quoted(outputs.iter().copied()), quoted(ctx.r.iter().map(|r| r.date.as_str())), values.join(","), crate::source_provenance(st, q)))
+    Ok(format!("{{\"label\":\"{LABEL}\",\"instrument\":{instrument},\"id\":\"{}\",\"tf\":\"{}\",\"params\":{{{params}}},\"outputs\":{},\"dates\":{},\"series\":{{{}}},\"provenance\":{}}}",
+        esc(id), tf.name(), quoted(outputs.iter().copied()), quoted(ctx.r.iter().map(|r| r.date.as_str())), values.join(","), crate::source_provenance(st, q)))
 }
 fn known_type(kind: &str, ticker: &str) -> Result<(), LabError> {
     if kind == "unknown" { return Err(LabError::NeedsData(format!("{ticker}: instrument type is not established (index, ETF or catalogued stock); no lab result is published"))); }
PATCH
```

- [ ] **Step 4: Run all lab tests**

Run: `cd Context/guru/backend && CARGO_BUILD_JOBS=2 cargo test lab:: 2>&1 | grep 'test result'`
Expected: `ok. 297 passed; 0 failed` (276 at HEAD plus this phase's tests so far; a different count means a test is missing or a patch was not fully applied). `lab::pinned::existing_specs_keep_their_exact_trades` and `lab::templates::templates_reproduce_the_book_engine_on_fixtures` must be among the passing tests.

- [ ] **Step 5: Commit (only with the user's explicit permission for git writes)**

With permission: `git add Context/guru/backend/src/lab && git commit -m "feat(lab): AmiBroker higher-timeframe semantics — Q/Y/nD, expand modes, compress, no lookahead"` (no trailer, no push). Without it: stop here and report `git status --short` and `git diff --stat`.

---
### Task 6: Calendar operands — `session` counters and `sessions_left`

Spec §5.2. Adds two catalogue indicators. `session` gives the trading-day numbers in the month, quarter, year and week, the week of the month, the quarter, and the weekdays left in the month, quarter and year; all are causal. `sessions_left` gives the sessions left in the month, quarter and year from the instrument's own session dates, and is NaN in the last period of the data. It is the one declared reader of future dates (never prices), so the generic truncation tests exempt it through `periods::USES_SESSION_DATES`, and a dedicated test proves that prices never move it.

**Files:**
- Modify: `Context/guru/backend/src/lab/catalogue.rs`
- Modify: `Context/guru/backend/src/lab/periods.rs`
- Modify: `Context/guru/backend/src/lab/series.rs`
- Modify: `Context/guru/backend/src/lab/sources.rs`

**Interfaces:**
- Consumes: Task 5 (the generic test loops).
- Produces: catalogue ids `session` (9 outputs) and `sessions_left` (m, q, y); `periods::USES_SESSION_DATES`; master list 299 → 301 (computed 183 → 185; sources tv/ab/both/lab 234/25/41/1).

> Note: The test patch updates the Phase 6 master-list assertion (299 → 301, 183 → 185, AmiBroker-name count 23 → 25) and the generic-test exemption lists.

- [ ] **Step 1: Write the failing tests** (apply from the repository root)

```bash
git apply --whitespace=nowarn <<'PATCH'
diff --git a/Context/guru/backend/src/lab/series.rs b/Context/guru/backend/src/lab/series.rs
--- a/Context/guru/backend/src/lab/series.rs
+++ b/Context/guru/backend/src/lab/series.rs
@@ -412,7 +412,7 @@
         // Every catalogue indicator on D, W and M: truncating the data never changes an earlier daily value.
         let full_bars = with_breadth(business_days(420, true));
         let full = Frames::new(full_bars.clone());
-        for d in all().iter().filter(|d| d.compute.is_some()) {
+        for d in all().iter().filter(|d| d.compute.is_some() && !crate::lab::periods::USES_SESSION_DATES.contains(&d.id)) {
             for tf in [Tf::D, Tf::W, Tf::M, Tf::Q, Tf::Y, Tf::Days(5)] {
                 let r = sref(d.id, tf, d.outputs.len() - 1, 0);
                 if d.needs.breadth && tf != Tf::D { assert_eq!(series(&full, &mut Cache::new(), &r).unwrap_err().status(), "needs_data"); continue; }
diff --git a/Context/guru/backend/src/lab/periods.rs b/Context/guru/backend/src/lab/periods.rs
--- a/Context/guru/backend/src/lab/periods.rs
+++ b/Context/guru/backend/src/lab/periods.rs
@@ -201,4 +243,25 @@
         assert_series(&go("lowest_bars", &d1(), &[("length", 3.)])[0], &[N, N, 2., 2., 0., 0., 1., 2.]);
     }
     #[test] fn rules_reach_periods() { for (id, out) in [("pivots_std", "s5"), ("mtp", "prev_close"), ("calendar", "dow"), ("moon_phases", "full_moon"), ("performance", "ytd")] { compiles(id, out).unwrap(); } }
-}
+
+    #[test] fn session_counters_are_causal_calendar_arithmetic() {
+        let b = dated(&[(1., 1., 1., 1.); 4], &["2024-03-27", "2024-03-28", "2024-04-01", "2024-04-02"]);
+        let o = go("session", &b, &[]);
+        for (k, want) in [[1., 2., 1., 2.], [1., 2., 1., 2.], [1., 2., 3., 4.], [1., 2., 1., 2.], [1., 1., 1., 1.], [1., 1., 2., 2.],
+                          [2., 1., 21., 20.], [2., 1., 64., 63.], [199., 198., 196., 195.]].iter().enumerate() {
+            assert_series(&o[k], want);
+        }
+        let w = go("session", &dated(&[(1., 1., 1., 1.); 3], &["2024-04-05", "2024-04-08", "2024-04-15"]), &[]);
+        assert_series(&w[4], &[1., 2., 3.]); // a new week inside the month
+    }
+    #[test] fn sessions_left_reads_dates_only_and_is_unknown_in_the_last_period() {
+        let x = f64::NAN;
+        let dates = ["2024-01-30", "2024-01-31", "2024-02-01", "2024-02-02", "2024-03-01", "2024-04-01"];
+        let o = go("sessions_left", &dated(&[(1., 1., 1., 1.); 6], &dates), &[]);
+        assert_series(&o[0], &[1., 0., 1., 0., 0., x]);
+        assert_series(&o[1], &[4., 3., 2., 1., 0., x]);
+        assert_series(&o[2], &[x; 6]);
+        let other = go("sessions_left", &dated(&[(5., 9., 2., 7.); 6], &dates), &[]); // other prices, same dates
+        for k in 0..3 { assert_series(&other[k], &o[k]); }
+    }
+}
diff --git a/Context/guru/backend/src/lab/catalogue.rs b/Context/guru/backend/src/lab/catalogue.rs
--- a/Context/guru/backend/src/lab/catalogue.rs
+++ b/Context/guru/backend/src/lab/catalogue.rs
@@ -71,7 +71,7 @@
     use super::*;
     use crate::lab::testdata::{close_eq, walk};
     /// Breadth indicators need market breadth on the bars; Task 2 tests them with testdata::with_breadth.
-    fn computable(d: &IndicatorDef) -> bool { d.compute.is_some() && !d.needs.breadth }
+    fn computable(d: &IndicatorDef) -> bool { d.compute.is_some() && !d.needs.breadth && !crate::lab::periods::USES_SESSION_DATES.contains(&d.id) }
     #[test] fn listed_entries_carry_a_note() {
         for d in all() {
             if d.compute.is_some() { assert!(d.note.is_empty(), "{}: computed entries have no note", d.id); continue; }
@@ -144,14 +144,14 @@
     }
     #[test] fn catalogue_matches_the_phase6_master_list() {
         let defs = all();
-        assert_eq!(defs.len(), 299, "Phase 6 master list: 66 Batch A + 233 new");
-        assert_eq!(defs.iter().filter(|d| d.compute.is_some()).count(), 183);
+        assert_eq!(defs.len(), 301, "Phase 6 master list: 66 Batch A + 233 new; Phase 7: session, sessions_left");
+        assert_eq!(defs.iter().filter(|d| d.compute.is_some()).count(), 185);
         assert_eq!(defs.iter().filter(|d| d.compute.is_none()).count(), 116);
         let mut c = std::collections::BTreeMap::new();
         for (_, tv, ab) in crate::lab::sources::SOURCES {
             *c.entry(match (tv.is_empty(), ab.is_empty()) { (false, false) => "both", (false, true) => "tv", (true, false) => "ab", _ => "lab" }).or_insert(0) += 1;
         }
-        assert_eq!((c["tv"], c["ab"], c["both"], c["lab"]), (234, 23, 41, 1));
+        assert_eq!((c["tv"], c["ab"], c["both"], c["lab"]), (234, 25, 41, 1));
     }
     #[test] fn volume_indicators_refuse_without_volume() {
         let bars = walk(300, 3, false);
PATCH
```

- [ ] **Step 2: Run the tests and confirm the failure**

Run: `cd Context/guru/backend && CARGO_BUILD_JOBS=2 cargo test lab:: 2>&1 | tail -20`
Expected: `session_counters_are_causal_calendar_arithmetic` fails with unknown indicator `session`, and the master-list counts fail.

- [ ] **Step 3: Implement** (apply from the repository root)

```bash
git apply --whitespace=nowarn <<'PATCH'
diff --git a/Context/guru/backend/src/lab/periods.rs b/Context/guru/backend/src/lab/periods.rs
--- a/Context/guru/backend/src/lab/periods.rs
+++ b/Context/guru/backend/src/lab/periods.rs
@@ -125,11 +125,53 @@
 fn highest_bars(b: &Bars, prm: &Params) -> Output { Ok(vec![bars_since(&b.high, n(prm, "length"), true)]) }
 fn lowest_bars(b: &Bars, prm: &Params) -> Output { Ok(vec![bars_since(&b.low, n(prm, "length"), false)]) }
 
+/// Trading-session counters (spec §5.2), all causal: the session's number in its month, quarter, year and week (1 = first
+/// session in the data's view of that period), the week of the month, the quarter, and Monday–Friday dates left after this
+/// one in the month, quarter and year (exchange holidays unknown).
+fn session(b: &Bars, _: &Params) -> Output {
+    let len = b.len();
+    let mut out = vec![vec![NA; len]; 9];
+    let (mut count, mut wom) = ([0usize; 4], 0usize);
+    for i in 0..len {
+        let d = &b.date[i];
+        let changed = |anchor: usize| i == 0 || period_key(d, anchor) != period_key(&b.date[i - 1], anchor);
+        for (k, anchor) in [2usize, 3, 4, 1].into_iter().enumerate() { count[k] = if changed(anchor) { 1 } else { count[k] + 1 }; }
+        wom = if changed(2) { 1 } else if changed(1) { wom + 1 } else { wom };
+        let (y, m): (i64, i64) = (d[0..4].parse().unwrap_or(1970), d[5..7].parse().unwrap_or(1));
+        let start = |y: i64, m: i64| day_number(&format!("{:04}-{:02}-01", y + (m - 1) / 12, (m - 1) % 12 + 1));
+        let today = day_number(d);
+        let weekdays = |end: i64| (today + 1..=end).filter(|n| (1..=5).contains(&(n + 4).rem_euclid(7))).count() as f64;
+        for (k, v) in [count[0], count[1], count[2], count[3], wom, ((m - 1) / 3 + 1) as usize].into_iter().enumerate() { out[k][i] = v as f64; }
+        out[6][i] = weekdays(start(y, m + 1) - 1);
+        out[7][i] = weekdays(start(y, ((m - 1) / 3 + 1) * 3 + 1) - 1);
+        out[8][i] = weekdays(start(y + 1, 1) - 1);
+    }
+    Ok(out)
+}
+/// Indicators that read future session *dates* (never prices); the generic truncation tests exempt them (spec §5.2).
+pub const USES_SESSION_DATES: &[&str] = &["sessions_left"];
+/// Sessions after this one in its month / quarter / year, from the instrument's own session dates (the published exchange
+/// calendar); NaN throughout the data's last period, whose remaining sessions are not in the data.
+fn sessions_left(b: &Bars, _: &Params) -> Output {
+    let len = b.len();
+    Ok([2usize, 3, 4].into_iter().map(|anchor| {
+        let key: Vec<i64> = b.date.iter().map(|d| period_key(d, anchor)).collect();
+        let (mut v, mut left) = (vec![NA; len], 0usize);
+        for i in (0..len).rev() {
+            left = if i + 1 < len && key[i + 1] == key[i] { left + 1 } else { 0 };
+            if key[i] != key[len - 1] { v[i] = left as f64; }
+        }
+        v
+    }).collect())
+}
+
 pub const DEFS: &[IndicatorDef] = &[
     def("pivots_std", "Pivot Points Standard", PV, &["p", "r1", "r2", "r3", "r4", "r5", "s1", "s2", "s3", "s4", "s5"], PR, NO, "levels from the previous completed period's H, L, C (DM: and its open; Woodie: the current period's first open): Traditional P=(H+L+C)/3, R1=2P−L, R2=P+(H−L), R3=H+2(P−L), R4=3P+(H−3L), R5=4P+(H−4L), S mirror; Fibonacci P±0.382/0.618/1.0·(H−L); Woodie P=(H+L+2·O)/4, R3=H+2(P−L), R4=R3+(H−L); Classic P±1/2/3·(H−L) (R1/S1 as Traditional); DM X = C<O ? H+2L+C : C>O ? 2H+L+C : H+L+2C, P=X/4, R1=X/2−L, S1=X/2−H; Camarilla C±1.1(H−L)/12, /6, /4, /2, R5=(H/L)·C, S5=C−(R5−C); undefined levels NaN", &[5; 11], false, pivots_std),
     def("mtp", "Multi-Time Period Charts", PE, &["open", "high", "low", "prev_high", "prev_low", "prev_close"], PR, NO, "current period's first open and high/low so far; previous completed period's high, low, close", &[0,0,0,5,5,5], false, mtp),
     def("performance", "Performance", &[], &["w1", "m1", "m3", "m6", "ytd", "y1"], PN, NO, "roc(close, 5/21/63/126/252 bars) (declared bar horizons); ytd = 100·(close/last close of the previous calendar year − 1)", &[5,21,63,126,366,252], false, performance),
     def("calendar", "Calendar", &[], &["dow", "day", "month", "year", "doy", "bar_index"], PN, NO, "AmiBroker DayOfWeek (0 Sunday … 6 Saturday), Day, Month, Year, DayOfYear (1–366), BarIndex (0-based) of the bar's date", &[0,0,0,0,0,0], false, calendar),
+    def("session", "Trading Session Counters", &[], &["tdom", "tdoq", "tdoy", "tdow", "week_of_month", "quarter", "weekdays_left_m", "weekdays_left_q", "weekdays_left_y"], PN, NO, "session number in its month / quarter / year / week (1 = first), week of the month, quarter 1–4, Monday–Friday dates left after today in the month / quarter / year (holidays unknown; causal)", &[0; 9], false, session),
+    def("sessions_left", "Sessions Left in Period", &[], &["m", "q", "y"], PN, NO, "sessions after today in the month / quarter / year from the instrument's own session dates (the published exchange calendar), never prices; NaN in the data's last period (declared adaptation, spec §5.2)", &[0; 3], true, sessions_left),
     def("moon_phases", "Moon Phases", &[], &["new_moon", "full_moon"], PR, NO, "1 on the first bar dated on or after a new (full) moon, UTC, Meeus ch. 49 with 15 periodic terms; else 0 (declared)", &[0,0], false, moon_phases),
     def("highest_bars", "Bars Since Highest High", LN, &["bars"], PN, NO, "bars since the highest high of the last length bars (0 = this bar; ties → the most recent) (AmiBroker HHVBars, TradingView ta.highestbars as a positive count)", &[19], false, highest_bars),
     def("lowest_bars", "Bars Since Lowest Low", LN, &["bars"], PN, NO, "bars since the lowest low of the last length bars (0 = this bar; ties → the most recent)", &[19], false, lowest_bars),
diff --git a/Context/guru/backend/src/lab/sources.rs b/Context/guru/backend/src/lab/sources.rs
--- a/Context/guru/backend/src/lab/sources.rs
+++ b/Context/guru/backend/src/lab/sources.rs
@@ -247,6 +247,8 @@
     ("mtp", "Multi-Time Period Charts", ""),
     ("performance", "Performance", ""),
     ("calendar", "", "DayOfWeek; Day; Month; Year; DayOfYear; BarIndex"),
+    ("session", "", "no built-in; AFL idiom BarsSince(Month() != Ref(Month(), -1)) + 1"),
+    ("sessions_left", "", "no built-in; AFL idiom with a holiday list"),
     ("moon_phases", "Moon Phases", ""),
     ("highest_bars", "ta.highestbars (Pine)", "HHVBars; HighestBars"),
     ("lowest_bars", "ta.lowestbars (Pine)", "LLVBars; LowestBars"),
PATCH
```

- [ ] **Step 4: Run all lab tests**

Run: `cd Context/guru/backend && CARGO_BUILD_JOBS=2 cargo test lab:: 2>&1 | grep 'test result'`
Expected: `ok. 299 passed; 0 failed` (276 at HEAD plus this phase's tests so far; a different count means a test is missing or a patch was not fully applied). `lab::pinned::existing_specs_keep_their_exact_trades` and `lab::templates::templates_reproduce_the_book_engine_on_fixtures` must be among the passing tests.

- [ ] **Step 5: Commit (only with the user's explicit permission for git writes)**

With permission: `git add Context/guru/backend/src/lab && git commit -m "feat(lab): session counters and sessions-left calendar operands"` (no trailer, no push). Without it: stop here and report `git status --short` and `git diff --stat`.

---
### Task 7: Two-sided specs — direction both, short_side, opposite reverse/exit/ignore, ambiguous entries

Spec §6.1–§6.2. `direction: "both"` keeps the long rules in `entry`/`exit` and adds `short_side`. `simulate` becomes one loop over one or two sides (`Compiled.other`). An opposite entry while in a position reverses (one fill, exempt from the same-bar re-entry block), exits, or is ignored. Both entries on one flat bar enter nothing. A one-sided spec runs the same path, so the pin stays green.

**Files:**
- Modify: `Context/guru/backend/src/lab/rules.rs`
- Modify: `Context/guru/backend/src/lab/series.rs`
- Modify: `Context/guru/backend/src/lab/simulate.rs`

**Interfaces:**
- Consumes: Tasks 2–6.
- Produces: `rules::{Opposite, Side}`, `Exec.opposite`, `Spec.short_side`, private `rules::{side, exec_of, exec_mut}`; `Compiled { other, cost }`; `simulate` over sides.

> Note: The test patch edits one existing row of `customize_fields_are_bounded_and_named`: `"direction": "both"` is now valid, so the invalid example becomes `"sideways"`.

- [ ] **Step 1: Write the failing tests** (apply from the repository root)

```bash
git apply --whitespace=nowarn <<'PATCH'
diff --git a/Context/guru/backend/src/lab/rules.rs b/Context/guru/backend/src/lab/rules.rs
--- a/Context/guru/backend/src/lab/rules.rs
+++ b/Context/guru/backend/src/lab/rules.rs
@@ -684,7 +734,7 @@
     }
     #[test] fn customize_fields_are_bounded_and_named() {
         for (text, needle) in [
-            (with(r#""direction": "long""#, r#""direction": "both""#), "direction must be"),
+            (with(r#""direction": "long""#, r#""direction": "sideways""#), "direction must be"),
             (with(r#""fill": "next_open""#, r#""fill": "open""#), "execution.fill must be"),
             (with(r#""fill": "next_open""#, r#""exit_fill": "later""#), "execution.exit_fill must be"),
             (with(r#""all": ["#, r#""limit": {"pct": 3}, "all": ["#), "remove execution.fill"),
@@ -857,4 +907,43 @@
             assert!(e.message().contains(needle), "{needle:?} not in {:?}", e.message());
         }
     }
-}
+
+    // ---- Phase 7: two-sided specs (spec §6.1)
+    const BOTH: &str = r#"{"v":1,"direction":"both",
+      "entry":{"all":[{"lhs":{"s":"sma","p":{"length":5}},"op":"crosses_above","rhs":{"s":"sma","p":{"length":20}}}]},"exit":{"any":[],"stop_loss":{"pct":5}},
+      "short_side":{"entry":{"all":[{"lhs":{"s":"sma","p":{"length":5}},"op":"crosses_below","rhs":{"s":"sma","p":{"length":20}}}]},"exit":{"any":[{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":10}}],"target":{"pct":8}}},
+      "execution":{"opposite":"exit"}}"#;
+    #[test] fn two_sided_specs_parse_and_validate() {
+        let s = parse_spec(BOTH).unwrap();
+        let x = s.short_side.as_ref().unwrap();
+        assert!(!s.exec.short && x.exec.short);
+        assert_eq!((s.exec.stop_loss, x.exec.target, x.exec.stop_loss), (Some(Dist::Pct(5.)), Some(Dist::Pct(8.)), None));
+        assert_eq!((s.exec.opposite, x.exec.opposite, x.exec.fill), (Opposite::Exit, Opposite::Exit, Fill::NextOpen));
+        assert_eq!(x.exit[0].cmp().unwrap().lhs, Operand::BarsHeld);
+        assert_eq!(parse_spec(&BOTH.replace(r#"{"opposite":"exit"}"#, "{}")).unwrap().exec.opposite, Opposite::Reverse);
+        let no_short = r#"{"v":1,"direction":"both","entry":{"all":[{"lhs":{"s":"close"},"op":">","rhs":{"value":1}}]},"exit":{"any":[{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":1}}]}}"#;
+        for (text, needle) in [
+            (BOTH.replace(r#""direction":"both","#, ""), "short_side applies to direction \"both\" only"),
+            (no_short.to_string(), "needs short_side"),
+            (no_short.replace(r#""direction":"both","#, "").replace(r#"{"value":1}}]}}"#, r#"{"value":1}}]},"execution":{"opposite":"exit"}}"#), "execution.opposite applies to direction \"both\" only"),
+            (BOTH.replace(r#""direction":"both""#, r#""direction":"sideways""#), "\"long\", \"short\" or \"both\""),
+            (BOTH.replace(r#""opposite":"exit""#, r#""opposite":"flip""#), "execution.opposite must be"),
+            (BOTH.replace(r#""target":{"pct":8}"#, r#""target":{"pct":0}"#), "short_side.exit.target.pct must be a number above 0"),
+            (BOTH.replace(r#""crosses_below""#, r#""below""#), "short_side.entry.all[0].op"),
+        ] {
+            let e = err(&text);
+            assert!(e.message().contains(needle), "{needle:?} not in {:?}", e.message());
+        }
+    }
+    #[test] fn optimize_reaches_the_short_side() {
+        let text = format!(r#"{},"optimize":[{{"path":"short_side.exit.target.pct","from":4,"to":8,"step":2}},{{"path":"short_side.entry.all[0].rhs.p.length","from":10,"to":30,"step":10}},{{"path":"exit.stop_loss.pct","from":1,"to":2,"step":1}}]}}"#, &BOTH[..BOTH.len() - 1]);
+        let s = parse_spec(&text).unwrap();
+        assert_eq!(s.optimize.iter().map(|d| current(&s, &d.target)).collect::<Vec<_>>(), vec![8., 20., 5.]);
+        let t = apply(&s, &s.optimize, &[4., 30., 1.]);
+        let x = t.short_side.as_ref().unwrap();
+        let Operand::Series(r) = &x.entry[0].cmp().unwrap().rhs else { panic!() };
+        assert_eq!((x.exec.target, r.params["length"], t.exec.stop_loss), (Some(Dist::Pct(4.)), 30., Some(Dist::Pct(1.))));
+        assert!(err(&text.replace("short_side.exit.target.pct", "short_side.exit.stop_loss.pct")).message().contains("the spec has no exit.stop_loss"));
+        assert!(err(&text.replace("short_side.entry.all[0]", "short_side.entry.all[3]")).message().contains("no condition at that index"));
+    }
+}
diff --git a/Context/guru/backend/src/lab/simulate.rs b/Context/guru/backend/src/lab/simulate.rs
--- a/Context/guru/backend/src/lab/simulate.rs
+++ b/Context/guru/backend/src/lab/simulate.rs
@@ -350,4 +383,40 @@
             }
         }
     }
+
+    // ---- Phase 7: two sides, reversal, opposite exit, conflict (spec §6.1)
+    fn both(long_in: &[usize], short_in: &[usize], exec: Exec) -> Compiled {
+        let mut c = with(long_in, &[], exec.clone());
+        c.other = Some(Box::new(with(short_in, &[], Exec { short: true, ..exec })));
+        c
+    }
+    #[test] fn reversal_at_the_next_open_closes_and_opens_at_one_price() {
+        let t = simulate(&both(&[1], &[4], Exec::default()), &bars(), 0, 9);
+        assert_eq!(fills(&t), vec![(2, 102., 5, 105.), (5, 105., 9, 109.5)]);
+        assert_eq!((t[0].dir, t[0].reason.as_str(), t[1].dir, t[1].entry_kind, t[1].open), (1., "reverse to short", -1., "reverse", true));
+        assert!(crate::trade_validation::check(&t, 0, 9, 0.0005).is_ok());
+    }
+    #[test] fn reversal_at_the_close_is_one_fill_and_exempt_from_the_reentry_block() {
+        let t = simulate(&both(&[1, 6], &[4], close_fill()), &bars(), 0, 9);
+        assert_eq!(fills(&t), vec![(1, 101.5, 4, 104.5), (4, 104.5, 6, 106.5), (6, 106.5, 9, 109.5)]);
+        assert_eq!((t[1].dir, t[2].dir, t[2].entry_kind), (-1., 1., "reverse"));
+    }
+    #[test] fn opposite_exit_only_closes_and_ignore_keeps_the_position() {
+        let exit = Exec { opposite: Opposite::Exit, ..Exec::default() };
+        let t = simulate(&both(&[1], &[4, 6], exit), &bars(), 0, 9);
+        // short entry at 4 only closes the long (open of 5); the next short signal (6) opens a short at 7's open
+        assert_eq!(fills(&t), vec![(2, 102., 5, 105.), (7, 107., 9, 109.5)]);
+        assert_eq!((t[0].reason.as_str(), t[1].dir), ("opposite entry", -1.));
+        let t = simulate(&both(&[1], &[4], Exec { opposite: Opposite::Ignore, ..Exec::default() }), &bars(), 0, 9);
+        assert_eq!(fills(&t), vec![(2, 102., 9, 109.5)]);
+    }
+    #[test] fn both_entries_on_one_flat_bar_enter_nothing() {
+        let t = simulate(&both(&[2, 5], &[2], Exec::default()), &bars(), 0, 9);
+        assert_eq!(fills(&t), vec![(6, 106., 9, 109.5)]);
+    }
+    #[test] fn one_sided_specs_run_the_same_path() {
+        // a single long side never looks at `other`; the pinned-output test covers every template and v1 spec
+        let t = simulate(&with(&[1, 4], &[3], Exec::default()), &bars(), 0, 9);
+        assert_eq!(fills(&t), vec![(2, 102., 4, 104.), (5, 105., 9, 109.5)]);
+    }
 }
PATCH
```

- [ ] **Step 2: Run the tests and confirm the failure**

Run: `cd Context/guru/backend && CARGO_BUILD_JOBS=2 cargo test lab:: 2>&1 | tail -20`
Expected: Compile errors: `Opposite`, `short_side` and `Compiled.other` are missing.

- [ ] **Step 3: Implement** (apply from the repository root)

```bash
git apply --whitespace=nowarn <<'PATCH'
diff --git a/Context/guru/backend/src/lab/rules.rs b/Context/guru/backend/src/lab/rules.rs
--- a/Context/guru/backend/src/lab/rules.rs
+++ b/Context/guru/backend/src/lab/rules.rs
@@ -101,10 +101,13 @@
 /// The price-order settings an optimize path can address (`entry.limit.pct`, `exit.stop_loss.atr`, ...).
 #[derive(Clone, Copy, Debug, PartialEq, Eq)]
 pub enum Slot { Limit, StopLoss, Target, Trailing }
-#[derive(Clone, Debug, PartialEq)]
-pub struct Exec { pub short: bool, pub fill: Fill, pub exit_fill: Fill, pub limit: Option<Dist>, pub stop_loss: Option<Dist>, pub target: Option<Dist>, pub trailing: Option<Dist> }
+/// What the other side's entry does while a position is open (`direction: both`, spec §6.1).
+#[derive(Clone, Copy, Debug, PartialEq, Eq)]
+pub enum Opposite { Reverse, Exit, Ignore }
+#[derive(Clone, Debug, PartialEq)]
+pub struct Exec { pub short: bool, pub fill: Fill, pub exit_fill: Fill, pub limit: Option<Dist>, pub stop_loss: Option<Dist>, pub target: Option<Dist>, pub trailing: Option<Dist>, pub opposite: Opposite }
 impl Default for Exec {
-    fn default() -> Self { Exec { short: false, fill: Fill::NextOpen, exit_fill: Fill::NextOpen, limit: None, stop_loss: None, target: None, trailing: None } }
+    fn default() -> Self { Exec { short: false, fill: Fill::NextOpen, exit_fill: Fill::NextOpen, limit: None, stop_loss: None, target: None, trailing: None, opposite: Opposite::Reverse } }
 }
 impl Exec {
     pub fn slot(&self, s: Slot) -> Option<Dist> { match s { Slot::Limit => self.limit, Slot::StopLoss => self.stop_loss, Slot::Target => self.target, Slot::Trailing => self.trailing } }
@@ -120,51 +123,81 @@
 #[derive(Clone, Debug, PartialEq)]
 pub struct Dim { pub path: String, pub target: Target, pub values: Vec<f64> }
 #[derive(Clone, Debug, PartialEq)]
-pub struct Spec { pub entry: Vec<Node>, pub exit: Vec<Node>, pub cost_bps: Option<f64>, pub optimize: Vec<Dim>, pub exec: Exec, pub origin: Option<Origin> }
+pub struct Spec { pub entry: Vec<Node>, pub exit: Vec<Node>, pub cost_bps: Option<f64>, pub optimize: Vec<Dim>, pub exec: Exec, pub origin: Option<Origin>, pub short_side: Option<Box<Side>> }
+/// The short side of a `direction: both` spec: its own rule lists and price orders (exec.short = true).
+#[derive(Clone, Debug, PartialEq)]
+pub struct Side { pub entry: Vec<Node>, pub exit: Vec<Node>, pub exec: Exec }
 
 pub fn parse_spec(text: &str) -> Result<Spec, LabError> {
     if text.len() > MAX_SPEC_BYTES { return Err(inv(format!("spec is {} bytes; the limit is {MAX_SPEC_BYTES}", text.len()))); }
     let root = json::parse(text).map_err(|e| inv(format!("spec is not valid JSON: {e}")))?;
-    fields(&root, "spec", &["v", "direction", "origin", "entry", "exit", "execution", "optimize"], &["short", "portfolio", "scale_in"])?;
+    fields(&root, "spec", &["v", "direction", "origin", "entry", "exit", "short_side", "execution", "optimize"], &["short", "portfolio", "scale_in"])?;
     if root.get("v").and_then(Json::as_f64) != Some(1.) { return Err(inv("v must be 1")); }
     let mut exec = Exec::default();
-    match root.get("direction") {
-        None => {}
-        Some(Json::Str(d)) if d == "long" => {}
-        Some(Json::Str(d)) if d == "short" => exec.short = true,
-        _ => return Err(inv("direction must be \"long\" or \"short\"")),
-    }
+    let both = match root.get("direction") {
+        None => false,
+        Some(Json::Str(d)) if d == "long" => false,
+        Some(Json::Str(d)) if d == "short" => { exec.short = true; false }
+        Some(Json::Str(d)) if d == "both" => true,
+        _ => return Err(inv("direction must be \"long\", \"short\" or \"both\"")),
+    };
     let origin = root.get("origin").map(origin).transpose()?;
-    let entry_obj = root.get("entry").ok_or_else(|| inv("entry is required"))?;
-    fields(entry_obj, "entry", &["all", "limit"], &["stop", "scale_in"])?;
-    let exit_obj = root.get("exit").ok_or_else(|| inv("exit is required"))?;
-    fields(exit_obj, "exit", &["any", "stop_loss", "target", "trailing"], &[])?;
+    let mut budget = Budget::default();
+    let (entry, exit) = side(root.get("entry"), root.get("exit"), "", &mut exec, &mut budget)?;
+    let mut short_side = match (both, root.get("short_side")) {
+        (true, Some(s)) => {
+            fields(s, "short_side", &["entry", "exit"], &[])?;
+            let mut x = Exec { short: true, ..Exec::default() };
+            let (entry, exit) = side(s.get("entry"), s.get("exit"), "short_side.", &mut x, &mut budget)?;
+            Some(Box::new(Side { entry, exit, exec: x }))
+        }
+        (true, None) => return Err(inv("direction \"both\" needs short_side with the short entry and exit rules")),
+        (false, Some(_)) => return Err(inv("short_side applies to direction \"both\" only")),
+        (false, None) => None,
+    };
+    let mut cost_bps = None;
+    if let Some(ex) = root.get("execution") {
+        fields(ex, "execution", &["fill", "exit_fill", "cost_bps", "opposite"], &["limit", "stop"])?;
+        if let Some(f) = ex.get("fill") { exec.fill = fill(f, "execution.fill")?; exec.exit_fill = exec.fill; }
+        if let Some(f) = ex.get("exit_fill") { exec.exit_fill = fill(f, "execution.exit_fill")?; }
+        let limited = exec.limit.is_some() || short_side.as_ref().is_some_and(|s| s.exec.limit.is_some());
+        if limited && ex.get("fill").is_some() {
+            return Err(inv("execution.fill applies to market entries; entry.limit already fills on the bar after the signal, so remove execution.fill"));
+        }
+        if let Some(c) = ex.get("cost_bps") {
+            cost_bps = Some(c.as_f64().filter(|c| (0. ..=5000.).contains(c)).ok_or_else(|| inv("execution.cost_bps must be a number from 0 to 5000"))?);
+        }
+        if let Some(o) = ex.get("opposite") {
+            if !both { return Err(inv("execution.opposite applies to direction \"both\" only")); }
+            exec.opposite = match o.as_str() {
+                Some("reverse") => Opposite::Reverse, Some("exit") => Opposite::Exit, Some("ignore") => Opposite::Ignore,
+                _ => return Err(inv("execution.opposite must be \"reverse\", \"exit\" or \"ignore\"")),
+            };
+        }
+    }
+    if let Some(s) = short_side.as_mut() { (s.exec.fill, s.exec.exit_fill, s.exec.opposite) = (exec.fill, exec.exit_fill, exec.opposite); }
+    let mut spec = Spec { entry, exit, cost_bps, optimize: vec![], exec, origin, short_side };
+    if let Some(o) = root.get("optimize") { spec.optimize = dims(&spec, o)?; }
+    Ok(spec)
+}
+
+/// One side's rule lists and price orders; `pre` is "" (the long side, or the only side) or "short_side.".
+fn side(entry: Option<&Json>, exit: Option<&Json>, pre: &str, exec: &mut Exec, b: &mut Budget) -> Result<(Vec<Node>, Vec<Node>), LabError> {
+    let entry_obj = entry.ok_or_else(|| inv(format!("{pre}entry is required")))?;
+    fields(entry_obj, &format!("{pre}entry"), &["all", "limit"], &["stop", "scale_in"])?;
+    let exit_obj = exit.ok_or_else(|| inv(format!("{pre}exit is required")))?;
+    fields(exit_obj, &format!("{pre}exit"), &["any", "stop_loss", "target", "trailing"], &[])?;
     for (slot, at, hi) in SLOTS {
         let obj = if slot == Slot::Limit { entry_obj } else { exit_obj };
-        if let Some(v) = obj.get(at.split_once('.').unwrap().1) { *exec.slot_mut(slot) = Some(dist(v, at, hi)?); }
+        if let Some(v) = obj.get(at.split_once('.').unwrap().1) { *exec.slot_mut(slot) = Some(dist(v, &format!("{pre}{at}"), hi)?); }
     }
     let price_exit = exec.stop_loss.is_some() || exec.target.is_some() || exec.trailing.is_some();
-    let mut budget = Budget::default();
-    let entry = conds(entry_obj.get("all"), "entry.all", false, false, &mut budget)?;
-    let exit = conds(exit_obj.get("any"), "exit.any", true, price_exit, &mut budget)?;
+    let entry = conds(entry_obj.get("all"), &format!("{pre}entry.all"), false, false, b)?;
+    let exit = conds(exit_obj.get("any"), &format!("{pre}exit.any"), true, price_exit, b)?;
     if entry.len() + exit.len() > MAX_CONDITIONS {
-        return Err(inv(format!("{} conditions; the limit is {MAX_CONDITIONS}", entry.len() + exit.len())));
-    }
-    let mut cost_bps = None;
-    if let Some(ex) = root.get("execution") {
-        fields(ex, "execution", &["fill", "exit_fill", "cost_bps"], &["limit", "stop"])?;
-        if let Some(f) = ex.get("fill") { exec.fill = fill(f, "execution.fill")?; exec.exit_fill = exec.fill; }
-        if let Some(f) = ex.get("exit_fill") { exec.exit_fill = fill(f, "execution.exit_fill")?; }
-        if exec.limit.is_some() && ex.get("fill").is_some() {
-            return Err(inv("execution.fill applies to market entries; entry.limit already fills on the bar after the signal, so remove execution.fill"));
-        }
-        if let Some(c) = ex.get("cost_bps") {
-            cost_bps = Some(c.as_f64().filter(|c| (0. ..=5000.).contains(c)).ok_or_else(|| inv("execution.cost_bps must be a number from 0 to 5000"))?);
-        }
-    }
-    let mut spec = Spec { entry, exit, cost_bps, optimize: vec![], exec, origin };
-    if let Some(o) = root.get("optimize") { spec.optimize = dims(&spec, o)?; }
-    Ok(spec)
+        return Err(inv(format!("{}{} conditions; the limit is {MAX_CONDITIONS}", if pre.is_empty() { "" } else { "short_side: " }, entry.len() + exit.len())));
+    }
+    Ok((entry, exit))
 }
 
 fn fields(v: &Json, at: &str, allowed: &[&str], reserved: &[&str]) -> Result<(), LabError> {
@@ -189,8 +222,9 @@
     fields(v, at, &["pct", "atr", "atr_length"], &[])?;
     match (v.get("pct"), v.get("atr")) {
         (Some(p), None) if v.get("atr_length").is_none() => {
-            let min = if at == "entry.limit" { "from 0" } else { "above 0" };
-            p.as_f64().filter(|p| *p <= hi && (*p > 0. || (*p == 0. && at == "entry.limit"))).map(Dist::Pct)
+            let limit = at.ends_with("entry.limit");
+            let min = if limit { "from 0" } else { "above 0" };
+            p.as_f64().filter(|p| *p <= hi && (*p > 0. || (*p == 0. && limit))).map(Dist::Pct)
                 .ok_or_else(|| inv(format!("{at}.pct must be a number {min} and at most {hi}")))
         }
         (None, Some(m)) => {
@@ -453,16 +487,22 @@
     Ok(out)
 }
 
+/// The execution settings a path addresses: the short side's for `short_side.…`, otherwise the spec's.
+fn exec_of<'a>(s: &'a Spec, path: &str) -> Option<&'a Exec> { if path.starts_with("short_side.") { s.short_side.as_ref().map(|x| &x.exec) } else { Some(&s.exec) } }
+fn exec_mut<'a>(s: &'a mut Spec, path: &str) -> Option<&'a mut Exec> { if path.starts_with("short_side.") { s.short_side.as_mut().map(|x| &mut x.exec) } else { Some(&mut s.exec) } }
+
 fn target(spec: &Spec, path: &str) -> Result<Target, String> {
-    if let Some((slot, at, _)) = SLOTS.iter().find(|(_, at, _)| path.starts_with(&format!("{at}."))) {
-        let field = &path[at.len() + 1..];
-        return match (spec.exec.slot(*slot), field) {
+    let rest = path.strip_prefix("short_side.").unwrap_or(path);
+    if let Some((slot, at, _)) = SLOTS.iter().find(|(_, at, _)| rest.starts_with(&format!("{at}."))) {
+        let field = &rest[at.len() + 1..];
+        let exec = exec_of(spec, path).ok_or("the spec has no short_side")?;
+        return match (exec.slot(*slot), field) {
             (Some(Dist::Pct(_)), "pct") | (Some(Dist::Atr { .. }), "atr") => Ok(Target { path: path.into(), slot: Some(*slot) }),
             (None, _) => Err(format!("the spec has no {at}")),
-            _ => Err(format!("{at} is set by {}; optimize that field", if matches!(spec.exec.slot(*slot), Some(Dist::Pct(_))) { "pct" } else { "atr" })),
+            _ => Err(format!("{at} is set by {}; optimize that field", if matches!(exec.slot(*slot), Some(Dist::Pct(_))) { "pct" } else { "atr" })),
         };
     }
-    if !path.starts_with("entry.all[") && !path.starts_with("exit.any[") {
+    if !rest.starts_with("entry.all[") && !rest.starts_with("exit.any[") {
         return Err("must start with entry.all[i], exit.any[i], entry.limit, exit.stop_loss, exit.target or exit.trailing".into());
     }
     let (mut found, mut near) = (false, None::<String>);
@@ -474,8 +514,13 @@
     });
     if found { return Ok(Target { path: path.into(), slot: None }); }
     if let Some(m) = near { return Err(m); }
-    let beyond = |list: &str, n: usize| path.strip_prefix(list).and_then(|r| r.split_once(']')).and_then(|(i, _)| i.parse::<usize>().ok()).is_some_and(|i| i >= n);
-    if beyond("entry.all[", spec.entry.len()) || beyond("exit.any[", spec.exit.len()) { return Err("no condition at that index".into()); }
+    let (entry, exit) = match (path.starts_with("short_side."), &spec.short_side) {
+        (false, _) => (spec.entry.len(), spec.exit.len()),
+        (true, Some(s)) => (s.entry.len(), s.exit.len()),
+        (true, None) => return Err("the spec has no short_side".into()),
+    };
+    let beyond = |list: &str, n: usize| rest.strip_prefix(list).and_then(|r| r.split_once(']')).and_then(|(i, _)| i.parse::<usize>().ok()).is_some_and(|i| i >= n);
+    if beyond("entry.all[", entry) || beyond("exit.any[", exit) { return Err("no condition at that index".into()); }
     Err("names no number in the spec (paths end in .value, .p.<key> or a whole-number field: bars, within, of, at_least)".into())
 }
 
@@ -485,6 +530,10 @@
 fn walk(s: &mut Spec, f: &mut dyn FnMut(String, Knob)) {
     for (k, n) in s.entry.iter_mut().enumerate() { walk_node(n, &format!("entry.all[{k}]"), f); }
     for (k, n) in s.exit.iter_mut().enumerate() { walk_node(n, &format!("exit.any[{k}]"), f); }
+    if let Some(x) = s.short_side.as_mut() {
+        for (k, n) in x.entry.iter_mut().enumerate() { walk_node(n, &format!("short_side.entry.all[{k}]"), f); }
+        for (k, n) in x.exit.iter_mut().enumerate() { walk_node(n, &format!("short_side.exit.any[{k}]"), f); }
+    }
 }
 fn walk_node(n: &mut Node, at: &str, f: &mut dyn FnMut(String, Knob)) {
     match n {
@@ -537,7 +586,7 @@
 
 fn set(s: &mut Spec, t: &Target, x: f64) {
     if let Some(slot) = t.slot {
-        match s.exec.slot_mut(slot) { Some(Dist::Pct(p)) => *p = x, Some(Dist::Atr { mult, .. }) => *mult = x, None => {} }
+        match exec_mut(s, &t.path).and_then(|e| e.slot_mut(slot).as_mut()) { Some(Dist::Pct(p)) => *p = x, Some(Dist::Atr { mult, .. }) => *mult = x, None => {} }
         return;
     }
     walk(s, &mut |p, k| if p == t.path {
@@ -552,7 +601,7 @@
 fn check_value(spec: &Spec, t: &Target, x: f64) -> Result<(), String> {
     if let Some(slot) = t.slot {
         let (_, at, hi) = SLOTS.iter().find(|(s, _, _)| *s == slot).unwrap();
-        let ok = match spec.exec.slot(slot) { Some(Dist::Pct(_)) => x <= *hi && (x > 0. || (x == 0. && slot == Slot::Limit)), _ => x > 0. && x <= MAX_ATR_MULT };
+        let ok = match exec_of(spec, &t.path).and_then(|e| e.slot(slot)) { Some(Dist::Pct(_)) => x <= *hi && (x > 0. || (x == 0. && slot == Slot::Limit)), _ => x > 0. && x <= MAX_ATR_MULT };
         return if ok { Ok(()) } else { Err(format!("{at} value {x} is out of range")) };
     }
     let mut s = spec.clone();
@@ -566,11 +615,12 @@
         };
     });
     res?;
-    s.entry.iter().chain(&s.exit).try_for_each(consistent)
+    let short = s.short_side.as_ref().map(|x| x.entry.iter().chain(&x.exit));
+    s.entry.iter().chain(&s.exit).chain(short.into_iter().flatten()).try_for_each(consistent)
 }
 /// The spec's own value at an optimize target (the "base" setting the ranking prefers on ties).
 pub fn current(spec: &Spec, t: &Target) -> f64 {
-    if let Some(slot) = t.slot { return match spec.exec.slot(slot) { Some(Dist::Pct(p)) => p, Some(Dist::Atr { mult, .. }) => mult, None => f64::NAN }; }
+    if let Some(slot) = t.slot { return match exec_of(spec, &t.path).and_then(|e| e.slot(slot)) { Some(Dist::Pct(p)) => p, Some(Dist::Atr { mult, .. }) => mult, None => f64::NAN }; }
     let mut v = f64::NAN;
     walk(&mut spec.clone(), &mut |p, k| if p == t.path {
         match k {
diff --git a/Context/guru/backend/src/lab/series.rs b/Context/guru/backend/src/lab/series.rs
--- a/Context/guru/backend/src/lab/series.rs
+++ b/Context/guru/backend/src/lab/series.rs
@@ -107,10 +107,12 @@
         }
     }
 }
-pub struct Compiled { pub entry: Vec<RCond>, pub exit: Vec<RCond>, pub exec: Exec, pub atr: BTreeMap<usize, Vec<f64>> }
+/// A compiled side. `other` is the short side of a `direction: both` spec; `cost` is the run's cost per side
+/// (fraction), used by equity-level rules (spec §6.5).
+pub struct Compiled { pub entry: Vec<RCond>, pub exit: Vec<RCond>, pub exec: Exec, pub atr: BTreeMap<usize, Vec<f64>>, pub other: Option<Box<Compiled>>, pub cost: f64 }
 impl Compiled {
     /// Long, market orders at the next open, no price exits (the v1 defaults).
-    pub fn new(entry: Vec<RCond>, exit: Vec<RCond>) -> Compiled { Compiled { entry, exit, exec: Exec::default(), atr: BTreeMap::new() } }
+    pub fn new(entry: Vec<RCond>, exit: Vec<RCond>) -> Compiled { Compiled { entry, exit, exec: Exec::default(), atr: BTreeMap::new(), other: None, cost: 0. } }
     /// Price distance of `d` from `px`, using ATR at bar `k` (a bar whose close is already known). NaN while ATR warms up.
     pub fn dist(&self, d: Dist, px: f64, k: usize) -> f64 {
         match d { Dist::Pct(p) => px * p / 100., Dist::Atr { mult, length } => self.atr.get(&length).map_or(f64::NAN, |a| mult * a[k]) }
@@ -291,8 +293,13 @@
     })
 }
 pub fn compile(spec: &Spec, fr: &Frames, cache: &mut Cache) -> Result<Compiled, LabError> {
-    let mut c = Compiled { entry: list(&spec.entry, "entry.all", fr, cache)?, exit: list(&spec.exit, "exit.any", fr, cache)?, exec: spec.exec.clone(), atr: BTreeMap::new() };
-    let x = &spec.exec;
+    let mut c = side(&spec.entry, &spec.exit, &spec.exec, "", fr, cache)?;
+    if let Some(s) = &spec.short_side { c.other = Some(Box::new(side(&s.entry, &s.exit, &s.exec, "short_side.", fr, cache)?)); }
+    Ok(c)
+}
+fn side(entry: &[Node], exit: &[Node], x: &Exec, pre: &str, fr: &Frames, cache: &mut Cache) -> Result<Compiled, LabError> {
+    let mut c = Compiled { entry: list(entry, &format!("{pre}entry.all"), fr, cache)?, exit: list(exit, &format!("{pre}exit.any"), fr, cache)?, ..Compiled::new(vec![], vec![]) };
+    c.exec = x.clone();
     for d in [x.limit, x.stop_loss, x.target, x.trailing].into_iter().flatten() {
         if let Dist::Atr { length, .. } = d {
             if c.atr.contains_key(&length) { continue; }
diff --git a/Context/guru/backend/src/lab/simulate.rs b/Context/guru/backend/src/lab/simulate.rs
--- a/Context/guru/backend/src/lab/simulate.rs
+++ b/Context/guru/backend/src/lab/simulate.rs
@@ -6,7 +6,7 @@
 //! (3) at the close, exit conditions (any) fill at this close or set a next-open exit, then entry conditions (all)
 //! fill at this close, set a next-open market order or place a limit for the next bar.
 use super::bars::Bars;
-use super::rules::{Dist, Fill};
+use super::rules::{Dist, Fill, Opposite};
 use super::series::Compiled;
 use crate::connors::{Leg, Trade};
 
@@ -43,55 +43,88 @@
     }
 }
 
+/// One position at a time over one side (long or short) or, for `direction: both`, two sides (`c` long, `c.other` short).
 pub fn simulate(c: &Compiled, b: &Bars, from: usize, to: usize) -> Vec<Trade> {
-    let x = &c.exec;
-    let d = if x.short { -1. } else { 1. };
+    let sides: Vec<&Compiled> = std::iter::once(c).chain(c.other.as_deref()).collect();
+    let dir = |s: usize| if sides[s].exec.short { -1. } else { 1. };
+    let x = &c.exec; // fill, exit_fill and opposite are shared by both sides
     // v1 rule kept: a next-open market entry may share the open with a next-open exit. Close fills and limit orders
     // follow the books instead: no new order on a bar that held a position at its close or exited after its open.
-    let shares_open = x.limit.is_none() && x.fill == Fill::NextOpen;
+    let shares_open = sides.iter().all(|s| s.exec.limit.is_none()) && x.fill == Fill::NextOpen;
     let mut trades = Vec::new();
-    let mut pos: Option<Pos> = None;
-    let mut pending_entry: Option<usize> = None;
-    let mut pending_limit: Option<(usize, f64)> = None;
+    let mut pos: Option<(usize, Pos)> = None; // (side, position)
+    let mut pending_entry: Option<(usize, usize, &'static str)> = None; // (side, signal bar, entry kind)
+    let mut pending_limit: Option<(usize, usize, f64)> = None; // (side, signal bar, level)
     let mut pending_exit: Option<(usize, String)> = None;
     for i in from..=to {
         if let Some((sig, why)) = pending_exit.take() {
-            if let Some(p) = pos.take() { trades.push(p.trade(i, b.open[i], why, sig, d)); }
-        }
-        if let Some(sig) = pending_entry.take() { pos = Some(Pos::open(c, i, b.open[i], sig, "next_open", d)); }
-        if let Some((sig, lim)) = pending_limit.take() {
+            if let Some((s, p)) = pos.take() { trades.push(p.trade(i, b.open[i], why, sig, dir(s))); }
+        }
+        if let Some((s, sig, kind)) = pending_entry.take() { pos = Some((s, Pos::open(sides[s], i, b.open[i], sig, kind, dir(s)))); }
+        if let Some((s, sig, lim)) = pending_limit.take() {
+            let d = dir(s);
             let touched = if d > 0. { b.low[i] <= lim } else { b.high[i] >= lim };
-            if touched { pos = Some(Pos::open(c, i, if d > 0. { b.open[i].min(lim) } else { b.open[i].max(lim) }, sig, "limit", d)); }
+            if touched { pos = Some((s, Pos::open(sides[s], i, if d > 0. { b.open[i].min(lim) } else { b.open[i].max(lim) }, sig, "limit", d))); }
         }
         let mut exited = false; // after the open: blocks a same-bar order under book rules
-        if let Some((px, why)) = pos.as_ref().and_then(|p| p.price_exit(c, b, i, d)) {
-            trades.push(pos.take().unwrap().trade(i, px, why, i, d));
+        if let Some((px, why)) = pos.as_ref().and_then(|(s, p)| p.price_exit(sides[*s], b, i, dir(*s))) {
+            let (s, p) = pos.take().unwrap();
+            trades.push(p.trade(i, px, why, i, dir(s)));
             exited = true;
         }
         let last = i == to;
-        if let Some(p) = &pos {
-            if let Some(r) = c.exit.iter().find(|r| r.holds(i, Some(p.e))) {
+        if let Some((s, p)) = &pos {
+            let s = *s;
+            if let Some(r) = sides[s].exit.iter().find(|r| r.holds(i, Some(p.e))) {
                 match x.exit_fill {
-                    Fill::Close => { trades.push(pos.take().unwrap().trade(i, b.close[i], r.label.clone(), i, d)); exited = true; }
+                    Fill::Close => { trades.push(pos.take().unwrap().1.trade(i, b.close[i], r.label.clone(), i, dir(s))); exited = true; }
                     Fill::NextOpen if !last => pending_exit = Some((i, r.label.clone())),
                     Fill::NextOpen => {} // no next open inside the window; the position is marked open below
                 }
             }
         }
-        if let Some(p) = pos.as_mut() {
+        if let Some((s, p)) = pos.as_mut() {
+            let d = dir(*s);
             if !(p.limit_bar && p.e == i) { p.best = if d > 0. { p.best.max(b.high[i]) } else { p.best.min(b.low[i]) }; }
         }
         if last { break; } // a signal on the last bar has no fill inside the window
+        let want: Vec<usize> = (0..sides.len()).filter(|&s| sides[s].entry.iter().all(|r| r.holds(i, None))).collect();
+        // The other side's entry while a position is still open (direction both): reverse, exit, or ignore.
+        if let Some(&(s, _)) = pos.as_ref().filter(|_| pending_exit.is_none()) {
+            if sides.len() == 2 && want.contains(&(1 - s)) {
+                let why = if x.opposite == Opposite::Reverse { format!("reverse to {}", if dir(1 - s) > 0. { "long" } else { "short" }) } else { "opposite entry".into() };
+                match (x.opposite, x.fill, sides[1 - s].exec.limit) {
+                    (Opposite::Ignore, ..) => {}
+                    // a reversal is one fill: exit and new entry at the same price, exempt from the same-bar re-entry rule
+                    (Opposite::Reverse, Fill::Close, None) => {
+                        trades.push(pos.take().unwrap().1.trade(i, b.close[i], why, i, dir(s)));
+                        pos = Some((1 - s, Pos::open(sides[1 - s], i, b.close[i], i, "reverse", dir(1 - s))));
+                    }
+                    (Opposite::Reverse, Fill::NextOpen, None) => { pending_exit = Some((i, why)); pending_entry = Some((1 - s, i, "reverse")); }
+                    // a limit-entry side closes the position at the exit fill and places its order for the next bar
+                    (Opposite::Reverse, _, Some(l)) => {
+                        if x.exit_fill == Fill::Close { trades.push(pos.take().unwrap().1.trade(i, b.close[i], why, i, dir(s))); } else { pending_exit = Some((i, why)); }
+                        pending_limit = Some((1 - s, i, b.close[i] - dir(1 - s) * sides[1 - s].dist(l, b.close[i], i)));
+                    }
+                    (Opposite::Exit, ..) => {
+                        if x.exit_fill == Fill::Close { trades.push(pos.take().unwrap().1.trade(i, b.close[i], why, i, dir(s))); } else { pending_exit = Some((i, why)); }
+                    }
+                }
+                continue;
+            }
+        }
         let may_enter = if shares_open { pos.is_none() || pending_exit.is_some() } else { pos.is_none() && !exited };
-        if may_enter && c.entry.iter().all(|r| r.holds(i, None)) {
-            match (x.limit, x.fill) {
-                (Some(l), _) => pending_limit = Some((i, b.close[i] - d * c.dist(l, b.close[i], i))),
-                (None, Fill::Close) => pos = Some(Pos::open(c, i, b.close[i], i, "close", d)),
-                (None, Fill::NextOpen) => pending_entry = Some(i),
+        // both sides' entries on one bar while flat: ambiguous, no entry (spec §6.1)
+        if let (true, [s]) = (may_enter, want.as_slice()) {
+            let (s, d, side) = (*s, dir(*s), sides[*s]);
+            match (side.exec.limit, x.fill) {
+                (Some(l), _) => pending_limit = Some((s, i, b.close[i] - d * side.dist(l, b.close[i], i))),
+                (None, Fill::Close) => pos = Some((s, Pos::open(side, i, b.close[i], i, "close", d))),
+                (None, Fill::NextOpen) => pending_entry = Some((s, i, "next_open")),
             }
         }
     }
-    if let Some(p) = pos { trades.push(Trade { open: true, ..p.trade(to, b.close[to], "open".into(), to, d) }); }
+    if let Some((s, p)) = pos { trades.push(Trade { open: true, ..p.trade(to, b.close[to], "open".into(), to, dir(s)) }); }
     trades
 }
 
PATCH
```

- [ ] **Step 4: Run all lab tests**

Run: `cd Context/guru/backend && CARGO_BUILD_JOBS=2 cargo test lab:: 2>&1 | grep 'test result'`
Expected: `ok. 306 passed; 0 failed` (276 at HEAD plus this phase's tests so far; a different count means a test is missing or a patch was not fully applied). `lab::pinned::existing_specs_keep_their_exact_trades` and `lab::templates::templates_reproduce_the_book_engine_on_fixtures` must be among the passing tests.

- [ ] **Step 5: Commit (only with the user's explicit permission for git writes)**

With permission: `git add Context/guru/backend/src/lab && git commit -m "feat(lab): two-sided specs with reversal, opposite exit and ambiguous-entry rule"` (no trailer, no push). Without it: stop here and report `git status --short` and `git diff --stat`.

---
### Task 8: Entry orders — stop, stop-limit, `at` levels, points, valid bars, delay bars, edge entries, per-side resting and reversal orders

Spec §6.1–§6.2. `entry.stop` (reserved until now) becomes real, and `entry.limit` together with `entry.stop` makes a stop-limit, which fills only where the daily bar proves it. Levels can be distances (`pct`, `atr`, new `points`) or `{"at": operand}`, all read at the signal bar. Orders rest for `valid_bars` (1..20, one per side) and can rest on both sides at once; two touched on one bar fill nothing. While in a position, the other side's order is a reversal order. `delay_bars` (1..10) moves next-open fills, and a pending delayed exit is not re-placed. `entry_on: edge` needs a false → true turn.

**Files:**
- Modify: `Context/guru/backend/src/lab/rules.rs`
- Modify: `Context/guru/backend/src/lab/series.rs`
- Modify: `Context/guru/backend/src/lab/simulate.rs`

**Interfaces:**
- Consumes: Task 7.
- Produces: `Dist::Points`, `Slot::Stop`, `Exec { stop, limit_at, stop_at, valid_bars, delay, edge }`, `Exec::has_order`, `MAX_VALID_BARS`, `MAX_DELAY`, `MAX_POINTS`; `Compiled { limit_at, stop_at }`, `Compiled::has_order`; `simulate::Order` (private).

> Note: The test patch removes the `entry.stop` row from `reserved_fields_are_unsupported`, because stop entries are now real (spec §8). The other reserved rows stay.

- [ ] **Step 1: Write the failing tests** (apply from the repository root)

```bash
git apply --whitespace=nowarn <<'PATCH'
diff --git a/Context/guru/backend/src/lab/rules.rs b/Context/guru/backend/src/lab/rules.rs
--- a/Context/guru/backend/src/lab/rules.rs
+++ b/Context/guru/backend/src/lab/rules.rs
@@ -704,7 +754,6 @@
     #[test] fn reserved_fields_are_unsupported() {
         for (text, needle) in [
             (with(r#""v": 1,"#, r#""v": 1, "portfolio": {"max_open": 3},"#), "spec.portfolio"),
-            (with(r#""all": ["#, r#""stop": {"pct": 1}, "all": ["#), "entry.stop"),
             (with(r#""fill": "next_open""#, r#""fill": "limit""#), "execution.fill 'limit'"),
             (with(r#""fill": "next_open""#, r#""fill": "stop""#), "execution.fill 'stop'"),
             (with(r#""v": 1,"#, r#""v": 1, "scale_in": [],"#), "spec.scale_in"),
@@ -946,4 +995,35 @@
         assert!(err(&text.replace("short_side.exit.target.pct", "short_side.exit.stop_loss.pct")).message().contains("the spec has no exit.stop_loss"));
         assert!(err(&text.replace("short_side.entry.all[0]", "short_side.entry.all[3]")).message().contains("no condition at that index"));
     }
-}
+
+    // ---- Phase 7: entry orders (spec §6.1)
+    #[test] fn entry_orders_parse_and_validate() {
+        let no_fill = |s: &str| s.replace(r#""fill": "next_open", "#, "");
+        let s = parse_spec(&no_fill(&with(r#""all": ["#, r#""stop": {"points": 2.5}, "limit": {"at": {"s":"highest","p":{"length":5}}}, "valid_bars": 3, "all": ["#))).unwrap();
+        assert_eq!((s.exec.stop, s.exec.valid_bars, s.exec.limit), (Some(Dist::Points(2.5)), 3, None));
+        assert!(matches!(&s.exec.limit_at, Some(Operand::Series(r)) if r.id == "highest"));
+        let s = parse_spec(&with(r#""fill": "next_open", "cost_bps": 12"#, r#""fill": "next_open", "delay_bars": 2, "entry_on": "edge""#)).unwrap();
+        assert_eq!((s.exec.delay, s.exec.edge, s.exec.valid_bars), (2, true, 1));
+        for (text, needle) in [
+            (with(r#""all": ["#, r#""stop": {"pct": 1}, "all": ["#), "remove execution.fill"),
+            (with(r#""all": ["#, r#""valid_bars": 2, "all": ["#), "entry.valid_bars applies to entry.limit or entry.stop orders"),
+            (no_fill(&with(r#""all": ["#, r#""stop": {"pct": 1}, "valid_bars": 21, "all": ["#)), "valid_bars must be a whole number from 1 to 20"),
+            (no_fill(&with(r#""all": ["#, r#""stop": {"pct": 1, "points": 2}, "all": ["#)), "exactly one of pct or atr, or points"),
+            (with(r#""any": ["#, r#""stop_loss": {"at": {"s":"low"}}, "any": ["#), "exit.stop_loss.at is for entry orders"),
+            (with(r#""any": ["#, r#""stop_loss": {"points": 0}, "any": ["#), "exit.stop_loss.points must be a number above 0"),
+            (with(r#""cost_bps": 12"#, r#""cost_bps": 12, "delay_bars": 0"#), "delay_bars must be a whole number from 1 to 10"),
+            (with(r#""fill": "next_open""#, r#""fill": "close", "delay_bars": 2"#), "delay_bars applies to next_open fills only"),
+            (with(r#""cost_bps": 12"#, r#""cost_bps": 12, "entry_on": "rising""#), "entry_on must be"),
+        ] {
+            let e = err(&text);
+            assert!(e.message().contains(needle), "{needle:?} not in {:?}", e.message());
+        }
+        let opt = no_fill(&with(r#""all": ["#, r#""stop": {"at": {"s":"highest","p":{"length":5}}}, "limit": {"points": 1}, "all": ["#))
+            .replace(r#"[ {"path":"entry.all[0].lhs.p.length","from":2,"to":6,"step":1} ]"#, r#"[ {"path":"entry.stop.at.p.length","from":3,"to":9,"step":3}, {"path":"entry.limit.points","from":0,"to":2,"step":1} ]"#);
+        let s = parse_spec(&opt).unwrap();
+        assert_eq!(s.optimize.iter().map(|d| current(&s, &d.target)).collect::<Vec<_>>(), vec![5., 1.]);
+        let t = apply(&s, &s.optimize, &[9., 0.]);
+        assert!(matches!(&t.exec.stop_at, Some(Operand::Series(r)) if r.params["length"] == 9.));
+        assert_eq!(t.exec.limit, Some(Dist::Points(0.)));
+    }
+}
diff --git a/Context/guru/backend/src/lab/simulate.rs b/Context/guru/backend/src/lab/simulate.rs
--- a/Context/guru/backend/src/lab/simulate.rs
+++ b/Context/guru/backend/src/lab/simulate.rs
@@ -181,7 +239,7 @@
         assert_eq!((t[0].exit_i, t[0].exit_px), (6, 90.));
     }
     // ---- Customize: close fills, next-open exits, short, limit entries, price exits (spec 2026-10-16 §5)
-    use crate::lab::rules::{Dist, Exec, Fill};
+    use crate::lab::rules::{Dist, Exec, Fill, Operand};
     fn with(entry: &[usize], exit: &[usize], exec: Exec) -> Compiled { Compiled { exec, ..Compiled::new(vec![rule(on(entry))], vec![rule(on(exit))]) } }
     fn close_fill() -> Exec { Exec { fill: Fill::Close, exit_fill: Fill::Close, ..Exec::default() } }
     fn fills(t: &[Trade]) -> Vec<(usize, f64, usize, f64)> { t.iter().map(|t| (t.legs[0].i, t.legs[0].px, t.exit_i, t.exit_px)).collect() }
@@ -419,4 +477,79 @@
         let t = simulate(&with(&[1, 4], &[3], Exec::default()), &bars(), 0, 9);
         assert_eq!(fills(&t), vec![(2, 102., 4, 104.), (5, 105., 9, 109.5)]);
     }
+
+    // ---- Phase 7: stop, stop-limit, `at` levels, valid_bars, delay, edge, reversal orders (spec §6.1–§6.2)
+    fn ordered(entry: &[usize], exit: &[usize], exec: Exec) -> Compiled { with(entry, exit, Exec { exit_fill: Fill::Close, ..exec }) }
+    #[test] fn stop_entries_fill_at_the_level_or_the_gapped_open() {
+        let stop = Exec { stop: Some(Dist::Pct(1.)), ..Exec::default() }; // bar 2 close 102.5 → stop 103.525
+        let t = simulate(&ordered(&[2], &[6], stop.clone()), &bars(), 0, 9);
+        assert_eq!((fills(&t), t[0].entry_kind), (vec![(3, 103.525, 6, 106.5)], "stop"));
+        let mut g = bars();
+        g.open[3] = 104.; g.high[3] = 105.; // gaps over the stop: filled at the open
+        assert_eq!(simulate(&ordered(&[2], &[6], stop.clone()), &g, 0, 9)[0].legs[0].px, 104.);
+        let mut short = bars();
+        short.low[3] = 101.; // short stop below the close: 101.475
+        let t = simulate(&ordered(&[2], &[6], Exec { short: true, ..stop }), &short, 0, 9);
+        assert_eq!((t[0].dir, t[0].legs[0].px), (-1., 101.475));
+    }
+    #[test] fn orders_rest_for_valid_bars_then_expire() {
+        let mut b = bars();
+        b.high[3] = 103.4; // below the 103.525 stop
+        let one = Exec { stop: Some(Dist::Pct(1.)), ..Exec::default() };
+        assert!(simulate(&ordered(&[2], &[7], one.clone()), &b, 0, 9).is_empty());
+        let t = simulate(&ordered(&[2], &[7], Exec { valid_bars: 2, ..one }), &b, 0, 9);
+        assert_eq!(fills(&t), vec![(4, 104., 7, 107.5)]); // bar 4 opens at 104, above the stop
+    }
+    #[test] fn stop_limit_fills_only_where_the_bar_proves_it() {
+        let sl = |limit: Vec<f64>| { let mut c = ordered(&[2], &[6], Exec { stop: Some(Dist::Pct(1.)), limit: Some(Dist::Pct(0.)), ..Exec::default() }); if !limit.is_empty() { c.exec.limit = None; c.exec.limit_at = Some(Operand::Value(0.)); c.limit_at = Some(limit); } c };
+        // stop 103.525 triggers inside bar 3 but the limit (102.5, the signal close) is below it: no proof of a fill
+        assert!(simulate(&sl(vec![]), &bars(), 0, 9).is_empty());
+        // triggered at the open (gap to 104) and the low comes back to the limit: filled at the limit, a limit-type bar
+        let mut g = bars();
+        g.open[3] = 104.; g.high[3] = 105.; g.low[3] = 102.;
+        let t = simulate(&sl(vec![]), &g, 0, 9);
+        assert_eq!((t[0].legs[0].px, t[0].entry_kind), (102.5, "stop_limit"));
+        // limit above the stop: an intrabar trigger fills at the stop
+        let t = simulate(&sl(vec![104.; 10]), &bars(), 0, 9);
+        assert_eq!((t[0].legs[0].i, t[0].legs[0].px), (3, 103.525));
+    }
+    #[test] fn at_levels_come_from_the_signal_bar_and_unknown_levels_place_nothing() {
+        let mut c = ordered(&[2], &[7], Exec { stop_at: Some(Operand::Value(0.)), valid_bars: 2, ..Exec::default() });
+        c.stop_at = Some(vec![f64::NAN, f64::NAN, 105., 1., 1., 1., 1., 1., 1., 1.]); // later values never move a resting order
+        assert_eq!(fills(&simulate(&c, &bars(), 0, 9)), vec![(4, 105., 7, 107.5)]); // bar 3 high 104 < 105; bar 4 high 105
+        c.stop_at = Some(vec![f64::NAN; 10]);
+        assert!(simulate(&c, &bars(), 0, 9).is_empty());
+    }
+    #[test] fn delay_bars_moves_next_open_fills() {
+        let t = simulate(&with(&[2, 8], &[6, 7], Exec { delay: 2, ..Exec::default() }), &bars(), 0, 9);
+        // entry signal 2 → open of 4; exit signal 6 → open of 8 (bar 7's repeat does not move it); signal 8 + 2 is past the window
+        assert_eq!(fills(&t), vec![(4, 104., 8, 108.)]);
+    }
+    #[test] fn edge_entries_need_a_false_to_true_turn() {
+        let level = simulate(&with(&[1, 2, 3, 6], &[2], Exec::default()), &bars(), 0, 9);
+        assert_eq!(fills(&level), vec![(2, 102., 3, 103.), (3, 103., 9, 109.5)]);
+        let edge = simulate(&with(&[1, 2, 3, 6], &[2], Exec { edge: true, ..Exec::default() }), &bars(), 0, 9);
+        assert_eq!(fills(&edge), vec![(2, 102., 3, 103.), (7, 107., 9, 109.5)]);
+    }
+    #[test] fn both_sides_rest_an_order_while_flat_and_the_touched_one_fills() {
+        let mut c = with(&[2], &[], Exec { stop: Some(Dist::Pct(1.)), valid_bars: 3, ..Exec::default() });
+        c.other = Some(Box::new(with(&[2], &[], Exec { short: true, stop: Some(Dist::Pct(1.)), valid_bars: 3, ..Exec::default() })));
+        // bar 2 close 102.5: long stop 103.525 above, short stop 101.475 below; bar 3 (high 104, low 102) touches only the long
+        let t = simulate(&c, &bars(), 0, 9);
+        assert_eq!((t[0].legs[0].i, t[0].legs[0].px, t[0].dir), (3, 103.525, 1.));
+        let mut wide = bars();
+        wide.low[3] = 101.; // both touched on one bar: ambiguous, nothing fills; bar 4 (low 103) touches only the long
+        let t = simulate(&c, &wide, 0, 9);
+        assert_eq!((t[0].legs[0].i, t[0].legs[0].px, t[0].dir), (4, 104., 1.));
+    }
+    #[test] fn a_resting_reversal_order_flips_the_position_at_its_level() {
+        let mut c = with(&[1], &[], Exec::default());
+        c.other = Some(Box::new(with(&[4], &[], Exec { short: true, stop: Some(Dist::Pct(1.)), ..Exec::default() })));
+        let mut b = bars();
+        b.low[5] = 103.; // short stop below bar 4's close: 104.5 × 0.99 = 103.455
+        let t = simulate(&c, &b, 0, 9);
+        assert_eq!(fills(&t), vec![(2, 102., 5, 103.455), (5, 103.455, 9, 109.5)]);
+        assert_eq!((t[0].reason.as_str(), t[1].dir, t[1].entry_kind), ("reverse to short", -1., "reverse"));
+        assert!(crate::trade_validation::check(&t, 0, 9, 0.0005).is_ok());
+    }
 }
PATCH
```

- [ ] **Step 2: Run the tests and confirm the failure**

Run: `cd Context/guru/backend && CARGO_BUILD_JOBS=2 cargo test lab:: 2>&1 | tail -20`
Expected: Compile errors: `Dist::Points`, `Exec.stop` and `Compiled.stop_at` are missing.

- [ ] **Step 3: Implement** (apply from the repository root)

```bash
git apply --whitespace=nowarn <<'PATCH'
diff --git a/Context/guru/backend/src/lab/rules.rs b/Context/guru/backend/src/lab/rules.rs
--- a/Context/guru/backend/src/lab/rules.rs
+++ b/Context/guru/backend/src/lab/rules.rs
@@ -97,27 +97,40 @@
 pub enum Fill { NextOpen, Close }
 /// A price distance: percent of a reference price, or a multiple of ATR(length) read at a bar already closed.
 #[derive(Clone, Copy, Debug, PartialEq)]
-pub enum Dist { Pct(f64), Atr { mult: f64, length: usize } }
+pub enum Dist { Pct(f64), Atr { mult: f64, length: usize }, Points(f64) }
 /// The price-order settings an optimize path can address (`entry.limit.pct`, `exit.stop_loss.atr`, ...).
 #[derive(Clone, Copy, Debug, PartialEq, Eq)]
-pub enum Slot { Limit, StopLoss, Target, Trailing }
+pub enum Slot { Limit, StopLoss, Target, Trailing, Stop }
+pub const MAX_VALID_BARS: usize = 20;
+pub const MAX_DELAY: usize = 10;
+pub const MAX_POINTS: f64 = 1e9;
 /// What the other side's entry does while a position is open (`direction: both`, spec §6.1).
 #[derive(Clone, Copy, Debug, PartialEq, Eq)]
 pub enum Opposite { Reverse, Exit, Ignore }
 #[derive(Clone, Debug, PartialEq)]
-pub struct Exec { pub short: bool, pub fill: Fill, pub exit_fill: Fill, pub limit: Option<Dist>, pub stop_loss: Option<Dist>, pub target: Option<Dist>, pub trailing: Option<Dist>, pub opposite: Opposite }
+pub struct Exec {
+    pub short: bool, pub fill: Fill, pub exit_fill: Fill, pub limit: Option<Dist>, pub stop_loss: Option<Dist>, pub target: Option<Dist>, pub trailing: Option<Dist>, pub opposite: Opposite,
+    /// Phase 7 orders (spec §6.1): a stop entry, absolute order levels from operands (`at`), how many bars an order rests,
+    /// the next-open delay in bars, and edge-only entries.
+    pub stop: Option<Dist>, pub limit_at: Option<Operand>, pub stop_at: Option<Operand>, pub valid_bars: usize, pub delay: usize, pub edge: bool,
+}
 impl Default for Exec {
-    fn default() -> Self { Exec { short: false, fill: Fill::NextOpen, exit_fill: Fill::NextOpen, limit: None, stop_loss: None, target: None, trailing: None, opposite: Opposite::Reverse } }
+    fn default() -> Self {
+        Exec { short: false, fill: Fill::NextOpen, exit_fill: Fill::NextOpen, limit: None, stop_loss: None, target: None, trailing: None, opposite: Opposite::Reverse,
+               stop: None, limit_at: None, stop_at: None, valid_bars: 1, delay: 1, edge: false }
+    }
 }
 impl Exec {
-    pub fn slot(&self, s: Slot) -> Option<Dist> { match s { Slot::Limit => self.limit, Slot::StopLoss => self.stop_loss, Slot::Target => self.target, Slot::Trailing => self.trailing } }
-    fn slot_mut(&mut self, s: Slot) -> &mut Option<Dist> { match s { Slot::Limit => &mut self.limit, Slot::StopLoss => &mut self.stop_loss, Slot::Target => &mut self.target, Slot::Trailing => &mut self.trailing } }
+    pub fn slot(&self, s: Slot) -> Option<Dist> { match s { Slot::Limit => self.limit, Slot::StopLoss => self.stop_loss, Slot::Target => self.target, Slot::Trailing => self.trailing, Slot::Stop => self.stop } }
+    fn slot_mut(&mut self, s: Slot) -> &mut Option<Dist> { match s { Slot::Limit => &mut self.limit, Slot::StopLoss => &mut self.stop_loss, Slot::Target => &mut self.target, Slot::Trailing => &mut self.trailing, Slot::Stop => &mut self.stop } }
+    /// A limit, stop or stop-limit entry order (the entry rests instead of filling at market).
+    pub fn has_order(&self) -> bool { self.limit.is_some() || self.stop.is_some() || self.limit_at.is_some() || self.stop_at.is_some() }
 }
 /// Lineage of a customized book template (`origin` in the spec). Checked against the template catalogue by the API.
 #[derive(Clone, Debug, PartialEq)]
 pub struct Origin { pub book: String, pub method: String, pub version: u32 }
 /// Slot, spec path, largest pct. A limit may sit at the signal close (0%); stop-loss, target and trailing need pct > 0.
-const SLOTS: [(Slot, &str, f64); 4] = [(Slot::Limit, "entry.limit", 50.), (Slot::StopLoss, "exit.stop_loss", 99.), (Slot::Target, "exit.target", 1000.), (Slot::Trailing, "exit.trailing", 99.)];
+const SLOTS: [(Slot, &str, f64); 5] = [(Slot::Limit, "entry.limit", 50.), (Slot::StopLoss, "exit.stop_loss", 99.), (Slot::Target, "exit.target", 1000.), (Slot::Trailing, "exit.trailing", 99.), (Slot::Stop, "entry.stop", 50.)];
 pub const MAX_ATR_MULT: f64 = 50.;
 pub const MAX_ATR_LENGTH: usize = 500;
 #[derive(Clone, Debug, PartialEq)]
@@ -157,13 +170,23 @@
     };
     let mut cost_bps = None;
     if let Some(ex) = root.get("execution") {
-        fields(ex, "execution", &["fill", "exit_fill", "cost_bps", "opposite"], &["limit", "stop"])?;
+        fields(ex, "execution", &["fill", "exit_fill", "cost_bps", "opposite", "delay_bars", "entry_on"], &["limit", "stop"])?;
         if let Some(f) = ex.get("fill") { exec.fill = fill(f, "execution.fill")?; exec.exit_fill = exec.fill; }
         if let Some(f) = ex.get("exit_fill") { exec.exit_fill = fill(f, "execution.exit_fill")?; }
-        let limited = exec.limit.is_some() || short_side.as_ref().is_some_and(|s| s.exec.limit.is_some());
-        if limited && ex.get("fill").is_some() {
-            return Err(inv("execution.fill applies to market entries; entry.limit already fills on the bar after the signal, so remove execution.fill"));
-        }
+        let ordered = exec.has_order() || short_side.as_ref().is_some_and(|s| s.exec.has_order());
+        if ordered && ex.get("fill").is_some() {
+            return Err(inv("execution.fill applies to market entries; entry.limit and entry.stop orders fill on the bars after the signal, so remove execution.fill"));
+        }
+        if let Some(n) = ex.get("delay_bars") {
+            if exec.fill != Fill::NextOpen || exec.exit_fill != Fill::NextOpen { return Err(inv("execution.delay_bars applies to next_open fills only")); }
+            exec.delay = n.as_f64().filter(|n| n.fract() == 0. && (1. ..=MAX_DELAY as f64).contains(n))
+                .ok_or_else(|| inv(format!("execution.delay_bars must be a whole number from 1 to {MAX_DELAY}")))? as usize;
+        }
+        exec.edge = match ex.get("entry_on").map(Json::as_str) {
+            None | Some(Some("level")) => false,
+            Some(Some("edge")) => true,
+            _ => return Err(inv("execution.entry_on must be \"level\" or \"edge\"")),
+        };
         if let Some(c) = ex.get("cost_bps") {
             cost_bps = Some(c.as_f64().filter(|c| (0. ..=5000.).contains(c)).ok_or_else(|| inv("execution.cost_bps must be a number from 0 to 5000"))?);
         }
@@ -175,7 +198,7 @@
             };
         }
     }
-    if let Some(s) = short_side.as_mut() { (s.exec.fill, s.exec.exit_fill, s.exec.opposite) = (exec.fill, exec.exit_fill, exec.opposite); }
+    if let Some(s) = short_side.as_mut() { (s.exec.fill, s.exec.exit_fill, s.exec.opposite, s.exec.delay, s.exec.edge) = (exec.fill, exec.exit_fill, exec.opposite, exec.delay, exec.edge); }
     let mut spec = Spec { entry, exit, cost_bps, optimize: vec![], exec, origin, short_side };
     if let Some(o) = root.get("optimize") { spec.optimize = dims(&spec, o)?; }
     Ok(spec)
@@ -184,12 +207,23 @@
 /// One side's rule lists and price orders; `pre` is "" (the long side, or the only side) or "short_side.".
 fn side(entry: Option<&Json>, exit: Option<&Json>, pre: &str, exec: &mut Exec, b: &mut Budget) -> Result<(Vec<Node>, Vec<Node>), LabError> {
     let entry_obj = entry.ok_or_else(|| inv(format!("{pre}entry is required")))?;
-    fields(entry_obj, &format!("{pre}entry"), &["all", "limit"], &["stop", "scale_in"])?;
+    fields(entry_obj, &format!("{pre}entry"), &["all", "limit", "stop", "valid_bars"], &["scale_in"])?;
     let exit_obj = exit.ok_or_else(|| inv(format!("{pre}exit is required")))?;
     fields(exit_obj, &format!("{pre}exit"), &["any", "stop_loss", "target", "trailing"], &[])?;
     for (slot, at, hi) in SLOTS {
-        let obj = if slot == Slot::Limit { entry_obj } else { exit_obj };
-        if let Some(v) = obj.get(at.split_once('.').unwrap().1) { *exec.slot_mut(slot) = Some(dist(v, &format!("{pre}{at}"), hi)?); }
+        let order = matches!(slot, Slot::Limit | Slot::Stop);
+        let Some(v) = (if order { entry_obj } else { exit_obj }).get(at.split_once('.').unwrap().1) else { continue };
+        if v.get("at").is_none() { *exec.slot_mut(slot) = Some(dist(v, &format!("{pre}{at}"), hi)?); continue; }
+        // an absolute level: the operand's value at the signal bar (entry orders only)
+        if !order { return Err(inv(format!("{pre}{at}.at is for entry orders (entry.limit, entry.stop)"))); }
+        fields(v, &format!("{pre}{at}"), &["at"], &[])?;
+        let o = operand(v.get("at"), &format!("{pre}{at}.at"), false, b)?;
+        if slot == Slot::Limit { exec.limit_at = Some(o); } else { exec.stop_at = Some(o); }
+    }
+    if let Some(n) = entry_obj.get("valid_bars") {
+        if !exec.has_order() { return Err(inv(format!("{pre}entry.valid_bars applies to entry.limit or entry.stop orders"))); }
+        exec.valid_bars = n.as_f64().filter(|n| n.fract() == 0. && (1. ..=MAX_VALID_BARS as f64).contains(n))
+            .ok_or_else(|| inv(format!("{pre}entry.valid_bars must be a whole number from 1 to {MAX_VALID_BARS}")))? as usize;
     }
     let price_exit = exec.stop_loss.is_some() || exec.target.is_some() || exec.trailing.is_some();
     let entry = conds(entry_obj.get("all"), &format!("{pre}entry.all"), false, false, b)?;
@@ -219,15 +253,20 @@
 }
 /// `{"pct": p}` or `{"atr": m, "atr_length": n}` (n defaults to 14).
 fn dist(v: &Json, at: &str, hi: f64) -> Result<Dist, LabError> {
-    fields(v, at, &["pct", "atr", "atr_length"], &[])?;
-    match (v.get("pct"), v.get("atr")) {
-        (Some(p), None) if v.get("atr_length").is_none() => {
-            let limit = at.ends_with("entry.limit");
-            let min = if limit { "from 0" } else { "above 0" };
-            p.as_f64().filter(|p| *p <= hi && (*p > 0. || (*p == 0. && limit))).map(Dist::Pct)
+    fields(v, at, &["pct", "atr", "atr_length", "points"], &[])?;
+    // an entry order may sit at the signal close (0); exits need a positive distance
+    let order = at.ends_with("entry.limit") || at.ends_with("entry.stop");
+    let min = if order { "from 0" } else { "above 0" };
+    match (v.get("pct"), v.get("atr"), v.get("points")) {
+        (Some(p), None, None) if v.get("atr_length").is_none() => {
+            p.as_f64().filter(|p| *p <= hi && (*p > 0. || (*p == 0. && order))).map(Dist::Pct)
                 .ok_or_else(|| inv(format!("{at}.pct must be a number {min} and at most {hi}")))
         }
-        (None, Some(m)) => {
+        (None, None, Some(p)) if v.get("atr_length").is_none() => {
+            p.as_f64().filter(|p| *p <= MAX_POINTS && (*p > 0. || (*p == 0. && order))).map(Dist::Points)
+                .ok_or_else(|| inv(format!("{at}.points must be a number {min} and at most {MAX_POINTS}")))
+        }
+        (None, Some(m), None) => {
             let mult = m.as_f64().filter(|m| *m > 0. && *m <= MAX_ATR_MULT).ok_or_else(|| inv(format!("{at}.atr must be a number above 0 and at most {MAX_ATR_MULT}")))?;
             let length = match v.get("atr_length") {
                 None => 14,
@@ -236,7 +275,7 @@
             };
             Ok(Dist::Atr { mult, length })
         }
-        _ => Err(inv(format!("{at} needs exactly one of pct or atr (atr_length goes with atr)"))),
+        _ => Err(inv(format!("{at} needs exactly one of pct or atr, or points (atr_length goes with atr)"))),
     }
 }
 fn origin(v: &Json) -> Result<Origin, LabError> {
@@ -493,16 +532,16 @@
 
 fn target(spec: &Spec, path: &str) -> Result<Target, String> {
     let rest = path.strip_prefix("short_side.").unwrap_or(path);
-    if let Some((slot, at, _)) = SLOTS.iter().find(|(_, at, _)| rest.starts_with(&format!("{at}."))) {
+    if let Some((slot, at, _)) = SLOTS.iter().find(|(_, at, _)| rest.starts_with(&format!("{at}.")) && !rest.starts_with(&format!("{at}.at."))) {
         let field = &rest[at.len() + 1..];
         let exec = exec_of(spec, path).ok_or("the spec has no short_side")?;
         return match (exec.slot(*slot), field) {
-            (Some(Dist::Pct(_)), "pct") | (Some(Dist::Atr { .. }), "atr") => Ok(Target { path: path.into(), slot: Some(*slot) }),
+            (Some(Dist::Pct(_)), "pct") | (Some(Dist::Atr { .. }), "atr") | (Some(Dist::Points(_)), "points") => Ok(Target { path: path.into(), slot: Some(*slot) }),
             (None, _) => Err(format!("the spec has no {at}")),
-            _ => Err(format!("{at} is set by {}; optimize that field", if matches!(exec.slot(*slot), Some(Dist::Pct(_))) { "pct" } else { "atr" })),
+            (Some(d), _) => Err(format!("{at} is set by {}; optimize that field", match d { Dist::Pct(_) => "pct", Dist::Atr { .. } => "atr", Dist::Points(_) => "points" })),
         };
     }
-    if !rest.starts_with("entry.all[") && !rest.starts_with("exit.any[") {
+    if !["entry.all[", "exit.any[", "entry.limit.at.", "entry.stop.at."].iter().any(|p| rest.starts_with(p)) {
         return Err("must start with entry.all[i], exit.any[i], entry.limit, exit.stop_loss, exit.target or exit.trailing".into());
     }
     let (mut found, mut near) = (false, None::<String>);
@@ -530,10 +569,16 @@
 fn walk(s: &mut Spec, f: &mut dyn FnMut(String, Knob)) {
     for (k, n) in s.entry.iter_mut().enumerate() { walk_node(n, &format!("entry.all[{k}]"), f); }
     for (k, n) in s.exit.iter_mut().enumerate() { walk_node(n, &format!("exit.any[{k}]"), f); }
+    for (pre, x) in [("", &mut s.exec)] { walk_levels(x, pre, f); }
     if let Some(x) = s.short_side.as_mut() {
         for (k, n) in x.entry.iter_mut().enumerate() { walk_node(n, &format!("short_side.entry.all[{k}]"), f); }
         for (k, n) in x.exit.iter_mut().enumerate() { walk_node(n, &format!("short_side.exit.any[{k}]"), f); }
-    }
+        walk_levels(&mut x.exec, "short_side.", f);
+    }
+}
+fn walk_levels(x: &mut Exec, pre: &str, f: &mut dyn FnMut(String, Knob)) {
+    if let Some(o) = x.limit_at.as_mut() { walk_operand(o, &format!("{pre}entry.limit.at"), f); }
+    if let Some(o) = x.stop_at.as_mut() { walk_operand(o, &format!("{pre}entry.stop.at"), f); }
 }
 fn walk_node(n: &mut Node, at: &str, f: &mut dyn FnMut(String, Knob)) {
     match n {
@@ -586,7 +631,7 @@
 
 fn set(s: &mut Spec, t: &Target, x: f64) {
     if let Some(slot) = t.slot {
-        match exec_mut(s, &t.path).and_then(|e| e.slot_mut(slot).as_mut()) { Some(Dist::Pct(p)) => *p = x, Some(Dist::Atr { mult, .. }) => *mult = x, None => {} }
+        match exec_mut(s, &t.path).and_then(|e| e.slot_mut(slot).as_mut()) { Some(Dist::Pct(p) | Dist::Points(p)) => *p = x, Some(Dist::Atr { mult, .. }) => *mult = x, None => {} }
         return;
     }
     walk(s, &mut |p, k| if p == t.path {
@@ -601,7 +646,12 @@
 fn check_value(spec: &Spec, t: &Target, x: f64) -> Result<(), String> {
     if let Some(slot) = t.slot {
         let (_, at, hi) = SLOTS.iter().find(|(s, _, _)| *s == slot).unwrap();
-        let ok = match exec_of(spec, &t.path).and_then(|e| e.slot(slot)) { Some(Dist::Pct(_)) => x <= *hi && (x > 0. || (x == 0. && slot == Slot::Limit)), _ => x > 0. && x <= MAX_ATR_MULT };
+        let order = matches!(slot, Slot::Limit | Slot::Stop);
+        let ok = match exec_of(spec, &t.path).and_then(|e| e.slot(slot)) {
+            Some(Dist::Pct(_)) => x <= *hi && (x > 0. || (x == 0. && order)),
+            Some(Dist::Points(_)) => x <= MAX_POINTS && (x > 0. || (x == 0. && order)),
+            _ => x > 0. && x <= MAX_ATR_MULT,
+        };
         return if ok { Ok(()) } else { Err(format!("{at} value {x} is out of range")) };
     }
     let mut s = spec.clone();
@@ -620,7 +670,7 @@
 }
 /// The spec's own value at an optimize target (the "base" setting the ranking prefers on ties).
 pub fn current(spec: &Spec, t: &Target) -> f64 {
-    if let Some(slot) = t.slot { return match exec_of(spec, &t.path).and_then(|e| e.slot(slot)) { Some(Dist::Pct(p)) => p, Some(Dist::Atr { mult, .. }) => mult, None => f64::NAN }; }
+    if let Some(slot) = t.slot { return match exec_of(spec, &t.path).and_then(|e| e.slot(slot)) { Some(Dist::Pct(p) | Dist::Points(p)) => p, Some(Dist::Atr { mult, .. }) => mult, None => f64::NAN }; }
     let mut v = f64::NAN;
     walk(&mut spec.clone(), &mut |p, k| if p == t.path {
         match k {
diff --git a/Context/guru/backend/src/lab/series.rs b/Context/guru/backend/src/lab/series.rs
--- a/Context/guru/backend/src/lab/series.rs
+++ b/Context/guru/backend/src/lab/series.rs
@@ -109,13 +109,18 @@
 }
 /// A compiled side. `other` is the short side of a `direction: both` spec; `cost` is the run's cost per side
 /// (fraction), used by equity-level rules (spec §6.5).
-pub struct Compiled { pub entry: Vec<RCond>, pub exit: Vec<RCond>, pub exec: Exec, pub atr: BTreeMap<usize, Vec<f64>>, pub other: Option<Box<Compiled>>, pub cost: f64 }
+/// `limit_at` / `stop_at` hold the daily values of `entry.limit.at` / `entry.stop.at` operands.
+pub struct Compiled {
+    pub entry: Vec<RCond>, pub exit: Vec<RCond>, pub exec: Exec, pub atr: BTreeMap<usize, Vec<f64>>, pub other: Option<Box<Compiled>>, pub cost: f64,
+    pub limit_at: Option<Vec<f64>>, pub stop_at: Option<Vec<f64>>,
+}
 impl Compiled {
     /// Long, market orders at the next open, no price exits (the v1 defaults).
-    pub fn new(entry: Vec<RCond>, exit: Vec<RCond>) -> Compiled { Compiled { entry, exit, exec: Exec::default(), atr: BTreeMap::new(), other: None, cost: 0. } }
+    pub fn new(entry: Vec<RCond>, exit: Vec<RCond>) -> Compiled { Compiled { entry, exit, exec: Exec::default(), atr: BTreeMap::new(), other: None, cost: 0., limit_at: None, stop_at: None } }
+    pub fn has_order(&self) -> bool { self.exec.has_order() }
     /// Price distance of `d` from `px`, using ATR at bar `k` (a bar whose close is already known). NaN while ATR warms up.
     pub fn dist(&self, d: Dist, px: f64, k: usize) -> f64 {
-        match d { Dist::Pct(p) => px * p / 100., Dist::Atr { mult, length } => self.atr.get(&length).map_or(f64::NAN, |a| mult * a[k]) }
+        match d { Dist::Pct(p) => px * p / 100., Dist::Atr { mult, length } => self.atr.get(&length).map_or(f64::NAN, |a| mult * a[k]), Dist::Points(p) => p }
     }
 }
 
@@ -300,7 +305,9 @@
 fn side(entry: &[Node], exit: &[Node], x: &Exec, pre: &str, fr: &Frames, cache: &mut Cache) -> Result<Compiled, LabError> {
     let mut c = Compiled { entry: list(entry, &format!("{pre}entry.all"), fr, cache)?, exit: list(exit, &format!("{pre}exit.any"), fr, cache)?, ..Compiled::new(vec![], vec![]) };
     c.exec = x.clone();
-    for d in [x.limit, x.stop_loss, x.target, x.trailing].into_iter().flatten() {
+    c.limit_at = x.limit_at.as_ref().map(|o| arr(o, fr, cache)).transpose()?;
+    c.stop_at = x.stop_at.as_ref().map(|o| arr(o, fr, cache)).transpose()?;
+    for d in [x.limit, x.stop_loss, x.target, x.trailing, x.stop].into_iter().flatten() {
         if let Dist::Atr { length, .. } = d {
             if c.atr.contains_key(&length) { continue; }
             let r = SeriesRef { id: "atr".into(), params: [("length".to_string(), length as f64)].into(), tf: Tf::D, out: 0, offset: 0 };
diff --git a/Context/guru/backend/src/lab/simulate.rs b/Context/guru/backend/src/lab/simulate.rs
--- a/Context/guru/backend/src/lab/simulate.rs
+++ b/Context/guru/backend/src/lab/simulate.rs
@@ -43,43 +43,99 @@
     }
 }
 
+/// A resting entry order for bars sig+1 ..= until (spec §6.2); a NaN level is an absent leg. At most one per side: placed
+/// while flat, or, for the other side of a `direction: both` spec, while a position is open (a reversal order).
+#[derive(Clone, Copy)]
+struct Order { side: usize, sig: usize, limit: f64, stop: f64, until: usize }
+impl Order {
+    /// Fill price, entry kind and whether the bar's favourable extreme may precede the fill (limit-type fills), or None.
+    /// A long limit fills when the low reaches it (at the better of open and limit); a long stop when the high reaches it
+    /// (at the worse of open and stop); a stop-limit fills only where daily bars prove it (spec §6.2). Shorts mirror.
+    fn fill(&self, b: &Bars, i: usize, d: f64) -> Option<(f64, &'static str, bool)> {
+        let (o, down, up) = (b.open[i], if d > 0. { b.low[i] } else { b.high[i] }, if d > 0. { b.high[i] } else { b.low[i] });
+        let reaches_limit = |p: f64| d * (p - self.limit) <= 0.;
+        let reaches_stop = |p: f64| d * (p - self.stop) >= 0.;
+        match (self.limit.is_nan(), self.stop.is_nan()) {
+            (false, true) => reaches_limit(down).then(|| (if d > 0. { o.min(self.limit) } else { o.max(self.limit) }, "limit", true)),
+            (true, false) => reaches_stop(up).then(|| (if d > 0. { o.max(self.stop) } else { o.min(self.stop) }, "stop", false)),
+            (false, false) if reaches_stop(o) => {
+                if reaches_limit(o) { Some((o, "stop_limit", false)) } else if reaches_limit(down) { Some((self.limit, "stop_limit", true)) } else { None }
+            }
+            (false, false) => (reaches_stop(up) && reaches_limit(self.stop)).then_some((self.stop, "stop_limit", false)),
+            (true, true) => None,
+        }
+    }
+}
+
 /// One position at a time over one side (long or short) or, for `direction: both`, two sides (`c` long, `c.other` short).
 pub fn simulate(c: &Compiled, b: &Bars, from: usize, to: usize) -> Vec<Trade> {
     let sides: Vec<&Compiled> = std::iter::once(c).chain(c.other.as_deref()).collect();
     let dir = |s: usize| if sides[s].exec.short { -1. } else { 1. };
-    let x = &c.exec; // fill, exit_fill and opposite are shared by both sides
-    // v1 rule kept: a next-open market entry may share the open with a next-open exit. Close fills and limit orders
+    let x = &c.exec; // fill, exit_fill, opposite, delay and edge are shared by both sides
+    let delay = x.delay;
+    // v1 rule kept: a next-open market entry may share the open with a next-open exit. Close fills and resting orders
     // follow the books instead: no new order on a bar that held a position at its close or exited after its open.
-    let shares_open = sides.iter().all(|s| s.exec.limit.is_none()) && x.fill == Fill::NextOpen;
+    let shares_open = sides.iter().all(|s| !s.has_order()) && x.fill == Fill::NextOpen;
+    let holds = |s: usize, i: usize| sides[s].entry.iter().all(|r| r.holds(i, None));
+    let fires = |s: usize, i: usize| holds(s, i) && (!x.edge || i == 0 || !holds(s, i - 1));
+    // An order's levels come from the signal bar; a configured level that is unknown (warm-up) places no order.
+    let place = |s: usize, i: usize| -> Option<Order> {
+        let (side, d, close) = (sides[s], dir(s), b.close[i]);
+        let limit = side.limit_at.as_ref().map(|a| a[i]).or(side.exec.limit.map(|l| close - d * side.dist(l, close, i)));
+        let stop = side.stop_at.as_ref().map(|a| a[i]).or(side.exec.stop.map(|st| close + d * side.dist(st, close, i)));
+        (limit.map_or(true, f64::is_finite) && stop.map_or(true, f64::is_finite))
+            .then_some(Order { side: s, sig: i, limit: limit.unwrap_or(f64::NAN), stop: stop.unwrap_or(f64::NAN), until: i + side.exec.valid_bars })
+    };
     let mut trades = Vec::new();
     let mut pos: Option<(usize, Pos)> = None; // (side, position)
-    let mut pending_entry: Option<(usize, usize, &'static str)> = None; // (side, signal bar, entry kind)
-    let mut pending_limit: Option<(usize, usize, f64)> = None; // (side, signal bar, level)
-    let mut pending_exit: Option<(usize, String)> = None;
+    let mut pending_entry: Option<(usize, usize, usize, &'static str)> = None; // (side, signal bar, fill bar, entry kind)
+    let mut pending_exit: Option<(usize, usize, String)> = None; // (signal bar, fill bar, reason)
+    let mut orders: [Option<Order>; 2] = [None, None]; // one resting order per side
     for i in from..=to {
-        if let Some((sig, why)) = pending_exit.take() {
+        if pending_exit.as_ref().is_some_and(|e| e.1 == i) {
+            let (sig, _, why) = pending_exit.take().unwrap();
             if let Some((s, p)) = pos.take() { trades.push(p.trade(i, b.open[i], why, sig, dir(s))); }
         }
-        if let Some((s, sig, kind)) = pending_entry.take() { pos = Some((s, Pos::open(sides[s], i, b.open[i], sig, kind, dir(s)))); }
-        if let Some((s, sig, lim)) = pending_limit.take() {
+        if pending_entry.is_some_and(|e| e.2 == i) {
+            let (s, sig, _, kind) = pending_entry.take().unwrap();
+            if pos.is_none() { pos = Some((s, Pos::open(sides[s], i, b.open[i], sig, kind, dir(s)))); }
+        }
+        if pos.is_none() {
+            // resting orders while flat: one fill opens a position (the other side's order keeps resting); two fills on one
+            // bar are ambiguous on daily bars and fill nothing
+            let filled: Vec<(Order, (f64, &'static str, bool))> = orders.iter().flatten().filter_map(|o| o.fill(b, i, dir(o.side)).map(|f| (*o, f))).collect();
+            if let [(o, (px, kind, limit_bar))] = filled.as_slice() {
+                orders[o.side] = None;
+                let mut p = Pos::open(sides[o.side], i, *px, o.sig, kind, dir(o.side));
+                p.limit_bar = *limit_bar;
+                pos = Some((o.side, p));
+            }
+        }
+        let mut exited = false; // after the open: blocks a same-bar order under book rules
+        if let Some(&(s, ref p)) = pos.as_ref() {
             let d = dir(s);
-            let touched = if d > 0. { b.low[i] <= lim } else { b.high[i] >= lim };
-            if touched { pos = Some((s, Pos::open(sides[s], i, if d > 0. { b.open[i].min(lim) } else { b.open[i].max(lim) }, sig, "limit", d))); }
-        }
-        let mut exited = false; // after the open: blocks a same-bar order under book rules
-        if let Some((px, why)) = pos.as_ref().and_then(|(s, p)| p.price_exit(sides[*s], b, i, dir(*s))) {
-            let (s, p) = pos.take().unwrap();
-            trades.push(p.trade(i, px, why, i, dir(s)));
-            exited = true;
-        }
+            if let Some((px, why)) = p.price_exit(sides[s], b, i, d) {
+                trades.push(pos.take().unwrap().1.trade(i, px, why, i, d));
+                exited = true;
+            } else if let Some((o, (px, _, limit_bar))) = orders[1 - s.min(1)].filter(|o| o.side != s).and_then(|o| o.fill(b, i, dir(o.side)).map(|f| (o, f))) {
+                // a reversal order fills: the position closes and the other side opens at the same price
+                trades.push(pos.take().unwrap().1.trade(i, px, format!("reverse to {}", if dir(o.side) > 0. { "long" } else { "short" }), i, d));
+                let mut p = Pos::open(sides[o.side], i, px, o.sig, "reverse", dir(o.side));
+                p.limit_bar = limit_bar;
+                pos = Some((o.side, p));
+                orders[o.side] = None;
+            }
+        }
+        if let Some((s, _)) = &pos { orders[*s] = None; } // a side in a position has no entry order
+        for o in orders.iter_mut() { if o.is_some_and(|o| i >= o.until) { *o = None; } }
         let last = i == to;
-        if let Some((s, p)) = &pos {
+        if let (Some((s, p)), None) = (&pos, &pending_exit) { // a pending (delayed) exit is not re-placed
             let s = *s;
             if let Some(r) = sides[s].exit.iter().find(|r| r.holds(i, Some(p.e))) {
                 match x.exit_fill {
                     Fill::Close => { trades.push(pos.take().unwrap().1.trade(i, b.close[i], r.label.clone(), i, dir(s))); exited = true; }
-                    Fill::NextOpen if !last => pending_exit = Some((i, r.label.clone())),
-                    Fill::NextOpen => {} // no next open inside the window; the position is marked open below
+                    Fill::NextOpen if i + delay <= to => pending_exit = Some((i, i + delay, r.label.clone())),
+                    Fill::NextOpen => {} // no fill inside the window; the position is marked open below
                 }
             }
         }
@@ -88,39 +144,41 @@
             if !(p.limit_bar && p.e == i) { p.best = if d > 0. { p.best.max(b.high[i]) } else { p.best.min(b.low[i]) }; }
         }
         if last { break; } // a signal on the last bar has no fill inside the window
-        let want: Vec<usize> = (0..sides.len()).filter(|&s| sides[s].entry.iter().all(|r| r.holds(i, None))).collect();
+        let want: Vec<usize> = (0..sides.len()).filter(|&s| fires(s, i)).collect();
         // The other side's entry while a position is still open (direction both): reverse, exit, or ignore.
         if let Some(&(s, _)) = pos.as_ref().filter(|_| pending_exit.is_none()) {
-            if sides.len() == 2 && want.contains(&(1 - s)) {
-                let why = if x.opposite == Opposite::Reverse { format!("reverse to {}", if dir(1 - s) > 0. { "long" } else { "short" }) } else { "opposite entry".into() };
-                match (x.opposite, x.fill, sides[1 - s].exec.limit) {
+            let o = 1 - s.min(1);
+            if sides.len() == 2 && want.contains(&o) {
+                let why = if x.opposite == Opposite::Reverse { format!("reverse to {}", if dir(o) > 0. { "long" } else { "short" }) } else { "opposite entry".into() };
+                match (x.opposite, x.fill, sides[o].has_order()) {
                     (Opposite::Ignore, ..) => {}
+                    (Opposite::Reverse, _, true) => { if orders[o].is_none() { orders[o] = place(o, i); } }
                     // a reversal is one fill: exit and new entry at the same price, exempt from the same-bar re-entry rule
-                    (Opposite::Reverse, Fill::Close, None) => {
+                    (Opposite::Reverse, Fill::Close, false) => {
                         trades.push(pos.take().unwrap().1.trade(i, b.close[i], why, i, dir(s)));
-                        pos = Some((1 - s, Pos::open(sides[1 - s], i, b.close[i], i, "reverse", dir(1 - s))));
+                        pos = Some((o, Pos::open(sides[o], i, b.close[i], i, "reverse", dir(o))));
                     }
-                    (Opposite::Reverse, Fill::NextOpen, None) => { pending_exit = Some((i, why)); pending_entry = Some((1 - s, i, "reverse")); }
-                    // a limit-entry side closes the position at the exit fill and places its order for the next bar
-                    (Opposite::Reverse, _, Some(l)) => {
-                        if x.exit_fill == Fill::Close { trades.push(pos.take().unwrap().1.trade(i, b.close[i], why, i, dir(s))); } else { pending_exit = Some((i, why)); }
-                        pending_limit = Some((1 - s, i, b.close[i] - dir(1 - s) * sides[1 - s].dist(l, b.close[i], i)));
-                    }
-                    (Opposite::Exit, ..) => {
-                        if x.exit_fill == Fill::Close { trades.push(pos.take().unwrap().1.trade(i, b.close[i], why, i, dir(s))); } else { pending_exit = Some((i, why)); }
-                    }
+                    (Opposite::Reverse, Fill::NextOpen, false) => if i + delay <= to { pending_exit = Some((i, i + delay, why)); pending_entry = Some((o, i, i + delay, "reverse")); },
+                    (Opposite::Exit, ..) => match x.exit_fill {
+                        Fill::Close => trades.push(pos.take().unwrap().1.trade(i, b.close[i], why, i, dir(s))),
+                        Fill::NextOpen => if i + delay <= to { pending_exit = Some((i, i + delay, why)); },
+                    },
                 }
                 continue;
             }
         }
-        let may_enter = if shares_open { pos.is_none() || pending_exit.is_some() } else { pos.is_none() && !exited };
-        // both sides' entries on one bar while flat: ambiguous, no entry (spec §6.1)
-        if let (true, [s]) = (may_enter, want.as_slice()) {
-            let (s, d, side) = (*s, dir(*s), sides[*s]);
-            match (side.exec.limit, x.fill) {
-                (Some(l), _) => pending_limit = Some((s, i, b.close[i] - d * side.dist(l, b.close[i], i))),
-                (None, Fill::Close) => pos = Some((s, Pos::open(side, i, b.close[i], i, "close", d))),
-                (None, Fill::NextOpen) => pending_entry = Some((s, i, "next_open")),
+        let flat = if shares_open { pos.is_none() || pending_exit.is_some() } else { pos.is_none() && !exited };
+        let may_enter = flat && pending_entry.is_none();
+        // order sides rest an order each (a channel breakout rests both); market entries need the only signal and no resting
+        // order — both sides' market entries on one bar while flat are ambiguous: no entry (spec §6.1)
+        if may_enter {
+            for &s in want.iter().filter(|&&s| sides[s].has_order()) { if orders[s].is_none() { orders[s] = place(s, i); } }
+            if let ([s], true) = (want.as_slice(), orders.iter().all(Option::is_none) && !want.iter().any(|&s| sides[s].has_order())) {
+                let (s, d, side) = (*s, dir(*s), sides[*s]);
+                match x.fill {
+                    Fill::Close => pos = Some((s, Pos::open(side, i, b.close[i], i, "close", d))),
+                    Fill::NextOpen => if i + delay <= to { pending_entry = Some((s, i, i + delay, "next_open")); },
+                }
             }
         }
     }
PATCH
```

- [ ] **Step 4: Run all lab tests**

Run: `cd Context/guru/backend && CARGO_BUILD_JOBS=2 cargo test lab:: 2>&1 | grep 'test result'`
Expected: `ok. 315 passed; 0 failed` (276 at HEAD plus this phase's tests so far; a different count means a test is missing or a patch was not fully applied). `lab::pinned::existing_specs_keep_their_exact_trades` and `lab::templates::templates_reproduce_the_book_engine_on_fixtures` must be among the passing tests.

- [ ] **Step 5: Commit (only with the user's explicit permission for git writes)**

With permission: `git add Context/guru/backend/src/lab && git commit -m "feat(lab): stop and stop-limit entries, at-levels, valid bars, delays, edge entries, reversal orders"` (no trailer, no push). Without it: stop here and report `git status --short` and `git diff --stat`.

---
### Task 9: Exits — trailing activation, breakeven, take-profits (scale-out), min bars, TradingView intrabar path

Spec §6.1–§6.3. `exit.trailing.activate` arms the trail only once the best price (through the previous bar) has moved far enough. `exit.breakeven {after, plus}` moves the stop. `exit.take_profits` (up to 3, each a fraction of the initial position) fill into the book trade's scale-out ledger. `exit.min_bars` delays rule exits only. `execution.intrabar: ohlc_path` takes the open's nearer extreme first. The protective level is the tightest of stop-loss, trailing and breakeven.

**Files:**
- Modify: `Context/guru/backend/src/lab/rules.rs`
- Modify: `Context/guru/backend/src/lab/series.rs`
- Modify: `Context/guru/backend/src/lab/simulate.rs`

**Interfaces:**
- Consumes: Task 8.
- Produces: `Slot::{Activate, BreakevenAfter, TakeProfit(k)}`, `Exec { trail_activate, breakeven, take_profits, min_bars, ohlc_path }`, `MAX_TAKE_PROFITS`, private `rules::{slot_at, without, Exec::put}`; `Pos::{price_exit(&mut), take}` and the scale-out ledger in `Pos::trade`.

- [ ] **Step 1: Write the failing tests** (apply from the repository root)

```bash
git apply --whitespace=nowarn <<'PATCH'
diff --git a/Context/guru/backend/src/lab/rules.rs b/Context/guru/backend/src/lab/rules.rs
--- a/Context/guru/backend/src/lab/rules.rs
+++ b/Context/guru/backend/src/lab/rules.rs
@@ -1026,4 +1089,33 @@
         assert!(matches!(&t.exec.stop_at, Some(Operand::Series(r)) if r.params["length"] == 9.));
         assert_eq!(t.exec.limit, Some(Dist::Points(0.)));
     }
-}
+
+    // ---- Phase 7: exits (spec §6.1)
+    #[test] fn exits_parse_validate_and_reach_optimize() {
+        let s = parse_spec(&with(r#""any": ["#, r#""trailing": {"pct": 8, "activate": {"atr": 2}}, "breakeven": {"after": {"pct": 3}, "plus": {"points": 0.5}}, "take_profits": [{"pct": 5, "size": 0.5}, {"atr": 4, "size": 0.25}], "min_bars": 2, "any": ["#)
+            .replace(r#""cost_bps": 12"#, r#""cost_bps": 12, "intrabar": "ohlc_path""#)).unwrap();
+        assert_eq!((s.exec.trailing, s.exec.trail_activate), (Some(Dist::Pct(8.)), Some(Dist::Atr { mult: 2., length: 14 })));
+        assert_eq!(s.exec.breakeven, Some((Dist::Pct(3.), Some(Dist::Points(0.5)))));
+        assert_eq!(s.exec.take_profits, vec![(Dist::Pct(5.), 0.5), (Dist::Atr { mult: 4., length: 14 }, 0.25)]);
+        assert_eq!((s.exec.min_bars, s.exec.ohlc_path), (2, true));
+        for (text, needle) in [
+            (with(r#""any": ["#, r#""take_profits": [{"pct": 5, "size": 0.6}, {"pct": 9, "size": 0.4}], "any": ["#), "sizes must add up to less than 1"),
+            (with(r#""any": ["#, r#""take_profits": [{"pct": 5}], "any": ["#), "exit.take_profits[0].size must be a fraction"),
+            (with(r#""any": ["#, r#""take_profits": [{"pct": 1, "size": 0.1}, {"pct": 2, "size": 0.1}, {"pct": 3, "size": 0.1}, {"pct": 4, "size": 0.1}], "any": ["#), "needs 1 to 3 levels"),
+            (with(r#""any": ["#, r#""breakeven": {"plus": {"pct": 1}}, "any": ["#), "exit.breakeven.after is required"),
+            (with(r#""any": ["#, r#""trailing": {"pct": 8, "activate": {"pct": 0}}, "any": ["#), "exit.trailing.activate.pct must be a number above 0"),
+            (with(r#""any": ["#, r#""min_bars": 1.5, "any": ["#), "exit.min_bars must be a whole number from 0"),
+            (with(r#""cost_bps": 12"#, r#""cost_bps": 12, "intrabar": "random""#), "execution.intrabar must be"),
+        ] {
+            let e = err(&text);
+            assert!(e.message().contains(needle), "{needle:?} not in {:?}", e.message());
+        }
+        let opt = with(r#""any": ["#, r#""trailing": {"pct": 8, "activate": {"pct": 2}}, "breakeven": {"after": {"pct": 3}}, "take_profits": [{"pct": 5, "size": 0.5}], "any": ["#)
+            .replace(r#"[ {"path":"entry.all[0].lhs.p.length","from":2,"to":6,"step":1} ]"#, r#"[ {"path":"exit.trailing.activate.pct","from":1,"to":3,"step":1}, {"path":"exit.breakeven.after.pct","from":2,"to":4,"step":1}, {"path":"exit.take_profits[0].pct","from":4,"to":6,"step":1}, {"path":"exit.trailing.pct","from":6,"to":8,"step":2} ]"#);
+        let s = parse_spec(&opt).unwrap();
+        assert_eq!(s.optimize.iter().map(|d| current(&s, &d.target)).collect::<Vec<_>>(), vec![2., 3., 5., 8.]);
+        let t = apply(&s, &s.optimize, &[1., 4., 6., 6.]);
+        assert_eq!((t.exec.trail_activate, t.exec.breakeven.unwrap().0, t.exec.take_profits[0].0, t.exec.trailing), (Some(Dist::Pct(1.)), Dist::Pct(4.), Dist::Pct(6.), Some(Dist::Pct(6.))));
+        assert!(err(&opt.replace("exit.take_profits[0].pct", "exit.take_profits[1].pct")).message().contains("the spec has no exit.take_profits[1]"));
+    }
+}
diff --git a/Context/guru/backend/src/lab/simulate.rs b/Context/guru/backend/src/lab/simulate.rs
--- a/Context/guru/backend/src/lab/simulate.rs
+++ b/Context/guru/backend/src/lab/simulate.rs
@@ -552,4 +583,51 @@
         assert_eq!((t[0].reason.as_str(), t[1].dir, t[1].entry_kind), ("reverse to short", -1., "reverse"));
         assert!(crate::trade_validation::check(&t, 0, 9, 0.0005).is_ok());
     }
+
+    // ---- Phase 7: trailing activation, breakeven, take-profits, min_bars, ohlc_path (spec §6.1–§6.3)
+    #[test] fn trailing_arms_only_after_the_activation_level() {
+        let exec = Exec { trailing: Some(Dist::Pct(2.)), trail_activate: Some(Dist::Pct(3.)), ..close_fill() };
+        let mut b = bars();
+        b.low[4] = 101.8; // would hit the unarmed 2% trail from bar 3's high (103.9 → 101.822), but the best (104) < 105.575
+        b.high[3] = 103.9;
+        let t = simulate(&with(&[2], &[], exec.clone()), &b, 0, 9);
+        assert!(t[0].open);
+        b.high[5] = 106.; b.low[6] = 103.8; // bar 5's high 106 ≥ 105.575 arms the trail; bar 6's level 106 × 0.98 = 103.88
+        let t = simulate(&with(&[2], &[], exec), &b, 0, 9);
+        assert_eq!((t[0].exit_i, t[0].reason.as_str()), (6, "exit.trailing"));
+        assert!((t[0].exit_px - 103.88).abs() < 1e-9);
+    }
+    #[test] fn breakeven_moves_the_stop_once_the_best_price_is_far_enough() {
+        let exec = Exec { stop_loss: Some(Dist::Pct(5.)), breakeven: Some((Dist::Pct(2.), Some(Dist::Points(0.5)))), ..close_fill() };
+        let mut b = bars(); // entry close 102.5; after 104.55 is reached by bar 3's high 104; bar 4's high 105 arms it
+        b.low[5] = 102.9; // the breakeven stop is 103 (entry + 0.5); the 5% stop (97.375) would not be touched
+        let t = simulate(&with(&[2], &[], exec), &b, 0, 9);
+        assert_eq!((t[0].exit_i, t[0].exit_px, t[0].reason.as_str()), (5, 103., "exit.breakeven"));
+    }
+    #[test] fn take_profits_scale_out_and_the_rest_exits_by_rule() {
+        let exec = Exec { take_profits: vec![(Dist::Pct(1.), 0.5), (Dist::Pct(3.), 0.25)], ..close_fill() };
+        // entry close 102.5: TP1 103.525 on bar 3 (high 104), TP2 105.575 on bar 5 (high 106); the rule exit closes the rest at 7's close
+        let t = simulate(&with(&[2], &[7], exec), &bars(), 0, 9);
+        assert_eq!(t[0].exits, vec![(3, 103.525, 0.5), (5, 105.575, 0.25), (7, 107.5, 0.25)]);
+        assert!((t[0].exit_px - (103.525 * 0.5 + 105.575 * 0.25 + 107.5 * 0.25)).abs() < 1e-9);
+        assert!(crate::trade_validation::check(&t, 0, 9, 0.0005).is_ok());
+    }
+    #[test] fn min_bars_delays_rule_exits_but_not_stops() {
+        let t = simulate(&with(&[2], &[3, 4, 5], Exec { min_bars: 3, ..close_fill() }), &bars(), 0, 9);
+        assert_eq!((t[0].exit_i, t[0].reason.as_str()), (5, "rule"));
+        let mut b = bars();
+        b.low[3] = 100.;
+        let t = simulate(&with(&[2], &[], Exec { min_bars: 3, stop_loss: Some(Dist::Pct(2.)), ..close_fill() }), &b, 0, 9);
+        assert_eq!((t[0].exit_i, t[0].reason.as_str()), (3, "exit.stop_loss"));
+    }
+    #[test] fn ohlc_path_takes_the_nearer_extreme_first() {
+        let exec = Exec { stop_loss: Some(Dist::Pct(2.)), target: Some(Dist::Pct(3.)), ..close_fill() };
+        let mut b = bars(); // entry close 102.5: stop 100.45, target 105.575
+        b.open[3] = 105.; b.high[3] = 106.; b.low[3] = 100.; // open nearer the high: O → H → L → C, the target first
+        assert_eq!(simulate(&with(&[2], &[], exec.clone()), &b, 0, 9)[0].reason, "exit.stop_loss"); // default: stop first
+        let t = simulate(&with(&[2], &[], Exec { ohlc_path: true, ..exec.clone() }), &b, 0, 9);
+        assert_eq!((t[0].exit_px, t[0].reason.as_str()), (105.575, "exit.target"));
+        b.open[3] = 101.; // open nearer the low: O → L → H → C, the stop first in both modes
+        assert_eq!(simulate(&with(&[2], &[], Exec { ohlc_path: true, ..exec }), &b, 0, 9)[0].reason, "exit.stop_loss");
+    }
 }
PATCH
```

- [ ] **Step 2: Run the tests and confirm the failure**

Run: `cd Context/guru/backend && CARGO_BUILD_JOBS=2 cargo test lab:: 2>&1 | tail -20`
Expected: Compile errors: the new `Exec` fields are missing.

- [ ] **Step 3: Implement** (apply from the repository root)

```bash
git apply --whitespace=nowarn <<'PATCH'
diff --git a/Context/guru/backend/src/lab/rules.rs b/Context/guru/backend/src/lab/rules.rs
--- a/Context/guru/backend/src/lab/rules.rs
+++ b/Context/guru/backend/src/lab/rules.rs
@@ -100,7 +100,8 @@
 pub enum Dist { Pct(f64), Atr { mult: f64, length: usize }, Points(f64) }
 /// The price-order settings an optimize path can address (`entry.limit.pct`, `exit.stop_loss.atr`, ...).
 #[derive(Clone, Copy, Debug, PartialEq, Eq)]
-pub enum Slot { Limit, StopLoss, Target, Trailing, Stop }
+pub enum Slot { Limit, StopLoss, Target, Trailing, Stop, Activate, BreakevenAfter, TakeProfit(usize) }
+pub const MAX_TAKE_PROFITS: usize = 3;
 pub const MAX_VALID_BARS: usize = 20;
 pub const MAX_DELAY: usize = 10;
 pub const MAX_POINTS: f64 = 1e9;
@@ -113,16 +114,34 @@
     /// Phase 7 orders (spec §6.1): a stop entry, absolute order levels from operands (`at`), how many bars an order rests,
     /// the next-open delay in bars, and edge-only entries.
     pub stop: Option<Dist>, pub limit_at: Option<Operand>, pub stop_at: Option<Operand>, pub valid_bars: usize, pub delay: usize, pub edge: bool,
+    /// Phase 7 exits (spec §6.1): trailing activation, breakeven (after, plus), partial take-profits (level, fraction of the
+    /// initial position), the bars before rule exits count, and the TradingView intrabar path instead of stop-first.
+    pub trail_activate: Option<Dist>, pub breakeven: Option<(Dist, Option<Dist>)>, pub take_profits: Vec<(Dist, f64)>, pub min_bars: usize, pub ohlc_path: bool,
 }
 impl Default for Exec {
     fn default() -> Self {
         Exec { short: false, fill: Fill::NextOpen, exit_fill: Fill::NextOpen, limit: None, stop_loss: None, target: None, trailing: None, opposite: Opposite::Reverse,
-               stop: None, limit_at: None, stop_at: None, valid_bars: 1, delay: 1, edge: false }
+               stop: None, limit_at: None, stop_at: None, valid_bars: 1, delay: 1, edge: false,
+               trail_activate: None, breakeven: None, take_profits: vec![], min_bars: 0, ohlc_path: false }
     }
 }
 impl Exec {
-    pub fn slot(&self, s: Slot) -> Option<Dist> { match s { Slot::Limit => self.limit, Slot::StopLoss => self.stop_loss, Slot::Target => self.target, Slot::Trailing => self.trailing, Slot::Stop => self.stop } }
-    fn slot_mut(&mut self, s: Slot) -> &mut Option<Dist> { match s { Slot::Limit => &mut self.limit, Slot::StopLoss => &mut self.stop_loss, Slot::Target => &mut self.target, Slot::Trailing => &mut self.trailing, Slot::Stop => &mut self.stop } }
+    pub fn slot(&self, s: Slot) -> Option<Dist> {
+        match s {
+            Slot::Limit => self.limit, Slot::StopLoss => self.stop_loss, Slot::Target => self.target, Slot::Trailing => self.trailing, Slot::Stop => self.stop,
+            Slot::Activate => self.trail_activate, Slot::BreakevenAfter => self.breakeven.map(|b| b.0), Slot::TakeProfit(k) => self.take_profits.get(k).map(|t| t.0),
+        }
+    }
+    fn slot_mut(&mut self, s: Slot) -> Option<&mut Dist> {
+        match s {
+            Slot::Limit => self.limit.as_mut(), Slot::StopLoss => self.stop_loss.as_mut(), Slot::Target => self.target.as_mut(), Slot::Trailing => self.trailing.as_mut(),
+            Slot::Stop => self.stop.as_mut(), Slot::Activate => self.trail_activate.as_mut(), Slot::BreakevenAfter => self.breakeven.as_mut().map(|b| &mut b.0),
+            Slot::TakeProfit(k) => self.take_profits.get_mut(k).map(|t| &mut t.0),
+        }
+    }
+    fn put(&mut self, s: Slot, d: Dist) {
+        match s { Slot::Limit => self.limit = Some(d), Slot::StopLoss => self.stop_loss = Some(d), Slot::Target => self.target = Some(d), Slot::Trailing => self.trailing = Some(d), Slot::Stop => self.stop = Some(d), _ => {} }
+    }
     /// A limit, stop or stop-limit entry order (the entry rests instead of filling at market).
     pub fn has_order(&self) -> bool { self.limit.is_some() || self.stop.is_some() || self.limit_at.is_some() || self.stop_at.is_some() }
 }
@@ -170,7 +189,12 @@
     };
     let mut cost_bps = None;
     if let Some(ex) = root.get("execution") {
-        fields(ex, "execution", &["fill", "exit_fill", "cost_bps", "opposite", "delay_bars", "entry_on"], &["limit", "stop"])?;
+        fields(ex, "execution", &["fill", "exit_fill", "cost_bps", "opposite", "delay_bars", "entry_on", "intrabar"], &["limit", "stop"])?;
+        exec.ohlc_path = match ex.get("intrabar").map(Json::as_str) {
+            None | Some(Some("stop_first")) => false,
+            Some(Some("ohlc_path")) => true,
+            _ => return Err(inv("execution.intrabar must be \"stop_first\" or \"ohlc_path\"")),
+        };
         if let Some(f) = ex.get("fill") { exec.fill = fill(f, "execution.fill")?; exec.exit_fill = exec.fill; }
         if let Some(f) = ex.get("exit_fill") { exec.exit_fill = fill(f, "execution.exit_fill")?; }
         let ordered = exec.has_order() || short_side.as_ref().is_some_and(|s| s.exec.has_order());
@@ -198,7 +222,9 @@
             };
         }
     }
-    if let Some(s) = short_side.as_mut() { (s.exec.fill, s.exec.exit_fill, s.exec.opposite, s.exec.delay, s.exec.edge) = (exec.fill, exec.exit_fill, exec.opposite, exec.delay, exec.edge); }
+    if let Some(s) = short_side.as_mut() {
+        (s.exec.fill, s.exec.exit_fill, s.exec.opposite, s.exec.delay, s.exec.edge, s.exec.ohlc_path) = (exec.fill, exec.exit_fill, exec.opposite, exec.delay, exec.edge, exec.ohlc_path);
+    }
     let mut spec = Spec { entry, exit, cost_bps, optimize: vec![], exec, origin, short_side };
     if let Some(o) = root.get("optimize") { spec.optimize = dims(&spec, o)?; }
     Ok(spec)
@@ -209,11 +235,16 @@
     let entry_obj = entry.ok_or_else(|| inv(format!("{pre}entry is required")))?;
     fields(entry_obj, &format!("{pre}entry"), &["all", "limit", "stop", "valid_bars"], &["scale_in"])?;
     let exit_obj = exit.ok_or_else(|| inv(format!("{pre}exit is required")))?;
-    fields(exit_obj, &format!("{pre}exit"), &["any", "stop_loss", "target", "trailing"], &[])?;
+    fields(exit_obj, &format!("{pre}exit"), &["any", "stop_loss", "target", "trailing", "breakeven", "take_profits", "min_bars"], &[])?;
     for (slot, at, hi) in SLOTS {
         let order = matches!(slot, Slot::Limit | Slot::Stop);
         let Some(v) = (if order { entry_obj } else { exit_obj }).get(at.split_once('.').unwrap().1) else { continue };
-        if v.get("at").is_none() { *exec.slot_mut(slot) = Some(dist(v, &format!("{pre}{at}"), hi)?); continue; }
+        if slot == Slot::Trailing {
+            if let Some(a) = v.get("activate") { exec.trail_activate = Some(dist(a, &format!("{pre}exit.trailing.activate"), 1000.)?); }
+            exec.put(slot, dist(&without(v, "activate"), &format!("{pre}{at}"), hi)?);
+            continue;
+        }
+        if v.get("at").is_none() { exec.put(slot, dist(v, &format!("{pre}{at}"), hi)?); continue; }
         // an absolute level: the operand's value at the signal bar (entry orders only)
         if !order { return Err(inv(format!("{pre}{at}.at is for entry orders (entry.limit, entry.stop)"))); }
         fields(v, &format!("{pre}{at}"), &["at"], &[])?;
@@ -225,7 +256,26 @@
         exec.valid_bars = n.as_f64().filter(|n| n.fract() == 0. && (1. ..=MAX_VALID_BARS as f64).contains(n))
             .ok_or_else(|| inv(format!("{pre}entry.valid_bars must be a whole number from 1 to {MAX_VALID_BARS}")))? as usize;
     }
-    let price_exit = exec.stop_loss.is_some() || exec.target.is_some() || exec.trailing.is_some();
+    if let Some(v) = exit_obj.get("breakeven") {
+        let at = format!("{pre}exit.breakeven");
+        fields(v, &at, &["after", "plus"], &[])?;
+        let after = dist(v.get("after").ok_or_else(|| inv(format!("{at}.after is required")))?, &format!("{at}.after"), 1000.)?;
+        exec.breakeven = Some((after, v.get("plus").map(|p| dist(p, &format!("{at}.plus"), 1000.)).transpose()?));
+    }
+    if let Some(v) = exit_obj.get("take_profits") {
+        let at = format!("{pre}exit.take_profits");
+        let Json::Arr(items) = v else { return Err(inv(format!("{at} must be a list"))) };
+        if items.is_empty() || items.len() > MAX_TAKE_PROFITS { return Err(inv(format!("{at} needs 1 to {MAX_TAKE_PROFITS} levels"))); }
+        for (k, it) in items.iter().enumerate() {
+            let a = format!("{at}[{k}]");
+            let size = it.get("size").and_then(Json::as_f64).filter(|s| *s > 0. && *s < 1.)
+                .ok_or_else(|| inv(format!("{a}.size must be a fraction above 0 and below 1 of the initial position")))?;
+            exec.take_profits.push((dist(&without(it, "size"), &a, 1000.)?, size));
+        }
+        if exec.take_profits.iter().map(|t| t.1).sum::<f64>() >= 1. { return Err(inv(format!("{at} sizes must add up to less than 1; the rest exits by the other rules"))); }
+    }
+    if let Some(n) = exit_obj.get("min_bars") { exec.min_bars = whole(Some(n), &format!("{pre}exit.min_bars"), 0)?; }
+    let price_exit = exec.stop_loss.is_some() || exec.target.is_some() || exec.trailing.is_some() || exec.breakeven.is_some();
     let entry = conds(entry_obj.get("all"), &format!("{pre}entry.all"), false, false, b)?;
     let exit = conds(exit_obj.get("any"), &format!("{pre}exit.any"), true, price_exit, b)?;
     if entry.len() + exit.len() > MAX_CONDITIONS {
@@ -233,6 +283,9 @@
     }
     Ok((entry, exit))
 }
+
+/// `v` without one key (`trailing.activate`, `take_profits[k].size` sit next to a distance).
+fn without(v: &Json, key: &str) -> Json { match v { Json::Obj(kv) => Json::Obj(kv.iter().filter(|(k, _)| k != key).cloned().collect()), _ => v.clone() } }
 
 fn fields(v: &Json, at: &str, allowed: &[&str], reserved: &[&str]) -> Result<(), LabError> {
     let Json::Obj(kv) = v else { return Err(inv(format!("{at} must be an object"))) };
@@ -530,13 +583,23 @@
 fn exec_of<'a>(s: &'a Spec, path: &str) -> Option<&'a Exec> { if path.starts_with("short_side.") { s.short_side.as_ref().map(|x| &x.exec) } else { Some(&s.exec) } }
 fn exec_mut<'a>(s: &'a mut Spec, path: &str) -> Option<&'a mut Exec> { if path.starts_with("short_side.") { s.short_side.as_mut().map(|x| &mut x.exec) } else { Some(&mut s.exec) } }
 
+/// The price-order slot a path (without `short_side.`) addresses: (slot, slot path, field, largest pct).
+fn slot_at(rest: &str) -> Option<(Slot, String, String, f64)> {
+    if let Some(r) = rest.strip_prefix("exit.take_profits[") {
+        let (k, f) = r.split_once("].")?;
+        let k: usize = k.parse().ok()?;
+        return Some((Slot::TakeProfit(k), format!("exit.take_profits[{k}]"), f.into(), 1000.));
+    }
+    [(Slot::Activate, "exit.trailing.activate", 1000.), (Slot::BreakevenAfter, "exit.breakeven.after", 1000.)].into_iter().chain(SLOTS)
+        .find_map(|(s, at, hi)| rest.strip_prefix(&format!("{at}.")).filter(|f| !f.starts_with("at.")).map(|f| (s, at.to_string(), f.to_string(), hi)))
+}
+
 fn target(spec: &Spec, path: &str) -> Result<Target, String> {
     let rest = path.strip_prefix("short_side.").unwrap_or(path);
-    if let Some((slot, at, _)) = SLOTS.iter().find(|(_, at, _)| rest.starts_with(&format!("{at}.")) && !rest.starts_with(&format!("{at}.at."))) {
-        let field = &rest[at.len() + 1..];
+    if let Some((slot, at, field, _)) = slot_at(rest) {
         let exec = exec_of(spec, path).ok_or("the spec has no short_side")?;
-        return match (exec.slot(*slot), field) {
-            (Some(Dist::Pct(_)), "pct") | (Some(Dist::Atr { .. }), "atr") | (Some(Dist::Points(_)), "points") => Ok(Target { path: path.into(), slot: Some(*slot) }),
+        return match (exec.slot(slot), field.as_str()) {
+            (Some(Dist::Pct(_)), "pct") | (Some(Dist::Atr { .. }), "atr") | (Some(Dist::Points(_)), "points") => Ok(Target { path: path.into(), slot: Some(slot) }),
             (None, _) => Err(format!("the spec has no {at}")),
             (Some(d), _) => Err(format!("{at} is set by {}; optimize that field", match d { Dist::Pct(_) => "pct", Dist::Atr { .. } => "atr", Dist::Points(_) => "points" })),
         };
@@ -631,7 +694,7 @@
 
 fn set(s: &mut Spec, t: &Target, x: f64) {
     if let Some(slot) = t.slot {
-        match exec_mut(s, &t.path).and_then(|e| e.slot_mut(slot).as_mut()) { Some(Dist::Pct(p) | Dist::Points(p)) => *p = x, Some(Dist::Atr { mult, .. }) => *mult = x, None => {} }
+        match exec_mut(s, &t.path).and_then(|e| e.slot_mut(slot)) { Some(Dist::Pct(p) | Dist::Points(p)) => *p = x, Some(Dist::Atr { mult, .. }) => *mult = x, None => {} }
         return;
     }
     walk(s, &mut |p, k| if p == t.path {
@@ -645,10 +708,10 @@
 }
 fn check_value(spec: &Spec, t: &Target, x: f64) -> Result<(), String> {
     if let Some(slot) = t.slot {
-        let (_, at, hi) = SLOTS.iter().find(|(s, _, _)| *s == slot).unwrap();
+        let (_, at, _, hi) = slot_at(t.path.strip_prefix("short_side.").unwrap_or(&t.path)).unwrap();
         let order = matches!(slot, Slot::Limit | Slot::Stop);
         let ok = match exec_of(spec, &t.path).and_then(|e| e.slot(slot)) {
-            Some(Dist::Pct(_)) => x <= *hi && (x > 0. || (x == 0. && order)),
+            Some(Dist::Pct(_)) => x <= hi && (x > 0. || (x == 0. && order)),
             Some(Dist::Points(_)) => x <= MAX_POINTS && (x > 0. || (x == 0. && order)),
             _ => x > 0. && x <= MAX_ATR_MULT,
         };
diff --git a/Context/guru/backend/src/lab/series.rs b/Context/guru/backend/src/lab/series.rs
--- a/Context/guru/backend/src/lab/series.rs
+++ b/Context/guru/backend/src/lab/series.rs
@@ -307,7 +307,8 @@
     c.exec = x.clone();
     c.limit_at = x.limit_at.as_ref().map(|o| arr(o, fr, cache)).transpose()?;
     c.stop_at = x.stop_at.as_ref().map(|o| arr(o, fr, cache)).transpose()?;
-    for d in [x.limit, x.stop_loss, x.target, x.trailing, x.stop].into_iter().flatten() {
+    let later = x.breakeven.iter().flat_map(|b| [Some(b.0), b.1]).chain(x.take_profits.iter().map(|t| Some(t.0)));
+    for d in [x.limit, x.stop_loss, x.target, x.trailing, x.stop, x.trail_activate].into_iter().chain(later).flatten() {
         if let Dist::Atr { length, .. } = d {
             if c.atr.contains_key(&length) { continue; }
             let r = SeriesRef { id: "atr".into(), params: [("length".to_string(), length as f64)].into(), tf: Tf::D, out: 0, offset: 0 };
diff --git a/Context/guru/backend/src/lab/simulate.rs b/Context/guru/backend/src/lab/simulate.rs
--- a/Context/guru/backend/src/lab/simulate.rs
+++ b/Context/guru/backend/src/lab/simulate.rs
@@ -10,36 +10,66 @@
 use super::series::Compiled;
 use crate::connors::{Leg, Trade};
 
-struct Pos { e: usize, px: f64, kind: &'static str, stop: f64, target: f64, best: f64, limit_bar: bool }
+struct Pos {
+    e: usize, px: f64, kind: &'static str, stop: f64, target: f64, best: f64, limit_bar: bool,
+    /// trailing arms once the best price reaches `arm` (NaN: always armed); breakeven moves the stop to `be` once the best
+    /// price reaches `be_after` (NaN: no breakeven); take-profits (level, fraction, filled); scale-out pieces filled so far.
+    arm: f64, be_after: f64, be: f64, tps: Vec<(f64, f64, bool)>, out: Vec<(usize, f64, f64)>,
+}
 impl Pos {
     /// Levels are fixed from the entry price and ATR at `sig`, the bar whose close produced the order.
     fn open(c: &Compiled, i: usize, px: f64, sig: usize, kind: &'static str, d: f64) -> Pos {
         let lvl = |x: Option<Dist>, side: f64| x.map_or(f64::NAN, |x| px + side * d * c.dist(x, px, sig));
-        Pos { e: i, px, kind, stop: lvl(c.exec.stop_loss, -1.), target: lvl(c.exec.target, 1.), best: px, limit_bar: kind == "limit" }
-    }
+        let x = &c.exec;
+        Pos { e: i, px, kind, stop: lvl(x.stop_loss, -1.), target: lvl(x.target, 1.), best: px, limit_bar: kind == "limit",
+              arm: lvl(x.trail_activate, 1.), be_after: lvl(x.breakeven.map(|b| b.0), 1.),
+              be: x.breakeven.map_or(f64::NAN, |b| b.1.map_or(px, |p| px + d * c.dist(p, px, sig))),
+              tps: x.take_profits.iter().map(|&(t, size)| (lvl(Some(t), 1.), size, false)).collect(), out: vec![] }
+    }
+    /// The trade, with the scale-out ledger when take-profits filled: pieces plus the final exit, exit_px their weighted average.
     fn trade(&self, x: usize, px: f64, reason: String, signal_i: usize, d: f64) -> Trade {
-        Trade { dir: d, legs: vec![Leg { i: self.e, px: self.px, w: 1. }], exit_i: x, exit_px: px, reason, entry_kind: self.kind, signal_i, open: false, exits: vec![] }
-    }
-    /// Stop-loss / trailing / target exit inside bar i (a close-fill entry is never checked on its own bar).
-    fn price_exit(&self, c: &Compiled, b: &Bars, i: usize, d: f64) -> Option<(f64, String)> {
-        // Trailing level from the best price through the previous bar and ATR at the previous close: known before bar i opens.
-        let trail = c.exec.trailing.map_or(f64::NAN, |t| self.best - d * c.dist(t, self.best, i - 1));
-        let (stop, why) = match (self.stop.is_nan(), trail.is_nan()) {
-            (false, true) => (self.stop, "exit.stop_loss"),
-            (true, false) => (trail, "exit.trailing"),
-            (false, false) if d * (trail - self.stop) > 0. => (trail, "exit.trailing"),
-            (false, false) => (self.stop, "exit.stop_loss"),
-            (true, true) => (f64::NAN, ""),
+        let (exit_px, exits) = if self.out.is_empty() { (px, vec![]) } else {
+            let mut v = self.out.clone();
+            v.push((x, px, 1. - self.out.iter().map(|e| e.2).sum::<f64>()));
+            (v.iter().map(|e| e.1 * e.2).sum(), v)
         };
+        Trade { dir: d, legs: vec![Leg { i: self.e, px: self.px, w: 1. }], exit_i: x, exit_px, reason, entry_kind: self.kind, signal_i, open: false, exits }
+    }
+    /// Stop-loss / breakeven / trailing / take-profits / target inside bar i (a close-fill entry is never checked on its own bar).
+    /// Returns a full exit; partial take-profit fills are added to `out`.
+    fn price_exit(&mut self, c: &Compiled, b: &Bars, i: usize, d: f64) -> Option<(f64, String)> {
+        // Trailing and breakeven levels from the best price through the previous bar and ATR at the previous close: known before bar i opens.
+        let armed = |at: f64| !at.is_nan() && d * (self.best - at) >= 0.;
+        let trail = c.exec.trailing.filter(|_| self.arm.is_nan() || armed(self.arm)).map_or(f64::NAN, |t| self.best - d * c.dist(t, self.best, i - 1));
+        let be = if armed(self.be_after) { self.be } else { f64::NAN };
+        // the protective level is the tightest of stop-loss, trailing and breakeven (ties keep the earlier name)
+        let (stop, why) = [(self.stop, "exit.stop_loss"), (trail, "exit.trailing"), (be, "exit.breakeven")].into_iter()
+            .filter(|(l, _)| !l.is_nan()).fold((f64::NAN, ""), |a, (l, w)| if a.0.is_nan() || d * (l - a.0) > 0. { (l, w) } else { a });
         let (o, adverse, favour) = (b.open[i], if d > 0. { b.low[i] } else { b.high[i] }, if d > 0. { b.high[i] } else { b.low[i] });
         if self.e < i {
             if d * (o - stop) <= 0. { return Some((o, format!("{why} (gap at the open)"))); }
+            self.take(i, |l| d * (o - l) >= 0., Some(o));
             if d * (o - self.target) >= 0. { return Some((o, "exit.target (gap at the open)".into())); }
         }
+        // On a limit-fill bar the high/low may precede the fill, so only the (adverse) stop is assumed reachable.
+        let favour_ok = !(self.limit_bar && self.e == i);
+        // TradingView path: the open's nearer extreme comes first (a tie goes O → L → H → C); default: adverse first.
+        let up_first = b.high[i] - o < o - b.low[i];
+        let favour_first = c.exec.ohlc_path && favour_ok && (up_first == (d > 0.));
+        if favour_first {
+            self.take(i, |l| d * (favour - l) >= 0., None);
+            if d * (favour - self.target) >= 0. { return Some((self.target, "exit.target".into())); }
+        }
         if d * (adverse - stop) <= 0. { return Some((stop, why.into())); }
-        // On a limit-fill bar the high/low may precede the fill, so only the (adverse) stop is assumed reachable.
-        if !(self.limit_bar && self.e == i) && d * (favour - self.target) >= 0. { return Some((self.target, "exit.target".into())); }
+        if favour_ok && !favour_first {
+            self.take(i, |l| d * (favour - l) >= 0., None);
+            if d * (favour - self.target) >= 0. { return Some((self.target, "exit.target".into())); }
+        }
         None
+    }
+    /// Fills every take-profit the bar reaches, at its level (or at `at`, the gapped open).
+    fn take(&mut self, i: usize, reached: impl Fn(f64) -> bool, at: Option<f64>) {
+        for tp in self.tps.iter_mut().filter(|t| !t.2 && reached(t.0)) { tp.2 = true; self.out.push((i, at.unwrap_or(tp.0), tp.1)); }
     }
 }
 
@@ -112,8 +142,8 @@
             }
         }
         let mut exited = false; // after the open: blocks a same-bar order under book rules
-        if let Some(&(s, ref p)) = pos.as_ref() {
-            let d = dir(s);
+        if let Some((s, p)) = pos.as_mut() {
+            let (s, d) = (*s, dir(*s));
             if let Some((px, why)) = p.price_exit(sides[s], b, i, d) {
                 trades.push(pos.take().unwrap().1.trade(i, px, why, i, d));
                 exited = true;
@@ -131,7 +161,8 @@
         let last = i == to;
         if let (Some((s, p)), None) = (&pos, &pending_exit) { // a pending (delayed) exit is not re-placed
             let s = *s;
-            if let Some(r) = sides[s].exit.iter().find(|r| r.holds(i, Some(p.e))) {
+            // rule exits count from min_bars after the fill (price exits stay active)
+            if let Some(r) = sides[s].exit.iter().find(|r| i - p.e >= sides[s].exec.min_bars && r.holds(i, Some(p.e))) {
                 match x.exit_fill {
                     Fill::Close => { trades.push(pos.take().unwrap().1.trade(i, b.close[i], r.label.clone(), i, dir(s))); exited = true; }
                     Fill::NextOpen if i + delay <= to => pending_exit = Some((i, i + delay, r.label.clone())),
PATCH
```

- [ ] **Step 4: Run all lab tests**

Run: `cd Context/guru/backend && CARGO_BUILD_JOBS=2 cargo test lab:: 2>&1 | grep 'test result'`
Expected: `ok. 321 passed; 0 failed` (276 at HEAD plus this phase's tests so far; a different count means a test is missing or a patch was not fully applied). `lab::pinned::existing_specs_keep_their_exact_trades` and `lab::templates::templates_reproduce_the_book_engine_on_fixtures` must be among the passing tests.

- [ ] **Step 5: Commit (only with the user's explicit permission for git writes)**

With permission: `git add Context/guru/backend/src/lab && git commit -m "feat(lab): trailing activation, breakeven, take-profits, min bars and the intrabar path option"` (no trailer, no push). Without it: stop here and report `git status --short` and `git diff --stat`.

---
### Task 10: Scale-in legs and unit sizing (pct of equity, risk per trade)

Spec §6.1, §7.4. `entry.scale_in {legs, min_bars, price}` adds legs while in a trade when the side's entry fires again (fractions sum to ≤ 1; `better`/`worse` price filters); it cannot be combined with take-profits. In unit mode, `execution.sizing` accepts `pct_equity` (weight p/100) and `risk_pct` (weight min(1, r / stop distance %); no stop level means no trade). `units` and `value` need capital (Task 13). One sizing applies to both sides.

**Files:**
- Modify: `Context/guru/backend/src/lab/rules.rs`
- Modify: `Context/guru/backend/src/lab/series.rs`
- Modify: `Context/guru/backend/src/lab/simulate.rs`

**Interfaces:**
- Consumes: Task 9.
- Produces: `rules::{ScaleIn, LegPrice, Sizing, MAX_LEGS}`, `Exec { scale_in, sizing }`, `Knob::Pct`; `Pos { w, base, adds, last }`, `Pos::add`.

- [ ] **Step 1: Write the failing tests** (apply from the repository root)

```bash
git apply --whitespace=nowarn <<'PATCH'
diff --git a/Context/guru/backend/src/lab/rules.rs b/Context/guru/backend/src/lab/rules.rs
--- a/Context/guru/backend/src/lab/rules.rs
+++ b/Context/guru/backend/src/lab/rules.rs
@@ -1118,4 +1169,30 @@
         assert_eq!((t.exec.trail_activate, t.exec.breakeven.unwrap().0, t.exec.take_profits[0].0, t.exec.trailing), (Some(Dist::Pct(1.)), Dist::Pct(4.), Dist::Pct(6.), Some(Dist::Pct(6.))));
         assert!(err(&opt.replace("exit.take_profits[0].pct", "exit.take_profits[1].pct")).message().contains("the spec has no exit.take_profits[1]"));
     }
-}
+
+    // ---- Phase 7: scale-in and unit sizing (spec §6.1, §7.4)
+    #[test] fn scale_in_and_sizing_parse_and_validate() {
+        let s = parse_spec(&with(r#""all": ["#, r#""scale_in": {"legs": [0.5, 0.3, 0.2], "min_bars": 2, "price": "better"}, "all": ["#)
+            .replace(r#""cost_bps": 12"#, r#""cost_bps": 12, "sizing": {"pct_equity": 50}"#)).unwrap();
+        assert_eq!(s.exec.scale_in, Some(ScaleIn { legs: vec![0.5, 0.3, 0.2], min_bars: 2, price: LegPrice::Better }));
+        assert_eq!(s.exec.sizing, Sizing::PctEquity(50.));
+        for (text, needle) in [
+            (with(r#""all": ["#, r#""scale_in": {"legs": [0.6, 0.6]}, "all": ["#), "legs must be a list of 2 to 5 fractions"),
+            (with(r#""all": ["#, r#""scale_in": {"legs": [1]}, "all": ["#), "legs must be a list of 2 to 5 fractions"),
+            (with(r#""all": ["#, r#""scale_in": {"legs": [0.5, 0.5], "price": "cheap"}, "all": ["#), "price must be"),
+            (with(r#""all": ["#, r#""scale_in": {"legs": [0.5, 0.5]}, "all": ["#).replace(r#""any": ["#, r#""take_profits": [{"pct": 5, "size": 0.5}], "any": ["#), "cannot combine with exit.take_profits"),
+            (with(r#""cost_bps": 12"#, r#""cost_bps": 12, "sizing": {"risk_pct": 1}"#), "risk_pct needs an exit.stop_loss"),
+            (with(r#""cost_bps": 12"#, r#""cost_bps": 12, "sizing": {"units": 10}"#), "need execution.capital"),
+            (with(r#""cost_bps": 12"#, r#""cost_bps": 12, "sizing": {"pct_equity": 150}"#), "pct_equity must be a number above 0 and at most 100"),
+            (with(r#""cost_bps": 12"#, r#""cost_bps": 12, "sizing": {}"#), "exactly one of pct_equity, risk_pct, units or value"),
+        ] {
+            let e = err(&text);
+            assert!(e.message().contains(needle), "{needle:?} not in {:?}", e.message());
+        }
+        let opt = with(r#""cost_bps": 12"#, r#""cost_bps": 12, "sizing": {"pct_equity": 50}"#)
+            .replace(r#"[ {"path":"entry.all[0].lhs.p.length","from":2,"to":6,"step":1} ]"#, r#"[ {"path":"execution.sizing.pct_equity","from":25,"to":100,"step":25} ]"#);
+        let s = parse_spec(&opt).unwrap();
+        assert_eq!((current(&s, &s.optimize[0].target), apply(&s, &s.optimize, &[75.]).exec.sizing), (50., Sizing::PctEquity(75.)));
+        assert!(err(&opt.replace(r#""to":100"#, r#""to":125"#)).message().contains("must be above 0 and at most 100"));
+    }
+}
diff --git a/Context/guru/backend/src/lab/simulate.rs b/Context/guru/backend/src/lab/simulate.rs
--- a/Context/guru/backend/src/lab/simulate.rs
+++ b/Context/guru/backend/src/lab/simulate.rs
@@ -630,4 +662,29 @@
         b.open[3] = 101.; // open nearer the low: O → L → H → C, the stop first in both modes
         assert_eq!(simulate(&with(&[2], &[], Exec { ohlc_path: true, ..exec }), &b, 0, 9)[0].reason, "exit.stop_loss");
     }
+
+    // ---- Phase 7: scale-in legs and unit sizing (spec §6.1, §7.4)
+    use crate::lab::rules::{LegPrice, ScaleIn, Sizing};
+    fn legs(t: &Trade) -> Vec<(usize, f64, f64)> { t.legs.iter().map(|l| (l.i, l.px, l.w)).collect() }
+    #[test] fn scale_in_adds_legs_on_repeated_signals() {
+        let si = |price| Some(ScaleIn { legs: vec![0.5, 0.3, 0.2], min_bars: 1, price });
+        let t = simulate(&with(&[2, 3, 5, 6], &[7], Exec { scale_in: si(LegPrice::Any), ..close_fill() }), &bars(), 0, 9);
+        assert_eq!(legs(&t[0]), vec![(2, 102.5, 0.5), (3, 103.5, 0.3), (5, 105.5, 0.2)]); // the fourth signal finds no leg left
+        assert!(crate::trade_validation::check(&t, 0, 9, 0.0005).is_ok());
+        let t = simulate(&with(&[2, 3, 5], &[7], Exec { scale_in: si(LegPrice::Better), ..close_fill() }), &bars(), 0, 9);
+        assert_eq!(legs(&t[0]).len(), 1); // closes only rise: never a better price for a long
+        let t = simulate(&with(&[2, 4], &[7], Exec { scale_in: si(LegPrice::Any), ..Exec::default() }), &bars(), 0, 9);
+        assert_eq!(legs(&t[0]), vec![(3, 103., 0.5), (5, 105., 0.3)]); // next-open legs
+    }
+    #[test] fn unit_sizing_scales_the_position() {
+        let full = simulate(&with(&[2], &[6], close_fill()), &bars(), 0, 9);
+        let half = simulate(&with(&[2], &[6], Exec { sizing: Sizing::PctEquity(50.), ..close_fill() }), &bars(), 0, 9);
+        assert!((half[0].ret(0.) - full[0].ret(0.) / 2.).abs() < 1e-12);
+        // risk 1% with a 2% stop → half a unit; with no stop level yet (ATR warming up) no trade is opened
+        let risk = simulate(&with(&[2], &[6], Exec { sizing: Sizing::RiskPct(1.), stop_loss: Some(Dist::Pct(2.)), ..close_fill() }), &bars(), 0, 9);
+        assert!((risk[0].legs[0].w - 0.5).abs() < 1e-12);
+        let mut c = with(&[2], &[6], Exec { sizing: Sizing::RiskPct(1.), stop_loss: Some(Dist::Atr { mult: 1., length: 14 }), ..close_fill() });
+        c.atr.insert(14, vec![f64::NAN; 10]);
+        assert!(simulate(&c, &bars(), 0, 9).is_empty());
+    }
 }
PATCH
```

- [ ] **Step 2: Run the tests and confirm the failure**

Run: `cd Context/guru/backend && CARGO_BUILD_JOBS=2 cargo test lab:: 2>&1 | tail -20`
Expected: Compile errors: `ScaleIn`, `LegPrice` and `Sizing` are missing.

- [ ] **Step 3: Implement** (apply from the repository root)

```bash
git apply --whitespace=nowarn <<'PATCH'
diff --git a/Context/guru/backend/src/lab/rules.rs b/Context/guru/backend/src/lab/rules.rs
--- a/Context/guru/backend/src/lab/rules.rs
+++ b/Context/guru/backend/src/lab/rules.rs
@@ -102,6 +102,16 @@
 #[derive(Clone, Copy, Debug, PartialEq, Eq)]
 pub enum Slot { Limit, StopLoss, Target, Trailing, Stop, Activate, BreakevenAfter, TakeProfit(usize) }
 pub const MAX_TAKE_PROFITS: usize = 3;
+pub const MAX_LEGS: usize = 5;
+/// Which price a scale-in leg needs relative to the previous leg: any, better (averaging down a long), or worse (adding to strength).
+#[derive(Clone, Copy, Debug, PartialEq, Eq)]
+pub enum LegPrice { Any, Better, Worse }
+/// Extra legs while in a trade (AmiBroker sigScaleIn, Pine pyramiding): `legs[0]` is the first entry; fractions of one position.
+#[derive(Clone, Debug, PartialEq)]
+pub struct ScaleIn { pub legs: Vec<f64>, pub min_bars: usize, pub price: LegPrice }
+/// Position size (spec §7.4). Unit mode accepts Full (one unit), PctEquity and RiskPct; Units and Value need execution.capital.
+#[derive(Clone, Copy, Debug, PartialEq)]
+pub enum Sizing { Full, PctEquity(f64), RiskPct(f64), Units(f64), Value(f64) }
 pub const MAX_VALID_BARS: usize = 20;
 pub const MAX_DELAY: usize = 10;
 pub const MAX_POINTS: f64 = 1e9;
@@ -117,12 +127,13 @@
     /// Phase 7 exits (spec §6.1): trailing activation, breakeven (after, plus), partial take-profits (level, fraction of the
     /// initial position), the bars before rule exits count, and the TradingView intrabar path instead of stop-first.
     pub trail_activate: Option<Dist>, pub breakeven: Option<(Dist, Option<Dist>)>, pub take_profits: Vec<(Dist, f64)>, pub min_bars: usize, pub ohlc_path: bool,
+    pub scale_in: Option<ScaleIn>, pub sizing: Sizing,
 }
 impl Default for Exec {
     fn default() -> Self {
         Exec { short: false, fill: Fill::NextOpen, exit_fill: Fill::NextOpen, limit: None, stop_loss: None, target: None, trailing: None, opposite: Opposite::Reverse,
                stop: None, limit_at: None, stop_at: None, valid_bars: 1, delay: 1, edge: false,
-               trail_activate: None, breakeven: None, take_profits: vec![], min_bars: 0, ohlc_path: false }
+               trail_activate: None, breakeven: None, take_profits: vec![], min_bars: 0, ohlc_path: false, scale_in: None, sizing: Sizing::Full }
     }
 }
 impl Exec {
@@ -189,7 +200,22 @@
     };
     let mut cost_bps = None;
     if let Some(ex) = root.get("execution") {
-        fields(ex, "execution", &["fill", "exit_fill", "cost_bps", "opposite", "delay_bars", "entry_on", "intrabar"], &["limit", "stop"])?;
+        fields(ex, "execution", &["fill", "exit_fill", "cost_bps", "opposite", "delay_bars", "entry_on", "intrabar", "sizing"], &["limit", "stop"])?;
+        if let Some(z) = ex.get("sizing") {
+            fields(z, "execution.sizing", &["pct_equity", "risk_pct", "units", "value"], &[])?;
+            let pct = |v: &Json, k: &str| v.as_f64().filter(|p| *p > 0. && *p <= 100.).ok_or_else(|| inv(format!("execution.sizing.{k} must be a number above 0 and at most 100")));
+            exec.sizing = match (z.get("pct_equity"), z.get("risk_pct"), z.get("units"), z.get("value")) {
+                (Some(p), None, None, None) => Sizing::PctEquity(pct(p, "pct_equity")?),
+                (None, Some(r), None, None) => {
+                    if exec.stop_loss.is_none() || short_side.as_ref().is_some_and(|s| s.exec.stop_loss.is_none()) {
+                        return Err(inv("execution.sizing.risk_pct needs an exit.stop_loss on every side (the risk is the stop distance)"));
+                    }
+                    Sizing::RiskPct(pct(r, "risk_pct")?)
+                }
+                (None, None, Some(_), None) | (None, None, None, Some(_)) => return Err(inv("execution.sizing units and value need execution.capital (money mode)")),
+                _ => return Err(inv("execution.sizing needs exactly one of pct_equity, risk_pct, units or value")),
+            };
+        }
         exec.ohlc_path = match ex.get("intrabar").map(Json::as_str) {
             None | Some(Some("stop_first")) => false,
             Some(Some("ohlc_path")) => true,
@@ -233,7 +259,8 @@
 /// One side's rule lists and price orders; `pre` is "" (the long side, or the only side) or "short_side.".
 fn side(entry: Option<&Json>, exit: Option<&Json>, pre: &str, exec: &mut Exec, b: &mut Budget) -> Result<(Vec<Node>, Vec<Node>), LabError> {
     let entry_obj = entry.ok_or_else(|| inv(format!("{pre}entry is required")))?;
-    fields(entry_obj, &format!("{pre}entry"), &["all", "limit", "stop", "valid_bars"], &["scale_in"])?;
+    fields(entry_obj, &format!("{pre}entry"), &["all", "limit", "stop", "valid_bars", "scale_in"], &[])?;
+    if let Some(v) = entry_obj.get("scale_in") { exec.scale_in = Some(scale_in(v, &format!("{pre}entry.scale_in"))?); }
     let exit_obj = exit.ok_or_else(|| inv(format!("{pre}exit is required")))?;
     fields(exit_obj, &format!("{pre}exit"), &["any", "stop_loss", "target", "trailing", "breakeven", "take_profits", "min_bars"], &[])?;
     for (slot, at, hi) in SLOTS {
@@ -275,6 +302,9 @@
         if exec.take_profits.iter().map(|t| t.1).sum::<f64>() >= 1. { return Err(inv(format!("{at} sizes must add up to less than 1; the rest exits by the other rules"))); }
     }
     if let Some(n) = exit_obj.get("min_bars") { exec.min_bars = whole(Some(n), &format!("{pre}exit.min_bars"), 0)?; }
+    if exec.scale_in.is_some() && !exec.take_profits.is_empty() {
+        return Err(inv(format!("{pre}entry.scale_in cannot combine with {pre}exit.take_profits (a trade's scale-out fractions are of one position)")));
+    }
     let price_exit = exec.stop_loss.is_some() || exec.target.is_some() || exec.trailing.is_some() || exec.breakeven.is_some();
     let entry = conds(entry_obj.get("all"), &format!("{pre}entry.all"), false, false, b)?;
     let exit = conds(exit_obj.get("any"), &format!("{pre}exit.any"), true, price_exit, b)?;
@@ -282,6 +312,20 @@
         return Err(inv(format!("{}{} conditions; the limit is {MAX_CONDITIONS}", if pre.is_empty() { "" } else { "short_side: " }, entry.len() + exit.len())));
     }
     Ok((entry, exit))
+}
+
+fn scale_in(v: &Json, at: &str) -> Result<ScaleIn, LabError> {
+    fields(v, at, &["legs", "min_bars", "price"], &[])?;
+    let bad = || inv(format!("{at}.legs must be a list of 2 to {MAX_LEGS} fractions above 0 that add up to at most 1"));
+    let Some(Json::Arr(ls)) = v.get("legs") else { return Err(bad()) };
+    let legs = ls.iter().map(|l| l.as_f64().filter(|w| *w > 0. && *w <= 1.)).collect::<Option<Vec<f64>>>().ok_or_else(bad)?;
+    if !(2..=MAX_LEGS).contains(&legs.len()) || legs.iter().sum::<f64>() > 1. + 1e-9 { return Err(bad()); }
+    let min_bars = match v.get("min_bars") { None => 1, Some(n) => whole(Some(n), &format!("{at}.min_bars"), 1)? };
+    let price = match v.get("price").map(Json::as_str) {
+        None | Some(Some("any")) => LegPrice::Any, Some(Some("better")) => LegPrice::Better, Some(Some("worse")) => LegPrice::Worse,
+        _ => return Err(inv(format!("{at}.price must be \"any\", \"better\" or \"worse\""))),
+    };
+    Ok(ScaleIn { legs, min_bars, price })
 }
 
 /// `v` without one key (`trailing.activate`, `take_profits[k].size` sit next to a distance).
@@ -604,7 +648,7 @@
             (Some(d), _) => Err(format!("{at} is set by {}; optimize that field", match d { Dist::Pct(_) => "pct", Dist::Atr { .. } => "atr", Dist::Points(_) => "points" })),
         };
     }
-    if !["entry.all[", "exit.any[", "entry.limit.at.", "entry.stop.at."].iter().any(|p| rest.starts_with(p)) {
+    if !["entry.all[", "exit.any[", "entry.limit.at.", "entry.stop.at.", "entry.scale_in.", "execution.sizing."].iter().any(|p| rest.starts_with(p)) {
         return Err("must start with entry.all[i], exit.any[i], entry.limit, exit.stop_loss, exit.target or exit.trailing".into());
     }
     let (mut found, mut near) = (false, None::<String>);
@@ -627,12 +671,17 @@
 }
 
 /// A number an optimize path can reach inside a rule list; `Series` is reported for error messages only.
-enum Knob<'a> { Value(&'a mut f64), Param(&'a mut SeriesRef, String), Whole(&'a mut usize), Series(&'a SeriesRef) }
+enum Knob<'a> { Value(&'a mut f64), Param(&'a mut SeriesRef, String), Whole(&'a mut usize), Series(&'a SeriesRef), Pct(&'a mut f64) }
 /// Visits every optimizable number in the rule lists with its spec path (`entry.all[0].lhs.p.length`, `exit.any[1].rhs.value`, `….bars`).
 fn walk(s: &mut Spec, f: &mut dyn FnMut(String, Knob)) {
     for (k, n) in s.entry.iter_mut().enumerate() { walk_node(n, &format!("entry.all[{k}]"), f); }
     for (k, n) in s.exit.iter_mut().enumerate() { walk_node(n, &format!("exit.any[{k}]"), f); }
-    for (pre, x) in [("", &mut s.exec)] { walk_levels(x, pre, f); }
+    walk_levels(&mut s.exec, "", f);
+    match &mut s.exec.sizing {
+        Sizing::PctEquity(p) => f("execution.sizing.pct_equity".into(), Knob::Pct(p)),
+        Sizing::RiskPct(p) => f("execution.sizing.risk_pct".into(), Knob::Pct(p)),
+        _ => {}
+    }
     if let Some(x) = s.short_side.as_mut() {
         for (k, n) in x.entry.iter_mut().enumerate() { walk_node(n, &format!("short_side.entry.all[{k}]"), f); }
         for (k, n) in x.exit.iter_mut().enumerate() { walk_node(n, &format!("short_side.exit.any[{k}]"), f); }
@@ -640,6 +689,7 @@
     }
 }
 fn walk_levels(x: &mut Exec, pre: &str, f: &mut dyn FnMut(String, Knob)) {
+    if let Some(si) = x.scale_in.as_mut() { f(format!("{pre}entry.scale_in.min_bars"), Knob::Whole(&mut si.min_bars)); }
     if let Some(o) = x.limit_at.as_mut() { walk_operand(o, &format!("{pre}entry.limit.at"), f); }
     if let Some(o) = x.stop_at.as_mut() { walk_operand(o, &format!("{pre}entry.stop.at"), f); }
 }
@@ -699,7 +749,7 @@
     }
     walk(s, &mut |p, k| if p == t.path {
         match k {
-            Knob::Value(v) => *v = x,
+            Knob::Value(v) | Knob::Pct(v) => *v = x,
             Knob::Param(r, key) => { r.params.insert(key, x); }
             Knob::Whole(n) => *n = x as usize,
             Knob::Series(_) => {}
@@ -724,6 +774,7 @@
         res = match k {
             Knob::Param(r, _) => catalogue::get(&r.id).ok_or_else(|| "unknown series".to_string()).and_then(|d| catalogue::resolve_params(&d, &r.params).map(|_| ())),
             Knob::Whole(_) if x.fract() != 0. || x < 1. || x > MAX_OFFSET as f64 => Err(format!("{} must be a whole number from 1 to {MAX_OFFSET}, not {x}", t.path)),
+            Knob::Pct(_) if !(x > 0. && x <= 100.) => Err(format!("{} must be above 0 and at most 100, not {x}", t.path)),
             _ => Ok(()),
         };
     });
@@ -737,7 +788,7 @@
     let mut v = f64::NAN;
     walk(&mut spec.clone(), &mut |p, k| if p == t.path {
         match k {
-            Knob::Value(x) => v = *x,
+            Knob::Value(x) | Knob::Pct(x) => v = *x,
             Knob::Param(r, key) => v = r.params.get(&key).copied().unwrap_or(f64::NAN),
             Knob::Whole(n) => v = *n as f64,
             Knob::Series(_) => {}
diff --git a/Context/guru/backend/src/lab/series.rs b/Context/guru/backend/src/lab/series.rs
--- a/Context/guru/backend/src/lab/series.rs
+++ b/Context/guru/backend/src/lab/series.rs
@@ -299,7 +299,11 @@
 }
 pub fn compile(spec: &Spec, fr: &Frames, cache: &mut Cache) -> Result<Compiled, LabError> {
     let mut c = side(&spec.entry, &spec.exit, &spec.exec, "", fr, cache)?;
-    if let Some(s) = &spec.short_side { c.other = Some(Box::new(side(&s.entry, &s.exit, &s.exec, "short_side.", fr, cache)?)); }
+    if let Some(s) = &spec.short_side {
+        let mut o = side(&s.entry, &s.exit, &s.exec, "short_side.", fr, cache)?;
+        o.exec.sizing = spec.exec.sizing; // one sizing for both sides (optimize changes the spec's)
+        c.other = Some(Box::new(o));
+    }
     Ok(c)
 }
 fn side(entry: &[Node], exit: &[Node], x: &Exec, pre: &str, fr: &Frames, cache: &mut Cache) -> Result<Compiled, LabError> {
diff --git a/Context/guru/backend/src/lab/simulate.rs b/Context/guru/backend/src/lab/simulate.rs
--- a/Context/guru/backend/src/lab/simulate.rs
+++ b/Context/guru/backend/src/lab/simulate.rs
@@ -6,7 +6,7 @@
 //! (3) at the close, exit conditions (any) fill at this close or set a next-open exit, then entry conditions (all)
 //! fill at this close, set a next-open market order or place a limit for the next bar.
 use super::bars::Bars;
-use super::rules::{Dist, Fill, Opposite};
+use super::rules::{Dist, Fill, LegPrice, Opposite, Sizing};
 use super::series::Compiled;
 use crate::connors::{Leg, Trade};
 
@@ -15,13 +15,23 @@
     /// trailing arms once the best price reaches `arm` (NaN: always armed); breakeven moves the stop to `be` once the best
     /// price reaches `be_after` (NaN: no breakeven); take-profits (level, fraction, filled); scale-out pieces filled so far.
     arm: f64, be_after: f64, be: f64, tps: Vec<(f64, f64, bool)>, out: Vec<(usize, f64, f64)>,
+    /// first leg's weight, the full-position weight `base` scale-in legs multiply, later legs, and the last leg's bar and price
+    w: f64, base: f64, adds: Vec<Leg>, last: (usize, f64),
 }
 impl Pos {
     /// Levels are fixed from the entry price and ATR at `sig`, the bar whose close produced the order.
     fn open(c: &Compiled, i: usize, px: f64, sig: usize, kind: &'static str, d: f64) -> Pos {
         let lvl = |x: Option<Dist>, side: f64| x.map_or(f64::NAN, |x| px + side * d * c.dist(x, px, sig));
         let x = &c.exec;
-        Pos { e: i, px, kind, stop: lvl(x.stop_loss, -1.), target: lvl(x.target, 1.), best: px, limit_bar: kind == "limit",
+        let stop = lvl(x.stop_loss, -1.);
+        // unit-mode size: a fraction of one unit (money mode re-sizes in lab/ledger.rs); 0 = cannot size (risk without a stop level)
+        let base = match x.sizing {
+            Sizing::PctEquity(p) => p / 100.,
+            Sizing::RiskPct(r) => { let risk = d * (px - stop) / px * 100.; if risk > 0. { (r / risk).min(1.) } else { 0. } }
+            Sizing::Full | Sizing::Units(_) | Sizing::Value(_) => 1.,
+        };
+        let w = base * x.scale_in.as_ref().map_or(1., |s| s.legs[0]);
+        Pos { e: i, px, kind, stop, target: lvl(x.target, 1.), best: px, limit_bar: kind == "limit", w, base, adds: vec![], last: (i, px),
               arm: lvl(x.trail_activate, 1.), be_after: lvl(x.breakeven.map(|b| b.0), 1.),
               be: x.breakeven.map_or(f64::NAN, |b| b.1.map_or(px, |p| px + d * c.dist(p, px, sig))),
               tps: x.take_profits.iter().map(|&(t, size)| (lvl(Some(t), 1.), size, false)).collect(), out: vec![] }
@@ -33,8 +43,10 @@
             v.push((x, px, 1. - self.out.iter().map(|e| e.2).sum::<f64>()));
             (v.iter().map(|e| e.1 * e.2).sum(), v)
         };
-        Trade { dir: d, legs: vec![Leg { i: self.e, px: self.px, w: 1. }], exit_i: x, exit_px, reason, entry_kind: self.kind, signal_i, open: false, exits }
-    }
+        let legs = std::iter::once(Leg { i: self.e, px: self.px, w: self.w }).chain(self.adds.iter().map(|l| Leg { i: l.i, px: l.px, w: l.w })).collect();
+        Trade { dir: d, legs, exit_i: x, exit_px, reason, entry_kind: self.kind, signal_i, open: false, exits }
+    }
+    fn add(&mut self, i: usize, px: f64, w: f64) { self.adds.push(Leg { i, px, w }); self.last = (i, px); }
     /// Stop-loss / breakeven / trailing / take-profits / target inside bar i (a close-fill entry is never checked on its own bar).
     /// Returns a full exit; partial take-profit fills are added to `out`.
     fn price_exit(&mut self, c: &Compiled, b: &Bars, i: usize, d: f64) -> Option<(f64, String)> {
@@ -121,6 +133,9 @@
     let mut pending_entry: Option<(usize, usize, usize, &'static str)> = None; // (side, signal bar, fill bar, entry kind)
     let mut pending_exit: Option<(usize, usize, String)> = None; // (signal bar, fill bar, reason)
     let mut orders: [Option<Order>; 2] = [None, None]; // one resting order per side
+    let mut pending_add: Option<(usize, f64)> = None; // (fill bar, leg weight)
+    // a position that cannot be sized (risk sizing without a stop level yet) is not opened
+    let sized = |s: usize, p: Pos| (p.w > 0.).then_some((s, p));
     for i in from..=to {
         if pending_exit.as_ref().is_some_and(|e| e.1 == i) {
             let (sig, _, why) = pending_exit.take().unwrap();
@@ -128,7 +143,11 @@
         }
         if pending_entry.is_some_and(|e| e.2 == i) {
             let (s, sig, _, kind) = pending_entry.take().unwrap();
-            if pos.is_none() { pos = Some((s, Pos::open(sides[s], i, b.open[i], sig, kind, dir(s)))); }
+            if pos.is_none() { pos = sized(s, Pos::open(sides[s], i, b.open[i], sig, kind, dir(s))); }
+        }
+        if let Some((_, w)) = pending_add.filter(|a| a.0 == i) {
+            pending_add = None;
+            if let Some((_, p)) = pos.as_mut() { p.add(i, b.open[i], w); }
         }
         if pos.is_none() {
             // resting orders while flat: one fill opens a position (the other side's order keeps resting); two fills on one
@@ -138,7 +157,7 @@
                 orders[o.side] = None;
                 let mut p = Pos::open(sides[o.side], i, *px, o.sig, kind, dir(o.side));
                 p.limit_bar = *limit_bar;
-                pos = Some((o.side, p));
+                pos = sized(o.side, p);
             }
         }
         let mut exited = false; // after the open: blocks a same-bar order under book rules
@@ -152,7 +171,7 @@
                 trades.push(pos.take().unwrap().1.trade(i, px, format!("reverse to {}", if dir(o.side) > 0. { "long" } else { "short" }), i, d));
                 let mut p = Pos::open(sides[o.side], i, px, o.sig, "reverse", dir(o.side));
                 p.limit_bar = limit_bar;
-                pos = Some((o.side, p));
+                pos = sized(o.side, p);
                 orders[o.side] = None;
             }
         }
@@ -176,6 +195,19 @@
         }
         if last { break; } // a signal on the last bar has no fill inside the window
         let want: Vec<usize> = (0..sides.len()).filter(|&s| fires(s, i)).collect();
+        // scale-in: the held side's entry fires again → the next leg (market fill), spaced and price-filtered (spec §6.1)
+        if let (Some((s, p)), None, None) = (pos.as_mut(), &pending_exit, pending_add) {
+            let (s, d) = (*s, dir(*s));
+            if let Some(si) = sides[s].exec.scale_in.as_ref().filter(|si| p.adds.len() + 1 < si.legs.len() && want.contains(&s) && i - p.last.0 >= si.min_bars) {
+                let ok = match si.price { LegPrice::Any => true, LegPrice::Better => d * (b.close[i] - p.last.1) < 0., LegPrice::Worse => d * (b.close[i] - p.last.1) > 0. };
+                let w = p.base * si.legs[p.adds.len() + 1];
+                match (ok, x.fill) {
+                    (false, _) => {}
+                    (true, Fill::Close) => p.add(i, b.close[i], w),
+                    (true, Fill::NextOpen) => if i + delay <= to { pending_add = Some((i + delay, w)); },
+                }
+            }
+        }
         // The other side's entry while a position is still open (direction both): reverse, exit, or ignore.
         if let Some(&(s, _)) = pos.as_ref().filter(|_| pending_exit.is_none()) {
             let o = 1 - s.min(1);
@@ -187,7 +219,7 @@
                     // a reversal is one fill: exit and new entry at the same price, exempt from the same-bar re-entry rule
                     (Opposite::Reverse, Fill::Close, false) => {
                         trades.push(pos.take().unwrap().1.trade(i, b.close[i], why, i, dir(s)));
-                        pos = Some((o, Pos::open(sides[o], i, b.close[i], i, "reverse", dir(o))));
+                        pos = sized(o, Pos::open(sides[o], i, b.close[i], i, "reverse", dir(o)));
                     }
                     (Opposite::Reverse, Fill::NextOpen, false) => if i + delay <= to { pending_exit = Some((i, i + delay, why)); pending_entry = Some((o, i, i + delay, "reverse")); },
                     (Opposite::Exit, ..) => match x.exit_fill {
@@ -207,7 +239,7 @@
             if let ([s], true) = (want.as_slice(), orders.iter().all(Option::is_none) && !want.iter().any(|&s| sides[s].has_order())) {
                 let (s, d, side) = (*s, dir(*s), sides[*s]);
                 match x.fill {
-                    Fill::Close => pos = Some((s, Pos::open(side, i, b.close[i], i, "close", d))),
+                    Fill::Close => pos = sized(s, Pos::open(side, i, b.close[i], i, "close", d)),
                     Fill::NextOpen => if i + delay <= to { pending_entry = Some((s, i, i + delay, "next_open")); },
                 }
             }
PATCH
```

- [ ] **Step 4: Run all lab tests**

Run: `cd Context/guru/backend && CARGO_BUILD_JOBS=2 cargo test lab:: 2>&1 | grep 'test result'`
Expected: `ok. 324 passed; 0 failed` (276 at HEAD plus this phase's tests so far; a different count means a test is missing or a patch was not fully applied). `lab::pinned::existing_specs_keep_their_exact_trades` and `lab::templates::templates_reproduce_the_book_engine_on_fixtures` must be among the passing tests.

- [ ] **Step 5: Commit (only with the user's explicit permission for git writes)**

With permission: `git add Context/guru/backend/src/lab && git commit -m "feat(lab): scale-in legs and unit-mode sizing"` (no trailer, no push). Without it: stop here and report `git status --short` and `git diff --stat`.

---
### Task 11: Re-entry and risk rules — cooldown, max trades per period, loss-streak pause, drawdown stop

Spec §6.1, §6.5. `execution.cooldown {bars, after any|loss|win}`, `execution.max_trades {count, per W|M|Q|Y}` and `execution.risk {max_consecutive_losses + pause_bars, max_drawdown_pct}` read closed trades with `ret(Compiled.cost)`. `simulate_events` returns the events; `simulate` keeps its signature. Blocked entries turn a reversal into an opposite exit.

**Files:**
- Modify: `Context/guru/backend/src/lab/rules.rs`
- Modify: `Context/guru/backend/src/lab/simulate.rs`

**Interfaces:**
- Consumes: Task 10.
- Produces: `rules::{After, Risk, MAX_PAUSE}`, `Exec { cooldown, max_trades, risk }`; `simulate::simulate_events(c, b, from, to) -> (Vec<Trade>, Vec<String>)`.

- [ ] **Step 1: Write the failing tests** (apply from the repository root)

```bash
git apply --whitespace=nowarn <<'PATCH'
diff --git a/Context/guru/backend/src/lab/rules.rs b/Context/guru/backend/src/lab/rules.rs
--- a/Context/guru/backend/src/lab/rules.rs
+++ b/Context/guru/backend/src/lab/rules.rs
@@ -1195,4 +1229,22 @@
         assert_eq!((current(&s, &s.optimize[0].target), apply(&s, &s.optimize, &[75.]).exec.sizing), (50., Sizing::PctEquity(75.)));
         assert!(err(&opt.replace(r#""to":100"#, r#""to":125"#)).message().contains("must be above 0 and at most 100"));
     }
-}
+
+    // ---- Phase 7: re-entry and risk rules (spec §6.1, §6.5)
+    #[test] fn reentry_and_risk_rules_parse_and_validate() {
+        let s = parse_spec(&with(r#""cost_bps": 12"#, r#""cost_bps": 12, "cooldown": {"bars": 3, "after": "loss"}, "max_trades": {"count": 2, "per": "M"}, "risk": {"max_consecutive_losses": 4, "pause_bars": 20, "max_drawdown_pct": 25}"#)).unwrap();
+        assert_eq!((s.exec.cooldown, s.exec.max_trades), (Some((3, After::Loss)), Some((2, 2))));
+        assert_eq!(s.exec.risk, Risk { losses: Some((4, 20)), max_dd_pct: Some(25.) });
+        for (extra, needle) in [
+            (r#""cooldown": {"bars": 0}"#, "execution.cooldown.bars must be a whole number from 1"),
+            (r#""cooldown": {"bars": 2, "after": "draw"}"#, "execution.cooldown.after must be"),
+            (r#""max_trades": {"count": 2, "per": "D"}"#, "execution.max_trades.per must be W, M, Q or Y"),
+            (r#""risk": {"max_consecutive_losses": 3}"#, "pause_bars must be a whole number"),
+            (r#""risk": {"pause_bars": 3}"#, "pause_bars goes with max_consecutive_losses"),
+            (r#""risk": {"max_drawdown_pct": 100}"#, "max_drawdown_pct must be a number above 0 and below 100"),
+        ] {
+            let e = err(&with(r#""cost_bps": 12"#, &format!(r#""cost_bps": 12, {extra}"#)));
+            assert!(e.message().contains(needle), "{needle:?} not in {:?}", e.message());
+        }
+    }
+}
diff --git a/Context/guru/backend/src/lab/simulate.rs b/Context/guru/backend/src/lab/simulate.rs
--- a/Context/guru/backend/src/lab/simulate.rs
+++ b/Context/guru/backend/src/lab/simulate.rs
@@ -687,4 +712,28 @@
         c.atr.insert(14, vec![f64::NAN; 10]);
         assert!(simulate(&c, &bars(), 0, 9).is_empty());
     }
+
+    // ---- Phase 7: cooldown, max trades per period, loss-streak pause, drawdown stop (spec §6.1, §6.5)
+    use crate::lab::rules::{After, Risk};
+    fn losing_shorts(exec: Exec) -> Compiled { with(&[1, 3, 5, 7], &[2, 4, 6, 8], Exec { short: true, fill: Fill::Close, exit_fill: Fill::Close, ..exec }) }
+    fn spans(t: &[Trade]) -> Vec<(usize, usize)> { t.iter().map(|t| (t.legs[0].i, t.exit_i)).collect() }
+    #[test] fn cooldown_waits_after_the_named_exits() {
+        // shorts in a rising market lose every time
+        let t = simulate(&losing_shorts(Exec { cooldown: Some((3, After::Loss)), ..Exec::default() }), &bars(), 0, 9);
+        assert_eq!(spans(&t), vec![(1, 2), (5, 6)]);
+        assert_eq!(spans(&simulate(&losing_shorts(Exec { cooldown: Some((3, After::Win)), ..Exec::default() }), &bars(), 0, 9)).len(), 4);
+    }
+    #[test] fn max_trades_counts_entries_per_calendar_period() {
+        // from_ohlc dates start Monday 2024-01-01: bars 0-6 are week 1, 7-9 week 2
+        let t = simulate(&with(&[1, 3, 8], &[2, 4, 9], Exec { max_trades: Some((1, 1)), ..close_fill() }), &bars(), 0, 9);
+        assert_eq!(spans(&t), vec![(1, 2), (8, 9)]);
+    }
+    #[test] fn a_loss_streak_pauses_and_a_drawdown_stops_new_entries() {
+        let (t, ev) = simulate_events(&losing_shorts(Exec { risk: Risk { losses: Some((2, 3)), max_dd_pct: None }, ..Exec::default() }), &bars(), 0, 9);
+        assert_eq!(spans(&t), vec![(1, 2), (3, 4), (7, 8)]); // paused for bars 4..6 after the second loss
+        assert_eq!(ev.len(), 1);
+        let (t, ev) = simulate_events(&losing_shorts(Exec { risk: Risk { losses: None, max_dd_pct: Some(1.5) }, ..Exec::default() }), &bars(), 0, 9);
+        assert_eq!(spans(&t), vec![(1, 2), (3, 4)]); // two ~1% losses: 1.96% closed-equity drawdown
+        assert!(ev[0].contains("no new entries after it"), "{ev:?}");
+    }
 }
PATCH
```

- [ ] **Step 2: Run the tests and confirm the failure**

Run: `cd Context/guru/backend && CARGO_BUILD_JOBS=2 cargo test lab:: 2>&1 | tail -20`
Expected: Compile errors: `After`, `Risk` and `simulate_events` are missing.

- [ ] **Step 3: Implement** (apply from the repository root)

```bash
git apply --whitespace=nowarn <<'PATCH'
diff --git a/Context/guru/backend/src/lab/rules.rs b/Context/guru/backend/src/lab/rules.rs
--- a/Context/guru/backend/src/lab/rules.rs
+++ b/Context/guru/backend/src/lab/rules.rs
@@ -112,6 +112,13 @@
 /// Position size (spec §7.4). Unit mode accepts Full (one unit), PctEquity and RiskPct; Units and Value need execution.capital.
 #[derive(Clone, Copy, Debug, PartialEq)]
 pub enum Sizing { Full, PctEquity(f64), RiskPct(f64), Units(f64), Value(f64) }
+/// Which exits start a cooldown (AmiBroker ApplyStop ReEntryDelay counts after stops; here after any, losing or winning trades).
+#[derive(Clone, Copy, Debug, PartialEq, Eq)]
+pub enum After { Any, Loss, Win }
+/// Equity-level entry rules (spec §6.5), evaluated on closed trades: pause after n consecutive losses, stop after a drawdown.
+#[derive(Clone, Copy, Debug, Default, PartialEq)]
+pub struct Risk { pub losses: Option<(usize, usize)>, pub max_dd_pct: Option<f64> }
+pub const MAX_PAUSE: usize = 100_000;
 pub const MAX_VALID_BARS: usize = 20;
 pub const MAX_DELAY: usize = 10;
 pub const MAX_POINTS: f64 = 1e9;
@@ -128,12 +135,15 @@
     /// initial position), the bars before rule exits count, and the TradingView intrabar path instead of stop-first.
     pub trail_activate: Option<Dist>, pub breakeven: Option<(Dist, Option<Dist>)>, pub take_profits: Vec<(Dist, f64)>, pub min_bars: usize, pub ohlc_path: bool,
     pub scale_in: Option<ScaleIn>, pub sizing: Sizing,
+    /// re-entry rules: cooldown (bars, after which exits), at most `count` entries per calendar period (anchor 1 W … 4 Y), risk
+    pub cooldown: Option<(usize, After)>, pub max_trades: Option<(usize, usize)>, pub risk: Risk,
 }
 impl Default for Exec {
     fn default() -> Self {
         Exec { short: false, fill: Fill::NextOpen, exit_fill: Fill::NextOpen, limit: None, stop_loss: None, target: None, trailing: None, opposite: Opposite::Reverse,
                stop: None, limit_at: None, stop_at: None, valid_bars: 1, delay: 1, edge: false,
-               trail_activate: None, breakeven: None, take_profits: vec![], min_bars: 0, ohlc_path: false, scale_in: None, sizing: Sizing::Full }
+               trail_activate: None, breakeven: None, take_profits: vec![], min_bars: 0, ohlc_path: false, scale_in: None, sizing: Sizing::Full,
+               cooldown: None, max_trades: None, risk: Risk::default() }
     }
 }
 impl Exec {
@@ -200,7 +210,31 @@
     };
     let mut cost_bps = None;
     if let Some(ex) = root.get("execution") {
-        fields(ex, "execution", &["fill", "exit_fill", "cost_bps", "opposite", "delay_bars", "entry_on", "intrabar", "sizing"], &["limit", "stop"])?;
+        fields(ex, "execution", &["fill", "exit_fill", "cost_bps", "opposite", "delay_bars", "entry_on", "intrabar", "sizing", "cooldown", "max_trades", "risk"], &["limit", "stop"])?;
+        if let Some(v) = ex.get("cooldown") {
+            fields(v, "execution.cooldown", &["bars", "after"], &[])?;
+            let after = match v.get("after").map(Json::as_str) {
+                None | Some(Some("any")) => After::Any, Some(Some("loss")) => After::Loss, Some(Some("win")) => After::Win,
+                _ => return Err(inv("execution.cooldown.after must be \"any\", \"loss\" or \"win\"")),
+            };
+            exec.cooldown = Some((whole(v.get("bars"), "execution.cooldown.bars", 1)?, after));
+        }
+        if let Some(v) = ex.get("max_trades") {
+            fields(v, "execution.max_trades", &["count", "per"], &[])?;
+            let per = match v.get("per").and_then(Json::as_str) { Some("W") => 1, Some("M") => 2, Some("Q") => 3, Some("Y") => 4, _ => return Err(inv("execution.max_trades.per must be W, M, Q or Y")) };
+            exec.max_trades = Some((whole(v.get("count"), "execution.max_trades.count", 1)?, per));
+        }
+        if let Some(v) = ex.get("risk") {
+            fields(v, "execution.risk", &["max_consecutive_losses", "pause_bars", "max_drawdown_pct"], &[])?;
+            if let Some(n) = v.get("max_consecutive_losses") {
+                let pause = v.get("pause_bars").and_then(Json::as_f64).filter(|p| p.fract() == 0. && *p >= 1. && *p <= MAX_PAUSE as f64)
+                    .ok_or_else(|| inv(format!("execution.risk.pause_bars must be a whole number from 1 to {MAX_PAUSE} (with max_consecutive_losses)")))? as usize;
+                exec.risk.losses = Some((whole(Some(n), "execution.risk.max_consecutive_losses", 1)?, pause));
+            } else if v.get("pause_bars").is_some() { return Err(inv("execution.risk.pause_bars goes with max_consecutive_losses")); }
+            if let Some(d) = v.get("max_drawdown_pct") {
+                exec.risk.max_dd_pct = Some(d.as_f64().filter(|d| *d > 0. && *d < 100.).ok_or_else(|| inv("execution.risk.max_drawdown_pct must be a number above 0 and below 100"))?);
+            }
+        }
         if let Some(z) = ex.get("sizing") {
             fields(z, "execution.sizing", &["pct_equity", "risk_pct", "units", "value"], &[])?;
             let pct = |v: &Json, k: &str| v.as_f64().filter(|p| *p > 0. && *p <= 100.).ok_or_else(|| inv(format!("execution.sizing.{k} must be a number above 0 and at most 100")));
diff --git a/Context/guru/backend/src/lab/simulate.rs b/Context/guru/backend/src/lab/simulate.rs
--- a/Context/guru/backend/src/lab/simulate.rs
+++ b/Context/guru/backend/src/lab/simulate.rs
@@ -6,7 +6,8 @@
 //! (3) at the close, exit conditions (any) fill at this close or set a next-open exit, then entry conditions (all)
 //! fill at this close, set a next-open market order or place a limit for the next bar.
 use super::bars::Bars;
-use super::rules::{Dist, Fill, LegPrice, Opposite, Sizing};
+use super::resample::period_key;
+use super::rules::{After, Dist, Fill, LegPrice, Opposite, Sizing};
 use super::series::Compiled;
 use crate::connors::{Leg, Trade};
 
@@ -110,7 +111,9 @@
 }
 
 /// One position at a time over one side (long or short) or, for `direction: both`, two sides (`c` long, `c.other` short).
-pub fn simulate(c: &Compiled, b: &Bars, from: usize, to: usize) -> Vec<Trade> {
+pub fn simulate(c: &Compiled, b: &Bars, from: usize, to: usize) -> Vec<Trade> { simulate_events(c, b, from, to).0 }
+/// The trades plus the equity-level events that changed them (risk pauses and stops, spec §6.5), for `execution_model.events`.
+pub fn simulate_events(c: &Compiled, b: &Bars, from: usize, to: usize) -> (Vec<Trade>, Vec<String>) {
     let sides: Vec<&Compiled> = std::iter::once(c).chain(c.other.as_deref()).collect();
     let dir = |s: usize| if sides[s].exec.short { -1. } else { 1. };
     let x = &c.exec; // fill, exit_fill, opposite, delay and edge are shared by both sides
@@ -136,6 +139,8 @@
     let mut pending_add: Option<(usize, f64)> = None; // (fill bar, leg weight)
     // a position that cannot be sized (risk sizing without a stop level yet) is not opened
     let sized = |s: usize, p: Pos| (p.w > 0.).then_some((s, p));
+    // equity-level state from closed trades (spec §6.5): loss streak, compounded closed equity and its peak, pause, stop
+    let (mut events, mut seen, mut streak, mut eq, mut peak, mut paused_until, mut halted) = (Vec::new(), 0usize, 0usize, 1f64, 1f64, 0usize, false);
     for i in from..=to {
         if pending_exit.as_ref().is_some_and(|e| e.1 == i) {
             let (sig, _, why) = pending_exit.take().unwrap();
@@ -194,6 +199,25 @@
             if !(p.limit_bar && p.e == i) { p.best = if d > 0. { p.best.max(b.high[i]) } else { p.best.min(b.low[i]) }; }
         }
         if last { break; } // a signal on the last bar has no fill inside the window
+        for t in &trades[seen..] {
+            let r = t.ret(c.cost);
+            (eq, streak) = (eq * (1. + r), if r <= 0. { streak + 1 } else { 0 });
+            peak = peak.max(eq);
+            if let Some((n, pause)) = x.risk.losses.filter(|(n, _)| streak >= *n) {
+                (paused_until, streak) = (t.exit_i + pause, 0);
+                events.push(format!("{n} consecutive losing trades by {}: no new entries for {pause} bars", b.date[t.exit_i]));
+            }
+            if let Some(dd) = x.risk.max_dd_pct.filter(|dd| !halted && (eq / peak - 1.) * 100. <= -dd) {
+                halted = true;
+                events.push(format!("closed-trade drawdown reached {dd}% on {}: no new entries after it", b.date[t.exit_i]));
+            }
+        }
+        seen = trades.len();
+        // new entries (and the opening half of a reversal) are blocked; closing is always allowed
+        let in_period = |t: &Trade, anchor: usize| period_key(&b.date[t.legs[0].i], anchor) == period_key(&b.date[i], anchor);
+        let blocked = halted || i < paused_until
+            || x.cooldown.is_some_and(|(bars, after)| trades.last().is_some_and(|t| i - t.exit_i < bars && match after { After::Any => true, After::Loss => t.ret(c.cost) <= 0., After::Win => t.ret(c.cost) > 0. }))
+            || x.max_trades.is_some_and(|(count, anchor)| trades.iter().filter(|t| in_period(t, anchor)).count() + usize::from(pos.as_ref().is_some_and(|(_, p)| period_key(&b.date[p.e], anchor) == period_key(&b.date[i], anchor))) >= count);
         let want: Vec<usize> = (0..sides.len()).filter(|&s| fires(s, i)).collect();
         // scale-in: the held side's entry fires again → the next leg (market fill), spaced and price-filtered (spec §6.1)
         if let (Some((s, p)), None, None) = (pos.as_mut(), &pending_exit, pending_add) {
@@ -212,8 +236,9 @@
         if let Some(&(s, _)) = pos.as_ref().filter(|_| pending_exit.is_none()) {
             let o = 1 - s.min(1);
             if sides.len() == 2 && want.contains(&o) {
-                let why = if x.opposite == Opposite::Reverse { format!("reverse to {}", if dir(o) > 0. { "long" } else { "short" }) } else { "opposite entry".into() };
-                match (x.opposite, x.fill, sides[o].has_order()) {
+                let opposite = if blocked && x.opposite == Opposite::Reverse { Opposite::Exit } else { x.opposite };
+                let why = if opposite == Opposite::Reverse { format!("reverse to {}", if dir(o) > 0. { "long" } else { "short" }) } else { "opposite entry".into() };
+                match (opposite, x.fill, sides[o].has_order()) {
                     (Opposite::Ignore, ..) => {}
                     (Opposite::Reverse, _, true) => { if orders[o].is_none() { orders[o] = place(o, i); } }
                     // a reversal is one fill: exit and new entry at the same price, exempt from the same-bar re-entry rule
@@ -231,7 +256,7 @@
             }
         }
         let flat = if shares_open { pos.is_none() || pending_exit.is_some() } else { pos.is_none() && !exited };
-        let may_enter = flat && pending_entry.is_none();
+        let may_enter = flat && pending_entry.is_none() && !blocked;
         // order sides rest an order each (a channel breakout rests both); market entries need the only signal and no resting
         // order — both sides' market entries on one bar while flat are ambiguous: no entry (spec §6.1)
         if may_enter {
@@ -246,7 +271,7 @@
         }
     }
     if let Some((s, p)) = pos { trades.push(Trade { open: true, ..p.trade(to, b.close[to], "open".into(), to, dir(s)) }); }
-    trades
+    (trades, events)
 }
 
 #[cfg(test)]
PATCH
```

- [ ] **Step 4: Run all lab tests**

Run: `cd Context/guru/backend && CARGO_BUILD_JOBS=2 cargo test lab:: 2>&1 | grep 'test result'`
Expected: `ok. 328 passed; 0 failed` (276 at HEAD plus this phase's tests so far; a different count means a test is missing or a patch was not fully applied). `lab::pinned::existing_specs_keep_their_exact_trades` and `lab::templates::templates_reproduce_the_book_engine_on_fixtures` must be among the passing tests.

- [ ] **Step 5: Commit (only with the user's explicit permission for git writes)**

With permission: `git add Context/guru/backend/src/lab && git commit -m "feat(lab): cooldown, max trades per period and equity-level risk rules"` (no trailer, no push). Without it: stop here and report `git status --short` and `git diff --stat`.

---
### Task 12: Execution model per instrument kind; legs, fills and limits in the API

Spec §7, §9. `model::model(kind, market, shorts, money)` says what a result means for an index (a signal study, not executable), crypto, a CFD, an India cash short (hypothetical in unit mode, refused in money mode) and a US short (borrow not modelled in unit mode). Run and optimize JSON gain `execution_model {kind, mode, executable, notes, events}`. Run trades gain `legs` and `fills` arrays when they have more than one entry. The API sets `Compiled.cost` for the risk rules. The catalogue gains `max_leaves`, `max_nodes`, `node_kinds`, `expressions`, `max_query_bytes` and `execution` options.

**Files:**
- Modify: `Context/guru/backend/src/lab/api.rs`
- Modify: `Context/guru/backend/src/lab/mod.rs`
- Create: `Context/guru/backend/src/lab/model.rs`

**Interfaces:**
- Consumes: Task 11 (`simulate_events`).
- Produces: `lab::model::{Model, model, INDEX, INDIA_SHORT, US_SHORT_UNIT, CRYPTO, CRYPTO_SHORT, CFD}`; private `api::{model_of, backtest -> (Run, events)}`.

> Note: `model.rs` is new; its tests are at the end of the file.

- [ ] **Step 1: Write the failing tests** (apply from the repository root)

```bash
git apply --whitespace=nowarn <<'PATCH'
diff --git a/Context/guru/backend/src/lab/api.rs b/Context/guru/backend/src/lab/api.rs
--- a/Context/guru/backend/src/lab/api.rs
+++ b/Context/guru/backend/src/lab/api.rs
@@ -807,6 +823,21 @@
             crate::lab::json::parse(&body).unwrap();
         }
     }
+
+    // ---- Phase 7: execution model, legs and fills, catalogue limits (spec §7, §9)
+    #[test] fn runs_carry_the_execution_model_and_scale_out_fills() {
+        let st = state(400, "SPX", false);
+        let both = r#"{"v":1,"direction":"both","entry":{"all":[{"lhs":{"s":"close"},"op":"crosses_above","rhs":{"s":"sma","p":{"length":5}}}]},"exit":{"any":[],"stop_loss":{"pct":3},"take_profits":[{"pct":1,"size":0.5}]},
+            "short_side":{"entry":{"all":[{"lhs":{"s":"close"},"op":"crosses_below","rhs":{"s":"sma","p":{"length":5}}}]},"exit":{"any":[{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":3}}]}}}"#;
+        let j = run_json(&st, &q(&[("spec", both)])).unwrap();
+        assert!(j.contains(&format!("\"execution_model\":{{\"kind\":\"index\",\"mode\":\"unit\",\"executable\":false,\"notes\":[\"{}\"],\"events\":[]}}", crate::lab::model::INDEX)), "{}", &j[..400]);
+        assert!(j.contains("\"dir\":-1") && j.contains("\"dir\":1") && j.contains("\"fills\":[["), "both sides trade and take-profits fill");
+        let o = optimize_json(&st, &q(&[])).unwrap();
+        assert!(o.contains("\"execution_model\":{\"kind\":\"index\",\"mode\":\"unit\""));
+        let c = crate::lab::json::parse(&catalogue_json()).unwrap();
+        assert_eq!((c.get("max_leaves").and_then(|v| v.as_f64()), c.get("max_query_bytes").and_then(|v| v.as_f64())), (Some(40.), Some(16384.)));
+        assert!(catalogue_json().contains("\"node_kinds\":[\"all\",\"any\",\"not\",\"at_least\",\"then\"]") && catalogue_json().contains("\"timeframes\":[\"D\",\"W\",\"M\",\"Q\",\"Y\"]"));
+    }
 }
 
 #[cfg(test)]
PATCH
```

- [ ] **Step 2: Run the tests and confirm the failure**

Run: `cd Context/guru/backend && CARGO_BUILD_JOBS=2 cargo test lab:: 2>&1 | tail -20`
Expected: Compile error: `crate::lab::model` is unresolved (the test reads `model::INDEX`).

- [ ] **Step 3: Implement** (apply from the repository root)

```bash
git apply --whitespace=nowarn <<'PATCH'
diff --git a/Context/guru/backend/src/lab/api.rs b/Context/guru/backend/src/lab/api.rs
--- a/Context/guru/backend/src/lab/api.rs
+++ b/Context/guru/backend/src/lab/api.rs
@@ -8,7 +8,7 @@
 use super::rules::{self, LabError, LABEL};
 use super::series::{self, Cache, Compiled, Frames};
 use super::rules::Spec;
-use super::simulate::simulate;
+use super::simulate::{simulate, simulate_events};
 use super::{fidelity, templates};
 use crate::{accounting, connors::{Ctx, StrategyDef}, instrument_policy, trade_validation, State};
 use std::collections::BTreeMap;
@@ -75,8 +75,9 @@
             d.compute.is_some(), quoted(d.outputs.iter().copied()), arr(d.first.iter().map(|&f| if f == usize::MAX { "null".into() } else { f.to_string() })), d.sparse, esc(d.definition), esc(d.note), esc(tv), esc(ab))
     });
     format!(
-        "{{\"label\":\"{LABEL}\",\"price_sources\":{},\"operators\":{},\"timeframes\":[\"D\",\"W\",\"M\",\"Q\",\"Y\"],\"custom_days\":[2,{}],\"expand_modes\":[\"last\",\"first\",\"point\"],\"max_conditions\":{},\"max_spec_bytes\":{},\"max_combos\":{},\"indicators\":{}}}",
-        quoted(rules::PRICE_SOURCES.iter().copied()), quoted(rules::OPERATORS.iter().copied()), super::resample::MAX_DAYS, rules::MAX_CONDITIONS, rules::MAX_SPEC_BYTES, crate::MAX_COMBOS, arr(defs))
+        "{{\"label\":\"{LABEL}\",\"price_sources\":{},\"operators\":{},\"timeframes\":[\"D\",\"W\",\"M\",\"Q\",\"Y\"],\"custom_days\":[2,{}],\"expand_modes\":[\"last\",\"first\",\"point\"],\"max_conditions\":{},\"max_leaves\":{},\"max_nodes\":{},\"node_kinds\":{},\"expressions\":{},\"max_spec_bytes\":{},\"max_query_bytes\":16384,\"execution\":{{\"direction\":[\"long\",\"short\",\"both\"],\"fill\":[\"next_open\",\"close\"],\"opposite\":[\"reverse\",\"exit\",\"ignore\"],\"entry_on\":[\"level\",\"edge\"],\"intrabar\":[\"stop_first\",\"ohlc_path\"],\"sizing\":[\"pct_equity\",\"risk_pct\",\"units\",\"value\"],\"distances\":[\"pct\",\"atr\",\"points\",\"ticks\",\"at\"],\"max_take_profits\":{},\"max_legs\":{},\"max_valid_bars\":{},\"max_delay_bars\":{}}},\"max_combos\":{},\"indicators\":{}}}",
+        quoted(rules::PRICE_SOURCES.iter().copied()), quoted(rules::OPERATORS.iter().copied()), super::resample::MAX_DAYS, rules::MAX_CONDITIONS, rules::MAX_LEAVES, rules::MAX_NODES,
+        quoted(rules::NODE_KINDS.iter().copied()), quoted(rules::EXPRESSIONS.iter().copied()), rules::MAX_SPEC_BYTES, rules::MAX_TAKE_PROFITS, rules::MAX_LEGS, rules::MAX_VALID_BARS, rules::MAX_DELAY, crate::MAX_COMBOS, arr(defs))
 }
 
 /// Instrument identity from the server's own data (never from the query): type, native flag, market, currency.
@@ -172,13 +173,19 @@
     markets::annualise(&mut s, markets::bars_per_year(&ctx.r[0].ticker));
     s
 }
-fn backtest(ctx: &Ctx, c: &Compiled, d: &Bars, from: usize, to: usize, cost: f64) -> Result<crate::Run, LabError> {
-    let trades = simulate(c, d, from, to);
+/// The run plus the simulator's equity-level events (risk pauses and stops).
+fn backtest(ctx: &Ctx, c: &Compiled, d: &Bars, from: usize, to: usize, cost: f64) -> Result<(crate::Run, Vec<String>), LabError> {
+    let (trades, events) = simulate_events(c, d, from, to);
     trade_validation::check(&trades, from, to, cost)?;
     let equity = crate::equity_curve(&ctx.c, &trades, from, to, cost);
     if equity.iter().any(|v| !v.is_finite() || *v <= 0.) { return Err(LabError::Invalid("calculation_unavailable: nonpositive or nonfinite equity; no result is published".into())); }
     let bh = (from..=to).map(|k| ctx.c[k] / ctx.c[from]).collect();
-    Ok(crate::Run { trades, from, to, equity, bh })
+    Ok((crate::Run { trades, from, to, equity, bh }, events))
+}
+/// The execution model of this spec on this instrument (spec §7); refusals are typed needs_data.
+fn model_of(selected: &crate::SelectedContext, ticker: &str, kind: &str, spec: &Spec) -> Result<super::model::Model, LabError> {
+    let market = markets::gl(ticker).map_or_else(|| selected.evidence().map(|e| e.market().code()).or_else(|| crate::preloaded_market(ticker).map(|m| m.code())), |_| None);
+    super::model::model(kind, market, spec.exec.short || spec.short_side.is_some(), false)
 }
 
 pub fn run_json(st: &State, q: &Query) -> Result<String, LabError> {
@@ -198,17 +205,24 @@
     let costs = accounting::Costs::parse(q)?;
     costs.ensure_rollover()?;
     let cost = costs.side_rate();
+    let model = model_of(&selected, &ctx.r[0].ticker, kind, &spec)?;
     let fr = Frames::new(Bars::from_ctx(ctx));
-    let compiled = series::compile(&spec, &fr, &mut Cache::new())?;
-    let run = backtest(ctx, &compiled, &fr.d, from, to, cost)?;
+    let mut compiled = series::compile(&spec, &fr, &mut Cache::new())?;
+    compiled.cost = cost;
+    let (run, events) = backtest(ctx, &compiled, &fr.d, from, to, cost)?;
     let gross = crate::Run { trades: run.trades.clone(), from, to, equity: crate::equity_curve(&ctx.c, &run.trades, from, to, 0.), bh: run.bh.clone() };
+    // scale-in legs and scale-out pieces as [date, price, weight / fraction] (only when a trade has more than one)
+    let pieces = |key: &str, v: Vec<(usize, f64, f64)>| if v.len() < 2 { String::new() } else {
+        format!(",\"{key}\":{}", arr(v.iter().map(|(k, p, w)| format!("[\"{}\",{},{}]", ctx.r[*k].date, crate::num(*p), crate::num(*w)))))
+    };
     let trades = arr(run.trades.iter().map(|t| format!(
-        "{{\"entry\":\"{}\",\"exit\":\"{}\",\"entry_i\":{},\"exit_i\":{},\"entry_px\":{},\"exit_px\":{},\"ret\":{},\"bars\":{},\"open\":{},\"signal\":\"{}\",\"reason\":\"{}\",\"dir\":{},\"entry_kind\":\"{}\"}}",
+        "{{\"entry\":\"{}\",\"exit\":\"{}\",\"entry_i\":{},\"exit_i\":{},\"entry_px\":{},\"exit_px\":{},\"ret\":{},\"bars\":{},\"open\":{},\"signal\":\"{}\",\"reason\":\"{}\",\"dir\":{},\"entry_kind\":\"{}\"{}{}}}",
         ctx.r[t.legs[0].i].date, ctx.r[t.exit_i].date, t.legs[0].i - from, t.exit_i - from, crate::num(t.legs[0].px), crate::num(t.exit_px),
-        crate::num(t.ret(cost)), t.exit_i - t.legs[0].i, t.open, ctx.r[t.signal_i].date, esc(&t.reason), t.dir, t.entry_kind)));
+        crate::num(t.ret(cost)), t.exit_i - t.legs[0].i, t.open, ctx.r[t.signal_i].date, esc(&t.reason), t.dir, t.entry_kind,
+        pieces("legs", t.legs.iter().map(|l| (l.i, l.px, l.w)).collect()), pieces("fills", t.exits.clone()))));
     Ok(format!(
-        "{{\"label\":\"{}\",\"kind\":\"lab_simulation\",\"origin\":{origin},\"instrument\":{instrument},\"from\":\"{}\",\"to\":\"{}\",\"start\":{start},\"dates\":{},\"close\":{},\"equity\":{},\"gross_equity\":{},\"bh\":{},\"trades\":{trades},\"stats\":{},\"gross_stats\":{},\"costs\":{},\"provenance\":{}}}",
-        esc(&label), ctx.r[from].date, ctx.r[to].date, quoted((from..=to).map(|k| ctx.r[k].date.as_str())), arr((from..=to).map(|k| crate::num(ctx.c[k]))),
+        "{{\"label\":\"{}\",\"kind\":\"lab_simulation\",\"origin\":{origin},\"instrument\":{instrument},\"execution_model\":{},\"from\":\"{}\",\"to\":\"{}\",\"start\":{start},\"dates\":{},\"close\":{},\"equity\":{},\"gross_equity\":{},\"bh\":{},\"trades\":{trades},\"stats\":{},\"gross_stats\":{},\"costs\":{},\"provenance\":{}}}",
+        esc(&label), model.json(&events), ctx.r[from].date, ctx.r[to].date, quoted((from..=to).map(|k| ctx.r[k].date.as_str())), arr((from..=to).map(|k| crate::num(ctx.c[k]))),
         arr(run.equity.iter().map(|v| crate::num(*v))), arr(gross.equity.iter().map(|v| crate::num(*v))), arr(run.bh.iter().map(|v| crate::num(*v))),
         crate::stats_json(&lab_stats(ctx, &run, costs)), crate::stats_json(&gross_stats(ctx, &gross)), costs.json(), crate::source_provenance(st, q)))
 }
@@ -236,16 +250,18 @@
         Some(s) => Some(Some(dates.partition_point(|d| *d < s.as_str())).filter(|&k| k > from + 20 && k + 20 < to)
             .ok_or_else(|| LabError::Invalid("split must leave more than 20 bars on each side of the range".into()))?),
     };
+    let model = model_of(&selected, &ctx.r[0].ticker, kind, &spec)?;
     let fr = Frames::new(Bars::from_ctx(ctx));
     let dims = &spec.optimize;
     let combos: usize = dims.iter().map(|d| d.values.len()).product();
     let metrics = |cache: &mut Cache, s: &Spec| -> Result<Vec<f64>, LabError> {
-        let c = series::compile(s, &fr, cache)?;
-        let stats = lab_stats(ctx, &backtest(ctx, &c, &fr.d, from, to, cost)?, costs);
+        let mut c = series::compile(s, &fr, cache)?;
+        c.cost = cost;
+        let stats = lab_stats(ctx, &backtest(ctx, &c, &fr.d, from, to, cost)?.0, costs);
         let mut out: Vec<f64> = crate::FULL_COLS.iter().map(|k| stats[k]).collect();
         let mut ins = Vec::new();
         for (n, (a, b)) in split.map(|s| [(from, s - 1), (s, to)]).into_iter().flatten().enumerate() {
-            let part = lab_stats(ctx, &backtest(ctx, &c, &fr.d, a, b, cost)?, costs);
+            let part = lab_stats(ctx, &backtest(ctx, &c, &fr.d, a, b, cost)?.0, costs);
             out.extend(crate::PART_COLS.iter().map(|k| part[k]));
             if n == 0 { ins = crate::FULL_COLS.iter().map(|k| part[k]).collect(); }
         }
@@ -323,8 +339,8 @@
     if split.is_some() { cols.extend(extra_rank_cols().iter().map(|c| format!("is_{c}"))); }
     let dims_json = arr(dims.iter().map(|d| format!("{{\"path\":\"{}\",\"values\":{}}}", esc(&d.path), arr(d.values.iter().map(|v| crate::num(*v))))));
     Ok(format!(
-        "{{\"label\":\"{}\",\"kind\":\"lab_optimization\",\"origin\":{origin},\"instrument\":{instrument},\"from\":\"{}\",\"to\":\"{}\",\"start\":{start},\"split\":{},\"combos\":{combos},\"threads\":{threads},\"ms\":{},\"dims\":{dims_json},\"cols\":{},\"base\":{},\"ranked_on\":\"{}\",\"rank\":{rank_json},\"picked\":{{{picked}}},\"rows\":[{rows}],\"costs\":{},\"provenance\":{}}}",
-        esc(&label), dates[from], dates[to], split.map_or("null".into(), |s| format!("\"{}\"", dates[s])), t0.elapsed().as_millis(),
+        "{{\"label\":\"{}\",\"kind\":\"lab_optimization\",\"origin\":{origin},\"instrument\":{instrument},\"execution_model\":{},\"from\":\"{}\",\"to\":\"{}\",\"start\":{start},\"split\":{},\"combos\":{combos},\"threads\":{threads},\"ms\":{},\"dims\":{dims_json},\"cols\":{},\"base\":{},\"ranked_on\":\"{}\",\"rank\":{rank_json},\"picked\":{{{picked}}},\"rows\":[{rows}],\"costs\":{},\"provenance\":{}}}",
+        esc(&label), model.json(&[]), dates[from], dates[to], split.map_or("null".into(), |s| format!("\"{}\"", dates[s])), t0.elapsed().as_millis(),
         quoted(cols.iter().map(String::as_str)), arr(base.iter().map(|v| crate::num(*v))), if split.is_some() { "in_sample" } else { "full" }, costs.json(), crate::source_provenance(st, q)))
 }
 /// A spec with `origin` is a customized book template: the template must exist (same book, version not newer).
diff --git a/Context/guru/backend/src/lab/mod.rs b/Context/guru/backend/src/lab/mod.rs
--- a/Context/guru/backend/src/lab/mod.rs
+++ b/Context/guru/backend/src/lab/mod.rs
@@ -13,6 +13,7 @@
 pub mod json;
 pub mod listed;
 pub mod markets;
+pub mod model;
 pub mod ma;
 pub mod ma2;
 pub mod momentum;
diff --git a/Context/guru/backend/src/lab/model.rs b/Context/guru/backend/src/lab/model.rs
new file mode 100644
--- /dev/null
+++ b/Context/guru/backend/src/lab/model.rs
@@ -0,0 +1,72 @@
+//! Execution model per instrument kind (Phase 7 spec §7): what a lab result means for an index, a stock, an ETF, crypto or a
+//! CFD, and which combinations are refused. The kinds come from the server's own instrument identity (api.rs instrument()).
+use super::json::esc;
+use super::rules::LabError;
+
+#[derive(Debug)]
+pub struct Model { pub kind: String, pub mode: &'static str, pub executable: bool, pub notes: Vec<String> }
+
+pub const INDEX: &str = "An index cannot be traded; returns are a study of index levels, not of an ETF or futures contract.";
+pub const INDIA_SHORT: &str = "Hypothetical — overnight short positions are not executable in the Indian cash segment (they need F&O or SLB borrowing).";
+pub const US_SHORT_UNIT: &str = "Short positions: borrow cost is not modelled in unit mode.";
+pub const CRYPTO: &str = "CRYPTO:* is a TradingView index composite, not an exchange order book.";
+pub const CRYPTO_SHORT: &str = "Hypothetical — shorting spot needs a margin borrow or perpetual data with funding, which is not staged.";
+pub const CFD: &str = "Provisional CFD quotes; CFD contract terms (size, financing) are not admitted.";
+
+/// `kind`: index | etf | individual_stock | cfd | crypto; `market`: "in" / "us" (None for gl instruments);
+/// `shorts`: the spec can open short positions; `money`: execution.capital was given.
+pub fn model(kind: &str, market: Option<&str>, shorts: bool, money: bool) -> Result<Model, LabError> {
+    let mut m = Model { kind: kind.into(), mode: if money { "money" } else { "unit" }, executable: true, notes: vec![] };
+    let refuse = |why: &str| Err(LabError::NeedsData(format!("{why}; no money result is published")));
+    match kind {
+        "index" => {
+            if money { return refuse("an index has no tradable quantity: money sizing needs an ETF or a futures contract with its own data (the index is never priced as a stand-in)"); }
+            (m.executable, m.notes) = (false, vec![INDEX.into()]);
+        }
+        "crypto" => {
+            if money { return refuse("the crypto series is a TradingView index composite, not an exchange order book"); }
+            m.executable = false;
+            m.notes.push(CRYPTO.into());
+            if shorts { m.notes.push(CRYPTO_SHORT.into()); }
+        }
+        "cfd" => {
+            if money { return refuse("CFD contract terms are not admitted"); }
+            (m.executable, m.notes) = (false, vec![CFD.into()]);
+        }
+        _ if shorts && market == Some("in") => {
+            if money { return refuse("India cash shorts held overnight need an F&O contract or an SLB borrow model, and neither is admitted"); }
+            (m.executable, m.notes) = (false, vec![INDIA_SHORT.into()]);
+        }
+        _ if shorts && !money => m.notes.push(US_SHORT_UNIT.into()),
+        _ => {}
+    }
+    Ok(m)
+}
+impl Model {
+    pub fn json(&self, events: &[String]) -> String {
+        let list = |v: &[String]| v.iter().map(|s| format!("\"{}\"", esc(s))).collect::<Vec<_>>().join(",");
+        format!("{{\"kind\":\"{}\",\"mode\":\"{}\",\"executable\":{},\"notes\":[{}],\"events\":[{}]}}", esc(&self.kind), self.mode, self.executable, list(&self.notes), list(events))
+    }
+}
+
+#[cfg(test)]
+mod tests {
+    use super::*;
+    #[test] fn each_kind_says_what_its_result_means() {
+        let m = model("index", Some("in"), false, false).unwrap();
+        assert_eq!((m.executable, m.notes[0].as_str()), (false, INDEX));
+        assert_eq!(model("index", Some("us"), false, true).unwrap_err().status(), "needs_data");
+        let m = model("individual_stock", Some("in"), true, false).unwrap();
+        assert_eq!((m.executable, m.notes[0].as_str()), (false, INDIA_SHORT));
+        assert!(model("individual_stock", Some("in"), true, true).unwrap_err().message().contains("F&O contract or an SLB"));
+        assert!(model("individual_stock", Some("in"), false, true).unwrap().executable);
+        assert_eq!(model("etf", Some("us"), true, false).unwrap().notes, vec![US_SHORT_UNIT.to_string()]);
+        assert!(model("etf", Some("us"), true, true).unwrap().notes.is_empty());
+        let m = model("crypto", None, true, false).unwrap();
+        assert_eq!((m.executable, m.notes.len()), (false, 2));
+        assert_eq!(model("crypto", None, false, true).unwrap_err().status(), "needs_data");
+        assert_eq!(model("cfd", None, false, true).unwrap_err().status(), "needs_data");
+        let j = model("index", Some("in"), false, false).unwrap().json(&["e \"1\"".to_string()]);
+        assert!(j.starts_with("{\"kind\":\"index\",\"mode\":\"unit\",\"executable\":false,\"notes\":[\"An index") && j.ends_with("\"events\":[\"e \\\"1\\\"\"]}"), "{j}");
+    }
+}
PATCH
```

- [ ] **Step 4: Run all lab tests**

Run: `cd Context/guru/backend && CARGO_BUILD_JOBS=2 cargo test lab:: 2>&1 | grep 'test result'`
Expected: `ok. 330 passed; 0 failed` (276 at HEAD plus this phase's tests so far; a different count means a test is missing or a patch was not fully applied). `lab::pinned::existing_specs_keep_their_exact_trades` and `lab::templates::templates_reproduce_the_book_engine_on_fixtures` must be among the passing tests.

- [ ] **Step 5: Commit (only with the user's explicit permission for git writes)**

With permission: `git add Context/guru/backend/src/lab && git commit -m "feat(lab): execution model per instrument kind; legs, fills and limits in the API"` (no trailer, no push). Without it: stop here and report `git status --short` and `git diff --stat`.

---
### Task 13: Money mode — capital, whole units, declared costs, effective prices, ledger

Spec §7.1, §7.4. `execution.capital` switches to money mode, with sizing `units`, `value`, `pct_equity` or `risk_pct`, and declared costs (commission per order, per unit or %; slippage in bps; named taxes by side; yearly borrow for shorts). `ledger::book` floors to whole lots, shrinks positions to fit cash, and folds each fill's costs into an effective price, so the book's `equity_curve` and `stats` (cost 0) give the money results unchanged. The API refuses the bps cost/slippage query in money mode, and requires a declared borrow for US shorts. Run JSON gains `money` and `ledger`.

**Files:**
- Modify: `Context/guru/backend/src/lab/api.rs`
- Create: `Context/guru/backend/src/lab/ledger.rs`
- Modify: `Context/guru/backend/src/lab/mod.rs`
- Modify: `Context/guru/backend/src/lab/rules.rs`

**Interfaces:**
- Consumes: Task 12 (`model`, `backtest`).
- Produces: `lab::ledger::{Contract, CASH, Money, Row, Booked, book, BASIS}`; `rules::{MoneyCosts, TaxSide, MAX_CAPITAL, MAX_TAXES}`, `Exec { capital, costs }`, `Sizing::{Units, Value}` accepted with capital.

> Note: `ledger.rs` is new; its tests are at the end of the file.

- [ ] **Step 1: Write the failing tests** (apply from the repository root)

```bash
git apply --whitespace=nowarn <<'PATCH'
diff --git a/Context/guru/backend/src/lab/rules.rs b/Context/guru/backend/src/lab/rules.rs
--- a/Context/guru/backend/src/lab/rules.rs
+++ b/Context/guru/backend/src/lab/rules.rs
@@ -1247,4 +1299,27 @@
             assert!(e.message().contains(needle), "{needle:?} not in {:?}", e.message());
         }
     }
-}
+
+    // ---- Phase 7: money mode (spec §7.4)
+    #[test] fn money_mode_parses_capital_sizing_and_costs() {
+        let base = with(r#""fill": "next_open", "cost_bps": 12"#, r#""fill": "next_open", "capital": 100000, "sizing": {"units": 10},
+            "costs": {"commission": {"per_order": 20, "pct": 0.03}, "slippage": {"bps": 5}, "taxes": [{"name": "STT", "pct": 0.1, "side": "both"}], "borrow_pct_year": 0.5}"#);
+        let s = parse_spec(&base).unwrap();
+        assert_eq!((s.exec.capital, s.exec.sizing), (Some(100000.), Sizing::Units(10.)));
+        assert_eq!(s.exec.costs, MoneyCosts { per_order: 20., pct: 0.03, slip_bps: 5., taxes: vec![("STT".into(), 0.1, TaxSide::Both)], borrow_pct_year: Some(0.5), ..MoneyCosts::default() });
+        for (from, to, needle) in [
+            (r#""capital": 100000"#, r#""capital": 0"#, "execution.capital must be a number above 0"),
+            (r#""capital": 100000, "#, "", "execution.costs applies to money mode (execution.capital)"),
+            (r#""bps": 5"#, r#""bps": 5, "ticks": 1"#, "exactly one of bps or ticks"),
+            (r#""side": "both""#, r#""side": "all""#, "side must be buy, sell or both"),
+            (r#""name": "STT""#, r#""name": "STT; DROP""#, "name must be 1 to 40 letters"),
+            (r#""pct": 0.03"#, r#""pct": 11"#, "commission.pct must be a number from 0 to 10"),
+            (r#""units": 10"#, r#""units": 2.5"#, "units must be a whole number"),
+        ] {
+            let e = err(&base.replace(from, to));
+            assert!(e.message().contains(needle), "{needle:?} not in {:?}", e.message());
+        }
+        assert!(err(&with(r#""cost_bps": 12"#, r#""cost_bps": 12, "capital": 5000"#)).message().contains("remove execution.cost_bps"));
+        assert!(err(&with(r#""cost_bps": 12"#, r#""cost_bps": 12, "costs": {}"#)).message().contains("applies to money mode"));
+    }
+}
diff --git a/Context/guru/backend/src/lab/api.rs b/Context/guru/backend/src/lab/api.rs
--- a/Context/guru/backend/src/lab/api.rs
+++ b/Context/guru/backend/src/lab/api.rs
@@ -838,6 +872,25 @@
         assert_eq!((c.get("max_leaves").and_then(|v| v.as_f64()), c.get("max_query_bytes").and_then(|v| v.as_f64())), (Some(40.), Some(16384.)));
         assert!(catalogue_json().contains("\"node_kinds\":[\"all\",\"any\",\"not\",\"at_least\",\"then\"]") && catalogue_json().contains("\"timeframes\":[\"D\",\"W\",\"M\",\"Q\",\"Y\"]"));
     }
+    #[test] fn money_mode_books_whole_shares_on_an_etf_and_refuses_an_index() {
+        let st = state(400, "SPY", false);
+        let money = r#"{"v":1,"entry":{"all":[{"lhs":{"s":"close"},"op":"crosses_above","rhs":{"s":"sma","p":{"length":5}}}]},"exit":{"any":[{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":3}}]},
+            "execution":{"capital":100000,"sizing":{"units":10},"costs":{"commission":{"per_order":1}}}}"#;
+        let qm = |spec: &str, t: &str| { let mut m = q(&[("spec", spec), ("ticker", t)]); m.remove("cost"); m };
+        let j = run_json(&st, &qm(money, "SPY")).unwrap();
+        let v = crate::lab::json::parse(&j).unwrap();
+        assert!(j.contains("\"execution_model\":{\"kind\":\"etf\",\"mode\":\"money\",\"executable\":true"), "{}", &j[..300]);
+        let Some(crate::lab::json::Json::Arr(rows)) = v.get("ledger") else { panic!("no ledger") };
+        assert!(rows.len() > 5 && rows.iter().all(|r| r.get("qty").and_then(|q| q.as_f64()) == Some(10.)));
+        let pnl: f64 = rows.iter().map(|r| r.get("pnl").unwrap().as_f64().unwrap()).sum();
+        let fin = v.get("money").and_then(|m| m.get("final_equity")).and_then(|f| f.as_f64()).unwrap();
+        assert!((fin - 100000. - pnl).abs() < 1e-3, "final equity = capital + the ledger's P&L");
+        assert!(run_json(&st, &q(&[("spec", money), ("ticker", "SPY")])).unwrap_err().message().contains("remove the cost and slippage query"));
+        let short = money.replace(r#"{"v":1,"#, r#"{"v":1,"direction":"short","#);
+        assert!(run_json(&st, &qm(&short, "SPY")).unwrap_err().message().contains("borrow_pct_year"));
+        let idx = state(400, "SPX", false);
+        assert_eq!(run_json(&idx, &qm(money, "SPX")).unwrap_err().status(), "needs_data");
+    }
 }
 
 #[cfg(test)]
PATCH
```

- [ ] **Step 2: Run the tests and confirm the failure**

Run: `cd Context/guru/backend && CARGO_BUILD_JOBS=2 cargo test lab:: 2>&1 | tail -20`
Expected: `money_mode_parses_capital_sizing_and_costs` fails to compile (`MoneyCosts` missing), and the API money test fails.

- [ ] **Step 3: Implement** (apply from the repository root)

```bash
git apply --whitespace=nowarn <<'PATCH'
diff --git a/Context/guru/backend/src/lab/rules.rs b/Context/guru/backend/src/lab/rules.rs
--- a/Context/guru/backend/src/lab/rules.rs
+++ b/Context/guru/backend/src/lab/rules.rs
@@ -119,6 +119,15 @@
 #[derive(Clone, Copy, Debug, Default, PartialEq)]
 pub struct Risk { pub losses: Option<(usize, usize)>, pub max_dd_pct: Option<f64> }
 pub const MAX_PAUSE: usize = 100_000;
+pub const MAX_CAPITAL: f64 = 1e12;
+pub const MAX_TAXES: usize = 6;
+/// Which side of a trade a declared tax applies to.
+#[derive(Clone, Copy, Debug, PartialEq, Eq)]
+pub enum TaxSide { Buy, Sell, Both }
+/// Money-mode costs (spec §7.4), all user-declared: commission per order / per unit / % of value, slippage in bps or ticks
+/// (ticks: futures only), named taxes as % of value on a side, and the yearly borrow % for shorts.
+#[derive(Clone, Debug, Default, PartialEq)]
+pub struct MoneyCosts { pub per_order: f64, pub per_unit: f64, pub pct: f64, pub slip_bps: f64, pub slip_ticks: f64, pub taxes: Vec<(String, f64, TaxSide)>, pub borrow_pct_year: Option<f64> }
 pub const MAX_VALID_BARS: usize = 20;
 pub const MAX_DELAY: usize = 10;
 pub const MAX_POINTS: f64 = 1e9;
@@ -137,13 +146,15 @@
     pub scale_in: Option<ScaleIn>, pub sizing: Sizing,
     /// re-entry rules: cooldown (bars, after which exits), at most `count` entries per calendar period (anchor 1 W … 4 Y), risk
     pub cooldown: Option<(usize, After)>, pub max_trades: Option<(usize, usize)>, pub risk: Risk,
+    /// money mode (spec §7.4): starting capital and declared costs
+    pub capital: Option<f64>, pub costs: MoneyCosts,
 }
 impl Default for Exec {
     fn default() -> Self {
         Exec { short: false, fill: Fill::NextOpen, exit_fill: Fill::NextOpen, limit: None, stop_loss: None, target: None, trailing: None, opposite: Opposite::Reverse,
                stop: None, limit_at: None, stop_at: None, valid_bars: 1, delay: 1, edge: false,
                trail_activate: None, breakeven: None, take_profits: vec![], min_bars: 0, ohlc_path: false, scale_in: None, sizing: Sizing::Full,
-               cooldown: None, max_trades: None, risk: Risk::default() }
+               cooldown: None, max_trades: None, risk: Risk::default(), capital: None, costs: MoneyCosts::default() }
     }
 }
 impl Exec {
@@ -210,7 +221,15 @@
     };
     let mut cost_bps = None;
     if let Some(ex) = root.get("execution") {
-        fields(ex, "execution", &["fill", "exit_fill", "cost_bps", "opposite", "delay_bars", "entry_on", "intrabar", "sizing", "cooldown", "max_trades", "risk"], &["limit", "stop"])?;
+        fields(ex, "execution", &["fill", "exit_fill", "cost_bps", "opposite", "delay_bars", "entry_on", "intrabar", "sizing", "cooldown", "max_trades", "risk", "capital", "costs"], &["limit", "stop"])?;
+        if let Some(c) = ex.get("capital") {
+            exec.capital = Some(c.as_f64().filter(|c| *c > 0. && *c <= MAX_CAPITAL).ok_or_else(|| inv(format!("execution.capital must be a number above 0 and at most {MAX_CAPITAL}")))?);
+            if ex.get("cost_bps").is_some() { return Err(inv("money mode takes costs from execution.costs; remove execution.cost_bps")); }
+        }
+        if let Some(v) = ex.get("costs") {
+            if exec.capital.is_none() { return Err(inv("execution.costs applies to money mode (execution.capital)")); }
+            exec.costs = money_costs(v)?;
+        }
         if let Some(v) = ex.get("cooldown") {
             fields(v, "execution.cooldown", &["bars", "after"], &[])?;
             let after = match v.get("after").map(Json::as_str) {
@@ -246,6 +265,8 @@
                     }
                     Sizing::RiskPct(pct(r, "risk_pct")?)
                 }
+                (None, None, Some(n), None) if ex.get("capital").is_some() => Sizing::Units(n.as_f64().filter(|n| n.fract() == 0. && *n >= 1. && *n <= 1e9).ok_or_else(|| inv("execution.sizing.units must be a whole number from 1 (shares, or lots for futures)"))?),
+                (None, None, None, Some(v)) if ex.get("capital").is_some() => Sizing::Value(v.as_f64().filter(|v| *v > 0. && *v <= MAX_CAPITAL).ok_or_else(|| inv("execution.sizing.value must be a number above 0"))?),
                 (None, None, Some(_), None) | (None, None, None, Some(_)) => return Err(inv("execution.sizing units and value need execution.capital (money mode)")),
                 _ => return Err(inv("execution.sizing needs exactly one of pct_equity, risk_pct, units or value")),
             };
@@ -346,6 +367,37 @@
         return Err(inv(format!("{}{} conditions; the limit is {MAX_CONDITIONS}", if pre.is_empty() { "" } else { "short_side: " }, entry.len() + exit.len())));
     }
     Ok((entry, exit))
+}
+
+fn money_costs(v: &Json) -> Result<MoneyCosts, LabError> {
+    fields(v, "execution.costs", &["commission", "slippage", "taxes", "borrow_pct_year"], &[])?;
+    let num = |v: Option<&Json>, at: &str, hi: f64| -> Result<f64, LabError> {
+        v.map_or(Ok(0.), |x| x.as_f64().filter(|x| (0. ..=hi).contains(x)).ok_or_else(|| inv(format!("execution.costs.{at} must be a number from 0 to {hi}"))))
+    };
+    let mut c = MoneyCosts::default();
+    if let Some(m) = v.get("commission") {
+        fields(m, "execution.costs.commission", &["per_order", "per_unit", "pct"], &[])?;
+        (c.per_order, c.per_unit, c.pct) = (num(m.get("per_order"), "commission.per_order", 1e9)?, num(m.get("per_unit"), "commission.per_unit", 1e6)?, num(m.get("pct"), "commission.pct", 10.)?);
+    }
+    if let Some(s) = v.get("slippage") {
+        fields(s, "execution.costs.slippage", &["bps", "ticks"], &[])?;
+        if s.get("bps").is_some() == s.get("ticks").is_some() { return Err(inv("execution.costs.slippage needs exactly one of bps or ticks")); }
+        (c.slip_bps, c.slip_ticks) = (num(s.get("bps"), "slippage.bps", 1000.)?, num(s.get("ticks"), "slippage.ticks", 100.)?);
+    }
+    if let Some(t) = v.get("taxes") {
+        let Json::Arr(items) = t else { return Err(inv("execution.costs.taxes must be a list")) };
+        if items.len() > MAX_TAXES { return Err(inv(format!("execution.costs.taxes allows at most {MAX_TAXES} entries"))); }
+        for (k, it) in items.iter().enumerate() {
+            let at = format!("execution.costs.taxes[{k}]");
+            fields(it, &at, &["name", "pct", "side"], &[])?;
+            let name = it.get("name").and_then(Json::as_str).filter(|n| !n.is_empty() && n.len() <= 40 && n.bytes().all(|b| b.is_ascii_alphanumeric() || b" _-".contains(&b)))
+                .ok_or_else(|| inv(format!("{at}.name must be 1 to 40 letters, digits, spaces, _ or -")))?;
+            let side = match it.get("side").and_then(Json::as_str) { Some("buy") => TaxSide::Buy, Some("sell") => TaxSide::Sell, Some("both") => TaxSide::Both, _ => return Err(inv(format!("{at}.side must be buy, sell or both"))) };
+            c.taxes.push((name.into(), num(it.get("pct"), &format!("taxes[{k}].pct"), 10.)?, side));
+        }
+    }
+    if let Some(b) = v.get("borrow_pct_year") { c.borrow_pct_year = Some(num(Some(b), "borrow_pct_year", 100.)?); }
+    Ok(c)
 }
 
 fn scale_in(v: &Json, at: &str) -> Result<ScaleIn, LabError> {
diff --git a/Context/guru/backend/src/lab/api.rs b/Context/guru/backend/src/lab/api.rs
--- a/Context/guru/backend/src/lab/api.rs
+++ b/Context/guru/backend/src/lab/api.rs
@@ -9,7 +9,7 @@
 use super::series::{self, Cache, Compiled, Frames};
 use super::rules::Spec;
 use super::simulate::{simulate, simulate_events};
-use super::{fidelity, templates};
+use super::{fidelity, ledger, templates};
 use crate::{accounting, connors::{Ctx, StrategyDef}, instrument_policy, trade_validation, State};
 use std::collections::BTreeMap;
 use std::net::TcpStream;
@@ -173,19 +173,44 @@
     markets::annualise(&mut s, markets::bars_per_year(&ctx.r[0].ticker));
     s
 }
-/// The run plus the simulator's equity-level events (risk pauses and stops).
-fn backtest(ctx: &Ctx, c: &Compiled, d: &Bars, from: usize, to: usize, cost: f64) -> Result<(crate::Run, Vec<String>), LabError> {
-    let (trades, events) = simulate_events(c, d, from, to);
-    trade_validation::check(&trades, from, to, cost)?;
+/// The run plus the simulator's equity-level events (risk pauses and stops) and, in money mode, the money ledger whose
+/// effective-price trades the run then holds (cost 0: costs are inside the prices).
+fn backtest(ctx: &Ctx, c: &Compiled, d: &Bars, from: usize, to: usize, cost: f64, money: Option<&ledger::Money>) -> Result<(crate::Run, Vec<String>, Option<ledger::Booked>), LabError> {
+    let (real, events) = simulate_events(c, d, from, to);
+    trade_validation::check(&real, from, to, cost)?;
+    let dates: Vec<String> = ctx.r.iter().map(|r| r.date.clone()).collect();
+    let booked = money.map(|m| ledger::book(&real, &dates, m));
+    let trades = booked.as_ref().map_or(real, |b| b.trades());
+    if booked.is_some() { trade_validation::check(&trades, from, to, 0.)?; } // cash: no leverage, so the book's own check applies
     let equity = crate::equity_curve(&ctx.c, &trades, from, to, cost);
     if equity.iter().any(|v| !v.is_finite() || *v <= 0.) { return Err(LabError::Invalid("calculation_unavailable: nonpositive or nonfinite equity; no result is published".into())); }
     let bh = (from..=to).map(|k| ctx.c[k] / ctx.c[from]).collect();
-    Ok((crate::Run { trades, from, to, equity, bh }, events))
+    Ok((crate::Run { trades, from, to, equity, bh }, events, booked))
+}
+/// Money mode on a cash instrument: capital, sizing and the declared costs (futures add their contract in lab/futures.rs).
+fn money_of(spec: &Spec, q: &Query, market: Option<&str>) -> Result<Option<ledger::Money>, LabError> {
+    let Some(capital) = spec.exec.capital else { return Ok(None) };
+    if ["cost", "slippage"].iter().any(|k| q.get(*k).is_some_and(|v| !v.is_empty())) {
+        return Err(LabError::Invalid("money mode takes costs from execution.costs; remove the cost and slippage query".into()));
+    }
+    let shorts = spec.exec.short || spec.short_side.is_some();
+    if shorts && market == Some("us") && spec.exec.costs.borrow_pct_year.is_none() {
+        return Err(LabError::Invalid("declare execution.costs.borrow_pct_year (0 is allowed) for US short positions in money mode".into()));
+    }
+    if spec.exec.costs.slip_ticks > 0. { return Err(LabError::Invalid("slippage in ticks needs a futures contract's tick size; use bps".into())); }
+    Ok(Some(ledger::Money { capital, sizing: spec.exec.sizing, costs: spec.exec.costs.clone(), contract: ledger::CASH }))
+}
+/// The instrument's market code ("in" / "us"; None for gl instruments).
+fn market_of(selected: &crate::SelectedContext, ticker: &str) -> Option<&'static str> {
+    markets::gl(ticker).map_or_else(|| selected.evidence().map(|e| e.market().code()).or_else(|| crate::preloaded_market(ticker).map(|m| m.code())), |_| None)
 }
 /// The execution model of this spec on this instrument (spec §7); refusals are typed needs_data.
 fn model_of(selected: &crate::SelectedContext, ticker: &str, kind: &str, spec: &Spec) -> Result<super::model::Model, LabError> {
-    let market = markets::gl(ticker).map_or_else(|| selected.evidence().map(|e| e.market().code()).or_else(|| crate::preloaded_market(ticker).map(|m| m.code())), |_| None);
-    super::model::model(kind, market, spec.exec.short || spec.short_side.is_some(), false)
+    super::model::model(kind, market_of(selected, ticker), spec.exec.short || spec.short_side.is_some(), spec.exec.capital.is_some())
+}
+fn money_json(b: &ledger::Booked, m: &ledger::Money, currency: Option<&str>) -> String {
+    format!("{{\"capital\":{},\"final_equity\":{},\"costs_total\":{},\"skipped\":{},\"currency\":{},\"basis\":\"{}\"}}", crate::num(m.capital), crate::num(b.final_equity),
+        crate::num(b.costs_total), b.skipped, currency.map_or("null".into(), |c| format!("\"{c}\"")), esc(ledger::BASIS))
 }
 
 pub fn run_json(st: &State, q: &Query) -> Result<String, LabError> {
@@ -206,10 +231,11 @@
     costs.ensure_rollover()?;
     let cost = costs.side_rate();
     let model = model_of(&selected, &ctx.r[0].ticker, kind, &spec)?;
+    let money = money_of(&spec, q, market_of(&selected, &ctx.r[0].ticker))?;
     let fr = Frames::new(Bars::from_ctx(ctx));
     let mut compiled = series::compile(&spec, &fr, &mut Cache::new())?;
     compiled.cost = cost;
-    let (run, events) = backtest(ctx, &compiled, &fr.d, from, to, cost)?;
+    let (run, events, booked) = backtest(ctx, &compiled, &fr.d, from, to, cost, money.as_ref())?;
     let gross = crate::Run { trades: run.trades.clone(), from, to, equity: crate::equity_curve(&ctx.c, &run.trades, from, to, 0.), bh: run.bh.clone() };
     // scale-in legs and scale-out pieces as [date, price, weight / fraction] (only when a trade has more than one)
     let pieces = |key: &str, v: Vec<(usize, f64, f64)>| if v.len() < 2 { String::new() } else {
@@ -220,8 +246,14 @@
         ctx.r[t.legs[0].i].date, ctx.r[t.exit_i].date, t.legs[0].i - from, t.exit_i - from, crate::num(t.legs[0].px), crate::num(t.exit_px),
         crate::num(t.ret(cost)), t.exit_i - t.legs[0].i, t.open, ctx.r[t.signal_i].date, esc(&t.reason), t.dir, t.entry_kind,
         pieces("legs", t.legs.iter().map(|l| (l.i, l.px, l.w)).collect()), pieces("fills", t.exits.clone()))));
+    // money mode: the effective prices stay internal; each trade row reports units, position value and costs separately
+    let money_block = match (&booked, &money) {
+        (Some(b), Some(m)) => format!(",\"money\":{},\"ledger\":{}", money_json(b, m, markets::gl(&ctx.r[0].ticker).map(|g| g.currency).or_else(|| market_of(&selected, &ctx.r[0].ticker).map(|c| if c == "in" { "INR" } else { "USD" }))),
+            arr(b.rows.iter().map(|r| format!("{{\"entry\":\"{}\",\"qty\":{},\"notional\":{},\"costs\":{},\"pnl\":{}}}", ctx.r[r.real.legs[0].i].date, crate::num(r.qty), crate::num(r.notional), crate::num(r.costs), crate::num(r.pnl))))),
+        _ => String::new(),
+    };
     Ok(format!(
-        "{{\"label\":\"{}\",\"kind\":\"lab_simulation\",\"origin\":{origin},\"instrument\":{instrument},\"execution_model\":{},\"from\":\"{}\",\"to\":\"{}\",\"start\":{start},\"dates\":{},\"close\":{},\"equity\":{},\"gross_equity\":{},\"bh\":{},\"trades\":{trades},\"stats\":{},\"gross_stats\":{},\"costs\":{},\"provenance\":{}}}",
+        "{{\"label\":\"{}\",\"kind\":\"lab_simulation\",\"origin\":{origin},\"instrument\":{instrument},\"execution_model\":{},\"from\":\"{}\",\"to\":\"{}\",\"start\":{start},\"dates\":{},\"close\":{},\"equity\":{},\"gross_equity\":{},\"bh\":{},\"trades\":{trades}{money_block},\"stats\":{},\"gross_stats\":{},\"costs\":{},\"provenance\":{}}}",
         esc(&label), model.json(&events), ctx.r[from].date, ctx.r[to].date, quoted((from..=to).map(|k| ctx.r[k].date.as_str())), arr((from..=to).map(|k| crate::num(ctx.c[k]))),
         arr(run.equity.iter().map(|v| crate::num(*v))), arr(gross.equity.iter().map(|v| crate::num(*v))), arr(run.bh.iter().map(|v| crate::num(*v))),
         crate::stats_json(&lab_stats(ctx, &run, costs)), crate::stats_json(&gross_stats(ctx, &gross)), costs.json(), crate::source_provenance(st, q)))
@@ -251,17 +283,19 @@
             .ok_or_else(|| LabError::Invalid("split must leave more than 20 bars on each side of the range".into()))?),
     };
     let model = model_of(&selected, &ctx.r[0].ticker, kind, &spec)?;
+    let money = money_of(&spec, q, market_of(&selected, &ctx.r[0].ticker))?;
     let fr = Frames::new(Bars::from_ctx(ctx));
     let dims = &spec.optimize;
     let combos: usize = dims.iter().map(|d| d.values.len()).product();
     let metrics = |cache: &mut Cache, s: &Spec| -> Result<Vec<f64>, LabError> {
         let mut c = series::compile(s, &fr, cache)?;
         c.cost = cost;
-        let stats = lab_stats(ctx, &backtest(ctx, &c, &fr.d, from, to, cost)?.0, costs);
+        // money mode: optimize values never touch capital or costs, so the spec's money settings apply to every combination
+        let stats = lab_stats(ctx, &backtest(ctx, &c, &fr.d, from, to, cost, money.as_ref())?.0, costs);
         let mut out: Vec<f64> = crate::FULL_COLS.iter().map(|k| stats[k]).collect();
         let mut ins = Vec::new();
         for (n, (a, b)) in split.map(|s| [(from, s - 1), (s, to)]).into_iter().flatten().enumerate() {
-            let part = lab_stats(ctx, &backtest(ctx, &c, &fr.d, a, b, cost)?.0, costs);
+            let part = lab_stats(ctx, &backtest(ctx, &c, &fr.d, a, b, cost, money.as_ref())?.0, costs);
             out.extend(crate::PART_COLS.iter().map(|k| part[k]));
             if n == 0 { ins = crate::FULL_COLS.iter().map(|k| part[k]).collect(); }
         }
diff --git a/Context/guru/backend/src/lab/mod.rs b/Context/guru/backend/src/lab/mod.rs
--- a/Context/guru/backend/src/lab/mod.rs
+++ b/Context/guru/backend/src/lab/mod.rs
@@ -11,6 +11,7 @@
 pub mod catalogue;
 pub mod fidelity;
 pub mod json;
+pub mod ledger;
 pub mod listed;
 pub mod markets;
 pub mod model;
diff --git a/Context/guru/backend/src/lab/ledger.rs b/Context/guru/backend/src/lab/ledger.rs
new file mode 100644
--- /dev/null
+++ b/Context/guru/backend/src/lab/ledger.rs
@@ -0,0 +1,131 @@
+//! Money mode (Phase 7 spec §7.1, §7.4): whole-unit quantities, user-declared costs and effective prices. Each fill's costs
+//! are folded into an effective price, so a unit-allocation `Trade` with weight = position value ÷ equity at the trade's
+//! start has `ret(0)` = money P&L ÷ that equity; the book's equity curve and statistics (read-only) then give money results.
+use super::rules::{MoneyCosts, Sizing, TaxSide};
+use crate::connors::{Leg, Trade};
+
+/// Units per lot, currency per point per unit, tick size (0 = none) and initial margin % (None = cash, no leverage).
+#[derive(Clone, Copy, Debug, PartialEq)]
+pub struct Contract { pub lot: f64, pub point_value: f64, pub tick: f64, pub margin_pct: Option<f64> }
+pub const CASH: Contract = Contract { lot: 1., point_value: 1., tick: 0., margin_pct: None };
+pub struct Money { pub capital: f64, pub sizing: Sizing, pub costs: MoneyCosts, pub contract: Contract }
+/// One booked trade: the simulator's trade (real fills), its effective-price twin, units held and costs paid.
+pub struct Row { pub real: Trade, pub qty: f64, pub notional: f64, pub costs: f64, pub pnl: f64 }
+pub struct Booked { pub rows: Vec<Row>, pub skipped: usize, pub final_equity: f64, pub costs_total: f64 }
+pub const BASIS: &str = "User-declared costs; not a verified statutory schedule. Quantities are computed on the admitted price series, whose corporate-action adjustment is not established; they are not verified as-traded quantities.";
+
+impl Booked {
+    /// The effective-price trades for the book's equity curve and statistics (cost 0: costs are inside the prices).
+    pub fn trades(&self) -> Vec<Trade> { self.rows.iter().map(|r| r.real.clone()).collect() }
+}
+
+/// Costs of one fill of `qty` units at `px`: commission, taxes on the side traded, slippage (market and stop fills only).
+fn fill_cost(c: &MoneyCosts, k: &Contract, px: f64, qty: f64, buy: bool, slips: bool) -> f64 {
+    let value = px * qty * k.point_value;
+    let tax: f64 = c.taxes.iter().filter(|t| matches!((t.2, buy), (TaxSide::Both, _) | (TaxSide::Buy, true) | (TaxSide::Sell, false))).map(|t| t.1 / 100. * value).sum();
+    let slip = if slips { c.slip_bps / 1e4 * value + c.slip_ticks * k.tick * qty * k.point_value } else { 0. };
+    if qty > 0. { c.per_order + c.per_unit * qty + c.pct / 100. * value + tax + slip } else { 0. }
+}
+/// Whole lots, rounded down.
+fn lots(units: f64, lot: f64) -> f64 { if units.is_finite() && units > 0. { (units / lot).floor() * lot } else { 0. } }
+
+/// Books the simulator's trades with money. `dates` are the bars' dates (borrow is charged per calendar day).
+pub fn book(trades: &[Trade], dates: &[String], m: &Money) -> Booked {
+    let (k, c) = (&m.contract, &m.costs);
+    let mut equity = m.capital;
+    let (mut rows, mut skipped, mut costs_total) = (Vec::new(), 0, 0.);
+    for t in trades {
+        let d = t.dir;
+        let buy = d > 0.;
+        // margin per unit (futures) or price per unit (cash) is what a unit of equity buys
+        let basis = |px: f64| px * k.point_value * k.margin_pct.map_or(1., |p| p / 100.);
+        let (mut legs, mut used, mut paid) = (Vec::new(), 0., 0.);
+        for l in &t.legs {
+            let wanted = match m.sizing {
+                Sizing::Units(n) => l.w * n * k.lot,
+                Sizing::Value(v) => l.w * v / basis(l.px),
+                Sizing::Full | Sizing::PctEquity(_) | Sizing::RiskPct(_) => l.w * equity / basis(l.px),
+            };
+            // shrink to what equity still covers (AmiBroker AllowPositionShrinking); no leverage beyond the declared margin
+            let room = (equity - used - paid - c.per_order).max(0.) / (basis(l.px) * (1. + (c.pct + c.slip_bps / 100. + c.taxes.iter().map(|t| t.1).sum::<f64>()) / 100.));
+            let q = lots(wanted.min(room), k.lot);
+            if q <= 0. { continue; }
+            let cost = fill_cost(c, k, l.px, q, buy, !matches!(t.entry_kind, "limit") || legs.len() > 0);
+            used += basis(l.px) * q;
+            paid += cost;
+            legs.push((l.i, l.px, q, cost));
+        }
+        if legs.is_empty() { skipped += 1; continue; }
+        let qty: f64 = legs.iter().map(|l| l.2).sum();
+        // exit pieces: (bar, price, fraction); take-profit and target fills are limit-type (no slippage)
+        let pieces: Vec<(usize, f64, f64)> = if t.exits.is_empty() { vec![(t.exit_i, t.exit_px, 1.)] } else { t.exits.clone() };
+        let last = pieces.len() - 1;
+        let mut exit_cost = 0.;
+        let eff_pieces: Vec<(usize, f64, f64)> = pieces.iter().enumerate().map(|(n, &(x, px, f))| {
+            let q = qty * f;
+            let limit_type = n < last || t.reason.starts_with("exit.target");
+            let mut cost = fill_cost(c, k, px, q, !buy, !limit_type);
+            if n == last && d < 0. { // short borrow, charged on the entry notional per calendar day held
+                let days = (super::resample::day_number(&dates[x]) - super::resample::day_number(&dates[t.legs[0].i])) as f64;
+                cost += c.borrow_pct_year.unwrap_or(0.) / 100. * legs.iter().map(|l| l.1 * l.2 * k.point_value).sum::<f64>() * days / 365.;
+            }
+            exit_cost += cost;
+            (x, px - d * cost / (q * k.point_value), f)
+        }).collect();
+        let exit_eff: f64 = eff_pieces.iter().map(|e| e.1 * e.2).sum();
+        let eff_legs: Vec<Leg> = legs.iter().map(|&(i, px, q, cost)| {
+            let eff = px + d * cost / (q * k.point_value);
+            Leg { i, px: eff, w: q * k.point_value * eff / equity }
+        }).collect();
+        let pnl: f64 = legs.iter().zip(&eff_legs).map(|(l, e)| l.2 * k.point_value * d * (exit_eff - e.px)).sum();
+        let real = Trade { legs: eff_legs, exit_px: exit_eff, exits: if t.exits.is_empty() { vec![] } else { eff_pieces }, ..t.clone() };
+        costs_total += paid + exit_cost;
+        rows.push(Row { real, qty, notional: legs.iter().map(|l| l.1 * l.2 * k.point_value).sum(), costs: paid + exit_cost, pnl });
+        equity += pnl;
+    }
+    Booked { rows, skipped, final_equity: equity, costs_total }
+}
+
+#[cfg(test)]
+mod tests {
+    use super::*;
+    fn trade(dir: f64, legs: &[(usize, f64, f64)], exit: (usize, f64), kind: &'static str) -> Trade {
+        Trade { dir, legs: legs.iter().map(|&(i, px, w)| Leg { i, px, w }).collect(), exit_i: exit.0, exit_px: exit.1, reason: "rule".into(), entry_kind: kind, signal_i: exit.0, open: false, exits: vec![] }
+    }
+    fn dates(n: usize) -> Vec<String> { (0..n).map(|i| crate::lab::resample::civil(crate::lab::resample::day_number("2024-01-01") + i as i64)).collect() }
+    fn money(sizing: Sizing, costs: MoneyCosts) -> Money { Money { capital: 10_000., sizing, costs, contract: CASH } }
+    #[test] fn whole_shares_and_costs_reach_the_book_return_exactly() {
+        let costs = MoneyCosts { per_order: 20., pct: 0.1, taxes: vec![("STT".into(), 0.1, TaxSide::Both)], ..MoneyCosts::default() };
+        let b = book(&[trade(1., &[(1, 99.5, 1.)], (5, 110.), "next_open")], &dates(10), &money(Sizing::PctEquity(100.), costs));
+        let r = &b.rows[0];
+        assert_eq!(r.qty, 100.); // 100.5 wanted; 10,000 covers 100 shares at 99.5 plus 0.2% costs and the 20 order fee
+        let buy = 20. + 0.002 * 99.5 * 100.;
+        let sell = 20. + 0.002 * 110. * 100.;
+        assert!((r.costs - buy - sell).abs() < 1e-9);
+        assert!((r.pnl - (100. * (110. - 99.5) - buy - sell)).abs() < 1e-9);
+        assert!((r.real.ret(0.) * 10_000. - r.pnl).abs() < 1e-6, "the effective-price trade carries the money P&L");
+        assert!((b.final_equity - 10_000. - r.pnl).abs() < 1e-9 && b.costs_total == r.costs);
+    }
+    #[test] fn units_value_shorts_borrow_and_skips() {
+        let units = book(&[trade(1., &[(1, 100., 1.)], (3, 101.), "close")], &dates(10), &money(Sizing::Units(7.), MoneyCosts::default()));
+        assert_eq!((units.rows[0].qty, units.rows[0].pnl), (7., 7.));
+        let value = book(&[trade(1., &[(1, 30., 1.)], (3, 31.), "close")], &dates(10), &money(Sizing::Value(1000.), MoneyCosts::default()));
+        assert_eq!(value.rows[0].qty, 33.);
+        // a short pays borrow on the entry notional per calendar day: 100 × 50 × 10% × 9/365
+        let short = book(&[trade(-1., &[(1, 50., 1.)], (10, 50.), "close")], &dates(12), &money(Sizing::Units(100.), MoneyCosts { borrow_pct_year: Some(10.), ..MoneyCosts::default() }));
+        assert!((short.rows[0].pnl + 5000. * 0.1 * 9. / 365.).abs() < 1e-9);
+        let skip = book(&[trade(1., &[(1, 20_000., 1.)], (3, 21_000.), "close")], &dates(10), &money(Sizing::PctEquity(100.), MoneyCosts::default()));
+        assert_eq!((skip.rows.len(), skip.skipped, skip.final_equity), (0, 1, 10_000.));
+    }
+    #[test] fn limit_fills_pay_no_slippage_and_futures_size_on_margin() {
+        let slip = MoneyCosts { slip_bps: 10., ..MoneyCosts::default() };
+        let market = book(&[trade(1., &[(1, 100., 1.)], (3, 100.), "next_open")], &dates(10), &money(Sizing::Units(10.), slip.clone()));
+        let limit = book(&[trade(1., &[(1, 100., 1.)], (3, 100.), "limit")], &dates(10), &money(Sizing::Units(10.), slip));
+        assert!((market.rows[0].costs - 2. * 0.001 * 1000.).abs() < 1e-9 && (limit.rows[0].costs - 0.001 * 1000.).abs() < 1e-9);
+        // a lot of 50 units at 20,000 with 12% margin needs 120,000 a lot: 1,000,000 × 50% buys 4 lots (200 units)
+        let fut = Money { capital: 1_000_000., sizing: Sizing::PctEquity(50.), costs: MoneyCosts::default(), contract: Contract { lot: 50., point_value: 1., tick: 0.05, margin_pct: Some(12.) } };
+        let b = book(&[trade(1., &[(1, 20_000., 0.5)], (3, 20_100.), "next_open")], &dates(10), &fut);
+        assert_eq!((b.rows[0].qty, b.rows[0].pnl), (200., 20_000.));
+        assert!(b.rows[0].real.legs[0].w > 1., "futures notional exceeds equity (margin leverage)");
+    }
+}
PATCH
```

- [ ] **Step 4: Run all lab tests**

Run: `cd Context/guru/backend && CARGO_BUILD_JOBS=2 cargo test lab:: 2>&1 | grep 'test result'`
Expected: `ok. 335 passed; 0 failed` (276 at HEAD plus this phase's tests so far; a different count means a test is missing or a patch was not fully applied). `lab::pinned::existing_specs_keep_their_exact_trades` and `lab::templates::templates_reproduce_the_book_engine_on_fixtures` must be among the passing tests.

- [ ] **Step 5: Commit (only with the user's explicit permission for git writes)**

With permission: `git add Context/guru/backend/src/lab && git commit -m "feat(lab): money mode with whole units, declared costs and an effective-price ledger"` (no trailer, no push). Without it: stop here and report `git status --short` and `git diff --stat`.

---
### Task 14: Futures contract model — chain, rolls, causal continuous series, actual-price trades, margin calls, ticks, expiry operand; needs_data on real requests

Spec §7.3.

- **Chain and continuous series.** `futures::continuous` stitches a contract chain at the roll (n sessions before expiry; on volume or open interest; or none), with `forward` (default), `ratio` or `none` adjustment. `back` is refused by the parser.
- **Trades on actual prices.** `to_contracts` maps fills to the held contract's actual prices and splits a trade at each roll into two costed trades.
- **`futures::run`.** It books each piece on its own contract's lot, point value and margin, and liquidates at the next open with a `margin_call` event.
- **Expiry operand.** `expiry` is computed from `Bars.expiry`, which only futures runs set; elsewhere it answers `needs_data`.
- **Ticks and rounding.** `ticks` distances are futures-only, and levels round to the tick.
- **Real requests.** Every run or optimize with `instrument.kind: futures` answers `needs_data`, because no contract data is admitted (spec §3.1).

**Files:**
- Modify: `Context/guru/backend/src/lab/api.rs`
- Modify: `Context/guru/backend/src/lab/bars.rs`
- Modify: `Context/guru/backend/src/lab/catalogue.rs`
- Create: `Context/guru/backend/src/lab/futures.rs`
- Modify: `Context/guru/backend/src/lab/mod.rs`
- Modify: `Context/guru/backend/src/lab/rules.rs`
- Modify: `Context/guru/backend/src/lab/series.rs`
- Modify: `Context/guru/backend/src/lab/simulate.rs`
- Modify: `Context/guru/backend/src/lab/sources.rs`

**Interfaces:**
- Consumes: Task 13 (`ledger::book`).
- Produces: `lab::futures::{Contract, Continuous, continuous, to_contracts, FuturesRun, run, DEFS}`; `rules::{Dist::Ticks, Roll, Adjust, Futures, MAX_TICKS}`, `Exec.futures`; `Compiled { tick }`, `Compiled::round`; `Bars.expiry`; master list 301 → 302.

> Note: `futures.rs` is new; its tests are at the end of the file.

> Note: The test patch updates the master-list assertion (302; computed 186; AmiBroker names 26) and makes the generic tests expect `needs_data` from `needs.derivatives` indicators.

- [ ] **Step 1: Write the failing tests** (apply from the repository root)

```bash
git apply --whitespace=nowarn <<'PATCH'
diff --git a/Context/guru/backend/src/lab/rules.rs b/Context/guru/backend/src/lab/rules.rs
--- a/Context/guru/backend/src/lab/rules.rs
+++ b/Context/guru/backend/src/lab/rules.rs
@@ -1322,4 +1389,27 @@
         assert!(err(&with(r#""cost_bps": 12"#, r#""cost_bps": 12, "capital": 5000"#)).message().contains("remove execution.cost_bps"));
         assert!(err(&with(r#""cost_bps": 12"#, r#""cost_bps": 12, "costs": {}"#)).message().contains("applies to money mode"));
     }
-}
+
+    // ---- Phase 7: futures instrument, ticks (spec §7.3)
+    #[test] fn futures_instrument_parses_and_refuses_lookahead_adjustment() {
+        let base = with(r#""cost_bps": 12"#, r#""cost_bps": 12, "instrument": {"kind": "futures", "roll": {"days_before_expiry": 2}, "adjust": "ratio"}"#)
+            .replace(r#""any": ["#, r#""stop_loss": {"ticks": 20}, "any": ["#);
+        let s = parse_spec(&base).unwrap();
+        assert_eq!((s.exec.futures, s.exec.stop_loss), (Some(Futures { roll: Roll::DaysBefore(2), adjust: Adjust::Ratio }), Some(Dist::Ticks(20.))));
+        assert_eq!(parse_spec(&base.replace(r#", "roll": {"days_before_expiry": 2}, "adjust": "ratio""#, "")).unwrap().exec.futures, Some(Futures { roll: Roll::DaysBefore(1), adjust: Adjust::Forward }));
+        let e = err(&base.replace(r#""adjust": "ratio""#, r#""adjust": "back""#));
+        assert_eq!((e.status(), e.message().contains("lookahead")), ("unsupported", true));
+        for (from, to, needle) in [
+            (r#", "instrument": {"kind": "futures", "roll": {"days_before_expiry": 2}, "adjust": "ratio"}"#, "", "ticks need a futures contract"),
+            (r#""roll": {"days_before_expiry": 2}"#, r#""roll": "none""#, "needs execution.exit_fill \"close\""),
+            (r#""roll": {"days_before_expiry": 2}"#, r#""roll": {"on": "price"}"#, "execution.instrument.roll must be"),
+            (r#""kind": "futures""#, r#""kind": "options""#, "kind must be cash or futures"),
+            (r#""kind": "futures", "#, "", "roll and adjust apply to kind futures"),
+            (r#""stop_loss": {"ticks": 20}"#, r#""stop_loss": {"ticks": 2.5}"#, "ticks must be a whole number"),
+            (r#""stop_loss": {"ticks": 20}"#, r#""stop_loss": {"ticks": 20}, "take_profits": [{"pct": 2, "size": 0.5}]"#, "do not combine with scale_in or take_profits"),
+        ] {
+            let e = err(&base.replace(from, to));
+            assert!(e.message().contains(needle), "{needle:?} not in {:?}", e.message());
+        }
+    }
+}
diff --git a/Context/guru/backend/src/lab/series.rs b/Context/guru/backend/src/lab/series.rs
--- a/Context/guru/backend/src/lab/series.rs
+++ b/Context/guru/backend/src/lab/series.rs
@@ -393,7 +398,7 @@
     }
 
     #[test] fn every_computed_indicator_is_reachable_from_a_rule() {
-        for d in all().iter().filter(|d| d.compute.is_some()) {
+        for d in all().iter().filter(|d| d.compute.is_some() && !d.needs.derivatives) {
             let out = d.outputs[d.outputs.len() - 1];
             if let Err(e) = crate::lab::testdata::compiles(d.id, out) { panic!("{}.{out}: {} {}", d.id, e.status(), e.message()); }
         }
@@ -434,7 +439,7 @@
         for d in all().iter().filter(|d| d.compute.is_some() && !crate::lab::periods::USES_SESSION_DATES.contains(&d.id)) {
             for tf in [Tf::D, Tf::W, Tf::M, Tf::Q, Tf::Y, Tf::Days(5)] {
                 let r = sref(d.id, tf, d.outputs.len() - 1, 0);
-                if d.needs.breadth && tf != Tf::D { assert_eq!(series(&full, &mut Cache::new(), &r).unwrap_err().status(), "needs_data"); continue; }
+                if (d.needs.breadth && tf != Tf::D) || d.needs.derivatives { assert_eq!(series(&full, &mut Cache::new(), &r).unwrap_err().status(), "needs_data"); continue; }
                 let want = series(&full, &mut Cache::new(), &r).unwrap();
                 for cut in [97usize, 203, 311] {
                     let pre = crate::lab::testdata::cut(&full_bars, cut);
@@ -479,7 +484,7 @@
                     let params: Params = d.params.iter().map(|p| (p.key.to_string(), pick(p))).collect();
                     for tf in [Tf::D, Tf::W, Tf::M, Tf::Q, Tf::Y, Tf::Days(5)] {
                         let r = SeriesRef { id: d.id.into(), params: params.clone(), tf, out: 0, offset: 0 };
-                        if d.needs.breadth && tf != Tf::D { assert_eq!(series(&fr, &mut Cache::new(), &r).unwrap_err().status(), "needs_data"); continue; }
+                        if (d.needs.breadth && tf != Tf::D) || d.needs.derivatives { assert_eq!(series(&fr, &mut Cache::new(), &r).unwrap_err().status(), "needs_data"); continue; }
                         assert_eq!(series(&fr, &mut Cache::new(), &r).unwrap().len(), fr.d.len(), "{} {tf:?}", d.id);
                     }
                 }
diff --git a/Context/guru/backend/src/lab/api.rs b/Context/guru/backend/src/lab/api.rs
--- a/Context/guru/backend/src/lab/api.rs
+++ b/Context/guru/backend/src/lab/api.rs
@@ -890,6 +897,10 @@
         assert!(run_json(&st, &qm(&short, "SPY")).unwrap_err().message().contains("borrow_pct_year"));
         let idx = state(400, "SPX", false);
         assert_eq!(run_json(&idx, &qm(money, "SPX")).unwrap_err().status(), "needs_data");
+        let fut = money.replace(r#""capital":100000"#, r#""instrument":{"kind":"futures"},"capital":100000"#);
+        let e = run_json(&idx, &qm(&fut, "SPX")).unwrap_err();
+        assert!(e.status() == "needs_data" && e.message().contains("futures contract data for SPX") && e.message().contains("never used as a stand-in"), "{e:?}");
+        assert_eq!(optimize_json(&idx, &qm(&fut.replace(r#""v":1,"#, r#""v":1,"optimize":[{"path":"exit.any[0].rhs.value","from":2,"to":3,"step":1}],"#), "SPX")).unwrap_err().status(), "needs_data");
     }
 }
 
diff --git a/Context/guru/backend/src/lab/catalogue.rs b/Context/guru/backend/src/lab/catalogue.rs
--- a/Context/guru/backend/src/lab/catalogue.rs
+++ b/Context/guru/backend/src/lab/catalogue.rs
@@ -71,7 +71,7 @@
     use super::*;
     use crate::lab::testdata::{close_eq, walk};
     /// Breadth indicators need market breadth on the bars; Task 2 tests them with testdata::with_breadth.
-    fn computable(d: &IndicatorDef) -> bool { d.compute.is_some() && !d.needs.breadth && !crate::lab::periods::USES_SESSION_DATES.contains(&d.id) }
+    fn computable(d: &IndicatorDef) -> bool { d.compute.is_some() && !d.needs.breadth && !d.needs.derivatives && !crate::lab::periods::USES_SESSION_DATES.contains(&d.id) }
     #[test] fn listed_entries_carry_a_note() {
         for d in all() {
             if d.compute.is_some() { assert!(d.note.is_empty(), "{}: computed entries have no note", d.id); continue; }
@@ -144,14 +144,14 @@
     }
     #[test] fn catalogue_matches_the_phase6_master_list() {
         let defs = all();
-        assert_eq!(defs.len(), 301, "Phase 6 master list: 66 Batch A + 233 new; Phase 7: session, sessions_left");
-        assert_eq!(defs.iter().filter(|d| d.compute.is_some()).count(), 185);
+        assert_eq!(defs.len(), 302, "Phase 6 master list: 66 Batch A + 233 new; Phase 7: session, sessions_left, expiry");
+        assert_eq!(defs.iter().filter(|d| d.compute.is_some()).count(), 186);
         assert_eq!(defs.iter().filter(|d| d.compute.is_none()).count(), 116);
         let mut c = std::collections::BTreeMap::new();
         for (_, tv, ab) in crate::lab::sources::SOURCES {
             *c.entry(match (tv.is_empty(), ab.is_empty()) { (false, false) => "both", (false, true) => "tv", (true, false) => "ab", _ => "lab" }).or_insert(0) += 1;
         }
-        assert_eq!((c["tv"], c["ab"], c["both"], c["lab"]), (234, 25, 41, 1));
+        assert_eq!((c["tv"], c["ab"], c["both"], c["lab"]), (234, 26, 41, 1));
     }
     #[test] fn volume_indicators_refuse_without_volume() {
         let bars = walk(300, 3, false);
PATCH
```

- [ ] **Step 2: Run the tests and confirm the failure**

Run: `cd Context/guru/backend && CARGO_BUILD_JOBS=2 cargo test lab:: 2>&1 | tail -20`
Expected: Compile errors: `Dist::Ticks`, `Exec.futures` and `lab::futures` are missing.

- [ ] **Step 3: Implement** (apply from the repository root)

```bash
git apply --whitespace=nowarn <<'PATCH'
diff --git a/Context/guru/backend/src/lab/rules.rs b/Context/guru/backend/src/lab/rules.rs
--- a/Context/guru/backend/src/lab/rules.rs
+++ b/Context/guru/backend/src/lab/rules.rs
@@ -97,7 +97,15 @@
 pub enum Fill { NextOpen, Close }
 /// A price distance: percent of a reference price, or a multiple of ATR(length) read at a bar already closed.
 #[derive(Clone, Copy, Debug, PartialEq)]
-pub enum Dist { Pct(f64), Atr { mult: f64, length: usize }, Points(f64) }
+pub enum Dist { Pct(f64), Atr { mult: f64, length: usize }, Points(f64), Ticks(f64) }
+/// Futures roll rule and continuous-series adjustment (spec §7.3).
+#[derive(Clone, Copy, Debug, PartialEq, Eq)]
+pub enum Roll { DaysBefore(usize), Volume, OpenInterest, None }
+#[derive(Clone, Copy, Debug, PartialEq, Eq)]
+pub enum Adjust { Forward, Ratio, None }
+#[derive(Clone, Copy, Debug, PartialEq, Eq)]
+pub struct Futures { pub roll: Roll, pub adjust: Adjust }
+pub const MAX_TICKS: f64 = 100_000.;
 /// The price-order settings an optimize path can address (`entry.limit.pct`, `exit.stop_loss.atr`, ...).
 #[derive(Clone, Copy, Debug, PartialEq, Eq)]
 pub enum Slot { Limit, StopLoss, Target, Trailing, Stop, Activate, BreakevenAfter, TakeProfit(usize) }
@@ -148,13 +156,15 @@
     pub cooldown: Option<(usize, After)>, pub max_trades: Option<(usize, usize)>, pub risk: Risk,
     /// money mode (spec §7.4): starting capital and declared costs
     pub capital: Option<f64>, pub costs: MoneyCosts,
+    /// execution.instrument kind futures (None = cash)
+    pub futures: Option<Futures>,
 }
 impl Default for Exec {
     fn default() -> Self {
         Exec { short: false, fill: Fill::NextOpen, exit_fill: Fill::NextOpen, limit: None, stop_loss: None, target: None, trailing: None, opposite: Opposite::Reverse,
                stop: None, limit_at: None, stop_at: None, valid_bars: 1, delay: 1, edge: false,
                trail_activate: None, breakeven: None, take_profits: vec![], min_bars: 0, ohlc_path: false, scale_in: None, sizing: Sizing::Full,
-               cooldown: None, max_trades: None, risk: Risk::default(), capital: None, costs: MoneyCosts::default() }
+               cooldown: None, max_trades: None, risk: Risk::default(), capital: None, costs: MoneyCosts::default(), futures: None }
     }
 }
 impl Exec {
@@ -221,7 +231,8 @@
     };
     let mut cost_bps = None;
     if let Some(ex) = root.get("execution") {
-        fields(ex, "execution", &["fill", "exit_fill", "cost_bps", "opposite", "delay_bars", "entry_on", "intrabar", "sizing", "cooldown", "max_trades", "risk", "capital", "costs"], &["limit", "stop"])?;
+        fields(ex, "execution", &["fill", "exit_fill", "cost_bps", "opposite", "delay_bars", "entry_on", "intrabar", "sizing", "cooldown", "max_trades", "risk", "capital", "costs", "instrument"], &["limit", "stop"])?;
+        if let Some(v) = ex.get("instrument") { exec.futures = instrument(v)?; }
         if let Some(c) = ex.get("capital") {
             exec.capital = Some(c.as_f64().filter(|c| *c > 0. && *c <= MAX_CAPITAL).ok_or_else(|| inv(format!("execution.capital must be a number above 0 and at most {MAX_CAPITAL}")))?);
             if ex.get("cost_bps").is_some() { return Err(inv("money mode takes costs from execution.costs; remove execution.cost_bps")); }
@@ -306,6 +317,16 @@
     if let Some(s) = short_side.as_mut() {
         (s.exec.fill, s.exec.exit_fill, s.exec.opposite, s.exec.delay, s.exec.edge, s.exec.ohlc_path) = (exec.fill, exec.exit_fill, exec.opposite, exec.delay, exec.edge, exec.ohlc_path);
     }
+    let execs: Vec<&Exec> = std::iter::once(&exec).chain(short_side.as_ref().map(|s| &s.exec)).collect();
+    if exec.futures.is_none() && execs.iter().any(|x| dists(x).iter().any(|d| matches!(d, Dist::Ticks(_)))) {
+        return Err(inv("ticks need a futures contract's tick size (execution.instrument kind futures)"));
+    }
+    if exec.futures.is_some() {
+        if execs.iter().any(|x| x.scale_in.is_some() || !x.take_profits.is_empty()) { return Err(inv("futures runs do not combine with scale_in or take_profits yet")); }
+        if exec.futures.is_some_and(|f| f.roll == Roll::None) && exec.exit_fill != Fill::Close {
+            return Err(inv("roll \"none\" closes positions at the expiry close, so it needs execution.exit_fill \"close\""));
+        }
+    }
     let mut spec = Spec { entry, exit, cost_bps, optimize: vec![], exec, origin, short_side };
     if let Some(o) = root.get("optimize") { spec.optimize = dims(&spec, o)?; }
     Ok(spec)
@@ -369,6 +390,46 @@
     Ok((entry, exit))
 }
 
+/// `execution.instrument`: cash (default) or a futures contract chain with its roll and adjustment (spec §7.3).
+fn instrument(v: &Json) -> Result<Option<Futures>, LabError> {
+    fields(v, "execution.instrument", &["kind", "roll", "adjust"], &[])?;
+    match v.get("kind").and_then(Json::as_str) {
+        None | Some("cash") if v.get("roll").is_none() && v.get("adjust").is_none() => Ok(None),
+        None | Some("cash") => Err(inv("execution.instrument roll and adjust apply to kind futures")),
+        Some("futures") => {
+            let roll = match v.get("roll") {
+                None => Roll::DaysBefore(1),
+                Some(Json::Str(s)) if s == "none" => Roll::None,
+                Some(r) => {
+                    fields(r, "execution.instrument.roll", &["days_before_expiry", "on"], &[])?;
+                    match (r.get("days_before_expiry"), r.get("on").and_then(Json::as_str)) {
+                        (Some(n), None) => Roll::DaysBefore(whole(Some(n), "execution.instrument.roll.days_before_expiry", 0)?),
+                        (None, Some("volume")) => Roll::Volume,
+                        (None, Some("open_interest")) => Roll::OpenInterest,
+                        _ => return Err(inv("execution.instrument.roll must be \"none\", {\"days_before_expiry\": n} or {\"on\": \"volume\" | \"open_interest\"}")),
+                    }
+                }
+            };
+            let adjust = match v.get("adjust").and_then(Json::as_str) {
+                None | Some("forward") => Adjust::Forward,
+                Some("ratio") => Adjust::Ratio,
+                Some("none") => Adjust::None,
+                Some("back") => return Err(LabError::Unsupported("execution.instrument.adjust \"back\" rewrites earlier prices at every roll (lookahead); use forward, ratio or none".into())),
+                _ => return Err(inv("execution.instrument.adjust must be forward, ratio or none")),
+            };
+            Ok(Some(Futures { roll, adjust }))
+        }
+        _ => Err(inv("execution.instrument.kind must be cash or futures")),
+    }
+}
+/// Every distance an execution setting holds (ticks are futures-only).
+fn dists(x: &Exec) -> Vec<Dist> {
+    let mut v: Vec<Dist> = [x.limit, x.stop_loss, x.target, x.trailing, x.stop, x.trail_activate].into_iter().flatten().collect();
+    v.extend(x.breakeven.iter().flat_map(|b| [Some(b.0), b.1]).flatten());
+    v.extend(x.take_profits.iter().map(|t| t.0));
+    v
+}
+
 fn money_costs(v: &Json) -> Result<MoneyCosts, LabError> {
     fields(v, "execution.costs", &["commission", "slippage", "taxes", "borrow_pct_year"], &[])?;
     let num = |v: Option<&Json>, at: &str, hi: f64| -> Result<f64, LabError> {
@@ -436,10 +497,15 @@
 }
 /// `{"pct": p}` or `{"atr": m, "atr_length": n}` (n defaults to 14).
 fn dist(v: &Json, at: &str, hi: f64) -> Result<Dist, LabError> {
-    fields(v, at, &["pct", "atr", "atr_length", "points"], &[])?;
+    fields(v, at, &["pct", "atr", "atr_length", "points", "ticks"], &[])?;
     // an entry order may sit at the signal close (0); exits need a positive distance
     let order = at.ends_with("entry.limit") || at.ends_with("entry.stop");
     let min = if order { "from 0" } else { "above 0" };
+    if let Some(t) = v.get("ticks") {
+        if ["pct", "atr", "atr_length", "points"].iter().any(|k| v.get(k).is_some()) { return Err(inv(format!("{at} needs exactly one of pct or atr, or points or ticks"))); }
+        return t.as_f64().filter(|n| n.fract() == 0. && *n <= MAX_TICKS && (*n > 0. || (*n == 0. && order))).map(Dist::Ticks)
+            .ok_or_else(|| inv(format!("{at}.ticks must be a whole number {min} and at most {MAX_TICKS}")));
+    }
     match (v.get("pct"), v.get("atr"), v.get("points")) {
         (Some(p), None, None) if v.get("atr_length").is_none() => {
             p.as_f64().filter(|p| *p <= hi && (*p > 0. || (*p == 0. && order))).map(Dist::Pct)
@@ -729,9 +795,9 @@
     if let Some((slot, at, field, _)) = slot_at(rest) {
         let exec = exec_of(spec, path).ok_or("the spec has no short_side")?;
         return match (exec.slot(slot), field.as_str()) {
-            (Some(Dist::Pct(_)), "pct") | (Some(Dist::Atr { .. }), "atr") | (Some(Dist::Points(_)), "points") => Ok(Target { path: path.into(), slot: Some(slot) }),
+            (Some(Dist::Pct(_)), "pct") | (Some(Dist::Atr { .. }), "atr") | (Some(Dist::Points(_)), "points") | (Some(Dist::Ticks(_)), "ticks") => Ok(Target { path: path.into(), slot: Some(slot) }),
             (None, _) => Err(format!("the spec has no {at}")),
-            (Some(d), _) => Err(format!("{at} is set by {}; optimize that field", match d { Dist::Pct(_) => "pct", Dist::Atr { .. } => "atr", Dist::Points(_) => "points" })),
+            (Some(d), _) => Err(format!("{at} is set by {}; optimize that field", match d { Dist::Pct(_) => "pct", Dist::Atr { .. } => "atr", Dist::Points(_) => "points", Dist::Ticks(_) => "ticks" })),
         };
     }
     if !["entry.all[", "exit.any[", "entry.limit.at.", "entry.stop.at.", "entry.scale_in.", "execution.sizing."].iter().any(|p| rest.starts_with(p)) {
@@ -830,7 +896,7 @@
 
 fn set(s: &mut Spec, t: &Target, x: f64) {
     if let Some(slot) = t.slot {
-        match exec_mut(s, &t.path).and_then(|e| e.slot_mut(slot)) { Some(Dist::Pct(p) | Dist::Points(p)) => *p = x, Some(Dist::Atr { mult, .. }) => *mult = x, None => {} }
+        match exec_mut(s, &t.path).and_then(|e| e.slot_mut(slot)) { Some(Dist::Pct(p) | Dist::Points(p) | Dist::Ticks(p)) => *p = x, Some(Dist::Atr { mult, .. }) => *mult = x, None => {} }
         return;
     }
     walk(s, &mut |p, k| if p == t.path {
@@ -849,6 +915,7 @@
         let ok = match exec_of(spec, &t.path).and_then(|e| e.slot(slot)) {
             Some(Dist::Pct(_)) => x <= hi && (x > 0. || (x == 0. && order)),
             Some(Dist::Points(_)) => x <= MAX_POINTS && (x > 0. || (x == 0. && order)),
+            Some(Dist::Ticks(_)) => x.fract() == 0. && x <= MAX_TICKS && (x > 0. || (x == 0. && order)),
             _ => x > 0. && x <= MAX_ATR_MULT,
         };
         return if ok { Ok(()) } else { Err(format!("{at} value {x} is out of range")) };
@@ -870,7 +937,7 @@
 }
 /// The spec's own value at an optimize target (the "base" setting the ranking prefers on ties).
 pub fn current(spec: &Spec, t: &Target) -> f64 {
-    if let Some(slot) = t.slot { return match exec_of(spec, &t.path).and_then(|e| e.slot(slot)) { Some(Dist::Pct(p) | Dist::Points(p)) => p, Some(Dist::Atr { mult, .. }) => mult, None => f64::NAN }; }
+    if let Some(slot) = t.slot { return match exec_of(spec, &t.path).and_then(|e| e.slot(slot)) { Some(Dist::Pct(p) | Dist::Points(p) | Dist::Ticks(p)) => p, Some(Dist::Atr { mult, .. }) => mult, None => f64::NAN }; }
     let mut v = f64::NAN;
     walk(&mut spec.clone(), &mut |p, k| if p == t.path {
         match k {
diff --git a/Context/guru/backend/src/lab/series.rs b/Context/guru/backend/src/lab/series.rs
--- a/Context/guru/backend/src/lab/series.rs
+++ b/Context/guru/backend/src/lab/series.rs
@@ -59,6 +59,7 @@
                     let def = catalogue::get(&s.id).ok_or_else(|| LabError::Invalid(format!("unknown series '{}'", s.id)))?;
                     let o = Rc::new(catalogue::run(&def, b, &s.params).map_err(|e| match e.as_str() {
                         "needs volume" => LabError::NeedsData(format!("{}: needs volume, which is missing on some admitted bars of this instrument", def.name)),
+                        "needs contract data" => LabError::NeedsData(format!("{}: needs futures contract data (expiry dates), which only a futures run has", def.name)),
                         "needs breadth" => LabError::NeedsData(format!("{}: needs market breadth (advances/declines) on daily bars; this market or timeframe has none", def.name)),
                         "needs intraday data" => LabError::NeedsData(format!("{}: needs intraday data", def.name)),
                         _ => LabError::Invalid(format!("{}: {e}", def.id)),
@@ -113,14 +114,18 @@
 pub struct Compiled {
     pub entry: Vec<RCond>, pub exit: Vec<RCond>, pub exec: Exec, pub atr: BTreeMap<usize, Vec<f64>>, pub other: Option<Box<Compiled>>, pub cost: f64,
     pub limit_at: Option<Vec<f64>>, pub stop_at: Option<Vec<f64>>,
+    /// futures tick size: `ticks` distances and order rounding (0 = none)
+    pub tick: f64,
 }
 impl Compiled {
     /// Long, market orders at the next open, no price exits (the v1 defaults).
-    pub fn new(entry: Vec<RCond>, exit: Vec<RCond>) -> Compiled { Compiled { entry, exit, exec: Exec::default(), atr: BTreeMap::new(), other: None, cost: 0., limit_at: None, stop_at: None } }
+    pub fn new(entry: Vec<RCond>, exit: Vec<RCond>) -> Compiled { Compiled { entry, exit, exec: Exec::default(), atr: BTreeMap::new(), other: None, cost: 0., limit_at: None, stop_at: None, tick: 0. } }
     pub fn has_order(&self) -> bool { self.exec.has_order() }
+    /// An order or exit level on the tick grid (futures; nearest tick, spec §7.3). Unchanged without a tick.
+    pub fn round(&self, p: f64) -> f64 { if self.tick > 0. { (p / self.tick).round() * self.tick } else { p } }
     /// Price distance of `d` from `px`, using ATR at bar `k` (a bar whose close is already known). NaN while ATR warms up.
     pub fn dist(&self, d: Dist, px: f64, k: usize) -> f64 {
-        match d { Dist::Pct(p) => px * p / 100., Dist::Atr { mult, length } => self.atr.get(&length).map_or(f64::NAN, |a| mult * a[k]), Dist::Points(p) => p }
+        match d { Dist::Pct(p) => px * p / 100., Dist::Atr { mult, length } => self.atr.get(&length).map_or(f64::NAN, |a| mult * a[k]), Dist::Points(p) => p, Dist::Ticks(n) => n * self.tick }
     }
 }
 
diff --git a/Context/guru/backend/src/lab/api.rs b/Context/guru/backend/src/lab/api.rs
--- a/Context/guru/backend/src/lab/api.rs
+++ b/Context/guru/backend/src/lab/api.rs
@@ -200,6 +200,11 @@
     if spec.exec.costs.slip_ticks > 0. { return Err(LabError::Invalid("slippage in ticks needs a futures contract's tick size; use bps".into())); }
     Ok(Some(ledger::Money { capital, sizing: spec.exec.sizing, costs: spec.exec.costs.clone(), contract: ledger::CASH }))
 }
+/// Futures need an admitted contract chain (expiries, lot sizes, tick); none is admitted, so every real request is refused.
+fn futures_refusal(spec: &Spec, ticker: &str) -> Result<(), LabError> {
+    if spec.exec.futures.is_none() { return Ok(()); }
+    Err(LabError::NeedsData(format!("futures contract data for {ticker} (contract chain with expiries, lot sizes and tick size) is not admitted; no futures result is published, and the index or spot series is never used as a stand-in")))
+}
 /// The instrument's market code ("in" / "us"; None for gl instruments).
 fn market_of(selected: &crate::SelectedContext, ticker: &str) -> Option<&'static str> {
     markets::gl(ticker).map_or_else(|| selected.evidence().map(|e| e.market().code()).or_else(|| crate::preloaded_market(ticker).map(|m| m.code())), |_| None)
@@ -230,6 +235,7 @@
     let costs = accounting::Costs::parse(q)?;
     costs.ensure_rollover()?;
     let cost = costs.side_rate();
+    futures_refusal(&spec, &ctx.r[0].ticker)?;
     let model = model_of(&selected, &ctx.r[0].ticker, kind, &spec)?;
     let money = money_of(&spec, q, market_of(&selected, &ctx.r[0].ticker))?;
     let fr = Frames::new(Bars::from_ctx(ctx));
@@ -282,6 +288,7 @@
         Some(s) => Some(Some(dates.partition_point(|d| *d < s.as_str())).filter(|&k| k > from + 20 && k + 20 < to)
             .ok_or_else(|| LabError::Invalid("split must leave more than 20 bars on each side of the range".into()))?),
     };
+    futures_refusal(&spec, &ctx.r[0].ticker)?;
     let model = model_of(&selected, &ctx.r[0].ticker, kind, &spec)?;
     let money = money_of(&spec, q, market_of(&selected, &ctx.r[0].ticker))?;
     let fr = Frames::new(Bars::from_ctx(ctx));
diff --git a/Context/guru/backend/src/lab/mod.rs b/Context/guru/backend/src/lab/mod.rs
--- a/Context/guru/backend/src/lab/mod.rs
+++ b/Context/guru/backend/src/lab/mod.rs
@@ -10,6 +10,7 @@
 pub mod breadth;
 pub mod catalogue;
 pub mod fidelity;
+pub mod futures;
 pub mod json;
 pub mod ledger;
 pub mod listed;
diff --git a/Context/guru/backend/src/lab/simulate.rs b/Context/guru/backend/src/lab/simulate.rs
--- a/Context/guru/backend/src/lab/simulate.rs
+++ b/Context/guru/backend/src/lab/simulate.rs
@@ -22,7 +22,7 @@
 impl Pos {
     /// Levels are fixed from the entry price and ATR at `sig`, the bar whose close produced the order.
     fn open(c: &Compiled, i: usize, px: f64, sig: usize, kind: &'static str, d: f64) -> Pos {
-        let lvl = |x: Option<Dist>, side: f64| x.map_or(f64::NAN, |x| px + side * d * c.dist(x, px, sig));
+        let lvl = |x: Option<Dist>, side: f64| x.map_or(f64::NAN, |x| c.round(px + side * d * c.dist(x, px, sig)));
         let x = &c.exec;
         let stop = lvl(x.stop_loss, -1.);
         // unit-mode size: a fraction of one unit (money mode re-sizes in lab/ledger.rs); 0 = cannot size (risk without a stop level)
@@ -126,8 +126,8 @@
     // An order's levels come from the signal bar; a configured level that is unknown (warm-up) places no order.
     let place = |s: usize, i: usize| -> Option<Order> {
         let (side, d, close) = (sides[s], dir(s), b.close[i]);
-        let limit = side.limit_at.as_ref().map(|a| a[i]).or(side.exec.limit.map(|l| close - d * side.dist(l, close, i)));
-        let stop = side.stop_at.as_ref().map(|a| a[i]).or(side.exec.stop.map(|st| close + d * side.dist(st, close, i)));
+        let limit = side.limit_at.as_ref().map(|a| a[i]).or(side.exec.limit.map(|l| close - d * side.dist(l, close, i))).map(|p| side.round(p));
+        let stop = side.stop_at.as_ref().map(|a| a[i]).or(side.exec.stop.map(|st| close + d * side.dist(st, close, i))).map(|p| side.round(p));
         (limit.map_or(true, f64::is_finite) && stop.map_or(true, f64::is_finite))
             .then_some(Order { side: s, sig: i, limit: limit.unwrap_or(f64::NAN), stop: stop.unwrap_or(f64::NAN), until: i + side.exec.valid_bars })
     };
diff --git a/Context/guru/backend/src/lab/catalogue.rs b/Context/guru/backend/src/lab/catalogue.rs
--- a/Context/guru/backend/src/lab/catalogue.rs
+++ b/Context/guru/backend/src/lab/catalogue.rs
@@ -39,7 +39,7 @@
     ParamSpec { key, label, default, min, max, step, integer: false }
 }
 pub fn all() -> Vec<IndicatorDef> {
-    [super::ma::DEFS, super::momentum::DEFS, super::trend::DEFS, super::volatility::DEFS, super::volume::DEFS, super::price::DEFS, super::listed::DEFS, super::breadth::DEFS, super::ma2::DEFS, super::bands::DEFS, super::stats::DEFS, super::osc2::DEFS, super::vol2::DEFS, super::stops::DEFS, super::volume2::DEFS, super::periods::DEFS, super::patterns::DEFS, super::candles::DEFS, super::ratings::DEFS].concat()
+    [super::ma::DEFS, super::momentum::DEFS, super::trend::DEFS, super::volatility::DEFS, super::volume::DEFS, super::price::DEFS, super::listed::DEFS, super::breadth::DEFS, super::ma2::DEFS, super::bands::DEFS, super::stats::DEFS, super::osc2::DEFS, super::vol2::DEFS, super::stops::DEFS, super::volume2::DEFS, super::periods::DEFS, super::patterns::DEFS, super::candles::DEFS, super::ratings::DEFS, super::futures::DEFS].concat()
 }
 pub fn get(id: &str) -> Option<IndicatorDef> { all().into_iter().find(|d| d.id == id) }
 pub fn resolve_params(def: &IndicatorDef, given: &Params) -> Result<Params, String> {
diff --git a/Context/guru/backend/src/lab/sources.rs b/Context/guru/backend/src/lab/sources.rs
--- a/Context/guru/backend/src/lab/sources.rs
+++ b/Context/guru/backend/src/lab/sources.rs
@@ -249,6 +249,7 @@
     ("calendar", "", "DayOfWeek; Day; Month; Year; DayOfYear; BarIndex"),
     ("session", "", "no built-in; AFL idiom BarsSince(Month() != Ref(Month(), -1)) + 1"),
     ("sessions_left", "", "no built-in; AFL idiom with a holiday list"),
+    ("expiry", "", "no built-in; AFL idiom with an expiry-date list (GetFnData has none)"),
     ("moon_phases", "Moon Phases", ""),
     ("highest_bars", "ta.highestbars (Pine)", "HHVBars; HighestBars"),
     ("lowest_bars", "ta.lowestbars (Pine)", "LLVBars; LowestBars"),
diff --git a/Context/guru/backend/src/lab/bars.rs b/Context/guru/backend/src/lab/bars.rs
--- a/Context/guru/backend/src/lab/bars.rs
+++ b/Context/guru/backend/src/lab/bars.rs
@@ -12,6 +12,8 @@
     pub end: Vec<usize>,
     /// Daily breadth of the market (from extra_data `<MARKET>_ADV_DEC.csv`); None on W/M bars and markets without it.
     pub breadth: Option<Breadth>,
+    /// Expiry date of the futures contract held on each daily bar (futures runs only, lab/futures.rs); None elsewhere.
+    pub expiry: Option<Vec<String>>,
 }
 
 /// Market breadth of the instrument's market, aligned to its daily bars (NaN = no value that day).
@@ -34,6 +36,7 @@
             volume,
             end: (0..rows.len()).collect(),
             breadth: None,
+            expiry: None,
         }
     }
     /// Daily bars plus the market's advance/decline series when the engine loaded `<MARKET>_ADV_DEC.csv`.
diff --git a/Context/guru/backend/src/lab/futures.rs b/Context/guru/backend/src/lab/futures.rs
new file mode 100644
--- /dev/null
+++ b/Context/guru/backend/src/lab/futures.rs
@@ -0,0 +1,223 @@
+//! Futures contract model (Phase 7 spec §7.3): a chain of contracts, a roll rule, a causal continuous series for signals,
+//! fills translated back to the held contract's actual prices, a roll inside a trade as two costed trades, money on each
+//! contract's lot / point value / margin, and margin calls. No futures contract data is admitted yet, so only Rust
+//! fixtures reach this module; real requests answer needs_data (api.rs futures_refusal).
+use super::bars::Bars;
+use super::catalogue::{IndicatorDef, Needs, Output, Params, Placement};
+use super::ledger::{self, Contract as Terms, Money, Row};
+use super::resample::{day_number, period_key};
+use super::rules::{Adjust, LabError, Op, Roll, Spec};
+use super::series::{self, Cache, Frames, RCond, Val};
+use super::simulate::simulate_events;
+use crate::connors::{Leg, Trade};
+
+/// One futures contract: its own daily bars (dates ascending), expiry date and terms.
+#[derive(Clone, Debug)]
+pub struct Contract { pub symbol: String, pub expiry: String, pub lot: f64, pub tick: f64, pub point_value: f64, pub margin_pct: f64, pub bars: Bars, pub oi: Option<Vec<f64>> }
+/// The continuous series (a daily bar for each session held) with what maps it back: the contract held on each bar, and
+/// continuous = actual × factor + offset for that bar's contract. `rolls` are the bars at whose close a roll happens, with the
+/// incoming contract's actual close there.
+#[derive(Debug, Default)]
+pub struct Continuous { pub bars: Bars, pub held: Vec<usize>, pub off: Vec<f64>, pub factor: Vec<f64>, pub rolls: Vec<(usize, f64)> }
+impl Continuous {
+    pub fn actual(&self, i: usize, px: f64) -> f64 { (px - self.off[i]) / self.factor[i] }
+}
+
+/// The continuous series. Forward adjustment keeps every earlier bar fixed and shifts later contracts onto it, so a value is
+/// never rewritten when a later roll arrives (causal); backward adjustment would be lookahead and is refused by the parser.
+pub fn continuous(chain: &[Contract], roll: Roll, adjust: Adjust) -> Result<Continuous, LabError> {
+    let bad = |m: String| LabError::Invalid(format!("futures chain: {m}"));
+    if chain.is_empty() { return Err(bad("no contracts".into())); }
+    if chain.windows(2).any(|w| w[0].expiry >= w[1].expiry || w[0].tick != w[1].tick) { return Err(bad("contracts must be ordered by expiry and share one tick size".into())); }
+    let mut out = Continuous::default();
+    out.bars.expiry = Some(vec![]);
+    let (mut off, mut factor, mut after): (f64, f64, Option<String>) = (0., 1., None);
+    for (k, c) in chain.iter().enumerate() {
+        let b = &c.bars;
+        let from = b.date.partition_point(|d| after.as_ref().is_some_and(|a| d <= a));
+        let exp = b.date.iter().rposition(|d| *d <= c.expiry).filter(|e| *e >= from).ok_or_else(|| bad(format!("{} has no bar on or before its expiry after the previous roll", c.symbol)))?;
+        let next = chain.get(k + 1);
+        let on_date = |n: &Contract, i: usize| n.bars.date.iter().position(|d| *d == b.date[i]);
+        let at = match (next, roll) {
+            (None, _) => exp,
+            (Some(_), Roll::DaysBefore(n)) => exp.saturating_sub(n).max(from),
+            (Some(_), Roll::None) => exp,
+            (Some(n), Roll::Volume | Roll::OpenInterest) => {
+                // the first session on which the next contract trades more (volume) or holds more open interest; else expiry
+                let v = |x: &Contract, i: usize| if roll == Roll::Volume { &x.bars.volume } else { &x.oi }.as_ref().map_or(f64::NAN, |v| v[i]);
+                (from..=exp).find(|&i| on_date(n, i).is_some_and(|j| v(n, j) > v(c, i))).unwrap_or(exp)
+            }
+        };
+        for i in from..=at {
+            let p = |x: f64| x * factor + off;
+            out.bars.date.push(b.date[i].clone());
+            out.bars.open.push(p(b.open[i])); out.bars.high.push(p(b.high[i])); out.bars.low.push(p(b.low[i])); out.bars.close.push(p(b.close[i]));
+            out.bars.end.push(out.bars.end.len());
+            out.bars.expiry.as_mut().unwrap().push(c.expiry.clone());
+            out.held.push(k); out.off.push(off); out.factor.push(factor);
+        }
+        if let Some(n) = next {
+            let j = on_date(n, at).ok_or_else(|| bad(format!("{} has no bar on the roll date {}", n.symbol, b.date[at])))?;
+            let (old, new) = (b.close[at], n.bars.close[j]);
+            out.rolls.push((out.bars.len() - 1, new));
+            match adjust {
+                Adjust::Forward => off += factor * (old - new),
+                Adjust::Ratio => factor *= old / new,
+                Adjust::None => {}
+            }
+            after = Some(b.date[at].clone());
+        }
+    }
+    Ok(out)
+}
+
+/// Simulator trades on the continuous series as trades on the held contracts' actual prices; a roll inside a trade closes
+/// the old contract at the roll-day close and opens the new one at the same close (`entry_kind` "roll").
+pub fn to_contracts(trades: &[Trade], c: &Continuous) -> Vec<(Trade, usize)> {
+    let mut out = Vec::new();
+    for t in trades {
+        let (mut e, mut px, mut kind) = (t.legs[0].i, c.actual(t.legs[0].i, t.legs[0].px), t.entry_kind);
+        let inside: Vec<(usize, f64)> = c.rolls.iter().copied().filter(|(r, _)| e <= *r && *r < t.exit_i).collect();
+        for (r, new) in inside {
+            let piece = Trade { legs: vec![Leg { i: e, px, w: 1. }], exit_i: r, exit_px: c.actual(r, c.bars.close[r]), reason: "roll".into(), entry_kind: kind, signal_i: r, open: false, exits: vec![], ..t.clone() };
+            out.push((piece, c.held[r]));
+            (e, px, kind) = (r, new, "roll");
+        }
+        out.push((Trade { legs: vec![Leg { i: e, px, w: t.legs[0].w }], exit_px: c.actual(t.exit_i, t.exit_px), entry_kind: kind, ..t.clone() }, c.held[t.exit_i]));
+    }
+    out
+}
+
+pub struct FuturesRun { pub series: Continuous, pub rows: Vec<(Trade, usize, Row)>, pub events: Vec<String>, pub final_equity: f64 }
+/// A whole futures backtest on a chain (fixtures only until contract data is admitted).
+pub fn run(chain: &[Contract], spec: &Spec, capital: f64) -> Result<FuturesRun, LabError> {
+    let f = spec.exec.futures.ok_or_else(|| LabError::Invalid("the spec is not a futures spec (execution.instrument kind futures)".into()))?;
+    let cont = continuous(chain, f.roll, f.adjust)?;
+    let fr = Frames::new(cont.bars.clone());
+    let mut c = series::compile(spec, &fr, &mut Cache::new())?;
+    c.tick = chain[0].tick;
+    if let Some(o) = c.other.as_mut() { o.tick = chain[0].tick; }
+    if f.roll == Roll::None { // no roll: a position closes at its contract's expiry close
+        let day: Vec<f64> = (0..cont.bars.len()).map(|i| if cont.bars.date[i] == chain[cont.held[i]].expiry { 1. } else { 0. }).collect();
+        let expiry = || RCond { lhs: Val::Arr(day.clone()), op: Op::Gt, rhs: Val::Const(0.5), label: "expiry".into() };
+        c.exit.insert(0, expiry());
+        if let Some(o) = c.other.as_mut() { o.exit.insert(0, expiry()); }
+    }
+    let (trades, mut events) = simulate_events(&c, &fr.d, 0, fr.d.len() - 1);
+    let (mut equity, mut rows) = (capital, Vec::new());
+    for (t, k) in to_contracts(&trades, &cont) {
+        let ct = &chain[k];
+        let m = Money { capital: equity, sizing: spec.exec.sizing, costs: spec.exec.costs.clone(), contract: Terms { lot: ct.lot, point_value: ct.point_value, tick: ct.tick, margin_pct: Some(ct.margin_pct) } };
+        let Some(row) = ledger::book(std::slice::from_ref(&t), &cont.bars.date, &m).rows.into_iter().next() else { continue };
+        // margin call: equity at a close below the margin the position needs → liquidated at the next open; trading stops
+        let (q, d, entry) = (row.qty * ct.point_value, t.dir, row.real.legs[0].px);
+        let call = (t.legs[0].i..t.exit_i).find(|&j| { let px = cont.actual(j, cont.bars.close[j]); equity + q * d * (px - entry) < q * px * ct.margin_pct / 100. });
+        if let Some(j) = call {
+            let cut = Trade { exit_i: j + 1, exit_px: cont.actual(j + 1, cont.bars.open[j + 1]), reason: "margin_call".into(), signal_i: j, open: false, exits: vec![], ..t.clone() };
+            if let Some(r) = ledger::book(std::slice::from_ref(&cut), &cont.bars.date, &m).rows.into_iter().next() { equity += r.pnl; rows.push((cut, k, r)); }
+            events.push(format!("margin call on {}: the position was liquidated at the next open and trading stopped", cont.bars.date[j]));
+            break;
+        }
+        equity += row.pnl;
+        rows.push((t, k, row));
+    }
+    Ok(FuturesRun { series: cont, rows, events, final_equity: equity })
+}
+
+/// Expiry operands from the held contract's expiry date (published contract metadata, so causal): weekdays left until expiry,
+/// 1 in the expiry week, 1 on the expiry day. Only futures runs carry expiry dates; elsewhere this answers needs_data.
+fn expiry(b: &Bars, _: &Params) -> Output {
+    let Some(exp) = b.expiry.as_ref() else { return Err("needs contract data".into()) };
+    let days = |d: &str, e: &str| (day_number(d) + 1..=day_number(e)).filter(|n| (1..=5).contains(&(n + 4).rem_euclid(7))).count() as f64;
+    Ok(vec![
+        b.date.iter().zip(exp).map(|(d, e)| days(d, e)).collect(),
+        b.date.iter().zip(exp).map(|(d, e)| if period_key(d, 1) == period_key(e, 1) { 1. } else { 0. }).collect(),
+        b.date.iter().zip(exp).map(|(d, e)| if d == e { 1. } else { 0. }).collect(),
+    ])
+}
+pub const DEFS: &[IndicatorDef] = &[IndicatorDef {
+    id: "expiry", name: "Futures Expiry", category: "Periods & calendar", params: &[], outputs: &["days_to_expiry", "expiry_week", "expiry_day"],
+    placement: Placement::Pane, needs: Needs { derivatives: true, ..Needs::NONE },
+    definition: "weekdays left until the held contract's expiry date, 1 in its expiry week, 1 on its expiry day (futures runs only; the expiry calendar is contract metadata)",
+    first: &[0, 0, 0], sparse: false, compute: Some(expiry), note: "",
+}];
+
+#[cfg(test)]
+mod tests {
+    use super::*;
+    use crate::lab::rules::parse_spec;
+    use crate::lab::testdata::assert_series;
+    /// Contract `k`: closes base + i on its own sessions `lo..=hi` of a shared 30-weekday calendar (O = H = L = C unless changed).
+    fn contract(k: usize, lo: usize, hi: usize, base: f64) -> Contract {
+        let mut b = Bars::default();
+        let mut day = day_number("2024-01-01");
+        let mut dates = Vec::new();
+        while dates.len() < 30 { if (1..=5).contains(&(day + 4).rem_euclid(7)) { dates.push(crate::lab::resample::civil(day)); } day += 1; }
+        for i in lo..=hi { let p = base + i as f64; b.date.push(dates[i].clone()); for v in [&mut b.open, &mut b.high, &mut b.low, &mut b.close] { v.push(p); } b.end.push(b.end.len()); }
+        b.volume = Some(vec![1000.; b.date.len()]);
+        Contract { symbol: format!("F{k}"), expiry: dates[hi].clone(), lot: 50., tick: 0.05, point_value: 1., margin_pct: 10., bars: b, oi: None }
+    }
+    fn chain() -> Vec<Contract> { vec![contract(0, 0, 9, 100.), contract(1, 0, 19, 105.), contract(2, 10, 29, 110.)] }
+    #[test] fn forward_adjustment_is_smooth_and_maps_back_to_actual_prices() {
+        let c = continuous(&chain(), Roll::DaysBefore(1), Adjust::Forward).unwrap();
+        assert_series(&c.bars.close, &(0..30).map(|i| 100. + i as f64).collect::<Vec<_>>());
+        assert_eq!((c.held[8], c.held[9], c.held[18], c.held[19]), (0, 1, 1, 2));
+        assert_eq!(c.rolls, vec![(8, 113.), (18, 128.)]);
+        assert_eq!((c.actual(12, c.bars.close[12]), c.actual(25, c.bars.close[25])), (117., 135.));
+        let raw = continuous(&chain(), Roll::DaysBefore(1), Adjust::None).unwrap();
+        assert_eq!((raw.bars.close[8], raw.bars.close[9]), (108., 114.)); // unadjusted: the roll gap shows
+        let ratio = continuous(&chain(), Roll::DaysBefore(1), Adjust::Ratio).unwrap();
+        assert!((ratio.bars.close[9] - 114. * 108. / 113.).abs() < 1e-9);
+        let mut busy = chain();
+        busy[1].bars.volume = Some((0..20).map(|i| if i >= 5 { 2000. } else { 500. }).collect());
+        assert_eq!(continuous(&busy, Roll::Volume, Adjust::None).unwrap().rolls[0].0, 5); // B out-trades A from session 5
+        busy[1].oi = Some(vec![9e9; 20]);
+        assert_eq!(continuous(&busy, Roll::OpenInterest, Adjust::None).unwrap().rolls[0].0, 9, "A has no OI column: roll at expiry");
+    }
+    #[test] fn a_trade_across_a_roll_becomes_two_costed_trades_on_actual_prices() {
+        let spec = parse_spec(r#"{"v":1,"entry":{"all":[{"lhs":{"s":"close"},"op":"=","rhs":{"value":102}}]},"exit":{"any":[{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":10}}]},
+            "execution":{"fill":"close","capital":1000000,"sizing":{"units":1},"costs":{"commission":{"per_order":20}},"instrument":{"kind":"futures"}}}"#).unwrap();
+        let r = run(&chain(), &spec, 1_000_000.).unwrap();
+        let spans: Vec<_> = r.rows.iter().map(|(t, k, row)| (t.legs[0].i, t.legs[0].px, t.exit_i, t.exit_px, *k, row.qty, t.entry_kind)).collect();
+        assert_eq!(spans, vec![(2, 102., 8, 108., 0, 50., "close"), (8, 113., 12, 117., 1, 50., "roll")]);
+        // P&L = 50 × (6 + 4) less four order fees: the continuous move, with the roll's costs explicit
+        assert!((r.final_equity - 1_000_000. - (500. - 80.)).abs() < 1e-6);
+    }
+    #[test] fn roll_none_closes_at_expiry_and_expiry_operands_come_from_the_contract() {
+        let spec = parse_spec(r#"{"v":1,"entry":{"all":[{"lhs":{"s":"close"},"op":"=","rhs":{"value":102}}]},"exit":{"any":[],"stop_loss":{"ticks":400}},
+            "execution":{"fill":"close","capital":1000000,"sizing":{"units":1},"instrument":{"kind":"futures","roll":"none"}}}"#).unwrap();
+        let r = run(&chain(), &spec, 1_000_000.).unwrap();
+        assert_eq!((r.rows[0].0.exit_i, r.rows[0].0.reason.as_str()), (9, "expiry"));
+        let e = expiry(&r.series.bars, &Params::new()).unwrap();
+        assert_eq!((e[0][0], e[0][9], e[2][9], e[2][8], e[1][7]), (9., 0., 1., 0., 1.)); // 2024-01-10 (bar 7) is in the expiry week of 2024-01-12
+        assert_eq!(expiry(&Bars::default(), &Params::new()).unwrap_err(), "needs contract data");
+    }
+    #[test] fn a_margin_call_liquidates_at_the_next_open_and_stops_trading() {
+        let mut falling = contract(0, 0, 29, 0.);
+        for (i, v) in falling.bars.close.iter_mut().enumerate() { *v = 100. - i as f64; }
+        falling.bars.open = falling.bars.close.clone(); falling.bars.high = falling.bars.close.clone(); falling.bars.low = falling.bars.close.clone();
+        let spec = parse_spec(r#"{"v":1,"entry":{"all":[{"lhs":{"s":"close"},"op":"=","rhs":{"value":100}}]},"exit":{"any":[{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":20}}]},
+            "execution":{"fill":"close","capital":600,"sizing":{"units":1},"instrument":{"kind":"futures"}}}"#).unwrap();
+        let r = run(&[falling], &spec, 600.).unwrap();
+        // 50 units, 10% margin: at 97, equity 600 − 150 = 450 < 485 → liquidated at the next open (96)
+        assert_eq!((r.rows.len(), r.rows[0].0.exit_i, r.rows[0].0.exit_px, r.rows[0].0.reason.as_str()), (1, 4, 96., "margin_call"));
+        assert!(r.events[0].starts_with("margin call on"));
+    }
+    #[test] fn later_bars_never_change_earlier_series_or_trades() {
+        let spec = parse_spec(r#"{"v":1,"entry":{"all":[{"lhs":{"s":"close"},"op":">","rhs":{"s":"sma","p":{"length":3}}}]},"exit":{"any":[{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":4}}]},
+            "execution":{"capital":1000000,"sizing":{"units":1},"instrument":{"kind":"futures","adjust":"forward"}}}"#).unwrap();
+        let full = chain();
+        let want = run(&full, &spec, 1e6).unwrap();
+        for k in [10usize, 15, 22] {
+            let mut alt = full.clone();
+            for c in alt.iter_mut() {
+                let n = c.bars.date.len();
+                for j in 0..n { if c.bars.date[j] > want.series.bars.date[k] { for v in [&mut c.bars.open, &mut c.bars.high, &mut c.bars.low, &mut c.bars.close] { v[j] *= 0.8; } } }
+            }
+            let got = run(&alt, &spec, 1e6).unwrap();
+            assert_series(&got.series.bars.close[..=k], &want.series.bars.close[..=k]);
+            let done = |r: &FuturesRun| r.rows.iter().filter(|(t, _, _)| !t.open && t.exit_i <= k).map(|(t, _, row)| (t.legs[0].i, t.exit_i, t.exit_px.to_bits(), row.pnl.to_bits())).collect::<Vec<_>>();
+            assert_eq!(done(&got), done(&want), "bars after {k} changed earlier futures trades");
+        }
+    }
+}
PATCH
```

- [ ] **Step 4: Run all lab tests**

Run: `cd Context/guru/backend && CARGO_BUILD_JOBS=2 cargo test lab:: 2>&1 | grep 'test result'`
Expected: `ok. 341 passed; 0 failed` (276 at HEAD plus this phase's tests so far; a different count means a test is missing or a patch was not fully applied). `lab::pinned::existing_specs_keep_their_exact_trades` and `lab::templates::templates_reproduce_the_book_engine_on_fixtures` must be among the passing tests.

- [ ] **Step 5: Commit (only with the user's explicit permission for git writes)**

With permission: `git add Context/guru/backend/src/lab && git commit -m "feat(lab): futures contract model on fixtures; needs_data until contract data is admitted"` (no trailer, no push). Without it: stop here and report `git status --short` and `git diff --stat`.

---
### Task 15: Always-in specs; documented AmiBroker/TradingView examples; trade-level no-lookahead for every feature

Spec §6.1, §10.1, §10.3. A two-sided spec may leave exits empty (always in) when `opposite` is `reverse`. `examples.rs` reproduces six documented examples on hand-computed fixtures:

- E1: AmiBroker stop-and-reverse MA cross with trade delays;
- E2: the AmiBroker KB limit order;
- E3: ApplyStop percent stop and N-bar stop;
- E4: TradingView Channel BreakOut;
- E5: TradingView Consecutive Up/Down;
- E6: the Pine `strategy.exit` bracket and trailing in ticks, in both intrabar modes.

One trade-level test perturbs every bar after k for six specs that use every Phase 7 feature.

**Files:**
- Create: `Context/guru/backend/src/lab/examples.rs`
- Modify: `Context/guru/backend/src/lab/mod.rs`
- Modify: `Context/guru/backend/src/lab/rules.rs`

**Interfaces:**
- Consumes: Tasks 1–14.
- Produces: `lab::examples` (test-only); `rules::side(…, both, …)`.

> Note: The E4 and E5 rules are TradingView built-in strategies, quoted from memory. Before committing, open "Channel BreakOut Strategy" and "Consecutive Up/Down Strategy" in the Pine editor and compare the entry calls. If they differ, change the fixture's spec to match (never the engine), and report the difference.

> Note: `examples.rs` is a new test-only module.

- [ ] **Step 1: Write the failing tests** (apply from the repository root)

```bash
git apply --whitespace=nowarn <<'PATCH'
diff --git a/Context/guru/backend/src/lab/rules.rs b/Context/guru/backend/src/lab/rules.rs
--- a/Context/guru/backend/src/lab/rules.rs
+++ b/Context/guru/backend/src/lab/rules.rs
@@ -1412,4 +1418,12 @@
             assert!(e.message().contains(needle), "{needle:?} not in {:?}", e.message());
         }
     }
-}
+
+    #[test] fn always_in_specs_need_reversal() {
+        let always = r#"{"v":1,"direction":"both","entry":{"all":[{"lhs":{"s":"close"},"op":"crosses_above","rhs":{"s":"sma","p":{"length":3}}}]},"exit":{"any":[]},
+            "short_side":{"entry":{"all":[{"lhs":{"s":"close"},"op":"crosses_below","rhs":{"s":"sma","p":{"length":3}}}]},"exit":{"any":[]}}}"#;
+        assert!(parse_spec(always).unwrap().exit.is_empty());
+        assert!(err(&always.replace("}}}}]}}}", r#"}}}]}},"execution":{"opposite":"exit"}}"#).replacen(r#"]},"exit":{"any":[]}}}"#, r#"]},"exit":{"any":[]}},"execution":{"opposite":"exit"}}"#, 1)).message().contains("needs execution.opposite \"reverse\""));
+        assert!(err(r#"{"v":1,"entry":{"all":[{"lhs":{"s":"close"},"op":">","rhs":{"value":1}}]},"exit":{"any":[]}}"#).message().contains("needs at least one condition"));
+    }
+}
diff --git a/Context/guru/backend/src/lab/mod.rs b/Context/guru/backend/src/lab/mod.rs
--- a/Context/guru/backend/src/lab/mod.rs
+++ b/Context/guru/backend/src/lab/mod.rs
@@ -44,3 +44,5 @@
 pub mod parity;
 #[cfg(test)]
 pub mod pinned;
+#[cfg(test)]
+pub mod examples;
diff --git a/Context/guru/backend/src/lab/examples.rs b/Context/guru/backend/src/lab/examples.rs
new file mode 100644
--- /dev/null
+++ b/Context/guru/backend/src/lab/examples.rs
@@ -0,0 +1,122 @@
+//! Documented-example fixtures (Phase 7 spec §10.3): published AmiBroker and TradingView rules written as lab specs, run on
+//! crafted bars, with the trades the platform's documented semantics give, computed by hand. They show that the lab
+//! expresses these rules the platforms' way; they verify no book and no platform output (nothing is exported from either).
+//! Plus a trade-level no-lookahead test for every Phase 7 feature.
+use super::bars::Bars;
+use super::rules::parse_spec;
+use super::series::{compile, Cache, Frames};
+use super::simulate::simulate;
+use super::testdata::{closes, from_ohlc};
+use crate::connors::Trade;
+
+fn run(spec: &str, b: &Bars) -> Vec<Trade> { simulate(&compile(&parse_spec(spec).unwrap(), &Frames::new(b.clone()), &mut Cache::new()).unwrap(), b, 0, b.len() - 1) }
+/// (entry bar, entry price, exit bar, exit price, direction, open)
+fn rows(t: &[Trade]) -> Vec<(usize, f64, usize, f64, f64, bool)> { t.iter().map(|t| (t.legs[0].i, (t.legs[0].px * 100.).round() / 100., t.exit_i, (t.exit_px * 100.).round() / 100., t.dir, t.open)).collect() }
+
+/// E1 — AmiBroker Backtesting guide, stop-and-reverse: `Buy = Cross(C, MA(C, 3)); Sell = Cross(MA(C, 3), C); Short = Sell;
+/// Cover = Buy;` with SetTradeDelays(1, 1, 1, 1) and BuyPrice = Open (https://www.amibroker.com/guide/h_backtest.html).
+#[test] fn e1_amibroker_stop_and_reverse_ma_cross() {
+    let spec = r#"{"v":1,"direction":"both","entry":{"all":[{"lhs":{"s":"close"},"op":"crosses_above","rhs":{"s":"sma","p":{"length":3}}}]},"exit":{"any":[]},
+        "short_side":{"entry":{"all":[{"lhs":{"s":"close"},"op":"crosses_below","rhs":{"s":"sma","p":{"length":3}}}]},"exit":{"any":[]}}}"#;
+    // crosses above on bar 3, below on bar 5, above on bar 8; fills at the next open (open = close here)
+    let t = run(spec, &closes(&[10., 10., 10., 13., 16., 13., 10., 7., 10., 13., 16.]));
+    assert_eq!(rows(&t), vec![(4, 16., 6, 10., 1., false), (6, 10., 9, 13., -1., false), (9, 13., 10, 16., 1., true)]);
+}
+/// E2 — AmiBroker KB "Handling limit orders in the backtester": `Buy = Ref(cond, -1) AND L < limit; BuyPrice = Min(O, limit)`
+/// with limit = 1% below the signal close (https://www.amibroker.com/kb/2014/11/26/handling-limit-orders-in-the-backtester/).
+/// The fixture avoids L == limit (AmiBroker's `<` against the lab's touch).
+#[test] fn e2_amibroker_limit_order_fills_at_the_limit_or_the_gapped_open() {
+    let mut bars = vec![(100., 101., 99., 100.); 10];
+    bars[3] = (99.5, 100., 98.5, 99.8); // touches the 99 limit: filled at 99
+    bars[7] = (97., 98., 96., 97.5); // opens below the limit: filled at the open, 97
+    let spec = r#"{"v":1,"entry":{"all":[{"any":[{"lhs":{"s":"calendar","out":"bar_index"},"op":"=","rhs":{"value":2}},{"lhs":{"s":"calendar","out":"bar_index"},"op":"=","rhs":{"value":6}}]}],"limit":{"pct":1}},
+        "exit":{"any":[{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":2}}]},"execution":{"exit_fill":"close"}}"#;
+    assert_eq!(rows(&run(spec, &from_ohlc(&bars))), vec![(3, 99., 5, 100., 1., false), (7, 97., 9, 100., 1., false)]);
+}
+/// E3 — AmiBroker ApplyStop (https://www.amibroker.com/guide/afl/applystop.html): `ApplyStop(stopTypeLoss, stopModePercent, 2,
+/// 1)` exits intraday at the stop level, or at the open when the bar gaps through it (forum t/39477); `ApplyStop(stopTypeNBar,
+/// stopModeBars, 5)` exits after 5 bars. Buy on the close (SetTradeDelays(0, 0, 0, 0), BuyPrice = Close).
+#[test] fn e3_amibroker_percent_stop_and_n_bar_stop() {
+    let mut bars = vec![(100., 101., 99., 100.); 15];
+    bars[3] = (99.5, 100., 97.5, 98.5); // touches the 98 stop
+    bars[6] = (96., 97., 95., 96.5); // gaps below the 98 stop: out at the open
+    let spec = r#"{"v":1,"entry":{"all":[{"any":[{"lhs":{"s":"calendar","out":"bar_index"},"op":"=","rhs":{"value":1}},{"lhs":{"s":"calendar","out":"bar_index"},"op":"=","rhs":{"value":5}},{"lhs":{"s":"calendar","out":"bar_index"},"op":"=","rhs":{"value":8}}]}]},
+        "exit":{"any":[{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":5}}],"stop_loss":{"pct":2}},"execution":{"fill":"close"}}"#;
+    let t = run(spec, &from_ohlc(&bars));
+    assert_eq!(rows(&t), vec![(1, 100., 3, 98., 1., false), (5, 100., 6, 96., 1., false), (8, 100., 13, 100., 1., false)]);
+    assert_eq!((t[1].reason.as_str(), t[2].reason.as_str()), ("exit.stop_loss (gap at the open)", "exit.any[0]: bars_held >= 5"));
+}
+/// E4 — TradingView built-in "Channel BreakOut Strategy" (length 5): every bar, `strategy.entry("ChBrkLE", long, stop =
+/// ta.highest(high, 5) + mintick)` once `close[5]` exists, and `strategy.entry("ChBrkSE", short, stop = ta.lowest(low, 5) -
+/// mintick)`; an opposite entry reverses. Quoted from the built-in's source (check it in the Pine editor); mintick 0.01.
+#[test] fn e4_tradingview_channel_breakout() {
+    let mut bars = vec![(100., 101., 99., 100.); 6];
+    bars.extend([(100., 103., 99.5, 102.5), (102.5, 104., 101., 103.), (103., 103.5, 100., 100.5), (100.5, 101., 98., 98.5), (98.5, 99., 97., 97.5),
+                 (97.5, 98., 96., 96.5), (96.5, 97., 95., 95.5), (95.5, 96., 94., 95.), (95., 102., 94.5, 101.5), (101.5, 102., 101., 101.8)]);
+    let spec = r#"{"v":1,"direction":"both",
+        "entry":{"all":[{"lhs":{"s":"close","offset":5},"op":">","rhs":{"value":0}}],"stop":{"at":{"add":[{"s":"highest","p":{"length":5}},{"value":0.01}]}}},"exit":{"any":[]},
+        "short_side":{"entry":{"all":[{"lhs":{"s":"close"},"op":">","rhs":{"value":0}}],"stop":{"at":{"sub":[{"s":"lowest","p":{"length":5}},{"value":0.01}]}}},"exit":{"any":[]}}}"#;
+    let t = run(spec, &from_ohlc(&bars));
+    // long stop 101.01 fills on bar 6; the short stop 98.99 reverses on bar 9; the long stop 101.01 (from bar 13) reverses on bar 14
+    assert_eq!(rows(&t), vec![(6, 101.01, 9, 98.99, 1., false), (9, 98.99, 14, 101.01, -1., false), (14, 101.01, 15, 101.8, 1., true)]);
+    assert_eq!((t[0].entry_kind, t[1].entry_kind), ("stop", "reverse"));
+}
+/// E5 — TradingView built-in "Consecutive Up/Down Strategy" (3 and 3): long after 3 higher closes, short after 3 lower closes,
+/// market orders at the next open, reversing. `ups >= 3` is a strictly rising run of 3 (lab `rising`, bars 3).
+#[test] fn e5_tradingview_consecutive_up_down() {
+    let spec = r#"{"v":1,"direction":"both","entry":{"all":[{"lhs":{"s":"close"},"op":"rising","bars":3}]},"exit":{"any":[]},
+        "short_side":{"entry":{"all":[{"lhs":{"s":"close"},"op":"falling","bars":3}]},"exit":{"any":[]}}}"#;
+    let t = run(spec, &closes(&[10., 11., 12., 13., 14., 13., 12., 11., 10., 11., 12., 13.]));
+    assert_eq!(rows(&t), vec![(4, 14., 8, 10., 1., false), (8, 10., 11, 13., -1., true)]); // the last bar's long signal has no fill
+}
+/// E6 — Pine User Manual, Strategies: `strategy.exit(profit = 100, loss = 50)` in ticks, and `trail_points` / `trail_offset`
+/// (https://www.tradingview.com/pine-script-docs/concepts/strategies/), on a futures fixture with tick 0.05. TradingView's
+/// broker emulator takes the open's nearer extreme first; the lab's default takes the stop first (spec §6.3).
+#[test] fn e6_tradingview_bracket_and_trailing_in_ticks() {
+    use crate::lab::futures::{run as fut, Contract};
+    let contract = |rows: &[(f64, f64, f64, f64)]| {
+        let b = from_ohlc(rows);
+        Contract { symbol: "F".into(), expiry: b.date[b.len() - 1].clone(), lot: 1., tick: 0.05, point_value: 1., margin_pct: 10., bars: b, oi: None }
+    };
+    let bracket = |intrabar: &str| format!(r#"{{"v":1,"entry":{{"all":[{{"lhs":{{"s":"calendar","out":"bar_index"}},"op":"=","rhs":{{"value":1}}}}]}},"exit":{{"any":[],"target":{{"ticks":100}},"stop_loss":{{"ticks":50}}}},
+        "execution":{{"fill":"close","intrabar":"{intrabar}","capital":100000,"sizing":{{"units":1}},"instrument":{{"kind":"futures"}}}}}}"#);
+    let c = contract(&[(100., 100., 100., 100.), (100., 100., 100., 100.), (104., 106., 97., 102.), (102., 102., 102., 102.)]);
+    // entry 100: target 105, stop 97.5; bar 2 opens nearer its high (O → H → L → C)
+    let tv = fut(&[c.clone()], &parse_spec(&bracket("ohlc_path")).unwrap(), 1e5).unwrap();
+    assert_eq!((tv.rows[0].0.exit_px, tv.rows[0].0.reason.as_str()), (105., "exit.target"));
+    let lab = fut(&[c], &parse_spec(&bracket("stop_first")).unwrap(), 1e5).unwrap();
+    assert_eq!((lab.rows[0].0.exit_px, lab.rows[0].0.reason.as_str()), (97.5, "exit.stop_loss"));
+    // trail_points 60 ticks (3 points) arms it, trail_offset 40 ticks (2 points) trails; bar 3 opens nearer its low, so
+    // TradingView meets the low (101) before a new high, against the trail from bar 2's high: 103.5 − 2 = 101.5
+    let trail = r#"{"v":1,"entry":{"all":[{"lhs":{"s":"calendar","out":"bar_index"},"op":"=","rhs":{"value":1}}]},"exit":{"any":[],"trailing":{"ticks":40,"activate":{"ticks":60}}},
+        "execution":{"fill":"close","capital":100000,"sizing":{"units":1},"instrument":{"kind":"futures"}}}"#;
+    let c = contract(&[(100., 100., 100., 100.), (100., 100., 100., 100.), (100., 103.5, 99.5, 103.), (102., 104., 101., 103.), (103., 103., 103., 103.)]);
+    let r = fut(&[c], &parse_spec(trail).unwrap(), 1e5).unwrap();
+    assert_eq!((r.rows[0].0.exit_i, r.rows[0].0.exit_px, r.rows[0].0.reason.as_str()), (3, 101.5, "exit.trailing"));
+}
+
+/// Every Phase 7 order, exit, re-entry and operand feature: perturbing every bar after k never changes a trade closed by k.
+#[test] fn phase7_features_never_use_later_bars() {
+    let rsi_lo = r#"{"lhs":{"s":"rsi","p":{"length":2}},"op":"<","rhs":{"value":25}}"#;
+    let rsi_hi = r#"{"lhs":{"s":"rsi","p":{"length":2}},"op":">","rhs":{"value":75}}"#;
+    let held = |n: u32| format!(r#"{{"lhs":{{"s":"bars_held"}},"op":">=","rhs":{{"value":{n}}}}}"#);
+    let specs = [
+        format!(r#"{{"v":1,"direction":"both","entry":{{"all":[{rsi_lo}]}},"exit":{{"any":[]}},"short_side":{{"entry":{{"all":[{rsi_hi}]}},"exit":{{"any":[]}}}}}}"#),
+        format!(r#"{{"v":1,"direction":"both","entry":{{"all":[{rsi_lo}],"stop":{{"at":{{"s":"highest","p":{{"length":3}}}}}},"valid_bars":3}},"exit":{{"any":[{}]}},"short_side":{{"entry":{{"all":[{rsi_hi}],"stop":{{"pct":0.5}},"limit":{{"pct":0}}}},"exit":{{"any":[{}]}}}}}}"#, held(6), held(6)),
+        format!(r#"{{"v":1,"entry":{{"all":[{rsi_lo}]}},"exit":{{"any":[{}]}},"execution":{{"delay_bars":2,"entry_on":"edge"}}}}"#, held(4)),
+        format!(r#"{{"v":1,"entry":{{"all":[{rsi_lo}]}},"exit":{{"any":[{}],"stop_loss":{{"pct":3}},"trailing":{{"pct":2,"activate":{{"pct":1}}}},"breakeven":{{"after":{{"pct":1.5}}}},"take_profits":[{{"pct":1,"size":0.3}},{{"atr":2,"size":0.3}}]}},"execution":{{"fill":"close","intrabar":"ohlc_path"}}}}"#, held(12)),
+        format!(r#"{{"v":1,"entry":{{"all":[{rsi_lo}],"scale_in":{{"legs":[0.4,0.3,0.3],"price":"better"}}}},"exit":{{"any":[{rsi_hi}],"stop_loss":{{"pct":6}}}},"execution":{{"sizing":{{"risk_pct":3}},"cooldown":{{"bars":3,"after":"loss"}},"max_trades":{{"count":2,"per":"M"}},"risk":{{"max_consecutive_losses":2,"pause_bars":10}}}}}}"#),
+        format!(r#"{{"v":1,"entry":{{"all":[{{"then":[{rsi_lo},{{"lhs":{{"s":"close"}},"op":">","rhs":{{"s":"close","tf":"Q","offset":1,"expand":"first"}}}}],"within":10}},{{"lhs":{{"bars_since":{rsi_hi}}},"op":">","rhs":{{"value":2}}}}]}},"exit":{{"any":[{{"lhs":{{"sum":{{"s":"rsi","p":{{"length":2}}}},"bars":2}},"op":">","rhs":{{"value":140}}}},{}]}}}}"#, held(15)),
+    ];
+    let full = super::testdata::walk(400, 7, false);
+    let close_by = |t: &[Trade], k: usize| t.iter().filter(|t| !t.open && t.exit_i <= k).map(|t| (t.legs.iter().map(|l| (l.i, l.px.to_bits())).collect::<Vec<_>>(), t.exit_i, t.exit_px.to_bits())).collect::<Vec<_>>();
+    for text in &specs {
+        let want = run(text, &full);
+        assert!(want.len() > 3, "{text}: only {} trades", want.len());
+        for k in (30usize..398).step_by(3) {
+            let mut alt = full.clone();
+            for i in k + 1..alt.len() { for v in [&mut alt.open, &mut alt.high, &mut alt.low, &mut alt.close] { v[i] *= 0.7; } }
+            assert_eq!(close_by(&run(text, &alt), k), close_by(&want, k), "{text}: bars after {k} changed earlier trades");
+        }
+    }
+}
PATCH
```

- [ ] **Step 2: Run the tests and confirm the failure**

Run: `cd Context/guru/backend && CARGO_BUILD_JOBS=2 cargo test lab:: 2>&1 | tail -20`
Expected: `always_in_specs_need_reversal` fails with "needs at least one condition", and E1, E4 and E5 fail to parse.

- [ ] **Step 3: Implement** (apply from the repository root)

```bash
git apply --whitespace=nowarn <<'PATCH'
diff --git a/Context/guru/backend/src/lab/rules.rs b/Context/guru/backend/src/lab/rules.rs
--- a/Context/guru/backend/src/lab/rules.rs
+++ b/Context/guru/backend/src/lab/rules.rs
@@ -217,12 +217,12 @@
     };
     let origin = root.get("origin").map(origin).transpose()?;
     let mut budget = Budget::default();
-    let (entry, exit) = side(root.get("entry"), root.get("exit"), "", &mut exec, &mut budget)?;
+    let (entry, exit) = side(root.get("entry"), root.get("exit"), "", both, &mut exec, &mut budget)?;
     let mut short_side = match (both, root.get("short_side")) {
         (true, Some(s)) => {
             fields(s, "short_side", &["entry", "exit"], &[])?;
             let mut x = Exec { short: true, ..Exec::default() };
-            let (entry, exit) = side(s.get("entry"), s.get("exit"), "short_side.", &mut x, &mut budget)?;
+            let (entry, exit) = side(s.get("entry"), s.get("exit"), "short_side.", true, &mut x, &mut budget)?;
             Some(Box::new(Side { entry, exit, exec: x }))
         }
         (true, None) => return Err(inv("direction \"both\" needs short_side with the short entry and exit rules")),
@@ -317,6 +317,11 @@
     if let Some(s) = short_side.as_mut() {
         (s.exec.fill, s.exec.exit_fill, s.exec.opposite, s.exec.delay, s.exec.edge, s.exec.ohlc_path) = (exec.fill, exec.exit_fill, exec.opposite, exec.delay, exec.edge, exec.ohlc_path);
     }
+    // always in (AFL Short = Sell; Cover = Buy): a side without exits leaves only through the reversal
+    let exitless = |x: &Exec, e: &[Node]| e.is_empty() && x.stop_loss.is_none() && x.target.is_none() && x.trailing.is_none() && x.breakeven.is_none();
+    if both && exec.opposite != Opposite::Reverse && (exitless(&exec, &exit) || short_side.as_ref().is_some_and(|s| exitless(&s.exec, &s.exit))) {
+        return Err(inv("a side without exit rules (always in) needs execution.opposite \"reverse\""));
+    }
     let execs: Vec<&Exec> = std::iter::once(&exec).chain(short_side.as_ref().map(|s| &s.exec)).collect();
     if exec.futures.is_none() && execs.iter().any(|x| dists(x).iter().any(|d| matches!(d, Dist::Ticks(_)))) {
         return Err(inv("ticks need a futures contract's tick size (execution.instrument kind futures)"));
@@ -333,7 +338,8 @@
 }
 
 /// One side's rule lists and price orders; `pre` is "" (the long side, or the only side) or "short_side.".
-fn side(entry: Option<&Json>, exit: Option<&Json>, pre: &str, exec: &mut Exec, b: &mut Budget) -> Result<(Vec<Node>, Vec<Node>), LabError> {
+/// `both`: a two-sided spec may leave exits empty (always in: the reversal is the exit; checked against `opposite` later).
+fn side(entry: Option<&Json>, exit: Option<&Json>, pre: &str, both: bool, exec: &mut Exec, b: &mut Budget) -> Result<(Vec<Node>, Vec<Node>), LabError> {
     let entry_obj = entry.ok_or_else(|| inv(format!("{pre}entry is required")))?;
     fields(entry_obj, &format!("{pre}entry"), &["all", "limit", "stop", "valid_bars", "scale_in"], &[])?;
     if let Some(v) = entry_obj.get("scale_in") { exec.scale_in = Some(scale_in(v, &format!("{pre}entry.scale_in"))?); }
@@ -383,7 +389,7 @@
     }
     let price_exit = exec.stop_loss.is_some() || exec.target.is_some() || exec.trailing.is_some() || exec.breakeven.is_some();
     let entry = conds(entry_obj.get("all"), &format!("{pre}entry.all"), false, false, b)?;
-    let exit = conds(exit_obj.get("any"), &format!("{pre}exit.any"), true, price_exit, b)?;
+    let exit = conds(exit_obj.get("any"), &format!("{pre}exit.any"), true, price_exit || both, b)?;
     if entry.len() + exit.len() > MAX_CONDITIONS {
         return Err(inv(format!("{}{} conditions; the limit is {MAX_CONDITIONS}", if pre.is_empty() { "" } else { "short_side: " }, entry.len() + exit.len())));
     }
PATCH
```

- [ ] **Step 4: Run all lab tests**

Run: `cd Context/guru/backend && CARGO_BUILD_JOBS=2 cargo test lab:: 2>&1 | grep 'test result'`
Expected: `ok. 349 passed; 0 failed` (276 at HEAD plus this phase's tests so far; a different count means a test is missing or a patch was not fully applied). `lab::pinned::existing_specs_keep_their_exact_trades` and `lab::templates::templates_reproduce_the_book_engine_on_fixtures` must be among the passing tests.

- [ ] **Step 5: Commit (only with the user's explicit permission for git writes)**

With permission: `git add Context/guru/backend/src/lab && git commit -m "test(lab): documented AmiBroker/TradingView examples and Phase 7 no-lookahead; always-in specs"` (no trailer, no push). Without it: stop here and report `git status --short` and `git diff --stat`.

---
### Task 16: Phase gate

**Files:** none, except fixes the gate finds. Each fix gets its own test before its code, in the task that owns it.

- [ ] **Step 1: Engine suite (real NIFTY data).**
  `cd Context/guru/backend && VAULT_IDX="/run/user/1000/gvfs/smb-share:server=192.168.4.50,share=share/Manan Pujara/Book-to-rust/idx_data.csv" CARGO_BUILD_JOBS=2 cargo test 2>&1 | grep 'test result'`
  → all pass. At planning this was `920 passed; 1 ignored` (`848; 1` at `$BASE`, +73 lab tests). Record both counts.
- [ ] **Step 2: Phase 5 gl data in the engine tests.**
  `NAS="/run/user/1000/gvfs/smb-share:server=192.168.4.50,share=share/Manan Pujara/Book-to-rust"; GLCSV=$(mktemp --suffix=.csv); python3 Context/guru/webapp/scripts/stage_gl.py "$NAS" "$GLCSV"`
  This reads the NAS and writes only `$GLCSV`. Then:
  `cd Context/guru/backend && VAULT_GL_CSV="$GLCSV" VAULT_IDX="$NAS/idx_data.csv" CARGO_BUILD_JOBS=2 cargo test lab:: 2>&1 | grep 'test result'`
  → 349 passed, including the Phase 5 real-data check `staged_gl_data_matches_the_declared_cuts`.
- [ ] **Step 3: Customize fidelity and the pin.**
  `cd Context/guru/backend && CARGO_BUILD_JOBS=2 cargo test lab::templates lab::pinned 2>&1 | grep -E 'test result|FAILED'`
  → all pass. All 43 templates reproduce the book engine on the four fixtures, and the pinned trade hashes are unchanged.
- [ ] **Step 4: Scripts.**
  `python3 -m unittest discover -s scripts -p 'test_*.py'` and `python3 scripts/check_upload.py` → both pass. The gateway is untouched; confirm with
  `cd Context/guru/webapp && CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="${VAULT_RUNTIME_ROOT:-$HOME/.cache/strategy-vault-webapp}/test-target" cargo test --locked --manifest-path gateway/Cargo.toml 2>&1 | grep 'test result'`
  → the same count as at `$BASE`.
- [ ] **Step 5: Book guard with engine compare.**
  `python3 scripts/check_book_untouched.py --base origin/Manan --engine-compare --csv "$NAS/idx_data.csv" --vix "$NAS/india_vix" --extra "$NAS/extra_data"`
  → `"problems": []`. Copy the `notes` into the report.
- [ ] **Step 6: Release build without lab warnings.**
  `cd Context/guru/backend && touch src/main.rs && CARGO_BUILD_JOBS=2 cargo build --release 2>&1 | grep -A3 '^warning' | grep -- '--> src/lab/'` → no output.
- [ ] **Step 7: Real-data smoke on India data** (bounded, NAS read-only, local port, one instrument per request).

```bash
cd Context/guru/backend
NAS="/run/user/1000/gvfs/smb-share:server=192.168.4.50,share=share/Manan Pujara/Book-to-rust"
PORT=$(python3 -c 'import socket;s=socket.socket();s.bind(("127.0.0.1",0));print(s.getsockname()[1])')
./target/release/guru "$NAS/idx_data.csv" "$PORT" "$NAS/india_vix" "$NAS/extra_data/india" >/dev/null 2>&1 & PID=$!
until curl -s -o /dev/null "http://127.0.0.1:$PORT/api/gurus"; do sleep 0.5; done
A="http://127.0.0.1:$PORT/api/lab"
enc() { python3 -c 'import sys,urllib.parse;print(urllib.parse.quote(sys.argv[1]))' "$1"; }
call() { curl -s -w ' %{http_code}\n' "$A/run?ticker=NIFTY%2050&spec=$(enc "$1")" | python3 -c 'import sys;t=sys.stdin.read();print(t[:160].replace("\n"," "), "…", t[-5:].strip())'; }
curl -s "$A/catalogue" | python3 -c 'import sys,json;d=json.load(sys.stdin);print(len(d["indicators"]),d["max_leaves"],d["max_nodes"],d["timeframes"],d["custom_days"])'
call '{"v":1,"direction":"both","entry":{"all":[{"lhs":{"s":"close"},"op":"crosses_above","rhs":{"s":"sma","p":{"length":20}}}]},"exit":{"any":[]},"short_side":{"entry":{"all":[{"lhs":{"s":"close"},"op":"crosses_below","rhs":{"s":"sma","p":{"length":20}}}]},"exit":{"any":[]}}}'
call '{"v":1,"entry":{"all":[{"lhs":{"s":"close"},"op":">","rhs":{"s":"close","tf":"Q","offset":1,"expand":"first"}},{"lhs":{"sum":{"s":"rsi","p":{"length":2}},"bars":3},"op":"<","rhs":{"value":50}}]},"exit":{"any":[{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":5}}],"take_profits":[{"pct":2,"size":0.5}],"stop_loss":{"pct":4}},"execution":{"fill":"close"}}'
call '{"v":1,"entry":{"all":[{"lhs":{"s":"close"},"op":">","rhs":{"s":"close","tf":"W","expand":"first"}}]},"exit":{"any":[{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":5}}]}}'
call '{"v":1,"entry":{"all":[{"lhs":{"s":"rsi"},"op":"<","rhs":{"value":30}}]},"exit":{"any":[{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":5}}]},"execution":{"capital":100000,"sizing":{"units":1}}}'
call '{"v":1,"entry":{"all":[{"lhs":{"s":"rsi"},"op":"<","rhs":{"value":30}}]},"exit":{"any":[{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":5}}]},"execution":{"instrument":{"kind":"futures"}}}'
call '{"v":1,"entry":{"all":[{"lhs":{"s":"rsi"},"op":"<","rhs":{"value":30}}]},"exit":{"any":[{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":5}}]},"execution":{"instrument":{"kind":"futures","adjust":"back"}}}'
curl -s "$A/fidelity?method=rsi2_4&ticker=NIFTY%2050" | python3 -c 'import sys,json;d=json.load(sys.stdin);print(d["reproduces"],d["matched"],d["book_trades"])'
kill $PID
```
Expected, in order:
- `302 40 120 ['D', 'W', 'M', 'Q', 'Y'] [2, 63]`;
- the two-sided run → 200, starting `{"label":…,"execution_model":{"kind":"index","mode":"unit","executable":false,"notes":["An index cannot be traded…`;
- the Q-operand / sum / take-profit run → 200;
- weekly `expand: first` at offset 0 → 400 `unsupported` "…expandFirst would show the period's final value…";
- money mode on the index → 400 `needs_data` "an index has no tradable quantity…";
- futures → 400 `needs_data` "futures contract data for NIFTY 50 … is not admitted …";
- `adjust: back` → 400 `unsupported` "…rewrites earlier prices at every roll (lookahead)…";
- fidelity → `True N N` (the template still reproduces the book engine on real data).

- [ ] **Step 8: Phase 5 gl smoke (provisional crypto).** Use the same release binary and `$GLCSV` from Step 2:

```bash
cd Context/guru/backend
PORT=$(python3 -c 'import socket;s=socket.socket();s.bind(("127.0.0.1",0));print(s.getsockname()[1])')
VAULT_LAB_MARKET=gl ./target/release/guru "$GLCSV" "$PORT" /nonexistent-vix /nonexistent-extra >/dev/null 2>&1 & PID=$!
until curl -s -o /dev/null "http://127.0.0.1:$PORT/api/lab/catalogue"; do sleep 0.5; done
A="http://127.0.0.1:$PORT/api/lab"
enc() { python3 -c 'import sys,urllib.parse;print(urllib.parse.quote(sys.argv[1]))' "$1"; }
SHORT='{"v":1,"direction":"short","entry":{"all":[{"lhs":{"s":"rsi"},"op":">","rhs":{"value":70}}]},"exit":{"any":[{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":5}}]}}'
curl -s "$A/run?ticker=CRYPTO:BTCUSD&spec=$(enc "$SHORT")" | python3 -c 'import sys,json;d=json.load(sys.stdin);print(d["provisional"],d["execution_model"]["kind"],d["execution_model"]["executable"],len(d["execution_model"]["notes"]))'
curl -s -w ' %{http_code}\n' "$A/run?ticker=CRYPTO:BTCUSD&spec=$(enc '{"v":1,"entry":{"all":[{"lhs":{"s":"rsi"},"op":"<","rhs":{"value":30}}]},"exit":{"any":[{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":5}}]},"execution":{"capital":1000,"sizing":{"value":500}}}')" | tail -c 200
kill $PID
```
Expected: `True crypto False 2` (composite note + short note), then a 400 `needs_data` body ("…TradingView index composite…") that carries `"provisional":true`.
- [ ] **Step 9: Scope proof.**
  - `git diff $BASE --stat` lists only `Context/guru/backend/src/lab/*` and the two docs.
  - `git diff $BASE -- Context/guru/backend/src/main.rs Context/guru/webapp Context/guru/frontend` → no output.
  - `git diff origin/Manan --stat -- Context/guru/backend/src/connors.rs Context/guru/backend/src/books Context/guru/backend/src/accounting.rs Context/guru/backend/src/trade_validation.rs` → no output.
- [ ] **Step 10: Report** (commit gate fixes only with permission).
  - Test counts before and after, guard notes, the smoke outputs.
  - The assumptions the user should look at first (spec §13): 3, 10, 12, 16, 20, 24, 29, 31, 33, 35, 38a, and the planning decisions above.
  - Open items:
    - check the E4/E5 Pine sources in the editor;
    - futures data admission (spec §12 item 1) before any real futures result;
    - the rule-builder UI for the new nodes and options in the frontend phase;
    - Phase 8 multi-symbol through the `symbol` seam.

  Do not push; the user pushes.

---

## Self-review (run while writing this plan)

**Spec coverage.**

| Spec | Task |
|---|---|
| §4.1 nodes and operators | 2 |
| §4.1 expressions | 3 |
| §4.1 state operands | 4 |
| §4.2 three-valued logic | 2 (tests `nodes_use_three_valued_logic`) |
| §4.3 limits | 2 (`whole_spec_bounds_…`); top-level 12 unchanged; catalogue limits in 12 |
| §4.4 conventions | 2–4 tests; `rising` strict in 2; E5 in 15 |
| §5.1 AmiBroker timeframes, Phase 8 seam | 5; seam in 3 |
| §5.2 calendar | 6; expiry operands in 14 |
| §6.1 two sides, opposite, always in | 7, 15 |
| §6.1 orders, delay, edge | 8 |
| §6.1 exits | 9 |
| §6.1 scale-in | 10 |
| §6.1 re-entry and risk | 11 |
| §6.2 bar event order | 7–9 tests |
| §6.3 intrabar modes | 9, E6 in 15 |
| §6.4 signal cleaning | 8 (`edge_entries_…`) |
| §6.5 equity-level state | 11 |
| §7.1 cash equity | 12–13 |
| §7.2 index / crypto / CFD | 12 |
| §7.3 futures | 14 |
| §7.4 sizing and costs | 10 (unit), 13 (money) |
| §8 compatibility | 1 (pin) plus every task's run; reserved fields in 8 |
| §9 API | 5, 12, 13, 14 |
| §10.1 no lookahead | 2–5, 14, 15 |
| §10.3 examples | 15 |
| §10.4 gate | 16 |

**Not implemented, as the spec decides:** options, intraday, SLB, perpetual funding, dated statutory costs, portfolio/multi-symbol (Phase 8), and new templates. These are the spec §12 follow-ups.

**Placeholder scan:** every code step is a complete patch, and every run states its expected result.

**Type consistency:** names were checked by compiling every task in order (Planning evidence).
