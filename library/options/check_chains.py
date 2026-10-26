#!/usr/bin/env python3
"""Coverage check of the derived option chains against the vault's daily bars (one ticker file at a time, streamed).
Per year: underlying bars, chain days, missing chain days, monthly expiries per day, share of days whose ATM front put
(nearest strike to the vault close) traded, US quotes with ask < bid or bid <= 0.
usage: check_chains.py STAGING_DIR"""
import collections, csv, sys

D = "/home/server/.cache/strategy-vault-webapp/data"
TICK = {"india": [("NIFTY 50", "NIFTY_50"), ("NIFTYBANK", "NIFTYBANK")], "us": [("SPY", "SPY"), ("QQQ", "QQQ"), ("IWM", "IWM")]}


def bars(path, ticker):
    out = {}
    for r in csv.DictReader(open(path)):
        if r["Ticker"] == ticker:
            d, m, y = r["Date"].split("-")
            out[f"{y}-{m}-{d}"] = float(r["Close"].replace(",", ""))
    return out


def main():
    st = sys.argv[1]
    for mkt, pairs in TICK.items():
        src = f"{D}/{'idx_data' if mkt == 'india' else 'us_data'}.csv"
        for t, f in pairs:
            cl = bars(src, t)
            per = collections.defaultdict(lambda: [0, 0, 0, 0, 0, 0])  # bars, chain days, missing, atm traded, exps, bad quotes
            seen = set()

            def day(d, rows):
                seen.add(d)
                if d not in cl:
                    return
                y = d[:4]
                per[y][1] += 1
                exps = sorted({r[1] for r in rows})
                per[y][4] += len(exps)
                puts = [r for r in rows if r[1] == exps[0] and r[2] == "P"]
                if puts:
                    atm = min(puts, key=lambda r: (abs(float(r[3]) - cl[d]), float(r[3])))
                    per[y][3] += float(atm[9] if mkt == "india" else atm[7]) > 0
                if mkt == "us":
                    per[y][5] += sum(1 for r in rows if float(r[4]) <= 0 or float(r[5]) < float(r[4]))
            cur, rows = None, []
            for line in open(f"{st}/{mkt}/{f}.opt.csv"):
                if line.startswith("Date"):
                    continue
                v = line.rstrip("\n").split(",")
                if v[0] != cur:
                    if cur:
                        day(cur, rows)
                    cur, rows = v[0], []
                rows.append(v)
            day(cur, rows)
            first, last = min(seen), max(seen)
            for d in cl:
                if first <= d <= last:
                    per[d[:4]][0] += 1
                    if d not in seen:
                        per[d[:4]][2] += 1
            days = seen
            print(f"== {t} ({mkt}) chain {first}..{last}; underlying bars inside: {sum(p[0] for p in per.values())}, chain days missing: {sum(p[2] for p in per.values())}")
            print("year bars chain_days missing atm_front_put_traded% mean_expiries" + (" one_sided_or_crossed_quotes" if mkt == "us" else ""))
            for y, p in sorted(per.items()):
                print(y, p[0], p[1], p[2], f"{100 * p[3] / max(p[1], 1):.1f}", f"{p[4] / max(p[1], 1):.2f}", *( [p[5]] if mkt == "us" else []))
            missing = [d for d in sorted(cl) if first <= d <= last and d not in days]
            if missing:
                print("missing chain days:", " ".join(missing[:40]), "..." if len(missing) > 40 else "")
            extra = [d for d in sorted(days) if d not in cl]
            if extra:
                print("chain days with no underlying bar:", " ".join(extra[:40]), "..." if len(extra) > 40 else "")


if __name__ == "__main__":
    main()
