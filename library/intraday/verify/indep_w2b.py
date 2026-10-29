#!/usr/bin/env python3
"""Intraday wave 2b: independent reference ledgers, written from the book rules (catalogue rule quotes, Collins' printed
TradeStation code in audits/acbff/code.txt), not from the Rust code.

backtest(id, B, a, b, P, ticker, M=None) -> list of trades {dir, ei, epx, xi, xpx, open} or None (id not covered here).
B = daily rows (date, o, h, l, c, v) as loaded by kpss_ref.load_instrument / load_stock; a..b = bar window (inclusive).
M = indep_ref.Minutes(ticker) for minute-path rules (1-minute bars read one day at a time).

Order conventions on daily bars (class A rules: daily bars decide every trade): market = the day's open, close = the
day's close, a stop/limit the open is already through fills at the open, otherwise at its level.
Minute path (class B), the wave-1 conventions:
  * an entry stop fills on the first minute that reaches it, at the level or the minute's open when it opened through it;
    two entry levels in one minute without opening through exactly one: order unknown, no trade that day;
  * fill minute that also reaches the protective stop: filled then stopped, unless that minute opened beyond the stop,
    then stopped only if it closed beyond it; later minutes fill the stop at its level or the minute's open;
  * a held position with two exit levels in one minute (mhp target/stop): the level nearer the minute's open first;
  * reversal systems (Saidenberg30, inside day) walk each minute along open -> nearer extreme -> other extreme.
  * NIFTY 50 minutes from 2026-08-11 are rejected (bad closing minutes in the source): no minute-path trade those days.
"""
import datetime, math, os, sys
HERE = os.path.dirname(os.path.abspath(__file__))
os.environ.setdefault("M1_DIR", "/home/server/.cache/strategy-vault-webapp/data/intraday")
sys.path.insert(0, HERE)
sys.path.insert(0, "/home/server/.cache/strategy-vault-webapp/claude-continuation-20261020/authors/collins/admission")
import collins_ref as CR  # noqa: E402  (Collins' printed-code engine and either-or blocks, independent of the Rust code)
from kpss_ref import load_instrument, load_stock  # noqa: E402,F401  (daily loaders shared by the admission references)
NAN = float("nan")
NIFTY_BAD_FROM = "2026-08-11"


def T(d, ei, epx, xi, xpx, open_=False):
    return dict(dir=int(d), ei=ei, epx=epx, xi=xi, xpx=xpx, open=open_)


def cols(B):
    return [r[1] for r in B], [r[2] for r in B], [r[3] for r in B], [r[4] for r in B]


def sma(v, n):
    return [NAN if i + 1 < n else sum(v[i + 1 - n:i + 1]) / n for i in range(len(v))]


def wd(d):  # 0 = Monday .. 4 = Friday
    return datetime.date.fromisoformat(d).weekday()


def tsdow(d):  # TradeStation DayOfWeek: 0 = Sunday .. 5 = Friday
    return (wd(d) + 1) % 7


INDIA_STOCKS = {"RELIANCE", "TCS"}


def mtick(t):  # the market's own tick: NSE 0.05, US 0.01
    return 0.05 if t.startswith("NIFTY") or t in INDIA_STOCKS else 0.01


# ============================================================ Collins (acbff): printed TradeStation code
def _acbff_day(B, a, b, sig, kind="mkt", k=0.0):
    """kind mkt: buy/short next bar at market; stop33: entry stops at o of tomorrow +/- k*avg(range,3);
    stop66: market entry plus sell/cover stop at o of tomorrow -/+ k*avg(range,3); all setexitonclose."""
    O_, H, L, C = cols(B); n = len(B); ar3 = sma([H[i] - L[i] for i in range(n)], 3)
    def s(i, mp, ei, ep, nx, tc):
        if i + 1 >= n: return
        bu, sh = sig(i)
        o1 = O_[i + 1]
        if kind == "stop33":
            if bu: CR.put(nx, CR.BUY, "stop", o1 + k * ar3[i], "b")
            if sh: CR.put(nx, CR.SHORT, "stop", o1 - k * ar3[i], "s")
        else:
            if bu: nx.append(CR.O(CR.BUY, "mkt"))
            if kind == "stop66": CR.put(nx, CR.SELL, "stop", o1 - k * ar3[i], "stop")
            if sh: nx.append(CR.O(CR.SHORT, "mkt"))
            if kind == "stop66": CR.put(nx, CR.COVER, "stop", o1 + k * ar3[i], "stop")
        CR.day_exit(nx)
    return [T(t["dir"], t["ei"], t["epx"], t["xi"], t["xpx"], t["open"]) for t in CR.engine(B, a, b, s)]


def _either(B):
    """The six either-or values of chapters 12/17/42: aa (2 vs 5 day), bb (40-day), cc (50-day order of extremes),
    dd (range vs 10-day, both halves), ee (15-day high/low midpoint); persisting EasyLanguage variables."""
    q, u, x, y, z = CR.five(B)
    return u, q, x, y, z


