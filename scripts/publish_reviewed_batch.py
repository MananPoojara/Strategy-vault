#!/usr/bin/env python3
"""Publish one independently reviewed strategy commit, or retain a held report.

This deterministic release step never decides book rules. It validates hashes,
real ledgers, exact files and methods, tests the candidate, and changes only the
Manan branch and (when explicitly requested) the two strategy engine services.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import re
import subprocess
import sys
import time
import urllib.error
import urllib.parse
import urllib.request
import uuid


class Held(ValueError):
    pass


def require(condition, message):
    if not condition:
        raise Held(message)


def sha256(path):
    hasher = hashlib.sha256()
    with Path(path).open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            hasher.update(chunk)
    return hasher.hexdigest()


def git(repo, *args):
    process = subprocess.run(["git", "-C", str(repo), *args], stdout=subprocess.PIPE,
                             stderr=subprocess.PIPE, text=True, timeout=60)
    require(process.returncode == 0, "Git precondition failed; inspect the private release log")
    return process.stdout.strip()


BOOK_HELPERS = {
    "Context/guru/backend/src/books/hbqts/reviewed.rs": "Context/guru/backend/src/books/hbqts.rs",
}


def allowed_file(name):
    path = PurePosixPath(name)
    if path.is_absolute() or ".." in path.parts:
        return False
    if re.fullmatch(r"Context/guru/backend/src/books/[a-zA-Z0-9_]+\.rs", name):
        return True
    if name in BOOK_HELPERS:
        return True
    if name in {"Context/guru/backend/src/instrument_policy.rs", "Context/guru/backend/src/readiness.rs",
                "Context/guru/backend/src/gate.rs", "Context/guru/backend/src/main.rs"}:
        return True
    if name.startswith("docs/") and path.suffix == ".md":
        return True
    if name.startswith("library/") and path.suffix in {".py", ".rs", ".js", ".mjs", ".md"}:
        return not any(part in {"data", "Books", "node_modules", "target"} for part in path.parts)
    return False


def validate_support_source(name, before, after, approved):
    """Permit gate records and main's tests without widening production edits."""
    if name not in {"Context/guru/backend/src/gate.rs", "Context/guru/backend/src/main.rs"}:
        return
    marker = "\n#[cfg(test)]\nmod tests {"
    require(before.count(marker) == after.count(marker) == 1,
            "Supporting source must retain its identifiable test module")
    old, new = before.split(marker, 1)[0], after.split(marker, 1)[0]
    if name.endswith("/main.rs"):
        require(old == new, "Strategy publication cannot change main production code")
        return
    declaration = "static GATE: &[G] = &[\n"
    require(old.count(declaration) == new.count(declaration) == 1,
            "Gate metadata declaration changed")
    old_prefix, old_rest = old.split(declaration, 1)
    new_prefix, new_rest = new.split(declaration, 1)
    require("\n];" in old_rest and "\n];" in new_rest, "Gate metadata terminator missing")
    old_body, old_suffix = old_rest.split("\n];", 1)
    new_body, new_suffix = new_rest.split("\n];", 1)
    require(old_prefix == new_prefix and old_suffix == new_suffix,
            "Strategy publication cannot change gate production helpers")
    def entries(body):
        result = {}
        for line in body.splitlines():
            if not line.strip():
                continue
            match = re.fullmatch(r'\s*G \{ id: r(#+)"([a-z][a-z0-9_]*)"\1, .+ \},', line)
            require(match is not None, "Gate contains a nonmetadata entry")
            key = match.group(2)
            require(key not in result, "Gate contains duplicate entries")
            result[key] = line
        return result
    previous, current = entries(old_body), entries(new_body)
    require(set(previous) <= set(current), "Existing gate metadata was removed")
    changed = {key for key in current if current[key] != previous.get(key)}
    require(changed <= set(approved), "Candidate changed unapproved gate metadata")


def candidate_source(candidate, name):
    relative = PurePosixPath(name)
    require(not relative.is_absolute() and ".." not in relative.parts,
            "Candidate source must remain inside the isolated repository")
    root = Path(candidate).resolve()
    path = root
    for component in relative.parts:
        path /= component
        require(not path.is_symlink(), "Candidate source must be a regular file, not a symlink")
    require(path.is_file() and path.resolve().is_relative_to(root),
            "Candidate source must be a regular file inside the isolated repository")
    return path


