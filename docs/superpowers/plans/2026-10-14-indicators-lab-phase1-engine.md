# Indicators Lab — Phase 1 (engine indicators) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a self-contained `lab` module to the Rust engine with daily→weekly/monthly resampling and the Batch A indicator catalogue (~65 TradingView-defined indicators), each pinned by tests, without changing any book strategy result.

**Architecture:** New files only under `Context/guru/backend/src/lab/`; `main.rs` gains the single line `mod lab;`. Indicators are pure functions over a `Bars` struct, registered in one catalogue with declared parameters, outputs, data needs and first-valid bar. Generic catalogue tests check every indicator for length, warm-up, causality (no lookahead) and data needs; family tests pin formulas with hand-computed and TradingView-verified values. A guard script proves book files and book outputs are untouched.

**Tech Stack:** Rust 2021, std only (no new crates), `cargo test`; Python 3 std-lib for the guard; git.

**Spec:** `docs/superpowers/specs/2026-10-14-indicators-lab-design.md` (§2a book protection, §5 catalogue, §8 testing, §9 phase 1).

## Global Constraints

- Branch `Nandini` only; never commit or push to `main`; no force-push. Commits authored by the repo-local identity; no `Co-Authored-By: Claude` trailer.
- Do not edit: `backend/src/connors.rs`, `backend/src/books/**`, `readiness.rs`, `gate.rs`, `instrument_policy.rs`, `baskets.rs`, `accounting.rs`, `trade_validation.rs`, `frontend/react/src/compat/*`, `src/pages/ResearchPages.tsx`.
- `main.rs` change in this phase is exactly one added line: `mod lab;` after `mod trade_validation;`.
- No new Cargo dependencies (`[dependencies]` stays empty). No data files, CSVs or exports committed.
- Cargo concurrency bounded: always `CARGO_BUILD_JOBS=2`. No whole-corpus backtests.
- NaN means "no value" (warm-up, missing input, division by zero). Every comparison with NaN is false.
- Formulas follow TradingView built-in chart indicators (spec §5); where a built-in and the same-named `ta.*` differ, the **built-in** is implemented and the difference is written in the definition string.
- Test tolerance for floating comparisons: relative 1e-9 (absolute 1e-9 near zero).

## Review Focus