def _cupcap(B, i):  # +1 cup, -1 cap (cap set last wins), 0
    O_, H, L, C = cols(B)
    if i < 4: return 0.0
    v = 0.0
    if L[i] > L[i - 1] and L[i - 1] < min(L[i - 4:i - 1]) and C[i] > C[i - 1] and C[i - 1] < C[i - 2]: v = 1.0
    if H[i] < H[i - 1] and H[i - 1] > max(H[i - 4:i - 1]) and C[i] < C[i - 1] and C[i - 1] > C[i - 2]: v = -1.0
    return v


def _sr(B, i, n=0.2):  # Table 16.1 (buy cond, short cond)
    O_, H, L, C = cols(B)
    if i < 2: return False, False
    hh, lh, hl, ll = max(H[i - 2:i + 1]), min(H[i - 2:i + 1]), max(L[i - 2:i + 1]), min(L[i - 2:i + 1])
    return hl - ll <= n * (hh - ll), hh - lh <= n * (hh - ll)


def _oc_e(B, persist):  # e: +1 c>o, -1 c<o; persisting (no else) or reset to 0
    O_, H, L, C = cols(B); out = []; e = 0.0
    for i in range(len(B)):
        if not persist: e = 0.0
        if C[i] > O_[i]: e = 1.0
        if C[i] < O_[i]: e = -1.0
        out.append(e)
    return out


def _ff23(B, persist_e):  # Table 42.1 vote, persisting ff
    a3 = sma(_oc_e(B, persist_e), 3)
    return CR.persist(len(B), lambda i: (a3[i] < 0, a3[i] > 0))


def _nee(B):  # Table 43.1 five non-either-or values (each 0 unless set this bar)
    O_, H, L, C = cols(B); out = []
    for i in range(len(B)):
        c = lambda k: C[i - k]  # noqa: E731
        aa = bb = 0.0
        if i >= 6:
            if c(1) < c(2) and c(2) > c(3) > c(4) > c(5) > c(6): aa = 1.0
            if c(1) > c(2) and c(2) < c(3) < c(4) < c(5) < c(6): aa = -1.0
        if i >= 7:
            if c(1) < c(2) and c(2) > c(3) > c(4) > c(5) > c(6) > c(7): bb = 1.0
            if c(1) > c(2) and c(2) < c(3) < c(4) < c(5) < c(6) < c(7): bb = -1.0
        bs, ss = _sr(B, i)
        dd = -1.0 if ss else (1.0 if bs else 0.0)
        ee = 0.0
        if i >= 1:
            if C[i] < O_[i] and C[i - 1] < O_[i - 1]: ee = 1.0
            if C[i] > O_[i] and C[i - 1] > O_[i - 1]: ee = -1.0
        out.append(aa + bb + _cupcap(B, i) + dd + ee)
    return out


def _dow_ef(B):  # Table 26.3/26.4: e = c-o on up days, f = o-c on down days (persisting); highest(e,k) vs highest(f,k)
    O_, H, L, C = cols(B); e, f = [], []; se = sf = 0.0
    for i in range(len(B)):
        if C[i] > O_[i]: se = C[i] - O_[i]
        if C[i] < O_[i]: sf = O_[i] - C[i]
        e.append(se); f.append(sf)
    def cmp(i, k):
        if i + 1 < k: return None
        return max(e[i + 1 - k:i + 1]), max(f[i + 1 - k:i + 1])
    return cmp


