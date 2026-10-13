import {createRequire} from 'node:module';
import test from 'node:test';
import assert from 'node:assert/strict';
const require=createRequire(import.meta.url),{authorAvailability:availability}=require('../../frontend/library-discovery.js');
const def=(id,book='one',family=id,requirement='index')=>({id,book,family,status:'live',instrument_policy:{requirement,allowed_types:requirement==='individual_stock'?['individual_stock']:['index']}});
const measured=(trades=5,net=.1)=>({stats:{trades,net_profit:net,total_return:.2,sharpe:null}});
test('reading books are counted without inventing strategy absence or a missing dataset',()=>{
 const result=availability(['one','two','one'],[def('foreign','other')],{},{});assert.equal(result.status,'reading_only');assert.equal(result.bookCount,2);assert.equal(result.readingBookCount,2);assert.equal(result.strategyCount,0);assert.equal(result.availableFamilies,0);assert.ok(!/missing data|verified/i.test(result.text));
});
test('actual closed results count families rather than variants and do not require defined Sharpe',()=>{
 const result=availability(['one'],[def('a','one','same'),def('b','one','same')],{a:measured(),b:measured()},{a:{status:'ready'},b:{status:'ready'}});assert.equal(result.status,'available');assert.equal(result.availableFamilies,1);assert.equal(result.availableVariants,2);assert.equal(result.familyCount,1);assert.ok(!/verified/i.test(result.title));
});
test('source or readiness blocks override old positive statistics',()=>{
 const result=availability(['one'],[def('a')],{a:measured(100)},{a:{status:'not_testable',note:'Source review required: rules are incomplete.'}});assert.equal(result.status,'source_review');assert.equal(result.availableVariants,0);
 const unknown=def('u');delete unknown.instrument_policy;assert.equal(availability(['one'],[unknown],{u:measured()},{}).status,'source_review');
});
test('stock mismatch requires an actual mismatch note and native benchmark gaps stay missing data',()=>{
 const methods=[def('stock','one','stock','individual_stock')];
 const wrong=availability(['one'],methods,{stock:{blocked:'Requires a validated individual stock; a cash index cannot replace its stock population.',readiness:'needs_data'}},{});assert.equal(wrong.status,'needs_stock_basket');
 const missing=availability(['one'],methods,{}, {stock:{status:'needs_data',note:'SPX benchmark volume/coverage is missing.'}});assert.equal(missing.status,'needs_data');
});
test('zero closed trades means measured no-trades only with a finite actual return',()=>{
 const methods=[def('a')];assert.equal(availability(['one'],methods,{a:measured(0,0)},{a:{status:'ready'}}).status,'no_trades');
 assert.equal(availability(['one'],methods,{}, {a:{status:'ready'}}).status,'no_eligible_results');
 assert.equal(availability(['one'],methods,{a:measured(0,NaN)},{}).status,'no_eligible_results');
 assert.equal(availability(['one'],methods,{a:measured(3,Infinity)},{}).availableVariants,0);
 assert.equal(availability(['one'],methods,{a:measured('3')},{}).availableVariants,0);
});
test('a mixed seventy-book author reports reading books and exact per-variant block counts',()=>{
 const books=Array.from({length:70},(_,i)=>'b'+i),methods=[def('yes','b0'),def('data','b1'),def('review','b2')];
 const result=availability(books,methods,{yes:measured()},{data:{status:'needs_data',note:'Option contract premiums are missing.'},review:{status:'not_testable',note:'Discretionary exit rules are not specified.'}});
 assert.equal(result.status,'available');assert.equal(result.bookCount,70);assert.equal(result.readingBookCount,67);assert.equal(result.strategyCount,3);assert.equal(result.counts.available,1);assert.equal(result.counts.needs_data,1);assert.equal(result.counts.source_review,1);assert.equal(result.variantReasons.length,3);
});
test('dominant blocked reason counts distinct families so many sibling variants cannot conceal another cause',()=>{
 const methods=[...Array.from({length:20},(_,i)=>def('v'+i,'one','same')),def('review1'),def('review2')];
 const ready=Object.fromEntries(methods.map(d=>[d.id,{status:d.id.startsWith('v')?'needs_data':'not_testable',note:d.id.startsWith('v')?'Intraday prices unavailable':'Source review required'}]));
 const result=availability(['one'],methods,{},ready);assert.equal(result.status,'source_review');assert.equal(result.counts.needs_data,20);assert.equal(result.familyCounts.needs_data,1);assert.equal(result.familyCounts.source_review,2);assert.equal(result.reasonCounts.find(r=>r.status==='needs_data').variants,20);
});
test('Map, keyed object and readiness document inputs agree and no input is mutated',()=>{
 const methods=[def('a')],summaries={a:measured()},ready={strategies:[{id:'a',status:'ready'}]},before=JSON.stringify([methods,summaries,ready]);
 const first=availability(['one'],methods,summaries,ready),second=availability(new Set(['one']),methods,new Map([['a',summaries.a]]),new Map([['a',ready.strategies[0]]]));assert.deepEqual(first,second);assert.equal(JSON.stringify([methods,summaries,ready]),before);
});
test('same family name in different books stays distinct and duplicate definitions do not inflate counts',()=>{
 const methods=[def('a','one','common'),def('b','two','common'),def('a','one','common')];const result=availability(['one','two'],methods,[{id:'a',...measured()},{id:'b',...measured()}],{});assert.equal(result.familyCount,2);assert.equal(result.availableFamilies,2);assert.equal(result.availableVariants,2);
});
test('explicit data requirements stay distinct from uncertain source rules',()=>{
 const result=availability(['one'],[def('f')],{}, {f:{status:'not_testable',note:'Requires actual futures contracts and expiry history.'}});assert.equal(result.status,'needs_data');
 const unclear=availability(['one'],[def('f')],{}, {f:{status:'needs_data',note:'Source review needed: subjective exits.'}});assert.equal(unclear.status,'source_review');
});
test('unsupported calculation is not relabelled as absent data and unknown policy names are withheld',()=>{
 const result=availability(['one'],[def('a')],{a:{blocked:'Nonpositive normalized equity: this model cannot publish results.',readiness:'calculation_unavailable'}},{});assert.equal(result.status,'no_eligible_results');assert.equal(result.counts.needs_data,0);
 const unknown=def('a');unknown.instrument_policy.requirement='new_unreviewed_type';assert.equal(availability(['one'],[unknown],{a:measured()},{}).status,'source_review');
});
test('unsupported source declarations or empty admitted types cannot be rescued by old stats',()=>{
 const stopped=def('a');stopped.status='not_testable';assert.equal(availability(['one'],[stopped],{a:measured()},{}).status,'source_review');
 const missing=def('a');missing.instrument_policy={requirement:'futures',allowed_types:[],source_basis:'Actual dated futures contract prices are required.'};assert.equal(availability(['one'],[missing],{a:measured()},{}).status,'needs_data');
});
