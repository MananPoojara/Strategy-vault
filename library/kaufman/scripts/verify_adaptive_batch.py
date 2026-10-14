#!/usr/bin/env python3
"""Independent TSM4 CD/daily-rule oracle; no Rust calculations are translated here.

Compile the actual kftsm.rs and connors.rs files. Compare chronological records,
including partial exits and terminal marks, against Decimal source predicates.
Read inputs only; write all normalized data and results to a private job directory.
The previous batch's I/O and arithmetic utilities are reused, never its oracle.
"""
from __future__ import annotations

import argparse
import csv
import io
import json
import subprocess
from decimal import Decimal as D, getcontext
from pathlib import Path

from verify_source_batch import REPO, average, crossed, read_data, sha

getcontext().prec = 40
ADAPT = {'period': 10, 'sdfilter': .1, 'mfilter': 0}
DRIVER = 'Chapter17/TSM(S) Adaptive systems.txt'
METHODS = {
    'kftsm_kama': dict(pages=[603, 604, 605], cd=[DRIVER, "Chapter17/TSM(F) KAMA Kaufman's Adaptive.txt"], params=ADAPT,
        rule='CD system1: ER=abs(C-C[n])/sum(abs(daily changes),n), zero noise ER1; sc=(ER*(.666-.0645)+.0645)^2; AMA=priorAMA+sc*(C-priorAMA). Buy/reverse on positive change beyond BOTH filters; short mirror.',
        assumptions=['First observed close seeds AMA; unavailable initial ER history uses ER1. Missing TradeStation MaxBarsBack/prehistory prevents original-platform initialization certification. CD function takes precedence over the different printed initialization.']),
    'kftsm_adapt_exp': dict(pages=[603, 605], cd=[DRIVER], params=ADAPT,
        rule='CD system0: SmoothedAverage(close,n); signed daily trend change beyond BOTH filters enters/reverses at close.',
        assumptions=['Platform SmoothedAverage initialization is not supplied: first full-period mean seed, then alpha1/n is the declared engine convention.']),
    'kftsm_vidya': dict(pages=[605, 606], cd=[DRIVER, 'Chapter17/TSM(F) VIDYA.txt'], params={**ADAPT, 'per2': 50},
        rule='CD system3: k=populationSD(C,n)/populationSD(C,histper); sc=2/(n+1); VIDYA=k*sc*C+(1-k*sc)*priorVIDYA. Apply the shared signed-change driver.',
        assumptions=['Require histper>n and its complete observed window. Seed at first complete historical window with that close; earlier trend values unavailable. Historical variance0 carries prior trend (explicit undefined-division convention), not a substituted close. No k clipping; MaxBarsBack/platform seed not certified.']),
    'kftsm_adapt_r2': dict(pages=[608], cd=[DRIVER, 'Chapter17/TSM(F) Adaptive R2.txt'], params=ADAPT,
        rule='CD system2: R2=Pearson(time,C,n)^2; AR2=priorAR2+R2*(C-priorAR2). Apply shared signed-change driver. Does not use printed (r+1)/2 or regression residuals.',
        assumptions=['Observed closes seed until complete correlation window exists. Zero price variance gives R2=0 and carries prior trend; missing platform prehistory is not invented.']),
    'kftsm_adapt_rsi': dict(pages=[608, 609], cd=[DRIVER, 'Chapter17/TSM(F) Adaptive RSI.txt'], params=ADAPT,
        rule='Author-described extension of CD driver: TSMAdaptiveRSI uses sc=abs(WilderRSI(C,n)/100-.5)*2; ARSI=priorARSI+sc*(C-priorARSI), first bar close seed. Apply the shared signed-change driver with close reversal.',
        assumptions=['The CD system introduction explicitly describes inserting this supplied function; it has no delivered fifth selector. period10/sdfilter.1/mfilter0 are inherited configurable driver defaults; the printed RSI indicator period20 is not substituted. Engine Wilder RSI seeds gains/losses from first n actual changes; RSI50 when both zero, RSI100 when only losses zero. Carry ARSI until RSI exists; platform MaxBarsBack/initialization is not certified.']),
    'kftsm_swg': dict(pages=[278, 288, 289], cd=['Chapter05/TSM(S) Swing System.txt', 'Chapter05/TSM(F) Swing signal.txt', 'Chapter08/TSM(S) Swing breakout.txt', 'Chapter08/TSM(F) Swing signal.txt'], params={'swing': 2.5},
        rule='Type0 only. Initialize current extrema from first close. Update high then test low<currentHigh-swing*C/100 when seeking high; ELSE update low then test high>currentLow+swing*C/100. Fix swing extrema. Down-leg current low below fixed swing low latches short; up-leg current high above fixed swing high latches long. Fill latched signal at close; reverse only.',
        assumptions=['CD Swing System input swing0 is unusable; configurable2.5% is borrowed from the separate TSM Swing input, explicitly a research default. Unestablished swing levels stay unavailable instead of the CD zero-value startup buy. This program is not the same algorithm as the before-update extremum-scaled TSM Swing.']),
    'kftsm_swing': dict(pages=[147, 149], cd=['Chapter05/TSM(S) Swing method.txt'], params={'swing': 2.5},
        rule='Type0 only. BEFORE updating today, high>fixedHigh with position<=0 buys at close; ELSE low<fixedLow with position>=0 sells. Then two sequential IF blocks favor reversals: low<currentHigh*(1-swing%) fixes high, else extend high; high>currentLow*(1+swing%) fixes low, else extend low. Reverse at opposite close signal.',
        assumptions=['Unestablished swing levels are unavailable rather than literal zero initialization. If both old levels break, source IF/ELSE and pre-close position choose the close order; no intraday stop execution is inferred.']),
    'kftsm_raschke_first_cross': dict(pages=[276], cd=['Chapter08/TSM(S) Raschke 1st cross.txt'], params={'osc1': 3, 'osc2': 10, 'ntrend': 16, 'vfilter': 0},
        rule='Fast oscillator=SMA3-SMA10; trend=SMA16(oscillator). Trend crossing0 resets first-pullback state and maximum reach. For positive setup, fastOsc[-1]>todayTrend and fastOsc<=todayTrend arms entry; higher low buys at close. Negative setup mirrors with >= and lower high. While held, a confirming close first arms stage1; subsequent adverse close exits. Use CD short comparison, resolving the printed typo.',
        assumptions=["Exit is Kaufman's suggested completion, not Raschke's own exit. CD nvlty5 is unused by the uncommented predicate. Pre-close position governs state tests. Opposite entry and old-position exit at the same close resolve as a single reversal at that price."]),
    'kftsm_outside_day': dict(pages=[125], cd=['Chapter04/TSM(S) Outside day.txt'], params={'length': 1, 'hold': 3, 'stop_ref': 0},
        rule='Today H>prior-day H AND L<prior-day L. Close above prior H buys at that close, below prior L shorts. Stop at outside-day L for long/H for short begins NEXT day. Strict penetration fills boundary or observed open through it; otherwise exit at close3 bars after entry. Old-position stop executes before close orders. Opposite close signal reverses; same-direction signal ignored.',
        assumptions=['Just below/above has no numeric tick offset. Strict boundary penetration with boundary fill is an explicit zero-offset research approximation, never a recovered tick size. Default follows prose1/3/outside extreme; CD2/2/prior-range stop differs. No pyramiding or same-direction clock reset. Nondefault range/holding/stop_ref are research variants, not literal CD parity.']),
    'kftsm_fc_bb_abx': dict(pages=[269], cd=['Chapter08/TSM(S) FC Bollinger ABX.txt'], params={'period': 21, 'nsd': 2},
        rule='SMA21 and populationSD21; outer +/-2SD, inner +/-1SD. Cross above/below outer enters two equal halves at NEXT observed open. Long cross below innerUpper or MA each exits one remaining half next open; short mirrors. Both crossings close both. Opposite entry reverses remainder and enters two new halves; no pyramiding. Final-close signals unfilled.',
        assumptions=['Source two-contract allocation becomes equal0.5 research weights, not contract dollars or futures account sizing. Platform strategy order arbitration unspecified: opposite entry takes precedence over same-open exits because the old remainder closes in full. Repeated inner-band crossings may close the remaining half, as the CD has no first-exit latch. Terminal remainder is marked, not sold: final piece is mark with open=true.']),
}
COMMON = [
    'Selected baseline contracts only; parameter/instrument variations do not add methods.',
    'Flat at selected start; earlier actual bars warm indicators/setup state. One position per symbol. No pooled universe, leverage, borrow, funding, dividends, portfolio allocation or contract rolls invented.',
    'Source close-order convention retained; no intraday entry/stop path inferred. Terminal records marked open, excluded from closed-trade statistics.',
    'Existing close-order engine convention requires an entry bar before the selected last bar; last-close entry signals do not create a zero-duration position. A reversal at the last close liquidates its old position only. ABX next-open orders from a preceding close can fill the last observed open and remain marked open.',
    'Adaptive driver: CurrentBar=i+1; eligible when i+1>period+5 and abs(todayChange)>sdfilter*populationSD(previous20 changes) AND abs(todayChange)>mfilter. SD excludes today and requires full finite window. Position changes only on opposite eligible sign; no pyramiding.',
    'Adaptive mfilter0 is the reviewed baseline. Nonzero mfilter denotes a declared instrument-local point research threshold, not a USD/INR conversion.',
    'Actual selected Nifty cash-index prices are a declared application of pure indicator/percentage rules. Index short trades are hypothetical price exposure; ETF and stock cases use actual ETF and individual-stock prices. Current constituents do not imply historical membership.',
    'Original source studies/performance/contract-dollar results and vendor corporate-action adjustment quality are not reproduced or certified.',
]