def acbff(id_, B, a, b, P, t):
    O_, H, L, C = cols(B); n = len(B); R_ = [H[i] - L[i] for i in range(n)]
    c1 = lambda i: C[i - 1] if i >= 1 else NAN  # noqa: E731
    if id_ == "acbff_c40":
        m = sma(C, int(P.get("n", 40))); return _acbff_day(B, a, b, lambda i: (C[i] > m[i], C[i] < m[i]))
    if id_ == "acbff_2v5":
        m2, m5 = sma(C, 2), sma(C, 5); return _acbff_day(B, a, b, lambda i: (m2[i] < m5[i], m2[i] > m5[i]))
    if id_ == "acbff_fade2c":
        return _acbff_day(B, a, b, lambda i: (i >= 2 and C[i] < C[i - 1] < C[i - 2], i >= 2 and C[i] > C[i - 1] > C[i - 2]))
    if id_ == "acbff_ext50":
        m = int(P.get("m", 50)); hb, lb = CR.xbar(C, m, True), CR.xbar(C, m, False)
        return _acbff_day(B, a, b, lambda i: (hb[i] > lb[i], hb[i] < lb[i]))
    if id_ == "acbff_three":  # Table 9.1
        u, q, x, y, z = _either(B); return _acbff_day(B, a, b, lambda i: (q[i] + u[i] + x[i] > 0, q[i] + u[i] + x[i] < 0))
    if id_ == "acbff_range10":  # Table 10.5 (both halves)
        ar = sma(R_, int(P.get("n", 10)))
        return _acbff_day(B, a, b, lambda i: ((R_[i] < ar[i] and C[i] > c1(i)) or (R_[i] > ar[i] and C[i] < c1(i)),
                                               (R_[i] < ar[i] and C[i] < c1(i)) or (R_[i] > ar[i] and C[i] > c1(i))))
    if id_ == "acbff_hl15":
        k = int(P.get("n", 15)); ah, al = sma(H, k), sma(L, k)
        return _acbff_day(B, a, b, lambda i: (C[i] > (ah[i] + al[i]) / 2, C[i] < (ah[i] + al[i]) / 2))
    if id_ in ("acbff_five", "acbff_five_4of5") or id_.startswith("acbff_c3_"):
        q, u, x, y, z = CR.five(B); V = [q, u, x, y, z]
        idx = [int(ch) - 1 for ch in (id_[9:] if id_.startswith("acbff_c3_") else "12345")]
        thr = 3 if id_ == "acbff_five_4of5" else None
        def sg(i):
            s = sum(V[k][i] for k in idx)
            return (s >= 3, s <= -3) if thr else (s > 0, s < 0)
        return _acbff_day(B, a, b, sg)
    if id_ == "acbff_fade_oc":
        return _acbff_day(B, a, b, lambda i: (C[i] < O_[i], C[i] > O_[i]))
    if id_ == "acbff_fade2oc":
        return _acbff_day(B, a, b, lambda i: (i >= 1 and C[i] < O_[i] and C[i - 1] < O_[i - 1], i >= 1 and C[i] > O_[i] and C[i - 1] > O_[i - 1]))
    if id_ == "acbff_cup_day":
        return _acbff_day(B, a, b, lambda i: (_cupcap(B, i) > 0, _cupcap(B, i) < 0))
    if id_ == "acbff_sr20":
        k = P.get("n", 0.2); return _acbff_day(B, a, b, lambda i: _sr(B, i, k))
    if id_.startswith("acbff_eight"):
        s8 = CR.eight(B, P.get("prec", 0) >= 0.5)
        thr = 3 if id_.endswith("_3") else None
        sg = (lambda i: (s8[i] >= 3, s8[i] <= -3)) if thr else (lambda i: (s8[i] > 0, s8[i] < 0))
        if "stop33" in id_: return _acbff_day(B, a, b, sg, "stop33", P.get("k", 0.33))
        if "stop66" in id_: return _acbff_day(B, a, b, sg, "stop66", P.get("k", 0.66))
        return _acbff_day(B, a, b, sg)
    if id_ == "acbff_monday":  # Table 26.1
        return _acbff_day(B, a, b, lambda i: (i >= 1 and tsdow(B[i][0]) == 5 and C[i] > C[i - 1], i >= 1 and tsdow(B[i][0]) == 5 and C[i] < C[i - 1]))
    if id_ == "acbff_tuesday":  # Table 26.2
        return _acbff_day(B, a, b, lambda i: (i >= 1 and tsdow(B[i][0]) == 1 and C[i] < C[i - 1], i >= 1 and tsdow(B[i][0]) == 1 and C[i] > C[i - 1]))
    if id_ in ("acbff_wed_fri", "acbff_dow"):  # Tables 26.3 / 26.4
        cmp = _dow_ef(B)
        def sg(i):
            d = tsdow(B[i][0]); bu = sh = False
            if id_ == "acbff_dow" and i >= 1:
                if d == 5: bu, sh = C[i] > C[i - 1], C[i] < C[i - 1]
                if d == 1: bu, sh = C[i] < C[i - 1], C[i] > C[i - 1]
            if d in (2, 3, 4):
                v = cmp(i, d)
                if v: bu, sh = v[0] < v[1], v[0] > v[1]
            return bu, sh
        return _acbff_day(B, a, b, sg)
    if id_ in ("acbff_dom_day", "acbff_dom_day_cc"):  # Tables 27.3 / 27.4
        nn, pp = int(P.get("n", 21)), int(P.get("p", 6)); allc = P.get("close_all", 0) >= 0.5
        def sg(i):
            d = int(B[i][0][8:10])
            if id_ == "acbff_dom_day": return d >= nn or d < pp, pp <= d < nn
            dn, up = i >= 1 and C[i] < C[i - 1], i >= 1 and C[i] > C[i - 1]
            bu = (dn and (d >= nn or d < pp)) if allc else ((dn and d >= nn) or d < pp)
            return bu, up and pp <= d < nn
        return _acbff_day(B, a, b, sg)
    if id_ == "acbff_5c_day":  # Table 36.1
        def sg(i):
            if i < 6: return False, False
            c = lambda k: C[i - k]  # noqa: E731
            return (c(0) > c(1) and c(1) < c(2) < c(3) < c(4) < c(5) < c(6), c(0) < c(1) and c(1) > c(2) > c(3) > c(4) > c(5) > c(6))
        return _acbff_day(B, a, b, sg)
    if id_ == "acbff_fade23oc":  # Table 42.1 (e persisting)
        ff = _ff23(B, True); return _acbff_day(B, a, b, lambda i: (ff[i] > 0, ff[i] < 0))
    if id_ in ("acbff_six", "acbff_six_stop66", "acbff_nee5", "acbff_six_q", "acbff_six_q66", "acbff_six_q66_v2"):
        u, q, x, y, z = _either(B)
        ff = _ff23(B, not id_.startswith("acbff_six_q"))  # 42.x: e persists; 44.x: "else e=0"
        six = [u[i] + q[i] + x[i] + y[i] + z[i] + ff[i] for i in range(n)]
        nee = _nee(B)
        if id_ == "acbff_nee5": sg = lambda i: (nee[i] > 0, nee[i] < 0)  # noqa: E731
        elif id_ in ("acbff_six", "acbff_six_stop66"): sg = lambda i: (six[i] > 0, six[i] < 0)  # noqa: E731
        elif id_ == "acbff_six_q66_v2":
            sg = lambda i: ((six[i] > 0 and nee[i] >= 0) or (six[i] == 0 and nee[i] > 0), (six[i] < 0 and nee[i] <= 0) or (six[i] == 0 and nee[i] < 0))  # noqa: E731
        else: sg = lambda i: (six[i] > 0 and nee[i] >= 0, six[i] < 0 and nee[i] <= 0)  # noqa: E731
        if "66" in id_: return _acbff_day(B, a, b, sg, "stop66", P.get("k", 0.66))
        return _acbff_day(B, a, b, sg)
    return None


