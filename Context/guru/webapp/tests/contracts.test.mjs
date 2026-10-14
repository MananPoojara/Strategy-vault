import { test } from 'node:test';
import assert from 'node:assert/strict';
import { validateNoteKey, validateTheme, roleAllows, safeReturnTo } from '../auth/src/contracts.mjs';

test('note identifiers accept existing strategy/market keys without path traversal', () => {
  for (const key of ['f1a.in', 'spmt_mts.us', 'ptjdt_adx15_breakout.in']) assert.equal(validateNoteKey(key), key);
  for (const key of ['', '../secrets', '.hidden', 'a/b', 'a'.repeat(81), 'a\n']) assert.throws(() => validateNoteKey(key));
});
test('themes match the existing interface and reject arbitrary CSS', () => {
  for (const theme of ['steel','aqua','lagoon','glacier']) assert.equal(validateTheme(theme), theme);
  for (const theme of ['purple', '<script>', '', null]) assert.throws(() => validateTheme(theme));
});
test('permissions distinguish reading, research and account administration', () => {
  assert.equal(roleAllows('viewer','read'), true);
  assert.equal(roleAllows('viewer','write'), false);
  assert.equal(roleAllows('viewer','optimize'), false);
  assert.equal(roleAllows('researcher','write'), true);
  assert.equal(roleAllows('researcher','optimize'), true);
  assert.equal(roleAllows('researcher','admin'), false);
  assert.equal(roleAllows('admin','admin'), true);
  assert.equal(roleAllows('unknown','read'), false);
});
test('post-login destinations cannot redirect to another origin', () => {
  assert.equal(safeReturnTo('/full-variant1.html#/s/f1a'), '/full-variant1.html#/s/f1a');
  for (const path of ['https://evil.invalid', '//evil.invalid', '/\\evil.invalid', '/login.html', '/\r/evil.invalid', '/\t/evil.invalid', null]) assert.equal(safeReturnTo(path), '/');
});

test('saved views accept local strategy routes and reject malformed destinations', async () => {
  const { validateSavedView } = await import('../auth/src/contracts.mjs');
  for (const value of ['#/', '#/strategies?region=us&tk=SPX', '#/s/f1a?from=2020-01-01', '#/guru/connors', '#/o/rsi2_5d?region=us', '#/guru/ross']) assert.equal(validateSavedView(value),value);
  for (const value of ['https://evil.invalid','#//evil.invalid','#/s/%zz','#/\n',null,'#/s/'+ 'x'.repeat(5000)]) assert.throws(()=>validateSavedView(value));
});

test('actual login script blocks control-character redirects', async () => {
  const { readFile } = await import('node:fs/promises');
  const { runInNewContext } = await import('node:vm');
  const source=(await readFile(new URL('../public/login.js',import.meta.url),'utf8')).split('\nfetch(')[0];
  for(const path of ['/\r/evil.invalid','/\t/evil.invalid','//evil.invalid','/\\evil.invalid']) {
    const next=runInNewContext(source+'\nnext',{URLSearchParams,location:{search:'?next='+encodeURIComponent(path)}});
    assert.equal(next,'/');
  }
});

test('login preserves a strategy fragment carried through the authentication redirect', async () => {
  const { readFile }=await import('node:fs/promises');const { runInNewContext }=await import('node:vm');
  const source=(await readFile(new URL('../public/login.js',import.meta.url),'utf8')).split('\nfetch(')[0];
  assert.equal(runInNewContext(source+'\nnext',{URLSearchParams,location:{search:'',hash:'#/s/f1a?region=us'}}),'/#/s/f1a?region=us');
});

test('whole-page themes are persisted without arbitrary CSS',()=>{
 for(const theme of ['midnight','forest','sand','plum','cobalt']) assert.equal(validateTheme(theme),theme);
});

test('research tags accept plain labels and restrict CEO recommendation changes to administrators',async()=>{
 const {validateAnnotation}=await import('../auth/src/contracts.mjs');
 assert.deepEqual(validateAnnotation({tags:['Reviewed','Reviewed',' India ']},'researcher'),{tags:['Reviewed','India']});
 assert.deepEqual(validateAnnotation({tags:[],ceo:true},'admin'),{tags:[],ceo:true});
 for(const data of [{tags:['a\nsecret']},{tags:Array(13).fill('x')},{tags:['x'.repeat(41)]},{tags:'x'},{tags:[],ceo:'true'}]) assert.throws(()=>validateAnnotation(data,'admin'));
 assert.throws(()=>validateAnnotation({tags:[],ceo:true},'researcher'));
});

const LAB_SPEC={v:1,direction:'long',entry:{all:[{lhs:{s:'rsi',p:{length:2}},op:'crosses_below',rhs:{value:10}}]},exit:{any:[{lhs:{s:'bars_held'},op:'>=',rhs:{value:10}}]}};
const LAB_ROW={id:7,owner:'u1',status:'draft',version:3,market:'in'};
const LAB_USERS={owner:{id:'u1',role:'researcher'},other:{id:'u2',role:'researcher'},viewer:{id:'u3',role:'viewer'},admin:{id:'u4',role:'admin'}};

