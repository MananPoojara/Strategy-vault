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


if __name__=='__main__':unittest.main()
