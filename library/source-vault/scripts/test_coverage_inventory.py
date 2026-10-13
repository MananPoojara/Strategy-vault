import json
import tempfile
import unittest
from pathlib import Path

from coverage_inventory import (
    author_kind, group_records, inventory_local, local_matches, normalize,
    parse_inventory, safe_direct_relative, summarize_api, archive_locator, member_filename,
)
from coverage_inventory import build as build_global
from app_coverage import source_unit, alias_keys, build_author_entry, author_matches, attach_reviewed_sources
from app_coverage import build as build_app
from app_coverage import attach_batch_sources
from app_coverage import attach_related_material


class InventoryTests(unittest.TestCase):
    def test_normalization_preserves_edition_and_volume_distinctions(self):
        self.assertEqual(normalize("John O'Neil: Trading"), normalize('john o neil trading'))
        self.assertNotEqual(normalize('Trading 2nd edition'), normalize('Trading 3rd edition'))
        self.assertNotEqual(normalize('Trading Vol I'), normalize('Trading Vol II'))

    def test_known_author_name_is_still_not_bibliographically_verified(self):
        self.assertEqual(author_kind('?'), 'unknown')
        self.assertEqual(author_kind('vendor: example.com'), 'vendor_metadata')
        self.assertEqual(author_kind('Joe Ross'), 'named_metadata')

    def test_tsv_nul_is_reported_and_rows_are_preserved(self):
        with tempfile.TemporaryDirectory() as d:
            p = Path(d) / 'books.tsv'
            p.write_bytes(b'author\ttitle\tyear\tpath_or_archive!member\tsize\thow_identified\tsource\n'
                          b'?\tBad\x00 title\t\t/vault/a.pdf\t12\tunresolved\tdrive\n')
            rows, audit = parse_inventory(p)
            self.assertEqual(len(rows), 1)
            self.assertEqual(audit['nul_bytes'], 1)
            self.assertEqual(rows[0]['metadata_status'], 'contains_replaced_nul')

    def test_same_author_title_groups_candidates_without_hash_duplicate_claim(self):
        base = dict(author='A', title='Book', year='', source='drive', size='12',
                    metadata_status='parsed', how_identified='filename')
        rows = [dict(base, source_record_id='r1', **{'path_or_archive!member': '/v/a.pdf'}),
                dict(base, source_record_id='r2', **{'path_or_archive!member': '/v/copy/a.pdf'})]
        groups = group_records(rows)
        self.assertEqual(len(groups), 1)
        self.assertEqual(len(groups[0]['records']), 2)
        self.assertFalse(groups[0]['byte_duplicates_verified'])

    def test_years_are_not_silently_combined(self):
        rows = [dict(author='A', title='Book', year=str(y), source_record_id=str(y)) for y in [1997, 2000]]
        self.assertEqual(len(group_records(rows)), 2)

    def test_source_relative_rejects_archive_escape_and_other_mount(self):
        vault = '/vault'
        self.assertEqual(safe_direct_relative('/vault/Books/A.pdf', vault), 'Books/A.pdf')
        for bad in ['/vault/a.zip!A.pdf', '/other/A.pdf', '/vault/../A.pdf', '/vault2/A.pdf']:
            self.assertIsNone(safe_direct_relative(bad, vault))

    def test_literal_exclamation_is_not_an_archive_and_filename_is_preserved(self):
        p = '/vault/!PDFs/Read This First!.pdf'
        self.assertIsNone(archive_locator(p))
        self.assertEqual(safe_direct_relative(p,'/vault'),'!PDFs/Read This First!.pdf')
        self.assertEqual(member_filename(p),'Read This First!.pdf')
        self.assertEqual(archive_locator('/vault/!packs/a.zip!inside/A.pdf'),
                         ('/vault/!packs/a.zip','inside/A.pdf'))

    def test_local_match_requires_both_exact_filename_and_size(self):
        row = {'path_or_archive!member': '/v/A.pdf', 'size': '12'}
        local = [{'filename': 'A.pdf', 'bytes': 12, 'relative': 'A/A.pdf'},
                 {'filename': 'A.pdf', 'bytes': 13, 'relative': 'B/A.pdf'},
                 {'filename': 'a.pdf', 'bytes': 12, 'relative': 'C/a.pdf'}]
        self.assertEqual([x['relative'] for x in local_matches(row, local)], ['A/A.pdf'])

    def test_archive_member_can_have_local_candidate_but_not_direct_import(self):
        row = {'path_or_archive!member': '/v/A.zip!chapter/A.pdf', 'size': '12'}
        self.assertEqual(len(local_matches(row, [{'filename': 'A.pdf', 'bytes': 12}])), 1)
        self.assertIsNone(safe_direct_relative(row['path_or_archive!member'], '/v'))

    def test_local_scan_is_bounded_and_skips_symlinks(self):
        with tempfile.TemporaryDirectory() as d:
            p = Path(d); (p / 'Author').mkdir(); (p / 'Author' / 'A.pdf').write_bytes(b'%PDF-data')
            (p / 'Author' / 'link.pdf').symlink_to(p / 'Author' / 'A.pdf')
            (p / 'Author' / 'v.mp4').write_bytes(b'data')
            rows, audit = inventory_local(p, max_files=10)
            self.assertEqual([x['filename'] for x in rows], ['A.pdf'])
            self.assertEqual(audit['symlinks_skipped'], 1)
            with self.assertRaises(ValueError):
                inventory_local(p, max_files=0)

    def test_api_live_and_gate_are_availability_not_verified_book_count(self):
        rows = [{'book': 'b', 'id': 's1', 'status': 'live', 'gate': {'verdict': 'adaptation'}},
                {'book': 'b', 'id': 's2', 'status': 'needs_data'},
                {'book': 'b', 'id': 's3', 'status': 'not_testable'}]
        by_book = summarize_api(rows)
        self.assertEqual(by_book['b']['statuses'], {'live': 1, 'needs_data': 1, 'not_testable': 1})
        self.assertIsNone(by_book['b']['independently_verified_book_count'])
        self.assertEqual(by_book['b']['strategies'], 3)

    def test_chapter_pdfs_in_known_book_folder_form_one_container(self):
        a = {'author': 'Joe Ross', 'title': 'Chapter One', 'path_or_archive!member':
             '/vault/Trading Books/Joe Ross - TNT IV/Manual_C01.pdf'}
        b = dict(a, title='Chapter Two', **{'path_or_archive!member':
             '/vault/USB HDD 2/tRADING KNO-2/Joe Ross - TNT IV/Manual_C02.pdf'})
        self.assertEqual(source_unit(a, '/vault')['key'], source_unit(b, '/vault')['key'])
        self.assertTrue(source_unit(a, '/vault')['is_container'])

    def test_flat_book_area_does_not_collapse_all_books(self):
        a = {'author': 'A', 'title': 'One', 'path_or_archive!member': '/vault/Other TradingInvesting books/One.pdf'}
        b = dict(a, title='Two', **{'path_or_archive!member': '/vault/Other TradingInvesting books/Two.pdf'})
        self.assertNotEqual(source_unit(a, '/vault')['key'], source_unit(b, '/vault')['key'])

    def test_generic_vendor_folder_is_not_a_single_book(self):
        a = {'author': 'A', 'title': 'Workshop one', 'path_or_archive!member':
             '/vault/USB HDD 1/TradeStation Systems & Indicators/@INO (ino.com)/one.pdf'}
        b = dict(a, title='Workshop two', **{'path_or_archive!member':
             '/vault/USB HDD 1/TradeStation Systems & Indicators/@INO (ino.com)/two.pdf'})
        self.assertNotEqual(source_unit(a, '/vault')['key'], source_unit(b, '/vault')['key'])
        self.assertFalse(source_unit(a, '/vault')['is_container'])

    def test_author_aliases_use_explicit_names_and_source_folder_only(self):
        x = alias_keys({'name': 'A & B', 'books': []}, ['C'])
        self.assertIn(normalize('C'), x)
        self.assertNotIn(normalize('unrelated'), x)

    def test_nonobjective_catalogue_card_retains_description_and_null_verification(self):
        app = {'id': 'a', 'name': 'A', 'books': [{'id': 'b', 'title': 'A manual', 'blurb': 'Discretionary source.'}]}
        result = build_author_entry(app, [], [], {}, [], '/vault')
        self.assertEqual(result['catalogue_books'][0]['summary'], 'Discretionary source.')
        self.assertEqual(result['catalogue_books'][0]['status'], 'no_strategy_metadata')
        self.assertIsNone(result['counts']['independently_verified_books'])

    def test_software_support_pdfs_are_excluded_from_author_book_candidates(self):
        app={'id':'pring','name':'Martin Pring','books':[]}
        rows=[{'source_record_id':'software','author':'Martin Pring','title':'annots','year':'',
               'metadata_status':'parsed','size':'12','path_or_archive!member':
               '/vault/Course/Introduction to Technical Analysis.7z!OmniPage/gs705src.zip!gs7.05/examples/annots.pdf'}]
        result=build_author_entry(app,rows,[],{},[],'/vault')
        self.assertEqual(result['source_titles'],[])
        self.assertEqual(result['counts']['source_title_candidates'],0)
        self.assertEqual(len(result['excluded_source_records']),1)
        self.assertIn('Software',result['excluded_source_records'][0]['reason'])

    def test_word_owner_lock_file_is_not_an_original_book(self):
        app={'id':'pring','name':'Martin Pring','books':[]}
        row={'source_record_id':'lock','author':'Martin Pring','title':'mmodity Funds','year':'',
             'metadata_status':'parsed','size':'162','path_or_archive!member':'/vault/Pring/Web Links/~$mmodity Funds.doc'}
        result=build_author_entry(app,[row],[],{},[],'/vault')
        self.assertEqual(result['source_titles'],[])
        self.assertIn('owner-lock',result['excluded_source_records'][0]['reason'])

    def test_reviewed_software_support_remains_readable_without_book_credit(self):
        title={'id':'t','title':'SUPPORT','files':[{'source_relative':'pack.zip!SUPPORT.DOC',
               'verified_local_copy':{'sha256':'a'*64}}]}
        author={'counts':{'source_title_candidates':1},'source_titles':[title]}
        review={'source_relative':'pack.zip!SUPPORT.DOC','source_body_sha256':'a'*64,
                'summary':'Actual source installation instructions.','summary_evidence':{'method':'Actual bounded source text.'}}
        attach_related_material(author,[review])
        self.assertFalse(title['book_count_eligible'])
        self.assertEqual(title['source_material_kind'],'software_support')
        self.assertEqual(title['summary'],review['summary'])
        self.assertEqual(author['counts']['source_title_candidates'],1)
        self.assertEqual(author['counts']['book_candidate_source_units'],0)

    def test_full_named_coauthor_and_middle_initial_are_matched_conservatively(self):
        self.assertTrue(author_matches('Tushar S. Chande & Stanley Kroll', {'tushar chande'}))
        self.assertFalse(author_matches('John Chande', {'tushar chande'}))
        self.assertFalse(author_matches('John Smith', {'smith'}))
        self.assertTrue(author_matches("William J. O'Neil", {normalize("William O'Neil")}))

    def test_reviewed_edition_does_not_replace_original_catalogue_year_or_certify_results(self):
        author = {'id': 'oneil', 'counts': {}, 'source_titles': [],
                  'catalogue_books': [{'id': 'canslim', 'year': 1988, 'summary': None}]}
        rec = {'app_collection_id': 'oneil', 'catalogue_book_id': 'canslim',
               'destination': 'Books/Oneil/book.pdf', 'source_relative': 'book.pdf',
               'title_verified': 'How to Make Money in Stocks', 'author_verified': "William O'Neil",
               'source_edition_year': 1995, 'identity_evidence': 'PDF3-4',
               'sha256': 'a'*64, 'bytes': 12, 'source_summary': 'Read contents.',
               'summary_evidence': 'PDF3-8'}
        attach_reviewed_sources(author, [rec])
        card = author['catalogue_books'][0]
        self.assertEqual(card['year'], 1988)
        self.assertEqual(card['reviewed_source_copies'][0]['source_edition_year'], 1995)
        self.assertFalse(card['reviewed_source_copies'][0]['verified_performance'])
        self.assertEqual(card['summary'], 'Read contents.')

    def test_manifest_pipeline_preserves_source_and_reports_unavailable_api(self):
        with tempfile.TemporaryDirectory() as d:
            root = Path(d); project = root/'project'; vault = root/'vault'
            lib = project/'library'; lib.mkdir(parents=True)
            frontend = project/'Context/guru/frontend'; frontend.mkdir(parents=True)
            books = project/'Books/A'; books.mkdir(parents=True)
            (books/'A.pdf').write_bytes(b'%PDF-example')
            source = vault/'Trading Books/A - A book/A.pdf'; source.parent.mkdir(parents=True)
            source.write_bytes(b'%PDF-example'); before = (source.read_bytes(),source.stat().st_mtime_ns)
            (lib/'research-drive-books.tsv').write_text(
                'author\ttitle\tyear\tpath_or_archive!member\tsize\thow_identified\tsource\n'
                f'A\tA book\t\t{source}\t12\tfilename\tdrive\n')
            (frontend/'full-variant1.html').write_text(
                "const LIB = [\n{id:'a',name:'A',books:[{id:'b',title:'A book',blurb:'An existing description.'}]}\n];")
            (frontend/'joe-ross-evidence.js').write_text('window.JoeRossEvidence = {};')
            glob = build_global(project,vault,'test','http://127.0.0.1:1/unavailable')
            app = build_app(project,'test',root/'no_notes')
            self.assertEqual(glob['strategy_metadata']['status'],'unavailable')
            self.assertEqual(app['authors']['a']['counts']['source_title_candidates'],1)
            self.assertEqual(app['authors']['a']['catalogue_books'][0]['summary'],'An existing description.')
            self.assertIsNone(app['authors']['a']['counts']['independently_verified_books'])
            self.assertEqual(before,(source.read_bytes(),source.stat().st_mtime_ns))

    def test_batch_proof_reuses_renamed_copy_without_promoting_rules(self):
        author = {'id':'a','counts':{},'source_titles':[{'id':'t','files':[
            {'source_relative':'original.pdf','archive_member':False}], 'local_copies':[]} ]}
        rec = {'source_relative':'original.pdf','status':'reused_local_identical_sha256',
               'destination':'Books/A/renamed.pdf','sha256':'b'*64,'bytes':12,
               'byte_identity_verified':True,'source_sha256_unchanged':True,'source_mtime_unchanged':True}
        attach_batch_sources(author,{'original.pdf':rec})
        title=author['source_titles'][0]
        self.assertEqual(title['local_status'],'byte_verified_source_copies_available')
        self.assertEqual(title['local_copies'][0]['path'],'Books/A/renamed.pdf')
        self.assertFalse(title.get('verified_performance',False))
        self.assertEqual(author['counts']['source_titles_with_verified_local_copies'],1)

    def test_held_import_remains_pending_with_its_reason(self):
        author = {'id':'a','counts':{},'source_titles':[{'id':'t','files':[
            {'source_relative':'original.pdf','archive_member':False}], 'local_copies':[]}]}
        attach_batch_sources(author,{'original.pdf':{'status':'held_for_review','reason':'Missing author identity'}})
        self.assertEqual(author['source_titles'][0]['files'][0]['import_reason'],'Missing author identity')
        self.assertEqual(author['counts']['source_titles_with_verified_local_copies'],0)


if __name__ == '__main__':
    unittest.main()
