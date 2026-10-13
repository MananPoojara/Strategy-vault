import test from 'node:test';
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
import {readFileSync} from 'node:fs';
import vm from 'node:vm';
const require=createRequire(import.meta.url), report=require('../../frontend/report-data.js');
const sandbox={}; vm.runInNewContext(readFileSync(new URL('../../frontend/optim-core.js',import.meta.url),'utf8'),sandbox); const core=sandbox.OptimCore;
const close=(a,b)=>assert.ok(Math.abs(a-b)<1e-12,`${a} != ${b}`);
test('first observation retains loss or fee from initial unit in first month and year',()=>{
 const d={dates:['2024-01-02','2024-01-03','2024-01-04'],equity:[.9,.99,.98]};
 close(report.periods(d,7)['2024-01'],-.02);close(report.periods(d,4)['2024'],-.02);close(core.monthly(d.dates,d.equity)['2024-01'],-.02);
 assert.equal(core.dsrOfEquity(d.equity,[.2,.4],2).T,3);
});
test('gain, flat, month boundary and year boundary chain back to measured final equity',()=>{
 for(const eq of [[1.1,1.21,1.452],[1,1,1],[.9,.99,.98]]){
  const d={dates:['2023-12-29','2024-01-31','2024-02-29'],equity:eq};
  for(const len of [4,7])close(Object.values(report.periods(d,len)).reduce((a,r)=>a*(1+r),1),eq.at(-1));
  close(report.periods(d,7)['2023-12'],eq[0]-1);
 }
});
test('buy and hold has zero first return and per-observation risk convention',()=>{
 const b=report.benchmark({d:['2024-01-02','2024-01-03','2024-01-04'],c:[100,110,99]},'2024-01-02','2024-01-04');
 const r=[0,.1,-.1],m=0,sd=.1;close(b.sharpe,m/sd*Math.sqrt(252));close(b.max_dd,-.1);close(b.total_return,-.01);
});
