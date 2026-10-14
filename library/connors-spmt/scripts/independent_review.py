#!/usr/bin/env python3
"""Second-review oracle derived from visually inspected 2007 rule pages.

Uses no conversion oracle or conversion output. Imports only the Rust harness
template, which compiles the actual candidate modules. Evidence stays private.
"""
import argparse
import csv
from datetime import datetime
from decimal import Decimal as D, getcontext
import hashlib
import io
import json
import os
from pathlib import Path
import resource
import subprocess

getcontext().prec = 48
ROOT = Path(__file__).resolve().parents[3]
PAGES = dict(eom_co=10, eom_rsi=14, eom_cc=18, rsi2_4=23, rsi4_25=27,
             rsi_div8=31, rsi_div2=35, rsi_1=39, rsi_2=43, rsi_3=47,
             pctb_1=75, pctb_2=79, price=84, dmi_1=114, dmi_2=118,
             dmi_3=122, dmi_4=126, dmi_5=130, short_4up=144, short_strong=148)
SPECS = [
    dict(symbol='SPY', ticker='SPY', path='/home/server/.cache/strategy-vault-webapp/data/us_data.csv', floor='1993-01-29'),
    dict(symbol='NIFTY50', ticker='NIFTY 50', path='/home/server/.cache/strategy-vault-webapp/data/idx_data.csv', floor='2000-01-01'),
]


def sha(path):
    h = hashlib.sha256()
    with Path(path).open('rb') as f:
        for chunk in iter(lambda: f.read(1048576), b''):
            h.update(chunk)
    return h.hexdigest()


def read_data(spec):
    result = []
    with open(spec['path'], newline='') as f:
        for row in csv.DictReader(f):
            if row['Ticker'] != spec['ticker']:
                continue
            day = datetime.strptime(row['Date'], '%d-%m-%Y').date().isoformat()
            if spec['floor'] <= day <= '2020-09-11':
                result.append(dict(date=day, **{k.lower(): D(row[k]) for k in ['Open', 'High', 'Low', 'Close']}))
    assert result and all(a['date'] < b['date'] for a, b in zip(result, result[1:]))
    assert all(r['low'] <= r['open'] <= r['high'] and r['low'] <= r['close'] <= r['high'] for r in result)
    return result


def mean(values, n):
    return [None if i < n-1 else sum(values[i-n+1:i+1], D(0))/n for i in range(len(values))]


def wilder(values, n):
    # Values start with the first price change / first interbar true range.
    result = [None] * len(values)
    if len(values) >= n:
        value = sum(values[:n], D(0))/n
        result[n-1] = value
        for i in range(n, len(values)):
            value = (value*(n-1) + values[i])/n
            result[i] = value
    return [None] + result


def indicators(rows):
    closes = [r['close'] for r in rows]
    answer = {'ma200': mean(closes, 200), 'ma5': mean(closes, 5)}
    delta = [b-a for a, b in zip(closes, closes[1:])]
    for n in [2, 3, 4]:
        gains = wilder([max(v, D(0)) for v in delta], n)
        losses = wilder([max(-v, D(0)) for v in delta], n)
        answer[f'rsi{n}'] = [None if g is None else D(50) if g+l == 0 else 100*g/(g+l) for g, l in zip(gains, losses)]
    tr, dm = [], []
    for a, b in zip(rows, rows[1:]):
        up, down = b['high']-a['high'], a['low']-b['low']
        dm.append(up if up > max(down, D(0)) else D(0))
        tr.append(max(b['high']-b['low'], abs(b['high']-a['close']), abs(b['low']-a['close'])))
    for n in [2, 3]:
        answer[f'di{n}'] = [None if v is None or t == 0 else 100*v/t for v, t in zip(wilder(dm, n), wilder(tr, n))]
    bands = []
    for i in range(len(rows)):
        if i < 4:
            bands.append(None)
            continue
        w = closes[i-4:i+1]
        avg = sum(w)/5
        deviation = (sum((v-avg)**2 for v in w)/5).sqrt()
        bands.append(None if deviation == 0 else (closes[i]-avg+deviation)/(2*deviation))
    answer['pb'] = bands
    return answer