def validate_review(review, candidate):
    require(review.get("schema_version") == 1, "Unsupported review schema")
    require(review.get("reviewer_id") and review.get("conversion_session_id") and
            review["reviewer_id"] != review["conversion_session_id"], "Independent reviewer and conversion job must differ")
    for flag in ("source_reviewed", "implementation_ready", "ledger_checked"):
        require(review.get(flag) is True, "Review has not passed " + flag)
    require(type(review.get("checked_trades")) is int and review["checked_trades"] > 0, "Review requires actual checked trades")
    require(type(review.get("row_mismatches")) is int and review["row_mismatches"] == 0, "Review has trade mismatches")
    methods = review.get("method_ids")
    require(isinstance(methods, list) and 0 < len(methods) <= 100 and len(set(methods)) == len(methods) and
            all(isinstance(item, str) and re.fullmatch(r"[a-z][a-z0-9_]{0,95}", item) for item in methods), "Review requires exact bounded method IDs")
    evidence = review.get("evidence")
    require(isinstance(evidence, list) and evidence and all(Path(item).is_absolute() and Path(item).is_file() for item in evidence), "Review evidence is missing")
    sources = review.get("source_hashes")
    require(isinstance(sources, dict) and sources, "Reviewed source hashes are missing")
    for name, digest in sources.items():
        require(Path(name).is_absolute() and Path(name).is_file() and sha256(name) == digest, "Reviewed source hash changed")
    hashes = review.get("candidate_hashes")
    require(isinstance(hashes, dict) and hashes, "Reviewed candidate hashes are missing")
    for name, digest in hashes.items():
        require(allowed_file(name) and sha256(candidate_source(candidate, name)) == digest, "Reviewed candidate hash changed or path is prohibited")
    reports = review.get("ledger_reports")
    require(isinstance(reports, list) and reports, "Independent ledger reports are required")
    checked, covered, all_compiled_modules = 0, set(), set()
    for entry in reports:
        path = Path(entry.get("path", ""))
        require(path.is_absolute() and path.is_file() and sha256(path) == entry.get("sha256"), "Independent ledger report hash changed")
        ledger = json.loads(path.read_text())
        require(all(ledger.get(flag) is True for flag in ("source_reviewed", "implementation_ready", "ledger_checked")), "Independent ledger has not passed")
        checks = ledger.get("checks")
        require(isinstance(checks, list) and checks, "Independent ledger has no per-method checks")
        modules = ledger.get("rust_module_sha256")
        require(isinstance(modules, dict) and modules, "Independent ledger lacks candidate Rust source hashes")
        covered_modules = set()
        for check in checks:
            require(check.get("method") in methods and type(check.get("row_mismatches")) is int and check["row_mismatches"] == 0, "Independent ledger has mismatches or unreviewed methods")
            require(type(check.get("actual_records")) is int and check["actual_records"] >= 0, "Independent ledger lacks actual row counts")
            require(type(check.get("expected_records")) is int and check["expected_records"] == check["actual_records"],
                    "Independent ledger expected and actual row counts differ")
            ohlc = Path(check.get("ohlc_path", ""))
            require(ohlc.is_absolute() and ohlc.is_file() and sha256(ohlc) == check.get("ohlc_sha256"),
                    "Independent ledger actual market-data hash changed")
            checked += check["actual_records"]
            covered.add(check["method"])
        for name, digest in modules.items():
            path_parts = Path(name).parts
            require("Context" in path_parts, "Ledger Rust source has no identifiable repository path")
            relative = Path(*path_parts[path_parts.index("Context"):])
            require(sha256(candidate_source(candidate, relative.as_posix())) == digest,
                    "Independent ledger was checked against different candidate Rust source")
            covered_modules.add(relative.as_posix())
        touched_books = {name for name in review.get("approved_files", []) if "/src/books/" in name and name.endswith(".rs")}
        require(covered_modules & touched_books, "Independent ledger has no reviewed candidate book module")
        all_compiled_modules.update(covered_modules)
    touched_books = {name for name in review.get("approved_files", []) if "/src/books/" in name and name.endswith(".rs")}
    require(touched_books <= all_compiled_modules,
            "Independent ledger must hash every changed book module, including nested helpers")
    require(covered == set(methods) and checked == review["checked_trades"], "Independent ledger method coverage or trade count differs from review")
    focused = review.get("focused_tests")
    require(isinstance(focused, list) and focused and all(isinstance(name, str) and re.fullmatch(r"[a-zA-Z0-9_:]+", name) for name in focused), "Focused Rust tests are required")
    cases = review.get("smoke_cases")
    require(isinstance(cases, list) and 0 < len(cases) <= 500, "Actual-instrument API cases are required")
    ready = set()
    for case in cases:
        require(case.get("method_id") in methods and case.get("guru") in {"connors", "ross", "chande"} and
                case.get("market") in {"india", "us"}, "Smoke case has an unapproved method or market")
        require(case.get("expected_readiness") in {"ready", "needs_data", "not_testable", "calculation_unavailable"} and
                case.get("instrument_kind") in {"index", "etf", "individual_stock"}, "Smoke case must state instrument compatibility")
        require(isinstance(case.get("query"), dict) and case["query"].get("ticker") and
                all(isinstance(k, str) and isinstance(v, str) for k, v in case["query"].items()), "Smoke case requires an exact actual ticker and string query")
        if case["expected_readiness"] == "ready":
            ready.add(case["method_id"])
    require(ready == set(methods), "Every reviewed method needs a ready actual-instrument API case")


