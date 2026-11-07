"""Independent Python oracle for the BTST AFL
('btst trad in intraday with flt and profit booking with IBS flt and ROC ranking .afl').

Re-derived from the AFL source, not from the vault spec.  Floats (IEEE double) are used
throughout; thresholds are compared exactly as the AFL does (<=, >=, >, <), so a value that
lands exactly on a threshold can differ by one ulp from AmiBroker's own float handling.

Run:  ~/.venvs/strategy-vault/bin/python -I oracle_btst.py --ledger OUT.csv [--days 3 ...]

Semantics implemented (see VERIFICATION-TRADELIST.md for the AFL-vs-spec notes):
  * 5-min bars start-stamped, built from 1-minute bars (bar t = minutes t..t+4).
  * TimeFrameExpand with the AFL default mode: during day D the daily/weekly/monthly value
    seen is that of the LAST COMPLETED period (shift by one compressed bar).
  * ROC base: Ref(C,-(ROCDAYS-2)) in the daily frame -> expanded (shift 1) ->
    then the AFL's  Ref(dailyclose1,-5)  taken LITERALLY (5 existing 5-min bars before the
    entry bar; identical to the entry-bar value whenever those bars are on the same day).
  * Sell = H>pb OR exitbar OR L<hs on every bar from the entry bar itself onwards.
"""
import argparse, csv, datetime as dt, sys
from collections import OrderedDict

NULL = -1e10   # AmiBroker's Null/EMPTY as seen by scalar loop reads


# ----------------------------------------------------------------------------- bars
def build_5min(parquet_path, drop_1530=False):
    """Start-stamped 5-minute bars from the 1-minute parquet.
    Returns (bars, partial) ; bars = list of dict(ts,hhmm,day,o,h,l,c); partial = #bars with <5 minutes
    (excluding the lone 15:30 bar).  drop_1530=True removes the lone 15:30 bar entirely (the
    1-minute 15:30 print in this feed is often a stray/auction print, e.g. 2019-05-17 11690.3 vs 11440.8 at 15:29),
    and with it every later bar (the session runs to 15:40 from 2026-08-03): the spec reads the feed 0915..1525."""
    import polars as pl
    df = pl.read_parquet(parquet_path)
    ren = {a: b for a, b in (("open", "o"), ("high", "h"), ("low", "l"), ("close", "c")) if a in df.columns}
    df = df.rename(ren).sort("ts")
    g = (df.with_columns(pl.col("ts").dt.truncate("5m").alias("bar"))
           .group_by("bar", maintain_order=True)
           .agg(pl.col("o").first(), pl.col("h").max(), pl.col("l").min(), pl.col("c").last(),
                pl.len().alias("n")))
    bars = []
    partial = 0
    for ts, o, h, l, c, n in g.iter_rows():
        hhmm = ts.hour * 100 + ts.minute
        if hhmm >= 1530 and drop_1530:
            continue
        if n < 5 and hhmm != 1530:
            partial += 1
        bars.append(dict(ts=ts, hhmm=hhmm, day=ts.date(), o=o, h=h, l=l, c=c))
    return bars, partial


