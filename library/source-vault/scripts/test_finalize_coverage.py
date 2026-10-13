import unittest
import csv
import json
from pathlib import Path
import tempfile
from finalize_coverage import residuals, run


class FinalizeTests(unittest.TestCase):
    def test_shared_group_counts_once_and_exposes_unverified_child_of_available_title(self):
        title={'id':'t','title':'Book','author_metadata':'A','local_copies':[{'byte_identity_verified':True}],
               'files':[{'source_relative':'book.pdf','filename':'book.pdf','verified_local_copy':{'byte_identity_verified':True}},
                        {'source_relative':'a.zip!chapter.doc','filename':'chapter.doc','archive_member':True}]}
        entry={'source_titles':[title],'excluded_source_records':[{'record_id':'lock','reason':'temporary lock'}]}
        r=residuals({'authors':{'a':entry,'b':entry}})
        self.assertEqual(r['distinct_source_title_candidates'],1)
        self.assertEqual(r['source_titles_with_verified_local_copy'],1)
        self.assertEqual(r['source_title_candidates_without_verified_local_copy'],0)
        self.assertEqual(r['unverified_source_file_locations'],1)
        self.assertEqual(r['excluded_nonbook_source_records'],1)
        self.assertIn('No verified original extraction',r['pending_files'][0]['reason'])

    def test_public_csv_and_snapshot_keep_verification_unknown_and_count_actual_files(self):
        with tempfile.TemporaryDirectory() as d:
            root=Path(d)/'library/source-vault'; root.mkdir(parents=True)
            counts={'catalogue_books':2,'source_title_candidates':0,'source_file_records':0,
                    'local_document_files':1,'local_pdf_files':1,'source_titles_with_verified_local_copies':0,
                    'books_with_live_strategy_metadata':1,'books_with_data_missing':1,'books_not_testable':0,
                    'books_without_strategy_metadata':0,'independently_verified_books':None}
            (root/'book-coverage.json').write_text(json.dumps({'authors':{'a':{'id':'a','name':'Author','counts':counts,'source_titles':[]}}}))
            (root/'local-files-test.json').write_text(json.dumps({'files':[{'extension':'.pdf'},{'extension':'.doc'}]}))
            records=[{'source_relative':'a.pdf','status':'copied','bytes':12},
                     {'source_relative':'a.pdf','status':'reused_local_identical_sha256','bytes':12}]
            (root/'batch-import-test.jsonl').write_text(''.join(json.dumps(r)+'\n' for r in records))
            run(d,'test')
            with (root/'author-coverage-test.csv').open() as inp: row=next(csv.DictReader(inp))
            self.assertEqual(row['independently_verified_books'],'not_established')
            self.assertEqual(row['books_with_live_strategy_metadata'],'1')
            s=json.loads((root/'final-copy-summary-test.json').read_text())
            self.assertEqual(s['local_document_files'],2)
            self.assertEqual(s['local_pdf_files'],1)
            self.assertEqual(s['copied_bytes'],12)
            self.assertEqual(s['import_unique_source_paths'],1)


if __name__=='__main__': unittest.main()