def validate_candidate(candidate, repo, review):
    candidate, repo = Path(candidate).resolve(), Path(repo).resolve()
    require(candidate != repo, "Candidate must be isolated from the authoritative checkout")
    require(git(repo, "branch", "--show-current") == "Manan", "Authoritative checkout must be on Manan")
    require(not git(repo, "status", "--porcelain") and not git(candidate, "status", "--porcelain"), "Candidate and authoritative checkout must be clean")
    base = git(repo, "rev-parse", "HEAD")
    commit = git(candidate, "rev-parse", "HEAD")
    require(base == review.get("base_commit"), "Reviewed base is stale; rebase and independently review a new candidate")
    require(commit == review.get("candidate_commit"), "Candidate commit differs from independent review")
    parents = git(candidate, "rev-list", "--parents", "-n", "1", commit).split()
    require(parents == [commit, base], "Candidate must contain one commit with the exact current base parent")
    records = git(candidate, "diff", "--name-status", "--no-renames", base, commit).splitlines()
    changed = []
    for record in records:
        status, name = record.split("\t", 1)
        require(status in {"A", "M"} and allowed_file(name), "Candidate changes a prohibited file or deletes existing source")
        mode = git(candidate, "ls-tree", commit, "--", name).split()[0]
        require(mode in {"100644", "100755"}, "Candidate changes must contain regular source files")
        candidate_source(candidate, name)
        if name in {"Context/guru/backend/src/gate.rs", "Context/guru/backend/src/main.rs"}:
            validate_support_source(name, git(candidate, "show", base + ":" + name),
                                    candidate_source(candidate, name).read_text(), review.get("method_ids", []))
        changed.append(name)
    require(changed and set(changed) == set(review.get("approved_files", [])) == set(review.get("candidate_hashes", {})), "Review must name the exact changed file set")
    validate_review(review, candidate)
    return changed


def check_catalogue_changes(before, after, approved):
    old, new = {item["id"]: item for item in before}, {item["id"]: item for item in after}
    changed = set()
    for method, item in old.items():
        require(method in new or item.get("status") != "live", "An existing live method was removed")
        if item.get("status") == "live":
            require(new.get(method, {}).get("status") == "live", "An existing live method was removed from admission")
    for method, item in new.items():
        previous = old.get(method, {})
        p, q = previous.get("instrument_policy", {}), item.get("instrument_policy", {})
        old_types, new_types = set(p.get("allowed_types", [])), set(q.get("allowed_types", []))
        require(method in approved or old_types == new_types, "Candidate changed an unapproved method's allowed instrument types")
        widened = new_types - old_types
        newly_live = item.get("status") == "live" and previous.get("status") != "live"
        newly_reviewed = q.get("source_rule_reviewed") is True and p.get("source_rule_reviewed") is not True
        if widened or newly_live or newly_reviewed:
            require(method in approved, "Candidate enabled an unapproved method or widened its instrument policy")
            changed.add(method)
    return changed