1. **Lookahead through a higher timeframe:** a weekly value must not be visible on Monday–Thursday of its own week — the causality test in Task 3 pins it on a partial week and a holiday week.
2. **Instruments without volume (indices, CFDs, crypto exports):** volume indicators must return `Err("needs volume")`, never zeros — generic test in Task 4.
3. **Flat bars / zero ranges (high == low, constant closes):** Stochastic, Williams %R, CCI, %B, CMF, RSI must return NaN (or RSI's 0/100 saturation), never ±inf — `flat_series_never_inf` test in Task 4.
4. **Out-of-range or non-integer parameters from a user** (length 0, 10.5, 100000): `resolve_params` rejects with a message naming the parameter — Task 4 tests.
5. **Short histories (fewer bars than the warm-up):** output is all-NaN of full length, no panic — `short_history_all_nan` generic test in Task 4.

---

## File structure

```
Context/guru/backend/src/lab/
  mod.rs          pub mod declarations + module doc (conventions)
  bars.rs         Bars (columnar OHLCV + date + `end` index) and price sources
  ta.rs           primitives with TradingView NaN semantics
  resample.rs     Tf, calendar maths, resample(), expand()
  catalogue.rs    ParamSpec, IndicatorDef, Params, all(), get(), resolve_params(), generic tests
  ma.rs           moving-average family (defs + compute fns + tests)
  momentum.rs     momentum/oscillators family
  trend.rs        trend family
  volatility.rs   volatility family
  volume.rs       volume family (+ listed-only VWAP / opening range)
  price.rs        price functions family
  testdata.rs     #[cfg(test)] deterministic synthetic bars + assert helpers
scripts/
  check_book_untouched.py        guard (spec §2a)
  test_check_book_untouched.py   guard tests (run by CI's unittest discover)
```

---

### Task 1: Book-protection guard

**Files:**
- Create: `scripts/check_book_untouched.py`
- Test: `scripts/test_check_book_untouched.py`

**Interfaces:**
- Produces: `check(repo: Path, base: str) -> list[str]` (list of violations, empty = pass); CLI `python3 scripts/check_book_untouched.py [--base origin/Manan] [--engine-compare --csv … --vix … --extra …]`, exit 1 on any violation.

- [ ] **Step 1: Write the failing test**

```python
"""Book protection guard uses real temporary Git histories."""
import importlib.util
from pathlib import Path
import subprocess
import tempfile
import unittest

MODULE = Path(__file__).with_name("check_book_untouched.py")
spec = importlib.util.spec_from_file_location("check_book_untouched", MODULE)
guard = importlib.util.module_from_spec(spec)
spec.loader.exec_module(guard)


def git(root, *args):
    return subprocess.check_output(["git", "-C", str(root), *args], stderr=subprocess.DEVNULL, text=True).strip()


class BookGuard(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.repo = Path(self.temp.name)
        git(self.repo, "init", "-q", "-b", "Manan")
        git(self.repo, "config", "user.email", "t@example.com")
        git(self.repo, "config", "user.name", "t")
        src = self.repo / "Context/guru/backend/src"
        (src / "books").mkdir(parents=True)
        (src / "connors.rs").write_text("fn a() {}\n")
        (src / "books/x.rs").write_text("fn b() {}\n")
        (src / "main.rs").write_text("mod books;\nmod trade_validation;\nfn main() {}\n")
        git(self.repo, "add", "-A")
        git(self.repo, "commit", "-q", "-m", "base")
        git(self.repo, "switch", "-q", "-c", "Nandini")

    def tearDown(self):
        self.temp.cleanup()

    def commit(self, path, text):
        p = self.repo / path
        p.parent.mkdir(parents=True, exist_ok=True)
        p.write_text(text)
        git(self.repo, "add", "-A")
        git(self.repo, "commit", "-q", "-m", "change")

    def test_new_lab_files_and_additive_main_pass(self):
        self.commit("Context/guru/backend/src/lab/mod.rs", "pub mod ta;\n")
        self.commit("Context/guru/backend/src/main.rs", "mod books;\nmod trade_validation;\nmod lab;\nfn main() {}\n")
        self.assertEqual(guard.check(self.repo, "Manan"), [])

    def test_protected_file_edit_fails(self):
        self.commit("Context/guru/backend/src/books/x.rs", "fn b() { let _ = 1; }\n")
        self.assertTrue(any("books/x.rs" in v for v in guard.check(self.repo, "Manan")))

    def test_removed_line_in_additive_file_fails(self):
        self.commit("Context/guru/backend/src/main.rs", "mod books;\nfn main() {}\n")
        self.assertTrue(any("main.rs" in v and "removes" in v for v in guard.check(self.repo, "Manan")))

    def test_uncommitted_protected_edit_fails(self):
        (self.repo / "Context/guru/backend/src/connors.rs").write_text("fn a() { }\n")
        self.assertTrue(any("connors.rs" in v for v in guard.check(self.repo, "Manan")))


if __name__ == "__main__":
    unittest.main()
```

- [ ] **Step 2: Run test to verify it fails**

Run: `python3 -m unittest scripts/test_check_book_untouched.py -v`
Expected: FAIL / ERROR — `check_book_untouched.py` does not exist.

- [ ] **Step 3: Write the implementation**

```python
#!/usr/bin/env python3
"""Fail when a branch changes book strategies or their results (indicators lab spec §2a).

Checks (always): no edit to protected paths; only added lines in additive-only files.
Optional --engine-compare: build base and HEAD engines and compare book catalogue/summary JSON hashes.
"""
import argparse, hashlib, io, json, os, socket, subprocess, sys, tarfile, tempfile, time, urllib.request
from pathlib import Path

PROTECTED = (
    "Context/guru/backend/src/connors.rs", "Context/guru/backend/src/books/",
    "Context/guru/backend/src/readiness.rs", "Context/guru/backend/src/gate.rs",
    "Context/guru/backend/src/instrument_policy.rs", "Context/guru/backend/src/baskets.rs",
    "Context/guru/backend/src/accounting.rs", "Context/guru/backend/src/trade_validation.rs",
    "Context/guru/frontend/react/src/compat/", "Context/guru/frontend/react/src/pages/ResearchPages.tsx",
)
ADDITIVE_ONLY = (
    "Context/guru/backend/src/main.rs", "Context/guru/webapp/gateway/src/policy.rs",
    "Context/guru/frontend/react/src/app/custom-pages.ts",
)
GURUS = ("connors", "chande", "ross")


def git(repo, *args):
    return subprocess.check_output(["git", "-C", str(repo), *args], text=True)


def check(repo, base):
    mb = git(repo, "merge-base", base, "HEAD").strip()
    changed = [p for p in git(repo, "diff", "--name-only", mb).splitlines() if p]
    problems = [f"protected file changed: {p}" for p in changed if p.startswith(PROTECTED)]
    for path in ADDITIVE_ONLY:
        diff = git(repo, "diff", "-U0", mb, "--", path)
        removed = [l for l in diff.splitlines() if l.startswith("-") and not l.startswith("---")]
        if removed:
            problems.append(f"{path} removes or edits {len(removed)} existing line(s); additions only")
    return problems


def free_port():
    with socket.socket() as s:
        s.bind(("127.0.0.1", 0))
        return s.getsockname()[1]


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
        out = {}
        for g in GURUS:
            for route in ("strategies", "summary?ticker=NIFTY%2050"):
                body = urllib.request.urlopen(f"http://127.0.0.1:{port}/api/guru/{g}/{route}", timeout=600).read()
                if not body:
                    raise RuntimeError(f"empty response for {g}/{route}")
                out[f"{g}/{route}"] = hashlib.sha256(body).hexdigest()
        return out
    finally:
        proc.terminate()
        proc.wait()


def compare_engines(repo, base, csv, vix, extra):
    # Plain export of the base commit into a temp folder (no git worktree, per user preference).
    with tempfile.TemporaryDirectory() as tmp:
        archive = subprocess.run(["git", "-C", str(repo), "archive", base], capture_output=True, check=True).stdout
        with tarfile.open(fileobj=io.BytesIO(archive)) as tar:
            tar.extractall(tmp, filter="data")
        before = engine_hashes(tmp, csv, vix, extra)
    after = engine_hashes(repo, csv, vix, extra)
    return [f"book output changed: {k}" for k in before if before[k] != after.get(k)]


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--base", default="origin/Manan")
    ap.add_argument("--engine-compare", action="store_true")
    ap.add_argument("--csv"); ap.add_argument("--vix"); ap.add_argument("--extra")
    a = ap.parse_args()
    repo = Path(git(Path.cwd(), "rev-parse", "--show-toplevel").strip())
    problems = check(repo, a.base)
    if a.engine_compare:
        if not (a.csv and a.vix and a.extra):
            ap.error("--engine-compare needs --csv, --vix and --extra (read-only data paths)")
        problems += compare_engines(repo, a.base, a.csv, a.vix, a.extra)
    print(json.dumps({"base": a.base, "engine_compared": a.engine_compare, "problems": problems}, indent=2))
    sys.exit(1 if problems else 0)


if __name__ == "__main__":
    main()
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `python3 -m unittest scripts/test_check_book_untouched.py -v` → Expected: 4 tests OK.
Run: `python3 scripts/check_book_untouched.py --base origin/Manan` → Expected: `"problems": []`, exit 0.

- [ ] **Step 5: Commit**

```bash
git add scripts/check_book_untouched.py scripts/test_check_book_untouched.py
git commit -m "Add book-protection guard for the indicators lab branch"
```

---

### Task 2: Lab scaffold, `Bars` and TradingView primitives

**Files:**
- Create: `Context/guru/backend/src/lab/mod.rs`, `bars.rs`, `ta.rs`, `testdata.rs`
- Modify: `Context/guru/backend/src/main.rs` — add the line `mod lab;` immediately after `mod trade_validation;` (line 11). Nothing else.

**Interfaces:**
- Produces:
  - `pub struct Bars { pub date: Vec<String>, pub open: Vec<f64>, pub high: Vec<f64>, pub low: Vec<f64>, pub close: Vec<f64>, pub volume: Option<Vec<f64>>, pub end: Vec<usize> }`
  - `impl Bars { pub fn from_rows(rows: &[crate::Row]) -> Bars; pub fn len(&self) -> usize; pub fn hl2(&self) -> Vec<f64>; pub fn hlc3(&self) -> Vec<f64>; pub fn ohlc4(&self) -> Vec<f64> }` — `volume` is `Some` only if every row has a volume; `end[i] == i` for daily bars.
  - `ta`: `sma, ema, rma, wma, sum (f(&[f64], usize) -> Vec<f64>)`, `stdev(&[f64], usize) -> Vec<f64>` (population), `dev(&[f64], usize)` (mean abs deviation), `highest, lowest (&[f64], usize)`, `change(&[f64], usize)`, `tr(b: &Bars, handle_na: bool) -> Vec<f64>`, `linreg(&[f64], usize, offset: usize)`, `nz(f64) -> f64`, `NA: f64 = f64::NAN`.
  - `testdata` (`#[cfg(test)]`): `pub fn walk(n: usize, seed: u64, with_volume: bool) -> Bars`, `pub fn from_ohlc(rows: &[(f64,f64,f64,f64)]) -> Bars`, `pub fn close_eq(a: f64, b: f64) -> bool`, `pub fn assert_series(got: &[f64], want: &[f64])`.

- [ ] **Step 1: Write the failing tests** (in `ta.rs`, `#[cfg(test)] mod tests`)

```rust
use super::*;
use crate::lab::testdata::{assert_series, from_ohlc};
const N: f64 = f64::NAN;

#[test] fn sma_basic_and_na_compaction() {
    assert_series(&sma(&[1., 2., 3., 4.], 2), &[N, 1.5, 2.5, 3.5]);
    // na bar returns na and does not enter the window (TradingView G4)
    assert_series(&sma(&[1., N, 3., 5.], 2), &[N, N, 2., 4.]);
}
#[test] fn ema_seeds_with_sma_of_first_n_verified_on_tradingview() {
    // alpha = 2/3 for n=2; seed = (1+2)/2 = 1.5 at index 1; then 1.5 + 2/3*(3-1.5) = 2.5
    assert_series(&ema(&[1., 2., 3.], 2), &[N, 1.5, 2.5]);
}
#[test] fn rma_wilder() { assert_series(&rma(&[1., 2., 3., 4.], 2), &[N, 1.5, 2.25, 3.125]); }
#[test] fn wma_linear_weights() { assert_series(&wma(&[1., 2., 3.], 3), &[N, N, (1. + 4. + 9.) / 6.]); }
#[test] fn stdev_is_population() { assert_series(&stdev(&[1., 3.], 2), &[N, 1.]); }
#[test] fn highest_lowest_window() {
    assert_series(&highest(&[3., 1., 2., 5.], 2), &[N, 3., 2., 5.]);
    assert_series(&lowest(&[3., 1., 2., 5.], 2), &[N, 1., 1., 2.]);
}
#[test] fn tr_bar0_handle_na() {
    let b = from_ohlc(&[(10., 12., 9., 11.), (11., 13., 10.5, 12.)]);
    assert_series(&tr(&b, true), &[3., 2.5]);
    assert_series(&tr(&b, false), &[N, 2.5]);
}
#[test] fn linreg_fits_line_exactly() { assert_series(&linreg(&[1., 3., 5.], 3, 0), &[N, N, 5.]); }
#[test] fn dev_mean_absolute() { assert_series(&dev(&[1., 3.], 2), &[N, 1.]); }
```

- [ ] **Step 2: Run to verify failure**

Run: `cd Context/guru/backend && CARGO_BUILD_JOBS=2 cargo test lab::ta` → Expected: compile error (module missing).

- [ ] **Step 3: Implement**

`lab/mod.rs`:
```rust
//! Indicators lab — user-defined indicator research. Separate from book strategies:
//! nothing here is read by connors.rs, books/* or the admission files.
//! Conventions: series are Vec<f64> aligned to bars; f64::NAN = no value; comparisons with NaN are false;
//! formulas follow TradingView built-in chart indicators (see each definition string).
pub mod bars;
pub mod catalogue;
pub mod ma;
pub mod momentum;
pub mod price;
pub mod resample;
pub mod ta;
pub mod trend;
pub mod volatility;
pub mod volume;
#[cfg(test)]
pub mod testdata;
```
(Until Tasks 4–10 exist, declare only `bars`, `ta`, `testdata`; add each `pub mod` in the task that creates the file.)

`lab/bars.rs`:
```rust
use crate::Row;

/// Columnar OHLCV. `end[i]` = index of the last source (daily) bar inside bar i.
#[derive(Clone, Debug, Default)]
pub struct Bars {
    pub date: Vec<String>,
    pub open: Vec<f64>,
    pub high: Vec<f64>,
    pub low: Vec<f64>,
    pub close: Vec<f64>,
    pub volume: Option<Vec<f64>>,
    pub end: Vec<usize>,
}

impl Bars {
    pub fn from_rows(rows: &[Row]) -> Bars {
        let volume = if !rows.is_empty() && rows.iter().all(|r| r.volume.is_some()) {
            Some(rows.iter().map(|r| r.volume.unwrap()).collect())
        } else {
            None
        };
        Bars {
            date: rows.iter().map(|r| r.date.clone()).collect(),
            open: rows.iter().map(|r| r.open).collect(),
            high: rows.iter().map(|r| r.high).collect(),
            low: rows.iter().map(|r| r.low).collect(),
            close: rows.iter().map(|r| r.close).collect(),
            volume,
            end: (0..rows.len()).collect(),
        }
    }
    pub fn len(&self) -> usize { self.close.len() }
    pub fn hl2(&self) -> Vec<f64> { (0..self.len()).map(|i| (self.high[i] + self.low[i]) / 2.).collect() }
    pub fn hlc3(&self) -> Vec<f64> { (0..self.len()).map(|i| (self.high[i] + self.low[i] + self.close[i]) / 3.).collect() }
    pub fn ohlc4(&self) -> Vec<f64> { (0..self.len()).map(|i| (self.open[i] + self.high[i] + self.low[i] + self.close[i]) / 4.).collect() }
}
```

`lab/ta.rs`:
```rust
//! TradingView `ta.*` primitives. Windows over "length non-na values" (sma/ema/rma/sum/stdev/dev) skip NaN
//! bars, which output NaN; bar-counted windows (linreg/change) let one NaN blank the next `n` outputs.
use super::bars::Bars;
pub const NA: f64 = f64::NAN;
pub fn nz(x: f64) -> f64 { if x.is_nan() { 0. } else { x } }

/// Calls `f(window)` with the last `n` non-NaN values for every non-NaN bar once `n` exist.
fn compact<F: Fn(&[f64]) -> f64>(x: &[f64], n: usize, f: F) -> Vec<f64> {
    let mut out = vec![NA; x.len()];
    let mut win: Vec<f64> = Vec::new();
    for (i, &v) in x.iter().enumerate() {
        if v.is_nan() { continue; }
        win.push(v);
        if win.len() > n { win.remove(0); }
        if win.len() == n { out[i] = f(&win); }
    }
    out
}
pub fn sum(x: &[f64], n: usize) -> Vec<f64> { compact(x, n, |w| w.iter().sum()) }
pub fn sma(x: &[f64], n: usize) -> Vec<f64> { compact(x, n, |w| w.iter().sum::<f64>() / n as f64) }
fn smoothed(x: &[f64], n: usize, alpha: f64) -> Vec<f64> {
    let seed = sma(x, n);
    let mut out = vec![NA; x.len()];
    let mut prev = NA;
    for i in 0..x.len() {
        if x[i].is_nan() { continue; }
        prev = if prev.is_nan() { seed[i] } else { prev + alpha * (x[i] - prev) };
        out[i] = prev;
    }
    out
}
/// EMA seeded with the SMA of the first n non-NaN values (verified on TradingView 2026-10-14).
pub fn ema(x: &[f64], n: usize) -> Vec<f64> { smoothed(x, n, 2. / (n as f64 + 1.)) }
/// Wilder / RMA, alpha 1/n, SMA seed.
pub fn rma(x: &[f64], n: usize) -> Vec<f64> { smoothed(x, n, 1. / n as f64) }
pub fn wma(x: &[f64], n: usize) -> Vec<f64> {
    let den = (n * (n + 1)) as f64 / 2.;
    compact(x, n, |w| w.iter().enumerate().map(|(j, v)| v * (j + 1) as f64).sum::<f64>() / den)
}
pub fn stdev(x: &[f64], n: usize) -> Vec<f64> {
    compact(x, n, |w| { let m = w.iter().sum::<f64>() / n as f64; (w.iter().map(|v| (v - m).powi(2)).sum::<f64>() / n as f64).sqrt() })
}
pub fn dev(x: &[f64], n: usize) -> Vec<f64> {
    compact(x, n, |w| { let m = w.iter().sum::<f64>() / n as f64; w.iter().map(|v| (v - m).abs()).sum::<f64>() / n as f64 })
}
fn window_extreme(x: &[f64], n: usize, max: bool) -> Vec<f64> {
    let mut out = vec![NA; x.len()];
    let mut start = 0; // reset after a NaN (TradingView G6)
    for i in 0..x.len() {
        if x[i].is_nan() { start = i + 1; continue; }
        if i + 1 < n { continue; }
        let lo = start.max(i + 1 - n);
        let w = &x[lo..=i];
        out[i] = if max { w.iter().cloned().fold(f64::MIN, f64::max) } else { w.iter().cloned().fold(f64::MAX, f64::min) };
    }
    out
}
pub fn highest(x: &[f64], n: usize) -> Vec<f64> { window_extreme(x, n, true) }
pub fn lowest(x: &[f64], n: usize) -> Vec<f64> { window_extreme(x, n, false) }
pub fn change(x: &[f64], k: usize) -> Vec<f64> { (0..x.len()).map(|i| if i >= k { x[i] - x[i - k] } else { NA }).collect() }
pub fn tr(b: &Bars, handle_na: bool) -> Vec<f64> {
    (0..b.len()).map(|i| if i == 0 {
        if handle_na { b.high[0] - b.low[0] } else { NA }
    } else {
        let pc = b.close[i - 1];
        (b.high[i] - b.low[i]).max((b.high[i] - pc).abs()).max((b.low[i] - pc).abs())
    }).collect()
}
/// Least-squares value at offset `off` from the newest bar (k = 1..n oldest→newest).
pub fn linreg(x: &[f64], n: usize, off: usize) -> Vec<f64> {
    let mut out = vec![NA; x.len()];
    for i in n.saturating_sub(1)..x.len() {
        let w = &x[i + 1 - n..=i];
        if w.iter().any(|v| v.is_nan()) { continue; }
        let nf = n as f64;
        let (mut sk, mut sy, mut skk, mut sky) = (0., 0., 0., 0.);
        for (j, &y) in w.iter().enumerate() { let k = (j + 1) as f64; sk += k; sy += y; skk += k * k; sky += k * y; }
        let den = nf * skk - sk * sk;
        let slope = if den == 0. { 0. } else { (nf * sky - sk * sy) / den };
        let intercept = sy / nf - slope * sk / nf + slope;
        out[i] = intercept + slope * (nf - 1. - off as f64);
    }
    out
}
```

`lab/testdata.rs`:
```rust
use super::bars::Bars;
pub fn walk(n: usize, seed: u64, with_volume: bool) -> Bars {
    let mut s = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    let mut next = || { s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407); ((s >> 11) as f64) / ((1u64 << 53) as f64) };
    let mut b = Bars::default();
    let mut c = 100.0;
    for i in 0..n {
        let o = c * (1. + (next() - 0.5) * 0.01);
        let cl = o * (1. + (next() - 0.5) * 0.04);
        let h = o.max(cl) * (1. + next() * 0.01);
        let l = o.min(cl) * (1. - next() * 0.01);
        b.date.push(format!("2020-{:02}-{:02}", 1 + (i / 28) % 12, 1 + i % 28));
        b.open.push(o); b.high.push(h); b.low.push(l); b.close.push(cl); b.end.push(i);
        c = cl;
    }
    if with_volume { b.volume = Some((0..n).map(|_| 1000. + (next() * 1000.).round()).collect()); }
    b
}
pub fn from_ohlc(rows: &[(f64, f64, f64, f64)]) -> Bars {
    let mut b = Bars::default();
    for (i, &(o, h, l, c)) in rows.iter().enumerate() {
        b.date.push(format!("2024-01-{:02}", i + 1));
        b.open.push(o); b.high.push(h); b.low.push(l); b.close.push(c); b.end.push(i);
    }
    b
}
pub fn close_eq(a: f64, b: f64) -> bool {
    (a.is_nan() && b.is_nan()) || (a - b).abs() <= 1e-9 * a.abs().max(b.abs()).max(1.)
}
pub fn assert_series(got: &[f64], want: &[f64]) {
    assert_eq!(got.len(), want.len(), "length");
    for (i, (g, w)) in got.iter().zip(want).enumerate() {
        assert!(close_eq(*g, *w), "index {i}: got {g}, want {w}");
    }
}
```
(Synthetic dates in `walk` are labels only; calendar tests in Task 3 build real dates.)

- [ ] **Step 4: Run tests** — `CARGO_BUILD_JOBS=2 cargo test lab::` → Expected: all `lab::ta` tests pass; `cargo test` overall still passes (book tests unchanged).

- [ ] **Step 5: Guard + commit**

```bash
python3 scripts/check_book_untouched.py --base origin/Manan
git add Context/guru/backend/src/main.rs Context/guru/backend/src/lab
git commit -m "Add lab module scaffold with TradingView-style primitives"
```

---

### Task 3: Resampling and multi-timeframe expansion

**Files:**
- Create: `Context/guru/backend/src/lab/resample.rs` (+ `pub mod resample;` in `lab/mod.rs`)

**Interfaces:**
- Consumes: `Bars` (Task 2).
- Produces: `#[derive(Clone, Copy, Debug, PartialEq, Eq)] pub enum Tf { D, W, M }`, `impl Tf { pub fn parse(s: &str) -> Option<Tf> }`, `pub fn day_number(date: &str) -> i64` (days since 1970-01-01, proleptic Gregorian), `pub fn resample(d: &Bars, tf: Tf) -> Bars`, `pub fn expand(values: &[f64], htf: &Bars, n: usize) -> Vec<f64>`.

Rules: a week is Monday–Sunday keyed by the Monday's day number (works for exchanges and 7-day crypto); a month is `YYYY-MM`. A resampled bar's `date` is its **last** source bar's date, `end` its last source index; open = first open, high = max, low = min, close = last close, volume = sum (None stays None). `expand` assigns a higher-timeframe value from its `end` index until the next bar's `end` − 1: usable only after that bar's last daily close (no lookahead). The final, possibly partial, bar is assigned only from its own `end`.

- [ ] **Step 1: Write the failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::lab::testdata::assert_series;
    fn daily(dates: &[&str], closes: &[f64]) -> Bars {
        let mut b = Bars::default();
        for (i, (d, c)) in dates.iter().zip(closes).enumerate() {
            b.date.push(d.to_string()); b.open.push(*c); b.high.push(c + 1.); b.low.push(c - 1.); b.close.push(*c); b.end.push(i);
        }
        b
    }
    #[test] fn day_number_epoch() { assert_eq!(day_number("1970-01-01"), 0); assert_eq!(day_number("2024-03-01"), 19783); }
    #[test] fn weekly_groups_by_monday_and_dates_last_day() {
        // Fri 2024-01-05, Mon 08, Tue 09, Fri 12 (holiday-short week), Mon 15
        let d = daily(&["2024-01-05", "2024-01-08", "2024-01-09", "2024-01-12", "2024-01-15"], &[1., 2., 3., 4., 5.]);
        let w = resample(&d, Tf::W);
        assert_eq!(w.date, vec!["2024-01-05", "2024-01-12", "2024-01-15"]);
        assert_eq!(w.end, vec![0, 3, 4]);
        assert_eq!(w.open, vec![1., 2., 5.]); assert_eq!(w.close, vec![1., 4., 5.]);
        assert_eq!(w.high, vec![2., 5., 6.]); assert_eq!(w.low, vec![0., 1., 4.]);
    }
    #[test] fn monthly_groups() {
        let d = daily(&["2024-01-30", "2024-01-31", "2024-02-01"], &[1., 2., 3.]);
        assert_eq!(resample(&d, Tf::M).end, vec![1, 2]);
    }
    #[test] fn expand_never_looks_ahead() {
        let d = daily(&["2024-01-05", "2024-01-08", "2024-01-09", "2024-01-12", "2024-01-15"], &[1., 2., 3., 4., 5.]);
        let w = resample(&d, Tf::W);
        // weekly value of week 2 (close 4) is not visible on Mon 08 / Tue 09, only from Fri 12
        assert_series(&expand(&w.close, &w, d.len()), &[1., 1., 1., 4., 5.]);
    }
    #[test] fn volume_sums_and_none_stays_none() {
        let mut d = daily(&["2024-01-08", "2024-01-09"], &[1., 2.]);
        assert!(resample(&d, Tf::W).volume.is_none());
        d.volume = Some(vec![10., 5.]);
        assert_eq!(resample(&d, Tf::W).volume, Some(vec![15.]));
    }
}
```

- [ ] **Step 2: Run** `CARGO_BUILD_JOBS=2 cargo test lab::resample` → FAIL (missing).

- [ ] **Step 3: Implement**

```rust
//! Daily → weekly / monthly bars. Weeks run Monday–Sunday; a higher-timeframe value becomes usable on the
//! last daily bar of its period, after that bar's close (AmiBroker expandLast, TradingView expr[1]+lookahead).
use super::bars::Bars;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tf { D, W, M }
impl Tf {
    pub fn parse(s: &str) -> Option<Tf> { match s { "D" => Some(Tf::D), "W" => Some(Tf::W), "M" => Some(Tf::M), _ => None } }
}
pub fn day_number(date: &str) -> i64 {
    let (y, m, d): (i64, i64, i64) = (date[0..4].parse().unwrap_or(1970), date[5..7].parse().unwrap_or(1), date[8..10].parse().unwrap_or(1));
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (m + if m > 2 { -3 } else { 9 }) + 2) / 5 + d - 1;
    era * 146_097 + (yoe * 365 + yoe / 4 - yoe / 100 + doy) - 719_468
}
fn key(date: &str, tf: Tf) -> i64 {
    match tf {
        Tf::D => day_number(date),
        Tf::W => { let n = day_number(date); n - (n + 3).rem_euclid(7) } // 1970-01-01 was a Thursday
        Tf::M => date[0..4].parse::<i64>().unwrap_or(0) * 12 + date[5..7].parse::<i64>().unwrap_or(0),
    }
}
pub fn resample(d: &Bars, tf: Tf) -> Bars {
    if tf == Tf::D { let mut b = d.clone(); b.end = (0..d.len()).collect(); return b; }
    let mut out = Bars::default();
    let mut vol: Vec<f64> = Vec::new();
    let mut last: Option<i64> = None;
    for i in 0..d.len() {
        let k = key(&d.date[i], tf);
        let v = d.volume.as_ref().map(|v| v[i]).unwrap_or(0.);
        if last != Some(k) {
            out.date.push(d.date[i].clone()); out.open.push(d.open[i]); out.high.push(d.high[i]);
            out.low.push(d.low[i]); out.close.push(d.close[i]); out.end.push(i); vol.push(v); last = Some(k);
        } else {
            let j = out.len() - 1;
            out.date[j] = d.date[i].clone(); out.high[j] = out.high[j].max(d.high[i]); out.low[j] = out.low[j].min(d.low[i]);
            out.close[j] = d.close[i]; out.end[j] = i; vol[j] += v;
        }
    }
    out.volume = d.volume.as_ref().map(|_| vol);
    out
}
pub fn expand(values: &[f64], htf: &Bars, n: usize) -> Vec<f64> {
    let mut out = vec![f64::NAN; n];
    for w in 0..htf.len() {
        let to = if w + 1 < htf.len() { htf.end[w + 1] } else { n };
        for slot in out.iter_mut().take(to).skip(htf.end[w]) { *slot = values[w]; }
    }
    out
}
```

- [ ] **Step 4: Run** `CARGO_BUILD_JOBS=2 cargo test lab::` → PASS.
- [ ] **Step 5: Commit** — `git add Context/guru/backend/src/lab && git commit -m "Add lab resampling with no-lookahead expansion"`

---

### Task 4: Catalogue types, parameter validation and generic invariants

**Files:**
- Create: `Context/guru/backend/src/lab/catalogue.rs` (+ `pub mod catalogue;`)

**Interfaces:**
- Consumes: `Bars`, `testdata`.
- Produces:
```rust
pub type Params = std::collections::BTreeMap<String, f64>;
pub type Output = Result<Vec<Vec<f64>>, String>;
#[derive(Clone, Copy, Debug, PartialEq, Eq)] pub enum Placement { Price, Pane }
#[derive(Clone, Copy, Debug)] pub struct ParamSpec { pub key: &'static str, pub label: &'static str, pub default: f64, pub min: f64, pub max: f64, pub step: f64, pub integer: bool }
#[derive(Clone, Copy, Debug, Default)] pub struct Needs { pub volume: bool, pub intraday: bool }
#[derive(Clone, Copy)] pub struct IndicatorDef {
    pub id: &'static str, pub name: &'static str, pub category: &'static str,
    pub params: &'static [ParamSpec], pub outputs: &'static [&'static str], pub placement: Placement,
    pub needs: Needs, pub definition: &'static str,
    /// First non-NaN index of each output for default parameters on gap-free OHLCV (TradingView-verified where noted).
    pub first: &'static [usize],
    /// Outputs that are legitimately NaN after warm-up (e.g. pivots between pivots).
    pub sparse: bool,
    pub compute: Option<fn(&Bars, &Params) -> Output>,
}
pub fn all() -> Vec<IndicatorDef>;          // ma::DEFS ++ momentum::DEFS ++ trend::DEFS ++ volatility::DEFS ++ volume::DEFS ++ price::DEFS
pub fn get(id: &str) -> Option<IndicatorDef>;
pub fn resolve_params(def: &IndicatorDef, given: &Params) -> Result<Params, String>;
pub fn run(def: &IndicatorDef, bars: &Bars, given: &Params) -> Output; // resolves params, checks needs, calls compute
pub fn p(params: &Params, key: &str) -> f64;   // panics only on programmer error (key not declared)
pub fn n(params: &Params, key: &str) -> usize; // p(...) as usize
```
Each family file exposes `pub const DEFS: &[IndicatorDef]`.

- [ ] **Step 1: Write the failing tests** (in `catalogue.rs`)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::lab::testdata::{close_eq, walk};
    fn defaults(d: &IndicatorDef) -> Params { resolve_params(d, &Params::new()).unwrap() }

    #[test] fn ids_unique_and_params_sane() {
        let defs = all();
        let mut ids: Vec<_> = defs.iter().map(|d| d.id).collect(); ids.sort(); let len = ids.len(); ids.dedup();
        assert_eq!(ids.len(), len, "duplicate indicator id");
        for d in &defs {
            assert_eq!(d.first.len(), d.outputs.len(), "{}: first[] per output", d.id);
            for p in d.params {
                assert!(p.min <= p.default && p.default <= p.max, "{}.{} default out of range", d.id, p.key);
                assert!(!p.integer || p.default.fract() == 0., "{}.{} integer default", d.id, p.key);
            }
        }
    }
    #[test] fn resolve_params_rejects_bad_input() {
        let d = get("sma").unwrap();
        let mut g = Params::new();
        g.insert("length".into(), 0.); assert!(resolve_params(&d, &g).unwrap_err().contains("length"));
        g.insert("length".into(), 10.5); assert!(resolve_params(&d, &g).unwrap_err().contains("length"));
        g.insert("length".into(), 1e6); assert!(resolve_params(&d, &g).is_err());
        g.clear(); g.insert("bogus".into(), 1.); assert!(resolve_params(&d, &g).unwrap_err().contains("bogus"));
    }
    #[test] fn every_indicator_length_warmup_and_no_gaps() {
        let bars = walk(700, 7, true);
        for d in all().iter().filter(|d| d.compute.is_some()) {
            let out = run(d, &bars, &Params::new()).unwrap_or_else(|e| panic!("{}: {e}", d.id));
            assert_eq!(out.len(), d.outputs.len(), "{}: output count", d.id);
            for (k, s) in out.iter().enumerate() {
                assert_eq!(s.len(), bars.len(), "{}.{} length", d.id, d.outputs[k]);
                let first = s.iter().position(|v| !v.is_nan()).unwrap_or(usize::MAX);
                assert_eq!(first, d.first[k], "{}.{} first valid bar", d.id, d.outputs[k]);
                if !d.sparse { assert!(s[first..].iter().all(|v| v.is_finite()), "{}.{} gap or inf after warm-up", d.id, d.outputs[k]); }
            }
        }
    }
    #[test] fn every_indicator_is_causal() {
        let bars = walk(400, 11, true);
        for d in all().iter().filter(|d| d.compute.is_some()) {
            let full = run(d, &bars, &Params::new()).unwrap();
            for cut in [150usize, 271, 399] {
                let mut pre = bars.clone();
                for v in [&mut pre.open, &mut pre.high, &mut pre.low, &mut pre.close] { v.truncate(cut); }
                pre.date.truncate(cut); pre.end.truncate(cut);
                if let Some(v) = pre.volume.as_mut() { v.truncate(cut); }
                let part = run(d, &pre, &Params::new()).unwrap();
                for (k, s) in part.iter().enumerate() {
                    for i in 0..cut { assert!(close_eq(s[i], full[k][i]), "{}.{} changes at bar {i} when later bars exist", d.id, d.outputs[k]); }
                }
            }
        }
    }
    #[test] fn volume_indicators_refuse_without_volume() {
        let bars = walk(300, 3, false);
        for d in all().iter().filter(|d| d.needs.volume && d.compute.is_some()) {
            assert_eq!(run(d, &bars, &Params::new()).unwrap_err(), "needs volume", "{}", d.id);
        }
    }
    #[test] fn intraday_indicators_are_listed_not_computed() {
        for d in all().iter().filter(|d| d.needs.intraday) { assert!(d.compute.is_none(), "{}", d.id); }
        assert!(get("vwap").is_some() && get("opening_range").is_some());
    }
    #[test] fn flat_series_never_inf() {
        let mut bars = walk(300, 5, true);
        for i in 0..bars.len() { bars.open[i] = 50.; bars.high[i] = 50.; bars.low[i] = 50.; bars.close[i] = 50.; }
        for d in all().iter().filter(|d| d.compute.is_some()) {
            for s in run(d, &bars, &Params::new()).unwrap() { assert!(s.iter().all(|v| !v.is_infinite()), "{} produced inf on flat prices", d.id); }
        }
    }
    #[test] fn short_history_all_nan() {
        let bars = walk(3, 9, true);
        for d in all().iter().filter(|d| d.compute.is_some() && d.first.iter().all(|&f| f >= 3)) {
            for s in run(d, &bars, &Params::new()).unwrap() { assert!(s.iter().all(|v| v.is_nan()), "{}", d.id); }
        }
    }
    #[test] fn defaults_resolve_for_all() { for d in all() { defaults(&d); } }
}
```