# ============================================================ other class-A rules (daily bars decide every trade)
def oc_days(B, a, b, pick, start=0):
    """pick(i) -> +1/-1/0: in at the open of bar i, out at its close."""
    out = []
    for i in range(max(a, start), b + 1):
        d = pick(i)
        if d: out.append(T(d, i, B[i][1], i, B[i][4]))
    return out


def stop_day(B, i, d, lvl):  # entry stop on bar i: fill price or None
    o, h, l = B[i][1], B[i][2], B[i][3]
    if d > 0: return max(o, lvl) if h >= lvl else None
    return min(o, lvl) if l <= lvl else None


def us_holidays(y):  # Merrill's seven, observed dates (jbcdt2 note): Good Friday, Memorial, July 4, Labor, Thanksgiving, Christmas, New Year's
    D = datetime.date
    def obs(d):
        return d - datetime.timedelta(1) if d.weekday() == 5 else d + datetime.timedelta(1) if d.weekday() == 6 else d
    a_ = y % 19; b_ = y // 100; c_ = y % 100; d_ = b_ // 4; e_ = b_ % 4; f_ = (b_ + 8) // 25; g_ = (b_ - f_ + 1) // 3
    h_ = (19 * a_ + b_ - d_ - g_ + 15) % 30; i_ = c_ // 4; k_ = c_ % 4; l_ = (32 + 2 * e_ + 2 * i_ - h_ - k_) % 7
    m_ = (a_ + 11 * h_ + 22 * l_) // 451; mo = (h_ + l_ - 7 * m_ + 114) // 31; da = (h_ + l_ - 7 * m_ + 114) % 31 + 1
    gf = D(y, mo, da) - datetime.timedelta(2)
    if y >= 1971:
        mem = D(y, 5, 31)
        while mem.weekday() != 0: mem -= datetime.timedelta(1)
    else: mem = D(y, 5, 30)
    lab = D(y, 9, 1)
    while lab.weekday() != 0: lab += datetime.timedelta(1)
    tg = D(y, 11, 1)
    while tg.weekday() != 3: tg += datetime.timedelta(1)
    tg += datetime.timedelta(21)
    out = [(gf, 0), (mem, 1), (obs(D(y, 7, 4)), 2), (lab, 3), (tg, 4), (obs(D(y, 12, 25)), 5)]
    ny = D(y, 1, 1)
    out.append((ny if ny.weekday() == 5 else obs(ny), 6))  # a Saturday New Year's Day closes no day: its target is the Friday before
    return out


