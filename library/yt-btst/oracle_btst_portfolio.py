"""BTST Phase 2 portfolio oracle (F&O stock universe), independent of the engine.

Step 1 (per symbol, `candidates()`): the single-symbol rules of oracle_btst.run() (variant 'core', the up-close
rules) give every trade the AFL would signal on that symbol, with its exit as if taken (the AFL's Buy/Sell
ExRem is per symbol and does not depend on the portfolio). Each candidate also carries the ranking inputs:
  price  = intradayclose at the entry bar (= entry price)            PositionScore H->L: price; L->H: 1000000 - price
  ibs    = (HighestSince(startbar,H) - P) / (HH - LL) incl. entry bar  IBS H->L: ibs; L->H: 100 - ibs
  roc<n> = (P - Ref(TimeFrameExpand(Ref(C,-(n-2)),inDaily),-5)) / base * 100   ROC AFL: 100 + roc
  lbc    = number of k in 1..4 for which the daily up-close + increasing-change test holds with lookback days+k
           at the last completed day (the ranking ver 2 / ver 11 'cond'; PositionScore = rank / 1000 - rank)
  nifty  = the NIFTY incremental condition (same lookback on NIFTY futures, NIFTY close at the entry time;
           that AFL defines no PositionScore)
Step 2 (`simulate()`): AmiBroker's portfolio backtester: per bar time, exits first, then the bar's signals
sorted by PositionScore (higher first; ties -> `tie` order), filled up to max_positions minus open positions;
size = pct % of equity (previous-bar equity: cash + open positions at their entry price), whole shares
(floor), AllowPositionShrinking (cash shortage -> fewer shares; < 1 share -> skipped), commission per side.

Run: python -I oracle_btst_portfolio.py --cands  (build/cached candidate tables)
"""
import datetime as dt, math, multiprocessing, os, sys
from concurrent.futures import ProcessPoolExecutor

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import oracle_btst as O  # noqa: E402
import btst_stock_series as S  # noqa: E402

STOCKS = S.OUT
WORK = os.path.expanduser("~/strategy-vault-data/oracle/phase2_work")
UNIVERSE = "FO STOCKS C&P"


def universe():
    import json
    u = json.load(open(os.path.join(STOCKS, "universe.json")))
    return [(r["symbol"], os.path.join(STOCKS, r["file"])) for r in u["stocks"]], u


def load_bars(path, mfilter=None):
    """Oracle bars of one symbol from its .bt5 file; mfilter (analysis only) rebuilds them from the AmiBroker
    file with some minutes removed: 'team' = the team's 2022 copy as reconstructed from the trade lists;
    'no1530', 'no1530all', 's59', 'after1530' = the single hypotheses tested on the way (findings, section 3)."""
    name, days, bars = S.read_bt5(path)
    if mfilter:
        src = _dbfiles()[name]
        if mfilter == "no1530":
            keep = lambda r, d, m: ~((m == 930) & (d >= 20100101))
        elif mfilter == "no1530all":
            keep = lambda r, d, m: m != 930
        elif mfilter == "s59":          # drop records stamped 15:30:59 (the :59-stamped 2011-2015 data)
            keep = lambda r, d, m: ~((m == 930) & (_sec(r) == 59))
        elif mfilter == "team":         # reconstruction of the team's 2022 copy (BTST_Phase2_Portfolio_Findings.md):
            # no 15:30:59 records (2011-2015 data), no records after 15:30:00 before 2009 and on 2021-02-24,
            # no 09:00-09:14 (2010 pre-open) records
            def keep(r, d, m):
                late = (m > 930) | ((m == 930) & (_sec(r) > 0))
                return ~(((m == 930) & (_sec(r) == 59)) | (late & (d < 20090101)) | (late & (d == 20210224))
                         | ((m < 555) & (d >= 20100101) & (d < 20110101)))
        elif mfilter == "after1530":    # drop every record stamped after 15:30:00
            keep = lambda r, d, m: ~((m > 930) | ((m == 930) & (_sec(r) > 0)))
        else:
            raise ValueError(mfilter)
        days, bars = S.build_arrays(src, keep)
    return S.to_oracle_bars(days, bars)


