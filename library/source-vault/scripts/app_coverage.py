#!/usr/bin/env python3
"""Author-group coverage for the existing app, with file/book distinctions.

Uses explicit catalogue names and source-folder references in existing notes.
Does not promote inferred aliases, chapter files, or Live strategies to proof.
"""
import collections
import hashlib
import json
from pathlib import Path, PurePosixPath
import re
from functools import lru_cache
from coverage_inventory import PROJECT, VAULT, normalize, author_kind, safe_direct_relative, archive_locator, member_filename

BOOK_AREAS = ('Trading Books/', 'Trading Knowledge I/', 'Trading Knowledge II/',
              'Other TradingInvesting books/', 'Larry Connors/', 'Prasad/ALL BOOKS/',
              'USB HDD 2/tRADING KNO-2/', 'USB HDD 1/TradeStation Systems & Indicators/',
              'USB HDD 1/MetaStock/@Systems & Indicators/')

# Spelling/full-name relations already present in local author folders and app.
# These are routing metadata, never author identity or book-fidelity certification.
EXPLICIT = {
 'covel': ['Michael Covel', 'Curtis Faith', 'Russell Sands'],
 'jaekle': ['Urban Jaekle & Emilio Tomasini'],
 'katz': ['Jeffrey Owen Katz', 'Jeffrey Owen Katz & Donna L. McCormick'],
 'hirsch': ['Jeffrey & Yale Hirsch', 'Yale Hirsch', 'Jeffrey A. Hirsch'],
 'demark': ['Thomas DeMark'], 'ruggiero': ['Murray A. Ruggiero Jr', 'Murray Ruggiero'],
 'momentum': ['Gary Antonacci', 'Josh Murakami', 'Hurst Ooi Pedersen', 'Leslie Payne'],
 'bandy': ['Howard B. Bandy'], 'weissman': ['Richard L. Weissman'],
 'clayburg': ['John Clayburg', 'John Clayburg'],
 'haggerty': ['Kevin Haggerty', 'Cynthia Kase'],
 'quant': ['Geoff Bysshe', 'Ernest Chan'],
 'seasonal': ['Jay Kaeppel', 'Marvin Appel'],
 'gettess': ['Lee E. Gettess', 'Peter W. Aan'],
 'lukac': ['Louis Lukac', 'J.D. Hamon'],
 'timers': ['Martin Zweig', 'Ned Davis', 'Jack Schannep', 'Ryan Jones'],
 'evidence': ['David Aronson', 'Emmanuel Acar'],
 'blau': ['William Blau', 'Tom Bierovic'],
 'sperandeo': ['Victor Sperandeo', 'David Vomund'],
 'berg': ['Jim Berg', 'Richard Anderson'],
 'mtiming': ['Leslie Masonson', 'Jeremy Siegel', 'Thomas Lydon', 'F.E. James', 'Martin Pring'],
 'sysmisc': ['John Sweeney', 'Anthony W. Warren', 'Chick Goslin', 'Markus Heitkoetter', 'Gary Ford', 'Joseph T. Duffy'],
 'swingstk': ['Steve Palmquist', 'Steve Burns', 'Thomas K. Carr', 'Larry Swing'],
 'vendors': ['Curtis Arnold', 'Elm Trader'],
 'oneil': ["William O'Neil"], 'guppy': ['Daryl Guppy'],
}