def other_a(id_, B, a, b, P, t):
    O_, H, L, C = cols(B); n = len(B); dates = [r[0] for r in B]
    if id_ == "ftm2_ac_vote":
        A, Bb, MA, EXT = (int(P.get(k, v)) for k, v in (("a", 2), ("b", 5), ("ma", 40), ("ext", 50)))
        sgn = lambda v: (v > 0) - (v < 0)  # noqa: E731
        def pick(i):
            if i < max(A, Bb, MA, EXT): return 0
            v1 = sgn(C[i - 1] - sum(C[i - MA:i]) / MA)
            v2 = sgn(sum(C[i - Bb:i]) / Bb - sum(C[i - A:i]) / A)
            w = C[i - EXT:i]; ih = max(k for k in range(EXT) if w[k] == max(w)); il = max(k for k in range(EXT) if w[k] == min(w))
            v3 = 1 if ih < il else -1 if ih > il else 0
            return sgn(v1 + v2 + v3)
        return oc_days(B, a, b, pick)
    if id_ == "ftm2_dm_skew":
        al, be, gm, sl, k, st = (P.get(x, v) for x, v in (("alpha", .1), ("beta", .05), ("gamma", .333), ("sl", 4.4), ("k", 1.25), ("startup", 60)))
        S = Dd = C[0]; smd = dmd = m3 = 0.0; prev = 0.0; clip = []
        for i in range(n):
            S = al * C[i] + (1 - al) * S; Dd = al * S + (1 - al) * Dd
            dev = C[i] - ((2 - al) * S - Dd) / (1 - al)
            smd = be * abs(dev) + (1 - be) * smd; dmd = be * smd + (1 - be) * dmd
            std = k * ((2 - be) * smd - dmd) / (1 - be)
            m3 = gm * dev ** 3 + (1 - gm) * m3
            g1 = m3 / std ** 3 if std > 0 else 0.0
            cl = max(g1, 0.0); clip.append(cl if cl > prev else 0.0); prev = cl
        out, busy = [], -1
        for d0 in range(max(int(st), 1), b):
            if d0 < a or not (clip[d0 - 1] < sl <= clip[d0]): continue
            e = d0 + 1
            if e <= busy: continue
            if C[e] > O_[e]: out.append(T(1, e, O_[e], e, C[e])); busy = e
            elif e + 1 <= b: out.append(T(1, e, O_[e], e + 1, C[e + 1])); busy = e + 1
            else: out.append(T(1, e, O_[e], b, C[b], True)); busy = b
        return out
    if id_ == "rgc_dow_oc":
        on = [P.get(k, v) >= 0.5 for k, v in (("mon", 1), ("tue", 0), ("wed", 1), ("thu", 0), ("fri", 0))]
        return oc_days(B, a, b, lambda i: 1 if wd(dates[i]) < 5 and on[wd(dates[i])] else 0)
    if id_ in ("rgc_dowm_thu_jul_long", "rgc_dowm_mon_dec_long", "rgc_dowm_thu_dec_short"):
        d, m, s = {"rgc_dowm_thu_jul_long": (3, 7, 1), "rgc_dowm_mon_dec_long": (0, 12, 1), "rgc_dowm_thu_dec_short": (3, 12, -1)}[id_]
        return oc_days(B, a, b, lambda i: s if wd(dates[i]) == d and int(dates[i][5:7]) == m else 0)
    if id_ == "rgc_cdow_fri_short":
        return oc_days(B, a, b, lambda i: -1 if wd(dates[i]) == 4 and C[i - 1] < O_[i - 1] else 0, start=1)
    if id_ in ("rgc_runaway_gap", "rgc_runaway_gap_f5"):
        k, sign = P.get("k", 0.4), (-1 if P.get("sign", 1) < 0 else 1)
        tr = [H[0] - L[0]] + [max(H[i], C[i - 1]) - min(L[i], C[i - 1]) for i in range(1, n)]
        atr = sma(tr, 3); out = []
        for i in range(max(a, 7), b + 1):
            A_ = atr[i - 1]
            if A_ != A_: continue
            if id_.endswith("_f5") and not C[i - 1] < C[i - 6]: continue
            if O_[i] > H[i - 1]: d, lv = 1, O_[i] + k * A_
            elif O_[i] < L[i - 1]: d, lv = -1, O_[i] + sign * k * A_
            else: continue
            px = stop_day(B, i, d, lv)
            if px is not None: out.append(T(d, i, px, i, C[i]))
        return out
    if id_ in ("jkbwts_buymon1400", "jk36k_jk1_buy_monday", "jkbk_buy_monday_ts83", "stad10_mwt"):
        stp = {"jkbwts_buymon1400": 1400 / 500, "jk36k_jk1_buy_monday": 1250 / 500, "jkbk_buy_monday_ts83": 1000 / 250, "stad10_mwt": 1500 / 500}[id_]
        m2 = sma(C, 2); out = []
        for i in range(a + 1, b + 1):  # orders from the Friday close fill at the next open
            if wd(dates[i - 1]) != 4: continue
            if id_ == "stad10_mwt":
                if i - 1 < 2: continue
                d = 1 if C[i - 1] > m2[i - 1] else -1
            else: d = 1
            sp = O_[i] - d * stp
            hit = (L[i] <= sp) if d > 0 else (H[i] >= sp)
            out.append(T(d, i, O_[i], i, sp if hit else C[i]))
        return out
    if id_ == "stad2_stats2":
        wk = lambda d: (datetime.date.fromisoformat(d) - datetime.date(1970, 1, 5)).days // 7  # noqa: E731
        return [T(1, i, O_[i], i, C[i]) for i in range(max(a + 1, 2), b + 1) if wk(dates[i]) != wk(dates[i - 1])]
    if id_ == "lg3k_tr_day":
        nn, pct = int(P.get("days", 4)), P.get("pct", 55) / 100
        tk = P.get("tick", 0) or mtick(t)
        tr = [H[0] - L[0]] + [max(H[i], C[i - 1]) - min(L[i], C[i - 1]) for i in range(1, n)]
        out = []
        for i in range(a, b + 1):
            if i < nn + 1: continue
            f = math.ceil(sum(tr[i - nn:i]) / nn * pct / tk - 1e-9) * tk
            bl, sl_ = O_[i] + f, O_[i] - f
            if H[i] >= bl: out.append(T(1, i, max(bl, O_[i]), i, C[i]))
            if L[i] <= sl_: out.append(T(-1, i, min(sl_, O_[i]), i, C[i]))
        return out
    if id_ in ("tdperl_clop_long", "tdperl_clop_short"):
        out = []
        for i in range(max(a, 5), b + 1):
            if id_.endswith("long"):
                lv = max(O_[i - 1], C[i - 1])
                if O_[i] < C[i - 1] and O_[i] < O_[i - 1] and H[i] > lv: out.append(T(1, i, lv, i, C[i]))
            else:
                lv = min(O_[i - 1], C[i - 1])
                if O_[i] > C[i - 1] and O_[i] > O_[i - 1] and L[i] < lv: out.append(T(-1, i, lv, i, C[i]))
        return out
    if id_ == "dgmf_vbo_taylor":
        mult = P.get("mult", 0.7); out = []
        for i in range(max(a, 4), b + 1):
            buy = O_[i] + (H[i - 1] - C[i - 1]) * mult
            if O_[i] < C[i - 1] and H[i] > buy and C[i - 1] < C[i - 2] < C[i - 3] < C[i - 4]:
                out.append(T(1, i, buy, i, O_[i] if O_[i] >= C[i] else C[i]))
        return out
    if id_ == "jbcdt2_dates_oc":
        sp = {(1, 5), (1, 14), (2, 2), (4, 12), (4, 16), (5, 19), (6, 1), (7, 14), (8, 27), (11, 24), (12, 13)}
        return oc_days(B, a, b, lambda i: 1 if (int(dates[i][5:7]), int(dates[i][8:10])) in sp else 0)
    if id_ == "jbcdt2_preholiday_oc":
        three = P.get("set", 0) >= 0.5
        hol = sorted(d for y in range(int(dates[a][:4]), int(dates[b][:4]) + 2) for d, tag in us_holidays(y) if not three or tag in (2, 5, 6))
        tgt = set()
        for h_ in hol:
            hs = h_.isoformat(); j = max((k for k in range(n) if dates[k] < hs), default=None)
            if j is not None and j + 1 < n: tgt.add(j)  # the session before the holiday; the data's last bar has no known next session
        return oc_days(B, a, b, lambda i: 1 if i in tgt else 0)
    if id_ == "jbssf_gap":
        q = P.get("gs_pct", 5) / 100; out = []
        for i in range(max(a, 1), b + 1):
            gs = q * (H[i - 1] - L[i - 1])
            if O_[i] > H[i - 1] and O_[i] >= H[i - 1] + gs: d, lv = -1, H[i - 1]
            elif O_[i] < L[i - 1] and O_[i] <= L[i - 1] - gs: d, lv = 1, L[i - 1]
            else: continue
            px = stop_day(B, i, d, lv)
            if px is not None: out.append(T(d, i, px, i, C[i]))
        return out
    return None


