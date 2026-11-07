"""BTST Phase 2: per-symbol `.bt5` stock series from the team's AmiBroker IEOD_FO database.

usage: python -I btst_stock_series.py [--db ~/strategy-vault-data/fo-ieod] [--out ~/strategy-vault-data/ieod-stocks]
                                      [--watchlist "FO STOCKS C&P"] [--jobs 6] [--extra n/NIFTY ...]

Format (BTST_PHASE2_SPEC.md section 1), little-endian:
  header : magic b'BT5STK01' (8), u32 n_days, u32 n_bars, u16 name_len, name (UTF-8, the AmiBroker symbol)
  days   : n_days x 32 bytes: i32 yyyymmdd, f32 DO, DH, DL, DC, DV, u32 NB (1-minute records), u32 first_bar
           (DO/DH/DL/DC/DV = the day's OHLC and volume from ALL its minutes, padded minutes included as stored)
  bars   : n_bars x 22 bytes, time order: u16 hhmm (start-stamped 5-minute bucket; every stored minute
           00:00-23:59 falls in a bucket), f32 O, H, L, C, V (V = sum of the bucket's 1-minute volumes)
File name: symbol with ' ' -> '_' and any other character outside [A-Za-z0-9_&-] -> %XX (UTF-8 bytes).
universe.json: {"watchlist", "stocks": [{symbol, file, first, last, days, watchlists[]}], "watchlists": {name: [symbols]},
                "extra": [...] (index files written on request, e.g. NIFTY), "missing": [...], "unlisted_files": [...]}
"""
import argparse, json, multiprocessing, os, struct, sys, time
from concurrent.futures import ProcessPoolExecutor

import numpy as np

MAGIC = b"BT5STK01"
DAY_DT = np.dtype([("ymd", "<i4"), ("o", "<f4"), ("h", "<f4"), ("l", "<f4"), ("c", "<f4"), ("v", "<f4"),
                   ("nb", "<u4"), ("first", "<u4")])
BAR_DT = np.dtype([("hhmm", "<u2"), ("o", "<f4"), ("h", "<f4"), ("l", "<f4"), ("c", "<f4"), ("v", "<f4")])
assert DAY_DT.itemsize == 32 and BAR_DT.itemsize == 22
DB = os.path.expanduser("~/strategy-vault-data/fo-ieod")
OUT = os.path.expanduser("~/strategy-vault-data/ieod-stocks")
SKIP_DIRS = {"WatchLists", "Layouts"}


def safe_name(sym):
    out = []
    for ch in sym:
        if ch == " ":
            out.append("_")
        elif ch.isascii() and (ch.isalnum() or ch in "_&-"):
            out.append(ch)
        else:
            out.extend(f"%{b:02X}" for b in ch.encode("utf-8"))
    return "".join(out)


