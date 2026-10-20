import test from 'node:test';
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
const require=createRequire(import.meta.url);
const browser=require('../../frontend/library-browser.js');
const LIB=[
 {id:'minervini',name:'Mark Minervini',books:[{id:'mm_trade',title:'Trade Like a Stock Market Wizard',year:2013,sourceIds:['source-bound']},{id:'mm_think',title:'Think & Trade Like a Champion',year:2017}]},
 {id:'pristine',name:'Pristine Trading — Greg Capra & Oliver Velez',aliases:['prestine','pristine','capra','velez'],books:[{id:'p_intraday_capra',title:'Intra-Day Trading Techniques',year:2001},{id:'p_tools_tactics',title:'Tools and Tactics for the Master DayTrader',year:2000},{id:'p_tpm1',title:'Trading the Pristine Method — Part 1',year:null}]},
 {id:'empty',name:'Author with no indexed books',books:[]}
];
const coverage={authors:{minervini:{id:'minervini',name:'Mark Minervini',source_titles:[
 {id:'source-bound',title:'Wizard scan',catalogue_candidates:[{id:'mm_trade'}],files:[{filename:'wizard.pdf'}]},
 {id:'source-reading',title:'Unreviewed interview',summary:'Original discussion',years_reported:['2019'],files:[{filename:'interview-original.pdf'}]},
 {id:'source-reading',title:'Repeated index row',files:[]},
 {id:'source-unmatched',title:'Unmatched source candidate',catalogue_candidates:[{id:'not-in-LIB'}],files:[]}
]},unknown:{id:'unknown',name:'Unknown source attribution',source_titles:[{id:'source-editions',title:'Collected notes',years_reported:['1998','2004'],files:[{filename:'notes.doc'}]}]}}};
const categories={mm_trade:'Rule-based',mm_think:{categories:['Psychology','Money management'],certainty:'editorial'},p_intraday_capra:['rules'],p_tools_tactics:'rule_based'};

test('records distinguish catalogue entries and exact-ID-deduplicated reading sources',()=>{
 const before=JSON.stringify({LIB,coverage,categories}),rows=browser.records(LIB,coverage,categories);
 assert.equal(rows.filter(r=>r.kind==='catalogue').length,5);assert.equal(rows.filter(r=>r.kind==='source').length,3);
 assert.ok(!rows.some(r=>r.id==='source-bound'));assert.equal(rows.filter(r=>r.id==='source-reading').length,1);
 assert.equal(rows.find(r=>r.id==='source-reading').source.files[0].filename,'interview-original.pdf');
 assert.equal(rows.find(r=>r.id==='source-editions').year,null);assert.deepEqual(rows.find(r=>r.id==='p_tpm1').categories,[]);
 assert.equal(rows.find(r=>r.id==='p_tpm1').categoryCertainty,'unassigned');assert.equal(JSON.stringify({LIB,coverage,categories}),before);
});

test('search finds named authors, Pristine spelling aliases, titles and original filenames',()=>{
 const rows=browser.records(LIB,coverage,categories);
 assert.equal(browser.search(rows,{query:'MINERVINI'}).filter(r=>r.kind==='catalogue').length,2);
 assert.equal(browser.search(rows,{query:'prestine'}).length,3);assert.equal(browser.search(rows,{query:'velez'}).length,3);
 assert.equal(browser.search(rows,{query:'stock market wizard'}).length,1);assert.equal(browser.search(rows,{query:'interview-original.pdf'})[0].id,'source-reading');
 const accented=browser.records([{id:'cafe',name:'Émile',books:[{id:'é',title:'Café Trading',year:null}]}],{},{});
 assert.equal(browser.search(accented,{query:'cafe'}).length,1);assert.equal(browser.search(accented,{query:'EMILE'}).length,1);
});

test('category filtering uses declared categories without assigning unknown sources',()=>{
 const rows=browser.records(LIB,coverage,categories);
 assert.deepEqual(browser.search(rows,{category:'psychology'}).map(r=>r.id),['mm_think']);
 assert.deepEqual(browser.search(rows,{category:'money_management'}).map(r=>r.id),['mm_think']);
 assert.equal(browser.search(rows,{category:'rules'}).length,3);assert.equal(browser.search(rows,{category:'unknown'}).length,0);
 assert.equal(browser.search(rows,{query:'Pristine',category:'psychology'}).length,0);
});

