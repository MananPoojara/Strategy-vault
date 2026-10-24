#!/usr/bin/env python3
"""Intraday phase 1A.1: clean real 1-minute bars into one CSV per ticker (regular session only).

Usage:  m1_normalise.py TICKER OUT_DIR      (TICKER = SPY | SPX | NIFTY 50 | NIFTYBANK | INDIA VIX)
        m1_normalise.py --test
Writes  OUT_DIR/<T>.m1idx.csv Date,offset,bytes (byte range of each day in <T>.m1.csv, read by src/intraday.rs)
        OUT_DIR/<T>.m1.csv   Date,Time,Open,High,Low,Close   (Time = START of the minute, exchange clock;
                                                             SPX converted from Central to Eastern time)
        OUT_DIR/<T>.days.csv Date,source,bars,dropped,flags,excluded   one row per source day, kept or not
Sources are read-only, one file at a time (machine guard between files). Rules (data-survey/PLAN.md 1A.1):
  * a minute that is padded (vendor filler), has no open (NULL), or is a stale first-minute open (SPX: open equals the
    previous session's last close) is DROPPED and counted, never filled in; such minutes can never produce a fill.
  * a day is EXCLUDED (logged with the reason) when more than 5% of its minutes were dropped, the session starts more
    than 5 minutes late, it ends early (not a known half day), or it is a listed outage day.
  * end-stamped files (minute stamped at its end, e.g. NSE 2010-14 bars 09:16..15:30) are shifted back one minute;
    an isolated pre-open auction print (NSE 09:08) and closing prints at/after the session close are dropped.
  * NIFTY 50: B (StockMock spot, Breeze) is the primary source from 2016-01-01; B 2015 rows are close-only snapshots
    (open = high = low = close on 100% of minutes) so 2008-2015 come from C. A B day that is excluded or missing falls
    back to the whole C day (never blended inside a day).
"""
import csv, os, sys, time
from datetime import datetime, timedelta, date

HOME = os.path.expanduser("~")
STAGE_C = f"{HOME}/.local/share/strategy-vault/data-staging/intraday"
SHARE_B = "/run/user/1000/gvfs/smb-share:server=192.168.4.50,share=share/Manan Pujara/#Data"
SRC = {
    "SPY": [("C", f"{STAGE_C}/SPY_1min.csv")],
    "NIFTYBANK": [("C", f"{STAGE_C}/NIFTYBANK_1min.csv")],
    "NIFTY 50": [("B", f"{SHARE_B}/stores/NIFTY_full/underlying/spot.parquet"), ("C", f"{STAGE_C}/NIFTY_50_1min.csv")],
    "SPX": [("B", f"{SHARE_B}/us data/hf_spx_mes_vix_1min_2026-08-18/raw/SPX_full_1min_CT.txt")],
    "INDIA VIX": [("B", f"{SHARE_B}/stores/NIFTY_full/underlying/india_vix.parquet")],
}
INDIA = {"NIFTY 50", "NIFTYBANK", "INDIA VIX"}
OUTAGE = {"2021-02-24"}  # NSE outage: the 1-min files stop at 10:08 though trading resumed 15:45-17:00
B_FROM = "2016-01-01"


def guard():
    while True:
        full = float(open("/proc/pressure/io").read().split("\n")[1].split()[1].split("=")[1])
        mem = int([l for l in open("/proc/meminfo") if l.startswith("MemAvailable")][0].split()[1])
        if full <= 15 and mem >= 8_000_000:
            return
        time.sleep(20)


def hm(s):
    s = s.strip()
    return int(s[:2]) * 60 + int(s[3:5])


def clock(m):
    return f"{m // 60:02d}:{m % 60:02d}"


def session(ticker, d):
    """(open minute, close minute) of the regular session, start-stamped exchange clock (close = first minute after)."""
    if ticker in INDIA:
        if d < "2010-01-04":
            return 9 * 60 + 55, 15 * 60 + 30
        if d < "2010-10-18":  # 09:00 open; pre-open call auction (09:00-09:08, print at 09:08, trading from 09:15) from 2010-10-18
            return 9 * 60, 15 * 60 + 30
        return 9 * 60 + 15, 15 * 60 + 30
    return 9 * 60 + 30, 16 * 60


