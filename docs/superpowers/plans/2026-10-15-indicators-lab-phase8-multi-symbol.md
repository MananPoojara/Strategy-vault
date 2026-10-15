# Indicators Lab — Phase 8 (Multi-symbol and portfolio rules) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let lab rules read other instruments of the same market (AmiBroker `Foreign`/`SetForeign`, market filters, pair indicators, higher timeframes across symbols), execute signals of one instrument on another, and run portfolio backtests over a basket or list with ranking, rotation, shared money-mode sizing and declared survivorship bias — without touching any book code or result.

**Architecture:** All engine work stays in `Context/guru/backend/src/lab/` (already dispatched by `main.rs` through the `/api/lab/` prefix; `lab/run` and `lab/optimize` are already allowlisted by the gateway). New files: `foreign.rs` (date alignment and session rule), `pairs.rs` (the four second-instrument indicators), `universe.rs` (basket/list members on the reference calendar), `portfolio.rs` (the shared-ledger simulator). Changed: `rules.rs` (operand fields `symbol`/`vs`/`fixup`, the `portfolio` object), `series.rs` (per-symbol lookup in operand resolution), `bars.rs` (`second`), `catalogue.rs`/`listed.rs` (pair entries become computed), `simulate.rs` (visibility only), `api.rs` (foreign loading, portfolio run/optimize). The auth service refuses to publish portfolio specs. `main.rs`, the gateway and every book file are untouched.

**Tech Stack:** Rust 2021 std-only engine (`cargo test`); Node 22 ESM auth service (`node --test`, existing lockfile).

**Spec:** `docs/superpowers/specs/2026-10-15-indicators-lab-phase8-multi-symbol-design.md` (read all of it, especially §4 alignment, §6 portfolio, §14 assumptions). Phase 7 spec (executed **before** this plan): `docs/superpowers/specs/2026-10-15-indicators-lab-phase7-rules-ii-design.md` and its plan `docs/superpowers/plans/2026-10-15-indicators-lab-phase7-rules-ii.md`. Format follows `docs/superpowers/plans/2026-10-16-customize-book-strategies.md`.

**Planning evidence and its limits (read before Task 1):**
- Code blocks were written against `Nandini` at `8136de0` (Phase 6 code + the Phase 7 spec; **Phase 7 code not yet written**). Phase 7 runs first and reshapes `rules.rs` (node tree, `symbol` reserved as `unsupported`, money mode `execution.capital/sizing/costs`, Q/Y/`nD`, `expand`), `series.rs` (node evaluator, `Frames` with more timeframes), `simulate.rs` (orders, intrabar modes) and adds `lab/ledger.rs`. Every edit below is therefore anchored on a **function or type name** and states the Phase 7 equivalent to use. If a quoted context line differs, apply the same change to the equivalent Phase 7 line; never change book files to make a patch apply.
- Task 1's code was compiled and its rules tests run once in a scratch copy of `8136de0`: 12 of 13 `lab::rules` tests passed; the failing case (a `bars_held` operand with `symbol` placed in `entry`, where "only available in exit conditions" fires first) is corrected below and was **not** re-run. Tasks 2–15 were **not compiled**: the shared scratch build directory was in use by another planner (see the report), and creating a new build directory under `/tmp` was not allowed. Expect small compile fixes; the tests define the behaviour.
- Data for the optional real-data checks lives on the live server (`VAULT_BASKET_DATA_ROOT`, `VAULT_US_BASKET_DATA_ROOT`); this workstation has only the NAS index file, VIX and extra data. Every required test uses synthetic data.

## Global Constraints

- Branch `Nandini`; never `main`; never push (the user pushes); no force-push; no git worktrees; commits without any Co-Authored-By trailer; stage only the files a task names (`git add <paths>`, never `git add -A`); other agents may commit concurrently.
- Book results untouched. Protected, no edits: `Context/guru/backend/src/connors.rs`, `backend/src/books/**`, `readiness.rs`, `gate.rs`, `instrument_policy.rs`, `baskets.rs`, `accounting.rs`, `trade_validation.rs`, frontend `src/compat/*`, `src/pages/ResearchPages.tsx`. `backend/src/main.rs`, `webapp/gateway/src/policy.rs` and `frontend/react/src/app/custom-pages.ts` are additive-only and **not touched at all in this phase**.
- Every existing spec keeps its meaning: Phase 7's pinned-output hash test, the 43 Customize templates' fidelity test and every existing lab test stay green after every task (only the listed mechanical test edits are allowed).
- Every lab body keeps `"label":"User-defined indicator strategy · simulated · not a book method"`; error bodies keep `{status, error, label}` with `invalid | unsupported | needs_data`.
- Limits (spec §6.10): `MAX_FOREIGN = 4` other instruments per spec; `MAX_SYMBOL_LEN = 80`; `MAX_UNIVERSE = 120`; `MAX_UNIVERSE_LIST = 50`; `MAX_OPEN = 50`; portfolio run 90 s; portfolio optimize ≤ 200 combinations, members × combinations ≤ 12,000, 300 s; split refused for portfolios.
- Alignment (spec §4.3): `same_date` (NaN on a missing date) for same-session pairs; `prior_date` for CBOE VIX when any order fills at the signal close, and for gl pairs whose timezone or calendar differs; `fixup: previous` fills only missing dates. No value is ever read from a date after the run bar.
- Portfolios: long only, money mode only (`execution.capital` required), members are `individual_stock` or `etf` only, reference calendar NIFTY 50 (`engine-in`) / SPX (`engine-us`), gl → `unsupported`; survivorship `membership` object on every portfolio body; portfolio specs never publishable.
- std-only Rust; no new crates; auth keeps its lockfile. `CARGO_BUILD_JOBS=2`. Engine suites need `VAULT_IDX="/run/user/1000/gvfs/smb-share:server=192.168.4.50,share=share/Manan Pujara/Book-to-rust/idx_data.csv"` (read-only). The NAS and basket roots are read-only; no data, `.env` or results in Git; no whole-corpus backtest; no backtest at startup; never run `docker`.

## Review Focus

1. **A foreign value from a later-closing instrument used at the run bar's close** (CBOE VIX with `execution.fill: close`, or a crypto/DAX value on an HSI run): expected `prior_date`, never the same-date value. Test: `alignment_rules_follow_session_and_fill_mode` (Task 2) and `cboe_vix_lags_one_bar_with_close_fills` (Task 5).
2. **`fixup: previous` filling an indicator's own warm-up or sparse NaN** (e.g. fractals between events) instead of only missing dates: expected the NaN stays. Test: `fixup_fills_missing_dates_not_indicator_gaps` (Task 4).
3. **A delisted / data-ended member holding cash hostage or being marked at an invented price**: expected exit at its last close with reason `data_end`, then the cash is free for others. Test: `data_end_closes_at_the_last_close_and_frees_cash` (Task 11).
4. **Two members tied on score with different sizing, or the universe listed in another order**: expected the AmiBroker KB order (size, then ticker) and byte-identical output. Tests: `ties_prefer_larger_size_then_alphabetical_order` (Task 9) and `universe_order_never_changes_the_result` (Task 14).
5. **A basket above 120 members, an index or gl id in a universe, or a gl symbol in an India/US spec**: expected a named refusal before any member file is read. Tests: `universes_are_bounded_and_typed` (Task 7) and `foreign_symbols_stay_in_one_market` (Task 5).

---

## Existing code this phase relies on (read-only unless a task modifies it)

| Item | Where (HEAD `8136de0`) | Used for |
|---|---|---|
| `rules::{parse_spec, Spec, SeriesRef, Operand, Cond, Exec, Fill, Dist, Target, dims, target, set, current, apply, series_ref, fields, LabError, LABEL}` | `lab/rules.rs` | grammar (Tasks 1, 3, 6); Phase 7 replaces `Cond` lists by nodes and reserves `symbol` |
| `series::{Frames, Cache, series, compile, describe, RCond, Val, Compiled}`; cache key = id, tf, params + instrument fingerprint | `lab/series.rs` | per-symbol lookup (Task 4) |
| `simulate::simulate`, private `Pos { open, trade, price_exit }` | `lab/simulate.rs` | per-position price exits in the portfolio (Task 11) |
| `resample::{resample, expand, period_key, Tf}` | `lab/resample.rs` | foreign timeframes, rebalance periods |
| `catalogue::{all, get, run, resolve_params, IndicatorDef, Needs, ParamSpec, int}`; `listed::DEFS` | `lab/catalogue.rs`, `lab/listed.rs` | pair indicators (Task 3) |
| `markets::{gl, GlInstrument{timezone, calendar, kind}, engine_market}` | `lab/markets.rs` | gl session rule, gl refusals |
| `api::{route, select, select_error, instrument, known_type, window, bars_clean, spec_of, run_json, optimize_json, series_json, lab_stats}`; test helpers `state(n, ticker, volume)`, `q(extra)` | `lab/api.rs` | Tasks 5, 12, 13 |
| `testdata::{walk, from_ohlc, closes, go, close_eq, assert_series, cut, with_breadth}`; `series::tests::business_days` | `lab/testdata.rs`, `lab/series.rs` | fixtures |
| `State { ctxs, baskets, vix, market_extra }` (private fields, visible to the child module `lab`); `select_context`, `SelectedContext::{context, native, evidence}`, `preloaded_market`, `native_benchmark`, `BENCHMARK`, `resolve_context`, `stats`, `years`, `conclude_rank_with_outcomes`, `RANK_STATS`, `MIN_TRADES`, `MIN_PER_YEAR`, `num`, `stats_json` | `main.rs` | read-only calls |
| `baskets::Catalogue::{tickers_json, native_evidence, rows, provenance_json}`, `NativeEvidence::{market, asset_type, membership_kind}` | `baskets.rs` | universe members (read-only) |
| `instrument_policy::selected_type(ticker, native)` | `instrument_policy.rs:137` | member and foreign types (read-only) |
| `connors::{Trade, Leg}`, `Trade::ret` | `connors.rs:375-420` | book-format portfolio trades for `stats()` |
| auth `labAction`, `LAB_PUBLISHABLE_MARKETS`; `lab-strategies.mjs` `view`; tests `harness`, `row`, `SPEC` | `webapp/auth/src/*.mjs`, `webapp/tests/lab-strategies.test.mjs` | Task 15 |

Phase 7 names this plan expects (confirm in "Before Task 1"): the reserved-`symbol` refusal in the operand parser; the node evaluator's leaf call to `series::series`; money mode fields on `Exec` (`capital`, `sizing`, `costs`); the per-position exit logic (Customize `Pos` or its Phase 7 successor); `lab/ledger.rs`; the execution-model function that classifies a run instrument (`executable`, `kind`).

## File structure

```
Context/guru/backend/src/lab/rules.rs      operand symbol/vs/fixup, Spec::foreign_symbols, MAX_FOREIGN (Task 1);
                                           pair-indicator vs rules (Task 3); portfolio object, optimize paths (Task 6)
Context/guru/backend/src/lab/foreign.rs    NEW Align, alignment(), align(), VIX names                       (Task 2)
Context/guru/backend/src/lab/bars.rs       Bars.second                                                      (Task 3)
Context/guru/backend/src/lab/pairs.rs      NEW correlation, correlation_log, ratio, spread                 (Task 3)
Context/guru/backend/src/lab/listed.rs     − 4 pair entries                                                 (Task 3)
Context/guru/backend/src/lab/catalogue.rs  + pairs::DEFS, second-instrument check, master-list pin          (Task 3)
Context/guru/backend/src/lab/series.rs     SeriesRef literals, describe (Task 1); test filters (Task 3);
                                           Others, series_in, compile_with (Task 4)
Context/guru/backend/src/lab/breadth.rs    SeriesRef literal (Task 1)
Context/guru/backend/src/lab/api.rs        series_ref call (Task 1); foreign loading + JSON (Task 5);
                                           portfolio run (Task 12) and optimize (Task 13)
Context/guru/backend/src/lab/universe.rs   NEW members, admission, reference calendar, membership note      (Task 7)
Context/guru/backend/src/lab/portfolio.rs  NEW shared-ledger simulator (Tasks 8–11), properties (Task 14)
Context/guru/backend/src/lab/simulate.rs   Pos and its methods pub(super) (visibility only, Task 11)
Context/guru/backend/src/lab/mod.rs        + pub mod foreign, pairs, universe, portfolio
Context/guru/webapp/auth/src/contracts.mjs     labPublishable (Task 15)
Context/guru/webapp/auth/src/lab-strategies.mjs  view uses labPublishable (Task 15)
Context/guru/webapp/tests/contracts.test.mjs, lab-strategies.test.mjs   tests (Task 15)
```

No other file changes: no `main.rs`, book, gateway, frontend, compose, Dockerfile, `package.json` or `Cargo.toml` edit.

## Before Task 1 (once)

- [ ] Confirm Phase 7 is merged into `Nandini` (its gate commit is in `git log --oneline -30`) and `git status --short` is empty.
- [ ] Read the Phase 7 plan's final code for: the operand parser and its reserved `symbol` refusal; where node leaves call `series::series`; `Exec` money fields; the per-position exit code; `lab/ledger.rs`; the execution-model classifier. Write the actual names next to this plan's "Phase 7 names" list in your task notes; every later task uses them.
- [ ] Baselines: `BASE=$(git rev-parse HEAD)`; `cd Context/guru/backend && VAULT_IDX="/run/user/1000/gvfs/smb-share:server=192.168.4.50,share=share/Manan Pujara/Book-to-rust/idx_data.csv" CARGO_BUILD_JOBS=2 cargo test 2>&1 | grep 'test result'` (record total and `cargo test lab:: 2>&1 | grep 'test result'` lab count; Phase 6 HEAD had 848 / 276 before Phase 7); `npm --prefix Context/guru/webapp/auth ci`.
- [ ] `python3 scripts/check_book_untouched.py --base origin/Manan` → `"problems": []`.

---

### Task 1: Operand fields `symbol`, `vs`, `fixup` (`lab/rules.rs`)

