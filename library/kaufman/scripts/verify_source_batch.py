#!/usr/bin/env python3
"""Bounded independent rule/ledger audit of selected Kaufman TSM4 methods.

Inputs are read-only local source books/CD code and real OHLC files. The oracle
uses Decimal arithmetic, independently expressed source predicates, and a small
position ledger. Rust executes the actual kftsm.rs AND connors.rs indicator/ctx
code through a temporary harness. This does not bypass the live API admission
policy, certify vendor quotes, or reproduce historical contract-dollar studies.
All generated source extracts, harnesses and reports go to the supplied private
output directory. No raw book text or market data belongs in public Git.
"""
from __future__ import annotations

import argparse
import csv
import hashlib
import io
import json
import math
import os
from pathlib import Path
import subprocess
from datetime import datetime
from decimal import Decimal, getcontext

getcontext().prec = 40
D = Decimal
REPO = Path(__file__).resolve().parents[3]

# These are reviewed method identities, not parameter variants or universe runs.
# PDF page numbers refer to the 942-page Books24x7 edition in the supplied vault.
METHODS = {
    'kftsm_ma': {'pages': [278], 'cd': ['Chapter08/TSM(S) Moving average.txt', 'Chapter08/TSM(F) Moving Average signal.txt'], 'params': {'period': 20, 'filter': 0, 'lag': 0}, 'rule': 'SMA slope / previous close in whole percent; latch positive/negative filtered signal; enter/reverse at close.'},
    'kftsm_exp': {'pages': [278, 287], 'cd': ['Chapter08/TSM(S) Exponential smoothing.txt', 'Chapter08/TSM(F) Exp smoothing signal.txt'], 'params': {'period': 20, 'filter': 0, 'lag': 0, 'kind': 0}, 'rule': 'SmoothedAverage seeded by first complete period; subsequent alpha=1/period; latch slope / prior close; close reversals.', 'assumptions': ['Companion system input period=0 is unusable; adjustable period20 is a declared research default, not an author-selected optimum.', 'CD calls the platform function without its initialization code/history. Full-window mean seed and alpha1/period are the explicit engine convention; original TradeStation MaxBarsBack/seed parity is not certified.']},
    'kftsm_lrs': {'pages': [278, 287, 288], 'cd': ['Chapter08/TSM(S) Linear regression slope.txt', 'Chapter08/TSM(F) Linear reg slope.txt', 'Chapter08/TSM(F) Linear reg slope signal.txt'], 'params': {'period': 20, 'filter': 0, 'lag': 0}, 'rule': 'OLS slope of last20 closes against x=1..20; latch slope sign above/below filter; close reversals.'},
    'kftsm_ndb': {'pages': [278, 288], 'cd': ['Chapter08/TSM(S) N-Day breakout 2.txt', 'Chapter08/TSM(F) N-day breakout signal.txt'], 'params': {'period': 20, 'printed': 0}, 'rule': 'High exceeds previous20 highs AND close exceeds prior close for long; low below previous20 lows AND close below prior close for short; close reversals. Current bar excluded from range.'},
    'kftsm_bb_ma': {'pages': [269], 'cd': ['Chapter08/TSM(S) Bollinger MA bands.txt'], 'params': {'period': 20, 'nsd': 2}, 'rule': 'Close crosses outer population-standard-deviation band; fill next open. Held long exits on close crossing below MA; short mirror; no pyramiding.'},
    'kftsm_ma_pct_band': {'pages': [267, 268], 'cd': ['Chapter08/TSM(S) MA percent bands.txt'], 'params': {'period': 20, 'pcband': 2}, 'rule': 'Long at close above SMA20 plus2% of SMA; short below SMA minus2%; hold until opposite signal.'},
    'kftsm_ma_tr_band': {'pages': [267, 268], 'cd': ['Chapter08/TSM(S) MA true range bands.txt'], 'params': {'period': 20, 'band': 5}, 'rule': 'Use previous20 closes and previous20 true ranges; long close above meanClose +5*meanTR, short mirror; hold until reversal.'},
    'kftsm_triple_smooth': {'pages': [273], 'cd': ['Chapter08/TSM(S) Triple smoothing.txt'], 'params': {'len1': 20, 'len2': 10, 'len3': 5}, 'rule': 'Three successive EMA stages, alpha2/(length+1), each seeded with first close; third-stage rising/falling enters/reverses at close.'},
    'kftsm_vol_breakout': {'pages': [273, 274, 656, 657], 'cd': ['Chapter08/TSM(S) Volatility breakout.txt', 'Chapter20/TSM(S) Volatility Breakout.txt'], 'params': {'length': 20, 'factor': 3, 'vlag': 1}, 'rule': 'Close changes by more than3 times previous20-bar mean true range; long/short close reversals.', 'assumptions': ['Default follows book previous-bar volatility; vlag=0 is the distinct Chapter08 CD current-bar calculation. Chapter20 factor1.4 is a configurable variant, not an additional strategy.']},
    'kftsm_keltner': {'pages': [269, 273], 'cd': [], 'params': {'period': 10}, 'rule': 'Mean10 of (H+L+C)/3 with mean10(H-L) bands; close outside upper/lower band enters/reverses.', 'assumptions': ['Original example is commodities; selected stocks/ETF/index are clearly declared OHLC adaptations, not futures-contract return reproductions.']},
    'kftsm_hh_ma': {'pages': [747, 748], 'cd': [], 'params': {'period': 35}, 'rule': 'Closing price crosses simple close-based MA; ignore high=low days; enter/reverse at close.', 'assumptions': ['35 is the source S&P500 table-selected period, not proof that35 is optimal on a new market. Start flat, wait for first eligible crossing; omitted fill time adapted to close.']},
    'kftsm_hh_cross': {'pages': [749, 752], 'cd': [], 'params': {'fast': 17, 'slow': 25}, 'rule': 'Short-period SMA crosses long-period SMA; ignore high=low days; enter/reverse at close.', 'assumptions': ['17/25 is the source S&P500 table-selected pair. Start flat until first eligible crossing; omitted fill time adapted to close.']},
    'kftsm_hh_channel': {'pages': [752, 753], 'cd': [], 'params': {'period': 100}, 'rule': 'Close above highest high of previous100 bars or below lowest low of previous100 bars; close reversal; current bar excluded.', 'assumptions': ['100 is table-selected for S&P500/Nasdaq100; omitted fill time adapted to close. Channel bounds are daily highs/lows, not substituted highest/lowest closes.']},
    'kftsm_mod_3ma': {'pages': [754], 'cd': ['Chapter21/TSM(S) Modified 3MA cross.txt'], 'params': {'length1': 20, 'length2': 5, 'length3': 2}, 'rule': 'SMA5 above SMA20 AND SMA2 rising gives long; both comparisons reversed gives short; close reversal. Source code uses states, not an invented crossover-only gate.'},
    'kftsm_4_9_18': {'pages': [754, 755, 756], 'cd': [], 'params': {'length1': 18, 'length2': 9, 'length3': 4}, 'rule': 'Same modified-three-average rule with9 vs18 and4-day confirming slope.'},
    'kftsm_month_end': {'pages': [374], 'cd': [], 'params': {'day': 0, 'exit_day': 4}, 'rule': 'Buy last observed trading day of a calendar month; exit fourth observed trading day of new month.', 'assumptions': ['Fill at close because source omits time. Calendar boundary uses supplied exchange-observed dates, not future prices. Vendor missing trading sessions can affect dates; no missing bars are invented. No portfolio/dividend/cash-interest model.']},
    'kftsm_hirsch': {'pages': [375], 'cd': [], 'params': {'in_month': 11, 'out_month': 4}, 'rule': 'Buy first observed trading day of November; exit last observed trading day of April.', 'assumptions': ['Fill at close because source omits time. No dividend reinvestment, leverage, cash-interest or tax model; this is price-only calendar adaptation.']},
}


