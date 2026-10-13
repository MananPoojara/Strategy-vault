import importlib.util
from pathlib import Path
import unittest
spec=importlib.util.spec_from_file_location('reconcile_historical',Path(__file__).with_name('build_historical.py'))
helper=importlib.util.module_from_spec(spec);spec.loader.exec_module(helper)

class CopyProofs(unittest.TestCase):
 def base(self):return {'status':'copied','byte_identity_verified':True,'sha256':'a'*64,'source_hash_after':'a'*64,'source_sha256_unchanged':True,'source_mtime_unchanged':True,'source_mtime_before_ns':123,'source_mtime_after_ns':123}
 def test_missing_time_proof_is_not_original_identity(self):
  row=self.base();del row['source_mtime_before_ns'];del row['source_mtime_after_ns'];self.assertFalse(helper.proof_valid(row))
 def test_archive_body_sha_is_distinct_from_container_sha(self):
  row=self.base();row.update(source_kind='archive_member',source_member_sha256='a'*64,archive_sha256_before='b'*64,archive_sha256_after='b'*64,archive_mtime_before_ns=12,archive_mtime_after_ns=12)
  self.assertTrue(helper.proof_valid(row));row['source_member_sha256']='b'*64;self.assertFalse(helper.proof_valid(row))
 def test_missing_or_changed_archive_volume_proof_is_not_admitted(self):
  row=self.base();row.update(source_kind='archive_member',source_member_sha256='a'*64)
  self.assertFalse(helper.proof_valid(row))
  row.update(archive_sha256_before='b'*64,archive_sha256_after='b'*64,archive_mtime_before_ns=12,archive_mtime_after_ns=12,archive_volumes=[{'sha256_before':'c'*64,'sha256_after':'d'*64,'mtime_before_ns':12,'mtime_after_ns':12}])
  self.assertFalse(helper.proof_valid(row))
 def test_exclusions_preserve_book_titles_containing_data_and_literal_exclamation(self):
  self.assertIsNone(helper.exclusion('Trading Books/Market Data Analysis/Original!.pdf'))
  self.assertIsNotNone(helper.exclusion('USB HDD 2/US market database/prices.pdf'))
  self.assertIsNotNone(helper.exclusion('USB HDD 1/SOFTWARE/guide.pdf'))

if __name__=='__main__':unittest.main()
