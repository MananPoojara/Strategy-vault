import unittest
from datetime import date, datetime

import polars as pl

import btst_series as b


def row(d, hh, mm, o, h, l, c, exp=date(2019, 4, 25)):
    return (datetime(*d, hh, mm), exp, o, h, l, c)


def frame(rows):
    return pl.DataFrame(rows, schema=["ts", "expiry", "open", "high", "low", "close"], orient="row")


class T(unittest.TestCase):
    def setUp(self):
        d = (2019, 2, 11)
        rows = [row(d, 9, 15 + i, 100 + i, 110 + i, 90 + i, 101 + i) for i in range(5)]
        # 09:20 bar: minutes 09:20, 09:21, 09:23 (09:22 and 09:24 missing)
        rows += [row(d, 9, 20, 200, 205, 195, 201), row(d, 9, 21, 201, 230, 190, 202), row(d, 9, 23, 203, 210, 199, 204)]
        rows += [row(d, 15, 30, 300, 301, 299, 300.5)]
        rows += [row((2019, 2, 12), 9, 15, 1, 2, 0.5, 1.5)]
        # Muhurat evening session
        rows += [row((2019, 11, 27), 18, 15, 5, 6, 4, 5), row((2019, 11, 27), 19, 0, 5, 6, 4, 5)]
        self.df = frame(rows)
        self.wide, self.info = b.build_bars(self.df)

    def test_header(self):
        self.assertEqual(self.wide.columns[:5], ["Date", "O0915", "H0915", "L0915", "C0915"])
        self.assertEqual(self.wide.columns[-4:], ["O1530", "H1530", "L1530", "C1530"])
        self.assertEqual(len(self.wide.columns), 305)

    def test_full_bar(self):
        r = self.wide.row(0, named=True)
        self.assertEqual(r["O0915"], 100)
        self.assertEqual(r["H0915"], 114)
        self.assertEqual(r["L0915"], 90)
        self.assertEqual(r["C0915"], 105)  # close of minute 09:19

    def test_partial_bar(self):
        r = self.wide.row(0, named=True)
        self.assertEqual((r["O0920"], r["H0920"], r["L0920"], r["C0920"]), (200, 230, 190, 204))
        self.assertEqual(self.info["partial"].height, 2)  # 09:20 on day 1, lone 09:15 minute on day 2

    def test_missing_bar_empty(self):
        self.assertIsNone(self.wide.row(0, named=True)["O0925"])

    def test_lone_1530(self):
        r = self.wide.row(0, named=True)
        self.assertEqual((r["O1530"], r["H1530"], r["L1530"], r["C1530"]), (300, 301, 299, 300.5))

    def test_evening_excluded(self):
        self.assertEqual(self.wide.height, 2)
        self.assertEqual(self.info["excluded_dates"], [date(2019, 11, 27)])
        rep = b.make_report(self.wide, self.info, self.df)
        self.assertEqual(rep["excluded_dates"], ["2019-11-27"])
        self.assertEqual(rep["days_written"], 2)


if __name__ == "__main__":
    unittest.main()
