#!/usr/bin/env python3
"""Independent source-based Decimal oracle for the approved SPMT daily subset.

Only two actual instrument populations are applicable: SPY and declared Nifty
cash-index timing. The supplied stock files are hashed and explicitly rejected,
not substituted. All normalized inputs, source excerpts and result files stay
in --output outside the repository. This script does not contact any live API.
"""
from __future__ import annotations
import argparse
import csv
from decimal import Decimal, getcontext
from datetime import datetime
import hashlib
import io
import json
import math
import os
from pathlib import Path
import resource
import subprocess

getcontext().prec = 40
D = Decimal
ROOT = Path(__file__).resolve().parents[3]
SOURCE = Path('/run/user/1000/gvfs/smb-share:server=192.168.4.50,share=share/Manan Pujara/Book-to-rust/Books/Larry Connors/Larry Connors - S&P Market Timing Course For E-mini & Options Traders (Manual 350 pgs, Feb 2007), $7500 (tradingmarkets.com)/S&P Market Timing Manual.pdf')
DATASETS = [
    dict(symbol='SPY', path='/home/server/.cache/strategy-vault-webapp/data/us_data.csv', ticker='SPY', kind='etf', market='us', floor=None),
    dict(symbol='NIFTY50', path='/home/server/.cache/strategy-vault-webapp/data/idx_data.csv', ticker='NIFTY 50', kind='cash_index', market='india', floor='2000-01-01'),
]
REJECTED_STOCKS = [
    dict(symbol='RELIANCE', path='/home/server/.cache/strategy-vault/market-data/daily/nifty50/9e95498193cc63fcd447.csv', market='india'),
    dict(symbol='AAPL', path='/home/server/.cache/strategy-vault/us-market-data/daily/c220fe04bfd335ae85d6e1ad19464b3aa58026473f102f47012d4618f3bc5319.csv', market='us'),
]

# Manually interpreted from the scanned rule pages, before ledger execution.
# source_entry refers to today t; all lookbacks count trading bars.
METHODS = {}
def contract(key, page, family, params, entry, short=False, assumption=None):
    shared = {'trend': 200, 'ma': 5} if short else {'trend': 200, 'rsi_n': 2, 'exit_rsi': 65}
    METHODS['spmt_' + key] = dict(book='spmt', pages=[3, page], family=family,
        variant=key, params=shared | params, source_entry=entry,
        entry='At the close; ' + ('C < SMA200, C >= SMA5; ' if short else 'C > SMA200; ') + entry,
        exit='C < SMA5 at close' if short else 'RSI2 > 65 at close',
        direction=-1 if short else 1, entry_fill='close', exit_fill='close',
        population='actual_SPY_or_declared_cash_index_timing',
        assumptions=([assumption] if assumption else []),
        duplicate_stc9=key, new_algorithm_increment=0)

contract('eom_co', 10, 'spmt_eom_red', {'dom':25,'bars':2}, 'Calendar day >=25; two consecutive C<O bars; RSI2<=65',
    assumption='Consecutive red bars ending today interprets page10; the parallel page18 explicitly says consecutive. The red-bar run may begin before the25th.')
contract('eom_rsi', 14, 'spmt_eom_rsi', {'dom':25,'rsi_lt':25}, 'Calendar day >=25; RSI2<25')
contract('eom_cc', 18, 'spmt_eom_down', {'dom':25,'bars':2}, 'Calendar day >=25; two consecutive lower closes; RSI2<=65')
contract('rsi2_4', 23, 'spmt_rsi_level', {'rsi_lt':4}, 'RSI2<4')
contract('rsi4_25', 27, 'spmt_rsi_level', {'slow_n':4,'slow_lt':25}, 'RSI4<25; RSI2<=65')
contract('rsi_div8', 31, 'spmt_rsi_divergence', {'look':8,'rsi_lt':15}, 'C<C[t-8]; RSI2>RSI2[t-8]; RSI2<15')
contract('rsi_div2', 35, 'spmt_rsi_divergence', {'look':2,'rsi_lt':30}, 'C<C[t-2]; RSI2>RSI2[t-2]; RSI2<30')
for key,page,n,days,level in [('rsi_1',39,2,3,50),('rsi_2',43,2,2,40),('rsi_3',47,3,2,45)]:
    contract(key,page,'spmt_rsi_sum',{'sum_n':n,'sum_days':days,'sum_lt':level},f'Sum of last{days} RSI({n}) values <{level}')
