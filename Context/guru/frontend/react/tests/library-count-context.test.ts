import {expect,it} from 'vitest';
import {visibleBacktestCount,linkedBookBacktestCount,measuredWindow} from '../src/services/library-count-context';
import {dateCoverageIssue} from '../src/services/summary-loader';
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
it('full-history snapshot rows link without display labels and open without an invalid-date notice',()=>{
 const row={book_id:'ss',method_id:'ss:turtle',market:'india',instrument:'NIFTY 50',run_settings:{from:'',to:''},evidence:{from:'full available history',to:'latest available bar'},execution:{cost:'0',slippage:'0',rollover:'0',research_mode:''}};
 expect(measuredWindow(row)).toEqual({from:'',to:''});
 expect(measuredWindow({run_settings:{from:'2000-01-03',to:''},evidence:{from:'2000-01-03',to:'latest available bar'}})).toEqual({from:'2000-01-03',to:''});
 expect(measuredWindow({evidence:{from:'full available history',to:'2020-09-11'}})).toEqual({from:'',to:'2020-09-11'});
 const q=new URLSearchParams(Object.entries({book:'ss',library_market:'in',...measuredWindow(row)}).filter(([,v])=>v));
 const link='#/guru/connors?'+q;
 expect(link).not.toMatch(/from=|to=/);
 expect(dateCoverageIssue({ticker:'NIFTY 50',label:'Nifty 50',from:q.get('from')||'',to:q.get('to')||''})).toBeUndefined();
 expect(dateCoverageIssue({ticker:'NIFTY 50',label:'Nifty 50',from:'full available history'})?.kind).toBe('invalid_range');
 expect(linkedBookBacktestCount([row],'ss',link)).toBe(1);
});
