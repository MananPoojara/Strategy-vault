#!/usr/bin/env python3
"""Independent reference for the options phase 2 rules, written from the book texts (quoted below), reading the derived
option chains (staging) and the vault's daily files directly. It does not import or translate the Rust code.

Book rules (page refs as on the cards):
  OISS (Connors 2004): "The index (or stock) is above its 200-day MA" (puts; calls: below). "The 3-day RSI of the
    at-the-money put, that is more than one week out from expiration, closes above 80 (use the next month's option if the
    current option is in expiration week)." "Sell the put at the close." "put on a stop-loss at double your entry price."
    "Exit the put when the 3-day RSI of the put closes below 25." Trade 12: on expiration day with no exit, buy back and
    sell the next month's ATM option. Plus Three: "Take an additional position if the 3-day RSI ... closes above 80,
    three days in a row"; "Exit all the puts when the 3-day RSI of the put closes below 25."
  CVR options (Options Trading & Volatility Trading): CVR I buy = VIX 15-day high and VIX close below its open; "By
    selling puts!" "as near the money as possible (within two to three strike prices)"; 3-day hold; "closing out the
    naked puts if prices reach your strike price".
  Weekly RSI Program (Chairman's Club 2008 S2 s32-33): weekly bars, weekly RSI(2) closes above 85: sell the ATM call;
    "Exit one week later or when the daily 2-period RSI drops under ... 60 (the latter is preferred)".
  Covered calls (Appel p.162): buy SPY, sell one call "with the nearest expiration date whose strike price is closest to
    (but not below) the current price"; one expiration each month (third Friday).
  EOM puts (S&P Market Timing p.168-169): above the 200-day MA, the 25th or later, the market drops two days in a row:
    sell the closest month puts about 2% out of the money; exit on the RSI(2) > 65 signal the next morning.
Price rules and the conventions the books leave open are the ones stated on the cards (fills only on traded rows, US at
the 16:00 mid; marks; expiry at intrinsic value; one option per unit of the underlying; account sized at equity).
"""
import collections, csv, math

D = "/home/server/.cache/strategy-vault-webapp/data"
ST = "/home/server/strategy-vault-data-staging/options-phase2-20261024"


def num(s):
    return float(s.replace(",", ""))


def load_daily(ticker):
    path = f"{D}/{'us_data' if ticker in ('SPY', 'QQQ', 'IWM') else 'idx_data'}.csv"
    out = {}
    for r in csv.DictReader(open(path, newline="")):
        if r["Ticker"] == ticker:
            d, m, y = r["Date"].split("-")
            out.setdefault(f"{y}-{m}-{d}", (num(r["Open"]), num(r["High"]), num(r["Low"]), num(r["Close"])))
    ds = sorted(out)
    return ds, [out[d] for d in ds]


def load_vix():
    v = {}
    for r in csv.DictReader(open(f"{D}/us_vix/VIX.csv")):
        v[r["Date"]] = tuple(float(r[k]) for k in ("Open", "High", "Low", "Close"))
    return v