contract('pctb_1',75,'spmt_pctb_rsi',{'pctb_n':5,'pctb_k':1,'days':2,'pctb_lt':0,'rsi_lt':10},'Last two %b(5,1SD)<0; RSI2<10')
contract('pctb_2',79,'spmt_pctb_range',{'pctb_n':5,'pctb_k':1,'days':3,'pctb_lt':0.2,'loc':25},'Last three %b(5,1SD)<0.2; (C-L)/(H-L)<=0.25; RSI2<=65')
contract('price',84,'spmt_price',{'bars':3,'loc':10},'Last three C<O bars; (C-L)/(H-L)<=0.10; RSI2<=65')
for k,page,n,days,level in [(1,114,3,1,5),(2,118,2,1,3),(3,122,2,2,10),(4,126,2,3,10),(5,130,2,4,15)]:
    contract(f'dmi_{k}',page,'spmt_dmi',{'di_n':n,'days':days,'di_lt':level},f'Last{days} +DI({n})<{level}; RSI2<=65')
contract('short_4up',144,'spmt_short_up',{'up':4},'Four consecutive higher closes',short=True)
contract('short_strong',148,'spmt_short_return',{'rise':3,'rsi_n':2,'rsi_gt':80},'C/C[t-2]-1 >0.03; RSI2>80',short=True)

HELD = {}
def hold(ids, pages, entry, exit, needs, reason, params=None):
    for key in ids:
        HELD['spmt_'+key] = dict(book='spmt', pages=pages, entry=entry, exit=exit,
            required_inputs=needs, reason=reason, parameters=params or {}, ready=False)
hold(['vix200'],[3,52],'C>SMA200; VIX>=1.05*SMA10(VIX) for3 days; RSI2<10','RSI2>65 at close',['CBOE VIX close'], 'No verified source-exchange VIX inputs in approved dataset specs',{'stretch_pct':5,'sma':10,'days':3,'rsi':2,'entry_rsi':10,'exit_rsi':65})
hold(['vix2'],[3,56],'C>SMA200; RSI2(VIX)>90; VIX.O>previous VIX.C; RSI2<30','RSI2>65 at close',['CBOE VIX open/close'],'Unverified exchange-specific VIX',{'vix_rsi':2,'vix_gt':90,'spy_rsi':2,'spy_lt':30,'exit_rsi':65})
hold(['vix4'],[3,60],'C>SMA200; VIX up5 days; RSI2<30','RSI2>65 at close',['CBOE VIX close'],'Unverified exchange-specific VIX',{'up_days':5,'rsi':2,'entry_lt':30,'exit_gt':65})
hold(['vix7'],[3,64],'C>SMA200; VIX up4 days; VIX.O>previous VIX.H; RSI2<30','RSI2>65 at close',['CBOE VIX OHLC'],'Unverified exchange-specific VIX',{'up_days':4,'rsi':2,'entry_lt':30,'exit_gt':65})
hold(['vix8'],[3,69],'C>SMA200; VIX>=1.10*SMA10(VIX) for2 days; RSI2<30','RSI2>65 at close',['CBOE VIX close'],'Unverified exchange-specific VIX',{'stretch_pct':10,'sma':10,'days':2,'rsi':2,'entry_lt':30,'exit_gt':65})
for k,page,level,days in [(1,89,1,4),(2,94,1.2,4),(3,98,1,3),(4,103,1.1,3),(5,108,1.1,2)]:
    hold([f'trin_{k}'],[3,page],f'C>SMA200; NYSE TRIN>{level} for{days} days; RSI2<50','RSI2>65 at close',['NYSE TRIN'],'Missing verified exchange-specific TRIN',{'trend':200,'trin_gt':level,'days':days,'rsi':2,'entry_lt':50,'exit_gt':65})
for key,page,level,days in [('short_vix53',135,5,3),('short_vix101',140,10,1)]:
    hold([key],[3,page],f'C<SMA200; VIX<=(1-{level}/100)*SMA10(VIX) for{days} days; RSI2>80; C>=SMA5','C<SMA5 at close',['CBOE VIX close'],'Unverified exchange-specific VIX',{'trend':200,'vix_sma':10,'stretch_pct':level,'days':days,'rsi':2,'entry_gt':80,'exit_ma':5})