def clean_day(ticker, d, bars, prev_close):
    """bars: list of (minute, o, h, l, c) as stamped in the source (o may be None). Returns (kept bars, dropped, flags, excluded)."""
    so, sc = session(ticker, d)
    bars = sorted({b[0]: b for b in reversed(bars)}.values(), key=lambda b: b[0])  # one bar per minute (first in file wins)
    flags, dropped = [], 0
    # isolated leading print (NSE 09:08 pre-open auction): gap to the next bar > 2 minutes
    if ticker in INDIA and len(bars) > 2 and bars[0][0] < so and bars[1][0] - bars[0][0] > 2:
        bars = bars[1:]
        flags.append("preopen_print_dropped")
    # end-stamped day: no bar at the open clock, first bar one minute later, a bar stamped at the close clock
    stamps = {b[0] for b in bars}
    if so not in stamps and (so + 1) in stamps and sc in stamps:
        bars = [(b[0] - 1,) + tuple(b[1:]) for b in bars]
        flags.append("end_stamped_shifted")
    bars = [b for b in bars if so <= b[0] < sc]  # closing prints / after-hours / pre-open
    kept = []
    for i, (m, o, h, l, c) in enumerate(bars):
        if o is None or h is None or l is None or c is None:
            dropped += 1
            continue
        if ticker == "SPX" and i == 0 and prev_close is not None and abs(o - prev_close) < 1e-9:
            dropped += 1
            flags.append("stale_open")
            continue
        kept.append((m, o, h, l, c))
    if ticker == "INDIA VIX" and len(kept) > 1 and kept[0][0] == so:
        # NSE's 09:15 India VIX print repeats the previous close (as main.rs::load_vix_dir): not a traded level
        kept = kept[1:]
        flags.append("open_print_dropped")
    total = len(bars)
    exc = ""
    if ticker in INDIA and d in OUTAGE:
        exc = "NSE outage day (session truncated in the source)"
    elif not kept:
        exc = "no regular-session bars"
    elif total and dropped > 0.05 * total:
        exc = f"{dropped} of {total} minutes dropped (>5%)"
    elif kept[0][0] > so + 5 and "stale_open" not in flags:
        exc = f"session starts late ({clock(kept[0][0])})"
    else:
        last = kept[-1][0]
        if ticker in INDIA:
            if last < 15 * 60:
                exc = f"session ends early ({clock(last)})"
        elif not (last >= 15 * 60 + 50 or 12 * 60 + 55 <= last <= 13 * 60 + 20):
            exc = f"session ends early ({clock(last)}), not a half day"
    return kept, dropped, flags, exc


def read_source(kind, path, ticker):
    """Yields (date, [(minute, o, h, l, c, padded)]) per day, start of day order."""
    days = {}
    if path.endswith(".parquet"):
        import polars as pl
        cols = pl.read_parquet_schema(path)
        df = pl.read_parquet(path, columns=[c for c in ["ts", "open", "high", "low", "close", "padded"] if c in cols])
        has_pad = "padded" in df.columns
        df = df.with_columns(pl.col("ts").dt.strftime("%Y-%m-%d").alias("d"), (pl.col("ts").dt.hour().cast(pl.Int32) * 60 + pl.col("ts").dt.minute().cast(pl.Int32)).alias("m"))
        for r in df.select(["d", "m", "open", "high", "low", "close"] + (["padded"] if has_pad else [])).iter_rows():
            days.setdefault(r[0], []).append((r[1], r[2], r[3], r[4], r[5], bool(r[6]) if has_pad else False))
        del df
    else:
        with open(path, newline="") as f:
            rd = csv.reader(f)
            head = [h.strip().lower() for h in next(rd)]
            ix = {k: head.index(k) for k in head}
            num = lambda v: float(v) if v.strip() not in ("", "nan", "NaN") else None
            for row in rd:
                if "datetime" in ix:  # SPX CT file: "2008-01-02 08:30:00" Central -> Eastern (+60 min)
                    dt = row[ix["datetime"]]
                    d, m = dt[:10], hm(dt[11:16]) + 60
                else:
                    d, m = row[ix["date"]], hm(row[ix["time"]])
                days.setdefault(d, []).append((m, num(row[ix["open"]]), num(row[ix["high"]]), num(row[ix["low"]]), num(row[ix["close"]]), False))
    return days


