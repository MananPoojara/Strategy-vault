#!/usr/bin/env python3
"""Stage the provisional global/crypto TradingView exports into one engine CSV for engine-gl.

Reads only the listed source files (never writes beside them). Writes Ticker,Date,Open,High,Low,Close,Volume with
the TradingView id as Ticker and an empty Volume (exports carry no traded volume, only CFD tick counts at most).
An instrument whose file is missing or has any unreadable, non-positive or duplicate-date row is left out whole,
so the engine answers needs_data for it; prices are never repaired. Usage: stage_gl.py SOURCE_ROOT OUT_CSV
"""
import csv, datetime, json, os, sys
from pathlib import Path

# (TradingView id, source path under the research root). Must match GL in Context/guru/backend/src/lab/markets.rs.
# Left out on purpose: SPCFD_SPX, DJCFD_DJI, NASDAQ_DLY_NDX (admitted SPX/DJI/NDX exist on /api/us; a provisional copy
# must not stand in for them), RUS_MOEX (stock or index unresolved), New folder/NASDAQ_DLY_IXIC (duplicate export).
MANIFEST = (
    ("BSE_DLY:SENSEX", "Global Bhavcopy/New folder/BSE_DLY_SENSEX, 1D_31c7b.csv"),
    ("CRYPTO:BTCUSD", "Global Bhavcopy/New folder/CRYPTO_BTCUSD, 1D_86731.csv"),
    ("CRYPTO:ETHUSD", "Global Bhavcopy/New folder/CRYPTO_ETHUSD, 1D_8ff42.csv"),
    ("HSI:HSI", "Global Bhavcopy/HSI_HSI, 1D_1033e.csv"),
    ("IG:DAX", "Global Bhavcopy/IG_DAX, 1D_6e586.csv"),
    ("IG:FTSE", "Global Bhavcopy/IG_FTSE, 1D_aecd1.csv"),
    ("NASDAQ_DLY:IXIC", "Global Bhavcopy/NASDAQ_DLY_IXIC, 1D_71afe.csv"),
    ("SPREADEX:NIKKEI", "Global Bhavcopy/SPREADEX_NIKKEI, 1D_b84f8.csv"),
    ("TVC:CAC40", "Global Bhavcopy/TVC_CAC40, 1D_6ed46.csv"),
    ("TVC:RUT", "Global Bhavcopy/New folder/TVC_RUT_SMALLCAP 2000 INDEX, 1D_0ea63.csv"),
)
HEADER = ["Ticker", "Date", "Open", "High", "Low", "Close", "Volume"]


def iso(text):
    text = text.strip()
    for fmt in ("%d-%m-%Y", "%Y-%m-%d"):
        try:
            return datetime.datetime.strptime(text, fmt).date().isoformat()
        except ValueError:
            pass
    return None


def read_export(path):
    """[(date, open, high, low, close)] sorted by date, or raise ValueError naming the first bad row."""
    with open(path, newline="") as f:
        rows = list(csv.reader(f))
    names = [h.strip().lower() for h in rows[0]] if rows else []
    # TradingView's date column is "time"; some exports name the ticker column "Date", so "time" wins.
    date_col = names.index("time") if "time" in names else names.index("date") if "date" in names else None
    if date_col is None or not all(k in names for k in ("open", "high", "low", "close")):
        raise ValueError(f"header {rows[0] if rows else []} lacks a date and open/high/low/close")
    cols = [names.index(k) for k in ("open", "high", "low", "close")]
    out = []
    for n, row in enumerate(rows[1:], start=2):
        if not any(cell.strip() for cell in row):
            continue
        try:
            date, prices = iso(row[date_col]), [float(row[c]) for c in cols]
        except (IndexError, ValueError):
            date, prices = None, []
        if date is None or len(prices) != 4 or not all(0 < p < float("inf") for p in prices):
            raise ValueError(f"line {n} is not a dated positive OHLC row: {row}")
        out.append((date, *(row[c].strip() for c in cols)))  # source text kept exactly; floats only validate
    out.sort()
    dupes = [a[0] for a, b in zip(out, out[1:]) if a[0] == b[0]]
    if dupes:
        raise ValueError(f"duplicate dates, first {dupes[0]}")
    if not out:
        raise ValueError("no rows")
    return out


def stage(source_root, out_csv):
    """Writes out_csv (header always, so engine-gl starts with zero instruments too); returns one note per instrument."""
    out_csv = Path(out_csv)
    out_csv.parent.mkdir(parents=True, exist_ok=True)
    notes, lines = [], []
    for ticker, rel in MANIFEST:
        path = Path(source_root) / rel
        if not path.is_file():
            notes.append(f"{ticker}: missing {path}; engine-gl answers needs_data")
            continue
        try:
            bars = read_export(path)
        except ValueError as e:
            notes.append(f"{ticker}: not staged ({e}); engine-gl answers needs_data")
            continue
        lines += [[ticker, *bar, ""] for bar in bars]
        notes.append(f"{ticker}: {len(bars)} bars {bars[0][0]} to {bars[-1][0]}")
    tmp = out_csv.with_name(out_csv.name + ".tmp")
    with open(tmp, "w", newline="") as f:
        writer = csv.writer(f)
        writer.writerow(HEADER)
        writer.writerows(lines)
    os.replace(tmp, out_csv)
    return notes


if __name__ == "__main__":
    if len(sys.argv) != 3:
        raise SystemExit("usage: stage_gl.py SOURCE_ROOT OUT_CSV")
    print(json.dumps(stage(sys.argv[1], sys.argv[2]), indent=1))
