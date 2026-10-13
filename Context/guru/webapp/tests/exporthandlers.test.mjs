import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import vm from 'node:vm';
const vendorContext={Uint8Array,ArrayBuffer,TextEncoder,TextDecoder,setTimeout,clearTimeout};
vm.runInNewContext(fs.readFileSync(new URL('../../frontend/vendor/xlsx-js-style-1.2.0.min.js',import.meta.url),'utf8'),vendorContext);
const XLSX=vendorContext.XLSX;
const reportCode=fs.readFileSync(new URL('../../frontend/report-ui.js',import.meta.url),'utf8');
const reportDataCode=fs.readFileSync(new URL('../../frontend/report-data.js',import.meta.url),'utf8');

function fixture(){return {dates:['2024-01-02','2024-01-09'],equity:[1,1.099],gross_equity:[1,1.1],bh:[1,1.1],stats:{total_return:.099,trades:1},gross_stats:{total_return:.1},costs:{fees_bps:3,slippage_bps:2,rollover_bps:0},provenance:{symbol:'REAL_SYMBOL',source_sha256:'a'.repeat(64),csv_sha256:'b'.repeat(64),membership:'Current snapshot; historical membership unverified'},trades:[{entry:'2024-01-02',exit:'2024-01-09',avg_px:100,exit_px:110,ret:.099,reason:'close, rule',accounting:{symbol:'REAL_SYMBOL',signal_side:'Long',pts:10,gross_ret:.1,net_ret:.099,days_held:7,slippage_ret:.001,mae_ret_observed:null,mfe_ret_observed:null,gross_pnl:null,net_pnl:null,taxes:null}}]};}
function harness({deferred=false}={}){
 const events=new Map(),downloads=[],objects=new Map(),status={textContent:''},images=[],canvases=[];
 let release;const loaded=deferred?new Promise(resolve=>release=resolve):Promise.resolve();
 const canvas=()=>({width:100,height:50,getContext:()=>({drawImage(){},fillRect(){},fillStyle:''}),toBlob(fn,type){fn(new Blob(['recorded pixels'],{type}));},toDataURL:()=> 'data:image/png;base64,cGl4ZWxz'});
 const elements=new Map([['page-action-message',status]]);
 const document={addEventListener(type,fn){const handlers=events.get(type)||[];handlers.push(fn);events.set(type,handlers);},dispatchEvent(){},getElementById:id=>elements.get(id)||null,querySelector:()=>null,querySelectorAll:()=>[],head:{append(){}},body:{append(){},classList:{add(){},remove(){}}},documentElement:{scrollWidth:1000},createElement(type){if(type==='canvas'){const c=canvas();canvases.push(c);return c;}if(type==='a')return {href:'',download:'',click(){downloads.push({name:this.download,blob:objects.get(this.href)});},remove(){}};return {dataset:{},addEventListener(){},setAttribute(){},remove(){}};}};
 class ImageStub{set src(value){images.push(value);queueMicrotask(()=>this.onload?.());}}
 const context={document,Blob,URL:{createObjectURL(blob){const id='blob:'+objects.size;objects.set(id,blob);return id;},revokeObjectURL(){}},XLSX,loadXlsx:()=>loaded,setTimeout:()=>0,clearTimeout(){},Image:ImageStub,XMLSerializer:class{serializeToString(){return '<svg xmlns="http://www.w3.org/2000/svg" width="100" height="50"><text>Recorded chart</text></svg>'; }},btoa:s=>Buffer.from(s,'binary').toString('base64'),unescape,encodeURIComponent,CustomEvent:class{},navigator:{clipboard:{writeText:async()=>{}}},localStorage:{getItem:()=>null,setItem(){}},location:{href:'http://local/#/s/f1a',hash:'#/s/f1a'},addEventListener(){},getComputedStyle:()=>({backgroundColor:'#fff',fontFamily:'Arial',color:'#000',getPropertyValue:()=>''}),finishMetricAnimations(){},LABEL:{'basket:nifty50:REAL_SYMBOL':'Real symbol',SPX:'S&P 500'},S:{region:'in',tk:'basket:nifty50:REAL_SYMBOL',cost:'3',slippage:'2',rollover:'0',p:{hold:5}},R:{name:'strategy',id:'f1a'},STRAT:{d:fixture(),tk:'basket:nifty50:REAL_SYMBOL',v:{id:'f1a',name:'Measured rule',book:'stc9'},px:null},CHARTS:[],ohlc:async()=>null};
 context.window=context;vm.createContext(context);vm.runInContext(reportDataCode,context);vm.runInContext(reportCode,context);
 async function click(id='',dataset={}){const button={id,dataset};const target={closest(selector){return selector.includes('[data-action]')?button:null;}};await Promise.all((events.get('click')||[]).map(fn=>fn({target})));}
 return {context,click,release,downloads,status,images,canvases,elements,canvas};
}
const cells=sheet=>Object.entries(sheet).filter(([key])=>!key.startsWith('!')).map(([,cell])=>cell);
const values=sheet=>cells(sheet).map(cell=>cell.v);

