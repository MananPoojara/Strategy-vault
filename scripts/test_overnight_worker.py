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