hold(['nyse_adv'],[3,153],'C>SMA200; RSI2(SMA7(NYSE advancing issues)) sum of last2<12; RSI2<=65','RSI2>65 at close',['NYSE advancing issues'],'Missing verified NYSE breadth; NSE substitution unreviewed',{'trend':200,'adv_ma':7,'adv_rsi':2,'sum_days':2,'sum_lt':12,'cap':65,'exit_gt':65})
hold(['tick'],[3,158],'C>SMA200; SMA5(NYSE TICK)<150 for3 days; RSI2<=75','RSI2>75 at close',['NYSE daily closing TICK'],'Missing actual NYSE TICK; literal150 is not an NSE breadth threshold',{'trend':200,'tick_ma':5,'tick_lt':150,'days':3,'cap':75,'exit_gt':75})
hold(['multi5','multi9'],[6],'5 daily or9 cumulative signals from full30-method long basket','RSI2>65 at close',['full long signal basket','allocation/recurrence contract'],'Partial signal counting, equal legs and6-leg cap are unverified; complete aux inputs missing')
hold(['mts'],[164,165,166],'First signal buy1 E-mini; later cumulative5/8/11 plus new signal add2/3/4 contracts on later days; shorts1/day max4','General exit not stated on rules pages; shared RSI/MA inferred in existing code',['full34-method basket','dated futures contracts','allocation/exit contract'],'Complete signal set, futures accounting and inferred exits unverified',{'long_contracts':[1,2,3,4],'cum_thresholds':[5,8,11],'short_max':4})
hold(['eom_und','eom_puts'],[168,169],'Above SMA200; day>=25; two lower closes; sell nearest-month SPY puts about2 strikes/2% away','Example RSI76.23 exits next morning; general numeric threshold absent',['exit contract','actual option contracts for options P&L'],'Existing underlying RSI>65 next-open runner is an inferred contract, not a complete source rule')
hold(['put_sale','itm_calls'],[216,218,219,222,230],'Conditional naked puts/spreads/ITM calls on market timing signals','RSI/market-timing exit',['exact options prices/expiries/strikes','allocation'],'Outside daily price-signal batch; no options P&L from underlying')
hold(['dt_gapdown','dt_unch','dt_gapup'],[240,254,257,259,260],'Gap-dependent first15-minute breakout or pullback buy zone','Intraday stops/trails/retries, flat at close',['actual15-minute bars','ticks/currency','causal path'],'Outside daily batch; no intraday data or source-supported daily path')
hold(['discretion'],[189,231],'Discretionary signal strength/ranking and allocations','Discretionary',['complete mechanical allocation/execution contract'],'Incomplete mechanical rule')

ASSUMPTIONS = [
    'Wilder RSI uses arithmetic mean of first n price changes, alpha1/n thereafter; flat up/down gives50. The source does not print initialization; this is an explicit research convention.',
    '+DMI uses Wilder smoothing of positive dominant directional move and true range starting at bar1; ties give zero directional move. This initialization is a declared convention.',
    'Rust +DM treats a dominance difference within EPSILON*(abs(current high)+abs(prior high)+abs(prior low)+abs(current low)) as a subtraction-roundoff tie. This scoped precision convention matches independent Decimal equal moves without changing shared STC IX DMI.',
    '%b uses population SD (divisor n), SMA of n closes and the printed1SD width. The source does not print the divisor; population SD is a declared convention. Zero SD gives no %b signal.',
    'SMA includes the current close; full original admitted warmup is retained. Nifty bars before2000-01-01 are excluded even from warmup.',
    'Bottom-range equality is accepted; a zero H-L range gives no bottom-range signal. No rounding before thresholds.',
    'Calendar day is local date day of month; lookbacks use chronological trading bars.',
    'Flat at selected start; one position per method/instrument; no pyramiding, no same-close reentry after exit. Source does not print a portfolio allocation.',
    'A final-close entry is retained as open with mark at that close. Unresolved positions are marked open, never turned into closed trades.',
    'Close orders at the closing price follow page3. No option P&L or futures dollar/tick costs; audited ledgers contain prices/directions only. Cash-index short exposure is hypothetical.',
    'Actual vendor OHLC are not an independent quote/corporate-action certification. No dividends, financing, portfolio sizing, book-performance or historical membership claim.',
]

