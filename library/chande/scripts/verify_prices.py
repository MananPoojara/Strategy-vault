"""Bounded real-series ledger audit; generated inputs/reports stay outside Git."""
import argparse
import csv
import hashlib
import io
import json
import subprocess
from datetime import date
from decimal import Decimal
from pathlib import Path
from price_oracle import ledger, DEFAULTS


def sha(path):
    h = hashlib.sha256()
    with Path(path).open('rb') as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b''):
            h.update(chunk)
    return h.hexdigest()


def normalized_date(value):
    if value[4:5] == '-':
        return date.fromisoformat(value).isoformat()
    d, m, y = map(int, value.replace('/', '-').split('-'))
    return date(y, m, d).isoformat()


def load(spec, end):
    result = []
    with Path(spec['path']).open(newline='') as stream:
        for raw in csv.DictReader(stream):
            if spec.get('csv_ticker') and raw['Ticker'] != spec['csv_ticker']:
                continue
            raw = {k.lower(): v for k, v in raw.items()}
            d = normalized_date(raw['date'])
            if d > end or d < spec.get('coverage_start', '0001-01-01'):
                continue
            row = {k: raw[k].replace(',', '').strip() for k in ('open', 'high', 'low', 'close')}
            row['date'] = d
            o, h, l, c = (Decimal(row[k]) for k in ('open', 'high', 'low', 'close'))
            assert all(v.is_finite() and v > 0 for v in (o, h, l, c)), (spec['symbol'], d)
            assert l <= o <= h and l <= c <= h, (spec['symbol'], d, row)
            result.append(row)
    assert result and all(a['date'] < b['date'] for a, b in zip(result, result[1:])), spec
    return result


def write_rows(path, rows):
    with path.open('w', newline='') as stream:
        w = csv.DictWriter(stream, fieldnames=['date', 'open', 'high', 'low', 'close'])
        w.writeheader(); w.writerows(rows)


def actual(binary, canonical, method, start, end, params):
    argument = ';'.join(f'{k}={v}' for k, v in params.items())
    cmd = [str(binary), str(canonical), method, str(start), str(end), argument]
    output = subprocess.run(cmd, check=True, text=True, capture_output=True, timeout=45).stdout
    return list(csv.DictReader(io.StringIO(output), delimiter='\t')), cmd


def compare(observed, expected):
    mismatch = []
    if len(observed) != len(expected):
        mismatch.append(dict(field='row_count', actual=len(observed), expected=len(expected)))
    for i, (a, e) in enumerate(zip(observed, expected)):
        for k, v in e.items():
            equal = abs(Decimal(a[k]) - Decimal(v)) <= Decimal('0.00000001') if k in ('entry_px', 'exit_px', 'weight') else a[k] == v
            if not equal:
                mismatch.append(dict(row=i, field=k, actual=a[k], expected=v))
    return mismatch


def dump(path, value):
    path.write_text(json.dumps(value, indent=2, ensure_ascii=False) + '\n')


