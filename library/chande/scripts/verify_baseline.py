"""Compare all diagnostic fills from independently calculated Decimal rules with Rust.

Uses actual local observed bars. This is implementation parity, not book-study
reproduction or a certification that cash prices are tradable futures contracts.
No production data, source books or NAS vault entries are changed.
"""
import argparse
import csv
import hashlib
import io
import json
import math
import subprocess
from datetime import date
from decimal import Decimal
from pathlib import Path


def reference(rows, length=65, confirmations=3, start=0):
    # Recompute each complete SMA window from Decimal source prices. Determine
    # all close signals first, then execute orders on the following observed open.
    prices = [Decimal(r['Close']) for r in rows]
    sides = []
    for i, price in enumerate(prices):
        if i + 1 < length:
            sides.append(0)
        else:
            average = sum(prices[i+1-length:i+1]) / length
            sides.append(1 if price > average else -1 if price < average else 0)
    targets = []
    for i, side in enumerate(sides):
        targets.append(side if side and i+1 >= confirmations
                       and all(x == side for x in sides[i+1-confirmations:i+1]) else 0)
    fills, current = [], None
    for execution in range(start+1, len(rows)):
        signal = execution-1
        target = targets[signal]
        if not target or current and current['direction'] == str(target):
            continue
        if current:
            current.update(exit_signal_date=rows[signal]['Date'],
                           exit_date=rows[execution]['Date'], exit_price=rows[execution]['Open'], open='false')
            fills.append(current)
        current = dict(direction=str(target), signal_date=rows[signal]['Date'],
                       entry_date=rows[execution]['Date'], entry_price=rows[execution]['Open'])
    if current:
        current.update(exit_signal_date=rows[-1]['Date'], exit_date=rows[-1]['Date'],
                       exit_price=rows[-1]['Close'], open='true')
        fills.append(current)
    return fills


def admitted_rows(path, ticker):
    rows=[]
    with path.open(newline='') as stream:
        for row in csv.DictReader(stream):
            if row['Ticker'] != ticker:
                continue
            parts = row['Date'].split('-')
            row['Date'] = date(int(parts[2]), int(parts[1]), int(parts[0])).isoformat()
            # Canonical CSV uses quoted thousands separators; remove numeric formatting only.
            for key in ('Open','High','Low','Close'):
                row[key]=row[key].replace(',','').strip()
            values = [float(row[k]) for k in ('Open', 'High', 'Low', 'Close')]
            o,h,l,c=values
            assert all(math.isfinite(v) and v>0 for v in values), row['Date']
            assert l <= o <= h and l <= c <= h, row['Date']
            rows.append(row)
    assert rows and all(a['Date'] < b['Date'] for a,b in zip(rows,rows[1:]))
    return rows


def verify(binary, rows, start):
    payload=''.join(f"{r['Date']}\t{r['Open']}\t{r['Close']}\n" for r in rows)
    output=subprocess.run([str(binary), '65', '3', str(start)], input=payload,
                          text=True,capture_output=True,check=True,timeout=30).stdout
    rust=list(csv.DictReader(io.StringIO(output), delimiter='\t'))
    expected=reference(rows,start=start)
    assert len(rust)==len(expected), (len(rust),len(expected))
    for i,(actual, oracle) in enumerate(zip(rust,expected)):
        for key,value in oracle.items():
            if key.endswith('_price'):
                assert abs(Decimal(actual[key])-Decimal(value)) <= Decimal('0.00000001'), (i,key,actual,oracle)
            else:
                assert actual[key]==value, (i,key,actual,oracle)
    return dict(start_index=start, first_signal_allowed=rows[start]['Date'],
                signals_matched=len(rust), closed_legs=sum(r['open']=='false' for r in rust),
                terminal_open_legs=sum(r['open']=='true' for r in rust),
                fields_matched=['direction','signal_date','entry_date','entry_price',
                                'exit_signal_date','exit_date','exit_price','open'],
                first_diagnostic_leg=rust[0] if rust else None,
                last_diagnostic_leg=rust[-1] if rust else None)


def main():
    parser=argparse.ArgumentParser()
    parser.add_argument('--binary',type=Path,required=True)
    parser.add_argument('--india',type=Path,required=True)
    parser.add_argument('--us',type=Path,required=True)
    parser.add_argument('--output',type=Path,required=True)
    args=parser.parse_args()
    report=dict(review_date='2026-10-09',kind='source-rule implementation diagnostic',
                published_performance=False, original_futures_study_reproduced=False,
                oracle='Independent Python Decimal complete-window SMA and prior-close target execution',
                limitations=['Cash index/ETF quotes are not the 23 continuous futures markets in Table 4.2.',
                             'No contract identifiers, point values, rolls or author trade ledger are verified here.',
                             'No sizing, dollar profit, cost-adjusted equity, optimization or significance is claimed.',
                             'Six small synthetic unit fixtures test execution edge cases; actual-data parity uses only observed bars.'],
                rust_source_sha256=hashlib.sha256((Path(__file__).parents[1]/'rust/baseline.rs').read_bytes()).hexdigest(),
                datasets=[])
    for path,ticker in ((args.india,'NIFTY 50'),(args.us,'SPY')):
        before=path.stat()
        source_hash=hashlib.sha256(path.read_bytes()).hexdigest()
        rows=admitted_rows(path,ticker)
        # Separate start-window test proves warming indicators never opens a historical carry position.
        tests=[verify(args.binary,rows,0), verify(args.binary,rows,next(i for i,r in enumerate(rows) if r['Date']>='2010-01-01'))]
        after=path.stat()
        assert before.st_size==after.st_size and before.st_mtime_ns==after.st_mtime_ns
        report['datasets'].append(dict(ticker=ticker,source=str(path),source_sha256=source_hash,
                                      source_unchanged=True,bars=len(rows),first=rows[0]['Date'],last=rows[-1]['Date'],
                                      diagnostic_only=True, comparisons=tests))
    args.output.write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps({d['ticker']:[x['signals_matched'] for x in d['comparisons']] for d in report['datasets']}))


if __name__=='__main__':
    main()