_DBF = None


def _sec(r):
    import numpy as np
    return ((r["u"] & np.uint64(0xffffffff)) >> np.uint64(26)).astype(np.int64)


def _dbfiles():
    global _DBF
    if _DBF is None:
        _DBF = S.db_files(S.DB)
    return _DBF


def _nifty1_cond(nd, nb_close, d_date, P_ts):
    """'1 UPCLOSE of NIFTY' AFL: nfcl2 (NIFTY close at the entry bar) > TimeFrameExpand(NIFTY daily close)."""
    i = nd.day_idx.get(d_date)
    if i is None or i < 1:
        return None
    P = nb_close(P_ts)
    return None if P is None else P > nd.daily[i - 1]["c"]


def _nifty_cond(nd, nb_close, days, d_date, P_ts):
    """NIFTY incremental condition at signal day: nd = NIFTY Data, nb_close(ts) = NIFTY close at the entry time."""
    i = nd.day_idx.get(d_date)
    if i is None:
        return None
    d = i - 1
    if d - (days - 1) < 1:
        return False
    C = lambda x: nd.daily[x]["c"]
    if not all(C(d - q) > C(d - q - 1) for q in range(days - 1)):
        return False
    for q in range(days - 2):
        x = d - q
        if not (C(x) - C(x - 1)) > (C(x - 1) - C(x - 2)):
            return False
    P = nb_close(P_ts)
    if P is None:
        return False
    return P > C(d) and (P - C(d)) > (C(d) - C(d - 1))


_REF = {}


def _ref_ts(mfilter):
    """5-minute bar times of the pad & align reference symbol (NIFTY futures; the team's NIFTY_F1)."""
    if mfilter not in _REF:
        _REF[mfilter] = [b["ts"] for b in load_bars(os.path.join(STOCKS, "NIFTY.bt5"), mfilter)]
    return _REF[mfilter]


def pad_align(bars, ref):
    """AmiBroker 'Pad and align all data to reference symbol': from the symbol's first bar to its last, the
    bar set becomes the reference's; a missing bar is a copy of the previous close (O=H=L=C, V=0), a bar
    the reference does not have is dropped."""
    import bisect, datetime as dt_
    if not bars:
        return bars
    own = {b["ts"]: b for b in bars}
    a = bisect.bisect_left(ref, bars[0]["ts"]); z = bisect.bisect_right(ref, bars[-1]["ts"])
    out, last = [], None
    for t in ref[a:z]:
        b = own.get(t)
        if b is None:
            if last is None:
                continue
            c = last["c"]
            b = dict(ts=t, hhmm=t.hour * 100 + t.minute, day=t.date(), o=c, h=c, l=c, c=c, v=0.0)
        out.append(b); last = b
    return out


def _upclose_ok(C, d, n):
    """Sum(C>Ref(C,-1), n-1) == n-1 AND Sum(DIFF>Ref(DIFF,-1), n-2) == n-2 at daily index d (Null -> False)."""
    if d - (n - 1) < 0:
        return False
    if not all(C[d - q] > C[d - q - 1] for q in range(n - 1)):
        return False
    for q in range(n - 2):
        x = d - q
        if x - 2 < 0 or not (C[x] - C[x - 1]) > (C[x - 1] - C[x - 2]):
            return False
    return True


