"""Unit tests for build_research_tradesheets.py on tiny synthetic workbooks (stdlib only):
python3 -m unittest Context/guru/webapp/scripts/test_build_research_tradesheets.py"""
import os
import sys
import tempfile
import unittest
import zipfile
from xml.sax.saxutils import escape

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import build_research_tradesheets as b  # noqa: E402


def col(i: int) -> str:
    s = ""
    i += 1
    while i:
        i, r = divmod(i - 1, 26)
        s = chr(65 + r) + s
    return s


def make_xlsx(path: str, sheets: dict) -> None:
    """A minimal .xlsx: strings as shared strings for the first sheet, inline strings for the others, numbers as numbers."""
    shared: list[str] = []
    names = list(sheets)
    with zipfile.ZipFile(path, "w") as z:
        z.writestr("[Content_Types].xml", '<?xml version="1.0"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"/>')
        z.writestr("xl/workbook.xml", '<?xml version="1.0"?><workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" '
                   'xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets>'
                   + "".join(f'<sheet name="{escape(n)}" sheetId="{i + 1}" r:id="rId{i + 1}"/>' for i, n in enumerate(names)) + "</sheets></workbook>")
        z.writestr("xl/_rels/workbook.xml.rels", '<?xml version="1.0"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">'
                   + "".join(f'<Relationship Id="rId{i + 1}" Type="ws" Target="worksheets/sheet{i + 1}.xml"/>' for i in range(len(names))) + "</Relationships>")
        for si, n in enumerate(names):
            xml = []
            for ri, row in enumerate(sheets[n]):
                cells = []
                for ci, v in enumerate(row):
                    ref = f"{col(ci)}{ri + 1}"
                    if v is None:
                        continue
                    if isinstance(v, (int, float)):
                        cells.append(f'<c r="{ref}"><v>{v}</v></c>')
                    elif si == 0:
                        shared.append(v)
                        cells.append(f'<c r="{ref}" t="s"><v>{len(shared) - 1}</v></c>')
                    else:
                        cells.append(f'<c r="{ref}" t="inlineStr"><is><t>{escape(v)}</t></is></c>')
                xml.append(f'<row r="{ri + 1}">{"".join(cells)}</row>')
            z.writestr(f"xl/worksheets/sheet{si + 1}.xml", '<?xml version="1.0"?><worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">'
                       f'<sheetData>{"".join(xml)}</sheetData></worksheet>')
        z.writestr("xl/sharedStrings.xml", '<?xml version="1.0"?><sst xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">'
                   + "".join(f"<si><t>{escape(s)}</t></si>" for s in shared) + "</sst>")


BY = ["Category", "Strategy id", "Strategy", "Trade-sheet status", "Detail"]
TS = ["Category", "Workbook", "Trade sheet", "Strategy id", "Team trades", "Our trades", "Matched / found", "Result", "Class", "Note", "Source"]


def match_book(path, made, rows, sheets=()):
    make_xlsx(path, {"By strategy": [BY, *rows, [None] * 5], "Trade sheets": [TS, *sheets], "Notes": [["Made", made]]})


class Parser(unittest.TestCase):
    def setUp(self):
        self.dir = tempfile.TemporaryDirectory()
        self.p = lambda n: os.path.join(self.dir.name, n)

    def tearDown(self):
        self.dir.cleanup()

    def test_reads_rows_counts_and_jay(self):
        match_book(self.p("m.xlsx"), "19-11-2026 (first)", [
            ["Inside Bar", "ytib_a", "IB A", "Matching", "2 of 2"],
            ["Inside Bar", "ytib_b", "IB B", "Partly matching", "1 of 2"],
            ["Sniper", "ytsn_x", "Sniper X", "Not compared yet", ""],
            ["Sniper", "ytsn_old", "Sniper old", "Not matching", ""],
        ], [["Inside Bar", "w.xlsx", "s1", "ytib_b", 10, 10, 10, "Match", "Matching", "", ""],
            ["Inside Bar", "w.xlsx", "s2", "ytib_b", 10, 12, 8, "No match", "Not matching", "", ""],
            ["Inside Bar", "w.xlsx", "s3", "ytib_b", 0, 0, 0, "Not compared", "Not compared", "", ""]])
        make_xlsx(self.p("s.xlsx"), {"Complete": [["Strategy", "Old Topics folder", "Code (module / main id)"],
                                                  ["Inside Bar", "Inside Bar", "ytib.rs"], ["Sniper System", "Sniper", "ytsn.rs (ytsn_old)"]],
                                     "Notes": [["Made", "11-11-2026"]]})
        data, hist = b.build(self.p("m.xlsx"), self.p("s.xlsx"), [], [])
        st = {s["id"]: s for s in data["strategies"]}
        self.assertEqual(len(st), 4)  # the empty trailing row is skipped
        self.assertEqual(data["totals"]["Matching"], 1)
        self.assertEqual(data["totals"]["Partly matching"], 1)
        self.assertEqual((st["ytib_b"]["sheets"], st["ytib_b"]["matched"]), (2, 1))
        # Module-only row: its Matching ids; bracketed ids: exactly those.
        self.assertEqual({i for i, s in st.items() if s["verified_by_jay"]}, {"ytib_a", "ytsn_old"})
        self.assertEqual(data["source"]["date"], "2026-11-19")
        self.assertEqual(len(hist), 1)

    def test_history_seed_and_changes(self):
        match_book(self.p("old.xlsx"), "18-11-2026", [["C", "ytx_a", "A", "Not compared yet", ""]])
        match_book(self.p("new.xlsx"), "20-11-2026", [["C", "ytx_a", "A", "Matching", ""], ["C", "ytx_b", "B", "No trade sheets", ""]])
        data, hist = b.build(self.p("new.xlsx"), None, [self.p("old.xlsx")], [])
        self.assertEqual([h["date"] for h in data["history"]], ["2026-11-18", "2026-11-20"])
        self.assertEqual(data["history"][1]["changes"], [{"id": "ytx_a", "name": "A", "from": "Not compared yet", "to": "Matching"},
                                                        {"id": "ytx_b", "name": "B", "from": None, "to": "No trade sheets"}])
        # Running again with the same workbooks adds nothing; a changed workbook of the same date replaces the run.
        _, hist2 = b.build(self.p("new.xlsx"), None, [self.p("old.xlsx")], hist)
        self.assertEqual(len(hist2), 2)
        match_book(self.p("new.xlsx"), "20-11-2026", [["C", "ytx_a", "A", "Not matching", ""]])
        data3, hist3 = b.build(self.p("new.xlsx"), None, [self.p("old.xlsx")], hist2)
        self.assertEqual(len(hist3), 2)
        self.assertEqual(data3["history"][-1]["counts"]["Not matching"], 1)

    def test_made_date_falls_back_to_mtime(self):
        match_book(self.p("m.xlsx"), "", [["C", "ytx_a", "A", "Matching", ""]])
        date, how = b.made_date("", self.p("m.xlsx"))
        self.assertEqual(how, "file modified time")
        self.assertRegex(date, r"^\d{4}-\d{2}-\d{2}$")


if __name__ == "__main__":
    unittest.main()
