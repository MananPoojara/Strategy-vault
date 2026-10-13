import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import vm from 'node:vm';
const html=readFileSync(new URL('../../frontend/full-variant1.html',import.meta.url),'utf8');
const source=html.slice(html.indexOf('async function getSum('),html.indexOf('// "before the book"'));
test('early navigation cannot cache an empty summary before owners initialize',async()=>{
 const state={API:'/api/in',SUM:{},owners:[],guruIds:()=>state.owners,URLSearchParams,fetch:async url=>({ok:true,json:async()=>[{id:'actual',stats:{trades:2},url}]})};vm.createContext(state);vm.runInContext(source,state);
 assert.equal(Object.keys(await state.getSum('NIFTY 50')).length,0);assert.equal(Object.keys(state.SUM).length,0);
 state.owners=['chande'];const result=await state.getSum('NIFTY 50');assert.equal(result.actual.stats.trades,2);assert.ok(result.actual.url.startsWith('/api/in/'));
});
test('summary retries keep the captured market rather than mix engines after navigation',async()=>{
 let calls=0;const urls=[];const state={API:'/api/in',SUM:{},guruIds:()=>['chande'],URLSearchParams,setTimeout:fn=>fn(),fetch:async url=>{urls.push(url);if(!calls++){state.API='/api/us';return {ok:false,text:async()=> 'temporary'};}return {ok:true,json:async()=>[{id:'actual',stats:{trades:2}}]};}};vm.createContext(state);vm.runInContext(source,state);
 const result=await state.getSum('NIFTY 50');assert.equal(result.actual.stats.trades,2);assert.equal(urls.length,2);assert.ok(urls.every(url=>url.startsWith('/api/in/')));
});
