#!/usr/bin/env python3
"""Bounded source-oracle audit, compiling actual ehrs/ehmd/connors source.

All data, generated harnesses and result JSON remain in --output-dir, outside
Git. The separate source_oracle module never reads Rust state or ledgers.
Run sequentially: one compiler/test process or series audit at a time.
"""
import argparse
import csv
from datetime import datetime
import hashlib
import io
import json
import math
import os
from pathlib import Path
import resource
import subprocess

from source_oracle import ledger, source_states

REPO = Path(__file__).resolve().parents[3]


def sha(path):
    h = hashlib.sha256()
    with Path(path).open('rb') as f:
        for block in iter(lambda: f.read(1024*1024), b''):
            h.update(block)
    return h.hexdigest()


def save(path, value):
    path.write_text(json.dumps(value, indent=2, allow_nan=False)+'\n')


def read_data(spec, last_date):
    result = []
    with Path(spec['path']).open(newline='') as f:
        for item in csv.DictReader(f):
            r = {k.lower(): v for k, v in item.items()}
            if spec.get('csv_ticker') and r['ticker'] != spec['csv_ticker']:
                continue
            try:
                day = datetime.strptime(r['date'], '%Y-%m-%d').date().isoformat()
            except ValueError:
                day = datetime.strptime(r['date'], '%d-%m-%Y').date().isoformat()
            if day < spec.get('coverage_start', '0000-01-01') or day > last_date:
                continue
            row = {'date': day, **{k: float(r[k].replace(',', '')) for k in ('open','high','low','close')}}
            if not all(math.isfinite(row[k]) and row[k]>0 for k in ('open','high','low','close')):
                raise ValueError(f'{spec["symbol"]}: invalid OHLC at {day}')
            if not row['low'] <= min(row['open'], row['close']) <= max(row['open'], row['close']) <= row['high']:
                raise ValueError(f'{spec["symbol"]}: inconsistent OHLC at {day}; no data repairs')
            if result and result[-1]['date'] >= day:
                raise ValueError('Dates must be unique and chronological')
            result.append(row)
    return result


def canonical(path, rows):
    with path.open('w', newline='') as f:
        w = csv.DictWriter(f, fieldnames=['date','open','high','low','close'])
        w.writeheader()
        w.writerows(rows)