def candidates(sym, path, cfg, nifty=None):
    """cfg: dict(days, entry_time, exit_time, pb_pct, hardstop, flt_type, ..., roc_days list).
    Returns list of tuples (see COLS)."""
    bars = load_bars(path, cfg.get("mfilter"))
    if cfg.get("pad"):
        bars = pad_align(bars, _ref_ts(cfg.get("mfilter")))
    data = O.Data(bars)
    p = O.spec_params("core", **{k: v for k, v in cfg.items() if k in O.SPEC_DEFAULTS})
    tr = O.run(p, data=data, variant="core")
    B, D = data.bars, data.daily
    idx = {b["ts"]: k for k, b in enumerate(B)}
    C = [x["c"] for x in D]
    out = []
    for t in tr:
        ets = dt.datetime.fromisoformat(t["entry_ts"])
        k = idx[ets]
        day = B[k]["day"]; k0 = data.first[day]; i = data.day_idx[day]; d = i - 1
        if d < 2:
            # buycond2 = Sum(DIFF > Ref(DIFF,-1), days-2) is Null until DIFF > Ref(DIFF,-1) is defined (needs
            # 3 daily bars), also for days = 2 where the Sum has 0 terms: the team never trades the 3rd day of
            # a symbol's data (BTST_Phase2_Portfolio_Findings.md). oracle_btst.run() checks this only for days >= 3.
            continue
        P = B[k]["c"]
        ih = max(b["h"] for b in B[k0:k + 1]); il = min(b["l"] for b in B[k0:k + 1])
        ibs = (ih - P) / (ih - il) if ih != il else None
        rocs = []
        for n in cfg.get("roc_days", (3, 4)):
            kk = k - 5
            r = None
            if kk >= 0:
                di = data.day_idx[B[kk]["day"]] - 1
                xi = di - (n - 2)
                if 0 <= di and 0 <= xi < len(D):
                    r = (P - D[xi]["c"]) / D[xi]["c"] * 100
            rocs.append(r)
        lbc = sum(_upclose_ok(C, d, cfg["days"] + j) for j in range(1, 5))
        lbc1 = sum(_upclose_ok(C, i, cfg["days"] + j) for j in range(1, 5))   # same, at the signal day's own close
        nf = nifty(cfg["days"], day, ets) if nifty else None
        nf1 = nifty(None, day, ets) if nifty else None
        # volume add-on (IBS + volume multiplier AFL, = Phase-1 'volume' rule): volume since the last bar at or
        # after the entry time (exclusive) up to the entry bar, over MA(daily V, vdays=5); vol_ratio > multiplier
        r_ = k - 1
        while r_ >= 1 and B[r_]["hhmm"] < cfg["entry_time"]:
            r_ -= 1
        vol = sum(B[x]["v"] for x in range(r_ + 1 if r_ >= 1 else 1, k + 1))
        nv = cfg.get("vol_days", 5)
        ma = sum(D[x]["v"] for x in range(d - nv + 1, d + 1)) / nv if d - nv + 1 >= 0 else None
        vr = vol / ma if ma else None
        out.append((sym, t["signal_date"], t["entry_ts"], t["entry_px"], t["exit_date"], t["exit_ts"], t["exit_px"],
                    t["exit_reason"], ibs, rocs[0], rocs[1] if len(rocs) > 1 else None, lbc, lbc1, nf, nf1, vr))
    return out


COLS = ["symbol", "signal_date", "entry_ts", "entry_px", "exit_date", "exit_ts", "exit_px", "exit_reason",
        "ibs", "roc_a", "roc_b", "lbc", "lbc1", "nifty", "nifty1", "vol_ratio"]

_NIFTY = {}


def _nifty_fn(mfilter=None):
    """NIFTY futures (n/NIFTY = the team's NIFTY_F1) with the same minute filter as the stocks."""
    if mfilter not in _NIFTY:
        nb = load_bars(os.path.join(STOCKS, "NIFTY.bt5"), mfilter)
        nd = O.Data(nb)
        ts = [b["ts"] for b in nb]; cl = [b["c"] for b in nb]
        import bisect

        def close_at(t):
            j = bisect.bisect_right(ts, t) - 1      # Foreign() aligned to the stock: NIFTY's last bar at or before t
            return cl[j] if j >= 0 and nb[j]["day"] == t.date() else None
        _NIFTY[mfilter] = (nd, close_at)
    nd, close_at = _NIFTY[mfilter]
    return lambda days, day, ets: (_nifty1_cond(nd, close_at, day, ets) if days is None
                                   else _nifty_cond(nd, close_at, days, day, ets))


def _job(a):
    sym, path, cfg, with_nifty = a
    return candidates(sym, path, cfg, _nifty_fn(cfg.get("mfilter")) if with_nifty else None)


