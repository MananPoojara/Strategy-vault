import {expect,it} from 'vitest';
import {sourceAccount,sourceAccountView,sourceAuditHeaders,sourceAuditRows,sourceRiskRows,sourceTradeHeaders} from '../src/services/source-account';
import {returnDisplay} from '../src/services/return-display';
import {accountFixture} from './helpers/source-account-fixture';
const parse=(d:unknown)=>sourceAccount(d,'elcim_impulse','elcim');
it('separates instrument return, weighted trade return and actual account return',()=>{
 const d=accountFixture(),a=parse(d)!;expect(a.trade_accounts[0].raw_instrument_gross_return).toBe(.1);expect(d.trades[0].ret).toBe(.0396);
 expect(sourceAccountView(a).actualReturn).toBeCloseTo(.055038,10);expect(sourceAccountView(a).closedTradeCount).toBe(1);
 const display=returnDisplay(d,'fixed',a);expect(display.value).toBe(.0396);expect(display.label).toBe('Closed return sum');expect(display.description).toContain('same basis');
 expect(d.stats.avg_net_pnl*d.stats.trades).toBe(display.value);expect(display.value).not.toBeCloseTo(d.stats.total_return,5);
});
it('closed compounding is hypothetical display only and excludes the terminal open mark',()=>{
 const d=accountFixture(),a=parse(d)!;const before=JSON.stringify(d);d.trades[0].ret_full_precision=.02;d.trades.splice(1,0,{...d.trades[0],ret_full_precision:-.01});
 const committed=JSON.stringify(d);expect(returnDisplay(d,'fixed',a).value).toBeCloseTo(.01,12);expect(returnDisplay(d,'compound',a).value).toBeCloseTo(.0098,12);
 expect(returnDisplay(d,'compound',a).reportMode).toContain('Hypothetical');expect(returnDisplay(d,'compound',a).description).toContain('never changes');expect(JSON.stringify(d)).toBe(committed);expect(before).not.toBe(committed);
});
it('keeps terminal open exit costs zero and audit columns explicitly distinguish basis',()=>{
 const d=accountFixture(),a=parse(d)!;expect(a.trade_accounts[1].exit_cost_account_contribution).toBe(0);d.trades[0].ret_full_precision=.039600123456789;const rows=sourceAuditRows(a,d.trades.map(t=>t.ret_full_precision));
 expect(sourceAuditHeaders[3]).toBe('Raw instrument gross return');expect(sourceAuditHeaders[8]).toContain('initial unit');expect(rows[1][1]).toContain('Open mark');expect(rows[1][10]).toBe(0);expect(sourceRiskRows(a)).toHaveLength(4);
 expect(sourceAuditHeaders[6]).toContain('unrounded');expect(rows[0][6]).toBe(.039600123456789);expect(()=>sourceAuditRows(a,[])).toThrow();
 const generic=Array.from({length:17},(_,i)=>String(i));expect(sourceTradeHeaders(generic)[13]).toContain('actual entry equity');expect(generic[13]).toBe('13');
});
it('preserves legacy results without source-account semantics',()=>{expect(sourceAccount({trades:[]},'r3','stc9')).toBeNull();expect(returnDisplay({trades:[]}).label).toBe('Net Profit');});
it('fails closed for missing, unknown, mismatched, malformed or monetary account results',()=>{
 const corrupt: ((d:ReturnType<typeof accountFixture>)=>unknown)[]=[
  d=>({...d,source_account:undefined}),d=>({...d,source_account:{...d.source_account,model:'unreviewed'}}),
  d=>({...d,source_account:{...d.source_account,book_performance_reproduced:true}}),
  d=>({...d,source_account:{...d.source_account,days:d.source_account.days.slice(1)}}),
  d=>({...d,source_account:{...d.source_account,days:d.source_account.days.map((row,i)=>i===1?{...row,equity:.9}:row)}}),
  d=>({...d,source_account:{...d.source_account,days:d.source_account.days.map((row,i)=>i===0?{...row,costs_paid:.5}:row)}}),
  d=>({...d,source_account:{...d.source_account,trade_accounts:[...d.source_account.trade_accounts].reverse()}}),
  d=>({...d,source_account:{...d.source_account,trade_accounts:d.source_account.trade_accounts.map((row,i)=>i===1?{...row,exit_cost_account_contribution:.001}:row)}}),
  d=>({...d,trades:d.trades.map((t,i)=>i===0?{...t,accounting:{...t.accounting,net_pnl:1000}}:t)}),
  d=>({...d,trades:d.trades.map((t,i)=>i===0?{...t,ret_full_precision:NaN}:t)}),d=>({...d,kind:'basket'}),
 ];
 for(const mutation of corrupt)expect(()=>parse(mutation(accountFixture()))).toThrow('response is missing or inconsistent');
 expect(()=>sourceAccount(accountFixture(),'elcim_fi2','elcim')).toThrow();expect(()=>sourceAccount(accountFixture(),'elcim_impulse','other')).toThrow();
});
