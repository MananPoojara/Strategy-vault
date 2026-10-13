import unittest
import tempfile
import json
from pathlib import Path
from scoped_import import source_jobs, destination_author, collision_books, run


class ScopedImportTests(unittest.TestCase):
    def test_jobs_are_only_captured_app_group_direct_pdf_originals(self):
        sources = [{'id': 'u', 'files': [
            {'record_id': 'r', 'source_relative': 'Trading Books/A.pdf', 'archive_member': False, 'filename': 'A.pdf'},
            {'record_id': 'z', 'source_relative': 'a.zip!A.pdf', 'archive_member': True, 'filename': 'A.pdf'},
            {'record_id': 'v', 'source_relative': 'a.mp4', 'archive_member': False, 'filename': 'a.mp4'}]}]
        x = {'authors': {'a': {'id': 'a', 'name': 'A', 'source_titles': sources},
                         'b': {'id': 'b', 'name': 'B', 'source_titles': sources}}}
        jobs = source_jobs(x, {'r': {'author': 'A'}})
        self.assertEqual(len(jobs), 1)
        self.assertEqual(jobs[0]['app_groups'], ['a', 'b'])

    def test_author_routing_prefers_existing_exact_folder(self):
        self.assertEqual(destination_author('Van K. Tharp', ['Van Tharp'], 'Van Tharp')[0], 'Van Tharp')
        self.assertEqual(destination_author('A & B', ['A & B', 'A'], 'Group')[0], 'A & B')

    def test_coauthored_book_can_route_to_its_explicit_existing_app_author(self):
        self.assertEqual(destination_author('Larry Connors, Cesar Alvarez',
                          ['Larry Connors','Cesar Alvarez'], 'Larry Connors')[0], 'Larry Connors')

    def test_unknown_or_unsafe_author_never_becomes_a_guessed_folder(self):
        for author in ['?', '../outside', 'A/B', 'bad\x00name']:
            with self.assertRaises(ValueError):
                destination_author(author, [], 'Group')

    def test_collision_subfolder_remains_inside_books(self):
        base = Path('/workspace/Books')
        p, folder = collision_books(base, 'Author', 'Trading Books/Some book/A.pdf')
        self.assertTrue(p.is_relative_to(base))
        self.assertEqual(p, base/'Author'/'Sources')
        self.assertTrue(folder.startswith('source-'))

    def test_serial_import_reuses_hash_duplicates_preserves_collision_and_source(self):
        with tempfile.TemporaryDirectory() as d:
            root = Path(d); project = root/'project'; vault = root/'vault'
            manifests = project/'library/source-vault'; manifests.mkdir(parents=True)
            author = project/'Books/Author'; author.mkdir(parents=True); vault.mkdir()
            old = author/'A.pdf'; old.write_bytes(b'%PDF-1.0\nold different bytes')
            (vault/'A.pdf').write_bytes(b'%PDF-1.0\nnew original')
            (vault/'B.pdf').write_bytes(b'%PDF-1.0\nnew original')
            before = {p.name: (p.read_bytes(),p.stat().st_mtime_ns) for p in vault.iterdir()}
            files = [{'record_id': str(i), 'source_relative': n, 'filename': n, 'archive_member': False}
                     for i,n in enumerate(['A.pdf','B.pdf','missing.pdf'])]
            coverage = {'authors': {'a': {'id':'a','name':'Author','source_titles':[{'id':'t','files':files}]}}}
            (manifests/'book-coverage.json').write_text(json.dumps(coverage))
            rows = [dict(source_record_id=str(i),author='Author',title='Book',metadata_status='parsed') for i in range(3)]
            (manifests/'source-records-test.jsonl').write_text(''.join(json.dumps(r)+'\n' for r in rows))
            run(project,vault,'test')
            log = [json.loads(l) for l in (manifests/'batch-import-test.jsonl').read_text().splitlines()]
            self.assertEqual([r['status'] for r in log], ['copied','reused_local_identical_sha256','original_unavailable'])
            self.assertEqual(old.read_bytes(), b'%PDF-1.0\nold different bytes')
            self.assertTrue((project/log[0]['destination']).is_file())
            self.assertIn('/Sources/source-',log[0]['destination'])
            self.assertEqual(before,{p.name:(p.read_bytes(),p.stat().st_mtime_ns) for p in vault.iterdir()})
            run(project,vault,'test')
            self.assertEqual(len((manifests/'batch-import-test.jsonl').read_text().splitlines()),3)


if __name__ == '__main__':
    unittest.main()