def sha(path):
    h=hashlib.sha256()
    with Path(path).open('rb') as f:
        for block in iter(lambda:f.read(1<<20), b''): h.update(block)
    return h.hexdigest()

def load(spec, end):
    rows=[]; removed=0
    with Path(spec['path']).open(newline='') as f:
        for r in csv.DictReader(f):
            if 'Ticker' in r and r['Ticker'] != spec['ticker']: continue
            s=r.get('Date',r.get('date'))
            day=datetime.strptime(s,'%d-%m-%Y' if 'Date' in r else '%Y-%m-%d').date().isoformat()
            if spec.get('floor') and day < spec['floor']: removed+=1; continue
            if day>end: continue
            values=[D(r.get(k,r.get(k.lower()))) for k in ['Open','High','Low','Close']]
            rows.append(dict(date=day,o=values[0],h=values[1],l=values[2],c=values[3]))
    if not rows: raise ValueError(f'no rows for {spec}')
    assert all(rows[i]['date']<rows[i+1]['date'] for i in range(len(rows)-1)), 'nonchronological or duplicate data'
    invalid=[r['date'] for r in rows if not all(v.is_finite() and v>0 for k,v in r.items() if k!='date') or not r['l']<=r['o']<=r['h'] or not r['l']<=r['c']<=r['h']]
    return rows,removed,invalid

def sma(a,n):
    # Direct windows intentionally independent of Rust's running-sum algorithm.
    return [None if i+1<n else sum(a[i+1-n:i+1])/n for i in range(len(a))]

def smooth_changes(a,n):
    out=[None]*len(a)
    if len(a)<n+1:return out
    value=sum(a[1:n+1])/n; out[n]=value
    for i in range(n+1,len(a)):
        value += (a[i]-value)/n;out[i]=value
    return out

def rsi(a,n):
    changes=[D(0)]+[a[i]-a[i-1] for i in range(1,len(a))]
    gain=smooth_changes([max(v,D(0)) for v in changes],n)
    loss=smooth_changes([max(-v,D(0)) for v in changes],n)
    return [None if u is None else D(50) if u==0 and d==0 else D(100) if d==0 else 100*u/(u+d) for u,d in zip(gain,loss)]

def pdi(rows,n):
    moves=[D(0)];ranges=[D(0)]
    for prev,r in zip(rows,rows[1:]):
        up=r['h']-prev['h'];down=prev['l']-r['l']
        moves.append(up if up>0 and up>down else D(0))
        ranges.append(max(r['h']-r['l'],abs(r['h']-prev['c']),abs(r['l']-prev['c'])))
    dm=smooth_changes(moves,n);tr=smooth_changes(ranges,n)
    return [None if a is None or b==0 else 100*a/b for a,b in zip(dm,tr)]

def percent_b(a,n,k):
    out=[None]*len(a)
    for i in range(n-1,len(a)):
        window=a[i+1-n:i+1];mean=sum(window)/n
        sd=(sum((v-mean)**2 for v in window)/n).sqrt()
        if sd:out[i]=(a[i]-mean+k*sd)/(2*k*sd)
    return out