- [ ] **Step 2: Run** `CARGO_BUILD_JOBS=2 cargo test lab::catalogue` → FAIL (missing).

- [ ] **Step 3: Implement**

```rust
//! Indicator catalogue: declared parameters, outputs, data needs and warm-up for every indicator.
use super::bars::Bars;
use std::collections::BTreeMap;

pub type Params = BTreeMap<String, f64>;
pub type Output = Result<Vec<Vec<f64>>, String>;
#[derive(Clone, Copy, Debug, PartialEq, Eq)] pub enum Placement { Price, Pane }
#[derive(Clone, Copy, Debug)]
pub struct ParamSpec { pub key: &'static str, pub label: &'static str, pub default: f64, pub min: f64, pub max: f64, pub step: f64, pub integer: bool }
#[derive(Clone, Copy, Debug, Default)]
pub struct Needs { pub volume: bool, pub intraday: bool }
#[derive(Clone, Copy)]
pub struct IndicatorDef {
    pub id: &'static str, pub name: &'static str, pub category: &'static str,
    pub params: &'static [ParamSpec], pub outputs: &'static [&'static str], pub placement: Placement,
    pub needs: Needs, pub definition: &'static str, pub first: &'static [usize], pub sparse: bool,
    pub compute: Option<fn(&Bars, &Params) -> Output>,
}
/// Integer length parameter helper used by the family files.
pub const fn len(key: &'static str, label: &'static str, default: f64) -> ParamSpec {
    ParamSpec { key, label, default, min: 1., max: 2000., step: 1., integer: true }
}
pub const fn real(key: &'static str, label: &'static str, default: f64, min: f64, max: f64, step: f64) -> ParamSpec {
    ParamSpec { key, label, default, min, max, step, integer: false }
}
pub fn all() -> Vec<IndicatorDef> {
    [super::ma::DEFS, super::momentum::DEFS, super::trend::DEFS, super::volatility::DEFS, super::volume::DEFS, super::price::DEFS].concat()
}
pub fn get(id: &str) -> Option<IndicatorDef> { all().into_iter().find(|d| d.id == id) }
pub fn resolve_params(def: &IndicatorDef, given: &Params) -> Result<Params, String> {
    if let Some(k) = given.keys().find(|k| !def.params.iter().any(|p| p.key == k.as_str())) {
        return Err(format!("{}: unknown parameter '{k}'", def.id));
    }
    let mut out = Params::new();
    for p in def.params {
        let v = *given.get(p.key).unwrap_or(&p.default);
        if !v.is_finite() || v < p.min || v > p.max { return Err(format!("{}: {} must be between {} and {}", def.id, p.key, p.min, p.max)); }
        if p.integer && v.fract() != 0. { return Err(format!("{}: {} must be a whole number", def.id, p.key)); }
        out.insert(p.key.to_string(), v);
    }
    Ok(out)
}
pub fn run(def: &IndicatorDef, bars: &Bars, given: &Params) -> Output {
    let params = resolve_params(def, given)?;
    if def.needs.intraday { return Err("needs intraday data".into()); }
    if def.needs.volume && bars.volume.is_none() { return Err("needs volume".into()); }
    let f = def.compute.ok_or_else(|| "computed by a later batch".to_string())?;
    f(bars, &params)
}
pub fn p(params: &Params, key: &str) -> f64 { *params.get(key).unwrap_or_else(|| panic!("parameter {key} not declared")) }
pub fn n(params: &Params, key: &str) -> usize { p(params, key) as usize }
```

