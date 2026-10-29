#!/usr/bin/env python3
"""Intraday wave 2b: compare the engine's ledgers (HTTP API, private port) with the independent references, trade by trade
(direction, entry date and price, exit date and price, open flag; prices to 1e-6) and by count.
Usage: compare_w2b.py PORT us|in OUT_JSON [id-prefix filter]"""
import bisect, importlib, json, os, sys, urllib.error, urllib.parse, urllib.request
HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import indep_w2b as W  # noqa: E402
import indep_ref as IR  # noqa: E402  (phase-1 minute loader: asserts every minute is finite and OHLC-consistent)

PORT, MARKET, OUT = sys.argv[1], sys.argv[2], sys.argv[3]
FILT = sys.argv[4] if len(sys.argv) > 4 else ""
AUTH = "/home/server/.cache/strategy-vault-webapp/claude-continuation-20261020/authors"
D_ = "/home/server/.cache/strategy-vault-webapp/data"
# already-admitted daily rules keep their family's independent reference (written from the book at admission)
FAM = {"hkmmts_bb_fade": "heitkoetter/admission/hk_ref", "lkcpt_box10": "kestner/admission/lk_ref", "lwcot_down_closes_daytrade": "lwilliams/admission/lw_ref",
       "pbwts_dow_oc": "pruitt/admission/pr_ref", "putg_daily_range": "pruitt/admission/pr_ref", "putg_gap": "pruitt/admission/pr_ref",
       "strtw_gd2": "stridsman/admission/str_ref", "strtw_meander1_short": "stridsman/admission/str_ref", "tdwb_tdline_fade_long": "demark/admission/demark_ref"}
ACBFF = ("acbff_2v5 acbff_5c_day acbff_c3_124 acbff_c3_125 acbff_c3_134 acbff_c3_135 acbff_c3_145 acbff_c3_234 acbff_c3_235 acbff_c3_245 acbff_c3_345 "
         "acbff_c40 acbff_cup_day acbff_dom_day acbff_dom_day_cc acbff_dow acbff_eight acbff_eight_3 acbff_eight_stop33 acbff_eight_stop33_3 "
         "acbff_eight_stop66 acbff_ext50 acbff_fade23oc acbff_fade2c acbff_fade2oc acbff_fade_oc acbff_five acbff_five_4of5 acbff_hl15 acbff_monday "
         "acbff_nee5 acbff_range10 acbff_six acbff_six_q acbff_six_q66 acbff_six_q66_v2 acbff_six_stop66 acbff_sr20 acbff_three acbff_tuesday acbff_wed_fri").split()
OTHER_A = ("ftm2_ac_vote ftm2_dm_skew rgc_cdow_fri_short rgc_dow_oc rgc_dowm_mon_dec_long rgc_dowm_thu_dec_short rgc_dowm_thu_jul_long rgc_runaway_gap "
           "rgc_runaway_gap_f5 stad2_stats2 lg3k_tr_day tdperl_clop_long tdperl_clop_short dgmf_vbo_taylor jbcdt2_dates_oc jbcdt2_preholiday_oc jbssf_gap").split()
SPX_A = "jk36k_jk1_buy_monday jkbk_buy_monday_ts83 jkbwts_buymon1400 stad10_mwt".split()
DAILY = ACBFF + OTHER_A + SPX_A + list(FAM)
MIN4 = "hfm_news hfm_news_long hfm_news_short lgvp_volpat mhp_ftse_12_20041015 mhp_ftse_3_20040804 putg_inside_day".split()
MINSPX = "bab80_daytrade jbcdt_gap_go jkbwts_saidenberg30 upg18_traders_dream".split()
US_ONLY = {"jbcdt2_preholiday_oc"}
if MARKET == "us":
    IDX, PATH = ("SPY", "SPX"), f"{D_}/us_data.csv"
    U = "/home/server/.cache/strategy-vault/us-market-data/daily/"
    STK = {"AAPL": "c220fe04bfd335ae85d6e1ad19464b3aa58026473f102f47012d4618f3bc5319", "MSFT": "264221e30a9c4fb6bd46a082532526fd9638e617f23af048c56ed7aca9e0783f"}
    API = {t: f"basket:us_sp500:{t}" for t in STK}
    MWIN = {"SPY": [("2000-01-03", "2010-12-31"), ("2011-01-03", "2025-10-21")], "SPX": [("2008-01-02", "2016-12-30"), ("2017-01-03", "2026-08-14")]}
else:
    IDX, PATH = ("NIFTY 50", "NIFTYBANK"), f"{D_}/idx_data.csv"
    U = "/home/server/.cache/strategy-vault/market-data-repaired-20261015-v2/daily/nifty50/"
    STK = {"RELIANCE": "9e95498193cc63fcd447", "TCS": "d628d9926390ba4143dd"}
    API = {t: f"basket:nifty50:{t}" for t in STK}
    MWIN = {"NIFTY 50": [("2008-01-01", "2016-12-30"), ("2017-01-02", "2026-10-07")], "NIFTYBANK": [("2010-03-02", "2017-12-29"), ("2018-01-01", "2026-04-22")]}