def build(ticker, out_dir):
    os.makedirs(out_dir, exist_ok=True)
    per_src = {}
    for kind, path in SRC[ticker]:
        guard()
        per_src[kind] = read_source(kind, path, ticker)
    dates = sorted(set().union(*[set(v) for v in per_src.values()]))
    t = ticker.replace(" ", "_")
    stats = {"days": 0, "kept_days": 0, "bars": 0, "dropped": 0, "padded": 0, "excluded": 0}
    prev_close = {k: None for k in per_src}
    with open(f"{out_dir}/{t}.m1.csv", "w", newline="") as fo, open(f"{out_dir}/{t}.days.csv", "w", newline="") as fd:
        wo, wd = csv.writer(fo, lineterminator="\n"), csv.writer(fd, lineterminator="\n")
        wo.writerow(["Date", "Time", "Open", "High", "Low", "Close"])
        wd.writerow(["Date", "source", "bars", "dropped", "padded", "flags", "excluded"])
        for d in dates:
            res = {}
            for kind, days in per_src.items():
                if d not in days:
                    continue
                raw = days[d]
                pad = sum(1 for b in raw if b[5])
                kept, dropped, flags, exc = clean_day(ticker, d, [b[:5] for b in raw if not b[5]], prev_close[kind])
                last = max((b for b in raw if b[4] is not None), key=lambda b: b[0], default=None)
                prev_close[kind] = last[4] if last else prev_close[kind]  # the source's last print of the day, after-hours included
                res[kind] = (kept, dropped, pad, flags, exc)
            order = ["B", "C"] if (ticker != "NIFTY 50" or d >= B_FROM) else ["C", "B"]
            if ticker == "NIFTY 50" and d < B_FROM:
                order = ["C"]  # B 2015 = close-only snapshots
            pick = next((k for k in order if k in res and not res[k][4]), None)
            stats["days"] += 1
            for k in res:  # log every source day (also the one not used) so the choice is auditable
                kept, dropped, pad, flags, exc = res[k]
                why = exc or ("" if k == pick else f"not used (source {pick} preferred)" if pick else "")
                wd.writerow([d, k, len(kept), dropped, pad, "|".join(flags), why])
                stats["dropped"] += dropped if k == pick or pick is None else 0
                stats["padded"] += pad if k == pick or pick is None else 0
            if pick is None:
                stats["excluded"] += 1
                continue
            kept = res[pick][0]
            stats["kept_days"] += 1
            stats["bars"] += len(kept)
            for m, o, h, l, c in kept:
                wo.writerow([d, clock(m), f"{o:.10g}", f"{h:.10g}", f"{l:.10g}", f"{c:.10g}"])
    return stats


def index(ticker, out_dir):
    """<T>.m1idx.csv: Date,offset,bytes of each day's rows in <T>.m1.csv (the engine seeks one day at a time)."""
    t = ticker.replace(" ", "_")
    rows, off = {}, 0
    with open(f"{out_dir}/{t}.m1.csv", "rb") as f:
        off = len(f.readline())
        for line in f:
            d = line[:10].decode()
            a = rows.setdefault(d, [off, 0])
            a[1] += len(line)
            off += len(line)
    with open(f"{out_dir}/{t}.m1idx.csv", "w") as f:
        f.write("Date,offset,bytes\n")
        for d, (o, n) in rows.items():
            f.write(f"{d},{o},{n}\n")
    return len(rows)


