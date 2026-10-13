#!/usr/bin/env python3
"""Publish per-group CSV and precise distinct-file/title residuals, never source writes."""
import collections
import csv
import json
from pathlib import Path
from coverage_inventory import PROJECT


def residuals(coverage):
    titles={t['id']:t for a in coverage['authors'].values() for t in a['source_titles']}
    files={f['source_relative']:f for t in titles.values() for f in t['files']}
    excluded={r['record_id']:r for a in coverage['authors'].values() for r in a.get('excluded_source_records',[])}
    pending=[]
    for f in files.values():
        if f.get('verified_local_copy',{}).get('byte_identity_verified'): continue
        rec=dict(f)
        rec['reason']=f.get('import_reason') or ('No verified original extraction has been recorded for this exact archive member.'
                                              if f.get('archive_member') else 'No verified import proof has been recorded for this exact original.')
        rec['local_copy_status']='filename_size_candidate_only' if f.get('local_candidates') else 'no_local_candidate'
        pending.append(rec)
    unverified=[{'id':t['id'],'title':t['title'],'author_metadata':t['author_metadata'],
                 'files':t['files'],'local_copies':t.get('local_copies',[])} for t in titles.values()
                if not any(c.get('byte_identity_verified') for c in t.get('local_copies',[]))]
    return {'distinct_source_title_candidates':len(titles),
            'source_titles_with_verified_local_copy':len(titles)-len(unverified),
            'source_title_candidates_without_verified_local_copy':len(unverified),
            'related_material_candidate_groups':sum(t.get('book_count_eligible') is False for t in titles.values()),
            'distinct_book_candidate_source_units':sum(t.get('book_count_eligible') is not False for t in titles.values()),
            'distinct_source_file_locations':len(files),
            'unverified_source_file_locations':len(pending),
            'excluded_nonbook_source_records':len(excluded),
            'pending_format_counts':dict(collections.Counter(Path(f['filename']).suffix.lower() for f in pending)),
            'candidates':unverified,'pending_files':pending,'excluded_records':list(excluded.values())}


def run(project=PROJECT,date='2026-10-13'):
    root=Path(project)/'library/source-vault'; coverage=json.loads((root/'book-coverage.json').read_text())
    result=residuals(coverage); result.update({'as_of':date,'source_access':'read_only',
        'scope':'Current 70 app author/editorial groups; historical source mapping only, not all global vault books.',
        'count_warning':'Candidates/folders can contain chapters, duplicate editions, articles and support documents. File equality does not certify authorship, full reading, rule fidelity or performance.'})
    (root/f'remaining-source-candidates-{date}.json').write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n')
    columns=['author_id','author_name','catalogue_cards','source_title_candidates','source_file_records',
             'local_document_files','local_pdf_files','source_titles_with_verified_local_copies',
             'books_with_live_strategy_metadata','books_with_data_missing','books_not_testable',
             'books_without_strategy_metadata','independently_verified_books','reviewed_source_copy_identities',
             'related_material_candidates','book_candidate_source_units','count_note']
    with (root/f'author-coverage-{date}.csv').open('w',newline='') as out:
        writer=csv.DictWriter(out,fieldnames=columns); writer.writeheader()
        for a in coverage['authors'].values():
            c=a['counts']; row={k:c.get(k,0) for k in columns}
            row.update(author_id=a['id'],author_name=a['name'],catalogue_cards=c['catalogue_books'],
                       independently_verified_books='not_established',
                       count_note='Source-title/folder candidates and file locations are not distinct verified books; groups overlap. Live strategy metadata is not independently verified book coverage.')
            writer.writerow(row)
    local=json.loads((root/f'local-files-{date}.json').read_text())['files']
    rows=list(map(json.loads,(root/f'batch-import-{date}.jsonl').read_text().splitlines()))
    latest={r['source_relative']:r for r in rows}
    summary={k:v for k,v in result.items() if k not in {'candidates','pending_files','excluded_records'}}
    summary.update(local_document_files=len(local),local_pdf_files=sum(f['extension']=='.pdf' for f in local),
                   import_unique_source_paths=len(latest),import_statuses=dict(collections.Counter(r['status'] for r in latest.values())),
                   copied_bytes=sum(r.get('bytes',0) for r in rows if r['status']=='copied'),
                   source_originals_byte_proof_only=True)
    (root/f'final-copy-summary-{date}.json').write_text(json.dumps(summary,indent=2)+'\n')
    print(json.dumps(summary,indent=2))


if __name__=='__main__': run()
