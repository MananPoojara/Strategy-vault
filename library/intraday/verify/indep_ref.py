#!/usr/bin/env python3
"""Independent reference for the intraday phase 1 rules, written from the book text (not from the Rust code).

Reads the daily file (idx_data.csv / us_data.csv) and the CLEANED 1-minute CSV (<T>.m1.csv from m1_normalise.py)
directly; it builds its own day offsets by streaming the CSV once and seeks one day at a time.

Rules (book quotes in the module rules text):
  ss_momentum_pinball  Street Smarts Ch 7 pp.51-52: LBR/RSI = 3-period RSI of the 1-day net change. Day one < 30 (sell > 70).
      Day two: buy stop 1 tick above the first-hour high; resting sell stop at the first-hour low. Profitable on the close:
      carry; day three: exit just above the day-two high (limit 1 tick above), else by the close. Otherwise out on the close.
  ss_80_20  Street Smarts Ch 6 p.46: yesterday opened in the top 20% of its range and closed in the bottom 20%; today trades
      >= 10 ticks below yesterday's low; buy stop at yesterday's low; protective stop at today's low; day trade.
  ats_15min_adx_breakout / ptjdt_adx15_breakout  ATS Ch 20 / PTJ Day Trading: daily ADX(14) > 30 and +DI > -DI (sells -DI > +DI)
      at yesterday's close; buy stop 1 tick above the first 15-minute high, stop 1 tick under its low; re-enter after a
      stop-out at the same stop, no more than 2 entries a day; out at the end of the day (ATS: daily close, PTJ: last minute).
Fill rules (store README): a stop fills at its level, or at the minute's open when the minute opens through it.
Declared conventions for one minute reaching two levels: Pinball/80-20 fill minute also reaching the stop -> stopped at the
stop; Pinball day 3 stop and limit in one minute -> stop; 80-20 the entry stop is live only from the minute after the
break; ADX15 the extreme nearer the minute's open came first. Every such minute is returned in `ambig`.
"""
import csv, math, os

M1 = os.environ.get("M1_DIR", "/home/server/strategy-vault-data-staging/intraday-phase1-20261024/m1")


def num(s):
    return float(s.replace(",", "")) if s.strip() else math.nan


def load_daily(path, ticker):
    out = []
    with open(path, newline="") as f:
        for r in csv.DictReader(f):
            if r["Ticker"] != ticker:
                continue
            d, m, y = r["Date"].split("-")
            out.append((f"{y}-{m}-{d}", num(r["Open"]), num(r["High"]), num(r["Low"]), num(r["Close"])))
    return sorted(out)


class Minutes:
    def __init__(self, ticker):
        self.path = f"{M1}/{ticker.replace(' ', '_')}.m1.csv"
        self.off = {}
        with open(self.path, "rb") as f:
            pos = len(f.readline())
            for line in f:
                d = line[:10].decode()
                if d not in self.off:
                    self.off[d] = [pos, 0]
                self.off[d][1] += len(line)
                pos += len(line)
        self.first, self.last = min(self.off), max(self.off)

    def day(self, d):
        if d not in self.off:
            return None
        o, n = self.off[d]
        with open(self.path, "rb") as f:
            f.seek(o)
            rows = f.read(n).decode().splitlines()
        bars = []
        for r in rows:
            v = r.split(",")
            assert v[0] == d
            hh, mm = v[1].split(":")
            o_, h, l, c = map(float, v[2:6])
            assert all(math.isfinite(x) for x in (o_, h, l, c)) and l <= min(o_, c) and h >= max(o_, c), r  # no NULL/odd minute fills
            bars.append((int(hh) * 60 + int(mm), o_, h, l, c))
        return bars


def open_clock(ticker, d):
    if not ticker.startswith("NIFTY"):
        return 9 * 60 + 30
    return 9 * 60 + 55 if d < "2010-01-04" else 9 * 60 if d < "2010-10-18" else 9 * 60 + 15


def wilder_rsi(x, n):
    """Wilder RSI of a series x (NaN until n changes are available; NaN inputs are skipped at the start)."""
    out = [math.nan] * len(x)
    ag = al = None
    prev = None
    ups, dns = [], []
    for i, v in enumerate(x):
        if not math.isfinite(v):
            continue
        if prev is None:
            prev = v
            continue
        ch = v - prev
        prev = v
        u, dn = max(ch, 0.0), max(-ch, 0.0)
        if ag is None:
            ups.append(u); dns.append(dn)
            if len(ups) == n:
                ag, al = sum(ups) / n, sum(dns) / n
            else:
                continue
        else:
            ag, al = (ag * (n - 1) + u) / n, (al * (n - 1) + dn) / n
        out[i] = 100.0 if al == 0 else 100.0 - 100.0 / (1.0 + ag / al)
    return out


