import {act} from 'react';
import {createRoot,type Root} from 'react-dom/client';
import {afterEach,beforeEach,expect,it,vi} from 'vitest';
import {backtestWorkbook,YtResults,tradeTable} from '../src/features/yt-results/YtResults';
import {YtOptimize,type YtOptimizeContext} from '../src/features/yt-results/YtOptimize';
import {optimizationList,optimizationWorkbook,withoutAtrOutOfOrder,type YtOptimizeResult} from '../src/features/yt-results/optimize-sheet';
import {ytRunResult,type EngineRun,type LabReport} from '../src/features/yt-results/types';
import {tsCosts} from '../src/features/yt-results/ts-workbook';
import {xlsx} from '../src/features/yt-results/xlsx';

let root:Root|undefined,host:HTMLDivElement|undefined;
beforeEach(()=>{
  Object.assign(globalThis,{IS_REACT_ACT_ENVIRONMENT:true});
  const series={setData:vi.fn(),setMarkers:vi.fn(),applyOptions:vi.fn()};
  (window as unknown as {LightweightCharts:unknown}).LightweightCharts={createChart:()=>({addAreaSeries:()=>series,addLineSeries:()=>series,
    timeScale:()=>({fitContent:vi.fn(),setVisibleRange:vi.fn()}),subscribeCrosshairMove:vi.fn(),unsubscribeCrosshairMove:vi.fn(),applyOptions:vi.fn(),resize:vi.fn(),remove:vi.fn()})};
  (window as unknown as {VaultReport:unknown}).VaultReport={monthly:()=>({years:['2020'],months:{'2020-01':0.02,'2020-02':-0.01},annual:{'2020':0.01},averages:[0.02,-0.01,null,null,null,null,null,null,null,null,null,null],annualAverage:0.01}),csv:()=>''};
});
afterEach(async()=>{if(root)await act(async()=>root!.unmount());host?.remove();root=undefined;host=undefined;});
async function mount(el:React.ReactNode){host=document.createElement('div');document.body.append(host);root=createRoot(host);await act(async()=>root!.render(el));return host;}

const group={count:1,pct:0.5,total:0.1,profit:0.1,avg:0.1,avg_bars:3,largest:0.1,bars_in_largest:3,max_consecutive:1};
const side={trades:2,avg:0.025,avg_bars:3,profit_factor:2,payoff:2,winners:group,losers:{...group,total:-0.05,profit:-0.05,avg:-0.05,largest:-0.05}};
const report:LabReport={all:side,long:side,short:{...side,trades:0},buyhold:side,
  system:{net_profit:0.05,cagr:0.0912,max_sys_dd:-0.4782,car_mdd:0.19,exposure:0.5273,max_sys_dd_abs:-1.985,rrr:0.56},buy_hold:{net_profit:0.1,cagr:0.1},rates:{sharpe:5,upi:5.4},notes:['A note.']};
const engine:EngineRun={dates:['2020-01-01','2020-01-02','2020-01-03','2020-01-06'],equity:[1,1.1,1.05,1.05],bh:[1,1.01,1.02,1.03],report,
  money:{capital:1_000_000,final_equity:1_050_000,costs_total:0},
  trades:[{dir:1,legs:[[0,100,1]],entry:'2020-01-01',exit:'2020-01-02',exit_px:110,ret:0.1,bars:1,open:false,reason:'Dual ATR',accounting:{mae_ret_observed:-0.02,mfe_ret_observed:0.12},units:10000,position_value:1_000_000,pnl:100000,costs_paid:0},
    {dir:1,legs:[[2,110,1]],entry:'2020-01-03',exit:'2020-01-06',exit_px:104.5,ret:-0.05,bars:1,open:false,reason:'Dual ATR',accounting:{mae_ret_observed:-0.06,mfe_ret_observed:0.01},units:10000,position_value:1_100_000,pnl:-50000,costs_paid:0}]};
