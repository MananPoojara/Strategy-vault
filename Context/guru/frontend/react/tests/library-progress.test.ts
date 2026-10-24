import {expect,it} from 'vitest';
import {libraryProgress,definitionSignature,reasonLabel} from '../src/services/library-progress';
import {identifiedAuthorCount,authorIdentities} from '../src/services/author-identities';
const defs = [
 {id:'a1',book:'one',family:'rsi',status:'live',instrument_policy:{allowed_types:['individual_stock']}},
 {id:'a2',book:'one',family:'rsi',status:'live',instrument_policy:{allowed_types:['individual_stock']}},
 {id:'b',book:'two',family:'rsi',status:'not_testable',status_note:'Discretionary exit remains undefined.'},
 {id:'c',book:'three',family:'cross',status:'needs_data',status_note:'Actual options quotes are missing.'},
 {id:'d',book:'four',family:'cross',status:'source_review'},
 {id:'e',book:'five',family:'cross',status:'not_testable',instrument_policy:{requirement:'source_review_required',allowed_types:[]}},
];
const row = {variant_id:'a1',book_id:'one',method_id:'one:rsi',result_status:'available',evidence:'Authenticated measured summary + readiness, closed trades > 0'};
const snapshot = {schema_version:1,as_of:'2026-10-20',catalogue_variant_ids:defs.map(v=>v.id),catalogue_rule_signatures:Object.fromEntries(defs.map(v=>[v.id,definitionSignature(v)])),measured_backtests:[row,{...row,variant_id:'a2',instrument:'Another stock'}]};
it('counts each book-specific rule once across variants, markets and instruments',()=>{
 const p=libraryProgress(defs,snapshot);
 expect(p.current).toBe(true);expect(p.strategiesFound).toBe(5);expect(p.backtestedStrategies).toBe(1);
 expect(p.counts(['one','two'])).toEqual({strategies:2,backtested:1});
 expect(p.book('one').status).toBe('backtested');
});
it('keeps only uncoded books in progress; coded books without results are not backtested with reasons',()=>{
 const p=libraryProgress(defs,snapshot);
 expect(p.book('unknown').status).toBe('in_progress');expect(p.book('four').status).toBe('not_backtested');expect(p.book('five').status).toBe('not_backtested');
 expect(p.book('two')).toMatchObject({status:'not_backtested',reason:'Discretionary exit remains undefined.'});
 expect(p.book('three')).toMatchObject({status:'not_backtested',reason:'Actual options quotes are missing.'});
});
it('never counts registration, fabricated IDs, blocked methods or evidence-free rows as completed',()=>{
 const p=libraryProgress(defs,{...snapshot,measured_backtests:[{...row,evidence:''},{...row,book_id:'two'},{...row,variant_id:'b',book_id:'two',method_id:'two:rsi'},{...row,variant_id:'bogus'}]});
 expect(p.backtestedStrategies).toBe(0);expect(p.book('one')).toMatchObject({status:'not_backtested',reason:expect.stringMatching(/Coded, not yet measured/)});
});
it('rejects missing and stale availability snapshots rather than publishing stale completion',()=>{
 for(const input of [null,{}, {...snapshot,catalogue_variant_ids:['a1']},{...snapshot,schema_version:2}]) {
  const p=libraryProgress(defs,input);expect(p.current).toBe(false);expect(p.backtestedStrategies).toBe(0);
 }
});
it('counts explicitly identified people instead of folders or curated profiles',()=>{
 expect(identifiedAuthorCount).toBe(99);expect(authorIdentities.livermore).toContain('Edwin LeFevre');
 expect(identifiedAuthorCount).toBe(new Set(Object.values(authorIdentities).flat()).size);
 expect(authorIdentities.connors).toContain('Cesar Alvarez');expect(authorIdentities.pristine).toContain('Oliver Velez');
 expect(Object.values(authorIdentities).flat()).not.toContain('Pristine Trading');
});

it('rejects changed rule text under unchanged variant IDs and honours explicit source-only review',()=>{
 const changed=defs.map(v=>v.id==='a1'?{...v,rules:['A different entry price']}:v);
 expect(libraryProgress(changed,snapshot).current).toBe(false);
 const p=libraryProgress(defs,{...snapshot,source_reviews:[{source_id:'reading',author_id:'person',progress_status:'NOT BACKTESTED',reason:'Reviewed essay; no executable entry/exit protocol recorded.'}]});
 expect(p.source('reading',['person'])).toMatchObject({status:'not_backtested',reason:'Reviewed essay; no executable entry/exit protocol recorded.'});
 expect(p.source('reading',['other']).status).toBe('in_progress');
});

it('names the plain reason for held rules without changing their status',()=>{
 expect(reasonLabel('Withheld by the accuracy check: more than half of its gain comes from same-day trades')).toBe('Held by the accuracy check');
 expect(reasonLabel('Required market data is missing: OPT_ATM_PUT.Close.')).toBe('Needs option prices');
 expect(reasonLabel('Backtest unavailable: discretionary. No rules or prices are invented to fill this gap.')).toBe('Rules not fully defined');
 expect(reasonLabel('Source review required before publishing this method on any instrument.')).toBeUndefined();
 const p=libraryProgress(defs,snapshot);
 expect(p.book('five')).toMatchObject({status:'not_backtested',reason:'No backtest is published yet for its 1 strategy: Held (1). Each strategy page gives the exact reason.'});
 expect(p.book('one').reason).toMatch(/available computed result/);
});