# ============================================================ class B: minute path
def mday(M, t, d):
    if M is None or (t == "NIFTY 50" and d >= NIFTY_BAD_FROM): return None
    return M.day(d)  # [(minute, o, h, l, c)]


AMB = {}  # convention counters: two entry levels in one minute (no trade), fill-minute stop, two exit levels in one minute


def amb(k): AMB[k] = AMB.get(k, 0) + 1


def touched(m, up, px): return m[2] >= px if up else m[3] <= px
def fill(m, up, px): return max(m[1], px) if up else min(m[1], px)


def stop_entry(bars, entries, stop_of):
    """entries [(up, px)]; stop_of(k, fill) -> (up, px) or None. Returns (k, fill, exit_px or None)."""
    for j, m in enumerate(bars):
        hit = [k for k, (up, px) in enumerate(entries) if touched(m, up, px)]
        if not hit: continue
        if len(hit) > 1:
            thr = [k for k in hit if (m[1] >= entries[k][1] if entries[k][0] else m[1] <= entries[k][1])]
            if len(thr) != 1: amb("two entries in one minute: no trade"); return None
            k, f = thr[0], m[1]
        else:
            k = hit[0]; f = fill(m, *entries[k])
        s = stop_of(k, f)
        if s is None: return k, f, None
        sup, spx = s
        beyond = (lambda v: v >= spx) if sup else (lambda v: v <= spx)
        if touched(m, sup, spx): amb("fill minute also reaches the stop")
        if (beyond(m[4]) if beyond(m[1]) else touched(m, sup, spx)): return k, f, spx
        for m2 in bars[j + 1:]:
            if touched(m2, sup, spx): return k, f, fill(m2, sup, spx)
        return k, f, None
    return None


def first_exit(bars, levels):
    """Held position: first of levels [(up, px)] touched; two in one minute: opened through exactly one -> it, at the open;
    else the level nearer the minute's open (tie: first listed). Returns (k, fill, bar index) or None."""
    for j, m in enumerate(bars):
        hit = [k for k, (up, px) in enumerate(levels) if touched(m, up, px)]
        if not hit: continue
        if len(hit) == 1: return hit[0], fill(m, *levels[hit[0]]), j
        thr = [k for k in hit if (m[1] >= levels[k][1] if levels[k][0] else m[1] <= levels[k][1])]
        if len(thr) == 1: return thr[0], m[1], j
        k = min(hit, key=lambda k: (abs(levels[k][1] - m[1]), k)); amb("two exit levels in one minute: nearer the open")
        return k, fill(m, *levels[k]), j
    return None


