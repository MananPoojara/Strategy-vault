#!/usr/bin/env python3
"""Trades → order-flow FOOTPRINT file for the Indicators lab (Context/guru/backend/src/lab/footprint.rs).

Input CSV: header `time,price,size` plus either `side` (B/S) or `bid,ask` quote columns.
Each trade is classified as a buy (at the ask) or a sell (at the bid):
  1. the given side;
  2. else the quote rule: price >= ask buy, price <= bid sell; inside the spread the midpoint rule (above mid buy, below sell);
  3. at the mid, the tick rule: higher than the last different trade price buy, lower sell (Lee & Ready 1991,
     "Inferring Trade Direction from Intraday Data", J. Finance 46(2); no 5-second quote lag: quotes are on the trade row);
  4. still undecided (no earlier different price): dropped and counted on stderr, never guessed.
Session date = the date part of `time` as given (no time-zone conversion). Levels: price rounded to `--tick`
(decimal, half-even). Output `<out_dir>/<TICKER spaces→_>__FOOTPRINT.csv`: date,price,bid_volume,ask_volume.

Usage: footprint_from_trades.py TRADES.csv TICKER OUT_DIR --tick 0.05
"""
import argparse
import csv
import sys
from collections import defaultdict
from decimal import ROUND_HALF_EVEN, Decimal
from pathlib import Path


def classify(rows):
    """Yield (row, 'B'|'S') for classified trades; return the dropped count via the list in the last element."""
    last_diff = None  # the price before the current run of equal prices
    prev = None
    dropped = 0
    out = []
    for r in rows:
        price = Decimal(r["price"])
        side = (r.get("side") or "").strip().upper()
        if side not in ("B", "S"):
            side = None
            bid, ask = r.get("bid"), r.get("ask")
            if bid not in (None, "") and ask not in (None, ""):
                bid, ask = Decimal(bid), Decimal(ask)
                if price >= ask:
                    side = "B"
                elif price <= bid:
                    side = "S"
                else:
                    mid = (bid + ask) / 2
                    side = "B" if price > mid else "S" if price < mid else None
            ref = prev if prev is not None and price != prev else last_diff  # the last trade price different from this one
            if side is None and ref is not None:
                side = "B" if price > ref else "S" if price < ref else None
        if prev is not None and price != prev:
            last_diff = prev
        prev = price
        if side is None:
            dropped += 1
            continue
        out.append((r, side))
    return out, dropped


def footprint(rows, tick):
    tick = Decimal(tick)
    levels = defaultdict(lambda: [Decimal(0), Decimal(0)])  # (date, level) -> [bid volume, ask volume]
    trades, dropped = classify(rows)
    for r, side in trades:
        level = (Decimal(r["price"]) / tick).quantize(Decimal(1), rounding=ROUND_HALF_EVEN) * tick
        level = level.quantize(tick)
        size = Decimal(r["size"])
        if size < 0:
            raise ValueError(f"negative size at {r['time']}")
        key = (r["time"].strip()[:10], level)
        levels[key][0 if side == "S" else 1] += size
    return sorted(levels.items()), dropped


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("trades")
    ap.add_argument("ticker")
    ap.add_argument("out_dir")
    ap.add_argument("--tick", required=True, help="price row size, e.g. 0.05")
    a = ap.parse_args()
    with open(a.trades, newline="") as f:
        rows = list(csv.DictReader(f))
    if rows and not {"time", "price", "size"} <= set(rows[0]):
        sys.exit("input needs time,price,size columns (plus side, or bid and ask)")
    out, dropped = footprint(rows, a.tick)
    path = Path(a.out_dir) / f"{a.ticker.replace(' ', '_')}__FOOTPRINT.csv"
    path.parent.mkdir(parents=True, exist_ok=True)
    with open(path, "w", newline="") as f:
        w = csv.writer(f, lineterminator="\n")
        w.writerow(["date", "price", "bid_volume", "ask_volume"])
        for (date, level), (b, s) in out:
            w.writerow([date, level, b, s])
    if dropped:
        print(f"{dropped} trades could not be classified (no side, no quote decision, no earlier price change) and were dropped", file=sys.stderr)
    print(path)


if __name__ == "__main__":
    main()