def cfg_key(cfg):
    k = "lb{days}_en{entry_time}_ex{exit_time}_pb{pb_pct}_hs{hardstop}_f{flt_type}".format(**cfg)
    k += ("_" + cfg["mfilter"] if cfg.get("mfilter") else "") + ("_pad" if cfg.get("pad") else "")
    rd = tuple(cfg.get("roc_days", (3, 4)))
    return k + ("" if rd == (3, 4) else "_roc" + "-".join(map(str, rd)))


def build_candidates(cfg, jobs=6, with_nifty=True, cache=True):
    """All-universe candidate table (polars DataFrame), cached under WORK."""
    import polars as pl
    os.makedirs(WORK, exist_ok=True)
    fn = os.path.join(WORK, f"cands_{cfg_key(cfg)}.parquet")
    if cache and os.path.exists(fn):
        df = pl.read_parquet(fn)
        if "vol_ratio" in df.columns:          # older caches lack nifty1 / vol_ratio: rebuild
            return df
    syms, _ = universe()
    rows = []
    with ProcessPoolExecutor(jobs, mp_context=multiprocessing.get_context("fork")) as pool:
        for r in pool.map(_job, [(s, p, cfg, with_nifty) for s, p in syms], chunksize=2):
            rows.extend(r)
    df = pl.DataFrame(rows, schema=dict(symbol=pl.Utf8, signal_date=pl.Utf8, entry_ts=pl.Utf8, entry_px=pl.Float64,
                                        exit_date=pl.Utf8, exit_ts=pl.Utf8, exit_px=pl.Float64, exit_reason=pl.Utf8,
                                        ibs=pl.Float64, roc_a=pl.Float64, roc_b=pl.Float64, lbc=pl.Int64, lbc1=pl.Int64,
                                        nifty=pl.Boolean, nifty1=pl.Boolean, vol_ratio=pl.Float64), orient="row")
    df.write_parquet(fn)
    return df


# ----------------------------------------------------------------------------- portfolio
def score_of(rank, r):
    """PositionScore of candidate row r (dict) for a ranking id; None = Null/undefined."""
    if rank == "price_hl":
        return r["entry_px"]
    if rank == "price_lh":
        return 1000000 - r["entry_px"]
    if rank == "ibs_hl":
        return r["ibs"]
    if rank == "ibs_lh":
        return None if r["ibs"] is None else 100 - r["ibs"]
    if rank == "roc_a":
        return None if r["roc_a"] is None else 100 + r["roc_a"]
    if rank == "roc_b":
        return None if r["roc_b"] is None else 100 + r["roc_b"]
    if rank == "ibs_vol":      # IBS ranking (H->L) + volume multiplier AFL: PositionScore = ibs (filter: vol_ratio)
        return r["ibs"]
    if rank == "ibs_roc":      # root 'IBS flt and ROC ranking' AFL: PositionScore = IBS + 100 + PCTDIFF (roc_a)
        return None if r["ibs"] is None or r["roc_a"] is None else r["ibs"] + 100 + r["roc_a"]
    if rank == "none":
        return 0.0
    raise ValueError(rank)


def lookback_rank_fn(version=2, tie_key=None):
    """Ranking ver 2 / ver 11: StaticVarGenerateRanks(cond, tiemode 1234) over the watchlist -> rank 1 = highest
    cond, equal cond ranked in list (alphabetical) order; PositionScore = rank (ver 2: lower cond first, among
    equal cond the later symbol) or 1000 - rank (ver 11: higher cond first, earlier symbol). The team's lists
    show a different, hidden but fixed per-symbol order among equal cond (findings section 4: OPEN)."""
    key = tie_key or sym_key

    def fn(sigs):
        o = sorted(range(len(sigs)), key=lambda j: key(sigs[j]["symbol"]))
        o.sort(key=lambda j: -sigs[j]["lbc"])                  # rank 1 = highest cond, ties in list order
        rk = [0] * len(sigs)
        for n, j in enumerate(o):
            rk[j] = n + 1
        return rk if version == 2 else [1000 - x for x in rk]
    return fn


