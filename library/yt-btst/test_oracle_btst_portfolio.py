"""Hand-calculated tests of the BTST portfolio rules (oracle_btst_portfolio.simulate; findings section 2).
Run: python -I -m unittest discover -s library/yt-btst"""
import os, sys, unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import oracle_btst_portfolio as P  # noqa: E402


def row(sym, ets, epx, xts, xpx, **kw):
    r = dict(symbol=sym, entry_ts=ets, entry_px=epx, exit_ts=xts, exit_px=xpx, ibs=None, roc_a=None, roc_b=None,
             lbc=0)
    r.update(kw)
    return r


D1, D2, D3 = "2007-01-05 14:50:00", "2007-01-08 09:55:00", "2007-01-08 14:50:00"


class Sizing(unittest.TestCase):
    def test_first_team_trade(self):
        # team list price H->L 10 open: RELCAPITAL 656.6 -> 152 shares (floor(100000 / 656.6) = 152)
        t = P.simulate([row("RELCAPITAL 2007-19", D1, 656.6, D2, 657.6)], 10, "price_hl")
        self.assertEqual(t[0]["shares"], 152)
        self.assertAlmostEqual(t[0]["entry_value"], 99803.2)
        self.assertAlmostEqual(t[0]["weight"], 0.0998032)

    def test_compounding_after_exit(self):
        # day 1: 1 share-sized position, exit +10 % -> equity grows; day 2 sizes on the new equity
        rows = [row("A", D1, 100.0, D2, 110.0), row("A", D3, 100.0, "2007-01-09 09:55:00", 100.0)]
        t = P.simulate(rows, 1, "price_hl", pct=100, equity0=1000.0, comm=0.0)
        self.assertEqual([x["shares"] for x in t], [10, 11])      # 1000/100 = 10 ; 1100/100 = 11

    def test_shrink_last_position(self):
        # equity 1000, 2 slots of 50 %, commission 1 %: first 50 sh x 10 = 500 + 5 = 505 -> cash 495;
        # second wants 50 sh (505) > 495 -> AllowPositionShrinking: floor(495 / 10.1) = 49
        rows = [row("A", D1, 10.0, D2, 10.0), row("B", D1, 10.0, D2, 10.0)]
        t = P.simulate(rows, 2, "none", equity0=1000.0, comm=0.01)
        self.assertEqual([(x["symbol"], x["shares"]) for x in t], [("A", 50), ("B", 49)])

    def test_no_shrink_skips(self):
        rows = [row("A", D1, 10.0, D2, 10.0), row("B", D1, 10.0, D2, 10.0)]
        t = P.simulate(rows, 2, "none", equity0=1000.0, comm=0.01, shrink=False)
        self.assertEqual([x["symbol"] for x in t], ["A"])

    def test_less_than_one_share_skipped(self):
        t = P.simulate([row("A", D1, 2000.0, D2, 2000.0), row("B", D1, 50.0, D2, 50.0)], 10, "price_hl",
                       equity0=10000.0)
        self.assertEqual([x["symbol"] for x in t], ["B"])         # 1000 / 2000 < 1 share; B gets 20

    def test_prev_bar_equity_marks_open_positions(self):
        # A (100) is held over day 2's entry bar. pct 30, equity 1000, no commission:
        # day 1: A 300 / 100 = 3 sh, cash 700. Day 2, B at 40:
        #   marked at 140 -> equity 700 + 3 x 140 = 1120 -> 336 / 40 = 8.4 -> 8 sh (weight 320 / 1120)
        #   unmarked (entry price) -> equity 700 + 300 = 1000 -> 300 / 40 = 7.5 -> 7 sh
        rows = [row("A", D1, 100.0, "2007-01-10 09:55:00", 100.0), row("B", D3, 40.0, "2007-01-09 09:55:00", 40.0)]
        t = P.simulate(rows, 2, "none", pct=30, equity0=1000.0, comm=0.0, mark=lambda s, ts: 140.0)
        self.assertEqual(t[1]["shares"], 8)
        self.assertAlmostEqual(t[1]["weight"], 320.0 / 1120.0)
        t = P.simulate(rows, 2, "none", pct=30, equity0=1000.0, comm=0.0)
        self.assertEqual(t[1]["shares"], 7)