Create empty family files so the crate compiles: each of `ma.rs, momentum.rs, trend.rs, volatility.rs, volume.rs, price.rs` starts as
```rust
use super::catalogue::IndicatorDef;
pub const DEFS: &[IndicatorDef] = &[];
```
and add their `pub mod` lines. `resolve_params_rejects_bad_input` and `intraday_indicators_are_listed_not_computed` stay failing until Tasks 5 and 9 — mark them `#[ignore = "enabled in Task 5/9"]` now and remove the `ignore` in those tasks.

- [ ] **Step 4: Run** `CARGO_BUILD_JOBS=2 cargo test lab::` → PASS (generic tests iterate an empty catalogue; two ignored).
- [ ] **Step 5: Commit** — `git commit -am "Add lab indicator catalogue with generic invariants"` (after `git add` of new files).

---

### Tasks 5–10: indicator families

Every family task follows the same five steps; the indicator table is the specification.

**Step 1 — failing tests** in the family file: (a) one hand-computed value test per indicator marked ✱ below (code given), (b) the generic catalogue tests now cover every new id automatically.
**Step 2 — run** `CARGO_BUILD_JOBS=2 cargo test lab::<family>` → FAIL.
**Step 3 — implement** each indicator as `fn <id>(b: &Bars, prm: &Params) -> Output` plus a `DEFS` entry with params, outputs, placement, `first` (from the table, defaults), `definition` (one line, the formula below).
**Step 4 — run** `CARGO_BUILD_JOBS=2 cargo test lab::` → PASS, and `python3 scripts/check_book_untouched.py` → no problems.
**Step 5 — commit** `git add Context/guru/backend/src/lab && git commit -m "Add lab <family> indicators"`.

