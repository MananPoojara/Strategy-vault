# Indicators Lab — Phase 5 (`engine-gl`: global and crypto with provisional labels) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Serve the indicator lab on the provisional global and crypto TradingView exports through a third engine instance `engine-gl` (same engine image) at `/api/gl/lab/*`, with every response flagged provisional, crypto annualised on its 7-day calendar, close-only/stale history excluded from next-open fills, absent data answered as `needs_data`, and no book strategy reachable on that data.

**Architecture:** One new lab file `Context/guru/backend/src/lab/markets.rs` declares the ten provisional instruments (TradingView `EXCHANGE:SYMBOL` ids, currency, timezone, calendar, bars/year, source, quirks) and the pure rules: `tradable_from` (first year with real opens), `annualise` (√(bars/252) rescale of the √252 book ratios), the engine-gl route guard and the provisional body marker. `lab/api.rs` uses it in `instrument()`, `select_error()`, `window()`, the stats calls and `serve()`, and gains `/api/lab/instruments`. `main.rs` gains one guard line so an engine started with `VAULT_LAB_MARKET=gl` answers lab routes only. `webapp/scripts/stage_gl.py` (called by two added lines in `setup.py`) converts the NAS exports into one engine CSV on local disk. The gateway accepts market `gl` for `lab/*` paths only (`policy::engine_market_allows`) and proxies it to `VAULT_ENGINE_GL`; `compose.yaml` adds the `engine-gl` service (same image, only `data/gl` mounted read-only).

**Tech Stack:** Rust 2021 std only for the engine (`cargo test`); gateway keeps its crates (axum 0.8, serde_json); Python 3 std-lib (+ PyYAML already on the gate machine, for the compose check); Docker Compose (manual gate only).

**Spec:** `docs/superpowers/specs/2026-10-14-indicators-lab-design.md` (§1 Markets; §2/§2a constraints; §3 "Global and crypto", "Instrument metadata", "Gateway"; §7 error handling; §8 testing; §9 phase 5 exit criterion "setup + compose verified on an isolated stack"). Format and conventions: `docs/superpowers/plans/2026-10-15-indicators-lab-phase2-rules-engine.md`; publish rule for gl/crypto: `docs/superpowers/plans/2026-10-15-indicators-lab-phase4-saved-strategies.md` Decision 5.

**Planning evidence:** every code block below was compiled and its tests run against a scratch copy of `Nandini` at `824632c` (exported with `git archive`, outside the repository; `d165472`, the HEAD when this plan was committed, changes only auth-service files). Engine: 701 passed / 1 ignored (baseline 690 / 1; +11: markets 6, api 5), with `VAULT_IDX` and `VAULT_GL_CSV` real data; release build added no `src/lab/` warnings. Gateway: 7 passed (baseline 6). Webapp scripts: 7 passed. `stage_gl.py` was run read-only against the NAS `Global Bhavcopy` folder into the scratch directory (10 instruments, 84,721 bars); the real-data test confirmed every declared `tradable_from` cut. The release binary was run with `VAULT_LAB_MARKET=gl` on a free localhost port against that staged file and against a header-only file (zero instruments): book routes 404 with the provisional flag, `/api/lab/instruments` 200, a CRYPTO:BTCUSD run 200 starting `2011-01-01` with `start.reason` "opens before 2011-01-01 are stale", HSI:HSI `from=1990-01-02` 400 `needs_data`, OBV on IG:DAX 400 `needs_data`, NIFTY 50 400 `invalid`. No Docker command was run.

## Global Constraints

- Branch Nandini; never main; no force-push; no git worktrees (user preference); commits without any Co-Authored-By trailer; never push (the user pushes). Another agent may commit concurrently: stage only the files named in the task (`git add <paths>`, never `git add -A`).
- Book results never touched: protected files `connors.rs`, `books/**`, `readiness.rs`, `gate.rs`, `instrument_policy.rs`, `baskets.rs`, `accounting.rs`, `trade_validation.rs`, frontend `compat/*`, `ResearchPages.tsx` — no edits; `main.rs`, gateway `policy.rs` and `custom-pages.ts` additive only (the guard enforces removed lines). This phase adds exactly one line to `main.rs` (Task 4).
- std-only Rust engine, no new crates; gateway uses its existing crates only; Python std-lib only in shipped scripts.
- CARGO_BUILD_JOBS=2; full engine suites need `VAULT_IDX="/run/user/1000/gvfs/smb-share:server=192.168.4.50,share=share/Manan Pujara/Book-to-rust/idx_data.csv"`. The NAS is read-only: never write there; staged output goes to a scratch or runtime directory on local disk. No whole-corpus backtests; no backtest at startup.
- Raw market data, `.env`, staged CSVs and generated results stay outside Git (`check_upload.py` blocks `.csv`).
- Never touch the live deployment (192.168.4.30:8180, Compose project `strategy-vault`). **No agent runs `docker` or `docker compose`.** Compose/isolated-stack verification is a manual gate item for the user (Task 7 Step 8).
- Provisional data: every engine-gl lab body carries `"provisional":true`; every gl/crypto instrument JSON carries `"provisional":true` and `"label":"Provisional · <source>"`; gl/crypto stay unpublishable to YT (Phase 4 `LAB_PUBLISHABLE_MARKETS` = in, us and the DB `CHECK` are unchanged).
- An index never substitutes for stocks; a provisional TradingView copy never substitutes for an admitted India/US series (Decision 1).
- Phase 2 rulings still hold: ranking with a split uses the in-sample window only; stale opens move the start or refuse with `needs_data`; an unknown ticker is `invalid`; data gaps are `needs_data`; every lab body carries the Phase 2 `label`.
- India/US lab responses stay byte-identical (no new fields; `annualise` returns early at 252).

## Review Focus

1. **A book strategy (or any non-lab route) answering on provisional data** — through the gateway (`/api/gl/guru/connors/run`, `/api/gl/tickers`, `/api/gl/notes`) or directly on the engine-gl container: gateway `global_and_crypto_reach_lab_routes_only` (Task 5); engine `engine_gl_serves_lab_routes_only_and_marks_every_body` (Task 1) and the Task 4 smoke run (`/api/guru/connors/run` → 404 JSON with the provisional flag).
2. **A provisional number shown without its flag** — an engine-gl error body, a 404, the catalogue or the instruments list arriving unmarked, or a crypto result looking like an India/US one: `engine_gl_responses_all_carry_the_provisional_flag` and `gl_instruments_are_labelled_provisional_and_annualised_by_their_own_calendar` (Task 2).
3. **Next-open fills on close-only or stale history** (Bitcoin 2009–2010, HSI 1987–1991, Russell 2000 to 2006, FTSE CFD 1999–2010): `tradable_from_skips_close_only_and_stale_open_years` (Task 1), `gl_close_only_years_move_the_start` (Task 2), real-data `staged_gl_data_matches_the_declared_cuts` (Task 4 Step 4 with `VAULT_GL_CSV`).
4. **A TradingView duplicate of an admitted series (SPX/DJI/NDX CFDs) or a mis-identified instrument (RUS_MOEX) offered on gl, or a gl id colliding with an India/US ticker**: `test_manifest_matches_the_engine_instrument_table` (exclusion list, Task 3), `every_gl_instrument_is_provisional_and_declares_its_calendar` (`gl("SPX")` is none, ids contain `:`, Task 1).
5. **No staged global data** (live or isolated stack before `setup.py` is re-run, NAS folder renamed, a corrupt export): engine-gl must start with zero instruments, answer `needs_data` per instrument and leave India/US healthy — `test_no_source_data_still_writes_a_loadable_empty_file`, `test_a_bad_row_or_duplicate_date_leaves_the_whole_instrument_out` (Task 3), `gl_data_gaps_are_needs_data_and_unknown_tickers_stay_invalid` (Task 2), the header-only smoke run (Task 4 Step 6), `test_gateway_routes_gl_to_engine_gl` (`service_started`, Task 6), manual gate (Task 7 Step 8).

---

## Data found on the NAS (read-only inspection, 2026-10-14)

Folder: `/run/user/1000/gvfs/smb-share:server=192.168.4.50,share=share/Manan Pujara/Book-to-rust/Global Bhavcopy/` (TradingView daily exports; dates `DD-MM-YYYY` except one ISO file). Weekday counts, close-only bars (O=H=L=C) and opens equal to the previous close were measured per calendar year.

| Source file | Rows | Range | Header | Volume | Calendar notes | Decision |
|---|---|---|---|---|---|---|
| `HSI_HSI, 1D_1033e.csv` | 9,828 | 1986-12-31 → 2026-09-24 | `Ticker,time,open,…,Volume` | blank | Mon–Fri; close-only 1987–1989, stale opens 1990–1991 | staged `HSI:HSI`, cut 1992-01-01 |
| `IG_DAX, 1D_6e586.csv` | 8,795 | 1995-01-02 → 2026-09-24 | same | CFD ticks, Sunday 0 | **Sunday bars since 2010-08-01 (~51/yr), 311 bars/yr 2011–2025** | staged `IG:DAX`, no cut |
| `IG_FTSE, 1D_aecd1.csv` | 8,782 | 1995-01-03 → 2026-09-24 | same | CFD ticks | Sunday bars as DAX; opens = previous close 1999–2010 | staged `IG:FTSE`, cut 2011-01-01 |
| `NASDAQ_DLY_IXIC, 1D_71afe.csv` | 14,025 | 1971-02-05 → 2026-09-23 | `time,open,high,low,close` (ISO dates) | none | close-only to 1984, stale opens 1996–1999 | staged `NASDAQ_DLY:IXIC`, cut 2000-01-01 |
| `SPREADEX_NIKKEI, 1D_b84f8.csv` | 2,474 | 2016-05-23 → 2026-09-24 | `Ticker,time,…,Volume` | spread-bet ticks | Mon–Fri, some missing sessions | staged `SPREADEX:NIKKEI`, no cut |
| `TVC_CAC40, 1D_6ed46.csv` | 9,816 | 1987-12-31 → 2026-09-24 | `Ticker,time,open,high,low,close` | none | Mon–Fri | staged `TVC:CAC40`, no cut |
| `New folder/BSE_DLY_SENSEX, 1D_31c7b.csv` | 10,961 | 1979-04-03 → 2026-09-22 | `Ticker,Date,open,…` | none | 7 Saturday + 2 Sunday special sessions; close-only to 1990; 29% stale opens 2009 | staged `BSE_DLY:SENSEX`, cut 2010-01-01 |
| `New folder/TVC_RUT_SMALLCAP 2000 INDEX, 1D_0ea63.csv` | 9,881 | 1987-06-29 → 2026-09-21 | `Date,time,…` (the "Date" column holds the name) | none | close-only to Oct 1992, stale opens 1993–2006 | staged `TVC:RUT`, cut 2007-01-01 |
| `New folder/CRYPTO_BTCUSD, 1D_86731.csv` | 6,099 | 2009-10-05 → 2026-09-22 | `Ticker,time,…` | none | 7-day (~364 bars/yr); close-only 2009–2010 | staged `CRYPTO:BTCUSD`, cut 2011-01-01 |
| `New folder/CRYPTO_ETHUSD, 1D_8ff42.csv` | 4,060 | 2015-08-07 → 2026-09-22 | same | none | 7-day; 6 close-only bars in 2015 (4%) | staged `CRYPTO:ETHUSD`, no cut |
| `SPCFD_SPX, 1D_9b062.csv`, `DJCFD_DJI, 1D_7c1bd.csv`, `NASDAQ_DLY_NDX, 1D_8be36.csv` | — | — | — | — | duplicates of admitted `SPX`, `DJI`, `NDX` on `/api/us` | **not staged** (Decision 1) |
| `RUS_MOEX, 1D_60d7f.csv` | 3,542 | 2013-02-15 → | — | yes | named "MOEX RUSSIA" but prices (55 → 148) match Moscow Exchange PJSC shares, not the IMOEX index | **not staged** (identity unresolved) |
| `New folder/NASDAQ_DLY_IXIC, 1D_71afe.csv` | 14,025 | same as the top-level file | — | — | duplicate export | not staged |

