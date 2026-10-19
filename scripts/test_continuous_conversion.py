import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from unittest import mock

MODULE = Path(__file__).with_name('continuous_conversion.py')
spec = importlib.util.spec_from_file_location('continuous_conversion', MODULE)
queue = importlib.util.module_from_spec(spec)
spec.loader.exec_module(queue)


class ContinuousConversionTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp.name)
        self.source = self.root/'book.txt'
        self.source.write_text('User-owned source fixture')
        self.state = self.root/'state'
        self.state.mkdir()
        self.workspace = self.root/'candidate'
        self.workspace.mkdir()
        (self.workspace/'.git').mkdir()
        self.task = {'id':'one-book','author':'Test Author','book':'Objective book',
                     'source_paths':[str(self.source)],'source_hashes':{str(self.source):queue.sha256(self.source)},
                     'data_paths':[],'method_ids':['test_rule'],'scope':'Review the explicit source rule',
                     'instrument':'actual source-compatible instrument','adaptations':[]}

    def tearDown(self):
        self.tmp.cleanup()

    def test_completed_and_held_books_are_not_repeated(self):
        second={**self.task,'id':'second-book'}
        for status in ('held','reviewed_pending_integration','failed'):
            state={'tasks':{'one-book':{'status':status}}}
            self.assertEqual(queue.choose_task([self.task,second],state)['id'],'second-book')

    def test_conversion_is_followed_by_a_distinct_review_job(self):
        template=Path(__file__).with_name('continuous_conversion_prompt.txt')
        convert=queue.make_job(self.task,'convert',self.workspace,self.state,'/usr/bin/codex',template)
        review=queue.make_job(self.task,'review',self.workspace,self.state,'/usr/bin/codex',template)
        self.assertNotEqual(convert['id'],review['id'])
        self.assertNotEqual(convert['result_file'],review['result_file'])
        self.assertIn('--json',review['argv'])
        self.assertIn('approval_policy="never"',review['argv'])
        self.assertIn('--output-schema',review['argv'])
        self.assertIn('--sandbox',review['argv'])
        self.assertNotIn('--dangerously-bypass-approvals-and-sandbox',review['argv'])
        schema=json.loads(Path(review['argv'][review['argv'].index('--output-schema')+1]).read_text())
        self.assertEqual(schema['properties']['admitted']['enum'],[False])

    def test_source_hash_change_stops_before_an_agent_is_started(self):
        self.source.write_text('Changed source')
        with self.assertRaisesRegex(ValueError,'source hash'):
            queue.verify_inputs(self.task)

    def test_reviewed_candidate_never_becomes_live_admission(self):
        result={'source_reviewed':True,'implementation_ready':True,'ledger_checked':True,
                'checked_trades':12,'admitted':False}
        self.assertEqual(queue.review_outcome(result),'reviewed_pending_integration')
        result['ledger_checked']=False
        self.assertEqual(queue.review_outcome(result),'held')

    def test_independent_review_cannot_claim_live_admission(self):
        with self.assertRaisesRegex(ValueError,'admission'):
            queue.review_outcome({'admitted':True})

    def test_candidate_changes_during_review_are_rejected(self):
        relative='Context/guru/backend/src/books/test.rs'
        path=self.workspace/relative
        path.parent.mkdir(parents=True)
        path.write_text('fn reviewed() {}')
        snapshot={relative:queue.sha256(path)}
        queue.verify_candidate(self.workspace,snapshot)
        path.write_text('fn changed() {}')
        with self.assertRaisesRegex(ValueError,'changed'):
            queue.verify_candidate(self.workspace,snapshot)

    def test_unsafe_and_duplicate_backlog_ids_are_rejected(self):
        path=self.state/'backlog.json'
        for tasks in ([self.task,self.task],[{**self.task,'id':'../escape'}]):
            path.write_text(json.dumps({'schema_version':1,'approved_by':'owner','tasks':tasks}))
            with self.assertRaises(ValueError):queue.load_backlog(path)

    def test_account_limit_stops_before_starting_another_book(self):
        backlog=self.state/'backlog.json'
        backlog.write_text(json.dumps({'schema_version':1,'approved_by':'owner','tasks':[self.task]}))
        (self.state/'LIMIT_REACHED.json').write_text('{}')
        with mock.patch.object(queue,'run_stage') as run_stage:
            self.assertEqual(queue.run(backlog,self.state,self.workspace,self.root/'worktrees','/usr/bin/codex'),0)
        run_stage.assert_not_called()
        self.assertEqual(json.loads((self.state/'pipeline.json').read_text())['status'],'usage_limit')

    def test_low_disk_space_waits_without_consuming_a_source_task(self):
        backlog=self.state/'backlog.json'
        backlog.write_text(json.dumps({'schema_version':1,'approved_by':'owner','tasks':[self.task]}))
        def stop_after_wait(*_):
            self.assertEqual(json.loads((self.state/'pipeline.json').read_text())['status'],'waiting_for_disk')
            (self.state/'STOP').write_text('Stop fixture')
        with mock.patch.object(queue,'available_disk_bytes',return_value=0), \
             mock.patch.object(queue.time,'sleep',side_effect=stop_after_wait), \
             mock.patch.object(queue,'run_stage') as run_stage:
            self.assertEqual(queue.run(backlog,self.state,self.workspace,self.root/'worktrees','/usr/bin/codex'),0)
        run_stage.assert_not_called()
        self.assertEqual(json.loads((self.state/'pipeline.json').read_text())['status'],'stopped')

    def test_real_queue_refills_and_reviews_two_books_without_a_human(self):
        backlog=self.state/'backlog.json'
        tasks=[self.task,{**self.task,'id':'second-book'}]
        backlog.write_text(json.dumps({'schema_version':1,'approved_by':'owner','tasks':tasks}))
        calls=[]
        result={'source_reviewed':True,'implementation_ready':True,'ledger_checked':True,
                'checked_trades':2,'admitted':False,'method_ids':['test_rule'],'evidence':[]}
        def stage(task,phase,*args,**kwargs):
            calls.append((task['id'],phase))
            return result
        with mock.patch.object(queue,'prepare_workspace',return_value=self.workspace), \
             mock.patch.object(queue,'run_stage',side_effect=stage), \
             mock.patch.object(queue,'candidate_snapshot',return_value={}):
            self.assertEqual(queue.run(backlog,self.state,self.workspace,self.root/'worktrees','/usr/bin/codex'),0)
        self.assertEqual(calls,[('one-book','convert'),('one-book','review'),('second-book','convert'),('second-book','review')])
        state=json.loads((self.state/'pipeline.json').read_text())
        self.assertEqual(state['status'],'backlog_processed')
        self.assertTrue(all(x['status']=='reviewed_pending_integration' for x in state['tasks'].values()))
        self.assertEqual(state['live_admissions'],0)


    def test_legacy_prompt_bytes_and_job_signature_remain_exact(self):
        template=Path(__file__).with_name('continuous_conversion_prompt.txt')
        folder=self.state/'jobs'/'one-book.convert'
        context={**self.task,'phase':'convert','artifact_dir':str(folder),
            'candidate_workspace':str(self.workspace),'converter_result':str(folder/'result.json'),
            'frozen_candidate':str(folder/'candidate-files.json'),'reviewer_identity':'one-book.convert','admitted':False}
        expected=template.read_text()+'\n'+json.dumps(context,indent=2)+'\n'
        job=queue.make_job(self.task,'convert',self.workspace,self.state,'/usr/bin/codex',template)
        self.assertEqual(Path(job['stdin_file']).read_text(),expected)
        first=queue.worker.digest(job)
        self.assertEqual(queue.worker.digest(queue.make_job(self.task,'convert',self.workspace,self.state,'/usr/bin/codex',template)),first)

    def test_cursor_optin_requires_typed_supported_contract(self):
        path=self.state/'backlog.json'
        for changes in ({'source_cursor_required':'true'},{'source_cursor_contract_version':True},
                        {'source_cursor_contract_version':2}):
            path.write_text(json.dumps({'schema_version':1,'approved_by':'owner','tasks':[{**self.task,**changes}]}))
            with self.assertRaisesRegex(ValueError,'source_cursor_required|source cursor contract'):
                queue.load_backlog(path)

    def test_flagged_task_with_existing_legacy_prompt_stays_legacy_both_phases(self):
        self.task['source_cursor_required']=True
        template=Path(__file__).with_name('continuous_conversion_prompt.txt')
        folder=self.state/'jobs'/'one-book.convert';folder.mkdir(parents=True)
        context={**self.task,'phase':'convert','artifact_dir':str(folder),'candidate_workspace':str(self.workspace),
            'converter_result':str(folder/'result.json'),'frozen_candidate':str(folder/'candidate-files.json'),
            'reviewer_identity':'one-book.convert','admitted':False}
        text=template.read_text()+'\n'+json.dumps(context,indent=2)+'\n';(folder/'prompt.txt').write_text(text)
        job=queue.make_job(self.task,'convert',self.workspace,self.state,'/usr/bin/codex',template)
        self.assertEqual(Path(job['stdin_file']).read_text(),text)
        review=queue.make_job(self.task,'review',self.workspace,self.state,'/usr/bin/codex',template)
        self.assertNotIn('"source_cursor":',Path(review['stdin_file']).read_text())
        self.assertEqual(queue.cursor.context_version(self.task,self.state),1)

    def test_new_optin_prompt_context_is_versioned_and_pinned_for_retry(self):
        self.task['source_cursor_required']=True
        template=self.root/'template.txt';template.write_text('Frozen base prompt fixture')
        first=queue.make_job(self.task,'convert',self.workspace,self.state,'/usr/bin/codex',template)
        text=Path(first['stdin_file']).read_text();self.assertIn('"source_cursor":',text)
        template.write_text('Changed future template; existing retry must stay frozen')
        retry=queue.make_job(self.task,'convert',self.workspace,self.state,'/usr/bin/codex',template)
        self.assertEqual(Path(retry['stdin_file']).read_text(),text);self.assertEqual(queue.worker.digest(first),queue.worker.digest(retry))
        self.assertEqual(queue.cursor.context_version(self.task,self.state),2)
        self.task['scope']='Changed task is not a retry of the same specification'
        with self.assertRaisesRegex(ValueError,'Pinned job context changed'):
            queue.make_job(self.task,'convert',self.workspace,self.state,'/usr/bin/codex',template)

    def test_new_cursor_task_records_held_scope_and_disjoint_proposals_end_to_end(self):
        from test_source_cursor import pdf_fixture,cells
        pdf=self.root/'source.pdf';pdf_fixture(pdf)
        self.task.update(source_paths=[str(pdf)],source_hashes={str(pdf):queue.sha256(pdf)},source_cursor_required=True)
        backlog=self.state/'backlog.json';payload={'schema_version':1,'approved_by':'owner','tasks':[self.task]};backlog.write_text(json.dumps(payload));before=backlog.read_bytes()
        template=Path(__file__).with_name('continuous_conversion_prompt.txt')
        result={'source_reviewed':True,'implementation_ready':False,'ledger_checked':False,'checked_trades':0,'admitted':False,'method_ids':['held-method'],'evidence':[]}
        def stage(task,phase,workspace,state,codex):
            queue.make_job(task,phase,workspace,state,codex,template)
            folder=state/'jobs'/(task['id']+'.'+phase);e=folder/'evidence.txt';e.write_text('Generated fixture evidence')
            coverage={'source_sha256':queue.sha256(pdf),'physical_pages':4,'bibliographic_completeness':'partial','missing_printed_pages':['Final volume not supplied'],'inventory_complete':True,
                'sections':[{'id':'read-chapter','pages':cells(1,2),'status':'read','methods':[{'id':'held-method','family_id':'one-family','kind':'strategy','state':'held','hold_reason':'Actual required derivative input absent'}],'evidence_ids':['proof']},
                            {'id':'unseen-chapter','pages':cells(3,4),'status':'unread','methods':[],'evidence_ids':[]}],
                'batch_section_ids':['read-chapter'],'batch_method_ids':['held-method']}
            value={'schema_version':1,'job_id':task['id']+'.'+phase,'coverage':coverage,'evidence':[{'id':'proof','path':'evidence.txt','sha256':queue.sha256(e),'pages':cells(1,2)}]}
            if phase=='review':value.update(accepted=True,converter_sidecar_sha256=queue.sha256(state/'jobs'/(task['id']+'.convert')/'source-coverage.json'))
            (folder/'source-coverage.json').write_text(json.dumps(value));return result
        with mock.patch.object(queue,'prepare_workspace',return_value=self.workspace),mock.patch.object(queue,'run_stage',side_effect=stage),mock.patch.object(queue,'candidate_snapshot',return_value={}):
            self.assertEqual(queue.run(backlog,self.state,self.workspace,self.root/'worktrees','/usr/bin/codex'),0)
        self.assertEqual(backlog.read_bytes(),before);pipeline=json.loads((self.state/'pipeline.json').read_text());self.assertEqual(pipeline['tasks']['one-book']['status'],'held');self.assertEqual(pipeline['live_admissions'],0)
        registry=json.loads((self.state/'source-cursors.json').read_text());record=registry['sources'][queue.sha256(pdf)];self.assertEqual(record['unread_section_ids'],['unseen-chapter']);self.assertEqual(record['proposals'][0]['section_ids'],['unseen-chapter']);self.assertFalse(record['complete_bibliographic_work_reviewed'])


if __name__=='__main__':unittest.main()