def admission_counts(before, after):
    """Directions/thresholds remain variants; book/family pairs define families."""
    old = {item["id"]: item for item in before if item.get("status") == "live"}
    new = {item["id"]: item for item in after if item.get("status") == "live"}
    key = lambda item: (item.get("book", ""), item.get("family", item["id"]))
    families = {key(item) for item in new.values()} - {key(item) for item in old.values()}
    variants = sorted(set(new) - set(old))
    return {"admitted_variant_ids": variants, "admitted_variant_count": len(variants),
            "admitted_family_keys": [{"book": book, "family": family} for book, family in sorted(families)],
            "admitted_family_count": len(families), "admitted_method_count": len(families)}


class Commands:
    def __init__(self, directory):
        self.directory = Path(directory)
        self.directory.mkdir(parents=True, exist_ok=True, mode=0o700)
        self.index = 0

    def run(self, argv, *, cwd=None, env=None, timeout=600, stage="command"):
        self.index += 1
        logfile = self.directory / (str(self.index).zfill(3) + "-" + stage + ".log")
        with logfile.open("wb") as handle:
            os.chmod(logfile, 0o600)
            process = subprocess.run(argv, cwd=cwd, env=env, stdin=subprocess.DEVNULL,
                                     stdout=handle, stderr=subprocess.STDOUT, timeout=timeout)
        require(process.returncode == 0, stage + " failed; inspect the private command log")
        require(logfile.stat().st_size <= 20 * 1024 * 1024, stage + " produced an oversized log")
        return logfile.read_text(errors="replace").strip()


def get_json(base, path, query=None):
    url = base + path + ("?" + urllib.parse.urlencode(query) if query else "")
    try:
        with urllib.request.urlopen(url, timeout=45) as response:
            return response.status, json.load(response)
    except urllib.error.HTTPError as error:
        return error.code, error.read(8000).decode(errors="replace")


def wait_api(base):
    deadline = time.monotonic() + 60
    while time.monotonic() < deadline:
        try:
            code, data = get_json(base, "/api/tickers")
            if code == 200 and isinstance(data, list) and data:
                return
        except (OSError, ValueError):
            pass
        time.sleep(1)
    raise Held("Engine API did not load the existing actual data")


def catalogue(base):
    output = []
    for guru in ("connors", "ross", "chande"):
        code, rows = get_json(base, "/api/guru/" + guru + "/strategies")
        require(code == 200 and isinstance(rows, list), "Engine catalogue is unavailable")
        output.extend({**row, "_publisher_guru": guru} for row in rows)
    return output


def touched_book_ids(candidate, review):
    """Use both versions' BOOK constants so an accidental identity edit cannot hide regression targets."""
    books = set()
    pattern = r'\bconst\s+BOOK\s*:\s*&(?:\x27static\s+)?str\s*=\s*"([a-zA-Z0-9_-]+)"'
    for name in review["approved_files"]:
        if "/backend/src/books/" not in name:
            continue
        name = BOOK_HELPERS.get(name, name)
        path = Path(candidate) / name
        texts = [path.read_text()]
        if git(candidate, "ls-tree", "--name-only", review["base_commit"], "--", name):
            texts.append(git(candidate, "show", review["base_commit"] + ":" + name))
        found = set().union(*(set(re.findall(pattern, text)) for text in texts))
        require(found, "Changed book helper has no explicit BOOK constant; define an explicit wider regression review")
        books.update(found)
    return books


def stable_run_hash(payload):
    require(isinstance(payload, dict) and all(name in payload for name in ("dates", "trades", "equity", "stats", "gross_equity", "gross_stats")),
            "Existing-method regression backtest lacks stable calculation fields")
    fields = ("dates", "close", "trades", "equity", "stats", "gross_equity", "gross_stats", "bh",
              "benchmark_symbol", "benchmark_stats", "costs", "model_initial_equity", "ohlc", "accounting_basis")
    stable = {name: payload[name] for name in fields if name in payload}
    return hashlib.sha256(json.dumps(stable, sort_keys=True, separators=(",", ":"), allow_nan=False).encode()).hexdigest()


