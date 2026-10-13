# Merge the analysts' research notes with the batch numbers into frontend/data/claude-conclusions.json,
# and audit every number they cite against the evidence for that strategy.
#   python3 merge-notes.py notes-A.json notes-B.json ...
import json, re, sys, os
from collections import Counter

here = os.path.dirname(os.path.abspath(__file__))
data = os.path.join(here, '../frontend/data')
batch = json.load(open(os.path.join(data, 'optim-batch.json')))
notes = {}
for f in sys.argv[1:]:
    notes.update(json.load(open(f)))
live = set(batch['strategies'])
print('notes given for', len(set(notes) & live), 'strategies; batch has', len(live), '| extra:', sorted(set(notes) - live))

def numbers_in(o, acc):
    if isinstance(o, dict): [numbers_in(v, acc) for v in o.values()]
    elif isinstance(o, list): [numbers_in(v, acc) for v in o]
    elif isinstance(o, (int, float)) and not isinstance(o, bool):
        for x in (o, o * 100):
            for d in range(4): acc.add(f'{abs(x):.{d}f}')
            acc.add(f'{abs(x):g}')
    elif isinstance(o, str): acc.update(m for m in re.findall(r'\d+(?:\.\d+)?', o.replace(',', '')))
    return acc

def strings(o):
    if isinstance(o, str): return [o]
    if isinstance(o, dict): return [t for v in o.values() for t in strings(v)]
    if isinstance(o, list): return [t for v in o for t in strings(v)]
    return []
generic = {str(y) for y in range(1990, 2031)} | {str(k) for k in range(0, 13)} | {'0.95', '0.5', '1.5', '95', '50', '30', '60', '40', '20', '100', '200', '5', '0.1', '0.28', '19.67', '51.5', '53.8', '2896', '2015', '2026'}
FIELDS = ('verdict', 'conviction', 'headline', 'recommendation', 'thesis', 'findings', 'rejected', 'risks', 'monitor')
pickm = lambda m: {k: m.get(k) for k in ('sharpe', 'is_sharpe', 'oos_sharpe', 'cagr', 'max_dd', 'trades', 'is_trades', 'oos_trades', 't_stat', 'is_t_stat', 'oos_t_stat', 'exposure')}
runs = sum(1 for v in batch['strategies'].values() for x in v['indices'].values() if 'combos' in x)
settings = sum(x.get('combos', 0) for v in batch['strategies'].values() for x in v['indices'].values())
US_TK = {'SPY', 'QQQ', 'MDY', 'IWM', 'SMH'}
# update in place: notes for other strategies (other books) are kept
prev = os.path.join(data, 'claude-conclusions.json')
out, unmatched = (json.load(open(prev)) if os.path.exists(prev) else {}), {}
for id_, n in notes.items():
    s = batch['strategies'][id_]
    ok = numbers_in(s, set()) | generic
    bad = [x for x in re.findall(r'\d+(?:\.\d+)?', ' '.join(t for t in strings(n) if not t in ('tuned', 'book', 'avoid', 'thin')).replace(',', '')) if x not in ok and x.rstrip('0').rstrip('.') not in ok]
    if bad: unmatched[id_] = bad
    rec = {}
    for mk in ('india', 'us'):
        part = n.get(mk, {}); idx = {}
        for tk, x in s['indices'].items():
            if not x.get('robust') or (tk in US_TK) != (mk == 'us'): continue
            a = part.get('indices', {}).get(tk, {}); r, b = x['robust'], x['book']
            idx[tk] = {'use': a.get('use', 'book'), 'call': a.get('call', ''), 'note': a.get('note', ''), 'p': {} if r['is_book'] else r['p'],
                       'sharpe': r['sharpe'], 'oos_sharpe': r['oos_sharpe'], 'trades': r['trades'], 'dsr': x['dsr'],
                       'book_sharpe': b.get('sharpe'), 'book_oos_sharpe': b.get('oos_sharpe'), 'book_trades': b.get('trades')}
        rec[mk] = {**{k: part[k] for k in FIELDS if k in part}, 'indices': idx}
    ev = {tk: {k: x.get(k) for k in ('from', 'to', 'split', 'combos', 'funnel', 'hist', 'dsr', 'sr0', 'post')} | {'book': pickm(x['book']), 'pick': pickm(x['robust']) | {'p': x['robust']['p'], 'is_book': x['robust']['is_book'], 'tier': x['robust']['tier']}}
          for tk, x in s['indices'].items() if x.get('robust')}
    out[id_] = {**rec, 'evidence': ev,
                'method': f"Research note by Claude from {runs} optimization runs covering {settings:,} settings (data/optim-batch.json, {batch['generated'][:10]}). " + batch['method']}
json.dump(out, open(os.path.join(data, 'claude-conclusions.json'), 'w'), indent=1, ensure_ascii=False)
print('numbers not found in the evidence:', json.dumps(unmatched))
for mk in ('india', 'us'):
    print(mk, Counter(n.get(mk, {}).get('verdict') for n in notes.values()), Counter(n.get(mk, {}).get('conviction') for n in notes.values()))