def edge_checks(binary, out):
    # Artificial diagnostic bars only; never part of production results/counts.
    def diagnostic_rows(closes):
        return [dict(date=date(2020, 1, 1).fromordinal(date(2020, 1, 1).toordinal()+i).isoformat(),
                     open=str(c), high=str(c+1), low=str(c-1), close=str(c)) for i, c in enumerate(closes)]
    stack = diagnostic_rows([100]*12 + [140, 100, 80, 70])
    ravi = diagnostic_rows([100]*72)
    ravi[6].update(open='95', high='96', low='94', close='95')
    for i, c in zip(range(66, 71), [100, 100, 101, 102, 102]):
        ravi[i].update(open=str(c), high=str(c+1), low=str(c-1), close=str(c))
    cases = [('mixed-hold-final-close-reversal', stack, 'ch_bta2_ma_stack', dict(first=2, count=2, slow=10), 0, 15, 2),
        ('warmup-no-entry', stack, 'ch_bta2_ma_stack', dict(first=2, count=2, slow=10), 0, 8, 0),
        ('ravi-exact-one-percent-neutral', ravi, 'ch_bta2_65sma_ravi', dict(fast=5, ravi=1), 0, 71, 0),
        ('ravi-just-below-one-percent-fills', ravi, 'ch_bta2_65sma_ravi', dict(fast=5, ravi=0.9999999), 0, 71, 1),
        ('ravi-just-above-one-percent-neutral', ravi, 'ch_bta2_65sma_ravi', dict(fast=5, ravi=1.0000001), 0, 71, 0),
        ('ravi-no-future-open', ravi, 'ch_bta2_65sma_ravi', dict(fast=5, ravi=0.9999999), 0, 70, 0),
        ('ravi-no-prestart-order', ravi, 'ch_bta2_65sma_ravi', dict(fast=5, ravi=0.9999999), 71, 71, 0)]
    result=[]
    for label, rows, method, params, start, end, wanted in cases:
        canonical=out / f'synthetic-diagnostic-{label}.csv'
        write_rows(canonical, rows)
        e, events=ledger(rows, method, start, end, params)
        a, cmd=actual(binary, canonical, method, start, end, params)
        mm=compare(a,e)
        result.append(dict(test=label, fixture_kind='synthetic diagnostic only; excluded from checked_trades',
            actual_records=len(a), expected_records=len(e), required_records=wanted, row_mismatches=len(mm),
            actual_command=cmd, passed=not mm and len(a)==wanted))
    return result


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--root', type=Path, required=True)
    parser.add_argument('--job', type=Path, required=True)
    parser.add_argument('--binary', type=Path, required=True)
    args = parser.parse_args()
    config = json.loads((args.job / 'verification-config.json').read_text())
    contracts = json.loads((args.job / 'source-review.json').read_text())
    out = args.job / 'verification'; out.mkdir(exist_ok=True)
    compiled = {
        str((args.root / p).resolve()): sha(args.root / p) for p in [
            'Context/guru/backend/src/books/chbta2.rs', 'Context/guru/backend/src/books/chntt.rs',
            'Context/guru/backend/src/connors.rs']}
    checks, coverage, supplemental = [], [], []
    variants = {
        'ch_bta2_ma_stack': [('printed-3-4', {}), ('printed-3-through-7', dict(count=5)),
                            ('research-4-through-6-vs-50', dict(first=4, count=3, slow=50))],
        'ch_bta2_65sma_ravi': [('figure-4-17', {}), ('figure-4-18', dict(ravi=1.5)),
                              ('research-barrier-0-5-no-stop', dict(ravi=0.5))],
    }
    for spec in config['datasets']:
        rows = load(spec, config['to_date'])
        start = next(i for i, r in enumerate(rows) if r['date'] >= config['from_date'])
        end = len(rows) - 1
        canonical = out / f"{spec['symbol']}-observed.csv"
        write_rows(canonical, rows)
        coverage.append(dict(symbol=spec['symbol'], instrument=spec['instrument'],
            loaded_first=rows[0]['date'], loaded_last=rows[-1]['date'], loaded_bars=len(rows),
            trading_first=rows[start]['date'], trading_last=rows[end]['date'], trading_bars=end-start+1,
            warmup_bars=start, source_path=spec['path'], source_sha256=sha(spec['path']),
            membership='Single actual stock; no historical constituent universe is asserted.' if spec['instrument']['kind']=='stock' else 'One actual named price series; no portfolio is asserted.'))
        for method, settings in variants.items():
            for variant, overrides in settings:
                expected, events = ledger(rows, method, start, end, overrides)
                observed, cmd = actual(args.binary, canonical, method, start, end, overrides)
                mismatches = compare(observed, expected)
                stem = f"{method}-{variant}-{spec['symbol']}"
                # Enrich records with dates and explicit fill/mark conventions.
                def enrich(records):
                    return [r | dict(entry_date=rows[int(r['entry'])]['date'],
                        exit_date=rows[int(r['exit'])]['date'], exit_signal_date=rows[int(r['signal_i'])]['date'],
                        exit_fill_mode='terminal_close_mark' if r['open']=='true' else r['entry_kind']) for r in records]
                dump(out / f'{stem}-rust.json', enrich(observed))
                dump(out / f'{stem}-oracle.json', enrich(expected))
                dump(out / f'{stem}-signals.json', events)
                dump(out / f'{stem}-mismatches.json', mismatches)
                checks.append(dict(method=method, symbol=spec['symbol'], variant=variant,
                    parameters=DEFAULTS[method] | overrides, actual_records=len(observed),
                    expected_records=len(expected), row_mismatches=len(mismatches),
                    ohlc_path=spec['path'], ohlc_sha256=sha(spec['path']),
                    canonical_ohlc_path=str(canonical), canonical_ohlc_sha256=sha(canonical),
                    rust_module_sha256=compiled, actual_command=cmd,
                    actual_ledger=str(out / f'{stem}-rust.json'), expected_ledger=str(out / f'{stem}-oracle.json'),
                    mismatches_path=str(out / f'{stem}-mismatches.json'),
                    signals_path=str(out / f'{stem}-signals.json'),
                    closed_records=sum(r['open']=='false' for r in observed),
                    terminal_open_records=sum(r['open']=='true' for r in observed),
                    unfilled_end_signals=sum(e['disposition']=='unfilled_end_of_sample' for e in events),
                    suppressed_signals=sum(e['disposition']=='same_direction_ignored' for e in events)))
                if not overrides:
                    # Causal prefix: completed trades before an earlier cutoff are unchanged.
                    cutoff = next(i for i, r in enumerate(rows) if r['date'] >= '2015-01-01')
                    ep, _ = ledger(rows, method, start, cutoff)
                    ap, pcmd = actual(args.binary, canonical, method, start, cutoff, {})
                    mm = compare(ap, ep)
                    completed = [r for r in ap if r['open']=='false']
                    unchanged = all(r in observed for r in completed)
                    supplemental.append(dict(test='causal_prefix', method=method, symbol=spec['symbol'],
                        actual_command=pcmd, records=len(ap), row_mismatches=len(mm), completed_unchanged=unchanged))
                    if method == 'ch_bta2_65sma_ravi':
                        signal = next(e['signal'] for e in events if e['disposition']=='filled')
                        eu, uevents = ledger(rows, method, start, signal)
                        au, ucmd = actual(args.binary, canonical, method, start, signal, {})
                        um = compare(au, eu)
                        supplemental.append(dict(test='real_unfilled_final_next_open_order', method=method,
                            symbol=spec['symbol'], signal_date=rows[signal]['date'], actual_command=ucmd,
                            actual_records=len(au), expected_records=len(eu), row_mismatches=len(um),
                            terminal_disposition=uevents[-1]['disposition']))
        print(spec['symbol'], 'observed bars', len(rows), flush=True)
    edges=edge_checks(args.binary, out)
    dump(args.job / 'edge-checks.json', edges)
    passed = (all(c['row_mismatches']==0 and c['actual_records']>0 for c in checks)
              and all(c['row_mismatches']==0 and c.get('completed_unchanged', True)
                      and c.get('terminal_disposition','unfilled_end_of_sample')=='unfilled_end_of_sample' for c in supplemental)
              and all(c['passed'] for c in edges))
    report = dict(schema_version=2, source_reviewed=True, implementation_ready=passed,
        admitted=False, ledger_checked=passed, checked_trades=sum(c['actual_records'] for c in checks) if passed else 0,
        verification_scope='bounded stop-free Chande price contracts; single-symbol research, not book performance',
        method_ids=list(variants) if passed else [], source_contracts=contracts['ready_contracts'],
        sources=contracts['sources'], checks=checks, coverage=coverage, supplemental_checks=supplemental, edge_checks=edges,
        binary_path=str(args.binary), binary_sha256=sha(args.binary), rust_module_sha256=compiled,
        driver_sha256=sha(args.root / 'library/chande/rust/actual_price_harness.rs'),
        oracle_sha256={str(Path(__file__).resolve()):sha(__file__),
                      str(Path(__file__).with_name('price_oracle.py').resolve()):sha(Path(__file__).with_name('price_oracle.py'))},
        source_performance_reproduced=False, prices='Supplied observed series, unchanged; native adjustment basis retained. No financing, dividends, slippage, futures contract accounting or pooled population result is asserted.')
    dump(args.job / 'ledger-report.json', report)
    print('checked_trades', report['checked_trades'], 'passed', passed)
    if not passed:
        raise SystemExit(1)


if __name__ == '__main__':
    main()
