#!/usr/bin/env python3
"""Fail when a branch changes book strategies or their results (indicators lab spec §2a).

Checks (always): no edit to protected paths; only added lines in additive-only files.
Optional --engine-compare: build base and HEAD engines and compare hashes of book catalogue, params, summary,
run and readiness (every strategy, NIFTY 50, default parameters) and one small optimize grid per guru.
"""
import argparse, hashlib, io, json, os, socket, subprocess, sys, tarfile, tempfile, time, urllib.error, urllib.parse, urllib.request
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


def compare_base(repo, base):
    # Branch point: newer commits on base must not count as changes on this branch.
    return git(repo, "merge-base", base, "HEAD").strip()


def preloaded_tickers(tickers_json):
    # Basket rows are not single preloaded tickers, so they get no summary hash.
    return [row["ticker"] for row in tickers_json if "basket" not in row]


def check(repo, base):
    mb = compare_base(repo, base)
    # --no-renames: a rename out of a protected path must show the protected source path as deleted.
    # ls-files --others: untracked new files under protected paths count as changes too.
    changed = [p for p in git(repo, "diff", "--no-renames", "--name-only", mb).splitlines() if p]
    changed += [p for p in git(repo, "ls-files", "--others", "--exclude-standard").splitlines() if p]
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


def compare_engines(repo, base, csv, vix, extra):
    # Plain export of the base commit into a temp folder (no git worktree, per user preference).
    with tempfile.TemporaryDirectory() as tmp:
        archive = subprocess.run(["git", "-C", str(repo), "archive", compare_base(repo, base)], capture_output=True, check=True).stdout
        with tarfile.open(fileobj=io.BytesIO(archive)) as tar:
            tar.extractall(tmp, filter="data")
        before, _ = engine_hashes(tmp, csv, vix, extra)
    after, notes = engine_hashes(repo, csv, vix, extra)
    return [f"book output changed: {k}" for k in before if before[k] != after.get(k)], notes + [f"hashed {len(before)} book responses"]


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


if __name__ == "__main__":
    main()
