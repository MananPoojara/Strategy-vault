import {createRequire} from 'node:module';
import test from 'node:test';
import assert from 'node:assert/strict';
const require=createRequire(import.meta.url),r=require('../../frontend/report-data.js');
test('monthly totals and annual average use measured returns including actual zero months',()=>{
 const d={dates:['2024-01-01','2024-01-31','2024-02-29','2025-01-31','2025-02-28'],equity:[1,1.1,1.1,1.21,1.452]};
 const m=r.monthly(d);assert.ok(Math.abs(m.months['2024-01']-.1)<1e-12);assert.equal(m.months['2024-02'],0);assert.ok(Math.abs(m.annualAverage-(.1+.32)/2)<1e-12);assert.equal(m.averages[11],null);
});
test('benchmark uses actual dated prices and rejects missing or invalid quotes',()=>{
 const px={d:['2023-01-01','2024-01-01','2025-01-01'],c:[100,110,99]};const b=r.benchmark(px,'2023-01-01','2025-01-01');assert.equal(b.total_return,-.01);assert.ok(Math.abs(b.max_dd+.1)<1e-12);assert.ok(b.cagr<0);assert.equal(b.trades,null);assert.equal(r.benchmark(px,'2026-01-01','2027-01-01'),null);assert.equal(r.benchmark({d:['2024-01-01','2024-01-02'],c:[1,0]},'2024-01-01','2024-01-02'),null);
});
test('tradesheet preserves the exact requested order and missing money/taxes without invented rupees',()=>{
 const d={trades:[{entry:'2024-01-02',exit:'2024-01-09',avg_px:100,exit_px:110,ret:.099,reason:'close, rule',accounting:{symbol:'REAL_SYMBOL',signal_side:'Long',pts:10,gross_ret:.1,net_ret:.099,days_held:7,slippage_ret:.001,mae:-.02,mfe:.12}}]};
 assert.deepEqual(r.tradeHeaders,['Symbol','Signal','Entry Date','Entry Price','Exit Date','Exit Price','Pts','Gross P&L','Gross P&L %','Days Held','Taxes','Slippage','Net P&L','Net P&L %','MAE','MFE','Exit Reason']);
 const row=r.tradeRows(d)[0];assert.equal(row[0],'REAL_SYMBOL');assert.equal(row[1],'LONG');assert.equal(row[2],'02-01-2024');assert.equal(row[7],null);assert.equal(row[10],null);assert.equal(row[11],.001);assert.equal(row[12],null);assert.equal(row[13],.099);assert.equal(row[14],-.02);
});
test('closed and open positions retain their distinction and missing excursion status',()=>{const row=r.tradeRows({trades:[{entry:'2024-01-02',exit:'2024-02-01',open:true,ret:.01,reason:'open'}]},'INDEX')[0];assert.equal(row[0],'INDEX');assert.equal(row[4],'Open');assert.equal(row[14],null);assert.equal(row[16],'open');});
test('CSV quotes commas, quotes, newlines and neutralizes spreadsheet formulas in text cells',()=>{const c=r.csv([['Symbol','Exit Reason'],['=CMD','a,"b"\nc']]);assert.equal(c,'Symbol,Exit Reason\r\n\'=CMD,"a,""b""\nc"');});
test('detailed stats exclude seven primary metrics and all capital fields',()=>{const rows=r.detailRows({stats:{cagr:.1,avg_win:.05,payoff:2},gross_stats:{cagr:.12,avg_win:.06,payoff:3}});assert.ok(rows.some(x=>x[0]==='Annualised return'&&x[1]===.1&&x[2]===.12));assert.ok(!rows.some(x=>/capital|Net profit|Win rate|Trades$|Max drawdown|Sharpe|Expectancy|Average net/.test(x[0])));});
test('weekly/monthly candles aggregate actual dated OHLC without adding synthetic sessions',()=>{const px={d:['2026-09-30','2026-10-01','2026-10-02','2026-10-05'],o:[10,12,11,15],h:[13,14,15,17],l:[9,10,10,14],c:[12,11,14,16],v:[3,4,5,6]};const w=r.aggregate(px,'W');assert.deepEqual(w.d,['2026-10-02','2026-10-05']);assert.deepEqual(w.o,[10,15]);assert.deepEqual(w.h,[15,17]);assert.deepEqual(w.l,[9,14]);assert.deepEqual(w.c,[14,16]);assert.deepEqual(w.v,[12,6]);assert.deepEqual(r.aggregate(px,'M').d,['2026-09-30','2026-10-05']);assert.equal(r.aggregate({d:['2026-10-01'],c:[1]},'W'),null);});
test('observed adverse summaries use only recorded closed trades and do not populate net fields',()=>{
 const d={trades:[{accounting:{mae_ret_observed:-.03}},{accounting:{mae_ret_observed:-.01}},{open:true,accounting:{mae_ret_observed:-.9}},{accounting:{mae_ret_observed:null}}]};
 const rows=r.detailRows(d),avg=rows.find(x=>x[0]==='Average observed adverse move'),worst=rows.find(x=>x[0]==='Worst observed adverse move'),total=rows.find(x=>x[0]==='Total observed adverse move (sum)');
 assert.equal(avg[1],null);assert.equal(avg[2],-.02);assert.equal(worst[2],-.03);assert.equal(total[2],-.04);
 const short=r.tradeRows({trades:[{dir:-1,entry:'unknown',accounting:{mae_ret_observed:-.04,mfe_ret_observed:.05}}]},'STOCK')[0];assert.equal(short[1],'SHORT');assert.equal(short[2],'unknown');assert.equal(short[14],-.04);
});
test('constant benchmark has measured zero return and undefined risk ratios; malformed series stay unavailable',()=>{
 const b=r.benchmark({d:['2024-01-01','2024-01-02'],c:[100,100]},'2024-01-01','2024-01-02');assert.equal(b.total_return,0);assert.equal(b.sharpe,null);assert.equal(b.sortino,null);assert.equal(b.mar,null);
 assert.equal(r.benchmark(null,'',''),null);assert.equal(r.benchmark({d:['2024-01-01'],c:[]},'',''),null);assert.equal(r.benchmark({d:['2024-01-01','2024-01-02'],c:[100,NaN]},'2024-01-01','2024-01-02'),null);
 assert.deepEqual(r.periods({dates:['2024-01-01'],equity:[]},4),{});assert.equal(r.monthly({dates:[],equity:[]}).annualAverage,null);assert.deepEqual(r.tradeRows(null),[]);
});
test('missing aggregate volume remains unavailable while actual OHLC is retained',()=>{
 const px={d:['2024-01-01','2024-01-02'],o:[10,12],h:[13,14],l:[9,11],c:[12,13],v:[null,10]};assert.deepEqual(r.aggregate(px,'M').v,[null]);assert.deepEqual(r.aggregate(px,'W').c,[13]);assert.equal(r.aggregate(px,'D'),null);
 assert.equal(r.csv([[null,-2,'@cmd','ordinary']]),",-2,'@cmd,ordinary");
});

test('calendar holding average retains same-day zero and excludes open positions',()=>{
 const rows=r.detailRows({stats:{avg_hold_days:99},trades:[{accounting:{days_held:0}},{accounting:{days_held:4}},{open:true,accounting:{days_held:100}}]});const hold=rows.find(x=>x[0]==='Average hold (calendar days)');assert.deepEqual(hold.slice(1,3),[2,2]);
});

test('time in market counts actual held dates once across overlapping and open positions',()=>{
 const d={dates:['2024-01-02','2024-01-03','2024-01-04','2024-01-05','2024-01-08'],trades:[{entry:'2024-01-02',exit:'2024-01-04'},{entry:'2024-01-03',exit:'2024-01-05'},{entry:'2024-01-08',exit:'2024-01-08',open:true}]};const row=r.detailRows(d).find(x=>x[0].startsWith('Time in market'));assert.deepEqual(row.slice(1),[5,5,'count']);
 assert.equal(r.detailRows({...d,trades:[]}).find(x=>x[0].startsWith('Time in market'))[1],0);assert.equal(r.detailRows({...d,trades:[{entry:'unknown',exit:'2024-01-08'}]}).find(x=>x[0].startsWith('Time in market'))[1],null);
});
