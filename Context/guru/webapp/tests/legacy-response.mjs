import assert from 'node:assert/strict';
export function projectLegacy(actual,expected){
 if(Array.isArray(expected)){
  assert.ok(Array.isArray(actual));
  if(expected.length&&expected.every(x=>x&&typeof x==='object'&&!Array.isArray(x)&&typeof x.ticker==='string'))return expected.map(x=>{const matches=actual.filter(a=>a?.ticker===x.ticker);assert.equal(matches.length,1);return projectLegacy(matches[0],x);});
  assert.equal(actual.length,expected.length);return expected.map((x,i)=>projectLegacy(actual[i],x));
 }
 if(expected&&typeof expected==='object'){assert.ok(actual&&typeof actual==='object');return Object.fromEntries(Object.keys(expected).map(k=>{assert.ok(Object.hasOwn(actual,k),'Missing legacy field '+k);return [k,projectLegacy(actual[k],expected[k])];}));}
 assert.deepEqual(actual,expected);return actual;
}
