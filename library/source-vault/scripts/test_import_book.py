import importlib.util
from pathlib import Path
from tempfile import TemporaryDirectory
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location('import_book', Path(__file__).with_name('import_book.py'))
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)

class ImportSafety(unittest.TestCase):
    def setUp(self):
        self.temp = TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.vault = self.root / 'vault'
        self.vault.mkdir()
        self.source = self.vault / 'book.pdf'
        self.source.write_bytes(b'%PDF-1.7\nprivate test bytes\n')
        self.books = self.root / 'Books'

    def copy(self, relative='book.pdf', author='Test Author', books=None):
        return module.import_pdf(relative, author, self.vault, books or self.books)

    def test_copy_and_repeat_preserve_source_and_existing_destination(self):
        before = self.source.read_bytes(), self.source.stat().st_mtime_ns
        first = self.copy()
        target = self.books / 'Test Author' / 'book.pdf'
        self.assertEqual(target.read_bytes(), before[0])
        self.assertEqual(first['sha256'], module.digest(self.source))
        timestamp = target.stat().st_mtime_ns
        self.assertEqual(self.copy()['status'], 'already_present_identical')
        self.assertEqual(target.stat().st_mtime_ns, timestamp)
        self.assertEqual((self.source.read_bytes(), self.source.stat().st_mtime_ns), before)
        self.assertEqual(list(self.vault.iterdir()), [self.source])
        self.assertFalse(first['source_completeness_verified'])
        self.assertFalse(first['independent_performance_verified'])

    def test_collision_never_overwrites_either_file(self):
        target = self.books / 'Test Author' / 'book.pdf'
        target.parent.mkdir(parents=True)
        target.write_bytes(b'existing different file')
        with self.assertRaises(FileExistsError): self.copy()
        self.assertEqual(target.read_bytes(), b'existing different file')
        self.assertEqual(self.source.read_bytes(), b'%PDF-1.7\nprivate test bytes\n')

    def test_source_escape_and_author_escape_are_rejected(self):
        outside = self.root / 'outside.pdf'
        outside.write_bytes(b'%PDF-1.7 outside')
        for relative in ['../outside.pdf', str(outside)]:
            with self.subTest(relative=relative), self.assertRaises(ValueError): self.copy(relative)
        for author in ['..', '/outside', 'a/b', 'a\\b', '']:
            with self.subTest(author=author), self.assertRaises(ValueError): self.copy(author=author)
        self.assertFalse(self.books.exists())

    def test_destination_inside_vault_and_symlink_escape_are_rejected(self):
        with self.assertRaises(ValueError): self.copy(books=self.vault / 'Books')
        self.books.mkdir()
        (self.books / 'Test Author').symlink_to(self.vault, target_is_directory=True)
        with self.assertRaises(ValueError): self.copy()
        self.assertEqual(list(self.vault.iterdir()), [self.source])

    def test_misnamed_non_pdf_is_rejected_without_copy(self):
        self.source.write_bytes(b'not a PDF')
        with self.assertRaises(ValueError): self.copy()
        self.assertFalse(self.books.exists())

    def test_failed_copy_verification_cleans_only_new_destination(self):
        original = module.digest(self.source)
        def fake_digest(path):
            return original if path == self.source else 'mismatch'
        with patch.object(module, 'digest', fake_digest), self.assertRaises(ValueError): self.copy()
        self.assertFalse((self.books / 'Test Author' / 'book.pdf').exists())
        self.assertTrue(self.source.exists())

if __name__ == '__main__': unittest.main()
