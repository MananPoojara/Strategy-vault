import {test} from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {createRequire} from 'node:module';
import {runInNewContext} from 'node:vm';
const require=createRequire(import.meta.url),OptimCore=require('../../frontend/optim-core.js'),VaultAnalyst=require('../../frontend/analyst-conclusion.js');
const source=readFileSync(new URL('../../frontend/react/src/compat/runtime.js',import.meta.url),'utf8');
const esc=s=>String(s??'').replace(/[&<>"']/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
function col(index){let out='';for(let n=index+1;n;n=Math.floor((n-1)/26))out=String.fromCharCode(65+(n-1)%26)+out;return out;}
function harness(){
 const written=[],utils={encode_col:col,encode_cell:({r,c})=>col(c)+(r+1),encode_range:({s,e})=>col(s.c)+(s.r+1)+':'+col(e.c)+(e.r+1),book_new:()=>({SheetNames:[],Sheets:{}}),book_append_sheet:(wb,ws,name)=>{wb.Sheets[name]=ws;wb.SheetNames.push(name);}};
 const XLSX={utils,writeFile:(wb,name)=>written.push({wb,name})};
 const sandbox={window:{XLSX},XLSX,OptimCore,VaultAnalyst,API:'/api',API_IN:'/api',S:{cost:'0',slippage:'5',rollover:'0',p:'fill:1',from:'2010-01-01',to:'2020-12-31'},R:{path:'/s/synthetic'},PUB:2015,TICKERS:[{ticker:'SYNTHETIC',first:'2000-01-01'}],LABEL:{SYNTHETIC:'Synthetic test stock'},BOOK:{test:{title:'Synthetic source title'}},esc,
  pct:(v,d=2)=>typeof v==='number'&&Number.isFinite(v)?(v*100).toFixed(d)+'%':'–',fx:(v,d=2)=>typeof v==='number'&&Number.isFinite(v)?v.toFixed(d):'–',cls:v=>v>0?'pos':v<0?'neg':'',isUS:()=>false,href:()=> '#/s/synthetic',console};
 const start=source.indexOf('const OPT =');runInNewContext(source.slice(start,source.indexOf('\nconst cv =',start)),sandbox);
 runInNewContext('OPT.schemaSync={synthetic:[{key:"fill",label:"Next open",def:0}]};globalThis.testOPT=OPT;globalThis.testDSet=dSet;',sandbox);
 return {sandbox,written};
}
function fixture(){
 // Clearly synthetic presentation fixture. This is not a published market result.
 const base={cagr:.02,net_profit:.01,total_return:.01,max_dd:-.1,win_rate:.5,sharpe:.2,monthly_thickness:.001,trades:2,expectancy_ratio:.1,mar:.2,car_mdd:.2},best={...base,cagr:.03,sharpe:.3,mar:.3,car_mdd:.3};
 const keys=Object.keys(base).filter(k=>k!=='car_mdd'),rankKeys=['cagr','sharpe','monthly_thickness','expectancy_ratio','mar'];
 const c={row:1,score:100,weakest:100,pct:[100,100,100,100,100],vs_leader:[1,1,1,1,1],groups:[0,1,2,3,4]};
 const res={id:'synthetic',dims:[{key:'fill',values:[0,1]}],fixed:{},defaults:{fill:0},cols:['fill',...keys],book:keys.map(k=>base[k]),rows:[ [0,...keys.map(k=>base[k])],[1,...keys.map(k=>best[k])] ],rank:{top3:[c],pool:[c],groups:rankKeys.map(stat=>({stat,label:stat,rows:[1,0]})),eligible:2,min_trades:10,per_year:1},combos:2,from:'2010-01-01',to:'2020-12-31'};
 const data=m=>({stats:m,dates:['2020-01-01','2020-01-02','2020-01-03'],equity:[1,1.1,.99],trades:[{entry:'2020-01-01',exit:'2020-01-02',ret:.1,open:false,legs:[{i:0,px:100,w:1}],avg_px:100,exit_px:110,reason:'Synthetic close'},{entry:'2020-01-02',exit:'2020-01-03',ret:-.1,open:false,legs:[{i:1,px:110,w:1}],avg_px:110,exit_px:99,reason:'Synthetic close'}]});
 const runs=[{name:'Default',book:true,p:{fill:0},m:base,d:data(base)},{name:'#1',book:false,c,p:{fill:1},m:best,d:data(best)}];
 return {v:{id:'synthetic',name:'Synthetic presentation',family_name:'Synthetic presentation',book:'test'},tk:'SYNTHETIC',res,def:{fill:0},runs,top:runs.slice(1),win:{from:'2020-01-01',to:'2020-01-03'},kind:'custom',pub:2015,cost:0,slippage:5,rollover:0,region:'in',selectionLabel:'Synthetic test window',made:'2026-10-15'};
}
test('active custom-settings banner identifies schema defaults without claiming printed book values',()=>{
 const {sandbox}=harness(),html=sandbox.optimBanner({id:'synthetic'});
 assert.ok(html.includes('default 0'));assert.ok(html.includes('Back to default settings'));
 assert.ok(!/book settings|book 0|as printed/i.test(html));
 assert.equal(sandbox.testDSet({fill:1},{fill:0}),'fill 1 (default 0)');
});
test('active conclusions retain publication periods and baseline flags while labelling configured defaults',async()=>{
 const {sandbox}=harness(),D=fixture(),A=VaultAnalyst.analyze(D);
 const html=sandbox.conclusionHtml(D)+sandbox.gptDetailHtml(D,A);
 assert.ok(html.includes('Default'));assert.ok(html.includes('default baseline'));assert.ok(html.includes('change from default settings'));
 assert.ok(html.includes('Same settings before and after the book (2015)'));
 assert.ok(!/as printed|vs book|change from book/.test(html));
 const before=JSON.stringify(D.res),calls=[];
 sandbox.runData=async(...args)=>{calls.push(args);return D.runs[0].d;};
 const actual=await sandbox.conclusionData(D.v,D.tk,D.res,D.win,{api:'/api',cost:0,slippage:5,rollover:0,pub:2015,us:false,first:'2000-01-01'});
 assert.equal(actual.runs[0].name,'Default');assert.equal(actual.runs[0].book,true);assert.equal(JSON.stringify(D.res),before);
 assert.equal(calls.length,6);assert.ok(calls.filter(call=>call[5]).every(call=>call[5]==='fill:1'));
 assert.equal(A.book.name,'Default');assert.ok(A.candidates[0].summary.some(s=>s.includes('vs default')));
});
test('active workbook exports Default reference labels and still links every calculation to its renamed trade sheet',async()=>{
 const {sandbox,written}=harness(),D=fixture(),before=JSON.stringify(D);
 await sandbox.exportXlsx(D,true);assert.equal(written.length,1);assert.equal(JSON.stringify(D),before);
 const wb=written[0].wb;assert.ok(wb.SheetNames.includes('Trades Default'));assert.ok(!wb.SheetNames.includes('Trades Book'));
 const strings=Object.values(wb.Sheets).flatMap(ws=>Object.values(ws).filter(c=>c&&typeof c.v==='string').map(c=>c.v));
 assert.ok(strings.some(s=>s.includes('Default settings')));assert.ok(strings.some(s=>s.includes('Synthetic source title')));
 assert.ok(strings.some(s=>s.includes('Same settings before and after the book')));
 assert.ok(!strings.some(s=>/Book settings, as printed|differs from the book|half the book[’']s yearly rate/.test(s)));
 const formulas=Object.values(wb.Sheets.Formulas).filter(c=>typeof c?.f==='string').map(c=>c.f);
 assert.ok(formulas.some(f=>f==="SUM('Trades Default'!D5:D6)"));assert.ok(formulas.some(f=>f.includes('AVERAGEIF')));
 assert.ok(!formulas.some(f=>f.includes('Trades Book')));
});