test('trades XLSX handler captures actual report before awaiting loader and keeps 17 columns/null money',async()=>{
 const h=harness({deferred:true}),pending=h.click('trades-xlsx');
 h.context.S.cost='99';h.context.S.tk='WRONG';h.context.STRAT.d.trades[0].accounting.symbol='WRONG';h.context.STRAT.v.id='WRONG';h.release();await pending;
 assert.equal(h.downloads.length,1,h.status.textContent);
 const bytes=new Uint8Array(await h.downloads[0].blob.arrayBuffer()),book=XLSX.read(bytes,{type:'array',cellStyles:true});
 const sheet=book.Sheets.Trades;assert.ok(sheet,'Trades worksheet');
 const rows=XLSX.utils.sheet_to_json(sheet,{header:1,defval:null});
 assert.equal(rows[0].length,17);assert.equal(rows[1][0],'REAL_SYMBOL');assert.equal(rows[1][7],null);assert.equal(rows[1][10],null);assert.equal(rows[1][12],null);assert.equal(rows[1][8],.1);assert.equal(rows[1][13],.099);
 assert.match(sheet.I2.z,/0\.00%/);assert.match(sheet.N2.z,/0\.00%/);
 assert.ok(book.Sheets.Metadata&&book.Sheets.Sources);
 const meta=values(book.Sheets.Metadata);assert.ok(meta.includes('REAL_SYMBOL'));assert.ok(meta.includes(3));assert.ok(!meta.includes(99));
 assert.ok(values(book.Sheets.Sources).includes('a'.repeat(64)));assert.ok(values(book.Sheets.Sources).some(x=>String(x).includes('not certification')));
 assert.ok(!h.downloads[0].name.includes('WRONG'));
});

test('CSV handler downloads recorded rows with quoting, blank money and formula-safe text',async()=>{
 const h=harness();h.context.STRAT.d.trades[0].reason=' =HYPERLINK("bad"),\ntext';await h.click('csv');
 assert.equal(h.downloads.length,1,h.status.textContent);const text=await h.downloads[0].blob.text();
 assert.ok(text.includes('Symbol,Signal,Entry Date'));assert.ok(text.includes('REAL_SYMBOL,LONG'));assert.ok(text.includes("' =HYPERLINK"));assert.ok(!text.includes('undefined'));
});

test('native chart PNG handler captures registered chart rather than exporting an empty canvas',async()=>{
 const h=harness();let captured=0;h.context.CHARTS=[{el:{id:'c-eq'},ch:{takeScreenshot(){captured++;return h.canvas();}}}];
 h.elements.set('c-eq',{id:'c-eq',getBoundingClientRect:()=>({width:100,height:50}),querySelector:()=>null});await h.click('',{chartSnapshot:'c-eq'});
 assert.equal(captured,1,h.status.textContent);assert.equal(h.downloads.length,1,h.status.textContent);assert.equal(h.downloads[0].blob.type,'image/png');assert.ok(h.canvases.every(c=>c.width*c.height<=8_000_000));
});

test('benchmark XLSX retains captured displayed market benchmark instead of selected-stock buy-and-hold',async()=>{
 const h=harness();h.context.VaultUI.benchmarkHTML(h.context.STRAT.d,'SPX',{d:['2024-01-02','2024-01-09'],c:[100,105]});
 await h.click('',{reportExport:'benchmark'});assert.equal(h.downloads.length,1,h.status.textContent);
 const book=XLSX.read(new Uint8Array(await h.downloads[0].blob.arrayBuffer()),{type:'array'});
 const rows=XLSX.utils.sheet_to_json(book.Sheets.Benchmark,{header:1,defval:null});
 assert.ok(rows[0][2].includes('S&P 500'));assert.ok(Math.abs(rows[1][2]-.05)<1e-12);
 assert.ok(values(book.Sheets.Metadata).includes('SPX'));
});

test('SVG chart exports through a CSP-compatible captured data URI with bounded pixels',async()=>{
 const h=harness(),clone={setAttribute(){},querySelectorAll:()=>[]};
 const svg={cloneNode:()=>clone,getBoundingClientRect:()=>({width:8000,height:4000}),querySelectorAll:()=>[]};
 h.elements.set('drawdown-chart',{querySelector:selector=>selector==='svg'?svg:null});
 await h.click('',{chartSnapshot:'drawdown-chart'});
 assert.equal(h.downloads.length,1,h.status.textContent);assert.equal(h.images.length,1);assert.match(h.images[0],/^data:image\/svg\+xml;base64,/);
 assert.ok(Buffer.from(h.images[0].split(',')[1],'base64').toString().includes('Recorded chart'));
 assert.ok(h.canvases.every(c=>c.width*c.height<=8_000_000));
});