class Chain:
    """Rows of one day: dict (expiry int, 'P'/'C', strike) -> dict. Days are read on demand from the byte index."""

    def __init__(self, ticker):
        mkt = "us" if ticker in ("SPY", "QQQ", "IWM") else "india"
        self.us = mkt == "us"
        self.path = f"{ST}/{mkt}/{ticker.replace(' ', '_')}.opt.csv"
        self.idx = {}
        for line in open(f"{ST}/{mkt}/{ticker.replace(' ', '_')}.optidx.csv").read().splitlines()[1:]:
            d, o, n = line.split(",")
            self.idx[d] = (int(o), int(n))
        self.cache = collections.OrderedDict()
        self.fh = open(self.path, "rb")

    def day(self, d):
        if d in self.cache:
            self.cache.move_to_end(d)
            return self.cache[d]
        rows = {}
        if d in self.idx:
            o, n = self.idx[d]
            self.fh.seek(o)
            for line in self.fh.read(n).decode().splitlines():
                f = line.split(",")
                k = (int(f[1]), f[2], float(f[3]))
                if self.us:
                    rows[k] = dict(bid=float(f[4]), ask=float(f[5]), vol=float(f[7]))
                else:
                    rows[k] = dict(open=float(f[4]), high=float(f[5]), close=float(f[7]), settle=float(f[8]), vol=float(f[9]))
        self.cache[d] = rows
        if len(self.cache) > 400:
            self.cache.popitem(last=False)
        return rows

    # ---- prices of one row (None = no such price)
    def close_fill(self, r):
        if r is None or r["vol"] <= 0:
            return None
        if self.us:
            return (r["bid"] + r["ask"]) / 2 if 0 < r["bid"] <= r["ask"] else None
        return r["close"] if r["close"] > 0 else None

    def open_fill(self, r):
        return r["open"] if (r is not None and not self.us and r["vol"] > 0 and r["open"] > 0) else None

    def high(self, r):
        return r["high"] if (r is not None and not self.us and r["vol"] > 0 and r["high"] > 0) else None

    def value(self, r):
        """(price, kind) for a daily valuation, or None."""
        p = self.close_fill(r)
        if p is not None:
            return p, "traded"
        if r is None:
            return None
        if self.us:
            return ((r["bid"] + r["ask"]) / 2, "quote") if 0 < r["bid"] <= r["ask"] else None
        return (r["settle"], "settle") if r["settle"] > 0 else None


def ymd(d):
    return int(d.replace("-", ""))


def civil(v):
    y, m, d = v // 10000, v // 100 % 100, v % 100
    import datetime as dt
    return dt.date(y, m, d).toordinal()


def monday(v):
    o = civil(v)
    import datetime as dt
    return o - dt.date.fromordinal(o).weekday()


class Mkt:
    def __init__(self, ticker):
        self.t = ticker
        self.dates, self.bars = load_daily(ticker)
        self.c = [b[3] for b in self.bars]
        self.l = [b[2] for b in self.bars]
        self.ch = Chain(ticker)
        self.ymd = [ymd(d) for d in self.dates]

    def last_day(self, e):
        """bar index of expiry e's last trading day (last bar dated <= e) when known"""
        import bisect
        k = bisect.bisect_right(self.ymd, e)
        if k > 0 and (k < len(self.ymd) or self.ymd[k - 1] == e):
            return k - 1
        return None

    def ltd(self, e):
        k = self.last_day(e)
        return e if k is None else self.ymd[k]

    def rows(self, i):
        return self.ch.day(self.dates[i])

    def expiries(self, i):
        return sorted({k[0] for k in self.rows(i)})

    def strikes(self, i, e, typ):
        return sorted(k[2] for k in self.rows(i) if k[0] == e and k[1] == typ)

    def q(self, i, con):
        return self.rows(i).get(con)


def nearest(ks, px):
    best = None
    for k in ks:  # ascending: a later strike must be strictly nearer to win (tie -> lower)
        if best is None or abs(k - px) < abs(best - px):
            best = k
    return best


def rsi(xs, n):
    if len(xs) < n + 1:
        return float("nan")
    up = [max(xs[i] - xs[i - 1], 0.0) for i in range(1, len(xs))]
    dn = [max(xs[i - 1] - xs[i], 0.0) for i in range(1, len(xs))]
    au, ad = sum(up[:n]) / n, sum(dn[:n]) / n
    for u, d in zip(up[n:], dn[n:]):
        au, ad = (au * (n - 1) + u) / n, (ad * (n - 1) + d) / n
    if ad == 0:
        return 50.0 if au == 0 else 100.0
    return 100 - 100 / (1 + au / ad)


def rsi_series(xs, n):
    out = [float("nan")] * len(xs)
    if len(xs) < n + 1:
        return out
    up = [0.0] + [max(xs[i] - xs[i - 1], 0.0) for i in range(1, len(xs))]
    dn = [0.0] + [max(xs[i - 1] - xs[i], 0.0) for i in range(1, len(xs))]
    au, ad = sum(up[1:n + 1]) / n, sum(dn[1:n + 1]) / n
    for i in range(n, len(xs)):
        if i > n:
            au, ad = (au * (n - 1) + up[i]) / n, (ad * (n - 1) + dn[i]) / n
        out[i] = (50.0 if au == 0 else 100.0) if ad == 0 else 100 - 100 / (1 + au / ad)
    return out