def compare_existing_runs(baseline_base, candidate_base, baseline_rows, cases, touched_books, approved_ids, *, max_cases=200):
    """Compare old live methods in touched books on deduplicated actual instrument windows."""
    contexts = {}
    for case in cases:
        query = {key: case["query"][key] for key in ("ticker", "from", "to") if key in case["query"]}
        key = (case["market"], query["ticker"], query.get("from"), query.get("to"))
        contexts.setdefault(key, query)
    previous = {row["id"]: row for row in baseline_rows if row.get("status") == "live" and
                row.get("book") in touched_books and row["id"] not in approved_ids}
    require(len(previous) * len(contexts) <= max_cases, "Existing-method regression exceeds its bounded case limit; split the reviewed batch")
    checks, withheld_checks, skipped = [], [], 0
    for method, row in sorted(previous.items()):
        guru = row.get("_publisher_guru", "connors")
        route = "/api/guru/" + guru
        for context in contexts.values():
            query = {**context, "id": method, "cost": "0", "slippage": "0", "rollover": "0"}
            code, ready = get_json(baseline_base, route + "/readiness", query)
            require(code == 200 and isinstance(ready, dict), "Existing-method baseline readiness failed")
            status = {item["id"]: item["status"] for item in ready.get("strategies", [])}.get(method)
            require(status is not None, "Existing-method baseline readiness omitted the exact method")
            if status != "ready":
                skipped += 1
                continue
            code, candidate_ready = get_json(candidate_base, route + "/readiness", query)
            actual = {item["id"]: item["status"] for item in candidate_ready.get("strategies", [])}.get(method) if isinstance(candidate_ready, dict) else None
            require(code == 200 and actual == "ready", "An unapproved existing method lost actual-instrument readiness")
            before_code, before = get_json(baseline_base, route + "/run", query)
            after_code, after = get_json(candidate_base, route + "/run", query)
            if before_code == 400 and isinstance(before, str) and before.startswith("calculation_unavailable:"):
                require(after_code == before_code and after == before,
                        "An unapproved existing method's calculation guard changed")
                error_hash = hashlib.sha256(before.encode()).hexdigest()
                withheld_checks.append({"method_id": method, "ticker": context["ticker"],
                                       "from": context.get("from"), "to": context.get("to"),
                                       "http_status": before_code, "baseline_sha256": error_hash,
                                       "candidate_sha256": error_hash})
                continue
            require(before_code == after_code == 200, "Existing-method actual-data backtest failed")
            before_hash, after_hash = stable_run_hash(before), stable_run_hash(after)
            require(before_hash == after_hash, "An unapproved existing method's trades, statistics or equity changed")
            checks.append({"method_id": method, "ticker": context["ticker"], "from": context.get("from"), "to": context.get("to"),
                           "trade_records": len(before["trades"]), "baseline_sha256": before_hash, "candidate_sha256": after_hash})
    return {"checked_cases": len(checks), "checked_withheld_cases": len(withheld_checks),
            "withheld_checks": withheld_checks, "skipped_not_ready": skipped, "context_count": len(contexts),
            "previous_live_methods": len(previous), "planned_cases": len(previous) * len(contexts), "checks": checks}


def smoke(base, cases, catalogue_rows):
    by_id = {item["id"]: item for item in catalogue_rows}
    results = []
    for case in cases:
        query = {"cost": "0", "slippage": "0", "rollover": "0", **case["query"], "id": case["method_id"]}
        route = "/api/guru/" + case["guru"]
        code, response = get_json(base, route + "/readiness", query)
        require(code == 200 and isinstance(response, dict), "Actual-instrument readiness request failed")
        states = {item["id"]: item["status"] for item in response.get("strategies", [])}
        actual = states.get(case["method_id"])
        require(actual == case["expected_readiness"], "Actual-instrument readiness differs from reviewed expectation")
        code, run = get_json(base, route + "/run", query)
        if actual == "ready":
            policy = by_id[case["method_id"]].get("instrument_policy", {})
            require(case["instrument_kind"] in policy.get("allowed_types", []), "Reviewed instrument class is not admitted by the method policy")
            require(code == 200 and isinstance(run, dict) and run.get("dates") and isinstance(run.get("trades"), list), "A ready method did not produce an actual-price backtest")
            require(len(run.get("equity", [])) == len(run["dates"]), "Actual-price backtest has inconsistent equity dates")
            results.append({"method_id": case["method_id"], "market": case["market"], "ticker": case["query"]["ticker"], "readiness": actual, "query": query, "trade_records": len(run["trades"])})
        else:
            require(code >= 400, "An incompatible instrument unexpectedly produced a backtest")
            results.append({"method_id": case["method_id"], "market": case["market"], "ticker": case["query"]["ticker"], "readiness": actual, "query": query, "blocked": True})
    return results