def setup(key, rows, v, i):
    r, c, rs = rows[i], rows[i]['close'], v['rsi2'][i]
    trend = v['ma200'][i]
    if trend is None or rs is None:
        return False
    def streak(n, test):
        return i >= n-1 and all(test(j) for j in range(i-n+1, i+1))
    def below(name, threshold, n):
        return streak(n, lambda j: v[name][j] is not None and v[name][j] < D(str(threshold)))
    red = lambda j: rows[j]['close'] < rows[j]['open']
    down = lambda j: j > 0 and rows[j]['close'] < rows[j-1]['close']
    up = lambda j: j > 0 and rows[j]['close'] > rows[j-1]['close']
    loc = None if r['high'] == r['low'] else (c-r['low'])/(r['high']-r['low'])
    if key.startswith('short_'):
        if not c < trend or v['ma5'][i] is None or c < v['ma5'][i]:
            return False
        return streak(4, up) if key == 'short_4up' else i >= 2 and c > rows[i-2]['close']*D('1.03') and rs > 80
    if not c > trend:
        return False
    if key.startswith('eom_'):
        if int(r['date'][-2:]) < 25:
            return False
        return rs < 25 if key == 'eom_rsi' else rs <= 65 and streak(2, red if key == 'eom_co' else down)
    if key == 'rsi2_4':
        return rs < 4
    if key == 'rsi4_25':
        return v['rsi4'][i] is not None and v['rsi4'][i] < 25 and rs <= 65
    if key.startswith('rsi_div'):
        n, limit = (8, 15) if key == 'rsi_div8' else (2, 30)
        return i >= n and v['rsi2'][i-n] is not None and c < rows[i-n]['close'] and rs > v['rsi2'][i-n] and rs < limit
    if key in ['rsi_1', 'rsi_2', 'rsi_3']:
        n, days, limit = {'rsi_1': (2, 3, 50), 'rsi_2': (2, 2, 40), 'rsi_3': (3, 2, 45)}[key]
        w = v[f'rsi{n}'][max(0, i-days+1):i+1]
        return len(w) == days and all(x is not None for x in w) and sum(w) < limit
    if key == 'pctb_1':
        return below('pb', 0, 2) and rs < 10
    if key == 'pctb_2':
        return below('pb', .2, 3) and loc is not None and loc <= D('.25') and rs <= 65
    if key == 'price':
        return streak(3, red) and loc is not None and loc <= D('.1') and rs <= 65
    n, days, limit = {'dmi_1': (3, 1, 5), 'dmi_2': (2, 1, 3), 'dmi_3': (2, 2, 10), 'dmi_4': (2, 3, 10), 'dmi_5': (2, 4, 15)}[key]
    return below(f'di{n}', limit, days) and rs <= 65


