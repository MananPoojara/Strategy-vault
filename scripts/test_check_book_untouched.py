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

    def test_rename_out_of_protected_path_fails(self):
        src = self.repo / "Context/guru/backend/src"
        (src / "lab").mkdir()
        git(self.repo, "mv", "Context/guru/backend/src/books/x.rs", "Context/guru/backend/src/lab/x.rs")
        git(self.repo, "commit", "-q", "-m", "rename")
        self.assertTrue(any("books/x.rs" in v for v in guard.check(self.repo, "Manan")))

    def test_untracked_new_file_in_protected_path_fails(self):
        p = self.repo / "Context/guru/backend/src/books/new.rs"
        p.write_text("fn n() {}\n")
        self.assertTrue(any("books/new.rs" in v for v in guard.check(self.repo, "Manan")))


if __name__ == "__main__":
    unittest.main()
