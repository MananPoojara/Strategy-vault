# Customize Book Strategies (backend) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let a researcher start a user variant from an admitted book method: 43 book templates (declared adaptations into lab rule specs), the engine fields they need (short, close fills, limit entries, stop-loss / target / trailing), a display-only fidelity check against the book engine, variant labels and instrument rules on lab runs, and lineage (`origin`) stored with saved strategies — without touching any book code or result.

**Architecture:** All engine work is inside `Context/guru/backend/src/lab/` (already dispatched by `main.rs` through the `/api/lab/` prefix, so `main.rs` does not change): `rules.rs` accepts the v1 fields that were reserved, `series.rs`/`simulate.rs` simulate them, new `fidelity.rs` compares trade lists, new `templates.rs` holds the templates and the "not available" reasons, and `api.rs` adds three routes and origin handling. Book code is only called (`connors::catalogue`, `connors::run`, `gate::get`, `instrument_policy::policy`, `readiness::check`, `main.rs`'s private `native_admission`/`window_start`/`resolve_context`/`catalogue_json`, reachable from the child module). The gateway gains three allowlist lines; the auth service validates and stores `spec.origin` (new `db/003-lab-origin.sql`). Frontend work (Customize button, pre-filled rule builder, fidelity panel, variant badges) is spec §11 and belongs to the later frontend phase, not this plan.

**Tech Stack:** Rust 2021, std only (engine); Rust gateway (`axum`, `serde_json`, existing lockfile); Node 22 ESM auth service (`hono`, `pg`, existing lockfile); PostgreSQL 18; `cargo test`, `node --test`.

**Spec:** `docs/superpowers/specs/2026-10-16-customize-book-strategies-design.md` (read §4–§9 and §12 before starting). Parent: `docs/superpowers/specs/2026-10-14-indicators-lab-design.md`. Format follows `docs/superpowers/plans/2026-10-15-indicators-lab-phase2-rules-engine.md` and `…-phase4-saved-strategies.md`.

**Planning evidence:** every code block below was compiled and run on a scratch copy of `Nandini` at `d165472` (outside the repository): engine `cargo test` **690 → 717** passed (1 ignored; +27 lab tests; the plan adds two more list checks in Tasks 6–7, so the gate expects 719), the intermediate states after Tasks 1 and 2 compile and pass (122 and 129 lab tests), gateway 6 → 7, auth in-process tests 39 → 43. With the real Indian index file (read-only from the NAS), the 31 index/ETF-type templates reproduced the book engine N/N on NIFTY 50 and NIFTY BANK (8,513 book trades, 0 differences) through `/api/lab/fidelity`; the 12 stock/ETF-only templates were refused there by the book's own admission. Phases 5 (`engine-gl`) and 6 (TradingView indicators) land **before** this plan runs and edit `lab/api.rs`, `lab/mod.rs`, the gateway allowlist and the indicator catalogue — Tasks 8–10 therefore anchor their edits on function names, not line numbers. If a quoted context line differs after Phases 5/6, apply the same change to the equivalent line; never change book files to make a patch apply.

## Global Constraints

- Branch `Nandini`; never `main`; never push (the user pushes); no force-push; no git worktrees; commits without any Co-Authored-By trailer; commit only the files a task names (other agents may commit concurrently; retry if `.git/index.lock` exists for a moment).
- Book results untouched. Protected, no edits: `Context/guru/backend/src/connors.rs`, `backend/src/books/**`, `readiness.rs`, `gate.rs`, `instrument_policy.rs`, `baskets.rs`, `accounting.rs`, `trade_validation.rs`, frontend `src/compat/*`, `src/pages/ResearchPages.tsx`. Additive-only: `backend/src/main.rs` (**not touched at all in this phase**), `webapp/gateway/src/policy.rs`, `frontend/react/src/app/custom-pages.ts` (not touched). No frontend file changes in this phase.
- Labels, verbatim: plain lab run `User-defined indicator strategy · simulated · not a book method` (unchanged); variant `Customized from <book> · <method name> · user variant · simulated`; template `Book template · declared adaptation into lab rules · not a verification`; fidelity `Template fidelity · display only · not a verification`; unavailable `Customize not available — <reason>`. Template, variant and fidelity responses never contain `claim`, `gate` or `readiness`.
- Spec stays `"v": 1`; every existing spec keeps its meaning; the v1 simulator tests stay unchanged except the mechanical `Compiled::new(…)` constructor change (Task 2).
- Templates exist only for admitted (`live`) methods, use the method's default parameters and must reproduce the book engine exactly on the four fixtures (Task 5). A template that fails is moved to `NO_TEMPLATE` with a named reason — never "fixed" by touching book code.
- Variants keep the book method's instrument admission (`instrument_policy::policy(def).allowed_types`): stock methods on actual stocks only; no index substitute. gl/crypto strategies stay unpublishable (Phase 4 rule unchanged).
- Rust builds with `CARGO_BUILD_JOBS=2`; engine tests need `VAULT_IDX="/run/user/1000/gvfs/smb-share:server=192.168.4.50,share=share/Manan Pujara/Book-to-rust/idx_data.csv"` (read-only); no data, `.env` or generated results committed; never start or recreate the live Compose project `strategy-vault`; DB-backed testing is the user's manual gate on an isolated `strategy-vault-dev-<name>` stack.
- Auth service: existing dependencies only (`package.json`/lockfile unchanged); parameterized SQL only; migrations idempotent; `001-app.sql` and `002-lab-strategies.sql` never edited.

## Review Focus

1. **A variant optimized (not just run) on an instrument its method does not admit, or losing its label in the optimizer** — expected: `needs_data` on the wrong type, variant label on the right one. Test: the optimize assertions in `variant_runs_carry_the_variant_label_and_the_methods_instrument_rule` (Task 8).
2. **Short positions with price exits** — stop above / target below the entry, mirrored fills. Test: `short_levels_mirror_long_ones` (Task 3).
3. **ATR not yet warmed up when a limit is placed** — expected: no order, no trade (never a fill at a NaN price). Test: `a_limit_without_atr_yet_places_no_order` (Task 2).
4. **A next-open fill stopped out on its own bar** — expected: exit at the stop on the fill bar (the open is the entry, so no gap rule). Test: `a_next_open_fill_can_stop_out_on_its_own_bar` (Task 3).
5. **An edit that silently drops or swaps the lineage** (copying another template's spec into a variant, or removing `origin`) — expected: 400, nothing written. Test: `a customized strategy stores its origin, and an edit cannot change it` (Task 12) and the integration subtest (Task 13).

---

## Existing code this phase relies on (read-only unless a task modifies it)

| Item | Where | Used for |
|---|---|---|
| `connors::catalogue() -> Vec<StrategyDef>`; `StrategyDef { id, name, group, dir, status, book, … }`; `Status::{Live, NeedsData(&str), NotTestable(&str)}` | `connors.rs:553-597` | template lookup, status, labels |
| `connors::run(id, &Ctx, from, to, &Params) -> Option<Vec<Trade>>` (falls through to `books::run`) | `connors.rs:959` | fidelity (tests and route) |
| shared book simulator semantics: `Entry::Close` fills at the close, exits not checked on that bar, no re-entry on a close-exit bar; `Entry::Limit` rests one bar, fills at `min(open, limit)` (long), can exit on its own close; `Exit::NextOpen` fills next open, re-entry allowed at that bar's close | `connors.rs:446-518` | what close fills, limits and `exit_fill` must reproduce |
| `Trade { dir, legs, exit_i, exit_px, reason, entry_kind, signal_i, open, exits }`, `Trade::ret` | `connors.rs:375-420` | lab trades in book format (shorts: `dir = -1`) |
| `catalogue_json(guru)` status rule (withheld gate without reviewed cash rule, or `source_review_required` → `not_testable`) | `main.rs:458-486` | mirrored by `templates::book_status`, checked by a test |
| `select_context`, `resolve_context`, `readiness::check`, `native_admission`, `window_start`, `backtest` | `main.rs:557, 630, 580, 649`, `readiness.rs:19` | fidelity route = the book `detail` path without stats |
| `instrument_policy::policy(def).allowed_types`, `.requirement`; `selected_type` | `instrument_policy.rs:44, 137` | variant instrument rule, template JSON |
| `gate::get(id) -> Option<&G>` (`verdict`, `reason`) | `gate.rs:2327` | status mirror |
| lab: `rules::parse_spec`, `Spec`, `Target`, `dims/target/set/current/check_value/apply`; `series::{Frames, Cache, compile, series, RCond, Val, Compiled}`; `simulate::simulate(&Compiled, &Bars, from, to)`; `api::{route, run_json, optimize_json, instrument, known_type, select}` | `lab/*.rs` | extended below |
| auth: `validateLabStrategy`, `labAction`, `LabRequestError`; routes `registerLabStrategies(app, pool)`; `bootstrap()` applies `001`, `002` on every start | `webapp/auth/src/*.mjs` | origin validation and storage |
| gateway: `allowed_engine_path`, `is_lab_optimization_path`, `lab_spec_problem` | `webapp/gateway/src/policy.rs` | three allowlist lines |

## File structure

```
Context/guru/backend/src/lab/rules.rs        spec fields: direction short, origin, execution.fill/exit_fill, entry.limit,
                                             exit.stop_loss/target/trailing; optimize slots              (Task 1)
Context/guru/backend/src/lab/series.rs       Compiled { exec, atr } + Compiled::new + dist; compile adds ATR (Task 2)
Context/guru/backend/src/lab/simulate.rs     close/next-open fills, short, limits (Task 2); price exits (Task 3)
Context/guru/backend/src/lab/fidelity.rs     NEW trade-list comparison                                    (Task 4)
Context/guru/backend/src/lab/templates.rs    NEW templates, reasons, status mirror, labels                (Tasks 5–7)
Context/guru/backend/src/lab/mod.rs          + pub mod fidelity; + pub mod templates                      (Tasks 4–5)
Context/guru/backend/src/lab/api.rs          origin in run/optimize (Task 8); templates/template/fidelity routes (Task 9)
Context/guru/webapp/gateway/src/policy.rs    + 3 allowlist lines + 1 test                                (Task 10)
Context/guru/webapp/auth/src/contracts.mjs   validateLabOrigin, sameLabOrigin, origin in validateLabStrategy (Task 11)
Context/guru/webapp/db/003-lab-origin.sql    NEW nullable origin column                                   (Task 12)
Context/guru/webapp/auth/src/bootstrap.mjs   + 1 line (003)                                               (Task 12)
Context/guru/webapp/auth/src/lab-strategies.mjs  store, return, keep origin                               (Task 12)
Context/guru/webapp/tests/contracts.test.mjs, lab-strategies.test.mjs   tests                             (Tasks 11–12)
Context/guru/webapp/tests/lab-strategies.integration.test.mjs   + 1 subtest (manual gate)                 (Task 13)
```

No other file changes: no `main.rs`, book, frontend, compose, Dockerfile, `package.json` or `Cargo.toml` edit.

## Before Task 1 (once)

- [ ] Confirm Phases 5 and 6 are merged into `Nandini` (`git log --oneline origin/Manan..HEAD` shows their gate commits) and the tree is clean (`git status --short` empty).
- [ ] Record the branch point and baselines: `BASE=$(git rev-parse HEAD)`; `cd Context/guru/backend && VAULT_IDX="…/idx_data.csv" CARGO_BUILD_JOBS=2 cargo test 2>&1 | grep 'test result'` (write the count down; it was 690 on `d165472` before Phases 5/6); `cargo test lab:: 2>&1 | grep 'test result'` (the lab count; 119 on `d165472`); `npm --prefix Context/guru/webapp/auth ci`.
- [ ] `python3 scripts/check_book_untouched.py --base origin/Manan` → `"problems": []`.

---

### Task 1: Rule spec fields for Customize (`lab/rules.rs`)

**Files:**
- Modify: `Context/guru/backend/src/lab/rules.rs` (types next to `Target`, `parse_spec`, new helpers before `conds`, optimize `target/set/check_value/current`)
- Test: `Context/guru/backend/src/lab/rules.rs` (`mod tests`)

**Interfaces:**
- Produces: `pub enum Fill { NextOpen, Close }`; `pub enum Dist { Pct(f64), Atr { mult: f64, length: usize } }`; `pub enum Slot { Limit, StopLoss, Target, Trailing }`; `pub struct Exec { pub short: bool, pub fill: Fill, pub exit_fill: Fill, pub limit: Option<Dist>, pub stop_loss: Option<Dist>, pub target: Option<Dist>, pub trailing: Option<Dist> }` with `Default` (long, next open, none) and `Exec::slot(&self, Slot) -> Option<Dist>`; `pub struct Origin { pub book: String, pub method: String, pub version: u32 }`; `Spec` gains `pub exec: Exec, pub origin: Option<Origin>`; `Target` gains `pub slot: Option<Slot>`; `pub const MAX_ATR_MULT: f64 = 50.`, `pub const MAX_ATR_LENGTH: usize = 500`. Optimize paths `entry.limit.pct|atr`, `exit.stop_loss.pct|atr`, `exit.target.pct|atr`, `exit.trailing.pct|atr`.

- [ ] **Step 1: Write the failing tests** — in `lab/rules.rs` `mod tests`, **replace** the whole `#[test] fn reserved_fields_are_unsupported() { … }` (direction short, `exit.stop_loss`, `exit.trailing` and `entry.limit` are no longer reserved) with the following four tests:

```rust
    #[test] fn reserved_fields_are_unsupported() {
        for (text, needle) in [
            (with(r#""v": 1,"#, r#""v": 1, "portfolio": {"max_open": 3},"#), "spec.portfolio"),
            (with(r#""all": ["#, r#""stop": {"pct": 1}, "all": ["#), "entry.stop"),
            (with(r#""fill": "next_open""#, r#""fill": "limit""#), "execution.fill 'limit'"),
            (with(r#""fill": "next_open""#, r#""fill": "stop""#), "execution.fill 'stop'"),
            (with(r#""v": 1,"#, r#""v": 1, "scale_in": [],"#), "spec.scale_in"),
        ] {
            let e = err(&text);
            assert_eq!(e.status(), "unsupported", "{text}: {e:?}");
            assert!(e.message().contains(needle), "{needle:?} not in {:?}", e.message());
        }
    }
    #[test] fn customize_fields_parse() {
        let s = parse_spec(&with(r#""direction": "long""#, r#""direction": "short", "origin": {"book":"stc9","method":"rsi2_4","template_version":1}"#)).unwrap();
        assert!(s.exec.short);
        assert_eq!(s.origin, Some(Origin { book: "stc9".into(), method: "rsi2_4".into(), version: 1 }));
        assert_eq!((s.exec.fill, s.exec.exit_fill, s.exec.limit, s.exec.stop_loss), (Fill::NextOpen, Fill::NextOpen, None, None));
        let s = parse_spec(&with(r#""fill": "next_open""#, r#""fill": "close""#)).unwrap();
        assert_eq!((s.exec.fill, s.exec.exit_fill), (Fill::Close, Fill::Close));
        let s = parse_spec(&with(r#""fill": "next_open""#, r#""fill": "close", "exit_fill": "next_open""#)).unwrap();
        assert_eq!((s.exec.fill, s.exec.exit_fill), (Fill::Close, Fill::NextOpen));
        let s = parse_spec(&with(r#""all": ["#, r#""limit": {"pct": 3}, "all": ["#).replace(r#""fill": "next_open", "#, "")).unwrap();
        assert_eq!(s.exec.limit, Some(Dist::Pct(3.)));
        let s = parse_spec(&with(r#""any": ["#, r#""stop_loss": {"atr": 2}, "target": {"pct": 10}, "trailing": {"atr": 3, "atr_length": 20}, "any": ["#)).unwrap();
        assert_eq!((s.exec.stop_loss, s.exec.target, s.exec.trailing), (Some(Dist::Atr { mult: 2., length: 14 }), Some(Dist::Pct(10.)), Some(Dist::Atr { mult: 3., length: 20 })));
        // A price exit lets exit.any be empty; without one it still needs a condition.
        let only_target = r#"{"v":1,"entry":{"all":[{"lhs":{"s":"close"},"op":">","rhs":{"value":1}}]},"exit":{"any":[],"target":{"pct":10}}}"#;
        assert!(parse_spec(only_target).unwrap().exit.is_empty());
        assert!(err(&only_target.replace(r#","target":{"pct":10}"#, "")).message().contains("or a stop_loss, target or trailing exit"));
    }
    #[test] fn customize_fields_are_bounded_and_named() {
        for (text, needle) in [
            (with(r#""direction": "long""#, r#""direction": "both""#), "direction must be"),
            (with(r#""fill": "next_open""#, r#""fill": "open""#), "execution.fill must be"),
            (with(r#""fill": "next_open""#, r#""exit_fill": "later""#), "execution.exit_fill must be"),
            (with(r#""all": ["#, r#""limit": {"pct": 3}, "all": ["#), "remove execution.fill"),
            (with(r#""all": ["#, r#""limit": {"pct": 51}, "all": ["#).replace(r#""fill": "next_open", "#, ""), "entry.limit.pct must be a number from 0 and at most 50"),
            (with(r#""any": ["#, r#""stop_loss": {"pct": 0}, "any": ["#), "exit.stop_loss.pct must be a number above 0"),
            (with(r#""any": ["#, r#""trailing": {"pct": 100}, "any": ["#), "exit.trailing.pct"),
            (with(r#""any": ["#, r#""target": {"pct": 5, "atr": 1}, "any": ["#), "exactly one of pct or atr"),
            (with(r#""any": ["#, r#""target": {"pct": 5, "atr_length": 3}, "any": ["#), "exactly one of pct or atr"),
            (with(r#""any": ["#, r#""stop_loss": {"atr": 0}, "any": ["#), "exit.stop_loss.atr must be"),
            (with(r#""any": ["#, r#""stop_loss": {"atr": 1, "atr_length": 501}, "any": ["#), "atr_length must be a whole number"),
            (with(r#""any": ["#, r#""stop_loss": {"atr": 1, "len": 5}, "any": ["#), "unknown field 'len' in exit.stop_loss"),
            (with(r#""v": 1,"#, r#""v": 1, "origin": {"book":"stc9","method":"rsi 2"},"#), "origin.method"),
            (with(r#""v": 1,"#, r#""v": 1, "origin": {"book":"stc9","method":"rsi2_4","template_version":0},"#), "origin.template_version"),
            (with(r#""v": 1,"#, r#""v": 1, "origin": {"book":"stc9","method":"rsi2_4","template_version":1,"x":1},"#), "unknown field 'x' in origin"),
        ] {
            let e = err(&text);
            assert_eq!(e.status(), "invalid", "{text}: {e:?}");
            assert!(e.message().contains(needle), "{needle:?} not in {:?}", e.message());
        }
    }
    #[test] fn optimize_reaches_price_orders() {
        let text = with(r#""any": ["#, r#""stop_loss": {"pct": 5}, "trailing": {"atr": 3}, "any": ["#)
            .replace(r#"[ {"path":"entry.all[0].lhs.p.length","from":2,"to":6,"step":1} ]"#, r#"[ {"path":"exit.stop_loss.pct","from":2,"to":8,"step":2}, {"path":"exit.trailing.atr","from":1,"to":3,"step":1} ]"#);
        let s = parse_spec(&text).unwrap();
        assert_eq!((s.optimize[0].values.len(), current(&s, &s.optimize[0].target), current(&s, &s.optimize[1].target)), (4, 5., 3.));
        let t = apply(&s, &s.optimize, &[4., 2.]);
        assert_eq!((t.exec.stop_loss, t.exec.trailing), (Some(Dist::Pct(4.)), Some(Dist::Atr { mult: 2., length: 14 })));
        assert!(err(&text.replace("exit.stop_loss.pct", "exit.stop_loss.atr")).message().contains("exit.stop_loss is set by pct"));
        assert!(err(&text.replace("exit.stop_loss.pct", "exit.target.pct")).message().contains("the spec has no exit.target"));
        assert!(err(&text.replace(r#""from":2,"to":8"#, r#""from":50,"to":120"#)).message().contains("out of range"));
    }
```

- [ ] **Step 2: Run to verify failure**

Run: `cd Context/guru/backend && CARGO_BUILD_JOBS=2 cargo test lab::rules`
Expected: compile errors — `cannot find type Origin / Fill / Dist`, no field `exec` / `origin` on `Spec`.

- [ ] **Step 3: Implement** — apply this patch to `lab/rules.rs` (from the repository root: save it as `/tmp/rules.diff` and run `git apply /tmp/rules.diff`, or make the same edits by hand; the `@@` line numbers are from `d165472`):

```diff
--- a/Context/guru/backend/src/lab/rules.rs
+++ b/Context/guru/backend/src/lab/rules.rs
@@ -40,46 +40,77 @@
 pub struct Cond { pub lhs: Operand, pub op: Op, pub rhs: Operand }
 /// Where an optimize path points: entry/exit list, condition index, side, and parameter key (None = `value`).
 #[derive(Clone, Debug, PartialEq)]
-pub struct Target { pub exit: bool, pub cond: usize, pub rhs: bool, pub param: Option<String> }
+pub struct Target { pub exit: bool, pub cond: usize, pub rhs: bool, pub param: Option<String>, pub slot: Option<Slot> }
+/// Market orders fill at the next bar's open (default) or at the signal bar's own close (the book convention).
+#[derive(Clone, Copy, Debug, PartialEq, Eq)]
+pub enum Fill { NextOpen, Close }
+/// A price distance: percent of a reference price, or a multiple of ATR(length) read at a bar already closed.
+#[derive(Clone, Copy, Debug, PartialEq)]
+pub enum Dist { Pct(f64), Atr { mult: f64, length: usize } }
+/// The price-order settings an optimize path can address (`entry.limit.pct`, `exit.stop_loss.atr`, ...).
+#[derive(Clone, Copy, Debug, PartialEq, Eq)]
+pub enum Slot { Limit, StopLoss, Target, Trailing }
+#[derive(Clone, Debug, PartialEq)]
+pub struct Exec { pub short: bool, pub fill: Fill, pub exit_fill: Fill, pub limit: Option<Dist>, pub stop_loss: Option<Dist>, pub target: Option<Dist>, pub trailing: Option<Dist> }
+impl Default for Exec {
+    fn default() -> Self { Exec { short: false, fill: Fill::NextOpen, exit_fill: Fill::NextOpen, limit: None, stop_loss: None, target: None, trailing: None } }
+}
+impl Exec {
+    pub fn slot(&self, s: Slot) -> Option<Dist> { match s { Slot::Limit => self.limit, Slot::StopLoss => self.stop_loss, Slot::Target => self.target, Slot::Trailing => self.trailing } }
+    fn slot_mut(&mut self, s: Slot) -> &mut Option<Dist> { match s { Slot::Limit => &mut self.limit, Slot::StopLoss => &mut self.stop_loss, Slot::Target => &mut self.target, Slot::Trailing => &mut self.trailing } }
+}
+/// Lineage of a customized book template (`origin` in the spec). Checked against the template catalogue by the API.
+#[derive(Clone, Debug, PartialEq)]
+pub struct Origin { pub book: String, pub method: String, pub version: u32 }
+/// Slot, spec path, largest pct. A limit may sit at the signal close (0%); stop-loss, target and trailing need pct > 0.
+const SLOTS: [(Slot, &str, f64); 4] = [(Slot::Limit, "entry.limit", 50.), (Slot::StopLoss, "exit.stop_loss", 99.), (Slot::Target, "exit.target", 1000.), (Slot::Trailing, "exit.trailing", 99.)];
+pub const MAX_ATR_MULT: f64 = 50.;
+pub const MAX_ATR_LENGTH: usize = 500;
 #[derive(Clone, Debug, PartialEq)]
 pub struct Dim { pub path: String, pub target: Target, pub values: Vec<f64> }
 #[derive(Clone, Debug, PartialEq)]
-pub struct Spec { pub entry: Vec<Cond>, pub exit: Vec<Cond>, pub cost_bps: Option<f64>, pub optimize: Vec<Dim> }
+pub struct Spec { pub entry: Vec<Cond>, pub exit: Vec<Cond>, pub cost_bps: Option<f64>, pub optimize: Vec<Dim>, pub exec: Exec, pub origin: Option<Origin> }
 
 pub fn parse_spec(text: &str) -> Result<Spec, LabError> {
     if text.len() > MAX_SPEC_BYTES { return Err(inv(format!("spec is {} bytes; the limit is {MAX_SPEC_BYTES}", text.len()))); }
     let root = json::parse(text).map_err(|e| inv(format!("spec is not valid JSON: {e}")))?;
-    fields(&root, "spec", &["v", "direction", "entry", "exit", "execution", "optimize"], &["short", "portfolio", "scale_in"])?;
+    fields(&root, "spec", &["v", "direction", "origin", "entry", "exit", "execution", "optimize"], &["short", "portfolio", "scale_in"])?;
     if root.get("v").and_then(Json::as_f64) != Some(1.) { return Err(inv("v must be 1")); }
+    let mut exec = Exec::default();
     match root.get("direction") {
         None => {}
         Some(Json::Str(d)) if d == "long" => {}
-        Some(Json::Str(d)) if d == "short" => return Err(LabError::Unsupported("direction 'short' is reserved; v1 simulates long only".into())),
-        _ => return Err(inv("direction must be \"long\"")),
+        Some(Json::Str(d)) if d == "short" => exec.short = true,
+        _ => return Err(inv("direction must be \"long\" or \"short\"")),
     }
+    let origin = root.get("origin").map(origin).transpose()?;
     let entry_obj = root.get("entry").ok_or_else(|| inv("entry is required"))?;
-    fields(entry_obj, "entry", &["all"], &["limit", "stop", "scale_in"])?;
+    fields(entry_obj, "entry", &["all", "limit"], &["stop", "scale_in"])?;
     let exit_obj = root.get("exit").ok_or_else(|| inv("exit is required"))?;
-    fields(exit_obj, "exit", &["any"], &["stop_loss", "target", "trailing"])?;
-    let entry = conds(entry_obj.get("all"), "entry.all", false)?;
-    let exit = conds(exit_obj.get("any"), "exit.any", true)?;
+    fields(exit_obj, "exit", &["any", "stop_loss", "target", "trailing"], &[])?;
+    for (slot, at, hi) in SLOTS {
+        let obj = if slot == Slot::Limit { entry_obj } else { exit_obj };
+        if let Some(v) = obj.get(at.split_once('.').unwrap().1) { *exec.slot_mut(slot) = Some(dist(v, at, hi)?); }
+    }
+    let price_exit = exec.stop_loss.is_some() || exec.target.is_some() || exec.trailing.is_some();
+    let entry = conds(entry_obj.get("all"), "entry.all", false, false)?;
+    let exit = conds(exit_obj.get("any"), "exit.any", true, price_exit)?;
     if entry.len() + exit.len() > MAX_CONDITIONS {
         return Err(inv(format!("{} conditions; the limit is {MAX_CONDITIONS}", entry.len() + exit.len())));
     }
     let mut cost_bps = None;
     if let Some(ex) = root.get("execution") {
-        fields(ex, "execution", &["fill", "cost_bps"], &["limit", "stop"])?;
-        match ex.get("fill") {
-            None => {}
-            Some(Json::Str(f)) if f == "next_open" => {}
-            Some(Json::Str(f)) => return Err(LabError::Unsupported(format!("execution.fill '{f}' is reserved; v1 fills at the next open"))),
-            _ => return Err(inv("execution.fill must be \"next_open\"")),
+        fields(ex, "execution", &["fill", "exit_fill", "cost_bps"], &["limit", "stop"])?;
+        if let Some(f) = ex.get("fill") { exec.fill = fill(f, "execution.fill")?; exec.exit_fill = exec.fill; }
+        if let Some(f) = ex.get("exit_fill") { exec.exit_fill = fill(f, "execution.exit_fill")?; }
+        if exec.limit.is_some() && ex.get("fill").is_some() {
+            return Err(inv("execution.fill applies to market entries; entry.limit already fills on the bar after the signal, so remove execution.fill"));
         }
         if let Some(c) = ex.get("cost_bps") {
             cost_bps = Some(c.as_f64().filter(|c| (0. ..=5000.).contains(c)).ok_or_else(|| inv("execution.cost_bps must be a number from 0 to 5000"))?);
         }
     }
-    let mut spec = Spec { entry, exit, cost_bps, optimize: vec![] };
+    let mut spec = Spec { entry, exit, cost_bps, optimize: vec![], exec, origin };
     if let Some(o) = root.get("optimize") { spec.optimize = dims(&spec, o)?; }
     Ok(spec)
 }
@@ -93,9 +124,48 @@
     Ok(())
 }
 
-fn conds(v: Option<&Json>, at: &str, exit: bool) -> Result<Vec<Cond>, LabError> {
+fn fill(v: &Json, at: &str) -> Result<Fill, LabError> {
+    match v.as_str() {
+        Some("next_open") => Ok(Fill::NextOpen),
+        Some("close") => Ok(Fill::Close),
+        Some(f @ ("limit" | "stop")) => Err(LabError::Unsupported(format!("{at} '{f}' is reserved; use entry.limit for limit entries (stop entries are a later version)"))),
+        _ => Err(inv(format!("{at} must be \"next_open\" or \"close\""))),
+    }
+}
+/// `{"pct": p}` or `{"atr": m, "atr_length": n}` (n defaults to 14).
+fn dist(v: &Json, at: &str, hi: f64) -> Result<Dist, LabError> {
+    fields(v, at, &["pct", "atr", "atr_length"], &[])?;
+    match (v.get("pct"), v.get("atr")) {
+        (Some(p), None) if v.get("atr_length").is_none() => {
+            let min = if at == "entry.limit" { "from 0" } else { "above 0" };
+            p.as_f64().filter(|p| *p <= hi && (*p > 0. || (*p == 0. && at == "entry.limit"))).map(Dist::Pct)
+                .ok_or_else(|| inv(format!("{at}.pct must be a number {min} and at most {hi}")))
+        }
+        (None, Some(m)) => {
+            let mult = m.as_f64().filter(|m| *m > 0. && *m <= MAX_ATR_MULT).ok_or_else(|| inv(format!("{at}.atr must be a number above 0 and at most {MAX_ATR_MULT}")))?;
+            let length = match v.get("atr_length") {
+                None => 14,
+                Some(n) => n.as_f64().filter(|n| n.fract() == 0. && *n >= 1. && *n <= MAX_ATR_LENGTH as f64)
+                    .ok_or_else(|| inv(format!("{at}.atr_length must be a whole number from 1 to {MAX_ATR_LENGTH}")))? as usize,
+            };
+            Ok(Dist::Atr { mult, length })
+        }
+        _ => Err(inv(format!("{at} needs exactly one of pct or atr (atr_length goes with atr)"))),
+    }
+}
+fn origin(v: &Json) -> Result<Origin, LabError> {
+    fields(v, "origin", &["book", "method", "template_version"], &[])?;
+    let id = |k: &str| v.get(k).and_then(Json::as_str).filter(|s| !s.is_empty() && s.len() <= 80 && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_'))
+        .map(String::from).ok_or_else(|| inv(format!("origin.{k} must be a catalogue id (letters, digits, _)")));
+    let (book, method) = (id("book")?, id("method")?);
+    let version = v.get("template_version").and_then(Json::as_f64).filter(|n| n.fract() == 0. && *n >= 1. && *n <= 1000.)
+        .ok_or_else(|| inv("origin.template_version must be a whole number from 1 to 1000"))? as u32;
+    Ok(Origin { book, method, version })
+}
+
+fn conds(v: Option<&Json>, at: &str, exit: bool, may_be_empty: bool) -> Result<Vec<Cond>, LabError> {
     let Some(Json::Arr(items)) = v else { return Err(inv(format!("{at} must be a list of conditions"))) };
-    if items.is_empty() { return Err(inv(format!("{at} needs at least one condition"))); }
+    if items.is_empty() && !may_be_empty { return Err(inv(format!("{at} needs at least one condition{}", if exit { " (or a stop_loss, target or trailing exit)" } else { "" }))); }
     if items.len() > MAX_CONDITIONS { return Err(inv(format!("{at} has {} conditions; the limit is {MAX_CONDITIONS}", items.len()))); }
     items.iter().enumerate().map(|(i, c)| cond(c, &format!("{at}[{i}]"), exit)).collect()
 }
@@ -190,10 +260,18 @@
 }
 
 fn target(spec: &Spec, path: &str) -> Result<Target, String> {
+    if let Some((slot, at, _)) = SLOTS.iter().find(|(_, at, _)| path.starts_with(&format!("{at}."))) {
+        let field = &path[at.len() + 1..];
+        return match (spec.exec.slot(*slot), field) {
+            (Some(Dist::Pct(_)), "pct") | (Some(Dist::Atr { .. }), "atr") => Ok(Target { exit: false, cond: 0, rhs: false, param: None, slot: Some(*slot) }),
+            (None, _) => Err(format!("the spec has no {at}")),
+            _ => Err(format!("{at} is set by {}; optimize that field", if matches!(spec.exec.slot(*slot), Some(Dist::Pct(_))) { "pct" } else { "atr" })),
+        };
+    }
     let (exit, rest) = match (path.strip_prefix("entry.all["), path.strip_prefix("exit.any[")) {
         (Some(r), _) => (false, r),
         (_, Some(r)) => (true, r),
-        _ => return Err("must start with entry.all[i] or exit.any[i]".into()),
+        _ => return Err("must start with entry.all[i], exit.any[i], entry.limit, exit.stop_loss, exit.target or exit.trailing".into()),
     };
     let (idx, rest) = rest.split_once("].").ok_or("expected ].lhs or ].rhs")?;
     let cond: usize = idx.parse().map_err(|_| "condition index must be a whole number")?;
@@ -204,12 +282,12 @@
         _ => return Err("expected lhs or rhs".into()),
     };
     match (if rhs { &c.rhs } else { &c.lhs }, field) {
-        (Operand::Value(_), "value") => Ok(Target { exit, cond, rhs, param: None }),
+        (Operand::Value(_), "value") => Ok(Target { exit, cond, rhs, param: None, slot: None }),
         (Operand::Series(s), f) if f.starts_with("p.") => {
             let def = catalogue::get(&s.id).ok_or("price sources have no parameters")?;
             let key = &f[2..];
             if !def.params.iter().any(|p| p.key == key) { return Err(format!("{} has no parameter '{key}'", def.id)); }
-            Ok(Target { exit, cond, rhs, param: Some(key.into()) })
+            Ok(Target { exit, cond, rhs, param: Some(key.into()), slot: None })
         }
         _ => Err("must end in .value (a number operand) or .p.<key> (an indicator parameter)".into()),
     }
@@ -220,6 +298,10 @@
     if t.rhs { &mut c.rhs } else { &mut c.lhs }
 }
 fn set(s: &mut Spec, t: &Target, x: f64) {
+    if let Some(slot) = t.slot {
+        match s.exec.slot_mut(slot) { Some(Dist::Pct(p)) => *p = x, Some(Dist::Atr { mult, .. }) => *mult = x, None => {} }
+        return;
+    }
     match (operand_mut(s, t), &t.param) {
         (Operand::Value(v), None) => *v = x,
         (Operand::Series(r), Some(k)) => { r.params.insert(k.clone(), x); }
@@ -227,6 +309,11 @@
     }
 }
 fn check_value(spec: &Spec, t: &Target, x: f64) -> Result<(), String> {
+    if let Some(slot) = t.slot {
+        let (_, at, hi) = SLOTS.iter().find(|(s, _, _)| *s == slot).unwrap();
+        let ok = match spec.exec.slot(slot) { Some(Dist::Pct(_)) => x <= *hi && (x > 0. || (x == 0. && slot == Slot::Limit)), _ => x > 0. && x <= MAX_ATR_MULT };
+        return if ok { Ok(()) } else { Err(format!("{at} value {x} is out of range")) };
+    }
     let mut s = spec.clone();
     set(&mut s, t, x);
     match operand_mut(&mut s, t) {
@@ -236,6 +323,7 @@
 }
 /// The spec's own value at an optimize target (the "base" setting the ranking prefers on ties).
 pub fn current(spec: &Spec, t: &Target) -> f64 {
+    if let Some(slot) = t.slot { return match spec.exec.slot(slot) { Some(Dist::Pct(p)) => p, Some(Dist::Atr { mult, .. }) => mult, None => f64::NAN }; }
     let mut s = spec.clone();
     match (operand_mut(&mut s, t), &t.param) {
         (Operand::Value(v), None) => *v,
```

- [ ] **Step 4: Run to verify pass**

Run: `cd Context/guru/backend && CARGO_BUILD_JOBS=2 cargo test lab::`
Expected: all lab tests pass (on `d165472`: 119 → 122; three new tests, `reserved_fields_are_unsupported` kept with its new cases). `simulate.rs` and `series.rs` still compile unchanged (they do not construct `Spec` or `Target`).

- [ ] **Step 5: Commit**

```bash
git add Context/guru/backend/src/lab/rules.rs
git commit -m "Accept short, close fills, limit entries and price exits in lab rule specs"
```

---

### Task 2: Execution settings in compiled specs; close fills, next-open exits, short and limit entries (`series.rs`, `simulate.rs`)

**Files:**
- Modify: `Context/guru/backend/src/lab/series.rs` (`use` line, `Compiled`, `compile`)
- Modify: `Context/guru/backend/src/lab/simulate.rs` (everything above `#[cfg(test)]`; six test constructors; new tests)

**Interfaces:**
- Consumes: `rules::{Dist, Exec, Fill}` (Task 1).
- Produces: `pub struct Compiled { pub entry: Vec<RCond>, pub exit: Vec<RCond>, pub exec: Exec, pub atr: BTreeMap<usize, Vec<f64>> }`; `Compiled::new(entry, exit) -> Compiled` (v1 defaults); `Compiled::dist(&self, d: Dist, px: f64, k: usize) -> f64` (pct of `px`, or `mult × ATR(length)[k]`, NaN while ATR warms up); `compile` fills `atr` for every ATR length the spec uses (lab `atr` indicator, daily). `simulate(c, b, from, to)` keeps its signature; trades carry `dir` = ±1 and `entry_kind` ∈ `"next_open" | "close" | "limit"`.

- [ ] **Step 1: Write the failing tests** — in `lab/simulate.rs` `mod tests`:
  (a) change the six existing constructors from `Compiled { entry: vec![…], exit: vec![…] }` to `Compiled::new(vec![…], vec![…])` (same arguments; behaviour must not change);
  (b) append before the module's final `}`:

```rust
    // ---- Customize: close fills, next-open exits, short, limit entries, price exits (spec 2026-10-16 §5)
    use crate::lab::rules::{Dist, Exec, Fill};
    fn with(entry: &[usize], exit: &[usize], exec: Exec) -> Compiled { Compiled { exec, ..Compiled::new(vec![rule(on(entry))], vec![rule(on(exit))]) } }
    fn close_fill() -> Exec { Exec { fill: Fill::Close, exit_fill: Fill::Close, ..Exec::default() } }
    fn fills(t: &[Trade]) -> Vec<(usize, f64, usize, f64)> { t.iter().map(|t| (t.legs[0].i, t.legs[0].px, t.exit_i, t.exit_px)).collect() }

    #[test] fn close_fill_uses_signal_closes_and_skips_exits_on_the_entry_bar() {
        // exit rule true on the entry bar 2 is ignored (book rule); it fires at bar 4's close
        let t = simulate(&with(&[2, 4, 6], &[2, 4], close_fill()), &bars(), 0, 9);
        // no re-entry at the close that exited (bar 4); the next entry is bar 6, still open at the end
        assert_eq!(fills(&t), vec![(2, 102.5, 4, 104.5), (6, 106.5, 9, 109.5)]);
        assert_eq!((t[0].entry_kind, t[0].signal_i, t[1].open), ("close", 4, true));
        assert!(crate::trade_validation::check(&t, 0, 9, 0.0005).is_ok());
    }
    #[test] fn close_fill_with_next_open_exit_may_reenter_at_the_exit_bars_close() {
        let exec = Exec { exit_fill: Fill::NextOpen, ..close_fill() };
        let t = simulate(&with(&[1, 3, 4], &[3], exec), &bars(), 0, 9);
        // exit signal at bar 3's close (no entry that bar: the position is still open) → open of 4; entry at 4's close
        assert_eq!(fills(&t), vec![(1, 101.5, 4, 104.), (4, 104.5, 9, 109.5)]);
        assert_eq!(t[0].signal_i, 3);
    }
    #[test] fn exit_on_the_last_bar_close_is_inside_the_window() {
        let t = simulate(&with(&[2], &[9], close_fill()), &bars(), 0, 9);
        assert_eq!((t[0].exit_i, t[0].exit_px, t[0].open), (9, 109.5, false));
    }
    #[test] fn short_positions_mirror_long_ones() {
        let t = simulate(&with(&[2], &[5], Exec { short: true, ..Exec::default() }), &bars(), 0, 9);
        assert_eq!((t[0].dir, t[0].legs[0].px, t[0].exit_px), (-1., 103., 106.));
        assert!(t[0].ret(0.) < 0.); // price rose against the short
    }
    #[test] fn limit_entry_fills_on_a_touch_at_the_better_price_and_otherwise_expires() {
        let exec = Exec { limit: Some(Dist::Pct(1.)), exit_fill: Fill::Close, ..Exec::default() };
        // bar 2 close 102.5 → limit 101.475; bar 3 low 102 does not touch: the order expires (no carry-over)
        assert!(simulate(&with(&[2], &[6], exec.clone()), &bars(), 0, 9).is_empty());
        let mut b = bars();
        b.low[3] = 101.;
        let t = simulate(&with(&[2], &[6], exec.clone()), &b, 0, 9);
        assert_eq!((t[0].legs[0].i, t[0].legs[0].px, t[0].entry_kind), (3, 101.475, "limit"));
        b.open[3] = 100.5; b.low[3] = 100.; // gap below the limit: filled at the open
        assert_eq!(simulate(&with(&[2], &[6], exec.clone()), &b, 0, 9)[0].legs[0].px, 100.5);
        // short limits sit above the close and fill on the high
        let mut s = bars();
        s.high[3] = 104.;
        let t = simulate(&with(&[2], &[6], Exec { short: true, ..exec }), &s, 0, 9);
        assert_eq!((t[0].dir, t[0].legs[0].px), (-1., 103.525));
    }
    #[test] fn a_limit_fill_bar_can_exit_at_its_close_and_blocks_a_new_order() {
        let mut b = bars();
        b.low[3] = 101.;
        let exec = Exec { limit: Some(Dist::Pct(1.)), exit_fill: Fill::Close, ..Exec::default() };
        let t = simulate(&with(&[2, 3], &[3], exec), &b, 0, 9);
        assert_eq!(fills(&t), vec![(3, 101.475, 3, 103.5)]); // no new limit from bar 3's entry signal
    }
    #[test] fn a_limit_without_atr_yet_places_no_order() {
        let mut c = with(&[2], &[6], Exec { limit: Some(Dist::Atr { mult: 1., length: 14 }), exit_fill: Fill::Close, ..Exec::default() });
        c.atr.insert(14, vec![f64::NAN; 10]);
        let mut b = bars();
        b.low[3] = 0.5;
        assert!(simulate(&c, &b, 0, 9).is_empty());
    }
```

- [ ] **Step 2: Run to verify failure**

Run: `cd Context/guru/backend && CARGO_BUILD_JOBS=2 cargo test lab::simulate`
Expected: compile errors — no function `Compiled::new`, no field `exec`/`atr`, no method `dist`.

- [ ] **Step 3: Implement `series.rs`** — apply:

```diff
--- a/Context/guru/backend/src/lab/series.rs
+++ b/Context/guru/backend/src/lab/series.rs
@@ -3,7 +3,7 @@
 use super::bars::Bars;
 use super::catalogue;
 use super::resample::{self, Tf};
-use super::rules::{Cond, LabError, Op, Operand, SeriesRef, Spec};
+use super::rules::{Cond, Dist, Exec, LabError, Op, Operand, SeriesRef, Spec};
 use std::collections::BTreeMap;
 use std::rc::Rc;
 
@@ -90,7 +90,15 @@
         }
     }
 }
-pub struct Compiled { pub entry: Vec<RCond>, pub exit: Vec<RCond> }
+pub struct Compiled { pub entry: Vec<RCond>, pub exit: Vec<RCond>, pub exec: Exec, pub atr: BTreeMap<usize, Vec<f64>> }
+impl Compiled {
+    /// Long, market orders at the next open, no price exits (the v1 defaults).
+    pub fn new(entry: Vec<RCond>, exit: Vec<RCond>) -> Compiled { Compiled { entry, exit, exec: Exec::default(), atr: BTreeMap::new() } }
+    /// Price distance of `d` from `px`, using ATR at bar `k` (a bar whose close is already known). NaN while ATR warms up.
+    pub fn dist(&self, d: Dist, px: f64, k: usize) -> f64 {
+        match d { Dist::Pct(p) => px * p / 100., Dist::Atr { mult, length } => self.atr.get(&length).map_or(f64::NAN, |a| mult * a[k]) }
+    }
+}
 
 fn val(o: &Operand, fr: &Frames, cache: &mut Cache) -> Result<Val, LabError> {
     Ok(match o { Operand::Series(s) => Val::Arr(series(fr, cache, s)?), Operand::Value(x) => Val::Const(*x), Operand::BarsHeld => Val::Held })
@@ -99,7 +107,16 @@
     cs.iter().enumerate().map(|(k, c)| Ok(RCond { lhs: val(&c.lhs, fr, cache)?, op: c.op, rhs: val(&c.rhs, fr, cache)?, label: format!("{at}[{k}]: {}", describe(c)) })).collect()
 }
 pub fn compile(spec: &Spec, fr: &Frames, cache: &mut Cache) -> Result<Compiled, LabError> {
-    Ok(Compiled { entry: list(&spec.entry, "entry.all", fr, cache)?, exit: list(&spec.exit, "exit.any", fr, cache)? })
+    let mut c = Compiled { entry: list(&spec.entry, "entry.all", fr, cache)?, exit: list(&spec.exit, "exit.any", fr, cache)?, exec: spec.exec.clone(), atr: BTreeMap::new() };
+    let x = &spec.exec;
+    for d in [x.limit, x.stop_loss, x.target, x.trailing].into_iter().flatten() {
+        if let Dist::Atr { length, .. } = d {
+            if c.atr.contains_key(&length) { continue; }
+            let r = SeriesRef { id: "atr".into(), params: [("length".to_string(), length as f64)].into(), tf: Tf::D, out: 0, offset: 0 };
+            c.atr.insert(length, series(fr, cache, &r)?);
+        }
+    }
+    Ok(c)
 }
 
 /// Human-readable condition, recorded as a trade's exit reason, e.g. `rsi(length=2) crosses_above 70`.
```

- [ ] **Step 4: Implement `simulate.rs`** — replace everything above `#[cfg(test)]` with:

```rust
//! Single-instrument simulation (spec §4, Customize §5). One position at a time; book-format `connors::Trade`s so
//! trade_validation, equity and stats are reused read-only.
//! Order of events on bar i: (1) at the open a pending exit fills, then a pending market entry, then a resting limit
//! entry (low/high touch, filled at the better of open and limit); (2) at the close, exit conditions (any) fill at
//! this close or set a next-open exit, then entry conditions (all) fill at this close, set a next-open market order
//! or place a limit for the next bar.
use super::bars::Bars;
use super::rules::Fill;
use super::series::Compiled;
use crate::connors::{Leg, Trade};

struct Pos { e: usize, px: f64, kind: &'static str }
impl Pos {
    fn trade(&self, x: usize, px: f64, reason: String, signal_i: usize, d: f64) -> Trade {
        Trade { dir: d, legs: vec![Leg { i: self.e, px: self.px, w: 1. }], exit_i: x, exit_px: px, reason, entry_kind: self.kind, signal_i, open: false, exits: vec![] }
    }
}

pub fn simulate(c: &Compiled, b: &Bars, from: usize, to: usize) -> Vec<Trade> {
    let x = &c.exec;
    let d = if x.short { -1. } else { 1. };
    // v1 rule kept: a next-open market entry may share the open with a next-open exit. Close fills and limit orders
    // follow the books instead: no new order on a bar that held a position at its close or exited after its open.
    let shares_open = x.limit.is_none() && x.fill == Fill::NextOpen;
    let mut trades = Vec::new();
    let mut pos: Option<Pos> = None;
    let mut pending_entry: Option<usize> = None;
    let mut pending_limit: Option<f64> = None;
    let mut pending_exit: Option<(usize, String)> = None;
    for i in from..=to {
        if let Some((sig, why)) = pending_exit.take() {
            if let Some(p) = pos.take() { trades.push(p.trade(i, b.open[i], why, sig, d)); }
        }
        if pending_entry.take().is_some() { pos = Some(Pos { e: i, px: b.open[i], kind: "next_open" }); }
        if let Some(lim) = pending_limit.take() {
            let touched = if d > 0. { b.low[i] <= lim } else { b.high[i] >= lim };
            if touched { pos = Some(Pos { e: i, px: if d > 0. { b.open[i].min(lim) } else { b.open[i].max(lim) }, kind: "limit" }); }
        }
        let mut exited = false; // after the open: blocks a same-bar order under book rules
        let last = i == to;
        if let Some(p) = &pos {
            if let Some(r) = c.exit.iter().find(|r| r.holds(i, Some(p.e))) {
                match x.exit_fill {
                    Fill::Close => { trades.push(pos.take().unwrap().trade(i, b.close[i], r.label.clone(), i, d)); exited = true; }
                    Fill::NextOpen if !last => pending_exit = Some((i, r.label.clone())),
                    Fill::NextOpen => {} // no next open inside the window; the position is marked open below
                }
            }
        }
        if last { break; } // a signal on the last bar has no fill inside the window
        let may_enter = if shares_open { pos.is_none() || pending_exit.is_some() } else { pos.is_none() && !exited };
        if may_enter && c.entry.iter().all(|r| r.holds(i, None)) {
            match (x.limit, x.fill) {
                (Some(l), _) => pending_limit = Some(b.close[i] - d * c.dist(l, b.close[i], i)),
                (None, Fill::Close) => pos = Some(Pos { e: i, px: b.close[i], kind: "close" }),
                (None, Fill::NextOpen) => pending_entry = Some(i),
            }
        }
    }
    if let Some(p) = pos { trades.push(Trade { open: true, ..p.trade(to, b.close[to], "open".into(), to, d) }); }
    trades
}
```

- [ ] **Step 5: Run to verify pass**

Run: `cd Context/guru/backend && CARGO_BUILD_JOBS=2 cargo test lab::`
Expected: all pass (on `d165472`: 129). The six v1 simulator tests pass unchanged — the defaults reproduce v1. Also `cargo test lab::api` passes (the API still calls `simulate(&compiled, &fr.d, from, to)`).

- [ ] **Step 6: Commit**

```bash
git add Context/guru/backend/src/lab/series.rs Context/guru/backend/src/lab/simulate.rs
git commit -m "Simulate close fills, next-open exits, short positions and limit entries in the lab"
```

---

### Task 3: Stop-loss, target and trailing exits; no-lookahead property (`simulate.rs`)

**Files:**
- Modify: `Context/guru/backend/src/lab/simulate.rs`

**Interfaces:**
- Consumes: Task 2 (`Compiled::dist`, `Exec.stop_loss/target/trailing`).
- Produces: price exits with reasons `exit.stop_loss`, `exit.trailing`, `exit.target`, each optionally suffixed ` (gap at the open)`; `signal_i` = exit bar for price exits. Event order and conservative rules exactly as spec §5.1.

- [ ] **Step 1: Write the failing tests** — append inside `mod tests` (before its final `}`):

```rust
    #[test] fn stop_and_target_fill_at_their_levels_or_the_gapped_open() {
        let exec = Exec { stop_loss: Some(Dist::Pct(2.)), target: Some(Dist::Pct(3.)), ..close_fill() };
        // entry close 102.5 → stop 100.45, target 105.575
        let mut b = bars();
        b.low[4] = 100.;
        let t = simulate(&with(&[2], &[], exec.clone()), &b, 0, 9);
        assert_eq!((t[0].exit_i, t[0].exit_px, t[0].reason.as_str()), (4, 100.45, "exit.stop_loss"));
        let t = simulate(&with(&[2], &[], exec.clone()), &bars(), 0, 9); // highs reach 105.575 on bar 5 (high 106)
        assert_eq!((t[0].exit_i, t[0].exit_px, t[0].reason.as_str()), (5, 105.575, "exit.target"));
        let mut g = bars();
        g.open[3] = 99.; g.low[3] = 98.; // gap through the stop
        assert_eq!((simulate(&with(&[2], &[], exec.clone()), &g, 0, 9)[0].exit_px), 99.);
        let mut g = bars();
        g.open[3] = 107.; g.high[3] = 108.; // gap through the target
        let t = simulate(&with(&[2], &[], exec), &g, 0, 9);
        assert_eq!((t[0].exit_px, t[0].reason.as_str()), (107., "exit.target (gap at the open)"));
    }
    #[test] fn stop_and_target_inside_one_bar_take_the_stop() {
        let exec = Exec { stop_loss: Some(Dist::Pct(2.)), target: Some(Dist::Pct(3.)), ..close_fill() };
        let mut b = bars();
        b.low[3] = 100.; b.high[3] = 106.;
        let t = simulate(&with(&[2], &[], exec), &b, 0, 9);
        assert_eq!((t[0].exit_i, t[0].exit_px, t[0].reason.as_str()), (3, 100.45, "exit.stop_loss"));
    }
    #[test] fn a_limit_fill_bar_assumes_the_stop_but_never_the_target() {
        let exec = Exec { limit: Some(Dist::Pct(1.)), stop_loss: Some(Dist::Pct(1.)), target: Some(Dist::Pct(1.)), exit_fill: Fill::Close, ..Exec::default() };
        let mut b = bars();
        b.low[3] = 101.; b.high[3] = 110.; // limit 101.475 fills; target 102.48975 inside the bar is not taken
        let t = simulate(&with(&[2], &[], exec.clone()), &b, 0, 9);
        assert_eq!((t[0].legs[0].i, t[0].exit_i, t[0].reason.as_str()), (3, 4, "exit.target (gap at the open)"));
        b.low[3] = 100.; // low also below the stop 100.46025: the stop is assumed hit after the fill
        let t = simulate(&with(&[2], &[], exec), &b, 0, 9);
        assert_eq!((t[0].exit_i, t[0].reason.as_str()), (3, "exit.stop_loss"));
        assert!((t[0].exit_px - 100.46025).abs() < 1e-9);
    }
    #[test] fn trailing_level_comes_from_bars_already_closed() {
        let exec = Exec { trailing: Some(Dist::Pct(2.)), ..close_fill() };
        let mut b = bars();
        // entry close 102.5 at bar 2. Bar 3 high 103.9 → level for bar 4 = 103.9 * 0.98 = 101.822.
        // Bar 3's own low 101.6 is below its own high's level but above the level known at its open (102.5 * 0.98 = 100.45).
        b.high[3] = 103.9; b.low[3] = 101.6;
        b.low[4] = 101.8; // touches 101.822 → exit at the level on bar 4
        let t = simulate(&with(&[2], &[], exec), &b, 0, 9);
        assert_eq!((t[0].exit_i, t[0].reason.as_str()), (4, "exit.trailing"));
        assert!((t[0].exit_px - 101.822).abs() < 1e-9);
    }
    #[test] fn atr_distances_are_read_at_the_signal_bar() {
        let mut c = with(&[2], &[], Exec { stop_loss: Some(Dist::Atr { mult: 2., length: 14 }), ..Exec::default() });
        c.atr.insert(14, vec![f64::NAN, f64::NAN, 1., 50., 50., 50., 50., 50., 50., 50.]);
        let mut b = bars();
        b.low[4] = 100.9; // stop = open of bar 3 (103) - 2 x ATR(bar 2 = 1) = 101; ATR 50 on the fill bar is not used
        let t = simulate(&c, &b, 0, 9);
        assert_eq!((t[0].legs[0].i, t[0].exit_i, t[0].exit_px), (3, 4, 101.));
        c.atr.insert(14, vec![f64::NAN; 10]); // ATR still warming up: no level, no stop
        assert!(simulate(&c, &b, 0, 9)[0].open);
    }
    #[test] fn short_levels_mirror_long_ones() {
        let exec = Exec { short: true, stop_loss: Some(Dist::Pct(2.)), target: Some(Dist::Pct(3.)), ..close_fill() };
        // short at the close 102.5 → stop 104.55 above, target 99.425 below; bar 4's high 105 reaches the stop
        let t = simulate(&with(&[2], &[], exec.clone()), &bars(), 0, 9);
        assert_eq!((t[0].dir, t[0].exit_i, t[0].reason.as_str()), (-1., 4, "exit.stop_loss"));
        assert!((t[0].exit_px - 104.55).abs() < 1e-9);
        let mut b = bars();
        b.low[3] = 99.;
        let t = simulate(&with(&[2], &[], exec), &b, 0, 9);
        assert_eq!((t[0].exit_i, t[0].reason.as_str()), (3, "exit.target"));
        assert!((t[0].exit_px - 99.425).abs() < 1e-9);
    }
    #[test] fn a_next_open_fill_can_stop_out_on_its_own_bar() {
        let mut b = bars();
        b.low[3] = 101.; // fill at the open 103, stop 101.97
        let t = simulate(&with(&[2], &[], Exec { stop_loss: Some(Dist::Pct(1.)), ..Exec::default() }), &b, 0, 9);
        assert_eq!((t[0].legs[0].i, t[0].exit_i, t[0].reason.as_str()), (3, 3, "exit.stop_loss"));
        assert!((t[0].exit_px - 101.97).abs() < 1e-9);
    }
    #[test] fn future_bars_never_change_past_trades() {
        use crate::lab::series::{compile, Cache, Frames};
        let specs = [
            r#"{"v":1,"entry":{"all":[{"lhs":{"s":"rsi","p":{"length":2}},"op":"<","rhs":{"value":20}}]},"exit":{"any":[{"lhs":{"s":"rsi","p":{"length":2}},"op":">","rhs":{"value":70}}]}}"#,
            r#"{"v":1,"entry":{"all":[{"lhs":{"s":"rsi","p":{"length":2}},"op":"<","rhs":{"value":20}}]},"exit":{"any":[],"stop_loss":{"pct":2},"target":{"pct":3}},"execution":{"fill":"close"}}"#,
            r#"{"v":1,"entry":{"all":[{"lhs":{"s":"close"},"op":">","rhs":{"s":"sma","p":{"length":5}}}]},"exit":{"any":[{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":8}}],"trailing":{"atr":2,"atr_length":10}}}"#,
            r#"{"v":1,"direction":"short","entry":{"all":[{"lhs":{"s":"rsi","p":{"length":2}},"op":">","rhs":{"value":80}}],"limit":{"atr":0.4,"atr_length":10}},"exit":{"any":[{"lhs":{"s":"rsi","p":{"length":2}},"op":"<","rhs":{"value":40}}],"stop_loss":{"atr":3}},"execution":{"exit_fill":"close"}}"#,
        ];
        let full = crate::lab::testdata::walk(400, 5, false);
        for text in specs {
            let spec = crate::lab::rules::parse_spec(text).unwrap();
            let run = |b: &Bars| simulate(&compile(&spec, &Frames::new(b.clone()), &mut Cache::new()).unwrap(), b, 0, b.len() - 1);
            let want = run(&full);
            assert!(want.len() > 3, "{text}: only {} trades", want.len());
            for k in [120usize, 233, 351] {
                let mut alt = full.clone();
                for i in k + 1..alt.len() { for v in [&mut alt.open, &mut alt.high, &mut alt.low, &mut alt.close] { v[i] *= 0.7; } }
                let done = |t: &[Trade]| fills(&t.iter().filter(|t| !t.open && t.exit_i <= k).cloned().collect::<Vec<_>>());
                assert_eq!(done(&run(&alt)), done(&want), "{text}: bars after {k} changed earlier trades");
            }
        }
    }
```

- [ ] **Step 2: Run to verify failure**

Run: `cd Context/guru/backend && CARGO_BUILD_JOBS=2 cargo test lab::simulate`
Expected: the new tests FAIL — price exits are ignored, so positions with only a stop/target never close (e.g. `stop_and_target_fill_at_their_levels_or_the_gapped_open` panics on `t[0].exit_i`, `future_bars_never_change_past_trades` reports "only 1 trades" for the stop/target spec).

- [ ] **Step 3: Implement** — replace everything above `#[cfg(test)]` in `simulate.rs` with:

```rust
//! Single-instrument simulation (spec §4, Customize §5). One position at a time; book-format `connors::Trade`s so
//! trade_validation, equity and stats are reused read-only.
//! Order of events on bar i: (1) at the open a pending exit fills, then a pending market entry, then a resting limit
//! entry (low/high touch, filled at the better of open and limit); (2) inside the bar, stop-loss / trailing / target
//! levels (gap through a level fills at the open; stop and target both inside one bar = the stop, conservative);
//! (3) at the close, exit conditions (any) fill at this close or set a next-open exit, then entry conditions (all)
//! fill at this close, set a next-open market order or place a limit for the next bar.
use super::bars::Bars;
use super::rules::{Dist, Fill};
use super::series::Compiled;
use crate::connors::{Leg, Trade};

struct Pos { e: usize, px: f64, kind: &'static str, stop: f64, target: f64, best: f64, limit_bar: bool }
impl Pos {
    /// Levels are fixed from the entry price and ATR at `sig`, the bar whose close produced the order.
    fn open(c: &Compiled, i: usize, px: f64, sig: usize, kind: &'static str, d: f64) -> Pos {
        let lvl = |x: Option<Dist>, side: f64| x.map_or(f64::NAN, |x| px + side * d * c.dist(x, px, sig));
        Pos { e: i, px, kind, stop: lvl(c.exec.stop_loss, -1.), target: lvl(c.exec.target, 1.), best: px, limit_bar: kind == "limit" }
    }
    fn trade(&self, x: usize, px: f64, reason: String, signal_i: usize, d: f64) -> Trade {
        Trade { dir: d, legs: vec![Leg { i: self.e, px: self.px, w: 1. }], exit_i: x, exit_px: px, reason, entry_kind: self.kind, signal_i, open: false, exits: vec![] }
    }
    /// Stop-loss / trailing / target exit inside bar i (a close-fill entry is never checked on its own bar).
    fn price_exit(&self, c: &Compiled, b: &Bars, i: usize, d: f64) -> Option<(f64, String)> {
        // Trailing level from the best price through the previous bar and ATR at the previous close: known before bar i opens.
        let trail = c.exec.trailing.map_or(f64::NAN, |t| self.best - d * c.dist(t, self.best, i - 1));
        let (stop, why) = match (self.stop.is_nan(), trail.is_nan()) {
            (false, true) => (self.stop, "exit.stop_loss"),
            (true, false) => (trail, "exit.trailing"),
            (false, false) if d * (trail - self.stop) > 0. => (trail, "exit.trailing"),
            (false, false) => (self.stop, "exit.stop_loss"),
            (true, true) => (f64::NAN, ""),
        };
        let (o, adverse, favour) = (b.open[i], if d > 0. { b.low[i] } else { b.high[i] }, if d > 0. { b.high[i] } else { b.low[i] });
        if self.e < i {
            if d * (o - stop) <= 0. { return Some((o, format!("{why} (gap at the open)"))); }
            if d * (o - self.target) >= 0. { return Some((o, "exit.target (gap at the open)".into())); }
        }
        if d * (adverse - stop) <= 0. { return Some((stop, why.into())); }
        // On a limit-fill bar the high/low may precede the fill, so only the (adverse) stop is assumed reachable.
        if !self.limit_bar && d * (favour - self.target) >= 0. { return Some((self.target, "exit.target".into())); }
        None
    }
}

pub fn simulate(c: &Compiled, b: &Bars, from: usize, to: usize) -> Vec<Trade> {
    let x = &c.exec;
    let d = if x.short { -1. } else { 1. };
    // v1 rule kept: a next-open market entry may share the open with a next-open exit. Close fills and limit orders
    // follow the books instead: no new order on a bar that held a position at its close or exited after its open.
    let shares_open = x.limit.is_none() && x.fill == Fill::NextOpen;
    let mut trades = Vec::new();
    let mut pos: Option<Pos> = None;
    let mut pending_entry: Option<usize> = None;
    let mut pending_limit: Option<(usize, f64)> = None;
    let mut pending_exit: Option<(usize, String)> = None;
    for i in from..=to {
        if let Some((sig, why)) = pending_exit.take() {
            if let Some(p) = pos.take() { trades.push(p.trade(i, b.open[i], why, sig, d)); }
        }
        if let Some(sig) = pending_entry.take() { pos = Some(Pos::open(c, i, b.open[i], sig, "next_open", d)); }
        if let Some((sig, lim)) = pending_limit.take() {
            let touched = if d > 0. { b.low[i] <= lim } else { b.high[i] >= lim };
            if touched { pos = Some(Pos::open(c, i, if d > 0. { b.open[i].min(lim) } else { b.open[i].max(lim) }, sig, "limit", d)); }
        }
        let mut exited = false; // after the open: blocks a same-bar order under book rules
        if let Some((px, why)) = pos.as_ref().and_then(|p| p.price_exit(c, b, i, d)) {
            trades.push(pos.take().unwrap().trade(i, px, why, i, d));
            exited = true;
        }
        let last = i == to;
        if let Some(p) = &pos {
            if let Some(r) = c.exit.iter().find(|r| r.holds(i, Some(p.e))) {
                match x.exit_fill {
                    Fill::Close => { trades.push(pos.take().unwrap().trade(i, b.close[i], r.label.clone(), i, d)); exited = true; }
                    Fill::NextOpen if !last => pending_exit = Some((i, r.label.clone())),
                    Fill::NextOpen => {} // no next open inside the window; the position is marked open below
                }
            }
        }
        if let Some(p) = pos.as_mut() {
            if !(p.limit_bar && p.e == i) { p.best = if d > 0. { p.best.max(b.high[i]) } else { p.best.min(b.low[i]) }; }
        }
        if last { break; } // a signal on the last bar has no fill inside the window
        let may_enter = if shares_open { pos.is_none() || pending_exit.is_some() } else { pos.is_none() && !exited };
        if may_enter && c.entry.iter().all(|r| r.holds(i, None)) {
            match (x.limit, x.fill) {
                (Some(l), _) => pending_limit = Some((i, b.close[i] - d * c.dist(l, b.close[i], i))),
                (None, Fill::Close) => pos = Some(Pos::open(c, i, b.close[i], i, "close", d)),
                (None, Fill::NextOpen) => pending_entry = Some(i),
            }
        }
    }
    if let Some(p) = pos { trades.push(Trade { open: true, ..p.trade(to, b.close[to], "open".into(), to, d) }); }
    trades
}
```

- [ ] **Step 4: Run to verify pass**

Run: `cd Context/guru/backend && CARGO_BUILD_JOBS=2 cargo test lab::`
Expected: all pass (on `d165472`: 137 lab tests).

- [ ] **Step 5: Commit**

```bash
git add Context/guru/backend/src/lab/simulate.rs
git commit -m "Add stop-loss, target and trailing exits with conservative intrabar fills"
```

---

### Task 4: Trade-list comparison for the fidelity check (`lab/fidelity.rs`)

**Files:**
- Create: `Context/guru/backend/src/lab/fidelity.rs`
- Modify: `Context/guru/backend/src/lab/mod.rs` (add `pub mod fidelity;` in alphabetical position, after `pub mod catalogue;`)

**Interfaces:**
- Produces: `pub const FIDELITY_LABEL: &str`, `pub const FIDELITY_NOTE: &str`, `pub const MAX_DIFFS: usize = 50`; `pub fn same(a: &Trade, b: &Trade) -> bool`; `pub struct Comparison<'a> { pub matched: usize, pub diffs: Vec<(Option<&'a Trade>, Option<&'a Trade>)> }`; `pub fn compare<'a>(book: &'a [Trade], lab: &'a [Trade]) -> Comparison<'a>` (pairs by entry bar; `(Some, Some)` = same entry bar but different fills, `(Some, None)` = book only, `(None, Some)` = template only).

- [ ] **Step 1: Write the file with its test first** — create `lab/fidelity.rs` with only the `#[cfg(test)] mod tests { … }` block from Step 3 and `use crate::connors::Trade;`, add `pub mod fidelity;` to `mod.rs`.

- [ ] **Step 2: Run to verify failure**

Run: `cd Context/guru/backend && CARGO_BUILD_JOBS=2 cargo test lab::fidelity`
Expected: compile errors — `cannot find function compare / same`.

- [ ] **Step 3: Implement** — the complete file:

```rust
//! Template fidelity (Customize spec §6): the unmodified template's trades next to the book engine's trades on the
//! same instrument and window. Display only — never a verification, never stored. Book code is only called.
use crate::connors::Trade;

pub const FIDELITY_LABEL: &str = "Template fidelity · display only · not a verification";
pub const FIDELITY_NOTE: &str = "Compares the template's simulated trades with the book engine's simulated trades on the same data, window and default parameters, before costs. It does not verify the book method, its published figures or either engine.";
/// At most this many differences are listed (the counts cover all of them).
pub const MAX_DIFFS: usize = 50;

fn same_px(a: f64, b: f64) -> bool { (a - b).abs() <= 1e-9 * a.abs().max(b.abs()).max(1.) }
/// Same direction, one leg each, same entry bar and price, same exit bar and price, same open flag.
pub fn same(a: &Trade, b: &Trade) -> bool {
    a.dir == b.dir && a.legs.len() == 1 && b.legs.len() == 1 && a.legs[0].i == b.legs[0].i && same_px(a.legs[0].px, b.legs[0].px)
        && a.exit_i == b.exit_i && same_px(a.exit_px, b.exit_px) && a.open == b.open
}
pub struct Comparison<'a> { pub matched: usize, pub diffs: Vec<(Option<&'a Trade>, Option<&'a Trade>)> }
/// Pairs trades by entry bar (both lists are chronological and non-overlapping).
pub fn compare<'a>(book: &'a [Trade], lab: &'a [Trade]) -> Comparison<'a> {
    let (mut i, mut j, mut matched, mut diffs) = (0, 0, 0, Vec::new());
    while i < book.len() || j < lab.len() {
        let (b, l) = (book.get(i), lab.get(j));
        match (b.map(|t| t.legs[0].i), l.map(|t| t.legs[0].i)) {
            (Some(x), Some(y)) if x == y => {
                if same(b.unwrap(), l.unwrap()) { matched += 1 } else { diffs.push((b, l)) }
                i += 1; j += 1;
            }
            (Some(x), Some(y)) if x < y => { diffs.push((b, None)); i += 1; }
            (Some(_), None) => { diffs.push((b, None)); i += 1; }
            _ => { diffs.push((None, l)); j += 1; }
        }
    }
    Comparison { matched, diffs }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connors::Leg;
    fn t(e: usize, x: usize, px: f64) -> Trade { Trade { dir: 1., legs: vec![Leg { i: e, px: 100., w: 1. }], exit_i: x, exit_px: px, reason: String::new(), entry_kind: "close", signal_i: x, open: false, exits: vec![] } }
    #[test] fn pairs_by_entry_bar_and_names_each_difference() {
        let book = [t(1, 3, 110.), t(5, 7, 105.), t(9, 10, 99.)];
        let lab = [t(1, 3, 110. + 1e-12), t(5, 8, 105.), t(12, 14, 101.)];
        let c = compare(&book, &lab);
        assert_eq!(c.matched, 1);
        let kinds: Vec<(Option<usize>, Option<usize>)> = c.diffs.iter().map(|(b, l)| (b.map(|t| t.legs[0].i), l.map(|t| t.legs[0].i))).collect();
        assert_eq!(kinds, vec![(Some(5), Some(5)), (Some(9), None), (None, Some(12))]);
        let mut short = t(1, 3, 110.);
        short.dir = -1.;
        assert!(!same(&short, &book[0]));
        let mut two = t(1, 3, 110.);
        two.legs.push(Leg { i: 2, px: 101., w: 0.5 });
        assert!(!same(&two, &book[0]), "a scale-in trade never counts as reproduced");
    }
}
```

- [ ] **Step 4: Run to verify pass**

Run: `cd Context/guru/backend && CARGO_BUILD_JOBS=2 cargo test lab::fidelity`
Expected: 1 passed (lab total 138 on `d165472`).

- [ ] **Step 5: Commit**

```bash
git add Context/guru/backend/src/lab/fidelity.rs Context/guru/backend/src/lab/mod.rs
git commit -m "Add trade-by-trade comparison for template fidelity"
```

---
### Task 5: Template module, reasons, status mirror and the Connors STC IX templates (`lab/templates.rs`)

**Files:**
- Create: `Context/guru/backend/src/lab/templates.rs`
- Modify: `Context/guru/backend/src/lab/mod.rs` (add `pub mod templates;` after `pub mod simulate;`)

**Interfaces:**
- Consumes: `rules::{parse_spec, Spec, Origin}` (Task 1), `series::{compile, Cache, Frames}`, `simulate::simulate` (Tasks 2–3), `fidelity::compare` (Task 4); book: `connors::{catalogue, run, Status, StrategyDef, Ctx, Params}`, `gate::get`, `instrument_policy::{policy, reviewed_cash_rule}`, `crate::catalogue_json` (tests only).
- Produces: `pub struct Template { pub method: &'static str, pub version: u32, pub spec: &'static str }`; `pub const TEMPLATES: &[Template]`; `pub const TEMPLATE_LABEL`, `pub const COMMON_NOTES: &[&str]`; `pub fn variant_label(&StrategyDef) -> String`; `pub fn method(id) -> Option<StrategyDef>`; `pub fn template(method) -> Option<&'static Template>`; `pub fn book_status(&StrategyDef) -> (&'static str, &'static str)`; `pub fn unavailable(&StrategyDef) -> Option<String>`; `pub fn parsed(&Template) -> Spec`; reason constants and `pub const NO_TEMPLATE: &[(&str, &str)]`.

Each template is a JSON spec whose `origin` names its own method, `execution` says how the book fills (`"fill":"close"` for "buy/sell on the close"; a limit template sets `entry.limit` and `"exit_fill":"close"`), and whose conditions are the book's rules with its default parameters (from `/api/guru/connors/params`; e.g. `rsi2_4`: trend 200, rsi_n 2, rsi_lt 4, exit_rsi 65). "N days in a row" becomes one condition per bar with `offset`; "lowest close of the last 10 days" becomes `close <= close[k]` for k = 1…9 (the book compares `c[i] <= x` for every x in the window); "today's low is the lowest low of 10 days" is `low <= lowest(10)`; "2-day return > 3%" is `roc(2) > 3`.

- [ ] **Step 1: Write the failing tests** — create `lab/templates.rs` containing only this test module (and add `pub mod templates;` to `mod.rs`):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::lab::bars::Bars;
    use crate::lab::fidelity::compare;
    use crate::lab::series::{compile, Cache, Frames};
    use crate::lab::simulate::simulate;
    use crate::{connors::{Ctx, Params}, Row};

    /// Business-like dates (monotonic, valid) and a random walk with a slow trend cycle so trend filters flip both ways.
    fn fixture(n: usize, seed: u64, vol: f64) -> &'static [Row] {
        let mut s = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let mut next = || { s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407); ((s >> 11) as f64) / ((1u64 << 53) as f64) };
        let mut c = 100.0;
        let rows: Vec<Row> = (0..n).map(|i| {
            let drift = 0.0015 * (i as f64 / 90.).sin();
            let o = c * (1. + (next() - 0.5) * vol * 0.5);
            let cl = o * (1. + drift + (next() - 0.5) * vol * 2.);
            let h = o.max(cl) * (1. + next() * vol);
            let l = o.min(cl) * (1. - next() * vol);
            c = cl;
            Row { ticker: "SPX".into(), date: format!("{}-{:02}-{:02}", 2000 + i / 250, 1 + (i % 250) / 21, 1 + i % 21), open: o, high: h, low: l, close: cl, volume: Some(1e6) }
        }).collect();
        Box::leak(rows.into_boxed_slice())
    }
    fn fixtures() -> Vec<&'static [Row]> { vec![fixture(1500, 1, 0.02), fixture(1500, 2, 0.02), fixture(1500, 3, 0.05), fixture(1500, 4, 0.08)] }

    #[test] fn every_template_parses_names_its_method_and_comes_from_an_admitted_book_method() {
        for t in TEMPLATES {
            let s = parsed(t);
            let d = method(t.method).unwrap_or_else(|| panic!("{}: not in the book catalogue", t.method));
            assert_eq!(s.origin, Some(rules::Origin { book: d.book.into(), method: t.method.into(), version: t.version }), "{}", t.method);
            assert_eq!(book_status(&d).0, "live", "{}: templates come only from admitted (live) methods", t.method);
            assert_eq!(s.exec.short, d.dir == "Short", "{}: direction", t.method);
            assert!(unavailable(&d).is_none());
        }
        assert!(TEMPLATES.iter().all(|t| !NO_TEMPLATE.iter().any(|(id, _)| *id == t.method)), "a method is either templated or has a reason");
    }
    #[test] fn book_status_matches_the_guru_catalogue() {
        for g in ["connors", "ross", "chande"] {
            let cat = crate::lab::json::parse(&crate::catalogue_json(g)).unwrap();
            let crate::lab::json::Json::Arr(items) = cat else { panic!() };
            for it in items {
                let id = it.get("id").unwrap().as_str().unwrap();
                let d = method(id).unwrap();
                assert_eq!(book_status(&d).0, it.get("status").unwrap().as_str().unwrap(), "{id}");
            }
        }
    }
    #[test] fn unavailable_reasons_are_specific() {
        assert!(unavailable(&method("r3").unwrap()).unwrap().contains("scale-in"));
        assert!(unavailable(&method("vix2").unwrap()).unwrap().contains("VIX"));
        let not_live = connors::catalogue().into_iter().find(|d| book_status(d).0 != "live").unwrap();
        assert!(unavailable(&not_live).unwrap().starts_with("the book method is not admitted for backtesting"));
    }
    #[test] fn templates_reproduce_the_book_engine_on_fixtures() {
        let mut report = Vec::new();
        for t in TEMPLATES {
            let spec = parsed(t);
            let (mut trades, mut ok) = (0, true);
            for rows in fixtures() {
                let ctx = Ctx::new(rows, rows, &[]);
                let to = rows.len() - 1;
                let book = connors::run(t.method, &ctx, 0, to, &Params::new()).unwrap();
                let fr = Frames::new(Bars::from_rows(rows));
                let lab = simulate(&compile(&spec, &fr, &mut Cache::new()).unwrap(), &fr.d, 0, to);
                let c = compare(&book, &lab);
                trades += book.len();
                if !c.diffs.is_empty() {
                    ok = false;
                    let (b, l) = c.diffs[0];
                    report.push(format!("{}: {}/{} matched; first difference book {:?} lab {:?}", t.method, c.matched, book.len(),
                        b.map(|t| (t.legs[0].i, t.legs[0].px, t.exit_i, t.exit_px, t.legs.len())), l.map(|t| (t.legs[0].i, t.legs[0].px, t.exit_i, t.exit_px))));
                }
            }
            if ok && trades == 0 { report.push(format!("{}: no book trades on any fixture (vacuous)", t.method)); }
        }
        assert!(report.is_empty(), "{}", report.join("\n"));
    }
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cd Context/guru/backend && CARGO_BUILD_JOBS=2 cargo test lab::templates`
Expected: compile errors — `cannot find value TEMPLATES`, functions `method`, `book_status`, `unavailable`, `parsed` not found.

- [ ] **Step 3: Implement** — put this above the test module (the `TEMPLATES` list starts with the 18 STC IX templates; Tasks 6 and 7 append the rest):

```rust
//! Book templates (Customize spec §4): declared adaptations of admitted book methods into lab rule specs.
//! A template is a starting point for a user variant; it never changes, replaces or verifies the book method.
//! Book code is read only: the catalogue for names, status and instrument policy; `connors::run` in the fidelity tests.
use super::rules::{self, Spec};
use crate::connors::{self, Status, StrategyDef};