def source_unit(row, vault=VAULT):
    path = row.get('path_or_archive!member') or ''
    relative = path[len(vault) + 1:] if path.startswith(vault + '/') else path
    author = row.get('author') or '?'
    for area in BOOK_AREAS:
        if relative.startswith(area):
            remainder = relative[len(area):]
            parts = PurePosixPath(remainder).parts
            if (len(parts) > 1 and not archive_locator(parts[0]) and not parts[0].startswith('@') and
                normalize(parts[0]) not in {'misc', 'articles', 'books', 'all books'}):
                label = parts[0]
                title = label.split(' - ', 1)[1] if ' - ' in label else label
                return {'key': (normalize(author), normalize(label), 'book_folder'),
                        'title': title, 'unit_label': label, 'is_container': True,
                        'basis': 'named book-area parent folder; may contain chapters/copies/editions'}
    return {'key': (normalize(author), normalize(row.get('title')), str(row.get('year') or '')),
            'title': row.get('title') or PurePosixPath(member_filename(relative)).stem,
            'unit_label': row.get('title') or '', 'is_container': False,
            'basis': 'recorded author/title/year; bibliographic identity unverified'}


def alias_keys(app_author, source_folders):
    names = [app_author['name']] + list(source_folders) + EXPLICIT.get(app_author.get('id'), [])
    # Strip a UI annotation, not a human name or coauthor expression.
    names.append(re.sub(r'\s*\(formula collection\)\s*$', '', app_author['name']))
    return {normalize(n) for n in names if normalize(n)}


@lru_cache(maxsize=16384)
def person_tokens(name):
    return tuple(t for t in normalize(str(name).replace("'", '').replace('’', '')).split() if len(t) > 1)


def author_matches(recorded, keys):
    if normalize(recorded) in keys:
        return True
    # Full first name and surname only; never a bare surname or "others".
    # Middle initials and explicit coauthors occur in the recorded author field.
    source = list(person_tokens(recorded))
    for key in keys:
        named = list(person_tokens(key))
        if len(named) < 2 or len(named[0]) < 3 or len(named[-1]) < 3:
            continue
        # O'Neil normalizes to "o neil" in routing keys; join that surname.
        if ' o neil' in key:
            named = [t for t in named if t != 'neil'] + ['oneil']
        if any(source[i:i+len(named)] == named for i in range(len(source)-len(named)+1)):
            return True
    return False


def source_folder_refs(app_author, local_files, notes_root):
    folders = set(x['author_folder'] for x in local_files if x.get('author_folder'))
    refs = []
    for book in app_author['books']:
        p = Path(notes_root) / book['id'] / 'README.md'
        if not p.exists():
            continue
        text = p.read_text(errors='replace')
        for folder in re.findall(r'Books/([^/\n`]+)/', text):
            if folder in folders:
                refs.append({'folder': folder, 'book_id': book['id'],
                             'basis': 'existing module README explicitly references local Books folder',
                             'review_note': f'library/{book["id"]}/README.md'})
    return refs


def catalogue_status(availability):
    if not availability:
        return 'no_strategy_metadata'
    statuses = availability['statuses']
    if statuses.get('live', 0):
        return 'strategy_available_verification_separate'
    if statuses.get('needs_data', 0):
        return 'data_missing'
    return 'not_testable' if statuses.get('not_testable', 0) else 'unknown'


