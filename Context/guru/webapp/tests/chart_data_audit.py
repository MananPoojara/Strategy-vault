#!/usr/bin/env python3
"""Compare displayed daily OHLC with the staged engine inputs, allowing cent rounding.

This checks consistency between app inputs, not independent vendor provenance.
"""
import csv
import json
import os
from datetime import datetime
from pathlib import Path

APP = Path(__file__).resolve().parents[1]
RUNTIME = Path(os.environ.get('VAULT_RUNTIME_ROOT', Path.home()/'.cache/strategy-vault-webapp'))
# Chart files are git-ignored in the checkout; the runtime's frontend-data copy is what the site serves.
CHART_DATA = Path(os.environ.get('VAULT_FRONTEND_DATA_ROOT') or (RUNTIME/'frontend-data' if (RUNTIME/'frontend-data').is_dir() else Path(__file__).resolve().parents[2]/'frontend/data'))
MARKETS = {'idx_data.csv': ['NIFTY 50', 'NIFTYBANK', 'NIFTYMIDCAP100'],
           'us_data.csv': ['SPX', 'NDX', 'DJI']}
report = []
for filename, tickers in MARKETS.items():
    inputs = {ticker: {} for ticker in tickers}
    with (RUNTIME/'data'/filename).open() as stream:
        for row in csv.DictReader(stream):
            if row['Ticker'] not in inputs:
                continue
            date = datetime.strptime(row['Date'], '%d-%m-%Y').date().isoformat()
            inputs[row['Ticker']][date] = [float(row[field].replace(',', '')) for field in ['Open', 'High', 'Low', 'Close']]
    for ticker in tickers:
        chart = json.loads((CHART_DATA/('ohlc-'+ticker.replace(' ', '')+'.json')).read_text())
        assert all(len(chart[field]) == len(chart['d']) for field in ['o', 'h', 'l', 'c'])
        assert len(set(chart['d'])) == len(chart['d']), (ticker, 'duplicate chart dates')
        bad = []
        for i, date in enumerate(chart['d']):
            expected = inputs[ticker].get(date)
            actual = [chart[field][i] for field in ['o', 'h', 'l', 'c']]
            if expected is None or any(abs(a-b) > 0.0051 for a, b in zip(actual, expected)):
                bad.append({'date': date, 'chart': actual, 'engine': expected})
        missing = len(set(inputs[ticker]) - set(chart['d']))
        report.append({'ticker': ticker, 'chart_bars': len(chart['d']), 'engine_bars': len(inputs[ticker]),
                       'price_mismatches': len(bad), 'engine_days_missing_from_chart': missing, 'examples': bad[:5]})
assert all(r['price_mismatches'] == r['engine_days_missing_from_chart'] == 0 for r in report), report
(APP/'verification').mkdir(exist_ok=True)  # git-ignored output folder; absent on a fresh checkout
(APP/'verification/chart-data-audit.json').write_text(json.dumps(report, indent=2)+'\n')
print(json.dumps({'indices': len(report), 'daily_bars': sum(r['chart_bars'] for r in report), 'mismatches': 0}))