def minute_b(id_, B, a, b, P, t, M):
    O_, H, L, C = cols(B); dates = [r[0] for r in B]; out = []
    if id_ in ("hfm_news", "hfm_news_long", "hfm_news_short"):
        dirs = {"hfm_news": (1, -1), "hfm_news_long": (1,), "hfm_news_short": (-1,)}[id_]
        tk = mtick(t)
        for i in range(max(a, 1), b + 1):
            # Rule #2/#3: gap below yesterday's low -> buy stop one tick above it; gap above the high -> sell stop one tick below
            if O_[i] < L[i - 1]: d, lv = 1, L[i - 1] + tk
            elif O_[i] > H[i - 1]: d, lv = -1, H[i - 1] - tk
            else: continue
            if d not in dirs: continue
            bars = mday(M, t, dates[i])
            if not bars: continue
            r = stop_entry(bars, [(d > 0, lv)], lambda k, f: (d < 0, O_[i]))  # Rule #4: stop at the morning's opening price
            if r: out.append(T(d, i, r[1], i, r[2] if r[2] is not None else C[i]))
        return out
    if id_ == "jbcdt_gap_go":
        pa, sl = P.get("pa", 0.1), P.get("stop", 5.0)
        for i in range(max(a, 1), b + 1):
            if O_[i] < L[i - 1]: d, lv = 1, L[i - 1] + pa
            elif O_[i] > H[i - 1]: d, lv = -1, H[i - 1] - pa
            else: continue
            bars = mday(M, t, dates[i])
            if not bars: continue
            r = stop_entry(bars, [(d > 0, lv)], lambda k, f: (d < 0, f - d * sl))
            if r: out.append(T(d, i, r[1], i, r[2] if r[2] is not None else C[i]))
        return out
    if id_ == "bab80_daytrade":
        nm, stp = 11, 3000 / 500
        for i in range(max(a, nm + 2), b + 1):
            # yesterday's 11-day momentum > that of two days ago; today's open > yesterday's (H+L+C)/3
            if not (C[i - 1] - C[i - 1 - nm] > C[i - 2] - C[i - 2 - nm] and O_[i] > (H[i - 1] + L[i - 1] + C[i - 1]) / 3): continue
            lv = C[i - 1] + max(H[i - 1], C[i - 2]) - min(L[i - 1], C[i - 2])
            bars = mday(M, t, dates[i])
            if not bars: continue
            r = stop_entry(bars, [(True, lv)], lambda k, f: (False, lv - stp))  # 600 points under the system entry point
            if r: out.append(T(1, i, r[1], i, r[2] if r[2] is not None else C[i]))
        return out
    if id_ == "upg18_traders_dream":
        stp = P.get("stop_pts", 1.2)
        for j in range(a + 1, b + 1):
            i = j - 1
            if i < 2: continue
            d = 1 if C[i] < C[i - 2] else -1  # IF C < C[2] buy next bar H stop; IF C >= C[2] sell short next bar L stop
            bars = mday(M, t, dates[j])
            if not bars: continue
            r = stop_entry(bars, [(d > 0, H[i] if d > 0 else L[i])], lambda k, f: (d < 0, f - d * stp))
            if r: out.append(T(d, j, r[1], j, r[2] if r[2] is not None else C[j]))
        return out
    if id_ == "lgvp_volpat":
        for i in range(max(a, 4), b + 1):
            bp, sp = volpat(B, i)
            bars = mday(M, t, dates[i])
            if not bars: continue
            ent = [(True, bp), (False, sp)]
            r = stop_entry(bars, ent, lambda k, f: ent[1 - k])  # OCO: the other order is the stop-loss
            if r: out.append(T(1 if r[0] == 0 else -1, i, r[1], i, r[2] if r[2] is not None else C[i]))
        return out
    if id_ in ("mhp_ftse_12_20041015", "mhp_ftse_3_20040804"):
        conds = {"mhp_ftse_12_20041015": [("L", 4, "H", 0), ("H", 0, "L", 3), ("L", 3, "L", 2), ("L", 2, "L", 1)],
                 "mhp_ftse_3_20040804": [("H", 4, "H", 3), ("H", 3, "H", 5), ("H", 5, "L", 4), ("L", 4, "L", 5), ("L", 5, "L", 3)]}[id_]
        F = {"O": O_, "H": H, "L": L, "C": C}
        tgt, stp = P.get("target", 2.0) / 100, P.get("stop", 4.0) / 100
        def sig(i):
            return all(i - max(x[1], x[3]) >= 0 and F[x[0]][i - x[1]] > F[x[2]][i - x[3]] for x in conds)
        cur = None
        for i in range(a, b + 1):
            if cur:
                k0, ep, tp, sp = cur
                bars = mday(M, t, dates[i]); ex = None
                if bars:
                    r = first_exit(bars, [(False, sp), (True, tp)])
                    if r: ex = r[1]
                else:  # no minutes: daily reading (stop first when both are inside the bar)
                    o = O_[i]
                    ex = o if o <= sp or o >= tp else sp if L[i] <= sp else tp if H[i] >= tp else None
                if ex is not None: out.append(T(1, k0, ep, i, ex)); cur = None
            if cur or not sig(i) or i >= b: continue
            k = i + 1; ep = O_[k]
            cur = (k, ep, ep * (1 + tgt), ep * (1 - stp))
            # target/stop work from the entry bar: handled when the loop reaches bar k
        if cur: out.append(T(1, cur[0], cur[1], b, C[b], True))
        return out
    if id_ == "jkbwts_saidenberg30":
        k, mt, stp = P.get("k", 0.66), int(P.get("max_trades", 2)), 1000 / 500
        last_n = 0
        for j in range(a + 1, b + 1):
            i = j - 1
            if last_n >= mt: last_n = 0; continue
            last_n = 0
            r = H[i] - L[i]
            bars = mday(M, t, dates[j])
            if not bars: continue
            live = [(1, H[i] + r * k), (-1, L[i] - r * k)]
            pos = None
            for m in bars:
                o, h, l = m[1], m[2], m[3]
                path = [o, l, h] if o - l <= h - o else [o, h, l]
                at, first = o, True
                for to_ in path[1:]:
                    while True:
                        up = to_ > at; best = None
                        cands = [(px, kk, d > 0) for kk, (d, px) in enumerate(live)]
                        if pos: cands.append((pos[1] - pos[0] * stp, -1, pos[0] < 0))
                        for px, kk, trig_up in cands:
                            if first and ((trig_up and px <= at) or (not trig_up and px >= at)): f = at
                            elif trig_up and up and at < px <= to_: f = px
                            elif not trig_up and not up and to_ <= px < at: f = px
                            else: continue
                            if best is None or abs(f - at) < abs(best[0] - at): best = (f, kk)
                        first = False
                        if best is None: break
                        f, kk = best
                        if kk == -1: out.append(T(pos[0], pos[2], pos[1], j, f)); pos = None
                        else:
                            d, _ = live.pop(kk)
                            if not (pos and pos[0] == d):
                                if pos: out.append(T(pos[0], pos[2], pos[1], j, f))
                                pos = (d, f, j); last_n += 1
                        at = f
                    at = to_
            if pos: out.append(T(pos[0], pos[2], pos[1], j, C[j]))
        return out
    if id_ == "putg_inside_day":
        off = P.get("ticks", 3) * P.get("tick", 0.05); pu = P.get("dir_filter", 1) >= 0.5
        for j in range(max(a, 3), b + 1):
            i = j - 1
            if not (H[i] < H[i - 1] and L[i] > L[i - 1]): continue  # inside day
            live = []
            if not pu or C[i - 1] > O_[i - 1]: live.append((1, H[i] + off))
            if not pu or C[i - 1] < O_[i - 1]: live.append((-1, L[i] - off))
            bars = mday(M, t, dates[j])
            if not bars: continue
            pos = None
            for m in bars:
                o, h, l = m[1], m[2], m[3]
                p = [o, l, h] if o - l < h - o else [o, h, l]
                ev = []
                for kk, (d, px) in enumerate(live):
                    if (d > 0 and o >= px) or (d < 0 and o <= px): ev.append((0.0, kk, o)); continue
                    for s in range(2):
                        x0, x1 = p[s], p[s + 1]
                        if (d > 0 and x0 < px <= x1) or (d < 0 and x0 > px >= x1):
                            ev.append((s + (px - x0) / (x1 - x0), kk, px)); break
                ev.sort()
                for _, kk, px in ev:
                    d = live[kk][0]
                    if pos and pos[0] == d: continue
                    if pos: out.append(T(pos[0], j, pos[1], j, px))
                    pos = (d, px)
                hit = {e[1] for e in ev}
                live = [q for kk, q in enumerate(live) if kk not in hit]
            if pos: out.append(T(pos[0], j, pos[1], j, C[j]))
        return out
    return None