def records(key, rows, values, start, stop, signals):
    short, entry, result = key.startswith('short_'), None, []
    def record(a, b, opened):
        return dict(entry=str(a), exit=str(b), entry_date=rows[a]['date'], exit_date=rows[b]['date'],
                    direction='-1' if short else '1', entry_price=str(rows[a]['close']), exit_price=str(rows[b]['close']),
                    entry_fill='close', exit_fill='mark' if opened else 'close', entry_signal=str(a), exit_signal=str(b), open=str(opened).lower())
    for i in range(start, stop+1):
        if entry is None:
            if signals[i]:
                entry = i
        elif (rows[i]['close'] < values['ma5'][i] if short else values['rsi2'][i] > 65):
            result.append(record(entry, i, False))
            entry = None
    if entry is not None:
        result.append(record(entry, stop, True))
    return result


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--output', type=Path, required=True)
    out = parser.parse_args().output.resolve()
    assert ROOT not in out.parents and out != ROOT
    out.mkdir(parents=True, exist_ok=True)
    os.sched_setaffinity(0, set(sorted(os.sched_getaffinity(0))[:2]))
    resource.setrlimit(resource.RLIMIT_AS, (1500*1024*1024, 1500*1024*1024))
    backend = ROOT/'Context/guru/backend/src'
    modules = [backend/'books/spmt.rs', backend/'connors.rs']
    hashes = {str(p): sha(p) for p in modules}
    harness = Path(__file__).with_name('ledger_harness.rs').read_text().replace('@CONNORS@', str(modules[1])).replace('@SPMT@', str(modules[0]))
    (out/'independent-harness.rs').write_text(harness)
    binary = out/'independent-rust'
    commands = [['rustc', '--edition=2021', '-C', 'debuginfo=0', '-C', 'opt-level=0', str(out/'independent-harness.rs'), '-o', str(binary)]]
    subprocess.run(commands[0], check=True, capture_output=True)
    checks, samples = [], []
    for spec in SPECS:
        rows = read_data(spec)
        values = indicators(rows)
        infile = out/(spec['symbol']+'-actual-input.csv')
        with infile.open('w', newline='') as f:
            writer = csv.writer(f)
            writer.writerow(['date', 'open', 'high', 'low', 'close'])
            writer.writerows([r['date'], r['open'], r['high'], r['low'], r['close']] for r in rows)
        start = next(i for i, r in enumerate(rows) if r['date'] >= '2010-01-01')
        for key, page in PAGES.items():
            method = 'spmt_'+key
            signals = [setup(key, rows, values, i) for i in range(len(rows))]
            expected_full = records(key, rows, values, start, len(rows)-1, signals)
            assert expected_full, (method, spec['symbol'], 'zero-trade-only check forbidden')
            # Additional actual-price windows lock a first entry on the final
            # bar and restart flat while its original position would be held.
            windows = [('standard', start, len(rows)-1), ('warmup', 0, 198), ('final_entry', start, int(expected_full[0]['entry'])),
                       ('flat_restart', int(expected_full[0]['entry'])+1, int(expected_full[0]['exit']))]
            for label, a, b in windows:
                prefix = out/f'{method}-{spec["symbol"]}-{label}'
                cmd = [str(binary), str(infile), method, str(a), str(b), '', str(prefix)+'-rust-signals.csv']
                commands.append(cmd)
                actual_text = subprocess.run(cmd, check=True, capture_output=True, text=True).stdout
                Path(str(prefix)+'-rust.csv').write_text(actual_text)
                actual = list(csv.DictReader(io.StringIO(actual_text)))
                expected = records(key, rows, values, a, b, signals)
                Path(str(prefix)+'-expected.json').write_text(json.dumps(expected, indent=2))
                mismatches = []
                if len(actual) != len(expected):
                    mismatches.append(dict(actual_count=len(actual), expected_count=len(expected)))
                for idx, (act, exp) in enumerate(zip(actual, expected)):
                    for field in exp:
                        if field.endswith('_price'):
                            equal = abs(D(act[field])-D(exp[field])) <= max(D('1e-9'), abs(D(exp[field]))*D('1e-12'))
                        else:
                            equal = act[field] == exp[field]
                        if not equal:
                            mismatches.append(dict(row=idx, field=field, actual=act[field], expected=exp[field]))
                with open(str(prefix)+'-rust-signals.csv') as f:
                    sigrows = list(csv.DictReader(f))
                assert len(sigrows) == b-a+1
                signal_errors = [r['date'] for r in sigrows if (r['signal'] == 'true') != signals[int(r['bar'])]]
                Path(str(prefix)+'-mismatches.json').write_text(json.dumps(dict(rows=mismatches, signals=signal_errors), indent=2))
                checks.append(dict(method=method, symbol=spec['symbol'], window=label, from_date=rows[a]['date'], to_date=rows[b]['date'],
                                   actual_records=len(actual), expected_records=len(expected), row_mismatches=len(mismatches),
                                   signal_mismatches=len(signal_errors), signal_bars=len(sigrows), ohlc_path=spec['path'], ohlc_sha256=sha(spec['path']),
                                   normalized_input_sha256=sha(infile), source_pages=[3, page], rust_ledger=str(prefix)+'-rust.csv', expected_ledger=str(prefix)+'-expected.json'))
                assert not mismatches and not signal_errors, (method, spec['symbol'], label, mismatches[:2], signal_errors[:2])
            if key in ['eom_co', 'pctb_2', 'dmi_3', 'short_strong', 'rsi2_4']:
                trade = expected_full[0]
                a, b = int(trade['entry']), int(trade['exit'])
                nearby = range(max(start, a-3), min(len(rows), b+2))
                samples.append(dict(method=method, symbol=spec['symbol'], source_pages=[3, page], trade=trade,
                                    bars=[dict(index=i, **{k:str(x) for k,x in rows[i].items()},
                                               **{k:str(x[i]) for k,x in values.items()}, setup=signals[i]) for i in nearby]))
    assert hashes == {str(p): sha(p) for p in modules}, 'candidate changed during compile/run'
    report = dict(schema_version=1, reviewer_id='connors-spmt-handoff-v2-review', source_reviewed=True,
                  implementation_ready=True, ledger_checked=True, checked_trades=sum(c['actual_records'] for c in checks), row_mismatches=0,
                  rust_module_sha256=hashes, oracle_path=str(Path(__file__).resolve()), oracle_sha256=sha(__file__),
                  checks=checks, arithmetic='48-digit Decimal; independent predicates and position loop; no conversion outputs used',
                  convention='One sequential position, flat window start, no exit-bar reentry, final position marked open; entry bar has no earlier exposure and all bars through the exit bar prevent overlapping/replacement entries')
    (out/'ledger-report.json').write_text(json.dumps(report, indent=2)+'\n')
    (out/'manual-sample-calculations.json').write_text(json.dumps(samples, indent=2)+'\n')
    (out/'commands.json').write_text(json.dumps(commands, indent=2)+'\n')
    print(json.dumps(dict(checks=len(checks), checked_trades=report['checked_trades'], row_mismatches=0)))


if __name__ == '__main__':
    main()
