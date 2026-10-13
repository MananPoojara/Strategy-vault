#!/usr/bin/env python3
"""Serial, resumable import of original PDFs captured for current app groups.

No writes into the research vault, no extraction, no execution of source files.
Reuse needs SHA-256 equality, not filename/size alone. Every outcome is recorded.
Routing uses captured metadata; title/author/content certification is separate.
"""
import collections
import argparse
import hashlib
import json
import os
from pathlib import Path
import time
from app_coverage import author_matches
from coverage_inventory import PROJECT, VAULT, author_kind, inventory_local, normalize
from import_book import digest, import_pdf


def source_jobs(coverage, source_records):
    jobs = {}
    for app in coverage['authors'].values():
        for title in app['source_titles']:
            for f in title['files']:
                relative = f.get('source_relative')
                if not relative or f.get('archive_member') or not f['filename'].lower().endswith('.pdf'):
                    continue
                row = source_records.get(f['record_id'], {})
                j = jobs.setdefault(relative, {'source_relative': relative,
                    'record_id': f['record_id'], 'recorded_author': row.get('author'),
                    'recorded_title': row.get('title'), 'metadata_status': row.get('metadata_status'),
                    'app_groups': [], 'app_names': [], 'source_title_ids': []})
                if app['id'] not in j['app_groups']:
                    j['app_groups'].append(app['id']); j['app_names'].append(app['name'])
                if title['id'] not in j['source_title_ids']:
                    j['source_title_ids'].append(title['id'])
    return sorted(jobs.values(), key=lambda j: j['source_relative'])


def destination_author(recorded, existing_folders, app_name):
    if (not recorded or author_kind(recorded) == 'unknown' or '/' in recorded or '\\' in recorded or
        '\0' in recorded or '\ufffd' in recorded or recorded in {'.', '..'}):
        raise ValueError('Unresolved or unsafe source author metadata; no guessed author folder')
    exact = [f for f in existing_folders if normalize(f) == normalize(recorded)]
    if exact:
        return exact[0], 'exact_existing_author_folder'
    candidates = [f for f in existing_folders if author_matches(recorded, {normalize(f)})]
    if len(candidates) == 1:
        return candidates[0], 'explicit_full_name_or_middle_initial_relation'
    if len(candidates) > 1:
        explicit = [f for f in candidates if normalize(f) == normalize(app_name)]
        if len(explicit) == 1:
            return explicit[0], 'coauthored_source_routed_to_explicit_app_author_collection'
        raise ValueError('Multiple existing author folders match; needs source identity review')
    if len(recorded.encode()) > 220:
        raise ValueError('Author metadata too long for a safe literal folder')
    return recorded.strip(), 'literal_recorded_author_collection_not_identity_certification'


def collision_books(books, author, relative):
    return Path(books) / author / 'Sources', 'source-' + hashlib.sha256(relative.encode()).hexdigest()[:16]