pub struct Template { pub method: &'static str, pub version: u32, pub spec: &'static str }
pub const TEMPLATE_LABEL: &str = "Book template · declared adaptation into lab rules · not a verification";
/// Adaptations every template declares (method-specific ones are listed with the template).
pub const COMMON_NOTES: &[&str] = &[
    "Indicators are the lab's TradingView definitions, not the book engine's implementations; edge values can differ (e.g. the lab RSI is 100 when a window has no losses and no gains, the book's Wilder RSI is 50).",
    "Lab comparisons are exact; some book engines compare with a 1e-9 relative tie band.",
    "Orders fill as the template's execution says (signal close, next open or a limit on the next bar); costs come from the lab run settings.",
    "Book-reported figures, gate verdicts and readiness are not part of a template.",
];

/// Readable label for a variant run (spec §9).
pub fn variant_label(def: &StrategyDef) -> String { format!("Customized from {} · {} · user variant · simulated", def.book, def.name) }

pub fn method(id: &str) -> Option<StrategyDef> { connors::catalogue().into_iter().find(|d| d.id == id) }
pub fn template(method: &str) -> Option<&'static Template> { TEMPLATES.iter().find(|t| t.method == method) }
/// The catalogue status the guru pages show (same rule as main.rs catalogue_json, read-only).
pub fn book_status(d: &StrategyDef) -> (&'static str, &'static str) {
    let gated = crate::gate::get(d.id);
    match d.status {
        Status::Live if gated.is_some_and(|g| g.verdict == "withheld") && !crate::instrument_policy::reviewed_cash_rule(d) => ("not_testable", gated.map_or("", |g| g.reason)),
        Status::Live if crate::instrument_policy::policy(d).requirement == "source_review_required" => ("not_testable", "Source-specific rules and required instrument have not completed admission review; no backtest is published."),
        Status::Live => ("live", ""),
        Status::NeedsData(w) => ("needs_data", w),
        Status::NotTestable(w) => ("not_testable", w),
    }
}
/// Why a method has no template ("Customize not available — <reason>"); None when it has one.
pub fn unavailable(d: &StrategyDef) -> Option<String> {
    if template(d.id).is_some() { return None; }
    let (status, why) = book_status(d);
    if status != "live" { return Some(format!("the book method is not admitted for backtesting ({status}): {why}")); }
    let named = NO_TEMPLATE.iter().find(|(id, _)| *id == d.id).map(|(_, r)| *r);
    let two_sided = (d.dir != "Long" && d.dir != "Short").then_some(TWO_SIDED);
    Some(named.or(two_sided).unwrap_or(NOT_YET).to_string())
}
pub fn parsed(t: &Template) -> Spec { rules::parse_spec(t.spec).expect("templates are checked by tests") }

