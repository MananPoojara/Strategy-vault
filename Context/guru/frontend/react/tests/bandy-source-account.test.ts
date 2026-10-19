import {expect,it} from 'vitest';
import {renderToStaticMarkup} from 'react-dom/server';
import {createElement} from 'react';
import {sourceAccount,sourceAccountView,sourceAccountCopy,sourceAccountTables,ELDER_ACCOUNT_MODEL} from '../src/services/source-account';
import {BANDY_ACCOUNT_MODEL} from '../src/services/bandy-source-account';
import {SourceAccountNotice} from '../src/components/SourceAccountNotice';
import {returnDisplay} from '../src/services/return-display';
import {bandyAccountFixture} from './helpers/bandy-source-account-fixture';
import {accountFixture} from './helpers/source-account-fixture';
const parse=(data:unknown,macd=false)=>sourceAccount(data,macd?'hbiab_macd_example':'hbiab_ma5_20','hbiab')!;
for (const macd of [false,true]) for (const bps of [0,5]) it(`validates conserved ${macd?'source25%':'declared100%'} cash account at ${bps}bps without changing it`,()=>{
  const d=bandyAccountFixture(macd,bps),before=JSON.stringify(d),account=parse(d,macd);
  expect(account.model).toBe(BANDY_ACCOUNT_MODEL);expect(sourceAccountView(account).actualReturn).toBe(d.equity.at(-1)!-1);
  const copy=sourceAccountCopy(account);expect(copy.bandy).toBe(true);expect(copy.sizing).toContain(macd?'25%':'100%');
  expect(copy.sizing).toContain(macd?'Source allocation':'Declared research convention');
  expect(returnDisplay(d,'fixed',account).value).toBe(d.stats.avg_net_pnl);expect(returnDisplay(d,'fixed',account).description).not.toContain('2%/6%');
  expect(returnDisplay(d,'compound',account).value).toBeCloseTo(d.stats.net_profit,12);
  expect(JSON.stringify(d)).toBe(before);
});
it('charges the actual exit notional and keeps open liquidation costs separately uncharged',()=>{
  const d=bandyAccountFixture(true),account=parse(d,true);const [closed,open]=account.trade_accounts;
  expect(closed.exit_cost_account_contribution).not.toBe(closed.entry_cost_account_contribution);
  expect(open.exit_cost_account_contribution).toBe(0);expect(d.equity.at(-1)).not.toBe(1+d.stats.net_profit);
  const tables=sourceAccountTables(account,d.trades.map(t=>t.ret_full_precision));
  expect(tables.audit.headers).toContain('Filled exit notional (initial unit)');expect(tables.audit.headers).toContain('Uncharged open liquidation estimate (initial unit)');
  expect(tables.audit.rows[1][12]).toBeNull();expect(tables.audit.rows[1][19]).toBeGreaterThan(0);expect(tables.daily.name).toBe('Account daily');
  expect(tables.daily.headers.join(' ')).not.toMatch(/monthly|stop|risk/i);
});
it('does not publish invented monthly risk, stops, capital or unescaped basis in the Bandy notice',()=>{
  const d=bandyAccountFixture();d.source_account.basis='<img src=x onerror=alert(1)>';
  const markup=renderToStaticMarkup(createElement(SourceAccountNotice,{account:sourceAccountView(parse(d))}));
  expect(markup).toContain('&lt;img');expect(markup).not.toContain('<img');expect(markup).not.toMatch(/2%|6%|monthly|SafeZone|₹/);
  expect(markup).toContain('observed sessions');expect(markup).toContain('not calendar days');
  expect(markup).toContain('Zero-return closed trades join non-wins');expect(markup).toContain('actual net entry equity');
});
it('preserves the Elder account model, notice wording and audit columns',()=>{
  const d=accountFixture(),account=sourceAccount(d,'elcim_impulse','elcim')!;
  expect(account.model).toBe(ELDER_ACCOUNT_MODEL);expect(sourceAccountCopy(account).accountEquity).toBe('Risk-sized account equity');
  const tables=sourceAccountTables(account,d.trades.map(t=>t.ret_full_precision));
  expect(tables.daily.name).toBe('Account risk');expect(tables.audit.headers).toContain('Risk budget / monthly reference');
  const markup=renderToStaticMarkup(createElement(SourceAccountNotice,{account:sourceAccountView(account)}));
  expect(markup).toContain('Protected Impulse');expect(markup).toContain('6% monthly');
});
it('fails closed for missing/unknown/wrong book or method models rather than unit-exposure fallback',()=>{
  for (const id of ['hbiab_ma5_20','hbiab_ma7_4','hbiab_macd_example']) expect(()=>sourceAccount({},id,'hbiab')).toThrow('Bandy cash account');
  expect(()=>sourceAccount(bandyAccountFixture(),'hbiab_ma5_20','other')).toThrow();
  expect(()=>sourceAccount(bandyAccountFixture(),'hbqts_ma5_25','hbqts')).toThrow();
  expect(()=>sourceAccount({...bandyAccountFixture(),kind:'basket'},'hbiab_ma5_20','hbiab')).toThrow();
  expect(()=>sourceAccount(accountFixture(),'hbiab_ma5_20','hbiab')).toThrow();
});
it('rejects source-sizing changes, reused Elder fields, invented exit charges and inconsistent ledgers',()=>{
  type Fixture=ReturnType<typeof bandyAccountFixture>;
  const mutations:((d:Fixture)=>unknown)[]=[
    d=>({...d,source_account:{...d.source_account,model:'unknown'}}),
    d=>({...d,source_account:{...d.source_account,sizing:{...d.source_account.sizing,target_equity_fraction:.5}}}),
    d=>({...d,source_account:{...d.source_account,sizing:{...d.source_account.sizing,origin:'source_rule'}}}),
    d=>({...d,source_account:{...d.source_account,days:d.source_account.days.map((r,i)=>i===1?{...r,free_cash:.5}:r)}}),
    d=>({...d,source_account:{...d.source_account,days:d.source_account.days.map((r,i)=>i===1?{...r,monthly_reference:1}:r)}}),
    d=>({...d,source_account:{...d.source_account,days:d.source_account.days.map((r,i)=>i===2?{...r,costs_paid:0}:r)}}),
    d=>({...d,source_account:{...d.source_account,days:d.source_account.days.map((r,i)=>i===2?{...r,gross_same_quantity_equity:2}:r)}}),
    d=>({...d,source_account:{...d.source_account,trade_accounts:[...d.source_account.trade_accounts].reverse()}}),
    d=>({...d,source_account:{...d.source_account,trade_accounts:d.source_account.trade_accounts.map((r,i)=>i===1?{...r,exit_cost_account_contribution:.001}:r)}}),
    d=>({...d,source_account:{...d.source_account,trade_accounts:d.source_account.trade_accounts.map((r,i)=>i===1?{...r,exit_notional_units:r.entry_notional_units}:r)}}),
    d=>({...d,source_account:{...d.source_account,trade_accounts:d.source_account.trade_accounts.map((r,i)=>i===0?{...r,quantity:NaN}:r)}}),
    d=>({...d,source_account:{...d.source_account,trade_accounts:d.source_account.trade_accounts.map((r,i)=>i===0?{...r,entry_equity_units:2}:r)}}),
    d=>({...d,trades:d.trades.map((t,i)=>i===0?{...t,ret_full_precision:0}:t)}),
    d=>({...d,trades:d.trades.map((t,i)=>i===0?{...t,accounting:{...t.accounting,gross_pnl:1000}}:t)}),
    d=>({...d,stats:{...d.stats,trades:2}}),d=>({...d,stats:{...d.stats,closed_return_sum:0}}),
    d=>({...d,source_account:{...d.source_account,pending_order:{side:'sell',signal_date:d.dates.at(-1),fill_phase:'open',filled:false},terminal_exit_pending:true}}),
  ];
  for (const mutation of mutations) expect(()=>parse(mutation(bandyAccountFixture()))).toThrow('Bandy cash account');
});
it('accepts a real terminal next-open sell queue only with an open holding and zero exit cost',()=>{
  const d=bandyAccountFixture(true);
  const pending={...d,source_account:{...d.source_account,terminal_exit_pending:true,pending_order:{side:'sell',signal_date:d.dates.at(-1),fill_phase:'open',filled:false}}};
  expect(parse(pending,true).model).toBe(BANDY_ACCOUNT_MODEL);
  expect(()=>parse({...pending,source_account:{...pending.source_account,terminal_entry_pending:true}},true)).toThrow();
});
it('retains a final unfilled buy as a queue without creating a position or charging entry costs',()=>{
  const d=bandyAccountFixture(true),closed=d.source_account.trade_accounts[0],cash=1+closed.net_account_contribution;
  const days=d.source_account.days.map((row,i)=>i===3?{...row,equity:cash,gross_same_quantity_equity:1+closed.gross_account_contribution,
    costs_paid:closed.entry_cost_account_contribution+closed.exit_cost_account_contribution,free_cash:cash,quantity:0,position_value:0,direction:0}:row);
  const flat={...d,equity:days.map(r=>r.equity),gross_equity:days.map(r=>r.gross_same_quantity_equity),trades:d.trades.slice(0,1),
    stats:{...d.stats,total_return:cash-1},source_account:{...d.source_account,days,trade_accounts:[closed],terminal_entry_pending:true,
      pending_order:{side:'buy',signal_date:d.dates.at(-1),fill_phase:'open',filled:false}}};
  expect(parse(flat,true).trade_accounts).toHaveLength(1);expect(sourceAccountView(parse(flat,true)).actualReturn).toBe(cash-1);
});

