#!/usr/bin/env python3
"""K39 EP check: aggregate every day of each 1-minute file into N-minute bars in Python (streamed, one day at a time) and
compare with the engine helper's dump (intraday_ep::tests::nbars_dump), bar by bar, every field.
Bars: bucket start = anchor + floor((minute - anchor) / N) * N; anchor = the session open (US 9:30; NSE 9:55 before
2010-01-04, 9:00 until 2010-10-17, 9:15 since) or 0 (clock "natural" bars); stamp = min(bucket start + N, session close);
O = first minute's open, H/L = extremes, C = last minute's close, count = minutes in the bucket.
Usage: nbars_check.py M1_DIR DUMP_DIR"""
import sys
M1, DUMP = sys.argv[1], sys.argv[2]

def sopen(t, d):
    if not t.startswith("NIFTY"): return 570
    return 595 if d < "2010-01-04" else 540 if d < "2010-10-18" else 555

def days(path):
    with open(path) as f:
        next(f); cur, rows = None, []
        for line in f:
            v = line.rstrip("\n").split(",")
            if v[0] != cur:
                if rows: yield cur, rows
                cur, rows = v[0], []
            hh, mm = v[1].split(":")
            rows.append((int(hh) * 60 + int(mm), v[2], v[3], v[4], v[5]))
        if rows: yield cur, rows

def agg(rows, anchor, n, close):
    out = {}
    order = []
    for m, o, h, l, c in rows:
        s = anchor + max(m - anchor, 0) // n * n
        if s not in out:
            out[s] = [o, float(h), float(l), c, 0, h, l]; order.append(s)
        b = out[s]
        if float(h) > b[1]: b[1], b[5] = float(h), h
        if float(l) < b[2]: b[2], b[6] = float(l), l
        b[3] = c; b[4] += 1
    for s in order:
        o, _, _, c, k, h, l = out[s]
        yield s, min(s + n, close), o, h, l, c, k

tot = bad = 0
for t in ["SPY", "SPX", "NIFTY 50", "NIFTYBANK"]:
    f = t.replace(" ", "_")
    close = 930 if t.startswith("NIFTY") else 960
    r = open(f"{DUMP}/{f}.rust.csv")
    n_t = bad_t = 0
    for d, rows in days(f"{M1}/{f}.m1.csv"):
        for anchor, n in [(sopen(t, d), 5), (sopen(t, d), 10), (sopen(t, d), 15), (sopen(t, d), 30), (sopen(t, d), 60), (sopen(t, d), 135), (0, 60)]:
            for s, e, o, h, l, c, k in agg(rows, anchor, n, close):
                got = r.readline().rstrip("\n").split(",")
                want = [d, str(anchor), str(n), str(s), str(e)]
                ok = got[:5] == want and [float(z) for z in got[5:9]] == [float(o), float(h), float(l), float(c)] and int(got[9]) == k
                n_t += 1
                if not ok:
                    bad_t += 1
                    if bad_t <= 5: print("MISMATCH", t, want, (o, h, l, c, k), got)
    extra = r.readline()
    if extra: bad_t += 1; print("EXTRA engine rows", t, extra.strip())
    print(f"{t}: {n_t} bars, {bad_t} mismatches", flush=True)
    tot += n_t; bad += bad_t
print("TOTAL", tot, "bars", bad, "mismatches")
