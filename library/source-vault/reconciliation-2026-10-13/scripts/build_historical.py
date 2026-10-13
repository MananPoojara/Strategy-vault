#!/usr/bin/env python3
"""Local metadata reconciliation only. No web, source-content reads, copies or source writes."""
import collections
import csv
import hashlib
import importlib.util
import json
from pathlib import Path
import sys

ROOT=Path(__file__).resolve().parents[4]
OUT=ROOT/'library/source-vault/reconciliation-2026-10-13'
VAULT='/run/user/1000/gvfs/smb-share:server=192.168.4.50,share=research'
spec=importlib.util.spec_from_file_location('recorded_inventory',ROOT/'library/source-vault/scripts/coverage_inventory.py')
inventory=importlib.util.module_from_spec(spec);spec.loader.exec_module(inventory)
DOCUMENTS=inventory.DOCUMENTS

def exclusion(relative):
    if not relative:return 'invalid_or_outside_research_locator'
    parts=[p.casefold() for p in Path(relative).parts]
    excluded={'#recycle','system volume information','us market database','software','meta trader','ninja tarder 6.5','ninja trader 7','multi charts'}
    for p in parts:
        if p in excluded:return 'excluded_nonbibliographic_area:'+p
    return None

def proof_valid(row):
    def sha_valid(value):return isinstance(value,str) and len(value)==64 and all(c in '0123456789abcdef' for c in value)
    def unchanged_times(before,after):return isinstance(before,int) and not isinstance(before,bool) and before>=0 and before==after and isinstance(after,int) and not isinstance(after,bool)
    if row.get('status') not in {'copied','reused_local_identical_sha256'} or row.get('byte_identity_verified') is not True:return False
    sha=row.get('sha256')
    if not sha_valid(sha):return False
    if row.get('source_sha256_unchanged') is not True or row.get('source_mtime_unchanged') is not True:return False
    if row.get('source_kind')=='archive_member':
        before=row.get('archive_sha256_before');after=row.get('archive_sha256_after')
        if row.get('source_member_sha256')!=sha or not sha_valid(before) or before!=after:return False
        if not unchanged_times(row.get('archive_mtime_before_ns',row.get('source_mtime_before_ns')),row.get('archive_mtime_after_ns',row.get('source_mtime_after_ns'))):return False
        if 'archive_volumes' in row:
            volumes=row['archive_volumes']
            if not isinstance(volumes,list) or not volumes:return False
            for volume in volumes:
                if not isinstance(volume,dict) or not sha_valid(volume.get('sha256_before')) or volume.get('sha256_before')!=volume.get('sha256_after') or not unchanged_times(volume.get('mtime_before_ns'),volume.get('mtime_after_ns')):return False
        return True
    return row.get('source_hash_after')==sha and unchanged_times(row.get('source_mtime_before_ns'),row.get('source_mtime_after_ns'))

