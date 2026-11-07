"""Build the BTST 5-minute series from the team's AmiBroker IEOD_FO database (one symbol file).

usage: python -I btst_series_ieod.py --input <AmiBroker symbol file> --output <TICKER>__BTST_5M.csv [--report r.json]

The symbol file is a native AmiBroker 'BROKDAt5' file: 1184-byte header, 40-byte records (packed date-time u64, then
float32 Close, Open, High, Low, Volume, OpenInt, Aux1, Aux2), 4-byte trailer. Every minute is used as stored, as
AmiBroker uses it: padded minutes (Aux1 = 1, an exact copy of the previous minute) included, sessions as recorded
(09:55 open before 2010, 09:00 in 2010, after-hours and Muhurat evening sessions).

Output: one row per day of data. Columns
  Date                                   YYYY-MM-DD
  O<HHMM> H<HHMM> L<HHMM> C<HHMM>        start-stamped 5-minute bars HHMM = 0900, 0905, ... 1930 (bar t = minutes t..t+4)
  V<HHMM>                                5-minute volume (padded minutes included, as AmiBroker sums them)
  DO DH DL DC DV                         the day's OHLC and volume from ALL its minutes (AmiBroker's daily compression)
  NB                                     number of 1-minute records that day
Empty cell = no minute in that bucket. The grid reaches 19:34 so that every stored minute is in a bar, as in AmiBroker:
after-hours (to 16:14 in 2007-08), the extended sessions of 2012-11-13 and 2021-02-24 (to 17:00) and the Muhurat
evening sessions (16:45-19:30, 17 days).
"""
import argparse, csv, json, sys

import numpy as np

HEADER, REC = 1184, 40
BUCKETS = list(range(9 * 60, 19 * 60 + 35, 5))          # 09:00 .. 19:30 bucket starts
HHMM = [f"{m // 60:02d}{m % 60:02d}" for m in BUCKETS]


def load(path):
    raw = open(path, 'rb').read()
    if raw[:8] != b'BROKDAt5':
        raise ValueError(f"{path}: not an AmiBroker native file")
    dt = np.dtype([('u', '<u8'), ('c', '<f4'), ('o', '<f4'), ('h', '<f4'), ('l', '<f4'),
                   ('v', '<f4'), ('oi', '<f4'), ('a1', '<f4'), ('a2', '<f4')])
    r = np.frombuffer(raw, dt, count=(len(raw) - HEADER) // REC, offset=HEADER)
    hi = (r['u'] >> np.uint64(32)).astype(np.int64)
    day = ((hi >> 20) & 0xfff) * 10000 + ((hi >> 16) & 0xf) * 100 + ((hi >> 11) & 0x1f)
    minute = ((hi >> 6) & 0x1f) * 60 + (hi & 0x3f)
    if np.any(np.diff(day * 1440 + minute) < 0):
        raise ValueError(f"{path}: records are not in time order")
    return r, day, minute


def fmt(x):
    return repr(float(np.float32(x)))  # the exact stored float32 value, shortest repr


def build(path):
    r, day, minute = load(path)
    rows, report = [], dict(days=0, records=int(len(r)), padded_minutes=int((r['a1'] == 1).sum()),
                            evening_session_days=[], days_with_bars_after_1530=0)
    starts = np.r_[0, np.nonzero(np.diff(day))[0] + 1]
    ends = np.r_[starts[1:], len(day)]
    for a, z in zip(starts, ends):
        d = int(day[a]); m = minute[a:z]
        row = {'Date': f"{d // 10000:04d}-{d // 100 % 100:02d}-{d % 100:02d}"}
        o, h, l, c, v = r['o'][a:z], r['h'][a:z], r['l'][a:z], r['c'][a:z], r['v'][a:z]
        row.update(DO=fmt(o[0]), DH=fmt(h.max()), DL=fmt(l.min()), DC=fmt(c[-1]), DV=fmt(v.sum()), NB=str(z - a))
        b = m - m % 5
        for t, name in zip(BUCKETS, HHMM):
            k = np.nonzero(b == t)[0]
            if len(k):
                row[f'O{name}'] = fmt(o[k[0]]); row[f'H{name}'] = fmt(h[k].max())
                row[f'L{name}'] = fmt(l[k].min()); row[f'C{name}'] = fmt(c[k[-1]]); row[f'V{name}'] = fmt(v[k].sum())
        if m[0] >= 17 * 60:
            report['evening_session_days'].append(row['Date'])
        if (m > 15 * 60 + 30).any():
            report['days_with_bars_after_1530'] += 1
        rows.append(row)
    report['days'] = len(rows); report['first'] = rows[0]['Date']; report['last'] = rows[-1]['Date']
    return rows, report


def header():
    cols = ['Date']
    for n in HHMM:
        cols += [f'O{n}', f'H{n}', f'L{n}', f'C{n}', f'V{n}']
    return cols + ['DO', 'DH', 'DL', 'DC', 'DV', 'NB']


def main(argv=None):
    ap = argparse.ArgumentParser()
    ap.add_argument('--input', required=True); ap.add_argument('--output', required=True); ap.add_argument('--report')
    a = ap.parse_args(argv)
    rows, rep = build(a.input)
    cols = header()
    with open(a.output, 'w', newline='') as f:
        w = csv.DictWriter(f, cols, restval=''); w.writeheader(); w.writerows(rows)
    if a.report:
        json.dump(rep, open(a.report, 'w'), indent=1)
    print(f"{a.output}: {rep['days']} days {rep['first']}..{rep['last']}, {len(cols) - 1} value columns, "
          f"{rep['padded_minutes']} padded minutes, {len(rep['evening_session_days'])} evening-session days", file=sys.stderr)


if __name__ == '__main__':
    main()