def build_5min_ieod(path):
    """Start-stamped 5-minute bars from an AmiBroker native symbol file of the team's IEOD_FO database
    (BROKDAt5: 1184-byte header, 40-byte records = packed date-time u64 + float32 C,O,H,L,V,OI,Aux1,Aux2).
    Every minute is kept as stored (padded Aux1=1 minutes included, sessions as recorded: 09:55 before 2010,
    late or evening sessions too), so daily bars match AmiBroker's own compression. Returns (bars, 0).
    Each bar also carries v = the sum of its 1-minute volumes (padded minutes included, as AmiBroker
    compresses them); daily_bars() sums these into the daily volume (all minutes of the day)."""
    import numpy as np
    with open(path, "rb") as fh:
        raw = fh.read()
    if raw[:8] != b"BROKDAt5":
        raise ValueError(f"{path}: not an AmiBroker native file")
    dt_ = np.dtype([("u", "<u8"), ("c", "<f4"), ("o", "<f4"), ("h", "<f4"), ("l", "<f4"),
                    ("v", "<f4"), ("oi", "<f4"), ("a1", "<f4"), ("a2", "<f4")])
    r = np.frombuffer(raw, dt_, count=(len(raw) - 1184) // 40, offset=1184)
    hi = (r["u"] >> np.uint64(32)).astype(np.int64)
    ymd = ((hi >> 20) & 0xfff) * 10000 + ((hi >> 16) & 0xf) * 100 + ((hi >> 11) & 0x1f)
    mins = ((hi >> 6) & 0x1f) * 60 + (hi & 0x3f)
    key = ymd * 10000 + (mins - mins % 5)
    vcum = np.concatenate([[0.0], np.cumsum(r["v"].astype(np.float64))])
    bars, start = [], 0
    for end in list(np.nonzero(np.diff(key))[0] + 1) + [len(key)]:
        k = int(key[start]); d = k // 10000; m = k % 10000
        day = dt.date(d // 10000, d // 100 % 100, d % 100)
        bars.append(dict(ts=dt.datetime(day.year, day.month, day.day, m // 60, m % 60), hhmm=m // 60 * 100 + m % 60, day=day,
                         o=float(r["o"][start]), h=float(r["h"][start:end].max()), l=float(r["l"][start:end].min()),
                         c=float(r["c"][end - 1]), v=float(vcum[end] - vcum[start])))
        start = end
    return bars, 0


def daily_bars(bars):
    """Daily OHLC(+V) from the 5-min bars: open=first bar open, close=LAST bar close, volume=sum
    (AmiBroker compression).  v is None when the bars carry no volume (feed parquet)."""
    days = OrderedDict()
    for b in bars:
        d = days.get(b["day"])
        v = b.get("v")
        if d is None:
            days[b["day"]] = dict(date=b["day"], o=b["o"], h=b["h"], l=b["l"], c=b["c"], v=v)
        else:
            d["h"] = max(d["h"], b["h"]); d["l"] = min(d["l"], b["l"]); d["c"] = b["c"]
            d["v"] = None if (v is None or d["v"] is None) else d["v"] + v
    return list(days.values())


def _compress(daily, keyf):
    out = OrderedDict()
    for d in daily:
        k = keyf(d["date"])
        p = out.get(k)
        if p is None:
            out[k] = dict(key=k, first=d["date"], o=d["o"], h=d["h"], l=d["l"], c=d["c"])
        else:
            p["h"] = max(p["h"], d["h"]); p["l"] = min(p["l"], d["l"]); p["c"] = d["c"]
    return list(out.values())


def weekly_bars(daily):   # calendar weeks Monday..Sunday
    return _compress(daily, lambda d: d.isocalendar()[:2])


def monthly_bars(daily):
    return _compress(daily, lambda d: (d.year, d.month))


def tf_bars(daily, tf):
    return {1: lambda x: _compress(x, lambda d: d), 2: weekly_bars, 3: monthly_bars}[tf](daily)


# ----------------------------------------------------------------------------- filters
def supertrend(bars_tf, period, factor, point):
    """The AFL's coded loop, line by line.  bars_tf: list of dict(o,h,l,c).  Returns IMtrend list."""
    n = len(bars_tf)
    H = [b["h"] for b in bars_tf]; L = [b["l"] for b in bars_tf]; C = [b["c"] for b in bars_tf]
    mid = [(h + l) / 2 if point == 1 else ((h + l + c) / 3 if point == 3 else c)
           for h, l, c in zip(H, L, C)]
    # ATR(1) == true range; first bar H-L
    tr = [H[0] - L[0]] + [max(H[i] - L[i], abs(H[i] - C[i - 1]), abs(L[i] - C[i - 1])) for i in range(1, n)]
    # satr = (sum_{i=0}^{period-1} Ref(ATR(1),-i)) / period ; Null until `period` bars exist
    satr = [None] * n
    for i in range(period - 1, n):
        satr[i] = sum(tr[i - k] for k in range(period)) / period
    Up = [NULL if s is None else m + factor * s for m, s in zip(mid, satr)]
    Dn = [NULL if s is None else m - factor * s for m, s in zip(mid, satr)]
    trend = [1] * n
    change = 0
    for i in range(1, n):
        trend[i] = 1
        if C[i] > Up[i - 1]:
            trend[i] = 1
            if trend[i - 1] == -1: change = 1
        elif C[i] < Dn[i - 1]:
            trend[i] = -1
            if trend[i - 1] == 1: change = 1
        elif trend[i - 1] == 1:
            trend[i] = 1; change = 0
        elif trend[i - 1] == -1:
            trend[i] = -1; change = 0
        flag = 1 if (trend[i] < 0 and trend[i - 1] > 0) else 0
        flagh = 1 if (trend[i] > 0 and trend[i - 1] < 0) else 0
        if trend[i] > 0 and Dn[i] < Dn[i - 1]: Dn[i] = Dn[i - 1]
        if trend[i] < 0 and Up[i] > Up[i - 1]: Up[i] = Up[i - 1]
        if flag == 1: Up[i] = mid[i] + factor * satr[i]     # flips can only happen once satr exists
        if flagh == 1: Dn[i] = mid[i] - factor * satr[i]
        # ITrendUp/ITrendDown/changeOfTrend (plot bookkeeping) never feed back into IMtrend
    return trend


def sma_filter_series(bars_tf, period):
    """FLTBuy per compressed bar index p (visible at the NEXT period): C[p] > MA(C)[p-1]. True/False."""
    C = [b["c"] for b in bars_tf]
    n = len(C)
    ma = [None] * n
    run = 0.0
    for i in range(n):
        run += C[i]
        if i >= period: run -= C[i - period]
        if i >= period - 1: ma[i] = run / period
    out = [False] * n
    for p in range(1, n):
        out[p] = ma[p - 1] is not None and C[p] > ma[p - 1]
    return out


def athc_buysig(closes, pct):
    """The ATHC-swing AFL's two daily-frame loops, line by line ('btst trad in intraday with Profit booking
    AND ATHC FLT.afl').  Hcl = HHV(C,BarCount) read as the running all-time-high close (incl. bar i);
    EXIT = C*pct/100; Up = Hcl; Dn = C-EXIT; trend1 like the Supertrend loop with Up/Dn ratchets; then
    BUYSIG[i] = 1 if C[i] > Hcl[i-1], else 0 if trend1[i]==-1 and BUYSIG[i-1]==1, else BUYSIG[i-1]
    (BUYSIG[0] = 0).  Returns (buysig, trend1)."""
    n = len(closes)
    C = list(closes)
    hcl, m = [], None
    for c in C:
        m = c if m is None else max(m, c); hcl.append(m)
    ex = [c * pct / 100 for c in C]
    Up = list(hcl); Dn = [c - e for c, e in zip(C, ex)]
    tr = [1] * n
    for i in range(1, n):
        tr[i] = 1
        if C[i] > Up[i - 1]: tr[i] = 1
        elif C[i] < Dn[i - 1]: tr[i] = -1
        elif tr[i - 1] == 1: tr[i] = 1
        elif tr[i - 1] == -1: tr[i] = -1
        flag = tr[i] < 0 and tr[i - 1] > 0
        flagh = tr[i] > 0 and tr[i - 1] < 0
        if tr[i] > 0 and Dn[i] < Dn[i - 1]: Dn[i] = Dn[i - 1]
        if tr[i] < 0 and Up[i] > Up[i - 1]: Up[i] = Up[i - 1]
        if flag: Up[i] = hcl[i]
        if flagh: Dn[i] = C[i] - ex[i]
    bs = [0] * n
    for i in range(1, n):
        if C[i] > hcl[i - 1]: bs[i] = 1
        elif tr[i] == -1 and bs[i - 1] == 1: bs[i] = 0
        else: bs[i] = bs[i - 1]
    return bs, tr


# Final-AFL defaults (legacy; run() without a spec preset).  The Phase-1 spec defaults are SPEC_DEFAULTS.
DEFAULTS = dict(days=3, entry_time=1515, exit_time=920, pb_pct=1.0, roc_days=2, ibs_thr=0.5, roc_thr=1.0,
                hardstop=0.0, flt_type=1, flt_tf=1, st_point=1, st_period=20, st_factor=1, sma_period=200)
# Add-on parameters (AFL defaults, with the spec's fixes: high_days 2 -> 3 (AFL range 3..15)).
ADDON_DEFAULTS = dict(ibs_dir=1,          # ibs_roc: 1 = ibs <= ibs_thr (final AFL), 2 = ibs >= ibs_thr (earlier AFL)
                      high_days=3,        # high_n: "highest close days" (AFL hdays)
                      green_mode=1,       # green: 1 = GREEN, 2 = ANY (AFL "CANDLE COLOR")
                      green_days=2,       # green: AFL hdays "Green Candle days" (today + green_days-1 daily candles)
                      vol_days=5,         # volume: AFL vdays "Volume check days"
                      vol_mult=1.0,       # volume: AFL Volume_MULTI (1 = the plain volume AFL)
                      athc_pct=10.0,      # athc: AFL EXITPCT "ATHC EXIT %"
                      lookahead=0)        # old_fixed: 0 = 15:15 price (Vault), 1 = literal AFL (day's final close)
SPEC_DEFAULTS = dict(DEFAULTS, days=2, entry_time=1450, exit_time=925, pb_pct=0.0, hardstop=0.0, flt_type=3,
                     roc_days=2, ibs_thr=0.5, roc_thr=1.0, **ADDON_DEFAULTS)
VARIANTS = ("core", "ibs_roc", "ath", "ath_only", "high_n", "green", "volume", "athc", "old_fixed")
UPCLOSE = {"core", "ibs_roc", "ath", "high_n", "green", "volume", "athc"}


def spec_params(variant, **over):
    """Phase-1 spec defaults for a sub-variant (BTST_PHASE1_SPEC.md section 3), overridden by `over`."""
    assert variant in VARIANTS, variant
    p = dict(SPEC_DEFAULTS); p.update(over)
    return p


class Data:
    def __init__(self, bars):
        self.bars = bars
        self.daily = daily_bars(bars)
        self.day_idx = {d["date"]: i for i, d in enumerate(self.daily)}
        self.first = {}                       # day -> flat index of first bar
        for k, b in enumerate(bars):
            self.first.setdefault(b["day"], k)
        self._flt = {}
        self._athc = {}
        m, self.ath = None, []                # running max of daily closes up to and incl. day i
        for d in self.daily:
            m = d["c"] if m is None else max(m, d["c"]); self.ath.append(m)

    def athc(self, pct):
        if pct not in self._athc:
            self._athc[pct] = athc_buysig([d["c"] for d in self.daily], pct)[0]
        return self._athc[pct]

    def filter_by_day(self, p):
        """list (per daily bar) of bool FLTBuy as seen on that day."""
        key = (p["flt_type"], p["flt_tf"], p["st_point"], p["st_period"], p["st_factor"], p["sma_period"])
        if key in self._flt: return self._flt[key]
        if p["flt_type"] == 3:
            res = [True] * len(self.daily)
        else:
            tfb = tf_bars(self.daily, p["flt_tf"])
            if p["flt_type"] == 1:
                tr = supertrend(tfb, p["st_period"], p["st_factor"], p["st_point"])
                seen = [False] + [t == 1 for t in tr[:-1]]          # last completed bar
            else:
                seen = sma_filter_series(tfb, p["sma_period"])
                seen = [False] + seen[:-1]
            # map day -> compressed period index
            keyf = {1: lambda d: d, 2: lambda d: d.isocalendar()[:2], 3: lambda d: (d.year, d.month)}[p["flt_tf"]]
            kidx = {b["key"]: i for i, b in enumerate(tfb)}
            res = [seen[kidx[keyf(d["date"])]] for d in self.daily]
        self._flt[key] = res
        return res


def _trade(day, eb, xb, px, why):
    return dict(signal_date=day.isoformat(), entry_ts=eb["ts"].isoformat(sep=" "), entry_px=eb["c"],
                exit_date=xb["day"].isoformat(), exit_ts=xb["ts"].isoformat(sep=" "), exit_px=px, exit_reason=why)


def run_old_fixed(data, lookahead=False, start=None, end=None):
    """'btst trad in intraday ver2_updated.afl': Buy on the bar with TimeNum()==151500 when
    C1 > C0 > C-1 > C-2 and (C1-C0) >= (C0-C-1) >= (C-1-C-2)  [all strict / non-strict as written], where
    C0, C-1, C-2 = closes of the last three COMPLETED days (expandLast) and C1 = Ref(dailyclose,1) expanded =
    TODAY's final daily close (lookahead=True, literal AFL) or the 15:15 bar close (lookahead=False, Vault).
    Sell = Ref(startbar,-1): the first bar after the entry whose previous bar is the first bar of a day
    (normally the 2nd bar of the next day); SellPrice = C.  No pb, no stop, no filter."""
    B, D = data.bars, data.daily
    out, last_exit = [], -1
    for i in range(3, len(D)):
        day = D[i]["date"]
        if (start and day < start) or (end and day > end): continue
        k = data.first[day]
        while k < len(B) and B[k]["day"] == day and B[k]["hhmm"] != 1515: k += 1
        if k >= len(B) or B[k]["day"] != day or k <= last_exit: continue
        c1 = D[i]["c"] if lookahead else B[k]["c"]
        c0, cm1, cm2 = D[i - 1]["c"], D[i - 2]["c"], D[i - 3]["c"]
        if not (c1 > c0 and c0 > cm1 and cm1 > cm2): continue
        if not ((c1 - c0) >= (c0 - cm1) and (c0 - cm1) >= (cm1 - cm2)): continue
        j = k + 1
        while j < len(B) and not (j - 1 == 0 or B[j - 1]["day"] != B[j - 2]["day"]): j += 1
        if j >= len(B): continue                           # still open at end of data: dropped
        out.append(_trade(day, B[k], B[j], B[j]["c"], "time"))
        last_exit = j
    return out


# ----------------------------------------------------------------------------- strategy
def run(params=None, data=None, bars=None, start=None, end=None, variant="ibs_roc"):
    """Return list of trades (dicts). `start`/`end` restrict SIGNAL dates only (history is always full).
    Missing params fall back to DEFAULTS (final AFL) + ADDON_DEFAULTS; use spec_params(variant) for the
    Phase-1 spec defaults.  variant: one of VARIANTS (default 'ibs_roc' = the original oracle behaviour).
    Every daily-frame value is read at the LAST COMPLETED day d = (entry day index - 1) (expandLast)."""
    assert variant in VARIANTS, variant
    p = dict(DEFAULTS); p.update(ADDON_DEFAULTS); p.update(params or {})
    if data is None: data = Data(bars)
    if variant == "old_fixed":
        return run_old_fixed(data, bool(p["lookahead"]), start, end)
    B = data.bars; D = data.daily
    n = p["days"]; et = p["entry_time"]; xt = p["exit_time"]
    flt = data.filter_by_day(p) if variant != "athc" else None     # athc: the ATHC trend replaces the filter
    athc = data.athc(p["athc_pct"]) if variant == "athc" else None
    pb_on = p["pb_pct"] > 0
    C = lambda x: D[x]["c"]
    trades = []
    last_exit_k = -1
    for i in range(len(D)):
        day = D[i]["date"]
        if (start and day < start) or (end and day > end): continue
        # entry bar: stamped exactly et on this day
        k0 = data.first[day]
        k = k0
        while k < len(B) and B[k]["day"] == day and B[k]["hhmm"] != et: k += 1
        if k >= len(B) or B[k]["day"] != day: continue
        if k <= last_exit_k: continue                     # ExRem: still in a position
        if i < 1: continue
        d = i - 1                                         # last completed day
        P = B[k]["c"]
        if variant in UPCLOSE:
            if d - (n - 1) < 0: continue
            # buycond1: Sum(C>Ref(C,-1), days-1) == days-1
            if not all(C(d - q) > C(d - q - 1) for q in range(0, n - 1)): continue
            # buycond2 (DIFF[x]=C[x]-C[x-1]; needs x-1>=0 and x-1-1 >=0 for the compared one)
            ok = True
            for q in range(0, n - 2):
                x = d - q
                if x - 2 < 0: ok = False; break
                if not ((C(x) - C(x - 1)) > (C(x - 1) - C(x - 2))): ok = False; break
            if not ok: continue
            if not P > C(d): continue
            if d - 1 < 0: continue
            if not (P - C(d)) > (C(d) - C(d - 1)): continue
        if flt is not None and not flt[i]: continue
        if variant == "athc" and athc[d] != 1: continue
        if variant in ("ath", "ath_only"):
            # intradayclose > TimeFrameExpand(HHV(C,BarCount)) : running ATH close up to day d inclusive
            if not P > data.ath[d]: continue
        elif variant == "high_n":
            # intradayclose > TimeFrameExpand(HHV(C,hdays)) : max close of days d-hdays+1 .. d
            if not P > max(C(x) for x in range(max(0, d - p["high_days"] + 1), d + 1)): continue
        elif variant == "green":
            if p["green_mode"] == 1:
                # intradayclose - ValueWhen(startbar,O) > 0  AND  Sum(C-O>0, hdays-1) == hdays-1 (days d-hdays+2..d)
                if not P - B[k0]["o"] > 0: continue
                g = p["green_days"] - 1
                if d - g + 1 < 0: continue
                if not all(D[x]["c"] - D[x]["o"] > 0 for x in range(d - g + 1, d + 1)): continue
        elif variant == "volume":
            # avgvol loop: totvol accumulates V[i] (i>=1) and is reset on every bar with timenow >= entrytime;
            # at the entry bar avgvol = sum V over (r, k], r = last earlier bar with hhmm >= entrytime.
            r = k - 1
            while r >= 1 and B[r]["hhmm"] < et: r -= 1
            lo = r + 1 if r >= 1 else 1
            vol = sum(B[x]["v"] for x in range(lo, k + 1))
            N = p["vol_days"]
            if d - N + 1 < 0: continue                     # MA(V,vdays) Null
            ma = sum(D[x]["v"] for x in range(d - N + 1, d + 1)) / N
            if not vol > p["vol_mult"] * ma: continue
        elif variant == "ibs_roc":
            # IBS: HighestSince(startbar,H) / LowestSince(startbar,L) up to and including the entry bar
            ih = max(b["h"] for b in B[k0:k + 1]); il = min(b["l"] for b in B[k0:k + 1])
            if ih == il: continue                              # 0/0 -> NaN -> comparison false
            ibs = (ih - P) / (ih - il)
            if p["ibs_dir"] == 1:
                if not ibs <= p["ibs_thr"]: continue
            elif not ibs >= p["ibs_thr"]: continue
            # ROC base: Ref(expand(Ref(C,-(ROCDAYS-2))), -5) -> literal 5 existing bars back
            kk = k - 5
            if kk < 0: continue
            di = data.day_idx[B[kk]["day"]] - 1                # expand shifts one compressed bar
            xi = di - (p["roc_days"] - 2)
            if di < 0 or xi < 0 or xi >= len(D): continue
            base = D[xi]["c"]
            if not (P - base) / base * 100 >= p["roc_thr"]: continue
        # ---- exit
        pb = P + P * p["pb_pct"] / 100 if pb_on else 99999999.0
        hs = P - P * p["hardstop"] / 100 if p["hardstop"] > 0 else 0.0
        j = k
        while True:
            b = B[j]
            exitbar = b["hhmm"] < et and b["hhmm"] >= xt
            hit_pb = b["h"] > pb; hit_hs = b["l"] < hs
            if hit_pb or exitbar or hit_hs:
                if hit_pb:
                    px = b["o"] - b["o"] * 0.25 / 100 if b["o"] > pb else pb; why = "pb"
                elif b["o"] < hs:
                    px = b["o"] - b["o"] * 0.25 / 100; why = "hs"
                elif b["l"] < hs:
                    px = hs; why = "hs"
                else:
                    px = b["c"]; why = "time"
                px = min(max(px, b["l"]), b["h"])     # AmiBroker backtester: a fill outside the bar's L..H moves to L/H
                trades.append(_trade(day, B[k], b, px, why))
                last_exit_k = j
                break
            j += 1
            if j >= len(B): break                          # still open at end of data: dropped
    return trades


# ============================================================================= E3 / E4 main variants
# E3 "Second-last candle": 'BTST Intraday (Buy at 2nd last candle_Sell at next day opening).afl' and the
#    '...with monthly filter for Bull Market.afl' / '...Bear Market.afl' copies (identical except == 1 / == -1).
# E4 "Candle-count": 'BTST on the basis of no. of candles_Intraday FO/Candles based Entry Exit(BTST).afl'.
E34_VARIANTS = ("slc", "slc_bull", "slc_bear", "candles")
E3_DEFAULTS = dict(src=1, tf=1, flt_tf=3, st_point=1, st_period=5, st_factor=1)
E4_DEFAULTS = dict(src=1, tf=1, entry_candle=0, exit_candle=1, pt_pct=5.0, buffer_pct=0.25, flt_type=1, flt_tf=1,
                   st_point=1, st_period=5, st_factor=4)


def e34_params(variant, **over):
    assert variant in E34_VARIANTS, variant
    p = dict(E4_DEFAULTS if variant == "candles" else E3_DEFAULTS); p.update(over)
    return p


def resample(bars, minutes):
    """Clock-aligned start-stamped buckets of `minutes` (15 -> :00/:15/:30/:45) from 5-minute (or finer) bars:
    O first, H max, L min, C last, V sum; a bucket without bars does not exist."""
    if minutes == 5:
        return bars
    out = []
    for b in bars:
        m = (b["ts"].hour * 60 + b["ts"].minute) // minutes * minutes
        ts = b["ts"].replace(hour=m // 60, minute=m % 60, second=0, microsecond=0)
        if out and out[-1]["ts"] == ts:
            x = out[-1]
            x["h"] = max(x["h"], b["h"]); x["l"] = min(x["l"], b["l"]); x["c"] = b["c"]
            x["v"] = None if (x.get("v") is None or b.get("v") is None) else x["v"] + b["v"]
        else:
            out.append(dict(ts=ts, hhmm=m // 60 * 100 + m % 60, day=b["day"], o=b["o"], h=b["h"], l=b["l"], c=b["c"],
                            v=b.get("v")))
    return out


def expand_index(bars, flt_tf):
    """TimeFrameExpand(x, tf) with the default expandLast, per intraday bar k: the index of the latest
    D/W/M period whose LAST bar is at or before k (on a period's last bar that period itself; inside it the
    previous period).  Bars of the FIRST period before its last bar see the first period (AmiBroker fills
    the leading bars with the first value: the team's lists trade on the data's first day / week, where only
    Mtrend[0] = 1 is defined).  Periods are built from the same bars (calendar weeks Mon..Sun, months).
    Returns (idx list, compressed period bars)."""
    daily = daily_bars(bars)
    keyf = {1: lambda d: d, 2: lambda d: d.isocalendar()[:2], 3: lambda d: (d.year, d.month)}[flt_tf]
    tfb = _compress(daily, keyf)
    pidx = {b["key"]: i for i, b in enumerate(tfb)}
    per = [pidx[keyf(b["day"])] for b in bars]
    n = len(bars)
    out = [-1] * n
    for k in range(n):
        last = k == n - 1 or per[k + 1] != per[k]
        out[k] = per[k] if last else max(per[k] - 1, 0)
    return out, tfb


def supertrend_bands(bars_tf, period, factor, point):
    """Same loop as supertrend() but also returns the final Up/Dn bands (NULL during warm-up), so that the
    AFL's TrendUp = Dn (trend 1) / TrendDown = Up (trend -1) can be tested for > 0 (E3's `abc`)."""
    n = len(bars_tf)
    H = [b["h"] for b in bars_tf]; L = [b["l"] for b in bars_tf]; C = [b["c"] for b in bars_tf]
    mid = [(h + l) / 2 if point == 1 else ((h + l + c) / 3 if point == 3 else c) for h, l, c in zip(H, L, C)]
    tr = [H[0] - L[0]] + [max(H[i] - L[i], abs(H[i] - C[i - 1]), abs(L[i] - C[i - 1])) for i in range(1, n)]
    satr = [None] * n
    for i in range(period - 1, n):
        satr[i] = sum(tr[i - k] for k in range(period)) / period
    Up = [NULL if s is None else m + factor * s for m, s in zip(mid, satr)]
    Dn = [NULL if s is None else m - factor * s for m, s in zip(mid, satr)]
    trend = [1] * n
    for i in range(1, n):
        trend[i] = 1
        if C[i] > Up[i - 1]: trend[i] = 1
        elif C[i] < Dn[i - 1]: trend[i] = -1
        elif trend[i - 1] == 1: trend[i] = 1
        elif trend[i - 1] == -1: trend[i] = -1
        flag = trend[i] < 0 and trend[i - 1] > 0
        flagh = trend[i] > 0 and trend[i - 1] < 0
        if trend[i] > 0 and Dn[i] < Dn[i - 1]: Dn[i] = Dn[i - 1]
        if trend[i] < 0 and Up[i] > Up[i - 1]: Up[i] = Up[i - 1]
        if flag: Up[i] = mid[i] + factor * satr[i]
        if flagh: Dn[i] = mid[i] - factor * satr[i]
    # TrendUp[i] = Dn[i] if trend 1, TrendDown[i] = Up[i] if trend -1 (bar 0: both Null; the AFL's
    # TrendUp[i-1] = Null / TrendDown[i-1] = ... on a change only touches a value that is already Null)
    band = [None] + [(Dn[i] if trend[i] == 1 else Up[i]) for i in range(1, n)]
    band = [None if (x is None or x == NULL) else x for x in band]
    return trend, band


def _p2(x):
    return round(x, 2)     # AFL Prec(x,2); prices are on a 0.05 tick grid, so round/truncate give the same equality


class E34Data:
    """Bars of one timeframe (tf 1 = 5-min, 2 = 15-min) + cached expanded Supertrend."""
    def __init__(self, bars5, tf=1):
        self.bars = resample(bars5, 5 if tf == 1 else 15)
        self._st = {}

    def trend(self, flt_tf, period, factor, point):
        key = (flt_tf, period, factor, point)
        if key not in self._st:
            idx, tfb = expand_index(self.bars, flt_tf)
            tr, band = supertrend_bands(tfb, period, factor, point)
            self._st[key] = ([tr[j] if j >= 0 else None for j in idx],
                             [(band[j] is not None and band[j] > 0) if j >= 0 else False for j in idx])
        return self._st[key]


def _t(eb, xb, epx, xpx, why):
    return dict(signal_date=eb["day"].isoformat(), entry_ts=eb["ts"].isoformat(sep=" "), entry_px=epx,
                exit_date=xb["day"].isoformat(), exit_ts=xb["ts"].isoformat(sep=" "), exit_px=xpx, exit_reason=why)


def run_slc(data, p, variant):
    """E3, the AFL loop literally (i from 1):
       buy  if flat AND MyDay != Ref(MyDay,2) AND MyDay == Ref(MyDay,1) [2nd-last candle of the day; at the
            data's 2nd-last bar Ref(,2) is Null -> true] AND NOT UPCIRCUIT (Prec(O,2)==Prec(C,2)==Prec(H,2))
            [AND trend == +1 / -1 AND abc for bull / bear]  -> BuyPrice = Close
       then (same bar too) sell if holding AND MyDay != Ref(MyDay,-1) [first candle of a day] AND NOT locircuit
            (Prec(O,2)==Prec(C,2)==Prec(L,2)) -> SellPrice = Close.
       A buy and a sell on the same bar (a 2-candle day) cancel: Buy[i]=0, Sell[i]=1, ExRem drops the sell."""
    B = data.bars; n = len(B)
    want = {"slc_bull": 1, "slc_bear": -1}.get(variant)
    if want is not None:
        tr, abc = data.trend(p["flt_tf"], p["st_period"], p["st_factor"], p["st_point"])
    out = []; pos = 0; ent = None
    for i in range(1, n):
        b = B[i]
        bought = False
        cand = (i + 2 >= n or B[i + 2]["day"] != b["day"]) and (i + 1 < n and B[i + 1]["day"] == b["day"])
        if pos == 0 and cand and not (_p2(b["o"]) == _p2(b["c"]) and _p2(b["o"]) == _p2(b["h"])) and b["c"] > 0 \
                and (want is None or (tr[i] == want and abc[i])):
            pos = 1; ent = i; bought = True
        if pos > 0 and B[i - 1]["day"] != b["day"] and not (_p2(b["o"]) == _p2(b["c"]) and _p2(b["o"]) == _p2(b["l"])):
            pos = 0
            if not bought:
                out.append(_t(B[ent], b, B[ent]["c"], b["c"], "time"))
    return out


def run_candles(data, p):
    """E4, the AFL loop literally (i from 1):
       if holding AND day(i) != buy day: count += 1   (counts candles, never reset until the exit)
       if flat AND Entry_Candle[i] AND Mtrend[i] == 1: buy at Close; PT = (1 + pt/100) x Close if pt > 0
          Entry_Candle: entry 0 -> MyDay != Ref(MyDay,1) (last candle); k -> MyDay != Ref(MyDay,k+1) AND
          MyDay == Ref(MyDay,k)  (k candles before the last; Ref beyond the data end = Null -> !=)
       else if holding: H >= PT -> sell at O x (1 - buffer/100) if O > PT else PT;
          else Mtrend == -1 -> sell at Close; else day != buy day AND count >= exit -> sell at Close.
       The sell price is then held inside the bar's Low..High, as AmiBroker's backtester does (a gap open
       above PT with O x (1 - buffer) below the Low fills at the Low: team BANKNIFTY list, 2009-05-18).
       Mtrend = TimeFrameExpand(Supertrend, D/W/M) (expandLast); flt_type 3 = filter off (Mtrend == 1 always)."""
    B = data.bars; n = len(B)
    on = p["flt_type"] == 1
    if on:
        tr, _ = data.trend(p["flt_tf"], p["st_period"], p["st_factor"], p["st_point"])
    k = p["entry_candle"]; ex = p["exit_candle"]
    pos = 0; dab = None; count = 0; PT = 9999999.0; ent = None
    out = []
    for i in range(1, n):
        b = B[i]
        if pos == 1 and b["day"] != dab:
            count += 1
        if k == 0:
            ec = i + 1 >= n or B[i + 1]["day"] != b["day"]
        else:
            ec = (i + k + 1 >= n or B[i + k + 1]["day"] != b["day"]) and (i + k < n and B[i + k]["day"] == b["day"])
        t = tr[i] if on else 1
        if pos == 0 and ec and t == 1:
            pos = 1; dab = b["day"]; ent = i
            if p["pt_pct"] > 0:
                PT = (1 + p["pt_pct"] * 0.01) * b["c"]
        elif pos > 0:
            px = why = None
            if b["h"] >= PT:
                px = b["o"] - b["o"] * p["buffer_pct"] / 100 if b["o"] > PT else PT; why = "pt"
            elif t == -1:
                px = b["c"]; why = "trend"
            elif b["day"] != dab and count >= ex:
                px = b["c"]; why = "time"
            if why:
                px = min(max(px, b["l"]), b["h"])     # AmiBroker backtester: a price outside the bar's L..H is moved to L/H
                out.append(_t(B[ent], b, B[ent]["c"], px, why))
                pos = 0; dab = None; count = 0; PT = 9999999.0
    return out


def run_e34(params=None, data=None, bars=None, variant="slc"):
    """E3/E4 trades. `data` = E34Data (built from 5-min `bars` with p['tf'] if not given)."""
    p = e34_params(variant, **(params or {}))
    if data is None:
        data = E34Data(bars, p["tf"])
    return run_candles(data, p) if variant == "candles" else run_slc(data, p, variant)


FIELDS = ["signal_date", "entry_ts", "entry_px", "exit_date", "exit_ts", "exit_px", "exit_reason"]


def write_ledger(trades, path):
    with open(path, "w", newline="") as f:
        w = csv.DictWriter(f, FIELDS); w.writeheader()
        for t in trades: w.writerow({k: (repr(t[k]) if isinstance(t[k], float) else t[k]) for k in FIELDS})


IEOD_DIR = "/home/aff33/strategy-vault-data/fo-ieod"
FEED_PARQUET = "/tmp/claude-1000/-home-aff33-Desktop-Strategy-Vault/af66f2bb-a019-4872-b960-3149e20bcd5e/scratchpad/intraday/rolling_futures.parquet"
TICKERS = {"NIFTY 50": "n/NIFTY", "NIFTY BANK": "b/BANKNIFTY"}


def phase1_cases():
    """The fixed Phase-1 case matrix: case id -> dict(variant, ticker, src, params)."""
    grid = {"a": {},
            "b": dict(days=3, entry_time=1515, exit_time=920, pb_pct=1.0, hardstop=2.0,
                      flt_type=1, flt_tf=1, st_period=20, st_factor=1),
            "c": dict(days=2, flt_type=1, flt_tf=2, st_period=20, st_factor=3),
            "d": dict(flt_type=2, flt_tf=1, sma_period=200)}
    cases = {}
    for v in VARIANTS:
        for tk in TICKERS:
            tag = "nifty" if tk == "NIFTY 50" else "bank"
            if v == "old_fixed":     # the AFL has no parameters: the no-lookahead and the literal reading
                for la in (0, 1):
                    cases[f"{v}_{tag}_ieod_{'lit' if la else 'a'}"] = dict(variant=v, ticker=tk, src=1,
                                                                        params=spec_params(v, lookahead=la))
                continue
            for g, over in grid.items():
                cases[f"{v}_{tag}_ieod_{g}"] = dict(variant=v, ticker=tk, src=1, params=spec_params(v, **over))
    for v in ("core", "ibs_roc"):
        cases[f"{v}_nifty_feed_a"] = dict(variant=v, ticker="NIFTY 50", src=2, params=spec_params(v))
    return cases


def write_phase1(outdir):
    import json, os
    os.makedirs(outdir, exist_ok=True)
    cases = phase1_cases()
    data = {}
    counts = {}
    for cid, c in cases.items():
        key = (c["ticker"], c["src"])
        if key not in data:
            if c["src"] == 1:
                data[key] = Data(build_5min_ieod(os.path.join(IEOD_DIR, TICKERS[c["ticker"]]))[0])
            else:
                data[key] = Data(build_5min(FEED_PARQUET, drop_1530=True)[0])
        tr = run(c["params"], data=data[key], variant=c["variant"])
        write_ledger(tr, os.path.join(outdir, cid + ".csv"))
        counts[cid] = len(tr)
        print(f"{cid}: {len(tr)}", file=sys.stderr)
    with open(os.path.join(outdir, "cases.json"), "w") as f:
        json.dump({cid: dict(c, ledger=cid + ".csv", trades=counts[cid]) for cid, c in cases.items()}, f, indent=1)
    return counts


def phase1_e34_cases():
    """E3/E4 case matrix: case id -> dict(variant, ticker, src, params)."""
    team = {"teamA": dict(entry_candle=3, exit_candle=3, pt_pct=0.0, flt_type=3),               # NIFTY 5-min WOF list
            "teamB": dict(entry_candle=3, exit_candle=3, pt_pct=0.0, flt_type=1, flt_tf=1, st_period=5, st_factor=2),
            "teamC": dict(tf=2, entry_candle=2, exit_candle=1, pt_pct=7.0, flt_type=1, flt_tf=2, st_period=20, st_factor=3)}
    cases = {}
    for tk in TICKERS:
        tag = "nifty" if tk == "NIFTY 50" else "bank"
        for v in E34_VARIANTS:
            cases[f"{v}_{tag}_ieod_def"] = dict(variant=v, ticker=tk, src=1, params=e34_params(v))
        for v in ("slc_bull", "slc_bear"):
            cases[f"{v}_{tag}_ieod_w20_3"] = dict(variant=v, ticker=tk, src=1, params=e34_params(v, flt_tf=2, st_period=20, st_factor=3))
        for g, over in team.items():
            cases[f"candles_{tag}_ieod_{g}"] = dict(variant="candles", ticker=tk, src=1, params=e34_params("candles", **over))
        cases[f"candles_{tag}_ieod_15m"] = dict(variant="candles", ticker=tk, src=1, params=e34_params("candles", tf=2))
        cases[f"candles_{tag}_ieod_off_pt2"] = dict(variant="candles", ticker=tk, src=1, params=e34_params("candles", flt_type=3, pt_pct=2.0))
    for v in E34_VARIANTS:
        cases[f"{v}_nifty_feed_def"] = dict(variant=v, ticker="NIFTY 50", src=2, params=e34_params(v, src=2))
    return cases


def write_phase1_e34(outdir):
    import json, os
    os.makedirs(outdir, exist_ok=True)
    cases = phase1_e34_cases()
    base, data, counts = {}, {}, {}
    for cid, c in cases.items():
        key = (c["ticker"], c["src"])
        if key not in base:
            base[key] = (build_5min_ieod(os.path.join(IEOD_DIR, TICKERS[c["ticker"]]))[0] if c["src"] == 1
                         else build_5min(FEED_PARQUET, drop_1530=True)[0])
        dk = key + (c["params"]["tf"],)
        if dk not in data:
            data[dk] = E34Data(base[key], c["params"]["tf"])
        tr = run_e34(c["params"], data=data[dk], variant=c["variant"])
        write_ledger(tr, os.path.join(outdir, cid + ".csv"))
        counts[cid] = len(tr)
        print(f"{cid}: {len(tr)}", file=sys.stderr)
    with open(os.path.join(outdir, "cases.json"), "w") as f:
        json.dump({cid: dict(c, ledger=cid + ".csv", trades=counts[cid]) for cid, c in cases.items()}, f, indent=1)
    return counts


def main(argv=None):
    ap = argparse.ArgumentParser()
    ap.add_argument("--ledger")
    ap.add_argument("--phase1", metavar="DIR", help="write the Phase-1 case matrix (cases.json + ledgers) to DIR")
    ap.add_argument("--phase1-e34", metavar="DIR", help="write the E3/E4 case matrix (cases.json + ledgers) to DIR")
    ap.add_argument("--parquet", default=FEED_PARQUET)
    ap.add_argument("--ieod", help="AmiBroker native IEOD file (instead of --parquet)")
    ap.add_argument("--variant", choices=VARIANTS, help="sub-variant; given -> spec defaults, absent -> legacy final-AFL run")
    ap.add_argument("--start"); ap.add_argument("--end")
    ap.add_argument("--drop-1530", action="store_true", help="drop the lone 15:30 bar from the data")
    for k, v in dict(DEFAULTS, **ADDON_DEFAULTS).items():
        ap.add_argument("--" + k.replace("_", "-"), type=type(v), default=None)
    a = ap.parse_args(argv)
    if a.phase1:
        write_phase1(a.phase1); return
    if a.phase1_e34:
        write_phase1_e34(a.phase1_e34); return
    if not a.ledger: ap.error("--ledger or --phase1 required")
    bars, partial = build_5min_ieod(a.ieod) if a.ieod else build_5min(a.parquet, a.drop_1530)
    print(f"{len(bars)} bars, {partial} partial", file=sys.stderr)
    base = spec_params(a.variant) if a.variant else dict(DEFAULTS, **ADDON_DEFAULTS)
    prm = {k: (getattr(a, k) if getattr(a, k) is not None else v) for k, v in base.items()}
    f = dt.date.fromisoformat
    tr = run(prm, bars=bars, start=f(a.start) if a.start else None, end=f(a.end) if a.end else None,
             variant=a.variant or "ibs_roc")
    write_ledger(tr, a.ledger)
    print(f"{len(tr)} trades -> {a.ledger}", file=sys.stderr)


if __name__ == "__main__":
    main()