def run(project=PROJECT, vault=VAULT, date='2026-10-13', retry_held=False):
    project = Path(project); root = project / 'library/source-vault'; books = project / 'Books'
    coverage = json.loads((root / 'book-coverage.json').read_text())
    records = {r['source_record_id']: r for r in
               (json.loads(l) for l in (root / f'source-records-{date}.jsonl').read_text().splitlines())}
    jobs = source_jobs(coverage, records)
    (root / f'scoped-pdf-copy-plan-{date}.json').write_text(json.dumps({'source_access': 'read_only',
                'scope': 'Only original PDFs captured under current app author/group coverage',
                'jobs': jobs}, ensure_ascii=False, indent=2)+'\n')
    log = root / f'batch-import-{date}.jsonl'
    progress = root / f'copy-progress-{date}.json'
    prior = [json.loads(l) for l in log.read_text().splitlines()] if log.exists() else []
    latest = {r['source_relative']: r for r in prior}
    completed = {path for path, r in latest.items()
                 if not retry_held or r['status'] not in {'held_for_review', 'original_unavailable'}}
    local, _ = inventory_local(books)
    by_size = collections.defaultdict(list)
    for f in local:
        by_size[f['bytes']].append(f)
    existing_folders = sorted({f['author_folder'] for f in local if f.get('author_folder')})
    local_hashes = {}
    counts = collections.Counter(r['status'] for r in latest.values())
    bytes_copied = sum(r.get('bytes', 0) for r in prior if r['status'] == 'copied')
    bytes_source_read = 0
    started = time.monotonic()

    def save_progress(current=None):
        payload = {'date': date, 'scope_pdf_paths': len(jobs), 'processed': sum(counts.values()),
                   'statuses': dict(counts), 'new_bytes_copied': bytes_copied,
                   'new_MiB_copied': round(bytes_copied/1024**2, 3),
                   'elapsed_seconds': round(time.monotonic()-started, 2),
                   'source_access': 'read_only', 'workers': 1, 'current': current,
                   'status': 'running' if current else 'complete'}
        temporary = progress.with_suffix('.tmp'); temporary.write_text(json.dumps(payload, indent=2)+'\n')
        temporary.replace(progress)

    with log.open('a') as output:
        for number, job in enumerate(jobs, 1):
            if job['source_relative'] in completed:
                continue
            rec = dict(job); rec['imported'] = date
            source = Path(vault) / job['source_relative']
            save_progress(job['source_relative'])
            try:
                if job['metadata_status'] != 'parsed':
                    raise ValueError('Source metadata is malformed; exact path/author review required')
                canonical, route = destination_author(job['recorded_author'], existing_folders, job['app_names'][0])
                rec.update({'destination_author': canonical, 'routing_basis': route,
                            'bibliographic_identity_verified': False, 'complete_book_review': False,
                            'verified_performance': False})
                source = source.resolve()
                if not source.is_relative_to(Path(vault).resolve()) or not source.is_file():
                    raise FileNotFoundError('Original PDF unavailable or outside read-only source vault')
                before = source.stat()
                rec.update({'bytes': before.st_size, 'source_mtime_before_ns': before.st_mtime_ns})
                if before.st_size > 512*1024**2:
                    raise ValueError('PDF exceeds bounded 512MiB single-file limit; explicit size review pending')
                with source.open('rb') as inp:
                    if b'%PDF-' not in inp.read(1024):
                        raise ValueError('No PDF header; not importing a renamed binary')
                source_hash = digest(source); bytes_source_read += before.st_size
                rec['sha256'] = source_hash
                identical = None
                for f in by_size.get(before.st_size, []):
                    p = books / f['relative']; stat = p.stat()
                    cache_key = (str(p), stat.st_size, stat.st_mtime_ns)
                    if cache_key not in local_hashes:
                        local_hashes[cache_key] = digest(p)
                    if local_hashes[cache_key] == source_hash:
                        identical = p; break
                if identical:
                    rec.update({'status': 'reused_local_identical_sha256',
                                'destination': str(identical.relative_to(project)),
                                'byte_identity_verified': True})
                else:
                    space = os.statvfs(books)
                    if space.f_bavail*space.f_frsize < before.st_size + 10*1024**3:
                        raise ValueError('Destination has less than 10GiB reserve; import held')
                    destination = books / canonical / source.name
                    copy_books, copy_author = books, canonical
                    if destination.exists() and digest(destination) != source_hash:
                        copy_books, copy_author = collision_books(books, canonical, job['source_relative'])
                        destination = copy_books / copy_author / source.name
                    imported = import_pdf(job['source_relative'], copy_author, vault=Path(vault), books=copy_books)
                    rec.update({'status': imported['status'], 'destination': str(destination.relative_to(project)),
                                'sha256': imported['sha256'], 'byte_identity_verified': True})
                    if imported['status'] == 'copied':
                        bytes_copied += before.st_size
                    f = {'relative': str(destination.relative_to(books)), 'filename': source.name,
                         'bytes': before.st_size, 'author_folder': canonical, 'extension': '.pdf'}
                    by_size[before.st_size].append(f)
                    stat = destination.stat(); local_hashes[(str(destination), stat.st_size, stat.st_mtime_ns)] = imported['sha256']
                    if canonical not in existing_folders:
                        existing_folders.append(canonical)
                after_hash = digest(source); after = source.stat(); bytes_source_read += after.st_size
                rec.update({'source_hash_after': after_hash, 'source_mtime_after_ns': after.st_mtime_ns,
                            'source_mtime_unchanged': before.st_mtime_ns == after.st_mtime_ns,
                            'source_sha256_unchanged': after_hash == source_hash})
                if after_hash != source_hash or before.st_mtime_ns != after.st_mtime_ns:
                    rec['status'] = 'source_changed_during_import'; rec['reason'] = 'Source changed externally; source/copy identity requires review'
            except FileNotFoundError as e:
                rec.update({'status': 'original_unavailable', 'reason': str(e)})
            except (ValueError, OSError) as e:
                rec.update({'status': 'held_for_review', 'reason': str(e)})
            previous = latest.get(job['source_relative'])
            if previous:
                counts[previous['status']] -= 1
            counts[rec['status']] += 1
            latest[job['source_relative']] = rec
            output.write(json.dumps(rec, ensure_ascii=False)+'\n'); output.flush()
            if number % 20 == 0:
                print(json.dumps({'processed': sum(counts.values()), 'of': len(jobs),
                           'MiB_copied': round(bytes_copied/1024**2, 3), 'statuses': dict(counts)}), flush=True)
        save_progress()
    print(json.dumps({'done': True, 'paths': len(jobs), 'statuses': dict(counts),
                      'MiB_copied': round(bytes_copied/1024**2, 3)}, indent=2), flush=True)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--retry-held', action='store_true')
    args = parser.parse_args()
    run(retry_held=args.retry_held)