Notation: `c,o,h,l,v` = close/open/high/low/volume; `n` = length; all MAs from `ta.rs`; "first" = first non-NaN index for default params.

#### Task 5: `ma.rs` — moving averages (remove the `ignore` on `resolve_params_rejects_bad_input`)

| id | params (default, range) | outputs | definition | first |
|---|---|---|---|---|
| sma ✱ | length 9 | sma | sma(c,n) | 8 |
| ema ✱ | length 9 | ema | ema(c,n), SMA seed | 8 |
| wma | length 9 | wma | wma(c,n) | 8 |
| smma | length 7 | smma | rma(c,n) | 6 |
| vwma (volume) | length 20 | vwma | sma(c·v,n)/sma(v,n) | 19 |
| hma ✱ | length 9 (2–2000) | hma | wma(2·wma(c,⌊n/2⌋) − wma(c,n), ⌊√n⌋) | 10 |
| alma | length 9, offset 0.85 (0–1, 0.05), sigma 6 (0.5–50, 0.5) | alma | m=offset·(n−1), s=n/σ, wᵢ=exp(−(i−m)²/(2s²)), i=0 oldest; Σwᵢ·x/Σwᵢ | 8 |
| lsma | length 25 | lsma | linreg(c,n,0) | 24 |
| dema | length 9 | dema | 2·e1 − ema(e1,n) | 16 |
| tema | length 9 | tema | 3(e1−e2)+e3 | 24 |
| kama | length 10, fast 2, slow 30 | kama | ER=|c−c[n]|/Σₙ|Δc| (0 if denom 0); α=(ER·(2/(fast+1)−2/(slow+1))+2/(slow+1))²; k=α·c+(1−α)·k[1], seed k=c at first bar where ER exists | 10 |
| mcginley | length 14 | mg | mg = na(mg[1]) ? ema(c,n) : mg[1] + (c−mg[1])/(n·(c/mg[1])⁴) | 13 |

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::lab::{catalogue::{run, get, Params}, testdata::{assert_series, from_ohlc}};
    const N: f64 = f64::NAN;
    fn closes(c: &[f64]) -> crate::lab::bars::Bars { from_ohlc(&c.iter().map(|&x| (x, x, x, x)).collect::<Vec<_>>()) }
    fn with(k: &str, v: f64) -> Params { let mut p = Params::new(); p.insert(k.into(), v); p }
    #[test] fn sma_len3() { assert_series(&run(&get("sma").unwrap(), &closes(&[1., 2., 3., 4.]), &with("length", 3.)).unwrap()[0], &[N, N, 2., 3.]); }
    #[test] fn ema_len2_sma_seed() { assert_series(&run(&get("ema").unwrap(), &closes(&[1., 2., 3.]), &with("length", 2.)).unwrap()[0], &[N, 1.5, 2.5]); }
    #[test] fn hma_len4_constant_input() { // any MA of a constant is the constant
        assert_series(&run(&get("hma").unwrap(), &closes(&[5.; 6]), &with("length", 4.)).unwrap()[0], &[N, N, N, N, 5., 5.]);
    }
}
```
(`hma` length 4: wma(…,2) on first value at index 3 → first 4.)

#### Task 6: `momentum.rs`

| id | params | outputs | definition | first |
|---|---|---|---|---|
| rsi ✱ | length 14 | rsi | up=rma(max(Δc,0)), dn=rma(max(−Δc,0)); dn==0→100; up==0→0; else 100−100/(1+up/dn) | 14 |
| macd | fast 12, slow 26, signal 9 | macd, signal, hist | ema(c,f)−ema(c,s); ema(macd,sig); macd−signal | 25, 33, 33 |
| stoch | k 14, smooth_k 1, d 3 | k, d | raw=100(c−ll)/(hh−ll) (NaN if hh==ll); k=sma(raw,smooth_k); d=sma(k,d) | 13, 15 |
| stochrsi | k 3, d 3, rsi_length 14, stoch_length 14 | k, d | r=rsi(c); k=sma(stoch(r,r,r,stoch_len),k) with highest/lowest window reset rule; d=sma(k,d) | 17, 19 (TradingView-verified K=17) |
| cci | length 20 | cci | (hlc3−sma)/(0.015·dev(hlc3,n)); dev 0 → NaN | 19 |
| mom | length 10 | mom | c−c[n] | 10 |
| roc | length 9 | roc | 100(c−c[n])/c[n] | 9 |
| willr | length 14 | willr | 100(c−hh)/(hh−ll), hh/ll of high/low; NaN if hh==ll | 13 |
| mfi (volume) | length 14 | mfi | src=hlc3; up=Σₙ v·(Δsrc≤0?0:src), dn=Σₙ v·(Δsrc≥0?0:src) with bar 0 counted in both (Δ NaN → both comparisons false); dn==0→100; 100−100/(1+up/dn) | 13 |
| tsi | long 25, short 13, signal 13 | tsi, signal | 100·ema(ema(Δc,long),short)/ema(ema(|Δc|,long),short); ema(tsi,signal) | 37, 49 |
| cmo | length 9 | cmo | S₁=Σ(Δc≥0?Δc:0), S₂=Σ(Δc≥0?0:−Δc); 100(S₁−S₂)/(S₁+S₂), 0/0 → NaN | 9 |
| uo | fast 7, mid 14, slow 28 | uo | bp=c−min(l,c[1]); tr=max(h,c[1])−min(l,c[1]); A_k=Σbp/Σtr; 100(4A₇+2A₁₄+A₂₈)/7; bar 0 NaN | 28 |
| ao | fast 5, slow 34 | ao | sma(hl2,5)−sma(hl2,34) | 33 |
| ppo | fast 12, slow 26, signal 9 | ppo, signal, hist | 100(ema f−ema s)/ema s; ema(ppo,sig); diff | 25, 33, 33 |
| trix | length 18 | trix | 10000·Δ(ema(ema(ema(ln c)))) | 52 |
| kst | roc1..4 10/15/20/30, sma1..4 10/10/10/15, signal 9 | kst, signal | Σ wⱼ·sma(roc(c,rⱼ),sⱼ), w=1..4; sma(kst,9) | 44, 52 |
| dpo | length 21 | dpo | c − sma(c,n)[⌊n/2+1⌋] (non-centered) | 31 |
| coppock | wma 10, long 14, short 11 | coppock | wma(roc(c,14)+roc(c,11),10) | 23 |
| fisher | length 9 | fisher, trigger | hl2 range over n; v=0.66((hl2−ll)/(hh−ll)−0.5)+0.67·nz(v[1]) clamped to ±0.999 when |v|>0.99 (clamped value stored); F=0.5ln((1+v)/(1−v))+0.5nz(F[1]); trigger=F[1] | 8, 9 |
| crsi | rsi 3, streak 2, rank 100 | crsi | avg(rsi(c,3), rsi(streak,2), percentrank(roc(c,1),100)); streak bar 0 = −1; percentrank = 100·#{prev n ≤ x}/n | 100 |

```rust
#[test] fn rsi_saturates_at_100_on_monotonic_rise() {
    let b = closes(&(1..=20).map(|x| x as f64).collect::<Vec<_>>());
    let r = &run(&get("rsi").unwrap(), &b, &with("length", 14.)).unwrap()[0];
    assert!(r[13].is_nan()); assert_eq!(r[14], 100.);
}
```
(`closes`/`with` helpers as in Task 5's test module.)

#### Task 7: `trend.rs`

| id | params | outputs | definition | first |
|---|---|---|---|---|
| dmi | di_length 14, adx_smoothing 14 | plus_di, minus_di, adx | up=Δh, dn=−Δl; +DM=(up>dn&&up>0)?up:0 (NaN at bar 0); tr=tr(false); +DI=100·rma(+DM)/rma(tr) held when NaN; ADX=100·rma(|+DI−−DI|/(sum==0?1:sum)) | 14, 14, 27 (TradingView-verified) |
| supertrend | atr_length 10, factor 3 (0.5–20, 0.5) | supertrend, direction | atr=rma(tr(true)); U/L bands from hl2 ± f·atr with carry rules (L:=L>L₁‖c[1]<L₁?L:L₁; U:=U<U₁‖c[1]>U₁?U:U₁); dir=first?1:(ST₁==U₁?(c>U?−1:1):(c<L?1:−1)); ST=dir==−1?L:U; output NaN where atr NaN (no bar-0 artefact) | 9, 9 |
| sar ✱ | start 0.02, increment 0.02, max 0.2 | sar | initial direction at bar 1: **long if h[1] > h[0]** (SAR=l[0], EP=h[1]) else short (SAR=h[0], EP=l[1]); each later bar SAR+=AF(EP−SAR); reversal on strict penetration (long: l<SAR → SAR=max(h,EP)? no: SAR=EP, EP=l, AF=start); new extreme updates EP and AF=min(AF+inc,max); clip long SAR ≤ min(l[1], l[2]), short ≥ max(h[1], h[2]) | 1 |
| aroon | length 14 | up, down | 100·(len − bars since highest high in len+1 bars)/len (ties → oldest); same for low | 14, 14 |
| aroon_osc | length 14 | osc | up − down | 14 |
| ichimoku | conversion 9, base 26, span_b 52, displacement 26 | tenkan, kijun, span_a, span_b | mid(n)=(highest(h,n)+lowest(l,n))/2; span_a=(tenkan+kijun)/2; values at the calculation bar; definition notes chart shift = displacement−1 = 25 (UI concern) | 8, 25, 25, 51 |
| vortex | length 14 | vi_plus, vi_minus | Σ|h−l[1]|/Σtr(true); Σ|l−h[1]|/Σtr(true) | 14, 14 |
| chop | length 14 | chop | 100·log10(Σtr(true)/(highest(h,n)−lowest(l,n)))/log10(n) | 13 |

SAR per bar i ≥ 2, exactly (TradingView reference algorithm, initial direction from highs as verified):
```
sar = sar + af * (ep - sar)
if long  and l[i] < sar  (strict): reversed; sar = max(h[i], ep); ep = l[i]; af = start; long = false
if short and h[i] > sar  (strict): reversed; sar = min(l[i], ep); ep = h[i]; af = start; long = true
if not reversed: if long and h[i] > ep { ep = h[i]; af = min(af + inc, max) }
                 if short and l[i] < ep { ep = l[i]; af = min(af + inc, max) }
