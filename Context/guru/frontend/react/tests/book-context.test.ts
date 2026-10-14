import {expect,it} from 'vitest';
import {bookContext} from '../src/services/book-context';
it('preserves recorded source dates and keeps other books publication metadata separate',()=>{
  const windows={stc9:['1995-01-01','2009-12-31',2010] as const};
  expect(bookContext('stc9',windows,{})).toEqual(windows.stc9);
  expect(bookContext('hbqts',windows,{hbqts:{year:2007}})).toEqual(['','',2007]);
  expect(bookContext('unknown',windows,{})).toEqual(['','',null]);
  expect(bookContext('pending',windows,{pending:{year:'Edition unknown'}})).toEqual(['','',null]);
});
