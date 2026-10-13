# Reproduce the ETF Sweet Spot book tables from the Rust engine (unit-level, in points, like the book).
#   python3 verify-sweetspot.py [api=http://localhost:8091]
import json, sys, urllib.request, urllib.parse, collections
API = sys.argv[1] if len(sys.argv) > 1 else 'http://localhost:8091'
# optional strategy settings, e.g. min_stretch=0 (book default: none)
PARAMS = {'p.' + k: v for k, v in (a.split('=') for a in sys.argv[2:])}
OUT = '/home/server/.local/share/strategy-vault/book2/verify-report' + ('-' + '-'.join(sys.argv[2:]) if sys.argv[2:] else '') + '.json'
get = lambda **q: json.load(urllib.request.urlopen(f'{API}/api/guru/connors/run?' + urllib.parse.urlencode({**q, **PARAMS})))

# Book, p.29-43: units, win %, net points, profit factor (all / long / short), test 1/3/00 - 6/16/04
BOOK = {
    'SPY': {'all': (88, 72.73, 100.01, 2.46), 'long': (36, 80.56, 46.78, 4.06), 'short': (52, 67.31, 53.23, 2.00), 'perf': 68.77},
    'QQQ': {'all': (71, 74.65, 70.50, 2.03), 'long': (36, 80.56, 20.51, 1.47), 'short': (35, 68.57, 49.99, 3.04), 'perf': 74.41},
    'IWM': {'all': (58, 72.41, 65.41, 2.46), 'long': (37, 83.78, 71.58, 6.53), 'short': (21, 52.38, -6.17, .81), 'perf': 71.83},
    'SMH': {'all': (62, 67.74, 68.45, 3.00), 'long': (25, 64.00, 12.60, 1.98), 'short': (37, 70.27, 55.85, 3.62), 'perf': 73.95},
    'MDY': {'all': (83, 79.52, 96.75, 2.94), 'long': (59, 89.83, 103.32, 11.26), 'short': (24, 54.17, -6.57, .84), 'perf': 120.80},
}
SPY_YEARS = {2003: (27.11, 20, 90.00), 2002: (9.76, 19, 63.16), 2001: (12.36, 21, 57.14), 2000: (49.68, 22, 77.27), 1999: (18.80, 16, 62.50), 1998: (15.28, 13, 69.23),
             1997: (27.28, 18, 77.78), 1996: (11.95, 19, 73.68), 1995: (7.12, 11, 90.91), 1994: (2.31, 17, 52.94), 1993: (0.75, 2, 100.00)}

def units(d):
    """Every unit as a separate trade, in points (the book's convention). Open trades excluded."""
    out = []
    for t in d['trades']:
        if t['open']: continue
        for i, px, w in t['legs']:
            out.append({'side': 'long' if t['dir'] > 0 else 'short', 'year': int(t['exit'][:4]), 'pts': t['dir'] * (t['exit_px'] - px)})
    return out

def summary(us):
    n = len(us); g = sum(u['pts'] for u in us if u['pts'] > 0); l = -sum(u['pts'] for u in us if u['pts'] <= 0)
    return (n, 100 * sum(u['pts'] > 0 for u in us) / n if n else float('nan'), sum(u['pts'] for u in us), g / l if l else float('inf'))

report = {'etf': {}, 'spy_years': {}}
print(f'{"ETF":5} {"side":6} {"units book/ours":>16} {"win% book/ours":>17} {"PF book/ours":>14} {"points book/ours (scaled)":>28}')
for tk, b in BOOK.items():
    d = get(id='ss_both', ticker=tk, **{'from': '2000-01-03', 'to': '2004-06-16'})
    scale = (b['all'][2] / (b['perf'] / 100)) / d['close'][0]   # book's start price / our start price (splits, adjustment)
    us = units(d); report['etf'][tk] = {'scale': scale}
    for side in ('all', 'long', 'short'):
        n, w, p, pf = summary([u for u in us if side == 'all' or u['side'] == side]); bn, bw, bp, bpf = b[side]
        report['etf'][tk][side] = {'book': b[side], 'ours': (n, w, p * scale, pf)}
        print(f'{tk:5} {side:6} {bn:>7} / {n:<7} {bw:>7.2f} / {w:<7.2f} {bpf:>5.2f} / {pf:<6.2f} {bp:>10.2f} / {p * scale:<10.2f}')
d = get(id='ss_both', ticker='SPY', **{'from': '1993-01-29', 'to': '2003-12-31'})
by = collections.defaultdict(list)
for u in units(d): by[u['year']].append(u)
print('\nSPY by year   points book/ours   units book/ours   win% book/ours')
for y, (bp, bn, bw) in SPY_YEARS.items():
    n, w, p, _ = summary(by[y]) if by[y] else (0, float('nan'), 0, 0)
    report['spy_years'][y] = {'book': (bp, bn, bw), 'ours': (p, n, w)}
    print(f'{y}   {bp:>7.2f} / {p:<7.2f}   {bn:>4} / {n:<4}   {bw:>6.2f} / {w:<6.2f}')
json.dump(report, open(OUT, 'w'), indent=1)