No other global/crypto daily data was found (no other TradingView export folder at the research root; `extra_data/` holds India/US series only).

## Existing code this phase relies on (read-only unless stated)

| Item | Where | Used for |
|---|---|---|
| `load_csv(path)` — columns `Ticker,Date,Open,High,Low,Close[,Volume]`, `normalize_date` accepts DD-MM-YYYY/ISO, volume ≤ 0 or blank → `None`, sorts and dedups by date | `main.rs:105`, `:62` | the staged `gl_data.csv` loads unchanged |
| `main()` args `csv port vix_dir extra_dir`; a missing vix/extra dir only warns (`load_vix_dir` `:146`, `load_extra` `:197` return empty); 0 tickers is fine | `main.rs:1102` | engine-gl command line |
| `handle()`: `let (path, q) = query(target);` then routes; `p if p.starts_with("/api/lab/") => lab::api::serve(…)` | `main.rs:1036`, `:1092` | one guard line after `:1036` (Task 4) |
| `stats()` annualises `vol`, `sharpe`, `sortino` with `252f64.sqrt()`; `years()` and every rate use calendar days / 365.25; `financial_stats` adds `gross_sharpe` from the zero-cost `stats` | `main.rs:299`, `:347–349`, `:399–417` | `markets::annualise` rescales exactly those four keys |
| `State.ctxs: BTreeMap<String, Ctx>`; `Ctx.r: &[Row]` (private fields, visible to the child module `lab`) | `main.rs:505` | `instruments_json`, `tradable_from` |
| `select_context` → `Err("unknown ticker {t}")` for any ticker not loaded | `main.rs:557` | `select_error` maps a declared-but-unstaged gl id to `needs_data` |
| `gate::real_open_from(ticker)` (NDX/SPX/DJI cuts), consumed by `lab::api::window()` | `gate.rs:2329`, `lab/api.rs:122` | gl cut joins it with `.or(…)` |
| `instrument_policy::selected_type` → `"unknown"` for every gl id | `instrument_policy.rs:137` | gl ids are answered from `markets::gl` first (protected file untouched) |
| `lab::api::{serve, route, instrument, select_error, window, run_json, optimize_json}`; tests `state(n, ticker, volume)`, `q(extra)` | `lab/api.rs:24–310`, `:314`, `:322` | modified in Task 2 |
| `lab::resample::{resample, expand, Tf}` (weeks Monday–Sunday) | `lab/resample.rs` | Sunday-bar test |
| `lab::json::{parse, esc, Json}` | `lab/json.rs` | JSON checks, escaping |
| HV indicator already annualises with √365 (TradingView) | `lab/volatility.rs:40` | unchanged |
| gateway `engine_proxy`: market check `matches!(market, "in" \| "us")` (`gateway/src/main.rs:225`), engine pick (`:303`), `App { india, us }` (`:21`), env defaults (`:429–430`), `health()` checks auth + india + us (`:311`) | `gateway/src/main.rs` | two edited + two added lines (Task 5); `health()` unchanged |
| `allowed_engine_path` multi-line `matches!` ending `\| "lab/optimize"` | `gateway/src/policy.rs:2–29` | one added line |
| `compose.yaml` `engine-us` (image only, no build), `gateway.environment`, `gateway.depends_on` | `compose.yaml:73`, `:89–118` | engine-gl service + two gateway lines |
| `setup.py` copies required data, then `chmod`s everything under `data` (line 21) | `webapp/scripts/setup.py` | two added lines |
| `scripts/check_book_untouched.py --engine-compare`, `scripts/check_upload.py` | repo root | gate |

## Decisions (open points resolved without the user; each with reason and cost if wrong)