def stdev(values, n):
    means = average(values, n) if all(v is not None for v in values) else [
        None if i+1 < n or any(v is None for v in values[i+1-n:i+1]) else sum(values[i+1-n:i+1], D(0))/n for i in range(len(values))]
    return [None if m is None else (sum((v-m)**2 for v in values[i+1-n:i+1])/n).sqrt() for i,m in enumerate(means)]


class Ledger:
    """A separate single-position ledger with explicit real exits versus marks."""
    def __init__(self, rows, first, last):
        self.rows, self.first, self.last = rows, first, last
        self.position, self.records = None, []

    @property
    def side(self):
        return 0 if self.position is None else self.position['dir']

    def leave(self, i, px, kind='close', signal=None):
        if self.position is not None:
            self.position.update(exit=i, exit_px=px, exit_kind=kind, open=False,
                                 signal_i=i if signal is None else signal)
            self.records.append(self.position)
            self.position = None

    def target(self, i, side, px, kind='close', allow_last=False):
        if i < self.first or self.side == side:
            return
        self.leave(i, px, kind, i-1 if kind=='open' else i)
        if side and (i < self.last or allow_last):
            self.position = dict(entry=i, entry_px=px, dir=side, entry_kind=kind, weight=D(1), pieces=[])

    def done(self):
        if self.position is not None:
            self.position.update(exit=self.last, exit_px=self.rows[self.last]['close'], exit_kind='mark', open=True, signal_i=self.last)
            self.records.append(self.position)
            self.position=None
        return self.records