def oracle_signals(rows,key,overrides=None):
    p={k:D(str(v)) for k,v in (METHODS[key]['params'] | (overrides or {})).items()}
    n=len(rows); c=[r['c'] for r in rows];trend=sma(c,int(p['trend']))
    rn=int(p.get('rsi_n',2));rs=rsi(c,rn)
    short=METHODS[key]['direction']<0;ma=sma(c,int(p['ma'])) if short else []
    result=[False]*n;name=key.removeprefix('spmt_')
    if name=='rsi4_25':slow=rsi(c,int(p['slow_n']))
    if name in ('rsi_1','rsi_2','rsi_3'):summed=rsi(c,int(p['sum_n']))
    if name.startswith('pctb'):pb=percent_b(c,int(p['pctb_n']),p['pctb_k'])
    if name.startswith('dmi_'):di=pdi(rows,int(p['di_n']))
    def lt(v,b):return v is not None and v<b
    def run(i,count,pred):return i+1>=count and all(pred(j) for j in range(i+1-count,i+1))
    def cap(i):return rs[i] is not None and rs[i]<=p['exit_rsi']
    def bottom(i):return rows[i]['h']>rows[i]['l'] and (c[i]-rows[i]['l'])/(rows[i]['h']-rows[i]['l'])<=p['loc']/100
    for i in range(n):
        if trend[i] is None:continue
        if short:
            if not c[i]<trend[i] or ma[i] is None or not c[i]>=ma[i]:continue
        elif not c[i]>trend[i]:continue
        if name in ('eom_co','eom_cc'):
            pred=(lambda j:c[j]<rows[j]['o']) if name=='eom_co' else (lambda j:j>0 and c[j]<c[j-1])
            ok=int(rows[i]['date'][-2:])>=p['dom'] and run(i,int(p['bars']),pred) and cap(i)
        elif name=='eom_rsi':ok=int(rows[i]['date'][-2:])>=p['dom'] and lt(rs[i],p['rsi_lt'])
        elif name=='rsi2_4':ok=lt(rs[i],p['rsi_lt'])
        elif name=='rsi4_25':ok=lt(slow[i],p['slow_lt']) and cap(i)
        elif name in ('rsi_div8','rsi_div2'):
            look=int(p['look']);ok=i>=look and rs[i] is not None and rs[i-look] is not None and c[i]<c[i-look] and rs[i]>rs[i-look] and rs[i]<p['rsi_lt']
        elif name in ('rsi_1','rsi_2','rsi_3'):
            days=int(p['sum_days']);window=summed[max(0,i+1-days):i+1]
            ok=len(window)==days and all(v is not None for v in window) and sum(window)<p['sum_lt']
        elif name.startswith('pctb'):
            ok=run(i,int(p['days']),lambda j:lt(pb[j],p['pctb_lt'])) and (lt(rs[i],p['rsi_lt']) if name=='pctb_1' else bottom(i) and cap(i))
        elif name=='price':ok=run(i,int(p['bars']),lambda j:c[j]<rows[j]['o']) and bottom(i) and cap(i)
        elif name.startswith('dmi_'):ok=run(i,int(p['days']),lambda j:lt(di[j],p['di_lt'])) and cap(i)
        elif name=='short_4up':ok=run(i,int(p['up']),lambda j:j>0 and c[j]>c[j-1])
        elif name=='short_strong':ok=i>=2 and c[i]/c[i-2]-1>p['rise']/100 and rs[i] is not None and rs[i]>p['rsi_gt']
        else:raise ValueError(name)
        result[i]=bool(ok)
    return result,rs,ma,p

FIELDS=['entry','exit','entry_date','exit_date','direction','entry_price','exit_price','entry_fill','exit_fill','entry_signal','exit_signal','open']
def record(rows,key,entry,exit,open_flag=False):
    return dict(entry=entry,exit=exit,entry_date=rows[entry]['date'],exit_date=rows[exit]['date'],direction=METHODS[key]['direction'],
        entry_price=rows[entry]['c'],exit_price=rows[exit]['c'],entry_fill='close',exit_fill='mark' if open_flag else 'close',entry_signal=entry,exit_signal=exit,open=open_flag)

def oracle(rows,key,start,to,overrides=None):
    signals,rs,ma,p=oracle_signals(rows,key,overrides)
    trades=[];reasons=[];held=None
    for i in range(start,to+1):
        exiting=held is not None and (rows[i]['c']<ma[i] if METHODS[key]['direction']<0 else rs[i] is not None and rs[i]>p['exit_rsi'])
        if held is None:
            if signals[i]:held=i;status='filled'
            else:status='no_signal'
        elif exiting:
            trades.append(record(rows,key,held,i));held=None;status='exit_close_no_reentry' if signals[i] else 'exit'
        else:status='position_already_open' if signals[i] else 'held'
        if signals[i] and status!='filled':reasons.append(dict(bar=i,date=rows[i]['date'],reason=status))
    if held is not None:trades.append(record(rows,key,held,to,True))
    return trades,signals,reasons

def write_csv(path,records,fields=FIELDS):
    with path.open('w',newline='') as f:
        writer=csv.DictWriter(f,fieldnames=fields);writer.writeheader();writer.writerows(records)

def read_ledger(text):
    records=list(csv.DictReader(io.StringIO(text)))
    for r in records:
        for k in ('entry','exit','entry_signal','exit_signal'):r[k]=int(r[k])
        r['direction']=int(D(r['direction']))
        for k in ('entry_price','exit_price'):r[k]=D(r[k])
        r['open']=r['open']=='true'
    return records