1. **Instrument set = ten exports (table above).** Reason: everything usable that exists. Left out: SPCFD:SPX, DJCFD:DJI, NASDAQ_DLY:NDX because admitted SPX/DJI/NDX already run on `/api/us` and a provisional copy must not substitute for them (AGENTS.md "no substitution"); RUS_MOEX because the name says index and the prices say stock; the second IXIC export as a duplicate. Cost if wrong: a researcher misses three US index CFD histories (re-adding = one manifest line + one table row + test) or MOEX (same, once its identity is confirmed).
2. **Ticker ids are TradingView `EXCHANGE:SYMBOL`** (`HSI:HSI`, `CRYPTO:BTCUSD`, …). Reason: names the source in the id, and the `:` never occurs in an India/US preloaded ticker (basket keys start `basket:`), so lab metadata keyed by id can be consulted on every engine without colliding. Cost if wrong: the frontend shows long ids — it displays `instrument.name` instead.
3. **Metadata is declared in `lab/markets.rs` (`GL`) and mirrored by the staging manifest; a test fails if the two lists differ.** Reason: `instrument_policy.rs`/`gate.rs` are protected; a runtime metadata file would need a `main.rs` loader. Cost if wrong: adding an instrument touches two files (the test points to both).
4. **Staging (`stage_gl.py`, called by `setup.py`).** Converts each export to `Ticker,Date,Open,High,Low,Close,Volume` in `<runtime>/data/gl/gl_data.csv` with the price text copied verbatim (floats only validate), dates normalised to ISO, Volume empty. Any unreadable, non-positive, non-finite or duplicate-date row leaves the **whole instrument** out (no repair); a missing file is a note; the header is always written (atomic replace) so engine-gl starts with zero instruments. Reason: the FUSE share is not readable by Docker (setup.py comment), and headers differ per export. Cost if wrong: one bad row hides an instrument (visible as `needs_data` with the setup note) — preferable to silently repaired prices.
5. **No volume is staged.** Reason: index exports carry none; IG/Spreadex "volume" is CFD tick count (Sunday 0), not traded volume. Volume indicators therefore answer `needs_data` (`Bars.volume` is `None`). Cost if wrong: OBV/MFI-style rules unavailable on gl until real volume is admitted.
6. **`tradable_from`: next-open fills start on 1 January after the last calendar year in which more than 20% of bars are close-only (O=H=L=C) or — on session markets — open exactly at the previous close; around-the-clock markets (crypto) count close-only bars only.** Computed from the loaded rows (so refreshed exports re-derive it), joined to `gate::real_open_from` in `window()`, so the Phase 2 behaviour applies unchanged: the start moves with a `start` disclosure, an earlier `from` is `needs_data`. Real-data cuts: SENSEX 2010, BTC 2011, HSI 1992, FTSE 2011, IXIC 2000, RUT 2007; ETH, DAX, Nikkei, CAC none. Reason: a next-open fill on a copied close is not a real fill. Cost if wrong: up to a year of usable history withheld (e.g. SENSEX 2009 at 29%), never fake fills.
7. **Bars per year are declared per instrument: crypto 365 (7-day calendar), IG CFDs 311 (measured 2011–2025 mean 310.7 incl. Sunday bars), every other gl index 252 (book convention; measured 246–256).** `annualise` multiplies `vol`, `sharpe`, `sortino`, `gross_sharpe` by √(bars/252); CAGR, years, trades/year, exposure and hold days already use calendar time; India/US return early (byte-identical). The optimizer ranking is unchanged (a positive constant does not reorder percentiles). Reason: `stats()` hard-codes √252 in protected-by-convention book code; rescaling in the lab avoids touching it. Cost if wrong: Sharpe/vol off by √(actual/declared) — ≤ 1.6% for indices; for IG windows before August 2010 (no Sunday bars) vol/Sharpe overstated by up to 11% (documented in `quirks`).
8. **Calendar quirks are declared, not repaired.** IG Sunday bars stay (TradingView shows them); weeks run Monday–Sunday in `resample`, so an IG weekly close is its Sunday bar (TradingView starts that symbol's week on the Sunday session — W values can differ from the chart; stated in `quirks`, pinned by a test showing no lookahead). SENSEX special weekend sessions stay. Reason: parity with the source over cosmetic cleaning. Cost if wrong: W/M operands on IG differ from the TradingView chart near week boundaries.
9. **engine-gl identity: `VAULT_LAB_MARKET=gl` (compose env).** The engine then (a) refuses every non-`/api/lab/` path with a 404 JSON body (one `main.rs` line) and (b) prefixes every lab body — results, errors, 404 — with `"provisional":true,"provisional_note":"Provisional · TradingView export · internal research only, not publishable to YT Strategies"`. Independently of the env var, a gl/crypto instrument JSON carries `name, timezone, calendar, bars_per_year, price (price|total_return), has_volume:false, provisional:true, label "Provisional · TradingView export <id>, daily", source, quirks`. India/US instrument JSON is unchanged. Reason: defence in depth for "book strategies are not offered on gl/crypto" (their admission rules know nothing about TradingView data) and one choke point (`serve`) for the flag. Cost if wrong: one extra `main.rs` line to remove.
10. **Gateway market list `in|us|gl`; `gl` reaches `lab/*` only** (`policy::engine_market_allows`, additive in `policy.rs`). `crypto` is not a URL market: the frontend maps saved market `crypto` → `/api/gl` (already in the Phase 4 frontend notes). Two existing lines of `gateway/src/main.rs` are edited (market check, engine pick) — that file is not in the additive-only list; `policy.rs` stays additive. A refused market/path answers the existing `400 Invalid market or query.` before any session lookup. Cost if wrong: none to India/US routing (same truth table, tested).
11. **Gateway health ignores engine-gl; compose `depends_on: engine-gl: service_started`.** Reason: a provisional research engine must not mark the whole site unhealthy or block India/US. Cost if wrong: an engine-gl outage shows as 502 on `/api/gl/*` only.
12. **engine-gl shares the gateway's single optimization slot and four read slots.** Reason: same CPU pair (`cpuset`), one optimization at a time is the existing contract. Cost if wrong: a gl optimization can make an India optimization wait (429 Retry-After 5), as two India users already do.
13. **New route `/api/lab/instruments`** (allowlisted on every market): all provisional instruments, `status` `ready` (with `first`, `last`, `bars`, `tradable_from`) or `needs_data` (with `reason`). Reason: `/api/gl/tickers` is a book route and is blocked on gl; the frontend needs a list. On `/api/in` and `/api/us` it lists the same instruments as `needs_data` ("served by /api/gl"). Cost if wrong: a harmless extra list on India/US.
14. **Errors:** a declared gl id that is not loaded → `needs_data` "… the TradingView export is not staged on this engine …"; any other unknown ticker stays `invalid` (Phase 2 ruling). A missing `ticker` still defaults to `NIFTY 50` (→ `invalid` on engine-gl); the frontend always sends `ticker`.
15. **Compose `engine-gl`:** same `${VAULT_ENGINE_IMAGE}` as engine-us (no `build`, no fork), command `[/data/gl/gl_data.csv, '8090', /data/gl/no-vix, /data/gl/no-extra]` (absent dirs by design: no VIX, no extra series), only `${VAULT_RUNTIME_ROOT}/data/gl` mounted `:ro`, no `ports`, `cpus: 1`, `mem_limit: 512m`, shared `cpuset`. If `setup.py` was not re-run, the bind mount is missing and engine-gl restart-loops while India/US stay healthy (Decision 11) — `scripts/start.sh` already runs `setup.py` first. Cost if wrong: a resource tweak.
16. **No DB, auth or frontend change.** Phase 4 already stores gl/crypto strategies and refuses their submission/publication; the DB `CHECK` keeps snapshots in/us. The UI work (show `instrument.label`, `provisional_note`, map crypto → gl, list from `/lab/instruments`) is recorded under "API for the frontend phase" below.
17. **Real-data engine test** `staged_gl_data_matches_the_declared_cuts` reads `VAULT_GL_CSV` and returns early (with a note) when unset, like the `VAULT_IDX` pattern; the gate sets it to a scratch staging of the NAS folder.
18. **Docs:** no README/DEVELOPMENT edit; the isolated-stack steps live in Task 7 Step 8 and go into the PR description.

## API for the frontend phase (append to the Phase 3/4 frontend notes)

- Base `/api/gl/lab/{catalogue,series,run,optimize,instruments}`; every body starts with `"provisional":true,"provisional_note":"…"`; show the note and `instrument.label` on every gl/crypto chart, result and optimization; never offer Submit (Phase 4 `publishable:false`).
- `GET /api/gl/lab/instruments` → `{provisional, provisional_note, label, instruments:[{instrument:{ticker,type,native,market,currency,name,timezone,calendar,bars_per_year,price,has_volume,provisional,label,source,quirks}, status:"ready", first, last, bars, tradable_from|null} | {instrument, status:"needs_data", reason}]}`. Use `instrument.market` (`gl`|`crypto`) as the saved market; send `ticker=<id>` to `/api/gl/…` for both.
- `start` disclosure appears when `tradable_from` moves the window (reason "opens before <date> are stale"); `from` earlier than the cut is `needs_data`.
- Volume indicators on gl are `needs_data` (no volume staged).

## File structure

```
Context/guru/backend/src/lab/markets.rs        NEW  GL table, gl(), tradable_from, annualise, bars_per_year, engine_market,
                                                    engine_route_allowed, refused_json, mark_provisional, not_staged, instruments_json   (Task 1)
Context/guru/backend/src/lab/mod.rs            + `pub mod markets;`                                                                  (Task 1)
Context/guru/backend/src/lab/api.rs            response()/serve, instruments route, gl instrument, needs_data, cut, annualised stats (Task 2)
Context/guru/backend/src/main.rs               + 1 line in handle()                                                                 (Task 4)
Context/guru/webapp/scripts/stage_gl.py        NEW  MANIFEST, read_export, stage, CLI                                               (Task 3)
Context/guru/webapp/scripts/setup.py           + 2 lines (import, stage call)                                                       (Task 3)
Context/guru/webapp/scripts/test_engine_gl.py  NEW  staging tests (Task 3) + compose/setup checks (Task 6)
Context/guru/webapp/gateway/src/policy.rs      + "lab/instruments", engine_market_allows, gl_tests                                  (Task 5)
Context/guru/webapp/gateway/src/main.rs        + App.gl, VAULT_ENGINE_GL; 2 edited lines (market check, engine pick)                (Task 5)
Context/guru/webapp/compose.yaml               + engine-gl service, + gateway env and depends_on lines                              (Task 6)
```

**Setup once per session (all tasks):** `export PHASE5_BASE=$(git rev-parse HEAD)` before Task 1 (the scope proof in Task 7 diffs against it). Scratch staging directory for Tasks 4 and 7: `export GLSTAGE="${TMPDIR:-/tmp}/phase5-gl"` (local disk, outside the repository and the NAS).

---

### Task 1: Provisional instrument table and pure rules (`lab/markets.rs`)

**Files:**
- Create: `Context/guru/backend/src/lab/markets.rs`
- Modify: `Context/guru/backend/src/lab/mod.rs` (add `pub mod markets;` after `pub mod json;`)

**Interfaces:**
- Consumes: `crate::Row` (`main.rs:22`), `super::json::{esc, parse, Json}`, `super::rules::LABEL`, `crate::State` (`ctxs`), `crate::load_csv` (test only), `crate::lab::resample::{resample, expand, Tf}`, `crate::lab::bars::Bars` (test only).
- Produces (used by Tasks 2 and 4): `pub const PROVISIONAL: &str`; `pub const BOOK_BARS_PER_YEAR: f64 = 252.`; `pub struct GlInstrument { ticker, name, kind, market, currency, timezone, calendar: &'static str, bars_per_year: f64, continuous: bool, total_return: bool, source, quirks: &'static str }` with `label() -> String`, `json() -> String`; `pub const GL: &[GlInstrument]` (10 rows); `pub fn gl(ticker: &str) -> Option<&'static GlInstrument>`; `pub fn tradable_from(rows: &[Row], continuous: bool) -> Option<String>` (`"YYYY-01-01"`); `pub fn bars_per_year(ticker: &str) -> f64`; `pub fn annualise(s: &mut BTreeMap<&'static str, f64>, bars_per_year: f64)`; `pub fn engine_market() -> Option<String>` (env `VAULT_LAB_MARKET`); `pub fn engine_route_allowed(market: Option<&str>, path: &str) -> bool`; `pub fn refused_json(path: &str) -> String`; `pub fn mark_provisional(body: &str) -> String`; `pub fn not_staged(ticker: &str) -> String`; `pub fn instruments_json(st: &crate::State) -> String` (tested in Task 2).

- [ ] **Step 1: Write the failing tests** — create `Context/guru/backend/src/lab/markets.rs` with only this test module, and add `pub mod markets;` to `lab/mod.rs` directly after `pub mod json;`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::lab::json::parse;
    fn row(date: &str, o: f64, h: f64, l: f64, c: f64) -> Row {
        Row { ticker: "T:X".into(), date: date.into(), open: o, high: h, low: l, close: c, volume: None }
    }

    #[test] fn every_gl_instrument_is_provisional_and_declares_its_calendar() {
        let mut seen = std::collections::BTreeSet::new();
        for m in GL {
            assert!(seen.insert(m.ticker) && m.ticker.contains(':'), "{}", m.ticker);
            assert!(matches!(m.kind, "index" | "cfd" | "crypto") && matches!(m.market, "gl" | "crypto"), "{}", m.ticker);
            assert_eq!(m.kind == "crypto", m.market == "crypto");
            if m.kind == "crypto" { assert!(m.continuous && m.bars_per_year == 365. && m.timezone == "UTC"); }
            else { assert!(!m.continuous && m.bars_per_year >= 252.); }
            let j = parse(&m.json()).unwrap();
            assert_eq!(j.get("ticker").unwrap().as_str(), Some(m.ticker));
            assert!(matches!(j.get("provisional"), Some(crate::lab::json::Json::Bool(true))));
            assert!(j.get("label").unwrap().as_str().unwrap().starts_with("Provisional · TradingView export "));
        }
        assert_eq!(gl("IG:DAX").unwrap().bars_per_year, 311.);
        assert!(gl("SPX").is_none() && gl("NIFTY 50").is_none() && gl("basket:us_sp500:AAPL").is_none());
    }
    #[test] fn tradable_from_skips_close_only_and_stale_open_years() {
        let mut rows = vec![row("2001-01-02", 10., 10., 10., 10.), row("2001-01-03", 11., 11., 11., 11.)]; // close-only year
        rows.extend([row("2002-01-02", 11., 12., 10., 11.5), row("2002-01-03", 11.5, 12., 11., 11.8)]); // opens = previous close
        rows.extend((0..9).map(|d| row(&format!("2003-01-{:02}", d + 2), 12. + d as f64, 13. + d as f64, 11. + d as f64, 12.5 + d as f64)));
        rows.push(row("2003-02-01", 20.5, 22., 20., 21.5)); // opens at the previous close: 1 stale bar in 10 is under 20%
        assert_eq!(tradable_from(&rows, false).as_deref(), Some("2003-01-01"));
        assert_eq!(tradable_from(&rows, true).as_deref(), Some("2002-01-01"), "around the clock an open at the previous close is normal");
        assert_eq!(tradable_from(&rows[4..], false), None);
        assert_eq!(tradable_from(&[], false), None);
    }
    #[test] fn annualisation_uses_the_instruments_bar_frequency() {
        let base: BTreeMap<&'static str, f64> = [("vol", 0.2), ("sharpe", 1.0), ("sortino", 1.5), ("gross_sharpe", 1.2), ("cagr", 0.1), ("trades_per_year", 4.)].into();
        let mut s = base.clone();
        annualise(&mut s, bars_per_year("NIFTY 50"));
        assert_eq!(s, base, "India/US results keep the book's sqrt(252)");
        annualise(&mut s, bars_per_year("CRYPTO:BTCUSD"));
        let f = (365f64 / 252.).sqrt();
        for k in ["vol", "sharpe", "sortino", "gross_sharpe"] { assert!((s[k] - base[k] * f).abs() < 1e-12, "{k}"); }
        assert_eq!((s["cagr"], s["trades_per_year"]), (0.1, 4.), "calendar-time statistics are unchanged");
    }
    #[test] fn engine_gl_serves_lab_routes_only_and_marks_every_body() {
        assert!(engine_route_allowed(None, "/api/guru/connors/run"));
        assert!(engine_route_allowed(Some("in"), "/api/tickers"));
        assert!(engine_route_allowed(Some("gl"), "/api/lab/run"));
        for p in ["/api/guru/connors/run", "/api/tickers", "/api/ohlc", "/api/notes", "/api/gurus", "/api/labx"] { assert!(!engine_route_allowed(Some("gl"), p), "{p}"); }
        for body in [refused_json("/api/guru/connors/run"), mark_provisional("{\"status\":\"invalid\",\"error\":\"x\",\"label\":\"l\"}")] {
            assert!(body.starts_with(&format!("{{\"provisional\":true,\"provisional_note\":\"{PROVISIONAL}\",")), "{body}");
            parse(&body).unwrap();
        }
    }
    #[test] fn a_cfd_sunday_bar_closes_its_monday_to_sunday_week() {
        use crate::lab::resample::{expand, resample, Tf};
        let rows = [row("2026-09-18", 10., 11., 9., 10.), row("2026-09-20", 10., 10.5, 9.5, 10.2), row("2026-09-21", 10.2, 12., 10., 11.)]; // Fri, Sun, Mon
        let d = crate::lab::bars::Bars::from_rows(&rows);
        let w = resample(&d, Tf::W);
        assert_eq!(w.date, ["2026-09-20", "2026-09-21"]);
        assert_eq!(w.close, [10.2, 11.]);
        assert_eq!(w.end, [1, 2]);
        let x = expand(&w.close, &w, 3);
        assert!(x[0].is_nan() && x[1] == 10.2, "the week ending on its Sunday bar is usable from that Sunday close, never on Friday");
    }
    /// Real-data check of the declared cuts: VAULT_GL_CSV = a gl_data.csv staged by Context/guru/webapp/scripts/stage_gl.py.
    #[test] fn staged_gl_data_matches_the_declared_cuts() {
        let Ok(path) = std::env::var("VAULT_GL_CSV") else { eprintln!("VAULT_GL_CSV not set; staged-data check skipped"); return };
        let data = crate::load_csv(&path).unwrap();
        let expected = [("BSE_DLY:SENSEX", Some("2010-01-01")), ("CRYPTO:BTCUSD", Some("2011-01-01")), ("CRYPTO:ETHUSD", None), ("HSI:HSI", Some("1992-01-01")),
            ("IG:DAX", None), ("IG:FTSE", Some("2011-01-01")), ("NASDAQ_DLY:IXIC", Some("2000-01-01")), ("SPREADEX:NIKKEI", None), ("TVC:CAC40", None), ("TVC:RUT", Some("2007-01-01"))];
        for (ticker, cut) in expected {
            let Some(rows) = data.get(ticker) else { eprintln!("{ticker} not staged"); continue };
            assert!(rows.iter().all(|r| r.volume.is_none()), "{ticker}: volume must not be staged");
            assert_eq!(tradable_from(rows, gl(ticker).unwrap().continuous).as_deref(), cut, "{ticker}");
        }
        assert!(data.keys().all(|t| gl(t).is_some()), "only declared instruments are staged: {:?}", data.keys().collect::<Vec<_>>());
    }
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cd Context/guru/backend && CARGO_BUILD_JOBS=2 cargo test lab::markets`
Expected: compile errors (`cannot find value GL`, `cannot find function tradable_from`, …).

- [ ] **Step 3: Implement** — put this above the test module in `markets.rs` (the file then matches the planning copy exactly):

```rust
//! Provisional global and crypto instruments served by `engine-gl` (TradingView exports, internal research only).
//! Their metadata lives here because the admission files (instrument_policy.rs, gate.rs) are book code and stay untouched.
use super::json::esc;
use super::rules::LABEL;
use crate::Row;
use std::collections::BTreeMap;

