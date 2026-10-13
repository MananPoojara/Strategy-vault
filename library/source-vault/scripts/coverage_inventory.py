#!/usr/bin/env python3
"""Reconcile recorded source metadata without treating availability as verification.

Reads the research share only. Writes generated manifests only to source-vault.
No PDF text extraction, archive extraction, source execution, or bulk import.
"""
import argparse
import collections
import csv
import hashlib
import io
import json
import os
from pathlib import Path, PurePosixPath
import re
import subprocess
import unicodedata
import urllib.request

VAULT = '/run/user/1000/gvfs/smb-share:server=192.168.4.50,share=research'
PROJECT = Path(__file__).resolve().parents[3]
DOCUMENTS = {'.pdf', '.doc', '.docx', '.epub', '.djvu', '.chm', '.azw3'}


def archive_locator(path):
    # Literal ! in a directory/title/filename is legal. Only a known archive
    # extension followed by ! marks the historical archive-member convention.
    match = re.search(r'\.(?:zip|rar|7z|iso|cab|tgz|tar|gz|bz2|xz)!', str(path or ''), re.I)
    return (path[:match.end()-1], path[match.end():]) if match else None


def member_filename(path):
    locator = archive_locator(path)
    return PurePosixPath(locator[1] if locator else str(path or '')).name


def document_extension(path):
    return PurePosixPath(member_filename(path)).suffix.lower()


def normalize(value):
    value = unicodedata.normalize('NFKC', str(value or '')).casefold()
    return ' '.join(re.findall(r'[^\W_]+', value, flags=re.UNICODE))


def author_kind(value):
    norm = normalize(value)
    if not norm or norm in {'unknown', 'anonymous', 'anon', 'unresolved'}:
        return 'unknown'
    return 'vendor_metadata' if str(value).lower().startswith('vendor:') else 'named_metadata'


def parse_inventory(path):
    raw = Path(path).read_bytes()
    lines = raw.decode('utf-8', errors='replace').splitlines()
    reader = csv.DictReader((line.replace('\0', '\ufffd') for line in lines), delimiter='\t')
    rows = []
    for n, row in enumerate(reader, 2):
        row['source_record_id'] = f'research-row-{n}'
        row['inventory_line'] = n
        row['metadata_status'] = ('malformed_fields' if None in row else
                                  'contains_replaced_nul' if '\ufffd' in str(row) else 'parsed')
        if None in row:
            row['extra_fields'] = row.pop(None)
        rows.append(row)
    return rows, {'bytes': len(raw), 'sha256': hashlib.sha256(raw).hexdigest(),
                  'nul_bytes': raw.count(b'\0'), 'rows': len(rows),
                  'metadata_issues': dict(collections.Counter(r['metadata_status'] for r in rows))}


def group_records(rows):
    groups = {}
    for row in rows:
        key = (normalize(row.get('author')), normalize(row.get('title')), str(row.get('year') or ''))
        # Unknown/empty titles never collapse unrelated records into one book.
        if not key[1]:
            key += (row['source_record_id'],)
        if key not in groups:
            groups[key] = {'id': 'title-' + hashlib.sha256(json.dumps(key).encode()).hexdigest()[:16],
                           'author': row.get('author') or '?', 'title': row.get('title') or '',
                           'year': row.get('year') or None, 'records': [],
                           'bibliographic_status': 'metadata_candidate_not_verified_book',
                           'byte_duplicates_verified': False}
        groups[key]['records'].append(row)
    return sorted(groups.values(), key=lambda g: (normalize(g['author']), normalize(g['title']), str(g['year'])))


def safe_direct_relative(path, vault=VAULT):
    if not path or archive_locator(path) or '\0' in path or '\ufffd' in path:
        return None
    p, root = PurePosixPath(path), PurePosixPath(vault)
    if '..' in p.parts or not p.is_absolute():
        return None
    try:
        return str(p.relative_to(root))
    except ValueError:
        return None


