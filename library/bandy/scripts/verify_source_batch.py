#!/usr/bin/env python3
"""Read-only real-stock/ETF verification against ACTUAL compiled hbqts modules."""
import argparse, csv, hashlib, io, json, os, subprocess, sys
from datetime import datetime
from decimal import Decimal as D
from pathlib import Path
from source_oracle import METHODS, READY, COMMON, signals, ledger

REPO=Path(__file__).resolve().parents[3]

def sha(path):
 h=hashlib.sha256()
 with Path(path).open('rb') as f:
  for b in iter(lambda:f.read(1024*1024),b''):h.update(b)
 return h.hexdigest()

def dates(v):
 for fmt in ('%Y-%m-%d','%d-%m-%Y'):
  try:return datetime.strptime(v,fmt).date().isoformat()
  except ValueError:pass
 raise ValueError(v)

def read(spec,end):
 rows=[]
 with Path(spec['path']).open(newline='') as f:
  for v in csv.DictReader(f):
   r={k.lower():v for k,v in v.items()}
   if spec.get('csv_ticker') and r.get('ticker')!=spec['csv_ticker']:continue
   day=dates(r['date'])
   if day<spec.get('coverage_start','0000-00-00') or day>end:continue
   item=dict(date=day,**{k:D(r[k].replace(',','')) for k in ('open','high','low','close')})
   if not all(item[k].is_finite() and item[k]>0 for k in ('open','high','low','close')):raise ValueError(f'Invalid OHLC {spec["symbol"]} {day}')
   if not item['low']<=min(item['open'],item['close'])<=max(item['open'],item['close'])<=item['high']:raise ValueError(f'Inconsistent OHLC {spec["symbol"]} {day}; no repair')
   if rows and rows[-1]['date']>=day:raise ValueError('Dates must be strictly ascending')
   rows.append(item)
 if not rows:raise ValueError('No actual rows')
 return rows

def save_csv(path,records,fields=None):
 with path.open('w',newline='') as f:
  fields=fields or list(records[0])
  w=csv.DictWriter(f,fieldnames=fields);w.writeheader();w.writerows(records)

