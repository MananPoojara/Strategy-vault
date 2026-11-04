#!/usr/bin/env python3
"""K39 independent references for the rules on N-minute bars (decision #202), written from the source texts:
  Collins, Beating the Financial Futures Market, printed TradeStation code pp.226-231 (Tables 31.2, 31.3, 31.4, 34.1, 35.1,
    35.2, 39.1, Index versions) and the prose pp.138-157, 171-175 (CST clock, 8:30-15:15 index session, day trades)
  Active Trader Jan 2004 Trading System Lab, 60-minute breakout (futures p.70, stock column p.50)
  Murakami, Trading System Design: L/S Model code p.76, Bar Breakout code pp.23-24, 4/21 MA example p.37
Reads the vault's 1-minute files directly, one day at a time (m1idx byte ranges), and the daily file for the calendar.
Readings shared with the engine card (declared there): book CT times = minutes after the 8:30 open; exits at/after the cash
close = the day's last close; a signal on the day's last bar makes no trade; NIFTY 50 days from 2026-08-11 are not used.
Each backtest returns trades as (dir, entry_date, entry_px, exit_date, exit_px, reason, open)."""
import os
M1 = os.environ.get("M1_DIR", "/home/server/.cache/strategy-vault-webapp/data/intraday")


class Day:
    """One day's minutes from <T>.m1.csv via <T>.m1idx.csv (offset, bytes): [(minute, o, h, l, c)]."""
    def __init__(self, t):
        f = t.replace(" ", "_")
        self.t, self.path = t, f"{M1}/{f}.m1.csv"
        self.idx = {}
        with open(f"{M1}/{f}.m1idx.csv") as fh:
            next(fh)
            for line in fh:
                d, o, n = line.strip().split(",")
                self.idx[d] = (int(o), int(n))

    def get(self, d):
        if self.t == "NIFTY 50" and d >= "2026-08-11":
            return None
        if d not in self.idx:
            return None
        o, n = self.idx[d]
        with open(self.path, "rb") as fh:
            fh.seek(o)
            text = fh.read(n).decode()
        out = []
        for line in text.splitlines():
            v = line.split(",")
            if v[0] != d:
                return None
            out.append((int(v[1][:2]) * 60 + int(v[1][3:5]), float(v[2]), float(v[3]), float(v[4]), float(v[5])))
        return out


def open_minute(t, d):  # regular-session open (start-stamped clock)
    if not t.startswith("NIFTY"):
        return 9 * 60 + 30
    return 9 * 60 + 55 if d < "2010-01-04" else 9 * 60 if d < "2010-10-18" else 9 * 60 + 15


def close_minute(t):
    return 15 * 60 + 30 if t.startswith("NIFTY") else 16 * 60


def bars(mins, anchor, n, close):
    """N-minute bars: dict per bucket {s, e, o, h, l, c, mins} in time order."""
    out = []
    for z in mins:
        s = anchor + (z[0] - anchor) // n * n
        if not out or out[-1]["s"] != s:
            out.append({"s": s, "e": min(s + n, close), "o": z[1], "h": z[2], "l": z[3], "c": z[4], "mins": [z]})
        else:
            b = out[-1]
            b["h"] = max(b["h"], z[2]); b["l"] = min(b["l"], z[3]); b["c"] = z[4]; b["mins"].append(z)
    return out


class Acct:
    def __init__(self):
        self.pos, self.trades = None, []  # pos = (dir, date, px)

    def close(self, d, px, why, open_=False):
        if self.pos:
            self.trades.append((self.pos[0], self.pos[1], self.pos[2], d, px, why, open_))
            self.pos = None

    def go(self, dr, d, px, why):  # TradeStation: same side ignored, opposite reverses
        if self.pos and self.pos[0] == dr:
            return
        self.close(d, px, why)
        self.pos = (dr, d, px)


def ct(hhmm):  # Collins CST clock -> minutes after the 8:30 open
    return hhmm // 100 * 60 + hhmm % 100 - 510