def inventory_local(books, max_files=20000, max_depth=8):
    if max_files < 1:
        raise ValueError('File bound must be positive')
    books = Path(books)
    rows, audit = [], {'symlinks_skipped': 0, 'non_documents': 0, 'depth_limited': [], 'files_seen': 0}
    for root, directories, files in os.walk(books, followlinks=False):
        directories.sort(); files.sort()
        for name in list(directories):
            if (Path(root) / name).is_symlink():
                directories.remove(name); audit['symlinks_skipped'] += 1
        if len(Path(root).relative_to(books).parts) >= max_depth:
            audit['depth_limited'].extend(str(Path(root, n).relative_to(books)) for n in directories)
            directories.clear()
        for name in files:
            audit['files_seen'] += 1
            if audit['files_seen'] > max_files:
                raise ValueError('Local inventory exceeded explicit file bound')
            p = Path(root) / name
            if p.is_symlink():
                audit['symlinks_skipped'] += 1; continue
            if p.suffix.lower() not in DOCUMENTS:
                audit['non_documents'] += 1; continue
            s = p.stat(); relative = p.relative_to(books)
            rows.append({'relative': str(relative), 'filename': name, 'bytes': s.st_size,
                         'mtime_ns': s.st_mtime_ns, 'extension': p.suffix.lower(),
                         'author_folder': relative.parts[0] if len(relative.parts) > 1 else None,
                         'byte_identity_verified': False})
    return rows, audit


def local_matches(row, local):
    try:
        size = int(row.get('size', ''))
    except (ValueError, TypeError):
        return []
    filename = member_filename(row.get('path_or_archive!member', ''))
    return [p for p in local if p['filename'] == filename and p['bytes'] == size]


def summarize_api(rows):
    result = {}
    for r in rows:
        book = r.get('book') or '(unspecified)'
        b = result.setdefault(book, {'strategies': 0, 'statuses': {}, 'gates': {},
                                    'independently_verified_book_count': None})
        b['strategies'] += 1
        status = r.get('status') or '(unspecified)'
        b['statuses'][status] = b['statuses'].get(status, 0) + 1
        gate = (r.get('gate') or {}).get('verdict') or '(no_gate_metadata)'
        b['gates'][gate] = b['gates'].get(gate, 0) + 1
    return result


def catalogue_snapshot(frontend):
    # Evaluate only the inspected catalogue literal, not the page/app scripts.
    text = (frontend / 'full-variant1.html').read_text()
    start = text.index('const LIB = [') + len('const LIB = ')
    end = text.index('\n];', start) + 3
    literal = text[start:end].rstrip().removesuffix(';')
    ross_text = (frontend / 'joe-ross-evidence.js').read_text()
    ross = json.loads(re.sub(r'^.*?window\.JoeRossEvidence\s*=\s*', '', ross_text, flags=re.S).rstrip().removesuffix(';'))
    payload = json.dumps({'literal': literal, 'ross': ross})
    script = '''const fs=require('fs'), vm=require('vm'); const p=JSON.parse(fs.readFileSync(0,'utf8'));
const value=vm.runInNewContext('('+p.literal+')',{window:{JoeRossEvidence:p.ross}},
{timeout:1000,contextCodeGeneration:{strings:false,wasm:false}}); process.stdout.write(JSON.stringify(value));'''
    p = subprocess.run(['node', '-e', script], input=payload, text=True, capture_output=True, timeout=5, check=True)
    return json.loads(p.stdout)


def fresh_top_level(vault, areas=('Trading Books', 'Other TradingInvesting books', 'Larry Connors'), limit=2500):
    result = {'scope': 'immediate entries in three named areas; no recursive or archive scan', 'areas': []}
    if not Path(vault).is_dir():
        result['status'] = 'vault_unavailable'; return result
    result['status'] = 'mounted_read_only_inventory'
    for area in areas:
        p = Path(vault) / area
        if not p.is_dir():
            result['areas'].append({'area': area, 'status': 'missing'}); continue
        entries = []
        with os.scandir(p) as scan:
            for e in scan:
                if len(entries) >= limit:
                    raise ValueError(f'Top-level bound exceeded for {area}')
                entries.append({'name': e.name, 'source_relative': str(PurePosixPath(area) / e.name),
                                'kind': 'symlink' if e.is_symlink() else 'directory' if e.is_dir(follow_symlinks=False) else 'file'})
        result['areas'].append({'area': area, 'status': 'listed', 'entries': sorted(entries, key=lambda e: e['name']),
                                'count': len(entries)})
    return result


def api_snapshot(endpoint):
    try:
        with urllib.request.urlopen(endpoint, timeout=8) as r:
            body = r.read(16 * 1024 * 1024 + 1)
        if len(body) > 16 * 1024 * 1024:
            raise ValueError('API metadata exceeds bound')
        rows = json.loads(body)
        if not isinstance(rows, list):
            raise ValueError('Expected strategy metadata array')
        return {'status': 'available', 'endpoint': endpoint, 'sha256': hashlib.sha256(body).hexdigest(),
                'rows': len(rows), 'by_book': summarize_api(rows)}
    except Exception as e:
        return {'status': 'unavailable', 'endpoint': endpoint, 'error': str(e), 'by_book': {}}