test('lab strategies accept a trimmed name, a v1 spec object, a lab market and an instrument',async()=>{
 const {validateLabStrategy}=await import('../auth/src/contracts.mjs');
 assert.deepEqual(validateLabStrategy({name:'  RSI dip ',spec:LAB_SPEC,market:'crypto',instrument:' BTCUSD ',extra:1}),{name:'RSI dip',spec:LAB_SPEC,market:'crypto',instrument:'BTCUSD'});
 for(const body of [null,[],{...{spec:LAB_SPEC,market:'in',instrument:'X'},name:'   '},{name:'x'.repeat(121),spec:LAB_SPEC,market:'in',instrument:'X'},{name:'a\nb',spec:LAB_SPEC,market:'in',instrument:'X'},
  {name:'A',spec:'{"v":1}',market:'in',instrument:'X'},{name:'A',spec:[],market:'in',instrument:'X'},{name:'A',spec:{...LAB_SPEC,v:2},market:'in',instrument:'X'},{name:'A',spec:{v:1,entry:LAB_SPEC.entry},market:'in',instrument:'X'},{name:'A',spec:{v:1,entry:[],exit:{}},market:'in',instrument:'X'},
  {name:'A',spec:LAB_SPEC,market:'eu',instrument:'X'},{name:'A',spec:LAB_SPEC,market:'in',instrument:''},{name:'A',spec:LAB_SPEC,market:'in',instrument:'x'.repeat(65)},{name:'A',spec:{...LAB_SPEC,note:'a\u0000b'},market:'in',instrument:'X'}]) assert.throws(()=>validateLabStrategy(body),{status:400});
});

test('lab specs over 8 KB are refused before storage',async()=>{
 const {validateLabStrategy}=await import('../auth/src/contracts.mjs');
 const sized=n=>({...LAB_SPEC,pad:'x'.repeat(n-JSON.stringify({...LAB_SPEC,pad:''}).length)});
 assert.equal(Buffer.byteLength(JSON.stringify(sized(8192))),8192);
 assert.ok(validateLabStrategy({name:'A',spec:sized(8192),market:'in',instrument:'X'}));
 assert.throws(()=>validateLabStrategy({name:'A',spec:sized(8193),market:'in',instrument:'X'}),{status:400,message:/8 KB/});
 assert.throws(()=>validateLabStrategy({name:'A',spec:{...LAB_SPEC,pad:'é'.repeat(4000)},market:'in',instrument:'X'}),{message:/8 KB/});
});

test('lab versions and review notes are bounded',async()=>{
 const {validateLabVersion,validateLabNote}=await import('../auth/src/contracts.mjs');
 assert.equal(validateLabVersion(4),4);
 for(const v of [undefined,0,-1,1.5,'3',null]) assert.throws(()=>validateLabVersion(v),{status:400});
 assert.equal(validateLabNote(undefined,false),null);
 assert.equal(validateLabNote(' Needs a stop \n rule ',true),'Needs a stop \n rule');
 for(const note of [undefined,'  ','x'.repeat(1001),'a\u0000b',5]) assert.throws(()=>validateLabNote(note,true),{status:400});
});

test('another user cannot read or change a draft; published strategies are readable by every role',async()=>{
 const {labCanRead,labAction}=await import('../auth/src/contracts.mjs');
 assert.equal(labCanRead(LAB_ROW,LAB_USERS.owner),true);
 assert.equal(labCanRead(LAB_ROW,LAB_USERS.admin),true);
 for(const user of [LAB_USERS.other,LAB_USERS.viewer]){
  assert.equal(labCanRead(LAB_ROW,user),false);
  for(const action of ['update','delete','duplicate','submit']) assert.throws(()=>labAction(LAB_ROW,user,action,3),{status:404});
 }
 assert.equal(labCanRead(undefined,LAB_USERS.admin),false);
 const published={...LAB_ROW,status:'published'};
 for(const user of Object.values(LAB_USERS)) assert.equal(labCanRead(published,user),true);
 assert.equal(labAction(published,LAB_USERS.other,'duplicate'),'draft');
 assert.throws(()=>labAction(published,LAB_USERS.viewer,'duplicate'),{status:403});
 assert.throws(()=>labAction(LAB_ROW,LAB_USERS.admin,'update',3),{status:403,message:/owner/});
});