def check(ticker, out_dir):
    """Per year: kept days, dropped minutes, excluded days, median |daily close from minutes / engine daily close - 1|."""
    import polars as pl
    t = ticker.replace(" ", "_")
    daily = "/home/server/.cache/strategy-vault-webapp/data/" + ("idx_data.csv" if ticker in INDIA else "us_data.csv")
    if ticker == "INDIA VIX":
        return print("INDIA VIX: daily check left to the India VIX OHLC owner (claude/vix-india)")
    eng = pl.read_csv(daily, columns=["Ticker", "Date", "Close"], schema_overrides={"Date": pl.Utf8, "Close": pl.Utf8}).filter(pl.col("Ticker") == ticker)
    eng = eng.with_columns(pl.col("Date").str.strptime(pl.Date, "%d-%m-%Y").dt.strftime("%Y-%m-%d"), pl.col("Close").str.replace_all(",", "").cast(pl.Float64))
    m = pl.read_csv(f"{out_dir}/{t}.m1.csv", schema_overrides={"Date": pl.Utf8}).group_by("Date").agg(pl.col("Close").last().alias("mc"))
    j = m.join(eng, on="Date", how="left").with_columns(((pl.col("mc") / pl.col("Close") - 1).abs() * 100).alias("diff"), pl.col("Date").str.slice(0, 4).alias("y"))
    dd = pl.read_csv(f"{out_dir}/{t}.days.csv", schema_overrides={"Date": pl.Utf8, "excluded": pl.Utf8, "flags": pl.Utf8}).with_columns(pl.col("Date").str.slice(0, 4).alias("y"))
    exc = dd.group_by("Date", "y").agg((pl.col("excluded").fill_null("").str.starts_with("not used") | (pl.col("excluded").fill_null("") == "")).any().alias("ok")).filter(~pl.col("ok")).group_by("y").agg(pl.len().alias("excluded_days"))
    st = dd.filter(pl.col("flags").fill_null("").str.contains("stale_open")).group_by("y").agg(pl.len().alias("stale_open_days"))
    out = j.group_by("y").agg(pl.len().alias("days"), pl.col("Close").is_null().sum().alias("no_daily_row"), pl.col("diff").median().round(4).alias("median_close_diff_pct"),
                              (pl.col("diff") > 0.5).sum().alias("days_diff_gt_0.5pct")).join(exc, on="y", how="left").join(st, on="y", how="left").sort("y").fill_null(0)
    with pl.Config(tbl_rows=40, tbl_cols=10):
        print(ticker, "median close diff (all years) %:", round(j["diff"].median(), 4), "dropped minutes:", dd.filter(pl.col("excluded").fill_null("").str.starts_with("not used").not_())["dropped"].sum())
        print(out)


def test():
    # end-stamped NSE day with a 09:08 pre-open print and a closing print: shifted to 09:15..15:29
    bars = [(9 * 60 + 8, 1, 1, 1, 1)] + [(9 * 60 + 16 + k, 2, 3, 1, 2) for k in range(375)]
    kept, dr, fl, exc = clean_day("NIFTY 50", "2012-05-02", bars, None)
    assert kept[0][0] == 9 * 60 + 15 and kept[-1][0] == 15 * 60 + 29 and len(kept) == 375 and not exc, (kept[0], kept[-1], fl, exc)
    # start-stamped day with a 15:30 closing print: dropped
    bars = [(9 * 60 + 15 + k, 2, 3, 1, 2) for k in range(376)]
    kept, dr, fl, exc = clean_day("NIFTY 50", "2020-05-04", bars, None)
    assert len(kept) == 375 and kept[-1][0] == 15 * 60 + 29
    # NULL opens dropped, >5% -> excluded
    bars = [(9 * 60 + 15 + k, None if k < 20 else 2, 3, 1, 2) for k in range(375)]
    kept, dr, fl, exc = clean_day("NIFTY 50", "2017-05-04", bars, None)
    assert dr == 20 and exc and len(kept) == 355
    # duplicate stamps: first in file wins
    assert clean_day("SPY", "2005-11-25", [(9 * 60 + 30, 5, 5, 5, 5), (9 * 60 + 30, 6, 6, 6, 6)] + [(9 * 60 + 31 + k, 1, 1, 1, 1) for k in range(209)], None)[0][0][1] == 5
    # SPX stale first minute: dropped and flagged, day kept
    bars = [(9 * 60 + 30 + k, 100.0 if k == 0 else 101.0, 101.5, 99.5, 101.0) for k in range(390)]
    kept, dr, fl, exc = clean_day("SPX", "2012-05-02", bars, 100.0)
    assert dr == 1 and "stale_open" in fl and kept[0][0] == 9 * 60 + 31 and not exc
    # US half day ok, truncated day excluded
    assert not clean_day("SPY", "2005-11-25", [(9 * 60 + 30 + k, 1, 1, 1, 1) for k in range(210)], None)[3]
    assert clean_day("SPY", "2005-11-21", [(9 * 60 + 30 + k, 1, 1, 1, 1) for k in range(300)], None)[3]
    assert clean_day("NIFTY 50", "2021-02-24", [(9 * 60 + 15 + k, 1, 1, 1, 1) for k in range(375)], None)[3]
    print("m1_normalise self-test ok")


if __name__ == "__main__":
    if sys.argv[1:2] == ["--test"]:
        test()
    elif sys.argv[1:2] == ["--check"]:
        check(sys.argv[2], sys.argv[3])
    elif len(sys.argv) == 3:
        print(sys.argv[1], build(sys.argv[1], sys.argv[2]), "indexed days:", index(sys.argv[1], sys.argv[2]))
    else:
        sys.exit(__doc__)