CACHE = {}
import re  # noqa: E402
_G = open(os.path.join(HERE, "../../../Context/guru/backend/src/gate.rs")).read()
CLAMP = {}
for gid, cl in re.findall(r'G \{ id: r##"(\w+)"##.*?clamp_open: (\w+), book', _G): CLAMP[gid] = cl == "true"  # the wave-2b block comes last and wins
REAL_OPEN = {"SPX": "2014-01-01", "NDX": "2001-01-01", "DJI": "2016-01-01"}  # gate.rs real_open_from


def mod(id_):
    if id_ in FAM:
        p = f"{AUTH}/{FAM[id_]}.py"; d, n = os.path.dirname(p), os.path.basename(p)[:-3]
        if d not in sys.path: sys.path.insert(0, d)
        return importlib.import_module(n)
    return W


def bars(m, t):
    k = (m.__name__, t)
    if k not in CACHE:
        CACHE[k] = m.load_stock(f"{U}{STK[t]}.csv") if t in STK else m.load_instrument(PATH, t)
    return CACHE[k]


def api(id_, t, a, b):
    q = urllib.parse.urlencode({"id": id_, "ticker": API.get(t, t), "from": a, "to": b})
    try:
        with urllib.request.urlopen(f"http://127.0.0.1:{PORT}/api/guru/connors/run?{q}", timeout=900) as r:
            return json.loads(r.read().decode())
    except urllib.error.HTTPError as e:
        return f"HTTP {e.code}: " + e.read().decode()


def same(x, y):
    return all((abs(p - q) <= 1e-6 * max(1.0, abs(p)) if isinstance(p, float) else p == q) for p, q in zip(x, y))


def run(id_, t, a_, b_, M=None, expect_refusal=False):
    m = mod(id_)
    B = bars(m, t); dates = [x[0] for x in B]
    got = api(id_, t, a_, b_)
    w = {"ticker": t, "id": id_, "from": a_, "to": b_}
    if isinstance(got, str) or "trades" not in got:
        w["refused"] = str(got)[:300]; w["expected"] = expect_refusal
        return w
    if expect_refusal:
        w.update(refused=None, expected=True, error="engine published a result where a refusal was expected", mismatches=1); return w
    a_eff = got.get("from") or a_
    a = bisect.bisect_left(dates, a_eff); b = bisect.bisect_right(dates, b_) - 1
    if m is W: refl = W.backtest(id_, B, a, b, {}, t, M)
    elif m.__name__ == "lw_ref": refl = m.backtest(id_, B, a, b, {})
    else: refl = m.backtest(id_, B, a, b, {}, t)
    ref = [(int(x["dir"]), dates[x["ei"]], float(x["epx"]), dates[x["xi"]], float(x["xpx"]), bool(x["open"])) for x in refl]
    rust = [(int(x["dir"]), x["entry"], float(x["legs"][0][1]), x["exit"], float(x["exit_px"]), bool(x["open"])) for x in got["trades"]]
    mism = [(i, r, e) for i, (r, e) in enumerate(zip(ref, rust)) if not same(r, e)]
    if len(ref) != len(rust): mism.append(("count", len(ref), len(rust)))
    w.update(from_eff=dates[a], to_eff=dates[b], ref_trades=len(ref), engine_trades=len(rust), mismatches=len(mism), first=[str(x) for x in mism[:3]])
    return w


res = {"windows": [], "totals": {"trades": 0, "mismatches": 0, "unexpected_refusals": 0, "expected_refusals": 0}}
def rec(w):
    res["windows"].append(w)
    if "refused" in w:
        if w.get("expected"): res["totals"]["expected_refusals"] += 1
        else: res["totals"]["unexpected_refusals"] += 1
    if "mismatches" in w:
        res["totals"]["trades"] += w.get("ref_trades", 0); res["totals"]["mismatches"] += w["mismatches"]
    print({k: v for k, v in w.items() if k != "first"}, w.get("first", [])[:1], flush=True)


for id_ in [i for i in DAILY if i.startswith(FILT)]:
    for t in list(IDX) + list(STK):
        exp = (id_ in SPX_A and t != "SPX") or (id_ in US_ONLY and MARKET == "in")
        B = bars(mod(id_), t)
        a0 = max(B[0][0], REAL_OPEN[t]) if CLAMP.get(id_) and t in REAL_OPEN else B[0][0]  # open-reading rules start where index opens are real
        rec(run(id_, t, a0, B[-1][0], expect_refusal=exp))
for id_ in [i for i in MIN4 + MINSPX if i.startswith(FILT)]:
    for t, wins in MWIN.items():
        if id_ in MINSPX and t != "SPX":
            rec(run(id_, t, wins[0][0], wins[-1][1], expect_refusal=True)); continue
        M = IR.Minutes(t)
        for a_, b_ in wins:
            # rules reading the daily open (and every SPX-only rule) start where SPX daily opens are real
            rec(run(id_, t, max(a_, "2014-01-02") if t == "SPX" and id_ != "putg_inside_day" else a_, b_, M))
    rec(run(id_, list(STK)[0], "2012-01-02", "2014-12-31", expect_refusal=True))  # no 1-minute bars for stocks
json.dump(res, open(OUT, "w"), indent=1)
print("TOTAL", res["totals"])