def oracle(method, rows, first, last, parameters):
    p={k:D(str(v)) for k,v in parameters.items()}
    o,h,l,c = ([r[k] for r in rows] for k in ('open','high','low','close'))
    size=len(rows)
    ledger=Ledger(rows,first,last)
    if method in ('kftsm_kama','kftsm_adapt_exp','kftsm_vidya','kftsm_adapt_r2','kftsm_adapt_rsi'):
        n=int(p['period'])
        if method=='kftsm_adapt_exp':
            trend=[None]*size
            if size>=n:
                trend[n-1]=sum(c[:n])/n
                for i in range(n,size): trend[i]=(trend[i-1]*(n-1)+c[i])/n
        elif method=='kftsm_kama':
            trend=[c[0]]
            for i in range(1,size):
                er=D(1)
                if i>=n:
                    noise=sum(abs(c[j]-c[j-1]) for j in range(i-n+1,i+1))
                    if noise: er=abs(c[i]-c[i-n])/noise
                alpha=(er*D('.6015')+D('.0645'))**2
                trend.append(trend[-1]+alpha*(c[i]-trend[-1]))
        elif method=='kftsm_vidya':
            history=int(p['per2']); short_sd=stdev(c,n); long_sd=stdev(c,history)
            trend=[None]*size
            for i in range(history-1,size):
                if i==history-1: trend[i]=c[i]
                elif long_sd[i]==0: trend[i]=trend[i-1]
                else:
                    alpha=D(2)/(n+1)*short_sd[i]/long_sd[i]
                    trend[i]=alpha*c[i]+(1-alpha)*trend[i-1]
        elif method=='kftsm_adapt_r2':
            trend=c[:n-1]
            times=[D(j) for j in range(n)]; xm=sum(times)/n
            xx=sum((j-xm)**2 for j in times)
            for i in range(n-1,size):
                prices=c[i-n+1:i+1]; ym=sum(prices)/n
                yy=sum((v-ym)**2 for v in prices)
                xy=sum((x-xm)*(y-ym) for x,y in zip(times,prices))
                r2=xy*xy/(xx*yy) if yy else D(0)
                trend.append(trend[-1]+r2*(c[i]-trend[-1]))
        else:
            # Independently calculate Wilder's gain/loss recursion from source
            # price changes; never call the Rust/reference RSI implementation.
            strength=[None]*size;gains=[max(b-a,D(0)) for a,b in zip(c,c[1:])];losses=[max(a-b,D(0)) for a,b in zip(c,c[1:])]
            if size>n:
                gain=sum(gains[:n])/n;loss=sum(losses[:n])/n
                for i in range(n,size):
                    if i>n:gain=(gain*(n-1)+gains[i-1])/n;loss=(loss*(n-1)+losses[i-1])/n
                    strength[i]=(D(50) if gain==0 else D(100)) if loss==0 else 100-100/(1+gain/loss)
            trend=[c[0]]
            for i in range(1,size):
                alpha=abs(strength[i]/100-D('.5'))*2 if strength[i] is not None else D(0)
                trend.append(trend[-1]+alpha*(c[i]-trend[-1]))
        change=[None]+[None if a is None or b is None else b-a for a,b in zip(trend,trend[1:])]
        prior_sd=[None]+stdev(change,20)[:-1]
        for i in range(first,last+1):
            if i+1<=n+5 or change[i] is None or prior_sd[i] is None: continue
            if abs(change[i])>p['sdfilter']*prior_sd[i] and abs(change[i])>p['mfilter']:
                ledger.target(i,1 if change[i]>0 else -1,c[i])
    elif method in ('kftsm_swg','kftsm_swing'):
        current_high=current_low=c[0]; fixed_high=fixed_low=None; last_fixed=0; signal=0
        for i in range(last+1):
            if method=='kftsm_swing':
                if ledger.side<=0 and fixed_high is not None and h[i]>fixed_high:
                    ledger.target(i,1,c[i])
                elif ledger.side>=0 and fixed_low is not None and l[i]<fixed_low:
                    ledger.target(i,-1,c[i])
                if last_fixed!=1:
                    if l[i]<current_high*(1-p['swing']/100):
                        last_fixed=1;fixed_high=current_high;current_low=l[i]
                    elif h[i]>current_high: current_high=h[i]
                if last_fixed!=-1:
                    if h[i]>current_low*(1+p['swing']/100):
                        last_fixed=-1;fixed_low=current_low;current_high=h[i]
                    elif l[i]<current_low: current_low=l[i]
            else:
                distance=p['swing']*c[i]/100
                if last_fixed!=1:
                    current_high=max(current_high,h[i])
                    if l[i]<current_high-distance:
                        last_fixed=1;fixed_high=current_high;current_low=l[i]
                else:
                    current_low=min(current_low,l[i])
                    if h[i]>current_low+distance:
                        last_fixed=-1;fixed_low=current_low;current_high=h[i]
                if last_fixed==1 and fixed_low is not None and current_low<fixed_low:signal=-1
                if last_fixed==-1 and fixed_high is not None and current_high>fixed_high:signal=1
                if signal:ledger.target(i,signal,c[i])
    elif method=='kftsm_outside_day':
        n=int(p['length']);entry=0;stop=None
        for i in range(max(first,n),last+1):
            if ledger.side and i>entry:
                if (ledger.side==1 and l[i]<stop) or (ledger.side==-1 and h[i]>stop):
                    px=min(o[i],stop) if ledger.side==1 else max(o[i],stop)
                    ledger.leave(i,px,'stop')
                elif i-entry>=int(p['hold']):ledger.target(i,0,c[i])
            previous_high=max(h[i-n:i]);previous_low=min(l[i-n:i])
            if h[i]>previous_high and l[i]<previous_low:
                side=1 if c[i]>previous_high else -1 if c[i]<previous_low else 0
                if side and side!=ledger.side:
                    ledger.target(i,side,c[i]);entry=i
                    stop=(previous_low if side==1 else previous_high) if p['stop_ref'] else (l[i] if side==1 else h[i])
    elif method=='kftsm_raschke_first_cross':
        fast_mean=average(c,int(p['osc1']));slow_mean=average(c,int(p['osc2']))
        fast=[None if a is None or b is None else a-b for a,b in zip(fast_mean,slow_mean)]
        n=int(p['ntrend']);trend=[None]*size
        for i in range(n-1,size):
            vals=fast[i+1-n:i+1]
            if all(v is not None for v in vals):trend[i]=sum(vals)/n
        setup=pullback=stage=0;reach=D(0)
        for i in range(1,last+1):
            if trend[i] is None or trend[i-1] is None:continue
            held=ledger.side
            if trend[i-1]<=0<trend[i]:setup=-1;pullback=0;reach=trend[i]
            if trend[i-1]>=0>trend[i]:setup=1;pullback=0;reach=-trend[i]
            if trend[i]>0 and trend[i-1]>0:reach=max(reach,trend[i])
            if trend[i]<0 and trend[i-1]<0:reach=max(reach,-trend[i])
            order=None
            if reach>p['vfilter']:
                if held!=1 and setup<0:
                    if pullback==0 and fast[i-1]>trend[i] and fast[i]<=trend[i]:pullback=-1
                    if pullback==-1 and l[i]>l[i-1]:order=1;stage=0
                if held!=-1 and setup>0:
                    if pullback==0 and fast[i-1]<trend[i] and fast[i]>=trend[i]:pullback=1
                    if pullback==1 and h[i]<h[i-1]:order=-1;stage=0
            exit_now=False
            if held:
                if stage==0 and held*(c[i]-c[i-1])>0:stage=1
                if stage==1 and held*(c[i]-c[i-1])<0:exit_now=True;setup=0
            if order is not None:ledger.target(i,order,c[i])
            elif exit_now:ledger.target(i,0,c[i])
    elif method=='kftsm_fc_bb_abx':
        n=int(p['period']);mid=average(c,n);sd=stdev(c,n)
        bands={k:[None if m is None else m+factor*p['nsd']*s for m,s in zip(mid,sd)]
               for k,factor in [('outer_up',D(1)),('outer_down',D(-1)),('inner_up',D('.5')),('inner_down',D('-.5'))]}
        position=None;pending=None;records=[]

        def piece(i,price,weight,kind):
            position['pieces'].append(dict(i=i,px=price,weight=weight,kind=kind))

        def finish(i,open_record=False):
            position.update(exit=i,exit_px=sum(v['px']*v['weight'] for v in position['pieces']),
                            open=open_record,exit_kind='mark' if open_record else 'open',signal_i=i if open_record else i-1)
            # Engine stores a single full exit directly, multi-part exits explicitly.
            if len(position['pieces'])==1:position['pieces']=[]
            position.pop('left');records.append(position)

        for i in range(first,last+1):
            if pending:
                entry_side,exit_count=pending
                if entry_side:
                    if position:
                        piece(i,o[i],position['left']/2,'open');finish(i)
                    position=dict(entry=i,entry_px=o[i],dir=entry_side,entry_kind='open',weight=D(1),pieces=[],left=D(2))
                elif exit_count and position:
                    count=min(D(exit_count),position['left']);piece(i,o[i],count/2,'open');position['left']-=count
                    if position['left']==0:finish(i);position=None
            pending=None
            side=position['dir'] if position else 0
            if crossed(c,bands['outer_up'],i,1) and side!=1:pending=(1,0)
            elif crossed(c,bands['outer_down'],i,-1) and side!=-1:pending=(-1,0)
            elif side:
                count=int(crossed(c,bands['inner_up' if side==1 else 'inner_down'],i,-side))+int(crossed(c,mid,i,-side))
                pending=(0,count)
        if position:
            piece(last,c[last],position['left']/2,'mark');finish(last,True)
        return records
    else:raise ValueError(method)
    return ledger.done()


