#!/usr/bin/env python3
"""AmiBroker databases -> databases and watchlists for the Indicators lab portfolio, as AmiBroker opens them.

Each database folder in a snapshot (e.g. share DATABASE/Updated Data Base For Optim/Updated Till 16-10-2026) becomes a
database; each of its WatchLists/*.tls a watchlist. Every symbol keeps its own Aux1 / Aux2 from that database, so the
lab's Aux1 / Aux2 switches gate buys per stock exactly as the AFLs' `Buy = Buy AND Aux1 == 1` does in AmiBroker
(in the broad C&P DATABASE Aux1 = NSE 500, Aux2 = F&O; in an index database Aux1 = that index).

Output (outside Git): prices/<sha>.csv (date,open,high,low,close,volume,open_interest,aux1,aux2), symbols.csv (one row per
database and symbol) and watchlists.csv (one row per watchlist member, the watchlist's basket id first). Sources are read
only; nothing is repaired except the known vendor hole (convert_cp.py), whose added row carries the previous bar's Aux.
The INDEX watchlist is left out: an index cannot stand in for a stock universe (AGENTS.md).
"""
import argparse, csv, hashlib, json, re, sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).parent))
from convert_cp import COLUMNS, HOLES, native, num, supplements  # noqa: E402

SNAPSHOT = Path('/run/user/1000/gvfs/smb-share:server=192.168.4.50,share=share/DATABASE/Updated Data Base For Optim/Updated Till 16-10-2026')
# folder prefix -> short id (basket ids are at most 40 letters, digits, _ or -)
SHORT = [('C&P DATABASE', 'cp'), ('NIFTY 50', 'n50'), ('MIDCAP100', 'mid100'), ('MIDCAP SELECT', 'midsel'), ('NIFTY ALPHA LOW', 'alv')]
SKIP_WATCHLISTS = {'INDEX'}
SYMBOLS = ['db', 'db_label', 'symbol', 'ticker', 'path', 'first', 'last', 'bars', 'supplemented', 'source_sha256']
WATCH = ['id', 'db', 'db_label', 'watchlist', 'ticker']


def short(folder):
    return next((s for p, s in SHORT if folder.upper().startswith(p)), re.sub(r'[^a-z0-9]', '', folder.lower())[:8])


def slug(name):
    return re.sub(r'[^a-z0-9]', '', name.lower())


def convert(out, snapshot=SNAPSHOT):
    out, snapshot = Path(out), Path(snapshot)
    (out / 'prices').mkdir(parents=True, exist_ok=True)
    extra, symbols, watch, written = supplements(), [], [], set()
    for root in sorted(p for p in snapshot.iterdir() if p.is_dir()):
        db, label = short(root.name), root.name.strip()
        files = {p.name: p for p in sorted(root.glob('*/*')) if p.is_file() and p.parent.name not in ('Layouts', 'WatchLists') and not p.name.startswith('broker')}
        tickers = {}
        for name, path in files.items():
            got = native(path)
            if not got:
                continue
            rows, _cut, src = got
            ticker = re.sub(r'[^A-Za-z0-9&.-]', '_', name)
            have, add = {r[0] for r in rows}, []
            for d, r in extra.get(ticker, {}).items():
                if d not in have and rows[0][0] < d < rows[-1][0]:
                    prev = max((x for x in rows if x[0] < d), key=lambda x: x[0])
                    add.append(tuple(r[:7]) + tuple(prev[7:9]))  # the vendor hole: official quote, this database's Aux of the day before
            full = sorted(list(rows) + add)
            text = ','.join(COLUMNS) + '\n' + ''.join(f'{r[0]},' + ','.join(num(x) for x in r[1:]) + '\n' for r in full)
            sha = hashlib.sha256(text.encode()).hexdigest()
            rel = f'prices/{sha[:24]}.csv'
            if rel not in written:
                (out / rel).write_text(text)
                written.add(rel)
            tickers[name] = ticker
            symbols.append(dict(db=db, db_label=label, symbol=name, ticker=ticker, path=rel, first=full[0][0], last=full[-1][0], bars=len(full),
                                supplemented=len(add), source_sha256=src))
        lists = 0
        for tls in sorted((root / 'WatchLists').glob('*.tls')):
            if tls.stem.upper() in SKIP_WATCHLISTS:
                continue
            names = [l.strip() for l in tls.read_text(errors='replace').splitlines() if l.strip()]
            members = sorted({tickers[n] for n in names if n in tickers})
            if not members:
                continue
            wid = f'ami-{db}-{slug(tls.stem)}'[:40]
            watch += [dict(id=wid, db=db, db_label=label, watchlist=tls.stem, ticker=t) for t in members]
            lists += 1
            missing = sorted({n for n in names if n not in tickers})
            if missing:
                print(f'{db} / {tls.stem}: {len(missing)} names without usable prices: {", ".join(missing[:8])}', file=sys.stderr)
        print(f'{db}: {len(tickers)} symbols of {len(files)} files, {lists} watchlists', file=sys.stderr)
    ids = {}
    for w in watch:
        ids.setdefault(w['id'], (w['db'], w['watchlist']))
        assert ids[w['id']] == (w['db'], w['watchlist']), f"two watchlists share the id {w['id']}"
    # the engine splits these files on commas (no quoting), so a comma or quote in a name is refused, never written
    bad = [v for r in symbols + watch for v in map(str, r.values()) if ',' in v or '"' in v]
    assert not bad, f'names with a comma or quote: {bad[:5]}'
    for name, rows, cols in (('symbols.csv', symbols, SYMBOLS), ('watchlists.csv', watch, WATCH)):
        with (out / name).open('w', newline='') as f:
            w = csv.DictWriter(f, cols)
            w.writeheader()
            w.writerows(rows)
    (out / 'source.json').write_text(json.dumps({'snapshot': str(snapshot), 'skip_watchlists': sorted(SKIP_WATCHLISTS), 'holes': sorted(HOLES)}, indent=1))


if __name__ == '__main__':
    a = argparse.ArgumentParser(description=__doc__)
    a.add_argument('out', help='output directory (outside Git), e.g. ~/.cache/strategy-vault-dharmil/ami-db-20261016')
    a.add_argument('--snapshot', default=str(SNAPSHOT), help='folder holding the AmiBroker database folders')
    args = a.parse_args()
    convert(args.out, args.snapshot)