**Files:**
- Modify: `Context/guru/backend/src/lab/rules.rs` (constants, `SeriesRef`, new `Foreign`, `Spec::foreign_symbols`, `parse_spec`, `operand`, `series_ref`)
- Modify: `Context/guru/backend/src/lab/series.rs` (three `SeriesRef` literals, a guard in `series`, `describe`)
- Modify: `Context/guru/backend/src/lab/breadth.rs` (one `SeriesRef` literal), `Context/guru/backend/src/lab/api.rs` (`series_json`'s `series_ref` call)
- Test: `rules.rs` tests module

**Interfaces:**
- Produces: `pub struct Foreign { pub symbol: Option<String>, pub vs: Option<String>, pub fixup: bool }` (Default, Clone, Debug, PartialEq); `SeriesRef.foreign: Foreign`; `Spec::foreign_symbols(&self) -> Vec<String>` (sorted, distinct; Phase 7: walk every leaf of every node, expression and `short_side` list); `series_ref(at, id, given, tf, out, offset, foreign: Foreign)`; `pub const MAX_FOREIGN: usize = 4; pub const MAX_SYMBOL_LEN: usize = 80;`. `series::series` refuses a non-default `foreign` with `Unsupported` until Task 4.

- [ ] **Step 1: Write the failing tests** — append to the `rules.rs` tests module (before `engine_errors_map_to_types`):

```rust
    // ---- Phase 8: operands on another instrument (spec 2026-10-15 phase 8 §4)
    #[test] fn foreign_operands_parse_and_name_their_instruments() {
        let s = parse_spec(&with(r#"{"s":"close","tf":"D"}"#, r#"{"s":"close","symbol":"NIFTY 50","fixup":"previous"}"#)).unwrap();
        let Operand::Series(r) = &s.entry[1].lhs else { panic!() };
        assert_eq!(r.foreign, Foreign { symbol: Some("NIFTY 50".into()), vs: None, fixup: true });
        assert_eq!(s.foreign_symbols(), vec!["NIFTY 50".to_string()]);
        assert_eq!(crate::lab::series::describe(&s.entry[1]), "NIFTY 50:close (fixup previous) > sma(length=40)@W");
        let w = parse_spec(&with(r#"{"s":"sma","p":{"length":40},"tf":"W"}"#, r#"{"s":"sma","p":{"length":40},"tf":"W","symbol":"basket:nifty50:TCS","offset":1}"#)).unwrap();
        let Operand::Series(r) = &w.entry[1].rhs else { panic!() };
        assert_eq!((r.tf, r.offset, r.foreign.symbol.as_deref(), r.foreign.fixup), (Tf::W, 1, Some("basket:nifty50:TCS"), false));
        assert!(parse_spec(EXAMPLE).unwrap().foreign_symbols().is_empty(), "v1 specs read no other instrument");
    }
    #[test] fn foreign_operands_are_bounded_and_named() {
        let cond = |sym: &str| format!(r#"{{"lhs":{{"s":"close","symbol":"{sym}"}},"op":">","rhs":{{"value":1}}}}"#);
        let five = format!(r#"{{"v":1,"entry":{{"all":[{}]}},"exit":{{"any":[{{"lhs":{{"s":"bars_held"}},"op":">=","rhs":{{"value":3}}}}]}}}}"#,
            ["A", "B", "C", "D", "E"].map(cond).join(","));
        for (text, needle) in [
            (with(r#"{"s":"close","tf":"D"}"#, r#"{"s":"close","symbol":""}"#), "entry.all[1].lhs.symbol must name an instrument"),
            (with(r#"{"s":"close","tf":"D"}"#, &format!(r#"{{"s":"close","symbol":"{}"}}"#, "X".repeat(81))), "1 to 80 characters"),
            (with(r#"{"s":"close","tf":"D"}"#, r#"{"s":"close","symbol":7}"#), "entry.all[1].lhs.symbol must name"),
            (with(r#"{"s":"close","tf":"D"}"#, r#"{"s":"close","fixup":"previous"}"#), "fixup applies to a symbol operand"),
            (with(r#"{"s":"close","tf":"D"}"#, r#"{"s":"close","symbol":"SPX","fixup":"yes"}"#), "fixup must be"),
            (with(r#"{"s":"close","tf":"D"}"#, r#"{"s":"close","vs":"SPX"}"#), "price source 'close' takes no p, out or vs"),
            (with(r#"{"s":"sma","p":{"length":40},"tf":"W"}"#, r#"{"s":"sma","p":{"length":40},"vs":"SPX"}"#), "vs applies to the second-instrument indicators"),
            (with(r#"{"s":"bars_held"}"#, r#"{"s":"bars_held","symbol":"SPX"}"#), "bars_held takes no"),
            (five, "reads 5 other instruments (A, B, C, D, E); the limit is 4"),
        ] {
            let e = err(&text);
            assert_eq!(e.status(), "invalid", "{text}: {e:?}");
            assert!(e.message().contains(needle), "{needle:?} not in {:?}", e.message());
        }
    }
```

Phase 7 note: Phase 7's own test that `symbol` answers `unsupported` ("operands from another symbol … arrive in Phase 8") is deleted in this task (it is replaced by the two tests above). If Phase 7's `with()` fixture or `EXAMPLE` differs, keep the same inputs and needles.

- [ ] **Step 2: Run to verify failure**

Run: `cd Context/guru/backend && CARGO_BUILD_JOBS=2 cargo test lab::rules 2>&1 | tail -5`
Expected: compile error `cannot find struct Foreign` / `no field foreign`.

- [ ] **Step 3: Implement** — in `rules.rs`:

(a) after `OPERATORS`:
```rust
/// Other instruments one spec may read (operand `symbol` and `vs`, together).
pub const MAX_FOREIGN: usize = 4;
pub const MAX_SYMBOL_LEN: usize = 80;
```
(b) `SeriesRef` gains a field, and the new type:
```rust
pub struct SeriesRef { pub id: String, pub params: Params, pub tf: Tf, pub out: usize, pub offset: usize, pub foreign: Foreign }
/// Another instrument behind an operand (Phase 8 spec §4): `symbol` = AmiBroker Foreign()/SetForeign, `vs` = the second
/// instrument of a pair indicator, `fixup` = fill missing dates from the latest earlier bar (AmiBroker fixup=1 analogue).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Foreign { pub symbol: Option<String>, pub vs: Option<String>, pub fixup: bool }
```
(Phase 7 adds `expand` to `SeriesRef`; keep it.)
(c) after `pub struct Spec { … }`:
```rust
impl Spec {
    /// Every other instrument the spec reads (`symbol` and `vs` of all operands), sorted and distinct.
    pub fn foreign_symbols(&self) -> Vec<String> {
        let mut out: Vec<String> = self.entry.iter().chain(&self.exit).flat_map(|c| [&c.lhs, &c.rhs])
            .filter_map(|o| if let Operand::Series(s) = o { Some(s) } else { None })
            .flat_map(|s| s.foreign.symbol.iter().chain(&s.foreign.vs)).cloned().collect();
        out.sort(); out.dedup();
        out
    }
}
```
Phase 7: iterate every `SeriesRef` leaf with Phase 7's leaf walker (the one that counts the 40 leaves), including `short_side` lists and state operands; the body after collecting is the same. Task 6 adds `portfolio.score`.
(d) in `parse_spec`, right after the `Spec { … }` value is built:
```rust
    let foreign = spec.foreign_symbols();
    if foreign.len() > MAX_FOREIGN { return Err(inv(format!("the spec reads {} other instruments ({}); the limit is {MAX_FOREIGN}", foreign.len(), foreign.join(", ")))); }
```
(e) in `operand` (Phase 7: the series-leaf parser): allowed fields add `"symbol", "vs", "fixup"` and **remove Phase 7's reserved-`symbol` refusal**; the `bars_held` check becomes
```rust
        if ["p", "tf", "out", "offset", "symbol", "vs", "fixup"].iter().any(|k| v.get(k).is_some()) { return Err(inv(format!("{at}: bars_held takes no p, tf, out, offset, symbol, vs or fixup"))); }
```
(Phase 7 adds `expand` to both lists.) The final call becomes:
```rust
    let name = |k: &str| -> Result<Option<String>, LabError> {
        match v.get(k) {
            None => Ok(None),
            Some(x) => x.as_str().filter(|s| !s.trim().is_empty() && s.len() <= MAX_SYMBOL_LEN && !s.chars().any(char::is_control))
                .map(|s| Some(s.to_string())).ok_or_else(|| inv(format!("{at}.{k} must name an instrument of this market (1 to {MAX_SYMBOL_LEN} characters)"))),
        }
    };
    let fixup = match v.get("fixup").map(Json::as_str) {
        None | Some(Some("none")) => false,
        Some(Some("previous")) => true,
        _ => return Err(inv(format!("{at}.fixup must be \"none\" or \"previous\""))),
    };
    let foreign = Foreign { symbol: name("symbol")?, vs: name("vs")?, fixup };
    if fixup && foreign.symbol.is_none() { return Err(inv(format!("{at}.fixup applies to a symbol operand (another instrument's missing dates)"))); }
    series_ref(at, id, &given, tf, out, offset, foreign).map(Operand::Series)
```
(f) `series_ref` gains `foreign: Foreign` as its last parameter; the price-source branch:
```rust
        if !given.is_empty() || out.is_some() || foreign.vs.is_some() { return Err(inv(format!("{at}: price source '{id}' takes no p, out or vs"))); }
        return Ok(SeriesRef { id: id.into(), params: Params::new(), tf, out: 0, offset, foreign });
```
before `let params = catalogue::resolve_params(…)`:
```rust
    if def.needs.second_symbol != foreign.vs.is_some() {
        return Err(inv(if foreign.vs.is_some() { format!("{at}.vs applies to the second-instrument indicators (correlation, correlation_log, ratio, spread)") }
                        else { format!("{at}: {} compares two instruments; name the second one in vs", def.name) }));
    }
    if foreign.vs.is_some() && tf != Tf::D { return Err(inv(format!("{at}: second-instrument indicators run on daily bars (tf D)"))); }
```
and the final `Ok(SeriesRef { …, offset, foreign })`. (Until Task 3 the pair entries are still listed, so their `needs_data` refusal fires before this check.)

In `series.rs`: add `foreign: Default::default()` to the three `SeriesRef { … }` literals (ATR in `compile`, `sref` in tests, the bounds test); at the top of `series()`:
```rust
    // Phase 8 Task 4 replaces this guard with the per-symbol lookup; until then another instrument is never read as this one.
    if s.foreign != Default::default() { return Err(LabError::Unsupported("operands on another instrument (symbol / vs) are not wired yet".into())); }
```
and in `describe`, after the offset line:
```rust
            if let Some(v) = &s.foreign.vs { t = format!("{t} vs {v}"); }
            if let Some(x) = &s.foreign.symbol { t = format!("{x}:{t}"); }
            if s.foreign.fixup { t = format!("{t} (fixup previous)"); }
```
In `breadth.rs` the `adl` literal and in `api.rs` `series_json` the call `rules::series_ref("series", id, &crate::params_of(q), tf, None, 0, Default::default())?`. (Phase 7 may have more literals: add the field to each.)

- [ ] **Step 4: Run to verify pass**

Run: `cd Context/guru/backend && CARGO_BUILD_JOBS=2 cargo test lab:: 2>&1 | grep 'test result'`
Expected: lab count = baseline + 2 (Phase 7's reserved-`symbol` assertion removed), 0 failed. Also `cargo test pinned` (Phase 7's pinned-output test) passes.

- [ ] **Step 5: Commit**

```bash
git add Context/guru/backend/src/lab/rules.rs Context/guru/backend/src/lab/series.rs Context/guru/backend/src/lab/breadth.rs Context/guru/backend/src/lab/api.rs
git commit -m "Accept symbol, vs and fixup on lab operands (Phase 8 grammar, not yet evaluated)"
```

---

### Task 2: Date alignment and the session rule (`lab/foreign.rs`)

**Files:**
- Create: `Context/guru/backend/src/lab/foreign.rs`
- Modify: `Context/guru/backend/src/lab/mod.rs` (`pub mod foreign;`)

**Interfaces:**
- Produces: `pub enum Align { SameDate, PriorDate }` with `name() -> &'static str` ("same_date"/"prior_date"); `pub fn alignment(run: &str, other: &str, close_fills: bool) -> Align`; `pub fn align(run_dates: &[String], other_dates: &[String], values: &[f64], rule: Align, fixup: bool) -> Vec<f64>`; `pub const VIX: [&str; 2] = ["INDIA VIX", "CBOE VIX"];`.

- [ ] **Step 1: Write the failing tests** — create `foreign.rs` with only the module doc, `use` lines and this test module:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::lab::testdata::assert_series;
    const N: f64 = f64::NAN;
    fn d(v: &[&str]) -> Vec<String> { v.iter().map(|s| s.to_string()).collect() }

    #[test] fn same_date_drops_other_only_dates_and_leaves_holes_empty() {
        // AmiBroker Foreign(): bars that exist only in the foreign symbol are removed; fixup 0 leaves holes Null.
        let run = d(&["2024-01-01", "2024-01-02", "2024-01-03", "2024-01-05"]);
        let other = d(&["2024-01-01", "2024-01-03", "2024-01-04", "2024-01-05"]);
        assert_series(&align(&run, &other, &[1., 3., 4., 5.], Align::SameDate, false), &[1., N, 3., 5.]);
        // fixup previous (AmiBroker fixup 1 analogue): the hole takes the latest earlier bar
        assert_series(&align(&run, &other, &[1., 3., 4., 5.], Align::SameDate, true), &[1., 1., 3., 5.]);
    }
    #[test] fn prior_date_reads_the_latest_bar_since_the_previous_run_bar() {
        let run = d(&["2024-01-01", "2024-01-02", "2024-01-03", "2024-01-04"]);
        let other = d(&["2024-01-01", "2024-01-02", "2024-01-04"]);
        // first run bar: nothing before it; 02 reads 01; 03 reads 02; 04: latest before is 02 < previous run date 03 → hole
        assert_series(&align(&run, &other, &[1., 2., 4.], Align::PriorDate, false), &[N, 1., 2., N]);
        assert_series(&align(&run, &other, &[1., 2., 4.], Align::PriorDate, true), &[N, 1., 2., 2.]);
    }
    #[test] fn crypto_weekend_reaches_monday_on_a_session_market() {
        let nyse = d(&["2024-01-05", "2024-01-08"]);
        let crypto = d(&["2024-01-05", "2024-01-06", "2024-01-07", "2024-01-08"]);
        assert_series(&align(&nyse, &crypto, &[5., 6., 7., 8.], Align::PriorDate, false), &[N, 7.]);
    }
    #[test] fn alignment_rules_follow_session_and_fill_mode() {
        use Align::*;
        for (run, other, close, want) in [
            ("basket:nifty50:TCS", "NIFTY 50", true, SameDate), ("NIFTY 50", "INDIA VIX", true, SameDate),
            ("SPY", "SPX", true, SameDate), ("SPY", "CBOE VIX", true, PriorDate), ("SPY", "CBOE VIX", false, SameDate),
            ("HSI:HSI", "IG:DAX", false, PriorDate), ("IG:DAX", "IG:FTSE", false, PriorDate),
            ("CRYPTO:BTCUSD", "CRYPTO:ETHUSD", true, SameDate), ("HSI:HSI", "HSI:HSI", true, SameDate),
            ("NIFTY 50", "IG:DAX", false, PriorDate),
        ] { assert_eq!(alignment(run, other, close), want, "{run} reading {other} (close fills {close})"); }
        assert_eq!((SameDate.name(), PriorDate.name()), ("same_date", "prior_date"));
    }
    #[test] fn aligned_values_never_come_from_a_later_date() {
        // pseudo-random calendars: every non-NaN output equals a value whose date is <= (same) or < (prior) the run date
        let mut s = 7u64;
        let mut next = || { s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407); (s >> 33) % 3 };
        let (mut run, mut other) = (vec![], vec![]);
        for day in 0..400 {
            let date = crate::lab::resample::civil(crate::lab::resample::day_number("2020-01-01") + day);
            if next() != 0 { run.push(date.clone()); }
            if next() != 0 { other.push(date); }
        }
        let values: Vec<f64> = (0..other.len()).map(|k| k as f64).collect();
        for rule in [Align::SameDate, Align::PriorDate] {
            for fixup in [false, true] {
                for (i, v) in align(&run, &other, &values, rule, fixup).iter().enumerate() {
                    if v.is_nan() { continue; }
                    let src = &other[*v as usize];
                    assert!(if rule == Align::SameDate { *src <= run[i] } else { *src < run[i] }, "{rule:?} fixup {fixup}: {} read {src}", run[i]);
                }
            }
        }
    }
}
```

- [ ] **Step 2: Run to verify failure** — add `pub mod foreign;` to `mod.rs` (alphabetical, after `fidelity`). Run: `CARGO_BUILD_JOBS=2 cargo test lab::foreign 2>&1 | tail -5` → compile errors (`align`, `Align`, `alignment` not found).

- [ ] **Step 3: Implement** — above the tests in `foreign.rs`:

```rust
//! Aligning another instrument's series to the run instrument's dates (Phase 8 spec §4.3). AmiBroker Foreign()
//! "synchronizes the data file you are referencing with the currently selected symbol" and drops bars the selected
//! symbol lacks; holes stay empty unless fixup is asked for. Values never come from a date after the run bar.
use super::markets;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Align { SameDate, PriorDate }
impl Align { pub fn name(self) -> &'static str { match self { Self::SameDate => "same_date", Self::PriorDate => "prior_date" } } }

/// Volatility indices an engine holds outside its instrument map (book VIX files); usable as signal symbols only.
pub const VIX: [&str; 2] = ["INDIA VIX", "CBOE VIX"];

/// Which bar of `other` a bar of `run` may read. `close_fills`: some order of the spec fills at the signal bar's close.
pub fn alignment(run: &str, other: &str, close_fills: bool) -> Align {
    match (markets::gl(run), markets::gl(other)) {
        (Some(a), Some(b)) if a.timezone == b.timezone && a.calendar == b.calendar => Align::SameDate,
        (Some(_), _) | (_, Some(_)) => Align::PriorDate, // different session (an India/US–gl mix is refused earlier by the API)
        // CBOE VIX settles about 15 minutes after the NYSE close: unknown at that close, known before the next open.
        _ if other == "CBOE VIX" && close_fills => Align::PriorDate,
        _ => Align::SameDate,
    }
}

/// `values[k]` belongs to `other_dates[k]` (both ascending, ISO dates). One output per run date: SameDate → the value
/// dated that day; PriorDate → the latest value dated before it and on/after the previous run date. Missing → NaN.
/// `fixup` drops the lower bound (the latest value on/before, or before, the run date): it fills missing dates only.
pub fn align(run_dates: &[String], other_dates: &[String], values: &[f64], rule: Align, fixup: bool) -> Vec<f64> {
    let mut out = vec![f64::NAN; run_dates.len()];
    let mut k = 0; // count of other dates already eligible
    for (i, t) in run_dates.iter().enumerate() {
        while k < other_dates.len() && (other_dates[k] < *t || (rule == Align::SameDate && other_dates[k] == *t)) { k += 1; }
        let Some(j) = k.checked_sub(1) else { continue };
        let ok = fixup || match rule {
            Align::SameDate => other_dates[j] == *t,
            Align::PriorDate => i > 0 && other_dates[j] >= run_dates[i - 1],
        };
        if ok { out[i] = values[j]; }
    }
    out
}
```

- [ ] **Step 4: Run to verify pass** — `CARGO_BUILD_JOBS=2 cargo test lab::foreign 2>&1 | grep 'test result'` → 5 passed.

- [ ] **Step 5: Commit**

```bash
git add Context/guru/backend/src/lab/foreign.rs Context/guru/backend/src/lab/mod.rs
git commit -m "Add lab date alignment for other instruments (same-date, prior-date, fixup)"
```

---

### Task 3: Second-instrument indicators computed (`pairs.rs`, `bars.rs`, `catalogue.rs`, `listed.rs`)

**Files:**
- Create: `Context/guru/backend/src/lab/pairs.rs`
- Modify: `Context/guru/backend/src/lab/bars.rs` (`second` field + `from_rows` literal), `catalogue.rs` (`all()`, `run()`, tests), `listed.rs` (remove 4 entries and `SECOND`), `series.rs` (three generic test filters), `mod.rs` (`pub mod pairs;`), `rules.rs` (tests)

**Interfaces:**
- Consumes: `Foreign.vs` validation (Task 1).
- Produces: `Bars.second: Option<Vec<f64>>` (None everywhere except pair computation); `pairs::DEFS` (ids `correlation`, `correlation_log`, `ratio`, `spread`, category "Second instrument", `needs.second_symbol: true`, computed); `pairs::pearson(x, y, n) -> Vec<f64>`; `catalogue::run` returns `Err("needs a second instrument")` when `second` is None for these.

- [ ] **Step 1: Write the failing tests** — (a) create `pairs.rs` with this test module:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::lab::catalogue::{get, run};
    use crate::lab::testdata::{assert_series, close_eq, closes, go, walk};
    const N: f64 = f64::NAN;
    fn pair(a: &[f64], b: &[f64]) -> Bars { let mut x = closes(a); x.second = Some(b.to_vec()); x }

    #[test] fn ratio_and_spread_follow_the_second_close() {
        let b = pair(&[10., 12., 9.], &[5., 0., N]);
        assert_series(&go("ratio", &b, &[])[0], &[2., N, N]);
        assert_series(&go("spread", &b, &[])[0], &[5., 12., N]);
    }
    #[test] fn correlation_is_pearson_over_a_bar_counted_window() {
        let b = pair(&[1., 2., 3., 5., 4.], &[2., 4., 6., 7., 9.]);
        assert_series(&go("correlation", &b, &[("length", 3.)])[0], &[N, N, 1., 13. / 14., (3f64 / 28.).sqrt()]);
        assert_series(&go("correlation", &pair(&[1., 2., 3., 4.], &[1., N, 3., 4.]), &[("length", 2.)])[0], &[N, N, N, 1.]);
        assert_series(&go("correlation", &pair(&[1., 1., 1.], &[1., 2., 3.]), &[("length", 3.)])[0], &[N, N, N]); // zero variance
        // log returns: identical returns correlate 1; a constant window has no value
        assert_series(&go("correlation_log", &pair(&[1., 2., 4., 6.], &[10., 20., 40., 60.]), &[("length", 2.)])[0], &[N, N, N, 1.]);
    }
    #[test] fn pairs_need_a_second_instrument_and_never_use_later_bars() {
        for id in ["correlation", "correlation_log", "ratio", "spread"] {
            let d = get(id).unwrap();
            assert!(d.compute.is_some() && d.needs.second_symbol && d.note.is_empty(), "{id}");
            assert_eq!(run(&d, &walk(30, 1, false), &Default::default()).unwrap_err(), "needs a second instrument");
            let mut full = walk(300, 2, false);
            full.second = Some(walk(300, 3, false).close);
            let want = run(&d, &full, &Default::default()).unwrap();
            for cut in [37usize, 150, 299] {
                let mut pre = crate::lab::testdata::cut(&full, cut);
                pre.second = Some(full.second.as_ref().unwrap()[..cut].to_vec());
                let got = run(&d, &pre, &Default::default()).unwrap();
                for i in 0..cut { assert!(close_eq(got[0][i], want[0][i]), "{id} bar {i} changes when later bars exist"); }
            }
            assert!(want[0][..d.first[0]].iter().all(|v| v.is_nan()) && !want[0][d.first[0]].is_nan(), "{id} first value at {}", d.first[0]);
        }
    }
}
```
(b) in `rules.rs` tests append:
```rust
    #[test] fn pair_indicators_need_vs_on_daily_bars() {
        let pair = |o: &str| format!(r#"{{"v":1,"entry":{{"all":[{{"lhs":{o},"op":">","rhs":{{"value":1}}}}]}},"exit":{{"any":[{{"lhs":{{"s":"bars_held"}},"op":">=","rhs":{{"value":3}}}}]}}}}"#);
        let s = parse_spec(&pair(r#"{"s":"correlation","p":{"length":60},"vs":"SPX"}"#)).unwrap();
        assert_eq!(s.foreign_symbols(), vec!["SPX".to_string()]);
        assert!(err(&pair(r#"{"s":"ratio"}"#)).message().contains("Ratio compares two instruments; name the second one in vs"));
        assert!(err(&pair(r#"{"s":"spread","vs":"SPX","tf":"W"}"#)).message().contains("run on daily bars (tf D)"));
        assert!(parse_spec(&pair(r#"{"s":"ratio","vs":"NIFTY 50","symbol":"basket:nifty50:TCS"}"#)).unwrap().foreign_symbols().len() == 2);
    }
```
(c) in `catalogue.rs` `catalogue_matches_the_phase6_master_list` change the two counts: `183` → `187` and `116` → `112` (message unchanged: the list is still 299 entries).

- [ ] **Step 2: Run to verify failure** — add `pub mod pairs;` to `mod.rs`; `CARGO_BUILD_JOBS=2 cargo test lab:: 2>&1 | tail -5` → compile errors (`second` field, `DEFS` missing).

- [ ] **Step 3: Implement** — (a) `bars.rs`: add to `Bars` after `breadth`:
```rust
    /// Close of a pair indicator's `vs` instrument aligned to these daily bars (lab/foreign.rs); None everywhere else.
    pub second: Option<Vec<f64>>,
```
and `second: None,` in the `from_rows` literal (W/M resamples get None through `Bars::default()`).
(b) `pairs.rs`, above the tests:
```rust
//! Second-instrument indicators (Phase 8 spec §4.5): TradingView "Correlation Coefficient", "Correlation - Log",
//! "Ratio" and "Spread"; AmiBroker Correl(C, Foreign(t,"C"), n) and RelStrength(t). `Bars.second` is the `vs`
//! instrument's close aligned to these bars (lab/foreign.rs), NaN where it has no value. Declared definitions;
//! TradingView parity fixtures pending, as for every Phase 6 indicator.
use super::bars::Bars;
use super::catalogue::{int, n, IndicatorDef, Needs, Output, ParamSpec, Params, Placement};
use super::ta::NA;

const SECOND: Needs = Needs { second_symbol: true, ..Needs::NONE };
const L20: &[ParamSpec] = &[int("length", "Length", 20., 2., 2000.)];
const fn def(id: &'static str, name: &'static str, params: &'static [ParamSpec], definition: &'static str, first: &'static [usize], f: fn(&Bars, &Params) -> Output) -> IndicatorDef {
    IndicatorDef { id, name, category: "Second instrument", params, outputs: &["value"], placement: Placement::Pane, needs: SECOND, definition, first, sparse: false, compute: Some(f), note: "" }
}
pub const DEFS: &[IndicatorDef] = &[
    def("correlation", "Correlation Coefficient (CC)", L20, "Pearson correlation of close and the vs instrument's close over length bars; a missing value in the window or zero variance gives no value", &[19], correlation),
    def("correlation_log", "Correlation - Log", L20, "Pearson correlation of the log returns ln(c/c[1]) of close and of the vs instrument's close over length bars", &[20], correlation_log),
    def("ratio", "Ratio", &[], "close ÷ the vs instrument's close (no value when that close is 0)", &[0], ratio),
    def("spread", "Spread", &[], "close − the vs instrument's close", &[0], spread),
];
fn second(b: &Bars) -> Result<&[f64], String> { b.second.as_deref().ok_or_else(|| "needs a second instrument".to_string()) }
fn ratio(b: &Bars, _: &Params) -> Output { let s = second(b)?; Ok(vec![b.close.iter().zip(s).map(|(a, c)| if *c == 0. { NA } else { a / c }).collect()]) }
fn spread(b: &Bars, _: &Params) -> Output { let s = second(b)?; Ok(vec![b.close.iter().zip(s).map(|(a, c)| a - c).collect()]) }
/// Bar-counted Pearson correlation: no value when any of the last n pairs is missing or either side is constant.
pub fn pearson(x: &[f64], y: &[f64], n: usize) -> Vec<f64> {
    (0..x.len()).map(|i| {
        if n == 0 || i + 1 < n { return NA; }
        let (a, b) = (&x[i + 1 - n..=i], &y[i + 1 - n..=i]);
        if a.iter().chain(b).any(|v| v.is_nan()) { return NA; }
        let (ma, mb) = (a.iter().sum::<f64>() / n as f64, b.iter().sum::<f64>() / n as f64);
        let (mut sab, mut saa, mut sbb) = (0., 0., 0.);
        for k in 0..n { let (da, db) = (a[k] - ma, b[k] - mb); sab += da * db; saa += da * da; sbb += db * db; }
        if saa == 0. || sbb == 0. { NA } else { sab / (saa * sbb).sqrt() }
    }).collect()
}
fn log_returns(x: &[f64]) -> Vec<f64> { (0..x.len()).map(|i| if i == 0 || !(x[i] > 0.) || !(x[i - 1] > 0.) { NA } else { (x[i] / x[i - 1]).ln() }).collect() }
fn correlation(b: &Bars, p: &Params) -> Output { let s = second(b)?; Ok(vec![pearson(&b.close, s, n(p, "length"))]) }
fn correlation_log(b: &Bars, p: &Params) -> Output { let s = second(b)?; Ok(vec![pearson(&log_returns(&b.close), &log_returns(s), n(p, "length"))]) }
```
(c) `catalogue.rs`: append `super::pairs::DEFS` to the list in `all()` (after `super::ratings::DEFS`); in `run()` after the breadth line: `if def.needs.second_symbol && bars.second.is_none() { return Err("needs a second instrument".into()); }`; in the tests module, `computable` becomes `d.compute.is_some() && !d.needs.breadth && !d.needs.second_symbol` (pairs are covered in `pairs.rs`).
(d) `listed.rs`: delete the `correlation`, `correlation_log`, `ratio`, `spread` lines and the now-unused `SECOND` constant. `sources.rs` keeps its rows (keyed by id).
(e) `series.rs` tests: in `every_computed_indicator_is_reachable_from_a_rule`, `signals_never_use_later_bars` and `every_indicator_at_parameter_bounds_never_panics` change the filter `d.compute.is_some()` to `d.compute.is_some() && !d.needs.second_symbol` (Task 4 adds the pair rule test); in `listed_entries_refuse_with_their_note` delete the `("correlation", "needs_data"),` pair. (Phase 7 extends these tests to Q/Y/`5D`; apply the same filter there.)

- [ ] **Step 4: Run to verify pass** — `CARGO_BUILD_JOBS=2 cargo test lab:: 2>&1 | grep 'test result'` → lab count = Task 1 count + 4, 0 failed. The api test `catalogue_lists_notes_needs_and_sources` still finds `"id":"correlation"` with `"second_symbol":true` (now with `"computed":true`).

- [ ] **Step 5: Commit**

```bash
git add Context/guru/backend/src/lab/pairs.rs Context/guru/backend/src/lab/bars.rs Context/guru/backend/src/lab/catalogue.rs Context/guru/backend/src/lab/listed.rs Context/guru/backend/src/lab/series.rs Context/guru/backend/src/lab/rules.rs Context/guru/backend/src/lab/mod.rs
git commit -m "Compute lab correlation, correlation-log, ratio and spread against a second instrument"
```

---

### Task 4: Operands resolved on other instruments (`series.rs`)

**Files:**
- Modify: `Context/guru/backend/src/lab/series.rs` (`Others`, `series_in`, `compile_with`, `compile`, the Task 1 guard, the cache key)
- Test: `series.rs` tests module

**Interfaces:**
- Consumes: `foreign::{align, alignment, Align}` (Task 2), `Bars.second` (Task 3), `Foreign` (Task 1).
- Produces: `pub struct Others { pub run: String, pub close_fills: bool, pub frames: std::sync::Arc<BTreeMap<String, Frames>> }` (Default; `Arc` so optimizer threads and portfolio members share one load) with `pub fn with_run(&self, run: &str) -> Others` (same frames, another run instrument); `pub fn series_in(fr: &Frames, others: &Others, cache: &mut Cache, s: &SeriesRef) -> Result<Vec<f64>, LabError>`; `pub fn compile_with(spec: &Spec, fr: &Frames, others: &Others, cache: &mut Cache) -> Result<Compiled, LabError>`; `compile(spec, fr, cache)` = `compile_with(spec, fr, &Others::default(), cache)`. Phase 7: every node/expression leaf that called `series(fr, cache, s)` calls `series_in(fr, others, cache, s)`; thread `others` through Phase 7's compile functions the same way `cache` is threaded.

- [ ] **Step 1: Write the failing tests** — append to the `series.rs` tests module:

```rust
    // ---- Phase 8: other instruments
    fn others(run: &str, close_fills: bool, list: Vec<(&str, Bars)>) -> Others {
        Others { run: run.into(), close_fills, frames: std::sync::Arc::new(list.into_iter().map(|(t, b)| (t.to_string(), Frames::new(b))).collect()) }
    }
    fn fref(id: &str, tf: Tf, symbol: Option<&str>, vs: Option<&str>, fixup: bool) -> SeriesRef {
        SeriesRef { foreign: crate::lab::rules::Foreign { symbol: symbol.map(Into::into), vs: vs.map(Into::into), fixup }, ..sref(id, tf, 0, 0) }
    }
    #[test] fn a_symbol_operand_reads_the_other_instruments_own_series_by_date() {
        let run = business_days(30, false);
        let mut idx = business_days(30, false);
        for c in idx.close.iter_mut() { *c *= 10.; }
        idx.date.remove(3); idx.close.remove(3); idx.open.remove(3); idx.high.remove(3); idx.low.remove(3); idx.end.pop(); // index holiday
        let o = others("basket:nifty50:TCS", false, vec![("NIFTY 50", idx.clone())]);
        let got = series_in(&Frames::new(run.clone()), &o, &mut Cache::new(), &fref("close", Tf::D, Some("NIFTY 50"), None, false)).unwrap();
        assert_eq!((got[2], got[4]), (idx.close[2], idx.close[3]));
        assert!(got[3].is_nan(), "a missing index date is empty, not filled");
        // weekly SMA computed on the index's own weeks, then aligned
        let w = series_in(&Frames::new(run), &o, &mut Cache::new(), &fref("close", Tf::W, Some("NIFTY 50"), None, false)).unwrap();
        assert!(w[..4].iter().all(|v| v.is_nan()) && w[4] == idx.close[3], "week 1 is visible from its last index session (Friday)");
    }
    #[test] fn fixup_fills_missing_dates_not_indicator_gaps() {
        let run = business_days(40, true);
        let mut other = business_days(40, true);
        for v in [&mut other.open, &mut other.high, &mut other.low, &mut other.close] { v.remove(20); }
        other.date.remove(20); other.end.pop(); other.volume.as_mut().unwrap().remove(20);
        let o = others("SPY", false, vec![("SPX", other.clone())]);
        let close = series_in(&Frames::new(run.clone()), &o, &mut Cache::new(), &fref("close", Tf::D, Some("SPX"), None, true)).unwrap();
        assert_eq!(close[20], other.close[19], "the missing date takes the previous bar");
        let sma = series_in(&Frames::new(run), &o, &mut Cache::new(), &fref("sma", Tf::D, Some("SPX"), None, true)).unwrap();
        assert!(sma[..8].iter().all(|v| v.is_nan()), "warm-up NaNs of the indicator itself are never filled");
    }
    #[test] fn pair_indicators_align_vs_on_the_base_instrument() {
        let (a, b) = (walk(120, 5, false), walk(120, 6, false));
        let o = others("A", false, vec![("B", b.clone()), ("A2", a.clone())]);
        let r = series_in(&Frames::new(a.clone()), &o, &mut Cache::new(), &fref("ratio", Tf::D, None, Some("B"), false)).unwrap();
        assert!(close_eq(r[50], a.close[50] / b.close[50]));
        // base = another symbol, vs = a third: computed on A2 against B, then aligned to the run
        let r2 = series_in(&Frames::new(a.clone()), &o, &mut Cache::new(), &fref("spread", Tf::D, Some("A2"), Some("B"), false)).unwrap();
        assert!(close_eq(r2[50], a.close[50] - b.close[50]));
        // two vs instruments on one base never share a cache entry
        let mut cache = Cache::new();
        let x = series_in(&Frames::new(a.clone()), &o, &mut cache, &fref("ratio", Tf::D, None, Some("B"), false)).unwrap();
        let y = series_in(&Frames::new(a.clone()), &o, &mut cache, &fref("ratio", Tf::D, None, Some("A2"), false)).unwrap();
        assert!(close_eq(y[50], 1.) && !close_eq(x[50], y[50]));
    }
    #[test] fn truncating_either_instrument_never_changes_earlier_aligned_values() {
        let (run, other) = (with_breadth(business_days(420, true)), business_days(420, true));
        for d in all().iter().filter(|d| d.compute.is_some() && !d.needs.second_symbol && !d.needs.breadth) {
            for tf in [Tf::D, Tf::W, Tf::M] {
                let r = SeriesRef { foreign: crate::lab::rules::Foreign { symbol: Some("X".into()), ..Default::default() }, ..sref(d.id, tf, d.outputs.len() - 1, 0) };
                let want = series_in(&Frames::new(run.clone()), &others("R", false, vec![("X", other.clone())]), &mut Cache::new(), &r).unwrap();
                for cut in [97usize, 311] {
                    let o = others("R", false, vec![("X", crate::lab::testdata::cut(&other, cut))]);
                    let got = series_in(&Frames::new(run.clone()), &o, &mut Cache::new(), &r).unwrap();
                    for i in 0..cut - 1 { assert!(close_eq(got[i], want[i]), "{} {tf:?} bar {i} changes when X has bars after {cut}", d.id); }
                }
            }
        }
    }
    #[test] fn compiled_rules_reach_pair_indicators_and_unknown_symbols_are_invalid() {
        let spec = crate::lab::rules::parse_spec(r#"{"v":1,"entry":{"all":[{"lhs":{"s":"correlation","vs":"B"},"op":">","rhs":{"value":0}}]},"exit":{"any":[{"lhs":{"s":"close","symbol":"C"},"op":"<","rhs":{"value":0}}]}}"#).unwrap();
        let fr = Frames::new(walk(100, 1, false));
        let o = others("A", false, vec![("B", walk(100, 2, false))]);
        let e = compile_with(&spec, &fr, &o, &mut Cache::new()).err().unwrap();
        assert_eq!((e.status(), e.message()), ("invalid", "C: not loaded for this request"));
        let o = others("A", false, vec![("B", walk(100, 2, false)), ("C", walk(100, 3, false))]);
        assert!(compile_with(&spec, &fr, &o, &mut Cache::new()).is_ok());
        assert_eq!(compile(&spec, &fr, &mut Cache::new()).err().unwrap().message(), "B: not loaded for this request");
    }
```

- [ ] **Step 2: Run to verify failure** — `CARGO_BUILD_JOBS=2 cargo test lab::series 2>&1 | tail -5` → compile errors (`Others`, `series_in`, `compile_with`).

- [ ] **Step 3: Implement** — in `series.rs`:

(a) imports: `use super::foreign;` and `use super::rules::Foreign;`.
(b) replace the Task 1 guard at the top of `series()` with a guard for `symbol` only (`vs` is computed from `fr.d.second`):
```rust
    if s.foreign.symbol.is_some() { return Err(LabError::Invalid(format!("{}: symbol operands resolve through series_in", s.id))); }
```
(c) in the cache key, append the vs name so two `vs` on one base never collide: change the `format!` to end `…,{},{:?}", …, fp(d.close.last()), s.foreign.vs)`.
(d) new items after `series()`:
```rust
/// Other instruments of one request (Phase 8 spec §4): their frames and how they align to the run instrument.
#[derive(Default)]
pub struct Others { pub run: String, pub close_fills: bool, pub frames: std::sync::Arc<BTreeMap<String, Frames>> }
impl Others {
    /// The same loaded instruments seen from another run instrument (each portfolio member).
    pub fn with_run(&self, run: &str) -> Others { Others { run: run.into(), close_fills: self.close_fills, frames: self.frames.clone() } }
    fn get(&self, t: &str) -> Result<&Frames, LabError> { self.frames.get(t).ok_or_else(|| LabError::Invalid(format!("{t}: not loaded for this request"))) }
}
/// One operand as a series aligned to the run instrument's daily bars, reading another instrument when the operand
/// names one: computed on that instrument's own bars (indicator, timeframe, offset), then aligned by date (§4.3).
pub fn series_in(fr: &Frames, others: &Others, cache: &mut Cache, s: &SeriesRef) -> Result<Vec<f64>, LabError> {
    let f = &s.foreign;
    if f.symbol.is_none() && f.vs.is_none() { return series(fr, cache, s); }
    let (base_name, base) = match &f.symbol { Some(x) => (x.as_str(), others.get(x)?), None => (others.run.as_str(), fr) };
    let local = SeriesRef { foreign: Foreign { vs: f.vs.clone(), ..Foreign::default() }, ..s.clone() };
    let raw = match &f.vs {
        None => series(base, cache, &local)?,
        Some(v) => {
            let vf = others.get(v)?;
            let mut d = base.d.clone();
            d.second = Some(foreign::align(&base.d.date, &vf.d.date, &vf.d.close, foreign::alignment(base_name, v, others.close_fills), false));
            // shortcut: re-resamples the base for each pair operand (tf D only); cache the paired frames if optimizer profiles show it.
            series(&Frames::new(d), cache, &local)?
        }
    };
    Ok(match &f.symbol {
        None => raw,
        Some(x) => foreign::align(&fr.d.date, &base.d.date, &raw, foreign::alignment(&others.run, x, others.close_fills), f.fixup),
    })
}
```
(`Frames::new` is Phase 7's constructor with all its timeframes; keep the call.)
(e) `val` and `list` take `others: &Others` and call `series_in(fr, others, cache, s)`; `compile` becomes:
```rust
pub fn compile(spec: &Spec, fr: &Frames, cache: &mut Cache) -> Result<Compiled, LabError> { compile_with(spec, fr, &Others::default(), cache) }
pub fn compile_with(spec: &Spec, fr: &Frames, others: &Others, cache: &mut Cache) -> Result<Compiled, LabError> {
    // body of the old compile, passing `others` to list(); ATR stays on the run instrument (fills and stops use its prices)
}
```
(f) in the test `listed_entries_refuse_with_their_note` nothing changes; the Task 3 filter on `every_computed_indicator_is_reachable_from_a_rule` stays (pairs are covered above).

- [ ] **Step 4: Run to verify pass** — `CARGO_BUILD_JOBS=2 cargo test lab:: 2>&1 | grep 'test result'` → +5, 0 failed; `cargo test pinned` and `cargo test templates_reproduce` pass (no spec without `symbol`/`vs` changes path).

- [ ] **Step 5: Commit**

```bash
git add Context/guru/backend/src/lab/series.rs
git commit -m "Resolve lab operands on other instruments: own-calendar compute, date alignment, pair inputs"
```

---

### Task 5: Foreign runs through the API (`api.rs`)

**Files:**
- Modify: `Context/guru/backend/src/lab/api.rs` (new `others_for`, `close_fills`, uses in `run_json`, `optimize_json`, `series_json`; tests)

**Interfaces:**
- Consumes: `Others`, `compile_with`, `series_in` (Task 4); `foreign::{alignment, VIX}` (Task 2).
- Produces: `fn others_for(st: &State, run: &str, spec_symbols: &[String], fixups: &BTreeSet<String>, close_fills: bool) -> Result<(Others, String), LabError>` (the `String` is `""` or `,"foreign":[…]`); `fn close_fills(x: &Exec) -> bool`; `fn frames_of(st: &State, ticker: &str) -> Result<(Frames, &'static str, String, String), LabError>` (frames, type, first, last) — reused by Task 7.

- [ ] **Step 1: Write the failing tests** — append to the `api.rs` tests module (the helpers `state`, `q` exist; add one multi-instrument state):

```rust
    // ---- Phase 8: other instruments
    fn multi(tickers: &[(&str, usize)]) -> State {
        let mut st = state(800, "SPX", false);
        for (t, skip) in tickers {
            let rows: &'static [Row] = Box::leak(st.ctxs["SPX"].r.iter().enumerate().filter(|(i, _)| i % 97 != *skip)
                .map(|(i, r)| Row { ticker: t.to_string(), close: r.close * (1. + (i % 7) as f64 / 100.), high: r.high * 1.07, ..r.clone() }).collect::<Vec<_>>().into_boxed_slice());
            st.ctxs.insert(t.to_string(), Ctx::new(rows, rows, &[]));
        }
        st
    }
    const FILTER: &str = r#"{"v":1,"entry":{"all":[{"lhs":{"s":"close","symbol":"SPX"},"op":">","rhs":{"s":"sma","p":{"length":50},"symbol":"SPX"}}]},"exit":{"any":[{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":5}}]}}"#;
    #[test] fn a_market_filter_reads_the_index_and_trades_the_etf() {
        let st = multi(&[("SPY", 5)]);
        let j = run_json(&st, &q(&[("ticker", "SPY"), ("spec", FILTER)])).unwrap();
        assert!(j.contains("\"instrument\":{\"ticker\":\"SPY\",\"type\":\"etf\""));
        assert!(j.contains("\"foreign\":[{\"ticker\":\"SPX\",\"type\":\"index\",\"alignment\":\"same_date\",\"fixup\":false"));
        let plain = run_json(&st, &q(&[("ticker", "SPY")])).unwrap();
        assert!(!plain.contains("\"foreign\""), "specs without other instruments keep their exact body");
        // fills come from SPY's own prices (SetForeign … RestorePriceArrays)
        let opens: Vec<f64> = st.ctxs["SPY"].r.iter().map(|r| r.open).collect();
        let entry_px: f64 = j.split("\"entry_px\":").nth(1).unwrap().split(',').next().unwrap().parse().unwrap();
        assert!(opens.iter().any(|o| (o - entry_px).abs() < 1e-9));
    }
    #[test] fn foreign_symbols_stay_in_one_market() {
        let st = multi(&[("SPY", 5)]);
        for (sym, status, needle) in [("NOPE", "invalid", "unknown ticker NOPE on this market"), ("IG:DAX", "needs_data", "served by /api/gl"),
                                       ("NIFTY 50", "needs_data", "cross-market references are not available")] {
            let e = run_json(&st, &q(&[("ticker", "SPY"), ("spec", &FILTER.replace("\"symbol\":\"SPX\"", &format!("\"symbol\":\"{sym}\"")))])).unwrap_err();
            assert_eq!(e.status(), status, "{sym}");
            assert!(e.message().contains(needle), "{sym}: {}", e.message());
        }
    }
    #[test] fn cboe_vix_lags_one_bar_with_close_fills() {
        let mut st = multi(&[("SPY", 5)]);
        st.vix = st.ctxs["SPX"].r.iter().map(|r| Row { ticker: "CBOE VIX".into(), ..r.clone() }).collect();
        let spec = r#"{"v":1,"entry":{"all":[{"lhs":{"s":"close","symbol":"CBOE VIX"},"op":">","rhs":{"value":0}}]},"exit":{"any":[{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":2}}]}EXEC}"#;
        let next = run_json(&st, &q(&[("ticker", "SPY"), ("spec", &spec.replace("EXEC", ""))])).unwrap();
        assert!(next.contains("\"ticker\":\"CBOE VIX\",\"type\":\"volatility_index\",\"alignment\":\"same_date\""));
        let close = run_json(&st, &q(&[("ticker", "SPY"), ("spec", &spec.replace("EXEC", r#","execution":{"fill":"close"}"#))])).unwrap();
        assert!(close.contains("\"alignment\":\"prior_date\""));
    }
    #[test] fn series_route_charts_a_pair_indicator_against_vs() {
        let st = multi(&[("SPY", 5)]);
        let j = series_json(&st, &q(&[("ticker", "SPY"), ("id", "ratio"), ("vs", "SPX")])).unwrap();
        assert!(j.contains("\"id\":\"ratio\"") && j.contains("\"outputs\":[\"value\"]"));
        assert_eq!(series_json(&st, &q(&[("ticker", "SPY"), ("id", "ratio")])).unwrap_err().status(), "invalid");
    }
```
(If Phase 7 makes `state()` rows differ, keep the construction: a second ETF ticker sharing SPX dates minus every 97th row.)

- [ ] **Step 2: Run to verify failure** — `CARGO_BUILD_JOBS=2 cargo test lab::api 2>&1 | tail -5` → the four tests fail (`unsupported`/`not loaded for this request`; no `foreign` key; `vs` ignored).

- [ ] **Step 3: Implement** — in `api.rs`:

```rust
use super::foreign;
use super::rules::{Exec, Fill};
use super::series::Others;
use std::collections::BTreeSet;

/// Any order of the spec fills at the signal bar's own close (decides CBOE VIX alignment, spec §4.3).
/// Phase 7: also true for any other Phase 7 order kind that fills at the signal close.
fn close_fills(x: &Exec) -> bool { x.fill == Fill::Close || x.exit_fill == Fill::Close }

/// Daily frames, type and coverage of an instrument this engine serves (preloaded, basket member or VIX).
fn frames_of(st: &State, ticker: &str) -> Result<(Frames, &'static str, String, String), LabError> {
    if foreign::VIX.contains(&ticker) {
        if st.vix.is_empty() || st.vix.iter().any(|r| r.ticker != ticker) { return Err(LabError::NeedsData(format!("{ticker} is not loaded on this engine"))); }
        let b = Bars::from_rows(&st.vix);
        let (first, last) = (b.date[0].clone(), b.date[b.len() - 1].clone());
        return Ok((Frames::new(b), "volatility_index", first, last));
    }
    let mut q = Query::new();
    q.insert("ticker".into(), ticker.into());
    let selected = select(st, &q)?;
    let held = selected.context(st);
    let (kind, _) = instrument(&selected, ticker);
    let (first, last) = (held.r[0].date.clone(), held.r[held.r.len() - 1].date.clone());
    Ok((Frames::new(Bars::from_ctx(&held)), kind, first, last))
}

/// Loads every other instrument a spec reads (spec §4.1): same engine only, gl never mixed with India/US.
fn others_for(st: &State, run: &str, symbols: &[String], fixups: &BTreeSet<String>, close: bool) -> Result<(Others, String), LabError> {
    let mut frames = BTreeMap::new();
    let mut items = Vec::new();
    for sym in symbols {
        if markets::gl(sym).is_some() != markets::gl(run).is_some() {
            let home = if markets::gl(sym).is_some() { "/api/gl" } else { "/api/in or /api/us" };
            return Err(LabError::NeedsData(format!("{sym} is served by {home}; one strategy reads one market's engine, and provisional gl data is never mixed into an India/US strategy")));
        }
        if let (Some(a), Some(b)) = (crate::preloaded_market(run).or_else(|| st.baskets.native_evidence(run).map(|e| e.market())), crate::preloaded_market(sym)) {
            if a != b { return Err(LabError::NeedsData(format!("{sym}: cross-market references are not available (this engine serves {})", a.code()))); }
        }
        let (fr, kind, first, last) = frames_of(st, sym).map_err(|e| match e {
            LabError::Invalid(m) if m.starts_with("unknown ticker") => LabError::Invalid(format!("{m} on this market (other instruments must be served by this market's engine)")),
            e => e,
        })?;
        let rule = foreign::alignment(run, sym, close);
        items.push(format!("{{\"ticker\":\"{}\",\"type\":\"{kind}\",\"alignment\":\"{}\",\"fixup\":{},\"first\":\"{first}\",\"last\":\"{last}\"}}", esc(sym), rule.name(), fixups.contains(sym)));
        frames.insert(sym.clone(), fr);
    }
    let o = Others { run: run.into(), close_fills: close, frames: std::sync::Arc::new(frames) };
    Ok((o, if items.is_empty() { String::new() } else { format!(",\"foreign\":[{}]", items.join(",")) }))
}
```
Add to `rules.rs` a helper used here (no test of its own; exercised by the API tests):
```rust
impl Spec {
    /// Instruments read with `fixup: previous` (reported in the API's `foreign` list).
    pub fn fixup_symbols(&self) -> std::collections::BTreeSet<String> { /* same leaf walk as foreign_symbols, keeping symbol where foreign.fixup */ }
}
```
written with the same iterator chain as `foreign_symbols`, filtered on `s.foreign.fixup` and mapping `s.foreign.symbol`.

In `run_json` and `optimize_json`, after `known_type(…)`:
```rust
    let (others, foreign_json) = others_for(st, &ctx.r[0].ticker, &spec.foreign_symbols(), &spec.fixup_symbols(), close_fills(&spec.exec))?;
```
replace `series::compile(&spec, &fr, &mut Cache::new())` with `series::compile_with(&spec, &fr, &others, &mut Cache::new())` (in `optimize_json`'s `metrics` closure: `series::compile_with(s, &fr, &others, cache)`), and in both final `format!` strings change `\"instrument\":{instrument},` to `\"instrument\":{instrument}{foreign_json},`. The unknown-ticker message: `select_error` already returns `invalid "unknown ticker X"`.

In `series_json`: build `let foreign = rules::Foreign { vs: q.get("vs").filter(|v| !v.is_empty()).cloned(), ..Default::default() };`, pass it to `series_ref`, load `let (others, _) = others_for(st, &ctx.r[0].ticker, &foreign.vs.iter().cloned().collect::<Vec<_>>(), &BTreeSet::new(), false)?;` and call `series::series_in(&fr, &others, &mut cache, …)`.

Phase 7: insert `{foreign_json}` after the `instrument` key wherever Phase 7 builds run/optimize bodies (including the money-mode body); Phase 7's execution model is still decided by the run instrument only (an index run stays a study).

- [ ] **Step 4: Run to verify pass** — `CARGO_BUILD_JOBS=2 cargo test lab:: 2>&1 | grep 'test result'` → +4, 0 failed.

- [ ] **Step 5: Commit**

```bash
git add Context/guru/backend/src/lab/api.rs Context/guru/backend/src/lab/rules.rs
git commit -m "Serve lab runs, optimizations and charts that read other instruments of the same market"
```

---

### Task 6: The `portfolio` object (`rules.rs`)

**Files:**
- Modify: `Context/guru/backend/src/lab/rules.rs` (types, `parse_spec`, optimize targets, `foreign_symbols`)
- Test: `rules.rs` tests module

**Interfaces:**
- Produces:
```rust
pub const MAX_UNIVERSE: usize = 120; pub const MAX_UNIVERSE_LIST: usize = 50; pub const MAX_OPEN: usize = 50;
#[derive(Clone, Copy, Debug, PartialEq, Eq)] pub enum Mode { Signals, Rotation }
#[derive(Clone, Debug, PartialEq)] pub enum Universe { Basket(String), Tickers(Vec<String>) }
/// None = rotate every bar; Some(anchor) = last bar of each period (resample::period_key anchor 1 W, 2 M, 3 Q, 4 Y).
pub type Rebalance = Option<usize>;
#[derive(Clone, Debug, PartialEq)]
pub struct Portfolio { pub universe: Universe, pub mode: Mode, pub score: Option<SeriesRef>, pub lowest: bool, pub max_open: usize, pub worst_rank_held: usize, pub rebalance: Rebalance }
```
`Spec.portfolio: Option<Portfolio>`; optimize paths `portfolio.max_open`, `portfolio.worst_rank_held`, `portfolio.score.p.<key>` (integers / catalogue bounds).

- [ ] **Step 1: Write the failing tests** — append:

```rust
    const PORT: &str = r#"{ "v": 1, "direction": "long",
      "entry": { "all": [ { "lhs": {"s":"rsi","p":{"length":2}}, "op":"<", "rhs": {"value":10} } ] },
      "exit": { "any": [ { "lhs": {"s":"bars_held"}, "op":">=", "rhs": {"value":5} } ] },
      "execution": { "capital": 1000000 },
      "portfolio": { "universe": {"basket":"nifty50"}, "score": {"s":"rsi","p":{"length":14}}, "rank": "lowest", "max_open": 5 } }"#;
    #[test] fn portfolio_specs_parse_with_amibroker_defaults() {
        let s = parse_spec(PORT).unwrap();
        let p = s.portfolio.as_ref().unwrap();
        assert_eq!((&p.universe, p.mode, p.lowest, p.max_open, p.worst_rank_held, p.rebalance), (&Universe::Basket("nifty50".into()), Mode::Signals, true, 5, 5, None));
        assert_eq!(p.score.as_ref().unwrap().params["length"], 14.);
        let rot = PORT.replace(r#""all": [ { "lhs": {"s":"rsi","p":{"length":2}}, "op":"<", "rhs": {"value":10} } ]"#, r#""all": []"#)
            .replace(r#""any": [ { "lhs": {"s":"bars_held"}, "op":">=", "rhs": {"value":5} } ]"#, r#""any": []"#)
            .replace(r#""max_open": 5"#, r#""max_open": 5, "mode": "rotation", "worst_rank_held": 8, "rebalance": "M""#);
        let r = parse_spec(&rot).unwrap();
        let p = r.portfolio.as_ref().unwrap();
        assert_eq!((p.mode, p.worst_rank_held, p.rebalance), (Mode::Rotation, 8, Some(2)));
        let t = parse_spec(&PORT.replace(r#"{"basket":"nifty50"}"#, r#"{"tickers":["basket:nifty50:TCS","basket:nifty50:INFY"]}"#)).unwrap();
        assert_eq!(t.portfolio.unwrap().universe, Universe::Tickers(vec!["basket:nifty50:TCS".into(), "basket:nifty50:INFY".into()]));
        let o = parse_spec(&PORT.replace(r#""max_open": 5 }"#, r#""max_open": 5 }, "optimize": [{"path":"portfolio.max_open","from":2,"to":6,"step":2},{"path":"portfolio.score.p.length","from":5,"to":20,"step":5}]"#)).unwrap();
        let a = apply(&o, &o.optimize, &[4., 10.]);
        assert_eq!((a.portfolio.as_ref().unwrap().max_open, a.portfolio.as_ref().unwrap().score.as_ref().unwrap().params["length"]), (4, 10.));
        assert_eq!((current(&o, &o.optimize[0].target), current(&o, &o.optimize[1].target)), (5., 14.));
    }
    #[test] fn portfolio_rules_are_bounded_and_name_the_field() {
        let list = |n: usize| format!(r#"{{"tickers":[{}]}}"#, (0..n).map(|i| format!("\"basket:nifty50:S{i}\"")).collect::<Vec<_>>().join(","));
        for (from, to, status, needle) in [
            (r#"{"basket":"nifty50"}"#, &list(51), "invalid", "at most 50"),
            (r#"{"basket":"nifty50"}"#, &list(1), "invalid", "2 to 50"),
            (r#"{"basket":"nifty50"}"#, r#"{"tickers":["A","A"]}"#, "invalid", "listed twice"),
            (r#"{"basket":"nifty50"}"#, r#"{"basket":"nifty 50!"}"#, "invalid", "portfolio.universe.basket"),
            (r#""max_open": 5"#, r#""max_open": 51"#, "invalid", "portfolio.max_open must be a whole number from 1 to 50"),
            (r#""max_open": 5"#, r#""max_open": 5, "worst_rank_held": 3"#, "invalid", "worst_rank_held applies to rotation"),
            (r#""rank": "lowest""#, r#""rank": "low""#, "invalid", "portfolio.rank must be"),
            (r#""direction": "long""#, r#""direction": "short""#, "unsupported", "portfolio shorts"),
            (r#""execution": { "capital": 1000000 }"#, r#""execution": { }"#, "invalid", "needs execution.capital"),
            (r#"{"s":"rsi","p":{"length":14}}"#, r#"{"s":"rsi","p":{"length":14},"symbol":"NIFTY 50"}"#, "invalid", "portfolio.score cannot read another symbol"),
            (r#""max_open": 5"#, r#""max_open": 5, "mode": "rotation""#, "invalid", "rotation ranks by score; exit.any must be empty"),
            (r#""all": [ {"#, r#""limit": {"pct": 2}, "all": [ {"#, "unsupported", "entry.limit in a portfolio"),
        ] {
            let text = PORT.replace(from, to);
            let e = err(&text);
            assert_eq!(e.status(), status, "{to}: {e:?}");
            assert!(e.message().contains(needle), "{needle:?} not in {:?}", e.message());
        }
        let rot_no_score = PORT.replace(r#""score": {"s":"rsi","p":{"length":14}}, "#, "").replace(r#""max_open": 5"#, r#""max_open": 5, "mode": "rotation""#)
            .replace(r#"{ "lhs": {"s":"bars_held"}, "op":">=", "rhs": {"value":5} }"#, "");
        assert!(err(&rot_no_score).message().contains("rotation needs portfolio.score"));
    }
```
(Phase 7 note: `execution.capital` is Phase 7's money-mode field; if Phase 7 names it differently, use its name in both the spec text and the check.)

- [ ] **Step 2: Run to verify failure** — `cargo test lab::rules` → compile errors (`portfolio` field, `Universe`, `Mode`).

- [ ] **Step 3: Implement** — (a) types and constants from "Interfaces" above. (b) `parse_spec`: move `"portfolio"` from the reserved list to the allowed list of `fields(&root, "spec", …)`; parse it **before** the entry list so rotation may leave `entry.all` empty: `let portfolio = root.get("portfolio").map(portfolio).transpose()?;` then `conds(entry_obj.get("all"), "entry.all", false, portfolio.as_ref().is_some_and(|p| p.mode == Mode::Rotation))`; build `Spec { …, portfolio }`; after the foreign-count check call `if spec.portfolio.is_some() { portfolio_rules(&spec)?; }`. (c) Parser:
```rust
fn portfolio(v: &Json) -> Result<Portfolio, LabError> {
    fields(v, "portfolio", &["universe", "mode", "score", "rank", "max_open", "worst_rank_held", "rebalance"], &[])?;
    let u = v.get("universe").ok_or_else(|| inv("portfolio.universe is required"))?;
    fields(u, "portfolio.universe", &["basket", "tickers"], &[])?;
    let universe = match (u.get("basket"), u.get("tickers")) {
        (Some(b), None) => Universe::Basket(b.as_str().filter(|s| !s.is_empty() && s.len() <= 40 && s.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-'))
            .ok_or_else(|| inv("portfolio.universe.basket must be a basket id (letters, digits, _ or -)"))?.into()),
        (None, Some(Json::Arr(items))) => {
            if !(2..=MAX_UNIVERSE_LIST).contains(&items.len()) { return Err(inv(format!("portfolio.universe.tickers needs 2 to {MAX_UNIVERSE_LIST} instruments (at most {MAX_UNIVERSE_LIST}); use a basket for more"))); }
            let mut out: Vec<String> = Vec::new();
            for (i, t) in items.iter().enumerate() {
                let t = t.as_str().filter(|s| !s.trim().is_empty() && s.len() <= MAX_SYMBOL_LEN && !s.chars().any(char::is_control))
                    .ok_or_else(|| inv(format!("portfolio.universe.tickers[{i}] must name an instrument")))?;
                if out.iter().any(|x| x == t) { return Err(inv(format!("portfolio.universe.tickers[{i}] '{t}' is listed twice"))); }
                out.push(t.into());
            }
            Universe::Tickers(out)
        }
        _ => return Err(inv("portfolio.universe needs exactly one of basket or tickers (a list)")),
    };
    let mode = match v.get("mode").map(Json::as_str) { None | Some(Some("signals")) => Mode::Signals, Some(Some("rotation")) => Mode::Rotation, _ => return Err(inv("portfolio.mode must be \"signals\" or \"rotation\"")) };
    let lowest = match v.get("rank").map(Json::as_str) { None | Some(Some("highest")) => false, Some(Some("lowest")) => true, _ => return Err(inv("portfolio.rank must be \"highest\" or \"lowest\"")) };
    let whole = |k: &str, lo: usize, hi: usize| -> Result<Option<usize>, LabError> {
        v.get(k).map(|x| x.as_f64().filter(|n| n.fract() == 0. && *n >= lo as f64 && *n <= hi as f64).map(|n| n as usize)
            .ok_or_else(|| inv(format!("portfolio.{k} must be a whole number from {lo} to {hi}")))).transpose()
    };
    let max_open = whole("max_open", 1, MAX_OPEN)?.unwrap_or(1);
    let worst = whole("worst_rank_held", 1, MAX_UNIVERSE)?;
    if worst.is_some() && mode != Mode::Rotation { return Err(inv("portfolio.worst_rank_held applies to rotation mode")); }
    let worst_rank_held = worst.unwrap_or(max_open);
    if worst_rank_held < max_open { return Err(inv("portfolio.worst_rank_held must be at least max_open")); }
    let rebalance = match v.get("rebalance").map(Json::as_str) {
        None | Some(Some("bar")) => None, Some(Some("W")) => Some(1), Some(Some("M")) => Some(2), Some(Some("Q")) => Some(3), Some(Some("Y")) => Some(4),
        _ => return Err(inv("portfolio.rebalance must be bar, W, M, Q or Y")),
    };
    if rebalance.is_some() && mode != Mode::Rotation { return Err(inv("portfolio.rebalance applies to rotation mode")); }
    let score = match v.get("score") {
        None => None,
        Some(_) => match operand(v.get("score"), "portfolio.score", false)? {
            Operand::Series(s) if s.foreign.symbol.is_some() => return Err(inv("portfolio.score cannot read another symbol (every member would get the same score); vs is allowed")),
            Operand::Series(s) => Some(s),
            _ => return Err(inv("portfolio.score must be a series (an indicator or price)")),
        },
    };
    if mode == Mode::Rotation && score.is_none() { return Err(inv("rotation needs portfolio.score (AmiBroker PositionScore)")); }
    Ok(Portfolio { universe, mode, score, lowest, max_open, worst_rank_held, rebalance })
}
/// What a portfolio cannot combine with (spec §6.1), each refusal naming the field.
fn portfolio_rules(s: &Spec) -> Result<(), LabError> {
    let p = s.portfolio.as_ref().unwrap();
    let un = |m: &str| Err(LabError::Unsupported(format!("{m} in a portfolio is a later version (Phase 8 spec §13)")));
    if s.exec.short { return Err(LabError::Unsupported("portfolio shorts are a later version (no short-margin model is declared)".into())); }
    if s.exec.limit.is_some() { return un("entry.limit"); }
    // Phase 7: also refuse entry.stop, scale_in, take_profits, cooldown, max_trades, risk, futures, delay_bars != 1 and direction both with `un`.
    if s.exec.capital.is_none() { return Err(inv("a portfolio needs execution.capital (money mode: shared cash and slots)")); }
    if p.mode == Mode::Rotation && (!s.exit.is_empty() || s.exec.stop_loss.is_some() || s.exec.target.is_some() || s.exec.trailing.is_some()) {
        return Err(inv("rotation ranks by score; exit.any must be empty and price exits are not used (exits are rank exits)"));
    }
    Ok(())
}
```
(`s.exec.capital` is Phase 7's money field.) (d) `foreign_symbols` also walks `portfolio.score` (its `vs`). (e) Optimize: `target()` accepts `portfolio.max_open`, `portfolio.worst_rank_held` (values must be whole and within the same bounds; `worst_rank_held` ≥ the combination's `max_open` is re-checked at run time by the API) and `portfolio.score.p.<key>` (catalogue bounds via `resolve_params`); `set`/`current` read and write them. Phase 7 replaces `target()` with "one generic walker": add these three path families to it with the same validation messages as above.

- [ ] **Step 4: Run to verify pass** — `cargo test lab::rules` → +2; the `reserved_fields_are_unsupported` row for `spec.portfolio` is **deleted** (it is no longer reserved); full `cargo test lab::` passes.

- [ ] **Step 5: Commit**

```bash
git add Context/guru/backend/src/lab/rules.rs
git commit -m "Accept the lab portfolio object: universe, signals or rotation, score, max open, worst rank, rebalance"
```

---

### Task 7: Universe members on the reference calendar (`lab/universe.rs`)

**Files:**
- Create: `Context/guru/backend/src/lab/universe.rs`; Modify: `mod.rs` (`pub mod universe;`)

**Interfaces:**
- Consumes: `rules::{Universe, MAX_UNIVERSE}`, `api::frames_of` (make it `pub(super)` in this task), `crate::{State, native_benchmark, preloaded_market}`, `baskets::Catalogue::{tickers_json, native_evidence}`.
- Produces:
```rust
pub struct Member { pub ticker: String, pub kind: &'static str, pub frames: Frames, pub at: Vec<Option<usize>> }
pub struct Loaded { pub reference: String, pub dates: Vec<String>, pub members: Vec<Member>, pub refused: Vec<(String, String)>, pub basket: Option<String>, pub membership: (&'static str, &'static str) }
pub fn basket_members(st: &State, basket: &str) -> Vec<(String, String)>   // (qualified ticker, membership_kind), sorted
pub fn calendar_map(reference: &[String], member: &[String]) -> Vec<Option<usize>>
pub fn membership_note(kind: &str) -> &'static str
pub fn load(st: &State, u: &Universe) -> Result<Loaded, LabError>
```

- [ ] **Step 1: Write the failing tests** — `universe.rs` test module. It builds a synthetic basket directory exactly like `baskets.rs`'s own fixture (write `index.csv` + one CSV per member into `std::env::temp_dir()` under a unique name, removed on drop; `sha256sum` is available in the engine image and on the gate machine):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::lab::rules::Universe;
    struct Dir(std::path::PathBuf);
    impl Drop for Dir { fn drop(&mut self) { let _ = std::fs::remove_dir_all(&self.0); } }
    /// A two-basket India catalogue: nifty50 (A, B) and nse500 (130 members; legacy India baskets need no typed columns), plus a NIFTY 50 reference index.
    pub(crate) fn india(name: &str) -> (Dir, State) {
        let dir = std::env::temp_dir().join(format!("vault-lab-universe-{}-{name}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let sha = |b: &[u8]| { use std::io::Write; let mut c = std::process::Command::new("sha256sum").stdin(std::process::Stdio::piped()).stdout(std::process::Stdio::piped()).spawn().unwrap();
            c.stdin.take().unwrap().write_all(b).unwrap(); String::from_utf8(c.wait_with_output().unwrap().stdout).unwrap()[..64].to_string() };
        let mut index = String::from("basket,ticker,path,first,last,bars,source,source_sha256,csv_sha256,membership,status,has_volume\n");
        let mut add = |basket: &str, sym: &str, rows: &str, first: &str, last: &str, n: usize| {
            let file = format!("{basket}_{sym}.csv");
            std::fs::write(dir.join(&file), rows).unwrap();
            index.push_str(&format!("{basket},{sym},{file},{first},{last},{n},Test,{},{},Current snapshot,ready,true\n", "a".repeat(64), sha(rows.as_bytes())));
        };
        let csv = |skip: usize, bad: bool| { let mut s = String::from("date,open,high,low,close,volume\n");
            for i in 0..60usize { if i == skip { continue; } let c = 100. + i as f64; let o = if bad && i == 40 { c + 5. } else { c }; s.push_str(&format!("2024-{:02}-{:02},{o},{},{},{c},1000\n", 1 + i / 28, 1 + i % 28, c + 2., c - 2.)); } s };
        add("nifty50", "A", &csv(99, false), "2024-01-01", "2024-03-04", 60);
        add("nifty50", "B", &csv(10, false), "2024-01-01", "2024-03-04", 59);
        for k in 0..130 { add("nse500", &format!("S{k:03}"), &csv(99, false), "2024-01-01", "2024-03-04", 60); }
        std::fs::write(dir.join("index.csv"), index).unwrap();
        let rows: &'static [crate::Row] = Box::leak((0..60usize).map(|i| crate::Row { ticker: "NIFTY 50".into(), date: format!("2024-{:02}-{:02}", 1 + i / 28, 1 + i % 28), open: 1., high: 2., low: 0.5, close: 1.5, volume: None }).collect::<Vec<_>>().into_boxed_slice());
        let st = State { ctxs: [("NIFTY 50".to_string(), crate::connors::Ctx::new(rows, rows, &[]))].into(), baskets: crate::baskets::Catalogue::load(&dir).unwrap(), vix: vec![], market_extra: Default::default() };
        (Dir(dir), st)
    }
    #[test] fn a_basket_expands_to_its_members_on_the_reference_calendar() {
        let (_d, st) = india("expand");
        assert_eq!(basket_members(&st, "nifty50").iter().map(|m| m.0.as_str()).collect::<Vec<_>>(), ["basket:nifty50:A", "basket:nifty50:B"]);
        let l = load(&st, &Universe::Basket("nifty50".into())).unwrap();
        assert_eq!((l.reference.as_str(), l.dates.len(), l.members.len(), l.basket.as_deref()), ("NIFTY 50", 60, 2, Some("nifty50")));
        let b = &l.members[1];
        assert_eq!((b.at[9], b.at[10], b.at[11]), (Some(9), None, Some(10)), "a member's missing day maps to None, later days shift");
        assert_eq!(l.membership.0, "current_snapshot");
        assert!(membership_note("current_snapshot").contains("biases results upward"));
    }
    #[test] fn universes_are_bounded_and_typed() {
        let (_d, st) = india("bounds");
        let e = load(&st, &Universe::Basket("nse500".into())).err().unwrap();
        assert_eq!(e.status(), "invalid");
        assert!(e.message().contains("nse500 has 130 members; the portfolio limit is 120"));
        let e = load(&st, &Universe::Tickers(vec!["basket:nifty50:A".into(), "NIFTY 50".into()])).err().unwrap();
        assert!(e.message().contains("NIFTY 50 (index) cannot be a portfolio member"), "{}", e.message());
        assert_eq!(load(&st, &Universe::Basket("nope".into())).err().unwrap().message(), "portfolio.universe.basket: no basket 'nope' on this engine");
        let e = load(&st, &Universe::Tickers(vec!["basket:nifty50:A".into(), "IG:DAX".into()])).err().unwrap();
        assert_eq!(e.status(), "needs_data");
        let l = load(&st, &Universe::Tickers(vec!["basket:nifty50:B".into(), "basket:nifty50:A".into()])).unwrap();
        assert_eq!(l.members.iter().map(|m| m.ticker.as_str()).collect::<Vec<_>>(), ["basket:nifty50:A", "basket:nifty50:B"], "members are sorted by ticker");
        assert_eq!(l.membership.0, "user_list");
    }
    #[test] fn calendar_map_never_points_to_a_different_date() {
        let r: Vec<String> = ["2024-01-01", "2024-01-02", "2024-01-03"].map(String::from).to_vec();
        let m: Vec<String> = ["2024-01-02", "2024-01-03", "2024-01-04"].map(String::from).to_vec();
        assert_eq!(calendar_map(&r, &m), vec![None, Some(0), Some(1)]);
    }
}
```

- [ ] **Step 2: Run to verify failure** — `cargo test lab::universe` → compile errors.

- [ ] **Step 3: Implement** — above the tests:

```rust
//! Portfolio universes (Phase 8 spec §5): basket members or a list of admitted stocks/ETFs, each mapped onto the
//! market's reference calendar (NIFTY 50 / SPX). Members are read through the book loaders, never edited.
use super::api::frames_of;
use super::json::{self, Json};
use super::markets;
use super::rules::{LabError, Universe, MAX_UNIVERSE};
use super::series::Frames;
use crate::State;

pub struct Member { pub ticker: String, pub kind: &'static str, pub frames: Frames, pub at: Vec<Option<usize>> }
pub struct Loaded { pub reference: String, pub dates: Vec<String>, pub members: Vec<Member>, pub refused: Vec<(String, String)>, pub basket: Option<String>, pub membership: (&'static str, &'static str) }

/// Qualified members of one basket with their membership kind, sorted (read from the public whitelist JSON).
pub fn basket_members(st: &State, basket: &str) -> Vec<(String, String)> {
    // shortcut: rebuilds the list from tickers_json() per request (~1 KB per whitelist row); cache it if profiles show it matters.
    let mut out: Vec<(String, String)> = st.baskets.tickers_json().iter().filter_map(|j| json::parse(j).ok()).filter(|v| v.get("basket").and_then(Json::as_str) == Some(basket))
        .filter_map(|v| Some((v.get("ticker")?.as_str()?.to_string(), v.get("membership_kind").and_then(Json::as_str).unwrap_or("current_snapshot").to_string()))).collect();
    out.sort();
    out
}
pub fn calendar_map(reference: &[String], member: &[String]) -> Vec<Option<usize>> {
    let mut k = 0;
    reference.iter().map(|d| { while k < member.len() && member[k] < *d { k += 1; } (k < member.len() && member[k] == *d).then_some(k) }).collect()
}
pub fn membership_note(kind: &str) -> &'static str {
    match kind {
        "current_plus_past_snapshot" => "Members include past constituents but without the dates they were in the index, so a stock can be traded years before it joined.",
        "user_list" => "A list chosen today carries hindsight selection bias.",
        _ => "Members are today's constituents tested over past dates: stocks that left the index (often after falling) are missing, which biases results upward.",
    }
}
pub fn load(st: &State, u: &Universe) -> Result<Loaded, LabError> {
    let (tickers, basket, kind): (Vec<String>, Option<String>, &'static str) = match u {
        Universe::Basket(b) => {
            let m = basket_members(st, b);
            if m.is_empty() { return Err(LabError::Invalid(format!("portfolio.universe.basket: no basket '{b}' on this engine"))); }
            if m.len() > MAX_UNIVERSE { return Err(LabError::Invalid(format!("{b} has {} members; the portfolio limit is {MAX_UNIVERSE} — choose a smaller basket or a list of at most 50 members", m.len()))); }
            let kind = if m[0].1 == "current_plus_past_snapshot" { "current_plus_past_snapshot" } else { "current_snapshot" };
            (m.into_iter().map(|x| x.0).collect(), Some(b.clone()), kind)
        }
        Universe::Tickers(t) => { let mut t = t.clone(); t.sort(); (t, None, "user_list") }
    };
    let mut market = None;
    for t in &tickers {
        if markets::gl(t).is_some() { return Err(LabError::NeedsData(format!("{t} is provisional gl data served by /api/gl; portfolios run on India/US stocks and ETFs"))); }
        let m = st.baskets.native_evidence(t).map(|e| e.market()).or_else(|| crate::preloaded_market(t));
        let kind = crate::instrument_policy::selected_type(t, st.baskets.native_evidence(t).is_some());
        if !matches!(kind, "individual_stock" | "etf") {
            return Err(LabError::Invalid(format!("{t} ({kind}) cannot be a portfolio member; members are actual stocks or ETFs with their own prices (an index can be a signal through symbol)")));
        }
        if market.is_some() && m != market { return Err(LabError::NeedsData(format!("{t}: cross-market universes are not available"))); }
        market = m;
    }
    let market = market.ok_or_else(|| LabError::Invalid("portfolio.universe: no member of this engine's market".into()))?;
    let reference = crate::native_benchmark(market).to_string();
    let rctx = st.ctxs.get(&reference).ok_or_else(|| LabError::NeedsData(format!("{reference} (the portfolio's reference calendar) is not loaded on this engine")))?;
    let dates: Vec<String> = rctx.r.iter().map(|r| r.date.clone()).collect();
    let mut members = Vec::new();
    for t in tickers {
        // one member at a time: rows become lab Bars and are dropped (bounded peak memory, spec §6.10)
        let (frames, kind, _, _) = frames_of(st, &t)?;
        let at = calendar_map(&dates, &frames.d.date);
        members.push(Member { ticker: t, kind, frames, at });
    }
    Ok(Loaded { reference, dates, members, refused: vec![], basket, membership: (kind, membership_note(kind)) })
}
```
(Window admission — no bar in the window, an invalid bar in the window, missing volume — is applied per member by the API in Task 12, which fills `refused`.) Make `api::frames_of` `pub(super)`.

- [ ] **Step 4: Run to verify pass** — `cargo test lab::universe` → 3 passed.

- [ ] **Step 5: Commit**

```bash
git add Context/guru/backend/src/lab/universe.rs Context/guru/backend/src/lab/mod.rs Context/guru/backend/src/lab/api.rs
git commit -m "Load lab portfolio universes from baskets or lists onto the reference calendar"
```

---

### Task 8: Shared-ledger simulator, signals mode (`lab/portfolio.rs`)

**Files:**
- Create: `Context/guru/backend/src/lab/portfolio.rs`; Modify: `mod.rs` (`pub mod portfolio;`)

**Interfaces:**
- Consumes: `series::{Compiled, RCond}` (Phase 7: its compiled node lists and `holds`-equivalent), `rules::{Mode, Fill}`, `universe::Member`.
- Produces:
```rust
pub struct Lane<'a> { pub m: &'a Member, pub c: Compiled, pub score: Option<Vec<f64>> }   // score on the member's own bars
pub enum Size { PctEquity(f64), Value(f64), Units(f64), RiskPct(f64) }
pub struct Charges { pub per_order: f64, pub per_unit: f64, pub buy_pct: f64, pub sell_pct: f64, pub slip_bps: f64 }
pub struct Book { pub capital: f64, pub mode: Mode, pub max_open: usize, pub worst_rank_held: usize, pub rebalance: Option<usize>, pub lowest: bool,
                  pub fill: Fill, pub exit_fill: Fill, pub size: Size, pub charges: Charges }