def collins(id_, dates, M, a, b):
    t = M.t
    n = {"acbff_open_rel": 30, "acbff_2bar_open": 60, "acbff_final_third": 135}.get(id_, 5)
    ac = Acct()
    prevc = None
    hh = ll = hhh = lll = 0.0
    xs = []                         # Table 35.1 x values (continuous)
    rs = {"p": [], "net": 0.0, "tot": 0.0}  # Table 39.1 RSI(9)
    for d in dates[a:b + 1]:
        mins = M.get(d)
        if not mins:
            continue
        op = open_minute(t, d)
        B = bars(mins, op, n, close_minute(t))
        dayopen = B[0]["o"]
        hi, lo = B[0]["h"], B[0]["l"]
        order = None
        for k, z in enumerate(B):
            if order == "exit":
                ac.close(d, z["o"], "time exit")
            elif order in (1, -1):
                ac.go(order, d, z["o"], "reverse")
            order = None
            hi, lo = max(hi, z["h"]), min(lo, z["l"])
            tm = z["e"] - op
            more = k < len(B) - 1
            go_long = go_short = False
            if id_ == "acbff_open_rel":       # x(9.3): at 9:30 above/below opend(0); y(15.15) sell this bar on close
                if tm == ct(930):
                    go_long, go_short = z["c"] > dayopen, z["c"] < dayopen
            elif id_ == "acbff_2bar_open":    # b(10.3) x(14.3)
                if ct(1030) <= tm < ct(1430):
                    if prevc is not None:
                        go_long = z["c"] > dayopen and prevc > dayopen and z["c"] > prevc
                        go_short = z["c"] < dayopen and prevc < dayopen and z["c"] < prevc
                if tm == ct(1430) and more:
                    order = "exit"
            elif id_ == "acbff_final_third":  # x(13): both 135-min bars vs the open
                if tm == ct(1300) and prevc is not None:
                    go_long = z["c"] > dayopen and prevc > dayopen
                    go_short = z["c"] < dayopen and prevc < dayopen
            elif id_ == "acbff_1230":
                if tm == ct(930):
                    hh, ll = z["h"], z["l"]
                if tm == ct(1130):
                    hhh, lll = z["h"], z["l"]
                if tm == ct(1230):
                    rng = hi - lo
                    go_long = dayopen - lo <= 0.25 * rng and hi > hhh and hhh > hh
                    go_short = hi - dayopen <= 0.25 * rng and lo < lll and lll < ll
            else:
                window = ct(915) <= tm <= ct(1445)
                if id_ == "acbff_9oc":
                    xs.append(1 if z["c"] > z["o"] else -1 if z["c"] < z["o"] else 0)
                    xs = xs[-9:]
                    if window and len(xs) == 9:
                        s = sum(xs)
                        go_long, go_short = s > 0, s < 0
                elif id_ == "acbff_midpoint":
                    if window:
                        mid = (hi + lo) / 2
                        go_long, go_short = z["c"] > mid, z["c"] < mid
                else:  # acbff_rsi_5min: Length 9, n 15
                    r = None
                    p = rs["p"]
                    if len(p) < 10:
                        p.append(z["c"])
                        if len(p) == 10:
                            rs["net"] = (p[9] - p[0]) / 9
                            rs["tot"] = sum(abs(p[j + 1] - p[j]) for j in range(9)) / 9
                            r = 50 * (rs["net"] / rs["tot"] + 1) if rs["tot"] != 0 else 50
                    else:
                        ch = z["c"] - p[-1]
                        p[-1] = z["c"]
                        rs["net"] += (ch - rs["net"]) / 9
                        rs["tot"] += (abs(ch) - rs["tot"]) / 9
                        r = 50 * (rs["net"] / rs["tot"] + 1) if rs["tot"] != 0 else 50
                    if window and r is not None:
                        go_long, go_short = r > 85, r < 15
            if more and go_long:
                order = 1
            if more and go_short:
                order = -1
            prevc = z["c"]
            if not more:
                ac.close(d, z["c"], "end of day")
    return ac.trades