def build(output):
    src=REPO/'Context/guru/backend/src'
    harness=output/'actual-adaptive-harness.rs'
    harness.write_text('''#![allow(dead_code)]
#[derive(Clone)] pub struct Row { pub ticker:String,pub date:String,pub open:f64,pub high:f64,pub low:f64,pub close:f64,pub volume:Option<f64> }
#[path=CONNORS] mod connors;
#[path=KAUFMAN] mod kftsm;
mod books {
 use crate::connors::{Ctx,Params,ParamDef,StrategyDef,Trade};
 pub fn catalogue()->Vec<StrategyDef>{vec![]}
 pub fn family_of(_: &str)->Option<(&'static str,&'static str,&'static str)>{None}
 pub fn params(_: &str)->Option<Vec<ParamDef>>{None}
 pub fn run(_: &str,_: &Ctx,_:usize,_:usize,_:&Params)->Option<Vec<Trade>>{None}
}
fn main(){
 let a:Vec<String>=std::env::args().collect();
 let data=std::fs::read_to_string(&a[1]).unwrap();
 let rows:Vec<Row>=data.lines().skip(1).map(|line| {let p:Vec<&str>=line.split(',').collect();
  Row{ticker:"REAL".into(),date:p[0].into(),open:p[1].parse().unwrap(),high:p[2].parse().unwrap(),low:p[3].parse().unwrap(),close:p[4].parse().unwrap(),volume:None}}).collect();
 let x=connors::Ctx::new(&rows,&rows,&[]);
 let params:connors::Params=a[5].split(';').filter(|s|!s.is_empty()).map(|s|{let(k,v)=s.split_once('=').unwrap();(k.into(),v.parse().unwrap())}).collect();
 let trades=kftsm::run(&a[2],&x,a[3].parse().unwrap(),a[4].parse().unwrap(),&params).expect("Rejected parameters/range");
 println!("entry\\texit\\tdir\\tentry_px\\texit_px\\topen\\tentry_kind\\tweight\\tsignal_i\\tpieces\\treason");
 for t in trades {assert_eq!(t.legs.len(),1);
  let pieces=t.exits.iter().map(|e|format!("{}:{:.17}:{:.17}",e.0,e.1,e.2)).collect::<Vec<_>>().join("|");
  println!("{}\\t{}\\t{}\\t{:.17}\\t{:.17}\\t{}\\t{}\\t{:.17}\\t{}\\t{}\\t{}",t.legs[0].i,t.exit_i,t.dir,t.legs[0].px,t.exit_px,t.open,t.entry_kind,t.legs[0].w,t.signal_i,pieces,t.reason);
 }
}
'''.replace('CONNORS',json.dumps(str(src/'connors.rs'))).replace('KAUFMAN',json.dumps(str(src/'books/kftsm.rs'))))
    binary=output/'actual-adaptive-harness'
    commands=[['rustc','--edition=2021','-C','opt-level=1','-C','codegen-units=1',str(harness),'-o',str(binary)],
              ['rustc','--edition=2021','--test','-C','codegen-units=1',str(harness),'-o',str(output/'actual-adaptive-tests')]]
    for command in commands:
        r=subprocess.run(command,capture_output=True,text=True,timeout=180)
        (output/(Path(command[-1]).name+'-build.txt')).write_text(r.stdout+r.stderr)
        r.check_returncode()
    tests=[]
    for suite in ('kftsm::tests',):
        command=[str(output/'actual-adaptive-tests'),suite,'--test-threads=1']
        r=subprocess.run(command,capture_output=True,text=True,timeout=60)
        tests.append(r.stdout+r.stderr);r.check_returncode();commands.append(command)
    (output/'rust-tests.txt').write_text('\n'.join(tests))
    return binary,commands