def build_author_entry(app, rows, local, by_book, source_folders, vault=VAULT):
    keys = alias_keys(app, source_folders)
    matched_authors = {a for a in set(r.get('author') for r in rows) if author_matches(a, keys)}
    relevant = [r for r in rows if r.get('author') in matched_authors]
    excluded = []
    reading = []
    for r in relevant:
        locator = archive_locator(r.get('path_or_archive!member'))
        filename = member_filename(r.get('path_or_archive!member'))
        if filename in {'~$mmodity Funds.doc', '~$ro Coupon Bond Funds.doc'} and str(r.get('size')) == '162':
            excluded.append({'record_id': r['source_record_id'], 'path': r.get('path_or_archive!member'),
                             'recorded_author': r.get('author'), 'recorded_title': r.get('title'),
                             'reason': 'Office owner-lock temporary file, 162 bytes with owner metadata; not original document content.'})
        elif locator and locator[0].endswith('/Introduction to Technical Analysis.7z') and (
                'OmniPage/' in locator[1] or 'gs705src.zip!' in locator[1] or
                (locator[1].endswith('/Guides/Guide ENG.pdf') and r.get('title') == 'OP12 User’s Guide')):
            excluded.append({'record_id': r['source_record_id'],
                             'path': r.get('path_or_archive!member'),
                             'recorded_author': r.get('author'), 'recorded_title': r.get('title'),
                             'reason': 'Software installer/support PDF; Pring course-folder placement is not trading-book authorship proof.'})
        else:
            reading.append(r)
    relevant = reading
    local_index = collections.defaultdict(list)
    for f in local:
        local_index[(f['filename'], f['bytes'])].append(f)
    groups = {}
    for r in relevant:
        u = source_unit(r, vault)
        key = (u['key'][1], 'book_folder') if u['is_container'] else u['key']
        g = groups.setdefault(key, {'unit': u, 'records': []})
        g['records'].append(r)
    source_titles = []
    for key, group in sorted(groups.items(), key=lambda kv: str(kv[0])):
        source_rows, u = group['records'], group['unit']
        files, local_copies, candidate_books = [], {}, []
        for r in source_rows:
            path = r.get('path_or_archive!member') or ''
            filename = member_filename(path)
            try:
                size = int(r.get('size', ''))
            except (TypeError, ValueError):
                size = None
            matches = local_index.get((filename, size), [])
            for f in matches:
                local_copies[f['relative']] = {'path': 'Books/' + f['relative'], 'bytes': f['bytes'],
                    'match': 'exact_filename_and_size_only', 'byte_identity_verified': False}
            files.append({'record_id': r['source_record_id'], 'filename': filename, 'recorded_bytes': size,
                          'source_relative': path[len(vault) + 1:] if path.startswith(vault + '/') else None,
                          'archive_member': bool(archive_locator(path)), 'metadata_status': r['metadata_status'],
                          'local_candidates': ['Books/' + f['relative'] for f in matches]})
        title_key = normalize(u['title'])
        for b in app['books']:
            bkey = normalize(b['title'])
            if bkey == title_key or (len(bkey) >= 18 and title_key.startswith(bkey + ' ')):
                candidate_books.append({'id': b['id'], 'match': 'exact_title_metadata' if bkey == title_key else
                                        'catalogue_title_prefix_candidate_not_identity_proof'})
        source_titles.append({'id': 'source-' + hashlib.sha256(json.dumps(key).encode()).hexdigest()[:16],
                  'title': u['title'], 'author_metadata': source_rows[0].get('author'),
                  'unit_label': u['unit_label'], 'count_basis': u['basis'],
                  'is_container': u['is_container'], 'bibliographic_identity': 'unverified_source_unit',
                  'years_reported': sorted(set(r.get('year') for r in source_rows if r.get('year'))),
                  'source_file_records': len(files), 'files': files, 'local_copies': list(local_copies.values()),
                  'local_status': 'candidate_local_copies' if local_copies else 'local_copy_not_matched',
                  'catalogue_candidates': candidate_books,
                  'summary_status': 'awaiting_source_review', 'summary': None,
                  'strategy_status': 'not_established', 'verified_performance': False,
                  'note': 'Folder/candidate coverage only. Child PDFs may be chapters, editions, articles or duplicates. No objective rules or test results inferred.'})
    cards = []
    for b in app['books']:
        availability = by_book.get(b['id'])
        status = catalogue_status(availability)
        links = [t['id'] for t in source_titles if any(m['id'] == b['id'] for m in t['catalogue_candidates'])]
        cards.append({'id': b['id'], 'title': b['title'], 'year': b.get('year'),
                      'summary': b.get('blurb'), 'summary_status': 'existing_catalogue_description' if b.get('blurb') else 'awaiting_source_review',
                      'status': status, 'source_title_candidates': links,
                      'strategy_availability': availability, 'verified_performance': False,
                      'verification_status': 'not_established_by_inventory'})
    app_local = [f for f in local if author_matches(f.get('author_folder'), keys)]
    return {'id': app['id'], 'name': app['name'],
            'group_kind': 'app_author_or_editorial_group',
            'author_alias_keys': sorted(keys), 'source_author_spellings': sorted(set(r.get('author') or '?' for r in relevant)),
            'counts': {'catalogue_books': len(cards), 'source_title_candidates': len(source_titles),
                       'source_file_records': len(relevant), 'local_document_files': len(app_local),
                       'local_pdf_files': sum(f['extension'] == '.pdf' for f in app_local),
                       'source_titles_with_local_candidates': sum(bool(t['local_copies']) for t in source_titles),
                       'books_with_live_strategy_metadata': sum(c['status'] == 'strategy_available_verification_separate' for c in cards),
                       'books_with_data_missing': sum(c['status'] == 'data_missing' for c in cards),
                       'books_not_testable': sum(c['status'] == 'not_testable' for c in cards),
                       'books_without_strategy_metadata': sum(c['status'] == 'no_strategy_metadata' for c in cards),
                       'independently_verified_books': None},
            'catalogue_books': cards, 'source_titles': source_titles,
            'excluded_source_records': excluded,
            'local_sources': [{'path': 'Books/' + f['relative'], 'bytes': f['bytes'],
                               'extension': f['extension'], 'byte_identity_verified': False} for f in app_local],
            'note': 'Counts distinguish app cards, source-folder/title candidates and files. Live strategy availability is not independently verified book coverage.'}


