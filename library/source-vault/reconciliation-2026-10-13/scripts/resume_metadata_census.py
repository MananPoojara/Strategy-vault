#!/usr/bin/env python3
"""Recover an interrupted metadata report locally and resume only unfinished top areas."""
import collections,csv,hashlib,importlib.util,json,os,shutil,stat,time
from pathlib import Path
ROOT=Path(__file__).resolve().parents[4];OUT=ROOT/'library/source-vault/reconciliation-2026-10-13';PRIVATE=Path('/home/server/.cache/strategy-vault/reconciliation-2026-10-13')
spec=importlib.util.spec_from_file_location('walk_policy',OUT/'scripts/walk_documents.py');policy=importlib.util.module_from_spec(spec);spec.loader.exec_module(policy)
spec2=importlib.util.spec_from_file_location('recorded_policy',OUT/'scripts/build_historical.py');hist=importlib.util.module_from_spec(spec2);spec2.loader.exec_module(hist)
VAULT=policy.VAULT

def atomic_json(path,value):
 temp=path.with_name('.'+path.name+'.writing');temp.write_text(json.dumps(value,ensure_ascii=False,indent=2)+'\n');os.replace(temp,path)

def main():
 PRIVATE.mkdir(parents=True,exist_ok=True)
 checkpoint=PRIVATE/'metadata-checkpoint.json';report_path=PRIVATE/'fresh-document-files.jsonl';start=time.monotonic()
 if checkpoint.exists():
  state=json.load(checkpoint.open());documents={r['path']:r for r in state['documents']};queue=[Path(p) for p in state['queue']];excluded=state['excluded'];errors=state['errors'];directories=state['directories'];seen=state['seen'];archives=state['archives'];base_count=state['base_count']
 else:
  documents={}
  with (OUT/'fresh-document-files.jsonl').open() as old:
   for line in old:
    if line.strip():
     row=json.loads(line);documents[row['path']]=row
  base_count=len(documents);excluded=[];errors=[];directories=seen=0;archives={}
  completed=['Videos','USB HDD 2','USB HDD 1','Trading Knowledge III']
  # Original walker was sorted depth-first with a LIFO stack; its last persisted
  # rows were in Trading Knowledge II. Restart that entire root, plus all roots
  # ordered before it. Already completed larger USB/media roots are not rescanned.
  roots=[]
  for p in sorted(VAULT.iterdir(),key=lambda p:p.name.casefold()):
   if p.name in completed:continue
   if p.name.casefold() in policy.EXCLUDED:
    excluded.append({'path':p.name,'reason':'nonbibliographic_payload_subtree_not_traversed'});continue
   if p.is_dir():roots.append(p)
  # Queue primary named book areas first rather than rerunning large USB trees.
  preferred=['Trading Books','Seasonality Top Reads','Other TradingInvesting books','Larry Connors','gold research paper','options','Prasad','Trading Courses,Seminars,Videos','Trading Knowledge I','Trading Knowledge II','Share']
  queue=[p for name in reversed(preferred) for p in roots if p.name==name]
  queue+=[p for p in roots if p.name not in preferred]
 with report_path.open('w',encoding='utf8') as out:
  for row in documents.values():out.write(json.dumps(row,ensure_ascii=False)+'\n')
  while queue:
   folder=queue.pop();rel=str(folder.relative_to(VAULT))
   if len(folder.relative_to(VAULT).parts)>policy.MAX_DEPTH:excluded.append({'path':rel,'reason':'explicit_depth_bound'});continue
   try:
    with os.scandir(folder) as it:entries=sorted(it,key=lambda e:e.name.casefold())
   except OSError as e:errors.append({'path':rel,'error':str(e)});continue
   directories+=1
   for e in entries:
    seen+=1;path=folder/e.name;relative=str(path.relative_to(VAULT))
    if seen>policy.MAX_ENTRIES or directories>policy.MAX_DIRECTORIES:errors.append({'path':relative,'error':'explicit resume traversal bound'});queue.clear();break
    try:
     if e.is_symlink():excluded.append({'path':relative,'reason':'symlink_not_followed'});continue
     if e.is_dir(follow_symlinks=False):
      if e.name.casefold() in policy.EXCLUDED:excluded.append({'path':relative,'reason':'nonbibliographic_payload_subtree_not_traversed'});continue
      queue.append(path);continue
     ext=path.suffix.lower()
     if ext not in policy.FORMATS|policy.ARCHIVES:continue
     info=e.stat(follow_symlinks=False)
     if not stat.S_ISREG(info.st_mode) or info.st_size<=0 or e.name.startswith('~$'):excluded.append({'path':relative,'reason':'not_regular_nonempty_document_body'});continue
     row={'path':relative,'filename':e.name,'bytes':info.st_size,'mtime_ns':info.st_mtime_ns,'format':ext[1:],'source_root':str(VAULT),'source_readability':'present_regular_file_metadata_only_content_and_mount_access_not_verified','source_hash':None}
     if ext in policy.ARCHIVES:archives[relative]=row;continue
     row['research_reader_id']=hashlib.sha256(('research:'+relative).encode()).hexdigest()[:32];row['reader_status']='not_live_research_mount_unavailable_copy_pending'
     documents[relative]=row;out.write(json.dumps(row,ensure_ascii=False)+'\n')
    except OSError as e:errors.append({'path':relative,'error':str(e)})
   if directories%50==0:
    out.flush();atomic_json(checkpoint,{'documents':list(documents.values()),'queue':[str(p) for p in queue],'excluded':excluded,'errors':errors,'directories':directories,'seen':seen,'archives':archives,'base_count':base_count})
    print(json.dumps({'resumed_directories':directories,'resumed_entries':seen,'unique_direct_documents':len(documents),'current_area':str(folder.relative_to(VAULT)).split('/')[0],'elapsed_seconds':round(time.monotonic()-start,1)}),flush=True)
 # Check the historical archive containers directly, without reading/listing archive bodies.
 historical_containers=set()
 rows,_=hist.inventory.parse_inventory(ROOT/'library/research-drive-books.tsv')
 for r in rows:
  parsed=hist.inventory.archive_locator(r.get('path_or_archive!member'))
  if parsed:
   relative=hist.inventory.safe_direct_relative(parsed[0])
   if relative and not hist.exclusion(relative):historical_containers.add(relative)
 for relative in sorted(historical_containers):
  if relative in archives:continue
  path=VAULT/relative
  try:
   parent=VAULT
   for part in Path(relative).parts:
    parent=parent/part;s=parent.lstat()
    if stat.S_ISLNK(s.st_mode):raise ValueError('symlink archive-container path')
   if not stat.S_ISREG(s.st_mode):raise ValueError('archive container is not a regular file')
   archives[relative]={'path':relative,'filename':path.name,'bytes':s.st_size,'mtime_ns':s.st_mtime_ns,'format':path.suffix.lower()[1:],'source_root':str(VAULT),'source_readability':'historical_container_path_metadata_present_member_not_checked','source_hash':None}
  except (OSError,ValueError) as e:errors.append({'path':relative,'error':'historical container metadata: '+str(e)})
 # Final direct-file report is deduplicated and written locally before one SMB publication copy.
 with report_path.open('w',encoding='utf8') as out:
  for row in documents.values():out.write(json.dumps(row,ensure_ascii=False)+'\n')
 summary={'as_of':'2026-10-13','no_web':True,'source_access':'metadata_only_read_only','scope_status':'scoped_metadata_census_recovered_with_explicit_archive_and_payload_exclusions',
  'source_root':str(VAULT),'direct_documents':len(documents),'direct_document_bytes':sum(r['bytes'] for r in documents.values()),'direct_format_counts':dict(collections.Counter(r['format'] for r in documents.values())),
  'recovered_direct_documents':base_count,'resumed_directories_listed':directories,'resumed_entries_seen':seen,'original_reported_directory_lower_bound':15600,
  'completed_original_top_roots_not_rescanned':['Videos','USB HDD 2','USB HDD 1','Trading Knowledge III'],
  'document_counts_by_top_area':dict(collections.Counter(r['path'].split('/')[0] for r in documents.values())),
  'archive_containers_metadata_observed':len(archives),'archive_container_bytes':sum(r['bytes'] for r in archives.values()),
  'archive_census_scope':'remaining top-level book areas plus individually stat-checked recorded archive containers; recovered USB/video roots not re-enumerated for unknown archives',
  'excluded':excluded,'errors':errors,'prior_unpersisted_scope_note':'The first SMB output flush failed after 17k rows. Prior directory/exclusion counters are not an exact final census and are not asserted. The interrupted Trading Knowledge II root was rescanned entirely; other remaining roots were completed.',
  'max_depth':policy.MAX_DEPTH,'document_formats':sorted(policy.FORMATS),'limits':['No source document payload/header/hash reads','No archive-member content/listing scan','Software/recycle/marketdata subtrees and symbolic links excluded','Research Docker bind unavailable; direct reader IDs are inactive inventory keys, not live links','File/title/recorded credit counts are not distinct authored books/verified individual authors','Other extensions and image-only volumes require separate bibliographic review'],
  'elapsed_seconds_resume':round(time.monotonic()-start,2)}
 atomic_json(PRIVATE/'fresh-scope.json',summary);atomic_json(PRIVATE/'fresh-archive-containers.json',{'as_of':'2026-10-13','archive_member_readability':'not_checked','files':list(archives.values())})
 manifest={'version':1,'source_root':str(VAULT),'reader_status':'inactive_research_bind_unavailable','hashes_certified':False,'files':{r['research_reader_id']:{k:r[k] for k in ('path','filename','bytes','mtime_ns','format')} for r in documents.values()}}
 assert len(manifest['files'])==len(documents)
 atomic_json(PRIVATE/'research-direct-file-manifest.json',manifest);(PRIVATE/'research-direct-file-manifest.json').chmod(0o600)
 for name in ['fresh-document-files.jsonl','fresh-scope.json','fresh-archive-containers.json']:shutil.copyfile(PRIVATE/name,OUT/name)
 if checkpoint.exists():checkpoint.unlink()
 print(json.dumps({k:v for k,v in summary.items() if k not in ('excluded','errors')},ensure_ascii=False,indent=2),flush=True)

if __name__=='__main__':main()
