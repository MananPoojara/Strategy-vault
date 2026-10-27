#!/usr/bin/env python3
"""Hand-checked cases for footprint_from_trades.py (run: python3 -I test_footprint_from_trades.py)."""
import sys
from decimal import Decimal
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from footprint_from_trades import classify, footprint  # noqa: E402


def t(time, price, size, side="", bid="", ask=""):
    return {"time": time, "price": price, "size": size, "side": side, "bid": bid, "ask": ask}


def test_rules():
    rows = [
        t("2024-01-02 09:15:00", "100.00", "5"),                         # no side, no quote, no earlier price → dropped
        t("2024-01-02 09:15:01", "100.05", "3", side="b"),               # given side (any case)
        t("2024-01-02 09:15:02", "100.00", "2", bid="100.00", ask="100.10"),  # at the bid → sell
        t("2024-01-02 09:15:03", "100.10", "4", bid="100.00", ask="100.10"),  # at the ask → buy
        t("2024-01-02 09:15:04", "100.08", "1", bid="100.00", ask="100.10"),  # above the mid → buy
        t("2024-01-02 09:15:05", "100.05", "6", bid="100.00", ask="100.10"),  # at the mid; previous trade 100.08 is higher → sell (tick rule)
    ]
    sides = [s for _, s in classify(rows)[0]]
    assert sides == ["B", "S", "B", "B", "S"], sides
    assert classify(rows)[1] == 1
    # tick rule against the previous different price (100.00), not the one before it (100.20)
    tick = [t("d", "100.20", "1", side="S"), t("d", "100.00", "1", side="B"), t("d", "100.05", "1", bid="100.00", ask="100.10"),
            t("d", "100.05", "1", bid="100.00", ask="100.10")]  # equal to the previous trade → still compared with 100.00
    assert [s for _, s in classify(tick)[0]] == ["S", "B", "B", "B"]


def test_levels_round_half_even_and_sum():
    rows = [t("2024-01-02", "100.025", "1", side="B"),  # 100.025 / 0.05 = 2000.5 → 2000 (even) → 100.00
            t("2024-01-02", "100.075", "2", side="B"),  # 2001.5 → 2002 → 100.10
            t("2024-01-02", "100.00", "3", side="S"),
            t("2024-01-03", "100.00", "4", side="S")]
    out, dropped = footprint(rows, "0.05")
    got = {(d, str(p)): (b, a) for (d, p), (b, a) in out}
    assert dropped == 0
    assert got == {("2024-01-02", "100.00"): (Decimal(3), Decimal(1)), ("2024-01-02", "100.10"): (Decimal(0), Decimal(2)),
                   ("2024-01-03", "100.00"): (Decimal(4), Decimal(0))}, got


if __name__ == "__main__":
    test_rules()
    test_levels_round_half_even_and_sum()
    print("ok")
