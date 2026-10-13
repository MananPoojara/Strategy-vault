import test from 'node:test';
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
const trust=createRequire(import.meta.url)('../../frontend/backtest-trust.js');
test('unavailable or blocked results cannot be counted as tested',()=>{
 for(const row of [null,{}, {blocked:'Missing options',stats:{trades:12}}, {stats:null}]) assert.equal(trust.hasResult(row),false);
 assert.equal(trust.hasResult({stats:{trades:0}}),true);
});
test('data gaps, calculation gaps and undefined book rules have distinct labels',()=>{
 assert.equal(trust.statusLabel('needs_data'),'Data missing');
 assert.equal(trust.statusLabel('calculation_unavailable'),'Backtest calculation unavailable');
 assert.equal(trust.statusLabel('not_testable'),'Rules unresolved');
 assert.equal(trust.statusLabel('ready'),'Data checks passed');
 assert.equal(trust.statusLabel(null),'Backtest unavailable');
});
test('book figures are separate claims and are escaped',()=>{
 assert.match(trust.bookReference(null),/No published/);
 const html=trust.bookReference({period:'2002–2004, OEX',trades:'12',win:'100%',avg:'<script>bad</script>',hold:'-'});
 assert.match(html,/reference claims/);assert.match(html,/not been established/);
 assert.match(html,/&lt;script&gt;/);assert.doesNotMatch(html,/<script>/);assert.doesNotMatch(html,/Holding period/);
 assert.equal(trust.escape("&<>\"'"),'&amp;&lt;&gt;&quot;&#39;');
});
test('buy and sell arrow colors depend on action, regardless of return',()=>{
 const colors={buy:'green',sell:'red'},dates=['2020-01-01','2020-01-02','2020-01-03'],barOf=[0,1,2];
 for(const dir of [1,-1]) for(const ret of [-.1,.1]){
  const markers=trust.actionMarkers([{dir,ret,legs:[[0,100,1],[1,101,1]],exit_i:2,open:false}],dates,barOf,colors);
  assert.equal(markers.length,3);
  for(const marker of markers) assert.equal(marker.color,marker.shape==='arrowUp'?'green':'red');
  assert.equal(markers[1].size,.7);
 }
 const markers=trust.actionMarkers([{dir:1,legs:[[0,100,1]],exit_i:2,open:true}],dates,barOf,colors);
 assert.equal(markers.length,1);assert.equal(markers[0].time,dates[0]);
 assert.deepEqual(trust.actionMarkers([],dates,barOf,colors),[]);
});
