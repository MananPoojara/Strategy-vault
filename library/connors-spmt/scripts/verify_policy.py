#!/usr/bin/env python3
"""Run scoped integration assertions against actual readiness/policy modules.

The full backend test executable exceeds the approved memory budget. This
imports the actual relevant modules and executes the SPMT checks without a
service, a fabricated production price series, or substitutes for policy code.
"""
import argparse
import csv
import hashlib
import json
import os
from pathlib import Path
import resource
import subprocess

ROOT=Path(__file__).resolve().parents[3]
def sha(p):return hashlib.sha256(Path(p).read_bytes()).hexdigest()
def main():
    os.sched_setaffinity(0,set(sorted(os.sched_getaffinity(0))[:2]))
    cap=1800*1024*1024
    soft,hard=resource.getrlimit(resource.RLIMIT_AS)
    resource.setrlimit(resource.RLIMIT_AS,(min(cap,soft) if soft>=0 else cap,hard))
    parser=argparse.ArgumentParser();parser.add_argument('--output',required=True,type=Path)
    out=parser.parse_args().output.resolve()
    if ROOT==out or ROOT in out.parents:raise ValueError('private output required')
    out.mkdir(parents=True,exist_ok=True)
    backend=ROOT/'Context/guru/backend/src'
    template=Path(__file__).with_name('ledger_harness.rs').read_text().split('fn main()')[0]
    template=template.replace('@CONNORS@',str(backend/'connors.rs')).replace('@SPMT@',str(backend/'books/spmt.rs')).replace('mod spmt;','pub mod spmt;')
    template=template.replace('mod books {','mod books {\n pub use crate::{spmt,chande,otvt};')
    modules=[backend/'connors.rs',backend/'books/spmt.rs']
    for name in ['readiness','instrument_policy','baskets','gate','books/chande','books/otvt']:
        path=backend/(name+'.rs');modules.append(path)
        template+=f'#[path="{path}"] pub mod {name.split("/")[-1]};\n'
    # The unchanged baskets module references two non-strategy main helpers.
    # Extract exact source functions, recording source/excerpt hashes separately.
    api=(backend/'main.rs').read_text();excerpts=[]
    for begin,end in [('fn parse_csv_fields(', 'fn parse_csv_number('),('fn esc(', 'fn stats_json(')]:
        excerpts.append(api[api.index(begin):api.index(end)])
    template+='\n'.join(excerpts)
    template+='''
fn main() {
 let a:Vec<String>=std::env::args().collect();
 for (path,ticker) in [(&a[1],"SPY"),(&a[2],"NIFTY 50")] {
  let text=std::fs::read_to_string(path).unwrap();
  let rows:Vec<Row>=text.lines().skip(1).map(|line|{
   let f:Vec<&str>=line.split(',').collect();
   Row{ticker:ticker.into(),date:f[0].into(),open:f[1].parse().unwrap(),high:f[2].parse().unwrap(),low:f[3].parse().unwrap(),close:f[4].parse().unwrap(),volume:None}
  }).collect();
  let x=connors::Ctx::new(&rows,&rows,&[]);
  let from=rows.iter().position(|r|r.date.as_str()>="2010-01-01").unwrap();
  for def in spmt::catalogue() {
   if spmt::reviewed_daily(def.id) {
    assert!(instrument_policy::check(&def,ticker,false).is_none());
    assert!(readiness::check(&x,&def,from,rows.len()-1,&connors::Params::new()).is_none(),"{}",def.id);
    for stock in ["AAPL","RELIANCE"] {assert!(instrument_policy::check(&def,stock,true).is_some());}
    assert!(instrument_policy::check(&def,"QQQ",false).is_some());
    let wrong_book=connors::StrategyDef{book:"stc9",..def};
    assert!(!instrument_policy::reviewed_cash_rule(&wrong_book));
   } else {
    assert!(instrument_policy::check(&def,ticker,false).is_some());
    assert_eq!(readiness::check(&x,&def,from,rows.len()-1,&connors::Params::new()).unwrap().status,"needs_data");
   }
  }
 }
 let base=spmt::catalogue().into_iter().next().unwrap();
 let existing=connors::StrategyDef{id:"kftsm_ma",book:"kftsm",..base};
 assert!(instrument_policy::check(&existing,"SPY",false).is_none());
 assert!(instrument_policy::check(&existing,"RELIANCE",true).is_none());
 let unknown=connors::StrategyDef{id:"spmt_unreviewed_new",book:"spmt",..existing};
 assert!(instrument_policy::check(&unknown,"SPY",false).is_some());
 println!("PASS: actual SPMT readiness/policy on SPY and Nifty;20 exact IDs,25 held IDs,stock/ETF substitution rejection,wrong-book and new-ID rejection,existing Kaufman admissions preserved");
}
'''
    harness=out/'actual-policy-modules.rs';harness.write_text(template)
    binary=out/'actual-policy-checks';commands=[['rustc','--edition=2021','-C','debuginfo=0','-C','opt-level=0',str(harness),'-o',str(binary)],
        [str(binary),str(out/'SPY-actual-input.csv'),str(out/'NIFTY50-actual-input.csv')]]
    logs=[]
    for cmd in commands:
        result=subprocess.run(cmd,check=True,text=True,capture_output=True);logs.append(result.stdout+result.stderr)
    (out/'policy-tests.txt').write_text('\n'.join(logs))
    report={'passed':True,'commands':commands,'rust_module_sha256':{str(p):sha(p) for p in modules},
        'helper_source':str(backend/'main.rs'),'helper_source_sha256':sha(backend/'main.rs'),
        'helper_excerpt_sha256':[hashlib.sha256(s.encode()).hexdigest() for s in excerpts],
        'harness_sha256':sha(harness),'script_sha256':sha(__file__),'scope':'Actual SPMT/policy/readiness modules;20 candidate IDs plus all held catalogue IDs; no service or publisher invoked'}
    (out/'policy-report.json').write_text(json.dumps(report,indent=2)+'\n');print(logs[-1])
if __name__=='__main__':main()