pub const PROVISIONAL: &str = "Provisional · TradingView export · internal research only, not publishable to YT Strategies";
/// The book statistics annualise volatility, Sharpe and Sortino with sqrt(252) (main.rs stats()).
pub const BOOK_BARS_PER_YEAR: f64 = 252.;

pub struct GlInstrument {
    /// TradingView "EXCHANGE:SYMBOL"; the colon keeps these ids apart from every India/US ticker.
    pub ticker: &'static str,
    pub name: &'static str,
    /// index | cfd | crypto
    pub kind: &'static str,
    /// gl | crypto (the saved-strategy market; both are served by /api/gl)
    pub market: &'static str,
    pub currency: &'static str,
    pub timezone: &'static str,
    pub calendar: &'static str,
    pub bars_per_year: f64,
    /// Trades around the clock: an open equal to the previous close is normal, not stale.
    pub continuous: bool,
    pub total_return: bool,
    pub source: &'static str,
    pub quirks: &'static str,
}

const NO_VOLUME: &str = "No volume is staged (TradingView exports carry none, or only CFD tick counts).";
pub const GL: &[GlInstrument] = &[
    GlInstrument { ticker: "BSE_DLY:SENSEX", name: "S&P BSE SENSEX", kind: "index", market: "gl", currency: "INR", timezone: "Asia/Kolkata",
        calendar: "BSE sessions Mon-Fri, plus occasional special weekend sessions", bars_per_year: 252., continuous: false, total_return: false,
        source: "TradingView export BSE_DLY:SENSEX, daily", quirks: "Close-only bars before 1991; many opens equal the previous close in 2009. No volume is staged." },
    GlInstrument { ticker: "CRYPTO:BTCUSD", name: "Bitcoin / US dollar (TradingView CRYPTO index)", kind: "crypto", market: "crypto", currency: "USD", timezone: "UTC",
        calendar: "7 days a week, UTC days", bars_per_year: 365., continuous: true, total_return: false,
        source: "TradingView export CRYPTO:BTCUSD, daily", quirks: "Close-only bars in 2009-2010. No volume is staged." },
    GlInstrument { ticker: "CRYPTO:ETHUSD", name: "Ether / US dollar (TradingView CRYPTO index)", kind: "crypto", market: "crypto", currency: "USD", timezone: "UTC",
        calendar: "7 days a week, UTC days", bars_per_year: 365., continuous: true, total_return: false,
        source: "TradingView export CRYPTO:ETHUSD, daily", quirks: "Erratic first weeks after the August 2015 launch (a 0.15 low). No volume is staged." },
    GlInstrument { ticker: "HSI:HSI", name: "Hang Seng Index", kind: "index", market: "gl", currency: "HKD", timezone: "Asia/Hong_Kong",
        calendar: "HKEX sessions Mon-Fri", bars_per_year: 252., continuous: false, total_return: false,
        source: "TradingView export HSI:HSI, daily", quirks: "Close-only bars 1987-1989; stale opens 1990-1991. No volume is staged." },
    GlInstrument { ticker: "IG:DAX", name: "DAX 40 (IG CFD)", kind: "cfd", market: "gl", currency: "EUR", timezone: "Europe/Berlin",
        calendar: "IG CFD: Mon-Fri plus a short Sunday-evening bar since August 2010", bars_per_year: 311., continuous: false, total_return: true,
        source: "TradingView export IG:DAX, daily", quirks: "About 51 Sunday bars a year count as bars; weeks here run Monday-Sunday, so a weekly close is its Sunday bar. DAX is a total-return index. CFD tick volume is not staged." },
    GlInstrument { ticker: "IG:FTSE", name: "FTSE 100 (IG CFD)", kind: "cfd", market: "gl", currency: "GBP", timezone: "Europe/London",
        calendar: "IG CFD: Mon-Fri plus a short Sunday-evening bar since August 2010", bars_per_year: 311., continuous: false, total_return: false,
        source: "TradingView export IG:FTSE, daily", quirks: "Opens equal the previous close 1999-2010; about 51 Sunday bars a year; weeks here run Monday-Sunday. CFD tick volume is not staged." },
    GlInstrument { ticker: "NASDAQ_DLY:IXIC", name: "NASDAQ Composite", kind: "index", market: "gl", currency: "USD", timezone: "America/New_York",
        calendar: "Nasdaq sessions Mon-Fri", bars_per_year: 252., continuous: false, total_return: false,
        source: "TradingView export NASDAQ_DLY:IXIC, daily", quirks: "Close-only bars 1971-1984; stale opens 1996-1999. No volume is staged." },
    GlInstrument { ticker: "SPREADEX:NIKKEI", name: "Nikkei 225 (Spreadex spread bet)", kind: "cfd", market: "gl", currency: "JPY", timezone: "Asia/Tokyo",
        calendar: "Spreadex quotes Mon-Fri", bars_per_year: 252., continuous: false, total_return: false,
        source: "TradingView export SPREADEX:NIKKEI, daily", quirks: "Starts 2016-05-23; some years have missing sessions. Spread-bet tick volume is not staged." },
    GlInstrument { ticker: "TVC:CAC40", name: "CAC 40", kind: "index", market: "gl", currency: "EUR", timezone: "Europe/Paris",
        calendar: "Euronext Paris sessions Mon-Fri", bars_per_year: 252., continuous: false, total_return: false,
        source: "TradingView export TVC:CAC40, daily", quirks: NO_VOLUME },
    GlInstrument { ticker: "TVC:RUT", name: "Russell 2000", kind: "index", market: "gl", currency: "USD", timezone: "America/New_York",
        calendar: "NYSE sessions Mon-Fri", bars_per_year: 252., continuous: false, total_return: false,
        source: "TradingView export TVC:RUT, daily", quirks: "Close-only bars before November 1992; stale opens 1993-2006. No volume is staged." },
];

pub fn gl(ticker: &str) -> Option<&'static GlInstrument> { GL.iter().find(|m| m.ticker == ticker) }

impl GlInstrument {
    pub fn label(&self) -> String { format!("Provisional · {}", self.source) }
    /// Same leading keys as the India/US instrument JSON (lab/api.rs instrument()), then the provisional metadata.
    pub fn json(&self) -> String {
        format!(
            "{{\"ticker\":\"{}\",\"type\":\"{}\",\"native\":false,\"market\":\"{}\",\"currency\":\"{}\",\"name\":\"{}\",\"timezone\":\"{}\",\"calendar\":\"{}\",\"bars_per_year\":{},\"price\":\"{}\",\"has_volume\":false,\"provisional\":true,\"label\":\"{}\",\"source\":\"{}\",\"quirks\":\"{}\"}}",
            esc(self.ticker), self.kind, self.market, self.currency, esc(self.name), self.timezone, esc(self.calendar), self.bars_per_year,
            if self.total_return { "total_return" } else { "price" }, esc(&self.label()), esc(self.source), esc(self.quirks))
    }
}

/// First date from which next-open fills use real opens: 1 January after the last calendar year in which more than 20% of
/// bars are close-only (O=H=L=C) or, on session markets, open exactly at the previous close. None: every year is usable.
pub fn tradable_from(rows: &[Row], continuous: bool) -> Option<String> {
    let mut years: BTreeMap<&str, (usize, usize)> = BTreeMap::new();
    for (i, r) in rows.iter().enumerate() {
        let close_only = r.open == r.high && r.high == r.low && r.low == r.close;
        let stale = !continuous && i > 0 && r.open == rows[i - 1].close;
        let y = years.entry(&r.date[..4]).or_default();
        y.0 += 1;
        if close_only || stale { y.1 += 1; }
    }
    years.iter().rev().find(|(_, (n, bad))| bad * 5 > *n).map(|(y, _)| format!("{}-01-01", y.parse::<i32>().unwrap_or(0) + 1))
}

pub fn bars_per_year(ticker: &str) -> f64 { gl(ticker).map_or(BOOK_BARS_PER_YEAR, |m| m.bars_per_year) }
/// Rescales the sqrt(252) ratios of the book statistics to the instrument's own bar frequency (crypto: 365 bars a year).
/// CAGR, years and trade rates already use calendar days, so nothing else changes.
pub fn annualise(s: &mut BTreeMap<&'static str, f64>, bars_per_year: f64) {
    if bars_per_year == BOOK_BARS_PER_YEAR { return; }
    let f = (bars_per_year / BOOK_BARS_PER_YEAR).sqrt();
    for k in ["vol", "sharpe", "sortino", "gross_sharpe"] {
        if let Some(v) = s.get_mut(k) { *v *= f; }
    }
}

