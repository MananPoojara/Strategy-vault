"""Unit tests for build_research_work.py (stdlib only): python3 -m unittest Context/guru/webapp/scripts/test_build_research_work.py"""
import os
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import build_research_work as b  # noqa: E402

DIFF = """diff --git a/Context/guru/backend/src/books/ytkr.rs b/Context/guru/backend/src/books/ytkr.rs
new file mode 100644
--- /dev/null
+++ b/Context/guru/backend/src/books/ytkr.rs
@@ -0,0 +1,6 @@
+pub const BOOK: &str = "yt_kr";
+pub const LONG: &str = "ytkr_long";
+pub const SHORT_V2: &str = "ytkr_short_v2";
+    pub const INDENTED: &str = "ytkr_not_top_level";
+const PRIVATE: &str = "ytkr_private";
+pub const SERIES: &str = "ADVANCES";
diff --git a/Context/guru/backend/src/books/ytbb.rs b/Context/guru/backend/src/books/ytbb.rs
--- a/Context/guru/backend/src/books/ytbb.rs
+++ b/Context/guru/backend/src/books/ytbb.rs
@@ -10,2 +10,2 @@
-pub const PLAIN: &str = "ytbb_plain";
+pub const PLAIN: &str = "ytbb_plain";
 pub const V1: &str = "ytbb_v1";
-pub const OLD: &str = "ytbb_old";
"""


class IdDetection(unittest.TestCase):
    def test_added_lines_only(self):
        # Book ids ("yt_..."), indented, private, removed and context lines are not counted; a re-added line is.
        self.assertEqual(b.added_ids_in_diff(DIFF), {"ytkr_long", "ytkr_short_v2", "ytbb_plain"})

    def test_plus_header_is_not_a_line(self):
        self.assertEqual(b.added_ids_in_diff('+++ b/pub const X: &str = "ytx_y";\n'), set())

    def test_tree_lines(self):
        lines = ['pub const A: &str = "ytadr_basic"', 'pub const BOOK: &str = "yt_adr"', 'pub const A2: &str = "ytadr_basic"']
        self.assertEqual(b.ids_in_lines(lines), {"ytadr_basic"})

    def test_trailers(self):
        work, names = b.split_trailers("- one\n- two\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>")
        self.assertEqual((work, names), ("- one\n- two", ["Claude Opus 5.5"]))

    def test_day_is_india_time(self):
        self.assertEqual(b.ist_day("2026-11-19T20:00:00+00:00"), "2026-11-20")
        self.assertEqual(b.ist_day("2026-11-20T09:00:00+05:30"), "2026-11-20")


if __name__ == "__main__":
    unittest.main()
