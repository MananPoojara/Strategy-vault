"""Replicate the team's NIFTY BTST tradelist (entry 14:50, exit 09:25, no filter, PB 0, HSL 0,
no IBS/ROC thresholds) with oracle_btst and report match rates + every mismatch with a reason.

~/.venvs/strategy-vault/bin/python -I verify_against_tradelist.py [--days 2] [--keep-1530]
Nothing is tuned: the only knobs are --days and --keep-1530 (see VERIFICATION-TRADELIST.md).
"""
import argparse, datetime as dt, os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import polars as pl
import oracle_btst as o

SCR = "/tmp/claude-1000/-home-aff33-Desktop-Strategy-Vault/af66f2bb-a019-4872-b960-3149e20bcd5e/scratchpad/intraday/"
ap = argparse.ArgumentParser()
ap.add_argument("--days", type=int, default=2)
ap.add_argument("--keep-1530", action="store_true")
ap.add_argument("--tl", default=SCR + "tl_5_min_timeframe.csv")
ap.add_argument("--parquet", default=SCR + "rolling_futures.parquet")
ap.add_argument("--start", default="2019-02-11"); ap.add_argument("--end", default="2021-10-26")
a = ap.parse_args()
start, end = dt.date.fromisoformat(a.start), dt.date.fromisoformat(a.end)

def xl(v):  # Excel serial -> datetime (to the second)
    return dt.datetime(1899, 12, 30) + dt.timedelta(seconds=round(v * 86400))

tl = []
for r in pl.read_csv(a.tl).iter_rows(named=True):   # columns 2..5 = Date, Price, Ex.date, Ex.Price
    en, ex = xl(r["2"]), xl(r["4"])
    if start <= en.date() <= end:
        tl.append(dict(en=en, px=r["3"], ex=ex, xpx=r["5"]))

bars, partial = o.build_5min(a.parquet, drop_1530=not a.keep_1530)
data = o.Data(bars)
ours = o.run(dict(days=a.days, entry_time=1450, exit_time=925, pb_pct=0, hardstop=0, ibs_thr=1, roc_thr=-100,
                  flt_type=3), data=data, start=start, end=end)
O = {t["signal_date"]: t for t in ours}

# expiry per day, to flag roll days
ex = (pl.read_parquet(a.parquet).group_by(pl.col("ts").dt.date().alias("d")).agg(pl.col("expiry").max()).sort("d"))
expiry = dict(zip(ex["d"].to_list(), ex["expiry"].to_list()))
dl = sorted(expiry)
# Manual diagnoses (investigated by hand against the parquet; keyed by signal date)
EXPLAIN = {
    "2019-02-25": "feed's rolling series is the APRIL-2019 contract all through Feb/Mar 2019 (~80 pts premium); TL is near-month F1",
    "2019-03-15": "same as 2019-02-25: April contract in the feed vs near-month in the TL",
    "2019-04-01": "0.35 pt exit-bar close difference (data vendor difference, no structural reason found)",
    "2021-01-01": "feed's rolling row for 2021-01-01 carries expiry=2021-01-01 (roll artefact); TL used the Jan-2021 contract (30.75 pts apart)",
    "2020-11-17": "feed has a full 75-bar session on Sat 2020-11-14 (Diwali muhurat) treated as a trading day => d-1 differs; TL excludes it (then P-C > DIFF holds)",
    "2020-11-18": "consequence of the 2020-11-14 extra day (see 2020-11-17)",
    "2021-01-04": "expiry roll 2020-12-31/2021-01-01 -> 2021-01-28 changes the daily closes used for up-close/diff",
    "2021-02-25": "2021-02-24 daily close in feed (15112.4) differs from what TL saw (monthly-expiry week; oracle signals 02-24 instead). Unresolved data difference",
    "2021-02-24": "oracle signals here, TL signals on 02-25 instead; same unresolved 2021-02-24 data difference",
}

def near_roll(d, back=3):
    if d not in expiry: return False
    i = dl.index(d)
    return any(expiry[dl[j]] != expiry[dl[j - 1]] for j in range(max(1, i - back), i + 1))

print(f"lookback days={a.days}  drop_1530={not a.keep_1530}  window {start}..{end}")
print(f"tradelist trades in window: {len(tl)} ; oracle trades: {len(ours)} (partial 5m bars in data: {partial})")
m_en = m_px = m_exd = m_expx = 0
issues = []
tl_dates = set()
for t in tl:
    d = t["en"].date().isoformat(); tl_dates.add(d)
    w = O.get(d)
    if w is None:
        why = []
        if near_roll(t["en"].date()): why.append("expiry roll within 3 days before signal (daily closes differ)")
        if t["en"].date() not in data.day_idx: why.append("no data that day")
        issues.append((d, "TL-ONLY (not signalled by oracle)", f"TL entry {t['px']}", EXPLAIN.get(d) or "; ".join(why) or "daily-close/diff condition differs"))
        continue
    m_en += 1
    if abs(w["entry_px"] - t["px"]) <= 0.05: m_px += 1
    else: issues.append((d, "entry px", f"TL {t['px']} oracle {w['entry_px']}", EXPLAIN.get(d) or ("roll-day / different contract price" if near_roll(t["en"].date(), 1) else "price differs")))
    tx = t["ex"]
    if w["exit_date"] == tx.date().isoformat(): m_exd += 1
    else: issues.append((d, "exit date", f"TL {tx} oracle {w['exit_ts']}", "different exit day"))
    if abs(w["exit_px"] - t["xpx"]) <= 0.05: m_expx += 1
    else:
        hh = tx.strftime("%H:%M")
        why = f"TL exit at {hh}, oracle {w['exit_ts'][11:16]} (TL bar differs)" if hh != "09:25" else ("roll-day" if near_roll(tx.date(), 1) else "price differs")
        issues.append((d, "exit px", f"TL {t['xpx']} oracle {w['exit_px']}", EXPLAIN.get(d) or why))
n = len(tl)
print(f"entry date match : {m_en}/{n}\nentry px match   : {m_px}/{m_en} of date-matched (tol 0.05)")
print(f"exit date match  : {m_exd}/{m_en}\nexit px match    : {m_expx}/{m_en} (tol 0.05)")
extra = [t for t in ours if t["signal_date"] not in tl_dates]
print(f"oracle-only trades: {len(extra)}")
print("\nMISMATCHES (tradelist-side)")
for r in issues: print("  " + " | ".join(r))
print("\nORACLE-ONLY")
for t in extra:
    print(f"  {t['signal_date']} | entry {t['entry_px']} | exit {t['exit_ts']} @ {t['exit_px']} | {EXPLAIN.get(t['signal_date'], 'no TL trade: borderline/other')}")
print("\nTL exit-time histogram:", {k: sum(1 for t in tl if t['ex'].strftime('%H:%M') == k) for k in sorted({t['ex'].strftime('%H:%M') for t in tl})})
