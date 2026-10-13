import io
import json
from pathlib import Path
import tempfile
import unittest
import zipfile
from unittest.mock import patch

from remaining_originals import detect_document, safe_member, zip_member, copy_body, archive_snapshot, NativeArchive, run


class RemainingOriginalTests(unittest.TestCase):
    def test_member_rejects_traversal_absolute_nested_and_executables(self):
        for name in ['../a.pdf', '/a.pdf', 'C:/a.pdf', 'a\\b.pdf', 'x.zip!a.pdf', 'a.exe', 'a/./b.pdf']:
            with self.assertRaises(ValueError):
                safe_member(name)
        self.assertEqual(safe_member('chapters/ReadThis!.PDF'), 'chapters/ReadThis!.PDF')
        self.assertEqual(safe_member('chapters/Study.doc'), 'chapters/Study.doc')

    def test_document_signatures_not_extension_only(self):
        with tempfile.TemporaryDirectory() as d:
            root = Path(d)
            for name, content, kind in [('a.pdf', b'%PDF-1.4\noriginal', 'pdf'),
                                        ('a.doc', bytes.fromhex('d0cf11e0a1b11ae1')+b'word', 'ole_document'),
                                        ('b.doc', b'{\\rtf1 hello}', 'rtf_document'),
                                        ('a.djvu', b'AT&TFORM0000DJVU', 'djvu')]:
                p = root/name; p.write_bytes(content)
                self.assertEqual(detect_document(p), kind)
            p = root/'fake.doc'; p.write_bytes(b'MZ executable')
            with self.assertRaises(ValueError): detect_document(p)
            p = root/'dos.doc'; p.write_bytes(b'Original software documentation\r\n\fpage 2\x1a')
            self.assertEqual(detect_document(p), 'plain_text_original_doc_extension')
            p = root/'legacy.doc'; p.write_bytes(b'Original technical\xa0notes \xa9 author\r\n')
            self.assertEqual(detect_document(p), 'plain_text_original_doc_extension')
            p = root/'bad.docx'
            with zipfile.ZipFile(p,'w') as z: z.writestr('x.txt','not Word')
            with self.assertRaises(ValueError): detect_document(p)
            p = root/'good.docx'
            with zipfile.ZipFile(p,'w') as z:
                z.writestr('[Content_Types].xml', '<Types/>'); z.writestr('word/document.xml','<document/>')
            self.assertEqual(detect_document(p), 'docx_container')
            p = root/'a.epub'
            with zipfile.ZipFile(p,'w') as z: z.writestr('mimetype', 'application/epub+zip')
            self.assertEqual(detect_document(p), 'epub_container')

    def test_targeted_zip_checks_size_and_does_not_execute_other_members(self):
        with tempfile.TemporaryDirectory() as d:
            p = Path(d)/'pack.zip'; body = b'%PDF-1.4\nactual original'
            with zipfile.ZipFile(p,'w') as z:
                z.writestr('inside/a.pdf',body); z.writestr('run.sh','rm -rf /')
            with zipfile.ZipFile(p) as z:
                with zip_member(z,'inside/a.pdf',len(body)) as stream:
                    self.assertEqual(stream.read(),body)
                with self.assertRaises(ValueError): zip_member(z,'inside/a.pdf',len(body)+1)
                with self.assertRaises(ValueError): zip_member(z,'run.sh',10)
            self.assertFalse((Path(d)/'run.sh').exists())

    def test_exclusive_copy_is_bounded_and_preserves_existing_destination(self):
        with tempfile.TemporaryDirectory() as d:
            p = Path(d)/'a.pdf'; body=b'%PDF-1.4\nbody'
            result = copy_body(io.BytesIO(body),p,len(body))
            self.assertEqual(result['bytes'],len(body))
            with self.assertRaises(FileExistsError): copy_body(io.BytesIO(body),p,len(body))
            self.assertEqual(p.read_bytes(),body)
            q = Path(d)/'short.pdf'
            with self.assertRaises(ValueError): copy_body(io.BytesIO(body[:-1]),q,len(body))
            self.assertFalse(q.exists())
            q = Path(d)/'long.pdf'
            with self.assertRaises(ValueError): copy_body(io.BytesIO(body+b'x'),q,len(body))
            self.assertFalse(q.exists())

    def test_source_archive_snapshot_records_each_volume_without_mutation(self):
        with tempfile.TemporaryDirectory() as d:
            root=Path(d); p=root/'a.rar'; p.write_bytes(b'archive original')
            before=(p.read_bytes(),p.stat().st_mtime_ns)
            a=archive_snapshot([p],root); b=archive_snapshot([p],root)
            self.assertEqual(a,b); self.assertEqual(a[0]['bytes'],len(before[0]))
            self.assertEqual(before,(p.read_bytes(),p.stat().st_mtime_ns))

    def test_native_reader_opaque_api_yields_exact_member_bytes(self):
        with tempfile.TemporaryDirectory() as d:
            p=Path(d)/'a.zip'; body=b'%PDF-1.4\nverified stream'
            with zipfile.ZipFile(p,'w') as z: z.writestr('a.pdf',body)
            with NativeArchive([p]) as reader:
                entries=[]
                for entry in reader.entries():
                    entries.append(entry)
                    self.assertEqual(reader.read(1024),body)
                    self.assertEqual(reader.read(1024),b'')
            self.assertEqual(entries[0]['bytes'],len(body))
            self.assertTrue(entries[0]['regular'])

    def test_complete_direct_and_member_import_records_distinct_body_container_proofs(self):
        with tempfile.TemporaryDirectory() as d:
            base=Path(d); project=base/'project'; vault=base/'vault'; vault.mkdir()
            root=project/'library/source-vault'; root.mkdir(parents=True)
            (project/'Books/Author').mkdir(parents=True)
            doc=bytes.fromhex('d0cf11e0a1b11ae1')+b'original Word document'
            (vault/'A.doc').write_bytes(doc)
            pdf=b'%PDF-1.4\noriginal PDF'
            with zipfile.ZipFile(vault/'A.zip','w') as z: z.writestr('inside/A.pdf',pdf)
            files=[{'record_id':'doc','filename':'A.doc','source_relative':'A.doc','recorded_bytes':len(doc)},
                   {'record_id':'pdf','filename':'A.pdf','source_relative':'A.zip!inside/A.pdf','recorded_bytes':len(pdf)}]
            coverage={'authors':{'a':{'id':'a','name':'Author','source_titles':[{'id':'t','files':files}]}}}
            (root/'book-coverage.json').write_text(json.dumps(coverage))
            rows=[dict(source_record_id=f['record_id'],author='Author',title='Source',metadata_status='parsed') for f in files]
            (root/'source-records-test.jsonl').write_text(''.join(json.dumps(r)+'\n' for r in rows))
            before={p.name:(p.read_bytes(),p.stat().st_mtime_ns) for p in vault.iterdir()}
            run(project,vault,'test')
            results=list(map(json.loads,(root/'batch-import-test.jsonl').read_text().splitlines()))
            self.assertEqual([r['status'] for r in results],['copied','copied'])
            member=results[1]
            self.assertEqual(member['sha256'],member['source_member_sha256'])
            self.assertEqual(member['archive_sha256_before'],member['archive_sha256_after'])
            self.assertNotEqual(member['sha256'],member['archive_sha256_after'])
            self.assertNotIn('source_hash_after',member)
            self.assertEqual(before,{p.name:(p.read_bytes(),p.stat().st_mtime_ns) for p in vault.iterdir()})
            run(project,vault,'test')
            self.assertEqual(len((root/'batch-import-test.jsonl').read_text().splitlines()),2)

    def test_corrupt_rar_target_cannot_poison_an_independent_valid_target(self):
        body=b'%PDF-1.4\noriginal'
        class Reader:
            def __init__(self,paths): self.poisoned=False; self.name=None; self.read_done=False
            def __enter__(self): return self
            def __exit__(self,*args): pass
            def entries(self):
                for name in ['bad.pdf','good.pdf']:
                    if self.poisoned: raise OSError('Reader state poisoned by failed selected member')
                    self.name=name; self.read_done=False
                    yield dict(name=name,bytes=len(body),regular=True,encrypted=False,size_set=True)
            def read(self,n):
                if self.name=='bad.pdf': self.poisoned=True; raise OSError('Bad selected member payload')
                if self.read_done:return b''
                self.read_done=True;return body
        with tempfile.TemporaryDirectory() as d:
            base=Path(d); project=base/'project';vault=base/'vault';vault.mkdir();(vault/'pack.rar').write_bytes(b'original container')
            root=project/'library/source-vault';root.mkdir(parents=True);(project/'Books/Author').mkdir(parents=True)
            files=[dict(record_id=n,filename=n,source_relative='pack.rar!'+n,recorded_bytes=len(body)) for n in ['bad.pdf','good.pdf']]
            (root/'book-coverage.json').write_text(json.dumps({'authors':{'a':{'id':'a','name':'Author','source_titles':[{'id':'t','files':files}]}}}))
            (root/'source-records-test.jsonl').write_text(''.join(json.dumps(dict(source_record_id=f['record_id'],author='Author',title='Original',metadata_status='parsed'))+'\n' for f in files))
            with patch('remaining_originals.NativeArchive',Reader):run(project,vault,'test')
            records={r['source_relative']:r for r in map(json.loads,(root/'batch-import-test.jsonl').read_text().splitlines())}
            self.assertEqual(records['pack.rar!bad.pdf']['status'],'held_for_review')
            self.assertEqual(records['pack.rar!good.pdf']['status'],'copied')
            self.assertTrue(records['pack.rar!good.pdf']['source_sha256_unchanged'])


if __name__ == '__main__': unittest.main()