def atm04_60min(dates, M, a, b, third=False):
    t = M.t
    ac = Acct()
    for d in dates[a:b + 1]:
        mins = M.get(d)
        if not mins:
            continue
        op = open_minute(t, d)
        B = bars(mins, op, 30, close_minute(t))
        if len(B) < 3 or B[1]["e"] != op + 60:
            continue
        top, bot = max(B[0]["h"], B[1]["h"]), min(B[0]["l"], B[1]["l"])
        done, want = False, None
        for k in range(2, len(B)):
            z = B[k]
            if want == "out":
                ac.close(d, z["o"], "opposite signal")
            elif want is not None:
                ac.go(want, d, z["o"], ""); done = True
            want = None
            side = ac.pos[0] if ac.pos else 0
            if side == 1 and z["c"] < bot or side == -1 and z["c"] > top:
                want = "out"
            elif side == 0 and not done and (not third or k == 2):
                want = 1 if z["c"] > top else -1 if z["c"] < bot else None
            if k == len(B) - 1:
                ac.close(d, z["c"], "end of day")
    return ac.trades


def touch_first(mins, levels):
    """levels = [(is_buy, px)]: first minute touching one; at the level or the minute's open when it opened through it.
    Two in one minute: the one the minute opened through, else the one nearer the open (first listed on a tie)."""
    for m, o, h, l, c in mins:
        hit = [k for k, (up, px) in enumerate(levels) if (h >= px if up else l <= px)]
        if not hit:
            continue
        if len(hit) > 1:
            thru = [k for k in hit if (o >= levels[k][1] if levels[k][0] else o <= levels[k][1])]
            if len(thru) == 1:
                return thru[0], o
            k = min(hit, key=lambda k: abs(levels[k][1] - o))
        else:
            k = hit[0]
        up, px = levels[k]
        return k, (max(o, px) if up else min(o, px))
    return None


def jmls(id_, dates, M, a, b):
    t = M.t
    ac = Acct()
    last = None
    if id_ == "jmls_ls_model":
        yclose = None
        for d in dates[a:b + 1]:
            mins = M.get(d)
            if not mins:
                continue
            B = bars(mins, 0, 60, close_minute(t))  # natural hours
            for z in B:
                if 1300 < z["e"] // 60 * 100 + z["e"] % 60 < 1605 and yclose is not None:
                    if z["c"] > yclose:
                        ac.go(1, d, z["c"], "reverse")
                    if z["c"] < yclose:
                        ac.go(-1, d, z["c"], "reverse")
            yclose = B[-1]["c"]; last = (d, B[-1]["c"])
    elif id_ == "jmls_bar_breakout":
        hist = []
        for d in dates[a:b + 1]:
            mins = M.get(d)
            if not mins:
                continue
            op, cl = open_minute(t, d), close_minute(t)
            B = bars(mins, op, 15, cl)
            orders = None
            for z in B:
                if orders:
                    hi_, lo_ = orders
                    lv = []
                    side = ac.pos[0] if ac.pos else 0
                    if side != 1:
                        lv.append((True, hi_))
                    if side != -1:
                        lv.append((False, lo_))
                    f = touch_first(z["mins"], lv)
                    if f:
                        ac.go(1 if lv[f[0]][0] else -1, d, f[1], "reverse")
                orders = None
                hist.append((z["h"], z["l"]))
                hist = hist[-25:]
                if len(hist) == 25 and op < z["e"] < cl:
                    orders = (max(x[0] for x in hist), min(x[1] for x in hist))
            last = (d, B[-1]["c"])
    else:  # jmls_ma_cross_30m
        cs = []
        for d in dates[a:b + 1]:
            mins = M.get(d)
            if not mins:
                continue
            B = bars(mins, open_minute(t, d), 30, close_minute(t))
            for z in B:
                cs.append(z["c"]); cs = cs[-21:]
                if len(cs) < 21:
                    continue
                f, s = sum(cs[-4:]) / 4, sum(cs) / 21
                if f > s:
                    ac.go(1, d, z["c"], "reverse")
                elif f < s:
                    ac.go(-1, d, z["c"], "reverse")
            last = (d, B[-1]["c"])
    if last:
        ac.close(last[0], last[1], "open", True)
    return ac.trades


def backtest(id_, dates, M, a, b, prm=None):
    prm = prm or {}
    if id_.startswith("acbff_"):
        return collins(id_, dates, M, a, b)
    if id_ == "atm04_60min":
        return atm04_60min(dates, M, a, b, prm.get("third", 0) >= 0.5)
    return jmls(id_, dates, M, a, b)
