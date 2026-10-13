import errno
import importlib.util
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

spec=importlib.util.spec_from_file_location('pristine_importer',Path(__file__).with_name('import_related_sources.py'))
worker=importlib.util.module_from_spec(spec);spec.loader.exec_module(worker)

class Preservation(unittest.TestCase):
    def setUp(self):
        self.temp=tempfile.TemporaryDirectory();self.root=Path(self.temp.name);(self.root/'source').mkdir();(self.root/'books').mkdir()
        self.original=self.root/'source/document.pdf';self.payload=b'%PDF-1.4\nprivate preservation fixture\n';self.original.write_bytes(self.payload)
        self.before=self.original.stat();self.src=worker.safe.open_directory(self.root/'source');self.dst=worker.safe.open_directory(self.root/'books')
    def tearDown(self):
        os.close(self.src);os.close(self.dst);self.temp.cleanup()
    def test_no_hardlink_support_keeps_exact_original_and_source_unchanged(self):
        with patch.object(worker.os,'link',side_effect=OSError(errno.EPERM,'share has no hard links')):
            digest,path,status,mtime=worker.preserve(self.src,'document.pdf',self.dst,'Author/Original/document.pdf',len(self.payload))
        self.assertEqual(status,'copied');self.assertEqual((self.root/'books'/path).read_bytes(),self.payload)
        self.assertEqual(self.original.read_bytes(),self.payload);self.assertEqual(self.original.stat().st_mtime_ns,self.before.st_mtime_ns)
        self.assertEqual(digest,worker.hashlib.sha256(self.payload).hexdigest())
    def test_conflicting_destination_is_never_overwritten(self):
        target=self.root/'books/existing.pdf';target.write_bytes(b'%PDF-conflicting original')
        with self.assertRaisesRegex(ValueError,'Conflicting'):
            worker.preserve(self.src,'document.pdf',self.dst,'existing.pdf',len(self.payload))
        self.assertEqual(target.read_bytes(),b'%PDF-conflicting original')
    def test_bad_header_and_symlink_do_not_create_reading_files(self):
        self.original.write_bytes(b'garbage');
        with self.assertRaisesRegex(ValueError,'PDF header'):worker.preserve(self.src,'document.pdf',self.dst,'invalid.pdf',7)
        self.assertFalse((self.root/'books/invalid.pdf').exists())
        (self.root/'source/link.pdf').symlink_to(self.original)
        with self.assertRaises(OSError):worker.preserve(self.src,'link.pdf',self.dst,'link.pdf',7)
    def test_same_named_local_candidate_is_reused_only_when_body_hash_matches(self):
        target=self.root/'books/local.pdf';target.write_bytes(self.payload)
        expected={'path':'local.pdf','sha256':worker.hashlib.sha256(self.payload).hexdigest()}
        result=worker.preserve(self.src,'document.pdf',self.dst,'new.pdf',len(self.payload),expected)
        self.assertEqual(result[1:3],('local.pdf','reused_local_identical_sha256'));self.assertFalse((self.root/'books/new.pdf').exists())

if __name__=='__main__':unittest.main()