clip (after the above): long  → sar = min(sar, l[i-1], l[i-2]);  short → sar = max(sar, h[i-1], h[i-2])
```
Bar 1 only initialises (sar = l[0] or h[0]; ep = h[1] or l[1]; af = start) and outputs that sar unchanged.
The verified fixture below is the arbiter.

```rust
#[test] fn sar_matches_tradingview_swiggy_first_bars() {
    // NSE:SWIGGY first four daily bars, ta.sar(0.02,0.02,0.2) read on TradingView 2026-10-14
    let b = from_ohlc(&[(420., 465.8, 391., 456.), (473.15, 489.4, 418.6, 430.7), (416.1, 438.7, 410.4, 420.1), (424., 430.9, 408.85, 412.45)]);
    assert_series(&run(&get("sar").unwrap(), &b, &Params::new()).unwrap()[0], &[f64::NAN, 391., 391., 392.968]);
}
```

#### Task 8: `volatility.rs`

| id | params | outputs | definition | first |
|---|---|---|---|---|
| atr ✱ | length 14 | atr | rma(tr(true),n) (bar-0 TR = h−l) | 13 (TradingView-verified) |
| bb | length 20, mult 2 (0.1–10, 0.1) | basis, upper, lower | sma(c,n) ± mult·stdev(c,n) population | 19, 19, 19 |
| bbpct | length 20, mult 2 | pct_b | (c−lower)/(upper−lower), fraction; zero width → NaN | 19 |
| bbw | length 20, mult 2 | width | 100(upper−lower)/basis | 19 |
| kc | length 20, mult 2, atr_length 10 | basis, upper, lower | ema(c,n) ± mult·rma(tr(true),atr_length) (chart built-in, not `ta.kc`) | 19 |
| donchian | length 20 | upper, lower, basis | highest(h), lowest(l), mean | 19 |
| stdev | length 20 | stdev | population stdev(c,n) | 19 |
| hv | length 10 | hv | 100·stdev(ln(c/c[1]),n)·√365 (TradingView daily annualisation) | 10 |

```rust
#[test] fn atr_len2_counts_bar0_range() {
    let b = from_ohlc(&[(10., 12., 9., 11.), (11., 13., 10.5, 12.), (12., 12.5, 11., 11.5)]);
    // tr = [3, 2.5, 1.5]; rma seed at index 1 = 2.75; then 2.75 + (1.5-2.75)/2 = 2.125
    assert_series(&run(&get("atr").unwrap(), &b, &with("length", 2.)).unwrap()[0], &[f64::NAN, 2.75, 2.125]);
}
```

#### Task 9: `volume.rs` (remove the `ignore` on `intraday_indicators_are_listed_not_computed`)

All except listed-only entries have `needs.volume = true`; volume NaN never occurs because `Bars.volume` is all-or-nothing.

| id | params | outputs | definition | first |
|---|---|---|---|---|
| obv ✱ | — | obv | cumulative sign(Δc)·v, bar 0 NaN | 1 |
| accdist | — | ad | cumulative ((h==l)‖(c==h&&c==l) ? 0 : (2c−l−h)/(h−l)·v) | 0 |
| cmf | length 20 | cmf | Σₙ mfv/Σₙ v (per-bar mfv as above) | 19 |
| chaikin_osc | fast 3, slow 10 | osc | ema(ad,3) − ema(ad,10) | 9 |
| pvt | — | pvt | cumulative Δc/c[1]·v, bar 0 NaN | 1 |
| pvo | fast 12, slow 26, signal 9 | pvo, signal, hist | 100(ema(v,f)−ema(v,s))/ema(v,s); ema(pvo,9); diff | 25, 33, 33 |
| efi | length 13 | efi | ema(Δc·v, n) | 13 |
| eom | length 14, divisor 10000 (1–1e9) | eom | sma(divisor·Δhl2·(h−l)/v, n); v==0 → NaN | 14 |
| nvi | ema 255 | nvi, signal | start 1000; v<v[1] (strict) → prev·(1+Δc/c[1]) else prev; ema(nvi,255) | 0, 254 |
| pvi | ema 255 | pvi, signal | same with v>v[1] | 0, 254 |
| vwap | listed only, `needs.intraday`, `compute: None` | vwap | session-anchored Σ(hlc3·v)/Σv; needs intraday bars | — |
| opening_range | listed only, `needs.intraday`, params minutes 15 (1–120) | high, low | high/low of the first N minutes of the session; needs intraday bars | — |

Listed-only defs use `first: &[usize::MAX]` per output (generic tests skip `compute: None`).

```rust
#[test] fn obv_starts_at_bar1() {
    let mut b = from_ohlc(&[(1., 1., 1., 1.), (2., 2., 2., 2.), (1.5, 1.5, 1.5, 1.5)]);
    b.volume = Some(vec![10., 20., 30.]);
    assert_series(&run(&get("obv").unwrap(), &b, &Params::new()).unwrap()[0], &[f64::NAN, 20., -10.]);
}
```

#### Task 10: `price.rs`

| id | params | outputs | definition | first | sparse |
|---|---|---|---|---|---|
| highest ✱ | length 20 | highest | highest(h,n) | 19 | no |
| lowest | length 20 | lowest | lowest(l,n) | 19 | no |
| change | length 1 | change | c − c[n] | 1 | no |
| pivot_high | left 5, right 5 | pivot | reported on bar t for pivot bar t−right; value h[t−right] if it is the max of the left+right+1 window (equal on left allowed, equal on right cancels) else NaN | first pivot (data-dependent) → declare `first` from the synthetic series in the test, `sparse: true` | yes |
| pivot_low | left 5, right 5 | pivot | mirror with lows | as above | yes |

For `pivot_high/low`, `first` cannot be a constant; set `first: &[usize::MAX]` and `sparse: true`, and change the generic warm-up test to skip `first == usize::MAX` entries **only for sparse defs** (edit the assertion: `if d.sparse { continue; }` before the `first` check). The causality test still applies (a pivot reported at t uses only bars ≤ t).

```rust
#[test] fn highest_len2() { assert_series(&run(&get("highest").unwrap(), &from_ohlc(&[(1.,3.,1.,1.),(1.,2.,1.,1.),(1.,5.,1.,1.)]), &with("length", 2.)).unwrap()[0], &[f64::NAN, 3., 5.]); }
#[test] fn pivot_high_reported_right_bars_late() {
    let hs = [1., 2., 5., 2., 1.];
    let b = from_ohlc(&hs.iter().map(|&h| (h, h, h, h)).collect::<Vec<_>>());
    let mut prm = Params::new(); prm.insert("left".into(), 2.); prm.insert("right".into(), 2.);
    assert_series(&run(&get("pivot_high").unwrap(), &b, &prm).unwrap()[0], &[f64::NAN, f64::NAN, f64::NAN, f64::NAN, 5.]);
}
```

---

### Task 11: TradingView parity fixtures (optional input, always-on harness)

**Files:**
- Create: `Context/guru/backend/src/lab/parity.rs` (+ `#[cfg(test)] pub mod parity;` in `lab/mod.rs`)
- Create: `docs/indicator-parity-fixtures.md`

