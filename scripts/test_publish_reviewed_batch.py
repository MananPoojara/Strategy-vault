"""Publication gates use actual temporary Git histories; Docker remains isolated."""
import hashlib
import importlib.util
import json
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest import mock

MODULE = Path(__file__).with_name("publish_reviewed_batch.py")
spec = importlib.util.spec_from_file_location("publish_reviewed_batch", MODULE)
publisher = importlib.util.module_from_spec(spec)
spec.loader.exec_module(publisher)


def git(root, *args):
    return subprocess.check_output(["git", "-C", str(root), *args], stderr=subprocess.DEVNULL, text=True).strip()


class PublicationGates(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        self.repo = self.root / "repo"
        self.repo.mkdir()
        git(self.repo, "init", "-b", "Manan")
        git(self.repo, "config", "user.email", "queue-test@example.invalid")
        git(self.repo, "config", "user.name", "Queue test")
        self.path = "Context/guru/backend/src/books/testbook.rs"
        target = self.repo / self.path
        target.parent.mkdir(parents=True)
        target.write_text('pub const BOOK: &str = "testbook";\n// baseline source\n')
        git(self.repo, "add", ".")
        git(self.repo, "commit", "-m", "baseline")
        self.base = git(self.repo, "rev-parse", "HEAD")
        self.candidate = self.root / "candidate"
        git(self.root, "clone", "--quiet", str(self.repo), str(self.candidate))
        git(self.candidate, "config", "user.email", "queue-test@example.invalid")
        git(self.candidate, "config", "user.name", "Queue test")
        (self.candidate / self.path).write_text('pub const BOOK: &str = "testbook";\n// reviewed source change\n')
        git(self.candidate, "add", ".")
        git(self.candidate, "commit", "-m", "reviewed source")
        self.commit = git(self.candidate, "rev-parse", "HEAD")
        self.source = self.root / "private-source.txt"
        self.source.write_text("temporary source identity")
        self.ohlc = self.root / "private-market-data.txt"
        self.ohlc.write_text("temporary operational market-file identity")
        self.ledger = self.root / "private-ledger.json"
        self.ledger.write_text(json.dumps({"source_reviewed": True, "implementation_ready": True,
            "ledger_checked": True, "checked_trades": 2,
            "rust_module_sha256": {str(self.candidate / self.path): publisher.sha256(self.candidate / self.path)},
            "checks": [{"method": "test_rule", "row_mismatches": 0, "actual_records": 2, "expected_records": 2,
                        "ohlc_path": str(self.ohlc), "ohlc_sha256": publisher.sha256(self.ohlc)}]}))
        self.review = {"schema_version": 1, "reviewer_id": "independent-review-job", "conversion_session_id": "conversion-job",
            "candidate_commit": self.commit, "base_commit": self.base,
            "approved_files": [self.path], "method_ids": ["test_rule"],
            "source_reviewed": True, "implementation_ready": True, "ledger_checked": True,
            "checked_trades": 2, "row_mismatches": 0, "evidence": [str(self.ledger)],
            "source_hashes": {str(self.source): publisher.sha256(self.source)},
            "candidate_hashes": {self.path: publisher.sha256(self.candidate / self.path)},
            "ledger_reports": [{"path": str(self.ledger), "sha256": publisher.sha256(self.ledger)}],
            "focused_tests": ["books::testbook::tests"],
            "smoke_cases": [{"method_id": "test_rule", "guru": "connors", "market": "india",
                "query": {"ticker": "NIFTY 50"}, "expected_readiness": "ready", "instrument_kind": "index"}]}

    def tearDown(self):
        self.temp.cleanup()

    def test_matching_clean_single_commit_is_accepted(self):
        self.assertEqual(publisher.validate_candidate(self.candidate, self.repo, self.review), [self.path])

    def test_dirty_authoritative_checkout_is_held(self):
        (self.repo / self.path).write_text("another developer is editing")
        with self.assertRaisesRegex(publisher.Held, "clean"):
            publisher.validate_candidate(self.candidate, self.repo, self.review)

    def test_uncommitted_candidate_changes_are_held(self):
        (self.candidate / self.path).write_text("unreviewed change")
        with self.assertRaisesRegex(publisher.Held, "clean"):
            publisher.validate_candidate(self.candidate, self.repo, self.review)

    def test_root_must_be_on_manan(self):
        git(self.repo, "checkout", "-b", "other-developer")
        with self.assertRaisesRegex(publisher.Held, "Manan"):
            publisher.validate_candidate(self.candidate, self.repo, self.review)

    def test_stale_base_never_merges_into_new_root_work(self):
        target = self.repo / "docs" / "new.md"
        target.parent.mkdir()
        target.write_text("new unrelated work")
        git(self.repo, "add", ".")
        git(self.repo, "commit", "-m", "new work")
        with self.assertRaisesRegex(publisher.Held, "base"):
            publisher.validate_candidate(self.candidate, self.repo, self.review)

    def test_review_cannot_hide_unapproved_changed_file(self):
        self.review["approved_files"] = []
        with self.assertRaisesRegex(publisher.Held, "exact"):
            publisher.validate_candidate(self.candidate, self.repo, self.review)

    def test_source_and_candidate_hash_changes_are_rejected(self):
        self.source.write_text("changed source")
        with self.assertRaisesRegex(publisher.Held, "hash"):
            publisher.validate_review(self.review, self.candidate)
        self.review["source_hashes"][str(self.source)] = publisher.sha256(self.source)
        self.review["candidate_hashes"][self.path] = "0" * 64
        with self.assertRaisesRegex(publisher.Held, "hash"):
            publisher.validate_review(self.review, self.candidate)

    def test_unreviewed_flags_self_review_and_missing_trades_are_rejected(self):
        for changes in ({"source_reviewed": False}, {"ledger_checked": False}, {"checked_trades": 0},
                        {"row_mismatches": 1}, {"reviewer_id": "conversion-job"}):
            with self.subTest(changes=changes), self.assertRaises(publisher.Held):
                publisher.validate_review({**self.review, **changes}, self.candidate)

    def test_hashed_ledger_with_mismatches_is_rejected(self):
        payload = json.loads(self.ledger.read_text())
        payload["checks"][0]["row_mismatches"] = 1
        self.ledger.write_text(json.dumps(payload))
        self.review["ledger_reports"][0]["sha256"] = publisher.sha256(self.ledger)
        with self.assertRaisesRegex(publisher.Held, "ledger"):
            publisher.validate_review(self.review, self.candidate)

    def test_new_admissions_must_be_exact_approved_method_ids(self):
        base = [{"id": "old", "status": "live", "instrument_policy": {"allowed_types": ["index"]}},
                {"id": "test_rule", "status": "not_testable", "instrument_policy": {"allowed_types": []}}]
        candidate = [base[0], {"id": "test_rule", "status": "live", "instrument_policy": {"allowed_types": ["index"]}}]
        self.assertEqual(publisher.check_catalogue_changes(base, candidate, {"test_rule"}), {"test_rule"})
        candidate.append({"id": "unreviewed_rule", "status": "live", "instrument_policy": {"allowed_types": ["index"]}})
        with self.assertRaisesRegex(publisher.Held, "unapproved"):
            publisher.check_catalogue_changes(base, candidate, {"test_rule"})

    def test_no_live_method_can_disappear_in_batch(self):
        with self.assertRaisesRegex(publisher.Held, "removed"):
            publisher.check_catalogue_changes([{"id": "existing", "status": "live"}], [], {"test_rule"})

    def test_path_allowlist_rejects_config_data_auth_and_frontend(self):
        for path in ("../outside.rs", "Context/guru/frontend/react/src/main.tsx", ".env",
                     "Context/guru/backend/src/main.rs", "library/book/raw.pdf", "docs/dump.csv"):
            self.assertFalse(publisher.allowed_file(path), path)
        for path in (self.path, "Context/guru/backend/src/instrument_policy.rs", "docs/BOOK-REVIEW.md",
                     "library/author/scripts/reference.py"):
            self.assertTrue(publisher.allowed_file(path), path)

    def simulated_publication(self, *, deploy=True, fail_stage=None, fail_live=False, existing_method=False):
        """Mock release infrastructure, retain actual precondition Git histories."""
        review_path = self.root / "review.json"
        review_path.write_text(json.dumps(self.review))
        result_path = self.root / "result.json"
        runtime = self.root / "runtime"
        runtime.mkdir(exist_ok=True)
        env_path = self.root / "runtime.env"
        env_path.write_text("VAULT_RUNTIME_ROOT=" + str(runtime) + "\n")
        stages = []

        def command(argv, **kwargs):
            stage = kwargs.get("stage")
            stages.append(stage)
            if stage == fail_stage:
                raise publisher.Held("Infrastructure test failure")
            if stage == "previous-image":
                return "sha256:" + "a" * 64
            if stage == "focused-rust":
                return "running 1 test\ntest result: ok"
            return ""

        original_git = publisher.git
        def git_call(repo, *args):
            if args[0] == "ls-remote":
                return self.base + "\trefs/heads/Manan"
            return original_git(repo, *args)

        old = [{"id": "test_rule", "status": "not_testable", "instrument_policy": {"allowed_types": []}}]
        new = [{"id": "test_rule", "status": "live", "instrument_policy": {"allowed_types": ["index"]}}]
        if existing_method:
            old = new
        calls = 0
        def smoke(*_):
            nonlocal calls
            calls += 1
            if fail_live and calls > 2:
                raise publisher.Held("Live API test failure")
            return [{"method_id": "test_rule", "readiness": "ready"}]

        with mock.patch.object(publisher, "Commands") as commands, mock.patch.object(publisher, "git", side_effect=git_call), \
             mock.patch.object(publisher, "engine_ids", return_value={"india": "in-id", "us": "us-id"}), \
             mock.patch.object(publisher, "live_base", return_value="http://127.0.0.1:1"), \
             mock.patch.object(publisher, "temporary_engine", return_value="http://127.0.0.1:1"), \
             mock.patch.object(publisher, "catalogue", side_effect=[old, new, old, new, new, new]), \
             mock.patch.object(publisher, "smoke", side_effect=smoke), mock.patch.object(publisher, "wait_api"):
            commands.return_value.run.side_effect = command
            code = publisher.publish(self.candidate, self.repo, review_path, result_path, env_path, deploy=deploy)
        return code, json.loads(result_path.read_text()), stages

    def test_successful_deployment_counts_only_validated_ids(self):
        code, report, stages = self.simulated_publication()
        self.assertEqual(code, 0)
        self.assertTrue(report["source_uploaded"])
        self.assertTrue(report["deployed"])
        self.assertTrue(report["admitted"])
        self.assertEqual(report["admitted_method_ids"], ["test_rule"])
        self.assertEqual(report["admitted_method_count"], 1)
        self.assertIn("deploy-engines-only", stages)
        self.assertNotIn("restore-previous-engines", stages)

    def test_failed_runtime_validation_restores_engines_but_retains_source(self):
        code, report, stages = self.simulated_publication(fail_live=True)
        self.assertEqual(code, 1)
        self.assertTrue(report["source_uploaded"])
        self.assertTrue(report["rollback_restored"])
        self.assertFalse(report["admitted"])
        self.assertEqual(report["admitted_method_count"], 0)
        self.assertIn("restore-previous-engines", stages)

    def test_failed_push_does_not_touch_runtime(self):
        code, report, stages = self.simulated_publication(fail_stage="push-manan")
        self.assertEqual(code, 1)
        self.assertFalse(report["source_uploaded"])
        self.assertFalse(report["admitted"])
        self.assertNotIn("deploy-engines-only", stages)
        self.assertNotIn("restore-previous-engines", stages)

    def test_source_only_publication_requires_explicit_deployment_flag(self):
        code, report, stages = self.simulated_publication(deploy=False)
        self.assertEqual(code, 0)
        self.assertTrue(report["source_uploaded"])
        self.assertFalse(report["deployed"])
        self.assertFalse(report["admitted"])
        self.assertNotIn("deploy-engines-only", stages)

    def test_verified_existing_method_correction_can_publish_with_zero_new_family_count(self):
        code, report, stages = self.simulated_publication(existing_method=True)
        self.assertEqual(code, 0)
        self.assertTrue(report["admitted"])
        self.assertEqual(report["reviewed_variant_count"], 1)
        self.assertEqual(report["admitted_family_count"], 0)
        self.assertEqual(report["admitted_variant_count"], 0)

    def test_zero_matching_rust_tests_holds_publication(self):
        code, report, stages = self.simulated_publication(fail_stage="focused-rust")
        self.assertEqual(code, 1)
        self.assertEqual(report["failed_stage"], "real_data_tests")
        self.assertNotIn("push-manan", stages)

    def test_ready_and_incompatible_smoke_cases_check_actual_api_shape(self):
        ready = self.review["smoke_cases"][0]
        blocked = {**ready, "expected_readiness": "needs_data", "instrument_kind": "individual_stock"}
        rows = [{"id": "test_rule", "instrument_policy": {"allowed_types": ["index"]}}]
        responses = [(200, {"strategies": [{"id": "test_rule", "status": "ready"}]}),
                     (200, {"dates": ["2020-01-01"], "equity": [1], "trades": []}),
                     (200, {"strategies": [{"id": "test_rule", "status": "needs_data"}]}),
                     (400, "Required data unavailable")]
        with mock.patch.object(publisher, "get_json", side_effect=responses):
            results = publisher.smoke("http://example.invalid", [ready, blocked], rows)
        self.assertEqual(results[0]["trade_records"], 0)
        self.assertTrue(results[1]["blocked"])
        invalid = [responses[0], (200, {"dates": ["2020-01-01"], "equity": [], "trades": []})]
        with mock.patch.object(publisher, "get_json", side_effect=invalid):
            with self.assertRaisesRegex(publisher.Held, "equity"):
                publisher.smoke("http://example.invalid", [ready], rows)

    def test_catalogue_and_engine_loading_contract(self):
        with mock.patch.object(publisher, "get_json", return_value=(200, [{"id": "test_rule"}])):
            self.assertEqual(len(publisher.catalogue("http://example.invalid")), 3)
            publisher.wait_api("http://example.invalid")
        with mock.patch.object(publisher, "get_json", return_value=(500, "server error")):
            with self.assertRaises(publisher.Held):
                publisher.catalogue("http://example.invalid")

    def test_commands_keep_output_private_and_reject_failed_command(self):
        import sys
        commands = publisher.Commands(self.root / "private-command-logs")
        output = commands.run([sys.executable, "-c", "print('operational test output')"], stage="fixture")
        self.assertEqual(output, "operational test output")
        with self.assertRaises(publisher.Held):
            commands.run([sys.executable, "-c", "raise SystemExit(3)"], stage="failing-fixture")
        self.assertEqual((self.root / "private-command-logs" / "001-fixture.log").stat().st_mode & 0o777, 0o600)

    def test_temporary_engine_uses_read_only_real_input_locations_and_limits(self):
        runtime = self.root / "runtime"
        (runtime / "data").mkdir(parents=True)
        (runtime / "data" / "idx_data.csv").write_text("temporary operational file identity")
        basket = self.root / "basket"
        basket.mkdir()
        commands = mock.Mock()
        commands.run.side_effect = ["temporary-container", "12345"]
        with mock.patch.object(publisher, "wait_api"):
            base = publisher.temporary_engine(commands, "candidate-test", "india", {
                "VAULT_RUNTIME_ROOT": str(runtime), "VAULT_BASKET_DATA_ROOT": str(basket)}, "owned-temp")
        argv = commands.run.call_args_list[0].args[0]
        self.assertIn("768m", argv)
        self.assertIn("--read-only", argv)
        self.assertTrue(any(arg.endswith(":/data:ro") for arg in argv))
        self.assertEqual(base, "http://127.0.0.1:12345")

    def test_threshold_and_direction_variants_do_not_inflate_family_counts(self):
        old = [{"id": "old_long", "book": "book", "family": "existing", "status": "live"}]
        new = old + [
            {"id": "old_short", "book": "book", "family": "existing", "status": "live"},
            {"id": "new_long", "book": "book", "family": "new_method", "status": "live"},
            {"id": "new_short", "book": "book", "family": "new_method", "status": "live"}]
        counts = publisher.admission_counts(old, new)
        self.assertEqual(counts["admitted_variant_count"], 3)
        self.assertEqual(counts["admitted_family_count"], 1)
        self.assertEqual(counts["admitted_method_count"], 1)

    def test_changed_actual_data_or_missing_rust_identity_holds_ledger(self):
        self.ohlc.write_text("modified operational data identity")
        with self.assertRaisesRegex(publisher.Held, "market-data hash"):
            publisher.validate_review(self.review, self.candidate)
        payload = json.loads(self.ledger.read_text())
        payload["rust_module_sha256"] = {}
        self.ledger.write_text(json.dumps(payload))
        self.review["ledger_reports"][0]["sha256"] = publisher.sha256(self.ledger)
        with self.assertRaisesRegex(publisher.Held, "Rust source hashes"):
            publisher.validate_review(self.review, self.candidate)

    def test_unapproved_instrument_type_narrowing_is_held(self):
        before = [{"id": "existing", "status": "live", "instrument_policy": {"allowed_types": ["index", "individual_stock"]}}]
        after = [{"id": "existing", "status": "live", "instrument_policy": {"allowed_types": ["index"]}}]
        with self.assertRaisesRegex(publisher.Held, "allowed instrument types"):
            publisher.check_catalogue_changes(before, after, {"test_rule"})

    def regression_fixture(self):
        row = {"id": "existing", "book": "testbook", "status": "live", "_publisher_guru": "connors"}
        case = self.review["smoke_cases"][0]
        ready = (200, {"strategies": [{"id": "existing", "status": "ready"}]})
        run = {"dates": ["2020-01-01", "2020-01-02"], "trades": [{"ret": 0.1}],
               "equity": [1, 1.1], "stats": {"trades": 1}, "gross_equity": [1, 1.1],
               "gross_stats": {"trades": 1}, "ms": 1, "provenance": {"source": "old"}}
        return row, case, ready, run

    def test_unchanged_catalogue_cannot_hide_old_trade_ledger_mutation(self):
        row, case, ready, run = self.regression_fixture()
        changed = {**run, "trades": [{"ret": 0.2}]}
        with mock.patch.object(publisher, "get_json", side_effect=[ready, ready, (200, run), (200, changed)]):
            with self.assertRaisesRegex(publisher.Held, "trades, statistics or equity changed"):
                publisher.compare_existing_runs("old", "new", [row], [case], {"testbook"}, {"test_rule"})

    def test_regression_deduplicates_windows_ignores_timing_and_retains_hash_evidence(self):
        row, case, ready, run = self.regression_fixture()
        same = {**run, "ms": 99, "provenance": {"source": "new"}}
        duplicate = {**case, "method_id": "another-approved", "query": {**case["query"], "p.ma": "14"}}
        with mock.patch.object(publisher, "get_json", side_effect=[ready, ready, (200, run), (200, same)]) as api:
            report = publisher.compare_existing_runs("old", "new", [row], [case, duplicate], {"testbook"}, {"test_rule"})
        self.assertEqual(api.call_count, 4)
        self.assertEqual(report["context_count"], 1)
        self.assertEqual(report["checked_cases"], 1)
        self.assertEqual(report["checks"][0]["baseline_sha256"], report["checks"][0]["candidate_sha256"])
        self.assertEqual(len(report["checks"][0]["baseline_sha256"]), 64)

    def test_exact_approved_method_corrections_are_excluded_from_old_method_regression(self):
        row, case, ready, run = self.regression_fixture()
        with mock.patch.object(publisher, "get_json") as api:
            report = publisher.compare_existing_runs("old", "new", [row], [case], {"testbook"}, {"existing"})
        self.assertEqual(report["checked_cases"], 0)
        api.assert_not_called()

    def test_baseline_not_ready_cases_skip_runs_and_case_limit_holds(self):
        row, case, ready, run = self.regression_fixture()
        held = (200, {"strategies": [{"id": "existing", "status": "needs_data"}]})
        with mock.patch.object(publisher, "get_json", return_value=held) as api:
            report = publisher.compare_existing_runs("old", "new", [row], [case], {"testbook"}, set())
        self.assertEqual(report["skipped_not_ready"], 1)
        self.assertEqual(api.call_count, 1)
        with self.assertRaisesRegex(publisher.Held, "bounded case limit"):
            publisher.compare_existing_runs("old", "new", [row], [case], {"testbook"}, set(), max_cases=0)


if __name__ == "__main__":
    unittest.main()