pub struct PTrade { pub lane: usize, pub signal_i: usize, pub entry_i: usize, pub exit_i: usize, pub entry_px: f64, pub exit_px: f64, pub qty: f64,
                    pub entry_cost: f64, pub exit_cost: f64, pub equity_at_signal: f64, pub rank: usize, pub score: f64, pub reason: String, pub open: bool, pub kind: &'static str }
#[derive(Default, Debug, PartialEq)] pub struct Skipped { pub no_slot: usize, pub no_cash: usize, pub no_bar: usize }
pub struct Ledger { pub equity: Vec<f64>, pub cash: Vec<f64>, pub invested: Vec<f64>, pub positions: Vec<usize>, pub trades: Vec<PTrade>, pub skipped: Skipped, pub events: Vec<String> }
pub fn run(lanes: &[Lane], b: &Book, dates: &[String], from: usize, to: usize) -> Ledger
```
Indices `signal_i/entry_i/exit_i` and the `Ledger` vectors are on the **reference calendar** (`Ledger` vectors cover `from..=to`).

- [ ] **Step 1: Write the failing tests** — `portfolio.rs` test module with a fixture builder used by Tasks 8–14:

```rust
#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::lab::rules::Op;
    use crate::lab::series::{Frames, RCond, Val};
    use crate::lab::testdata::from_ohlc;
    use crate::lab::universe::Member;
    pub(crate) const N: f64 = f64::NAN;
    pub(crate) fn dates(n: usize) -> Vec<String> { (0..n).map(|i| crate::lab::resample::civil(crate::lab::resample::day_number("2024-01-01") + i as i64)).collect() }
    /// A member whose open = base + i and close = base + i + 0.5 (fills identifiable by price), present on every date unless in `gaps`.
    pub(crate) fn member(t: &str, base: f64, n: usize, gaps: &[usize]) -> Member {
        let all = dates(n);
        let keep: Vec<usize> = (0..n).filter(|i| !gaps.contains(i)).collect();
        let mut b = from_ohlc(&keep.iter().map(|&i| { let o = base + i as f64; (o, o + 1., o - 1., o + 0.5) }).collect::<Vec<_>>());
        b.date = keep.iter().map(|&i| all[i].clone()).collect();
        let at = crate::lab::universe::calendar_map(&all, &b.date);
        Member { ticker: t.into(), kind: "individual_stock", frames: Frames::new(b), at }
    }
    /// Entry true on reference bars `on`, exit true on `off` (both on the reference calendar, mapped to the member's bars).
    pub(crate) fn lane<'a>(m: &'a Member, on: &[usize], off: &[usize], score: Option<Vec<f64>>) -> Lane<'a> {
        let sig = |bars: &[usize]| { let mut v = vec![0.; m.frames.d.len()]; for &i in bars { if let Some(k) = m.at[i] { v[k] = 1.; } } v };
        let rule = |v: Vec<f64>| RCond { lhs: Val::Arr(v), op: Op::Gt, rhs: Val::Const(0.5), label: "rule".into() };
        Lane { m, c: Compiled::new(vec![rule(sig(on))], vec![rule(sig(off))]), score }
    }
    pub(crate) fn book(max_open: usize, size: Size) -> Book {
        Book { capital: 10_000., mode: Mode::Signals, max_open, worst_rank_held: max_open, rebalance: None, lowest: false, fill: Fill::NextOpen, exit_fill: Fill::NextOpen,
               size, charges: Charges { per_order: 0., per_unit: 0., buy_pct: 0., sell_pct: 0., slip_bps: 0. } }
    }
    /// Spec §6.4 invariants on every bar.
    pub(crate) fn invariants(l: &Ledger, lanes: &[Lane], b: &Book, from: usize) {
        for (k, e) in l.equity.iter().enumerate() {
            assert!((e - (l.cash[k] + l.invested[k])).abs() <= 1e-9 * e.abs().max(1.), "bar {}: equity {e} != cash {} + invested {}", from + k, l.cash[k], l.invested[k]);
            assert!(l.cash[k] >= -1e-9 && l.invested[k] <= e + 1e-9 && l.positions[k] <= b.max_open, "bar {}", from + k);
        }
        for t in &l.trades {
            assert_eq!(t.qty.fract(), 0., "whole shares");
            let d = &lanes[t.lane].m.frames.d;
            for (i, px) in [(t.entry_i, t.entry_px), (t.exit_i, t.exit_px)] {
                if let Some(k) = lanes[t.lane].m.at[i] { assert!(px >= d.low[k] - 1e-9 && px <= d.high[k] + 1e-9, "fill {px} outside bar {i}"); }
            }
        }
    }

    #[test] fn one_member_fills_like_the_single_instrument_simulator() {
        let a = member("A", 100., 10, &[]);
        let lanes = [lane(&a, &[2], &[5], None)];
        let b = book(1, Size::PctEquity(50.));
        let l = run(&lanes, &b, &dates(10), 0, 9);
        let t = &l.trades[0];
        // signal close 2 → open of 3 (103); exit signal 5 → open of 6 (106); 50% of 10,000 at the signal close 102.5 → 48 shares
        assert_eq!((t.entry_i, t.entry_px, t.exit_i, t.exit_px, t.qty, t.open), (3, 103., 6, 106., 48., false));
        assert_eq!(l.equity[9], 10_000. + 48. * 3.);
        invariants(&l, &lanes, &b, 0);
    }
    #[test] fn slots_and_cash_are_shared_and_exits_free_them_on_the_same_bar() {
        let (a, bm, c) = (member("A", 100., 12, &[]), member("B", 200., 12, &[]), member("C", 300., 12, &[]));
        // A and B enter at bar 2's signal; C signals at 4 while both are held (no slot); A exits at 5's signal, C signals again at 5
        let lanes = [lane(&a, &[2], &[5], None), lane(&bm, &[2], &[], None), lane(&c, &[4, 5], &[], None)];
        let b = book(2, Size::PctEquity(50.));
        let l = run(&lanes, &b, &dates(12), 0, 11);
        let entries: Vec<(usize, usize)> = l.trades.iter().map(|t| (t.lane, t.entry_i)).collect();
        assert_eq!(entries, vec![(0, 3), (1, 3), (2, 6)], "C fills at bar 6's open, after A's exit at the same open");
        assert_eq!(l.skipped.no_slot, 1);
        invariants(&l, &lanes, &b, 0);
    }
    #[test] fn close_fills_rank_and_fill_at_the_signal_close_without_same_bar_reentry() {
        let a = member("A", 100., 10, &[]);
        let lanes = [lane(&a, &[2, 4], &[4], None)];
        let b = Book { fill: Fill::Close, exit_fill: Fill::Close, ..book(1, Size::PctEquity(100.)) };
        let l = run(&lanes, &b, &dates(10), 0, 9);
        assert_eq!(l.trades.iter().map(|t| (t.entry_i, t.entry_px, t.exit_i)).collect::<Vec<_>>(), vec![(2, 102.5, 4)], "no re-entry at the close that exited");
        invariants(&l, &lanes, &b, 0);
    }
    #[test] fn shrinking_to_cash_and_costs_keep_the_ledger_exact() {
        let (a, bm) = (member("A", 100., 10, &[]), member("B", 100., 10, &[]));
        let lanes = [lane(&a, &[2], &[], None), lane(&bm, &[2], &[], None)];
        let b = Book { charges: Charges { per_order: 20., per_unit: 0., buy_pct: 0.1, sell_pct: 0.1, slip_bps: 0. }, ..book(2, Size::PctEquity(60.)) };
        let l = run(&lanes, &b, &dates(10), 0, 9);
        // 60% each requested; the second shrinks to the remaining cash after the first fill and its costs
        let (q0, q1) = (l.trades[0].qty, l.trades[1].qty);
        assert_eq!(q0, (10_000. * 0.6 / 102.5_f64).floor());
        assert!(q1 < q0 && l.cash[1..].iter().all(|c| *c >= 0.));
        invariants(&l, &lanes, &b, 0);
    }
}
```

- [ ] **Step 2: Run to verify failure** — `cargo test lab::portfolio` → compile errors.

- [ ] **Step 3: Implement** — above the tests:

```rust
//! Portfolio backtest with one shared money ledger (Phase 8 spec §6; AmiBroker portfolio backtester semantics).
//! Event order on reference bar i: open (pending exits, then pending entries in rank order), inside the bar (price
//! exits, Task 11), close (data-end exits, rule or rank exits, ranked entries), then the equity mark.
use super::rules::{Fill, Mode};
use super::series::Compiled;
use super::universe::Member;

