#!/usr/bin/env python3
"""Current & Past (C&P) AmiBroker databases -> dated index membership for the Indicators lab portfolio.

The research team's C&P databases (share DATABASE/Database Directory.xlsx) hold every stock that was ever in an
index, with a per-day flag in AmiBroker's Aux fields: AUX1 = 1 while the stock is in the index the database is
named for, AUX2 = 1 while it is in the NSE F&O list (C&P DATABASE only). The lab buys a stock only on days its
flag is 1 (the AFLs' `Buy = Buy AND Aux1 == 1`); a held stock that leaves the list keeps its own exit (user, 20-11-2026).

Output (outside Git): prices/<sha>.csv once per stock (date,open,high,low,close,volume,open_interest,aux1,aux2; aux2 =
the F&O flag) and index.csv, one row per universe and stock with the periods the flag was 1 ("from..to;from..to").
Prices come from the broad C&P DATABASE, which has each company's full history; an index database can start late
(BEL and TRENT start 2022-06-23 in the NIFTY 50 one). Sources are read only. Nothing is repaired; the one known vendor
hole (2025-12-16, every stock) takes the official NSE quote already admitted in the engine's basket data, when present.

AmiBroker native file (BROKDAt5): 0x4A0-byte header (u32 record count at byte 1180; bytes after the counted records
are left-over garbage, e.g. ABB's last 6), then 40-byte records:
  u64 packed date (year bits 52-63, month 48-51, day 43-47), f32 close, open, high, low, volume, open interest, aux1, aux2.
"""
import argparse, csv, hashlib, json, re, struct, sys
from pathlib import Path

SHARE = Path('/run/user/1000/gvfs/smb-share:server=192.168.4.50,share=share/DATABASE/Updated Data Base For Optim/Updated Till 16-10-2026')
BROAD = 'C&P DATABASE DATABASE UPDATED TILL 16-10-2026'
# universe id, database folder, aux field (1 = the index, 2 = F&O), label
UNIVERSES = [
    ('nifty50_cp', 'NIFTY 50 C&P (With Aux On - Off)_Updated Till 16-10-2026', 1, 'NIFTY 50 (members by date)'),
    ('nse500_cp', BROAD, 1, 'NSE 500 (members by date)'),
    ('fo_cp', BROAD, 2, 'F&O stocks (members by date)'),
    ('midcap100_cp', 'MIDCAP100 C&P_Updated Till  16-10-2026', 1, 'NIFTY MIDCAP 100 (members by date)'),
    ('midcapselect_cp', 'MIDCAP SELECT C&P UPDATED TILL 16-10-2026', 1, 'NIFTY MIDCAP SELECT (members by date)'),
]
SUPPLEMENT = Path('/home/aff34/.cache/strategy-vault-dharmil/basket-data')  # admitted official quotes for the vendor hole
HOLES = {'2025-12-16'}
HEADER, RECORD, MAGIC = 0x4A0, 40, b'BROKDAt5'
COLUMNS = ['date', 'open', 'high', 'low', 'close', 'volume', 'open_interest', 'aux1', 'aux2']
INDEX = ['universe', 'label', 'ticker', 'symbol', 'path', 'first', 'last', 'bars', 'member_days', 'periods', 'price_source',
         'excluded_prefix_count', 'supplemented', 'source_sha256', 'csv_sha256']


def packed_date(v):
    return f'{(v >> 52) & 0xFFF:04d}-{(v >> 48) & 0xF:02d}-{(v >> 43) & 0x1F:02d}'


def read_native(data):
    """Rows (date, open, high, low, close, volume, oi, aux1, aux2) in file order."""
    if data[:8] != MAGIC:
        raise ValueError('not an AmiBroker BROKDAt5 file')
    count = struct.unpack_from('<I', data, HEADER - 4)[0]
    if HEADER + count * RECORD > len(data):
        raise ValueError('record count runs past the end of the file')
    out = []
    for at in range(HEADER, HEADER + count * RECORD, RECORD):
        r = data[at:at + RECORD]
        c, o, h, l, vol, oi, a1, a2 = struct.unpack('<8f', r[8:])
        out.append((packed_date(struct.unpack('<Q', r[:8])[0]), o, h, l, c, vol, oi, a1, a2))
    return out


def valid(row):
    d, o, h, l, c, v = row[:6]
    ok_date = re.fullmatch(r'\d{4}-\d{2}-\d{2}', d) and 1900 <= int(d[:4]) <= 2200 and 1 <= int(d[5:7]) <= 12 and 1 <= int(d[8:]) <= 31
    nums = all(x == x and abs(x) != float('inf') for x in row[1:])
    return bool(ok_date) and nums and min(o, h, l, c) > 0 and v >= 0 and l <= h and l <= o <= h and l <= c <= h