it('rejects stale or missing MACD signals and conflicting chart/tradesheet fields',()=>{
 type Fixture=ReturnType<typeof bandyAccountFixture>;
 const corrupt:((d:Fixture)=>unknown)[]=[
  d=>({...d,source_account:{...d.source_account,trade_accounts:d.source_account.trade_accounts.map((t,i)=>i===1?{...t,entry_signal_date:d.dates[0]}:t)}}),
  d=>({...d,source_account:{...d.source_account,trade_accounts:d.source_account.trade_accounts.map((t,i)=>i===0?{...t,entry_signal_date:'2020-01-03'}:t)}}),
  d=>({...d,source_account:{...d.source_account,trade_accounts:d.source_account.trade_accounts.map((t,i)=>i===0?{...t,exit_signal_date:d.dates[0]}:t)}}),
  d=>({...d,trades:d.trades.map((t,i)=>i===0?{...t,legs:[[0,100,t.legs[0][2]]]}:t)}),
  d=>({...d,trades:d.trades.map((t,i)=>i===0?{...t,legs:[[1,100,.5]]}:t)}),
  d=>({...d,trades:d.trades.map((t,i)=>i===0?{...t,exit_i:3}:t)}),
  d=>({...d,trades:d.trades.map((t,i)=>i===0?{...t,signal:d.dates[0]}:t)}),
  d=>({...d,trades:d.trades.map((t,i)=>i===0?{...t,accounting:{...t.accounting,signal_side:'Short'}}:t)}),
  d=>({...d,trades:d.trades.map((t,i)=>i===0?{...t,accounting:{...t.accounting,pts:0}}:t)}),
  d=>({...d,trades:d.trades.map((t,i)=>i===0?{...t,accounting:{...t.accounting,days_held:2}}:t)}),
  d=>({...d,trades:d.trades.map((t,i)=>i===0?{...t,accounting:{...t.accounting,symbol:'OTHER'}}:t)}),
  d=>({...d,costs:{...d.costs,total_side_bps:99}}),
  d=>({...d,trades:d.trades.map((t,i)=>i===1?{...t,accounting:{...t.accounting,cost_status:'open_liquidation_estimate'}}:t)}),
 ];
 for(const mutate of corrupt)expect(()=>parse(mutate(bandyAccountFixture(true)),true)).toThrow('Bandy cash account');
});

