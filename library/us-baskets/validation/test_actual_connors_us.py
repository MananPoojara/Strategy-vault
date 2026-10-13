"""Independent convention fixtures: entry/exit cost, hold and terminal marks."""
from datetime import date, timedelta
from decimal import Decimal
import unittest

from verify_actual_connors_us import oracle


def source(values):
    return [{"date": (date(1999, 1, 1) + timedelta(days=i)).isoformat(), "close": str(c)}
            for i, c in enumerate(values)]


class USConventionFixtures(unittest.TestCase):
    def test_initial_fee_is_retained_and_exit_cost_is_deducted_once(self):
        rows = source([100]*200 + [120, 121, 123, 124, 126, 132])
        trades, _, eq, gross = oracle(rows, "f1a", rows[200]["date"], rows[205]["date"])
        self.assertEqual(len(trades), 1)
        self.assertEqual(trades[0]["bars"], 5)
        self.assertFalse(trades[0]["open"])
        self.assertEqual(eq[0], Decimal("0.9997"))
        self.assertEqual(eq[-1], Decimal("1.0994"))
        self.assertEqual(gross[-1], Decimal("1.1"))
        self.assertEqual(trades[0]["return_value"], Decimal("0.0994"))

    def test_terminal_mark_is_open_and_keeps_liquidation_estimate_separate(self):
        rows = source([100]*200 + [120, 126, 132])
        trades, _, eq, _ = oracle(rows, "f1a", rows[200]["date"], rows[202]["date"])
        self.assertTrue(trades[0]["open"])
        self.assertEqual(trades[0]["bars"], 2)
        self.assertEqual(sum(t["return_value"] for t in trades if not t["open"]), 0)
        self.assertEqual(eq[-1], Decimal("1.0994"))

    def test_pullback_entry_is_independent_of_the_five_bar_exit(self):
        rows = source([100]*190 + list(range(120, 130)) + [119] + list(range(130, 140)))
        trades, _, _, _ = oracle(rows, "f3a", rows[200]["date"], rows[207]["date"])
        self.assertEqual(len(trades), 1)
        self.assertEqual(trades[0]["entry_price"], 119)
        self.assertEqual(trades[0]["exit_price"], 134)
        self.assertEqual(trades[0]["bars"], 5)

    def test_unreviewed_methods_cannot_inherit_a_source_oracle(self):
        rows = source([100]*220)
        with self.assertRaises(ValueError):
            oracle(rows, "new_stock_strategy", rows[0]["date"], rows[-1]["date"])


if __name__ == "__main__":
    unittest.main()
