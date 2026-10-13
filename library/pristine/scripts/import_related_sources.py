#!/usr/bin/env python3
"""Preserve exact enumerated Pristine-related documents; never traverse video/software trees."""
import csv
import errno
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import stat
import sys
import tempfile

WORKSPACE = Path(__file__).resolve().parents[3]
SOURCE_ROOT = Path('/run/user/1000/gvfs/smb-share:server=192.168.4.50,share=research')
STAGER = WORKSPACE / 'library/source-vault/scripts/stage_book_reader.py'
spec = importlib.util.spec_from_file_location('pristine_reader_stager', STAGER)
safe = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = safe
spec.loader.exec_module(safe)

def segment(value):
    return re.sub(r'[^\w .&()\-]+', '-', value, flags=re.UNICODE).strip(' .-')[:100] or 'Attribution unestablished'

def preserve(source_root_fd, source_relative, books_fd, destination, expected_size, existing=None):
    src_parent, src_name = safe._parent(source_root_fd, source_relative)
    src = parent = tmp = None
    temp_name = None
    try:
        src = os.open(src_name, os.O_RDONLY | safe.NOFOLLOW, dir_fd=src_parent)
        before = safe._regular(src)
        if before.st_size != expected_size:
            raise ValueError('Source size changed since discovery')
        if os.read(src, 5) != b'%PDF-':
            raise ValueError('PDF header does not match the reader admission contract; original retained')
        # Only explicitly named, same-size previously verified local files are reuse candidates.
        candidate = existing is not None
        if not candidate:
            parent, name = safe._parent(books_fd, destination, create=True)
            temp_name = '.pristine-import-' + os.urandom(12).hex()
            tmp = os.open(temp_name, os.O_WRONLY | os.O_CREAT | os.O_EXCL | safe.NOFOLLOW, 0o600, dir_fd=parent)
        digest = safe._digest(src, tmp)
        if digest != safe._digest(src) or safe._signature(before) != safe._signature(safe._regular(src)):
            raise ValueError('Source changed while hashing/copying')
        if safe._signature(before) != safe._signature(os.stat(src_name, dir_fd=src_parent, follow_symlinks=False)):
            raise ValueError('Source path identity changed while copying')
        if candidate and digest == existing['sha256']:
            local_parent, local_name = safe._parent(books_fd, existing['path'])
            try:
                local_fd = os.open(local_name, os.O_RDONLY | safe.NOFOLLOW, dir_fd=local_parent)
                try:
                    local_before = safe._regular(local_fd)
                    if local_before.st_size != before.st_size or safe._digest(local_fd) != digest:
                        raise ValueError('Existing local candidate no longer equals source')
                    if safe._signature(local_before) != safe._signature(safe._regular(local_fd)):
                        raise ValueError('Existing local candidate changed while verifying')
                finally:
                    os.close(local_fd)
            finally:
                os.close(local_parent)
            return digest, existing['path'], 'reused_local_identical_sha256', before.st_mtime_ns
        if candidate:
            parent, name = safe._parent(books_fd, destination, create=True)
            temp_name = '.pristine-import-' + os.urandom(12).hex()
            tmp = os.open(temp_name, os.O_WRONLY | os.O_CREAT | os.O_EXCL | safe.NOFOLLOW, 0o600, dir_fd=parent)
            if safe._digest(src, tmp) != digest or safe._signature(before) != safe._signature(safe._regular(src)):
                raise ValueError('Source changed while copying variant')
        os.fsync(tmp)
        os.close(tmp)
        tmp = None
        verify = os.open(temp_name, os.O_RDONLY | safe.NOFOLLOW, dir_fd=parent)
        try:
            if safe._digest(verify) != digest:
                raise ValueError('Temporary destination differs from source')
        finally:
            os.close(verify)
        try:
            try:
                os.link(temp_name, name, src_dir_fd=parent, dst_dir_fd=parent, follow_symlinks=False)
            except OSError as error:
                if error.errno not in (errno.EPERM, errno.ENOSYS, errno.EXDEV, errno.EOPNOTSUPP):
                    raise
                # GVFS/SMB does not implement hard links. Exclusive creation still
                # prevents overwriting; completion is logged only after full verification.
                final = os.open(name, os.O_RDWR | os.O_CREAT | os.O_EXCL | safe.NOFOLLOW, 0o600, dir_fd=parent)
                staged = None
                success = False
                try:
                    staged = os.open(temp_name, os.O_RDONLY | safe.NOFOLLOW, dir_fd=parent)
                    if safe._digest(staged, final) != digest:
                        raise ValueError('Exclusive destination copy differs from verified temporary source')
                    os.fsync(final)
                    if safe._digest(final) != digest:
                        raise ValueError('Exclusive destination body differs from original')
                    success = True
                finally:
                    if staged is not None:
                        os.close(staged)
                    os.close(final)
                    if not success:
                        os.unlink(name, dir_fd=parent)
        except FileExistsError:
            old = os.open(name, os.O_RDONLY | safe.NOFOLLOW, dir_fd=parent)
            try:
                if safe._regular(old).st_size != before.st_size or safe._digest(old) != digest:
                    raise ValueError('Conflicting existing destination; never overwritten')
            finally:
                os.close(old)
            status = 'reused_local_identical_sha256'
        else:
            status = 'copied'
        os.unlink(temp_name, dir_fd=parent)
        temp_name = None
        # NAS share permissions may not support chmod; byte identity is independent of its mode.
        return digest, destination, status, before.st_mtime_ns
    finally:
        for fd in (tmp, src, parent, src_parent):
            if fd is not None:
                os.close(fd)
        if temp_name is not None and parent is not None:
            cleanup_parent, _ = safe._parent(books_fd, destination)
            try:
                os.unlink(temp_name, dir_fd=cleanup_parent)
            finally:
                os.close(cleanup_parent)