const rules={strategy:'Monday Reversal · Type 1',position:'Long' as const,filter:'Without filter',setup:'—',entry:['Monday'],exit:['Dual ATR'],settings:[['Entry condition','Fri O>Mon H']] as [string,string][],tab:'BN_Type 1'};

it('shows the lab Results: seven tiles, the report cards and the three tabs',async()=>{
  const el=await mount(<YtResults result={ytRunResult(engine,'NIFTYBANK','NIFTY BANK')} rules={rules} loadBars={async()=>[]}/>);
  expect([...el.querySelectorAll('[role=tab]')].map(t=>t.textContent)).toEqual(['Stats & equity drawdown','Trades','YOY-Monthly']);
  expect([...el.querySelectorAll('.metric > span')].map(t=>t.firstChild?.textContent)).toEqual(['Net Profit %','Annual return %','Max. system % drawdown','CAR/MDD','Winners %','# Trades','Exposure %']);
  expect([...el.querySelectorAll('.metric b')].map(t=>t.textContent)).toEqual(['5.00%','9.12%','-47.82%','0.19','50.00%','2','52.73%']);
  expect([...el.querySelectorAll('.lab-report-card caption')].map(c=>c.textContent)).toEqual(['Returns','Trades','Winners','Losers','Risk & ratios']);
  expect(el.textContent).toContain('Ending capital');expect(el.textContent).toContain('1,050,000.00');
  await act(async()=>(el.querySelectorAll('[role=tab]')[1] as HTMLButtonElement).click());
  expect(el.textContent).toContain('Tradesheet 2 trades');expect(el.querySelector('[data-lab-download-ts]')?.textContent).toBe('Download TS');
  await act(async()=>(el.querySelectorAll('[role=tab]')[2] as HTMLButtonElement).click());
  expect(el.querySelector('[data-lab-yoy-download]')).not.toBeNull();
});

it('keeps the whole-unit money columns and running drawdown in the trade table',()=>{
  const t=tradeTable(ytRunResult(engine,'NIFTYBANK'));
  expect(t.header).toEqual(['Symbol','Trade type','Entry date','Entry price','Exit date','Exit price','% Change','Bars held','Days','Net profit','MAE','MFE','DD %','Shares','Position value','Costs']);
  expect(t.rows[0].slice(0,10)).toEqual(['NIFTYBANK','Long','01-01-2020','100.00','02-01-2020','110.00','10.00%','1','1','100,000.00']);
  expect(t.rows[1][12]).toBe('-4.5%');
  expect(t.csv[0].at(-1)).toBe('Exit reason');
});

// Two settings: Initial ATR × (2, 3) and Favour ATR × (2, 3); 3 × 2 is out of order and comes back unranked (nulls).
const names=['Net Profit','Net % Profit','Exposure %','CAR','RAR','Max. Trade Drawdown','Max. Trade % Drawdown','Max. Sys Drawdown','Max. Sys % Drawdown','Recovery Factor','CAR/MDD','RAR/MDD','Profit Factor','Payoff Ratio','Standard Error','RRR','Ulcer Index','Ulcer Perf. Index','Sharpe Ratio','K-Ratio','# Trades','Avg Profit/Loss','Avg % Profit/Loss','Avg Bars Held','# of winners','% of Winners','W. Tot. Profit','W. Avg. Profit','W. Avg % Profit','W. Avg. Bars Held','# of losers','% of Losers','L. Tot. Loss','L. Avg. Loss','L. Avg % Loss','L. Avg. Bars Held'];
const sheetRow=(a:number,f:number,carMdd:number)=>[a,f,...names.map(n=>n==='CAR/MDD'?carMdd:n==='Avg % Profit/Loss'?2.3:n==='Avg Bars Held'?28.42:n==='# Trades'?128:n==='W. Avg % Profit'?9:n==='L. Avg % Loss'?-3.6:n==='% of Winners'?46.88:n==='% of Losers'?53.12:n.includes('Bars Held')?20:1)];
const opt:YtOptimizeResult={from:'2012-10-01',to:'2023-10-31',combos:4,ms:900,threads:4,dims:[{key:'atr_mult',values:[2,3]},{key:'fav_mult',values:[2,3]}],cols:['p.atr_mult','p.fav_mult','cagr','trades'],book:[0.1,50],
  rank:{eligible:3,distinct:3,total:4,min_trades:13,per_year:1.2,top3:[{row:1,score:80,pct:[1]},{row:3,score:60,pct:[1]},{row:0,score:40,pct:[1]}]},
  picked:{'0':[2,2,0.08,60],'1':[2,3,0.09,128],'3':[3,3,0.07,40]},rows:[],sheet_cols:names,
  sheet:[sheetRow(2,2,0.1),sheetRow(2,3,0.19),[3,2,...names.map(()=>null)],sheetRow(3,3,0.15)],
  table_cols:['cagr','trades'],table:[[2,2,0.08,60],[2,3,0.09,128],[3,2,null,null],[3,3,0.07,40]]};
