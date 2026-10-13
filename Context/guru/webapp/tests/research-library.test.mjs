import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import vm from 'node:vm';
import {createRequire} from 'node:module';
const require=createRequire(import.meta.url);
const browser=require('../../frontend/library-browser.js');
const page=fs.readFileSync(new URL('../../frontend/full-variant1.html',import.meta.url),'utf8');
const helpers=page.slice(page.indexOf('function mergeResearchLibrary('),page.indexOf('const canonicalCategory='));

function context(){
 const g={id:'tharp',name:'Van Tharp',books:[{id:'tharp',title:'Trade Your Way',sourceIds:['source-book']}]};
 const c={LIB:[g],GURU:{tharp:g},BOOK_COVERAGE:{authors:{tharp:{catalogue_books:[{id:'tharp'}],counts:{local_pdf_files:5}}}},BOOK_CATEGORIES:{books:{},source_titles:{}},SOURCE_REVIEWS:{sources:{},catalogue:{}},SOURCE_ROW_BY_ID:new Map(),LIBRARY_RECORD_CACHE:null,LIBRARY_AUTHOR_CACHE:null,Map};
 c.bookSources=(g,b)=>(c.BOOK_COVERAGE.authors[g.id]?.source_titles||[]).filter(s=>b.sourceIds?.includes(s.id));
 vm.createContext(c);vm.runInContext(helpers,c);return c;
}
test('all-share reading entries retain existing authors and do not become implemented strategies',()=>{
 const c=context();c.mergeResearchLibrary({schema_version:1,authors:{tharp:{name:'Van Tharp',source_titles:[{id:'source-book',title:'Trade Your Way',files:[{filename:'one.pdf'},{filename:'two.pdf'}]},{id:'article',title:'Article || snippet: old extracted text',files:[]}],counts:{source_title_candidates:2}},'research-credit':{name:'Recorded Coauthors',source_titles:[{id:'new',title:'Unreviewed work',files:[]}],counts:{source_title_candidates:1}}}});
 assert.equal(c.LIB.length,2);assert.equal(c.GURU.tharp.books.length,1);assert.equal(c.GURU['research-credit'].books.length,0);
 assert.equal(c.BOOK_COVERAGE.authors.tharp.catalogue_books[0].id,'tharp');assert.equal(c.BOOK_COVERAGE.authors.tharp.source_titles[1].title,'Article');
 c.finalizeReadingIndex();const rows=browser.records(c.LIB,c.BOOK_COVERAGE,{});
 assert.equal(rows.length,3);assert.equal(rows.filter(r=>r.authorId==='tharp').length,2);assert.ok(c.SOURCE_ROW_BY_ID.has('source-book'));
 assert.equal(c.BOOK_COVERAGE.authors.tharp.source_titles[0].files.length,2);
});
test('actual reviewed source credits and categories replace filename hints without performance certification',()=>{
 const c=context();c.mergeResearchLibrary({schema_version:1,authors:{tharp:{source_titles:[{id:'source-book',title:'Old filename',author_metadata:'Van Tharp',files:[]}]}}});
 c.applyReadingReviews({schema_version:1,sources:{'source-book':{summary:'Actual source contents',categories:['psychology'],source_identity:{title:'Actual Book',credit:'Van Tharp & Brian June',year:2001},review:{status_label:'Bounded actual source review',full_book_review:false}}},catalogue:{}});
 const source=c.BOOK_COVERAGE.authors.tharp.source_titles[0];assert.equal(source.title,'Actual Book');assert.equal(source.author_metadata,'Van Tharp & Brian June');
 assert.equal(source.review.full_book_review,false);assert.deepEqual(Array.from(source.category_review.categories),['psychology']);assert.equal(source.verified_performance,undefined);
 assert.equal(c.GURU.tharp.books[0].live,undefined);
});
test('invalid inventory schema or route identities cannot create a research author',()=>{
 const c=context();c.mergeResearchLibrary({schema_version:2,authors:{fake:{source_titles:[]}}});c.mergeResearchLibrary({schema_version:1,authors:{'../fake':{source_titles:[]},good:{source_titles:'not an array'}}});
 assert.equal(c.LIB.length,1);assert.equal(c.GURU.fake,undefined);
});
