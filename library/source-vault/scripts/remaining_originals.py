#!/usr/bin/env python3
"""Serial exact original document/member imports; never modifies source archives.

ZIP uses Python's bounded member stream with EOF/CRC verification. RAR uses the
installed libarchive 3.6 C API, opaque pointers and published function types.
Only explicitly captured current-app original PDF/DOC members are extracted.
No archive executable/script is extracted or executed. No generic extractall.
"""
import collections
import ctypes as C
import ctypes.util
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import re
import stat
import tempfile
import time
import zipfile

from coverage_inventory import PROJECT, VAULT, archive_locator, inventory_local
from scoped_import import destination_author, collision_books
from import_book import digest

MAX_BYTES = 512 * 1024**2
SUPPORTED_DIRECT = {'.doc', '.docx', '.djvu', '.epub'}
EXCLUDED_ARCHIVE = 'Introduction to Technical Analysis.7z'


def safe_member(name):
    if (not name or '\\' in name or '\0' in name or ':' in name or
            name.startswith('/') or archive_locator(name) or
            any(p in {'', '.', '..'} for p in name.split('/')) or
            PurePosixPath(name).suffix.lower() not in {'.pdf', '.doc'}):
        raise ValueError('Only a normalized relative authorized original PDF/DOC member is allowed')
    return name


def detect_document(path):
    path = Path(path)
    with path.open('rb') as stream: head = stream.read(4096)
    extension = path.suffix.lower()
    if head.startswith((b'MZ', b'\x7fELF')):
        raise ValueError('Executable signature; original document withheld')
    if extension == '.pdf' and b'%PDF-' in head[:1024]: return 'pdf'
    if extension == '.djvu' and head.startswith(b'AT&TFORM') and head[12:16] in {b'DJVU', b'DJVM'}: return 'djvu'
    if extension == '.doc':
        if head.startswith(bytes.fromhex('d0cf11e0a1b11ae1')): return 'ole_document'
        if head.lstrip().startswith(b'{\\rtf'): return 'rtf_document'
        if head.lstrip().lower().startswith((b'<!doctype html',b'<html')): return 'html_document_original_doc_extension'
        for encoding in ('utf-8', 'cp1252'):
            try:
                text = head.decode(encoding)
                if text.strip() and all(c.isprintable() or c in '\r\n\t\f\x1a\u00a0' for c in text):
                    return 'plain_text_original_doc_extension'
            except UnicodeDecodeError: pass
    if extension in {'.docx', '.epub'} and head.startswith(b'PK\x03\x04'):
        with zipfile.ZipFile(path) as z:
            names = z.namelist()
            if extension == '.docx' and '[Content_Types].xml' in names and 'word/document.xml' in names:
                return 'docx_container'
            if extension == '.epub' and 'mimetype' in names:
                m = z.getinfo('mimetype')
                if m.file_size <= 100 and z.read(m) == b'application/epub+zip': return 'epub_container'
    raise ValueError('Document header/container not recognized; no extension-only admission')


def copy_body(stream, destination, expected):
    if not isinstance(expected,int) or not 0 < expected <= MAX_BYTES:
        raise ValueError('Original member requires a finite bounded positive byte count')
    destination = Path(destination); destination.parent.mkdir(parents=True,exist_ok=True)
    created = False; total = 0; h = hashlib.sha256()
    try:
        with destination.open('xb') as out:
            created = True
            for data in iter(lambda: stream.read(1024*1024),b''):
                total += len(data)
                if total > expected: raise ValueError('Original stream exceeds declared byte count')
                h.update(data); out.write(data)
        if total != expected: raise ValueError('Original stream shorter than declared byte count')
        body_hash = h.hexdigest()
        if digest(destination) != body_hash: raise ValueError('Copied original failed SHA-256 equality')
        return {'bytes':total,'sha256':body_hash}
    except BaseException:
        if created: destination.unlink(missing_ok=True)
        raise


def zip_member(z, member, expected):
    safe_member(member)
    matches = [m for m in z.infolist() if m.filename == member]
    if len(matches) != 1: raise ValueError('Member missing or duplicate exact name')
    m = matches[0]
    if m.flag_bits & 1 or m.is_dir() or stat.S_ISLNK(m.external_attr >> 16):
        raise ValueError('Encrypted/nonregular member withheld')
    if not 0 < expected <= MAX_BYTES or m.file_size != expected:
        raise ValueError('Archive member size differs from captured original size')
    return z.open(m,'r')


