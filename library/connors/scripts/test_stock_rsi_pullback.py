"""Independent arithmetic and execution-boundary fixtures for the diagnostic oracle."""
from datetime import date, timedelta
from decimal import Decimal
import unittest

from verify_stock_rsi_pullback import moving_average, wilder_rsi2, diagnostic_trades


def rows(closes):
    return [{"date": (date(2019, 1, 1) + timedelta(days=i)).isoformat(), "close": str(value)}
            for i, value in enumerate(closes)]


class StockOracleTests(unittest.TestCase):
    def test_direct_sma_has_exact_window_and_no_partial_warmup(self):
        self.assertEqual(moving_average(list(map(Decimal, [1, 2, 3, 4])), 3), [None, None, Decimal(2), Decimal(3)])

    def test_wilder_seed_and_flat_up_down_edges(self):
        self.assertEqual(wilder_rsi2(list(map(Decimal, [100, 100, 100])))[2], 50)
        self.assertEqual(wilder_rsi2(list(map(Decimal, [100, 101, 102])))[2], 100)
        self.assertEqual(wilder_rsi2(list(map(Decimal, [100, 99, 98])))[2], 0)
        values = wilder_rsi2(list(map(Decimal, [100, 102, 100, 102])))
        self.assertEqual(values[2], 50)
        self.assertEqual(values[3], 75)

    def test_ten_close_low_above_sma_exits_five_observed_bars_later(self):
        close = [100] * 190 + list(range(120, 130)) + [119] + list(range(130, 140))
        source = rows(close)
        trades, _ = diagnostic_trades(source, "f3a", source[200]["date"], source[207]["date"])
        self.assertEqual(len(trades), 1)
        self.assertEqual((trades[0]["entry_price"], trades[0]["exit_price"], trades[0]["bars"], trades[0]["open"]),
                         (119, 134, 5, False))

    def test_low_ties_are_a_declared_inclusive_window_and_terminal_position_remains_open(self):
        close = [100] * 190 + list(range(120, 130)) + [119, 119, 121, 123]
        source = rows(close)
        trades, _ = diagnostic_trades(source, "f3a", source[201]["date"], source[203]["date"])
        self.assertEqual(len(trades), 1)
        self.assertEqual(trades[0]["entry_price"], 119)
        self.assertTrue(trades[0]["open"])
        self.assertEqual(trades[0]["bars"], 2)

    def test_rsi_threshold_and_five_close_sma_exit_are_separate_rules(self):
        close = [100] * 190 + list(range(200, 210)) + [189, 169, 149, 155, 181, 178]
        source = rows(close)
        trades, _ = diagnostic_trades(source, "rsi2_5d", source[200]["date"], source[205]["date"])
        self.assertEqual(len(trades), 1)
        self.assertEqual(trades[0]["entry"], source[201]["date"])
        self.assertEqual(trades[0]["exit"], source[204]["date"])
        self.assertLess(trades[0]["entry_rsi2"], 2)
        self.assertGreater(trades[0]["entry_price"], trades[0]["entry_ma200"])
        self.assertGreater(trades[0]["exit_price"], trades[0]["exit_ma5"])

    def test_final_requested_bar_is_not_a_new_trade(self):
        source = rows([100] * 190 + list(range(120, 130)) + [119])
        trades, dates = diagnostic_trades(source, "f3a", source[200]["date"], source[200]["date"])
        self.assertEqual(trades, [])
        self.assertEqual(dates, [source[200]["date"]])


if __name__ == "__main__":
    unittest.main()