def runtime_settings(path):
    settings = {}
    for line in Path(path).read_text().splitlines():
        if "=" in line and not line.lstrip().startswith("#"):
            key, value = line.split("=", 1)
            settings[key.strip()] = value.strip().strip("\"'")
    require(settings.get("VAULT_RUNTIME_ROOT") and Path(settings["VAULT_RUNTIME_ROOT"]).is_dir(), "Existing private runtime data is unavailable")
    return settings


def compose_prefix(repo, runtime_env):
    return ["docker", "compose", "-f", str(Path(repo) / "Context/guru/webapp/compose.yaml"), "--env-file", str(runtime_env)]


def engine_ids(commands, prefix):
    return {market: commands.run(prefix + ["ps", "-q", service], stage="locate-" + market)
            for market, service in (("india", "engine-in"), ("us", "engine-us"))}


def live_base(commands, container_id):
    address = commands.run(["docker", "inspect", "--format", "{{range .NetworkSettings.Networks}}{{.IPAddress}}{{end}}", container_id], stage="engine-address")
    require(re.fullmatch(r"[0-9.]+", address), "Engine does not have one inspectable local network address")
    return "http://" + address + ":8090"


def temporary_engine(commands, image, market, settings, name):
    runtime = Path(settings["VAULT_RUNTIME_ROOT"])
    basket = settings.get("VAULT_BASKET_DATA_ROOT", "/home/server/.cache/strategy-vault/market-data") if market == "india" else settings.get("VAULT_US_BASKET_DATA_ROOT", "/home/server/.cache/strategy-vault/us-market-data")
    csv = "idx_data.csv" if market == "india" else "us_data.csv"
    vix = "india_vix" if market == "india" else "us_vix"
    require((runtime / "data" / csv).is_file() and Path(basket).is_dir(), "Actual market or basket data is missing")
    commands.run(["docker", "run", "-d", "--name", name, "--cpus", "1", "--memory", "768m", "--memory-swap", "768m", "--pids-limit", "128", "--read-only", "-p", "127.0.0.1::8090", "-e", "VAULT_BASKET_DATA_DIR=/basket-data", "-v", str(runtime / "data") + ":/data:ro", "-v", str(Path(basket).resolve()) + ":/basket-data:ro", image, "/data/" + csv, "8090", "/data/" + vix, "/data/extra_data/" + market], stage="candidate-start")
    port = commands.run(["docker", "inspect", "--format", '{{(index (index .NetworkSettings.Ports "8090/tcp") 0).HostPort}}', name], stage="candidate-port")
    require(port.isdigit(), "Candidate engine did not expose a local validation port")
    base = "http://127.0.0.1:" + port
    wait_api(base)
    return base


def write_result(path, report):
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True, mode=0o700)
    temporary = path.with_name(path.name + ".tmp")
    with temporary.open("w") as handle:
        os.chmod(temporary, 0o600)
        json.dump(report, handle, indent=2, allow_nan=False)
        handle.write("\n")
        handle.flush()
        os.fsync(handle.fileno())
    os.replace(temporary, path)


