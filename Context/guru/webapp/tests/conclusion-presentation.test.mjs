import {test} from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {createRequire} from 'node:module';
import {runInNewContext} from 'node:vm';
const require=createRequire(import.meta.url);
const VaultAnalyst=require('../../frontend/analyst-conclusion.js');
const OptimCore=require('../../frontend/optim-core.js');
const captured=JSON.parse(readFileSync(process.env.VAULT_OPT_FIXTURE||new URL('../verification/joe-ross/optimization-3-row-input.json',import.meta.url),'utf8'));
const esc=s=>String(s??'').replace(/[&<>"']/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
function column(index){let out='';for(let n=index+1;n;n=Math.floor((n-1)/26))out=String.fromCharCode(65+(n-1)%26)+out;return out;}
function harness(){
 const written=[];
 const utils={encode_col:column,encode_cell:({r,c})=>column(c)+(r+1),encode_range:({s,e})=>column(s.c)+(s.r+1)+':'+column(e.c)+(e.r+1),book_new:()=>({SheetNames:[],Sheets:{}}),book_append_sheet:(wb,ws,name)=>{wb.Sheets[name]=ws;wb.SheetNames.push(name);}};
 const XLSX={utils,writeFile:(wb,name)=>written.push({wb,name})};
 const sandbox={window:{XLSX},XLSX,OptimCore,VaultAnalyst,API:'/api',API_IN:'/api',S:{cost:'9',region:'us',from:'',to:''},PUB:2010,TICKERS:[{ticker:'NIFTY',first:'2000-01-01'}],LABEL:{NIFTY:'NIFTY'},BOOK:{test:{title:'Test book'}},esc,
  pct:(v,d=2)=>typeof v==='number'&&Number.isFinite(v)?(v*100).toFixed(d)+'%':'–',fx:(v,d=2)=>typeof v==='number'&&Number.isFinite(v)?v.toFixed(d):'–',cls:v=>v>0?'pos':v<0?'neg':'',isUS:()=>true,href:()=> '#/o/f1a',console};
 runInNewContext(readFileSync(new URL('../../frontend/optim.js',import.meta.url),'utf8'),sandbox);
 runInNewContext('OPT.schemaSync={f1a:[{key:"hold",def:5},{key:"trend",def:200}]};',sandbox);
 return {sandbox,written};
}
function metrics(res,values){const m=Object.fromEntries(res.cols.slice(res.dims.length).map((k,i)=>[k,values[i]]));return {...m,net_profit:m.total_return,car_mdd:m.mar};}
function fixture(){
 const res=structuredClone(captured);res.fixed={trend:250};
 const v={id:'f1a',name:'Captured parameter research',family_name:'Captured research',book:'test'},def={hold:5,trend:200};
 // Equity/trades here are private presentation fixtures, never published market results.
 const data=m=>({stats:m,dates:['2020-01-01','2020-01-02','2020-01-03'],equity:[1,.98,.97],trades:[{entry:'2020-01-01',exit:'2020-01-03',ret:-.03,open:false,legs:[{i:0,px:100,w:1}],avg_px:100,exit_px:97,reason:'Test loss'}]});
 const bm=metrics(res,res.book),runs=[{name:'Book',book:true,p:def,m:bm,d:data(bm)}];
 res.rank.top3.forEach((c,i)=>{const values=res.rows[c.row],m=metrics(res,values.slice(1));runs.push({name:'#'+(i+1),c,p:{hold:values[0],trend:250},m,d:data(m)});});
 return {v,tk:'NIFTY',res,def,runs,top:runs.slice(1),win:{from:'2020-01-01',to:'2020-01-03'},kind:'custom',pub:2010,cost:3,region:'in',selectionLabel:'Custom dates',made:'2026-10-09'};
}
test('candidate reconstruction retains varied values and manually fixed overrides',()=>{
 const {sandbox}=harness(),D=fixture(),r=sandbox.rowOf(D.res,D.res.rows[1]);
 assert.equal(r.p.hold,3);assert.equal(r.p.trend,250);assert.equal(r.m.car_mdd,.52791);
});
test('conclusionData captures cost, region and selection context independently of live globals',async()=>{
 const {sandbox}=harness(),D=fixture(),calls=[];
 sandbox.runData=async(id,tk,from,to,cost,p,api,execution)=>{calls.push({from,to,cost,p,api,execution});return {stats:D.runs[0].m,dates:['2020-01-01','2020-01-03'],equity:[1,1],trades:[]};};
 const context={api:'/captured-india',cost:'3',slippage:'2',rollover:'0',pub:2010,us:false,first:'2000-01-01',selectionLabel:'Custom dates'};
 const result=await sandbox.conclusionData(D.v,D.tk,D.res,D.win,context);
 assert.equal(result.cost,3);assert.equal(result.region,'in');assert.equal(result.selectionLabel,'Custom dates');
 assert.equal(calls.length,12);assert.ok(calls.every(c=>String(c.cost)==='3'&&c.api==='/captured-india'));assert.equal(result.slippage,2);assert.ok(calls.every(c=>c.execution.slippage==='2'&&c.execution.rollover==='0'));
 assert.ok(calls.filter(c=>c.p).every(c=>c.p.includes('trend:250')));
});
test('GPT presentation exposes nine metrics and distinct controls while escaping independent text',()=>{
 const {sandbox}=harness(),D=fixture(),A=VaultAnalyst.analyze(D);
 A.summary.push('<script>unsafe()</script>');
 const html=sandbox.gptConclusionHtml(D,A);
 for(const label of ['CAGR','Net profit','Max DD','Win %','Sharpe','Monthly thickness','Trades','Expectancy ratio','CAR/MDD'])assert.ok(html.includes(label),label);
 assert.ok(html.includes('gpt-more'));assert.ok(html.includes('gpt-xlsx'));
 assert.ok(!html.includes('id="cc-more"'));assert.ok(!html.includes('id="cc-xlsx"'));
 assert.ok(html.includes('&lt;script&gt;'));assert.ok(!html.includes('<script>unsafe'));
 assert.ok(html.includes('measured tradeoffs'));assert.ok(html.includes('2025-12-31'));
});
test('non-positive leaders cannot appear as a 200 percent performance advantage',()=>{
 const {sandbox}=harness(),D=fixture(),c=D.res.rank.top3[0];
 D.res.rows.forEach(r=>{r[2]=-1;});D.res.rows[c.row][2]=-2;c.vs_leader[0]=2;
 assert.equal(sandbox.ratioToLeader(D.res,c,0),null);
 const html=sandbox.gptDetailHtml(D,VaultAnalyst.analyze(D));
 assert.ok(!/(?<![-\d])200(?:\.00)?%/.test(html));
 D.res.rows[c.row][2]=0;assert.equal(sandbox.ratioToLeader(D.res,c,0),null);
});
test('GPT workbook retains captured cost and exports guarded all-loss expectancy formulas',async()=>{
 const {sandbox,written}=harness(),D=fixture();
 D.runs.forEach(r=>{r.m={...r.m,win_rate:0,expectancy_ratio:-1};r.d.stats=r.m;});
 await sandbox.exportXlsx(D,true);
 assert.equal(written.length,1);const {wb}=written[0];
 assert.ok(wb.SheetNames.includes('Formulas'));assert.ok(wb.SheetNames.includes('MoM'));assert.ok(wb.SheetNames.includes('Daily'));
 const cells=Object.values(wb.Sheets.Summary).filter(c=>c&&typeof c.v==='string').map(c=>c.v);
 assert.ok(cells.some(s=>/fees\s*3\s*\+\s*slippage\s*0\s*bps/i.test(s)),cells.slice(0,3).join('\n'));assert.ok(!cells.some(s=>/fees\s*9\s*\+/i.test(s)));
 assert.ok(!cells.some(s=>/settings chosen on all data/i.test(s)),'A custom-window selection must not be described as all-history selection');
 assert.ok(cells.some(s=>s.includes('GPT')));
 const formulas=wb.Sheets.Formulas,expectancy=[];
 for(const [address,cell] of Object.entries(formulas))if(cell?.v==='Expectancy ratio')expectancy.push(formulas['B'+address.match(/\d+$/)[0]]?.f);
 assert.equal(expectancy.length,4);
 expectancy.forEach(formula=>{assert.equal(typeof formula,'string');assert.ok(/IF(?:ERROR)?\(/i.test(formula),'All-loss average win must be guarded');assert.ok(formula.includes('AVERAGEIF'));assert.ok(formula.includes('<=0'));});
});
test('zero-trade workbook keeps undefined engine statistics and formula results blank, with unavailable checks',async()=>{
 const {sandbox,written}=harness(),D=fixture();
 D.runs.forEach(run=>{
  run.m=Object.fromEntries(VaultAnalyst.metricKeys.map(key=>[key,key==='trades'||key==='net_profit'?0:null]));
  run.d={...run.d,stats:run.m,equity:[1,1,1],trades:[]};
 });
 await sandbox.exportXlsx(D,true);
 const ws=written[0].wb.Sheets.Formulas;
 const labels=['CAGR','Max DD','Win %','Sharpe','Monthly thickness','Expectancy ratio','CAR/MDD'];
 let verified=0;
 for(const [address,cell] of Object.entries(ws)){
  if(!address.startsWith('A')||!labels.includes(cell?.v))continue;
  const row=address.match(/\d+$/)[0],engine=ws['C'+row];
  assert.equal(engine.t,'s');assert.equal(engine.v,'','Missing engine values cannot become zero');
  if(['Win %','Monthly thickness'].includes(cell.v)){
   assert.ok(ws['B'+row].f.endsWith(',"")'),cell.v+' must produce blank when no trades exist: '+ws['B'+row].f);
  }
  const scope={IF:(condition,a,b)=>condition?a:b,AND:(...values)=>values.every(Boolean),ISNUMBER:v=>typeof v==='number'&&Number.isFinite(v),ABS:Math.abs,MAX:Math.max};
  scope['B'+row]='';scope['C'+row]=engine.v;
  assert.equal(runInNewContext(ws['E'+row].f,scope),'unavailable','Blank engine/formula values must not falsely match');
  assert.equal(runInNewContext(ws['D'+row].f,scope),'','No numeric difference can be inferred from undefined statistics');
  verified++;
 }
 assert.equal(verified,28);
});