pub const NOT_YET: &str = "no template has been written for this method yet";
pub const SCALE_IN: &str = "the method adds units while in a trade (scale-in), which the lab spec does not simulate yet";
pub const TWO_SIDED: &str = "the method is long and short in one strategy (always in or reversing); a lab strategy has one direction";
pub const VIX: &str = "the rules read the VIX, a second instrument; lab rules read the traded instrument only";
pub const SERIES: &str = "the rules read an external series (e.g. TRIN) that lab rules cannot reference";
pub const FUTURES: &str = "the method needs futures contract prices, which the lab does not admit";
pub const CALENDAR: &str = "the rules use calendar dates (day of month, month end, named months); the lab has no calendar operand";
pub const SUM: &str = "the rules add an indicator over several days; lab conditions compare single values";
pub const RANGE: &str = "the rules place the close within the day's range (a ratio of bar prices); lab conditions compare single values";
pub const BOOK_FILTERS: &str = "the rules use book-specific series (Liquidity Filter against the benchmark, the book's HV and ADX definitions or HV-scaled stretches)";
pub const CUSTOM: &str = "the rules use an indicator the lab catalogue does not have (e.g. a smoothed oscillator of an oscillator, MAMA, CGO)";
pub const COUNT: &str = "the rules count days in a window (e.g. 4 of the last 5); lab conditions are all-of or any-of";
pub const EXIT_ALL: &str = "the exit needs several conditions together, but lab exits fire on any one condition";
pub const INTRADAY: &str = "the method trades within the day (today's open, stop entries above the open, buy the close and sell the next open)";
pub const PORTFOLIO: &str = "the rules include portfolio settings (position caps, ranking, cash in another ETF)";
pub const FILL_CONVENTION: &str = "the book fills a limit at the limit price even when the bar opens beyond it (AmiBroker BuyPrice); the lab fills a gapped limit at the open, so a template would not reproduce the book engine";
pub const SIGNAL_CLOCK: &str = "the time exit counts bars since the latest buy signal, which repeats while in the trade (AmiBroker BarsSince); lab bars_held counts from the fill";
pub const MULTI_DAY: &str = "the entry is a multi-day setup with a pending order window, which the lab does not model";
pub const NO_TEMPLATE: &[(&str, &str)] = &[
    ("r3", SCALE_IN), ("tps_stocks", SCALE_IN), ("tps_long", SCALE_IN), ("tps_short", SCALE_IN), ("s12345", SCALE_IN),
    ("x555", BOOK_FILTERS), ("h10", BOOK_FILTERS),
    ("eom_co", CALENDAR), ("eom_rsi", CALENDAR), ("eom_cc", CALENDAR), ("spmt_eom_co", CALENDAR), ("spmt_eom_rsi", CALENDAR), ("spmt_eom_cc", CALENDAR),
    ("rsi_1", SUM), ("rsi_2", SUM), ("rsi_3", SUM), ("spmt_rsi_1", SUM), ("spmt_rsi_2", SUM), ("spmt_rsi_3", SUM),
    ("pctb_2", RANGE), ("spmt_pctb_2", RANGE), ("spmt_price", RANGE),
    ("vix200", VIX), ("vix2", VIX), ("vix4", VIX), ("vix7", VIX), ("vix8", VIX), ("vix53_short", VIX), ("vix101_short", VIX),
    ("r25_rsi25_vix", VIX), ("r25_rsi25_15", SCALE_IN), ("r25_rsi25_15_vix", VIX),
    ("ss_long", SCALE_IN), ("ss_short", SCALE_IN), ("ss_both", TWO_SIDED),
    ("trin4", SERIES), ("trin4_12", SERIES), ("trin3_10", SERIES), ("trin3_11", SERIES), ("trin2_11", SERIES),
    ("ats_dvmtm", MULTI_DAY), ("pbs_percent_b_swing", TWO_SIDED), ("pbs_percent_b_plus_one", TWO_SIDED),
    ("alpha_spy_rsi4_mr", SCALE_IN), ("alpha_etf_avalanches_single", PORTFOLIO), ("radge_ma220", FUTURES),
    ("ststw_overnight", INTRADAY), ("ststw_double7", EXIT_ALL),
    ("hpetf_3dhl_long_aggr", SCALE_IN), ("hpetf_rsi25_long_aggr", SCALE_IN), ("hpetf_r3_long_aggr", SCALE_IN), ("hpetf_pctb_long_aggr", SCALE_IN),
    ("hpetf_pctb_short_aggr", SCALE_IN), ("hpetf_mdd_long", COUNT), ("hpetf_mdd_long_aggr", SCALE_IN), ("hpetf_mdu_short", COUNT),
    ("hpetf_mdu_short_aggr", SCALE_IN), ("hpetf_rsi10_6_long", SCALE_IN), ("hpetf_rsi90_94_short", SCALE_IN), ("hpetf_tps_long", SCALE_IN), ("hpetf_tps_short", SCALE_IN),
    ("afactor_baseline", INTRADAY), ("r3r4_base", BOOK_FILTERS), ("r3r4_base_scale", SCALE_IN), ("r3r4_bigcap", BOOK_FILTERS), ("r3r4_bigcap_scale", SCALE_IN),
    ("r3r4_bigcap_deck", BOOK_FILTERS), ("r3r4_bigcap_deck_scale", SCALE_IN), ("r3r4_midcap", BOOK_FILTERS), ("r3r4_midcap_scale", SCALE_IN),
    ("mmft_nasdaq79", TWO_SIDED), ("lwdg2_vol_up_up", FUTURES), ("lwdg2_vol_dn_up", FUTURES), ("lwdg2_vol_up_dn", FUTURES), ("lwdg2_vol_dn_dn", FUTURES), ("lwdg2_weekly_entry", FUTURES),
    ("kftsm_month_end", CALENDAR), ("kftsm_hirsch", CALENDAR), ("lwrsr_oct_apr", CALENDAR), ("lwrsr_apr_oct", CALENDAR),
    ("ehmd_mama", CUSTOM), ("ch_cbpb_time", CUSTOM),
    ("hbqts_fixed_interval", CUSTOM), ("hbqts_bb_breakout", TWO_SIDED), ("hbqts_cgo", CUSTOM), ("hbqts_stoch_rsi", CUSTOM), ("hbqts_dpo", CUSTOM),
    ("hbqts_dom", TWO_SIDED), ("hbqts_limit_entry", FILL_CONVENTION), ("hbqts_entry_id1", SIGNAL_CLOCK), ("hbqts_macd_daily", CUSTOM), ("hbqts_cmo", CUSTOM),
];