**Interfaces:**
- Consumes: `catalogue::{get, run}`, `Bars`.
- Fixture format (kept outside Git, path in env `LAB_TV_FIXTURES`): one CSV per case named `<indicator_id>__<k=v,k=v>.csv` with header `time,open,high,low,close[,Volume],<output columns in DEFS order>`; `time` ISO date. Produced with TradingView's own "Export chart data" on a chart that has only that indicator with those parameters.

- [ ] **Step 1: Test**

```rust
#[test]
fn tradingview_exports_match_when_supplied() {
    let Ok(dir) = std::env::var("LAB_TV_FIXTURES") else { eprintln!("LAB_TV_FIXTURES not set; parity check skipped"); return; };
    let mut checked = 0;
    for entry in std::fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("csv") { continue; }
        let stem = path.file_stem().unwrap().to_str().unwrap().to_string();
        let (id, args) = stem.split_once("__").unwrap_or((&stem, ""));
        let def = get(id).unwrap_or_else(|| panic!("unknown indicator {id} in {}", path.display()));
        let mut prm = Params::new();
        for kv in args.split(',').filter(|s| !s.is_empty()) { let (k, v) = kv.split_once('=').unwrap(); prm.insert(k.into(), v.parse().unwrap()); }
        let text = std::fs::read_to_string(&path).unwrap();
        let mut lines = text.lines();
        let header: Vec<&str> = lines.next().unwrap().split(',').collect();
        let vol_col = header.iter().position(|h| h.eq_ignore_ascii_case("volume"));
        let first_out = 5 + vol_col.is_some() as usize;
        let rows: Vec<Vec<f64>> = lines.map(|l| l.split(',').skip(1).map(|x| x.trim().parse().unwrap_or(f64::NAN)).collect()).collect();
        let mut b = Bars::default();
        for (i, r) in rows.iter().enumerate() { b.date.push(format!("{i}")); b.open.push(r[0]); b.high.push(r[1]); b.low.push(r[2]); b.close.push(r[3]); b.end.push(i); }
        if let Some(vc) = vol_col { b.volume = Some(rows.iter().map(|r| r[vc - 1]).collect()); }
        let got = run(&def, &b, &prm).unwrap();
        for (k, series) in got.iter().enumerate() {
            for (i, r) in rows.iter().enumerate() {
                let want = r[first_out - 1 + k];
                let ok = (want.is_nan() && series[i].is_nan()) || (want - series[i]).abs() <= 1e-6 * want.abs().max(1.) ;
                assert!(ok, "{}: output {} bar {i}: TradingView {want}, lab {}", path.display(), def.outputs[k], series[i]);
            }
        }
        checked += 1;
    }
    eprintln!("checked {checked} TradingView fixtures");
}
```
(Tolerance 1e-6 relative because exported values are rounded to display precision.)

