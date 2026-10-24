#!/usr/bin/env python3
"""Options phase 2 (US): compact per-underlying monthly-option chains from the SPY/QQQ/IWM EOD option set.

Source (read-only): share/Manan Pujara/#Data/us data/gh_options_eod_spy_qqq_iwm_2008-2025_2026-08-18/raw/<sym>/
options_YYYY.parquet (16:00 ET snapshot; MIT mirror of philippdubach/options-dataset-hist), ONE yearly file at a time
(machine guard between files). Output (staging only):
  <out>/us/years/<SYM>/<YYYY>.csv, <out>/us/<SYM>.opt.csv, <out>/us/<SYM>.optidx.csv (Date,offset,bytes)
Columns: Date,Expiry,Type,Strike,Bid,Ask,Last,Volume,OI

Kept rows: standard MONTHLY expiries only (see monthly_set: the Saturday after the third Friday as listed before 2015, else the
third Friday, else the Thursday before it when that Friday was an exchange holiday; daily/weekly/quarterly expirations dropped),
among the first three monthly expiries on or after the date, strike within +-12% of the dataset's own underlying close
that day (a size filter only). The source `mark` column is NOT used: in 2008 87% of rows carry mark = 0.01 with a
two-sided quote far away (checked: |mark - (bid+ask)/2| > 0.006 on 87% of 2008 quoted rows, 0.8% in 2012, 0 after 2019).
The engine computes mid = (bid + ask) / 2 itself and fills only when volume > 0 and 0 < bid <= ask. No price is filled
or repaired here.
usage: build_us.py OUT_DIR [SYM ...]   (default SPY QQQ IWM)
"""
import datetime as dt, glob, os, sys, time
import numpy as np
import pyarrow.parquet as pq

SRC = "/run/user/1000/gvfs/smb-share:server=192.168.4.50,share=share/Manan Pujara/#Data/us data/gh_options_eod_spy_qqq_iwm_2008-2025_2026-08-18/raw"
BAND = 0.12
COLS = ["date", "expiration", "strike", "type", "bid", "ask", "last", "volume", "open_interest"]


def guard():
    while True:
        full = float(open("/proc/pressure/io").read().split("\n")[1].split()[1].split("=")[1])
        mem = int([l for l in open("/proc/meminfo") if l.startswith("MemAvailable")][0].split()[1])
        if full <= 15 and mem >= 8_000_000:
            return
        time.sleep(20)


def third_friday(y, m):
    d = dt.date(y, m, 1)
    return d + dt.timedelta((4 - d.weekday()) % 7 + 14)


def monthly_set(exps):
    """The standard monthly expiry of each month among the listed expirations: the Saturday after the third Friday (as
    listed before 2015), else the third Friday, else the Thursday before it (an exchange holiday on that Friday). Daily
    and weekly expirations on other days, including Thursdays in months that have a Friday monthly, are dropped."""
    have = set(exps)
    out = set()
    for y, m in {(int(e[:4]), int(e[5:7])) for e in exps}:
        f = third_friday(y, m)
        for d in (f + dt.timedelta(1), f, f - dt.timedelta(1)):
            if d.isoformat() in have:
                out.add(d.isoformat())
                break
    return out


def fmt(v):
    s = repr(float(v))
    return s[:-2] if s.endswith(".0") else s


def year_rows(path, under):
    t = pq.read_table(path, columns=COLS)
    date = np.array(t["date"].to_pylist(), dtype=object)
    exp = np.array(t["expiration"].to_pylist(), dtype=object)
    strike = t["strike"].to_numpy()
    exps = sorted(set(exp.tolist()))
    mset = monthly_set(exps)
    monthly = sorted(mset)
    keep = np.array([e in mset for e in exp]) & (exp >= date)
    ref = np.array([under.get(d, np.nan) for d in date])
    keep &= ~(np.abs(strike / ref - 1) > BAND)  # NaN ref keeps the row
    # first three monthly expiries on or after the date
    days = sorted(set(date[keep].tolist()))
    ahead = {d: set([m for m in monthly if m >= d][:3]) for d in days}
    idx = [i for i in np.nonzero(keep)[0] if exp[i] in ahead[date[i]]]
    typ = t["type"].to_pylist()
    cols = {c: t[c].to_numpy() for c in ("bid", "ask", "last", "volume", "open_interest")}
    out = [(date[i], exp[i].replace("-", ""), "P" if typ[i] == "put" else "C", strike[i], cols["bid"][i], cols["ask"][i], cols["last"][i],
            int(cols["volume"][i]), int(cols["open_interest"][i])) for i in idx if typ[i] in ("put", "call")]
    out.sort(key=lambda x: (x[0], x[1], x[2], x[3]))
    return out


def main():
    out_dir = sys.argv[1]
    syms = sys.argv[2:] or ["SPY", "QQQ", "IWM"]
    for sym in syms:
        u = pq.read_table(f"{SRC}/{sym.lower()}/underlying_prices.parquet", columns=["date", "close"]).to_pylist()
        under = {str(r["date"])[:10]: r["close"] for r in u}
        ydir = f"{out_dir}/us/years/{sym}"
        os.makedirs(ydir, exist_ok=True)
        for f in sorted(glob.glob(f"{SRC}/{sym.lower()}/options_*.parquet")):
            y = os.path.basename(f)[8:12]
            dst = f"{ydir}/{y}.csv"
            if os.path.exists(dst):
                continue
            guard()
            rows = year_rows(f, under)
            with open(dst + ".tmp", "w") as fh:
                for r in rows:
                    fh.write(",".join([r[0], r[1], r[2], fmt(r[3]), fmt(r[4]), fmt(r[5]), fmt(r[6]), str(r[7]), str(r[8])]) + "\n")
            os.replace(dst + ".tmp", dst)
            print(sym, y, len(rows), "rows", flush=True)
        opt, idx = f"{out_dir}/us/{sym}.opt.csv", f"{out_dir}/us/{sym}.optidx.csv"
        with open(opt + ".tmp", "wb") as fo, open(idx + ".tmp", "w") as fi:
            head = b"Date,Expiry,Type,Strike,Bid,Ask,Last,Volume,OI\n"
            fo.write(head)
            fi.write("Date,offset,bytes\n")
            off = len(head)
            for m in sorted(glob.glob(f"{ydir}/*.csv")):
                cur, start = None, off
                for line in open(m, "rb"):
                    d = line[:10].decode()
                    if d != cur:
                        if cur is not None:
                            fi.write(f"{cur},{start},{off - start}\n")
                        cur, start = d, off
                    fo.write(line)
                    off += len(line)
                if cur is not None:
                    fi.write(f"{cur},{start},{off - start}\n")
        os.replace(opt + ".tmp", opt)
        os.replace(idx + ".tmp", idx)
        print("wrote", opt, flush=True)


if __name__ == "__main__":
    main()
