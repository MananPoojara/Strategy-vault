#!/usr/bin/env python3
"""Intraday wave 3 (K24, 3 Nov 2026): independent reference for tdperl_tdline_short, written from Jason Perl, DeMark
Indicators (Bloomberg 2008) Ch 4 pp.91-101 (read again from the PDF for this lane), not from the Rust code. Works on real
prices (no mirroring). Reads the daily file and the cleaned 1-minute CSV through indep_ref's loaders.

Rule (short side, TD Demand Line, level L = 1):
  TD Demand Point: a low with L higher lows on either side (p.91), known once the L-th bar to its right has closed.
  TD Demand Line: the most recent demand point and the next-most-recent LOWER one, extended bar by bar (p.92-93).
  Breakout bar X: the first bar whose low is below the line (each line is broken once). Qualified (p.94-96) if
    Q1 close[X-1] > close[X-2], or Q2 open[X] < line(X), or Q3 close[X-1] - (max(high[X-1], close[X-2]) - close[X-1]) > line(X).
  Entry: "sell on the break of the TD Demand Line": a sell stop at line(X).
  Objective (p.96): line(X) - (highest true high above the line - the line under it); the window, from the older line
    point to the bar before X, is the card's stated reading [book silent].
  "The objective remains intact unless the market has a qualified breakout to the upside": a qualified TD Supply Line
    break (mirror rules, p.98-100) on a later day covers the short at that supply line (buy stop).
  Failure exits (p.97) on the bar after X: open above line(X) -> cover at the open; open above the (extended) line and
    close above line(X) -> cover at the close; low not below the low of X -> cover at the close.
Minute conventions (stated on the card): stops and limits fill at the level or at the minute's open when the minute opens
through it. Entry day: the objective only counts from the fill minute on (it lies below the fill, so a minute that
reaches it after filling reached it after the fill). Later days: open events at the daily open first (open failure,
objective gap, supply break gap), then the first of objective / supply-line stop on the minutes; one minute reaching
both without opening through exactly one -> the level nearer that minute's open first (listed as ambiguous). A later day
without minutes: decided by the daily bar if it reaches one exit level; both -> the trade is dropped (listed).
"""
import os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))


def points(D, n, lv, low_side):
    """Confirmed TD Points p (usable from bar p+lv+1)."""
    out = []
    for p in range(lv, n - lv):
        x = D[p][3] if low_side else D[p][2]
        nb = [D[p - k][3 if low_side else 2] for k in range(1, lv + 1)] + [D[p + k][3 if low_side else 2] for k in range(1, lv + 1)]
        if all((x < y) if low_side else (x > y) for y in nb):
            out.append(p)
    return out


def breaks(D, n, lv, demand):
    """Qualified first breaks of TD Demand (demand=True, downside) or Supply lines. -> {bar: (line value, p1, p2)}"""
    pts = points(D, n, lv, demand)
    k = 3 if demand else 2
    res, done = {}, set()
    for i in range(2, n):
        usable = [p for p in pts if p + lv + 1 <= i]
        if not usable:
            continue
        p1 = usable[-1]
        older = [p for p in usable[:-1] if (D[p][k] < D[p1][k] if demand else D[p][k] > D[p1][k])]
        if not older:
            continue
        p2 = older[-1]
        if (p2, p1) in done:
            continue
        line = lambda j, p1=p1, p2=p2: D[p1][k] + (D[p1][k] - D[p2][k]) / (p1 - p2) * (j - p1)  # bind this pair
        L = line(i)
        o, h, l, c = D[i][1:5]
        if not ((l < L) if demand else (h > L)):
            continue
        done.add((p2, p1))
        c1, c2, h1, l1 = D[i - 1][4], D[i - 2][4], D[i - 1][2], D[i - 1][3]
        if demand:
            q = c1 > c2 or o < L or c1 - (max(h1, c2) - c1) > L
        else:
            q = c1 < c2 or o > L or c1 + (c1 - min(l1, c2)) < L
        if q:
            res[i] = (L, p1, p2, line)
    return res


def true_high(D, j):
    return max(D[j][2], D[j - 1][4]) if j > 0 else D[j][2]


def backtest(D, M, a, b, lv=1, amb=None, dropped=None, reject=lambda d: False):
    amb = [] if amb is None else amb
    dropped = [] if dropped is None else dropped
    n = b + 1
    dem, sup = breaks(D, n, lv, True), breaks(D, n, lv, False)
    out, pos = [], None

    def day(i):
        return None if reject(D[i][0]) else M.day(D[i][0])

    def T(epx, ei, xi, xpx, why, opn=False):
        out.append({"dir": -1, "entry": D[ei][0], "entry_px": epx, "exit": D[xi][0], "exit_px": xpx, "reason": why, "open": opn})

    for i in range(n):
        o, h, l, c = D[i][1:5]
        if pos:
            ei, epx, L0, tgt, line = pos
            after = i == ei + 1
            adv = sup.get(i)
            pos = None
            if after and o > L0:
                T(epx, ei, i, o, "open failure"); continue
            if o <= tgt:
                T(epx, ei, i, o, "objective gap"); continue
            if adv and o >= adv[0]:
                T(epx, ei, i, o, "supply break at the open"); continue
            bars = day(i)
            hit = None
            if bars is not None:
                for z in bars:
                    t_hit, a_hit = z[3] <= tgt, adv is not None and z[2] >= adv[0]
                    if t_hit and a_hit:
                        # opened through exactly one? (both cannot be: tgt < open < supply line here)
                        if z[1] <= tgt:
                            hit = (z[1], "objective")
                        elif z[1] >= adv[0]:
                            hit = (z[1], "supply break")
                        else:
                            amb.append((D[i][0], "objective and supply-line stop in one minute", z[0]))
                            hit = (tgt, "objective") if abs(tgt - z[1]) <= abs(adv[0] - z[1]) else (adv[0], "supply break")
                        break
                    if t_hit:
                        hit = (min(z[1], tgt), "objective"); break
                    if a_hit:
                        hit = (max(z[1], adv[0]), "supply break"); break
            else:
                t_hit, a_hit = l <= tgt, adv is not None and h >= adv[0]
                if t_hit and a_hit:
                    dropped.append((D[ei][0], D[i][0])); continue
                if a_hit:
                    hit = (max(o, adv[0]), "supply break")
                elif t_hit:
                    hit = (tgt, "objective")
            if hit:
                T(epx, ei, i, hit[0], hit[1]); continue
            if after and o > line(i) and c > L0:
                T(epx, ei, i, c, "close failure"); continue
            if after and l >= D[ei][3]:
                T(epx, ei, i, c, "low failure"); continue
            pos = (ei, epx, L0, tgt, line)
            continue
        brk = dem.get(i)
        if not brk or not (a <= i < b):
            continue
        L0, p1, p2, line = brk
        j = max(range(p2, i), key=lambda q: (true_high(D, q), q))  # highest true high; latest on ties
        tgt = L0 - (true_high(D, j) - line(j))
        bars = day(i)
        if bars is None:
            continue
        kf = next((k for k, z in enumerate(bars) if z[3] <= L0), None)
        if kf is None:
            continue
        epx = min(bars[kf][1], L0)
        z = next((z for z in bars[kf:] if z[3] <= tgt), None)
        if z is not None:
            T(epx, i, i, min(z[1], tgt), "objective (entry day)"); continue
        pos = (i, epx, L0, tgt, line)
    if pos:
        T(pos[1], pos[0], b, D[b][4], "open", True)
    return out
