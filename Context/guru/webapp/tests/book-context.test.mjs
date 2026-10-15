import assert from 'node:assert/strict';
import {test} from 'node:test';
import {bookContext} from '../../frontend/react/src/services/book-context.ts';
const known={stc9:['1995-01-01','2009-12-31',2010]};
test('Bandy uses its own 2007 publication year without borrowing another course study dates',()=>{
 assert.deepEqual(bookContext('hbqts',known,{hbqts:{year:2007}}),['','',2007]);
});
test('retains explicitly recorded source windows and leaves unknown source metadata unknown',()=>{
 assert.deepEqual(bookContext('stc9',known,{}),known.stc9);
 for(const year of [undefined,null,0,NaN,'Edition unknown'])assert.deepEqual(bookContext('reading',known,{reading:{year}}),['','',null]);
 assert.deepEqual(bookContext('jr_rule',known,{jr_rule:{year:1994}}),['','',1994]);
});