def sma(xs, n):
    return [sum(xs[i + 1 - n:i + 1]) / n if i + 1 >= n else float("nan") for i in range(len(xs))]


# ---------------------------------------------------------------- one option leg held until an exit
def hold(m, con, sign, i, px0, to, rule):
    """sign -1 sold / +1 bought. rule(k) -> None | ('close'|'open', why). Returns (exit bar, pnl points list, why, open)."""
    pnl, last, pending = [0.0], px0, None
    end = m.last_day(con[0])
    for k in range(i + 1, to + 1):
        r = m.q(k, con)
        v = m.ch.value(r)
        if v:
            last = v[0]
        cf = m.ch.close_fill(r)
        out = None
        if pending is not None:
            if cf is not None:
                out = (cf, pending + " (next traded close)")
        else:
            x = rule(k)
            if x:
                how, why = x
                if how == "close":
                    if cf is not None:
                        out = (cf, why)
                    else:
                        pending = why
                else:
                    of = m.ch.open_fill(r)
                    if of is not None:
                        out = (of, why)
                    elif cf is not None:
                        out = (cf, why + " (close)")
                    else:
                        pending = why
        if out is None and end == k:
            s = m.c[k]
            out = (max(con[2] - s, 0.0) if con[1] == "P" else max(s - con[2], 0.0), "settlement")
        if out:
            pnl.append(sign * (out[0] - px0))
            return k, pnl, out[1], False
        pnl.append(sign * (last - px0))
        if k == to:
            return k, pnl, "open", True
    return i, pnl, "open", True


# ---------------------------------------------------------------- strategies -> positions (entry, exit, s0, pnl list, open)
def oiss(m, a, b, put, plus3):
    typ = "P" if put else "C"
    ma = sma(m.c, 200)
    trend = (lambda i: m.c[i] > ma[i]) if put else (lambda i: m.c[i] < ma[i])

    def scan(i):
        ex = m.expiries(i)
        if not ex:
            return None
        e = ex[0]
        if monday(m.ltd(e)) == monday(m.ymd[i]):
            if len(ex) < 2:
                return None
            e = ex[1]
        k = nearest(m.strikes(i, e, typ), m.c[i])
        return None if k is None else (e, typ, k)

    def closes(con, lo, hi):
        return [p for p in (m.ch.close_fill(m.q(k, con)) for k in range(max(lo, 0), hi + 1)) if p is not None]

    out, live = [], []
    for i in range(a, b + 1):
        keep = []
        for p in live:
            r = m.q(i, p["con"])
            v = m.ch.value(r)
            if v:
                p["mark"] = v[0]
            cf = m.ch.close_fill(r)
            val = lambda price: sum(e - (price + p["adj"]) for e in p["sales"])
            h = m.ch.high(r)
            if h is not None and h >= p["stop"]:
                o = m.ch.open_fill(r)
                fill = o if (o is not None and o >= p["stop"]) else p["stop"]
                p["pnl"].append(val(fill)); out.append((p["entry"], i, p["s0"], p["pnl"], False)); continue
            if cf is not None and rsi(closes(p["con"], p["origin"] - 30, i), 3) < 25:
                p["pnl"].append(val(cf)); out.append((p["entry"], i, p["s0"], p["pnl"], False)); continue
            if m.last_day(p["con"][0]) == i:
                buy = cf if cf is not None else (max(p["con"][2] - m.c[i], 0) if put else max(m.c[i] - p["con"][2], 0))
                s = scan(i)
                new = m.ch.close_fill(m.q(i, s)) if (s and s[0] > p["con"][0]) else None
                if new is None:
                    p["pnl"].append(val(buy)); out.append((p["entry"], i, p["s0"], p["pnl"], False)); continue
                p["adj"] += buy - new
                p.update(origin=i, con=s, stop=2 * new, streak=0, mark=new)
                p["pnl"].append(val(new)); keep.append(p); continue
            if plus3 and cf is not None:
                r3 = rsi(closes(p["con"], p["origin"] - 30, i), 3)
                p["streak"] = p["streak"] + 1 if r3 > 80 else 0
                if not p["added"] and p["streak"] >= 3 and trend(i):
                    p["added"] = True
                    p["sales"].append(cf + p["adj"])
            p["pnl"].append(val(p["mark"])); keep.append(p)
        live = keep
        s = scan(i)
        if s is None:
            continue
        c = m.ch.close_fill(m.q(i, s))
        if c is None or not trend(i) or any(p["con"] == s for p in live):
            continue
        if civil(m.ltd(s[0])) - civil(m.ymd[i]) < 7:
            continue
        if not rsi(closes(s, i - 30, i), 3) > 80:
            continue
        live.append(dict(origin=i, con=s, entry=i, s0=m.c[i], sales=[c], adj=0.0, stop=2 * c, streak=1, added=False, mark=c, pnl=[0.0]))
    for p in live:
        p["pnl"].append(sum(e - (p["mark"] + p["adj"]) for e in p["sales"]))
        out.append((p["entry"], b, p["s0"], p["pnl"], True))
    return out


