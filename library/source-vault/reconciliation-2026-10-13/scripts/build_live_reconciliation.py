#!/usr/bin/env python3
"""Join fresh filesystem metadata to recorded bibliography. Never read source payloads or publish app files."""
import collections
import copy
import hashlib
import importlib.util
import json
from pathlib import Path, PurePosixPath
import sys
import os
import shutil

ROOT=Path(__file__).resolve().parents[4];OUT=ROOT/'library/source-vault/reconciliation-2026-10-13'
spec=importlib.util.spec_from_file_location('historical_reconciliation',OUT/'scripts/build_historical.py');h=importlib.util.module_from_spec(spec);spec.loader.exec_module(h)

def load_lines(path):
    with path.open(encoding='utf8') as source:
        for line in source:
            if line.strip():yield json.loads(line)

def source_relative(locator):
    parsed=h.inventory.archive_locator(locator)
    return h.inventory.safe_direct_relative(parsed[0] if parsed else locator)

def compact_file(record,live):
    relative=record.get('source_relative')
    result={'record_id':record['id'],'filename':h.inventory.member_filename(record['locator']),'source_relative':relative or record['locator'],
        'format':record['format'],'recorded_bytes':record.get('recorded_bytes'),'source_readability':record['source_readability'],
        'local_candidates':[record['verified_local_copy']['path']] if record.get('verified_local_copy') else [],'metadata_status':record.get('metadata_status')}
    if record.get('source_kind')=='archive_member':result['source_relative']=record['locator'][len(h.VAULT)+1:] if record['locator'].startswith(h.VAULT+'/') else record['locator'];result['archive_member']=True
    if record.get('verified_local_copy'):result['verified_local_copy']=record['verified_local_copy']
    if record.get('reader_id'):result['reader_id']=record['reader_id']
    if live:
        result.update(observed_bytes=live['bytes'],observed_mtime_ns=live['mtime_ns'],research_reader_id=live['research_reader_id'],research_reader_status='inactive_mount_unavailable_copy_pending')
    return result

