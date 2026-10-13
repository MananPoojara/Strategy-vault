/* Reproduce the deployed reading count pipeline without network, DOM, source payloads or auth secrets. */
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import vm from 'node:vm';
import crypto from 'node:crypto';
import {fileURLToPath} from 'node:url';
import {createRequire} from 'node:module';
const require=createRequire(import.meta.url),root=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'../../../..');
const frontend=path.join(root,'Context/guru/frontend'),output=path.join(root,'library/source-vault/reconciliation-2026-10-13');
const html=fs.readFileSync(path.join(frontend,'full-variant1.html'),'utf8');
const names=['book-coverage.json','pristine-reading-index.json','book-file-index.json','book-categories.json','research-share-library.json','tharp-source-review.json'];
const sourceBytes=Object.fromEntries(names.map(name=>[name,fs.readFileSync(path.join(frontend,'data',name))]));
const data=Object.fromEntries(names.map(name=>[name,JSON.parse(sourceBytes[name])]));
const context={window:{},VaultLibraryBrowser:require(path.join(frontend,'library-browser.js')),data};vm.createContext(context);
vm.runInContext(fs.readFileSync(path.join(frontend,'joe-ross-evidence.js'),'utf8'),context);
const start=html.indexOf('const LIB = ['),end=html.indexOf('const canonicalCategory=',start);
assert.ok(start>=0&&end>start,'Inspected catalogue/count function region missing');
vm.runInContext(html.slice(start,end),context);
const bookSourcesStart=html.indexOf('function bookSources('),bookSourcesEnd=html.indexOf('\nfunction readingSummary(',bookSourcesStart);
assert.ok(bookSourcesStart>=0&&bookSourcesEnd>bookSourcesStart);
vm.runInContext(html.slice(bookSourcesStart,bookSourcesEnd),context);
vm.runInContext('const GURU=Object.fromEntries(LIB.map(g=>[g.id,g]));',context);
vm.runInContext(`
BOOK_COVERAGE=data['book-coverage.json'];
const pristine=data['pristine-reading-index.json'],entries=(pristine.entries||[]).map(b=>({...b,author_metadata:b.source_author,years_reported:b.year?[String(b.year)]:[]}));
BOOK_COVERAGE.authors.pristine={id:'pristine',name:'Pristine Trading',source_titles:entries,catalogue_books:GURU.pristine.books.map(b=>({id:b.id,title:b.title,summary:b.blurb})),counts:{catalogue_books:3,source_title_candidates:entries.length,local_pdf_files:pristine.counts.source_locations}};
BOOK_FILE_INDEX=data['book-file-index.json'];BOOK_CATEGORIES=data['book-categories.json'];
mergeResearchLibrary(data['research-share-library.json']);applyReadingReviews(data['tharp-source-review.json']);finalizeReadingIndex();
const localReaderIds=sources=>{
 const paths=new Set(sources.flatMap(b=>[...(b.local_copies||[]).map(x=>x.path),...(b.files||[]).flatMap(f=>[...(f.local_candidates||[]),...(f.verified_local_copy?.path?[f.verified_local_copy.path]:[])]),...(b.verified_local_copy?.path?[b.verified_local_copy.path]:[])]));
 return new Set([...paths].map(p=>BOOK_FILE_INDEX.files[p.replace(/^Books\\//,'')]).filter(Boolean).map(f=>f.id));
};
globalThis.rows=LIB.map(g=>{
 const entries=authorLibraryRecords(g.id),counts=libraryEntryCounts(g.id),readerFiles=new Set(),locators=new Set();
 let files=0,available=0,catalogueAvailable=0,readingAvailable=0;
 for(const entry of entries){
  const sources=entry.kind==='catalogue'?bookSources(g,entry.book):[entry.source],ids=localReaderIds(sources);
  if(ids.size){available++;if(entry.kind==='catalogue')catalogueAvailable++;else readingAvailable++;}
  for(const id of ids)readerFiles.add(id);
  for(const source of sources)for(const f of source.files||[]){files++;const locator=f.source_relative||f.source_path||f.record_id;if(locator)locators.add(locator);}
 }
 return {group_id:g.id,group_name:g.name,group_kind:g.source_group?'recorded_source_credit_group':'curated_author_or_editorial_group',library_entries:counts.total,catalogue_entries:counts.catalogue,reading_source_entries:counts.reading,
  source_file_positions:files,unique_recorded_source_file_locators:locators.size,entries_with_admitted_local_reader:available,catalogue_entries_with_admitted_local_reader:catalogueAvailable,reading_source_entries_with_admitted_local_reader:readingAvailable,
  unique_admitted_local_reader_files:readerFiles.size,entries_without_admitted_local_reader:counts.total-available,independently_verified_distinct_books:'',complete_book_conversions:'',
  count_warning:'Library entries include catalogue cards and reading-source records. Articles, manuals, chapters, versions and filename-only records do not establish distinct authored books, individual authors or complete conversions. File positions can repeat across entries. Reader availability is admitted-cache metadata; uncopied Research files have no live streaming route.'};
});`,context,{timeout:15000});
const rows=context.rows;
assert.equal(rows.length,4745);assert.equal(rows.reduce((n,r)=>n+r.library_entries,0),35712);
assert.equal(rows.reduce((n,r)=>n+r.catalogue_entries,0),373);assert.equal(rows.reduce((n,r)=>n+r.reading_source_entries,0),35339);
for(const row of rows){assert.equal(row.library_entries,row.catalogue_entries+row.reading_source_entries);assert.equal(row.entries_with_admitted_local_reader,row.catalogue_entries_with_admitted_local_reader+row.reading_source_entries_with_admitted_local_reader);assert.ok(row.entries_without_admitted_local_reader>=0);}
const safeText=value=>typeof value==='string'&&/^[\s\uFEFF]*[=+\-@]/.test(value)?"'"+value:value;
assert.equal(safeText(' =HYPERLINK("example")'),"' =HYPERLINK(\"example\")");assert.equal(safeText('@recorded credit'),"'@recorded credit");assert.equal(safeText('Van Tharp'),'Van Tharp');assert.equal(safeText(0),0);
const cell=value=>{const text=String(safeText(value)??'');return /[",\r\n]/.test(text)?'"'+text.replaceAll('"','""')+'"':text;};
const fields=Object.keys(rows[0]),csv=[fields.map(cell).join(','),...rows.map(row=>fields.map(field=>cell(row[field])).join(','))].join('\r\n')+'\r\n';
const target=path.join(output,'platform-reading-coverage.csv');fs.writeFileSync(target,csv);
const summary={as_of:'2026-10-13',method:'Pure trusted catalogue and actual deployed count functions, exact load/review/finalize sequence; no browser/network/source payload reads',group_rows:rows.length,library_entries:35712,catalogue_entries:373,reading_source_entries:35339,
 source_file_positions:rows.reduce((n,r)=>n+r.source_file_positions,0),entries_with_admitted_local_reader:rows.reduce((n,r)=>n+r.entries_with_admitted_local_reader,0),
 csv_sha256:crypto.createHash('sha256').update(csv).digest('hex'),csv_bytes:Buffer.byteLength(csv),
 input_sha256:{'full-variant1.html':crypto.createHash('sha256').update(html).digest('hex'),...Object.fromEntries(names.map(name=>[name,crypto.createHash('sha256').update(sourceBytes[name]).digest('hex')]))},
 tharp:rows.find(r=>r.group_id==='tharp'),limits:'Counts match visible deduplicated library entries. Book identity/completeness/conversion counts are unestablished and blank. Source-file positions may repeat across entries; reader counts use admitted file lookup, not a fresh HTTP/content probe.'};
fs.writeFileSync(path.join(output,'platform-reading-coverage-validation.json'),JSON.stringify(summary,null,2)+'\n');
console.log(JSON.stringify({group_rows:summary.group_rows,library_entries:summary.library_entries,catalogue_entries:summary.catalogue_entries,reading_source_entries:summary.reading_source_entries,source_file_positions:summary.source_file_positions,entries_with_admitted_local_reader:summary.entries_with_admitted_local_reader,csv_sha256:summary.csv_sha256,csv_bytes:summary.csv_bytes,tharp:summary.tharp},null,2));