def compare(actual,expected):
    mismatches=[]
    for i in range(max(len(actual),len(expected))):
        if i>=len(actual) or i>=len(expected):mismatches.append(dict(row=i,reason='missing_or_extra_row'));continue
        differences={}
        for k in FIELDS:
            a=actual[i][k];e=expected[i][k]
            same=abs(a-e)<=max(D('1e-9'),abs(e)*D('1e-12')) if k.endswith('_price') else a==e
            if not same:differences[k]={'actual':str(a),'expected':str(e)}
        if differences:mismatches.append(dict(row=i,fields=differences))
    return mismatches

def dump(path,obj):path.write_text(json.dumps(obj,indent=2,default=str)+'\n')

def main():
    os.sched_setaffinity(0,set(sorted(os.sched_getaffinity(0))[:2]))
    cap=1800*1024*1024
    soft,hard=resource.getrlimit(resource.RLIMIT_AS)
    resource.setrlimit(resource.RLIMIT_AS,(min(cap,soft) if soft>=0 else cap,hard))
    parser=argparse.ArgumentParser();parser.add_argument('--output',type=Path,required=True)
    parser.add_argument('--from-date',default='2010-01-01');parser.add_argument('--to-date',default='2020-09-11')
    args=parser.parse_args();out=args.output.resolve()
    if out==ROOT or ROOT in out.parents:raise ValueError('Evidence output must be outside Git')
    out.mkdir(parents=True,exist_ok=True)
    modules={str(ROOT/'Context/guru/backend/src/connors.rs'):sha(ROOT/'Context/guru/backend/src/connors.rs'),str(ROOT/'Context/guru/backend/src/books/spmt.rs'):sha(ROOT/'Context/guru/backend/src/books/spmt.rs')}
    source_sha=sha(SOURCE)
    for spec in METHODS.values():spec.update(pdf_path=str(SOURCE),pdf_sha256=source_sha,assumptions=ASSUMPTIONS+spec['assumptions'])
    dump(out/'source-contracts.json',{'ready_contracts':METHODS,'held_contracts':HELD,'source_sha256':source_sha})
    template=(Path(__file__).with_name('ledger_harness.rs')).read_text()
    harness=out/'actual-modules.rs';harness.write_text(template.replace('@CONNORS@',str(ROOT/'Context/guru/backend/src/connors.rs')).replace('@SPMT@',str(ROOT/'Context/guru/backend/src/books/spmt.rs')))
    commands=[]
    def command(a,**kw):commands.append(a);return subprocess.run(a,check=True,text=True,capture_output=True,**kw)
    rust=out/'actual-rust-ledger';command(['rustc','--edition=2021','-C','debuginfo=0','-C','opt-level=1','-C','codegen-units=1',str(harness),'-o',str(rust)])
    tests=out/'scoped-tests';command(['rustc','--edition=2021','--test','-C','debuginfo=0',str(harness),'-o',str(tests)])
    test_result=command([str(tests),'spmt::tests::daily','--test-threads=1']);(out/'rust-daily-tests.txt').write_text(test_result.stdout+test_result.stderr)
    catalog=command([str(rust),'--contracts']).stdout;(out/'compiled-contracts.tsv').write_text(catalog)
    observed={}
    for line in catalog.splitlines():
        key,ready,family,params,reason=line.split('\t');observed[key]=(ready,family,params,reason)
    assert {k for k,v in observed.items() if v[0]=='true'}==set(METHODS)
    assert set(observed)==set(METHODS)|set(HELD)
    for key,spec in METHODS.items():
        actual={k:D(v) for k,v in [s.split('=') for s in observed[key][2].split(';')]}
        assert actual=={k:D(str(v)) for k,v in spec['params'].items()},(key,'parameter contract mismatch')
        assert observed[key][1]==spec['family']
    checks=[];coverage=[]
    for dataset in DATASETS:
        rows,removed,invalid=load(dataset,args.to_date)
        assert not invalid,(dataset['symbol'],invalid[:10])
        start=next(i for i,r in enumerate(rows) if r['date']>=args.from_date);to=len(rows)-1
        canonical=out/f"{dataset['symbol']}-actual-input.csv"
        write_csv(canonical,[dict(date=r['date'],open=r['o'],high=r['h'],low=r['l'],close=r['c']) for r in rows],['date','open','high','low','close'])
        input_hash=sha(dataset['path'])
        coverage.append(dataset|dict(ohlc_sha256=input_hash,rows=len(rows),warmup_rows=start,excluded_pre_floor=removed,first=rows[0]['date'],last=rows[-1]['date'],test_first=rows[start]['date'],invalid_ohlc_dates=invalid,normalized_sha256=sha(canonical)))
        for key in METHODS:
            stem=f"{key}-{dataset['symbol']}";signals_path=out/f'{stem}-rust-signals.csv'
            rust_text=command([str(rust),str(canonical),key,str(start),str(to),'',str(signals_path)]).stdout
            (out/f'{stem}-rust.csv').write_text(rust_text)
            actual=read_ledger(rust_text);expected,signals,unfilled=oracle(rows,key,start,to)
            write_csv(out/f'{stem}-oracle.csv',expected)
            mismatches=compare(actual,expected);dump(out/f'{stem}-mismatches.json',mismatches)
            rs=list(csv.DictReader(signals_path.open()))
            signal_mismatches=[dict(bar=int(r['bar']),actual=r['signal'],expected=signals[int(r['bar'])]) for r in rs if (r['signal']=='true')!=signals[int(r['bar'])]]
            expected_signals=[dict(bar=i,date=rows[i]['date'],signal=signals[i]) for i in range(start,to+1)]
            write_csv(out/f'{stem}-oracle-signals.csv',expected_signals,['bar','date','signal'])
            dump(out/f'{stem}-signal-audit.json',dict(signal_mismatches=signal_mismatches,unfilled_signals=unfilled,total_signals=sum(signals[start:to+1]),filled_entries=len(actual),terminal_signal=signals[to],warmup_signals=sum(signals[:start])))
            checks.append(dict(method=key,symbol=dataset['symbol'],actual_records=len(actual),expected_records=len(expected),row_mismatches=len(mismatches),signal_mismatches=len(signal_mismatches),ohlc_path=dataset['path'],ohlc_sha256=input_hash,rust_module_sha256=modules,actual_ledger=str(out/f'{stem}-rust.csv'),expected_ledger=str(out/f'{stem}-oracle.csv'),mismatches_path=str(out/f'{stem}-mismatches.json'),signal_audit=str(out/f'{stem}-signal-audit.json'),open_records=sum(t['open'] for t in actual),unfilled_signals=len(unfilled),test_first=rows[start]['date'],test_last=rows[to]['date']))
    rejected=[]
    for s in REJECTED_STOCKS:
        rejected.append(s|dict(ohlc_sha256=sha(s['path']),kind='stock',ledger_run=False,reason='SPMT daily market timing requires SPY or declared cash-index timing; actual stocks are the wrong population. Current constituents are not historical membership.'))
    ok=all(c['row_mismatches']==0 and c['signal_mismatches']==0 for c in checks) and all(sum(c['actual_records'] for c in checks if c['method']==key)>0 for key in METHODS)
    report=dict(schema_version=1,source_reviewed=True,implementation_ready=ok,admitted=False,ledger_checked=ok,checked_trades=sum(c['actual_records'] for c in checks),source_contracts=METHODS,held_methods=HELD,checks=checks,coverage=coverage,population_rejections=rejected,rust_module_sha256=modules,source_sha256=source_sha,source_path=str(SOURCE),source_images_reviewed='source/page-003,010,014,018,023,027,031,035,039,043,047,075,079,084,114,118,122,126,130,144,148.png (in parent job directory)',new_algorithm_increment=0,variant_count=len(METHODS),execution='close-only; no costs/performance figures; open marks retained',tolerance='Dates/directions/fill modes/open flags exact; prices max(1e-9,abs(expected)*1e-12)',commands_path=str(out/'commands.json'),verification_scripts_sha256={str(Path(__file__).resolve()):sha(__file__),str(Path(__file__).with_name('ledger_harness.rs')):sha(Path(__file__).with_name('ledger_harness.rs'))})
    # Confirm that every hashed module stayed unchanged during actual compilation.
    assert all(sha(p)==h for p,h in modules.items())
    dump(out/'commands.json',commands);dump(out/'ledger-report.json',report)
    print(json.dumps({k:report[k] for k in ['source_reviewed','implementation_ready','ledger_checked','checked_trades']}))
    if not ok:raise SystemExit(1)

if __name__=='__main__':main()