/* the Interfaces types, verbatim */

struct Held { qty: f64, entry_k: usize, entry_i: usize, entry_px: f64, entry_cost: f64, signal_i: usize, eq: f64, rank: usize, score: f64, kind: &'static str }
struct Cand { lane: usize, key: f64, want: f64, score: f64, signal_i: usize, eq: f64, rank: usize }

impl Charges {
    fn cost(&self, qty: f64, px: f64, buy: bool) -> f64 { if qty <= 0. { 0. } else { self.per_order + self.per_unit * qty + qty * px * (if buy { self.buy_pct } else { self.sell_pct }) / 100. } }
    fn slip(&self, px: f64, buy: bool) -> f64 { px * (1. + if buy { 1. } else { -1. } * self.slip_bps / 1e4) }
}
impl Size {
    /// Requested quantity at the signal close (Phase 7: equity and price at the signal close). Risk sizing needs the stop distance.
    fn qty(&self, equity: f64, px: f64, stop_dist: f64) -> f64 {
        let q = match *self { Size::PctEquity(p) => equity * p / 100. / px, Size::Value(v) => v / px, Size::Units(n) => n, Size::RiskPct(r) => equity * r / 100. / stop_dist };
        if q.is_finite() { q.floor().max(0.) } else { 0. }
    }
}

pub fn run(lanes: &[Lane], b: &Book, dates: &[String], from: usize, to: usize) -> Ledger {
    let n = to - from + 1;
    let mut l = Ledger { equity: vec![0.; n], cash: vec![0.; n], invested: vec![0.; n], positions: vec![0; n], trades: vec![], skipped: Skipped::default(), events: vec![] };
    let mut cash = b.capital;
    let mut held: Vec<Option<Held>> = lanes.iter().map(|_| None).collect();
    let mut mark = vec![f64::NAN; lanes.len()];          // last known close per member
    let mut pend_exit: Vec<Option<(usize, String)>> = lanes.iter().map(|_| None).collect();
    let mut pend_entry: Vec<Cand> = vec![];
    let mut equity = b.capital;
    for i in from..=to {
        let mut exited_after_open = vec![false; lanes.len()];
        // 1. open: exits first, then entries in rank order (cash and slots freed above are usable)
        for (m, ln) in lanes.iter().enumerate() {
            let Some(k) = ln.m.at[i] else { continue };
            if let Some((sig, why)) = pend_exit[m].take() {
                let h = held[m].take().unwrap();
                cash += close_out(&mut l, b, m, h, i, ln.m.frames.d.open[k], sig, why);
            }
        }
        for c in std::mem::take(&mut pend_entry) {
            let Some(k) = lanes[c.lane].m.at[i] else { l.skipped.no_bar += 1; continue };
            if held[c.lane].is_some() { continue; }
            if held.iter().filter(|h| h.is_some()).count() >= b.max_open { l.skipped.no_slot += 1; continue; }
            open_pos(&mut l, b, &mut held, &mut cash, &c, i, k, lanes[c.lane].m.frames.d.open[k], "next_open");
        }
        // 2. inside the bar: price exits (Task 11)
        // 3. close
        for (m, ln) in lanes.iter().enumerate() {
            if let Some(k) = ln.m.at[i] { mark[m] = ln.m.frames.d.close[k]; }
        }
        let last = i == to;
        for (m, ln) in lanes.iter().enumerate() {
            let (Some(k), Some(h)) = (ln.m.at[i], held[m].as_ref()) else { continue };
            if pend_exit[m].is_some() || b.mode != Mode::Signals { continue; }
            if let Some(r) = ln.c.exit.iter().find(|r| r.holds(k, Some(h.entry_k))) {
                match b.exit_fill {
                    Fill::Close => { let h = held[m].take().unwrap(); cash += close_out(&mut l, b, m, h, i, ln.m.frames.d.close[k], i, r.label.clone()); exited_after_open[m] = true; }
                    Fill::NextOpen if !last => pend_exit[m] = Some((i, r.label.clone())),
                    Fill::NextOpen => {}
                }
            }
        }
        let invested: f64 = held.iter().enumerate().filter_map(|(m, h)| h.as_ref().map(|h| h.qty * mark[m])).sum();
        equity = cash + invested;
        if !last && b.mode == Mode::Signals {
            let mut cands: Vec<Cand> = vec![];
            for (m, ln) in lanes.iter().enumerate() {
                let Some(k) = ln.m.at[i] else { continue };
                let flat = held[m].is_none() || (b.fill == Fill::NextOpen && pend_exit[m].is_some());
                if !flat || exited_after_open[m] || !ln.c.entry.iter().all(|r| r.holds(k, None)) { continue; }
                let score = ln.score.as_ref().map_or(0., |s| s[k]);
                if score.is_nan() { continue; }
                let px = ln.m.frames.d.close[k];
                let want = b.size.qty(equity, px, f64::NAN /* Task 11: stop distance */);
                cands.push(Cand { lane: m, key: if b.lowest { -score } else { score }, want, score, signal_i: i, eq: equity, rank: 0 });
            }
            rank(&mut cands, lanes, b);
            let free = (b.max_open + pend_exit.iter().filter(|p| p.is_some()).count()).saturating_sub(held.iter().filter(|h| h.is_some()).count());
            match b.fill {
                Fill::Close => {
                    let mut used = 0;
                    for c in cands {
                        if used >= free { l.skipped.no_slot += 1; continue; }
                        let k = lanes[c.lane].m.at[i].unwrap();
                        if open_pos(&mut l, b, &mut held, &mut cash, &c, i, k, lanes[c.lane].m.frames.d.close[k], "close") { used += 1; }
                    }
                }
                Fill::NextOpen => pend_entry = cands, // slots and cash are checked at the fill; a full book counts no_slot there
            }
        }
        let invested: f64 = held.iter().enumerate().filter_map(|(m, h)| h.as_ref().map(|h| h.qty * mark[m])).sum();
        let k = i - from;
        (l.cash[k], l.invested[k], l.equity[k], l.positions[k]) = (cash, invested, cash + invested, held.iter().filter(|h| h.is_some()).count());
    }
    for (m, h) in held.iter_mut().enumerate() {
        if let Some(h) = h.take() { let px = mark[m]; l.trades.push(trade(m, h, to, px, 0., "open".into(), true)); }
    }
    l
}
/// Ranking (spec §6.3): effective score, then requested size, then ticker — the AmiBroker KB tie order.
fn rank(c: &mut [Cand], lanes: &[Lane], _b: &Book) {
    c.sort_by(|x, y| y.key.total_cmp(&x.key).then(y.want.total_cmp(&x.want)).then(lanes[x.lane].m.ticker.cmp(&lanes[y.lane].m.ticker)));
}
fn open_pos(l: &mut Ledger, b: &Book, held: &mut [Option<Held>], cash: &mut f64, c: &Cand, i: usize, k: usize, raw: f64, kind: &'static str) -> bool {
    let px = b.charges.slip(raw, true);
    let mut qty = c.want;
    // shrink to the cash left after this bar's earlier fills and their costs (AmiBroker position shrinking; Phase 7 rule)
    while qty > 0. && qty * px + b.charges.cost(qty, px, true) > *cash + 1e-9 {
        qty = ((*cash - b.charges.per_order) / (px * (1. + b.charges.buy_pct / 100.) + b.charges.per_unit)).floor().min(qty - 1.).max(0.);
    }
    if qty < 1. { l.skipped.no_cash += 1; return false; }
    let cost = b.charges.cost(qty, px, true);
    *cash -= qty * px + cost;
    held[c.lane] = Some(Held { qty, entry_k: k, entry_i: i, entry_px: px, entry_cost: cost, signal_i: c.signal_i, eq: c.eq, rank: c.rank, score: c.score, kind });
    true
}
fn close_out(l: &mut Ledger, b: &Book, m: usize, h: Held, i: usize, raw: f64, signal_i: usize, why: String) -> f64 {
    let px = b.charges.slip(raw, false);
    let cost = b.charges.cost(h.qty, px, false);
    let back = h.qty * px - cost;
    let mut t = trade(m, h, i, px, cost, why, false);
    t.signal_i = signal_i;
    l.trades.push(t);
    back
}
fn trade(lane: usize, h: Held, exit_i: usize, exit_px: f64, exit_cost: f64, reason: String, open: bool) -> PTrade {
    PTrade { lane, signal_i: h.signal_i, entry_i: h.entry_i, exit_i, entry_px: h.entry_px, exit_px, qty: h.qty, entry_cost: h.entry_cost, exit_cost,
             equity_at_signal: h.eq, rank: h.rank, score: h.score, reason, open, kind: h.kind }
}
```
Record each entry's rank (1-based position in `cands`) into `Held.rank` (pass it in `Cand`). Phase 7: `r.holds(k, Some(entry_k))` is the Customize call; use Phase 7's top-level evaluator for node lists (`entry_on: edge` = require that the entry list was not true at the member's previous bar).

- [ ] **Step 4: Run to verify pass** — `cargo test lab::portfolio` → 4 passed.

- [ ] **Step 5: Commit**

```bash
git add Context/guru/backend/src/lab/portfolio.rs Context/guru/backend/src/lab/mod.rs
git commit -m "Add the lab portfolio ledger: shared cash and slots, exits before entries, money sizing"
```

---

### Task 9: Ranking, ties and sizing modes; AmiBroker fixtures P1 and P2 (`portfolio.rs`)

**Files:** Modify: `Context/guru/backend/src/lab/portfolio.rs` (tests; `rank`, `Size::RiskPct` plumbing)

**Interfaces:** Consumes Task 8. Produces: `Cand.want` = the **requested** size in the sizing method's own measure, before rounding (AmiBroker KB: "$ value is compared" for spsValue, "number of shares" for spsShares, "% equity" for spsPercentOfEquity): `PctEquity(p)` → p, `Value(v)` → v, `Units(n)` → n, `RiskPct(r)` → equity × r/100 ÷ stop distance × signal close; `PTrade.rank` = 1-based rank among that bar's candidates. `want` is a tie-break only; the filled quantity still comes from `Size::qty`.

- [ ] **Step 1: Write the failing tests** — append to the test module:

```rust
    #[test] fn p1_lower_rsi_is_preferred_with_25_percent_of_equity_each() {
        // AmiBroker h_portfolio.html: PositionScore = 50 - RSI(14) ("the stock with lower RSI will be preferred"); PositionSize = -25.
        let ms: Vec<Member> = ["A", "B", "C", "D"].iter().enumerate().map(|(j, t)| member(t, 100. + 50. * j as f64, 10, &[])).collect();
        let rsi = [40., 20., 35., 60.]; // B then C are the two lowest
        let lanes: Vec<Lane> = ms.iter().zip(rsi).map(|(m, r)| lane(m, &[2], &[], Some(vec![r; 10]))).collect();
        let b = Book { lowest: true, ..book(2, Size::PctEquity(25.)) };
        let l = run(&lanes, &b, &dates(10), 0, 9);
        assert_eq!(l.trades.iter().map(|t| lanes[t.lane].m.ticker.as_str()).collect::<Vec<_>>(), ["B", "C"]);
        assert_eq!(l.trades.iter().map(|t| t.rank).collect::<Vec<_>>(), [1, 2]);
        for t in &l.trades { assert_eq!(t.qty, (10_000. * 0.25 / (lanes[t.lane].m.frames.d.close[2])).floor()); }
        assert_eq!(l.skipped.no_slot, 2);
        invariants(&l, &lanes, &b, 0);
    }
    #[test] fn ties_prefer_larger_size_then_alphabetical_order() {
        // AmiBroker KB 2015-09-28: equal PositionScore → greater PositionSize, then alphabetical order.
        let ms: Vec<Member> = ["CCC", "AAA", "BBB"].iter().map(|t| member(t, 100., 10, &[])).collect();
        let lanes: Vec<Lane> = ms.iter().map(|m| lane(m, &[2], &[], Some(vec![1.; 10]))).collect();
        let l = run(&lanes, &book(2, Size::PctEquity(20.)), &dates(10), 0, 9);
        assert_eq!(l.trades.iter().map(|t| lanes[t.lane].m.ticker.as_str()).collect::<Vec<_>>(), ["AAA", "BBB"]);
        // with value sizing equal too, a cheaper member gets more shares but the same value: still alphabetical
        let cheap = [member("ZZZ", 10., 10, &[]), member("AAA", 100., 10, &[])];
        let lanes2: Vec<Lane> = cheap.iter().map(|m| lane(m, &[2], &[], Some(vec![1.; 10]))).collect();
        let l2 = run(&lanes2, &book(1, Size::Value(2_000.)), &dates(10), 0, 9);
        assert_eq!(lanes2[l2.trades[0].lane].m.ticker, "AAA");
        // units: the larger share count wins before the name
        let l3 = run(&lanes2, &Book { size: Size::Units(5.), ..book(1, Size::Units(5.)) }, &dates(10), 0, 9);
        assert_eq!(lanes2[l3.trades[0].lane].m.ticker, "AAA", "equal units → alphabetical");
    }
    #[test] fn risk_sizing_uses_the_stop_distance_and_needs_a_stop() {
        let a = member("A", 100., 10, &[]);
        let mut ln = lane(&a, &[2], &[], None);
        ln.c.exec.stop_loss = Some(crate::lab::rules::Dist::Pct(5.));
        let l = run(&[ln], &book(1, Size::RiskPct(1.)), &dates(10), 0, 9);
        // 1% of 10,000 = 100 at risk; stop distance 5% of the signal close 102.5 = 5.125 → 19 shares
        assert_eq!(l.trades[0].qty, (100. / 5.125_f64).floor());
    }