def main():
    rows = json.loads(Path('/tmp/pristine-sources-metadata.json').read_text())
    index = json.loads(Path('/home/server/.cache/strategy-vault-webapp/book-index.json').read_text())['files']
    existing_by_name = {}
    for entry in index.values():
        if entry.get('sha256') and entry.get('bytes') is not None:
            existing_by_name.setdefault((entry['filename'], entry['bytes']), []).append(entry)
    root_fd = safe.open_directory(SOURCE_ROOT)
    books_fd = safe.open_directory(WORKSPACE / 'Books')
    base = WORKSPACE / 'library/pristine'
    report = []
    log = base / 'import-completed-2026-10-13.jsonl'
    try:
        with log.open('a', encoding='utf8') as completed:
            for n, row in enumerate(rows, 1):
                record = dict(row)
                path = Path(row['source_path'])
                try:
                    relative = str(path.relative_to(SOURCE_ROOT))
                    if path.suffix.lower() != '.pdf' or path.name.startswith('~$'):
                        raise ValueError('Outside the authorized original PDF document scope')
                    candidates = existing_by_name.get((path.name, row['bytes']), [])
                    # Multiple plausible original paths are not collapsed through a filename heuristic.
                    existing = candidates[0] if len(candidates) == 1 else None
                    author_group = segment(row['source_author'] if row['source_author'] != '?' else 'Attribution unestablished')
                    source_group = hashlib.sha256(relative.encode()).hexdigest()[:12]
                    destination = f"Pristine Trading/Related sources/{author_group}/{row['source_id']}/{source_group}/{path.name}"
                    digest, local, status, mtime = preserve(root_fd, relative, books_fd, destination, row['bytes'], existing)
                    record.update(status=status, sha256=digest, local_verified_path='Books/' + local,
                                  source_hash_after=digest, source_sha256_unchanged=True, source_mtime_unchanged=True,
                                  source_mtime_before_ns=mtime, source_mtime_after_ns=mtime, byte_identity_verified=True,
                                  destination='Books/' + local, source_kind='direct_document')
                    completed.write(json.dumps(record, ensure_ascii=False) + '\n')
                    completed.flush()
                    os.fsync(completed.fileno())
                except (OSError, ValueError) as error:
                    record.update(status='missing', error=str(error), byte_identity_verified=False)
                    print(json.dumps({'withheld_source': row['source_id'], 'error': str(error)}), flush=True)
                report.append(record)
                print(json.dumps({'processed': n, 'total': len(rows), 'status': record['status'], 'bytes': row.get('bytes'), 'source_id': row['source_id']}, ensure_ascii=False), flush=True)
    finally:
        os.close(root_fd)
        os.close(books_fd)
    groups = {}
    year_by_id = {}
    raw_coverage = json.loads((WORKSPACE / 'library/source-vault/coverage-2026-10-13.json').read_text())
    for author in raw_coverage['authors']:
        for source in author.get('title_candidates', []):
            year_by_id[source['id']] = int(source['year']) if str(source.get('year', '')).isdigit() and len(str(source['year'])) == 4 else None
    curated = {
        'title-fb39cba60187745c': ('p_intraday_capra', 'Greg Capra/Greg Capra - Intraday Trading Techniques.pdf'),
        'title-3e013bb826ff1ea3': ('p_tools_tactics', 'Greg Capra/Oliver Velez & Greg Capra - Tools & Tactics for the Master Day Trader.pdf'),
        'title-7aaf90018dc83546': ('p_tpm1', 'Greg Capra/TPM1 Live Color Manual.pdf')
    }
    for row in report:
        group = groups.setdefault(row['source_id'], {'id': row['source_id'], 'title': row['candidate_title'],
            'source_author': row['source_author'], 'year': year_by_id.get(row['source_id']), 'files': [], 'local_copies': [],
            'local_status': 'missing', 'count_basis': 'related source-title candidate; articles/manuals are not verified distinct authored books',
            'summary': 'Original source identity and availability checked. Content summary and complete objective trading rules await independent review; no backtest is attributed to this reading source.'})
        f = {k: row.get(k) for k in ('source_path', 'sha256', 'bytes', 'status', 'source_mtime_before_ns', 'source_mtime_after_ns', 'source_sha256_unchanged', 'source_mtime_unchanged')}
        f['filename'] = Path(row['source_path']).name
        if row.get('error'):
            f['error'] = row['error']
        if row.get('byte_identity_verified'):
            f['verified_local_copy'] = {'path': row['local_verified_path'], 'sha256': row['sha256'], 'bytes': row['bytes'], 'byte_identity_verified': True}
            group['local_copies'].append({'path': row['local_verified_path']})
            mapping = curated.get(row['source_id'])
            if mapping and row['local_verified_path'] == 'Books/' + mapping[1]:
                group['catalogue_candidates'] = [{'id': mapping[0], 'match': 'curated_reading_original_sha256_identity_verified_not_strategy_certification'}]
        group['files'].append(f)
    for group in groups.values():
        if all(f.get('verified_local_copy') for f in group['files']):
            group['local_status'] = 'verified_local_copy'
        elif group['local_copies']:
            group['local_status'] = 'partial_verified_local_copy'
    output = {'as_of': '2026-10-13', 'scope': '31 exact originally recorded PDF paths, no videos/software/archive execution',
              'counts': {'source_title_candidates': len(groups), 'source_locations': len(report),
                         'copied': sum(r['status'] == 'copied' for r in report), 'reused': sum(r['status'] == 'reused_local_identical_sha256' for r in report),
                         'missing': sum(r['status'] == 'missing' for r in report)}, 'entries': list(groups.values())}
    (base / 'source-reading-index.json').write_text(json.dumps(output, ensure_ascii=False, indent=2) + '\n')
    fields = ['source_id','source_author','candidate_title','source_path','sha256','local_verified_path','bytes','status','source_mtime_before_ns','source_mtime_after_ns','source_sha256_unchanged','source_mtime_unchanged','error']
    with (base / 'source-reading-index.csv').open('w', encoding='utf8', newline='') as file:
        writer = csv.DictWriter(file, fieldnames=fields, extrasaction='ignore');writer.writeheader();writer.writerows(report)
    unique = {}
    for r in report:
        if r.get('byte_identity_verified'):
            local = WORKSPACE / r['local_verified_path']
            unique[str(local.relative_to(WORKSPACE / 'Books'))] = {'relative': str(local.relative_to(WORKSPACE / 'Books')), 'bytes': r['bytes'], 'mtime_ns': local.stat().st_mtime_ns, 'sha256': r['sha256']}
    (base / 'reader-delta-snapshot.json').write_text(json.dumps({'files': list(unique.values())}, ensure_ascii=False, indent=2)+'\n')
    print(json.dumps(output['counts']), flush=True)

if __name__ == '__main__':
    main()