def rust_records(text,method):
    result=[]
    for row in csv.DictReader(io.StringIO(text),delimiter='\t'):
        r={k:int(D(row[k])) for k in ('entry','exit','dir','signal_i')}
        r.update({k:D(row[k]) for k in ('entry_px','exit_px','weight')})
        r.update(open=row['open']=='true',entry_kind=row['entry_kind'],reason=row['reason'],pieces=[])
        r['exit_kind']='mark' if r['open'] else ('open' if method=='kftsm_fc_bb_abx' else 'stop' if row['reason']=='outside-day stop' else 'close')
        pieces=row['pieces'].split('|') if row['pieces'] else []
        for j,item in enumerate(pieces):
            i,px,weight=item.split(':')
            r['pieces'].append(dict(i=int(i),px=D(px),weight=D(weight),kind='mark' if r['open'] and j==len(pieces)-1 else 'open'))
        result.append(r)
    return result


def compare(actual,expected):
    differences=[]
    if len(actual)!=len(expected):differences.append(dict(kind='count',actual=len(actual),expected=len(expected)))
    def equal_number(a,b):return abs(a-b)<=max(D('0.00000001'),abs(b)*D('0.000000000001'))
    for i,(a,b) in enumerate(zip(actual,expected)):
        for k in ('entry','exit','dir','open','entry_kind','exit_kind'):
            if a[k]!=b[k]:differences.append(dict(row=i,field=k,actual=a[k],expected=b[k]))
        for k in ('entry_px','exit_px','weight'):
            if not equal_number(a[k],b[k]):differences.append(dict(row=i,field=k,actual=str(a[k]),expected=str(b[k])))
        if len(a['pieces'])!=len(b['pieces']):differences.append(dict(row=i,field='pieces.count',actual=len(a['pieces']),expected=len(b['pieces'])))
        for j,(ap,bp) in enumerate(zip(a['pieces'],b['pieces'])):
            for k in ('i','kind'):
                if ap[k]!=bp[k]:differences.append(dict(row=i,piece=j,field=k,actual=ap[k],expected=bp[k]))
            for k in ('px','weight'):
                if not equal_number(ap[k],bp[k]):differences.append(dict(row=i,piece=j,field=k,actual=str(ap[k]),expected=str(bp[k])))
        # Older Bk tracks the stop/time execution day, whereas ABX has next-open signal metadata.
        if a['signal_i']!=b['signal_i']:differences.append(dict(row=i,field='signal_i',actual=a['signal_i'],expected=b['signal_i']))
    return differences


