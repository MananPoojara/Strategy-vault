import {expect,it} from 'vitest';
import {visibleBacktestCount,linkedBookBacktestCount} from '../src/services/library-count-context';
it('counts available visible strategies once across parameter variants and excludes withheld or open-only rows',()=>{
 const row=(id:string,book:string,family:string,trades:number)=>({v:{id,book,family},s:{trades,net_profit:0}});
 expect(visibleBacktestCount([row('a','book','rsi',3),row('b','book','rsi',4),row('c','other','rsi',2),row('d','book','open',0),{v:{id:'e',book:'book',family:'unavailable'},s:null}])).toBe(2);
 expect(visibleBacktestCount([{v:{id:'bad',book:'book',family:'bad'},s:{trades:3,net_profit:NaN,total_return:4}}])).toBe(0);
});
it('book-card availability matches its actual linked instrument, dates, costs and execution mode',()=>{
 const base={book_id:'book',method_id:'book:rsi',market:'india',instrument:'NIFTY 50',evidence:{from:'2010-01-01',to:'2020-09-11'},execution:{cost:'0',slippage:'0',rollover:'0',research_mode:''}};
 const rows=[base,{...base}, {...base,method_id:'book:stocks',instrument:'basket:nifty50:RELIANCE'}, {...base,method_id:'book:other-window',evidence:{from:'',to:''}}, {...base,method_id:'book:cost',execution:{...base.execution,slippage:'5'}}, {...base,book_id:'other',method_id:'other:rsi'}, {...base,method_id:'book:research',execution:{...base.execution,research_mode:'special'}}];
 const link='#/guru/author?book=book&from=2010-01-01&to=2020-09-11';
 expect(linkedBookBacktestCount(rows,'book',link)).toBe(1);
 expect(linkedBookBacktestCount(rows,'book',link+'&slippage=5')).toBe(1);
 expect(linkedBookBacktestCount(rows,'book',link+'&tk=basket%3Anifty50%3ARELIANCE')).toBe(1);
 expect(linkedBookBacktestCount(rows,'book','#/guru/author?book=book')).toBe(1);
 expect(linkedBookBacktestCount(rows,'book',link+'&region=us')).toBe(0);
});
