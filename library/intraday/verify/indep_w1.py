#!/usr/bin/env python3
"""Independent reference for the intraday wave-1 rules, written from the book text (rule quotes and page cites in each
module's catalogue), not from the Rust code. Reads the daily file and the cleaned 1-minute CSV through indep_ref's
loaders (they stream the CSV themselves and assert every minute is finite and OHLC-consistent).

Rules (defaults = the book):
  ats_10pct_oops (ATS Ch 21) / btp2_10pct_oops[_long|_short] (Vol. II): day one closes in the bottom (top) 10% of its range;
      day two opens below its low (above its high): buy (sell) stop at the day-one low (high); MOC; no stop.
  ats_wide_range_gap (ATS Ch 23): day one moves >= 2 SD close to close (1 SD = prior close x HV100 / sqrt(256), HV = sample
      SD of 99 log returns x sqrt(260), measured the day before the move); day two opens beyond the day-one extreme: stop
      entry at the day-one close, protective stop at the day-one extreme; close in the top (bottom) 15% of the range ->
      carry to the next open, else out on the close.
  btp2_wide_range_exhaustion_gap_short (Vol. II): same sells, 1 SD = close x HV100 / sqrt(252), HV = sample SD of 100 log
      returns x sqrt(252); stop at the day-one high.
  hfm_vix (Hit and Run / VIX day-trading program, SPX only): VIX close >= 15 -> buy stop .50 pt above yesterday's high
      unless today opened above it; <= 11 -> sell stop .50 below the low unless opened below; stop 2.00 pts from the fill;
      MOC; an unfilled inside day carries the side to the next day.
  tdnmtt_trap_long/_short (DeMark p.233): open inside yesterday's true range; one tick above the true high / below the
      true low, whichever breaks first; out on the close. Tick = 0.01 US, 0.05 NSE.
  baboops_oops (Babcock p.12-17, SPX only): open gapped > 15 ticks (0.75) beyond yesterday's low/high: buy (sell) stop at
      that extreme; out on the close; no stop.
  tcdt same-day ORBs (Crabel, SPX only): Day-2 pattern, then today a stop/limit at today's open +/- the printed S&P points;
      out on the close; no stops. Row data (pattern, points, side) = the printed tables as transcribed in books/tcdt.rs.
  lwdg2_0105 (Williams 1985 p.133): buy stop at yesterday's close x 1.0105, sell stop at x 0.9895, the first touched;
      out on the close; no stop.
  lwlts_mon_vb (Williams LTS p.87, SPX only): Mondays, buy stop at the open + 0.05 x Friday's range; $3,000 stop
      (12 S&P points); out on the close.
  jb30d_gap_pct (Bernstein p.100-102): open beyond yesterday's low/high by >= 5% of yesterday's range; stop entry 5% of
      the range back inside; stop 25% of the range from the fill; out at the end of the day.
Minute conventions (stated on the cards): a level fills at the level or at the minute's open when the minute opened
through it; two levels in one minute without opening through exactly one = unknown -> no trade (listed); in the fill
minute a stop counts if touched, except when that minute opened beyond the stop: then only if it closed beyond it.
"""
import math, os, re, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from indep_ref import load_daily, Minutes  # noqa: E402  (loaders only)

TCDT_RS = os.path.join(os.path.dirname(os.path.abspath(__file__)), "../../../Context/guru/backend/src/books/tcdt.rs")


def tcdt_rows():
    """Printed ORB rows (id, pattern, signed points, side, hold) - table data only."""
    txt = open(TCDT_RS).read()
    rows = {}
    for m in re.finditer(r'orb!\("(tcdt_[a-z0-9_]+)","[^"]*",(\w+),(-?[0-9.]+),([BS]),([0-9.]+),', txt):
        rows[m.group(1)] = (m.group(2), float(m.group(3)), 1.0 if m.group(4) == "B" else -1.0, float(m.group(5)))
    return rows


TCDT = tcdt_rows()
TCDT_SAME = [k for k, v in TCDT.items() if v[3] == 0.0] + ["tcdt_gap_orb", "tcdt_triseg_orb", "tcdt_quad_orb"]
# table row 1 of each (default param row = 1): (gap pts, gap dir) / (open seg, close seg) and (points, side)
GAP1 = (0.2, 1.0, 1.6, 1.0)
TRI1 = (3, 3, 0.8, -1.0)
QUAD1 = (5, 5, 0.8, 1.0)