def valid_suffix(rows):
    """The longest contiguous run of valid, strictly increasing dates ending at the last record (the engine's
    loader refuses a file with any bad row); the count of earlier records left out is reported, never repaired."""
    start = len(rows)
    while start > 0 and valid(rows[start - 1]) and (start == len(rows) or rows[start - 1][0] < rows[start][0]):
        start -= 1
    return rows[start:], start


def periods(dates, flags):
    """Runs of consecutive records whose flag is 1, as 'first..last' joined by ';'."""
    out, start, prev = [], None, None
    for d, f in zip(dates, flags):
        if f == 1 and start is None:
            start = d
        if f != 1 and start is not None:
            out.append(f'{start}..{prev}')
            start = None
        prev = d
    if start is not None:
        out.append(f'{start}..{prev}')
    return ';'.join(out)


def num(x):
    return repr(float(x)) if x != int(x) else f'{float(x):.1f}'


def symbols(folder):
    root = SHARE / folder
    return {p.name: p for p in sorted(root.glob('*/*')) if p.is_file() and p.parent.name not in ('Layouts', 'WatchLists') and not p.name.startswith('broker')}


def native(path):
    data = path.read_bytes()
    try:
        rows, cut = valid_suffix(read_native(data))
    except ValueError:
        return None
    return (rows, cut, hashlib.sha256(data).hexdigest()) if len(rows) >= 2 else None


def supplements():
    """Official quotes for the vendor hole, by ticker, from the engine's admitted basket data."""
    out = {}
    index = SUPPLEMENT / 'index.csv'
    if not index.exists():
        return out
    for r in csv.DictReader(index.open()):
        if r['ticker'] in out:
            continue
        for row in csv.DictReader((SUPPLEMENT / r['path']).open()):
            if row['date'] in HOLES:
                out.setdefault(r['ticker'], {})[row['date']] = tuple([row['date']] + [float(row[k]) for k in COLUMNS[1:]])
    return out


def convert(out, only=None):
    out = Path(out)
    (out / 'prices').mkdir(parents=True, exist_ok=True)
    broad_files, extra = symbols(BROAD), supplements()
    loaded, written, index = {}, {}, []

    def load(folder, name, path):
        if (folder, name) not in loaded:
            loaded[(folder, name)] = native(path)
        return loaded[(folder, name)]

    for uid, folder, aux, label in UNIVERSES:
        if only and uid not in only:
            continue
        kept = 0
        files = symbols(folder)
        for name, path in files.items():
            own = load(folder, name, path)
            if not own or not any(r[6 + aux] == 1 for r in own[0]):
                continue  # never in this list: not part of the universe
            rows, cut, src = own
            flags = periods([r[0] for r in rows], [r[6 + aux] for r in rows])
            # prices: the broad database's full company history when it has this stock and starts no later
            broad = load(BROAD, name, broad_files[name]) if name in broad_files else None
            price_source = folder
            if broad and broad[0][0][0] <= rows[0][0]:
                rows, cut, src, price_source = broad[0], broad[1], broad[2], BROAD
            ticker = re.sub(r'[^A-Za-z0-9&.-]', '_', name)
            if ticker not in written or written[ticker][0] != price_source:
                have = {r[0] for r in rows}
                add = [r for d, r in extra.get(ticker, {}).items() if d not in have and rows[0][0] < d < rows[-1][0]]
                full = sorted(list(rows) + add)
                text = ','.join(COLUMNS) + '\n' + ''.join(f'{r[0]},' + ','.join(num(x) for x in r[1:]) + '\n' for r in full)
                sha = hashlib.sha256(text.encode()).hexdigest()
                rel = f'prices/{sha[:24]}.csv'
                (out / rel).write_text(text)
                written[ticker] = (price_source, rel, sha, full, len(add))
            _, rel, sha, full, added = written[ticker]
            kept += 1
            index.append(dict(universe=uid, label=label, ticker=ticker, symbol=name, path=rel, first=full[0][0], last=full[-1][0],
                              bars=len(full), member_days=sum(1 for r in own[0] if r[6 + aux] == 1), periods=flags,
                              price_source=price_source, excluded_prefix_count=cut, supplemented=added, source_sha256=src, csv_sha256=sha))
        print(f'{uid}: {kept} stocks ever in the list, of {len(files)} files', file=sys.stderr)
    with (out / 'index.csv').open('w', newline='') as f:
        w = csv.DictWriter(f, INDEX)
        w.writeheader()
        w.writerows(index)
    (out / 'source.json').write_text(json.dumps({'share': str(SHARE), 'broad': BROAD, 'universes': UNIVERSES, 'holes': sorted(HOLES)}, indent=1))
    return index


if __name__ == '__main__':
    a = argparse.ArgumentParser(description=__doc__)
    a.add_argument('out', help='output directory (outside Git), e.g. ~/.cache/strategy-vault-dharmil/cp-data-20261016')
    a.add_argument('--only', nargs='*', help='universe ids to convert (default: all)')
    args = a.parse_args()
    convert(args.out, set(args.only or []))