def archive_snapshot(paths,vault):
    result=[]
    for p in paths:
        p=Path(p).resolve()
        if not p.is_relative_to(Path(vault).resolve()) or not p.is_file():
            raise ValueError('Archive volume must be an existing file inside read-only vault')
        before=p.stat(); h=digest(p); after=p.stat()
        if (before.st_size,before.st_mtime_ns)!=(after.st_size,after.st_mtime_ns):
            raise ValueError('Archive changed externally while hashing')
        result.append({'source_relative':str(p.relative_to(vault)), 'bytes':before.st_size,
                       'sha256':h,'mtime_ns':before.st_mtime_ns})
    return result


class NativeArchive:
    """Opaque libarchive API; prototypes from official libarchive v3.6.0 headers."""
    def __init__(self, paths):
        self.lib=C.CDLL(ctypes.util.find_library('archive'))
        signatures={
          'archive_read_new':([],C.c_void_p),
          'archive_read_support_filter_all':([C.c_void_p],C.c_int),
          'archive_read_support_format_rar':([C.c_void_p],C.c_int),
          'archive_read_support_format_rar5':([C.c_void_p],C.c_int),
          'archive_read_support_format_zip':([C.c_void_p],C.c_int),
          'archive_read_open_filenames':([C.c_void_p,C.POINTER(C.c_char_p),C.c_size_t],C.c_int),
          'archive_read_next_header':([C.c_void_p,C.POINTER(C.c_void_p)],C.c_int),
          'archive_entry_pathname_utf8':([C.c_void_p],C.c_char_p),
          'archive_entry_pathname':([C.c_void_p],C.c_char_p),
          'archive_entry_size':([C.c_void_p],C.c_int64),
          'archive_entry_size_is_set':([C.c_void_p],C.c_int),
          'archive_entry_filetype':([C.c_void_p],C.c_uint),
          'archive_entry_is_encrypted':([C.c_void_p],C.c_int),
          'archive_read_data':([C.c_void_p,C.c_void_p,C.c_size_t],C.c_ssize_t),
          'archive_read_data_skip':([C.c_void_p],C.c_int),
          'archive_error_string':([C.c_void_p],C.c_char_p),
          'archive_read_free':([C.c_void_p],C.c_int)}
        for n,(args,result) in signatures.items():
            fn=getattr(self.lib,n); fn.argtypes=args; fn.restype=result
        self.handle=self.lib.archive_read_new()
        if not self.handle: raise OSError('libarchive allocation failed')
        self.files=(C.c_char_p*(len(paths)+1))(*[os.fsencode(p) for p in paths],None)
        try:
            for n in ('filter_all','format_rar','format_rar5','format_zip'):
                if getattr(self.lib,'archive_read_support_'+n)(self.handle)<0: raise OSError(self.error())
            if self.lib.archive_read_open_filenames(self.handle,self.files,65536)<0: raise OSError(self.error())
        except BaseException:
            self.close(); raise
    def error(self):
        value=self.lib.archive_error_string(self.handle)
        return value.decode('utf-8','replace') if value else 'Native archive reader failure'
    def entries(self):
        for _ in range(20000):
            entry=C.c_void_p(); code=self.lib.archive_read_next_header(self.handle,C.byref(entry))
            if code==1: return
            if code<0: raise OSError(self.error())
            name=(self.lib.archive_entry_pathname_utf8(entry) or self.lib.archive_entry_pathname(entry))
            if not name: raise ValueError('Archive member has no readable path')
            yield {'name':name.decode('utf-8','strict'),'bytes':self.lib.archive_entry_size(entry),
                   'size_set':bool(self.lib.archive_entry_size_is_set(entry)),
                   'regular':stat.S_ISREG(self.lib.archive_entry_filetype(entry)),
                   'encrypted':bool(self.lib.archive_entry_is_encrypted(entry))}
            if self.lib.archive_read_data_skip(self.handle)<0: raise OSError(self.error())
        raise ValueError('Archive header scan exceeds bounded 20000-entry limit')
    def read(self,n):
        buf=C.create_string_buffer(n); count=self.lib.archive_read_data(self.handle,buf,n)
        if count<0: raise OSError(self.error())
        return buf.raw[:count]
    def close(self):
        if self.handle: self.lib.archive_read_free(self.handle); self.handle=None
    def __enter__(self): return self
    def __exit__(self,*args): self.close()