def build(job):
 src=REPO/'Context/guru/backend/src'
 source=job/'actual-hbqts-harness.rs'
 source.write_text('''#![allow(dead_code)]
#[derive(Clone)] pub struct Row {pub ticker:String,pub date:String,pub open:f64,pub high:f64,pub low:f64,pub close:f64,pub volume:Option<f64>}
#[path=CONNORS] mod connors;
#[path=BANDY] mod hbqts;
mod books {
 use crate::connors::{Ctx,Params,ParamDef,StrategyDef,Trade};
 pub fn catalogue()->Vec<StrategyDef>{vec![]}
 pub fn family_of(_: &str)->Option<(&'static str,&'static str,&'static str)>{None}
 pub fn params(_: &str)->Option<Vec<ParamDef>>{None}
 pub fn run(_: &str,_: &Ctx,_:usize,_:usize,_:&Params)->Option<Vec<Trade>>{None}
}
fn main(){
 let a:Vec<String>=std::env::args().collect();let data=std::fs::read_to_string(&a[1]).unwrap();
 let rows:Vec<Row>=data.lines().skip(1).map(|s|{let p:Vec<_>=s.split(',').collect();Row{ticker:"ACTUAL".into(),date:p[0].into(),open:p[1].parse().unwrap(),high:p[2].parse().unwrap(),low:p[3].parse().unwrap(),close:p[4].parse().unwrap(),volume:None}}).collect();
 let x=connors::Ctx::new(&rows,&rows,&[]);let p:connors::Params=a[5].split(';').filter(|s|!s.is_empty()).map(|s|{let(k,v)=s.split_once('=').unwrap();(k.into(),v.parse().unwrap())}).collect();
 let from:usize=a[3].parse().unwrap();let to:usize=a[4].parse().unwrap();
 if a.get(6).is_some_and(|v|v=="signals"){
  println!("bar,raw_buy,buy,sell,short,cover,buy_px,sell_px");
  for(i,s)in hbqts::reviewed::signals(&a[2],&x,to,&p).unwrap().into_iter().enumerate(){
   println!("{},{},{},{},{},{},{:.17},{:.17}",i,s.raw_buy,s.buy,s.sell,s.short,s.cover,s.buy_px,s.sell_px);
  }
 }else{
  println!("entry,exit,dir,entry_px,exit_px,open,entry_kind,exit_kind,signal_i,entry_date,exit_date");
  for t in hbqts::run(&a[2],&x,from,to,&p).unwrap(){assert_eq!(t.legs.len(),1);assert!(t.exits.is_empty());
   let kind=if t.open{"mark"}else if a[2]=="hbqts_profit_target"{"target"}else{"close"};
   println!("{},{},{},{:.17},{:.17},{},{},{},{},{},{}",t.legs[0].i,t.exit_i,t.dir,t.legs[0].px,t.exit_px,t.open,t.entry_kind,kind,t.signal_i,rows[t.legs[0].i].date,rows[t.exit_i].date);
  }
 }
}
'''.replace('CONNORS',json.dumps(str(src/'connors.rs'))).replace('BANDY',json.dumps(str(src/'books/hbqts.rs'))))
 binary=job/'actual-hbqts-harness';test=job/'actual-hbqts-tests'
 for args in (["-C","opt-level=1",str(source),"-o",str(binary)],["--test",str(source),"-o",str(test)]):
  r=subprocess.run(['rustc','--edition=2021',*args],capture_output=True,text=True,timeout=180)
  (job/('rust-build-'+('tests' if '--test' in args else 'ledger')+'.txt')).write_text(r.stdout+r.stderr)
  r.check_returncode()
 r=subprocess.run([str(test),'hbqts::','--test-threads=1'],capture_output=True,text=True,timeout=60)
 (job/'rust-tests.txt').write_text(r.stdout+r.stderr);r.check_returncode()
 return binary

def boolv(v):return v.lower()=='true'
def mismatch(actual,expected):
 problems=[]
 if len(actual)!=len(expected):problems.append(dict(field='row_count',actual=len(actual),expected=len(expected)))
 for i,(got,want) in enumerate(zip(actual,expected)):
  for k,value in want.items():
   if isinstance(value,bool):equal=boolv(got[k])==value
   elif isinstance(value,D) or value is None:
    g=D(got[k]);equal=(not g.is_finite()) if value is None else g.is_finite() and abs(g-value)<=max(D('1e-8'),abs(value)*D('1e-12'))
   elif isinstance(value,int):equal=int(float(got[k]))==value
   else:equal=got[k]==value
   if not equal:problems.append(dict(row=i,field=k,actual=got[k],expected=str(value)))
 return problems

def call(binary,canonical,method,first,last,params,mode=None):
 cmd=[str(binary),str(canonical),method,str(first),str(last),';'.join(f'{k}={v}' for k,v in params.items())]
 if mode:cmd.append(mode)
 r=subprocess.run(cmd,check=True,capture_output=True,text=True,timeout=60)
 return list(csv.DictReader(io.StringIO(r.stdout))),cmd,r.stdout

def source_contracts(root):
 book=root/'Quantitative Trading Systems Practical Methods for Design Testing and Validation Howard B Bandy.pdf'
 errata=[root/'Quantitative Trading Systems Errata.pdf',root/'Introduction to AmiBroker Errata.pdf']
 identities=[dict(path=str(x),sha256=sha(x)) for x in [book,*errata]]
 return {m:dict(**v,common_contract=COMMON,pdf=identities[0],errata=identities[1:],afl=dict(path=str(root/'QTS Book Code'/f'Figure_{v["figure"]}.afl'),sha256=sha(root/'QTS Book Code'/f'Figure_{v["figure"]}.afl')),errata_resolution='QTS May12,2007: chapter5 figure references, Moon Phase caption, chapter15 reference, Figure17.9 scan comment and Figure17.17 runs degenerate denominator. IAB Nov10,2008: reserved V renamed VV p92, spelling p568. No correction changes this subset; future dependent review must apply corrections first.') for m,v in METHODS.items()}

