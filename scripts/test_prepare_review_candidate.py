"""Host Git handoff preserves candidate work while reviewers keep a read-only .git."""
import importlib.util
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest import mock

sys.path.insert(0, str(Path(__file__).parent))
spec = importlib.util.spec_from_file_location("prepare_review_candidate", Path(__file__).with_name("prepare_review_candidate.py"))
handoff = importlib.util.module_from_spec(spec)
spec.loader.exec_module(handoff)


def git(root, *args):
    return subprocess.check_output(["git", "-C", str(root), *args], stderr=subprocess.DEVNULL, text=True).strip()


class HostHandoffTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        self.repo = self.root / "repo"
        self.repo.mkdir()
        git(self.repo, "init", "-b", "Manan")
        git(self.repo, "config", "user.email", "handoff@example.invalid")
        git(self.repo, "config", "user.name", "Handoff tests")
        self.book = "Context/guru/backend/src/books/testbook.rs"
        path = self.repo / self.book
        path.parent.mkdir(parents=True)
        path.write_text('pub const BOOK: &str = "testbook";\n// old source\n')
        (self.repo / ".gitignore").write_text("ignored-cache/\n")
        installer = self.repo / "scripts/install-hooks.sh"
        installer.parent.mkdir()
        installer.write_text("#!/bin/sh\ngit config core.hooksPath .githooks\n")
        git(self.repo, "add", ".")
        git(self.repo, "commit", "-m", "base")
        self.initial = git(self.repo, "rev-parse", "HEAD")
        self.candidate = self.root / "candidate"
        git(self.root, "clone", "--quiet", str(self.repo), str(self.candidate))
        git(self.candidate, "config", "user.email", "handoff@example.invalid")
        git(self.candidate, "config", "user.name", "Handoff tests")
        self.artifacts = self.root / "private"
        self.artifacts.mkdir()
        self.result = self.artifacts / "prepare.json"
        self.new = "library/testbook/scripts/new_reference.py"

    def tearDown(self):
        self.temp.cleanup()

    def conversion_edits(self):
        (self.candidate / self.book).write_text('pub const BOOK: &str = "testbook";\n// reviewed conversion\n')
        path = self.candidate / self.new
        path.parent.mkdir(parents=True)
        path.write_text("# interpreted reference code\n")
        ignored = self.candidate / "ignored-cache" / "work.txt"
        ignored.parent.mkdir()
        ignored.write_text("Preserve even ignored working material")

    def advance_root(self, *, conflicting=False):
        path = self.repo / self.book if conflicting else self.repo / "docs" / "new-host-tool.md"
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text("// concurrent source change\n" if conflicting else "new host tooling\n")
        git(self.repo, "add", ".")
        git(self.repo, "commit", "-m", "root advances")
        return git(self.repo, "rev-parse", "HEAD")

    def review(self):
        files = [self.book, self.new]
        return {"schema_version": 1, "reviewer_id": "review-job", "conversion_session_id": "conversion-job",
                "candidate_commit": "", "base_commit": git(self.repo, "rev-parse", "HEAD"),
                "source_reviewed": True, "implementation_ready": True, "ledger_checked": True,
                "checked_trades": 2, "approved_files": files,
                "candidate_hashes": {name: handoff.publisher.sha256(self.candidate / name) for name in files},
                "evidence": [str(self.result)]}

    def test_prepare_refreshes_base_and_preserves_tracked_untracked_and_ignored_work(self):
        self.conversion_edits()
        current = self.advance_root()
        report = handoff.prepare(self.repo, self.candidate, self.result)
        self.assertEqual(report["status"], "prepared")
        self.assertEqual(git(self.candidate, "rev-parse", "HEAD"), current)
        self.assertEqual((self.candidate / self.new).read_text(), "# interpreted reference code\n")
        self.assertIn("reviewed conversion", (self.candidate / self.book).read_text())
        backup = Path(report["original_backup"])
        self.assertEqual((backup / "ignored-cache/work.txt").read_text(), "Preserve even ignored working material")
        self.assertEqual(git(backup, "rev-parse", "HEAD"), self.initial)
        self.assertTrue((self.candidate / "docs/new-host-tool.md").is_file())
        self.assertFalse(git(self.repo, "status", "--porcelain"))
        self.assertEqual(git(self.candidate, "config", "core.hooksPath"), ".githooks")

    def test_conflict_holds_without_changing_original_candidate(self):
        self.conversion_edits()
        original = (self.candidate / self.book).read_bytes()
        self.advance_root(conflicting=True)
        report = handoff.prepare(self.repo, self.candidate, self.result)
        self.assertEqual(report["status"], "held")
        self.assertEqual((self.candidate / self.book).read_bytes(), original)
        self.assertEqual(git(self.candidate, "rev-parse", "HEAD"), self.initial)
        self.assertTrue(Path(report["original_backup"]).is_dir())
        self.assertTrue(Path(report["conflict_candidate"]).is_dir())

    def test_unapproved_edit_holds_before_candidate_mutation(self):
        self.conversion_edits()
        (self.candidate / "unauthorized.txt").write_text("outside allowed scope")
        report = handoff.prepare(self.repo, self.candidate, self.result)
        self.assertEqual(report["status"], "held")
        self.assertEqual(git(self.candidate, "rev-parse", "HEAD"), self.initial)
        self.assertTrue((self.candidate / self.new).is_file())

    def test_dirty_authoritative_repo_is_never_changed(self):
        self.conversion_edits()
        (self.repo / self.book).write_text("developer editing")
        report = handoff.prepare(self.repo, self.candidate, self.result)
        self.assertEqual(report["status"], "held")
        self.assertEqual((self.repo / self.book).read_text(), "developer editing")

    def test_finalize_validates_exact_hashes_before_one_commit(self):
        self.conversion_edits()
        handoff.prepare(self.repo, self.candidate, self.result)
        review_path = self.artifacts / "independent-review.json"
        review_path.write_text(json.dumps(self.review()))
        with mock.patch.object(handoff.publisher, "validate_review") as validate:
            report = handoff.finalize(self.repo, self.candidate, review_path, self.artifacts / "finalize.json")
        self.assertEqual(report["status"], "finalized")
        validate.assert_called()
        final = json.loads(review_path.read_text())
        self.assertEqual(final["candidate_commit"], git(self.candidate, "rev-parse", "HEAD"))
        self.assertEqual(git(self.candidate, "rev-parse", "HEAD^"), final["base_commit"])
        self.assertFalse(git(self.candidate, "status", "--porcelain"))
        self.assertEqual(git(self.repo, "rev-parse", "HEAD"), self.initial)

    def test_extra_untracked_file_cannot_enter_final_commit(self):
        self.conversion_edits()
        handoff.prepare(self.repo, self.candidate, self.result)
        review_path = self.artifacts / "independent-review.json"
        review_path.write_text(json.dumps(self.review()))
        extra = self.candidate / "library/testbook/EXTRA.md"
        extra.write_text("unreviewed source note")
        with mock.patch.object(handoff.publisher, "validate_review"):
            report = handoff.finalize(self.repo, self.candidate, review_path, self.artifacts / "finalize.json")
        self.assertEqual(report["status"], "held")
        self.assertEqual(git(self.candidate, "rev-parse", "HEAD"), self.initial)
        self.assertEqual(json.loads(review_path.read_text())["candidate_commit"], "")

    def test_failed_independent_source_gate_never_creates_commit(self):
        self.conversion_edits()
        handoff.prepare(self.repo, self.candidate, self.result)
        review_path = self.artifacts / "independent-review.json"
        review_path.write_text(json.dumps(self.review()))
        with mock.patch.object(handoff.publisher, "validate_review", side_effect=handoff.publisher.Held("Source hash mismatch")):
            report = handoff.finalize(self.repo, self.candidate, review_path, self.artifacts / "finalize.json")
        self.assertEqual(report["status"], "held")
        self.assertEqual(git(self.candidate, "rev-parse", "HEAD"), self.initial)

    def test_symlink_to_authoritative_git_metadata_is_rejected_without_root_write(self):
        import shutil
        shutil.rmtree(self.candidate / ".git")
        (self.candidate / ".git").symlink_to(self.repo / ".git", target_is_directory=True)
        before = (self.repo / ".git/config").read_bytes()
        report = handoff.prepare(self.repo, self.candidate, self.result)
        self.assertEqual(report["status"], "held")
        self.assertEqual((self.repo / ".git/config").read_bytes(), before)

    def test_concurrent_candidate_edit_holds_before_atomic_swap(self):
        self.conversion_edits()
        original = handoff.install_hooks
        def during_preparation(stage):
            original(stage)
            (self.candidate / self.new).write_text("# concurrent latest work\n")
        with mock.patch.object(handoff, "install_hooks", side_effect=during_preparation):
            report = handoff.prepare(self.repo, self.candidate, self.result)
        self.assertEqual(report["status"], "held")
        self.assertIn("changed during preparation", report["error"])
        self.assertEqual((self.candidate / self.new).read_text(), "# concurrent latest work\n")
        self.assertEqual(git(self.candidate, "rev-parse", "HEAD"), self.initial)

    def test_parent_symlink_cannot_supply_allowed_source_file(self):
        target = self.root / "outside-source"
        target.mkdir()
        (target / "reference.py").write_text("outside file")
        path = self.candidate / "library" / "linked"
        path.parent.mkdir()
        path.symlink_to(target, target_is_directory=True)
        report = handoff.prepare(self.repo, self.candidate, self.result)
        self.assertEqual(report["status"], "held")

    def test_ignored_work_never_copies_through_a_new_base_symlink(self):
        self.conversion_edits()
        outside = self.root / "outside-destination"
        outside.mkdir()
        (self.repo / "ignored-cache").symlink_to(outside, target_is_directory=True)
        git(self.repo, "add", "-f", "ignored-cache")
        git(self.repo, "commit", "-m", "new base symlink")
        report = handoff.prepare(self.repo, self.candidate, self.result)
        self.assertEqual(report["status"], "held")
        self.assertFalse((outside / "work.txt").exists())
        self.assertEqual((self.candidate / "ignored-cache/work.txt").read_text(), "Preserve even ignored working material")


if __name__ == "__main__":
    unittest.main()