def attach_reviewed_sources(author, records):
    for r in records:
        if r.get('app_collection_id') != author['id']:
            continue
        public = {k: r.get(k) for k in ('destination', 'source_relative', 'title_verified',
                 'author_verified', 'source_edition_year', 'identity_evidence', 'sha256', 'bytes',
                 'source_summary', 'summary_evidence')}
        public.update({'byte_identity_verified': True, 'complete_book_review': False,
                       'verified_performance': False})
        author.setdefault('reviewed_source_copies', []).append(public)
        for title in author['source_titles']:
            if any(f.get('source_relative') == r['source_relative'] for f in title['files']):
                title.setdefault('reviewed_source_copies', []).append(public)
                title['authored_by_verified_for_reviewed_copy'] = r.get('author_verified')
                if r.get('source_summary'):
                    title.update({'summary': r['source_summary'], 'summary_status': 'bounded_preface_or_contents_review',
                                  'summary_evidence': r.get('summary_evidence')})
                title['local_status'] = 'reviewed_copy_available_byte_verified'
        for card in author['catalogue_books']:
            if card['id'] == r.get('catalogue_book_id'):
                card.setdefault('reviewed_source_copies', []).append(public)
                if r.get('source_summary'):
                    card.update({'summary': r['source_summary'], 'summary_status': 'bounded_preface_or_contents_review',
                                 'summary_evidence': r.get('summary_evidence')})
        author['counts']['reviewed_source_copy_identities'] = len(author.get('reviewed_source_copies', []))


def attach_batch_sources(author, latest):
    verified_titles = 0
    for title in author['source_titles']:
        verified = {}
        counts = collections.Counter()
        for f in title['files']:
            rec = latest.get(f.get('source_relative'))
            if not rec:
                continue
            status = rec['status']; counts[status] += 1
            f['import_status'] = status
            if rec.get('reason'):
                f['import_reason'] = rec['reason']
            if (status in {'copied', 'already_present_identical', 'reused_local_identical_sha256'} and
                rec.get('byte_identity_verified') and rec.get('source_sha256_unchanged') and
                rec.get('source_mtime_unchanged')):
                copy = {'path': rec['destination'], 'bytes': rec['bytes'], 'sha256': rec['sha256'],
                        'match': 'source_and_destination_sha256_equal', 'byte_identity_verified': True}
                verified[rec['destination']] = copy
                f['verified_local_copy'] = copy
        title['import_statuses'] = dict(counts)
        if verified:
            other = [c for c in title.get('local_copies', []) if c['path'] not in verified]
            title['local_copies'] = list(verified.values()) + other
            title['local_status'] = 'byte_verified_source_copies_available'
            verified_titles += 1
        title['copy_verification_scope'] = 'Original-file byte equality only; bibliography, completeness, rules and performance remain separate.'
    author['counts']['source_titles_with_verified_local_copies'] = verified_titles