test('published lab strategies are frozen and statuses move only along draft → submitted → published | rejected',async()=>{
 const {labAction}=await import('../auth/src/contracts.mjs');
 const at=status=>({...LAB_ROW,status});
 assert.equal(labAction(at('draft'),LAB_USERS.owner,'update',3),'draft');
 assert.equal(labAction(at('rejected'),LAB_USERS.owner,'update',3),'draft');
 assert.equal(labAction(at('draft'),LAB_USERS.owner,'submit',3),'submitted');
 assert.equal(labAction(at('rejected'),LAB_USERS.owner,'submit',3),'submitted');
 assert.equal(labAction(at('submitted'),LAB_USERS.owner,'withdraw',3),'draft');
 assert.equal(labAction(at('submitted'),LAB_USERS.admin,'publish',3),'published');
 assert.equal(labAction(at('submitted'),LAB_USERS.admin,'reject',3),'rejected');
 for(const action of ['update','delete','submit','withdraw']) assert.throws(()=>labAction(at('published'),LAB_USERS.owner,action,3),{status:409});
 for(const action of ['publish','reject']) assert.throws(()=>labAction(at('published'),LAB_USERS.admin,action,3),{status:409});
 for(const action of ['update','delete','submit']) assert.throws(()=>labAction(at('submitted'),LAB_USERS.owner,action,3),{status:409});
 assert.throws(()=>labAction(at('draft'),LAB_USERS.admin,'publish',3),{status:409});
 assert.throws(()=>labAction(at('submitted'),LAB_USERS.owner,'publish',3),{status:403});
 assert.throws(()=>labAction(at('submitted'),{id:'u1',role:'viewer'},'withdraw',3),{status:403});
});

test('a stale version is a conflict, never a lost update',async()=>{
 const {labAction}=await import('../auth/src/contracts.mjs');
 for(const [action,user,status] of [['update',LAB_USERS.owner,'draft'],['submit',LAB_USERS.owner,'draft'],['withdraw',LAB_USERS.owner,'submitted'],['publish',LAB_USERS.admin,'submitted'],['reject',LAB_USERS.admin,'submitted']])
  assert.throws(()=>labAction({...LAB_ROW,status},user,action,2),{status:409,message:/version 3/});
});

test('global and crypto strategies cannot be submitted or published to YT Strategies',async()=>{
 const {labAction,LAB_PUBLISHABLE_MARKETS}=await import('../auth/src/contracts.mjs');
 assert.deepEqual([...LAB_PUBLISHABLE_MARKETS],['in','us']);
 for(const market of ['gl','crypto']){
  assert.throws(()=>labAction({...LAB_ROW,market},LAB_USERS.owner,'submit',3),{status:400,message:/provisional/});
  assert.throws(()=>labAction({...LAB_ROW,market,status:'submitted'},LAB_USERS.admin,'publish',3),{status:400,message:/provisional/});
  assert.equal(labAction({...LAB_ROW,market},LAB_USERS.owner,'update',3),'draft');
 }
 assert.equal(labAction({...LAB_ROW,market:'us'},LAB_USERS.owner,'submit',3),'submitted');
});

test('duplicate always yields a draft, only for owners or published rows',async()=>{
 const {labAction}=await import('../auth/src/contracts.mjs');
 assert.equal(labAction({...LAB_ROW,status:'submitted'},LAB_USERS.owner,'duplicate'),'draft');
 assert.throws(()=>labAction(LAB_ROW,LAB_USERS.admin,'duplicate'),{status:403,message:/Only the owner can duplicate an unpublished strategy/});
});

test('mutating lab actions require a version and unknown actions are refused',async()=>{
 const {labAction,validateLabNote}=await import('../auth/src/contracts.mjs');
 for(const [action,user,status] of [['update',LAB_USERS.owner,'draft'],['delete',LAB_USERS.owner,'draft'],['submit',LAB_USERS.owner,'draft'],['withdraw',LAB_USERS.owner,'submitted'],['publish',LAB_USERS.admin,'submitted'],['reject',LAB_USERS.admin,'submitted']])
  for(const v of [undefined,null]) assert.throws(()=>labAction({...LAB_ROW,status},user,action,v),{status:400,message:/version required/});
 assert.throws(()=>labAction(LAB_ROW,LAB_USERS.owner,'toString',3),{status:400,message:/unknown action/});
 assert.equal(validateLabNote('',false),null);
});

test('lab migration is additive, re-runnable and keeps provisional markets out of published snapshots',async()=>{
 const {readFile}=await import('node:fs/promises');const {LAB_PUBLISHABLE_MARKETS,LAB_MARKETS}=await import('../auth/src/contracts.mjs');
 const sql=await readFile(new URL('../db/002-lab-strategies.sql',import.meta.url),'utf8');
 assert.doesNotMatch(sql,/^\s*(DROP|ALTER|DELETE|UPDATE|TRUNCATE|INSERT)\b/im);
 const creates=sql.match(/^\s*CREATE\s+(UNIQUE\s+)?(TABLE|INDEX)\b/gim).length;
 assert.equal(sql.match(/^\s*CREATE\s+(UNIQUE\s+)?(TABLE|INDEX)\s+IF\s+NOT\s+EXISTS/gim).length,creates);
 assert.match(sql,/CHECK \(status <> 'published' OR market IN \('in','us'\)\)/);
 const checks=[...sql.matchAll(/market IN \(([^)]*)\)/g)].map(m=>m[1].replace(/'/g,'').split(','));
 assert.deepEqual(checks,[[...LAB_MARKETS],[...LAB_PUBLISHABLE_MARKETS],[...LAB_PUBLISHABLE_MARKETS]]);
 const bootstrap=await readFile(new URL('../auth/src/bootstrap.mjs',import.meta.url),'utf8');
 assert.ok(bootstrap.indexOf('001-app.sql')>0&&bootstrap.indexOf('002-lab-strategies.sql')>bootstrap.indexOf('001-app.sql'));
});
