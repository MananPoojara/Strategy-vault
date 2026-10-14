#!/usr/bin/env python3
"""Run an explicitly reviewed private queue, without inferring strategy verification.

Linux, Python 3.10+, standard library only. Commands execute sequentially. This
is an execution supervisor, not a rule extractor or a strategy admission gate.
"""
import argparse
import collections
import fcntl
import hashlib
import json
import math
import os
from pathlib import Path
import re
import resource
import signal
import shutil
import subprocess
import sys
import time
from datetime import datetime, timezone

KINDS = {"source_review", "reference_backtest", "ledger_check", "data_audit", "agent_conversion"}
FLAGS = ("source_reviewed", "implementation_ready", "admitted", "ledger_checked")


def timestamp():
    return datetime.now(timezone.utc).isoformat()


def digest(value):
    return hashlib.sha256(json.dumps(value, sort_keys=True, separators=(",", ":")).encode()).hexdigest()


def atomic_json(path, value):
    path = Path(path)
    temporary = path.with_name(path.name + ".tmp")
    with temporary.open("w") as handle:
        os.chmod(temporary, 0o600)
        json.dump(value, handle, indent=2, allow_nan=False)
        handle.write("\n")
        handle.flush()
        os.fsync(handle.fileno())
    os.replace(temporary, path)


def inside(path, parent):
    return Path(path).resolve().is_relative_to(Path(parent).resolve())


def cgroup_memory_limits(cgroup_root=Path("/sys/fs/cgroup"), membership_file=Path("/proc/self/cgroup")):
    """Read effective cgroup-v2 hard limits, including tighter ancestor limits."""
    try:
        membership = next(line[3:] for line in Path(membership_file).read_text().splitlines() if line.startswith("0::"))
        root = Path(cgroup_root).resolve()
        current = (root / membership.lstrip("/")).resolve()
        if not current.is_relative_to(root):
            return None
        memory, swap = [], []
        paths = [current, *(parent for parent in current.parents if parent.is_relative_to(root))]
        for directory in paths:
            for filename, values in (("memory.max", memory), ("memory.swap.max", swap)):
                path = directory / filename
                if path.is_file():
                    value = path.read_text().strip()
                    if value != "max":
                        parsed = int(value)
                        if parsed < 0:
                            return None
                        values.append(parsed)
        return {"memory_max_bytes": min(memory) if memory else None,
                "swap_max_bytes": min(swap) if swap else None, "cgroup_path": membership}
    except (OSError, ValueError, StopIteration):
        return None


def validate_memory_mode(job):
    mode = job.get("memory_mode", "address_space")
    if mode not in {"address_space", "resident"}:
        raise ValueError("memory_mode must be address_space or resident")
    if mode != "resident":
        return None
    argv = job["argv"]
    checkout = Path(__file__).resolve().parent.parent
    executable = shutil.which(argv[0])
    script = (Path(job["cwd"]) / argv[1]).resolve() if len(argv) > 1 else None
    if (job["kind"] != "source_review" or not executable or
            Path(executable).resolve() != Path(sys.executable).resolve() or
            script != checkout / "scripts/publish_reviewed_batch.py"):
        raise ValueError("Resident memory mode is restricted to the trusted publisher")
    if argv.count("--repo") != 1:
        raise ValueError("Resident trusted publisher requires one explicit --repo")
    index = argv.index("--repo")
    if index + 1 >= len(argv) or Path(argv[index + 1]).resolve() != checkout:
        raise ValueError("Resident trusted publisher must use the authoritative checkout")
    limits = cgroup_memory_limits()
    hard_limit = limits.get("memory_max_bytes") if limits else None
    if (not isinstance(hard_limit, int) or not 0 < hard_limit <= 3 * 1024 ** 3 or
            limits.get("swap_max_bytes") != 0):
        raise ValueError("Resident publisher requires a cgroup hard memory cap at most 3 GiB and zero swap")
    return limits


