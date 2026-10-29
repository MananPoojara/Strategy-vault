"""Intraday wave 2a: independent reference for five Crabel open-to-close rules, written from the page images of
Day Trading with Short Term Price Patterns and Opening Range Breakout (1990), not from the Rust code.
- Inside Days in the S&P, printed p.115: Pattern 1 (ID, then sell a lower / buy a higher open), 2a/2b (ID with a higher close
  then a higher open = buy; lower close then lower open = sell), 3 (Day 1 open below Day 2's mid-range and close; Day 2 inside
  Day 3; Day 3 higher low than Day 4; sell), 4 (Day 1 open above Day 2's close; Day 2 inside Day 3; Day 3 lower high than
  Day 4; buy). Entry on Day 1's open, exit on Day 1's close, no stop. Higher/lower open = vs Day 2's close.
  Inside day read as strictly lower high and strictly higher low (the glossary has no entry).
- Table E, printed p.72 (S&P 1982-88): Day 2 = NR4 / NR / control / RE / RE4; pattern = Day 2 close vs Day 3 close, then Day 1
  open vs Day 2 close. Glossary p.285: NR = range smaller than the previous day's; NR4 = narrower than each of the previous
  three. RE / RE4 = the opposite (wider) [read by analogy; the glossary defines only the narrow forms].
Daily bars decide every trade completely (open, then close): no intraday order question."""
# rows in printed order: for each pattern, NR4, NR, CONTROL, RE, RE4 with the printed B/S
TABLE_E = [("++", "NR4", "B"), ("++", "NR", "B"), ("++", "CTL", "S"), ("++", "RE", "S"), ("++", "RE4", "S"),
           ("-+", "NR4", "S"), ("-+", "NR", "B"), ("-+", "CTL", "B"), ("-+", "RE", "B"), ("-+", "RE4", "B"),
           ("+-", "NR4", "B"), ("+-", "NR", "B"), ("+-", "CTL", "B"), ("+-", "RE", "S"), ("+-", "RE4", "B"),
           ("--", "NR4", "B"), ("--", "NR", "B"), ("--", "CTL", "B"), ("--", "RE", "S"), ("--", "RE4", "S")]


def sg(a, b):
    return "+" if a > b else "-" if a < b else "="


def backtest(id_, B, a, b, prm):
    O = [r[1] for r in B]; H = [r[2] for r in B]; L = [r[3] for r in B]; C = [r[4] for r in B]
    R = [h - l for h, l in zip(H, L)]
    out = []
    for i in range(max(a, 4), b + 1):
        d2, d3, d4 = i - 1, i - 2, i - 3
        ins = H[d2] < H[d3] and L[d2] > L[d3]
        side = None
        if id_ == "tcdt_idp1" and ins:
            side = {"+": 1, "-": -1}.get(sg(O[i], C[d2]))
        elif id_ == "tcdt_idp2" and ins:
            side = {("+", "+"): 1, ("-", "-"): -1}.get((sg(C[d2], C[d3]), sg(O[i], C[d2])))
        elif id_ == "tcdt_idp3" and ins and L[d3] > L[d4] and O[i] < (H[d2] + L[d2]) / 2 and O[i] < C[d2]:
            side = -1
        elif id_ == "tcdt_idp4" and ins and H[d3] < H[d4] and O[i] > C[d2]:
            side = 1
        elif id_ == "tcdt_oc_nr_re":
            pat, cond, bs = TABLE_E[int(prm.get("row", 1)) - 1]
            ok = {"CTL": True, "NR": R[d2] < R[d3], "RE": R[d2] > R[d3],
                  "NR4": all(R[d2] < R[d2 - k] for k in (1, 2, 3)), "RE4": all(R[d2] > R[d2 - k] for k in (1, 2, 3))}[cond]
            if ok and sg(C[d2], C[d3]) + sg(O[i], C[d2]) == pat:
                ov = float(prm.get("side", 0))
                side = 1 if ov > 0.5 else -1 if ov < -0.5 else (1 if bs == "B" else -1)
        if side:
            out.append(dict(dir=side, ei=i, epx=O[i], xi=i, xpx=C[i], open=False))
    return out


if __name__ == "__main__":  # hand check
    B = [("d0", 10, 12, 8, 11), ("d1", 11, 13, 9, 12), ("d2", 12, 14, 10, 13), ("d3", 13, 13.5, 10.5, 13.2), ("d4", 13.5, 14, 13, 13.8)]
    assert backtest("tcdt_idp1", B, 0, 4, {}) == [dict(dir=1, ei=4, epx=13.5, xi=4, xpx=13.8, open=False)]
    assert backtest("tcdt_idp2", B, 0, 4, {})[0]["dir"] == 1
    assert backtest("tcdt_idp4", B, 0, 4, {}) == []  # day 3 high 14 is not below day 4's 13
    print("ok")