def compile_actual(out):
    src = REPO/'Context/guru/backend/src'
    harness = out/'actual-ehlers-harness.rs'
    harness.write_text(r'''#![allow(dead_code)]
#[derive(Clone,Debug)]
pub struct Row { pub ticker:String,pub date:String,pub open:f64,pub high:f64,
pub low:f64,pub close:f64,pub volume:Option<f64> }
#[path=CONNORS] mod connors;
mod books {
 #[path=EHRS] pub mod ehrs;
 #[path=EHMD] pub mod ehmd;
 use crate::connors::{Ctx,Params,ParamDef,StrategyDef,Trade};
 pub fn catalogue()->Vec<StrategyDef>{vec![]}
 pub fn family_of(_: &str)->Option<(&'static str,&'static str,&'static str)>{None}
 pub fn params(_: &str)->Option<Vec<ParamDef>>{None}
 pub fn run(_: &str,_:&Ctx,_:usize,_:usize,_:&Params)->Option<Vec<Trade>>{None}
}
fn main(){
 let a:Vec<String>=std::env::args().collect();
 let text=std::fs::read_to_string(&a[1]).unwrap();
 let rows:Vec<Row>=text.lines().skip(1).map(|line|{
 let p:Vec<&str>=line.split(',').collect();
 Row{ticker:"REAL".into(),date:p[0].into(),open:p[1].parse().unwrap(),high:p[2].parse().unwrap(),
 low:p[3].parse().unwrap(),close:p[4].parse().unwrap(),volume:None}}).collect();
 let x=connors::Ctx::new(&rows,&rows,&[]);
 let first:usize=a[3].parse().unwrap();let last:usize=a[4].parse().unwrap();
 let p:connors::Params=a[5].split(';').map(|s|{let(k,v)=s.split_once('=').unwrap();(k.to_string(),v.parse().unwrap())}).collect();
 let mbb=p["max_bars_back"] as usize;
 if a[6]=="ledger" {
  let ts=if a[2]=="ehrs_sinetrend" {books::ehrs::run(&a[2],&x,first,last,&p)}else{books::ehmd::run(&a[2],&x,first,last,&p)}.expect("Rejected candidate");
  println!("entry_i,entry_date,entry_signal_i,entry_signal_date,dir,entry_px,entry_fill,exit_i,exit_date,exit_signal_i,exit_signal_date,exit_px,exit_fill,open");
  for t in ts {assert_eq!(t.legs.len(),1);assert!(t.exits.is_empty());assert_eq!(t.entry_kind,"open");
   let e=t.legs[0].i;
   println!("{},{},{},{},{},{:.17},next_open,{},{},{},{},{:.17},{},{}",e,rows[e].date,e-1,rows[e-1].date,t.dir,t.legs[0].px,
   t.exit_i,rows[t.exit_i].date,if t.open{String::new()}else{t.signal_i.to_string()},
   if t.open{""}else{&rows[t.signal_i].date},t.exit_px,if t.open{"terminal_close_mark"}else{"next_open"},t.open);
  }
 } else if a[6]=="signals" {
  let st=books::ehrs::sinetrend_with_mbb(&x,*p.get("cyc_part").unwrap_or(&1.),mbb);
  let(m,f)=books::ehmd::mama_with_mbb(&x,*p.get("fast_limit").unwrap_or(&0.5),*p.get("slow_limit").unwrap_or(&0.05),mbb);
  let mut pos=0;
  println!("signal_i,signal_date,target,changes_position,has_next_bar");
  for i in first..=last {
   let order=if a[2]=="ehrs_sinetrend" {books::ehrs::sinetrend_order(&st,i,pos,false,mbb)}else{books::ehmd::mama_order(&m,&f,i,mbb)};
   if let Some((target,_))=order {println!("{},{},{},{},{}",i,rows[i].date,target,target!=pos,i<last);if i<last{pos=target;}}
  }
 } else {
  let st=books::ehrs::sinetrend_with_mbb(&x,*p.get("cyc_part").unwrap_or(&1.),mbb);
  let hd=books::ehrs::homodyne_with_mbb(&x,mbb);
  let(m,f)=books::ehmd::mama_with_mbb(&x,*p.get("fast_limit").unwrap_or(&0.5),*p.get("slow_limit").unwrap_or(&0.05),mbb);
  println!("i,smooth,smooth_period,trendline,sine,lead,trend,mama,fama");
  for i in first..=last {println!("{},{:.17},{:.17},{:.17},{:.17},{:.17},{},{:.17},{:.17}",i,st.smooth[i],hd.smooth_period[i],st.trendline[i],st.sine[i],st.lead[i],st.trend[i],m[i],f[i]);}
 }
}
'''.replace('CONNORS', json.dumps(str(src/'connors.rs')))
        .replace('EHRS', json.dumps(str(src/'books/ehrs.rs')))
        .replace('EHMD', json.dumps(str(src/'books/ehmd.rs'))))
    binary=out/'actual-ehlers-harness'
    cmd=['rustc','--edition=2021','-C','opt-level=1','-C','codegen-units=1',str(harness),'-o',str(binary)]
    cp=subprocess.run(cmd,capture_output=True,text=True,timeout=180)
    (out/'harness-build.txt').write_text(cp.stdout+cp.stderr)
    cp.check_returncode()
    tests=out/'actual-ehlers-tests'
    subprocess.run(['rustc','--edition=2021','--test','-C','codegen-units=1',str(harness),'-o',str(tests)],check=True,capture_output=True,text=True,timeout=180)
    cp=subprocess.run([str(tests),'books::e','--test-threads=1'],capture_output=True,text=True,timeout=60)
    (out/'rust-tests.txt').write_text(cp.stdout+cp.stderr)
    cp.check_returncode()
    return binary


def rust_run(binary, path, method, first, last, parameters, mode='ledger'):
    cmd=[str(binary),str(path),method,str(first),str(last),';'.join(f'{k}={v}' for k,v in parameters.items()),mode]
    cp=subprocess.run(cmd,capture_output=True,text=True,check=True,timeout=60)
    result=list(csv.DictReader(io.StringIO(cp.stdout)))
    for row in result:
        for key, value in list(row.items()):
            if key.endswith('_i') or key in ('i','dir','target','trend'):
                row[key]=int(float(value)) if value else None
            elif key in ('open','changes_position','has_next_bar'):
                row[key]=value=='true'
            elif key.endswith('_px') or key in ('smooth','smooth_period','trendline','sine','lead','mama','fama'):
                row[key]=float(value)
            elif value=='':
                row[key]=None
    return result, cmd


def differences(actual, expected):
    diffs=[]
    if len(actual)!=len(expected):
        diffs.append({'field':'record_count','actual':len(actual),'expected':len(expected)})
    for i,(a,b) in enumerate(zip(actual,expected)):
        for key,want in b.items():
            got=a.get(key)
            equal=math.isclose(got,want,rel_tol=1e-12,abs_tol=1e-8) if isinstance(want,float) and isinstance(got,float) else got==want
            if not equal:
                diffs.append({'row':i,'field':key,'actual':got,'expected':want})
    return diffs