def validate_agent(job, state_dir):
    """Permit an explicit isolated, noninteractive Codex invocation only."""
    argv = job["argv"]
    if Path(argv[0]).name != "codex" or "exec" not in argv or "--json" not in argv:
        raise ValueError("agent_conversion requires an explicit Codex exec --json command")
    if job.get("isolated_workspace") is not True or not (Path(job["cwd"]) / ".git").exists():
        raise ValueError("agent_conversion requires an isolated Git worktree")
    if any("dangerously-bypass" in arg for arg in argv):
        raise ValueError("agent_conversion must retain the workspace sandbox")
    if not any(arg in {"approval_policy=\"never\"", "approval_policy='never'", "approval_policy=never"} for arg in argv):
        raise ValueError("agent_conversion requires approval_policy=never")
    def flag_value(flag):
        index = argv.index(flag) if flag in argv else len(argv)
        return argv[index + 1] if index + 1 < len(argv) else None
    if flag_value("--sandbox") not in {"workspace-write", "read-only"}:
        raise ValueError("agent_conversion requires an explicit workspace sandbox")
    output_flag = "--output-last-message" if "--output-last-message" in argv else "-o"
    if not flag_value(output_flag) or Path(flag_value(output_flag)).resolve() != Path(job["result_file"]).resolve():
        raise ValueError("agent_conversion must write its structured final result to result_file")
    if not flag_value("--output-schema") or not Path(flag_value("--output-schema")).is_file():
        raise ValueError("agent_conversion requires a structured output schema")
    if not job.get("stdin_file") or not Path(job["stdin_file"]).is_file():
        raise ValueError("agent_conversion requires an explicit reviewed stdin_file prompt")
    if not inside(job["stdin_file"], state_dir):
        raise ValueError("agent prompt must remain in the private state directory")
    if not inside(job["result_file"], Path(job["cwd"])) and not any(
            arg == "--add-dir" and index + 1 < len(argv) and inside(job["result_file"], argv[index + 1])
            for index, arg in enumerate(argv)):
        raise ValueError("The workspace sandbox must explicitly permit the private result directory")


def load_manifest(path, state_dir):
    manifest = json.loads(Path(path).read_text())
    if manifest.get("schema_version") != 1 or not manifest.get("approved_by"):
        raise ValueError("Manifest requires schema_version 1 and approved_by")
    jobs = manifest.get("jobs")
    if not isinstance(jobs, list) or len(jobs) > 200:
        raise ValueError("Manifest jobs must be a bounded list of at most 200")
    seen = set()
    for job in jobs:
        job_id = job.get("id", "")
        if not re.fullmatch(r"[a-zA-Z0-9][a-zA-Z0-9_.-]{0,95}", job_id) or job_id in seen:
            raise ValueError("Each job requires a unique safe id")
        seen.add(job_id)
        argv = job.get("argv")
        if not isinstance(argv, list) or not argv or any(not isinstance(arg, str) or "\0" in arg for arg in argv):
            raise ValueError("Jobs require an argv list, never a shell command")
        if job.get("approved") not in (True, False) or not isinstance(job.get("approved"), bool):
            raise ValueError("Jobs require an explicit approved boolean")
        if job.get("kind") not in KINDS:
            raise ValueError("Unsupported job kind")
        cwd = Path(job.get("cwd", ""))
        if not cwd.is_absolute() or not cwd.is_dir() or "/gvfs/" in str(cwd):
            raise ValueError("cwd must be an existing local directory, not an original share")
        result_file = Path(job.get("result_file", ""))
        if not result_file.is_absolute() or not inside(result_file, state_dir):
            raise ValueError("result_file must be inside the private state directory")
        for name, default, lower, upper in (("timeout_seconds", 600, 0.05, 7200), ("max_attempts", 1, 1, 3)):
            value = job.get(name, default)
            if isinstance(value, bool) or not isinstance(value, (int, float)) or not math.isfinite(value) or not lower <= value <= upper:
                raise ValueError("Invalid bounded " + name)
            if name == "max_attempts" and not isinstance(value, int):
                raise ValueError("max_attempts must be an integer")
        provenance = job.get("provenance", {})
        if not isinstance(provenance, dict) or not all(provenance.get(key) for key in ("author", "book", "method", "instrument")):
            raise ValueError("Every job requires book, method and instrument provenance")
        for name in ("source_paths", "data_paths", "adaptations"):
            if not isinstance(provenance.get(name), list):
                raise ValueError("Provenance requires an explicit " + name + " list")
        if job["kind"] == "agent_conversion":
            validate_agent(job, state_dir)
        validate_memory_mode(job)
    return manifest


