import {expect,it} from 'vitest';
import {unavailableBookMethods} from '../src/services/unavailable-book-methods';
const defs=[{id:'a',book:'one',family:'rsi',family_name:'RSI entry'},{id:'b',book:'one',family:'rsi'},{id:'c',book:'one',family:'stock',family_name:'Stock setup'},{id:'d',book:'one',family:'stock'},{id:'e',book:'one',family:'zero'},{id:'f',book:'one',family:'pending'}];
it('shows unavailable strategies once each, preserving their name and actual data or rule reason',()=>{
 const items=unavailableBookMethods(defs,[{id:'a',status:'available'},{id:'b',status:'source_review',note:'A sibling is pending'},{id:'c',status:'needs_stock_basket',note:'Actual individual stocks are required.'},{id:'d',status:'needs_stock_basket',note:'Actual individual stocks are required.'},{id:'e',status:'no_trades',note:'The measured run has no closed trades.'}]);
 expect(items.map(x=>x.key)).toEqual(['one:stock','one:zero','one:pending']);
 expect(items[0]).toMatchObject({name:'Stock setup',status:'needs_stock_basket',reasons:['Actual individual stocks are required.']});
 expect(items[1].status).toBe('no_trades');expect(items[2].reasons[0]).toContain('No measured result');
});
it('does not invent performance or merge unrelated books with the same method label',()=>{
 const items=unavailableBookMethods([{id:'a',book:'one',family:'same'},{id:'b',book:'two',family:'same'}],[{id:'a',status:'source_review',note:'Exit rule is unspecified.'},{id:'b',status:'needs_data',note:'Options quotes are missing.'}]);
 expect(items).toHaveLength(2);expect(items[0].reasons).toEqual(['Exit rule is unspecified.']);expect(items[1].reasons).toEqual(['Options quotes are missing.']);
 expect(items.every(x=>!('stats'in x)&&!('spark'in x))).toBe(true);
});
