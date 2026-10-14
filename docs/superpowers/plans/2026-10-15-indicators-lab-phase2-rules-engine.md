# Indicators Lab — Phase 2 (rules engine, simulator, lab routes, gateway) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let a researcher send a JSON rule spec (spec §4) for one admitted instrument and get a labelled, simulated long-only backtest or optimization from the engine through `/api/{in|us}/lab/{catalogue,series,run,optimize}`, with every book strategy result byte-identical and proven so by an extended guard.

**Architecture:** New engine files only under `Context/guru/backend/src/lab/` — `json.rs` (std-only JSON reader), `rules.rs` (spec parse/validate/optimize paths), `series.rs` (operands → daily-aligned series, compiled conditions), `simulate.rs` (next-open fills producing book-format `connors::Trade`), `api.rs` (routes). Because `lab` is a child module of the crate root, it calls `main.rs`'s private helpers read-only through `crate::` (the same way `accounting.rs` already does `use crate::{day_number,esc,num}`): `select_context`, `resolve_context`, `accounting::Costs::parse`, `trade_validation::check`, `equity_curve`, `financial_stats`/`stats`, `grid`, `MAX_COMBOS`, `FULL_COLS`/`PART_COLS` and `conclude_rank_with_outcomes`. No lab copies of book formulas are needed. `main.rs` gains exactly one match arm. The gateway gains four allowlist lines, a lab-optimization flag and a spec bound in `policy.rs`, plus three added lines in `gateway/src/main.rs`.

**Tech Stack:** Rust 2021 std only for the engine (`cargo test`); gateway keeps its existing crates (axum, serde_json); Python 3 std-lib for the guard; git.

**Spec:** `docs/superpowers/specs/2026-10-14-indicators-lab-design.md` (§2a book protection, §3 architecture, §4 rule spec, §7 error handling, §8 testing, §9 phase 2). Phase 1 plan for format and conventions: `docs/superpowers/plans/2026-10-14-indicators-lab-phase1-engine.md`.

**Planning evidence:** every code block below was compiled and its tests run against a scratch copy of `Nandini` at `ce57dd7` while writing this plan (engine: 677 tests incl. `VAULT_IDX` real-data tests, 0 failures; gateway: 6 policy tests; guard: 12 tests; one extended guard hash pass on NIFTY 50 data: 8,753 responses hashed in ~66 s including the release build). The release build added no `src/lab/` warnings.

## Global Constraints