def sym_key(s):
    """Tie order = ascending symbol name (BTST_Phase2_Portfolio_Findings.md section 2: AmiBroker takes equal
    PositionScores, and every signal when PositionScore is not defined, in alphabetical order)."""
    return s.upper()


class PrevClose:
    """mark(symbol, ts) -> close of the symbol's last bar BEFORE ts (UsePrevBarEquityForPosSizing marks open
    positions at the previous bar). Loads a symbol's .bt5 only when a position is held over an entry bar."""
    def __init__(self, mfilter=None, paths=None):
        self.mfilter = mfilter; self.paths = paths or dict(universe()[0]); self.cache = {}

    def __call__(self, sym, ts):
        import bisect
        if sym not in self.cache:
            if len(self.cache) > 64:
                self.cache.clear()
            b = load_bars(self.paths[sym], self.mfilter)
            self.cache[sym] = ([x["ts"].isoformat(sep=" ") for x in b], [x["c"] for x in b])
        T, C = self.cache[sym]
        j = bisect.bisect_left(T, ts) - 1
        return C[j] if j >= 0 else None


def simulate(rows, max_pos, rank, pct=None, equity0=1e6, comm=0.0003, tie="alpha", null_score=0.0,
             shrink=True, rank_fn=None, tie_key=None, mark=None):
    """AmiBroker portfolio backtester for BTST candidates (rules settled in BTST_Phase2_Portfolio_Findings.md).
    rows: candidate dicts (symbol, entry_ts, entry_px, exit_ts, exit_px, ...); a candidate without an exit is
    not a row (a signal on a symbol's last data day is never entered and takes no slot).
    Per entry bar time: (1) positions whose exit_ts <= this bar are closed first (cash += value - commission);
    (2) the bar's signals are ordered by PositionScore (higher first; undefined/Null -> null_score; ties and
    the no-PositionScore case -> ascending tie_key(symbol), default the upper-case name), symbols already held
    are skipped, filled until max_pos positions are open; (3) size = floor(E x pct / 100 / price) whole
    shares, E = equity at the previous bar = cash + open positions marked by mark(symbol, ts) (default: entry
    price); if that cost plus commission exceeds the cash, AllowPositionShrinking buys
    floor(cash / (price x (1 + comm))); a signal is skipped only when that is < 1 share.
    Returns the taken trades: row + shares, entry_value, score, weight (= entry_value / E)."""
    pct = pct if pct is not None else 100.0 / max_pos
    key = tie_key or sym_key
    by_ts = {}
    for r in rows:
        by_ts.setdefault(r["entry_ts"], []).append(r)
    times = sorted(by_ts)
    cash = equity0
    open_ = []          # (exit_ts, row, shares)
    taken = []
    for ts in times:
        # exits at or before this bar time (exits are processed before entries on the same bar)
        keep = []
        for x in open_:
            if x[0] <= ts:
                r, sh = x[1], x[2]
                val = sh * r["exit_px"]
                cash += val - val * comm
            else:
                keep.append(x)
        open_ = keep
        sigs = by_ts[ts]
        if rank_fn is not None:
            sc = rank_fn(sigs)
        else:
            sc = [score_of(rank, r) for r in sigs]
        sc = [null_score if (s is None or (isinstance(s, float) and math.isnan(s))) else s for s in sc]
        order = list(range(len(sigs)))
        if tie == "alpha":
            order.sort(key=lambda j: key(sigs[j]["symbol"]))
        elif tie == "ralpha":
            order.sort(key=lambda j: key(sigs[j]["symbol"]), reverse=True)
        order.sort(key=lambda j: -sc[j])            # stable: ties keep the tie order
        held = {x[1]["symbol"] for x in open_}
        equity = cash
        for x in open_:
            m = mark(x[1]["symbol"], ts) if mark is not None else None
            equity += x[2] * (m if m is not None else x[1]["entry_px"])
        for j in order:
            if len(open_) >= max_pos:
                break
            r = sigs[j]
            if r["symbol"] in held:
                continue
            px = r["entry_px"]
            want = equity * pct / 100.0
            sh = math.floor(want / px + 1e-9)
            cost = sh * px * (1 + comm)
            if cost > cash:
                if not shrink:
                    continue
                sh = math.floor(cash / (px * (1 + comm)) + 1e-9)
            if sh < 1:
                continue
            cash -= sh * px * (1 + comm)
            open_.append((r["exit_ts"], r, sh))
            held.add(r["symbol"])
            taken.append(dict(r, shares=sh, entry_value=sh * px, score=sc[j], weight=sh * px / equity))
    return taken