```

- [ ] **Step 2: Run to verify failure** — the risk test fails (stop distance NaN → 0 shares) and `rank` is 0.

- [ ] **Step 3: Implement** — in the candidate loop compute the stop distance at the signal close with the lane's compiled exec: `let stop = ln.c.exec.stop_loss.map_or(f64::NAN, |d| ln.c.dist(d, px, k));` and pass it to `b.size.qty(equity, px, stop)`; set `want` with a new `fn requested(&self, equity: f64, px: f64, stop: f64) -> f64` on `Size` (the rule above); after `rank(&mut cands, …)` set each `Cand.rank = position + 1` and copy it into `Held.rank`. (Phase 7: `Compiled::dist` may move; use Phase 7's distance helper for a `Dist` at bar k.)

- [ ] **Step 4: Run to verify pass** — `cargo test lab::portfolio` → 7 passed.

- [ ] **Step 5: Commit**

```bash
git add Context/guru/backend/src/lab/portfolio.rs
git commit -m "Rank lab portfolio entries by score, size and name; risk-based sizing (AmiBroker examples)"
```

---

### Task 10: Rotation mode and periodic rebalancing; fixture P3 (`portfolio.rs`)

**Files:** Modify: `Context/guru/backend/src/lab/portfolio.rs`

**Interfaces:** Consumes Tasks 8–9, `resample::period_key`. Produces: rotation behaviour of `run` when `b.mode == Mode::Rotation` (`Lane.c.entry` = eligibility, `Lane.score` required); reasons `"rank exit (rank N > worst_rank_held W)"` and `"not eligible"`.

- [ ] **Step 1: Write the failing tests**:

```rust
    fn rotation(max_open: usize, worst: usize, rebalance: Option<usize>) -> Book {
        Book { mode: Mode::Rotation, worst_rank_held: worst, rebalance, ..book(max_open, Size::PctEquity(100. / max_open as f64)) }
    }
    #[test] fn p3_holdings_stay_until_their_rank_drops_below_worst_rank_held() {
        // backtestRotational: enters from the highest rank; holds until rank drops below WorstRankHeld.
        let ms: Vec<Member> = ["A", "B", "C", "D"].iter().map(|t| member(t, 100., 12, &[])).collect();
        // scores per bar (same for all bars except where changed): bar 2 ranks A,B,C,D; bar 5 ranks C,D,A,B (A=3 kept); bar 8 ranks C,D,B,A (A=4 out → C)
        let mut s = vec![vec![4.; 12], vec![3.; 12], vec![2.; 12], vec![1.; 12]];
        for i in 5..12 { s[0][i] = 2.; s[1][i] = 1.; s[2][i] = 4.; s[3][i] = 3.; }
        for i in 8..12 { s[0][i] = 0.5; }
        let lanes: Vec<Lane> = ms.iter().zip(s).map(|(m, sc)| lane(m, &(0..12).collect::<Vec<_>>(), &[], Some(sc))).collect();
        let b = rotation(2, 3, None);
        let l = run(&lanes, &b, &dates(12), 0, 11);
        let held_at = |i: usize| -> Vec<&str> { let mut v: Vec<&str> = l.trades.iter().filter(|t| t.entry_i <= i && (t.open || t.exit_i > i)).map(|t| lanes[t.lane].m.ticker.as_str()).collect(); v.sort(); v };
        assert_eq!(held_at(1), ["A", "B"], "entered at bar 1 from bar 0's ranks");
        assert_eq!(held_at(6), ["A", "C"], "bar 5: B (rank 4) out, A (rank 3) kept, C in");
        assert_eq!(held_at(9), ["C", "D"], "bar 8: A drops to rank 4 and is replaced by D");
        assert!(l.trades.iter().any(|t| t.reason.starts_with("rank exit (rank 4 > worst_rank_held 3)")));
        invariants(&l, &lanes, &b, 0);
    }
    #[test] fn ineligible_holdings_exit_and_monthly_rebalance_waits_for_month_end() {
        let ms: Vec<Member> = ["A", "B"].iter().map(|t| member(t, 100., 70, &[])).collect();
        // A is the best score but becomes ineligible on bars 40..; monthly: rebalance only on the last bar of each month
        let lanes = [lane(&ms[0], &(0..40).collect::<Vec<_>>(), &[], Some(vec![2.; 70])), lane(&ms[1], &(0..70).collect::<Vec<_>>(), &[], Some(vec![1.; 70]))];
        let b = rotation(1, 1, Some(2));
        let l = run(&lanes, &b, &dates(70), 0, 69);
        let d = dates(70);
        // first rebalance = 2024-01-31 (bar 30) → A fills at bar 31's open; A ineligible from 40, exits at the next month end 2024-02-29 (bar 59)
        assert_eq!((lanes[l.trades[0].lane].m.ticker.as_str(), d[l.trades[0].signal_i].as_str(), l.trades[0].entry_i), ("A", "2024-01-31", 31));
        assert_eq!((d[l.trades[0].exit_i - 1].as_str(), l.trades[0].reason.as_str()), ("2024-02-29", "not eligible"));
        assert_eq!(lanes[l.trades[1].lane].m.ticker, "B");
        invariants(&l, &lanes, &b, 0);
    }
    #[test] fn a_held_member_without_a_bar_is_kept_until_it_trades_again() {
        let ms = [member("A", 100., 10, &[4]), member("B", 100., 10, &[])];
        let lanes = [lane(&ms[0], &(0..10).collect::<Vec<_>>(), &[], Some(vec![2., 2., 2., 2., 2., 0.5, 0.5, 0.5, 0.5, 0.5])), lane(&ms[1], &(0..10).collect::<Vec<_>>(), &[], Some(vec![1.; 10]))];
        let l = run(&lanes, &rotation(1, 1, None), &dates(10), 0, 9);
        assert_eq!((l.trades[0].exit_i, l.trades[0].signal_i), (6, 5), "no rank or price on bar 4: kept; ranked out at 5, out at 6's open");
    }