def main():
    fresh=list(load_lines(OUT/'fresh-document-files.jsonl'));fresh_by_path={r['path']:r for r in fresh}
    provisional='--provisional' in sys.argv
    archive_data=json.loads((OUT/'fresh-archive-containers.json').read_text()) if (OUT/'fresh-archive-containers.json').exists() else {'files':[]}
    archives={r['path']:r for r in archive_data['files']}
    scope=json.loads((OUT/'fresh-scope.json').read_text()) if (OUT/'fresh-scope.json').exists() else {'scope_status':'live_metadata_recovery_in_progress','errors':[],'excluded':[],'limits':['Historical source locators retained; fresh metadata completion pending','No source payload/hash rescan or archive-member listing','No Research streaming route is live']}
    if provisional:scope['scope_status']='provisional_historical_plus_recovered_live_metadata' 
    if scope['errors'] or any(e['reason']=='explicit_depth_bound' for e in scope['excluded']):scope['scope_status']='scoped_walk_partial'
    raw=list(load_lines(OUT/'historical-candidates.jsonl'))
    current_app=json.loads((ROOT/'library/source-vault/book-coverage.json').read_text())
    base_authors=copy.deepcopy(current_app['authors'])
    # Pristine source units were verified separately after the historical app subset was generated.
    pristine=json.loads((ROOT/'library/pristine/source-reading-index.json').read_text())
    pristine_author=base_authors.setdefault('pristine',{'id':'pristine','name':'Pristine Trading — Greg Capra & Oliver Velez','source_titles':[],'counts':{},'group_kind':'app_editorial_group'})
    for source in pristine['entries']:
        pristine_author['source_titles'].append({'id':source['id'],'title':source['title'],'author_metadata':source['source_author'],
            'years_reported':[source['year']] if source.get('year') else [],'catalogue_candidates':source.get('catalogue_candidates',[]),
            'files':[{'record_id':None,'filename':f['filename'],'source_relative':f['source_path'][len(h.VAULT)+1:],'local_candidates':[f['verified_local_copy']['path']],
                      'verified_local_copy':f['verified_local_copy'],'reader_id':f.get('reader_id')} for f in source['files']],
            'local_status':'verified_local_copy','summary':source['summary'],'summary_status':'source_identity_checked_content_review_pending','count_basis':source['count_basis']})
    # Full locator maps connect recorded rows to existing editorial source units without guessing authorship.
    historical_by_id={r['id']:r for c in raw for r in c['source_records']}
    rows_by_locator={r['locator']:r for r in historical_by_id.values()}
    explicit_material={}
    review=json.loads((ROOT/'library/source-vault/related-material-review-2026-10-13.json').read_text())
    for r in review.get('records',[]):
        if r.get('book_count_eligible') is False:explicit_material[r['source_relative']]=r.get('source_material_kind','related_nonbook_material')
    claimed_paths=set();status_counts=collections.Counter();observed_paths=set();archive_declared=0;archive_records=0
    with (OUT/'reconciled-candidates.jsonl').open('w',encoding='utf8') as output:
        for candidate in raw:
            for r in candidate['source_records']:
                relative=r.get('source_relative');live=fresh_by_path.get(relative) if r['source_kind']=='direct_document' else None
                rawrelative=r['locator'][len(h.VAULT)+1:] if r['locator'].startswith(h.VAULT+'/') else None
                if rawrelative in explicit_material:r['scope_exclusion']='explicit_source_content_review:'+explicit_material[rawrelative]
                if r['scope_exclusion']:
                    r['source_readability']='excluded_from_active_bibliography';r['active_source']=False
                elif live:
                    r['source_readability']=live['source_readability'];r['observed_bytes']=live['bytes'];r['observed_mtime_ns']=live['mtime_ns'];r['research_reader_id']=live['research_reader_id'];r['research_reader_status']='inactive_research_mount_unavailable';r['active_source']=True;observed_paths.add(relative)
                elif r['source_kind']=='archive_member' and relative in archives:
                    member=PurePosixPath(str(r.get('archive_member') or '').replace('\\','/'))
                    if '..' in member.parts or member.is_absolute():r['scope_exclusion']='unsafe_historical_archive_member_locator';r['active_source']=False;r['source_readability']='excluded_from_active_bibliography'
                    else:r['source_readability']='archive_container_present_member_not_freshly_censused';r['archive_container_bytes']=archives[relative]['bytes'];r['active_source']=True
                else:
                    r['source_readability']='historical_locator_not_freshly_verified_yet' if provisional else 'not_found_in_scoped_fresh_document_census';r['active_source']=provisional or bool(r.get('reader_id'))
                    if r.get('reader_id'):r['source_readability']='historical_original_unconfirmed_verified_cached_copy_available'
                status_counts[r['source_readability']]+=1
                if r['source_kind']=='archive_member':archive_records+=1;archive_declared+=max(0,r.get('recorded_bytes') or 0)
            candidate['active_source_records']=sum(r['active_source'] for r in candidate['source_records'])
            candidate['scope_status']='active_reading_source_candidate' if candidate['active_source_records'] else 'historical_excluded_or_unconfirmed_source_only'
            output.write(json.dumps(candidate,ensure_ascii=False)+'\n')
    authors={};used_source_ids=set();base_unit_count=0
    for aid,author in base_authors.items():
        out={'id':aid,'name':author['name'],'group_kind':author.get('group_kind','app_author_or_editorial_group'),
             'author_alias_keys':author.get('author_alias_keys',[]),'source_author_spellings':author.get('source_author_spellings',[]),'source_titles':[],
             'note':'Reading-source metadata and existing editorial associations are not independently verified authored-book identity or complete strategy conversion.'}
        for source in author.get('source_titles',[]):
            files=[]
            for f in source.get('files',[]):
                record=historical_by_id.get(f.get('record_id'))
                relative=f.get('source_relative','')
                locator=h.VAULT+'/'+relative
                record=record or rows_by_locator.get(locator)
                if not record or not record.get('active_source'):continue
                file=compact_file(record,fresh_by_path.get(record['source_relative']) if record['source_kind']=='direct_document' else None)
                files.append(file);claimed_paths.add(record['locator'])
            if not files:continue
            item={k:source[k] for k in ('id','title','author_metadata','years_reported','catalogue_candidates','count_basis','summary','summary_status','bibliographic_identity','source_material_kind') if k in source}
            item['files']=files;item['local_copies']=[f['verified_local_copy'] for f in files if f.get('verified_local_copy')]
            item['local_status']='verified_cached_original_copy_available' if any(f.get('reader_id') for f in files) else 'original_copy_pending'
            item.setdefault('summary_status','awaiting_source_review');item.setdefault('summary',None);item.setdefault('count_basis','existing editorial source unit; files/copies/chapters do not establish distinct authored books')
            out['source_titles'].append(item);used_source_ids.add(item['id']);base_unit_count+=1
        out['counts']={'source_title_candidates':len(out['source_titles']),'source_file_records':sum(len(s['files']) for s in out['source_titles']),'independently_verified_books':None}
        authors[aid]=out
    additional_count=0
    for candidate in raw:
        active=[r for r in candidate['source_records'] if r.get('active_source') and r['locator'] not in claimed_paths]
        if not active or candidate['candidate_id'] in used_source_ids:continue
        # Preserve the full recorded author/coauthor/vendor credit as its own research identity.
        aid='research-'+candidate['author_id'];out=authors.setdefault(aid,{'id':aid,'name':candidate['author'],'group_kind':'recorded_source_credit_not_verified_individual_author',
            'identity':candidate['author_identity'],'source_titles':[],'note':'Recorded author/coauthor/vendor metadata. Identity and complete book review are unverified.'})
        files=[compact_file(r,fresh_by_path.get(r['source_relative']) if r['source_kind']=='direct_document' else None) for r in active]
        out['source_titles'].append({'id':candidate['candidate_id'],'title':candidate['title'],'author_metadata':candidate['author'],
            'years_reported':[candidate['year']] if candidate.get('year') else [],'files':files,'local_copies':[f['verified_local_copy'] for f in files if f.get('verified_local_copy')],
            'catalogue_candidates':[{'id':bid,'match':'historical_metadata_association_not_bibliographic_or_performance_certification'} for bid in candidate.get('platform_book_ids_candidate',[])],
            'count_basis':'normalized recorded author/title/year candidate; articles/manuals/copies/editions are not verified distinct authored books',
            'bibliographic_identity':'unverified_source_unit','local_status':'verified_cached_original_copy_available' if any(f.get('reader_id') for f in files) else 'original_copy_pending',
            'summary_status':'awaiting_source_review','summary':None})
        used_source_ids.add(candidate['candidate_id']);additional_count+=1
    # Newly observed direct files have filename evidence only; no embedded content/title/author read is implied.
    historical_direct_paths={r.get('source_relative') for r in historical_by_id.values() if r['source_kind']=='direct_document'}
    new_files=[dict(f) for f in fresh if f['path'] not in historical_direct_paths]
    for f in new_files:
        parts=PurePosixPath(f['path']).parts
        f['book_area']=parts[0] if parts else None
        f['parent_folder']=str(PurePosixPath(f['path']).parent)
        f['parent_folder_name']=PurePosixPath(f['path']).parent.name
        f['folder_label_claim']=None
        f['folder_author_credit_claim']=None
        f['folder_title_claim']=None
        for position in range(len(parts)-2,0,-1):
            if ' - ' in parts[position]:
                label=parts[position];credit,title=label.split(' - ',1)
                f['folder_label_claim']=label;f['folder_author_credit_claim']=credit.strip();f['folder_title_claim']=title.strip();break
        f['bibliographic_status']='filename_and_folder_label_only_author_title_edition_unverified'
        f['duplicate_note']='No source-body hash was read; identical names/sizes or repeated folders do not establish byte-identical duplicates.'
    with (OUT/'new-live-files.jsonl').open('w',encoding='utf8') as output:
        for f in new_files:output.write(json.dumps(f,ensure_ascii=False)+'\n')
    if new_files:
        aid='research-live-unassigned';authors[aid]={'id':aid,'name':'New source files — attribution awaiting review','identity':'unknown','group_kind':'unassigned_live_file_metadata','source_titles':[]}
        for f in new_files:
            sid='live-file-'+f['research_reader_id'];file={'filename':f['filename'],'source_relative':f['path'],'format':'.'+f['format'],'observed_bytes':f['bytes'],'research_reader_id':f['research_reader_id'],'research_reader_status':'inactive_mount_unavailable_copy_pending','local_candidates':[]}
            authors[aid]['source_titles'].append({'id':sid,'title':f['filename'],'author_metadata':'Attribution awaiting review','years_reported':[],'files':[file],'local_copies':[],
                'aliases':[f['parent_folder_name'],f.get('folder_label_claim') or ''],'catalogue_candidates':[],'count_basis':'newly observed file, not an established authored book','bibliographic_identity':'unreviewed_filename_only','local_status':'original_copy_pending','summary_status':'awaiting_source_review','summary':None})
    for author in authors.values():author['counts']={'source_title_candidates':len(author['source_titles']),'source_file_records':sum(len(s['files']) for s in author['source_titles']),'local_pdf_files':sum(f.get('format')=='.pdf' and bool(f.get('reader_id')) for s in author['source_titles'] for f in s['files']),'independently_verified_books':None}
    ui={'schema_version':1,'as_of':'2026-10-13','source_access':'read_only_metadata','source_reader_status':'inactive_research_mount_unavailable','authors':authors,
        'scope':scope['scope_status'],'counts_are':'source units and recorded author credits, not verified distinct authored books or individual authors'}
    private=Path('/home/server/.cache/strategy-vault/reconciliation-2026-10-13');private.mkdir(parents=True,exist_ok=True)
    payload=private/'ui-library.json';payload.write_text(json.dumps(ui,ensure_ascii=False,separators=(',',':'))+'\n')
    temporary=OUT/'.ui-library-publishing.json';shutil.copyfile(payload,temporary);os.replace(temporary,OUT/'ui-library.json')
    summary={'as_of':'2026-10-13','historical_candidates':len(raw),'fresh_direct_files':len(fresh),'fresh_direct_bytes':sum(f['bytes'] for f in fresh),'fresh_archive_containers':len(archives),
        'fresh_archive_container_bytes':sum(f['bytes'] for f in archives.values()),'matched_historical_direct_paths':len(observed_paths),'historical_direct_paths_not_observed_in_scoped_census':len(historical_direct_paths-set(fresh_by_path)),'historical_archive_member_records':archive_records,'historical_archive_declared_member_bytes':archive_declared,
        'historical_original_source_readability':dict(status_counts),'new_live_direct_files_without_historical_bibliography':len(new_files),
        'ui_author_or_credit_groups':len(authors),'ui_preserved_app_editorial_source_units':base_unit_count,'ui_additional_historical_candidate_units':additional_count,
        'ui_total_source_unit_positions':sum(len(a['source_titles']) for a in authors.values()),'ui_distinct_source_unit_ids':len({s['id'] for a in authors.values() for s in a['source_titles']}),'scope_status':scope['scope_status'],'source_reader_live':False,
        'source_hash_rescan_performed':False,'limits':scope['limits']}
    (OUT/'live-reconciliation-summary.json').write_text(json.dumps(summary,ensure_ascii=False,indent=2)+'\n');print(json.dumps(summary,ensure_ascii=False,indent=2))

if __name__=='__main__':main()