it('accepts fractional fee/slippage bps using arithmetic conservation instead of exact binary addition spelling',()=>{
 const d=bandyAccountFixture(false,.3);d.costs.fees_bps=.1;d.costs.slippage_bps=.2;d.costs.total_side_bps=.3;
 d.trades=d.trades.map((trade,i)=>{
  const row=d.source_account.trade_accounts[i],notional=row.entry_notional_units+(row.exit_notional_units||0);
  return {...trade,accounting:{...trade.accounting,fee_ret:notional*.1/10000/row.entry_equity_units,slippage_ret:notional*.2/10000/row.entry_equity_units}};
 });
 expect(.1+.2).not.toBe(.3);expect(parse(d).model).toBe(BANDY_ACCOUNT_MODEL);
});

it('retains tiny positive modeled costs and rejects zeroed paid charges or cost metadata',()=>{
 const d=bandyAccountFixture(true,.0000003);expect(parse(d,true).model).toBe(BANDY_ACCOUNT_MODEL);
 const t=d.source_account.trade_accounts[0];expect(t.entry_cost_account_contribution).toBeGreaterThan(0);expect(t.entry_cost_account_contribution).toBeLessThan(1e-10);
 const omitted={...d,source_account:{...d.source_account,trade_accounts:d.source_account.trade_accounts.map((row,i)=>i===0?{...row,entry_cost_account_contribution:0}:row)}};
 expect(()=>parse(omitted,true)).toThrow();
 expect(()=>parse({...d,costs:{...d.costs,total_side_bps:0}},true)).toThrow();
 expect(()=>parse({...d,trades:d.trades.map((row,i)=>i===0?{...row,accounting:{...row.accounting,slippage_ret:0}}:row)},true)).toThrow();
 expect(()=>parse({...d,source_account:{...d.source_account,days:d.source_account.days.map((row,i)=>i===1?{...row,costs_paid:0}:row)}},true)).toThrow();
});

