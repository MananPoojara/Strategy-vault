"""Source-coverage state and safety contracts, using generated source fixtures only."""
import copy,hashlib,json,tempfile,unittest
from pathlib import Path
from unittest import mock
import source_cursor as cursor

def pdf_fixture(path,pages=4):
 objects=[b'<< /Type /Catalog /Pages 2 0 R >>',('<< /Type /Pages /Count '+str(pages)+' /Kids ['+' '.join(str(i+3)+' 0 R' for i in range(pages))+'] >>').encode()]+[b'<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> >>']*pages
 data=b'%PDF-1.4\n';offset=[0]
 for i,obj in enumerate(objects,1):offset.append(len(data));data+=str(i).encode()+b' 0 obj\n'+obj+b'\nendobj\n'
 xref=len(data);data+=('xref\n0 '+str(len(offset))+'\n0000000000 65535 f \n').encode()+b''.join(f'{n:010} 00000 n \n'.encode() for n in offset[1:]);data+=f'trailer\n<< /Size {len(offset)} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n'.encode();path.write_bytes(data)
def cells(*pages):return [{'page':p,'region':'whole'} for p in pages]
class SourceCursorTests(unittest.TestCase):
 def setUp(self):
  self.tmp=tempfile.TemporaryDirectory();self.root=Path(self.tmp.name);self.source=self.root/'fixture.pdf';pdf_fixture(self.source);self.sha=hashlib.sha256(self.source.read_bytes()).hexdigest();self.task={'id':'source-one','author':'Fixture','book':'Generated source','source_paths':[str(self.source)],'source_hashes':{str(self.source):self.sha},'data_paths':[],'adaptations':[],'method_ids':[],'scope':'Bounded source fixture','instrument':'source-compatible only','source_cursor_required':True};self.state=self.root/'state';self.state.mkdir()
 def tearDown(self):self.tmp.cleanup()
 def sidecar(self,phase='convert',methods=3):
  folder=self.state/'jobs'/(self.task['id']+'.'+phase);folder.mkdir(parents=True,exist_ok=True);e=folder/'evidence.txt';e.write_text('Generated evidence fixture, never a real book excerpt.');method=[{'id':'m'+str(i),'family_id':'family-one','kind':'strategy','state':'held' if i<=3 else 'pending',**({'hold_reason':'Generated material-input fixture hold'} if i<=3 else {})} for i in range(1,methods+1)]
  value={'schema_version':1,'job_id':self.task['id']+'.'+phase,'coverage':{'source_sha256':self.sha,'physical_pages':4,'bibliographic_completeness':'unverified','missing_printed_pages':[],'inventory_complete':True,'sections':[{'id':'chapter1','label':'First section','pages':cells(1,2),'status':'read','methods':method,'evidence_ids':['text']},{'id':'chapter2','label':'Unseen section','pages':cells(3,4),'status':'unread','methods':[],'evidence_ids':[]}],'batch_section_ids':['chapter1'],'batch_method_ids':['m'+str(i) for i in range(1,min(methods,3)+1)]},'evidence':[{'id':'text','path':'evidence.txt','sha256':hashlib.sha256(e.read_bytes()).hexdigest(),'pages':cells(1,2)}]}
  path=folder/'source-coverage.json';path.write_text(json.dumps(value));return path,value
 def pair(self,methods=3,change=None):
  path,value=self.sidecar(methods=methods)
  if change:change(value);path.write_text(json.dumps(value))
  cursor.freeze_converter(self.task,self.state);rp,review=self.sidecar('review',methods);review['coverage']=copy.deepcopy(value['coverage']);review['accepted']=True;review['converter_sidecar_sha256']=hashlib.sha256(path.read_bytes()).hexdigest();rp.write_text(json.dumps(review));return path,rp,review
 def test_actual_generated_pdf_page_count_and_hash(self):
  info=cursor.source_identity(self.task);self.assertEqual(info['physical_pages'],4);self.assertEqual(info['sha256'],self.sha)
 def test_held_three_method_batch_retains_unseen_chapter(self):
  self.pair();result=cursor.accept_review(self.task,self.state);self.assertFalse(result['all_available_sections_reviewed']);self.assertFalse(result['complete_bibliographic_work_reviewed']);self.assertIn('chapter2',result['unread_section_ids']);self.assertEqual(result['blocked_method_ids'],['m1','m2','m3']);self.assertEqual(result['proposals'][0]['section_ids'],['chapter2'])
 def test_more_than_three_discovered_methods_are_not_discarded(self):
  self.pair(methods=5);r=cursor.accept_review(self.task,self.state);self.assertEqual(r['pending_method_ids'],['m4','m5']);self.assertTrue(any(p.get('method_ids')==['m4','m5'] for p in r['proposals']))
 def test_partial_scan_can_finish_available_pages_without_whole_book_claim(self):
  def change(v):
   c=v['coverage'];c['bibliographic_completeness']='partial';c['missing_printed_pages']=['5–20 absent from supplied scan'];c['sections'][0]['pages']=cells(1,2,3,4);c['sections'][1].update(pages=[],status='source_missing');v['evidence'][0]['pages']=cells(1,2,3,4)
  path,v=self.sidecar();change(v);path.write_text(json.dumps(v));cursor.freeze_converter(self.task,self.state);rp,r=self.sidecar('review');r['coverage']=copy.deepcopy(v['coverage']);r['evidence'][0]['pages']=cells(1,2,3,4);r.update(accepted=True,converter_sidecar_sha256=hashlib.sha256(path.read_bytes()).hexdigest());rp.write_text(json.dumps(r));out=cursor.accept_review(self.task,self.state);self.assertTrue(out['all_available_sections_reviewed']);self.assertFalse(out['complete_bibliographic_work_reviewed']);self.assertEqual(out['missing_source_section_ids'],['chapter2'])
 def test_source_hash_page_bounds_and_overlap_errors_are_rejected(self):
  for edit in [lambda v:v['coverage'].update(source_sha256='0'*64),lambda v:v['coverage'].update(physical_pages=5),lambda v:v['coverage']['sections'][1].update(pages=cells(4,5)),lambda v:v['coverage']['sections'][1].update(pages=cells(2,3)),lambda v:v['coverage']['sections'].append(copy.deepcopy(v['coverage']['sections'][0])),lambda v:v['coverage'].update(batch_method_ids=['m1','m2','m3','m4'])]:
   with self.subTest(edit=edit):
    path,v=self.sidecar();edit(v);path.write_text(json.dumps(v))
    with self.assertRaises(ValueError):cursor.freeze_converter(self.task,self.state)
 def test_reviewer_must_agree_and_be_distinct_and_accept(self):
  for edit in [lambda r:r['coverage']['sections'][0]['methods'][0].update(state='source_checked'),lambda r:r.update(job_id='source-one.convert'),lambda r:r.update(accepted=False),lambda r:r.update(converter_sidecar_sha256='0'*64)]:
   with self.subTest(edit=edit):
    _,rp,r=self.pair();edit(r);rp.write_text(json.dumps(r))
    with self.assertRaises(ValueError):cursor.accept_review(self.task,self.state)
 def test_converter_mutation_after_freeze_is_rejected(self):
  cp,_,_=self.pair();cp.write_text(cp.read_text()+' ')
  with self.assertRaisesRegex(ValueError,'changed'):cursor.accept_review(self.task,self.state)
 def test_rewriting_phase_binding_cannot_reseal_converter_during_review(self):
  cp,_,_=self.pair();binding=cp.parent/'source-coverage-binding.json';v=json.loads(binding.read_text());v['original_sha256']='0'*64;binding.write_text(json.dumps(v))
  with self.assertRaisesRegex(ValueError,'binding changed'):cursor.accept_review(self.task,self.state)
 def test_refreeze_is_idempotent_and_cannot_overwrite_original_frozen_evidence(self):
  cp,_,_=self.pair();frozen=cp.parent/'source-coverage.frozen.json';trusted=self.state/'source-cursor-bindings'/(self.task['id']+'.json');old=(frozen.read_bytes(),trusted.read_bytes());first=cursor.freeze_converter(self.task,self.state);self.assertEqual(first,json.loads(trusted.read_text()));self.assertEqual((frozen.read_bytes(),trusted.read_bytes()),old);cp.write_text(cp.read_text()+' ')
  with self.assertRaisesRegex(ValueError,'binding changed'):cursor.freeze_converter(self.task,self.state)
  self.assertEqual((frozen.read_bytes(),trusted.read_bytes()),old)
 def test_refreeze_detects_tampered_frozen_copy_without_replacing_it(self):
  cp,_,_=self.pair();frozen=cp.parent/'source-coverage.frozen.json';frozen.write_text(frozen.read_text()+' ');tampered=frozen.read_bytes()
  with self.assertRaisesRegex(ValueError,'artifact changed'):cursor.freeze_converter(self.task,self.state)
  self.assertEqual(frozen.read_bytes(),tampered)
 def test_evidence_escape_hash_or_symlink_is_rejected(self):
  for mode in ['escape','absolute','hash','symlink']:
   with self.subTest(mode=mode):
    path,v=self.sidecar();e=path.parent/'evidence.txt'
    if mode=='escape':v['evidence'][0]['path']='../outside.txt'
    elif mode=='absolute':v['evidence'][0]['path']=str(e)
    elif mode=='hash':v['evidence'][0]['sha256']='0'*64
    else:e.unlink();e.symlink_to(self.source)
    path.write_text(json.dumps(v))
    with self.assertRaises(ValueError):cursor.freeze_converter(self.task,self.state)
    if e.is_symlink():e.unlink()
 def test_malformed_json_types_fail_closed_without_coordinator_typeerror(self):
  for edit in [lambda v:v.update(schema_version=True),lambda v:v['evidence'][0].update(id=7),lambda v:v['evidence'][0].update(sha256=None),lambda v:v['coverage']['sections'][0].update(id=[]),lambda v:v['coverage']['sections'][0]['methods'][0].update(family_id={}),lambda v:v['coverage'].update(batch_section_ids='chapter1')]:
   with self.subTest(edit=edit):
    path,v=self.sidecar();edit(v);path.write_text(json.dumps(v))
    with self.assertRaises(ValueError):cursor.freeze_converter(self.task,self.state)
 def test_symlink_job_directory_cannot_escape_private_state(self):
  outside=self.root/'outside';outside.mkdir();(self.state/'jobs').symlink_to(outside,target_is_directory=True)
  with self.assertRaisesRegex(ValueError,'private job'):cursor.job_folder(self.task,self.state,'convert')
 def test_left_and_right_spread_regions_are_disjoint(self):
  cp,v=self.sidecar();v['coverage']['sections'][0]['pages']=[{'page':1,'region':'left'}];v['coverage']['sections'][1]['pages']=[{'page':1,'region':'right'}];v['coverage']['inventory_complete']=False;v['evidence'][0]['pages']=[{'page':1,'region':'left'}];cp.write_text(json.dumps(v));cursor.freeze_converter(self.task,self.state)
 def test_child_proposals_are_capped_and_stable_without_backlog_changes(self):
  cp,v=self.sidecar();v['coverage']['inventory_complete']=False;v['coverage']['sections'][1:]=[{'id':'next'+str(i),'pages':[{'page':p,'region':r}],'status':'unread','methods':[],'evidence_ids':[]} for i,(p,r) in enumerate([(3,'left'),(3,'right'),(4,'left'),(4,'right')])];cp.write_text(json.dumps(v));cursor.freeze_converter(self.task,self.state);rp,r=self.sidecar('review');r['coverage']=copy.deepcopy(v['coverage']);r.update(accepted=True,converter_sidecar_sha256=hashlib.sha256(cp.read_bytes()).hexdigest());rp.write_text(json.dumps(r));a=cursor.accept_review(self.task,self.state,max_proposals=2);b=cursor.accept_review(self.task,self.state,max_proposals=2);self.assertEqual(len(a['proposals']),2);self.assertEqual(a['proposals'],b['proposals']);self.assertEqual(len(a['unread_section_ids']),4);self.assertFalse((self.state/'backlog.json').exists())
 def test_oversized_unread_section_is_retained_without_invented_page_chunk(self):
  coverage={'source_sha256':self.sha,'sections':[{'id':'large','pages':cells(*range(1,34)),'status':'unread','methods':[]}]}
  proposals,oversized=cursor.residual_proposals(coverage,{'reviewer_job_id':'reviewed'},3);self.assertEqual(proposals,[]);self.assertEqual(oversized,['large'])
 def test_actual_source_change_and_invalid_pdf_are_failures_not_empty_books(self):
  self.source.write_bytes(b'Changed actual source')
  with self.assertRaisesRegex(ValueError,'hash'):cursor.source_identity(self.task)
  self.task['source_hashes'][str(self.source)]=hashlib.sha256(self.source.read_bytes()).hexdigest()
  with self.assertRaisesRegex(ValueError,'page count'):cursor.source_identity(self.task)
 def test_incomplete_inventory_cannot_claim_all_available_pages(self):
  path,v=self.sidecar();v['coverage']['sections'][1]['pages']=cells(3);path.write_text(json.dumps(v))
  with self.assertRaisesRegex(ValueError,'all actual physical'):cursor.freeze_converter(self.task,self.state)
 def test_unread_section_cannot_carry_unverified_method_inventory(self):
  path,v=self.sidecar();v['coverage']['sections'][1]['methods']=[{'id':'not-read','family_id':'unread-family','kind':'strategy','state':'pending'}];path.write_text(json.dumps(v))
  with self.assertRaisesRegex(ValueError,'reviewed source'):cursor.freeze_converter(self.task,self.state)
 def test_registry_prevents_erasing_held_methods_or_reviewed_sections(self):
  self.pair();cursor.accept_review(self.task,self.state);self.task={**self.task,'id':'source-two'};cp,v=self.sidecar();v['coverage']['sections'][0]['methods'][0]['state']='pending';cp.write_text(json.dumps(v));cursor.freeze_converter(self.task,self.state);rp,r=self.sidecar('review');r['coverage']=copy.deepcopy(v['coverage']);r.update(accepted=True,converter_sidecar_sha256=hashlib.sha256(cp.read_bytes()).hexdigest());rp.write_text(json.dumps(r))
  with self.assertRaisesRegex(ValueError,'regress|held'):cursor.accept_review(self.task,self.state)
 def test_registry_seed_is_metadata_only_and_legacy_is_unproven(self):
  legacy={**self.task,'id':'legacy'};legacy.pop('source_cursor_required')
  with mock.patch.object(cursor,'source_identity',side_effect=AssertionError('No corpus scan')):cursor.seed_registry([legacy,self.task],self.state)
  r=json.loads((self.state/'source-cursors.json').read_text());self.assertEqual(r['tasks']['legacy']['coverage_status'],'legacy_coverage_unproven');self.assertEqual(r['sources'][self.sha]['coverage_status'],'coverage_unproven')
if __name__=='__main__':unittest.main()
