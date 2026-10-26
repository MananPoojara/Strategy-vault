#!/usr/bin/env python3
"""Intraday phase 1A.2: daily per-ticker series (extra_data format) from one cleaned 1-minute file.

Usage: build_session_series.py TICKER M1_DIR OUT_DIR
Writes OUT_DIR/<T>__OPEN60_HIGH.csv, __OPEN60_LOW, __OPEN60_CLOSE, __LAST60_OPEN (Date,<SERIES>) and <T>__M15.csv
(Date,O01..O26,H01..H26,L01..L26,C01..C26). Windows are clock-based from the session's opening minute (US 09:30 ET;
NSE 09:55 before 2010-01-04, 09:00 to 2010-10-17, 09:15 since). OPEN60 = minutes [open, open+60); OPEN60_CLOSE = close
of the last minute in it; LAST60_OPEN = open of the first minute at or after (last minute - 59). A 15-minute bar is
written only if all 15 of its minutes exist (else blank), so no bar is built from a partial window; a day without its
opening minute gets no OPEN60/M15 values. Excluded days (see <T>.days.csv) have no row.
"""
import csv, os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from m1_normalise import session, guard


def days(path):
    cur, bars = None, []
    with open(path, newline="") as f:
        rd = csv.reader(f)
        next(rd)
        for d, t, o, h, l, c in rd:
            if d != cur:
                if bars:
                    yield cur, bars
                cur, bars = d, []
            bars.append((int(t[:2]) * 60 + int(t[3:5]), float(o), float(h), float(l), float(c)))
    if bars:
        yield cur, bars


def build(ticker, m1_dir, out_dir):
    guard()
    t = ticker.replace(" ", "_")
    os.makedirs(out_dir, exist_ok=True)
    names = ["OPEN60_HIGH", "OPEN60_LOW", "OPEN60_CLOSE", "LAST60_OPEN"]
    fs = {n: open(f"{out_dir}/{t}__{n}.csv", "w", newline="") for n in names}
    ws = {n: csv.writer(fs[n], lineterminator="\n") for n in names}
    for n in names:
        ws[n].writerow(["Date", n])
    fm = open(f"{out_dir}/{t}__M15.csv", "w", newline="")
    wm = csv.writer(fm, lineterminator="\n")
    wm.writerow(["Date"] + [f"{p}{k:02d}" for p in "OHLC" for k in range(1, 27)])
    n_days = n_open60 = 0
    for d, bars in days(f"{m1_dir}/{t}.m1.csv"):
        so, _ = session(ticker, d)
        n_days += 1
        by = {b[0]: b for b in bars}
        if so in by:
            w = [b for b in bars if b[0] < so + 60]
            n_open60 += 1
            ws["OPEN60_HIGH"].writerow([d, f"{max(b[2] for b in w):.10g}"])
            ws["OPEN60_LOW"].writerow([d, f"{min(b[3] for b in w):.10g}"])
            ws["OPEN60_CLOSE"].writerow([d, f"{w[-1][4]:.10g}"])
            m15 = []
            for k in range(26):
                mins = [by.get(so + 15 * k + i) for i in range(15)]
                m15.append(None if any(x is None for x in mins) else (mins[0][1], max(x[2] for x in mins), min(x[3] for x in mins), mins[-1][4]))
            wm.writerow([d] + [("" if b is None else f"{b[f]:.10g}") for f in range(4) for b in m15])
        last = bars[-1][0]
        l60 = next(b for b in bars if b[0] >= last - 59)
        ws["LAST60_OPEN"].writerow([d, f"{l60[1]:.10g}"])
    for f in list(fs.values()) + [fm]:
        f.close()
    return n_days, n_open60


if __name__ == "__main__":
    if len(sys.argv) != 4:
        sys.exit(__doc__)
    print(sys.argv[1], "days, days with the opening minute:", build(*sys.argv[1:]))