```

- [ ] **Step 2: Run to verify failure** — rotation is not implemented (no trades).

- [ ] **Step 3: Implement** — in `run`, at the close step (after the mark and data-end exits), when `b.mode == Mode::Rotation && !last`:
```rust
            let rebalance_bar = b.rebalance.map_or(true, |a| crate::lab::resample::period_key(&dates[i], a) != crate::lab::resample::period_key(&dates[i + 1], a));
            if rebalance_bar {
                // rank every eligible member with a bar today and a finite score
                let mut ranked: Vec<Cand> = lanes.iter().enumerate().filter_map(|(m, ln)| {
                    let k = ln.m.at[i]?;
                    let sc = ln.score.as_ref()?[k];
                    (sc.is_finite() && ln.c.entry.iter().all(|r| r.holds(k, None))).then(|| { let px = ln.m.frames.d.close[k];
                        Cand { lane: m, key: if b.lowest { -sc } else { sc }, want: b.size.requested(equity, px, f64::NAN), score: sc, signal_i: i, eq: equity, rank: 0 } })
                }).collect();
                rank(&mut ranked, lanes, b);
                let pos = |m: usize| ranked.iter().position(|c| c.lane == m);
                for m in 0..lanes.len() {
                    if held[m].is_none() || lanes[m].m.at[i].is_none() || pend_exit[m].is_some() { continue; }
                    let why = match pos(m) { None => Some("not eligible".to_string()), Some(r) if r + 1 > b.worst_rank_held => Some(format!("rank exit (rank {} > worst_rank_held {})", r + 1, b.worst_rank_held)), _ => None };
                    if let Some(why) = why { /* same exit_fill handling as the signals-mode rule exit */ }
                }
                let holding: Vec<usize> = (0..lanes.len()).filter(|&m| held[m].is_some() && pend_exit[m].is_none()).collect();
                let free = b.max_open.saturating_sub(holding.len());
                let mut cands: Vec<Cand> = ranked.into_iter().enumerate().filter(|(_, c)| held[c.lane].is_none() || pend_exit[c.lane].is_some()).map(|(r, mut c)| { c.rank = r + 1; c }).take(free).collect();
                /* fill `cands` exactly like the signals-mode entries (close fills now, or pending next-open entries) */
            }