def run_rust(binary,canonical,method,first,last,params):
    command=[str(binary),str(canonical),method,str(first),str(last),';'.join(f'{k}={v}' for k,v in params.items())]
    r=subprocess.run(command,capture_output=True,text=True,timeout=60);r.check_returncode()
    return rust_records(r.stdout,method),command


def enrich(records,rows):
    return [{**r,'entry_date':rows[r['entry']]['date'],'exit_date':rows[r['exit']]['date'],
             'pieces':[{**p,'date':rows[p['i']]['date']} for p in r['pieces']]} for r in records]


def main():
    ap=argparse.ArgumentParser();ap.add_argument('--manifest',type=Path,required=True);ap.add_argument('--output',type=Path,required=True)
    args=ap.parse_args();cfg=json.loads(args.manifest.read_text());output=args.output.resolve();output.mkdir(parents=True,exist_ok=True)
    selected=cfg.get('methods',list(METHODS))
    if not selected or any(m not in METHODS for m in selected):raise ValueError('Only this bounded reviewed set may run')
    source=Path(cfg['source_root']);book=source/cfg['book_path'];cd=source/cfg['cd_path'];digest=sha(book)
    contracts={m:{**METHODS[m],'pdf_path':str(book),'pdf_sha256':digest,
                  'cd_sources':[dict(path=str(cd/f),sha256=sha(cd/f)) for f in METHODS[m]['cd']]} for m in selected}
    modules=[REPO/'Context/guru/backend/src'/p for p in ('books/kftsm.rs','connors.rs')]
    hashes={str(p):sha(p) for p in modules};binary,commands=build(output)
    report=dict(schema_version=2,source_reviewed=True,implementation_ready=False,admitted=False,ledger_checked=False,checked_trades=0,
        source_performance_reproduced=False,verification_scope='selected-baseline-ledger-check',method_ids=selected,
        source_contracts=contracts,common_adaptations=COMMON,rust_module_sha256=hashes,oracle_path=str(Path(__file__).resolve()),
        oracle_sha256=sha(Path(__file__)),utility_sha256=sha(Path(__file__).with_name('verify_source_batch.py')),
        binary_sha256=sha(binary),harness_sha256=sha(output/'actual-adaptive-harness.rs'),build_commands=commands,
        exit_mode_evidence='Engine stores entry_kind and exit reason, not a separate exit_kind field. Exit modes are annotated from the actual runner branch/reason; ABX open terminal final piece is a mark. Raw reasons and pieces are preserved.',
        checks=[],edge_checks=[],summary='')
    for spec in cfg['datasets']:
        path=Path(spec['path']);before=path.stat();input_sha=sha(path);rows=read_data(spec)
        first=next(i for i,r in enumerate(rows) if r['date']>=cfg['from']);last=max(i for i,r in enumerate(rows) if r['date']<=cfg['to'])
        canonical=output/(spec['symbol']+'-actual.csv')
        with canonical.open('w',newline='') as f:
            w=csv.writer(f);w.writerow(['date','open','high','low','close']);w.writerows([r['date'],r['open'],r['high'],r['low'],r['close']] for r in rows)
        for method in selected:
            params=METHODS[method]['params'];expected=oracle(method,rows,first,last,params)
            actual,command=run_rust(binary,canonical,method,first,last,params);differences=compare(actual,expected)
            ledger_path=output/(method+'-'+spec['symbol']+'-ledger.json')
            ledger_path.write_text(json.dumps(dict(actual=enrich(actual,rows),expected=enrich(expected,rows),mismatches=differences),default=str,indent=2)+'\n')
            closed=[r for r in expected if not r['open']];returns=[D(r['dir'])*(r['exit_px']/r['entry_px']-1) for r in closed]
            check=dict(method=method,symbol=spec['symbol'],instrument=spec['instrument'],parameters=params,
                actual_records=len(actual),expected_records=len(expected),row_mismatches=len(differences),first_mismatches=differences[:5],
                ohlc_path=str(path),ohlc_sha256=input_sha,rust_module_sha256=hashes,canonical_sha256=sha(canonical),
                warmup_first=rows[0]['date'],loaded_bars=len(rows),selected_first=rows[first]['date'],selected_last=rows[last]['date'],
                population='one actual supplied instrument; current-constituent selection is not historical membership',
                closed_trades=len(closed),open_records=sum(r['open'] for r in expected),gross_closed_return_sum=str(sum(returns,D(0))),
                ledger_path=str(ledger_path),command=command)
            report['checks'].append(check)
            if not differences:report['checked_trades']+=len(actual)
            print(json.dumps(dict(method=method,symbol=spec['symbol'],records=len(actual),mismatches=len(differences))),flush=True)
            if method=='kftsm_fc_bb_abx' and expected:
                # Use an actual baseline entry's preceding close as a fresh flat
                # start. Confirm its signal cannot fill without a following open,
                # then extend by exactly one observed bar and compare that fill.
                fill_bar=expected[0]['entry'];signal_bar=fill_bar-1
                terminal,terminal_command=run_rust(binary,canonical,method,signal_bar,signal_bar,params)
                extended,extended_command=run_rust(binary,canonical,method,signal_bar,fill_bar,params)
                wanted=oracle(method,rows,signal_bar,fill_bar,params);edge_diff=compare(extended,wanted)
                report['edge_checks'].append(dict(method=method,symbol=spec['symbol'],kind='actual outside-band close: absent next open is unfilled; one-bar extension fills observed open',
                    signal_date=rows[signal_bar]['date'],fill_date=rows[fill_bar]['date'],unfilled_records=len(terminal),filled_records=len(extended),
                    expected_open=str(rows[fill_bar]['open']),row_mismatches=len(edge_diff)+(len(terminal)!=0)+(len(extended)!=1),
                    commands=[terminal_command,extended_command]))
                edge_path=output/(method+'-'+spec['symbol']+'-unfilled-check.json')
                edge_path.write_text(json.dumps(dict(terminal=enrich(terminal,rows),extended=enrich(extended,rows),expected=enrich(wanted,rows),mismatches=edge_diff),default=str,indent=2)+'\n')
            # Re-run on the same real prefix: a following bar must not fill terminal close signals.
            # Keep to identical, cut only future bars from context. All marks/fills must agree.
            if spec['symbol']=='NIFTY50':
                prefix=output/'NIFTY50-prefix-actual.csv'
                if not prefix.exists():
                    with prefix.open('w',newline='') as f:
                        w=csv.writer(f);w.writerow(['date','open','high','low','close']);w.writerows([r['date'],r['open'],r['high'],r['low'],r['close']] for r in rows[:last+1])
                prefix_actual,prefix_command=run_rust(binary,prefix,method,first,last,params)
                prefix_diff=compare(prefix_actual,expected)
                report['edge_checks'].append(dict(method=method,symbol=spec['symbol'],kind='future-bar truncation / unfilled terminal signals',row_mismatches=len(prefix_diff),command=prefix_command))
        after=path.stat()
        if (before.st_size,before.st_mtime_ns)!=(after.st_size,after.st_mtime_ns) or sha(path)!=input_sha:raise RuntimeError('Read-only input changed')
    if any(sha(Path(p))!=digest for p,digest in hashes.items()):raise RuntimeError('Compiled Rust source changed during audit')
    okay=all(c['row_mismatches']==0 and c['actual_records']>0 for c in report['checks']) and all(c['row_mismatches']==0 for c in report['edge_checks'])
    report.update(implementation_ready=okay,ledger_checked=okay,new_original_book_algorithms=0,summary=f'{len(selected)} reviewed research contracts (existing IDs and declared RSI driver extension); {len(report["checks"])} real-series baseline cases; {report["checked_trades"]} matched actual records. Zero new original book algorithms; prior17 excluded; admission false.')
    report_path=output/'ledger-report.json';report_path.write_text(json.dumps(report,default=str,indent=2)+'\n')
    print(json.dumps(dict(report=str(report_path),passed=okay,checked_trades=report['checked_trades'])))
    if not okay:raise SystemExit(1)


if __name__=='__main__':main()