test('sorting is stable, preserves original order and places unestablished years last',()=>{
 const rows=browser.records(LIB,coverage,categories),original=rows.map(r=>r.id);
 assert.deepEqual(browser.search(rows,{}).map(r=>r.id),original);
 const newest=browser.search(rows,{sort:'newest'});assert.equal(newest[0].id,'source-reading');assert.equal(newest[1].id,'mm_think');
 assert.equal(newest.at(-1).id,'source-editions');assert.deepEqual(rows.map(r=>r.id),original);
 const tie=[{id:'a',title:'Same',authorName:'Same',year:2000,categories:[]},{id:'b',title:'Same',authorName:'Same',year:2000,categories:[]}];
 for(const sort of ['title-asc','title-desc','author','newest'])assert.deepEqual(browser.search(tie,{sort}).map(r=>r.id),['a','b']);
 assert.equal(browser.search(rows,{sort:'title-asc'})[0].id,'source-editions');
 assert.equal(browser.search(rows,{sort:'title-desc'})[0].id,'source-reading');
});

test('matchingAuthors combines title/alias matches with actual matching book categories',()=>{
 const rows=browser.records(LIB,coverage,categories);
 assert.deepEqual(browser.matchingAuthors(LIB,rows,'stock market wizard','rules').map(g=>g.id),['minervini']);
 assert.deepEqual(browser.matchingAuthors(LIB,rows,'prestine','psychology'),[]);
 assert.deepEqual(browser.matchingAuthors(LIB,rows,'prestine','rules').map(g=>g.id),['pristine']);
 assert.deepEqual(browser.matchingAuthors(LIB,rows,'','psychology').map(g=>g.id),['minervini']);
 assert.equal(browser.matchingAuthors(LIB,rows,'','').length,3);
});

test('weak and untyped catalogue candidates stay visible, preserving every source author association',()=>{
 const duplicated={authors:{one:{id:'one',name:'One',source_titles:[{id:'same',title:'First listing'}]},two:{id:'two',name:'Two',source_titles:[{id:'same',title:'Second listing',catalogue_candidates:[{id:'p_tpm1'}]}]}}};
 const rows=browser.records(LIB,duplicated,(id,item,author,kind)=>kind==='catalogue'&&id==='p_tpm1'?{categories:['money'],categoryCertainty:'reviewed_category'}:undefined);
 const source=rows.find(r=>r.id==='same');assert.ok(source);assert.equal(rows.filter(r=>r.id==='same').length,1);
 assert.equal(source.authorId,'one');assert.equal(source.authorName,'One');
 assert.deepEqual(source.authorIds,['one','two']);assert.deepEqual(source.authorNames,['One','Two']);
 assert.equal(source.sources.length,2);assert.equal(browser.search(rows,{query:'Second listing'})[0].id,'same');
 assert.deepEqual(rows.find(r=>r.id==='p_tpm1').categories,['money-management']);
 assert.equal(rows.find(r=>r.id==='p_tpm1').categoryCertainty,'reviewed_category');
});

test('title metadata and prefix candidates never merge distinct editions or source IDs',()=>{
 const sources={authors:{editions:{id:'editions',name:'Edition credits',source_titles:[
  {id:'exact',title:'Trade Like a Stock Market Wizard',catalogue_candidates:[{id:'mm_trade',match:'exact_title_metadata'}]},
  {id:'prefix',title:'Trade Like a Stock Market Wizard revised',catalogue_candidates:[{id:'mm_trade',match:'catalogue_title_prefix_candidate_not_identity_proof'}]},
  {id:'string',title:'Trade Like a Stock Market Wizard',catalogue_candidates:['mm_trade']},
  {id:'unknown',title:'Known SHA association to missing catalogue book',catalogue_candidates:[{id:'absent',match:'curated_reading_original_sha256_identity_verified_not_strategy_certification'}]}
 ]}}};
 const rows=browser.records(LIB,sources,{});
 assert.deepEqual(rows.filter(r=>r.kind==='source').map(r=>r.id),['exact','prefix','string','unknown']);
});