```
Move the shared "exit with exit_fill" and "enter candidates" code into two small functions used by both modes (no duplicated logic). `period_key` is `resample::period_key(date, anchor)` (1 W, 2 M, 3 Q, 4 Y). Rotation never evaluates `c.exit` or price exits.

- [ ] **Step 4: Run to verify pass** — `cargo test lab::portfolio` → 10 passed.

- [ ] **Step 5: Commit**

```bash
git add Context/guru/backend/src/lab/portfolio.rs
git commit -m "Add lab rotational portfolios: rank exits by worst rank held, eligibility, periodic rebalance"
```

---

### Task 11: Price exits, data end and missing bars in a portfolio (`portfolio.rs`, `simulate.rs`)

**Files:** Modify: `Context/guru/backend/src/lab/simulate.rs` (visibility only: `pub(super) struct Pos`, `pub(super) fn open`, `pub(super) fn price_exit`, fields `pub(super)`), `Context/guru/backend/src/lab/portfolio.rs`

**Interfaces:** Consumes the single-instrument per-position exit logic (Customize `Pos::{open, price_exit}`; Phase 7: its per-position struct incl. trailing activation, breakeven and `intrabar`). Produces: reasons `exit.stop_loss`, `exit.target`, `exit.trailing` (as single-instrument) and `data_end`; `Ledger.events` lines `"<ticker>: data ends <date>; closed at the last close"`.

- [ ] **Step 1: Write the failing tests**:

```rust
    #[test] fn price_exits_use_each_members_own_bars() {
        let mut a = member("A", 100., 10, &[]);
        { let d = &mut a.frames.d; d.low[5] = 90.; } // 5% stop from the entry 103 = 97.85, touched on bar 5
        let mut ln = lane(&a, &[2], &[], None);
        ln.c.exec.stop_loss = Some(crate::lab::rules::Dist::Pct(5.));
        let l = run(&[ln], &book(1, Size::PctEquity(100.)), &dates(10), 0, 9);
        assert_eq!((l.trades[0].exit_i, l.trades[0].exit_px, l.trades[0].reason.as_str()), (5, 103. * 0.95, "exit.stop_loss"));
    }
    #[test] fn data_end_closes_at_the_last_close_and_frees_cash() {
        let gone = { let mut m = member("GONE", 100., 6, &[]); m.at.extend([None; 6]); m }; // data ends at bar 5 of 12
        let next = member("NEXT", 100., 12, &[]);
        let lanes = [lane(&gone, &[1], &[], None), lane(&next, &[1, 7], &[], None)];
        let b = book(1, Size::PctEquity(100.));
        let l = run(&lanes, &b, &dates(12), 0, 11);
        let t = &l.trades[0];
        assert_eq!((lanes[t.lane].m.ticker.as_str(), t.exit_i, t.exit_px, t.reason.as_str()), ("GONE", 5, 105.5, "data_end"));
        assert!(l.events[0].starts_with("GONE: data ends 2024-01-06"));
        assert_eq!((lanes[l.trades[1].lane].m.ticker.as_str(), l.trades[1].entry_i), ("NEXT", 8), "the freed cash funds the next signal");
        invariants(&l, &lanes, &b, 0);
    }
    #[test] fn a_member_without_a_bar_keeps_its_last_close_as_the_mark() {
        let a = member("A", 100., 10, &[5]);
        let lanes = [lane(&a, &[2], &[], None)];
        let b = book(1, Size::PctEquity(100.));
        let l = run(&lanes, &b, &dates(10), 0, 9);
        let q = l.trades[0].qty;
        assert_eq!(l.invested[5], q * 104.5, "bar 5 has no quote: marked at bar 4's close");
        invariants(&l, &lanes, &b, 0);
    }
```

- [ ] **Step 2: Run to verify failure** — no price exits, no data-end exit (the GONE position stays open), wrong mark.

- [ ] **Step 3: Implement** — (a) `simulate.rs`: change only visibility (`struct Pos` → `pub(super) struct Pos` with `pub(super)` fields; `fn open`/`fn price_exit` → `pub(super)`); no code change; `cargo test lab::simulate` and `pinned` stay green. (b) `portfolio.rs`: `Held` keeps a `pos: crate::lab::simulate::Pos` built with `Pos::open(&ln.c, k, px, signal_k, kind, 1.)` at each entry (member-bar indices). Step 2 of the bar ("inside the bar"): for each held member with a bar `k` today and no pending exit, `if let Some((px, why)) = h.pos.price_exit(&ln.c, &ln.m.frames.d, k, 1.)` → `close_out(…, px, i, why)`; mark `exited_after_open[m] = true`; after the close, update `h.pos.best` exactly as `simulate()` does. (c) Data end: at the close step, before rule exits, for each held member where `ln.m.at[i].is_some()` and no later reference bar in `i+1..=to` maps to a member bar (precompute `last_i[m]` = the last reference index with `at[_].is_some()`), and `i < to`: `close_out(…, close[k], i, "data_end")` and push the event `format!("{}: data ends {}; closed at the last close", ticker, dates[i])`. (d) The mark already uses the last known close (`mark[m]` updated only on bars the member has).

- [ ] **Step 4: Run to verify pass** — `cargo test lab::portfolio lab::simulate pinned` → all pass (13 portfolio tests).

- [ ] **Step 5: Commit**

```bash
git add Context/guru/backend/src/lab/portfolio.rs Context/guru/backend/src/lab/simulate.rs
git commit -m "Apply single-instrument price exits per member; close data-ended members at their last close"
```

---

### Task 12: Portfolio runs through `lab/run` (`api.rs`)

**Files:** Modify: `Context/guru/backend/src/lab/api.rs` (`run_json` dispatch, new `portfolio_run_json`, `portfolio_book`, `portfolio_stats`; tests)

**Interfaces:**
- Consumes: `universe::load` (Task 7), `portfolio::{run, Lane, Book, Size, Charges}` (Tasks 8–11), `others_for` (Task 5), `rules::Portfolio` (Task 6).
- Produces: `GET /api/lab/run` with `spec.portfolio` → body of spec §10 (`kind:"lab_portfolio_simulation"`); refusals: `ticker` present → invalid "portfolio runs read their instruments from spec.portfolio.universe; omit ticker"; gl → unsupported; the bps `cost` query → invalid (Phase 7 money rule); 90 s → invalid "portfolio run stopped after 90 s; use fewer members or a shorter window".

- [ ] **Step 1: Write the failing tests** — append to the `api.rs` tests (reuse `universe::tests::india` by making it `pub(crate)`):

```rust
    const PSPEC: &str = r#"{"v":1,"entry":{"all":[{"lhs":{"s":"close"},"op":">","rhs":{"s":"sma","p":{"length":5}}},{"lhs":{"s":"close","symbol":"NIFTY 50"},"op":">","rhs":{"value":0}}]},"exit":{"any":[{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":3}}]},"execution":{"capital":100000},"portfolio":{"universe":{"basket":"nifty50"},"score":{"s":"roc","p":{"length":3}},"max_open":2}}"#;
    fn pq(spec: &str) -> Query { [("spec", spec)].iter().map(|(k, v)| (k.to_string(), v.to_string())).collect() }
    #[test] fn a_portfolio_run_is_labelled_bounded_and_declares_survivorship() {
        let (_d, st) = crate::lab::universe::tests::india("api-run");
        let j = run_json(&st, &pq(PSPEC)).unwrap();
        for k in ["\"label\":\"User-defined indicator strategy · simulated · not a book method\"", "\"kind\":\"lab_portfolio_simulation\"",
                  "\"membership\":{\"kind\":\"current_snapshot\",\"historical_membership_verified\":false", "\"reference\":{\"ticker\":\"NIFTY 50\",\"type\":\"index\"}",
                  "\"universe\":{\"basket\":\"nifty50\",\"members\":2", "\"foreign\":[{\"ticker\":\"NIFTY 50\"", "\"skipped\":{", "\"money\":{\"capital\":100000"] {
            assert!(j.contains(k), "{k} missing");
        }
        for k in ["claim", "gate", "readiness"] { assert!(!j.contains(&format!("\"{k}\""))); }
    }
    #[test] fn portfolio_refusals_are_typed() {
        let (_d, st) = crate::lab::universe::tests::india("api-refuse");
        let mut with_ticker = pq(PSPEC); with_ticker.insert("ticker".into(), "NIFTY 50".into());
        assert!(run_json(&st, &with_ticker).unwrap_err().message().contains("omit ticker"));
        let mut bps = pq(PSPEC); bps.insert("cost".into(), "10".into());
        assert_eq!(run_json(&st, &bps).unwrap_err().status(), "invalid");
        let spec = rules::parse_spec(PSPEC).unwrap();
        assert_eq!(portfolio_run_json(&st, &pq(PSPEC), &spec, true).unwrap_err().status(), "unsupported", "engine-gl refuses portfolios (its 400 body gets the provisional flag from response())");
    }
    #[test] fn members_without_window_data_are_listed_not_fatal() {
        let (_d, st) = crate::lab::universe::tests::india("api-refused");
        let spec = PSPEC.replace(r#"{"basket":"nifty50"}"#, r#"{"tickers":["basket:nifty50:A","basket:nifty50:B"]}"#);
        let mut q = pq(&spec); q.insert("from".into(), "2024-01-20".into());
        let j = run_json(&st, &q).unwrap();
        assert!(j.contains("\"refused\":[]") || j.contains("\"refused\":[{"), "refused list always present");
    }
```
(The engine-gl market comes from `VAULT_LAB_MARKET`, so `portfolio_run_json` takes `gl: bool` and the test passes `true` directly; gl members are refused separately by `universe::load`.)

- [ ] **Step 2: Run to verify failure** — `run_json` refuses `portfolio` (Phase 7: `unsupported`); after Task 6 it parses, so the failure is the missing body keys.

- [ ] **Step 3: Implement** — at the top of `run_json`, after `spec_of`: `if spec.portfolio.is_some() { return portfolio_run_json(st, q, &spec, markets::engine_market().as_deref() == Some("gl")); }` and the same dispatch in `optimize_json` (Task 13). Then:

```rust
const PORTFOLIO_SECONDS: u64 = 90;
fn portfolio_run_json(st: &State, q: &Query, spec: &Spec, gl: bool) -> Result<String, LabError> {
    let t0 = std::time::Instant::now();
    if gl { return Err(LabError::Unsupported("portfolio mode runs on India/US stocks and ETFs; gl instruments are provisional indices, CFDs and crypto".into())); }
    if q.get("ticker").is_some_and(|t| !t.is_empty()) { return Err(LabError::Invalid("portfolio runs read their instruments from spec.portfolio.universe; omit ticker".into())); }
    if q.get("split").is_some_and(|s| !s.is_empty()) { return Err(LabError::Invalid("split applies to /api/lab/optimize only".into())); }
    // Phase 7 money rule: the bps cost/slippage queries and execution.cost_bps are refused in money mode
    if ["cost", "slippage", "rollover"].iter().any(|k| q.get(*k).is_some()) || spec.cost_bps.is_some() { return Err(LabError::Invalid("money mode takes costs from execution.costs; remove the cost query".into())); }
    let p = spec.portfolio.as_ref().unwrap();
    let loaded = super::universe::load(st, &p.universe)?;
    let rctx = &st.ctxs[&loaded.reference];
    let (_, mut from, mut to, _) = crate::resolve_context(rctx, q)?;
    // default window: reference coverage ∩ [earliest member first bar, latest member last bar] (spec §5.2)
    if q.get("from").is_none() { if let Some(f) = loaded.members.iter().filter_map(|m| m.at.iter().position(Option::is_some)).min() { from = from.max(f); } }
    if q.get("to").is_none() { if let Some(t) = loaded.members.iter().filter_map(|m| m.at.iter().rposition(Option::is_some)).max() { to = to.min(t); } }
    if from >= to { return Err(LabError::NeedsData("no member has data inside the window".into())); }
    let (others, foreign_json) = others_for(st, &loaded.reference, &spec.foreign_symbols(), &spec.fixup_symbols(), close_fills(&spec.exec))?;
    let mut refused: Vec<(String, String)> = vec![];
    let mut lanes = vec![];
    for m in &loaded.members {
        if t0.elapsed().as_secs() >= PORTFOLIO_SECONDS { return Err(LabError::Invalid(format!("portfolio run stopped after {PORTFOLIO_SECONDS} s; use fewer members or a shorter window"))); }
        let ks: Vec<usize> = (from..=to).filter_map(|i| m.at[i]).collect();
        let Some((&a, &z)) = ks.first().zip(ks.last()) else { refused.push((m.ticker.clone(), "no bars inside the window".into())); continue };
        if let Some(why) = bad_bar(&m.frames.d, a, z) { refused.push((m.ticker.clone(), why)); continue; }
        let mo = others.with_run(&m.ticker); // the run name drives the session rule
        let mut cache = Cache::new();
        let c = match series::compile_with(spec, &m.frames, &mo, &mut cache) { Ok(c) => c, Err(e) => { refused.push((m.ticker.clone(), e.message().into())); continue } };
        let score = match &p.score { None => None, Some(s) => match series::series_in(&m.frames, &mo, &mut cache, s) { Ok(v) => Some(v), Err(e) => { refused.push((m.ticker.clone(), e.message().into())); continue } } };
        lanes.push(super::portfolio::Lane { m, c, score });
    }
    if lanes.is_empty() { return Err(LabError::NeedsData("no universe member is admitted for this window (see the reasons per member)".into())); }
    let book = portfolio_book(spec)?;
    let led = super::portfolio::run(&lanes, &book, &loaded.dates, from, to);
    let gross = super::portfolio::run(&lanes, &super::portfolio::Book { charges: super::portfolio::Charges::default(), ..book.clone() }, &loaded.dates, from, to);
    /* JSON: see below */
}
```
Helper notes (write them as small functions, not inline):
- `bad_bar(d: &Bars, a: usize, z: usize) -> Option<String>`: the `bars_clean` predicate applied to `Bars` (extract the `why` match from `bars_clean` into `fn bar_problem(o, h, l, c) -> Option<&'static str>` and use it in both; `bars_clean`'s messages stay identical).
- `portfolio_book(spec) -> Result<Book, LabError>`: `capital` from Phase 7's `execution.capital`; `size` from Phase 7's `execution.sizing` (`pct_equity`→`PctEquity`, `value`→`Value`, `units`→`Units`, `risk_pct`→`RiskPct`; absent → `PctEquity(100 / max_open)`); `charges` from Phase 7's `execution.costs` (commission `per_order`, `per_unit`, `pct` on both sides; each tax's pct added to `buy_pct`/`sell_pct` by its `side`; slippage `bps`; slippage `ticks` → `unsupported` "tick slippage needs a futures contract"); `fill`/`exit_fill` from `spec.exec`; `mode`, `max_open`, `worst_rank_held`, `rebalance`, `lowest` from the portfolio; re-check `worst_rank_held >= max_open` (optimize can break it) → invalid. Derive `Clone` on `Book`, `Size`, `Charges` and `Default` on `Charges`.
- Statistics: map each `PTrade` to a book `Trade` on the reference calendar: `Trade { dir: 1., legs: vec![Leg { i: entry_i, px: (entry_px * qty + entry_cost) / qty, w: (entry_px * qty + entry_cost) / equity_at_signal }], exit_i, exit_px: (exit_px * qty - exit_cost) / qty, reason, entry_kind, signal_i, open, exits: vec![] }` (Phase 7 effective prices: `ret(0)` = net P&L ÷ equity at the signal). `Run { trades (sorted by entry_i), from, to, equity: led.equity / capital, bh: reference close / reference close[from] }`; `stats = crate::stats(rctx, &run, 0.)` then `markets::annualise(&mut s, 252.)` (no-op) and add `avg_exposure` = mean(invested/equity) and `max_positions`. Gross stats from the zero-charge run.
- Body: `{"label":LABEL,"kind":"lab_portfolio_simulation","origin":null,"portfolio":{"mode","max_open","worst_rank_held","rebalance","rank","universe":{"basket":…|null,"members":N,"admitted":M,"refused":[{"ticker","reason"}]}},"membership":{"kind",…,"historical_membership_verified":false,"note"},"execution_model":{"kind":"cash","mode":"money","executable":true,"notes":["computed on the admitted price series, whose corporate-action adjustment is not established; not verified as-traded quantities"],"events":[…led.events]},"reference":{"ticker","type":"index"}{foreign_json},"from","to","dates","equity","gross_equity","cash","exposure","positions","bh","trades":[…],"skipped":{"no_slot","no_cash","no_bar"},"stats","gross_stats","money":{"capital","final_equity","costs_total","currency"},"provenance":[member provenance_json or null]}`; `trades[]` entries `{"ticker","entry","exit","entry_i","exit_i","entry_px","exit_px","qty","notional","costs","ret","bars","open","signal","reason","rank","score","dir":1,"entry_kind"}` with `*_i` relative to `from`. Currency from the first member's market (`INR`/`USD`).

- [ ] **Step 4: Run to verify pass** — `cargo test lab::api` → +3; full `cargo test lab::` green.

- [ ] **Step 5: Commit**

```bash
git add Context/guru/backend/src/lab/api.rs Context/guru/backend/src/lab/series.rs Context/guru/backend/src/lab/portfolio.rs Context/guru/backend/src/lab/universe.rs
git commit -m "Serve lab portfolio runs: members, shared ledger, survivorship note, money statistics"
```

