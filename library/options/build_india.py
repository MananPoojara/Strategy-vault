#!/usr/bin/env python3
"""Options phase 2 (India): compact per-underlying monthly-option chains from the NSE F&O EOD bhavcopy.

Source (read-only): the local Parquet copy of A, ~/.local/share/strategy-vault/fno_parquet/year=YYYY/YYYY-MM.parquet,
read ONE monthly file at a time (machine guard between files). Output (staging only):
  <out>/india/months/<SYM>/<YYYY-MM>.csv   one file per source month (rebuilt only if missing)
  <out>/india/<TICKER>.opt.csv            all months concatenated, sorted by Date, Expiry, Type, Strike
  <out>/india/<TICKER>.optidx.csv         Date,offset,bytes of each day's block (the engine reads one day at a time)
Columns: Date,Expiry,Type,Strike,Open,High,Low,Close,Settle,Contracts,OI  (Type P/C; prices as printed)

Kept rows: OPTIDX of the symbol whose expiry is a MONTHLY expiry (the last listed expiry of its calendar month; weeklies
dropped), among the first three monthly expiries on or after the trade date, strike within +-12% of that day's
near-month index-future close (a size filter only; the engine selects strikes from the vault's index close).
Nothing is filled, repaired or interpolated. Rows with Contracts = 0 are kept (their Settle is the exchange settlement
price, usable only as a labelled mark); the engine never fills on them. The printed Close of an untraded row is
not a price (it can be any stale number) and the engine never reads it.
usage: build_india.py OUT_DIR [SYM ...]   (default NIFTY BANKNIFTY)
"""
import glob, os, sys, time
import pyarrow.parquet as pq
import pyarrow.compute as pc

SRC = os.path.expanduser("~/.local/share/strategy-vault/fno_parquet")
TICKER = {"NIFTY": "NIFTY_50", "BANKNIFTY": "NIFTYBANK"}
BAND = 0.12
COLS = ["Date", "ExpiryDate", "Instrument", "Symbol", "StrikePrice", "OptionType", "Open", "High", "Low", "Close", "SettledPrice", "Contracts", "OpenInterest"]


def guard():
    while True:
        full = float(open("/proc/pressure/io").read().split("\n")[1].split()[1].split("=")[1])
        mem = int([l for l in open("/proc/meminfo") if l.startswith("MemAvailable")][0].split()[1])
        if full <= 15 and mem >= 8_000_000:
            return
        time.sleep(20)


def fmt(v):
    s = repr(float(v))
    return s[:-2] if s.endswith(".0") else s


def month_rows(path, sym):
    t = pq.read_table(path, columns=COLS, filters=[("Symbol", "=", sym)])
    t = t.filter(pc.is_in(t["Instrument"], value_set=__import__("pyarrow").array(["OPTIDX", "FUTIDX"])))
    rows = t.to_pylist()
    fut = {}  # date -> (expiry, close) of the nearest future
    for r in rows:
        if r["Instrument"] == "FUTIDX" and r["Close"] and r["Close"] > 0:
            k = r["Date"]
            if k not in fut or r["ExpiryDate"] < fut[k][0]:
                fut[k] = (r["ExpiryDate"], r["Close"])
    opts = [r for r in rows if r["Instrument"] == "OPTIDX" and r["OptionType"] in ("PE", "CE")]
    # monthly = the last listed expiry of each calendar month (all expiries listed anywhere in this file)
    last = {}
    for r in opts:
        e = r["ExpiryDate"]
        last[(e.year, e.month)] = max(last.get((e.year, e.month), e), e)
    monthly = sorted(set(last.values()))
    out = []
    for r in opts:
        d, e = r["Date"], r["ExpiryDate"]
        if e not in last.values() or e < d:
            continue
        ahead = [m for m in monthly if m >= d][:3]
        if e not in ahead:
            continue
        ref = fut.get(d, (None, None))[1]
        if ref and abs(r["StrikePrice"] / ref - 1) > BAND:
            continue
        out.append((d.isoformat(), e.isoformat().replace("-", ""), "P" if r["OptionType"] == "PE" else "C", r["StrikePrice"],
                    r["Open"], r["High"], r["Low"], r["Close"], r["SettledPrice"], r["Contracts"], r["OpenInterest"]))
    out.sort(key=lambda x: (x[0], x[1], x[2], x[3]))
    return out, len(fut)


def main():
    out_dir = sys.argv[1]
    syms = sys.argv[2:] or ["NIFTY", "BANKNIFTY"]
    files = sorted(glob.glob(f"{SRC}/year=*/*.parquet"))
    for sym in syms:
        mdir = f"{out_dir}/india/months/{sym}"
        os.makedirs(mdir, exist_ok=True)
        for f in files:
            name = os.path.basename(f)[:-8]
            dst = f"{mdir}/{name}.csv"
            if os.path.exists(dst):
                continue
            guard()
            rows, nfut = month_rows(f, sym)
            tmp = dst + ".tmp"
            with open(tmp, "w") as fh:
                for r in rows:
                    fh.write(",".join([r[0], r[1], r[2], fmt(r[3])] + [fmt(v) for v in r[4:9]] + [str(r[9]), str(r[10])]) + "\n")
            os.replace(tmp, dst)
            print(sym, name, len(rows), "rows", nfut, "future days", flush=True)
        # concatenate + day index
        tk = TICKER[sym]
        opt, idx = f"{out_dir}/india/{tk}.opt.csv", f"{out_dir}/india/{tk}.optidx.csv"
        with open(opt + ".tmp", "wb") as fo, open(idx + ".tmp", "w") as fi:
            head = b"Date,Expiry,Type,Strike,Open,High,Low,Close,Settle,Contracts,OI\n"
            fo.write(head)
            fi.write("Date,offset,bytes\n")
            off = len(head)
            for m in sorted(glob.glob(f"{mdir}/*.csv")):
                cur, start, n = None, off, 0
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
        print("wrote", opt, idx, flush=True)


if __name__ == "__main__":
    main()