test('explicit source IDs merge only the linked source and retain its searchable original metadata',()=>{
 const rows=browser.records(LIB,coverage,categories),book=rows.find(r=>r.id==='mm_trade');
 assert.ok(!rows.some(r=>r.id==='source-bound'));
 assert.deepEqual(book.sourceIds,['source-bound']);assert.equal(book.sources[0].title,'Wizard scan');
 assert.equal(book.sources[0].files[0].filename,'wizard.pdf');
 assert.equal(browser.search(rows,{query:'Wizard scan'})[0].id,'mm_trade');
 assert.equal(browser.search(rows,{query:'wizard.pdf'})[0].id,'mm_trade');
});

test('strong reviewed identity merges a shared source globally and keeps every credit and original',()=>{
 const strong='curated_reading_original_sha256_identity_verified_not_strategy_certification';
 const authors=[...LIB,{id:'one',name:'One credit',books:[]},{id:'two',name:'Two credit',aliases:['Second alias'],books:[]}];
 const shared={authors:{one:{id:'one',name:'One credit',source_titles:[{id:'shared',title:'Original first listing',files:[{filename:'first.pdf'}]}]},two:{id:'two',name:'Two credit',source_titles:[{id:'shared',title:'Verified second listing',catalogue_candidates:[{id:'p_tpm1',match:strong}],files:[{filename:'second.pdf'}]}]}}};
 const before=JSON.stringify({authors,shared}),rows=browser.records(authors,shared,{p_tpm1:'money_management'}),book=rows.find(r=>r.id==='p_tpm1');
 assert.ok(!rows.some(r=>r.id==='shared'));assert.deepEqual(book.sourceIds,['shared']);assert.equal(book.sources.length,2);
 assert.deepEqual(book.authorIds,['pristine','one','two']);assert.deepEqual(book.authorNames,['Pristine Trading — Greg Capra & Oliver Velez','One credit','Two credit']);
 assert.equal(book.authorId,'pristine');assert.equal(book.authorName,'Pristine Trading — Greg Capra & Oliver Velez');
 for(const query of ['Original first listing','Verified second listing','first.pdf','second.pdf','Two credit','Second alias'])assert.equal(browser.search(rows,{query})[0].id,'p_tpm1');
 assert.deepEqual(browser.matchingAuthors(authors,rows,'second.pdf','money').map(a=>a.id),['pristine','one','two']);
 assert.equal(JSON.stringify({authors,shared}),before);
});

test('matchingAuthors finds shared reading sources for all associated authors with category filtering',()=>{
 const authors=[{id:'a',name:'Alice Author',books:[]},{id:'b',name:'Bob Author',books:[]}];
 const shared={authors:{a:{id:'a',source_titles:[{id:'joint',title:'Shared study'}]},b:{id:'b',source_titles:[{id:'joint',title:'Shared study'}]}}};
 const rows=browser.records(authors,shared,{joint:'psychology'});
 assert.equal(rows.length,1);assert.deepEqual(rows[0].authorIds,['a','b']);
 assert.deepEqual(browser.matchingAuthors(authors,rows,'Shared study','psychology').map(a=>a.id),['a','b']);
 assert.equal(browser.search(rows,{query:'Bob Author'})[0].id,'joint');
 assert.deepEqual(browser.matchingAuthors(authors,rows,'Shared study','rules'),[]);
});

test('browser bundle exposes the same pure API without requiring DOM or network',async()=>{
 const fs=await import('node:fs'),vm=await import('node:vm');const context={window:{}};
 vm.runInNewContext(fs.readFileSync(new URL('../../frontend/library-browser.js',import.meta.url),'utf8'),context);
 assert.deepEqual(Object.keys(context.window.VaultLibraryBrowser).sort(),['matchingAuthors','records','search']);
 assert.equal(context.window.VaultLibraryBrowser.records([],{},{}).length,0);
});
