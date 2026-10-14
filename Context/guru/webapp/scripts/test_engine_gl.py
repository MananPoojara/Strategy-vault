import csv, re, tempfile, unittest
from pathlib import Path

import stage_gl

REPO = Path(__file__).resolve().parents[4]


def write(root, rel, text):
    path = Path(root) / rel
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text)


class StageGl(unittest.TestCase):
    def test_exports_with_each_header_style_become_one_engine_csv_without_volume(self):
        with tempfile.TemporaryDirectory() as src, tempfile.TemporaryDirectory() as out:
            files = dict(stage_gl.MANIFEST)
            write(src, files["HSI:HSI"], "Ticker,time,open,high,low,close,Volume\nHANG SENG,02-01-1987,2540.1001,2541,2539,2540.5,\nHANG SENG,31-12-1986,2568.3,2568.3,2568.3,2568.3,\n")
            write(src, files["NASDAQ_DLY:IXIC"], "time,open,high,low,close\n1971-02-05,100,100,100,100\n")
            write(src, files["TVC:RUT"], "Date,time,open,high,low,close\nUS-SMALL CAP 2000_RUSSELL 2000,29-06-1987,164.96999,164.96999,164.96999,164.96999\n")
            write(src, files["BSE_DLY:SENSEX"], "Ticker,Date,open,high,low,close\nSENSEX,03-04-1979,124.15,124.15,124.15,124.15\n")
            write(src, files["IG:DAX"], "Ticker,time,open,high,low,close,Volume\nDAX 40,02-01-1995,2091.47,2092.33,2078.46,2079.19,9063\n")
            notes = stage_gl.stage(src, Path(out) / "gl" / "gl_data.csv")
            with open(Path(out) / "gl" / "gl_data.csv", newline="") as f:
                rows = list(csv.reader(f))
        self.assertEqual(rows[0], ["Ticker", "Date", "Open", "High", "Low", "Close", "Volume"])
        self.assertEqual([r for r in rows if r[0] == "HSI:HSI"], [["HSI:HSI", "1986-12-31", "2568.3", "2568.3", "2568.3", "2568.3", ""],
                                     ["HSI:HSI", "1987-01-02", "2540.1001", "2541", "2539", "2540.5", ""]])
        self.assertIn(["NASDAQ_DLY:IXIC", "1971-02-05", "100", "100", "100", "100", ""], rows)
        self.assertIn(["TVC:RUT", "1987-06-29", "164.96999", "164.96999", "164.96999", "164.96999", ""], rows)
        self.assertIn(["BSE_DLY:SENSEX", "1979-04-03", "124.15", "124.15", "124.15", "124.15", ""], rows)
        self.assertIn(["IG:DAX", "1995-01-02", "2091.47", "2092.33", "2078.46", "2079.19", ""], rows, "CFD tick volume is not staged")
        self.assertIn("HSI:HSI: 2 bars 1986-12-31 to 1987-01-02", notes)
        self.assertTrue(any(n.startswith("CRYPTO:BTCUSD: missing ") for n in notes))

    def test_a_bad_row_or_duplicate_date_leaves_the_whole_instrument_out(self):
        with tempfile.TemporaryDirectory() as src, tempfile.TemporaryDirectory() as out:
            files = dict(stage_gl.MANIFEST)
            write(src, files["TVC:CAC40"], "Ticker,time,open,high,low,close\nCAC 40,31-12-1987,1007.45,1007.45,1000,1000\nCAC 40,04-01-1988,0,984.6,984.6,984.6\n")
            write(src, files["CRYPTO:ETHUSD"], "Ticker,time,open,high,low,close\nETHEREUM,07-08-2015,3,3,3,3\nETHEREUM,07-08-2015,3,3,3,3\n")
            write(src, files["IG:FTSE"], "Ticker,time,open,high,low,close\nFTSE 100,2015-13-40,1,1,1,1\n")
            write(src, files["SPREADEX:NIKKEI"], "Ticker,when,open,high,low,close\nNIKKEI 225,23-05-2016,1,1,1,1\n")
            notes = "\n".join(stage_gl.stage(src, Path(out) / "gl_data.csv"))
            text = (Path(out) / "gl_data.csv").read_text()
        self.assertEqual(text.strip(), "Ticker,Date,Open,High,Low,Close,Volume")
        self.assertIn("TVC:CAC40: not staged (line 3 ", notes)
        self.assertIn("CRYPTO:ETHUSD: not staged (duplicate dates, first 2015-08-07)", notes)
        self.assertIn("IG:FTSE: not staged (line 2 ", notes)
        self.assertIn("SPREADEX:NIKKEI: not staged (header ", notes)

    def test_no_source_data_still_writes_a_loadable_empty_file(self):
        with tempfile.TemporaryDirectory() as src, tempfile.TemporaryDirectory() as out:
            notes = stage_gl.stage(src, Path(out) / "gl_data.csv")
            self.assertEqual((Path(out) / "gl_data.csv").read_text().strip(), "Ticker,Date,Open,High,Low,Close,Volume")
        self.assertEqual(len(notes), len(stage_gl.MANIFEST))
        self.assertTrue(all("needs_data" in n for n in notes))

    def test_manifest_matches_the_engine_instrument_table(self):
        rust = (REPO / "Context/guru/backend/src/lab/markets.rs").read_text()
        declared = re.findall(r'GlInstrument \{ ticker: "([^"]+)"', rust)
        self.assertEqual(sorted(t for t, _ in stage_gl.MANIFEST), sorted(declared))
        sources = " ".join(rel for _, rel in stage_gl.MANIFEST)
        for left_out in ("SPCFD_SPX", "DJCFD_DJI", "NASDAQ_DLY_NDX", "RUS_MOEX", "New folder/NASDAQ_DLY_IXIC"):
            self.assertNotIn(left_out, sources, "admitted US indices are not duplicated; unresolved identities are not staged")