# ----------------------------------------------------------------------------- Phase-2 ledgers
PHASE2_DEFAULTS = dict(days=2, entry_time=1450, exit_time=925, pb_pct=0.0, hardstop=0.0, flt_type=3, src=1,
                       universe=UNIVERSE, equity=1000000.0, comm_pct=0.03, pad=0)
# variant id -> (extra params, how the oracle ranks/filters: rank id or 'lookback', row filter)
PHASE2_VARIANTS = {
    "ytbtst_core": [dict()],
    "ytbtst_rank_price": [dict(rank_dir=1), dict(rank_dir=2)],          # 1 = high to low, 2 = low to high
    "ytbtst_rank_ibs": [dict(rank_dir=1), dict(rank_dir=2)],
    "ytbtst_rank_lookback": [dict(lb_version=2), dict(lb_version=11)],
    "ytbtst_rank_roc": [dict(roc_days=3)],                              # AFL ROC DAYS 3 = 2-day ROC (team lists)
    "ytbtst_rank_ibs_volume": [dict(vol_days=5, vol_mult=1.0)],
    "ytbtst_nifty_upclose": [dict()],
    "ytbtst_nifty_incremental": [dict()],
}


def phase2_rows(variant, prm, rows):
    """-> (rows passing the sub-variant's extra condition, rank id or None, rank_fn or None)."""
    if variant == "ytbtst_core":
        return rows, "none", None
    if variant == "ytbtst_rank_price":
        return rows, "price_hl" if prm["rank_dir"] == 1 else "price_lh", None
    if variant == "ytbtst_rank_ibs":
        return rows, "ibs_hl" if prm["rank_dir"] == 1 else "ibs_lh", None
    if variant == "ytbtst_rank_lookback":
        return rows, None, lookback_rank_fn(prm["lb_version"])
    if variant == "ytbtst_rank_roc":
        assert prm["roc_days"] in (3, 4)
        return rows, "roc_a" if prm["roc_days"] == 3 else "roc_b", None
    if variant == "ytbtst_rank_ibs_volume":
        return [r for r in rows if r["vol_ratio"] is not None and r["vol_ratio"] > prm["vol_mult"]], "ibs_vol", None
    if variant == "ytbtst_nifty_upclose":
        return [r for r in rows if r["nifty1"]], "none", None
    if variant == "ytbtst_nifty_incremental":
        return [r for r in rows if r["nifty"]], "none", None
    raise ValueError(variant)


def phase2_cases():
    """case id -> dict(variant, params): the defaults of every runnable stock sub-variant at 10 and 20 positions,
    plus the team-list settings (exit 09:15; lookback 3 lists; pad & align for the Foreign-using AFLs)."""
    cases = {}
    for v, opts in PHASE2_VARIANTS.items():
        for o in opts:
            for n in (10, 20):
                tag = "_".join(f"{k}{val}" for k, val in o.items() if k in ("rank_dir", "lb_version"))
                cid = f"{v}{'_' + tag if tag else ''}_{n}"
                cases[cid] = dict(variant=v, params=dict(PHASE2_DEFAULTS, max_positions=n, pct=100.0 / n, **o))
    team = [("ytbtst_rank_price", dict(rank_dir=1), (10, 20), (2, 3), 0), ("ytbtst_rank_price", dict(rank_dir=2), (10, 20), (2,), 0),
            ("ytbtst_rank_ibs", dict(rank_dir=1), (10, 20), (2,), 0), ("ytbtst_rank_ibs", dict(rank_dir=2), (10, 20), (2,), 0),
            ("ytbtst_rank_roc", dict(roc_days=3), (20,), (2,), 0), ("ytbtst_rank_roc", dict(roc_days=4), (20,), (3,), 0),
            ("ytbtst_rank_lookback", dict(lb_version=2), (10, 20), (2,), 1), ("ytbtst_nifty_incremental", dict(), (10, 20), (2,), 1)]
    for v, o, ns, lbs, pad in team:
        for n in ns:
            for lb in lbs:
                tag = "_".join(f"{k}{val}" for k, val in o.items())
                cid = f"team_{v}{'_' + tag if tag else ''}_{n}_lb{lb}"
                cases[cid] = dict(variant=v, params=dict(PHASE2_DEFAULTS, days=lb, exit_time=915, pad=pad,
                                                         max_positions=n, pct=100.0 / n, **o))
    return cases


