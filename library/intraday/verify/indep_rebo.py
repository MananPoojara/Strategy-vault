#!/usr/bin/env python3
"""K25 (3 Nov 2026): independent reference for tdperl_rebo on real 1-minute bars, written from Jason Perl, DeMark
Indicators (Bloomberg 2008) Ch 9 pp.157-160 (read again from the book text for this lane), not from the Rust code.
Reads the daily file and the cleaned 1-minute CSV through indep_ref's loaders.

Book (p.158): "apply TD REBO to daily price bars and use the current price bar's open as the base. First, multiply the
previous day's true range separately by 0.382 and by 0.618. Then to calculate upside levels, add those values to the
current day's opening price. To determine downside levels, subtract them from the current day's opening price."
p.157: TD REBO "attempts to identify that initial price break, to enable traders to participate in the move, until a
signal occurs in the opposite direction."
Rule as coded on the card (unchanged): buy stop at open + 0.382 x previous true range, sell stop at open - 0.382 x previous
true range, stop and reverse on the opposite break; true range = max(high, prior close) - min(low, prior close).
Minute path (this lane): the levels are standing stop orders for the whole day, so every touch of the level opposite to
the position reverses it (several reversals in one day are possible).
  - a level fills at the level, or at the minute's open when the minute opened through it (first touch in the minute);
  - one minute reaching both levels: the one it opened through came first, else the level nearer the minute's open came
    first [convention, counted as ambiguous]; then the other; then the first again only if the minute closed beyond it
    (the shortest path through the minute's open, both levels and close); later touches in that minute fill at the level;
  - no new position on the window's last bar (exits still happen there); an open position is marked at the last close;
  - a day without minute data (none loaded, excluded by the cleaner, NIFTY 50 from 2026-08-11) opens no position; a held
    position is closed at the adverse level if the daily bar reaches it (that exit happens whatever the order).
"""
import os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

K = 0.382


def touches(z, up, dn):
    """Ordered (level_name, fill) touches inside one minute z=(m,o,h,l,c), and whether the order is a convention."""
    m, o, h, l, c = z
    hu, hd = h >= up, l <= dn
    if hu and not hd:
        return [("up", max(o, up))], False
    if hd and not hu:
        return [("dn", min(o, dn))], False
    if not hu:
        return [], False
    thru_u, thru_d = o >= up, o <= dn
    if thru_u != thru_d:
        first, amb = ("up" if thru_u else "dn"), False
    else:
        first, amb = ("up" if abs(up - o) <= abs(o - dn) else "dn"), True
    lv = {"up": up, "dn": dn}
    other = "dn" if first == "up" else "up"
    seq = [(first, max(o, up) if first == "up" else min(o, dn)), (other, lv[other])]
    if (first == "up" and c >= up) or (first == "dn" and c <= dn):
        seq.append((first, lv[first]))
    return seq, amb


def backtest(D, M, a, b, amb_out=None, k=K):
    """D: [(date,o,h,l,c)], M.day(date) -> [(m,o,h,l,c)] or None. Trades on bars a..b."""
    out, cur = [], None  # cur = dict(dir, entry, entry_px)

    def close(i, px, why):
        nonlocal cur
        out.append(dict(cur, exit=D[i][0], exit_px=px, reason=why, open=False))
        cur = None

    for i in range(max(a, 2), b + 1):
        d, o, h, l, c = D[i]
        tr = max(D[i - 1][2], D[i - 2][4]) - min(D[i - 1][3], D[i - 2][4])
        up, dn = o + k * tr, o - k * tr
        bars = M.day(d)
        if bars is None:
            if cur and cur["dir"] > 0 and l <= dn:
                close(i, min(o, dn), "Opposite REBO break")
            elif cur and cur["dir"] < 0 and h >= up:
                close(i, max(o, up), "Opposite REBO break")
            continue
        done = False
        for z in bars:
            seq, amb = touches(z, up, dn)
            acted = False
            for name, px in seq:
                want = 1 if name == "up" else -1
                if cur and cur["dir"] == want:
                    continue
                if cur:
                    close(i, px, "Opposite REBO break")
                acted = True
                if i == b:
                    done = True
                    break
                cur = dict(dir=want, entry=d, entry_px=px)
            if acted and amb and amb_out is not None:
                amb_out.append((d, z[0]))
            if done:
                break
        if done:
            break
    if cur:
        out.append(dict(cur, exit=D[b][0], exit_px=D[b][4], reason="open", open=True))
    return out


if __name__ == "__main__":
    # self-check on a hand-made day: open 100, previous true range 10 -> levels 103.82 / 96.18
    class Mm:
        def __init__(self, days): self.days = days
        def day(self, d): return self.days.get(d)
    D = [("2020-01-01", 100, 105, 95, 100), ("2020-01-02", 100, 105, 95, 100), ("2020-01-03", 100, 110, 90, 100),
         ("2020-01-04", 100, 101, 99, 100)]
    M = Mm({"2020-01-03": [(570, 100, 104, 99, 103), (571, 103, 103.5, 96, 97), (572, 97, 98, 97, 98)],
            "2020-01-04": [(570, 100, 101, 99, 100)]})
    t = backtest(D, M, 0, 3)
    assert [(x["dir"], x["entry_px"], x["exit_px"], x["open"]) for x in t] == [(1, 103.82, 96.18, False), (-1, 96.18, 100, True)], t
    # both levels in one minute, open nearer the low: down first [convention], then up; it closed above up, not below down: no third touch
    s, amb = touches((0, 97, 105, 95, 104), 103.82, 96.18)
    assert amb and [n for n, _ in s] == ["dn", "up"], s
    s, amb = touches((0, 104, 105, 95, 97), 103.82, 96.18)
    assert not amb and s == [("up", 104), ("dn", 96.18)], s
    print("ok")