const ctx:YtOptimizeContext={dir:'long',label:k=>({atr_mult:'Initial ATR multiplier',fav_mult:'Favour ATR multiplier'})[k]??k,group:()=>'Dual ATR exit',instrumentLabel:'NIFTY BANK',ticker:'NIFTYBANK',
  strategy:'Monday Reversal · Type 1',aflPath:'Monday Reversal.afl',capital:1_000_000,range:()=>'2,3,1',rules:{filter:[],setup:[],entry:['Monday'],exit:['Dual ATR']},
  now:k=>({atr_mult:2,fav_mult:2})[k]??NaN,show:(_k,v)=>String(v),runCombo:async()=>ytRunResult(engine,'NIFTYBANK'),tsRules:()=>rules,loadBars:async()=>[],onApply:vi.fn()};

it('lists the optimization as the team sheet: CAR/MDD first, Final columns, out-of-order ATR dropped',()=>{
  const r=withoutAtrOutOfOrder(opt,ctx.now);
  expect(r.skipped).toBe(1);
  const list=optimizationList(r,'long',ctx.label)!;
  expect(list.rows.map(x=>x[0])).toEqual([2,4,1]);
  expect(list.shown.map(i=>list.head[i])).toEqual(['Initial ATR multiplier','Favour ATR multiplier','CAR','Max. Sys % Drawdown','CAR/MDD','# Trades','Avg % Profit/Loss','Avg Bars Held','% of Winners','Expectancy Ratio','Final Avg % Profit/Loss','Final M. Thk','Final Expectancy Ratio','Total Net Profit']);
  const finalAvg=list.rows[0][list.head.indexOf('Final Avg % Profit/Loss')]!;
  expect(finalAvg).toBeCloseTo(2.3-(1.4*27.42*0.033+0.3),9);
  const [rulesSheet,sheet]=optimizationWorkbook(r,ctx)!;
  expect(rulesSheet.rows.some(row=>row[2]==='Initial ATR multiplier - 2,3,1')).toBe(true);
  expect(sheet.rows.length).toBe(4);
});

it('shows Now and #1-#3 with Stats, the surface, and the downloads',async()=>{
  const el=await mount(<YtOptimize result={opt} ctx={ctx}/>);
  expect(el.textContent).toContain('4 settings tested · 3 eligible (at least 13 trades) · 3 distinct · 1 skipped: Initial ATR × above Favour ATR ×');
  const top=[...el.querySelectorAll('[data-lab-optimize-top] tbody tr')].map(tr=>tr.firstElementChild!.textContent);
  expect(top).toEqual(['Now','#1','#2','#3']);
  expect(el.querySelector('.lab-surface, svg')).not.toBeNull();
  expect(el.textContent).not.toContain('Download all tradesheets');
  await act(async()=>(el.querySelector('[aria-label="Stats #1"]') as HTMLButtonElement).click());
  expect(ctx.onApply).toHaveBeenCalledWith({atr_mult:2,fav_mult:3});
});