def volpat(B, i, kc=0.70, ko=0.75, tick=0.05):
    """VOLPAT p.8-21: 3-day total true range, x.70 from yesterday's close / x.75 from today's open, rounded UP to 0.05;
    the lower buy / higher sell point, except that an open beyond the close-based point uses the open-based one; then the
    patterns #3, #2, #1 (first one found, either side, is the only pattern that day), which may only lower the buy / raise
    the sell point."""
    O_, H, L, C = cols(B)
    w = range(i - 3, i)
    ttr = max(max(H[j] for j in w) - min(C[j] for j in w), max(C[j] for j in w) - min(L[j] for j in w))
    ru = lambda v: math.ceil(v / tick - 1e-9) * tick  # noqa: E731
    fc, fo = ru(ttr * kc), ru(ttr * ko)
    op = O_[i]
    bc, sc, bo, so = C[i - 1] + fc, C[i - 1] - fc, op + fo, op - fo
    buy = bo if op > bc else min(bc, bo)
    sell = so if op < sc else max(sc, so)
    A, Bb, Cc, Dd = i - 1, i - 2, i - 3, i - 4
    pats = [
        (H[A] > H[Bb] and C[A] < C[Bb] and op < C[Bb], L[A] < L[Bb] and C[A] > C[Bb] and op > C[Bb], C[Bb], C[Bb]),   # 3
        (L[Bb] < L[Cc] and L[A] > L[Bb] and op < C[Bb], H[Bb] > H[Cc] and H[A] < H[Bb] and op > C[Bb], C[Bb], C[Bb]),  # 2
        (C[Cc] < C[Dd] and C[Bb] > C[Cc] and C[A] < C[Bb] and op < H[A], C[Cc] > C[Dd] and C[Bb] < C[Cc] and C[A] > C[Bb] and op > L[A], H[A], L[A]),  # 1
    ]
    for pb, ps, vb, vs in pats:
        if pb or ps:
            if pb and vb < buy: buy = vb
            if ps and vs > sell: sell = vs
            break
    return buy, sell


def backtest(id_, B, a, b, P, t, M=None):
    if id_.startswith("acbff_"): return acbff(id_, B, a, b, P, t)
    r = other_a(id_, B, a, b, P, t)
    if r is not None: return r
    return minute_b(id_, B, a, b, P, t, M)