LEDGER_FIELDS = ["symbol", "signal_date", "entry_ts", "entry_px", "exit_date", "exit_ts", "exit_px", "exit_reason", "weight"]


def write_phase2(outdir, jobs=6):
    import csv, json
    os.makedirs(outdir, exist_ok=True)
    cases = phase2_cases()
    counts = {}
    mark = PrevClose()
    for cid, c in cases.items():
        p = c["params"]
        cfg = dict(days=p["days"], entry_time=p["entry_time"], exit_time=p["exit_time"], pb_pct=p["pb_pct"],
                   hardstop=p["hardstop"], flt_type=p["flt_type"], roc_days=(3, 4))
        if p["pad"]:
            cfg["pad"] = True
        rows = build_candidates(cfg, jobs).to_dicts()
        rows, rank, fn = phase2_rows(c["variant"], p, rows)
        taken = simulate(rows, p["max_positions"], rank or "none", pct=p["pct"], equity0=p["equity"],
                         comm=p["comm_pct"] / 100, rank_fn=fn, mark=mark)
        with open(os.path.join(outdir, cid + ".csv"), "w", newline="") as f:
            w = csv.DictWriter(f, LEDGER_FIELDS); w.writeheader()
            for t in taken:
                w.writerow({k: (repr(t[k]) if isinstance(t[k], float) else t[k]) for k in LEDGER_FIELDS})
        counts[cid] = len(taken)
        print(f"{cid}: {len(taken)}", file=sys.stderr, flush=True)
    meta = dict(_rules="BTST/BTST_Phase2_Portfolio_Findings.md section 2; prices = .bt5 as stored (src 1); "
                       "weight = entry value / previous-bar equity; commission comm_pct % per side; tie order = "
                       "ascending symbol name; rank_dir 1 = high to low, 2 = low to high")
    with open(os.path.join(outdir, "cases.json"), "w") as f:
        json.dump(dict(meta, **{cid: dict(c, ledger=cid + ".csv", trades=counts[cid]) for cid, c in cases.items()}),
                  f, indent=1)
    return counts


def main(argv=None):
    import argparse
    ap = argparse.ArgumentParser()
    ap.add_argument("--cands", action="store_true")
    ap.add_argument("--phase2", metavar="DIR", help="write the Phase-2 case ledgers + cases.json to DIR")
    ap.add_argument("--days", type=int, default=2); ap.add_argument("--entry", type=int, default=1450)
    ap.add_argument("--exit", type=int, default=915); ap.add_argument("--jobs", type=int, default=6)
    a = ap.parse_args(argv)
    if a.phase2:
        write_phase2(os.path.expanduser(a.phase2), a.jobs)
        return
    cfg = dict(days=a.days, entry_time=a.entry, exit_time=a.exit, pb_pct=0.0, hardstop=0.0, flt_type=3,
               roc_days=(3, 4))
    import time
    t0 = time.time()
    df = build_candidates(cfg, a.jobs, cache=False)
    print(f"{cfg_key(cfg)}: {len(df)} candidates in {time.time() - t0:.0f} s", file=sys.stderr)


if __name__ == "__main__":
    main()