def load_vix(path="/home/server/.cache/strategy-vault-webapp/data/us_vix/VIX.csv"):
    v = {}
    for line in open(path).read().splitlines()[1:]:
        f = line.split(",")
        v[f[1]] = float(f[6])
    return v


# ---------------------------------------------------------------- minute order
def hit(b, up, px):
    return b[2] >= px if up else b[3] <= px


def fill(b, up, px):
    return max(b[1], px) if up else min(b[1], px)


def first(bars, start, levels):
    """('hit', k, bar, px) | ('amb', bar) | None"""
    for j in range(start, len(bars)):
        b = bars[j]
        ks = [k for k, (up, px) in enumerate(levels) if hit(b, up, px)]
        if not ks:
            continue
        if len(ks) == 1:
            return ("hit", ks[0], j, fill(b, *levels[ks[0]]))
        thr = [k for k in ks if (b[1] >= levels[k][1] if levels[k][0] else b[1] <= levels[k][1])]
        if len(thr) == 1:
            return ("hit", thr[0], j, b[1])
        return ("amb", j)
    return None


def day(bars, levels, stop_of, amb, tag):
    """Entry k, fill, stop exit or None.  None when not filled / unknown order."""
    t = first(bars, 0, levels)
    if t is None:
        return None
    if t[0] == "amb":
        amb.append((tag, "two entry levels in one minute: no trade"))
        return None
    _, k, j, px = t
    s = stop_of(k, px)
    if s is None:
        return k, px, None
    up, sp = s
    b = bars[j]
    beyond = (lambda v: v >= sp) if up else (lambda v: v <= sp)
    if beyond(b[1]):
        if beyond(b[4]):
            amb.append((tag, "fill minute opened beyond the stop and closed beyond it: stopped"))
            return k, px, sp
        amb.append((tag, "fill minute opened beyond the stop and closed inside it: not stopped in that minute"))
    elif hit(b, up, sp):
        amb.append((tag, "fill minute also reached the stop: filled then stopped"))
        return k, px, sp
    t2 = first(bars, j + 1, [s])
    return k, px, (t2[3] if t2 else None)


# ---------------------------------------------------------------- daily helpers
def cloc(D, i):
    r = D[i][2] - D[i][3]
    return (D[i][4] - D[i][3]) / r if r > 0 else math.nan


def hv_sample(C, nret, ann):
    """sample SD of the nret log returns ending at bar i, x sqrt(ann) x 100"""
    out = [math.nan] * len(C)
    lr = [math.nan] + [math.log(C[i] / C[i - 1]) for i in range(1, len(C))]
    for i in range(nret, len(C)):
        w = lr[i - nret + 1:i + 1]
        m = sum(w) / nret
        out[i] = math.sqrt(sum((x - m) ** 2 for x in w) / (nret - 1)) * math.sqrt(ann) * 100
    return out


def weekday(d):
    import datetime
    return datetime.date(int(d[:4]), int(d[5:7]), int(d[8:10])).weekday()


def tick_of(t):
    return 0.05 if t.startswith("NIFTY") else 0.01


def rng(D, j):
    return D[j][2] - D[j][3]


def pattern(D, pat, j):
    o, h, l, c = (lambda k: D[k][1]), (lambda k: D[k][2]), (lambda k: D[k][3]), (lambda k: D[k][4])
    narrow = lambda n: j >= n and all(rng(D, j) < rng(D, j - k) for k in range(1, n + 1))
    wide = lambda n: j >= n and all(rng(D, j) > rng(D, j - k) for k in range(1, n + 1))
    inside = j >= 1 and h(j) < h(j - 1) and l(j) > l(j - 1)
    doji = abs(o(j) - c(j)) < 0.50

    def nbar(n, look=20):
        if j < look + n:
            return False
        R = lambda e: max(h(k) for k in range(e - n + 1, e + 1)) - min(l(k) for k in range(e - n + 1, e + 1))
        return all(R(j) < R(e) for e in range(j - look + n - 1, j))
    return {"Any": True, "Id": inside, "Doji": doji, "Nr4": narrow(3), "DojiNr4": doji and narrow(3), "IdNr4": inside and narrow(3),
            "Nr7": narrow(6), "Ws4": wide(3), "Ws7": wide(6),
            "BearHook": j >= 1 and o(j) < l(j - 1) and c(j) > c(j - 1) and rng(D, j) < rng(D, j - 1),
            "BullHook": j >= 1 and o(j) > h(j - 1) and c(j) < c(j - 1) and rng(D, j) < rng(D, j - 1),
            "DojiR1": doji and rng(D, j) < 2.0, "R1": rng(D, j) < 2.0, "Bnr2": nbar(2), "Bnr3": nbar(3)}[pat]


