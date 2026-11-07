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
  for (const value of ['#/', '#/strategies?region=us&tk=SPX', '#/s/f1a?from=2020-01-01', '#/guru/connors', '#/o/rsi2_5d?region=us', '#/guru/ross', '#/yt', '#/yt?tab=sharpe']) assert.equal(validateSavedView(value),value);
  for (const value of ['https://evil.invalid','#//evil.invalid','#/s/%zz','#/\n',null,'#/ytx','#/yt/x','#/s/'+ 'x'.repeat(5000)]) assert.throws(()=>validateSavedView(value));
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