- [ ] **Step 2–4:** Run with and without `LAB_TV_FIXTURES`; without → passes with "skipped" message.
- [ ] **Step 5:** Write `docs/indicator-parity-fixtures.md` (how to export, naming, where to keep them outside Git, which indicators still need a fixture: Keltner, TSI, TRIX, DPO, Fisher, KST, Aroon ties, Ichimoku, Connors RSI, MFI, UO). Commit.

---

### Task 12: Phase gate

- [ ] **Step 1:** `cd Context/guru/backend && CARGO_BUILD_JOBS=2 cargo test` → all tests pass, including every pre-existing book test (count must be ≥ the count on `origin/Manan`; record both numbers).
- [ ] **Step 2:** `python3 -m unittest discover -s scripts -p 'test_*.py'` and `python3 scripts/check_upload.py` → pass.
- [ ] **Step 3:** `python3 scripts/check_book_untouched.py --base origin/Manan --engine-compare --csv <NAS>/idx_data.csv --vix <NAS>/india_vix --extra <NAS>/extra_data` (read-only NAS paths) → `"problems": []`.
- [ ] **Step 4:** `cargo build --release` warnings: none new in `lab/` (dead-code allowed only via `#[allow(dead_code)]` on `lab` until routes exist in Phase 2 — add `#![allow(dead_code)]` at the top of `lab/mod.rs`).
- [ ] **Step 5:** Commit any gate fixes; write a short phase report (tests count before/after, indicators added, fixtures still pending) in the PR description draft. Do not push without the user's OK.
