"""Durable queue behavior, using small temporary processes rather than market data."""
import importlib.util
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import threading
import time
import unittest
from unittest import mock
import contextlib
import fcntl
import io
import resource

MODULE = Path(__file__).with_name("overnight_worker.py")
spec = importlib.util.spec_from_file_location("overnight_worker", MODULE)
worker = importlib.util.module_from_spec(spec)
spec.loader.exec_module(worker)


class WorkerTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        self.state = self.root / "private"
        self.state.mkdir()
        self.result = self.state / "result.json"
        self.manifest = self.root / "manifest.json"
        self.job = {
            "id": "book-method", "approved": True, "kind": "ledger_check",
            "argv": [sys.executable, "-c", "pass"], "cwd": str(self.root),
            "timeout_seconds": 2, "max_attempts": 1,
            "result_file": str(self.result),
            "provenance": {
                "author": "Test fixture", "book": "Temporary queue test", "method": "test",
                "source_paths": [], "data_paths": [], "adaptations": [],
                "instrument": {"kind": "stock", "market": "test", "symbols": ["TEST"]},
            },
        }

    def tearDown(self):
        self.temp.cleanup()

    def write_manifest(self):
        self.manifest.write_text(json.dumps({"schema_version": 1, "approved_by": "test", "jobs": [self.job]}))

    def writes_result(self, payload=None):
        code = "import json,pathlib;pathlib.Path(" + repr(str(self.result)) + ").write_text(json.dumps(" + repr(payload or {"summary": "task completed"}) + "))"
        self.job["argv"] = [sys.executable, "-c", code]

    def run_worker(self, **kwargs):
        self.write_manifest()
        return worker.run(self.manifest, self.state, min_available_mb=0, poll_seconds=0.02, **kwargs)

    def test_success_does_not_invent_review_or_admission(self):
        self.writes_result()
        self.assertEqual(self.run_worker(), 0)
        status = worker.read_status(self.state)
        self.assertEqual(status["counts"]["succeeded"], 1)
        self.assertEqual(status["counts"]["source_reviewed"], 0)
        self.assertEqual(status["counts"]["admitted"], 0)
        self.assertEqual(status["counts"]["ledger_checked"], 0)

    def test_actual_evidence_flags_and_trade_counts_preserved(self):
        evidence = self.state / "evidence.txt"
        evidence.write_text("independent comparison test artifact")
        self.writes_result({"source_reviewed": True, "implementation_ready": True,
                            "admitted": True, "ledger_checked": True, "checked_trades": 4,
                            "evidence": [str(evidence)]})
        self.assertEqual(self.run_worker(), 0)
        status = worker.read_status(self.state)
        self.assertEqual(status["counts"]["ledger_checked"], 1)
        self.assertEqual(status["jobs"]["book-method"]["result"]["checked_trades"], 4)

    def test_successful_job_is_not_repeated_on_resume(self):
        count = self.state / "count.txt"
        self.writes_result()
        self.job["argv"][2] += ";pathlib.Path(" + repr(str(count)) + ").write_text('one')"
        self.assertEqual(self.run_worker(), 0)
        count.write_text("sentinel")
        self.assertEqual(self.run_worker(), 0)
        self.assertEqual(count.read_text(), "sentinel")
        self.assertEqual(worker.read_status(self.state)["jobs"]["book-method"]["attempts"], 1)

    def test_nonzero_exit_is_retried_only_up_to_cap(self):
        self.job["argv"] = [sys.executable, "-c", "raise SystemExit(7)"]
        self.job["max_attempts"] = 2
        self.assertEqual(self.run_worker(), 1)
        item = worker.read_status(self.state)["jobs"]["book-method"]
        self.assertEqual(item["attempts"], 2)
        self.assertEqual(item["status"], "failed")
        self.assertEqual(item["exit_code"], 7)

    def test_timeout_kills_process_and_records_failure(self):
        self.job["argv"] = [sys.executable, "-c", "import time;time.sleep(30)"]
        self.job["timeout_seconds"] = 0.1
        self.assertEqual(self.run_worker(), 1)
        self.assertEqual(worker.read_status(self.state)["jobs"]["book-method"]["error"], "Job timeout")

    def test_stop_file_interrupts_current_job(self):
        self.job["argv"] = [sys.executable, "-c", "import time;time.sleep(30)"]
        def stop():
            time.sleep(0.12)
            worker.request_stop(self.state)
        thread = threading.Thread(target=stop)
        thread.start()
        self.assertEqual(self.run_worker(), 0)
        thread.join()
        self.assertEqual(worker.read_status(self.state)["status"], "stopped")
        self.assertEqual(worker.read_status(self.state)["jobs"]["book-method"]["status"], "interrupted")

    def test_changed_job_id_cannot_replace_completed_provenance(self):
        self.writes_result()
        self.assertEqual(self.run_worker(), 0)
        self.job["provenance"]["book"] = "different book"
        with self.assertRaisesRegex(ValueError, "changed"):
            self.run_worker()

    def test_unapproved_job_is_held(self):
        self.job["approved"] = False
        self.writes_result()
        self.assertEqual(self.run_worker(), 0)
        self.assertFalse(self.result.exists())
        self.assertEqual(worker.read_status(self.state)["counts"]["awaiting_approval"], 1)

    def test_missing_artifact_does_not_count_as_success(self):
        self.assertEqual(self.run_worker(), 1)
        self.assertIn("result", worker.read_status(self.state)["jobs"]["book-method"]["error"].lower())

    def test_unsupported_claim_without_evidence_is_rejected(self):
        self.writes_result({"source_reviewed": True, "ledger_checked": True, "checked_trades": 0})
        self.assertEqual(self.run_worker(), 1)
        self.assertIn("evidence", worker.read_status(self.state)["jobs"]["book-method"]["error"].lower())

    def test_result_path_cannot_escape_private_state(self):
        self.job["result_file"] = str(self.root / "outside.json")
        self.write_manifest()
        with self.assertRaisesRegex(ValueError, "result_file"):
            worker.load_manifest(self.manifest, self.state)

    def test_resource_limit_contract_is_visible_in_child(self):
        code = "import os,resource,json,pathlib;pathlib.Path(" + repr(str(self.result)) + ").write_text(json.dumps({'nice':os.getpriority(os.PRIO_PROCESS,0),'cores':len(os.sched_getaffinity(0)),'memory':resource.getrlimit(resource.RLIMIT_AS)[0],'threads':os.environ['OMP_NUM_THREADS']}))"
        self.job["argv"] = [sys.executable, "-c", code]
        self.assertEqual(self.run_worker(memory_mb=256, cpu_cores=1), 0)
        result = worker.read_status(self.state)["jobs"]["book-method"]["result"]
        self.assertGreaterEqual(result["nice"], 10)
        self.assertEqual(result["cores"], 1)
        self.assertEqual(result["memory"], 256 * 1024 * 1024)
        self.assertEqual(result["threads"], "1")

    def trusted_publisher_job(self):
        checkout = MODULE.resolve().parent.parent
        self.job.update(kind="source_review", memory_mode="resident", cwd=str(checkout))
        self.job["argv"] = [sys.executable, str(checkout / "scripts/publish_reviewed_batch.py"),
                            "--candidate", str(self.root / "candidate"), "--repo", str(checkout),
                            "--review", str(self.state / "review.json"), "--result", str(self.result),
                            "--runtime-env", str(self.state / "runtime.env")]
        return {"memory_max_bytes": 3 * 1024 ** 3, "swap_max_bytes": 0, "cgroup_path": "/test-service"}

    def test_resident_mode_is_only_accepted_for_trusted_publisher_in_bounded_cgroup(self):
        limits = self.trusted_publisher_job()
        self.write_manifest()
        with mock.patch.object(worker, "cgroup_memory_limits", return_value=limits):
            manifest = worker.load_manifest(self.manifest, self.state)
        self.assertEqual(manifest["jobs"][0]["memory_mode"], "resident")
        for override in (None, {**limits, "memory_max_bytes": None},
                         {**limits, "memory_max_bytes": 4 * 1024 ** 3}, {**limits, "swap_max_bytes": None}):
            with self.subTest(override=override), mock.patch.object(worker, "cgroup_memory_limits", return_value=override):
                with self.assertRaisesRegex(ValueError, "cgroup"):
                    worker.load_manifest(self.manifest, self.state)

    def test_generic_python_jobs_cannot_disable_address_space_limit(self):
        self.job["memory_mode"] = "resident"
        self.write_manifest()
        with self.assertRaisesRegex(ValueError, "trusted publisher"):
            worker.load_manifest(self.manifest, self.state)
        self.job["memory_mode"] = "disabled"
        self.write_manifest()
        with self.assertRaisesRegex(ValueError, "memory_mode"):
            worker.load_manifest(self.manifest, self.state)

    def test_trusted_publisher_resident_mode_still_terminates_at_job_rss_budget(self):
        limits = self.trusted_publisher_job()
        self.write_manifest()
        # Keep process execution operational, while testing the trusted-job gate
        # against its real argv before substituting a harmless sleeping child.
        real_popen = worker.subprocess.Popen
        def harmless_child(argv, **kwargs):
            return real_popen([sys.executable, "-c", "import time;time.sleep(30)"], **kwargs)
        with mock.patch.object(worker, "cgroup_memory_limits", return_value=limits), \
             mock.patch.object(worker.subprocess, "Popen", side_effect=harmless_child), \
             mock.patch.object(worker, "descendant_rss_bytes", return_value=2048 * 1024 ** 2 + 1):
            self.assertEqual(worker.run(self.manifest, self.state, min_available_mb=0, poll_seconds=0.02), 1)
        item = worker.read_status(self.state)["jobs"]["book-method"]
        self.assertEqual(item["error"], "Job resident memory limit")
        self.assertEqual(item["memory_mode"], "resident")

    def test_resident_mode_omits_only_virtual_limit_in_actual_child(self):
        limits = self.trusted_publisher_job()
        self.write_manifest()
        real_popen = worker.subprocess.Popen
        code = "import resource,json,pathlib;pathlib.Path(" + repr(str(self.result)) + ").write_text(json.dumps({'virtual_limit':resource.getrlimit(resource.RLIMIT_AS)[0],'cpu_limit':resource.getrlimit(resource.RLIMIT_CPU)[0],'core_limit':resource.getrlimit(resource.RLIMIT_CORE)[0]}))"
        def harmless_child(argv, **kwargs):
            return real_popen([sys.executable, "-c", code], **kwargs)
        with mock.patch.object(worker, "cgroup_memory_limits", return_value=limits), mock.patch.object(worker.subprocess, "Popen", side_effect=harmless_child):
            self.assertEqual(worker.run(self.manifest, self.state, min_available_mb=0, poll_seconds=0.02), 0)
        result = worker.read_status(self.state)["jobs"]["book-method"]["result"]
        self.assertEqual(result["virtual_limit"], resource.getrlimit(resource.RLIMIT_AS)[0])
        self.assertGreater(result["cpu_limit"], 0)
        self.assertEqual(result["core_limit"], 0)

    def test_cgroup_limit_probe_uses_tighter_ancestor_and_preserves_swap_protection(self):
        cgroups = self.root / "cgroups"
        unit = cgroups / "parent" / "unit"
        unit.mkdir(parents=True)
        (unit / "memory.max").write_text("max")
        (unit / "memory.swap.max").write_text("max")
        (unit.parent / "memory.max").write_text(str(3 * 1024 ** 3))
        (unit.parent / "memory.swap.max").write_text("0")
        membership = self.root / "membership"
        membership.write_text("0::/parent/unit\n")
        limits = worker.cgroup_memory_limits(cgroups, membership)
        self.assertEqual(limits["memory_max_bytes"], 3 * 1024 ** 3)
        self.assertEqual(limits["swap_max_bytes"], 0)
        membership.write_text("1:memory:/legacy\n")
        self.assertIsNone(worker.cgroup_memory_limits(cgroups, membership))

    def test_resident_publisher_cannot_redirect_authoritative_repo_or_python_script(self):
        self.trusted_publisher_job()
        self.job["argv"][self.job["argv"].index("--repo") + 1] = str(self.root)
        self.write_manifest()
        with self.assertRaisesRegex(ValueError, "authoritative checkout"):
            worker.load_manifest(self.manifest, self.state)
        self.trusted_publisher_job()
        self.job["argv"][1] = str(self.state / "publish_reviewed_batch.py")
        self.write_manifest()
        with self.assertRaisesRegex(ValueError, "trusted publisher"):
            worker.load_manifest(self.manifest, self.state)

    def test_resident_publisher_cannot_raise_two_gib_job_budget(self):
        limits = self.trusted_publisher_job()
        self.write_manifest()
        with mock.patch.object(worker, "cgroup_memory_limits", return_value=limits), \
             mock.patch.object(worker.subprocess, "Popen") as process:
            self.assertEqual(worker.run(self.manifest, self.state, memory_mb=2049, min_available_mb=0, poll_seconds=0.02), 1)
        process.assert_not_called()
        self.assertIn("2 GiB", worker.read_status(self.state)["jobs"]["book-method"]["error"])

    def test_duplicate_publisher_repo_argument_is_rejected(self):
        self.trusted_publisher_job()
        self.job["argv"].extend(["--repo", str(MODULE.resolve().parent.parent)])
        self.write_manifest()
        with self.assertRaisesRegex(ValueError, "one explicit --repo"):
            worker.load_manifest(self.manifest, self.state)

    def test_cli_status_and_stop(self):
        self.writes_result()
        self.assertEqual(self.run_worker(), 0)
        status = subprocess.run([sys.executable, str(MODULE), "status", "--state-dir", str(self.state)], capture_output=True, text=True, check=True)
        self.assertEqual(json.loads(status.stdout)["counts"]["succeeded"], 1)
        subprocess.run([sys.executable, str(MODULE), "stop", "--state-dir", str(self.state)], capture_output=True, check=True)
        self.assertTrue((self.state / "STOP").exists())

    def test_isolated_agent_contract_accepts_explicit_noninteractive_flags(self):
        worktree = self.root / "worktree"
        worktree.mkdir()
        (worktree / ".git").write_text("gitdir: temporary test fixture")
        (self.root / "outside-prompt.txt").write_text("outside private state")
        prompt = self.state / "prompt.txt"
        prompt.write_text("Review only the temporary test; do not spawn other sessions.")
        schema = self.state / "schema.json"
        schema.write_text("{}")
        self.job.update(kind="agent_conversion", isolated_workspace=True, stdin_file=str(prompt), cwd=str(worktree))
        self.job["argv"] = ["/usr/bin/codex", "exec", "-c", 'approval_policy="never"',
                            "--sandbox", "workspace-write", "--json", "--output-schema", str(schema),
                            "--output-last-message", str(self.result), "--add-dir", str(self.state), "-"]
        self.write_manifest()
        self.assertEqual(worker.load_manifest(self.manifest, self.state)["jobs"][0]["kind"], "agent_conversion")
        base = json.loads(json.dumps(self.job))
        for change in (
                {"argv": ["/usr/bin/codex"]},
                {"isolated_workspace": False},
                {"argv": base["argv"] + ["--dangerously-bypass-approvals-and-sandbox"]},
                {"argv": [arg for arg in base["argv"] if arg != 'approval_policy="never"']},
                {"argv": ["read-only" if arg == "workspace-write" else arg for arg in base["argv"] if arg != "--sandbox"]},
                {"argv": [arg for arg in base["argv"] if arg != "--output-last-message"]},
                {"argv": [arg for arg in base["argv"] if arg != "--output-schema"]},
                {"stdin_file": str(self.state / "missing-prompt.txt")},
                {"stdin_file": str(self.root / "outside-prompt.txt")},
                {"argv": base["argv"][:-3]},
        ):
            with self.subTest(change=change):
                self.job = {**base, **change}
                self.write_manifest()
                with self.assertRaises(ValueError):
                    worker.load_manifest(self.manifest, self.state)

    def test_manifest_validation_rejects_unbounded_or_malformed_jobs(self):
        base = json.loads(json.dumps(self.job))
        for change in ({"id": "../unsafe"}, {"argv": "echo hi"}, {"approved": 1},
                       {"kind": "arbitrary"}, {"cwd": "/nonexistent-queue-directory"},
                       {"timeout_seconds": 7201}, {"max_attempts": 1.5},
                       {"provenance": {}}, {"provenance": {**base["provenance"], "data_paths": "not a list"}}):
            with self.subTest(change=change):
                self.job = {**base, **change}
                self.write_manifest()
                with self.assertRaises(ValueError):
                    worker.load_manifest(self.manifest, self.state)
        self.manifest.write_text(json.dumps({"schema_version": 2, "jobs": []}))
        with self.assertRaises(ValueError):
            worker.load_manifest(self.manifest, self.state)
        self.manifest.write_text(json.dumps({"schema_version": 1, "approved_by": "test", "jobs": [base] * 201}))
        with self.assertRaises(ValueError):
            worker.load_manifest(self.manifest, self.state)

    def test_provenance_records_real_input_hashes_and_missing_inputs(self):
        source = self.root / "source.txt"
        source.write_text("a test source identity")
        large = self.root / "large-input"
        with large.open("wb") as handle:
            handle.truncate(33 * 1024 * 1024)
        self.job["provenance"]["source_paths"] = [str(source), str(self.root / "missing")]
        self.job["provenance"]["data_paths"] = [str(large)]
        snapshot = worker.provenance_snapshot(self.job)
        self.assertEqual(len(snapshot["inputs"][0]["sha256"]), 64)
        self.assertFalse(snapshot["inputs"][1]["exists"])
        self.assertIn("sha256_status", snapshot["inputs"][2])

    def test_false_approval_can_become_true_without_changing_task(self):
        self.writes_result()
        self.job["approved"] = False
        self.assertEqual(self.run_worker(), 0)
        self.job["approved"] = True
        self.assertEqual(self.run_worker(), 0)
        self.assertTrue(self.result.exists())

    def test_worker_waits_for_available_memory(self):
        self.writes_result()
        self.write_manifest()
        with mock.patch.object(worker, "available_memory_mb", side_effect=[0, 4096]):
            self.assertEqual(worker.run(self.manifest, self.state, min_available_mb=2048, poll_seconds=0.02), 0)
        self.assertEqual(worker.read_status(self.state)["status"], "completed")

    def test_watch_queue_respects_runtime_limit(self):
        self.writes_result()
        self.assertEqual(self.run_worker(watch=True, max_runtime_seconds=0.3), 0)
        self.assertEqual(worker.read_status(self.state)["status"], "runtime_limit")

    def test_continuous_mode_has_no_deadline_and_remains_stoppable(self):
        self.writes_result()
        def stop():
            for _ in range(500):
                if self.result.exists():
                    break
                time.sleep(0.01)
            worker.request_stop(self.state)
        thread = threading.Thread(target=stop)
        thread.start()
        try:
            self.assertEqual(self.run_worker(watch=True, max_runtime_seconds=None), 0)
        finally:
            thread.join()
        status = worker.read_status(self.state)
        self.assertEqual(status["status"], "stopped")
        self.assertIsNone(status["limits"]["max_runtime_seconds"])
        self.assertEqual(status["jobs"]["book-method"]["attempts"], 1)

    def test_usage_limit_detection_requires_a_real_agent_error_event(self):
        path = self.state / "agent.jsonl"
        for event in (
            {"type": "turn.failed", "error": {"code": "usage_limit_reached", "message": "Usage limit reached"}},
            {"type": "error", "message": "You've hit your usage limit. Try again later."},
            {"type": "error", "error": {"code": "insufficient_quota", "message": "Quota exhausted"}},
            {"type": "turn.failed", "error": {"code": "rate_limit_exceeded", "message": "Too many requests"}},
        ):
            with self.subTest(event=event):
                path.write_text(json.dumps(event) + "\n")
                self.assertIsNotNone(worker.agent_usage_limit(path))
        for event in (
            {"type": "item.completed", "item": {"text": "Test the usage_limit_reached error"}},
            {"type": "error", "message": "An ordinary source file could not be read"},
            {"type": "turn.completed", "usage": {"input_tokens": 123}},
        ):
            with self.subTest(event=event):
                path.write_text(json.dumps(event) + "\n")
                self.assertIsNone(worker.agent_usage_limit(path))

    def test_usage_limit_stops_queue_without_retrying_or_starting_next_job(self):
        self.write_manifest()
        manifest = json.loads(self.manifest.read_text())
        second = {**self.job, "id": "next-book", "result_file": str(self.state / "next.json")}
        manifest["jobs"].append(second)
        self.manifest.write_text(json.dumps(manifest))
        def limited(job, state_dir, item, state, **kwargs):
            item.update(status="usage_limit", error="usage_limit_reached")
        with mock.patch.object(worker, "execute", side_effect=limited) as execute:
            self.assertEqual(worker.run(self.manifest, self.state, min_available_mb=0), 0)
        self.assertEqual(execute.call_count, 1)
        self.assertEqual(worker.read_status(self.state)["status"], "usage_limit")
        self.assertEqual(worker.read_status(self.state)["jobs"]["next-book"]["status"], "queued")
        self.assertTrue((self.state / "LIMIT_REACHED.json").is_file())

    def test_limit_marker_prevents_automatic_restart_until_explicitly_cleared(self):
        self.writes_result()
        self.write_manifest()
        (self.state / "LIMIT_REACHED.json").write_text(json.dumps({"reason": "quota"}))
        with mock.patch.object(worker, "execute") as execute:
            self.assertEqual(worker.run(self.manifest, self.state, min_available_mb=0), 0)
        execute.assert_not_called()
        self.assertEqual(worker.read_status(self.state)["status"], "usage_limit")

    def test_agent_error_is_classified_before_generic_nonzero_exit(self):
        payload = {"type": "turn.failed", "error": {"code": "usage_limit_reached", "message": "limit reached"}}
        self.job["argv"] = [sys.executable, "-c", "import json;print(" + repr(json.dumps(payload)) + ");raise SystemExit(1)"]
        self.write_manifest()
        state = {"jobs": {self.job["id"]: {"attempts": 1, "status": "queued"}}}
        with mock.patch.object(worker, "validate_memory_mode", return_value=None):
            worker.execute({**self.job, "kind": "agent_conversion"}, self.state, state["jobs"][self.job["id"]], state,
                           memory_mb=256, cpu_cores=1, poll_seconds=0.02,
                           deadline=time.monotonic()+3, log_limit_bytes=1024*1024)
        self.assertEqual(state["jobs"][self.job["id"]]["status"], "usage_limit")

    def test_live_usage_limit_event_interrupts_a_retrying_agent(self):
        payload={"type":"error","message":"You've hit your usage limit."}
        self.job['argv']=[sys.executable,'-c',"import time;print("+repr(json.dumps(payload))+",flush=True);time.sleep(30)"]
        self.job['timeout_seconds']=2
        state={'jobs':{self.job['id']:{'attempts':1,'status':'queued'}}}
        worker.execute({**self.job,'kind':'agent_conversion'},self.state,state['jobs'][self.job['id']],state,
                       memory_mb=256,cpu_cores=1,poll_seconds=0.02,
                       deadline=time.monotonic()+5,log_limit_bytes=1024*1024)
        self.assertEqual(state['jobs'][self.job['id']]['status'],'usage_limit')

    def test_existing_file_lock_prevents_second_worker(self):
        self.write_manifest()
        with (self.state / "worker.lock").open("a") as lock:
            fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
            with self.assertRaisesRegex(ValueError, "Another worker"):
                worker.run(self.manifest, self.state)

    def test_result_contract_rejects_false_evidence_and_invalid_claims(self):
        for payload in ([], {"admitted": True}, {"source_reviewed": "yes"},
                        {"checked_trades": -1}, {"evidence": [str(self.root / "missing")]},
                        {"evidence": "not a list"}):
            with self.subTest(payload=payload):
                self.result.write_text(json.dumps(payload))
                with self.assertRaises(ValueError):
                    worker.validate_result(self.result)

    def test_missing_executable_is_recorded_without_hanging(self):
        self.job["argv"] = ["/nonexistent-queue-executable"]
        self.assertEqual(self.run_worker(), 1)
        self.assertEqual(worker.read_status(self.state)["jobs"]["book-method"]["status"], "failed")

    def test_invalid_resource_limits_are_rejected(self):
        self.write_manifest()
        with self.assertRaises(ValueError):
            worker.run(self.manifest, self.state, memory_mb=8193)
        with self.assertRaises(ValueError):
            worker.run(self.manifest, self.state, cpu_cores=3)

    def test_main_run_status_stop_and_invalid_manifest(self):
        self.writes_result()
        self.write_manifest()
        with mock.patch.object(sys, "argv", [str(MODULE), "run", "--manifest", str(self.manifest), "--state-dir", str(self.state)]):
            self.assertEqual(worker.main(), 0)
        for command in ("status", "stop"):
            with mock.patch.object(sys, "argv", [str(MODULE), command, "--state-dir", str(self.state)]), contextlib.redirect_stdout(io.StringIO()):
                self.assertEqual(worker.main(), 0)
        self.manifest.write_text("invalid JSON")
        with mock.patch.object(sys, "argv", [str(MODULE), "run", "--manifest", str(self.manifest), "--state-dir", str(self.state)]), contextlib.redirect_stderr(io.StringIO()):
            self.assertEqual(worker.main(), 2)


if __name__ == "__main__":
    unittest.main()