class EngineGlCompose(unittest.TestCase):
    def setUp(self):
        import yaml  # PyYAML; the phase gate machine has it
        self.compose = yaml.safe_load((REPO / "Context/guru/webapp/compose.yaml").read_text())
        self.services = self.compose["services"]

    def test_engine_gl_reuses_the_engine_image_with_only_staged_gl_data_read_only(self):
        gl, us = self.services["engine-gl"], self.services["engine-us"]
        self.assertEqual(self.compose["name"], "strategy-vault")
        self.assertEqual(gl["image"], us["image"], "same engine image; no fork of strategy calculations")
        self.assertNotIn("build", gl)
        self.assertNotIn("ports", gl, "reachable only through the gateway")
        self.assertEqual(gl["environment"], {"VAULT_LAB_MARKET": "gl"})
        self.assertEqual(gl["command"][:2], ["/data/gl/gl_data.csv", "8090"])
        self.assertEqual(gl["volumes"], ["${VAULT_RUNTIME_ROOT}/data/gl:/data/gl:ro"])
        self.assertLessEqual(gl["cpus"], 2)
        self.assertIn("mem_limit", gl)

    def test_gateway_routes_gl_to_engine_gl(self):
        gateway = self.services["gateway"]
        self.assertEqual(gateway["environment"]["VAULT_ENGINE_GL"], "http://engine-gl:8090")
        self.assertEqual(gateway["depends_on"]["engine-gl"], {"condition": "service_started"})

    def test_setup_stages_gl_data_before_fixing_permissions(self):
        setup = (REPO / "Context/guru/webapp/scripts/setup.py").read_text()
        self.assertLess(setup.index("stage_gl.stage(root, data/'gl'/'gl_data.csv')"), setup.index("for item in [data, *data.rglob('*')]"))


if __name__ == "__main__":
    unittest.main()
