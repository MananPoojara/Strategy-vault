#!/usr/bin/env python3
"""Bounded serial metadata-only bibliography census; no document payload/archive reads."""
import collections
import hashlib
import json
import os
from pathlib import Path
import stat
import time

ROOT=Path(__file__).resolve().parents[4]
OUT=ROOT/'library/source-vault/reconciliation-2026-10-13'
VAULT=Path('/run/user/1000/gvfs/smb-share:server=192.168.4.50,share=research')
FORMATS={'.pdf','.doc','.docx','.epub','.djvu','.chm','.azw3'}
ARCHIVES={'.zip','.rar','.7z','.iso','.cab','.tgz','.tar','.gz','.bz2','.xz'}
EXCLUDED={'#recycle','$recycle.bin','system volume information','us market database','global database','database','software','meta trader','metastock','e signal','ninja tarder 6.5','ninja trader 7','multi charts','tradestation','node_modules','.git'}
MAX_ENTRIES=200000
MAX_DIRECTORIES=30000
MAX_DEPTH=22

def main():
    queue=[VAULT];documents=[];archives=[];excluded=[];errors=[];areas=collections.Counter();seen=directories=non_documents=0;start=time.monotonic()
    with (OUT/'fresh-document-files.jsonl').open('w',encoding='utf8') as output:
        while queue:
            folder=queue.pop();relative=str(folder.relative_to(VAULT))
            if len(folder.relative_to(VAULT).parts)>MAX_DEPTH:
                excluded.append({'path':relative,'reason':'explicit_depth_bound'});continue
            try:
                with os.scandir(folder) as iterator:entries=sorted(iterator,key=lambda e:e.name.casefold())
            except OSError as error:
                errors.append({'path':relative,'error':type(error).__name__+': '+str(error)});continue
            directories+=1
            if directories>MAX_DIRECTORIES:
                errors.append({'path':relative,'error':'explicit directory bound exceeded'});break
            for entry in entries:
                seen+=1
                if seen>MAX_ENTRIES:
                    errors.append({'path':relative,'error':'explicit entry bound exceeded'});queue.clear();break
                path=folder/entry.name;rel=str(path.relative_to(VAULT))
                try:
                    if entry.is_symlink():excluded.append({'path':rel,'reason':'symlink_not_followed'});continue
                    if entry.is_dir(follow_symlinks=False):
                        if entry.name.casefold() in EXCLUDED:excluded.append({'path':rel,'reason':'nonbibliographic_payload_subtree_not_traversed'});continue
                        queue.append(path);continue
                    extension=path.suffix.lower()
                    if extension not in FORMATS|ARCHIVES:non_documents+=1;continue
                    info=entry.stat(follow_symlinks=False)
                    if not stat.S_ISREG(info.st_mode):excluded.append({'path':rel,'reason':'not_a_regular_file'});continue
                    if path.name.startswith('~$'):excluded.append({'path':rel,'reason':'office_owner_lock_not_book_body'});continue
                    if info.st_size<=0:excluded.append({'path':rel,'reason':'empty_source_file'});continue
                    row={'path':rel,'filename':path.name,'bytes':info.st_size,'mtime_ns':info.st_mtime_ns,'format':extension[1:],
                        'source_root':str(VAULT),'source_readability':'present_regular_file_metadata_only_content_and_mount_access_not_verified','source_hash':None}
                    if extension in ARCHIVES:
                        archives.append(row);continue
                    key=hashlib.sha256(('research:'+rel).encode()).hexdigest()[:32];row['research_reader_id']=key;row['reader_status']='not_live_research_mount_unavailable_copy_pending'
                    documents.append(row);output.write(json.dumps(row,ensure_ascii=False)+'\n');areas[rel.split('/')[0]]+=1
                except OSError as error:
                    errors.append({'path':rel,'error':type(error).__name__+': '+str(error)})
            if directories%50==0:
                output.flush();print(json.dumps({'directories':directories,'entries_seen':seen,'documents':len(documents),'archive_containers':len(archives),'excluded_subtrees':len(excluded),'elapsed_seconds':round(time.monotonic()-start,1)}),flush=True)
    complete=not queue and not errors and not any(row['reason']=='explicit_depth_bound' for row in excluded)
    summary={'as_of':'2026-10-13','no_web':True,'source_access':'metadata_only_read_only','scope_status':'scoped_walk_complete' if complete else 'scoped_walk_partial',
        'source_root':str(VAULT),'directories_listed':directories,'entries_seen':seen,'direct_documents':len(documents),
        'direct_document_bytes':sum(row['bytes'] for row in documents),'direct_format_counts':dict(collections.Counter(r['format'] for r in documents)),
        'archive_containers':len(archives),'archive_container_bytes':sum(r['bytes'] for r in archives),'non_document_filenames_seen_without_payload_reads':non_documents,
        'document_counts_by_top_area':dict(areas),'excluded':excluded,'errors':errors,'max_entries':MAX_ENTRIES,'max_directories':MAX_DIRECTORIES,'max_depth':MAX_DEPTH,
        'limits':['No source document content/signature reads','No recursive archive-member listing','Software/marketdata/recycle subtrees explicitly excluded','Direct-reader IDs are safe inventory identifiers only: Research Docker bind failed, no streaming link is live','Source-file counts are not distinct authored-book counts'],
        'elapsed_seconds':round(time.monotonic()-start,2)}
    (OUT/'fresh-scope.json').write_text(json.dumps(summary,ensure_ascii=False,indent=2)+'\n')
    (OUT/'fresh-archive-containers.json').write_text(json.dumps({'as_of':'2026-10-13','archive_member_readability':'not_checked','files':archives},ensure_ascii=False,indent=2)+'\n')
    private=Path('/home/server/.cache/strategy-vault/reconciliation-2026-10-13');private.mkdir(parents=True,exist_ok=True)
    manifest={'version':1,'source_root':str(VAULT),'reader_status':'inactive_research_bind_unavailable','hashes_certified':False,'files':{r['research_reader_id']:{k:r[k] for k in ('path','filename','bytes','mtime_ns','format')} for r in documents}}
    assert len(manifest['files'])==len(documents),'Opaque path-ID collision'
    (private/'research-direct-file-manifest.json').write_text(json.dumps(manifest,ensure_ascii=False,indent=2)+'\n');(private/'research-direct-file-manifest.json').chmod(0o600)
    print(json.dumps({k:v for k,v in summary.items() if k not in ('excluded','errors')},ensure_ascii=False,indent=2),flush=True)

if __name__=='__main__':main()