def remaining_jobs(coverage,records):
    jobs={}; exclusions=[]
    for a in coverage['authors'].values():
        for t in a['source_titles']:
            for f in t['files']:
                relative=f.get('source_relative'); ext=Path(f['filename']).suffix.lower()
                if not relative: continue
                locator=archive_locator(relative)
                if locator and locator[0].endswith(EXCLUDED_ARCHIVE):
                    exclusions.append({'source_relative':relative,'reason':'Software installer/support PDF; course-folder placement is not trading-book authorship proof'})
                    continue
                if not ((locator and ext in {'.pdf','.doc'}) or (not locator and ext in SUPPORTED_DIRECT)): continue
                row=records.get(f['record_id'],{})
                job=jobs.setdefault(relative,{'source_relative':relative,'record_id':f['record_id'],
                    'recorded_author':row.get('author'),'recorded_title':row.get('title'),
                    'metadata_status':row.get('metadata_status'),'expected_bytes':f.get('recorded_bytes'),
                    'source_kind':'archive_member' if locator else 'direct_original',
                    'app_groups':[],'app_names':[],'source_title_ids':[]})
                if a['id'] not in job['app_groups']:
                    job['app_groups'].append(a['id']); job['app_names'].append(a['name'])
                if t['id'] not in job['source_title_ids']: job['source_title_ids'].append(t['id'])
    return sorted(jobs.values(),key=lambda j:j['source_relative']),list({e['source_relative']:e for e in exclusions}.values())


