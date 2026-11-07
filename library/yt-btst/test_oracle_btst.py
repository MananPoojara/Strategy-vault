"""Hand-calculated unit tests for oracle_btst (synthetic bars).  Run:
   ~/.venvs/strategy-vault/bin/python -I test_oracle_btst.py
Layout: every synthetic day has the 5-min grid 09:15..15:25 with flat price (o=h=l=c=base) unless overridden.
"""
import datetime as dt, os, sys, unittest
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import oracle_btst as o

GRID = [h * 100 + m for h in range(9, 16) for m in range(0, 60, 5) if 915 <= h * 100 + m <= 1525]
BASE = dict(days=2, entry_time=1450, exit_time=925, pb_pct=0, roc_days=2, ibs_thr=1.0, roc_thr=-100,
            hardstop=0, flt_type=3)


def mkday(day, base, over=None, times=None, vols=None):
    """bars of one day; over = {hhmm: (o,h,l,c)}; times = restrict grid; vols = {hhmm: volume} (default 0)."""
    over = over or {}; vols = vols or {}
    out = []
    for t in (times or GRID):
        ov = over.get(t, (base, base, base, base))
        out.append(dict(ts=dt.datetime.combine(day, dt.time(t // 100, t % 100)), hhmm=t, day=day,
                        o=ov[0], h=ov[1], l=ov[2], c=ov[3], v=float(vols.get(t, 0))))
    return out


def d(i):  # i-th weekday starting Mon 2024-01-01
    x = dt.date(2024, 1, 1); n = 0
    while True:
        if x.weekday() < 5:
            if n == i: return x
            n += 1
        x += dt.timedelta(days=1)


def history(closes):
    out = []
    for i, c in enumerate(closes): out += mkday(d(i), c)
    return out


def entry_day(i, P=105.0, ih=115.0, il=95.0, extra=None):
    """day i: 09:15 bar spans il..ih, 14:50 bar closes P, all other bars flat at P."""
    ov = {915: (P, ih, il, P), 1450: (P, P, P, P)}
    ov.update(extra or {})
    return mkday(d(i), P, ov)


def trades(bars, variant="ibs_roc", **kw):
    p = dict(BASE); p.update(kw)
    return o.run(p, bars=bars, variant=variant)


class T(unittest.TestCase):
    # ---- up-close lookback
    def test_lookback2_signal(self):
        # closes 98,100 -> d=100 (> 98); DIFF[d]=2; P=105: P>100 and 5>2
        b = history([98, 100]) + entry_day(2) + mkday(d(3), 105.2)
        t = trades(b)
        self.assertEqual(len(t), 1)
        self.assertEqual((t[0]["entry_px"], t[0]["exit_px"], t[0]["exit_reason"]), (105.0, 105.2, "time"))

    def test_lookback2_no_upclose(self):
        b = history([100, 98]) + entry_day(2) + mkday(d(3), 105.2)
        self.assertEqual(trades(b), [])

    def test_lookback2_intradiff_must_exceed_diff(self):
        # DIFF[d]=2 ; P=102 -> P-C=2 not > 2 ; P=102.01 passes
        self.assertEqual(trades(history([98, 100]) + entry_day(2, P=102.0, ih=110, il=95) + mkday(d(3), 99)), [])
        self.assertEqual(len(trades(history([98, 100]) + entry_day(2, P=102.01, ih=110, il=95) + mkday(d(3), 99))), 1)

    def test_lookback3(self):
        # closes 96,98,100: up,up ; DIFF 2,2 -> DIFF[d]>DIFF[d-1] false -> no trade for days=3
        b = history([96, 98, 100]) + entry_day(3) + mkday(d(4), 105.2)
        self.assertEqual(trades(b, days=3), [])
        self.assertEqual(len(trades(b, days=2)), 1)           # days=2 only needs the last close up
        # closes 96,98,101: DIFF 2,3 -> ok ; P=106: 5>3
        b = history([96, 98, 101]) + entry_day(3, P=106, ih=116) + mkday(d(4), 105.2)
        self.assertEqual(len(trades(b, days=3)), 1)
        # first close not up -> days=3 fails although the last close is up
        b = history([99, 98, 101]) + entry_day(3, P=106, ih=116) + mkday(d(4), 105.2)
        self.assertEqual(trades(b, days=3), [])

    # ---- IBS
    def test_ibs_threshold(self):
        # IH=115, IL=95, P=105 -> IBS = 10/20 = 0.5 ; <= is inclusive
        b = history([98, 100]) + entry_day(2) + mkday(d(3), 105.2)
        self.assertEqual(len(trades(b, ibs_thr=0.5)), 1)
        self.assertEqual(trades(b, ibs_thr=0.49), [])
        # high AFTER the entry bar must not count
        b2 = history([98, 100]) + entry_day(2, extra={1455: (105, 130, 105, 105)}) + mkday(d(3), 105.2)
        self.assertEqual(len(trades(b2, ibs_thr=0.5)), 1)

    def test_ibs_flat_day_no_trade(self):
        # IH == IL on the entry day -> 0/0 -> no trade (and P=105 flat day)
        b = history([98, 100]) + mkday(d(2), 105.0) + mkday(d(3), 105.2)
        self.assertEqual(trades(b), [])

    # ---- ROC base day: ROCDAYS=2 -> C[D-1]=100 (5%), ROCDAYS=3 -> C[D-2]=98 (7.1428%)
    def test_roc_base(self):
        b = history([98, 100]) + entry_day(2) + mkday(d(3), 105.2)
        self.assertEqual(len(trades(b, roc_days=2, roc_thr=4.99)), 1)
        self.assertEqual(trades(b, roc_days=2, roc_thr=5.01), [])
        self.assertEqual(trades(b, roc_days=2, roc_thr=7.0), [])
        self.assertEqual(len(trades(b, roc_days=3, roc_thr=7.0)), 1)
        self.assertEqual(trades(b, roc_days=3, roc_thr=7.2), [])

    # ---- exits (entry P=105; pb 1% -> 106.05 ; hs 1% -> 103.95)
    def _exit(self, over_next, **kw):
        b = history([98, 100]) + entry_day(2) + mkday(d(3), 105.2, over_next)
        t = trades(b, **kw)
        self.assertEqual(len(t), 1)
        return t[0]

    def test_profit_booking_at_pb(self):
        t = self._exit({920: (105.5, 107.0, 105.5, 106)}, pb_pct=1.0)
        self.assertEqual(t["exit_reason"], "pb")
        self.assertAlmostEqual(t["exit_px"], 106.05, 9)
        self.assertEqual(t["exit_ts"], "%s 09:20:00" % d(3))

    def test_profit_booking_gap_open(self):
        t = self._exit({915: (107.0, 107.5, 106.5, 107.2)}, pb_pct=1.0)
        self.assertAlmostEqual(t["exit_px"], 107.0 - 107.0 * 0.25 / 100, 9)       # 106.7325
        self.assertEqual(t["exit_ts"], "%s 09:15:00" % d(3))

    def test_pb_exactly_at_level_not_hit(self):
        # H > pb is strict: H == 106.05 does not exit early
        t = self._exit({920: (105.5, 106.05, 105.5, 106)}, pb_pct=1.0)
        self.assertEqual(t["exit_reason"], "time")

    def test_hard_stop(self):
        t = self._exit({920: (104.5, 104.6, 103.0, 103.5)}, hardstop=1.0)
        self.assertEqual(t["exit_reason"], "hs")
        self.assertAlmostEqual(t["exit_px"], 105.0 - 1.05, 9)                        # 103.95

    def test_fill_clamped_to_bar_range(self):
        # pb gap: O 107, L 106.9 -> 107 - 0.2675 = 106.7325 < L -> filled at L 106.9
        t = self._exit({915: (107.0, 107.5, 106.9, 107.2)}, pb_pct=1.0)
        self.assertEqual((t["exit_reason"], t["exit_px"]), ("pb", 106.9))
        # stop gap: O 103, L 102.9 -> 102.7425 < L -> 102.9
        t = self._exit({915: (103.0, 103.5, 102.9, 103.0)}, hardstop=1.0)
        self.assertEqual((t["exit_reason"], t["exit_px"]), ("hs", 102.9))

    def test_hard_stop_gap(self):
        t = self._exit({915: (103.0, 103.5, 102.5, 103.0)}, hardstop=1.0)
        self.assertAlmostEqual(t["exit_px"], 103.0 - 103.0 * 0.25 / 100, 9)          # 102.7425

    def test_time_exit_next_day_and_exit_time_param(self):
        t = self._exit({925: (105.2, 105.3, 105.1, 105.25), 930: (105.4, 105.6, 105.4, 105.5)})
        self.assertEqual((t["exit_ts"], t["exit_px"]), ("%s 09:25:00" % d(3), 105.25))
        t = self._exit({925: (105.2, 105.3, 105.1, 105.25), 930: (105.4, 105.6, 105.4, 105.5)}, exit_time=930)
        self.assertEqual((t["exit_ts"], t["exit_px"]), ("%s 09:30:00" % d(3), 105.5))
        self.assertEqual(t["exit_date"], str(d(3)))                                  # never on the entry day

    def test_time_exit_skips_missing_bar(self):
        # no 09:25 bar next day -> first available bar >= 09:25
        b = history([98, 100]) + entry_day(2) + mkday(d(3), 105.2, {930: (105.2, 105.7, 105.2, 105.7)},
                                                       times=[t for t in GRID if t != 925])
        t = trades(b)
        self.assertEqual((t[0]["exit_ts"], t[0]["exit_px"]), ("%s 09:30:00" % d(3), 105.7))

    def test_entry_bar_same_bar_exit(self):
        # entry bar itself has H=106.1 > pb=106.05 -> exit on the same bar at pb (AllowSameBarExit)
        b = history([98, 100]) + entry_day(2, extra={1450: (104.0, 106.1, 104.0, 105.0)}) + mkday(d(3), 105.2)
        t = trades(b, pb_pct=1.0)
        self.assertEqual(len(t), 1)
        self.assertEqual(t[0]["exit_ts"], t[0]["entry_ts"])
        self.assertAlmostEqual(t[0]["exit_px"], 106.05, 9)

    def test_same_day_pb_after_entry(self):
        b = history([98, 100]) + entry_day(2, extra={1500: (105, 106.5, 105, 106)}) + mkday(d(3), 105.2)
        t = trades(b, pb_pct=1.0)
        self.assertEqual((t[0]["exit_date"], t[0]["exit_ts"][11:16], t[0]["exit_reason"]), (str(d(2)), "15:00", "pb"))

    # ---- one position at a time
    def test_one_position_at_a_time(self):
        # day 3 only has bars from 14:50 on, so the position opened on day 2 has not met an exit bar
        # (hhmm in [09:25,14:50)) when day 3's 14:50 signal fires -> ignored; exit happens day 4 09:25
        d3 = mkday(d(3), 111.0, {1450: (111, 112, 110, 111)}, times=[1450, 1455])
        b = history([98, 100]) + entry_day(2) + d3 + mkday(d(4), 111.5)
        t = trades(b, roc_days=2)
        self.assertEqual(len(t), 1)
        self.assertEqual(t[0]["exit_date"], str(d(4)))
        # control: with a full day 3 the position is closed on d3 09:25 and d3's own signal trades again
        d3f = mkday(d(3), 111.0, {915: (111, 125, 100, 111), 1450: (111, 111, 111, 111)})
        b = history([98, 100]) + entry_day(2) + d3f + mkday(d(4), 111.5)
        self.assertEqual(len(trades(b)), 2)

    # ---- supertrend (hand calc, period 2, factor 1, mid-point)
    def test_supertrend_by_hand(self):
        bars = [dict(h=10, l=8, c=9), dict(h=11, l=9, c=10.5), dict(h=12, l=10, c=11.8),
                dict(h=9, l=5, c=6), dict(h=8, l=6, c=7), dict(h=13, l=9, c=13)]
        self.assertEqual(o.supertrend(bars, 2, 1, 1), [1, 1, 1, -1, -1, 1])

    def test_sma_filter(self):
        bars = [dict(c=x) for x in (1, 2, 3, 4, 5)]
        self.assertEqual(o.sma_filter_series(bars, 2), [False, False, True, True, True])

    def test_weekly_monthly(self):
        days = [dict(date=dt.date(2024, 1, 5), o=1, h=5, l=1, c=4), dict(date=dt.date(2024, 1, 8), o=4, h=9, l=3, c=8),
                dict(date=dt.date(2024, 1, 9), o=8, h=10, l=7, c=9), dict(date=dt.date(2024, 2, 1), o=9, h=9, l=2, c=2)]
        w = o.weekly_bars(days)
        self.assertEqual([(x["o"], x["h"], x["l"], x["c"]) for x in w], [(1, 5, 1, 4), (4, 10, 3, 9), (9, 9, 2, 2)])
        m = o.monthly_bars(days)
        self.assertEqual([(x["o"], x["h"], x["l"], x["c"]) for x in m], [(1, 10, 1, 9), (9, 9, 2, 2)])

    def test_filter_uses_last_completed_bar(self):
        # SMA(2) daily filter: day2 sees C[1]=2? build closes 10,10,10,12 then entry on day 4.
        # visible on day 4: C[3]=12 > MA(2)[2]=10 -> True ; closes 10,10,10,9 -> 9>10 False
        up = history([10, 10, 10, 12]) + entry_day(4, P=15.0, ih=16, il=9) + mkday(d(5), 14.2)
        self.assertEqual(len(trades(up, flt_type=2, sma_period=2)), 1)
        # closes 20,20,8,9: up-close ok (9>8) but 9 < MA(2)[2]=14 -> filtered ; filter OFF trades
        dn = history([20, 20, 8, 9]) + entry_day(4, P=15.0, ih=16, il=9) + mkday(d(5), 14.2)
        self.assertEqual(len(trades(dn)), 1)
        self.assertEqual(trades(dn, flt_type=2, sma_period=2), [])


class Variants(unittest.TestCase):
    """Phase-1 sub-variants (hand-calculated).  BASE timing: lookback 2, 14:50 -> 09:25, pb 0, stop 0, filter off."""

    def test_spec_defaults(self):
        p = o.spec_params("core")
        self.assertEqual((p["days"], p["entry_time"], p["exit_time"], p["pb_pct"], p["hardstop"], p["flt_type"]),
                         (2, 1450, 925, 0.0, 0.0, 3))
        self.assertEqual((p["ibs_thr"], p["roc_days"], p["roc_thr"], p["ibs_dir"], p["high_days"], p["green_mode"],
                          p["green_days"], p["vol_days"], p["vol_mult"], p["athc_pct"], p["lookahead"]),
                         (0.5, 2, 1.0, 1, 3, 1, 2, 5, 1.0, 10.0, 0))

    def test_core_ignores_ibs_roc(self):
        # IBS 0.5 and ROC 5%: with ibs_thr 0.4 / roc_thr 6 the ibs_roc variant drops it, core keeps it
        b = history([98, 100]) + entry_day(2) + mkday(d(3), 105.2)
        self.assertEqual(trades(b, ibs_thr=0.4), [])
        self.assertEqual(trades(b, roc_thr=6.0), [])
        self.assertEqual(len(trades(b, "core", ibs_thr=0.4, roc_thr=6.0)), 1)

    # ---- ibs_dir 2: ibs >= level (earlier AFL)
    def test_ibs_dir2(self):
        b = history([98, 100]) + entry_day(2) + mkday(d(3), 105.2)          # IBS = 0.5
        self.assertEqual(len(trades(b, ibs_dir=2, ibs_thr=0.5)), 1)          # >= inclusive
        self.assertEqual(trades(b, ibs_dir=2, ibs_thr=0.51), [])
        self.assertEqual(len(trades(b, ibs_dir=2, ibs_thr=0.2)), 1)
        self.assertEqual(trades(b, ibs_dir=1, ibs_thr=0.2), [])

    # ---- ATH close: P > max(all daily closes up to the last completed day, inclusive), strict
    def test_ath(self):
        b = history([110, 98, 100]) + entry_day(3, P=105) + mkday(d(4), 105.2)
        self.assertEqual(len(trades(b, "core")), 1)
        self.assertEqual(trades(b, "ath"), [])                              # ATH 110
        b = history([110, 98, 100]) + entry_day(3, P=110, ih=120) + mkday(d(4), 105.2)
        self.assertEqual(trades(b, "ath"), [])                              # 110 > 110 false
        b = history([110, 98, 100]) + entry_day(3, P=110.5, ih=120) + mkday(d(4), 105.2)
        self.assertEqual(len(trades(b, "ath")), 1)
        # today's own high above P does not matter (close-based, completed days only)
        b = history([98, 100]) + entry_day(2, P=105, ih=130) + mkday(d(3), 105.2)
        self.assertEqual(len(trades(b, "ath")), 1)

    def test_ath_only(self):
        # no up-close rules: closes 100, 98 (a down close) still trade when P > ATH 100
        b = history([100, 98]) + entry_day(2, P=105) + mkday(d(3), 105.2)
        self.assertEqual(trades(b, "core"), [])
        self.assertEqual(len(trades(b, "ath_only")), 1)
        # the last completed day's close counts: closes 98, 106 -> ATH 106 > P 105
        # (history day 1 itself trades in the first case: its flat bars 106 > ATH 98; only day 2 is checked)
        on2 = lambda b: [t for t in trades(b, "ath_only") if t["signal_date"] == str(d(2))]
        self.assertEqual(on2(history([98, 106]) + entry_day(2, P=105) + mkday(d(3), 105.2)), [])
        self.assertEqual(on2(history([106, 98]) + entry_day(2, P=105) + mkday(d(3), 105.2)), [])
        self.assertEqual(len(on2(history([98, 104]) + entry_day(2, P=105) + mkday(d(3), 105.2))), 1)

    # ---- highest close of N days: P > max(C[d-N+1..d]), strict
    def test_high_n(self):
        b = history([110, 98, 100]) + entry_day(3, P=105) + mkday(d(4), 105.2)
        self.assertEqual(trades(b, "high_n", high_days=3), [])              # 110 in window
        self.assertEqual(len(trades(b, "high_n", high_days=2)), 1)          # window 98, 100
        b = history([110, 97, 98, 100]) + entry_day(4, P=105) + mkday(d(5), 105.2)
        self.assertEqual(len(trades(b, "high_n", high_days=3)), 1)
        self.assertEqual(trades(b, "high_n", high_days=4), [])
        b = history([104, 98, 100]) + entry_day(3, P=104, ih=115) + mkday(d(4), 105.2)
        self.assertEqual(trades(b, "high_n", high_days=3), [])              # 104 > 104 false
        b = history([104, 98, 100]) + entry_day(3, P=104.01, ih=115) + mkday(d(4), 105.2)
        self.assertEqual(len(trades(b, "high_n", high_days=3)), 1)

    # ---- green candle: P > today's first-bar open AND the last green_days-1 daily candles C > O
    @staticmethod
    def _gday(i, op, cl):
        return mkday(d(i), cl, {915: (op, max(op, cl), min(op, cl), cl)})

    def test_green(self):
        today_green = {915: (100, 115, 95, 105)}                             # open 100 < P 105
        today_red = {915: (106, 115, 95, 105)}                               # open 106 > P 105
        hist = self._gday(0, 98, 98) + self._gday(1, 99, 100)                # day0 flat, day1 green
        g = lambda ov: hist + entry_day(2, extra=ov) + mkday(d(3), 105.2)
        self.assertEqual(len(trades(g(today_green), "green")), 1)
        self.assertEqual(trades(g(today_red), "green"), [])
        self.assertEqual(len(trades(g(today_red), "green", green_mode=2)), 1)  # ANY
        self.assertEqual(trades(g(today_green), "green", green_days=3), [])    # day0 flat (C == O is not green)
        hist2 = self._gday(0, 97, 98) + self._gday(1, 99, 100)
        b = hist2 + entry_day(2, extra=today_green) + mkday(d(3), 105.2)
        self.assertEqual(len(trades(b, "green", green_days=3)), 1)
        # last completed day red -> no trade
        hist3 = self._gday(0, 97, 98) + self._gday(1, 101, 100)
        b = hist3 + entry_day(2, extra=today_green) + mkday(d(3), 105.2)
        self.assertEqual(trades(b, "green"), [])

    # ---- volume: sum V from after the last bar with hhmm >= entry_time up to the entry bar (inclusive)
    #      > vol_mult x MA(daily V, vol_days) of completed days, strict
    def _vol(self, v1450, day2=None, **kw):
        h = mkday(d(0), 90) + mkday(d(1), 98, vols={1000: 100}) + (day2 or mkday(d(2), 100, vols={1000: 100}))
        e = mkday(d(2 + 1), 105, {915: (105, 115, 95, 105)}, vols={915: 60, 1450: v1450, 1455: 1000})
        kw.setdefault("vol_days", 2)
        return trades(h + e + mkday(d(4), 105.2), "volume", **kw)

    def test_volume(self):
        self.assertEqual(self._vol(40), [])                                  # 60+40 = 100, MA 100: not >
        self.assertEqual(len(self._vol(41)), 1)                              # 101 > 100 (14:55 bar not counted)
        self.assertEqual(self._vol(41, vol_mult=2.0), [])                    # 101 > 200 false
        self.assertEqual(len(self._vol(141, vol_mult=2.0)), 1)               # 201 > 200
        # vol_days 3 -> MA of days 0..2 = (0+100+100)/3 = 66.67 -> 100 passes
        self.assertEqual(len(self._vol(40, vol_days=3)), 1)
        # vol_days 4 > completed days 3 -> MA Null -> no trade
        self.assertEqual(trades(
            mkday(d(0), 90) + mkday(d(1), 98) + mkday(d(2), 100)
            + mkday(d(3), 105, {915: (105, 115, 95, 105)}, vols={915: 60, 1450: 40}) + mkday(d(4), 105.2),
            "volume", vol_days=4), [])

    def test_volume_literal_reset(self):
        # AFL loop resets only on bars with timenow >= entrytime: day 2 ends at 14:00, so its 100 is still
        # in the running total at day 3's 14:50 bar: 100 + 60 + 40 = 200 > MA(2) = 100 -> trade
        short = mkday(d(2), 100, vols={1000: 100}, times=[t for t in GRID if t <= 1400])
        self.assertEqual(len(self._vol(40, day2=short)), 1)

    # ---- ATHC swing
    def test_athc_buysig_by_hand(self):
        bs, tr = o.athc_buysig([100, 105, 103, 94, 96, 106], 10)
        self.assertEqual(tr, [1, 1, 1, -1, -1, 1])
        self.assertEqual(bs, [0, 1, 1, 0, 0, 1])

    def test_athc_variant(self):
        # closes 100,105,94,96: pct 10 -> 94 < Dn 94.5 -> trend -1 -> BUYSIG 0 at d=3 -> no trade
        b = history([100, 105, 94, 96]) + entry_day(4, P=105) + mkday(d(5), 105.2)
        self.assertEqual(len(trades(b, "core")), 1)
        self.assertEqual(trades(b, "athc", athc_pct=10.0), [])
        # pct 20 -> Dn 84, trend stays 1, BUYSIG stays 1 (set on 105 > 100)
        self.assertEqual(len(trades(b, "athc", athc_pct=20.0)), 1)
        # the ST/SMA filter is not used by the ATHC AFL: SMA(2) would block (96 < 99.5), athc still trades
        self.assertEqual(trades(b, "core", flt_type=2, sma_period=2), [])
        self.assertEqual(len(trades(b, "athc", athc_pct=20.0, flt_type=2, sma_period=2)), 1)
        # BUYSIG never set (no close above the previous ATH close) -> no trade
        b = history([110, 98, 100]) + entry_day(3, P=105) + mkday(d(4), 105.2)
        self.assertEqual(trades(b, "athc", athc_pct=20.0), [])

    # ---- old fixed AFL (ver2_updated)
    def _old(self, P, Cf, closes=(100, 101, 103), la=0):
        h = history(list(closes))
        e = mkday(d(3), P, {1515: (P, P, P, P), 1525: (Cf, Cf, Cf, Cf)})
        nx = mkday(d(4), 107, {915: (107, 107, 107, 106.5), 920: (107, 108, 107, 107.5)})
        return o.run(o.spec_params("old_fixed", lookahead=la), bars=h + e + nx, variant="old_fixed")

    def test_old_fixed(self):
        # diffs 1, 2 ; no lookahead: C1 = 15:15 price 105 -> 105-103 = 2 >= 2 >= 1 (non-strict) -> trade
        t = self._old(105, 104)
        self.assertEqual(len(t), 1)
        self.assertEqual((t[0]["entry_ts"][11:16], t[0]["entry_px"]), ("15:15", 105))
        self.assertEqual((t[0]["exit_date"], t[0]["exit_ts"][11:16], t[0]["exit_px"]), (str(d(4)), "09:20", 107.5))
        # literal: C1 = day's final close 104 -> 1 >= 2 false
        self.assertEqual(self._old(105, 104, la=1), [])
        self.assertEqual(self._old(104, 106), [])
        self.assertEqual(len(self._old(104, 106, la=1)), 1)
        # ties of the earlier diffs pass (>=): closes 100,101,102 diffs 1,1 ; P 103 -> 1 >= 1 >= 1
        self.assertEqual(len(self._old(103, 103, closes=(100, 101, 102))), 1)
        # but the closes must rise strictly
        self.assertEqual(self._old(103, 103, closes=(100, 101, 101)), [])

    # ---- IEOD reader keeps volumes
    def test_ieod_volume(self):
        import struct, tempfile
        def u(y, mo, dd, hh, mi):
            return ((y << 20) | (mo << 16) | (dd << 11) | (hh << 6) | mi) << 32
        recs = b""
        for i, mi in enumerate(range(15, 22)):          # 09:15..09:21, v = 1..7
            c = 100.0 + i
            recs += struct.pack("<Q8f", u(2024, 1, 2, 9, mi), c, c - 0.5, c + 1, c - 1, i + 1, 0, 0, 0)
        recs += struct.pack("<Q8f", u(2024, 1, 3, 9, 15), 200, 199, 201, 198, 50, 0, 0, 0)
        with tempfile.TemporaryDirectory() as td:
            pth = os.path.join(td, "X")
            with open(pth, "wb") as fh:
                fh.write(b"BROKDAt5" + b"\0" * 1176 + recs + b"\0" * 4)
            bars, _ = o.build_5min_ieod(pth)
        self.assertEqual([(b["hhmm"], b["v"], b["o"], b["c"]) for b in bars],
                         [(915, 15.0, 99.5, 104.0), (920, 13.0, 104.5, 106.0), (915, 50.0, 199.0, 200.0)])
        dl = o.daily_bars(bars)
        self.assertEqual([(x["v"], x["o"], x["h"], x["l"], x["c"]) for x in dl],
                         [(28.0, 99.5, 107.0, 99.0, 106.0), (50.0, 199.0, 201.0, 198.0, 200.0)])


class E34(unittest.TestCase):
    """E3 second-last candle and E4 candle-count (hand-calculated; GRID = 09:15..15:25, 75 candles a day)."""
    NF = (100.0, 101.0, 99.0, 100.5)           # a candle that is neither up- nor low-circuit

    @staticmethod
    def _day(i, base=100.0, over=None, times=None):
        ov = {t: (base, base + 1, base - 1, base + 0.5) for t in (times or GRID)}   # every candle "normal"
        ov.update(over or {})
        return mkday(d(i), base, ov, times=times)

    @staticmethod
    def slc(bars, variant="slc", **kw):
        return o.run_e34(kw, bars=bars, variant=variant)

    def test_expand_index(self):
        b = mkday(d(0), 1, times=[915, 920]) + mkday(d(1), 1, times=[915, 920]) + mkday(d(4), 1, times=[915, 920]) \
            + mkday(d(5), 1, times=[915, 920])          # d(4) = Fri 2024-01-05, d(5) = Mon 2024-01-08
        idx, _ = o.expand_index(b, 1)
        self.assertEqual(idx, [0, 0, 0, 1, 1, 2, 2, 3])  # first day's leading bars see day 0; a day's last bar sees itself
        idx, w = o.expand_index(b, 2)
        self.assertEqual(len(w), 2)
        self.assertEqual(idx, [0, 0, 0, 0, 0, 0, 0, 1])  # week 0 everywhere until the data's last bar (week 1's last)

    def test_resample15(self):
        b = mkday(d(0), 100, {955: (100, 102, 99, 101), 1000: (101, 103, 100, 102), 1005: (102, 104, 98, 103),
                              1010: (103, 103, 103, 103)}, times=[955, 1000, 1005, 1010], vols={955: 1, 1000: 2, 1005: 3, 1010: 4})
        r = o.resample(b, 15)
        self.assertEqual([(x["hhmm"], x["o"], x["h"], x["l"], x["c"], x["v"]) for x in r],
                         [(945, 100, 102, 99, 101, 1.0), (1000, 101, 104, 98, 103, 9.0)])   # 09:55 start -> 09:45 candle

    # ---- E3
    def test_slc_basic(self):
        b = self._day(0, over={1520: (100, 101, 99, 100.5)}) + self._day(1, over={915: (101, 102, 100, 101.5)}) \
            + self._day(2)
        t = self.slc(b)
        self.assertEqual(len(t), 2)                     # day 2's buy is still open at the end: not a trade
        self.assertEqual((t[0]["entry_ts"][11:16], t[0]["entry_px"], t[0]["exit_ts"][11:16], t[0]["exit_px"]),
                         ("15:20", 100.5, "09:15", 101.5))
        self.assertEqual((t[1]["signal_date"], t[1]["entry_ts"][11:16]), (str(d(1)), "15:20"))

    def test_slc_upcircuit_blocks_buy(self):
        b = self._day(0, over={1520: (100, 100, 99, 100)}) + self._day(1) + self._day(2)   # O == C == H
        t = self.slc(b)
        self.assertEqual([x["signal_date"] for x in t], [str(d(1))])

    def test_slc_lowcircuit_delays_sell(self):
        b = self._day(0) + self._day(1, over={915: (100, 101, 100, 100)}) + self._day(2, over={915: (99, 100, 98, 99.5)}) \
            + self._day(3)
        t = self.slc(b)
        self.assertEqual((t[0]["exit_date"], t[0]["exit_px"]), (str(d(2)), 99.5))   # day 1 first candle O == C == L
        self.assertEqual(t[1]["signal_date"], str(d(2)))                             # no buy on day 1 (still holding)

    def test_slc_two_candle_day(self):
        # day 1 has 2 candles: its first candle sells; it is also the 2nd-last candle but the position was open
        b = self._day(0) + self._day(1, times=[915, 920]) + self._day(2) + self._day(3)
        t = self.slc(b)
        self.assertEqual([(x["signal_date"], x["exit_date"]) for x in t], [(str(d(0)), str(d(1))), (str(d(2)), str(d(3)))])
        # flat on a 2-candle day: buy and sell on the same candle cancel -> no trade, still flat
        b = self._day(0, times=[915, 920]) + self._day(1) + self._day(2)
        self.assertEqual([x["signal_date"] for x in self.slc(b)], [str(d(1))])

    def test_slc_bull_bear_abc(self):
        b = sum((self._day(i, base=100.0 + i) for i in range(5)), [])
        self.assertEqual(len(self.slc(b)), 4)
        # daily ST period 2: band Null for day 0 -> day 1's 2nd-last candle (sees day 0) cannot buy
        bull = self.slc(b, "slc_bull", flt_tf=1, st_period=2, st_factor=1)
        self.assertEqual([x["signal_date"] for x in bull], [str(d(2)), str(d(3))])
        self.assertEqual(self.slc(b, "slc_bear", flt_tf=1, st_period=2, st_factor=1), [])
        # band <= 0 (huge factor) -> abc false -> no trade
        self.assertEqual(self.slc(b, "slc_bull", flt_tf=1, st_period=2, st_factor=1000), [])

    # ---- E4
    @staticmethod
    def cc(bars, **kw):
        p = dict(pt_pct=0, flt_type=3); p.update(kw)
        return o.run_e34(p, bars=bars, variant="candles")

    def test_candles_entry_exit_count(self):
        b = self._day(0, over={1525: (100, 101, 99, 100.25), 1510: (100, 101, 99, 100.75)}) \
            + self._day(1, over={915: (100, 101, 99, 100.1), 925: (100, 101, 99, 100.3)}) + self._day(2)
        t = self.cc(b)                                                    # entry 0 = last candle, exit 1 = first candle
        self.assertEqual((t[0]["entry_ts"][11:16], t[0]["entry_px"], t[0]["exit_ts"][11:16], t[0]["exit_px"]),
                         ("15:25", 100.25, "09:15", 100.1))
        t = self.cc(b, entry_candle=3, exit_candle=3)
        self.assertEqual((t[0]["entry_ts"][11:16], t[0]["entry_px"], t[0]["exit_ts"][11:16], t[0]["exit_px"]),
                         ("15:10", 100.75, "09:25", 100.3))

    def test_candles_count_is_candles_not_days(self):
        # exit 3 with a 2-candle next day: count 1, 2 on day 1, 3 on day 2's first candle
        b = self._day(0) + self._day(1, times=[915, 920]) + self._day(2, over={915: (100, 101, 99, 100.2)})
        t = self.cc(b, exit_candle=3)
        self.assertEqual((t[0]["exit_date"], t[0]["exit_ts"][11:16], t[0]["exit_px"]), (str(d(2)), "09:15", 100.2))

    def test_candles_profit_target(self):
        e = {1525: (100, 100.5, 99.5, 100)}                             # buy 100, PT 1% = 101
        t = self.cc(self._day(0, over=e) + self._day(1, over={915: (100.5, 101, 100, 100.8)}), pt_pct=1)
        self.assertEqual((t[0]["exit_px"], t[0]["exit_reason"]), (101.0, "pt"))   # H >= PT (non-strict), O < PT
        t = self.cc(self._day(0, over=e) + self._day(1, over={915: (102, 103, 101.5, 102.5)}), pt_pct=1)
        self.assertAlmostEqual(t[0]["exit_px"], 102 * (1 - 0.0025), 9)            # gap above PT: O - 0.25%
        t = self.cc(self._day(0, over=e) + self._day(1, over={915: (102, 103, 102, 102.5)}), pt_pct=1)
        self.assertEqual(t[0]["exit_px"], 102)                                       # 101.745 < Low 102 -> filled at Low
        # PT is live on the entry day after the entry bar (not on the entry bar itself)
        t = self.cc(self._day(0, over={1510: (100, 105, 99, 100), 1515: (100, 101.5, 99, 101)}) + self._day(1),
                    entry_candle=3, pt_pct=1)
        self.assertEqual((t[0]["exit_date"], t[0]["exit_ts"][11:16], t[0]["exit_px"]), (str(d(0)), "15:15", 101.0))

    def test_candles_trend_expand_last(self):
        # daily ST period 2 factor 1: days 0-2 H 101 L 99 C 100 -> Up 102 / Dn 98; day 3 closes 90 < 98 -> trend -1,
        # visible only on day 3's LAST candle: buy at 15:10 (sees day 2, trend 1), sell at 15:25 @ 90 ("trend")
        flat = {915: (100, 101, 99, 100)}
        fl = lambda i, ov=None: mkday(d(i), 100.0, {**flat, **(ov or {})})  # noqa: E731
        b = fl(0) + fl(1) + fl(2) + fl(3, {1525: (100, 100, 90, 90)}) + fl(4)
        t = self.cc(b, flt_type=1, flt_tf=1, st_period=2, st_factor=1, entry_candle=3, exit_candle=1)
        t3 = [x for x in t if x["signal_date"] == str(d(3))]
        self.assertEqual((t3[0]["entry_ts"][11:16], t3[0]["exit_ts"][11:16], t3[0]["exit_px"], t3[0]["exit_reason"]),
                         ("15:10", "15:25", 90, "trend"))
        # the data's first day trades (leading bars see the first day's trend, Mtrend[0] = 1)
        self.assertEqual(t[0]["signal_date"], str(d(0)))
        # filter off: no trend exit, sells next day's first candle
        t = self.cc(b, entry_candle=3, exit_candle=1)
        t3 = [x for x in t if x["signal_date"] == str(d(3))]
        self.assertEqual((t3[0]["exit_date"], t3[0]["exit_reason"]), (str(d(4)), "time"))


if __name__ == "__main__":
    unittest.main()