def main():
    raw,audit=inventory.parse_inventory(ROOT/'library/research-drive-books.tsv')
    candidates=inventory.group_records(raw)
    coverage=json.loads((ROOT/'library/source-vault/coverage-2026-10-13.json').read_text())
    author_id_by_norm={inventory.normalize(a['name']):a['id'] for a in coverage['authors']}
    app=json.loads((ROOT/'library/source-vault/book-coverage.json').read_text())
    cache=json.loads(Path('/home/server/.cache/strategy-vault-webapp/book-index.json').read_text())['files']
    cache_by_path={v['path']:dict(v,reader_id=k) for k,v in cache.items()}
    associations=collections.defaultdict(lambda:{'authors':set(),'books':set(),'sources':set()})
    for appid,a in app['authors'].items():
        for source in a.get('source_titles',[]):
            books={b.get('id') if isinstance(b,dict) else b for b in source.get('catalogue_candidates',[])}
            for f in source.get('files',[]):
                target=associations[f.get('record_id')];target['authors'].add(appid);target['sources'].add(source['id']);target['books'].update(books)
    locator_proofs={};record_proofs={}
    for path in [ROOT/'library/source-vault/batch-import-2026-10-13.jsonl',ROOT/'library/pristine/import-completed-2026-10-13.jsonl']:
        if not path.exists():continue
        for line in path.open():
            if not line.endswith('\n'):continue
            row=json.loads(line)
            if not proof_valid(row):continue
            if row.get('record_id'):record_proofs[row['record_id']]=row
            locator=row.get('source_path') or (VAULT+'/'+row['source_relative'] if row.get('source_relative') else None)
            if locator:locator_proofs[locator]=row
    pristine_path=ROOT/'library/pristine/source-reading-index.json'
    if pristine_path.exists():
        pristine=json.loads(pristine_path.read_text())
        for source in pristine['entries']:
            for f in source['files']:
                for r in raw:
                    if r.get('path_or_archive!member')==f['source_path']:
                        target=associations[r['source_record_id']];target['authors'].add('pristine');target['sources'].add(source['id']);target['books'].update(b['id'] for b in source.get('catalogue_candidates',[]))
    areas=collections.Counter();author_rows={};all_sources=[];count=collections.Counter()
    by_book=coverage.get('strategy_metadata',{}).get('by_book',{})
    with (OUT/'historical-candidates.jsonl').open('w',encoding='utf8') as output:
        for candidate in candidates:
            author=candidate['author'];norm=inventory.normalize(author)
            aid=author_id_by_norm.get(norm) or 'author-'+hashlib.sha256(norm.encode()).hexdigest()[:16]
            records=[]
            for raw_record in candidate['records']:
                locator=raw_record.get('path_or_archive!member','');archive=inventory.archive_locator(locator)
                container=archive[0] if archive else locator
                relative=inventory.safe_direct_relative(container)
                excluded=exclusion(relative)
                ext=inventory.document_extension(locator)
                if ext not in DOCUMENTS:excluded=excluded or 'unsupported_recorded_document_format'
                if inventory.member_filename(locator).startswith('~$'):excluded='office_owner_lock_not_book_body'
                proof=record_proofs.get(raw_record['source_record_id']) or locator_proofs.get(locator)
                linked=associations.get(raw_record['source_record_id'],{'authors':set(),'books':set(),'sources':set()})
                try:size=int(raw_record.get('size',''))
                except (ValueError,TypeError):size=None
                if size is not None and size<=0:excluded=excluded or 'empty_or_invalid_recorded_source_size'
                local=None;cached=None
                if proof:
                    destination=proof.get('destination','')
                    if destination.startswith('Books/'):
                        local={'path':destination,'sha256':proof['sha256'],'bytes':proof['bytes'],'byte_identity_verified':True,'evidence':'completed original-copy log; source before/after hash and mtime equal'}
                        observed=cache_by_path.get(destination[6:])
                        if observed and observed.get('sha256')==proof['sha256'] and observed.get('bytes')==proof['bytes']:cached=observed
                record={'id':raw_record['source_record_id'],'locator':locator,'source_relative':relative,'format':ext,'source_kind':'archive_member' if archive else 'direct_document',
                    'archive_member':archive[1] if archive else None,'recorded_bytes':size,'how_identified':raw_record.get('how_identified'),
                    'metadata_status':raw_record['metadata_status'],'known_hash':proof['sha256'] if proof else None,
                    'copy_status':proof['status'] if local else 'no_completed_original_copy_proof','verified_local_copy':local,
                    'cache_status':'admitted_manifest_exact_hash_match_not_rehashed_in_this_inventory' if cached else 'not_established',
                    'reader_id':cached['reader_id'] if cached else None,'source_readability':'historical_locator_not_freshly_checked',
                    'matched_platform_author_ids':sorted(linked['authors']),'matched_catalogue_book_ids':sorted(b for b in linked['books'] if b),'matched_coverage_source_ids':sorted(linked['sources']),
                    'association_basis':'existing metadata candidate association; not independent bibliographic identity or whole-book conversion certification',
                    'scope_exclusion':excluded}
                if archive and proof:record['archive_container_proof']={k:proof.get(k) for k in ('archive_sha256_before','archive_sha256_after','archive_mtime_before_ns','archive_mtime_after_ns','archive_volumes')}
                records.append(record);all_sources.append(record)
                areas[relative.split('/')[0] if relative else '(invalid locator)']+=1
                count['copied_records']+=bool(local);count['cached_records']+=bool(cached);count['excluded_historical_records']+=bool(excluded)
            ids=sorted(set(b for r in records for b in r['matched_catalogue_book_ids']))
            data={'candidate_id':candidate['id'],'author_id':aid,'author':author,'author_normalized_key':norm,'author_identity':inventory.author_kind(author),
                'title':candidate['title'],'title_normalized_key':inventory.normalize(candidate['title']),'year':candidate['year'],
                'source_count':len(records),'pdf_count':sum(r['format']=='.pdf' for r in records),'source_records':records,
                'duplicate_note':'Repeated locations/copies/chapters/editions are preserved. Only completed matching SHA proofs establish byte identity; candidate count is not a distinct authored-book count.',
                'bibliographic_identity':'unverified_metadata_candidate','whole_book_review_status':'not_established_by_inventory',
                'platform_book_ids_candidate':ids,'platform_metadata_snapshot':{b:by_book[b] for b in ids if b in by_book},
                'platform_status_basis':'cached availability/gate metadata snapshot, not whole-book conversion or verified performance',
                'gap_reasons':(["no_known_platform_metadata_association"] if not any(r['matched_platform_author_ids'] for r in records) else [])+(["no_completed_original_copy_proof"] if not any(r['verified_local_copy'] for r in records) else [])+['independent_bibliographic_identity_and_complete_book_review_not_established']}
            output.write(json.dumps(data,ensure_ascii=False)+'\n')
            row=author_rows.setdefault(aid,{'author_id':aid,'author':author,'normalized_key':norm,'identity':inventory.author_kind(author),'candidate_count':0,'source_records':0,'pdf_records':0,'copied_records':0,'cached_records':0,'platform_associated_candidates':0})
            row['candidate_count']+=1;row['source_records']+=len(records);row['pdf_records']+=data['pdf_count'];row['copied_records']+=sum(bool(r['verified_local_copy']) for r in records);row['cached_records']+=sum(bool(r['reader_id']) for r in records);row['platform_associated_candidates']+=any(r['matched_platform_author_ids'] for r in records)
            count['platform_associated_candidates']+=any(r['matched_platform_author_ids'] for r in records)
    with (OUT/'historical-source-records.jsonl').open('w',encoding='utf8') as file:
        for r in all_sources:file.write(json.dumps(r,ensure_ascii=False)+'\n')
    authors=list(author_rows.values())
    with (OUT/'historical-authors.csv').open('w',encoding='utf8',newline='') as file:
        writer=csv.DictWriter(file,fieldnames=list(authors[0]));writer.writeheader();writer.writerows(authors)
    with (OUT/'historical-areas.csv').open('w',encoding='utf8',newline='') as file:
        writer=csv.writer(file);writer.writerow(['recorded_area','source_records']);writer.writerows(areas.most_common())
    summary={'as_of':'2026-10-13','phase':'historical_metadata_reconciled_live_document_delta_pending','no_web':True,'source_access':'read_only',
        'inventory_audit':audit,'historical_source_records':len(raw),'historical_title_candidates':len(candidates),'recorded_author_credit_groups':len(authors),
        'recorded_identity_groups':dict(collections.Counter(r['identity'] for r in authors)),'app_coverage_groups_before_current_pristine_extension':len(app['authors']),
        'app_coverage_source_candidates_before_current_pristine_extension':app['totals']['distinct_source_title_candidates'],
        'candidate_units_without_known_platform_association':len(candidates)-count['platform_associated_candidates'],**dict(count),
        'scope':'Full Oct10 recorded Research document metadata including archival member locators; fresh selected document-area filesystem verification is separate and pending. Not a verified distinct book/individual author census.',
        'excluded_areas':['#recycle','System Volume Information','US market database','SOFTWARE','META TRADER','NINJA TARDER 6.5','NINJA TRADER 7','MULTI CHARTS'],
        'limits':['No source document payload/hash rescan','No archive extraction/member census in this phase','Known copy/cache hashes reused only from completed original-copy proofs and admitted manifests','No whole-book conversion inferred from rules, previews or catalogue status','Source credit strings may be coauthors/vendors/unknowns; no guessed individual-author splitting']}
    (OUT/'summary.json').write_text(json.dumps(summary,ensure_ascii=False,indent=2)+'\n')
    print(json.dumps(summary,ensure_ascii=False,indent=2))

if __name__=='__main__':main()