def cvr_puts(m, a, b, vix):
    v = [vix.get(d) for d in m.dates]
    n, out, i = 15, [], a
    def buy_sig(i):
        if i + 1 < n or v[i] is None:
            return False
        hh = max(x[1] for x in v[i + 1 - n:i + 1] if x is not None)
        return v[i][1] >= hh and v[i][3] < v[i][0]
    while i <= b:
        if not buy_sig(i):
            i += 1; continue
        e = next((e for e in m.expiries(i) if civil(m.ltd(e)) - civil(m.ymd[i]) > 7), None)
        below = [k for k in m.strikes(i, e, "P") if k < m.c[i]] if e else []
        if len(below) < 3:
            i += 1; continue
        con = (e, "P", below[-3])
        px0 = m.ch.close_fill(m.q(i, con))
        if px0 is None:
            i += 1; continue
        st = i
        j, pnl, why, op = hold(m, con, -1, i, px0, b,
                               lambda k: ("close", "stop") if m.l[k] <= con[2] else (("close", "3 days") if k >= st + 3 else None))
        out.append((i, j, m.c[i], pnl, op))
        i = j + 1
    return out


def weekly_calls(m, a, b):
    nb = len(m.dates)
    wk = [monday(x) for x in m.ymd]
    end = [i + 1 < nb and wk[i + 1] != wk[i] for i in range(nb)]
    ends = [i for i in range(nb) if end[i]]
    wr = rsi_series([m.c[i] for i in ends], 2)
    wrsi = {i: wr[k] for k, i in enumerate(ends)}
    d2 = rsi_series(m.c, 2)
    out, i = [], a
    while i <= b:
        if not (end[i] and wrsi[i] > 85):
            i += 1; continue
        e = next((e for e in m.expiries(i) if civil(m.ltd(e)) - civil(m.ymd[i]) > 7), None)
        k = nearest(m.strikes(i, e, "C"), m.c[i]) if e else None
        if k is None:
            i += 1; continue
        con = (e, "C", k)
        px0 = m.ch.close_fill(m.q(i, con))
        if px0 is None:
            i += 1; continue
        nxt = next((x for x in range(i + 1, nb) if end[x]), 10 ** 9)
        j, pnl, why, op = hold(m, con, -1, i, px0, b, lambda x: ("close", "rsi") if d2[x] < 60 else (("close", "week") if x >= nxt else None))
        out.append((i, j, m.c[i], pnl, op))
        i = j + 1
    return out


