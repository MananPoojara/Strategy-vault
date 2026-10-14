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