def main():
    # Child compiler and harness processes inherit this bounded CPU affinity.
    os.sched_setaffinity(0, set(sorted(os.sched_getaffinity(0))[:2]))
    ap=argparse.ArgumentParser()
    ap.add_argument('--manifest',type=Path,required=True)
    ap.add_argument('--output-dir',type=Path,required=True)
    args=ap.parse_args()
    cfg=json.loads(args.manifest.read_text())
    out=args.output_dir.resolve()
    out.mkdir(parents=True,exist_ok=True)
    sources={p:sha(p) for p in cfg['source_files']}
    source_stats={p:(Path(p).stat().st_size,Path(p).stat().st_mtime_ns) for p in sources}
    rust_sources={str(p):sha(p) for p in [REPO/'Context/guru/backend/src/books/ehrs.rs',REPO/'Context/guru/backend/src/books/ehmd.rs',REPO/'Context/guru/backend/src/connors.rs']}
    binary=compile_actual(out)
    report={'schema_version':1,'source_reviewed':True,'implementation_ready':False,'admitted':False,
        'ledger_checked':False,'checked_trades':0,'checks':[],'edge_checks':[],
        'source_contracts':cfg['contracts'],'source_sha256':sources,'rust_module_sha256':rust_sources,
        'oracle_sha256':sha(Path(__file__).with_name('source_oracle.py')),
        'verifier_sha256':sha(Path(__file__)),'harness_sha256':sha(out/'actual-ehlers-harness.rs'),
        'compiled_binary_sha256':sha(binary),'independent_oracle':'Separately written Python source recurrences and streaming crossing/position ledger',
        'price_tolerance':{'absolute':1e-8,'relative':1e-12},'source_performance_reproduced':False,
        'verification_scope':'selected daily baseline and explicit research conventions only',
        'range':{'requested_from':cfg['from'],'requested_to':cfg['to']},
        'resource_limits':{'cpu_affinity':sorted(os.sched_getaffinity(0)),
                           'sequential_child_processes':True,'rust_codegen_units':1}}
    for spec in cfg['datasets']:
        path=Path(spec['path']);before=path.stat();digest=sha(path)
        rows=read_data(spec,cfg['to'])
        first=next(i for i,r in enumerate(rows) if r['date']>=cfg['from'])
        last=len(rows)-1
        data_path=out/(spec['symbol']+'-actual.csv');canonical(data_path,rows)
        for method,contract in cfg['contracts'].items():
            if method=='ehmd_mama' and spec['instrument']['kind']!='stock':
                continue
            parameters=contract['params']
            expected,expected_signals,states=ledger(method,rows,first,last,parameters)
            actual,cmd=rust_run(binary,data_path,method,first,last,parameters)
            actual_signals,signal_cmd=rust_run(binary,data_path,method,first,last,parameters,'signals')
            diffs=differences(actual,expected)
            signal_diffs=differences(actual_signals,expected_signals)
            ledger_path=out/f'{method}-{spec["symbol"]}-ledger.json'
            save(ledger_path,{'method':method,'symbol':spec['symbol'],'parameters':parameters,'actual':actual,'expected':expected,'mismatches':diffs,
                'actual_signals':actual_signals,'expected_signals':expected_signals,'signal_mismatches':signal_diffs,'rust_command':cmd,'signal_command':signal_cmd})
            returns=[t['dir']*(t['exit_px']/t['entry_px']-1) for t in actual if not t['open']]
            check={'method':method,'symbol':spec['symbol'],'instrument':spec['instrument'],'actual_records':len(actual),
                'expected_records':len(expected),'row_mismatches':len(diffs),'signal_mismatches':len(signal_diffs),
                'ohlc_path':str(path),'ohlc_sha256':digest,'rust_module_sha256':rust_sources,'ledger_path':str(ledger_path),
                'loaded_bars':len(rows),'warmup_first':rows[0]['date'],'selected_first':rows[first]['date'],'selected_last':rows[last]['date'],
                'actual_warmup_bars':first,'parameters':parameters,'closed_trades':len(returns),'open_records':sum(t['open'] for t in actual),
                'closed_gross_return_sum':sum(returns),'closed_negative_returns':sum(r<0 for r in returns),
                'population_label':'Selected actual series; no pooled portfolio or historical constituent membership claimed'}
            report['checks'].append(check)
            # checked_trades follows the required actual-record sum. Readiness is
            # false on any mismatch, even if the count alone happens to agree.
            report['checked_trades']+=len(actual)

            # Initial source state and later real-series states. These are not
            # synthetic market inputs, and do not count as additional trades.
            rust_states,state_cmd=rust_run(binary,data_path,method,0,last,parameters,'states')
            indices=list(range(0,int(parameters['max_bars_back'])+16))+list(range(first,last+1,101))+[last]
            wanted=[{'i':i,**{k:states[i].get(k,0.0) for k in ('smooth','smooth_period','trendline','sine','lead','trend','mama','fama')}} for i in sorted(set(indices))]
            sampled=[rust_states[r['i']] for r in wanted]
            state_diffs=differences(sampled,wanted)
            edge={'method':method,'symbol':spec['symbol'],'kind':'source_state_startup_and_real_samples','sampled_bars':len(wanted),'mismatches':state_diffs,'rust_command':state_cmd}
            report['edge_checks'].append(edge)

            # Three actual signal-day cutoffs per case. The last signal must be
            # visible but unfilled; full-context and actual-prefix outputs agree.
            cut_signals=[s for s in expected_signals if s['changes_position'] and first+25<s['signal_i']<last-2][:3]
            for signal in cut_signals:
                cut=signal['signal_i'];prefix=rows[:cut+1]
                prefix_path=out/f'{method}-{spec["symbol"]}-prefix-{cut}.csv';canonical(prefix_path,prefix)
                want,signals,_=ledger(method,prefix,first,cut,parameters)
                got,prefix_cmd=rust_run(binary,prefix_path,method,first,cut,parameters)
                full_context,_=rust_run(binary,data_path,method,first,cut,parameters)
                got_signals,_=rust_run(binary,prefix_path,method,first,cut,parameters,'signals')
                errors=differences(got,want)+differences(got,full_context)+differences(got_signals,signals)
                if not got_signals or got_signals[-1]['has_next_bar'] or not got_signals[-1]['changes_position']:
                    errors.append({'kind':'terminal_signal_not_recorded_unfilled'})
                # Full run has an execution at the next actual open.
                full_fill=any(t['entry_i']==cut+1 or (not t['open'] and t['exit_i']==cut+1) for t in actual)
                if not full_fill:
                    errors.append({'kind':'next_actual_open_fill_missing'})
                report['edge_checks'].append({'method':method,'symbol':spec['symbol'],'kind':'real_unfilled_signal_and_prefix_causality',
                    'last_bar':rows[cut]['date'],'signal':got_signals[-1] if got_signals else None,'full_run_next_open_fill':full_fill,
                    'prefix_sha256':sha(prefix_path),'mismatches':errors,'rust_command':prefix_cmd})
            if len(cut_signals)!=3:
                raise RuntimeError('Insufficient nonzero real signal cases for unfilled audit')
            print(json.dumps({'method':method,'symbol':spec['symbol'],'records':len(actual),'row_mismatches':len(diffs),'signal_mismatches':len(signal_diffs)}),flush=True)
        after=path.stat()
        if (before.st_size,before.st_mtime_ns)!=(after.st_size,after.st_mtime_ns) or sha(path)!=digest:
            raise RuntimeError('Original market file changed during read-only audit')
    for path,digest in {**sources,**rust_sources}.items():
        if sha(path)!=digest:
            raise RuntimeError('Compiled/source input changed during audit')
    if any((Path(p).stat().st_size,Path(p).stat().st_mtime_ns)!=s for p,s in source_stats.items()):
        raise RuntimeError('Original source metadata changed during audit')
    okay=all(c['actual_records']>0 and c['row_mismatches']==0 and c['signal_mismatches']==0 for c in report['checks']) and all(not e['mismatches'] for e in report['edge_checks'])
    report.update(implementation_ready=okay,ledger_checked=okay and report['checked_trades']>0,
        method_ids=list(cfg['contracts']) if okay else [],new_source_distinct_method_count=0,
        reviewed_existing_method_count=len(cfg['contracts']) if okay else 0,
        summary=f'2 reviewed existing baseline IDs; zero new algorithms; {len(report["checks"])} real-series cases; {report["checked_trades"]} actual ledger records; admission false.')
    report['resource_limits']['parent_peak_rss_kib']=resource.getrusage(resource.RUSAGE_SELF).ru_maxrss
    report['resource_limits']['child_peak_rss_kib']=resource.getrusage(resource.RUSAGE_CHILDREN).ru_maxrss
    save(out/'ledger-report.json',report)
    print(json.dumps({'report':str(out/'ledger-report.json'),'passed':okay,'checked_trades':report['checked_trades']}))
    if not okay:
        raise SystemExit(1)


if __name__=='__main__':
    main()