def covered_calls(m, a, b):
    out, i = [], a
    while i <= b:
        e = next((e for e in m.expiries(i) if m.ltd(e) > m.ymd[i]), None)
        k = next((s for s in m.strikes(i, e, "C") if s >= m.c[i]), None) if e else None
        px0 = m.ch.close_fill(m.q(i, (e, "C", k))) if k is not None else None
        if px0 is None:
            i += 1; continue
        j, pnl, why, op = hold(m, (e, "C", k), -1, i, px0, b, lambda x: None)
        pnl = [p + m.c[i + t] - m.c[i] for t, p in enumerate(pnl)]
        out.append((i, j, m.c[i], pnl, op))
        if op or j == i:
            break
        i = j
    return out


def eom_puts(m, a, b):
    ma, r2 = sma(m.c, 200), rsi_series(m.c, 2)
    # underlying signal (p.168): entry at the close; exit at the next open after RSI(2) > 65; no entry on the last bar
    sigs, cur, pend = [], None, False
    for i in range(a, b + 1):
        if pend:
            sigs.append((cur, i)); cur, pend = None, False
        if cur is not None:
            if i > cur and r2[i] > 65 and i < b:
                pend = True
            continue
        if i == b:
            break
        if m.c[i] > ma[i] and int(m.dates[i][8:10]) >= 25 and i >= 2 and m.c[i] < m.c[i - 1] < m.c[i - 2]:
            cur = i
    if cur is not None:
        sigs.append((cur, None))
    out, busy = [], 0
    for i, x in sigs:
        if i < busy:
            continue
        e = next((e for e in m.expiries(i) if m.ltd(e) > m.ymd[i]), None)
        k = nearest(m.strikes(i, e, "P"), m.c[i] * 0.98) if e else None
        px0 = m.ch.close_fill(m.q(i, (e, "P", k))) if k is not None else None
        if px0 is None:
            continue
        j, pnl, why, op = hold(m, (e, "P", k), -1, i, px0, b, lambda k_: ("open", "rsi") if k_ == x else None)
        busy = j + 1
        out.append((i, j, m.c[i], pnl, op))
    return out


# ---------------------------------------------------------------- account
def account(pos, a, b):
    """Each position sized at the account equity on its entry close; equity = cash + open positions' value."""
    pos = sorted(pos, key=lambda p: (p[0], p[1]))
    alloc = [0.0] * len(pos)
    cash, eq = 1.0, []
    for k in range(a, b + 1):
        e = cash + sum(alloc[j] * p[3][k - p[0]] / p[2] for j, p in enumerate(pos) if p[0] < k <= p[1] and alloc[j] > 0)
        for j, p in enumerate(pos):
            if p[0] < k == p[1] and alloc[j] > 0:
                cash += alloc[j] * p[3][-1] / p[2]
        e2 = e
        for j, p in enumerate(pos):
            if p[0] == k and e > 0:
                alloc[j] = e
                if p[1] == k:
                    cash += e * p[3][-1] / p[2]; e2 += e * p[3][-1] / p[2]
                else:
                    e2 += e * p[3][0] / p[2]
        eq.append(e2)
        if e2 <= 0:
            eq += [0.0] * (b - k)
            break
    return eq, alloc


def run(id_, m, a, b, vix=None):
    f = {"oiss_short_put": lambda: oiss(m, a, b, True, False), "oiss_short_call": lambda: oiss(m, a, b, False, False),
         "oiss_plus_three_put": lambda: oiss(m, a, b, True, True), "oiss_plus_three_call": lambda: oiss(m, a, b, False, True),
         "otvt_cvr1_sell_puts": lambda: cvr_puts(m, a, b, vix), "ccl08_weekly_rsi_options": lambda: weekly_calls(m, a, b),
         "mahrs_covered_calls": lambda: covered_calls(m, a, b), "spmt_eom_puts": lambda: eom_puts(m, a, b)}[id_]
    pos = f()
    eq, alloc = account(pos, a, b)
    trades = sorted(((m.dates[p[0]], m.dates[p[1]], p[3][-1] / p[2], p[4]) for p in pos), key=lambda t: (t[0], t[1]))
    return trades, eq
