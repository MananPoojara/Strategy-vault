import contextlib
import csv
import hashlib
import io
from pathlib import Path
import struct
import tempfile
import unittest

import stage_yt_aux_cash as stage


def native(records):
    header = bytearray(1184)
    header[:8] = b"BROKDAt5"
    struct.pack_into("<I", header, 1180, len(records))
    data = bytearray(header)
    for day, close, aux1, aux2 in records:
        date = (2024 << 52) | (1 << 48) | (day << 43)
        data.extend(struct.pack("<Q8f", date, close, close, close + 1, close - 1, 100, 0, aux1, aux2))
    data.extend(b"\0" * 4)
    return bytes(data)


class StageTests(unittest.TestCase):
    def test_dated_flags_create_separate_source_qualified_baskets(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp); source = root / "source"; (source / "a").mkdir(parents=True)
            a = native([(2, 10, 1, 0), (3, 11, 0, 1)])
            (source / "a" / "A").write_bytes(a)
            with contextlib.redirect_stdout(io.StringIO()):
                stage.main(source, root / "output")
            with (root / "output/prices/index.csv").open() as f:
                rows = list(csv.DictReader(f))
            self.assertEqual({r["basket"] for r in rows}, {"nse500", "allfo"})
            self.assertTrue(all(r["source_sha256"] == hashlib.sha256(a).hexdigest() for r in rows))
            self.assertEqual((source / "a" / "A").read_bytes(), a)
            with self.assertRaises(SystemExit):
                stage.main(source, root / "output")

    def test_invalid_eligible_bar_or_duplicate_date_is_rejected(self):
        with self.assertRaisesRegex(ValueError, "invalid price"):
            stage.read_native(native([(2, 0, 1, 0)]))
        with self.assertRaisesRegex(ValueError, "duplicate"):
            stage.read_native(native([(2, 10, 1, 0), (2, 11, 1, 0)]))
        with self.assertRaisesRegex(ValueError, "record count"):
            stage.read_native(native([(2, 10, 1, 0)])[:-10])

    def test_source_containing_output_is_refused(self):
        with tempfile.TemporaryDirectory() as tmp:
            with self.assertRaises(SystemExit):
                stage.main(Path(tmp), Path(tmp) / "output")


if __name__ == "__main__":
    unittest.main()