/// `VAULT_LAB_MARKET=gl` marks the engine-gl container (compose.yaml); the India and US engines leave it unset.
pub fn engine_market() -> Option<String> { std::env::var("VAULT_LAB_MARKET").ok() }
/// engine-gl answers indicator-lab routes only: book strategies have no admission rules for provisional data.
pub fn engine_route_allowed(market: Option<&str>, path: &str) -> bool { market != Some("gl") || path.starts_with("/api/lab/") }
pub fn refused_json(path: &str) -> String {
    mark_provisional(&format!("{{\"status\":\"not_found\",\"error\":\"engine-gl serves indicator-lab routes only; {} is not available on provisional TradingView data\",\"label\":\"{LABEL}\"}}", esc(path)))
}
/// Every engine-gl lab body (results and errors) starts with the provisional flag. Lab bodies are JSON objects.
pub fn mark_provisional(body: &str) -> String {
    format!("{{\"provisional\":true,\"provisional_note\":\"{PROVISIONAL}\",{}", body.strip_prefix('{').unwrap_or(body))
}
pub fn not_staged(ticker: &str) -> String {
    format!("{ticker}: the TradingView export is not staged on this engine (served by /api/gl once setup.py stages it); no result is published")
}
/// /api/lab/instruments: every provisional instrument and whether this engine holds its data.
pub fn instruments_json(st: &crate::State) -> String {
    let opt = |s: Option<String>| s.map_or("null".into(), |s| format!("\"{s}\""));
    let rows: Vec<String> = GL.iter().map(|m| match st.ctxs.get(m.ticker).filter(|c| !c.r.is_empty()) {
        Some(c) => format!("{{\"instrument\":{},\"status\":\"ready\",\"first\":\"{}\",\"last\":\"{}\",\"bars\":{},\"tradable_from\":{}}}",
            m.json(), c.r[0].date, c.r[c.r.len() - 1].date, c.r.len(), opt(tradable_from(c.r, m.continuous))),
        None => format!("{{\"instrument\":{},\"status\":\"needs_data\",\"reason\":\"{}\"}}", m.json(), esc(&not_staged(m.ticker))),
    }).collect();
    format!("{{\"label\":\"{LABEL}\",\"instruments\":[{}]}}", rows.join(","))
}
```

- [ ] **Step 4: Run tests**

Run: `cd Context/guru/backend && CARGO_BUILD_JOBS=2 cargo test lab::markets`
Expected: `6 passed` (the real-data test prints "VAULT_GL_CSV not set; staged-data check skipped" and passes; it is exercised in Task 4).

- [ ] **Step 5: Guard + commit**

```bash
python3 scripts/check_book_untouched.py --base origin/Manan
git add Context/guru/backend/src/lab/markets.rs Context/guru/backend/src/lab/mod.rs
git commit -m "Add provisional global and crypto instrument rules for the lab"
```
Expected guard output: `"problems": []`.

---

### Task 2: Lab API on gl/crypto — instrument metadata, needs_data, tradable-from cut, annualised stats, provisional bodies, instruments route

**Files:**
- Modify: `Context/guru/backend/src/lab/api.rs` (lab code; not a protected or additive-only file)
- Test: `Context/guru/backend/src/lab/api.rs` (`mod tests`, appended)

**Interfaces:**
- Consumes: everything Task 1 produces.
- Produces: `pub fn response(market: Option<&str>, path: &str, st: &State, q: &Query) -> (&'static str, String)` (HTTP status line text and body; `serve` = `response(markets::engine_market().as_deref(), …)` + `crate::respond`); route `/api/lab/instruments`; private `lab_stats(ctx, run, costs)` and `gross_stats(ctx, run)` (book stats + `annualise`). Run/optimize/series JSON on a gl id: `"instrument":<GlInstrument::json()>`.

- [ ] **Step 1: Write the failing tests** — append inside `mod tests` of `lab/api.rs` (before its final `}`):

```rust
    fn num_at(j: &str, path: &[&str]) -> f64 {
        let v = crate::lab::json::parse(j).unwrap();
        path.iter().fold(&v, |v, k| v.get(k).unwrap()).as_f64().unwrap_or_else(|| panic!("{path:?} is null"))
    }
    #[test] fn gl_instruments_are_labelled_provisional_and_annualised_by_their_own_calendar() {
        let (spx, btc) = (run_json(&state(800, "SPX", false), &q(&[])).unwrap(), run_json(&state(800, "CRYPTO:BTCUSD", false), &q(&[("ticker", "CRYPTO:BTCUSD")])).unwrap());
        assert!(btc.contains(&format!("\"instrument\":{}", markets::gl("CRYPTO:BTCUSD").unwrap().json())), "{}", &btc[..400]);
        assert!(btc.contains("\"provisional\":true") && btc.contains("\"label\":\"Provisional · TradingView export CRYPTO:BTCUSD, daily\"") && btc.contains("\"bars_per_year\":365"));
        assert!(!spx.contains("provisional"), "India/US lab output is unchanged");
        let f = (365f64 / 252.).sqrt();
        for path in [&["stats", "sharpe"][..], &["stats", "vol"], &["stats", "gross_sharpe"], &["gross_stats", "sharpe"]] {
            let (a, b) = (num_at(&spx, path), num_at(&btc, path));
            assert!((b - a * f).abs() <= 2e-6 * (1. + b.abs()), "{path:?}: {a} vs {b}");
        }
        for path in [&["stats", "cagr"][..], &["stats", "trades"], &["stats", "max_dd"], &["stats", "trades_per_year"]] { assert_eq!(num_at(&spx, path), num_at(&btc, path), "{path:?}"); }
        let o = optimize_json(&state(800, "CRYPTO:BTCUSD", false), &q(&[("ticker", "CRYPTO:BTCUSD")])).unwrap();
        assert!(o.contains("\"provisional\":true") && o.contains("\"kind\":\"lab_optimization\""));
    }
    #[test] fn gl_data_gaps_are_needs_data_and_unknown_tickers_stay_invalid() {
        let st = state(400, "SPX", false);
        for t in ["HSI:HSI", "CRYPTO:ETHUSD"] {
            let e = run_json(&st, &q(&[("ticker", t)])).unwrap_err();
            assert_eq!(e.status(), "needs_data");
            assert!(e.message().contains(t) && e.message().contains("not staged"), "{}", e.message());
        }
        assert_eq!(run_json(&st, &q(&[("ticker", "HSI:NOPE")])).unwrap_err().status(), "invalid");
        let e = run_json(&state(400, "TVC:CAC40", false), &q(&[("ticker", "TVC:CAC40"), ("spec", r#"{"v":1,"entry":{"all":[{"lhs":{"s":"obv"},"op":">","rhs":{"value":0}}]},"exit":{"any":[{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":3}}]}}"#)])).unwrap_err();
        assert_eq!(e.status(), "needs_data", "{}", e.message());
    }
    #[test] fn gl_close_only_years_move_the_start() {
        // HSI:HSI rows start 2015 (state()); make 2015 close-only so next-open fills begin 2016-01-01.
        let rows: &'static [Row] = Box::leak((0..700).map(|i| {
            let c = 100. + (i as f64 * 0.3).sin() * 8. + i as f64 * 0.02;
            let date = format!("{}-{:02}-{:02}", 2015 + i / 336, 1 + (i / 28) % 12, 1 + i % 28);
            let (o, h, l) = if i < 336 { (c, c, c) } else { (c - 0.2, c + 1., c - 1.) };
            Row { ticker: "HSI:HSI".into(), date, open: o, high: h, low: l, close: c, volume: None }
        }).collect::<Vec<_>>().into_boxed_slice());
        let st = State { ctxs: BTreeMap::from([("HSI:HSI".into(), Ctx::new(rows, rows, &[]))]), baskets: Default::default(), vix: vec![], market_extra: Default::default() };
        let hsi = |extra: &[(&str, &str)]| { let mut x = q(&[("ticker", "HSI:HSI")]); for (k, v) in extra { x.insert(k.to_string(), v.to_string()); } x };
        let j = run_json(&st, &hsi(&[])).unwrap();
        assert!(j.contains("\"start\":{\"date\":\"2016-01-01\",\"reason\":\"opens before 2016-01-01 are stale\"}"), "{}", &j[..600]);
        let e = run_json(&st, &hsi(&[("from", "2015-06-01")])).unwrap_err();
        assert!(e.status() == "needs_data" && e.message().contains("choose from >= 2016-01-01"), "{}", e.message());
        assert!(markets::instruments_json(&st).contains("\"tradable_from\":\"2016-01-01\""));
    }
    #[test] fn engine_gl_responses_all_carry_the_provisional_flag() {
        let st = state(400, "CRYPTO:BTCUSD", false);
        let cases = [("/api/lab/catalogue", q(&[]), "200 OK"), ("/api/lab/run", q(&[("ticker", "CRYPTO:BTCUSD")]), "200 OK"),
            ("/api/lab/run", q(&[("ticker", "CRYPTO:BTCUSD"), ("spec", "{")]), "400 Bad Request"), ("/api/lab/run", q(&[("ticker", "HSI:HSI")]), "400 Bad Request"),
            ("/api/lab/instruments", q(&[]), "200 OK"), ("/api/lab/nope", q(&[]), "404 Not Found")];
        for (path, query, status) in cases {
            let (s, body) = response(Some("gl"), path, &st, &query);
            assert_eq!(s, status, "{path}: {body}");
            assert!(body.starts_with("{\"provisional\":true,\"provisional_note\":\"Provisional · TradingView export"), "{path}: {}", &body[..80.min(body.len())]);
            crate::lab::json::parse(&body).unwrap();
            let (_, plain) = response(None, path, &st, &query);
            assert!(!plain.starts_with("{\"provisional\""), "India/US engines are not marked");
        }
    }
    #[test] fn instruments_route_lists_ready_and_missing_gl_data() {
        let j = route("/api/lab/instruments", &state(400, "IG:DAX", false), &q(&[])).unwrap().unwrap();
        let v = crate::lab::json::parse(&j).unwrap();
        let Some(crate::lab::json::Json::Arr(rows)) = v.get("instruments") else { panic!("{j}") };
        assert_eq!(rows.len(), markets::GL.len());
        for r in rows {
            let t = r.get("instrument").unwrap().get("ticker").unwrap().as_str().unwrap();
            assert_eq!(r.get("status").unwrap().as_str(), Some(if t == "IG:DAX" { "ready" } else { "needs_data" }), "{t}");
        }
        assert!(j.contains("\"first\":\"2015-01-01\"") && j.contains("\"tradable_from\":null"));
    }
```

- [ ] **Step 2: Run to verify failure**

Run: `cd Context/guru/backend && CARGO_BUILD_JOBS=2 cargo test lab::api`
Expected: compile errors (`cannot find function response`, unresolved `markets` in the test module). Without the edits below the behaviour would also be wrong: a crypto instrument reports `"type":"unknown"` (→ `needs_data`), `HSI:HSI` unloaded is `invalid`, there is no `start` cut and no provisional prefix.

- [ ] **Step 3: Implement** — eight exact edits in `lab/api.rs`:

(a) imports — after `use super::json::esc;` add:
```rust
use super::markets;
```

(b) replace the whole `serve` function with `serve` + `response`:
```rust
pub fn serve(stream: &mut TcpStream, path: &str, st: &State, q: &Query) {
    let (status, body) = response(markets::engine_market().as_deref(), path, st, q);
    crate::respond(stream, status, "application/json; charset=utf-8", body.as_bytes());
}
/// HTTP status and body of one lab request; on engine-gl (`market` "gl") every body carries the provisional flag.
pub fn response(market: Option<&str>, path: &str, st: &State, q: &Query) -> (&'static str, String) {
    let (status, body) = match route(path, st, q) {
        None => ("404 Not Found", not_found_json(path)),
        Some(Ok(body)) => ("200 OK", body),
        Some(Err(e)) => ("400 Bad Request", e.json()),
    };
    (status, if market == Some("gl") { markets::mark_provisional(&body) } else { body })
}
```

(c) in `route`, after the line `        "/api/lab/optimize" => optimize_json,` add:
```rust
        "/api/lab/instruments" => |st, _| Ok(markets::instruments_json(st)),
```

(d) first line of `fn instrument(…)` body:
```rust
    if let Some(m) = markets::gl(ticker) { return (m.kind, m.json()); }
```

(e) first line of `fn select_error(…)` body:
```rust
    if let Some(m) = e.strip_prefix("unknown ticker ").and_then(markets::gl) { return LabError::NeedsData(markets::not_staged(m.ticker)); }
```

(f) in `window`, replace the line `    if let Some(cut) = crate::gate::real_open_from(&ctx.r[0].ticker) {` with:
```rust
    let gl_cut = markets::gl(&ctx.r[0].ticker).and_then(|m| markets::tradable_from(ctx.r, m.continuous));
    if let Some(cut) = crate::gate::real_open_from(&ctx.r[0].ticker).or(gl_cut.as_deref()) {
```

(g) directly above `fn backtest(` add:
```rust
/// Book statistics (read-only) rescaled to the instrument's own bar frequency (lab/markets.rs annualise).
fn lab_stats(ctx: &Ctx, run: &crate::Run, costs: accounting::Costs) -> BTreeMap<&'static str, f64> {
    let mut s = crate::financial_stats(ctx, run, costs);
    markets::annualise(&mut s, markets::bars_per_year(&ctx.r[0].ticker));
    s
}
fn gross_stats(ctx: &Ctx, run: &crate::Run) -> BTreeMap<&'static str, f64> {
    let mut s = crate::stats(ctx, run, 0.);
    markets::annualise(&mut s, markets::bars_per_year(&ctx.r[0].ticker));
    s
}
```

(h) use them — three replacements:
- in `run_json`: `crate::stats_json(&crate::financial_stats(ctx, &run, costs)), crate::stats_json(&crate::stats(ctx, &gross, 0.)), costs.json()` → `crate::stats_json(&lab_stats(ctx, &run, costs)), crate::stats_json(&gross_stats(ctx, &gross)), costs.json()`
- in `optimize_json` `metrics`: `let stats = crate::financial_stats(ctx, &backtest(ctx, &c, &fr.d, from, to, cost)?, costs);` → `let stats = lab_stats(ctx, &backtest(ctx, &c, &fr.d, from, to, cost)?, costs);`
- and `let part = crate::financial_stats(ctx, &backtest(ctx, &c, &fr.d, a, b, cost)?, costs);` → `let part = lab_stats(ctx, &backtest(ctx, &c, &fr.d, a, b, cost)?, costs);`

After the edits `grep -n "crate::financial_stats\|crate::stats(" src/lab/api.rs` lists only the two lines inside `lab_stats`/`gross_stats`.

- [ ] **Step 4: Run tests**

Run: `cd Context/guru/backend && CARGO_BUILD_JOBS=2 cargo test lab::`
Expected: all lab tests pass (130 at planning time: 119 existing + 6 markets + 5 new api). The existing api tests pass **unchanged** — that is the proof India/US lab output did not move.

- [ ] **Step 5: Guard + commit**

```bash
python3 scripts/check_book_untouched.py --base origin/Manan
git add Context/guru/backend/src/lab/api.rs
git commit -m "Serve lab results on provisional global and crypto instruments"
```

---

### Task 3: Stage the TradingView exports (`stage_gl.py`, `setup.py`)

**Files:**
- Create: `Context/guru/webapp/scripts/stage_gl.py`
- Create: `Context/guru/webapp/scripts/test_engine_gl.py` (class `StageGl`; Task 6 appends `EngineGlCompose`)
- Modify: `Context/guru/webapp/scripts/setup.py` (2 added lines, no edited line)

**Interfaces:**
- Consumes: `GL` tickers in `lab/markets.rs` (the test reads the Rust source to compare).
- Produces: `stage_gl.MANIFEST: tuple[(ticker, relative_path)]`; `stage_gl.read_export(path) -> list[(iso_date, open_text, high_text, low_text, close_text)]` (raises `ValueError`); `stage_gl.stage(source_root, out_csv) -> list[str]` notes; CLI `python3 stage_gl.py SOURCE_ROOT OUT_CSV` prints the notes as JSON. Output file `<runtime>/data/gl/gl_data.csv` (Task 6 mounts `data/gl`).

- [ ] **Step 1: Write the failing tests** — create `Context/guru/webapp/scripts/test_engine_gl.py`:

```python
import csv, re, tempfile, unittest
from pathlib import Path

import stage_gl

REPO = Path(__file__).resolve().parents[4]


def write(root, rel, text):
    path = Path(root) / rel
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text)


class StageGl(unittest.TestCase):
    def test_exports_with_each_header_style_become_one_engine_csv_without_volume(self):
        with tempfile.TemporaryDirectory() as src, tempfile.TemporaryDirectory() as out:
            files = dict(stage_gl.MANIFEST)
            write(src, files["HSI:HSI"], "Ticker,time,open,high,low,close,Volume\nHANG SENG,02-01-1987,2540.1001,2541,2539,2540.5,\nHANG SENG,31-12-1986,2568.3,2568.3,2568.3,2568.3,\n")
            write(src, files["NASDAQ_DLY:IXIC"], "time,open,high,low,close\n1971-02-05,100,100,100,100\n")
            write(src, files["TVC:RUT"], "Date,time,open,high,low,close\nUS-SMALL CAP 2000_RUSSELL 2000,29-06-1987,164.96999,164.96999,164.96999,164.96999\n")
            write(src, files["BSE_DLY:SENSEX"], "Ticker,Date,open,high,low,close\nSENSEX,03-04-1979,124.15,124.15,124.15,124.15\n")
            write(src, files["IG:DAX"], "Ticker,time,open,high,low,close,Volume\nDAX 40,02-01-1995,2091.47,2092.33,2078.46,2079.19,9063\n")
            notes = stage_gl.stage(src, Path(out) / "gl" / "gl_data.csv")
            with open(Path(out) / "gl" / "gl_data.csv", newline="") as f:
                rows = list(csv.reader(f))
        self.assertEqual(rows[0], ["Ticker", "Date", "Open", "High", "Low", "Close", "Volume"])
        self.assertEqual([r for r in rows if r[0] == "HSI:HSI"], [["HSI:HSI", "1986-12-31", "2568.3", "2568.3", "2568.3", "2568.3", ""],
                                     ["HSI:HSI", "1987-01-02", "2540.1001", "2541", "2539", "2540.5", ""]])
        self.assertIn(["NASDAQ_DLY:IXIC", "1971-02-05", "100", "100", "100", "100", ""], rows)
        self.assertIn(["TVC:RUT", "1987-06-29", "164.96999", "164.96999", "164.96999", "164.96999", ""], rows)
        self.assertIn(["BSE_DLY:SENSEX", "1979-04-03", "124.15", "124.15", "124.15", "124.15", ""], rows)
        self.assertIn(["IG:DAX", "1995-01-02", "2091.47", "2092.33", "2078.46", "2079.19", ""], rows, "CFD tick volume is not staged")
        self.assertIn("HSI:HSI: 2 bars 1986-12-31 to 1987-01-02", notes)
        self.assertTrue(any(n.startswith("CRYPTO:BTCUSD: missing ") for n in notes))

    def test_a_bad_row_or_duplicate_date_leaves_the_whole_instrument_out(self):
        with tempfile.TemporaryDirectory() as src, tempfile.TemporaryDirectory() as out:
            files = dict(stage_gl.MANIFEST)
            write(src, files["TVC:CAC40"], "Ticker,time,open,high,low,close\nCAC 40,31-12-1987,1007.45,1007.45,1000,1000\nCAC 40,04-01-1988,0,984.6,984.6,984.6\n")
            write(src, files["CRYPTO:ETHUSD"], "Ticker,time,open,high,low,close\nETHEREUM,07-08-2015,3,3,3,3\nETHEREUM,07-08-2015,3,3,3,3\n")
            write(src, files["IG:FTSE"], "Ticker,time,open,high,low,close\nFTSE 100,2015-13-40,1,1,1,1\n")
            write(src, files["SPREADEX:NIKKEI"], "Ticker,when,open,high,low,close\nNIKKEI 225,23-05-2016,1,1,1,1\n")
            notes = "\n".join(stage_gl.stage(src, Path(out) / "gl_data.csv"))
            text = (Path(out) / "gl_data.csv").read_text()
        self.assertEqual(text.strip(), "Ticker,Date,Open,High,Low,Close,Volume")
        self.assertIn("TVC:CAC40: not staged (line 3 ", notes)
        self.assertIn("CRYPTO:ETHUSD: not staged (duplicate dates, first 2015-08-07)", notes)
        self.assertIn("IG:FTSE: not staged (line 2 ", notes)
        self.assertIn("SPREADEX:NIKKEI: not staged (header ", notes)

    def test_no_source_data_still_writes_a_loadable_empty_file(self):
        with tempfile.TemporaryDirectory() as src, tempfile.TemporaryDirectory() as out:
            notes = stage_gl.stage(src, Path(out) / "gl_data.csv")
            self.assertEqual((Path(out) / "gl_data.csv").read_text().strip(), "Ticker,Date,Open,High,Low,Close,Volume")
        self.assertEqual(len(notes), len(stage_gl.MANIFEST))
        self.assertTrue(all("needs_data" in n for n in notes))

    def test_manifest_matches_the_engine_instrument_table(self):
        rust = (REPO / "Context/guru/backend/src/lab/markets.rs").read_text()
        declared = re.findall(r'GlInstrument \{ ticker: "([^"]+)"', rust)
        self.assertEqual(sorted(t for t, _ in stage_gl.MANIFEST), sorted(declared))
        sources = " ".join(rel for _, rel in stage_gl.MANIFEST)
        for left_out in ("SPCFD_SPX", "DJCFD_DJI", "NASDAQ_DLY_NDX", "RUS_MOEX", "New folder/NASDAQ_DLY_IXIC"):
            self.assertNotIn(left_out, sources, "admitted US indices are not duplicated; unresolved identities are not staged")


if __name__ == "__main__":
    unittest.main()
```

- [ ] **Step 2: Run to verify failure**

Run: `python3 -m unittest discover -s Context/guru/webapp/scripts -p 'test_*.py'`
Expected: `ModuleNotFoundError: No module named 'stage_gl'`.

- [ ] **Step 3: Implement** — create `Context/guru/webapp/scripts/stage_gl.py`:

```python
#!/usr/bin/env python3
"""Stage the provisional global/crypto TradingView exports into one engine CSV for engine-gl.

Reads only the listed source files (never writes beside them). Writes Ticker,Date,Open,High,Low,Close,Volume with
the TradingView id as Ticker and an empty Volume (exports carry no traded volume, only CFD tick counts at most).
An instrument whose file is missing or has any unreadable, non-positive or duplicate-date row is left out whole,
so the engine answers needs_data for it; prices are never repaired. Usage: stage_gl.py SOURCE_ROOT OUT_CSV
"""
import csv, datetime, json, os, sys
from pathlib import Path

# (TradingView id, source path under the research root). Must match GL in Context/guru/backend/src/lab/markets.rs.
# Left out on purpose: SPCFD_SPX, DJCFD_DJI, NASDAQ_DLY_NDX (admitted SPX/DJI/NDX exist on /api/us; a provisional copy
# must not stand in for them), RUS_MOEX (stock or index unresolved), New folder/NASDAQ_DLY_IXIC (duplicate export).
MANIFEST = (
    ("BSE_DLY:SENSEX", "Global Bhavcopy/New folder/BSE_DLY_SENSEX, 1D_31c7b.csv"),
    ("CRYPTO:BTCUSD", "Global Bhavcopy/New folder/CRYPTO_BTCUSD, 1D_86731.csv"),
    ("CRYPTO:ETHUSD", "Global Bhavcopy/New folder/CRYPTO_ETHUSD, 1D_8ff42.csv"),
    ("HSI:HSI", "Global Bhavcopy/HSI_HSI, 1D_1033e.csv"),
    ("IG:DAX", "Global Bhavcopy/IG_DAX, 1D_6e586.csv"),
    ("IG:FTSE", "Global Bhavcopy/IG_FTSE, 1D_aecd1.csv"),
    ("NASDAQ_DLY:IXIC", "Global Bhavcopy/NASDAQ_DLY_IXIC, 1D_71afe.csv"),
    ("SPREADEX:NIKKEI", "Global Bhavcopy/SPREADEX_NIKKEI, 1D_b84f8.csv"),
    ("TVC:CAC40", "Global Bhavcopy/TVC_CAC40, 1D_6ed46.csv"),
    ("TVC:RUT", "Global Bhavcopy/New folder/TVC_RUT_SMALLCAP 2000 INDEX, 1D_0ea63.csv"),
)
HEADER = ["Ticker", "Date", "Open", "High", "Low", "Close", "Volume"]


def iso(text):
    text = text.strip()
    for fmt in ("%d-%m-%Y", "%Y-%m-%d"):
        try:
            return datetime.datetime.strptime(text, fmt).date().isoformat()
        except ValueError:
            pass
    return None


def read_export(path):
    """[(date, open, high, low, close)] sorted by date, or raise ValueError naming the first bad row."""
    with open(path, newline="") as f:
        rows = list(csv.reader(f))
    names = [h.strip().lower() for h in rows[0]] if rows else []
    # TradingView's date column is "time"; some exports name the ticker column "Date", so "time" wins.
    date_col = names.index("time") if "time" in names else names.index("date") if "date" in names else None
    if date_col is None or not all(k in names for k in ("open", "high", "low", "close")):
        raise ValueError(f"header {rows[0] if rows else []} lacks a date and open/high/low/close")
    cols = [names.index(k) for k in ("open", "high", "low", "close")]
    out = []
    for n, row in enumerate(rows[1:], start=2):
        if not any(cell.strip() for cell in row):
            continue
        try:
            date, prices = iso(row[date_col]), [float(row[c]) for c in cols]
        except (IndexError, ValueError):
            date, prices = None, []
        if date is None or len(prices) != 4 or not all(0 < p < float("inf") for p in prices):
            raise ValueError(f"line {n} is not a dated positive OHLC row: {row}")
        out.append((date, *(row[c].strip() for c in cols)))  # source text kept exactly; floats only validate
    out.sort()
    dupes = [a[0] for a, b in zip(out, out[1:]) if a[0] == b[0]]
    if dupes:
        raise ValueError(f"duplicate dates, first {dupes[0]}")
    if not out:
        raise ValueError("no rows")
    return out


def stage(source_root, out_csv):
    """Writes out_csv (header always, so engine-gl starts with zero instruments too); returns one note per instrument."""
    out_csv = Path(out_csv)
    out_csv.parent.mkdir(parents=True, exist_ok=True)
    notes, lines = [], []
    for ticker, rel in MANIFEST:
        path = Path(source_root) / rel
        if not path.is_file():
            notes.append(f"{ticker}: missing {path}; engine-gl answers needs_data")
            continue
        try:
            bars = read_export(path)
        except ValueError as e:
            notes.append(f"{ticker}: not staged ({e}); engine-gl answers needs_data")
            continue
        lines += [[ticker, *bar, ""] for bar in bars]
        notes.append(f"{ticker}: {len(bars)} bars {bars[0][0]} to {bars[-1][0]}")
    tmp = out_csv.with_name(out_csv.name + ".tmp")
    with open(tmp, "w", newline="") as f:
        writer = csv.writer(f)
        writer.writerow(HEADER)
        writer.writerows(lines)
    os.replace(tmp, out_csv)
    return notes


if __name__ == "__main__":
    if len(sys.argv) != 3:
        raise SystemExit("usage: stage_gl.py SOURCE_ROOT OUT_CSV")
    print(json.dumps(stage(sys.argv[1], sys.argv[2]), indent=1))
```

Then two added lines in `Context/guru/webapp/scripts/setup.py`: after `import os, secrets, shutil` add
```python
import stage_gl
```
and directly before the line `for item in [data, *data.rglob('*')]: item.chmod(0o755 if item.is_dir() else 0o644)` add
```python
for note in stage_gl.stage(root, data/'gl'/'gl_data.csv'): print(f'engine-gl: {note}')
```
(`setup.py` runs as a script, so its own directory is on `sys.path`; staging before the `chmod` line gives the staged file the same 0644/0755 modes as the other runtime data. Do **not** run `setup.py` here: it writes runtime secrets and `Context/guru/webapp/.env`.)

- [ ] **Step 4: Run tests**

Run: `python3 -m unittest discover -s Context/guru/webapp/scripts -p 'test_*.py' -v`
Expected: 4 tests OK. Also `git diff -U0 -- Context/guru/webapp/scripts/setup.py | grep '^-[^-]'` → no output.

- [ ] **Step 5: Commit**

```bash
git add Context/guru/webapp/scripts/stage_gl.py Context/guru/webapp/scripts/test_engine_gl.py Context/guru/webapp/scripts/setup.py
git commit -m "Stage provisional global and crypto exports for engine-gl"
```

---

### Task 4: engine-gl answers lab routes only — the one `main.rs` line, real-data check and local smoke run

**Files:**
- Modify: `Context/guru/backend/src/main.rs` (one added line in `handle()`, no edited line)

**Interfaces:**
- Consumes: `lab::markets::{engine_route_allowed, engine_market, refused_json}` (Task 1); `stage_gl.py` CLI (Task 3).
- Produces: an engine process that, with `VAULT_LAB_MARKET=gl`, returns `404 application/json` `refused_json(path)` for every path outside `/api/lab/` (including `POST /api/notes`), and behaves exactly as before when the variable is unset.

- [ ] **Step 1: Failing check** — the predicate is unit-tested (Task 1); the observable failure is the process: build and run the current engine in gl mode against a one-instrument file and request a book route.

```bash
cd Context/guru/backend && CARGO_BUILD_JOBS=2 cargo build --release
mkdir -p "$GLSTAGE/tiny" && printf 'Ticker,Date,Open,High,Low,Close,Volume\nTVC:CAC40,2020-01-02,1,2,0.5,1.5,\nTVC:CAC40,2020-01-03,1.5,2,1,1.8,\n' > "$GLSTAGE/tiny/gl_data.csv"
PORT=$(python3 -c 'import socket;s=socket.socket();s.bind(("127.0.0.1",0));print(s.getsockname()[1])')
VAULT_LAB_MARKET=gl target/release/guru "$GLSTAGE/tiny/gl_data.csv" $PORT "$GLSTAGE/no-vix" "$GLSTAGE/no-extra" & PID=$!; sleep 1
curl -s -o /dev/null -w '%{http_code}\n' "http://127.0.0.1:$PORT/api/guru/connors/strategies"; kill $PID
```
Expected now: `200` (the book catalogue is served on provisional data — the bug this task closes).

- [ ] **Step 2: Implement** — in `main.rs` `handle()`, directly after the line `    let (path, q) = query(target);` add exactly:

```rust
    if !lab::markets::engine_route_allowed(lab::markets::engine_market().as_deref(), &path) { return respond(&mut stream, "404 Not Found", "application/json; charset=utf-8", lab::markets::refused_json(&path).as_bytes()); }
```

- [ ] **Step 3: Re-run the Step 1 commands** (rebuild first). Expected: `404`; `curl -s "http://127.0.0.1:$PORT/api/guru/connors/strategies"` starts `{"provisional":true,"provisional_note":"Provisional · TradingView export · internal research only, not publishable to YT Strategies","status":"not_found","error":"engine-gl serves indicator-lab routes only; …`. Without `VAULT_LAB_MARKET` the same request returns `200` (India/US engines unchanged). Stop the process.

- [ ] **Step 4: Real-data staging and test** (NAS read-only; output on local disk):

```bash
NAS="/run/user/1000/gvfs/smb-share:server=192.168.4.50,share=share/Manan Pujara/Book-to-rust"
python3 Context/guru/webapp/scripts/stage_gl.py "$NAS" "$GLSTAGE/gl_data.csv"
cd Context/guru/backend && VAULT_GL_CSV="$GLSTAGE/gl_data.csv" CARGO_BUILD_JOBS=2 cargo test staged_gl_data -- --nocapture
```
Expected: ten notes, e.g. `"CRYPTO:BTCUSD: 6099 bars 2009-10-05 to 2026-09-22"`, `"HSI:HSI: 9828 bars 1986-12-31 to 2026-09-24"` (later end dates are fine if the exports were refreshed); `1 passed` with no "not staged" line. If the NAS is not mounted, record "real-data check pending" in the report — never copy data into the repository.

- [ ] **Step 5: Real-data smoke** — run `VAULT_LAB_MARKET=gl target/release/guru "$GLSTAGE/gl_data.csv" $PORT "$GLSTAGE/no-vix" "$GLSTAGE/no-extra"` on a fresh free port and check (spec = `{"v":1,"entry":{"all":[{"lhs":{"s":"rsi","p":{"length":2}},"op":"crosses_below","rhs":{"value":10}}]},"exit":{"any":[{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":5}}]}}` URL-encoded with `python3 -c 'import urllib.parse,sys;print(urllib.parse.quote(sys.argv[1]))' '<spec>'`):

| Request | Expected |
|---|---|
| `/api/lab/instruments` | 200, `provisional:true`, 10 rows `status:"ready"` |
| `/api/lab/run?ticker=CRYPTO:BTCUSD&spec=…` | 200, `from` `2011-01-01`, `start.reason` "opens before 2011-01-01 are stale", `instrument.bars_per_year` 365 |
| `/api/lab/run?ticker=HSI:HSI&from=1990-01-02&spec=…` | 400 `needs_data` "… choose from >= 1992-01-01" |
| `/api/lab/run?ticker=IG:DAX&spec=<OBV > 0 entry>` | 400 `needs_data` "On Balance Volume: needs volume …" |
| `/api/lab/run?ticker=NIFTY%2050&spec=…` | 400 `invalid` "unknown ticker NIFTY 50" |
| `/api/tickers` | 404 `refused_json` |

Stop the process.

- [ ] **Step 6: Empty-data smoke** — `mkdir -p "$GLSTAGE/empty-src" && python3 Context/guru/webapp/scripts/stage_gl.py "$GLSTAGE/empty-src" "$GLSTAGE/empty/gl_data.csv"`, start the engine on it in gl mode: the log says `(0 tickers from …)`; `/api/lab/instruments` lists 10 rows `status:"needs_data"`; `/api/lab/catalogue` is 200 with the provisional prefix. Stop the process.

- [ ] **Step 7: Guard + commit**

```bash
python3 scripts/check_book_untouched.py --base origin/Manan
git diff -U0 origin/Manan -- Context/guru/backend/src/main.rs | grep '^[-+][^-+]'
git add Context/guru/backend/src/main.rs
git commit -m "Restrict engine-gl to indicator-lab routes"
```
Expected: guard `"problems": []`; the diff shows the Phase 2 `+` lab arm and this one `+` line, no `-` line.

---

### Task 5: Gateway — market `gl` for lab routes only, proxied to engine-gl

**Files:**
- Modify: `Context/guru/webapp/gateway/src/policy.rs` (additive only)
- Modify: `Context/guru/webapp/gateway/src/main.rs` (two added lines, two edited lines)

**Interfaces:**
- Consumes: engine routes `/api/lab/{catalogue,series,run,optimize,instruments}` (Tasks 2, 4).
- Produces: `allowed_engine_path("lab/instruments") == true`; `pub fn engine_market_allows(market: &str, path: &str) -> bool` (`in`/`us`: true; `gl`: `path.starts_with("lab/")`; else false); `App.gl` from `VAULT_ENGINE_GL` (default `http://127.0.0.1:8092`). `/api/gl/lab/*` gets the same session check, role/slot rules for `lab/optimize`, spec bounds and timeouts as India/US.

- [ ] **Step 1: Write the failing test** — append to `policy.rs`:

```rust
#[cfg(test)]
mod gl_tests {
    use super::*;
    #[test]
    fn global_and_crypto_reach_lab_routes_only() {
        for path in ["lab/catalogue", "lab/series", "lab/run", "lab/optimize", "lab/instruments"] {
            assert!(allowed_engine_path(path) && engine_market_allows("gl", path), "{path}");
            assert!(engine_market_allows("in", path) && engine_market_allows("us", path));
        }
        for path in ["tickers", "ohlc", "gurus", "guru/connors/run", "guru/connors/optimize", "guru/chande/summary", "notes"] {
            assert!(!engine_market_allows("gl", path), "{path}");
            assert!(engine_market_allows("in", path) && engine_market_allows("us", path), "{path}");
        }
        for market in ["crypto", "GL", "", "eu", "in/../gl"] {
            assert!(!engine_market_allows(market, "lab/run"), "{market}");
        }
    }
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cd Context/guru/webapp && CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="${VAULT_RUNTIME_ROOT:-$HOME/.cache/strategy-vault-webapp}/test-target" cargo test --locked --manifest-path gateway/Cargo.toml`
Expected: compile error `cannot find function engine_market_allows`.

- [ ] **Step 3: Implement** — in `policy.rs` `allowed_engine_path`, after the line `            | "lab/optimize"` add:
```rust
            | "lab/instruments"
```
Append to `policy.rs` (above the new test module):
```rust
/// Engine markets: India and US serve every allowlisted path; `gl` (engine-gl: provisional global and crypto
/// TradingView exports) serves indicator-lab paths only, because book strategies have no admission rules for that data.
pub fn engine_market_allows(market: &str, path: &str) -> bool {
    match market {
        "in" | "us" => true,
        "gl" => path.starts_with("lab/"),
        _ => false,
    }
}
```
In `gateway/src/main.rs`:
- `struct App`: after `    us: String,` add `    gl: String,`
- `engine_proxy`: replace `    if !matches!(market.as_str(), "in" | "us") || !policy::valid_query(uri.query()) {` with
  ```rust
      if !policy::engine_market_allows(&market, &path) || !policy::valid_query(uri.query()) {
  ```
- `engine_proxy`: replace `    let engine = if market == "in" { &app.india } else { &app.us };` with
  ```rust
      let engine = match market.as_str() { "in" => &app.india, "gl" => &app.gl, _ => &app.us };
  ```
- `main()`: after `        us: env("VAULT_ENGINE_US", "http://127.0.0.1:8091"),` add
  ```rust
          gl: env("VAULT_ENGINE_GL", "http://127.0.0.1:8092"),
  ```
`health()` is not changed (Decision 11).

- [ ] **Step 4: Run tests** — same command as Step 2 → `7 passed`. `git diff -U0 -- Context/guru/webapp/gateway/src/policy.rs | grep '^-[^-]'` → no output; `git diff -U0 -- Context/guru/webapp/gateway/src/main.rs | grep '^-[^-]'` → exactly the two replaced lines.

- [ ] **Step 5: Guard + commit**

```bash
python3 scripts/check_book_untouched.py --base origin/Manan
git add Context/guru/webapp/gateway/src/policy.rs Context/guru/webapp/gateway/src/main.rs
git commit -m "Route global and crypto lab requests to engine-gl"
```

---

### Task 6: Compose — the `engine-gl` service

**Files:**
- Modify: `Context/guru/webapp/compose.yaml` (additions only)
- Test: `Context/guru/webapp/scripts/test_engine_gl.py` (append class `EngineGlCompose`)

**Interfaces:**
- Consumes: `stage_gl.stage` output path `<runtime>/data/gl/gl_data.csv` (Task 3); `VAULT_LAB_MARKET=gl` (Tasks 1, 4); `VAULT_ENGINE_GL` (Task 5).
- Produces: service `engine-gl`; gateway env `VAULT_ENGINE_GL: http://engine-gl:8090`; `depends_on.engine-gl: service_started`.

- [ ] **Step 1: Write the failing tests** — append to `test_engine_gl.py`, before `if __name__ == "__main__":`:

```python
class EngineGlCompose(unittest.TestCase):
    def setUp(self):
        import yaml  # PyYAML; the phase gate machine has it
        self.compose = yaml.safe_load((REPO / "Context/guru/webapp/compose.yaml").read_text())
        self.services = self.compose["services"]

    def test_engine_gl_reuses_the_engine_image_with_only_staged_gl_data_read_only(self):
        gl, us = self.services["engine-gl"], self.services["engine-us"]
        self.assertEqual(self.compose["name"], "strategy-vault")
        self.assertEqual(gl["image"], us["image"], "same engine image; no fork of strategy calculations")
        self.assertNotIn("build", gl)
        self.assertNotIn("ports", gl, "reachable only through the gateway")
        self.assertEqual(gl["environment"], {"VAULT_LAB_MARKET": "gl"})
        self.assertEqual(gl["command"][:2], ["/data/gl/gl_data.csv", "8090"])
        self.assertEqual(gl["volumes"], ["${VAULT_RUNTIME_ROOT}/data/gl:/data/gl:ro"])
        self.assertLessEqual(gl["cpus"], 2)
        self.assertIn("mem_limit", gl)

    def test_gateway_routes_gl_to_engine_gl(self):
        gateway = self.services["gateway"]
        self.assertEqual(gateway["environment"]["VAULT_ENGINE_GL"], "http://engine-gl:8090")
        self.assertEqual(gateway["depends_on"]["engine-gl"], {"condition": "service_started"})

    def test_setup_stages_gl_data_before_fixing_permissions(self):
        setup = (REPO / "Context/guru/webapp/scripts/setup.py").read_text()
        self.assertLess(setup.index("stage_gl.stage(root, data/'gl'/'gl_data.csv')"), setup.index("for item in [data, *data.rglob('*')]"))
```

- [ ] **Step 2: Run to verify failure**

Run: `python3 -m unittest discover -s Context/guru/webapp/scripts -p 'test_*.py'`
Expected: `KeyError: 'engine-gl'` and `KeyError: 'VAULT_ENGINE_GL'` (2 errors; the setup check passes since Task 3).

- [ ] **Step 3: Implement** — in `compose.yaml`, insert directly before `  gateway:` (after the `engine-us` block's `restart: unless-stopped`):

```yaml
  engine-gl:
    # Provisional global/crypto TradingView exports (staged by scripts/setup.py); lab routes only, never publishable.
    image: ${VAULT_ENGINE_IMAGE:-strategy-vault-engine:local}
    command: [/data/gl/gl_data.csv, '8090', /data/gl/no-vix, /data/gl/no-extra]
    environment:
      VAULT_LAB_MARKET: gl
    volumes:
      - '${VAULT_RUNTIME_ROOT}/data/gl:/data/gl:ro'
    cpus: 1
    cpuset: '${VAULT_CPUSET:-0,1}'
    mem_limit: 512m
    restart: unless-stopped
```
In `gateway.environment`, after `      VAULT_ENGINE_US: http://engine-us:8090` add `      VAULT_ENGINE_GL: http://engine-gl:8090`; in `gateway.depends_on`, after `      engine-us: {condition: service_started}` add `      engine-gl: {condition: service_started}`.

- [ ] **Step 4: Run tests** — same command → `Ran 7 tests … OK`. `git diff -U0 -- Context/guru/webapp/compose.yaml | grep '^-[^-]'` → no output. Do **not** run `docker compose config`/`up` (manual gate, Task 7 Step 8).

- [ ] **Step 5: Commit**

```bash
git add Context/guru/webapp/compose.yaml Context/guru/webapp/scripts/test_engine_gl.py
git commit -m "Add engine-gl service for provisional global and crypto data"
```

---

### Task 7: Phase gate

- [ ] **Step 1: Engine suite** — `cd Context/guru/backend && VAULT_GL_CSV="$GLSTAGE/gl_data.csv" VAULT_IDX="/run/user/1000/gvfs/smb-share:server=192.168.4.50,share=share/Manan Pujara/Book-to-rust/idx_data.csv" CARGO_BUILD_JOBS=2 nice -n 10 cargo test` → all pass; record the count and the count on `$PHASE5_BASE` (planning: 701 passed / 1 ignored vs 690 / 1).
- [ ] **Step 2: Release build without lab warnings** — `cd Context/guru/backend && touch src/main.rs && CARGO_BUILD_JOBS=2 cargo build --release 2>&1 | grep -A3 '^warning' | grep -- '--> src/lab/'` → no output.
- [ ] **Step 3: Book guard, extended (engine code changed)** — `NAS="/run/user/1000/gvfs/smb-share:server=192.168.4.50,share=share/Manan Pujara/Book-to-rust"; python3 scripts/check_book_untouched.py --base origin/Manan --engine-compare --csv "$NAS/idx_data.csv" --vix "$NAS/india_vix" --extra "$NAS/extra_data"` → `"problems": []`; copy the `notes` (hash count, basket note, ross optimize note) into the report. The compared engines run without `VAULT_LAB_MARKET`, which is exactly the India/US configuration.
- [ ] **Step 4: Gateway** — `cd Context/guru/webapp && CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="${VAULT_RUNTIME_ROOT:-$HOME/.cache/strategy-vault-webapp}/test-target" cargo test --locked --manifest-path gateway/Cargo.toml` → 7 passed.
- [ ] **Step 5: Scripts** — `python3 -m unittest discover -s Context/guru/webapp/scripts -p 'test_*.py'` (7 OK) and `python3 -m unittest discover -s scripts -p 'test_*.py'` (unchanged count, OK).
- [ ] **Step 6: Upload check** — `python3 scripts/check_upload.py` → `"findings": []` (no staged CSV, `.env` or data file tracked).
- [ ] **Step 7: Scope proof** — `git diff $PHASE5_BASE --stat` lists only the ten files in "File structure" (plus any other agent's commits, which must be identified by `git log $PHASE5_BASE..HEAD --format='%h %an %s'` and excluded from this phase's report). Additive proof: `git diff $PHASE5_BASE -U0 -- Context/guru/backend/src/main.rs Context/guru/webapp/gateway/src/policy.rs Context/guru/webapp/scripts/setup.py Context/guru/webapp/compose.yaml Context/guru/backend/src/lab/mod.rs | grep '^-[^-]'` → no output; `git diff $PHASE5_BASE -U0 -- Context/guru/webapp/gateway/src/main.rs | grep -c '^-[^-]'` → `2`. No protected path: `git diff $PHASE5_BASE --name-only | grep -E 'connors\.rs|/books/|readiness\.rs|gate\.rs|instrument_policy\.rs|baskets\.rs|accounting\.rs|trade_validation\.rs|compat/|ResearchPages\.tsx|frontend/|auth/|db/'` → no output.
- [ ] **Step 8: Manual gate item for the user — setup + compose on an isolated stack (spec §9 exit criterion). Not run by an agent.** On the user's own isolated dev stack (separate runtime dir, port, engine image tag and Compose project name, `docs/DEVELOPMENT.md` "Two developers on one machine"; never the live `strategy-vault` project):
  ```bash
  NAS="/run/user/1000/gvfs/smb-share:server=192.168.4.50,share=share/Manan Pujara/Book-to-rust"
  VAULT_RUNTIME_ROOT="$HOME/.cache/strategy-vault-dev-<name>" VAULT_SOURCE_ROOT="$NAS" python3 Context/guru/webapp/scripts/setup.py
  #   → ten "engine-gl: <id>: N bars …" lines; $HOME/.cache/strategy-vault-dev-<name>/data/gl/gl_data.csv exists
  docker compose -p strategy-vault-dev-<name> -f Context/guru/webapp/compose.yaml --env-file "$HOME/.cache/strategy-vault-dev-<name>/.env" up -d --build
  docker compose -p strategy-vault-dev-<name> -f Context/guru/webapp/compose.yaml --env-file "$HOME/.cache/strategy-vault-dev-<name>/.env" logs engine-gl | tail -3
  #   → "Guru API on … (10 tickers from /data/gl/gl_data.csv)"
  ```
  Signed in at the dev URL (administrator credentials in `$HOME/.cache/strategy-vault-dev-<name>/administrator.txt`), in the browser (or `curl -b <cookie jar>`), check:
  | URL | Expected |
  |---|---|
  | `/healthz` | `{"status":"ok"}` |
  | `/api/gl/lab/instruments` | 200, `"provisional":true`, 10 × `"status":"ready"` |
  | `/api/gl/lab/run?ticker=CRYPTO:BTCUSD&spec=<encoded RSI spec from Task 4>` | 200, starts `{"provisional":true`, `instrument.label` "Provisional · TradingView export CRYPTO:BTCUSD, daily" |
  | `/api/gl/guru/connors/strategies`, `/api/gl/tickers` | 400 `{"error":"Invalid market or query."}` |
  | `/api/crypto/lab/catalogue` | 400 `{"error":"Invalid market or query."}` |
  | `/api/in/tickers`, `/api/us/lab/catalogue` | 200, no `provisional` key |
  | `/api/gl/lab/optimize?...` as a viewer | 403 "Researcher access is required for optimization." |

  Empty-data check: `docker compose -p strategy-vault-dev-<name> … stop engine-gl`, replace `data/gl/gl_data.csv` in the **dev** runtime with its header line only, `… start engine-gl` → `/api/gl/lab/instruments` lists 10 × `needs_data`, `/healthz` still ok, `/api/in/tickers` still 200; re-run the `setup.py` line to restore. Tear down with `docker compose -p strategy-vault-dev-<name> … down`. Record the outcome in the PR description ("pending user run" until done).
- [ ] **Step 9: Report** — commit any gate fixes (named files only), then draft the PR description: test counts before/after, guard notes, the data table and Decisions 1–18, the frontend API notes, the manual gate result or "pending user run", and the carried open item from Phase 1 (TradingView parity fixtures still need the user's waiver before merge into Manan). Do not push; the user pushes.