def publish(candidate, repo, review_path, result_path, runtime_env, *, deploy=False):
    candidate, repo = Path(candidate).resolve(), Path(repo).resolve()
    result_path = Path(result_path).resolve()
    review = {}
    report = {"schema_version": 1, "source_reviewed": False, "implementation_ready": False,
              "ledger_checked": False, "checked_trades": 0, "admitted": False, "source_uploaded": False,
              "deployed": False, "status": "held", "evidence": [], "admitted_method_ids": [], "admitted_method_count": 0}
    report.update(admitted_family_count=0, admitted_variant_count=0)
    commands = Commands(result_path.parent / (result_path.stem + "-logs"))
    stage = "preconditions"
    temporary_names = []
    old_images = {}
    prefix = compose_prefix(repo, runtime_env)
    image = None
    try:
        require(not result_path.is_relative_to(repo) and not result_path.is_relative_to(candidate),
                "Private publication output must remain outside both Git checkouts")
        review = json.loads(Path(review_path).read_text())
        validate_candidate(candidate, repo, review)
        image = "strategy-vault-engine:reviewed-" + review["candidate_commit"][:12]
        settings = runtime_settings(runtime_env)
        report.update(source_reviewed=True, implementation_ready=True, ledger_checked=True,
                      checked_trades=review["checked_trades"], evidence=list(review["evidence"]),
                      candidate_commit=review["candidate_commit"], method_ids=review["method_ids"])
        commands.run(["python3", str(repo / "scripts/check_upload.py"), "--env-file", str(runtime_env)], cwd=candidate, stage="candidate-upload-scan")
        stage = "real_data_tests"
        env = {**os.environ, "VAULT_IDX": str(Path(settings["VAULT_RUNTIME_ROOT"]) / "data/idx_data.csv"),
               "CARGO_BUILD_JOBS": "1", "CARGO_INCREMENTAL": "0", "CARGO_PROFILE_DEV_DEBUG": "0",
               "CARGO_PROFILE_TEST_DEBUG": "0", "RAYON_NUM_THREADS": "1",
               "CARGO_TARGET_DIR": str(result_path.parent / "release-cargo-target")}
        cargo = ["cargo", "test", "--locked", "--manifest-path", str(candidate / "Context/guru/backend/Cargo.toml")]
        for test in review["focused_tests"]:
            output = commands.run(cargo + [test, "--", "--test-threads=1"], cwd=candidate, env=env, timeout=900, stage="focused-rust")
            require(re.search(r"running [1-9][0-9]* tests?", output), "Focused Rust filter ran zero tests")
        commands.run(cargo + ["--", "--test-threads=1"], cwd=candidate, env=env, timeout=1800, stage="full-rust-actual-data")
        stage = "candidate_image"
        commands.run(["docker", "build", "-t", image, "-f", str(candidate / "Context/guru/webapp/infra/engine.Dockerfile"), str(candidate / "Context/guru")], cwd=candidate, timeout=1800, stage="engine-build")
        ids = engine_ids(commands, prefix)
        require(all(ids.values()), "Existing two engine services are unavailable")
        for market, container_id in ids.items():
            old_images[market] = commands.run(["docker", "inspect", "--format", "{{.Image}}", container_id], stage="previous-image")
        if deploy:
            require(old_images["india"] == old_images["us"], "Engine images differ; a two-image rollback needs explicit handling")
            configured = settings.get("VAULT_ENGINE_IMAGE", "strategy-vault-engine:local")
            require("@" not in configured and not configured.startswith("sha256:"), "Runtime uses an immutable image setting; preserve it and configure the new image explicitly")
        stage = "candidate_api_validation"
        api_evidence, newly_admitted = [], set()
        before_catalogues, after_catalogues = [], []
        existing_regressions = []
        remaining_regression_cases = 200
        touched_books = touched_book_ids(candidate, review)
        for market in ("india", "us"):
            cases = [case for case in review["smoke_cases"] if case["market"] == market]
            baseline_base = live_base(commands, ids[market])
            baseline = catalogue(baseline_base)
            before_catalogues.extend(baseline)
            name = "vault-reviewed-" + uuid.uuid4().hex[:12]
            temporary_names.append(name)
            base = temporary_engine(commands, image, market, settings, name)
            current = catalogue(base)
            after_catalogues.extend(current)
            newly_admitted |= check_catalogue_changes(baseline, current, set(review["method_ids"]))
            api_evidence.extend(smoke(base, cases, current))
            regression = compare_existing_runs(baseline_base, base, baseline, cases, touched_books,
                                               set(review["method_ids"]), max_cases=remaining_regression_cases)
            remaining_regression_cases -= regression["planned_cases"]
            existing_regressions.append({"market": market, **regression})
            commands.run(["docker", "rm", "-f", name], stage="candidate-cleanup")
            temporary_names.remove(name)
        candidate_live_ids = {row["id"] for row in after_catalogues if row.get("status") == "live"}
        require(set(review["method_ids"]) <= candidate_live_ids, "An approved method is not admitted by the candidate catalogue")
        report["candidate_api_checks"] = api_evidence
        report["existing_method_regression"] = existing_regressions
        validate_candidate(candidate, repo, review)
        stage = "source_publication"
        remote = git(repo, "ls-remote", "origin", "refs/heads/Manan").split()
        require(remote and remote[0] == review["base_commit"], "Remote Manan advanced; preserve the other developer's work and re-review")
        commands.run(["git", "-C", str(repo), "fetch", "--no-tags", str(candidate), review["candidate_commit"]], stage="fetch-reviewed-commit")
        require(not git(repo, "status", "--porcelain") and git(repo, "rev-parse", "HEAD") == review["base_commit"],
                "Authoritative source changed before publication; preserve that work")
        commands.run(["git", "-C", str(repo), "cherry-pick", "--ff", review["candidate_commit"]], stage="apply-reviewed-commit")
        commands.run(["python3", "scripts/check_upload.py", "--env-file", str(runtime_env)], cwd=repo, stage="public-upload-scan")
        commands.run(["git", "-C", str(repo), "-c", "credential.helper=!gh auth git-credential", "push", "origin", "HEAD:refs/heads/Manan"], env={**os.environ, "GIT_TERMINAL_PROMPT": "0"}, timeout=120, stage="push-manan")
        report["source_uploaded"] = True
        if not deploy:
            report.update(status="source_uploaded", summary="Reviewed source uploaded on Manan; runtime deployment was not requested.")
            write_result(result_path, report)
            return 0
        stage = "engine_deployment"
        rollback = "strategy-vault-engine:before-reviewed-" + review["candidate_commit"][:12]
        commands.run(["docker", "tag", old_images["india"], rollback], stage="rollback-tag")
        deploy_env = {**os.environ, "VAULT_ENGINE_IMAGE": image}
        try:
            commands.run(prefix + ["up", "-d", "--no-build", "--no-deps", "engine-in", "engine-us"], env=deploy_env, timeout=120, stage="deploy-engines-only")
            live_checks = []
            for market, container_id in engine_ids(commands, prefix).items():
                base = live_base(commands, container_id)
                wait_api(base)
                current = catalogue(base)
                live_checks.extend(smoke(base, [case for case in review["smoke_cases"] if case["market"] == market], current))
            commands.run(["docker", "tag", image, configured], stage="persist-reviewed-image-alias")
            report.update(status="published", deployed=True, admitted=True, live_api_checks=live_checks,
                          engine_image=image, rollback_image=rollback,
                          admitted_method_ids=sorted(newly_admitted), reviewed_variant_count=len(review["method_ids"]),
                          summary="Independent source and actual-trade checks passed; reviewed Manan source and engine-only deployment are live.")
            report.update(admission_counts(before_catalogues, after_catalogues))
        except (Held, OSError, subprocess.TimeoutExpired, ValueError) as error:
            rollback_env = {**os.environ, "VAULT_ENGINE_IMAGE": rollback}
            commands.run(prefix + ["up", "-d", "--no-build", "--no-deps", "engine-in", "engine-us"], env=rollback_env, timeout=120, stage="restore-previous-engines")
            for container_id in engine_ids(commands, prefix).values():
                wait_api(live_base(commands, container_id))
            report.update(rollback_restored=True, summary="Reviewed source remains on Manan; failed runtime deployment was restored to the previous engine image.")
            raise Held("Runtime verification failed; previous engines restored") from error
        write_result(result_path, report)
        return 0
    except (Held, OSError, subprocess.TimeoutExpired, ValueError, TypeError, KeyError, AttributeError) as error:
        report.update(status="held", failed_stage=stage, error=str(error), admitted=False, admitted_method_count=0,
                      admitted_family_count=0, admitted_variant_count=0,
                      summary=report.get("summary", "Publication held; no unverified batch is claimed as live."))
        write_result(result_path, report)
        return 1
    finally:
        for name in temporary_names:
            try:
                subprocess.run(["docker", "rm", "-f", name], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=30)
            except (OSError, subprocess.TimeoutExpired):
                pass


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--candidate", type=Path, required=True)
    parser.add_argument("--repo", type=Path, required=True)
    parser.add_argument("--review", type=Path, required=True)
    parser.add_argument("--result", type=Path, required=True)
    parser.add_argument("--runtime-env", type=Path, required=True)
    parser.add_argument("--deploy-user-approved", action="store_true")
    args = parser.parse_args()
    return publish(args.candidate, args.repo, args.review, args.result, args.runtime_env, deploy=args.deploy_user_approved)


if __name__ == "__main__":
    raise SystemExit(main())
