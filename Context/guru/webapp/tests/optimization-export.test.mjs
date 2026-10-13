import {test} from 'node:test';
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
import {readFileSync,existsSync} from 'node:fs';
const capture=process.env.VAULT_OPT_FIXTURE||new URL('../verification/joe-ross/optimization-3-row-input.json',import.meta.url);
const schemas=process.env.VAULT_PARAM_FIXTURE||new URL('../verification/reference/in-params.json',import.meta.url);
const require=createRequire(import.meta.url);
const {fullGrid}=require('../../frontend/optimization-export.js');
test('captured real engine response retains its p-prefixed parameter contract',{skip:!existsSync(capture)||!existsSync(schemas)},()=>{
 const res=JSON.parse(readFileSync(capture));
 const schema=JSON.parse(readFileSync(schemas));
 const table=fullGrid(res,schema[res.id]);
 assert.equal(res.cols[0],'p.hold');
 assert.equal(table.rows.length,3);
 assert.deepEqual(table.header.slice(-2),schema[res.id].map(s=>s.label));
 assert.deepEqual(table.rows.map(r=>r.at(-1)),[2,3,4]);
 assert.deepEqual(table.rows.map(r=>r[1]),res.rows.map(r=>r[1]));
});
test('full optimization export includes every row, metrics before all settings, including fixed ones',()=>{
 const res={cols:['p.hold','cagr','sharpe','trades'],dims:[{key:'hold',values:[2,3]}],rows:[[2,.1,.2,12],[3,.3,.4,14]],combos:2};
 const t=fullGrid(res,[{key:'trend',label:'Trend SMA',def:200},{key:'hold',label:'Hold bars',def:2}]);
 assert.deepEqual(t.header,['Run','CAGR','Sharpe','No. of trades','Trend SMA','Hold bars']);
 assert.deepEqual(t.rows,[[1,.1,.2,12,200,2],[2,.3,.4,14,200,3]]);
});
test('export rejects truncated grids and malformed column shapes',()=>{
 assert.throws(()=>fullGrid({cols:['p.hold','sharpe'],dims:[{key:'hold'}],rows:[[2,.5]],combos:10},[]),/complete/);
 assert.throws(()=>fullGrid({cols:['p.hold','sharpe'],dims:[{key:'hold'}],rows:[[2]],combos:1},[]),/columns/);
 assert.throws(()=>fullGrid({cols:['wrong','sharpe'],dims:[{key:'hold'}],rows:[[2,.5]],combos:1},[]),/parameter/);
});
test('fixed API settings override schema defaults in every exported row',()=>{
 const t=fullGrid({cols:['p.hold','sharpe'],dims:[{key:'hold'}],rows:[[2,.5]],fixed:{trend:100},combos:1},[{key:'trend',label:'Trend',def:200}]);
 assert.deepEqual(t.header,['Run','Sharpe','Trend','hold']);
 assert.deepEqual(t.rows,[[1,.5,100,2]]);
});
test('all columns survive and non-finite statistics are empty, never fabricated zero',()=>{
 const t=fullGrid({cols:['sharpe','unfamiliar_metric'],dims:[],rows:[[NaN,null]],combos:1},[]);
 assert.deepEqual(t.header,['Run','Sharpe','unfamiliar_metric']);
 assert.deepEqual(t.rows,[[1,null,null]]);
});