def build(project=PROJECT, vault=VAULT, date='2026-10-13', endpoint='http://127.0.0.1:8090/api/guru/connors/strategies'):
    project = Path(project); vault = str(vault); output = project / 'library/source-vault'
    rows, inventory_audit = parse_inventory(project / 'library/research-drive-books.tsv')
    groups = group_records(rows)
    local, local_audit = inventory_local(project / 'Books')
    catalogue = catalogue_snapshot(project / 'Context/guru/frontend')
    api = api_snapshot(endpoint)
    api['source_catalogues'] = [{k: api.get(k) for k in ('endpoint', 'status', 'rows', 'sha256')}]
    api['additional_guru_books'] = {}
    if endpoint.endswith('/guru/connors/strategies'):
        for guru in ('chande', 'ross'):
            extra = api_snapshot(endpoint.replace('/guru/connors/', f'/guru/{guru}/'))
            api['source_catalogues'].append({k: extra.get(k) for k in ('endpoint', 'status', 'rows', 'sha256')})
            for book, data in extra['by_book'].items():
                if book not in api['by_book']:
                    api['by_book'][book] = data
                    api['additional_guru_books'][book] = guru
    fresh = fresh_top_level(vault)
    local_index = collections.defaultdict(list)
    for l in local:
        local_index[(l['filename'], l['bytes'])].append(l)
    catalog_index = collections.defaultdict(list)
    for author in catalogue:
        for book in author['books']:
            catalog_index[(normalize(author['name']), normalize(book['title']))].append(
                {'author_id': author['id'], 'author_name': author['name'], 'book_id': book['id'],
                 'title': book['title'], 'year': book.get('year'), 'description': book.get('blurb'),
                 'strategy_availability': api['by_book'].get(book['id']),
                 'independent_verification_status': 'not_established_by_catalogue_presence'})
    authors, plan = {}, []
    for g in groups:
        author_key = normalize(g['author']) or '(unknown)'
        a = authors.setdefault(author_key, {'id': 'author-' + hashlib.sha256(author_key.encode()).hexdigest()[:16],
                  'name': g['author'], 'source_author_spellings': [], 'identity': author_kind(g['author']),
                  'title_candidates': [], 'source_records': 0, 'pdf_records': 0,
                  'local_matched_title_candidates': 0, 'exact_catalogue_title_matches': 0,
                  'tested_book_count': None, 'independently_verified_book_count': None})
        if g['author'] not in a['source_author_spellings']:
            a['source_author_spellings'].append(g['author'])
        candidates, direct = {}, []
        for r in g['records']:
            path = r.get('path_or_archive!member') or ''
            try:
                size = int(r.get('size', ''))
            except (ValueError, TypeError):
                size = -1
            for match in local_index.get((member_filename(path), size), []):
                candidates[match['relative']] = match
            relative = safe_direct_relative(path, vault)
            if relative and PurePosixPath(relative).suffix.lower() == '.pdf' and r['metadata_status'] == 'parsed':
                direct.append({'source_record_id': r['source_record_id'], 'source_relative': relative, 'recorded_bytes': size})
        matches = catalog_index.get((author_key, normalize(g['title'])), [])
        title = {k: v for k, v in g.items() if k != 'records'}
        title.update({'source_records': len(g['records']),
                      'source_record_ids': [r['source_record_id'] for r in g['records']],
                      'formats': sorted(set(document_extension(r.get('path_or_archive!member')) for r in g['records'])),
                      'recorded_locations': [r.get('path_or_archive!member') for r in g['records']],
                      'archive_member_records': sum(bool(archive_locator(r.get('path_or_archive!member'))) for r in g['records']),
                      'original_availability': 'mounted_paths_not_individually_verified' if fresh['status'].startswith('mounted') else 'vault_unavailable',
                      'local_copy_status': 'filename_size_match_not_byte_verified' if candidates else 'no_exact_filename_size_match',
                      'local_candidates': list(candidates.values()), 'catalogue_matches': matches,
                      'summary_status': 'existing_catalogue_description_not_full_source_review' if matches else 'awaiting_source_review',
                      'summary': None, 'test_status': 'not_established',
                      'pdf_records': sum(document_extension(r.get('path_or_archive!member')) == '.pdf' for r in g['records'])})
        a['title_candidates'].append(title)
        a['source_records'] += len(g['records']); a['pdf_records'] += title['pdf_records']
        a['local_matched_title_candidates'] += bool(candidates)
        a['exact_catalogue_title_matches'] += bool(matches)
        if not candidates:
            preferred = sorted(direct, key=lambda d: (not d['source_relative'].startswith('Trading Books/'), d['source_relative']))
            plan.append({'title_id': g['id'], 'author': g['author'], 'title': g['title'],
                         'mapping_status': 'exact_catalogue_metadata_candidate' if matches else 'source_identity_review_required',
                         'status': 'needs_source_confirmation' if preferred and author_kind(g['author']) == 'named_metadata' else
                                   'archive_or_nonpdf_only' if not preferred else 'author_identity_required',
                         'copy_allowed_without_source_confirmation': False,
                         'source_candidates': preferred, 'catalogue_matches': matches,
                         'reason': 'No byte-identical local copy established. Filename metadata alone is insufficient to authorize a bibliographic mapping.'})
    result = {'schema_version': 1, 'generated': date, 'vault': vault, 'vault_access': 'read_only',
              'inventory_basis': 'library/research-drive-books.tsv, recorded 2026-10-10; current scope is limited to immediate named book areas',
              'count_definitions': {
                  'source_records': 'One row in the historical manifest; includes duplicate locations, articles, manuals and archive members.',
                  'pdf_records': 'Manifest records with .pdf member/file extension; not distinct books.',
                  'title_candidates': 'Normalized author + title + reported year; editions/volumes remain distinct; bibliographic identity is unverified.',
                  'author_groups': 'Normalized recorded author/vendor text; unknown metadata stays explicit. Not a verified author census.',
                  'local_matches': 'Exact filename and byte size only; no byte identity/completeness claim.',
                  'catalogue_entries': 'Explicit author/book-card metadata. Can combine books or split editions; not a source-file count.',
                  'tested_book_count': 'Null until explicit documented book-level verification is reconciled. Live/implemented/read is insufficient.'},
              'inventory_audit': inventory_audit, 'fresh_scope': fresh,
              'totals': {'source_records': len(rows), 'pdf_records': sum(a['pdf_records'] for a in authors.values()),
                         'title_candidates': len(groups), 'author_groups': len(authors),
                         'unknown_author_records': sum(a['source_records'] for a in authors.values() if a['identity'] == 'unknown'),
                         'local_document_files': len(local), 'local_pdf_files': sum(l['extension'] == '.pdf' for l in local),
                         'local_matched_title_candidates': sum(a['local_matched_title_candidates'] for a in authors.values()),
                         'catalogue_author_entries': len(catalogue), 'catalogue_book_entries': sum(len(a['books']) for a in catalogue),
                         'exact_catalogue_title_matches': sum(a['exact_catalogue_title_matches'] for a in authors.values()),
                         'independently_verified_books': None},
              'catalogue_snapshot': catalogue, 'strategy_metadata': api, 'authors': list(authors.values()),
              'limitations': ['No books are marked fully read by this inventory.',
                             'No summary prose or strategy rule is generated from filenames.',
                             'Missing filename matches do not prove the local source is absent under a renamed filename.',
                             'Same candidate title does not prove duplicate bytes or identical editions.',
                             'This inventory builder performs no imports/extraction. Separately authorized exact document/member copies are recorded in batch-import; videos and executables are excluded.',
                             'Historical inventory paths are not all rechecked today.']}
    output.mkdir(exist_ok=True)
    def write(name, value):
        p = output / name
        p.write_text(json.dumps(value, ensure_ascii=False, indent=2) + '\n')
    write(f'coverage-{date}.json', result)
    write(f'local-files-{date}.json', {'generated': date, 'audit': local_audit, 'files': local})
    write(f'missing-copy-plan-{date}.json', {'generated': date, 'source_read_only': True,
                 'automatic_imports_performed': 0, 'count': len(plan), 'candidates': plan})
    (output / f'source-records-{date}.jsonl').write_text(''.join(json.dumps(r, ensure_ascii=False) + '\n' for r in rows))
    print(json.dumps(result['totals'], indent=2))
    return result


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--date', default='2026-10-13')
    parser.add_argument('--vault', default=VAULT)
    parser.add_argument('--endpoint', default='http://127.0.0.1:8090/api/guru/connors/strategies')
    args = parser.parse_args()
    build(vault=args.vault, date=args.date, endpoint=args.endpoint)
