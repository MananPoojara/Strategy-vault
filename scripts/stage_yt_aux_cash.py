#!/usr/bin/env python3
"""Stage a separate read-only-source cash database for AUX portfolio research.

Example: python3 scripts/stage_yt_aux_cash.py SOURCE_DIR NEW_OUTPUT_DIR
Writes only under NEW_OUTPUT_DIR; it must not exist. Never modifies SOURCE_DIR.
AUX-bearing symbols are staged from all native symbol directories, not a
watchlist. Files that cannot be decoded are recorded and prevent admission.
"""
import csv
import datetime
import hashlib
import json
import math
import os
from pathlib import Path
import struct
import sys

HEADER, RECORD = 1184, 40


def read_native(raw):
    if len(raw) < HEADER or raw[:8] != b"BROKDAt5":
        raise ValueError("unsupported native format")
    count = struct.unpack_from("<I", raw, 1180)[0]
    end = HEADER + count * RECORD
    if count > 100000 or end > len(raw):
        raise ValueError("invalid native record count")
    if len(raw) - end > 4:
        raise ValueError("unexpected bytes after the declared native records")
    seen = set()
    rows, a1, a2, dropped, widened = [], False, False, 0, 0
    for off in range(HEADER, end, RECORD):
        d, c, o, h, l, v, oi, x, y = struct.unpack_from("<Q8f", raw, off)
        year, month, day = d >> 52, (d >> 48) & 15, (d >> 43) & 31
        try:
            date = datetime.date(year, month, day).isoformat()
        except ValueError:
            if x == 1.0 or y == 1.0:
                raise ValueError("AUX-eligible source record has an invalid date")
            dropped += 1
            continue
        if date in seen:
            raise ValueError("duplicate native dates")
        seen.add(date)
        if not all(map(math.isfinite, (c, o, h, l, v, x, y))):
            raise ValueError("nonfinite price or AUX")
        if min(c, o, h, l) <= 0:
            if x == 1.0 or y == 1.0:
                raise ValueError("AUX-eligible source record has an invalid price")
            dropped += 1
            continue
        # Match the existing basket importer; record every high/low repair.
        hh, ll = max(h, o, c), min(l, o, c)
        widened += (hh != h or ll != l)
        rows.append((date, o, hh, ll, c, max(v, 0)))
        a1 |= x == 1.0
        a2 |= y == 1.0
    rows.sort()
    return rows, a1, a2, dropped, widened


def main(source, output):
    source = Path(source).resolve(strict=True)
    requested = Path(output).absolute()
    output = requested.parent.resolve(strict=True) / requested.name
    if output.exists() or output == source or source in output.parents:
        raise SystemExit("output must be a new directory outside the original database")
    output.mkdir(parents=True)
    native, prices = output / "native", output / "prices"
    native.mkdir(); prices.mkdir()
    index, failures = [], []
    counters = {"source_files": 0, "empty_or_no_aux": 0, "eligible_symbols": 0}
    for folder in sorted(source.iterdir()):
        if not folder.is_dir() or len(folder.name) != 1 or folder.name not in "0123456789_abcdefghijklmnopqrstuvwxyz":
            continue
        for path in sorted(folder.iterdir()):
            if not path.is_file():
                continue
            counters["source_files"] += 1
            symbol = path.name
            if not symbol or any(c in symbol for c in "/\\\r\n,") or symbol in (".", ".."):
                failures.append((symbol, "unsafe symbol name")); continue
            try:
                raw = path.read_bytes()
                if len(raw) > 32 * 1024 * 1024:
                    raise ValueError("source too large")
                rows, a1, a2, dropped, widened = read_native(raw)
                if not a1 and not a2:
                    counters["empty_or_no_aux"] += 1
                    continue
                if len(rows) < 2:
                    raise ValueError("eligible symbol has fewer than two valid price bars")
                dest = native / folder.name / symbol
                dest.parent.mkdir(exist_ok=True)
                dest.write_bytes(raw)
                # CSV output is private staged data, never committed to Git.
                output_path = prices / "symbols" / (symbol + ".csv")
                output_path.parent.mkdir(exist_ok=True)
                with output_path.open("w", newline="") as f:
                    w = csv.writer(f, lineterminator="\n")
                    w.writerow(("date", "open", "high", "low", "close", "volume"))
                    for date, o, h, l, c, vol in rows:
                        w.writerow((date, f"{o:.4f}", f"{h:.4f}", f"{l:.4f}", f"{c:.4f}", f"{vol:.0f}"))
                source_hash = hashlib.sha256(raw).hexdigest()
                csv_hash = hashlib.sha256(output_path.read_bytes()).hexdigest()
                note = f"Full database AUX inventory; {dropped} invalid dates or prices omitted; {widened} high/low bounds widened"
                for basket, eligible in (("nse500", a1), ("allfo", a2)):
                    if eligible:
                        index.append([basket, symbol, f"symbols/{symbol}.csv", rows[0][0], rows[-1][0], len(rows),
                                      "Team AmiBroker database cp_updated_2026", source_hash, csv_hash,
                                      "Dated AUX values; source inventory staged for research", "ready",
                                      "true" if any(r[5] > 0 for r in rows) else "false", note])
                counters["eligible_symbols"] += 1
            except (OSError, ValueError, struct.error, OverflowError) as e:
                failures.append((symbol, str(e)))
    counters["failures"] = len(failures)
    with (output / "stage-report.json").open("w") as f:
        json.dump({"counts": counters, "failures": failures}, f, indent=2)
    if failures:
        raise SystemExit(f"Staging incomplete: {len(failures)} failures; see stage-report.json. No index published.")
    counts = {basket: sum(r[0] == basket for r in index) for basket in ("nse500", "allfo")}
    if not all(counts.values()):
        raise SystemExit("Incomplete AUX inventories; no index published")
    with (prices / "index.csv").open("w", newline="") as f:
        w = csv.writer(f, lineterminator="\n")
        w.writerow(("basket", "ticker", "path", "first", "last", "bars", "source", "source_sha256", "csv_sha256", "membership", "status", "has_volume", "coverage_note", "basket_expected_members"))
        for row in sorted(index):
            w.writerow([*row, counts[row[0]]])
    print(json.dumps({"counts": counters, "basket_counts": counts}))


if __name__ == "__main__":
    if len(sys.argv) != 3:
        raise SystemExit(__doc__)
    main(*sys.argv[1:])