def wilder_dmi(D, n):
    """Wilder +DI, -DI, ADX on daily bars."""
    L = len(D)
    pdm, mdm, tr = [math.nan] * L, [math.nan] * L, [math.nan] * L
    for i in range(1, L):
        up, dn = D[i][2] - D[i - 1][2], D[i - 1][3] - D[i][3]
        pdm[i] = up if up > dn and up > 0 else 0.0
        mdm[i] = dn if dn > up and dn > 0 else 0.0
        tr[i] = max(D[i][2], D[i - 1][4]) - min(D[i][3], D[i - 1][4])

    def ws(v):
        o, s, k = [math.nan] * L, None, []
        for i, x in enumerate(v):
            if not math.isfinite(x):
                continue
            if s is None:
                k.append(x)
                if len(k) == n:
                    s = sum(k) / n
                    o[i] = s
            else:
                s = (s * (n - 1) + x) / n
                o[i] = s
        return o
    sp, sm, st = ws(pdm), ws(mdm), ws(tr)
    pdi, mdi, dx = [math.nan] * L, [math.nan] * L, [math.nan] * L
    for i in range(L):
        if st[i] > 0:
            pdi[i], mdi[i] = 100 * sp[i] / st[i], 100 * sm[i] / st[i]
            dx[i] = 100 * abs(pdi[i] - mdi[i]) / (pdi[i] + mdi[i]) if pdi[i] + mdi[i] > 0 else 0.0
    return pdi, mdi, ws(dx)


def touch(b, up, lv):
    return b[2] >= lv if up else b[3] <= lv


def fill(b, up, lv):
    return max(b[1], lv) if up else min(b[1], lv)


def T(dr, d, epx, x, xpx, why, opn=False):
    return {"dir": dr, "entry": d, "entry_px": epx, "exit": x, "exit_px": xpx, "reason": why, "open": opn}


def pinball(D, M, a, b, ticker, ambig):
    c = [r[4] for r in D]
    lb = wilder_rsi([math.nan] + [c[i] - c[i - 1] for i in range(1, len(c))], 3)
    tick = lambda i: c[i] * 0.0001  # 1 tick = 0.01% of price on an index/ETF (module convention), from a known close
    out, j = [], max(a, 1)
    while j <= b:
        d = j - 1
        dr = 1 if lb[d] < 30 else -1 if lb[d] > 70 else 0
        bars = M.day(D[j][0]) if dr else None
        if not bars:
            j += 1; continue
        oc = open_clock(ticker, D[j][0])
        fh_bars = [z for z in bars if z[0] < oc + 60]
        rest = [z for z in bars if z[0] >= oc + 60]
        if not fh_bars or not rest:
            j += 1; continue
        fh, fl = max(z[2] for z in fh_bars), min(z[3] for z in fh_bars)
        up = dr > 0
        lv_in = fh + tick(d) if up else fl - tick(d)
        lv_st = fl if up else fh
        k = next((k for k, z in enumerate(rest) if touch(z, up, lv_in)), None)
        if k is None:
            j += 1; continue
        px = fill(rest[k], up, lv_in)
        if touch(rest[k], not up, lv_st):
            ambig.append((D[j][0], "pinball fill+stop", rest[k][0]))
            out.append(T(dr, D[j][0], px, D[j][0], lv_st, "stop")); j += 1; continue
        s = next((z for z in rest[k + 1:] if touch(z, not up, lv_st)), None)
        if s:
            out.append(T(dr, D[j][0], px, D[j][0], fill(s, not up, lv_st), "stop")); j += 1; continue
        if j == b:
            out.append(T(dr, D[j][0], px, D[j][0], c[j], "open", True)); break
        if dr * (c[j] - px) <= 0:
            out.append(T(dr, D[j][0], px, D[j][0], c[j], "close")); j += 1; continue
        k3 = j + 1
        b3 = M.day(D[k3][0])
        if not b3:
            j += 1; continue  # day three without minutes: not resolvable, not taken
        tgt = D[j][2] + tick(j) if up else D[j][3] - tick(j)
        res = None
        for z in b3:
            hs, ht = touch(z, not up, lv_st), touch(z, up, tgt)
            if hs and ht:
                if (z[1] <= lv_st) if up else (z[1] >= lv_st):
                    res = (z[1], "stop")
                elif (z[1] >= tgt) if up else (z[1] <= tgt):
                    res = (z[1], "target")
                else:
                    ambig.append((D[k3][0], "pinball day3 stop+limit", z[0]))
                    res = (lv_st, "stop")
                break
            if hs:
                res = (fill(z, not up, lv_st), "stop"); break
            if ht:
                res = (fill(z, up, tgt), "target"); break
        res = res or (c[k3], "close")
        out.append(T(dr, D[j][0], px, D[k3][0], res[0], res[1]))
        j = k3 + 1
    return out