def read_status(state_dir):
    path = Path(state_dir) / "status.json"
    return json.loads(path.read_text()) if path.exists() else {"status": "not_started", "jobs": {}, "counts": {}}


def request_stop(state_dir):
    root = Path(state_dir)
    root.mkdir(parents=True, exist_ok=True, mode=0o700)
    (root / "STOP").write_text(timestamp() + "\n")


def available_memory_mb():
    for line in Path("/proc/meminfo").read_text().splitlines():
        if line.startswith("MemAvailable:"):
            return int(line.split()[1]) // 1024
    return 0


def child_limits(memory_mb, cpu_cores, timeout_seconds, address_space=True):
    def configure():
        os.nice(10)
        affinity = sorted(os.sched_getaffinity(0))[:cpu_cores]
        os.sched_setaffinity(0, affinity)
        memory = memory_mb * 1024 * 1024
        if address_space:
            resource.setrlimit(resource.RLIMIT_AS, (memory, memory))
        cpu_time = math.ceil(timeout_seconds) + 10
        resource.setrlimit(resource.RLIMIT_CPU, (cpu_time, cpu_time))
        resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
    return configure


def descendant_rss_bytes(pid):
    """Observe resident memory; systemd MemoryMax also bounds escaped descendants."""
    pending = [pid]
    seen = set()
    total = 0
    while pending:
        current = pending.pop()
        if current in seen:
            continue
        seen.add(current)
        try:
            for line in Path(f"/proc/{current}/status").read_text().splitlines():
                if line.startswith("VmRSS:"):
                    total += int(line.split()[1]) * 1024
                    break
            children = Path(f"/proc/{current}/task/{current}/children").read_text().split()
            pending.extend(int(value) for value in children)
        except (OSError, ValueError):
            pass
    return total


