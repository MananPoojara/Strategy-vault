import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import vm from 'node:vm';
import {loadOwnerRows} from '../../frontend/react/src/services/summary-loader.ts';
const runtime=readFileSync(new URL('../../frontend/react/src/compat/runtime.js',import.meta.url),'utf8');
const source=runtime.slice(runtime.indexOf('function summaryContext('),runtime.indexOf('// "before the book"'));
const response=(status,body)=>({ok:status===200,status,text:async()=>typeof body==='string'?body:JSON.stringify(body),json:async()=>body});
function harness(fetcher,patch={}){
 const state={API:'/api/in',ALLCAT:[{id:'actual',api:'chande'},{id:'failed',api:'connors'}],owners:['chande'],busy:1,
 TICK_ALL_IN:[{ticker:'NIFTY 50',first:'2000-01-03',last:'2026-10-07'}],TICK_ALL_US:[{ticker:'basket:us_sp500:AAPL',first:'2000-01-03',last:'2020-09-11'}],LABEL:{'NIFTY 50':'Nifty 50','basket:us_sp500:AAPL':'AAPL'},
 S:{cost:'0'},VaultUI:{costState:()=>({slippage:'0',rollover:'0'})},URLSearchParams,setTimeout:fn=>fn(),loadOwnerRows,fetch:fetcher,...patch};
 state.guruIds=()=>state.owners;state.ownerOf=id=>state.ALLCAT.find(v=>v.id===id)?.api;state.decodeP=()=>({});
 vm.createContext(state);vm.runInContext(source+'\nglobalThis.research={getSum,getReadiness,summaryIssues};',state);return state;
}
test('early navigation cannot cache an empty summary before owners initialize',async()=>{
 let calls=0;const state=harness(async()=>{calls++;return response(200,[{id:'actual',stats:{trades:2}}]);},{owners:[]});
 assert.equal(Object.keys(await state.research.getSum('NIFTY 50')).length,0);assert.equal(calls,0);
 state.owners=['chande'];assert.equal((await state.research.getSum('NIFTY 50')).actual.stats.trades,2);assert.equal(calls,1);
});
test('summary retries keep the captured market rather than mix engines after navigation',async()=>{
 let calls=0;const urls=[];let state;state=harness(async url=>{urls.push(url);if(!calls++){state.API='/api/us';return response(503,'temporary');}return response(200,[{id:'actual',stats:{trades:2}}]);});
 const result=await state.research.getSum('NIFTY 50');assert.equal(result.actual.stats.trades,2);assert.equal(urls.length,2);assert.ok(urls.every(url=>url.startsWith('/api/in/')));
});
test('one empty owner response preserves actual healthy statistics and withholds only failed IDs',async()=>{
 let failedCalls=0;const state=harness(async url=>{
 if(url.includes('/guru/connors/')){failedCalls++;return response(400,'empty date range');}
 return response(200,[{id:'actual',stats:{trades:2,net_profit:.01}}]);
 },{owners:['chande','connors']});
 const result=await state.research.getSum('NIFTY 50');assert.equal(result.actual.stats.trades,2);assert.equal(result.failed.readiness,'needs_data');assert.equal(result.failed.stats,undefined);assert.equal(failedCalls,1);
 const issues=state.research.summaryIssues('NIFTY 50');assert.equal(issues.length,1);assert.equal(issues[0].owner,'connors');assert.match(issues[0].note,/Nifty 50/);
});
test('a known disjoint stock range stays unavailable without a request or fabricated zero statistics',async()=>{
 let calls=0;const state=harness(async()=>{calls++;return response(200,[]);},{API:'/api/us',owners:['chande','connors']});
 const result=await state.research.getSum('basket:us_sp500:AAPL','2024-01-01','2025-12-31');
 assert.equal(calls,0);assert.ok(Object.values(result).every(row=>row.readiness==='needs_data'&&!row.stats));
 assert.match(state.research.summaryIssues('basket:us_sp500:AAPL','2024-01-01','2025-12-31')[0].note,/2020-09-11/);
});
test('readiness failures keep healthy owner readiness and do not rescue a failed owner',async()=>{
 const state=harness(async url=>url.includes('/guru/connors/')?response(400,'empty date range'):response(200,{strategies:[{id:'actual',status:'ready'}]}),{owners:['chande','connors']});
 const ready=await state.research.getReadiness('NIFTY 50');assert.equal(ready.actual.status,'ready');assert.equal(ready.failed.status,'needs_data');assert.equal(ready.failed.stats,undefined);
});
test('failed outcomes are shared within one render and retried after user navigation',async()=>{
 let calls=0,healthy=false;const state=harness(async()=>{calls++;return healthy?response(200,[{id:'actual',stats:{trades:2}}]):response(400,'empty date range');});
 await state.research.getSum('NIFTY 50');await state.research.getSum('NIFTY 50');assert.equal(calls,1);
 healthy=true;state.busy++;
 assert.equal((await state.research.getSum('NIFTY 50')).actual.stats.trades,2);assert.equal(calls,2);assert.equal(state.research.summaryIssues('NIFTY 50').length,0);
});