test('details, monthly and equity handlers write recorded values and percentage formats',async()=>{
 const h=harness();h.context.STRAT.d.stats.profit_factor=null;h.context.STRAT.d.stats.cagr=.15;
 for(const kind of ['details','monthly','equity'])await h.click('',{reportExport:kind});
 assert.equal(h.downloads.length,3,h.status.textContent);
 const books=await Promise.all(h.downloads.map(async file=>XLSX.read(new Uint8Array(await file.blob.arrayBuffer()),{type:'array',cellStyles:true})));
 for(const book of books)assert.ok(book.Sheets.Metadata&&book.Sheets.Sources&&book.Sheets.Parameters);
 const monthly=books[1].Sheets['Monthly returns'],rows=XLSX.utils.sheet_to_json(monthly,{header:1,defval:null});
 assert.equal(rows[1][0],'2024');assert.ok(Math.abs(rows[1][1]-.099)<1e-12);assert.equal(rows[1][2],null);assert.match(monthly.B2.z,/0\.00%/);
 const equity=XLSX.utils.sheet_to_json(books[2].Sheets.Equity,{header:1,defval:null});assert.equal(equity[1][1],1);assert.equal(equity[2][1],1.099);
 const detailCells=cells(books[0].Sheets['Detailed statistics']);assert.ok(detailCells.some(c=>c.v===.15&&c.z.includes('%')));
});

test('XLSX source strings remain text while zero-trade exports contain no fabricated trade',async()=>{
 const h=harness();h.context.STRAT.d.trades=[];h.context.STRAT.d.provenance.source='=HYPERLINK("untrusted")';
 await h.click('trades-xlsx');assert.equal(h.downloads.length,1,h.status.textContent);
 const book=XLSX.read(new Uint8Array(await h.downloads[0].blob.arrayBuffer()),{type:'array'});
 assert.equal(XLSX.utils.sheet_to_json(book.Sheets.Trades,{header:1,defval:null}).length,1);
 const sourceCell=cells(book.Sheets.Sources).find(c=>c.v==='=HYPERLINK("untrusted")');assert.ok(sourceCell);assert.equal(sourceCell.t,'s');assert.equal(sourceCell.f,undefined);
});

test('page navigation during PDF preparation prevents wrong-page download and print fallback',async()=>{
 const h=harness();let printed=0;h.context.print=()=>printed++;
 h.elements.set('main',{scrollWidth:1000,scrollHeight:1000});
 h.context.document.head.append=script=>{h.context.location.href='http://local/#/s/other';queueMicrotask(()=>script.onload());};
 await h.click('',{action:'pdf'});
 assert.equal(h.downloads.length,0);assert.equal(printed,0);assert.match(h.status.textContent,/page changed/);
});

test('Like toggles a per-account browser preference without aggregate counts or export side effects',async()=>{
 const h=harness(),storage=new Map(),attrs=new Map();
 h.context.localStorage={getItem:key=>storage.get(key)||null,setItem:(key,value)=>storage.set(key,value)};
 h.context.VAULT_USER={id:'account-A'};
 const button={innerHTML:'',title:'',classList:{toggle(name,on){attrs.set(name,on);}},setAttribute:(key,value)=>attrs.set(key,value)};
 h.context.document.querySelector=selector=>selector==='[data-action=like]'?button:null;
 assert.match(h.context.VaultUI.toolbar(),/data-action="like"[^>]*aria-pressed="false"/);
 await h.click('',{action:'like'});assert.equal(attrs.get('aria-pressed'),'true');assert.equal(attrs.get('on'),true);
 assert.match(button.innerHTML,/ic-line/);assert.match(button.innerHTML,/<span>Like<\/span>/);assert.match(h.status.textContent,/this browser/);assert.equal(h.downloads.length,0);
 assert.equal(storage.size,1);assert.ok([...storage.keys()][0].includes('account-A'));
 assert.match(h.context.VaultUI.toolbar(),/data-action="like"[^>]*aria-pressed="true"/);
 h.context.VAULT_USER={id:'account-B'};assert.match(h.context.VaultUI.toolbar(),/data-action="like"[^>]*aria-pressed="false"/);
 h.context.VAULT_USER={id:'account-A'};await h.click('',{action:'like'});assert.equal(attrs.get('aria-pressed'),'false');
 assert.equal(JSON.parse([...storage.values()][0]).length,0);
});

test('Like remains truthful when browser storage fails and corrupt stored preferences recover',async()=>{
 const h=harness();h.context.localStorage={getItem:()=>'{corrupt',setItem(){throw Error('blocked');}};
 assert.match(h.context.VaultUI.toolbar(),/data-action="like"[^>]*aria-pressed="false"/);
 await h.click('',{action:'like'});assert.match(h.status.textContent,/Could not store/);assert.equal(h.downloads.length,0);
 assert.match(h.context.VaultUI.toolbar(),/data-action="like"[^>]*aria-pressed="false"/);
});
