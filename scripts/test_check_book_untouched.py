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

    def test_compare_base_is_branch_point_not_base_tip(self):
        branch_point = git(self.repo, "rev-parse", "Manan")
        git(self.repo, "switch", "-q", "Manan")
        self.commit("Context/guru/backend/src/lab/later.rs", "pub mod later;\n")
        git(self.repo, "switch", "-q", "Nandini")
        self.assertEqual(guard.compare_base(self.repo, "Manan"), branch_point)

    def test_preloaded_tickers_drops_basket_rows(self):
        rows = [{"ticker": "NIFTY 50"}, {"ticker": "BKT", "basket": ["A", "B"]}, {"ticker": "RELIANCE"}]
        self.assertEqual(guard.preloaded_tickers(rows), ["NIFTY 50", "RELIANCE"])

    def test_strategy_routes_cover_every_id_with_run_and_readiness(self):
        routes = guard.strategy_routes("connors", [{"id": "f1a"}, {"id": "r 3"}])
        self.assertEqual(routes, [
            ("connors/run?id=f1a", "run?id=f1a&ticker=NIFTY%2050"),
            ("connors/readiness?id=f1a", "readiness?id=f1a&ticker=NIFTY%2050"),
            ("connors/run?id=r 3", "run?id=r%203&ticker=NIFTY%2050"),
            ("connors/readiness?id=r 3", "readiness?id=r%203&ticker=NIFTY%2050"),
        ])

    def test_optimize_route_uses_first_live_strategy_with_params(self):
        catalogue = [{"id": "a", "status": "not_testable"}, {"id": "b", "status": "live"}, {"id": "c", "status": "live"}]
        params = {"a": [{"key": "x", "def": 1, "step": 1}], "b": [], "c": [{"key": "trend", "def": 200, "step": 25}, {"key": "hold", "def": 5, "step": 1}]}
        self.assertEqual(guard.optimize_route("connors", catalogue, params),
                         ("connors/optimize?id=c&g.trend=200:225:25", "optimize?id=c&ticker=NIFTY%2050&g.trend=200:225:25"))
        self.assertEqual(guard.optimize_route("chande", [{"id": "d", "status": "live"}], {"d": [{"key": "k", "def": 0.1, "step": 0.2}]})[0],
                         "chande/optimize?id=d&g.k=0.1:0.3:0.2")
        self.assertIsNone(guard.optimize_route("ross", catalogue, {}))

    def test_optimizer_timing_is_not_hashed(self):
        a = guard.stable("connors/optimize?id=c", b'{"id":"c","ms":12,"rows":[[1]]}')
        b = guard.stable("connors/optimize?id=c", b'{"rows":[[1]],"ms":95,"id":"c"}')
        self.assertEqual(a, b)
        self.assertEqual(guard.stable("connors/run?id=c", b'{"ms":1}'), b'{"ms":1}')
        self.assertEqual(guard.stable("connors/optimize?id=c", b"not json"), b"not json")

    def test_basket_tickers_are_bounded_and_sorted(self):
        rows = [{"ticker": "NIFTY 50"}, {"ticker": "basket:b:Z", "basket": "b"}, {"ticker": "basket:a:Y", "basket": "a"},
                {"ticker": "basket:a:X", "basket": "a"}, {"ticker": "basket:c:W", "basket": "c"}]
        self.assertEqual(guard.basket_tickers(rows), ["basket:a:X", "basket:a:Y", "basket:b:Z"])


if __name__ == "__main__":
    unittest.main()