def seg(lo, hi, p, parts):
    r = hi - lo
    if not r > 0:
        return None
    return min(max(int(math.floor((p - lo) / r * parts)) + 1, 1), parts)


# ---------------------------------------------------------------- rules
def backtest(id_, D, M, a, b, ticker, amb, vix=None):
    out = []

    def tr(dr, i, px, xi, xp, kind, opn=False):
        out.append({"dir": int(dr), "entry": D[i][0], "entry_px": px, "exit": D[xi][0], "exit_px": xp, "reason": kind, "open": opn})

    O = [x[1] for x in D]; H = [x[2] for x in D]; L = [x[3] for x in D]; C = [x[4] for x in D]
    if id_ in ("ats_10pct_oops", "btp2_10pct_oops", "btp2_10pct_oops_long", "btp2_10pct_oops_short"):
        lo_ok = not id_.endswith("_short"); sh_ok = not id_.endswith("_long")
        for i in range(max(a, 2 if id_.startswith("btp2") else 1), b + 1):
            k = i - 1; q = cloc(D, k)
            if lo_ok and q <= 0.10 and O[i] < L[k]:
                dr, lv = 1, L[k]
            elif sh_ok and q >= 0.90 and O[i] > H[k]:
                dr, lv = -1, H[k]
            else:
                continue
            bars = M.day(D[i][0])
            if bars is None:
                continue
            r = day(bars, [(dr > 0, lv)], lambda k_, px: None, amb, D[i][0])
            if r:
                tr(dr, i, r[1], i, C[i], "close")
    elif id_ in ("ats_wide_range_gap", "btp2_wide_range_exhaustion_gap_short"):
        ats = id_ == "ats_wide_range_gap"
        hv = hv_sample(C, 99, 260) if ats else hv_sample(C, 100, 252)
        div = math.sqrt(256) if ats else math.sqrt(252)
        for i in range(max(a, 2), b + 1):
            k = i - 1
            sd = C[k - 1] * hv[k - 1] / 100 / div
            mv = C[k] - C[k - 1]
            if ats and mv <= -2 * sd and O[i] < L[k]:
                dr, lv, sp = 1, C[k], L[k]
            elif mv >= 2 * sd and O[i] > H[k]:
                dr, lv, sp = -1, C[k], H[k]
            else:
                continue
            bars = M.day(D[i][0])
            if bars is None:
                continue
            r = day(bars, [(dr > 0, lv)], lambda k_, px: (dr < 0, sp), amb, D[i][0])
            if not r:
                continue
            q = cloc(D, i)
            strong = (q >= 0.85) if dr > 0 else (q <= 0.15)
            if r[2] is not None:
                tr(dr, i, r[1], i, r[2], "stop")
            elif strong and i < b:
                tr(dr, i, r[1], i + 1, O[i + 1], "close")
            elif strong:
                tr(dr, i, r[1], i, C[i], "open", True)
            else:
                tr(dr, i, r[1], i, C[i], "close")
    elif id_ == "hfm_vix":
        carry = 0
        for i in range(max(a, 1), b + 1):
            v = vix.get(D[i - 1][0], math.nan)
            side = 1 if v >= 15 else -1 if v <= 11 else carry
            carry = 0
            if side == 0:
                continue
            if (side > 0 and O[i] > H[i - 1]) or (side < 0 and O[i] < L[i - 1]):
                continue
            bars = M.day(D[i][0])
            if bars is None:
                continue
            lv = H[i - 1] + 0.5 if side > 0 else L[i - 1] - 0.5
            r = day(bars, [(side > 0, lv)], lambda k_, px: (side < 0, px - side * 2.0), amb, D[i][0])
            if r:
                tr(side, i, r[1], i, r[2] if r[2] is not None else C[i], "stop" if r[2] is not None else "close")
            elif H[i] < H[i - 1] and L[i] > L[i - 1]:
                carry = side
    elif id_ in ("tdnmtt_trap_long", "tdnmtt_trap_short"):
        t = tick_of(ticker); want = 1 if id_.endswith("_short") else 0
        for i in range(max(a, 2), b + 1):
            th, tl = max(H[i - 1], C[i - 2]), min(L[i - 1], C[i - 2])
            if not (tl <= O[i] <= th):
                continue
            bars = M.day(D[i][0])
            if bars is None:
                continue
            r = day(bars, [(True, th + t), (False, tl - t)], lambda k_, px: None, amb, D[i][0])
            if r and r[0] == want:
                tr(-1 if want else 1, i, r[1], i, C[i], "close")
    elif id_ == "baboops_oops":
        g = round(15 * 0.05, 6)
        for i in range(max(a, 1), b + 1):
            if O[i] < L[i - 1] - g - 1e-9:
                dr, lv = 1, L[i - 1]
            elif O[i] > H[i - 1] + g + 1e-9:
                dr, lv = -1, H[i - 1]
            else:
                continue
            bars = M.day(D[i][0])
            if bars is None:
                continue
            r = day(bars, [(dr > 0, lv)], lambda k_, px: None, amb, D[i][0])
            if r:
                tr(dr, i, r[1], i, C[i], "close")
    elif id_.startswith("tcdt_"):
        for i in range(max(a, 1), b + 1):
            j = i - 1
            if id_ == "tcdt_gap_orb":
                gap, gdir, pts, side = GAP1
                ok = (O[i] - C[j]) * gdir >= gap - 1e-9
            elif id_ in ("tcdt_triseg_orb", "tcdt_quad_orb"):
                so, sc, pts, side = TRI1 if id_ == "tcdt_triseg_orb" else QUAD1
                parts = 3 if id_ == "tcdt_triseg_orb" else 5
                ok = seg(L[j], H[j], O[j], parts) == so and seg(L[j], H[j], C[j], parts) == sc
            else:
                pat, pts, side, _ = TCDT[id_]
                ok = pattern(D, pat, j)
            if not ok:
                continue
            bars = M.day(D[i][0])
            if bars is None:
                continue
            r = day(bars, [(pts >= 0, O[i] + pts)], lambda k_, px: None, amb, D[i][0])
            if r:
                tr(side, i, r[1], i, C[i], "close")
    elif id_ == "lwdg2_0105":
        for i in range(max(a, 1), b + 1):
            bars = M.day(D[i][0])
            if bars is None:
                continue
            dlt = 0.0105 * C[i - 1]
            r = day(bars, [(True, C[i - 1] + dlt), (False, C[i - 1] - dlt)], lambda k_, px: None, amb, D[i][0])
            if r:
                tr(1 if r[0] == 0 else -1, i, r[1], i, C[i], "close")
    elif id_ == "lwlts_mon_vb":
        for i in range(max(a, 1), b + 1):
            if weekday(D[i][0]) != 0:
                continue
            bars = M.day(D[i][0])
            if bars is None:
                continue
            r = day(bars, [(True, O[i] + 0.05 * rng(D, i - 1))], lambda k_, px: (False, px - 3000 / 250), amb, D[i][0])
            if r:
                tr(1, i, r[1], i, r[2] if r[2] is not None else C[i], "stop" if r[2] is not None else "close")
    elif id_ == "jb30d_gap_pct":
        for i in range(max(a, 1), b + 1):
            R = rng(D, i - 1); gs = pa = 0.05 * R
            if O[i] <= L[i - 1] - gs and O[i] < L[i - 1]:
                dr, lv = 1, L[i - 1] + pa
            elif O[i] >= H[i - 1] + gs and O[i] > H[i - 1]:
                dr, lv = -1, H[i - 1] - pa
            else:
                continue
            bars = M.day(D[i][0])
            if bars is None:
                continue
            r = day(bars, [(dr > 0, lv)], lambda k_, px: (dr < 0, px - dr * 0.25 * R), amb, D[i][0])
            if r:
                tr(dr, i, r[1], i, r[2] if r[2] is not None else C[i], "stop" if r[2] is not None else "close")
    else:
        raise KeyError(id_)
    return out
