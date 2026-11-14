import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPT = Path(__file__).with_name('check_share_progress.py')

class ShareProgressCheck(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.share = Path(self.tmp.name) / 'share.json'
        self.progress = Path(self.tmp.name) / 'progress.json'
        self.p = {'run': 'run-1', 'as_of': 'date-1', 'totals': {'strategies': 3, 'verified': 2, 'held': 1, 'unreviewed': 0}, 'strategies': [{}, {}, {}]}
        self.s = {'claude': {'run': 'run-1', 'as_of': 'date-1', 'strategies': 3, 'backtested': 2, 'held': 1, 'unreviewed': 0, 'unmatched': {'strategies': 1, 'backtested': 1, 'held': 0, 'unreviewed': 0}}, 'sources': [{'strategies': 2, 'backtested': 1, 'held': 1, 'unreviewed': 0}]}

    def check(self):
        self.share.write_text(json.dumps(self.s))
        self.progress.write_text(json.dumps(self.p))
        return subprocess.run([sys.executable, str(SCRIPT), str(self.share), str(self.progress)], capture_output=True, text=True)

    def test_matching(self):
        self.assertEqual(self.check().returncode, 0)

    def test_same_counts_different_run_fails(self):
        self.s['claude']['run'] = 'old'
        self.assertNotEqual(self.check().returncode, 0)

    def test_different_date_fails(self):
        self.s['claude']['as_of'] = 'old'
        self.assertNotEqual(self.check().returncode, 0)

    def test_stale_count_fails(self):
        self.s['claude']['backtested'] = 1
        self.assertNotEqual(self.check().returncode, 0)

    def test_bad_attribution_fails(self):
        self.s['sources'][0]['held'] = 0
        self.assertNotEqual(self.check().returncode, 0)

    def test_missing_field_fails(self):
        del self.s['claude']['as_of']
        self.assertNotEqual(self.check().returncode, 0)

    def test_missing_input_fails(self):
        result = subprocess.run([sys.executable, str(SCRIPT), str(self.share), str(self.progress)], capture_output=True)
        self.assertNotEqual(result.returncode, 0)

if __name__ == '__main__':
    unittest.main()