pub const TEMPLATES: &[Template] = &[
    Template { method: "f1a", version: 1, spec: r#"{"v":1,"direction":"long","origin":{"book":"stc9","method":"f1a","template_version":1},"entry":{"all":[{"lhs":{"s":"close"},"op":">","rhs":{"s":"sma","p":{"length":200}}}]},"exit":{"any":[{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":5}}]},"execution":{"fill":"close"}}"# },
    Template { method: "f3a", version: 1, spec: r#"{"v":1,"direction":"long","origin":{"book":"stc9","method":"f3a","template_version":1},"entry":{"all":[{"lhs":{"s":"close"},"op":">","rhs":{"s":"sma","p":{"length":200}}},{"lhs":{"s":"close"},"op":"<=","rhs":{"s":"close","offset":1}},{"lhs":{"s":"close"},"op":"<=","rhs":{"s":"close","offset":2}},{"lhs":{"s":"close"},"op":"<=","rhs":{"s":"close","offset":3}},{"lhs":{"s":"close"},"op":"<=","rhs":{"s":"close","offset":4}},{"lhs":{"s":"close"},"op":"<=","rhs":{"s":"close","offset":5}},{"lhs":{"s":"close"},"op":"<=","rhs":{"s":"close","offset":6}},{"lhs":{"s":"close"},"op":"<=","rhs":{"s":"close","offset":7}},{"lhs":{"s":"close"},"op":"<=","rhs":{"s":"close","offset":8}},{"lhs":{"s":"close"},"op":"<=","rhs":{"s":"close","offset":9}}]},"exit":{"any":[{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":5}}]},"execution":{"fill":"close"}}"# },
    Template { method: "f4a", version: 1, spec: r#"{"v":1,"direction":"long","origin":{"book":"stc9","method":"f4a","template_version":1},"entry":{"all":[{"lhs":{"s":"close"},"op":">","rhs":{"s":"sma","p":{"length":200}}},{"lhs":{"s":"low"},"op":"<=","rhs":{"s":"lowest","p":{"length":10}}}],"limit":{"pct":3}},"exit":{"any":[{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":5}}]},"execution":{"exit_fill":"close"}}"# },
    Template { method: "f4c", version: 1, spec: r#"{"v":1,"direction":"long","origin":{"book":"stc9","method":"f4c","template_version":1},"entry":{"all":[{"lhs":{"s":"close"},"op":">","rhs":{"s":"sma","p":{"length":200}}},{"lhs":{"s":"low"},"op":"<=","rhs":{"s":"lowest","p":{"length":10}}}],"limit":{"pct":7}},"exit":{"any":[{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":5}}]},"execution":{"exit_fill":"close"}}"# },
    Template { method: "f4d", version: 1, spec: r#"{"v":1,"direction":"long","origin":{"book":"stc9","method":"f4d","template_version":1},"entry":{"all":[{"lhs":{"s":"close"},"op":">","rhs":{"s":"sma","p":{"length":200}}},{"lhs":{"s":"low"},"op":"<=","rhs":{"s":"lowest","p":{"length":10}}}],"limit":{"pct":7}},"exit":{"any":[{"lhs":{"s":"close"},"op":">","rhs":{"s":"sma","p":{"length":5}}}]},"execution":{"exit_fill":"close"}}"# },
    Template { method: "rsi2_5d", version: 1, spec: r#"{"v":1,"direction":"long","origin":{"book":"stc9","method":"rsi2_5d","template_version":1},"entry":{"all":[{"lhs":{"s":"close"},"op":">","rhs":{"s":"sma","p":{"length":200}}},{"lhs":{"s":"rsi","p":{"length":2}},"op":"<","rhs":{"value":2}}]},"exit":{"any":[{"lhs":{"s":"close"},"op":">","rhs":{"s":"sma","p":{"length":5}}}]},"execution":{"fill":"close"}}"# },
    Template { method: "rsi2_4", version: 1, spec: r#"{"v":1,"direction":"long","origin":{"book":"stc9","method":"rsi2_4","template_version":1},"entry":{"all":[{"lhs":{"s":"close"},"op":">","rhs":{"s":"sma","p":{"length":200}}},{"lhs":{"s":"rsi","p":{"length":2}},"op":"<","rhs":{"value":4}}]},"exit":{"any":[{"lhs":{"s":"rsi","p":{"length":2}},"op":">","rhs":{"value":65}}]},"execution":{"fill":"close"}}"# },
    Template { method: "rsi4_25", version: 1, spec: r#"{"v":1,"direction":"long","origin":{"book":"stc9","method":"rsi4_25","template_version":1},"entry":{"all":[{"lhs":{"s":"close"},"op":">","rhs":{"s":"sma","p":{"length":200}}},{"lhs":{"s":"rsi","p":{"length":4}},"op":"<","rhs":{"value":25}},{"lhs":{"s":"rsi","p":{"length":2}},"op":"<=","rhs":{"value":65}}]},"exit":{"any":[{"lhs":{"s":"rsi","p":{"length":2}},"op":">","rhs":{"value":65}}]},"execution":{"fill":"close"}}"# },
    Template { method: "rsi_div8", version: 1, spec: r#"{"v":1,"direction":"long","origin":{"book":"stc9","method":"rsi_div8","template_version":1},"entry":{"all":[{"lhs":{"s":"close"},"op":">","rhs":{"s":"sma","p":{"length":200}}},{"lhs":{"s":"close"},"op":"<","rhs":{"s":"close","offset":8}},{"lhs":{"s":"rsi","p":{"length":2}},"op":">","rhs":{"s":"rsi","p":{"length":2},"offset":8}},{"lhs":{"s":"rsi","p":{"length":2}},"op":"<","rhs":{"value":15}}]},"exit":{"any":[{"lhs":{"s":"rsi","p":{"length":2}},"op":">","rhs":{"value":65}}]},"execution":{"fill":"close"}}"# },
    Template { method: "rsi_div2", version: 1, spec: r#"{"v":1,"direction":"long","origin":{"book":"stc9","method":"rsi_div2","template_version":1},"entry":{"all":[{"lhs":{"s":"close"},"op":">","rhs":{"s":"sma","p":{"length":200}}},{"lhs":{"s":"close"},"op":"<","rhs":{"s":"close","offset":2}},{"lhs":{"s":"rsi","p":{"length":2}},"op":">","rhs":{"s":"rsi","p":{"length":2},"offset":2}},{"lhs":{"s":"rsi","p":{"length":2}},"op":"<","rhs":{"value":30}}]},"exit":{"any":[{"lhs":{"s":"rsi","p":{"length":2}},"op":">","rhs":{"value":65}}]},"execution":{"fill":"close"}}"# },
    Template { method: "pctb_1", version: 1, spec: r#"{"v":1,"direction":"long","origin":{"book":"stc9","method":"pctb_1","template_version":1},"entry":{"all":[{"lhs":{"s":"close"},"op":">","rhs":{"s":"sma","p":{"length":200}}},{"lhs":{"s":"bbpct","p":{"length":5,"mult":1}},"op":"<","rhs":{"value":0}},{"lhs":{"s":"bbpct","p":{"length":5,"mult":1},"offset":1},"op":"<","rhs":{"value":0}},{"lhs":{"s":"rsi","p":{"length":2}},"op":"<","rhs":{"value":10}}]},"exit":{"any":[{"lhs":{"s":"rsi","p":{"length":2}},"op":">","rhs":{"value":65}}]},"execution":{"fill":"close"}}"# },
    Template { method: "dmi_1", version: 1, spec: r#"{"v":1,"direction":"long","origin":{"book":"stc9","method":"dmi_1","template_version":1},"entry":{"all":[{"lhs":{"s":"close"},"op":">","rhs":{"s":"sma","p":{"length":200}}},{"lhs":{"s":"dmi","p":{"di_length":3},"out":"plus_di"},"op":"<","rhs":{"value":5}},{"lhs":{"s":"rsi","p":{"length":2}},"op":"<=","rhs":{"value":65}}]},"exit":{"any":[{"lhs":{"s":"rsi","p":{"length":2}},"op":">","rhs":{"value":65}}]},"execution":{"fill":"close"}}"# },
    Template { method: "dmi_2", version: 1, spec: r#"{"v":1,"direction":"long","origin":{"book":"stc9","method":"dmi_2","template_version":1},"entry":{"all":[{"lhs":{"s":"close"},"op":">","rhs":{"s":"sma","p":{"length":200}}},{"lhs":{"s":"dmi","p":{"di_length":2},"out":"plus_di"},"op":"<","rhs":{"value":3}},{"lhs":{"s":"rsi","p":{"length":2}},"op":"<=","rhs":{"value":65}}]},"exit":{"any":[{"lhs":{"s":"rsi","p":{"length":2}},"op":">","rhs":{"value":65}}]},"execution":{"fill":"close"}}"# },
    Template { method: "dmi_3", version: 1, spec: r#"{"v":1,"direction":"long","origin":{"book":"stc9","method":"dmi_3","template_version":1},"entry":{"all":[{"lhs":{"s":"close"},"op":">","rhs":{"s":"sma","p":{"length":200}}},{"lhs":{"s":"dmi","p":{"di_length":2},"out":"plus_di"},"op":"<","rhs":{"value":10}},{"lhs":{"s":"dmi","p":{"di_length":2},"out":"plus_di","offset":1},"op":"<","rhs":{"value":10}},{"lhs":{"s":"rsi","p":{"length":2}},"op":"<=","rhs":{"value":65}}]},"exit":{"any":[{"lhs":{"s":"rsi","p":{"length":2}},"op":">","rhs":{"value":65}}]},"execution":{"fill":"close"}}"# },
    Template { method: "dmi_4", version: 1, spec: r#"{"v":1,"direction":"long","origin":{"book":"stc9","method":"dmi_4","template_version":1},"entry":{"all":[{"lhs":{"s":"close"},"op":">","rhs":{"s":"sma","p":{"length":200}}},{"lhs":{"s":"dmi","p":{"di_length":2},"out":"plus_di"},"op":"<","rhs":{"value":10}},{"lhs":{"s":"dmi","p":{"di_length":2},"out":"plus_di","offset":1},"op":"<","rhs":{"value":10}},{"lhs":{"s":"dmi","p":{"di_length":2},"out":"plus_di","offset":2},"op":"<","rhs":{"value":10}},{"lhs":{"s":"rsi","p":{"length":2}},"op":"<=","rhs":{"value":65}}]},"exit":{"any":[{"lhs":{"s":"rsi","p":{"length":2}},"op":">","rhs":{"value":65}}]},"execution":{"fill":"close"}}"# },
    Template { method: "dmi_5", version: 1, spec: r#"{"v":1,"direction":"long","origin":{"book":"stc9","method":"dmi_5","template_version":1},"entry":{"all":[{"lhs":{"s":"close"},"op":">","rhs":{"s":"sma","p":{"length":200}}},{"lhs":{"s":"dmi","p":{"di_length":2},"out":"plus_di"},"op":"<","rhs":{"value":15}},{"lhs":{"s":"dmi","p":{"di_length":2},"out":"plus_di","offset":1},"op":"<","rhs":{"value":15}},{"lhs":{"s":"dmi","p":{"di_length":2},"out":"plus_di","offset":2},"op":"<","rhs":{"value":15}},{"lhs":{"s":"dmi","p":{"di_length":2},"out":"plus_di","offset":3},"op":"<","rhs":{"value":15}},{"lhs":{"s":"rsi","p":{"length":2}},"op":"<=","rhs":{"value":65}}]},"exit":{"any":[{"lhs":{"s":"rsi","p":{"length":2}},"op":">","rhs":{"value":65}}]},"execution":{"fill":"close"}}"# },
    Template { method: "short_4up", version: 1, spec: r#"{"v":1,"direction":"short","origin":{"book":"stc9","method":"short_4up","template_version":1},"entry":{"all":[{"lhs":{"s":"close"},"op":"<","rhs":{"s":"sma","p":{"length":200}}},{"lhs":{"s":"close"},"op":">","rhs":{"s":"close","offset":1}},{"lhs":{"s":"close","offset":1},"op":">","rhs":{"s":"close","offset":2}},{"lhs":{"s":"close","offset":2},"op":">","rhs":{"s":"close","offset":3}},{"lhs":{"s":"close","offset":3},"op":">","rhs":{"s":"close","offset":4}},{"lhs":{"s":"close"},"op":">=","rhs":{"s":"sma","p":{"length":5}}}]},"exit":{"any":[{"lhs":{"s":"close"},"op":"<","rhs":{"s":"sma","p":{"length":5}}}]},"execution":{"fill":"close"}}"# },
    Template { method: "short_strong", version: 1, spec: r#"{"v":1,"direction":"short","origin":{"book":"stc9","method":"short_strong","template_version":1},"entry":{"all":[{"lhs":{"s":"close"},"op":"<","rhs":{"s":"sma","p":{"length":200}}},{"lhs":{"s":"roc","p":{"length":2}},"op":">","rhs":{"value":3}},{"lhs":{"s":"rsi","p":{"length":2}},"op":">","rhs":{"value":80}},{"lhs":{"s":"close"},"op":">=","rhs":{"s":"sma","p":{"length":5}}}]},"exit":{"any":[{"lhs":{"s":"close"},"op":"<","rhs":{"s":"sma","p":{"length":5}}}]},"execution":{"fill":"close"}}"# },
];
```

- [ ] **Step 4: Run to verify pass**

Run: `cd Context/guru/backend && VAULT_IDX="…/idx_data.csv" CARGO_BUILD_JOBS=2 cargo test lab::templates`
Expected: 4 passed (lab total 142 on `d165472`). If `templates_reproduce_the_book_engine_on_fixtures` reports a template (e.g. after a Phase 6 change to `rsi`, `dmi` or `bbpct`), first check the template against the book's `run` arm in `connors.rs` (read-only); if the difference is an indicator or fill convention the lab will not copy, delete the template line, add `("<id>", <REASON>)` to `NO_TEMPLATE` with a new reason constant that names the convention, and record it in the commit message. Never edit book code.

- [ ] **Step 5: Commit**

```bash
git add Context/guru/backend/src/lab/templates.rs Context/guru/backend/src/lab/mod.rs
git commit -m "Add book templates module with STC IX templates and named reasons"
```

---

### Task 6: S&P market-timing and RSI 25 templates

**Files:**
- Modify: `Context/guru/backend/src/lab/templates.rs` (append to `TEMPLATES`)

**Interfaces:**
- Consumes/Produces: as Task 5 (14 more entries).

- [ ] **Step 1: Write the failing test** — append inside `mod tests` of `templates.rs`:

```rust
    #[test] fn spmt_and_rsi25_templates_are_listed() {
        for m in ["spmt_rsi2_4", "spmt_rsi4_25", "spmt_rsi_div8", "spmt_rsi_div2", "spmt_pctb_1", "spmt_dmi_1", "spmt_dmi_2", "spmt_dmi_3", "spmt_dmi_4", "spmt_dmi_5",
                  "spmt_short_4up", "spmt_short_strong", "r25_rsi25_basic_no_filter", "r25_rsi25_official_sma200"] {
            assert!(template(m).is_some(), "{m}");
        }
    }
```

- [ ] **Step 2: Run to verify failure**

Run: `cd Context/guru/backend && CARGO_BUILD_JOBS=2 cargo test lab::templates::tests::spmt_and_rsi25_templates_are_listed`
Expected: FAIL `spmt_rsi2_4`.

- [ ] **Step 3: Implement** — append these lines inside `TEMPLATES` (before `];`):

```rust
    Template { method: "spmt_rsi2_4", version: 1, spec: r#"{"v":1,"direction":"long","origin":{"book":"spmt","method":"spmt_rsi2_4","template_version":1},"entry":{"all":[{"lhs":{"s":"close"},"op":">","rhs":{"s":"sma","p":{"length":200}}},{"lhs":{"s":"rsi","p":{"length":2}},"op":"<","rhs":{"value":4}}]},"exit":{"any":[{"lhs":{"s":"rsi","p":{"length":2}},"op":">","rhs":{"value":65}}]},"execution":{"fill":"close"}}"# },
    Template { method: "spmt_rsi4_25", version: 1, spec: r#"{"v":1,"direction":"long","origin":{"book":"spmt","method":"spmt_rsi4_25","template_version":1},"entry":{"all":[{"lhs":{"s":"close"},"op":">","rhs":{"s":"sma","p":{"length":200}}},{"lhs":{"s":"rsi","p":{"length":4}},"op":"<","rhs":{"value":25}},{"lhs":{"s":"rsi","p":{"length":2}},"op":"<=","rhs":{"value":65}}]},"exit":{"any":[{"lhs":{"s":"rsi","p":{"length":2}},"op":">","rhs":{"value":65}}]},"execution":{"fill":"close"}}"# },
    Template { method: "spmt_rsi_div8", version: 1, spec: r#"{"v":1,"direction":"long","origin":{"book":"spmt","method":"spmt_rsi_div8","template_version":1},"entry":{"all":[{"lhs":{"s":"close"},"op":">","rhs":{"s":"sma","p":{"length":200}}},{"lhs":{"s":"close"},"op":"<","rhs":{"s":"close","offset":8}},{"lhs":{"s":"rsi","p":{"length":2}},"op":">","rhs":{"s":"rsi","p":{"length":2},"offset":8}},{"lhs":{"s":"rsi","p":{"length":2}},"op":"<","rhs":{"value":15}}]},"exit":{"any":[{"lhs":{"s":"rsi","p":{"length":2}},"op":">","rhs":{"value":65}}]},"execution":{"fill":"close"}}"# },
    Template { method: "spmt_rsi_div2", version: 1, spec: r#"{"v":1,"direction":"long","origin":{"book":"spmt","method":"spmt_rsi_div2","template_version":1},"entry":{"all":[{"lhs":{"s":"close"},"op":">","rhs":{"s":"sma","p":{"length":200}}},{"lhs":{"s":"close"},"op":"<","rhs":{"s":"close","offset":2}},{"lhs":{"s":"rsi","p":{"length":2}},"op":">","rhs":{"s":"rsi","p":{"length":2},"offset":2}},{"lhs":{"s":"rsi","p":{"length":2}},"op":"<","rhs":{"value":30}}]},"exit":{"any":[{"lhs":{"s":"rsi","p":{"length":2}},"op":">","rhs":{"value":65}}]},"execution":{"fill":"close"}}"# },
    Template { method: "spmt_pctb_1", version: 1, spec: r#"{"v":1,"direction":"long","origin":{"book":"spmt","method":"spmt_pctb_1","template_version":1},"entry":{"all":[{"lhs":{"s":"close"},"op":">","rhs":{"s":"sma","p":{"length":200}}},{"lhs":{"s":"bbpct","p":{"length":5,"mult":1}},"op":"<","rhs":{"value":0}},{"lhs":{"s":"bbpct","p":{"length":5,"mult":1},"offset":1},"op":"<","rhs":{"value":0}},{"lhs":{"s":"rsi","p":{"length":2}},"op":"<","rhs":{"value":10}}]},"exit":{"any":[{"lhs":{"s":"rsi","p":{"length":2}},"op":">","rhs":{"value":65}}]},"execution":{"fill":"close"}}"# },
    Template { method: "spmt_dmi_1", version: 1, spec: r#"{"v":1,"direction":"long","origin":{"book":"spmt","method":"spmt_dmi_1","template_version":1},"entry":{"all":[{"lhs":{"s":"close"},"op":">","rhs":{"s":"sma","p":{"length":200}}},{"lhs":{"s":"dmi","p":{"di_length":3},"out":"plus_di"},"op":"<","rhs":{"value":5}},{"lhs":{"s":"rsi","p":{"length":2}},"op":"<=","rhs":{"value":65}}]},"exit":{"any":[{"lhs":{"s":"rsi","p":{"length":2}},"op":">","rhs":{"value":65}}]},"execution":{"fill":"close"}}"# },
    Template { method: "spmt_dmi_2", version: 1, spec: r#"{"v":1,"direction":"long","origin":{"book":"spmt","method":"spmt_dmi_2","template_version":1},"entry":{"all":[{"lhs":{"s":"close"},"op":">","rhs":{"s":"sma","p":{"length":200}}},{"lhs":{"s":"dmi","p":{"di_length":2},"out":"plus_di"},"op":"<","rhs":{"value":3}},{"lhs":{"s":"rsi","p":{"length":2}},"op":"<=","rhs":{"value":65}}]},"exit":{"any":[{"lhs":{"s":"rsi","p":{"length":2}},"op":">","rhs":{"value":65}}]},"execution":{"fill":"close"}}"# },
    Template { method: "spmt_dmi_3", version: 1, spec: r#"{"v":1,"direction":"long","origin":{"book":"spmt","method":"spmt_dmi_3","template_version":1},"entry":{"all":[{"lhs":{"s":"close"},"op":">","rhs":{"s":"sma","p":{"length":200}}},{"lhs":{"s":"dmi","p":{"di_length":2},"out":"plus_di"},"op":"<","rhs":{"value":10}},{"lhs":{"s":"dmi","p":{"di_length":2},"out":"plus_di","offset":1},"op":"<","rhs":{"value":10}},{"lhs":{"s":"rsi","p":{"length":2}},"op":"<=","rhs":{"value":65}}]},"exit":{"any":[{"lhs":{"s":"rsi","p":{"length":2}},"op":">","rhs":{"value":65}}]},"execution":{"fill":"close"}}"# },
    Template { method: "spmt_dmi_4", version: 1, spec: r#"{"v":1,"direction":"long","origin":{"book":"spmt","method":"spmt_dmi_4","template_version":1},"entry":{"all":[{"lhs":{"s":"close"},"op":">","rhs":{"s":"sma","p":{"length":200}}},{"lhs":{"s":"dmi","p":{"di_length":2},"out":"plus_di"},"op":"<","rhs":{"value":10}},{"lhs":{"s":"dmi","p":{"di_length":2},"out":"plus_di","offset":1},"op":"<","rhs":{"value":10}},{"lhs":{"s":"dmi","p":{"di_length":2},"out":"plus_di","offset":2},"op":"<","rhs":{"value":10}},{"lhs":{"s":"rsi","p":{"length":2}},"op":"<=","rhs":{"value":65}}]},"exit":{"any":[{"lhs":{"s":"rsi","p":{"length":2}},"op":">","rhs":{"value":65}}]},"execution":{"fill":"close"}}"# },
    Template { method: "spmt_dmi_5", version: 1, spec: r#"{"v":1,"direction":"long","origin":{"book":"spmt","method":"spmt_dmi_5","template_version":1},"entry":{"all":[{"lhs":{"s":"close"},"op":">","rhs":{"s":"sma","p":{"length":200}}},{"lhs":{"s":"dmi","p":{"di_length":2},"out":"plus_di"},"op":"<","rhs":{"value":15}},{"lhs":{"s":"dmi","p":{"di_length":2},"out":"plus_di","offset":1},"op":"<","rhs":{"value":15}},{"lhs":{"s":"dmi","p":{"di_length":2},"out":"plus_di","offset":2},"op":"<","rhs":{"value":15}},{"lhs":{"s":"dmi","p":{"di_length":2},"out":"plus_di","offset":3},"op":"<","rhs":{"value":15}},{"lhs":{"s":"rsi","p":{"length":2}},"op":"<=","rhs":{"value":65}}]},"exit":{"any":[{"lhs":{"s":"rsi","p":{"length":2}},"op":">","rhs":{"value":65}}]},"execution":{"fill":"close"}}"# },
    Template { method: "spmt_short_4up", version: 1, spec: r#"{"v":1,"direction":"short","origin":{"book":"spmt","method":"spmt_short_4up","template_version":1},"entry":{"all":[{"lhs":{"s":"close"},"op":"<","rhs":{"s":"sma","p":{"length":200}}},{"lhs":{"s":"close"},"op":">","rhs":{"s":"close","offset":1}},{"lhs":{"s":"close","offset":1},"op":">","rhs":{"s":"close","offset":2}},{"lhs":{"s":"close","offset":2},"op":">","rhs":{"s":"close","offset":3}},{"lhs":{"s":"close","offset":3},"op":">","rhs":{"s":"close","offset":4}},{"lhs":{"s":"close"},"op":">=","rhs":{"s":"sma","p":{"length":5}}}]},"exit":{"any":[{"lhs":{"s":"close"},"op":"<","rhs":{"s":"sma","p":{"length":5}}}]},"execution":{"fill":"close"}}"# },
    Template { method: "spmt_short_strong", version: 1, spec: r#"{"v":1,"direction":"short","origin":{"book":"spmt","method":"spmt_short_strong","template_version":1},"entry":{"all":[{"lhs":{"s":"close"},"op":"<","rhs":{"s":"sma","p":{"length":200}}},{"lhs":{"s":"roc","p":{"length":2}},"op":">","rhs":{"value":3}},{"lhs":{"s":"rsi","p":{"length":2}},"op":">","rhs":{"value":80}},{"lhs":{"s":"close"},"op":">=","rhs":{"s":"sma","p":{"length":5}}}]},"exit":{"any":[{"lhs":{"s":"close"},"op":"<","rhs":{"s":"sma","p":{"length":5}}}]},"execution":{"fill":"close"}}"# },
    Template { method: "r25_rsi25_basic_no_filter", version: 1, spec: r#"{"v":1,"direction":"long","origin":{"book":"rsi25","method":"r25_rsi25_basic_no_filter","template_version":1},"entry":{"all":[{"lhs":{"s":"rsi","p":{"length":4}},"op":"<","rhs":{"value":25}}]},"exit":{"any":[{"lhs":{"s":"rsi","p":{"length":4}},"op":">","rhs":{"value":55}}]},"execution":{"fill":"close"}}"# },
    Template { method: "r25_rsi25_official_sma200", version: 1, spec: r#"{"v":1,"direction":"long","origin":{"book":"rsi25","method":"r25_rsi25_official_sma200","template_version":1},"entry":{"all":[{"lhs":{"s":"close"},"op":">","rhs":{"s":"sma","p":{"length":200}}},{"lhs":{"s":"rsi","p":{"length":4}},"op":"<","rhs":{"value":25}}]},"exit":{"any":[{"lhs":{"s":"rsi","p":{"length":4}},"op":">","rhs":{"value":55}}]},"execution":{"fill":"close"}}"# },
```

- [ ] **Step 4: Run to verify pass**

Run: `cd Context/guru/backend && VAULT_IDX="…/idx_data.csv" CARGO_BUILD_JOBS=2 cargo test lab::templates`
Expected: 5 passed (the fixture test now covers 32 templates). Same rule as Task 5 Step 4 if one does not reproduce.

- [ ] **Step 5: Commit**

```bash
git add Context/guru/backend/src/lab/templates.rs
git commit -m "Add S&P market-timing and RSI 25 book templates"
```

---

### Task 7: STSTW, High-Probability ETF and Quantitative Trading templates; completeness

**Files:**
- Modify: `Context/guru/backend/src/lab/templates.rs`

**Interfaces:**
- Produces: the final 43 templates; a test that every admitted method has a template or a named reason.

- [ ] **Step 1: Write the failing tests** — append inside `mod tests`:

```rust
    #[test] fn every_live_method_has_a_template_or_a_named_reason() {
        // A new admitted book method fails here until it gets a template or a NO_TEMPLATE reason.
        let vague: Vec<&str> = connors::catalogue().iter().filter(|d| book_status(d).0 == "live" && unavailable(d).as_deref() == Some(NOT_YET)).map(|d| d.id).collect();
        assert!(vague.is_empty(), "{vague:?}");
    }
    #[test] fn the_last_batch_is_listed_and_two_amibroker_methods_are_named() {
        for m in ["ststw_10d_high_buy", "ststw_10d_high_short", "ststw_10d_low_buy", "ststw_rsi2_under5", "ststw_sp_short", "hpetf_3dhl_long", "hpetf_rsi75_short", "hpetf_r3_long",
                  "hbqts_ma5_25", "hbqts_profit_target", "hbqts_ma1_36"] {
            assert!(template(m).is_some(), "{m}");
        }
        assert_eq!(TEMPLATES.len(), 43);
        assert!(unavailable(&method("hbqts_limit_entry").unwrap()).unwrap().contains("AmiBroker BuyPrice"));
        assert!(unavailable(&method("hbqts_entry_id1").unwrap()).unwrap().contains("AmiBroker BarsSince"));
    }
```

- [ ] **Step 2: Run to verify failure**

Run: `cd Context/guru/backend && CARGO_BUILD_JOBS=2 cargo test lab::templates`
Expected: `the_last_batch_is_listed…` FAILS (`ststw_10d_high_buy`); `every_live_method_has_a_template_or_a_named_reason` FAILS listing the 11 methods of this batch (they still get `NOT_YET`).

- [ ] **Step 3: Implement** — append inside `TEMPLATES`:

```rust
    Template { method: "ststw_10d_high_buy", version: 1, spec: r#"{"v":1,"direction":"long","origin":{"book":"ststw","method":"ststw_10d_high_buy","template_version":1},"entry":{"all":[{"lhs":{"s":"close"},"op":">=","rhs":{"s":"close","offset":1}},{"lhs":{"s":"close"},"op":">=","rhs":{"s":"close","offset":2}},{"lhs":{"s":"close"},"op":">=","rhs":{"s":"close","offset":3}},{"lhs":{"s":"close"},"op":">=","rhs":{"s":"close","offset":4}},{"lhs":{"s":"close"},"op":">=","rhs":{"s":"close","offset":5}},{"lhs":{"s":"close"},"op":">=","rhs":{"s":"close","offset":6}},{"lhs":{"s":"close"},"op":">=","rhs":{"s":"close","offset":7}},{"lhs":{"s":"close"},"op":">=","rhs":{"s":"close","offset":8}},{"lhs":{"s":"close"},"op":">=","rhs":{"s":"close","offset":9}}]},"exit":{"any":[{"lhs":{"s":"close"},"op":"<","rhs":{"s":"sma","p":{"length":10}}}]},"execution":{"fill":"close"}}"# },
    Template { method: "ststw_10d_high_short", version: 1, spec: r#"{"v":1,"direction":"short","origin":{"book":"ststw","method":"ststw_10d_high_short","template_version":1},"entry":{"all":[{"lhs":{"s":"close"},"op":">=","rhs":{"s":"close","offset":1}},{"lhs":{"s":"close"},"op":">=","rhs":{"s":"close","offset":2}},{"lhs":{"s":"close"},"op":">=","rhs":{"s":"close","offset":3}},{"lhs":{"s":"close"},"op":">=","rhs":{"s":"close","offset":4}},{"lhs":{"s":"close"},"op":">=","rhs":{"s":"close","offset":5}},{"lhs":{"s":"close"},"op":">=","rhs":{"s":"close","offset":6}},{"lhs":{"s":"close"},"op":">=","rhs":{"s":"close","offset":7}},{"lhs":{"s":"close"},"op":">=","rhs":{"s":"close","offset":8}},{"lhs":{"s":"close"},"op":">=","rhs":{"s":"close","offset":9}}]},"exit":{"any":[{"lhs":{"s":"close"},"op":"<","rhs":{"s":"sma","p":{"length":10}}}]},"execution":{"fill":"close"}}"# },
    Template { method: "ststw_10d_low_buy", version: 1, spec: r#"{"v":1,"direction":"long","origin":{"book":"ststw","method":"ststw_10d_low_buy","template_version":1},"entry":{"all":[{"lhs":{"s":"close"},"op":"<=","rhs":{"s":"close","offset":1}},{"lhs":{"s":"close"},"op":"<=","rhs":{"s":"close","offset":2}},{"lhs":{"s":"close"},"op":"<=","rhs":{"s":"close","offset":3}},{"lhs":{"s":"close"},"op":"<=","rhs":{"s":"close","offset":4}},{"lhs":{"s":"close"},"op":"<=","rhs":{"s":"close","offset":5}},{"lhs":{"s":"close"},"op":"<=","rhs":{"s":"close","offset":6}},{"lhs":{"s":"close"},"op":"<=","rhs":{"s":"close","offset":7}},{"lhs":{"s":"close"},"op":"<=","rhs":{"s":"close","offset":8}},{"lhs":{"s":"close"},"op":"<=","rhs":{"s":"close","offset":9}}]},"exit":{"any":[{"lhs":{"s":"close"},"op":">","rhs":{"s":"sma","p":{"length":10}}}]},"execution":{"fill":"close"}}"# },
    Template { method: "ststw_rsi2_under5", version: 1, spec: r#"{"v":1,"direction":"long","origin":{"book":"ststw","method":"ststw_rsi2_under5","template_version":1},"entry":{"all":[{"lhs":{"s":"close"},"op":">","rhs":{"s":"sma","p":{"length":200}}},{"lhs":{"s":"rsi","p":{"length":2}},"op":"<","rhs":{"value":5}}]},"exit":{"any":[{"lhs":{"s":"close"},"op":">","rhs":{"s":"sma","p":{"length":5}}}]},"execution":{"fill":"close"}}"# },
    Template { method: "ststw_sp_short", version: 1, spec: r#"{"v":1,"direction":"short","origin":{"book":"ststw","method":"ststw_sp_short","template_version":1},"entry":{"all":[{"lhs":{"s":"close"},"op":"<","rhs":{"s":"sma","p":{"length":200}}},{"lhs":{"s":"close"},"op":">","rhs":{"s":"close","offset":1}},{"lhs":{"s":"close","offset":1},"op":">","rhs":{"s":"close","offset":2}},{"lhs":{"s":"close","offset":2},"op":">","rhs":{"s":"close","offset":3}},{"lhs":{"s":"close","offset":3},"op":">","rhs":{"s":"close","offset":4}}]},"exit":{"any":[{"lhs":{"s":"close"},"op":"<","rhs":{"s":"sma","p":{"length":5}}}]},"execution":{"fill":"close"}}"# },
    Template { method: "hpetf_3dhl_long", version: 1, spec: r#"{"v":1,"direction":"long","origin":{"book":"hpetf","method":"hpetf_3dhl_long","template_version":1},"entry":{"all":[{"lhs":{"s":"close"},"op":">","rhs":{"s":"sma","p":{"length":200}}},{"lhs":{"s":"close"},"op":"<","rhs":{"s":"sma","p":{"length":5}}},{"lhs":{"s":"high"},"op":"<","rhs":{"s":"high","offset":1}},{"lhs":{"s":"low"},"op":"<","rhs":{"s":"low","offset":1}},{"lhs":{"s":"high","offset":1},"op":"<","rhs":{"s":"high","offset":2}},{"lhs":{"s":"low","offset":1},"op":"<","rhs":{"s":"low","offset":2}},{"lhs":{"s":"high","offset":2},"op":"<","rhs":{"s":"high","offset":3}},{"lhs":{"s":"low","offset":2},"op":"<","rhs":{"s":"low","offset":3}}]},"exit":{"any":[{"lhs":{"s":"close"},"op":">","rhs":{"s":"sma","p":{"length":5}}}]},"execution":{"fill":"close"}}"# },
    Template { method: "hpetf_rsi75_short", version: 1, spec: r#"{"v":1,"direction":"short","origin":{"book":"hpetf","method":"hpetf_rsi75_short","template_version":1},"entry":{"all":[{"lhs":{"s":"close"},"op":"<","rhs":{"s":"sma","p":{"length":200}}},{"lhs":{"s":"rsi","p":{"length":4}},"op":">","rhs":{"value":75}}]},"exit":{"any":[{"lhs":{"s":"rsi","p":{"length":4}},"op":"<","rhs":{"value":45}}]},"execution":{"fill":"close"}}"# },
    Template { method: "hpetf_r3_long", version: 1, spec: r#"{"v":1,"direction":"long","origin":{"book":"hpetf","method":"hpetf_r3_long","template_version":1},"entry":{"all":[{"lhs":{"s":"close"},"op":">","rhs":{"s":"sma","p":{"length":200}}},{"lhs":{"s":"rsi","p":{"length":2}},"op":"<","rhs":{"s":"rsi","p":{"length":2},"offset":1}},{"lhs":{"s":"rsi","p":{"length":2},"offset":1},"op":"<","rhs":{"s":"rsi","p":{"length":2},"offset":2}},{"lhs":{"s":"rsi","p":{"length":2},"offset":2},"op":"<","rhs":{"s":"rsi","p":{"length":2},"offset":3}},{"lhs":{"s":"rsi","p":{"length":2},"offset":2},"op":"<","rhs":{"value":60}},{"lhs":{"s":"rsi","p":{"length":2}},"op":"<","rhs":{"value":10}}]},"exit":{"any":[{"lhs":{"s":"rsi","p":{"length":2}},"op":">","rhs":{"value":70}}]},"execution":{"fill":"close"}}"# },
    Template { method: "hbqts_ma5_25", version: 1, spec: r#"{"v":1,"direction":"long","origin":{"book":"hbqts","method":"hbqts_ma5_25","template_version":1},"entry":{"all":[{"lhs":{"s":"sma","p":{"length":5}},"op":"crosses_above","rhs":{"s":"sma","p":{"length":25}}}]},"exit":{"any":[{"lhs":{"s":"sma","p":{"length":5}},"op":"crosses_below","rhs":{"s":"sma","p":{"length":25}}}]},"execution":{"fill":"close"}}"# },
    Template { method: "hbqts_profit_target", version: 1, spec: r#"{"v":1,"direction":"long","origin":{"book":"hbqts","method":"hbqts_profit_target","template_version":1},"entry":{"all":[{"lhs":{"s":"sma","p":{"length":5}},"op":"crosses_above","rhs":{"s":"sma","p":{"length":25}}}]},"exit":{"any":[],"target":{"pct":10}},"execution":{"fill":"close"}}"# },
    Template { method: "hbqts_ma1_36", version: 1, spec: r#"{"v":1,"direction":"long","origin":{"book":"hbqts","method":"hbqts_ma1_36","template_version":1},"entry":{"all":[{"lhs":{"s":"close"},"op":"crosses_above","rhs":{"s":"sma","p":{"length":36}}}]},"exit":{"any":[{"lhs":{"s":"close"},"op":"crosses_below","rhs":{"s":"sma","p":{"length":36}}}]},"execution":{"fill":"close"}}"# },
```

- [ ] **Step 4: Run to verify pass**

Run: `cd Context/guru/backend && VAULT_IDX="…/idx_data.csv" CARGO_BUILD_JOBS=2 cargo test lab::`
Expected: all pass (on `d165472`: 145 lab tests). If the completeness test lists methods added to the catalogue after `d165472`, give each a template or a `NO_TEMPLATE` reason (never `NOT_YET`).

- [ ] **Step 5: Commit**

```bash
git add Context/guru/backend/src/lab/templates.rs
git commit -m "Add STSTW, ETF and quantitative trading templates; require a reason for every admitted method"
```

---

### Task 8: Variant runs — origin, label, instrument rule, trade direction (`lab/api.rs`)

**Files:**
- Modify: `Context/guru/backend/src/lab/api.rs` (imports; `run_json`; `optimize_json`; new private functions before `extra_rank_cols`)
- Test: `Context/guru/backend/src/lab/api.rs` (`mod tests`)

**Interfaces:**
- Consumes: `templates::{template, method, variant_label}` (Task 5), `Spec.origin` (Task 1), `instrument_policy::policy` (read-only).
- Produces: private `origin_of(&Spec) -> Result<Option<StrategyDef>, LabError>`, `origin_admits(Option<&StrategyDef>, kind, ticker) -> Result<(), LabError>`, `method_json(&StrategyDef) -> String`, `labelled(Option<&StrategyDef>) -> (String, String)` (label, origin JSON). Run and optimize JSON: `"label"` = variant label when `origin` is set, `"origin":null|{"method":{id,name,book,group,dir},"template_version":n}` right after `"kind"`; each run trade gains `"dir"` and `"entry_kind"`.

- [ ] **Step 1: Write the failing test** — append inside `mod tests` (before its final `}`):

```rust
    // ---- Customize (spec 2026-10-16)
    fn spec_of_template(m: &str) -> String { templates::template(m).unwrap().spec.to_string() }
    #[test] fn variant_runs_carry_the_variant_label_and_the_methods_instrument_rule() {
        let st = state(700, "SPX", false);
        let j = run_json(&st, &q(&[("spec", &spec_of_template("r25_rsi25_basic_no_filter"))])).unwrap();
        let d = templates::method("r25_rsi25_basic_no_filter").unwrap();
        assert!(j.starts_with(&format!("{{\"label\":\"{}\",\"kind\":\"lab_simulation\",\"origin\":{{\"method\":{{\"id\":\"r25_rsi25_basic_no_filter\"", templates::variant_label(&d))), "{}", &j[..300]);
        assert!(j.contains("\"dir\":1,\"entry_kind\":\"close\""));
        assert!(!j.contains("\"gate\"") && !j.contains("\"claim\"") && !j.contains("\"readiness\""));
        // f1a is a stock method: an index never stands in for a stock
        let e = run_json(&st, &q(&[("spec", &spec_of_template("f1a"))])).unwrap_err();
        assert_eq!(e.status(), "needs_data");
        assert!(e.message().starts_with("SPX (index): variants of stc9") && e.message().contains("individual_stock only"), "{}", e.message());
        let bad = spec_of_template("rsi2_4");
        for (from, to) in [("\"method\":\"rsi2_4\"", "\"method\":\"r3\""), ("\"template_version\":1", "\"template_version\":2"), ("\"book\":\"stc9\"", "\"book\":\"spmt\"")] {
            let e = run_json(&st, &q(&[("spec", &bad.replace(from, to))])).unwrap_err();
            assert!(e.message().starts_with("origin names no book template"), "{}", e.message());
        }
        // optimize keeps the variant label and the instrument rule
        let grid = |m: &str| { let t = spec_of_template(m); format!("{},\"optimize\":[{{\"path\":\"exit.any[0].rhs.value\",\"from\":50,\"to\":60,\"step\":5}}]}}", &t[..t.len() - 1]) };
        let o = optimize_json(&st, &q(&[("spec", &grid("r25_rsi25_basic_no_filter"))])).unwrap();
        assert!(o.starts_with(&format!("{{\"label\":\"{}\",\"kind\":\"lab_optimization\",\"origin\":{{", templates::variant_label(&d))), "{}", &o[..200]);
        assert_eq!(optimize_json(&st, &q(&[("spec", &grid("f1a").replace("exit.any[0].rhs.value\",\"from\":50,\"to\":60", "exit.any[0].rhs.value\",\"from\":3,\"to\":6"))])).unwrap_err().status(), "needs_data");
        // plain lab specs keep the v1 label and a null origin
        assert!(run_json(&st, &q(&[])).unwrap().contains(&format!("\"label\":\"{LABEL}\",\"kind\":\"lab_simulation\",\"origin\":null")));
    }
```

- [ ] **Step 2: Run to verify failure**

Run: `cd Context/guru/backend && CARGO_BUILD_JOBS=2 cargo test lab::api`
Expected: FAIL — the label is the plain lab label and there is no `"origin"`; `f1a` runs on SPX.

- [ ] **Step 3: Implement** — five edits in `lab/api.rs`:

(a) imports — after `use super::simulate::simulate;` add `use super::templates;` (Task 9 widens it to `use super::{fidelity, templates};`) and change `connors::Ctx` in the `use crate::{…}` line to `connors::{Ctx, StrategyDef}`.

(b) in `run_json` and in `optimize_json`: right after `let spec = spec_of(q)?;` (in `optimize_json`: after the `spec.optimize.is_empty()` check) add
```rust
    let origin = origin_of(&spec)?;
```
and right after the `known_type(kind, &ctx.r[0].ticker)?;` line add
```rust
    origin_admits(origin.as_ref(), kind, &ctx.r[0].ticker)?;
    let (label, origin) = labelled(origin.as_ref());
```

(c) in `run_json`'s per-trade `format!`: append `,\"dir\":{},\"entry_kind\":\"{}\"` before the closing `}}` of the trade object and `, t.dir, t.entry_kind` to its arguments.

(d) in the final `format!` of `run_json` and of `optimize_json`: replace the leading `\"label\":\"{LABEL}\",\"kind\":\"lab_simulation\",` (respectively `…\"lab_optimization\",`) with `\"label\":\"{}\",\"kind\":\"lab_simulation\",\"origin\":{origin},` (respectively `lab_optimization`) and insert `esc(&label), ` as the first format argument. (If Phase 5 changed what follows `kind`, keep its fields; only the label and the inserted `origin` change.)

(e) add these functions before `/// RANK_STATS metrics without an is_ PART_COLS column`:

```rust
/// A spec with `origin` is a customized book template: the template must exist (same book, version not newer).
fn origin_of(spec: &Spec) -> Result<Option<StrategyDef>, LabError> {
    let Some(o) = &spec.origin else { return Ok(None) };
    let found = templates::template(&o.method).filter(|t| o.version <= t.version).and(templates::method(&o.method)).filter(|d| d.book == o.book);
    found.map(Some).ok_or_else(|| LabError::Invalid(format!("origin names no book template ({} · {} · version {})", o.book, o.method, o.version)))
}
/// A variant keeps its book method's instrument admission: stock methods on actual stocks, ETF methods on ETF prices.
fn origin_admits(d: Option<&StrategyDef>, kind: &str, ticker: &str) -> Result<(), LabError> {
    let Some(d) = d else { return Ok(()) };
    let allowed = instrument_policy::policy(d).allowed_types;
    if allowed.contains(&kind) { return Ok(()); }
    Err(LabError::NeedsData(format!("{ticker} ({kind}): variants of {} · {} run on {} only (no index or other substitute)", d.book, d.name, allowed.join(", "))))
}
fn method_json(d: &StrategyDef) -> String {
    format!("{{\"id\":\"{}\",\"name\":\"{}\",\"book\":\"{}\",\"group\":\"{}\",\"dir\":\"{}\"}}", esc(d.id), esc(d.name), esc(d.book), esc(d.group), esc(d.dir))
}
/// The run label and the `origin` JSON (null for a plain lab strategy).
fn labelled(d: Option<&StrategyDef>) -> (String, String) {
    match d {
        None => (LABEL.into(), "null".into()),
        Some(d) => (templates::variant_label(d), format!("{{\"method\":{},\"template_version\":{}}}", method_json(d), templates::template(d.id).map_or(0, |t| t.version))),
    }
}
```

- [ ] **Step 4: Run to verify pass**

Run: `cd Context/guru/backend && CARGO_BUILD_JOBS=2 cargo test lab::api`
Expected: all pass, including the unchanged `run_is_labelled_simulated_and_reuses_book_accounting` (`starts_with("{\"label\":\"<LABEL>\",\"kind\":\"lab_simulation\"")` still holds) and Phase 5's provisional tests.

- [ ] **Step 5: Commit**

```bash
git add Context/guru/backend/src/lab/api.rs
git commit -m "Label customized lab runs and keep the book method's instrument admission"
```

---

### Task 9: Template and fidelity routes (`lab/api.rs`)

**Files:**
- Modify: `Context/guru/backend/src/lab/api.rs` (`route` match; new public functions after Task 8's functions)
- Test: `Context/guru/backend/src/lab/api.rs`

**Interfaces:**
- Consumes: Tasks 4, 5, 8; book read-only: `crate::resolve_context`, `crate::readiness::check`, `crate::native_admission`, `crate::window_start`, `crate::connors::run`, `crate::connors::catalogue`.
- Produces (also change Task 8's `use super::templates;` to `use super::{fidelity, templates};`): routes `/api/lab/templates` (`templates_json() -> String`), `/api/lab/template` (`template_json(&State, &Query)`), `/api/lab/fidelity` (`fidelity_json(&State, &Query)`), shapes as spec §6 and §8.

- [ ] **Step 1: Write the failing tests** — append inside `mod tests`:

```rust
    #[test] fn template_routes_list_explain_and_refuse() {
        let st = state(50, "SPX", false);
        let list = route("/api/lab/templates", &st, &q(&[])).unwrap().unwrap();
        let v = crate::lab::json::parse(&list).unwrap();
        let crate::lab::json::Json::Arr(items) = v.get("templates").unwrap() else { panic!() };
        assert_eq!(items.len(), templates::TEMPLATES.len());
        assert!(list.starts_with(&format!("{{\"label\":\"{}\"", templates::TEMPLATE_LABEL)) && !list.contains("\"claim\"") && !list.contains("\"gate\""));
        let one = template_json(&st, &q(&[("method", "rsi2_4")])).unwrap();
        let v = crate::lab::json::parse(&one).unwrap();
        assert_eq!(v.get("status").unwrap().as_str(), Some("available"));
        assert!(one.contains("\"allowed_types\":[") && one.contains("\"notes\":["));
        let crate::lab::json::Json::Obj(_) = v.get("template").unwrap().get("spec").unwrap() else { panic!("spec is embedded JSON") };
        let no = template_json(&st, &q(&[("method", "r3")])).unwrap();
        assert!(no.contains("\"status\":\"unavailable\"") && no.contains("scale-in"), "{no}");
        assert_eq!(template_json(&st, &q(&[("method", "nope")])).unwrap_err().message(), "unknown book method nope");
        assert_eq!(template_json(&st, &q(&[])).unwrap_err().status(), "invalid");
    }
    #[test] fn fidelity_compares_the_template_with_the_book_engine() {
        let st = state(700, "SPX", false);
        let j = fidelity_json(&st, &q(&[("method", "r25_rsi25_basic_no_filter")])).unwrap();
        let v = crate::lab::json::parse(&j).unwrap();
        let n = |k: &str| v.get(k).and_then(|x| x.as_f64()).unwrap();
        assert!(n("book_trades") > 3., "{j}");
        assert_eq!((n("matched"), n("template_trades"), v.get("reproduces"), n("differences_total")), (n("book_trades"), n("book_trades"), Some(&crate::lab::json::Json::Bool(true)), 0.));
        assert!(j.starts_with(&format!("{{\"label\":\"{}\",\"kind\":\"lab_template_fidelity\"", fidelity::FIDELITY_LABEL)) && j.contains("does not verify"));
        assert_eq!(fidelity_json(&st, &q(&[("method", "r3")])).unwrap_err().status(), "unsupported");
        assert_eq!(fidelity_json(&st, &q(&[("method", "rsi2_4"), ("p.rsi_lt", "5")])).unwrap_err().status(), "invalid");
        let e = fidelity_json(&st, &q(&[("method", "f1a")])).unwrap_err();
        assert_eq!(e.status(), "needs_data", "{e:?}");
    }
```

- [ ] **Step 2: Run to verify failure**

Run: `cd Context/guru/backend && CARGO_BUILD_JOBS=2 cargo test lab::api`
Expected: compile errors — `template_json`, `fidelity_json` not found; `route("/api/lab/templates", …)` is `None`.

- [ ] **Step 3: Implement** — (a) in `route`'s `match path`, after the `"/api/lab/optimize" => optimize_json,` arm (and any arm Phase 5 added, e.g. `"/api/lab/instruments"`) add:
```rust
        "/api/lab/templates" => |_, _| Ok(templates_json()),
        "/api/lab/template" => template_json,
        "/api/lab/fidelity" => fidelity_json,
```
(b) add after Task 8's `labelled` function:

```rust
pub fn templates_json() -> String {
    let all = crate::connors::catalogue();
    let live = all.iter().filter(|d| templates::book_status(d).0 == "live").count();
    let items = arr(templates::TEMPLATES.iter().filter_map(|t| templates::method(t.method).map(|d| {
        let p = instrument_policy::policy(&d);
        format!("{{\"method\":{},\"template_version\":{},\"requirement\":\"{}\",\"allowed_types\":{}}}", method_json(&d), t.version, p.requirement, quoted(p.allowed_types.iter().copied()))
    })));
    format!("{{\"label\":\"{}\",\"methods\":{},\"admitted\":{live},\"templates\":{items}}}", templates::TEMPLATE_LABEL, all.len())
}
pub fn template_json(_: &State, q: &Query) -> Result<String, LabError> {
    let id = q.get("method").ok_or_else(|| LabError::Invalid("method is required".into()))?;
    let d = templates::method(id).ok_or_else(|| LabError::Invalid(format!("unknown book method {id}")))?;
    let head = format!("{{\"label\":\"{}\",\"method\":{}", templates::TEMPLATE_LABEL, method_json(&d));
    if let Some(why) = templates::unavailable(&d) { return Ok(format!("{head},\"status\":\"unavailable\",\"reason\":\"{}\"}}", esc(&why))); }
    let t = templates::template(id).expect("available means templated");
    let p = instrument_policy::policy(&d);
    Ok(format!("{head},\"status\":\"available\",\"instrument\":{{\"requirement\":\"{}\",\"allowed_types\":{}}},\"template\":{{\"version\":{},\"spec\":{},\"notes\":{}}}}}",
        p.requirement, quoted(p.allowed_types.iter().copied()), t.version, t.spec, quoted(templates::COMMON_NOTES.iter().copied())))
}
/// Unmodified template vs the book engine on the book's own admitted window and default parameters (spec §6).
pub fn fidelity_json(st: &State, q: &Query) -> Result<String, LabError> {
    let id = q.get("method").ok_or_else(|| LabError::Invalid("method is required".into()))?;
    let d = templates::method(id).ok_or_else(|| LabError::Invalid(format!("unknown book method {id}")))?;
    if let Some(why) = templates::unavailable(&d) { return Err(LabError::Unsupported(format!("Customize not available — {why}"))); }
    if q.keys().any(|k| k.starts_with("p.")) { return Err(LabError::Invalid("the fidelity check uses the book's default parameters; remove p.* settings".into())); }
    let t = templates::template(id).expect("available means templated");
    let selected = select(st, q)?;
    let held = selected.context(st);
    let (ctx, from, to, _) = crate::resolve_context(&held, q)?;
    let none = crate::connors::Params::new();
    if let Some(i) = crate::readiness::check(ctx, &d, from, to, &none).or_else(|| crate::native_admission(ctx, &d, from, to, &selected, st, q)) {
        return Err(LabError::NeedsData(format!("the book method is not admitted on this instrument and window ({}: {}); no comparison is shown", i.status, i.note)));
    }
    let from = crate::window_start(ctx, &d, from).filter(|&f| f < to).ok_or_else(|| LabError::NeedsData("required volatility-index data does not cover this period".into()))?;
    let book = crate::connors::run(id, ctx, from, to, &none).ok_or_else(|| LabError::Invalid("book implementation unavailable".into()))?;
    let fr = Frames::new(Bars::from_rows(ctx.r));
    let lab = simulate(&series::compile(&templates::parsed(t), &fr, &mut Cache::new())?, &fr.d, from, to);
    let c = fidelity::compare(&book, &lab);
    let (_, instrument) = instrument(&selected, &ctx.r[0].ticker);
    let tj = |t: Option<&crate::connors::Trade>| t.map_or("null".into(), |t| format!(
        "{{\"entry\":\"{}\",\"exit\":\"{}\",\"entry_px\":{},\"exit_px\":{},\"dir\":{},\"legs\":{},\"open\":{}}}",
        ctx.r[t.legs[0].i].date, ctx.r[t.exit_i].date, crate::num(t.legs[0].px), crate::num(t.exit_px), t.dir, t.legs.len(), t.open));
    let diffs = arr(c.diffs.iter().take(fidelity::MAX_DIFFS).map(|(b, l)| format!("{{\"book\":{},\"template\":{}}}", tj(*b), tj(*l))));
    Ok(format!(
        "{{\"label\":\"{}\",\"kind\":\"lab_template_fidelity\",\"method\":{},\"template_version\":{},\"instrument\":{instrument},\"from\":\"{}\",\"to\":\"{}\",\"book_trades\":{},\"template_trades\":{},\"matched\":{},\"reproduces\":{},\"differences_total\":{},\"differences\":{diffs},\"note\":\"{}\"}}",
        fidelity::FIDELITY_LABEL, method_json(&d), t.version, ctx.r[from].date, ctx.r[to].date, book.len(), lab.len(), c.matched,
        c.matched == book.len() && c.matched == lab.len(), c.diffs.len(), fidelity::FIDELITY_NOTE))
}
```

- [ ] **Step 4: Run to verify pass**

Run: `cd Context/guru/backend && VAULT_IDX="…/idx_data.csv" CARGO_BUILD_JOBS=2 cargo test lab::`
Expected: all pass (on `d165472`: 148 lab tests). On `engine-gl` (Phase 5 `response()` wrapper) these routes are wrapped with the provisional prefix like every lab route; no extra code.

- [ ] **Step 5: Commit**

```bash
git add Context/guru/backend/src/lab/api.rs
git commit -m "Add lab template, template list and fidelity routes"
```

---

### Task 10: Gateway allowlist for the new routes

**Files:**
- Modify: `Context/guru/webapp/gateway/src/policy.rs` (additive only)

**Interfaces:**
- Produces: `allowed_engine_path("lab/templates" | "lab/template" | "lab/fidelity") == true`; not optimization paths; no spec bound applies (no `spec` parameter). Phase 5's `engine_market_allows("gl", …)` already admits every `lab/` path.

- [ ] **Step 1: Write the failing test** — inside `mod lab_tests` in `policy.rs`, add before `fn lab_specs_are_bounded_before_the_engine`:

```rust
    #[test]
    fn customize_routes_are_allowlisted_reads() {
        for route in ["templates", "template", "fidelity"] {
            let path = format!("lab/{route}");
            assert!(allowed_engine_path(&path));
            assert!(!is_lab_optimization_path(&path) && !is_optimization_path(&path));
            assert_eq!(lab_spec_problem(&path, &BTreeMap::new()), None, "no spec is sent: the engine uses its own template");
        }
        assert!(!allowed_engine_path("lab/templates/x"));
    }
```

- [ ] **Step 2: Run to verify failure**

Run: `cd Context/guru/webapp && CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="${VAULT_RUNTIME_ROOT:-$HOME/.cache/strategy-vault-webapp}/test-target" cargo test --locked --manifest-path gateway/Cargo.toml customize`
Expected: FAIL at `allowed_engine_path(&path)`.

- [ ] **Step 3: Implement** — in `allowed_engine_path`'s `matches!`, after the last `| "lab/…"` entry (Phase 5 adds `"lab/instruments"`), add three lines:

```rust
            | "lab/templates"
            | "lab/template"
            | "lab/fidelity"
```

- [ ] **Step 4: Run to verify pass**

Run: the Step 2 command without the filter → all gateway tests pass (6 → 7 on `d165472`, plus Phase 5's).
Check additive-only: `git diff -U0 -- Context/guru/webapp/gateway/src/policy.rs | grep '^-[^-]'` → no output.

- [ ] **Step 5: Commit**

```bash
git add Context/guru/webapp/gateway/src/policy.rs
git commit -m "Allow the lab template and fidelity routes through the gateway"
```

---

### Task 11: Origin shape in saved lab strategies (`contracts.mjs`)

**Files:**
- Modify: `Context/guru/webapp/auth/src/contracts.mjs` (inside `validateLabStrategy`; two new exports after it)
- Test: `Context/guru/webapp/tests/contracts.test.mjs` (append)

**Interfaces:**
- Produces: `validateLabOrigin(origin) -> {book, method, template_version} | null` (throws `LabRequestError` 400 "Rule spec origin must be {book, method, template_version} from a book template."); `sameLabOrigin(a, b) -> boolean` (null/undefined equal; field-wise, key order irrelevant); `validateLabStrategy(body)` returns `{name, spec, market, instrument}` plus `origin` only when `spec.origin` is present (existing callers and tests unchanged).

- [ ] **Step 1: Write the failing test** — append to `tests/contracts.test.mjs` (it reuses `LAB_SPEC` defined earlier in that file):

```js
const LAB_ORIGIN={book:'stc9',method:'rsi2_4',template_version:1};
test('customized lab specs carry a book-template origin of a fixed shape',async()=>{
 const {validateLabStrategy,validateLabOrigin,sameLabOrigin}=await import('../auth/src/contracts.mjs');
 const spec={...LAB_SPEC,origin:LAB_ORIGIN};
 assert.deepEqual(validateLabStrategy({name:'My RSI(2)',spec,market:'us',instrument:'SPY'}),{name:'My RSI(2)',spec,market:'us',instrument:'SPY',origin:LAB_ORIGIN});
 assert.equal(validateLabOrigin(undefined),null);
 for(const origin of [null,[],'stc9',{book:'stc9',method:'rsi2_4'},{...LAB_ORIGIN,extra:1},{...LAB_ORIGIN,method:'rsi 2'},{...LAB_ORIGIN,book:''},{...LAB_ORIGIN,template_version:0},{...LAB_ORIGIN,template_version:1.5},{...LAB_ORIGIN,template_version:'1'}])
  assert.throws(()=>validateLabStrategy({name:'A',spec:{...LAB_SPEC,origin},market:'us',instrument:'SPY'}),/origin must be/);
 assert.ok(sameLabOrigin(null,undefined)&&sameLabOrigin(LAB_ORIGIN,{template_version:1,method:'rsi2_4',book:'stc9'}));
 assert.ok(!sameLabOrigin(LAB_ORIGIN,null)&&!sameLabOrigin(LAB_ORIGIN,{...LAB_ORIGIN,template_version:2}));
});
```

- [ ] **Step 2: Run to verify failure**

Run: `cd Context/guru/webapp && node --test tests/contracts.test.mjs`
Expected: the new test fails (`validateLabOrigin` is not exported / no `origin` in the result).

- [ ] **Step 3: Implement** — in `validateLabStrategy`, before the `if (!LAB_MARKETS.has(body.market))` line add
```js
  const origin = validateLabOrigin(spec.origin);
```
replace its `return { name, spec, market: body.market, instrument };` with
```js
  return { name, spec, market: body.market, instrument, ...(origin ? { origin } : {}) };
```
and add right after the function:
```js

// Customize (spec 2026-10-16 §7): spec.origin names the book template a variant came from. Shape only here;
// the engine checks that the template exists and keeps the book method's instrument rule.
const ORIGIN_ID = /^[A-Za-z0-9_]{1,80}$/;
export function validateLabOrigin(origin) {
  if (origin === undefined) return null;
  const ok = origin && typeof origin === 'object' && !Array.isArray(origin) && Object.keys(origin).sort().join() === 'book,method,template_version'
    && ORIGIN_ID.test(origin.book) && ORIGIN_ID.test(origin.method) && Number.isSafeInteger(origin.template_version) && origin.template_version >= 1 && origin.template_version <= 1000;
  if (!ok) throw new LabRequestError('Rule spec origin must be {book, method, template_version} from a book template.');
  return { book: origin.book, method: origin.method, template_version: origin.template_version };
}
export function sameLabOrigin(a, b) {
  return (a?.book ?? null) === (b?.book ?? null) && (a?.method ?? null) === (b?.method ?? null) && (a?.template_version ?? null) === (b?.template_version ?? null);
}
```

- [ ] **Step 4: Run to verify pass**

Run: `cd Context/guru/webapp && node --test tests/contracts.test.mjs tests/lab-strategies.test.mjs tests/book-library.test.mjs`
Expected: `# fail 0`.

- [ ] **Step 5: Commit**

```bash
git add Context/guru/webapp/auth/src/contracts.mjs Context/guru/webapp/tests/contracts.test.mjs
git commit -m "Validate the book-template origin of saved lab strategies"
```

---

### Task 12: Store and keep the origin — migration 003 and routes

**Files:**
- Create: `Context/guru/webapp/db/003-lab-origin.sql`
- Modify: `Context/guru/webapp/auth/src/bootstrap.mjs` (+1 line after the `002-lab-strategies.sql` line)
- Modify: `Context/guru/webapp/auth/src/lab-strategies.mjs` (import, `COLUMNS`, `insert`, published list, PUT)
- Test: `Context/guru/webapp/tests/contracts.test.mjs`, `Context/guru/webapp/tests/lab-strategies.test.mjs`

**Interfaces:**
- Consumes: `sameLabOrigin`, `validateLabStrategy(...).origin` (Task 11).
- Produces: column `lab_strategy.origin jsonb NULL`; every strategy JSON has `origin` (object or null); `GET …/published` rows have `origin` (from `spec->'origin'`); PUT whose `spec.origin` differs from the row's origin → `400 {"error":"A strategy keeps the book template it was customized from (spec.origin); start a new customization instead."}`, nothing written; duplicate copies `origin`.

- [ ] **Step 1: Write the failing tests** — (a) append to `tests/contracts.test.mjs`:

```js
test('origin migration only adds a nullable column and runs after 002',async()=>{
 const {readFile}=await import('node:fs/promises');
 const sql=await readFile(new URL('../db/003-lab-origin.sql',import.meta.url),'utf8');
 const statements=sql.split('\n').filter(l=>l.trim()&&!l.trim().startsWith('--')).join(' ').split(';').map(s=>s.trim()).filter(Boolean);
 assert.equal(statements.length,1);
 assert.match(statements[0],/^ALTER TABLE lab_strategy ADD COLUMN IF NOT EXISTS origin jsonb\s+CHECK \(origin IS NULL OR /);
 assert.doesNotMatch(sql,/\b(DROP|DELETE|UPDATE|TRUNCATE|INSERT|NOT NULL DEFAULT)\b/i);
 const bootstrap=await readFile(new URL('../auth/src/bootstrap.mjs',import.meta.url),'utf8');
 assert.ok(bootstrap.indexOf('003-lab-origin.sql')>bootstrap.indexOf('002-lab-strategies.sql')&&bootstrap.indexOf('002-lab-strategies.sql')>0);
});
```

(b) in `tests/lab-strategies.test.mjs`, let the recording pool also keep query parameters and answer `RETURNING id` (so a create can succeed in-process): in `harness`, change `const log = [], sqls = [];` to `const log = [], sqls = [], args = [];`, add `args.push(params);` after `sqls.push(sql);`, add `if (/RETURNING id/.test(sql)) return { rows: [{ id: 9 }], rowCount: 1 };` as the first line after those pushes, and return `{ log, sqls, args, call }`. Then append:

```js
const ORIGIN = { book: 'stc9', method: 'rsi2_4', template_version: 1 };
test('a customized strategy stores its origin, and an edit cannot change it', async () => {
  const created = harness();
  assert.equal((await created.call('owner', 'POST', '', { name: 'My RSI(2)', spec: { ...SPEC, origin: ORIGIN }, market: 'us', instrument: 'SPY' })).status, 201);
  const insert = created.sqls.findIndex(sql => /INSERT INTO lab_strategy \(owner,name,spec,market,instrument,origin\)/.test(sql));
  assert.equal(created.args[insert][5], JSON.stringify(ORIGIN));
  const plain = harness();
  await plain.call('owner', 'POST', '', { name: 'Mine', spec: SPEC, market: 'us', instrument: 'SPY' });
  assert.equal(plain.args[plain.sqls.findIndex(sql => /INSERT INTO lab_strategy \(/.test(sql))][5], null);
  const variant = harness({ 7: row({ spec: { ...SPEC, origin: ORIGIN }, origin: ORIGIN }) });
  for (const spec of [SPEC, { ...SPEC, origin: { ...ORIGIN, method: 'rsi4_25' } }]) {
    const response = await variant.call('owner', 'PUT', '/7', { version: 3, name: 'B', spec, market: 'us', instrument: 'SPY' });
    assert.equal(response.status, 400);
    assert.match((await response.json()).error, /keeps the book template/);
  }
  assert.ok(!variant.log.includes('UPDATE'));
  assert.equal((await variant.call('owner', 'PUT', '/7', { version: 3, name: 'B', spec: { ...SPEC, origin: ORIGIN, exit: { any: [], stop_loss: { pct: 5 } } }, market: 'us', instrument: 'SPY' })).status, 200);
  assert.ok(variant.log.includes('UPDATE'));
  const fresh = harness({ 7: row() });
  assert.equal((await fresh.call('owner', 'PUT', '/7', { version: 3, name: 'B', spec: { ...SPEC, origin: ORIGIN }, market: 'us', instrument: 'SPY' })).status, 400, 'origin is set at creation only');
});

test('duplicating a variant keeps its origin and YT Strategies lists the origin of each snapshot', async () => {
  const { sqls, args, call } = harness({ 7: row({ status: 'published', spec: { ...SPEC, origin: ORIGIN }, origin: ORIGIN }) });
  assert.equal((await call('other', 'POST', '/7/duplicate')).status, 201);
  assert.equal(args[sqls.findIndex(sql => /INSERT INTO lab_strategy \(/.test(sql))][5], JSON.stringify(ORIGIN));
  await call('viewer', 'GET', '/published');
  assert.ok(sqls.some(sql => /v\.spec->'origin' AS origin/.test(sql)));
});
```

- [ ] **Step 2: Run to verify failure**

Run: `cd Context/guru/webapp && node --test tests/contracts.test.mjs tests/lab-strategies.test.mjs`
Expected: the migration test fails (`ENOENT … 003-lab-origin.sql`); the two route tests fail (the INSERT has no `origin` column; the PUT is accepted). The existing route tests still pass with the extended harness.

- [ ] **Step 3: Implement** — (a) create `db/003-lab-origin.sql`:

```sql
-- Customize book strategies (spec 2026-10-16 §7): lineage of a lab strategy created from a book template.
-- Additive and idempotent (bootstrap runs it on every start). NULL = written from scratch in the lab.
-- Set once at creation from spec.origin; the app never changes it (an edit that changes spec.origin is refused).
ALTER TABLE lab_strategy ADD COLUMN IF NOT EXISTS origin jsonb
    CHECK (origin IS NULL OR (jsonb_typeof(origin) = 'object' AND origin ? 'book' AND origin ? 'method' AND origin ? 'template_version'));
```

(b) `bootstrap.mjs`: after the line that applies `002-lab-strategies.sql` add the same line for `003-lab-origin.sql`:
```js
  await pool.query(await readFile(new URL('../../db/003-lab-origin.sql', import.meta.url), 'utf8').catch(() => readFile(new URL('../../../db/003-lab-origin.sql', import.meta.url), 'utf8')));
```
(`infra/auth.Dockerfile` already copies `webapp/db` into the image.)

(c) `lab-strategies.mjs`:
- import `sameLabOrigin` from `./contracts.mjs` (add it to the existing import list);
- `COLUMNS`: `s.name, s.spec, s.market,` → `s.name, s.spec, s.origin, s.market,`;
- `insert`: the INSERT becomes
```js
    const id = (await client.query('INSERT INTO lab_strategy (owner,name,spec,market,instrument,origin) VALUES ($1,$2,$3::jsonb,$4,$5,$6::jsonb) RETURNING id', [user.id, data.name, JSON.stringify(data.spec), data.market, data.instrument, data.origin ? JSON.stringify(data.origin) : null])).rows[0].id;
```
  (duplicate passes the locked row, whose `origin` comes from the new column);
- published list: `v.name, v.spec, v.market,` → `v.name, v.spec, v.spec->'origin' AS origin, v.market,`;
- PUT: after `labAction(row, user, 'update', version);` add
```js
      if (!sameLabOrigin(row.origin, data.origin)) throw new LabRequestError('A strategy keeps the book template it was customized from (spec.origin); start a new customization instead.');
```

- [ ] **Step 4: Run to verify pass**

Run: `cd Context/guru/webapp && node --test tests/contracts.test.mjs tests/lab-strategies.test.mjs tests/book-library.test.mjs && npm --prefix auth test`
Expected: `# fail 0` (39 → 43 in-process tests on `d165472`).

- [ ] **Step 5: Commit**

```bash
git add Context/guru/webapp/db/003-lab-origin.sql Context/guru/webapp/auth/src/bootstrap.mjs Context/guru/webapp/auth/src/lab-strategies.mjs Context/guru/webapp/tests/contracts.test.mjs Context/guru/webapp/tests/lab-strategies.test.mjs
git commit -m "Store the template origin of lab strategies and keep it through edits and copies"
```

---

### Task 13: Isolated-stack subtest for variants (manual gate)

**Files:**
- Modify: `Context/guru/webapp/tests/lab-strategies.integration.test.mjs` (one subtest added before `two simultaneous saves of one version…`)

**Interfaces:**
- Consumes: the routes of Task 12 on a running isolated stack. Skips unless `VAULT_ENV_FILE` and `VAULT_COMPOSE_PROJECT` are set (existing guard; the live project is refused).

- [ ] **Step 1: Add the subtest:**

```js
    await t.test('a customized variant keeps its origin through edit, copy and publish', async () => {
      const ORIGIN = { book: 'stc9', method: 'rsi2_4', template_version: 1 };
      const spec = { ...SPEC, origin: ORIGIN };
      const variant = (await call(S, { cookie: owner, method: 'POST', body: { name: 'My RSI(2)', spec, market: 'us', instrument: 'SPY' } })).json();
      assert.deepEqual(variant.origin, ORIGIN);
      const dropped = await call(`${S}/${variant.id}`, { cookie: owner, method: 'PUT', body: { version: 1, name: 'x', spec: SPEC, market: 'us', instrument: 'SPY' } });
      assert.equal(dropped.status, 400); assert.match(dropped.json().error, /keeps the book template/);
      const edited = (await call(`${S}/${variant.id}`, { cookie: owner, method: 'PUT', body: { version: 1, name: 'My RSI(2) + stop', spec: { ...spec, exit: { ...SPEC.exit, stop_loss: { pct: 5 } } }, market: 'us', instrument: 'SPY' } })).json();
      assert.deepEqual([edited.version, edited.origin], [2, ORIGIN]);
      assert.deepEqual((await call(`${S}/${variant.id}/duplicate`, { cookie: owner, method: 'POST' })).json().origin, ORIGIN);
      assert.equal((await call(`${S}/${variant.id}/submit`, { cookie: owner, method: 'POST', body: { version: 2 } })).status, 200);
      assert.equal((await call(`${S}/${variant.id}/publish`, { cookie: admin, method: 'POST', body: { version: 2 } })).status, 200);
      const snapshot = (await call(`${S}/published`, { cookie: viewer })).json().find(s => s.id === variant.id);
      assert.deepEqual(snapshot.origin, ORIGIN);
    });
```

- [ ] **Step 2: Check it skips cleanly without a stack**

Run: `cd Context/guru/webapp && node --test tests/lab-strategies.integration.test.mjs`
Expected: `# skipped 1`, `# fail 0`.

- [ ] **Step 3: Commit**

```bash
git add Context/guru/webapp/tests/lab-strategies.integration.test.mjs
git commit -m "Cover customized variants in the isolated-stack lab strategy test"
```

---

### Task 14: Phase gate

- [ ] **Step 1: Engine suite** — `cd Context/guru/backend && VAULT_IDX="/run/user/1000/gvfs/smb-share:server=192.168.4.50,share=share/Manan Pujara/Book-to-rust/idx_data.csv" CARGO_BUILD_JOBS=2 cargo test` → all pass; the count is the "Before Task 1" count **+29** (690 → 719 on `d165472`: rules 3, simulate 15, fidelity 1, templates 7, api 3). Record both counts.
- [ ] **Step 2: Release build without lab warnings** — `cd Context/guru/backend && touch src/main.rs && CARGO_BUILD_JOBS=2 cargo build --release 2>&1 | grep -A3 '^warning' | grep -- '--> src/lab/'` → no output.
- [ ] **Step 3: Book guard with engine compare** — `NAS="/run/user/1000/gvfs/smb-share:server=192.168.4.50,share=share/Manan Pujara/Book-to-rust"; python3 scripts/check_book_untouched.py --base origin/Manan --engine-compare --csv "$NAS/idx_data.csv" --vix "$NAS/india_vix" --extra "$NAS/extra_data"` → `"problems": []`; copy its `notes` into the report. If `VAULT_BASKET_DATA_DIR` is available, run once more with it set.
- [ ] **Step 4: Gateway and auth** — `cd Context/guru/webapp && CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="${VAULT_RUNTIME_ROOT:-$HOME/.cache/strategy-vault-webapp}/test-target" cargo test --locked --manifest-path gateway/Cargo.toml` → pass; `node --test tests/contracts.test.mjs tests/lab-strategies.test.mjs tests/book-library.test.mjs` → `# fail 0`; `npm --prefix auth test` → pass; from the repo root `python3 -m unittest discover -s scripts -p 'test_*.py'` → pass.
- [ ] **Step 5: Scope proof** — from the repo root, with `BASE` from "Before Task 1":
  - `git diff --stat $BASE` lists only the files in "File structure" (plus this plan if it was edited);
  - `git diff --quiet $BASE -- Context/guru/backend/src/main.rs Context/guru/backend/src/connors.rs Context/guru/backend/src/books Context/guru/backend/src/readiness.rs Context/guru/backend/src/gate.rs Context/guru/backend/src/instrument_policy.rs Context/guru/backend/src/baskets.rs Context/guru/backend/src/accounting.rs Context/guru/backend/src/trade_validation.rs Context/guru/frontend` → exit 0;
  - `git diff -U0 $BASE -- Context/guru/webapp/gateway/src/policy.rs Context/guru/webapp/auth/src/bootstrap.mjs | grep '^-[^-]'` → no output;
  - `git diff --name-only $BASE | grep -E 'package(-lock)?\.json|Cargo\.(toml|lock)|compose|Dockerfile|001-app\.sql|002-lab-strategies\.sql'` → no output.
- [ ] **Step 6: Upload check** — `python3 scripts/check_upload.py` → `"findings": []`.
- [ ] **Step 7: Real-data fidelity spot check (optional, read-only)** — build the engine and run it on the NAS index file on a free local port (never the live stack): `cd Context/guru/backend && CARGO_BUILD_JOBS=2 cargo build && ./target/debug/guru "$NAS/idx_data.csv" 18978 "$NAS/india_vix" "$NAS/extra_data" &`, then `curl -s 'localhost:18978/api/lab/fidelity?method=rsi2_4&ticker=NIFTY%2050'` → `"reproduces":true` (96/96 at planning time) and `curl -s 'localhost:18978/api/lab/fidelity?method=f1a&ticker=NIFTY%2050'` → `needs_data` (stock method on an index). Stop the process afterwards (`kill %1`).
- [ ] **Step 8: Manual gate item for the user — isolated stack.** Not run by an agent. On the user's own dev stack (separate runtime dir, port, image tags and Compose project `strategy-vault-dev-<name>`, as in `docs/DEVELOPMENT.md`; never `strategy-vault`):
  ```bash
  docker compose -p strategy-vault-dev-<name> -f Context/guru/webapp/compose.yaml --env-file "$HOME/.cache/strategy-vault-dev-<name>/.env" up -d --build postgres auth engine-in engine-us gateway web
  cd Context/guru/webapp
  VAULT_ENV_FILE="$HOME/.cache/strategy-vault-dev-<name>/.env" VAULT_COMPOSE_PROJECT=strategy-vault-dev-<name> node --test tests/lab-strategies.integration.test.mjs
  ```
  Expected: all subtests pass, including `a customized variant keeps its origin…`; after `docker compose -p strategy-vault-dev-<name> … restart auth` the logs show `Application schema ready` (003 re-runs cleanly); `… exec -T postgres psql -U vault -d vault -c '\d lab_strategy'` shows `origin | jsonb`; through the dev gateway, signed in: `/api/us/lab/templates` lists 43 templates and `/api/in/lab/fidelity?method=rsi2_4&ticker=NIFTY%2050` answers `"reproduces":true`. Record the result in the PR description.
- [ ] **Step 9: Report** — commit any gate fixes, then draft the PR description (Nandini → Manan): test counts before/after, guard notes, the template count (43 of 164 admitted; 4,365 methods), dropped templates and why, spec §12 assumptions, the manual gate result or "pending user run". Do not push; the user pushes.