def terminate(process):
    try:
        os.killpg(process.pid, signal.SIGTERM)
    except ProcessLookupError:
        return
    try:
        process.wait(timeout=2)
    except subprocess.TimeoutExpired:
        try:
            os.killpg(process.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass
        process.wait(timeout=5)


def validate_result(path):
    path = Path(path)
    if not path.is_file() or path.stat().st_size > 20 * 1024 * 1024:
        raise ValueError("Missing or oversized result artifact")
    result = json.loads(path.read_text())
    if not isinstance(result, dict):
        raise ValueError("Result artifact must be a JSON object")
    for flag in FLAGS:
        if flag in result and not isinstance(result[flag], bool):
            raise ValueError("Result evidence flags must be booleans")
    if result.get("admitted") and not (result.get("source_reviewed") and result.get("implementation_ready")):
        raise ValueError("Admission requires reviewed source rules and implementation readiness")
    trades = result.get("checked_trades", 0)
    if isinstance(trades, bool) or not isinstance(trades, int) or trades < 0:
        raise ValueError("checked_trades must be a nonnegative integer")
    evidence = result.get("evidence", [])
    if not isinstance(evidence, list) or any(not isinstance(item, str) or not Path(item).is_absolute() or not Path(item).is_file() for item in evidence):
        raise ValueError("Result evidence must name existing absolute file paths")
    if result.get("ledger_checked") and not (result.get("source_reviewed") and evidence and trades > 0):
        raise ValueError("A checked ledger requires source review, evidence and checked trades")
    return result


def provenance_snapshot(job):
    """Retain source identity; large market files are hashed by their verifier."""
    paths = job["provenance"]["source_paths"] + job["provenance"]["data_paths"]
    if job.get("stdin_file"):
        paths.append(job["stdin_file"])
    records = []
    for name in paths:
        path = Path(name)
        record = {"path": str(path), "exists": path.exists()}
        if path.is_file():
            stat = path.stat()
            record.update(size_bytes=stat.st_size, mtime_ns=stat.st_mtime_ns)
            if stat.st_size <= 32 * 1024 * 1024:
                hasher = hashlib.sha256()
                with path.open("rb") as handle:
                    for chunk in iter(lambda: handle.read(1024 * 1024), b""):
                        hasher.update(chunk)
                record["sha256"] = hasher.hexdigest()
            else:
                record["sha256_status"] = "Verifier must hash this large input"
        records.append(record)
    return {"declared": job["provenance"], "inputs": records}


def save_status(state_dir, state, event=None):
    counts = collections.Counter(item["status"] for item in state["jobs"].values())
    for flag in FLAGS:
        counts[flag] = sum(item.get("status") == "succeeded" and item.get("result", {}).get(flag) is True for item in state["jobs"].values())
    state["counts"] = dict(counts)
    state["updated_at"] = timestamp()
    atomic_json(Path(state_dir) / "status.json", state)
    if event:
        with (Path(state_dir) / "journal.jsonl").open("a") as handle:
            os.chmod(handle.name, 0o600)
            handle.write(json.dumps({"at": timestamp(), **event}, allow_nan=False) + "\n")
            handle.flush()
            os.fsync(handle.fileno())


def execute(job, state_dir, item, state, *, memory_mb, cpu_cores, poll_seconds, deadline, log_limit_bytes):
    result_path = Path(job["result_file"])
    result_path.parent.mkdir(parents=True, exist_ok=True, mode=0o700)
    if result_path.exists():
        # An older final response cannot make a failed/resumed command look successful.
        archive = result_path.with_name(result_path.name + ".before-attempt-" + str(item["attempts"]))
        os.replace(result_path, archive)
    item.update(status="running", started_at=timestamp(), provenance=provenance_snapshot(job))
    item.pop("result", None)
    log_path = Path(state_dir) / "logs" / (job["id"] + ".attempt-" + str(item["attempts"]) + ".log")
    log_path.parent.mkdir(exist_ok=True, mode=0o700)
    item["log_file"] = str(log_path)
    save_status(state_dir, state, {"event": "job_started", "id": job["id"], "attempt": item["attempts"]})
    env = dict(os.environ)
    env.update(OMP_NUM_THREADS="1", OPENBLAS_NUM_THREADS="1", MKL_NUM_THREADS="1", NUMEXPR_NUM_THREADS="1", RAYON_NUM_THREADS="1", VAULT_OVERNIGHT_CHILD="1")
    started = time.monotonic()
    with log_path.open("wb") as log:
        os.chmod(log_path, 0o600)
        input_handle = open(job["stdin_file"], "rb") if job.get("stdin_file") else subprocess.DEVNULL
        try:
            cgroup_limits = validate_memory_mode(job)
            if job.get("memory_mode") == "resident" and memory_mb > 2048:
                raise ValueError("Resident publisher job RSS budget must stay at or below 2 GiB")
            address_space = (job.get("memory_mode", "address_space") != "resident" and
                             job["kind"] != "agent_conversion" and Path(job["argv"][0]).name not in {"node", "nodejs"})
            item["memory_mode"] = "address_space" if address_space else "resident"
            item["memory_limits"] = {"job_rss_bytes": memory_mb * 1024 * 1024, "cgroup": cgroup_limits,
                                     "address_space_limited": address_space}
            process = subprocess.Popen(job["argv"], cwd=job["cwd"], stdin=input_handle,
                                       stdout=log, stderr=subprocess.STDOUT, env=env,
                                       start_new_session=True,
                                       preexec_fn=child_limits(memory_mb, cpu_cores, job.get("timeout_seconds", 600),
                                                              address_space=address_space))
            item["pid"] = process.pid
            save_status(state_dir, state)
            failure = None
            while process.poll() is None:
                if (Path(state_dir) / "STOP").exists():
                    failure = "Stop requested"
                elif time.monotonic() >= deadline:
                    failure = "Worker runtime limit"
                elif time.monotonic() - started > job.get("timeout_seconds", 600):
                    failure = "Job timeout"
                elif log_path.stat().st_size > log_limit_bytes:
                    failure = "Job log limit"
                elif descendant_rss_bytes(process.pid) > memory_mb * 1024 * 1024:
                    failure = "Job resident memory limit"
                if failure:
                    terminate(process)
                    break
                time.sleep(poll_seconds)
            item["exit_code"] = process.returncode
            if failure:
                item.update(status="interrupted" if failure in {"Stop requested", "Worker runtime limit"} else "failed", error=failure)
            elif process.returncode != 0:
                item.update(status="failed", error="Nonzero command exit")
            else:
                item.update(status="succeeded", result=validate_result(result_path))
                item.pop("error", None)
        except (OSError, ValueError, json.JSONDecodeError) as error:
            item.update(status="failed", error=str(error))
        finally:
            if input_handle != subprocess.DEVNULL:
                input_handle.close()
    item.update(finished_at=timestamp(), elapsed_seconds=round(time.monotonic() - started, 3))
    item.pop("pid", None)
    save_status(state_dir, state, {"event": "job_finished", "id": job["id"], "status": item["status"], "attempt": item["attempts"]})


def run(manifest_path, state_dir, *, watch=False, max_runtime_seconds=24 * 3600,
        memory_mb=2048, cpu_cores=2, min_available_mb=2048,
        poll_seconds=0.5, log_limit_bytes=20 * 1024 * 1024):
    if not 64 <= memory_mb <= 8192 or not 1 <= cpu_cores <= 2:
        raise ValueError("Worker limits require 64..8192 MB and 1..2 CPU cores")
    state_dir = Path(state_dir).resolve()
    state_dir.mkdir(parents=True, exist_ok=True, mode=0o700)
    os.chmod(state_dir, 0o700)
    deadline = time.monotonic() + max_runtime_seconds
    lock = (state_dir / "worker.lock").open("a")
    try:
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
    except BlockingIOError:
        lock.close()
        raise ValueError("Another worker already owns this state directory")
    try:
        state = read_status(state_dir)
        state.update(schema_version=1, status="running", worker_pid=os.getpid(), limits={"memory_mb_per_job": memory_mb, "cpu_cores": cpu_cores, "sequential": True, "min_available_mb": min_available_mb})
        for item in state["jobs"].values():
            if item["status"] == "running":
                item.update(status="interrupted", error="Previous worker stopped before recording completion")
        last_saved=None
        while True:
            manifest = load_manifest(manifest_path, state_dir)
            state.update(manifest_file=str(Path(manifest_path).resolve()), manifest_sha256=digest(manifest), approved_by=manifest["approved_by"])
            jobs = manifest["jobs"]
            ready = None
            for job in jobs:
                specification = digest({key: value for key, value in job.items() if key != "approved"})
                item = state["jobs"].get(job["id"])
                if item and item["specification_sha256"] != specification:
                    raise ValueError("Job specification changed for existing id " + job["id"] + "; use a new id")
                if not item:
                    item = state["jobs"][job["id"]] = {"status": "queued" if job["approved"] else "awaiting_approval", "attempts": 0, "kind": job["kind"], "specification_sha256": specification, "provenance": {"declared": job["provenance"]}}
                elif item["status"] == "awaiting_approval" and job["approved"]:
                    item["status"] = "queued"
                if job["approved"] and item["status"] in {"queued", "failed", "interrupted"} and item["attempts"] < job.get("max_attempts", 1) and ready is None:
                    ready = job
            if digest({k:v for k,v in state.items() if k != "updated_at"}) != last_saved:
                save_status(state_dir, state)
                last_saved=digest({k:v for k,v in state.items() if k != "updated_at"})
            if (state_dir / "STOP").exists():
                state["status"] = "stopped"
                break
            if time.monotonic() >= deadline:
                state["status"] = "runtime_limit"
                break
            if ready and available_memory_mb() < min_available_mb:
                state["status"] = "waiting_for_memory"
                save_status(state_dir, state)
                time.sleep(min(10, max(poll_seconds, 0.1)))
                continue
            if ready:
                state["status"] = "running"
                item = state["jobs"][ready["id"]]
                item["attempts"] += 1
                execute(ready, state_dir, item, state, memory_mb=memory_mb, cpu_cores=cpu_cores,
                        poll_seconds=poll_seconds, deadline=deadline, log_limit_bytes=log_limit_bytes)
            elif watch:
                if state["status"] != "waiting_for_jobs":
                    state["status"] = "waiting_for_jobs"
                    save_status(state_dir, state)
                    last_saved=digest({k:v for k,v in state.items() if k != "updated_at"})
                time.sleep(min(10, max(poll_seconds, 5)))
            else:
                state["status"] = "completed_with_failures" if any(item["status"] == "failed" for item in state["jobs"].values()) else "completed"
                break
        save_status(state_dir, state, {"event": "worker_stopped", "status": state["status"]})
        return 1 if state["status"] == "completed_with_failures" else 0
    finally:
        fcntl.flock(lock, fcntl.LOCK_UN)
        lock.close()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    subcommands = parser.add_subparsers(dest="command", required=True)
    for name in ("run", "status", "stop"):
        command = subcommands.add_parser(name)
        command.add_argument("--state-dir", type=Path, required=True)
        if name == "run":
            command.add_argument("--manifest", type=Path, required=True)
            command.add_argument("--watch", action="store_true")
            command.add_argument("--max-runtime-hours", type=float, default=24)
            command.add_argument("--memory-mb", type=int, default=2048)
            command.add_argument("--cpu-cores", type=int, default=2)
            command.add_argument("--min-available-mb", type=int, default=2048)
        if name == "status":
            command.add_argument("--summary", action="store_true")
    args = parser.parse_args()
    if args.command == "status":
        state=read_status(args.state_dir)
        if args.summary:
            state={"status":state["status"],"counts":state.get("counts",{}),"updated_at":state.get("updated_at"),"jobs":[{"id":key,"status":job["status"],"error":job.get("error"),"summary":job.get("result",{}).get("summary")} for key,job in state.get("jobs",{}).items()]}
        print(json.dumps(state, indent=2))
        return 0
    if args.command == "stop":
        request_stop(args.state_dir)
        print(json.dumps({"stop_requested": True}))
        return 0
    if not 0 < args.max_runtime_hours <= 72:
        parser.error("max-runtime-hours must be above zero and at most 72")
    try:
        signal.signal(signal.SIGTERM, lambda *_: request_stop(args.state_dir))
        signal.signal(signal.SIGINT, lambda *_: request_stop(args.state_dir))
        return run(args.manifest, args.state_dir, watch=args.watch,
                   max_runtime_seconds=args.max_runtime_hours * 3600,
                   memory_mb=args.memory_mb, cpu_cores=args.cpu_cores,
                   min_available_mb=args.min_available_mb)
    except (OSError, ValueError) as error:
        print(json.dumps({"worker_error": str(error)}), file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