---

### Task 13: Portfolio optimization through `lab/optimize` (`api.rs`)

**Files:** Modify: `Context/guru/backend/src/lab/api.rs` (`optimize_json` dispatch, `portfolio_optimize_json`; tests)

**Interfaces:** Consumes Task 12's member preparation (factor it as `fn prepare(st, q, spec) -> Result<Prepared, LabError>` holding `loaded`, `from`, `to`, `others`, `refused`) and `rules::apply`. Produces `kind:"lab_portfolio_optimization"` with the Phase 2 optimize shape (`dims`, `cols`, `base`, `rank`, `rows`, `ranked_on:"full"`, `split:null`); limits `PORTFOLIO_MAX_COMBOS = 200`, `PORTFOLIO_MAX_WORK = 12_000` (members × combos), `split` → invalid "split is not available for portfolio optimizations in this phase".

- [ ] **Step 1: Write the failing tests**:

```rust
    #[test] fn portfolio_optimize_ranks_combinations_within_its_bounds() {
        let (_d, st) = crate::lab::universe::tests::india("api-opt");
        let spec = PSPEC.replace(r#""max_open":2}}"#, r#""max_open":2}, "optimize":[{"path":"portfolio.max_open","from":1,"to":2,"step":1},{"path":"entry.all[0].rhs.p.length","from":3,"to":6,"step":3}]}"#);
        let j = optimize_json(&st, &pq(&spec)).unwrap();
        assert!(j.contains("\"kind\":\"lab_portfolio_optimization\"") && j.contains("\"combos\":4") && j.contains("\"cols\":[\"portfolio.max_open\",\"entry.all[0].rhs.p.length\""));
        let mut split = pq(&spec); split.insert("split".into(), "2024-02-01".into());
        assert!(optimize_json(&st, &split).unwrap_err().message().contains("split is not available for portfolio"));
        let big = PSPEC.replace(r#""max_open":2}}"#, r#""max_open":2}, "optimize":[{"path":"entry.all[0].rhs.p.length","from":2,"to":202,"step":1}]}"#);
        assert!(optimize_json(&st, &pq(&big)).unwrap_err().message().contains("at most 200 combinations"));
    }
    #[test] fn portfolio_optimize_is_deterministic_across_threads() {
        let (_d, st) = crate::lab::universe::tests::india("api-opt-det");
        let spec = PSPEC.replace(r#""max_open":2}}"#, r#""max_open":2}, "optimize":[{"path":"portfolio.score.p.length","from":2,"to":8,"step":2}]}"#);
        let strip = |s: String| s.split(",\"ms\":").next().unwrap().to_string() + &s[s.find(",\"dims\"").unwrap()..];
        assert_eq!(strip(optimize_json(&st, &pq(&spec)).unwrap()), strip(optimize_json(&st, &pq(&spec)).unwrap()));
    }
```

- [ ] **Step 2: Run to verify failure** — `optimize_json` takes the single-instrument path.

- [ ] **Step 3: Implement** — `portfolio_optimize_json(st, q, spec, gl)`: refuse gl, `ticker`, `split`; `combos = Π values` → `> 200` → invalid "portfolio optimize allows at most 200 combinations"; `members × combos > 12_000` → invalid "members × combinations is N; the limit is 12000 — use fewer members or a smaller grid"; `prepare` once; worker threads exactly as `optimize_json` (mixed-radix order, stop flag, 300 s, first real error wins), each combination: `rules::apply` → re-compile every lane (`compile_with` with one `Cache` per member per thread) → `portfolio::run` → the portfolio stats row (cols `PCOLS = ["total_return","cagr","sharpe","sortino","max_dd","mar","trades","win_rate","profit_factor","avg_trade","exposure","t_stat","avg_bars","expectancy_ratio","monthly_thickness","avg_hold_days","avg_exposure","max_positions"]`); ranking with `crate::conclude_rank_with_outcomes(&rows, col("trades"), &crate::RANK_STATS.map(|(k, _)| col(k)), min_trades, per_year, &defs, None)` using the same trade-rate floor arithmetic as `optimize_json` (`crate::years(rctx, from, to)`, `MIN_PER_YEAR`, `MIN_TRADES`). Body: the `optimize_json` format with `kind`, `PCOLS`, `"split":null`, `"ranked_on":"full"`, plus `membership` and `portfolio` as in Task 12.

- [ ] **Step 4: Run to verify pass** — `cargo test lab::api` → +2.

- [ ] **Step 5: Commit**

```bash
git add Context/guru/backend/src/lab/api.rs
git commit -m "Optimize lab portfolios within bounded grids on the shared optimize slot"
```

---

### Task 14: Cross-symbol no-lookahead, holiday alignment and determinism (`portfolio.rs`, `api.rs`)

**Files:** Modify: `Context/guru/backend/src/lab/portfolio.rs` (tests), `Context/guru/backend/src/lab/api.rs` (tests)

**Interfaces:** Consumes everything above. Produces tests only.

- [ ] **Step 1: Write the tests** — (a) `portfolio.rs`:

```rust
    /// Perturbs every bar after k of every member (prices ×1.3 and +noise); trades closed by k and the ledger through k must not change.
    #[test] fn future_bars_never_change_the_past_portfolio() {
        let base: Vec<Member> = (0..5).map(|j| { let mut m = member(&format!("M{j}"), 100. + 7. * j as f64, 80, &[j * 3 + 1]); let d = &mut m.frames.d;
            for i in 0..d.len() { let w = ((i * (j + 3)) % 11) as f64 - 5.; d.close[i] += w; d.high[i] = d.high[i].max(d.close[i]) + 1.; d.low[i] = d.low[i].min(d.close[i]) - 1.; } m }).collect();
        for (mode, fill) in [(Mode::Signals, Fill::NextOpen), (Mode::Signals, Fill::Close), (Mode::Rotation, Fill::NextOpen), (Mode::Rotation, Fill::Close)] {
            let mk = |ms: &[Member]| -> Ledger {
                let lanes: Vec<Lane> = ms.iter().map(|m| { let d = &m.frames.d; let on: Vec<usize> = (0..80).filter(|&i| m.at[i].is_some_and(|k| d.close[k] > d.open[k])).collect();
                    let off: Vec<usize> = (0..80).filter(|i| i % 4 == 0).collect(); lane(m, &on, if mode == Mode::Signals { &off } else { &[] }, Some(d.close.clone())) }).collect();
                let b = Book { mode, fill, exit_fill: fill, worst_rank_held: 3, ..book(2, Size::PctEquity(50.)) };
                run(&lanes, &b, &dates(80), 0, 79)
            };
            let full = mk(&base);
            for k in [20usize, 47, 63] {
                let cut: Vec<Member> = base.iter().map(|m| { let mut m2 = member(&m.ticker, 1., 80, &[]); m2.frames = Frames::new({ let mut d = m.frames.d.clone();
                    for i in 0..d.len() { if m.at.iter().position(|x| *x == Some(i)).is_some_and(|r| r > k) { d.open[i] *= 1.3; d.high[i] *= 1.4; d.low[i] *= 1.2; d.close[i] *= 1.3; } } d }); m2.at = m.at.clone(); m2 }).collect();
                let part = mk(&cut);
                assert_eq!(&full.equity[..=k], &part.equity[..=k], "{mode:?} {fill:?}: equity through {k}");
                let closed = |l: &Ledger| l.trades.iter().filter(|t| !t.open && t.exit_i <= k).map(|t| (t.lane, t.entry_i, t.exit_i, t.qty, t.entry_px.to_bits(), t.exit_px.to_bits())).collect::<Vec<_>>();
                assert_eq!(closed(&full), closed(&part), "{mode:?} {fill:?}: trades closed by {k}");
            }
        }
    }
    #[test] fn universe_order_never_changes_the_result() {
        let ms: Vec<Member> = ["B", "A", "C"].iter().map(|t| member(t, 100., 30, &[])).collect();
        let run_in = |order: &[usize]| { let lanes: Vec<Lane> = order.iter().map(|&j| lane(&ms[j], &[2, 9, 15], &[6, 12], Some(vec![1.; 30]))).collect();
            let l = run(&lanes, &book(2, Size::PctEquity(50.)), &dates(30), 0, 29);
            let mut t: Vec<(String, usize, usize)> = l.trades.iter().map(|t| (lanes[t.lane].m.ticker.clone(), t.entry_i, t.exit_i)).collect(); t.sort(); (t, l.equity) };
        assert_eq!(run_in(&[0, 1, 2]), run_in(&[2, 1, 0]));
    }
```
(The test re-uses the original `at` maps so perturbed members keep their calendars; if `Member` gains fields, copy them too.)
(b) `api.rs` — foreign trade-level property and holiday mismatch:
```rust
    #[test] fn future_bars_of_the_other_instrument_never_change_past_trades() {
        let st = multi(&[("SPY", 5)]);
        let full = run_json(&st, &q(&[("ticker", "SPY"), ("spec", FILTER)])).unwrap();
        let mut cut = multi(&[("SPY", 5)]);
        let k = 500;
        let rows: &'static [Row] = Box::leak(cut.ctxs["SPX"].r.iter().enumerate().map(|(i, r)| if i > k { Row { close: r.close * 1.5, open: r.open * 1.5, high: r.high * 1.5, low: r.low * 1.5, ..r.clone() } } else { r.clone() }).collect::<Vec<_>>().into_boxed_slice());
        cut.ctxs.insert("SPX".into(), Ctx::new(rows, rows, &[]));
        let part = run_json(&cut, &q(&[("ticker", "SPY"), ("spec", FILTER)])).unwrap();
        let date_k = &cut.ctxs["SPX"].r[k].date;
        let closed = |j: &str| -> Vec<String> { json::parse(j).unwrap().get("trades").and_then(|t| if let json::Json::Arr(a) = t { Some(a.clone()) } else { None }).unwrap()
            .iter().filter(|t| t.get("exit").and_then(json::Json::as_str).is_some_and(|e| e <= date_k.as_str())).map(|t| format!("{t:?}")).collect() };
        assert!(!closed(&full).is_empty());
        assert_eq!(closed(&full), closed(&part));
    }
```
(`json` is `super::super::json`; import it in the test module.)

- [ ] **Step 2: Run** — `cargo test lab::portfolio lab::api` → the new tests pass if Tasks 2–13 are right; a failure here is a real lookahead or ordering bug: fix the code, never the assertion.

- [ ] **Step 3: Optional real-data smoke (live-server data only)** — when `VAULT_BASKET_DATA_DIR` points at the India basket cache, run the engine on a free local port (never the live stack) with `VAULT_BASKET_DATA_DIR` set and call `/api/lab/run?spec=<PSPEC with "basket":"nifty50", "capital":1000000>`; expect 200, `"members":50`, `"membership":{"kind":"current_snapshot"`. Record the result or "not available on this machine".

- [ ] **Step 4: Commit**

```bash
git add Context/guru/backend/src/lab/portfolio.rs Context/guru/backend/src/lab/api.rs
git commit -m "Test lab multi-symbol causality, holiday alignment and portfolio determinism"
```

---

### Task 15: Portfolio strategies are research-only in saved strategies (auth)

**Files:**
- Modify: `Context/guru/webapp/auth/src/contracts.mjs` (`labPublishable`, use in `labAction`), `Context/guru/webapp/auth/src/lab-strategies.mjs` (`view`)
- Test: `Context/guru/webapp/tests/lab-strategies.test.mjs`, `Context/guru/webapp/tests/contracts.test.mjs`

**Interfaces:** Produces `export function labPublishable(row)` → `LAB_PUBLISHABLE_MARKETS.has(row.market) && !row.spec?.portfolio`; submit/publish of a portfolio spec → `400` "Portfolio strategies use today's basket lists, not dated index membership (survivorship bias); they stay research-only until historical membership is admitted."; every strategy JSON's `publishable` follows `labPublishable`.

- [ ] **Step 1: Write the failing tests** — (a) `tests/lab-strategies.test.mjs`:
```js
test('a portfolio strategy is saved but cannot be submitted to YT Strategies', async () => {
  const PORT = { ...SPEC, portfolio: { universe: { basket: 'nifty50' } } };
  const { log, call } = harness({ 7: row({ spec: PORT, instrument: 'portfolio:nifty50' }) });
  const response = await call('owner', 'POST', '/7/submit', { version: 3 });
  assert.equal(response.status, 400);
  assert.match((await response.json()).error, /survivorship bias/);
  assert.ok(!log.includes('UPDATE'));
  const read = await call('owner', 'GET', '/7');
  assert.equal((await read.json()).publishable, false);
});
```
(b) `tests/contracts.test.mjs`:
```js
test('publishable needs an India/US market and no portfolio', async () => {
  const { labPublishable } = await import('../auth/src/contracts.mjs');
  assert.equal(labPublishable({ market: 'in', spec: { v: 1 } }), true);
  assert.equal(labPublishable({ market: 'gl', spec: { v: 1 } }), false);
  assert.equal(labPublishable({ market: 'us', spec: { v: 1, portfolio: { universe: { basket: 'us_dow30' } } } }), false);
});
```
- [ ] **Step 2: Run to verify failure** — `cd Context/guru/webapp && node --test tests/contracts.test.mjs tests/lab-strategies.test.mjs` → the two new tests fail.
- [ ] **Step 3: Implement** — `contracts.mjs`: add
```js
// Phase 8: portfolios run on today's basket lists (no dated membership) and stay research-only.
export function labPublishable(row) { return LAB_PUBLISHABLE_MARKETS.has(row.market) && !row.spec?.portfolio; }
```
and in `labAction`, after the existing provisional-market line:
```js
  if (rule.publishable && row.spec?.portfolio) throw new LabRequestError('Portfolio strategies use today\'s basket lists, not dated index membership (survivorship bias); they stay research-only until historical membership is admitted.', 400);
```
`lab-strategies.mjs`: import `labPublishable` and change `view` to `row && { ...row, publishable: labPublishable(row) }`. (Rows read for actions already carry `spec`; if the locked-row query of submit/publish selects fewer columns, add `s.spec` to it.)
- [ ] **Step 4: Run to verify pass** — `node --test tests/contracts.test.mjs tests/lab-strategies.test.mjs tests/book-library.test.mjs && npm --prefix auth test` → `# fail 0`.
- [ ] **Step 5: Commit**
```bash
git add Context/guru/webapp/auth/src/contracts.mjs Context/guru/webapp/auth/src/lab-strategies.mjs Context/guru/webapp/tests/contracts.test.mjs Context/guru/webapp/tests/lab-strategies.test.mjs
git commit -m "Keep lab portfolio strategies research-only: no YT submission while membership is undated"
```

---

### Task 16: Phase gate

- [ ] **Step 1: Engine suite** — `cd Context/guru/backend && VAULT_IDX="/run/user/1000/gvfs/smb-share:server=192.168.4.50,share=share/Manan Pujara/Book-to-rust/idx_data.csv" CARGO_BUILD_JOBS=2 cargo test` → all pass; count = "Before Task 1" count **+ 46** new tests (rules 5, foreign 5, pairs 3, series 5, api 10, universe 3, portfolio 15); the retired Phase 7 reserved-`symbol`/`portfolio` assertions are rows inside existing tests, so they change no count unless Phase 7 made them whole tests (then subtract them). Record both counts.
- [ ] **Step 2: Compatibility** — `cargo test pinned` (Phase 7 pinned-output hash) and `cargo test templates_reproduce_the_book_engine_on_fixtures` (43 Customize templates) pass unchanged; `git diff $BASE -- Context/guru/backend/src/lab/pinned.rs Context/guru/backend/src/lab/templates.rs` is empty.
- [ ] **Step 3: Release build without lab warnings** — `touch src/main.rs && CARGO_BUILD_JOBS=2 cargo build --release 2>&1 | grep -A3 '^warning' | grep -- '--> src/lab/'` → no output.
- [ ] **Step 4: Book guard with engine compare** — `NAS="/run/user/1000/gvfs/smb-share:server=192.168.4.50,share=share/Manan Pujara/Book-to-rust"; python3 scripts/check_book_untouched.py --base origin/Manan --engine-compare --csv "$NAS/idx_data.csv" --vix "$NAS/india_vix" --extra "$NAS/extra_data"` → `"problems": []`; copy its notes into the report. If `VAULT_BASKET_DATA_DIR` is available, run once more with it set.
- [ ] **Step 5: Customize fidelity on real data (read-only)** — build and start the engine on a free local port with the NAS index file (as in the Customize plan, Step 7): `/api/lab/fidelity?method=rsi2_4&ticker=NIFTY%2050` → `"reproduces":true`; stop the process.
- [ ] **Step 6: Phase 5 gl checks** — with the engine started as `VAULT_LAB_MARKET=gl` on staged gl data (or the unit tests if no staged file exists): a gl run reading another gl id answers 200 with `"provisional":true` and a `foreign` entry with `"alignment":"prior_date"` for different calendars; a portfolio spec answers 400 `unsupported` with `"provisional":true`; on the India engine a spec reading `IG:DAX` answers 400 `needs_data`. Saved gl strategies remain unpublishable (Phase 4 test still passes).
- [ ] **Step 7: Auth and gateway** — `cd Context/guru/webapp && node --test tests/contracts.test.mjs tests/lab-strategies.test.mjs tests/book-library.test.mjs && npm --prefix auth test` → pass; gateway tests unchanged and passing: `CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="${VAULT_RUNTIME_ROOT:-$HOME/.cache/strategy-vault-webapp}/test-target" cargo test --locked --manifest-path gateway/Cargo.toml`.
- [ ] **Step 8: Upload check** — `python3 scripts/check_upload.py` → `"findings": []`.
- [ ] **Step 9: Scope proof** — from the repo root:
  - `git diff --stat $BASE` lists only the files in "File structure" (plus this plan if edited);
  - `git diff --quiet $BASE -- Context/guru/backend/src/main.rs Context/guru/backend/src/connors.rs Context/guru/backend/src/books Context/guru/backend/src/readiness.rs Context/guru/backend/src/gate.rs Context/guru/backend/src/instrument_policy.rs Context/guru/backend/src/baskets.rs Context/guru/backend/src/accounting.rs Context/guru/backend/src/trade_validation.rs Context/guru/webapp/gateway Context/guru/frontend` → exit 0;
  - `git diff --name-only $BASE | grep -E 'package(-lock)?\.json|Cargo\.(toml|lock)|compose|Dockerfile|\.sql$'` → no output.
- [ ] **Step 10: Report** — draft the PR description (Nandini → Manan) in `.superpowers/pr/phase8.md` (git-excluded): test counts before/after, guard notes, gl checks, the optional real-data smoke result or "not available here", spec §14 assumptions, and the list of Phase 7 names actually used. Do not push; the user pushes.

---

## Self-review (done at planning time)

- **Spec coverage:** §4.1 grammar → Tasks 1, 3; §4.2 execution on the run instrument → Tasks 5, 12 (`a_market_filter_reads_the_index_and_trades_the_etf`); §4.3 alignment → Tasks 2, 4, 5; §4.4 timeframes across symbols → Task 4 (`truncating_either_instrument…`, weekly assertion); §4.5 pair indicators → Tasks 3, 4, 5; §5 universe/calendar/data end → Tasks 7, 11, 12; §6.1–6.4 → Tasks 6, 8–11; §6.5 statistics → Task 12; §6.6 optimize → Task 13; §6.9 survivorship → Tasks 7, 12; §6.10 bounds → Tasks 1, 6, 7, 12, 13; §7 markets → Tasks 5, 7, 12, 16; §8 transport → no code (gateway unchanged; Task 16 scope proof); §10 API → Tasks 5, 12, 13; §11 auth → Task 15; §12 testing → Tasks 2, 4, 9, 10, 14, 16.
- **Placeholders:** Tasks 12–13 describe the JSON body and helper functions in prose where the exact Phase 7 money field names are unknown at planning time; each names the Phase 7 field to read. Everything else carries code.
- **Type consistency:** `Foreign{symbol,vs,fixup}`, `Others{run,close_fills,frames}`, `series_in`, `compile_with`, `Align`, `alignment`, `align`, `Universe`, `Mode`, `Portfolio`, `Member{ticker,kind,frames,at}`, `Loaded`, `Lane{m,c,score}`, `Book`, `Size`, `Charges`, `PTrade`, `Ledger`, `Skipped` are used with the same names and fields in every task. `Others.frames` is an `Arc` from Task 4 on.