def sha(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open('rb') as f:
        for block in iter(lambda: f.read(1024 * 1024), b''):
            digest.update(block)
    return digest.hexdigest()


def date(value: str) -> str:
    for fmt in ('%Y-%m-%d', '%d-%m-%Y'):
        try:
            return datetime.strptime(value, fmt).date().isoformat()
        except ValueError:
            continue
    raise ValueError(f'Unsupported input date {value!r}')


def read_data(spec: dict) -> list[dict]:
    path = Path(spec['path'])
    rows = []
    with path.open(newline='') as f:
        for source in csv.DictReader(f):
            r = {k.lower(): v for k, v in source.items()}
            if spec.get('csv_ticker') and r['ticker'] != spec['csv_ticker']:
                continue
            day = date(r['date'])
            if day < spec.get('coverage_start', '0000-00-00'):
                continue
            # Vendor CSV values may use quoted thousands separators. Removing
            # formatting commas is parsing, not a change to the observed quote.
            item = {'date': day, **{k: D(r[k].replace(',', '')) for k in ('open', 'high', 'low', 'close')}}
            if not all(item[k].is_finite() and item[k] > 0 for k in ('open', 'high', 'low', 'close')):
                raise ValueError(f'{spec["symbol"]}: invalid OHLC at {day}; no repairs permitted')
            if not item['low'] <= min(item['open'], item['close']) <= max(item['open'], item['close']) <= item['high']:
                raise ValueError(f'{spec["symbol"]}: inconsistent OHLC at {day}; no repairs permitted')
            if rows and rows[-1]['date'] >= day:
                raise ValueError(f'{spec["symbol"]}: dates not unique and ascending')
            rows.append(item)
    if not rows:
        raise ValueError('No actual rows loaded')
    return rows


def average(values: list[D], length: int) -> list[D | None]:
    return [None if i + 1 < length else sum(values[i + 1 - length:i + 1], D(0)) / length for i in range(len(values))]


def exponent(values: list[D], alpha: D) -> list[D]:
    output = [values[0]]
    for value in values[1:]:
        output.append(output[-1] + alpha * (value - output[-1]))
    return output


def crossed(a, b, i: int, sign: int) -> bool:
    if i == 0 or any(v is None for v in (a[i], b[i], a[i-1], b[i-1])):
        return False
    return sign*(a[i] - b[i]) > 0 and sign*(a[i-1] - b[i-1]) <= 0


def oracle(method: str, rows: list[dict], first: int, last: int, params: dict) -> list[dict]:
    """Source predicates evaluated separately from the Rust implementation."""
    p = {k: D(str(v)) for k, v in params.items()}
    c, h, l, o = ([r[key] for r in rows] for key in ('close', 'high', 'low', 'open'))
    size = len(rows)
    position = None
    ledger = []

    def target(i: int, side: int, price: D, fill: str = 'close'):
        nonlocal position
        if i < first or (position and side == position['dir']):
            return
        if position:
            position.update(exit=i, exit_px=price, open=False)
            ledger.append(position)
            position = None
        if side and i < last:
            position = dict(entry=i, entry_px=price, dir=side, entry_kind=fill)

    tr = [h[0]-l[0]] + [max(h[i]-l[i], abs(h[i]-c[i-1]), abs(l[i]-c[i-1])) for i in range(1, size)]
    if method in ('kftsm_ma', 'kftsm_exp', 'kftsm_lrs', 'kftsm_ndb'):
        n = int(p['period'])
        if method == 'kftsm_ma':
            trend = average(c, n)
        elif method == 'kftsm_exp':
            kind = int(p['kind'])
            if kind == 0:
                trend = [None]*(n-1) + [sum(c[:n], D(0))/n]
                for i in range(n, size): trend.append((trend[-1]*(n-1)+c[i])/n)
            else:
                trend = exponent(c, D(2)/(n+1) if kind == 1 else D(1)/(n+1))
        elif method == 'kftsm_lrs':
            xs = [D(i) for i in range(1, n+1)]
            xm = sum(xs)/n
            denom = sum((v-xm)**2 for v in xs)
            trend = [None]*(n-1)
            for i in range(n-1, size):
                window = c[i+1-n:i+1]
                mean = sum(window)/n
                trend.append(sum((x-xm)*(y-mean) for x,y in zip(xs,window))/denom)
        side = 0
        for i in range(size):
            if method == 'kftsm_ndb':
                window = n if p['printed'] == 0 else max(1, n-1)
                if i >= window:
                    if h[i] > max(h[i-window:i]) and c[i] > c[i-1]: side = 1
                    if l[i] < min(l[i-window:i]) and c[i] < c[i-1]: side = -1
            else:
                j = i - int(p['lag'])
                change = None
                if j > 0 and trend[j] is not None:
                    if method == 'kftsm_lrs': change = trend[j]
                    elif trend[j-1] is not None: change = (trend[j]-trend[j-1])*100/c[j-1]
                if change is not None:
                    if change > p['filter']: side = 1
                    elif change < -p['filter']: side = -1
            if first <= i <= last and side: target(i, side, c[i])
    elif method == 'kftsm_bb_ma':
        n = int(p['period']); ma = average(c,n)
        sd = [None if m is None else (sum((v-m)**2 for v in c[i+1-n:i+1])/n).sqrt() for i,m in enumerate(ma)]
        upper = [None if m is None else m+p['nsd']*s for m,s in zip(ma,sd)]
        lower = [None if m is None else m-p['nsd']*s for m,s in zip(ma,sd)]
        pending = None
        for i in range(first,last+1):
            if pending is not None: target(i,pending,o[i],'open')
            side = position['dir'] if position else 0
            if crossed(c,upper,i,1) and side != 1: pending=1
            elif crossed(c,lower,i,-1) and side != -1: pending=-1
            elif side and crossed(c,ma,i,-side): pending=0
            else: pending=None
    elif method in ('kftsm_ma_pct_band','kftsm_ma_tr_band','kftsm_keltner'):
        n=int(p['period'])
        if method=='kftsm_keltner':
            ma=average([(a+b+d)/3 for a,b,d in zip(h,l,c)],n); width=average([a-b for a,b in zip(h,l)],n)
        else:
            ma=average(c,n)
            width=average(tr,n) if method=='kftsm_ma_tr_band' else [None if m is None else m*p['pcband']/100 for m in ma]
        for i in range(first,last+1):
            j=i-1 if method=='kftsm_ma_tr_band' else i
            if j < 0 or ma[j] is None: continue
            w=width[j]*(p['band'] if method=='kftsm_ma_tr_band' else 1)
            if c[i]>ma[j]+w:target(i,1,c[i])
            elif c[i]<ma[j]-w:target(i,-1,c[i])
    elif method=='kftsm_triple_smooth':
        sm=c
        for key in ('len1','len2','len3'):sm=exponent(sm,D(2)/(p[key]+1))
        for i in range(max(first,1),last+1):
            if sm[i]>sm[i-1]:target(i,1,c[i])
            elif sm[i]<sm[i-1]:target(i,-1,c[i])
    elif method=='kftsm_vol_breakout':
        vol=average(tr,int(p['length']));lag=int(p['vlag'])
        for i in range(max(first,1),last+1):
            if vol[i-lag] is None:continue
            v=vol[i-lag]*p['factor']
            if c[i]-c[i-1]>v:target(i,1,c[i])
            elif c[i-1]-c[i]>v:target(i,-1,c[i])
    elif method in ('kftsm_hh_ma','kftsm_hh_cross'):
        a=c if method=='kftsm_hh_ma' else average(c,int(p['fast']))
        b=average(c,int(p['period'] if method=='kftsm_hh_ma' else p['slow']))
        for i in range(first,last+1):
            if h[i]==l[i]:continue
            if crossed(a,b,i,1):target(i,1,c[i])
            elif crossed(a,b,i,-1):target(i,-1,c[i])
    elif method=='kftsm_hh_channel':
        n=int(p['period'])
        for i in range(max(first,n),last+1):
            if c[i]>max(h[i-n:i]):target(i,1,c[i])
            elif c[i]<min(l[i-n:i]):target(i,-1,c[i])
    elif method in ('kftsm_mod_3ma','kftsm_4_9_18'):
        slow,fast,confirm=[average(c,int(p[k])) for k in ('length1','length2','length3')]
        for i in range(max(first,1),last+1):
            if any(v is None for v in (slow[i],fast[i],confirm[i],confirm[i-1])):continue
            if fast[i]>slow[i] and confirm[i]>confirm[i-1]:target(i,1,c[i])
            elif fast[i]<slow[i] and confirm[i]<confirm[i-1]:target(i,-1,c[i])
    elif method=='kftsm_month_end':
        month=[r['date'][:7] for r in rows];hold=0
        for i in range(first,last+1):
            if position:
                if month[i]!=month[i-1]:hold=1
                elif hold:hold+=1
                if hold>=int(p['exit_day']):target(i,0,c[i]);hold=0
            else:
                after=i+1+int(p['day'])
                if after<size and month[after]!=month[i] and month[after-1]==month[i]:target(i,1,c[i]);hold=0
    elif method=='kftsm_hirsch':
        month=[int(r['date'][5:7]) for r in rows]
        for i in range(first,last+1):
            if position and month[i]==int(p['out_month']) and i+1<size and month[i+1]!=month[i]:target(i,0,c[i])
            elif not position and i>0 and month[i]==int(p['in_month']) and month[i-1]!=month[i]:target(i,1,c[i])
    else:
        raise ValueError(f'Unreviewed method {method}')
    if position:
        position.update(exit=last,exit_px=c[last],open=True);ledger.append(position)
    return ledger


def build_harness(output_dir: Path) -> Path:
    """Compile the actual modules, rather than another Rust reimplementation."""
    src=REPO/'Context/guru/backend/src'
    harness=output_dir/'actual-kftsm-harness.rs'
    harness.write_text('''#![allow(dead_code)]
#[derive(Clone)]
pub struct Row { pub ticker:String, pub date:String, pub open:f64, pub high:f64,
 pub low:f64, pub close:f64, pub volume:Option<f64> }
#[path=CONNORS_PATH] mod connors;
#[path=KAUFMAN_PATH] mod kftsm;
mod books {
 use crate::connors::{Ctx,Params,ParamDef,StrategyDef,Trade};
 pub fn catalogue()->Vec<StrategyDef>{vec![]}
 pub fn family_of(_: &str)->Option<(&'static str,&'static str,&'static str)>{None}
 pub fn params(_: &str)->Option<Vec<ParamDef>>{None}
 pub fn run(_: &str,_: &Ctx,_:usize,_:usize,_:&Params)->Option<Vec<Trade>>{None}
}
fn main(){
 let a:Vec<String>=std::env::args().collect();
 let text=std::fs::read_to_string(&a[1]).unwrap();
 let rows:Vec<Row>=text.lines().skip(1).map(|line|{
  let p:Vec<&str>=line.split(',').collect();
  Row{ticker:"REAL".into(),date:p[0].into(),open:p[1].parse().unwrap(),
   high:p[2].parse().unwrap(),low:p[3].parse().unwrap(),close:p[4].parse().unwrap(),volume:None}
 }).collect();
 let x=connors::Ctx::new(&rows,&rows,&[]);
 let params:connors::Params=if a[5].is_empty(){connors::Params::new()}else{
  a[5].split(';').map(|s|{let(k,v)=s.split_once('=').unwrap();(k.to_string(),v.parse().unwrap())}).collect()
 };
 let ts=kftsm::run(&a[2],&x,a[3].parse().unwrap(),a[4].parse().unwrap(),&params).expect("Candidate rejected");
 println!("entry,exit,dir,entry_px,exit_px,open,entry_kind");
 for t in ts {assert_eq!(t.legs.len(),1);assert!(t.exits.is_empty());
  println!("{},{},{},{:.17},{:.17},{},{}",t.legs[0].i,t.exit_i,t.dir,t.legs[0].px,t.exit_px,t.open,t.entry_kind);
 }
}
'''.replace('CONNORS_PATH',json.dumps(str(src/'connors.rs'))).replace('KAUFMAN_PATH',json.dumps(str(src/'books/kftsm.rs'))))
    binary=output_dir/'actual-kftsm-harness'
    subprocess.run(['rustc','--edition=2021','-C','opt-level=1',str(harness),'-o',str(binary)],check=True,timeout=180,capture_output=True,text=True)
    tests=output_dir/'actual-kftsm-tests'
    subprocess.run(['rustc','--edition=2021','--test',str(harness),'-o',str(tests)],check=True,timeout=180,capture_output=True,text=True)
    check=subprocess.run([str(tests),'kftsm::tests','--test-threads=1'],check=True,timeout=60,capture_output=True,text=True)
    (output_dir/'rust-tests.txt').write_text(check.stdout+check.stderr)
    return binary


def main():
    ap=argparse.ArgumentParser();ap.add_argument('--manifest',type=Path,required=True);ap.add_argument('--output',type=Path,required=True)
    args=ap.parse_args();cfg=json.loads(args.manifest.read_text())
    output=args.output.resolve();output.parent.mkdir(parents=True,exist_ok=True)
    source_root=Path(cfg['source_root']);book=source_root/cfg['book_path'];cd=source_root/cfg['cd_path']
    selected=cfg.get('methods',list(METHODS))
    if not selected or any(m not in METHODS for m in selected):raise ValueError('Only explicitly reviewed methods may run')
    book_hash=sha(book)
    sources={}
    for method in selected:
        contract=METHODS[method]
        records=[]
        for name in contract['cd']:
            path=cd/name
            records.append({'path':str(path),'sha256':sha(path)})
        sources[method]={'pdf_path':str(book),'pdf_sha256':book_hash,'pdf_pages':contract['pages'],'cd_sources':records,
                         'rule':contract['rule'],'default_parameters':contract['params'],'declared_assumptions':contract.get('assumptions',[])}
    rust_sources={str(p):sha(p) for p in [REPO/'Context/guru/backend/src/books/kftsm.rs',REPO/'Context/guru/backend/src/connors.rs']}
    binary=build_harness(output.parent)
    report={'schema_version':1,'author':'Perry J. Kaufman','book':'New Trading Systems and Methods, Fourth Edition (2005) + companion CD',
            'source_reviewed':True,'implementation_ready':False,'admitted':False,'ledger_checked':False,'checked_trades':0,
            'original_studies_reproduced':False,'vendor_quotes_certified':False,'independent_oracle':'Python Decimal40 source predicates versus actual compiled Rust modules',
            'common_adaptations':['One position per symbol; no pyramiding or pooled portfolio sizing.','Flat at selected start; earlier actual bars warm indicators.','Terminal position stays open, excluded from closed-trade statistics.','Cash-index short exposure is hypothetical, without futures basis, borrow or funding.','No original contract dollars, source-book P&L, dividends or interest are substituted for computed price returns.'],
            'source_contracts':sources,'rust_module_sha256':rust_sources,'checks':[],
            'evidence':[str(output.parent/'rust-tests.txt'),str(output)],'summary':''}
    for spec in cfg['datasets']:
        source_path=Path(spec['path']);before=source_path.stat();digest=sha(source_path);rows=read_data(spec)
        first=next(i for i,r in enumerate(rows) if r['date']>=cfg['from']);last=max(i for i,r in enumerate(rows) if r['date']<=cfg['to'])
        canonical=output.parent/(spec['symbol']+'-actual.csv')
        with canonical.open('w',newline='') as f:
            w=csv.writer(f);w.writerow(['date','open','high','low','close'])
            w.writerows([r['date'],r['open'],r['high'],r['low'],r['close']] for r in rows)
        for method in selected:
            params=METHODS[method]['params'];expected=oracle(method,rows,first,last,params)
            command=[str(binary),str(canonical),method,str(first),str(last),';'.join(f'{k}={v}' for k,v in params.items())]
            response=subprocess.run(command,check=True,capture_output=True,text=True,timeout=60)
            actual=list(csv.DictReader(io.StringIO(response.stdout)));mismatches=[]
            if len(actual)!=len(expected):mismatches.append({'kind':'count','expected':len(expected),'actual':len(actual)})
            for n,(got,want) in enumerate(zip(actual,expected)):
                for key in ('entry','exit','dir'):
                    if int(float(got[key]))!=want[key]:mismatches.append({'trade':n,'field':key,'actual':got[key],'expected':want[key]})
                for key in ('entry_px','exit_px'):
                    if abs(D(got[key])-want[key])>max(D('0.00000001'),abs(want[key])*D('0.000000000001')):
                        mismatches.append({'trade':n,'field':key,'actual':got[key],'expected':str(want[key])})
                if (got['open']=='true')!=want['open'] or got['entry_kind']!=want['entry_kind']:
                    mismatches.append({'trade':n,'field':'fill/open','actual':[got['entry_kind'],got['open']],'expected':[want['entry_kind'],want['open']]})
            returns=[D(t['dir'])*(t['exit_px']/t['entry_px']-1) for t in expected if not t['open']]
            check={'method':method,'symbol':spec['symbol'],'instrument':spec['instrument'],'ohlc_path':str(source_path),'ohlc_sha256':digest,
                   'loaded_bars':len(rows),'warmup_first':rows[0]['date'],'from':rows[first]['date'],'to':rows[last]['date'],
                   'parameters':params,'expected_records':len(expected),'actual_records':len(actual),'closed_trades':len(returns),
                   'row_mismatches':len(mismatches),'first_mismatches':mismatches[:5],
                   'closed_gross_return_sum':str(sum(returns,D(0))),'mean_closed_gross_return':str(sum(returns,D(0))/len(returns)) if returns else None}
            report['checks'].append(check);report['checked_trades']+=len(expected)
            print(json.dumps({'method':method,'symbol':spec['symbol'],'records':len(expected),'mismatches':len(mismatches)}),flush=True)
        after=source_path.stat()
        if (before.st_size,before.st_mtime_ns)!=(after.st_size,after.st_mtime_ns):raise RuntimeError('Original OHLC file changed during read-only audit')
    if any(sha(Path(path))!=digest for path,digest in rust_sources.items()):
        raise RuntimeError('A Rust module changed during the audit; rerun on stable source')
    okay=all(c['row_mismatches']==0 for c in report['checks'])
    report.update(implementation_ready=okay,ledger_checked=okay and report['checked_trades']>0,
                  methods_reviewed=len(selected),cases_checked=len(report['checks']),
                  summary=f'{len(selected)} source-reviewed method identities; {len(report["checks"])} actual-data cases; {report["checked_trades"]} ledger records; admission remains a separate reviewed step.')
    temp=output.with_suffix(output.suffix+'.tmp');temp.write_text(json.dumps(report,indent=2)+'\n');temp.replace(output)
    print(json.dumps({'output':str(output),'methods':len(selected),'cases':len(report['checks']),'records':report['checked_trades'],'passed':okay}))
    if not okay:raise SystemExit(1)


if __name__=='__main__':main()