def attach_related_material(author, records):
    related = set()
    for t in author['source_titles']:
        for f in t['files']:
            for r in records:
                if (f.get('source_relative') == r['source_relative'] and
                        f.get('verified_local_copy', {}).get('sha256') == r['source_body_sha256']):
                    f['source_material_kind'] = 'software_support'
                    if len(t['files']) == 1:
                        t.update({'source_material_kind': 'software_support', 'book_count_eligible': False,
                                  'summary': r['summary'], 'summary_status': 'bounded_actual_document_text_review',
                                  'summary_evidence': r['summary_evidence'],
                                  'note': 'Related software documentation, not an author-verified trading book or strategy source.'})
                        related.add(t['id'])
    author['counts']['related_material_candidates'] = len(related)
    author['counts']['book_candidate_source_units'] = author['counts']['source_title_candidates'] - len(related)


def build(project=PROJECT, date='2026-10-13', notes_root='/home/server/.local/share/strategy-vault/library'):
    root = Path(project) / 'library/source-vault'
    coverage = json.loads((root / f'coverage-{date}.json').read_text())
    rows = [json.loads(line) for line in (root / f'source-records-{date}.jsonl').read_text().splitlines()]
    local = json.loads((root / f'local-files-{date}.json').read_text())['files']
    authors, mapping = {}, {}
    imports = root / f'import-manifest-{date}.json'
    reviewed = json.loads(imports.read_text())['records'] if imports.exists() else []
    related_path = root / f'related-material-review-{date}.json'
    related = json.loads(related_path.read_text())['records'] if related_path.exists() else []
    batch = root / f'batch-import-{date}.jsonl'
    latest = {}
    if batch.exists():
        for line in batch.read_text().splitlines():
            try:
                r = json.loads(line)
                latest[r['source_relative']] = r
            except json.JSONDecodeError:
                # Running append-only log can expose a trailing in-progress row.
                continue
    for app in coverage['catalogue_snapshot']:
        refs = source_folder_refs(app, local, notes_root)
        mapping[app['id']] = refs
        authors[app['id']] = build_author_entry(app, rows, local,
                         coverage['strategy_metadata']['by_book'], [r['folder'] for r in refs], coverage['vault'])
        attach_reviewed_sources(authors[app['id']], reviewed)
        attach_batch_sources(authors[app['id']], latest)
        attach_related_material(authors[app['id']], related)
    result = {'schema_version': 1, 'as_of': date, 'source_inventory_date': '2026-10-10',
              'current_vault_status': coverage['fresh_scope']['status'],
              'source_access': 'read_only', 'authors': authors,
              'count_definitions': coverage['count_definitions'],
              'count_warning': 'Source title candidates are not a verified distinct-book census. App groups may share authors; never sum group counts as unique books.',
              'totals': {'app_author_groups': len(authors),
                         'catalogue_books': sum(a['counts']['catalogue_books'] for a in authors.values()),
                         'distinct_source_title_candidates': len(set(t['id'] for a in authors.values() for t in a['source_titles'])),
                         'independently_verified_books': None}}
    (root / 'book-coverage.json').write_text(json.dumps(result, ensure_ascii=False, indent=2) + '\n')
    (root / 'app-author-source-mapping.json').write_text(json.dumps({'as_of': date, 'explicit_aliases': EXPLICIT,
                        'module_note_source_folder_refs': mapping,
                        'identity_certification': False}, ensure_ascii=False, indent=2) + '\n')
    print(json.dumps(result['totals'], indent=2))
    return result


if __name__ == '__main__':
    build()
