#!/usr/bin/env python3
"""Intraday wave 2a: run compare_w1.py (wave-1 reference, unchanged) with the wave-2a data exclusion applied on the
reference side: NIFTY 50 minute days from 2026-08-11 are rejected (bad closing minutes, data-verification-20261027),
so the reference makes no trade on them. Same arguments as compare_w1.py."""
import os, runpy, sys
HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import indep_ref
_day = indep_ref.Minutes.day


def day(self, d):
    if self.path.endswith("/NIFTY_50.m1.csv") and d >= "2026-08-11":
        return None
    return _day(self, d)


indep_ref.Minutes.day = day
sys.argv = [os.path.join(HERE, "compare_w1.py")] + sys.argv[1:]
runpy.run_path(sys.argv[0], run_name="__main__")