def e8020(D, M, a, b, ticker, ambig):
    out = []
    for j in range(max(a, 1), b + 1):
        _, o, h, l, c = D[j - 1]
        r = h - l
        if r <= 0:
            continue
        ol, cl = (o - l) / r, (c - l) / r
        dr = 1 if ol >= 0.8 and cl <= 0.2 else -1 if ol <= 0.2 and cl >= 0.8 else 0
        bars = M.day(D[j][0]) if dr else None
        if not bars:
            continue
        up = dr > 0
        tk = c * 0.0001
        lv = l if up else h
        brk = lv - 10 * tk if up else lv + 10 * tk
        kb = next((k for k, z in enumerate(bars) if (z[3] <= brk if up else z[2] >= brk)), None)
        if kb is None:
            continue
        # a minute that OPENED beyond the break level made the break at its open, so its later trade back to yesterday's
        # extreme is a valid fill; a minute that broke after its open and also traded back cannot show the order
        opened_beyond = bars[kb][1] <= brk if up else bars[kb][1] >= brk
        if touch(bars[kb], up, lv) and not opened_beyond:
            ambig.append((D[j][0], "80-20 break minute also back at the level (not filled there)", bars[kb][0]))
        k_from = kb if opened_beyond else kb + 1
        kf = next((k for k in range(k_from, len(bars)) if touch(bars[k], up, lv)), None)
        if kf is None:
            continue
        px = fill(bars[kf], up, lv)
        # today's extreme before the fill: earlier minutes and the fill minute's own open
        st = min([z[3] for z in bars[:kf]] + [bars[kf][1]]) if up else max([z[2] for z in bars[:kf]] + [bars[kf][1]])
        if touch(bars[kf], not up, st):
            ambig.append((D[j][0], "80-20 fill+stop", bars[kf][0]))
            out.append(T(dr, D[j][0], px, D[j][0], st, "stop")); continue
        s = next((z for z in bars[kf + 1:] if touch(z, not up, st)), None)
        out.append(T(dr, D[j][0], px, D[j][0], fill(s, not up, st) if s else D[j][4], "stop" if s else "close"))
    return out


def adx15(D, M, a, b, ticker, ambig, last_close):
    pdi, mdi, adx = wilder_dmi(D, 14)
    tk = 0.05 if ticker.startswith("NIFTY") else 0.01
    out = []
    for i in range(max(a, 1), b + 1):
        k = i - 1
        if not adx[k] > 30:
            continue
        up = pdi[k] > mdi[k]
        if not (up or mdi[k] > pdi[k]):
            continue
        bars = M.day(D[i][0])
        if not bars:
            continue
        oc = open_clock(ticker, D[i][0])
        w = [z for z in bars if z[0] < oc + 15]
        rest = [z for z in bars if z[0] >= oc + 15]
        if not w or not rest:
            continue
        h15, l15 = max(z[2] for z in w), min(z[3] for z in w)
        lv_in, lv_st = (h15 + tk, l15 - tk) if up else (l15 - tk, h15 + tk)
        dr, entries, held = (1 if up else -1), 0, None
        close = bars[-1][4] if last_close else D[i][4]
        for z in rest:
            if held is not None:
                if touch(z, not up, lv_st):
                    out.append(T(dr, D[i][0], held, D[i][0], fill(z, not up, lv_st), "stop")); held = None
                continue
            if entries >= 2 or not touch(z, up, lv_in):
                continue
            entries += 1
            px = fill(z, up, lv_in)
            if touch(z, not up, lv_st):
                ambig.append((D[i][0], "adx15 fill+stop", z[0]))
                low_first = (z[1] - z[3] < z[2] - z[1]) if up else (z[2] - z[1] < z[1] - z[3])
                if low_first:
                    held = px
                else:
                    out.append(T(dr, D[i][0], px, D[i][0], lv_st, "stop"))
            else:
                held = px
        if held is not None:
            out.append(T(dr, D[i][0], held, D[i][0], close, "close"))
    return out


def backtest(id_, D, M, a, b, ticker, ambig):
    if id_ == "ss_momentum_pinball":
        return pinball(D, M, a, b, ticker, ambig)
    if id_ == "ss_80_20":
        return e8020(D, M, a, b, ticker, ambig)
    if id_ == "ats_15min_adx_breakout":
        return adx15(D, M, a, b, ticker, ambig, False)
    if id_ == "ptjdt_adx15_breakout":
        return adx15(D, M, a, b, ticker, ambig, True)
    raise KeyError(id_)