- Branch Nandini; never main; no force-push; no git worktrees (user preference); commits without any Co-Authored-By trailer; never push (the user pushes).
- Book results never touched: protected files connors.rs, books/**, readiness.rs, gate.rs, instrument_policy.rs, baskets.rs, accounting.rs, trade_validation.rs, frontend compat/*, ResearchPages.tsx — no edits; main.rs, gateway policy.rs and custom-pages.ts additive only (the guard enforces removed lines).
- std-only Rust engine, no new crates; gateway may use its existing crates only.
- CARGO_BUILD_JOBS=2; full suites need VAULT_IDX="/run/user/1000/gvfs/smb-share:server=192.168.4.50,share=share/Manan Pujara/Book-to-rust/idx_data.csv" (read-only NAS; never write there). No whole-corpus backtests.
- NaN = no value; no panics on user input; results labelled simulated, never mixed with book verification.
- `main.rs` change in this phase is exactly one added line (Task 6). `gateway/src/main.rs` gets three added lines and no edited line (Task 9). No existing function, route or JSON field changes anywhere.
- Every lab response (success or error) carries `"label":"User-defined indicator strategy · simulated · not a book method"`; no lab response contains `gate`, `claim` or `readiness` fields.
- Raw data, `.env`, generated results stay outside Git. Smoke runs bind a free localhost port and are stopped afterwards.

## Review Focus

1. **Lookahead through a weekly/monthly operand or an offset:** a weekly value must not be usable on Monday–Thursday of its own week, and truncating the data must never change an earlier daily signal value — `weekly_operand_is_visible_only_after_the_week_closes`, `offsets_count_bars_of_their_own_timeframe` and `signals_never_use_later_bars` (every catalogue indicator × D/W/M) in Task 4.
2. **Wrong fill timing:** a signal filled on its own bar, an entry filled before the same bar's exit, a fill beyond the window, or a pyramided position — `entry_signal_at_close_fills_next_open`, `exit_fills_before_a_same_bar_reentry`, `entries_need_all_exits_any_and_no_pyramiding`, `last_bar_signal_and_pre_window_signal_do_not_trade`, `gaps_fill_at_the_gapped_open` in Task 5.
3. **A mistyped or hostile spec silently accepted, or crashing the engine:** typo'd fields ignored, reserved fields (short, limit/stop entries, stops/targets/trailing, scale-in, portfolio) simulated as something else, deep nesting or a panic dropping the connection — `malformed_input_is_invalid_and_names_the_field`, `reserved_fields_are_unsupported` (Task 3), `depth_is_bounded` (Task 2), `panics_become_typed_errors` (Task 6).
4. **Numbers published where data is missing:** an index without volume used for OBV/volume, an instrument of unknown type, dates outside admitted coverage, rollover costs without contracts, or a spec cost contradicting the query — `errors_are_typed_and_name_the_problem` (Task 7), `needs_volume_is_typed` (Task 4).
5. **Optimizer runaway or ranking drift from the book optimizer:** more than 50,000 combinations, an unbounded spec at the gateway, a degenerate split, or a ranking that is not `conclude_rank_with_outcomes` — `optimize_grid_is_capped_like_the_book_optimizer` (Task 3), `optimize_ranks_like_the_book_optimizer` (Task 8), `lab_specs_are_bounded_before_the_engine` (Task 9).

---

## Existing code this phase relies on (read-only)

| Item | Where | Used for |
|---|---|---|
| `struct Row` | `main.rs:22` | `Bars::from_rows` input (Phase 1) |
| `struct Run { trades, from, to, equity, bh }` (private fields, visible to child modules) | `main.rs:225` | lab backtest result fed to stats |
| `backtest()` | `main.rs:233` | pattern copied in `lab::api::backtest` minus `connors::run` (validation → equity → positivity check → B&H) |
| `equity_curve(close, trades, from, to, cost)` | `main.rs:243` | lab equity, gross equity |
| `years(ctx, from, to)` | `main.rs:299` | optimizer trade-rate floor |
| `stats(ctx, run, cost)` / `financial_stats(ctx, run, costs)` | `main.rs:303` / `main.rs:399` | lab statistics (identical formulas, no copy) |
| `num`, `stats_json` | `main.rs:427`, `main.rs:433` | JSON numbers (`esc` at `main.rs:430` does not escape control characters, so lab uses `lab::json::esc`) |
| `struct State`, `enum SelectedContext`, `.native()`, `.evidence()` | `main.rs:505`, `:512`, `:537`, `:538` | instrument identity |
| `preloaded_market(ticker)` | `main.rs:545` | market/currency of preloaded indices/ETFs |
| `select_context(st, q)` | `main.rs:557` | preloaded ticker or catalogued native stock (`basket:…` keys) |
| `source_provenance(st, q)` | `main.rs:566` | basket provenance in lab responses |
| `query()` / `pct_decode()` | `main.rs:595` | `spec=<url-encoded JSON>` arrives decoded (`+` → space; clients encode with `encodeURIComponent`) |
| `params_of(q)` | `main.rs:625` | `p.<key>` for `/api/lab/series` (drops non-numbers, so lab validates `p.*` first) |
| `resolve_context(ctx, q)` | `main.rs:630` | `from`/`to` → inclusive bar indices; also runs `Costs::parse` |
| `grid("min:max:step")` | `main.rs:770` | optimize ranges (≤ 1000 values, 1e-9 rounding) |
| `MAX_COMBOS`, `FULL_COLS`, `PART_COLS` | `main.rs:785–787` | optimizer cap and columns |
| `optimize()` | `main.rs:792` | query format `id`, `ticker`, `g.<key>=min:max:step`, `split`, `slim=1`; split rule (`main.rs:835`), threads (`:854`), trade-rate floor (`:887`), outcome columns (`:893`) |
| `RANK_STATS`, `MIN_TRADES`, `MIN_PER_YEAR`, `conclude_rank_with_outcomes` | `main.rs:915–917`, `:931` | identical ranking |
| `respond(stream, status, ctype, body)` | `main.rs:1000` | lab responses (the `json` closure at `:1050` returns text/plain errors, so lab has its own JSON error path) |
| `handle()` match, `"/api/notes"` arm | `main.rs:1006`, `:1093` | the one added arm goes immediately before `:1093` |
| `catalogue_json` / `params_json` / `readiness_json` / `detail` / `summary` | `main.rs:458` / `:754` / `:487` / `:697` / `:661` | guard hashing (Task 1) |
| `connors::{Trade, Leg}` | `connors.rs:375–420` | simulator output type |
| `trade_validation::check(trades, from, to, cost)` | `trade_validation.rs:3` | structural admission of lab trades |
| `accounting::Costs::{parse, side_rate, ensure_rollover, json}` | `accounting.rs:8–25` | `cost`/`slippage`/`rollover` bps per side |
| `instrument_policy::selected_type(ticker, native)` | `instrument_policy.rs:137` | instrument type label |
| `baskets::valid_date` (`pub(crate)`) | `baskets.rs:84` | date validation |
| `lab::catalogue::{all, get, resolve_params, run}`, `lab::resample::{Tf, resample, expand, day_number}` | Phase 1 | indicators and timeframes |

## Decisions (spec ambiguities resolved)

1. **Parameter keys** are the catalogue's (`length`, `fast`, …). The spec §4 example's `{"len":2}` is written `{"length":2}`; `len` is rejected as `unknown parameter 'len'`.
2. **Operand fields:** `{"s": id, "p": {..}, "tf": "D|W|M", "out": "<output name>", "offset": n}` or `{"value": number}`. `tf` defaults to `D`, `out` to the first output, `offset` to 0. `offset` is a whole number 0–1000 counted in bars of the operand's own timeframe (W offset 1 = previous completed week), applied before W/M values are spread to daily bars.
3. **Price operands** `open, high, low, close, hl2, hlc3, ohlc4, volume` are built in (they are not catalogue indicators); they take no `p` or `out`. `volume` on an instrument without volume is `needs_data`.
4. **`bars_held`** (exit conditions only) = daily bars since the entry fill, 0 on the fill bar (TradingView `bar_index - strategy.opentrades.entry_bar_index(0)`); NaN (false) when flat.
5. **Same-close exit and entry:** when a close fires an exit and the entry rule, the exit fills at the next open first, then the new entry fills at the same open (spec §4 "exits before entries"). Entry rules are evaluated only when flat or exiting, so there is never more than one position.
6. **Window edges:** signals are evaluated on bars `from..to-1`; an entry signal on `to` has no next open inside the window and is dropped; a position still open after `to` is marked at `close[to]` with `open: true`, `reason: "open"` (the book convention, `connors.rs:391`). Indicators warm up on the full admitted history before `from`.
7. **Costs:** `cost`/`slippage`/`rollover` query parameters through `accounting::Costs::parse` exactly as book routes. `execution.cost_bps` in a saved spec fills `cost` when the query has none; a different `cost` is `invalid` (no silent choice). `rollover > 0` is `needs_data` (`Costs::ensure_rollover`).
8. **Error contract (spec §7):** HTTP 400, `Content-Type: application/json`, body `{"status":"invalid|unsupported|needs_data","error":"…","label":"…"}`. Reserved fields → `unsupported`; malformed → `invalid` naming the field path (e.g. `entry.all[1].rhs.tf`); missing data (volume, intraday, coverage, unknown ticker, rollover) → `needs_data`. Trade-validation failures are `invalid` with the `calculation_unavailable:` text. A panic is caught and returned as `invalid`.
9. **Instrument identity:** type comes from `instrument_policy::selected_type` with `native` from the server catalogue. Type `unknown` (e.g. `NIFTYMIDCAPSELECT`, which `selected_type` does not list) → `run`/`optimize` refuse with `needs_data`; `series` (chart only) still answers and labels it `unknown`. No book gate, readiness or admission rule is applied to a lab spec.
10. **Coverage:** unlike book routes for preloaded tickers (which clamp), lab `from` before the first admitted bar or `to` after the last is `needs_data` for every instrument; dates must pass `baskets::valid_date`.
11. **Split:** a `split` that leaves ≤ 20 bars on either side is `invalid` (the book optimizer silently drops it, `main.rs:835`).
12. **Optimize response** mirrors `optimize()` but names the unoptimized row `base` (not `book`) and uses spec paths as `dims[].path` and leading `cols`; the "fewest changes" tie-break uses the spec's own values (`rules::current`).
13. **Bounds:** spec ≤ 8192 bytes, ≤ 12 conditions, JSON depth ≤ 16, grid ≤ 1000 values per path and ≤ 50,000 combinations (engine and gateway), optimizer wall clock 300 s in the engine (gateway timeout 600 s). Gateway spec numbers must be finite and ≤ 1e12 in size (volume thresholds exceed the 1e6 bound used for `p.*`). The gateway's 16,384-byte query cap (`valid_query`) still applies to the URL-encoded spec.
14. **Series cache** is per request (per optimizer thread), keyed `(id, tf, resolved params)`, holding all outputs on their own timeframe; it is cleared when it reaches 64 entries (memory bound).
15. **Integration test:** not added. `Context/guru/webapp/tests/integration.test.mjs` needs the deployed stack, the private `.env` (admin password) and engines loaded with the private NAS data, so it cannot run in the phase gate without private data. Gateway behaviour is pinned by `policy.rs` unit tests; the three gateway `main.rs` lines are shown in full for review.

## File structure

```
Context/guru/backend/src/lab/
  mod.rs        + pub mod api; json; rules; series; simulate;   (one line each, in their tasks)
  json.rs       Json enum, parse(), esc()                         (Task 2)
  rules.rs      LABEL, limits, LabError, Spec/Cond/Operand/SeriesRef/Dim/Target, parse_spec(), series_ref(), apply(), current()   (Task 3)
  series.rs     Frames, Cache, series(), Val, RCond, Compiled, compile(), describe()                                             (Task 4)
  simulate.rs   simulate() → Vec<connors::Trade>                  (Task 5)
  api.rs        serve(), route(), guarded(), catalogue/series/run/optimize JSON   (Tasks 6–8)
Context/guru/backend/src/main.rs                 + 1 match arm (Task 6)
Context/guru/webapp/gateway/src/policy.rs        + 4 allowlist lines, is_lab_optimization_path, lab_spec_problem, tests (Task 9)
Context/guru/webapp/gateway/src/main.rs          + 3 lines (Task 9)
scripts/check_book_untouched.py, scripts/test_check_book_untouched.py   extended engine compare (Task 1)
```

### Rule spec v1 as implemented (Tasks 3–5)

```json
{ "v": 1, "direction": "long",
  "entry": { "all": [
    { "lhs": {"s":"rsi","p":{"length":2},"tf":"D"}, "op":"crosses_below", "rhs": {"value":10} },
    { "lhs": {"s":"close","tf":"D"}, "op":">", "rhs": {"s":"sma","p":{"length":40},"tf":"W"} } ] },
  "exit": { "any": [
    { "lhs": {"s":"rsi","p":{"length":2},"tf":"D"}, "op":"crosses_above", "rhs": {"value":70} },
    { "lhs": {"s":"bars_held"}, "op":">=", "rhs": {"value":10} } ] },
  "execution": { "fill": "next_open", "cost_bps": 12 },
  "optimize": [ {"path":"entry.all[0].lhs.p.length","from":2,"to":6,"step":1} ] }
```

| Field | Rule |
|---|---|
| `v` | required, must be `1` |
| `direction` | optional; `"long"`; `"short"` → `unsupported` |
| `entry` | `{"all":[cond…]}`, ≥ 1; reserved `limit`, `stop`, `scale_in` → `unsupported` |
| `exit` | `{"any":[cond…]}`, ≥ 1; reserved `stop_loss`, `target`, `trailing` → `unsupported` |
| top level reserved | `short`, `portfolio`, `scale_in` → `unsupported` |
| `execution` | optional; `fill` `"next_open"` (other strings → `unsupported`); `cost_bps` 0–5000; reserved `limit`, `stop` |
| cond | `{"lhs","op","rhs"}`; `op` ∈ `crosses_above` (a>b and a[1]<=b[1]), `crosses_below`, `>`, `<`, `>=`, `<=`; two numbers → `invalid` |
| total conditions | entry + exit ≤ 12 |
| `optimize[]` | `{"path","from","to","step"}`; path `entry.all[i]|exit.any[i]` + `.lhs|.rhs` + `.p.<key>` (indicator param) or `.value` (number operand); each value validated by `resolve_params`; ≤ 1000 values each, ≤ 50,000 combinations; a path listed twice → `invalid` |
| any other key | `invalid`: `unknown field '<k>' in <path>` |

---

### Task 1: Extend the book guard to run, readiness and optimize outputs

**Files:**
- Modify: `scripts/check_book_untouched.py`
- Test: `scripts/test_check_book_untouched.py`

**Interfaces:**
- Consumes (engine, read-only HTTP): `/api/tickers`; `/api/guru/{g}/strategies` (`catalogue_json`, `main.rs:458`, rows `{"id","status":"live|needs_data|not_testable",…}`); `/api/guru/{g}/params` (`params_json`, `main.rs:754`, `{"<id>":[{"key","label","def","min","max","step"}]}` for Live strategies only); `/api/guru/{g}/summary?ticker=`; `/api/guru/{g}/run?id=&ticker=` (`detail`, `main.rs:697`); `/api/guru/{g}/readiness?id=&ticker=` (`readiness_json`, `main.rs:487`); `/api/guru/{g}/optimize?id=&ticker=&g.<key>=min:max:step` (`optimize`, `main.rs:792`; grid syntax `main.rs:770`).
- Produces: `fetch(url, timeout=600) -> (int, bytes)`; `strategy_routes(guru, catalogue, ticker="NIFTY 50") -> list[(key, path)]`; `optimize_route(guru, catalogue, params, ticker="NIFTY 50") -> (key, path) | None`; `stable(key, body) -> bytes` (drops the optimizer's wall-clock `"ms"`); `basket_tickers(tickers_json, limit=3) -> list[str]`; `engine_hashes(...) -> (dict[str,str], list[str])`; `compare_engines(...) -> (problems, notes)`. Each hash is `sha256(f"{status}\n" + body)`, so a blocked strategy's 400 text is protected too. CLI JSON gains `"notes"` (e.g. `"basket summary skipped: VAULT_BASKET_DATA_DIR is not set"`, `"ross: no live strategy with parameters; optimize not hashed"`, `"hashed N book responses"`).

- [ ] **Step 1: Write the failing tests** — add these methods to `class BookGuard` in `scripts/test_check_book_untouched.py` (after `test_preloaded_tickers_drops_basket_rows`):

```python
    def test_strategy_routes_cover_every_id_with_run_and_readiness(self):
        routes = guard.strategy_routes("connors", [{"id": "f1a"}, {"id": "r 3"}])
        self.assertEqual(routes, [
            ("connors/run?id=f1a", "run?id=f1a&ticker=NIFTY%2050"),
            ("connors/readiness?id=f1a", "readiness?id=f1a&ticker=NIFTY%2050"),
            ("connors/run?id=r 3", "run?id=r%203&ticker=NIFTY%2050"),
            ("connors/readiness?id=r 3", "readiness?id=r%203&ticker=NIFTY%2050"),
        ])

    def test_optimize_route_uses_first_live_strategy_with_params(self):
        catalogue = [{"id": "a", "status": "not_testable"}, {"id": "b", "status": "live"}, {"id": "c", "status": "live"}]
        params = {"a": [{"key": "x", "def": 1, "step": 1}], "b": [], "c": [{"key": "trend", "def": 200, "step": 25}, {"key": "hold", "def": 5, "step": 1}]}
        self.assertEqual(guard.optimize_route("connors", catalogue, params),
                         ("connors/optimize?id=c&g.trend=200:225:25", "optimize?id=c&ticker=NIFTY%2050&g.trend=200:225:25"))
        self.assertEqual(guard.optimize_route("chande", [{"id": "d", "status": "live"}], {"d": [{"key": "k", "def": 0.1, "step": 0.2}]})[0],
                         "chande/optimize?id=d&g.k=0.1:0.3:0.2")
        self.assertIsNone(guard.optimize_route("ross", catalogue, {}))

    def test_optimizer_timing_is_not_hashed(self):
        a = guard.stable("connors/optimize?id=c", b'{"id":"c","ms":12,"rows":[[1]]}')
        b = guard.stable("connors/optimize?id=c", b'{"rows":[[1]],"ms":95,"id":"c"}')
        self.assertEqual(a, b)
        self.assertEqual(guard.stable("connors/run?id=c", b'{"ms":1}'), b'{"ms":1}')
        self.assertEqual(guard.stable("connors/optimize?id=c", b"not json"), b"not json")

    def test_basket_tickers_are_bounded_and_sorted(self):
        rows = [{"ticker": "NIFTY 50"}, {"ticker": "basket:b:Z", "basket": "b"}, {"ticker": "basket:a:Y", "basket": "a"},
                {"ticker": "basket:a:X", "basket": "a"}, {"ticker": "basket:c:W", "basket": "c"}]
        self.assertEqual(guard.basket_tickers(rows), ["basket:a:X", "basket:a:Y", "basket:b:Z"])
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `python3 -m unittest scripts/test_check_book_untouched.py -v`
Expected: the 4 new tests ERROR with `AttributeError: module 'check_book_untouched' has no attribute 'strategy_routes'` (and siblings); the 8 existing tests pass.

- [ ] **Step 3: Implement**

In `scripts/check_book_untouched.py`:

1. Replace the docstring line `Optional --engine-compare: build base and HEAD engines and compare book catalogue/summary JSON hashes.` with:
```python
Optional --engine-compare: build base and HEAD engines and compare hashes of book catalogue, params, summary,
run and readiness (every strategy, NIFTY 50, default parameters) and one small optimize grid per guru.
```
2. Add `urllib.error` to the import line:
```python
import argparse, hashlib, io, json, os, socket, subprocess, sys, tarfile, tempfile, time, urllib.error, urllib.parse, urllib.request
```
3. Insert these helpers after `free_port()`:
```python
def fetch(url, timeout=600):
    """(HTTP status, body). A blocked strategy's 400 message is book output too, so error bodies are kept."""
    try:
        with urllib.request.urlopen(url, timeout=timeout) as r:
            return r.status, r.read()
    except urllib.error.HTTPError as e:
        return e.code, e.read()


def strategy_routes(guru, catalogue, ticker="NIFTY 50"):
    """(hash key, URL path) for run and readiness of every strategy id, default parameters, one ticker."""
    t = urllib.parse.quote(ticker)
    out = []
    for row in catalogue:
        sid = row["id"]
        q = urllib.parse.quote(sid)
        out.append((f"{guru}/run?id={sid}", f"run?id={q}&ticker={t}"))
        out.append((f"{guru}/readiness?id={sid}", f"readiness?id={q}&ticker={t}"))
    return out


def optimize_route(guru, catalogue, params, ticker="NIFTY 50"):
    """One two-value grid (default and default + step of the first parameter) for the first live strategy with params."""
    live = {row["id"] for row in catalogue if row.get("status") == "live"}
    for sid, defs in params.items():
        if sid in live and defs:
            p = defs[0]
            grid = f"{p['def']:.12g}:{p['def'] + p['step']:.12g}:{p['step']:.12g}"
            return (f"{guru}/optimize?id={sid}&g.{p['key']}={grid}",
                    f"optimize?id={urllib.parse.quote(sid)}&ticker={urllib.parse.quote(ticker)}&g.{p['key']}={grid}")
    return None


def stable(key, body):
    """Optimizer responses carry their run time in "ms"; drop it so equal results hash equal."""
    if "/optimize?" not in key:
        return body
    try:
        data = json.loads(body)
    except ValueError:
        return body
    data.pop("ms", None)
    return json.dumps(data, sort_keys=True).encode()


def basket_tickers(tickers_json, limit=3):
    # Bounded: a few catalogued stocks, never the whole basket corpus.
    return sorted(row["ticker"] for row in tickers_json if "basket" in row)[:limit]
```
4. Replace the whole `engine_hashes` function with:
```python
def engine_hashes(src_dir, csv, vix, extra):
    env = dict(os.environ, CARGO_BUILD_JOBS="2")
    subprocess.run(["cargo", "build", "--release", "--manifest-path", f"{src_dir}/Context/guru/backend/Cargo.toml"], check=True, env=env)
    port = free_port()
    proc = subprocess.Popen([f"{src_dir}/Context/guru/backend/target/release/guru", csv, str(port), vix, extra], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    try:
        for _ in range(600):
            try:
                urllib.request.urlopen(f"http://127.0.0.1:{port}/api/gurus", timeout=2).read()
                break
            except OSError:
                time.sleep(0.5)
        api = f"http://127.0.0.1:{port}/api"
        tickers_json = json.loads(fetch(f"{api}/tickers", 60)[1])
        tickers = preloaded_tickers(tickers_json)
        notes = []
        if os.environ.get("VAULT_BASKET_DATA_DIR"):
            tickers += basket_tickers(tickers_json)
        else:
            notes.append("basket summary skipped: VAULT_BASKET_DATA_DIR is not set")
        out = {}

        def put(key, route):
            status, body = fetch(f"{api}/guru/{route}")
            if not body:
                raise RuntimeError(f"empty response for {route}")
            out[key] = hashlib.sha256(f"{status}\n".encode() + stable(key, body)).hexdigest()
            return body

        for g in GURUS:
            catalogue = json.loads(put(f"{g}/strategies", f"{g}/strategies"))
            params = json.loads(put(f"{g}/params", f"{g}/params"))
            for t in tickers:
                put(f"{g}/summary?ticker={t}", f"{g}/summary?ticker={urllib.parse.quote(t)}")
            for key, route in strategy_routes(g, catalogue):
                put(key, f"{g}/{route}")
            opt = optimize_route(g, catalogue, params)
            if opt:
                put(opt[0], f"{g}/{opt[1]}")
            else:
                notes.append(f"{g}: no live strategy with parameters; optimize not hashed")
        return out, notes
    finally:
        proc.terminate()
        proc.wait()
```
5. Replace `compare_engines` with:
```python
def compare_engines(repo, base, csv, vix, extra):
    # Plain export of the base commit into a temp folder (no git worktree, per user preference).
    with tempfile.TemporaryDirectory() as tmp:
        archive = subprocess.run(["git", "-C", str(repo), "archive", compare_base(repo, base)], capture_output=True, check=True).stdout
        with tarfile.open(fileobj=io.BytesIO(archive)) as tar:
            tar.extractall(tmp, filter="data")
        before, _ = engine_hashes(tmp, csv, vix, extra)
    after, notes = engine_hashes(repo, csv, vix, extra)
    return [f"book output changed: {k}" for k in before if before[k] != after.get(k)], notes + [f"hashed {len(before)} book responses"]
```
6. Replace `main` with:
```python
def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--base", default="origin/Manan")
    ap.add_argument("--engine-compare", action="store_true")
    ap.add_argument("--csv"); ap.add_argument("--vix"); ap.add_argument("--extra")
    a = ap.parse_args()
    repo = Path(git(Path.cwd(), "rev-parse", "--show-toplevel").strip())
    problems = check(repo, a.base)
    notes = []
    if a.engine_compare:
        if not (a.csv and a.vix and a.extra):
            ap.error("--engine-compare needs --csv, --vix and --extra (read-only data paths)")
        changed, notes = compare_engines(repo, a.base, a.csv, a.vix, a.extra)
        problems += changed
    print(json.dumps({"base": a.base, "engine_compared": a.engine_compare, "notes": notes, "problems": problems}, indent=2))
    sys.exit(1 if problems else 0)
```

The base engine is the merge-base export (Phase 1 behaviour, no worktree); the engine process inherits the environment, so `VAULT_BASKET_DATA_DIR` reaches both engines when set, and at most three catalogued stocks get a summary hash.

- [ ] **Step 4: Run tests and the cheap guard**

Run: `python3 -m unittest scripts/test_check_book_untouched.py -v` → Expected: 12 tests OK.
Run: `python3 scripts/check_book_untouched.py --base origin/Manan` → Expected: `"notes": []`, `"problems": []`, exit 0.
Optional now (mandatory in Task 10): `python3 scripts/check_book_untouched.py --base origin/Manan --engine-compare --csv "$NAS/idx_data.csv" --vix "$NAS/india_vix" --extra "$NAS/extra_data"` with `NAS="/run/user/1000/gvfs/smb-share:server=192.168.4.50,share=share/Manan Pujara/Book-to-rust"` → Expected: `"problems": []`, notes include the basket-skip line (unless `VAULT_BASKET_DATA_DIR` is set), the ross line and `hashed N book responses` (≈ 8,750 on the current catalogue). Two release builds; about 2–3 minutes.

- [ ] **Step 5: Commit**

```bash
git add scripts/check_book_untouched.py scripts/test_check_book_untouched.py
git commit -m "Guard book run, readiness and optimize outputs in the engine compare"
```

---

### Task 2: Std-only JSON reader for rule specs

**Files:**
- Create: `Context/guru/backend/src/lab/json.rs` (+ `pub mod json;` in `lab/mod.rs`, after `pub mod catalogue;`)

**Interfaces:**
- Produces: `#[derive(Clone, Debug, PartialEq)] pub enum Json { Null, Bool(bool), Num(f64), Str(String), Arr(Vec<Json>), Obj(Vec<(String, Json)>) }`; `impl Json { pub fn get(&self, key: &str) -> Option<&Json>; pub fn as_f64(&self) -> Option<f64>; pub fn as_str(&self) -> Option<&str> }`; `pub fn parse(text: &str) -> Result<Json, String>` (errors say what and the byte offset; duplicate keys, non-finite numbers, depth ≥ 16, control characters in strings, non-BMP `\u` escapes and trailing text are errors; never panics); `pub fn esc(s: &str) -> String` (escapes `"`, `\` and every control character as `\u00XX`; used for every user-derived string the lab writes).

- [ ] **Step 1: Write the failing tests** (bottom of `json.rs`)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn parses_spec_shapes_in_order() {
        let v = parse(r#" {"v":1,"a":[true,false,null,-2.5e1,"x\"y\u00e9"],"o":{}} "#).unwrap();
        assert_eq!(v.get("v").and_then(Json::as_f64), Some(1.));
        assert_eq!(v.get("a"), Some(&Json::Arr(vec![Json::Bool(true), Json::Bool(false), Json::Null, Json::Num(-25.), Json::Str("x\"yé".into())])));
        assert_eq!(v.get("o"), Some(&Json::Obj(vec![])));
        if let Json::Obj(kv) = &v { assert_eq!(kv.iter().map(|(k, _)| k.as_str()).collect::<Vec<_>>(), ["v", "a", "o"]); }
    }
    #[test] fn rejects_malformed_input_without_panicking() {
        for bad in ["", "{", "[1,]", "{\"a\":1,}", "{\"a\" 1}", "{\"a\":1}x", "\"abc", "{\"a\":1,\"a\":2}", "1e999", "-", "nul", "[\"\\ud800\"]", "\"a\u{1}\"", "{a:1}", "\"\\x\""] {
            assert!(parse(bad).is_err(), "{bad:?} should fail");
        }
    }
    #[test] fn depth_is_bounded() {
        assert!(parse(&"[".repeat(17).chars().chain("]".repeat(17).chars()).collect::<String>()).is_err());
        assert!(parse(&format!("{}{}", "[".repeat(16), "]".repeat(16))).is_ok());
        assert!(parse(&"[".repeat(100_000)).is_err());
    }
    #[test] fn esc_escapes_controls() { assert_eq!(esc("a\"b\\c\nd"), "a\\\"b\\\\c\\u000ad"); }
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cd Context/guru/backend && CARGO_BUILD_JOBS=2 cargo test lab::json` → Expected: compile error (`parse`, `Json`, `esc` not found).

- [ ] **Step 3: Implement** (top of `json.rs`, above the tests)

```rust
//! Minimal JSON reader for lab rule specs (std only). Objects keep key order; duplicate keys, non-finite
//! numbers, nesting deeper than MAX_DEPTH and trailing text are errors. Writers use `esc` for every user string.
#[derive(Clone, Debug, PartialEq)]
pub enum Json { Null, Bool(bool), Num(f64), Str(String), Arr(Vec<Json>), Obj(Vec<(String, Json)>) }

const MAX_DEPTH: usize = 16;

impl Json {
    pub fn get(&self, key: &str) -> Option<&Json> {
        match self { Json::Obj(kv) => kv.iter().find(|(k, _)| k == key).map(|(_, v)| v), _ => None }
    }
    pub fn as_f64(&self) -> Option<f64> { if let Json::Num(n) = self { Some(*n) } else { None } }
    pub fn as_str(&self) -> Option<&str> { if let Json::Str(s) = self { Some(s) } else { None } }
}

pub fn parse(text: &str) -> Result<Json, String> {
    let mut p = Parser { b: text.as_bytes(), i: 0 };
    let v = p.value(0)?;
    p.ws();
    if p.i != p.b.len() { return Err(format!("unexpected text at byte {}", p.i)); }
    Ok(v)
}

/// JSON string body escaping (quotes, backslash and every control character).
pub fn esc(s: &str) -> String {
    let mut o = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            c if (c as u32) < 0x20 => o.push_str(&format!("\\u{:04x}", c as u32)),
            c => o.push(c),
        }
    }
    o
}

struct Parser<'a> { b: &'a [u8], i: usize }
impl Parser<'_> {
    fn ws(&mut self) { while self.i < self.b.len() && matches!(self.b[self.i], b' ' | b'\t' | b'\n' | b'\r') { self.i += 1; } }
    fn err<T>(&self, what: &str) -> Result<T, String> { Err(format!("{what} at byte {}", self.i)) }
    fn lit(&mut self, word: &str, v: Json) -> Result<Json, String> {
        if self.b[self.i..].starts_with(word.as_bytes()) { self.i += word.len(); Ok(v) } else { self.err("invalid literal") }
    }
    fn value(&mut self, depth: usize) -> Result<Json, String> {
        if depth >= MAX_DEPTH { return self.err("nesting too deep"); }
        self.ws();
        match self.b.get(self.i) {
            None => self.err("unexpected end"),
            Some(b'{') => {
                self.i += 1;
                let mut kv: Vec<(String, Json)> = Vec::new();
                self.ws();
                if self.b.get(self.i) == Some(&b'}') { self.i += 1; return Ok(Json::Obj(kv)); }
                loop {
                    self.ws();
                    if self.b.get(self.i) != Some(&b'"') { return self.err("expected a quoted key"); }
                    let k = self.string()?;
                    if kv.iter().any(|(x, _)| *x == k) { return Err(format!("duplicate key '{k}'")); }
                    self.ws();
                    if self.b.get(self.i) != Some(&b':') { return self.err("expected ':'"); }
                    self.i += 1;
                    let v = self.value(depth + 1)?;
                    kv.push((k, v));
                    self.ws();
                    match self.b.get(self.i) {
                        Some(b',') => self.i += 1,
                        Some(b'}') => { self.i += 1; return Ok(Json::Obj(kv)); }
                        _ => return self.err("expected ',' or '}'"),
                    }
                }
            }
            Some(b'[') => {
                self.i += 1;
                let mut a = Vec::new();
                self.ws();
                if self.b.get(self.i) == Some(&b']') { self.i += 1; return Ok(Json::Arr(a)); }
                loop {
                    a.push(self.value(depth + 1)?);
                    self.ws();
                    match self.b.get(self.i) {
                        Some(b',') => self.i += 1,
                        Some(b']') => { self.i += 1; return Ok(Json::Arr(a)); }
                        _ => return self.err("expected ',' or ']'"),
                    }
                }
            }
            Some(b'"') => Ok(Json::Str(self.string()?)),
            Some(b't') => self.lit("true", Json::Bool(true)),
            Some(b'f') => self.lit("false", Json::Bool(false)),
            Some(b'n') => self.lit("null", Json::Null),
            Some(c) if *c == b'-' || c.is_ascii_digit() => {
                let start = self.i;
                while self.i < self.b.len() && matches!(self.b[self.i], b'0'..=b'9' | b'-' | b'+' | b'.' | b'e' | b'E') { self.i += 1; }
                let s = std::str::from_utf8(&self.b[start..self.i]).unwrap_or("");
                match s.parse::<f64>() {
                    Ok(n) if n.is_finite() => Ok(Json::Num(n)),
                    _ => Err(format!("invalid number '{s}' at byte {start}")),
                }
            }
            Some(_) => self.err("unexpected character"),
        }
    }
    fn string(&mut self) -> Result<String, String> {
        self.i += 1; // opening quote
        let mut out: Vec<u8> = Vec::new();
        loop {
            let Some(&c) = self.b.get(self.i) else { return self.err("unterminated string") };
            self.i += 1;
            match c {
                b'"' => return String::from_utf8(out).map_err(|_| "string is not UTF-8".to_string()),
                b'\\' => {
                    let Some(&e) = self.b.get(self.i) else { return self.err("unterminated escape") };
                    self.i += 1;
                    match e {
                        b'"' => out.push(b'"'), b'\\' => out.push(b'\\'), b'/' => out.push(b'/'),
                        b'b' => out.push(8), b'f' => out.push(12), b'n' => out.push(b'\n'), b'r' => out.push(b'\r'), b't' => out.push(b'\t'),
                        b'u' => {
                            let hex = self.b.get(self.i..self.i + 4).and_then(|h| std::str::from_utf8(h).ok()).and_then(|h| u32::from_str_radix(h, 16).ok());
                            // shortcut: surrogate pairs (non-BMP \u escapes) are rejected; spec ids and keys are ASCII.
                            let Some(ch) = hex.and_then(char::from_u32) else { return self.err("unsupported \\u escape") };
                            self.i += 4;
                            out.extend_from_slice(ch.to_string().as_bytes());
                        }
                        _ => return self.err("invalid escape"),
                    }
                }
                c if c < 0x20 => return self.err("control character in string"),
                c => out.push(c),
            }
        }
    }
}
```

- [ ] **Step 4: Run tests** — `CARGO_BUILD_JOBS=2 cargo test lab::json` → Expected: 4 passed.

- [ ] **Step 5: Guard + commit**

```bash
python3 scripts/check_book_untouched.py --base origin/Manan
git add Context/guru/backend/src/lab/json.rs Context/guru/backend/src/lab/mod.rs
git commit -m "Add std-only JSON reader for lab rule specs"
```

---

### Task 3: Rule spec parsing and validation

**Files:**
- Create: `Context/guru/backend/src/lab/rules.rs` (+ `pub mod rules;` in `lab/mod.rs`, after `pub mod resample;`)

**Interfaces:**
- Consumes: `json::{parse, Json}` (Task 2); `catalogue::{get, resolve_params, Params}`, `resample::Tf` (Phase 1); `crate::grid` (`main.rs:770`), `crate::MAX_COMBOS` (`main.rs:785`).
- Produces:
  - `pub const LABEL: &str = "User-defined indicator strategy · simulated · not a book method";` `MAX_CONDITIONS = 12`, `MAX_SPEC_BYTES = 8192`, `MAX_OFFSET = 1000`, `PRICE_SOURCES`, `OPERATORS`.
  - `#[derive(Clone, Debug, PartialEq)] pub enum LabError { Invalid(String), Unsupported(String), NeedsData(String) }` with `status() -> &'static str` (`"invalid" | "unsupported" | "needs_data"`), `message() -> &str`, `json() -> String` (`{"status","error","label"}`), and `impl From<String>` (`"needs_data: …"` → `NeedsData`, anything else → `Invalid`) so engine helpers' `Result<_, String>` work with `?`.
  - `pub enum Op { CrossesAbove, CrossesBelow, Gt, Lt, Ge, Le }`; `pub struct SeriesRef { pub id: String, pub params: Params, pub tf: Tf, pub out: usize, pub offset: usize }` (params already resolved to the full declared set); `pub enum Operand { Series(SeriesRef), Value(f64), BarsHeld }`; `pub struct Cond { pub lhs: Operand, pub op: Op, pub rhs: Operand }`; `pub struct Target { pub exit: bool, pub cond: usize, pub rhs: bool, pub param: Option<String> }`; `pub struct Dim { pub path: String, pub target: Target, pub values: Vec<f64> }`; `pub struct Spec { pub entry: Vec<Cond>, pub exit: Vec<Cond>, pub cost_bps: Option<f64>, pub optimize: Vec<Dim> }`.
  - `pub fn parse_spec(text: &str) -> Result<Spec, LabError>`; `pub fn series_ref(at: &str, id: &str, given: &Params, tf: Tf, out: Option<&str>, offset: usize) -> Result<SeriesRef, LabError>` (shared with `/api/lab/series`); `pub fn apply(spec: &Spec, dims: &[Dim], values: &[f64]) -> Spec`; `pub fn current(spec: &Spec, t: &Target) -> f64`.

- [ ] **Step 1: Write the failing tests** (bottom of `rules.rs`)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    const EXAMPLE: &str = r#"{ "v": 1, "direction": "long",
      "entry": { "all": [
        { "lhs": {"s":"rsi","p":{"length":2},"tf":"D"}, "op":"crosses_below", "rhs": {"value":10} },
        { "lhs": {"s":"close","tf":"D"}, "op":">", "rhs": {"s":"sma","p":{"length":40},"tf":"W"} } ] },
      "exit": { "any": [
        { "lhs": {"s":"rsi","p":{"length":2},"tf":"D"}, "op":"crosses_above", "rhs": {"value":70} },
        { "lhs": {"s":"bars_held"}, "op":">=", "rhs": {"value":10} } ] },
      "execution": { "fill": "next_open", "cost_bps": 12 },
      "optimize": [ {"path":"entry.all[0].lhs.p.length","from":2,"to":6,"step":1} ] }"#;
    fn with(old: &str, new: &str) -> String { assert!(EXAMPLE.contains(old), "{old}"); EXAMPLE.replacen(old, new, 1) }
    fn err(text: &str) -> LabError { parse_spec(text).unwrap_err() }

    #[test] fn spec_example_round_trips() {
        let s = parse_spec(EXAMPLE).unwrap();
        assert_eq!((s.entry.len(), s.exit.len(), s.cost_bps), (2, 2, Some(12.)));
        let Operand::Series(r) = &s.entry[1].rhs else { panic!() };
        assert_eq!((r.id.as_str(), r.tf, r.params["length"]), ("sma", Tf::W, 40.));
        assert_eq!(s.exit[1].lhs, Operand::BarsHeld);
        assert_eq!(s.optimize[0].values, vec![2., 3., 4., 5., 6.]);
        assert_eq!(current(&s, &s.optimize[0].target), 2.);
        let t = apply(&s, &s.optimize, &[5.]);
        let Operand::Series(r) = &t.entry[0].lhs else { panic!() };
        assert_eq!(r.params["length"], 5.);
    }
    #[test] fn defaults_fill_params_tf_and_output() {
        let s = parse_spec(&with(r#"{"s":"close","tf":"D"}"#, r#"{"s":"macd","out":"signal"}"#)).unwrap();
        let Operand::Series(r) = &s.entry[1].lhs else { panic!() };
        assert_eq!((r.tf, r.out, r.params["fast"], r.params["slow"]), (Tf::D, 1, 12., 26.));
    }
    #[test] fn malformed_input_is_invalid_and_names_the_field() {
        for (text, needle) in [
            (with(r#""v": 1"#, r#""v": 2"#), "v must be 1"),
            (with(r#""op":"crosses_below""#, r#""op":"below""#), "entry.all[0].op"),
            (with(r#""tf":"W""#, r#""tf":"H""#), "entry.all[1].rhs.tf"),
            (with(r#"{"length":40}"#, r#"{"length":0}"#), "entry.all[1].rhs.p: sma: length"),
            (with(r#"{"length":40}"#, r#"{"len":40}"#), "unknown parameter 'len'"),
            (with(r#""s":"sma""#, r#""s":"smaa""#), "unknown series 'smaa'"),
            (with(r#""rhs": {"value":10}"#, r#""rhs": {"value":10,"offset":1}"#), "unknown field 'offset' in entry.all[0].rhs"),
            (with(r#"{"s":"close","tf":"D"}"#, r#"{"s":"close","offset":-1}"#), "entry.all[1].lhs.offset"),
            (with(r#"{"s":"close","tf":"D"}"#, r#"{"s":"close","offset":1.5}"#), "entry.all[1].lhs.offset"),
            (with(r#"{"s":"close","tf":"D"}"#, r#"{"s":"bars_held"}"#), "only available in exit"),
            (with(r#"{"s":"rsi","p":{"length":2},"tf":"D"}"#, r#"{"value":3}"#), "compares two numbers"),
            (with(r#"{"s":"close","tf":"D"}"#, r#"{"s":"macd","out":"sig"}"#), "macd has outputs macd, signal, hist"),
            (with(r#""entry""#, r#""entri""#), "unknown field 'entri' in spec"),
            (with(r#""all": ["#, r#""all": [], "x": ["#), "unknown field 'x' in entry"),
            ("{\"v\":1}".to_string(), "entry is required"),
            ("not json".to_string(), "not valid JSON"),
            (format!("{{\"v\":1,\"pad\":\"{}\"}}", "x".repeat(MAX_SPEC_BYTES)), "the limit is 8192"),
            (with("\"from\":2,\"to\":6", "\"from\":0,\"to\":6"), "optimize[0]: rsi: length must be between"),
            (with("entry.all[0].lhs.p.length", "entry.all[0].lhs.p.bogus"), "rsi has no parameter 'bogus'"),
            (with("entry.all[0].lhs.p.length", "entry.all[1].lhs.p.length"), "price sources have no parameters"),
            (with("entry.all[0].lhs.p.length", "entry.all[9].lhs.p.length"), "no condition at that index"),
            (with("\"step\":1", "\"step\":0"), "bad range"),
        ] {
            let e = err(&text);
            assert_eq!(e.status(), "invalid", "{text}: {e:?}");
            assert!(e.message().contains(needle), "{needle:?} not in {:?}", e.message());
        }
    }
    #[test] fn reserved_fields_are_unsupported() {
        for (text, needle) in [
            (with(r#""direction": "long""#, r#""direction": "short""#), "short"),
            (with(r#""v": 1,"#, r#""v": 1, "portfolio": {"max_open": 3},"#), "spec.portfolio"),
            (with(r#""all": ["#, r#""limit": {"value": 1}, "all": ["#), "entry.limit"),
            (with(r#""any": ["#, r#""stop_loss": {"pct": 2}, "any": ["#), "exit.stop_loss"),
            (with(r#""any": ["#, r#""trailing": {"pct": 2}, "any": ["#), "exit.trailing"),
            (with(r#""fill": "next_open""#, r#""fill": "limit""#), "execution.fill 'limit'"),
            (with(r#""v": 1,"#, r#""v": 1, "scale_in": [],"#), "spec.scale_in"),
        ] {
            let e = err(&text);
            assert_eq!(e.status(), "unsupported", "{text}: {e:?}");
            assert!(e.message().contains(needle), "{needle:?} not in {:?}", e.message());
        }
    }
    #[test] fn intraday_and_condition_limits() {
        let e = err(&with(r#""s":"sma""#, r#""s":"vwap""#));
        assert_eq!(e.status(), "needs_data");
        let cond = r#"{ "lhs": {"s":"close"}, "op":">", "rhs": {"value":1} }"#;
        let many = format!(r#"{{"v":1,"entry":{{"all":[{}]}},"exit":{{"any":[{}]}}}}"#, vec![cond; 7].join(","), vec![cond; 6].join(","));
        assert!(err(&many).message().contains("13 conditions; the limit is 12"));
    }
    #[test] fn optimize_grid_is_capped_like_the_book_optimizer() {
        let big = with(r#"[ {"path":"entry.all[0].lhs.p.length","from":2,"to":6,"step":1} ]"#,
            r#"[ {"path":"entry.all[0].lhs.p.length","from":2,"to":301,"step":1}, {"path":"entry.all[1].rhs.p.length","from":1,"to":200,"step":1} ]"#);
        assert!(err(&big).message().contains("more than 50000 combinations"));
        let twice = with(r#"{"path":"entry.all[0].lhs.p.length","from":2,"to":6,"step":1}"#, r#"{"path":"entry.all[0].lhs.p.length","from":2,"to":3,"step":1},{"path":"entry.all[0].lhs.p.length","from":2,"to":3,"step":1}"#);
        assert!(err(&twice).message().contains("listed twice"));
        let value = parse_spec(&with("entry.all[0].lhs.p.length\",\"from\":2,\"to\":6", "entry.all[0].rhs.value\",\"from\":5,\"to\":15")).unwrap();
        assert_eq!((value.optimize[0].values.len(), current(&value, &value.optimize[0].target)), (11, 10.));
    }
    #[test] fn engine_errors_map_to_types() {
        assert_eq!(LabError::from("needs_data: no VIX".to_string()), LabError::NeedsData("no VIX".into()));
        assert_eq!(LabError::from("empty date range".to_string()).status(), "invalid");
        let j = LabError::Invalid("bad \"x\"\n".into()).json();
        assert!(j.starts_with("{\"status\":\"invalid\",\"error\":\"bad \\\"x\\\"\\u000a\",\"label\":\"User-defined indicator strategy"), "{j}");
    }
}
```

- [ ] **Step 2: Run to verify failure** — `CARGO_BUILD_JOBS=2 cargo test lab::rules` → Expected: compile error (items missing).

- [ ] **Step 3: Implement** (top of `rules.rs`)

```rust
//! Rule spec v1 (indicators lab spec §4): parse, validate against the catalogue, expand optimize grids.
//! Reserved fields return `Unsupported`; malformed input returns `Invalid` naming the field.
use super::catalogue::{self, Params};
use super::json::{self, Json};
use super::resample::Tf;

pub const LABEL: &str = "User-defined indicator strategy · simulated · not a book method";
pub const MAX_CONDITIONS: usize = 12;
pub const MAX_SPEC_BYTES: usize = 8192;
pub const MAX_OFFSET: usize = 1000;
pub const PRICE_SOURCES: &[&str] = &["open", "high", "low", "close", "hl2", "hlc3", "ohlc4", "volume"];
pub const OPERATORS: &[&str] = &["crosses_above", "crosses_below", ">", "<", ">=", "<="];

#[derive(Clone, Debug, PartialEq)]
pub enum LabError { Invalid(String), Unsupported(String), NeedsData(String) }
impl LabError {
    pub fn status(&self) -> &'static str {
        match self { Self::Invalid(_) => "invalid", Self::Unsupported(_) => "unsupported", Self::NeedsData(_) => "needs_data" }
    }
    pub fn message(&self) -> &str { match self { Self::Invalid(m) | Self::Unsupported(m) | Self::NeedsData(m) => m } }
    pub fn json(&self) -> String {
        format!("{{\"status\":\"{}\",\"error\":\"{}\",\"label\":\"{LABEL}\"}}", self.status(), json::esc(self.message()))
    }
}
/// Engine helpers report data gaps as "needs_data: …"; any other engine error is an invalid request.
impl From<String> for LabError {
    fn from(e: String) -> Self {
        match e.strip_prefix("needs_data:") { Some(m) => Self::NeedsData(m.trim().into()), None => Self::Invalid(e) }
    }
}
fn inv(m: impl Into<String>) -> LabError { LabError::Invalid(m.into()) }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Op { CrossesAbove, CrossesBelow, Gt, Lt, Ge, Le }
#[derive(Clone, Debug, PartialEq)]
pub struct SeriesRef { pub id: String, pub params: Params, pub tf: Tf, pub out: usize, pub offset: usize }
#[derive(Clone, Debug, PartialEq)]
pub enum Operand { Series(SeriesRef), Value(f64), BarsHeld }
#[derive(Clone, Debug, PartialEq)]
pub struct Cond { pub lhs: Operand, pub op: Op, pub rhs: Operand }
/// Where an optimize path points: entry/exit list, condition index, side, and parameter key (None = `value`).
#[derive(Clone, Debug, PartialEq)]
pub struct Target { pub exit: bool, pub cond: usize, pub rhs: bool, pub param: Option<String> }
#[derive(Clone, Debug, PartialEq)]
pub struct Dim { pub path: String, pub target: Target, pub values: Vec<f64> }
#[derive(Clone, Debug, PartialEq)]
pub struct Spec { pub entry: Vec<Cond>, pub exit: Vec<Cond>, pub cost_bps: Option<f64>, pub optimize: Vec<Dim> }

pub fn parse_spec(text: &str) -> Result<Spec, LabError> {
    if text.len() > MAX_SPEC_BYTES { return Err(inv(format!("spec is {} bytes; the limit is {MAX_SPEC_BYTES}", text.len()))); }
    let root = json::parse(text).map_err(|e| inv(format!("spec is not valid JSON: {e}")))?;
    fields(&root, "spec", &["v", "direction", "entry", "exit", "execution", "optimize"], &["short", "portfolio", "scale_in"])?;
    if root.get("v").and_then(Json::as_f64) != Some(1.) { return Err(inv("v must be 1")); }
    match root.get("direction") {
        None => {}
        Some(Json::Str(d)) if d == "long" => {}
        Some(Json::Str(d)) if d == "short" => return Err(LabError::Unsupported("direction 'short' is reserved; v1 simulates long only".into())),
        _ => return Err(inv("direction must be \"long\"")),
    }
    let entry_obj = root.get("entry").ok_or_else(|| inv("entry is required"))?;
    fields(entry_obj, "entry", &["all"], &["limit", "stop", "scale_in"])?;
    let exit_obj = root.get("exit").ok_or_else(|| inv("exit is required"))?;
    fields(exit_obj, "exit", &["any"], &["stop_loss", "target", "trailing"])?;
    let entry = conds(entry_obj.get("all"), "entry.all", false)?;
    let exit = conds(exit_obj.get("any"), "exit.any", true)?;
    if entry.len() + exit.len() > MAX_CONDITIONS {
        return Err(inv(format!("{} conditions; the limit is {MAX_CONDITIONS}", entry.len() + exit.len())));
    }
    let mut cost_bps = None;
    if let Some(ex) = root.get("execution") {
        fields(ex, "execution", &["fill", "cost_bps"], &["limit", "stop"])?;
        match ex.get("fill") {
            None => {}
            Some(Json::Str(f)) if f == "next_open" => {}
            Some(Json::Str(f)) => return Err(LabError::Unsupported(format!("execution.fill '{f}' is reserved; v1 fills at the next open"))),
            _ => return Err(inv("execution.fill must be \"next_open\"")),
        }
        if let Some(c) = ex.get("cost_bps") {
            cost_bps = Some(c.as_f64().filter(|c| (0. ..=5000.).contains(c)).ok_or_else(|| inv("execution.cost_bps must be a number from 0 to 5000"))?);
        }
    }
    let mut spec = Spec { entry, exit, cost_bps, optimize: vec![] };
    if let Some(o) = root.get("optimize") { spec.optimize = dims(&spec, o)?; }
    Ok(spec)
}

fn fields(v: &Json, at: &str, allowed: &[&str], reserved: &[&str]) -> Result<(), LabError> {
    let Json::Obj(kv) = v else { return Err(inv(format!("{at} must be an object"))) };
    for (k, _) in kv {
        if reserved.contains(&k.as_str()) { return Err(LabError::Unsupported(format!("{at}.{k} is reserved for a later version; v1 does not simulate it"))); }
        if !allowed.contains(&k.as_str()) { return Err(inv(format!("unknown field '{k}' in {at}"))); }
    }
    Ok(())
}

fn conds(v: Option<&Json>, at: &str, exit: bool) -> Result<Vec<Cond>, LabError> {
    let Some(Json::Arr(items)) = v else { return Err(inv(format!("{at} must be a list of conditions"))) };
    if items.is_empty() { return Err(inv(format!("{at} needs at least one condition"))); }
    if items.len() > MAX_CONDITIONS { return Err(inv(format!("{at} has {} conditions; the limit is {MAX_CONDITIONS}", items.len()))); }
    items.iter().enumerate().map(|(i, c)| cond(c, &format!("{at}[{i}]"), exit)).collect()
}

fn cond(v: &Json, at: &str, exit: bool) -> Result<Cond, LabError> {
    fields(v, at, &["lhs", "op", "rhs"], &[])?;
    let op = match v.get("op").and_then(Json::as_str) {
        Some("crosses_above") => Op::CrossesAbove,
        Some("crosses_below") => Op::CrossesBelow,
        Some(">") => Op::Gt,
        Some("<") => Op::Lt,
        Some(">=") => Op::Ge,
        Some("<=") => Op::Le,
        _ => return Err(inv(format!("{at}.op must be one of {}", OPERATORS.join(", ")))),
    };
    let lhs = operand(v.get("lhs"), &format!("{at}.lhs"), exit)?;
    let rhs = operand(v.get("rhs"), &format!("{at}.rhs"), exit)?;
    if matches!((&lhs, &rhs), (Operand::Value(_), Operand::Value(_))) { return Err(inv(format!("{at} compares two numbers; use at least one series"))); }
    Ok(Cond { lhs, op, rhs })
}

fn operand(v: Option<&Json>, at: &str, exit: bool) -> Result<Operand, LabError> {
    let v = v.ok_or_else(|| inv(format!("{at} is required")))?;
    if v.get("value").is_some() {
        fields(v, at, &["value"], &[])?;
        return v.get("value").and_then(Json::as_f64).map(Operand::Value).ok_or_else(|| inv(format!("{at}.value must be a number")));
    }
    fields(v, at, &["s", "p", "tf", "out", "offset"], &[])?;
    let id = v.get("s").and_then(Json::as_str).ok_or_else(|| inv(format!("{at}.s must name a series (or give a value)")))?;
    if id == "bars_held" {
        if !exit { return Err(inv(format!("{at}: bars_held is only available in exit conditions"))); }
        if ["p", "tf", "out", "offset"].iter().any(|k| v.get(k).is_some()) { return Err(inv(format!("{at}: bars_held takes no p, tf, out or offset"))); }
        return Ok(Operand::BarsHeld);
    }
    let tf = match v.get("tf") { None => Tf::D, Some(t) => t.as_str().and_then(Tf::parse).ok_or_else(|| inv(format!("{at}.tf must be D, W or M")))? };
    let offset = match v.get("offset") {
        None => 0,
        Some(o) => o.as_f64().filter(|n| n.fract() == 0. && *n >= 0. && *n <= MAX_OFFSET as f64)
            .ok_or_else(|| inv(format!("{at}.offset must be a whole number from 0 to {MAX_OFFSET} (past bars only)")))? as usize,
    };
    let mut given = Params::new();
    if let Some(p) = v.get("p") {
        let Json::Obj(kv) = p else { return Err(inv(format!("{at}.p must be an object"))) };
        for (k, x) in kv { given.insert(k.clone(), x.as_f64().ok_or_else(|| inv(format!("{at}.p.{k} must be a number")))?); }
    }
    let out = match v.get("out") { None => None, Some(o) => Some(o.as_str().ok_or_else(|| inv(format!("{at}.out must be a string")))?) };
    series_ref(at, id, &given, tf, out, offset).map(Operand::Series)
}

/// Validates one series operand against the catalogue (shared by rule specs and /api/lab/series).
pub fn series_ref(at: &str, id: &str, given: &Params, tf: Tf, out: Option<&str>, offset: usize) -> Result<SeriesRef, LabError> {
    if PRICE_SOURCES.contains(&id) {
        if !given.is_empty() || out.is_some() { return Err(inv(format!("{at}: price source '{id}' takes no p or out"))); }
        return Ok(SeriesRef { id: id.into(), params: Params::new(), tf, out: 0, offset });
    }
    let def = catalogue::get(id).ok_or_else(|| inv(format!("{at}.s: unknown series '{id}'")))?;
    if def.needs.intraday { return Err(LabError::NeedsData(format!("{at}: {} needs intraday data, which is not admitted", def.name))); }
    if def.compute.is_none() { return Err(LabError::Unsupported(format!("{at}: {} is listed but not computed yet", def.name))); }
    let params = catalogue::resolve_params(&def, given).map_err(|e| inv(format!("{at}.p: {e}")))?;
    let out = match out {
        None => 0,
        Some(o) => def.outputs.iter().position(|x| *x == o).ok_or_else(|| inv(format!("{at}.out: {} has outputs {}", def.id, def.outputs.join(", "))))?,
    };
    Ok(SeriesRef { id: id.into(), params, tf, out, offset })
}

fn dims(spec: &Spec, v: &Json) -> Result<Vec<Dim>, LabError> {
    let Json::Arr(items) = v else { return Err(inv("optimize must be a list")) };
    let mut out: Vec<Dim> = Vec::new();
    let mut combos = 1usize;
    for (i, it) in items.iter().enumerate() {
        let at = format!("optimize[{i}]");
        fields(it, &at, &["path", "from", "to", "step"], &[])?;
        let path = it.get("path").and_then(Json::as_str).ok_or_else(|| inv(format!("{at}.path must be a string")))?;
        let target = target(spec, path).map_err(|e| inv(format!("{at}.path '{path}': {e}")))?;
        if out.iter().any(|d| d.target == target) { return Err(inv(format!("{at}.path '{path}' is listed twice"))); }
        let num = |k: &str| it.get(k).and_then(Json::as_f64).ok_or_else(|| inv(format!("{at}.{k} must be a number")));
        // Same grid rule as the book optimizer (main.rs grid(): ≤ 1000 values, 1e-9 rounding).
        let values = crate::grid(&format!("{}:{}:{}", num("from")?, num("to")?, num("step")?)).map_err(|e| inv(format!("{at}: {e}")))?;
        for &x in &values { check_value(spec, &target, x).map_err(|e| inv(format!("{at}: {e}")))?; }
        combos = combos.saturating_mul(values.len());
        if combos > crate::MAX_COMBOS { return Err(inv(format!("optimize gives more than {} combinations; narrow a range or raise a step", crate::MAX_COMBOS))); }
        out.push(Dim { path: path.into(), target, values });
    }
    Ok(out)
}

fn target(spec: &Spec, path: &str) -> Result<Target, String> {
    let (exit, rest) = match (path.strip_prefix("entry.all["), path.strip_prefix("exit.any[")) {
        (Some(r), _) => (false, r),
        (_, Some(r)) => (true, r),
        _ => return Err("must start with entry.all[i] or exit.any[i]".into()),
    };
    let (idx, rest) = rest.split_once("].").ok_or("expected ].lhs or ].rhs")?;
    let cond: usize = idx.parse().map_err(|_| "condition index must be a whole number")?;
    let c = (if exit { &spec.exit } else { &spec.entry }).get(cond).ok_or("no condition at that index")?;
    let (rhs, field) = match (rest.strip_prefix("lhs."), rest.strip_prefix("rhs.")) {
        (Some(f), _) => (false, f),
        (_, Some(f)) => (true, f),
        _ => return Err("expected lhs or rhs".into()),
    };
    match (if rhs { &c.rhs } else { &c.lhs }, field) {
        (Operand::Value(_), "value") => Ok(Target { exit, cond, rhs, param: None }),
        (Operand::Series(s), f) if f.starts_with("p.") => {
            let def = catalogue::get(&s.id).ok_or("price sources have no parameters")?;
            let key = &f[2..];
            if !def.params.iter().any(|p| p.key == key) { return Err(format!("{} has no parameter '{key}'", def.id)); }
            Ok(Target { exit, cond, rhs, param: Some(key.into()) })
        }
        _ => Err("must end in .value (a number operand) or .p.<key> (an indicator parameter)".into()),
    }
}

fn operand_mut<'a>(s: &'a mut Spec, t: &Target) -> &'a mut Operand {
    let c = if t.exit { &mut s.exit[t.cond] } else { &mut s.entry[t.cond] };
    if t.rhs { &mut c.rhs } else { &mut c.lhs }
}
fn set(s: &mut Spec, t: &Target, x: f64) {
    match (operand_mut(s, t), &t.param) {
        (Operand::Value(v), None) => *v = x,
        (Operand::Series(r), Some(k)) => { r.params.insert(k.clone(), x); }
        _ => {}
    }
}
fn check_value(spec: &Spec, t: &Target, x: f64) -> Result<(), String> {
    let mut s = spec.clone();
    set(&mut s, t, x);
    match operand_mut(&mut s, t) {
        Operand::Series(r) => catalogue::resolve_params(&catalogue::get(&r.id).ok_or("unknown series")?, &r.params).map(|_| ()),
        _ => Ok(()),
    }
}
/// The spec's own value at an optimize target (the "base" setting the ranking prefers on ties).
pub fn current(spec: &Spec, t: &Target) -> f64 {
    let mut s = spec.clone();
    match (operand_mut(&mut s, t), &t.param) {
        (Operand::Value(v), None) => *v,
        (Operand::Series(r), Some(k)) => r.params.get(k).copied().unwrap_or(f64::NAN),
        _ => f64::NAN,
    }
}
/// The spec with one grid combination applied (values in `dims` order; validated when the spec was parsed).
pub fn apply(spec: &Spec, dims: &[Dim], values: &[f64]) -> Spec {
    let mut s = spec.clone();
    for (d, &x) in dims.iter().zip(values) { set(&mut s, &d.target, x); }
    s
}
```

Messages are plain text; escaping happens once, in `LabError::json`.

- [ ] **Step 4: Run tests** — `CARGO_BUILD_JOBS=2 cargo test lab::rules` → Expected: 7 passed.

- [ ] **Step 5: Guard + commit**

```bash
python3 scripts/check_book_untouched.py --base origin/Manan
git add Context/guru/backend/src/lab/rules.rs Context/guru/backend/src/lab/mod.rs
git commit -m "Parse and validate lab rule specs against the indicator catalogue"
```

---

### Task 4: Series resolution and compiled conditions (no lookahead)

**Files:**
- Create: `Context/guru/backend/src/lab/series.rs` (+ `pub mod series;` in `lab/mod.rs`, after `pub mod rules;`)

**Interfaces:**
- Consumes: `Bars` (`hl2/hlc3/ohlc4`), `catalogue::{get, run}`, `resample::{resample, expand, Tf}` (Phase 1); `rules::{Cond, LabError, Op, Operand, SeriesRef, Spec}` (Task 3).
- Produces: `pub struct Frames { pub d: Bars, pub w: Bars, pub m: Bars }` with `Frames::new(d: Bars) -> Frames` and `bars(&self, tf: Tf) -> &Bars`; `pub type Cache = BTreeMap<String, Rc<Vec<Vec<f64>>>>`; `pub fn series(fr: &Frames, cache: &mut Cache, s: &SeriesRef) -> Result<Vec<f64>, LabError>` (length = daily bars; offset applied on the operand's timeframe; W/M via `expand`); `pub enum Val { Arr(Vec<f64>), Const(f64), Held }`; `pub struct RCond { pub lhs: Val, pub op: Op, pub rhs: Val, pub label: String }` with `holds(&self, i: usize, entry: Option<usize>) -> bool`; `pub struct Compiled { pub entry: Vec<RCond>, pub exit: Vec<RCond> }`; `pub fn compile(spec: &Spec, fr: &Frames, cache: &mut Cache) -> Result<Compiled, LabError>`; `pub fn describe(c: &Cond) -> String` (e.g. `macd(fast=12,signal=9,slow=26).signal@W[1] < 0`; recorded as the exit reason prefixed with `exit.any[k]: `).
- Catalogue errors map: `"needs volume"` / `"needs intraday data"` → `NeedsData`; anything else → `Invalid`.

- [ ] **Step 1: Write the failing tests** (bottom of `series.rs`)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::lab::catalogue::{all, resolve_params, Params};
    use crate::lab::rules::parse_spec;
    use crate::lab::testdata::{assert_series, close_eq, walk};

    /// Real calendar days (weekends skipped) so W/M grouping matches an exchange calendar.
    pub fn business_days(n: usize, with_volume: bool) -> Bars {
        let mut b = walk(n, 21, with_volume);
        let (mut day, mut k) = (resample::day_number("2021-01-04"), 0); // a Monday
        while k < n {
            if (day + 3).rem_euclid(7) < 5 { b.date[k] = civil(day); k += 1; }
            day += 1;
        }
        b
    }
    fn civil(z: i64) -> String {
        let z = z + 719_468;
        let era = z.div_euclid(146_097);
        let doe = z - era * 146_097;
        let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d = doy - (153 * mp + 2) / 5 + 1;
        let m = if mp < 10 { mp + 3 } else { mp - 9 };
        format!("{:04}-{m:02}-{d:02}", yoe + era * 400 + i64::from(m <= 2))
    }
    fn sref(id: &str, tf: Tf, out: usize, offset: usize) -> SeriesRef {
        let params = catalogue::get(id).map_or(Params::new(), |d| resolve_params(&d, &Params::new()).unwrap());
        SeriesRef { id: id.into(), params, tf, out, offset }
    }

    #[test] fn civil_round_trips() { for d in ["2021-01-04", "2024-02-29", "2000-03-01"] { assert_eq!(civil(resample::day_number(d)), d); } }
    #[test] fn weekly_operand_is_visible_only_after_the_week_closes() {
        let fr = Frames::new(business_days(30, false));
        let w = series(&fr, &mut Cache::new(), &sref("close", Tf::W, 0, 0)).unwrap();
        // 2021-01-04..08 is week 1 (indices 0..4): Mon–Thu see nothing, Friday sees its own weekly close.
        assert!(w[..4].iter().all(|v| v.is_nan()));
        assert_eq!(w[4], fr.d.close[4]);
        // Week 2 Mon–Thu still show week 1; week 2's close appears on its Friday (index 9).
        assert!(w[5..9].iter().all(|v| *v == fr.d.close[4]));
        assert_eq!(w[9], fr.d.close[9]);
    }
    #[test] fn offsets_count_bars_of_their_own_timeframe() {
        let fr = Frames::new(business_days(30, false));
        let w1 = series(&fr, &mut Cache::new(), &sref("close", Tf::W, 0, 1)).unwrap();
        assert!(w1[..9].iter().all(|v| v.is_nan()));
        assert_eq!(w1[9], fr.d.close[4]); // previous week's close, from week 2's Friday
        let d2 = series(&fr, &mut Cache::new(), &sref("close", Tf::D, 0, 2)).unwrap();
        assert_series(&d2[..4], &[f64::NAN, f64::NAN, fr.d.close[0], fr.d.close[1]]);
    }
    #[test] fn signals_never_use_later_bars() {
        // Every catalogue indicator on D, W and M: truncating the data never changes an earlier daily value.
        let full_bars = business_days(420, true);
        let full = Frames::new(full_bars.clone());
        for d in all().iter().filter(|d| d.compute.is_some()) {
            for tf in [Tf::D, Tf::W, Tf::M] {
                let r = sref(d.id, tf, d.outputs.len() - 1, 0);
                let want = series(&full, &mut Cache::new(), &r).unwrap();
                for cut in [97usize, 203, 311] {
                    let mut pre = full_bars.clone();
                    for v in [&mut pre.open, &mut pre.high, &mut pre.low, &mut pre.close] { v.truncate(cut); }
                    pre.date.truncate(cut); pre.end.truncate(cut);
                    if let Some(v) = pre.volume.as_mut() { v.truncate(cut); }
                    let got = series(&Frames::new(pre), &mut Cache::new(), &r).unwrap();
                    // The final bar of the cut may sit inside an unfinished week/month, so only bars before it must match.
                    for i in 0..cut - 1 { assert!(close_eq(got[i], want[i]), "{} {tf:?} bar {i} changes when bars after {cut} exist", d.id); }
                }
            }
        }
    }
    #[test] fn crosses_and_nan_semantics() {
        let c = |op, a: Vec<f64>| RCond { lhs: Val::Arr(a), op, rhs: Val::Const(10.), label: String::new() };
        let up = c(Op::CrossesAbove, vec![9., 10., 11., 12., f64::NAN, 11.]);
        assert_eq!((0..6).map(|i| up.holds(i, None)).collect::<Vec<_>>(), [false, false, true, false, false, false]);
        let down = c(Op::CrossesBelow, vec![11., 10., 9.]);
        assert_eq!((0..3).map(|i| down.holds(i, None)).collect::<Vec<_>>(), [false, false, true]);
        let ge = c(Op::Ge, vec![10., f64::NAN]);
        assert_eq!((ge.holds(0, None), ge.holds(1, None)), (true, false));
        let held = RCond { lhs: Val::Held, op: Op::Ge, rhs: Val::Const(2.), label: String::new() };
        assert_eq!((held.holds(5, None), held.holds(5, Some(4)), held.holds(6, Some(4))), (false, false, true));
    }
    #[test] fn needs_volume_is_typed() {
        let fr = Frames::new(business_days(60, false));
        assert_eq!(series(&fr, &mut Cache::new(), &sref("obv", Tf::D, 0, 0)).unwrap_err().status(), "needs_data");
        assert_eq!(series(&fr, &mut Cache::new(), &sref("volume", Tf::W, 0, 0)).unwrap_err().status(), "needs_data");
    }
    #[test] fn cache_reuses_runs_and_stays_bounded() {
        let fr = Frames::new(business_days(60, false));
        let mut cache = Cache::new();
        series(&fr, &mut cache, &sref("macd", Tf::D, 0, 0)).unwrap();
        series(&fr, &mut cache, &sref("macd", Tf::D, 1, 0)).unwrap();
        assert_eq!(cache.len(), 1);
        for n in 1..=100 { let mut r = sref("sma", Tf::D, 0, 0); r.params.insert("length".into(), n as f64); series(&fr, &mut cache, &r).unwrap(); }
        assert!(cache.len() <= CACHE_CAP);
    }
    #[test] fn every_indicator_at_parameter_bounds_never_panics() {
        for bars in [business_days(3, true), business_days(300, true)] {
            let fr = Frames::new(bars);
            for d in all().iter().filter(|d| d.compute.is_some()) {
                let picks: [fn(&catalogue::ParamSpec) -> f64; 2] = [|p| p.min, |p| p.max];
                for pick in picks {
                    let params: Params = d.params.iter().map(|p| (p.key.to_string(), pick(p))).collect();
                    for tf in [Tf::D, Tf::W, Tf::M] {
                        let r = SeriesRef { id: d.id.into(), params: params.clone(), tf, out: 0, offset: 0 };
                        assert_eq!(series(&fr, &mut Cache::new(), &r).unwrap().len(), fr.d.len(), "{} {tf:?}", d.id);
                    }
                }
            }
        }
    }
    #[test] fn compile_labels_conditions() {
        let spec = parse_spec(r#"{"v":1,"entry":{"all":[{"lhs":{"s":"macd","out":"signal","tf":"W","offset":1},"op":"<","rhs":{"value":0}}]},"exit":{"any":[{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":3}}]}}"#).unwrap();
        let c = compile(&spec, &Frames::new(business_days(60, false)), &mut Cache::new()).unwrap();
        assert_eq!(c.entry[0].label, "entry.all[0]: macd(fast=12,signal=9,slow=26).signal@W[1] < 0");
        assert_eq!(c.exit[0].label, "exit.any[0]: bars_held >= 3");
    }
}
```

`signals_never_use_later_bars` compares every bar except the cut's last one: Phase 1's `expand` shows an unfinished final week/month from its latest daily bar (documented in `resample.rs`), so only that bar may legitimately differ. Any real lookahead (e.g. a week's value on its Monday) differs on an earlier bar of some cut and fails. `every_indicator_at_parameter_bounds_never_panics` passes on the current Phase 1 code (checked while planning); if a later catalogue change makes it fail, fix that indicator in its family file with a regression test — never narrow a parameter bound to hide a panic.

- [ ] **Step 2: Run to verify failure** — `CARGO_BUILD_JOBS=2 cargo test lab::series` → Expected: compile error.

- [ ] **Step 3: Implement** (top of `series.rs`)

```rust
//! Operands → daily-aligned series and compiled conditions. Indicators run on their own timeframe's bars;
//! W/M values reach daily bars through `resample::expand` (usable after the period's last daily close).
use super::bars::Bars;
use super::catalogue;
use super::resample::{self, Tf};
use super::rules::{Cond, LabError, Op, Operand, SeriesRef, Spec};
use std::collections::BTreeMap;
use std::rc::Rc;

/// Daily bars of one instrument plus their weekly and monthly resamples (built once per request).
pub struct Frames { pub d: Bars, pub w: Bars, pub m: Bars }
impl Frames {
    pub fn new(d: Bars) -> Frames { let (w, m) = (resample::resample(&d, Tf::W), resample::resample(&d, Tf::M)); Frames { d, w, m } }
    pub fn bars(&self, tf: Tf) -> &Bars { match tf { Tf::D => &self.d, Tf::W => &self.w, Tf::M => &self.m } }
}
/// Indicator outputs on their own timeframe, keyed by (id, tf, resolved params). One per request / optimizer thread.
pub type Cache = BTreeMap<String, Rc<Vec<Vec<f64>>>>;
const CACHE_CAP: usize = 64;

fn price(b: &Bars, id: &str) -> Option<Result<Vec<f64>, LabError>> {
    Some(Ok(match id {
        "open" => b.open.clone(),
        "high" => b.high.clone(),
        "low" => b.low.clone(),
        "close" => b.close.clone(),
        "hl2" => b.hl2(),
        "hlc3" => b.hlc3(),
        "ohlc4" => b.ohlc4(),
        "volume" => return Some(b.volume.clone().ok_or_else(|| LabError::NeedsData("volume: this instrument's admitted data has no volume".into()))),
        _ => return None,
    }))
}

/// One operand as a series aligned to the daily bars (NaN = no value).
pub fn series(fr: &Frames, cache: &mut Cache, s: &SeriesRef) -> Result<Vec<f64>, LabError> {
    let b = fr.bars(s.tf);
    let raw = match price(b, &s.id) {
        Some(r) => r?,
        None => {
            let key = format!("{}|{:?}|{:?}", s.id, s.tf, s.params);
            let outs = match cache.get(&key) {
                Some(o) => o.clone(),
                None => {
                    let def = catalogue::get(&s.id).ok_or_else(|| LabError::Invalid(format!("unknown series '{}'", s.id)))?;
                    let o = Rc::new(catalogue::run(&def, b, &s.params).map_err(|e| match e.as_str() {
                        "needs volume" => LabError::NeedsData(format!("{}: needs volume; this instrument's admitted data has none", def.name)),
                        "needs intraday data" => LabError::NeedsData(format!("{}: needs intraday data", def.name)),
                        _ => LabError::Invalid(format!("{}: {e}", def.id)),
                    })?);
                    // shortcut: a full reset bounds memory (64 × outputs × bars); switch to LRU if optimizer profiles show recomputation.
                    if cache.len() >= CACHE_CAP { cache.clear(); }
                    cache.insert(key, o.clone());
                    o
                }
            };
            outs[s.out].clone()
        }
    };
    // Offset counts bars of the operand's own timeframe (W offset 1 = the previous completed week).
    let shifted: Vec<f64> = (0..raw.len()).map(|i| if i >= s.offset { raw[i - s.offset] } else { f64::NAN }).collect();
    Ok(if s.tf == Tf::D { shifted } else { resample::expand(&shifted, b, fr.d.len()) })
}

pub enum Val { Arr(Vec<f64>), Const(f64), Held }
impl Val {
    /// Value at the close of daily bar i. `entry` = fill bar of the open position; bars_held is 0 on that bar.
    fn at(&self, i: usize, entry: Option<usize>) -> f64 {
        match self {
            Val::Arr(v) => v[i],
            Val::Const(x) => *x,
            Val::Held => entry.filter(|&e| e <= i).map_or(f64::NAN, |e| (i - e) as f64),
        }
    }
}
pub struct RCond { pub lhs: Val, pub op: Op, pub rhs: Val, pub label: String }
impl RCond {
    /// NaN on either side (or on the previous bar for crosses) makes the condition false.
    pub fn holds(&self, i: usize, entry: Option<usize>) -> bool {
        let (a, b) = (self.lhs.at(i, entry), self.rhs.at(i, entry));
        let prev = || if i == 0 { (f64::NAN, f64::NAN) } else { (self.lhs.at(i - 1, entry), self.rhs.at(i - 1, entry)) };
        match self.op {
            Op::Gt => a > b,
            Op::Lt => a < b,
            Op::Ge => a >= b,
            Op::Le => a <= b,
            Op::CrossesAbove => { let (a1, b1) = prev(); a > b && a1 <= b1 }
            Op::CrossesBelow => { let (a1, b1) = prev(); a < b && a1 >= b1 }
        }
    }
}
pub struct Compiled { pub entry: Vec<RCond>, pub exit: Vec<RCond> }

fn val(o: &Operand, fr: &Frames, cache: &mut Cache) -> Result<Val, LabError> {
    Ok(match o { Operand::Series(s) => Val::Arr(series(fr, cache, s)?), Operand::Value(x) => Val::Const(*x), Operand::BarsHeld => Val::Held })
}
fn list(cs: &[Cond], at: &str, fr: &Frames, cache: &mut Cache) -> Result<Vec<RCond>, LabError> {
    cs.iter().enumerate().map(|(k, c)| Ok(RCond { lhs: val(&c.lhs, fr, cache)?, op: c.op, rhs: val(&c.rhs, fr, cache)?, label: format!("{at}[{k}]: {}", describe(c)) })).collect()
}
pub fn compile(spec: &Spec, fr: &Frames, cache: &mut Cache) -> Result<Compiled, LabError> {
    Ok(Compiled { entry: list(&spec.entry, "entry.all", fr, cache)?, exit: list(&spec.exit, "exit.any", fr, cache)? })
}

/// Human-readable condition, recorded as a trade's exit reason, e.g. `rsi(length=2) crosses_above 70`.
pub fn describe(c: &Cond) -> String {
    let side = |o: &Operand| match o {
        Operand::Value(x) => format!("{x}"),
        Operand::BarsHeld => "bars_held".into(),
        Operand::Series(s) => {
            let p: Vec<String> = s.params.iter().map(|(k, v)| format!("{k}={v}")).collect();
            let mut t = if p.is_empty() { s.id.clone() } else { format!("{}({})", s.id, p.join(",")) };
            if s.out > 0 { if let Some(d) = catalogue::get(&s.id) { t = format!("{t}.{}", d.outputs[s.out]); } }
            if s.tf != Tf::D { t = format!("{t}@{:?}", s.tf); }
            if s.offset > 0 { t = format!("{t}[{}]", s.offset); }
            t
        }
    };
    let op = match c.op { Op::CrossesAbove => "crosses_above", Op::CrossesBelow => "crosses_below", Op::Gt => ">", Op::Lt => "<", Op::Ge => ">=", Op::Le => "<=" };
    format!("{} {op} {}", side(&c.lhs), side(&c.rhs))
}
```

- [ ] **Step 4: Run tests** — `CARGO_BUILD_JOBS=2 cargo test lab::series` → Expected: 9 passed.

- [ ] **Step 5: Guard + commit**

```bash
python3 scripts/check_book_untouched.py --base origin/Manan
git add Context/guru/backend/src/lab/series.rs Context/guru/backend/src/lab/mod.rs
git commit -m "Resolve lab operands to daily series without lookahead"
```

---

### Task 5: Long-only simulator (signal at close, fill next open)

**Files:**
- Create: `Context/guru/backend/src/lab/simulate.rs` (+ `pub mod simulate;` in `lab/mod.rs`, after `pub mod series;`)

**Interfaces:**
- Consumes: `series::Compiled` (Task 4); `crate::connors::{Leg, Trade}` (`connors.rs:375`, `:382`, read-only).
- Produces: `pub fn simulate(c: &Compiled, b: &Bars, from: usize, to: usize) -> Vec<Trade>` — `dir: 1.`, one leg `{i: fill bar, px: open, w: 1.}`, `entry_kind: "next_open"`, `signal_i` = exit signal bar (`exit_i - 1`) or `to` for the open trade, `reason` = first matching exit rule's label or `"open"`, `exits: vec![]`. Output always passes `trade_validation::check` (single position, no overlap, terminal open only).
- Why `connors::Trade`: it lets `trade_validation::check`, `equity_curve`, `stats` and `financial_stats` run unchanged (Tasks 7–8), so the lab has no copied formulas (spec §2a "if one needs a change, the lab gets its own copy" — none needed).

- [ ] **Step 1: Write the failing tests** (bottom of `simulate.rs`)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::lab::rules::Op;
    use crate::lab::series::{RCond, Val};
    use crate::lab::testdata::from_ohlc;

    fn on(bars: &[usize]) -> Vec<f64> { (0..10).map(|i| if bars.contains(&i) { 1. } else { 0. }).collect() }
    fn rule(signal: Vec<f64>) -> RCond { RCond { lhs: Val::Arr(signal), op: Op::Gt, rhs: Val::Const(0.5), label: "rule".into() } }
    fn held_at_least(n: f64) -> RCond { RCond { lhs: Val::Held, op: Op::Ge, rhs: Val::Const(n), label: format!("bars_held >= {n}") } }
    /// Open = 100 + i, close = 100.5 + i, so fills are identifiable by price.
    fn bars() -> Bars { from_ohlc(&(0..10).map(|i| { let o = 100. + i as f64; (o, o + 1., o - 1., o + 0.5) }).collect::<Vec<_>>()) }

    #[test] fn entry_signal_at_close_fills_next_open() {
        let c = Compiled { entry: vec![rule(on(&[2]))], exit: vec![held_at_least(2.)] };
        let t = simulate(&c, &bars(), 0, 9);
        assert_eq!(t.len(), 1);
        assert_eq!((t[0].legs[0].i, t[0].legs[0].px), (3, 103.)); // signal bar 2 → open of bar 3
        // bars_held is 0 on the fill bar: 2 at the close of bar 5 → exit at the open of bar 6
        assert_eq!((t[0].signal_i, t[0].exit_i, t[0].exit_px, t[0].open), (5, 6, 106., false));
        assert_eq!(t[0].reason, "bars_held >= 2");
    }
    #[test] fn exit_fills_before_a_same_bar_reentry() {
        let c = Compiled { entry: vec![rule(on(&[1, 4]))], exit: vec![rule(on(&[4]))] };
        let t = simulate(&c, &bars(), 0, 9);
        assert_eq!(t.len(), 2);
        assert_eq!((t[0].legs[0].i, t[0].exit_i), (2, 5));
        assert_eq!((t[1].legs[0].i, t[1].legs[0].px), (5, 105.)); // re-entry at the same open, after the exit
        assert!(crate::trade_validation::check(&t, 0, 9, 0.0005).is_ok());
    }
    #[test] fn entries_need_all_exits_any_and_no_pyramiding() {
        let c = Compiled { entry: vec![rule(on(&[1, 2, 3, 6])), rule(on(&[1, 3, 6]))], exit: vec![rule(on(&[7])), rule(on(&[4]))] };
        let t = simulate(&c, &bars(), 0, 9);
        assert_eq!(t.iter().map(|t| (t.legs[0].i, t.exit_i)).collect::<Vec<_>>(), vec![(2, 5), (7, 8)]);
        assert_eq!(t[0].reason, "rule");
    }
    #[test] fn open_position_at_the_end_is_marked_open() {
        let c = Compiled { entry: vec![rule(on(&[7]))], exit: vec![rule(on(&[]))] };
        let t = simulate(&c, &bars(), 0, 9);
        assert_eq!((t[0].legs[0].i, t[0].exit_i, t[0].exit_px, t[0].open, t[0].reason.as_str()), (8, 9, 109.5, true, "open"));
    }
    #[test] fn last_bar_signal_and_pre_window_signal_do_not_trade() {
        let c = Compiled { entry: vec![rule(on(&[1, 9]))], exit: vec![rule(on(&[]))] };
        assert!(simulate(&c, &bars(), 2, 9).is_empty());
    }
    #[test] fn gaps_fill_at_the_gapped_open() {
        let mut b = bars();
        b.open[6] = 90.; b.low[6] = 89.; // gap down through the prior lows
        let c = Compiled { entry: vec![rule(on(&[2]))], exit: vec![rule(on(&[5]))] };
        let t = simulate(&c, &b, 0, 9);
        assert_eq!((t[0].exit_i, t[0].exit_px), (6, 90.));
    }
}
```

- [ ] **Step 2: Run to verify failure** — `CARGO_BUILD_JOBS=2 cargo test lab::simulate` → Expected: compile error.

- [ ] **Step 3: Implement** (top of `simulate.rs`)

```rust
//! Long-only v1 simulation (spec §4): signals at a bar's close fill at the next bar's open; on each bar the
//! exit fill comes before the entry fill; entries need all conditions, exits any one. Produces book-format
//! `connors::Trade`s so trade_validation, equity and stats are reused read-only.
use super::bars::Bars;
use super::series::Compiled;
use crate::connors::{Leg, Trade};

pub fn simulate(c: &Compiled, b: &Bars, from: usize, to: usize) -> Vec<Trade> {
    let mut trades = Vec::new();
    let mut held: Option<usize> = None; // fill bar of the open position
    let mut entry_signal = false;
    let mut exit_signal: Option<(usize, String)> = None;
    let trade = |e: usize, x: usize, px: f64, reason: String, signal_i: usize, open: bool| Trade {
        dir: 1., legs: vec![Leg { i: e, px: b.open[e], w: 1. }], exit_i: x, exit_px: px, reason, entry_kind: "next_open", signal_i, open, exits: vec![],
    };
    for i in from..=to {
        if let (Some((sig, reason)), Some(e)) = (exit_signal.take(), held) {
            trades.push(trade(e, i, b.open[i], reason, sig, false));
            held = None;
        }
        if std::mem::take(&mut entry_signal) { held = Some(i); }
        if i == to { break; } // a signal on the last bar has no next open inside the window
        if let Some(e) = held { exit_signal = c.exit.iter().find(|r| r.holds(i, Some(e))).map(|r| (i, r.label.clone())); }
        // A bar whose close signals an exit may also signal a new entry; both fill at the next open, exit first.
        if (held.is_none() || exit_signal.is_some()) && c.entry.iter().all(|r| r.holds(i, None)) { entry_signal = true; }
    }
    if let Some(e) = held { trades.push(trade(e, to, b.close[to], "open".into(), to, true)); }
    trades
}
```

- [ ] **Step 4: Run tests** — `CARGO_BUILD_JOBS=2 cargo test lab::simulate` → Expected: 6 passed.

- [ ] **Step 5: Guard + commit**

```bash
python3 scripts/check_book_untouched.py --base origin/Manan
git add Context/guru/backend/src/lab/simulate.rs Context/guru/backend/src/lab/mod.rs
git commit -m "Simulate lab rules with next-open fills and exits before entries"
```

---

### Task 6: Lab API module, catalogue and series routes, the one `main.rs` line

**Files:**
- Create: `Context/guru/backend/src/lab/api.rs` (+ `pub mod api;` in `lab/mod.rs`, first in the list)
- Modify: `Context/guru/backend/src/main.rs` — add exactly one line immediately before line 1093 (`        "/api/notes" => match note_path(&q) {`):

```rust
        p if p.starts_with("/api/lab/") => lab::api::serve(&mut stream, p, &st, &q),
```

**Interfaces:**
- Consumes: `crate::{select_context, SelectedContext, preloaded_market, params_of, source_provenance, respond, MAX_COMBOS, State}`, `instrument_policy::selected_type`, `connors::Ctx` (all read-only); Tasks 2–4.
- Produces:
  - `pub fn serve(stream: &mut TcpStream, path: &str, st: &State, q: &BTreeMap<String,String>)` — unknown `/api/lab/*` → 404 text (same as other unknown paths); `Ok` → `200 application/json; charset=utf-8`; `Err(e)` → `400 application/json` with `e.json()`.
  - `pub fn route(path, st, q) -> Option<Result<String, LabError>>`, `pub fn guarded(f) -> Result<String, LabError>` (`catch_unwind`; a panic → `invalid`).
  - `GET /api/lab/catalogue` → `{"label","price_sources":[8],"operators":[6],"timeframes":["D","W","M"],"max_conditions":12,"max_spec_bytes":8192,"max_combos":50000,"indicators":[{"id","name","category","placement":"price|pane","needs":{"volume":bool,"intraday":bool},"computed":bool,"params":[{"key","label","default","min","max","step","integer"}],"outputs":[…],"first":[int|null],"sparse":bool,"definition"}]}` (`first` `usize::MAX` → `null`).
  - `GET /api/lab/series?ticker=&id=&tf=D|W|M&p.<key>=<n>` → `{"label","instrument":{"ticker","type":"index|etf|individual_stock|unknown","native":bool,"market":"in|us|null","currency":"INR|USD|null"},"id","tf","params":{resolved},"outputs":[…],"dates":[every admitted daily date],"series":{"<output>":[number|null per date]},"provenance":…}` — values are the daily-aligned series the rules use (W/M usable from the period's last day), full precision like `/api/ohlc`.

- [ ] **Step 1: Write the failing tests** (bottom of `api.rs`)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;

    fn state(n: usize, ticker: &str, volume: bool) -> State {
        let rows: &'static [Row] = Box::leak((0..n).map(|i| {
            let c = 100. + (i as f64 * 0.3).sin() * 8. + i as f64 * 0.02;
            Row { ticker: ticker.into(), date: format!("{}-{:02}-{:02}", 2015 + i / 336, 1 + (i / 28) % 12, 1 + i % 28), open: c - 0.2, high: c + 1., low: c - 1., close: c, volume: volume.then_some(1000.) }
        }).collect::<Vec<_>>().into_boxed_slice());
        State { ctxs: BTreeMap::from([(ticker.into(), Ctx::new(rows, rows, &[]))]), baskets: Default::default(), vix: vec![], market_extra: Default::default() }
    }
    const SPEC: &str = r#"{"v":1,"entry":{"all":[{"lhs":{"s":"close"},"op":"crosses_above","rhs":{"s":"sma","p":{"length":5}}}]},"exit":{"any":[{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":3}}]},"optimize":[{"path":"entry.all[0].rhs.p.length","from":3,"to":7,"step":2},{"path":"exit.any[0].rhs.value","from":2,"to":4,"step":1}]}"#;
    fn q(extra: &[(&str, &str)]) -> Query {
        let mut q: Query = [("ticker", "SPX"), ("spec", SPEC), ("cost", "5")].iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
        for (k, v) in extra { q.insert(k.to_string(), v.to_string()); }
        q
    }

    #[test] fn catalogue_lists_every_indicator_with_label() {
        let j = catalogue_json();
        assert!(j.starts_with(&format!("{{\"label\":\"{LABEL}\"")));
        for d in catalogue::all() { assert!(j.contains(&format!("{{\"id\":\"{}\"", d.id)), "{}", d.id); }
        assert!(j.contains("\"id\":\"vwap\"") && j.contains("\"intraday\":true") && j.contains("\"first\":[null]"));
        assert!(crate::lab::json::parse(&j).is_ok());
    }
    #[test] fn panics_become_typed_errors() {
        assert_eq!(guarded(|| panic!("boom")).unwrap_err().status(), "invalid");
        let st = state(50, "SPX", false);
        assert!(route("/api/lab/nothing", &st, &q(&[])).is_none());
        assert!(route("/api/lab/catalogue", &st, &q(&[])).unwrap().is_ok());
    }
    #[test] fn series_route_returns_daily_aligned_outputs() {
        let st = state(120, "SPX", true);
        let mut query = q(&[("id", "macd"), ("tf", "W"), ("p.fast", "3")]);
        let j = series_json(&st, &query).unwrap();
        let v = crate::lab::json::parse(&j).unwrap();
        let crate::lab::json::Json::Arr(dates) = v.get("dates").unwrap() else { panic!() };
        let crate::lab::json::Json::Arr(signal) = v.get("series").unwrap().get("signal").unwrap() else { panic!() };
        assert_eq!((dates.len(), signal.len()), (120, 120));
        assert!(j.contains("\"params\":{\"fast\":3,\"signal\":9,\"slow\":26}") && j.contains("\"tf\":\"W\""));
        query.insert("p.fast".into(), "abc".into());
        assert!(series_json(&st, &query).unwrap_err().message().contains("p.fast"));
        query.insert("p.fast".into(), "0".into());
        assert!(series_json(&st, &query).unwrap_err().message().contains("fast must be between"));
        assert_eq!(series_json(&st, &q(&[("id", "close"), ("tf", "Q")])).unwrap_err().message(), "tf must be D, W or M");
    }
}
```

- [ ] **Step 2: Run to verify failure** — `CARGO_BUILD_JOBS=2 cargo test lab::api` → Expected: compile error.

- [ ] **Step 3: Implement** (top of `api.rs`), then add the `main.rs` line above.

```rust
//! /api/lab/* routes. Book code (data selection, costs, trade validation, equity, stats, ranking) is called
//! read-only; every response carries LABEL and no gate, readiness or book claim.
use super::bars::Bars;
use super::catalogue::{self, Placement};
use super::json::esc;
use super::resample::Tf;
use super::rules::{self, LabError, LABEL};
use super::series::{self, Cache, Frames};
use crate::{connors::Ctx, instrument_policy, State};
use std::collections::BTreeMap;
use std::net::TcpStream;

type Query = BTreeMap<String, String>;

pub fn serve(stream: &mut TcpStream, path: &str, st: &State, q: &Query) {
    match route(path, st, q) {
        None => crate::respond(stream, "404 Not Found", "text/plain", b"not found"),
        Some(Ok(body)) => crate::respond(stream, "200 OK", "application/json; charset=utf-8", body.as_bytes()),
        Some(Err(e)) => crate::respond(stream, "400 Bad Request", "application/json; charset=utf-8", e.json().as_bytes()),
    }
}
pub fn route(path: &str, st: &State, q: &Query) -> Option<Result<String, LabError>> {
    let f: fn(&State, &Query) -> Result<String, LabError> = match path {
        "/api/lab/catalogue" => |_, _| Ok(catalogue_json()),
        "/api/lab/series" => series_json,
        _ => return None,
    };
    Some(guarded(|| f(st, q)))
}
/// A panic anywhere in a lab request becomes a typed 400, never a dropped connection.
/// A panic anywhere in a lab request becomes a typed 400, never a dropped connection.
pub fn guarded(f: impl FnOnce() -> Result<String, LabError>) -> Result<String, LabError> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(f))
        .unwrap_or_else(|_| Err(LabError::Invalid("internal error while evaluating this request; no result is published".into())))
}

fn full(v: f64) -> String { if v.is_finite() { v.to_string() } else { "null".into() } }
fn arr(it: impl Iterator<Item = String>) -> String { format!("[{}]", it.collect::<Vec<_>>().join(",")) }
fn quoted<'a>(it: impl Iterator<Item = &'a str>) -> String { arr(it.map(|s| format!("\"{}\"", esc(s)))) }

pub fn catalogue_json() -> String {
    let defs = catalogue::all().into_iter().map(|d| {
        let params = arr(d.params.iter().map(|p| format!(
            "{{\"key\":\"{}\",\"label\":\"{}\",\"default\":{},\"min\":{},\"max\":{},\"step\":{},\"integer\":{}}}",
            p.key, esc(p.label), full(p.default), full(p.min), full(p.max), full(p.step), p.integer)));
        format!(
            "{{\"id\":\"{}\",\"name\":\"{}\",\"category\":\"{}\",\"placement\":\"{}\",\"needs\":{{\"volume\":{},\"intraday\":{}}},\"computed\":{},\"params\":{params},\"outputs\":{},\"first\":{},\"sparse\":{},\"definition\":\"{}\"}}",
            d.id, esc(d.name), esc(d.category), if d.placement == Placement::Price { "price" } else { "pane" }, d.needs.volume, d.needs.intraday,
            d.compute.is_some(), quoted(d.outputs.iter().copied()), arr(d.first.iter().map(|&f| if f == usize::MAX { "null".into() } else { f.to_string() })), d.sparse, esc(d.definition))
    });
    format!(
        "{{\"label\":\"{LABEL}\",\"price_sources\":{},\"operators\":{},\"timeframes\":[\"D\",\"W\",\"M\"],\"max_conditions\":{},\"max_spec_bytes\":{},\"max_combos\":{},\"indicators\":{}}}",
        quoted(rules::PRICE_SOURCES.iter().copied()), quoted(rules::OPERATORS.iter().copied()), rules::MAX_CONDITIONS, rules::MAX_SPEC_BYTES, crate::MAX_COMBOS, arr(defs))
}

/// Instrument identity from the server's own data (never from the query): type, native flag, market, currency.
fn instrument(selected: &crate::SelectedContext, ticker: &str) -> (&'static str, String) {
    let native = selected.native();
    let kind = instrument_policy::selected_type(ticker, native);
    let market = selected.evidence().map(|e| e.market()).or_else(|| crate::preloaded_market(ticker));
    let opt = |s: Option<&str>| s.map_or("null".into(), |s| format!("\"{s}\""));
    (kind, format!("{{\"ticker\":\"{}\",\"type\":\"{kind}\",\"native\":{native},\"market\":{},\"currency\":{}}}",
        esc(ticker), opt(market.map(|m| m.code())), opt(market.map(|m| m.currency()))))
}
fn select<'a>(st: &'a State, q: &Query) -> Result<crate::SelectedContext<'a>, LabError> {
    crate::select_context(st, q).map_err(|e| LabError::NeedsData(e.trim_start_matches("needs_data:").trim().into()))
}
pub fn series_json(st: &State, q: &Query) -> Result<String, LabError> {
    let id = q.get("id").ok_or_else(|| LabError::Invalid("id is required".into()))?;
    let tf = q.get("tf").map_or(Some(Tf::D), |t| Tf::parse(t)).ok_or_else(|| LabError::Invalid("tf must be D, W or M".into()))?;
    if let Some((k, _)) = q.iter().find(|(k, v)| k.starts_with("p.") && !v.parse::<f64>().is_ok_and(f64::is_finite)) {
        return Err(LabError::Invalid(format!("{k} must be a finite number")));
    }
    let r = rules::series_ref("series", id, &crate::params_of(q), tf, None, 0)?;
    let selected = select(st, q)?;
    let held = selected.context(st);
    let ctx: &Ctx = &held;
    let (_, instrument) = instrument(&selected, &ctx.r[0].ticker);
    let fr = Frames::new(Bars::from_rows(ctx.r));
    let outputs: Vec<&str> = catalogue::get(id).map_or(vec![id.as_str()], |d| d.outputs.to_vec());
    let mut cache = Cache::new();
    let mut values = Vec::new();
    for (k, name) in outputs.iter().enumerate() {
        let s = series::series(&fr, &mut cache, &rules::SeriesRef { out: k, ..r.clone() })?;
        values.push(format!("\"{}\":{}", esc(name), arr(s.into_iter().map(full))));
    }
    let params = r.params.iter().map(|(k, v)| format!("\"{}\":{}", esc(k), full(*v))).collect::<Vec<_>>().join(",");
    Ok(format!("{{\"label\":\"{LABEL}\",\"instrument\":{instrument},\"id\":\"{}\",\"tf\":\"{:?}\",\"params\":{{{params}}},\"outputs\":{},\"dates\":{},\"series\":{{{}}},\"provenance\":{}}}",
        esc(id), tf, quoted(outputs.iter().copied()), quoted(ctx.r.iter().map(|r| r.date.as_str())), values.join(","), crate::source_provenance(st, q)))
}
```

- [ ] **Step 4: Run tests** — `CARGO_BUILD_JOBS=2 cargo test lab::api` → Expected: 3 passed. `git diff origin/Manan -- Context/guru/backend/src/main.rs` shows only the one `+` line.

- [ ] **Step 5: Guard + commit**

```bash
python3 scripts/check_book_untouched.py --base origin/Manan
git add Context/guru/backend/src/lab/api.rs Context/guru/backend/src/lab/mod.rs Context/guru/backend/src/main.rs
git commit -m "Serve the lab indicator catalogue and aligned series"
```

---

### Task 7: `/api/lab/run` — simulate and report with book accounting

**Files:**
- Modify: `Context/guru/backend/src/lab/api.rs`

**Interfaces:**
- Consumes: `crate::{resolve_context, equity_curve, financial_stats, stats, stats_json, num, Run}`, `accounting::Costs`, `trade_validation::check`, `baskets::valid_date` (read-only); `rules::Spec`, `series::Compiled`, `simulate` (Tasks 3–5).
- Produces: `GET /api/lab/run?ticker=&from=&to=&cost=&slippage=&rollover=&spec=<url-encoded JSON>` → `{"label","kind":"lab_simulation","instrument":{…},"from","to","dates":[window],"close":[…],"equity":[…],"gross_equity":[…],"bh":[…],"trades":[{"entry","exit","entry_i","exit_i","entry_px","exit_px","ret","bars","open","signal","reason"}],"stats":{financial_stats keys},"gross_stats":{stats keys},"costs":{Costs::json},"provenance":…}` (`entry_i`/`exit_i` relative to `from`, as `detail()`).
- Order of checks: spec (`invalid`/`unsupported`/`needs_data` for intraday) → `execution.cost_bps` vs `cost` → ticker (`needs_data`) → instrument type known → dates valid and covered → costs and rollover → series (`needs_data` for volume) → simulate → `trade_validation::check` → equity positive.

- [ ] **Step 1: Write the failing tests** — add inside `mod tests` of `api.rs`:

```rust
    #[test] fn run_is_labelled_simulated_and_reuses_book_accounting() {
        let st = state(400, "SPX", false);
        let j = run_json(&st, &q(&[("from", "2015-02-01")])).unwrap();
        assert!(j.starts_with(&format!("{{\"label\":\"{LABEL}\",\"kind\":\"lab_simulation\"")), "{}", &j[..200]);
        assert!(j.contains("\"instrument\":{\"ticker\":\"SPX\",\"type\":\"index\",\"native\":false,\"market\":\"us\",\"currency\":\"USD\"}"));
        assert!(j.contains("\"reason\":\"exit.any[0]: bars_held >= 3\"") && j.contains("\"fees_bps\":5"));
        let v = crate::lab::json::parse(&j).unwrap();
        assert!(v.get("stats").and_then(|s| s.get("trades")).and_then(|t| t.as_f64()).unwrap() > 5.);
        assert!(!j.contains("\"gate\"") && !j.contains("\"claim\"") && !j.contains("\"readiness\""));
    }
    #[test] fn errors_are_typed_and_name_the_problem() {
        let st = state(400, "SPX", false);
        let cases: Vec<(Query, &str, &str)> = vec![
            (q(&[("spec", "{")]), "invalid", "not valid JSON"),
            (q(&[("ticker", "NOPE")]), "needs_data", "unknown ticker NOPE"),
            (q(&[("from", "2014-01-01")]), "needs_data", "precedes admitted data"),
            (q(&[("to", "2031-01-01")]), "needs_data", "exceeds admitted data"),
            (q(&[("from", "2015-02-30")]), "invalid", "valid YYYY-MM-DD"),
            (q(&[("rollover", "4")]), "needs_data", "contract"),
            (q(&[("spec", &SPEC.replace("\"close\"},\"op\"", "\"obv\"},\"op\""))]), "needs_data", "needs volume"),
            (q(&[("spec", &SPEC.replace("\"v\":1,", "\"v\":1,\"execution\":{\"cost_bps\":9},"))]), "invalid", "differs from the cost query"),
        ];
        for (query, status, needle) in cases {
            let e = run_json(&st, &query).unwrap_err();
            assert_eq!(e.status(), status, "{e:?}");
            assert!(e.message().contains(needle), "{needle:?} not in {:?}", e.message());
        }
        let unknown = state(400, "MYSTERY", false);
        assert_eq!(run_json(&unknown, &q(&[("ticker", "MYSTERY")])).unwrap_err().status(), "needs_data");
    }
```

- [ ] **Step 2: Run to verify failure** — `CARGO_BUILD_JOBS=2 cargo test lab::api` → Expected: compile error (`run_json` not found).

- [ ] **Step 3: Implement**

Add to the imports of `api.rs`:
```rust
use super::rules::Spec;
use super::series::Compiled;
use super::simulate::simulate;
use crate::{accounting, trade_validation};
```
Add the arm to `route()` after `"/api/lab/series" => series_json,`:
```rust
        "/api/lab/run" => run_json,
```
Add these functions above `#[cfg(test)]`:
```rust
fn known_type(kind: &str, ticker: &str) -> Result<(), LabError> {
    if kind == "unknown" { return Err(LabError::NeedsData(format!("{ticker}: instrument type is not established (index, ETF or catalogued stock); no lab result is published"))); }
    Ok(())
}
/// Requested dates must be real and inside admitted coverage; nothing is filled or silently shortened.
fn window(ctx: &Ctx, q: &Query) -> Result<(usize, usize), LabError> {
    for key in ["from", "to", "split"] {
        if q.get(key).is_some_and(|v| !v.is_empty() && !crate::baskets::valid_date(v)) { return Err(LabError::Invalid(format!("{key} must be a valid YYYY-MM-DD date"))); }
    }
    let (first, last) = (&ctx.r[0].date, &ctx.r[ctx.r.len() - 1].date);
    if q.get("from").is_some_and(|f| !f.is_empty() && f < first) { return Err(LabError::NeedsData(format!("from precedes admitted data starting {first}; no earlier prices are filled"))); }
    if q.get("to").is_some_and(|t| !t.is_empty() && t > last) { return Err(LabError::NeedsData(format!("to exceeds admitted data ending {last}; the range is not silently shortened"))); }
    let (_, from, to, _) = crate::resolve_context(ctx, q)?;
    Ok((from, to))
}
fn spec_of(q: &Query) -> Result<Spec, LabError> { rules::parse_spec(q.get("spec").ok_or_else(|| LabError::Invalid("spec is required".into()))?) }
/// execution.cost_bps (saved with a strategy) and the `cost` query must agree; one of them may be absent.
fn with_spec_cost(q: &Query, spec: &Spec) -> Result<Query, LabError> {
    let mut q = q.clone();
    if let Some(c) = spec.cost_bps {
        match q.get("cost").map(|v| v.parse::<f64>()) {
            None => { q.insert("cost".into(), c.to_string()); }
            Some(Ok(v)) if v == c => {}
            _ => return Err(LabError::Invalid(format!("execution.cost_bps ({c}) differs from the cost query; send one value"))),
        }
    }
    Ok(q)
}
fn backtest(ctx: &Ctx, c: &Compiled, d: &Bars, from: usize, to: usize, cost: f64) -> Result<crate::Run, LabError> {
    let trades = simulate(c, d, from, to);
    trade_validation::check(&trades, from, to, cost)?;
    let equity = crate::equity_curve(&ctx.c, &trades, from, to, cost);
    if equity.iter().any(|v| !v.is_finite() || *v <= 0.) { return Err(LabError::Invalid("calculation_unavailable: nonpositive or nonfinite equity; no result is published".into())); }
    let bh = (from..=to).map(|k| ctx.c[k] / ctx.c[from]).collect();
    Ok(crate::Run { trades, from, to, equity, bh })
}

pub fn run_json(st: &State, q: &Query) -> Result<String, LabError> {
    let spec = spec_of(q)?;
    let q = &with_spec_cost(q, &spec)?;
    let selected = select(st, q)?;
    let held = selected.context(st);
    let ctx: &Ctx = &held;
    let (kind, instrument) = instrument(&selected, &ctx.r[0].ticker);
    known_type(kind, &ctx.r[0].ticker)?;
    let (from, to) = window(ctx, q)?;
    let costs = accounting::Costs::parse(q)?;
    costs.ensure_rollover()?;
    let cost = costs.side_rate();
    let fr = Frames::new(Bars::from_rows(ctx.r));
    let compiled = series::compile(&spec, &fr, &mut Cache::new())?;
    let run = backtest(ctx, &compiled, &fr.d, from, to, cost)?;
    let gross = crate::Run { trades: run.trades.clone(), from, to, equity: crate::equity_curve(&ctx.c, &run.trades, from, to, 0.), bh: run.bh.clone() };
    let trades = arr(run.trades.iter().map(|t| format!(
        "{{\"entry\":\"{}\",\"exit\":\"{}\",\"entry_i\":{},\"exit_i\":{},\"entry_px\":{},\"exit_px\":{},\"ret\":{},\"bars\":{},\"open\":{},\"signal\":\"{}\",\"reason\":\"{}\"}}",
        ctx.r[t.legs[0].i].date, ctx.r[t.exit_i].date, t.legs[0].i - from, t.exit_i - from, crate::num(t.legs[0].px), crate::num(t.exit_px),
        crate::num(t.ret(cost)), t.exit_i - t.legs[0].i, t.open, ctx.r[t.signal_i].date, esc(&t.reason))));
    Ok(format!(
        "{{\"label\":\"{LABEL}\",\"kind\":\"lab_simulation\",\"instrument\":{instrument},\"from\":\"{}\",\"to\":\"{}\",\"dates\":{},\"close\":{},\"equity\":{},\"gross_equity\":{},\"bh\":{},\"trades\":{trades},\"stats\":{},\"gross_stats\":{},\"costs\":{},\"provenance\":{}}}",
        ctx.r[from].date, ctx.r[to].date, quoted((from..=to).map(|k| ctx.r[k].date.as_str())), arr((from..=to).map(|k| crate::num(ctx.c[k]))),
        arr(run.equity.iter().map(|v| crate::num(*v))), arr(gross.equity.iter().map(|v| crate::num(*v))), arr(run.bh.iter().map(|v| crate::num(*v))),
        crate::stats_json(&crate::financial_stats(ctx, &run, costs)), crate::stats_json(&crate::stats(ctx, &gross, 0.)), costs.json(), crate::source_provenance(st, q)))
}
```

- [ ] **Step 4: Run tests** — `CARGO_BUILD_JOBS=2 cargo test lab::api` → Expected: 5 passed.

- [ ] **Step 5: Guard + commit**

```bash
python3 scripts/check_book_untouched.py --base origin/Manan
git add Context/guru/backend/src/lab/api.rs
git commit -m "Run lab rule specs with book trade validation, costs and statistics"
```

---

### Task 8: `/api/lab/optimize` — grid over spec paths with the book ranking

**Files:**
- Modify: `Context/guru/backend/src/lab/api.rs`

**Interfaces:**
- Consumes: `crate::{FULL_COLS, PART_COLS, RANK_STATS, MIN_TRADES, MIN_PER_YEAR, years, conclude_rank_with_outcomes}` (`main.rs:786–787`, `:915–917`, `:299`, `:931`); `rules::{apply, current}`.
- Produces: `GET /api/lab/optimize?…run params…&split=YYYY-MM-DD&slim=1` → `{"label","kind":"lab_optimization","instrument","from","to","split":"date|null","combos","threads","ms","dims":[{"path","values"}],"cols":[paths…, FULL_COLS…, "is_"+PART_COLS…, "oos_"+PART_COLS…],"base":[metrics of the spec as sent],"rank":{conclude_rank JSON},"picked":{"<row>":[…]},"rows":[[…]],"costs","provenance"}`. Rows are in mixed-radix order (last path fastest), as `optimize()` (`main.rs:792`). Threads = `available_parallelism().min(combos)`, one `Cache` per thread; a worker stops with `invalid` after 300 s (no partial ranking).

- [ ] **Step 1: Write the failing test** — add inside `mod tests` of `api.rs`:

```rust
    #[test] fn optimize_ranks_like_the_book_optimizer() {
        let st = state(700, "SPX", false);
        let j = optimize_json(&st, &q(&[("split", "2016-06-01")])).unwrap();
        let v = crate::lab::json::parse(&j).unwrap();
        assert_eq!(v.get("combos").and_then(|c| c.as_f64()), Some(9.));
        assert!(j.starts_with(&format!("{{\"label\":\"{LABEL}\",\"kind\":\"lab_optimization\"")));
        assert!(j.contains("\"cols\":[\"entry.all[0].rhs.p.length\",\"exit.any[0].rhs.value\",\"total_return\""));
        assert!(j.contains("\"is_total_return\"") && j.contains("\"oos_t_stat\""));
        let crate::lab::json::Json::Arr(rows) = v.get("rows").unwrap() else { panic!() };
        assert_eq!(rows.len(), 9);
        // first row = first value of every dimension, last dimension fastest
        assert!(j.contains("\"rows\":[[3,2,") && j.contains("[3,3,"));
        assert!(v.get("rank").and_then(|r| r.get("eligible")).is_some());
        let big = SPEC.replace("\"from\":3,\"to\":7,\"step\":2", "\"from\":1,\"to\":1000,\"step\":1").replace("\"from\":2,\"to\":4", "\"from\":1,\"to\":100");
        assert!(optimize_json(&st, &q(&[("spec", &big)])).unwrap_err().message().contains("more than 50000"));
        assert!(optimize_json(&st, &q(&[("split", "2015-01-05")])).unwrap_err().message().contains("split"));
        let none = SPEC.split(",\"optimize\"").next().unwrap().to_string() + "}";
        assert!(optimize_json(&st, &q(&[("spec", &none)])).unwrap_err().message().contains("no parameter ranges"));
    }
```

- [ ] **Step 2: Run to verify failure** — `CARGO_BUILD_JOBS=2 cargo test lab::api` → Expected: compile error (`optimize_json` not found).

- [ ] **Step 3: Implement**

Add the `OPT_SECONDS` constant after `type Query = BTreeMap<String, String>;` and the `optimize_json` function above `#[cfg(test)]` (both shown here):
```rust
/// Wall-clock bound for one lab optimization (the gateway allows 600 s).
const OPT_SECONDS: u64 = 300;

pub fn optimize_json(st: &State, q: &Query) -> Result<String, LabError> {
    let t0 = std::time::Instant::now();
    let spec = spec_of(q)?;
    if spec.optimize.is_empty() { return Err(LabError::Invalid("optimize lists no parameter ranges".into())); }
    let q = &with_spec_cost(q, &spec)?;
    let selected = select(st, q)?;
    let held = selected.context(st);
    let ctx: &Ctx = &held;
    let (kind, instrument) = instrument(&selected, &ctx.r[0].ticker);
    known_type(kind, &ctx.r[0].ticker)?;
    let (from, to) = window(ctx, q)?;
    let costs = accounting::Costs::parse(q)?;
    costs.ensure_rollover()?;
    let cost = costs.side_rate();
    let dates: Vec<&str> = ctx.r.iter().map(|r| r.date.as_str()).collect();
    let split = match q.get("split").filter(|s| !s.is_empty()) {
        None => None,
        Some(s) => Some(Some(dates.partition_point(|d| *d < s.as_str())).filter(|&k| k > from + 20 && k + 20 < to)
            .ok_or_else(|| LabError::Invalid("split must leave more than 20 bars on each side of the range".into()))?),
    };
    let fr = Frames::new(Bars::from_rows(ctx.r));
    let dims = &spec.optimize;
    let combos: usize = dims.iter().map(|d| d.values.len()).product();
    let metrics = |cache: &mut Cache, s: &Spec| -> Result<Vec<f64>, LabError> {
        let c = series::compile(s, &fr, cache)?;
        let stats = crate::financial_stats(ctx, &backtest(ctx, &c, &fr.d, from, to, cost)?, costs);
        let mut out: Vec<f64> = crate::FULL_COLS.iter().map(|k| stats[k]).collect();
        for (a, b) in split.map(|s| [(from, s - 1), (s, to)]).into_iter().flatten() {
            let part = crate::financial_stats(ctx, &backtest(ctx, &c, &fr.d, a, b, cost)?, costs);
            out.extend(crate::PART_COLS.iter().map(|k| part[k]));
        }
        Ok(out)
    };
    let base = metrics(&mut Cache::new(), &spec)?;
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get()).min(combos);
    // Rows in mixed-radix order (last dimension fastest), as the book optimizer (main.rs optimize()).
    let rows_v: Vec<Vec<f64>> = std::thread::scope(|sc| -> Result<Vec<Vec<f64>>, LabError> {
        let hs: Vec<_> = (0..threads).map(|t| {
            let (metrics, spec) = (&metrics, &spec);
            sc.spawn(move || {
                let mut cache = Cache::new();
                (t..combos).step_by(threads).map(|k| {
                    if t0.elapsed().as_secs() >= OPT_SECONDS {
                        return Err(LabError::Invalid(format!("optimization stopped after {OPT_SECONDS} s; narrow the grid. No partial ranking is published.")));
                    }
                    let (mut r, mut vals) = (k, vec![0.0; dims.len()]);
                    for (j, d) in dims.iter().enumerate().rev() { vals[j] = d.values[r % d.values.len()]; r /= d.values.len(); }
                    let s = rules::apply(spec, dims, &vals);
                    vals.extend(metrics(&mut cache, &s)?);
                    Ok((k, vals))
                }).collect::<Result<Vec<_>, LabError>>()
            })
        }).collect();
        let mut all: Vec<(usize, Vec<f64>)> = Vec::new();
        for h in hs { all.extend(h.join().map_err(|_| LabError::Invalid("optimization worker failed; no result is published".into()))??); }
        all.sort_by_key(|r| r.0);
        Ok(all.into_iter().map(|r| r.1).collect())
    })?;
    // Ranking exactly as the book optimizer (main.rs optimize(): trade-rate floor, outcome columns, conclude_rank).
    let col = |name: &str| dims.len() + crate::FULL_COLS.iter().position(|c| *c == name).unwrap();
    let yrs = crate::years(ctx, from, to);
    let base_tpy = base[crate::FULL_COLS.iter().position(|c| *c == "trades").unwrap()] / yrs;
    let per_year = (base_tpy * 0.5).clamp(crate::MIN_PER_YEAR.0, crate::MIN_PER_YEAR.1);
    let min_trades = (per_year * yrs).ceil().max(crate::MIN_TRADES);
    let defs: Vec<f64> = dims.iter().map(|d| rules::current(&spec, &d.target)).collect();
    let outcome_cols: Vec<usize> = (dims.len()..dims.len() + 16).chain(dims.len() + crate::FULL_COLS.len()..rows_v.first().map_or(0, Vec::len)).collect();
    let (rank_json, pool) = crate::conclude_rank_with_outcomes(&rows_v, col("trades"), &crate::RANK_STATS.map(|(k, _)| col(k)), min_trades, per_year, &defs, Some(&outcome_cols));
    let slim = q.get("slim").is_some_and(|v| v == "1");
    let row_s = |r: &Vec<f64>| arr(r.iter().map(|v| crate::num(*v)));
    let rows = if slim { String::new() } else { rows_v.iter().map(row_s).collect::<Vec<_>>().join(",") };
    let picked = if slim { pool.iter().map(|&i| format!("\"{i}\":{}", row_s(&rows_v[i]))).collect::<Vec<_>>().join(",") } else { String::new() };
    let mut cols: Vec<String> = dims.iter().map(|d| d.path.clone()).collect();
    cols.extend(crate::FULL_COLS.iter().map(|c| c.to_string()));
    for part in split.map(|_| ["is_", "oos_"]).into_iter().flatten() { cols.extend(crate::PART_COLS.iter().map(|c| format!("{part}{c}"))); }
    let dims_json = arr(dims.iter().map(|d| format!("{{\"path\":\"{}\",\"values\":{}}}", esc(&d.path), arr(d.values.iter().map(|v| crate::num(*v))))));
    Ok(format!(
        "{{\"label\":\"{LABEL}\",\"kind\":\"lab_optimization\",\"instrument\":{instrument},\"from\":\"{}\",\"to\":\"{}\",\"split\":{},\"combos\":{combos},\"threads\":{threads},\"ms\":{},\"dims\":{dims_json},\"cols\":{},\"base\":{},\"rank\":{rank_json},\"picked\":{{{picked}}},\"rows\":[{rows}],\"costs\":{},\"provenance\":{}}}",
        dates[from], dates[to], split.map_or("null".into(), |s| format!("\"{}\"", dates[s])), t0.elapsed().as_millis(),
        quoted(cols.iter().map(String::as_str)), arr(base.iter().map(|v| crate::num(*v))), costs.json(), crate::source_provenance(st, q)))
}
```
Add the arm to `route()` after `"/api/lab/run" => run_json,`:
```rust
        "/api/lab/optimize" => optimize_json,
```

- [ ] **Step 4: Run tests** — `CARGO_BUILD_JOBS=2 cargo test lab::` → Expected: all lab tests pass (6 in `lab::api`).

- [ ] **Step 5: Real-data smoke (bounded, read-only data)**

```bash
cd Context/guru/backend && CARGO_BUILD_JOBS=2 cargo build --release
NAS="/run/user/1000/gvfs/smb-share:server=192.168.4.50,share=share/Manan Pujara/Book-to-rust"
PORT=$(python3 -c 'import socket;s=socket.socket();s.bind(("127.0.0.1",0));print(s.getsockname()[1])')
./target/release/guru "$NAS/idx_data.csv" "$PORT" /nonexistent /nonexistent >/dev/null 2>&1 & PID=$!
until curl -s -o /dev/null "http://127.0.0.1:$PORT/api/gurus"; do sleep 0.5; done
SPEC='{"v":1,"entry":{"all":[{"lhs":{"s":"rsi","p":{"length":2}},"op":"crosses_below","rhs":{"value":10}},{"lhs":{"s":"close"},"op":">","rhs":{"s":"sma","p":{"length":40},"tf":"W"}}]},"exit":{"any":[{"lhs":{"s":"rsi","p":{"length":2}},"op":"crosses_above","rhs":{"value":70}},{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":10}}]},"optimize":[{"path":"entry.all[0].lhs.p.length","from":2,"to":6,"step":1},{"path":"entry.all[0].rhs.value","from":5,"to":30,"step":5}]}'
ENC=$(python3 -c 'import sys,urllib.parse;print(urllib.parse.quote(sys.argv[1],safe=""))' "$SPEC")
curl -s "http://127.0.0.1:$PORT/api/lab/run?ticker=NIFTY%2050&from=2005-01-01&cost=5&spec=$ENC" | python3 -c 'import sys,json;d=json.load(sys.stdin);print(d["label"],d["instrument"]["type"],d["stats"]["trades"])'
curl -s "http://127.0.0.1:$PORT/api/lab/optimize?ticker=NIFTY%2050&from=2005-01-01&split=2016-01-01&slim=1&spec=$ENC" | python3 -c 'import sys,json;d=json.load(sys.stdin);print(d["combos"],d["rank"]["eligible"])'
curl -s -w ' %{http_code}\n' "http://127.0.0.1:$PORT/api/lab/run?ticker=NIFTY%2050&spec=%7B"
kill $PID
```
Expected: label + `index` + a trade count (175 at planning time); `30` combos with an `eligible` count; the malformed spec returns `{"status":"invalid","error":"spec is not valid JSON: …","label":…} 400`.

- [ ] **Step 6: Guard + commit**

```bash
python3 scripts/check_book_untouched.py --base origin/Manan
git add Context/guru/backend/src/lab/api.rs
git commit -m "Optimize lab rule specs with the book optimizer's cap and ranking"
```

---

### Task 9: Gateway — allow lab routes, bound specs, single-slot lab optimization

**Files:**
- Modify: `Context/guru/webapp/gateway/src/policy.rs` (additive only)
- Modify: `Context/guru/webapp/gateway/src/main.rs` (three added lines, no edited line)

**Interfaces:**
- Produces: `allowed_engine_path` accepts `lab/catalogue`, `lab/series`, `lab/run`, `lab/optimize` (four new `| "…"` lines inside the existing multi-line `matches!`, after `| "guru/chande/optimize"`); `pub fn is_lab_optimization_path(path: &str) -> bool`; `pub const LAB_SPEC_MAX_BYTES: usize = 8192; pub const LAB_MAX_CONDITIONS: usize = 12;`; `pub fn lab_spec_problem(path: &str, params: &BTreeMap<String,String>) -> Option<&'static str>` (only for `lab/run` and `lab/optimize`: spec present, ≤ 8 KB, valid JSON, ≤ 12 conditions, every number finite and ≤ 1e12, each optimize range numeric with positive step and ≤ 1000 values, ≤ 50,000 combinations). Gateway errors keep the existing `{"error": "…"}` shape (`main.rs` `error()`).
- `is_optimization_path` is a single-line `matches!`; adding a pattern would edit a line, so lab optimization uses the new function and a shadowing line in `main.rs`. `lab/optimize` therefore gets: researcher/admin role (`can_optimize`), cross-site rejection, the single optimization slot (`try_acquire`, 429 + `Retry-After: 5`) and the 600 s timeout — the same path as book optimizers. `p.*` for `lab/series` stays bounded by `safe_engine_numbers`.

- [ ] **Step 1: Write the failing tests** — append to `policy.rs`:

```rust
#[cfg(test)]
mod lab_tests {
    use super::*;
    use std::collections::BTreeMap;
    const SPEC: &str = r#"{"v":1,"entry":{"all":[{"lhs":{"s":"close"},"op":">","rhs":{"value":1}}]},"exit":{"any":[{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":5}}]},"optimize":[{"path":"exit.any[0].rhs.value","from":1,"to":10,"step":1}]}"#;
    fn q(spec: &str) -> BTreeMap<String, String> {
        BTreeMap::from([("spec".to_string(), spec.to_string())])
    }
    #[test]
    fn lab_routes_are_allowlisted_and_lab_optimize_is_an_optimization() {
        for route in ["catalogue", "series", "run", "optimize"] {
            assert!(allowed_engine_path(&format!("lab/{route}")));
        }
        assert!(!allowed_engine_path("lab/strategies"));
        assert!(!allowed_engine_path("lab/../notes"));
        assert!(is_lab_optimization_path("lab/optimize"));
        assert!(!is_lab_optimization_path("lab/run"));
        assert!(!is_optimization_path("lab/optimize"), "existing book list stays unchanged");
    }
    #[test]
    fn lab_specs_are_bounded_before_the_engine() {
        assert_eq!(lab_spec_problem("lab/run", &q(SPEC)), None);
        assert_eq!(lab_spec_problem("lab/catalogue", &BTreeMap::new()), None);
        assert_eq!(lab_spec_problem("guru/connors/run", &BTreeMap::new()), None);
        assert!(lab_spec_problem("lab/run", &BTreeMap::new()).is_some());
        let padded = SPEC.replacen("{", &format!("{{\"pad\":\"{}\",", "x".repeat(LAB_SPEC_MAX_BYTES)), 1);
        let cond = r#"{"lhs":{"s":"close"},"op":">","rhs":{"value":1}}"#;
        let thirteen = format!(r#"{{"v":1,"entry":{{"all":[{}]}},"exit":{{"any":[{}]}}}}"#, vec![cond; 7].join(","), vec![cond; 6].join(","));
        for (spec, why) in [
            (padded.as_str(), "8 KB"),
            ("{", "valid JSON"),
            (thirteen.as_str(), "12 conditions"),
            (&SPEC.replace("\"value\":1}", "\"value\":1e13}"), "finite"),
            (&SPEC.replace("\"to\":10", "\"to\":5000"), "1,000 values"),
            (&SPEC.replace("\"step\":1", "\"step\":0"), "positive step"),
            (&SPEC.replace("\"step\":1}", "\"step\":1},{\"path\":\"x\",\"from\":1,\"to\":1000,\"step\":1},{\"path\":\"y\",\"from\":1,\"to\":1000,\"step\":1}"), "50,000"),
        ] {
            let problem = lab_spec_problem("lab/optimize", &q(spec)).unwrap_or_else(|| panic!("accepted {spec}"));
            assert!(problem.contains(why), "{why}: {problem}");
        }
    }
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cd Context/guru/webapp && CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="${VAULT_RUNTIME_ROOT:-$HOME/.cache/strategy-vault-webapp}/test-target" cargo test --locked --manifest-path gateway/Cargo.toml` → Expected: compile error (`is_lab_optimization_path`, `lab_spec_problem` missing).

- [ ] **Step 3: Implement**

In `allowed_engine_path`, insert after the line `            | "guru/chande/optimize"`:
```rust
            | "lab/catalogue"
            | "lab/series"
            | "lab/run"
            | "lab/optimize"
```
Append to `policy.rs` (above the new test module):
```rust
/// Indicators lab: one optimization at a time and research roles only, like the book optimizers.
pub fn is_lab_optimization_path(path: &str) -> bool {
    path == "lab/optimize"
}
pub const LAB_SPEC_MAX_BYTES: usize = 8192;
pub const LAB_MAX_CONDITIONS: usize = 12;
/// Bounds a lab rule spec before it reaches the engine (the engine re-validates every field).
pub fn lab_spec_problem(path: &str, params: &std::collections::BTreeMap<String, String>) -> Option<&'static str> {
    if !matches!(path, "lab/run" | "lab/optimize") {
        return None;
    }
    let Some(spec) = params.get("spec") else { return Some("A lab rule spec is required.") };
    if spec.len() > LAB_SPEC_MAX_BYTES {
        return Some("The rule spec is larger than 8 KB.");
    }
    let Ok(value) = serde_json::from_str::<serde_json::Value>(spec) else { return Some("The rule spec is not valid JSON.") };
    let count = |list: &str, key: &str| value.get(list).and_then(|v| v.get(key)).and_then(|v| v.as_array()).map_or(0, Vec::len);
    if count("entry", "all") + count("exit", "any") > LAB_MAX_CONDITIONS {
        return Some("A rule spec allows at most 12 conditions.");
    }
    fn numbers_ok(v: &serde_json::Value) -> bool {
        match v {
            serde_json::Value::Number(n) => n.as_f64().is_some_and(|x| x.is_finite() && x.abs() <= 1e12),
            serde_json::Value::Array(a) => a.iter().all(numbers_ok),
            serde_json::Value::Object(o) => o.values().all(numbers_ok),
            _ => true,
        }
    }
    if !numbers_ok(&value) {
        return Some("Rule spec numbers must be finite and no larger than 1e12.");
    }
    let mut combinations = 1usize;
    for item in value.get("optimize").and_then(|v| v.as_array()).into_iter().flatten() {
        let n = |k: &str| item.get(k).and_then(serde_json::Value::as_f64);
        let (Some(lo), Some(hi), Some(step)) = (n("from"), n("to"), n("step")) else {
            return Some("Each optimize range needs numeric from, to and step.");
        };
        let count = ((hi - lo) / step + 1e-9).floor() + 1.0;
        if step <= 0.0 || hi < lo || !(1.0..=1000.0).contains(&count) {
            return Some("Each optimize range needs a positive step and at most 1,000 values.");
        }
        combinations = match combinations.checked_mul(count as usize) {
            Some(c) if c <= 50_000 => c,
            _ => return Some("Use a grid of at most 50,000 combinations."),
        };
    }
    None
}
```
In `gateway/src/main.rs` `engine_proxy`, directly after the `safe_engine_numbers` block (the `return error(StatusCode::BAD_REQUEST,"Use finite parameter values …");` and its closing `}`), add the lab bound; and directly after `let optimizing = policy::is_optimization_path(&path);` add the shadowing line, so the result is:
```rust
    if !policy::safe_engine_numbers(&query) {
        return error(StatusCode::BAD_REQUEST,"Use finite parameter values and a grid of at most 50,000 combinations, with at most 1,000 values per setting.");
    }
    if let Some(problem) = policy::lab_spec_problem(&path, &query) {
        return error(StatusCode::BAD_REQUEST, problem);
    }
    let optimizing = policy::is_optimization_path(&path);
    let optimizing = optimizing || policy::is_lab_optimization_path(&path);
```

- [ ] **Step 4: Run tests** — same command as Step 2 → Expected: 6 passed. `git diff origin/Manan -- Context/guru/webapp/gateway/src` shows only `+` lines.

- [ ] **Step 5: Integration test decision** — do not add a case to `tests/integration.test.mjs`: it signs in with the private `.env` admin credentials and calls engines loaded with the private NAS data on the deployed stack, so it cannot run without private data (Decision 15). Record this in the PR description.

- [ ] **Step 6: Guard + commit**

```bash
python3 scripts/check_book_untouched.py --base origin/Manan
git add Context/guru/webapp/gateway/src/policy.rs Context/guru/webapp/gateway/src/main.rs
git commit -m "Allow lab engine routes with bounded specs and single-slot optimization"
```

---

### Task 10: Phase gate

- [ ] **Step 1: Engine suite** — `cd Context/guru/backend && VAULT_IDX="/run/user/1000/gvfs/smb-share:server=192.168.4.50,share=share/Manan Pujara/Book-to-rust/idx_data.csv" CARGO_BUILD_JOBS=2 cargo test` → all pass; record the count and the count on `origin/Manan` (this phase adds 32 lab tests: json 4, rules 7, series 9, simulate 6, api 6; 677 passed / 1 ignored at planning time).
- [ ] **Step 2: Scripts** — `python3 -m unittest discover -s scripts -p 'test_*.py'` and `python3 scripts/check_upload.py` → pass.
- [ ] **Step 3: Book guard, extended** — `NAS="/run/user/1000/gvfs/smb-share:server=192.168.4.50,share=share/Manan Pujara/Book-to-rust"; python3 scripts/check_book_untouched.py --base origin/Manan --engine-compare --csv "$NAS/idx_data.csv" --vix "$NAS/india_vix" --extra "$NAS/extra_data"` → `"problems": []`; copy the `notes` (hash count, basket skip or basket tickers, ross optimize note) into the phase report. If `VAULT_BASKET_DATA_DIR` is available locally, run once more with it set so three catalogued stocks are hashed.
- [ ] **Step 4: Gateway** — `cd Context/guru/webapp && CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="${VAULT_RUNTIME_ROOT:-$HOME/.cache/strategy-vault-webapp}/test-target" cargo test --locked --manifest-path gateway/Cargo.toml` → pass.
- [ ] **Step 5: Release build without lab warnings** — `cd Context/guru/backend && touch src/main.rs && CARGO_BUILD_JOBS=2 cargo build --release 2>&1 | grep -A3 '^warning' | grep -- '--> src/lab/'` → no output (pre-existing warnings in `books/*`, `baskets.rs`, `accounting.rs` and `main.rs` `conclude_rank` are not this phase's).
- [ ] **Step 6: Additive-only proof** — `git diff origin/Manan --stat` lists only new `lab/*.rs` files, `lab/mod.rs`, `main.rs` (+1), gateway `policy.rs`/`main.rs` (additions), the guard and its test, and this plan; `git diff origin/Manan -U0 -- Context/guru/backend/src/main.rs Context/guru/webapp/gateway/src | grep '^-[^-]'` → no output.
- [ ] **Step 7: Report** — commit any gate fixes, then draft the PR description (tests before/after, guard notes, decisions 1–15, integration-test note). Do not push; the user pushes.
