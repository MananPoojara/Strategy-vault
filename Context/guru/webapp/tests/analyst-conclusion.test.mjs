import {test} from 'node:test';
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
import {readFileSync} from 'node:fs';
import {runInNewContext} from 'node:vm';
const require=createRequire(import.meta.url);
const {analyze,metricKeys}=require('../../frontend/analyst-conclusion.js');
const captured=JSON.parse(readFileSync(process.env.VAULT_OPT_FIXTURE||new URL('../verification/joe-ross/optimization-3-row-input.json',import.meta.url),'utf8'));
function stats(res,row){return Object.fromEntries(res.cols.slice(res.dims.length).map((key,i)=>[key,row[i]]));}
function fixture(){
 const res=structuredClone(captured),def={hold:5};
 const book=stats(res,res.book),runs=[{name:'Book',book:true,p:def,m:book,d:{dates:[res.from,res.to]}}];
 res.rank.top3.forEach((c,i)=>{const row=res.rows[c.row];runs.push({name:'#'+(i+1),p:{hold:row[0]},c,m:stats(res,row.slice(1)),d:{dates:[res.from,res.to]}});});
 return {v:{id:'f1a',name:'Captured optimization'},tk:'NIFTY',res,runs,top:runs.slice(1),def,win:{from:res.from,to:res.to},pub:2010,cost:3,selectionLabel:'Custom dates'};
}
test('captured grid yields actual nine statistics, five-metric comparisons and verified rank without reranking',()=>{
 const D=fixture(),r=analyze(D);
 assert.equal(r.status,'computed');assert.equal(metricKeys.length,9);
 assert.deepEqual(r.candidates.map(c=>c.row),[1,0,2]);
 assert.equal(r.candidates[0].stats.cagr,.083466);
 assert.ok(Math.abs(r.candidates[0].comparisons.find(c=>c.key==='cagr').delta-(.083466-.085728))<1e-12);
 assert.equal(r.candidates[0].ranking.scoreVerified,true);
 assert.equal(r.candidates[0].ranking.groupsVerified,true);
 assert.equal(r.candidates[0].comparisons.find(c=>c.key==='max_dd').assessment,'improved');
 assert.equal(r.metadata.selection.label,'Custom dates');assert.equal(r.metadata.cost,3);
 assert.ok(r.limits.some(s=>s.includes('untouched holdout')));
 assert.ok(r.limits.some(s=>s.includes('independent confirmations')));
 assert.ok(r.candidates[0].summary.some(s=>s.includes('CAGR')));
 assert.ok(r.candidates[0].summary.some(s=>s.includes('measured tradeoffs')));
});
test('neighbor analysis uses full selection grid and retains fixed settings, never claims general robustness',()=>{
 const D=fixture();D.res.fixed={trend:250};const r=analyze(D),n=r.candidates[0].neighborhood;
 assert.equal(n.available,true);assert.equal(n.inspected,2);
 assert.deepEqual(n.rows,[0,2]);assert.equal(r.candidates[0].parameters.trend,250);
 assert.equal(n.metrics.find(m=>m.key==='cagr').min,.04088);
 assert.ok(n.note.includes('local sensitivity'));
 const slim=fixture();slim.res.picked=Object.fromEntries(slim.res.rows.map((r,i)=>[i,r]));slim.res.rows=[];
 assert.equal(analyze(slim).candidates[0].neighborhood.available,false);
 const partial=fixture();partial.res.rows.pop();assert.equal(analyze(partial).candidates[0].neighborhood.available,false);
});
test('missing/nonfinite data stays unavailable rather than becoming zero or an improvement',()=>{
 const D=fixture();D.runs[1].m={cagr:null,sharpe:NaN,total_return:Infinity,trades:null,max_dd:null};
 const c=analyze(D).candidates[0];
 assert.equal(c.stats.cagr,null);assert.equal(c.stats.net_profit,null);assert.equal(c.stats.trades,null);
 assert.equal(c.comparisons.find(m=>m.key==='cagr').assessment,'unavailable');
 assert.ok(c.flags.includes('Some evaluation statistics are unavailable.'));
});
test('before/after assessment reports actual counts and deltas and missing periods honestly',()=>{
 const D=fixture();D.runs[1].before={m:{cagr:.1,sharpe:-.2,total_return:.3,max_dd:-.2,trades:8}};
 D.runs[1].after={m:{cagr:.05,sharpe:-.1,total_return:.2,max_dd:-.1,trades:90}};
 const c=analyze(D).candidates[0];
 assert.equal(c.beforeAfter.before.trades,8);assert.equal(c.beforeAfter.after.trades,90);
 assert.ok(Math.abs(c.beforeAfter.deltas.find(m=>m.key==='sharpe').delta-.1)<1e-12);
 assert.ok(c.beforeAfter.summary.some(s=>s.includes('8')));
 assert.ok(!c.beforeAfter.summary.some(s=>s.includes('lost money')));
 assert.equal(analyze(D).candidates[1].beforeAfter.available,false);
});
test('negative leaders, one eligible outcome and losing selected choices cannot gain a positive endorsement',()=>{
 const D=fixture();D.res.rank.eligible=1;D.res.rows.forEach(r=>{r[1]=-.2;r[2]=-.1;r[3]=-.2;});
 D.runs.slice(1).forEach(r=>{r.m.total_return=-.2;r.m.net_profit=-.2;r.m.cagr=-.1;r.m.sharpe=-.2;});
 const out=analyze(D);assert.ok(out.flags.some(s=>s.includes('Only one')));
 assert.ok(out.flags.some(s=>s.includes('All selected candidates have negative')));
 const comparison=out.candidates[0].ranking.leaderComparisons.find(m=>m.key==='cagr');
 assert.equal(comparison.ratio,null);assert.ok(comparison.note.includes('non-positive'));
 assert.ok(out.candidates[0].summary.some(s=>s.includes('negative')));
});
test('altered rank math and group metadata is flagged while supplied candidate order is preserved',()=>{
 const D=fixture();D.res.rank.top3[0].score=55;D.res.rank.top3[0].weakest=5;D.res.rank.top3[0].groups=[];
 const r=analyze(D);assert.equal(r.candidates[0].ranking.scoreVerified,false);
 assert.equal(r.candidates[0].ranking.weakestVerified,false);assert.equal(r.candidates[0].ranking.groupsVerified,false);
 assert.deepEqual(r.candidates.map(c=>c.row),[1,0,2]);
});
test('withheld source-only methods get no synthetic candidates, statistics or unsupported dates',()=>{
 const out=analyze({v:{guru:'ross',name:'Ross Hook'},pub:null,cost:null});
 assert.equal(out.status,'withheld');assert.deepEqual(out.candidates,[]);assert.equal(out.book,null);
 assert.equal(out.metadata.publicationYear,null);assert.equal(out.metadata.cost,null);
 assert.ok(out.summary[0].includes('No supported optimization'));
});
test('text remains plain data and no input mutation occurs',()=>{
 const D=fixture();D.v.name='<img src=x onerror=alert(1)>';const before=JSON.stringify(D);
 const r=analyze(D);assert.equal(JSON.stringify(D),before);assert.equal(r.metadata.strategy,D.v.name);
 assert.equal(r.candidates[0].summary.every(s=>typeof s==='string'),true);
});
test('browser UMD supplies the same pure API without a DOM',()=>{
 const sandbox={};runInNewContext(readFileSync(new URL('../../frontend/analyst-conclusion.js',import.meta.url),'utf8'),sandbox);
 assert.equal(typeof sandbox.VaultAnalyst.analyze,'function');
 assert.equal(sandbox.VaultAnalyst.analyze(fixture()).candidates.length,3);
});
test('empty eligible pool returns book evidence with no invented candidate',()=>{
 const D=fixture();D.res.rank.top3=[];D.res.rank.eligible=0;
 const r=analyze(D);assert.equal(r.status,'computed');assert.equal(r.candidates.length,0);
 assert.equal(r.book.stats.cagr,.085728);assert.ok(r.summary.some(s=>s.includes('No eligible')));
});
test('absent, malformed or dimension-free neighbors cannot acquire a robustness statement',()=>{
 for(const mutation of [D=>{D.res.rows[0]=[];},D=>{D.res.dims=[];},D=>{D.res.dims[0].values=[9,10];},D=>{D.res.rows[2][0]=2;}]){
  const D=fixture();mutation(D);assert.equal(analyze(D).candidates[0].neighborhood.available,false);
 }
 const D=fixture();D.res.rank.top3[0].pct=[100,100,null,100,100];
 assert.equal(analyze(D).candidates[0].ranking.scoreVerified,false);
});

test('closed-trade sum remains distinct from compounded returns in the analyst output',()=>{
 const D=fixture();D.runs[1].m.net_profit=.2;D.runs[1].m.total_return=6000;
 assert.equal(analyze(D).candidates[0].stats.net_profit,.2);
 delete D.runs[1].m.net_profit;
 assert.equal(analyze(D).candidates[0].stats.net_profit,null);
 D.runs[1].m.closed_return_sum=.15;
 assert.equal(analyze(D).candidates[0].stats.net_profit,.15);
});