it('builds Download backtest results as one workbook: Settings, Statistics, Trades, YOY-Monthly',()=>{
  const book=backtestWorkbook(ytRunResult(engine,'NIFTYBANK','NIFTY BANK'),rules);
  expect(book.map(s=>s.name)).toEqual(['Settings','Statistics','Trades','YOY-Monthly']);
  expect(book[0].rows.some(r=>r[1]==='Index:'&&r[2]==='NIFTY BANK')).toBe(true);
  expect(book[0].rows.some(r=>r[2]==='Entry condition: Fri O>Mon H')).toBe(true);
  expect(book[2].rows.length).toBe(3);
});

it('shows Download TS at the header box corner; Download backtest results is switched off',async()=>{
  const {ytBridge,YtDownloadSlot}=await import('../src/features/yt-results/bridge');
  ytBridge.setResults({result:ytRunResult(engine,'NIFTYBANK','NIFTY BANK'),rules,loadBars:async()=>[]});
  const el=await mount(<YtDownloadSlot/>);
  expect([...el.querySelectorAll('button')].map(b=>b.textContent)).toEqual(['Download TS']);
  await act(async()=>ytBridge.setResults(null));
  expect(el.querySelector('button')).toBeNull();
});

it('puts Download optimization results beside Run optimization, greyed until a result exists',async()=>{
  const {ytBridge,YtOptimizeDownloadSlot}=await import('../src/features/yt-results/bridge');
  await act(async()=>ytBridge.setOptimize(null));
  const el=await mount(<YtOptimizeDownloadSlot/>), b=()=>el.querySelector('[data-yt-optimize-download]') as HTMLButtonElement;
  expect(b().textContent).toBe('Download optimization results');expect(b().disabled).toBe(true);
  await act(async()=>ytBridge.setOptimize({result:opt,ctx}));
  expect(b().disabled).toBe(false);
});

it('mounts the YT optimizer and its download beside Run optimization on the optimization page',async()=>{
  const {OptimizationPage}=await import('../src/pages/ResearchPages');
  const {ytBridge}=await import('../src/features/yt-results/bridge');
  await act(async()=>ytBridge.setOptimize({result:opt,ctx}));
  const html='<section class="box" id="optim"><form id="oform"><button id="orun">Run optimization</button><span data-react-slot="yt-optimize-download"></span></form><div data-react-slot="yt-optimize"></div></section>';
  const el=await mount(<OptimizationPage snapshot={{revision:1,loading:false,html,error:null,route:null,status:'',state:{},home:null,reading:null} as never}/>);
  expect(el.querySelector('[data-yt-optimize-download]')).not.toBeNull();
  expect(el.querySelector('[data-lab-optimize-top]')).not.toBeNull();
  expect(el.querySelector('[data-lab-optimize-sheet-list]')).not.toBeNull();
  await act(async()=>ytBridge.setOptimize(null));
});
it('the Rollover setting fills the tradesheet D1 (long +x%) and E1 (short -x%) per 30 days',()=>{
  expect(tsCosts(1)).toEqual({impact:0.003,rollLong:0.01,rollShort:-0.01});
  expect(tsCosts(0)).toEqual({impact:0.003,rollLong:0,rollShort:0});
  expect(ytRunResult(engine,'NIFTYBANK','NIFTY BANK',2).rollover).toBe(2);
});
it('report downloads write dd-mm-yyyy dates as Excel dates, so Excel can average them, and keep other text as text',()=>{
  const book=new TextDecoder().decode(xlsx([{name:'Trades',rows:[['Entry date','Exit date'],['05-01-2012','Open'],['31-02-2012','29-02-2012']]}]));
  expect(book).toContain('<c r="A2" s="9"><v>40913</v></c>');
  expect(book).toContain('<c r="B3" s="15"><v>40968</v></c>');
  expect(book).toMatch(/<c r="B2" s="4" t="inlineStr"><is><t xml:space="preserve">Open/);
  expect(book).toMatch(/<c r="A3" s="10" t="inlineStr">/);
  expect(book).toContain('formatCode="dd-mm-yyyy"');
});
