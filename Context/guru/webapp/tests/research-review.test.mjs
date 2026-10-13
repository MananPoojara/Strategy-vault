import {test} from 'node:test';
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
const require=createRequire(import.meta.url);
const {review}=require('../../frontend/research-review.js');
test('recorded missing data and unresolved rules remain distinct independent blockers',()=>{
 const {blockers}=require('../../frontend/research-review.js');
 const e={missing_data:['Actual futures OHLC and roll history','Contract tick values'],rule_gaps:['Partial exit quantities and final stop rules']};
 const b=blockers(e);
 assert.deepEqual(b,[{status:'needs_data',label:'Data missing',items:e.missing_data},{status:'not_testable',label:'Rules unresolved',items:e.rule_gaps}]);
 assert.deepEqual(blockers({missing_data:['Intraday quotes'],rule_gaps:[]}),[{status:'needs_data',label:'Data missing',items:['Intraday quotes']}]);
 assert.deepEqual(blockers({rule_gaps:['Exit not specified']}),[{status:'not_testable',label:'Rules unresolved',items:['Exit not specified']}]);
 assert.deepEqual(blockers(null),[]);
 assert.deepEqual(blockers({missing_data:[null,''],rule_gaps:['  ']}),[]);
});
test('unavailable methods receive a source assessment without invented results',()=>{
 const r=review({name:'Gimmee',guru:'ross'},null,{status:'not_testable',note:'Exit needs judgment'}, {conclusion:'96.5% is incorrect',pages:'p. 110',external:['https://www.bollingerbands.com/bollinger-band-rules'],verified_performance:false});
 assert.equal(r.verdict,'Backtest withheld');
 assert.ok(r.evidence.some(s=>s.includes('Exit needs judgment')));
 assert.ok(r.evidence.some(s=>s.includes('96.5%')));
 assert.ok(r.caveats.some(s=>s.includes('No independent trade')));
 assert.equal(r.links.length,1);
});
test('a real result is described without pretending its book fidelity is certified',()=>{
 const r=review({guru:'connors'},{stats:{trades:12},dates:['2020-01-01','2021-01-01']},null,null);
 assert.equal(r.verdict,'Computed result; source fidelity not certified');
 assert.ok(r.caveats.some(s=>s.includes('12')));
 assert.ok(r.caveats.some(s=>s.includes('full-history')));
});
test('unknown or malformed result cannot acquire a verified verdict or active unsafe link',()=>{
 const r=review({},null,null,{external:['javascript:alert(1)','https://evil.invalid/a','https://tradingeducators.com/edition-733']});
 assert.equal(r.verdict,'Backtest withheld');
 assert.deepEqual(r.links,['https://tradingeducators.com/edition-733']);
 const bad=review({},{stats:{trades:NaN},dates:[]},null,null);
 assert.equal(bad.verdict,'Backtest withheld');
});
test('blocked source assessment preserves the exact registry evidence and distinct data limitation',()=>{
 const source={path:'Books/Joe Ross/Reviewed chapter.pdf',sha256:'a'.repeat(64)};
 const e={source,pages:'printed pp. 59–67 / PDF pp. 30–34',conclusion:'Congestion takes precedence over a newly forming hook.',reason:'Contract tick history and automatic exits are absent.',verified_performance:false};
 const r=review({guru:'ross',rules:['E|This catalog text must not become a source quotation']},null,{note:'Intraday ordering is unavailable'},e);
 assert.deepEqual(r.assessment.map(x=>x.key),['source','rules','available_evidence','missing_evidence','optimization','prerequisite']);
 const row=key=>r.assessment.find(x=>x.key===key);
 assert.deepEqual(row('source').values,[source.path,e.pages,'SHA-256: '+source.sha256]);
 assert.deepEqual(row('rules').values,[e.conclusion]);
 assert.deepEqual(row('missing_evidence').values,['Intraday ordering is unavailable',e.reason]);
 assert.equal(row('optimization').status,'withheld');
 assert.match(row('optimization').values.join(' '),/No candidate ranking or performance conclusion/);
 assert.match(row('prerequisite').values.join(' '),/recorded rule and data limitations/);
 assert.ok(r.assessment.every(x=>Array.isArray(x.values)&&x.values.every(v=>typeof v==='string')));
 assert.ok(!JSON.stringify(r.assessment).includes('This catalog text'));
});
test('absent source evidence stays unknown and never acquires fabricated performance or source claims',()=>{
 const r=review({guru:'ross'},null,null,null);
 const row=key=>r.assessment.find(x=>x.key===key);
 assert.equal(row('source').status,'unavailable');
 assert.equal(row('rules').status,'unavailable');
 assert.match(row('available_evidence').values.join(' '),/No source-specific evidence record/);
 assert.equal(row('optimization').status,'withheld');
 assert.match(row('prerequisite').values.join(' '),/source-specific rule and data evidence/);
 assert.ok(!Object.keys(r).some(k=>['stats','top3','equity','score'].includes(k)));
});
test('ready assessment distinguishes a computed run from unavailable optimization and source certification',()=>{
 const r=review({guru:'connors'},{stats:{trades:0},dates:['2020-01-01','2021-01-01']},null,null);
 const row=key=>r.assessment.find(x=>x.key===key);
 assert.equal(r.verdict,'Computed result; source fidelity not certified');
 assert.equal(row('available_evidence').status,'computed');
 assert.equal(row('optimization').status,'unassessed');
 assert.match(row('optimization').values.join(' '),/computed run does not establish/);
 assert.equal(row('prerequisite').status,'required');
 assert.ok(!r.assessment.some(x=>x.status==='verified'));
});