def load_minutes(path):
    """AmiBroker native 'BROKDAt5' file -> (records, yyyymmdd, minute-of-day)."""
    raw = open(path, "rb").read()
    if raw[:8] != b"BROKDAt5":
        raise ValueError(f"{path}: not an AmiBroker native file")
    dt = np.dtype([("u", "<u8"), ("c", "<f4"), ("o", "<f4"), ("h", "<f4"), ("l", "<f4"),
                   ("v", "<f4"), ("oi", "<f4"), ("a1", "<f4"), ("a2", "<f4")])
    r = np.frombuffer(raw, dt, count=(len(raw) - 1184) // 40, offset=1184)
    hi = (r["u"] >> np.uint64(32)).astype(np.int64)
    day = ((hi >> 20) & 0xfff) * 10000 + ((hi >> 16) & 0xf) * 100 + ((hi >> 11) & 0x1f)
    minute = ((hi >> 6) & 0x1f) * 60 + (hi & 0x3f)
    if np.any(np.diff(day * 1440 + minute) < 0):
        raise ValueError(f"{path}: records are not in time order")
    return r, day, minute


def _segments(key):
    s = np.r_[0, np.nonzero(np.diff(key))[0] + 1]
    return s, np.r_[s[1:], len(key)]


def build_arrays(path, keep=None):
    """-> (days structured array DAY_DT, bars structured array BAR_DT). keep(r, day, minute) -> bool mask
    (analysis only: e.g. drop the 15:30 minute); the .bt5 files use every stored minute."""
    r, day, minute = load_minutes(path)
    if keep is not None:
        m = keep(r, day, minute)
        r, day, minute = r[m], day[m], minute[m]
    o, h, l, c = r["o"], r["h"], r["l"], r["c"]
    v64 = r["v"].astype(np.float64)
    vc = np.r_[0.0, np.cumsum(v64)]
    bucket = minute - minute % 5
    bs, be = _segments(day * 10000 + bucket)
    bars = np.empty(len(bs), BAR_DT)
    bars["hhmm"] = (bucket[bs] // 60) * 100 + bucket[bs] % 60
    bars["o"] = o[bs]; bars["c"] = c[be - 1]
    bars["h"] = np.maximum.reduceat(h, bs); bars["l"] = np.minimum.reduceat(l, bs)
    bars["v"] = (vc[be] - vc[bs]).astype(np.float32)
    ds, de = _segments(day)
    days = np.empty(len(ds), DAY_DT)
    days["ymd"] = day[ds]; days["o"] = o[ds]; days["c"] = c[de - 1]
    days["h"] = np.maximum.reduceat(h, ds); days["l"] = np.minimum.reduceat(l, ds)
    days["v"] = (vc[de] - vc[ds]).astype(np.float32); days["nb"] = de - ds
    bday = day[bs]
    days["first"] = np.searchsorted(bday, days["ymd"], side="left")
    return days, bars


def write_bt5(path, name, days, bars):
    nm = name.encode("utf-8")
    tmp = path + ".tmp"
    with open(tmp, "wb") as f:
        f.write(MAGIC + struct.pack("<IIH", len(days), len(bars), len(nm)) + nm)
        f.write(days.astype(DAY_DT, copy=False).tobytes()); f.write(bars.astype(BAR_DT, copy=False).tobytes())
    os.replace(tmp, path)


def read_bt5(path):
    """-> (name, days DAY_DT array, bars BAR_DT array)."""
    raw = open(path, "rb").read()
    if raw[:8] != MAGIC:
        raise ValueError(f"{path}: not a BT5STK01 file")
    nd, nbar, ln = struct.unpack_from("<IIH", raw, 8)
    off = 18
    name = raw[off:off + ln].decode("utf-8"); off += ln
    days = np.frombuffer(raw, DAY_DT, nd, off); off += nd * 32
    bars = np.frombuffer(raw, BAR_DT, nbar, off); off += nbar * 22
    if off != len(raw):
        raise ValueError(f"{path}: size mismatch ({len(raw)} bytes, expected {off})")
    return name, days, bars


def to_oracle_bars(days, bars):
    """bt5 arrays -> the list-of-dict bars used by oracle_btst.run() (o/h/l/c/v as Python floats of the f32)."""
    import datetime as dt
    out = []
    nd = len(days)
    for i in range(nd):
        y = int(days["ymd"][i]); a = int(days["first"][i]); z = int(days["first"][i + 1]) if i + 1 < nd else len(bars)
        D = dt.date(y // 10000, y // 100 % 100, y % 100)
        seg = bars[a:z]
        for hm, o, h, l, c, v in zip(seg["hhmm"].tolist(), seg["o"].tolist(), seg["h"].tolist(), seg["l"].tolist(),
                                     seg["c"].tolist(), seg["v"].tolist()):
            out.append(dict(ts=dt.datetime(D.year, D.month, D.day, hm // 100, hm % 100), hhmm=hm, day=D,
                            o=o, h=h, l=l, c=c, v=v))
    return out


def db_files(db):
    files = {}
    for d in sorted(os.listdir(db)):
        p = os.path.join(db, d)
        if os.path.isdir(p) and d not in SKIP_DIRS:
            for f in sorted(os.listdir(p)):
                files[f] = os.path.join(p, f)
    return files


def watchlists(db):
    out = {}
    wd = os.path.join(db, "WatchLists")
    for f in sorted(os.listdir(wd)):
        if f.endswith(".tls"):
            with open(os.path.join(wd, f), encoding="latin-1") as fh:
                out[f[:-4]] = [s.strip() for s in fh if s.strip()]
    return out


def _job(args):
    sym, src, out = args
    days, bars = build_arrays(src)
    fn = safe_name(sym) + ".bt5"
    write_bt5(os.path.join(out, fn), sym, days, bars)
    f = lambda y: f"{y // 10000:04d}-{y // 100 % 100:02d}-{y % 100:02d}"
    return dict(symbol=sym, file=fn, first=f(int(days["ymd"][0])), last=f(int(days["ymd"][-1])), days=int(len(days)),
                bars=int(len(bars)), minutes=int(days["nb"].sum()))


def build_all(db=DB, out=OUT, watchlist="FO STOCKS C&P", jobs=6, extra=("n/NIFTY",)):
    t0 = time.time()
    os.makedirs(out, exist_ok=True)
    files = db_files(db)
    wls = watchlists(db)
    syms = wls[watchlist]
    missing = [s for s in syms if s not in files]
    unlisted = sorted(set(files) - set(syms))
    names = [safe_name(s) for s in syms]
    assert len(set(names)) == len(names), "safe-name collision"
    work = [(s, files[s], out) for s in syms if s in files]
    ex = []
    for e in extra:
        s = os.path.basename(e)
        if s not in syms:
            work.append((s, os.path.join(db, e), out)); ex.append(s)
    with ProcessPoolExecutor(jobs, mp_context=multiprocessing.get_context("fork")) as pool:
        res = list(pool.map(_job, work, chunksize=4))
    member = {}
    for wl, ss in wls.items():
        for s in ss:
            member.setdefault(s, []).append(wl)
    stocks = []
    extras = []
    for r in res:
        r["watchlists"] = member.get(r["symbol"], [])
        (extras if r["symbol"] in ex else stocks).append(r)
    size = sum(os.path.getsize(os.path.join(out, r["file"])) for r in res)
    meta = dict(format="BT5STK01", source=db, watchlist=watchlist, built=time.strftime("%Y-%m-%d %H:%M:%S"),
                stocks=stocks, extra=extras, watchlists=wls, missing=missing, unlisted_files=unlisted,
                total_bytes=size, seconds=round(time.time() - t0, 1))
    with open(os.path.join(out, "universe.json"), "w") as f:
        json.dump(meta, f, indent=1)
    return meta


def main(argv=None):
    ap = argparse.ArgumentParser()
    ap.add_argument("--db", default=DB); ap.add_argument("--out", default=OUT)
    ap.add_argument("--watchlist", default="FO STOCKS C&P"); ap.add_argument("--jobs", type=int, default=6)
    ap.add_argument("--extra", nargs="*", default=["n/NIFTY"])
    a = ap.parse_args(argv)
    m = build_all(a.db, a.out, a.watchlist, a.jobs, a.extra)
    print(f"{len(m['stocks'])} stock files (+{len(m['extra'])} extra) -> {a.out}: {m['total_bytes'] / 1e6:.1f} MB "
          f"in {m['seconds']} s; missing {m['missing']}; unlisted {len(m['unlisted_files'])}", file=sys.stderr)


if __name__ == "__main__":
    main()