it('binds the selected traded instrument and provenance independently of its benchmark',()=>{
 const d=bandyAccountFixture();d.benchmark_symbol='SPX';
 expect(sourceAccount(d,'hbiab_ma5_20','hbiab','AAPL')!.model).toBe(BANDY_ACCOUNT_MODEL);
 expect(()=>sourceAccount(d,'hbiab_ma5_20','hbiab','SPY')).toThrow();
 const native={...d,provenance:{qualified_ticker:'basket:us_sp500:AAPL',symbol:'AAPL'}};
 expect(sourceAccount(native,'hbiab_ma5_20','hbiab','basket:us_sp500:AAPL')!.model).toBe(BANDY_ACCOUNT_MODEL);
 expect(()=>sourceAccount(native,'hbiab_ma5_20','hbiab','basket:us_nasdaq100:AAPL')).toThrow();
 expect(()=>sourceAccount(d,'hbiab_ma5_20','hbiab','basket:us_sp500:AAPL')).toThrow();
 expect(()=>sourceAccount({...native,provenance:{...native.provenance,symbol:'OTHER'}},'hbiab_ma5_20','hbiab','basket:us_sp500:AAPL')).toThrow();
});

 it('requires matching outer method/book identity without changing legacy account requirements',()=>{
 const d=bandyAccountFixture();
 expect(sourceAccount({...d,id:'hbiab_ma5_20',book:'hbiab'},'hbiab_ma5_20','hbiab')!.model).toBe(BANDY_ACCOUNT_MODEL);
 expect(sourceAccount({...d,id:'hbiab_ma7_4',book:'hbiab'},'hbiab_ma7_4','hbiab')!.model).toBe(BANDY_ACCOUNT_MODEL);
 const {id:omittedId,book:omittedBook,...withoutIdentity}=d;
 expect(()=>sourceAccount(withoutIdentity,'hbiab_ma5_20','hbiab')).toThrow('Bandy cash account');
 for(const fields of [{id:'hbiab_ma7_4'},{book:'other'},{id:null},{book:null},{id:undefined},{book:undefined}])
   expect(()=>sourceAccount({...d,...fields},'hbiab_ma5_20','hbiab')).toThrow('Bandy cash account');
 expect(sourceAccount(accountFixture(),'elcim_impulse','elcim')!.model).toBe(ELDER_ACCOUNT_MODEL);
 });

it('accepts a single observed session with an unfilled terminal buy and no manufactured fills or charges',()=>{
 const d=bandyAccountFixture(true),day=d.source_account.days[0];
 const flat={...d,dates:[day.date],close:[100],equity:[1],gross_equity:[1],trades:[],stats:{total_return:0,net_profit:0,closed_return_sum:0,avg_net_pnl:null,trades:0},gross_stats:{total_return:0},
 source_account:{...d.source_account,days:[day],trade_accounts:[],terminal_entry_pending:true,pending_order:{side:'buy',signal_date:day.date,fill_phase:'open',filled:false}}};
 const account=parse(flat,true);expect(account.days).toHaveLength(1);expect(account.trade_accounts).toHaveLength(0);expect(account.days[0].costs_paid).toBe(0);expect(sourceAccountView(account).actualReturn).toBe(0);
 expect(()=>parse({...flat,dates:[],close:[],equity:[],gross_equity:[],source_account:{...flat.source_account,days:[]}},true)).toThrow('Bandy cash account');
});
