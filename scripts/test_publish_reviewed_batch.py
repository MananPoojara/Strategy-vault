"""Publication gates use actual temporary Git histories; Docker remains isolated."""
import hashlib
import importlib.util
import json
from pathlib import Path
import subprocess
import tempfile
import unittest

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
        target.write_text("// baseline source\n")
        git(self.repo, "add", ".")
        git(self.repo, "commit", "-m", "baseline")
        self.base = git(self.repo, "rev-parse", "HEAD")
        self.candidate = self.root / "candidate"
        git(self.root, "clone", "--quiet", str(self.repo), str(self.candidate))
        git(self.candidate, "config", "user.email", "queue-test@example.invalid")
        git(self.candidate, "config", "user.name", "Queue test")
        (self.candidate / self.path).write_text("// reviewed source change\n")
        git(self.candidate, "add", ".")
        git(self.candidate, "commit", "-m", "reviewed source")
        self.commit = git(self.candidate, "rev-parse", "HEAD")
        self.source = self.root / "private-source.txt"
        self.source.write_text("temporary source identity")
        self.ledger = self.root / "private-ledger.json"
        self.ledger.write_text(json.dumps({"source_reviewed": True, "implementation_ready": True,
            "ledger_checked": True, "checked_trades": 2,
            "checks": [{"method": "test_rule", "row_mismatches": 0, "actual_records": 2}]}))
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


if __name__ == "__main__":
    unittest.main()
