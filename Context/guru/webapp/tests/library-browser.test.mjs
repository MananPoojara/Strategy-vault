import test from 'node:test';
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
const require=createRequire(import.meta.url);
const browser=require('../../frontend/library-browser.js');
const LIB=[
 {id:'minervini',name:'Mark Minervini',books:[{id:'mm_trade',title:'Trade Like a Stock Market Wizard',year:2013},{id:'mm_think',title:'Think & Trade Like a Champion',year:2017}]},
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

test('category callback and global candidate suppression keep source relationships conservative',()=>{
 const duplicated={authors:{one:{id:'one',name:'One',source_titles:[{id:'same',title:'First listing'}]},two:{id:'two',name:'Two',source_titles:[{id:'same',title:'Second listing',catalogue_candidates:[{id:'p_tpm1'}]}]}}};
 const rows=browser.records(LIB,duplicated,(id,item,author,kind)=>kind==='catalogue'&&id==='p_tpm1'?{categories:['money'],categoryCertainty:'reviewed_category'}:undefined);
 assert.ok(!rows.some(r=>r.id==='same'));assert.deepEqual(rows.find(r=>r.id==='p_tpm1').categories,['money-management']);
 assert.equal(rows.find(r=>r.id==='p_tpm1').categoryCertainty,'reviewed_category');
});

test('browser bundle exposes the same pure API without requiring DOM or network',async()=>{
 const fs=await import('node:fs'),vm=await import('node:vm');const context={window:{}};
 vm.runInNewContext(fs.readFileSync(new URL('../../frontend/library-browser.js',import.meta.url),'utf8'),context);
 assert.deepEqual(Object.keys(context.window.VaultLibraryBrowser).sort(),['matchingAuthors','records','search']);
 assert.equal(context.window.VaultLibraryBrowser.records([],{},{}).length,0);
});