def main():
 ap=argparse.ArgumentParser();ap.add_argument('--manifest',required=True,type=Path);ap.add_argument('--output',required=True,type=Path);a=ap.parse_args()
 cfg=json.loads(a.manifest.read_text());job=a.output.resolve().parent;job.mkdir(parents=True,exist_ok=True)
 # One process, at most two eligible CPU cores; rustc builds and backtests are sequential.
 if hasattr(os,'sched_getaffinity'):os.sched_setaffinity(0,sorted(os.sched_getaffinity(0))[:2])
 os.environ['RAYON_NUM_THREADS']='1';os.environ['OMP_NUM_THREADS']='1'
 selected=cfg.get('methods',READY)
 if not selected or any(m not in READY for m in selected):raise ValueError('Only reviewed ready-subset methods')
 contracts=source_contracts(Path(cfg['source_root']));(job/'source-contracts.json').write_text(json.dumps(contracts,indent=2)+'\n')
 src=REPO/'Context/guru/backend/src';rust={str(p):sha(p) for p in [src/'books/hbqts.rs',src/'books/hbqts/reviewed.rs',src/'connors.rs']}
 binary=build(job)
 report=dict(schema_version=1,author='Howard B. Bandy',book='Quantitative Trading Systems (2007) + supplied companion AFL',source_reviewed=True,implementation_ready=False,admitted=False,ledger_checked=False,checked_trades=0,source_contracts=contracts,rust_module_sha256=rust,independent_oracle='Separately transcribed Python Decimal40 AFL arrays and ledger against actual compiled hbqts.rs + hbqts/reviewed.rs + connors.rs',oracle_sha256={str(Path(__file__).resolve()):sha(Path(__file__)),str(Path(__file__).with_name('source_oracle.py').resolve()):sha(Path(__file__).with_name('source_oracle.py'))},checks=[],original_studies_reproduced=False,native_amibroker_export_parity=False,population=COMMON['population'],commands=[],held_methods=[dict(method=m,reason=v['held']) for m,v in METHODS.items() if 'held' in v])
 for spec in cfg['datasets']:
  methods=spec.get('methods',selected)
  if any(m not in selected or spec['instrument']['kind']!=METHODS[m]['instrument_requirement'] for m in methods):raise ValueError('Source instrument requirement mismatch; no index/stock/ETF substitution')
  path=Path(spec['path']);before=path.stat();digest=sha(path);rows=read(spec,cfg['to'])
  first=next(i for i,r in enumerate(rows) if r['date']>=cfg['from']);last=len(rows)-1
  canonical=job/(spec['symbol']+'-actual.csv');save_csv(canonical,rows,['date','open','high','low','close'])
  for method in methods:
   params=METHODS[method]['params'];arrays=signals(method,rows,params);expected=ledger(method,rows,first,last,params,arrays)
   actual,cmd,raw=call(binary,canonical,method,first,last,params);report['commands'].append(cmd)
   prefix=job/(method+'-'+spec['symbol'])
   actual_file=Path(str(prefix)+'-rust.csv');actual_file.write_text(raw)
   expected_file=Path(str(prefix)+'-oracle.csv');save_csv(expected_file,expected,['entry','exit','dir','entry_px','exit_px','entry_kind','open','exit_kind','entry_date','exit_date'])
   problems=mismatch(actual,expected)
   for i,r in enumerate(actual):
    if int(r['signal_i'])!=int(r['exit']):problems.append(dict(row=i,field='signal_i',actual=r['signal_i'],expected=r['exit']))
   actual_s,cmd_s,raw_s=call(binary,canonical,method,first,last,params,'signals');report['commands'].append(cmd_s)
   signal_file=Path(str(prefix)+'-rust-signals.csv');signal_file.write_text(raw_s)
   signal_problems=mismatch(actual_s,arrays)
   all_trades=len(actual);matched=max(0,min(len(actual),len(expected))-len({x['row'] for x in problems if 'row' in x}))
   closed=[D(r['dir'])*(D(r['exit_px'])/D(r['entry_px'])-1) for r in actual if not boolv(r['open'])]
   entries={int(r['entry']) for r in actual};eligible=sum(s['buy'] or s['short'] for s in arrays[first:last+1]);unfilled=[]
   if method=='hbqts_limit_entry':
    for i in range(max(first,1),last+1):
     if arrays[i-1]['raw_buy'] and not arrays[i]['buy']:unfilled.append(dict(signal_bar=i-1,next_bar=i,signal_date=rows[i-1]['date'],next_date=rows[i]['date'],limit=str(arrays[i]['buy_px']),low=str(rows[i]['low'])))
    Path(str(prefix)+'-unfilled-signals.json').write_text(json.dumps(unfilled,indent=2)+'\n')
    if any(int(r['entry']) in {v['next_bar'] for v in unfilled} for r in actual):problems.append(dict(field='unfilled_limit_was_entered'))
   check=dict(method=method,symbol=spec['symbol'],instrument=spec['instrument'],ohlc_path=str(path),ohlc_sha256=digest,canonical_input_sha256=sha(canonical),loaded_bars=len(rows),warmup_first=rows[0]['date'],warmup_bars=first,from_date=rows[first]['date'],to_date=rows[last]['date'],parameters=params,actual_records=all_trades,expected_records=len(expected),matched_records=matched,row_mismatches=len(problems),first_mismatches=problems[:10],signal_row_mismatches=len(signal_problems),first_signal_mismatches=signal_problems[:10],signal_rows_checked=len(arrays),eligible_signals=eligible,eligible_signals_without_new_entry=eligible-len(entries),unfilled_limit_signals=len(unfilled),closed_trades=len(closed),negative_closed_trades=sum(v<0 for v in closed),open_records=sum(boolv(r['open']) for r in actual),gross_price_return_sum=str(sum(closed,D(0))),rust_ledger=str(actual_file),oracle_ledger=str(expected_file),rust_signal_ledger=str(signal_file))
   report['checks'].append(check)
   if not problems and not signal_problems:report['checked_trades']+=all_trades
   print(json.dumps(dict(method=method,symbol=spec['symbol'],records=all_trades,mismatches=len(problems),signal_mismatches=len(signal_problems))),flush=True)
  after=path.stat()
  if (before.st_size,before.st_mtime_ns)!=(after.st_size,after.st_mtime_ns) or sha(path)!=digest:raise ValueError('Original input changed')
 if any(sha(p)!=h for p,h in rust.items()):raise RuntimeError('Compiled source changed; rerun')
 report['implementation_ready']=all(c['row_mismatches']==0 and c['signal_row_mismatches']==0 for c in report['checks'])
 report['ledger_checked']=report['implementation_ready'] and all(any(c['method']==m and c['actual_records']>0 for c in report['checks']) for m in selected)
 report['method_ids']=[m for m in selected if all(c['row_mismatches']==0 and c['signal_row_mismatches']==0 for c in report['checks'] if c['method']==m) and any(c['actual_records']>0 for c in report['checks'] if c['method']==m)]
 report['summary']=f'{len(selected)} bounded source methods; {len(report["checks"])} real stock/ETF cases; {report["checked_trades"]} matching Rust ledger records. Stop-entry and APZ remain held; admission false.'
 a.output.write_text(json.dumps(report,indent=2)+'\n')
 if not report['ledger_checked']:sys.exit(1)
if __name__=='__main__':main()