def run(project=PROJECT,vault=VAULT,date='2026-10-13',only_source=None,only_archived_docs=False):
    project=Path(project).resolve(); vault=Path(vault).resolve(); root=project/'library/source-vault'; books=project/'Books'
    if books.resolve().is_relative_to(vault) or vault.is_relative_to(books.resolve()): raise ValueError('Destination must remain separate from read-only source vault')
    records={r['source_record_id']:r for r in map(json.loads,(root/f'source-records-{date}.jsonl').read_text().splitlines())}
    jobs,exclusions=remaining_jobs(json.loads((root/'book-coverage.json').read_text()),records)
    (root/f'remaining-original-copy-plan-{date}.json').write_text(json.dumps({'jobs':jobs,'exclusions':exclusions,'workers':1,'source_access':'read_only'},ensure_ascii=False,indent=2)+'\n')
    log=root/f'batch-import-{date}.jsonl'; prior=list(map(json.loads,log.read_text().splitlines())) if log.exists() else []
    latest={r['source_relative']:r for r in prior}
    jobs=[j for j in jobs if latest.get(j['source_relative'],{}).get('status') not in {'copied','already_present_identical','reused_local_identical_sha256'}]
    if only_source is not None: jobs=[j for j in jobs if j['source_relative']==only_source]
    if only_archived_docs: jobs=[j for j in jobs if j['source_kind']=='archive_member' and Path(j['source_relative']).suffix.lower()=='.doc']
    local,_=inventory_local(books); by_size=collections.defaultdict(list)
    for f in local: by_size[f['bytes']].append(books/f['relative'])
    folders=sorted({f['author_folder'] for f in local if f.get('author_folder')}); hashes={}
    started=time.monotonic(); completed=[]; copied_bytes=0
    progress=root/f'remaining-copy-progress-{date}.json'
    def save(current=None):
        payload={'status':'running' if current else 'complete','jobs':len(jobs),'processed':len(completed),
                 'statuses':dict(collections.Counter(r['status'] for r in completed)),
                 'new_bytes_copied':copied_bytes,'elapsed_seconds':round(time.monotonic()-started,2),'current':current,'workers':1}
        progress.write_text(json.dumps(payload,indent=2)+'\n')
    def base(j):
        rec=dict(j); canonical,route=destination_author(j['recorded_author'],folders,j['app_names'][0])
        rec.update({'destination_author':canonical,'routing_basis':route,'imported':date,
                    'bibliographic_identity_verified':False,'complete_book_review':False,'verified_performance':False})
        if j['metadata_status']!='parsed': raise ValueError('Malformed captured source identity')
        if not isinstance(j['expected_bytes'],int) or not 0<j['expected_bytes']<=MAX_BYTES: raise ValueError('Missing/unbounded declared original byte count')
        return rec
    def publish(rec,body,name):
        nonlocal copied_bytes
        kind=detect_document(body); size=body.stat().st_size; h=digest(body)
        if size!=rec['expected_bytes']: raise ValueError('Original differs from captured byte count')
        canonical=rec['destination_author']; identical=None
        for p in by_size[size]:
            s=p.stat(); key=(str(p),s.st_size,s.st_mtime_ns)
            if key not in hashes: hashes[key]=digest(p)
            if hashes[key]==h: identical=p; break
        status='reused_local_identical_sha256'
        if identical is None:
            dest=books/canonical/name
            if dest.exists():
                basepath,sub=collision_books(books,canonical,rec['source_relative']); dest=basepath/sub/name
            if not dest.resolve().is_relative_to(books.resolve()): raise ValueError('Destination escaped Books')
            if dest.exists():
                if digest(dest)!=h: raise ValueError('Different collision destination exists; refusing overwrite')
            else:
                sp=os.statvfs(books)
                if sp.f_bavail*sp.f_frsize<size+10*1024**3: raise ValueError('Less than 10GiB destination reserve')
                with body.open('rb') as stream: copy_body(stream,dest,size)
                copied_bytes+=size; status='copied'
            identical=dest; by_size[size].append(dest)
            if canonical not in folders: folders.append(canonical)
        rec.update({'status':status,'destination':str(identical.relative_to(project)),
                    'bytes':size,'sha256':h,'format_magic':kind,'byte_identity_verified':True})
        if digest(identical)!=h: raise ValueError('Published original SHA-256 differs')
    def emit(rec):
        completed.append(rec)
        with log.open('a') as out: out.write(json.dumps(rec,ensure_ascii=False)+'\n'); out.flush()
        save(rec['source_relative'])
        print(json.dumps({'processed':len(completed),'of':len(jobs),'status':rec['status'],'MiB_copied':round(copied_bytes/1024**2,2)}),flush=True)
    grouped=collections.defaultdict(list)
    for j in jobs:
        loc=archive_locator(j['source_relative'])
        if loc: grouped[loc[0]].append(j); continue
        save(j['source_relative']); rec=dict(j)
        try:
            rec=base(j); source=(vault/j['source_relative']).resolve()
            if not source.is_relative_to(vault) or not source.is_file(): raise FileNotFoundError('Direct original unavailable/outside vault')
            before=source.stat(); first=digest(source)
            publish(rec,source,source.name)
            second=digest(source); after=source.stat()
            rec.update({'source_hash_after':second,'source_mtime_before_ns':before.st_mtime_ns,
                        'source_mtime_after_ns':after.st_mtime_ns,
                        'source_sha256_unchanged':first==second==rec['sha256'],
                        'source_mtime_unchanged':before.st_mtime_ns==after.st_mtime_ns and before.st_size==after.st_size})
            if not rec['source_sha256_unchanged'] or not rec['source_mtime_unchanged']: raise ValueError('Source changed externally during import')
        except (OSError,ValueError,zipfile.BadZipFile) as e: rec.update(status='held_for_review',reason=str(e),byte_identity_verified=False)
        emit(rec)
    for relative,group in sorted(grouped.items()):
        save(relative); outer=(vault/relative).resolve(); out_records=[]
        try:
            if not outer.is_relative_to(vault) or not outer.is_file(): raise FileNotFoundError('Source archive unavailable/outside vault')
            paths=[outer]
            if outer.name.endswith('.part01.rar'):
                paths=[outer.with_name(outer.name.replace('.part01.rar',f'.part{i:02d}.rar')) for i in range(1,10)]
                if not all(p.is_file() for p in paths): raise ValueError('Exact expected 9 multipart RAR volumes not all available')
            before=archive_snapshot(paths,vault)
            wanted={archive_locator(j['source_relative'])[1]:j for j in group}
            for member in wanted: safe_member(member)
            with tempfile.TemporaryDirectory(prefix='vault-original-') as temp:
                def extract(j,stream):
                    rec=dict(j)
                    try:
                        rec=base(j); member=archive_locator(j['source_relative'])[1]; target=Path(temp)/Path(member).name
                        copy_body(stream,target,j['expected_bytes']); publish(rec,target,target.name)
                        rec['source_member_sha256']=rec['sha256']; target.unlink()
                    except (OSError,ValueError,zipfile.BadZipFile) as e:
                        rec.update(status='held_for_review',reason=str(e),byte_identity_verified=False)
                    out_records.append(rec)
                if outer.suffix.lower()=='.zip':
                    with zipfile.ZipFile(outer) as z:
                        for member,j in wanted.items():
                            try:
                                with zip_member(z,member,j['expected_bytes']) as stream: extract(j,stream)
                            except (OSError,ValueError,zipfile.BadZipFile) as e:
                                out_records.append(dict(j,status='held_for_review',reason=str(e),byte_identity_verified=False))
                elif outer.suffix.lower()=='.rar':
                    # Scan metadata separately; a failed selected compressed
                    # body must not poison decoder state for another original.
                    metadata={}; names=collections.Counter()
                    with NativeArchive(paths) as reader:
                        for entry in reader.entries():
                            name=entry['name']
                            if name not in wanted: continue
                            names[name]+=1; metadata[name]=entry
                    for name,j in wanted.items():
                        entry=metadata.get(name)
                        if not entry or names[name]!=1:
                            out_records.append(dict(j,status='held_for_review',reason='RAR member absent or duplicate exact name',byte_identity_verified=False)); continue
                        if entry['encrypted'] or not entry['regular'] or not entry['size_set'] or entry['bytes']!=j['expected_bytes']:
                            out_records.append(dict(j,status='held_for_review',reason='RAR target is encrypted/nonregular or declared-size mismatch',byte_identity_verified=False)); continue
                        with NativeArchive(paths) as reader:
                            for current in reader.entries():
                                if current['name']==name:
                                    extract(j,reader)
                                    # Close the selected reader even on a body
                                    # failure, rather than advancing poisoned state.
                                    break
                else: raise ValueError('Target archive format not admitted')
            after=archive_snapshot(paths,vault); unchanged=before==after
            single=len(paths)==1
            bh=before[0]['sha256'] if single else hashlib.sha256(json.dumps(before,sort_keys=True).encode()).hexdigest()
            ah=after[0]['sha256'] if single else hashlib.sha256(json.dumps(after,sort_keys=True).encode()).hexdigest()
            volumes=[{'source_relative':b['source_relative'],'bytes':b['bytes'],'sha256_before':b['sha256'],'sha256_after':a['sha256'],
                      'mtime_before_ns':b['mtime_ns'],'mtime_after_ns':a['mtime_ns']} for b,a in zip(before,after)]
            for rec in out_records:
                rec.update({'archive_source_relative':relative,'archive_sha256_before':bh,'archive_sha256_after':ah,
                            'archive_hash_scope':'single_container_bytes' if single else 'sha256_of_sorted_json_volume_snapshot',
                            'archive_mtime_before_ns':before[0]['mtime_ns'],'archive_mtime_after_ns':after[0]['mtime_ns'],
                            'archive_volumes':volumes,'source_sha256_unchanged':unchanged,'source_mtime_unchanged':unchanged,
                            'archive_reader':'Python zipfile CRC checked on stream EOF' if outer.suffix.lower()=='.zip' else 'libarchive 3.6 native RAR reader; no independent CRC claim'})
                if not unchanged: rec.update(status='held_for_review',reason='Archive externally changed during extraction',byte_identity_verified=False)
        except (OSError,ValueError,zipfile.BadZipFile,UnicodeDecodeError) as e:
            # Do not admit partial extraction without complete source preservation proof.
            out_records=[dict(j,status='held_for_review',reason=str(e),byte_identity_verified=False) for j in group]
        for rec in out_records: emit(rec)
    save(); print(json.dumps({'done':True,'processed':len(completed),'new_MiB':round(copied_bytes/1024**2,3),'statuses':dict(collections.Counter(r['status'] for r in completed))}),flush=True)


if __name__=='__main__': run()
