"""Tests for btst_stock_series.py (run: python -I -m pytest test_btst_stock_series.py, or python -I this file)."""
import os, struct, sys, tempfile

import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import btst_stock_series as S  # noqa: E402
import oracle_btst as O  # noqa: E402

DB = S.DB


def _fake_db(tmp):
    """A tiny AmiBroker native file: 2 days, minutes incl. 09:55, an evening minute and a padded copy."""
    def pack(y, m, d, hh, mi):
        hi = (y << 20) | (m << 16) | (d << 11) | (hh << 6) | mi
        return hi << 32
    recs = [((2008, 1, 2, 9, 55), 10, 10, 11, 9, 100), ((2008, 1, 2, 9, 56), 10.5, 10, 10.6, 9.9, 50),
            ((2008, 1, 2, 9, 59), 10.4, 10.5, 10.5, 10.3, 7), ((2008, 1, 2, 10, 0), 10.2, 10.4, 10.4, 10.1, 1),
            ((2008, 1, 2, 23, 59), 10.2, 10.2, 10.2, 10.2, 1),
            ((2008, 1, 3, 9, 55), 12, 11, 12.5, 11, 3)]
    dt = np.dtype([("u", "<u8"), ("c", "<f4"), ("o", "<f4"), ("h", "<f4"), ("l", "<f4"),
                   ("v", "<f4"), ("oi", "<f4"), ("a1", "<f4"), ("a2", "<f4")])
    a = np.zeros(len(recs), dt)
    for i, (t, c, o, h, l, v) in enumerate(recs):
        a[i] = (pack(*t), c, o, h, l, v, 0, 0, 0)
    p = os.path.join(tmp, "FAKE")
    with open(p, "wb") as f:
        f.write(b"BROKDAt5" + b"\0" * (1184 - 8)); f.write(a.tobytes()); f.write(b"\0" * 4)
    return p


def test_safe_name():
    assert S.safe_name("ABB 2007-13") == "ABB_2007-13"
    assert S.safe_name("M&M") == "M&M"
    assert S.safe_name("BAJAJ-AUTO") == "BAJAJ-AUTO"
    assert S.safe_name("S&P CNX NIFTY (INDEX)") == "S&P_CNX_NIFTY_%28INDEX%29"
    assert S.safe_name("A.B%") == "A%2EB%25"


def test_build_small():
    with tempfile.TemporaryDirectory() as tmp:
        days, bars = S.build_arrays(_fake_db(tmp))
        assert days["ymd"].tolist() == [20080102, 20080103]
        assert days["nb"].tolist() == [5, 1] and days["first"].tolist() == [0, 3]
        assert bars["hhmm"].tolist() == [955, 1000, 2355, 955]
        b = bars[0]
        assert (b["o"], b["h"], b["l"], b["c"], b["v"]) == (np.float32(10), np.float32(11), np.float32(9),
                                                             np.float32(10.4), np.float32(157))
        d = days[0]
        assert (d["o"], d["h"], d["l"], d["c"], d["v"]) == (np.float32(10), np.float32(11), np.float32(9),
                                                             np.float32(10.2), np.float32(159))


def test_round_trip():
    with tempfile.TemporaryDirectory() as tmp:
        days, bars = S.build_arrays(_fake_db(tmp))
        p = os.path.join(tmp, "x.bt5")
        S.write_bt5(p, "ABB 2007-13", days, bars)
        raw = open(p, "rb").read()
        assert raw[:8] == b"BT5STK01" and struct.unpack_from("<IIH", raw, 8) == (2, 4, 11)
        assert len(raw) == 18 + 11 + 2 * 32 + 4 * 22
        name, d2, b2 = S.read_bt5(p)
        assert name == "ABB 2007-13"
        assert d2.tobytes() == days.tobytes() and b2.tobytes() == bars.tobytes()


def test_real_symbol_matches_oracle_bars():
    """bt5 bars == oracle_btst.build_5min_ieod bars (rounded to f32) for a real stock, if the DB is present."""
    src = os.path.join(DB, "a", "ACC")
    if not os.path.exists(src):
        return
    days, bars = S.build_arrays(src)
    ob = O.build_5min_ieod(src)[0]
    mine = S.to_oracle_bars(days, bars)
    assert len(mine) == len(ob)
    f32 = lambda x: float(np.float32(x))
    for a, b in zip(mine[:: 97], ob[:: 97]):
        assert a["ts"] == b["ts"] and a["o"] == b["o"] and a["h"] == b["h"] and a["l"] == b["l"] and a["c"] == b["c"]
        assert a["v"] == f32(b["v"])
    od = O.daily_bars(ob)
    assert [d["c"] for d in od] == days["c"].astype(float).tolist()


if __name__ == "__main__":
    for k, f in list(globals().items()):
        if k.startswith("test_"):
            f(); print("ok", k)