class Ranking(unittest.TestCase):
    def test_higher_score_first_and_max_positions(self):
        rows = [row("A", D1, 10.0, D2, 10.0), row("B", D1, 30.0, D2, 30.0), row("C", D1, 20.0, D2, 20.0)]
        self.assertEqual([x["symbol"] for x in P.simulate(rows, 2, "price_hl")], ["B", "C"])
        self.assertEqual([x["symbol"] for x in P.simulate(rows, 2, "price_lh")], ["A", "C"])

    def test_ties_alphabetical(self):
        rows = [row("ZEEL", D1, 10.0, D2, 10.0), row("ACC", D1, 10.0, D2, 10.0), row("M&M", D1, 10.0, D2, 10.0)]
        self.assertEqual([x["symbol"] for x in P.simulate(rows, 2, "price_hl")], ["ACC", "M&M"])
        # no PositionScore (NIFTY incremental AFL): every signal ties -> alphabetical
        self.assertEqual([x["symbol"] for x in P.simulate(rows, 2, "none")], ["ACC", "M&M"])
        # tie key = the team's 2022 name (ZYDUSLIFE was CADILAHC)
        key = lambda s: {"ZEEL": "CADILAHC"}.get(s, s)
        self.assertEqual([x["symbol"] for x in P.simulate(rows, 2, "none", tie_key=key)], ["ACC", "ZEEL"])

    def test_null_score_ranks_as_zero(self):
        rows = [row("A", D1, 10.0, D2, 10.0, ibs=None), row("B", D1, 10.0, D2, 10.0, ibs=0.2)]
        self.assertEqual([x["symbol"] for x in P.simulate(rows, 1, "ibs_hl")], ["B"])

    def test_ibs_roc_score(self):
        self.assertAlmostEqual(P.score_of("ibs_roc", dict(ibs=0.25, roc_a=3.5)), 103.75)
        self.assertEqual(P.score_of("ibs_lh", dict(ibs=0.25)), 99.75)

    def test_lookback_rank(self):
        sigs = [dict(symbol="A", lbc=1), dict(symbol="B", lbc=0), dict(symbol="C", lbc=0), dict(symbol="D", lbc=2)]
        # ver 2: rank 1 = highest cond (D), then A, then B, C (list order); PositionScore = rank
        self.assertEqual(P.lookback_rank_fn(2)(sigs), [2, 3, 4, 1])
        self.assertEqual(P.lookback_rank_fn(11)(sigs), [998, 997, 996, 999])
        rows = [row(s["symbol"], D1, 10.0, D2, 10.0, lbc=s["lbc"]) for s in sigs]
        self.assertEqual([x["symbol"] for x in P.simulate(rows, 2, "none", rank_fn=P.lookback_rank_fn(2))], ["C", "B"])


class Slots(unittest.TestCase):
    def test_exit_before_entry_on_same_bar(self):
        # A exits at D3 exactly when B signals: the slot is free for B
        rows = [row("A", D1, 10.0, D3, 10.0), row("B", D3, 10.0, "2007-01-09 09:55:00", 10.0)]
        self.assertEqual([x["symbol"] for x in P.simulate(rows, 1, "none")], ["A", "B"])

    def test_open_position_blocks_slot_and_symbol(self):
        rows = [row("A", D1, 10.0, "2007-01-10 09:55:00", 10.0), row("A", D3, 10.0, "2007-01-09 09:55:00", 10.0),
                row("B", D3, 10.0, "2007-01-09 09:55:00", 10.0), row("C", D3, 10.0, "2007-01-09 09:55:00", 10.0)]
        t = P.simulate(rows, 2, "none")
        self.assertEqual([(x["symbol"], x["entry_ts"]) for x in t], [("A", D1), ("B", D3)])


class Keys(unittest.TestCase):
    def test_cfg_key_roc_days(self):
        c = dict(days=2, entry_time=1440, exit_time=915, pb_pct=0.0, hardstop=0.0, flt_type=3)
        self.assertEqual(P.cfg_key(c), "lb2_en1440_ex915_pb0.0_hs0.0_f3")
        self.assertEqual(P.cfg_key(dict(c, roc_days=(2, 1), mfilter="team")), "lb2_en1440_ex915_pb0.0_hs0.0_f3_team_roc2-1")


if __name__ == "__main__":
    unittest.main()
